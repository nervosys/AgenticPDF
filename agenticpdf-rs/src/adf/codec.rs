// SPDX-License-Identifier: AGPL-3.0-or-later
//! Binary encoding of the semantic model.
//!
//! Text dominates a document, and the same strings recur constantly — a font
//! name on every run, a language on every code block, a URL across every link
//! to the same target. So strings are not stored inline: they go once into a
//! heap and everything else refers to them by index. On real documents the heap
//! is a large majority of the bytes and deduplicating it is most of the saving,
//! which is also why the heap is its own chunk rather than being repeated in
//! each section.
//!
//! Each value is written as a tag byte then its fields. Tags are explicit
//! rather than positional so that a field added in a later minor version can be
//! appended to a variant without renumbering the ones beside it.

use std::collections::HashMap;

use crate::doc::{
    Align, Block, Cell, Footnote, ImageRef, Inline, List, ListItem, PageSize, Row, Section,
    SectionKind, SemanticDoc, Table, TextStyle,
};

use super::AdfError;
use super::wire::{Reader, Writer};

// ============================================================================
// String heap
// ============================================================================

/// Collects strings while writing, assigning each a stable index.
#[derive(Debug, Default)]
pub struct StringTable {
    strings: Vec<String>,
    seen: HashMap<String, u32>,
}

impl StringTable {
    pub fn new() -> StringTable {
        StringTable::default()
    }

    /// Intern a string, returning its index. Repeats cost nothing.
    pub fn intern(&mut self, value: &str) -> u32 {
        if let Some(&id) = self.seen.get(value) {
            return id;
        }
        let id = self.strings.len() as u32;
        self.strings.push(value.to_string());
        self.seen.insert(value.to_string(), id);
        id
    }

    pub fn len(&self) -> usize {
        self.strings.len()
    }

    pub fn is_empty(&self) -> bool {
        self.strings.is_empty()
    }

    /// Serialise as `count`, `count + 1` end offsets, then the bytes.
    ///
    /// Offsets rather than lengths, and one extra: a string's bounds are then
    /// `offsets[i]..offsets[i + 1]` with no scan and no special case for the
    /// last entry.
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Writer::new();
        out.u32(self.strings.len() as u32);

        let mut offset = 0u32;
        out.u32(offset);
        for value in &self.strings {
            offset += value.len() as u32;
            out.u32(offset);
        }
        for value in &self.strings {
            out.raw(value.as_bytes());
        }
        out.bytes
    }
}

/// A string heap read from a file, borrowing the mapped bytes.
#[derive(Debug, Clone, Copy, Default)]
pub struct StringHeap<'a> {
    offsets: &'a [u8],
    data: &'a [u8],
    count: usize,
}

impl<'a> StringHeap<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<StringHeap<'a>, AdfError> {
        let mut reader = Reader::new(bytes);
        let count = reader.u32()? as usize;

        // count + 1 offsets, four bytes each.
        let table_len = count
            .checked_add(1)
            .and_then(|n| n.checked_mul(4))
            .ok_or(AdfError::Malformed("string table too large"))?;
        let offsets = reader.take(table_len)?;
        let data = reader.take(reader.remaining())?;

        let heap = StringHeap {
            offsets,
            data,
            count,
        };
        // Validate once here so `get` can stay infallible-ish and cheap.
        if heap.offset_at(count) as usize > data.len() {
            return Err(AdfError::Malformed("string heap offsets exceed its data"));
        }
        Ok(heap)
    }

    fn offset_at(&self, index: usize) -> u32 {
        let at = index * 4;
        match self.offsets.get(at..at + 4) {
            Some(bytes) => u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            None => 0,
        }
    }

    pub fn len(&self) -> usize {
        self.count
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Borrow string `id` out of the mapped bytes.
    ///
    /// Returns `None` for an out-of-range id or non-UTF-8 bytes rather than
    /// substituting a placeholder: a silently empty string in a document body
    /// is far harder to notice than a decode error.
    pub fn get(&self, id: u32) -> Option<&'a str> {
        let index = id as usize;
        if index >= self.count {
            return None;
        }
        let start = self.offset_at(index) as usize;
        let end = self.offset_at(index + 1) as usize;
        if start > end {
            return None;
        }
        std::str::from_utf8(self.data.get(start..end)?).ok()
    }

    fn string(&self, id: u32) -> Result<&'a str, AdfError> {
        self.get(id)
            .ok_or(AdfError::Malformed("string id out of range"))
    }

    fn owned(&self, id: u32) -> Result<String, AdfError> {
        self.string(id).map(str::to_string)
    }
}

