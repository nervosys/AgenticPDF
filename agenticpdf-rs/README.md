# AgenticPDF — Rust CLI & WASM

A self-contained, **zero-runtime** PDF engine for agentic AI workloads: text,
metadata, reading-order Markdown, structured layout with bounding boxes,
semantic chunks, images, annotations, and outlines — as a single static binary
or a WASM module. No JVM, no Python, no model server.

## Why this over a JVM/Python pipeline

| | AgenticPDF (Rust) | Typical Java/Python loaders |
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
# binary: target/release/apdf

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
apdf markdown paper.pdf --output paper.md
apdf markdown paper.pdf --pages 1-3

# Structured, citation-ready layout (typed blocks + bounding boxes)
apdf layout paper.pdf --output layout.json

# Reconstruct tables (bordered, booktabs, borderless) → Markdown or JSON
apdf table report.pdf
apdf table report.pdf --format json --output tables.json

# Tagged-PDF logical structure tree (author-provided, no heuristics)
apdf structure tagged.pdf

# Detect hidden / off-page text (prompt-injection defense)
apdf scan untrusted.pdf
apdf markdown untrusted.pdf --sanitize   # strip hidden text from output

# Detect figures and link them to captions
apdf figures paper.pdf

# Reconstruct best-effort LaTeX for formulas
apdf formula paper.pdf

# Flag likely-scanned pages that need OCR
apdf scanned paper.pdf

# OCR scanned pages (requires `--features ocr`)
apdf ocr scanned.pdf --lang eng                       # bundled Tesseract CLI
apdf ocr scanned.pdf --server http://localhost:8868/ocr  # PaddleOCR/EasyOCR server
apdf ocr scanned.pdf --vlm http://localhost:8000/v1/chat/completions  # PaddleOCR-VL-1.6 (VLM)

# Run as an MCP server for agent clients (Claude Desktop, etc.)
apdf mcp

# Plain or positioned text
apdf text document.pdf
apdf text document.pdf --pages 1-5 --format json

# Metadata, outline, annotations, images
apdf meta document.pdf
apdf outline document.pdf
apdf annotations document.pdf
apdf images document.pdf            # dimensions, color space, filter, size

# Semantic chunks for RAG
apdf chunk document.pdf --size 500 --overlap 50 --format json

# Everything in one pass: metadata, text, outline, chunks, Markdown, tables,
# figures, formulas, and a prompt-injection scan — one call, full understanding
apdf all document.pdf --output full.json

# Machine-readable capability discovery for agents
apdf describe                       # JSON-LD ontology
apdf info
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
} from './pkg/agenticpdf.js';

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

## MCP server

`apdf mcp` runs a Model Context Protocol server over stdio (newline-delimited
JSON-RPC 2.0), exposing tools `extract_text`, `markdown`, `layout`, `tables`,
`figures`, `formula`, `scanned`, `scan_injection`, `metadata`, `outline`,
`annotations`, `images`, and `chunk`. Register it with an MCP client, e.g.:

```json
{
  "mcpServers": {
    "agenticpdf": { "command": "apdf", "args": ["mcp"] }
  }
}
```

Each tool takes a `path` argument (plus options like `sanitize`, `size`,
`overlap`); results come back as text/JSON content.

## vs. liteparse / LlamaParse

[`liteparse`](https://github.com/run-llama/liteparse) is a fast local Rust PDF
parser, but its README explicitly defers **table extraction, image analysis,
reading-order optimization, document chunking, and metadata extraction** to the
LlamaParse *cloud*. AgenticPDF does all of those locally — plus figure↔caption
linking, formula→LaTeX, tagged-PDF structure, prompt-injection scanning, and an
MCP server — as a single zero-runtime binary or WASM module.

| | AgenticPDF | liteparse | LlamaParse (cloud) |
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
works as a drop-in — AgenticPDF is a strict superset of that integration.

## vs. PDF.js

PDF.js is the reference JS engine for **rendering** PDFs to a canvas. On the
data-extraction surface an agent uses, AgenticPDF matches it and goes further:

| Extraction feature | AgenticPDF | PDF.js |
| --- | --- | --- |
| Text with positions | ✅ | ✅ (`getTextContent`) |
| Annotations | ✅ | ✅ (`getAnnotations`) |
| AcroForm fields | ✅ `apdf forms` | ✅ (`getFieldObjects`) |
| Outline / metadata | ✅ | ✅ |
| Reading-order Markdown | ✅ | ✗ |
| Tables / figures / formulas | ✅ | ✗ |
| Tagged-PDF structure tree | ✅ | partial |
| Semantic chunks, injection scan, MCP, OCR | ✅ | ✗ |
| Page **rendering** to canvas | ✅ **WebGL2 (GPU)** | ✅ (Canvas2D) |
| Footprint | single static binary / WASM, no runtime | JS engine |

For rendering, AgenticPDF goes a step further than PDF.js's Canvas2D engine: the
Rust/WASM core emits a device-space **display list** and a **WebGL2 renderer**
rasterizes vector fills/strokes on the **GPU** (see Rendering, below).

## Rendering (hardware-accelerated)

The engine emits a device-space **display list** of draw primitives — flattened
fill/stroke subpaths (RGBA, even-odd/width), text runs (position/size/colour),
and image placements — via `engine::extract_display_list` (CLI: `apdf
displaylist <file> --page N`; WASM: `displayList(bytes, page)`).

`render/webgl-renderer.ts` rasterizes that list on the **GPU with WebGL2**:
vector fills via the stencil even-odd technique (concave paths + holes, no CPU
triangulation), strokes as expanded segment quads, images as quads, and crisp
text on a 2D overlay layer. See `render/demo.html` for a runnable viewer:

```bash
wasm-pack build --target web --features wasm --no-default-features
npx http-server agenticpdf-rs -p 8080   # open /render/demo.html
```

The display-list extraction is unit-tested in the Rust engine; the WebGL layer
is browser code (validate in a WebGL2-capable browser).

## Architecture

```shell
src/
├── lib.rs       # Public types (PdfDocument, PdfPage, TextBlock, …) + JSON-LD ontology
├── engine.rs    # Object model, lexer, xref (table+stream), object streams,
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

OCR: `apdf scanned` flags image-dominated pages always. Building with
`--features ocr` adds the image-decode pipeline and two backends:
- **`TesseractCli`** — `apdf ocr document.pdf` runs the `tesseract` binary (no
  FFI, no model downloads; just `tesseract` on `PATH`).
- **`HttpOcrBackend`** — `apdf ocr document.pdf --server <url>` POSTs each
  scanned page image to a **PaddleOCR** / EasyOCR / liteparse-compatible `/ocr`
  endpoint (`{results:[{text,bbox,confidence}]}`).
- **`VlmOcrBackend`** — `apdf ocr document.pdf --vlm <url>` sends each page image
  to a document-parsing **vision-language model** over an OpenAI-compatible
  `/v1/chat/completions` endpoint and returns its Markdown. This is how you wire
  **PaddleOCR-VL-1.6** (SOTA on OmniDocBench), e.g.:

  ```bash
  # Serve the HF model with an OpenAI-compatible API:
  vllm serve PaddlePaddle/PaddleOCR-VL-1.6
  # Then parse scanned pages through it:
  apdf ocr scanned.pdf --vlm http://localhost:8000/v1/chat/completions \
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

## License

AGPL-3.0-or-later / Commercial dual license — NERVOSYS, LLC
