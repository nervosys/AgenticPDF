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
| Tests | 819 Rust, 950 TypeScript |
| Hostile input | 3,739 damage cases and 10 structural attacks, none panicking or exceeding budget |
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

- [ ] **iOS has never been built or run.** The code paths exist; nothing has
      executed them. Needs macOS.
- [ ] Non-PDF formats and the ADF container are covered by their test suites and
      by 111 real-producer files, but not by a corpus at the scale the PDF path
      enjoys.
- [ ] Only two producers, and only on Windows. Nothing here has met a document
      written by Google Docs, Apple Pages, or an older Office than the one
      installed.

**What the real-producer files have been worth so far.** Writing one document
out of Office in every format it supports and diffing the readers against each
other has found **over forty defects** that fixtures written in this repository
could not: a parser panic on a multi-byte character, a spreadsheet number
reported to seventeen digits, several style-inheritance gaps, off-by-one walks
through the legacy `.doc` list definitions, speaker notes attached to the wrong
slide, dates reported as their serial number, an error code reported as zero,
tracked deletions read as text, hidden text and comments read as content, notes
and text boxes dropped entirely, and table styles ignored.

The rule the technique rests on: **a document saved in several formats must read
back the same.** Where two readers differ, at least one is wrong, and no ground
truth is needed to know that.

**Two producers, not one.** Every file above came out of Microsoft Office, so
the comparison was only ever between *formats*. Converting the same documents
with LibreOffice added the missing axis and immediately found nine more
defects, including one that had never been exercised at all: the `.ppt` reader
took the document container from the wrong field of the `UserEditAtom`, and
only ever worked because PowerPoint writes zeros there. A rule written from a
specification and never run against the producer it describes has not been
tested. Calibre and LibreOffice also gave EPUB its first real-producer files,
which found that the reader ignored the stylesheets an EPUB carries inside
itself -- text lost, and a way past the hidden-text scan.

Not every difference is a defect: a converter loses things of its own, and each
one has to be read out of the file before it is attributed. LibreOffice's EPUB
export writes headings as paragraphs and bullets as `<ol>`; Calibre flattens
nested lists. Those are recorded, not fixed.

**A third oracle, needing no second implementation.** Render a document to
Markdown, read that Markdown, render it again: any difference is something the
writer emits and the reader does not understand. Markdown is the form this tool
most often hands to a caller, so a caller reading it back should get the
document rather than an approximation. The check found footnotes and inline
HTML unreadable by our own reader, and two places where the writer emitted
whitespace that re-rendering did not reproduce. 107 of 111 real-producer files
now render identically twice over; the four that do not differ only in blank
lines between adjacent lists, which Markdown cannot keep apart.

Applied to the HTML writer it found five more, and one of them was a plain
reading defect rather than a matter of fidelity: a paragraph holding only a
picture was dropped for having no text in it, so any HTML with a standalone
figure lost the figure — the same rule that had already cost four other
readers theirs, with this one left behind when they were fixed. The others
were footnotes, `<section>` divisions, the `<aside>` beside a slide, a
`<figure>`'s pairing of picture and caption, and an explicit page break: all
written by this tool and none read by it. 110 of 111 files now survive that
round trip.

Applied to the two remaining surfaces it gave one clean result and one defect.
Every one of the 111 documents comes back from **ADF** identical in both
renderings and in its assets, which is the answer an archival format wants;
the check is kept as a test over the corpus rather than over a hand-built
document, since the constructs nobody thinks to hand-build are the ones real
producers write. **JSON** failed on the first document: the block model is an
internally tagged enum, which has nowhere to put a sequence with no name, so a
document containing a quotation could not be serialised at all — and every
consumer of the model as JSON would have hit that.

**A fourth oracle: conservation.** A retrieval pipeline never sees the
document, only the chunks, so a word that falls between two of them cannot be
retrieved, cited or quoted — and nothing downstream can tell it is missing.
Asserting that every word of every corpus document lands in some chunk, at four
chunk sizes, found five defects in the path between the model and the page:
fragments joined with a space that broke `H2O` into `H 2 O` and made it
unfindable, that same join running across a page boundary, a table inside a
cell laid out and then discarded, footnote text on no page at all, and a
picture's alt text — the only words a picture has — in the extracted text and
nowhere else. None was visible from any comparison of readers, because they all
sit downstream of reading.

