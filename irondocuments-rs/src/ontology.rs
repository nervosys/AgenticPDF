// SPDX-License-Identifier: AGPL-3.0-or-later
//! What this engine can be asked to do, declared under NS-ONTO-001.
//!
//! The `describe` command already tells an agent what the commands *are*. This
//! module states the three things a command surface cannot: what an operation
//! does to state, what a caller must hold to ask for it, and whether an
//! autonomous caller should. NS-ONTO-001 §3 keeps those three independent
//! because every attempt to derive one from another has been wrong.
//!
//! # Why this is written by hand
//!
//! NS-ONTO-001 §2.4 — *declared, then verified, never reflected*. A description
//! generated from the code would always be accurate and never mean anything: it
//! could report that `sheet` exists, never that it changes nothing, never how
//! it declines. So the declaration is written, and `tests/ontology.rs` holds it
//! to the running product in both directions — every declared invocation must
//! be reachable, and every entry point the product serves must be declared or
//! named in [`Manifest::unmapped`].
//!
//! That test exists because the drift it catches is not hypothetical. The
//! `sheet` and `recalc` capabilities were added to the command line and to the
//! `describe` output and not to the MCP tool table, which is how agents
//! actually reach this engine. Three hand-maintained surfaces and nothing that
//! noticed one had been missed.

use nervosys_ontology::capability::{
    Absence, Authority, Capability, Evidence, Invocation, Manifest, Refusal, RefusalStatus,
    Unmapped,
};
use nervosys_ontology::concept::{Concept, Property};
use nervosys_ontology::iri::Iri;
use nervosys_ontology::surface::{Surface, SurfaceKind};
use nervosys_ontology::unit::UnitRef;

/// A field of a returned object.
///
/// What this engine returns is text and structure rather than measurements, so
/// nothing here carries a unit; a property that did would name one.
fn field(name: &'static str, meaning: &'static str) -> Property {
    Property {
        name,
        unit: UnitRef::none(),
        meaning,
        optional: false,
        invariants: Vec::new(),
    }
}

/// The engine's self-description.
pub fn manifest() -> Manifest {
    Manifest {
        schema: 1,
        product: "IronDocuments",
        base: Iri::parse("https://ontology.nervosys.com/entity/irondocuments")
            .expect("the product base is a well-formed IRI"),
        purpose: "Read any document format and answer questions about it in a form an agent can \
                  cite: text, structure, tables, cells, formulas and provenance.",
        principles: vec![
            "A quotation is checkable or it is not offered. Provenance is recorded at import, so \
             `verify` answers matched, drifted or unrecorded — never a guess.",
            "Hidden content is kept and not surfaced. What an author concealed stays in the file \
             and out of every text view.",
            "Formats are adapters. Nothing this engine reads becomes its internal model.",
            "What it cannot do is reported, not approximated. An unknown spreadsheet function is \
             #NAME? and the sheet still computes.",
        ],
        capabilities: capabilities(),
        concepts: concepts(),
        surfaces: surfaces(),
        refusal_statuses: vec![
            RefusalStatus {
                code: 1,
                meaning: "refused: the input is not something this command can answer for",
                retryable: false,
            },
            RefusalStatus {
                code: 2,
                meaning: "the arguments do not parse",
                retryable: false,
            },
        ],
        unmapped: unmapped(),
    }
}

/// The authority every capability requires.
///
/// `Public` is the honest answer, not an unexamined one: this is a local
/// binary with no authentication of its own, and the gate on reading a file is
/// the operating system's. Declaring [`Authority::Unaudited`] would say nobody
/// had established what protects it, which is not the case.
fn local() -> Authority {
    Authority::Public
}

/// A read-only capability reachable from the command line and, where it has
/// one, from an MCP tool of a different name.
fn read(name: &'static str, subcommand: &'static str, purpose: &'static str) -> Capability {
    Capability::cli_read(name, subcommand, purpose).requiring(local())
}

