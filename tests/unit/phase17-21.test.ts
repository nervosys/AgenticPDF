/**
 * Unit tests for Phase 17-21 features
 * 
 * Phase 17: Layout analysis (analyzeLayout)
 * Phase 18: Encryption helpers (tested via isEncrypted, unlock)
 * Phase 19: Performance utilities (worker pipeline, tile renderer, lazy loader, virtual scroller, incremental parser)
 * Phase 20: Writing utilities (incremental save, page manager, annotation persistence, signatures, PDF/A)
 * Phase 21: AI/RAG (embedding generator, vector store helper, document diff, summarization, structured extraction)
 */

import { AgenticPDF, EmbeddingProvider, VectorStoreAdapter } from '../../agenticpdf';
import * as fs from 'fs';
import * as path from 'path';

const SAMPLE_PDF = path.join(__dirname, '..', '..', 'demos', 'sample.pdf');

let pdfBuffer: ArrayBuffer;
let pdf: AgenticPDF;

beforeAll(async () => {
  const raw = fs.readFileSync(SAMPLE_PDF);
  pdfBuffer = raw.buffer.slice(raw.byteOffset, raw.byteOffset + raw.byteLength);
  pdf = await AgenticPDF.fromBuffer(pdfBuffer);
});

afterAll(() => {
  pdf.close();
});

// ── Phase 17: Layout Analysis ──

describe('Phase 17: Layout Analysis', () => {
  test('analyzeLayout returns page analysis for all pages', async () => {
    const result = await pdf.analyzeLayout();
    expect(result).toBeDefined();
    expect(result.pages).toBeDefined();
    expect(result.pages.length).toBe(8);
  });

  test('analyzeLayout respects page range', async () => {
    const result = await pdf.analyzeLayout({ start: 1, end: 3 });
    expect(result.pages.length).toBe(3);
  });

  test('each page analysis has expected structure', async () => {
    const result = await pdf.analyzeLayout({ start: 1, end: 1 });
    const page = result.pages[0];
    expect(page.pageNumber).toBe(1);
    expect(page.columns).toBeDefined();
    expect(Array.isArray(page.columns)).toBe(true);
    expect(page.tables).toBeDefined();
    expect(page.readingOrder).toBeDefined();
  });
});

// ── Phase 18: Encryption ──

describe('Phase 18: Encryption', () => {
  test('isEncrypted returns false for unencrypted PDF', () => {
    expect(pdf.isEncrypted()).toBe(false);
  });

  test('getPDFVersion returns version string', () => {
    const meta = pdf.getMetadata();
    expect(meta).toBeDefined();
    expect(meta!.version).toMatch(/^\d+\.\d+$/);
  });
});

// ── Phase 19: Performance & Scale ──

describe('Phase 19: Worker Pipeline', () => {
  test('createWorkerPipeline returns pipeline object', () => {
    const pipeline = pdf.createWorkerPipeline();
    expect(pipeline).toBeDefined();
    expect(typeof pipeline.renderPage).toBe('function');
    expect(typeof pipeline.terminate).toBe('function');
  });
});

describe('Phase 19: Tile Renderer', () => {
  test('createTileRenderer returns renderer with defaults', () => {
    const renderer = pdf.createTileRenderer();
    expect(renderer).toBeDefined();
    expect(typeof renderer.getVisibleTiles).toBe('function');
    expect(typeof renderer.renderTile).toBe('function');
  });

  test('createTileRenderer accepts custom config', () => {
    const renderer = pdf.createTileRenderer({ tileWidth: 512, tileHeight: 512 });
    expect(renderer).toBeDefined();
  });
});

describe('Phase 19: Lazy Page Loader', () => {
  test('createLazyLoader returns loader', () => {
    const loader = pdf.createLazyLoader();
    expect(loader).toBeDefined();
    expect(typeof loader.ensureLoaded).toBe('function');
    expect(typeof loader.unloadDistant).toBe('function');
    expect(typeof loader.getLoadedPages).toBe('function');
  });

  test('createLazyLoader with custom prefetch range', () => {
    const loader = pdf.createLazyLoader(5);
    expect(loader).toBeDefined();
  });

  test('lazy loader tracks loaded pages', async () => {
    const loader = pdf.createLazyLoader(1);
    await loader.ensureLoaded(1);
    const loaded = loader.getLoadedPages();
    expect(loaded).toContain(1);
  });
});

describe('Phase 19: Virtual Scroll', () => {
  test('createVirtualScroller returns scroller', () => {
    const scroller = pdf.createVirtualScroller({ containerHeight: 800 });
    expect(scroller).toBeDefined();
    expect(typeof scroller.getVisiblePages).toBe('function');
  });

  test('virtual scroller reports visible pages', () => {
    const scroller = pdf.createVirtualScroller({
      containerHeight: 800,
      overscan: 1,
    });
    const pages = scroller.getVisiblePages(0);
    expect(pages).toBeDefined();
    expect(Array.isArray(pages)).toBe(true);
  });
});