The same question asked of **search** — every word a document holds must bring
back at least one result — found that the scan walked each section's body
blocks and nothing else. A spreadsheet's sheet names and a deck's slide titles,
the first thing anyone would type, could not be found at all; nor could the
notes beside a slide, nor a footnote's text. A word present and unfindable is
worse than one absent, because the answer comes back confidently empty.

Asked end to end — every line `search` hands over must verify as `matched` at
the locator `search` gave for it — it found the cost of that fix: the new hits
carried a locator that already meant something else, and `verify` does not
answer "I cannot tell". Asked about a slide's title at block zero it found the
first body block recorded there and reported the source as *edited since
import*, which is a claim about the document's history and was false. There is
now one index space for everything quotable, and provenance records all of it:
2,153 citations verify where the body alone offered 479.

**Agreement is not correctness.** A differential between readers is blind to
anything they all get wrong together, and no amount of extra formats or extra
producers can see past that — only new content can. The first fixture built to
provoke it (merged cells across and down, a table inside a cell, a captioned
picture, and run properties that carry meaning) found six more defects, one of
them invisible by construction: every reader read sub- and superscript
correctly and the Markdown writer threw them away, so `H<sub>2</sub>O` came
back as `H2O` in all eight files at once. The others were figures dropped
because the paragraph holding one has no text in it, a merged cell's blank
padded onto the end of its row instead of the column it covers, and a picture's
bytes left behind a header nobody followed.

**Injection scan: white-on-white is caught in `.docx` and HTML, not yet in the
rest.** The question is never "is this white" but "is this the colour of what it
sits on", so a reader has to follow shading — the run, its paragraph, its cell,
and the table style behind that — and then say what it found, including that
there was none and the page is white. `.docx` does; HTML has always required a
document to state both. `.odt`, `.rtf`, `.doc` and `.odp` state nothing, and a
reader that cannot see a fill must not conclude there is none: their colours
are left unjudged rather than guessed at. Each is separate work of the same
shape.

The second half is the harder one and the reason this went slowly. Missing a
payload leaves the commonest injection uncaught; calling a table header an
attack is worse, because a warning nobody can trust is a warning nobody reads.
An early version of the `.docx` rule read an absent background as "the page"
and reported six ordinary documents' headers as injections before that was
caught.

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

The CLI and its JSON-LD ontology are now pinned to each other by
`tests/agent_surface.rs`: the ontology must list exactly the commands the binary
has and exactly the formats the build reads, and a command claiming to take any
document must actually take one. An agent has no way to tell a stale description
from an accurate one, so the description is tested rather than maintained.


- [ ] Table reconstruction beyond bordered tables.
- [ ] Figure and caption linking.
- [ ] Optional OCR behind a feature flag, keeping the default build pure Rust
      with no heavy ML dependency.
- [ ] Prompt-injection filtering of hidden and off-page text — the scan exists;
      the policy around it does not.
- [ ] A tighter bound for untrusted input. The pipeline is bounded — a ZIP
      member is capped at 128 MB, the archive at 512 MB declared, and the
      typesetter at 20,000 pages — but those ceilings together still let a small
      hostile upload buy tens of seconds of CPU. Lowering them would refuse
      legitimate large documents, so the useful change is to make them
      configurable rather than to guess at new numbers.

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
      gh api -X PUT repos/nervosys/AgenticPDF/branches/master/protection \
        -F required_status_checks=null \
        -F enforce_admins=false \
        -F required_pull_request_reviews=null \
        -F restrictions=null \
        -F allow_force_pushes=false \
        -F allow_deletions=false
      ```
- [ ] **Is a FIPS-capable build a product requirement?** Assessed in
      [`docs/development/FIPS_ASSESSMENT.md`](docs/development/FIPS_ASSESSMENT.md):
      only two primitives have to move, but a validated module is a native
      dependency that does not build for WASM — so it costs the footprint and
      reach this project positions itself on. The decision is whether a buyer
      is asking.

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
