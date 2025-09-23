# ModernPDF AI Agent Instructions

## Project Overview

ModernPDF is a comprehensive, production-ready PDF processing library with first-class support for streaming and AI systems. This is a **single-file TypeScript library** (`modernpdf.ts`) containing a complete PDF processing implementation optimized for modern applications.

## Architecture & Core Components

### Main Class Structure
- **`ModernPDF`** - Primary API class with factory methods (`fromFile()`, `fromUrl()`, `fromBuffer()`, `fromStream()`)
- **Core Parsers**: `PDFParser`, `StreamingPDFParser`, `ContentStreamParser` for handling PDF structure
- **Extraction Classes**: `TextExtractor`, `ImageExtractor`, `FormExtractor`, `AnnotationExtractor`
- **AI-Specific**: `AIAnalyzer`, `SemanticChunker` for intelligent content analysis
- **Rendering**: `PDFRenderer`, `PDFExporter`, `PDFWriter` for output generation

### Data Flow Patterns
1. **Streaming-First**: All major operations support both immediate and streaming modes
2. **Lazy Loading**: Pages and content loaded on-demand with `options.lazyLoad`
3. **AI Integration**: Built-in semantic chunking, structural analysis, and embedding support
4. **Memory Management**: Configurable via `maxMemoryUsage` and cleanup methods

## Key Development Patterns

### Loading Documents
```typescript
// Factory methods for different sources
const pdf = await ModernPDF.fromFile(file, options);
const pdf = await ModernPDF.fromUrl(url, streamOptions);
const pdf = ModernPDF.fromStream(stream, options);
```

### Streaming Operations
- **Text Extraction**: Use `streamText()` instead of `extractText()` for large documents
- **Semantic Chunks**: Use `streamSemanticChunks()` for memory-efficient RAG processing
- **Progress Tracking**: All streaming operations support `progressCallback` in `StreamOptions`

### AI Features Configuration
```typescript
const aiFeatures = await pdf.getAIFeatures({
  embeddingProvider: customProvider,
  enableStructuralAnalysis: true,
  enableSemanticChunking: true,
  chunkSize: 1000,
  chunkOverlap: 200
});
```

### Memory Management
- Always call `pdf.close()` to cleanup resources
- Use streaming APIs for documents > 100MB
- Configure `maxMemoryUsage` in PDFOptions for memory-constrained environments

## Critical Interfaces & Types

### Core Data Types
- **`PDFMetadata`** - Document properties and structure info
- **`PDFPage`** - Individual page with content, resources, and geometry
- **`TextContent`** - Extracted text with positioning and styling
- **`AIFeatures`** - Structural analysis results with semantic chunks

### Configuration Objects
- **`PDFOptions`** - Main configuration (streaming, caching, workers)
- **`StreamOptions`** - Streaming behavior (chunk size, progress, abort signals)
- **`AIOptions`** - AI feature configuration
- **`ChunkingOptions`** - Semantic chunking strategies

### AI-Specific Types
- **`SemanticChunk`** - Text chunks with metadata, embeddings, and context
- **`StructuralAnalysis`** - Document structure (sections, tables, figures)
- **`DocumentType`** enum - Auto-detected document categories
- **`ChunkType`** enum - Content type classification

## TypeScript Patterns

### Error Handling
- Async operations throw descriptive errors
- Stream operations support `AbortSignal` for cancellation
- Check `metadata` existence before accessing properties

### Type Safety
- All interfaces are fully typed with optional properties clearly marked
- Use type guards for `DocumentType` and `ChunkType` enums
- Prefer factory methods over direct constructor usage

### Performance Optimizations
- Use `lazyLoad: true` for large documents
- Enable `useWebWorkers` for CPU-intensive operations
- Implement `EmbeddingProvider` interface for custom AI integrations

## Integration Points

### AI Systems
- Implement `EmbeddingProvider` interface for custom embedding models
- Use `SemanticChunker` for RAG system preparation
- Access `StructuralAnalysis` for document understanding

### Web Applications
- Use streaming APIs with `ReadableStream` for real-time processing
- Leverage `renderPage()` for canvas-based viewers
- Support `AbortSignal` for user cancellation

### Server Environments
- Memory management via `maxMemoryUsage` configuration
- Worker support via `workerUrl` and `useWebWorkers`
- Export capabilities to multiple formats (`text`, `html`, `markdown`, `json`)

### LLM Integration Patterns
- **Streaming Analysis**: Process documents in chunks to avoid token limits
- **Structured Extraction**: Use AI features to extract entities, tables, and metadata
- **Context-Aware Chunking**: Preserve semantic boundaries for better LLM understanding
- **Progressive Processing**: Handle large documents with progress callbacks

### Real-time Applications
```typescript
// WebSocket streaming for live PDF processing
const websocket = new WebSocket('ws://localhost:3001');

for await (const chunk of pdf.streamSemanticChunks()) {
  websocket.send(JSON.stringify({
    type: 'chunk',
    data: chunk,
    metadata: {
      pageNumbers: chunk.pageNumbers,
      confidence: chunk.metadata.confidence
    }
  }));
}
```

