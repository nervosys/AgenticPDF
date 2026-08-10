// SPDX-License-Identifier: AGPL-3.0-or-later
//! The format-neutral document facade.
//!
//! [`Document`] is the front door for every supported format. It sniffs the
//! input, routes it to the right parser, caches the result, and exposes one API
//! whether the source was a PDF, a Word file or a Markdown note.
//!
//! ## Two paths, one surface
//!
//! A PDF carries *geometry* and no structure, so its structure is inferred by
//! [`crate::layout`] from where the glyphs sit. Word, HTML and the rest carry
//! *structure* and no geometry, so they parse into [`crate::doc::SemanticDoc`]
//! and their geometry is computed by the typesetter.
//!
//! Where a format has authored structure, using it beats inferring: heading
//! levels, list numbering, merged cells and hidden-text flags come from the
//! source instead of from font-size heuristics. So `Document` prefers the
//! semantic path when one exists and falls back to the geometric path
//! otherwise — per capability, not per document.
//!
//! ## Geometry
//!
//! Formats without geometry get it computed by [`crate::typeset`] when the
//! document is opened, so bounding boxes, display lists and rendering work for
//! every supported format. Those coordinates are the typesetter's own — a
//! `.docx` has no single correct pagination, and Word would place the same text
//! differently — but they are internally consistent: the boxes a citation
//! reports are the boxes a renderer draws.

use crate::detect::{self, Format};
use crate::doc::{self, SemanticDoc};
use crate::typeset;
use crate::engine::{self, DisplayList, PageImage, StructNode};
use crate::{
    FullExtraction, PdfDocument, PdfError, PdfMetadata, SemanticChunk, figures,
    formula, layout, sanitize, tables,
};

/// A parsed document of any supported format.
#[derive(Debug)]
pub struct Document {
    format: Format,
    /// Authored structure. `None` for PDF, which has none to read.
    semantic: Option<SemanticDoc>,
    /// Geometric view: read from the file for PDF, computed for everything else.
    geometric: PdfDocument,
    /// Computed geometry, for the formats whose pages the typesetter built.
    /// `None` for PDF, whose geometry comes from the file itself.
    typeset: Option<typeset::Typeset>,
    metadata: PdfMetadata,
    /// The original bytes, kept because several engine entry points re-parse
    /// from them rather than from an already-built document.
    data: Vec<u8>,
}

impl Document {
    /// Detect the format from the bytes and parse.
    pub fn open(data: &[u8]) -> Result<Document, PdfError> {
        Document::open_with_hint(data, None)
    }

    /// Parse with an optional filename hint.
    ///
    /// The hint only breaks ties within the plain-text family; it never
    /// overrides a binary signature, so a mislabelled file is still read as
    /// what it actually is.
    pub fn open_with_hint(data: &[u8], hint: Option<&str>) -> Result<Document, PdfError> {
        let format = detect::detect(data, hint)?;
        Document::open_as(data, format)
    }

    /// Parse as a specific format, skipping detection.
    pub fn open_as(data: &[u8], format: Format) -> Result<Document, PdfError> {
        if format == Format::Pdf {
            let geometric = PdfDocument::from_bytes(data)?;
            let mut metadata = geometric.get_metadata().clone();
            metadata.format = format.id().to_string();
            return Ok(Document {
                format,
                semantic: None,
                geometric,
                typeset: None,
                metadata,
                data: data.to_vec(),
            });
        }

        let semantic = crate::formats::parse(data, format)?;
        let mut metadata = metadata_from_semantic(&semantic, format, data.len());

        // Compute geometry up front. Everything downstream — bounding boxes,
        // rendering, figure detection — needs it, and laying the document out
        // twice would risk the two answers disagreeing.
        let laid_out = typeset::typeset(&semantic);
        metadata.has_annotations = !laid_out.annotations.is_empty();
        // The rendered page count, not the section count: this is what every
        // page-indexed call reports and accepts.
        metadata.page_count = laid_out.pages.len();

        Ok(Document {
            format,
            geometric: PdfDocument::from_parts(
                String::new(),
                laid_out.pages.clone(),
                metadata.clone(),
                laid_out.annotations.clone(),
                Vec::new(),
            ),
            typeset: Some(laid_out),
            semantic: Some(semantic),
            metadata,
            data: data.to_vec(),
        })
    }

