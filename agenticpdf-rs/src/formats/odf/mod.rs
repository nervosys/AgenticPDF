// SPDX-License-Identifier: AGPL-3.0-or-later
//! OpenDocument parsers (`.odt`, `.ods`, `.odp`).
//!
//! ODF is a ZIP of XML like OOXML, but organised the other way round. OOXML
//! splits a document across many parts wired together by relationships; ODF puts
//! essentially everything in one `content.xml` and describes formatting through
//! *named styles* rather than inline properties.
//!
//! That makes the style table the load-bearing part. A run is bold because it
//! carries `text:style-name="T3"` and `T3` is defined — usually in the same
//! file's `<office:automatic-styles>` — as having `fo:font-weight="bold"`.
//! Reading the content without the styles yields text with no formatting at
//! all, no headings distinguishable from body text, and no way to see hidden
//! text.
//!
//! This module holds that shared layer: the package, the style table, metadata,
//! and the text reader that `.odt` bodies and `.odp` text boxes both use.
//! [`sheet`] handles the spreadsheet grid, which is shaped differently enough
//! to be worth its own module.

pub mod sheet;

#[cfg(test)]
mod tests;

use std::collections::HashMap;

use crate::PdfError;
use crate::container::zip::ZipArchive;
use crate::detect::Format;
use crate::doc::{
    Align, Block, Cell, ImageRef, Inline, List, ListItem, PageSize, Row, Run, Section, SectionKind,
    SemanticDoc, Table, TextStyle, image_media_type, inline_text,
};
use crate::xml::{self, Element, Event, Reader, ns};

/// Cap on list and table nesting.
const MAX_DEPTH: usize = 64;

/// Parse an OpenDocument package.
pub fn parse(data: &[u8], format: Format) -> Result<SemanticDoc, PdfError> {
    let archive = ZipArchive::open(data)?;
    let content = archive.read("content.xml")?;

    // Styles live in two places: `content.xml` carries the automatic styles
    // generated for this document, `styles.xml` the named ones a user can pick.
    // Both are needed, and the automatic ones win.
    let mut styles = Styles::default();
    if let Some(bytes) = archive.read_optional("styles.xml") {
        styles.read(&bytes);
    }
    styles.read(&content);

    let mut document = read_metadata(&archive);
    let mut package = Package {
        archive: &archive,
        styles,
        images: HashMap::new(),
    };

    match format {
        Format::Odt => {
            // `<office:text>` is in the *office* namespace, not the text one —
            // the prefix and the local name deliberately disagree here.
            let blocks = read_body(
                &content,
                &mut package,
                &mut document,
                ns::ODF_OFFICE,
                "text",
            );
            document.sections = vec![Section {
                blocks,
                ..Section::default()
            }];
        }
        Format::Ods => sheet::read(&content, &mut document),
        Format::Odp => read_slides(&content, &mut package, &mut document),
        other => {
            return Err(PdfError::Unsupported(format!(
                "{} is not an OpenDocument format",
                other.label()
            )));
        }
    }

    if document.sections.is_empty() {
        document.sections.push(Section::default());
    }
    Ok(document)
}

/// The package a reader is working inside.
pub struct Package<'a> {
    archive: &'a ZipArchive<'a>,
    styles: Styles,
    /// Archive path → registered asset id, so an image used twice is stored once.
    images: HashMap<String, String>,
}

impl Package<'_> {
    /// Register a picture from the archive as an asset.
    fn register_image(&mut self, path: &str, document: &mut SemanticDoc) -> Option<String> {
        if let Some(id) = self.images.get(path) {
            return Some(id.clone());
        }
        let bytes = self.archive.read_optional(path)?;
        let media_type = image_media_type(&bytes).to_string();
        let reference = document.add_asset(media_type, bytes);
        self.images
            .insert(path.to_string(), reference.asset_id.clone());
        Some(reference.asset_id)
    }
}

// ============================================================================
// Metadata
// ============================================================================

