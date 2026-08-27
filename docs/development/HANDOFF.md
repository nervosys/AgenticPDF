<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
# Handoff — `fix/security-hardening`

Written 2026-08-26, revised the same day. This is the state of the render-correctness work and
everything a person needs to pick it up: what is done, how to reproduce every
measurement, what is open, and what needs a human decision.

The living render-correctness report is the artifact *AgenticPDF Render Status*;
it goes into the **findings** in detail. This file is about the **work** — the
harnesses, the branch, the traps.

## Where it stands

| | |
| --- | --- |
| Corpus | 290 documents, 711 pages, 683 comparable against PDF.js |
| Matching (total variation ≤ 0.12) | **666 of 683 — 97.5 %** |
| Over the threshold | 17 |
| Not comparable | 28 (no reference, or a page we decline to render) |
| Tests | 595 crate + 45 integration, 73 reader; clippy `-D warnings` clean, `cargo fmt --check` clean |
| Branch | 79 commits ahead of `master`, unpushed |

The corpus is `~/Documents` and `~/Desktop`, three pages a document, less
the 129 MB catalogue. It is not the same set of files the first table was
measured on -- the references were recaptured on 2026-08-26 -- so read the
percentage, not the difference in page counts.

Of the 17 failures, **none** has a mean absolute difference above 0.10. Four
sit between 0.04 and 0.10. The remaining thirteen are below 0.04: sparse pages where the normalised score
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

Each shard prints only its worst thirty rows, which is what a person reads and
is useless for the per-page diff below. `APDF_LIST=100000` prints every page:
run it on both revisions and diff the two sets of rows by name. A page that
regresses from 0.01 to 0.05 never enters anyone's top thirty.

Give the capture Windows-style paths (`C:/Users/...`). A list of MSYS paths
(`/c/Users/...`) reaches Firefox unchanged and every single file is skipped with
`NS_ERROR_FILE_NOT_FOUND`, which reads as a corpus of unopenable documents.

**Look at one page.** These are the diagnostics that actually find bugs:

```bash
APDF_RENDER_PDF=<file> APDF_RENDER_PAGE=n cargo test -- --ignored why_fallback
APDF_RENDER_PDF=<file> APDF_IMAGE_OUT=out.ppm cargo test -- --ignored write_page_image
APDF_RENDER_PDF=<file> cargo test -- --ignored glyphs_land_in_their_run
```

**Look at every page at once.** Two corpus-wide diagnostics answer questions the
ink metric structurally cannot, and both take a tree of documents rather than a
tree of references:

```bash
APDF_CORPUS_PDFS=<dir> cargo test --release -- --ignored images_without_pixels
APDF_CORPUS_PDFS=<dir> cargo test --release -- --ignored images_drawn_faded
```

`images_without_pixels` counts images a page places and the painter has no
texture for, so it draws a grey frame instead. A whole class of undecodable
image can go missing across a corpus without a single test failing and without
the ink metric moving, because a frame has ink of its own; run it on two
revisions and diff the totals. `images_drawn_faded` lists every image drawn at
less than full opacity — ask it whether the corpus exercises a transparency
change at all before reading a flat sweep as "this did nothing".

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

## Done since this file was written

**Partial alpha on images.** `RenderOp::Image` carries the constant alpha it was
drawn under, defaulting to 1 so an older display list still reads as opaque
rather than invisible. Dewey's painter has no opacity argument, so a faded image
is faded in its own alpha channel — the same thing under the straight-alpha
compositing every host does — and the WebGL renderer scales the texel's alpha in
the fragment shader. The fade happens before the content key is taken, or the
first placement of a picture would win and every later one would be drawn at
whatever opacity that one had.

**Mesh shadings were declined, and the gap cost more than a guess would
have.** The reasoning was that a mesh guessed at wrongly is worse than a gap.
That had never been checked against what a gap costs. One infographic ebook
draws its backgrounds with tensor patches and sets its contents page in
**white** on top: declining the mesh lost the background and thirty-four lines
of text together. Coons and tensor patches are now diced into flat quads, the
same trade this module already makes for axial and radial gradients. The four
interior control points a type 7 adds are read past rather than used — an
approximation, said plainly. **2 pages improved, none regressed, net -1.396**,
and they were the two worst in the corpus: 0.872 → **0.020** and 0.568 →
**0.024**.

`shading_in` had been keeping only the shading's *dictionary*, so a mesh had
nothing to be drawn from even once there was code to draw it — the geometry and
the corner colours are entirely in the stream.

**An image was placed by its bounding box rather than its own transform.** A
PDF image is painted into the *unit square* under the current transform, which
is free to turn it, mirror it or shear it; the op carried only a box. One eBook
places a photograph with a quarter turn and a mirror, and it came out lying on
its side — while the page's ink barely moved, 0.4226 against 0.4089, because a
grid of ink mass cannot see which way up a photograph is. The op now carries
`[a b c d]` and the painter resamples anything not upright by walking the output
and asking where each pixel came from — one routine for mirrors, turns and
shears alike, because enumerating the cases is how a sign ends up backwards.
**13 pages improved, none regressed, net -1.069.** Sized before it was built:
`images_placed_askew` counts 59 non-upright placements against 5,564 upright.

