# Rendering architecture, and how it differs from PDF.js

PDF.js is this engine's reference renderer: correctness is measured as agreement
with it, page by page, over a corpus of real documents (see
[`HANDOFF.md`](HANDOFF.md) for the harness and the current numbers). So it is
worth being precise about what PDF.js actually does, where this engine follows
it, and where it cannot.

The short version: **the first half of the two designs is the same idea, and the
second half is not.** Both interpret a content stream into a flat list of
drawing operations. PDF.js then hands that list to a full 2D graphics API.
This engine hands it to a deliberately minimal one, and everything that API
cannot do has to be turned into geometry before it gets there.

---

## How PDF.js does it

*Described at the level this document needs. Details of its internals are
summarised from its published design rather than verified here, and the
comparison below does not depend on the fine points.*

**A parse/render split across a thread boundary.** A worker parses the file,
decodes images, and translates embedded fonts. It emits an *operator list* — a
flat array of opcodes and arguments. The main thread replays that list against a
`CanvasRenderingContext2D`.

**Canvas2D does the hard parts.** This single choice defines the rest of the
architecture:

- arbitrary clip paths — `ctx.clip()` takes any path, with a winding rule
- per-pixel alpha — `globalAlpha`
- blend modes — `globalCompositeOperation`
- axial and radial shadings — canvas gradient objects
- arbitrary image placement — `ctx.transform`, so a rotated or skewed image is
  the canvas's problem

**Soft masks and transparency groups become temporary canvases.** The group
renders into its own buffer, and the buffer is composited back. A luminosity
mask is a per-pixel operation on that buffer.

**Fonts go to the browser.** Embedded font programs are translated into
OpenType/CFF in the worker, installed as web fonts, and drawn with `fillText`.
The browser rasterises, hints and antialiases them. Type 3 fonts are the
exception, and necessarily so: a Type 3 glyph is a content stream, not an
outline, so its `/CharProcs` run as nested operator lists.

**Codecs are its own**, in JavaScript, with OpenJPEG compiled to WASM for JPEG
2000.

---

## How this engine does it

`agenticpdf-rs/src/engine.rs` interprets content streams into a `Vec<RenderOp>`.
That is an operator list by another name, and the resemblance is not accidental:

```rust
pub enum RenderOp {
    Fill { subpaths, color, even_odd },
    Stroke { subpaths, color, width },
    Text { text, x, y, size, advances, codes, .. },
    Image { x, y, w, h, name, alpha, tint, mat },
    Clip { rect, subpaths },
    Save,
    Restore,
}
```

The list is in PDF user space. `Transform::fit` maps it to a device rectangle at
paint time, so one list serves any zoom or surface.

### The constraint that shapes everything

The sink is not a 2D graphics API. It is the `Painter` trait from `dewey`, whose
relevant surface is:

```rust
fn fill_path(&mut self, points: &[Position], color: Color);   // one closed polygon
fn stroke_path(&mut self, points: &[Position], color: Color, width: f32);
fn draw_image(&mut self, rect: Rect, image: &ImageData);      // axis-aligned, no matrix
fn push_clip(&mut self, rect: Rect);                          // rectangles only
fn pop_clip(&mut self);
```

No winding rule. No subpaths. No clip shapes. No alpha groups, blend modes or
gradients. Backends implement this to reach a GPU surface, an image buffer, or a
JSON recording that a browser, Android or iOS host replays.

So every per-pixel effect Canvas2D performs has to be **decomposed into geometry
in the display list**, before it reaches a painter:

| Canvas2D does per-pixel | This engine decomposes |
| --- | --- |
| `ctx.clip()` on any path | the clip is carried as a polygon; `cut_to_shapes` clears an image's alpha outside it, `clip_to_convex` cuts shading bands |
| Nonzero / even-odd fill | `fill_winding` resolves subpaths and winding itself |
| Soft mask via a temporary canvas | the mask is sliced into cells of constant coverage (`MASK_GRID`), and every fill under it is cut once per cell |
| Canvas gradients | axial and radial shadings are emitted as bands; Coons and tensor patches, and type 4/5 triangle meshes, are diced into flat quads |
| Group `globalAlpha` | `fade_group` folds the group's alpha into the ops it produced |
| `ctx.transform` on an image | the image is resampled through its own matrix in `placed` |
| Image alpha and stencil tint | folded into the pixels by `recolour` |

### Text

Glyph outlines are parsed and rasterised here, not by a font engine: supersampled
scanlines with analytic horizontal coverage and non-zero winding, producing an
alpha mask cached by font, face object number, character code, Unicode,
quantised size, angle and colour. The mask is then drawn as an image.

This is the largest single departure from PDF.js, which delegates the whole job
to the browser's rasteriser.

### Codecs

CCITT, JBIG2 (with its MQ arithmetic decoder), JPEG and JPX are implemented in
the crate.

---

## Why the difference

The display list is not an implementation detail here; it is the product.

It has to serialise, cross a process boundary, and render the same way on a
desktop GPU surface, a headless image buffer, a browser canvas, Android and iOS.
A `ctx.clip()` call cannot cross that boundary. A polygon can.

It is also the substrate for everything the project does that is not painting:
text extraction, layout and table analysis, the semantic document model, and the
ADF container. A page has to be available as *data* that can be queried, not
only as pixels that can be looked at.

---

## What it costs

This is the honest trade, and it is worth stating plainly because it predicts
where the defects are.

**PDF.js buys compositing correctness cheaply.** Anything the canvas implements
is correct by delegation. This engine pays for the same correctness in
approximation — and the approximations are exactly where the recent defects
lived:

- A soft mask sliced into 16×16 cells of constant coverage could not make a drop
  shadow *fall off*: the outer cells averaged to something where the reference
  had almost nothing. Two pages carried 15 % too much ink for the page until the
  grid was quartered.
- A clip applied as its bounding box, rather than its shape, framed five
  photographs as rectangles on one brochure cover.
- Shading bands, mesh dicing and mask cells are all quantisations of something a
  canvas gets continuously.

**Still declined, with counts from the corpus census** (`what_the_corpus_declines`,
304 documents):

| Not implemented | Occurrences |
| --- | --- |
| Alpha (non-`/Luminosity`) soft masks | 6 |
| Strokes inside a masked group | 4 |
| Text as a clipping path (`Tr` 4–7) | see note below |

Text clipping is the one that needs the missing capability most directly: it
wants glyph *outlines* as a clip path, which is precisely what the painter
cannot take.

**What the trade buys:** no browser, no DOM and no GPU required; deterministic
output across five hosts; rendering in a headless process; and a page that can
be queried as a structure rather than only rasterised.

---

## A caveat about the measurement

Correctness here is defined as agreement with PDF.js, so a corpus where every
page matches means agreement with **one particular renderer's choices** —
including its antialiasing, and including its own approximations. It is a strong
reference and a practical one, but it is not ground truth; Acrobat would be a
different bar.

Two consequences worth keeping in mind:

1. A change can be more correct against the specification and still score worse
   against PDF.js.
2. The two remaining ways the harness itself can lie — a reference index that has
   gone stale, and a sweep that silently drops shards — are documented in
   [`HANDOFF.md`](HANDOFF.md). Both have produced a confident wrong number
   before.
