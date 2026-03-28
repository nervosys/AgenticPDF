# AgenticPDF v1.0.0 Release Notes

**Release Date:** 2025  
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

### Ontology & Agent Discovery
- **Machine-Readable API**: `AgenticPDF.describe()` returns JSON-LD ontology
- **Capability Map**: `AgenticPDF.getCapabilities()` organized by category
- **Method Signatures**: `AgenticPDF.getMethodSignatures()` for code generation
- **Workflow Templates**: 7 pre-built workflow templates for common tasks

## Security

Two comprehensive security audit passes (25 total fixes):

- SSRF protection with private IP blocking and redirect validation
- Path traversal prevention
- Cryptographic `Math.random` replacement
- XSS sanitization in HTML export
- ReDoS-safe regex patterns
- Prototype pollution protection
- Bounded streaming and recursion depth limits
- PKCS#7 padding oracle mitigation
- CSV formula injection prevention
- Worker URL validation
- Demo DOM XSS fixes

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

- **871 tests** across 23 test suites — all passing
- **TypeScript**: 0 compilation errors
- Integration, unit, and visual regression tests

## Interactive Demos

### Browser Demos
- **Full PDF Viewer** (`demos/pdf-viewer.html`) — Complete viewer with theme toggle
- **Simple Demo** (`demos/simple-demo.html`) — Basic integration example
- **Theme Showcase** (`demos/theme-toggle-demo.html`) — Theme functionality demo
- **Render Engine** (`demos/render-engine-demo.html`) — Canvas rendering demo
- **API Explorer** (`demos/examples-demo.html`) — Interactive API demonstrations

### CLI
- `apdf` / `agenticpdf` commands for text extraction, metadata, search, and more

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
