/**
 * Integration tests against real sample PDF (demos/sample.pdf)
 * 
 * Tests actual PDF parsing, text extraction, metadata, annotations,
 * named destinations, ontology, and AI features against a real document.
 * 
 * sample.pdf: "Inverting Trojans in LLMs" — 8-page academic paper, PDF 1.7
 */

import { AgenticPDF } from '../../agenticpdf';
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

describe('Real PDF: Loading & Metadata', () => {
  test('should load sample.pdf successfully', () => {
    expect(pdf).toBeDefined();
  });

  test('should report correct page count', () => {
    expect(pdf.getPageCount()).toBe(8);
  });

  test('should extract metadata title', () => {
    const meta = pdf.getMetadata();
    expect(meta).toBeDefined();
    expect(meta!.title).toBe('Inverting Trojans in LLMs');
  });

  test('should extract metadata author', () => {
    const meta = pdf.getMetadata();
    expect(meta).toBeDefined();
    expect(meta!.author).toContain('Zhengxing Li');
    expect(meta!.author).toContain('George Kesidis');
  });

  test('should extract producer', () => {
    const meta = pdf.getMetadata();
    expect(meta).toBeDefined();
    expect(meta!.producer).toContain('pikepdf');
  });

  test('should extract creator', () => {
    const meta = pdf.getMetadata();
    expect(meta).toBeDefined();
    expect(meta!.creator).toContain('arXiv');
  });

  test('should report correct page count in metadata', () => {
    const meta = pdf.getMetadata();
    expect(meta).toBeDefined();
    expect(meta!.pageCount).toBe(8);
  });

  test('should detect PDF version 1.7', () => {
    const meta = pdf.getMetadata();
    expect(meta).toBeDefined();
    expect(meta!.version).toBe('1.7');
  });

  test('should report file size', () => {
    expect(pdf.getFileSize()).toBeGreaterThan(300000);
    expect(pdf.getFileSize()).toBeLessThan(400000);
  });

  test('should not be encrypted', () => {
    expect(pdf.isEncrypted()).toBe(false);
  });
});

describe('Real PDF: Text Extraction', () => {
  let textContent: Awaited<ReturnType<typeof pdf.extractText>>;

  beforeAll(async () => {
    textContent = await pdf.extractText();
  });

  test('should extract text items from all pages', () => {
    expect(textContent.length).toBeGreaterThan(500);
  });

  test('should have text on all 8 pages', () => {
    const pages = new Set(textContent.map(t => t.pageNumber));
    expect(pages.size).toBe(8);
    for (let i = 1; i <= 8; i++) {
      expect(pages.has(i)).toBe(true);
    }
  });

  test('should extract paper title on page 1', () => {
    const page1Text = textContent
      .filter(t => t.pageNumber === 1)
      .map(t => t.text)
      .join(' ');
    expect(page1Text).toContain('Inverting Trojans');
  });

  test('should extract author names on page 1', () => {
    const page1Text = textContent
      .filter(t => t.pageNumber === 1)
      .map(t => t.text)
      .join(' ');
    expect(page1Text).toContain('Zhengxing Li');
    expect(page1Text).toContain('Penn State');
  });

  test('should extract abstract on page 1', () => {
    const page1Text = textContent
      .filter(t => t.pageNumber === 1)
      .map(t => t.text)
      .join(' ');
    expect(page1Text).toContain('Abstract');
    expect(page1Text).toContain('backdoor');
  });

  test('text items should have valid page numbers', () => {
    for (const item of textContent) {
      expect(item.pageNumber).toBeGreaterThanOrEqual(1);
      expect(item.pageNumber).toBeLessThanOrEqual(8);
    }
  });

  test('text items should have non-empty text', () => {
    const nonEmpty = textContent.filter(t => t.text.trim().length > 0);
    expect(nonEmpty.length).toBeGreaterThan(400);
  });

  test('page 1 should have substantial text items', () => {
    const page1Count = textContent.filter(t => t.pageNumber === 1).length;
    expect(page1Count).toBeGreaterThan(50);
  });
});

