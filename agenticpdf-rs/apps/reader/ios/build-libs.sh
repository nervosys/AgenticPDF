#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Build the Rust core for iOS and combine the simulator slices into an XCFramework.
# Must run on macOS: linking needs the Xcode SDK.
set -euo pipefail

CRATE_ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
OUT="$(dirname "$0")/libs"
mkdir -p "$OUT"

for target in aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios; do
  echo "building $target"
  (cd "$CRATE_ROOT" && cargo build --release -p apdf-reader --lib --target "$target")
done

# The simulator slices share an architecture family with the device slice, so
# they cannot sit in one .a — an XCFramework is the only way to ship both.
lipo -create \
  "$CRATE_ROOT/target/aarch64-apple-ios-sim/release/libapdf_reader.a" \
  "$CRATE_ROOT/target/x86_64-apple-ios/release/libapdf_reader.a" \
  -output "$OUT/libapdf_reader_sim.a"

rm -rf "$OUT/APDFReader.xcframework"
xcodebuild -create-xcframework \
  -library "$CRATE_ROOT/target/aarch64-apple-ios/release/libapdf_reader.a" \
  -headers "$(dirname "$0")/Reader/apdf.h" \
  -library "$OUT/libapdf_reader_sim.a" \
  -headers "$(dirname "$0")/Reader/apdf.h" \
  -output "$OUT/APDFReader.xcframework"

echo "built $OUT/APDFReader.xcframework"
