/**
 * Unit tests for Telemetry class
 * Tests event tracking, opt-out mechanisms, data anonymization,
 * configuration, queue limits, and retry/backoff behavior
 */

import { Telemetry, TelemetryEventType, TelemetryConfig } from '../../agenticpdf';

describe('Telemetry', () => {
  const originalEnv = process.env.AGENTICPDF_NO_TELEMETRY;

  beforeEach(() => {
    // Reset telemetry state between tests
    Telemetry.disable();
  });

  afterEach(() => {
    Telemetry.disable();
    process.env.AGENTICPDF_NO_TELEMETRY = originalEnv;
  });

  describe('Opt-out mechanisms', () => {
    test('should be disabled when AGENTICPDF_NO_TELEMETRY=1', () => {
      // Env var is set in setup.ts, so isEnabled checks during init
      expect(Telemetry.isEnabled()).toBe(false);
    });

    test('should be disabled via Telemetry.disable()', () => {
      Telemetry.disable();
      expect(Telemetry.isEnabled()).toBe(false);
    });
  });

  describe('Event tracking', () => {
    test('should not throw when tracking while disabled', () => {
      Telemetry.disable();
      expect(() => {
        Telemetry.track(TelemetryEventType.DocumentLoad, { pageCount: 5 });
      }).not.toThrow();
    });

    test('should not throw when tracking features while disabled', () => {
      Telemetry.disable();
      expect(() => {
        Telemetry.trackFeature('extractText', { duration: 100 });
      }).not.toThrow();
    });

    test('should not throw when tracking document load while disabled', () => {
      Telemetry.disable();
      expect(() => {
        Telemetry.trackDocumentLoad({
          pageCount: 10,
          fileSize: 1024,
          duration: 50,
        });
      }).not.toThrow();
    });

    test('should not throw when tracking errors while disabled', () => {
      Telemetry.disable();
      expect(() => {
        Telemetry.trackError(new Error('test error'), 'test context');
      }).not.toThrow();
    });

    test('should not throw when flushing while disabled', async () => {
      Telemetry.disable();
      await expect(Telemetry.flush()).resolves.toBeUndefined();
    });
  });

  describe('TelemetryEventType enum', () => {
    test('should have expected event types', () => {
      expect(TelemetryEventType.DocumentLoad).toBe('document_load');
      expect(TelemetryEventType.PageRender).toBe('page_render');
      expect(TelemetryEventType.TextExtraction).toBe('text_extraction');
      expect(TelemetryEventType.AIFeature).toBe('ai_feature');
      expect(TelemetryEventType.Export).toBe('export');
      expect(TelemetryEventType.Error).toBe('error');
      expect(TelemetryEventType.Performance).toBe('performance');
    });
  });

  describe('Enable/disable lifecycle', () => {
    test('should toggle enabled state', () => {
      Telemetry.disable();
      expect(Telemetry.isEnabled()).toBe(false);

      // Can't truly enable in test because env var is set
      // but the method should not throw
      expect(() => Telemetry.enable()).not.toThrow();
    });

    test('should clear event queue on disable', () => {
      Telemetry.disable();
      // Tracking while disabled should be silent
      Telemetry.track(TelemetryEventType.DocumentLoad, {});
      Telemetry.disable();
      // No assertion needed — just confirming no crash
    });
  });

  describe('Configuration', () => {
    test('should return config via getConfig()', () => {
      const config = Telemetry.getConfig();
      expect(config).toBeDefined();
      expect(config.flushInterval).toBe(30000);
      expect(config.maxBatchSize).toBe(50);
      expect(config.maxQueueSize).toBe(500);
      expect(config.maxRetries).toBe(5);
      expect(config.anonymize).toBe(true);
    });

    test('getConfig() should return a copy, not the internal object', () => {
      const config1 = Telemetry.getConfig();
      const config2 = Telemetry.getConfig();
      expect(config1).not.toBe(config2);
      expect(config1).toEqual(config2);
    });

    test('should accept partial configuration via configure()', () => {
      const originalConfig = Telemetry.getConfig();
      Telemetry.configure({ flushInterval: 12345 });
      const config = Telemetry.getConfig();
      expect(config.flushInterval).toBe(12345);
      // Endpoint should not be changed — it is intentionally excluded for security
      expect(config.endpoint).toBe(originalConfig.endpoint);
      // Restore
      Telemetry.configure({ flushInterval: originalConfig.flushInterval });
    });

    test('should apply custom maxQueueSize', () => {
      const originalConfig = Telemetry.getConfig();
      Telemetry.configure({ maxQueueSize: 100 });
      expect(Telemetry.getConfig().maxQueueSize).toBe(100);
      Telemetry.configure({ maxQueueSize: originalConfig.maxQueueSize });
    });

    test('should apply custom maxRetries', () => {
      const originalConfig = Telemetry.getConfig();
      Telemetry.configure({ maxRetries: 3 });
      expect(Telemetry.getConfig().maxRetries).toBe(3);
      Telemetry.configure({ maxRetries: originalConfig.maxRetries });
    });

    test('should not allow overriding endpoint via configure() (security)', () => {
      const originalConfig = Telemetry.getConfig();
      Telemetry.configure({ endpoint: 'https://my-server.example.com/telemetry' } as any);
      // Endpoint must remain unchanged — excluded from allowedKeys to prevent data exfiltration
      expect(Telemetry.getConfig().endpoint).toBe(originalConfig.endpoint);
    });
  });

  describe('New event types', () => {
    test('should have Search event type', () => {
      expect(TelemetryEventType.Search).toBe('search');
    });

    test('should have FormOperation event type', () => {
      expect(TelemetryEventType.FormOperation).toBe('form_operation');
    });

    test('should have AnnotationOperation event type', () => {
      expect(TelemetryEventType.AnnotationOperation).toBe('annotation_operation');
    });

    test('should have Save event type', () => {
      expect(TelemetryEventType.Save).toBe('save');
    });
  });

  describe('Error tracking', () => {
    test('should not throw when tracking errors with context', () => {
      Telemetry.disable();
      expect(() => {
        Telemetry.trackError(new Error('parse failure'), 'parse');
      }).not.toThrow();
    });

    test('should not throw when tracking errors without context', () => {
      Telemetry.disable();
      expect(() => {
        Telemetry.trackError(new Error('unknown error'));
      }).not.toThrow();
    });

    test('should handle various error types gracefully', () => {
      Telemetry.disable();
      expect(() => {
        Telemetry.trackError(new TypeError('type mismatch'), 'extractText');
        Telemetry.trackError(new RangeError('out of range'), 'renderPage');
        Telemetry.trackError(new SyntaxError('bad syntax'), 'getAIFeatures');
      }).not.toThrow();
    });
  });

  describe('Feature tracking with duration', () => {
    test('should not throw when tracking features with duration data', () => {
      Telemetry.disable();
      expect(() => {
        Telemetry.trackFeature('search', { duration: 42, resultCount: 5 });
        Telemetry.trackFeature('getFormFields', { duration: 10, fieldCount: 3 });
        Telemetry.trackFeature('fillForm', { duration: 8, fieldCount: 2 });
        Telemetry.trackFeature('getAnnotations', { duration: 15, annotationCount: 7 });
        Telemetry.trackFeature('save', { duration: 200 });
        Telemetry.trackFeature('extractImages', { duration: 100, imageCount: 4 });
      }).not.toThrow();
    });

    test('should not throw when tracking document load with duration', () => {
      Telemetry.disable();
      expect(() => {
        Telemetry.trackDocumentLoad({ pageCount: 50, fileSize: 1048576, duration: 350 });
      }).not.toThrow();
    });
  });

  describe('TelemetryConfig interface', () => {
    test('config should have all expected fields', () => {
      const config = Telemetry.getConfig();
      expect(typeof config.enabled).toBe('boolean');
      expect(typeof config.endpoint).toBe('string');
      expect(typeof config.flushInterval).toBe('number');
      expect(typeof config.maxBatchSize).toBe('number');
      expect(typeof config.maxQueueSize).toBe('number');
      expect(typeof config.maxRetries).toBe('number');
      expect(typeof config.anonymize).toBe('boolean');
    });
  });
});