describe('Real PDF: Named Destinations', () => {
  test('should resolve named destinations', () => {
    const dests = pdf.getNamedDestinations();
    expect(dests).toBeDefined();
    expect(dests.size).toBe(57);
  });

  test('named destinations should have valid page references', () => {
    const dests = pdf.getNamedDestinations();
    for (const [_name, dest] of dests) {
      expect(dest.page).toBeGreaterThanOrEqual(0);
      expect(dest.page).toBeLessThanOrEqual(8);
    }
  });
});

describe('Real PDF: Ontology & Discovery', () => {
  test('should return ontology with concepts', () => {
    const ontology = AgenticPDF.describe();
    expect(ontology.concepts.length).toBe(21);
  });

  test('should return capabilities', () => {
    const caps = AgenticPDF.getCapabilities();
    expect(caps.length).toBe(14);
  });

  test('should return method signatures', () => {
    const methods = AgenticPDF.getMethodSignatures();
    expect(methods.length).toBeGreaterThanOrEqual(26);
  });

  test('should return workflow templates', () => {
    const workflows = AgenticPDF.getWorkflows();
    expect(workflows.length).toBe(14);
  });

  test('describeDocument should report on loaded PDF', () => {
    const report = pdf.describeDocument();
    expect(report).toBeDefined();
    if (report) {
      expect(report.documentInfo.pageCount).toBe(8);
    }
  });
});

describe('Real PDF: Streaming Text', () => {
  test('should stream text content', async () => {
    const chunks: string[] = [];
    for await (const item of pdf.streamText()) {
      chunks.push(item.text);
    }
    expect(chunks.length).toBeGreaterThan(0);
    expect(chunks.join(' ')).toContain('Inverting');
  });
});

describe('Real PDF: AI Features', () => {
  test('should generate semantic chunks', async () => {
    const chunks = await pdf.generateSemanticChunks({
      strategy: 'fixed',
      maxChunkSize: 500,
    });
    expect(chunks.length).toBeGreaterThan(0);
    expect(chunks[0].content.length).toBeGreaterThan(0);
  });

  test('should stream semantic chunks', async () => {
    const chunks: unknown[] = [];
    for await (const chunk of pdf.streamSemanticChunks({
      strategy: 'fixed',
      maxChunkSize: 500,
    })) {
      chunks.push(chunk);
    }
    expect(chunks.length).toBeGreaterThan(0);
  });

  test('should get AI features', async () => {
    const features = await pdf.getAIFeatures({
      enableStructuralAnalysis: true,
      enableSemanticChunking: true,
      chunkSize: 500,
    });
    expect(features).toBeDefined();
    expect(features.semanticChunks.length).toBeGreaterThan(0);
  });
});

describe('Real PDF: Search', () => {
  test('should find text by keyword', async () => {
    const results = await pdf.search('Abstract');
    expect(results.length).toBeGreaterThanOrEqual(1);
  });

  test('should find text across pages', async () => {
    const results = await pdf.search('the');
    expect(results.length).toBeGreaterThan(1);
  });

  test('should return empty for non-existent text', async () => {
    const results = await pdf.search('xyzzyfoobarbaz_nonexistent');
    expect(results.length).toBe(0);
  });
});

describe('Real PDF: Export', () => {
  test('should export as text', async () => {
    const result = await pdf.exportAs('text');
    const text = typeof result === 'string' ? result : await result.text();
    expect(text).toContain('Inverting');
    expect(text.length).toBeGreaterThan(1000);
  });

  test('should export as html', async () => {
    const result = await pdf.exportAs('html');
    const html = typeof result === 'string' ? result : await result.text();
    expect(html).toContain('<html');
    expect(html).toContain('Inverting');
  });

  test('should export as markdown', async () => {
    const result = await pdf.exportAs('markdown');
    const md = typeof result === 'string' ? result : await result.text();
    expect(md).toContain('Inverting');
  });

  test('should export as json', async () => {
    const result = await pdf.exportAs('json');
    const json = typeof result === 'string' ? result : await result.text();
    const parsed = JSON.parse(json);
    expect(parsed).toBeDefined();
  });
});

describe('Real PDF: Second Instance from Buffer', () => {
  test('should create independent instance', async () => {
    const pdf2 = await AgenticPDF.fromBuffer(pdfBuffer);
    expect(pdf2.getPageCount()).toBe(8);
    const meta = pdf2.getMetadata();
    expect(meta).toBeDefined();
    expect(meta!.title).toBe('Inverting Trojans in LLMs');
    pdf2.close();
  });
});
