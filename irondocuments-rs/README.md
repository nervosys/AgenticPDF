# IronDocuments — Rust CLI & WASM

A self-contained, **zero-runtime** document engine for agentic AI workloads:
text, metadata, reading-order Markdown, structured layout with bounding boxes,
semantic chunks, tables, images, annotations, and outlines — as a single static
binary or a WASM module. No JVM, no Python, no model server.

PDF is where it goes deepest, but it is not PDF-only: the same commands read
Word, Excel, PowerPoint, EPUB, RTF, HTML, Markdown and CSV — including
**rendering them**, since formats that carry no geometry of their own get it
computed by the built-in typesetter.

## Supported formats

Format is identified from **content, not file extension** — a `.docx` renamed to
`.pdf` is still read as a `.docx`. The extension is consulted only to break ties
inside the plain-text family, where the bytes genuinely are ambiguous.

| Format | Extensions | Status |
| --- | --- | --- |
| **Agentic Document Format** | `.adf` | ✅ **read *and written*** — chunk-indexed random access, embedded retrieval index, per-block provenance, CRDT edit log. [See below](#adf--the-agentic-document-format). |
| PDF | `.pdf` | ✅ full — authored geometry, rendering, tables, figures, formulas, forms, OCR |
| Word (OOXML) | `.docx` `.docm` | ✅ styles, headings, numbering, tables + table styles, hyperlinks, images, text boxes, footnotes and endnotes, hidden text; typeset + rendered |
| Excel (OOXML) | `.xlsx` `.xlsm` | ✅ sheets, shared strings, sparse cells, cell types, dates, formula results; hidden sheets, rows and columns skipped; typeset + rendered |
| PowerPoint (OOXML) | `.pptx` `.pptm` | ✅ slides in presentation order, titles, bullets, tables + table styles, speaker notes, images; hidden shapes and slides flagged; one page per slide |
| EPUB | `.epub` | ✅ spine order, Dublin Core metadata, per-chapter sections, images, the stylesheets the package carries |
| Rich Text Format | `.rtf` | ✅ stylesheet headings and list depth, tables incl. nested, lists, footnotes, text boxes, fields and hyperlinks, colour table, per-font code pages, `\u` escapes, tracked deletions excluded, hidden text |
| HTML / XHTML | `.html` `.htm` `.xhtml` | ✅ structure, tables, links, images, declared charset, entities, the document's own `<style>` rules for emphasis and hidden-text scan |
| Markdown | `.md` `.markdown` | ✅ round-trips through the model |
| Delimited text | `.csv` `.tsv` | ✅ RFC 4180 quoting, inferred delimiter |
| Plain text | `.txt` | ✅ |
| OpenDocument Text | `.odt` | ✅ named styles + parent chains, headings, lists incl. inherited depth, tables + table templates, text boxes and shapes, footnotes, tracked deletions excluded, hidden text |
| OpenDocument Sheet | `.ods` | ✅ sheets, repeat-count expansion, typed and displayed values; hidden sheets, rows and columns skipped; cell comments excluded |
| OpenDocument Slides | `.odp` | ✅ slides, title placeholders, tables + table templates, notes, one page per slide; page numbers, dates and running heads skipped |
| Word 97-2003 | `.doc` | ✅ piece table, style chains, SPRM deltas, lists, tables, footnotes and endnotes, text boxes placed at their anchor, fields and hyperlinks, run colour, tracked deletions excluded, hidden text |
| Excel 97-2003 | `.xls` | ✅ BIFF8 records, shared strings, RK values, dates, formula results; hidden sheets, rows and columns skipped |
| PowerPoint 97-2003 | `.ppt` | ✅ persist-resolved slide order, titles, outline levels and character formatting, tables, hyperlinks, pictures, notes; hidden shapes and slides flagged |

`irondoc formats` prints this table with each format's capabilities;
`irondoc describe` includes the same data as JSON for agent discovery.

**Every format the engine detects, it can also parse** — the compiler enforces
that, since the format dispatch has no catch-all arm. Writing is deliberately
narrower: ADF, Markdown, HTML, text and JSON. There are no OOXML or ODF writers,
and the ontology says so rather than letting an agent attempt a save that cannot
work.

### Two models, one API

A PDF carries *geometry* and no structure, so its structure is inferred from
where the glyphs sit. HTML, Markdown and Office carry *structure* and no
geometry, so their pages are computed by the typesetter. Every format ends up
with both models, and `Document` uses whichever is authoritative per capability:

- Where a format states its own structure, that is used — heading levels, list
  numbering, merged table cells and hidden-text flags come from the source
  rather than from font-size heuristics, which makes their Markdown strictly
  better than a PDF's.
- Where a format states its own geometry, that is used. Where it does not, the
  typesetter computes it — so `layout` bounding boxes, `displaylist` and GPU
  rendering work for a `.docx` exactly as they do for a PDF.

Computed coordinates are the typesetter's own: a `.docx` has no single correct
pagination, and Word would place the same text differently. They are, however,
*internally consistent* — the same font metrics that choose the line breaks
position the glyphs, so the box a citation reports is the box a renderer draws.

A handful of capabilities remain PDF-only because they read PDF structures that
a computed page has no equivalent of: `forms` (AcroForm fields), `outline`,
`images` (embedded XObjects) and `scanned`. Those name the format rather than
returning an empty result.

## ADF — the Agentic Document Format

Every format above was designed for a word processor, a printer or a browser.
ADF is the one this engine writes, and it is designed for an agent. Four
differences follow from that:

- **Seek, don't scan.** A 64-byte header and a fixed-stride chunk table.
  Sections, pages, assets and embeddings are found by offset, so answering a
  question about page 400 of a large document touches three chunks rather than
  reading the file. Chunk lookup, strings, assets, provenance rows and
  embeddings are copy-free views into the bytes; block content is decoded on
  access, per section, because the semantic model is a recursive Rust enum
  rather than a flat record.
- **The index travels with the document.** Retrieval chunks, an inverted term
  index, and optionally their embeddings, live *inside* the file. A document is
  searchable the moment it is opened — no embedding pass, no side database —
  and the index cannot go stale relative to its content, because changing the
  content rewrites it.
- **Citations can be checked.** Each imported block keeps a content hash and
  the source document, page and bounding box it came from. A quotation is
  reported as **matching**, **drifted** or **unrecorded** — never guessed.
- **Edits are an append-only log.** Concurrent human and agent edits merge by
  set union, so the result does not depend on arrival order. Ordering is RGA;
  content conflicts resolve last-writer-wins by **Lamport clock**, never the
  wall clock. Granularity is the block: edits to different paragraphs always
  merge, two edits to the same paragraph discard one. Every operation records
  its author, including whether it was a model. Saving an edit appends the
  edit, not the document.

```bash
# Import anything. Provenance is recorded per block as it is imported.
irondoc convert report.docx --to adf --output report.adf
irondoc markdown report.adf                       # read it back

# Search. ADF answers from the index inside the file; every other format is
# scanned — deliberately the same command, because an agent should not have to
# know what it is holding before it can look something up.
irondoc search report.adf "revenue emea"
irondoc search report.adf "revenue emea" --json   # [{section, block, text}, …]

# Check a quotation. Exits 0 on a match, 1 otherwise, so a script or an agent
# can gate on it without parsing anything.
irondoc verify report.adf "Revenue grew by 12% across EMEA." --section 0 --block 1
#   matched   — the text is exactly what was imported
#   drifted   — the block exists, its text has changed since
#   unrecorded — no provenance was stored for that block
```

### One format, several kinds of document

ADF carries a **profile** in its header saying what kind of document it holds.
One container, one extension, one set of guarantees; the profile decides which
chunks are inside it. Reading it costs the header alone, so a tool can pick a
decoder before paying for anything else.

| Profile | Holds | Chunks |
| --- | --- | --- |
| `document` | Sections of blocks — the original ADF, and the default | `Section`, `Footnotes`, `Asset` |
| `spreadsheet` | Sheets of typed cells | `Workbook`, `Sheet` |

The profile lives in the word the v1.1 header reserved, so a file written
before profiles existed reads back as `document`, which is what it was, and a
v1.1 reader opens a v1.2 file with the chunks it does not know skipped.

### The spreadsheet profile

A spreadsheet used to arrive as a table of strings: `=SUM(B2:B9)` came through
as `1783.5`, `007` came through as the number seven, and a date came through as
`46095` or as text, depending on a format code that was then thrown away. Under
this profile a cell keeps what it is:

- **The type.** Number, text, boolean or one of the seven spreadsheet errors.
  `42` and `"42"` are different cells and only one of them can be summed.
- **The formula.** The expression *and* the result the source cached for it, so
  a reader that cannot evaluate still shows what the author saw.
- **The address.** Cells are sparse and sorted by `(row, column)`, so a sheet
  with one cell at `ZZ10000` costs one cell rather than ten million.
- **The parts that are not content.** Number formats, merges, frozen panes,
  column widths, defined names, and which rows, columns and sheets the author
  hid.

```bash
# Read a spreadsheet as cells rather than as text.
irondoc sheet budget.xlsx
#   Ledger — 7 rows x 3 columns, 17 cells
#     A6       Total
#     B6       =SUM(B3:B4)  -> 813.75
#     C7       =1/0         -> #DIV/0!

irondoc sheet budget.xlsx --sheet Ledger --format json
#   {"sheets":[{"name":"Ledger","cells":[
#     {"row":5,"column":1,"value":{"type":"number","value":813.75},
#      "formula":"SUM(B3:B4)"}, …

# Import it. A spreadsheet becomes cells; everything else becomes blocks.
irondoc convert budget.xlsx --to adf --output budget.adf
irondoc sheet budget.adf              # the cells, back out
irondoc markdown budget.adf           # and still a document
irondoc search budget.adf "power"     # rows are indexed, so figures are findable
```

Retrieval and provenance work on a workbook the way they work on prose, at the
granularity of a **row**: a figure on its own is neither searchable text nor a
citable answer, while the row it sits in carries its label and the figures
beside it. A search hit names the sheet and the row, and `verify` checks a
quoted row against what was imported — so a number an agent reports from a
spreadsheet is as checkable as a sentence it quotes from a report.

Hidden sheets, rows and columns are **kept in the cells and kept out of the
text**. Dropping them would mean a conversion silently writing out a workbook
missing part of the one it read; rendering them would show a reader what the
author chose to conceal. They are stored, marked, and excluded from every text
view — the same treatment the readers of the source formats have always given
them.

### Recalculation

Storing a formula is only half of it. An agent that changes a cell needs to know
what that changes, and an agent handed a spreadsheet needs to know whether the
numbers in it still follow from the formulas beside them:

```bash
irondoc recalc budget.xlsx
#   3 formula(s) evaluated
#   Every stored result follows from its formula.

irondoc recalc stale.xlsx
#   12 formula(s) evaluated
#     Ledger!B6: stored 500, computes to 5
```

A disagreement means the stored result no longer follows from its own formula —
the cell is stale, or it was edited by hand. Nothing is written back: the file
is audited rather than repaired, because overwriting the values a source cached
would destroy the evidence that they had drifted.

The evaluator resolves **dependencies rather than positions**, so a total above
its inputs is as correct as one below them. It covers arithmetic with the usual
precedence (`^` right-associative), comparisons, `&`, percentages, absolute and
sheet-qualified references, ranges, defined names, and a function library of
`SUM`, `PRODUCT`, `AVERAGE`, `MIN`, `MAX`, `COUNT`, `COUNTA`, `IF`, `IFERROR`,
`AND`, `OR`, `NOT`, `ABS`, `SQRT`, `INT`, `ROUND`, `LEN`, `LEFT`, `RIGHT`,
`MID`, `UPPER`, `LOWER`, `TRIM`, `CONCAT` and `NA`.

What it will not do is guess. A **circular reference is reported, not
resolved** — the cell keeps the value the file cached, because replacing it
with a zero would substitute this build's invention for the source's answer. An
**unknown function is `#NAME?`**, exactly as a spreadsheet answers one, and the
rest of the sheet still computes. A formula that will not parse is listed
rather than swallowed. Numbers compare with a relative tolerance, so a workbook
that stores `0.3` for `0.1 + 0.2` is reported as right, not as drifted.

Reading `.xlsx` into cells is implemented. `.ods` and the legacy `.xls` still
take the text path, and writing a spreadsheet back out to `.xlsx` is not
implemented at all — ADF remains the only format this engine writes.

## The reader app

`apps/reader/` is an agentic-first document reader and editor built on
[Dewey](https://github.com/nervosys/Dewey), NERVOSYS's Rust GUI framework, whose
ontology makes a running program discoverable by agents.

It reads all 17 formats, edits and round-trips ADF, and exports Markdown, HTML
and text. **Desktop and Android run; mobile web runs as a wasm bundle; iOS
compiles but has not been built or run — that needs macOS.**

The two properties that shape it:

- **One page-painting implementation.** `canvas::paint_page` emits through
  Dewey's `Painter`. Desktop rasterises directly; the browser, Android and iOS
  replay a recorded JSON form of the same calls, so no platform can drift in how
  a page looks.
- **One command vocabulary.** The desktop UI, the web shell, the Android shell
  and any agent all go through the same `Session` and the same 12 actions. A
  capability cannot exist for one caller and not the others — which is what
  keeps the ontology an accurate description of the program rather than a
  parallel document that rots.

```bash
cargo run --release -p irondoc-reader -- report.pdf
irondoc-reader --capabilities        # the agent-facing ontology, no window opened
```

## Why this over a JVM/Python pipeline

| | IronDocuments (Rust) | Typical Java/Python loaders |
| --- | --- | --- |
| Runtime | none — single static binary / WASM | JVM 11+ or Python 3.10+ |
| Cold start | milliseconds | seconds (JVM/interp warmup) |
| Edge / serverless / browser | ✅ (WASM, ~no deps) | ✗ / awkward |
| Tagged-PDF structure (`/StructTreeRoot`) | ✅ author-provided tree | ✅ |
| Reading order, headings, lists | ✅ deterministic, local | often needs an AI backend |
| Tables (bordered + borderless + panels) | ✅ rulings + text-alignment, local | often needs an AI backend |
| Figure ↔ caption linking | ✅ image bbox + caption proximity | varies |
| Formula → LaTeX (best-effort) | ✅ symbols, super/subscripts, fractions | usually needs an AI backend |
| Scanned detection + OCR pipeline | ✅ detection always; pixel decode behind `ocr` | bundled OCR |
| Prompt-injection / hidden-text scan | ✅ off-page + tiny-text + `--sanitize` | rarely |
| Citation bounding boxes | ✅ `[left, bottom, right, top]` | varies |
| Agent integration | JSON-LD ontology + **built-in MCP server** | usually none |

The engine is a real object model — a lexer, recursive-descent object parser,
cross-reference resolver (classic **xref tables** *and* PDF 1.5+ **xref
streams**), **object-stream** decompression, page-tree traversal with inherited
attributes, and content-stream text extraction with Unicode decoding
(**ToUnicode CMaps**, WinAnsi + `/Differences`, Identity-H composite fonts).

## Build

### CLI (native binary)

```bash
cargo build --release
# binaries: target/release/irondoc and target/release/idoc (same CLI, two names)

# With the optional image-decode pipeline for OCR backends:
cargo build --release --features ocr
```

### WASM (browser / edge)

```bash
cargo install wasm-pack
wasm-pack build --target web --features wasm --no-default-features
```

## CLI Usage

```bash
# Reading-order Markdown for an LLM context window
irondoc markdown paper.pdf --output paper.md
irondoc markdown paper.pdf --pages 1-3

# Structured, citation-ready layout (typed blocks + bounding boxes)
irondoc layout paper.pdf --output layout.json

# Reconstruct tables (bordered, booktabs, borderless) → Markdown or JSON
irondoc table report.pdf
irondoc table report.pdf --format json --output tables.json

# Tagged-PDF logical structure tree (author-provided, no heuristics)
irondoc structure tagged.pdf

# Detect hidden / off-page text (prompt-injection defense)
irondoc scan untrusted.pdf
irondoc markdown untrusted.pdf --sanitize   # strip hidden text from output

# Detect figures and link them to captions
irondoc figures paper.pdf

# Reconstruct best-effort LaTeX for formulas
irondoc formula paper.pdf

# Flag likely-scanned pages that need OCR
irondoc scanned paper.pdf

# OCR scanned pages (requires `--features ocr`)
irondoc ocr scanned.pdf --lang eng                       # bundled Tesseract CLI
irondoc ocr scanned.pdf --server http://localhost:8868/ocr  # PaddleOCR/EasyOCR server
irondoc ocr scanned.pdf --vlm http://localhost:8000/v1/chat/completions  # PaddleOCR-VL-1.6 (VLM)

# Run as an MCP server for agent clients (Claude Desktop, etc.)
irondoc mcp

# Any supported format, same commands
irondoc markdown report.docx
irondoc table budget.xlsx
irondoc layout deck.pptx
irondoc displaylist notes.md --page 1   # GPU display list for a typeset document

# Convert to another representation
irondoc convert report.html --to markdown
irondoc convert report.html --to json --output report.json
irondoc convert untrusted.html --to markdown --sanitize

# What can this build read?
irondoc formats
irondoc formats --format json

# Plain or positioned text
irondoc text document.pdf
irondoc text document.pdf --pages 1-5 --format json

# Metadata, outline, annotations, images
irondoc meta document.pdf
irondoc outline document.pdf
irondoc annotations document.pdf
irondoc images document.pdf            # dimensions, color space, filter, size

# Semantic chunks for RAG
irondoc chunk document.pdf --size 500 --overlap 50 --format json

# Everything in one pass: metadata, text, outline, chunks, Markdown, tables,
# figures, formulas, and a prompt-injection scan — one call, full understanding
irondoc all document.pdf --output full.json

# Machine-readable capability discovery for agents
irondoc describe                       # JSON-LD ontology
irondoc info
```

### `layout` output (per block)

```json
{
  "kind": "heading",
  "text": "Inverting Trojans in LLMs",
  "level": 2,
  "page_number": 1,
  "bbox": [214.25, 664.44, 429.44, 681.66],
  "font_size": 17.21
}
```

`kind` is one of `heading` | `paragraph` | `list_item`. Multi-column pages are
read column-by-column via a lightweight XY-cut split. `bbox` is in PDF points
(`[left, bottom, right, top]`) for precise source citations.

## WASM API

```javascript
import init, {
  toMarkdown, toLayout, extractTables, extractFigures, extractFormulas,
  detectScanned, scanInjection, extractText, parsePdfMetadata,
  generateChunks, listImages,
} from './pkg/irondocuments.js';

await init();
const bytes = new Uint8Array(await (await fetch('document.pdf')).arrayBuffer());

const md      = toMarkdown(bytes);                 // reading-order Markdown
const layout  = JSON.parse(toLayout(bytes));       // structured blocks + bbox
const tables  = JSON.parse(extractTables(bytes));  // reconstructed tables
const figures = JSON.parse(extractFigures(bytes)); // figures linked to captions
const math    = JSON.parse(extractFormulas(bytes));// formulas → best-effort LaTeX
const scanned = JSON.parse(detectScanned(bytes));  // pages needing OCR
const scan    = JSON.parse(scanInjection(bytes));  // hidden/off-page text report
const text    = extractText(bytes);
const meta    = JSON.parse(parsePdfMetadata(bytes));
const chunks  = JSON.parse(generateChunks(bytes, 500, 50));
const images  = JSON.parse(listImages(bytes));
```

## Capability declaration (NS-ONTO-001)

`describe` says what the commands *are*. The manifest in `src/ontology.rs` says
the three things a command surface cannot, and which NS-ONTO-001 §3 keeps
independent because deriving one from another has been wrong every time:

| | |
| --- | --- |
| **effect** | what it does to state — most of this engine is read-only, `convert` is not |
| **authority** | what a caller must hold. `public` here is examined, not assumed: this is a local binary and the gate on reading a file is the operating system's |
| **agent-safety** | whether an autonomous caller should, which a read-only marking does not settle |

Capabilities also declare how they refuse — `when`, `response` and **`because`**
— so an agent can plan around a refusal instead of learning it by spending a
call. And because NS-ONTO-001 defines evidence as *the command that re-derives
the claim independently*, `search` and `chunk` declare `irondoc verify` as
theirs.

The declaration is written by hand (§2.4: *declared, then verified, never
reflected* — a generated description is always accurate and never means
anything) and `tests/ontology.rs` holds it to the running product in both
directions: every declared invocation must be reachable on the surface it
claims, and every entry point the engine serves must be declared or named in
`unmapped`. It reads the command line from the binary's own `--help` and the
tools from the MCP tool table, so neither is a list kept beside the code.

That test earned itself on its first run: `sheet` and `recalc` had reached the
command line and the `describe` output but not the MCP tool table, leaving the
whole spreadsheet surface invisible to the agents that reach this engine
through it.

## MCP server

`irondoc mcp` runs a Model Context Protocol server over stdio (newline-delimited
JSON-RPC 2.0), exposing tools `extract_text`, `markdown`, `layout`, `tables`,
`figures`, `formula`, `scanned`, `scan_injection`, `metadata`, `outline`,
`annotations`, `images`, and `chunk`. Register it with an MCP client, e.g.:

```json
{
  "mcpServers": {
    "irondocuments": { "command": "irondoc", "args": ["mcp"] }
  }
}
```

Each tool takes a `path` argument (plus options like `sanitize`, `size`,
`overlap`); results come back as text/JSON content.

## vs. anydoc

[`anydoc`](https://github.com/firecrawl/anydoc) is a fast pure-Rust converter
covering 14 formats. It is a *parser*: every input becomes GitHub-Flavored
Markdown and stops there. IronDocuments overlaps on that surface and keeps going —
the same document also yields geometry, citable bounding boxes, GPU rendering,
an injection scan and an MCP server.

| | IronDocuments | anydoc |
| --- | --- | --- |
| Format coverage | 16 formats: PDF, Office (OOXML + legacy binary), OpenDocument, EPUB, RTF, HTML, Markdown, CSV, text | 14 formats |
| Content-based format detection | ✅ | ✅ |
| → GitHub-Flavored Markdown | ✅ | ✅ |
| → HTML, JSON, plain text | ✅ | ✗ (Markdown only) |
| Bounding boxes for citations | ✅ | ✗ |
| Page **rendering** (WebGL2 / GPU) | ✅ | ✗ |
| Tables from ruling lines (untagged PDF) | ✅ | ✗ |
| Formula → LaTeX, figure ↔ caption linking | ✅ | ✗ |
| Prompt-injection / hidden-text scan | ✅ | ✗ |
| Semantic chunking for RAG | ✅ | ✗ |
| MCP server + JSON-LD agent ontology | ✅ | ✗ |
| Runtime | none (static binary / WASM) | none (static binary / WASM) |

Coverage is now comparable; where this goes deeper is everything an agent does
*after* it has the text.

The hidden-text scan is worth singling out, because it is the one capability
that changes what an agent can safely be handed. Word's `<w:vanish/>`, RTF's
`\v`, HTML's `display:none` and a PDF's off-page text all render invisible to a
human reviewer while remaining perfectly readable to a model. `irondoc scan`
reports them and `--sanitize` strips them, across every format, through one
command.

## vs. liteparse / LlamaParse

[`liteparse`](https://github.com/run-llama/liteparse) is a fast local Rust PDF
parser, but its README explicitly defers **table extraction, image analysis,
reading-order optimization, document chunking, and metadata extraction** to the
LlamaParse *cloud*. IronDocuments does all of those locally — plus figure↔caption
linking, formula→LaTeX, tagged-PDF structure, prompt-injection scanning, and an
MCP server — as a single zero-runtime binary or WASM module.

| | IronDocuments | liteparse | LlamaParse (cloud) |
| --- | --- | --- | --- |
| Reading order, headings, lists | ✅ | ✗ (deferred) | ✅ |
| Tables | ✅ | ✗ (deferred) | ✅ |
| Metadata, chunking | ✅ | ✗ (deferred) | ✅ |
| Figures, formulas (LaTeX), tagged structure | ✅ | ✗ | partial |
| Prompt-injection / hidden-text scan | ✅ | ✗ | ✗ |
| Pluggable OCR (`/ocr` HTTP: PaddleOCR/EasyOCR) | ✅ | ✅ | n/a |
| VLM OCR (PaddleOCR-VL-1.6 via OpenAI API) | ✅ | ✗ | (cloud) |
| Bundled Tesseract OCR | ✅ (`--features ocr`) | ✅ | n/a |
| MCP server / JSON-LD agent ontology | ✅ | ✗ | ✗ |
| Runs fully local, no cloud | ✅ | ✅ | ✗ |

The HTTP OCR backend speaks liteparse's exact `/ocr` contract
(`{results:[{text,bbox,confidence}]}`), so the same PaddleOCR/EasyOCR server
works as a drop-in — IronDocuments is a strict superset of that integration.

## vs. PDF.js

PDF.js is the reference JS engine for **rendering** PDFs to a canvas. On the
data-extraction surface an agent uses, IronDocuments matches it and goes further:

| Extraction feature | IronDocuments | PDF.js |
| --- | --- | --- |
| Text with positions | ✅ | ✅ (`getTextContent`) |
| Annotations | ✅ | ✅ (`getAnnotations`) |
| AcroForm fields | ✅ `irondoc forms` | ✅ (`getFieldObjects`) |
| Outline / metadata | ✅ | ✅ |
| Reading-order Markdown | ✅ | ✗ |
| Tables / figures / formulas | ✅ | ✗ |
| Tagged-PDF structure tree | ✅ | partial |
| Semantic chunks, injection scan, MCP, OCR | ✅ | ✗ |
| Page **rendering** to canvas | ✅ **WebGL2 (GPU)** | ✅ (Canvas2D) |
| Footprint | single static binary / WASM, no runtime | JS engine |

For rendering, IronDocuments goes a step further than PDF.js's Canvas2D engine: the
Rust/WASM core emits a device-space **display list** and a **WebGL2 renderer**
rasterizes vector fills/strokes on the **GPU** (see Rendering, below).

## Rendering (hardware-accelerated)

The engine emits a device-space **display list** of draw primitives — flattened
fill/stroke subpaths (RGBA, even-odd/width), text runs (position/size/colour),
and image placements — for **any supported format** (CLI: `irondoc displaylist
<file> --page N`; WASM: `displayList(bytes, page)`).

For a PDF the list comes from the file's own content streams. For a `.docx`,
`.pptx`, `.xlsx`, `.epub`, `.rtf`, HTML or Markdown it comes from the
typesetter (`typeset/`), which flows the document's authored structure into
pages: greedy line breaking against standard-14 metrics, CJK per-character
wrapping, alignment and justification, hanging list markers with real counters,
tables with rulings that split across pages repeating their header, and images
scaled to the content width.

Because both paths emit the *same* `RenderOp`s, the renderer is unchanged
either way — it never learns which kind of document it is drawing.

`render/webgl-renderer.ts` rasterizes that list on the **GPU with WebGL2**:
vector fills via the stencil even-odd technique (concave paths + holes, no CPU
triangulation), strokes as expanded segment quads, **images as textured quads**
(JPEG decoded by the browser; raw/indexed/bilevel decoded to RGBA — with
**`/SMask` soft-mask alpha** — in Rust via `pageImages`), **non-rectangular
clip paths** (a two-bit stencil: bit 0 = clip mask, bit 1 = fill even-odd, so
arbitrary clip shapes and fills coexist; a scissor box pre-clips), and crisp
text on a 2D overlay layer.

Validated in a real browser: a headless Playwright harness
(`render/test-render.mjs`, Firefox then Chromium/SwiftShader) reads back GL
pixels and confirms three things draw — vector content from `demos/sample.pdf`
(page 1), a decoded image texture (page 6), and a **reflowable Markdown
document** that has pixels only because the typesetter gave it some.

```bash
# Build the WASM engine and the renderer, then serve the demo:
wasm-pack build irondocuments-rs --target web --features wasm --no-default-features
cd irondocuments-rs/render && npm install && npm run build && npm run serve
# open http://localhost:8080/render/demo.html
```

The display-list and image-pixel extraction are unit-tested in the Rust engine,
and the renderer type-checks under `tsc --strict`; the WebGL drawing itself is
browser code (validate in a WebGL2-capable browser).

## Architecture

```shell
src/
├── lib.rs       # Public types (PdfDocument, PdfPage, TextBlock, …) + JSON-LD ontology
├── document.rs  # Document facade — detect, dispatch, cache; one API per format
├── detect.rs    # Content-based format identification (magic bytes, container parts)
├── doc.rs       # Format-neutral semantic model + GFM and HTML serialisers
├── typeset/     # Semantic structure → page geometry (line breaking, pagination,
│             #   tables, images) → PdfPage + PageGraphics + DisplayList
├── formats/     # Per-format parsers → semantic model
│   ├── text.rs  #   plain text, CSV/TSV (RFC 4180), Markdown
│   ├── html.rs  #   HTML/XHTML tokenizer + tree builder, hidden-text detection
│   ├── rtf.rs   #   RTF control words, destinations, cp1252/\u, stylesheet headings
│   ├── epub.rs  #   EPUB container → OPF package → XHTML chapters
│   ├── ooxml/   #   OPC relationships + docx / xlsx / pptx readers
│   ├── odf/     #   ODF styles + odt / ods / odp readers
│   └── legacy/  #   OLE2 binaries: doc (piece table, SPRM, STSH), xls (BIFF8), ppt
├── container/   # Packaging layers the formats arrive in
│   ├── zip.rs   #   read-only ZIP (OOXML, OpenDocument, EPUB)
│   └── ole.rs   #   OLE2 compound file + code pages (legacy Office)
├── xml.rs       # Namespace-aware pull XML parser (no entity expansion — no XXE)
├── engine.rs    # PDF object model, lexer, xref (table+stream), object streams,
│                # page tree, fonts/encoding, content-stream text extraction
│                # plus tagged-PDF /StructTreeRoot logical structure extraction
├── layout.rs    # Reading order, XY-cut columns, block classification, Markdown
├── tables.rs    # Table reconstruction (rulings + text-alignment + panels) → Markdown/JSON
├── figures.rs   # Figure detection + caption linking
├── formula.rs   # Math detection → best-effort LaTeX (symbols, sub/superscripts, fractions)
├── text_norm.rs # Diacritic recombination + positional accent reconstruction
├── sanitize.rs  # Prompt-injection / hidden-text scan + sanitized extraction
├── ocr.rs       # Scanned-page detection + image-decode pipeline + OcrBackend (feature `ocr`)
├── mcp.rs       # MCP (Model Context Protocol) stdio server
├── parser.rs    # FlateDecode + PNG-predictor primitives; thin parse() facade
├── main.rs      # CLI (clap) — self-describing for agent discovery
└── wasm.rs      # wasm-bindgen exports
```

## Status & roadmap

Working today: modern + classic PDFs, Unicode text (incl. ligatures, diacritic
recombination, and **positional accent reconstruction** for TeX), reading-order
Markdown, structured layout with bounding boxes, column detection, heading/list
detection, **table reconstruction** (bordered, booktabs, borderless,
side-by-side panels), **figure ↔ caption linking**, **formula → best-effort
LaTeX** (symbols, super/subscripts, **fractions**, radicals), **scanned-page
detection** with an image-decode pipeline + pluggable OCR backend (`ocr`
feature), **prompt-injection / hidden-text scan + sanitized extraction**, image
enumeration, annotations, outline, metadata, semantic chunks, and a built-in
**MCP server**.

OCR: `irondoc scanned` flags image-dominated pages always. Building with
`--features ocr` adds the image-decode pipeline and two backends:
- **`TesseractCli`** — `irondoc ocr document.pdf` runs the `tesseract` binary (no
  FFI, no model downloads; just `tesseract` on `PATH`).
- **`HttpOcrBackend`** — `irondoc ocr document.pdf --server <url>` POSTs each
  scanned page image to a **PaddleOCR** / EasyOCR / liteparse-compatible `/ocr`
  endpoint (`{results:[{text,bbox,confidence}]}`).
- **`VlmOcrBackend`** — `irondoc ocr document.pdf --vlm <url>` sends each page image
  to a document-parsing **vision-language model** over an OpenAI-compatible
  `/v1/chat/completions` endpoint and returns its Markdown. This is how you wire
  **PaddleOCR-VL-1.6** (SOTA on OmniDocBench), e.g.:

  ```bash
  # Serve the HF model with an OpenAI-compatible API:
  vllm serve PaddlePaddle/PaddleOCR-VL-1.6
  # Then parse scanned pages through it:
  irondoc ocr scanned.pdf --vlm http://localhost:8000/v1/chat/completions \
      --model PaddleOCR-VL-1.6
  ```

Or implement `ImageOcrBackend` for a custom engine and call `ocr_scanned_images`.

Math: symbol mapping, super/subscripts, operator limits (`\sum_{i}^{n}`),
`\frac` (from rule bars) including **nested fractions**, multi-token `\sqrt`
radicands, and bracket-delimited matrices → `\begin{bmatrix}…`.

CID fonts: ToUnicode CMaps, WinAnsi + `/Differences`, Identity-H, embedded-CMap
**codespace ranges** (mixed-byte-width tokenization), and **Unicode predefined
CMaps** (`UniGB-UCS2-H` etc.) so CJK text decodes without a ToUnicode map.

OCR image decode (`--features ocr`): JPEG (DCTDecode), **CCITT Group 4** fax
(via the `fax` crate), 8-bit Gray/RGB, **1/2/4-bit bilevel/low-depth** grayscale,
and **indexed/palette** images (RGB/Gray/CMYK lookup → luma).

Planned (require external assets / large vectors, so deferred rather than
shipped unverified): a model-weighted OCR engine bundled with weights; JBIG2
(arithmetic-coded symbol dictionaries); and CID→Unicode fallback tables for the
character-collection CMaps (Adobe-Japan1, GB1, …) of CJK fonts that ship no
ToUnicode.

## Third-party code

The Word 97-2003 (`.doc`) and PowerPoint 97-2003 (`.ppt`) readers, and the SPRM
and style-sheet machinery they share, are derived from
[anydoc](https://github.com/firecrawl/anydoc) — MIT licensed, Copyright (c) 2026
Sideguide Technologies Inc. The full notice is in
[LICENSE-MIT-anydoc.txt](LICENSE-MIT-anydoc.txt), and each derived file carries
it in its header. The `.xls` reader is not derived from anydoc, which delegates
that format to the `calamine` crate.

## Dependencies

Five, all pure Rust: `serde`, `serde_json`, `miniz_oxide` (deflate, for the ZIP
container), `cfb` (OLE2 compound files) and `encoding_rs` (the legacy code
pages). ZIP, XML, RTF and BIFF8 are read by this crate directly rather than
through further dependencies.

`encoding_rs` carries the code-page tables and is the largest contributor to the
WASM blob. A build that will never see a legacy binary Office file can drop both
it and `cfb`; open an issue if you want that behind a feature flag.

## License

AGPL-3.0-or-later / Commercial dual license — NERVOSYS, LLC
