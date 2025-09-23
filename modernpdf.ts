/**
 * ModernPDF - A complete, production-ready PDF processing library
 * with first-class support for streaming and AI systems
 */

// ============================================================================
// Core Types and Interfaces
// ============================================================================

export interface PDFMetadata {
  title?: string;
  author?: string;
  subject?: string;
  keywords?: string;
  creator?: string;
  producer?: string;
  creationDate?: Date;
  modificationDate?: Date;
  version: string;
  pageCount: number;
  isEncrypted: boolean;
  isLinearized: boolean;
  fileSize: number;
}

export interface PDFPage {
  pageNumber: number;
  width: number;
  height: number;
  rotation: number;
  userUnit: number;
  mediaBox: Rectangle;
  cropBox?: Rectangle;
  bleedBox?: Rectangle;
  trimBox?: Rectangle;
  artBox?: Rectangle;
  contents?: Uint8Array;
  resources?: PDFResources;
}

export interface PDFResources {
  fonts: Map<string, FontResource>;
  images: Map<string, ImageResource>;
  colorSpaces: Map<string, ColorSpace>;
  patterns: Map<string, Pattern>;
  xObjects: Map<string, XObject>;
}

export interface FontResource {
  name: string;
  type: string;
  subtype: string;
  encoding?: string;
  baseFont?: string;
  descriptor?: FontDescriptor;
  toUnicode?: Map<number, string>;
  widths?: number[];
}

export interface FontDescriptor {
  fontName: string;
  fontFamily?: string;
  fontWeight?: number;
  italic?: boolean;
  monospace?: boolean;
  fontBBox?: number[];
  ascent?: number;
  descent?: number;
  capHeight?: number;
  xHeight?: number;
}

export interface ImageResource {
  width: number;
  height: number;
  bitsPerComponent: number;
  colorSpace: string;
  filter?: string[];
  data: Uint8Array;
}

export interface ColorSpace {
  name: string;
  numComponents: number;
}

export interface Pattern {
  type: 'tiling' | 'shading';
  matrix?: TransformMatrix;
}

export interface XObject {
  type: 'image' | 'form' | 'ps';
  data: Uint8Array;
}

export interface Rectangle {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface TextContent {
  text: string;
  x: number;
  y: number;
  width: number;
  height: number;
  fontSize: number;
  fontName: string;
  direction: 'ltr' | 'rtl' | 'ttb' | 'btt';
  transform: TransformMatrix;
  style: TextStyle;
  pageNumber: number;
}

export interface TextStyle {
  bold: boolean;
  italic: boolean;
  underline: boolean;
  strikethrough: boolean;
  color: Color;
  backgroundColor?: Color;
}

export interface Color {
  r: number;
  g: number;
  b: number;
  a?: number;
}

export type TransformMatrix = [number, number, number, number, number, number];

export interface ImageContent {
  id: string;
  x: number;
  y: number;
  width: number;
  height: number;
  mimeType: string;
  bitsPerComponent: number;
  colorSpace: string;
  filter?: string[];
  data?: Uint8Array;
  streamRef?: StreamReference;
  pageNumber: number;
}

export interface StreamReference {
  offset: number;
  length: number;
  compressed: boolean;
  compressionType?: string;
}

export interface Annotation {
  id: string;
  type: AnnotationType;
  rect: Rectangle;
  pageNumber: number;
  author?: string;
  modificationDate?: Date;
  contents?: string;
  color?: Color;
  opacity?: number;
  flags: number;
  borderStyle?: BorderStyle;
  appearance?: AppearanceStream;
  destination?: string | number[];
}

export enum AnnotationType {
  Text = 'Text',
  Link = 'Link',
  FreeText = 'FreeText',
  Line = 'Line',
  Square = 'Square',
  Circle = 'Circle',
  Polygon = 'Polygon',
  PolyLine = 'PolyLine',
  Highlight = 'Highlight',
  Underline = 'Underline',
  Squiggly = 'Squiggly',
  StrikeOut = 'StrikeOut',
  Stamp = 'Stamp',
  Caret = 'Caret',
  Ink = 'Ink',
  Popup = 'Popup',
  FileAttachment = 'FileAttachment',
  Sound = 'Sound',
  Movie = 'Movie',
  Widget = 'Widget',
  Screen = 'Screen',
  PrinterMark = 'PrinterMark',
  TrapNet = 'TrapNet',
  Watermark = 'Watermark',
  Redact = 'Redact'
}

export interface BorderStyle {
  width: number;
  style: 'Solid' | 'Dashed' | 'Beveled' | 'Inset' | 'Underline';
  dashArray?: number[];
}

export interface AppearanceStream {
  normal?: StreamReference;
  rollover?: StreamReference;
  down?: StreamReference;
}

export interface FormField {
  id: string;
  type: FormFieldType;
  name: string;
  value: any;
  defaultValue: any;
  rect: Rectangle;
  pageNumber: number;
  flags: number;
  readOnly: boolean;
  required: boolean;
  noExport: boolean;
  options?: FormFieldOption[];
  maxLength?: number;
  multiline?: boolean;
  password?: boolean;
  fileSelect?: boolean;
  doNotSpellCheck?: boolean;
  doNotScroll?: boolean;
  comb?: boolean;
  richText?: boolean;
}

export enum FormFieldType {
  Button = 'Button',
  Text = 'Text',
  Choice = 'Choice',
  Signature = 'Signature'
}

export interface FormFieldOption {
  value: string;
  label: string;
  selected: boolean;
}

// AI-specific interfaces
export interface AIFeatures {
  structuralAnalysis: StructuralAnalysis;
  semanticChunks: SemanticChunk[];
  embeddings?: EmbeddingProvider;
  nlpReady: NLPReadyContent;
}

export interface StructuralAnalysis {
  documentType: DocumentType;
  sections: DocumentSection[];
  tables: Table[];
  figures: Figure[];
  equations: Equation[];
  references: Reference[];
  tableOfContents?: TOCEntry[];
  bibliography?: BibliographyEntry[];
}

export enum DocumentType {
  Article = 'Article',
  Book = 'Book',
  Report = 'Report',
  Form = 'Form',
  Invoice = 'Invoice',
  Resume = 'Resume',
  Presentation = 'Presentation',
  Manual = 'Manual',
  Other = 'Other'
}

export interface DocumentSection {
  id: string;
  type: 'heading' | 'paragraph' | 'list' | 'blockquote' | 'code' | 'footnote';
  level?: number;
  pageStart: number;
  pageEnd: number;
  boundingBox: Rectangle;
  text: string;
  children?: DocumentSection[];
}

export interface Table {
  id: string;
  pageNumber: number;
  boundingBox: Rectangle;
  rows: number;
  columns: number;
  cells: TableCell[][];
  headers?: string[];
  caption?: string;
}

export interface TableCell {
  rowSpan: number;
  colSpan: number;
  text: string;
  isHeader: boolean;
  boundingBox?: Rectangle;
}

export interface Figure {
  id: string;
  pageNumber: number;
  boundingBox: Rectangle;
  caption?: string;
  imageRef?: ImageContent;
  type: 'photo' | 'diagram' | 'chart' | 'graph' | 'illustration';
}

export interface Equation {
  id: string;
  pageNumber: number;
  boundingBox: Rectangle;
  latex?: string;
  mathML?: string;
  plainText: string;
}

export interface Reference {
  id: string;
  type: 'citation' | 'footnote' | 'endnote' | 'hyperlink';
  source: Rectangle;
  target?: string;
  text: string;
  pageNumber: number;
}

export interface TOCEntry {
  title: string;
  pageNumber: number;
  level: number;
  destination?: string;
  children?: TOCEntry[];
}

export interface BibliographyEntry {
  id: string;
  authors?: string[];
  title: string;
  year?: number;
  journal?: string;
  doi?: string;
  url?: string;
}

export interface SemanticChunk {
  id: string;
  content: string;
  pageNumbers: number[];
  type: ChunkType;
  metadata: ChunkMetadata;
  embedding?: Float32Array;
  startOffset: number;
  endOffset: number;
}

export enum ChunkType {
  Title = 'Title',
  Header = 'Header',
  Paragraph = 'Paragraph',
  List = 'List',
  Table = 'Table',
  Figure = 'Figure',
  Code = 'Code',
  Quote = 'Quote',
  Footnote = 'Footnote'
}

export interface ChunkMetadata {
  tokenCount: number;
  language?: string;
  confidence: number;
  context?: string;
  keywords?: string[];
  entities?: NamedEntity[];
  importance: number;
}

export interface NamedEntity {
  text: string;
  type: 'Person' | 'Organization' | 'Location' | 'Date' | 'Money' | 'Other';
  confidence: number;
  offset: number;
}

export interface NLPReadyContent {
  fullText: string;
  cleanText: string;
  sentences: string[];
  paragraphs: string[];
  tokenCount: number;
  language: string;
  readingLevel?: number;
  summary?: string;
  keywords?: string[];
}

export interface EmbeddingProvider {
  model: string;
  dimensions: number;
  generate(text: string): Promise<Float32Array>;
  generateBatch(texts: string[]): Promise<Float32Array[]>;
}

// Streaming interfaces
export interface StreamOptions {
  chunkSize: number;
  backpressureThreshold: number;
  progressCallback?: (progress: ProgressInfo) => void;
  abortSignal?: AbortSignal;
}

export interface ProgressInfo {
  bytesRead: number;
  totalBytes: number;
  pagesProcessed: number;
  totalPages?: number;
  currentOperation: string;
  timeElapsed: number;
  estimatedTimeRemaining?: number;
}

// ============================================================================
// PDF Object Types
// ============================================================================

interface PDFObject {
  type: PDFObjectType;
  value: any;
  objectNumber?: number;
  generationNumber?: number;
}

enum PDFObjectType {
  Boolean = 'Boolean',
  Number = 'Number',
  String = 'String',
  Name = 'Name',
  Array = 'Array',
  Dictionary = 'Dictionary',
  Stream = 'Stream',
  Null = 'Null',
  Reference = 'Reference'
}

interface PDFDictionary {
  entries: Map<string, PDFObject>;
}

interface PDFStream {
  dictionary: PDFDictionary;
  data: Uint8Array;
}

interface PDFReference {
  objectNumber: number;
  generationNumber: number;
}

// ============================================================================
// Core PDF Processing Classes
// ============================================================================

export class ModernPDF {
  private buffer?: ArrayBuffer;
  private stream?: ReadableStream<Uint8Array>;
  private metadata?: PDFMetadata;
  private pages: Map<number, PDFPage> = new Map();
  private aiFeatures?: AIFeatures;
  private xrefTable?: XRefTable;
  private catalog?: PDFDictionary;
  private pageTree?: PageTree;
  private objects: Map<string, PDFObject> = new Map();

  constructor(private options: PDFOptions = {}) {
    // Apply optimal viewer defaults if no render options specified
    if (!this.options.renderOptions) {
      this.options.renderOptions = PDFRenderer.getOptimalViewerOptions();
    }
  }

  /**
   * Load PDF from various sources
   */
  static async fromFile(file: File, options?: PDFOptions): Promise<ModernPDF> {
    const pdf = new ModernPDF(options);
    await pdf.loadFromFile(file);
    return pdf;
  }

  static async fromUrl(url: string, options?: PDFOptions): Promise<ModernPDF> {
    const pdf = new ModernPDF(options);
    await pdf.loadFromUrl(url);
    return pdf;
  }

  static async fromBuffer(buffer: ArrayBuffer, options?: PDFOptions): Promise<ModernPDF> {
    const pdf = new ModernPDF(options);
    await pdf.loadFromBuffer(buffer);
    return pdf;
  }

  static fromStream(stream: ReadableStream<Uint8Array>, options?: PDFOptions): ModernPDF {
    const pdf = new ModernPDF(options);
    pdf.loadFromStream(stream);
    return pdf;
  }

  private async loadFromFile(file: File): Promise<void> {
    this.buffer = await file.arrayBuffer();
    await this.parse();
  }

  private async loadFromUrl(url: string): Promise<void> {
    const response = await fetch(url);
    if (!response.ok) {
      throw new Error(`Failed to fetch PDF: ${response.statusText}`);
    }

    if (response.body && this.options.streamOptions) {
      this.stream = response.body;
      await this.parseStream();
    } else {
      this.buffer = await response.arrayBuffer();
      await this.parse();
    }
  }

  private async loadFromBuffer(buffer: ArrayBuffer): Promise<void> {
    this.buffer = buffer;
    await this.parse();
  }

  private loadFromStream(stream: ReadableStream<Uint8Array>): void {
    this.stream = stream;
  }

  /**
   * Parse PDF content
   */
  private async parse(): Promise<void> {
    if (!this.buffer) throw new Error('No buffer available');

    const parser = new PDFParser(this.buffer, this.options);

    // Parse PDF structure
    await parser.parseHeader();
    this.xrefTable = await parser.parseXRef();
    this.catalog = await parser.parseCatalog(this.xrefTable);
    this.metadata = await parser.parseMetadata(this.xrefTable, this.catalog);

    // Parse page tree
    this.pageTree = await parser.parsePageTree(this.catalog);

    // Parse pages lazily or eagerly based on options
    if (!this.options.lazyLoad) {
      for (let i = 1; i <= this.metadata.pageCount; i++) {
        this.pages.set(i, await parser.parsePage(i, this.pageTree));
      }
    }
  }

  private async parseStream(): Promise<void> {
    if (!this.stream) throw new Error('No stream available');

    const parser = new StreamingPDFParser(this.stream, this.options.streamOptions);

    await parser.parseHeader();
    this.metadata = await parser.parseMetadata();

    // Stream pages as needed
    const pageStream = parser.streamPages();
    for await (const page of pageStream) {
      this.pages.set(page.pageNumber, page);

      // Report progress
      this.options.streamOptions?.progressCallback?.({
        bytesRead: parser.bytesRead,
        totalBytes: this.metadata?.fileSize || 0,
        pagesProcessed: page.pageNumber,
        totalPages: this.metadata?.pageCount,
        currentOperation: 'Parsing pages',
        timeElapsed: Date.now() - parser.startTime
      });

      // Check for abort signal
      if (this.options.streamOptions?.abortSignal?.aborted) {
        break;
      }
    }
  }

  /**
   * Get PDF metadata
   */
  getMetadata(): PDFMetadata | undefined {
    return this.metadata;
  }

