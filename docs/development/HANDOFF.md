<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
# Handoff — `fix/security-hardening`

Written 2026-08-26. This is the state of the render-correctness work and
everything a person needs to pick it up: what is done, how to reproduce every
measurement, what is open, and what needs a human decision.

The living render-correctness report is the artifact *AgenticPDF Render Status*;
it goes into the **findings** in detail. This file is about the **work** — the
harnesses, the branch, the traps.

## Where it stands

| | |
| --- | --- |
| Corpus | 288 documents, 719 pages, 691 comparable against PDF.js |
| Matching (total variation ≤ 0.12) | **657 of 691 — 95.1 %** |
| Over the threshold | 34 |
| Not comparable | 28 (no reference, or a page we decline to render) |
| Tests | 633 crate, 67 reader; clippy `-D warnings` clean, `cargo fmt --check` clean |
| Branch | 63 commits ahead of `master`, unpushed |

Of the 34 failures, four have a mean absolute difference above 0.10 — those are
the only ones a reader would call visibly wrong. Nine sit between 0.04 and 0.10.
The remaining twenty-one are below 0.04: sparse pages where the normalised score
divides by very little ink, so a shadow edge or a thin band scores like a broken
page. **That is not an argument for moving the threshold** — the 0.12 line was
calibrated on 91 dense technical pages and is doing something different on a
corpus that includes receipts. Reporting both numbers together is what keeps the
difference visible.

## Reproducing the measurements

Everything below runs from `agenticpdf-rs/`.

**Capture PDF.js references.** One Firefox instance for the whole corpus; each
page is saved as a PPM beside a `source.txt` naming the document.

```bash
node render/pdfjs-refs.mjs <list-file-or-dir> <outdir> [pages-per-file] [width]
```

`width` is an *upper bound*, not a target. Do not lower it to save disk:
downscaling the reference inflates the score, because our side is then rendered
at the same reduced width and thin strokes land differently on both sides. One
page measured 0.111 against a 640-wide reference and 0.031 against the native
one.

**Compare.** A corpus of any size is one long single-threaded walk, so shard it:

```bash
APDF_CORPUS=<refs-dir> APDF_SHARD=k/6 \
  cargo test --release --lib -p apdf-reader -- --ignored compare_corpus --nocapture
```

Six shards side by side finish in a few minutes. Each prints its own
`compared N, within 0.12: M, not comparable K` line; sum them.

**Look at one page.** These are the diagnostics that actually find bugs:

```bash
APDF_RENDER_PDF=<file> APDF_RENDER_PAGE=n cargo test -- --ignored why_fallback
APDF_RENDER_PDF=<file> APDF_IMAGE_OUT=out.ppm cargo test -- --ignored write_page_image
APDF_RENDER_PDF=<file> cargo test -- --ignored glyphs_land_in_their_run
```

**Run the desktop app.** Launch with PowerShell `Start-Process`, not a
backgrounded shell job — a `&`-spawned process is reaped when the tool call that
started it is cleaned up, and the timing looks exactly like a crash on whatever
input was sent just before. Drive it with synthetic mouse clicks
(`SetCursorPos` + `mouse_event`); `SendKeys` does not reach the window even when
`AppActivate` returns true. Capture with `PrintWindow(h, hdc, 2)`.

**Build and run Android.** This machine *does* have the SDK, NDK, JDK 21, Gradle
and an x86_64 AVD — see `apps/reader/TOOLCHAIN.md`. The emulator is x86_64, so
build that target and not only arm64:

```bash
export ANDROID_NDK_HOME="$LOCALAPPDATA/Android/Sdk/ndk/27.2.12479018"
export PATH="$PATH:$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/windows-x86_64/bin"
cargo build --release -p apdf-reader --lib --target x86_64-linux-android
cp target/x86_64-linux-android/release/libapdf_reader.so \
   apps/reader/android/app/src/main/jniLibs/x86_64/

cd apps/reader/android
export JAVA_HOME="$LOCALAPPDATA/apdf-build-tools/jdk-21.0.12+8"
export ANDROID_HOME="$LOCALAPPDATA/Android/Sdk"
"$LOCALAPPDATA/apdf-build-tools/gradle-8.10.2/bin/gradle" assembleDebug --no-daemon
```

