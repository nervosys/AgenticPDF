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
| Corpus | 285 reference sets, 710 pages, 681 comparable against PDF.js |
| Matching (total variation ≤ 0.12) | **673 of 681 — 98.8 %** |
| Over the threshold | 8 |
| Not comparable | 29 (no reference, or a page we decline to render) |
| Tests | 617 crate + 2 integration, 77 reader; clippy `-D warnings` clean, `cargo fmt` clean |
| Branch | `master`, pushed |

**Check the reference index before trusting a page count.** Each reference
directory holds a `source.txt` naming the document it was captured from, and
`compare_corpus` silently skips any whose file has moved. Twenty-two had, and
the sweep quietly reported 626 pages instead of 681 -- a fifth of the corpus
gone with no error anywhere. Relocating them by basename recovered twenty-one;
one document is genuinely gone. Re-run that check before reading any sweep as a
regression.

The corpus is `~/Documents` and `~/Desktop`, three pages a document, less
the 129 MB catalogue. It is not the same set of files the first table was
measured on -- the references were recaptured on 2026-08-26 -- so read the
percentage, not the difference in page counts.

Of the 14 failures, **none** has a mean absolute difference above 0.10. Three
sit between 0.04 and 0.10. The remaining eleven are below 0.04: sparse pages where the normalised score
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

**A predictor hiding behind a reference.** `decode_stream` reads
`/DecodeParms` by matching `Object::Dict`, and an indirect reference does not
match. The parameters were dropped in silence and the stream handed on *still
filtered*, with its per-row PNG filter byte in place -- so every row after the
first was read one byte late and the image sheared into noise. **Nothing
failed**: the length check passed, the image had pixels, and every diagnostic
this project owns reported it present and correct.

A returns label does exactly this on an 812-wide indexed image; its barcode was
a diagonal smear. 0.241 to **0.052**, 1 improved, 0 regressed. Every path that
holds a `Document` now decodes through `decode_stream_in`, which follows the
reference first; the plain form remains for the parser, which has no document
yet.

**This one is worth reading as a lesson about this handoff, not about PDF.**
That page was on the "sparse pages, threshold artefact, not a defect" list --
a claim this file has repeated for several revisions and nobody had ever
checked. The first one looked at was a real bug. The claim was not evidence, it
was a guess that had been written down often enough to look like one.


**A transparency group's alpha outlives the `gs` inside it.** The last page in
the corpus that was not a sparse-page artefact -- an ebook spread at 0.213 --
now scores **0.028**.

The page sets `ca 0.1`, draws a form that declares `/Group`, and the group's
first act is `/GS0 gs` with `ca 1`. We inherited the alpha into the group, so
that `gs` threw the fade away and a banner meant to be a 10 % wash painted at
full strength. A group is painted at full strength into its own buffer and the
*result* is composited with the alpha in force at the `Do` (PDF 11.6.6), so the
alpha belongs to the group as a whole -- which is exactly what this engine
already says about soft masks, in a comment, three lines away.

Inside the group the alpha now starts at 1 and what the group painted is faded
on the way out. A form *without* `/Group` is unchanged: it shares the page's
state and its `gs` genuinely does set the alpha. Both halves have a test, and
the contrast between them is the point.

Corpus: **7 improved, 1 regressed, net -0.304**; 671 to 672 of 681. The
regression is +0.005 on a page at 0.038.

The arithmetic was checked before a line was written, which is why this took one
attempt: pink `0.871 0.106 0.463` at `ca 0.1` over the page's own
`0.961 0.957 0.941` predicts (243, 222, 228) and the reference reads
(242, 221, 226); the purple band at `ca 0.2` predicts (221, 211, 231) against
(220, 210, 230). Two colours, six channels, all within one count.


**Dash patterns are drawn, and the corpus has nothing to say about it.** The `d`
operator was not implemented at all, so every dashed rule was drawn solid. It is
now applied in the geometry -- each stroke's polylines are cut into the
pattern's lit runs before they are emitted -- which is the choice already made
for gradient bands and clipped shading: every renderer draws it, none of them
learns anything.

