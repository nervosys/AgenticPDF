<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
# AgenticPDF Reader — Android

The Android shell. Rust owns the document; Kotlin owns the window.

## How it fits together

```
Session / actions / canvas        ← shared Rust core (also desktop + web)
        │
   RecordingPainter               ← paints into JSON instead of pixels
        │  JNI
   PageView.onDraw                ← replays the recording onto android.graphics.Canvas
```

`paint_page` is the same function the desktop binary and the browser call. The
platform layer is a switch over drawing primitives, which is why `PageView` is
short and why the three platforms cannot drift apart in how a page looks.

`MainActivity` never implements a document operation itself — every button
calls `Reader.execute` with the same action names the web shell and any driving
agent use (`open`, `search`, `insert_block`, `save`, `export`, …). Run
`Reader.capabilities()` for the list.

## Building

**1. The native libraries** (this part is verified working):

```bash
export ANDROID_NDK_HOME=~/AppData/Local/Android/Sdk/ndk/27.2.12479018
./build-libs.sh
```

Builds `arm64-v8a`, `armeabi-v7a` and `x86_64` and stages them into
`app/src/main/jniLibs/`. Linker configuration lives in
`agenticpdf-rs/.cargo/config.toml`; the NDK's clang wrappers already know their
sysroots, so nothing else needs setting.

Cargo builds the core, not Gradle. That keeps `cargo build` the single way to
build the Rust and stops an APK embedding a library Gradle rebuilt differently.

**2. The APK:**

```bash
gradle assembleDebug          # or ./gradlew once a wrapper is committed
```

### Toolchain requirements for step 2

Step 2 has **not** been run in this checkout. It needs two things that are not
installed here:

- **Gradle** — not available through `winget`; install from
  <https://gradle.org/releases/> or commit a Gradle wrapper.
- **A JDK between 17 and 21.** Android Gradle Plugin 8.7 does not support
  newer ones, and the JDK on this machine is **Java 25**. Point Gradle at a
  supported JDK with `org.gradle.java.home` or `JAVA_HOME`.

Already installed and working: NDK 27.2.12479018, platform 35, build-tools 35,
platform-tools 37.

## What is verified

- The three `.so` files link and export the six `Java_ai_nervosys_apdf_Reader_*`
  symbols that `Reader.kt` declares (`llvm-nm -D` on the arm64 build).
- The core compiles for all three Android ABIs with no warnings.
- The same core, through the same `RecordingPainter`, is exercised end to end by
  the web shell's browser test — so the recording format and the page painting
  are tested even though the Kotlin replay is not.

## What is not verified

The Kotlin code has never been compiled or run: that needs step 2. Treat
`MainActivity`, `PageView` and `Reader.kt` as reviewed-but-unrun code. The JNI
signatures are the risky part, and they are the part checked against the
exported symbols above.