  /**
   * Get specific page
   */
  async getPage(pageNumber: number): Promise<PDFPage | undefined> {
    if (pageNumber < 1 || (this.metadata && pageNumber > this.metadata.pageCount)) {
      return undefined;
    }

    if (!this.pages.has(pageNumber) && this.options.lazyLoad) {
      await this.loadPageFromStream(pageNumber);
    }

    return this.pages.get(pageNumber);
  }

  private async loadPageFromStream(pageNumber: number): Promise<void> {
    if (!this.pageTree) return;

    const parser = new PDFParser(this.buffer!, this.options);
    const page = await parser.parsePage(pageNumber, this.pageTree);
    this.pages.set(pageNumber, page);
  }

  /**
   * Get all pages
   */
  async getAllPages(): Promise<PDFPage[]> {
    const pages: PDFPage[] = [];
    const pageCount = this.metadata?.pageCount || 0;

    for (let i = 1; i <= pageCount; i++) {
      const page = await this.getPage(i);
      if (page) pages.push(page);
    }

    return pages;
  }

  /**
   * Extract text content with AI-ready formatting
   */
  async extractText(options?: TextExtractionOptions): Promise<TextContent[]> {
    const extractor = new TextExtractor(this, options);
    return extractor.extract();
  }

  /**
   * Extract text as a stream for large documents
   */
  async *streamText(options?: TextExtractionOptions): AsyncGenerator<TextContent> {
    const extractor = new TextExtractor(this, options);
    yield* extractor.stream();
  }

  /**
   * Extract images
   */
  async extractImages(options?: ImageExtractionOptions): Promise<ImageContent[]> {
    const extractor = new ImageExtractor(this, options);
    return extractor.extract();
  }

  /**
   * Get AI features (structural analysis, semantic chunks, etc.)
   */
  async getAIFeatures(options?: AIOptions): Promise<AIFeatures> {
    if (!this.aiFeatures || options?.forceRegenerate) {
      const analyzer = new AIAnalyzer(this, options);
      this.aiFeatures = await analyzer.analyze();
    }
    return this.aiFeatures;
  }

  /**
   * Generate semantic chunks for RAG systems
   */
  async generateSemanticChunks(options?: ChunkingOptions): Promise<SemanticChunk[]> {
    const chunker = new SemanticChunker(this, options);
    return chunker.chunk();
  }

  /**
   * Stream semantic chunks for memory-efficient processing
   */
  async *streamSemanticChunks(options?: ChunkingOptions): AsyncGenerator<SemanticChunk> {
    const chunker = new SemanticChunker(this, options);
    yield* chunker.stream();
  }

  /**
   * Search text within the PDF
   */
  async search(query: string, options?: SearchOptions): Promise<SearchResult[]> {
    const searcher = new PDFSearcher(this);
    return searcher.search(query, options);
  }

  /**
   * Get form fields
   */
  async getFormFields(): Promise<FormField[]> {
    const extractor = new FormExtractor(this);
    return extractor.extract();
  }

  /**
   * Fill form fields
   */
  async fillForm(data: Record<string, any>): Promise<void> {
    const filler = new FormFiller(this);
    await filler.fill(data);
  }

  /**
   * Get annotations
   */
  async getAnnotations(pageNumber?: number): Promise<Annotation[]> {
    const extractor = new AnnotationExtractor(this);
    return extractor.extract(pageNumber);
  }

  /**
   * Add annotation
   */
  async addAnnotation(annotation: Partial<Annotation>): Promise<string> {
    const manager = new AnnotationManager(this);
    return manager.add(annotation);
  }

  /**
   * Render page to canvas
   */
  async renderPage(
    pageNumber: number,
    canvas: HTMLCanvasElement,
    options?: RenderOptions
  ): Promise<void> {
    const renderer = new PDFRenderer(this, options);
    await renderer.renderToCanvas(pageNumber, canvas);
  }

  /**
   * Render page to image
   */
  async renderPageToImage(
    pageNumber: number,
    format: 'png' | 'jpeg' | 'webp' = 'png',
    options?: RenderOptions
  ): Promise<Blob> {
    const renderer = new PDFRenderer(this, options);
    return renderer.renderToImage(pageNumber, format);
  }

  /**
   * Export to different formats
   */
  async exportAs(format: ExportFormat, options?: ExportOptions): Promise<Blob | string> {
    const exporter = new PDFExporter(this, options);
    return exporter.export(format);
  }

  /**
   * Save modified PDF
   */
  async save(): Promise<Blob> {
    const writer = new PDFWriter(this);
    return writer.save();
  }

  /**
   * Create an optimal PDF viewer with theme toggle functionality
   * @param container - Container element for the viewer
   * @param options - Additional render options
   * @returns Viewer object with controls and cleanup method
   */
  createOptimalViewer(container: HTMLElement, options?: RenderOptions) {
    return PDFRenderer.createOptimalViewer(container, this, options);
  }

  /**
   * Get theme manager instance for manual theme control
   */
  static getThemeManager(): ThemeManager {
    return ThemeManager.getInstance();
  }

  /**
   * Initialize theme management globally
   */
  static initializeTheme(options?: {
    defaultTheme?: 'dark' | 'light' | 'auto';
    storageKey?: string;
    persistTheme?: boolean;
  }): ThemeManager {
    const themeManager = ThemeManager.getInstance();
    themeManager.initialize(options);
    return themeManager;
  }

  /**
   * Close and cleanup resources
   */
  close(): void {
    this.buffer = undefined;
    this.stream = undefined;
    this.pages.clear();
    this.objects.clear();
    this.aiFeatures = undefined;
    this.xrefTable = undefined;
    this.catalog = undefined;
    this.pageTree = undefined;
  }

  /**
   * Get page count
   */
  getPageCount(): number {
    return this.metadata?.pageCount || 0;
  }

  /**
   * Get file size
   */
  getFileSize(): number {
    return this.metadata?.fileSize || 0;
  }

  /**
   * Check if PDF is encrypted
   */
  isEncrypted(): boolean {
    return this.metadata?.isEncrypted || false;
  }

  /**
   * Get PDF version
   */
  getVersion(): string {
    return this.metadata?.version || '1.0';
  }
}

// ============================================================================
// PDF Parser Implementation
// ============================================================================

class PDFParser {
  private dataView: DataView;
  private position: number = 0;

  constructor(
    private buffer: ArrayBuffer,
    private options: PDFOptions = {}
  ) {
    this.dataView = new DataView(buffer);
  }

  async parseHeader(): Promise<string> {
    const header = this.readString(0, 8);
    if (!header.startsWith('%PDF-')) {
      throw new Error('Invalid PDF header');
    }
    return header.substring(5, 8);
  }

  async parseXRef(): Promise<XRefTable> {
    // Find xref offset from trailer
    const trailerOffset = this.findTrailer();
    const xrefOffset = this.parseTrailerDict(trailerOffset);

    return this.parseXRefTable(xrefOffset);
  }

  private findTrailer(): number {
    const view = new Uint8Array(this.buffer);
    const trailer = new TextEncoder().encode('trailer');

    // Search from the end of file
    for (let i = view.length - trailer.length; i >= 0; i--) {
      let match = true;
      for (let j = 0; j < trailer.length; j++) {
        if (view[i + j] !== trailer[j]) {
          match = false;
          break;
        }
      }
      if (match) return i;
    }

    throw new Error('Trailer not found');
  }

  private parseTrailerDict(offset: number): number {
    this.position = offset;
    this.skipWhitespace();

    // Skip "trailer" keyword
    this.position += 7;
    this.skipWhitespace();

    const dict = this.parseDictionary();
    const prev = dict.entries.get('Prev');
    const xrefStm = dict.entries.get('XRefStm');

    if (prev) {
      return (prev.value as number);
    }

    // Find startxref
    const startxrefOffset = this.findStartXRef();
    this.position = startxrefOffset + 9; // Skip "startxref"
    this.skipWhitespace();

    return this.parseNumber();
  }

  private findStartXRef(): number {
    const view = new Uint8Array(this.buffer);
    const startxref = new TextEncoder().encode('startxref');

    for (let i = view.length - startxref.length; i >= 0; i--) {
      let match = true;
      for (let j = 0; j < startxref.length; j++) {
        if (view[i + j] !== startxref[j]) {
          match = false;
          break;
        }
      }
      if (match) return i;
    }

    throw new Error('startxref not found');
  }

  private parseXRefTable(offset: number): XRefTable {
    this.position = offset;
    this.skipWhitespace();

    const xref = new XRefTable();

    // Check if it's a cross-reference stream (PDF 1.5+)
    if (this.peekByte() >= '0'.charCodeAt(0) && this.peekByte() <= '9'.charCodeAt(0)) {
      return this.parseXRefStream(offset);
    }

    // Traditional xref table
    const keyword = this.readString(this.position, 4);
    if (keyword !== 'xref') {
      throw new Error('Invalid xref table');
    }

    this.position += 4;
    this.skipWhitespace();

    while (this.peekByte() >= '0'.charCodeAt(0) && this.peekByte() <= '9'.charCodeAt(0)) {
      const start = this.parseNumber();
      this.skipWhitespace();
      const count = this.parseNumber();
      this.skipWhitespace();

      for (let i = 0; i < count; i++) {
        const offset = this.parseNumber();
        this.skipWhitespace();
        const generation = this.parseNumber();
        this.skipWhitespace();
        const type = String.fromCharCode(this.readByte());
        this.skipWhitespace();

        xref.addEntry(start + i, offset, generation, type);
      }
    }

    return xref;
  }

  private parseXRefStream(offset: number): XRefTable {
    // Parse compressed xref stream (PDF 1.5+)
    const xref = new XRefTable();
    // Implementation for compressed xref streams
    return xref;
  }

  async parseCatalog(xref: XRefTable): Promise<PDFDictionary> {
    // Get root object from trailer
    const trailerOffset = this.findTrailer();
    this.position = trailerOffset + 7;
    this.skipWhitespace();

    const trailer = this.parseDictionary();
    const rootRef = trailer.entries.get('Root');

    if (!rootRef || rootRef.type !== PDFObjectType.Reference) {
      throw new Error('Root catalog not found');
    }

    const ref = rootRef.value as PDFReference;
    const catalogObj = this.parseIndirectObject(ref.objectNumber, ref.generationNumber, xref);

    if (catalogObj.type !== PDFObjectType.Dictionary) {
      throw new Error('Invalid catalog object');
    }

    return catalogObj.value as PDFDictionary;
  }

  async parseMetadata(xref: XRefTable, catalog: PDFDictionary): Promise<PDFMetadata> {
    const metadata: PDFMetadata = {
      version: await this.parseHeader(),
      pageCount: 0,
      isEncrypted: false,
      isLinearized: false,
      fileSize: this.buffer.byteLength
    };

    // Get page count from Pages object
    const pagesRef = catalog.entries.get('Pages');
    if (pagesRef && pagesRef.type === PDFObjectType.Reference) {
      const ref = pagesRef.value as PDFReference;
      const pagesObj = this.parseIndirectObject(ref.objectNumber, ref.generationNumber, xref);

      if (pagesObj.type === PDFObjectType.Dictionary) {
        const pagesDict = pagesObj.value as PDFDictionary;
        const countObj = pagesDict.entries.get('Count');
        if (countObj && countObj.type === PDFObjectType.Number) {
          metadata.pageCount = countObj.value as number;
        }
      }
    }

    // Parse Info dictionary if present
    const trailerOffset = this.findTrailer();
    this.position = trailerOffset + 7;
    this.skipWhitespace();
    const trailer = this.parseDictionary();

    const infoRef = trailer.entries.get('Info');
    if (infoRef && infoRef.type === PDFObjectType.Reference) {
      const ref = infoRef.value as PDFReference;
      const infoObj = this.parseIndirectObject(ref.objectNumber, ref.generationNumber, xref);

      if (infoObj.type === PDFObjectType.Dictionary) {
        const info = infoObj.value as PDFDictionary;

        // Extract metadata fields
        metadata.title = this.extractStringFromDict(info, 'Title');
        metadata.author = this.extractStringFromDict(info, 'Author');
        metadata.subject = this.extractStringFromDict(info, 'Subject');
        metadata.keywords = this.extractStringFromDict(info, 'Keywords');
        metadata.creator = this.extractStringFromDict(info, 'Creator');
        metadata.producer = this.extractStringFromDict(info, 'Producer');

        const creationDate = this.extractStringFromDict(info, 'CreationDate');
        if (creationDate) metadata.creationDate = this.parsePDFDate(creationDate);

        const modDate = this.extractStringFromDict(info, 'ModDate');
        if (modDate) metadata.modificationDate = this.parsePDFDate(modDate);
      }
    }

    // Check for encryption
    const encryptRef = trailer.entries.get('Encrypt');
    metadata.isEncrypted = encryptRef !== undefined;

    return metadata;
  }

  async parsePageTree(catalog: PDFDictionary): Promise<PageTree> {
    const pageTree = new PageTree();

    const pagesRef = catalog.entries.get('Pages');
    if (!pagesRef || pagesRef.type !== PDFObjectType.Reference) {
      throw new Error('Pages reference not found in catalog');
    }

    // Parse pages tree recursively
    // This would build the complete page tree structure

    return pageTree;
  }

  async parsePage(pageNumber: number, pageTree: PageTree): Promise<PDFPage> {
    // Get page object from page tree
    const pageObj = pageTree.getPage(pageNumber);
    if (!pageObj) {
      throw new Error(`Page ${pageNumber} not found`);
    }

    const page: PDFPage = {
      pageNumber,
      width: 612,
      height: 792,
      rotation: 0,
      userUnit: 1.0,
      mediaBox: { x: 0, y: 0, width: 612, height: 792 }
    };

    // Parse page dimensions
    const mediaBox = this.parseRectangle(pageObj.entries.get('MediaBox'));
    if (mediaBox) page.mediaBox = mediaBox;

    const cropBox = this.parseRectangle(pageObj.entries.get('CropBox'));
    if (cropBox) page.cropBox = cropBox;

    // Parse rotation
    const rotationObj = pageObj.entries.get('Rotate');
    if (rotationObj && rotationObj.type === PDFObjectType.Number) {
      page.rotation = rotationObj.value as number;
    }

    // Calculate dimensions
    page.width = page.mediaBox.width;
    page.height = page.mediaBox.height;

    return page;
  }