Then `emulator -avd medium_phone -no-snapshot-load -gpu swiftshader_indirect`,
`adb install -r`, `adb push` a PDF to `/sdcard/Download/`, and broadcast
`MEDIA_SCANNER_SCAN_FILE` for it — a pushed file is invisible to the system file
picker until MediaStore indexes it. Under Git Bash set `MSYS2_ARG_CONV_EXCL='*'`,
or every `/sdcard/...` argument is rewritten into a Windows path.

iOS still cannot be built here: it needs macOS.

## Open

Ordered by what it costs, cheapest first.

**Partial alpha on images.** An image carries no alpha of its own in the display
list. Fully transparent images are suppressed, which is what the corpus asked
for, but an image at `ca 0.5` is still drawn opaque. Needs an alpha on the op and
three renderers taught to honour it.

**Soft masks on text and images.** Fills inside a transparency group composite
through the group's soft mask. Text and images inside one still paint at full
strength, and a mask whose own group paints with text or an image produces no
regions and is skipped. Both fail unmasked, which is the safe direction.

**Page textures on mobile.** Confirmed by running the Android app rather than
inferred: text now renders correctly, including the font-subset fix, but the
handbook cover's photograph is absent — the page shows its gradient over white.
Neither mobile host resolves page images.

**Annotation appearance streams.** Collected for extraction, never rendered:
every form field, stamp, signature and highlight is invisible in our output.
**This needs a decision before it needs code.** PDF.js draws form widgets into an
HTML layer rather than the canvas, so rendering them would read as divergence in
this harness while being right for our own reader.

**Mesh shadings (types 4–7).** Coons and tensor patches, declined rather than
approximated. Exactly one document in the corpus draws its artwork with them, and
stays at 0.872 and 0.568.

**Web recording volume.** Measured, not estimated: 672 KB per steady-state frame
for a text page, 74 % of it 5,325 per-glyph ops at 92 bytes each. A positional
encoding with a key table would cut that roughly threefold — but the iOS and
Android replayers parse this schema field by field, and iOS cannot be built here,
so a re-encoding would break a host silently. Mobile is worse than the web figure
besides: its native side builds a fresh painter every frame, so every mask's
pixels are re-sent every time.

**A 129 MB catalogue.** Intermittently pathological: the same page builds its
display list in seconds on one run and aborts on an allocation failure on the
next, with 46 GB free. Currently excluded from the sweep.

## Needs a decision

- **The branch name is wrong.** `fix/security-hardening` holds 63 commits, of
  which exactly one is about security. It started there and grew. It should be
  renamed or split before it merges.
- **Nothing is pushed, and `master` has no branch protection**, so there is
  nothing forcing review of 23,000 changed lines.

## Traps this work already fell into

Each of these cost real time. They are written down so the next person does not
pay for them twice.

- **A good `compare_corpus` score is not evidence that text is correct.** The
  metric measures where ink is, not which glyph put it there. One cover scored
  0.010 while rendering its title as "qhe pingle-Board Computer e andbook". 691
  automated comparisons did not find that and structurally could not; opening the
  application and reading the page did, in about a minute.
- **A paint change is not shippable for being more correct in principle.** Run
  the full sweep *and diff per page*. Three separate changes sat at or near
  net-zero while one document improved and another regressed.
- **"I built it and it did nothing" is evidence about the implementation**, not
  always about the hypothesis. Soft masks were reverted once as unverified before
  the real problem was found — they were applied per-fill instead of at the
  transparency-group boundary. Check whether the code ever ran on a document that
  should exercise it before discarding the idea.
- **Never bisect on the 129 MB catalogue.** It aborts non-deterministically, and
  a single-run bisect wrongly implicated the line-width commit.
- **This machine mislays files.** A directory listing has returned a Rust
  toolchain as missing while a search listed files inside it. Retry before
  concluding anything is gone.
- **`/BaseFont` is not a font identity.** A document may embed two subsets under
  one name. Text runs carry the object number of the font dictionary they were
  set in for exactly this reason.
- **A `.gitignore` pattern without a leading slash matches at every depth.** A
  bare `app/` under the Electron heading kept the entire website source and the
  whole Android application out of version control, and neither had ever been
  committed.
