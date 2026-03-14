# NeoPDF Examples

This directory contains comprehensive examples demonstrating the capabilities of the NeoPDF library. Each example focuses on different aspects and use cases of PDF processing with AI integration.

## � Quick Start

### Method 1: Interactive CLI Runner

```bash
# Install dependencies first
npm install

# Run interactive example selector
npm run examples

# Or run with specific file
npm run examples -- --file=sample.pdf

# Run all examples with a file
npm run examples -- --file=sample.pdf --all

# Show help
npm run examples:help
```

### Method 2: Browser Demo

```bash
# Open the full interactive web demo
npm run examples:demo
# Then open examples-demo.html in your browser

# Open the simple HTML demo
npm run examples:simple
# Then open simple-demo.html in your browser
```

### Method 3: Individual Examples

```typescript
// Import and run specific examples
import { basicPDFProcessing } from './examples/01-basic-processing';
import { aiIntegrationExample } from './examples/02-ai-integration';

const file = new File([pdfBuffer], 'document.pdf');
await basicPDFProcessing(file);
await aiIntegrationExample(file, 'your-api-key');
```

## �📁 Available Examples

### 1. Basic Processing (`01-basic-processing.ts`)

**Fundamental PDF operations**
- Loading PDFs from various sources (File, URL, Buffer)
- Extracting text content with formatting preservation
- Reading metadata and page information
- Basic search functionality
- Memory management best practices

**Key Features Demonstrated:**
- `NeoPDF.fromFile()`, `fromUrl()`, `fromBuffer()`
- `extractText()` with various options
- `getMetadata()` and `getPage()`
- `search()` functionality
- Resource cleanup with `close()`

**Usage:**
```typescript
import { basicPDFProcessing } from './01-basic-processing';
await basicPDFProcessing(pdfFile);
```

### 2. AI Integration (`02-ai-integration.ts`)

**AI-powered document analysis**
- Semantic chunking for RAG systems
- Custom embedding provider implementation
- Document intelligence and structural analysis
- Streaming processing for memory efficiency

**Key Features Demonstrated:**
- `generateSemanticChunks()` and `streamSemanticChunks()`
- Custom `EmbeddingProvider` interface implementation
- `getAIFeatures()` with comprehensive analysis
- Chunk type classification and metadata extraction
- OpenAI and mock embedding providers

**Usage:**
```typescript
import { aiIntegrationExample } from './02-ai-integration';
await aiIntegrationExample(pdfFile, 'your-openai-api-key');
```

### 3. Streaming to LLM (`03-streaming-to-llm.ts`)

**Large Language Model integration**
- Context-aware chunking for better LLM understanding
- Token management and context window optimization
- Progress tracking and error handling
- Interactive document querying

**Key Features Demonstrated:**
- Streaming PDF content to LLMs with context preservation
- Multiple LLM provider abstractions (OpenAI, Mock)
- Context window management and token estimation
- Relevance scoring for chunk selection

**Usage:**
```typescript
import { streamToLLMExample } from './03-streaming-to-llm';
await streamToLLMExample(pdfFile, 'your-api-key');
```
- Query-based document analysis

### 4. Batch Processing (`04-batch-processing.ts`)

**Efficient multi-document processing**
- Memory-efficient concurrent processing
- Progress tracking and error handling
- Result aggregation and reporting
- Document similarity analysis

**Key Features Demonstrated:**
- `BatchProcessor` class for scalable processing
- Concurrent processing with memory limits
- Document complexity scoring
- Topic extraction and similarity analysis
- Comprehensive batch statistics

### 5. Real-time WebSocket (`05-realtime-websocket.ts`)

**Live processing with WebSocket communication**
- Real-time progress updates
- Chunk-by-chunk streaming
- Client-server communication patterns
- Interactive UI with live feedback

**Key Features Demonstrated:**
- `PDFWebSocketProcessor` for real-time communication
- Live progress tracking and chunk streaming
- WebSocket message protocols
- Real-time UI updates
- Error handling and recovery

## 🚀 Getting Started

### Quick Start
```typescript
import { examples, runAllExamples } from './examples';

// Run all examples with a PDF file
await runAllExamples(pdfFile, 'your-api-key-optional');

// Or run specific examples
await examples[0].run({ file: pdfFile }); // Basic processing
await examples[1].run({ file: pdfFile, apiKey: 'your-key' }); // AI integration
```

### Browser Usage
Simply open any example file in a browser, and it will create an interactive UI for testing. The `index.ts` file provides a comprehensive example runner with a web interface.

### Node.js Usage
Each example can be imported and used directly:

```typescript
import { basicPDFProcessing } from './examples/01-basic-processing';
import { BatchProcessor } from './examples/04-batch-processing';
import { PDFWebSocketProcessor } from './examples/05-realtime-websocket';

// Basic processing
await basicPDFProcessing();

// Batch processing
const processor = new BatchProcessor({ maxConcurrent: 3 });
const results = await processor.processBatch(files);

// Real-time processing
const wsProcessor = new PDFWebSocketProcessor('ws://localhost:8080');
await wsProcessor.connect();
await wsProcessor.processWithWebSocket(file);
```