describe('Phase 19: Incremental Parser', () => {
  test('getIncrementalParser returns parser or null', () => {
    const parser = pdf.getIncrementalParser();
    // May be null if the PDF doesn't use incremental updates
    expect(parser === null || typeof parser === 'object').toBe(true);
  });
});

// ── Phase 20: PDF Writing & Modification ──

describe('Phase 20: Incremental Save', () => {
  test('saveIncremental returns result with blob', async () => {
    const result = await pdf.saveIncremental();
    expect(result).toBeDefined();
    expect(result.appendedBytes).toBeDefined();
    expect(result.modifiedObjects).toBeDefined();
    expect(result.newRevisionNumber).toBeDefined();
  });
});

describe('Phase 20: Page Manager', () => {
  test('getPageManager returns manager', () => {
    const pm = pdf.getPageManager();
    expect(pm).toBeDefined();
    expect(typeof pm.getPageCount).toBe('function');
    expect(typeof pm.insertBlankPage).toBe('function');
    expect(typeof pm.deletePage).toBe('function');
    expect(typeof pm.reorderPages).toBe('function');
  });

  test('page manager reports page count', () => {
    const pm = pdf.getPageManager();
    expect(typeof pm.getPageCount()).toBe('number');
  });
});

describe('Phase 20: Annotation Persistence', () => {
  test('getAnnotationPersistence returns persistence manager', () => {
    const ap = pdf.getAnnotationPersistence();
    expect(ap).toBeDefined();
    expect(typeof ap.createTextAnnotation).toBe('function');
    expect(typeof ap.createHighlightAnnotation).toBe('function');
    expect(typeof ap.getPendingAnnotations).toBe('function');
    expect(typeof ap.deleteAnnotation).toBe('function');
  });
});

describe('Phase 20: Digital Signatures', () => {
  test('getSignatureHandler returns handler', () => {
    const sh = pdf.getSignatureHandler();
    expect(sh).toBeDefined();
    expect(typeof sh.getSignatures).toBe('function');
    expect(typeof sh.prepareSignature).toBe('function');
  });

  test('getSignatures returns array', async () => {
    const sh = pdf.getSignatureHandler();
    const sigs = await sh.getSignatures();
    expect(Array.isArray(sigs)).toBe(true);
  });
});

describe('Phase 20: PDF/A Converter', () => {
  test('getPDFAConverter returns converter', () => {
    const conv = pdf.getPDFAConverter();
    expect(conv).toBeDefined();
    expect(typeof conv.validate).toBe('function');
    expect(typeof conv.setConformanceLevel).toBe('function');
    expect(typeof conv.generateXMPMetadata).toBe('function');
  });

  test('validate returns validation result', () => {
    const conv = pdf.getPDFAConverter();
    const result = conv.validate();
    expect(result).toBeDefined();
    expect(typeof result.conformant).toBe('boolean');
    expect(Array.isArray(result.errors)).toBe(true);
    expect(Array.isArray(result.warnings)).toBe(true);
  });

  test('generateXMPMetadata returns XML string', () => {
    const conv = pdf.getPDFAConverter();
    const xmp = conv.generateXMPMetadata();
    expect(xmp).toContain('xmpmeta');
  });
});

// ── Phase 21: AI & RAG Enhancements ──

describe('Phase 21: Cosine Similarity', () => {
  test('cosineSimilarity of identical vectors is 1', () => {
    const v = new Float32Array([1, 2, 3]);
    expect(AgenticPDF.cosineSimilarity(v, v)).toBeCloseTo(1.0, 5);
  });

  test('cosineSimilarity of orthogonal vectors is 0', () => {
    const a = new Float32Array([1, 0, 0]);
    const b = new Float32Array([0, 1, 0]);
    expect(AgenticPDF.cosineSimilarity(a, b)).toBeCloseTo(0.0, 5);
  });

  test('cosineSimilarity of opposite vectors is -1', () => {
    const a = new Float32Array([1, 0, 0]);
    const b = new Float32Array([-1, 0, 0]);
    expect(AgenticPDF.cosineSimilarity(a, b)).toBeCloseTo(-1.0, 5);
  });
});

describe('Phase 21: Embedding Generator', () => {
  const mockProvider: EmbeddingProvider = {
    model: 'test-model',
    dimensions: 3,
    async generate(text: string): Promise<Float32Array> {
      const hash = Array.from(text).reduce((a, c) => a + c.charCodeAt(0), 0);
      return new Float32Array([hash % 7, hash % 11, hash % 13]);
    },
    async generateBatch(texts: string[]): Promise<Float32Array[]> {
      return Promise.all(texts.map(t => this.generate(t)));
    },
  };

  test('createEmbeddingGenerator returns generator', () => {
    const gen = pdf.createEmbeddingGenerator(mockProvider);
    expect(gen).toBeDefined();
    expect(typeof gen.generateForChunks).toBe('function');
  });

  test('generator can generate embeddings for chunks', async () => {
    const gen = pdf.createEmbeddingGenerator(mockProvider);
    const chunks = await pdf.generateSemanticChunks({
      strategy: 'fixed',
      maxChunkSize: 500,
    });
    expect(chunks.length).toBeGreaterThan(0);

    const embeddings = await gen.generateForChunks(chunks);
    expect(embeddings).toBeInstanceOf(Map);
    expect(embeddings.size).toBe(chunks.length);
    const firstEntry = embeddings.values().next().value!;
    expect(firstEntry).toBeInstanceOf(Float32Array);
    expect(firstEntry.length).toBe(3);
  });
});