/// Read `meta.xml` into an otherwise empty document.
fn read_metadata(archive: &ZipArchive) -> SemanticDoc {
    let mut document = SemanticDoc::default();
    let Some(bytes) = archive.read_optional("meta.xml") else {
        return document;
    };

    let mut reader = Reader::new(&bytes);
    while let Some(event) = reader.read_event() {
        let Event::Start(element) = event else {
            continue;
        };
        let slot = match (element.ns.as_str(), element.local.as_str()) {
            (ns::DC, "title") => &mut document.title,
            (ns::DC, "creator") => &mut document.author,
            (ns::DC, "subject") | (ns::DC, "description") => &mut document.subject,
            (ns::DC, "date") => &mut document.modified,
            (ns::ODF_META, "creation-date") => &mut document.created,
            (ns::ODF_META, "initial-creator") => &mut document.creator,
            _ => continue,
        };
        let value = xml::text_of(&mut reader, &element.qname).trim().to_string();
        if !value.is_empty() && slot.is_none() {
            *slot = Some(value);
        }
    }
    document
}

// ============================================================================
// Styles
// ============================================================================

/// The document's style table.
#[derive(Debug, Default)]
pub struct Styles {
    /// Character formatting by style name, for text and paragraph styles alike.
    text: HashMap<String, TextStyle>,
    /// Paragraph alignment by style name.
    align: HashMap<String, Align>,
    /// Heading level implied by a paragraph style's ancestry.
    headings: HashMap<String, u8>,
    /// Parent of each style, so properties can be inherited.
    parents: HashMap<String, String>,
    /// List style name → whether level 0 is numbered.
    ordered_lists: HashMap<String, bool>,
}

impl Styles {
    /// Read every `<style:style>` and `<text:list-style>` in a part.
    fn read(&mut self, xml_bytes: &[u8]) {
        let mut reader = Reader::new(xml_bytes);
        let mut current: Option<String> = None;
        let mut list_style: Option<String> = None;

        while let Some(event) = reader.read_event() {
            let Event::Start(element) = event else {
                continue;
            };

            match (element.ns.as_str(), element.local.as_str()) {
                (ns::ODF_STYLE, "style") => {
                    let name = element.attr_local("name").unwrap_or_default().to_string();
                    if name.is_empty() {
                        current = None;
                        continue;
                    }
                    if let Some(parent) = element.attr_local("parent-style-name") {
                        self.parents.insert(name.clone(), parent.to_string());
                        // ODF spells a style's display name with `_20_` for a
                        // space, so `Heading_20_1` is "Heading 1".
                        if let Some(level) = heading_level(parent) {
                            self.headings.insert(name.clone(), level);
                        }
                    }
                    if let Some(level) = heading_level(&name) {
                        self.headings.insert(name.clone(), level);
                    }
                    current = Some(name);
                }
                (ns::ODF_STYLE, "text-properties") => {
                    if let Some(name) = &current {
                        let entry = self.text.entry(name.clone()).or_default();
                        apply_text_properties(&element, entry);
                    }
                }
                (ns::ODF_STYLE, "paragraph-properties") => {
                    if let Some(name) = &current
                        && let Some(value) = element.attr_local("text-align")
                    {
                        self.align.insert(name.clone(), parse_align(value));
                    }
                }
                (ns::ODF_TEXT, "list-style") => {
                    list_style = element.attr_local("name").map(str::to_string);
                }
                // The level-1 style decides whether the list reads as numbered.
                (ns::ODF_TEXT, "list-level-style-number") => {
                    if let Some(name) = &list_style {
                        self.ordered_lists.entry(name.clone()).or_insert(true);
                    }
                }
                (ns::ODF_TEXT, "list-level-style-bullet") => {
                    if let Some(name) = &list_style {
                        self.ordered_lists.entry(name.clone()).or_insert(false);
                    }
                }
                _ => {}
            }
        }
    }

