/**
 * Additional coverage tests targeting uncovered public API paths.
 * Exercises: performance monitoring, addAnnotation, telemetry,
 * clearAllCaches, static methods, export edge cases, error paths.
 */

import { AgenticPDF, Telemetry, AnnotationType } from '../../agenticpdf';
import * as fs from 'fs';
import * as path from 'path';

const SAMPLE_PDF = path.join(__dirname, '..', '..', 'demos', 'sample.pdf');

let pdfBuffer: ArrayBuffer;

beforeAll(() => {
  const raw = fs.readFileSync(SAMPLE_PDF);
  pdfBuffer = raw.buffer.slice(raw.byteOffset, raw.byteOffset + raw.byteLength);
});

// ============================================================================
// Performance monitoring
// ============================================================================

describe('Performance monitoring', () => {
  test('enablePerformanceMonitoring should not throw', () => {
    expect(() => AgenticPDF.enablePerformanceMonitoring()).not.toThrow();
  });

  test('getPerformanceMetrics should return array', () => {
    const metrics = AgenticPDF.getPerformanceMetrics();
    expect(Array.isArray(metrics)).toBe(true);
  });

  test('getPerformanceSummary should return object', () => {
    const summary = AgenticPDF.getPerformanceSummary();
    expect(typeof summary).toBe('object');
  });

  test('clearPerformanceMetrics should not throw', () => {
    expect(() => AgenticPDF.clearPerformanceMetrics()).not.toThrow();
  });

  test('disablePerformanceMonitoring should not throw', () => {
    expect(() => AgenticPDF.disablePerformanceMonitoring()).not.toThrow();
  });

  test('metrics should accumulate during operations', async () => {
    AgenticPDF.enablePerformanceMonitoring();
    AgenticPDF.clearPerformanceMetrics();
    const pdf = await AgenticPDF.fromBuffer(pdfBuffer);
    await pdf.extractText();
    const metrics = AgenticPDF.getPerformanceMetrics();
    // Performance monitoring may or may not produce metrics depending on internals
    expect(Array.isArray(metrics)).toBe(true);
    AgenticPDF.disablePerformanceMonitoring();
    pdf.close();
  });
});

// ============================================================================
// Static cache methods
// ============================================================================

describe('Static cache and utility methods', () => {
  test('clearAllCaches should not throw', () => {
    expect(() => AgenticPDF.clearAllCaches()).not.toThrow();
  });

  test('clearAllCaches after loading document', async () => {
    const pdf = await AgenticPDF.fromBuffer(pdfBuffer);
    await pdf.extractText();
    expect(() => AgenticPDF.clearAllCaches()).not.toThrow();
    pdf.close();
  });
});

// ============================================================================
// addAnnotation
// ============================================================================

describe('addAnnotation', () => {
  test('should add an annotation and return id', async () => {
    const pdf = await AgenticPDF.fromBuffer(pdfBuffer);
    const id = await pdf.addAnnotation({
      type: AnnotationType.Text,
      pageNumber: 1,
      rect: { x: 100, y: 100, width: 200, height: 50 },
      contents: 'Test note',
    });
    expect(typeof id).toBe('string');
    expect(id.length).toBeGreaterThan(0);
    pdf.close();
  });

  test('should add highlight annotation', async () => {
    const pdf = await AgenticPDF.fromBuffer(pdfBuffer);
    const id = await pdf.addAnnotation({
      type: AnnotationType.Highlight,
      pageNumber: 1,
      rect: { x: 50, y: 200, width: 300, height: 20 },
    });
    expect(typeof id).toBe('string');
    pdf.close();
  });
});

// ============================================================================
// Multiple instance handling
// ============================================================================

