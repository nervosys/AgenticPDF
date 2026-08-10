#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Build the Rust core for every Android ABI and stage the results where Gradle
# expects them. Run before `gradle assembleDebug`.
#
# Requires ANDROID_NDK_HOME and the NDK's clang wrappers on PATH; the linker
# names are configured in agenticpdf-rs/.cargo/config.toml.
set -euo pipefail

: "${ANDROID_NDK_HOME:?set ANDROID_NDK_HOME to your NDK, e.g. .../Sdk/ndk/27.2.12479018}"
export PATH="$PATH:$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/windows-x86_64/bin"

CRATE_ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
JNI_LIBS="$(dirname "$0")/app/src/main/jniLibs"

# Rust target -> Android ABI directory name.
declare -A ABIS=(
  [aarch64-linux-android]=arm64-v8a
  [armv7-linux-androideabi]=armeabi-v7a
  [x86_64-linux-android]=x86_64
)

for target in "${!ABIS[@]}"; do
  echo "building $target"
  (cd "$CRATE_ROOT" && cargo build --release -p apdf-reader --lib --target "$target")
  mkdir -p "$JNI_LIBS/${ABIS[$target]}"
  cp "$CRATE_ROOT/target/$target/release/libapdf_reader.so" "$JNI_LIBS/${ABIS[$target]}/"
done

echo "staged into $JNI_LIBS"