    /// Character formatting for a style, following its parent chain.
    fn text_style(&self, name: &str) -> TextStyle {
        let mut chain = Vec::new();
        let mut current = Some(name.to_string());
        let mut guard = 0;
        while let Some(step) = current {
            if guard > 16 {
                break;
            }
            guard += 1;
            chain.push(step.clone());
            current = self.parents.get(&step).cloned();
        }

        // Nearest style wins, so apply from the far end of the chain inwards.
        let mut style = TextStyle::default();
        for name in chain.iter().rev() {
            if let Some(defined) = self.text.get(name) {
                merge(&mut style, defined);
            }
        }
        style
    }

    fn alignment(&self, name: &str) -> Align {
        self.align.get(name).copied().unwrap_or_default()
    }

    fn heading(&self, name: &str) -> Option<u8> {
        self.headings.get(name).copied()
    }

    fn is_ordered(&self, name: &str) -> bool {
        self.ordered_lists.get(name).copied().unwrap_or(false)
    }

    /// Whether a paragraph style is a block quote, following its ancestry.
    ///
    /// Word writes a quoted paragraph as `text:style-name="Quote"` and nothing
    /// else, so without this it reads as body text that happens to be italic —
    /// which is what the same document saved as .docx and .doc did not say.
    /// LibreOffice calls its own "Quotations", hence both spellings.
    fn is_quote(&self, name: &str) -> bool {
        let mut at = name;
        // A cap rather than a visited set: a cycle is malformed input.
        for _ in 0..8 {
            let decoded = at.replace("_20_", " ").to_ascii_lowercase();
            if decoded.contains("quote") || decoded.contains("quotation") {
                return true;
            }
            match self.parents.get(at) {
                Some(parent) => at = parent,
                None => break,
            }
        }
        false
    }
}

/// Overlay one style's stated properties onto another.
fn merge(into: &mut TextStyle, from: &TextStyle) {
    into.bold |= from.bold;
    into.italic |= from.italic;
    into.underline |= from.underline;
    into.strikethrough |= from.strikethrough;
    into.superscript |= from.superscript;
    into.subscript |= from.subscript;
    into.hidden |= from.hidden;
    if from.font.is_some() {
        into.font = from.font.clone();
    }
    if from.size.is_some() {
        into.size = from.size;
    }
    if from.color.is_some() {
        into.color = from.color;
    }
}

/// Read `<style:text-properties>` into a character style.
fn apply_text_properties(element: &Element, style: &mut TextStyle) {
    if let Some(weight) = element.attr_local("font-weight") {
        style.bold = weight != "normal";
    }
    if let Some(slope) = element.attr_local("font-style") {
        style.italic = matches!(slope, "italic" | "oblique");
    }
    if let Some(value) = element.attr_local("text-underline-style") {
        style.underline = value != "none";
    }
    if let Some(value) = element.attr_local("text-line-through-style") {
        style.strikethrough = value != "none";
    }
    match element.attr_local("text-position") {
        Some(value) if value.starts_with("super") => style.superscript = true,
        Some(value) if value.starts_with("sub") => style.subscript = true,
        _ => {}
    }
    // ODF's hidden text: rendered nowhere, extracted everywhere. The same
    // prompt-injection vector as Word's `w:vanish`.
    if element.attr_local("display") == Some("none") {
        style.hidden = true;
    }
    if let Some(name) = element.attr_local("font-name").filter(|n| !n.is_empty()) {
        style.font = Some(name.to_string());
    }
    if let Some(size) = element.attr_local("font-size").and_then(parse_length) {
        style.size = Some(size);
    }
    if let Some(color) = element.attr_local("color").and_then(parse_color) {
        style.color = Some(color);
    }
}

/// Heading level from an ODF style name such as `Heading_20_2`.
fn heading_level(name: &str) -> Option<u8> {
    let decoded = name.replace("_20_", " ").to_ascii_lowercase();
    let rest = decoded.strip_prefix("heading")?;
    let level: u8 = rest.trim().parse().ok()?;
    (1..=9).contains(&level).then_some(level)
}

