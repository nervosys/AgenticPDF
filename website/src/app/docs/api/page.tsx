import { HighlightedCode } from "@/components/highlighted-code";

const methods = [
  {
    category: "Factory Methods",
    items: [
      { sig: "static async fromFile(file: File | Blob, options?: PDFOptions): Promise<AgenticPDF>", desc: "Load a PDF from a File or Blob object (browser)." },
      { sig: "static async fromUrl(url: string, options?: PDFOptions): Promise<AgenticPDF>", desc: "Fetch and load a PDF from a URL." },
      { sig: "static async fromBuffer(buffer: ArrayBuffer, options?: PDFOptions): Promise<AgenticPDF>", desc: "Load a PDF from an ArrayBuffer (Node.js / browser)." },
      { sig: "static fromStream(stream: ReadableStream<Uint8Array>, options?: PDFOptions): AgenticPDF", desc: "Load a PDF from a ReadableStream for progressive parsing." },
    ],
  },
  {
    category: "Document Info",
    items: [
      { sig: "getMetadata(): PDFMetadata | undefined", desc: "Returns document metadata (title, author, page count, etc.)." },
      { sig: "getPageCount(): number", desc: "Returns total number of pages." },
      { sig: "async getPage(pageNumber: number): Promise<PDFPage | undefined>", desc: "Get a specific page object." },
      { sig: "async getAllPages(): Promise<PDFPage[]>", desc: "Get all page objects." },
      { sig: "close(): void", desc: "Release resources. Always call when done." },
    ],
  },
  {
    category: "Text Extraction",
    items: [
      { sig: "async extractText(options?: TextExtractionOptions): Promise<TextContent[]>", desc: "Extract text from all or specified pages." },
      { sig: "async *streamText(options?: TextExtractionOptions): AsyncGenerator<TextContent>", desc: "Stream text page-by-page for large documents." },
      { sig: "async search(query: string, options?: SearchOptions): Promise<SearchResult[]>", desc: "Search text across the document." },
    ],
  },
  {
    category: "AI & Semantic Chunking",
    items: [
      { sig: "async getAIFeatures(options?: AIOptions): Promise<AIFeatures>", desc: "Run AI analysis: structural analysis, NER, summarization, and semantic chunking." },
      { sig: "async generateSemanticChunks(options?: ChunkingOptions): Promise<SemanticChunk[]>", desc: "Generate semantic chunks for RAG pipelines." },
      { sig: "async *streamSemanticChunks(options?: ChunkingOptions): AsyncGenerator<SemanticChunk>", desc: "Stream chunks for memory-efficient RAG ingestion." },
    ],
  },
  {
    category: "Agentic Ingestion",
    items: [
      { sig: "async ingest(options?: IngestOptions): Promise<IngestResult>", desc: "Single-call ingestion: returns metadata, structure, semantic chunks, and statistics in one object." },
      { sig: "async *streamIngest(options?: IngestOptions): AsyncGenerator<string>", desc: "NDJSON streaming ingestion: emits header, chunk records, and footer for real-time processing." },
    ],
  },
  {
    category: "Rendering",
    items: [
      { sig: "async renderPage(pageNumber: number, canvas: HTMLCanvasElement, options?: RenderOptions): Promise<void>", desc: "Render a page to an HTML Canvas element." },
      { sig: "async renderPageToImage(pageNumber: number, format?, options?): Promise<Blob>", desc: "Render a page to an image Blob." },
      { sig: "async buildTextLayer(pageNumber: number, container: HTMLElement, viewport, options?): Promise<void>", desc: "Build a selectable text layer over rendered canvas." },
      { sig: "createOptimalViewer(container: HTMLElement, options?: RenderOptions): {...}", desc: "Create a full-featured PDF viewer with toolbar and theme toggle." },
    ],
  },
  {
    category: "Images",
    items: [
      { sig: "async extractImages(options?: ImageExtractionOptions): Promise<ImageContent[]>", desc: "Extract embedded images with metadata." },
      { sig: "imageToDataURL(image: ImageContent, format?, quality?): string", desc: "Convert an extracted image to a data URL." },
    ],
  },
  {
    category: "Forms",
    items: [
      { sig: "async getFormFields(): Promise<FormField[]>", desc: "Extract all form fields from the document." },
      { sig: "async fillForm(data: Record<string, any>): Promise<void>", desc: "Fill form fields programmatically." },
      { sig: "async getFormData(): Promise<Record<string, any>>", desc: "Get current form data as key-value pairs." },
    ],
  },
  {
    category: "Export",
    items: [
      { sig: "async exportAs(format: ExportFormat, options?: ExportOptions): Promise<Blob | string>", desc: "Export to text, HTML, Markdown, JSON, XML, CSV, or aPDF." },
      { sig: "async save(): Promise<Blob>", desc: "Save modifications and return the PDF as a Blob." },
    ],
  },
  {
    category: "aPDF (Agentic PDF)",
    items: [
      { sig: "async generateAPDFMetadata(): Promise<APDFDocument>", desc: "Generate aPDF metadata with identifiers, structure, and AI content." },
      { sig: "async generateAPDFBinary(options?): Promise<Uint8Array>", desc: "Generate an aPDF binary file (PDF + metadata envelope)." },
      { sig: "static async readAPDF(data: Uint8Array, password?): Promise<{metadata, pdfData}>", desc: "Read an aPDF file and extract metadata + PDF." },
    ],
  },
  {
    category: "Pretext Layout",
    items: [
      { sig: "static prepareText(text: string, font: string, options?): PreparedTextWithSegments", desc: "Measure text glyphs for layout. Requires enablePretextLayout: true." },
      { sig: "static layoutText(prepared, maxWidth: number, lineHeight: number): PretextLayoutResult", desc: "Perform line-breaking on prepared text. Requires enablePretextLayout: true." },
    ],
  },
  {
    category: "Agent Discovery & Ontology",
    items: [
      { sig: "static describe(): LibraryOntology", desc: "Get complete machine-readable ontology (JSON-LD)." },
      { sig: "static getCapabilities(): Capability[]", desc: "Get all capabilities organized by category." },
      { sig: "static getMethodSignatures(): MethodDescriptor[]", desc: "Get all method signatures for code generation." },
      { sig: "static getWorkflows(): Workflow[]", desc: "Get 16 pre-built workflow templates including agentic ingestion." },
      { sig: "static getToolSchemas(format?): any[]", desc: "Get tool schemas in OpenAI or Anthropic format." },
      { sig: "static getMCPManifest(): MCPManifest", desc: "Get MCP (Model Context Protocol) manifest." },
    ],
  },
];

