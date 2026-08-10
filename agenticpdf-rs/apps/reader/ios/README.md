<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
# AgenticPDF Reader — iOS

Rust owns the document; Swift owns the window. Identical in shape to the
Android and web shells:

```
Session / actions / canvas        ← shared Rust core (desktop, web, Android, iOS)
        │
   RecordingPainter               ← paints into JSON instead of pixels
        │  C ABI
   PageView.draw(_:)              ← replays the recording onto a CGContext
```

`paint_page` is the same function every platform calls. `ViewController`
implements no document logic — each control calls `Reader.execute` with the same
action names the other shells and any driving agent use.

## Building

**Requires macOS with Xcode.** Linking an iOS binary needs the Apple SDK, which
does not exist on other platforms.

```bash
rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios
./build-libs.sh          # produces libs/APDFReader.xcframework
```

Then, in an Xcode app target:

1. Add `Reader/*.swift` to the target.
2. Add `libs/APDFReader.xcframework` under *Frameworks, Libraries, and
   Embedded Content*.
3. Set **Objective-C Bridging Header** to `Reader/Bridging-Header.h`.

## Status

**Verified on Windows:** the Rust core and `src/apple.rs` compile clean for both
`aarch64-apple-ios` and `aarch64-apple-ios-sim` (`cargo check`). The C surface
in `apdf.h` matches the `#[unsafe(no_mangle)] extern "C"` functions it declares.

**Not verified:** nothing here has been linked, compiled as Swift, or run — all
three need macOS. Treat the Swift as reviewed-but-unrun code. The equivalent
Android shell *was* run on a device, and doing so exposed two layout bugs that
review had missed (toolbar overflow, and content drawing under the system bars).
Expect the iOS shell to have its own such bugs until someone runs it; the safe
area handling in `ViewController.viewDidLoad` is the first thing to check.

## Ownership rule

Every C function returning `char *` transfers ownership. `Reader.take(_:)`
wraps all of them with a `defer { apdf_string_free(...) }`, so no call site has
to remember. Add new FFI calls through that helper.
