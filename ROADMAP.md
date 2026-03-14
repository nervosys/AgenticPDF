# AgenticPDF — Implementation Roadmap

> **Last updated:** 2026-03-14  
> **Version:** 1.0.0  
> **Architecture:** Single-file TypeScript library (`agenticpdf.ts`, ~11,000+ lines)  
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

## Phase 8.5 — Ontology & AI Agent Discovery ✅

- [x] `describe()` — full JSON-LD style ontology (concepts, capabilities, workflows, enums)
- [x] `getCapabilities()` — capability map organized by category
- [x] `getMethodSignatures()` — all method signatures for code generation
- [x] `getWorkflows()` — pre-built workflow templates for common operations
- [x] `describeDocument()` — instance-level document capability report
- [x] 9 ontology concepts, 11 capability categories, 26 method descriptors, 7 workflow templates

---

## Phase 9 — Text Selection & Copy ✅

- [x] Text layer overlay (`buildTextLayer()` integration in demo)
- [x] Transparent text spans positioned over rendered glyphs
- [x] Native browser text selection and clipboard copy
- [x] Search-in-document (Ctrl+F) via text layer

## Phase 10 — Outline / Bookmarks Panel ✅

- [x] Parse `/Outlines` tree from catalog
- [x] Sidebar bookmark panel with nested tree UI
- [x] Click-to-navigate from outline entries
- [x] Expand/collapse outline sections

---

## Planned — Not Yet Implemented

## Phase 11 — Form Interaction ✅

- [x] Render form fields as interactive HTML input overlays (text, textarea, select, checkbox, radio)
- [x] Form field parent chain walking for inherited FT/Ff/T/V/Opt properties
- [x] Button sub-type detection (checkbox, radio, pushbutton) from `/Ff` flags
- [x] `fillForm()` stores values via `_formValues` map; `getFormData()` merges original + filled
- [x] `buttonSubType` property on `FormField` interface
- [x] Sidebar form panel with field listing and Export JSON button
- [x] Form overlay zoom/reposition in `applyZoomCSS()`
- [x] Required field highlighting and read-only field disabling

### Phase 12 — Image Extraction & Formats ✅

- [x] JPEG passthrough (DCTDecode) — direct data passthrough with `image/jpeg` MIME
- [x] JPEG2000 support (JPXDecode) — direct data passthrough with `image/jp2` MIME
- [x] CCITT fax (Group 3/4) decoding — `CCITTFaxDecoder` class with packed-bit expansion
- [x] JBIG2 decoding — raw data passthrough for browser-level decoding
- [x] ICC color profile handling — resolve `/ICCBased` color spaces via stream `/N` component count
- [x] Image export API — `imageToDataURL()`, `exportImageAsDataURL()`, enhanced `ImageExtractor`
- [x] Enhanced `ImageResource` parsing — `filter[]`, `decodeParms`, `smaskData/Width/Height`
- [x] Demo image gallery panel with lightbox viewer

### Phase 13 — Agentic AI Workflow Optimization ✅

- [x] OpenAI / Anthropic function-calling tool schema generation (`getToolSchemas()`)
- [x] MCP (Model Context Protocol) server manifest generation (`getMCPManifest()`)
- [x] JSON Schema generation for all input/output types (`getJSONSchemas()`)
- [x] `ToolSchema` interface with parameters, constraints, and examples
- [x] Enhanced ontology with tool-use constraints and agent guidance
- [x] Agent session context tracking (`AgentSession` interface)
- [x] `describeForAgent()` — single-call introspection for AI agents

### Phase 14 — Rust CLI & WASM ✅

- [x] Rust project scaffold (`agenticpdf-rs/`)
- [x] Core PDF parser in Rust (header, xref, objects, streams)
- [x] FlateDecode / PNG predictor decompression in Rust
- [x] CLI binary (`agenticpdf`) with text, metadata, images, chunk commands
- [x] WASM compilation target (`wasm32-unknown-unknown`)
- [x] `wasm-bindgen` bridge for browser integration
- [x] Performance-critical hot paths (inflate, CCITT, text extraction)
- [x] Benchmark suite comparing TS vs. Rust/WASM performance

### Phase 15 — DoD Security Audit & Compliance ✅

- [x] CVE vulnerability assessment for PDF parsing attack surfaces
- [x] MITRE ATT&CK mapping (T1203, T1204, T1059 — PDF-specific vectors)
- [x] NIST FIPS 140-3 compliance review (cryptographic modules)
- [x] CMMC 2.0 Level 2 compliance checklist (AC, AU, IA, SC controls)
- [x] Input validation hardening (size limits, recursion depth, object count)
- [x] Memory safety audit (buffer bounds, integer overflow, DoS prevention)
- [x] SBOM generation (`generateSBOM()` method)
- [x] `SECURITY_AUDIT.md` comprehensive report
- [x] Supply chain security documentation

### Phase 16 — Real-Time Translation & Text-to-Speech ✅

- [x] `TranslationProvider` interface for pluggable translation engines
- [x] `TTSProvider` interface for pluggable text-to-speech engines
- [x] `translateDocument()` — batch translation preserving positional metadata
- [x] `streamTranslation()` — streaming page-by-page translation
- [x] `synthesizeSpeech()` — batch TTS with sentence-boundary splitting
- [x] `streamSpeechSynthesis()` — streaming TTS for real-time playback
- [x] `exportTranslation()` — export as text, JSON, or SRT subtitle format
- [x] Cross-lingual TTS pipeline (translate → synthesize in single call)
- [x] Word-level timing support (`TTSWordTiming`) for synchronized highlighting
- [x] Glossary override support for domain-specific term translation
- [x] Abort signal and progress callback on all operations

### Phase 17 — Advanced Text & Layout ✅

- [x] Multi-column layout detection
- [x] Table structure extraction (rows, cells, headers)
- [x] Reading order reconstruction
- [x] Ligature and complex script handling
- [x] Vertical text rendering (CJK)
- [x] Right-to-left text (Arabic, Hebrew)

### Phase 18 — Encryption & Security ✅

- [x] Standard security handler (RC4, AES-128, AES-256)
- [x] Password-protected PDF opening
- [x] Permission flag enforcement
- [x] Certificate-based encryption

### Phase 19 — Performance & Scale

- [x] Web Worker rendering pipeline
- [x] Tile-based rendering for large pages
- [x] Page-level lazy loading in viewer
- [x] Virtual scroll for 1000+ page documents
- [x] Incremental parsing for append-mode PDFs

### Phase 20 — PDF Writing & Modification

- [ ] Incremental save (append changes without rewriting)
- [ ] Page insertion / deletion / reordering
- [ ] Annotation creation and persistence
- [ ] Digital signature support
- [ ] PDF/A conformance output

### Phase 21 — AI & RAG Enhancements

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
| Type declarations   | `agenticpdf.d.ts`               |
| Demo viewer         | `demos/render-engine-demo.html` |
| Build script        | `scripts/build-browser.cjs`     |

## Known Constraints

- **CRLF line endings** in `agenticpdf.ts` and demo HTML — use `.cjs` scripts for programmatic edits
- **`"type": "module"`** in `package.json` — helper scripts must use `.cjs` extension
- **Single-file architecture** — all library code in one `agenticpdf.ts` file by design