    /// The format this document was parsed as.
    pub fn format(&self) -> Format {
        self.format
    }

    /// The authored structure, for formats that carry any.
    pub fn semantic(&self) -> Option<&SemanticDoc> {
        self.semantic.as_ref()
    }

    /// The geometric view. Empty for non-PDF formats until they are typeset.
    pub fn geometric(&self) -> &PdfDocument {
        &self.geometric
    }

    /// Document metadata, including the source format.
    pub fn metadata(&self) -> &PdfMetadata {
        &self.metadata
    }

    /// Number of pages.
    ///
    /// This is the *rendered* page count, which is what every page-indexed call
    /// here takes. For a deck or a workbook it equals the slide or sheet count;
    /// for a reflowable document it is however many pages the typesetter needed,
    /// which is generally more than the one section the content came from.
    pub fn page_count(&self) -> usize {
        self.geometric.pages.len()
    }

    /// Number of authored divisions: slides, worksheets, or the single flow of
    /// a reflowable document.
    pub fn section_count(&self) -> usize {
        match &self.semantic {
            Some(semantic) => semantic.sections.len(),
            None => self.geometric.pages.len(),
        }
    }

    /// Reading-order Markdown.
    pub fn to_markdown(&self) -> String {
        match &self.semantic {
            Some(semantic) => doc::to_markdown(semantic),
            None => self.geometric.to_markdown(),
        }
    }

    /// Reading-order Markdown for a 1-based, inclusive page range.
    ///
    /// The whole document goes through the semantic path, which is the
    /// higher-fidelity one. A *subset* of pages cannot: the authored structure
    /// has no page numbers in it, so a range has to be taken from the laid-out
    /// geometry, where the pages actually exist.
    pub fn to_markdown_range(&self, start: usize, end: usize) -> String {
        let whole_document = start <= 1 && end >= self.page_count();
        if whole_document {
            return self.to_markdown();
        }

        let mut structured = self.geometric.to_structured();
        structured
            .pages
            .retain(|page| page.page_number >= start && page.page_number <= end);
        layout::render_markdown(&structured)
    }

    /// Render as an HTML fragment.
    ///
    /// A PDF has no authored structure to serialise, so its inferred
    /// reading-order blocks are lifted into the semantic model first — the same
    /// blocks its Markdown comes from, rendered to a different target.
    pub fn to_html(&self) -> String {
        match &self.semantic {
            Some(semantic) => doc::to_html(semantic),
            None => doc::to_html(&structured_to_semantic(&self.geometric.to_structured())),
        }
    }

    /// All text content, in reading order.
    pub fn extract_text(&self) -> String {
        match &self.semantic {
            Some(semantic) => semantic.text(),
            None => self.geometric.extract_text(),
        }
    }

    /// The document's logical structure tree.
    ///
    /// For semantic formats this is the author's own structure; for PDF it is
    /// the tagged-PDF `/StructTreeRoot`, which is absent from untagged files.
    pub fn structure(&self) -> Result<Vec<StructNode>, PdfError> {
        match &self.semantic {
            Some(semantic) => Ok(semantic.to_struct_nodes()),
            None => engine::extract_structure(&self.data),
        }
    }

    /// Tables in the document.
    ///
    /// Semantic formats state their tables outright, so those are read rather
    /// than reconstructed — merged cells and header rows survive exactly. PDF
    /// tables are recovered from ruling lines and text alignment.
    pub fn tables(&self) -> Vec<tables::Table> {
        match &self.semantic {
            Some(semantic) => semantic_tables(semantic),
            None => {
                let graphics = self.page_graphics();
                tables::detect_tables(&graphics, &self.geometric.pages)
            }
        }
    }

