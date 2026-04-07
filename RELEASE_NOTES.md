# AgenticPDF v1.0.0 Release Notes

**Release Date:** March 2026  
**License:** AGPL-3.0-or-later  

The first stable release of AgenticPDF — a comprehensive, production-ready PDF processing library with first-class support for streaming and AI systems.

## Major Features

### Streaming-First Architecture
- **Memory Efficient**: Process large PDFs without loading everything into memory
- **Progress Tracking**: Real-time feedback during document processing
- **Abort Support**: Cancellable operations via `AbortSignal`
- **Backpressure Handling**: Automatic flow control for smooth streaming

### AI-Native Design
- **Semantic Chunking**: Intelligent content segmentation for RAG systems
- **Structural Analysis**: Automatic detection of sections, tables, and figures
- **Embedding Provider Interface**: Support for custom embedding models (OpenAI, etc.)
- **Document Intelligence**: Content classification and entity extraction
- **NLP-Ready Processing**: Text processing optimized for language models

### Complete PDF Processing
- **Text Extraction**: Advanced text extraction with formatting preservation
- **Image Processing**: Extract and handle images in multiple formats
- **Form Support**: Read and fill PDF forms programmatically
- **Annotation Handling**: Extract and process PDF annotations
- **Multi-format Export**: Export to text, HTML, Markdown, and JSON

### Theme Toggle & Modern UI
- **Dark/Light Mode Support**: Built-in theme toggle with smooth transitions
- **Optimal Viewer Configuration**: Pre-configured PDF viewer settings
- **Theme Persistence**: User preferences saved and restored via localStorage
- **Responsive Design**: Auto-fitting viewers maintaining aspect ratios

### aPDF Binary Format
- **Efficient Serialization**: Custom binary format with LZ77 compression
- **Metadata Preservation**: Full round-trip fidelity for PDF metadata
- **Version 1.1**: Improved size limits and security constraints

### OpenTelemetry Observability
- **OTLP Traces & Metrics**: Built-in instrumentation via `@opentelemetry/api`
- **Graceful Degradation**: Falls back to no-ops when OTEL packages are absent
- **Configurable**: Standard `OTEL_*` environment variables
- **Optional SDK Module**: Import `agenticpdf/otel` for full SDK bootstrap

### PretextLayout Engine
- **Native Multiline Text Layout**: Zero-dependency text measurement and line-breaking engine (inspired by [pretext](https://github.com/chenglou/pretext))
- **Opt-In Design**: Convenience methods gated by `enablePretextLayout` flag; standalone `PretextLayout` class always importable directly
- **Full API**: `prepare()`, `layout()`, `layoutWithLines()`, `walkLineRanges()`, `layoutNextLine()`, `clearCache()`, `setLocale()`
- **CJK & Grapheme-Aware**: Correct line-breaking via `Intl.Segmenter`, per-character CJK breaks, overflow-wrap at grapheme boundaries
- **Canvas + Server Fallback**: Canvas/OffscreenCanvas measurement with LRU cache; heuristic fallback for server-side environments

### Ontology & Agent Discovery
- **Machine-Readable API**: `AgenticPDF.describe()` returns JSON-LD ontology
- **Capability Map**: `AgenticPDF.getCapabilities()` organized by category
- **Method Signatures**: `AgenticPDF.getMethodSignatures()` for code generation
- **Workflow Templates**: 7 pre-built workflow templates for common tasks

## Security

Three comprehensive security audit passes (25+ total fixes):

**Pass 1 & 2:**
- SSRF protection with private IP blocking and redirect validation
- Path traversal prevention
- Cryptographic `Math.random` replacement
- XSS sanitization in HTML export
- ReDoS-safe regex patterns
- Prototype pollution protection
- Bounded streaming and recursion depth limits
- PKCS#7 padding oracle mitigation (constant-time validation)
- CSV formula injection prevention
- Worker URL validation
- Demo DOM XSS fixes

**Pass 3 — 4-Framework Audit (CVE, MITRE ATT&CK, NIST FIPS 140-3, CMMC 2.0 Level 2):**
- CLI path traversal hardening on all write operations
- `crypto.getRandomValues()` for all ID generation
- ReDoS guard with 64-char limit on user-supplied regex
- Regex special character escaping for whole-word search
- Security headers in server (`X-Content-Type-Options`, `X-Frame-Options`, `Referrer-Policy`)

## Technical Highlights

### Single File Architecture
- Complete implementation in one TypeScript file (`agenticpdf.ts`)
- TypeScript-native with full type safety and IntelliSense
- Maximum portability — easy integration into any project

### Cross-Platform Support
- **Browser Ready**: Works in modern browsers with CDN support
- **Node.js Compatible**: Full server-side functionality (Node >= 18)
- **Web Worker Support**: CPU-intensive operations offloaded to workers

### Performance
- **Memory Management**: Configurable limits and lazy loading
- **Worker Threading**: Parallel processing for heavy operations
- **Progressive Loading**: Load content on-demand

## Testing

- **924 tests** across 24 test suites — all passing
- **TypeScript**: 0 compilation errors
- Integration, unit, visual regression, and PretextLayout tests

## Interactive Demos

### Browser Demos
- **Full PDF Viewer** (`demos/pdf-viewer.html`) — Complete viewer with theme toggle
- **Simple Demo** (`demos/simple-demo.html`) — Basic integration example
- **Theme Showcase** (`demos/theme-toggle-demo.html`) — Theme functionality demo
- **Render Engine** (`demos/render-engine-demo.html`) — Canvas rendering demo
- **API Explorer** (`demos/examples-demo.html`) — Interactive API demonstrations

### TypeScript CLI
- `apdf` / `agenticpdf` npm bin commands for text extraction, metadata, search, and more

### Rust CLI (`agenticpdf-rs/`)
- Native `apdf` binary — 801 KB release build (opt-level "z", LTO, stripped)
- 10 commands: `text`, `meta`, `annotations`, `outline`, `images`, `chunk`, `all`, `describe`, `info`, `generate`
- `apdf describe` outputs full JSON-LD ontology (673 lines)
- 10 Rust tests, zero warnings

### Website
- Next.js 15.3 + React 19.1 + Tailwind CSS 4.1
- Shiki 4.0 syntax highlighting for all code examples
- Dark/light theme support

## Quick Start

### Installation

```bash
npm install agenticpdf
```

### Basic Usage

```typescript
import AgenticPDF from 'agenticpdf';

const pdf = await AgenticPDF.fromFile(file);
const text = await pdf.extractText();
const chunks = await pdf.generateSemanticChunks();
pdf.close();
```

### Optional OTEL Instrumentation

```typescript
import 'agenticpdf/otel'; // Boots the OTEL SDK from .env
import AgenticPDF from 'agenticpdf';
```

## Distribution

- **NPM**: `npm install agenticpdf`
- **Single File**: Copy `agenticpdf.ts` directly
- **CDN**: `https://unpkg.com/agenticpdf/dist/agenticpdf.js`

## License

AgenticPDF is released under the **AGPL-3.0-or-later** license.

---

For documentation, examples, and API reference, visit the [GitHub repository](https://github.com/nervosys/AgenticPDF).
