# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.0] - 2026-04-01

### Added

#### Core Library (`agenticpdf.ts`)
- **Streaming-First Architecture**: `streamText()`, `streamSemanticChunks()` for memory-efficient processing
- **AI-Native Design**: Semantic chunking, structural analysis, embedding provider interface
- **Canvas Rendering**: PDF-to-canvas with text, images, vector graphics, form XObjects
- **Complete Extraction**: Text, images, forms, annotations, metadata
- **Zero Dependencies**: Single TypeScript file, no runtime deps
- **Web Worker Support**: CPU-intensive operations offloaded to workers
- **Memory Management**: Configurable limits, lazy loading, automatic cleanup
- **Multi-format Export**: Text, HTML, Markdown, JSON, aPDF

#### PretextLayout Engine
- Native multiline text layout (inspired by [pretext](https://github.com/chenglou/pretext))
- Grapheme-aware line breaking via `Intl.Segmenter`
- CJK support with per-character breakable segments
- Canvas/OffscreenCanvas measurement with LRU cache (10K entries)
- Server-side heuristic fallback when canvas is unavailable

#### PDF Writing & Modification
- Incremental save (append-only, preserves signatures)
- Page management: insert, delete, reorder
- Annotation persistence: text, highlight, link annotations
- Digital signature preparation and application
- PDF/A compliance validation and XMP metadata generation

#### AI & RAG Enhancements
- Embedding generator with custom provider interface
- Vector store helper for semantic search indexing
- Document differ for PDF comparison
- Summarization pipeline (extractive, no external services)
- Structured data extraction (invoices, papers, resumes)

#### aPDF Binary Format (v1.1)
- Custom binary container: `%aPDF-1.1` magic, JSON metadata + PDF data
- LZ77 compression with full round-trip fidelity
- Security: 2GB size caps, bounded metadata, `JSON.parse` safety
- CLI support: `apdf generate -i paper.pdf -o paper.apdf`

#### Ontology & Agent Discovery
- `AgenticPDF.describe()` returns full JSON-LD ontology
- `AgenticPDF.getCapabilities()` organized by category
- `AgenticPDF.getMethodSignatures()` for code generation
- `AgenticPDF.getWorkflows()` — 16 pre-built workflow templates
- Instance-level `pdf.describeDocument()` for loaded documents

#### Unified Agentic Ingestion
- `pdf.ingest(options?)` — single call returns metadata, structure, semantic chunks, and stats
- `pdf.streamIngest(options?)` — streaming NDJSON variant (header → chunks → footer)
- `AgenticPDF.describeForAgent(format?)` — full introspection payload (ontology + tools + schemas + guidance)
- `AgenticPDF.getToolSchemas(format)` — OpenAI, Anthropic, and generic function-calling schemas
- `AgenticPDF.getMCPManifest()` — MCP server manifest for MCP-compatible agents
- `AgenticPDF.getJSONSchemas()` — JSON schemas for all library types
- CLI `apdf ingest` command with `--ndjson`, `--include-text`, `--chunk-size` flags
- CLI `apdf tool-schema` command with `--tool-schema openai|anthropic|generic|mcp`
- 34 tool definitions, 43 JSON schemas, skill handler for agentic workflows

#### Rust CLI (`agenticpdf-rs/`)
- Native `apdf` binary (801 KB release build, opt-level "z", LTO, stripped)
- 10 commands: `text`, `meta`, `annotations`, `outline`, `images`, `chunk`, `all`, `describe`, `info`, `generate`
- `apdf describe` outputs full JSON-LD ontology (673 lines)
- Parser: annotation extraction, recursive outline parsing, font name detection
- 10 Rust tests, zero warnings

#### OpenTelemetry Integration
- `@opentelemetry/api` integration in `Telemetry` class
- Lazy resolution — activated only when OTEL packages are present
- Span emission for tracked operations; counter and histogram metrics
- Standalone `otel.ts` module for full SDK bootstrap
- `.env` / `.env.example` for OTEL configuration
- Graceful degradation to no-ops when OTEL is unavailable

#### Website
- Next.js 15.3 + React 19.1 + Tailwind CSS 4.1
- Shiki 4.0 syntax highlighting for all code examples
- Dark/light theme support

#### Theme Toggle & Modern UI
- Built-in dark/light mode for PDF viewers
- Theme persistence via localStorage
- Responsive design with auto-fitting viewers

#### Interactive Demos
- Full PDF viewer (`demos/pdf-viewer.html`)
- Render engine demo with sidebar controls
- Theme toggle showcase
- API explorer with interactive examples

#### Developer Experience
- CLI: `apdf` / `agenticpdf` commands via npm bin
- TypeScript examples in `examples/` (8 scenarios)
- Jest test suite: **950 tests** across 25 suites — all passing
- GitHub Actions CI on Node 18/20/22
- Automated release workflow with npm provenance

### Security

Three comprehensive security audit passes (25+ total fixes):

#### Pass 1 (12 fixes)
- SSRF protocol validation on `fromUrl()`
- Path traversal prevention in file operations
- Replaced `Math.random` with cryptographic PRNG
- XSS sanitization in HTML export
- ReDoS-safe regex patterns
- Prototype pollution protection in object merging
- Bounded streaming (max buffer sizes)
- Recursion depth limits in PDF object parsing
- aPDF metadata size limits
- Error message information disclosure prevention
- Fixed duplicate TypeScript exports (TS2484)

#### Pass 2 (13 findings, 10 code fixes)
- SSRF private IP blocking (RFC 1918, link-local, loopback)
- HTTP redirect validation (limit count, block protocol downgrade)
- Telemetry endpoint exfiltration prevention
- YAML frontmatter injection sanitization
- CSV formula injection prevention in exports
- PKCS#7 padding oracle mitigation (constant-time validation)
- aPDF v1.0 entry size limits
- `JSON.parse` safety wrappers
- Demo DOM XSS fixes (input sanitization)
- Worker URL validation (same-origin, blob/data only)

#### Pass 3 — 4-Framework Audit (CVE, MITRE ATT&CK, NIST FIPS 140-3, CMMC 2.0 Level 2)
- CLI path traversal hardening (`validateOutputPath()` on all write operations)
- `crypto.getRandomValues()` for all ID generation (replaced remaining `Math.random` usage)
- ReDoS guard with 64-char limit on user-supplied regex
- Regex special character escaping for whole-word search
- Security headers in `server.cjs` (`X-Content-Type-Options`, `X-Frame-Options`, `Referrer-Policy`)

### Technical Specifications

- **Node.js**: >= 18.0.0
- **TypeScript**: 5.9.3
- **Tests**: 950 across 25 suites
- **License**: AGPL-3.0-or-later
- **Architecture**: Single file (`agenticpdf.ts`), optional `otel.ts` module
- **Browser Support**: ES2022+

---

For more details about any release, please see the [GitHub releases page](https://github.com/nervosys/AgenticPDF/releases).