Corpus: **11 pages improved, 0 regressed, net -0.052.** Small, and every one of
the eleven is a thin rule getting lighter, which is what a dash is. Counted
against a binary that actually contains the change, over both corpus trees:
**298** patterns set across 304 documents, **316** strokes painted while one is
in force, 16,483 pieces emitted. (`~/Documents` 219/237/15,567, `~/Desktop`
79/79/916 -- quote both, or the count is of half a corpus.)

**That number is a correction.** The commit that introduced this claimed the
corpus effect was "exactly zero across all 681 pages". It was not; the sweep
behind that claim ran against a **stale binary** -- its build had failed with
`LNK1104` because a previous sweep still held the executable open, and the
script carried on and swept the old one anyway. Re-measured against a binary
that actually contains the change, eleven pages move.

Two counting mistakes sit behind that, and both are worth keeping:

- The first census said 517 strokes were painted under a live dash. The counter
  was a plain `bool` outside the `q`/`Q` stack, so it stayed true past the `Q`
  that popped the dash. **A counter for a piece of graphics state has to be
  graphics state.**
- The corrected counter then said *zero*, which the sweep contradicted. That
  reading was taken against the same stale binary; re-run against a current one
  it says 237. **When a counter and a sweep disagree, the sweep is the
  measurement** -- a counter is a hypothesis about where to look, and here the
  sweep was right twice while the counter was wrong twice in opposite
  directions.


**A spot colour is the ink, not the coverage.** A Separation names one colorant
and carries a *tint*; the engine read that tint as ink coverage on white, so
full tint was black. Right sense, often wrong value -- and where it is wrong it
is very wrong: a catalogue sets a fifth of its page in one Pantone at full
tint, and that ink is an orange.

The tint transform is now evaluated. `shading::eval` already did functions of
one variable, which is exactly what a Separation's transform is, so this is
mostly plumbing: the space carries its function and what the function's output
means, and the colour comes out the far end. DeviceN keeps the coverage
reading -- its function takes one input per colorant and `eval` takes one
input -- and so does anything whose transform cannot be evaluated.

**The alternate space's component count is not enough to read its numbers.**
The first version assumed three components meant RGB. A Lab alternate also has
three, running 0..100 and -128..127, and a datasheet's PANTONE 2768 C came out
as `rgba(11.37, 6.00, -31.00)` -- not a colour, and worth **+0.073** on that
page, the largest single regression of the session. Lab is now converted
properly, through XYZ with the Bradford-adapted D50 matrix and the sRGB curve,
and anything still unrecognised falls back to coverage rather than being read
as if it were RGB.

Corpus: **22 improved, 1 regressed, net -0.572**; 668 to 671 of 681. Insurance
forms, brochures, datasheets and the catalogue all moved, three of them across
the threshold. The one regression is +0.011 on a page at 0.073, well inside it.


**A clip is a shape, not a box.** A catalogue page went from 0.236 to 0.010 --
it was the worst dense page left -- and the cause was two places that had
quietly replaced a clip path with the rectangle around it.

The page clips both a gradient and a full-width photograph to the same wedge:
`(0,119) (0,0) (612,0) (612,413)`, a trapezoid whose slanted edge crosses the
whole page. Its bounding box is nearly twice its area.

*In the engine*, `sh` fills the current clip, and the current clip was carried
as four numbers. The clip is now also kept as a polygon while it is a single
convex one, and each band is cut to it before it is emitted -- which is the same
argument that made gradients into bands in the first place: doing it once in the
geometry fixes every renderer and none of them has to learn anything.

*In the painter*, the shape was carried only when a subpath had more than five
points, on the reasoning that a four-cornered path "is already described by the
rectangle". That is true of an axis-aligned rectangle and of nothing else. A
trapezoid has four corners. So the photograph was drawn across the whole box,
over artwork the page had clipped it away from.

Corpus: 3 improved, 4 regressed, net -0.235, and one more page inside the
threshold. **The four regressions are real and small** -- +0.006 each, on
journal covers whose logos are clipped to rotated squares that we now honour and
whose edges we draw hard where the reference antialiases. They are the price of
clipping correctly, and they are named rather than buried.

