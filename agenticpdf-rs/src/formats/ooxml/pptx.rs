// SPDX-License-Identifier: AGPL-3.0-or-later
//! PresentationML (`.pptx`) reader.
//!
//! Each slide becomes one [`Section`], with its title placeholder promoted to
//! the section title and its speaker notes kept separately. That structure is
//! what makes a deck useful to an agent: "which slide said this" survives into
//! Markdown, chunk provenance and the structure tree.
//!
//! Two details drive the shape of this reader:
//!
//! - **Slide order comes from `<p:sldIdLst>`, not from part names.** Reordering
//!   slides in PowerPoint rewrites that list and leaves `slide1.xml`,
//!   `slide2.xml` where they are, so reading parts by name gives the *original*
//!   order rather than the current one.
//! - **A slide's text is in DrawingML, not PresentationML.** The shapes are
//!   `<p:sp>` but the text inside them is `<a:t>` in the drawing namespace —
//!   the same namespace a chart or a diagram uses. Matching local names alone
//!   would confuse `<a:t>` with `<w:t>`, which is why the reader matches on
//!   resolved namespaces.
//!
//! Unlike a word processor document, a slide carries real geometry: shapes have
//! explicit positions and the deck has a page size. Those are read here so the
//! typesetter can honour them rather than reflowing a layout the author placed
//! by hand.

use std::collections::HashMap;

use crate::PdfError;
use crate::container::zip::ZipArchive;
use crate::doc::{
    Align, Block, Cell, ImageRef, Inline, List, ListItem, PageSize, Row, Run, Section, SectionKind,
    SemanticDoc, Table, TextStyle, image_media_type, inline_text,
};
use crate::formats::ooxml::{Package, Rels, attr_i64, emu_to_points, rel_id};
use crate::xml::{Element, Event, Reader, ns};

/// Cap on slides read, bounding a corrupt slide list.
const MAX_SLIDES: usize = 5_000;

/// Parse a PresentationML package.
pub fn parse(archive: &ZipArchive, package: &Package) -> Result<SemanticDoc, PdfError> {
    let presentation = archive.read(&package.main_part)?;
    let base = package.base();

    let (slide_ids, page_size) = read_presentation(&presentation);
    let mut document = SemanticDoc::default();
    let mut images: HashMap<String, String> = HashMap::new();

    for relationship in slide_ids.into_iter().take(MAX_SLIDES) {
        let Some(path) = package.main_rels.resolve(&base, &relationship) else {
            continue;
        };
        let Some(bytes) = archive.read_optional(&path) else {
            continue;
        };

        let slide_rels = Rels::for_part(archive, &path);
        let slide_base = crate::formats::ooxml::split_path(&path).0;

        let mut slide = SlideReader {
            archive,
            rels: &slide_rels,
            base: slide_base,
            document: &mut document,
            images: &mut images,
        };
        let (title, blocks) = slide.read_slide(&bytes, false);

        // Notes live in a separate part, reachable only through this slide's
        // own relationships.
        let notes = slide_rels
            .find("notesSlide")
            .and_then(|rel| {
                let notes_path = crate::formats::ooxml::resolve_path(&slide.base, &rel.target);
                archive.read_optional(&notes_path)
            })
            .map(|bytes| {
                let mut reader = SlideReader {
                    archive,
                    rels: &slide_rels,
                    base: crate::formats::ooxml::split_path(&path).0,
                    document: &mut document,
                    images: &mut images,
                };
                reader.read_slide(&bytes, true).1
            })
            .unwrap_or_default();

        document.sections.push(Section {
            kind: SectionKind::Slide,
            title,
            blocks,
            notes,
            page_size,
        });
    }

    if document.sections.is_empty() {
        return Err(PdfError::MissingPart("pptx contains no slides".into()));
    }
    Ok(document)
}

