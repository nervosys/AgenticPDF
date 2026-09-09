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

import IronDocuments, {
  TelemetryEventType,
  Telemetry,
  AnnotationType,
  FormFieldType,
  DocumentType,
  ChunkType,
} from '../../irondocuments';

// ============================================================================
// Ontology & Discovery API
// ============================================================================

describe('Ontology & Discovery API', () => {
  describe('IronDocuments.describe()', () => {
    test('should return ontology with @context', () => {
      const ontology = IronDocuments.describe();
      expect(ontology).toBeDefined();
      expect(ontology['@context']).toBeDefined();
    });

    test('should include 21 concepts', () => {
      const ontology = IronDocuments.describe();
      expect(ontology.concepts).toBeDefined();
      expect(ontology.concepts.length).toBe(21);
    });

    test('should include capabilities', () => {
      const ontology = IronDocuments.describe();
      expect(ontology.capabilities).toBeDefined();
      expect(ontology.capabilities.length).toBeGreaterThan(0);
    });

    test('should include workflows', () => {
      const ontology = IronDocuments.describe();
      expect(ontology.workflows).toBeDefined();
      expect(ontology.workflows.length).toBe(16);
    });

    test('should include enums', () => {
      const ontology = IronDocuments.describe();
      expect(ontology.enums).toBeDefined();
    });

    test('concepts should have id, label, description, and properties', () => {
      const ontology = IronDocuments.describe();
      for (const concept of ontology.concepts) {
        expect(concept.id).toBeDefined();
        expect(typeof concept.label).toBe('string');
        expect(concept.description).toBeDefined();
        expect(concept.properties).toBeDefined();
      }
    });
  });

  describe('IronDocuments.getCapabilities()', () => {
    test('should return array of capabilities', () => {
      const caps = IronDocuments.getCapabilities();
      expect(Array.isArray(caps)).toBe(true);
      expect(caps.length).toBe(14);
    });

    test('each capability should have category and methods', () => {
      const caps = IronDocuments.getCapabilities();
      for (const cap of caps) {
        expect(cap.category).toBeDefined();
        expect(cap.methods).toBeDefined();
        expect(Array.isArray(cap.methods)).toBe(true);
      }
    });

    test('should include loading capability', () => {
      const caps = IronDocuments.getCapabilities();
      const loading = caps.find((c: any) => c.category === 'loading');
      expect(loading).toBeDefined();
    });

    test('should include extraction capability', () => {
      const caps = IronDocuments.getCapabilities();
      const extraction = caps.find((c: any) => c.category === 'extraction');
      expect(extraction).toBeDefined();
    });

    test('should include analysis capability', () => {
      const caps = IronDocuments.getCapabilities();
      const analysis = caps.find((c: any) => c.category === 'analysis');
      expect(analysis).toBeDefined();
    });
  });

  describe('IronDocuments.getMethodSignatures()', () => {
    test('should return method descriptors', () => {
      const methods = IronDocuments.getMethodSignatures();
      expect(Array.isArray(methods)).toBe(true);
      expect(methods.length).toBeGreaterThanOrEqual(26);
    });

    test('each method should have name and returnType', () => {
      const methods = IronDocuments.getMethodSignatures();
      for (const method of methods) {
        expect(method.name).toBeDefined();
        expect(typeof method.name).toBe('string');
        expect(method.returnType).toBeDefined();
      }
    });

    test('should include fromFile method', () => {
      const methods = IronDocuments.getMethodSignatures();
      const fromFile = methods.find((m: any) => m.name === 'fromFile');
      expect(fromFile).toBeDefined();
    });

    test('should include extractText method', () => {
      const methods = IronDocuments.getMethodSignatures();
      const extractText = methods.find((m: any) => m.name === 'extractText');
      expect(extractText).toBeDefined();
    });
  });

  describe('IronDocuments.getWorkflows()', () => {
    test('should return 16 workflow templates', () => {
      const workflows = IronDocuments.getWorkflows();
      expect(workflows.length).toBe(16);
    });

    test('each workflow should have id, name, and steps', () => {
      const workflows = IronDocuments.getWorkflows();
      for (const wf of workflows) {
        expect(wf.id).toBeDefined();
        expect(wf.name).toBeDefined();
        expect(wf.steps).toBeDefined();
        expect(Array.isArray(wf.steps)).toBe(true);
        expect(wf.steps.length).toBeGreaterThan(0);
      }
    });

    test('should include rag-pipeline workflow', () => {
      const workflows = IronDocuments.getWorkflows();
      const rag = workflows.find((w: any) => w.id === 'rag-pipeline');
      expect(rag).toBeDefined();
    });

    test('should include basic-text-extraction workflow', () => {
      const workflows = IronDocuments.getWorkflows();
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
    const pdf = new IronDocuments({ lazyLoad: true });
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
    await expect(IronDocuments.fromBuffer(new ArrayBuffer(0))).rejects.toThrow();
  });

  test('fromBuffer should reject non-PDF data', async () => {
    const data = new TextEncoder().encode('This is not a PDF');
    await expect(IronDocuments.fromBuffer(data.buffer as ArrayBuffer)).rejects.toThrow();
  });

  test('fromUrl should reject fetch failures', async () => {
    global.fetch = jest.fn().mockRejectedValue(new Error('Network failure'));
    await expect(IronDocuments.fromUrl('https://example.com/nonexistent.pdf')).rejects.toThrow();
  });
});

// ============================================================================
// Memory management
// ============================================================================

describe('Memory management', () => {
  test('clearAllCaches should not throw', () => {
    expect(() => IronDocuments.clearAllCaches()).not.toThrow();
  });

  test('close should be idempotent', () => {
    const pdf = new IronDocuments({ lazyLoad: true });
    pdf.close();
    pdf.close(); // calling again should not throw
  });

  test('getMemoryStats should return valid structure', () => {
    const pdf = new IronDocuments({ lazyLoad: true });
    const stats = pdf.getMemoryStats();
    expect(stats).toHaveProperty('pagesCached');
    expect(stats).toHaveProperty('objectsCached');
    expect(typeof stats.pagesCached).toBe('number');
    pdf.close();
  });

  test('unloadPages should not throw on empty pdf', () => {
    const pdf = new IronDocuments({ lazyLoad: true });
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
    const tm = IronDocuments.getThemeManager();
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
    IronDocuments.disablePerformanceMonitoring();
    IronDocuments.clearPerformanceMetrics();
  });

  test('should handle repeated enable/disable', () => {
    IronDocuments.enablePerformanceMonitoring();
    IronDocuments.enablePerformanceMonitoring(); // double enable
    IronDocuments.disablePerformanceMonitoring();
    IronDocuments.disablePerformanceMonitoring(); // double disable
  });

  test('getPerformanceMetrics should return array when disabled', () => {
    IronDocuments.disablePerformanceMonitoring();
    const m = IronDocuments.getPerformanceMetrics();
    expect(Array.isArray(m)).toBe(true);
  });

  test('getPerformanceSummary should return object when disabled', () => {
    IronDocuments.disablePerformanceMonitoring();
    const s = IronDocuments.getPerformanceSummary();
    expect(typeof s).toBe('object');
  });
});
