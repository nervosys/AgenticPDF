# AgenticPDF

[![CI](https://github.com/nervosys/AgenticPDF/actions/workflows/ci.yml/badge.svg)](https://github.com/nervosys/AgenticPDF/actions/workflows/ci.yml)
[![TypeScript](https://img.shields.io/badge/TypeScript-100%25-blue.svg)](https://www.typescriptlang.org/)
[![License: AGPL v3](https://img.shields.io/badge/License-AGPL%20v3-blue.svg)](https://www.gnu.org/licenses/agpl-3.0)
[![Zero Dependencies](https://img.shields.io/badge/Dependencies-Zero-brightgreen.svg)](https://www.npmjs.com/package/agenticpdf)

**Agentic-native PDF processing and rendering library for the 21st century**

## Features

- **Streaming-First** — Process large PDFs without memory bloat via `streamText()` and `streamSemanticChunks()`
- **AI-Native** — Built-in semantic chunking, structural analysis, and embedding provider interface for RAG pipelines
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

```bash
npm run examples
```

## Architecture

AgenticPDF is a single-file library (`agenticpdf.ts`) with these core components:

| Component             | Purpose                                                           |
| --------------------- | ----------------------------------------------------------------- |
| `AgenticPDF`          | Primary API — factory methods, extraction, rendering              |
| `PDFParser`           | Binary PDF parsing — xref tables, object streams, page tree       |
| `StreamingPDFParser`  | Incremental parsing for streaming sources                         |
| `ContentStreamParser` | PDF content stream operator parsing                               |
| `PDFGraphicsExecutor` | Canvas 2D rendering — text, paths, images, color spaces           |
| `PDFGlyphMetrics`     | Font width tables and glyph metrics (standard 14 fonts)           |
| `PDFTextDecoder`      | Character encoding — ToUnicode CMaps, PDFDocEncoding, glyph names |
| `TextExtractor`       | Text extraction with formatting preservation                      |
| `ImageExtractor`      | Image extraction and decoding (JPEG, PNG, CCITT)                  |
| `FormExtractor`       | AcroForm field extraction and filling                             |
| `AnnotationExtractor` | Annotation parsing                                                |
| `AIAnalyzer`          | Document structure analysis and NLP preparation                   |
| `SemanticChunker`     | Intelligent text chunking for RAG systems                         |
| `PDFRenderer`         | Page rendering coordinator                                        |
| `PDFExporter`         | Multi-format export (text, HTML, Markdown, JSON)                  |
| `PDFWriter`           | PDF modification and writing                                      |

## Scripts

```bash
npm run build            # Generate TypeScript declarations
npm run build:browser    # Build browser IIFE bundle
npm test                 # Run all tests (Jest, 560 tests)
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

## License

AgenticPDF is dual-licensed:

- **[AGPLv3](https://www.gnu.org/licenses/agpl-3.0.html)** for open-source use
- **[Commercial License](https://nervosys.com/licensing)** for proprietary use

See [LICENSE](LICENSE) for details.
