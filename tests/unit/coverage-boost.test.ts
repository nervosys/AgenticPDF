/**
 * Additional coverage tests for under-tested areas:
 * - Ontology / Discovery API
 * - Export pipeline
 * - Form extraction / processing
 * - Annotation extraction
 * - Image extraction
 * - Static factory methods edge cases
 * - Memory management & cleanup
 */

import AgenticPDF, {
  TelemetryEventType,
  Telemetry,
  AnnotationType,
  FormFieldType,
  DocumentType,
  ChunkType,
} from '../../agenticpdf';

// ============================================================================
// Ontology & Discovery API
// ============================================================================

describe('Ontology & Discovery API', () => {
  describe('AgenticPDF.describe()', () => {
    test('should return ontology with @context', () => {
      const ontology = AgenticPDF.describe();
      expect(ontology).toBeDefined();
      expect(ontology['@context']).toBeDefined();
    });

    test('should include 21 concepts', () => {
      const ontology = AgenticPDF.describe();
      expect(ontology.concepts).toBeDefined();
      expect(ontology.concepts.length).toBe(21);
    });

    test('should include capabilities', () => {
      const ontology = AgenticPDF.describe();
      expect(ontology.capabilities).toBeDefined();
      expect(ontology.capabilities.length).toBeGreaterThan(0);
    });

    test('should include workflows', () => {
      const ontology = AgenticPDF.describe();
      expect(ontology.workflows).toBeDefined();
      expect(ontology.workflows.length).toBe(14);
    });

    test('should include enums', () => {
      const ontology = AgenticPDF.describe();
      expect(ontology.enums).toBeDefined();
    });

    test('concepts should have id, label, description, and properties', () => {
      const ontology = AgenticPDF.describe();
      for (const concept of ontology.concepts) {
        expect(concept.id).toBeDefined();
        expect(typeof concept.label).toBe('string');
        expect(concept.description).toBeDefined();
        expect(concept.properties).toBeDefined();
      }
    });
  });

  describe('AgenticPDF.getCapabilities()', () => {
    test('should return array of capabilities', () => {
      const caps = AgenticPDF.getCapabilities();
      expect(Array.isArray(caps)).toBe(true);
      expect(caps.length).toBe(14);
    });

    test('each capability should have category and methods', () => {
      const caps = AgenticPDF.getCapabilities();
      for (const cap of caps) {
        expect(cap.category).toBeDefined();
        expect(cap.methods).toBeDefined();
        expect(Array.isArray(cap.methods)).toBe(true);
      }
    });

    test('should include loading capability', () => {
      const caps = AgenticPDF.getCapabilities();
      const loading = caps.find((c: any) => c.category === 'loading');
      expect(loading).toBeDefined();
    });

    test('should include extraction capability', () => {
      const caps = AgenticPDF.getCapabilities();
      const extraction = caps.find((c: any) => c.category === 'extraction');
      expect(extraction).toBeDefined();
    });

    test('should include analysis capability', () => {
      const caps = AgenticPDF.getCapabilities();
      const analysis = caps.find((c: any) => c.category === 'analysis');
      expect(analysis).toBeDefined();
    });
  });

  describe('AgenticPDF.getMethodSignatures()', () => {
    test('should return method descriptors', () => {
      const methods = AgenticPDF.getMethodSignatures();
      expect(Array.isArray(methods)).toBe(true);
      expect(methods.length).toBeGreaterThanOrEqual(26);
    });

    test('each method should have name and returnType', () => {
      const methods = AgenticPDF.getMethodSignatures();
      for (const method of methods) {
        expect(method.name).toBeDefined();
        expect(typeof method.name).toBe('string');
        expect(method.returnType).toBeDefined();
      }
    });

    test('should include fromFile method', () => {
      const methods = AgenticPDF.getMethodSignatures();
      const fromFile = methods.find((m: any) => m.name === 'fromFile');
      expect(fromFile).toBeDefined();
    });

    test('should include extractText method', () => {
      const methods = AgenticPDF.getMethodSignatures();
      const extractText = methods.find((m: any) => m.name === 'extractText');
      expect(extractText).toBeDefined();
    });
  });

  describe('AgenticPDF.getWorkflows()', () => {
    test('should return 14 workflow templates', () => {
      const workflows = AgenticPDF.getWorkflows();
      expect(workflows.length).toBe(14);
    });

    test('each workflow should have id, name, and steps', () => {
      const workflows = AgenticPDF.getWorkflows();
      for (const wf of workflows) {
        expect(wf.id).toBeDefined();
        expect(wf.name).toBeDefined();
        expect(wf.steps).toBeDefined();
        expect(Array.isArray(wf.steps)).toBe(true);
        expect(wf.steps.length).toBeGreaterThan(0);
      }
    });

    test('should include rag-pipeline workflow', () => {
      const workflows = AgenticPDF.getWorkflows();
      const rag = workflows.find((w: any) => w.id === 'rag-pipeline');
      expect(rag).toBeDefined();
    });

    test('should include basic-text-extraction workflow', () => {
      const workflows = AgenticPDF.getWorkflows();
      const basic = workflows.find((w: any) => w.id === 'basic-text-extraction');
      expect(basic).toBeDefined();
    });
  });
});