fn capabilities() -> Vec<Capability> {
    vec![
        // -- reading a document ------------------------------------------
        read(
            "document.text",
            "text",
            "Extract the text of any supported document.",
        )
        .returning(&["TextExtraction"])
        .reachable(Invocation::machine("extract_text")),
        read(
            "document.markdown",
            "markdown",
            "Render a document as Markdown, preserving headings, tables and lists.",
        )
        .reachable(Invocation::machine("markdown")),
        read(
            "document.meta",
            "meta",
            "Document metadata: title, author, dates, producer.",
        )
        .returning(&["DocumentMeta"])
        .absent(Absence {
            concept: "title",
            when: "the document carries none. Absent rather than the file name, which is not a \
                   title and would read as one.",
        })
        .reachable(Invocation::machine("metadata")),
        read(
            "document.outline",
            "outline",
            "The document's bookmark or heading tree.",
        )
        .absent(Absence {
            concept: "outline",
            when: "the document has no bookmarks and no heading structure to derive one from.",
        })
        .reachable(Invocation::machine("outline")),
        read(
            "document.structure",
            "structure",
            "The semantic structure tree: sections, blocks and their kinds.",
        )
        .reachable(Invocation::machine("structure")),
        read(
            "document.annotations",
            "annotations",
            "Links, highlights, notes and widget annotations.",
        )
        .reachable(Invocation::machine("annotations")),
        read(
            "document.images",
            "images",
            "Raster images embedded in the document.",
        )
        .reachable(Invocation::machine("images")),
        read(
            "document.figures",
            "figures",
            "Figures with their captions, linked to the text that refers to them.",
        )
        .reachable(Invocation::machine("figures")),
        read(
            "document.tables",
            "table",
            "Reconstructed tables, as Markdown or JSON.",
        )
        .reachable(Invocation::machine("tables")),
        read("document.forms", "forms", "Form fields and their values.")
            .reachable(Invocation::machine("forms")),
        read(
            "document.formula",
            "formula",
            "Mathematical formulas, reconstructed to LaTeX where possible.",
        )
        .reachable(Invocation::machine("formula")),
        read(
            "document.layout",
            "layout",
            "Per-block bounding boxes, so a citation can name a place on a page.",
        )
        .reachable(Invocation::machine("layout")),
        read(
            "document.all",
            "all",
            "Metadata, Markdown, tables, chunks and an injection scan in one call.",
        )
        .reachable(Invocation::machine("extract_all")),
        read(
            "document.html",
            "html",
            "Render a document as an HTML fragment with semantic elements.",
        )
        .on_surface(SurfaceKind::Machine),
        // -- retrieval and provenance ------------------------------------
        read(
            "document.chunk",
            "chunk",
            "Split a document into retrieval chunks at block boundaries, each carrying the \
             section and block it came from.",
        )
        .returning(&["RetrievalChunk"])
        .with_evidence(Evidence {
            records: "the section and block every chunk was taken from, against provenance \
                      recorded at import",
            verify_command: Some(
                "irondoc verify <file> \"<chunk text>\" --section <n> --block <n>",
            ),
        })
        .reachable(Invocation::machine("chunk")),
        read(
            "document.search",
            "search",
            "Search a document. ADF answers from the index inside the file; every other format \
             is scanned.",
        )
        .returning(&["RetrievalChunk"])
        .with_evidence(Evidence {
            records: "the section and block of every hit, so a quotation can be checked rather \
                      than trusted",
            verify_command: Some("irondoc verify <file> \"<hit text>\" --section <n> --block <n>"),
        })
        .reachable(Invocation::machine("search")),
        read(
            "document.verify",
            "verify",
            "Check a quotation against the provenance recorded when the document was imported.",
        )
        .returning(&["Verification"])
        .with_evidence(Evidence {
            records: "a content hash per block, written at import and compared here",
            verify_command: None,
        })
        .absent(Absence {
            concept: "source_page",
            when: "the source format has no pages. Zero means unpaginated, not page zero.",
        })
        .refusing(Refusal {
            when: "the file is not ADF",
            response: "exit 1, `only ADF documents carry provenance`",
            because: "provenance is recorded at import and stored in the file; no other format \
                      carries it, and answering from the text alone would be the confabulation \
                      this command exists to prevent.",
        })
        .reachable(Invocation::machine("verify")),
        // -- spreadsheets -------------------------------------------------
        read(
            "sheet.cells",
            "sheet",
            "Read a spreadsheet as typed cells: the type of every value, the formula behind the \
             computed ones, and the number format that says whether a number is a date.",
        )
        .returning(&["Workbook"])
        .absent(Absence {
            concept: "number_format",
            when: "the cell uses General, which is the absence of a format rather than a format.",
        })
        .refusing(Refusal {
            when: "the file is not a spreadsheet this build can read as cells",
            response: "exit 1, `has no cell model yet; read it as a document instead`",
            because: "a report is not a workbook with one sheet, and returning an empty grid for \
                      one would answer a question the caller did not ask.",
        })
        .reachable(Invocation::machine("sheet")),
        read(
            "sheet.recalc",
            "recalc",
            "Recompute a spreadsheet's formulas and report where the stored result no longer \
             follows from the formula beside it. Nothing is written back.",
        )
        .returning(&["RecalcReport"])
        .refusing(Refusal {
            when: "a cell refers to itself, directly or through others",
            response: "the cycle is listed and the cell keeps its stored value",
            because: "replacing a circular result with a zero would substitute this build's \
                      invention for the source's answer.",
        })
        .reachable(Invocation::machine("recalc")),
        // -- inspection ---------------------------------------------------
        read(
            "document.scan",
            "scan",
            "Detect hidden, off-page and prompt-injection text.",
        )
        .returning(&["ScanReport"])
        .reachable(Invocation::machine("scan_injection")),
        read(
            "document.scanned",
            "scanned",
            "Detect image-dominated pages that carry no extractable text and need OCR.",
        )
        .reachable(Invocation::machine("scanned")),
        read(
            "document.displaylist",
            "displaylist",
            "The GPU display list for a page, typeset from the document's structure.",
        ),
        // -- the engine describing itself ---------------------------------
        read(
            "engine.formats",
            "formats",
            "The formats this build can read.",
        )
        .reachable(Invocation::machine("formats")),
        read(
            "engine.describe",
            "describe",
            "A machine-readable JSON-LD ontology of every command, its parameters and its output \
             schema.",
        ),
        read(
            "engine.info",
            "info",
            "What this build is: version, features compiled in, limits.",
        ),
        // -- the two that are not read-only -------------------------------
        Capability::cli_read(
            "document.convert",
            "convert",
            "Convert a document to ADF, Markdown, HTML, text or JSON. ADF records provenance per \
             block as it imports, and is the only target that can later be searched by index or \
             verified.",
        )
        .mutating(local(), false)
        .refusing(Refusal {
            when: "the target is ADF and no --output path is given",
            response: "exit 1, `ADF is binary; use --output to write it to a file`",
            because: "binary on a terminal corrupts it, and a caller who wanted bytes wanted a \
                      file.",
        })
        .reachable(Invocation::machine("convert")),
        Capability::cli_read(
            "engine.mcp",
            "mcp",
            "Serve the engine's capabilities over the Model Context Protocol on stdio.",
        )
        .on_surface(SurfaceKind::Cli),
    ]
}

