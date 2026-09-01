<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->

# AgenticPDF — Roadmap

> **Last updated:** 2026-09-01
> **Direction:** the Rust crate `agenticpdf-rs/` is the engine; new capability
> goes there.

This file says where the work is going and what it is measured against. What
already shipped is in [`CHANGELOG.md`](CHANGELOG.md); how the render engine is
built and why is in
[`docs/development/RENDERING_ARCHITECTURE.md`](docs/development/RENDERING_ARCHITECTURE.md);
the harnesses, reproduction steps and known traps are in
[`docs/development/HANDOFF.md`](docs/development/HANDOFF.md).

---

## Where it stands

| | |
| --- | --- |
| Render agreement with PDF.js | **681 of 681** comparable pages, across 285 reference sets |
| Document formats read | **17** — PDF, OOXML, legacy Office, OpenDocument, EPUB, HTML, Markdown, CSV, RTF, text, ADF |
| Tests | 754 Rust, 950 TypeScript |
| Hosts | desktop, headless image buffer, browser, Android, iOS *(iOS never built — needs macOS)* |
| Advisories | 0 npm; 2 Rust, both triaged and unreachable from document input |

"Measured" throughout this project means checked against something outside the
repository — a reference renderer, a file a real producer wrote, a running
device. A passing test is not evidence that a page rendered correctly, and the
distinction is kept deliberately.

---

## Direction

**One engine, in Rust.** `agenticpdf-rs/` is the single source of truth. The
24k-line `agenticpdf.ts` is the legacy reference implementation: it is
maintained and shipped, but capability is not added to it, and long term the
npm package should be backed by Rust compiled to WASM.

**The wedge is footprint, reach and agent-native output.** A single static
binary with no runtime, reaching edge, serverless and browser; a document
available as queryable structure rather than only as pixels; JSON-LD ontology
and MCP for agents to discover and drive.

**Rendering is judged against PDF.js, and that is a floor rather than a
ceiling.** Agreement on every page means agreement with one renderer's choices,
including its own approximations. It is a strong practical reference, not ground
truth.

---

## Next

Ordered by evidence rather than by appetite. The counts come from
`what_the_corpus_declines`, which walks a directory of documents and tallies
every construct the engine turns down.

### Measurement gaps

- [ ] **EPUB has never met a file a real producer wrote.** Every other format
      now has one: a workbook and a document were written out of Office in ten
      formats and read back, which immediately found a spreadsheet number bug
      that hand-written fixtures could not. No EPUB producer is installed and no
      `.epub` exists on the development machine, so this one still rests on
      fixtures we wrote ourselves.
- [ ] **iOS has never been built or run.** The code paths exist; nothing has
      executed them. Needs macOS.
- [ ] Non-PDF formats and the ADF container are covered by their test suites and
      by the ten real-producer files, but not by a corpus at the scale the PDF
      path enjoys.

### Render engine

- [ ] Alpha (non-`/Luminosity`) soft masks — **6** occurrences in 304 documents.
- [ ] Strokes inside a masked group — **4**. Fills and images are cut by the
      mask; strokes are not.
- [ ] Text as a clipping path (`Tr` 4–7). The one limit the architecture blocks
      directly: it needs glyph outlines as a clip path, and the painter takes
      only rectangles and polygons.
- [ ] Annotation appearance streams — form fields, stamps and comments are not
      painted. **Counted before building:** across the 282 corpus documents
      there are 39,898 `Link` annotations (no appearance to paint), 176 form
      `Widget`s, and **11 markup annotations in total** — 5 Popup, 3 Highlight,
      1 StrikeOut, 1 Square, 1 FreeText. PDF.js renders form widgets into an
      HTML layer rather than the canvas, so the reference cannot adjudicate
      those either. This is a product decision about what a reader should show,
      not a measurable render defect.

### Agentic surface

- [ ] Table reconstruction beyond bordered tables.
- [ ] Figure and caption linking.
- [ ] Optional OCR behind a feature flag, keeping the default build pure Rust
      with no heavy ML dependency.
- [ ] Prompt-injection filtering of hidden and off-page text — the scan exists;
      the policy around it does not.

---

## Waiting on a decision

These are the owner's, not the engine's.

- [ ] **Rotate the exposed OTEL token.** Verified 2026-09-01: the repository is
      clean. `.env.example` has a single commit and it contains placeholders
      only; no tracked file carries a bearer token or a real collector endpoint.
      The exposure predates the history rewrite — and rewriting history does not
      un-publish what forks, clones and caches already took. Rotation at the
      collector is the only thing that closes it, and nothing in this repository
      can do it.
- [ ] **Branch protection on `master`.** There is none, and the choice of rule
      is the decision — requiring pull requests changes a solo maintainer's
      workflow, while blocking force-pushes and deletions costs nothing. Both
      are one command:

      ```bash
      # Minimal: prevent history rewrites and accidental deletion.
      gh api -X PUT repos/nervosys/AgenticPDF/branches/master/protection         -F required_status_checks=null -F enforce_admins=false         -F required_pull_request_reviews=null -F restrictions=null         -F allow_force_pushes=false -F allow_deletions=false
      ```
- [ ] **Is a FIPS-capable build a product requirement?** Assessed in
      [`docs/development/FIPS_ASSESSMENT.md`](docs/development/FIPS_ASSESSMENT.md):
      only two primitives have to move, but a validated module is a native
      dependency that does not build for WASM — so it costs the footprint and
      reach this project positions itself on. The decision is whether a buyer
      is asking.
- [ ] **How the project presents itself.** `README.md` still describes a
      TypeScript-only, zero-dependency single file and does not mention the Rust
      engine in its features. That is a positioning choice rather than a
      factual error to fix quietly.

---

## Constraints worth knowing

- **`agenticpdf.ts` is one file by design**, with CRLF line endings; use `.cjs`
  helper scripts for programmatic edits, since `package.json` sets
  `"type": "module"`.
- **Real Office fixtures are gitignored** at `agenticpdf-rs/tests/fixtures/`,
  because Office stamps the author's name into every file it writes. Tests that
  use them skip when they are absent — which means an empty directory looks
  exactly like a passing suite. Regenerate them with Office COM automation when
  the format readers change.
- **The render corpus is not in the repository.** It is the maintainer's own
  documents, referred to by kind rather than by name.
