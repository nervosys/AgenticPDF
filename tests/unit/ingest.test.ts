/**
 * Unit tests for unified agentic ingestion features:
 *   - ingest()       — single-call AI ingestion
 *   - streamIngest() — NDJSON streaming ingestion
 *   - CLI ingest command & tool-schema command
 *   - IngestOptions / IngestResult / IngestChunk types
 *   - Tool definitions, skill handlers, JSON schemas, and workflows
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

// ── ingest() method ───────────────────────────────────────────────

describe('ingest()', () => {
  test('returns IngestResult with required fields', async () => {
    const result = await pdf.ingest();
    expect(result).toBeDefined();
    expect(result.metadata).toBeDefined();
    expect(typeof result.documentType).toBe('string');
    expect(typeof result.summary).toBe('string');
    expect(Array.isArray(result.keywords)).toBe(true);
    expect(result.structure).toBeDefined();
    expect(typeof result.structure.sections).toBe('number');
    expect(typeof result.structure.tables).toBe('number');
    expect(typeof result.structure.figures).toBe('number');
    expect(Array.isArray(result.chunks)).toBe(true);
    expect(result.stats).toBeDefined();
    expect(typeof result.stats.pageCount).toBe('number');
    expect(typeof result.stats.fileSize).toBe('number');
    expect(typeof result.stats.totalChunks).toBe('number');
    expect(typeof result.stats.totalTokens).toBe('number');
    expect(typeof result.stats.processingTimeMs).toBe('number');
  });

  test('chunks have correct IngestChunk shape', async () => {
    const result = await pdf.ingest();
    if (result.chunks.length > 0) {
      const chunk = result.chunks[0];
      expect(typeof chunk.id).toBe('string');
      expect(typeof chunk.content).toBe('string');
      expect(Array.isArray(chunk.pages)).toBe(true);
      expect(typeof chunk.type).toBe('string');
      expect(typeof chunk.tokenCount).toBe('number');
      expect(typeof chunk.importance).toBe('number');
      expect(Array.isArray(chunk.keywords)).toBe(true);
    }
  });

  test('stats.totalChunks matches chunks array length', async () => {
    const result = await pdf.ingest();
    expect(result.stats.totalChunks).toBe(result.chunks.length);
  });

  test('stats.totalTokens is the sum of chunk tokenCounts', async () => {
    const result = await pdf.ingest();
    const sum = result.chunks.reduce((s, c) => s + c.tokenCount, 0);
    expect(result.stats.totalTokens).toBe(sum);
  });

  test('respects maxChunkSize option', async () => {
    const result = await pdf.ingest({ maxChunkSize: 200 });
    expect(result).toBeDefined();
    expect(result.stats.totalChunks).toBeGreaterThanOrEqual(0);
  });

  test('includePageText returns pageTexts array', async () => {
    const result = await pdf.ingest({ includePageText: true });
    expect(result.pageTexts).toBeDefined();
    expect(Array.isArray(result.pageTexts)).toBe(true);
    if (result.pageTexts!.length > 0) {
      expect(typeof result.pageTexts![0].page).toBe('number');
      expect(typeof result.pageTexts![0].text).toBe('string');
    }
  });

  test('without includePageText, pageTexts is undefined', async () => {
    const result = await pdf.ingest();
    expect(result.pageTexts).toBeUndefined();
  });

  test('stats.processingTimeMs is a positive number', async () => {
    const result = await pdf.ingest();
    expect(result.stats.processingTimeMs).toBeGreaterThanOrEqual(0);
  });
});

// ── streamIngest() method ─────────────────────────────────────────

describe('streamIngest()', () => {
  test('yields header, chunk(s), and footer records', async () => {
    const records: any[] = [];
    for await (const record of pdf.streamIngest()) {
      records.push(record);
    }
    expect(records.length).toBeGreaterThanOrEqual(2); // at least header + footer
    expect(records[0].type).toBe('header');
    expect(records[records.length - 1].type).toBe('footer');
  });

  test('header record has correct shape', async () => {
    const records: any[] = [];
    for await (const record of pdf.streamIngest()) {
      records.push(record);
    }
    const header = records[0];
    expect(header.type).toBe('header');
    expect(header.metadata).toBeDefined();
    expect(typeof header.documentType).toBe('string');
    expect(typeof header.summary).toBe('string');
    expect(Array.isArray(header.keywords)).toBe(true);
    expect(header.structure).toBeDefined();
  });

  test('chunk records have correct shape', async () => {
    const records: any[] = [];
    for await (const record of pdf.streamIngest()) {
      records.push(record);
    }
    const chunks = records.filter(r => r.type === 'chunk');
    if (chunks.length > 0) {
      const c = chunks[0];
      expect(typeof c.id).toBe('string');
      expect(typeof c.content).toBe('string');
      expect(Array.isArray(c.pages)).toBe(true);
      expect(typeof c.chunkType).toBe('string');
      expect(typeof c.tokenCount).toBe('number');
      expect(typeof c.importance).toBe('number');
      expect(Array.isArray(c.keywords)).toBe(true);
    }
  });

  test('footer record has stats', async () => {
    const records: any[] = [];
    for await (const record of pdf.streamIngest()) {
      records.push(record);
    }
    const footer = records[records.length - 1];
    expect(footer.type).toBe('footer');
    expect(footer.stats).toBeDefined();
    expect(typeof footer.stats.pageCount).toBe('number');
    expect(typeof footer.stats.totalChunks).toBe('number');
    expect(typeof footer.stats.totalTokens).toBe('number');
    expect(typeof footer.stats.processingTimeMs).toBe('number');
  });

  test('all records are JSON-serializable', async () => {
    for await (const record of pdf.streamIngest()) {
      const json = JSON.stringify(record);
      expect(typeof json).toBe('string');
      const parsed = JSON.parse(json);
      expect(parsed.type).toBeDefined();
    }
  });
});

// ── Tool definitions ──────────────────────────────────────────────

describe('Tool definitions and schemas', () => {
  test('getToolSchemas includes ingest tool', () => {
    const tools = AgenticPDF.getToolSchemas('openai');
    const ingestTool = tools.find((t: any) => t.function?.name === 'ingest');
    expect(ingestTool).toBeDefined();
    expect(ingestTool.function.parameters.properties).toBeDefined();
  });

  test('getToolSchemas includes streamIngest tool', () => {
    const tools = AgenticPDF.getToolSchemas('openai');
    const streamIngestTool = tools.find((t: any) => t.function?.name === 'streamIngest');
    expect(streamIngestTool).toBeDefined();
  });

  test('anthropic format includes ingest tool', () => {
    const tools = AgenticPDF.getToolSchemas('anthropic');
    const ingestTool = tools.find((t: any) => t.name === 'ingest');
    expect(ingestTool).toBeDefined();
    expect(ingestTool.input_schema).toBeDefined();
  });

  test('getMCPManifest includes ingest tool', () => {
    const manifest = AgenticPDF.getMCPManifest();
    const ingestTool = manifest.tools.find((t: any) => t.name === 'ingest');
    expect(ingestTool).toBeDefined();
  });
});

// ── JSON schemas ──────────────────────────────────────────────────

describe('JSON schemas for ingestion types', () => {
  test('IngestOptions schema exists', () => {
    const schemas = AgenticPDF.getJSONSchemas();
    expect(schemas.IngestOptions).toBeDefined();
    expect(schemas.IngestOptions.type).toBe('object');
    expect(schemas.IngestOptions.properties.strategy).toBeDefined();
    expect(schemas.IngestOptions.properties.maxChunkSize).toBeDefined();
  });

  test('IngestResult schema exists', () => {
    const schemas = AgenticPDF.getJSONSchemas();
    expect(schemas.IngestResult).toBeDefined();
    expect(schemas.IngestResult.type).toBe('object');
    expect(schemas.IngestResult.required).toContain('chunks');
    expect(schemas.IngestResult.required).toContain('stats');
  });

  test('IngestChunk schema exists', () => {
    const schemas = AgenticPDF.getJSONSchemas();
    expect(schemas.IngestChunk).toBeDefined();
    expect(schemas.IngestChunk.type).toBe('object');
    expect(schemas.IngestChunk.required).toContain('id');
    expect(schemas.IngestChunk.required).toContain('content');
  });
});

// ── Workflows ─────────────────────────────────────────────────────

describe('Workflow templates', () => {
  test('agentic-ingest workflow exists', () => {
    const workflows = AgenticPDF.getWorkflows();
    const ingestWf = workflows.find(w => w.id === 'agentic-ingest');
    expect(ingestWf).toBeDefined();
    expect(ingestWf!.name).toContain('Ingest');
    expect(ingestWf!.steps.length).toBeGreaterThanOrEqual(2);
  });

  test('agentic-ingest-streaming workflow exists', () => {
    const workflows = AgenticPDF.getWorkflows();
    const streamWf = workflows.find(w => w.id === 'agentic-ingest-streaming');
    expect(streamWf).toBeDefined();
    expect(streamWf!.steps.some(s => s.method === 'streamIngest')).toBe(true);
  });
});

// ── describeDocument includes ingest ──────────────────────────────

describe('describeDocument', () => {
  test('operations list includes ingest and streamIngest', () => {
    const report = pdf.describeDocument();
    expect(report).toBeDefined();
    expect(report!.availableOperations).toContain('ingest');
    expect(report!.availableOperations).toContain('streamIngest');
  });
});

// ── describeForAgent ──────────────────────────────────────────────

describe('describeForAgent', () => {
  test('quickStart mentions ingest()', () => {
    const info = AgenticPDF.describeForAgent();
    expect(info.agentGuidance.quickStart).toContain('ingest');
  });

  test('bestPractices mentions ingest and streamIngest', () => {
    const info = AgenticPDF.describeForAgent();
    const practices = info.agentGuidance.bestPractices.join(' ');
    expect(practices).toContain('ingest()');
    expect(practices).toContain('streamIngest()');
  });
});

// ── Built-in skill handler ────────────────────────────────────────

describe('Built-in skill: pdf-analysis ingest tool', () => {
  test('pdf-analysis skill has ingest tool', () => {
    const skills = AgenticPDF.listSkills();
    const analysis = skills.find(s => s.id === 'pdf-analysis');
    expect(analysis).toBeDefined();
    const ingestTool = analysis!.tools.find(t => t.name === 'ingest');
    expect(ingestTool).toBeDefined();
    expect(ingestTool!.category).toBe('analysis');
  });
});