fn concepts() -> Vec<Concept> {
    vec![
        Concept {
            name: "RetrievalChunk",
            meaning: "A span of a document an agent may quote, with the place it came from.",
            found_in: "search, chunk",
            properties: vec![
                field(
                    "section",
                    "Index of the section, or the sheet for a workbook.",
                ),
                field(
                    "blocks",
                    "First and last block covered, or the row for a workbook.",
                ),
                field("text", "The quotable text itself."),
            ],
        },
        Concept {
            name: "Verification",
            meaning: "The answer to whether a quotation is what was imported.",
            found_in: "verify",
            properties: vec![field(
                "status",
                "matched, drifted or unrecorded. Never a guess: `unrecorded` means no \
                          provenance was stored, which is a different fact from `drifted`.",
            )],
        },
        Concept {
            name: "Workbook",
            meaning: "Sheets of typed cells, with formulas kept alongside the results the source \
                      cached for them.",
            found_in: "sheet",
            properties: vec![
                field("sheets", "Worksheets, hidden ones included and marked."),
                field("defined_names", "Named references such as Tax_Rate."),
            ],
        },
        Concept {
            name: "RecalcReport",
            meaning: "Where a stored result no longer follows from its own formula.",
            found_in: "recalc",
            properties: vec![
                field(
                    "disagreements",
                    "Cells whose cached value differs from the recomputed one.",
                ),
                field(
                    "cycles",
                    "Cells that refer to themselves. Reported, not resolved.",
                ),
            ],
        },
        Concept {
            name: "ScanReport",
            meaning: "Hidden, off-page and prompt-injection text found in a document.",
            found_in: "scan",
            properties: vec![field(
                "clean",
                "Whether anything was found. False is a finding; true is a check that \
                          ran and found nothing.",
            )],
        },
    ]
}

