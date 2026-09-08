// SPDX-License-Identifier: AGPL-3.0-or-later
//! WordprocessingML (`.docx`) reader.
//!
//! `word/document.xml` is a flat sequence of `<w:p>` paragraphs and `<w:tbl>`
//! tables. Almost all the interesting information is in properties rather than
//! element names: a heading is a paragraph whose `<w:pStyle>` names a heading
//! style, a list item is a paragraph with a `<w:numPr>`, and bold text is a run
//! whose `<w:rPr>` contains `<w:b/>`.
//!
//! Three things this reader is careful about, because they are where naive
//! extraction goes wrong:
//!
//! - **`w:val="0"` means off.** `<w:b/>` is bold but `<w:b w:val="0"/>` is not,
//!   and a style that switches formatting off is common in real documents.
//! - **Whitespace is significant when marked.** `<w:t xml:space="preserve">`
//!   carries the spaces *between* runs; trimming it welds words together.
//! - **`<w:vanish/>` is hidden text.** It stays fully extractable while being
//!   invisible on the page — the Word equivalent of `display:none`, and a
//!   prompt-injection vector this reader flags rather than silently passes on.

use std::collections::HashMap;

use crate::PdfError;
use crate::container::zip::ZipArchive;
use crate::doc::{
    Align, Block, Cell, ImageRef, Inline, ListItem, Row, Run, SemanticDoc, Table, TextStyle,
    image_media_type, inline_text,
};
use crate::formats::ooxml::{Package, Rels, attr_i64, emu_to_points, is_on, resolve_path};
use crate::xml::{Element, Event, Reader, ns};

/// Twips (1/20 point) per point, for indent measurements.
const TWIPS_PER_POINT: f64 = 20.0;

/// Parse a WordprocessingML package.
pub fn parse(archive: &ZipArchive, package: &Package) -> Result<SemanticDoc, PdfError> {
    let body = archive.read(&package.main_part)?;
    let base = package.base();

    let numbering = Numbering::read(archive, &base);
    let styles = Styles::read(archive, &base);

    let mut reader = DocxReader {
        archive,
        rels: &package.main_rels,
        base,
        numbering,
        styles,
        document: SemanticDoc::default(),
        images: HashMap::new(),
        notes: HashMap::new(),
        placed_notes: HashMap::new(),
        deferred: Vec::new(),
        table_style: None,
        cell_style: TextStyle::default(),
    };

    // Before the body, so a reference found there has somewhere to resolve to.
    reader.read_notes();
    let blocks = reader.read_body(&body);
    let mut document = std::mem::take(&mut reader.document);
    document.sections = vec![crate::doc::Section {
        blocks,
        ..crate::doc::Section::default()
    }];
    Ok(document)
}

struct DocxReader<'a> {
    archive: &'a ZipArchive<'a>,
    rels: &'a Rels,
    base: String,
    numbering: Numbering,
    styles: Styles,
    document: SemanticDoc,
    /// Relationship id → registered asset id, so an image used twice is stored
    /// once.
    images: HashMap<String, String>,
    /// The style of the table being read, if any. A stack is unnecessary:
    /// `read_table` saves and restores it around a nested one.
    table_style: Option<String>,
    /// The formatting the current cell inherits from that style.
    cell_style: TextStyle,
    /// Footnote and endnote bodies, by the id a reference names.
    notes: HashMap<NoteId, Vec<Block>>,
    /// Note id → its place in the document's footnote list, so a note
    /// referenced twice is stored once and numbered once.
    placed_notes: HashMap<NoteId, usize>,
    /// Blocks a drawing produced while a paragraph was being read, to be
    /// emitted after it. A text box is anchored inside a paragraph but is not
    /// part of the sentence, so its content cannot go inline and must not be
    /// lost.
    deferred: Vec<Block>,
}

/// A note's identity: its number, and which of the two lists it is in.
///
/// The two are numbered separately, so a footnote and an endnote may both be
/// id 2 and mean different things.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct NoteId {
    endnote: bool,
    id: i64,
}

