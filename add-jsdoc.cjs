const fs = require('fs');
const path = require('path');

const filePath = path.join(__dirname, 'agenticpdf.ts');
let src = fs.readFileSync(filePath, 'utf-8');

// Detect line ending
const eol = src.includes('\r\n') ? '\r\n' : '\n';
console.log('Line ending:', eol === '\r\n' ? 'CRLF' : 'LF');

function makeBlock(lines) {
  return lines.join(eol);
}

// Each entry: [oldCommentLines, newCommentLines, anchorLine]
const replacements = [
  [
    ['  /**', '   * Load PDF from various sources', '   */'],
    ['  /**', '   * Create an AgenticPDF instance from a File or Blob.', '   * @param file - The File or Blob containing PDF data', '   * @param options - Configuration options for PDF processing', '   * @returns A loaded AgenticPDF instance ready for operations', '   */'],
    '  static async fromFile(file: File | Blob, options?: PDFOptions): Promise<AgenticPDF> {'
  ],
  [
    ['  /**', '   * Get all pages', '   */'],
    ['  /**', '   * Retrieve all pages from the PDF document.', '   * @returns Array of PDFPage objects for every page in the document', '   */'],
    '  async getAllPages(): Promise<PDFPage[]> {'
  ],
  [
    ['  /**', '   * Extract text content with AI-ready formatting', '   */'],
    ['  /**', '   * Extract text content from the PDF with positioning, font, and styling data.', '   * @param options - Text extraction options (formatting, page range, tables, etc.)', '   * @returns Array of TextContent items with text, position, and font metadata', '   */'],
    '  async extractText(options?: TextExtractionOptions): Promise<TextContent[]> {'
  ],
  [
    ['  /**', '   * Extract text as a stream for large documents', '   */'],
    ['  /**', '   * Stream text content for memory-efficient processing of large documents.', '   * @param options - Text extraction options', '   * @yields TextContent items one at a time as they are extracted', '   */'],
    '  async *streamText(options?: TextExtractionOptions): AsyncGenerator<TextContent> {'
  ],
  [
    ['  /**', '   * Get AI features (structural analysis, semantic chunks, etc.)', '   */'],
    ['  /**', '   * Get AI features including structural analysis, semantic chunks, and NLP-ready content.', '   * @param options - AI analysis options (embedding provider, structural analysis, NER, etc.)', '   * @returns AIFeatures with structural analysis, semantic chunks, and NLP metadata', '   */'],
    '  async getAIFeatures(options?: AIOptions): Promise<AIFeatures> {'
  ],
  [
    ['  /**', '   * Generate semantic chunks for RAG systems', '   */'],
    ['  /**', '   * Generate semantic chunks optimized for RAG (Retrieval-Augmented Generation) systems.', '   * @param options - Chunking strategy and size configuration', '   * @returns Array of SemanticChunk objects with content, metadata, and page references', '   */'],
    '  async generateSemanticChunks(options?: ChunkingOptions): Promise<SemanticChunk[]> {'
  ],
  [
    ['  /**', '   * Stream semantic chunks for memory-efficient processing', '   */'],
    ['  /**', '   * Stream semantic chunks for memory-efficient RAG processing.', '   * @param options - Chunking strategy and size configuration', '   * @yields SemanticChunk objects one at a time', '   */'],
    '  async *streamSemanticChunks(options?: ChunkingOptions): AsyncGenerator<SemanticChunk> {'
  ],
  [
    ['  /**', '   * Search text within the PDF', '   */'],
    ['  /**', '   * Search for text within the PDF document.', '   * Supports case-sensitive, whole-word, and regex search modes.', '   * @param query - The search string or regex pattern', '   * @param options - Search options (caseSensitive, wholeWord, regex, contextLength)', '   * @returns Array of SearchResult objects with matches, page numbers, and context', '   */'],
    '  async search(query: string, options?: SearchOptions): Promise<SearchResult[]> {'
  ],
  [
    ['  /**', '   * Get form fields', '   */'],
    ['  /**', '   * Get all form fields in the PDF document.', '   * @returns Array of FormField objects with name, type, value, and properties', '   */'],
    '  async getFormFields(): Promise<FormField[]> {'
  ],
  [
    ['  /**', '   * Fill form fields', '   */'],
    ['  /**', '   * Fill form fields with the provided data.', '   * @param data - Key-value pairs mapping field names to their new values', '   */'],
    '  async fillForm(data: Record<string, any>): Promise<void> {'
  ],
  [
    ['  /**', '   * Get annotations', '   */'],
    ['  /**', '   * Get annotations from the PDF, optionally filtered to a specific page.', '   * @param pageNumber - Optional page number to filter annotations (1-based)', '   * @returns Array of Annotation objects with type, position, and content', '   */'],
    '  async getAnnotations(pageNumber?: number): Promise<Annotation[]> {'
  ],
  [
    ['  /**', '   * Add annotation', '   */'],
    ['  /**', '   * Add a new annotation to the PDF document.', '   * @param annotation - Partial annotation object with type, position, and content', '   * @returns The unique ID of the newly created annotation', '   */'],
    '  async addAnnotation(annotation: Partial<Annotation>): Promise<string> {'
  ],
  [
    ['  /**', '   * Render page to canvas', '   */'],
    ['  /**', '   * Render a PDF page to an HTML canvas element.', '   * @param pageNumber - Page number to render (1-based)', '   * @param canvas - Target HTML canvas element', '   * @param options - Render options (scale, rotation, etc.)', '   */'],
    '  async renderPage('
  ],
  [
    ['  /**', '   * Render page to image', '   */'],
    ['  /**', "   * Render a PDF page to an image Blob.", '   * @param pageNumber - Page number to render (1-based)', "   * @param format - Output image format: 'png', 'jpeg', or 'webp'", '   * @param options - Render options (scale, rotation, etc.)', '   * @returns Blob containing the rendered image data', '   */'],
    '  async renderPageToImage('
  ],
  [
    ['  /**', '   * Export to different formats', '   */'],
    ['  /**', '   * Export the PDF to a different format.', "   * Supported formats: 'text', 'html', 'markdown', 'json', 'xml', 'csv'.", '   * @param format - Target export format', '   * @param options - Export options (includeMetadata, includeAnnotations, pageRange, etc.)', '   * @returns Exported content as a Blob or string depending on format', '   */'],
    '  async exportAs(format: ExportFormat, options?: ExportOptions): Promise<Blob | string> {'
  ],
  [
    ['  /**', '   * Save modified PDF', '   */'],
    ['  /**', '   * Save the current PDF (including any modifications) as a Blob.', '   * @returns Blob containing the serialized PDF data', '   */'],
    '  async save(): Promise<Blob> {'
  ],
  [
    ['  /**', '   * Close and cleanup resources', '   */'],
    ['  /**', '   * Close the PDF instance and release all resources.', '   * Call this when done to free memory. The instance should not be used after closing.', '   */'],
    '  close(): void {'
  ],
  [
    ['  /**', '   * Clear all caches to free memory', '   */'],
    ['  /**', '   * Clear all internal parser and color space caches across all instances.', '   */'],
    '  static clearAllCaches(): void {'
  ],
  [
    ['  /**', '   * Get memory usage statistics', '   */'],
    ['  /**', '   * Get memory usage statistics for cached pages, objects, and parser state.', '   * @returns Object with counts for cached pages, objects, parser cache, and color space caches', '   */'],
    '  getMemoryStats(): {'
  ],
  [
    ['  /**', '   * Unload pages to free memory (keeps metadata)', '   */'],
    ['  /**', '   * Unload pages from the cache to free memory.', '   * @param keepPages - Optional array of page numbers to keep cached; if omitted, all pages are unloaded', '   */'],
    '  unloadPages(keepPages?: number[]): void {'
  ],
  [
    ['  /**', '   * Enable performance monitoring', '   */'],
    ['  /**', '   * Enable global performance monitoring for all AgenticPDF operations.', '   */'],
    '  static enablePerformanceMonitoring(): void {'
  ],
  [
    ['  /**', '   * Disable performance monitoring', '   */'],
    ['  /**', '   * Disable global performance monitoring.', '   */'],
    '  static disablePerformanceMonitoring(): void {'
  ],
  [
    ['  /**', '   * Get performance metrics', '   */'],
    ['  /**', '   * Get all recorded performance metrics.', '   * @returns Array of PerformanceMetrics entries', '   */'],
    '  static getPerformanceMetrics(): PerformanceMetrics[] {'
  ],
  [
    ['  /**', '   * Get performance summary', '   */'],
    ['  /**', '   * Get a summary of performance metrics grouped by operation.', '   * @returns Record mapping operation names to count, average duration, and total duration', '   */'],
    '  static getPerformanceSummary(): Record<string, { count: number; avgDuration: number; totalDuration: number }> {'
  ],
  [
    ['  /**', '   * Clear performance metrics', '   */'],
    ['  /**', '   * Clear all recorded performance metrics.', '   */'],
    '  static clearPerformanceMetrics(): void {'
  ],
  [
    ['  /**', '   * Get page count', '   */'],
    ['  /**', '   * Get the total number of pages in the PDF.', '   * @returns Page count, or 0 if metadata is not loaded', '   */'],
    '  getPageCount(): number {'
  ],
  [
    ['  /**', '   * Get file size', '   */'],
    ['  /**', '   * Get the file size of the loaded PDF in bytes.', '   * @returns File size in bytes, or 0 if not available', '   */'],
    '  getFileSize(): number {'
  ],
  [
    ['  /**', '   * Check if PDF is encrypted', '   */'],
    ['  /**', '   * Check whether the PDF is encrypted.', '   * @returns true if the document is encrypted, false otherwise', '   */'],
    '  isEncrypted(): boolean {'
  ],
];