    /// Semantic chunks for RAG pipelines.
    ///
    /// Chunks are cut from the laid-out pages rather than the authored
    /// structure, so each one is attributed to the page it appears on. A
    /// citation that says "page 4" is then something a reader can act on —
    /// which is the point of carrying provenance at all.
    pub fn generate_chunks(&self, max_chunk_size: usize, overlap: usize) -> Vec<SemanticChunk> {
        self.geometric.generate_chunks(max_chunk_size, overlap)
    }

    /// Scan for hidden text that an agent would read but a human would not see.
    ///
    /// The two formats hide text differently and both are checked: PDF puts it
    /// off-page or at zero size, while Office and HTML mark it invisible with
    /// styling that leaves it fully extractable.
    pub fn scan(&self) -> sanitize::ScanReport {
        let Some(semantic) = &self.semantic else {
            return sanitize::scan(&self.geometric);
        };

        let findings: Vec<sanitize::Finding> = semantic
            .hidden_text()
            .into_iter()
            .map(|(section, text)| sanitize::Finding {
                reason: sanitize::Reason::Hidden,
                page_number: section + 1,
                text,
                x: 0.0,
                y: 0.0,
                font_size: 0.0,
            })
            .collect();

        sanitize::ScanReport {
            clean: findings.is_empty(),
            suspicious_fragments: findings.len(),
            findings,
        }
    }

    /// A copy with hidden text removed.
    pub fn sanitized(&self) -> Document {
        let Some(semantic) = &self.semantic else {
            return Document {
                format: self.format,
                semantic: None,
                geometric: sanitize::sanitized(&self.geometric),
                typeset: None,
                metadata: self.metadata.clone(),
                data: self.data.clone(),
            };
        };

        // Re-typeset after stripping: removing text changes where everything
        // after it sits, and geometry that still described the unsanitised
        // document would put boxes over words that are no longer there.
        let mut stripped = semantic.clone();
        doc::strip_hidden(&mut stripped);
        let laid_out = typeset::typeset(&stripped);

        Document {
            format: self.format,
            geometric: PdfDocument::from_parts(
                String::new(),
                laid_out.pages.clone(),
                self.metadata.clone(),
                laid_out.annotations.clone(),
                Vec::new(),
            ),
            typeset: Some(laid_out),
            semantic: Some(stripped),
            metadata: self.metadata.clone(),
            data: self.data.clone(),
        }
    }

    /// Structured, reading-order layout with bounding boxes.
    pub fn to_structured(&self) -> Result<layout::StructuredDoc, PdfError> {
        Ok(self.geometric.to_structured())
    }

    /// A page's device-space display list, ready for a rasterizer.
    pub fn display_list(&self, page_number: usize) -> Result<DisplayList, PdfError> {
        let Some(laid_out) = &self.typeset else {
            return engine::extract_display_list(&self.data, page_number);
        };
        laid_out
            .display
            .get(page_number.saturating_sub(1))
            .cloned()
            .ok_or_else(|| self.no_such_page(page_number))
    }

    /// A page's decoded images, keyed to match its display list.
    pub fn page_images(&self, page_number: usize) -> Result<Vec<PageImage>, PdfError> {
        let Some(laid_out) = &self.typeset else {
            return engine::extract_page_images(&self.data, page_number);
        };
        laid_out
            .images
            .get(page_number.saturating_sub(1))
            .cloned()
            .ok_or_else(|| self.no_such_page(page_number))
    }

    /// Figures linked to their captions.
    pub fn figures(&self) -> Result<Vec<figures::Figure>, PdfError> {
        let placed = match &self.typeset {
            Some(laid_out) => laid_out.placed_images(),
            None => engine::extract_placed_images(&self.data)?,
        };
        Ok(figures::extract_figures_from(&placed, &self.geometric))
    }