**A run was answered by a stranger's subset.** Naming a face object is also a
statement about the others: a run set in object 19 is not set in object 12. The
lookup put the named face first and then fell through to *every* embedded face
sharing the `/BaseFont` name, so a run whose own object carries no font program
at all got a different object's subset — by name, with the document's codes.
One AMD guide has three objects called `TimesNewRomanPS-BoldMT`, one embedded
and two not, and its contents page is set in one of the two: "List of Tables"
came out as "i ist of qables" and every dot leader as a solid black bar.
**35 pages improved, none regressed, net -1.449** — the largest on the branch.
The page went 0.459 → 0.042. This is the *third* time `/BaseFont` has not been
an identity and the first two fixes both stopped one step short.

**A substituted run was laid out on the stand-in's widths, not the
document's.** One ratio for the whole run — its recorded width over the
stand-in's natural width — which only means anything when every character is
off by about the same amount. A line of dot leaders is not: `Author . . . . .`
is one run of 134 characters, six of them letters, and the stand-in's period is
far wider than the document's. The ratio came out at **0.54** because the dots
outvoted everything, and the word was drawn at 54 % spacing — an unreadable blot
beside a leader that looked perfectly fine. Both paths now advance by the
document's own per-glyph widths; the stand-in's widths keep only the job of
dividing a cluster's single advance among its components. **74 pages improved,
none regressed, net -0.999** — the cleanest result of the branch, and it came
from looking at a page rather than at a score.

**Pattern fills were painting solid black.** `scn` in a pattern space names a
resource rather than giving numbers, and read as a colour it fell through to
black — 442 fills across 24 of 294 documents. Tiling patterns are now run as
forms, clipped to the path the fill drew and tiled by `/XStep`/`/YStep` under a
cap; shading patterns are sliced into bands like `sh`; anything else paints
nothing, because a hole is the safe direction and black is not. **20 pages
improved, 4 regressed by ≤ 0.004, net -0.958.**

Three separate mistakes hid one another, and the order is the lesson.
`/Pattern cs` names the space *directly*, where every other space is looked up
in the resource dictionary — so the pattern was never recognised at all. A
tiling pattern's images live in the pattern's own resources, so the texture
walk had to go there or the page names a texture nobody has. And a shading
pattern is a plain **dictionary**, not a stream, because the gradient is its
whole definition; requiring a stream rejected every one of them, and that
surfaced only because a page *regressed* in the sweep and got looked at.

**A nested clip was widening the one around it.** The painter pushed each clip
rectangle as it stood instead of intersecting it with what was already in
force, so a clip could add paint rather than remove it. Found through patterns,
where it is unmissable — one cover drew nine copies of the same photograph
across the page and over its own title — but it was never specific to them.

**Inline images: lexed as content, then not drawn, and both are fixed.**
`BI ... ID <bytes> EI` had no handling at all, so the raw samples went through
the content lexer, where a `(` swallows everything to the next unbalanced `)`
and the short operator names turn up constantly. They are stepped over now, and
then decoded and drawn — dictionary rebuilt from the tokens in front of the
samples, keyed by a hash of them so the display list and the texture walk agree
without either knowing where the other looked.

Also `true` and `false` were lexed as **operators**, and an operator clears the
operand stack, so `/IM true` wiped the dictionary being accumulated in front of
it. They are keywords and are now operands. That one is not specific to inline
images.

**None of it moved a single page, and the number that motivated it was wrong.**
A grep over inflated streams said 2,742 inline images across nine documents.
The lexer says **five**, in two documents, over the first eight pages — the
samples are binary and two of them spell `BI` often enough to fool a grep. All
683 comparable pages are unchanged to three decimals, and so are all sixteen
pages of the two documents that really do contain them, whose references were
captured to eight pages to check exactly that.

**Image masks were dropped rather than stencilled.** An `/ImageMask` is one
bit a sample with no colour space, painted in the fill colour in force at the
`Do`. Nothing matched it, so it decoded to nothing and drew a grey frame. The
texture now carries coverage in its alpha and the colour rides on the op — a
per-object texture cannot hold it, because the same stencil can be painted
twice on a page in two colours. Six pages improved, none regressed, net -0.030;
images placed but undecodable are now **0 of 1,301**.

**Predicted colour images were being dropped.** A PNG predictor's row is
`/Columns` *samples* wide and its filters subtract the pixel to the left, which
is `/Colors` components away. The un-filter read `/Columns` as bytes and never
looked at `/Colors` at all, so a colour image's row stride came out three times
too small, the length check rejected the stream, and the picture was discarded.
It stayed hidden because the one predicted stream every PDF is guaranteed to
have — a cross-reference stream — is one component of eight bits, where the old
arithmetic is right. Images placed but undecodable fell from 43 across 14
documents to 2 in one; 15 pages improved, one regressed by 0.003, net -0.628.