fn parse_align(value: &str) -> Align {
    match value {
        "center" => Align::Center,
        "end" | "right" => Align::Right,
        "justify" => Align::Justify,
        _ => Align::Left,
    }
}

/// Parse an ODF length into points.
fn parse_length(value: &str) -> Option<f64> {
    let value = value.trim();
    let split = value.find(|c: char| c.is_alphabetic() || c == '%')?;
    let number: f64 = value[..split].parse().ok()?;
    match &value[split..] {
        "pt" => Some(number),
        "in" => Some(number * 72.0),
        "cm" => Some(number * 72.0 / 2.54),
        "mm" => Some(number * 72.0 / 25.4),
        "px" => Some(number * 72.0 / 96.0),
        // A percentage is relative to something this layer does not know.
        _ => None,
    }
}

fn parse_color(value: &str) -> Option<[f64; 3]> {
    let hex = value.trim().strip_prefix('#')?;
    if hex.len() != 6 {
        return None;
    }
    let channel = |at: usize| u8::from_str_radix(&hex[at..at + 2], 16).ok();
    Some([
        channel(0)? as f64 / 255.0,
        channel(2)? as f64 / 255.0,
        channel(4)? as f64 / 255.0,
    ])
}

// ============================================================================
// Shared content reading
// ============================================================================

/// Read a document body, stopping at the close of `root` in `namespace`.
fn read_body(
    content: &[u8],
    package: &mut Package,
    document: &mut SemanticDoc,
    namespace: &str,
    root: &str,
) -> Vec<Block> {
    let mut reader = Reader::new(content);
    while let Some(event) = reader.read_event() {
        if let Event::Start(element) = event
            && element.is(namespace, root)
        {
            return read_blocks(&mut reader, &element.qname, package, document, 0);
        }
    }
    Vec::new()
}

/// Read block-level content until `closer` closes.
fn read_blocks(
    reader: &mut Reader,
    closer: &str,
    package: &mut Package,
    document: &mut SemanticDoc,
    depth: usize,
) -> Vec<Block> {
    let mut blocks = Vec::new();
    if depth > MAX_DEPTH {
        return blocks;
    }

    while let Some(event) = reader.read_event() {
        match event {
            Event::End(name) if name == closer => break,
            Event::Start(element) => match (element.ns.as_str(), element.local.as_str()) {
                (ns::ODF_TEXT, "h") => {
                    // The outline level is stated on the element itself, which
                    // is more reliable than deducing it from the style name.
                    let level = element
                        .attr_local("outline-level")
                        .and_then(|value| value.parse::<u8>().ok())
                        .or_else(|| {
                            element
                                .attr_local("style-name")
                                .and_then(|name| package.styles.heading(name))
                        })
                        .unwrap_or(1);
                    let content = read_inlines(reader, &element, package, document);
                    if !inline_text(&content).trim().is_empty() {
                        blocks.push(Block::Heading {
                            level: level.clamp(1, 6),
                            content,
                        });
                    }
                }
                (ns::ODF_TEXT, "p") => {
                    let style = element
                        .attr_local("style-name")
                        .unwrap_or_default()
                        .to_string();
                    let content = read_inlines(reader, &element, package, document);
                    if inline_text(&content).trim().is_empty() {
                        continue;
                    }
                    // A paragraph whose style descends from a heading style is
                    // a heading, even though it is not a `<text:h>`.
                    match package.styles.heading(&style) {
                        Some(level) => blocks.push(Block::Heading {
                            level: level.clamp(1, 6),
                            content,
                        }),
                        None if package.styles.is_quote(&style) => {
                            blocks.push(Block::Quote(vec![Block::Paragraph {
                                content,
                                align: package.styles.alignment(&style),
                                indent: 0.0,
                            }]))
                        }
                        None => blocks.push(Block::Paragraph {
                            content,
                            align: package.styles.alignment(&style),
                            indent: 0.0,
                        }),
                    }
                }
                (ns::ODF_TEXT, "list") => {
                    let list = read_list(reader, &element, package, document, depth + 1);
                    if list.items.is_empty() {
                        continue;
                    }
                    // Producers routinely emit one `<text:list>` per item rather
                    // than one per list — Word's ODF export does. Merging the
                    // consecutive ones keeps a bulleted run a single list
                    // instead of a stack of one-item lists with gaps between.
                    match blocks.last_mut() {
                        Some(Block::List(previous)) if previous.ordered == list.ordered => {
                            previous.items.extend(list.items);
                        }
                        _ => blocks.push(Block::List(list)),
                    }
                }
                (ns::ODF_TABLE, "table") => {
                    if let Some(table) = read_table(reader, &element, package, document, depth + 1)
                    {
                        blocks.push(Block::Table(table));
                    }
                }
                // A frame at block level may wrap a picture *or* a text box —
                // slide notes and callouts are text boxes, and reading only the
                // picture case silently drops their content.
                (ns::ODF_DRAW, "frame") => {
                    blocks.extend(read_frame_blocks(reader, &element, package, document));
                }
                _ => {}
            },
            _ => {}
        }
    }
    blocks
}

