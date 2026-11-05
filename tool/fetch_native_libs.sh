#!/usr/bin/env bash
# Download the prebuilt Cactus native libraries (libcactus.so, libcactus_util.so, arm64-v8a) from the release named in
# native_libs.release and check them against native_libs.sha256.
#
#   tool/fetch_native_libs.sh
#
# The release lives on the CopyleftCultivars Cactus fork, which is a public repository, so no login is needed. Files that already
# match the recorded checksums are not downloaded again.
set -euo pipefail
cd "$(dirname "$0")/.."
# shellcheck source=/dev/null
source native_libs.release

dest=packages/cactus/android/src/main/jniLibs/arm64-v8a
mkdir -p "$dest"

if sha256sum --check --quiet native_libs.sha256 >/dev/null 2>&1; then
  echo "native libraries already match native_libs.sha256"
  exit 0
fi

if command -v curl >/dev/null; then
  get() { curl --fail --location --silent --show-error --output "$2" "$1"; }
elif command -v wget >/dev/null; then
  get() { wget --quiet --output-document="$2" "$1"; }
else
  echo "need curl or wget" >&2
  exit 1
fi

for name in libcactus.so libcactus_util.so; do
  url="https://github.com/${NATIVE_LIBS_REPO}/releases/download/${NATIVE_LIBS_TAG}/${name}"
  echo "downloading ${name} from release ${NATIVE_LIBS_TAG}"
  get "$url" "$dest/${name}.download"
  mv "$dest/${name}.download" "$dest/${name}"
done

if ! sha256sum --check --quiet native_libs.sha256; then
  echo "downloaded libraries do not match native_libs.sha256; not using them" >&2
  exit 1
fi
echo "native libraries match native_libs.sha256"