## Open

Ordered by what it costs, cheapest first.

**Soft masks on text and images.** Fills inside a transparency group composite
through the group's soft mask. Text and images inside one still paint at full
strength, and a mask whose own group paints with text or an image produces no
regions and is skipped. Both fail unmasked, which is the safe direction.

**A half-page of artwork that nothing has explained.**
`design-article-optimizing-bldc-motor-control` p3 (on the *Desktop*) is the
worst page in the corpus that is not the mesh-shading ebook: 0.208, absolute
difference 0.096, and its **entire left half renders white** where the
reference shows a title, body text and a large product photograph.

What is measured, not guessed: the page places thirteen full-half-page images.
The first, `4128`, decodes to real artwork (mean RGB 214). The twelve after it
decode **fully opaque** — not one transparent pixel between them — at mean RGB
253–254, so each paints a white sheet over what came before. Their streams are
9 KB for 7.7 M samples, so they really are near-uniform.

Four causes were checked and are **not** it:
- *A soft mask on the group.* Masking images inside a transparency group was
  implemented and measured: the branch fired **zero** times across 60 documents,
  and the change moved no page. Reverted as unverified code that can only remove
  paint — the same call this repository made once before.
- *A blend mode.* The whole file contains one `/BM /Multiply`.
- *Optional content.* No `/OCProperties`, no `/OCGs`, no `/OC` on the XObjects.
- *An undecodable texture.* Every one of the thirteen resolves; the diagnostic
  reports 1 image without pixels in the entire Desktop tree, in another file.

**One page nothing has explained.** `atmosphere-10-00549` p3 came down from
0.469 to 0.328 with pattern fills and is still the third-worst page in the
corpus. Its 32 image ops carry no stencil tint, so whatever it masks is not
reached as a page-level image XObject; it is *not* inline images, which the
document contains none of. `ADA617071` p1 came off this list — it was pattern
fills, and it now scores 0.009. It lives on the **Desktop**, not in
`~/Documents`.

**The cover text is a substitution difference, not a defect.** It was written
up here as one. `ADA617071` names `/ArialNarrow` and `/BookmanOldStyle,Italic`
with **no `/FontFile`** and supplies narrow `/Widths` (228, 456, 547…). We
substitute a condensed face and honour those widths; PDF.js has no Arial Narrow
and substitutes regular Arial. Ours is the closer of the two to what the
document asked for. Nothing to fix; chasing the reference here would make us
*less* faithful.

**Page textures on mobile.** Confirmed by running the Android app rather than
inferred: text now renders correctly, including the font-subset fix, but the
handbook cover's photograph is absent — the page shows its gradient over white.
Neither mobile host resolves page images.

**Annotation appearance streams.** Collected for extraction, never rendered:
every form field, stamp, signature and highlight is invisible in our output.
**This needs a decision before it needs code.** PDF.js draws form widgets into an
HTML layer rather than the canvas, so rendering them would read as divergence in
this harness while being right for our own reader.

**Mesh shadings of types 4 and 5.** Free-form and lattice triangle meshes are
still declined; no document in the corpus uses one. Types 6 and 7 are drawn.

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

- **The branch name is wrong.** `fix/security-hardening` holds 65 commits, of
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
- **Never count a content-stream construct with a grep.** Inline images were
  reported as 2,742 across nine documents from a regex over inflated streams.
  The real figure, counted by the lexer, is *five* in two documents: image
  samples are binary and two bytes spell `BI` often. `inline_images_reached`
  exists for this. The same doubt applies to any `BI`/`ID`/`EI`-shaped search.
- **A visual difference is not automatically our defect.** The reference is one
  more renderer, not the truth. A cover whose title looked wrong beside PDF.js
  turned out to be a document naming a font nobody embedded: we substitute a
  condensed face and honour the document's own narrow widths, PDF.js
  substitutes regular Arial. Check what the document *asks for* before calling
  a divergence a bug — twice now that check has changed the answer.
- **Check whether a document actually contains the thing you blamed.** The
  0.469 page was written up as probably inline images; it contains none. One
  grep over its inflated streams settled it, and would have settled it before
  the claim was written down rather than after.
- **A missing picture is nearly invisible to the ink metric.** The painter draws
  a grey frame where a texture will not decode, and a frame has ink of its own,
  in roughly the right place. Forty-three dropped images across fourteen
  documents moved the sweep by three pages. Count the images, do not infer them
  from the score.
- **A transparency change can be right and still move nothing.** Of 270
  documents, four pages draw a faded image at all. Before reading a flat sweep
  as evidence against a change, ask `images_drawn_faded` whether the corpus
  contains a single document that exercises it.
- **`/BaseFont` is not a font identity.** A document may embed two subsets under
  one name. Text runs carry the object number of the font dictionary they were
  set in for exactly this reason.
- **A `.gitignore` pattern without a leading slash matches at every depth.** A
  bare `app/` under the Electron heading kept the entire website source and the
  whole Android application out of version control, and neither had ever been
  committed.