/// Read a `<text:list>`.
fn read_list(
    reader: &mut Reader,
    start: &Element,
    package: &mut Package,
    document: &mut SemanticDoc,
    depth: usize,
) -> List {
    let ordered = start
        .attr_local("style-name")
        .map(|name| package.styles.is_ordered(name))
        .unwrap_or(false);
    let mut items = Vec::new();
    let mut nesting = 1usize;

    while let Some(event) = reader.read_event() {
        match event {
            Event::End(name) if name == start.qname => {
                nesting -= 1;
                if nesting == 0 {
                    break;
                }
            }
            Event::Start(element) if element.qname == start.qname => nesting += 1,
            Event::Start(element) if element.is(ns::ODF_TEXT, "list-item") => {
                let blocks = read_blocks(reader, &element.qname, package, document, depth);
                if !blocks.is_empty() {
                    items.push(ListItem {
                        blocks,
                        checked: None,
                    });
                }
            }
            _ => {}
        }
    }

    List {
        ordered,
        start: 1,
        items,
    }
}

/// Read a `<table:table>` from a text or presentation document.
fn read_table(
    reader: &mut Reader,
    start: &Element,
    package: &mut Package,
    document: &mut SemanticDoc,
    depth: usize,
) -> Option<Table> {
    let mut rows: Vec<Row> = Vec::new();
    let mut header_rows = 0usize;
    let mut in_header = false;
    let mut nesting = 1usize;

    while let Some(event) = reader.read_event() {
        match event {
            Event::End(name) if name == start.qname => {
                nesting -= 1;
                if nesting == 0 {
                    break;
                }
            }
            Event::Start(element) if element.qname == start.qname => nesting += 1,
            Event::End(name) if name.ends_with("table-header-rows") => in_header = false,
            Event::Start(element) if element.in_ns(ns::ODF_TABLE) => {
                match element.local.as_str() {
                    // ODF marks header rows by wrapping them, rather than by
                    // flagging each one.
                    "table-header-rows" => in_header = true,
                    "table-row" => {
                        let row = read_row(reader, &element, package, document, depth);
                        if !row.cells.is_empty() {
                            if in_header && header_rows == rows.len() {
                                header_rows += 1;
                            }
                            rows.push(row);
                        }
                    }
                    _ => {}
                }
            }
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
        column_widths: Vec::new(),
    })
}