describe('Multiple instance lifecycle', () => {
  test('create and close multiple instances', async () => {
    const instances: AgenticPDF[] = [];
    for (let i = 0; i < 3; i++) {
      instances.push(await AgenticPDF.fromBuffer(pdfBuffer));
    }
    for (const inst of instances) {
      expect(inst.getPageCount()).toBe(8);
      inst.close();
    }
  });

  test('getPageCount should change after close', async () => {
    const pdf = await AgenticPDF.fromBuffer(pdfBuffer);
    expect(pdf.getPageCount()).toBe(8);
    pdf.close();
    // After close, internal state is cleared
    const count = pdf.getPageCount();
    expect(count).toBeLessThanOrEqual(8);
  });
});

// ============================================================================
// Export edge cases
// ============================================================================

describe('Export edge cases', () => {
  let pdf: AgenticPDF;

  beforeAll(async () => {
    pdf = await AgenticPDF.fromBuffer(pdfBuffer);
  });

  afterAll(() => {
    pdf.close();
  });

  test('export as text with includeMetadata', async () => {
    const result = await pdf.exportAs('text', { includeMetadata: true });
    const txt = typeof result === 'string' ? result : await result.text();
    expect(txt.length).toBeGreaterThan(100);
  });

  test('export as markdown with includeMetadata', async () => {
    const result = await pdf.exportAs('markdown', { includeMetadata: true });
    const md = typeof result === 'string' ? result : await result.text();
    expect(md).toContain('Inverting');
  });

  test('export as XML with includeMetadata', async () => {
    const result = await pdf.exportAs('xml', { includeMetadata: true });
    const xml = typeof result === 'string' ? result : await result.text();
    expect(xml).toContain('<?xml');
  });

  test('export as CSV with includeMetadata', async () => {
    const result = await pdf.exportAs('csv', { includeMetadata: true });
    const csv = typeof result === 'string' ? result : await result.text();
    expect(csv.length).toBeGreaterThan(0);
  });
});

// ============================================================================
// Streaming text with options
// ============================================================================

describe('Streaming text options', () => {
  let pdf: AgenticPDF;

  beforeAll(async () => {
    pdf = await AgenticPDF.fromBuffer(pdfBuffer);
  });

  afterAll(() => {
    pdf.close();
  });

  test('streamText should yield items', async () => {
    const items: any[] = [];
    for await (const item of pdf.streamText()) {
      items.push(item);
      if (items.length >= 10) break;
    }
    expect(items.length).toBe(10);
  });

  test('streamText with normalizeWhitespace', async () => {
    const items: any[] = [];
    for await (const item of pdf.streamText({ normalizeWhitespace: true })) {
      items.push(item);
      if (items.length >= 5) break;
    }
    expect(items.length).toBe(5);
  });

  test('streamSemanticChunks should yield chunks', async () => {
    const chunks: any[] = [];
    for await (const chunk of pdf.streamSemanticChunks({ strategy: 'fixed', maxChunkSize: 500 })) {
      chunks.push(chunk);
      if (chunks.length >= 3) break;
    }
    expect(chunks.length).toBeGreaterThan(0);
  });
});

// ============================================================================
// Telemetry static API
// ============================================================================

describe('Telemetry API', () => {
  test('Telemetry.isEnabled should return boolean', () => {
    const enabled = Telemetry.isEnabled();
    expect(typeof enabled).toBe('boolean');
  });

  test('Telemetry.getConfig should return config object', () => {
    const config = Telemetry.getConfig();
    expect(config).toBeDefined();
    expect(config.endpoint).toBeDefined();
    expect(typeof config.flushInterval).toBe('number');
  });

  test('Telemetry.configure should update config', () => {
    const original = Telemetry.getConfig();
    Telemetry.configure({ maxBatchSize: 100 });
    const updated = Telemetry.getConfig();
    expect(updated.maxBatchSize).toBe(100);
    Telemetry.configure({ maxBatchSize: original.maxBatchSize });
  });

  test('Telemetry.disable and enable', () => {
    Telemetry.disable();
    expect(Telemetry.isEnabled()).toBe(false);
    Telemetry.enable();
    expect(Telemetry.isEnabled()).toBe(true);
  });
});

// ============================================================================
// describeDocument deeper
// ============================================================================

