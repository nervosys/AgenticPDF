/**
 * otel.ts — OpenTelemetry instrumentation for AgenticPDF.
 *
 * Initialises the OTEL SDK using standard OTEL_* environment variables
 * (loaded from .env via loadEnv()). Provides a tracer and meter that the
 * Telemetry class uses to emit spans and metrics to any OTLP-compatible
 * collector.
 *
 * If the OTEL packages are not installed or the env vars are absent the
 * module degrades gracefully — all exported helpers become no-ops.
 */

// ── Env-file loader (tiny, zero-dep) ────────────────────────────────────────
function loadEnv(): void {
  try {
    // Only in Node-like runtimes
    if (typeof process === 'undefined' || !process.env) return;
    const fs = require('fs');
    const path = require('path');

    // Walk up from __dirname to find .env (handles dist/ or root)
    let dir = typeof __dirname !== 'undefined' ? __dirname : process.cwd();
    for (let i = 0; i < 5; i++) {
      const candidate = path.join(dir, '.env');
      if (fs.existsSync(candidate)) {
        const content: string = fs.readFileSync(candidate, 'utf8');
        for (const line of content.split(/\r?\n/)) {
          const trimmed = line.trim();
          if (!trimmed || trimmed.startsWith('#')) continue;
          const eqIdx = trimmed.indexOf('=');
          if (eqIdx === -1) continue;
          const key = trimmed.slice(0, eqIdx).trim();
          let value = trimmed.slice(eqIdx + 1).trim();
          // Strip surrounding quotes
          if ((value.startsWith('"') && value.endsWith('"')) ||
              (value.startsWith("'") && value.endsWith("'"))) {
            value = value.slice(1, -1);
          }
          // Only set if not already present (real env takes precedence)
          if (!(key in process.env)) {
            process.env[key] = value;
          }
        }
        break;
      }
      const parent = path.dirname(dir);
      if (parent === dir) break;
      dir = parent;
    }
  } catch {
    // Non-Node environment or permission error — skip silently
  }
}

// Load .env as early as possible so OTEL SDK picks it up.
loadEnv();

// ── Types ───────────────────────────────────────────────────────────────────

/** Minimal subset of OTEL Tracer used by AgenticPDF. */
export interface OtelTracer {
  startActiveSpan<T>(name: string, fn: (span: OtelSpan) => T): T;
}

/** Minimal subset of OTEL Span. */
export interface OtelSpan {
  setAttribute(key: string, value: string | number | boolean): void;
  setStatus(status: { code: number; message?: string }): void;
  end(): void;
}

/** Minimal subset of OTEL Meter. */
export interface OtelMeter {
  createCounter(name: string, options?: { description?: string }): OtelCounter;
  createHistogram(name: string, options?: { description?: string; unit?: string }): OtelHistogram;
}

export interface OtelCounter {
  add(value: number, attributes?: Record<string, string | number | boolean>): void;
}

export interface OtelHistogram {
  record(value: number, attributes?: Record<string, string | number | boolean>): void;
}

// ── Noop implementations ────────────────────────────────────────────────────

const noopSpan: OtelSpan = {
  setAttribute() {},
  setStatus() {},
  end() {},
};

const noopTracer: OtelTracer = {
  startActiveSpan<T>(_name: string, fn: (span: OtelSpan) => T): T {
    return fn(noopSpan);
  },
};

const noopCounter: OtelCounter = { add() {} };
const noopHistogram: OtelHistogram = { record() {} };

const noopMeter: OtelMeter = {
  createCounter(): OtelCounter { return noopCounter; },
  createHistogram(): OtelHistogram { return noopHistogram; },
};

// ── SDK initialisation ──────────────────────────────────────────────────────

let _tracer: OtelTracer = noopTracer;
let _meter: OtelMeter = noopMeter;
let _sdkStarted = false;

/**
 * Best-effort SDK bootstrap. Safe to call multiple times — only the first
 * invocation that finds both OTEL env vars *and* the SDK packages will
 * actually start the pipeline.
 */
export function ensureOtelStarted(): void {
  if (_sdkStarted) return;
  _sdkStarted = true; // Prevent re-entry even if init fails

  try {
    if (typeof process === 'undefined' || !process.env) return;

    const endpoint = process.env.OTEL_EXPORTER_OTLP_ENDPOINT;
    if (!endpoint) return; // No endpoint configured — remain in noop mode

    // Validate endpoint URL
    let parsed: URL;
    try {
      parsed = new URL(endpoint);
    } catch {
      console.warn('[agenticpdf:otel] Invalid OTEL_EXPORTER_OTLP_ENDPOINT — OTEL disabled');
      return;
    }
    if (parsed.protocol !== 'https:' && parsed.protocol !== 'http:') {
      console.warn('[agenticpdf:otel] OTEL endpoint must use http(s) — OTEL disabled');
      return;
    }

    // Dynamic require — allows tree-shaking when OTEL deps are absent
    const { NodeSDK } = require('@opentelemetry/sdk-node');
    const { OTLPTraceExporter } = require('@opentelemetry/exporter-trace-otlp-proto');
    const { OTLPMetricExporter } = require('@opentelemetry/exporter-metrics-otlp-proto');
    const { PeriodicExportingMetricReader } = require('@opentelemetry/sdk-metrics');
    const { Resource } = require('@opentelemetry/resources');
    const { ATTR_SERVICE_NAME, ATTR_SERVICE_VERSION } = require('@opentelemetry/semantic-conventions');
    const otelApi = require('@opentelemetry/api');

    const serviceName = process.env.OTEL_SERVICE_NAME || 'agenticpdf';

    // Build resource
    const resource = new Resource({
      [ATTR_SERVICE_NAME]: serviceName,
      [ATTR_SERVICE_VERSION]: '1.0.0',
      'library.name': 'agenticpdf',
    });

    // Build exporters — the SDK reads OTEL_EXPORTER_OTLP_* env vars automatically
    const traceExporter = new OTLPTraceExporter();
    const metricExporter = new OTLPMetricExporter();
    const metricReader = new PeriodicExportingMetricReader({
      exporter: metricExporter,
      exportIntervalMillis: 30_000,
    });

    const sdk = new NodeSDK({
      resource,
      traceExporter,
      metricReader,
    });

    sdk.start();

    // Grab tracer & meter from the running SDK
    _tracer = otelApi.trace.getTracer('agenticpdf', '1.0.0') as OtelTracer;
    _meter = otelApi.metrics.getMeter('agenticpdf', '1.0.0') as OtelMeter;

    // Graceful shutdown
    if (typeof process.on === 'function') {
      const shutdown = () => { sdk.shutdown().catch(() => {}); };
      process.on('SIGTERM', shutdown);
      process.on('SIGINT', shutdown);
    }
  } catch {
    // OTEL packages not installed or other init error — stay in noop mode
  }
}

// ── Public accessors ────────────────────────────────────────────────────────

/** Return the active OTEL tracer (noop if OTEL is not configured). */
export function getTracer(): OtelTracer {
  ensureOtelStarted();
  return _tracer;
}

/** Return the active OTEL meter (noop if OTEL is not configured). */
export function getMeter(): OtelMeter {
  ensureOtelStarted();
  return _meter;
}

/** True when a real (non-noop) OTEL SDK is running. */
export function isOtelActive(): boolean {
  ensureOtelStarted();
  return _tracer !== noopTracer;
}
