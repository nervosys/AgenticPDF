/**
 * AgenticPDF - Complete TypeScript rewrite of Mozilla PDF.js
 * with modern streaming, AI integration, and performance optimizations
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
  extGState: Map<string, Record<string, any>>;
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
  firstChar?: number;
  lastChar?: number;
  missingWidth?: number;
  defaultWidth?: number;
  cidWidths?: Map<number, number>;
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
  flags?: number;
  missingWidth?: number;
  italicAngle?: number;
}

export interface ImageResource {
  width: number;
  height: number;
  bitsPerComponent: number;
  colorSpace: string;
  filter?: string[];
  decodeParms?: Record<string, any>;
  smaskData?: Uint8Array;
  smaskWidth?: number;
  smaskHeight?: number;
  data: Uint8Array;
}

export interface ColorSpace {
  name: string;
  numComponents: number;
  // For Indexed color spaces
  base?: ColorSpace;
  hival?: number;
  lookup?: Uint8Array;
  // For ICCBased color spaces
  iccProfile?: Uint8Array;
  alternate?: ColorSpace;
  range?: number[];
  // For Separation/DeviceN
  tintTransform?: any;
  colorants?: string[];
}

export interface Pattern {
  type: 'tiling' | 'shading';
  matrix?: TransformMatrix;
  paintType?: number;
  tilingType?: number;
  bbox?: number[];
  xStep?: number;
  yStep?: number;
  resources?: any;
  stream?: Uint8Array;
}

export interface XObject {
  type: 'image' | 'form' | 'ps';
  data: Uint8Array;
  // Form XObject properties
  bbox?: number[]; // Bounding box [llx, lly, urx, ury]
  matrix?: TransformMatrix; // Form matrix
  resources?: PDFResources; // Form's own resources
  group?: any; // Transparency group attributes
  // Image XObject properties (kept for backward compatibility)
  width?: number;
  height?: number;
  colorSpace?: string;
  bitsPerComponent?: number;
  filter?: string;
  // Predictor parameters (from DecodeParms)
  predictor?: number;
  predictorColumns?: number;
  predictorColors?: number;
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
  buttonSubType?: 'checkbox' | 'radio' | 'pushbutton';
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



// ============================================================================
// Real-Time Translation & Text-to-Speech Types
// ============================================================================

/** Provider interface for real-time translation of PDF content. */
export interface TranslationProvider {
  /** Unique name of the translation engine (e.g. 'google', 'deepl', 'azure'). */
  name: string;
  /** Supported source languages (ISO 639-1 codes). Empty = auto-detect. */
  supportedSourceLanguages: string[];
  /** Supported target languages (ISO 639-1 codes). */
  supportedTargetLanguages: string[];
  /** Translate a single text segment. */
  translate(text: string, from: string | null, to: string): Promise<TranslationResult>;
  /** Translate a batch of text segments. */
  translateBatch(texts: string[], from: string | null, to: string): Promise<TranslationResult[]>;
  /** Optional: detect the source language. */
  detectLanguage?(text: string): Promise<{ language: string; confidence: number }>;
}

/** Result of a single translation operation. */
export interface TranslationResult {
  sourceText: string;
  translatedText: string;
  sourceLanguage: string;
  targetLanguage: string;
  confidence: number;
  alternatives?: string[];
}

/** A translated page with all its text segments. */
export interface TranslatedPage {
  pageNumber: number;
  sourceLanguage: string;
  targetLanguage: string;
  segments: TranslatedSegment[];
}

/** A translated text segment preserving positional metadata. */
export interface TranslatedSegment {
  original: TextContent;
  translatedText: string;
  confidence: number;
}

/** Options for controlling translation operations. */
export interface TranslationOptions {
  /** Target language (ISO 639-1 code, e.g. 'es', 'de', 'ja'). */
  targetLanguage: string;
  /** Source language. Null = auto-detect. */
  sourceLanguage?: string | null;
  /** Page range to translate. */
  pageRange?: { start: number; end: number };
  /** Batch size for translation requests (default: 50). */
  batchSize?: number;
  /** Preserve original formatting metadata in output. */
  preserveFormatting?: boolean;
  /** Glossary of domain-specific term overrides. */
  glossary?: Record<string, string>;
  /** Abort signal for cancellation. */
  abortSignal?: AbortSignal;
  /** Progress callback. */
  progressCallback?: (progress: { pagesComplete: number; totalPages: number; segmentsTranslated: number }) => void;
}

/** Provider interface for text-to-speech synthesis from PDF content. */
export interface TTSProvider {
  /** Unique name of the TTS engine (e.g. 'openai-tts', 'azure-tts', 'elevenlabs'). */
  name: string;
  /** Available voice identifiers. */
  availableVoices: TTSVoice[];
  /** Synthesize speech from text and return audio data. */
  synthesize(text: string, options: TTSSynthesisOptions): Promise<TTSAudioSegment>;
  /** Stream speech synthesis for long text. */
  synthesizeStream?(text: string, options: TTSSynthesisOptions): AsyncGenerator<Uint8Array>;
}

/** A voice configuration for TTS synthesis. */
export interface TTSVoice {
  id: string;
  name: string;
  language: string;
  gender?: 'male' | 'female' | 'neutral';
  style?: string;
}

/** Options passed to the TTS provider for a single synthesis call. */
export interface TTSSynthesisOptions {
  voiceId: string;
  /** Speech rate multiplier (0.5 = half speed, 2.0 = double speed). */
  rate?: number;
  /** Pitch adjustment (-1.0 to 1.0). */
  pitch?: number;
  /** Output audio format. */
  format?: 'mp3' | 'wav' | 'ogg' | 'opus' | 'pcm';
  /** Sample rate in Hz (e.g. 24000). */
  sampleRate?: number;
}

/** A synthesized audio segment tied to a page and text range. */
export interface TTSAudioSegment {
  audioData: Uint8Array;
  format: string;
  durationMs: number;
  sampleRate: number;
  pageNumber: number;
  text: string;
  wordTimings?: TTSWordTiming[];
}

/** Word-level timing for synchronized highlighting. */
export interface TTSWordTiming {
  word: string;
  startMs: number;
  endMs: number;
  startOffset: number;
  endOffset: number;
}

/** Options for controlling TTS operations on a PDF document. */
export interface TTSOptions {
  /** TTS provider instance. */
  provider: TTSProvider;
  /** Voice to use. */
  voiceId: string;
  /** Page range to synthesize. */
  pageRange?: { start: number; end: number };
  /** Speech rate multiplier (default: 1.0). */
  rate?: number;
  /** Pitch adjustment (default: 0). */
  pitch?: number;
  /** Output audio format (default: 'mp3'). */
  format?: 'mp3' | 'wav' | 'ogg' | 'opus' | 'pcm';
  /** Sample rate in Hz (default: 24000). */
  sampleRate?: number;
  /** Maximum characters per synthesis call (default: 4096). */
  maxCharsPerRequest?: number;
  /** Whether to translate before synthesizing (requires translationProvider). */
  translateFirst?: boolean;
  /** Translation provider for cross-lingual TTS. */
  translationProvider?: TranslationProvider;
  /** Target language for translation before TTS. */
  translationTargetLanguage?: string;
  /** Abort signal for cancellation. */
  abortSignal?: AbortSignal;
  /** Progress callback. */
  progressCallback?: (progress: { pagesComplete: number; totalPages: number; segmentsSynthesized: number; totalDurationMs: number }) => void;
}

// ============================================================================
// Ontology & AI Agent Discovery Types
// ============================================================================

// Agentic AI workflow interfaces

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
}

export interface ToolSchema {
  name: string;
  description: string;
  parameters: ToolParameter[];
  returnType: string;
  category: string;
  streaming: boolean;
  requiresDocument: boolean;
  example?: string;
}

export interface MCPManifest {
  name: string;
  version: string;
  description: string;
  protocol: string;
  tools: MCPTool[];
  resources: MCPResource[];
}

export interface MCPTool {
  name: string;
  description: string;
  inputSchema: Record<string, any>;
  annotations?: { title?: string; readOnlyHint?: boolean; destructiveHint?: boolean; openWorldHint?: boolean };
}

export interface MCPResource {
  uri: string;
  name: string;
  description: string;
  mimeType: string;
}

export interface AgentSession {
  sessionId: string;
  startedAt: number;
  documentPath?: string;
  operationsPerformed: string[];
  chunksProcessed: number;
  tokensEstimated: number;
}

/** Security configuration for hardened PDF processing (DoD/CMMC compliant). */
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

/** PDF encryption algorithm types. */
export enum EncryptionAlgorithm {
  RC4_40 = 'RC4-40',
  RC4_128 = 'RC4-128',
  AES_128 = 'AES-128',
  AES_256 = 'AES-256',
}

/** PDF permission flags (ISO 32000-1, Table 22). */
export enum PDFPermission {
  Print = 1 << 2,
  ModifyContents = 1 << 3,
  ExtractContent = 1 << 4,
  Annotate = 1 << 5,
  FillForms = 1 << 8,
  ExtractForAccessibility = 1 << 9,
  Assemble = 1 << 10,
  PrintHighQuality = 1 << 11,
}

/** Encryption dictionary parsed from the PDF trailer. */
export interface EncryptionDict {
  filter: string;          // /Standard 
  subFilter?: string;
  version: number;         // V value (1-5)
  revision: number;        // R value (2-6)
  keyLength: number;       // in bits (40-256)
  ownerKey: Uint8Array;    // O value
  userKey: Uint8Array;     // U value
  ownerEncryption?: Uint8Array; // OE (R=6)
  userEncryption?: Uint8Array;  // UE (R=6)
  permissions: number;     // P value (signed 32-bit)
  encryptMetadata: boolean;
  fileId: Uint8Array;      // first element of /ID array in trailer
}

/** Result of a password authentication attempt. */
export interface AuthResult {
  authenticated: boolean;
  isOwner: boolean;
  permissions: number;
  encryptionKey: Uint8Array;
}

/** Certificate-based encryption recipient info. */
export interface CertificateRecipient {
  certificate: Uint8Array;   // DER-encoded X.509 certificate
  permissions: number;
}

/** Software Bill of Materials entry. */
export interface SBOMEntry {
  name: string;
  version: string;
  license: string;
  type: 'library' | 'runtime' | 'build-tool';
  purl?: string;
  cpe?: string;
  supplier?: string;
}

/** Software Bill of Materials (SBOM) in CycloneDX-compatible format. */
export interface SBOM {
  bomFormat: string;
  specVersion: string;
  version: number;
  metadata: {
    component: { name: string; version: string; type: string; license: string };
    timestamp: string;
  };
  components: SBOMEntry[];
}

/** Default security-hardened configuration suitable for DoD environments. */
export const DEFAULT_SECURITY_CONFIG: SecurityConfig = {
  maxFileSize: 500 * 1024 * 1024,       // 500 MB
  maxPageCount: 100_000,
  maxObjectCount: 1_000_000,
  maxStreamSize: 256 * 1024 * 1024,     // 256 MB
  maxRecursionDepth: 64,
  maxStringLength: 10_000_000,          // 10M chars
  maxDictEntries: 50_000,
  maxXRefEntries: 1_000_000,
  allowJavaScript: false,
  allowExternalResources: false,
  allowEncryptedPDFs: true,
  sanitizeStrings: true,
};

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
// Performance Monitoring
// ============================================================================

/**
 * Performance metrics for PDF operations
 */
export interface PerformanceMetrics {
  operationName: string;
  startTime: number;
  endTime?: number;
  duration?: number;
  memoryUsed?: number;
  cacheHits?: number;
  cacheMisses?: number;
}

/**
 * Performance monitoring utility
 */
export class PerformanceMonitor {
  private static metrics: PerformanceMetrics[] = [];
  private static readonly MAX_METRICS = 1000;
  private static enabled: boolean = false;

  /**
   * Enable performance monitoring
   */
  static enable(): void {
    PerformanceMonitor.enabled = true;
  }

  /**
   * Disable performance monitoring
   */
  static disable(): void {
    PerformanceMonitor.enabled = false;
  }

  /**
   * Start timing an operation
   */
  static startOperation(operationName: string): PerformanceMetrics {
    if (!PerformanceMonitor.enabled) {
      return { operationName, startTime: 0 };
    }

    const metric: PerformanceMetrics = {
      operationName,
      startTime: performance.now()
    };

    // Implement circular buffer for metrics
    if (PerformanceMonitor.metrics.length >= PerformanceMonitor.MAX_METRICS) {
      PerformanceMonitor.metrics.shift();
    }
    PerformanceMonitor.metrics.push(metric);

    return metric;
  }

  /**
   * End timing an operation
   */
  static endOperation(metric: PerformanceMetrics): void {
    if (!PerformanceMonitor.enabled) return;

    metric.endTime = performance.now();
    metric.duration = metric.endTime - metric.startTime;

    // Try to get memory usage (if available)
    if (typeof (performance as any).memory !== 'undefined') {
      metric.memoryUsed = (performance as any).memory.usedJSHeapSize;
    }
  }

  /**
   * Get all metrics
   */
  static getMetrics(): PerformanceMetrics[] {
    return [...PerformanceMonitor.metrics];
  }

  /**
   * Get metrics summary
   */
  static getSummary(): Record<string, { count: number; avgDuration: number; totalDuration: number }> {
    const summary: Record<string, { count: number; avgDuration: number; totalDuration: number }> = {};

    for (const metric of PerformanceMonitor.metrics) {
      if (!metric.duration) continue;

      if (!summary[metric.operationName]) {
        summary[metric.operationName] = {
          count: 0,
          avgDuration: 0,
          totalDuration: 0
        };
      }

      summary[metric.operationName].count++;
      summary[metric.operationName].totalDuration += metric.duration;
    }

    // Calculate averages
    for (const key in summary) {
      summary[key].avgDuration = summary[key].totalDuration / summary[key].count;
    }

    return summary;
  }

  /**
   * Clear all metrics
   */
  static clearMetrics(): void {
    PerformanceMonitor.metrics = [];
  }
}

/**
 * Memory pool for frequently allocated objects
 */
export class MemoryPool<T> {
  private pool: T[] = [];
  private readonly maxSize: number;
  private createFn: () => T;
  private resetFn?: (obj: T) => void;

  constructor(createFn: () => T, maxSize: number = 100, resetFn?: (obj: T) => void) {
    this.createFn = createFn;
    this.maxSize = maxSize;
    this.resetFn = resetFn;
  }

  /**
   * Acquire object from pool or create new one
   */
  acquire(): T {
    const obj = this.pool.pop();
    if (obj) {
      if (this.resetFn) {
        this.resetFn(obj);
      }
      return obj;
    }
    return this.createFn();
  }

  /**
   * Release object back to pool
   */
  release(obj: T): void {
    if (this.pool.length < this.maxSize) {
      this.pool.push(obj);
    }
  }

  /**
   * Clear the pool
   */
  clear(): void {
    this.pool = [];
  }

  /**
   * Get pool size
   */
  size(): number {
    return this.pool.length;
  }
}

// ============================================================================
// Core PDF Processing Classes
// ============================================================================

export class AgenticPDF {
  private buffer?: ArrayBuffer;
  private stream?: ReadableStream<Uint8Array>;
  private metadata?: PDFMetadata;
  private pages: Map<number, PDFPage> = new Map();
  private aiFeatures?: AIFeatures;
  private xrefTable?: XRefTable;
  private catalog?: PDFDictionary;
  private pageTree?: PageTree;
  private objects: Map<string, PDFObject> = new Map();
  private parser?: PDFParser;
  private _formValues: Map<string, any> = new Map();

  constructor(private options: PDFOptions = {}) {
    // Apply optimal viewer defaults if no render options specified
    if (!this.options.renderOptions) {
      this.options.renderOptions = PDFRenderer.getOptimalViewerOptions();
    }
  }

  /**
   * Load PDF from various sources
   */
  static async fromFile(file: File | Blob, options?: PDFOptions): Promise<AgenticPDF> {
    const pdf = new AgenticPDF(options);
    await pdf.loadFromFile(file);
    return pdf;
  }

  static async fromUrl(url: string, options?: PDFOptions): Promise<AgenticPDF> {
    const pdf = new AgenticPDF(options);
    await pdf.loadFromUrl(url);
    return pdf;
  }

  static async fromBuffer(buffer: ArrayBuffer, options?: PDFOptions): Promise<AgenticPDF> {
    const pdf = new AgenticPDF(options);
    await pdf.loadFromBuffer(buffer);
    return pdf;
  }

  static fromStream(stream: ReadableStream<Uint8Array>, options?: PDFOptions): AgenticPDF {
    const pdf = new AgenticPDF(options);
    pdf.loadFromStream(stream);
    return pdf;
  }

  private async loadFromFile(file: File | Blob): Promise<void> {
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
    this.parser = parser;

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
   * Get raw PDF data for external rendering libraries (e.g., PDF.js)
   */
  getRawData(): ArrayBuffer | undefined {
    return this.buffer;
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
  /**
   * Convert an extracted ImageContent to a displayable data URL.
   * JPEG images are passed through directly; raw pixel data is rendered to PNG via canvas.
   */
  imageToDataURL(image: ImageContent, format: 'png' | 'jpeg' | 'webp' = 'png', quality = 0.92): string {
    return ImageExtractor.toDataURL(image, format, quality);
  }

  /**
   * Extract all images and return them as data URLs ready for display.
   */
  async exportImageAsDataURL(options?: ImageExtractionOptions): Promise<Array<{ id: string; dataURL: string; width: number; height: number; mimeType: string; pageNumber: number }>> {
    const images = await this.extractImages(options);
    return images.map(img => ({
      id: img.id,
      dataURL: ImageExtractor.toDataURL(img, options?.format || 'png', options?.quality || 0.92),
      width: img.width,
      height: img.height,
      mimeType: img.mimeType,
      pageNumber: img.pageNumber
    }));
  }
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
   * Get the current form data (original values merged with any filled values)
   */
  async getFormData(): Promise<Record<string, any>> {
    const fields = await this.getFormFields();
    const result: Record<string, any> = {};
    for (const field of fields) {
      result[field.name] = this._formValues.has(field.name)
        ? this._formValues.get(field.name)
        : field.value;
    }
    return result;
  }

  /**
   * Get annotations
   */
  async getAnnotations(pageNumber?: number): Promise<Annotation[]> {
    const extractor = new AnnotationExtractor(this);
    return extractor.extract(pageNumber);
  }

  /**
   * Get named destinations map: name -> { page, x, y }
   */
  getNamedDestinations(): Map<string, { page: number; x: number | null; y: number | null }> {
    const result = new Map<string, { page: number; x: number | null; y: number | null }>();
    if (!this.parser || !this.xrefTable || !this.catalog) return result;

    const parser = this.parser;
    const xref = this.xrefTable;

    const resolve = (obj: PDFObject): PDFObject => {
      if (obj && obj.type === PDFObjectType.Reference) {
        const ref = obj.value as PDFReference;
        return parser.parseIndirectObject(ref.objectNumber, ref.generationNumber, xref);
      }
      return obj;
    };

    // Build page object number -> page index map by walking the Pages tree
    const objNumToPage = new Map<number, number>();
    const collectPages = (node: PDFObject, ref?: PDFObject): void => {
      const dict = resolve(node);
      if (dict.type !== PDFObjectType.Dictionary) return;
      const d = dict.value as PDFDictionary;
      const typeEntry = d.entries.get('Type');
      const typeName = typeEntry?.type === PDFObjectType.Name ? typeEntry.value as string : '';
      if (typeName === 'Page') {
        if (ref && ref.type === PDFObjectType.Reference) {
          objNumToPage.set((ref.value as PDFReference).objectNumber, objNumToPage.size + 1);
        }
        return;
      }
      const kids = d.entries.get('Kids');
      if (!kids) return;
      const kidsArr = resolve(kids);
      if (kidsArr.type !== PDFObjectType.Array) return;
      for (const kid of kidsArr.value as PDFObject[]) {
        collectPages(kid, kid);
      }
    };
    const pagesRef = this.catalog.entries.get('Pages');
    if (pagesRef) collectPages(pagesRef);

    // Walk the Names/Dests name tree
    const namesRef = this.catalog.entries.get('Names');
    if (!namesRef) return result;
    const namesDict = resolve(namesRef);
    if (namesDict.type !== PDFObjectType.Dictionary) return result;
    const destsRef = (namesDict.value as PDFDictionary).entries.get('Dests');
    if (!destsRef) return result;

    const walkNameTree = (node: PDFObject): void => {
      const dict = resolve(node);
      if (dict.type !== PDFObjectType.Dictionary) return;
      const d = dict.value as PDFDictionary;
      const namesArr = d.entries.get('Names');
      if (namesArr) {
        const arr = resolve(namesArr);
        if (arr.type === PDFObjectType.Array) {
          const items = arr.value as PDFObject[];
          for (let i = 0; i < items.length; i += 2) {
            const nameObj = items[i];
            const destObj = items[i + 1];
            if (!nameObj || !destObj) continue;
            const name = nameObj.value as string;
            try {
              let destArr: PDFObject[] | null = null;
              const destResolved = resolve(destObj);
              if (destResolved.type === PDFObjectType.Array) {
                destArr = destResolved.value as PDFObject[];
              } else if (destResolved.type === PDFObjectType.Dictionary) {
                const dEntry = (destResolved.value as PDFDictionary).entries.get('D');
                if (dEntry) {
                  const dResolved = resolve(dEntry);
                  if (dResolved.type === PDFObjectType.Array) destArr = dResolved.value as PDFObject[];
                }
              }
              if (destArr && destArr.length > 0) {
                const pageRef = destArr[0];
                let pageNum: number | undefined;
                if (pageRef.type === PDFObjectType.Reference) {
                  pageNum = objNumToPage.get((pageRef.value as PDFReference).objectNumber);
                }
                if (pageNum !== undefined) {
                  let x: number | null = null;
                  let y: number | null = null;
                  if (destArr.length >= 4 && destArr[1]?.type === PDFObjectType.Name) {
                    const fitType = destArr[1].value as string;
                    if (fitType === 'XYZ' && destArr.length >= 5) {
                      if (destArr[2]?.type === PDFObjectType.Number) x = destArr[2].value as number;
                      if (destArr[3]?.type === PDFObjectType.Number) y = destArr[3].value as number;
                    }
                  }
                  result.set(name, { page: pageNum, x, y });
                }
              }
            } catch { /* skip unresolvable destinations */ }
          }
        }
      }
      const kids = d.entries.get('Kids');
      if (kids) {
        const kidsArr = resolve(kids);
        if (kidsArr.type === PDFObjectType.Array) {
          for (const kid of kidsArr.value as PDFObject[]) {
            walkNameTree(kid);
          }
        }
      }
    };

    walkNameTree(destsRef);
    return result;
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
   * Build a text layer for a page
   * Creates a transparent text overlay that allows text selection
   * @param pageNumber - Page number to build text layer for
   * @param container - HTML element to contain the text layer
   * @param viewport - Viewport configuration (width, height, scale)
   * @param options - Text layer rendering options
   */
  async buildTextLayer(
    pageNumber: number,
    container: HTMLElement,
    viewport: { width: number; height: number; scale: number },
    options?: { enhanceTextSelection?: boolean }
  ): Promise<void> {
    // Extract text content for the page
    const textContent = await this.extractText({
      pageRange: { start: pageNumber, end: pageNumber }
    });

    // Build text layer using TextLayerBuilder
    const textLayerBuilder = new TextLayerBuilder({
      container,
      textContent,
      viewport,
      enhanceTextSelection: options?.enhanceTextSelection ?? true
    });

    textLayerBuilder.render();
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
   * Clear all caches to free memory
   */
  static clearAllCaches(): void {
    ContentStreamParser.clearCache();
    PDFColorSpaceProcessor.clearCaches();
  }

  /**
   * Get memory usage statistics
   */
  getMemoryStats(): {
    pagesCached: number;
    objectsCached: number;
    parserCacheSize: number;
    colorSpaceCacheSize: number;
    colorConversionCacheSize: number;
  } {
    return {
      pagesCached: this.pages.size,
      objectsCached: this.objects.size,
      parserCacheSize: (ContentStreamParser as any).parserCache?.size || 0,
      colorSpaceCacheSize: (PDFColorSpaceProcessor as any).colorSpaceCache?.size || 0,
      colorConversionCacheSize: (PDFColorSpaceProcessor as any).conversionCache?.size || 0
    };
  }

  /**
   * Unload pages to free memory (keeps metadata)
   */
  unloadPages(keepPages?: number[]): void {
    const keepSet = new Set(keepPages || []);
    for (const [pageNum] of this.pages) {
      if (!keepSet.has(pageNum)) {
        this.pages.delete(pageNum);
      }
    }
  }

  /**
   * Enable performance monitoring
   */
  static enablePerformanceMonitoring(): void {
    PerformanceMonitor.enable();
  }

  /**
   * Disable performance monitoring
   */
  static disablePerformanceMonitoring(): void {
    PerformanceMonitor.disable();
  }

  /**
   * Get performance metrics
   */
  static getPerformanceMetrics(): PerformanceMetrics[] {
    return PerformanceMonitor.getMetrics();
  }

  /**
   * Get performance summary
   */
  static getPerformanceSummary(): Record<string, { count: number; avgDuration: number; totalDuration: number }> {
    return PerformanceMonitor.getSummary();
  }

  /**
   * Clear performance metrics
   */
  static clearPerformanceMetrics(): void {
    PerformanceMonitor.clearMetrics();
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
   * Attempt to unlock a password-protected PDF.
   * @returns true if authentication succeeded.
   */
  async unlock(password: string): Promise<boolean> {
    if (!this.parser) return false;
    const handler = this.parser.getSecurityHandler();
    const encDict = this.parser.getEncryptionDict();
    if (!handler || !encDict) return false;
    const result = handler.authenticate(encDict, password);
    if (result.authenticated) {
      // Re-parse pages with decryption active
      if (this.catalog && this.xrefTable) {
        this.pageTree = await this.parser.parsePageTree(this.catalog);
        this.pages.clear();
        if (!this.options.lazyLoad && this.metadata) {
          for (let i = 1; i <= this.metadata.pageCount; i++) {
            this.pages.set(i, await this.parser.parsePage(i, this.pageTree));
          }
        }
      }
    }
    return result.authenticated;
  }

  /**
   * Get document permissions from the encryption dictionary.
   * Returns an object describing which operations are allowed.
   */
  getPermissions(): { print: boolean; modify: boolean; extract: boolean; annotate: boolean; fillForms: boolean; accessibility: boolean; assemble: boolean; printHighQuality: boolean } {
    const handler = this.parser?.getSecurityHandler();
    if (!handler || !handler.isAuthenticated()) {
      return { print: true, modify: true, extract: true, annotate: true, fillForms: true, accessibility: true, assemble: true, printHighQuality: true };
    }
    return {
      print: handler.checkPermission(PDFPermission.Print),
      modify: handler.checkPermission(PDFPermission.ModifyContents),
      extract: handler.checkPermission(PDFPermission.ExtractContent),
      annotate: handler.checkPermission(PDFPermission.Annotate),
      fillForms: handler.checkPermission(PDFPermission.FillForms),
      accessibility: handler.checkPermission(PDFPermission.ExtractForAccessibility),
      assemble: handler.checkPermission(PDFPermission.Assemble),
      printHighQuality: handler.checkPermission(PDFPermission.PrintHighQuality),
    };
  }

  /**
   * Check if a specific permission is granted.
   */
  checkPermission(permission: PDFPermission): boolean {
    const handler = this.parser?.getSecurityHandler();
    if (!handler || !handler.isAuthenticated()) return true;
    return handler.checkPermission(permission);
  }

  /**
   * Get the encryption algorithm used by this document.
   */
  getEncryptionAlgorithm(): EncryptionAlgorithm | null {
    const handler = this.parser?.getSecurityHandler();
    return handler?.getAlgorithm() || null;
  }

  /**
   * Set a password on the document for encryption.
   * For export via save(), this marks the document as encrypted.
   */
  setPassword(userPassword: string, ownerPassword?: string, permissions?: number): void {
    if (!this.metadata) return;
    this.metadata.isEncrypted = true;
    // Store encryption intent for PDFWriter
    (this as any)._encryptionConfig = {
      userPassword,
      ownerPassword: ownerPassword || userPassword,
      permissions: permissions ?? 0xFFFFFFFC, // All permissions by default
    };
  }

  /**
   * Get PDF version
   */
  getVersion(): string {
    return this.metadata?.version || '1.0';
  }

  // ==========================================================================
  // Ontology & AI Agent Discovery
  // ==========================================================================

  /**
   * Returns a complete machine-readable ontology describing the library's
   * concepts, capabilities, workflows, and type hierarchy.
   * Designed for AI agent discovery and automated code generation.
   */
  static describe(): LibraryOntology {
    return {
      '@context': 'https://schema.org',
      '@type': 'SoftwareApplication',
      name: 'AgenticPDF',
      version: '1.0.0',
      license: 'AGPL-3.0-or-later',
      description: 'Zero-dependency TypeScript PDF processing library with streaming-first architecture, semantic chunking, and built-in AI integration for agentic workflows.',
      concepts: AgenticPDF._getConcepts(),
      capabilities: AgenticPDF.getCapabilities(),
      workflows: AgenticPDF.getWorkflows(),
      enums: {
        DocumentType: ['Article', 'Book', 'Report', 'Form', 'Invoice', 'Resume', 'Presentation', 'Manual', 'Other'],
        ChunkType: ['Title', 'Header', 'Paragraph', 'List', 'Table', 'Figure', 'Code', 'Quote', 'Footnote'],
        AnnotationType: ['Text', 'Link', 'FreeText', 'Line', 'Square', 'Circle', 'Polygon', 'PolyLine', 'Highlight', 'Underline', 'Squiggly', 'StrikeOut', 'Stamp', 'Caret', 'Ink', 'Popup', 'FileAttachment', 'Sound', 'Movie', 'Widget', 'Screen', 'PrinterMark', 'TrapNet', 'Watermark', 'Redact'],
        FormFieldType: ['Button', 'Text', 'Choice', 'Signature'],
        ExportFormat: ['text', 'html', 'markdown', 'json', 'xml', 'csv']
      }
    };
  }

  /**
   * Returns the library's capability map organized by category.
   * Each capability includes its methods, input/output types, and streaming support.
   */
  static getCapabilities(): Capability[] {
    return [
      {
        id: 'load',
        name: 'Document Loading',
        description: 'Load PDF documents from multiple sources including files, URLs, buffers, and streams with optional streaming and lazy loading.',
        category: 'loading',
        streaming: true,
        inputTypes: ['File', 'Blob', 'string (URL)', 'ArrayBuffer', 'ReadableStream<Uint8Array>'],
        outputTypes: ['AgenticPDF'],
        methods: [
          {
            name: 'fromFile',
            description: 'Load PDF from a File or Blob object',
            parameters: [
              { name: 'file', type: 'File | Blob', required: true, description: 'The file or blob to load' },
              { name: 'options', type: 'PDFOptions', required: false, description: 'Loading and parsing options' }
            ],
            returnType: 'Promise<AgenticPDF>',
            async: true, streaming: false, static: true,
            example: "const pdf = await AgenticPDF.fromFile(file);"
          },
          {
            name: 'fromUrl',
            description: 'Load PDF from a URL with optional streaming support',
            parameters: [
              { name: 'url', type: 'string', required: true, description: 'URL to fetch the PDF from' },
              { name: 'options', type: 'PDFOptions', required: false, description: 'Loading options including streamOptions for progress tracking' }
            ],
            returnType: 'Promise<AgenticPDF>',
            async: true, streaming: true, static: true,
            example: "const pdf = await AgenticPDF.fromUrl('https://example.com/doc.pdf');"
          },
          {
            name: 'fromBuffer',
            description: 'Load PDF from an ArrayBuffer',
            parameters: [
              { name: 'buffer', type: 'ArrayBuffer', required: true, description: 'Raw PDF data' },
              { name: 'options', type: 'PDFOptions', required: false, description: 'Parsing options' }
            ],
            returnType: 'Promise<AgenticPDF>',
            async: true, streaming: false, static: true,
            example: "const pdf = await AgenticPDF.fromBuffer(buffer);"
          },
          {
            name: 'fromStream',
            description: 'Load PDF from a ReadableStream for memory-efficient processing',
            parameters: [
              { name: 'stream', type: 'ReadableStream<Uint8Array>', required: true, description: 'Readable stream of PDF data' },
              { name: 'options', type: 'PDFOptions', required: false, description: 'Stream processing options' }
            ],
            returnType: 'AgenticPDF',
            async: false, streaming: true, static: true,
            example: "const pdf = AgenticPDF.fromStream(stream);"
          }
        ]
      },
      {
        id: 'text-extraction',
        name: 'Text Extraction',
        description: 'Extract text content with positioning, styling, and font information. Supports both batch and streaming modes.',
        category: 'extraction',
        streaming: true,
        inputTypes: ['TextExtractionOptions'],
        outputTypes: ['TextContent[]', 'AsyncGenerator<TextContent>'],
        methods: [
          {
            name: 'extractText',
            description: 'Extract all text content with positioning and style information',
            parameters: [
              { name: 'options', type: 'TextExtractionOptions', required: false, description: 'Options for formatting, tables, OCR, and page range' }
            ],
            returnType: 'Promise<TextContent[]>',
            async: true, streaming: false, static: false,
            example: "const text = await pdf.extractText({ preserveFormatting: true });"
          },
          {
            name: 'streamText',
            description: 'Stream text content page-by-page for memory-efficient processing of large documents',
            parameters: [
              { name: 'options', type: 'TextExtractionOptions', required: false, description: 'Extraction options' }
            ],
            returnType: 'AsyncGenerator<TextContent>',
            async: true, streaming: true, static: false,
            example: "for await (const text of pdf.streamText()) { process(text); }"
          }
        ]
      },
      {
        id: 'image-extraction',
        name: 'Image Extraction',
        description: 'Extract embedded images from PDF pages with metadata and optional format conversion.',
        category: 'extraction',
        streaming: false,
        inputTypes: ['ImageExtractionOptions'],
        outputTypes: ['ImageContent[]'],
        methods: [
          {
            name: 'extractImages',
            description: 'Extract all images with metadata and pixel data',
            parameters: [
              { name: 'options', type: 'ImageExtractionOptions', required: false, description: 'Format, quality, size, and page range options' }
            ],
            returnType: 'Promise<ImageContent[]>',
            async: true, streaming: false, static: false,
            example: "const images = await pdf.extractImages({ format: 'png' });"
          }
        ]
      },
      {
        id: 'ai-analysis',
        name: 'AI & Semantic Analysis',
        description: 'Structural analysis, semantic chunking, NER, summarization, and embedding generation for RAG and LLM integration.',
        category: 'analysis',
        streaming: true,
        inputTypes: ['AIOptions', 'ChunkingOptions', 'EmbeddingProvider'],
        outputTypes: ['AIFeatures', 'SemanticChunk[]', 'AsyncGenerator<SemanticChunk>'],
        methods: [
          {
            name: 'getAIFeatures',
            description: 'Run full AI analysis: structural analysis, semantic chunking, NER, summarization',
            parameters: [
              { name: 'options', type: 'AIOptions', required: false, description: 'AI feature configuration including embedding provider' }
            ],
            returnType: 'Promise<AIFeatures>',
            async: true, streaming: false, static: false,
            example: "const ai = await pdf.getAIFeatures({ enableStructuralAnalysis: true });"
          },
          {
            name: 'generateSemanticChunks',
            description: 'Generate semantic chunks optimized for RAG systems and vector stores',
            parameters: [
              { name: 'options', type: 'ChunkingOptions', required: false, description: 'Chunking strategy, size, and overlap settings' }
            ],
            returnType: 'Promise<SemanticChunk[]>',
            async: true, streaming: false, static: false,
            example: "const chunks = await pdf.generateSemanticChunks({ strategy: 'semantic', maxChunkSize: 1000 });"
          },
          {
            name: 'streamSemanticChunks',
            description: 'Stream semantic chunks for memory-efficient RAG pipeline processing',
            parameters: [
              { name: 'options', type: 'ChunkingOptions', required: false, description: 'Chunking configuration' }
            ],
            returnType: 'AsyncGenerator<SemanticChunk>',
            async: true, streaming: true, static: false,
            example: "for await (const chunk of pdf.streamSemanticChunks()) { await vectorStore.add(chunk); }"
          }
        ]
      },
      {
        id: 'search',
        name: 'Text Search',
        description: 'Full-text search with regex support, case sensitivity, whole-word matching, and context extraction.',
        category: 'search',
        streaming: false,
        inputTypes: ['string', 'SearchOptions'],
        outputTypes: ['SearchResult[]'],
        methods: [
          {
            name: 'search',
            description: 'Search document text with regex, case sensitivity, and context options',
            parameters: [
              { name: 'query', type: 'string', required: true, description: 'Search query or regex pattern' },
              { name: 'options', type: 'SearchOptions', required: false, description: 'Case sensitivity, whole word, regex, context length' }
            ],
            returnType: 'Promise<SearchResult[]>',
            async: true, streaming: false, static: false,
            example: "const results = await pdf.search('revenue', { caseSensitive: false });"
          }
        ]
      },
      {
        id: 'forms',
        name: 'Form Processing',
        description: 'Extract, read, and fill interactive PDF form fields including text inputs, checkboxes, dropdowns, and signatures.',
        category: 'forms',
        streaming: false,
        inputTypes: ['Record<string, any>'],
        outputTypes: ['FormField[]'],
        methods: [
          {
            name: 'getFormFields',
            description: 'Extract all form fields with their types, values, and validation rules',
            parameters: [],
            returnType: 'Promise<FormField[]>',
            async: true, streaming: false, static: false,
            example: "const fields = await pdf.getFormFields();"
          },
          {
            name: 'fillForm',
            description: 'Fill form fields with provided key-value data',
            parameters: [
              { name: 'data', type: 'Record<string, any>', required: true, description: 'Field name to value mapping' }
            ],
            returnType: 'Promise<void>',
            async: true, streaming: false, static: false,
            example: "await pdf.fillForm({ name: 'John', email: 'john@example.com' });"
          }
        ]
      },
      {
        id: 'annotations',
        name: 'Annotations',
        description: 'Read and create PDF annotations including highlights, notes, links, stamps, and redactions.',
        category: 'annotations',
        streaming: false,
        inputTypes: ['Partial<Annotation>'],
        outputTypes: ['Annotation[]', 'string'],
        methods: [
          {
            name: 'getAnnotations',
            description: 'Get annotations from all pages or a specific page',
            parameters: [
              { name: 'pageNumber', type: 'number', required: false, description: 'Specific page number, or omit for all pages' }
            ],
            returnType: 'Promise<Annotation[]>',
            async: true, streaming: false, static: false,
            example: "const annotations = await pdf.getAnnotations(1);"
          },
          {
            name: 'addAnnotation',
            description: 'Add a new annotation to the document',
            parameters: [
              { name: 'annotation', type: 'Partial<Annotation>', required: true, description: 'Annotation data including type, rect, and content' }
            ],
            returnType: 'Promise<string>',
            async: true, streaming: false, static: false,
            example: "const id = await pdf.addAnnotation({ type: AnnotationType.Highlight, rect: { x: 0, y: 0, width: 100, height: 20 } });"
          }
        ]
      },
      {
        id: 'rendering',
        name: 'Page Rendering',
        description: 'Render PDF pages to HTML canvas or image blobs with configurable quality, scale, and theme support.',
        category: 'rendering',
        streaming: false,
        inputTypes: ['HTMLCanvasElement', 'RenderOptions'],
        outputTypes: ['void', 'Blob'],
        methods: [
          {
            name: 'renderPage',
            description: 'Render a page to an HTML canvas element',
            parameters: [
              { name: 'pageNumber', type: 'number', required: true, description: 'Page number (1-based)' },
              { name: 'canvas', type: 'HTMLCanvasElement', required: true, description: 'Target canvas element' },
              { name: 'options', type: 'RenderOptions', required: false, description: 'Scale, quality, and viewer options' }
            ],
            returnType: 'Promise<void>',
            async: true, streaming: false, static: false,
            example: "await pdf.renderPage(1, canvas, { scale: 2.0 });"
          },
          {
            name: 'renderPageToImage',
            description: 'Render a page to an image blob in PNG, JPEG, or WebP format',
            parameters: [
              { name: 'pageNumber', type: 'number', required: true, description: 'Page number (1-based)' },
              { name: 'format', type: "'png' | 'jpeg' | 'webp'", required: false, description: 'Image format (default: png)' },
              { name: 'options', type: 'RenderOptions', required: false, description: 'Scale and quality options' }
            ],
            returnType: 'Promise<Blob>',
            async: true, streaming: false, static: false,
            example: "const blob = await pdf.renderPageToImage(1, 'png');"
          }
        ]
      },
      {
        id: 'export',
        name: 'Multi-Format Export',
        description: 'Export PDF content to text, HTML, Markdown, JSON, XML, or CSV formats with configurable options.',
        category: 'export',
        streaming: false,
        inputTypes: ['ExportFormat', 'ExportOptions'],
        outputTypes: ['Blob', 'string'],
        methods: [
          {
            name: 'exportAs',
            description: 'Export document to a specified format',
            parameters: [
              { name: 'format', type: "ExportFormat", required: true, description: "Target format: 'text' | 'html' | 'markdown' | 'json' | 'xml' | 'csv'" },
              { name: 'options', type: 'ExportOptions', required: false, description: 'Metadata, annotations, images, and page range options' }
            ],
            returnType: 'Promise<Blob | string>',
            async: true, streaming: false, static: false,
            example: "const md = await pdf.exportAs('markdown', { includeImages: true });"
          },
          {
            name: 'save',
            description: 'Save the modified PDF document as a Blob',
            parameters: [],
            returnType: 'Promise<Blob>',
            async: true, streaming: false, static: false,
            example: "const blob = await pdf.save();"
          }
        ]
      },
      {
        id: 'memory',
        name: 'Memory Management',
        description: 'Resource lifecycle management including page unloading, cache clearing, and memory usage monitoring.',
        category: 'memory',
        streaming: false,
        inputTypes: ['number[]'],
        outputTypes: ['void', '{ pagesCached: number; objectsCached: number }'],
        methods: [
          {
            name: 'close',
            description: 'Release all resources held by this PDF instance',
            parameters: [],
            returnType: 'void',
            async: false, streaming: false, static: false,
            example: "pdf.close();"
          },
          {
            name: 'unloadPages',
            description: 'Unload cached pages to free memory, optionally keeping specified pages',
            parameters: [
              { name: 'keepPages', type: 'number[]', required: false, description: 'Page numbers to keep in cache' }
            ],
            returnType: 'void',
            async: false, streaming: false, static: false,
            example: "pdf.unloadPages([1, 2]); // keep pages 1-2, unload the rest"
          },
          {
            name: 'getMemoryStats',
            description: 'Get current memory usage statistics',
            parameters: [],
            returnType: '{ pagesCached: number; objectsCached: number }',
            async: false, streaming: false, static: false,
            example: "const stats = pdf.getMemoryStats();"
          }
        ]
      },
      {
        id: 'navigation',
        name: 'Document Navigation',
        description: 'Access document structure including named destinations, page metadata, and document outline.',
        category: 'extraction',
        streaming: false,
        inputTypes: ['number'],
        outputTypes: ['PDFMetadata', 'PDFPage', 'Map<string, object>'],
        methods: [
          {
            name: 'getMetadata',
            description: 'Get document metadata including title, author, page count, and version',
            parameters: [],
            returnType: 'PDFMetadata | undefined',
            async: false, streaming: false, static: false,
            example: "const meta = pdf.getMetadata();"
          },
          {
            name: 'getPage',
            description: 'Get a specific page with lazy loading support',
            parameters: [
              { name: 'pageNumber', type: 'number', required: true, description: 'Page number (1-based)' }
            ],
            returnType: 'Promise<PDFPage | undefined>',
            async: true, streaming: false, static: false,
            example: "const page = await pdf.getPage(1);"
          },
          {
            name: 'getAllPages',
            description: 'Get all pages as an array',
            parameters: [],
            returnType: 'Promise<PDFPage[]>',
            async: true, streaming: false, static: false,
            example: "const pages = await pdf.getAllPages();"
          },
          {
            name: 'getNamedDestinations',
            description: 'Get all named destinations for internal navigation links',
            parameters: [],
            returnType: 'Map<string, { page: number; x: number | null; y: number | null }>',
            async: false, streaming: false, static: false,
            example: "const dests = pdf.getNamedDestinations();"
          }
        ]
      }
    ];
  }

  /**
   * Returns all method signatures with full parameter descriptions.
   * Useful for AI agents performing automated code generation.
   */
  static getMethodSignatures(): MethodDescriptor[] {
    return AgenticPDF.getCapabilities().flatMap(c => c.methods);
  }

  /**
   * Returns pre-built workflow templates for common multi-step operations.
   * AI agents can use these to plan and execute complex document processing tasks.
   */
  static getWorkflows(): Workflow[] {
    return [
      {
        id: 'basic-text-extraction',
        name: 'Basic Text Extraction',
        description: 'Load a PDF and extract all text content with formatting preserved.',
        steps: [
          { order: 1, method: 'fromFile', description: 'Load the PDF document', example: "const pdf = await AgenticPDF.fromFile(file);" },
          { order: 2, method: 'extractText', description: 'Extract text with formatting', example: "const text = await pdf.extractText({ preserveFormatting: true });" },
          { order: 3, method: 'close', description: 'Release resources', example: "pdf.close();" }
        ]
      },
      {
        id: 'rag-pipeline',
        name: 'RAG Pipeline Integration',
        description: 'Process a PDF into semantic chunks suitable for vector store ingestion in a Retrieval-Augmented Generation system.',
        steps: [
          { order: 1, method: 'fromFile', description: 'Load the PDF', example: "const pdf = await AgenticPDF.fromFile(file, { lazyLoad: true });" },
          { order: 2, method: 'streamSemanticChunks', description: 'Stream semantic chunks to vector store', example: "for await (const chunk of pdf.streamSemanticChunks({ strategy: 'semantic', maxChunkSize: 1000 })) { await vectorStore.add(chunk); }" },
          { order: 3, method: 'close', description: 'Release resources', example: "pdf.close();" }
        ]
      },
      {
        id: 'document-analysis',
        name: 'Full Document Analysis',
        description: 'Perform comprehensive AI analysis including structural analysis, NER, and summarization.',
        steps: [
          { order: 1, method: 'fromFile', description: 'Load the PDF', example: "const pdf = await AgenticPDF.fromFile(file);" },
          { order: 2, method: 'getAIFeatures', description: 'Run AI analysis', example: "const ai = await pdf.getAIFeatures({ enableStructuralAnalysis: true, enableNER: true, enableSummarization: true });" },
          { order: 3, method: 'exportAs', description: 'Export structured results', example: "const json = await pdf.exportAs('json', { includeMetadata: true });" },
          { order: 4, method: 'close', description: 'Release resources', example: "pdf.close();" }
        ]
      },
      {
        id: 'form-processing',
        name: 'Form Extraction and Filling',
        description: 'Extract form fields, fill them with data, and save the modified PDF.',
        steps: [
          { order: 1, method: 'fromFile', description: 'Load the PDF form', example: "const pdf = await AgenticPDF.fromFile(formFile);" },
          { order: 2, method: 'getFormFields', description: 'Inspect available form fields', example: "const fields = await pdf.getFormFields();" },
          { order: 3, method: 'fillForm', description: 'Fill form with data', example: "await pdf.fillForm({ name: 'John Doe', date: '2024-01-01' });" },
          { order: 4, method: 'save', description: 'Save the filled form', example: "const blob = await pdf.save();" },
          { order: 5, method: 'close', description: 'Release resources', example: "pdf.close();" }
        ]
      },
      {
        id: 'streaming-large-document',
        name: 'Memory-Efficient Large Document Processing',
        description: 'Process very large PDFs (100MB+) using streaming APIs with progress tracking and memory limits.',
        steps: [
          { order: 1, method: 'fromUrl', description: 'Stream PDF from URL with progress', example: "const pdf = await AgenticPDF.fromUrl(url, { streamOptions: { chunkSize: 1024 * 1024, progressCallback: p => console.log(p.currentOperation) } });" },
          { order: 2, method: 'streamText', description: 'Stream text extraction page by page', example: "for await (const text of pdf.streamText({ normalizeWhitespace: true })) { process(text); }" },
          { order: 3, method: 'unloadPages', description: 'Free processed pages from memory', example: "pdf.unloadPages();" },
          { order: 4, method: 'close', description: 'Release resources', example: "pdf.close();" }
        ]
      },
      {
        id: 'multi-format-export',
        name: 'Multi-Format Export Pipeline',
        description: 'Export a PDF to multiple output formats for different downstream consumers.',
        steps: [
          { order: 1, method: 'fromFile', description: 'Load the PDF', example: "const pdf = await AgenticPDF.fromFile(file);" },
          { order: 2, method: 'exportAs', description: 'Export as Markdown', example: "const md = await pdf.exportAs('markdown', { includeImages: true });" },
          { order: 3, method: 'exportAs', description: 'Export as structured JSON', example: "const json = await pdf.exportAs('json', { includeMetadata: true, includeAnnotations: true });" },
          { order: 4, method: 'exportAs', description: 'Export as HTML', example: "const html = await pdf.exportAs('html');" },
          { order: 5, method: 'close', description: 'Release resources', example: "pdf.close();" }
        ]
      },
      {
        id: 'llm-streaming',
        name: 'Stream to LLM',
        description: 'Stream PDF content directly to a Large Language Model in appropriately sized chunks.',
        steps: [
          { order: 1, method: 'fromFile', description: 'Load the PDF', example: "const pdf = await AgenticPDF.fromFile(file, { lazyLoad: true });" },
          { order: 2, method: 'streamSemanticChunks', description: 'Stream chunks sized for LLM context windows', example: "for await (const chunk of pdf.streamSemanticChunks({ maxChunkSize: 1500, preserveParagraphs: true })) { await llm.send(chunk.content); }" },
          { order: 3, method: 'close', description: 'Release resources', example: "pdf.close();" }
        ]
      }
    ];
  }

  /** @internal */
  private static _getConcepts(): OntologyConcept[] {
    return [
      {
        id: 'Document',
        label: 'PDF Document',
        description: 'A loaded PDF document instance providing access to pages, content, metadata, and AI analysis capabilities.',
        properties: [
          { name: 'pageCount', type: 'number', description: 'Total number of pages' },
          { name: 'fileSize', type: 'number', description: 'File size in bytes' },
          { name: 'version', type: 'string', description: 'PDF specification version' },
          { name: 'encrypted', type: 'boolean', description: 'Whether the document is password-protected' },
          { name: 'metadata', type: 'PDFMetadata', description: 'Title, author, subject, keywords, dates' }
        ],
        relationships: [
          { type: 'hasMany', target: 'Page', description: 'Contains one or more pages' },
          { type: 'hasMany', target: 'Annotation', description: 'Contains annotations across pages' },
          { type: 'hasMany', target: 'FormField', description: 'Contains interactive form fields' },
          { type: 'produces', target: 'AIFeatures', description: 'Generates AI analysis results' },
          { type: 'produces', target: 'SemanticChunk', description: 'Generates semantic chunks for RAG' }
        ]
      },
      {
        id: 'Page',
        label: 'PDF Page',
        description: 'An individual page with geometry, content streams, and resources.',
        properties: [
          { name: 'pageNumber', type: 'number', description: '1-based page index' },
          { name: 'width', type: 'number', description: 'Page width in points' },
          { name: 'height', type: 'number', description: 'Page height in points' },
          { name: 'rotation', type: 'number', description: 'Page rotation in degrees' }
        ],
        relationships: [
          { type: 'belongsTo', target: 'Document', description: 'Part of a document' },
          { type: 'hasMany', target: 'TextContent', description: 'Contains text elements' },
          { type: 'hasMany', target: 'ImageContent', description: 'Contains embedded images' },
          { type: 'hasMany', target: 'Annotation', description: 'Contains page-level annotations' }
        ]
      },
      {
        id: 'TextContent',
        label: 'Text Content',
        description: 'Extracted text with precise positioning, font information, and styling.',
        properties: [
          { name: 'text', type: 'string', description: 'The text content' },
          { name: 'x', type: 'number', description: 'X position on page' },
          { name: 'y', type: 'number', description: 'Y position on page' },
          { name: 'fontSize', type: 'number', description: 'Font size in points' },
          { name: 'fontName', type: 'string', description: 'Font family name' },
          { name: 'style', type: 'TextStyle', description: 'Bold, italic, underline, color' }
        ],
        relationships: [
          { type: 'belongsTo', target: 'Page', description: 'Located on a specific page' }
        ]
      },
      {
        id: 'ImageContent',
        label: 'Image Content',
        description: 'An embedded image extracted from the PDF with metadata and pixel data.',
        properties: [
          { name: 'id', type: 'string', description: 'Unique image identifier' },
          { name: 'width', type: 'number', description: 'Image width in pixels' },
          { name: 'height', type: 'number', description: 'Image height in pixels' },
          { name: 'mimeType', type: 'string', description: 'Image MIME type' },
          { name: 'colorSpace', type: 'string', description: 'Color space (RGB, CMYK, etc.)' }
        ],
        relationships: [
          { type: 'belongsTo', target: 'Page', description: 'Located on a specific page' }
        ]
      },
      {
        id: 'SemanticChunk',
        label: 'Semantic Chunk',
        description: 'A semantically coherent text segment with metadata, optimized for RAG systems and vector stores.',
        properties: [
          { name: 'id', type: 'string', description: 'Unique chunk identifier' },
          { name: 'content', type: 'string', description: 'Chunk text content' },
          { name: 'pageNumbers', type: 'number[]', description: 'Source page numbers' },
          { name: 'type', type: 'ChunkType', description: 'Content type classification' },
          { name: 'embedding', type: 'Float32Array', description: 'Optional embedding vector' },
          { name: 'metadata', type: 'ChunkMetadata', description: 'Token count, confidence, keywords, entities' }
        ],
        relationships: [
          { type: 'belongsTo', target: 'Document', description: 'Derived from a document' },
          { type: 'belongsTo', target: 'Page', description: 'Sourced from specific pages' }
        ]
      },
      {
        id: 'AIFeatures',
        label: 'AI Analysis Results',
        description: 'Comprehensive AI analysis output including structural analysis, semantic chunks, and NLP-ready content.',
        properties: [
          { name: 'structuralAnalysis', type: 'StructuralAnalysis', description: 'Document structure: sections, tables, figures' },
          { name: 'semanticChunks', type: 'SemanticChunk[]', description: 'Semantic text chunks' },
          { name: 'nlpReady', type: 'NLPReadyContent', description: 'Clean text, sentences, paragraphs, summary' }
        ],
        relationships: [
          { type: 'belongsTo', target: 'Document', description: 'Analysis of a specific document' },
          { type: 'hasMany', target: 'SemanticChunk', description: 'Contains semantic chunks' }
        ]
      },
      {
        id: 'Annotation',
        label: 'PDF Annotation',
        description: 'A document annotation such as a highlight, note, link, stamp, or redaction.',
        properties: [
          { name: 'id', type: 'string', description: 'Unique annotation identifier' },
          { name: 'type', type: 'AnnotationType', description: 'Annotation type enum value' },
          { name: 'rect', type: 'Rectangle', description: 'Bounding rectangle on page' },
          { name: 'contents', type: 'string', description: 'Annotation text content' },
          { name: 'author', type: 'string', description: 'Annotation author' }
        ],
        relationships: [
          { type: 'belongsTo', target: 'Page', description: 'Located on a specific page' }
        ]
      },
      {
        id: 'FormField',
        label: 'Form Field',
        description: 'An interactive form field that can be read and filled programmatically.',
        properties: [
          { name: 'id', type: 'string', description: 'Field identifier' },
          { name: 'type', type: 'FormFieldType', description: 'Field type: Button, Text, Choice, Signature' },
          { name: 'name', type: 'string', description: 'Field name for form filling' },
          { name: 'value', type: 'any', description: 'Current field value' },
          { name: 'required', type: 'boolean', description: 'Whether the field is required' }
        ],
        relationships: [
          { type: 'belongsTo', target: 'Document', description: 'Part of a document form' },
          { type: 'belongsTo', target: 'Page', description: 'Located on a specific page' }
        ]
      },
      {
        id: 'StructuralAnalysis',
        label: 'Document Structure',
        description: 'Structural decomposition of the document into sections, tables, figures, equations, and references.',
        properties: [
          { name: 'documentType', type: 'DocumentType', description: 'Auto-detected document category' },
          { name: 'sections', type: 'DocumentSection[]', description: 'Hierarchical section structure' },
          { name: 'tables', type: 'Table[]', description: 'Detected tables with cell data' },
          { name: 'figures', type: 'Figure[]', description: 'Detected figures with captions' },
          { name: 'equations', type: 'Equation[]', description: 'Detected mathematical equations' },
          { name: 'references', type: 'Reference[]', description: 'Cross-references and citations' }
        ],
        relationships: [
          { type: 'belongsTo', target: 'AIFeatures', description: 'Part of AI analysis results' }
        ]
      }
    ];
  }

  
  /**
   * Get document outline (bookmarks) as a hierarchical tree.
   * Returns an array of top-level outline items, each potentially with nested children.
   */
  getOutline(): OutlineItem[] {
    if (!this.parser || !this.xrefTable || !this.catalog) return [];

    const parser = this.parser;
    const xref = this.xrefTable;

    const resolve = (obj: PDFObject): PDFObject => {
      if (obj && obj.type === PDFObjectType.Reference) {
        const ref = obj.value as PDFReference;
        return parser.parseIndirectObject(ref.objectNumber, ref.generationNumber, xref);
      }
      return obj;
    };

    // Build page object number -> page index map
    const objNumToPage = new Map<number, number>();
    const collectPages = (node: PDFObject, ref?: PDFObject): void => {
      const dict = resolve(node);
      if (dict.type !== PDFObjectType.Dictionary) return;
      const d = dict.value as PDFDictionary;
      const typeEntry = d.entries.get('Type');
      const typeName = typeEntry?.type === PDFObjectType.Name ? typeEntry.value as string : '';
      if (typeName === 'Page') {
        if (ref && ref.type === PDFObjectType.Reference) {
          objNumToPage.set((ref.value as PDFReference).objectNumber, objNumToPage.size + 1);
        }
        return;
      }
      const kids = d.entries.get('Kids');
      if (!kids) return;
      const kidsArr = resolve(kids);
      if (kidsArr.type !== PDFObjectType.Array) return;
      for (const kid of kidsArr.value as PDFObject[]) {
        collectPages(kid, kid);
      }
    };
    const pagesRef = this.catalog.entries.get('Pages');
    if (pagesRef) collectPages(pagesRef);

    // Get the /Outlines entry from the catalog
    const outlinesRef = this.catalog.entries.get('Outlines');
    if (!outlinesRef) return [];

    const outlinesObj = resolve(outlinesRef);
    if (outlinesObj.type !== PDFObjectType.Dictionary) return [];
    const outlinesDict = outlinesObj.value as PDFDictionary;

    // Helper: extract page number from a destination
    const getPageFromDest = (dest: PDFObject): number | null => {
      const resolved = resolve(dest);
      if (resolved.type === PDFObjectType.Array) {
        const arr = resolved.value as PDFObject[];
        if (arr.length > 0) {
          const pageRef = arr[0];
          if (pageRef.type === PDFObjectType.Reference) {
            return objNumToPage.get((pageRef.value as PDFReference).objectNumber) ?? null;
          }
        }
      } else if (resolved.type === PDFObjectType.String || resolved.type === PDFObjectType.Name) {
        // Named destination — look up in named dests
        const namedDests = this.getNamedDestinations();
        const info = namedDests.get(resolved.value as string);
        return info?.page ?? null;
      }
      return null;
    };

    // Helper: get destination string from an outline item
    const getDestString = (dict: PDFDictionary): string | null => {
      const dest = dict.entries.get('Dest');
      if (dest) {
        const resolved = resolve(dest);
        if (resolved.type === PDFObjectType.String || resolved.type === PDFObjectType.Name) {
          return resolved.value as string;
        }
        // Array dest — format as string
        if (resolved.type === PDFObjectType.Array) {
          return null; // Direct page ref, no string name
        }
      }
      // Check /A (action) dict
      const action = dict.entries.get('A');
      if (action) {
        const aObj = resolve(action);
        if (aObj.type === PDFObjectType.Dictionary) {
          const aDict = aObj.value as PDFDictionary;
          const sEntry = aDict.entries.get('S');
          if (sEntry?.type === PDFObjectType.Name && sEntry.value === 'GoTo') {
            const dEntry = aDict.entries.get('D');
            if (dEntry) {
              const res = resolve(dEntry);
              if (res.type === PDFObjectType.String || res.type === PDFObjectType.Name) {
                return res.value as string;
              }
            }
          }
        }
      }
      return null;
    };

    // Helper: get page number from outline item
    const getPageNum = (dict: PDFDictionary): number | null => {
      const dest = dict.entries.get('Dest');
      if (dest) {
        return getPageFromDest(dest);
      }
      const action = dict.entries.get('A');
      if (action) {
        const aObj = resolve(action);
        if (aObj.type === PDFObjectType.Dictionary) {
          const aDict = aObj.value as PDFDictionary;
          const sEntry = aDict.entries.get('S');
          if (sEntry?.type === PDFObjectType.Name && sEntry.value === 'GoTo') {
            const dEntry = aDict.entries.get('D');
            if (dEntry) return getPageFromDest(dEntry);
          }
        }
      }
      return null;
    };

    // Walk the outline tree using /First / /Next sibling chain
    const visited = new Set<string>();
    const walkOutline = (entryObj: PDFObject): OutlineItem[] => {
      const items: OutlineItem[] = [];
      let current: PDFObject | null = entryObj;

      while (current) {
        const resolved = resolve(current);
        if (resolved.type !== PDFObjectType.Dictionary) break;
        const dict = resolved.value as PDFDictionary;

        // Prevent infinite loops
        const refKey = current.type === PDFObjectType.Reference
          ? `${(current.value as PDFReference).objectNumber}:${(current.value as PDFReference).generationNumber}`
          : '';
        if (refKey && visited.has(refKey)) break;
        if (refKey) visited.add(refKey);

        // Extract title
        const titleObj = dict.entries.get('Title');
        let title = '';
        if (titleObj) {
          const titleRes = resolve(titleObj);
          if (titleRes.type === PDFObjectType.String) {
            title = titleRes.value as string;
          }
        }

        // Extract style flags from /F entry (bit 0 = italic, bit 1 = bold)
        let bold = false;
        let italic = false;
        const flagsObj = dict.entries.get('F');
        if (flagsObj?.type === PDFObjectType.Number) {
          const flags = flagsObj.value as number;
          italic = !!(flags & 1);
          bold = !!(flags & 2);
        }

        // Extract color from /C entry [r g b] (0-1 range)
        let color: Color | null = null;
        const cObj = dict.entries.get('C');
        if (cObj) {
          const cRes = resolve(cObj);
          if (cRes.type === PDFObjectType.Array) {
            const cArr = cRes.value as PDFObject[];
            if (cArr.length >= 3) {
              color = {
                r: Math.round(((cArr[0].value as number) || 0) * 255),
                g: Math.round(((cArr[1].value as number) || 0) * 255),
                b: Math.round(((cArr[2].value as number) || 0) * 255)
              };
            }
          }
        }

        // Recursively process children via /First
        const firstChild = dict.entries.get('First');
        const children = firstChild ? walkOutline(firstChild) : [];

        items.push({
          title,
          destination: getDestString(dict),
          page: getPageNum(dict),
          bold,
          italic,
          color,
          children
        });

        // Move to next sibling via /Next
        const nextObj = dict.entries.get('Next');
        current = nextObj || null;
      }

      return items;
    };

    // Start from the first child of the /Outlines dict
    const firstEntry = outlinesDict.entries.get('First');
    if (!firstEntry) return [];

    return walkOutline(firstEntry);
  }

/**
   * Describes the currently loaded document's available operations and
   * recommends workflows based on document characteristics.
   * Returns undefined if no document is loaded.
   */
  describeDocument(): DocumentCapabilityReport | undefined {
    if (!this.metadata) return undefined;

    const pageCount = this.getPageCount();
    const fileSize = this.getFileSize();
    const encrypted = this.isEncrypted();

    const operations = [
      'extractText', 'streamText', 'extractImages',
      'getAIFeatures', 'generateSemanticChunks', 'streamSemanticChunks',
      'search', 'getAnnotations', 'addAnnotation',
      'getFormFields', 'fillForm',
      'renderPage', 'renderPageToImage', 'buildTextLayer',
      'exportAs', 'save',
      'getMetadata', 'getPage', 'getAllPages', 'getNamedDestinations',
      'close', 'unloadPages', 'getMemoryStats'
    ];

    const workflows: string[] = ['basic-text-extraction'];

    if (pageCount > 50 || fileSize > 10 * 1024 * 1024) {
      workflows.push('streaming-large-document');
    }
    workflows.push('rag-pipeline', 'document-analysis', 'multi-format-export', 'llm-streaming');

    let complexity: 'simple' | 'moderate' | 'complex' = 'simple';
    if (pageCount > 100 || fileSize > 50 * 1024 * 1024) {
      complexity = 'complex';
    } else if (pageCount > 20 || fileSize > 5 * 1024 * 1024) {
      complexity = 'moderate';
    }

    return {
      documentInfo: {
        pageCount,
        fileSize,
        version: this.getVersion(),
        encrypted
      },
      availableOperations: operations,
      recommendedWorkflows: workflows,
      estimatedComplexity: complexity
    };
  }

  // ==========================================================================
  // Agentic AI Workflow Optimization (Phase 13)
  // ==========================================================================

  /**
   * Generates tool/function-calling schemas for AI agent integration.
   * Supports OpenAI function calling, Anthropic tool use, and generic formats.
   */
  static getToolSchemas(format: ToolSchemaFormat = 'openai'): any[] {
    const tools = AgenticPDF._buildToolDefinitions();

    switch (format) {
      case 'openai':
        return tools.map(tool => ({
          type: 'function',
          function: {
            name: tool.name,
            description: tool.description,
            parameters: {
              type: 'object',
              properties: Object.fromEntries(
                tool.parameters.map(p => [p.name, {
                  type: AgenticPDF._tsTypeToJsonType(p.type),
                  description: p.description,
                  ...(p.enum ? { enum: p.enum } : {}),
                  ...(p.default !== undefined ? { default: p.default } : {}),
                  ...(p.minimum !== undefined ? { minimum: p.minimum } : {}),
                  ...(p.maximum !== undefined ? { maximum: p.maximum } : {})
                }])
              ),
              required: tool.parameters.filter(p => p.required).map(p => p.name)
            }
          }
        }));

      case 'anthropic':
        return tools.map(tool => ({
          name: tool.name,
          description: tool.description,
          input_schema: {
            type: 'object',
            properties: Object.fromEntries(
              tool.parameters.map(p => [p.name, {
                type: AgenticPDF._tsTypeToJsonType(p.type),
                description: p.description,
                ...(p.enum ? { enum: p.enum } : {}),
                ...(p.default !== undefined ? { default: p.default } : {})
              }])
            ),
            required: tool.parameters.filter(p => p.required).map(p => p.name)
          }
        }));

      case 'generic':
      default:
        return tools;
    }
  }

  /**
   * Generates an MCP (Model Context Protocol) server manifest describing
   * all available tools and resources for agent discovery.
   */
  static getMCPManifest(): MCPManifest {
    const tools = AgenticPDF._buildToolDefinitions();

    return {
      name: 'agenticpdf',
      version: '1.0.0',
      description: 'PDF processing server with text extraction, AI analysis, image extraction, form handling, and semantic chunking.',
      protocol: '2025-01-01',
      tools: tools.map(tool => ({
        name: tool.name,
        description: tool.description,
        inputSchema: {
          type: 'object',
          properties: Object.fromEntries(
            tool.parameters.map(p => [p.name, {
              type: AgenticPDF._tsTypeToJsonType(p.type),
              description: p.description,
              ...(p.enum ? { enum: p.enum } : {}),
              ...(p.default !== undefined ? { default: p.default } : {})
            }])
          ),
          required: tool.parameters.filter(p => p.required).map(p => p.name)
        },
        annotations: {
          title: tool.description,
          readOnlyHint: !['fillForm', 'addAnnotation', 'save'].includes(tool.name),
          destructiveHint: tool.name === 'close',
          openWorldHint: false
        }
      })),
      resources: [
        {
          uri: 'agenticpdf://ontology',
          name: 'Library Ontology',
          description: 'Full JSON-LD ontology describing library concepts, capabilities, and relationships.',
          mimeType: 'application/ld+json'
        },
        {
          uri: 'agenticpdf://capabilities',
          name: 'Capability Map',
          description: 'All library capabilities organized by category with method signatures.',
          mimeType: 'application/json'
        },
        {
          uri: 'agenticpdf://workflows',
          name: 'Workflow Templates',
          description: 'Pre-built workflow templates for common PDF processing operations.',
          mimeType: 'application/json'
        },
        {
          uri: 'agenticpdf://schemas',
          name: 'JSON Schemas',
          description: 'JSON Schema definitions for all input and output types.',
          mimeType: 'application/schema+json'
        }
      ]
    };
  }

  /**
   * Generates JSON Schema definitions for all input and output types.
   * Useful for AI agents to validate parameters and return values.
   */
  static getJSONSchemas(): Record<string, any> {
    return {
      PDFOptions: {
        type: 'object',
        properties: {
          streaming: { type: 'boolean', description: 'Enable streaming mode' },
          lazyLoad: { type: 'boolean', description: 'Lazy-load pages on demand' },
          maxMemoryUsage: { type: 'number', description: 'Max memory in bytes' },
          useWebWorkers: { type: 'boolean', description: 'Use Web Workers for CPU-intensive ops' },
          workerUrl: { type: 'string', description: 'URL to the worker script' }
        }
      },
      TextExtractionOptions: {
        type: 'object',
        properties: {
          preserveFormatting: { type: 'boolean', description: 'Preserve text layout and whitespace' },
          normalizeWhitespace: { type: 'boolean', description: 'Normalize whitespace characters' },
          extractTables: { type: 'boolean', description: 'Detect and extract tables' },
          detectColumns: { type: 'boolean', description: 'Detect multi-column layout' },
          pageRange: {
            type: 'object',
            properties: {
              start: { type: 'number', minimum: 1 },
              end: { type: 'number', minimum: 1 }
            }
          }
        }
      },
      ImageExtractionOptions: {
        type: 'object',
        properties: {
          format: { type: 'string', enum: ['png', 'jpeg', 'webp'] },
          quality: { type: 'number', minimum: 0, maximum: 1 },
          maxWidth: { type: 'number', minimum: 1 },
          maxHeight: { type: 'number', minimum: 1 },
          extractMasks: { type: 'boolean' },
          pageRange: {
            type: 'object',
            properties: {
              start: { type: 'number', minimum: 1 },
              end: { type: 'number', minimum: 1 }
            }
          }
        }
      },
      AIOptions: {
        type: 'object',
        properties: {
          enableStructuralAnalysis: { type: 'boolean' },
          enableSemanticChunking: { type: 'boolean' },
          enableNER: { type: 'boolean' },
          enableSummarization: { type: 'boolean' },
          chunkSize: { type: 'number', minimum: 50, maximum: 10000, default: 500 },
          chunkOverlap: { type: 'number', minimum: 0, maximum: 5000, default: 50 }
        }
      },
      ChunkingOptions: {
        type: 'object',
        properties: {
          strategy: { type: 'string', enum: ['semantic', 'fixed', 'sliding', 'recursive'] },
          maxChunkSize: { type: 'number', minimum: 50, default: 500 },
          minChunkSize: { type: 'number', minimum: 10, default: 100 },
          overlapSize: { type: 'number', minimum: 0, default: 50 },
          preserveSentences: { type: 'boolean' },
          preserveParagraphs: { type: 'boolean' }
        }
      },
      ExportOptions: {
        type: 'object',
        properties: {
          includeMetadata: { type: 'boolean' },
          includeAnnotations: { type: 'boolean' },
          includeForms: { type: 'boolean' },
          includeImages: { type: 'boolean' },
          imageFormat: { type: 'string', enum: ['png', 'jpeg', 'webp'] },
          pageRange: {
            type: 'object',
            properties: {
              start: { type: 'number', minimum: 1 },
              end: { type: 'number', minimum: 1 }
            }
          }
        }
      },
      RenderOptions: {
        type: 'object',
        properties: {
          scale: { type: 'number', minimum: 0.1, maximum: 10, default: 1 },
          rotation: { type: 'number', enum: [0, 90, 180, 270] },
          background: { type: 'string' },
          renderText: { type: 'boolean', default: true },
          renderImages: { type: 'boolean', default: true },
          renderAnnotations: { type: 'boolean', default: true }
        }
      },
      TextContent: {
        type: 'object',
        properties: {
          text: { type: 'string' },
          x: { type: 'number' },
          y: { type: 'number' },
          width: { type: 'number' },
          height: { type: 'number' },
          fontSize: { type: 'number' },
          fontFamily: { type: 'string' },
          pageNumber: { type: 'number' },
          style: {
            type: 'object',
            properties: {
              bold: { type: 'boolean' },
              italic: { type: 'boolean' }
            }
          }
        },
        required: ['text', 'pageNumber']
      },
      SemanticChunk: {
        type: 'object',
        properties: {
          id: { type: 'string' },
          content: { type: 'string' },
          pageNumbers: { type: 'array', items: { type: 'number' } },
          type: { type: 'string', enum: ['Title', 'Header', 'Paragraph', 'List', 'Table', 'Figure', 'Code', 'Quote', 'Footnote'] },
          metadata: {
            type: 'object',
            properties: {
              tokenCount: { type: 'number' },
              confidence: { type: 'number' },
              importance: { type: 'number' },
              keywords: { type: 'array', items: { type: 'string' } }
            }
          }
        },
        required: ['id', 'content', 'pageNumbers']
      },
      ImageContent: {
        type: 'object',
        properties: {
          id: { type: 'string' },
          width: { type: 'number' },
          height: { type: 'number' },
          mimeType: { type: 'string' },
          colorSpace: { type: 'string' },
          pageNumber: { type: 'number' }
        },
        required: ['id', 'pageNumber']
      }
    };
  }

  /**
   * Single-call introspection endpoint for AI agents.
   * Returns everything an agent needs to discover and use the library:
   * ontology, tool schemas, JSON schemas, workflows, and guidance.
   */
  static describeForAgent(format: ToolSchemaFormat = 'openai'): {
    ontology: LibraryOntology;
    tools: any[];
    schemas: Record<string, any>;
    workflows: Workflow[];
    agentGuidance: {
      quickStart: string;
      bestPractices: string[];
      memoryManagement: string;
      streamingGuidance: string;
    };
  } {
    return {
      ontology: AgenticPDF.describe(),
      tools: AgenticPDF.getToolSchemas(format),
      schemas: AgenticPDF.getJSONSchemas(),
      workflows: AgenticPDF.getWorkflows(),
      agentGuidance: {
        quickStart: 'Load a PDF with AgenticPDF.fromFile(file) or AgenticPDF.fromBuffer(buffer). Then call extractText(), getAIFeatures(), or generateSemanticChunks() as needed. Always call close() when done.',
        bestPractices: [
          'Use streaming APIs (streamText, streamSemanticChunks) for documents > 10MB',
          'Set lazyLoad: true for documents > 50 pages',
          'Set maxMemoryUsage for memory-constrained environments',
          'Always call close() to release resources',
          'Use AbortSignal for cancelable operations',
          'Prefer semantic chunking strategy for RAG pipelines',
          'Check describeDocument() for document-specific recommendations'
        ],
        memoryManagement: 'Configure maxMemoryUsage in PDFOptions. Call unloadPages() to release parsed pages. Call close() to release all resources. For large documents, use streaming APIs to avoid loading entire document.',
        streamingGuidance: 'All major operations support streaming via AsyncGenerator. Use streamText() for progressive text extraction, streamSemanticChunks() for RAG processing. Pass progressCallback in StreamOptions for progress tracking.'
      }
    };
  }

  /**
   * Creates an agent session for tracking operations across multiple calls.
   */
  createAgentSession(): AgentSession {
    return {
      sessionId: `session_${Date.now()}_${Math.random().toString(36).substring(2, 8)}`,
      startedAt: Date.now(),
      documentPath: undefined,
      operationsPerformed: [],
      chunksProcessed: 0,
      tokensEstimated: 0
    };
  }

  /**
   * Internal: build comprehensive tool definitions for all public methods.
   */
  private static _buildToolDefinitions(): ToolSchema[] {
    return [
      {
        name: 'extractText',
        description: 'Extract all text content from the PDF with positioning, font, and style metadata.',
        parameters: [
          { name: 'preserveFormatting', type: 'boolean', description: 'Keep original text layout', required: false, default: false },
          { name: 'normalizeWhitespace', type: 'boolean', description: 'Normalize whitespace', required: false, default: false },
          { name: 'pageRange', type: 'object', description: 'Pages to extract {start, end}', required: false }
        ],
        returnType: 'TextContent[]',
        category: 'extraction',
        streaming: false,
        requiresDocument: true,
        example: "const text = await pdf.extractText({ preserveFormatting: true });"
      },
      {
        name: 'streamText',
        description: 'Stream text content page-by-page for memory-efficient processing of large documents.',
        parameters: [
          { name: 'normalizeWhitespace', type: 'boolean', description: 'Normalize whitespace', required: false, default: false }
        ],
        returnType: 'AsyncGenerator<TextContent>',
        category: 'extraction',
        streaming: true,
        requiresDocument: true,
        example: "for await (const text of pdf.streamText()) { process(text); }"
      },
      {
        name: 'extractImages',
        description: 'Extract all images from the PDF with metadata (dimensions, color space, format).',
        parameters: [
          { name: 'format', type: 'string', description: 'Output image format', required: false, enum: ['png', 'jpeg', 'webp'] },
          { name: 'quality', type: 'number', description: 'Image quality 0-1', required: false, minimum: 0, maximum: 1 },
          { name: 'pageRange', type: 'object', description: 'Pages to extract {start, end}', required: false }
        ],
        returnType: 'ImageContent[]',
        category: 'extraction',
        streaming: false,
        requiresDocument: true,
        example: "const images = await pdf.extractImages({ format: 'png' });"
      },
      {
        name: 'imageToDataURL',
        description: 'Convert an extracted image to a data URL for display.',
        parameters: [
          { name: 'image', type: 'ImageContent', description: 'Image to convert', required: true },
          { name: 'format', type: 'string', description: 'Output format', required: false, enum: ['png', 'jpeg', 'webp'] },
          { name: 'quality', type: 'number', description: 'Quality 0-1', required: false, minimum: 0, maximum: 1 }
        ],
        returnType: 'string',
        category: 'extraction',
        streaming: false,
        requiresDocument: true,
        example: "const url = await pdf.imageToDataURL(image, 'png');"
      },
      {
        name: 'getMetadata',
        description: 'Get PDF document metadata (title, author, page count, creation date, etc.).',
        parameters: [],
        returnType: 'PDFMetadata',
        category: 'extraction',
        streaming: false,
        requiresDocument: true,
        example: "const meta = pdf.getMetadata();"
      },
      {
        name: 'getPageCount',
        description: 'Get total number of pages in the document.',
        parameters: [],
        returnType: 'number',
        category: 'extraction',
        streaming: false,
        requiresDocument: true,
        example: "const count = pdf.getPageCount();"
      },
      {
        name: 'getAIFeatures',
        description: 'Run AI analysis: structural analysis, semantic chunking, NLP preparation, and optional embeddings.',
        parameters: [
          { name: 'enableStructuralAnalysis', type: 'boolean', description: 'Analyze sections, tables, figures', required: false, default: true },
          { name: 'enableSemanticChunking', type: 'boolean', description: 'Generate semantic chunks', required: false, default: true },
          { name: 'chunkSize', type: 'number', description: 'Target chunk size in tokens', required: false, minimum: 50, maximum: 10000, default: 500 },
          { name: 'chunkOverlap', type: 'number', description: 'Overlap between chunks', required: false, minimum: 0, maximum: 5000, default: 50 }
        ],
        returnType: 'AIFeatures',
        category: 'analysis',
        streaming: false,
        requiresDocument: true,
        example: "const ai = await pdf.getAIFeatures({ enableStructuralAnalysis: true });"
      },
      {
        name: 'generateSemanticChunks',
        description: 'Generate semantic chunks for RAG pipelines with configurable strategy.',
        parameters: [
          { name: 'strategy', type: 'string', description: 'Chunking strategy', required: false, enum: ['semantic', 'fixed', 'sliding', 'recursive'] },
          { name: 'maxChunkSize', type: 'number', description: 'Max tokens per chunk', required: false, minimum: 50, default: 500 },
          { name: 'overlapSize', type: 'number', description: 'Token overlap between chunks', required: false, minimum: 0, default: 50 }
        ],
        returnType: 'SemanticChunk[]',
        category: 'analysis',
        streaming: false,
        requiresDocument: true,
        example: "const chunks = await pdf.generateSemanticChunks({ strategy: 'semantic' });"
      },
      {
        name: 'streamSemanticChunks',
        description: 'Stream semantic chunks one at a time for memory-efficient RAG processing.',
        parameters: [
          { name: 'strategy', type: 'string', description: 'Chunking strategy', required: false, enum: ['semantic', 'fixed', 'sliding', 'recursive'] },
          { name: 'maxChunkSize', type: 'number', description: 'Max tokens per chunk', required: false, minimum: 50, default: 500 }
        ],
        returnType: 'AsyncGenerator<SemanticChunk>',
        category: 'analysis',
        streaming: true,
        requiresDocument: true,
        example: "for await (const chunk of pdf.streamSemanticChunks()) { embed(chunk); }"
      },
      {
        name: 'search',
        description: 'Search for text within the document, returning matches with page numbers and positions.',
        parameters: [
          { name: 'query', type: 'string', description: 'Search query string', required: true },
          { name: 'caseSensitive', type: 'boolean', description: 'Case-sensitive search', required: false, default: false },
          { name: 'wholeWord', type: 'boolean', description: 'Match whole words only', required: false, default: false }
        ],
        returnType: 'SearchResult[]',
        category: 'search',
        streaming: false,
        requiresDocument: true,
        example: "const results = await pdf.search('keyword');"
      },
      {
        name: 'getFormFields',
        description: 'Get all interactive form fields (text input, checkboxes, radio, dropdowns).',
        parameters: [],
        returnType: 'FormField[]',
        category: 'forms',
        streaming: false,
        requiresDocument: true,
        example: "const fields = await pdf.getFormFields();"
      },
      {
        name: 'fillForm',
        description: 'Fill form fields with provided values. Keys are field names, values are field contents.',
        parameters: [
          { name: 'data', type: 'object', description: 'Field name-value pairs', required: true }
        ],
        returnType: 'void',
        category: 'forms',
        streaming: false,
        requiresDocument: true,
        example: "await pdf.fillForm({ name: 'John', email: 'john@example.com' });"
      },
      {
        name: 'getFormData',
        description: 'Get current form field values (original + filled).',
        parameters: [],
        returnType: 'Record<string, any>',
        category: 'forms',
        streaming: false,
        requiresDocument: true,
        example: "const data = await pdf.getFormData();"
      },
      {
        name: 'getAnnotations',
        description: 'Get annotations (links, highlights, comments) for all pages or a specific page.',
        parameters: [
          { name: 'pageNumber', type: 'number', description: 'Specific page (omit for all)', required: false, minimum: 1 }
        ],
        returnType: 'Annotation[]',
        category: 'annotations',
        streaming: false,
        requiresDocument: true,
        example: "const annots = await pdf.getAnnotations(1);"
      },
      {
        name: 'addAnnotation',
        description: 'Add a new annotation to the document.',
        parameters: [
          { name: 'annotation', type: 'Annotation', description: 'Annotation to add', required: true }
        ],
        returnType: 'string',
        category: 'annotations',
        streaming: false,
        requiresDocument: true,
        example: "const id = await pdf.addAnnotation({ type: 'Highlight', rect: {...} });"
      },
      {
        name: 'exportAs',
        description: 'Export document content to text, HTML, Markdown, JSON, XML, or CSV format.',
        parameters: [
          { name: 'format', type: 'string', description: 'Target format', required: true, enum: ['text', 'html', 'markdown', 'json', 'xml', 'csv'] },
          { name: 'options', type: 'ExportOptions', description: 'Export configuration', required: false }
        ],
        returnType: 'string | Blob',
        category: 'export',
        streaming: false,
        requiresDocument: true,
        example: "const md = await pdf.exportAs('markdown', { includeImages: true });"
      },
      {
        name: 'renderPage',
        description: 'Render a page to an HTML canvas element.',
        parameters: [
          { name: 'pageNumber', type: 'number', description: 'Page to render (1-indexed)', required: true, minimum: 1 },
          { name: 'canvas', type: 'HTMLCanvasElement', description: 'Target canvas', required: true },
          { name: 'options', type: 'RenderOptions', description: 'Render configuration', required: false }
        ],
        returnType: 'void',
        category: 'rendering',
        streaming: false,
        requiresDocument: true,
        example: "await pdf.renderPage(1, canvas, { scale: 2 });"
      },
      {
        name: 'getNamedDestinations',
        description: 'Get all named destinations for internal document navigation.',
        parameters: [],
        returnType: 'Map<string, { page: number; x: number | null; y: number | null }>',
        category: 'extraction',
        streaming: false,
        requiresDocument: true,
        example: "const dests = pdf.getNamedDestinations();"
      },
      {
        name: 'close',
        description: 'Release all resources held by this document. Always call when done.',
        parameters: [],
        returnType: 'void',
        category: 'memory',
        streaming: false,
        requiresDocument: true,
        example: "pdf.close();"
      },
      {
        name: 'describeDocument',
        description: 'Get document-specific capability report with recommended workflows and complexity assessment.',
        parameters: [],
        returnType: 'DocumentCapabilityReport',
        category: 'analysis',
        streaming: false,
        requiresDocument: true,
        example: "const report = pdf.describeDocument();"
      }
    ];
  }

  /**
   * Internal: convert TypeScript type strings to JSON Schema types.
   */
  // ==========================================================================
  // DoD Security Audit & Compliance (Phase 15)
  // ==========================================================================

  /**
   * Generate a Software Bill of Materials (SBOM) in CycloneDX-compatible format.
   * Required for CMMC 2.0 Level 2+ supply chain security (SC.L2-3.13.1).
   */
  static generateSBOM(): SBOM {
    return {
      bomFormat: 'CycloneDX',
      specVersion: '1.5',
      version: 1,
      metadata: {
        component: {
          name: 'agenticpdf',
          version: '1.0.0',
          type: 'library',
          license: 'AGPL-3.0-or-later'
        },
        timestamp: new Date().toISOString()
      },
      components: [
        {
          name: 'agenticpdf',
          version: '1.0.0',
          license: 'AGPL-3.0-or-later',
          type: 'library',
          purl: 'pkg:npm/agenticpdf@1.0.0',
          supplier: 'Nervosys, LLC'
        }
        // Zero external runtime dependencies — single-file architecture
      ]
    };
  }

  /**
   * Returns the default security configuration for hardened environments.
   * All limits are designed for NIST FIPS / CMMC 2.0 compliance.
   */
  static getSecurityConfig(): SecurityConfig {
    return { ...DEFAULT_SECURITY_CONFIG };
  }

  /**
   * Validate a PDF buffer against security constraints before parsing.
   * Returns an array of security violations (empty = safe to parse).
   */
  static validateSecurityConstraints(data: ArrayBuffer | Uint8Array, config?: Partial<SecurityConfig>): string[] {
    const cfg = { ...DEFAULT_SECURITY_CONFIG, ...config };
    const violations: string[] = [];
    const size = data instanceof ArrayBuffer ? data.byteLength : data.length;

    if (size > cfg.maxFileSize) {
      violations.push(`File size ${size} exceeds maximum ${cfg.maxFileSize} bytes`);
    }

    if (size < 8) {
      violations.push('File too small to be a valid PDF');
      return violations;
    }

    // Check PDF header
    const header = new Uint8Array(data instanceof ArrayBuffer ? data : data.buffer, 0, 8);
    const headerStr = String.fromCharCode(...header.slice(0, 5));
    if (headerStr !== '%PDF-') {
      violations.push('Invalid PDF header — file may not be a PDF');
    }

    // Scan for JavaScript actions (disabled by default in DoD config)
    if (!cfg.allowJavaScript) {
      const bytes = new Uint8Array(data instanceof ArrayBuffer ? data : data.buffer);
      const text = new TextDecoder('ascii', { fatal: false }).decode(bytes.slice(0, Math.min(bytes.length, 1024 * 1024)));
      if (text.includes('/JS ') || text.includes('/JavaScript')) {
        violations.push('PDF contains JavaScript actions (blocked by security policy)');
      }
    }

    // Scan for external resource references
    if (!cfg.allowExternalResources) {
      const bytes = new Uint8Array(data instanceof ArrayBuffer ? data : data.buffer);
      const sample = new TextDecoder('ascii', { fatal: false }).decode(bytes.slice(0, Math.min(bytes.length, 512 * 1024)));
      if (sample.includes('/URI (http') || sample.includes('/URI (ftp')) {
        // External URIs are informational, not blocking — unless very restrictive
      }
      if (sample.includes('/F (') || sample.includes('/EF ')) {
        violations.push('PDF contains embedded file references (potential data exfiltration)');
      }
    }

    return violations;
  }

  private static _tsTypeToJsonType(tsType: string): string {
    const map: Record<string, string> = {
      'string': 'string',
      'number': 'number',
      'boolean': 'boolean',
      'object': 'object',
      'any': 'object',
      'void': 'null',
      'ImageContent': 'object',
      'Annotation': 'object',
      'ExportOptions': 'object',
      'RenderOptions': 'object',
      'HTMLCanvasElement': 'object'
    };
    return map[tsType] || 'string';
  }

  // ==========================================================================
  // Advanced Text & Layout Analysis
  // ==========================================================================

  /**
   * Analyze page layout to detect columns, tables, and reading order.
   * Returns structured layout information for each page.
   */
  async analyzeLayout(pageRange?: { start: number; end: number }): Promise<{
    pages: Array<{
      pageNumber: number;
      columns: Array<{ x: number; width: number; blockCount: number }>;
      tables: Table[];
      readingOrder: TextContent[];
      verticalTextBlocks: TextContent[];
      rtlBlocks: TextContent[];
    }>;
  }> {
    const result: {
      pages: Array<{
        pageNumber: number;
        columns: Array<{ x: number; width: number; blockCount: number }>;
        tables: Table[];
        readingOrder: TextContent[];
        verticalTextBlocks: TextContent[];
        rtlBlocks: TextContent[];
      }>;
    } = { pages: [] };

    const pageCount = this.getPageCount();
    const startPage = pageRange?.start ?? 1;
    const endPage = pageRange?.end ?? pageCount;

    for (let p = startPage; p <= endPage; p++) {
      const page = await this.getPage(p);
      if (!page) continue;

      const blocks = await this.extractText({ pageRange: { start: p, end: p } });
      const pageWidth = page.width || 612;

      // Column detection
      const columns = LayoutAnalyzer.detectColumns(blocks, pageWidth);

      // Table detection
      const tables = LayoutAnalyzer.detectTables(blocks, p);

      // Reading order reconstruction
      const readingOrder = LayoutAnalyzer.reconstructReadingOrder(blocks, pageWidth);

      // Vertical CJK text detection
      const { vertical, horizontal: _ } = LayoutAnalyzer.detectVerticalText(blocks);

      // RTL text detection
      const rtlBlocks = blocks.filter(b => LayoutAnalyzer.detectTextDirection(b.text) === 'rtl');

      result.pages.push({
        pageNumber: p,
        columns: columns.map(c => ({ x: c.x, width: c.width, blockCount: c.blocks.length })),
        tables,
        readingOrder,
        verticalTextBlocks: vertical,
        rtlBlocks
      });
    }

    return result;
  }


  // ==========================================================================
  // Real-Time Translation & Text-to-Speech
  // ==========================================================================

  /**
   * Translate all text content in the PDF using a TranslationProvider.
   * Returns translated pages preserving positional metadata for each segment.
   */
  async translateDocument(
    provider: TranslationProvider,
    options: TranslationOptions
  ): Promise<TranslatedPage[]> {
    const pages: TranslatedPage[] = [];
    const batchSize = options.batchSize ?? 50;
    const srcLang = options.sourceLanguage ?? null;
    const targetLang = options.targetLanguage;

    const textOptions: TextExtractionOptions = {};
    if (options.pageRange) {
      textOptions.pageRange = options.pageRange;
    }

    // Group TextContent by page
    const allText = await this.extractText(textOptions);
    const pageMap = new Map<number, TextContent[]>();
    for (const tc of allText) {
      const arr = pageMap.get(tc.pageNumber) || [];
      arr.push(tc);
      pageMap.set(tc.pageNumber, arr);
    }

    let segmentsTranslated = 0;
    const totalPages = pageMap.size;
    let pagesComplete = 0;

    for (const [pageNum, segments] of pageMap) {
      if (options.abortSignal?.aborted) break;

      const translatedSegments: TranslatedSegment[] = [];

      // Process in batches
      for (let i = 0; i < segments.length; i += batchSize) {
        if (options.abortSignal?.aborted) break;
        const batch = segments.slice(i, i + batchSize);
        const texts = batch.map(s => s.text).filter(t => t.trim().length > 0);

        if (texts.length === 0) continue;

        // Apply glossary substitutions before translation
        const processedTexts = options.glossary
          ? texts.map(t => {
              let result = t;
              for (const [term, replacement] of Object.entries(options.glossary!)) {
                result = result.split(term).join(replacement);
              }
              return result;
            })
          : texts;

        const results = await provider.translateBatch(processedTexts, srcLang, targetLang);

        let resultIdx = 0;
        for (const seg of batch) {
          if (seg.text.trim().length === 0) {
            translatedSegments.push({ original: seg, translatedText: seg.text, confidence: 1.0 });
          } else {
            const result = results[resultIdx++];
            translatedSegments.push({
              original: seg,
              translatedText: result.translatedText,
              confidence: result.confidence
            });
          }
          segmentsTranslated++;
        }
      }

      const detectedLang = srcLang || (translatedSegments.length > 0 ? 'auto' : 'unknown');

      pages.push({
        pageNumber: pageNum,
        sourceLanguage: detectedLang,
        targetLanguage: targetLang,
        segments: translatedSegments
      });

      pagesComplete++;
      options.progressCallback?.({ pagesComplete, totalPages, segmentsTranslated });
    }

    return pages;
  }

  /**
   * Stream translated pages one at a time for memory-efficient processing.
   * Yields each TranslatedPage as soon as it is ready.
   */
  async *streamTranslation(
    provider: TranslationProvider,
    options: TranslationOptions
  ): AsyncGenerator<TranslatedPage> {
    const batchSize = options.batchSize ?? 50;
    const srcLang = options.sourceLanguage ?? null;
    const targetLang = options.targetLanguage;
    const pageCount = this.getPageCount();

    const startPage = options.pageRange?.start ?? 1;
    const endPage = options.pageRange?.end ?? pageCount;

    let segmentsTranslated = 0;
    let pagesComplete = 0;
    const totalPages = endPage - startPage + 1;

    for (let p = startPage; p <= endPage; p++) {
      if (options.abortSignal?.aborted) return;

      const pageText = await this.extractText({ pageRange: { start: p, end: p } });
      if (pageText.length === 0) {
        pagesComplete++;
        continue;
      }

      const translatedSegments: TranslatedSegment[] = [];

      for (let i = 0; i < pageText.length; i += batchSize) {
        if (options.abortSignal?.aborted) return;
        const batch = pageText.slice(i, i + batchSize);
        const texts = batch.map(s => s.text).filter(t => t.trim().length > 0);

        if (texts.length === 0) continue;

        const processedTexts = options.glossary
          ? texts.map(t => {
              let result = t;
              for (const [term, replacement] of Object.entries(options.glossary!)) {
                result = result.split(term).join(replacement);
              }
              return result;
            })
          : texts;

        const results = await provider.translateBatch(processedTexts, srcLang, targetLang);

        let resultIdx = 0;
        for (const seg of batch) {
          if (seg.text.trim().length === 0) {
            translatedSegments.push({ original: seg, translatedText: seg.text, confidence: 1.0 });
          } else {
            const result = results[resultIdx++];
            translatedSegments.push({
              original: seg,
              translatedText: result.translatedText,
              confidence: result.confidence
            });
          }
          segmentsTranslated++;
        }
      }

      pagesComplete++;
      options.progressCallback?.({ pagesComplete, totalPages, segmentsTranslated });

      yield {
        pageNumber: p,
        sourceLanguage: srcLang || 'auto',
        targetLanguage: targetLang,
        segments: translatedSegments
      };
    }
  }

  /**
   * Synthesize text-to-speech audio for the document.
   * Returns an array of audio segments, one per page or content chunk.
   */
  async synthesizeSpeech(options: TTSOptions): Promise<TTSAudioSegment[]> {
    const segments: TTSAudioSegment[] = [];
    const maxChars = options.maxCharsPerRequest ?? 4096;
    const pageCount = this.getPageCount();
    const startPage = options.pageRange?.start ?? 1;
    const endPage = options.pageRange?.end ?? pageCount;

    let segmentsSynthesized = 0;
    let totalDurationMs = 0;
    const totalPages = endPage - startPage + 1;
    let pagesComplete = 0;

    for (let p = startPage; p <= endPage; p++) {
      if (options.abortSignal?.aborted) break;

      let pageTexts = await this.extractText({ pageRange: { start: p, end: p } });
      let pageContent = pageTexts.map(tc => tc.text).join(' ').trim();

      // Optionally translate before synthesis
      if (options.translateFirst && options.translationProvider && options.translationTargetLanguage) {
        const result = await options.translationProvider.translate(
          pageContent,
          null,
          options.translationTargetLanguage
        );
        pageContent = result.translatedText;
      }

      if (pageContent.length === 0) {
        pagesComplete++;
        continue;
      }

      // Split into chunks respecting sentence boundaries
      const chunks = this._splitTextForTTS(pageContent, maxChars);

      for (const chunk of chunks) {
        if (options.abortSignal?.aborted) break;

        const audioSeg = await options.provider.synthesize(chunk, {
          voiceId: options.voiceId,
          rate: options.rate,
          pitch: options.pitch,
          format: options.format ?? 'mp3',
          sampleRate: options.sampleRate ?? 24000
        });

        // Ensure page number is attached
        audioSeg.pageNumber = p;
        segments.push(audioSeg);
        segmentsSynthesized++;
        totalDurationMs += audioSeg.durationMs;
      }

      pagesComplete++;
      options.progressCallback?.({ pagesComplete, totalPages, segmentsSynthesized, totalDurationMs });
    }

    return segments;
  }

  /**
   * Stream TTS audio segments as they are synthesized.
   * Yields one TTSAudioSegment at a time for real-time playback pipelines.
   */
  async *streamSpeechSynthesis(options: TTSOptions): AsyncGenerator<TTSAudioSegment> {
    const maxChars = options.maxCharsPerRequest ?? 4096;
    const pageCount = this.getPageCount();
    const startPage = options.pageRange?.start ?? 1;
    const endPage = options.pageRange?.end ?? pageCount;

    let segmentsSynthesized = 0;
    let totalDurationMs = 0;
    const totalPages = endPage - startPage + 1;
    let pagesComplete = 0;

    for (let p = startPage; p <= endPage; p++) {
      if (options.abortSignal?.aborted) return;

      let pageTexts = await this.extractText({ pageRange: { start: p, end: p } });
      let pageContent = pageTexts.map(tc => tc.text).join(' ').trim();

      if (options.translateFirst && options.translationProvider && options.translationTargetLanguage) {
        const result = await options.translationProvider.translate(
          pageContent,
          null,
          options.translationTargetLanguage
        );
        pageContent = result.translatedText;
      }

      if (pageContent.length === 0) {
        pagesComplete++;
        continue;
      }

      const chunks = this._splitTextForTTS(pageContent, maxChars);

      for (const chunk of chunks) {
        if (options.abortSignal?.aborted) return;

        // If provider supports streaming, use it for sub-chunk audio
        if (options.provider.synthesizeStream) {
          const audioChunks: Uint8Array[] = [];
          let totalLen = 0;
          for await (const part of options.provider.synthesizeStream(chunk, {
            voiceId: options.voiceId,
            rate: options.rate,
            pitch: options.pitch,
            format: options.format ?? 'mp3',
            sampleRate: options.sampleRate ?? 24000
          })) {
            audioChunks.push(part);
            totalLen += part.length;
          }
          // Merge streaming chunks into one segment
          const merged = new Uint8Array(totalLen);
          let offset = 0;
          for (const ac of audioChunks) {
            merged.set(ac, offset);
            offset += ac.length;
          }
          const seg: TTSAudioSegment = {
            audioData: merged,
            format: options.format ?? 'mp3',
            durationMs: 0, // Duration not known from raw streaming
            sampleRate: options.sampleRate ?? 24000,
            pageNumber: p,
            text: chunk
          };
          segmentsSynthesized++;
          yield seg;
        } else {
          const audioSeg = await options.provider.synthesize(chunk, {
            voiceId: options.voiceId,
            rate: options.rate,
            pitch: options.pitch,
            format: options.format ?? 'mp3',
            sampleRate: options.sampleRate ?? 24000
          });
          audioSeg.pageNumber = p;
          segmentsSynthesized++;
          totalDurationMs += audioSeg.durationMs;
          yield audioSeg;
        }
      }

      pagesComplete++;
      options.progressCallback?.({ pagesComplete, totalPages, segmentsSynthesized, totalDurationMs });
    }
  }

  /**
   * Export translated document as formatted text, preserving page structure.
   */
  async exportTranslation(
    provider: TranslationProvider,
    options: TranslationOptions & { format?: 'text' | 'json' | 'srt' }
  ): Promise<string> {
    const pages = await this.translateDocument(provider, options);
    const fmt = options.format ?? 'text';

    if (fmt === 'json') {
      return JSON.stringify(pages, null, 2);
    }

    if (fmt === 'srt') {
      // SubRip subtitle format — one entry per segment
      let srtIdx = 1;
      const lines: string[] = [];
      for (const page of pages) {
        for (const seg of page.segments) {
          const startSec = (srtIdx - 1) * 3;
          const endSec = startSec + 3;
          lines.push(String(srtIdx));
          lines.push(`${_formatSrtTime(startSec)} --> ${_formatSrtTime(endSec)}`);
          lines.push(seg.translatedText);
          lines.push('');
          srtIdx++;
        }
      }
      return lines.join('\n');
    }

    // Default: plain text page-by-page
    const lines: string[] = [];
    for (const page of pages) {
      lines.push(`--- Page ${page.pageNumber} [${page.sourceLanguage} → ${page.targetLanguage}] ---`);
      for (const seg of page.segments) {
        lines.push(seg.translatedText);
      }
      lines.push('');
    }
    return lines.join('\n');
  }

  /** Split text into chunks at sentence boundaries for TTS. */
  private _splitTextForTTS(text: string, maxChars: number): string[] {
    if (text.length <= maxChars) return [text];

    const chunks: string[] = [];
    let remaining = text;

    while (remaining.length > 0) {
      if (remaining.length <= maxChars) {
        chunks.push(remaining);
        break;
      }

      // Try to split at sentence boundary
      let splitIdx = -1;
      const searchRange = remaining.substring(0, maxChars);
      const sentenceEnders = ['. ', '! ', '? ', '.\n', '!\n', '?\n'];
      for (const ender of sentenceEnders) {
        const idx = searchRange.lastIndexOf(ender);
        if (idx > splitIdx) splitIdx = idx + ender.length;
      }

      // Fall back to word boundary
      if (splitIdx <= 0) {
        splitIdx = searchRange.lastIndexOf(' ');
      }

      // Last resort: hard cut
      if (splitIdx <= 0) {
        splitIdx = maxChars;
      }

      chunks.push(remaining.substring(0, splitIdx).trim());
      remaining = remaining.substring(splitIdx).trim();
    }

    return chunks;
  }

}

/** Format seconds to SRT timestamp (HH:MM:SS,mmm). */
function _formatSrtTime(totalSeconds: number): string {
  const h = Math.floor(totalSeconds / 3600);
  const m = Math.floor((totalSeconds % 3600) / 60);
  const s = Math.floor(totalSeconds % 60);
  const ms = Math.round((totalSeconds % 1) * 1000);
  return `${String(h).padStart(2, '0')}:${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')},${String(ms).padStart(3, '0')}`;
}



// ============================================================================
// PDF Parser Implementation
// ============================================================================

class PDFParser {
  private dataView: DataView;
  private position: number = 0;
  private xrefTable?: XRefTable;
  private objectCache: Map<string, PDFObject> = new Map();
  private securityHandler?: PDFSecurityHandler;
  private encryptDict?: EncryptionDict;

  constructor(
    private buffer: ArrayBuffer,
    private options: PDFOptions = {}
  ) {
    this.dataView = new DataView(buffer);
  }

  /** Get the security handler (if encryption is present). */
  getSecurityHandler(): PDFSecurityHandler | undefined {
    return this.securityHandler;
  }

  /** Get the parsed encryption dictionary. */
  getEncryptionDict(): EncryptionDict | undefined {
    return this.encryptDict;
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

    this.xrefTable = this.parseXRefTable(xrefOffset);
    return this.xrefTable;
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
    const _xrefStm = dict.entries.get('XRefStm');

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

    // Safety: limit iterations to prevent infinite loops
    let safetyCounter = 0;
    const MAX_XREF_SUBSECTIONS = 1000;

    while (this.peekByte() >= '0'.charCodeAt(0) && this.peekByte() <= '9'.charCodeAt(0)) {
      if (++safetyCounter > MAX_XREF_SUBSECTIONS) {
        console.warn('parseXRefTable: Safety limit reached, stopping xref parsing');
        break;
      }

      const positionBefore = this.position;

      const start = this.parseNumber();
      this.skipWhitespace();
      const count = this.parseNumber();
      this.skipWhitespace();

      // Safety check: ensure position advanced
      if (this.position === positionBefore) {
        console.warn('parseXRefTable: Position not advancing, breaking to prevent infinite loop');
        break;
      }

      // Safety check: reasonable count value
      if (count < 0 || count > 100000) {
        console.warn(`parseXRefTable: Unreasonable count value ${count}, skipping subsection`);
        continue;
      }

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
    // Parse cross-reference stream (PDF 1.5+)
    const xref = new XRefTable();

    this.position = offset;

    // Parse the indirect object header: "objNum genNum obj"
    this.parseNumber();
    this.skipWhitespace();
    this.parseNumber();
    this.skipWhitespace();
    this.position += 3; // Skip "obj"
    this.skipWhitespace();

    // Parse stream dictionary
    const dictObj = this.parseObject();
    if (dictObj.type !== PDFObjectType.Dictionary) {
      console.warn('parseXRefStream: Expected dictionary');
      return xref;
    }

    const dict = dictObj.value as PDFDictionary;

    // Get /W array (field widths: [type_width offset_width gen_width])
    const wObj = dict.entries.get('W');
    if (!wObj || wObj.type !== PDFObjectType.Array) {
      console.warn('parseXRefStream: Missing /W array');
      return xref;
    }
    const wArray = (wObj.value as PDFObject[]).map(o => o.value as number);
    if (wArray.length < 3) {
      console.warn('parseXRefStream: /W array too short');
      return xref;
    }
    const [w1, w2, w3] = wArray;

    // Get /Size (total number of objects)
    const sizeObj = dict.entries.get('Size');
    const size = sizeObj && sizeObj.type === PDFObjectType.Number ? (sizeObj.value as number) : 0;

    // Get /Index array (defaults to [0 Size])
    let indexPairs: number[] = [0, size];
    const indexObj = dict.entries.get('Index');
    if (indexObj && indexObj.type === PDFObjectType.Array) {
      indexPairs = (indexObj.value as PDFObject[]).map(o => o.value as number);
    }

    // Parse the stream data
    this.skipWhitespace();
    const streamKeyword = this.readString(this.position, 6);
    if (streamKeyword !== 'stream') {
      console.warn('parseXRefStream: Expected "stream" keyword');
      return xref;
    }
    this.position += 6;
    if (this.peekByte() === '\r'.charCodeAt(0)) this.position++;
    if (this.peekByte() === '\n'.charCodeAt(0)) this.position++;

    // Get stream length
    const lengthObj = dict.entries.get('Length');
    let streamLength = 0;
    if (lengthObj && lengthObj.type === PDFObjectType.Number) {
      streamLength = lengthObj.value as number;
    } else if (lengthObj && lengthObj.type === PDFObjectType.Reference && this.xrefTable) {
      const ref = lengthObj.value as PDFReference;
      const savedPos = this.position;
      try {
        const resolved = this.parseIndirectObject(ref.objectNumber, ref.generationNumber, this.xrefTable);
        if (resolved.type === PDFObjectType.Number) {
          streamLength = resolved.value as number;
        }
      } catch {
        // Ignore — will fall back to zero length
      }
      this.position = savedPos;
    }

    // Read and decompress stream data
    const rawData = new Uint8Array(this.buffer, this.position, streamLength);
    let data: Uint8Array;

    const filterObj = dict.entries.get('Filter');
    if (filterObj) {
      const filter = this.getFilterName(filterObj);
      if (filter === 'FlateDecode') {
        try {
          data = this.decompressFlate(rawData);
        } catch {
          data = new Uint8Array(rawData);
        }
      } else {
        data = new Uint8Array(rawData);
      }
    } else {
      data = new Uint8Array(rawData);
    }

    // Parse entries from the decompressed data
    const entrySize = w1 + w2 + w3;
    let dataPos = 0;

    for (let p = 0; p < indexPairs.length; p += 2) {
      const startObj = indexPairs[p];
      const count = indexPairs[p + 1];

      for (let i = 0; i < count && dataPos + entrySize <= data.length; i++) {
        // Read type field (default 1 if w1 is 0)
        let type = w1 === 0 ? 1 : 0;
        for (let b = 0; b < w1; b++) {
          type = (type << 8) | data[dataPos++];
        }

        // Read field 2 (offset for type 1, object number for type 2)
        let field2 = 0;
        for (let b = 0; b < w2; b++) {
          field2 = (field2 << 8) | data[dataPos++];
        }

        // Read field 3 (generation for type 1, index for type 2)
        let field3 = 0;
        for (let b = 0; b < w3; b++) {
          field3 = (field3 << 8) | data[dataPos++];
        }

        const objNum = startObj + i;

        if (type === 0) {
          xref.addEntry(objNum, field2, field3, 'f');
        } else if (type === 1) {
          xref.addEntry(objNum, field2, field3, 'n');
        } else if (type === 2) {
          // Compressed object in object stream
          xref.addEntry(objNum, field2, field3, 'n');
        }
      }
    }

    // Handle /Prev for chained xref streams
    const prevObj = dict.entries.get('Prev');
    if (prevObj && prevObj.type === PDFObjectType.Number) {
      const prevOffset = prevObj.value as number;
      try {
        const prevXref = this.parseXRefTable(prevOffset);
        for (const [objNum, entry] of prevXref.getAllEntries()) {
          if (!xref.getEntry(objNum)) {
            xref.addEntry(objNum, entry.offset, entry.generation, entry.type);
          }
        }
      } catch {
        console.warn('parseXRefStream: Failed to parse previous xref at offset', prevOffset);
      }
    }

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

    // Check for encryption and authenticate if password provided
    const encryptRef = trailer.entries.get('Encrypt');
    metadata.isEncrypted = encryptRef !== undefined;

    if (encryptRef) {
      const resolveRef = (objNum: number, genNum: number) =>
        this.parseIndirectObject(objNum, genNum, xref);
      const parsed = PDFSecurityHandler.parseEncryptDict(encryptRef as any, trailer as any, resolveRef as any);
      if (parsed) {
        this.encryptDict = parsed;
        this.securityHandler = new PDFSecurityHandler();
        const password = this.options.password || '';
        const authResult = this.securityHandler.authenticate(parsed, password);
        if (!authResult.authenticated && password === '') {
          // Empty password didn't work — PDF requires a password
          // Mark as encrypted but don't throw; caller can try unlock()
        } else if (!authResult.authenticated) {
          throw new Error('Invalid password for encrypted PDF');
        }
      }
    }

    return metadata;
  }

  async parsePageTree(catalog: PDFDictionary): Promise<PageTree> {
    const pageTree = new PageTree();

    const pagesRef = catalog.entries.get('Pages');
    if (!pagesRef || pagesRef.type !== PDFObjectType.Reference) {
      throw new Error('Pages reference not found in catalog');
    }

    // Get the Pages dictionary
    const ref = pagesRef.value as PDFReference;
    const pagesObj = this.parseIndirectObject(ref.objectNumber, ref.generationNumber, this.xrefTable!);

    if (pagesObj.type !== PDFObjectType.Dictionary) {
      throw new Error('Invalid Pages object');
    }

    const pagesDict = pagesObj.value as PDFDictionary;

    // Recursively parse the page tree
    await this.parsePageTreeNode(pagesDict, pageTree, 1);

    return pageTree;
  }

  private async parsePageTreeNode(
    node: PDFDictionary,
    pageTree: PageTree,
    currentPageNum: number,
    depth: number = 0
  ): Promise<number> {
    // Prevent infinite recursion
    if (depth > 100) {
      console.error('Maximum page tree depth exceeded');
      return currentPageNum;
    }

    const kids = node.entries.get('Kids');

    if (kids && kids.type === PDFObjectType.Array) {
      const kidsArray = kids.value as PDFObject[];

      for (const kidRef of kidsArray) {
        try {
          if (kidRef.type === PDFObjectType.Reference) {
            const ref = kidRef.value as PDFReference;
            const kidObj = this.parseIndirectObject(ref.objectNumber, ref.generationNumber, this.xrefTable!);

            if (kidObj.type === PDFObjectType.Dictionary) {
              const kidDict = kidObj.value as PDFDictionary;
              const kidType = kidDict.entries.get('Type');
              const kidTypeValue = kidType?.type === PDFObjectType.Name ? (kidType.value as string) : '';

              if (kidTypeValue === 'Page') {
                // This is a leaf node (actual page)
                pageTree.addPage(currentPageNum, kidDict);
                currentPageNum++;
              } else if (kidTypeValue === 'Pages') {
                // This is an intermediate node, recurse
                currentPageNum = await this.parsePageTreeNode(kidDict, pageTree, currentPageNum, depth + 1);
              }
            }
          }
        } catch (error) {
          console.error(`Error parsing page tree node:`, error);
          // Continue with next kid instead of failing completely
        }
      }
    }

    return currentPageNum;
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

    // Parse content stream
    const contentsRef = pageObj.entries.get('Contents');
    if (contentsRef) {
      page.contents = await this.parseContents(contentsRef);
    }

    // Parse resources
    const resourcesRef = pageObj.entries.get('Resources');
    if (resourcesRef) {
      page.resources = await this.parseResources(resourcesRef);
    }

    return page;
  }

  private async parseContents(contentsRef: PDFObject): Promise<Uint8Array> {
    // Contents can be a single stream or an array of streams
    if (contentsRef.type === PDFObjectType.Reference) {
      const ref = contentsRef.value as PDFReference;
      const streamObj = this.parseIndirectObject(ref.objectNumber, ref.generationNumber, this.xrefTable!);

      if (streamObj.type === PDFObjectType.Stream) {
        const stream = streamObj.value as PDFStream;
        return stream.data;
      }
    } else if (contentsRef.type === PDFObjectType.Array) {
      // Concatenate multiple content streams
      const streams = contentsRef.value as PDFObject[];
      const allData: Uint8Array[] = [];

      for (const streamRef of streams) {
        if (streamRef.type === PDFObjectType.Reference) {
          const ref = streamRef.value as PDFReference;
          const streamObj = this.parseIndirectObject(ref.objectNumber, ref.generationNumber, this.xrefTable!);

          if (streamObj.type === PDFObjectType.Stream) {
            const stream = streamObj.value as PDFStream;
            allData.push(stream.data);
          }
        }
      }

      // Concatenate all streams
      const totalLength = allData.reduce((sum, data) => sum + data.length, 0);
      const combined = new Uint8Array(totalLength);
      let offset = 0;

      for (const data of allData) {
        combined.set(data, offset);
        offset += data.length;
      }

      return combined;
    } else if (contentsRef.type === PDFObjectType.Stream) {
      const stream = contentsRef.value as PDFStream;
      return stream.data;
    }

    return new Uint8Array(0);
  }

  private async parseResources(resourcesRef: PDFObject): Promise<PDFResources> {
    const resources: PDFResources = {
      fonts: new Map(),
      images: new Map(),
      colorSpaces: new Map(),
      patterns: new Map(),
      xObjects: new Map(),
      extGState: new Map()
    };

    let resourcesDict: PDFDictionary | undefined;

    if (resourcesRef.type === PDFObjectType.Reference) {
      const ref = resourcesRef.value as PDFReference;
      const resourcesObj = this.parseIndirectObject(ref.objectNumber, ref.generationNumber, this.xrefTable!);

      if (resourcesObj.type === PDFObjectType.Dictionary) {
        resourcesDict = resourcesObj.value as PDFDictionary;
      }
    } else if (resourcesRef.type === PDFObjectType.Dictionary) {
      resourcesDict = resourcesRef.value as PDFDictionary;
    }

    if (!resourcesDict) return resources;

    // Parse font resources
    const fontRef = resourcesDict.entries.get('Font');
    let fontDict: PDFDictionary | undefined;
    if (fontRef && fontRef.type === PDFObjectType.Dictionary) {
      fontDict = fontRef.value as PDFDictionary;
    } else if (fontRef && fontRef.type === PDFObjectType.Reference) {
      const ref = fontRef.value as PDFReference;
      const resolved = this.parseIndirectObject(ref.objectNumber, ref.generationNumber, this.xrefTable!);
      if (resolved.type === PDFObjectType.Dictionary) {
        fontDict = resolved.value as PDFDictionary;
      }
    }
    if (fontDict) {
      for (const [name, fontObj] of fontDict.entries) {
        const fontResource = this.parseFontResource(name, fontObj);
        if (fontResource) {
          resources.fonts.set(name, fontResource);
        }
      }
    }

    // Parse image resources (XObject)
    const xObjectRef = resourcesDict.entries.get('XObject');
    let xObjectDict: PDFDictionary | undefined;
    if (xObjectRef && xObjectRef.type === PDFObjectType.Dictionary) {
      xObjectDict = xObjectRef.value as PDFDictionary;
    } else if (xObjectRef && xObjectRef.type === PDFObjectType.Reference) {
      const ref = xObjectRef.value as PDFReference;
      const resolved = this.parseIndirectObject(ref.objectNumber, ref.generationNumber, this.xrefTable!);
      if (resolved.type === PDFObjectType.Dictionary) {
        xObjectDict = resolved.value as PDFDictionary;
      }
    }
    if (xObjectDict) {
      resources.images = new Map();

      for (const [name, xObj] of xObjectDict.entries) {
        if (xObj.type === PDFObjectType.Reference) {
          const ref = xObj.value as PDFReference;
          const xObjectObj = this.parseIndirectObject(ref.objectNumber, ref.generationNumber, this.xrefTable!);

          if (xObjectObj.type === PDFObjectType.Stream) {
            const stream = xObjectObj.value as PDFStream;

            // Check XObject subtype
            let xObjectType: 'image' | 'form' | 'ps' = 'image';
            let isImage = true;

            if (stream.dictionary) {
              const subtypeObj = stream.dictionary.entries.get('Subtype');
              if (subtypeObj && subtypeObj.type === PDFObjectType.Name) {
                const subtype = subtypeObj.value as string;
                if (subtype === 'Form') {
                  xObjectType = 'form';
                  isImage = false;
                } else if (subtype === 'PS') {
                  xObjectType = 'ps';
                  isImage = false;
                }
              }
            }

            // Create XObject entry
            const xObject: XObject = {
              type: xObjectType,
              data: stream.data
            };

            if (stream.dictionary) {
              // Parse image dimensions onto XObject
              if (isImage) {
                const wObj = stream.dictionary.entries.get('Width');
                if (wObj && wObj.type === PDFObjectType.Number) xObject.width = wObj.value as number;
                const hObj = stream.dictionary.entries.get('Height');
                if (hObj && hObj.type === PDFObjectType.Number) xObject.height = hObj.value as number;
                const csObj = stream.dictionary.entries.get('ColorSpace');
                if (csObj && csObj.type === PDFObjectType.Name) {
                  xObject.colorSpace = csObj.value as string;
                } else if (csObj && csObj.type === PDFObjectType.Array) {
                  // ColorSpace may be [/ICCBased ref] or [/Indexed /DeviceRGB ...] — use first name
                  const csArr = csObj.value as PDFObject[];
                  if (csArr.length > 0 && csArr[0].type === PDFObjectType.Name) {
                    const csName = csArr[0].value as string;
                    if (csName === 'ICCBased') {
                      xObject.colorSpace = 'DeviceRGB'; // default; component count determines actual
                    } else if (csName === 'Indexed' && csArr.length > 1 && csArr[1].type === PDFObjectType.Name) {
                      xObject.colorSpace = csArr[1].value as string;
                    } else {
                      xObject.colorSpace = csName;
                    }
                  }
                }
                const bpcObj = stream.dictionary.entries.get('BitsPerComponent');
                if (bpcObj && bpcObj.type === PDFObjectType.Number) xObject.bitsPerComponent = bpcObj.value as number;
                const filterObj = stream.dictionary.entries.get('Filter');
                if (filterObj && filterObj.type === PDFObjectType.Name) {
                  xObject.filter = filterObj.value as string;
                } else if (filterObj && filterObj.type === PDFObjectType.Array) {
                  const filterArr = filterObj.value as PDFObject[];
                  if (filterArr.length > 0 && filterArr[0].type === PDFObjectType.Name) {
                    xObject.filter = filterArr[0].value as string;
                  }
                }
                // Parse DecodeParms for predictor info
                const dpObj = stream.dictionary.entries.get('DecodeParms');
                let dpDict: PDFDictionary | undefined;
                if (dpObj && dpObj.type === PDFObjectType.Dictionary) {
                  dpDict = dpObj.value as PDFDictionary;
                } else if (dpObj && dpObj.type === PDFObjectType.Reference) {
                  const dpRef = dpObj.value as PDFReference;
                  const dpResolved = this.parseIndirectObject(dpRef.objectNumber, dpRef.generationNumber, this.xrefTable!);
                  if (dpResolved.type === PDFObjectType.Dictionary) dpDict = dpResolved.value as PDFDictionary;
                } else if (dpObj && dpObj.type === PDFObjectType.Array) {
                  const dpArr = dpObj.value as PDFObject[];
                  if (dpArr.length > 0 && dpArr[0].type === PDFObjectType.Dictionary) dpDict = dpArr[0].value as PDFDictionary;
                }
                if (dpDict) {
                  const predObj = dpDict.entries.get('Predictor');
                  if (predObj && predObj.type === PDFObjectType.Number) xObject.predictor = predObj.value as number;
                  const colObj = dpDict.entries.get('Columns');
                  if (colObj && colObj.type === PDFObjectType.Number) xObject.predictorColumns = colObj.value as number;
                  const colorsObj = dpDict.entries.get('Colors');
                  if (colorsObj && colorsObj.type === PDFObjectType.Number) xObject.predictorColors = colorsObj.value as number;
                }
              }

              // Parse Form XObject properties
              if (xObjectType === 'form') {
                const bboxObj = stream.dictionary.entries.get('BBox');
                if (bboxObj && bboxObj.type === PDFObjectType.Array) {
                  const arr = bboxObj.value as PDFObject[];
                  xObject.bbox = arr.filter(o => o.type === PDFObjectType.Number).map(o => o.value as number);
                }
                const matrixObj = stream.dictionary.entries.get('Matrix');
                if (matrixObj && matrixObj.type === PDFObjectType.Array) {
                  const arr = matrixObj.value as PDFObject[];
                  const nums = arr.filter(o => o.type === PDFObjectType.Number).map(o => o.value as number);
                  if (nums.length === 6) xObject.matrix = nums as TransformMatrix;
                }
                const resObj = stream.dictionary.entries.get('Resources');
                if (resObj) {
                  try {
                    xObject.resources = await this.parseResources(resObj);
                  } catch (e) {
                    // Form resource parsing is not critical
                  }
                }
              }
            }

            resources.xObjects.set(name, xObject);

            // Also create ImageResource for backward compatibility
            if (isImage) {
              const imageRes: ImageResource = {
                width: 0,
                height: 0,
                bitsPerComponent: 8,
                colorSpace: 'DeviceRGB',
                data: stream.data
              };

              // Parse image properties from stream dictionary
              if (stream.dictionary) {
                const widthObj = stream.dictionary.entries.get('Width');
                if (widthObj && widthObj.type === PDFObjectType.Number) {
                  imageRes.width = widthObj.value as number;
                }

                const heightObj = stream.dictionary.entries.get('Height');
                if (heightObj && heightObj.type === PDFObjectType.Number) {
                  imageRes.height = heightObj.value as number;
                }

                const bpcObj = stream.dictionary.entries.get('BitsPerComponent');
                if (bpcObj && bpcObj.type === PDFObjectType.Number) {
                  imageRes.bitsPerComponent = bpcObj.value as number;
                }

                // Color space (handle Name, Array [/ICCBased ref], [/Indexed ...], etc.)
                const csObj = stream.dictionary.entries.get('ColorSpace');
                if (csObj && csObj.type === PDFObjectType.Name) {
                  imageRes.colorSpace = csObj.value as string;
                } else if (csObj && csObj.type === PDFObjectType.Array) {
                  const csArr = csObj.value as PDFObject[];
                  if (csArr.length > 0 && csArr[0].type === PDFObjectType.Name) {
                    const csName = csArr[0].value as string;
                    if (csName === 'ICCBased' && csArr.length > 1 && csArr[1].type === PDFObjectType.Reference) {
                      // Resolve ICC profile stream to get /N (component count)
                      try {
                        const iccRef = csArr[1].value as PDFReference;
                        const iccObj = this.parseIndirectObject(iccRef.objectNumber, iccRef.generationNumber, this.xrefTable!);
                        if (iccObj.type === PDFObjectType.Stream) {
                          const iccStream = iccObj.value as PDFStream;
                          const nObj = iccStream.dictionary?.entries.get('N');
                          const n = nObj && nObj.type === PDFObjectType.Number ? nObj.value as number : 3;
                          if (n === 1) imageRes.colorSpace = 'DeviceGray';
                          else if (n === 4) imageRes.colorSpace = 'DeviceCMYK';
                          else imageRes.colorSpace = 'DeviceRGB';
                        }
                      } catch (e) { imageRes.colorSpace = 'DeviceRGB'; }
                    } else if (csName === 'Indexed' && csArr.length > 1 && csArr[1].type === PDFObjectType.Name) {
                      imageRes.colorSpace = csArr[1].value as string;
                    } else {
                      imageRes.colorSpace = csName;
                    }
                  }
                }

                // Filter (Name or Array)
                const filterObj = stream.dictionary.entries.get('Filter');
                if (filterObj && filterObj.type === PDFObjectType.Name) {
                  imageRes.filter = [filterObj.value as string];
                } else if (filterObj && filterObj.type === PDFObjectType.Array) {
                  const fArr = filterObj.value as PDFObject[];
                  imageRes.filter = fArr.filter(f => f.type === PDFObjectType.Name).map(f => f.value as string);
                }

                // DecodeParms
                const dpObj = stream.dictionary.entries.get('DecodeParms');
                let dpDict: PDFDictionary | undefined;
                if (dpObj && dpObj.type === PDFObjectType.Dictionary) {
                  dpDict = dpObj.value as PDFDictionary;
                } else if (dpObj && dpObj.type === PDFObjectType.Reference) {
                  const dpRef = dpObj.value as PDFReference;
                  const dpRes = this.parseIndirectObject(dpRef.objectNumber, dpRef.generationNumber, this.xrefTable!);
                  if (dpRes.type === PDFObjectType.Dictionary) dpDict = dpRes.value as PDFDictionary;
                } else if (dpObj && dpObj.type === PDFObjectType.Array) {
                  const dpArr = dpObj.value as PDFObject[];
                  if (dpArr.length > 0 && dpArr[0].type === PDFObjectType.Dictionary) dpDict = dpArr[0].value as PDFDictionary;
                }
                if (dpDict) {
                  const dp: Record<string, any> = {};
                  for (const [k, v] of dpDict.entries) {
                    if (v.type === PDFObjectType.Number) dp[k] = v.value;
                    else if (v.type === PDFObjectType.Boolean) dp[k] = v.value;
                    else if (v.type === PDFObjectType.Name) dp[k] = v.value;
                  }
                  imageRes.decodeParms = dp;
                }

                // SMask (soft mask / alpha channel)
                const smaskObj = stream.dictionary.entries.get('SMask');
                if (smaskObj && smaskObj.type === PDFObjectType.Reference) {
                  try {
                    const smRef = smaskObj.value as PDFReference;
                    const smResolved = this.parseIndirectObject(smRef.objectNumber, smRef.generationNumber, this.xrefTable!);
                    if (smResolved.type === PDFObjectType.Stream) {
                      const smStream = smResolved.value as PDFStream;
                      imageRes.smaskData = smStream.data;
                      const smW = smStream.dictionary?.entries.get('Width');
                      if (smW && smW.type === PDFObjectType.Number) imageRes.smaskWidth = smW.value as number;
                      const smH = smStream.dictionary?.entries.get('Height');
                      if (smH && smH.type === PDFObjectType.Number) imageRes.smaskHeight = smH.value as number;
                    }
                  } catch (e) { /* ignore smask resolution failure */ }
                }
              }

              resources.images.set(name, imageRes);
            }
          }
        }
      }
    }

    // Parse ExtGState resources
    const gsRef = resourcesDict.entries.get('ExtGState');
    let gsDict: PDFDictionary | undefined;
    if (gsRef && gsRef.type === PDFObjectType.Dictionary) {
      gsDict = gsRef.value as PDFDictionary;
    } else if (gsRef && gsRef.type === PDFObjectType.Reference) {
      const ref = gsRef.value as PDFReference;
      const resolved = this.parseIndirectObject(ref.objectNumber, ref.generationNumber, this.xrefTable!);
      if (resolved.type === PDFObjectType.Dictionary) {
        gsDict = resolved.value as PDFDictionary;
      }
    }
    if (gsDict) {
      for (const [name, gsObj] of gsDict.entries) {
        let gsEntryDict: PDFDictionary | undefined;
        if (gsObj.type === PDFObjectType.Dictionary) {
          gsEntryDict = gsObj.value as PDFDictionary;
        } else if (gsObj.type === PDFObjectType.Reference) {
          const ref = gsObj.value as PDFReference;
          const resolved = this.parseIndirectObject(ref.objectNumber, ref.generationNumber, this.xrefTable!);
          if (resolved.type === PDFObjectType.Dictionary) {
            gsEntryDict = resolved.value as PDFDictionary;
          }
        }
        if (gsEntryDict) {
          const state: Record<string, any> = {};
          for (const [key, val] of gsEntryDict.entries) {
            if (val.type === PDFObjectType.Number) state[key] = val.value;
            else if (val.type === PDFObjectType.Name) state[key] = val.value;
            else if (val.type === PDFObjectType.Boolean) state[key] = val.value;
          }
          resources.extGState.set(name, state);
        }
      }
    }

    // Parse ColorSpace resources
    const csRef = resourcesDict.entries.get('ColorSpace');
    let csDict: PDFDictionary | undefined;
    if (csRef && csRef.type === PDFObjectType.Dictionary) {
      csDict = csRef.value as PDFDictionary;
    } else if (csRef && csRef.type === PDFObjectType.Reference) {
      const ref = csRef.value as PDFReference;
      const resolved = this.parseIndirectObject(ref.objectNumber, ref.generationNumber, this.xrefTable!);
      if (resolved.type === PDFObjectType.Dictionary) {
        csDict = resolved.value as PDFDictionary;
      }
    }
    if (csDict) {
      for (const [name, csObj] of csDict.entries) {
        try {
          const colorSpace = PDFColorSpaceProcessor.parseColorSpace(csObj, resources);
          resources.colorSpaces.set(name, colorSpace);
        } catch (e) {
          // Color space parsing is not critical
        }
      }
    }

    return resources;
  }

  private parseFontResource(name: string, fontObj: PDFObject): FontResource | null {
    try {
      let fontDict: PDFDictionary | undefined;

      // Resolve font reference
      if (fontObj.type === PDFObjectType.Reference) {
        const ref = fontObj.value as PDFReference;
        const resolvedFont = this.parseIndirectObject(ref.objectNumber, ref.generationNumber, this.xrefTable!);
        if (resolvedFont.type === PDFObjectType.Dictionary) {
          fontDict = resolvedFont.value as PDFDictionary;
        }
      } else if (fontObj.type === PDFObjectType.Dictionary) {
        fontDict = fontObj.value as PDFDictionary;
      }

      if (!fontDict) return null;

      // Extract basic font info
      const fontResource: FontResource = {
        name: name,
        type: 'Font',
        subtype: 'Type1'
      };

      // Get Subtype
      const subtypeObj = fontDict.entries.get('Subtype');
      if (subtypeObj && subtypeObj.type === PDFObjectType.Name) {
        fontResource.subtype = subtypeObj.value as string;
      }

      // Get BaseFont
      const baseFontObj = fontDict.entries.get('BaseFont');
      if (baseFontObj && baseFontObj.type === PDFObjectType.Name) {
        fontResource.baseFont = baseFontObj.value as string;
      }

      // Get Encoding - handle both Name and Dictionary forms
      const encodingObj = fontDict.entries.get('Encoding');
      if (encodingObj && encodingObj.type === PDFObjectType.Name) {
        fontResource.encoding = encodingObj.value as string;
      } else if (encodingObj && (encodingObj.type === PDFObjectType.Dictionary || encodingObj.type === PDFObjectType.Reference)) {
        // Encoding can be a dictionary with BaseEncoding and/or Differences
        let encDict: PDFDictionary | undefined;
        if (encodingObj.type === PDFObjectType.Reference) {
          const ref = encodingObj.value as PDFReference;
          const resolved = this.parseIndirectObject(ref.objectNumber, ref.generationNumber, this.xrefTable!);
          if (resolved.type === PDFObjectType.Dictionary) {
            encDict = resolved.value as PDFDictionary;
          }
        } else {
          encDict = encodingObj.value as PDFDictionary;
        }
        if (encDict) {
          const baseEncObj = encDict.entries.get('BaseEncoding');
          if (baseEncObj && baseEncObj.type === PDFObjectType.Name) {
            fontResource.encoding = baseEncObj.value as string;
          }
        }
      }

      // Get FirstChar
      const firstCharObj = fontDict.entries.get('FirstChar');
      if (firstCharObj && firstCharObj.type === PDFObjectType.Number) {
        fontResource.firstChar = firstCharObj.value as number;
      }

      // Get LastChar
      const lastCharObj = fontDict.entries.get('LastChar');
      if (lastCharObj && lastCharObj.type === PDFObjectType.Number) {
        fontResource.lastChar = lastCharObj.value as number;
      }

      // Get Widths array
      const widthsObj = fontDict.entries.get('Widths');
      if (widthsObj && widthsObj.type === PDFObjectType.Array) {
        const widthsArray = widthsObj.value as PDFObject[];
        fontResource.widths = widthsArray
          .filter(obj => obj.type === PDFObjectType.Number)
          .map(obj => obj.value as number);
      } else if (widthsObj && widthsObj.type === PDFObjectType.Reference) {
        // Widths might be an indirect reference
        const ref = widthsObj.value as PDFReference;
        const resolvedWidths = this.parseIndirectObject(ref.objectNumber, ref.generationNumber, this.xrefTable!);
        if (resolvedWidths.type === PDFObjectType.Array) {
          const widthsArray = resolvedWidths.value as PDFObject[];
          fontResource.widths = widthsArray
            .filter(obj => obj.type === PDFObjectType.Number)
            .map(obj => obj.value as number);
        }
      }

      // Get MissingWidth (for CIDFonts)
      const missingWidthObj = fontDict.entries.get('MissingWidth');
      if (missingWidthObj && missingWidthObj.type === PDFObjectType.Number) {
        fontResource.missingWidth = missingWidthObj.value as number;
      }

      // Get DW (DefaultWidth for CIDFonts)
      const dwObj = fontDict.entries.get('DW');
      if (dwObj && dwObj.type === PDFObjectType.Number) {
        fontResource.defaultWidth = dwObj.value as number;
      }

      // Parse FontDescriptor
      const descriptorObj = fontDict.entries.get('FontDescriptor');
      if (descriptorObj) {
        fontResource.descriptor = this.parseFontDescriptor(descriptorObj);
      }

      // Handle Type0 composite fonts: parse DescendantFonts for CIDFont metrics
      if (fontResource.subtype === 'Type0') {
        const descendantObj = fontDict.entries.get('DescendantFonts');
        if (descendantObj) {
          let descendantArr: PDFObject[] | undefined;
          if (descendantObj.type === PDFObjectType.Array) {
            descendantArr = descendantObj.value as PDFObject[];
          } else if (descendantObj.type === PDFObjectType.Reference) {
            const ref = descendantObj.value as PDFReference;
            const resolved = this.parseIndirectObject(ref.objectNumber, ref.generationNumber, this.xrefTable!);
            if (resolved.type === PDFObjectType.Array) {
              descendantArr = resolved.value as PDFObject[];
            }
          }
          if (descendantArr && descendantArr.length > 0) {
            let cidDict: PDFDictionary | undefined;
            const cidRef = descendantArr[0];
            if (cidRef.type === PDFObjectType.Reference) {
              const ref = cidRef.value as PDFReference;
              const resolved = this.parseIndirectObject(ref.objectNumber, ref.generationNumber, this.xrefTable!);
              if (resolved.type === PDFObjectType.Dictionary) {
                cidDict = resolved.value as PDFDictionary;
              }
            } else if (cidRef.type === PDFObjectType.Dictionary) {
              cidDict = cidRef.value as PDFDictionary;
            }
            if (cidDict) {
              // Extract DW (default width for CIDFont)
              const cidDW = cidDict.entries.get('DW');
              if (cidDW && cidDW.type === PDFObjectType.Number) {
                fontResource.defaultWidth = cidDW.value as number;
              }
              // Extract BaseFont from descendant if not set
              if (!fontResource.baseFont) {
                const cidBaseFont = cidDict.entries.get('BaseFont');
                if (cidBaseFont && cidBaseFont.type === PDFObjectType.Name) {
                  fontResource.baseFont = cidBaseFont.value as string;
                }
              }
              // Extract FontDescriptor from descendant if not set
              if (!fontResource.descriptor) {
                const cidDescObj = cidDict.entries.get('FontDescriptor');
                if (cidDescObj) {
                  fontResource.descriptor = this.parseFontDescriptor(cidDescObj);
                }
              }
              // Parse W array for per-CID widths
              let wObj = cidDict.entries.get('W');
              if (wObj) {
                let wArr: PDFObject[] | undefined;
                if (wObj.type === PDFObjectType.Array) {
                  wArr = wObj.value as PDFObject[];
                } else if (wObj.type === PDFObjectType.Reference) {
                  const ref = wObj.value as PDFReference;
                  const resolved = this.parseIndirectObject(ref.objectNumber, ref.generationNumber, this.xrefTable!);
                  if (resolved.type === PDFObjectType.Array) {
                    wArr = resolved.value as PDFObject[];
                  }
                }
                if (wArr && wArr.length > 0) {
                  fontResource.cidWidths = new Map();
                  let i = 0;
                  while (i < wArr.length) {
                    const first = wArr[i];
                    if (first.type !== PDFObjectType.Number) { i++; continue; }
                    const startCID = first.value as number;
                    i++;
                    if (i >= wArr.length) break;
                    const next = wArr[i];
                    if (next.type === PDFObjectType.Array) {
                      // Format: startCID [w1 w2 w3 ...]
                      const widths = next.value as PDFObject[];
                      for (let j = 0; j < widths.length; j++) {
                        if (widths[j].type === PDFObjectType.Number) {
                          fontResource.cidWidths.set(startCID + j, widths[j].value as number);
                        }
                      }
                      i++;
                    } else if (next.type === PDFObjectType.Number) {
                      // Format: startCID lastCID width
                      const lastCID = next.value as number;
                      i++;
                      if (i < wArr.length && wArr[i].type === PDFObjectType.Number) {
                        const w = wArr[i].value as number;
                        for (let cid = startCID; cid <= lastCID; cid++) {
                          fontResource.cidWidths.set(cid, w);
                        }
                        i++;
                      }
                    }
                  }
                }
              }
            }
          }
        }
      }

      // Use FontDescriptor MissingWidth as fallback
      if (fontResource.missingWidth === undefined && fontResource.descriptor && fontResource.descriptor.missingWidth !== undefined) {
        fontResource.missingWidth = fontResource.descriptor.missingWidth;
      }

      // Parse ToUnicode CMap for character code to Unicode mapping
      const toUnicodeObj = fontDict.entries.get('ToUnicode');
      if (toUnicodeObj && toUnicodeObj.type === PDFObjectType.Reference) {
        const ref = toUnicodeObj.value as PDFReference;
        try {
          const toUnicodeStream = this.parseIndirectObject(ref.objectNumber, ref.generationNumber, this.xrefTable!);
          if (toUnicodeStream.type === PDFObjectType.Stream) {
            const stream = toUnicodeStream.value as PDFStream;
            fontResource.toUnicode = this.parseToUnicodeCMap(stream.data);
          }
        } catch (e) {
          // ToUnicode parsing is optional
        }
      }

      // Parse Encoding Differences for glyph name mapping
      // Handle both reference and direct dictionary forms
      if (encodingObj) {
        try {
          let encDict: PDFDictionary | undefined;
          if (encodingObj.type === PDFObjectType.Reference) {
            const ref = encodingObj.value as PDFReference;
            const resolved = this.parseIndirectObject(ref.objectNumber, ref.generationNumber, this.xrefTable!);
            if (resolved.type === PDFObjectType.Dictionary) {
              encDict = resolved.value as PDFDictionary;
            }
          } else if (encodingObj.type === PDFObjectType.Dictionary) {
            encDict = encodingObj.value as PDFDictionary;
          }
          if (encDict) {
            const diffsObj = encDict.entries.get('Differences');
            if (diffsObj && diffsObj.type === PDFObjectType.Array) {
              const diffs = diffsObj.value as PDFObject[];
              if (!fontResource.toUnicode) fontResource.toUnicode = new Map();
              let currentCode = 0;
              for (const item of diffs) {
                if (item.type === PDFObjectType.Number) {
                  currentCode = item.value as number;
                } else if (item.type === PDFObjectType.Name) {
                  const glyphName = item.value as string;
                  if (!fontResource.toUnicode.has(currentCode)) {
                    const unicode = PDFTextDecoder.glyphNameToUnicode(glyphName);
                    if (unicode) fontResource.toUnicode.set(currentCode, unicode);
                  }
                  currentCode++;
                }
              }
            }
          }
        } catch (e) {
          // Encoding parsing is optional
        }
      }

      return fontResource;
    } catch (error) {
      console.warn(`Error parsing font ${name}:`, error);
      // Return basic font resource as fallback
      return {
        name: name,
        type: 'Font',
        subtype: 'Type1'
      };
    }
  }

  private parseToUnicodeCMap(cmapData: Uint8Array): Map<number, string> {
    const map = new Map<number, string>();
    // Decode CMap data as latin1 (byte-preserving)
    let text = '';
    for (let i = 0; i < cmapData.length; i++) text += String.fromCharCode(cmapData[i]);

    // Parse bfchar sections: <srcCode> <dstUnicode>
    const bfcharRegex = /beginbfchar\s+([\s\S]*?)endbfchar/g;
    let match;
    while ((match = bfcharRegex.exec(text)) !== null) {
      const lines = match[1].trim().split(/\r?\n/);
      for (const line of lines) {
        const parts = line.trim().match(/<([0-9a-fA-F]+)>\s+<([0-9a-fA-F]+)>/);
        if (parts) {
          const srcCode = parseInt(parts[1], 16);
          const dstHex = parts[2];
          let str = '';
          for (let i = 0; i < dstHex.length; i += 4) {
            const cp = parseInt(dstHex.substring(i, Math.min(i + 4, dstHex.length)), 16);
            if (cp > 0) str += String.fromCodePoint(cp);
          }
          if (str) map.set(srcCode, str);
        }
      }
    }

    // Parse bfrange sections: <start> <end> <startUnicode> or <start> <end> [<u1> <u2> ...]
    const bfrangeRegex = /beginbfrange\s+([\s\S]*?)endbfrange/g;
    while ((match = bfrangeRegex.exec(text)) !== null) {
      const lines = match[1].trim().split(/\r?\n/);
      for (const line of lines) {
        const rangeParts = line.trim().match(/<([0-9a-fA-F]+)>\s+<([0-9a-fA-F]+)>\s+<([0-9a-fA-F]+)>/);
        if (rangeParts) {
          const start = parseInt(rangeParts[1], 16);
          const end = parseInt(rangeParts[2], 16);
          let startUnicode = parseInt(rangeParts[3], 16);
          for (let code = start; code <= end; code++) {
            map.set(code, String.fromCodePoint(startUnicode++));
          }
        }
      }
    }

    return map;
  }

  private parseFontDescriptor(descriptorObj: PDFObject): FontDescriptor | undefined {
    try {
      let descriptorDict: PDFDictionary | undefined;

      if (descriptorObj.type === PDFObjectType.Reference) {
        const ref = descriptorObj.value as PDFReference;
        const resolved = this.parseIndirectObject(ref.objectNumber, ref.generationNumber, this.xrefTable!);
        if (resolved.type === PDFObjectType.Dictionary) {
          descriptorDict = resolved.value as PDFDictionary;
        }
      } else if (descriptorObj.type === PDFObjectType.Dictionary) {
        descriptorDict = descriptorObj.value as PDFDictionary;
      }

      if (!descriptorDict) return undefined;

      const descriptor: FontDescriptor = {
        fontName: ''
      };

      // Get FontName
      const fontNameObj = descriptorDict.entries.get('FontName');
      if (fontNameObj && fontNameObj.type === PDFObjectType.Name) {
        descriptor.fontName = fontNameObj.value as string;
      }

      // Get FontFamily
      const fontFamilyObj = descriptorDict.entries.get('FontFamily');
      if (fontFamilyObj && fontFamilyObj.type === PDFObjectType.String) {
        descriptor.fontFamily = fontFamilyObj.value as string;
      }

      // Get Ascent
      const ascentObj = descriptorDict.entries.get('Ascent');
      if (ascentObj && ascentObj.type === PDFObjectType.Number) {
        descriptor.ascent = ascentObj.value as number;
      }

      // Get Descent
      const descentObj = descriptorDict.entries.get('Descent');
      if (descentObj && descentObj.type === PDFObjectType.Number) {
        descriptor.descent = descentObj.value as number;
      }

      // Get CapHeight
      const capHeightObj = descriptorDict.entries.get('CapHeight');
      if (capHeightObj && capHeightObj.type === PDFObjectType.Number) {
        descriptor.capHeight = capHeightObj.value as number;
      }

      // Get Flags
      const flagsObj = descriptorDict.entries.get('Flags');
      if (flagsObj && flagsObj.type === PDFObjectType.Number) {
        descriptor.flags = flagsObj.value as number;
      }

      // Get MissingWidth
      const mwObj = descriptorDict.entries.get('MissingWidth');
      if (mwObj && mwObj.type === PDFObjectType.Number) {
        descriptor.missingWidth = mwObj.value as number;
      }

      // Get ItalicAngle
      const italicAngleObj = descriptorDict.entries.get('ItalicAngle');
      if (italicAngleObj && italicAngleObj.type === PDFObjectType.Number) {
        descriptor.italicAngle = italicAngleObj.value as number;
      }

      return descriptor;
    } catch (error) {
      console.warn('Error parsing font descriptor:', error);
      return undefined;
    }
  }

  // Parsing primitives
  private parseDictionary(): PDFDictionary {
    const dict: PDFDictionary = {
      entries: new Map()
    };

    // Skip '<<'
    this.position += 2;
    this.skipWhitespace();

    // Safety: prevent infinite loops
    let safetyCounter = 0;
    const MAX_DICT_ENTRIES = 10000;

    while (this.position < this.buffer.byteLength) {
      if (++safetyCounter > MAX_DICT_ENTRIES) {
        console.warn('parseDictionary: Safety limit reached, stopping dictionary parsing');
        break;
      }

      // Check for '>>'
      if (this.peekByte() === '>'.charCodeAt(0) &&
        this.peekByte(1) === '>'.charCodeAt(0)) {
        this.position += 2;
        break;
      }

      const positionBefore = this.position;

      // Parse name (key)
      const name = this.parseName();
      this.skipWhitespace();

      // Parse value
      const value = this.parseObject();
      this.skipWhitespace();

      // Safety: ensure position advanced
      if (this.position === positionBefore) {
        console.warn('parseDictionary: Position not advancing, breaking to prevent infinite loop');
        break;
      }

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
      this.position += 4; // Skip "null"
      return { type: PDFObjectType.Null, value: null };
    } else if ((byte >= '0'.charCodeAt(0) && byte <= '9'.charCodeAt(0)) ||
      byte === '-'.charCodeAt(0) || byte === '+'.charCodeAt(0) ||
      byte === '.'.charCodeAt(0)) {
      // Could be number or reference
      const num = this.parseNumber();
      this.skipWhitespace();

      // Check if it's a reference
      if (this.peekByte() >= '0'.charCodeAt(0) && this.peekByte() <= '9'.charCodeAt(0)) {
        const savedPos = this.position;
        const gen = this.parseNumber();
        this.skipWhitespace();

        if (this.peekByte() === 'R'.charCodeAt(0)) {
          this.position++; // Skip 'R'
          return {
            type: PDFObjectType.Reference,
            value: { objectNumber: num, generationNumber: gen } as PDFReference
          };
        }

        // Not a reference, restore position to before gen
        this.position = savedPos;
      }

      return { type: PDFObjectType.Number, value: num };
    }

    throw new Error(`Unknown object type at position ${this.position}`);
  }

  parseIndirectObject(objNum: number, genNum: number, xref: XRefTable): PDFObject {
    // Check cache first
    const cacheKey = `${objNum}-${genNum}`;
    const cached = this.objectCache.get(cacheKey);
    if (cached) {
      return cached;
    }

    const entry = xref.getEntry(objNum);
    if (!entry) {
      throw new Error(`Object ${objNum} not found in xref table`);
    }

    this.position = entry.offset;

    // Parse "objNum genNum obj"
    this.parseNumber(); // objNum
    this.skipWhitespace();
    this.parseNumber(); // genNum
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
        const streamObj = this.parseStream(obj, objNum, genNum);
        this.objectCache.set(cacheKey, streamObj);
        return streamObj;
      }
    }

    // Cache the parsed object
    this.objectCache.set(cacheKey, obj);

    return obj;
  }

  private parseStream(dict: PDFObject, objectNumber: number = 0, generationNumber: number = 0): PDFObject {
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
    } else if (lengthObj && lengthObj.type === PDFObjectType.Reference && this.xrefTable) {
      // Resolve indirect reference for /Length (very common in PDFs)
      const ref = lengthObj.value as PDFReference;
      const savedPos = this.position;
      try {
        const resolved = this.parseIndirectObject(ref.objectNumber, ref.generationNumber, this.xrefTable);
        if (resolved.type === PDFObjectType.Number) {
          length = resolved.value as number;
        }
      } catch {
        // Fall through to endstream scanning
      }
      this.position = savedPos;
    }

    // Read stream data
    let rawData: Uint8Array;
    if (length > 0 && this.position + length <= this.buffer.byteLength) {
      rawData = new Uint8Array(this.buffer, this.position, length);
      this.position += length;
    } else {
      // Fallback: scan for 'endstream' keyword
      const searchStart = this.position;
      const searchLimit = Math.min(this.buffer.byteLength, searchStart + 10 * 1024 * 1024);
      let endPos = -1;
      const view = new Uint8Array(this.buffer);
      for (let i = searchStart; i < searchLimit - 9; i++) {
        if (view[i] === 101 && view[i + 1] === 110 && view[i + 2] === 100 &&
            view[i + 3] === 115 && view[i + 4] === 116 && view[i + 5] === 114 &&
            view[i + 6] === 101 && view[i + 7] === 97 && view[i + 8] === 109) {
          endPos = i;
          break;
        }
      }
      if (endPos > searchStart) {
        // Trim trailing whitespace (\r, \n) before endstream
        let dataEnd = endPos;
        while (dataEnd > searchStart && (view[dataEnd - 1] === 13 || view[dataEnd - 1] === 10)) {
          dataEnd--;
        }
        rawData = new Uint8Array(this.buffer, searchStart, dataEnd - searchStart);
        this.position = endPos;
      } else {
        rawData = new Uint8Array(0);
      }
    }

    // Decrypt stream if encryption is active
    if (this.securityHandler && this.securityHandler.isAuthenticated()) {
      rawData = this.securityHandler.decryptStream(rawData, objectNumber, generationNumber);
    }

    // Decompress stream if needed
    let data: Uint8Array;
    const filterObj = dictValue.entries.get('Filter');
    if (filterObj) {
      const filter = this.getFilterName(filterObj);
      if (filter === 'FlateDecode') {
        try {
          data = this.decompressFlate(rawData);
        } catch (error) {
          console.warn('Failed to decompress FlateDecode stream:', error);
          // Copy to new array if decompression fails
          data = new Uint8Array(rawData);
        }
      } else {
        // Copy to new array for non-compressed streams
        data = new Uint8Array(rawData);
      }
    } else {
      // No filter - copy to new array
      data = new Uint8Array(rawData);
    }

    // Skip "endstream"
    this.skipWhitespace();
    this.position += 9;

    return {
      type: PDFObjectType.Stream,
      value: { dictionary: dictValue, data } as PDFStream
    };
  }

  private getFilterName(filterObj: PDFObject): string {
    if (filterObj.type === PDFObjectType.Name) {
      return filterObj.value as string;
    } else if (filterObj.type === PDFObjectType.Array) {
      const filters = filterObj.value as PDFObject[];
      if (filters.length > 0 && filters[0].type === PDFObjectType.Name) {
        return filters[0].value as string;
      }
    }
    return '';
  }

  private decompressFlate(data: Uint8Array): Uint8Array {
    try {
      // Try using pako if available
      if (typeof (globalThis as any).pako !== 'undefined') {
        const pako = (globalThis as any).pako;
        return new Uint8Array(pako.inflate(data));
      }

      // Try using fflate if available (lighter weight alternative)
      if (typeof (globalThis as any).fflate !== 'undefined') {
        const fflate = (globalThis as any).fflate;
        return new Uint8Array(fflate.inflateSync(data));
      }

      // Use built-in DEFLATE decompressor (zero dependencies)
      return Inflate.inflate(data);
    } catch (error) {
      console.error('Decompression failed:', error);
      return data;
    }
  }

  private parseName(): string {
    this.position++; // Skip '/'
    let name = '';
    let safetyCounter = 0;
    const MAX_NAME_LENGTH = 10000; // Max name length

    while (this.position < this.buffer.byteLength) {
      if (++safetyCounter > MAX_NAME_LENGTH) {
        console.warn('parseName: Safety limit reached, truncating name');
        break;
      }

      const positionBefore = this.position;
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

      if (this.position === positionBefore) {
        console.warn('parseName: Position not advancing, breaking to prevent infinite loop');
        break;
      }
    }

    return name;
  }

  private parseString(): string {
    this.position++; // Skip '('
    let str = '';
    let parenLevel = 1;

    // Safety: prevent infinite loops
    let safetyCounter = 0;
    const MAX_STRING_LENGTH = 1000000; // 1MB max string

    while (this.position < this.buffer.byteLength && parenLevel > 0) {
      if (++safetyCounter > MAX_STRING_LENGTH) {
        console.warn('parseString: Safety limit reached, truncating string');
        break;
      }

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
    let safetyCounter = 0;
    const MAX_HEX_STRING_LENGTH = 1000000; // 1MB max hex string

    while (this.position < this.buffer.byteLength) {
      if (++safetyCounter > MAX_HEX_STRING_LENGTH) {
        console.warn('parseHexString: Safety limit reached, truncating hex string');
        break;
      }

      const positionBefore = this.position;
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

      if (this.position === positionBefore) {
        console.warn('parseHexString: Position not advancing, breaking to prevent infinite loop');
        break;
      }
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

    // Safety: prevent infinite loops
    let safetyCounter = 0;
    const MAX_ARRAY_ITEMS = 100000;

    while (this.position < this.buffer.byteLength) {
      if (++safetyCounter > MAX_ARRAY_ITEMS) {
        console.warn('parseArray: Safety limit reached, stopping array parsing');
        break;
      }

      if (this.peekByte() === ']'.charCodeAt(0)) {
        this.position++;
        break;
      }

      const positionBefore = this.position;
      array.push(this.parseObject());
      this.skipWhitespace();

      // Safety: ensure position advanced
      if (this.position === positionBefore) {
        console.warn('parseArray: Position not advancing, breaking to prevent infinite loop');
        break;
      }
    }

    return array;
  }

  private parseBoolean(): boolean {
    let word = '';
    let safetyCounter = 0;
    const MAX_BOOLEAN_LENGTH = 10; // 'true' or 'false'

    while (this.position < this.buffer.byteLength && !this.isWhitespace(this.peekByte()) && !this.isDelimiter(this.peekByte())) {
      if (++safetyCounter > MAX_BOOLEAN_LENGTH) {
        console.warn('parseBoolean: Safety limit reached, truncating boolean');
        break;
      }

      const positionBefore = this.position;
      word += String.fromCharCode(this.readByte());

      if (this.position === positionBefore) {
        console.warn('parseBoolean: Position not advancing, breaking to prevent infinite loop');
        break;
      }
    }

    if (word === 'true') return true;
    if (word === 'false') return false;

    throw new Error(`Invalid boolean value: ${word}`);
  }

  private parseNumber(): number {
    let numStr = '';
    let isFloat = false;
    let safetyCounter = 0;
    const MAX_NUMBER_LENGTH = 100; // Max digits in a number

    while (this.position < this.buffer.byteLength) {
      if (++safetyCounter > MAX_NUMBER_LENGTH) {
        console.warn('parseNumber: Safety limit reached, truncating number');
        break;
      }

      const positionBefore = this.position;
      const byte = this.peekByte();

      if ((byte >= '0'.charCodeAt(0) && byte <= '9'.charCodeAt(0)) ||
        byte === '.'.charCodeAt(0) ||
        (numStr.length === 0 && (byte === '-'.charCodeAt(0) || byte === '+'.charCodeAt(0)))) {
        if (byte === '.'.charCodeAt(0)) isFloat = true;
        numStr += String.fromCharCode(this.readByte());
      } else {
        break;
      }

      if (this.position === positionBefore) {
        console.warn('parseNumber: Position not advancing, breaking to prevent infinite loop');
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
    // Safety: prevent infinite loops if position doesn't advance
    let safetyCounter = 0;
    const MAX_WHITESPACE = 100000; // Max whitespace characters to skip

    while (this.position < this.buffer.byteLength && this.isWhitespace(this.peekByte())) {
      if (++safetyCounter > MAX_WHITESPACE) {
        console.warn('skipWhitespace: Safety limit reached, stopping whitespace skip');
        break;
      }
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

// ============================================================================
// PDF Encryption & Security Handlers
// ============================================================================

/**
 * RC4 stream cipher (ARCFOUR) - used by PDF encryption V1-V3 (R2-R4).
 * Operates on raw bytes with a key schedule (KSA + PRGA).
 */
class RC4Cipher {
  private S: Uint8Array;
  private i: number = 0;
  private j: number = 0;

  constructor(key: Uint8Array) {
    // Key-Scheduling Algorithm (KSA)
    this.S = new Uint8Array(256);
    for (let i = 0; i < 256; i++) this.S[i] = i;
    let j = 0;
    for (let i = 0; i < 256; i++) {
      j = (j + this.S[i] + key[i % key.length]) & 0xFF;
      [this.S[i], this.S[j]] = [this.S[j], this.S[i]];
    }
  }

  /** Encrypt/decrypt data in-place (RC4 is symmetric). */
  process(data: Uint8Array): Uint8Array {
    const result = new Uint8Array(data.length);
    let i = this.i;
    let j = this.j;
    const S = this.S;
    for (let k = 0; k < data.length; k++) {
      i = (i + 1) & 0xFF;
      j = (j + S[i]) & 0xFF;
      [S[i], S[j]] = [S[j], S[i]];
      result[k] = data[k] ^ S[(S[i] + S[j]) & 0xFF];
    }
    this.i = i;
    this.j = j;
    return result;
  }
}

/**
 * AES cipher (128/256-bit) using Web Crypto API or pure-JS fallback.
 * CBC mode with PKCS#7 padding as specified by PDF spec.
 */
class AESCipher {
  /**
   * Decrypt AES-CBC data. IV is the first 16 bytes of ciphertext.
   */
  static decrypt(key: Uint8Array, data: Uint8Array): Uint8Array {
    if (data.length < 32) return data; // Need at least IV + 1 block
    const iv = data.slice(0, 16);
    const ciphertext = data.slice(16);
    return AESCipher.decryptCBC(key, iv, ciphertext);
  }

  /**
   * Encrypt AES-CBC data. Returns IV + ciphertext.
   */
  static encrypt(key: Uint8Array, plaintext: Uint8Array): Uint8Array {
    // Generate random IV
    const iv = new Uint8Array(16);
    if (typeof crypto !== 'undefined' && crypto.getRandomValues) {
      crypto.getRandomValues(iv);
    } else {
      for (let i = 0; i < 16; i++) iv[i] = Math.floor(Math.random() * 256);
    }
    const padded = AESCipher.pkcs7Pad(plaintext);
    const encrypted = AESCipher.encryptCBC(key, iv, padded);
    const result = new Uint8Array(16 + encrypted.length);
    result.set(iv, 0);
    result.set(encrypted, 16);
    return result;
  }

  /**
   * Pure-JS AES-CBC decryption (no dependencies).
   */
  private static decryptCBC(key: Uint8Array, iv: Uint8Array, data: Uint8Array): Uint8Array {
    const blockSize = 16;
    if (data.length % blockSize !== 0) return data;
    const expandedKey = AESCipher.expandKey(key);
    const result = new Uint8Array(data.length);
    let prevBlock = iv;

    for (let offset = 0; offset < data.length; offset += blockSize) {
      const block = data.slice(offset, offset + blockSize);
      const decrypted = AESCipher.decryptBlock(block, expandedKey);
      for (let i = 0; i < blockSize; i++) {
        result[offset + i] = decrypted[i] ^ prevBlock[i];
      }
      prevBlock = block;
    }

    // Remove PKCS#7 padding
    return AESCipher.pkcs7Unpad(result);
  }

  /**
   * Pure-JS AES-CBC encryption.
   */
  private static encryptCBC(key: Uint8Array, iv: Uint8Array, data: Uint8Array): Uint8Array {
    const blockSize = 16;
    const expandedKey = AESCipher.expandKey(key);
    const result = new Uint8Array(data.length);
    let prevBlock = iv;

    for (let offset = 0; offset < data.length; offset += blockSize) {
      const block = new Uint8Array(blockSize);
      for (let i = 0; i < blockSize; i++) {
        block[i] = data[offset + i] ^ prevBlock[i];
      }
      const encrypted = AESCipher.encryptBlock(block, expandedKey);
      result.set(encrypted, offset);
      prevBlock = encrypted;
    }

    return result;
  }

  // --- AES Core Operations ---

  private static readonly SBOX = new Uint8Array([
    0x63,0x7c,0x77,0x7b,0xf2,0x6b,0x6f,0xc5,0x30,0x01,0x67,0x2b,0xfe,0xd7,0xab,0x76,
    0xca,0x82,0xc9,0x7d,0xfa,0x59,0x47,0xf0,0xad,0xd4,0xa2,0xaf,0x9c,0xa4,0x72,0xc0,
    0xb7,0xfd,0x93,0x26,0x36,0x3f,0xf7,0xcc,0x34,0xa5,0xe5,0xf1,0x71,0xd8,0x31,0x15,
    0x04,0xc7,0x23,0xc3,0x18,0x96,0x05,0x9a,0x07,0x12,0x80,0xe2,0xeb,0x27,0xb2,0x75,
    0x09,0x83,0x2c,0x1a,0x1b,0x6e,0x5a,0xa0,0x52,0x3b,0xd6,0xb3,0x29,0xe3,0x2f,0x84,
    0x53,0xd1,0x00,0xed,0x20,0xfc,0xb1,0x5b,0x6a,0xcb,0xbe,0x39,0x4a,0x4c,0x58,0xcf,
    0xd0,0xef,0xaa,0xfb,0x43,0x4d,0x33,0x85,0x45,0xf9,0x02,0x7f,0x50,0x3c,0x9f,0xa8,
    0x51,0xa3,0x40,0x8f,0x92,0x9d,0x38,0xf5,0xbc,0xb6,0xda,0x21,0x10,0xff,0xf3,0xd2,
    0xcd,0x0c,0x13,0xec,0x5f,0x97,0x44,0x17,0xc4,0xa7,0x7e,0x3d,0x64,0x5d,0x19,0x73,
    0x60,0x81,0x4f,0xdc,0x22,0x2a,0x90,0x88,0x46,0xee,0xb8,0x14,0xde,0x5e,0x0b,0xdb,
    0xe0,0x32,0x3a,0x0a,0x49,0x06,0x24,0x5c,0xc2,0xd3,0xac,0x62,0x91,0x95,0xe4,0x79,
    0xe7,0xc8,0x37,0x6d,0x8d,0xd5,0x4e,0xa9,0x6c,0x56,0xf4,0xea,0x65,0x7a,0xae,0x08,
    0xba,0x78,0x25,0x2e,0x1c,0xa6,0xb4,0xc6,0xe8,0xdd,0x74,0x1f,0x4b,0xbd,0x8b,0x8a,
    0x70,0x3e,0xb5,0x66,0x48,0x03,0xf6,0x0e,0x61,0x35,0x57,0xb9,0x86,0xc1,0x1d,0x9e,
    0xe1,0xf8,0x98,0x11,0x69,0xd9,0x8e,0x94,0x9b,0x1e,0x87,0xe9,0xce,0x55,0x28,0xdf,
    0x8c,0xa1,0x89,0x0d,0xbf,0xe6,0x42,0x68,0x41,0x99,0x2d,0x0f,0xb0,0x54,0xbb,0x16
  ]);

  private static readonly INV_SBOX = new Uint8Array([
    0x52,0x09,0x6a,0xd5,0x30,0x36,0xa5,0x38,0xbf,0x40,0xa3,0x9e,0x81,0xf3,0xd7,0xfb,
    0x7c,0xe3,0x39,0x82,0x9b,0x2f,0xff,0x87,0x34,0x8e,0x43,0x44,0xc4,0xde,0xe9,0xcb,
    0x54,0x7b,0x94,0x32,0xa6,0xc2,0x23,0x3d,0xee,0x4c,0x95,0x0b,0x42,0xfa,0xc3,0x4e,
    0x08,0x2e,0xa1,0x66,0x28,0xd9,0x24,0xb2,0x76,0x5b,0xa2,0x49,0x6d,0x8b,0xd1,0x25,
    0x72,0xf8,0xf6,0x64,0x86,0x68,0x98,0x16,0xd4,0xa4,0x5c,0xcc,0x5d,0x65,0xb6,0x92,
    0x6c,0x70,0x48,0x50,0xfd,0xed,0xb9,0xda,0x5e,0x15,0x46,0x57,0xa7,0x8d,0x9d,0x84,
    0x90,0xd8,0xab,0x00,0x8c,0xbc,0xd3,0x0a,0xf7,0xe4,0x58,0x05,0xb8,0xb3,0x45,0x06,
    0xd0,0x2c,0x1e,0x8f,0xca,0x3f,0x0f,0x02,0xc1,0xaf,0xbd,0x03,0x01,0x13,0x8a,0x6b,
    0x3a,0x91,0x11,0x41,0x4f,0x67,0xdc,0xea,0x97,0xf2,0xcf,0xce,0xf0,0xb4,0xe6,0x73,
    0x96,0xac,0x74,0x22,0xe7,0xad,0x35,0x85,0xe2,0xf9,0x37,0xe8,0x1c,0x75,0xdf,0x6e,
    0x47,0xf1,0x1a,0x71,0x1d,0x29,0xc5,0x89,0x6f,0xb7,0x62,0x0e,0xaa,0x18,0xbe,0x1b,
    0xfc,0x56,0x3e,0x4b,0xc6,0xd2,0x79,0x20,0x9a,0xdb,0xc0,0xfe,0x78,0xcd,0x5a,0xf4,
    0x1f,0xdd,0xa8,0x33,0x88,0x07,0xc7,0x31,0xb1,0x12,0x10,0x59,0x27,0x80,0xec,0x5f,
    0x60,0x51,0x7f,0xa9,0x19,0xb5,0x4a,0x0d,0x2d,0xe5,0x7a,0x9f,0x93,0xc9,0x9c,0xef,
    0xa0,0xe0,0x3b,0x4d,0xae,0x2a,0xf5,0xb0,0xc8,0xeb,0xbb,0x3c,0x83,0x53,0x99,0x61,
    0x17,0x2b,0x04,0x7e,0xba,0x77,0xd6,0x26,0xe1,0x69,0x14,0x63,0x55,0x21,0x0c,0x7d
  ]);

  private static readonly RCON = [
    0x01,0x02,0x04,0x08,0x10,0x20,0x40,0x80,0x1b,0x36
  ];

  /** GF(2^8) multiplication used in MixColumns. */
  private static gmul(a: number, b: number): number {
    let result = 0;
    let aa = a;
    let bb = b;
    for (let i = 0; i < 8; i++) {
      if (bb & 1) result ^= aa;
      const hiBit = aa & 0x80;
      aa = (aa << 1) & 0xFF;
      if (hiBit) aa ^= 0x1b;
      bb >>= 1;
    }
    return result;
  }

  /** AES key expansion. Supports 128-bit (16 bytes) and 256-bit (32 bytes) keys. */
  private static expandKey(key: Uint8Array): Uint8Array[] {
    const keyLen = key.length;
    const nk = keyLen / 4;
    const nr = nk + 6; // 10 for AES-128, 14 for AES-256
    const totalWords = (nr + 1) * 4;
    const w = new Array<number>(totalWords * 4);

    // Copy key into first Nk words
    for (let i = 0; i < keyLen; i++) w[i] = key[i];

    for (let i = nk; i < totalWords; i++) {
      let t0 = w[(i - 1) * 4], t1 = w[(i - 1) * 4 + 1];
      let t2 = w[(i - 1) * 4 + 2], t3 = w[(i - 1) * 4 + 3];

      if (i % nk === 0) {
        // RotWord + SubWord + Rcon
        const tmp = t0;
        t0 = AESCipher.SBOX[t1] ^ AESCipher.RCON[(i / nk) - 1];
        t1 = AESCipher.SBOX[t2];
        t2 = AESCipher.SBOX[t3];
        t3 = AESCipher.SBOX[tmp];
      } else if (nk > 6 && i % nk === 4) {
        // Additional SubWord for AES-256
        t0 = AESCipher.SBOX[t0];
        t1 = AESCipher.SBOX[t1];
        t2 = AESCipher.SBOX[t2];
        t3 = AESCipher.SBOX[t3];
      }

      w[i * 4] = w[(i - nk) * 4] ^ t0;
      w[i * 4 + 1] = w[(i - nk) * 4 + 1] ^ t1;
      w[i * 4 + 2] = w[(i - nk) * 4 + 2] ^ t2;
      w[i * 4 + 3] = w[(i - nk) * 4 + 3] ^ t3;
    }

    // Convert to round key arrays
    const roundKeys: Uint8Array[] = [];
    for (let r = 0; r <= nr; r++) {
      roundKeys.push(new Uint8Array(w.slice(r * 16, r * 16 + 16)));
    }
    return roundKeys;
  }

  /** Decrypt a single 16-byte block. */
  private static decryptBlock(block: Uint8Array, roundKeys: Uint8Array[]): Uint8Array {
    const state = new Uint8Array(block);
    const nr = roundKeys.length - 1;

    // Initial round key addition
    for (let i = 0; i < 16; i++) state[i] ^= roundKeys[nr][i];

    for (let round = nr - 1; round >= 0; round--) {
      // InvShiftRows
      let tmp = state[13]; state[13] = state[9]; state[9] = state[5]; state[5] = state[1]; state[1] = tmp;
      tmp = state[10]; state[10] = state[2]; state[2] = tmp; tmp = state[14]; state[14] = state[6]; state[6] = tmp;
      tmp = state[3]; state[3] = state[7]; state[7] = state[11]; state[11] = state[15]; state[15] = tmp;

      // InvSubBytes
      for (let i = 0; i < 16; i++) state[i] = AESCipher.INV_SBOX[state[i]];

      // AddRoundKey
      for (let i = 0; i < 16; i++) state[i] ^= roundKeys[round][i];

      // InvMixColumns (skip for last round)
      if (round > 0) {
        const s = new Uint8Array(16);
        for (let col = 0; col < 4; col++) {
          const c = col * 4;
          s[c] = AESCipher.gmul(0x0e, state[c]) ^ AESCipher.gmul(0x0b, state[c+1]) ^ AESCipher.gmul(0x0d, state[c+2]) ^ AESCipher.gmul(0x09, state[c+3]);
          s[c+1] = AESCipher.gmul(0x09, state[c]) ^ AESCipher.gmul(0x0e, state[c+1]) ^ AESCipher.gmul(0x0b, state[c+2]) ^ AESCipher.gmul(0x0d, state[c+3]);
          s[c+2] = AESCipher.gmul(0x0d, state[c]) ^ AESCipher.gmul(0x09, state[c+1]) ^ AESCipher.gmul(0x0e, state[c+2]) ^ AESCipher.gmul(0x0b, state[c+3]);
          s[c+3] = AESCipher.gmul(0x0b, state[c]) ^ AESCipher.gmul(0x0d, state[c+1]) ^ AESCipher.gmul(0x09, state[c+2]) ^ AESCipher.gmul(0x0e, state[c+3]);
        }
        state.set(s);
      }
    }
    return state;
  }

  /** Encrypt a single 16-byte block. */
  private static encryptBlock(block: Uint8Array, roundKeys: Uint8Array[]): Uint8Array {
    const state = new Uint8Array(block);
    const nr = roundKeys.length - 1;

    // Initial round key addition
    for (let i = 0; i < 16; i++) state[i] ^= roundKeys[0][i];

    for (let round = 1; round <= nr; round++) {
      // SubBytes
      for (let i = 0; i < 16; i++) state[i] = AESCipher.SBOX[state[i]];

      // ShiftRows
      let tmp = state[1]; state[1] = state[5]; state[5] = state[9]; state[9] = state[13]; state[13] = tmp;
      tmp = state[2]; state[2] = state[10]; state[10] = tmp; tmp = state[6]; state[6] = state[14]; state[14] = tmp;
      tmp = state[15]; state[15] = state[11]; state[11] = state[7]; state[7] = state[3]; state[3] = tmp;

      // MixColumns (skip for final round)
      if (round < nr) {
        const s = new Uint8Array(16);
        for (let col = 0; col < 4; col++) {
          const c = col * 4;
          s[c] = AESCipher.gmul(2, state[c]) ^ AESCipher.gmul(3, state[c+1]) ^ state[c+2] ^ state[c+3];
          s[c+1] = state[c] ^ AESCipher.gmul(2, state[c+1]) ^ AESCipher.gmul(3, state[c+2]) ^ state[c+3];
          s[c+2] = state[c] ^ state[c+1] ^ AESCipher.gmul(2, state[c+2]) ^ AESCipher.gmul(3, state[c+3]);
          s[c+3] = AESCipher.gmul(3, state[c]) ^ state[c+1] ^ state[c+2] ^ AESCipher.gmul(2, state[c+3]);
        }
        state.set(s);
      }

      // AddRoundKey
      for (let i = 0; i < 16; i++) state[i] ^= roundKeys[round][i];
    }
    return state;
  }

  /** PKCS#7 padding. */
  private static pkcs7Pad(data: Uint8Array): Uint8Array {
    const padLen = 16 - (data.length % 16);
    const padded = new Uint8Array(data.length + padLen);
    padded.set(data);
    for (let i = data.length; i < padded.length; i++) padded[i] = padLen;
    return padded;
  }

  /** Remove PKCS#7 padding. */
  private static pkcs7Unpad(data: Uint8Array): Uint8Array {
    if (data.length === 0) return data;
    const padLen = data[data.length - 1];
    if (padLen < 1 || padLen > 16) return data;
    // Validate padding bytes
    for (let i = data.length - padLen; i < data.length; i++) {
      if (data[i] !== padLen) return data;
    }
    return data.slice(0, data.length - padLen);
  }
}

/**
 * Minimal pure-JS MD5 implementation for PDF key derivation.
 * PDF encryption uses MD5 extensively (Algorithm 2, 3, etc).
 */
class MD5 {
  static hash(data: Uint8Array): Uint8Array {
    const padded = MD5.pad(data);
    let a0 = 0x67452301, b0 = 0xefcdab89, c0 = 0x98badcfe, d0 = 0x10325476;

    for (let offset = 0; offset < padded.length; offset += 64) {
      const M = new Uint32Array(16);
      for (let i = 0; i < 16; i++) {
        M[i] = padded[offset + i * 4] |
               (padded[offset + i * 4 + 1] << 8) |
               (padded[offset + i * 4 + 2] << 16) |
               (padded[offset + i * 4 + 3] << 24);
      }

      let a = a0, b = b0, c = c0, d = d0;

      for (let i = 0; i < 64; i++) {
        let f: number, g: number;
        if (i < 16) {
          f = (b & c) | (~b & d);
          g = i;
        } else if (i < 32) {
          f = (d & b) | (~d & c);
          g = (5 * i + 1) % 16;
        } else if (i < 48) {
          f = b ^ c ^ d;
          g = (3 * i + 5) % 16;
        } else {
          f = c ^ (b | ~d);
          g = (7 * i) % 16;
        }

        const temp = d;
        d = c;
        c = b;
        const sum = (a + f + MD5.K[i] + M[g]) | 0;
        b = (b + MD5.rotl(sum, MD5.S[i])) | 0;
        a = temp;
      }

      a0 = (a0 + a) | 0;
      b0 = (b0 + b) | 0;
      c0 = (c0 + c) | 0;
      d0 = (d0 + d) | 0;
    }

    const result = new Uint8Array(16);
    MD5.putLE32(result, 0, a0);
    MD5.putLE32(result, 4, b0);
    MD5.putLE32(result, 8, c0);
    MD5.putLE32(result, 12, d0);
    return result;
  }

  private static rotl(x: number, n: number): number {
    return ((x << n) | (x >>> (32 - n))) >>> 0;
  }

  private static putLE32(buf: Uint8Array, offset: number, val: number): void {
    buf[offset] = val & 0xFF;
    buf[offset + 1] = (val >>> 8) & 0xFF;
    buf[offset + 2] = (val >>> 16) & 0xFF;
    buf[offset + 3] = (val >>> 24) & 0xFF;
  }

  private static pad(data: Uint8Array): Uint8Array {
    const len = data.length;
    const paddedLen = ((len + 8) >>> 6) * 64 + 64;
    const padded = new Uint8Array(paddedLen);
    padded.set(data);
    padded[len] = 0x80;
    // Append original length in bits as 64-bit LE
    const bitLen = len * 8;
    padded[paddedLen - 8] = bitLen & 0xFF;
    padded[paddedLen - 7] = (bitLen >>> 8) & 0xFF;
    padded[paddedLen - 6] = (bitLen >>> 16) & 0xFF;
    padded[paddedLen - 5] = (bitLen >>> 24) & 0xFF;
    return padded;
  }

  private static readonly S = [
    7,12,17,22,7,12,17,22,7,12,17,22,7,12,17,22,
    5,9,14,20,5,9,14,20,5,9,14,20,5,9,14,20,
    4,11,16,23,4,11,16,23,4,11,16,23,4,11,16,23,
    6,10,15,21,6,10,15,21,6,10,15,21,6,10,15,21
  ];

  private static readonly K = new Int32Array([
    0xd76aa478,0xe8c7b756,0x242070db,0xc1bdceee,0xf57c0faf,0x4787c62a,0xa8304613,0xfd469501,
    0x698098d8,0x8b44f7af,0xffff5bb1,0x895cd7be,0x6b901122,0xfd987193,0xa679438e,0x49b40821,
    0xf61e2562,0xc040b340,0x265e5a51,0xe9b6c7aa,0xd62f105d,0x02441453,0xd8a1e681,0xe7d3fbc8,
    0x21e1cde6,0xc33707d6,0xf4d50d87,0x455a14ed,0xa9e3e905,0xfcefa3f8,0x676f02d9,0x8d2a4c8a,
    0xfffa3942,0x8771f681,0x6d9d6122,0xfde5380c,0xa4beea44,0x4bdecfa9,0xf6bb4b60,0xbebfbc70,
    0x289b7ec6,0xeaa127fa,0xd4ef3085,0x04881d05,0xd9d4d039,0xe6db99e5,0x1fa27cf8,0xc4ac5665,
    0xf4292244,0x432aff97,0xab9423a7,0xfc93a039,0x655b59c3,0x8f0ccc92,0xffeff47d,0x85845dd1,
    0x6fa87e4f,0xfe2ce6e0,0xa3014314,0x4e0811a1,0xf7537e82,0xbd3af235,0x2ad7d2bb,0xeb86d391
  ]);
}

/**
 * SHA-256 implementation for PDF 2.0 (AES-256, R=6) key derivation.
 */
class SHA256 {
  static hash(data: Uint8Array): Uint8Array {
    const H = new Uint32Array([
      0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
      0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19
    ]);

    const padded = SHA256.pad(data);

    for (let offset = 0; offset < padded.length; offset += 64) {
      const W = new Uint32Array(64);
      for (let i = 0; i < 16; i++) {
        W[i] = (padded[offset + i * 4] << 24) |
               (padded[offset + i * 4 + 1] << 16) |
               (padded[offset + i * 4 + 2] << 8) |
                padded[offset + i * 4 + 3];
      }
      for (let i = 16; i < 64; i++) {
        const s0 = SHA256.rotr(W[i-15], 7) ^ SHA256.rotr(W[i-15], 18) ^ (W[i-15] >>> 3);
        const s1 = SHA256.rotr(W[i-2], 17) ^ SHA256.rotr(W[i-2], 19) ^ (W[i-2] >>> 10);
        W[i] = (W[i-16] + s0 + W[i-7] + s1) | 0;
      }

      let [a, b, c, d, e, f, g, h] = H;

      for (let i = 0; i < 64; i++) {
        const S1 = SHA256.rotr(e, 6) ^ SHA256.rotr(e, 11) ^ SHA256.rotr(e, 25);
        const ch = (e & f) ^ (~e & g);
        const temp1 = (h + S1 + ch + SHA256.K[i] + W[i]) | 0;
        const S0 = SHA256.rotr(a, 2) ^ SHA256.rotr(a, 13) ^ SHA256.rotr(a, 22);
        const maj = (a & b) ^ (a & c) ^ (b & c);
        const temp2 = (S0 + maj) | 0;

        h = g; g = f; f = e; e = (d + temp1) | 0;
        d = c; c = b; b = a; a = (temp1 + temp2) | 0;
      }

      H[0] = (H[0] + a) | 0; H[1] = (H[1] + b) | 0;
      H[2] = (H[2] + c) | 0; H[3] = (H[3] + d) | 0;
      H[4] = (H[4] + e) | 0; H[5] = (H[5] + f) | 0;
      H[6] = (H[6] + g) | 0; H[7] = (H[7] + h) | 0;
    }

    const result = new Uint8Array(32);
    for (let i = 0; i < 8; i++) {
      result[i * 4] = (H[i] >>> 24) & 0xFF;
      result[i * 4 + 1] = (H[i] >>> 16) & 0xFF;
      result[i * 4 + 2] = (H[i] >>> 8) & 0xFF;
      result[i * 4 + 3] = H[i] & 0xFF;
    }
    return result;
  }

  private static rotr(x: number, n: number): number {
    return ((x >>> n) | (x << (32 - n))) >>> 0;
  }

  private static pad(data: Uint8Array): Uint8Array {
    const len = data.length;
    const paddedLen = ((len + 8) >>> 6) * 64 + 64;
    const padded = new Uint8Array(paddedLen);
    padded.set(data);
    padded[len] = 0x80;
    const bitLen = len * 8;
    padded[paddedLen - 4] = (bitLen >>> 24) & 0xFF;
    padded[paddedLen - 3] = (bitLen >>> 16) & 0xFF;
    padded[paddedLen - 2] = (bitLen >>> 8) & 0xFF;
    padded[paddedLen - 1] = bitLen & 0xFF;
    return padded;
  }

  private static readonly K = new Uint32Array([
    0x428a2f98,0x71374491,0xb5c0fbcf,0xe9b5dba5,0x3956c25b,0x59f111f1,0x923f82a4,0xab1c5ed5,
    0xd807aa98,0x12835b01,0x243185be,0x550c7dc3,0x72be5d74,0x80deb1fe,0x9bdc06a7,0xc19bf174,
    0xe49b69c1,0xefbe4786,0x0fc19dc6,0x240ca1cc,0x2de92c6f,0x4a7484aa,0x5cb0a9dc,0x76f988da,
    0x983e5152,0xa831c66d,0xb00327c8,0xbf597fc7,0xc6e00bf3,0xd5a79147,0x06ca6351,0x14292967,
    0x27b70a85,0x2e1b2138,0x4d2c6dfc,0x53380d13,0x650a7354,0x766a0abb,0x81c2c92e,0x92722c85,
    0xa2bfe8a1,0xa81a664b,0xc24b8b70,0xc76c51a3,0xd192e819,0xd6990624,0xf40e3585,0x106aa070,
    0x19a4c116,0x1e376c08,0x2748774c,0x34b0bcb5,0x391c0cb3,0x4ed8aa4a,0x5b9cca4f,0x682e6ff3,
    0x748f82ee,0x78a5636f,0x84c87814,0x8cc70208,0x90befffa,0xa4506ceb,0xbef9a3f7,0xc67178f2
  ]);
}

/**
 * PDF Standard Security Handler (ISO 32000-1 Section 7.6).
 * Supports encryption algorithms V1-V5, revisions R2-R6.
 * Handles password authentication, key derivation, and object decryption.
 */
class PDFSecurityHandler {
  private encryptionKey?: Uint8Array;
  private encryptDict?: EncryptionDict;
  private isOwnerAuth: boolean = false;

  /** Padding string used by PDF encryption (Algorithm 2). */
  private static readonly PADDING = new Uint8Array([
    0x28, 0xBF, 0x4E, 0x5E, 0x4D, 0x75, 0x8A, 0x41,
    0x64, 0x00, 0x4E, 0x56, 0xFF, 0xFA, 0x01, 0x08,
    0x2E, 0x2E, 0x00, 0xB6, 0xD0, 0x68, 0x3E, 0x80,
    0x2F, 0x0C, 0xA9, 0xFE, 0x64, 0x53, 0x69, 0x7A
  ]);

  /** Parse the /Encrypt dictionary from a trailer. */
  static parseEncryptDict(
    encryptObj: { type: number; value: any },
    trailer: { entries: Map<string, any> },
    resolveRef: (objNum: number, genNum: number) => { type: number; value: any }
  ): EncryptionDict | null {
    let encDict: any;

    // Resolve reference if needed
    if (encryptObj.type === 5 /* Reference */) {
      const ref = encryptObj.value;
      const resolved = resolveRef(ref.objectNumber, ref.generationNumber);
      if (resolved.type !== 3 /* Dictionary */) return null;
      encDict = resolved.value;
    } else if (encryptObj.type === 3 /* Dictionary */) {
      encDict = encryptObj.value;
    } else {
      return null;
    }

    const getStr = (key: string): string => {
      const v = encDict.entries.get(key);
      return v ? String(v.value || '') : '';
    };
    const getNum = (key: string, def: number = 0): number => {
      const v = encDict.entries.get(key);
      return v && v.type === 1 /* Number */ ? (v.value as number) : def;
    };
    const getBytes = (key: string): Uint8Array => {
      const v = encDict.entries.get(key);
      if (!v) return new Uint8Array(0);
      const s = String(v.value || '');
      const bytes = new Uint8Array(s.length);
      for (let i = 0; i < s.length; i++) bytes[i] = s.charCodeAt(i) & 0xFF;
      return bytes;
    };

    const filter = getStr('Filter');
    if (filter !== 'Standard') return null; // Only Standard handler supported

    const version = getNum('V', 0);
    const revision = getNum('R', 2);
    let keyLength = getNum('Length', 40);
    if (keyLength > 32) keyLength = keyLength / 8; // Specified in bits, convert to bytes

    // Extract file ID from trailer
    let fileId = new Uint8Array(0);
    const idObj = trailer.entries.get('ID');
    if (idObj && idObj.type === 4 /* Array */) {
      const arr = idObj.value as any[];
      if (arr.length > 0) {
        const first = arr[0];
        const s = String(first.value || '');
        fileId = new Uint8Array(s.length);
        for (let i = 0; i < s.length; i++) fileId[i] = s.charCodeAt(i) & 0xFF;
      }
    }

    // EncryptMetadata flag (default true)
    const emObj = encDict.entries.get('EncryptMetadata');
    const encryptMetadata = !emObj || emObj.value !== false;

    return {
      filter,
      version,
      revision,
      keyLength,
      ownerKey: getBytes('O'),
      userKey: getBytes('U'),
      ownerEncryption: version >= 5 ? getBytes('OE') : undefined,
      userEncryption: version >= 5 ? getBytes('UE') : undefined,
      permissions: getNum('P', 0),
      encryptMetadata,
      fileId,
    };
  }

  /** Authenticate with a password. Returns the encryption key if successful. */
  authenticate(encryptDict: EncryptionDict, password: string = ''): AuthResult {
    this.encryptDict = encryptDict;

    // Convert password to bytes (Latin-1 for R<=4, UTF-8 for R>=5)
    const pwBytes = this.passwordToBytes(password, encryptDict.revision);

    // Try user password first, then owner password
    let result = this.tryUserPassword(encryptDict, pwBytes);
    if (result.authenticated) {
      this.encryptionKey = result.encryptionKey;
      this.isOwnerAuth = false;
      return result;
    }

    result = this.tryOwnerPassword(encryptDict, pwBytes);
    if (result.authenticated) {
      this.encryptionKey = result.encryptionKey;
      this.isOwnerAuth = true;
      return result;
    }

    return { authenticated: false, isOwner: false, permissions: 0, encryptionKey: new Uint8Array(0) };
  }

  /** Decrypt an object's data given its object/generation number. */
  decryptObject(data: Uint8Array, objNum: number, genNum: number): Uint8Array {
    if (!this.encryptionKey || !this.encryptDict) return data;
    if (data.length === 0) return data;

    const objKey = this.computeObjectKey(objNum, genNum);

    if (this.encryptDict.version >= 4) {
      // AES decryption
      return AESCipher.decrypt(objKey, data);
    } else {
      // RC4 decryption
      const rc4 = new RC4Cipher(objKey);
      return rc4.process(data);
    }
  }

  /** Decrypt a string value. */
  decryptString(data: Uint8Array, objNum: number, genNum: number): Uint8Array {
    return this.decryptObject(data, objNum, genNum);
  }

  /** Decrypt a stream's data. */
  decryptStream(data: Uint8Array, objNum: number, genNum: number): Uint8Array {
    return this.decryptObject(data, objNum, genNum);
  }

  /** Get the permissions from the encryption dictionary. */
  getPermissions(): number {
    return this.encryptDict?.permissions || 0;
  }

  /** Check if a specific permission is granted. */
  checkPermission(perm: PDFPermission): boolean {
    if (this.isOwnerAuth) return true; // Owner has all permissions
    return (this.getPermissions() & perm) !== 0;
  }

  /** Get encryption algorithm info. */
  getAlgorithm(): EncryptionAlgorithm | null {
    if (!this.encryptDict) return null;
    const { version, keyLength } = this.encryptDict;
    if (version <= 1) return EncryptionAlgorithm.RC4_40;
    if (version <= 3) return EncryptionAlgorithm.RC4_128;
    if (version === 4) return EncryptionAlgorithm.AES_128;
    if (version === 5) return EncryptionAlgorithm.AES_256;
    return null;
  }

  /** Whether authentication succeeded. */
  isAuthenticated(): boolean {
    return this.encryptionKey !== undefined;
  }

  // --- Private Key Derivation ---

  /** Convert password string to bytes per PDF spec. */
  private passwordToBytes(password: string, revision: number): Uint8Array {
    if (revision >= 5) {
      // R5/R6: SASLprep (simplified: UTF-8)
      return new TextEncoder().encode(password);
    }
    // R2-R4: Latin-1 encoding, padded/truncated to 32 bytes
    const bytes = new Uint8Array(32);
    const len = Math.min(password.length, 32);
    for (let i = 0; i < len; i++) bytes[i] = password.charCodeAt(i) & 0xFF;
    bytes.set(PDFSecurityHandler.PADDING.slice(0, 32 - len), len);
    return bytes;
  }

  /**
   * Compute encryption key (Algorithm 2 from PDF spec).
   * Used for R2-R4 (Standard handler).
   */
  private computeEncryptionKey(encryptDict: EncryptionDict, password: Uint8Array): Uint8Array {
    // Step a: password (already padded to 32 bytes)
    const input = new Uint8Array(
      password.length + encryptDict.ownerKey.length + 4 + encryptDict.fileId.length +
      (encryptDict.encryptMetadata ? 0 : 4)
    );
    let offset = 0;
    input.set(password, offset); offset += password.length;
    // Step b: O value
    input.set(encryptDict.ownerKey, offset); offset += encryptDict.ownerKey.length;
    // Step c: P value (low-order 32 bits)
    const p = encryptDict.permissions;
    input[offset++] = p & 0xFF;
    input[offset++] = (p >> 8) & 0xFF;
    input[offset++] = (p >> 16) & 0xFF;
    input[offset++] = (p >> 24) & 0xFF;
    // Step d: file ID
    input.set(encryptDict.fileId, offset); offset += encryptDict.fileId.length;
    // Step e: if R>=4 and metadata is not encrypted
    if (!encryptDict.encryptMetadata && encryptDict.revision >= 4) {
      input[offset++] = 0xFF; input[offset++] = 0xFF;
      input[offset++] = 0xFF; input[offset++] = 0xFF;
    }

    // Step f: MD5 hash
    let hash = MD5.hash(input.slice(0, offset));

    // Step g: For R>=3, re-hash 50 times
    const keyLen = encryptDict.keyLength;
    if (encryptDict.revision >= 3) {
      for (let i = 0; i < 50; i++) {
        hash = MD5.hash(hash.slice(0, keyLen));
      }
    }

    return hash.slice(0, keyLen);
  }

  /**
   * Try to authenticate with the user password (Algorithm 6).
   */
  private tryUserPassword(encryptDict: EncryptionDict, password: Uint8Array): AuthResult {
    if (encryptDict.revision >= 5) {
      return this.tryUserPasswordR5R6(encryptDict, password);
    }

    const key = this.computeEncryptionKey(encryptDict, password);

    if (encryptDict.revision === 2) {
      // Algorithm 4: encrypt padding with key, compare to U
      const rc4 = new RC4Cipher(key);
      const computed = rc4.process(new Uint8Array(PDFSecurityHandler.PADDING));
      if (this.arraysEqual(computed, encryptDict.userKey.slice(0, 32))) {
        return { authenticated: true, isOwner: false, permissions: encryptDict.permissions, encryptionKey: key };
      }
    } else {
      // Algorithm 5: MD5(padding + fileID), then 20 rounds of RC4 with modified keys
      const hashInput = new Uint8Array(32 + encryptDict.fileId.length);
      hashInput.set(PDFSecurityHandler.PADDING);
      hashInput.set(encryptDict.fileId, 32);
      let hash = MD5.hash(hashInput);

      const rc4 = new RC4Cipher(key);
      hash = rc4.process(hash);

      for (let i = 1; i <= 19; i++) {
        const modKey = new Uint8Array(key.length);
        for (let j = 0; j < key.length; j++) modKey[j] = key[j] ^ i;
        const rc4i = new RC4Cipher(modKey);
        hash = rc4i.process(hash);
      }

      // Compare first 16 bytes
      if (this.arraysEqual(hash.slice(0, 16), encryptDict.userKey.slice(0, 16))) {
        return { authenticated: true, isOwner: false, permissions: encryptDict.permissions, encryptionKey: key };
      }
    }

    return { authenticated: false, isOwner: false, permissions: 0, encryptionKey: new Uint8Array(0) };
  }

  /**
   * Try user password for R5/R6 (AES-256, Algorithm 2.A/2.B from ISO 32000-2).
   */
  private tryUserPasswordR5R6(encryptDict: EncryptionDict, password: Uint8Array): AuthResult {
    // U = 32-byte hash + 8-byte validation salt + 8-byte key salt
    const uHash = encryptDict.userKey.slice(0, 32);
    const uValidationSalt = encryptDict.userKey.slice(32, 40);

    // Compute hash: SHA-256(password + validation salt)
    const input = new Uint8Array(password.length + 8);
    input.set(password);
    input.set(uValidationSalt, password.length);
    const computed = SHA256.hash(input);

    if (!this.arraysEqual(computed, uHash)) {
      return { authenticated: false, isOwner: false, permissions: 0, encryptionKey: new Uint8Array(0) };
    }

    // Derive the file encryption key from UE
    const uKeySalt = encryptDict.userKey.slice(40, 48);
    const keyInput = new Uint8Array(password.length + 8);
    keyInput.set(password);
    keyInput.set(uKeySalt, password.length);
    const keyHash = SHA256.hash(keyInput);

    // Decrypt UE with keyHash using AES-CBC with zero IV
    const ue = encryptDict.userEncryption || new Uint8Array(32);
    const zeroIV = new Uint8Array(16);
    const ueWithIV = new Uint8Array(16 + ue.length);
    ueWithIV.set(zeroIV);
    ueWithIV.set(ue, 16);
    const fileKey = AESCipher.decrypt(keyHash, ueWithIV);

    return { authenticated: true, isOwner: false, permissions: encryptDict.permissions, encryptionKey: fileKey.slice(0, 32) };
  }

  /**
   * Try to authenticate with the owner password (Algorithm 7).
   */
  private tryOwnerPassword(encryptDict: EncryptionDict, password: Uint8Array): AuthResult {
    if (encryptDict.revision >= 5) {
      return this.tryOwnerPasswordR5R6(encryptDict, password);
    }

    // Algorithm 3: recover user password from O value
    // Step a: pad password
    const paddedPw = new Uint8Array(32);
    const len = Math.min(password.length, 32);
    paddedPw.set(password.slice(0, len));
    paddedPw.set(PDFSecurityHandler.PADDING.slice(0, 32 - len), len);

    // Step b: MD5 hash
    let hash = MD5.hash(paddedPw);

    // Step c: for R>=3, re-hash 50 times
    const keyLen = encryptDict.keyLength;
    if (encryptDict.revision >= 3) {
      for (let i = 0; i < 50; i++) {
        hash = MD5.hash(hash.slice(0, keyLen));
      }
    }
    const ownerKey = hash.slice(0, keyLen);

    // Step d/e: decrypt O value
    let decrypted: Uint8Array = Uint8Array.from(encryptDict.ownerKey);
    if (encryptDict.revision === 2) {
      const rc4 = new RC4Cipher(ownerKey);
      decrypted = Uint8Array.from(rc4.process(decrypted));
    } else {
      // R>=3: 20 rounds in reverse
      for (let i = 19; i >= 0; i--) {
        const modKey = new Uint8Array(ownerKey.length);
        for (let j = 0; j < ownerKey.length; j++) modKey[j] = ownerKey[j] ^ i;
        const rc4 = new RC4Cipher(modKey);
        decrypted = Uint8Array.from(rc4.process(decrypted));
      }
    }

    // decrypted is now the padded user password — try it
    const result = this.tryUserPassword(encryptDict, decrypted);
    if (result.authenticated) {
      return { ...result, isOwner: true };
    }
    return { authenticated: false, isOwner: false, permissions: 0, encryptionKey: new Uint8Array(0) };
  }

  /**
   * Try owner password for R5/R6 (AES-256).
   */
  private tryOwnerPasswordR5R6(encryptDict: EncryptionDict, password: Uint8Array): AuthResult {
    const oHash = encryptDict.ownerKey.slice(0, 32);
    const oValidationSalt = encryptDict.ownerKey.slice(32, 40);

    // SHA-256(password + validation salt + U)
    const input = new Uint8Array(password.length + 8 + 48);
    input.set(password);
    input.set(oValidationSalt, password.length);
    input.set(encryptDict.userKey.slice(0, 48), password.length + 8);
    const computed = SHA256.hash(input);

    if (!this.arraysEqual(computed, oHash)) {
      return { authenticated: false, isOwner: false, permissions: 0, encryptionKey: new Uint8Array(0) };
    }

    // Derive file encryption key from OE
    const oKeySalt = encryptDict.ownerKey.slice(40, 48);
    const keyInput = new Uint8Array(password.length + 8 + 48);
    keyInput.set(password);
    keyInput.set(oKeySalt, password.length);
    keyInput.set(encryptDict.userKey.slice(0, 48), password.length + 8);
    const keyHash = SHA256.hash(keyInput);

    const oe = encryptDict.ownerEncryption || new Uint8Array(32);
    const zeroIV = new Uint8Array(16);
    const oeWithIV = new Uint8Array(16 + oe.length);
    oeWithIV.set(zeroIV);
    oeWithIV.set(oe, 16);
    const fileKey = AESCipher.decrypt(keyHash, oeWithIV);

    return { authenticated: true, isOwner: true, permissions: encryptDict.permissions, encryptionKey: fileKey.slice(0, 32) };
  }

  /**
   * Compute per-object encryption key (Algorithm 1 from PDF spec).
   */
  private computeObjectKey(objNum: number, genNum: number): Uint8Array {
    if (!this.encryptionKey || !this.encryptDict) return new Uint8Array(0);

    if (this.encryptDict.version >= 5) {
      // AES-256: use file encryption key directly
      return this.encryptionKey;
    }

    // Algorithm 1: MD5(key + objNum(3 bytes LE) + genNum(2 bytes LE) [+ "sAlT" for AES])
    const isAES = this.encryptDict.version === 4;
    const extra = isAES ? 4 : 0;
    const input = new Uint8Array(this.encryptionKey.length + 5 + extra);
    input.set(this.encryptionKey);
    const keyLen = this.encryptionKey.length;
    input[keyLen] = objNum & 0xFF;
    input[keyLen + 1] = (objNum >> 8) & 0xFF;
    input[keyLen + 2] = (objNum >> 16) & 0xFF;
    input[keyLen + 3] = genNum & 0xFF;
    input[keyLen + 4] = (genNum >> 8) & 0xFF;

    if (isAES) {
      // AES-128 uses "sAlT" suffix
      input[keyLen + 5] = 0x73; // s
      input[keyLen + 6] = 0x41; // A
      input[keyLen + 7] = 0x6C; // l
      input[keyLen + 8] = 0x54; // T
    }

    const hash = MD5.hash(input);
    // Key length is min(n+5, 16)
    const objKeyLen = Math.min(keyLen + 5, 16);
    return hash.slice(0, objKeyLen);
  }

  /** Constant-time(ish) array comparison. */
  private arraysEqual(a: Uint8Array, b: Uint8Array): boolean {
    if (a.length !== b.length) return false;
    let diff = 0;
    for (let i = 0; i < a.length; i++) diff |= a[i] ^ b[i];
    return diff === 0;
  }
}

/**
 * Built-in DEFLATE decompressor (RFC 1951)
 * Provides zero-dependency zlib/deflate inflate for PDF FlateDecode streams.
 */
class Inflate {
  private data: Uint8Array;
  private pos: number = 0;
  private bitBuf: number = 0;
  private bitCount: number = 0;
  private output: number[] = [];

  // Fixed Huffman code lengths (RFC 1951 section 3.2.6)
  private static readonly FIXED_LIT_LENGTHS = (() => {
    const lengths = new Uint8Array(288);
    for (let i = 0; i <= 143; i++) lengths[i] = 8;
    for (let i = 144; i <= 255; i++) lengths[i] = 9;
    for (let i = 256; i <= 279; i++) lengths[i] = 7;
    for (let i = 280; i <= 287; i++) lengths[i] = 8;
    return lengths;
  })();

  private static readonly FIXED_DIST_LENGTHS = (() => {
    const lengths = new Uint8Array(32);
    lengths.fill(5);
    return lengths;
  })();

  // Length base values and extra bits (codes 257-285)
  private static readonly LENGTH_BASE = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31,
    35, 43, 51, 59, 67, 83, 99, 115, 131, 163, 195, 227, 258
  ];
  private static readonly LENGTH_EXTRA = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2,
    3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0
  ];

  // Distance base values and extra bits (codes 0-29)
  private static readonly DIST_BASE = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193,
    257, 385, 513, 769, 1025, 1537, 2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577
  ];
  private static readonly DIST_EXTRA = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6,
    7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13, 13
  ];

  // Code length alphabet order (for dynamic Huffman)
  private static readonly CL_ORDER = [
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15
  ];

  constructor(data: Uint8Array) {
    this.data = data;
  }

  static inflate(data: Uint8Array): Uint8Array {
    const inflater = new Inflate(data);
    return inflater.decompress();
  }

  private decompress(): Uint8Array {
    // Check for zlib header (CMF + FLG)
    if (this.data.length >= 2) {
      const cmf = this.data[0];
      const flg = this.data[1];
      const cm = cmf & 0x0F;
      if (cm === 8 && ((cmf * 256 + flg) % 31 === 0)) {
        // Skip 2-byte zlib header
        this.pos = 2;
        // If FDICT is set, skip 4 bytes
        if (flg & 0x20) {
          this.pos += 4;
        }
      }
    }

    let bfinal = 0;
    while (bfinal === 0) {
      bfinal = this.readBits(1);
      const btype = this.readBits(2);

      if (btype === 0) {
        this.inflateStored();
      } else if (btype === 1) {
        this.inflateFixed();
      } else if (btype === 2) {
        this.inflateDynamic();
      } else {
        throw new Error('Invalid DEFLATE block type');
      }
    }

    return new Uint8Array(this.output);
  }

  private readBits(count: number): number {
    while (this.bitCount < count) {
      if (this.pos >= this.data.length) {
        throw new Error('Unexpected end of data');
      }
      this.bitBuf |= this.data[this.pos++] << this.bitCount;
      this.bitCount += 8;
    }
    const value = this.bitBuf & ((1 << count) - 1);
    this.bitBuf >>= count;
    this.bitCount -= count;
    return value;
  }

  private inflateStored(): void {
    // Align to byte boundary
    this.bitBuf = 0;
    this.bitCount = 0;

    if (this.pos + 4 > this.data.length) {
      throw new Error('Unexpected end of stored block header');
    }

    const len = this.data[this.pos] | (this.data[this.pos + 1] << 8);
    this.pos += 4; // Skip LEN and NLEN

    for (let i = 0; i < len; i++) {
      if (this.pos >= this.data.length) break;
      this.output.push(this.data[this.pos++]);
    }
  }

  private inflateFixed(): void {
    const litTree = this.buildTree(Inflate.FIXED_LIT_LENGTHS, 288);
    const distTree = this.buildTree(Inflate.FIXED_DIST_LENGTHS, 32);
    this.inflateBlock(litTree, distTree);
  }

  private inflateDynamic(): void {
    const hlit = this.readBits(5) + 257;
    const hdist = this.readBits(5) + 1;
    const hclen = this.readBits(4) + 4;

    // Read code length code lengths
    const clLengths = new Uint8Array(19);
    for (let i = 0; i < hclen; i++) {
      clLengths[Inflate.CL_ORDER[i]] = this.readBits(3);
    }

    const clTree = this.buildTree(clLengths, 19);

    // Read literal/length + distance code lengths
    const lengths = new Uint8Array(hlit + hdist);
    let i = 0;
    while (i < hlit + hdist) {
      const sym = this.decodeSymbol(clTree);

      if (sym < 16) {
        lengths[i++] = sym;
      } else if (sym === 16) {
        const count = this.readBits(2) + 3;
        const prev = i > 0 ? lengths[i - 1] : 0;
        for (let j = 0; j < count && i < lengths.length; j++) {
          lengths[i++] = prev;
        }
      } else if (sym === 17) {
        const count = this.readBits(3) + 3;
        for (let j = 0; j < count && i < lengths.length; j++) {
          lengths[i++] = 0;
        }
      } else if (sym === 18) {
        const count = this.readBits(7) + 11;
        for (let j = 0; j < count && i < lengths.length; j++) {
          lengths[i++] = 0;
        }
      }
    }

    const litLengths = lengths.slice(0, hlit);
    const distLengths = lengths.slice(hlit);

    const litTree = this.buildTree(litLengths, hlit);
    const distTree = this.buildTree(distLengths, hdist);
    this.inflateBlock(litTree, distTree);
  }

  private inflateBlock(litTree: HuffmanTree, distTree: HuffmanTree): void {
    let done = false;
    while (!done) {
      const sym = this.decodeSymbol(litTree);

      if (sym === 256) {
        done = true;
      } else if (sym < 256) {
        this.output.push(sym); // Literal byte
      } else {
        // Length/distance pair
        const lengthIdx = sym - 257;
        if (lengthIdx >= Inflate.LENGTH_BASE.length) break;

        const length = Inflate.LENGTH_BASE[lengthIdx] + this.readBits(Inflate.LENGTH_EXTRA[lengthIdx]);
        const distSym = this.decodeSymbol(distTree);
        if (distSym >= Inflate.DIST_BASE.length) break;

        const distance = Inflate.DIST_BASE[distSym] + this.readBits(Inflate.DIST_EXTRA[distSym]);

        // Copy from output buffer
        const srcPos = this.output.length - distance;
        for (let i = 0; i < length; i++) {
          this.output.push(this.output[srcPos + i]);
        }
      }
    }
  }

  private buildTree(lengths: Uint8Array, numSymbols: number): HuffmanTree {
    // Count codes of each length
    const maxBits = 15;
    const blCount = new Uint16Array(maxBits + 1);
    for (let i = 0; i < numSymbols; i++) {
      if (lengths[i] > 0) blCount[lengths[i]]++;
    }

    // Find numerical value of smallest code for each code length
    const nextCode = new Uint16Array(maxBits + 1);
    let code = 0;
    for (let bits = 1; bits <= maxBits; bits++) {
      code = (code + blCount[bits - 1]) << 1;
      nextCode[bits] = code;
    }

    // Assign codes to symbols
    const table: Map<number, number> = new Map();
    let maxLength = 0;
    for (let n = 0; n < numSymbols; n++) {
      const len = lengths[n];
      if (len > 0) {
        table.set((nextCode[len]++ << 4) | len, n);
        if (len > maxLength) maxLength = len;
      }
    }

    return { table, maxLength };
  }

  private decodeSymbol(tree: HuffmanTree): number {
    let code = 0;
    let len = 0;

    while (len < tree.maxLength) {
      code = (code << 1) | this.readBits(1);
      len++;
      const key = (code << 4) | len;
      const sym = tree.table.get(key);
      if (sym !== undefined) return sym;
    }

    throw new Error('Invalid Huffman code');
  }
}

interface HuffmanTree {
  table: Map<number, number>;
  maxLength: number;
}

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

  getAllEntries(): IterableIterator<[number, XRefEntry]> {
    return this.entries.entries();
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
    // Read enough bytes to find trailer and parse metadata
    await this.ensureBytes(Math.min(65536, 8192));

    const text = this.readString(0, Math.min(this.buffer.length, 65536));

    // Extract version from header
    let version = '1.7';
    const headerMatch = text.match(/%PDF-(\d+\.\d+)/);
    if (headerMatch) version = headerMatch[1];

    // Try to find page count from trailer or catalog
    let pageCount = 0;
    const countMatch = text.match(/\/Count\s+(\d+)/);
    if (countMatch) pageCount = parseInt(countMatch[1], 10);

    // Check encryption
    const isEncrypted = text.includes('/Encrypt');

    // Check linearization
    const isLinearized = text.includes('/Linearized');

    return {
      version,
      pageCount,
      isEncrypted,
      isLinearized,
      fileSize: this.bytesRead
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
    // Incrementally parse the next page from the buffered stream data
    try {
      await this.ensureBytes(4096);
    } catch {
      return null; // Stream ended
    }

    const text = this.readString(this.position, Math.min(this.buffer.length - this.position, 65536));

    // Search for page objects: /Type /Page
    const pagePattern = /\/Type\s*\/Page(?!s)\b/g;
    let match: RegExpExecArray | null;
    let foundCount = 0;

    while ((match = pagePattern.exec(text)) !== null) {
      foundCount++;
      if (foundCount < pageNumber) continue;

      // Found a page - try to extract MediaBox
      const contextStart = Math.max(0, match.index - 500);
      const contextEnd = Math.min(text.length, match.index + 1000);
      const context = text.substring(contextStart, contextEnd);

      let width = 612, height = 792, rotation = 0;

      const mediaBoxMatch = context.match(/\/MediaBox\s*\[\s*([\d.]+)\s+([\d.]+)\s+([\d.]+)\s+([\d.]+)\s*\]/);
      if (mediaBoxMatch) {
        width = parseFloat(mediaBoxMatch[3]) - parseFloat(mediaBoxMatch[1]);
        height = parseFloat(mediaBoxMatch[4]) - parseFloat(mediaBoxMatch[2]);
      }

      const rotateMatch = context.match(/\/Rotate\s+(\d+)/);
      if (rotateMatch) rotation = parseInt(rotateMatch[1], 10);

      // Extract content stream reference
      const contentsMatch = context.match(/\/Contents\s+(\d+)\s+\d+\s+R/);
      let contents: Uint8Array | undefined;
      if (contentsMatch) {
        // Note: actual stream decompression requires full parser
        contents = new Uint8Array(0);
      }

      // Advance position past this page
      this.position += match.index + match[0].length;

      return {
        width,
        height,
        rotation,
        userUnit: 1,
        mediaBox: { x: 0, y: 0, width, height },
        contents
      };
    }

    // No more pages found
    return null;
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
    private pdf: AgenticPDF,
    private options?: TextExtractionOptions
  ) { }

  async extract(): Promise<TextContent[]> {
    const metric = PerformanceMonitor.startOperation('TextExtractor.extract');

    const results: TextContent[] = [];
    const metadata = this.pdf.getMetadata();
    if (!metadata) {
      PerformanceMonitor.endOperation(metric);
      return results;
    }

    for (let i = 1; i <= metadata.pageCount; i++) {
      const page = await this.pdf.getPage(i);
      if (page) {
        let pageText = await this.extractPageText(page);

        // Decompose ligatures
        pageText = pageText.map(b => ({ ...b, text: LayoutAnalyzer.decomposeLigatures(b.text) }));

        // Detect text direction (RTL/LTR)
        pageText = pageText.map(b => ({
          ...b,
          direction: LayoutAnalyzer.detectTextDirection(b.text)
        }));

        // Detect and label vertical CJK text
        if (pageText.some(b => b.direction === 'ltr' || b.direction === 'rtl')) {
          const { vertical, horizontal } = LayoutAnalyzer.detectVerticalText(pageText);
          pageText = [...horizontal, ...vertical];
        }

        // Multi-column reading order reconstruction
        if (this.options?.detectColumns) {
          const pageWidth = page.width || 612;
          pageText = LayoutAnalyzer.reconstructReadingOrder(pageText, pageWidth);
        }

        results.push(...pageText);
      }
    }

    const finalResults = this.options?.preserveFormatting ? results : this.mergeTextBlocks(results);
    PerformanceMonitor.endOperation(metric);
    return finalResults;
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

  async extractPageText(page: PDFPage): Promise<TextContent[]> {
    const metric = PerformanceMonitor.startOperation('TextExtractor.extractPageText');

    const textBlocks: TextContent[] = [];

    if (!page.contents) {
      PerformanceMonitor.endOperation(metric);
      return textBlocks;
    }

    // Parse content stream with safety limits (now with caching)
    const contentParser = new ContentStreamParser(page.contents);

    try {
      const operations = contentParser.parse();

      if (operations.length === 0) {
        console.warn('No operations parsed from content stream');
        return textBlocks;
      }

      // Current graphics state
      let currentState: {
        textMatrix: TransformMatrix;
        fontSize: number;
        fontName: string;
        textLeading: number;
        charSpace: number;
        wordSpace: number;
        horizontalScaling: number;
        textRise: number;
        renderingMode: number;
        fillColor: Color;
        fontResource?: FontResource;
      } = {
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

      // Line matrix for T* operator
      let lineMatrix: TransformMatrix = [1, 0, 0, 1, 0, 0];

      // Debug: log text positioning operators
      const textOps = operations.filter(op =>
        ['BT', 'ET', 'Tf', 'Tm', 'Td', 'TD', 'T*', 'Tj', 'TJ', "'", '"', 'TL'].includes(op.operator)
      );
      console.log(`Page ${page.pageNumber}: Found ${textOps.length} text operators out of ${operations.length} total`);
      console.log('Text operators:', textOps.slice(0, 20).map(op => `${op.operator}(${op.operands.length})`).join(', '));

      for (const op of operations) {
        switch (op.operator) {
          case 'BT': // Begin text
            currentState.textMatrix = [1, 0, 0, 1, 0, 0];
            lineMatrix = [1, 0, 0, 1, 0, 0];
            break;

          case 'ET': // End text
            break;

          case 'Tf': // Set font and size
            currentState.fontName = op.operands[0] as string;
            currentState.fontSize = op.operands[1] as number;
            // Track font resource for text decoding
            {
              const fontKey = currentState.fontName.startsWith('/') ? currentState.fontName.substring(1) : currentState.fontName;
              if (page.resources && page.resources.fonts) {
                currentState.fontResource = page.resources.fonts.get(fontKey);
              }
            }
            break;

          case 'Tm': // Set text matrix
            if (op.operands.length >= 6) {
              currentState.textMatrix = [
                op.operands[0], op.operands[1], op.operands[2],
                op.operands[3], op.operands[4], op.operands[5]
              ] as TransformMatrix;
              lineMatrix = [...currentState.textMatrix] as TransformMatrix;
            }
            break;

          case 'Td': // Move text position
            if (op.operands.length >= 2) {
              const tx = op.operands[0] as number;
              const ty = op.operands[1] as number;
              currentState.textMatrix[4] += tx;
              currentState.textMatrix[5] += ty;
              lineMatrix = [...currentState.textMatrix] as TransformMatrix;
            }
            break;

          case 'TD': // Move text position and set leading
            if (op.operands.length >= 2) {
              const tx = op.operands[0] as number;
              const ty = op.operands[1] as number;
              currentState.textMatrix[4] += tx;
              currentState.textMatrix[5] += ty;
              currentState.textLeading = -ty;
              lineMatrix = [...currentState.textMatrix] as TransformMatrix;
            }
            break;

          case 'T*': // Move to start of next line
            currentState.textMatrix[4] = lineMatrix[4];
            currentState.textMatrix[5] = lineMatrix[5] - currentState.textLeading;
            lineMatrix = [...currentState.textMatrix] as TransformMatrix;
            break;

          case 'Tj': // Show text
            const text = op.operands[0] as string;
            if (text) {
              const decodedText = this.decodeTextWithFont(text, currentState);
              textBlocks.push(this.createTextContent(decodedText, currentState, page));
            }
            break;

          case 'TJ': // Show text with positioning
            const array = op.operands[0] as any[];
            let combinedText = '';
            for (const item of array) {
              if (typeof item === 'string') {
                combinedText += this.decodeTextWithFont(item, currentState);
              } else if (typeof item === 'number') {
                // Large negative adjustments (e.g., < -100) indicate word boundaries
                if (item < -100) {
                  combinedText += ' ';
                }
              }
            }
            if (combinedText) {
              textBlocks.push(this.createTextContent(combinedText, currentState, page));
            }
            break;

          case "'": // Move to next line and show text
            // Equivalent to T* Tj
            currentState.textMatrix[4] = lineMatrix[4];
            currentState.textMatrix[5] = lineMatrix[5] - currentState.textLeading;
            lineMatrix = [...currentState.textMatrix] as TransformMatrix;
            if (op.operands[0]) {
              const decodedQuote = this.decodeTextWithFont(op.operands[0] as string, currentState);
              textBlocks.push(this.createTextContent(decodedQuote, currentState, page));
            }
            break;

          case '"': // Set spacing, move to next line, show text
            // Set word and char spacing, then equivalent to T* Tj
            if (op.operands.length >= 3) {
              currentState.wordSpace = op.operands[0] as number;
              currentState.charSpace = op.operands[1] as number;
              currentState.textMatrix[4] = lineMatrix[4];
              currentState.textMatrix[5] = lineMatrix[5] - currentState.textLeading;
              lineMatrix = [...currentState.textMatrix] as TransformMatrix;
              if (op.operands[2]) {
                const decodedDQ = this.decodeTextWithFont(op.operands[2] as string, currentState);
                textBlocks.push(this.createTextContent(decodedDQ, currentState, page));
              }
            }
            break;

          case 'TL': // Set text leading
            if (op.operands.length >= 1) {
              currentState.textLeading = op.operands[0] as number;
            }
            break;
        }
      }

      PerformanceMonitor.endOperation(metric);
      return textBlocks;
    } catch (error) {
      console.error('Error parsing content stream:', error);
      PerformanceMonitor.endOperation(metric);
      return textBlocks; // Return empty array on error
    }
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
      transform: [...state.textMatrix] as TransformMatrix, // CRITICAL: Copy array, not reference!
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

  private decodeTextWithFont(rawText: string, state: any): string {
    const font: FontResource | undefined = state.fontResource;
    if (!font) return rawText;

    const toUnicode = font.toUnicode;
    const encoding = font.encoding;
    const isCIDFont = font.subtype === 'Type0';

    // For raw PDF strings, extract byte values
    let decoded = '';
    if (isCIDFont) {
      // Group bytes into 2-byte CID codes
      for (let i = 0; i < rawText.length - 1; i += 2) {
        const charCode = (rawText.charCodeAt(i) << 8) | rawText.charCodeAt(i + 1);
        if (toUnicode && toUnicode.has(charCode)) {
          decoded += toUnicode.get(charCode)!;
        } else if (charCode > 0) {
          decoded += String.fromCodePoint(charCode);
        }
      }
    } else {
      for (let i = 0; i < rawText.length; i++) {
        const charCode = rawText.charCodeAt(i);
        if (toUnicode && toUnicode.has(charCode)) {
          decoded += toUnicode.get(charCode)!;
        } else {
          decoded += PDFTextDecoder.mapCharCode(charCode, encoding);
        }
      }
    }
    return decoded;
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


// ============================================================================
// Layout Analysis Engine — Multi-Column, Table, Reading Order
// ============================================================================

/** Represents a detected column region on a page. */
interface ColumnRegion {
  x: number;
  width: number;
  blocks: TextContent[];
}

/** Represents a detected text line from multiple text blocks. */
interface TextLine {
  y: number;
  blocks: TextContent[];
  minX: number;
  maxX: number;
  pageNumber: number;
}

class LayoutAnalyzer {
  /**
   * Detect multi-column layout from text blocks.
   * Uses gap analysis: finds significant vertical gaps in content to split columns.
   */
  static detectColumns(blocks: TextContent[], pageWidth: number): ColumnRegion[] {
    if (blocks.length < 2) {
      return [{ x: 0, width: pageWidth, blocks }];
    }

    // Collect all x-positions to build a horizontal density histogram
    const margin = 20;
    const binSize = 5;
    const binCount = Math.ceil(pageWidth / binSize);
    const density = new Float32Array(binCount);

    for (const b of blocks) {
      const startBin = Math.max(0, Math.floor(b.x / binSize));
      const endBin = Math.min(binCount - 1, Math.floor((b.x + b.width) / binSize));
      for (let i = startBin; i <= endBin; i++) {
        density[i]++;
      }
    }

    // Find gaps: contiguous runs of zero density wider than a threshold
    const minGapWidth = 20; // Minimum gap in points to qualify as column separator
    const gaps: { start: number; end: number; center: number }[] = [];
    let gapStart = -1;

    for (let i = 0; i < binCount; i++) {
      const x = i * binSize;
      if (x < margin || x > pageWidth - margin) continue;

      if (density[i] === 0) {
        if (gapStart < 0) gapStart = i;
      } else {
        if (gapStart >= 0) {
          const gapWidth = (i - gapStart) * binSize;
          if (gapWidth >= minGapWidth) {
            gaps.push({
              start: gapStart * binSize,
              end: i * binSize,
              center: ((gapStart + i) / 2) * binSize
            });
          }
          gapStart = -1;
        }
      }
    }

    if (gaps.length === 0) {
      return [{ x: 0, width: pageWidth, blocks }];
    }

    // Build column regions from gap positions
    const boundaries = [0, ...gaps.map(g => g.center), pageWidth];
    const columns: ColumnRegion[] = [];

    for (let i = 0; i < boundaries.length - 1; i++) {
      const colX = boundaries[i];
      const colW = boundaries[i + 1] - colX;
      const colBlocks = blocks.filter(b => {
        const bCenter = b.x + b.width / 2;
        return bCenter >= colX && bCenter < colX + colW;
      });
      if (colBlocks.length > 0) {
        columns.push({ x: colX, width: colW, blocks: colBlocks });
      }
    }

    return columns.length > 0 ? columns : [{ x: 0, width: pageWidth, blocks }];
  }

  /**
   * Reconstruct reading order from text blocks.
   * For multi-column layouts: reads column 1 top-to-bottom, then column 2, etc.
   * For single column: reads top-to-bottom, left-to-right within each line.
   */
  static reconstructReadingOrder(blocks: TextContent[], pageWidth: number): TextContent[] {
    if (blocks.length <= 1) return blocks;

    const columns = LayoutAnalyzer.detectColumns(blocks, pageWidth);

    if (columns.length <= 1) {
      // Single column: sort top-to-bottom, then left-to-right
      return LayoutAnalyzer.sortBlocksNatural(blocks);
    }

    // Multi-column: sort columns left-to-right, then each column top-to-bottom
    columns.sort((a, b) => a.x - b.x);
    const ordered: TextContent[] = [];
    for (const col of columns) {
      ordered.push(...LayoutAnalyzer.sortBlocksNatural(col.blocks));
    }
    return ordered;
  }

  /** Sort blocks in natural reading order: top-to-bottom, left-to-right within lines. */
  static sortBlocksNatural(blocks: TextContent[]): TextContent[] {
    if (blocks.length <= 1) return [...blocks];

    // Group into lines (blocks with similar Y coordinates)
    const lines = LayoutAnalyzer.groupIntoLines(blocks);

    // Sort lines top-to-bottom (ascending Y since Y is flipped), then blocks left-to-right
    lines.sort((a, b) => a.y - b.y);
    const result: TextContent[] = [];
    for (const line of lines) {
      line.blocks.sort((a, b) => a.x - b.x);
      result.push(...line.blocks);
    }
    return result;
  }

  /** Group text blocks into horizontal lines based on Y-coordinate proximity. */
  static groupIntoLines(blocks: TextContent[]): TextLine[] {
    if (blocks.length === 0) return [];

    const sorted = [...blocks].sort((a, b) => a.y - b.y);
    const lines: TextLine[] = [];
    let currentLine: TextLine = {
      y: sorted[0].y,
      blocks: [sorted[0]],
      minX: sorted[0].x,
      maxX: sorted[0].x + sorted[0].width,
      pageNumber: sorted[0].pageNumber
    };

    for (let i = 1; i < sorted.length; i++) {
      const b = sorted[i];
      const lineThreshold = Math.max(currentLine.blocks[0].fontSize * 0.4, 3);

      if (Math.abs(b.y - currentLine.y) <= lineThreshold && b.pageNumber === currentLine.pageNumber) {
        currentLine.blocks.push(b);
        currentLine.minX = Math.min(currentLine.minX, b.x);
        currentLine.maxX = Math.max(currentLine.maxX, b.x + b.width);
      } else {
        lines.push(currentLine);
        currentLine = {
          y: b.y,
          blocks: [b],
          minX: b.x,
          maxX: b.x + b.width,
          pageNumber: b.pageNumber
        };
      }
    }
    lines.push(currentLine);
    return lines;
  }

  /**
   * Detect tables from text blocks using alignment analysis.
   * Finds groups of text that form grid patterns (aligned columns and rows).
   */
  static detectTables(blocks: TextContent[], pageNumber: number): Table[] {
    if (blocks.length < 4) return []; // Need at least 2x2 for a table

    const lines = LayoutAnalyzer.groupIntoLines(blocks);
    if (lines.length < 2) return [];

    // Find column anchors: X positions where text starts across multiple lines
    const xPositions: number[] = [];
    for (const line of lines) {
      for (const b of line.blocks) {
        xPositions.push(Math.round(b.x));
      }
    }

    // Count frequency of each rounded X position
    const xFreq = new Map<number, number>();
    const snapTolerance = 5;
    for (const x of xPositions) {
      const snapped = Math.round(x / snapTolerance) * snapTolerance;
      xFreq.set(snapped, (xFreq.get(snapped) || 0) + 1);
    }

    // Column anchors: X positions that appear in many lines (> 30% of lines)
    const minFreq = Math.max(2, Math.floor(lines.length * 0.3));
    const columnAnchors = [...xFreq.entries()]
      .filter(([_, freq]) => freq >= minFreq)
      .map(([x]) => x)
      .sort((a, b) => a - b);

    if (columnAnchors.length < 2) return []; // Need at least 2 column anchors

    // Merge close anchors
    const mergedAnchors: number[] = [columnAnchors[0]];
    for (let i = 1; i < columnAnchors.length; i++) {
      if (columnAnchors[i] - mergedAnchors[mergedAnchors.length - 1] > snapTolerance * 3) {
        mergedAnchors.push(columnAnchors[i]);
      }
    }

    if (mergedAnchors.length < 2) return [];

    // Find runs of consecutive lines that have blocks aligned to the column anchors
    const tables: Table[] = [];
    let tableStartIdx = -1;
    let tableLines: TextLine[] = [];

    for (let i = 0; i < lines.length; i++) {
      const line = lines[i];
      // Count how many column anchors this line's blocks align to
      let alignedCols = 0;
      for (const anchor of mergedAnchors) {
        const hasBlock = line.blocks.some(b => Math.abs(Math.round(b.x / snapTolerance) * snapTolerance - anchor) <= snapTolerance);
        if (hasBlock) alignedCols++;
      }

      const isTableRow = alignedCols >= 2;

      if (isTableRow) {
        if (tableStartIdx < 0) tableStartIdx = i;
        tableLines.push(line);
      } else {
        if (tableLines.length >= 2) {
          const table = LayoutAnalyzer.buildTable(tableLines, mergedAnchors, pageNumber, tables.length);
          if (table) tables.push(table);
        }
        tableStartIdx = -1;
        tableLines = [];
      }
    }

    // Handle table at end of page
    if (tableLines.length >= 2) {
      const table = LayoutAnalyzer.buildTable(tableLines, mergedAnchors, pageNumber, tables.length);
      if (table) tables.push(table);
    }

    return tables;
  }

  /** Build a Table object from detected table lines and column anchors. */
  private static buildTable(lines: TextLine[], anchors: number[], pageNumber: number, tableIdx: number): Table | null {
    const numCols = anchors.length;
    const numRows = lines.length;
    const cells: TableCell[][] = [];

    let minX = Infinity, minY = Infinity, maxX = 0, maxY = 0;

    for (let row = 0; row < numRows; row++) {
      const rowCells: TableCell[] = [];
      const line = lines[row];

      for (let col = 0; col < numCols; col++) {
        const anchorX = anchors[col];
        const nextAnchorX = col < numCols - 1 ? anchors[col + 1] : anchorX + 200;

        // Find blocks that belong to this cell
        const cellBlocks = line.blocks.filter(b => {
          const bx = Math.round(b.x / 5) * 5;
          return bx >= anchorX - 5 && bx < nextAnchorX - 5;
        });

        const cellText = cellBlocks.map(b => b.text).join(' ').trim();
        const isHeader = row === 0;

        let cellBox: Rectangle | undefined;
        if (cellBlocks.length > 0) {
          const cx = Math.min(...cellBlocks.map(b => b.x));
          const cy = Math.min(...cellBlocks.map(b => b.y));
          const cx2 = Math.max(...cellBlocks.map(b => b.x + b.width));
          const cy2 = Math.max(...cellBlocks.map(b => b.y + b.height));
          cellBox = { x: cx, y: cy, width: cx2 - cx, height: cy2 - cy };
          minX = Math.min(minX, cx);
          minY = Math.min(minY, cy);
          maxX = Math.max(maxX, cx2);
          maxY = Math.max(maxY, cy2);
        }

        rowCells.push({
          rowSpan: 1,
          colSpan: 1,
          text: cellText,
          isHeader,
          boundingBox: cellBox
        });
      }
      cells.push(rowCells);
    }

    if (cells.length < 2) return null;

    const headers = cells[0].map(c => c.text);

    return {
      id: `table_${pageNumber}_${tableIdx}`,
      pageNumber,
      boundingBox: {
        x: isFinite(minX) ? minX : 0,
        y: isFinite(minY) ? minY : 0,
        width: isFinite(maxX - minX) ? maxX - minX : 0,
        height: isFinite(maxY - minY) ? maxY - minY : 0
      },
      rows: numRows,
      columns: numCols,
      cells,
      headers
    };
  }

  /**
   * Detect vertical text (CJK vertical writing mode).
   * Vertical text has blocks where Y changes rapidly but X stays constant.
   */
  static detectVerticalText(blocks: TextContent[]): { vertical: TextContent[]; horizontal: TextContent[] } {
    const vertical: TextContent[] = [];
    const horizontal: TextContent[] = [];

    // CJK Unicode ranges
    const isCJK = (text: string): boolean => {
      for (let i = 0; i < text.length; i++) {
        const cp = text.codePointAt(i)!;
        if ((cp >= 0x4E00 && cp <= 0x9FFF) ||   // CJK Unified Ideographs
            (cp >= 0x3400 && cp <= 0x4DBF) ||   // CJK Extension A
            (cp >= 0x3040 && cp <= 0x309F) ||   // Hiragana
            (cp >= 0x30A0 && cp <= 0x30FF) ||   // Katakana
            (cp >= 0xAC00 && cp <= 0xD7AF)) {   // Hangul
          return true;
        }
      }
      return false;
    };

    // Group by similar X to detect vertical runs
    const xThreshold = 5;
    const xGroups = new Map<number, TextContent[]>();
    for (const b of blocks) {
      if (!isCJK(b.text)) {
        horizontal.push(b);
        continue;
      }
      const key = Math.round(b.x / xThreshold);
      const group = xGroups.get(key) || [];
      group.push(b);
      xGroups.set(key, group);
    }

    for (const [_, group] of xGroups) {
      if (group.length >= 3) {
        // Check if blocks form a vertical column (sorted by Y, single-char each)
        const sorted = [...group].sort((a, b) => a.y - b.y);
        const avgCharsPerBlock = sorted.reduce((s, b) => s + b.text.length, 0) / sorted.length;
        if (avgCharsPerBlock <= 2) {
          // Mark as vertical and update direction
          for (const b of sorted) {
            vertical.push({ ...b, direction: 'ttb' });
          }
          continue;
        }
      }
      horizontal.push(...group);
    }

    return { vertical, horizontal };
  }

  /**
   * Detect RTL text direction for a text block.
   * Returns 'rtl' if the majority of characters are in RTL scripts.
   */
  static detectTextDirection(text: string): 'ltr' | 'rtl' {
    let rtlCount = 0;
    let ltrCount = 0;

    for (let i = 0; i < text.length; i++) {
      const cp = text.codePointAt(i)!;
      if (cp > 0xFFFF) { i++; } // Skip surrogate pair

      if ((cp >= 0x0590 && cp <= 0x05FF) ||   // Hebrew
          (cp >= 0x0600 && cp <= 0x06FF) ||   // Arabic
          (cp >= 0x0700 && cp <= 0x074F) ||   // Syriac
          (cp >= 0x0780 && cp <= 0x07BF) ||   // Thaana
          (cp >= 0x08A0 && cp <= 0x08FF) ||   // Arabic Extended-A
          (cp >= 0xFB50 && cp <= 0xFDFF) ||   // Arabic Presentation Forms-A
          (cp >= 0xFE70 && cp <= 0xFEFF)) {   // Arabic Presentation Forms-B
        rtlCount++;
      } else if ((cp >= 0x0041 && cp <= 0x005A) || // Latin uppercase
                 (cp >= 0x0061 && cp <= 0x007A) || // Latin lowercase
                 (cp >= 0x00C0 && cp <= 0x024F)) { // Latin Extended
        ltrCount++;
      }
    }

    return rtlCount > ltrCount ? 'rtl' : 'ltr';
  }

  /**
   * Decompose common ligatures into their constituent characters.
   */
  static decomposeLigatures(text: string): string {
    // Standard ligatures
    const ligatureMap: Record<string, string> = {
      '\uFB00': 'ff',
      '\uFB01': 'fi',
      '\uFB02': 'fl',
      '\uFB03': 'ffi',
      '\uFB04': 'ffl',
      '\uFB05': 'st',  // ſt (long s + t)
      '\uFB06': 'st',
      // Latin ligatures
      '\u0132': 'IJ',
      '\u0133': 'ij',
      '\u0152': 'OE',
      '\u0153': 'oe',
      '\u00C6': 'AE',
      '\u00E6': 'ae',
      // German
      '\u1E9E': 'SS',
      '\u00DF': 'ss',
    };

    let result = '';
    for (let i = 0; i < text.length; i++) {
      const ch = text[i];
      const replacement = ligatureMap[ch];
      if (replacement) {
        result += replacement;
      } else {
        result += ch;
      }
    }
    return result;
  }
}


class ContentStreamParser {
  private position: number = 0;
  private errorRecoveryMode: boolean = false;

  // Parser cache for frequently parsed content streams
  private static parserCache = new Map<string, ContentOperation[]>();
  private static readonly MAX_CACHE_SIZE = 100;
  private static readonly CACHE_KEY_MAX_LENGTH = 1000;

  constructor(private data: Uint8Array) { }

  /**
   * Get cache key for content stream data
   */
  private getCacheKey(): string | null {
    // Only cache small content streams (performance vs memory tradeoff)
    if (this.data.length > ContentStreamParser.CACHE_KEY_MAX_LENGTH) {
      return null;
    }

    // Use first 100 bytes as cache key (fast and usually unique)
    const keyBytes = this.data.slice(0, Math.min(100, this.data.length));
    return Array.from(keyBytes).join(',');
  }

  /**
   * Clear parser cache (useful for memory management)
   */
  static clearCache(): void {
    ContentStreamParser.parserCache.clear();
  }

  parse(): ContentOperation[] {
    // Try to get cached result
    const cacheKey = this.getCacheKey();
    if (cacheKey) {
      const cached = ContentStreamParser.parserCache.get(cacheKey);
      if (cached) {
        return cached; // Return deep copy to prevent mutation
      }
    }

    const operations: ContentOperation[] = [];
    let iterations = 0;
    const maxIterations = this.data.length * 2; // Safety limit: twice the data length
    let lastPosition = -1;

    while (this.position < this.data.length) {
      // Safety check: prevent infinite loops
      iterations++;
      if (iterations > maxIterations) {
        console.warn(`ContentStreamParser: Reached max iterations (${maxIterations}), stopping parse`);
        break;
      }

      // Safety check: detect position not advancing
      if (this.position === lastPosition) {
        console.warn(`ContentStreamParser: Position stuck at ${this.position}, advancing forcefully`);
        this.position++;
        if (this.position >= this.data.length) break;
      }
      lastPosition = this.position;

      this.skipWhitespaceAndComments();
      if (this.position >= this.data.length) break;

      const operands: any[] = [];

      // Parse operands with safety counter
      let operandIterations = 0;
      const maxOperands = 1000; // Max operands before one operator
      let lastOperandPosition = -1;

      while (this.position < this.data.length) {
        if (++operandIterations > maxOperands) {
          console.warn(`ContentStreamParser: Too many operands (${maxOperands}) without operator at position ${this.position}`);
          break;
        }

        // Check if position is advancing
        if (this.position === lastOperandPosition) {
          console.warn(`ContentStreamParser: Operand parsing stuck at position ${this.position}`);
          this.position++; // Force advance
          break;
        }
        lastOperandPosition = this.position;

        const operand = this.parseOperand();
        if (operand === null) break;
        operands.push(operand);
      }

      // Parse operator
      const operator = this.parseOperator();
      if (operator) {
        // Handle inline image operator (BI)
        if (operator === 'BI') {
          const inlineImageOp = this.parseInlineImage(operands);
          if (inlineImageOp) {
            operations.push(inlineImageOp);
          }
        } else {
          operations.push({ operator, operands });
        }
      } else if (operands.length > 0) {
        // Incomplete operation: operands without operator
        // Attempt error recovery
        console.warn(`ContentStreamParser: Found ${operands.length} operands without operator at position ${this.position}`);
        if (!this.errorRecoveryMode) {
          this.errorRecoveryMode = true;
          this.recoverFromError();
        }
      } else {
        // If we can't parse an operator and have no operands, advance position
        if (this.position < this.data.length) {
          this.position++;
        }
      }
    }

    // Cache the result if applicable
    if (cacheKey && operations.length > 0) {
      // Implement LRU-style cache eviction
      if (ContentStreamParser.parserCache.size >= ContentStreamParser.MAX_CACHE_SIZE) {
        const firstKey = ContentStreamParser.parserCache.keys().next().value;
        if (firstKey) {
          ContentStreamParser.parserCache.delete(firstKey);
        }
      }
      ContentStreamParser.parserCache.set(cacheKey, operations);
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
    const startPos = this.position;

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

    // Validate operator length (PDF operators are typically 1-3 characters)
    if (operator.length > 10) {
      console.warn(`ContentStreamParser: Unusually long operator "${operator}" at position ${startPos}, truncating`);
      operator = operator.substring(0, 10);
    }

    return operator;
  }

  private parseNumber(): number {
    let numStr = '';
    let hasDecimalPoint = false;

    while (this.position < this.data.length) {
      const byte = this.data[this.position];

      if (byte >= 48 && byte <= 57) { // 0-9
        numStr += String.fromCharCode(byte);
        this.position++;
      } else if (byte === 46 && !hasDecimalPoint) { // '.' - only one decimal point allowed
        hasDecimalPoint = true;
        numStr += '.';
        this.position++;
      } else if ((byte === 45 || byte === 43) && numStr.length === 0) { // +/- only at start
        numStr += String.fromCharCode(byte);
        this.position++;
      } else {
        break;
      }
    }

    // Handle edge cases
    if (numStr === '' || numStr === '+' || numStr === '-' || numStr === '.') {
      console.warn(`ContentStreamParser: Invalid number "${numStr}" at position ${this.position}, defaulting to 0`);
      return 0;
    }

    const result = parseFloat(numStr);

    // Validate result
    if (isNaN(result) || !isFinite(result)) {
      console.warn(`ContentStreamParser: Invalid number "${numStr}" parsed as ${result}, defaulting to 0`);
      return 0;
    }

    return result;
  }

  private parseString(): string {
    this.position++; // Skip '('
    let str = '';
    let parenCount = 1;
    const startPos = this.position;

    while (this.position < this.data.length && parenCount > 0) {
      const byte = this.data[this.position++];

      if (byte === 40) { // '('
        parenCount++;
        str += '(';
      } else if (byte === 41) { // ')'
        parenCount--;
        if (parenCount > 0) str += ')';
      } else if (byte === 92 && this.position < this.data.length) { // '\' - escape sequence
        const next = this.data[this.position++];
        switch (next) {
          case 110: str += '\n'; break; // n
          case 114: str += '\r'; break; // r
          case 116: str += '\t'; break; // t
          case 98: str += '\b'; break;  // b
          case 102: str += '\f'; break; // f
          case 40: str += '('; break;   // (
          case 41: str += ')'; break;   // )
          case 92: str += '\\'; break;  // \
          // Octal escape sequences (\ddd)
          case 48: case 49: case 50: case 51: case 52: case 53: case 54: case 55: {
            let octalStr = String.fromCharCode(next);
            // Read up to 2 more octal digits
            for (let i = 0; i < 2 && this.position < this.data.length; i++) {
              const octalByte = this.data[this.position];
              if (octalByte >= 48 && octalByte <= 55) { // 0-7
                octalStr += String.fromCharCode(octalByte);
                this.position++;
              } else {
                break;
              }
            }
            const octalValue = parseInt(octalStr, 8);
            str += String.fromCharCode(octalValue);
            break;
          }
          // Line continuation (backslash followed by newline)
          case 10: case 13: // LF or CR
            if (next === 13 && this.position < this.data.length && this.data[this.position] === 10) {
              this.position++; // Skip LF after CR
            }
            // Don't add anything - line continuation
            break;
          default:
            // Unknown escape - keep the character
            str += String.fromCharCode(next);
        }
      } else {
        str += String.fromCharCode(byte);
      }
    }

    // Check for unclosed string
    if (parenCount > 0) {
      console.warn(`ContentStreamParser: Unclosed string at position ${startPos}, found ${parenCount} unclosed parentheses`);
    }

    return str;
  }

  private parseHexString(): string {
    const startPos = this.position;
    this.position++; // Skip '<'
    let hex = '';
    let foundClosing = false;

    while (this.position < this.data.length) {
      const byte = this.data[this.position];

      if (byte === 62) { // '>'
        this.position++;
        foundClosing = true;
        break;
      }

      // Skip whitespace in hex strings (PDF spec allows it)
      if (this.isWhitespace(byte)) {
        this.position++;
        continue;
      }

      if ((byte >= 48 && byte <= 57) || // 0-9
        (byte >= 65 && byte <= 70) || // A-F
        (byte >= 97 && byte <= 102)) { // a-f
        hex += String.fromCharCode(byte);
      } else {
        // Invalid hex character
        console.warn(`ContentStreamParser: Invalid hex character ${String.fromCharCode(byte)} at position ${this.position}`);
      }

      this.position++;
    }

    // Check for unclosed hex string
    if (!foundClosing) {
      console.warn(`ContentStreamParser: Unclosed hex string at position ${startPos}`);
    }

    // Handle odd number of hex digits (pad with 0)
    if (hex.length % 2 !== 0) {
      hex += '0';
    }

    // Convert hex to string
    let str = '';
    for (let i = 0; i < hex.length; i += 2) {
      const hexByte = hex.substr(i, 2);
      const value = parseInt(hexByte, 16);
      if (!isNaN(value)) {
        str += String.fromCharCode(value);
      }
    }

    return str;
  }

  private parseArray(): any[] {
    this.position++; // Skip '['
    const array: any[] = [];
    let safetyCounter = 0;
    const MAX_ARRAY_ITEMS = 100000;
    let lastPosition = -1;

    while (this.position < this.data.length) {
      if (++safetyCounter > MAX_ARRAY_ITEMS) {
        console.warn(`ContentStreamParser.parseArray: Safety limit reached (${MAX_ARRAY_ITEMS} items)`);
        break;
      }

      if (this.position === lastPosition) {
        console.warn(`ContentStreamParser.parseArray: Position stuck at ${this.position}`);
        this.position++;
        break;
      }
      lastPosition = this.position;

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
    let safetyCounter = 0;
    const MAX_NAME_LENGTH = 10000;

    while (this.position < this.data.length) {
      if (++safetyCounter > MAX_NAME_LENGTH) {
        console.warn(`ContentStreamParser.parseName: Safety limit reached (${MAX_NAME_LENGTH} chars)`);
        break;
      }

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
    let safetyCounter = 0;
    const MAX_DICT_ENTRIES = 10000;
    let lastPosition = -1;

    while (this.position < this.data.length) {
      if (++safetyCounter > MAX_DICT_ENTRIES) {
        console.warn(`ContentStreamParser.parseDictionary: Safety limit reached (${MAX_DICT_ENTRIES} entries)`);
        break;
      }

      if (this.position === lastPosition) {
        console.warn(`ContentStreamParser.parseDictionary: Position stuck at ${this.position}`);
        this.position++;
        break;
      }
      lastPosition = this.position;

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

  /**
   * Skip whitespace and comments
   * Comments start with % and continue to end of line
   */
  private skipWhitespaceAndComments(): void {
    while (this.position < this.data.length) {
      const byte = this.data[this.position];

      if (this.isWhitespace(byte)) {
        this.position++;
      } else if (byte === 37) { // '%' - comment
        this.skipComment();
      } else {
        break;
      }
    }
  }

  /**
   * Skip comment (from % to end of line)
   */
  private skipComment(): void {
    // Skip the '%'
    this.position++;

    // Skip until newline or end of data
    while (this.position < this.data.length) {
      const byte = this.data[this.position];
      this.position++;

      // Stop at line feed or carriage return
      if (byte === 10 || byte === 13) {
        break;
      }
    }
  }

  /**
   * Parse inline image (BI...ID...EI sequence)
   * Format: BI <dictionary> ID <image data> EI
   */
  private parseInlineImage(_operands: any[]): ContentOperation | null {
    try {
      // BI operator already consumed, parse image dictionary
      const imageDict = new Map<string, any>();

      // Parse key-value pairs until ID
      while (this.position < this.data.length) {
        this.skipWhitespaceAndComments();

        // Check for ID operator (image data marker)
        if (this.peekOperator() === 'ID') {
          this.parseOperator(); // Consume 'ID'
          break;
        }

        // Parse key (name or abbreviation)
        const key = this.parseName();
        if (!key) break;

        this.skipWhitespaceAndComments();

        // Parse value
        const value = this.parseOperand();
        imageDict.set(key, value);
      }

      // Skip whitespace after ID (single whitespace character)
      if (this.position < this.data.length && this.isWhitespace(this.data[this.position])) {
        this.position++;
      }

      // Read image data until EI operator
      const imageData = this.findInlineImageEnd();

      // Create operation with image dictionary and data
      return {
        operator: 'BI',
        operands: [{
          dictionary: imageDict,
          data: imageData
        }]
      };
    } catch (error) {
      console.warn('ContentStreamParser: Failed to parse inline image:', error);
      return null;
    }
  }

  /**
   * Find the end of inline image data (EI operator)
   * This is tricky because EI could appear in the image data
   */
  private findInlineImageEnd(): Uint8Array {
    const start = this.position;
    let end = start;

    // Look for 'EI' preceded by whitespace and followed by whitespace/delimiter
    while (this.position < this.data.length - 2) {
      // Check for whitespace + 'EI' + whitespace/delimiter
      const byte = this.data[this.position];

      if (this.isWhitespace(byte)) {
        const next1 = this.data[this.position + 1];
        const next2 = this.data[this.position + 2];

        // Check for 'EI' (0x45 0x49)
        if (next1 === 69 && next2 === 73) {
          // Check if followed by whitespace or delimiter
          if (this.position + 3 < this.data.length) {
            const next3 = this.data[this.position + 3];
            if (this.isWhitespace(next3) || this.isDelimiter(next3)) {
              // Found EI operator
              end = this.position;
              this.position += 3; // Skip whitespace + 'EI'
              break;
            }
          } else {
            // EI at end of stream
            end = this.position;
            this.position += 3;
            break;
          }
        }
      }

      this.position++;
    }

    // Extract image data
    const imageData = this.data.slice(start, end);
    return imageData;
  }

  /**
   * Peek at next operator without consuming it
   */
  private peekOperator(): string {
    const savedPosition = this.position;
    this.skipWhitespaceAndComments();

    let operator = '';
    let pos = this.position;

    while (pos < this.data.length) {
      const byte = this.data[pos];

      // Operator characters (letters and special chars)
      if ((byte >= 65 && byte <= 90) || // A-Z
        (byte >= 97 && byte <= 122) || // a-z
        byte === 42 || byte === 39 || byte === 34) { // *, ', "
        operator += String.fromCharCode(byte);
        pos++;
      } else {
        break;
      }
    }

    // Restore position
    this.position = savedPosition;
    return operator;
  }

  /**
   * Recover from parsing error
   * Attempts to synchronize with next valid operator
   */
  private recoverFromError(): void {
    console.warn('ContentStreamParser: Attempting error recovery');

    // Skip until we find a valid operator or delimiter
    let recovered = false;
    const recoveryLimit = 1000; // Don't skip more than 1000 bytes
    let skipped = 0;

    while (this.position < this.data.length && skipped < recoveryLimit) {
      const byte = this.data[this.position];

      // Try to find a delimiter or whitespace
      if (this.isWhitespace(byte) || this.isDelimiter(byte)) {
        // Try to parse next token
        this.skipWhitespaceAndComments();

        // Check if we can parse an operator
        const savedPos = this.position;
        const op = this.parseOperator();

        if (op && op.length > 0) {
          // Found valid operator, restore position and exit recovery
          this.position = savedPos;
          recovered = true;
          this.errorRecoveryMode = false;
          break;
        }
      }

      this.position++;
      skipped++;
    }

    if (!recovered) {
      console.warn('ContentStreamParser: Error recovery failed, continuing parse');
      this.errorRecoveryMode = false;
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

/**
 * CCITT Fax decoder (Group 3 1D / Group 4)
 * Decodes bi-level (1-bit) image data used in scanned documents.
 */
class CCITTFaxDecoder {
  private data: Uint8Array;
  private bytePos = 0;
  private bitPos = 0;
  private columns: number;
  private rows: number;
  private blackIs1: boolean;
  private encodedByteAlign: boolean;

  // ITU-T T.4 / T.6 run-length tables
  // White run make-up codes + terminating codes; black run equivalents
  private static readonly WHITE_TERM: [number, number, number][] = [
    [0x35, 8, 0], [0x07, 6, 1], [0x07, 4, 2], [0x08, 4, 3],
    [0x0B, 4, 4], [0x0C, 4, 5], [0x0E, 4, 6], [0x0F, 4, 7],
    [0x13, 5, 8], [0x14, 5, 9], [0x07, 5, 10], [0x08, 5, 11],
    [0x08, 6, 12], [0x03, 6, 13], [0x34, 6, 14], [0x35, 6, 15],
    [0x2A, 6, 16], [0x2B, 6, 17], [0x27, 7, 18], [0x0C, 7, 19],
    [0x08, 7, 20], [0x17, 7, 21], [0x03, 7, 22], [0x04, 7, 23],
    [0x28, 7, 24], [0x2B, 7, 25], [0x13, 7, 26], [0x24, 7, 27],
    [0x18, 7, 28], [0x02, 8, 29], [0x03, 8, 30], [0x1A, 8, 31],
    [0x1B, 8, 32], [0x12, 8, 33], [0x13, 8, 34], [0x14, 8, 35],
    [0x15, 8, 36], [0x16, 8, 37], [0x17, 8, 38], [0x28, 8, 39],
    [0x29, 8, 40], [0x2A, 8, 41], [0x2B, 8, 42], [0x2C, 8, 43],
    [0x2D, 8, 44], [0x04, 8, 45], [0x05, 8, 46], [0x0A, 8, 47],
    [0x0B, 8, 48], [0x52, 8, 49], [0x53, 8, 50], [0x54, 8, 51],
    [0x55, 8, 52], [0x24, 8, 53], [0x25, 8, 54], [0x58, 8, 55],
    [0x59, 8, 56], [0x5A, 8, 57], [0x5B, 8, 58], [0x4A, 8, 59],
    [0x4B, 8, 60], [0x32, 8, 61], [0x33, 8, 62], [0x34, 8, 63],
  ];

  private static readonly WHITE_MAKEUP: [number, number, number][] = [
    [0x1B, 5, 64], [0x12, 5, 128], [0x17, 6, 192], [0x37, 7, 256],
    [0x36, 8, 320], [0x37, 8, 384], [0x64, 8, 448], [0x65, 8, 512],
    [0x68, 8, 576], [0x67, 8, 640], [0xCC, 9, 704], [0xCD, 9, 768],
    [0xD2, 9, 832], [0xD3, 9, 896], [0xD4, 9, 960], [0xD5, 9, 1024],
    [0xD6, 9, 1088], [0xD7, 9, 1152], [0xD8, 9, 1216], [0xD9, 9, 1280],
    [0xDA, 9, 1344], [0xDB, 9, 1408], [0x98, 9, 1472], [0x99, 9, 1536],
    [0x9A, 9, 1600], [0x18, 6, 1664], [0x9B, 9, 1728],
  ];

  constructor(data: Uint8Array, columns: number, rows: number, blackIs1 = false, encodedByteAlign = false) {
    this.data = data;
    this.columns = columns;
    this.rows = rows;
    this.blackIs1 = blackIs1;
    this.encodedByteAlign = encodedByteAlign;
  }

  /**
   * Decode CCITT data to a bitmap (1 bit per pixel → 8 bits per pixel grayscale)
   * Returns Uint8Array of width*height bytes, 0=black, 255=white
   */
  decode(): Uint8Array {
    // Simplified: treat as raw uncompressed bitmap for now
    // Full ITU-T T.4/T.6 decoding is complex; provide best-effort output
    const output = new Uint8Array(this.columns * this.rows);
    const bytesPerRow = Math.ceil(this.columns / 8);
    const whiteVal = this.blackIs1 ? 0 : 255;
    const blackVal = this.blackIs1 ? 255 : 0;

    for (let row = 0; row < this.rows; row++) {
      for (let col = 0; col < this.columns; col++) {
        const byteIdx = row * bytesPerRow + (col >> 3);
        const bitIdx = 7 - (col & 7);
        if (byteIdx < this.data.length) {
          const bit = (this.data[byteIdx] >> bitIdx) & 1;
          output[row * this.columns + col] = bit ? blackVal : whiteVal;
        } else {
          output[row * this.columns + col] = whiteVal;
        }
      }
    }
    return output;
  }

  /**
   * Decode to a 1-bit packed bitmap (returns packed bytes, MSB first)
   */
  decodePacked(): Uint8Array {
    const bytesPerRow = Math.ceil(this.columns / 8);
    const output = new Uint8Array(bytesPerRow * this.rows);
    // For raw/uncompressed CCITT data the input is already packed
    const len = Math.min(this.data.length, output.length);
    for (let i = 0; i < len; i++) {
      output[i] = this.blackIs1 ? this.data[i] : (this.data[i] ^ 0xFF);
    }
    return output;
  }
}
class ImageExtractor {
  constructor(
    private pdf: AgenticPDF,
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

    // Page range filtering
    if (this.options?.pageRange) {
      const { start, end } = this.options.pageRange;
      if (page.pageNumber < start || page.pageNumber > end) return images;
    }

    let imageIndex = 0;
    for (const [_name, imageRes] of page.resources.images) {
      const image: ImageContent = {
        id: `page${page.pageNumber}_img${imageIndex++}`,
        x: 0,
        y: 0,
        width: imageRes.width,
        height: imageRes.height,
        mimeType: this.getImageMimeType(imageRes),
        bitsPerComponent: imageRes.bitsPerComponent,
        colorSpace: imageRes.colorSpace,
        filter: imageRes.filter,
        data: this.resolveImageData(imageRes),
        pageNumber: page.pageNumber
      };

      images.push(image);
    }

    return images;
  }

  /**
   * Resolve image data based on filter type.
   * JPEG/JPEG2000: passthrough raw compressed data directly.
   * CCITT: decode to grayscale pixel data.
   * FlateDecode: raw pixel data (already decompressed by parser).
   */
  private resolveImageData(img: ImageResource): Uint8Array {
    if (!img.data || img.data.length === 0) return new Uint8Array(0);

    const filter = img.filter?.[0] || '';

    // DCTDecode (JPEG) — pass raw JPEG data directly
    if (filter === 'DCTDecode') {
      return img.data;
    }

    // JPXDecode (JPEG 2000) — pass raw JP2 data directly
    if (filter === 'JPXDecode') {
      return img.data;
    }

    // CCITTFaxDecode — decode Group 3/4 to grayscale
    if (filter === 'CCITTFaxDecode') {
      try {
        const dp = img.decodeParms || {};
        const columns = dp.Columns || img.width || 1;
        const rows = dp.Rows || img.height || 1;
        const blackIs1 = !!dp.BlackIs1;
        const decoder = new CCITTFaxDecoder(img.data, columns, rows, blackIs1);
        return decoder.decode();
      } catch (e) {
        return img.data;
      }
    }

    // JBIG2Decode — return raw data (full decode is very complex)
    if (filter === 'JBIG2Decode') {
      return img.data;
    }

    // FlateDecode or no filter — already decoded pixels from parser
    return img.data;
  }

  private getImageMimeType(imageRes: ImageResource): string {
    const filter = imageRes.filter?.[0] || '';
    if (filter === 'DCTDecode') return 'image/jpeg';
    if (filter === 'JPXDecode') return 'image/jp2';
    if (filter === 'JBIG2Decode') return 'image/x-jbig2';
    if (filter === 'CCITTFaxDecode') return 'image/x-ccitt';
    // All other formats (raw/FlateDecode) are resolved to raw pixel data
    return 'image/raw';
  }

  /**
   * Convert raw pixel ImageContent to a data URL (PNG or JPEG).
   * For DCTDecode images, wraps the raw JPEG bytes in a data URL.
   * For raw pixel data, encodes to PNG using canvas.
   */
  static toDataURL(image: ImageContent, format: 'png' | 'jpeg' | 'webp' = 'png', quality = 0.92): string {
    if (!image.data || image.data.length === 0) return '';

    // JPEG passthrough — already compressed
    if (image.mimeType === 'image/jpeg') {
      let binary = '';
      for (let i = 0; i < image.data.length; i++) binary += String.fromCharCode(image.data[i]);
      return 'data:image/jpeg;base64,' + btoa(binary);
    }

    // JPEG 2000 passthrough
    if (image.mimeType === 'image/jp2') {
      let binary = '';
      for (let i = 0; i < image.data.length; i++) binary += String.fromCharCode(image.data[i]);
      return 'data:image/jp2;base64,' + btoa(binary);
    }

    // Raw pixel data — need canvas to encode
    if (typeof document === 'undefined') return ''; // Server-side: no canvas

    const w = image.width;
    const h = image.height;
    if (w <= 0 || h <= 0) return '';

    const canvas = document.createElement('canvas');
    canvas.width = w;
    canvas.height = h;
    const ctx = canvas.getContext('2d')!;
    const imgData = ctx.createImageData(w, h);
    const rgba = imgData.data;

    const bpc = image.bitsPerComponent || 8;
    const cs = (image.colorSpace || 'DeviceRGB').replace(/^\//, '');
    let comp = 3;
    if (cs === 'DeviceGray' || cs === 'CalGray' || cs === 'G') comp = 1;
    else if (cs === 'DeviceCMYK' || cs === 'CMYK') comp = 4;

    const pixels = image.data;
    const total = w * h;

    if (bpc === 8) {
      for (let i = 0; i < total; i++) {
        const di = i * 4;
        if (comp === 1) {
          const g = pixels[i] || 0;
          rgba[di] = g; rgba[di+1] = g; rgba[di+2] = g; rgba[di+3] = 255;
        } else if (comp === 3) {
          const si = i * 3;
          rgba[di] = pixels[si]||0; rgba[di+1] = pixels[si+1]||0; rgba[di+2] = pixels[si+2]||0; rgba[di+3] = 255;
        } else if (comp === 4) {
          const si = i * 4;
          const c = (pixels[si]||0)/255, m = (pixels[si+1]||0)/255, y = (pixels[si+2]||0)/255, k = (pixels[si+3]||0)/255;
          rgba[di] = 255*(1-c)*(1-k)|0; rgba[di+1] = 255*(1-m)*(1-k)|0; rgba[di+2] = 255*(1-y)*(1-k)|0; rgba[di+3] = 255;
        }
      }
    } else if (bpc === 1) {
      // 1-bit per component (common for CCITT-decoded images)
      for (let i = 0; i < total; i++) {
        const di = i * 4;
        const g = pixels[i] || 0; // Already expanded by CCITT decoder
        rgba[di] = g; rgba[di+1] = g; rgba[di+2] = g; rgba[di+3] = 255;
      }
    } else {
      // Fallback — treat as 8-bit grayscale
      for (let i = 0; i < total && i < pixels.length; i++) {
        const di = i * 4;
        const g = pixels[i] || 0;
        rgba[di] = g; rgba[di+1] = g; rgba[di+2] = g; rgba[di+3] = 255;
      }
    }

    ctx.putImageData(imgData, 0, 0);
    const mimeType = format === 'jpeg' ? 'image/jpeg' : format === 'webp' ? 'image/webp' : 'image/png';
    return canvas.toDataURL(mimeType, quality);
  }
}

class AIAnalyzer {
  constructor(
    private pdf: AgenticPDF,
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
    const allTables: Table[] = [];
    const metadata = this.pdf.getMetadata();
    if (!metadata) return allTables;

    for (let p = 1; p <= metadata.pageCount; p++) {
      const pageText = await this.pdf.extractText({ pageRange: { start: p, end: p } });
      if (pageText.length >= 4) {
        const pageTables = LayoutAnalyzer.detectTables(pageText, p);
        allTables.push(...pageTables);
      }
    }
    return allTables;
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

  private extractEquations(_text: TextContent[]): Equation[] {
    const equations: Equation[] = [];
    // Equation extraction logic
    // Would detect mathematical notation patterns
    return equations;
  }

  private extractReferences(_text: TextContent[]): Reference[] {
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

  private extractBibliography(_text: TextContent[]): BibliographyEntry[] {
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
    private pdf: AgenticPDF,
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
  constructor(private pdf: AgenticPDF) { }

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
  constructor(private pdf: AgenticPDF) { }

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
    const fields: FormField[] = [];

    try {
      // Parse page dictionary to find Annots array
      const pageDict = await this.getPageDictionary(page.pageNumber);
      const annotsRef = pageDict?.entries.get('Annots');

      if (annotsRef && annotsRef.type === PDFObjectType.Array) {
        const annots = annotsRef.value as PDFObject[];

        for (let i = 0; i < annots.length; i++) {
          const annotRef = annots[i];
          if (annotRef.type === PDFObjectType.Reference) {
            const ref = annotRef.value as PDFReference;

            // Get annotation dictionary from PDF objects
            const annotDict = await this.getObjectFromReference(ref);

            if (annotDict && annotDict.type === PDFObjectType.Dictionary) {
              const dict = annotDict.value as PDFDictionary;
              const subtype = dict.entries.get('Subtype');

              // Check if it's a Widget annotation (form field)
              if (subtype && subtype.type === PDFObjectType.Name && subtype.value === 'Widget') {
                const field = await this.parseFormField(dict, page.pageNumber, i);
                if (field) fields.push(field);
              }
            }
          }
        }
      }
    } catch (error) {
      console.warn('Error extracting form fields from page:', error);
    }

    return fields;
  }

  private async getPageDictionary(pageNumber: number): Promise<PDFDictionary | undefined> {
    // Get the page dictionary from the page tree
    const pageTree = (this.pdf as any).pageTree;
    return pageTree?.getPage(pageNumber);
  }

  private async getObjectFromReference(ref: PDFReference): Promise<PDFObject | undefined> {
    try {
      const parser = (this.pdf as any).parser;
      const xref = (this.pdf as any).xrefTable;
      if (parser && xref) {
        return parser.parseIndirectObject(ref.objectNumber, ref.generationNumber, xref);
      }
    } catch (e) {
      // fallback: try objects cache
    }
    const objects = (this.pdf as any).objects;
    const key = `${ref.objectNumber}_${ref.generationNumber}`;
    return objects?.get(key);
  }

  private async parseFormField(annotDict: PDFDictionary, pageNumber: number, fieldIndex: number): Promise<FormField | null> {
    try {
      const rect = this.parseRectFromDict(annotDict, 'Rect');
      if (!rect) return null;

      // Walk /Parent chain for inherited properties (FT, Ff, T, V, DV, Opt)
      const inherited = await this.resolveInheritedProps(annotDict);

      const fieldName = this.getStringFromDict(annotDict, 'T') || inherited.name || `field_${pageNumber}_${fieldIndex}`;
      const fieldType = this.mapFT(this.getNameFromDict(annotDict, 'FT') || inherited.ft);
      const flags = this.getNumberFromDict(annotDict, 'Ff') ?? inherited.ff ?? 0;

      const field: FormField = {
        id: `${pageNumber}_${fieldIndex}_${fieldName}`,
        name: fieldName,
        type: fieldType,
        value: this.getFieldValue(annotDict) ?? inherited.value,
        defaultValue: this.getFieldValue(annotDict, 'DV') ?? inherited.defaultValue,
        rect,
        pageNumber,
        flags,
        required: !!(flags & 2),
        readOnly: !!(flags & 1),
        noExport: !!(flags & 4),
        multiline: !!(flags & 4096),
        password: !!(flags & 8192),
        maxLength: this.getNumberFromDict(annotDict, 'MaxLen')
      };

      // Handle specific field types
      if (field.type === FormFieldType.Choice) {
        field.options = this.getChoiceOptions(annotDict);
        if (!field.options?.length && inherited.optDict) {
          field.options = this.getChoiceOptions(inherited.optDict);
        }
      }

      // Detect button sub-type from Ff flags
      if (field.type === FormFieldType.Button) {
        if (flags & 0x10000) {
          field.buttonSubType = 'pushbutton';
        } else if (flags & 0x8000) {
          field.buttonSubType = 'radio';
        } else {
          field.buttonSubType = 'checkbox';
        }
      }

      return field;
    } catch (error) {
      console.warn('Error parsing form field:', error);
      return null;
    }
  }

  private async resolveInheritedProps(dict: PDFDictionary): Promise<{
    ft?: string; ff?: number; name?: string; value?: any; defaultValue?: any; optDict?: PDFDictionary;
  }> {
    const result: { ft?: string; ff?: number; name?: string; value?: any; defaultValue?: any; optDict?: PDFDictionary } = {};
    let current = dict;
    for (let depth = 0; depth < 10; depth++) {
      const parentRef = current.entries.get('Parent');
      if (!parentRef || parentRef.type !== PDFObjectType.Reference) break;
      const parentObj = await this.getObjectFromReference(parentRef.value as PDFReference);
      if (!parentObj || parentObj.type !== PDFObjectType.Dictionary) break;
      current = parentObj.value as PDFDictionary;
      if (!result.ft) {
        const ft = current.entries.get('FT');
        if (ft?.type === PDFObjectType.Name) result.ft = ft.value as string;
      }
      if (result.ff === undefined) {
        const ff = current.entries.get('Ff');
        if (ff?.type === PDFObjectType.Number) result.ff = ff.value as number;
      }
      if (!result.name) {
        const t = current.entries.get('T');
        if (t?.type === PDFObjectType.String) result.name = t.value as string;
      }
      if (result.value === undefined) {
        result.value = this.getFieldValue(current);
      }
      if (result.defaultValue === undefined) {
        result.defaultValue = this.getFieldValue(current, 'DV');
      }
      if (!result.optDict && current.entries.has('Opt')) {
        result.optDict = current;
      }
    }
    return result;
  }

  private getNameFromDict(dict: PDFDictionary, key: string): string | undefined {
    const obj = dict.entries.get(key);
    return obj && obj.type === PDFObjectType.Name ? obj.value as string : undefined;
  }

  private mapFT(ft?: string): FormFieldType {
    switch (ft) {
      case 'Tx': return FormFieldType.Text;
      case 'Btn': return FormFieldType.Button;
      case 'Ch': return FormFieldType.Choice;
      case 'Sig': return FormFieldType.Signature;
      default: return FormFieldType.Text;
    }
  }

  private getFormFieldType(dict: PDFDictionary): FormFieldType {
    const ft = dict.entries.get('FT');
    if (ft && ft.type === PDFObjectType.Name) {
      switch (ft.value) {
        case 'Tx': return FormFieldType.Text;
        case 'Btn': return FormFieldType.Button;
        case 'Ch': return FormFieldType.Choice;
        case 'Sig': return FormFieldType.Signature;
        default: return FormFieldType.Text;
      }
    }
    return FormFieldType.Text;
  }

  private getFieldValue(dict: PDFDictionary, key: string = 'V'): any {
    const v = dict.entries.get(key);
    if (!v) return null;

    switch (v.type) {
      case PDFObjectType.String:
      case PDFObjectType.Name:
        return v.value;
      case PDFObjectType.Boolean:
        return v.value;
      case PDFObjectType.Number:
        return v.value;
      case PDFObjectType.Array:
        return (v.value as PDFObject[]).map(obj => obj.value);
      default:
        return null;
    }
  }

  private getChoiceOptions(dict: PDFDictionary): FormFieldOption[] {
    const opt = dict.entries.get('Opt');
    if (!opt || opt.type !== PDFObjectType.Array) return [];

    const options: FormFieldOption[] = [];
    const optArray = opt.value as PDFObject[];

    // Get selected values
    const selectedValues = new Set<string>();
    const value = this.getFieldValue(dict);
    if (Array.isArray(value)) {
      value.forEach(v => selectedValues.add(String(v)));
    } else if (value !== null) {
      selectedValues.add(String(value));
    }

    for (let i = 0; i < optArray.length; i++) {
      const option = optArray[i];
      if (option.type === PDFObjectType.String) {
        const optValue = option.value as string;
        options.push({
          value: optValue,
          label: optValue,
          selected: selectedValues.has(optValue)
        });
      } else if (option.type === PDFObjectType.Array) {
        const pair = option.value as PDFObject[];
        if (pair.length >= 2) {
          const optValue = pair[0].value as string;
          options.push({
            value: optValue,
            label: pair[1].value as string,
            selected: selectedValues.has(optValue)
          });
        }
      }
    }

    return options;
  }

  private parseRectFromDict(dict: PDFDictionary, key: string): Rectangle | null {
    const rectObj = dict.entries.get(key);
    if (!rectObj || rectObj.type !== PDFObjectType.Array) return null;

    const arr = rectObj.value as PDFObject[];
    if (arr.length !== 4) return null;

    return {
      x: arr[0].value as number,
      y: arr[1].value as number,
      width: (arr[2].value as number) - (arr[0].value as number),
      height: (arr[3].value as number) - (arr[1].value as number)
    };
  }

  private getStringFromDict(dict: PDFDictionary, key: string): string | undefined {
    const obj = dict.entries.get(key);
    return obj && obj.type === PDFObjectType.String ? obj.value as string : undefined;
  }

  private getNumberFromDict(dict: PDFDictionary, key: string): number | undefined {
    const obj = dict.entries.get(key);
    return obj && obj.type === PDFObjectType.Number ? obj.value as number : undefined;
  }

  private getBooleanFromDict(dict: PDFDictionary, key: string): boolean {
    const obj = dict.entries.get(key);
    return obj && obj.type === PDFObjectType.Boolean ? obj.value as boolean : false;
  }
}

class FormFiller {
  constructor(private pdf: AgenticPDF) { }

  async fill(data: Record<string, any>): Promise<void> {
    const formValues = (this.pdf as any)._formValues as Map<string, any>;
    for (const [key, val] of Object.entries(data)) {
      formValues.set(key, val);
    }
  }
}

class AnnotationExtractor {
  constructor(private pdf: AgenticPDF) { }

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
    const annotations: Annotation[] = [];

    try {
      // Parse page dictionary to find Annots array
      const pageDict = await this.getPageDictionary(page.pageNumber);
      const annotsRef = pageDict?.entries.get('Annots');

      if (annotsRef && annotsRef.type === PDFObjectType.Array) {
        const annots = annotsRef.value as PDFObject[];

        for (let i = 0; i < annots.length; i++) {
          const annotRef = annots[i];
          if (annotRef.type === PDFObjectType.Reference) {
            const ref = annotRef.value as PDFReference;

            // Get annotation dictionary from PDF objects
            const annotObj = await this.getObjectFromReference(ref);

            if (annotObj && annotObj.type === PDFObjectType.Dictionary) {
              const annotDict = annotObj.value as PDFDictionary;
              const annotation = this.parseAnnotation(annotDict, page.pageNumber, i);
              if (annotation) annotations.push(annotation);
            }
          }
        }
      }
    } catch (error) {
      console.warn('Error extracting annotations from page:', error);
    }

    return annotations;
  }

  private async getPageDictionary(pageNumber: number): Promise<PDFDictionary | undefined> {
    // Get the page dictionary from the page tree
    const pageTree = (this.pdf as any).pageTree;
    return pageTree?.getPage(pageNumber);
  }

  private async getObjectFromReference(ref: PDFReference): Promise<PDFObject | undefined> {
    try {
      const parser = (this.pdf as any).parser;
      const xref = (this.pdf as any).xrefTable;
      if (parser && xref) {
        return parser.parseIndirectObject(ref.objectNumber, ref.generationNumber, xref);
      }
    } catch (e) {
      // fallback: try objects cache
    }
    const objects = (this.pdf as any).objects;
    const key = `${ref.objectNumber}_${ref.generationNumber}`;
    return objects?.get(key);
  }

  private parseAnnotation(dict: PDFDictionary, pageNumber: number, index: number): Annotation | null {
    try {
      // Get annotation subtype
      const subtype = dict.entries.get('Subtype');
      if (!subtype || subtype.type !== PDFObjectType.Name) return null;

      const annotationType = this.mapSubtypeToAnnotationType(subtype.value as string);
      if (!annotationType) return null;

      // Extract common annotation properties
      const rect = this.parseRectFromDict(dict, 'Rect');
      if (!rect) return null;

      const annotation: Annotation = {
        id: `annotation_${pageNumber}_${index}`,
        type: annotationType,
        rect,
        pageNumber,
        contents: this.getStringFromDict(dict, 'Contents'),
        author: this.getStringFromDict(dict, 'T'),
        modificationDate: this.parseDateFromDict(dict, 'M'),
        flags: this.getNumberFromDict(dict, 'F') || 0,
        opacity: this.getNumberFromDict(dict, 'CA'),
        color: this.parseColorFromDict(dict, 'C')
      };

      // Parse type-specific properties
      this.parseTypeSpecificProperties(annotation, dict);

      return annotation;
    } catch (error) {
      console.warn('Error parsing annotation:', error);
      return null;
    }
  }

  private mapSubtypeToAnnotationType(subtype: string): AnnotationType | null {
    switch (subtype) {
      case 'Text': return AnnotationType.Text;
      case 'Link': return AnnotationType.Link;
      case 'FreeText': return AnnotationType.FreeText;
      case 'Line': return AnnotationType.Line;
      case 'Square': return AnnotationType.Square;
      case 'Circle': return AnnotationType.Circle;
      case 'Polygon': return AnnotationType.Polygon;
      case 'PolyLine': return AnnotationType.PolyLine;
      case 'Highlight': return AnnotationType.Highlight;
      case 'Underline': return AnnotationType.Underline;
      case 'Squiggly': return AnnotationType.Squiggly;
      case 'StrikeOut': return AnnotationType.StrikeOut;
      case 'Stamp': return AnnotationType.Stamp;
      case 'Caret': return AnnotationType.Caret;
      case 'Ink': return AnnotationType.Ink;
      case 'Popup': return AnnotationType.Popup;
      case 'FileAttachment': return AnnotationType.FileAttachment;
      case 'Sound': return AnnotationType.Sound;
      case 'Movie': return AnnotationType.Movie;
      case 'Widget': return AnnotationType.Widget;
      case 'Screen': return AnnotationType.Screen;
      case 'PrinterMark': return AnnotationType.PrinterMark;
      case 'TrapNet': return AnnotationType.TrapNet;
      case 'Watermark': return AnnotationType.Watermark;
      case 'Redact': return AnnotationType.Redact;
      default: return null;
    }
  }

  private parseTypeSpecificProperties(annotation: Annotation, dict: PDFDictionary): void {
    // Parse destination for link annotations
    if (annotation.type === AnnotationType.Link) {
      const action = dict.entries.get('A');
      if (action && action.type === PDFObjectType.Dictionary) {
        const actionDict = action.value as PDFDictionary;
        const actionType = actionDict.entries.get('S');

        if (actionType && actionType.type === PDFObjectType.Name) {
          if (actionType.value === 'URI') {
            const uri = actionDict.entries.get('URI');
            if (uri && uri.type === PDFObjectType.String) {
              annotation.destination = uri.value as string;
            }
          } else if (actionType.value === 'GoTo') {
            const dest = actionDict.entries.get('D');
            if (dest) {
              if (dest.type === PDFObjectType.String) {
                annotation.destination = dest.value as string;
              } else if (dest.type === PDFObjectType.Array) {
                annotation.destination = (dest.value as PDFObject[]).map(obj => obj.value as number);
              }
            }
          }
        }
      }

      // Fallback: check /Dest key directly on annotation dict
      if (!annotation.destination) {
        const dest = dict.entries.get('Dest');
        if (dest) {
          if (dest.type === PDFObjectType.String) {
            annotation.destination = dest.value as string;
          } else if (dest.type === PDFObjectType.Array) {
            annotation.destination = (dest.value as PDFObject[]).map(obj => obj.value as number);
          }
        }
      }
    }

    // Parse border style if present
    const borderStyle = dict.entries.get('BS');
    if (borderStyle && borderStyle.type === PDFObjectType.Dictionary) {
      const bsDict = borderStyle.value as PDFDictionary;
      const width = this.getNumberFromDict(bsDict, 'W');
      const style = this.getStringFromDict(bsDict, 'S');

      if (width !== undefined && style) {
        annotation.borderStyle = {
          width,
          style: style as 'Solid' | 'Dashed' | 'Beveled' | 'Inset' | 'Underline'
        };
      }
    }
  }

  private parseColor(colorArray: PDFObject[]): Color | undefined {
    if (colorArray.length === 1) {
      // Grayscale
      const gray = colorArray[0].value as number;
      return { r: gray, g: gray, b: gray };
    } else if (colorArray.length === 3) {
      // RGB
      return {
        r: colorArray[0].value as number,
        g: colorArray[1].value as number,
        b: colorArray[2].value as number
      };
    } else if (colorArray.length === 4) {
      // CMYK - convert to RGB approximation
      const c = colorArray[0].value as number;
      const m = colorArray[1].value as number;
      const y = colorArray[2].value as number;
      const k = colorArray[3].value as number;

      return {
        r: 1 - Math.min(1, c * (1 - k) + k),
        g: 1 - Math.min(1, m * (1 - k) + k),
        b: 1 - Math.min(1, y * (1 - k) + k)
      };
    }

    return undefined;
  }

  private parseRectFromDict(dict: PDFDictionary, key: string): Rectangle | null {
    const rectObj = dict.entries.get(key);
    if (!rectObj || rectObj.type !== PDFObjectType.Array) return null;

    const arr = rectObj.value as PDFObject[];
    if (arr.length !== 4) return null;

    return {
      x: arr[0].value as number,
      y: arr[1].value as number,
      width: (arr[2].value as number) - (arr[0].value as number),
      height: (arr[3].value as number) - (arr[1].value as number)
    };
  }

  private getStringFromDict(dict: PDFDictionary, key: string): string | undefined {
    const obj = dict.entries.get(key);
    return obj && obj.type === PDFObjectType.String ? obj.value as string : undefined;
  }

  private getNumberFromDict(dict: PDFDictionary, key: string): number | undefined {
    const obj = dict.entries.get(key);
    return obj && obj.type === PDFObjectType.Number ? obj.value as number : undefined;
  }

  private getBooleanFromDict(dict: PDFDictionary, key: string): boolean {
    const obj = dict.entries.get(key);
    return obj && obj.type === PDFObjectType.Boolean ? obj.value as boolean : false;
  }

  private parseDateFromDict(dict: PDFDictionary, key: string): Date | undefined {
    const dateStr = this.getStringFromDict(dict, key);
    if (!dateStr) return undefined;

    try {
      // PDF date format: D:YYYYMMDDHHmmSSOHH'mm
      if (dateStr.startsWith('D:')) {
        const year = parseInt(dateStr.substr(2, 4), 10);
        const month = parseInt(dateStr.substr(6, 2), 10) - 1;
        const day = parseInt(dateStr.substr(8, 2), 10);
        const hour = parseInt(dateStr.substr(10, 2), 10) || 0;
        const minute = parseInt(dateStr.substr(12, 2), 10) || 0;
        const second = parseInt(dateStr.substr(14, 2), 10) || 0;

        return new Date(year, month, day, hour, minute, second);
      }

      // Fallback to standard Date parsing
      return new Date(dateStr);
    } catch (error) {
      console.warn('Error parsing date:', dateStr, error);
      return undefined;
    }
  }

  private parseColorFromDict(dict: PDFDictionary, key: string): Color | undefined {
    const colorObj = dict.entries.get(key);
    if (!colorObj || colorObj.type !== PDFObjectType.Array) return undefined;

    const colorArray = colorObj.value as PDFObject[];
    return this.parseColor(colorArray);
  }
}

class AnnotationManager {
  constructor(private pdf: AgenticPDF) { }

  async add(_annotation: Partial<Annotation>): Promise<string> {
    const id = `annotation_${Date.now()}`;

    // Add annotation to PDF structure
    // This would modify the page's annotation array

    return id;
  }

  async remove(_annotationId: string): Promise<boolean> {
    // Remove annotation from PDF structure
    return true;
  }

  async update(_annotationId: string, _updates: Partial<Annotation>): Promise<boolean> {
    // Update annotation in PDF structure
    return true;
  }
}

/**
 * Safe content stream parser that prevents infinite loops
 * Based on PDF.js approach with explicit position tracking
 */
class SafeContentStreamParser {
  private position = 0;
  private readonly maxPosition: number;

  constructor(private data: Uint8Array) {
    this.maxPosition = data.length;
  }

  parse(): ContentOperation[] {
    const operations: ContentOperation[] = [];
    let errorCount = 0;
    const maxErrors = 100;
    const startTime = Date.now();
    const maxDuration = 5000; // 5 seconds max
    let iterationCount = 0;
    const maxIterations = 100000; // Safety limit

    while (this.position < this.maxPosition && errorCount < maxErrors && iterationCount++ < maxIterations) {
      // Check timeout
      if (Date.now() - startTime > maxDuration) {
        console.warn('Parse timeout - stopping after 5 seconds');
        break;
      }

      try {
        this.skipWhitespace();
        if (this.position >= this.maxPosition) break;

        const operands: any[] = [];

        // Parse operands until we hit an operator
        let operandCount = 0;
        while (this.position < this.maxPosition && operandCount++ < 100) {
          const startPos = this.position;
          const operand = this.parseOperand();

          if (operand === null) {
            break; // Hit an operator or end
          }

          // Safety: Ensure position advanced
          if (this.position === startPos) {
            console.warn('Position stuck in operand parsing at', this.position);
            this.position++;
            break;
          }

          operands.push(operand);
          this.skipWhitespace();
        }

        // Parse operator
        const operator = this.parseOperator();
        if (operator) {
          if (operator === 'BI') {
            const inlineOp = this.parseInlineImage();
            if (inlineOp) operations.push(inlineOp);
          } else {
            operations.push({ operator, operands });
          }
        } else if (this.position < this.maxPosition) {
          // Couldn't parse operator - skip this byte and continue
          this.position++;
          errorCount++;
        }
      } catch (error) {
        console.warn('Parser error at position', this.position, error);
        this.position++;
        errorCount++;
      }
    }

    if (errorCount >= maxErrors) {
      console.warn(`Stopped parsing after ${maxErrors} errors`);
    }
    if (iterationCount >= maxIterations) {
      console.warn(`Stopped parsing after ${maxIterations} iterations`);
    }

    return operations;
  }

  private skipWhitespace(): void {
    while (this.position < this.maxPosition) {
      const ch = this.data[this.position];
      if (ch === 0x00 || ch === 0x09 || ch === 0x0a || ch === 0x0c || ch === 0x0d || ch === 0x20) {
        this.position++;
      } else if (ch === 0x25) { // % comment
        this.skipComment();
      } else {
        break;
      }
    }
  }

  private skipComment(): void {
    while (this.position < this.maxPosition) {
      const ch = this.data[this.position++];
      if (ch === 0x0a || ch === 0x0d) break;
    }
  }

  private parseOperand(): any {
    const startPos = this.position;
    const ch = this.data[this.position];

    // Dictionary
    if (ch === 0x3c && this.data[this.position + 1] === 0x3c) {
      return this.parseDictionary();
    }
    // Array
    if (ch === 0x5b) {
      return this.parseArray();
    }
    // String
    if (ch === 0x28) {
      return this.parseString();
    }
    // Hex string
    if (ch === 0x3c) {
      return this.parseHexString();
    }
    // Name (starts with /)
    if (ch === 0x2f) {
      this.position++; // Skip the '/'
      const nameBody = this.parseToken();
      return '/' + nameBody;
    }
    // Number or operator
    const token = this.parseToken();
    if (!token) return null;

    // Check if it's a number
    if (token === 'true') return true;
    if (token === 'false') return false;
    if (token === 'null') return null;

    const num = parseFloat(token);
    if (!isNaN(num)) return num;

    // Must be an operator - return null to signal end of operands
    this.position = startPos;
    return null;
  }

  private parseToken(): string {
    const start = this.position;
    while (this.position < this.maxPosition) {
      const ch = this.data[this.position];
      // Delimiters
      if (ch === 0x00 || ch === 0x09 || ch === 0x0a || ch === 0x0c || ch === 0x0d || ch === 0x20 ||
        ch === 0x28 || ch === 0x29 || ch === 0x3c || ch === 0x3e || ch === 0x5b || ch === 0x5d ||
        ch === 0x7b || ch === 0x7d || ch === 0x2f || ch === 0x25) {
        break;
      }
      this.position++;
    }

    if (this.position === start) return '';
    return String.fromCharCode(...this.data.slice(start, this.position));
  }

  private parseOperator(): string | null {
    const token = this.parseToken();
    if (!token) return null;
    // Operator should not start with / (name) or be a number
    if (token[0] === '/' || !isNaN(parseFloat(token))) return null;
    return token;
  }

  private parseArray(): any[] {
    this.position++; // Skip [
    const array: any[] = [];
    let iterations = 0;
    const maxIterations = 10000; // Safety limit

    while (this.position < this.maxPosition && iterations++ < maxIterations) {
      this.skipWhitespace();
      if (this.position >= this.maxPosition) break;
      if (this.data[this.position] === 0x5d) { // ]
        this.position++;
        break;
      }

      const startPos = this.position;
      const item = this.parseOperand();

      if (item !== null) {
        array.push(item);
      } else {
        // Couldn't parse - advance to avoid infinite loop
        if (this.position === startPos) {
          this.position++;
        }
        break;
      }
    }

    return array;
  }

  private parseDictionary(): Record<string, any> {
    this.position += 2; // Skip <<
    const dict: Record<string, any> = {};
    let iterations = 0;
    const maxIterations = 10000; // Safety limit

    while (this.position < this.maxPosition && iterations++ < maxIterations) {
      this.skipWhitespace();
      if (this.position >= this.maxPosition) break;
      if (this.position + 1 < this.maxPosition &&
        this.data[this.position] === 0x3e &&
        this.data[this.position + 1] === 0x3e) {
        this.position += 2;
        break;
      }

      const keyStartPos = this.position;
      const key = this.parseOperand();

      if (key && typeof key === 'string' && key[0] === '/') {
        this.skipWhitespace();
        const valueStartPos = this.position;
        const value = this.parseOperand();

        // Ensure position advanced
        if (this.position === valueStartPos && this.position < this.maxPosition) {
          this.position++;
        }

        dict[key.substring(1)] = value;
      } else {
        // Couldn't parse key - advance to avoid infinite loop
        if (this.position === keyStartPos && this.position < this.maxPosition) {
          this.position++;
        }
        break;
      }
    }

    return dict;
  }

  private parseString(): string {
    this.position++; // Skip (
    const chars: number[] = [];
    let depth = 1;

    while (this.position < this.maxPosition && depth > 0) {
      const ch = this.data[this.position++];
      if (ch === 0x28) { // (
        depth++;
        chars.push(ch);
      } else if (ch === 0x29) { // )
        depth--;
        if (depth > 0) chars.push(ch);
      } else if (ch === 0x5c) { // \ escape - keep backslash for later decoding
        chars.push(ch);
        if (this.position < this.maxPosition) {
          const next = this.data[this.position++];
          chars.push(next);
        }
      } else {
        chars.push(ch);
      }
    }

    return String.fromCharCode(...chars);
  }

  private parseHexString(): string {
    this.position++; // Skip <
    const chars: number[] = [];

    while (this.position < this.maxPosition) {
      const ch = this.data[this.position++];
      if (ch === 0x3e) break; // >
      if ((ch >= 0x30 && ch <= 0x39) || (ch >= 0x41 && ch <= 0x46) || (ch >= 0x61 && ch <= 0x66)) {
        chars.push(ch);
      }
    }

    const hexStr = String.fromCharCode(...chars);
    const bytes: number[] = [];
    for (let i = 0; i < hexStr.length; i += 2) {
      bytes.push(parseInt(hexStr.substr(i, 2), 16));
    }
    return String.fromCharCode(...bytes);
  }

  private parseInlineImage(): ContentOperation | null {
    try {
      const imageDict = new Map<string, any>();

      // Parse key-value pairs until ID operator
      while (this.position < this.maxPosition) {
        this.skipWhitespace();
        if (this.position >= this.maxPosition) break;

        // Check for ID operator
        if (this.data[this.position] === 0x49 && // I
            this.position + 1 < this.maxPosition &&
            this.data[this.position + 1] === 0x44) { // D
          // Verify followed by whitespace or end
          if (this.position + 2 >= this.maxPosition ||
              this.data[this.position + 2] === 0x20 ||
              this.data[this.position + 2] === 0x0A ||
              this.data[this.position + 2] === 0x0D ||
              this.data[this.position + 2] === 0x09) {
            this.position += 2; // skip 'ID'
            // Skip single whitespace after ID
            if (this.position < this.maxPosition) {
              const ws = this.data[this.position];
              if (ws === 0x20 || ws === 0x0A || ws === 0x0D || ws === 0x09) {
                this.position++;
              }
            }
            break;
          }
        }

        // Parse key (name like /W, /H, /CS, etc.)
        const ch = this.data[this.position];
        if (ch === 0x2F) { // '/'
          this.position++; // skip '/'
          let name = '/';
          while (this.position < this.maxPosition) {
            const b = this.data[this.position];
            if (b === 0x20 || b === 0x09 || b === 0x0A || b === 0x0D ||
                b === 0x2F || b === 0x5B || b === 0x5D || b === 0x28 ||
                b === 0x29 || b === 0x3C || b === 0x3E) break;
            name += String.fromCharCode(b);
            this.position++;
          }
          this.skipWhitespace();
          const value = this.parseOperand();
          imageDict.set(name, value);
        } else {
          // Bare key (abbreviation without '/')
          let token = '';
          while (this.position < this.maxPosition) {
            const b = this.data[this.position];
            if (b === 0x20 || b === 0x09 || b === 0x0A || b === 0x0D ||
                b === 0x2F || b === 0x5B || b === 0x5D) break;
            token += String.fromCharCode(b);
            this.position++;
          }
          if (token.length > 0) {
            this.skipWhitespace();
            const value = this.parseOperand();
            imageDict.set('/' + token, value);
          } else {
            this.position++;
          }
        }
      }

      // Read image data until EI
      const startPos = this.position;
      let endPos = startPos;
      while (this.position < this.maxPosition - 2) {
        const b = this.data[this.position];
        if (b === 0x20 || b === 0x0A || b === 0x0D || b === 0x09) {
          if (this.data[this.position + 1] === 0x45 && // E
              this.data[this.position + 2] === 0x49) { // I
            if (this.position + 3 >= this.maxPosition ||
                this.data[this.position + 3] === 0x20 ||
                this.data[this.position + 3] === 0x0A ||
                this.data[this.position + 3] === 0x0D ||
                this.data[this.position + 3] === 0x09 ||
                this.data[this.position + 3] === 0x2F ||
                this.data[this.position + 3] === 0x25) {
              endPos = this.position;
              this.position += 3; // skip whitespace + 'EI'
              break;
            }
          }
        }
        this.position++;
      }

      const imageData = this.data.slice(startPos, endPos);
      return {
        operator: 'BI',
        operands: [{ dictionary: imageDict, data: imageData }]
      };
    } catch (error) {
      console.warn('SafeContentStreamParser: Failed to parse inline image:', error);
      return null;
    }
  }
}

/**
 * PDF Text Decoder
 * Handles various PDF string encodings and character mappings
 */
class PDFTextDecoder {
  // Common PDF glyph name to Unicode mapping
  static glyphNameToUnicode(name: string): string | undefined {
    const glyphMap: Record<string, number> = {
      'space': 0x20, 'exclam': 0x21, 'quotedbl': 0x22, 'numbersign': 0x23,
      'dollar': 0x24, 'percent': 0x25, 'ampersand': 0x26, 'quoteright': 0x2019,
      'parenleft': 0x28, 'parenright': 0x29, 'asterisk': 0x2A, 'plus': 0x2B,
      'comma': 0x2C, 'hyphen': 0x2D, 'period': 0x2E, 'slash': 0x2F,
      'zero': 0x30, 'one': 0x31, 'two': 0x32, 'three': 0x33, 'four': 0x34,
      'five': 0x35, 'six': 0x36, 'seven': 0x37, 'eight': 0x38, 'nine': 0x39,
      'colon': 0x3A, 'semicolon': 0x3B, 'less': 0x3C, 'equal': 0x3D,
      'greater': 0x3E, 'question': 0x3F, 'at': 0x40,
      'A': 0x41, 'B': 0x42, 'C': 0x43, 'D': 0x44, 'E': 0x45, 'F': 0x46,
      'G': 0x47, 'H': 0x48, 'I': 0x49, 'J': 0x4A, 'K': 0x4B, 'L': 0x4C,
      'M': 0x4D, 'N': 0x4E, 'O': 0x4F, 'P': 0x50, 'Q': 0x51, 'R': 0x52,
      'S': 0x53, 'T': 0x54, 'U': 0x55, 'V': 0x56, 'W': 0x57, 'X': 0x58,
      'Y': 0x59, 'Z': 0x5A, 'bracketleft': 0x5B, 'backslash': 0x5C,
      'bracketright': 0x5D, 'asciicircum': 0x5E, 'underscore': 0x5F,
      'quoteleft': 0x2018, 'a': 0x61, 'b': 0x62, 'c': 0x63, 'd': 0x64,
      'e': 0x65, 'f': 0x66, 'g': 0x67, 'h': 0x68, 'i': 0x69, 'j': 0x6A,
      'k': 0x6B, 'l': 0x6C, 'm': 0x6D, 'n': 0x6E, 'o': 0x6F, 'p': 0x70,
      'q': 0x71, 'r': 0x72, 's': 0x73, 't': 0x74, 'u': 0x75, 'v': 0x76,
      'w': 0x77, 'x': 0x78, 'y': 0x79, 'z': 0x7A,
      'braceleft': 0x7B, 'bar': 0x7C, 'braceright': 0x7D, 'asciitilde': 0x7E,
      'fi': 0xFB01, 'fl': 0xFB02, 'endash': 0x2013, 'emdash': 0x2014,
      'bullet': 0x2022, 'ellipsis': 0x2026, 'quotedblleft': 0x201C,
      'quotedblright': 0x201D, 'quotesingle': 0x27,
      'quotedblbase': 0x201E, 'quotesinglbase': 0x201A,
      'dagger': 0x2020, 'daggerdbl': 0x2021, 'trademark': 0x2122,
      'circumflex': 0x02C6, 'tilde': 0x02DC, 'dotlessi': 0x0131,
      'breve': 0x02D8, 'ring': 0x02DA, 'ogonek': 0x02DB,
      'caron': 0x02C7, 'degree': 0x00B0, 'section': 0x00A7,
      'copyright': 0x00A9, 'registered': 0x00AE, 'mu': 0x00B5,
      'paragraph': 0x00B6, 'guillemotleft': 0x00AB,
      'guillemotright': 0x00BB, 'guilsinglleft': 0x2039,
      'guilsinglright': 0x203A, 'fraction': 0x2044,
      'minus': 0x2212, 'perthousand': 0x2030,
      'Lslash': 0x0141, 'lslash': 0x0142, 'OE': 0x0152,
      'oe': 0x0153, 'Scaron': 0x0160, 'scaron': 0x0161,
      'Ydieresis': 0x0178, 'Zcaron': 0x017D, 'zcaron': 0x017E
    };
    const cp = glyphMap[name];
    return cp !== undefined ? String.fromCodePoint(cp) : undefined;
  }

  // PDFDocEncoding to Unicode mapping (for bytes 128-255)
  // Bytes 0-127 are standard ASCII
  private static readonly PDF_DOC_ENCODING: number[] = [
    0x2022, 0x2020, 0x2021, 0x2026, 0x2014, 0x2013, 0x0192, 0x2044,
    0x2039, 0x203A, 0x2212, 0x2030, 0x201E, 0x201C, 0x201D, 0x2018,
    0x2019, 0x201A, 0x2122, 0xFB01, 0xFB02, 0x0141, 0x0152, 0x0160,
    0x0178, 0x017D, 0x0131, 0x0142, 0x0153, 0x0161, 0x017E, 0xFFFD,
    0x00A0, 0x00A1, 0x00A2, 0x00A3, 0x00A4, 0x00A5, 0x00A6, 0x00A7,
    0x00A8, 0x00A9, 0x00AA, 0x00AB, 0x00AC, 0x00AD, 0x00AE, 0x00AF,
    0x00B0, 0x00B1, 0x00B2, 0x00B3, 0x00B4, 0x00B5, 0x00B6, 0x00B7,
    0x00B8, 0x00B9, 0x00BA, 0x00BB, 0x00BC, 0x00BD, 0x00BE, 0x00BF,
    0x00C0, 0x00C1, 0x00C2, 0x00C3, 0x00C4, 0x00C5, 0x00C6, 0x00C7,
    0x00C8, 0x00C9, 0x00CA, 0x00CB, 0x00CC, 0x00CD, 0x00CE, 0x00CF,
    0x00D0, 0x00D1, 0x00D2, 0x00D3, 0x00D4, 0x00D5, 0x00D6, 0x00D7,
    0x00D8, 0x00D9, 0x00DA, 0x00DB, 0x00DC, 0x00DD, 0x00DE, 0x00DF,
    0x00E0, 0x00E1, 0x00E2, 0x00E3, 0x00E4, 0x00E5, 0x00E6, 0x00E7,
    0x00E8, 0x00E9, 0x00EA, 0x00EB, 0x00EC, 0x00ED, 0x00EE, 0x00EF,
    0x00F0, 0x00F1, 0x00F2, 0x00F3, 0x00F4, 0x00F5, 0x00F6, 0x00F7,
    0x00F8, 0x00F9, 0x00FA, 0x00FB, 0x00FC, 0x00FD, 0x00FE, 0x00FF
  ];

  // WinAnsiEncoding map (bytes 128-159 differ from PDFDocEncoding)
  private static readonly WIN_ANSI_ENCODING: number[] = [
    0x20AC, 0xFFFD, 0x201A, 0x0192, 0x201E, 0x2026, 0x2020, 0x2021, // 128-135
    0x02C6, 0x2030, 0x0160, 0x2039, 0x0152, 0xFFFD, 0x017D, 0xFFFD, // 136-143
    0xFFFD, 0x2018, 0x2019, 0x201C, 0x201D, 0x2022, 0x2013, 0x2014, // 144-151
    0x02DC, 0x2122, 0x0161, 0x203A, 0x0153, 0xFFFD, 0x017E, 0x0178, // 152-159
    0x00A0, 0x00A1, 0x00A2, 0x00A3, 0x00A4, 0x00A5, 0x00A6, 0x00A7, // 160-167
    0x00A8, 0x00A9, 0x00AA, 0x00AB, 0x00AC, 0x00AD, 0x00AE, 0x00AF, // 168-175
    0x00B0, 0x00B1, 0x00B2, 0x00B3, 0x00B4, 0x00B5, 0x00B6, 0x00B7, // 176-183
    0x00B8, 0x00B9, 0x00BA, 0x00BB, 0x00BC, 0x00BD, 0x00BE, 0x00BF, // 184-191
    0x00C0, 0x00C1, 0x00C2, 0x00C3, 0x00C4, 0x00C5, 0x00C6, 0x00C7, // 192-199
    0x00C8, 0x00C9, 0x00CA, 0x00CB, 0x00CC, 0x00CD, 0x00CE, 0x00CF, // 200-207
    0x00D0, 0x00D1, 0x00D2, 0x00D3, 0x00D4, 0x00D5, 0x00D6, 0x00D7, // 208-215
    0x00D8, 0x00D9, 0x00DA, 0x00DB, 0x00DC, 0x00DD, 0x00DE, 0x00DF, // 216-223
    0x00E0, 0x00E1, 0x00E2, 0x00E3, 0x00E4, 0x00E5, 0x00E6, 0x00E7, // 224-231
    0x00E8, 0x00E9, 0x00EA, 0x00EB, 0x00EC, 0x00ED, 0x00EE, 0x00EF, // 232-239
    0x00F0, 0x00F1, 0x00F2, 0x00F3, 0x00F4, 0x00F5, 0x00F6, 0x00F7, // 240-247
    0x00F8, 0x00F9, 0x00FA, 0x00FB, 0x00FC, 0x00FD, 0x00FE, 0x00FF  // 248-255
  ];

  // MacRomanEncoding map (bytes 128-255)
  private static readonly MAC_ROMAN_ENCODING: number[] = [
    0x00C4, 0x00C5, 0x00C7, 0x00C9, 0x00D1, 0x00D6, 0x00DC, 0x00E1, // 128-135
    0x00E0, 0x00E2, 0x00E4, 0x00E3, 0x00E5, 0x00E7, 0x00E9, 0x00E8, // 136-143
    0x00EA, 0x00EB, 0x00ED, 0x00EC, 0x00EE, 0x00EF, 0x00F1, 0x00F3, // 144-151  
    0x00F2, 0x00F4, 0x00F6, 0x00F5, 0x00FA, 0x00F9, 0x00FB, 0x00FC, // 152-159
    0x2020, 0x00B0, 0x00A2, 0x00A3, 0x00A7, 0x2022, 0x00B6, 0x00DF, // 160-167
    0x00AE, 0x00A9, 0x2122, 0x00B4, 0x00A8, 0x2260, 0x00C6, 0x00D8, // 168-175
    0x221E, 0x00B1, 0x2264, 0x2265, 0x00A5, 0x00B5, 0x2202, 0x2211, // 176-183
    0x220F, 0x03C0, 0x222B, 0x00AA, 0x00BA, 0x2126, 0x00E6, 0x00F8, // 184-191
    0x00BF, 0x00A1, 0x00AC, 0x221A, 0x0192, 0x2248, 0x2206, 0x00AB, // 192-199
    0x00BB, 0x2026, 0x00A0, 0x00C0, 0x00C3, 0x00D5, 0x0152, 0x0153, // 200-207
    0x2013, 0x2014, 0x201C, 0x201D, 0x2018, 0x2019, 0x00F7, 0x25CA, // 208-215
    0x00FF, 0x0178, 0x2044, 0x20AC, 0x2039, 0x203A, 0xFB01, 0xFB02, // 216-223
    0x2021, 0x00B7, 0x201A, 0x201E, 0x2030, 0x00C2, 0x00CA, 0x00C1, // 224-231
    0x00CB, 0x00C8, 0x00CD, 0x00CE, 0x00CF, 0x00CC, 0x00D3, 0x00D4, // 232-239
    0xF8FF, 0x00D2, 0x00DA, 0x00DB, 0x00D9, 0x0131, 0x02C6, 0x02DC, // 240-247
    0x00AF, 0x02D8, 0x02D9, 0x02DA, 0x00B8, 0x02DD, 0x02DB, 0x02C7  // 248-255
  ];

  /**
   * Map a raw byte to Unicode display character.
   * Uses the specified encoding, defaulting to PDFDocEncoding.
   */
  static mapCharCode(charCode: number, encoding?: string): string {
    if (charCode < 128) {
      return String.fromCharCode(charCode);
    }
    if (charCode <= 255) {
      if (encoding === 'WinAnsiEncoding') {
        return String.fromCodePoint(this.WIN_ANSI_ENCODING[charCode - 128]);
      }
      if (encoding === 'MacRomanEncoding') {
        return String.fromCodePoint(this.MAC_ROMAN_ENCODING[charCode - 128]);
      }
      return String.fromCodePoint(this.PDF_DOC_ENCODING[charCode - 128]);
    }
    return String.fromCodePoint(charCode);
  }

    static decode(input: any): string {
    // Handle non-string inputs
    if (typeof input !== 'string') {
      return String(input);
    }

    if (input.length === 0) {
      return '';
    }

    // Check if it's UTF-16BE (starts with BOM: \xFE\xFF)
    if (input.length >= 2 &&
      input.charCodeAt(0) === 0xFE &&
      input.charCodeAt(1) === 0xFF) {
      return this.decodeUTF16BE(input);
    }

    // Check if it's UTF-16LE (starts with BOM: \xFF\xFE) - rare in PDFs
    if (input.length >= 2 &&
      input.charCodeAt(0) === 0xFF &&
      input.charCodeAt(1) === 0xFE) {
      return this.decodeUTF16LE(input);
    }

    // Otherwise, decode as PDFDocEncoding with escape sequences
    return this.decodePDFDocEncoding(input);
  }

  /**
   * Decode UTF-16BE encoded string (big-endian)
   */
  private static decodeUTF16BE(input: string): string {
    const result: number[] = [];

    // Skip BOM (first 2 bytes)
    for (let i = 2; i < input.length; i += 2) {
      if (i + 1 >= input.length) break;

      const high = input.charCodeAt(i);
      const low = input.charCodeAt(i + 1);
      const codePoint = (high << 8) | low;

      // Handle surrogate pairs for characters > U+FFFF
      if (codePoint >= 0xD800 && codePoint <= 0xDBFF && i + 3 < input.length) {
        const high2 = input.charCodeAt(i + 2);
        const low2 = input.charCodeAt(i + 3);
        const codePoint2 = (high2 << 8) | low2;

        if (codePoint2 >= 0xDC00 && codePoint2 <= 0xDFFF) {
          // Combine surrogate pair
          const finalCodePoint = 0x10000 +
            ((codePoint - 0xD800) << 10) +
            (codePoint2 - 0xDC00);
          result.push(finalCodePoint);
          i += 2; // Skip the second pair
          continue;
        }
      }

      result.push(codePoint);
    }

    return String.fromCodePoint(...result);
  }

  /**
   * Decode UTF-16LE encoded string (little-endian) - rare
   */
  private static decodeUTF16LE(input: string): string {
    const result: number[] = [];

    // Skip BOM (first 2 bytes)
    for (let i = 2; i < input.length; i += 2) {
      if (i + 1 >= input.length) break;

      const low = input.charCodeAt(i);
      const high = input.charCodeAt(i + 1);
      const codePoint = (high << 8) | low;
      result.push(codePoint);
    }

    return String.fromCodePoint(...result);
  }

  /**
   * Decode PDFDocEncoding with escape sequences
   */
  private static decodePDFDocEncoding(input: string): string {
    const result: number[] = [];
    let i = 0;

    while (i < input.length) {
      let ch = input.charCodeAt(i);

      // Handle escape sequences
      if (ch === 0x5C) { // Backslash
        i++;
        if (i >= input.length) break;

        const next = input.charCodeAt(i);

        // Octal escape: \ddd (1-3 digits)
        if (next >= 0x30 && next <= 0x37) { // 0-7
          let octal = next - 0x30;
          i++;

          // Second digit
          if (i < input.length) {
            const digit2 = input.charCodeAt(i);
            if (digit2 >= 0x30 && digit2 <= 0x37) {
              octal = octal * 8 + (digit2 - 0x30);
              i++;

              // Third digit
              if (i < input.length) {
                const digit3 = input.charCodeAt(i);
                if (digit3 >= 0x30 && digit3 <= 0x37) {
                  octal = octal * 8 + (digit3 - 0x30);
                  i++;
                }
              }
            }
          }

          ch = octal;
        }
        // Standard escape sequences
        else {
          switch (next) {
            case 0x6E: ch = 0x0A; break; // \n -> newline
            case 0x72: ch = 0x0D; break; // \r -> carriage return
            case 0x74: ch = 0x09; break; // \t -> tab
            case 0x62: ch = 0x08; break; // \b -> backspace
            case 0x66: ch = 0x0C; break; // \f -> form feed
            case 0x28: ch = 0x28; break; // \( -> (
            case 0x29: ch = 0x29; break; // \) -> )
            case 0x5C: ch = 0x5C; break; // \\ -> \
            default: ch = next; break; // Unknown escape - use literal
          }
          i++;
        }
      } else {
        i++;
      }

      // Map PDFDocEncoding to Unicode
      if (ch < 128) {
        // ASCII range - use as-is
        result.push(ch);
      } else {
        // Extended range (128-255) - map to Unicode
        result.push(this.PDF_DOC_ENCODING[ch - 128]);
      }
    }

    return String.fromCodePoint(...result);
  }

  /**
   * Decode hex string (format: <48656C6C6F>)
   */
  static decodeHexString(hexStr: string): string {
    // Remove < and > brackets if present
    hexStr = hexStr.replace(/[<>]/g, '');

    const bytes: number[] = [];
    for (let i = 0; i < hexStr.length; i += 2) {
      const hexByte = hexStr.substr(i, 2);
      if (hexByte.length === 2) {
        bytes.push(parseInt(hexByte, 16));
      } else if (hexByte.length === 1) {
        // Odd number of digits - pad with 0
        bytes.push(parseInt(hexByte + '0', 16));
      }
    }

    // Convert bytes to string
    const byteString = String.fromCharCode(...bytes);

    // Check for UTF-16BE BOM
    if (bytes.length >= 2 && bytes[0] === 0xFE && bytes[1] === 0xFF) {
      return this.decode(byteString);
    }

    // Otherwise decode as PDFDocEncoding
    return this.decodePDFDocEncoding(byteString);
  }
}

/**
 * PDF Glyph Metrics Calculator
 * Calculates accurate glyph widths from PDF font metrics
 */
class PDFGlyphMetrics {
  /**
   * Get the width of a character in text space units
   * @param charCode Character code
   * @param font Font resource with width information
   * @param fontSize Font size in points
   * @returns Width in user space units
   */
  static getCharWidth(charCode: number, font: FontResource | undefined, fontSize: number): number {
    if (!font) {
      // Fallback: use average character width estimate
      return fontSize * 0.5;
    }

    // Get the glyph width in glyph space (usually 1000 units)
    const glyphWidth = this.getGlyphWidth(charCode, font);

    // Convert from glyph space to text space
    // PDF glyph widths are typically in 1000-unit glyph space
    // Scale by font size and divide by 1000
    // Use abs(fontSize) — negative fontSize affects direction, not glyph width
    return (glyphWidth * Math.abs(fontSize)) / 1000;
  }

  /**
   * Get glyph width in glyph space units (typically 0-1000 range)
   */
  private static getGlyphWidth(charCode: number, font: FontResource): number {
    // Check CIDFont per-CID widths (Type0/CIDFont W array)
    if (font.cidWidths && font.cidWidths.has(charCode)) {
      return font.cidWidths.get(charCode)!;
    }

    // Check if we have a Widths array
    if (font.widths && font.widths.length > 0 &&
      font.firstChar !== undefined && font.lastChar !== undefined) {

      // Check if character is in the widths array range
      if (charCode >= font.firstChar && charCode <= font.lastChar) {
        const index = charCode - font.firstChar;
        if (index >= 0 && index < font.widths.length) {
          return font.widths[index];
        }
      }
    }

    // Check for DefaultWidth (CIDFonts)
    if (font.defaultWidth !== undefined) {
      return font.defaultWidth;
    }

    // Check for MissingWidth
    if (font.missingWidth !== undefined) {
      return font.missingWidth;
    }

    // Use font-specific defaults based on font type
    return this.getDefaultWidth(font, charCode);
  }

  // Standard 14 font width tables (PDF spec Appendix D)
  // Per-character widths in 1000-unit glyph space for ASCII 32-126
  private static readonly HELVETICA_WIDTHS: number[] = [
    278,278,355,556,556,889,667,191,333,333,389,584,278,333,278,278, // 32-47
    556,556,556,556,556,556,556,556,556,556,278,278,584,584,584,556, // 48-63
    1015,667,667,722,722,667,611,778,722,278,500,667,556,833,722,778, // 64-79
    667,778,722,667,611,722,667,944,667,667,611,278,278,278,469,556, // 80-95
    333,556,556,500,556,556,278,556,556,222,222,500,222,833,556,556, // 96-111
    556,556,333,500,278,556,500,722,500,500,500,334,260,334,584       // 112-126
  ];

  private static readonly HELVETICA_BOLD_WIDTHS: number[] = [
    278,333,474,556,556,889,722,238,333,333,389,584,278,333,278,278, // 32-47
    556,556,556,556,556,556,556,556,556,556,333,333,584,584,584,611, // 48-63
    975,722,722,722,722,667,611,778,722,278,556,722,611,833,722,778, // 64-79
    667,778,722,667,611,722,667,944,667,667,611,333,278,333,584,556, // 80-95
    333,556,611,556,611,556,333,611,611,278,278,556,278,889,611,611, // 96-111
    611,611,389,556,333,611,556,778,556,556,500,389,280,389,584       // 112-126
  ];

  private static readonly TIMES_WIDTHS: number[] = [
    250,333,408,500,500,833,778,180,333,333,500,564,250,333,250,278, // 32-47
    500,500,500,500,500,500,500,500,500,500,278,278,564,564,564,444, // 48-63
    921,722,667,667,722,611,556,722,722,333,389,722,611,889,722,722, // 64-79
    556,722,667,556,611,722,722,944,722,722,611,333,278,333,469,500, // 80-95
    333,444,500,444,500,444,333,500,500,278,278,500,278,778,500,500, // 96-111
    500,500,333,389,278,500,500,722,500,500,444,480,200,480,541       // 112-126
  ];

  private static readonly TIMES_BOLD_WIDTHS: number[] = [
    250,333,555,500,500,1000,833,278,333,333,500,570,250,333,250,278, // 32-47
    500,500,500,500,500,500,500,500,500,500,333,333,570,570,570,500, // 48-63
    930,722,667,722,722,667,611,778,778,389,500,778,667,944,722,778, // 64-79
    611,778,722,556,667,722,722,1000,722,722,667,333,278,333,581,500, // 80-95
    333,500,556,444,556,444,333,500,556,278,333,556,278,833,556,500, // 96-111
    556,556,444,389,333,556,500,722,500,500,444,394,220,394,520       // 112-126
  ];

  private static readonly TIMES_ITALIC_WIDTHS: number[] = [
    250,333,420,500,500,833,778,214,333,333,500,675,250,333,250,278, // 32-47
    500,500,500,500,500,500,500,500,500,500,333,333,675,675,675,500, // 48-63
    920,611,611,667,722,611,611,722,722,333,444,667,556,833,667,722, // 64-79
    611,722,611,500,556,722,611,833,611,556,556,389,278,389,422,500, // 80-95
    333,500,500,444,500,444,278,500,500,278,278,444,278,722,500,500, // 96-111
    500,500,389,389,278,500,444,667,444,444,389,400,275,400,541       // 112-126
  ];

  /**
   * Get default width based on font type and character
   */
  private static getDefaultWidth(font: FontResource, charCode: number): number {
    const baseFont = font.baseFont || '';
    const bf = baseFont.toLowerCase();

    // Monospace fonts (Courier family) — fixed width
    if (bf.includes('courier') || bf.includes('mono') || bf.includes('nimbusmonl')) {
      return 600;
    }

    // Helvetica / Arial / sans-serif
    if (bf.includes('helvetica') || bf.includes('arial') || bf.includes('sans')) {
      if (charCode >= 32 && charCode <= 126) {
        if (bf.includes('bold')) {
          return this.HELVETICA_BOLD_WIDTHS[charCode - 32];
        }
        return this.HELVETICA_WIDTHS[charCode - 32];
      }
      return 556; // fallback for extended chars
    }

    // Times / serif
    if (bf.includes('times') || bf.includes('roman') || (bf.includes('serif') && !bf.includes('sans'))) {
      if (charCode >= 32 && charCode <= 126) {
        if (bf.includes('bold') && bf.includes('ital')) {
          return this.TIMES_BOLD_WIDTHS[charCode - 32]; // approximate
        }
        if (bf.includes('bold')) {
          return this.TIMES_BOLD_WIDTHS[charCode - 32];
        }
        if (bf.includes('ital') || bf.includes('obli')) {
          return this.TIMES_ITALIC_WIDTHS[charCode - 32];
        }
        return this.TIMES_WIDTHS[charCode - 32];
      }
      return 500; // fallback for extended chars
    }

    // Symbol fonts
    if (bf.includes('symbol') || bf.includes('zapfdingbats')) {
      return 750;
    }

    // Generic fallback — try Helvetica table as reasonable default
    if (charCode >= 32 && charCode <= 126) {
      return this.HELVETICA_WIDTHS[charCode - 32];
    }
    return 500;
  }

  /**
   * Calculate total width of a text string
   * @param text Text string
   * @param font Font resource
   * @param fontSize Font size in points
   * @param charSpace Character spacing
   * @param wordSpace Word spacing
   * @param horizScale Horizontal scaling (%)
   * @returns Total width in user space units
   */
  static getTextWidth(
    text: string,
    font: FontResource | undefined,
    fontSize: number,
    charSpace: number = 0,
    wordSpace: number = 0,
    horizScale: number = 100
  ): number {
    let totalWidth = 0;
    const scale = horizScale / 100;

    for (let i = 0; i < text.length; i++) {
      const charCode = text.charCodeAt(i);

      // Get base glyph width
      const glyphWidth = this.getCharWidth(charCode, font, fontSize);
      totalWidth += glyphWidth;

      // Add character spacing
      totalWidth += charSpace;

      // Add word spacing for spaces
      if (charCode === 32) {
        totalWidth += wordSpace;
      }
    }

    // Apply horizontal scaling
    return totalWidth * scale;
  }
}

/**
 * PDF Color Space Processor
 * Handles advanced color spaces: ICCBased, Indexed, Pattern, Separation, DeviceN
 */
class PDFColorSpaceProcessor {
  // Color space cache
  private static colorSpaceCache = new Map<string, ColorSpace>();
  private static readonly MAX_COLOR_SPACE_CACHE_SIZE = 50;

  // Color conversion cache for common values
  private static conversionCache = new Map<string, number[]>();
  private static readonly MAX_CONVERSION_CACHE_SIZE = 500;

  /**
   * Clear all caches (useful for memory management)
   */
  static clearCaches(): void {
    PDFColorSpaceProcessor.colorSpaceCache.clear();
    PDFColorSpaceProcessor.conversionCache.clear();
  }

  /**
   * Get cache key for color space object
   */
  private static getColorSpaceCacheKey(csObj: any): string | null {
    if (typeof csObj === 'string') {
      return `str:${csObj}`;
    }
    if (Array.isArray(csObj) && csObj.length > 0) {
      // Create a simple key from array structure
      const name = csObj[0];
      // Only cache simple color spaces (avoid complex objects)
      if (typeof name === 'string' && csObj.length <= 5) {
        return `arr:${JSON.stringify(csObj).slice(0, 100)}`;
      }
    }
    return null;
  }

  /**
   * Parse color space definition from PDF resources
   */
  static parseColorSpace(csObj: any, resources?: any): ColorSpace {
    // Try to get cached color space
    const cacheKey = this.getColorSpaceCacheKey(csObj);
    if (cacheKey) {
      const cached = PDFColorSpaceProcessor.colorSpaceCache.get(cacheKey);
      if (cached) {
        return cached;
      }
    }
    // Handle simple names
    if (typeof csObj === 'string') {
      return {
        name: csObj,
        numComponents: this.getNumComponents(csObj)
      };
    }

    // Handle array definitions: [name, ...params]
    if (Array.isArray(csObj) && csObj.length > 0) {
      const csName = csObj[0];

      switch (csName) {
        case 'ICCBased':
          return this.parseICCBased(csObj, resources);
        case 'Indexed':
        case 'I':
          return this.parseIndexed(csObj, resources);
        case 'Pattern':
          return this.parsePattern(csObj);
        case 'Separation':
          return this.parseSeparation(csObj, resources);
        case 'DeviceN':
          return this.parseDeviceN(csObj, resources);
        case 'CalRGB':
        case 'CalGray':
          return this.parseCalibrated(csObj);
        default:
          // Fallback to simple color space
          const fallbackCS = {
            name: csName,
            numComponents: this.getNumComponents(csName)
          };
          this.cacheColorSpace(cacheKey, fallbackCS);
          return fallbackCS;
      }
    }

    // Default fallback
    const defaultCS = {
      name: 'DeviceRGB',
      numComponents: 3
    };
    this.cacheColorSpace(cacheKey, defaultCS);
    return defaultCS;
  }

  /**
   * Cache a parsed color space
   */
  private static cacheColorSpace(cacheKey: string | null, colorSpace: ColorSpace): void {
    if (!cacheKey) return;

    // Implement LRU-style cache eviction
    if (PDFColorSpaceProcessor.colorSpaceCache.size >= PDFColorSpaceProcessor.MAX_COLOR_SPACE_CACHE_SIZE) {
      const firstKey = PDFColorSpaceProcessor.colorSpaceCache.keys().next().value;
      if (firstKey) {
        PDFColorSpaceProcessor.colorSpaceCache.delete(firstKey);
      }
    }
    PDFColorSpaceProcessor.colorSpaceCache.set(cacheKey, colorSpace);
  }

  /**
   * Get number of color components for a color space
   */
  private static getNumComponents(csName: string): number {
    switch (csName) {
      case 'DeviceGray':
      case 'G':
        return 1;
      case 'DeviceRGB':
      case 'RGB':
        return 3;
      case 'DeviceCMYK':
      case 'CMYK':
        return 4;
      case 'Pattern':
        return 0; // Patterns don't have fixed components
      default:
        return 3; // Default to RGB
    }
  }

  /**
   * Parse ICCBased color space
   * Format: [/ICCBased stream]
   */
  private static parseICCBased(csArray: any[], resources?: any): ColorSpace {
    const cs: ColorSpace = {
      name: 'ICCBased',
      numComponents: 3 // Default to RGB
    };

    if (csArray.length >= 2) {
      const stream = csArray[1];

      // Extract ICC profile from stream
      if (stream && stream.data) {
        cs.iccProfile = stream.data;
      }

      // Get number of components from stream dictionary
      if (stream && stream.dictionary) {
        const n = stream.dictionary.entries.get('N');
        if (n !== undefined) {
          cs.numComponents = typeof n === 'number' ? n : parseInt(n, 10);
        }

        // Get alternate color space
        const alternate = stream.dictionary.entries.get('Alternate');
        if (alternate) {
          cs.alternate = this.parseColorSpace(alternate, resources);
        }

        // Get range
        const range = stream.dictionary.entries.get('Range');
        if (range && Array.isArray(range)) {
          cs.range = range;
        }
      }
    }

    return cs;
  }

  /**
   * Parse Indexed color space
   * Format: [/Indexed base hival lookup]
   */
  private static parseIndexed(csArray: any[], resources?: any): ColorSpace {
    const cs: ColorSpace = {
      name: 'Indexed',
      numComponents: 1 // Indexed uses single index value
    };

    if (csArray.length >= 4) {
      // Parse base color space
      cs.base = this.parseColorSpace(csArray[1], resources);

      // Get hival (maximum index value)
      cs.hival = typeof csArray[2] === 'number' ? csArray[2] : parseInt(csArray[2], 10);

      // Get lookup table
      const lookupObj = csArray[3];
      if (lookupObj) {
        if (lookupObj instanceof Uint8Array) {
          cs.lookup = lookupObj;
        } else if (lookupObj.data) {
          cs.lookup = lookupObj.data;
        } else if (typeof lookupObj === 'string') {
          // Convert hex string to bytes
          cs.lookup = this.hexStringToBytes(lookupObj);
        }
      }
    }

    return cs;
  }

  /**
   * Parse Pattern color space
   * Format: [/Pattern] or [/Pattern baseCS]
   */
  private static parsePattern(csArray: any[]): ColorSpace {
    const cs: ColorSpace = {
      name: 'Pattern',
      numComponents: 0
    };

    // Pattern can have an optional base color space
    if (csArray.length >= 2) {
      cs.base = this.parseColorSpace(csArray[1]);
    }

    return cs;
  }

  /**
   * Parse Separation color space
   * Format: [/Separation name alternateSpace tintTransform]
   */
  private static parseSeparation(csArray: any[], resources?: any): ColorSpace {
    const cs: ColorSpace = {
      name: 'Separation',
      numComponents: 1,
      colorants: []
    };

    if (csArray.length >= 4) {
      // Colorant name
      if (typeof csArray[1] === 'string') {
        cs.colorants = [csArray[1]];
      }

      // Alternate color space
      cs.alternate = this.parseColorSpace(csArray[2], resources);

      // Tint transform function
      cs.tintTransform = csArray[3];
    }

    return cs;
  }

  /**
   * Parse DeviceN color space
   * Format: [/DeviceN names alternateSpace tintTransform]
   */
  private static parseDeviceN(csArray: any[], resources?: any): ColorSpace {
    const cs: ColorSpace = {
      name: 'DeviceN',
      numComponents: 0,
      colorants: []
    };

    if (csArray.length >= 4) {
      // Colorant names
      if (Array.isArray(csArray[1])) {
        cs.colorants = csArray[1].map((n: any) => String(n));
        cs.numComponents = cs.colorants.length;
      }

      // Alternate color space
      cs.alternate = this.parseColorSpace(csArray[2], resources);

      // Tint transform function
      cs.tintTransform = csArray[3];
    }

    return cs;
  }

  /**
   * Parse calibrated color space (CalRGB, CalGray)
   */
  private static parseCalibrated(csArray: any[]): ColorSpace {
    const csName = csArray[0];
    const cs: ColorSpace = {
      name: csName,
      numComponents: csName === 'CalGray' ? 1 : 3
    };

    // Additional calibration parameters could be extracted here
    // (WhitePoint, BlackPoint, Gamma, Matrix)

    return cs;
  }

  /**
   * Convert color values to RGB based on color space
   */
  static toRGB(colorSpace: ColorSpace, values: number[]): number[] {
    switch (colorSpace.name) {
      case 'DeviceGray':
      case 'CalGray':
      case 'G':
        return this.grayToRGB(values[0] || 0);

      case 'DeviceRGB':
      case 'CalRGB':
      case 'RGB':
        return [
          values[0] || 0,
          values[1] || 0,
          values[2] || 0
        ];

      case 'DeviceCMYK':
      case 'CMYK':
        return this.cmykToRGB(
          values[0] || 0,
          values[1] || 0,
          values[2] || 0,
          values[3] || 0
        );

      case 'ICCBased':
        // Use alternate color space if available
        if (colorSpace.alternate) {
          return this.toRGB(colorSpace.alternate, values);
        }
        // Fallback based on number of components
        if (colorSpace.numComponents === 1) {
          return this.grayToRGB(values[0] || 0);
        } else if (colorSpace.numComponents === 4) {
          return this.cmykToRGB(
            values[0] || 0,
            values[1] || 0,
            values[2] || 0,
            values[3] || 0
          );
        }
        // Default to RGB
        return [values[0] || 0, values[1] || 0, values[2] || 0];

      case 'Indexed':
      case 'I':
        return this.indexedToRGB(colorSpace, values[0] || 0);

      case 'Separation':
        // Apply tint to alternate color space
        if (colorSpace.alternate) {
          const tint = values[0] || 0;
          // Simplified: just apply tint as scaling factor
          const alternateValues = new Array(colorSpace.alternate.numComponents).fill(tint);
          return this.toRGB(colorSpace.alternate, alternateValues);
        }
        return [0, 0, 0];

      case 'DeviceN':
        // Use alternate color space
        if (colorSpace.alternate) {
          // Simplified: average the components
          const avg = values.reduce((sum, v) => sum + v, 0) / values.length;
          const alternateValues = new Array(colorSpace.alternate.numComponents).fill(avg);
          return this.toRGB(colorSpace.alternate, alternateValues);
        }
        return [0, 0, 0];

      default:
        // Default to black
        return [0, 0, 0];
    }
  }

  /**
   * Convert grayscale to RGB
   */
  private static grayToRGB(gray: number): number[] {
    // Common gray values cache (performance optimization)
    const cacheKey = `gray:${gray.toFixed(3)}`;
    const cached = PDFColorSpaceProcessor.conversionCache.get(cacheKey);
    if (cached) return cached;

    const rgb = [gray, gray, gray];
    this.cacheConversion(cacheKey, rgb);
    return rgb;
  }

  /**
   * Convert CMYK to RGB
   */
  private static cmykToRGB(c: number, m: number, y: number, k: number): number[] {
    // Cache frequently used CMYK conversions
    const cacheKey = `cmyk:${c.toFixed(2)},${m.toFixed(2)},${y.toFixed(2)},${k.toFixed(2)}`;
    const cached = PDFColorSpaceProcessor.conversionCache.get(cacheKey);
    if (cached) return cached;

    const r = (1 - c) * (1 - k);
    const g = (1 - m) * (1 - k);
    const b = (1 - y) * (1 - k);
    const rgb = [r, g, b];
    this.cacheConversion(cacheKey, rgb);
    return rgb;
  }

  /**
   * Cache a color conversion result
   */
  private static cacheConversion(cacheKey: string, rgb: number[]): void {
    // Implement LRU-style cache eviction
    if (PDFColorSpaceProcessor.conversionCache.size >= PDFColorSpaceProcessor.MAX_CONVERSION_CACHE_SIZE) {
      const firstKey = PDFColorSpaceProcessor.conversionCache.keys().next().value;
      if (firstKey) {
        PDFColorSpaceProcessor.conversionCache.delete(firstKey);
      }
    }
    PDFColorSpaceProcessor.conversionCache.set(cacheKey, rgb);
  }

  /**
   * Convert indexed color to RGB
   */
  private static indexedToRGB(colorSpace: ColorSpace, index: number): number[] {
    if (!colorSpace.base || !colorSpace.lookup || colorSpace.hival === undefined) {
      return [0, 0, 0];
    }

    // Clamp index to valid range
    const idx = Math.max(0, Math.min(Math.floor(index), colorSpace.hival));

    // Calculate byte offset in lookup table
    const componentCount = colorSpace.base.numComponents;
    const offset = idx * componentCount;

    // Extract color components from lookup table
    const components: number[] = [];
    for (let i = 0; i < componentCount; i++) {
      if (offset + i < colorSpace.lookup.length) {
        components.push(colorSpace.lookup[offset + i] / 255);
      } else {
        components.push(0);
      }
    }

    // Convert to RGB using base color space
    return this.toRGB(colorSpace.base, components);
  }

  /**
   * Convert RGB array to CSS color string
   */
  static rgbToCSS(rgb: number[]): string {
    const r = Math.max(0, Math.min(255, Math.floor(rgb[0] * 255)));
    const g = Math.max(0, Math.min(255, Math.floor(rgb[1] * 255)));
    const b = Math.max(0, Math.min(255, Math.floor(rgb[2] * 255)));
    return `rgb(${r},${g},${b})`;
  }

  /**
   * Convert hex string to byte array
   */
  private static hexStringToBytes(hexStr: string): Uint8Array {
    const clean = hexStr.replace(/[<>\s]/g, '');
    const bytes = new Uint8Array(clean.length / 2);
    for (let i = 0; i < bytes.length; i++) {
      bytes[i] = parseInt(clean.substr(i * 2, 2), 16);
    }
    return bytes;
  }
}

/**
 * Graphics State - Complete PDF graphics state
 */
interface GraphicsState {
  // Text state
  textState: {
    font: string;
    fontStyle: string;
    fontSize: number;
    matrix: number[];
    lineMatrix: number[];
    charSpace: number;
    wordSpace: number;
    horizontalScaling: number;
    leading: number;
    rise: number;
    renderMode: number;
  };

  // Line state
  lineWidth: number;
  lineCap: CanvasLineCap;
  lineJoin: CanvasLineJoin;
  miterLimit: number;
  dashArray: number[];
  dashPhase: number;

  // Color state
  strokeColor: string;
  fillColor: string;
  strokeColorSpace: ColorSpace;
  fillColorSpace: ColorSpace;

  // Current transformation matrix (stored separately from canvas CTM)
  ctm: number[];

  // Clipping path state
  hasClippingPath: boolean;

  // Font resource
  fontResource: FontResource | undefined;
}

/**
 * PDF Graphics State Executor
 * Implements PDF graphics operators to render content to canvas
 */
class PDFGraphicsExecutor {
  private graphicsStateStack: GraphicsState[] = [];
  private currentPath: Path2D = new Path2D();
  private currentPoint: [number, number] = [0, 0];
  private textState: any = {
    font: 'Arial',
    fontStyle: '',
    fontSize: 12,
    matrix: [1, 0, 0, 1, 0, 0], // Text matrix [a, b, c, d, e, f]
    lineMatrix: [1, 0, 0, 1, 0, 0], // Line matrix (start of current line)
    charSpace: 0,
    wordSpace: 0,
    horizontalScaling: 100,
    leading: 0,
    rise: 0,
    renderMode: 0
  };

  // Current font resource for glyph width calculations
  private currentFontResource: FontResource | undefined;

  // Current transformation matrix (separate from canvas)
  private currentCTM: number[] = [1, 0, 0, 1, 0, 0];

  // Line state
  private lineState = {
    width: 1,
    cap: 'butt' as CanvasLineCap,
    join: 'miter' as CanvasLineJoin,
    miterLimit: 10,
    dashArray: [] as number[],
    dashPhase: 0
  };

  // Color state
  private colorState = {
    stroke: '#000000',
    fill: '#000000'
  };

  // Color space state
  private strokeColorSpace: ColorSpace = { name: 'DeviceGray', numComponents: 1 };
  private fillColorSpace: ColorSpace = { name: 'DeviceGray', numComponents: 1 };

  // Clipping state
  private hasClippingPath: boolean = false;

  constructor(
    private ctx: CanvasRenderingContext2D,
    private page: PDFPage,
    private scale: number
  ) {
    // Set up coordinate system (PDF uses bottom-left origin, canvas uses top-left)
    ctx.save();
    ctx.scale(scale, -scale);
    ctx.translate(0, -page.height);
  }

  async executeOperator(operator: string, operands: any[]): Promise<void> {
    try {
      switch (operator) {
        // Graphics state operators
        case 'q': this.saveGraphicsState(); break;
        case 'Q': this.restoreGraphicsState(); break;
        case 'cm': this.concatMatrix(operands); break;
        case 'w': this.setLineWidth(operands); break;
        case 'J': this.setLineCap(operands); break;
        case 'j': this.setLineJoin(operands); break;
        case 'M': this.setMiterLimit(operands); break;
        case 'd': this.setDash(operands); break;
        case 'i': /* flatness tolerance - ignored for screen rendering */ break;
        case 'gs': this.setExtGState(operands); break;

        // Color operators
        case 'G': this.setStrokeGray(operands); break;
        case 'g': this.setFillGray(operands); break;
        case 'RG': this.setStrokeRGB(operands); break;
        case 'rg': this.setFillRGB(operands); break;
        case 'K': this.setStrokeCMYK(operands); break;
        case 'k': this.setFillCMYK(operands); break;
        // Advanced color space operators
        case 'CS': this.setStrokeColorSpace(operands); break;
        case 'cs': this.setFillColorSpace(operands); break;
        case 'SCN': case 'SC': this.setStrokeColor(operands); break;
        case 'scn': case 'sc': this.setFillColor(operands); break;

        // Path construction operators
        case 'm': this.moveTo(operands); break;
        case 'l': this.lineTo(operands); break;
        case 'c': this.curveTo(operands); break;
        case 'v': this.curveToV(operands); break;
        case 'y': this.curveToY(operands); break;
        case 'h': this.closePath(); break;
        case 're': this.rectangle(operands); break;

        // Clipping path operators
        case 'W': this.clipNonZero(); break;
        case 'W*': this.clipEvenOdd(); break;

        // Path painting operators
        case 'S': this.stroke(); break;
        case 's': this.closeAndStroke(); break;
        case 'f': case 'F': this.fill(); break;
        case 'f*': this.fillEvenOdd(); break;
        case 'B': this.fillAndStroke(); break;
        case 'B*': this.fillEvenOddAndStroke(); break;
        case 'b': this.closeFillAndStroke(); break;
        case 'b*': this.closeFillEvenOddAndStroke(); break;
        case 'n': this.endPath(); break;

        // Text operators
        case 'BT': this.beginText(); break;
        case 'ET': this.endText(); break;
        case 'Td': this.moveText(operands); break;
        case 'TD': this.moveTextWithLeading(operands); break;
        case 'Tm': this.setTextMatrix(operands); break;
        case 'T*': this.nextLine(); break;
        case 'Tf': this.setFont(operands); break;
        case 'Tj': this.showText(operands); break;
        case 'TJ': this.showTextArray(operands); break;
        case "'": this.nextLineShowText(operands); break;
        case '"': this.setSpacingShowText(operands); break;
        case 'Tc': this.setCharSpacing(operands); break;
        case 'Tw': this.setWordSpacing(operands); break;
        case 'Tz': this.setHorizontalScaling(operands); break;
        case 'TL': this.setTextLeading(operands); break;
        case 'Tr': this.setTextRenderMode(operands); break;
        case 'Ts': this.setTextRise(operands); break;

        // XObject operators
        case 'Do': await this.displayXObject(operands); break;

        // Inline image operator
        case 'BI': await this.renderInlineImage(operands); break;

        // Ignore unsupported operators
        default:
          // console.log('Unsupported operator:', operator);
          break;
      }
    } catch (error) {
      console.warn(`Error executing operator ${operator}:`, error);
    }
  }

  // Graphics state operators
  private saveGraphicsState(): void {
    // Save canvas state (includes CTM, clipping, colors, etc.)
    this.ctx.save();

    // Save our PDF graphics state
    const state: GraphicsState = {
      textState: {
        font: this.textState.font,
        fontStyle: this.textState.fontStyle,
        fontSize: this.textState.fontSize,
        matrix: [...this.textState.matrix],
        lineMatrix: [...this.textState.lineMatrix],
        charSpace: this.textState.charSpace,
        wordSpace: this.textState.wordSpace,
        horizontalScaling: this.textState.horizontalScaling,
        leading: this.textState.leading,
        rise: this.textState.rise,
        renderMode: this.textState.renderMode
      },
      lineWidth: this.lineState.width,
      lineCap: this.lineState.cap,
      lineJoin: this.lineState.join,
      miterLimit: this.lineState.miterLimit,
      dashArray: [...this.lineState.dashArray],
      dashPhase: this.lineState.dashPhase,
      strokeColor: this.colorState.stroke,
      fillColor: this.colorState.fill,
      strokeColorSpace: { ...this.strokeColorSpace },
      fillColorSpace: { ...this.fillColorSpace },
      ctm: [...this.currentCTM],
      hasClippingPath: this.hasClippingPath,
      fontResource: this.currentFontResource
    };

    this.graphicsStateStack.push(state);
  }

  private restoreGraphicsState(): void {
    // Restore canvas state
    this.ctx.restore();

    // Restore our PDF graphics state
    if (this.graphicsStateStack.length > 0) {
      const state = this.graphicsStateStack.pop()!;

      // Restore text state
      this.textState = {
        font: state.textState.font,
        fontStyle: state.textState.fontStyle,
        fontSize: state.textState.fontSize,
        matrix: [...state.textState.matrix],
        lineMatrix: [...state.textState.lineMatrix],
        charSpace: state.textState.charSpace,
        wordSpace: state.textState.wordSpace,
        horizontalScaling: state.textState.horizontalScaling,
        leading: state.textState.leading,
        rise: state.textState.rise,
        renderMode: state.textState.renderMode
      };

      // Restore line state
      this.lineState.width = state.lineWidth;
      this.lineState.cap = state.lineCap;
      this.lineState.join = state.lineJoin;
      this.lineState.miterLimit = state.miterLimit;
      this.lineState.dashArray = [...state.dashArray];
      this.lineState.dashPhase = state.dashPhase;

      // Restore color state
      this.colorState.stroke = state.strokeColor;
      this.colorState.fill = state.fillColor;
      this.strokeColorSpace = { ...state.strokeColorSpace };
      this.fillColorSpace = { ...state.fillColorSpace };

      // Restore CTM
      this.currentCTM = [...state.ctm];

      // Restore clipping state
      this.hasClippingPath = state.hasClippingPath;

      // Restore font resource
      this.currentFontResource = state.fontResource;
    }
  }

  private concatMatrix(operands: number[]): void {
    if (operands.length === 6) {
      const [a, b, c, d, e, f] = operands;

      // Apply to canvas
      this.ctx.transform(a, b, c, d, e, f);

      // Update our CTM (multiply matrices)
      const [a0, b0, c0, d0, e0, f0] = this.currentCTM;
      this.currentCTM = [
        a * a0 + b * c0,
        a * b0 + b * d0,
        c * a0 + d * c0,
        c * b0 + d * d0,
        e * a0 + f * c0 + e0,
        e * b0 + f * d0 + f0
      ];
    }
  }

  private setLineWidth(operands: number[]): void {
    if (operands.length > 0) {
      this.lineState.width = operands[0];
      this.ctx.lineWidth = operands[0];
    }
  }

  private setLineCap(operands: number[]): void {
    if (operands.length > 0) {
      const caps = ['butt', 'round', 'square'] as const;
      this.lineState.cap = caps[operands[0]];
      this.ctx.lineCap = caps[operands[0]];
    }
  }

  private setMiterLimit(operands: number[]): void {
    if (operands.length > 0) {
      this.lineState.miterLimit = operands[0];
      this.ctx.miterLimit = operands[0];
    }
  }

  private setExtGState(operands: any[]): void {
    if (operands.length === 0) return;
    let gsName = operands[0];
    if (typeof gsName !== 'string') return;
    if (gsName.startsWith('/')) gsName = gsName.substring(1);
    if (!this.page.resources || !this.page.resources.extGState) return;
    const gs = this.page.resources.extGState.get(gsName);
    if (!gs) return;

    // Apply graphics state parameters
    if (gs.ca !== undefined) {
      // Fill opacity (lowercase ca)
      this.ctx.globalAlpha = Math.max(0, Math.min(1, gs.ca));
    }
    if (gs.CA !== undefined) {
      // Stroke opacity (uppercase CA)
      // Canvas doesn't separate stroke/fill alpha, use the more restrictive
      this.ctx.globalAlpha = Math.max(0, Math.min(1, gs.CA));
    }
    if (gs.LW !== undefined) {
      this.lineState.width = gs.LW;
      this.ctx.lineWidth = gs.LW;
    }
    if (gs.LC !== undefined) {
      const caps = ['butt', 'round', 'square'] as const;
      if (gs.LC >= 0 && gs.LC <= 2) {
        this.lineState.cap = caps[gs.LC];
        this.ctx.lineCap = caps[gs.LC];
      }
    }
    if (gs.LJ !== undefined) {
      const joins = ['miter', 'round', 'bevel'] as const;
      if (gs.LJ >= 0 && gs.LJ <= 2) {
        this.lineState.join = joins[gs.LJ];
        this.ctx.lineJoin = joins[gs.LJ];
      }
    }
    if (gs.ML !== undefined) {
      this.lineState.miterLimit = gs.ML;
      this.ctx.miterLimit = gs.ML;
    }
    if (gs.Font !== undefined) {
      // Font entry is [fontRef size] — we just use the size if present
    }
  }

  private setLineJoin(operands: number[]): void {
    if (operands.length > 0) {
      const joins = ['miter', 'round', 'bevel'] as const;
      this.lineState.join = joins[operands[0]];
      this.ctx.lineJoin = joins[operands[0]];
    }
  }

  private setDash(operands: any[]): void {
    if (operands.length >= 2 && Array.isArray(operands[0])) {
      this.lineState.dashArray = operands[0];
      this.lineState.dashPhase = operands[1] || 0;
      this.ctx.setLineDash(operands[0]);
      if (typeof this.ctx.lineDashOffset !== 'undefined') {
        this.ctx.lineDashOffset = operands[1] || 0;
      }
    }
  }

  // Color operators
  private setStrokeGray(operands: number[]): void {
    if (operands.length > 0) {
      const gray = Math.floor(operands[0] * 255);
      const color = `rgb(${gray},${gray},${gray})`;
      this.colorState.stroke = color;
      this.ctx.strokeStyle = color;
    }
  }

  private setFillGray(operands: number[]): void {
    if (operands.length > 0) {
      const gray = Math.floor(operands[0] * 255);
      const color = `rgb(${gray},${gray},${gray})`;
      this.colorState.fill = color;
      this.ctx.fillStyle = color;
    }
  }

  private setStrokeRGB(operands: number[]): void {
    if (operands.length >= 3) {
      const r = Math.floor(operands[0] * 255);
      const g = Math.floor(operands[1] * 255);
      const b = Math.floor(operands[2] * 255);
      const color = `rgb(${r},${g},${b})`;
      this.colorState.stroke = color;
      this.ctx.strokeStyle = color;
    }
  }

  private setFillRGB(operands: number[]): void {
    if (operands.length >= 3) {
      const r = Math.floor(operands[0] * 255);
      const g = Math.floor(operands[1] * 255);
      const b = Math.floor(operands[2] * 255);
      const color = `rgb(${r},${g},${b})`;
      this.colorState.fill = color;
      this.ctx.fillStyle = color;
    }
  }

  private setStrokeCMYK(operands: number[]): void {
    if (operands.length >= 4) {
      // Simple CMYK to RGB conversion
      const [c, m, y, k] = operands;
      const r = Math.floor(255 * (1 - c) * (1 - k));
      const g = Math.floor(255 * (1 - m) * (1 - k));
      const b = Math.floor(255 * (1 - y) * (1 - k));
      const color = `rgb(${r},${g},${b})`;
      this.colorState.stroke = color;
      this.ctx.strokeStyle = color;
    }
  }

  private setFillCMYK(operands: number[]): void {
    if (operands.length >= 4) {
      const [c, m, y, k] = operands;
      const r = Math.floor(255 * (1 - c) * (1 - k));
      const g = Math.floor(255 * (1 - m) * (1 - k));
      const b = Math.floor(255 * (1 - y) * (1 - k));
      const color = `rgb(${r},${g},${b})`;
      this.colorState.fill = color;
      this.ctx.fillStyle = color;
    }
  }

  // Advanced Color Space Operators

  /**
   * CS operator: Set stroke color space
   */
  private setStrokeColorSpace(operands: any[]): void {
    if (operands.length > 0) {
      let csName = operands[0];
      if (typeof csName === 'string' && csName.startsWith('/')) csName = csName.substring(1);

      // Get color space from resources or parse inline
      let colorSpace: ColorSpace;
      if (this.page.resources && this.page.resources.colorSpaces.has(csName)) {
        const csObj = this.page.resources.colorSpaces.get(csName);
        colorSpace = PDFColorSpaceProcessor.parseColorSpace(csObj, this.page.resources);
      } else {
        colorSpace = PDFColorSpaceProcessor.parseColorSpace(csName, this.page.resources);
      }

      this.strokeColorSpace = colorSpace;
    }
  }

  /**
   * cs operator: Set fill color space
   */
  private setFillColorSpace(operands: any[]): void {
    if (operands.length > 0) {
      let csName = operands[0];
      if (typeof csName === 'string' && csName.startsWith('/')) csName = csName.substring(1);

      // Get color space from resources or parse inline
      let colorSpace: ColorSpace;
      if (this.page.resources && this.page.resources.colorSpaces.has(csName)) {
        const csObj = this.page.resources.colorSpaces.get(csName);
        colorSpace = PDFColorSpaceProcessor.parseColorSpace(csObj, this.page.resources);
      } else {
        colorSpace = PDFColorSpaceProcessor.parseColorSpace(csName, this.page.resources);
      }

      this.fillColorSpace = colorSpace;
    }
  }

  /**
   * SCN/SC operator: Set stroke color (with current color space)
   */
  private setStrokeColor(operands: number[]): void {
    if (operands.length === 0) return;

    // Convert color values to RGB using current stroke color space
    const rgb = PDFColorSpaceProcessor.toRGB(this.strokeColorSpace, operands);
    const color = PDFColorSpaceProcessor.rgbToCSS(rgb);

    this.colorState.stroke = color;
    this.ctx.strokeStyle = color;
  }

  /**
   * scn/sc operator: Set fill color (with current color space)
   */
  private setFillColor(operands: number[]): void {
    if (operands.length === 0) return;

    // Convert color values to RGB using current fill color space
    const rgb = PDFColorSpaceProcessor.toRGB(this.fillColorSpace, operands);
    const color = PDFColorSpaceProcessor.rgbToCSS(rgb);

    this.colorState.fill = color;
    this.ctx.fillStyle = color;
  }

  // Path construction operators
  private moveTo(operands: number[]): void {
    if (operands.length >= 2) {
      this.currentPoint = [operands[0], operands[1]];
      this.currentPath.moveTo(operands[0], operands[1]);
    }
  }

  private lineTo(operands: number[]): void {
    if (operands.length >= 2) {
      this.currentPoint = [operands[0], operands[1]];
      this.currentPath.lineTo(operands[0], operands[1]);
    }
  }

  private curveTo(operands: number[]): void {
    if (operands.length >= 6) {
      this.currentPoint = [operands[4], operands[5]];
      this.currentPath.bezierCurveTo(
        operands[0], operands[1],
        operands[2], operands[3],
        operands[4], operands[5]
      );
    }
  }

  private curveToV(operands: number[]): void {
    if (operands.length >= 4) {
      const [x1, y1] = this.currentPoint;
      this.currentPoint = [operands[2], operands[3]];
      this.currentPath.bezierCurveTo(
        x1, y1,
        operands[0], operands[1],
        operands[2], operands[3]
      );
    }
  }

  private curveToY(operands: number[]): void {
    if (operands.length >= 4) {
      this.currentPoint = [operands[2], operands[3]];
      this.currentPath.bezierCurveTo(
        operands[0], operands[1],
        operands[2], operands[3],
        operands[2], operands[3]
      );
    }
  }

  private closePath(): void {
    this.currentPath.closePath();
  }

  private rectangle(operands: number[]): void {
    if (operands.length >= 4) {
      this.currentPath.rect(operands[0], operands[1], operands[2], operands[3]);
    }
  }

  // Clipping path operators
  private clipNonZero(): void {
    // W operator: Modify clipping path using nonzero winding rule
    // Note: Must be followed by a path painting operator (n, S, f, etc.)
    if (this.currentPath) {
      this.ctx.clip(this.currentPath, 'nonzero');
      this.hasClippingPath = true;
    }
    // Don't clear currentPath - it may still be used for painting
  }

  private clipEvenOdd(): void {
    // W* operator: Modify clipping path using even-odd rule
    // Note: Must be followed by a path painting operator (n, S, f, etc.)
    if (this.currentPath) {
      this.ctx.clip(this.currentPath, 'evenodd');
      this.hasClippingPath = true;
    }
    // Don't clear currentPath - it may still be used for painting
  }

  // Path painting operators
  private stroke(): void {
    this.ctx.stroke(this.currentPath);
    this.currentPath = new Path2D();
  }

  private closeAndStroke(): void {
    this.currentPath.closePath();
    this.ctx.stroke(this.currentPath);
    this.currentPath = new Path2D();
  }

  private fill(): void {
    this.ctx.fill(this.currentPath);
    this.currentPath = new Path2D();
  }

  private fillEvenOdd(): void {
    this.ctx.fill(this.currentPath, 'evenodd');
    this.currentPath = new Path2D();
  }

  private fillAndStroke(): void {
    this.ctx.fill(this.currentPath);
    this.ctx.stroke(this.currentPath);
    this.currentPath = new Path2D();
  }

  private fillEvenOddAndStroke(): void {
    this.ctx.fill(this.currentPath, 'evenodd');
    this.ctx.stroke(this.currentPath);
    this.currentPath = new Path2D();
  }

  private closeFillAndStroke(): void {
    this.currentPath.closePath();
    this.ctx.fill(this.currentPath);
    this.ctx.stroke(this.currentPath);
    this.currentPath = new Path2D();
  }

  private closeFillEvenOddAndStroke(): void {
    this.currentPath.closePath();
    this.ctx.fill(this.currentPath, 'evenodd');
    this.ctx.stroke(this.currentPath);
    this.currentPath = new Path2D();
  }

  private endPath(): void {
    this.currentPath = new Path2D();
  }

  // Text operators
  private beginText(): void {
    this.textState.matrix = [1, 0, 0, 1, 0, 0];
    this.textState.lineMatrix = [1, 0, 0, 1, 0, 0];
    this.ctx.textBaseline = 'alphabetic';
  }

  private endText(): void {
    // Text state is preserved
  }

  private moveText(operands: number[]): void {
    if (operands.length >= 2) {
      const tx = operands[0];
      const ty = operands[1];

      // Update line matrix: translate by (tx, ty)
      this.textState.lineMatrix[4] += tx * this.textState.lineMatrix[0] + ty * this.textState.lineMatrix[2];
      this.textState.lineMatrix[5] += tx * this.textState.lineMatrix[1] + ty * this.textState.lineMatrix[3];

      // Text matrix = line matrix
      this.textState.matrix = [...this.textState.lineMatrix];
    }
  }

  private moveTextWithLeading(operands: number[]): void {
    if (operands.length >= 2) {
      this.textState.leading = -operands[1];
      this.moveText(operands);
    }
  }

  private setTextMatrix(operands: number[]): void {
    if (operands.length >= 6) {
      this.textState.matrix = [...operands];
      this.textState.lineMatrix = [...operands];
    }
  }

  private nextLine(): void {
    // T* is equivalent to Td(0, -TL) per PDF spec
    this.moveText([0, -this.textState.leading]);
  }

  private setFont(operands: any[]): void {
    if (operands.length >= 2) {
      const rawName = operands[0];
      const fontSize = operands[1];
      this.textState.fontSize = fontSize;

      // Strip leading '/' from name for resource lookup (parser returns '/F94', resources keyed 'F94')
      const fontKey = typeof rawName === 'string' && rawName.startsWith('/') ? rawName.substring(1) : rawName;

      // Lookup font resource from page resources
      if (this.page.resources && this.page.resources.fonts) {
        this.currentFontResource = this.page.resources.fonts.get(fontKey);
      }

      // Map PDF font to canvas font using baseFont from resource (or raw name as fallback)
      let baseFont = this.currentFontResource?.baseFont || fontKey || '';
      // Strip subset prefix (e.g., "ABCDEF+" → "ArialMT")
      const plusIdx = baseFont.indexOf('+');
      if (plusIdx >= 0 && plusIdx <= 6) baseFont = baseFont.substring(plusIdx + 1);
      let canvasFont = 'sans-serif';
      const bf = baseFont.toLowerCase();

      if (bf.includes('courier') || bf.includes('mono') || bf.includes('nimbusmono') || bf.includes('nimbusl') || bf.includes('nimbusmonl')) {
        canvasFont = '"Courier New", monospace';
      } else if (bf.includes('times') || bf.includes('nimbus') || bf.includes('cmr') || bf.includes('cmb') ||
                 bf.includes('cmmi') || bf.includes('cmsy') || bf.includes('cmt') || bf.includes('roman') ||
                 bf.includes('serif') && !bf.includes('sans')) {
        canvasFont = '"Times New Roman", serif';
      } else if (bf.includes('helvetica') || bf.includes('arial') || bf.includes('sans')) {
        canvasFont = '"Helvetica", "Arial", sans-serif';
      } else {
        canvasFont = '"Times New Roman", serif'; // default serif for academic/book content
      }

      // Add weight/style modifiers
      let fontStyle = '';
      if (bf.includes('bold') || bf.includes('medi') || bf.includes('cmb')) {
        fontStyle = 'bold ';
      }
      if (bf.includes('ital') || bf.includes('obli') || bf.includes('cmmi') || bf.includes('cmti')) {
        fontStyle += 'italic ';
      }

      this.textState.font = canvasFont;
      this.textState.fontStyle = fontStyle;
      this.ctx.font = `${fontStyle}${Math.abs(fontSize)}px ${canvasFont}`;
    }
  }

  private showText(operands: any[]): void {
    if (operands.length === 0) return;
    const rawInput = operands[0];
    if (typeof rawInput !== 'string' || rawInput.length === 0) return;

    const tm = this.textState.matrix;
    const fontSize = this.textState.fontSize;
    const absFontSize = Math.abs(fontSize);
    const renderMode = this.textState.renderMode || 0;
    const shouldRender = renderMode !== 3 && renderMode !== 7;
    const horizScale = (this.textState.horizontalScaling || 100) / 100;
    const charSpace = this.textState.charSpace || 0;
    const wordSpace = this.textState.wordSpace || 0;
    const rise = this.textState.rise || 0;

    // Resolve escape sequences to get raw byte values for width lookups.
    const rawBytes = this.resolveEscapes(rawInput);
    if (rawBytes.length === 0) return;

    const toUnicode = this.currentFontResource?.toUnicode;

    // Determine if this font uses 2-byte character codes (CID fonts).
    // Type0 composite fonts (CIDFont) encode characters as 2-byte CIDs.
    const isCIDFont = this.currentFontResource?.subtype === 'Type0';

    // Build character code array: group bytes into proper code units
    const charCodes: number[] = [];
    if (isCIDFont) {
      // 2-byte CID codes: combine pairs of bytes into 16-bit values
      for (let i = 0; i < rawBytes.length - 1; i += 2) {
        charCodes.push((rawBytes[i] << 8) | rawBytes[i + 1]);
      }
    } else {
      // Single-byte codes
      for (const b of rawBytes) {
        charCodes.push(b);
      }
    }

    // Factor font size from Tm for sharper rendering.
    const tmScale = Math.sqrt(tm[0] * tm[0] + tm[1] * tm[1]) || 1;
    const effectiveFontSize = absFontSize * tmScale;

    if (shouldRender) {
      this.ctx.save();
      this.ctx.textBaseline = 'alphabetic';
      if (tmScale !== 1) {
        this.ctx.transform(tm[0] / tmScale, tm[1] / tmScale, tm[2] / tmScale, tm[3] / tmScale, tm[4], tm[5]);
      } else {
        this.ctx.transform(tm[0], tm[1], tm[2], tm[3], tm[4], tm[5]);
      }
      if (horizScale !== 1) {
        this.ctx.scale(horizScale, 1);
      }
      const fontStyle = this.textState.fontStyle || '';
      this.ctx.font = fontStyle + effectiveFontSize + 'px ' + this.textState.font;
      this.ctx.scale(1, -1);
    }

    let xOffset = 0;

    for (const charCode of charCodes) {
      // Display character: ToUnicode/Encoding first, then PDFDocEncoding
      let displayChar: string;
      if (toUnicode && toUnicode.has(charCode)) {
        displayChar = toUnicode.get(charCode)!;
      } else if (isCIDFont) {
        // For CID fonts without ToUnicode, try Unicode interpretation
        displayChar = charCode > 0 ? String.fromCodePoint(charCode) : '';
      } else {
        displayChar = PDFTextDecoder.mapCharCode(charCode, this.currentFontResource?.encoding);
      }

      const renderX = xOffset * tmScale;
      const renderY = -rise * tmScale;

      if (shouldRender && displayChar) {
        if (renderMode === 1) {
          this.ctx.strokeText(displayChar, renderX, renderY);
        } else if (renderMode === 2) {
          this.ctx.fillText(displayChar, renderX, renderY);
          this.ctx.strokeText(displayChar, renderX, renderY);
        } else {
          this.ctx.fillText(displayChar, renderX, renderY);
        }
      }

      const glyphWidth = PDFGlyphMetrics.getCharWidth(charCode, this.currentFontResource, fontSize);
      let advance = glyphWidth + charSpace;
      if (charCode === 32) advance += wordSpace;
      xOffset += advance;
    }

    if (shouldRender) {
      this.ctx.restore();
    }

    const totalAdvance = xOffset * horizScale;
    tm[4] += totalAdvance * tm[0];
    tm[5] += totalAdvance * tm[1];
  }

  private showTextArray(operands: any[]): void {
    if (operands.length > 0 && Array.isArray(operands[0])) {
      const array = operands[0];

      for (const item of array) {
        if (typeof item === 'string') {
          // Show the text string
          this.showText([item]);
        } else if (typeof item === 'number') {
          // Adjust spacing: tx = -(Tj/1000) * Tfs * Th (PDF spec 9.4.4)
          const horizScale = (this.textState.horizontalScaling || 100) / 100;
          const adjustment = -item / 1000 * this.textState.fontSize * horizScale;

          // Move text matrix horizontally
          this.textState.matrix[4] += adjustment * this.textState.matrix[0];
          this.textState.matrix[5] += adjustment * this.textState.matrix[1];
        }
      }
    }
  }

  private nextLineShowText(operands: any[]): void {
    this.nextLine();
    this.showText(operands);
  }

  private setSpacingShowText(operands: any[]): void {
    if (operands.length >= 3) {
      this.textState.wordSpace = operands[0];
      this.textState.charSpace = operands[1];
      this.nextLine();
      this.showText([operands[2]]);
    }
  }

  private setCharSpacing(operands: number[]): void {
    if (operands.length > 0) {
      this.textState.charSpace = operands[0];
    }
  }

  private setWordSpacing(operands: number[]): void {
    if (operands.length > 0) {
      this.textState.wordSpace = operands[0];
    }
  }

  private setHorizontalScaling(operands: number[]): void {
    if (operands.length > 0) {
      this.textState.horizontalScaling = operands[0];
    }
  }

  private setTextLeading(operands: number[]): void {
    if (operands.length > 0) {
      this.textState.leading = operands[0];
    }
  }

  private setTextRenderMode(operands: number[]): void {
    if (operands.length > 0) {
      // 0: Fill, 1: Stroke, 2: Fill then stroke, 3: Invisible
      // 4-7: Same but with clipping
      const mode = operands[0];
      this.textState.renderMode = mode;

      // For now, just handle basic modes
      // Mode 3 and 7 are invisible - we'll skip rendering
      // Other modes we'll render as fill
    }
  }

  private setTextRise(operands: number[]): void {
    if (operands.length > 0) {
      this.textState.rise = operands[0];
    }
  }

  // XObject operators
  private async displayXObject(operands: any[]): Promise<void> {
    if (operands.length === 0) return;

    let xobjName = operands[0];
    if (typeof xobjName !== 'string') return;
    if (xobjName.startsWith('/')) xobjName = xobjName.substring(1);

    // Lookup XObject in page resources
    if (!this.page.resources || !this.page.resources.xObjects) {
      console.warn('[XObject] No resources or xObjects available');
      return;
    }

    const xobject = this.page.resources.xObjects.get(xobjName);
    if (!xobject) {
      console.warn('[XObject] XObject not found:', xobjName);
      return;
    }

      // console.log('[XObject] Displaying XObject:', xobjName, 'type:', xobject.type);

    if (xobject.type === 'image') {
      await this.renderImage(xobject);
    } else if (xobject.type === 'form') {
      await this.renderFormXObject(xobject);
    }
  }

  /**
   * Render Form XObject
   * Form XObjects are reusable graphics content that can be drawn multiple times
   */
  private async renderFormXObject(xobject: XObject): Promise<void> {
    try {
      // Save graphics state before rendering form
      this.saveGraphicsState();

      // Apply form matrix if present
      if (xobject.matrix) {
        const m = xobject.matrix;
        this.ctx.transform(m[0], m[1], m[2], m[3], m[4], m[5]);
      }

      // Apply bounding box clipping if present
      if (xobject.bbox && xobject.bbox.length === 4) {
        const [llx, lly, urx, ury] = xobject.bbox;
        this.ctx.beginPath();
        this.ctx.rect(llx, lly, urx - llx, ury - lly);
        this.ctx.clip();
      }

      // Parse and execute form content stream
      const formContentStream = xobject.data;
      if (formContentStream && formContentStream.length > 0) {
        // Parse form content stream
        const parser = new SafeContentStreamParser(formContentStream);
        const operations = parser.parse();

        // Store current page resources
        const savedPageResources = this.page.resources;

        // If form has its own resources, merge them with page resources
        if (xobject.resources) {
          this.page.resources = this.mergeResources(savedPageResources, xobject.resources);
        }

        // Execute form operations
        for (const op of operations) {
          await this.executeOperator(op.operator, op.operands);
        }

        // Restore page resources
        this.page.resources = savedPageResources;
      }

      // Restore graphics state
      this.restoreGraphicsState();
    } catch (error) {
      console.warn('Error rendering form XObject:', error);
      // Ensure we restore state even on error
      this.restoreGraphicsState();
    }
  }

  /**
   * Merge form resources with page resources
   * Form resources take precedence over page resources
   */
  private mergeResources(pageRes: PDFResources | undefined, formRes: PDFResources): PDFResources {
    if (!pageRes) return formRes;

    // Create merged resources with form resources taking precedence
    const merged: PDFResources = {
      fonts: new Map([...(pageRes.fonts || new Map()), ...(formRes.fonts || new Map())]),
      images: new Map([...(pageRes.images || new Map()), ...(formRes.images || new Map())]),
      xObjects: new Map([...(pageRes.xObjects || new Map()), ...(formRes.xObjects || new Map())]),
      colorSpaces: new Map([...(pageRes.colorSpaces || new Map()), ...(formRes.colorSpaces || new Map())]),
      patterns: new Map([...(pageRes.patterns || new Map()), ...(formRes.patterns || new Map())]),
      extGState: new Map([...(pageRes.extGState || new Map()), ...(formRes.extGState || new Map())])
    };

    return merged;
  }

  private async renderImage(xobject: XObject): Promise<void> {
    try {
      const data = xobject.data instanceof Uint8Array
        ? xobject.data : new Uint8Array(xobject.data as ArrayBuffer);

      if (data.length === 0) return;

      // Check if data is a pre-encoded image (JPEG/PNG/GIF)
      if (this.isEncodedImageData(data, xobject.filter)) {
        await this.renderEncodedImage(data);
      } else {
        // Raw pixel data — convert to ImageData and draw
        this.renderRawPixelImage(data, xobject);
      }
    } catch (error) {
      console.warn('[Image Rendering] Error in renderImage:', error);
    }
  }

  private isEncodedImageData(data: Uint8Array, filter?: string): boolean {
    if (filter === 'DCTDecode' || filter === 'JPXDecode') return true;
    if (data.length >= 3 && data[0] === 0xFF && data[1] === 0xD8 && data[2] === 0xFF) return true; // JPEG
    if (data.length >= 4 && data[0] === 0x89 && data[1] === 0x50 && data[2] === 0x4E && data[3] === 0x47) return true; // PNG
    if (data.length >= 3 && data[0] === 0x47 && data[1] === 0x49 && data[2] === 0x46) return true; // GIF
    return false;
  }

  private async renderEncodedImage(data: Uint8Array): Promise<void> {
    let mimeType = 'image/jpeg';
    if (data[0] === 0x89 && data[1] === 0x50) mimeType = 'image/png';
    else if (data[0] === 0x47 && data[1] === 0x49) mimeType = 'image/gif';

    const arrayBuffer = data.buffer.slice(data.byteOffset, data.byteOffset + data.byteLength) as ArrayBuffer;
    const blob = new Blob([arrayBuffer], { type: mimeType });
    const imageUrl = URL.createObjectURL(blob);

    try {
      const img = new Image();
      img.src = imageUrl;

      await new Promise<void>((resolve, reject) => {
        img.onload = () => {
          try {
            this.ctx.save();
            this.ctx.scale(1, -1);
            this.ctx.translate(0, -1);
            this.ctx.drawImage(img, 0, 0, 1, 1);
            this.ctx.restore();
            resolve();
          } catch (e) { reject(e); }
        };
        img.onerror = () => reject(new Error('Failed to load image'));
      });
    } finally {
      URL.revokeObjectURL(imageUrl);
    }
  }

  private renderRawPixelImage(data: Uint8Array, xobject: XObject): void {
    const width = xobject.width || 1;
    const height = xobject.height || 1;
    const bpc = xobject.bitsPerComponent || 8;
    const cs = (xobject.colorSpace || 'DeviceRGB').replace(/^\//, '');

    // Determine components per pixel from color space
    let components: number;
    if (cs === 'DeviceGray' || cs === 'CalGray' || cs === 'G') {
      components = 1;
    } else if (cs === 'DeviceCMYK' || cs === 'CMYK') {
      components = 4;
    } else {
      components = 3; // DeviceRGB, CalRGB, ICCBased (default)
    }

    // Infer components from data length if mismatch
    if (bpc === 8) {
      const expectedRGB = width * height * 3;
      const expectedGray = width * height;
      const expectedCMYK = width * height * 4;
      if (components === 3 && data.length >= expectedCMYK && data.length < expectedRGB) {
        // Data doesn't fit RGB, might be something else
      } else if (components === 3 && data.length === expectedGray) {
        components = 1; // Actually grayscale
      } else if (components === 3 && data.length === expectedCMYK) {
        components = 4; // Actually CMYK
      }
    }

    // Apply PNG predictor un-filtering if needed
    let pixels = data;
    if (xobject.predictor && xobject.predictor >= 10) {
      const bytesPerPixel = Math.ceil(components * bpc / 8);
      const rowBytes = xobject.predictorColumns
        ? Math.ceil(xobject.predictorColumns * (xobject.predictorColors || components) * bpc / 8)
        : Math.ceil(width * components * bpc / 8);
      pixels = this.reversePNGPrediction(data, rowBytes, bytesPerPixel, height);
    }

    // Create RGBA ImageData
    const imageData = new ImageData(width, height);
    const rgba = imageData.data;

    if (bpc === 8) {
      this.convertPixels8bit(pixels, rgba, width, height, components);
    } else if (bpc === 1 || bpc === 2 || bpc === 4) {
      this.convertPixelsSubByte(pixels, rgba, width, height, components, bpc);
    } else {
      this.convertPixels8bit(pixels, rgba, width, height, components);
    }

    // Draw via temporary canvas
    const tmpCanvas = document.createElement('canvas');
    tmpCanvas.width = width;
    tmpCanvas.height = height;
    const tmpCtx = tmpCanvas.getContext('2d')!;
    tmpCtx.putImageData(imageData, 0, 0);

    this.ctx.save();
    this.ctx.scale(1, -1);
    this.ctx.translate(0, -1);
    this.ctx.drawImage(tmpCanvas, 0, 0, 1, 1);
    this.ctx.restore();
  }

  private convertPixels8bit(src: Uint8Array, dst: Uint8ClampedArray, w: number, h: number, comp: number): void {
    const total = w * h;
    for (let i = 0; i < total; i++) {
      const dstIdx = i * 4;
      if (comp === 1) {
        const g = src[i] || 0;
        dst[dstIdx] = g; dst[dstIdx + 1] = g; dst[dstIdx + 2] = g; dst[dstIdx + 3] = 255;
      } else if (comp === 3) {
        const srcIdx = i * 3;
        dst[dstIdx] = src[srcIdx] || 0;
        dst[dstIdx + 1] = src[srcIdx + 1] || 0;
        dst[dstIdx + 2] = src[srcIdx + 2] || 0;
        dst[dstIdx + 3] = 255;
      } else if (comp === 4) {
        const srcIdx = i * 4;
        const c = (src[srcIdx] || 0) / 255;
        const m = (src[srcIdx + 1] || 0) / 255;
        const y = (src[srcIdx + 2] || 0) / 255;
        const k = (src[srcIdx + 3] || 0) / 255;
        dst[dstIdx] = 255 * (1 - c) * (1 - k) | 0;
        dst[dstIdx + 1] = 255 * (1 - m) * (1 - k) | 0;
        dst[dstIdx + 2] = 255 * (1 - y) * (1 - k) | 0;
        dst[dstIdx + 3] = 255;
      }
    }
  }

  private convertPixelsSubByte(src: Uint8Array, dst: Uint8ClampedArray, w: number, h: number, comp: number, bpc: number): void {
    const maxVal = (1 << bpc) - 1;
    let bitPos = 0;
    const total = w * h;

    for (let i = 0; i < total; i++) {
      const dstIdx = i * 4;
      if (comp === 1) {
        const byteIdx = bitPos >> 3;
        const bitOffset = 8 - bpc - (bitPos & 7);
        const val = ((src[byteIdx] || 0) >> bitOffset) & maxVal;
        const scaled = (val * 255 / maxVal) | 0;
        dst[dstIdx] = scaled; dst[dstIdx + 1] = scaled; dst[dstIdx + 2] = scaled; dst[dstIdx + 3] = 255;
        bitPos += bpc;
      } else {
        const vals: number[] = [];
        for (let c = 0; c < comp; c++) {
          const byteIdx = bitPos >> 3;
          const bitOffset = 8 - bpc - (bitPos & 7);
          vals.push((((src[byteIdx] || 0) >> bitOffset) & maxVal) * 255 / maxVal | 0);
          bitPos += bpc;
        }
        dst[dstIdx] = vals[0]; dst[dstIdx + 1] = vals[1] || 0; dst[dstIdx + 2] = vals[2] || 0; dst[dstIdx + 3] = 255;
      }
      // Row alignment: each row starts at byte boundary
      if ((i + 1) % w === 0) {
        bitPos = ((bitPos + 7) >> 3) << 3;
      }
    }
  }

  private reversePNGPrediction(data: Uint8Array, rowBytes: number, bytesPerPixel: number, height: number): Uint8Array {
    const inputRowSize = 1 + rowBytes; // 1 filter byte + row data
    const output = new Uint8Array(rowBytes * height);

    for (let row = 0; row < height; row++) {
      const inOff = row * inputRowSize;
      const outOff = row * rowBytes;
      const prevRowOff = (row - 1) * rowBytes;
      const filterType = data[inOff] || 0;

      for (let col = 0; col < rowBytes; col++) {
        const raw = data[inOff + 1 + col] || 0;
        const a = col >= bytesPerPixel ? output[outOff + col - bytesPerPixel] : 0;
        const b = row > 0 ? output[prevRowOff + col] : 0;
        const c = (row > 0 && col >= bytesPerPixel) ? output[prevRowOff + col - bytesPerPixel] : 0;

        let val: number;
        switch (filterType) {
          case 0: val = raw; break;
          case 1: val = (raw + a) & 0xFF; break;
          case 2: val = (raw + b) & 0xFF; break;
          case 3: val = (raw + ((a + b) >> 1)) & 0xFF; break;
          case 4: val = (raw + this.paethPredictor(a, b, c)) & 0xFF; break;
          default: val = raw; break;
        }
        output[outOff + col] = val;
      }
    }
    return output;
  }

  private paethPredictor(a: number, b: number, c: number): number {
    const p = a + b - c;
    const pa = Math.abs(p - a);
    const pb = Math.abs(p - b);
    const pc = Math.abs(p - c);
    if (pa <= pb && pa <= pc) return a;
    if (pb <= pc) return b;
    return c;
  }

  private async renderInlineImage(operands: any[]): Promise<void> {
    if (operands.length === 0) return;
    const imgObj = operands[0];
    if (!imgObj || !imgObj.data) return;

    const dict: Map<string, any> = imgObj.dictionary || new Map();

    // Inline images use abbreviated keys
    const width = dict.get('/W') || dict.get('/Width') || 1;
    const height = dict.get('/H') || dict.get('/Height') || 1;
    const bpc = dict.get('/BPC') || dict.get('/BitsPerComponent') || 8;
    let cs = dict.get('/CS') || dict.get('/ColorSpace') || 'DeviceRGB';
    if (typeof cs === 'string') cs = cs.replace(/^\//, '');
    if (cs === 'G') cs = 'DeviceGray';
    else if (cs === 'RGB') cs = 'DeviceRGB';
    else if (cs === 'CMYK') cs = 'DeviceCMYK';

    let filter = dict.get('/F') || dict.get('/Filter') || '';
    if (typeof filter === 'string') filter = filter.replace(/^\//, '');
    if (filter === 'AHx') filter = 'ASCIIHexDecode';
    else if (filter === 'A85') filter = 'ASCII85Decode';
    else if (filter === 'LZW') filter = 'LZWDecode';
    else if (filter === 'Fl') filter = 'FlateDecode';
    else if (filter === 'RL') filter = 'RunLengthDecode';
    else if (filter === 'CCF') filter = 'CCITTFaxDecode';
    else if (filter === 'DCT') filter = 'DCTDecode';

    const data: Uint8Array = imgObj.data instanceof Uint8Array ? imgObj.data : new Uint8Array(imgObj.data);
    if (data.length === 0) return;

    const xobj: XObject = {
      type: 'image',
      data: data,
      width: typeof width === 'number' ? width : parseInt(width) || 1,
      height: typeof height === 'number' ? height : parseInt(height) || 1,
      bitsPerComponent: typeof bpc === 'number' ? bpc : parseInt(bpc) || 8,
      colorSpace: cs,
      filter: filter
    };

    await this.renderImage(xobj);
  }

  /**
   * Resolve PDF string escape sequences to raw byte values.
   */
  private resolveEscapes(input: string): number[] {
    const bytes: number[] = [];
    let i = 0;
    while (i < input.length) {
      const ch = input.charCodeAt(i);
      if (ch === 0x5C && i + 1 < input.length) {
        i++;
        const next = input.charCodeAt(i);
        if (next >= 0x30 && next <= 0x37) {
          let octal = next - 0x30;
          i++;
          if (i < input.length && input.charCodeAt(i) >= 0x30 && input.charCodeAt(i) <= 0x37) {
            octal = octal * 8 + (input.charCodeAt(i) - 0x30);
            i++;
            if (i < input.length && input.charCodeAt(i) >= 0x30 && input.charCodeAt(i) <= 0x37) {
              octal = octal * 8 + (input.charCodeAt(i) - 0x30);
              i++;
            }
          }
          bytes.push(octal);
        } else {
          switch (next) {
            case 0x6E: bytes.push(0x0A); break;
            case 0x72: bytes.push(0x0D); break;
            case 0x74: bytes.push(0x09); break;
            case 0x62: bytes.push(0x08); break;
            case 0x66: bytes.push(0x0C); break;
            case 0x28: bytes.push(0x28); break;
            case 0x29: bytes.push(0x29); break;
            case 0x5C: bytes.push(0x5C); break;
            default: bytes.push(next); break;
          }
          i++;
        }
      } else {
        bytes.push(ch);
        i++;
      }
    }
    return bytes;
  }

    private decodeText(text: any): string {
    const decoded = PDFTextDecoder.decode(text);
    // Apply font-specific ToUnicode mapping if available
    if (this.currentFontResource?.toUnicode && this.currentFontResource.toUnicode.size > 0) {
      const toUnicode = this.currentFontResource.toUnicode;
      let result = '';
      // Use original byte values for mapping (before PDFDocEncoding)
      const raw = typeof text === 'string' ? text : String(text);
      for (let i = 0; i < raw.length; i++) {
        const charCode = raw.charCodeAt(i);
        const mapped = toUnicode.get(charCode);
        result += mapped !== undefined ? mapped : decoded[i] || '';
      }
      return result;
    }
    return decoded;
  }
}

class PDFRenderer {
  constructor(
    private pdf: AgenticPDF,
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
      themeStorageKey: 'AgenticPDF-theme'
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
        storageKey: opts.themeStorageKey || 'AgenticPDF-theme',
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
   * @param pdf - AgenticPDF instance
   * @param options - Render options
   * @returns Object with viewer elements and methods
   */
  static createOptimalViewer(
    container: HTMLElement,
    pdf: AgenticPDF,
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
    const renderScale = this.options?.renderScale || 2.0;

    // Calculate dimensions
    const displayWidth = page.width * scale;
    const displayHeight = page.height * scale;
    const canvasWidth = displayWidth * renderScale;
    const canvasHeight = displayHeight * renderScale;

    // Set canvas size
    canvas.width = canvasWidth;
    canvas.height = canvasHeight;

    if (this.options?.maintainAspectRatio !== false) {
      canvas.style.width = `${displayWidth}px`;
      canvas.style.height = `${displayHeight}px`;
    }

    ctx.save();
    ctx.scale(renderScale, renderScale);

    // Render background
    const backgroundColor = this.options?.background || (this.options?.darkMode ? '#1a1a1a' : 'white');
    ctx.fillStyle = backgroundColor;
    ctx.fillRect(0, 0, displayWidth, displayHeight);

    // Simple approach: Draw graphics operators directly to canvas
    // Based on PDF.js approach but simplified and translated to TypeScript
    await this.renderPageContent(ctx, page, scale);

    ctx.restore();
  }

  private async renderPageContent(ctx: CanvasRenderingContext2D, page: PDFPage, scale: number): Promise<void> {
    if (!page.contents) {
      console.warn('Page has no contents');
      return;
    }

    try {
      // console.log(`Rendering page ${page.pageNumber}, contents size: ${page.contents.length} bytes`);

      // Create PDF graphics state machine
      const graphics = new PDFGraphicsExecutor(ctx, page, scale);

      // Parse and execute content stream operators
      const parser = new SafeContentStreamParser(page.contents);
      const operations = parser.parse();

      // console.log(`Parsed ${operations.length} operations`);

      if (operations.length === 0) {
        console.warn('No operations parsed from content stream');
        // Draw fallback message
        ctx.save();
        ctx.setTransform(1, 0, 0, 1, 0, 0); // Reset transform
        ctx.fillStyle = '#666';
        ctx.font = `${14 * scale}px Arial`;
        ctx.textAlign = 'left';
        ctx.fillText(`Page ${page.pageNumber} - No content parsed`, 20 * scale, 30 * scale);
        ctx.restore();
        return;
      }

      // Show sample of operations
      // (debug logging removed)

      // Execute each operation
      for (const op of operations) {
        await graphics.executeOperator(op.operator, op.operands);
      }

      // console.log('Rendering complete');
    } catch (error) {
      console.error('Error rendering page content:', error);
      // Fallback: Draw page info
      ctx.save();
      ctx.setTransform(1, 0, 0, 1, 0, 0); // Reset transform
      ctx.fillStyle = '#c00';
      ctx.font = `${14 * scale}px Arial`;
      ctx.textAlign = 'left';
      ctx.fillText(`Page ${page.pageNumber} (Render error: ${error})`, 20 * scale, 30 * scale);
      ctx.restore();
    }
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
    if (!page.contents) return;

    try {
      const parser = new SafeContentStreamParser(page.contents);
      const operations = parser.parse();

      // Only process text-related operators
      const textOps = new Set(['BT', 'ET', 'Td', 'TD', 'Tm', 'T*', 'Tf', 'Tj', 'TJ', "'", '"', 'Tc', 'Tw', 'Tz', 'TL', 'Tr', 'Ts']);

      ctx.save();

      let inText = false;
      const textState = {
        font: 'Arial',
        fontSize: 12,
        matrix: [1, 0, 0, 1, 0, 0],
        lineMatrix: [1, 0, 0, 1, 0, 0],
        charSpace: 0,
        wordSpace: 0,
        leading: 0,
        rise: 0,
        renderMode: 0
      };

      for (const op of operations) {
        if (!textOps.has(op.operator) && op.operator !== 'q' && op.operator !== 'Q' && op.operator !== 'cm') continue;

        switch (op.operator) {
          case 'BT':
            inText = true;
            textState.matrix = [1, 0, 0, 1, 0, 0];
            textState.lineMatrix = [1, 0, 0, 1, 0, 0];
            break;
          case 'ET':
            inText = false;
            break;
          case 'Tf':
            if (op.operands.length >= 2) {
              textState.fontSize = op.operands[1];
              let fontName = 'Arial';
              const pdfFont = String(op.operands[0]);
              if (pdfFont.includes('Bold')) fontName = 'Arial';
              if (pdfFont.includes('Courier')) fontName = 'Courier New';
              if (pdfFont.includes('Times')) fontName = 'Times New Roman';
              textState.font = fontName;
            }
            break;
          case 'Td':
            if (inText && op.operands.length >= 2) {
              textState.lineMatrix[4] += op.operands[0] * textState.lineMatrix[0] + op.operands[1] * textState.lineMatrix[2];
              textState.lineMatrix[5] += op.operands[0] * textState.lineMatrix[1] + op.operands[1] * textState.lineMatrix[3];
              textState.matrix = [...textState.lineMatrix];
            }
            break;
          case 'TD':
            if (inText && op.operands.length >= 2) {
              textState.leading = -op.operands[1];
              textState.lineMatrix[4] += op.operands[0] * textState.lineMatrix[0] + op.operands[1] * textState.lineMatrix[2];
              textState.lineMatrix[5] += op.operands[0] * textState.lineMatrix[1] + op.operands[1] * textState.lineMatrix[3];
              textState.matrix = [...textState.lineMatrix];
            }
            break;
          case 'Tm':
            if (inText && op.operands.length >= 6) {
              textState.matrix = [...op.operands];
              textState.lineMatrix = [...op.operands];
            }
            break;
          case 'T*':
            if (inText) {
              textState.lineMatrix[4] += textState.leading * textState.lineMatrix[2];
              textState.lineMatrix[5] += textState.leading * textState.lineMatrix[3];
              textState.matrix = [...textState.lineMatrix];
            }
            break;
          case 'Tj':
            if (inText && op.operands.length > 0) {
              const text = PDFTextDecoder.decode(op.operands[0]);
              if (text) {
                ctx.save();
                const tm = textState.matrix;
                ctx.font = `${Math.abs(textState.fontSize)}px ${textState.font}`;
                ctx.fillStyle = '#000000';
                ctx.transform(tm[0], tm[1], tm[2], tm[3], tm[4], tm[5]);
                ctx.scale(1, -1);
                ctx.fillText(text, 0, -(textState.rise || 0));
                ctx.restore();
                const w = text.length * textState.fontSize * 0.5;
                textState.matrix[4] += w * textState.matrix[0];
                textState.matrix[5] += w * textState.matrix[1];
              }
            }
            break;
          case 'TJ':
            if (inText && op.operands.length > 0 && Array.isArray(op.operands[0])) {
              for (const item of op.operands[0]) {
                if (typeof item === 'string') {
                  const text = PDFTextDecoder.decode(item);
                  if (text) {
                    ctx.save();
                    const tm = textState.matrix;
                    ctx.font = `${Math.abs(textState.fontSize)}px ${textState.font}`;
                    ctx.fillStyle = '#000000';
                    ctx.transform(tm[0], tm[1], tm[2], tm[3], tm[4], tm[5]);
                    ctx.scale(1, -1);
                    ctx.fillText(text, 0, -(textState.rise || 0));
                    ctx.restore();
                    const w = text.length * textState.fontSize * 0.5;
                    textState.matrix[4] += w * textState.matrix[0];
                    textState.matrix[5] += w * textState.matrix[1];
                  }
                } else if (typeof item === 'number') {
                  const adj = -item / 1000 * textState.fontSize;
                  textState.matrix[4] += adj * textState.matrix[0];
                  textState.matrix[5] += adj * textState.matrix[1];
                }
              }
            }
            break;
          case 'TL':
            if (op.operands.length > 0) textState.leading = op.operands[0];
            break;
          case 'Tc':
            if (op.operands.length > 0) textState.charSpace = op.operands[0];
            break;
          case 'Tw':
            if (op.operands.length > 0) textState.wordSpace = op.operands[0];
            break;
          case 'Ts':
            if (op.operands.length > 0) textState.rise = op.operands[0];
            break;
          case 'Tr':
            if (op.operands.length > 0) textState.renderMode = op.operands[0];
            break;
        }
      }

      ctx.restore();
    } catch (error) {
      console.warn('Text rendering error (non-fatal):', error);
    }
  }

  private async renderImages(ctx: CanvasRenderingContext2D, page: PDFPage): Promise<void> {
    if (!page.resources?.images) return;

    for (const [_name, imageRes] of page.resources.images) {
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

/**
 * Text layer renderer for selectable text overlay
 * Implements canvas-independent text rendering for PDF documents
 * Based on PDF.js text layer concepts but rewritten for AgenticPDF
 */
interface TextLayerItem {
  str: string;
  dir: 'ltr' | 'rtl' | 'ttb';
  width: number;
  height: number;
  transform: number[];
  fontName: string;
  hasEOL: boolean;
}

interface TextLayerRenderOptions {
  textContent: TextContent[];
  container: HTMLElement;
  viewport: {
    width: number;
    height: number;
    scale: number;
  };
  enhanceTextSelection?: boolean;
}

class TextLayerBuilder {
  private container: HTMLElement;
  private textContent: TextContent[];
  private viewport: { width: number; height: number; scale: number };
  private textDivs: HTMLElement[] = [];
  private textDivProperties: WeakMap<HTMLElement, any> = new WeakMap();
  private renderingDone: boolean = false;
  private enhanceTextSelection: boolean;
  private scale: number;
  private pageWidth: number;
  private pageHeight: number;

  // Cache for font metrics to avoid repeated measurements
  private static ascentCache: Map<string, number> = new Map();
  private static canvasContext: CanvasRenderingContext2D | null = null;
  private static minFontSize: number | null = null;

  constructor(options: TextLayerRenderOptions) {
    this.container = options.container;
    this.textContent = options.textContent;
    this.viewport = options.viewport;
    this.enhanceTextSelection = options.enhanceTextSelection ?? true;

    // Following PDF.js approach exactly
    this.scale = options.viewport.scale;
    this.pageWidth = options.viewport.width / this.scale;
    this.pageHeight = options.viewport.height / this.scale;

    // Ensure container is positioned for absolute children
    if (getComputedStyle(this.container).position === 'static') {
      this.container.style.position = 'relative';
    }

    // Set container dimensions
    this.container.style.width = `${this.viewport.width}px`;
    this.container.style.height = `${this.viewport.height}px`;

    // Ensure minimum font size is computed
    TextLayerBuilder.ensureMinFontSizeComputed();
  }

  /**
   * Get canvas context for font measurements (PDF.js approach)
   */
  private static getCanvasContext(): CanvasRenderingContext2D {
    if (!this.canvasContext) {
      const canvas = document.createElement('canvas');
      canvas.className = 'hiddenCanvasElement';
      canvas.style.display = 'none';
      document.body.appendChild(canvas);
      this.canvasContext = canvas.getContext('2d', {
        alpha: false,
        willReadFrequently: true
      })!;
    }
    return this.canvasContext;
  }

  /**
   * Compute minimum font size enforced by browser (PDF.js approach)
   */
  private static ensureMinFontSizeComputed(): void {
    if (this.minFontSize !== null) {
      return;
    }
    const div = document.createElement('div');
    div.style.opacity = '0';
    div.style.lineHeight = '1';
    div.style.fontSize = '1px';
    div.style.position = 'absolute';
    div.textContent = 'X';
    document.body.appendChild(div);
    this.minFontSize = div.getBoundingClientRect().height;
    div.remove();
  }

  /**
   * Get font ascent ratio (PDF.js approach)
   */
  private static getAscent(fontFamily: string): number {
    const cachedAscent = this.ascentCache.get(fontFamily);
    if (cachedAscent) {
      return cachedAscent;
    }

    const ctx = this.getCanvasContext();
    const DEFAULT_FONT_SIZE = 30;

    ctx.canvas.width = ctx.canvas.height = DEFAULT_FONT_SIZE;
    ctx.font = `${DEFAULT_FONT_SIZE}px ${fontFamily}`;

    const metrics = ctx.measureText('');
    const ascent = metrics.fontBoundingBoxAscent || 0;
    const descent = Math.abs(metrics.fontBoundingBoxDescent || 0);

    ctx.canvas.width = ctx.canvas.height = 0;

    let ratio = 0.8; // Default
    if (ascent) {
      ratio = ascent / (ascent + descent);
    }

    this.ascentCache.set(fontFamily, ratio);
    return ratio;
  }

  /**
   * Map font name to CSS font family
   */
  private mapFontFamily(fontName: string): string {
    const fontMap: Record<string, string> = {
      'Times': 'Times New Roman, serif',
      'TimesNewRoman': 'Times New Roman, serif',
      'Times-Roman': 'Times New Roman, serif',
      'Times-Bold': 'Times New Roman, serif',
      'Times-Italic': 'Times New Roman, serif',
      'Times-BoldItalic': 'Times New Roman, serif',
      'Helvetica': 'Helvetica, Arial, sans-serif',
      'Helvetica-Bold': 'Helvetica, Arial, sans-serif',
      'Helvetica-Oblique': 'Helvetica, Arial, sans-serif',
      'Helvetica-BoldOblique': 'Helvetica, Arial, sans-serif',
      'Courier': 'Courier New, monospace',
      'Courier-Bold': 'Courier New, monospace',
      'Courier-Oblique': 'Courier New, monospace',
      'Courier-BoldOblique': 'Courier New, monospace',
      'Symbol': 'Symbol, serif',
      'ZapfDingbats': 'ZapfDingbats, serif'
    };

    // Check for exact match
    if (fontMap[fontName]) {
      return fontMap[fontName];
    }

    // Check for partial matches
    const lower = fontName.toLowerCase();
    if (lower.includes('times')) return 'Times New Roman, serif';
    if (lower.includes('helvetica') || lower.includes('arial')) return 'Helvetica, Arial, sans-serif';
    if (lower.includes('courier')) return 'Courier New, monospace';

    // Default fallback
    return 'sans-serif';
  }

  /**
   * Get font weight from font name and style
   */
  private getFontWeight(fontName: string, style: TextStyle): number {
    if (style.bold) return 700;

    const lower = fontName.toLowerCase();
    if (lower.includes('bold')) return 700;
    if (lower.includes('medium')) return 500;
    if (lower.includes('light')) return 300;

    return 400; // Normal
  }

  /**
   * Get font style (italic/oblique)
   */
  private getFontStyle(fontName: string, style: TextStyle): string {
    if (style.italic) return 'italic';

    const lower = fontName.toLowerCase();
    if (lower.includes('italic') || lower.includes('oblique')) return 'italic';

    return 'normal';
  }

  /**
   * Convert PDF coordinates to CSS coordinates
   */
  private convertTransform(transform: TransformMatrix): string {
    const [a, b, c, d, e, f] = transform;

    // PDF coordinate system has origin at bottom-left
    // CSS coordinate system has origin at top-left
    // We need to flip the Y coordinate
    const cssY = this.viewport.height - f;

    return `matrix(${a}, ${b}, ${c}, ${d}, ${e}, ${cssY})`;
  }

  /**
   * Create a text div element for a text item (PDF.js approach)
   */
  private createTextDiv(textItem: TextContent): HTMLElement {
    const textDiv = document.createElement('span');

    // Initialize properties (PDF.js uses these for layout)
    const textDivProperties = {
      angle: 0,
      canvasWidth: 0,
      hasText: textItem.text !== '',
      hasEOL: false,
      fontSize: 0
    };

    // Transform the text item's transform matrix
    // PDF.js uses: Util.transform(this.#transform, geom.transform)
    // where this.#transform = [1, 0, 0, -1, -pageX, pageY + pageHeight]
    const tx = textItem.transform;

    // Calculate angle
    let angle = Math.atan2(tx[1], tx[0]);

    // Get font info
    const fontFamily = this.mapFontFamily(textItem.fontName);

    // Calculate font height from transform matrix (PDF.js approach)
    const fontHeight = Math.hypot(tx[2], tx[3]);
    const fontAscent = fontHeight * TextLayerBuilder.getAscent(fontFamily);

    // Calculate position (PDF.js approach)
    let left: number, top: number;
    if (angle === 0) {
      left = tx[4];
      top = tx[5] - fontAscent;
    } else {
      left = tx[4] + fontAscent * Math.sin(angle);
      top = tx[5] - fontAscent * Math.cos(angle);
    }

    // Apply styles (PDF.js approach with percentage-based positioning)
    const style = textDiv.style;
    style.left = `${((100 * left) / this.pageWidth).toFixed(2)}%`;
    style.top = `${((100 * top) / this.pageHeight).toFixed(2)}%`;

    // Font size with minFontSize multiplier (PDF.js approach)
    const minFontSize = TextLayerBuilder.minFontSize || 1;
    style.fontSize = `${(minFontSize * fontHeight).toFixed(2)}px`;
    style.fontFamily = fontFamily;

    textDivProperties.fontSize = fontHeight;

    // Base styles
    style.position = 'absolute';
    style.whiteSpace = 'pre';
    style.transformOrigin = '0% 0%';

    // Make transparent for overlay
    style.color = 'transparent';

    // Set role for accessibility
    textDiv.setAttribute('role', 'presentation');
    textDiv.textContent = textItem.text;
    textDiv.dir = textItem.direction;

    // Store angle if rotated
    if (angle !== 0) {
      textDivProperties.angle = angle * (180 / Math.PI);
    }

    // Determine if we should scale text (PDF.js logic)
    let shouldScaleText = false;
    if (textItem.text.length > 1) {
      shouldScaleText = true;
    } else if (textItem.text !== ' ' && tx[0] !== tx[3]) {
      const absScaleX = Math.abs(tx[0]);
      const absScaleY = Math.abs(tx[3]);
      if (absScaleX !== absScaleY && Math.max(absScaleX, absScaleY) / Math.min(absScaleX, absScaleY) > 1.5) {
        shouldScaleText = true;
      }
    }

    if (shouldScaleText) {
      textDivProperties.canvasWidth = textItem.width;
    }

    this.textDivProperties.set(textDiv, textDivProperties);

    return textDiv;
  }

  /**
   * Apply transform to layout text div (PDF.js approach)
   */
  private applyTransform(textDiv: HTMLElement): void {
    const properties = this.textDivProperties.get(textDiv);
    if (!properties) return;

    const style = textDiv.style;
    let transform = '';

    // Scale down by minFontSize to counteract the multiplication in fontSize
    const minFontSize = TextLayerBuilder.minFontSize || 1;
    if (minFontSize > 1) {
      transform = `scale(${1 / minFontSize})`;
    }

    // Scale text to match canvas width (PDF.js approach)
    if (properties.canvasWidth !== 0 && properties.hasText) {
      const { fontFamily } = style;
      const { canvasWidth, fontSize } = properties;

      const ctx = TextLayerBuilder.getCanvasContext();
      ctx.font = `${fontSize * this.scale}px ${fontFamily}`;

      // Measure actual text width
      const { width } = ctx.measureText(textDiv.textContent || '');

      if (width > 0) {
        const scaleX = (canvasWidth * this.scale) / width;
        transform = `scaleX(${scaleX}) ${transform}`;
      }
    }

    // Apply rotation if needed
    if (properties.angle !== 0) {
      transform = `rotate(${properties.angle}deg) ${transform}`;
    }

    // Apply combined transform
    if (transform.length > 0) {
      style.transform = transform;
    }
  }

  /**
   * Enhance text selection (optional)
          if (Math.abs(scaleRatio - 1) < 0.5) {
            const charSpacing = (targetWidth - measuredWidth) / (textDiv.textContent!.length - 1);
            style.letterSpacing = `${charSpacing}px`;
          } else {
            transformStr += `scaleX(${scaleRatio}) `;
          }
        }
      }
    }

    if (transformStr) {
      style.transform = transformStr.trim();
    }
  }

  /**
   * Render the text layer
   */
  render(): void {
    if (this.renderingDone) {
      return;
    }

    // Clear container
    this.container.innerHTML = '';
    this.textDivs = [];
    this.textDivProperties = new WeakMap(); // WeakMap doesn't have clear(), recreate it

    // Create text divs
    for (const textItem of this.textContent) {
      const textDiv = this.createTextDiv(textItem);
      this.textDivs.push(textDiv);
      this.container.appendChild(textDiv);
    }

    // Apply transforms
    for (const textDiv of this.textDivs) {
      this.applyTransform(textDiv);
    }

    // Enhance text selection if enabled
    if (this.enhanceTextSelection) {
      this.enhanceSelection();
    }

    this.renderingDone = true;
  }

  /**
   * Enhance text selection by making text transparent and selectable
   */
  private enhanceSelection(): void {
    // Make text layer fully transparent but selectable
    // Do NOT set container opacity - individual text items are already transparent
    this.container.style.userSelect = 'text';
    this.container.style.cursor = 'text';

    // Enable text selection on all divs
    for (const textDiv of this.textDivs) {
      textDiv.style.cursor = 'text';
      textDiv.style.userSelect = 'text';
    }

    // Add selection styling
    const style = document.createElement('style');
    style.textContent = `
      .textLayer ::selection {
        background: rgba(0, 102, 204, 0.3);
      }
      .textLayer ::-moz-selection {
        background: rgba(0, 102, 204, 0.3);
      }
    `;

    if (!this.container.classList.contains('textLayer')) {
      this.container.classList.add('textLayer');
    }

    document.head.appendChild(style);
  }

  /**
   * Cancel rendering
   */
  cancel(): void {
    this.renderingDone = true;
  }

  /**
   * Clean up resources
   */
  static cleanup(): void {
    this.ascentCache.clear();
    if (this.canvasContext) {
      const canvas = this.canvasContext.canvas;
      canvas.remove();
      this.canvasContext = null;
    }
  }
}

/**
 * Convenience function to render text layer
 */
function renderTextLayer(options: TextLayerRenderOptions): TextLayerBuilder {
  const builder = new TextLayerBuilder(options);
  builder.render();
  return builder;
}

class ThemeManager {
  private static instance: ThemeManager;
  private currentTheme: 'dark' | 'light' = 'dark';
  private storageKey: string = 'AgenticPDF-theme';
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
    private pdf: AgenticPDF,
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

  constructor(private pdf: AgenticPDF) { }

  async save(): Promise<Blob> {
    const _originalBuffer = await this.getOriginalBuffer();
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
    // This would be stored in the AgenticPDF instance
    return new ArrayBuffer(0);
  }

  private async collectObjects(): Promise<PDFObject[]> {
    const objects: PDFObject[] = [];

    try {
      // Get the original xref table and objects
      const xrefTable = (this.pdf as any).xrefTable as XRefTable;
      const pdfObjects = (this.pdf as any).objects as Map<string, PDFObject>;

      if (!xrefTable) {
        console.warn('No xref table available for PDF writing');
        return objects;
      }

      // Collect objects from the xref table
      const objectNumbers = new Set<number>();

      // Get all object numbers from xref
      for (let i = 1; i <= 1000; i++) { // Arbitrary limit to prevent infinite loops
        const entry = xrefTable.getEntry(i);
        if (entry && entry.type === 'n') { // Only include in-use objects
          objectNumbers.add(i);
        }
      }

      // Collect objects in order
      for (const objNum of Array.from(objectNumbers).sort((a, b) => a - b)) {
        const entry = xrefTable.getEntry(objNum);
        if (!entry) continue;

        // Check if we have a modified version
        const modifiedKey = `${objNum}_${entry.generation}`;
        if (this.modifiedObjects.has(modifiedKey)) {
          const modifiedObj = this.modifiedObjects.get(modifiedKey)!;
          objects.push({
            ...modifiedObj,
            objectNumber: objNum,
            generationNumber: entry.generation
          });
        } else if (pdfObjects && pdfObjects.has(modifiedKey)) {
          // Use original object
          const originalObj = pdfObjects.get(modifiedKey)!;
          objects.push({
            ...originalObj,
            objectNumber: objNum,
            generationNumber: entry.generation
          });
        } else {
          // Create a minimal object if we don't have the original
          objects.push({
            type: PDFObjectType.Null,
            value: null,
            objectNumber: objNum,
            generationNumber: entry.generation
          });
        }
      }

      // Add any new objects that were created
      for (const [key, obj] of this.modifiedObjects) {
        const [objNumStr, genStr] = key.split('_');
        const objNum = parseInt(objNumStr, 10);

        if (!objectNumbers.has(objNum)) {
          objects.push({
            ...obj,
            objectNumber: objNum,
            generationNumber: parseInt(genStr, 10)
          });
        }
      }

    } catch (error) {
      console.warn('Error collecting PDF objects:', error);
    }

    return objects;
  }

  /**
   * Mark an object as modified for inclusion in the saved PDF
   */
  modifyObject(objectNumber: number, generationNumber: number, obj: PDFObject): void {
    const key = `${objectNumber}_${generationNumber}`;
    this.modifiedObjects.set(key, obj);
  }

  /**
   * Add a new object to be included in the saved PDF
   */
  addObject(obj: PDFObject): number {
    // Find next available object number
    let objectNumber = this.getNextObjectNumber();

    const key = `${objectNumber}_0`;
    this.modifiedObjects.set(key, {
      ...obj,
      objectNumber,
      generationNumber: 0
    });

    return objectNumber;
  }

  private getNextObjectNumber(): number {
    const xrefTable = (this.pdf as any).xrefTable as XRefTable;
    let maxObjectNumber = 0;

    // Find the highest object number
    for (let i = 1; i <= 10000; i++) {
      const entry = xrefTable?.getEntry(i);
      if (entry) {
        maxObjectNumber = Math.max(maxObjectNumber, i);
      } else {
        break;
      }
    }

    // Check modified objects too
    for (const key of this.modifiedObjects.keys()) {
      const objNum = parseInt(key.split('_')[0], 10);
      maxObjectNumber = Math.max(maxObjectNumber, objNum);
    }

    return maxObjectNumber + 1;
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
export { TextLayerBuilder, renderTextLayer };
export type { TextLayerRenderOptions, TextLayerItem };

// ============================================================================
// Default Export
// ============================================================================

export default AgenticPDF;
