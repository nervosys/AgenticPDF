# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

#### NS-ONTO-001 conformance (`irondocuments-rs/src/ontology.rs`)

The engine now declares itself under the company capability standard, so what
it does, what a caller must hold to ask, and whether an autonomous caller
should are three stated facts rather than three inferences from a command name.

- **A manifest** declaring 26 capabilities with effect, authority,
  agent-safety, reversibility, refusals and evidence; the concepts they return;
  the three surfaces they are reachable on; and `unmapped` naming what this
  declaration leaves out — `.ods`/`.xls` cell reading, spreadsheet writing, VBA
  execution, the deliberately small formula library, and the argument parser's
  own `help`.
- **Evidence is real here rather than nominal.** NS-ONTO-001 defines evidence
  as the command that re-derives a claim independently, which is exactly what
  `verify` is, so `search` and `chunk` declare it with the command that checks
  their output.
- **The vocabulary is vendored** at `irondocuments-rs/vendor/nervosys-ontology`
  rather than depended on across repositories. NS-ONTO-001 §7.1 makes the copy
  the delivery mechanism: re-vendoring brings new normative clauses in as
  failing tests here, which is the earliest they can be learned without
  watching the hub. A path dependency on a sibling checkout was tried and
  removed — CI checks out this repository alone, so it built on a developer's
  machine and nowhere else. `vendor/` is excluded from the workspace, so
  `--workspace` does not lint or test a copy this project is held to rather
  than maintains.
- **A conformance test** (`tests/ontology.rs`) holding the declaration to the
  running product in both directions: every declared invocation must be
  reachable on the surface it claims, and every entry point the engine serves
  must be declared or named in `unmapped`. It reads the command line from the
  binary's own `--help` and the tools from the MCP tool table, so neither is a
  list maintained beside the code.

### Fixed

- **`sheet` and `recalc` were unreachable over MCP.** They were added to the
  command line and to the `describe` ontology and not to the tool table, which
  is how agents actually reach this engine, so the entire spreadsheet surface
  was invisible to them. Found by the conformance test above on its first run.
- The command line and the MCP server now load a workbook through one function
  (`agent_ops::workbook_of`) rather than two implementations of the same
  question.


#### ADF profiles, and the spreadsheet profile (`irondocuments-rs/src/sheet.rs`)

ADF now says in its header what *kind* of document it holds, so one container
and one extension cover the document types an office suite needs. Format
version 1.2.

- **`Profile` in the header** — `document` (the default, and what every file
  written so far is) or `spreadsheet`. It occupies the word the v1.1 header
  reserved, so existing files read back as `document` and a v1.1 reader opens a
  v1.2 file with the chunks it does not know skipped. An unrecognised profile
  from a future build is carried rather than rejected.
- **A spreadsheet model** — sparse cells sorted by `(row, column)`, each with a
  typed value (number, text, boolean or one of the seven spreadsheet errors),
  the formula behind it *and* the result the source cached, its number format,
  and its hyperlink. Plus merges, frozen panes, column widths and defined
  names.
