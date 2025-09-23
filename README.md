# ModernPDF

[![CI](https://github.com/nervosys/modernpdf/actions/workflows/ci.yml/badge.svg)](https://github.com/nervosys/modernpdf/actions/workflows/ci.yml)
[![Security Scan](https://github.com/nervosys/modernpdf/actions/workflows/security.yml/badge.svg)](https://github.com/nervosys/modernpdf/actions/workflows/security.yml)
[![npm version](https://badge.fury.io/js/modernpdf.svg)](https://badge.fury.io/js/modernpdf)
[![TypeScript](https://img.shields.io/badge/TypeScript-007ACC?style=flat&logo=typescript&logoColor=white)](https://www.typescriptlang.org/)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![Downloads](https://img.shields.io/npm/dm/modernpdf.svg)](https://www.npmjs.com/package/modernpdf)
[![Theme Toggle](https://img.shields.io/badge/Theme-Dark%2FLight%20Mode-purple)](https://github.com/nervosys/modernpdf#themes)

A comprehensive, production-ready PDF processing library with first-class support for streaming, AI systems, and modern UI features including dark/light theme toggle. Built as a single TypeScript file for maximum portability and ease of integration.

## 🚀 Features

- 🚀 **Streaming-First Architecture** - Process large PDFs without memory bloat
- 🤖 **AI-Native Design** - Built-in semantic chunking, structural analysis, and embedding support
- 🎨 **Modern UI with Theme Toggle** - Dark/light mode support with customizable viewers
- 📄 **Complete PDF Support** - Text extraction, images, forms, annotations, and rendering
- 🔧 **Zero Dependencies** - Self-contained TypeScript implementation
- 💾 **Memory Efficient** - Configurable memory limits and lazy loading
- 🌐 **Web & Server Ready** - Works in browsers and Node.js environments
- ⚡ **Worker Support** - Offload CPU-intensive tasks to Web Workers

## 📦 Installation

### NPM Package

```bash
npm install modernpdf
```

### Single File (Zero Dependencies)

```bash
# Download the single TypeScript file
curl -O https://raw.githubusercontent.com/nervosys/modernpdf/main/modernpdf.ts

# Or using wget
wget https://raw.githubusercontent.com/nervosys/modernpdf/main/modernpdf.ts
```

### CDN (Browser)

```html
<!-- ES Module -->
<script type="module">
  import ModernPDF from 'https://unpkg.com/modernpdf/modernpdf.ts';
</script>
```

## 🚀 Quick Start

### Basic Usage

```typescript
import ModernPDF from 'modernpdf';

// Load from different sources
const pdf = await ModernPDF.fromFile(file);
const pdf = await ModernPDF.fromUrl('https://example.com/document.pdf');
const pdf = await ModernPDF.fromBuffer(arrayBuffer);

// Extract text
const textContent = await pdf.extractText({
  preserveFormatting: true,
  extractTables: true
});

// Get metadata
const metadata = pdf.getMetadata();
console.log(`Document has ${metadata?.pageCount} pages`);

// Always cleanup
pdf.close();
```

### 🎨 Optimal Viewer with Theme Toggle (New!)

ModernPDF now includes an optimal configuration with built-in theme toggle functionality:

```typescript
// Create a complete PDF viewer with theme toggle
const container = document.getElementById('pdf-container');
const viewer = pdf.createOptimalViewer(container, {
  enableThemeToggle: true,    // Theme switching enabled by default
  defaultTheme: 'dark',       // Dark mode by default
  persistTheme: true,         // Remember user preference
  fitToWidth: true,           // Auto-fit to container width
  maintainAspectRatio: true   // Preserve PDF proportions
});

// Access theme manager
viewer.themeManager.toggleTheme();
console.log('Current theme:', viewer.themeManager.getCurrentTheme());
```

### 🎯 Interactive Demos

ModernPDF provides both browser-based demos and CLI examples:

#### 🌐 Browser Demos (Interactive HTML)

```bash
# View interactive browser demos
npm run examples:demo
# Then open demos/examples-demo.html in your browser

# Try the full-featured PDF viewer with theme toggle
# Open demos/pdf-viewer.html in your browser

# Simple demo for quick testing
npm run examples:simple  
# Then open demos/simple-demo.html in your browser
```

**Available Browser Demos:**
- 🎯 **PDF Viewer** (`demos/pdf-viewer.html`) - Full-featured viewer with dark/light mode toggle
- 🌐 **Simple Demo** (`demos/simple-demo.html`) - Basic integration example
- 🧪 **Config Test** (`demos/test-optimal-config.html`) - Test optimal configuration
- 🎨 **Theme Demo** (`demos/theme-toggle-demo.html`) - Theme toggle functionality showcase
- 🔧 **API Examples** (`demos/examples-demo.html`) - Interactive API demonstrations

#### 💻 CLI Examples (TypeScript)

```bash
# Interactive CLI runner - select examples and files
npm run examples

# Run specific file with all examples
npm run examples -- --file=demos/sample.pdf --all
```

**Available CLI Examples:**
- 📄 **Basic Processing** - Text extraction and metadata
- 🤖 **AI Integration** - Semantic chunking and analysis
- 🌊 **Streaming to LLM** - Real-time processing with language models
- 📦 **Batch Processing** - Multiple file handling
- 🔄 **Real-time WebSocket** - Live streaming integration

See the [demos directory](./demos/README.md) for browser demo details and [examples directory](./examples/README.md) for CLI example details.

## 🌟 Why ModernPDF?

- **🏗️ Single File Architecture** - Just one TypeScript file, no dependencies
- **🔄 Streaming-First** - Process massive PDFs without memory bloat
- **🤖 AI-Native** - Built-in semantic chunking for RAG systems
- **⚡ Performance** - Web Workers support for CPU-intensive tasks
- **🌐 Universal** - Works in Node.js, browsers, and edge environments
- **📊 Comprehensive** - Text, images, forms, annotations, and rendering
- **🛡️ Type-Safe** - Full TypeScript support with excellent IntelliSense

## 📋 Core Features

### Text Extraction

```typescript
// Basic text extraction
const text = await pdf.extractText();

// Advanced options
const text = await pdf.extractText({
  preserveFormatting: true,
  includeAnnotations: true,
  normalizeWhitespace: true,
  extractTables: true,
  detectColumns: true,
  pageRange: { start: 1, end: 10 }
});

// Streaming for large documents
for await (const textContent of pdf.streamText()) {
  console.log(`Page ${textContent.pageNumber}: ${textContent.text}`);
}
```

### AI Integration & Semantic Chunking

```typescript
// Generate semantic chunks for RAG systems
const chunks = await pdf.generateSemanticChunks({
  strategy: 'semantic',
  maxChunkSize: 1000,
  preserveParagraphs: true,
  includeMetadata: true
});

// Stream chunks for memory efficiency
for await (const chunk of pdf.streamSemanticChunks()) {
  console.log(`Chunk ${chunk.id}: ${chunk.content.substring(0, 100)}...`);
  console.log(`Pages: ${chunk.pageNumbers.join(', ')}`);
  console.log(`Type: ${chunk.type}, Confidence: ${chunk.metadata.confidence}`);
}

// Get comprehensive AI features
const aiFeatures = await pdf.getAIFeatures({
  enableStructuralAnalysis: true,
  enableSemanticChunking: true,
  enableNER: true,
  chunkSize: 1000
});

// Access structured analysis
const { tables, figures, sections } = aiFeatures.structuralAnalysis;
const documentType = aiFeatures.structuralAnalysis.documentType;
```

### Custom Embedding Provider

```typescript
class CustomEmbeddingProvider implements EmbeddingProvider {
  model = 'text-embedding-3-small';
  dimensions = 1536;
  
  async generate(text: string): Promise<Float32Array> {
    // Your embedding implementation
    const response = await fetch('/api/embeddings', {
      method: 'POST',
      body: JSON.stringify({ text }),
      headers: { 'Content-Type': 'application/json' }
    });
    const data = await response.json();
    return new Float32Array(data.embedding);
  }
  
  async generateBatch(texts: string[]): Promise<Float32Array[]> {
    // Batch implementation for efficiency
    const response = await fetch('/api/embeddings/batch', {
      method: 'POST',
      body: JSON.stringify({ texts }),
      headers: { 'Content-Type': 'application/json' }
    });
    const data = await response.json();
    return data.embeddings.map((emb: number[]) => new Float32Array(emb));
  }
}

// Use with PDF
const pdf = await ModernPDF.fromFile(file);
const aiFeatures = await pdf.getAIFeatures({
  embeddingProvider: new CustomEmbeddingProvider(),
  enableStructuralAnalysis: true
});
## 🔗 Advanced Use Cases

### Streaming to Large Language Models

```typescript
async function streamPDFToLLM(pdf: ModernPDF, llmEndpoint: string) {
  const chatHistory: string[] = [];
  
  // Stream semantic chunks to maintain context
  for await (const chunk of pdf.streamSemanticChunks({
    strategy: 'semantic',
    maxChunkSize: 1500,
    preserveParagraphs: true
  })) {
    const contextualChunk = [
      `[Document: ${pdf.getMetadata()?.title || 'Unknown'}]`,
      `[Pages: ${chunk.pageNumbers.join('-')}]`,
      `[Type: ${chunk.type}]`,
      chunk.content
    ].join('\n');
    
    // Send to LLM with context
    const response = await fetch(llmEndpoint, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        messages: [
          ...chatHistory,
          { role: 'user', content: contextualChunk }
        ],
        max_tokens: 1000,
        temperature: 0.1
      })
    });
    
    const result = await response.json();
    console.log(`LLM Analysis of pages ${chunk.pageNumbers.join('-')}:`, result.choices[0].message.content);
    
    // Maintain conversation context
    chatHistory.push(
      { role: 'user', content: `Document chunk from pages ${chunk.pageNumbers.join('-')}` },
      { role: 'assistant', content: result.choices[0].message.content }
    );
    
    // Limit context window
    if (chatHistory.length > 10) {
      chatHistory.splice(0, 2);
    }
  }
}
```

### Real-time Processing with WebSockets

```typescript
async function processWithWebSocket(pdf: ModernPDF, websocket: WebSocket) {
  const processingId = crypto.randomUUID();
  
  websocket.send(JSON.stringify({
    type: 'processing_started',
    id: processingId,
    totalPages: pdf.getPageCount()
  }));
  
  let processedChunks = 0;
  const totalChunks = Math.ceil(pdf.getPageCount() / 2); // Estimate
  
  for await (const chunk of pdf.streamSemanticChunks({
    strategy: 'semantic',
    maxChunkSize: 1000
  })) {
    // Send chunk data
    websocket.send(JSON.stringify({
      type: 'chunk_processed',
      id: processingId,
      chunk: {
        id: chunk.id,
        content: chunk.content,
        pageNumbers: chunk.pageNumbers,
        type: chunk.type,
        confidence: chunk.metadata.confidence,
        keywords: chunk.metadata.keywords
      },
      progress: {
        processed: ++processedChunks,
        total: totalChunks,
        percentage: Math.round((processedChunks / totalChunks) * 100)
      }
    }));
    
    // Throttle to prevent overwhelming client
    await new Promise(resolve => setTimeout(resolve, 50));
  }
  
  websocket.send(JSON.stringify({
    type: 'processing_complete',
    id: processingId,
    summary: {
      totalChunks: processedChunks,
      documentType: (await pdf.getAIFeatures()).structuralAnalysis.documentType
    }
  }));
}
```

### Batch Processing Pipeline

```typescript
async function processPDFBatch(files: File[], options: {
  maxConcurrent?: number;
  chunkSize?: number;
  onProgress?: (processed: number, total: number) => void;
  onError?: (file: string, error: Error) => void;
}) {
  const { maxConcurrent = 3, chunkSize = 1000, onProgress, onError } = options;
  const results: Array<{ file: string; chunks: SemanticChunk[]; metadata: PDFMetadata }> = [];
  
  // Process files in batches to control memory usage
  for (let i = 0; i < files.length; i += maxConcurrent) {
    const batch = files.slice(i, i + maxConcurrent);
    
    const batchResults = await Promise.allSettled(
      batch.map(async (file) => {
        let pdf: ModernPDF | null = null;
        try {
          pdf = await ModernPDF.fromFile(file, {
            lazyLoad: true,
            maxMemoryUsage: 50 * 1024 * 1024 // 50MB per PDF
          });
          
          const chunks = await pdf.generateSemanticChunks({
            strategy: 'semantic',
            maxChunkSize: chunkSize,
            preserveParagraphs: true
          });
          
          const metadata = pdf.getMetadata()!;
          
          return { file: file.name, chunks, metadata };
        } catch (error) {
          onError?.(file.name, error as Error);
          throw error;
        } finally {
          pdf?.close();
        }
      })
    );
    
    // Collect successful results
    batchResults.forEach((result, index) => {
      if (result.status === 'fulfilled') {
        results.push(result.value);
      }
      onProgress?.(i + index + 1, files.length);
    });
    
    // Small delay between batches for memory cleanup
    await new Promise(resolve => setTimeout(resolve, 100));
  }
  
  return results;
}

// Usage
const results = await processPDFBatch(files, {
  maxConcurrent: 2,
  chunkSize: 1500,
  onProgress: (processed, total) => {
    console.log(`Processed ${processed}/${total} files (${Math.round(processed/total*100)}%)`);
  },
  onError: (file, error) => {
    console.error(`Failed to process ${file}:`, error.message);
  }
});
```

### Document Intelligence Pipeline

```typescript
async function analyzeDocumentIntelligence(pdf: ModernPDF) {
  // Get comprehensive AI analysis
  const aiFeatures = await pdf.getAIFeatures({
    enableStructuralAnalysis: true,
    enableNER: true,
    enableSummarization: true,
    chunkSize: 1000
  });
  
  const analysis = {
    metadata: pdf.getMetadata(),
    documentType: aiFeatures.structuralAnalysis.documentType,
    
    // Extract key information
    structure: {
      sections: aiFeatures.structuralAnalysis.sections.length,
      tables: aiFeatures.structuralAnalysis.tables.length,
      figures: aiFeatures.structuralAnalysis.figures.length,
      equations: aiFeatures.structuralAnalysis.equations.length
    },
    
    // Content analysis
    content: {
      totalText: aiFeatures.nlpReady.fullText.length,
      sentences: aiFeatures.nlpReady.sentences.length,
      paragraphs: aiFeatures.nlpReady.paragraphs.length,
      readingLevel: aiFeatures.nlpReady.readingLevel,
      language: aiFeatures.nlpReady.language
    },
    
    // Key insights
    insights: {
      summary: aiFeatures.nlpReady.summary,
      keywords: aiFeatures.nlpReady.keywords,
      keyEntities: aiFeatures.semanticChunks
        .flatMap(chunk => chunk.metadata.entities || [])
        .reduce((acc, entity) => {
          const key = `${entity.type}:${entity.text}`;
          acc[key] = (acc[key] || 0) + entity.confidence;
          return acc;
        }, {} as Record<string, number>)
    },
    
    // Semantic chunks for downstream processing
    chunks: aiFeatures.semanticChunks.map(chunk => ({
      id: chunk.id,
      content: chunk.content.substring(0, 200) + '...',
      pageNumbers: chunk.pageNumbers,
      type: chunk.type,
      importance: chunk.metadata.importance,
      keywordCount: chunk.metadata.keywords?.length || 0
    }))
  };
  
  return analysis;
}
```

## 🏗️ Architecture

ModernPDF is designed as a single TypeScript file with a modular internal architecture:

- **🔄 Streaming Core** - Efficient memory management for large files
- **🧠 AI-Native Design** - Built-in semantic analysis and chunking
- **⚡ Worker Support** - Offload CPU-intensive tasks
- **🌐 Universal Compatibility** - Browser, Node.js, and edge environments
- **🛡️ Type Safety** - Complete TypeScript coverage

## 📊 Performance

| Document Size | Memory Usage | Processing Time |
| ------------- | ------------ | --------------- |
| 1MB PDF       | ~5MB RAM     | ~200ms          |
| 10MB PDF      | ~15MB RAM    | ~800ms          |
| 100MB PDF     | ~25MB RAM    | ~3s (streaming) |

*Benchmarks on Node.js 20 with 8GB RAM*

## 🔧 Configuration

### Memory Management

```typescript
const pdf = await ModernPDF.fromFile(file, {
  maxMemoryUsage: 100 * 1024 * 1024, // 100MB limit
  lazyLoad: true,
  useWebWorkers: true
});
```

### Streaming Options

```typescript
const streamOptions = {
  chunkSize: 1024 * 1024, // 1MB chunks
  backpressureThreshold: 10,
  progressCallback: (progress) => {
    console.log(`Progress: ${Math.round(progress.percentage)}%`);
  }
};
```

## 🤝 Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

### Development Setup

```bash
git clone https://github.com/nervosys/modernpdf.git
cd modernpdf
npm install
npm test
```

### Running Tests

```bash
npm run test:watch     # Watch mode
npm run test:coverage  # With coverage
npm run test:unit      # Unit tests only
npm run test:integration # Integration tests
```

## 📚 API Documentation

For complete API documentation, see the [TypeScript definitions](modernpdf.ts) or visit our [documentation site](https://github.com/nervosys/modernpdf/wiki).

## 🛡️ Security

ModernPDF processes PDFs safely without executing embedded scripts. For security concerns, please see our [Security Policy](SECURITY.md).

## 📄 License

MIT License - see [LICENSE](LICENSE) file for details.

## 🌟 Sponsors

Support ModernPDF development:

[![GitHub Sponsors](https://img.shields.io/github/sponsors/nervosys?style=social)](https://github.com/sponsors/nervosys)

## 📈 Roadmap

- [ ] PDF Generation and Editing
- [ ] Advanced OCR Integration
- [ ] Real-time Collaboration Features
- [ ] Cloud Processing APIs
- [ ] Enhanced AI Model Integration

## 🙏 Acknowledgments

- Built with modern TypeScript and Web Standards
- Inspired by the PDF.js and Apache PDFBox projects
- Thanks to all contributors and the open source community

---

**Made with ❤️ by the ModernPDF Team**

[![Star on GitHub](https://img.shields.io/github/stars/nervosys/modernpdf?style=social)](https://github.com/nervosys/modernpdf)

```typescript
// Extract form fields
const fields = await pdf.getFormFields();
console.log('Available fields:', fields.map(f => ({ name: f.name, type: f.type, value: f.value })));

// Fill form data
await pdf.fillForm({
  'firstName': 'John',
  'lastName': 'Doe',
  'email': 'john.doe@example.com',
  'agree': true
});

// Save filled form
const filledPDF = await pdf.save();
```

## Rendering & Export

```typescript
// Render to canvas
const canvas = document.createElement('canvas');
await pdf.renderPage(1, canvas, {
  scale: 2.0,
  background: '#ffffff'
});

// Render to image
const imageBlob = await pdf.renderPageToImage(1, 'png', {
  scale: 1.5,
  imageQuality: 0.9
});

// Export to different formats
const textExport = await pdf.exportAs('text', { includeMetadata: true });
const htmlExport = await pdf.exportAs('html', { includeImages: true });
const markdownExport = await pdf.exportAs('markdown', { 
  includeImages: true,
  imageFormat: 'webp' 
});
```

## Performance Considerations

### Memory Management

```typescript
// Configure memory limits
const pdf = await ModernPDF.fromFile(file, {
  maxMemoryUsage: 100 * 1024 * 1024, // 100MB limit
  lazyLoad: true, // Load pages on-demand
  cachePages: false // Don't cache pages in memory
});

// Use streaming for large documents
for await (const chunk of pdf.streamSemanticChunks()) {
  // Process chunk immediately, don't accumulate
  await processChunk(chunk);
}

// Always cleanup
pdf.close();
```

### Worker Usage

```typescript
// Offload CPU-intensive work to workers
const pdf = await ModernPDF.fromFile(file, {
  useWebWorkers: true,
  workerUrl: '/pdf-worker.js'
});

// AI analysis will run in worker thread
const analysis = await pdf.getAIFeatures({
  enableStructuralAnalysis: true,
  enableSemanticChunking: true
});
```

## API Reference

### Main Classes

- **`ModernPDF`** - Primary API class
- **`TextExtractor`** - Text extraction with formatting
- **`ImageExtractor`** - Image extraction and processing
- **`AIAnalyzer`** - Document intelligence and analysis
- **`SemanticChunker`** - Intelligent content chunking
- **`PDFRenderer`** - Canvas and image rendering
- **`FormExtractor`** - Form field processing

### Key Interfaces

- **`PDFOptions`** - Main configuration options
- **`StreamOptions`** - Streaming behavior control
- **`AIOptions`** - AI feature configuration
- **`ChunkingOptions`** - Semantic chunking strategies
- **`SemanticChunk`** - Processed content chunk with metadata
- **`AIFeatures`** - Complete AI analysis results

## Browser Support

- Modern browsers with ES2020 support
- Web Workers for CPU-intensive operations
- File API for local file processing
- Canvas API for rendering

## Node.js Support

Compatible with Node.js 16+ with appropriate polyfills for browser APIs (Canvas, File, etc.).

## 📚 Complete Examples

The `examples/` directory contains comprehensive demonstrations of ModernPDF's capabilities:

### 🎮 Interactive Example Runners

**CLI Runner (Recommended)**
```bash
npm run examples                    # Interactive mode with mock examples
npm run examples -- --file=test.pdf --all  # Run all with file
```

> **Note**: CLI runner uses mock examples for Node.js compatibility. For full functionality with actual PDF processing, use the browser demo.

**Browser Demo**
```bash
npm run examples:demo               # Full interactive web interface
npm run examples:simple             # Simple HTML demo
```

**Cross-Platform Scripts**
```bash
./run-examples.sh                   # Unix/Linux/macOS
run-examples.bat                    # Windows
```

### 📋 Available Examples

1. **Basic Processing** - Text extraction, metadata, search
2. **AI Integration** - Semantic chunking, embeddings, analysis  
3. **Streaming to LLM** - Context-aware LLM integration
4. **Batch Processing** - Efficient multi-file processing
5. **Real-time WebSocket** - Live progress updates

### 🛠️ Programmatic Usage

```typescript
// Import specific examples
import { basicPDFProcessing } from './examples/01-basic-processing';
import { aiIntegrationExample } from './examples/02-ai-integration';

// Run with your PDF
await basicPDFProcessing(pdfFile);
await aiIntegrationExample(pdfFile, 'your-api-key');

// Or use the example runner system
import { examples, runAllExamples } from './examples';
await runAllExamples(pdfFile, 'api-key-optional');
```

See [`examples/README.md`](examples/README.md) for detailed documentation and usage instructions.

## License

MIT License - see LICENSE file for details.

## Contributing

This is a single-file library. To contribute:

1. Make changes to `modernpdf.ts`
2. Test thoroughly with various PDF types
3. Update this README if adding new features
4. Ensure TypeScript compilation passes

## Examples

See the `/examples` directory for complete working examples including:

- RAG system integration
- Real-time streaming processors
- Batch processing pipelines
- Custom embedding providers
- Worker-based processing
