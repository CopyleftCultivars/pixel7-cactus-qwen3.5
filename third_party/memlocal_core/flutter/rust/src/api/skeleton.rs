use std::sync::Arc;

use flutter_rust_bridge::DartFnFuture;
use memlocal_core::api::MemlocalEngine;
use memlocal_core::models::{CoreConfig, MemoryItem, MemoryType, StorageConfig};

/// Opaque handle held by Dart. (FRB treats unknown structs as opaque.)
pub struct Memlocal {
    pub(crate) engine: Arc<MemlocalEngine>,
}

impl Memlocal {
    /// Open an in-memory engine. Phase 0 smoke entry point.
    pub fn open_in_memory(dimensions: u32) -> Result<Memlocal, String> {
        let config = CoreConfig {
            storage: StorageConfig {
                in_memory: true,
                embedding_dimensions: dimensions,
                ..Default::default()
            },
            ..Default::default()
        };
        let engine = MemlocalEngine::open(config).map_err(|e| e.to_string())?;
        Ok(Memlocal {
            engine: Arc::new(engine),
        })
    }

    /// Total stored memories (None = all types).
    pub fn memory_count(&self) -> Result<u32, String> {
        self.engine
            .memory_count(None)
            .map(|c| c as u32)
            .map_err(|e| e.to_string())
    }

    /// Open a persistent engine backed by a SQLite file at `db_path`.
    pub fn open(db_path: String, dimensions: u32) -> Result<Memlocal, String> {
        let config = CoreConfig {
            storage: StorageConfig {
                in_memory: false,
                db_path: Some(db_path),
                embedding_dimensions: dimensions,
                ..Default::default()
            },
            ..Default::default()
        };
        let engine = MemlocalEngine::open(config).map_err(|e| e.to_string())?;
        Ok(Memlocal {
            engine: Arc::new(engine),
        })
    }

    /// Open a persistent keyword-only engine without allocating embedding storage or HNSW indexes.
    pub fn open_keyword_only(db_path: String) -> Result<Memlocal, String> {
        Self::open(db_path, 0)
    }

    /// Store `content` as a memory of the given type (stored-name string, e.g. "factual",
    /// "episodic", "spatial", ...) with an optional caller-supplied embedding. Omit it
    /// when the engine was opened in keyword-only mode. Returns the new id.
    pub fn add_memory(
        &self,
        content: String,
        kind: String,
        embedding: Option<Vec<f64>>,
    ) -> Result<String, String> {
        let memory_type = MemoryType::from_stored_name(&kind);
        let item = MemoryItem::new(content, memory_type);
        let embedding: Vec<f32> = embedding
            .unwrap_or_default()
            .into_iter()
            .map(|value| value as f32)
            .collect();
        self.engine
            .put_memory(&item, &embedding)
            .map_err(|e| e.to_string())?;
        Ok(item.id)
    }

    /// Semantic (HNSW) search using a caller-supplied query embedding. Returns the top-k recalled memories.
    pub fn search_semantic(
        &self,
        embedding: Vec<f32>,
        k: u32,
    ) -> Result<Vec<RecalledMemory>, String> {
        let items = self
            .engine
            .search_semantic(&embedding, k as usize, None, None)
            .map_err(|e| e.to_string())?;
        Ok(items
            .into_iter()
            .map(|m| RecalledMemory {
                id: m.id,
                content: m.content,
                kind: m.memory_type.stored_name().to_string(),
                score: m.score,
            })
            .collect())
    }

    /// BM25 keyword search using the local full-text index.
    pub fn search_keyword(&self, query: String, k: u32) -> Result<Vec<RecalledMemory>, String> {
        let items = self
            .engine
            .search_text(&query, k as usize)
            .map_err(|e| e.to_string())?;
        Ok(items
            .into_iter()
            .map(|m| RecalledMemory {
                id: m.id,
                content: m.content,
                kind: m.memory_type.stored_name().to_string(),
                score: m.score,
            })
            .collect())
    }
}

/// A memory returned from a search, flattened for the FFI boundary.
pub struct RecalledMemory {
    pub id: String,
    pub content: String,
    /// The stored memory-type name (e.g. "factual").
    pub kind: String,
    /// Relevance score from the search, if available.
    pub score: Option<f64>,
}

/// Calls a Dart-provided async closure and returns its result.
/// Proves FRB can invoke Dart back from Rust (foundation for Dart-side providers).
pub async fn call_dart_closure(value: i32, callback: impl Fn(i32) -> DartFnFuture<i32>) -> i32 {
    callback(value).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opens_in_memory_and_counts_zero() {
        let m = Memlocal::open_in_memory(1536).expect("engine opens");
        assert_eq!(m.memory_count().expect("count works"), 0);
    }

    #[test]
    fn add_then_search_roundtrip() {
        let m = Memlocal::open_in_memory(8).expect("engine opens");
        let emb = vec![0.1_f32, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8];
        let id = m
            .add_memory(
                "hello world".to_string(),
                "factual".to_string(),
                Some(emb.iter().map(|value| f64::from(*value)).collect()),
            )
            .expect("add");
        assert!(!id.is_empty());
        let hits = m.search_semantic(emb, 5).expect("search");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].content, "hello world");
    }

    #[test]
    fn keyword_only_accepts_missing_embedding_and_rejects_semantic_search() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("memlocal-ffi-{nonce}.db"));
        let m =
            Memlocal::open_keyword_only(path.to_string_lossy().into_owned()).expect("engine opens");
        let id = m
            .add_memory("Valle del Cauca".to_string(), "factual".to_string(), None)
            .expect("add without embedding");
        assert!(!id.is_empty());
        assert_eq!(m.memory_count().expect("count works"), 1);
        match m.search_semantic(vec![], 5) {
            Err(error) => assert!(error.contains("keyword-only")),
            Ok(_) => panic!("semantic search should fail in keyword-only mode"),
        }
        drop(m);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn keyword_search_returns_ranked_results_and_handles_empty_query() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("memlocal-search-ffi-{nonce}.db"));
        let m = Memlocal::open_keyword_only(path.to_string_lossy().into_owned())
            .expect("keyword-only engine opens");
        let cauca = m
            .add_memory(
                "Finca en Valle del Cauca: maíz y yuca".to_string(),
                "factual".to_string(),
                None,
            )
            .expect("add Cauca memory");
        m.add_memory(
            "San José uses coffee pulp as compost".to_string(),
            "factual".to_string(),
            None,
        )
        .expect("add coffee memory");
        m.add_memory(
            "Moringa plants are useful bioaccumulators".to_string(),
            "factual".to_string(),
            None,
        )
        .expect("add moringa memory");

        let hits = m
            .search_keyword("VALLE DEL CAUCA".to_string(), 3)
            .expect("keyword search");
        assert_eq!(hits[0].id, cauca);
        assert!(hits[0].score.is_some());
        assert!(m
            .search_keyword("!!!".to_string(), 3)
            .expect("empty sanitized query")
            .is_empty());

        drop(m);
        let _ = std::fs::remove_file(path);
    }
}