export default function ApiPage() {
  return (
    <div className="mx-auto max-w-4xl px-4 sm:px-6 py-12">
      <h1 className="text-4xl font-bold tracking-tight mb-2" style={{ color: "var(--text)" }}>
        API Reference
      </h1>
      <p className="mb-10" style={{ color: "var(--text-muted)" }}>
        Complete reference for the <code>AgenticPDF</code> class and related APIs.
      </p>

      {methods.map((group) => (
        <section key={group.category} className="mb-12" id={group.category.toLowerCase().replace(/[^a-z]+/g, "-")}>
          <h2 className="text-2xl font-bold mb-6 pb-2 border-b" style={{ color: "var(--text)", borderColor: "var(--border)" }}>
            {group.category}
          </h2>
          <div className="space-y-4">
            {group.items.map((m) => (
              <div
                key={m.sig}
                className="rounded-lg p-4"
                style={{ background: "var(--bg-card)", border: "1px solid var(--border)" }}
              >
                <code className="text-sm block mb-2 break-all" style={{ color: "var(--accent)" }}>
                  {m.sig}
                </code>
                <p className="text-sm" style={{ color: "var(--text-muted)" }}>
                  {m.desc}
                </p>
              </div>
            ))}
          </div>
        </section>
      ))}

      {/* Key interfaces */}
      <section className="mb-12">
        <h2 className="text-2xl font-bold mb-6 pb-2 border-b" style={{ color: "var(--text)", borderColor: "var(--border)" }}>
          Key Interfaces
        </h2>

        <h3 className="text-lg font-semibold mt-6 mb-3" style={{ color: "var(--text)" }}>PDFOptions</h3>
        <HighlightedCode code={`interface PDFOptions {
  lazyLoad?: boolean;           // Load pages on-demand
  useWebWorkers?: boolean;      // CPU-intensive ops in workers
  workerUrl?: string;           // Worker script URL
  maxMemoryUsage?: number;      // Byte limit for memory
  cachePages?: boolean;         // Cache page objects
  enablePretextLayout?: boolean;// Enable text layout engine
  streamOptions?: StreamOptions;
  renderOptions?: RenderOptions;
}`} />

        <h3 className="text-lg font-semibold mt-8 mb-3" style={{ color: "var(--text)" }}>EmbeddingProvider</h3>
        <HighlightedCode code={`interface EmbeddingProvider {
  model: string;
  dimensions: number;
  generate(text: string): Promise<Float32Array>;
  generateBatch(texts: string[]): Promise<Float32Array[]>;
}`} />

        <h3 className="text-lg font-semibold mt-8 mb-3" style={{ color: "var(--text)" }}>AIOptions</h3>
        <HighlightedCode code={`interface AIOptions {
  embeddingProvider?: EmbeddingProvider;
  enableStructuralAnalysis?: boolean;
  enableSemanticChunking?: boolean;
  enableNER?: boolean;
  enableSummarization?: boolean;
  chunkSize?: number;
  chunkOverlap?: number;
}`} />

        <h3 className="text-lg font-semibold mt-8 mb-3" style={{ color: "var(--text)" }}>ChunkingOptions</h3>
        <HighlightedCode code={`interface ChunkingOptions {
  strategy?: 'fixed' | 'sliding' | 'recursive' | 'semantic';
  maxChunkSize?: number;      // Default: 500
  minChunkSize?: number;      // Default: 100
  overlapSize?: number;
  preserveParagraphs?: boolean;
  includeMetadata?: boolean;
}`} />
      </section>
    </div>
  );
}
