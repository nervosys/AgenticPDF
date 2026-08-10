// SPDX-License-Ientifier: AGPL-3.0-or-later
//! The document session: everything the app does that is not drawing.
//!
//! Opening, navigating, searching, editing and saving live here rather than in
//! the UI model, for two reasons. The obvious one is testability — all of it is
//! exercised headlessly below. The load-bearing one is that the **agent** and
//! the **user interface** must drive exactly the same operations. If an agent
//! could only do what a separate command layer exposed, the two would drift,
//! and the ontology would describe an app that no longer matches the one the
//! user sees. Here there is one implementation and two callers.

use agenticpdf::adf::oplog::{Actor, Change, OpId, OpLog};
use agenticpdf::adf::{AdfDoc, AdfWriter};
use agenticpdf::detect::Format;
use agenticpdf::doc::{Block, SemanticDoc};
use agenticpdf::document::Document;
use agenticpdf::{PdfError, engine::DisplayList};

/// Actor id used for edits made through the user interface.
pub const ACTOR_USER: u64 = 1;
/// Actor id used for edits made by an agent over the protocol.
pub const ACTOR_AGENT: u64 = 2;

/// One search hit, with enough context to navigate and to cite.
#[derive(Debug, Clone, PartialEq)]
pub struct Hit {
    pub section: u32,
    pub block: u32,
    pub page: usize,
    pub text: String,
    /// Similarity for a semantic hit; `None` for an exact term match.
    pub score: Option<f32>,
}

/// An open document and everything the app knows about it.
pub struct Session {
    document: Document,
    /// Original bytes, kept so a save can append to an ADF file rather than
    /// rewriting it, and so re-detection never has to guess.
    source: Vec<u8>,
    log: OpLog,
    /// Highest operation present when the file was opened, so a save writes
    /// only what came after it.
    saved_mark: Option<OpId>,
    page: usize,
    /// Scale applied when rendering pages, as a multiple of natural size.
    zoom: f32,
    dirty: bool,
}

impl Session {
    /// Open a document from bytes.
    pub fn open(bytes: Vec<u8>) -> Result<Session, PdfError> {
        let document = Document::open(&bytes)?;

        // An ADF file brings its own history; anything else is seeded from its
        // blocks so it is editable — and mergeable — from the first keystroke
        // rather than only after a save.
        let log = match document.format() {
            Format::Adf => AdfDoc::open(&bytes)
                .and_then(|adf| adf.oplog())
                .unwrap_or_default(),
            _ => {
                let blocks = document
                    .semantic()
                    .map(top_level_blocks)
                    .unwrap_or_default();
                let mut log = agenticpdf::adf::oplog::from_blocks(&blocks, ACTOR_USER, 0);
                log.register_actor(Actor {
                    id: ACTOR_USER,
                    name: "user".into(),
                    is_agent: false,
                });
                log
            }
        };
        let saved_mark = log.ops().next_back().map(|op| op.id);

        Ok(Session {
            document,
            source: bytes,
            log,
            saved_mark,
            page: 1,
            zoom: 1.0,
            dirty: false,
        })
    }

    // ------------------------------------------------------------------
    // Inspection
    // ------------------------------------------------------------------

    pub fn format(&self) -> Format {
        self.document.format()
    }

    pub fn page_count(&self) -> usize {
        self.document.page_count().max(1)
    }

    pub fn page(&self) -> usize {
        self.page
    }