// ============================================================================
// Tags
// ============================================================================

mod tag {
    pub const HEADING: u8 = 1;
    pub const PARAGRAPH: u8 = 2;
    pub const LIST: u8 = 3;
    pub const TABLE: u8 = 4;
    pub const QUOTE: u8 = 5;
    pub const CODE: u8 = 6;
    pub const FIGURE: u8 = 7;
    pub const DIVIDER: u8 = 8;
    pub const PAGE_BREAK: u8 = 9;

    pub const RUN: u8 = 1;
    pub const LINK: u8 = 2;
    pub const IMAGE: u8 = 3;
    pub const BREAK: u8 = 4;
    pub const FOOTNOTE_REF: u8 = 5;
}

/// Bit positions for the boolean half of [`TextStyle`].
///
/// Packed into two bytes rather than written as eight separate flags: style is
/// attached to every run, so this is one of the few places where a handful of
/// bits is worth the trouble.
mod style_bits {
    pub const BOLD: u16 = 1 << 0;
    pub const ITALIC: u16 = 1 << 1;
    pub const STRIKETHROUGH: u16 = 1 << 2;
    pub const UNDERLINE: u16 = 1 << 3;
    pub const CODE: u16 = 1 << 4;
    pub const SUPERSCRIPT: u16 = 1 << 5;
    pub const SUBSCRIPT: u16 = 1 << 6;
    pub const HIDDEN: u16 = 1 << 7;
    pub const HAS_FONT: u16 = 1 << 8;
    pub const HAS_SIZE: u16 = 1 << 9;
    pub const HAS_COLOR: u16 = 1 << 10;
}

// ============================================================================
// Encoding
// ============================================================================

/// Encode one section's blocks, interning strings into `table`.
pub fn encode_section(section: &Section, table: &mut StringTable) -> Vec<u8> {
    let mut out = Writer::new();

    out.u8(match section.kind {
        SectionKind::Flow => 0,
        SectionKind::Slide => 1,
        SectionKind::Sheet => 2,
    });
    write_opt_str(&mut out, section.title.as_deref(), table);
    write_opt_page_size(&mut out, section.page_size);
    write_blocks(&mut out, &section.blocks, table);
    write_blocks(&mut out, &section.notes, table);

    out.bytes
}

/// Decode a section written by [`encode_section`].
pub fn decode_section(bytes: &[u8], heap: &StringHeap<'_>) -> Result<Section, AdfError> {
    let mut reader = Reader::new(bytes);

    let kind = match reader.u8()? {
        0 => SectionKind::Flow,
        1 => SectionKind::Slide,
        2 => SectionKind::Sheet,
        _ => return Err(AdfError::Malformed("unknown section kind")),
    };
    let title = read_opt_str(&mut reader, heap)?;
    let page_size = read_opt_page_size(&mut reader)?;
    let blocks = read_blocks(&mut reader, heap)?;
    let notes = read_blocks(&mut reader, heap)?;

    Ok(Section {
        kind,
        title,
        blocks,
        notes,
        page_size,
    })
}

/// Encode the footnote list.
pub fn encode_footnotes(footnotes: &[Footnote], table: &mut StringTable) -> Vec<u8> {
    let mut out = Writer::new();
    out.usize(footnotes.len());
    for footnote in footnotes {
        write_opt_str(&mut out, footnote.label.as_deref(), table);
        write_blocks(&mut out, &footnote.blocks, table);
    }
    out.bytes
}

