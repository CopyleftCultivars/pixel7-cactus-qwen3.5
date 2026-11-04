# Natural Farming Chat

An Android app that runs a fine-tuned Qwen3.5-2B model on the phone with the [Cactus](https://github.com/cactus-compute/cactus) engine and answers bioaccumulator recipe requests through a deterministic recipe tool. Everything runs on the device; the only download is the model, once, on first launch.

The model, its training code and its benchmarks live elsewhere:

* Training and evaluation: [`CopyleftCultivars/qwen3.5-finetune`](https://github.com/CopyleftCultivars/qwen3.5-finetune), which uses this repository as the submodule `natural_farming_chat/`.
* On-device benchmark and the Python references the golden tests come from: [`CopyleftCultivars/bioaccumulator-benchmark`](https://github.com/CopyleftCultivars/bioaccumulator-benchmark).
* Weights: [`qwen3.5-2b-natural-farming-v5`](https://huggingface.co/CopyleftCultivars/qwen3.5-2b-natural-farming-v5) (bf16) and [`qwen3.5-2b-natural-farming-v5-cactus-cq4`](https://huggingface.co/CopyleftCultivars/qwen3.5-2b-natural-farming-v5-cactus-cq4) (what the app downloads).

## What the app does with a message

`lib/main.dart` sends each message through one pipeline (pure-Dart modules in `lib/services`, with golden tests generated from the Python references in the benchmark repository):

1. **Turn limit** (`turn_limit.dart`): sending stops at 20 finished turns, or when the last decision prompt was above 4,096 tokens. A "New chat" button replaces the text field.
2. **Routed history** (`routed_history.dart`): a trigger message is decided with an empty history; a bare area after a missing-area refusal is rebuilt into the trigger message; every other message sees a compact ledger of earlier recipe results.
3. **Decision** by the model, with all six tools.
4. **Argument guard** (`argument_grounding.dart`, `guard_aliases.dart`): a recipe call whose plant, crop or area was not in the conversation does not run.
5. **Tool executor**, then the model writes the instructions. Every amount comes from the recipe tool (`recipe_tool_service.dart`), never from the model.
6. **Answer check** (`answer_check.dart`): unit slips and missing assumption sentences are repaired from the tool result.

Logs carry counts and kinds only, never message text.

## Layout

| Path | What it is |
|---|---|
| `lib/`, `test/`, `assets/` | the app, its tests (with golden fixtures) and the bundled plant and recipe data |
| `android/`, `ios/` | Flutter runners. One Android flavor, `qwen35` (`com.copyleftcultivars.natural_farming_chat.qwen35`) |
| `packages/cactus/` | the Dart plugin for Cactus (cactus 1.3.0, a local fork of cactus-flutter for the v1.10 `libcactus.so` and the CACT weight format) |
| `cactus-sdk/` | **submodule**: the CopyleftCultivars fork of Cactus (branch `qwen35-rotary-fix`), with the Qwen3.5 graph-transpile and engine fixes the model needs |
| `third_party/memlocal_core/` | the memlocal Flutter plugin and the Rust crate it builds (upstream Apache-2.0, plus this project's changes; see `UPSTREAM.md` there). The app keeps its long-term memory in it |
| `tool/` | build, install, model staging and prompt-parity scripts |
| `native_libs.sha256` | checksums of the prebuilt Cactus libraries (below) |

## Getting started

```sh
git clone --recurse-submodules git@github.com:CopyleftCultivars/pixel7-cactus-qwen3.5.git
cd pixel7-cactus-qwen3.5
flutter test                       # about 15 s, no phone needed
```

Building the APK also needs the Android SDK and NDK, and a Rust toolchain: the Android build compiles the memlocal library through cargokit.

### Native libraries

`libcactus.so` and `libcactus_util.so` (arm64-v8a) are not in git. They are published as a release of the Cactus fork, and `native_libs.release` names the release this version of the app runs with:

```sh
tool/fetch_native_libs.sh      # downloads into packages/cactus/android/src/main/jniLibs/arm64-v8a/ and checks native_libs.sha256
```

`tool/check_native_libs.sh` (run by every build) compares what is there with `native_libs.sha256`: the build every benchmark and phone test so far used. The release is tagged on the fork at the `cactus-sdk` commit this repository pins, and this repository's version tags (`v*`) say which release a given app version belongs to.

**Known gap:** those two binaries were built outside this repository, and nothing records which commit produced them; the release is attached to the pinned commit because that is the fork state the app was developed against, not because the build was reproduced from it. `cactus-sdk/android/build.sh` builds the engine from the submodule, but a library built that way has not been compared with the released one or run on the phone. Until it has, the recorded checksums, not the submodule, are the reference.

### Build and install

```sh
tool/build_debug.sh   --install    # debug build (what the benchmark harness uses)
tool/build_release.sh --install    # release build (AOT, use this on a phone)
```

Both run `flutter test` first (`--skip-tests` skips it). `--device SERIAL` or `ANDROID_SERIAL` picks the phone. The release build is still signed with the debug key; a store release needs a real keystore (`android/app/build.gradle.kts`). There is no CI.

## The model

On first launch the app downloads the model (about 1.1 GB, so use Wi-Fi) from Hugging Face into its private storage. No login is needed.

* The app is pinned to one commit of the model repository (`hostedQwen35Model` in `lib/services/model_downloader.dart`), so a later push cannot change what a phone gets. To ship a new model: upload it, then change the pinned revision. Phones with the older revision replace their copy on the next launch.
* Every file is checked against the size and hash the repository lists (SHA-256, or the git blob SHA-1 for small files) before it is kept. An interrupted download resumes where it stopped; `config.txt`, the "installed" marker, is fetched last and the folder is moved into place in one step. If the download fails the loading screen shows the reason and a "Try again" button.

Developer and benchmark phones can skip the download by pushing the bundle once (`tool/stage_model.sh`, which needs the `outputs/` folders of the finetune repository). Choose the source when building:

```sh
tool/build_release.sh --install --model-source hosted   # always download (what a new user gets)
tool/build_release.sh --install --model-source staged   # only the pushed bundle
tool/build_release.sh --install                         # auto: pushed bundle if complete, else download
```

To test the real download against Hugging Face from a computer (about 1.1 GB):
`HF_NETWORK_TEST=1 flutter test test/model_downloader_network_test.dart`.

## Licenses

The app code in this repository is under the license in `LICENSE`. The Cactus engine (`cactus-sdk/`, `packages/cactus/`, and the `libcactus` binaries) is under Cactus Compute's own license (`cactus-sdk/LICENSE`, shipped next to the released binaries): free for individuals using it non-commercially, for educational institutions and 501(c)(3) non-profits, and for organizations with under $2 million in funding and under $2 million in annual revenue; everyone else needs a commercial license from Cactus Compute, and anyone who redistributes it must keep the license text with it. memlocal (`third_party/memlocal_core/`) is Apache-2.0. The model is Apache-2.0 (see its Hugging Face cards).

## Tests

`flutter test` runs the unit tests, including the golden files under `test/fixtures`. The on-device benchmark harness lives in the benchmark repository; it copies its own `integration_test` files into a disposable copy of this app.