impl DocxReader<'_> {
    /// Read the footnote and endnote parts, so a reference can resolve.
    ///
    /// A note's text lives in a part of its own and the run carries only a
    /// reference to it, so a reader that looks no further drops every footnote
    /// in the document. The model has somewhere to put them, and both the
    /// Markdown and HTML writers already render them.
    fn read_notes(&mut self) {
        for (part, endnote) in [("footnotes.xml", false), ("endnotes.xml", true)] {
            let Some(bytes) = self.archive.read_optional(&format!("{}{part}", self.base)) else {
                continue;
            };
            let mut reader = Reader::new(&bytes);
            while let Some(event) = reader.read_event() {
                let Event::Start(element) = event else {
                    continue;
                };
                if !element.in_ns(ns::W) || element.local != "footnote" && element.local != "endnote"
                {
                    continue;
                }
                // The separators are the rules Word draws above a note, not
                // notes of their own, and they carry no id worth resolving.
                if matches!(
                    element.attr_local("type"),
                    Some("separator") | Some("continuationSeparator")
                ) {
                    continue;
                }
                let Some(id) = attr_i64(&element, "id") else {
                    continue;
                };
                let blocks = self.read_nested_blocks(&mut reader, &element);
                if !blocks.is_empty() {
                    self.notes.insert(NoteId { endnote, id }, blocks);
                }
            }
        }
    }

    /// Skip an `<mc:Fallback>`, which repeats content already offered.
    ///
    /// `<mc:AlternateContent>` states the same thing twice: a `<mc:Choice>` for
    /// a reader that understands the newer markup, and a `<mc:Fallback>` in
    /// older markup for one that does not. This reader understands both, so
    /// reading each produced everything inside twice -- a text box appeared
    /// once from the drawing and again from the picture beneath it.
    fn skip_fallback(&mut self, reader: &mut Reader, element: &Element) -> bool {
        if element.local != "Fallback" {
            return false;
        }
        let _ = crate::xml::text_of(reader, &element.qname);
        true
    }

    /// Read the blocks inside one element: paragraphs and tables like any others.
    ///
    /// Used for a note's body and for a text box's, which are the same shape.
    fn read_nested_blocks(&mut self, reader: &mut Reader, start: &Element) -> Vec<Block> {
        let mut blocks: Vec<Block> = Vec::new();
        let mut depth = 1usize;

        while let Some(event) = reader.read_event() {
            match event {
                Event::End(name) if name == start.qname => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                Event::Start(element) if element.qname == start.qname => depth += 1,
                Event::Start(element) if self.skip_fallback(reader, &element) => {}
                Event::Start(element) if element.in_ns(ns::W) => match element.local.as_str() {
                    "p" => {
                        if let Some(paragraph) = self.read_paragraph(reader, &element) {
                            push_paragraph(&mut blocks, paragraph);
                        }
                    }
                    "tbl" => {
                        if let Some(table) = self.read_table(reader, &element) {
                            blocks.push(Block::Table(table));
                        }
                    }
                    _ => {}
                },
                _ => {}
            }
            self.drain_deferred(&mut blocks);
        }
        blocks
    }

    /// Add a note to the document's list the first time it is referenced.
    ///
    /// Returns where it sits in that list, which is the number it is given. A
    /// note referenced twice keeps the number it already has.
    fn place_note(&mut self, id: NoteId) -> Option<usize> {
        if let Some(index) = self.placed_notes.get(&id) {
            return Some(*index);
        }
        let blocks = self.notes.get(&id)?.clone();
        let index = self.document.footnotes.len();
        self.document.footnotes.push(crate::doc::Footnote {
            label: None,
            blocks,
        });
        self.placed_notes.insert(id, index);
        Some(index)
    }

    /// Read `<w:body>` into blocks.
    fn read_body(&mut self, xml: &[u8]) -> Vec<Block> {
        let mut reader = Reader::new(xml);
        let mut blocks: Vec<Block> = Vec::new();

        while let Some(event) = reader.read_event() {
            let Event::Start(element) = event else {
                continue;
            };
            if self.skip_fallback(&mut reader, &element) {
                continue;
            }
            if !element.in_ns(ns::W) {
                continue;
            }
            match element.local.as_str() {
                "p" => {
                    if let Some(paragraph) = self.read_paragraph(&mut reader, &element) {
                        push_paragraph(&mut blocks, paragraph);
                    }
                }
                "tbl" => {
                    if let Some(table) = self.read_table(&mut reader, &element) {
                        blocks.push(Block::Table(table));
                    }
                }
                _ => {}
            }
            self.drain_deferred(&mut blocks);
        }
        blocks
    }

    /// Emit what a drawing produced, after the paragraph that anchored it.
    ///
    /// A text box is a box beside the text rather than a word in it, so
    /// splicing it into the sentence would read as part of it.
    fn drain_deferred(&mut self, blocks: &mut Vec<Block>) {
        if !self.deferred.is_empty() {
            let deferred = std::mem::take(&mut self.deferred);
            blocks.extend(deferred);
        }
    }

    // ------------------------------------------------------------------
    // Paragraphs
    // ------------------------------------------------------------------

    /// Read one `<w:p>` and classify it as a heading, list item or paragraph.
    fn read_paragraph(&mut self, reader: &mut Reader, start: &Element) -> Option<Paragraph> {
        let mut properties = ParagraphProperties::default();
        let mut content: Vec<Inline> = Vec::new();
        let mut depth = 1usize;

        while let Some(event) = reader.read_event() {
            match event {
                Event::End(name) if name == start.qname => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                Event::Start(element) if element.qname == start.qname => depth += 1,
                Event::Start(element) if element.in_ns(ns::W) => match element.local.as_str() {
                    "pPr" => self.read_paragraph_properties(reader, &element, &mut properties),
                    "r" => {
                        // Read before the call: `read_run` borrows self.
                        // A cell's table style sits under the paragraph's own,
                        // which sits under the run's direct properties.
                        let inherited = crate::doc::layer_style(
                            &self.cell_style,
                            &self.styles.text_style(&properties.style_id),
                        );
                        self.read_run(reader, &element, &inherited, &mut content)
                    }
                    "hyperlink" => self.read_hyperlink(reader, &element, &mut content),
                    _ => {}
                },
                _ => {}
            }
        }

        if inline_text(&content).trim().is_empty() && !properties.page_break {
            return None;
        }

        // Resolve the numbering reference now, while the definitions are in
        // reach; the classifier that files the block only sees the properties.
        if let Some(reference) = properties.numbering.as_mut() {
            reference.ordered = self.numbering.is_ordered(reference.id, reference.level);
        }

        Some(Paragraph {
            content,
            properties,
        })
    }

    fn read_paragraph_properties(
        &mut self,
        reader: &mut Reader,
        start: &Element,
        properties: &mut ParagraphProperties,
    ) {
        while let Some(event) = reader.read_event() {
            match event {
                Event::End(name) if name == start.qname => break,
                Event::Start(element) if element.in_ns(ns::W) => match element.local.as_str() {
                    "pStyle" => {
                        let id = element.attr_local("val").unwrap_or_default();
                        properties.style_id = id.to_string();
                        properties.heading = self.styles.heading_level(id);
                        properties.quote = self.styles.is_quote(id);
                        // Only where the paragraph has not said so itself. In a
                        // well-formed `<w:pPr>` the style comes first, so this
                        // is the inherited value and an inline `<w:numPr>`
                        // below overwrites it -- including with `numId` zero,
                        // which is how a paragraph opts out of its style's list.
                        if properties.numbering.is_none() {
                            properties.numbering = self.styles.numbering(id);
                        }
                    }
                    "outlineLvl" => {
                        // An explicit outline level beats the style name.
                        if let Some(level) = attr_i64(&element, "val") {
                            properties.heading = Some((level.clamp(0, 8) as u8) + 1);
                        }
                    }
                    "numPr" => properties.numbering = Some(NumberingRef::default()),
                    "ilvl" => {
                        if let Some(reference) = properties.numbering.as_mut() {
                            reference.level = attr_i64(&element, "val").unwrap_or(0).max(0) as u8;
                        }
                    }
                    "numId" => {
                        if let Some(reference) = properties.numbering.as_mut() {
                            reference.id = attr_i64(&element, "val").unwrap_or(0);
                        }
                    }
                    "jc" => {
                        properties.align = match element.attr_local("val").unwrap_or_default() {
                            "center" => Align::Center,
                            "right" | "end" => Align::Right,
                            "both" | "distribute" => Align::Justify,
                            _ => Align::Left,
                        }
                    }
                    "ind" => {
                        if let Some(twips) =
                            attr_i64(&element, "left").or(attr_i64(&element, "start"))
                        {
                            properties.indent = (twips as f64 / TWIPS_PER_POINT).max(0.0);
                        }
                    }
                    "br" if element.attr_local("type") == Some("page") => {
                        properties.page_break = true;
                    }
                    _ => {}
                },
                _ => {}
            }
        }

        // `<w:numId w:val="0"/>` is how a paragraph opts out of the list its
        // style would otherwise give it. It reads as a numbering reference like
        // any other, so without this a paragraph that explicitly left the list
        // is rendered as a list item.
        if properties.numbering.as_ref().is_some_and(|n| n.id == 0) {
            properties.numbering = None;
        }
    }

    // ------------------------------------------------------------------
    // Runs
    // ------------------------------------------------------------------

    /// Read one `<w:r>`, appending its content with the resolved style.
    fn read_run(
        &mut self,
        reader: &mut Reader,
        start: &Element,
        inherited: &TextStyle,
        into: &mut Vec<Inline>,
    ) {
        let mut style = inherited.clone();

        while let Some(event) = reader.read_event() {
            match event {
                Event::End(name) if name == start.qname => break,
                Event::Start(element) if self.skip_fallback(reader, &element) => {}
                Event::Start(element) if element.in_ns(ns::W) => match element.local.as_str() {
                    "rPr" => self.read_run_properties(reader, &element, &mut style),
                    "t" => {
                        let preserve_space = element.attr("xml:space") == Some("preserve");
                        let text = crate::xml::text_of(reader, &element.qname);
                        // Only `xml:space="preserve"` guarantees the spaces
                        // matter; elsewhere leading/trailing whitespace is
                        // formatting noise from a pretty-printed file.
                        let text = if preserve_space {
                            text
                        } else {
                            text.trim().to_string()
                        };
                        if !text.is_empty() {
                            push_run(into, Run::styled(text, style.clone()));
                        }
                    }
                    "footnoteReference" | "endnoteReference" => {
                        let endnote = element.local == "endnoteReference";
                        if let Some(id) = attr_i64(&element, "id")
                            && let Some(index) = self.place_note(NoteId { endnote, id })
                        {
                            into.push(Inline::FootnoteRef { index });
                        }
                    }
                    "tab" => push_run(into, Run::styled("\t", style.clone())),
                    "br" => into.push(Inline::Break),
                    "noBreakHyphen" => push_run(into, Run::styled("\u{2011}", style.clone())),
                    "softHyphen" => push_run(into, Run::styled("\u{00AD}", style.clone())),
                    "drawing" | "pict" | "object" => {
                        if let Some(image) = self.read_drawing(reader, &element) {
                            into.push(Inline::Image(image));
                        }
                    }
                    _ => {}
                },
                _ => {}
            }
        }
    }

    fn read_run_properties(&mut self, reader: &mut Reader, start: &Element, style: &mut TextStyle) {
        while let Some(event) = reader.read_event() {
            match event {
                Event::End(name) if name == start.qname => break,
                Event::Start(element) if element.in_ns(ns::W) => {
                    if element.local == "rStyle"
                        && self
                            .styles
                            .is_code(element.attr_local("val").unwrap_or_default())
                    {
                        style.code = true;
                    }
                    apply_run_property(&element, style);
                }
                _ => {}
            }
        }
    }

    /// Read a `<w:hyperlink>`, resolving its relationship to a URL.
    fn read_hyperlink(&mut self, reader: &mut Reader, start: &Element, into: &mut Vec<Inline>) {
        let href = start
            .attr_local("id")
            .and_then(|id| self.rels.resolve(&self.base, id))
            // An internal link points at a bookmark rather than a URL.
            .or_else(|| start.attr_local("anchor").map(|a| format!("#{a}")))
            .unwrap_or_default();

        let mut content: Vec<Inline> = Vec::new();
        while let Some(event) = reader.read_event() {
            match event {
                Event::End(name) if name == start.qname => break,
                Event::Start(element) if element.is(ns::W, "r") => {
                    self.read_run(reader, &element, &TextStyle::default(), &mut content);
                }
                _ => {}
            }
        }

        let runs: Vec<Run> = content
            .iter()
            .filter_map(|inline| match inline {
                Inline::Run(run) => Some(run.clone()),
                _ => None,
            })
            .collect();

        if runs.is_empty() {
            return;
        }
        if href.is_empty() {
            into.extend(runs.into_iter().map(Inline::Run));
        } else {
            into.push(Inline::Link { href, runs });
        }
    }

    /// Read a `<w:drawing>` and register the image it references.
    fn read_drawing(&mut self, reader: &mut Reader, start: &Element) -> Option<ImageRef> {
        let mut relationship: Option<String> = None;
        let mut alt: Option<String> = None;
        let mut width: Option<f64> = None;
        let mut height: Option<f64> = None;

        while let Some(event) = reader.read_event() {
            match event {
                Event::End(name) if name == start.qname => break,
                Event::Start(element) => {
                    match element.local.as_str() {
                        // `<a:blip r:embed="rIdN"/>` names the image part.
                        "blip" => {
                            relationship = element
                                .attr_local("embed")
                                .or(element.attr_local("link"))
                                .map(str::to_string);
                        }
                        // Alt text lives on the drawing's non-visual properties.
                        "docPr" | "cNvPr" => {
                            alt = element
                                .attr_local("descr")
                                .or(element.attr_local("title"))
                                .filter(|value| !value.trim().is_empty())
                                .map(str::to_string);
                        }
                        "extent" | "ext" => {
                            width = attr_i64(&element, "cx").map(emu_to_points);
                            height = attr_i64(&element, "cy").map(emu_to_points);
                        }
                        // A drawing may hold a text box as well as, or instead
                        // of, a picture. Reading only the picture dropped its
                        // text, as all four readers of one document did.
                        "txbxContent" => {
                            let blocks = self.read_nested_blocks(reader, &element);
                            self.deferred.extend(blocks);
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }

        let relationship = relationship?;
        let asset_id = self.register_image(&relationship)?;
        Some(ImageRef {
            asset_id,
            alt,
            width,
            height,
        })
    }

    /// Register an image part as an asset, reusing it if already registered.
    fn register_image(&mut self, relationship: &str) -> Option<String> {
        if let Some(id) = self.images.get(relationship) {
            return Some(id.clone());
        }
        let path = self.rels.resolve(&self.base, relationship)?;
        let bytes = self.archive.read_optional(&path)?;
        let media_type = image_media_type(&bytes).to_string();
        let reference = self.document.add_asset(media_type, bytes);
        self.images
            .insert(relationship.to_string(), reference.asset_id.clone());
        Some(reference.asset_id)
    }

    // ------------------------------------------------------------------
    // Tables
    // ------------------------------------------------------------------

    fn read_table(&mut self, reader: &mut Reader, start: &Element) -> Option<Table> {
        let mut rows: Vec<Row> = Vec::new();
        let mut column_widths: Vec<f64> = Vec::new();
        let mut header_rows = 0usize;
        let mut depth = 1usize;
        // Saved and restored so a table nested in a cell does not take the
        // outer table's style with it.
        let outer_style = self.table_style.take();

        while let Some(event) = reader.read_event() {
            match event {
                Event::End(name) if name == start.qname => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                Event::Start(element) if element.qname == start.qname => depth += 1,
                Event::Start(element) if element.in_ns(ns::W) => match element.local.as_str() {
                    "tblStyle" => {
                        self.table_style = element.attr_local("val").map(str::to_string);
                    }
                    "gridCol" => {
                        if let Some(width) = attr_i64(&element, "w") {
                            column_widths.push(width as f64 / TWIPS_PER_POINT);
                        }
                    }
                    "tr" => {
                        let (row, is_header) = self.read_row(reader, &element);
                        if !row.cells.is_empty() {
                            // `<w:tblHeader/>` marks a row that repeats across
                            // pages, which is exactly a header row.
                            if is_header && header_rows == rows.len() {
                                header_rows += 1;
                            }
                            rows.push(row);
                        }
                    }
                    _ => {}
                },
                _ => {}
            }
        }

        self.table_style = outer_style;
        if rows.is_empty() {
            return None;
        }
        Some(Table {
            caption: None,
            header_rows,
            rows,
            column_widths,
        })
    }

    /// Read one `<w:tr>`, returning it and whether it is a header row.
    fn read_row(&mut self, reader: &mut Reader, start: &Element) -> (Row, bool) {
        let mut cells: Vec<Cell> = Vec::new();
        let mut is_header = false;
        let mut position = CellPosition::default();
        let mut depth = 1usize;

        while let Some(event) = reader.read_event() {
            match event {
                Event::End(name) if name == start.qname => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                Event::Start(element) if element.qname == start.qname => depth += 1,
                Event::Start(element) if element.in_ns(ns::W) => match element.local.as_str() {
                    "tblHeader" => is_header = is_on(&element),
                    // The row's own position, which its cells inherit.
                    "cnfStyle" => position.read(&element),
                    "tc" => {
                        if let Some(cell) = self.read_cell(reader, &element, &position) {
                            cells.push(cell);
                        }
                    }
                    _ => {}
                },
                _ => {}
            }
        }
        (Row { cells }, is_header)
    }

    /// The formatting a cell inherits from its table's style, if it has one.
    fn inherited_cell_style(&self, position: &CellPosition) -> TextStyle {
        match &self.table_style {
            Some(id) => self.styles.table_style(id, position),
            None => TextStyle::default(),
        }
    }

    fn read_cell(
        &mut self,
        reader: &mut Reader,
        start: &Element,
        row_position: &CellPosition,
    ) -> Option<Cell> {
        let mut blocks: Vec<Block> = Vec::new();
        let mut col_span = 1usize;
        let mut vertical_merge: Option<bool> = None;
        let mut position = row_position.clone();
        let mut depth = 1usize;
        let outer_cell_style = self.cell_style.clone();
        // Set before any content is read, so a cell that states no position of
        // its own still inherits its row's rather than the last cell's.
        self.cell_style = self.inherited_cell_style(&position);

        while let Some(event) = reader.read_event() {
            match event {
                Event::End(name) if name == start.qname => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                Event::Start(element) if element.qname == start.qname => depth += 1,
                Event::Start(element) if element.in_ns(ns::W) => match element.local.as_str() {
                    // The cell's own position adds its column to the row's.
                    // It precedes the cell's content, so the formatting it
                    // implies is in force by the time any paragraph is read.
                    "cnfStyle" => {
                        position.read(&element);
                        self.cell_style = self.inherited_cell_style(&position);
                    }
                    "gridSpan" => {
                        col_span = attr_i64(&element, "val").unwrap_or(1).clamp(1, 1000) as usize;
                    }
                    "vMerge" => {
                        // "restart" begins a vertical merge; anything else
                        // continues one, and continuation cells are absorbed by
                        // the cell above rather than emitted.
                        vertical_merge = Some(element.attr_local("val") == Some("restart"));
                    }
                    "p" => {
                        if let Some(paragraph) = self.read_paragraph(reader, &element) {
                            push_paragraph(&mut blocks, paragraph);
                        }
                    }
                    "tbl" => {
                        if let Some(table) = self.read_table(reader, &element) {
                            blocks.push(Block::Table(table));
                        }
                    }
                    _ => {}
                },
                _ => {}
            }
        }

        // A continuation cell carries no content of its own.
        if vertical_merge == Some(false) && blocks.iter().all(Block::is_empty) {
            return None;
        }

        self.cell_style = outer_cell_style;
        Some(Cell {
            blocks,
            col_span,
            row_span: 1,
        })
    }
}

// ============================================================================
// Paragraph classification
// ============================================================================

struct Paragraph {
    content: Vec<Inline>,
    properties: ParagraphProperties,
}

/// Apply one `<w:rPr>` child to a text style.
///
/// A free function because the same properties appear in two places: on a run,
/// and in a style definition. Word puts a "Quote" paragraph's italic only in
/// the style, so a reader that looks at runs alone reports the quote as upright
/// -- the same document saved as .doc and .odt says italic.
fn apply_run_property(element: &Element, style: &mut TextStyle) {
    let on = is_on(element);
    match element.local.as_str() {
        "b" | "bCs" => style.bold = on,
        "i" | "iCs" => style.italic = on,
        "strike" | "dstrike" => style.strikethrough = on,
        "u" => style.underline = on && element.attr_local("val") != Some("none"),
        // Word's hidden-text property: invisible on the page, fully present in
        // the file.
        "vanish" | "webHidden" => style.hidden = on,
        "vertAlign" => match element.attr_local("val") {
            Some("superscript") => style.superscript = true,
            Some("subscript") => style.subscript = true,
            _ => {}
        },
        // Half-points, as everywhere in WordprocessingML.
        "sz" | "szCs" => style.size = attr_i64(element, "val").map(|v| v as f64 / 2.0),
        "rFonts" => {
            style.font = element
                .attr_local("ascii")
                .or(element.attr_local("hAnsi"))
                .filter(|name| !name.is_empty())
                .map(str::to_string);
        }
        "color" => style.color = parse_color(element.attr_local("val")),
        _ => {}
    }
}

#[derive(Debug, Default)]
struct ParagraphProperties {
    /// The style the paragraph names, so its runs can inherit the character
    /// formatting the style declares.
    style_id: String,
    heading: Option<u8>,
    numbering: Option<NumberingRef>,
    align: Align,
    indent: f64,
    quote: bool,
    page_break: bool,
}

#[derive(Debug, Default, Clone, Copy)]
struct NumberingRef {
    id: i64,
    level: u8,
    /// Resolved from `word/numbering.xml` once the paragraph is complete.
    ordered: bool,
}

/// Turn a paragraph into a block and file it, merging consecutive list items.
fn push_paragraph(blocks: &mut Vec<Block>, paragraph: Paragraph) {
    let Paragraph {
        content,
        properties,
    } = paragraph;

    if properties.page_break {
        blocks.push(Block::PageBreak);
        if inline_text(&content).trim().is_empty() {
            return;
        }
    }

    if let Some(level) = properties.heading {
        blocks.push(Block::Heading {
            level: level.clamp(1, 6),
            content,
        });
        return;
    }

    let block = Block::Paragraph {
        content,
        align: properties.align,
        indent: properties.indent,
    };

    if properties.quote {
        blocks.push(Block::Quote(vec![block]));
        return;
    }

    let Some(reference) = properties.numbering else {
        blocks.push(block);
        return;
    };

    // Consecutive numbered paragraphs at the same level form one list; a
    // different level nests inside the item before it.
    let item = ListItem {
        blocks: vec![block],
        checked: None,
    };
    crate::formats::append_list_item(blocks, item, reference.level, reference.ordered, 1);
}

/// Which of a table style's conditional formats apply to one cell.
///
/// Read from the `<w:cnfStyle>` Word writes on the row and on the cell, rather
/// than worked out from the cell's index: the producer already states it, and
/// banding in particular depends on settings this reader does not otherwise
/// need to track.
#[derive(Debug, Clone, Default)]
struct CellPosition {
    first_row: bool,
    last_row: bool,
    first_column: bool,
    last_column: bool,
    odd_h_band: bool,
    even_h_band: bool,
    odd_v_band: bool,
    even_v_band: bool,
}

impl CellPosition {
    /// Read the flags `<w:cnfStyle>` carries, keeping any already set: a cell's
    /// own element states its column, and its row's states the row.
    fn read(&mut self, element: &Element) {
        for (attribute, field) in [
            ("firstRow", &mut self.first_row),
            ("lastRow", &mut self.last_row),
            ("firstColumn", &mut self.first_column),
            ("lastColumn", &mut self.last_column),
            ("oddHBand", &mut self.odd_h_band),
            ("evenHBand", &mut self.even_h_band),
            ("oddVBand", &mut self.odd_v_band),
            ("evenVBand", &mut self.even_v_band),
        ] {
            *field |= matches!(element.attr_local(attribute), Some("1") | Some("true"));
        }
    }

    /// The conditional formats that apply, weakest first.
    fn conditionals(&self) -> Vec<&'static str> {
        let mut kinds = Vec::new();
        for (applies, name) in [
            (self.odd_v_band, "band1Vert"),
            (self.even_v_band, "band2Vert"),
            (self.odd_h_band, "band1Horz"),
            (self.even_h_band, "band2Horz"),
            (self.first_column, "firstCol"),
            (self.last_column, "lastCol"),
            (self.first_row, "firstRow"),
            (self.last_row, "lastRow"),
        ] {
            if applies {
                kinds.push(name);
            }
        }
        kinds
    }
}

// ============================================================================
// Styles and numbering
// ============================================================================

/// Style definitions from `word/styles.xml`.
#[derive(Debug, Default)]
struct Styles {
    /// Style id → nesting level for the built-in list styles.
    list_levels: HashMap<String, u8>,
    /// Style id → heading level (1-9).
    headings: HashMap<String, u8>,
    quotes: Vec<String>,
    code: Vec<String>,
    /// Style id → the numbering the *style* declares, for producers that put it
    /// there rather than on the paragraph. Word does: a "List Bullet"
    /// paragraph carries `<w:pStyle w:val="ListBullet"/>` and nothing else,
    /// and the `<w:numPr>` lives in the style definition.
    numbering: HashMap<String, NumberingRef>,
    /// Style id → the style it is based on, so inherited numbering is found.
    based_on: HashMap<String, String>,
    /// Style id → the character formatting the style itself declares.
    text: HashMap<String, TextStyle>,
    /// (table style id, conditional format) → the formatting it declares.
    ///
    /// A table style states its header row, its first column and its banding
    /// separately, in `<w:tblStylePr>`. Word resolves these when it writes any
    /// other format, so the .doc, .rtf and .odt of one document all reported a
    /// bold white header where the .docx reported nothing at all.
    conditional: HashMap<(String, String), TextStyle>,
}

impl Styles {
    fn read(archive: &ZipArchive, base: &str) -> Styles {
        let mut styles = Styles::default();
        let Some(bytes) = archive.read_optional(&resolve_path(base, "styles.xml")) else {
            return styles;
        };

        let mut reader = Reader::new(&bytes);
        let mut current: Option<String> = None;
        let mut conditional: Option<String> = None;
        while let Some(event) = reader.read_event() {
            let Event::Start(element) = event else {
                continue;
            };
            if !element.in_ns(ns::W) {
                continue;
            }
            match element.local.as_str() {
                "style" => {
                    current = element.attr_local("styleId").map(str::to_string);
                    conditional = None;
                }
                // Everything after this within the style belongs to one
                // conditional format rather than to the style itself. The
                // style's own properties come first, so this only ever
                // redirects what follows it.
                "tblStylePr" => conditional = element.attr_local("type").map(str::to_string),
                "basedOn" => {
                    if let (Some(id), Some(parent)) = (&current, element.attr_local("val")) {
                        styles.based_on.insert(id.clone(), parent.to_string());
                    }
                }
                // Inside a `<w:style>` these appear only under `<w:pPr><w:numPr>`,
                // so their presence is enough to attribute them to the style.
                "numId" => {
                    if let Some(id) = &current
                        && let Some(value) = attr_i64(&element, "val")
                    {
                        styles.numbering.entry(id.clone()).or_default().id = value;
                    }
                }
                "b" | "bCs" | "i" | "iCs" | "strike" | "dstrike" | "u" | "vanish" | "webHidden"
                | "vertAlign" | "sz" | "szCs" | "rFonts" | "color" => {
                    if let Some(id) = &current {
                        let into = match &conditional {
                            Some(kind) => styles
                                .conditional
                                .entry((id.clone(), kind.clone()))
                                .or_default(),
                            None => styles.text.entry(id.clone()).or_default(),
                        };
                        apply_run_property(&element, into);
                    }
                }
                "ilvl" => {
                    if let Some(id) = &current
                        && let Some(value) = attr_i64(&element, "val")
                    {
                        styles.numbering.entry(id.clone()).or_default().level =
                            value.clamp(0, 8) as u8;
                    }
                }
                // The human-readable name is more reliable than the id, which
                // is localised by some producers ("berschrift1" in German Word).
                "name" => {
                    if let (Some(id), Some(name)) = (&current, element.attr_local("val")) {
                        styles.classify(id, name);
                    }
                }
                _ => {}
            }
        }

        styles
    }

    fn classify(&mut self, id: &str, name: &str) {
        let lower = name.to_ascii_lowercase();
        let compact: String = lower.chars().filter(|c| !c.is_whitespace()).collect();

        if let Some(rest) = compact.strip_prefix("heading")
            && let Ok(level) = rest.parse::<u8>()
            && (1..=9).contains(&level)
        {
            self.headings.insert(id.to_string(), level);
            return;
        }
        if compact == "title" {
            self.headings.insert(id.to_string(), 1);
            return;
        }
        if compact == "subtitle" {
            self.headings.insert(id.to_string(), 2);
            return;
        }
        if let Some(level) = crate::formats::list_style_level(&compact) {
            self.list_levels.insert(id.to_string(), level);
            return;
        }
        if compact.contains("quote") {
            self.quotes.push(id.to_string());
            return;
        }
        if compact.contains("code") || compact.contains("htmlcode") {
            self.code.push(id.to_string());
        }
    }

    /// The formatting a table style gives a cell in the given position.
    ///
    /// Applied outermost first, so a header row's colour beats the banding
    /// underneath it and a direct run property beats them all.
    fn table_style(&self, id: &str, position: &CellPosition) -> TextStyle {
        let mut style = self.text.get(id).cloned().unwrap_or_default();
        for kind in position.conditionals() {
            if let Some(layer) = self.conditional.get(&(id.to_string(), kind.to_string())) {
                style = crate::doc::layer_style(&style, layer);
            }
        }
        style
    }

    fn heading_level(&self, id: &str) -> Option<u8> {
        if let Some(level) = self.headings.get(id) {
            return Some(*level);
        }
        // Fall back to the conventional id form when styles.xml is absent.
        let compact = id.to_ascii_lowercase();
        let rest = compact.strip_prefix("heading")?;
        rest.parse::<u8>().ok().filter(|l| (1..=9).contains(l))
    }

    /// The numbering a style carries, following `basedOn` where it does not
    /// carry one itself.
    ///
    /// A `numId` of zero is not "no numbering here" but "remove the numbering
    /// this style would otherwise inherit", which is why it stops the walk
    /// rather than continuing up the chain.
    fn numbering(&self, id: &str) -> Option<NumberingRef> {
        let mut reference = self.numbering_of(id)?;
        // Only where the style chain gave no level of its own: an explicit
        // `<w:ilvl>` says what it means and is not to be second-guessed.
        if reference.level == 0
            && let Some(level) = self.list_levels.get(id)
        {
            reference.level = *level;
        }
        Some(reference)
    }

    /// The numbering the style chain carries, before the name is consulted.
    fn numbering_of(&self, id: &str) -> Option<NumberingRef> {
        let mut at = id;
        // A cap rather than a visited set: a cycle is malformed input, and
        // eight levels is deeper than any real style chain.
        for _ in 0..8 {
            if let Some(reference) = self.numbering.get(at) {
                return match reference.id {
                    0 => None,
                    _ => Some(*reference),
                };
            }
            match self.based_on.get(at) {
                Some(parent) => at = parent,
                None => break,
            }
        }
        None
    }

    /// The character formatting a paragraph style implies, with `basedOn`
    /// ancestors applied first so the named style overrides them.
    fn text_style(&self, id: &str) -> TextStyle {
        let mut chain = vec![id];
        let mut at = id;
        for _ in 0..8 {
            match self.based_on.get(at) {
                Some(parent) => {
                    chain.push(parent);
                    at = parent;
                }
                None => break,
            }
        }
        let mut style = TextStyle::default();
        for name in chain.into_iter().rev() {
            if let Some(declared) = self.text.get(name) {
                style = declared.clone();
            }
        }
        style
    }

    fn is_quote(&self, id: &str) -> bool {
        self.quotes.iter().any(|s| s == id) || id.to_ascii_lowercase().contains("quote")
    }

    fn is_code(&self, id: &str) -> bool {
        self.code.iter().any(|s| s == id)
            || matches!(id.to_ascii_lowercase().as_str(), "code" | "htmlcode")
    }
}

/// Numbering definitions from `word/numbering.xml`.
///
/// A paragraph references a `numId`, which indirects through `<w:num>` to an
/// `<w:abstractNum>` holding the per-level formats. Without following that
/// chain every list looks like a bullet list.
#[derive(Debug, Default)]
struct Numbering {
    /// numId → abstractNumId.
    instances: HashMap<i64, i64>,
    /// (abstractNumId, level) → ordered?
    formats: HashMap<(i64, u8), bool>,
}

impl Numbering {
    fn read(archive: &ZipArchive, base: &str) -> Numbering {
        let mut numbering = Numbering::default();
        let Some(bytes) = archive.read_optional(&resolve_path(base, "numbering.xml")) else {
            return numbering;
        };

        let mut reader = Reader::new(&bytes);
        let mut abstract_id: Option<i64> = None;
        let mut level: Option<u8> = None;
        let mut instance_id: Option<i64> = None;

        while let Some(event) = reader.read_event() {
            let Event::Start(element) = event else {
                continue;
            };
            if !element.in_ns(ns::W) {
                continue;
            }
            match element.local.as_str() {
                "abstractNum" => {
                    abstract_id = attr_i64(&element, "abstractNumId");
                    instance_id = None;
                }
                "num" => {
                    instance_id = attr_i64(&element, "numId");
                    abstract_id = None;
                }
                "abstractNumId" => {
                    // Inside <w:num>, this is the pointer to the definition.
                    if let (Some(id), Some(target)) = (instance_id, attr_i64(&element, "val")) {
                        numbering.instances.insert(id, target);
                    }
                }
                "lvl" => level = attr_i64(&element, "ilvl").map(|v| v.clamp(0, 8) as u8),
                "numFmt" => {
                    if let (Some(id), Some(level)) = (abstract_id, level) {
                        let format = element.attr_local("val").unwrap_or("bullet");
                        numbering
                            .formats
                            .insert((id, level), !matches!(format, "bullet" | "none"));
                    }
                }
                _ => {}
            }
        }
        numbering
    }

    /// Whether `(numId, level)` is an ordered list.
    fn is_ordered(&self, num_id: i64, level: u8) -> bool {
        let Some(&abstract_id) = self.instances.get(&num_id) else {
            // Without a definition, assume bullets: a wrong bullet reads better
            // than invented numbering.
            return false;
        };
        self.formats
            .get(&(abstract_id, level))
            .copied()
            .unwrap_or(false)
    }
}

fn push_run(into: &mut Vec<Inline>, run: Run) {
    if let Some(Inline::Run(last)) = into.last_mut()
        && last.style == run.style
    {
        last.text.push_str(&run.text);
        return;
    }
    into.push(Inline::Run(run));
}

/// Parse a `RRGGBB` colour into 0.0-1.0 components.
fn parse_color(value: Option<&str>) -> Option<[f64; 3]> {
    let value = value?.trim().trim_start_matches('#');
    if value.len() != 6 || value.eq_ignore_ascii_case("auto") {
        return None;
    }
    let channel = |at: usize| u8::from_str_radix(&value[at..at + 2], 16).ok();
    Some([
        channel(0)? as f64 / 255.0,
        channel(2)? as f64 / 255.0,
        channel(4)? as f64 / 255.0,
    ])
}