    /// Best-effort LaTeX for detected formulas.
    ///
    /// Formula reconstruction reads super/subscript positions and fraction
    /// rules out of the page, so it applies to typeset documents too — though a
    /// word processor's equations are usually embedded objects this cannot see.
    pub fn formulas(&self) -> Vec<formula::Formula> {
        let graphics = match &self.typeset {
            Some(laid_out) => laid_out.graphics.clone(),
            None => engine::extract_graphics(&self.data).unwrap_or_default(),
        };
        formula::extract_formulas(&self.geometric, &graphics)
    }

    /// Ruling geometry for each page.
    fn page_graphics(&self) -> Vec<engine::PageGraphics> {
        match &self.typeset {
            Some(laid_out) => laid_out.graphics.clone(),
            None => engine::extract_graphics(&self.data).unwrap_or_default(),
        }
    }

    fn no_such_page(&self, page_number: usize) -> PdfError {
        PdfError::Unsupported(format!(
            "page {page_number} is out of range; this {} document has {} page(s)",
            self.format.label(),
            self.page_count()
        ))
    }

    /// Everything in one pass, for agentic workflows.
    pub fn extract_all(&self, chunk_size: usize, chunk_overlap: usize) -> FullExtraction {
        if self.semantic.is_none() {
            return self
                .geometric
                .extract_all_with_data(&self.data, chunk_size, chunk_overlap);
        }

        FullExtraction {
            metadata: self.metadata.clone(),
            pages: Vec::new(),
            annotations: Vec::new(),
            outline: Vec::new(),
            chunks: self.generate_chunks(chunk_size, chunk_overlap),
            markdown: self.to_markdown(),
            tables: self.tables(),
            figures: Vec::new(),
            formulas: Vec::new(),
            scan: Some(self.scan()),
        }
    }

    /// The original bytes, for the engine entry points that re-parse them.
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Borrow the geometric document.
    ///
    /// Every supported format has geometry — read from the file for PDF,
    /// computed by the typesetter otherwise — so this only fails for
    /// capabilities that need the *original PDF bytes* rather than coordinates.
    pub fn require_geometry(&self, _capability: &str) -> Result<&PdfDocument, PdfError> {
        Ok(&self.geometric)
    }

    /// Borrow the geometric document, requiring that it came from a real PDF.
    ///
    /// A few capabilities read PDF structures that a typeset page has no
    /// equivalent of — AcroForm fields, scanned-page detection, embedded image
    /// XObjects. Those go through here, so the failure names the format instead
    /// of returning an empty result that looks like a legitimate answer.
    pub fn require_pdf(&self, capability: &str) -> Result<&PdfDocument, PdfError> {
        if self.typeset.is_some() {
            return Err(PdfError::Unsupported(format!(
                "{capability} reads PDF-specific structures, which {} documents do not have",
                self.format.label()
            )));
        }
        Ok(&self.geometric)
    }
}

/// Lift an inferred reading-order layout into the semantic model.
///
/// This is the bridge that lets geometry-only formats reach the serialisers
/// written against authored structure. Consecutive list items are regrouped
/// into a single list, since [`layout`] classifies them one line at a time.
fn structured_to_semantic(structured: &layout::StructuredDoc) -> SemanticDoc {
    let mut document = SemanticDoc::default();

    for page in &structured.pages {
        let mut section = doc::Section::default();
        let mut pending_items: Vec<doc::ListItem> = Vec::new();

        for block in &page.blocks {
            match block.kind {
                layout::BlockKind::ListItem => pending_items.push(doc::ListItem {
                    blocks: vec![doc::Block::paragraph(block.text.clone())],
                    checked: None,
                }),
                _ => {
                    if !pending_items.is_empty() {
                        section.blocks.push(doc::Block::List(doc::List {
                            ordered: false,
                            start: 1,
                            items: std::mem::take(&mut pending_items),
                        }));
                    }
                    section.blocks.push(match block.kind {
                        layout::BlockKind::Heading => {
                            doc::Block::heading(block.level.max(1), block.text.clone())
                        }
                        _ => doc::Block::paragraph(block.text.clone()),
                    });
                }
            }
        }

        if !pending_items.is_empty() {
            section.blocks.push(doc::Block::List(doc::List {
                ordered: false,
                start: 1,
                items: pending_items,
            }));
        }
        document.sections.push(section);
    }

    document
}

