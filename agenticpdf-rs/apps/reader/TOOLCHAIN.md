<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
# Build toolchain

What the mobile builds need, and where it was installed on the machine this was
first built on (Windows). None of it is required for the **desktop** build,
which needs only a Rust toolchain.

## Installed

| What | Where | Size | How |
| --- | --- | --- | --- |
| Android SDK + NDK 27.2.12479018, platform-35, build-tools 35, platform-tools, emulator + system image | `%LOCALAPPDATA%\Android\Sdk` | ~9.3 GB | `winget install Google.AndroidCLI`, then `android sdk install ndk;27.2.12479018 platforms;android-35 build-tools;35.0.0 platform-tools` |
| Temurin JDK 21 + Gradle 8.10.2 | `%LOCALAPPDATA%\apdf-build-tools` | ~800 MB | Portable zips, extracted in place |
| Swift 6.3.3 | `%LOCALAPPDATA%\Programs\Swift` | — | `winget install Swift.Toolchain` |

The bulk of the Android figure is the emulator and its system image; a build-only
setup (NDK + build-tools) is a small fraction of it. Remove any of this with
`winget uninstall`, or by deleting the directory for the portable ones.

## Two things that bite

- **The Temurin MSI fails under `winget` with exit 1602** — it needs
  elevation. The portable zip from the Adoptium API works without admin:
  `https://api.adoptium.net/v3/binary/latest/21/ga/windows/x64/jdk/hotspot/normal/eclipse`
- **Gradle is not in `winget`.** Take the zip from
  `https://services.gradle.org/distributions/`. And AGP 8.7 rejects newer JDKs —
  this machine's system Java is 25, which is why a separate JDK 21 is needed.
  Point Gradle at it with `JAVA_HOME`.

## Building

```bash
# Android — see android/README.md
export ANDROID_NDK_HOME="$LOCALAPPDATA/Android/Sdk/ndk/27.2.12479018"
export JAVA_HOME="$LOCALAPPDATA/apdf-build-tools/jdk-21.0.12+8"
export ANDROID_HOME="$LOCALAPPDATA/Android/Sdk"
cd android && ./build-libs.sh && gradle assembleDebug --no-daemon

# Mobile web
wasm-pack build --target web --release --out-dir web/pkg

# iOS — macOS only; see ios/README.md
cd ios && ./build-libs.sh
```

Per-target linker configuration lives in `agenticpdf-rs/.cargo/config.toml`.
That file is part of the source, not a local artifact — the Android build needs
it. Build outputs (`android/app/src/main/jniLibs/`, `web/pkg/`, `ios/libs/`) are
gitignored.