## 🔧 Configuration Options

### Common Options Across Examples
- **Memory Management**: `maxMemoryUsage`, `lazyLoad`, `cachePages`
- **AI Features**: `enableAI`, `embeddingProvider`, `chunkSize`
- **Processing**: `maxConcurrent`, `streamOptions`, `useWebWorkers`
- **Output**: `outputFormat`, `includeMetadata`, `progressCallback`

### Example-Specific Options

#### Batch Processing
```typescript
const options = {
  maxConcurrent: 3,        // Number of PDFs to process simultaneously
  chunkSize: 1000,         // Size of semantic chunks
  enableAI: true,          // Enable AI analysis
  outputFormat: 'summary', // 'summary' | 'detailed' | 'chunks'
  onProgress: (processed, total, fileName) => { /* callback */ }
};
```

#### AI Integration
```typescript
const aiOptions = {
  embeddingProvider: new CustomEmbeddingProvider(),
  enableStructuralAnalysis: true,
  enableSemanticChunking: true,
  enableNER: true,
  chunkSize: 1000,
  chunkOverlap: 200
};
```

#### Streaming to LLM
```typescript
const streamOptions = {
  strategy: 'semantic',
  maxChunkSize: 1500,
  preserveParagraphs: true,
  includeMetadata: true
};
```

## 📊 Performance Considerations

### Memory Usage
- **Single PDF**: 50-100MB recommended limit per document
- **Batch Processing**: 50MB per concurrent PDF
- **Streaming**: Use `lazyLoad: true` and avoid caching pages
- **AI Processing**: Use Web Workers for CPU-intensive operations

### Optimization Tips
1. **Use streaming APIs** for documents larger than 10MB
2. **Enable lazy loading** to reduce initial memory footprint
3. **Configure appropriate chunk sizes** based on your use case
4. **Implement progress callbacks** for long-running operations
5. **Clean up resources** with `pdf.close()` when done

### Recommended Limits
- **Batch Size**: 2-4 concurrent PDFs (depending on available memory)
- **Chunk Size**: 800-1500 characters for LLM processing
- **Context Window**: Use 70% of LLM's context limit for safety
- **Progress Updates**: Every 5-10 operations to avoid UI blocking

## 🔗 Integration Patterns

### RAG System Integration
```typescript
// Stream chunks directly to vector database
for await (const chunk of pdf.streamSemanticChunks()) {
  const embedding = await embeddingProvider.generate(chunk.content);
  await vectorDB.store({
    id: chunk.id,
    content: chunk.content,
    embedding: embedding,
    metadata: chunk.metadata
  });
}
```

### LLM Processing Pipeline
```typescript
// Context-aware streaming to LLM
const conversationHistory = [];
for await (const chunk of pdf.streamSemanticChunks()) {
  const response = await llm.process(chunk, conversationHistory);
  conversationHistory.push({ chunk, response });
  
  // Manage context window
  if (estimateTokens(conversationHistory) > contextLimit) {
    conversationHistory.splice(0, 2); // Remove oldest entries
  }
}
```

### Real-time Processing
```typescript
// WebSocket streaming with progress
const processor = new PDFWebSocketProcessor('ws://your-server');
processor.onProgress = (progress) => {
  websocket.send(JSON.stringify({
    type: 'progress_update',
    data: progress
  }));
};
await processor.processWithWebSocket(file);
```

## 🐛 Troubleshooting

### Common Issues

1. **Memory Errors**
   - Reduce `maxMemoryUsage` settings
   - Enable `lazyLoad` for large documents
   - Use streaming APIs instead of loading everything at once

2. **Processing Timeouts**
   - Increase timeout values in API calls
   - Use smaller chunk sizes
   - Enable Web Workers for CPU-intensive tasks

3. **WebSocket Connection Issues**
   - Check server availability and CORS settings
   - Implement reconnection logic
   - Handle connection state properly

4. **AI API Rate Limits**
   - Implement exponential backoff
   - Use batch processing for embeddings
   - Cache results when possible

### Performance Issues
- Monitor memory usage with browser dev tools
- Use profiling to identify bottlenecks
- Consider server-side processing for large documents
- Implement proper error boundaries

## 📝 Example Output

Each example provides detailed console output showing:
- Processing progress and timing
- Document metadata and statistics
- AI analysis results
- Error handling and recovery
- Performance metrics

The examples are designed to be educational and demonstrate best practices for production use.

## 🎯 Next Steps

After running these examples, consider:
1. Integrating with your specific AI/ML pipeline
2. Customizing chunk strategies for your use case
3. Building production-ready error handling
4. Implementing proper logging and monitoring
5. Scaling for your expected document volumes

For more advanced usage patterns, see the main [README.md](../README.md) and [copilot instructions](../.github/copilot-instructions.md).