/// Build metadata from a semantic document.
fn metadata_from_semantic(semantic: &SemanticDoc, format: Format, size: usize) -> PdfMetadata {
    PdfMetadata {
        title: semantic.title.clone(),
        author: semantic.author.clone(),
        subject: semantic.subject.clone(),
        creator: semantic.creator.clone(),
        producer: None,
        creation_date: semantic.created.clone(),
        modification_date: semantic.modified.clone(),
        page_count: semantic.sections.len(),
        file_size: size,
        format: format.id().to_string(),
        pdf_version: String::new(),
        encrypted: false,
        has_forms: false,
        has_annotations: false,
        has_outlines: semantic
            .sections
            .iter()
            .any(|section| section.title.is_some()),
    }
}

/// Convert the semantic model's tables to the engine's reporting type.
///
/// The bounding box is zeroed: these tables are read from authored structure,
/// which has no coordinates until the document is typeset. Reporting `[0,0,0,0]`
/// is honest; inventing a box would corrupt citations.
fn semantic_tables(semantic: &SemanticDoc) -> Vec<tables::Table> {
    let mut out = Vec::new();
    for (index, section) in semantic.sections.iter().enumerate() {
        for block in &section.blocks {
            collect_tables(block, index + 1, &mut out);
        }
    }
    out
}