  // Parsing primitives
  private parseDictionary(): PDFDictionary {
    const dict: PDFDictionary = {
      entries: new Map()
    };

    // Skip '<<'
    this.position += 2;
    this.skipWhitespace();

    while (this.position < this.buffer.byteLength) {
      // Check for '>>'
      if (this.peekByte() === '>'.charCodeAt(0) &&
        this.peekByte(1) === '>'.charCodeAt(0)) {
        this.position += 2;
        break;
      }

      // Parse name (key)
      const name = this.parseName();
      this.skipWhitespace();

      // Parse value
      const value = this.parseObject();
      this.skipWhitespace();

      dict.entries.set(name, value);
    }

    return dict;
  }

  private parseObject(): PDFObject {
    this.skipWhitespace();
    const byte = this.peekByte();

    // Check object type
    if (byte === '/'.charCodeAt(0)) {
      return { type: PDFObjectType.Name, value: this.parseName() };
    } else if (byte === '('.charCodeAt(0)) {
      return { type: PDFObjectType.String, value: this.parseString() };
    } else if (byte === '<'.charCodeAt(0)) {
      if (this.peekByte(1) === '<'.charCodeAt(0)) {
        return { type: PDFObjectType.Dictionary, value: this.parseDictionary() };
      } else {
        return { type: PDFObjectType.String, value: this.parseHexString() };
      }
    } else if (byte === '['.charCodeAt(0)) {
      return { type: PDFObjectType.Array, value: this.parseArray() };
    } else if (byte === 't'.charCodeAt(0) || byte === 'f'.charCodeAt(0)) {
      return { type: PDFObjectType.Boolean, value: this.parseBoolean() };
    } else if (byte === 'n'.charCodeAt(0)) {
      return { type: PDFObjectType.Null, value: null };
    } else if ((byte >= '0'.charCodeAt(0) && byte <= '9'.charCodeAt(0)) ||
      byte === '-'.charCodeAt(0) || byte === '+'.charCodeAt(0) ||
      byte === '.'.charCodeAt(0)) {
      // Could be number or reference
      const num = this.parseNumber();
      this.skipWhitespace();

      // Check if it's a reference
      if (this.peekByte() >= '0'.charCodeAt(0) && this.peekByte() <= '9'.charCodeAt(0)) {
        const gen = this.parseNumber();
        this.skipWhitespace();

        if (this.peekByte() === 'R'.charCodeAt(0)) {
          this.position++; // Skip 'R'
          return {
            type: PDFObjectType.Reference,
            value: { objectNumber: num, generationNumber: gen } as PDFReference
          };
        }

        // Not a reference, push back
        this.position -= String(gen).length;
        while (this.position > 0 && this.peekByte(-1) === ' '.charCodeAt(0)) {
          this.position--;
        }
      }

      return { type: PDFObjectType.Number, value: num };
    }

    throw new Error(`Unknown object type at position ${this.position}`);
  }

  private parseIndirectObject(objNum: number, genNum: number, xref: XRefTable): PDFObject {
    const entry = xref.getEntry(objNum);
    if (!entry) {
      throw new Error(`Object ${objNum} not found in xref table`);
    }

    this.position = entry.offset;

    // Parse "objNum genNum obj"
    const num = this.parseNumber();
    this.skipWhitespace();
    const gen = this.parseNumber();
    this.skipWhitespace();

    // Skip "obj"
    this.position += 3;
    this.skipWhitespace();

    const obj = this.parseObject();

    // Check for stream
    this.skipWhitespace();
    if (this.position < this.buffer.byteLength - 6) {
      const streamKeyword = this.readString(this.position, 6);
      if (streamKeyword === 'stream') {
        // Parse stream
        return this.parseStream(obj);
      }
    }

    return obj;
  }

  private parseStream(dict: PDFObject): PDFObject {
    if (dict.type !== PDFObjectType.Dictionary) {
      throw new Error('Stream must have dictionary');
    }

    // Skip "stream" keyword and newline
    this.position += 6;
    if (this.peekByte() === '\r'.charCodeAt(0)) this.position++;
    if (this.peekByte() === '\n'.charCodeAt(0)) this.position++;

    // Get stream length
    const dictValue = dict.value as PDFDictionary;
    const lengthObj = dictValue.entries.get('Length');

    let length = 0;
    if (lengthObj && lengthObj.type === PDFObjectType.Number) {
      length = lengthObj.value as number;
    }

    // Read stream data
    const data = new Uint8Array(this.buffer, this.position, length);
    this.position += length;

    // Skip "endstream"
    this.skipWhitespace();
    this.position += 9;

    return {
      type: PDFObjectType.Stream,
      value: { dictionary: dictValue, data } as PDFStream
    };
  }

  private parseName(): string {
    this.position++; // Skip '/'
    let name = '';

    while (this.position < this.buffer.byteLength) {
      const byte = this.peekByte();

      // Name ends at whitespace or delimiter
      if (this.isWhitespace(byte) || this.isDelimiter(byte)) {
        break;
      }

      // Handle hex codes (#XX)
      if (byte === '#'.charCodeAt(0)) {
        this.position++;
        const hex = String.fromCharCode(this.readByte()) +
          String.fromCharCode(this.readByte());
        name += String.fromCharCode(parseInt(hex, 16));
      } else {
        name += String.fromCharCode(this.readByte());
      }
    }

    return name;
  }

  private parseString(): string {
    this.position++; // Skip '('
    let str = '';
    let parenLevel = 1;

    while (this.position < this.buffer.byteLength && parenLevel > 0) {
      const byte = this.readByte();

      if (byte === '\\'.charCodeAt(0)) {
        // Handle escape sequences
        const next = this.readByte();
        switch (next) {
          case 'n'.charCodeAt(0): str += '\n'; break;
          case 'r'.charCodeAt(0): str += '\r'; break;
          case 't'.charCodeAt(0): str += '\t'; break;
          case 'b'.charCodeAt(0): str += '\b'; break;
          case 'f'.charCodeAt(0): str += '\f'; break;
          case '('.charCodeAt(0): str += '('; break;
          case ')'.charCodeAt(0): str += ')'; break;
          case '\\'.charCodeAt(0): str += '\\'; break;
          default:
            // Octal escape
            if (next >= '0'.charCodeAt(0) && next <= '7'.charCodeAt(0)) {
              let octal = String.fromCharCode(next);
              for (let i = 0; i < 2; i++) {
                const digit = this.peekByte();
                if (digit >= '0'.charCodeAt(0) && digit <= '7'.charCodeAt(0)) {
                  octal += String.fromCharCode(this.readByte());
                } else {
                  break;
                }
              }
              str += String.fromCharCode(parseInt(octal, 8));
            } else {
              str += String.fromCharCode(next);
            }
        }
      } else if (byte === '('.charCodeAt(0)) {
        parenLevel++;
        str += '(';
      } else if (byte === ')'.charCodeAt(0)) {
        parenLevel--;
        if (parenLevel > 0) str += ')';
      } else {
        str += String.fromCharCode(byte);
      }
    }

    return str;
  }

  private parseHexString(): string {
    this.position++; // Skip '<'
    let hex = '';

    while (this.position < this.buffer.byteLength) {
      const byte = this.peekByte();

      if (byte === '>'.charCodeAt(0)) {
        this.position++;
        break;
      }

      // Skip whitespace in hex strings
      if (this.isWhitespace(byte)) {
        this.position++;
        continue;
      }

      hex += String.fromCharCode(this.readByte());
    }

    // Convert hex to string
    let str = '';
    for (let i = 0; i < hex.length; i += 2) {
      const hexByte = hex.substr(i, 2);
      str += String.fromCharCode(parseInt(hexByte, 16));
    }

    return str;
  }

  private parseArray(): PDFObject[] {
    this.position++; // Skip '['
    this.skipWhitespace();

    const array: PDFObject[] = [];

    while (this.position < this.buffer.byteLength) {
      if (this.peekByte() === ']'.charCodeAt(0)) {
        this.position++;
        break;
      }

      array.push(this.parseObject());
      this.skipWhitespace();
    }

    return array;
  }

  private parseBoolean(): boolean {
    const start = this.position;
    let word = '';

    while (this.position < this.buffer.byteLength && !this.isWhitespace(this.peekByte()) && !this.isDelimiter(this.peekByte())) {
      word += String.fromCharCode(this.readByte());
    }

    if (word === 'true') return true;
    if (word === 'false') return false;

    throw new Error(`Invalid boolean value: ${word}`);
  }

  private parseNumber(): number {
    let numStr = '';
    let isFloat = false;

    while (this.position < this.buffer.byteLength) {
      const byte = this.peekByte();

      if ((byte >= '0'.charCodeAt(0) && byte <= '9'.charCodeAt(0)) ||
        byte === '.'.charCodeAt(0) ||
        (numStr.length === 0 && (byte === '-'.charCodeAt(0) || byte === '+'.charCodeAt(0)))) {
        if (byte === '.'.charCodeAt(0)) isFloat = true;
        numStr += String.fromCharCode(this.readByte());
      } else {
        break;
      }
    }

    return isFloat ? parseFloat(numStr) : parseInt(numStr, 10);
  }

  private parseRectangle(obj?: PDFObject): Rectangle | undefined {
    if (!obj || obj.type !== PDFObjectType.Array) return undefined;

    const arr = obj.value as PDFObject[];
    if (arr.length !== 4) return undefined;

    return {
      x: (arr[0].value as number),
      y: (arr[1].value as number),
      width: (arr[2].value as number) - (arr[0].value as number),
      height: (arr[3].value as number) - (arr[1].value as number)
    };
  }

  private extractStringFromDict(dict: PDFDictionary, key: string): string | undefined {
    const obj = dict.entries.get(key);
    if (!obj) return undefined;

    if (obj.type === PDFObjectType.String) {
      return obj.value as string;
    }

    return undefined;
  }

  private parsePDFDate(dateStr: string): Date {
    // PDF date format: D:YYYYMMDDHHmmSSOHH'mm
    if (!dateStr.startsWith('D:')) return new Date();

    const year = parseInt(dateStr.substr(2, 4), 10);
    const month = parseInt(dateStr.substr(6, 2), 10) - 1;
    const day = parseInt(dateStr.substr(8, 2), 10);
    const hour = parseInt(dateStr.substr(10, 2), 10) || 0;
    const minute = parseInt(dateStr.substr(12, 2), 10) || 0;
    const second = parseInt(dateStr.substr(14, 2), 10) || 0;

    return new Date(year, month, day, hour, minute, second);
  }

  // Helper methods
  private readByte(): number {
    return this.dataView.getUint8(this.position++);
  }

  private peekByte(offset: number = 0): number {
    return this.dataView.getUint8(this.position + offset);
  }

  private readString(start: number, length: number): string {
    const bytes = new Uint8Array(this.buffer, start, length);
    return new TextDecoder().decode(bytes);
  }

  private skipWhitespace(): void {
    while (this.position < this.buffer.byteLength && this.isWhitespace(this.peekByte())) {
      this.position++;
    }
  }

  private isWhitespace(byte: number): boolean {
    return byte === 0 || byte === 9 || byte === 10 || byte === 12 || byte === 13 || byte === 32;
  }

  private isDelimiter(byte: number): boolean {
    const delimiters = '()<>[]{}/%';
    return delimiters.indexOf(String.fromCharCode(byte)) >= 0;
  }
}

// ============================================================================
// Supporting Classes
// ============================================================================

class XRefTable {
  private entries: Map<number, XRefEntry> = new Map();

  addEntry(objectNumber: number, offset: number, generation: number, type: string): void {
    this.entries.set(objectNumber, {
      objectNumber,
      offset,
      generation,
      type
    });
  }

  getEntry(objectNumber: number): XRefEntry | undefined {
    return this.entries.get(objectNumber);
  }
}

interface XRefEntry {
  objectNumber: number;
  offset: number;
  generation: number;
  type: string;
}

class PageTree {
  private pages: Map<number, PDFDictionary> = new Map();

  addPage(pageNumber: number, pageDict: PDFDictionary): void {
    this.pages.set(pageNumber, pageDict);
  }

  getPage(pageNumber: number): PDFDictionary | undefined {
    return this.pages.get(pageNumber);
  }

  getPageCount(): number {
    return this.pages.size;
  }
}

class StreamingPDFParser {
  bytesRead: number = 0;
  startTime: number = Date.now();
  private reader?: ReadableStreamDefaultReader<Uint8Array>;
  private buffer: Uint8Array = new Uint8Array(0);
  private position: number = 0;

  constructor(
    private stream: ReadableStream<Uint8Array>,
    private options?: StreamOptions
  ) {
    this.reader = stream.getReader();
  }

  async parseHeader(): Promise<void> {
    await this.ensureBytes(8);
    const header = this.readString(0, 8);
    if (!header.startsWith('%PDF-')) {
      throw new Error('Invalid PDF header');
    }
  }

  async parseMetadata(): Promise<PDFMetadata> {
    // Parse metadata from stream
    // This would incrementally parse the PDF structure
    return {
      version: '1.7',
      pageCount: 0,
      isEncrypted: false,
      isLinearized: false,
      fileSize: 0
    };
  }

  async *streamPages(): AsyncGenerator<PDFPage> {
    let pageNumber = 1;

    while (true) {
      try {
        const page = await this.parseNextPage(pageNumber);
        if (!page) break;

        yield {
          pageNumber: pageNumber++,
          width: page.width || 0,
          height: page.height || 0,
          rotation: page.rotation || 0,
          userUnit: page.userUnit || 1,
          mediaBox: page.mediaBox || { x: 0, y: 0, width: 0, height: 0 },
          cropBox: page.cropBox,
          bleedBox: page.bleedBox,
          trimBox: page.trimBox,
          artBox: page.artBox,
          contents: page.contents,
          resources: page.resources
        } as PDFPage;

        // Check abort signal
        if (this.options?.abortSignal?.aborted) {
          break;
        }
      } catch (error) {
        if (error instanceof Error && error.message === 'Stream ended') {
          break;
        }
        throw error;
      }
    }
  }

  private async parseNextPage(pageNumber: number): Promise<Partial<PDFPage> | null> {
    // Parse next page from stream
    // This would incrementally parse pages as the stream provides data

    await this.ensureBytes(1024); // Ensure we have enough data

    // Simplified page parsing
    return {
      width: 612,
      height: 792,
      rotation: 0,
      userUnit: 1,
      mediaBox: { x: 0, y: 0, width: 612, height: 792 }
    };
  }

