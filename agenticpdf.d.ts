/**
 * ModernPDF - A complete, production-ready PDF processing library
 * with first-class support for streaming and AI systems
 */
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
    decodeParms?: Record<string, any>;
    smaskData?: Uint8Array;
    smaskWidth?: number;
    smaskHeight?: number;
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
export declare enum AnnotationType {
    Text = "Text",
    Link = "Link",
    FreeText = "FreeText",
    Line = "Line",
    Square = "Square",
    Circle = "Circle",
    Polygon = "Polygon",
    PolyLine = "PolyLine",
    Highlight = "Highlight",
    Underline = "Underline",
    Squiggly = "Squiggly",
    StrikeOut = "StrikeOut",
    Stamp = "Stamp",
    Caret = "Caret",
    Ink = "Ink",
    Popup = "Popup",
    FileAttachment = "FileAttachment",
    Sound = "Sound",
    Movie = "Movie",
    Widget = "Widget",
    Screen = "Screen",
    PrinterMark = "PrinterMark",
    TrapNet = "TrapNet",
    Watermark = "Watermark",
    Redact = "Redact"
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
    buttonSubType?: 'checkbox' | 'radio' | 'pushbutton';
}
export declare enum FormFieldType {
    Button = "Button",
    Text = "Text",
    Choice = "Choice",
    Signature = "Signature"
}
export interface FormFieldOption {
    value: string;
    label: string;
    selected: boolean;
}
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
export declare enum DocumentType {
    Article = "Article",
    Book = "Book",
    Report = "Report",
    Form = "Form",
    Invoice = "Invoice",
    Resume = "Resume",
    Presentation = "Presentation",
    Manual = "Manual",
    Other = "Other"
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
export declare enum ChunkType {
    Title = "Title",
    Header = "Header",
    Paragraph = "Paragraph",
    List = "List",
    Table = "Table",
    Figure = "Figure",
    Code = "Code",
    Quote = "Quote",
    Footnote = "Footnote"
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
// Agentic AI Tool Schema Types
export type ToolSchemaFormat = 'openai' | 'anthropic' | 'generic';

export interface ToolParameter {
    name: string;
    type: string;
    description: string;
    required: boolean;
    enum?: string[];
    default?: any;
    minimum?: number;
    maximum?: number;
    items?: { type: string };
}

export interface ToolSchema {
    name: string;
    description: string;
    parameters: ToolParameter[];
    returnType: string;
    category: string;
    streaming: boolean;
    readOnly: boolean;
}

export interface MCPManifest {
    protocol: string;
    name: string;
    version: string;
    description: string;
    tools: MCPTool[];
    resources: MCPResource[];
}

export interface MCPTool {
    name: string;
    description: string;
    inputSchema: Record<string, any>;
    annotations?: Record<string, any>;
}

export interface MCPResource {
    uri: string;
    name: string;
    description: string;
    mimeType: string;
}

export interface AgentSession {
    sessionId: string;
    documentInfo: {
        pageCount: number;
        fileSize: number;
        version: string;
        encrypted: boolean;
    };
    availableTools: string[];
    created: string;
}

// DoD Security Types
export interface SecurityConfig {
    maxFileSize: number;
    maxPageCount: number;
    maxObjectCount: number;
    maxStreamSize: number;
    maxRecursionDepth: number;
    maxStringLength: number;
    maxDictEntries: number;
    maxXRefEntries: number;
    allowJavaScript: boolean;
    allowExternalResources: boolean;
    allowEncryptedPDFs: boolean;
    sanitizeStrings: boolean;
}

export interface SBOMEntry {
    name: string;
    version: string;
    license: string;
    type: string;
    purl?: string;
    cpe?: string;
    supplier?: string;
}

export interface SBOM {
    bomFormat: string;
    specVersion: string;
    version: number;
    metadata: {
        timestamp: string;
        component: SBOMEntry;
        tools: { name: string; version: string }[];
    };
    components: SBOMEntry[];
}

export declare const DEFAULT_SECURITY_CONFIG: SecurityConfig;

// Ontology & AI Agent Discovery Types
export interface OntologyConcept {
    id: string;
    label: string;
    description: string;
    properties: { name: string; type: string; description: string }[];
    relationships: { type: string; target: string; description: string }[];
}

export interface Capability {
    id: string;
    name: string;
    description: string;
    category: 'loading' | 'extraction' | 'rendering' | 'analysis' | 'search' | 'forms' | 'annotations' | 'export' | 'memory' | 'streaming';
    methods: MethodDescriptor[];
    inputTypes: string[];
    outputTypes: string[];
    streaming: boolean;
}

export interface MethodDescriptor {
    name: string;
    description: string;
    parameters: { name: string; type: string; required: boolean; description: string }[];
    returnType: string;
    async: boolean;
    streaming: boolean;
    static: boolean;
    example?: string;
}

export interface Workflow {
    id: string;
    name: string;
    description: string;
    steps: { order: number; method: string; description: string; example?: string }[];
}

export interface LibraryOntology {
    '@context': string;
    '@type': string;
    name: string;
    version: string;
    license: string;
    description: string;
    concepts: OntologyConcept[];
    capabilities: Capability[];
    workflows: Workflow[];
    enums: Record<string, string[]>;
}

export interface DocumentCapabilityReport {
    documentInfo: { pageCount: number; fileSize: number; version: string; encrypted: boolean };
    availableOperations: string[];
    recommendedWorkflows: string[];
    estimatedComplexity: 'simple' | 'moderate' | 'complex';
}

export interface OutlineItem {
    title: string;
    destination: string | null;
    page: number | null;
    bold: boolean;
    italic: boolean;
    color: Color | null;
    children: OutlineItem[];
}

export interface EmbeddingProvider {
    model: string;
    dimensions: number;
    generate(text: string): Promise<Float32Array>;
    generateBatch(texts: string[]): Promise<Float32Array[]>;
}
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
export declare class AgenticPDF {
    private options;
    private buffer?;
    private stream?;
    private metadata?;
    private pages;
    private aiFeatures?;
    private xrefTable?;
    private catalog?;
    private pageTree?;
    private objects;
    constructor(options?: PDFOptions);
    /**
     * Load PDF from various sources
     */
    static fromFile(file: File, options?: PDFOptions): Promise<ModernPDF>;
    static fromUrl(url: string, options?: PDFOptions): Promise<ModernPDF>;
    static fromBuffer(buffer: ArrayBuffer, options?: PDFOptions): Promise<ModernPDF>;
    static fromStream(stream: ReadableStream<Uint8Array>, options?: PDFOptions): ModernPDF;
    private loadFromFile;
    private loadFromUrl;
    private loadFromBuffer;
    private loadFromStream;
    /**
     * Parse PDF content
     */
    private parse;
    private parseStream;
    /**
     * Get PDF metadata
     */
    getMetadata(): PDFMetadata | undefined;
    /**
     * Get specific page
     */
    getPage(pageNumber: number): Promise<PDFPage | undefined>;
    private loadPageFromStream;
    /**
     * Get all pages
     */
    getAllPages(): Promise<PDFPage[]>;
    /**
     * Extract text content with AI-ready formatting
     */
    extractText(options?: TextExtractionOptions): Promise<TextContent[]>;
    /**
     * Extract text as a stream for large documents
     */
    streamText(options?: TextExtractionOptions): AsyncGenerator<TextContent>;
    /**
     * Extract images
     */
    extractImages(options?: ImageExtractionOptions): Promise<ImageContent[]>;
    /**
     * Get AI features (structural analysis, semantic chunks, etc.)
     */
    getAIFeatures(options?: AIOptions): Promise<AIFeatures>;
    /**
     * Generate semantic chunks for RAG systems
     */
    generateSemanticChunks(options?: ChunkingOptions): Promise<SemanticChunk[]>;
    /**
     * Stream semantic chunks for memory-efficient processing
     */
    streamSemanticChunks(options?: ChunkingOptions): AsyncGenerator<SemanticChunk>;
    /**
     * Search text within the PDF
     */
    search(query: string, options?: SearchOptions): Promise<SearchResult[]>;
    /**
     * Get form fields
     */
    getFormFields(): Promise<FormField[]>;
    /**
     * Fill form fields
     */
    fillForm(data: Record<string, any>): Promise<void>;
    /**
     * Get current form data (original + filled values)
     */
    getFormData(): Promise<Record<string, any>>;
    /**
     * Convert an extracted image to a data URL for display
     */
    imageToDataURL(image: ImageContent, format?: 'png' | 'jpeg' | 'webp', quality?: number): Promise<string>;
    /**
     * Extract all images and return as display-ready data URLs
     */
    exportImageAsDataURL(options?: ImageExtractionOptions): Promise<Array<{ image: ImageContent; dataUrl: string }>>;
    /**
     * Get annotations
     */
    getAnnotations(pageNumber?: number): Promise<Annotation[]>;
    /**
     * Add annotation
     */
    addAnnotation(annotation: Partial<Annotation>): Promise<string>;
    /**
     * Render page to canvas
     */
    renderPage(pageNumber: number, canvas: HTMLCanvasElement, options?: RenderOptions): Promise<void>;
    /**
     * Render page to image
     */
    renderPageToImage(pageNumber: number, format?: 'png' | 'jpeg' | 'webp', options?: RenderOptions): Promise<Blob>;
    /**
     * Export to different formats
     */
    exportAs(format: ExportFormat, options?: ExportOptions): Promise<Blob | string>;
    /**
     * Save modified PDF
     */
    save(): Promise<Blob>;
    /**
     * Create an optimal PDF viewer with theme toggle functionality
     * @param container - Container element for the viewer
     * @param options - Additional render options
     * @returns Viewer object with controls and cleanup method
     */
    createOptimalViewer(container: HTMLElement, options?: RenderOptions): {
        canvas: HTMLCanvasElement;
        toolbar: HTMLElement;
        themeToggle: HTMLButtonElement;
        themeManager: ThemeManager;
        destroy: () => void;
    };
    /**
     * Get theme manager instance for manual theme control
     */
    static getThemeManager(): ThemeManager;
    /**
     * Initialize theme management globally
     */
    static initializeTheme(options?: {
        defaultTheme?: 'dark' | 'light' | 'auto';
        storageKey?: string;
        persistTheme?: boolean;
    }): ThemeManager;
        /**
     * Returns a complete machine-readable ontology describing the library
     */
    static describe(): LibraryOntology;
    /**
     * Returns the library's capability map organized by category
     */
    static getCapabilities(): Capability[];
    /**
     * Returns all method signatures for code generation
     */
    static getMethodSignatures(): MethodDescriptor[];
    /**
     * Returns pre-built workflow templates for common operations
     */
    static getWorkflows(): Workflow[];
    /**
     * Parse and return the document outline (bookmarks) tree
     */
    getOutline(): OutlineItem[];
    /**
     * Generate tool/function-calling schemas for AI agent integration
     */
    static getToolSchemas(format?: ToolSchemaFormat): Record<string, any>[];
    /**
     * Generate MCP (Model Context Protocol) server manifest
     */
    static getMCPManifest(): MCPManifest;
    /**
     * Generate JSON Schema definitions for all input/output types
     */
    static getJSONSchemas(): Record<string, Record<string, any>>;
    /**
     * Single-call introspection endpoint for AI agents
     */
    static describeForAgent(format?: ToolSchemaFormat): Record<string, any>;
    /**
     * Generate CycloneDX SBOM for supply chain security
     */
    static generateSBOM(): SBOM;
    /**
     * Get hardened security configuration for DoD environments
     */
    static getSecurityConfig(): SecurityConfig;
    /**
     * Validate PDF data against security constraints
     */
    static validateSecurityConstraints(data: Uint8Array, config?: Partial<SecurityConfig>): string[];
    /**
     * Create an agent session context for tool orchestration
     */
    createAgentSession(): AgentSession;
    /**
     * Describes the loaded document's available operations and recommended workflows
     */
    describeDocument(): DocumentCapabilityReport | undefined;
    /**
     * Close and cleanup resources
     */
    close(): void;
    /**
     * Get page count
     */
    getPageCount(): number;
    /**
     * Get file size
     */
    getFileSize(): number;
    /**
     * Check if PDF is encrypted
     */
    isEncrypted(): boolean;
    /**
     * Get PDF version
     */
    getVersion(): string;
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
declare class ThemeManager {
    private static instance;
    private currentTheme;
    private storageKey;
    private observers;
    private constructor();
    static getInstance(): ThemeManager;
    /**
     * Initialize theme management with options
     */
    initialize(options?: {
        defaultTheme?: 'dark' | 'light' | 'auto';
        storageKey?: string;
        persistTheme?: boolean;
    }): void;
    /**
     * Toggle between dark and light themes
     */
    toggleTheme(): void;
    /**
     * Set a specific theme
     */
    setTheme(theme: 'dark' | 'light'): void;
    /**
     * Get current theme
     */
    getCurrentTheme(): 'dark' | 'light';
    /**
     * Add theme change observer
     */
    addObserver(callback: (theme: 'dark' | 'light') => void): void;
    /**
     * Remove theme change observer
     */
    removeObserver(callback: (theme: 'dark' | 'light') => void): void;
    /**
     * Apply theme to DOM elements
     */
    private applyTheme;
    /**
     * Save theme to localStorage
     */
    private saveTheme;
    /**
     * Load theme from localStorage
     */
    private loadTheme;
    /**
     * Detect system theme preference
     */
    private detectSystemTheme;
    /**
     * Notify all observers of theme change
     */
    private notifyObservers;
    /**
     * Create theme toggle button element
     */
    static createThemeToggleButton(options?: {
        className?: string;
        position?: 'fixed' | 'relative';
        size?: 'small' | 'medium' | 'large';
    }): HTMLButtonElement;
}
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
    pageRange?: {
        start: number;
        end: number;
    };
}
export interface ImageExtractionOptions {
    format?: 'png' | 'jpeg' | 'webp';
    quality?: number;
    maxWidth?: number;
    maxHeight?: number;
    extractMasks?: boolean;
    extractVectors?: boolean;
    pageRange?: {
        start: number;
        end: number;
    };
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
    fitToWidth?: boolean;
    maintainAspectRatio?: boolean;
    renderScale?: number;
    autoFitOnLoad?: boolean;
    continuousScrolling?: boolean;
    darkMode?: boolean;
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
    pageRange?: {
        start: number;
        end: number;
    };
}
export type ExportFormat = 'text' | 'html' | 'markdown' | 'json' | 'xml' | 'csv';
export { ThemeManager };
export default AgenticPDF;
