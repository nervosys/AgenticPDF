# AgenticPDF

[![CI](https://github.com/nervosys/AgenticPDF/actions/workflows/ci.yml/badge.svg)](https://github.com/nervosys/AgenticPDF/actions/workflows/ci.yml)
[![TypeScript](https://img.shields.io/badge/TypeScript-100%25-blue.svg)](https://www.typescriptlang.org/)
[![License: AGPL v3](https://img.shields.io/badge/License-AGPL%20v3-blue.svg)](https://www.gnu.org/licenses/agpl-3.0)
[![Zero Dependencies](https://img.shields.io/badge/Dependencies-Zero-brightgreen.svg)](https://www.npmjs.com/package/agenticpdf)

**Agentic-first PDF processing and rendering library for the 21st century**

## Features

- **Streaming-First** — Process large PDFs without memory bloat via `streamText()` and `streamSemanticChunks()`
- **Agentic Ingestion** — Single-call `ingest()` returns metadata, structure, chunks, and stats; `streamIngest()` yields NDJSON
- **AI-Native** — Built-in semantic chunking, structural analysis, and embedding provider interface for RAG pipelines
- **Tool Schemas** — Export OpenAI, Anthropic, and MCP function-calling schemas via `getToolSchemas()` and `getMCPManifest()`
- **Canvas Rendering** — Full PDF-to-canvas rendering with text, images, vector graphics, and form XObjects
- **Complete Extraction** — Text, images, forms, annotations, and metadata
- **Zero Dependencies** — Single TypeScript file (`agenticpdf.ts`), no runtime deps
- **Memory Efficient** — Configurable limits, lazy loading, and automatic cleanup
- **Universal** — Works in browsers and Node.js
- **Theme Support** — Dark/light mode rendering for viewer UIs

## Installation

```bash
npm install agenticpdf
```

### Single File

```bash
curl -O https://raw.githubusercontent.com/nervosys/agenticpdf/main/agenticpdf.ts
```

### Browser (CDN)

```html
<script src="https://unpkg.com/agenticpdf/agenticpdf-browser.js"></script>
<script>
  // AgenticPDF is available as window.AgenticPDF
</script>
```

## Quick Start

### Text Extraction

```typescript
import AgenticPDF from 'agenticpdf';

const pdf = await AgenticPDF.fromFile(file);

const text = await pdf.extractText({
  preserveFormatting: true,
  extractTables: true,
});

console.log(`${pdf.getMetadata()?.pageCount} pages`);
pdf.close();
```

### Streaming (Large Documents)

```typescript
for await (const content of pdf.streamText({ normalizeWhitespace: true })) {
  console.log(`Page ${content.pageNumber}: ${content.text}`);
}
```

### Canvas Rendering

```typescript
const canvas = document.getElementById('viewer') as HTMLCanvasElement;
await pdf.renderPage(1, canvas, { scale: 1.5, renderScale: 2 });
```

### Semantic Chunking for RAG

```typescript
for await (const chunk of pdf.streamSemanticChunks({
  strategy: 'semantic',
  maxChunkSize: 1000,
  preserveParagraphs: true,
})) {
  await vectorStore.add(chunk);
}
```

### AI Features

```typescript
const ai = await pdf.getAIFeatures({
  enableStructuralAnalysis: true,
  enableSemanticChunking: true,
  chunkSize: 1000,
  chunkOverlap: 200,
});

const tables = ai.structuralAnalysis.tables;
const keywords = ai.nlpReady.keywords;
```

### Custom Embedding Provider

```typescript
class MyEmbeddings implements EmbeddingProvider {
  model = 'text-embedding-3-small';
  dimensions = 1536;

  async generate(text: string): Promise<Float32Array> {
    // Call your embedding API
  }

  async generateBatch(texts: string[]): Promise<Float32Array[]> {
    // Batch embedding
  }
}

const ai = await pdf.getAIFeatures({
  embeddingProvider: new MyEmbeddings(),
});
```

### Agentic Ingestion

One call gets everything an AI agent needs — metadata, structure, semantic chunks, and stats:

```typescript
const result = await pdf.ingest({ maxChunkSize: 1000 });

console.log(result.documentType);          // "AcademicPaper"
console.log(result.summary);               // Extractive summary
console.log(result.stats.totalChunks);     // Number of semantic chunks
console.log(result.stats.processingTimeMs); // End-to-end time

for (const chunk of result.chunks) {
  await vectorStore.add(chunk.content, {
    pages: chunk.pages,
    importance: chunk.importance,
    keywords: chunk.keywords,
  });
}
```

Stream as NDJSON for pipelines and large documents:

```typescript
for await (const record of pdf.streamIngest()) {
  process.stdout.write(JSON.stringify(record) + '\n');
  // Yields: header → chunk → chunk → ... → footer
}
```

### Tool Schemas & Agent Discovery

Export function-calling schemas for LLM integrations:

```typescript
// Full introspection payload (ontology + tools + schemas + guidance)
const info = AgenticPDF.describeForAgent('openai');

// Tool schemas for specific platforms
const openaiTools = AgenticPDF.getToolSchemas('openai');
const anthropicTools = AgenticPDF.getToolSchemas('anthropic');

// MCP server manifest
const manifest = AgenticPDF.getMCPManifest();

// JSON schemas for all types
const schemas = AgenticPDF.getJSONSchemas();
```

### Form Processing

```typescript
const fields = await pdf.getFormFields();
await pdf.fillForm({ name: 'John Doe', date: '2025-01-01' });
const filled = await pdf.save();
```

### Export

```typescript
const markdown = await pdf.exportAs('markdown', { includeImages: true });
const html = await pdf.exportAs('html');
const json = await pdf.exportAs('json', { includeAnnotations: true });
```

### aPDF (Agentic PDF) Metadata

Generate a rich, machine-readable metadata envelope optimized for agentic AI, research linking, and web display:

```typescript
// Generate the full aPDF envelope
const apdf = await pdf.generateAPDFMetadata();

console.log(apdf.metadata.title);             // "Attention Is All You Need"
console.log(apdf.metadata.identifiers.arxivId); // "1706.03762"
console.log(apdf.metadata.identifiers.doi);     // "10.48550/arXiv.1706.03762"

// Linked research artifacts (models, datasets, code)
for (const artifact of apdf.artifacts) {
  console.log(`[${artifact.type}] ${artifact.name} → ${artifact.url}`);
  // [model] google/flan-t5-base → https://huggingface.co/google/flan-t5-base
  // [code]  google-research/t5x → https://github.com/google-research/t5x
}

// AI-ready chunks with full provenance
for (const chunk of apdf.aiContent.chunks) {
  await vectorStore.add({
    content: chunk.content,
    metadata: {
      doi: apdf.metadata.identifiers.doi,
      pages: chunk.pageNumbers,
      importance: chunk.importance,
    },
  });
}

// Or export as JSON directly
const apdfJson = await pdf.exportAs('apdf');
```

The aPDF format includes:
- **Identifiers**: DOI, arXiv, PMID, ISBN, HuggingFace, Semantic Scholar
- **Artifacts**: Linked models, datasets, spaces, code repos, and papers
- **Structure**: TOC, sections, tables, figures, equations, bibliography
- **AI Content**: Semantic chunks, entities, keywords, summary, token stats
- **Display Hints**: Reading order, fonts, math detection, theme suggestions
- **Provenance**: Generator info, processing pipeline, timestamp
- **JSON-LD**: Schema.org `@context`/`@type` for linked data interoperability

## Loading Sources

```typescript
const pdf = await AgenticPDF.fromFile(file);
const pdf = await AgenticPDF.fromUrl(url, streamOptions);
const pdf = await AgenticPDF.fromBuffer(arrayBuffer);
const pdf = AgenticPDF.fromStream(readableStream, options);
```

## Configuration

```typescript
const pdf = await AgenticPDF.fromFile(file, {
  lazyLoad: true,                        // Load pages on-demand
  useWebWorkers: true,                   // Offload CPU work
  workerUrl: '/pdf-worker.js',
  maxMemoryUsage: 100 * 1024 * 1024,     // 100MB limit
  streamOptions: {
    chunkSize: 1024 * 1024,              // 1MB streaming chunks
    progressCallback: (p) => console.log(`${p.currentOperation}: ${Math.round(p.bytesRead / p.totalBytes * 100)}%`),
    abortSignal: controller.signal,
  },
});
```

## Privacy & Telemetry

AgenticPDF collects **anonymous, aggregate usage metrics** to guide development. It is privacy-preserving by design:

- **Your documents never leave your machine.** No PDF content, extracted text, file names, file paths, URLs, or error messages are ever transmitted. Only coarse metrics are sent: event type, page-count, file-size *bucket* (e.g. `<1MB`), operation duration, feature name, and error *type* (e.g. `TypeError`).
- All event data is anonymized before sending; any string containing `/`, `\`, or `@`, or longer than 100 characters, is dropped. The collection endpoint is fixed in code and **cannot be reconfigured by callers**, so it can't be repurposed to exfiltrate data.
- Telemetry is **on by default** and sends batched events to `https://telemetry.nervosys.ai`.

**To disable telemetry**, use any one of:

```bash
export AGENTICPDF_NO_TELEMETRY=1   # or AGENTICPDF_OFFLINE=1
```

```typescript
import { Telemetry } from 'agenticpdf';
Telemetry.disable();                       // programmatic opt-out
```

```js
globalThis.__AGENTICPDF_NO_TELEMETRY__ = true;  // browser, before first use
```

When disabled, no network requests are made. Optional [OpenTelemetry](https://opentelemetry.io/) export is **off** unless you set `OTEL_EXPORTER_OTLP_ENDPOINT` yourself; see `.env.example`.

## Demos

The `demos/` directory contains ready-to-run HTML demos:

| Demo                      | Description                                                         |
| ------------------------- | ------------------------------------------------------------------- |
| `render-engine-demo.html` | Full rendering engine with sidebar controls and performance metrics |
| `pdf-viewer.html`         | Multi-page PDF viewer                                               |
| `theme-toggle-demo.html`  | Dark/light theme switching                                          |
| `simple-demo.html`        | Minimal usage example                                               |
| `examples-demo.html`      | Interactive API examples                                            |

```bash
npx http-server demos -p 8080 --cors
```

## Examples

TypeScript examples in `examples/`:

1. **Basic Processing** — Text extraction and metadata (`01-basic-processing.ts`)
2. **AI Integration** — Semantic chunking and structural analysis (`02-ai-integration.ts`)
3. **Streaming to LLM** — Stream chunks to language models (`03-streaming-to-llm.ts`)
4. **Batch Processing** — Process multiple PDFs efficiently (`04-batch-processing.ts`)
5. **Real-time WebSocket** — Live PDF processing over WebSocket (`05-realtime-websocket.ts`)
6. **aPDF Metadata** — Generate and inspect aPDF metadata envelopes (`06-apdf-metadata.ts`)
7. **aPDF Use Cases** — Real-world aPDF scenarios (`07-apdf-use-cases.ts`)
8. **Typesetting & Web Display** — PretextLayout for rich text rendering (`08-typesetting-web-display.ts`)

```bash
npm run examples
```

## Architecture

AgenticPDF is a single-file library (`agenticpdf.ts`) with these core components:

| Component             | Purpose                                                           |
| --------------------- | ----------------------------------------------------------------- |
| `AgenticPDF`          | Primary API — factory methods, extraction, rendering, ingestion   |
| `PDFParser`           | Binary PDF parsing — xref tables, object streams, page tree       |
| `StreamingPDFParser`  | Incremental parsing for streaming sources                         |
| `ContentStreamParser` | PDF content stream operator parsing                               |
| `PDFGraphicsExecutor` | Canvas 2D rendering — text, paths, images, color spaces           |
| `PDFGlyphMetrics`     | Font width tables and glyph metrics (standard 14 fonts)           |
| `PDFTextDecoder`      | Character encoding — ToUnicode CMaps, PDFDocEncoding, glyph names |
| `TextExtractor`       | Text extraction with formatting preservation                      |
| `ImageExtractor`      | Image extraction and decoding (JPEG, PNG, CCITT)                  |
| `FormExtractor`       | AcroForm field extraction and filling                             |
| `AIAnalyzer`          | Structural analysis, NER, summarization, document typing          |
| `SemanticChunker`     | Configurable chunking strategies for RAG pipelines                |
| `PretextLayout`       | Native multiline text layout with CJK and grapheme support        |

## Advanced Features

### Layout Analysis

Detect columns, tables, and reading order in complex document layouts:

```typescript
const layout = await pdf.analyzeLayout({ start: 1, end: 5 });
for (const page of layout.pages) {
  console.log(`Page ${page.pageNumber}: ${page.columns.length} columns, ${page.tables.length} tables`);
}
```

### Document Summarization

Generate extractive summaries without external AI services:

```typescript
const result = await pdf.summarize({ sentenceCount: 5 });
console.log(result.summary);
console.log('Key points:', result.keyPoints);
console.log(`Compression ratio: ${result.compressionRatio}`);
```

### Structured Data Extraction

Extract structured fields from invoices, academic papers, resumes, and more:

```typescript
const data = await pdf.extractStructuredData('paper');
console.log(`Type: ${data.documentType}, confidence: ${data.confidence}`);
for (const [key, field] of Object.entries(data.fields)) {
  console.log(`${key}: ${field.value} (page ${field.pageNumber})`);
}
```

### Document Comparison

Compare two PDFs and identify differences:

```typescript
const other = await AgenticPDF.fromFile(otherFile);
const diff = await pdf.compareWith(other);
console.log(`Similarity: ${diff.overallSimilarity}`);
console.log(`Added pages: ${diff.addedPages.length}`);
console.log(`Modified pages: ${diff.modifiedPages.length}`);
other.close();
```

### Vector Store Integration

Index documents into a vector database for semantic search:

```typescript
const helper = pdf.createVectorStoreHelper(vectorStoreAdapter, embeddingProvider);
const { indexed, errors } = await helper.indexDocument(pdf, {
  chunkingOptions: { strategy: 'semantic', maxChunkSize: 1000 },
});
console.log(`Indexed ${indexed} chunks`);

const results = await helper.query('What are the key findings?', 5);
for (const r of results) {
  console.log(`[${r.score.toFixed(2)}] Page ${r.pageNumbers.join(',')}: ${r.content.slice(0, 100)}`);
}
```

### PDF Writing & Modification

```typescript
// Incremental save (append-only, preserves signatures)
const result = await pdf.saveIncremental();

// Page management
const pm = pdf.getPageManager();
pm.insertBlankPage(3);      // Insert blank page at position 3
pm.deletePage(5);           // Delete page 5
pm.reorderPages([3, 1, 2]); // Reorder pages

// Add annotations
const ap = pdf.getAnnotationPersistence();
ap.createTextAnnotation(1, 100, 200, 'Review this section');
ap.createHighlightAnnotation(1, { x: 50, y: 300, width: 200, height: 20 });
```

### Digital Signatures

```typescript
const sh = pdf.getSignatureHandler();
const sig = sh.prepareSignature({
  signerName: 'Jane Doe',
  reason: 'Approval',
  hashAlgorithm: 'SHA-256',
});
// Apply external signature bytes
sh.applySignature(sig, signatureBytes);
```

### PDF/A Compliance

```typescript
const converter = pdf.getPDFAConverter();
const validation = converter.validate();
console.log(`Conformant: ${validation.conformant}`);
console.log(`Errors: ${validation.errors.length}, Warnings: ${validation.warnings.length}`);

const xmp = converter.generateXMPMetadata();
```

### Performance at Scale

```typescript
// Virtual scrolling for 1000+ page documents
const scroller = pdf.createVirtualScroller({ containerHeight: 800 });
const visible = scroller.getVisiblePages(scrollTop);

// Tile rendering for large/zoomed pages
const tileRenderer = pdf.createTileRenderer({ tileWidth: 512, tileHeight: 512 });
const tiles = tileRenderer.getVisibleTiles(1, pageWidth, pageHeight, scale, vx, vy, vw, vh);

// Lazy page loading with prefetch
const loader = pdf.createLazyLoader(3);
await loader.ensureLoaded(currentPage);
```

### Agent Discovery API

AI agents can programmatically discover capabilities:

```typescript
// Full introspection in one call (ontology + tools + schemas + guidance)
const info = AgenticPDF.describeForAgent('openai');

// Individual discovery endpoints
const ontology = AgenticPDF.describe();              // Full JSON-LD ontology
const capabilities = AgenticPDF.getCapabilities();   // Capability map
const methods = AgenticPDF.getMethodSignatures();     // All method signatures
const workflows = AgenticPDF.getWorkflows();          // 16 pre-built workflow templates

// Tool schemas for LLM function-calling
const tools = AgenticPDF.getToolSchemas('openai');    // OpenAI, Anthropic, or generic
const manifest = AgenticPDF.getMCPManifest();         // MCP server manifest
const schemas = AgenticPDF.getJSONSchemas();          // JSON schemas for all types

// Instance-level: what's possible with this specific document
const report = pdf.describeDocument();
console.log(`Recommended workflows: ${report.recommendedWorkflows}`);
```

## CLI

AgenticPDF ships a full-featured CLI (`apdf` / `agenticpdf`):

```bash
# Text extraction
apdf text -i document.pdf -o output.txt

# Unified AI ingestion (single JSON)
apdf ingest -i document.pdf -o ingested.json

# Streaming AI ingestion (NDJSON to stdout)
apdf ingest -i document.pdf --ndjson

# Ingest with custom chunk size and per-page text
apdf ingest -i document.pdf --chunk-size 500 --include-text -o result.json

# Export tool schemas for AI agent integration
apdf tool-schema --tool-schema openai
apdf tool-schema --tool-schema mcp

# Other commands
apdf meta -i document.pdf          # Metadata
apdf chunk -i document.pdf         # Semantic chunks
apdf describe                      # JSON-LD ontology
apdf generate -i paper.pdf -o paper.apdf  # aPDF format
```

## Rust CLI — multi-format document engine

A native Rust CLI (`apdf`) is available in `agenticpdf-rs/` for fast extraction
without a Node.js runtime. It is not PDF-only: the same commands read Word,
Excel, PowerPoint, EPUB, RTF, HTML, Markdown, CSV and plain text.

```bash
# Build the Rust CLI
cd agenticpdf-rs && cargo build --release

# The same commands work across formats
apdf markdown report.docx
apdf table budget.xlsx
apdf markdown deck.pptx
apdf scan untrusted.rtf
apdf layout report.docx            # bounding boxes for citations
apdf displaylist deck.pptx --page 1 # GPU display list, typeset from structure
apdf text document.pdf

# Convert anything to Markdown, HTML, JSON or text
apdf convert report.html --to markdown
apdf convert untrusted.html --to markdown --sanitize

# Discovery
apdf formats            # what this build can read
apdf describe           # full JSON-LD ontology
```

Format is identified from **content, not file extension** — a `.docx` renamed
to `.pdf` is still read as a `.docx`.

| Format | Extensions | Status |
| --- | --- | --- |
| PDF | `.pdf` | ✅ full — authored geometry, rendering, tables, figures, formulas, forms, OCR |
| Word (OOXML) | `.docx` `.docm` | ✅ styles, headings, numbering, tables, hyperlinks, images, hidden text |
| Excel (OOXML) | `.xlsx` `.xlsm` | ✅ sheets, shared strings, sparse cells, formula results |
| PowerPoint (OOXML) | `.pptx` `.pptm` | ✅ slides in order, titles, bullets, speaker notes |
| EPUB | `.epub` | ✅ spine order, metadata, per-chapter sections, images |
| Rich Text Format | `.rtf` | ✅ stylesheet headings, tables, lists, hidden text |
| HTML / XHTML | `.html` `.htm` `.xhtml` | ✅ structure, tables, links, hidden-text scan |
| Markdown | `.md` `.markdown` | ✅ |
| Delimited text | `.csv` `.tsv` | ✅ RFC 4180 quoting |
| Plain text | `.txt` | ✅ |
| OpenDocument | `.odt` `.ods` `.odp` | ✅ named styles, sheets with repeat counts, slides with notes |
| Legacy Office | `.doc` `.xls` `.ppt` | ✅ piece table + SPRM deltas, BIFF8 records, persist-resolved slides |

**Every format the engine detects, it can also parse.**

**Every supported format has page geometry**, so `layout` bounding boxes,
`displaylist` and GPU rendering work across the board. A PDF's coordinates are
read from the file; the reflowable formats get theirs from a built-in
typesetter that flows their authored structure into pages — line breaking,
pagination, tables with rulings, images. Both paths emit the same display list,
so the WebGL2 renderer draws a `.docx` without knowing it isn't a PDF.

A few capabilities stay PDF-only because they read PDF structures nothing else
has: `forms`, `outline`, `images` and `scanned`.

**Hidden-text scanning works across every format.** Word's `<w:vanish/>`, RTF's
`\v`, HTML's `display:none` and a PDF's off-page text are all invisible to a
human reviewer but fully readable by a model — a prompt-injection vector.
`apdf scan` reports them and `--sanitize` strips them.

See [agenticpdf-rs/README.md](agenticpdf-rs/README.md) for details.

## Scripts

```bash
npm run build            # Generate TypeScript declarations
npm run build:browser    # Build browser IIFE bundle
npm test                 # Run all tests (Jest, 950 tests across 25 suites)
npm run test:coverage    # Tests with coverage report
npm run typecheck        # TypeScript type checking
npm run lint             # ESLint
npm run ci               # Full CI: typecheck + lint + test:coverage
```

## Browser Bundle

The browser bundle (`agenticpdf-browser.js`) is an IIFE that exposes `window.AgenticPDF`:

```bash
npm run build:browser
```

```html
<script src="agenticpdf-browser.js"></script>
<script>
  const pdf = await AgenticPDF.fromUrl('document.pdf');
  const text = await pdf.extractText();
  pdf.close();
</script>
```

## Memory Management

Always call `pdf.close()` when done. For large documents, use streaming APIs:

```typescript
const pdf = await AgenticPDF.fromFile(file, {
  lazyLoad: true,
  maxMemoryUsage: 50 * 1024 * 1024,
});

try {
  for await (const chunk of pdf.streamSemanticChunks()) {
    await processChunk(chunk);
  }
} finally {
  pdf.close();
}
```

## Security

Audited 2026-08-11 against CVE/CVSS, MITRE ATT&CK, NIST FIPS 140-3, and CMMC 2.0
Level 2. Findings below are stated as measured, including the ones that are
unresolved. Report vulnerabilities per [SECURITY.md](SECURITY.md) — please do not
open a public issue.

### Threat model

The product parses **untrusted documents** and exposes them to **autonomous
agents**. Those two facts drive everything here: a malicious document is the
expected input, not an edge case, and an agent acting on that document is a
confused deputy with the operator's privileges.

### Parser hardening (ATT&CK T1203, Exploitation for Client Execution)

| Control | Where |
| --- | --- |
| No `unsafe` in the engine core | `unsafe` appears only in the mobile FFI shims, each with a documented safety contract, bounds and null checks, and `catch_unwind` at the boundary |
| XXE and entity expansion impossible by construction | `xml.rs` never processes `<!ENTITY>` declarations and skips `<!DOCTYPE>` wholesale; asserted by test, not just by policy |
| Zip-bomb and decompression caps | 128 MB per entry, 512 MB total, 65,536 entries, 64 KB EOCD search window |
| Depth and count caps on every recursive format | XML, HTML, ADF, PDF objects, OLE2 streams, PPT records, spreadsheet cells |
| No network egress in the core | Verified across the crate; the only outbound call is the optional `ocr` feature, to an endpoint the caller supplies explicitly |

Memory-unsafe parsing is the historical root of document-format CVEs. Rust plus
the above closes that class structurally rather than by patching instances.

### Prompt injection (ATT&CK T1204 / T1059, adapted to agents)

`apdf scan` reports text a human reader cannot see but a model ingests —
off-page, zero-size, white-on-white, and format-specific hiding (`w:vanish`,
`display:none`, hidden rows). Documents are an injection vector into any
retrieval pipeline; this is a differentiator, and we know of no comparable
extractor that checks.

### Provenance is integrity, not authenticity — **known limitation**

ADF records a content hash per block so a quotation is reported as matching,
drifted, or unrecorded. That hash is **FNV-1a (64-bit), a non-cryptographic
hash**, as are the chunk and header checksums.

It detects accidental drift and corruption reliably. It does **not** withstand an
adversary: FNV-1a offers no collision or preimage resistance, and 64 bits falls to
a generic birthday search regardless. **Do not rely on ADF provenance as a
tamper-evidence control.** Making it one means SHA-256 and a signature over the
chunk table — a format change we have not made.

### FIPS 140-3

**The product performs no cryptographic operations and makes no FIPS claim.** It
has no cryptographic dependency; it neither encrypts, signs, nor derives keys.
Consequently no FIPS 140-3 validated module is in use and none is required. Where
FIPS-validated integrity is a requirement, the non-cryptographic hashing above is
the gap to close — FNV-1a is not an approved algorithm, so provenance and
checksums must not be presented as satisfying SC-13 or SI-7.

### Agent file access — **open finding, highest severity**

The MCP server and CLI read and write **any path the process can reach**, with no
root, allowlist, or confirmation. Verified, not theorized:

- `apdf text <path>` on a non-document returns its bytes verbatim — the text
  format accepts anything, so this is arbitrary file read (ATT&CK **T1005**, Data
  from Local System).
- `apdf convert <src> --output <path>` silently overwrites an existing file
  (ATT&CK **T1565.001**, Stored Data Manipulation).

Reachable by an agent over MCP, a prompt-injected model can exfiltrate or destroy
local files within the server's privileges. Mitigating today: the server runs
with the privileges its operator gives it, and MCP clients gate tool calls. That
is the *client's* control, not ours. **Run the MCP server with least privilege
and do not expose it to untrusted document content and sensitive paths at once.**
The fix is a configurable root with path canonicalization, which is not yet
implemented.

### Dependency CVEs

Scope matters more than the totals; the engine and the GUI differ sharply.

| Surface | Result |
| --- | --- |
| **Rust core library** (`agenticpdf`, 53 crates) | **Zero advisories** (`cargo audit`) |
| Rust reader app (desktop GUI) | 2 high (`quick-xml` RUSTSEC-2026-0194/0195, DoS) via `accesskit_unix` → **Linux desktop only**, absent on Windows, macOS, wasm and mobile; 2 unmaintained (`paste`, `ttf-parser`) via egui |
| **npm, production** | 26 (1 critical, 5 high) — **all** in the OpenTelemetry tree, `protobufjs` GHSA-xq3m-2v4x-88gg being the critical |
| npm, including dev | 42 (2 critical, 17 high) |

Every production npm finding is in `optionalDependencies` behind telemetry, which
initializes **only** when `OTEL_EXPORTER_OTLP_ENDPOINT` is set and is otherwise a
no-op. The packages are still installed, so the code is on disk even when unused:
the exposure is conditional, not absent. Upgrading the OTel SDK (pinned at
0.57/1.30, now several majors behind) is the open remediation.

### CMMC 2.0 Level 2 posture

Assessed for practices a source repository can satisfy. This is a
self-assessment of the project, not a certification, and the product is not an
authorized CUI boundary.

| Practice | State |
| --- | --- |
| AC.L2-3.1.5 least privilege | **Partial.** Security workflows scope `permissions:`; `ci.yml` and `rust.yml` do not, so they take default token scope |
| CM.L2-3.4.1 baseline config | Met — pinned toolchains, committed lockfiles, Dewey pinned by commit rev rather than a mutable branch |
| CM.L2-3.4.9 third-party software | **Gap.** `trivy-action@master` and `trufflehog@main` are mutable refs; a compromised upstream executes with the repo token. Pin to SHAs |
| RA.L2-3.11.2 vulnerability scanning | **Gap — controls are dark.** CodeQL, Trivy, Semgrep, gitleaks and TruffleHog are configured but all three security workflows are `disabled_inactivity` and have not run since 2026-08-04 |
| SI.L2-3.14.1 flaw remediation | **Partial.** 26 production npm CVEs are known and unremediated |
| SI.L2-3.14.2 malicious content protection | Met — `apdf scan`, parser caps, no egress |
| IR.L2-3.6.1 incident handling | Met — [SECURITY.md](SECURITY.md) defines private disclosure and response targets |
| AU.L2-3.3.1 audit records | Met for documents — the ADF op log records every edit with its author and whether that author was a model |
| CA.L2-3.12.1 control assessment | This audit; no external assessment has been performed |

### Priorities

1. Re-enable the three security workflows — detection is configured but not running.
2. Constrain MCP/CLI file access to a configured root.
3. Upgrade the OpenTelemetry SDK to clear the critical `protobufjs` CVE.
4. Pin GitHub Actions to commit SHAs.
5. Move ADF provenance to SHA-256 before anyone treats it as tamper evidence.

Branch protection is not enabled on `master`, so no status check is required to
merge; CI passing is a convention here, not an enforced gate.

## License

AgenticPDF is dual-licensed:

- **[AGPLv3](https://www.gnu.org/licenses/agpl-3.0.html)** for open-source use
- **[Commercial License](https://nervosys.ai/licensing)** for proprietary use

See [LICENSE](LICENSE) for details.