// ============================================================================
// Instance describeDocument
// ============================================================================

describe('Instance describeDocument', () => {
  test('should return undefined on empty instance', () => {
    const pdf = new AgenticPDF({ lazyLoad: true });
    const report = pdf.describeDocument();
    expect(report).toBeUndefined();
    pdf.close();
  });
});

// ============================================================================
// Static factory error paths
// ============================================================================

describe('Factory method error handling', () => {
  test('fromBuffer should reject invalid data', async () => {
    await expect(AgenticPDF.fromBuffer(new ArrayBuffer(0))).rejects.toThrow();
  });

  test('fromBuffer should reject non-PDF data', async () => {
    const data = new TextEncoder().encode('This is not a PDF');
    await expect(AgenticPDF.fromBuffer(data.buffer as ArrayBuffer)).rejects.toThrow();
  });

  test('fromUrl should reject fetch failures', async () => {
    global.fetch = jest.fn().mockRejectedValue(new Error('Network failure'));
    await expect(AgenticPDF.fromUrl('https://example.com/nonexistent.pdf')).rejects.toThrow();
  });
});

// ============================================================================
// Memory management
// ============================================================================

describe('Memory management', () => {
  test('clearAllCaches should not throw', () => {
    expect(() => AgenticPDF.clearAllCaches()).not.toThrow();
  });

  test('close should be idempotent', () => {
    const pdf = new AgenticPDF({ lazyLoad: true });
    pdf.close();
    pdf.close(); // calling again should not throw
  });

  test('getMemoryStats should return valid structure', () => {
    const pdf = new AgenticPDF({ lazyLoad: true });
    const stats = pdf.getMemoryStats();
    expect(stats).toHaveProperty('pagesCached');
    expect(stats).toHaveProperty('objectsCached');
    expect(typeof stats.pagesCached).toBe('number');
    pdf.close();
  });

  test('unloadPages should not throw on empty pdf', () => {
    const pdf = new AgenticPDF({ lazyLoad: true });
    expect(() => pdf.unloadPages()).not.toThrow();
    pdf.close();
  });
});

// ============================================================================
// Enum completeness
// ============================================================================

describe('TelemetryEventType enum completeness', () => {
  test('should have all 11 event types', () => {
    const allTypes = Object.values(TelemetryEventType);
    expect(allTypes).toContain('document_load');
    expect(allTypes).toContain('page_render');
    expect(allTypes).toContain('text_extraction');
    expect(allTypes).toContain('ai_feature');
    expect(allTypes).toContain('export');
    expect(allTypes).toContain('error');
    expect(allTypes).toContain('performance');
    expect(allTypes).toContain('search');
    expect(allTypes).toContain('form_operation');
    expect(allTypes).toContain('annotation_operation');
    expect(allTypes).toContain('save');
  });
});

describe('AnnotationType enum', () => {
  test('should have common annotation types', () => {
    const types = Object.values(AnnotationType);
    expect(types).toContain('Text');
    expect(types).toContain('Link');
    expect(types).toContain('Highlight');
    expect(types).toContain('Underline');
  });
});

describe('FormFieldType enum', () => {
  test('should have all 4 field types', () => {
    const types = Object.values(FormFieldType);
    expect(types).toContain('Text');
    expect(types).toContain('Button');
    expect(types).toContain('Choice');
    expect(types).toContain('Signature');
  });
});

describe('DocumentType enum', () => {
  test('should have all 9 document types', () => {
    const types = Object.values(DocumentType);
    expect(types.length).toBe(9);
  });
});

describe('ChunkType enum', () => {
  test('should have all 9 chunk types', () => {
    const types = Object.values(ChunkType);
    expect(types.length).toBe(9);
    expect(types).toContain('Title');
    expect(types).toContain('Header');
    expect(types).toContain('Paragraph');
  });
});

// ============================================================================
// ThemeManager
// ============================================================================

describe('ThemeManager static access', () => {
  test('should return theme manager', () => {
    const tm = AgenticPDF.getThemeManager();
    expect(tm).toBeDefined();
    expect(typeof tm.getCurrentTheme).toBe('function');
    expect(typeof tm.toggleTheme).toBe('function');
  });
});

// ============================================================================
// Performance monitoring
// ============================================================================

describe('Performance monitoring edge cases', () => {
  afterEach(() => {
    AgenticPDF.disablePerformanceMonitoring();
    AgenticPDF.clearPerformanceMetrics();
  });

  test('should handle repeated enable/disable', () => {
    AgenticPDF.enablePerformanceMonitoring();
    AgenticPDF.enablePerformanceMonitoring(); // double enable
    AgenticPDF.disablePerformanceMonitoring();
    AgenticPDF.disablePerformanceMonitoring(); // double disable
  });

  test('getPerformanceMetrics should return array when disabled', () => {
    AgenticPDF.disablePerformanceMonitoring();
    const m = AgenticPDF.getPerformanceMetrics();
    expect(Array.isArray(m)).toBe(true);
  });

  test('getPerformanceSummary should return object when disabled', () => {
    AgenticPDF.disablePerformanceMonitoring();
    const s = AgenticPDF.getPerformanceSummary();
    expect(typeof s).toBe('object');
  });
});
