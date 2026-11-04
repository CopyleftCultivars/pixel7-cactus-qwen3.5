#!/usr/bin/env bash
# Check that the prebuilt Cactus libraries are where the Dart plugin loads them and are the ones the app was benchmarked with.
#
#   tool/check_native_libs.sh
#
# The libraries are not in git (57 MB and 19 MB). native_libs.sha256 records the SHA-256 of the build that was benchmarked on the
# Pixel 7 (candidate 4, Cactus CQ4 bundle); tool/fetch_native_libs.sh downloads them from the release named in native_libs.release
# into packages/cactus/android/src/main/jniLibs/arm64-v8a/.
set -euo pipefail
cd "$(dirname "$0")/.."
if ! sha256sum --check --quiet native_libs.sha256; then
  echo "The Cactus native libraries are missing or are not the recorded build (run tool/fetch_native_libs.sh; see README.md, 'Native libraries')." >&2
  exit 1
fi
echo "native libraries match native_libs.sha256"