fn surfaces() -> Vec<Surface> {
    vec![
        Surface {
            id: "cli",
            kind: SurfaceKind::Cli,
            purpose: "The command line, and the primary door for every capability but one.",
            implemented: true,
            not_built: None,
            source: Some("irondocuments-rs/src/main.rs"),
            views: Vec::new(),
        },
        Surface {
            id: "mcp",
            kind: SurfaceKind::Machine,
            purpose: "Model Context Protocol over stdio: how an agent actually reaches this \
                      engine.",
            implemented: true,
            not_built: None,
            source: Some("irondocuments-rs/src/mcp.rs"),
            views: Vec::new(),
        },
        Surface {
            id: "wasm",
            kind: SurfaceKind::Machine,
            purpose: "The engine compiled to WebAssembly, for a browser or an edge runtime.",
            implemented: true,
            not_built: None,
            source: Some("irondocuments-rs/src/wasm.rs"),
            views: Vec::new(),
        },
    ]
}

/// What this declaration leaves out, and why.
///
/// NS-ONTO-001 §2.5: a model that cannot say what it omits invites the reader
/// to assume it is complete. Under §7.4 this list is also what excuses an entry
/// point the product serves and no capability declares.
fn unmapped() -> Vec<Unmapped> {
    vec![
        Unmapped {
            area: "help",
            why: "the argument parser's own subcommand. It answers about the parser, not about \
                  any document, and is not a capability of the engine.",
        },
        Unmapped {
            area: "ocr",
            why: "present only in builds with the `ocr` feature, and it delegates recognition to \
                  a backend this declaration cannot speak for. Declaring it unconditionally \
                  would describe a capability half of all builds do not have.",
        },
        Unmapped {
            area: "spreadsheet writing",
            why: "nothing writes .xlsx or .ods back out. ADF is the only format this engine \
                  writes, and a round trip through it is lossy for anything the cell model does \
                  not carry.",
        },
        Unmapped {
            area: "ods and xls cell model",
            why: "OpenDocument and legacy Excel spreadsheets read as text, not as typed cells. \
                  `sheet` refuses them rather than returning a grid of strings that would look \
                  like the real thing.",
        },
        Unmapped {
            area: "formula function library",
            why: "the evaluator implements the functions a document pipeline meets; anything \
                  else answers #NAME?. The set is deliberately small and is not enumerated here, \
                  because a list that drifts is worse than a stated boundary.",
        },
        Unmapped {
            area: "VBA execution",
            why: "macros are parsed and their presence reported; none is executed, and no \
                  capability offers to.",
        },
    ]
}