fn read_row(
    reader: &mut Reader,
    start: &Element,
    package: &mut Package,
    document: &mut SemanticDoc,
    depth: usize,
) -> Row {
    let mut cells = Vec::new();

    while let Some(event) = reader.read_event() {
        match event {
            Event::End(name) if name == start.qname => break,
            Event::Start(element) if element.is(ns::ODF_TABLE, "table-cell") => {
                let span = element
                    .attr_local("number-columns-spanned")
                    .and_then(|value| value.parse::<usize>().ok())
                    .unwrap_or(1)
                    .clamp(1, 1000);
                let repeat = element
                    .attr_local("number-columns-repeated")
                    .and_then(|value| value.parse::<usize>().ok())
                    .unwrap_or(1)
                    .clamp(1, 1000);
                let blocks = read_blocks(reader, &element.qname, package, document, depth);

                for _ in 0..repeat {
                    cells.push(Cell {
                        blocks: blocks.clone(),
                        col_span: span,
                        row_span: 1,
                    });
                }
            }
            // A covered cell is the continuation of a span; it holds nothing.
            Event::Start(element) if element.is(ns::ODF_TABLE, "covered-table-cell") => {}
            _ => {}
        }
    }

    Row { cells }
}

/// Read the inline content of a paragraph or heading.
fn read_inlines(
    reader: &mut Reader,
    start: &Element,
    package: &mut Package,
    document: &mut SemanticDoc,
) -> Vec<Inline> {
    let base = start
        .attr_local("style-name")
        .map(|name| package.styles.text_style(name))
        .unwrap_or_default();
    let mut content = Vec::new();
    read_inline_run(
        reader,
        &start.qname,
        &base,
        package,
        document,
        &mut content,
        0,
    );
    content
}

/// Read inline content until `closer`, carrying `inherited` formatting down.
#[allow(clippy::too_many_arguments)]
fn read_inline_run(
    reader: &mut Reader,
    closer: &str,
    inherited: &TextStyle,
    package: &mut Package,
    document: &mut SemanticDoc,
    into: &mut Vec<Inline>,
    depth: usize,
) {
    if depth > MAX_DEPTH {
        return;
    }

    while let Some(event) = reader.read_event() {
        match event {
            Event::End(name) if name == closer => break,
            Event::Text(text) => {
                if !text.is_empty() {
                    push_run(into, Run::styled(text, inherited.clone()));
                }
            }
            Event::Start(element) => match (element.ns.as_str(), element.local.as_str()) {
                (ns::ODF_TEXT, "span") => {
                    let mut style = inherited.clone();
                    if let Some(name) = element.attr_local("style-name") {
                        merge(&mut style, &package.styles.text_style(name));
                    }
                    read_inline_run(
                        reader,
                        &element.qname,
                        &style,
                        package,
                        document,
                        into,
                        depth + 1,
                    );
                }
                (ns::ODF_TEXT, "a") => {
                    let href = element.attr_local("href").unwrap_or_default().to_string();
                    let mut inner = Vec::new();
                    read_inline_run(
                        reader,
                        &element.qname,
                        inherited,
                        package,
                        document,
                        &mut inner,
                        depth + 1,
                    );
                    let runs: Vec<Run> = inner
                        .into_iter()
                        .filter_map(|item| match item {
                            Inline::Run(run) => Some(run),
                            _ => None,
                        })
                        .collect();
                    if runs.is_empty() {
                        continue;
                    }
                    if href.is_empty() {
                        into.extend(runs.into_iter().map(Inline::Run));
                    } else {
                        into.push(Inline::Link { href, runs });
                    }
                }
                // ODF encodes runs of spaces explicitly, because XML would
                // otherwise let them collapse.
                (ns::ODF_TEXT, "s") => {
                    let count = element
                        .attr_local("c")
                        .and_then(|value| value.parse::<usize>().ok())
                        .unwrap_or(1)
                        .clamp(1, 4096);
                    push_run(into, Run::styled(" ".repeat(count), inherited.clone()));
                }
                (ns::ODF_TEXT, "tab") => push_run(into, Run::styled("\t", inherited.clone())),
                (ns::ODF_TEXT, "line-break") => into.push(Inline::Break),
                (ns::ODF_DRAW, "frame") => {
                    if let Some(image) = read_frame(reader, &element, package, document) {
                        into.push(Inline::Image(image));
                    }
                }
                // Notes and annotations are commentary, not body text; their
                // content would otherwise be spliced into the sentence.
                (ns::ODF_TEXT, "note") | (ns::ODF_OFFICE, "annotation") => {
                    let _ = xml::text_of(reader, &element.qname);
                }
                _ => {}
            },
            _ => {}
        }
    }
}

