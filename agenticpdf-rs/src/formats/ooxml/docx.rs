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
    Align, Block, Cell, ImageRef, Inline, List, ListItem, Row, Run, SemanticDoc, Table, TextStyle,
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
    };

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
}

impl DocxReader<'_> {
    /// Read `<w:body>` into blocks.
    fn read_body(&mut self, xml: &[u8]) -> Vec<Block> {
        let mut reader = Reader::new(xml);
        let mut blocks: Vec<Block> = Vec::new();

        while let Some(event) = reader.read_event() {
            let Event::Start(element) = event else {
                continue;
            };
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
        }
        blocks
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
                        let inherited = self.styles.text_style(&properties.style_id);
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
                    "tc" => {
                        if let Some(cell) = self.read_cell(reader, &element) {
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

    fn read_cell(&mut self, reader: &mut Reader, start: &Element) -> Option<Cell> {
        let mut blocks: Vec<Block> = Vec::new();
        let mut col_span = 1usize;
        let mut vertical_merge: Option<bool> = None;
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
    append_list_item(blocks, item, reference.level, reference.ordered);
}

/// Append a list item at `level`, creating or nesting lists as needed.
fn append_list_item(blocks: &mut Vec<Block>, item: ListItem, level: u8, ordered: bool) {
    if level > 0
        && let Some(Block::List(list)) = blocks.last_mut()
        && let Some(last) = list.items.last_mut()
    {
        append_list_item(&mut last.blocks, item, level - 1, ordered);
        return;
    }

    if let Some(Block::List(list)) = blocks.last_mut()
        && list.ordered == ordered
    {
        list.items.push(item);
        return;
    }

    blocks.push(Block::List(List {
        ordered,
        start: 1,
        items: vec![item],
    }));
}

// ============================================================================
// Styles and numbering
// ============================================================================

/// Style definitions from `word/styles.xml`.
#[derive(Debug, Default)]
struct Styles {
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
}

impl Styles {
    fn read(archive: &ZipArchive, base: &str) -> Styles {
        let mut styles = Styles::default();
        let Some(bytes) = archive.read_optional(&resolve_path(base, "styles.xml")) else {
            return styles;
        };

        let mut reader = Reader::new(&bytes);
        let mut current: Option<String> = None;
        while let Some(event) = reader.read_event() {
            let Event::Start(element) = event else {
                continue;
            };
            if !element.in_ns(ns::W) {
                continue;
            }
            match element.local.as_str() {
                "style" => current = element.attr_local("styleId").map(str::to_string),
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
                        apply_run_property(&element, styles.text.entry(id.clone()).or_default());
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
        if compact.contains("quote") {
            self.quotes.push(id.to_string());
            return;
        }
        if compact.contains("code") || compact.contains("htmlcode") {
            self.code.push(id.to_string());
        }
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