fn collect_tables(block: &doc::Block, page_number: usize, out: &mut Vec<tables::Table>) {
    match block {
        doc::Block::Table(table) => {
            let grid = doc::table_grid(table);
            let cols = grid.iter().map(Vec::len).max().unwrap_or(0);
            out.push(tables::Table {
                page_number,
                rows: grid.len(),
                cols,
                bbox: [0.0; 4],
                cells: grid,
            });
        }
        doc::Block::Quote(blocks) => {
            for block in blocks {
                collect_tables(block, page_number, out);
            }
        }
        doc::Block::List(list) => {
            for item in &list.items {
                for block in &item.blocks {
                    collect_tables(block, page_number, out);
                }
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opens_markdown_and_prefers_authored_structure() {
        let source = b"# Title\n\nSome prose.\n\n- a\n- b\n";
        let document = Document::open(source).unwrap();
        assert_eq!(document.format(), Format::Markdown);
        assert!(document.semantic().is_some());
        assert_eq!(document.metadata().title.as_deref(), Some("Title"));
        assert_eq!(document.metadata().format, "markdown");
        assert!(document.to_markdown().starts_with("# Title"));
    }

    #[test]
    fn opens_html_and_reads_its_tables() {
        let html = b"<h1>T</h1><table><tr><th>a</th><th>b</th></tr><tr><td>1</td><td>2</td></tr></table>";
        let document = Document::open(html).unwrap();
        assert_eq!(document.format(), Format::Html);
        let tables = document.tables();
        assert_eq!(tables.len(), 1);
        assert_eq!((tables[0].rows, tables[0].cols), (2, 2));
        assert_eq!(tables[0].cells[1], vec!["1", "2"]);
    }

    #[test]
    fn opens_csv_as_a_single_table() {
        let document = Document::open(b"name,age\nada,36\n").unwrap();
        assert_eq!(document.format(), Format::Csv);
        assert_eq!(document.tables().len(), 1);
        assert!(document.extract_text().contains("ada"));
    }

    #[test]
    fn opens_pdf_through_the_geometric_path() {
        let pdf = b"%PDF-1.4\n\
            1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
            2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
            3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] >>\nendobj\n\
            xref\n0 4\n\
            0000000000 65535 f \n\
            0000000009 00000 n \n\
            0000000058 00000 n \n\
            0000000115 00000 n \n\
            trailer\n<< /Size 4 /Root 1 0 R >>\n\
            startxref\n195\n%%EOF";
        let document = Document::open(pdf).unwrap();
        assert_eq!(document.format(), Format::Pdf);
        assert!(document.semantic().is_none());
        assert_eq!(document.metadata().format, "pdf");
        assert_eq!(document.page_count(), 1);
        // Geometry-dependent capabilities work for PDF.
        assert!(document.to_structured().is_ok());
    }

    #[test]
    fn page_count_follows_sections_for_semantic_formats() {
        let document = Document::open(b"# One\n\ntext\n").unwrap();
        assert_eq!(document.page_count(), 1);
    }

    #[test]
    fn chunks_carry_section_provenance() {
        let document = Document::open(b"# Title\n\nalpha beta gamma delta\n").unwrap();
        let chunks = document.generate_chunks(2, 0);
        assert!(!chunks.is_empty());
        assert!(chunks.iter().all(|c| c.page_numbers == vec![1]));
        let joined: String = chunks.iter().map(|c| c.content.clone()).collect();
        assert!(joined.contains("alpha"));
    }

    #[test]
    fn structure_comes_from_authored_markup() {
        let document = Document::open(b"# H\n\np\n").unwrap();
        let nodes = document.structure().unwrap();
        assert_eq!(nodes[0].kind, "Document");
        assert_eq!(nodes[0].children[0].kind, "H1");
    }

    #[test]
    fn scan_reports_hidden_html_text() {
        let html = br#"<p>ok <span style="display:none">payload</span></p>"#;
        let document = Document::open(html).unwrap();
        let report = document.scan();
        assert!(!report.clean);
        assert_eq!(report.suspicious_fragments, 1);
        assert_eq!(report.findings[0].text.trim(), "payload");
    }

    #[test]
    fn sanitized_removes_hidden_text_but_keeps_the_rest() {
        let html = br#"<p>keep <span style="display:none">drop</span></p>"#;
        let document = Document::open(html).unwrap().sanitized();
        let markdown = document.to_markdown();
        assert!(markdown.contains("keep"));
        assert!(!markdown.contains("drop"));
        assert!(document.scan().clean);
    }

    #[test]
    fn geometry_dependent_calls_work_for_semantic_formats() {
        let document = Document::open(b"# Title\n\nSome body text.\n").unwrap();

        let structured = document.to_structured().unwrap();
        assert_eq!(structured.pages.len(), document.page_count());
        assert!(!structured.pages[0].blocks.is_empty());

        let list = document.display_list(1).unwrap();
        assert_eq!(list.page_number, 1);
        assert!(!list.ops.is_empty());

        assert!(document.page_images(1).is_ok());
        assert!(document.figures().is_ok());
    }

    #[test]
    fn a_page_out_of_range_says_how_many_there_are() {
        let document = Document::open(b"# T\n\nbody\n").unwrap();
        let error = document.display_list(99).unwrap_err();
        let PdfError::Unsupported(message) = error else {
            panic!("expected Unsupported")
        };
        assert!(message.contains("out of range"), "got: {message}");
        assert!(message.contains("Markdown"), "format not named: {message}");
    }

    #[test]
    fn pdf_only_capabilities_still_name_the_format() {
        // Form fields and scanned-page detection read PDF structures that a
        // typeset page has no equivalent of.
        let document = Document::open(b"# T\n\nbody\n").unwrap();
        let error = document.require_pdf("form fields").unwrap_err();
        let PdfError::Unsupported(message) = error else {
            panic!("expected Unsupported")
        };
        assert!(message.contains("Markdown"), "got: {message}");
        assert!(message.contains("PDF-specific"), "got: {message}");
    }

    #[test]
    fn bounding_boxes_land_on_the_page() {
        let document = Document::open(b"# Heading\n\nA paragraph of text.\n").unwrap();
        let structured = document.to_structured().unwrap();
        let page = &structured.pages[0];

        for block in &page.blocks {
            let [left, bottom, right, top] = block.bbox;
            assert!(right > left && top > bottom, "degenerate bbox: {:?}", block.bbox);
            assert!(left >= 0.0 && right <= page.width, "off page: {:?}", block.bbox);
            assert!(bottom >= 0.0 && top <= page.height, "off page: {:?}", block.bbox);
        }
    }

    #[test]
    fn a_long_document_paginates_and_chunks_cite_real_pages() {
        let source: String = (0..300)
            .map(|i| format!("Paragraph number {i} with enough words to take up room.\n\n"))
            .collect();
        let document = Document::open(source.as_bytes()).unwrap();

        assert!(document.page_count() > 1, "expected pagination");
        assert_eq!(document.section_count(), 1, "still one authored flow");
        assert_eq!(document.metadata().page_count, document.page_count());

        let chunks = document.generate_chunks(120, 10);
        let cited: Vec<usize> = chunks.iter().flat_map(|c| c.page_numbers.clone()).collect();
        assert!(
            cited.iter().any(|&page| page > 1),
            "chunks all cite page 1 despite {} pages",
            document.page_count()
        );
        assert!(
            cited.iter().all(|&page| page <= document.page_count()),
            "a chunk cites a page that does not exist"
        );
    }

    #[test]
    fn extract_all_bundles_the_semantic_capabilities() {
        let html = b"<h1>T</h1><p>body</p><table><tr><th>a</th></tr><tr><td>1</td></tr></table>";
        let bundle = Document::open(html).unwrap().extract_all(500, 50);
        assert_eq!(bundle.metadata.format, "html");
        assert!(bundle.markdown.contains("# T"));
        assert_eq!(bundle.tables.len(), 1);
        assert!(!bundle.chunks.is_empty());
        assert!(bundle.scan.is_some());
    }

    #[test]
    fn open_as_bypasses_detection() {
        // Content that sniffs as Markdown, forced to plain text.
        let document = Document::open_as(b"- a\n- b\n", Format::Text).unwrap();
        assert_eq!(document.format(), Format::Text);
        assert!(document.to_markdown().contains("\\- a"));
    }

    #[test]
    fn opens_rtf_through_the_semantic_path() {
        let document = Document::open(br"{\rtf1\ansi\deff0 \b Bold\b0  text\par}").unwrap();
        assert_eq!(document.format(), Format::Rtf);
        assert_eq!(document.to_markdown().trim(), "**Bold** text");
    }

    #[test]
    fn every_detectable_format_has_a_parser() {
        // Detection and parsing are separate capabilities, and it used to be
        // possible to detect a format the engine could not read. That gap is
        // closed; this keeps it closed.
        for format in Format::all() {
            assert!(
                format.is_supported(),
                "{format:?} is detected but has no parser"
            );
        }
    }

    #[test]
    fn a_truncated_legacy_binary_reports_the_container_not_the_bytes() {
        // Recognisable as a compound file, but with nothing readable inside.
        let mut ole2 = vec![0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];
        ole2.extend(std::iter::repeat_n(0u8, 500));
        ole2.extend("WordDocument".bytes().flat_map(|b| [b, 0]));

        assert_eq!(crate::detect::detect(&ole2, None).unwrap(), Format::Doc);
        // It fails, but as a container problem rather than "invalid PDF header".
        let error = Document::open(&ole2).unwrap_err();
        assert!(
            matches!(&error, PdfError::Malformed(_) | PdfError::MissingPart(_)),
            "got {error:?}"
        );
    }

    #[test]
    fn opens_opendocument_through_the_semantic_path() {
        let content = br#"<office:document-content
              xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
              xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0">
            <office:body><office:text>
              <text:h text:outline-level="1">Title</text:h>
              <text:p>Body.</text:p>
            </office:text></office:body></office:document-content>"#;
        let odt = crate::testing::build_zip(&[
            (
                "mimetype",
                b"application/vnd.oasis.opendocument.text",
                false,
            ),
            ("content.xml", content, true),
        ]);
        let document = Document::open(&odt).unwrap();
        assert_eq!(document.format(), Format::Odt);
        assert_eq!(document.to_markdown(), "# Title\n\nBody.\n");
        // And it has geometry, like every other supported format.
        assert!(document.display_list(1).is_ok());
    }
}