/// Read a `<draw:frame>`, registering the picture it wraps.
fn read_frame(
    reader: &mut Reader,
    start: &Element,
    package: &mut Package,
    document: &mut SemanticDoc,
) -> Option<ImageRef> {
    let width = start.attr_local("width").and_then(parse_length);
    let height = start.attr_local("height").and_then(parse_length);
    let alt_from_name = start
        .attr_local("name")
        .filter(|name| !name.trim().is_empty())
        .map(str::to_string);

    let mut href: Option<String> = None;
    let mut alt: Option<String> = None;
    let mut nesting = 1usize;

    while let Some(event) = reader.read_event() {
        match event {
            Event::End(name) if name == start.qname => {
                nesting -= 1;
                if nesting == 0 {
                    break;
                }
            }
            Event::Start(element) if element.qname == start.qname => nesting += 1,
            Event::Start(element) => match (element.ns.as_str(), element.local.as_str()) {
                (ns::ODF_DRAW, "image") => {
                    href = element
                        .attr_local("href")
                        .filter(|value| !value.is_empty())
                        .map(|value| value.trim_start_matches("./").to_string());
                }
                (ns::ODF_TEXT, "desc") | (ns::SVG, "title") | (ns::SVG, "desc") => {
                    let text = xml::text_of(reader, &element.qname).trim().to_string();
                    if !text.is_empty() {
                        alt = Some(text);
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }

    let asset_id = package.register_image(&href?, document)?;
    Some(ImageRef {
        asset_id,
        alt: alt.or(alt_from_name),
        width,
        height,
    })
}

fn push_run(into: &mut Vec<Inline>, run: Run) {
    if run.text.is_empty() {
        return;
    }
    if let Some(Inline::Run(last)) = into.last_mut()
        && last.style == run.style
    {
        last.text.push_str(&run.text);
        return;
    }
    into.push(Inline::Run(run));
}

// ============================================================================
// Presentations
// ============================================================================

/// Read `<office:presentation>`: one section per `<draw:page>`.
fn read_slides(content: &[u8], package: &mut Package, document: &mut SemanticDoc) {
    let page_size = read_slide_size(content);
    let mut reader = Reader::new(content);

    while let Some(event) = reader.read_event() {
        let Event::Start(element) = event else {
            continue;
        };
        if !element.is(ns::ODF_DRAW, "page") {
            continue;
        }

        let name = element
            .attr_local("name")
            .filter(|value| !value.trim().is_empty())
            .map(str::to_string);
        let (title, blocks, notes) = read_slide(&mut reader, &element, package, document);

        document.sections.push(Section {
            kind: SectionKind::Slide,
            title: title.or(name),
            blocks,
            notes,
            page_size,
        });
    }
}

/// Read one slide, separating its title placeholder and its notes.
fn read_slide(
    reader: &mut Reader,
    start: &Element,
    package: &mut Package,
    document: &mut SemanticDoc,
) -> (Option<String>, Vec<Block>, Vec<Block>) {
    let mut title: Option<String> = None;
    let mut blocks: Vec<Block> = Vec::new();
    let mut notes: Vec<Block> = Vec::new();
    let mut nesting = 1usize;

    while let Some(event) = reader.read_event() {
        match event {
            Event::End(name) if name == start.qname => {
                nesting -= 1;
                if nesting == 0 {
                    break;
                }
            }
            Event::Start(element) if element.qname == start.qname => nesting += 1,
            Event::Start(element) if element.is(ns::ODF_PRESENTATION, "notes") => {
                notes = read_blocks(reader, &element.qname, package, document, 1);
            }
            Event::Start(element) if element.is(ns::ODF_DRAW, "frame") => {
                // `presentation:class` says what role the frame plays.
                let class = element.attr_local("class").unwrap_or_default().to_string();
                let content = read_frame_blocks(reader, &element, package, document);

                match class.as_str() {
                    "title" | "subtitle" if title.is_none() => {
                        let text = content
                            .iter()
                            .map(|block| {
                                let mut out = String::new();
                                crate::doc::block_text_into(block, &mut out);
                                out
                            })
                            .collect::<String>();
                        if text.trim().is_empty() {
                            blocks.extend(content);
                        } else {
                            title = Some(text.trim().to_string());
                        }
                    }
                    // Page numbers, dates and footers repeat on every slide.
                    "page-number" | "date-time" | "footer" | "header" => {}
                    _ => blocks.extend(content),
                }
            }
            _ => {}
        }
    }

    (title, blocks, notes)
}

/// Read a slide frame's content: either a text box or a picture.
fn read_frame_blocks(
    reader: &mut Reader,
    start: &Element,
    package: &mut Package,
    document: &mut SemanticDoc,
) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut images: Vec<ImageRef> = Vec::new();
    // The description may follow the image inside the frame, so alt text is
    // collected across the whole frame and applied at the end.
    let mut alt: Option<String> = None;
    let mut nesting = 1usize;

    while let Some(event) = reader.read_event() {
        match event {
            Event::End(name) if name == start.qname => {
                nesting -= 1;
                if nesting == 0 {
                    break;
                }
            }
            Event::Start(element) if element.qname == start.qname => nesting += 1,
            Event::Start(element) if element.is(ns::ODF_DRAW, "text-box") => {
                blocks.extend(read_blocks(reader, &element.qname, package, document, 1));
            }
            Event::Start(element) if element.is(ns::ODF_DRAW, "image") => {
                let href = element
                    .attr_local("href")
                    .map(|value| value.trim_start_matches("./").to_string());
                if let Some(href) = href
                    && let Some(asset_id) = package.register_image(&href, document)
                {
                    images.push(ImageRef {
                        asset_id,
                        alt: None,
                        width: start.attr_local("width").and_then(parse_length),
                        height: start.attr_local("height").and_then(parse_length),
                    });
                }
            }
            Event::Start(element)
                if element.is(ns::ODF_TEXT, "desc")
                    || element.is(ns::SVG, "title")
                    || element.is(ns::SVG, "desc") =>
            {
                let text = xml::text_of(reader, &element.qname).trim().to_string();
                if !text.is_empty() {
                    alt = Some(text);
                }
            }
            _ => {}
        }
    }

    // A frame's `draw:name` is the last resort for alt text: it is the label a
    // user sees in the application, so it is more useful than nothing.
    let fallback = start
        .attr_local("name")
        .filter(|name| !name.trim().is_empty())
        .map(str::to_string);
    for mut image in images {
        image.alt = alt.clone().or_else(|| fallback.clone());
        blocks.push(Block::Figure {
            image,
            caption: None,
        });
    }

    blocks
}

/// Read the slide dimensions from the master page's layout.
fn read_slide_size(content: &[u8]) -> Option<PageSize> {
    let mut reader = Reader::new(content);
    while let Some(event) = reader.read_event() {
        let Event::Start(element) = event else {
            continue;
        };
        if !element.is(ns::ODF_STYLE, "page-layout-properties") {
            continue;
        }
        let width = element.attr_local("page-width").and_then(parse_length)?;
        let height = element.attr_local("page-height").and_then(parse_length)?;
        // As with PowerPoint, the semantic model keeps reading order rather
        // than shape coordinates, so flowed content needs an inset.
        let inset = (width.min(height) * 0.05).max(18.0);
        return Some(PageSize {
            width,
            height,
            margin_left: inset,
            margin_right: inset,
            margin_top: inset,
            margin_bottom: inset,
        });
    }
    None
}
