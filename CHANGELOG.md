# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

#### ADF — the Agentic Document Format (`agenticpdf-rs/src/adf/`)

The engine's own binary format, and the only one it writes as well as reads.
Designed for retrieval and agent editing rather than for a printer:

- **Seek, don't scan.** A 64-byte header and a fixed-stride chunk table; a
  section, page, asset or embedding is found by offset. Opening a large
  document to answer a question about one page touches three chunks.
- **The index travels with the document.** Retrieval chunks, an inverted term
  index and optional embeddings live *in* the file, so a document is
  searchable the moment it opens and the index cannot drift from its content.
- **Provenance.** Every imported block keeps a content hash and its source
  document, page and bounding box. A quotation is reported as matching,
  drifted, or unrecorded — never guessed.
- **Append-only CRDT edit log.** Concurrent human and agent edits merge by set
  union (block granularity, RGA ordering, last-writer-wins by Lamport clock).
  Every operation records its author, including whether it was a model.
  Appending an edit to a large document writes the edit, not the document.

Detection, `formats::parse` and the CLI all accept `.adf`; `apdf convert <file>
--to adf --output out.adf` writes it.

#### The reader app (`agenticpdf-rs/apps/reader/`)

An agentic-first document reader and editor built on
[Dewey](https://github.com/nervosys/Dewey), NERVOSYS's Rust GUI framework.

- Reads all 17 formats; edits and round-trips ADF; exports Markdown, HTML, text.
- **One command vocabulary.** The desktop UI, the browser, Android and any agent
  all call the same 12 actions through one `Session`. A capability cannot exist
  for one caller and not the others.
- **Agent surface is an ontology**, not a chat box: Dewey's `OntologyRegistry`
  plus `execute_action`, discoverable with `apdf-reader --capabilities`.
- **Platforms.** Desktop (egui) and Android (JNI, three ABIs) run; mobile web
  runs as a wasm bundle; iOS compiles but has not been built or run — that
  needs macOS.
- Page painting is shared: `paint_page` emits through Dewey's `Painter`, and
  each platform either rasterises it directly or replays a recorded form of it.

### Changed

- `Format` gained an `Adf` variant; `apdf convert` gained an `adf` target and
  now writes bytes rather than text.
- `agenticpdf-rs` is now a Cargo workspace, with the library as its root
  package and the app as a member.

### Security

Remediation of the 2026-08-11 audit (CVE/CVSS, MITRE ATT&CK, NIST FIPS 140-3,
CMMC 2.0 Level 2).

- **MCP file access is confined to a set of roots.** The server previously read
  and wrote any path the process could reach while the *model* chose that path,
  and a document can carry text arguing for a particular one — a confused deputy
  holding its operator's privileges. `apdf text <any file>` returned the bytes
  verbatim (ATT&CK **T1005**) and `convert --output` silently overwrote an
  existing file (**T1565.001**). The default root is now the working directory
  the operator chose to serve from; `APDF_MCP_ROOTS` sets the list and `*`
  disables confinement deliberately. Enforcement is by canonicalization, so
  `..` traversal and symlinks planted inside a root are both refused. The CLI
  is deliberately unchanged — a person running it already has a shell.
- **ADF provenance and integrity now use SHA-256 instead of FNV-1a.** FNV has
  no collision or preimage resistance, so a fabricated citation could be made
  to report as `Matches`; provenance rows carry a full 32-byte digest and the
  stride grows 48 → 64 bytes (`VERSION_MINOR` 0 → 1, no migration — ADF is
  unreleased). FNV-1a was deleted rather than kept for cheap cases.
- **Production dependency CVEs cleared: 26 → 0**, including the critical
  `protobufjs` arbitrary code execution (GHSA-xq3m-2v4x-88gg), by upgrading the
  OpenTelemetry SDK from 0.57/1.30 to 0.221/2.10. SDK 2.x replaced the
  `Resource` class with `resourceFromAttributes`; both spellings are accepted,
  and a failed OTEL start now warns instead of silently falling back to no-op,
  which is how such a break disables audit records unnoticed.
- **CI hardening.** Re-enabled the three security workflows, which had been
  `disabled_inactivity` since 2026-08-04, leaving CodeQL, Trivy, Semgrep,
  gitleaks and TruffleHog configured but not running; fixed the TruffleHog step
  that made every one of those runs fail (base and head were both `master` on a
  push); pinned `trivy-action@master` and `trufflehog@main` to commit SHAs; and
  added `permissions: contents: read` to `ci.yml` and `rust.yml`.

### Fixed

- **The `apdf` / `agenticpdf` CLI could not start when installed.** `cli.js`
  launches `cli.ts` through `tsx`, but `tsx` was declared nowhere in
  `package.json` — it worked only where an extraneous copy happened to be
  present. It is now a real dependency. `cli.js` also resolves it through
  Node's module resolution instead of a hardcoded
  `<pkg>/node_modules/tsx/dist/cli.mjs`, which missed whenever npm hoisted the
  dependency to the top level, i.e. in every real installation.
- The CLI integration suites had failed on CI since 2026-06-04 with
  `spawn tsx ENOENT` for the same reason: `npm ci` installs from the manifest,
  so it pruned the undeclared tsx.
- Two tests failed on Node 18, which the above had masked. `File` only became
  a global in Node 20, so the test mock now takes it from `node:buffer`
  (exported there since 18.13) when the global is absent. The library is
  unaffected — it uses `File` only as a type.

### Upstream (Dewey)

Three additions, all as default trait methods so existing backends are
unaffected: `Painter::fill_path` / `stroke_path` / `draw_image` (with an
`ImageData` type), and `Model::execute_action`, which lets an agent *act* on an
application rather than only inspect it.



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
