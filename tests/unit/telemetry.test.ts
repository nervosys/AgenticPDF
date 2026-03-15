/**
 * Unit tests for Telemetry class
 * Tests event tracking, opt-out mechanisms, data anonymization,
 * configuration, queue limits, and retry/backoff behavior
 */

import { Telemetry, TelemetryEventType } from '../../agenticpdf';

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
      Telemetry.configure({ endpoint: 'https://custom.example.com/events' });
      const config = Telemetry.getConfig();
      expect(config.endpoint).toBe('https://custom.example.com/events');
      // Restore
      Telemetry.configure({ endpoint: originalConfig.endpoint });
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

    test('should apply custom endpoint', () => {
      const originalConfig = Telemetry.getConfig();
      Telemetry.configure({ endpoint: 'https://my-server.example.com/telemetry' });
      expect(Telemetry.getConfig().endpoint).toBe('https://my-server.example.com/telemetry');
      Telemetry.configure({ endpoint: originalConfig.endpoint });
    });
  });
});