- **Two chunk kinds** — `Workbook` (the directory: sheet names and defined
  names) and `Sheet` (one worksheet's cells). Listing a workbook reads the
  directory and nothing else; opening one sheet decodes one chunk.
- **`.xlsx` imports as cells**, keeping what the text reader discards: the type
  of every value, the `<f>` element it used to skip, and the number format that
  is the only thing distinguishing a date from its serial number.
- **`irondoc sheet <file>`** — read a spreadsheet as cells, in text or JSON,
  whole or one sheet at a time. Reads `.xlsx` directly and `.adf` under the
  spreadsheet profile. Listed in the `describe` ontology.
- **`convert --to adf` imports a spreadsheet as cells** rather than as a table
  of text, through the CLI and the MCP surface alike.
- **Retrieval and provenance at row granularity** — a search hit names the
  sheet and the row, and `verify` checks a quoted row against what was
  imported, so a figure taken from a spreadsheet is as checkable as a sentence
  quoted from a report.
- **Hidden sheets, rows and columns are kept in the cells and kept out of the
  text.** Dropping them would make a conversion silently lose part of the
  workbook it read; rendering them would surface what the author concealed.

#### Recalculation (`irondocuments-rs/src/calc.rs`)

A formula evaluator, so a stored formula can be checked rather than only
displayed.

- **`irondoc recalc <file>`** reports cells whose stored result no longer
  follows from their own formula. Nothing is written back — overwriting the
  cached values would destroy the evidence that they had drifted. Listed in the
  `describe` ontology.
- **Dependencies decide evaluation order**, not position, so a total above its
  inputs is as correct as one below them.
- Arithmetic with the usual precedence (`^` right-associative), comparisons,
  `&`, percentages, absolute and sheet-qualified references, ranges and defined
  names; functions `SUM`, `PRODUCT`, `AVERAGE`, `MIN`, `MAX`, `COUNT`,
  `COUNTA`, `IF`, `IFERROR`, `AND`, `OR`, `NOT`, `ABS`, `SQRT`, `INT`, `ROUND`,
  `LEN`, `LEFT`, `RIGHT`, `MID`, `UPPER`, `LOWER`, `TRIM`, `CONCAT`, `NA`.
- **Cycles are reported, not resolved**; the cell keeps its cached value. An
  unknown function is `#NAME?` and the rest of the sheet still computes. A
  formula that will not parse is reported rather than swallowed. Nesting past
  64 deep is refused rather than overflowing the stack.
- Numbers compare with a relative tolerance, so a workbook storing `0.3` for
  `0.1 + 0.2` is not reported as drifted.

Not implemented: `.ods` and `.xls` still take the text path rather than
producing cells. Nothing writes `.xlsx` back out; ADF remains the only format
this engine writes. The function library is small by design — anything outside
it answers `#NAME?`.

### Added

#### ADF — the Agentic Document Format (`irondocuments-rs/src/adf/`)

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

Detection, `formats::parse` and the CLI all accept `.adf`; `irondoc convert <file>
--to adf --output out.adf` writes it.

#### The reader app (`irondocuments-rs/apps/reader/`)

An agentic-first document reader and editor built on
[Dewey](https://github.com/nervosys/Dewey), NERVOSYS's Rust GUI framework.

- Reads all 17 formats; edits and round-trips ADF; exports Markdown, HTML, text.
- **One command vocabulary.** The desktop UI, the browser, Android and any agent
  all call the same 12 actions through one `Session`. A capability cannot exist
  for one caller and not the others.
- **Agent surface is an ontology**, not a chat box: Dewey's `OntologyRegistry`
  plus `execute_action`, discoverable with `irondoc-reader --capabilities`.
- **Platforms.** Desktop (egui) and Android (JNI, three ABIs) run; mobile web
  runs as a wasm bundle; iOS compiles but has not been built or run — that
  needs macOS.
- Page painting is shared: `paint_page` emits through Dewey's `Painter`, and
  each platform either rasterises it directly or replays a recorded form of it.

### Changed

- `Format` gained an `Adf` variant; `irondoc convert` gained an `adf` target and
  now writes bytes rather than text.
- `irondocuments-rs` is now a Cargo workspace, with the library as its root
  package and the app as a member.

### Security

Remediation of the 2026-08-11 audit (CVE/CVSS, MITRE ATT&CK, NIST FIPS 140-3,
CMMC 2.0 Level 2).

- **MCP file access is confined to a set of roots.** The server previously read
  and wrote any path the process could reach while the *model* chose that path,
  and a document can carry text arguing for a particular one — a confused deputy
  holding its operator's privileges. `irondoc text <any file>` returned the bytes
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

- **PDF text is now drawn with the document's own fonts.** The renderer used a
  substitute face, whose advances differ from the document's, so runs ran into
  one another and sentences overlapped. `/FontFile` programs are now decoded
  into glyph outlines and rasterised, which is what PDF.js and Okular do and
  the only way the page matches: correct letterforms, real italics, and
  spacing that is the document's rather than an approximation. Fonts that are
  not embedded — the standard fourteen — still fall back to laying the run out
  and fitting it to the width the document reserved.
- **PDFs could not be converted to ADF, and opened as an empty document in the
  reader.** `Document::semantic()` is `None` for PDF by design — it carries
  geometry, not authored structure — and every caller that took that option and
  gave up silently excluded the format the project is named after. `convert
  --to adf` refused it outright, the reader's block view came up empty, and
  in-app search found nothing. `Document::semantic_view()` now derives a
  semantic view from page geometry via the existing `layout` heuristics,
  borrowing where the structure was authored so formats that carry real
  structure never have inference substituted for what the author wrote.
- **Every keyboard shortcut in the reader fired twice.** The backend reports a
  key pressed and released; `handle_event` matched on the code alone, so one
  press of Right advanced two pages.
- The reader showed "Untitled" for documents that name themselves in container
  metadata, such as most PDFs.

- **The `apdf` / `irondocuments` CLI could not start when installed.** `cli.js`
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

#### Core Library (`irondocuments.ts`)
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
- CLI support: `irondoc generate -i paper.pdf -o paper.apdf`

#### Ontology & Agent Discovery
- `IronDocuments.describe()` returns full JSON-LD ontology
- `IronDocuments.getCapabilities()` organized by category
- `IronDocuments.getMethodSignatures()` for code generation
- `IronDocuments.getWorkflows()` — 16 pre-built workflow templates
- Instance-level `pdf.describeDocument()` for loaded documents

#### Unified Agentic Ingestion
- `pdf.ingest(options?)` — single call returns metadata, structure, semantic chunks, and stats
- `pdf.streamIngest(options?)` — streaming NDJSON variant (header → chunks → footer)
- `IronDocuments.describeForAgent(format?)` — full introspection payload (ontology + tools + schemas + guidance)
- `IronDocuments.getToolSchemas(format)` — OpenAI, Anthropic, and generic function-calling schemas
- `IronDocuments.getMCPManifest()` — MCP server manifest for MCP-compatible agents
- `IronDocuments.getJSONSchemas()` — JSON schemas for all library types
- CLI `irondoc ingest` command with `--ndjson`, `--include-text`, `--chunk-size` flags
- CLI `irondoc tool-schema` command with `--tool-schema openai|anthropic|generic|mcp`
- 34 tool definitions, 43 JSON schemas, skill handler for agentic workflows

#### Rust CLI (`irondocuments-rs/`)
- Native `apdf` binary (801 KB release build, opt-level "z", LTO, stripped)
- 10 commands: `text`, `meta`, `annotations`, `outline`, `images`, `chunk`, `all`, `describe`, `info`, `generate`
- `irondoc describe` outputs full JSON-LD ontology (673 lines)
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
- CLI: `apdf` / `irondocuments` commands via npm bin
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
- **Architecture**: Single file (`irondocuments.ts`), optional `otel.ts` module
- **Browser Support**: ES2022+

---

For more details about any release, please see the [GitHub releases page](https://github.com/nervosys/IronDocuments/releases).