/// Read the slide list and page size from `ppt/presentation.xml`.
fn read_presentation(xml: &[u8]) -> (Vec<String>, Option<PageSize>) {
    let mut slides = Vec::new();
    let mut page_size = None;

    let mut reader = Reader::new(xml);
    while let Some(event) = reader.read_event() {
        let Event::Start(element) = event else {
            continue;
        };
        match element.local.as_str() {
            // The presentation order, which need not match the part names.
            "sldId" => {
                if let Some(relationship) = rel_id(&element) {
                    slides.push(relationship.to_string());
                }
            }
            "sldSz" => {
                let width = attr_i64(&element, "cx").map(emu_to_points).unwrap_or(720.0);
                let height = attr_i64(&element, "cy").map(emu_to_points).unwrap_or(540.0);
                // PowerPoint positions each shape absolutely, but the semantic
                // model keeps a slide's reading order rather than its shape
                // coordinates, so the typesetter flows the content instead.
                // Flowed content needs an inset: PowerPoint's own content
                // placeholders sit about 5% in from each edge, and without one
                // the first line would touch the very top of the slide.
                let inset = (width.min(height) * 0.05).max(18.0);
                page_size = Some(PageSize {
                    width,
                    height,
                    margin_left: inset,
                    margin_right: inset,
                    margin_top: inset,
                    margin_bottom: inset,
                });
            }
            _ => {}
        }
    }
    (slides, page_size)
}

struct SlideReader<'a> {
    archive: &'a ZipArchive<'a>,
    rels: &'a Rels,
    base: String,
    document: &'a mut SemanticDoc,
    images: &'a mut HashMap<String, String>,
}