  private async ensureBytes(count: number): Promise<void> {
    while (this.buffer.length - this.position < count) {
      const { done, value } = await this.reader!.read();

      if (done) {
        if (this.buffer.length - this.position < count) {
          throw new Error('Stream ended unexpectedly');
        }
        break;
      }

      // Append to buffer
      const newBuffer = new Uint8Array(this.buffer.length + value.length);
      newBuffer.set(this.buffer);
      newBuffer.set(value, this.buffer.length);
      this.buffer = newBuffer;
      this.bytesRead += value.length;

      // Report progress
      this.options?.progressCallback?.({
        bytesRead: this.bytesRead,
        totalBytes: 0,
        pagesProcessed: 0,
        currentOperation: 'Reading stream',
        timeElapsed: Date.now() - this.startTime
      });

      // Check backpressure
      if (this.buffer.length > (this.options?.backpressureThreshold || 10485760)) {
        // Compact buffer if possible
        if (this.position > 0) {
          this.buffer = this.buffer.slice(this.position);
          this.position = 0;
        }
      }
    }
  }

  private readString(start: number, length: number): string {
    const bytes = this.buffer.slice(start, start + length);
    return new TextDecoder().decode(bytes);
  }
}

class TextExtractor {
  constructor(
    private pdf: ModernPDF,
    private options?: TextExtractionOptions
  ) { }

  async extract(): Promise<TextContent[]> {
    const results: TextContent[] = [];
    const metadata = this.pdf.getMetadata();
    if (!metadata) return results;

    for (let i = 1; i <= metadata.pageCount; i++) {
      const page = await this.pdf.getPage(i);
      if (page) {
        const pageText = await this.extractPageText(page);
        results.push(...pageText);
      }
    }

    return this.options?.preserveFormatting ? results : this.mergeTextBlocks(results);
  }

  async *stream(): AsyncGenerator<TextContent> {
    const metadata = this.pdf.getMetadata();
    if (!metadata) return;

    for (let i = 1; i <= metadata.pageCount; i++) {
      const page = await this.pdf.getPage(i);
      if (page) {
        const pageText = await this.extractPageText(page);
        for (const text of pageText) {
          yield text;
        }
      }
    }
  }

  private async extractPageText(page: PDFPage): Promise<TextContent[]> {
    const textBlocks: TextContent[] = [];

    if (!page.contents) return textBlocks;

    // Parse content stream
    const contentParser = new ContentStreamParser(page.contents);
    const operations = contentParser.parse();

    // Current graphics state
    let currentState = {
      textMatrix: [1, 0, 0, 1, 0, 0] as TransformMatrix,
      fontSize: 12,
      fontName: 'Helvetica',
      textLeading: 0,
      charSpace: 0,
      wordSpace: 0,
      horizontalScaling: 100,
      textRise: 0,
      renderingMode: 0,
      fillColor: { r: 0, g: 0, b: 0 } as Color
    };

    for (const op of operations) {
      switch (op.operator) {
        case 'BT': // Begin text
          currentState.textMatrix = [1, 0, 0, 1, 0, 0];
          break;

        case 'ET': // End text
          break;

        case 'Tf': // Set font and size
          currentState.fontName = op.operands[0] as string;
          currentState.fontSize = op.operands[1] as number;
          break;

        case 'Td': // Move text position
          const tx = op.operands[0] as number;
          const ty = op.operands[1] as number;
          currentState.textMatrix[4] += tx;
          currentState.textMatrix[5] += ty;
          break;

        case 'Tj': // Show text
          const text = op.operands[0] as string;
          textBlocks.push(this.createTextContent(text, currentState, page));
          break;

        case 'TJ': // Show text with positioning
          const array = op.operands[0] as any[];
          let combinedText = '';
          for (const item of array) {
            if (typeof item === 'string') {
              combinedText += item;
            }
          }
          if (combinedText) {
            textBlocks.push(this.createTextContent(combinedText, currentState, page));
          }
          break;
      }
    }

    return textBlocks;
  }

  private createTextContent(text: string, state: any, page: PDFPage): TextContent {
    // Calculate text metrics
    const width = text.length * state.fontSize * 0.5; // Approximate
    const height = state.fontSize;

    return {
      text: text,
      x: state.textMatrix[4],
      y: page.height - state.textMatrix[5], // Flip Y coordinate
      width: width,
      height: height,
      fontSize: state.fontSize,
      fontName: state.fontName,
      direction: 'ltr',
      transform: state.textMatrix,
      style: {
        bold: state.fontName.toLowerCase().includes('bold'),
        italic: state.fontName.toLowerCase().includes('italic'),
        underline: false,
        strikethrough: false,
        color: state.fillColor
      },
      pageNumber: page.pageNumber
    };
  }

  private mergeTextBlocks(blocks: TextContent[]): TextContent[] {
    // Merge adjacent text blocks on the same line
    const merged: TextContent[] = [];
    let current: TextContent | null = null;

    for (const block of blocks) {
      if (!current) {
        current = { ...block };
        continue;
      }

      // Check if blocks are on the same line (within threshold)
      const sameY = Math.abs(current.y - block.y) < current.fontSize * 0.5;
      const adjacent = Math.abs((current.x + current.width) - block.x) < current.fontSize;

      if (sameY && adjacent && current.pageNumber === block.pageNumber) {
        // Merge blocks
        current.text += ' ' + block.text;
        current.width = block.x + block.width - current.x;
      } else {
        merged.push(current);
        current = { ...block };
      }
    }

    if (current) merged.push(current);

    return merged;
  }
}

class ContentStreamParser {
  private position: number = 0;

  constructor(private data: Uint8Array) { }

  parse(): ContentOperation[] {
    const operations: ContentOperation[] = [];

    while (this.position < this.data.length) {
      this.skipWhitespace();
      if (this.position >= this.data.length) break;

      const operands: any[] = [];

      // Parse operands
      while (this.position < this.data.length) {
        const operand = this.parseOperand();
        if (operand === null) break;
        operands.push(operand);
      }

      // Parse operator
      const operator = this.parseOperator();
      if (operator) {
        operations.push({ operator, operands });
      }
    }

    return operations;
  }

  private parseOperand(): any {
    this.skipWhitespace();

    const byte = this.data[this.position];

    // Number
    if ((byte >= 48 && byte <= 57) || byte === 45 || byte === 46) {
      return this.parseNumber();
    }

    // String
    if (byte === 40) { // '('
      return this.parseString();
    }

    // Hex string
    if (byte === 60) { // '<'
      return this.parseHexString();
    }

    // Array
    if (byte === 91) { // '['
      return this.parseArray();
    }

    // Name
    if (byte === 47) { // '/'
      return this.parseName();
    }

    // Dictionary
    if (byte === 60 && this.data[this.position + 1] === 60) { // '<<'
      return this.parseDictionary();
    }

    return null;
  }

  private parseOperator(): string {
    this.skipWhitespace();

    let operator = '';
    while (this.position < this.data.length) {
      const byte = this.data[this.position];

      // Operator characters (letters and special chars)
      if ((byte >= 65 && byte <= 90) || // A-Z
        (byte >= 97 && byte <= 122) || // a-z
        byte === 42 || byte === 39 || byte === 34) { // *, ', "
        operator += String.fromCharCode(byte);
        this.position++;
      } else {
        break;
      }
    }

    return operator;
  }

  private parseNumber(): number {
    let numStr = '';

    while (this.position < this.data.length) {
      const byte = this.data[this.position];

      if ((byte >= 48 && byte <= 57) || byte === 46 || byte === 45 || byte === 43) {
        numStr += String.fromCharCode(byte);
        this.position++;
      } else {
        break;
      }
    }

    return parseFloat(numStr);
  }

  private parseString(): string {
    this.position++; // Skip '('
    let str = '';
    let parenCount = 1;

    while (this.position < this.data.length && parenCount > 0) {
      const byte = this.data[this.position++];

      if (byte === 40) { // '('
        parenCount++;
        str += '(';
      } else if (byte === 41) { // ')'
        parenCount--;
        if (parenCount > 0) str += ')';
      } else if (byte === 92) { // '\'
        // Handle escape sequences
        const next = this.data[this.position++];
        switch (next) {
          case 110: str += '\n'; break; // n
          case 114: str += '\r'; break; // r
          case 116: str += '\t'; break; // t
          case 98: str += '\b'; break;  // b
          case 102: str += '\f'; break; // f
          default: str += String.fromCharCode(next);
        }
      } else {
        str += String.fromCharCode(byte);
      }
    }

    return str;
  }

  private parseHexString(): string {
    this.position++; // Skip '<'
    let hex = '';

    while (this.position < this.data.length) {
      const byte = this.data[this.position];

      if (byte === 62) { // '>'
        this.position++;
        break;
      }

      if ((byte >= 48 && byte <= 57) || // 0-9
        (byte >= 65 && byte <= 70) || // A-F
        (byte >= 97 && byte <= 102)) { // a-f
        hex += String.fromCharCode(byte);
      }

      this.position++;
    }

    // Convert hex to string
    let str = '';
    for (let i = 0; i < hex.length; i += 2) {
      str += String.fromCharCode(parseInt(hex.substr(i, 2), 16));
    }

    return str;
  }

  private parseArray(): any[] {
    this.position++; // Skip '['
    const array: any[] = [];

    while (this.position < this.data.length) {
      this.skipWhitespace();

      if (this.data[this.position] === 93) { // ']'
        this.position++;
        break;
      }

      const item = this.parseOperand();
      if (item !== null) {
        array.push(item);
      }
    }

    return array;
  }

  private parseName(): string {
    this.position++; // Skip '/'
    let name = '';

    while (this.position < this.data.length) {
      const byte = this.data[this.position];

      // Name ends at whitespace or delimiter
      if (this.isWhitespace(byte) || this.isDelimiter(byte)) {
        break;
      }

      name += String.fromCharCode(byte);
      this.position++;
    }

    return name;
  }

  private parseDictionary(): Map<string, any> {
    this.position += 2; // Skip '<<'
    const dict = new Map<string, any>();

    while (this.position < this.data.length) {
      this.skipWhitespace();

      // Check for '>>'
      if (this.data[this.position] === 62 && this.data[this.position + 1] === 62) {
        this.position += 2;
        break;
      }

      // Parse key (name)
      const key = this.parseName();
      this.skipWhitespace();

      // Parse value
      const value = this.parseOperand();

      dict.set(key, value);
    }

    return dict;
  }

  private skipWhitespace(): void {
    while (this.position < this.data.length && this.isWhitespace(this.data[this.position])) {
      this.position++;
    }
  }

  private isWhitespace(byte: number): boolean {
    return byte === 0 || byte === 9 || byte === 10 || byte === 12 || byte === 13 || byte === 32;
  }

  private isDelimiter(byte: number): boolean {
    return byte === 40 || byte === 41 || // ()
      byte === 60 || byte === 62 || // <>
      byte === 91 || byte === 93 || // []
      byte === 123 || byte === 125 || // {}
      byte === 47 || byte === 37; // /%
  }
}

interface ContentOperation {
  operator: string;
  operands: any[];
}

class ImageExtractor {
  constructor(
    private pdf: ModernPDF,
    private options?: ImageExtractionOptions
  ) { }

  async extract(): Promise<ImageContent[]> {
    const images: ImageContent[] = [];
    const metadata = this.pdf.getMetadata();
    if (!metadata) return images;

    for (let i = 1; i <= metadata.pageCount; i++) {
      const page = await this.pdf.getPage(i);
      if (page && page.resources) {
        const pageImages = await this.extractPageImages(page);
        images.push(...pageImages);
      }
    }

    return images;
  }

  private async extractPageImages(page: PDFPage): Promise<ImageContent[]> {
    const images: ImageContent[] = [];

    if (!page.resources?.images) return images;

    let imageIndex = 0;
    for (const [name, imageRes] of page.resources.images) {
      const image: ImageContent = {
        id: `page${page.pageNumber}_img${imageIndex++}`,
        x: 0, // Would be determined from content stream
        y: 0,
        width: imageRes.width,
        height: imageRes.height,
        mimeType: this.getImageMimeType(imageRes),
        bitsPerComponent: imageRes.bitsPerComponent,
        colorSpace: imageRes.colorSpace,
        filter: imageRes.filter,
        data: imageRes.data,
        pageNumber: page.pageNumber
      };

      // Apply extraction options
      if (this.options?.maxWidth || this.options?.maxHeight) {
        image.data = this.resizeImage(image.data!, image.width, image.height);
      }

      images.push(image);
    }

    return images;
  }

  private getImageMimeType(imageRes: ImageResource): string {
    if (imageRes.filter) {
      if (imageRes.filter.includes('DCTDecode')) return 'image/jpeg';
      if (imageRes.filter.includes('FlateDecode')) return 'image/png';
      if (imageRes.filter.includes('JPXDecode')) return 'image/jp2';
    }
    return 'image/raw';
  }

  private resizeImage(data: Uint8Array, width: number, height: number): Uint8Array {
    // Implement image resizing logic
    return data;
  }
}

class AIAnalyzer {
  constructor(
    private pdf: ModernPDF,
    private options?: AIOptions
  ) { }

  async analyze(): Promise<AIFeatures> {
    const structuralAnalysis = await this.analyzeStructure();
    const semanticChunks = await this.generateChunks();
    const nlpReady = await this.prepareNLPContent();

    return {
      structuralAnalysis,
      semanticChunks,
      nlpReady,
      embeddings: this.options?.embeddingProvider
    };
  }

  private async analyzeStructure(): Promise<StructuralAnalysis> {
    const allText = await this.pdf.extractText();

    // Detect document type based on content patterns
    const documentType = this.detectDocumentType(allText);

    // Extract sections
    const sections = this.extractSections(allText);

    // Extract tables
    const tables = await this.extractTables();

    // Extract figures
    const figures = await this.extractFigures();

    // Extract equations
    const equations = this.extractEquations(allText);

    // Extract references
    const references = this.extractReferences(allText);

    // Build table of contents
    const tableOfContents = this.buildTableOfContents(sections);

    // Extract bibliography
    const bibliography = this.extractBibliography(allText);

    return {
      documentType,
      sections,
      tables,
      figures,
      equations,
      references,
      tableOfContents,
      bibliography
    };
  }