/// Decode the footnote list.
pub fn decode_footnotes(bytes: &[u8], heap: &StringHeap<'_>) -> Result<Vec<Footnote>, AdfError> {
    let mut reader = Reader::new(bytes);
    let count = reader.count()?;
    let mut footnotes = Vec::with_capacity(count);
    for _ in 0..count {
        footnotes.push(Footnote {
            label: read_opt_str(&mut reader, heap)?,
            blocks: read_blocks(&mut reader, heap)?,
        });
    }
    Ok(footnotes)
}

/// Encode document-level metadata.
pub fn encode_meta(
    document: &SemanticDoc,
    source_format: &str,
    table: &mut StringTable,
) -> Vec<u8> {
    let mut out = Writer::new();
    for field in [
        document.title.as_deref(),
        document.author.as_deref(),
        document.subject.as_deref(),
        document.creator.as_deref(),
        document.created.as_deref(),
        document.modified.as_deref(),
    ] {
        write_opt_str(&mut out, field, table);
    }
    out.u32(table.intern(source_format));
    out.bytes
}

/// Metadata as read back from a file.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Meta {
    pub title: Option<String>,
    pub author: Option<String>,
    pub subject: Option<String>,
    pub creator: Option<String>,
    pub created: Option<String>,
    pub modified: Option<String>,
    /// The format this document was imported from, or `adf` if authored here.
    pub source_format: String,
}

/// Decode document-level metadata.
pub fn decode_meta(bytes: &[u8], heap: &StringHeap<'_>) -> Result<Meta, AdfError> {
    let mut reader = Reader::new(bytes);
    Ok(Meta {
        title: read_opt_str(&mut reader, heap)?,
        author: read_opt_str(&mut reader, heap)?,
        subject: read_opt_str(&mut reader, heap)?,
        creator: read_opt_str(&mut reader, heap)?,
        created: read_opt_str(&mut reader, heap)?,
        modified: read_opt_str(&mut reader, heap)?,
        source_format: heap.owned(reader.u32()?)?,
    })
}

// ---------------------------------------------------------------------------
// Standalone blocks
// ---------------------------------------------------------------------------

/// Encode a single block carrying its own strings.
///
/// Section chunks share one heap, which is what makes them compact. An edit-log
/// entry cannot: it has to stay meaningful when merged into a document whose
/// heap it has never seen, so it pays for a private heap in exchange for being
/// self-contained.
pub fn encode_block_standalone(block: &Block) -> Vec<u8> {
    let mut table = StringTable::new();
    let mut body = Writer::new();
    write_block(&mut body, block, &mut table);

    let mut out = Writer::new();
    out.bytes_with_len(&table.encode());
    out.raw(&body.bytes);
    out.bytes
}

/// Decode a block written by [`encode_block_standalone`].
pub fn decode_block_standalone(bytes: &[u8]) -> Result<Block, AdfError> {
    let mut reader = Reader::new(bytes);
    let heap_bytes = reader.bytes_with_len()?;
    let heap = StringHeap::parse(heap_bytes)?;
    read_block(&mut reader, &heap, 0)
}

// ---------------------------------------------------------------------------
// Blocks
// ---------------------------------------------------------------------------

fn write_blocks(out: &mut Writer, blocks: &[Block], table: &mut StringTable) {
    out.usize(blocks.len());
    for block in blocks {
        write_block(out, block, table);
    }
}

fn read_blocks(reader: &mut Reader<'_>, heap: &StringHeap<'_>) -> Result<Vec<Block>, AdfError> {
    read_blocks_at(reader, heap, 0)
}

/// Maximum block nesting honoured while decoding.
///
/// Blocks nest through quotes, list items and table cells, and decoding
/// recurses once per level. A file claiming thousands of levels would overflow
/// the stack — an abort, not an error — so the depth is bounded here exactly as
/// the Markdown reader bounds it on the way in.
const MAX_DEPTH: usize = 64;