    pub fn zoom(&self) -> f32 {
        self.zoom
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn title(&self) -> String {
        self.document
            .semantic()
            .and_then(|semantic| semantic.title.clone())
            .unwrap_or_else(|| "Untitled".to_string())
    }

    /// The geometry for the current page, for the canvas to paint.
    pub fn display_list(&self) -> Result<DisplayList, PdfError> {
        self.document.display_list(self.page)
    }

    /// The blocks as the edit log currently has them.
    pub fn blocks(&self) -> Vec<(OpId, Block)> {
        self.log.materialize_with_ids(OpId::ROOT)
    }

    pub fn log(&self) -> &OpLog {
        &self.log
    }

    /// Who last touched a node, and when.
    pub fn attribution(&self, node: OpId) -> Option<(String, bool, u64)> {
        self.log
            .attribution(node)
            .map(|(actor, at)| (actor.name.clone(), actor.is_agent, at))
    }

    // ------------------------------------------------------------------
    // Navigation
    // ------------------------------------------------------------------

    /// Move to `page`, clamped to the document. Returns whether it moved.
    pub fn go_to_page(&mut self, page: usize) -> bool {
        let target = page.clamp(1, self.page_count());
        let moved = target != self.page;
        self.page = target;
        moved
    }

    pub fn next_page(&mut self) -> bool {
        self.go_to_page(self.page + 1)
    }

    pub fn previous_page(&mut self) -> bool {
        self.go_to_page(self.page.saturating_sub(1).max(1))
    }

    /// Set zoom, bounded so the canvas cannot be asked for an absurd surface.
    pub fn set_zoom(&mut self, zoom: f32) {
        self.zoom = zoom.clamp(0.1, 8.0);
    }

    // ------------------------------------------------------------------
    // Search
    // ------------------------------------------------------------------

    /// Search the document.
    ///
    /// ADF answers from the index it carries; every other format is scanned.
    /// The distinction is invisible to the caller on purpose — an agent should
    /// not have to ask what it is holding before it can look something up.
    pub fn search(&self, query: &str) -> Vec<Hit> {
        if query.trim().is_empty() {
            return Vec::new();
        }
        if self.document.format() == Format::Adf
            && let Ok(adf) = AdfDoc::open(&self.source)
            && let Ok(hits) = adf.search(query)
            && !hits.is_empty()
        {
            return hits
                .into_iter()
                .map(|(chunk, text)| Hit {
                    section: chunk.section,
                    block: chunk.blocks[0],
                    page: chunk.page.max(1) as usize,
                    text,
                    score: None,
                })
                .collect();
        }
        self.scan(query)
    }

    /// Semantic search, available when the document carries embeddings.
    pub fn search_similar(&self, query: &[f32], limit: usize) -> Vec<Hit> {
        let Ok(adf) = AdfDoc::open(&self.source) else {
            return Vec::new();
        };
        adf.search_similar(query, limit)
            .unwrap_or_default()
            .into_iter()
            .map(|(chunk, text, score)| Hit {
                section: chunk.section,
                block: chunk.blocks[0],
                page: chunk.page.max(1) as usize,
                text,
                score: Some(score),
            })
            .collect()
    }

    /// Linear fallback: every block whose text contains all the query's terms.
    fn scan(&self, query: &str) -> Vec<Hit> {
        let Some(semantic) = self.document.semantic() else {
            return Vec::new();
        };
        let terms: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();

        let mut hits = Vec::new();
        for (section_index, section) in semantic.sections.iter().enumerate() {
            for (block_index, block) in section.blocks.iter().enumerate() {
                let mut text = String::new();
                agenticpdf::doc::block_text_into(block, &mut text);
                let haystack = text.to_lowercase();
                if terms.iter().all(|term| haystack.contains(term.as_str())) {
                    hits.push(Hit {
                        section: section_index as u32,
                        block: block_index as u32,
                        page: 1,
                        text: text.trim().to_string(),
                        score: None,
                    });
                }
            }
        }
        hits
    }

    // ------------------------------------------------------------------
    // Editing
    // ------------------------------------------------------------------

    /// Insert a block after `after`, or at the start when `after` is `None`.
    pub fn insert_block(&mut self, author: u64, after: Option<OpId>, block: Block) -> OpId {
        self.ensure_actor(author);
        self.dirty = true;
        self.log.push(author, crate::now_millis(), Change::Insert {
            parent: OpId::ROOT,
            left: after,
            block,
        })
    }

    /// Replace a node's content.
    pub fn replace_block(&mut self, author: u64, target: OpId, block: Block) -> OpId {
        self.ensure_actor(author);
        self.dirty = true;
        self.log
            .push(author, crate::now_millis(), Change::Replace { target, block })
    }

    /// Delete a node.
    pub fn delete_block(&mut self, author: u64, target: OpId) -> OpId {
        self.ensure_actor(author);
        self.dirty = true;
        self.log
            .push(author, crate::now_millis(), Change::Delete { target })
    }

    /// Merge edits made elsewhere — another window, another device, an agent
    /// working on a copy. Convergence is the log's property, not this method's.
    pub fn merge(&mut self, other: &OpLog) {
        self.log.merge(other);
        self.dirty = true;
    }

    fn ensure_actor(&mut self, id: u64) {
        if self.log.actor(id).is_none() {
            self.log.register_actor(Actor {
                id,
                name: if id == ACTOR_AGENT {
                    "agent".into()
                } else {
                    "user".into()
                },
                is_agent: id == ACTOR_AGENT,
            });
        }
    }

    // ------------------------------------------------------------------
    // Saving
    // ------------------------------------------------------------------

    /// Serialise to ADF.
    ///
    /// An ADF document that only gained edits is **appended to**; anything else
    /// is written fresh, because it is being converted rather than updated.
    pub fn save(&mut self) -> Result<Vec<u8>, PdfError> {
        let bytes = if self.document.format() == Format::Adf {
            let mut file = self.source.clone();
            agenticpdf::adf::write::append_ops(&mut file, &self.log, self.saved_mark);
            file
        } else {
            let semantic = self.materialized();
            AdfWriter::new()
                .with_oplog(self.log.clone())
                .write(&semantic, self.document.format().id())
        };

        self.saved_mark = self.log.ops().next_back().map(|op| op.id);
        self.dirty = false;
        Ok(bytes)
    }

    /// The document as the edit log currently describes it.
    ///
    /// Metadata and assets come from what was opened; the block content comes
    /// from the log, because the log is the authority once editing starts.
    pub fn materialized(&self) -> SemanticDoc {
        let mut semantic = self.document.semantic().cloned().unwrap_or_default();
        let blocks = self.log.materialize(OpId::ROOT);

        match semantic.sections.first_mut() {
            Some(section) => section.blocks = blocks,
            None => semantic.sections.push(agenticpdf::doc::Section {
                blocks,
                ..agenticpdf::doc::Section::default()
            }),
        }
        semantic
    }

    /// Export to a text format.
    pub fn export(&self, to: &str) -> Result<String, PdfError> {
        let semantic = self.materialized();
        Ok(match to.to_ascii_lowercase().as_str() {
            "markdown" | "md" => agenticpdf::doc::to_markdown(&semantic),
            "html" => agenticpdf::doc::to_html(&semantic),
            "text" | "txt" => semantic.text(),
            other => {
                return Err(PdfError::Unsupported(format!(
                    "export target '{other}' (expected markdown, html or text)"
                )));
            }
        })
    }
}

/// Every top-level block across every section, flattened.
///
/// The edit log is a single ordered list, while a document may have several
/// sections. Flattening loses section boundaries for editing purposes, which is
/// correct for reflowable text and wrong for slides — noted here because it is
/// the first thing to revisit when slide editing lands.
fn top_level_blocks(semantic: &SemanticDoc) -> Vec<Block> {
    semantic
        .sections
        .iter()
        .flat_map(|section| section.blocks.iter().cloned())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const MARKDOWN: &str = "# Report\n\nRevenue grew across EMEA.\n\n- Hiring on plan\n- Churn down\n";

    fn session() -> Session {
        Session::open(MARKDOWN.as_bytes().to_vec()).unwrap()
    }

    #[test]
    fn opening_seeds_an_editable_log_from_any_format() {
        let session = session();
        assert_eq!(session.format(), Format::Markdown);
        assert!(!session.is_dirty());
        // Heading, paragraph, list.
        assert_eq!(session.blocks().len(), 3);
    }

    #[test]
    fn navigation_clamps_to_the_document() {
        let mut session = session();
        assert_eq!(session.page(), 1);
        assert!(!session.previous_page(), "already at the first page");
        assert_eq!(session.page(), 1);

        session.go_to_page(9999);
        assert_eq!(session.page(), session.page_count());
    }

    #[test]
    fn zoom_is_bounded() {
        let mut session = session();
        session.set_zoom(1000.0);
        assert!(session.zoom() <= 8.0);
        session.set_zoom(-5.0);
        assert!(session.zoom() >= 0.1);
    }

    #[test]
    fn search_finds_blocks_and_requires_every_term() {
        let session = session();
        let hits = session.search("revenue emea");
        assert_eq!(hits.len(), 1);
        assert!(hits[0].text.contains("EMEA"));

        assert!(session.search("revenue churn").is_empty());
        assert!(session.search("   ").is_empty(), "a blank query matches nothing");
    }

    #[test]
    fn editing_marks_the_session_dirty_and_shows_in_the_blocks() {
        let mut session = session();
        let last = session.blocks().last().map(|(id, _)| *id);
        session.insert_block(ACTOR_USER, last, Block::paragraph("appended"));

        assert!(session.is_dirty());
        let blocks = session.blocks();
        assert_eq!(blocks.len(), 4);

        let mut text = String::new();
        agenticpdf::doc::block_text_into(&blocks[3].1, &mut text);
        assert_eq!(text.trim(), "appended");
    }

    #[test]
    fn an_agent_edit_is_attributed_to_the_agent() {
        let mut session = session();
        let node = session.insert_block(ACTOR_AGENT, None, Block::paragraph("by the agent"));

        let (name, is_agent, _) = session.attribution(node).unwrap();
        assert_eq!(name, "agent");
        assert!(is_agent, "the app must be able to say a model wrote this");
    }

    #[test]
    fn deleting_removes_a_block() {
        let mut session = session();
        let first = session.blocks()[0].0;
        session.delete_block(ACTOR_USER, first);
        assert_eq!(session.blocks().len(), 2);
    }

    #[test]
    fn saving_produces_an_adf_document_that_reopens_with_the_edits() {
        let mut session = session();
        session.insert_block(ACTOR_USER, None, Block::paragraph("inserted first"));
        let saved = session.save().unwrap();

        assert!(!session.is_dirty(), "saving should clear the dirty flag");
        assert!(AdfDoc::sniff(&saved), "save must produce ADF");

        let reopened = Session::open(saved).unwrap();
        assert_eq!(reopened.format(), Format::Adf);
        assert!(reopened.export("markdown").unwrap().contains("inserted first"));
        // The seeded history survives the save, so attribution is not lost.
        assert!(reopened.log().len() >= 4);
    }

    #[test]
    fn saving_an_adf_document_twice_appends_rather_than_rewriting() {
        let mut session = session();
        let first = Session::open(session.save().unwrap()).unwrap();

        let mut second = first;
        let before = second.save().unwrap().len();
        second.insert_block(ACTOR_USER, None, Block::paragraph("a small edit"));
        let after = second.save().unwrap().len();

        let grew = after - before;
        assert!(grew < 512, "a one-block edit grew the file by {grew} bytes");
    }

    #[test]
    fn a_saved_document_is_searchable_through_its_own_index() {
        let mut session = session();
        let reopened = Session::open(session.save().unwrap()).unwrap();

        let hits = reopened.search("revenue emea");
        assert_eq!(hits.len(), 1, "the embedded index should answer this");
        assert!(hits[0].text.contains("EMEA"));
    }

    #[test]
    fn concurrent_sessions_merge_without_losing_either_edit() {
        let mut left = session();
        let mut right = session();

        left.insert_block(ACTOR_USER, None, Block::paragraph("from the user"));
        right.insert_block(ACTOR_AGENT, None, Block::paragraph("from the agent"));

        left.merge(right.log());
        let markdown = left.export("markdown").unwrap();
        assert!(markdown.contains("from the user"));
        assert!(markdown.contains("from the agent"));
    }

    #[test]
    fn export_rejects_a_format_it_cannot_write() {
        let session = session();
        assert!(session.export("markdown").is_ok());
        assert!(session.export("docx").is_err(), "we do not write OOXML");
    }

    #[test]
    fn a_real_pdf_opens_paginates_and_yields_paintable_geometry() {
        // Skip-if-absent, matching `tests/real_pdfs.rs`: the sample is not
        // guaranteed to be present in every checkout, and a missing fixture is
        // not a failing test.
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../demos/sample.pdf");
        let Ok(bytes) = std::fs::read(path) else {
            eprintln!("skipping: {path} is not present");
            return;
        };

        let mut session = Session::open(bytes).unwrap();
        assert_eq!(session.format(), Format::Pdf);
        assert!(session.page_count() > 1, "a real PDF should paginate");

        // The canvas needs geometry for whatever page is showing; a PDF is the
        // case where it comes from the file rather than from the typesetter.
        let list = session.display_list().expect("page 1 should have geometry");
        assert!(!list.ops.is_empty(), "page 1 produced no draw operations");
        assert!(list.width > 0.0 && list.height > 0.0);

        session.go_to_page(2);
        assert!(session.display_list().is_ok(), "page 2 should also render");

        // Search works on a format with no embedded index, via the scan path.
        assert!(session.search("the").len() <= session.blocks().len() + 64);
    }

    #[test]
    fn opening_rubbish_fails_rather_than_producing_an_empty_document() {
        assert!(Session::open(vec![0x00, 0x01, 0x02, 0x03]).is_err());
        assert!(Session::open(Vec::new()).is_err());
    }
}