let count = 0;
for (const [oldLines, newLines, anchor] of replacements) {
  const oldBlock = makeBlock(oldLines) + eol + anchor;
  const newBlock = makeBlock(newLines) + eol + anchor;
  if (src.includes(oldBlock)) {
    src = src.replace(oldBlock, newBlock);
    count++;
  } else {
    // Debug: check if anchor exists
    if (!src.includes(anchor)) {
      console.log('MISSING ANCHOR:', anchor.substring(0, 60));
    } else {
      console.log('OLD COMMENT NOT MATCHED for:', anchor.substring(0, 60));
    }
  }
}

// Handle fromUrl, fromBuffer, fromStream which may not have individual JSDoc
const insertions = [
  ['  static async fromUrl(url: string, options?: PDFOptions): Promise<AgenticPDF> {',
   ['  /**', '   * Create an AgenticPDF instance from a URL.', '   * @param url - URL pointing to the PDF document', '   * @param options - Configuration options for PDF processing', '   * @returns A loaded AgenticPDF instance ready for operations', '   */']],
  ['  static async fromBuffer(buffer: ArrayBuffer, options?: PDFOptions): Promise<AgenticPDF> {',
   ['  /**', '   * Create an AgenticPDF instance from an ArrayBuffer.', '   * @param buffer - Raw PDF data as an ArrayBuffer', '   * @param options - Configuration options for PDF processing', '   * @returns A loaded AgenticPDF instance ready for operations', '   */']],
  ['  static fromStream(stream: ReadableStream<Uint8Array>, options?: PDFOptions): AgenticPDF {',
   ['  /**', '   * Create an AgenticPDF instance from a ReadableStream for progressive loading.', '   * @param stream - ReadableStream of PDF data chunks', '   * @param options - Configuration options for PDF processing', '   * @returns An AgenticPDF instance that processes data as it arrives', '   */']],
];

for (const [anchor, docLines] of insertions) {
  // Only insert if there's no JSDoc right before the anchor
  const docBlock = makeBlock(docLines);
  const withDoc = docBlock + eol + anchor;
  if (src.includes(anchor) && !src.includes(withDoc)) {
    src = src.replace(anchor, withDoc);
    count++;
    console.log('Inserted JSDoc before:', anchor.substring(0, 60));
  }
}

fs.writeFileSync(filePath, src, 'utf-8');
console.log('Total applied: ' + count);