fn read_blocks_at(
    reader: &mut Reader<'_>,
    heap: &StringHeap<'_>,
    depth: usize,
) -> Result<Vec<Block>, AdfError> {
    if depth > MAX_DEPTH {
        return Err(AdfError::Malformed("block nesting exceeds the depth limit"));
    }
    let count = reader.count()?;
    let mut blocks = Vec::with_capacity(count.min(1024));
    for _ in 0..count {
        blocks.push(read_block(reader, heap, depth)?);
    }
    Ok(blocks)
}

fn write_block(out: &mut Writer, block: &Block, table: &mut StringTable) {
    match block {
        Block::Heading { level, content } => {
            out.u8(tag::HEADING);
            out.u8(*level);
            write_inlines(out, content, table);
        }
        Block::Paragraph {
            content,
            align,
            indent,
        } => {
            out.u8(tag::PARAGRAPH);
            out.u8(match align {
                Align::Left => 0,
                Align::Center => 1,
                Align::Right => 2,
                Align::Justify => 3,
            });
            out.f64(*indent);
            write_inlines(out, content, table);
        }
        Block::List(list) => {
            out.u8(tag::LIST);
            out.u8(u8::from(list.ordered));
            out.varint(list.start);
            out.usize(list.items.len());
            for item in &list.items {
                out.option(item.checked, |w, checked| w.u8(u8::from(checked)));
                write_blocks(out, &item.blocks, table);
            }
        }
        Block::Table(table_block) => {
            out.u8(tag::TABLE);
            write_opt_str(out, table_block.caption.as_deref(), table);
            out.usize(table_block.header_rows);
            out.usize(table_block.column_widths.len());
            for width in &table_block.column_widths {
                out.f64(*width);
            }
            out.usize(table_block.rows.len());
            for row in &table_block.rows {
                out.usize(row.cells.len());
                for cell in &row.cells {
                    out.usize(cell.col_span);
                    out.usize(cell.row_span);
                    write_blocks(out, &cell.blocks, table);
                }
            }
        }
        Block::Quote(blocks) => {
            out.u8(tag::QUOTE);
            write_blocks(out, blocks, table);
        }
        Block::Code { language, text } => {
            out.u8(tag::CODE);
            write_opt_str(out, language.as_deref(), table);
            out.u32(table.intern(text));
        }
        Block::Figure { image, caption } => {
            out.u8(tag::FIGURE);
            write_image_ref(out, image, table);
            write_opt_str(out, caption.as_deref(), table);
        }
        Block::Divider => out.u8(tag::DIVIDER),
        Block::PageBreak => out.u8(tag::PAGE_BREAK),
    }
}

fn read_block(
    reader: &mut Reader<'_>,
    heap: &StringHeap<'_>,
    depth: usize,
) -> Result<Block, AdfError> {
    let next = depth + 1;
    match reader.u8()? {
        tag::HEADING => Ok(Block::Heading {
            level: reader.u8()?,
            content: read_inlines(reader, heap)?,
        }),
        tag::PARAGRAPH => {
            let align = match reader.u8()? {
                0 => Align::Left,
                1 => Align::Center,
                2 => Align::Right,
                3 => Align::Justify,
                _ => return Err(AdfError::Malformed("unknown alignment")),
            };
            let indent = reader.f64()?;
            Ok(Block::Paragraph {
                content: read_inlines(reader, heap)?,
                align,
                indent,
            })
        }
        tag::LIST => {
            let ordered = reader.u8()? != 0;
            let start = reader.varint()?;
            let count = reader.count()?;
            let mut items = Vec::with_capacity(count.min(1024));
            for _ in 0..count {
                let checked = reader.option(|r| Ok(r.u8()? != 0))?;
                items.push(ListItem {
                    checked,
                    blocks: read_blocks_at(reader, heap, next)?,
                });
            }
            Ok(Block::List(List {
                ordered,
                start,
                items,
            }))
        }
        tag::TABLE => {
            let caption = read_opt_str(reader, heap)?;
            let header_rows = reader.varint()? as usize;
            let width_count = reader.count()?;
            let mut column_widths = Vec::with_capacity(width_count.min(1024));
            for _ in 0..width_count {
                column_widths.push(reader.f64()?);
            }
            let row_count = reader.count()?;
            let mut rows = Vec::with_capacity(row_count.min(1024));
            for _ in 0..row_count {
                let cell_count = reader.count()?;
                let mut cells = Vec::with_capacity(cell_count.min(1024));
                for _ in 0..cell_count {
                    cells.push(Cell {
                        col_span: reader.varint()? as usize,
                        row_span: reader.varint()? as usize,
                        blocks: read_blocks_at(reader, heap, next)?,
                    });
                }
                rows.push(Row { cells });
            }
            Ok(Block::Table(Table {
                caption,
                header_rows,
                rows,
                column_widths,
            }))
        }
        tag::QUOTE => Ok(Block::Quote(read_blocks_at(reader, heap, next)?)),
        tag::CODE => Ok(Block::Code {
            language: read_opt_str(reader, heap)?,
            text: heap.owned(reader.u32()?)?,
        }),
        tag::FIGURE => Ok(Block::Figure {
            image: read_image_ref(reader, heap)?,
            caption: read_opt_str(reader, heap)?,
        }),
        tag::DIVIDER => Ok(Block::Divider),
        tag::PAGE_BREAK => Ok(Block::PageBreak),
        _ => Err(AdfError::Malformed("unknown block tag")),
    }
}