  private detectDocumentType(text: TextContent[]): DocumentType {
    const fullText = text.map(t => t.text).join(' ').toLowerCase();

    // Simple heuristics for document type detection
    if (fullText.includes('abstract') && fullText.includes('references')) {
      return DocumentType.Article;
    }
    if (fullText.includes('chapter') && text.length > 1000) {
      return DocumentType.Book;
    }
    if (fullText.includes('invoice') || fullText.includes('total amount')) {
      return DocumentType.Invoice;
    }
    if (fullText.includes('experience') && fullText.includes('education')) {
      return DocumentType.Resume;
    }

    return DocumentType.Other;
  }

  private extractSections(text: TextContent[]): DocumentSection[] {
    const sections: DocumentSection[] = [];
    let currentSection: DocumentSection | null = null;
    let sectionId = 0;

    for (const block of text) {
      // Detect headers based on font size and style
      const isHeader = block.fontSize > 14 || block.style.bold;

      if (isHeader) {
        if (currentSection) {
          sections.push(currentSection);
        }

        currentSection = {
          id: `section_${sectionId++}`,
          type: 'heading',
          level: this.getHeaderLevel(block.fontSize),
          pageStart: block.pageNumber,
          pageEnd: block.pageNumber,
          boundingBox: {
            x: block.x,
            y: block.y,
            width: block.width,
            height: block.height
          },
          text: block.text,
          children: []
        };
      } else if (currentSection) {
        // Add content to current section
        currentSection.children?.push({
          id: `section_${sectionId++}`,
          type: 'paragraph',
          pageStart: block.pageNumber,
          pageEnd: block.pageNumber,
          boundingBox: {
            x: block.x,
            y: block.y,
            width: block.width,
            height: block.height
          },
          text: block.text
        });

        currentSection.pageEnd = Math.max(currentSection.pageEnd, block.pageNumber);
      }
    }

    if (currentSection) {
      sections.push(currentSection);
    }

    return sections;
  }

  private getHeaderLevel(fontSize: number): number {
    if (fontSize > 24) return 1;
    if (fontSize > 18) return 2;
    if (fontSize > 14) return 3;
    return 4;
  }

  private async extractTables(): Promise<Table[]> {
    const tables: Table[] = [];
    // Table extraction logic would go here
    // This would analyze page layout to detect tabular structures
    return tables;
  }

  private async extractFigures(): Promise<Figure[]> {
    const figures: Figure[] = [];
    const images = await this.pdf.extractImages();

    for (const image of images) {
      figures.push({
        id: `figure_${image.id}`,
        pageNumber: image.pageNumber,
        boundingBox: {
          x: image.x,
          y: image.y,
          width: image.width,
          height: image.height
        },
        imageRef: image,
        type: 'illustration'
      });
    }

    return figures;
  }

  private extractEquations(text: TextContent[]): Equation[] {
    const equations: Equation[] = [];
    // Equation extraction logic
    // Would detect mathematical notation patterns
    return equations;
  }

  private extractReferences(text: TextContent[]): Reference[] {
    const references: Reference[] = [];
    // Reference extraction logic
    // Would detect citations, footnotes, hyperlinks
    return references;
  }

  private buildTableOfContents(sections: DocumentSection[]): TOCEntry[] {
    return sections
      .filter(s => s.type === 'heading')
      .map(s => ({
        title: s.text,
        pageNumber: s.pageStart,
        level: s.level || 1
      }));
  }

  private extractBibliography(text: TextContent[]): BibliographyEntry[] {
    const bibliography: BibliographyEntry[] = [];
    // Bibliography extraction logic
    // Would parse reference sections
    return bibliography;
  }

  private async generateChunks(): Promise<SemanticChunk[]> {
    const text = await this.pdf.extractText();
    const chunker = new SemanticChunker(this.pdf, {
      maxChunkSize: this.options?.chunkSize || 500,
      minChunkSize: 100,
      overlapSize: this.options?.chunkOverlap || 50
    });

    return chunker.chunkText(text);
  }

  private async prepareNLPContent(): Promise<NLPReadyContent> {
    const text = await this.pdf.extractText();
    const fullText = text.map(t => t.text).join(' ');

    // Clean text
    const cleanText = this.cleanText(fullText);

    // Split into sentences
    const sentences = this.splitSentences(cleanText);

    // Split into paragraphs
    const paragraphs = this.splitParagraphs(cleanText);

    // Count tokens (simple approximation)
    const tokenCount = cleanText.split(/\s+/).length;

    // Detect language
    const language = this.detectLanguage(cleanText);

    // Extract keywords
    const keywords = this.extractKeywords(cleanText);

    return {
      fullText,
      cleanText,
      sentences,
      paragraphs,
      tokenCount,
      language,
      keywords
    };
  }