impl SlideReader<'_> {
    /// Read one slide part, returning its title and body blocks.
    ///
    /// `notes` selects the notes-page reading: its paragraphs are prose rather
    /// than the bulleted lists a slide body is.
    fn read_slide(&mut self, xml: &[u8], notes: bool) -> (Option<String>, Vec<Block>) {
        let mut reader = Reader::new(xml);
        let mut title: Option<String> = None;
        let mut blocks: Vec<Block> = Vec::new();

        while let Some(event) = reader.read_event() {
            let Event::Start(element) = event else {
                continue;
            };
            match (element.ns.as_str(), element.local.as_str()) {
                (ns::P, "sp") => {
                    let shape = self.read_shape(&mut reader, &element, notes);
                    if shape.blocks.is_empty() || shape.is_furniture {
                        continue;
                    }
                    // The title placeholder names the slide; everything else is
                    // body content.
                    if shape.is_title && title.is_none() {
                        let text = shape
                            .blocks
                            .iter()
                            .filter_map(|block| match block {
                                Block::Paragraph { content, .. }
                                | Block::Heading { content, .. } => Some(inline_text(content)),
                                _ => None,
                            })
                            .collect::<Vec<_>>()
                            .join(" ");
                        if !text.trim().is_empty() {
                            title = Some(text.trim().to_string());
                            continue;
                        }
                    }
                    blocks.extend(shape.blocks);
                }
                // A table lives inside a `<p:graphicFrame>` rather than a
                // shape, so a walk that knows only `<p:sp>` and `<p:pic>` drops
                // it silently -- a slide of figures came back empty.
                (ns::P, "graphicFrame") => {
                    if let Some(table) = self.read_graphic_frame(&mut reader, &element) {
                        blocks.push(Block::Table(table));
                    }
                }
                (ns::P, "pic") => {
                    if let Some(image) = self.read_picture(&mut reader, &element) {
                        blocks.push(Block::Figure {
                            image,
                            caption: None,
                        });
                    }
                }
                _ => {}
            }
        }
        (title, blocks)
    }

    /// Read one `<p:sp>` shape.
    fn read_shape(&mut self, reader: &mut Reader, start: &Element, notes: bool) -> Shape {
        let mut shape = Shape::default();
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
                Event::Start(element) => match (element.ns.as_str(), element.local.as_str()) {
                    // `<p:ph>` identifies what role this shape plays.
                    (ns::P, "ph") => match element.attr_local("type") {
                        Some("title") | Some("ctrTitle") => shape.is_title = true,
                        // Slide numbers, dates and footers are page furniture
                        // that PowerPoint repeats on every slide. Emitting them
                        // puts a stray "1" in the middle of the content.
                        Some("sldNum") | Some("dt") | Some("ftr") => shape.is_furniture = true,
                        _ => {}
                    },
                    (ns::A, "p") => {
                        if let Some(mut paragraph) = self.read_paragraph(reader, &element) {
                            // A title is a heading, not a list item, whatever
                            // the placeholder's inherited bullet says. The
                            // placeholder is declared before the text body, so
                            // this is known by the time the paragraph arrives.
                            //
                            // Notes are prose: PowerPoint shows them as
                            // paragraphs, not as the bulleted list a slide body
                            // is.
                            if shape.is_title || notes {
                                paragraph.bullet = Bullet::None;
                            }
                            push_paragraph(&mut shape.blocks, paragraph);
                        }
                    }
                    _ => {}
                },
                _ => {}
            }
        }
        shape
    }

    /// Read a `<p:graphicFrame>`, returning its table if it holds one.
    ///
    /// A frame can also carry a chart or a diagram; those have no text to
    /// recover here and are skipped, which is why this returns an option
    /// rather than an empty table.
    fn read_graphic_frame(&mut self, reader: &mut Reader, start: &Element) -> Option<Table> {
        let mut rows: Vec<Row> = Vec::new();
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
                Event::Start(element) if element.is(ns::A, "tr") => {
                    rows.push(self.read_table_row(reader, &element));
                }
                _ => {}
            }
        }

        match rows.iter().all(|row| row.cells.is_empty()) {
            true => None,
            false => Some(Table {
                rows,
                // DrawingML marks a header row in the table style rather than
                // on the row, and the first row of a slide table is one in
                // every deck anybody writes.
                header_rows: 1,
                caption: None,
                // The frame's `<a:gridCol>` widths are in EMU and the layout
                // computes its own; carrying them would only be a second
                // opinion nothing reads.
                column_widths: Vec::new(),
            }),
        }
    }

    /// Read one `<a:tr>`, whose cells hold text bodies like any shape's.
    fn read_table_row(&mut self, reader: &mut Reader, start: &Element) -> Row {
        let mut cells: Vec<Cell> = Vec::new();
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
                Event::Start(element) if element.is(ns::A, "tc") => {
                    cells.push(self.read_table_cell(reader, &element));
                }
                _ => {}
            }
        }
        Row { cells }
    }

    /// Read one `<a:tc>`: the same paragraphs a shape holds, without bullets.
    fn read_table_cell(&mut self, reader: &mut Reader, start: &Element) -> Cell {
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
                Event::Start(element) if element.is(ns::A, "p") => {
                    if let Some(mut paragraph) = self.read_paragraph(reader, &element) {
                        // A cell's contents are its value, never a list item.
                        paragraph.bullet = Bullet::None;
                        paragraph.level = 0;
                        push_paragraph(&mut blocks, paragraph);
                    }
                }
                _ => {}
            }
        }
        Cell {
            blocks,
            ..Cell::default()
        }
    }

    /// Read one DrawingML `<a:p>` paragraph.
    fn read_paragraph(&mut self, reader: &mut Reader, start: &Element) -> Option<SlideParagraph> {
        let mut paragraph = SlideParagraph::default();
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
                Event::Start(element) if element.in_ns(ns::A) => match element.local.as_str() {
                    "pPr" => {
                        paragraph.level = attr_i64(&element, "lvl").unwrap_or(0).clamp(0, 8) as u8;
                        paragraph.align = match element.attr_local("algn") {
                            Some("ctr") => Align::Center,
                            Some("r") => Align::Right,
                            Some("just") => Align::Justify,
                            _ => Align::Left,
                        };
                    }
                    // Bullet formatting is declared by which of these appears:
                    // `buNone` opts a paragraph out of its placeholder's list.
                    "buNone" => paragraph.bullet = Bullet::None,
                    "buChar" => paragraph.bullet = Bullet::Unordered,
                    "buAutoNum" => paragraph.bullet = Bullet::Ordered,
                    "r" => self.read_run(reader, &element, &mut paragraph.content),
                    "br" => paragraph.content.push(Inline::Break),
                    // A field is generated text such as a slide number.
                    "fld" => self.read_run(reader, &element, &mut paragraph.content),
                    _ => {}
                },
                _ => {}
            }
        }

        if inline_text(&paragraph.content).trim().is_empty() {
            return None;
        }
        Some(paragraph)
    }

    /// Read one `<a:r>` run.
    fn read_run(&mut self, reader: &mut Reader, start: &Element, into: &mut Vec<Inline>) {
        let mut style = TextStyle::default();
        let mut href: Option<String> = None;

        while let Some(event) = reader.read_event() {
            match event {
                Event::End(name) if name == start.qname => break,
                Event::Start(element) if element.in_ns(ns::A) => match element.local.as_str() {
                    "rPr" => {
                        style.bold = element.attr_local("b").is_some_and(truthy);
                        style.italic = element.attr_local("i").is_some_and(truthy);
                        style.strikethrough = matches!(
                            element.attr_local("strike"),
                            Some("sngStrike") | Some("dblStrike")
                        );
                        style.underline =
                            element.attr_local("u").is_some_and(|value| value != "none");
                        // DrawingML sizes are in hundredths of a point.
                        style.size = attr_i64(&element, "sz").map(|v| v as f64 / 100.0);
                    }
                    "hlinkClick" => {
                        href = element
                            .attr_local("id")
                            .and_then(|id| self.rels.resolve(&self.base, id));
                    }
                    "latin" => {
                        style.font = element
                            .attr_local("typeface")
                            .filter(|name| !name.is_empty())
                            .map(str::to_string);
                    }
                    "t" => {
                        let text = crate::xml::text_of(reader, &element.qname);
                        if !text.is_empty() {
                            match &href {
                                Some(url) => into.push(Inline::Link {
                                    href: url.clone(),
                                    runs: vec![Run::styled(text, style.clone())],
                                }),
                                None => push_run(into, Run::styled(text, style.clone())),
                            }
                        }
                    }
                    _ => {}
                },
                _ => {}
            }
        }
    }

    /// Read a `<p:pic>` and register the image it references.
    fn read_picture(&mut self, reader: &mut Reader, start: &Element) -> Option<ImageRef> {
        let mut relationship: Option<String> = None;
        let mut alt: Option<String> = None;
        let mut width: Option<f64> = None;
        let mut height: Option<f64> = None;
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
                Event::Start(element) => match element.local.as_str() {
                    "blip" => {
                        relationship = element
                            .attr_local("embed")
                            .or(element.attr_local("link"))
                            .map(str::to_string);
                    }
                    "cNvPr" => {
                        alt = element
                            .attr_local("descr")
                            .or(element.attr_local("name"))
                            .filter(|value| !value.trim().is_empty())
                            .map(str::to_string);
                    }
                    "ext" => {
                        width = attr_i64(&element, "cx").map(emu_to_points);
                        height = attr_i64(&element, "cy").map(emu_to_points);
                    }
                    _ => {}
                },
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
}