// ---------------------------------------------------------------------------
// Inlines
// ---------------------------------------------------------------------------

fn write_inlines(out: &mut Writer, inlines: &[Inline], table: &mut StringTable) {
    out.usize(inlines.len());
    for inline in inlines {
        match inline {
            Inline::Run(run) => {
                out.u8(tag::RUN);
                write_run(out, run, table);
            }
            Inline::Link { href, runs } => {
                out.u8(tag::LINK);
                out.u32(table.intern(href));
                out.usize(runs.len());
                for run in runs {
                    write_run(out, run, table);
                }
            }
            Inline::Image(image) => {
                out.u8(tag::IMAGE);
                write_image_ref(out, image, table);
            }
            Inline::Break => out.u8(tag::BREAK),
            Inline::FootnoteRef { index } => {
                out.u8(tag::FOOTNOTE_REF);
                out.usize(*index);
            }
        }
    }
}

fn read_inlines(reader: &mut Reader<'_>, heap: &StringHeap<'_>) -> Result<Vec<Inline>, AdfError> {
    let count = reader.count()?;
    let mut inlines = Vec::with_capacity(count.min(4096));
    for _ in 0..count {
        inlines.push(match reader.u8()? {
            tag::RUN => Inline::Run(read_run(reader, heap)?),
            tag::LINK => {
                let href = heap.owned(reader.u32()?)?;
                let run_count = reader.count()?;
                let mut runs = Vec::with_capacity(run_count.min(4096));
                for _ in 0..run_count {
                    runs.push(read_run(reader, heap)?);
                }
                Inline::Link { href, runs }
            }
            tag::IMAGE => Inline::Image(read_image_ref(reader, heap)?),
            tag::BREAK => Inline::Break,
            tag::FOOTNOTE_REF => Inline::FootnoteRef {
                index: reader.varint()? as usize,
            },
            _ => return Err(AdfError::Malformed("unknown inline tag")),
        });
    }
    Ok(inlines)
}

fn write_run(out: &mut Writer, run: &crate::doc::Run, table: &mut StringTable) {
    out.u32(table.intern(&run.text));
    write_style(out, &run.style, table);
}

fn read_run(reader: &mut Reader<'_>, heap: &StringHeap<'_>) -> Result<crate::doc::Run, AdfError> {
    Ok(crate::doc::Run {
        text: heap.owned(reader.u32()?)?,
        style: read_style(reader, heap)?,
    })
}