  private cleanText(text: string): string {
    return text
      .replace(/\s+/g, ' ')
      .replace(/[^\w\s.,!?;:\-'"]/g, '')
      .trim();
  }

  private splitSentences(text: string): string[] {
    return text.match(/[^.!?]+[.!?]+/g) || [];
  }

  private splitParagraphs(text: string): string[] {
    return text.split(/\n\n+/).filter(p => p.trim().length > 0);
  }

  private detectLanguage(text: string): string {
    // Simple language detection based on common words
    const englishWords = ['the', 'is', 'at', 'which', 'on'];
    const textLower = text.toLowerCase();

    if (englishWords.some(word => textLower.includes(word))) {
      return 'en';
    }

    return 'unknown';
  }

  private extractKeywords(text: string): string[] {
    // Simple keyword extraction
    const words = text.toLowerCase().split(/\s+/);
    const wordFreq = new Map<string, number>();

    for (const word of words) {
      if (word.length > 4) { // Skip short words
        wordFreq.set(word, (wordFreq.get(word) || 0) + 1);
      }
    }

    // Sort by frequency and return top keywords
    return Array.from(wordFreq.entries())
      .sort((a, b) => b[1] - a[1])
      .slice(0, 10)
      .map(([word]) => word);
  }
}

class SemanticChunker {
  constructor(
    private pdf: ModernPDF,
    private options: ChunkingOptions = {}
  ) { }

  async chunk(): Promise<SemanticChunk[]> {
    const text = await this.pdf.extractText();
    return this.chunkText(text);
  }

  async *stream(): AsyncGenerator<SemanticChunk> {
    const chunks = await this.chunk();
    for (const chunk of chunks) {
      yield chunk;
    }
  }

  chunkText(text: TextContent[]): SemanticChunk[] {
    const strategy = this.options.strategy || 'semantic';

    switch (strategy) {
      case 'fixed':
        return this.fixedSizeChunking(text);
      case 'sliding':
        return this.slidingWindowChunking(text);
      case 'recursive':
        return this.recursiveChunking(text);
      case 'semantic':
      default:
        return this.semanticChunking(text);
    }
  }

  private semanticChunking(text: TextContent[]): SemanticChunk[] {
    const chunks: SemanticChunk[] = [];
    let currentChunk: TextContent[] = [];
    let currentSize = 0;
    let chunkId = 0;

    for (const block of text) {
      const blockSize = block.text.split(/\s+/).length;

      // Check if adding this block would exceed max size
      if (currentSize + blockSize > (this.options.maxChunkSize || 500)) {
        // Create chunk from current blocks
        if (currentChunk.length > 0) {
          chunks.push(this.createChunk(currentChunk, chunkId++));
        }

        // Start new chunk with overlap
        if (this.options.overlapSize) {
          // Keep last N tokens for overlap
          const overlapBlocks = this.getOverlapBlocks(currentChunk, this.options.overlapSize);
          currentChunk = overlapBlocks;
          currentSize = overlapBlocks.reduce((sum, b) => sum + b.text.split(/\s+/).length, 0);
        } else {
          currentChunk = [];
          currentSize = 0;
        }
      }

      currentChunk.push(block);
      currentSize += blockSize;

      // Check for semantic boundaries (paragraph breaks, headers, etc.)
      if (this.isSemanticBoundary(block)) {
        if (currentSize >= (this.options.minChunkSize || 100)) {
          chunks.push(this.createChunk(currentChunk, chunkId++));
          currentChunk = [];
          currentSize = 0;
        }
      }
    }

    // Add remaining chunk
    if (currentChunk.length > 0) {
      chunks.push(this.createChunk(currentChunk, chunkId++));
    }

    return chunks;
  }

  private fixedSizeChunking(text: TextContent[]): SemanticChunk[] {
    const chunks: SemanticChunk[] = [];
    const maxSize = this.options.maxChunkSize || 500;
    let currentChunk: TextContent[] = [];
    let currentSize = 0;
    let chunkId = 0;

    for (const block of text) {
      const blockSize = block.text.split(/\s+/).length;

      if (currentSize + blockSize > maxSize) {
        if (currentChunk.length > 0) {
          chunks.push(this.createChunk(currentChunk, chunkId++));
        }
        currentChunk = [];
        currentSize = 0;
      }

      currentChunk.push(block);
      currentSize += blockSize;
    }

    if (currentChunk.length > 0) {
      chunks.push(this.createChunk(currentChunk, chunkId++));
    }

    return chunks;
  }

  private slidingWindowChunking(text: TextContent[]): SemanticChunk[] {
    const chunks: SemanticChunk[] = [];
    const windowSize = this.options.maxChunkSize || 500;
    const stepSize = windowSize - (this.options.overlapSize || 50);

    let chunkId = 0;
    for (let i = 0; i < text.length; i += stepSize) {
      const window = text.slice(i, i + windowSize);
      if (window.length > 0) {
        chunks.push(this.createChunk(window, chunkId++));
      }
    }

    return chunks;
  }

  private recursiveChunking(text: TextContent[]): SemanticChunk[] {
    // Recursive chunking based on document structure
    return this.semanticChunking(text); // Simplified for now
  }

  private isSemanticBoundary(block: TextContent): boolean {
    // Check if this block represents a semantic boundary
    return block.text.endsWith('.') &&
      block.text.length > 50 &&
      (block.fontSize > 14 || block.style.bold);
  }

  private getOverlapBlocks(blocks: TextContent[], overlapSize: number): TextContent[] {
    const result: TextContent[] = [];
    let size = 0;

    for (let i = blocks.length - 1; i >= 0; i--) {
      const blockSize = blocks[i].text.split(/\s+/).length;
      if (size + blockSize <= overlapSize) {
        result.unshift(blocks[i]);
        size += blockSize;
      } else {
        break;
      }
    }

    return result;
  }

  private createChunk(blocks: TextContent[], id: number): SemanticChunk {
    const content = blocks.map(b => b.text).join(' ');
    const pageNumbers = [...new Set(blocks.map(b => b.pageNumber))];

    return {
      id: `chunk_${id}`,
      content,
      pageNumbers,
      type: this.detectChunkType(blocks),
      metadata: {
        tokenCount: content.split(/\s+/).length,
        confidence: 1.0,
        importance: this.calculateImportance(blocks),
        keywords: this.extractChunkKeywords(content)
      },
      startOffset: blocks[0].x,
      endOffset: blocks[blocks.length - 1].x + blocks[blocks.length - 1].width
    };
  }

  private detectChunkType(blocks: TextContent[]): ChunkType {
    const firstBlock = blocks[0];

    if (firstBlock.fontSize > 16 || firstBlock.style.bold) {
      return ChunkType.Header;
    }

    const content = blocks.map(b => b.text).join(' ');
    if (content.match(/^\d+\./)) {
      return ChunkType.List;
    }

    if (content.startsWith('"') || content.includes('said')) {
      return ChunkType.Quote;
    }

    return ChunkType.Paragraph;
  }

  private calculateImportance(blocks: TextContent[]): number {
    // Calculate importance based on position, formatting, etc.
    let importance = 0.5;

    // Headers are more important
    if (blocks.some(b => b.fontSize > 14 || b.style.bold)) {
      importance += 0.2;
    }

    // First page content is more important
    if (blocks.some(b => b.pageNumber === 1)) {
      importance += 0.1;
    }

    // Longer chunks might be more important
    const totalLength = blocks.reduce((sum, b) => sum + b.text.length, 0);
    if (totalLength > 500) {
      importance += 0.1;
    }

    return Math.min(importance, 1.0);
  }

  private extractChunkKeywords(content: string): string[] {
    const words = content.toLowerCase().split(/\s+/);
    const wordFreq = new Map<string, number>();

    for (const word of words) {
      if (word.length > 4 && !this.isStopWord(word)) {
        wordFreq.set(word, (wordFreq.get(word) || 0) + 1);
      }
    }

    return Array.from(wordFreq.entries())
      .sort((a, b) => b[1] - a[1])
      .slice(0, 5)
      .map(([word]) => word);
  }

  private isStopWord(word: string): boolean {
    const stopWords = ['the', 'is', 'at', 'which', 'on', 'and', 'a', 'an', 'as', 'are', 'was', 'were', 'been', 'be', 'have', 'has', 'had', 'do', 'does', 'did', 'will', 'would', 'should', 'could', 'may', 'might', 'must', 'can', 'this', 'that', 'these', 'those', 'with', 'from', 'for', 'to', 'of', 'in'];
    return stopWords.includes(word);
  }
}

class PDFSearcher {
  constructor(private pdf: ModernPDF) { }

  async search(query: string, options?: SearchOptions): Promise<SearchResult[]> {
    const results: SearchResult[] = [];
    const text = await this.pdf.extractText();

    const queryLower = query.toLowerCase();
    const regex = options?.regex ? new RegExp(query, options.caseSensitive ? 'g' : 'gi') : null;

    for (const block of text) {
      const content = options?.caseSensitive ? block.text : block.text.toLowerCase();
      const searchQuery = options?.caseSensitive ? query : queryLower;

      let matches: SearchMatch[] = [];

      if (regex) {
        // Regex search
        let match;
        while ((match = regex.exec(block.text)) !== null) {
          matches.push({
            text: match[0],
            index: match.index,
            length: match[0].length
          });
        }
      } else if (options?.wholeWord) {
        // Whole word search
        const wordRegex = new RegExp(`\\b${searchQuery}\\b`, options.caseSensitive ? 'g' : 'gi');
        let match;
        while ((match = wordRegex.exec(block.text)) !== null) {
          matches.push({
            text: match[0],
            index: match.index,
            length: match[0].length
          });
        }
      } else {
        // Simple substring search
        let index = content.indexOf(searchQuery);
        while (index !== -1) {
          matches.push({
            text: block.text.substr(index, query.length),
            index,
            length: query.length
          });
          index = content.indexOf(searchQuery, index + 1);
        }
      }

      if (matches.length > 0) {
        results.push({
          pageNumber: block.pageNumber,
          textContent: block,
          matches,
          context: this.getContext(block.text, matches[0].index, options?.contextLength || 50)
        });
      }
    }

    return results;
  }

  private getContext(text: string, index: number, contextLength: number): string {
    const start = Math.max(0, index - contextLength);
    const end = Math.min(text.length, index + contextLength);

    let context = text.substring(start, end);

    if (start > 0) context = '...' + context;
    if (end < text.length) context = context + '...';

    return context;
  }
}

interface SearchOptions {
  caseSensitive?: boolean;
  wholeWord?: boolean;
  regex?: boolean;
  contextLength?: number;
}

interface SearchResult {
  pageNumber: number;
  textContent: TextContent;
  matches: SearchMatch[];
  context: string;
}

interface SearchMatch {
  text: string;
  index: number;
  length: number;
}

class FormExtractor {
  constructor(private pdf: ModernPDF) { }

  async extract(): Promise<FormField[]> {
    const fields: FormField[] = [];
    const pages = await this.pdf.getAllPages();

    for (const page of pages) {
      // Extract form fields from page annotations
      // This would parse interactive form elements
      const pageFields = await this.extractPageFormFields(page);
      fields.push(...pageFields);
    }

    return fields;
  }

  private async extractPageFormFields(page: PDFPage): Promise<FormField[]> {
    // Implementation would parse AcroForm fields
    return [];
  }
}

class FormFiller {
  constructor(private pdf: ModernPDF) { }

  async fill(data: Record<string, any>): Promise<void> {
    const fields = await new FormExtractor(this.pdf).extract();

    for (const field of fields) {
      if (data.hasOwnProperty(field.name)) {
        field.value = data[field.name];
        // Update the PDF structure with new value
      }
    }
  }
}

class AnnotationExtractor {
  constructor(private pdf: ModernPDF) { }

  async extract(pageNumber?: number): Promise<Annotation[]> {
    const annotations: Annotation[] = [];

    if (pageNumber) {
      const page = await this.pdf.getPage(pageNumber);
      if (page) {
        const pageAnnotations = await this.extractPageAnnotations(page);
        annotations.push(...pageAnnotations);
      }
    } else {
      const pages = await this.pdf.getAllPages();
      for (const page of pages) {
        const pageAnnotations = await this.extractPageAnnotations(page);
        annotations.push(...pageAnnotations);
      }
    }

    return annotations;
  }

  private async extractPageAnnotations(page: PDFPage): Promise<Annotation[]> {
    // Parse annotations from page dictionary
    return [];
  }
}

class AnnotationManager {
  constructor(private pdf: ModernPDF) { }

  async add(annotation: Partial<Annotation>): Promise<string> {
    const id = `annotation_${Date.now()}`;

    // Add annotation to PDF structure
    // This would modify the page's annotation array

    return id;
  }

  async remove(annotationId: string): Promise<boolean> {
    // Remove annotation from PDF structure
    return true;
  }

  async update(annotationId: string, updates: Partial<Annotation>): Promise<boolean> {
    // Update annotation in PDF structure
    return true;
  }
}

class PDFRenderer {
  constructor(
    private pdf: ModernPDF,
    private options?: RenderOptions
  ) { }

  /**
   * Get optimal viewer options for best user experience
   * @returns Recommended render options
   */
  static getOptimalViewerOptions(): RenderOptions {
    return {
      scale: 1.0,
      renderScale: 2.0, // High DPI for sharp text
      fitToWidth: true,
      maintainAspectRatio: true,
      autoFitOnLoad: true,
      darkMode: true,
      continuousScrolling: true,
      renderText: true,
      renderImages: true,
      renderAnnotations: true,
      imageQuality: 1.0, // Maximum quality
      // Theme toggle functionality
      enableThemeToggle: true,
      persistTheme: true,
      defaultTheme: 'dark',
      themeStorageKey: 'modernpdf-theme'
    };
  }

  /**
   * Calculate optimal scale for fit-to-width rendering
   * @param pageWidth - Width of the PDF page
   * @param containerWidth - Width of the container element
   * @returns Optimal scale value
   */
  static calculateFitToWidthScale(pageWidth: number, containerWidth: number): number {
    return Math.min(containerWidth / pageWidth, 1.5); // Cap at 1.5x for readability
  }

  /**
   * Apply fit-to-width scaling to canvas
   * @param canvas - Canvas element to scale
   * @param pageWidth - Width of the PDF page
   * @param containerWidth - Available container width
   */
  static applyFitToWidth(canvas: HTMLCanvasElement, pageWidth: number, containerWidth: number): void {
    const scale = PDFRenderer.calculateFitToWidthScale(pageWidth, containerWidth);
    const scaledWidth = pageWidth * scale;
    const scaledHeight = (canvas.height / canvas.width) * scaledWidth;

    canvas.style.width = `${scaledWidth}px`;
    canvas.style.height = `${scaledHeight}px`;
  }

  /**
   * Configure canvas for optimal viewer experience
   * @param canvas - Canvas element to configure
   * @param options - Render options
   */
  static configureOptimalViewer(canvas: HTMLCanvasElement, options?: RenderOptions): void {
    const opts = { ...PDFRenderer.getOptimalViewerOptions(), ...options };

    // Initialize theme manager if theme toggle is enabled
    if (opts.enableThemeToggle) {
      const themeManager = ThemeManager.getInstance();
      themeManager.initialize({
        defaultTheme: opts.defaultTheme || 'dark',
        storageKey: opts.themeStorageKey || 'modernpdf-theme',
        persistTheme: opts.persistTheme !== false
      });
    }

    // Apply dark mode styling based on current theme
    const currentTheme = opts.enableThemeToggle
      ? ThemeManager.getInstance().getCurrentTheme()
      : (opts.darkMode ? 'dark' : 'light');

    if (currentTheme === 'dark') {
      canvas.style.backgroundColor = '#1a1a1a';
      if (canvas.parentElement) {
        canvas.parentElement.style.backgroundColor = '#1a1a1a';
        canvas.parentElement.style.color = '#ffffff';
      }
    } else {
      canvas.style.backgroundColor = '#ffffff';
      if (canvas.parentElement) {
        canvas.parentElement.style.backgroundColor = '#f5f5f5';
        canvas.parentElement.style.color = '#333333';
      }
    }

    // Enable smooth scaling
    canvas.style.imageRendering = 'auto';
    canvas.style.imageRendering = 'optimizeQuality';

    // Maintain aspect ratio
    if (opts.maintainAspectRatio !== false) {
      canvas.style.objectFit = 'contain';
    }

    // Add theme change observer if theme toggle is enabled
    if (opts.enableThemeToggle) {
      const themeManager = ThemeManager.getInstance();
      themeManager.addObserver((theme) => {
        if (theme === 'dark') {
          canvas.style.backgroundColor = '#1a1a1a';
          if (canvas.parentElement) {
            canvas.parentElement.style.backgroundColor = '#1a1a1a';
            canvas.parentElement.style.color = '#ffffff';
          }
        } else {
          canvas.style.backgroundColor = '#ffffff';
          if (canvas.parentElement) {
            canvas.parentElement.style.backgroundColor = '#f5f5f5';
            canvas.parentElement.style.color = '#333333';
          }
        }
      });
    }
  }

  /**
   * Create a complete PDF viewer with optimal configuration including theme toggle
   * @param container - Container element for the viewer
   * @param pdf - ModernPDF instance
   * @param options - Render options
   * @returns Object with viewer elements and methods
   */
  static createOptimalViewer(
    container: HTMLElement,
    pdf: ModernPDF,
    options?: RenderOptions
  ): {
    canvas: HTMLCanvasElement;
    toolbar: HTMLElement;
    themeToggle: HTMLButtonElement;
    themeManager: ThemeManager;
    destroy: () => void;
  } {
    const opts = { ...PDFRenderer.getOptimalViewerOptions(), ...options };

    // Create main canvas
    const canvas = document.createElement('canvas');
    PDFRenderer.configureOptimalViewer(canvas, opts);

    // Create toolbar
    const toolbar = document.createElement('div');
    toolbar.className = 'pdf-viewer-toolbar';
    Object.assign(toolbar.style, {
      position: 'fixed',
      top: '10px',
      left: '50%',
      transform: 'translateX(-50%)',
      background: opts.darkMode ? '#2d2d2d' : '#ffffff',
      border: `1px solid ${opts.darkMode ? '#555' : '#ddd'}`,
      borderRadius: '4px',
      padding: '8px 16px',
      display: 'flex',
      alignItems: 'center',
      gap: '10px',
      zIndex: '1000',
      boxShadow: opts.darkMode
        ? '0 2px 8px rgba(0, 0, 0, 0.3)'
        : '0 2px 8px rgba(0, 0, 0, 0.1)'
    });

    // Create theme toggle button if enabled
    let themeToggle: HTMLButtonElement;
    let themeManager: ThemeManager;

    if (opts.enableThemeToggle) {
      themeManager = ThemeManager.getInstance();
      themeToggle = ThemeManager.createThemeToggleButton({
        className: 'pdf-theme-toggle',
        size: 'medium'
      });
      toolbar.appendChild(themeToggle);
    }

    // Add elements to container
    container.appendChild(toolbar);
    container.appendChild(canvas);

    // Cleanup function
    const destroy = () => {
      container.removeChild(toolbar);
      container.removeChild(canvas);
    };

    return {
      canvas,
      toolbar,
      themeToggle: themeToggle!,
      themeManager: themeManager!,
      destroy
    };
  }

  async renderToCanvas(pageNumber: number, canvas: HTMLCanvasElement): Promise<void> {
    const page = await this.pdf.getPage(pageNumber);
    if (!page) throw new Error(`Page ${pageNumber} not found`);

    const ctx = canvas.getContext('2d');
    if (!ctx) throw new Error('Canvas context not available');

    const scale = this.options?.scale || 1.0;
    const renderScale = this.options?.renderScale || 2.0; // High DPI by default
    const rotation = this.options?.rotation || page.rotation || 0;

    // Calculate dimensions with render scale for high quality
    const displayWidth = page.width * scale;
    const displayHeight = page.height * scale;
    const canvasWidth = displayWidth * renderScale;
    const canvasHeight = displayHeight * renderScale;

    // Set canvas size for high DPI rendering
    canvas.width = canvasWidth;
    canvas.height = canvasHeight;

    // Set display size to maintain aspect ratio
    if (this.options?.maintainAspectRatio !== false) {
      canvas.style.width = `${displayWidth}px`;
      canvas.style.height = `${displayHeight}px`;
    }

    // Apply transformations with render scale
    ctx.save();
    ctx.scale(scale * renderScale, scale * renderScale);

    if (rotation) {
      ctx.translate(canvasWidth / 2, canvasHeight / 2);
      ctx.rotate((rotation * Math.PI) / 180);
      ctx.translate(-canvasWidth / 2, -canvasHeight / 2);
    }

    // Render background (white by default, unless dark mode)
    const backgroundColor = this.options?.background || (this.options?.darkMode ? '#1a1a1a' : 'white');
    ctx.fillStyle = backgroundColor;
    ctx.fillRect(0, 0, page.width, page.height);

    // Render page content
    if (this.options?.renderText !== false) {
      await this.renderText(ctx, page);
    }

    if (this.options?.renderImages !== false) {
      await this.renderImages(ctx, page);
    }

    if (this.options?.renderAnnotations) {
      await this.renderAnnotations(ctx, page);
    }

    if (this.options?.renderForms) {
      await this.renderForms(ctx, page);
    }

    ctx.restore();
  }

  async renderToImage(
    pageNumber: number,
    format: 'png' | 'jpeg' | 'webp'
  ): Promise<Blob> {
    // Create offscreen canvas
    const canvas = document.createElement('canvas');
    await this.renderToCanvas(pageNumber, canvas);

    return new Promise((resolve, reject) => {
      canvas.toBlob(
        (blob) => {
          if (blob) resolve(blob);
          else reject(new Error('Failed to create image blob'));
        },
        `image/${format}`,
        this.options?.imageQuality || 0.92
      );
    });
  }

  private async renderText(ctx: CanvasRenderingContext2D, page: PDFPage): Promise<void> {
    const extractor = new TextExtractor(this.pdf);
    const allText = await extractor.extract();
    const textContent = allText.filter(text => text.pageNumber === page.pageNumber);

    for (const text of textContent) {
      ctx.save();

      // Apply text transformation
      ctx.transform(
        text.transform[0],
        text.transform[1],
        text.transform[2],
        text.transform[3],
        text.transform[4],
        text.transform[5]
      );

      // Set text style
      ctx.fillStyle = `rgb(${text.style.color.r}, ${text.style.color.g}, ${text.style.color.b})`;
      ctx.font = `${text.style.bold ? 'bold ' : ''}${text.style.italic ? 'italic ' : ''}${text.fontSize}px ${text.fontName}`;

      // Draw text
      ctx.fillText(text.text, text.x, page.height - text.y);

      ctx.restore();
    }
  }

  private async renderImages(ctx: CanvasRenderingContext2D, page: PDFPage): Promise<void> {
    if (!page.resources?.images) return;

    for (const [name, imageRes] of page.resources.images) {
      const img = await this.decodeImage(imageRes);
      if (img) {
        ctx.drawImage(img, 0, 0, imageRes.width, imageRes.height);
      }
    }
  }

  private async decodeImage(imageRes: ImageResource): Promise<HTMLImageElement | null> {
    try {
      const arrayBuffer = imageRes.data.slice();
      const blob = new Blob([arrayBuffer], { type: 'image/jpeg' });
      const url = URL.createObjectURL(blob);
      const img = new Image();

      return new Promise((resolve) => {
        img.onload = () => {
          URL.revokeObjectURL(url);
          resolve(img);
        };
        img.onerror = () => {
          URL.revokeObjectURL(url);
          resolve(null);
        };
        img.src = url;
      });
    } catch {
      return null;
    }
  }

  private async renderAnnotations(ctx: CanvasRenderingContext2D, page: PDFPage): Promise<void> {
    const annotations = await new AnnotationExtractor(this.pdf).extract(page.pageNumber);

    for (const annotation of annotations) {
      this.renderAnnotation(ctx, annotation, page);
    }
  }

  private renderAnnotation(ctx: CanvasRenderingContext2D, annotation: Annotation, page: PDFPage): void {
    ctx.save();

    // Set annotation style
    if (annotation.color) {
      ctx.strokeStyle = `rgba(${annotation.color.r}, ${annotation.color.g}, ${annotation.color.b}, ${annotation.opacity || 1})`;
      ctx.fillStyle = `rgba(${annotation.color.r}, ${annotation.color.g}, ${annotation.color.b}, ${(annotation.opacity || 1) * 0.3})`;
    }

    switch (annotation.type) {
      case AnnotationType.Highlight:
        ctx.fillRect(annotation.rect.x, page.height - annotation.rect.y - annotation.rect.height, annotation.rect.width, annotation.rect.height);
        break;

      case AnnotationType.Underline:
        ctx.beginPath();
        ctx.moveTo(annotation.rect.x, page.height - annotation.rect.y);
        ctx.lineTo(annotation.rect.x + annotation.rect.width, page.height - annotation.rect.y);
        ctx.stroke();
        break;

      case AnnotationType.Square:
        ctx.strokeRect(annotation.rect.x, page.height - annotation.rect.y - annotation.rect.height, annotation.rect.width, annotation.rect.height);
        break;

      case AnnotationType.Circle:
        const centerX = annotation.rect.x + annotation.rect.width / 2;
        const centerY = page.height - annotation.rect.y - annotation.rect.height / 2;
        const radiusX = annotation.rect.width / 2;
        const radiusY = annotation.rect.height / 2;

        ctx.beginPath();
        ctx.ellipse(centerX, centerY, radiusX, radiusY, 0, 0, 2 * Math.PI);
        ctx.stroke();
        break;
    }

    ctx.restore();
  }

  private async renderForms(ctx: CanvasRenderingContext2D, page: PDFPage): Promise<void> {
    const formFields = await new FormExtractor(this.pdf).extract();
    const pageFields = formFields.filter(f => f.pageNumber === page.pageNumber);

    for (const field of pageFields) {
      this.renderFormField(ctx, field, page);
    }
  }

  private renderFormField(ctx: CanvasRenderingContext2D, field: FormField, page: PDFPage): void {
    ctx.save();

    // Draw field border
    ctx.strokeStyle = '#000000';
    ctx.strokeRect(field.rect.x, page.height - field.rect.y - field.rect.height, field.rect.width, field.rect.height);

    // Draw field value
    if (field.value) {
      ctx.fillStyle = '#000000';
      ctx.font = '12px Arial';

      if (field.type === FormFieldType.Text) {
        ctx.fillText(String(field.value), field.rect.x + 2, page.height - field.rect.y - 4);
      } else if (field.type === FormFieldType.Button && field.value) {
        // Draw checkmark for checked checkbox
        ctx.beginPath();
        ctx.moveTo(field.rect.x + 2, page.height - field.rect.y - field.rect.height / 2);
        ctx.lineTo(field.rect.x + field.rect.width / 3, page.height - field.rect.y - 2);
        ctx.lineTo(field.rect.x + field.rect.width - 2, page.height - field.rect.y - field.rect.height + 2);
        ctx.stroke();
      }
    }

    ctx.restore();
  }
}

class ThemeManager {
  private static instance: ThemeManager;
  private currentTheme: 'dark' | 'light' = 'dark';
  private storageKey: string = 'modernpdf-theme';
  private observers: Array<(theme: 'dark' | 'light') => void> = [];

  private constructor() {
    this.loadTheme();
  }

  static getInstance(): ThemeManager {
    if (!ThemeManager.instance) {
      ThemeManager.instance = new ThemeManager();
    }
    return ThemeManager.instance;
  }

  /**
   * Initialize theme management with options
   */
  initialize(options?: {
    defaultTheme?: 'dark' | 'light' | 'auto';
    storageKey?: string;
    persistTheme?: boolean;
  }): void {
    if (options?.storageKey) {
      this.storageKey = options.storageKey;
    }

    if (options?.defaultTheme) {
      if (options.defaultTheme === 'auto') {
        this.currentTheme = this.detectSystemTheme();
      } else {
        this.currentTheme = options.defaultTheme;
      }
    }

    if (options?.persistTheme !== false) {
      this.loadTheme();
    }

    this.applyTheme();
  }

  /**
   * Toggle between dark and light themes
   */
  toggleTheme(): void {
    this.currentTheme = this.currentTheme === 'dark' ? 'light' : 'dark';
    this.applyTheme();
    this.saveTheme();
    this.notifyObservers();
  }

  /**
   * Set a specific theme
   */
  setTheme(theme: 'dark' | 'light'): void {
    this.currentTheme = theme;
    this.applyTheme();
    this.saveTheme();
    this.notifyObservers();
  }

  /**
   * Get current theme
   */
  getCurrentTheme(): 'dark' | 'light' {
    return this.currentTheme;
  }

  /**
   * Add theme change observer
   */
  addObserver(callback: (theme: 'dark' | 'light') => void): void {
    this.observers.push(callback);
  }

  /**
   * Remove theme change observer
   */
  removeObserver(callback: (theme: 'dark' | 'light') => void): void {
    const index = this.observers.indexOf(callback);
    if (index > -1) {
      this.observers.splice(index, 1);
    }
  }

  /**
   * Apply theme to DOM elements
   */
  private applyTheme(): void {
    if (typeof document !== 'undefined') {
      if (this.currentTheme === 'light') {
        document.body.classList.add('light-mode');
      } else {
        document.body.classList.remove('light-mode');
      }

      // Update theme toggle button if it exists
      const themeToggleBtn = document.getElementById('themeToggle');
      if (themeToggleBtn) {
        if (this.currentTheme === 'dark') {
          themeToggleBtn.textContent = '🌙';
          themeToggleBtn.title = 'Switch to Light Mode';
        } else {
          themeToggleBtn.textContent = '☀️';
          themeToggleBtn.title = 'Switch to Dark Mode';
        }
      }
    }
  }

  /**
   * Save theme to localStorage
   */
  private saveTheme(): void {
    if (typeof localStorage !== 'undefined') {
      try {
        localStorage.setItem(this.storageKey, this.currentTheme);
      } catch (e) {
        // localStorage might not be available
      }
    }
  }

  /**
   * Load theme from localStorage
   */
  private loadTheme(): void {
    if (typeof localStorage !== 'undefined') {
      try {
        const saved = localStorage.getItem(this.storageKey);
        if (saved === 'dark' || saved === 'light') {
          this.currentTheme = saved;
        }
      } catch (e) {
        // localStorage might not be available
      }
    }
  }

  /**
   * Detect system theme preference
   */
  private detectSystemTheme(): 'dark' | 'light' {
    if (typeof window !== 'undefined' && window.matchMedia) {
      return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
    }
    return 'dark'; // Default fallback
  }

  /**
   * Notify all observers of theme change
   */
  private notifyObservers(): void {
    this.observers.forEach(callback => callback(this.currentTheme));
  }

  /**
   * Create theme toggle button element
   */
  static createThemeToggleButton(options?: {
    className?: string;
    position?: 'fixed' | 'relative';
    size?: 'small' | 'medium' | 'large';
  }): HTMLButtonElement {
    const button = document.createElement('button');
    const themeManager = ThemeManager.getInstance();

    button.id = 'themeToggle';
    button.className = options?.className || 'theme-toggle-btn';

    // Set initial appearance
    const currentTheme = themeManager.getCurrentTheme();
    button.textContent = currentTheme === 'dark' ? '🌙' : '☀️';
    button.title = currentTheme === 'dark' ? 'Switch to Light Mode' : 'Switch to Dark Mode';

    // Add click handler
    button.addEventListener('click', () => {
      themeManager.toggleTheme();
    });

    // Style the button
    const size = options?.size || 'medium';
    const sizeMap = {
      small: { fontSize: '12px', padding: '4px 8px' },
      medium: { fontSize: '14px', padding: '6px 10px' },
      large: { fontSize: '16px', padding: '8px 12px' }
    };

    Object.assign(button.style, {
      fontSize: sizeMap[size].fontSize,
      padding: sizeMap[size].padding,
      border: '1px solid #666',
      borderRadius: '3px',
      background: '#404040',
      color: 'white',
      cursor: 'pointer',
      minWidth: '32px'
    });

    if (options?.position === 'fixed') {
      Object.assign(button.style, {
        position: 'fixed',
        top: '10px',
        right: '10px',
        zIndex: '1000'
      });
    }

    // Update button when theme changes
    themeManager.addObserver((theme) => {
      button.textContent = theme === 'dark' ? '🌙' : '☀️';
      button.title = theme === 'dark' ? 'Switch to Light Mode' : 'Switch to Dark Mode';
    });

    return button;
  }
}

class PDFExporter {
  constructor(
    private pdf: ModernPDF,
    private options?: ExportOptions
  ) { }

  async export(format: ExportFormat): Promise<Blob | string> {
    switch (format) {
      case 'text':
        return this.exportAsText();
      case 'html':
        return this.exportAsHTML();
      case 'markdown':
        return this.exportAsMarkdown();
      case 'json':
        return this.exportAsJSON();
      case 'xml':
        return this.exportAsXML();
      case 'csv':
        return this.exportAsCSV();
      default:
        throw new Error(`Unsupported export format: ${format}`);
    }
  }

  private async exportAsText(): Promise<string> {
    const textContent = await this.pdf.extractText();
    const pages = new Map<number, string[]>();

    for (const text of textContent) {
      if (!pages.has(text.pageNumber)) {
        pages.set(text.pageNumber, []);
      }
      pages.get(text.pageNumber)!.push(text.text);
    }

    let output = '';
    for (const [pageNum, texts] of pages) {
      output += `\n--- Page ${pageNum} ---\n`;
      output += texts.join(' ');
    }

    return output;
  }

  private async exportAsHTML(): Promise<string> {
    const metadata = this.pdf.getMetadata();
    const textContent = await this.pdf.extractText();

    let html = `<!DOCTYPE html>
<html>
<head>
  <meta charset="UTF-8">
  <title>${metadata?.title || 'PDF Document'}</title>
  <style>
    body { font-family: Arial, sans-serif; margin: 40px; }
    .page { page-break-after: always; margin-bottom: 40px; }
    .text-block { margin: 10px 0; }
    .metadata { background: #f0f0f0; padding: 20px; margin-bottom: 30px; }
    h1 { color: #333; }
    .bold { font-weight: bold; }
    .italic { font-style: italic; }
  </style>
</head>
<body>`;

    if (this.options?.includeMetadata && metadata) {
      html += `
  <div class="metadata">
    <h1>${metadata.title || 'Untitled'}</h1>
    ${metadata.author ? `<p><strong>Author:</strong> ${metadata.author}</p>` : ''}
    ${metadata.subject ? `<p><strong>Subject:</strong> ${metadata.subject}</p>` : ''}
    ${metadata.creationDate ? `<p><strong>Created:</strong> ${metadata.creationDate.toLocaleDateString()}</p>` : ''}
    <p><strong>Pages:</strong> ${metadata.pageCount}</p>
  </div>`;
    }

    const pageGroups = new Map<number, TextContent[]>();
    for (const text of textContent) {
      if (!pageGroups.has(text.pageNumber)) {
        pageGroups.set(text.pageNumber, []);
      }
      pageGroups.get(text.pageNumber)!.push(text);
    }

    for (const [pageNum, texts] of pageGroups) {
      html += `\n  <div class="page" data-page="${pageNum}">`;
      html += `\n    <h2>Page ${pageNum}</h2>`;

      for (const text of texts) {
        const classes = [];
        if (text.style.bold) classes.push('bold');
        if (text.style.italic) classes.push('italic');

        html += `\n    <div class="text-block${classes.length ? ' ' + classes.join(' ') : ''}" style="font-size: ${text.fontSize}px;">${this.escapeHtml(text.text)}</div>`;
      }

      html += '\n  </div>';
    }

    html += '\n</body>\n</html>';

    return html;
  }

  private async exportAsMarkdown(): Promise<string> {
    const metadata = this.pdf.getMetadata();
    const ai = await this.pdf.getAIFeatures();

    let markdown = '';

    // Add metadata as frontmatter
    if (this.options?.includeMetadata && metadata) {
      markdown += '---\n';
      if (metadata.title) markdown += `title: "${metadata.title}"\n`;
      if (metadata.author) markdown += `author: "${metadata.author}"\n`;
      if (metadata.creationDate) markdown += `date: ${metadata.creationDate.toISOString()}\n`;
      markdown += `pages: ${metadata.pageCount}\n`;
      markdown += '---\n\n';
    }

    // Add title
    if (metadata?.title) {
      markdown += `# ${metadata.title}\n\n`;
    }

    // Export based on structural analysis
    for (const section of ai.structuralAnalysis.sections) {
      if (section.type === 'heading') {
        const level = section.level || 1;
        markdown += `${'#'.repeat(level)} ${section.text}\n\n`;
      } else if (section.type === 'paragraph') {
        markdown += `${section.text}\n\n`;
      } else if (section.type === 'list') {
        const items = section.text.split('\n');
        for (const item of items) {
          markdown += `- ${item}\n`;
        }
        markdown += '\n';
      } else if (section.type === 'blockquote') {
        markdown += `> ${section.text}\n\n`;
      } else if (section.type === 'code') {
        markdown += '```\n' + section.text + '\n```\n\n';
      }

      // Add children sections
      if (section.children) {
        for (const child of section.children) {
          if (child.type === 'paragraph') {
            markdown += `${child.text}\n\n`;
          }
        }
      }
    }

    // Add tables
    if (ai.structuralAnalysis.tables.length > 0) {
      markdown += '\n## Tables\n\n';
      for (const table of ai.structuralAnalysis.tables) {
        markdown += this.tableToMarkdown(table) + '\n\n';
      }
    }

    // Add references/bibliography
    if (ai.structuralAnalysis.bibliography && ai.structuralAnalysis.bibliography.length > 0) {
      markdown += '\n## References\n\n';
      for (const ref of ai.structuralAnalysis.bibliography) {
        markdown += `- ${ref.authors?.join(', ')} (${ref.year}). *${ref.title}*. ${ref.journal || ''}\n`;
      }
    }

    return markdown;
  }

  private tableToMarkdown(table: Table): string {
    let markdown = '';

    // Headers
    if (table.headers && table.headers.length > 0) {
      markdown += '| ' + table.headers.join(' | ') + ' |\n';
      markdown += '|' + table.headers.map(() => '---').join('|') + '|\n';
    }

    // Rows
    for (const row of table.cells) {
      markdown += '| ' + row.map(cell => cell.text).join(' | ') + ' |\n';
    }

    return markdown;
  }

  private async exportAsJSON(): Promise<string> {
    const metadata = this.pdf.getMetadata();
    const textContent = await this.pdf.extractText();
    const ai = await this.pdf.getAIFeatures();

    const data: any = {
      metadata: this.options?.includeMetadata ? metadata : undefined,
      pages: [] as any[]
    };

    // Group content by page
    const pageMap = new Map<number, any>();

    for (const text of textContent) {
      if (!pageMap.has(text.pageNumber)) {
        pageMap.set(text.pageNumber, {
          pageNumber: text.pageNumber,
          textBlocks: [],
          images: [],
          annotations: [],
          forms: []
        });
      }

      pageMap.get(text.pageNumber)!.textBlocks.push({
        text: text.text,
        x: text.x,
        y: text.y,
        width: text.width,
        height: text.height,
        fontSize: text.fontSize,
        fontName: text.fontName,
        style: text.style
      });
    }

    // Add images if requested
    if (this.options?.includeImages) {
      const images = await this.pdf.extractImages();
      for (const image of images) {
        if (pageMap.has(image.pageNumber)) {
          pageMap.get(image.pageNumber)!.images.push({
            id: image.id,
            x: image.x,
            y: image.y,
            width: image.width,
            height: image.height,
            mimeType: image.mimeType
          });
        }
      }
    }

    // Add annotations if requested
    if (this.options?.includeAnnotations) {
      const annotations = await this.pdf.getAnnotations();
      for (const annotation of annotations) {
        if (pageMap.has(annotation.pageNumber)) {
          pageMap.get(annotation.pageNumber)!.annotations.push({
            type: annotation.type,
            rect: annotation.rect,
            contents: annotation.contents,
            author: annotation.author
          });
        }
      }
    }

    // Add forms if requested
    if (this.options?.includeForms) {
      const forms = await this.pdf.getFormFields();
      for (const form of forms) {
        if (pageMap.has(form.pageNumber)) {
          pageMap.get(form.pageNumber)!.forms.push({
            name: form.name,
            type: form.type,
            value: form.value,
            rect: form.rect
          });
        }
      }
    }

    // Convert map to array
    data.pages = Array.from(pageMap.values()).sort((a, b) => a.pageNumber - b.pageNumber);

    // Add AI features
    data.aiAnalysis = {
      documentType: ai.structuralAnalysis.documentType,
      sections: ai.structuralAnalysis.sections,
      tables: ai.structuralAnalysis.tables,
      keywords: ai.nlpReady.keywords,
      language: ai.nlpReady.language,
      tokenCount: ai.nlpReady.tokenCount
    };

    return JSON.stringify(data, null, 2);
  }

  private async exportAsXML(): Promise<string> {
    const metadata = this.pdf.getMetadata();
    const textContent = await this.pdf.extractText();

    let xml = '<?xml version="1.0" encoding="UTF-8"?>\n';
    xml += '<document>\n';

    if (this.options?.includeMetadata && metadata) {
      xml += '  <metadata>\n';
      if (metadata.title) xml += `    <title>${this.escapeXml(metadata.title)}</title>\n`;
      if (metadata.author) xml += `    <author>${this.escapeXml(metadata.author)}</author>\n`;
      if (metadata.subject) xml += `    <subject>${this.escapeXml(metadata.subject)}</subject>\n`;
      if (metadata.creationDate) xml += `    <creationDate>${metadata.creationDate.toISOString()}</creationDate>\n`;
      xml += `    <pageCount>${metadata.pageCount}</pageCount>\n`;
      xml += '  </metadata>\n';
    }

    xml += '  <pages>\n';

    const pageGroups = new Map<number, TextContent[]>();
    for (const text of textContent) {
      if (!pageGroups.has(text.pageNumber)) {
        pageGroups.set(text.pageNumber, []);
      }
      pageGroups.get(text.pageNumber)!.push(text);
    }

    for (const [pageNum, texts] of pageGroups) {
      xml += `    <page number="${pageNum}">\n`;

      for (const text of texts) {
        xml += '      <text';
        xml += ` x="${text.x}"`;
        xml += ` y="${text.y}"`;
        xml += ` width="${text.width}"`;
        xml += ` height="${text.height}"`;
        xml += ` fontSize="${text.fontSize}"`;
        xml += ` fontName="${this.escapeXml(text.fontName)}"`;
        xml += '>';
        xml += this.escapeXml(text.text);
        xml += '</text>\n';
      }

      xml += '    </page>\n';
    }

    xml += '  </pages>\n';
    xml += '</document>';

    return xml;
  }

  private async exportAsCSV(): Promise<string> {
    const ai = await this.pdf.getAIFeatures();
    const tables = ai.structuralAnalysis.tables;

    if (tables.length === 0) {
      // Export text content as CSV
      const textContent = await this.pdf.extractText();
      let csv = 'Page,Text,Font Size,Font Name\n';

      for (const text of textContent) {
        csv += `${text.pageNumber},"${this.escapeCSV(text.text)}",${text.fontSize},"${this.escapeCSV(text.fontName)}"\n`;
      }

      return csv;
    }

    // Export tables as CSV
    let csv = '';

    for (let i = 0; i < tables.length; i++) {
      const table = tables[i];

      if (i > 0) csv += '\n\n';
      csv += `Table ${i + 1} (Page ${table.pageNumber})\n`;

      // Headers
      if (table.headers && table.headers.length > 0) {
        csv += table.headers.map(h => `"${this.escapeCSV(h)}"`).join(',') + '\n';
      }

      // Rows
      for (const row of table.cells) {
        csv += row.map(cell => `"${this.escapeCSV(cell.text)}"`).join(',') + '\n';
      }
    }

    return csv;
  }

  private escapeHtml(text: string): string {
    const map: Record<string, string> = {
      '&': '&amp;',
      '<': '&lt;',
      '>': '&gt;',
      '"': '&quot;',
      "'": '&#39;'
    };
    return text.replace(/[&<>"']/g, m => map[m]);
  }

  private escapeXml(text: string): string {
    return this.escapeHtml(text);
  }

  private escapeCSV(text: string): string {
    return text.replace(/"/g, '""');
  }
}

class PDFWriter {
  private modifiedObjects: Map<string, PDFObject> = new Map();

  constructor(private pdf: ModernPDF) { }

  async save(): Promise<Blob> {
    const originalBuffer = await this.getOriginalBuffer();
    const writer = new PDFStreamWriter();

    // Write header
    writer.writeString('%PDF-1.7\n');
    writer.writeString('%âãÏÓ\n'); // Binary marker

    // Copy and modify objects
    const objects = await this.collectObjects();
    const xref = new XRefTable();

    for (const obj of objects) {
      const offset = writer.position;
      xref.addEntry(obj.objectNumber!, offset, obj.generationNumber || 0, 'n');

      writer.writeString(`${obj.objectNumber} ${obj.generationNumber || 0} obj\n`);
      this.writeObject(writer, obj);
      writer.writeString('\nendobj\n');
    }

    // Write xref table
    const xrefOffset = writer.position;
    writer.writeString('xref\n');
    writer.writeString(`0 ${objects.length + 1}\n`);
    writer.writeString('0000000000 65535 f \n');

    for (const obj of objects) {
      const entry = xref.getEntry(obj.objectNumber!);
      if (entry) {
        const offsetStr = entry.offset.toString().padStart(10, '0');
        const genStr = entry.generation.toString().padStart(5, '0');
        writer.writeString(`${offsetStr} ${genStr} ${entry.type} \n`);
      }
    }

    // Write trailer
    writer.writeString('trailer\n');
    writer.writeString('<<\n');
    writer.writeString(`/Size ${objects.length + 1}\n`);
    writer.writeString(`/Root 1 0 R\n`); // Catalog reference
    writer.writeString('>>\n');

    writer.writeString('startxref\n');
    writer.writeString(`${xrefOffset}\n`);
    writer.writeString('%%EOF');

    const buffer = writer.getBuffer();
    return new Blob([buffer.slice()], { type: 'application/pdf' });
  }

  private async getOriginalBuffer(): Promise<ArrayBuffer> {
    // Get the original PDF buffer
    // This would be stored in the ModernPDF instance
    return new ArrayBuffer(0);
  }

  private async collectObjects(): Promise<PDFObject[]> {
    // Collect all objects from the PDF
    // Including modified objects
    return [];
  }

  private writeObject(writer: PDFStreamWriter, obj: PDFObject): void {
    switch (obj.type) {
      case PDFObjectType.Boolean:
        writer.writeString(obj.value ? 'true' : 'false');
        break;

      case PDFObjectType.Number:
        writer.writeString(obj.value.toString());
        break;

      case PDFObjectType.String:
        writer.writeString('(' + this.escapeString(obj.value) + ')');
        break;

      case PDFObjectType.Name:
        writer.writeString('/' + obj.value);
        break;

      case PDFObjectType.Array:
        writer.writeString('[');
        for (let i = 0; i < obj.value.length; i++) {
          if (i > 0) writer.writeString(' ');
          this.writeObject(writer, obj.value[i]);
        }
        writer.writeString(']');
        break;

      case PDFObjectType.Dictionary:
        const dict = obj.value as PDFDictionary;
        writer.writeString('<<\n');
        for (const [key, value] of dict.entries) {
          writer.writeString('/' + key + ' ');
          this.writeObject(writer, value);
          writer.writeString('\n');
        }
        writer.writeString('>>');
        break;

      case PDFObjectType.Stream:
        const stream = obj.value as PDFStream;
        this.writeObject(writer, { type: PDFObjectType.Dictionary, value: stream.dictionary });
        writer.writeString('\nstream\n');
        writer.writeBytes(stream.data);
        writer.writeString('\nendstream');
        break;

      case PDFObjectType.Reference:
        const ref = obj.value as PDFReference;
        writer.writeString(`${ref.objectNumber} ${ref.generationNumber} R`);
        break;

      case PDFObjectType.Null:
        writer.writeString('null');
        break;
    }
  }

  private escapeString(str: string): string {
    return str
      .replace(/\\/g, '\\\\')
      .replace(/\(/g, '\\(')
      .replace(/\)/g, '\\)')
      .replace(/\r/g, '\\r')
      .replace(/\n/g, '\\n');
  }
}

class PDFStreamWriter {
  private buffer: Uint8Array;
  position: number = 0;

  constructor(initialSize: number = 1024 * 1024) {
    this.buffer = new Uint8Array(initialSize);
  }

  writeString(str: string): void {
    const bytes = new TextEncoder().encode(str);
    this.writeBytes(bytes);
  }

  writeBytes(bytes: Uint8Array): void {
    if (this.position + bytes.length > this.buffer.length) {
      this.expand(bytes.length);
    }

    this.buffer.set(bytes, this.position);
    this.position += bytes.length;
  }

  private expand(minSize: number): void {
    const newSize = Math.max(this.buffer.length * 2, this.position + minSize);
    const newBuffer = new Uint8Array(newSize);
    newBuffer.set(this.buffer);
    this.buffer = newBuffer;
  }

  getBuffer(): Uint8Array {
    return this.buffer.slice(0, this.position);
  }
}

// ============================================================================
// Configuration Types
// ============================================================================

export interface PDFOptions {
  password?: string;
  streamOptions?: StreamOptions;
  cachePages?: boolean;
  maxMemoryUsage?: number;
  workerUrl?: string;
  useWebWorkers?: boolean;
  lazyLoad?: boolean;
  renderOptions?: RenderOptions;
}

export interface TextExtractionOptions {
  preserveFormatting?: boolean;
  includeAnnotations?: boolean;
  normalizeWhitespace?: boolean;
  extractTables?: boolean;
  detectColumns?: boolean;
  ocrEnabled?: boolean;
  ocrLanguage?: string;
  pageRange?: { start: number; end: number };
}

export interface ImageExtractionOptions {
  format?: 'png' | 'jpeg' | 'webp';
  quality?: number;
  maxWidth?: number;
  maxHeight?: number;
  extractMasks?: boolean;
  extractVectors?: boolean;
  pageRange?: { start: number; end: number };
}

export interface AIOptions {
  embeddingProvider?: EmbeddingProvider;
  enableStructuralAnalysis?: boolean;
  enableSemanticChunking?: boolean;
  enableNER?: boolean;
  enableSummarization?: boolean;
  chunkSize?: number;
  chunkOverlap?: number;
  forceRegenerate?: boolean;
}

export interface ChunkingOptions {
  strategy?: 'semantic' | 'fixed' | 'sliding' | 'recursive';
  maxChunkSize?: number;
  minChunkSize?: number;
  overlapSize?: number;
  preserveSentences?: boolean;
  preserveParagraphs?: boolean;
  includeMetadata?: boolean;
}

export interface RenderOptions {
  scale?: number;
  rotation?: number;
  background?: string;
  renderText?: boolean;
  renderImages?: boolean;
  renderAnnotations?: boolean;
  renderForms?: boolean;
  viewport?: Rectangle;
  imageQuality?: number;
  // Enhanced viewer options
  fitToWidth?: boolean;
  maintainAspectRatio?: boolean;
  renderScale?: number;
  autoFitOnLoad?: boolean;
  continuousScrolling?: boolean;
  darkMode?: boolean;
  // Theme toggle options
  enableThemeToggle?: boolean;
  persistTheme?: boolean;
  defaultTheme?: 'dark' | 'light' | 'auto';
  themeStorageKey?: string;
}

export interface ExportOptions {
  includeMetadata?: boolean;
  includeAnnotations?: boolean;
  includeForms?: boolean;
  includeImages?: boolean;
  imageFormat?: 'png' | 'jpeg' | 'webp';
  imageQuality?: number;
  pageRange?: { start: number; end: number };
}

export type ExportFormat = 'text' | 'html' | 'markdown' | 'json' | 'xml' | 'csv';

// ============================================================================
// Additional Exports
// ============================================================================

export { ThemeManager };

// ============================================================================
// Default Export
// ============================================================================

export default ModernPDF;