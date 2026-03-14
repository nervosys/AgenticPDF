# AgenticPDF — Implementation Roadmap

> **Last updated:** 2026-03-13  
> **Version:** 1.0.0  
> **Architecture:** Single-file TypeScript library (`agenticpdf.ts`, ~10,000+ lines)  
> **Tests:** 560 passing across 16 suites

---

## Phase 1 — Core PDF Parser ✅

Foundation: from-scratch PDF parser in TypeScript, zero external dependencies.

- [x] PDF file structure parsing (header, xref, trailer)
- [x] Cross-reference table and stream parsing
- [x] Indirect object resolution and caching
- [x] Dictionary, array, string, name, number, boolean, null parsing
- [x] `null` keyword position advancement fix (was causing infinite loops)
- [x] Stream decompression (FlateDecode, PNG predictors)
- [x] Content stream operator parsing
- [x] Factory methods: `fromFile()`, `fromUrl()`, `fromBuffer()`, `fromStream()`

## Phase 2 — Text Extraction ✅

- [x] Content stream text operator handling (Tj, TJ, ', ")
- [x] Font resource resolution (Type1, TrueType, Type0/CIDFont)
- [x] Standard 14 font width tables
- [x] WinAnsiEncoding / MacRomanEncoding character mapping
- [x] ToUnicode CMap parsing and character decoding
- [x] Encoding Differences array support
- [x] `decodeTextWithFont()` for accurate text extraction
- [x] TJ array word-boundary detection (negative kern thresholds)
- [x] Font subset prefix stripping (e.g., `ABCDEF+TimesRoman` → `TimesRoman`)
- [x] BaseEncoding inheritance parsing
- [x] CID 2-byte font encoding in `showText()`
- [x] DescendantFonts (Type0 → CIDFont) resolution

## Phase 3 — Font Classification & Metrics ✅

- [x] Serif / sans-serif / monospace classification from font name
- [x] NimbusMonL-Regu correctly classified as monospace
- [x] Per-character width lookup from font descriptor
- [x] Default width fallback by font class (serif: 500, mono: 600, sans: 556)

## Phase 4 — Rendering Engine ✅

Canvas-based PDF rendering in the browser.

- [x] Per-character text rendering with correct positioning
- [x] Font style mapping (bold, italic, bold-italic)
- [x] Color space handling (DeviceRGB, DeviceCMYK, DeviceGray)
- [x] Graphics state stack (save/restore, CTM transforms)
- [x] Path drawing operations (moveTo, lineTo, curveTo, rect, clip)
- [x] Form XObject rendering with recursive resource resolution
- [x] Inline image parsing and rendering
- [x] Raw pixel image rendering with PNG un-filtering
- [x] Configurable render scale and background options
- [x] GPU acceleration hints (desynchronized, offscreen canvas, image bitmap)

## Phase 5 — Browser Demo & Viewer ✅

Interactive demo at `demos/render-engine-demo.html`.

- [x] Sidebar controls (zoom slider, render scale, backend, background)
- [x] Debounced slider re-render (`scheduleRerender()`)
- [x] Multi-page continuous scroll layout
- [x] Performance metrics display (render time, pixel count, canvas size)
- [x] Debug log panel
- [x] Dark theme UI
- [x] Theme toggle support

## Phase 6 — Annotations & Links ✅

- [x] `AnnotationExtractor` with `getAnnotations(pageNumber)`
- [x] Parser reference stored on `AgenticPDF` for annotation resolution
- [x] `parseIndirectObject()` made accessible for cross-class use
- [x] Link annotation extraction (URI actions, GoTo actions, `/Dest` fallback)
- [x] Clickable link overlay `<a>` elements in demo viewer
- [x] External URL links open in new tab (`target="_blank"`)
- [x] Correct z-index stacking (canvas z:1, links z:10)

## Phase 7 — Zoom Controls ✅

- [x] Overlay zoom buttons (+/−/reset) with fixed positioning
- [x] Ctrl+scroll wheel zoom on canvas area
- [x] Zoom step presets: 25%, 50%, 75%, 100%, 125%, 150%, 200%, 250%, 300%, 400%
- [x] `setZoom()` syncs sidebar slider, overlay label, and triggers re-render

## Phase 8 — Internal Link Navigation ✅

- [x] Named destinations resolution from `/Names/Dests` name tree
- [x] Recursive name tree walking (handles `/Kids` intermediate nodes)
- [x] Page object number → page index mapping via Pages tree traversal
- [x] `getNamedDestinations()` API: returns `Map<string, { page, x, y }>`
- [x] Click handler scrolls to target page + Y offset (PDF→DOM coordinate conversion)
- [x] Smooth scroll with `canvasArea.scrollTo()`
- [x] Tooltip shows destination name and page number on hover
- [x] 57 named destinations resolved across 8 pages in sample PDF

---

## Planned — Not Yet Implemented

### Phase 9 — Text Selection & Copy

- [ ] Text layer overlay (`buildTextLayer()` integration in demo)
- [ ] Transparent text spans positioned over rendered glyphs
- [ ] Native browser text selection and clipboard copy
- [ ] Search-in-document (Ctrl+F) via text layer

### Phase 10 — Outline / Bookmarks Panel

- [ ] Parse `/Outlines` tree from catalog
- [ ] Sidebar bookmark panel with nested tree UI
- [ ] Click-to-navigate from outline entries
- [ ] Expand/collapse outline sections

### Phase 11 — Form Interaction

- [ ] Render form fields as HTML input overlays
- [ ] Text fields, checkboxes, radio buttons, dropdowns
- [ ] `fillForm()` and `save()` round-trip
- [ ] Form field validation display

### Phase 12 — Image Extraction & Formats

- [ ] JPEG passthrough (DCTDecode)
- [ ] JPEG2000 support (JPXDecode)
- [ ] CCITT fax (Group 3/4) decoding
- [ ] JBIG2 decoding
- [ ] ICC color profile handling
- [ ] Image export API (`extractImages()`)

### Phase 13 — Advanced Text & Layout

- [ ] Multi-column layout detection
- [ ] Table structure extraction (rows, cells, headers)
- [ ] Reading order reconstruction
- [ ] Ligature and complex script handling
- [ ] Vertical text rendering (CJK)
- [ ] Right-to-left text (Arabic, Hebrew)

### Phase 14 — Encryption & Security

- [ ] Standard security handler (RC4, AES-128, AES-256)
- [ ] Password-protected PDF opening
- [ ] Permission flag enforcement
- [ ] Certificate-based encryption

### Phase 15 — Performance & Scale

- [ ] Web Worker rendering pipeline
- [ ] Tile-based rendering for large pages
- [ ] Page-level lazy loading in viewer
- [ ] Virtual scroll for 1000+ page documents
- [ ] Incremental parsing for append-mode PDFs

### Phase 16 — PDF Writing & Modification

- [ ] Incremental save (append changes without rewriting)
- [ ] Page insertion / deletion / reordering
- [ ] Annotation creation and persistence
- [ ] Digital signature support
- [ ] PDF/A conformance output

### Phase 17 — AI & RAG Enhancements

- [ ] Embedding generation via `EmbeddingProvider` interface
- [ ] Vector store integration helpers
- [ ] Document comparison / diff
- [ ] Automatic summarization pipeline
- [ ] Structured data extraction (invoices, receipts, papers)

---

## Test Coverage

| Suite                      |                      Tests |
| -------------------------- | -------------------------: |
| Unit: AI features          |                          ✅ |
| Unit: Constructor coverage |                          ✅ |
| Unit: Error handling       |                          ✅ |
| Unit: Extraction           |                          ✅ |
| Unit: PDF parser           |                          ✅ |
| Unit: Streaming            |                          ✅ |
| Unit: Utility coverage     |                          ✅ |
| Integration: AgenticPDF    |                          ✅ |
| **Total**                  | **560 passing, 16 suites** |

## Build Artifacts

| Artifact            | Path                            |
| ------------------- | ------------------------------- |
| TypeScript source   | `agenticpdf.ts`                 |
| Browser IIFE bundle | `demos/agenticpdf-browser.js`   |
| Type declarations   | `modernpdf.d.ts`                |
| Demo viewer         | `demos/render-engine-demo.html` |
| Build script        | `scripts/build-browser.cjs`     |

## Known Constraints

- **CRLF line endings** in `agenticpdf.ts` and demo HTML — use `.cjs` scripts for programmatic edits
- **`"type": "module"`** in `package.json` — helper scripts must use `.cjs` extension
- **Single-file architecture** — all library code in one `agenticpdf.ts` file by design