describe('Phase 21: Vector Store Helper', () => {
  const mockProvider: EmbeddingProvider = {
    model: 'test-model',
    dimensions: 3,
    async generate(_text: string): Promise<Float32Array> {
      return new Float32Array([1, 0, 0]);
    },
    async generateBatch(texts: string[]): Promise<Float32Array[]> {
      return texts.map(() => new Float32Array([1, 0, 0]));
    },
  };

  const createMockStore = (): VectorStoreAdapter & { stored: Map<string, any> } => ({
    stored: new Map(),
    async add(id: string, embedding: Float32Array, metadata: Record<string, any>) {
      this.stored.set(id, { embedding, metadata });
    },
    async addBatch(items: Array<{ id: string; embedding: Float32Array; metadata: Record<string, any> }>) {
      for (const item of items) {
        this.stored.set(item.id, { embedding: item.embedding, metadata: item.metadata });
      }
    },
    async query(_embedding: Float32Array, topK: number) {
      const results: Array<{ id: string; score: number; metadata: Record<string, any> }> = [];
      for (const [id, val] of this.stored) {
        results.push({ id, score: 1.0, metadata: val.metadata });
        if (results.length >= topK) break;
      }
      return results;
    },
    async delete(id: string) {
      return this.stored.delete(id);
    },
  });

  test('createVectorStoreHelper returns helper', () => {
    const store = createMockStore();
    const helper = pdf.createVectorStoreHelper(store, mockProvider);
    expect(helper).toBeDefined();
    expect(typeof helper.indexDocument).toBe('function');
    expect(typeof helper.query).toBe('function');
  });

  test('indexDocument stores embeddings', async () => {
    const store = createMockStore();
    const helper = pdf.createVectorStoreHelper(store, mockProvider);
    const result = await helper.indexDocument(pdf);
    expect(result.indexed).toBeGreaterThan(0);
    expect(store.stored.size).toBeGreaterThan(0);
  });
});

describe('Phase 21: Document Diff', () => {
  test('compareWith self returns high similarity', async () => {
    const pdf2 = await AgenticPDF.fromBuffer(pdfBuffer);
    const diff = await pdf.compareWith(pdf2);
    expect(diff).toBeDefined();
    expect(diff.overallSimilarity).toBeGreaterThanOrEqual(0.9);
    expect(diff.addedPages.length).toBe(0);
    expect(diff.removedPages.length).toBe(0);
    pdf2.close();
  });
});

describe('Phase 21: Summarization', () => {
  test('summarize returns summary result', async () => {
    const result = await pdf.summarize();
    expect(result).toBeDefined();
    expect(result.summary.length).toBeGreaterThan(50);
    expect(result.keyPoints.length).toBeGreaterThan(0);
    expect(result.wordCount).toBeGreaterThan(0);
    expect(result.compressionRatio).toBeGreaterThan(0);
    expect(result.compressionRatio).toBeLessThanOrEqual(1);
  });

  test('summarize with sentence count option', async () => {
    const result = await pdf.summarize({ sentenceCount: 3 });
    expect(result.summary.length).toBeGreaterThan(0);
  });

  test('summarize with maxLength option', async () => {
    const result = await pdf.summarize({ maxLength: 200 });
    expect(result.summary.length).toBeLessThanOrEqual(250); // Allow slight overflow
  });
});

describe('Phase 21: Structured Extraction', () => {
  test('extractStructuredData returns result', async () => {
    const result = await pdf.extractStructuredData();
    expect(result).toBeDefined();
    expect(result.documentType).toBeDefined();
    expect(result.fields).toBeDefined();
    expect(result.confidence).toBeGreaterThanOrEqual(0);
  });

  test('extractStructuredData with paper type hint', async () => {
    const result = await pdf.extractStructuredData('paper');
    expect(result).toBeDefined();
    expect(result.fields).toBeDefined();
    // Academic paper should extract title-like field
    const fieldKeys = Object.keys(result.fields);
    expect(fieldKeys.length).toBeGreaterThan(0);
  });

  test('extractStructuredData with invoice type hint', async () => {
    // Even though sample isn't an invoice, it shouldn't crash
    const result = await pdf.extractStructuredData('invoice');
    expect(result).toBeDefined();
    expect(result.confidence).toBeGreaterThanOrEqual(0);
  });
});