### Batch Processing Systems
```typescript
// Process multiple PDFs efficiently
async function processPDFBatch(files: File[]) {
  const results = await Promise.allSettled(
    files.map(async (file) => {
      const pdf = await ModernPDF.fromFile(file, { 
        lazyLoad: true,
        maxMemoryUsage: 100 * 1024 * 1024 // 100MB limit
      });
      
      try {
        const chunks = await pdf.generateSemanticChunks({
          strategy: 'semantic',
          maxChunkSize: 1000
        });
        return { file: file.name, chunks };
      } finally {
        pdf.close(); // Always cleanup
      }
    })
  );
  
  return results;
}
```

### Worker Thread Integration
```typescript
// Use with Web Workers for CPU-intensive operations
const pdf = await ModernPDF.fromFile(file, {
  useWebWorkers: true,
  workerUrl: '/pdf-worker.js'
});

// AI analysis runs in worker thread
const analysis = await pdf.getAIFeatures({
  enableStructuralAnalysis: true,
  enableSemanticChunking: true
});
```

## Common Development Workflows

### Basic Text Extraction
```typescript
const text = await pdf.extractText({
  preserveFormatting: true,
  extractTables: true,
  pageRange: { start: 1, end: 10 }
});
```

### Memory-Efficient Streaming for Large Documents
```typescript
// Stream text content to avoid loading entire document in memory
for await (const textContent of pdf.streamText({ normalizeWhitespace: true })) {
  console.log(`Page ${textContent.pageNumber}: ${textContent.text}`);
}
```

### RAG System Integration
```typescript
for await (const chunk of pdf.streamSemanticChunks({
  strategy: 'semantic',
  maxChunkSize: 1000,
  preserveParagraphs: true
})) {
  // Process chunk with embedding model
  await vectorStore.add(chunk);
}
```

### Streaming PDF Content to LLMs
```typescript
// Stream semantic chunks directly to an LLM for analysis
async function streamToLLM(pdf: ModernPDF, llmEndpoint: string) {
  const chunks: string[] = [];
  
  for await (const chunk of pdf.streamSemanticChunks({
    strategy: 'semantic',
    maxChunkSize: 1500,
    includeMetadata: true
  })) {
    chunks.push(`[Page ${chunk.pageNumbers.join('-')}] ${chunk.content}`);
    
    // Send in batches to LLM
    if (chunks.length >= 5) {
      await sendToLLM(chunks.join('\n\n'), llmEndpoint);
      chunks.length = 0; // Clear array
    }
  }
  
  // Send remaining chunks
  if (chunks.length > 0) {
    await sendToLLM(chunks.join('\n\n'), llmEndpoint);
  }
}
```

### Real-time Processing with Progress Tracking
```typescript
const pdf = await ModernPDF.fromUrl(url, {
  streamOptions: {
    chunkSize: 1024 * 1024, // 1MB chunks
    progressCallback: (progress) => {
      console.log(`${progress.currentOperation}: ${Math.round(progress.bytesRead / progress.totalBytes * 100)}%`);
    },
    abortSignal: abortController.signal
  }
});
```

### Custom Embedding Provider Integration
```typescript
class OpenAIEmbeddingProvider implements EmbeddingProvider {
  model = 'text-embedding-3-small';
  dimensions = 1536;
  
  async generate(text: string): Promise<Float32Array> {
    const response = await openai.embeddings.create({
      model: this.model,
      input: text
    });
    return new Float32Array(response.data[0].embedding);
  }
  
  async generateBatch(texts: string[]): Promise<Float32Array[]> {
    const response = await openai.embeddings.create({
      model: this.model,
      input: texts
    });
    return response.data.map(item => new Float32Array(item.embedding));
  }
}

// Use with AI features
const aiFeatures = await pdf.getAIFeatures({
  embeddingProvider: new OpenAIEmbeddingProvider(),
  enableStructuralAnalysis: true,
  chunkSize: 1000
});
```

### Form Processing
```typescript
const fields = await pdf.getFormFields();
await pdf.fillForm({ fieldName: 'value' });
const filled = await pdf.save();
```

### Advanced Document Analysis
```typescript
const aiFeatures = await pdf.getAIFeatures({
  enableStructuralAnalysis: true,
  enableNER: true,
  enableSummarization: true
});

// Access structured data
const tables = aiFeatures.structuralAnalysis.tables;
const figures = aiFeatures.structuralAnalysis.figures;
const entities = aiFeatures.nlpReady.keywords;

// Generate document summary
const summary = aiFeatures.nlpReady.summary;
```

### Multi-format Export Pipeline
```typescript
// Export to multiple formats for different use cases
const textExport = await pdf.exportAs('text', { includeMetadata: true });
const htmlExport = await pdf.exportAs('html', { includeImages: true });
const markdownExport = await pdf.exportAs('markdown', { 
  includeImages: true,
  imageFormat: 'webp' 
});

// Stream large documents during export
const jsonStream = await pdf.exportAs('json', {
  includeAnnotations: true,
  pageRange: { start: 1, end: 100 }
});
```

## Project-Specific Conventions

- **Single File Design**: All functionality in one `modernpdf.ts` file
- **Streaming by Default**: Always prefer streaming APIs for production usage
- **AI-Ready**: Built-in support for embeddings, chunking, and structural analysis
- **TypeScript Native**: Full type safety without runtime dependencies
- **Memory Conscious**: Explicit cleanup patterns and configurable limits

When working with this codebase, prioritize streaming operations, leverage the built-in AI features, and always consider memory management for large documents.