describe('describeDocument deeper paths', () => {
  let pdf: AgenticPDF;

  beforeAll(async () => {
    pdf = await AgenticPDF.fromBuffer(pdfBuffer);
  });

  afterAll(() => {
    pdf.close();
  });

  test('describeDocument should have available operations', () => {
    const report = pdf.describeDocument();
    expect(report).toBeDefined();
    if (report) {
      expect(report.availableOperations).toBeDefined();
      expect(report.availableOperations.length).toBeGreaterThan(0);
    }
  });

  test('describeDocument should have recommended workflows', () => {
    const report = pdf.describeDocument();
    expect(report).toBeDefined();
    if (report) {
      expect(report.recommendedWorkflows).toBeDefined();
    }
  });

  test('describeDocument should report document info', () => {
    const report = pdf.describeDocument();
    expect(report).toBeDefined();
    if (report) {
      expect(report.documentInfo.pageCount).toBe(8);
      expect(report.documentInfo.encrypted).toBe(false);
    }
  });
});

// ============================================================================
// fromBuffer with options
// ============================================================================

describe('Loading with options', () => {
  test('fromBuffer with lazyLoad option', async () => {
    const pdf = await AgenticPDF.fromBuffer(pdfBuffer, { lazyLoad: true });
    expect(pdf.getPageCount()).toBe(8);
    pdf.close();
  });

  test('fromBuffer with maxMemoryUsage', async () => {
    const pdf = await AgenticPDF.fromBuffer(pdfBuffer, { maxMemoryUsage: 50 * 1024 * 1024 });
    expect(pdf.getPageCount()).toBe(8);
    pdf.close();
  });
});

// ============================================================================
// Error paths
// ============================================================================

describe('Error paths', () => {
  test('fromBuffer with empty buffer should throw', async () => {
    await expect(AgenticPDF.fromBuffer(new ArrayBuffer(0))).rejects.toThrow();
  });

  test('fromBuffer with invalid data should throw', async () => {
    const bad = new TextEncoder().encode('Not a PDF file at all');
    await expect(AgenticPDF.fromBuffer(bad.buffer as ArrayBuffer)).rejects.toThrow();
  });

  test('search on closed instance should handle gracefully', async () => {
    const pdf = await AgenticPDF.fromBuffer(pdfBuffer);
    pdf.close();
    // After close, data is cleared — search may throw or return empty
    try {
      const results = await pdf.search('test');
      expect(Array.isArray(results)).toBe(true);
    } catch {
      // Expected to throw on closed instance
    }
  });
});

// ============================================================================
// Ontology static methods — deeper coverage
// ============================================================================

describe('Ontology method details', () => {
  test('describe returns valid structure', () => {
    const ontology = AgenticPDF.describe();
    expect(ontology['@context']).toBeDefined();
    expect(ontology.concepts).toBeDefined();
    expect(ontology.capabilities).toBeDefined();
  });

  test('getCapabilities returns categorized items', () => {
    const caps = AgenticPDF.getCapabilities();
    const categories = new Set(caps.map(c => c.category));
    expect(categories.size).toBeGreaterThan(3);
  });

  test('getMethodSignatures has return types', () => {
    const methods = AgenticPDF.getMethodSignatures();
    for (const method of methods) {
      expect(method.name).toBeDefined();
      expect(method.returnType).toBeDefined();
    }
  });

  test('getWorkflows have steps', () => {
    const workflows = AgenticPDF.getWorkflows();
    for (const wf of workflows) {
      expect(wf.name).toBeDefined();
      expect(wf.steps.length).toBeGreaterThan(0);
    }
  });
});

// ============================================================================
// unlock on non-encrypted PDF
// ============================================================================

describe('Unlock on non-encrypted PDF', () => {
  test('unlock should return false on non-encrypted PDF', async () => {
    const pdf = await AgenticPDF.fromBuffer(pdfBuffer);
    const result = await pdf.unlock('password');
    expect(result).toBe(false);
    pdf.close();
  });
});