fn write_style(out: &mut Writer, style: &TextStyle, table: &mut StringTable) {
    let mut bits = 0u16;
    for (set, bit) in [
        (style.bold, style_bits::BOLD),
        (style.italic, style_bits::ITALIC),
        (style.strikethrough, style_bits::STRIKETHROUGH),
        (style.underline, style_bits::UNDERLINE),
        (style.code, style_bits::CODE),
        (style.superscript, style_bits::SUPERSCRIPT),
        (style.subscript, style_bits::SUBSCRIPT),
        (style.hidden, style_bits::HIDDEN),
        (style.font.is_some(), style_bits::HAS_FONT),
        (style.size.is_some(), style_bits::HAS_SIZE),
        (style.color.is_some(), style_bits::HAS_COLOR),
    ] {
        if set {
            bits |= bit;
        }
    }
    out.u16(bits);

    if let Some(font) = &style.font {
        out.u32(table.intern(font));
    }
    if let Some(size) = style.size {
        out.f64(size);
    }
    if let Some(color) = style.color {
        for channel in color {
            out.f64(channel);
        }
    }
}

fn read_style(reader: &mut Reader<'_>, heap: &StringHeap<'_>) -> Result<TextStyle, AdfError> {
    let bits = reader.u16()?;
    let has = |bit: u16| bits & bit != 0;

    Ok(TextStyle {
        bold: has(style_bits::BOLD),
        italic: has(style_bits::ITALIC),
        strikethrough: has(style_bits::STRIKETHROUGH),
        underline: has(style_bits::UNDERLINE),
        code: has(style_bits::CODE),
        superscript: has(style_bits::SUPERSCRIPT),
        subscript: has(style_bits::SUBSCRIPT),
        hidden: has(style_bits::HIDDEN),
        font: if has(style_bits::HAS_FONT) {
            Some(heap.owned(reader.u32()?)?)
        } else {
            None
        },
        size: if has(style_bits::HAS_SIZE) {
            Some(reader.f64()?)
        } else {
            None
        },
        color: if has(style_bits::HAS_COLOR) {
            Some([reader.f64()?, reader.f64()?, reader.f64()?])
        } else {
            None
        },
    })
}

// ---------------------------------------------------------------------------
// Leaves
// ---------------------------------------------------------------------------

fn write_image_ref(out: &mut Writer, image: &ImageRef, table: &mut StringTable) {
    out.u32(table.intern(&image.asset_id));
    write_opt_str(out, image.alt.as_deref(), table);
    out.option(image.width, |w, value| w.f64(value));
    out.option(image.height, |w, value| w.f64(value));
}

fn read_image_ref(reader: &mut Reader<'_>, heap: &StringHeap<'_>) -> Result<ImageRef, AdfError> {
    Ok(ImageRef {
        asset_id: heap.owned(reader.u32()?)?,
        alt: read_opt_str(reader, heap)?,
        width: reader.option(|r| r.f64())?,
        height: reader.option(|r| r.f64())?,
    })
}

fn write_opt_str(out: &mut Writer, value: Option<&str>, table: &mut StringTable) {
    match value {
        Some(text) => {
            out.u8(1);
            out.u32(table.intern(text));
        }
        None => out.u8(0),
    }
}

fn read_opt_str(
    reader: &mut Reader<'_>,
    heap: &StringHeap<'_>,
) -> Result<Option<String>, AdfError> {
    match reader.u8()? {
        0 => Ok(None),
        1 => Ok(Some(heap.owned(reader.u32()?)?)),
        _ => Err(AdfError::Malformed("option tag is not 0 or 1")),
    }
}

fn write_opt_page_size(out: &mut Writer, size: Option<PageSize>) {
    out.option(size, |w, value| {
        for field in [
            value.width,
            value.height,
            value.margin_left,
            value.margin_right,
            value.margin_top,
            value.margin_bottom,
        ] {
            w.f64(field);
        }
    });
}

fn read_opt_page_size(reader: &mut Reader<'_>) -> Result<Option<PageSize>, AdfError> {
    reader.option(|r| {
        Ok(PageSize {
            width: r.f64()?,
            height: r.f64()?,
            margin_left: r.f64()?,
            margin_right: r.f64()?,
            margin_top: r.f64()?,
            margin_bottom: r.f64()?,
        })
    })
}