*Rejected on the way, both measured:* sixteen-sample coverage antialiasing for
the polygon clip, which changed the four pages it was written for by **nothing**
and made two corpus shards slow enough to stall -- it is O(pixels x shapes x
vertices). And a half-point "too small to see" tolerance on the rectangle test,
which over 681 pages also changed nothing, so the tolerance is back to being
slack for arithmetic rather than a judgement.


**The two pages nobody could explain are explained.** Both were found the same
way -- by dumping our render and the reference as coarse ASCII and looking at
them side by side -- and neither was findable from a score. A page can be
badly wrong and barely move the ink metric, and both of these did.

*The journal page (0.328, the corpus's worst, now 0.066).* Its figures and
equation strips are ordinary images carrying `/Mask <n> 0 R`, a separate
one-bit stencil saying which of their pixels are painted. Every stencil is
CCITT fax-coded, and the mask reader took its samples straight from
`decode_stream` -- which passes CCITT through untouched, deliberately, because
it exists to get text out of Flate streams and says so in a comment. The mask
arrived still compressed, 279 bytes where a 669x29 stencil needs 2425,
`unpack_samples` refused it, and the mask was dropped. With no mask the image
painted its own opaque background and each figure became a flat grey rectangle.
`images_without_pixels` was satisfied throughout: the image was not missing, it
was wrong.

*The prepress spread (0.208, now 0.025).* Half the page rendered white. Thirteen
full-half-page separation plates, all near-white, all opaque, all placed at
exactly the same spot over the artwork. The reason they are allowed to be opaque
is that the page sets its headline in `7 Tr` -- add to clipping path, paint
nothing -- and draws every plate through the resulting glyph-shaped clip. We do
not implement text clipping, so every plate painted its full rectangle and
erased what was under it. Four earlier hypotheses had been ruled out here (a
soft mask on the group, a blend mode, optional content, an undecodable texture),
and two more were ruled out this round: no blend mode is in force at any image
on that page (counted, zero), and applying the soft masks that *are* there moves
it not at all.

**Soft masks drawn with a picture are read.** Counted before anything was
written: of 46 soft masks across 270 documents, 36 were declined, every one
because the mask is painted with a grayscale image rather than with fills --
which is simply how every drawing tool exports a graded mask. They are now
sampled into a 16x16 grid of constant-coverage cells, walked in the image's own
unit square so a rotated placement needs no inversion, and collapsed to one
region when the mask is a single flat tone. Masks built: 10 of 46 before, 45 of
46 after. Corpus: 2 improved, 0 regressed.

**Mesh shadings types 4 and 5.** Free-form and lattice triangle meshes are
diced into flat sub-triangles the way patches are diced into quads, sharing one
header reader with types 6 and 7. **The corpus contains neither**, so the flat
sweep after this change is not evidence; the synthetic tests are. The one
difference between the two readers that fails silently has its own test: a
free-form mesh pads every vertex to a byte and a lattice mesh does not, and
aligning both puts the mesh somewhere else entirely.

**Full-page pictures reach the mobile shells.** An image past the 6 MB inline
bound was sent as geometry with no pixels and no key, for the host to resolve
itself. Only the browser ever did. So the one image on a page big enough to trip
the bound -- the full-page background photograph -- was the one that never
arrived on Android and iOS. It is now sampled one step further to fit the bound.
No host-side change, so iOS is fixed too without a Mac.

**Verified on the Android emulator**, which is what the previous round's lesson
demanded: a brochure cover whose full-page background photograph was absent now
shows it, behind the five hexagon-framed photographs that the earlier texture
fix restored. One observation worth keeping rather than burying: opening that
16.7 MB catalogue raised an "isn't responding" dialog for about half a minute
before the page appeared. The emulator renders through swiftshader, so this is
not by itself evidence of a real-device problem -- but decoding a full-page
photograph is not obviously off the UI thread, and nobody has checked.

**The 129 MB catalogue does not reproduce.** All 300 pages build their display
list and decode their images in about a tenth of a second each, no failures. It
decodes only what it draws, so the "decodes the whole document per page"
hypothesis is wrong. The likeliest explanation for the original intermittent
abort is that the disk was full, so Windows could not grow its page file --
free RAM was never the binding constraint, which is why "aborts with 46 GB free"
read as a mystery. **Check free disk before believing an allocation failure on
this machine.**


**Partial alpha on images.** `RenderOp::Image` carries the constant alpha it was
drawn under, defaulting to 1 so an older display list still reads as opaque
rather than invisible. Dewey's painter has no opacity argument, so a faded image
is faded in its own alpha channel — the same thing under the straight-alpha
compositing every host does — and the WebGL renderer scales the texel's alpha in
the fragment shader. The fade happens before the content key is taken, or the
first placement of a picture would win and every later one would be drawn at
whatever opacity that one had.

**Both mobile shells were passing no textures at all.** See the open list for
what remains; the fix itself was three lines each and is verified on Android by
running it.

**An image was clipped to its clip path's bounding box.** `Painter` clips to a
rectangle, so a non-rectangular clip has always been applied as its box. For a
fill that is survivable; for an image it is not. One brochure frames five
photographs in hexagons and every one came out as a full rectangle with a
hexagon drawn on top. An image's pixels are now masked by the shape, folded
into the same copy that applies alpha, tint and placement. **21 pages improved,
none regressed, net -0.201** — and the hexagon cover moves only 0.131 → 0.128
for it, which is the usual story about a metric that measures where ink is
rather than what shape it is in.

The first attempt was **net -0.9 across eighteen regressed pages**: it treated
each subpath of a clip path as a region of its own and demanded a point be
inside all of them. One clip path is one region however many subpaths draw it —
a ring is two circles and a point between them is inside. One page went 0.034 →
0.555. *The per-page diff caught it; the summary count would not have*, because
twenty-one other pages improved in the same run.

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

Ordered by what it costs, cheapest first. Everything the previous revision of
this section listed has been either fixed or measured and rejected; what
follows is what is actually left.

**iOS.** Unverifiable here: it needs macOS. One thing that did change is that
the full-page-picture fix below required no host-side change, so the iOS shell
gets it without being rebuilt. The texture fix from the previous round is still
unconfirmed on iOS and still three lines against the same shared function.

**The web recording's volume.** 672 KB per steady-state frame for a text page,
74 % of it 5,325 per-glyph ops at 92 bytes each. A positional encoding with a
key table would cut that roughly threefold. **Still blocked for the same reason,
and it is a good one:** the iOS and Android replayers parse this schema field by
field, and iOS cannot be built or run on this machine, so a re-encoding would
break a host silently and nothing here could tell. This is not a task waiting
for effort; it is waiting for a Mac.

**Text clipping is declined, not implemented.** `Tr` 4-7 confine everything
painted after `ET` to the glyph outlines. This engine has no glyph outlines --
text leaves as a `Text` op and the painter rasterises it -- so an image under a
text clip is declined rather than painted over the whole rectangle it would
otherwise fill. That is the right trade (see *Done*), but it does lose the
duotone headline it was meant to draw. Implementing it properly means glyph
outlines in the engine or a new op every renderer must learn, which is the cost
this project already refused for gradients.

**A possible ANR on a large document.** See *Done*: the Android shell showed an
"isn't responding" dialog for roughly thirty seconds while opening a 16.7 MB
catalogue, then rendered it correctly. Measured on a software-rendered emulator,
so it may be nothing; the question nobody has asked is whether a full-page image
decode runs on the UI thread.

**Fills under a text clip are still painted.** Only images are declined. Nothing
in the corpus exercises a fill under a text clip, so nothing was written for it;
the same counter that found the image case would find this one.

**Soft masks on text.** Text inside a masked group still paints at full
strength. The census counts this at **zero** across 270 documents, so there is
nothing to measure a change against and none was written.

**Mesh types 4 and 5 are implemented and unmeasured.** No document in the corpus
uses one, so the sweep after them is flat by construction. They are verified by
synthetic tests against decoded geometry instead. Treat the corpus as silent on
them, not as endorsing them. Everything the previous revision of
this section listed has been fixed, or measured and rejected; what follows is
what is actually left.

**iOS.** Unverifiable here: it needs macOS. One thing did change in its favour
-- the full-page-picture fix needed no host-side change, so the iOS shell gets
it without being rebuilt, and it is verified on Android. The texture fix from the previous round is still
unconfirmed there, and still three lines against the same shared function.

**The web recording's volume.** 672 KB per steady-state frame for a text page,
74 % of it 5,325 per-glyph ops at 92 bytes each. A positional encoding with a
key table would cut that roughly threefold. **Still blocked, for the same good
reason:** the iOS and Android replayers parse this schema field by field, and
iOS cannot be built or run on this machine, so a re-encoding would break a host
silently and nothing here could tell. This is not waiting for effort; it is
waiting for a Mac.

**Text clipping is declined, not implemented.** `Tr` 4-7 confine everything
painted after `ET` to the glyph outlines. This engine has no glyph outlines --
text leaves as a `Text` op and the painter rasterises it -- so an image under a
text clip is now declined rather than painted across the whole rectangle it
would otherwise fill. That is the right trade and it fixed the standing mystery
page, but it does lose the duotone headline it was meant to draw. Doing it
properly means glyph outlines in the engine, or a new op every renderer has to
learn, which is the cost this project already refused for gradients.

**Fills under a text clip are still painted.** Only images are declined. Nothing
in the corpus exercises a fill under a text clip, so nothing was written for it.
The counter that found the image case would find this one.

**Soft masks on text.** Text inside a masked group still paints at full
strength. The census counts this at **zero** across 270 documents, so there is
nothing to measure a change against and none was written.

**Soft masks on a directly drawn image.** A mask applies to everything painted
under it, not only to a transparency group, and a mask set immediately before an
image currently does nothing -- eleven times across the corpus. It was
implemented and taken out again: drawing the image once per mask region inside a
clip of that region moved 681 pages by **-0.007**, one better by 0.014 and two
worse by 0.005 and 0.003. A wash. The counter (`mask_ignored_at_image`) stays so
the question can be re-asked against a corpus that exercises it properly.

**Mesh types 4 and 5 are implemented and unmeasured.** No document in the corpus
uses one, so the sweep after them is flat by construction rather than by
evidence. They are verified by synthetic tests against decoded geometry instead.
Treat the corpus as silent on them, not as endorsing them.

**A fill or stroke painted under a live soft mask is unmasked.** The mask is
applied only where a transparency group is composited, and the spec applies it
to everything painted. Now counted: **2 fills** across the 34-document tree,
zero strokes, zero images. Small, and no longer invisible.

**DeviceN still reads as coverage.** Its tint transform takes one input per
colorant and this engine evaluates functions of one variable. Nothing in the
corpus made it matter; the Separation case did, twenty-two times.

**The worst pages left are sparse ones** -- *probably*. The claim now has one
data point behind it and one against: the highest-ink page on the list turned
out to be a genuine decode bug, and the rest have **not** been looked at. Every
remaining failure has a mean absolute difference under 0.014, which is the
argument for the artefact reading, but that is exactly what was said about the
returns label. Look before believing it. An order-details receipt p1 at 0.266
(mae 0.004), a returns label at 0.241 (mae 0.033), a technical reference p3 at
0.239 (mae 0.012). Every one of them has a *mean absolute difference under
0.04*: the normalised score is dividing by very little ink, which is the
artefact the threshold note below describes rather than a broken page. The
densest failure left is a catalogue p2 at 0.156 (mae 0.043), which is the same
document whose p3 this round fixed and is the one worth looking at next.

**Hard edges on a polygon clip.** Where a clip is a shape rather than a box, the
painter masks pixel by pixel with no coverage, so a small rotated shape comes
out jagged against a reference that antialiases. Antialiasing it was tried and
measured at zero (see *Done*); the four +0.006 regressions this round are this,
and a cheaper approach than sixteen samples a pixel would probably collect
them.

## Needs a decision

*Resolved since this section was written:* the work **is** pushed. `master` now
carries all of it, and `fix/security-hardening` is kept in sync as a mirror
rather than a branch waiting to merge, so the naming complaint below is moot in
practice. What is not resolved is that **`master` still has no branch
protection**, so nothing forced a review of 23,000 changed lines and nothing
will force one on the next batch either. That is the item to act on.

Still genuinely open, and each is an owner's call rather than an engineer's:

- **Annotation appearance streams.** See *Open*: rendering them is right for our
  reader and reads as divergence in this harness, so the measurement cannot
  decide it.
- **A FIPS-capable build**, if it is a product requirement. `SECURITY_AUDIT.md`
  section 3.1 lists the four steps; if it is *not* a requirement, saying so in
  `SECURITY.md` stops the question being asked again.
- **The 129 MB catalogue**, currently excluded from the sweep. Either it gets
  diagnosed or the exclusion gets written into the harness with a reason, rather
  than living in a shell variable.

## Security, since this file was written

The seven findings the audit had been carrying (F-001 through F-007) were
re-checked against the source instead of restated. **Four were already fixed**
and had been carried forward as open by a revision that never looked; three were
real and are now closed. The one that mattered was F-007: a `firstName` split
out of author metadata went straight into `new RegExp()`, so a crafted document
chose the regex. It is escaped now.

Both npm projects report **zero** advisories. That did not need either breaking
upgrade the audit had assumed: root's six HIGH were one `minimatch` ReDoS
counted again at each `@typescript-eslint` level above it, and the website's
HIGH plus moderate were one `postcss` pin inside `next`. Two `overrides`
entries, both inside the existing major, cleared all eight. **Read the advisory
graph before quoting its count** — `effects` and `via` are the fields that turn
eight findings back into two.

The Rust side keeps two HIGH ignores, and they are triage rather than silencing:
`quick-xml` 0.30.0 arrives only through `accesskit_unix`, the Linux AT-SPI
backend, which parses D-Bus introspection XML and never a document. The chain is
written out in `.github/workflows/rust.yml` along with the condition for
removing the ignores.

## Traps this work already fell into

Each of these cost real time. They are written down so the next person does not
pay for them twice.

**Look at the pixels.** Both pages that had resisted explanation for rounds gave
themselves up in minutes to a coarse ASCII dump of our render beside the
reference. One showed flat (127,127,127) blocks where figures belonged; the
other showed an empty left half. No aggregate, and no diagnostic this project
owns, said either of those things -- `images_without_pixels` was satisfied in
both cases, because nothing was missing, it was wrong.

**A diagnostic that reports zero is only as good as the question it asks.** The
whole class of "the mask was silently abandoned" was invisible because every
counter asked whether an image had pixels. Abandoned masks are now counted, by
the codec that beat them.

**Attribute an improvement before reverting the change that caused it.** Two
changes went in together and the pair improved seven pages. The second was then
isolated and measured at a wash, so both were pulled -- and three of the seven
improvements went with them, because they belonged to the first. Reverting a
bundle is not reverting the thing you measured.

**Never sweep without checking the binary was rebuilt.** Twice this session a
measurement was taken against a stale executable: once because
`cargo test --no-run` failed with `LNK1104` -- a previous sweep still held the
`.exe` open -- and the script swept the old binary anyway, and once because a
build was still running when the sweep started. Both produced a confident
"nothing changed". A sweep script should fail loudly when its build does, and
until it does, check the binary's timestamp before believing a flat result.

**When a counter and a sweep disagree, believe the sweep.** A counter is a
hypothesis about where to look; the sweep is the measurement. The dash counter
said zero strokes were painted under a live pattern and the corpus said eleven
pages moved.

**A census over a directory that does not exist reports zeros**, and zeros look
exactly like a real answer. Two probes this session were run against a
`mkdir`/`cp` that had landed somewhere else, and both "no soft masks, no
shadings" readings were fiction. The diagnostic prints how many documents it
found; do not grep that line away.

**A counter for graphics state has to be graphics state.** The dash census
said 517 strokes were painted under a live pattern. The counter was a plain
`bool` outside the `q`/`Q` stack, so it stayed true past the `Q` that popped the
dash and counted every later stroke. The real number is **zero**. A diagnostic
that does not read the same state the painter reads will answer a different
question, confidently.

**Count uses, not definitions.** "The whole file contains one `/BM /Multiply`"
is a fact about the byte string, not about how often it is in force -- one
ExtGState can be referenced on every draw. The right form of the question is a
counter at the point of painting, which said zero and closed the hypothesis for
good.

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