#[derive(Default)]
struct Shape {
    is_title: bool,
    /// A slide-number, date or footer placeholder: repeated page furniture
    /// rather than content.
    is_furniture: bool,
    blocks: Vec<Block>,
}

#[derive(Default)]
struct SlideParagraph {
    content: Vec<Inline>,
    level: u8,
    align: Align,
    bullet: Bullet,
}

/// How a slide paragraph is bulleted.
///
/// Defaults to unordered: body placeholders in PowerPoint are bulleted lists
/// unless a paragraph opts out with `<a:buNone/>`.
#[derive(Default, PartialEq, Eq, Clone, Copy)]
enum Bullet {
    #[default]
    Unordered,
    Ordered,
    None,
}

fn push_paragraph(blocks: &mut Vec<Block>, paragraph: SlideParagraph) {
    let block = Block::Paragraph {
        content: paragraph.content,
        align: paragraph.align,
        indent: 0.0,
    };

    if paragraph.bullet == Bullet::None {
        blocks.push(block);
        return;
    }

    let item = ListItem {
        blocks: vec![block],
        checked: None,
    };
    append_list_item(
        blocks,
        item,
        paragraph.level,
        paragraph.bullet == Bullet::Ordered,
    );
}

/// Append a list item at `level`, nesting into the item above as needed.
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

fn push_run(into: &mut Vec<Inline>, run: Run) {
    if let Some(Inline::Run(last)) = into.last_mut()
        && last.style == run.style
    {
        last.text.push_str(&run.text);
        return;
    }
    into.push(Inline::Run(run));
}

/// DrawingML writes booleans as `1`/`0` or `true`/`false` attributes.
fn truthy(value: &str) -> bool {
    matches!(value.trim(), "1" | "true" | "on")
}
