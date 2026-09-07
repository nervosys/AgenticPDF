// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Derived from anydoc (https://github.com/firecrawl/anydoc), MIT licensed,
// Copyright (c) 2026 Sideguide Technologies Inc. See LICENSE-MIT-anydoc.txt.
//
//! PowerPoint 97-2003 binary (`.ppt`), per [MS-PPT].
//!
//! The `PowerPoint Document` stream is a tree of records — an 8-byte header
//! then a body, where a body is either more records or an atom's payload — but
//! its *order* is not the presentation's order. PowerPoint appends edits rather
//! than rewriting, so the stream accumulates superseded copies of slides, and
//! reading it front to back yields a mixture of current and stale content in
//! whatever order the edits happened.
//!
//! The current presentation is found by walking backwards instead:
//!
//! 1. The `Current User` stream points at the most recent `UserEditAtom`.
//! 2. Each `UserEditAtom` points at the previous one and at a persist
//!    directory fragment; together the chain maps persist ids to offsets.
//! 3. The current `DocumentContainer` holds slide and notes lists giving the
//!    persist id of each slide in presentation order.
//!
//! This is the same problem the `.pptx` reader solves by reading `sldIdLst`
//! rather than the part names. When the chain is unusable — truncated files,
//! recovered fragments — the reader falls back to a forward walk, which gets
//! the content back in stream order.

use std::collections::HashMap;

use crate::PdfError;
use crate::container::ole::{Ole2, decode_utf16le, u32_at};
use crate::doc::{
    Align, Block, Cell, Inline, ListItem, Row, Run, Section, SectionKind, SemanticDoc, Table,
    TextStyle,
};

// Record types.
const RT_DOCUMENT: u16 = 0x03E8;
const RT_SLIDE: u16 = 0x03EE;
const RT_NOTES: u16 = 0x03F0;
const RT_USER_EDIT_ATOM: u16 = 0x0FF5;
const RT_PERSIST_DIRECTORY_ATOM: u16 = 0x1772;
const RT_SLIDE_LIST_WITH_TEXT: u16 = 0x0FF0;
const RT_SLIDE_PERSIST_ATOM: u16 = 0x03F3;
/// Opens a notes record, and names the slide the notes belong to.
const RT_NOTES_ATOM: u16 = 0x03F1;
const RT_TEXT_HEADER_ATOM: u16 = 0x0F9F;
const RT_TEXT_CHARS_ATOM: u16 = 0x0FA0;
const RT_TEXT_BYTES_ATOM: u16 = 0x0FA8;
const RT_STYLE_TEXT_PROP_ATOM: u16 = 0x0FA1;

// The drawing layer. A table has no record of its own in this format: it is a
// group of ordinary shapes, marked as a table by a property on the group.
const ART_SPGR_CONTAINER: u16 = 0xF003;
const ART_SP_CONTAINER: u16 = 0xF004;
const ART_SECONDARY_FOPT: u16 = 0xF121;
const ART_TERTIARY_FOPT: u16 = 0xF122;
const ART_FOPT: u16 = 0xF00B;
const ART_CHILD_ANCHOR: u16 = 0xF00F;
/// `tableProperties`: non-zero on the group shape of a table.
const PID_TABLE_PROPERTIES: u16 = 0x039F;
const RT_CRYPT_SESSION_10: u16 = 0x2F14;

// Text types, which say what role a text shape plays on its page.
const TEXT_TYPE_TITLE: u8 = 0;
const TEXT_TYPE_BODY: u8 = 1;
const TEXT_TYPE_NOTES: u8 = 2;
const TEXT_TYPE_CENTER_BODY: u8 = 5;
const TEXT_TYPE_CENTER_TITLE: u8 = 6;
/// Not a placeholder at all. Used for the shapes this reader synthesises,
/// which are not the title and are not bulleted.
const TEXT_TYPE_OTHER: u8 = 0xFF;

/// Cap on records visited, bounding a cyclic or corrupt tree.
const MAX_RECORDS: usize = 500_000;
/// Cap on recursion into nested containers.
const MAX_DEPTH: usize = 64;

/// Parse a PowerPoint 97-2003 presentation.
pub fn parse(data: &[u8]) -> Result<SemanticDoc, PdfError> {
    let mut ole = Ole2::open(data)?;
    let stream = ole.read("PowerPoint Document")?;
    let current_user = ole.read_optional("Current User").unwrap_or_default();

    // An encrypted presentation has a session record instead of readable text.
    if find_record(&stream, RT_CRYPT_SESSION_10).is_some() {
        return Err(PdfError::Encrypted);
    }

    let mut document = SemanticDoc::default();
    let sections = match layout(&stream, &current_user) {
        Some(layout) => read_in_presentation_order(&stream, &layout),
        // No usable persist chain: read the stream forward instead.
        None => read_in_stream_order(&stream),
    };
    document.sections = sections;

    if document.sections.is_empty() {
        return Err(PdfError::MissingPart("ppt contains no slides".into()));
    }
    Ok(document)
}

// ============================================================================
// Record tree
// ============================================================================

/// One record: its version/instance field, type, and body.
struct RecordRef<'a> {
    ver_inst: u16,
    kind: u16,
    body: &'a [u8],
}

impl RecordRef<'_> {
    /// Whether this record contains other records rather than an atom payload.
    fn is_container(&self) -> bool {
        self.ver_inst & 0x0F == 0x0F
    }

    /// The instance field, which distinguishes same-typed siblings.
    fn instance(&self) -> u16 {
        self.ver_inst >> 4
    }
}

/// Read the record at `pos`.
fn record_at(data: &[u8], pos: usize) -> Option<(RecordRef<'_>, usize)> {
    let ver_inst = crate::container::ole::u16_at(data, pos)?;
    let kind = crate::container::ole::u16_at(data, pos + 2)?;
    let len = u32_at(data, pos + 4)? as usize;
    let body = data.get(pos + 8..)?.get(..len)?;
    Some((
        RecordRef {
            ver_inst,
            kind,
            body,
        },
        pos + 8 + len,
    ))
}

/// Iterate a container's immediate children.
fn children(data: &[u8]) -> impl Iterator<Item = RecordRef<'_>> {
    let mut pos = 0usize;
    std::iter::from_fn(move || {
        let (record, next) = record_at(data, pos)?;
        pos = next;
        Some(record)
    })
}

/// Find the first record of a type anywhere in the tree.
fn find_record(data: &[u8], kind: u16) -> Option<&[u8]> {
    fn walk<'a>(data: &'a [u8], kind: u16, depth: usize, budget: &mut usize) -> Option<&'a [u8]> {
        if depth > MAX_DEPTH {
            return None;
        }
        for record in children(data) {
            if *budget == 0 {
                return None;
            }
            *budget -= 1;
            if record.kind == kind {
                return Some(record.body);
            }
            if record.is_container()
                && let Some(hit) = walk(record.body, kind, depth + 1, budget)
            {
                return Some(hit);
            }
        }
        None
    }
    let mut budget = MAX_RECORDS;
    walk(data, kind, 0, &mut budget)
}

// ============================================================================
// Persist resolution
// ============================================================================

/// Where the current presentation's pieces live.
struct Layout<'a> {
    /// Persist id → byte offset in the stream.
    persist: HashMap<u32, usize>,
    /// Slide list, in presentation order.
    slide_list: &'a [u8],
    /// Notes list, pairing notes with slides.
    notes_list: Option<&'a [u8]>,
}

/// Resolve the edit chain into a persist directory and the current document.
fn layout<'a>(stream: &'a [u8], current_user: &[u8]) -> Option<Layout<'a>> {
    // The Current User stream points at the most recent edit.
    let mut offset = u32_at(current_user, 0x10).map(|value| value as usize)?;

    let mut persist: HashMap<u32, usize> = HashMap::new();
    let mut document_offset = None;
    let mut visited = 0usize;

    // Walk the edit chain backwards. Earlier fragments must not overwrite
    // later ones, so an id already present is left alone.
    while offset != 0 && offset < stream.len() && visited < 1024 {
        visited += 1;
        let (record, _) = record_at(stream, offset)?;
        if record.kind != RT_USER_EDIT_ATOM {
            break;
        }
        let body = record.body;
        if document_offset.is_none() {
            document_offset = u32_at(body, 0).map(|value| value as usize);
        }
        let directory = u32_at(body, 12).map(|value| value as usize);
        let previous = u32_at(body, 8).map(|value| value as usize).unwrap_or(0);

        if let Some(directory) = directory
            && directory < stream.len()
            && let Some((fragment, _)) = record_at(stream, directory)
            && fragment.kind == RT_PERSIST_DIRECTORY_ATOM
        {
            read_persist_fragment(fragment.body, &mut persist);
        }
        offset = previous;
    }

    // The document container is itself addressed by persist id.
    let document_at = persist
        .get(&(document_offset? as u32))
        .copied()
        .or(document_offset)?;
    let (document, _) = record_at(stream, document_at)?;
    if document.kind != RT_DOCUMENT {
        return None;
    }

    // Instance 0 is the slide list; instance 2 is the notes list.
    let mut slide_list = None;
    let mut notes_list = None;
    for child in children(document.body) {
        if child.kind == RT_SLIDE_LIST_WITH_TEXT {
            match child.instance() {
                0 => slide_list = Some(child.body),
                2 => notes_list = Some(child.body),
                _ => {}
            }
        }
    }

    Some(Layout {
        persist,
        slide_list: slide_list?,
        notes_list,
    })
}

/// Read one persist directory fragment: runs of consecutive ids and offsets.
fn read_persist_fragment(body: &[u8], persist: &mut HashMap<u32, usize>) {
    let mut pos = 0usize;
    while pos + 4 <= body.len() {
        let Some(header) = u32_at(body, pos) else {
            return;
        };
        pos += 4;
        // The header packs a starting id and how many offsets follow it.
        let first_id = header & 0x000F_FFFF;
        let count = (header >> 20) as usize;
        for index in 0..count {
            let Some(offset) = u32_at(body, pos) else {
                return;
            };
            pos += 4;
            // A later fragment already holds the current value for this id.
            persist
                .entry(first_id + index as u32)
                .or_insert(offset as usize);
        }
    }
}

/// The persist ids listed in a slide or notes list, in order.
fn persist_ids(list: &[u8]) -> Vec<u32> {
    persist_entries(list)
        .into_iter()
        .map(|(id, _)| id)
        .collect()
}

/// `(persist id, slide id)` for each entry in a slide list.
///
/// A `SlidePersistAtom` opens with the persist id that locates the record and
/// carries the slide's own id twelve bytes in. A body too short to hold the
/// slide id still yields its persist id: losing the slide entirely would be a
/// far worse answer than not knowing its id, and zero reads as "not known".
fn persist_entries(list: &[u8]) -> Vec<(u32, u32)> {
    children(list)
        .filter(|record| record.kind == RT_SLIDE_PERSIST_ATOM)
        .filter_map(|record| {
            Some((
                u32_at(record.body, 0)?,
                u32_at(record.body, 12).unwrap_or(0),
            ))
        })
        .collect()
}

/// The slide a notes record names, from the `NotesAtom` that opens it.
///
/// This is the authoritative link, and the only one. The notes list's own
/// `SlidePersistAtom` also holds a slide id, and it is *not* the owning slide:
/// on a two-slide deck whose note belongs to the second slide it read 256 while
/// the slides were 256 and 257, and the `NotesAtom` read 257. Pairing on the
/// former puts the note on the wrong slide, and pairing by position puts it
/// there too as soon as one slide has no notes.
fn notes_owner(stream: &[u8], layout: &Layout, persist: u32) -> Option<u32> {
    let offset = *layout.persist.get(&persist)?;
    let (record, _) = record_at(stream, offset)?;
    if record.kind != RT_NOTES {
        return None;
    }
    children(record.body)
        .find(|child| child.kind == RT_NOTES_ATOM)
        .and_then(|atom| u32_at(atom.body, 0))
}

// ============================================================================
// Reading slides
// ============================================================================

/// Read slides through the resolved persist directory.
fn read_in_presentation_order(stream: &[u8], layout: &Layout) -> Vec<Section> {
    // Pair each notes record to the slide its `NotesAtom` names.
    //
    // Position is right only while every slide has notes. A two-slide deck with
    // a note on the second slide has one notes entry, and by position it landed
    // on the first -- where the same deck saved as .pptx and .odp put it on the
    // second, which is where PowerPoint shows it.
    let notes_ids = layout.notes_list.map(persist_ids).unwrap_or_default();
    let notes_by_slide: HashMap<u32, u32> = notes_ids
        .iter()
        .filter_map(|persist| Some((notes_owner(stream, layout, *persist)?, *persist)))
        .collect();

    let mut sections = Vec::new();
    for (index, (id, slide_id)) in persist_entries(layout.slide_list).into_iter().enumerate() {
        let Some(&offset) = layout.persist.get(&id) else {
            continue;
        };
        let Some((record, _)) = record_at(stream, offset) else {
            continue;
        };
        if record.kind != RT_SLIDE {
            continue;
        }

        let shapes = collect_shapes(record.body);
        // Fall back to position only where no notes record named a slide at
        // all, so a producer that omits the atom keeps the old behaviour rather
        // than losing its notes.
        // Position is the fallback for a file that names no owners at all, and
        // for a slide whose own id is unknown; neither can be paired by id.
        let notes_id = match notes_by_slide.is_empty() || slide_id == 0 {
            false => notes_by_slide.get(&slide_id).copied(),
            true => notes_ids.get(index).copied(),
        };
        let notes = notes_id
            .as_ref()
            .and_then(|id| layout.persist.get(id))
            .and_then(|&offset| record_at(stream, offset))
            .filter(|(record, _)| record.kind == RT_NOTES)
            .map(|(record, _)| collect_shapes(record.body))
            .unwrap_or_default();

        sections.push(build_slide(shapes, notes));
    }
    sections
}

/// Read slides by walking the stream forward.
///
/// The fallback when the persist chain is unusable. Superseded copies of a
/// slide may still be present, so this can show stale content — but it recovers
/// text from files the resolved path cannot open at all.
fn read_in_stream_order(stream: &[u8]) -> Vec<Section> {
    let mut sections: Vec<Section> = Vec::new();
    let mut budget = MAX_RECORDS;

    fn walk(data: &[u8], depth: usize, budget: &mut usize, sections: &mut Vec<Section>) {
        if depth > MAX_DEPTH {
            return;
        }
        for record in children(data) {
            if *budget == 0 {
                return;
            }
            *budget -= 1;
            match record.kind {
                RT_SLIDE => sections.push(build_slide(collect_shapes(record.body), Vec::new())),
                // Notes attach to the slide they follow.
                RT_NOTES => {
                    let notes = collect_shapes(record.body);
                    if let Some(section) = sections.last_mut() {
                        section.notes = notes_blocks(notes);
                    }
                }
                _ if record.is_container() => walk(record.body, depth + 1, budget, sections),
                _ => {}
            }
        }
    }

    walk(stream, 0, &mut budget, &mut sections);
    sections
}

/// A text shape gathered from a slide.
struct Shape {
    /// Paragraph and character formatting, where the shape states it.
    props: Option<TextProps>,
    /// The grid, where this shape is a group PowerPoint marked as a table.
    table: Option<Table>,
    /// Text type: 0 and 6 are the title placeholders.
    kind: u8,
    text: String,
}

/// Collect the text shapes in a slide container, in document order.
fn collect_shapes(data: &[u8]) -> Vec<Shape> {
    let mut shapes: Vec<Shape> = Vec::new();
    let mut budget = MAX_RECORDS;

    fn walk(data: &[u8], depth: usize, budget: &mut usize, shapes: &mut Vec<Shape>) {
        if depth > MAX_DEPTH {
            return;
        }
        for record in children(data) {
            if *budget == 0 {
                return;
            }
            *budget -= 1;
            match record.kind {
                // A new text shape begins; its type says what role it plays.
                RT_TEXT_HEADER_ATOM => shapes.push(Shape {
                    kind: record.body.first().copied().unwrap_or(1),
                    text: String::new(),
                    props: None,
                    table: None,
                }),
                // UTF-16 text.
                RT_TEXT_CHARS_ATOM => push_text(shapes, decode_utf16le(record.body)),
                // The low bytes of UTF-16 code units, for text that fits in
                // the Latin-1 range.
                RT_TEXT_BYTES_ATOM => {
                    push_text(shapes, record.body.iter().map(|&b| b as char).collect())
                }
                // The formatting for the text just read. It follows the text
                // atoms of the same shape, so the length it must account for
                // is already known -- which is what makes the parse
                // self-checking.
                RT_STYLE_TEXT_PROP_ATOM => {
                    if let Some(shape) = shapes.last_mut() {
                        let units = shape.text.encode_utf16().count();
                        shape.props = read_style_props(record.body, units);
                    }
                }
                // A table is a group of cell shapes. Read as loose shapes
                // its cells arrive as a column of stray paragraphs, so the
                // group is taken whole and its children are not walked again.
                _ if record.is_container() => {
                    if record.kind == ART_SPGR_CONTAINER
                        && let Some(table) = read_table_group(record.body)
                    {
                        shapes.push(Shape {
                            // Not a placeholder of any kind; it is the grid.
                            kind: TEXT_TYPE_OTHER,
                            text: String::new(),
                            props: None,
                            table: Some(table),
                        });
                        continue;
                    }
                    walk(record.body, depth + 1, budget, shapes);
                }
                _ => {}
            }
        }
    }

    walk(data, 0, &mut budget, &mut shapes);
    shapes.retain(|shape| !shape.text.trim().is_empty() || shape.table.is_some());
    shapes
}

/// Paragraph and character formatting for one text shape.
///
/// PowerPoint states both as runs of characters rather than as markup: the
/// paragraph runs carry the outline level that makes a bullet a sub-bullet, and
/// the character runs carry bold and italic. Without them a slide's body is a
/// flat list of unstyled lines, which is what the .pptx of the same deck
/// disagreed with.
#[derive(Debug, Default, Clone)]
struct TextProps {
    /// Characters covered, and the outline level, per paragraph run.
    levels: Vec<(u32, u8)>,
    /// Characters covered, and the formatting, per character run.
    runs: Vec<(u32, TextStyle)>,
}

/// Read a `StyleTextPropAtom`, or nothing if it does not describe this text.
///
/// Both run arrays must account for exactly the text plus its terminator. That
/// total is the check on every optional field's width below: get one wrong and
/// the walk lands somewhere arbitrary, the totals miss, and the whole atom is
/// discarded rather than used to mangle text that is currently correct. Real
/// files exercise this -- a master placeholder carries an atom covering one
/// character of an eighty-character string, and is rejected here.
fn read_style_props(body: &[u8], text_units: usize) -> Option<TextProps> {
    // The terminating carriage return is counted by the runs but is not in the
    // text, so every total is one more than the text's own length.
    let want = text_units as u64 + 1;
    let mut at = 0usize;
    let mut props = TextProps::default();

    let mut total = 0u64;
    while total < want {
        let count = u32_at(body, at)?;
        let level = u16_at(body, at + 4)?;
        at = at.checked_add(6)?;
        at = at.checked_add(paragraph_exception_len(body, at)?)?;
        if count == 0 {
            return None;
        }
        props.levels.push((count, level.min(8) as u8));
        total += u64::from(count);
    }
    if total != want {
        return None;
    }

    let mut total = 0u64;
    while total < want {
        let count = u32_at(body, at)?;
        at = at.checked_add(4)?;
        let (size, style) = character_exception(body, at)?;
        at = at.checked_add(size)?;
        if count == 0 {
            return None;
        }
        props.runs.push((count, style));
        total += u64::from(count);
    }
    match total == want {
        true => Some(props),
        false => None,
    }
}

/// The size of one `TextPFException`, whose fields are present per its mask.
///
/// Nothing here is read; the paragraph's only interesting property, its outline
/// level, sits *before* this structure. All that is needed is to step over it
/// to reach the character runs.
fn paragraph_exception_len(body: &[u8], at: usize) -> Option<usize> {
    // Mask bit, and the width of the field it guards. The order matters: the
    // tab-stop array between the two halves is counted rather than fixed, so
    // its length has to be read from the file at exactly its own offset.
    const BEFORE_TABS: [(u32, usize); 12] = [
        (0x0000_0007, 2), // bullet flags
        (0x0000_0080, 2), // bullet character
        (0x0000_0010, 2), // bullet font
        (0x0000_0040, 2), // bullet size
        (0x0000_0020, 4), // bullet colour
        (0x0000_0800, 2), // alignment
        (0x0000_1000, 2), // line spacing
        (0x0000_2000, 2), // space before
        (0x0000_4000, 2), // space after
        (0x0000_0100, 2), // left margin
        (0x0000_0400, 2), // indent
        (0x0000_0200, 2), // default tab size
    ];
    const AFTER_TABS: [(u32, usize); 3] = [
        (0x0001_0000, 2), // font alignment
        (0x0006_0000, 2), // wrap flags
        (0x0020_0000, 2), // text direction
    ];

    let masks = u32_at(body, at)?;
    let mut size = 4usize;
    for (bit, width) in BEFORE_TABS {
        if masks & bit != 0 {
            size = size.checked_add(width)?;
        }
    }
    if masks & 0x0010_0000 != 0 {
        let count = u16_at(body, at.checked_add(size)?)? as usize;
        size = size.checked_add(2)?.checked_add(count.checked_mul(4)?)?;
    }
    for (bit, width) in AFTER_TABS {
        if masks & bit != 0 {
            size = size.checked_add(width)?;
        }
    }
    match at.checked_add(size)? <= body.len() {
        true => Some(size),
        false => None,
    }
}

/// One `TextCFException`: its size, and the formatting it states.
fn character_exception(body: &[u8], at: usize) -> Option<(usize, TextStyle)> {
    /// Mask bit, and the width of the field it guards, after `style`.
    const FIELDS: [(u32, usize); 7] = [
        (0x0001_0000, 2), // typeface
        (0x0020_0000, 2), // old East Asian typeface
        (0x0040_0000, 2), // ANSI typeface
        (0x0080_0000, 2), // symbol typeface
        (0x0002_0000, 2), // size
        (0x0004_0000, 4), // colour
        (0x0008_0000, 2), // position
    ];

    let masks = u32_at(body, at)?;
    let mut size = 4usize;
    let mut style = TextStyle::default();

    // The low half of the mask names the character properties; any of them
    // being claimed means the `style` field that holds their values is present.
    if masks & 0x0000_FFFF != 0 {
        let bits = u16_at(body, at.checked_add(size)?)?;
        // A bit in `style` counts only where the mask claims that property:
        // the field is shared, and a value for a property nobody claimed is
        // left over from whatever wrote it.
        style.bold = masks & 0x1 != 0 && bits & 0x1 != 0;
        style.italic = masks & 0x2 != 0 && bits & 0x2 != 0;
        style.underline = masks & 0x4 != 0 && bits & 0x4 != 0;
        size = size.checked_add(2)?;
    }
    for (bit, width) in FIELDS {
        if masks & bit != 0 {
            size = size.checked_add(width)?;
        }
    }
    match at.checked_add(size)? <= body.len() {
        true => Some((size, style)),
        false => None,
    }
}

fn u16_at(data: &[u8], at: usize) -> Option<u16> {
    let bytes = data.get(at..at.checked_add(2)?)?;
    Some(u16::from_le_bytes([bytes[0], bytes[1]]))
}

/// Walks a run array, reporting what covers the character at the cursor.
struct Runs<'a, T> {
    runs: &'a [(u32, T)],
    index: usize,
    used: u32,
}

impl<'a, T> Runs<'a, T> {
    fn new(runs: &'a [(u32, T)]) -> Self {
        Runs {
            runs,
            index: 0,
            used: 0,
        }
    }

    fn current(&self) -> Option<&'a T> {
        self.runs.get(self.index).map(|(_, value)| value)
    }

    /// Step over `units` UTF-16 code units, which is what the counts measure.
    fn advance(&mut self, units: usize) {
        for _ in 0..units {
            let Some((count, _)) = self.runs.get(self.index) else {
                return;
            };
            self.used += 1;
            if self.used >= *count {
                self.index += 1;
                self.used = 0;
            }
        }
    }
}

/// Append text to the shape being gathered, starting one if none is open.
fn push_text(shapes: &mut Vec<Shape>, text: String) {
    match shapes.last_mut() {
        Some(shape) => shape.text.push_str(&text),
        None => shapes.push(Shape {
            kind: 1,
            text,
            props: None,
            table: None,
        }),
    }
}

/// Read a shape group as a table, if that is what PowerPoint drew.
///
/// There is no table record in this format. A table is an `SpgrContainer` whose
/// first child is the group's own shape, carrying a `tableProperties` property;
/// the rest are one shape per cell, each with a child anchor giving its
/// rectangle, plus a set of zero-area shapes that draw the rules. The grid has
/// to be rebuilt from those rectangles, because nothing else states it.
fn read_table_group(body: &[u8]) -> Option<Table> {
    let mut records = children(body);
    let group = records.next()?;
    if group.kind != ART_SP_CONTAINER || !is_table_group(group.body) {
        return None;
    }

    let mut cells: Vec<(Rect, Cell)> = Vec::new();
    for record in records {
        if record.kind != ART_SP_CONTAINER {
            continue;
        }
        let Some(rect) = child_anchor(record.body) else {
            continue;
        };
        // The rules between cells are shapes of no width or no height.
        if rect.right <= rect.left || rect.bottom <= rect.top {
            continue;
        }
        let mut blocks = Vec::new();
        for shape in collect_shapes(record.body) {
            push_shape_blocks(&mut blocks, &shape.text, shape.props.as_ref(), false);
        }
        cells.push((
            rect,
            Cell {
                blocks,
                ..Cell::default()
            },
        ));
    }

    match cells.is_empty() {
        true => None,
        false => Some(build_grid(cells)),
    }
}

/// Whether a group's own shape claims to be a table.
///
/// The property lives in one of the three property tables a shape may carry,
/// and PowerPoint writes this one into the tertiary. All three are searched
/// rather than assuming which, since the choice is the producer's.
fn is_table_group(body: &[u8]) -> bool {
    children(body)
        .filter(|record| {
            matches!(
                record.kind,
                ART_FOPT | ART_SECONDARY_FOPT | ART_TERTIARY_FOPT
            )
        })
        .any(|record| {
            // Each entry is a property id and a value; the count is in the
            // record's instance field, and complex values follow the entries.
            (0..record.instance() as usize).any(|index| {
                let at = index * 6;
                let Some(id) = u16_at(record.body, at) else {
                    return false;
                };
                let value = u32_at(record.body, at + 2).unwrap_or(0);
                // The top two bits flag a complex or blip value, not the id.
                id & 0x3FFF == PID_TABLE_PROPERTIES && value != 0
            })
        })
}

/// A shape's rectangle within its group, in the group's own coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Rect {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

/// Read a shape's `ChildAnchor`, which is what a shape inside a group carries.
fn child_anchor(body: &[u8]) -> Option<Rect> {
    let anchor = children(body).find(|record| record.kind == ART_CHILD_ANCHOR)?;
    Some(Rect {
        left: u32_at(anchor.body, 0)? as i32,
        top: u32_at(anchor.body, 4)? as i32,
        right: u32_at(anchor.body, 8)? as i32,
        bottom: u32_at(anchor.body, 12)? as i32,
    })
}

/// Rebuild the grid from the cells' rectangles.
///
/// The distinct left edges are the columns and the distinct top edges the rows,
/// which is what makes a merged cell recoverable: it starts on one boundary and
/// covers several, and the count of boundaries it covers is its span.
fn build_grid(cells: Vec<(Rect, Cell)>) -> Table {
    let mut lefts: Vec<i32> = cells.iter().map(|(rect, _)| rect.left).collect();
    lefts.sort_unstable();
    lefts.dedup();
    let mut tops: Vec<i32> = cells.iter().map(|(rect, _)| rect.top).collect();
    tops.sort_unstable();
    tops.dedup();

    // A column runs from its own left edge to the next one, and the last to
    // the furthest right edge of any cell.
    let edge = cells
        .iter()
        .map(|(rect, _)| rect.right)
        .max()
        .unwrap_or_default();
    let widths: Vec<f64> = lefts
        .iter()
        .enumerate()
        .map(|(index, left)| f64::from(lefts.get(index + 1).copied().unwrap_or(edge) - left))
        .filter(|width| *width > 0.0)
        .collect();
    let widths = match widths.len() == lefts.len() {
        true => widths,
        // A cell reaching left of a column start, or two columns at the same
        // edge: the widths would not line up with the grid, so state none.
        false => Vec::new(),
    };

    let mut rows: Vec<Vec<(i32, Cell)>> = vec![Vec::new(); tops.len()];
    for (rect, mut cell) in cells {
        let Ok(row) = tops.binary_search(&rect.top) else {
            continue;
        };
        cell.col_span = span(&lefts, rect.left, rect.right);
        cell.row_span = span(&tops, rect.top, rect.bottom);
        rows[row].push((rect.left, cell));
    }

    let rows = rows
        .into_iter()
        .filter(|row| !row.is_empty())
        .map(|mut row| {
            row.sort_by_key(|(left, _)| *left);
            Row {
                cells: row.into_iter().map(|(_, cell)| cell).collect(),
            }
        })
        .collect();

    Table {
        rows,
        // PowerPoint states a header row in the table's style rather than on
        // the row, and the first row of a slide table is one in every deck
        // anybody writes -- the same assumption the .pptx reader makes.
        header_rows: 1,
        // The rectangles give the real widths, so the typesetter gets the
        // proportions the author drew rather than an even split.
        column_widths: widths,
        caption: None,
    }
}

/// How many grid boundaries a cell covers, which is its span.
fn span(boundaries: &[i32], start: i32, end: i32) -> usize {
    boundaries
        .iter()
        .filter(|at| **at >= start && **at < end)
        .count()
        .max(1)
}

/// Turn a slide's shapes into a section.
fn build_slide(shapes: Vec<Shape>, notes: Vec<Shape>) -> Section {
    let mut title: Option<String> = None;
    let mut blocks: Vec<Block> = Vec::new();

    for shape in shapes {
        // Text types 0 and 6 are the title and centred-title placeholders.
        if matches!(shape.kind, TEXT_TYPE_TITLE | TEXT_TYPE_CENTER_TITLE) && title.is_none() {
            title = Some(clean(&shape.text));
            continue;
        }
        if let Some(table) = shape.table {
            blocks.push(Block::Table(table));
            continue;
        }
        // A body placeholder reads as a bulleted list, as it is drawn.
        let bulleted = matches!(shape.kind, TEXT_TYPE_BODY | TEXT_TYPE_CENTER_BODY);
        push_shape_blocks(&mut blocks, &shape.text, shape.props.as_ref(), bulleted);
    }

    Section {
        kind: SectionKind::Slide,
        title: title.filter(|text| !text.is_empty()),
        blocks,
        notes: notes_blocks(notes),
        page_size: None,
    }
}

/// Notes are prose, not the bulleted list a slide body is.
///
/// A notes page carries more than the note: PowerPoint puts slide-number, date
/// and footer placeholders on it too, and their text — often a single
/// substitution character — would otherwise be appended to what the author
/// wrote. Text type 2 is the notes body; the rest is page furniture.
fn notes_blocks(notes: Vec<Shape>) -> Vec<Block> {
    let mut blocks = Vec::new();
    for shape in notes
        .into_iter()
        .filter(|shape| shape.kind == TEXT_TYPE_NOTES)
    {
        push_shape_blocks(&mut blocks, &shape.text, shape.props.as_ref(), false);
    }
    blocks
}

/// Split a shape's text into paragraphs and file them.
///
/// PowerPoint separates paragraphs with a carriage return and uses a vertical
/// tab for a line break inside one.
fn push_shape_blocks(
    blocks: &mut Vec<Block>,
    text: &str,
    props: Option<&TextProps>,
    bulleted: bool,
) {
    for (level, content) in split_paragraphs(text, props) {
        let block = Block::Paragraph {
            content,
            align: Align::Left,
            indent: 0.0,
        };
        if !bulleted {
            blocks.push(block);
            continue;
        }
        let item = ListItem {
            blocks: vec![block],
            checked: None,
        };
        crate::formats::append_list_item(blocks, item, level, false, 1);
    }
}

/// Split a shape's text into paragraphs, each with its level and styled runs.
///
/// The formatting is stated as runs of characters counted in UTF-16 code units,
/// which is what the text was before it was decoded — so the cursors are
/// advanced by each character's own width in those units, not by one per
/// character. Where the shape states no properties every paragraph is at level
/// zero and unstyled, which is what this reader did for all of them before.
fn split_paragraphs(text: &str, props: Option<&TextProps>) -> Vec<(u8, Vec<Inline>)> {
    let empty_levels: &[(u32, u8)] = &[];
    let empty_runs: &[(u32, TextStyle)] = &[];
    let mut levels = Runs::new(props.map_or(empty_levels, |p| p.levels.as_slice()));
    let mut styles = Runs::new(props.map_or(empty_runs, |p| p.runs.as_slice()));

    let mut out: Vec<(u8, Vec<Inline>)> = Vec::new();
    let mut content: Vec<Inline> = Vec::new();
    let mut buffer = String::new();
    let mut buffer_style = TextStyle::default();
    // The level belongs to the paragraph, and is read at its first character.
    let mut level = levels.current().copied().unwrap_or(0);

    for character in text.chars() {
        let width = character.len_utf16();
        if matches!(character, '\r' | '\u{0B}') {
            flush_run(&mut content, &mut buffer, &buffer_style);
            finish_paragraph(&mut out, level, std::mem::take(&mut content));
            levels.advance(width);
            styles.advance(width);
            level = levels.current().copied().unwrap_or(0);
            continue;
        }
        let style = styles.current().cloned().unwrap_or_default();
        if !buffer.is_empty() && style != buffer_style {
            flush_run(&mut content, &mut buffer, &buffer_style);
        }
        if buffer.is_empty() {
            buffer_style = style;
        }
        buffer.push(character);
        levels.advance(width);
        styles.advance(width);
    }

    flush_run(&mut content, &mut buffer, &buffer_style);
    finish_paragraph(&mut out, level, content);
    out
}

/// Move the characters gathered so far into a run of their own.
///
/// Deliberately not trimmed: the space between a bold word and the one after it
/// falls at the boundary between two runs, and trimming each one would close it
/// up. The paragraph is trimmed as a whole instead.
fn flush_run(content: &mut Vec<Inline>, buffer: &mut String, style: &TextStyle) {
    let text = strip_controls(&std::mem::take(buffer));
    if !text.is_empty() {
        content.push(Inline::Run(Run::styled(text, style.clone())));
    }
}

/// File a finished paragraph, trimmed at its ends, dropping an empty one.
fn finish_paragraph(out: &mut Vec<(u8, Vec<Inline>)>, level: u8, mut content: Vec<Inline>) {
    while let Some(Inline::Run(run)) = content.first_mut() {
        run.text = run.text.trim_start().to_string();
        if !run.text.is_empty() {
            break;
        }
        content.remove(0);
    }
    while let Some(Inline::Run(run)) = content.last_mut() {
        run.text = run.text.trim_end().to_string();
        if !run.text.is_empty() {
            break;
        }
        content.pop();
    }
    if !content.is_empty() {
        out.push((level, content));
    }
}

/// Strip the control characters PowerPoint embeds in shape text.
/// Strip the control characters PowerPoint embeds, leaving the spacing.
///
/// A tab is real content; every other control character is PowerPoint's own
/// bookkeeping, and the two break characters are handled by the splitter.
fn strip_controls(text: &str) -> String {
    text.chars()
        .filter(|&character| !character.is_control() || character == '\t')
        .collect()
}

fn clean(text: &str) -> String {
    text.chars()
        .filter(|&character| {
            // A vertical tab is a line break and a CR a paragraph break; both
            // are handled by the splitter, so anything left is noise.
            !character.is_control() || character == '\t'
        })
        .collect::<String>()
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a record.
    fn record(ver_inst: u16, kind: u16, body: &[u8]) -> Vec<u8> {
        let mut out = ver_inst.to_le_bytes().to_vec();
        out.extend_from_slice(&kind.to_le_bytes());
        out.extend_from_slice(&(body.len() as u32).to_le_bytes());
        out.extend_from_slice(body);
        out
    }

    /// Build a container record.
    fn container(kind: u16, body: &[u8]) -> Vec<u8> {
        record(0x000F, kind, body)
    }

    fn text_chars(text: &str) -> Vec<u8> {
        let bytes: Vec<u8> = text.encode_utf16().flat_map(u16::to_le_bytes).collect();
        record(0, RT_TEXT_CHARS_ATOM, &bytes)
    }

    /// A `SlidePersistAtom` body: persist id, then the slide id twelve bytes in.
    fn persist_atom(persist: u32, slide: u32) -> Vec<u8> {
        let mut body = persist.to_le_bytes().to_vec();
        body.extend_from_slice(&[0u8; 8]); // flags, cTexts
        body.extend_from_slice(&slide.to_le_bytes());
        body.extend_from_slice(&[0u8; 4]); // reserved
        record(0, RT_SLIDE_PERSIST_ATOM, &body)
    }

    /// A notes record naming the slide it belongs to, carrying one line.
    fn notes_record(owner: u32, text: &str) -> Vec<u8> {
        let mut body = record(0, RT_NOTES_ATOM, &owner.to_le_bytes());
        body.extend(record(0, RT_TEXT_HEADER_ATOM, &[2])); // 2: the notes body
        body.extend(text_chars(text));
        container(RT_NOTES, &body)
    }

    /// Notes go to the slide their `NotesAtom` names, not to the slide in the
    /// same position in the notes list.
    ///
    /// The two lists run in parallel only while every slide has notes. Here the
    /// second slide has the only note, so pairing by position puts it on the
    /// first — which is what a deck saved from PowerPoint did, while the same
    /// deck as .pptx and .odp put it on the second.
    #[test]
    fn notes_go_to_the_slide_their_atom_names() {
        let mut stream: Vec<u8> = Vec::new();
        let mut persist: HashMap<u32, usize> = HashMap::new();

        for (id, title) in [(16u32, "First"), (17, "Second")] {
            persist.insert(id, stream.len());
            let mut body = record(0, RT_TEXT_HEADER_ATOM, &[0]); // 0: the title
            body.extend(text_chars(title));
            stream.extend(container(RT_SLIDE, &body));
        }
        persist.insert(18, stream.len());
        // Names slide id 257, which is the *second* slide.
        stream.extend(notes_record(257, "belongs to the second"));

        let mut slide_list = persist_atom(16, 256);
        slide_list.extend(persist_atom(17, 257));
        let notes_list = persist_atom(18, 256);

        let layout = Layout {
            persist,
            slide_list: &slide_list,
            notes_list: Some(&notes_list),
        };
        let sections = read_in_presentation_order(&stream, &layout);

        assert_eq!(sections.len(), 2, "{sections:?}");
        assert!(
            sections[0].notes.is_empty(),
            "the first slide has no notes: {:?}",
            sections[0].notes
        );
        assert!(
            !sections[1].notes.is_empty(),
            "the second slide has the note: {:?}",
            sections[1].notes
        );
    }

    #[test]
    fn distinguishes_containers_from_atoms() {
        let stream = container(RT_SLIDE, &record(0, RT_TEXT_BYTES_ATOM, b"hi"));
        let outer: Vec<_> = children(&stream).collect();
        assert_eq!(outer.len(), 1);
        assert!(outer[0].is_container());
        let inner: Vec<_> = children(outer[0].body).collect();
        assert!(!inner[0].is_container());
    }

    #[test]
    fn a_truncated_record_ends_the_walk() {
        // A header claiming more body than exists.
        let stream = vec![0x0F, 0x00, 0xEE, 0x03, 0xFF, 0xFF, 0xFF, 0xFF];
        assert_eq!(children(&stream).count(), 0);
    }

    #[test]
    fn collects_text_from_both_atom_encodings() {
        let mut body = record(0, RT_TEXT_HEADER_ATOM, &[1]);
        body.extend(text_chars("wide text"));
        body.extend(record(0, RT_TEXT_HEADER_ATOM, &[1]));
        body.extend(record(0, RT_TEXT_BYTES_ATOM, b"byte text"));

        let shapes = collect_shapes(&container(RT_SLIDE, &body));
        assert_eq!(shapes.len(), 2);
        assert_eq!(shapes[0].text, "wide text");
        assert_eq!(shapes[1].text, "byte text");
    }

    #[test]
    fn the_title_placeholder_becomes_the_section_title() {
        let mut body = record(0, RT_TEXT_HEADER_ATOM, &[0]); // title
        body.extend(text_chars("Slide Title"));
        body.extend(record(0, RT_TEXT_HEADER_ATOM, &[1])); // body
        body.extend(text_chars("first point\rsecond point"));

        let shapes = collect_shapes(&container(RT_SLIDE, &body));
        let section = build_slide(shapes, Vec::new());
        assert_eq!(section.title.as_deref(), Some("Slide Title"));

        // The body placeholder reads as a bulleted list.
        let Block::List(list) = &section.blocks[0] else {
            panic!("expected a list, got {:?}", section.blocks[0])
        };
        assert_eq!(list.items.len(), 2);
    }

    #[test]
    fn notes_are_prose_rather_than_bullets() {
        let mut body = record(0, RT_TEXT_HEADER_ATOM, &[2]);
        body.extend(text_chars("Remember the numbers."));
        let notes = collect_shapes(&container(RT_NOTES, &body));

        let blocks = notes_blocks(notes);
        assert!(
            matches!(blocks[0], Block::Paragraph { .. }),
            "notes bulleted"
        );
    }

    /// A `TextPFRun`: the characters it covers, its level, and empty masks.
    fn pf_run(count: u32, level: u16) -> Vec<u8> {
        let mut out = count.to_le_bytes().to_vec();
        out.extend_from_slice(&level.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes()); // TextPFException: no fields
        out
    }

    /// A `TextCFRun`. `style` is only read where `masks` claims the property.
    fn cf_run(count: u32, masks: u32, style: u16) -> Vec<u8> {
        let mut out = count.to_le_bytes().to_vec();
        out.extend_from_slice(&masks.to_le_bytes());
        if masks & 0x0000_FFFF != 0 {
            out.extend_from_slice(&style.to_le_bytes());
        }
        out
    }

    /// PowerPoint states a slide's outline levels and character formatting as
    /// runs of characters, not as markup.
    ///
    /// Taken from a real deck: a bold top-level bullet, two at the second
    /// level, and an italic one back at the top. Read without this atom the
    /// slide is four flat, unstyled lines -- which is what the .pptx of the
    /// same deck disagreed with.
    #[test]
    fn reads_outline_levels_and_character_formatting() {
        let text = "Revenue grew\rEMEA at 8 percent\rAPAC at 17 percent\rMargin held";
        // The counts include each paragraph's own carriage return, and the
        // last one counts a terminator that is not in the text.
        let mut props = pf_run(13, 0);
        props.extend(pf_run(37, 1));
        props.extend(pf_run(12, 0));
        props.extend(cf_run(13, 0x1, 0x1)); // bold
        props.extend(cf_run(37, 0x0, 0x0));
        props.extend(cf_run(12, 0x2, 0x2)); // italic

        let mut body = record(0, RT_TEXT_HEADER_ATOM, &[1]); // body placeholder
        body.extend(text_chars(text));
        body.extend(record(0, RT_STYLE_TEXT_PROP_ATOM, &props));

        let section = build_slide(collect_shapes(&container(RT_SLIDE, &body)), Vec::new());
        let mut document = SemanticDoc::default();
        document.sections.push(section);
        assert_eq!(
            crate::doc::to_markdown(&document),
            "- **Revenue grew**\n  - EMEA at 8 percent\n  - APAC at 17 percent\n- _Margin held_\n"
        );
    }

    /// An atom that does not account for the text is not used at all.
    ///
    /// Real files carry these: a master placeholder states one character of an
    /// eighty-character string. Since the run widths are what the walk depends
    /// on, a total that misses means the walk went wrong somewhere, and using
    /// what it produced would corrupt text that is otherwise correct.
    #[test]
    fn a_style_atom_that_does_not_cover_the_text_is_discarded() {
        let text = "one\rtwo";
        // Claims three characters of a seven-character string.
        let mut props = pf_run(3, 1);
        props.extend(cf_run(3, 0x1, 0x1));
        assert!(read_style_props(&props, text.encode_utf16().count()).is_none());

        // And the reader falls back to what it did before: flat and unstyled.
        let mut body = record(0, RT_TEXT_HEADER_ATOM, &[1]);
        body.extend(text_chars(text));
        body.extend(record(0, RT_STYLE_TEXT_PROP_ATOM, &props));
        let section = build_slide(collect_shapes(&container(RT_SLIDE, &body)), Vec::new());
        let mut document = SemanticDoc::default();
        document.sections.push(section);
        assert_eq!(crate::doc::to_markdown(&document), "- one\n- two\n");
    }

    /// A property is only set where the mask claims it.
    ///
    /// The `style` field is shared by every character property, so a bit set in
    /// it for one nobody claimed is left over from whatever wrote the file.
    #[test]
    fn a_style_bit_without_its_mask_is_ignored() {
        // Claims bold; the field also has the italic bit set, unclaimed.
        let props = cf_run(1, 0x1, 0x3);
        let mut atom = pf_run(1, 0);
        atom.extend(props);
        let read = read_style_props(&atom, 0).expect("covers the empty text");
        assert_eq!(read.runs.len(), 1);
        assert!(read.runs[0].1.bold);
        assert!(!read.runs[0].1.italic, "italic was not claimed");
    }

    /// The space between two differently styled words survives the split.
    #[test]
    fn a_run_boundary_does_not_swallow_the_space_across_it() {
        let text = "bold plain";
        // "bold" is styled; the space after it belongs to the plain run, and
        // falls exactly on the boundary between the two.
        let mut props = pf_run(11, 0);
        props.extend(cf_run(4, 0x1, 0x1));
        props.extend(cf_run(7, 0x0, 0x0));

        let mut body = record(0, RT_TEXT_HEADER_ATOM, &[2]); // notes: prose
        body.extend(text_chars(text));
        body.extend(record(0, RT_STYLE_TEXT_PROP_ATOM, &props));
        let shapes = collect_shapes(&container(RT_NOTES, &body));
        let mut blocks = Vec::new();
        for shape in shapes {
            push_shape_blocks(&mut blocks, &shape.text, shape.props.as_ref(), false);
        }
        let mut document = SemanticDoc::default();
        document.sections.push(Section {
            kind: SectionKind::Slide,
            title: None,
            blocks,
            notes: Vec::new(),
            page_size: None,
        });
        assert_eq!(crate::doc::to_markdown(&document), "**bold** plain\n");
    }

    /// An OfficeArt record: the instance nibble carries a count for property
    /// tables, so it is spelled out rather than folded into the version.
    fn art(instance: u16, kind: u16, body: &[u8]) -> Vec<u8> {
        record((instance << 4) | 0x0F, kind, body)
    }

    fn art_atom(instance: u16, kind: u16, body: &[u8]) -> Vec<u8> {
        record(instance << 4, kind, body)
    }

    /// A property table holding one property id and value.
    fn fopt(id: u16, value: u32) -> Vec<u8> {
        let mut body = id.to_le_bytes().to_vec();
        body.extend_from_slice(&value.to_le_bytes());
        art_atom(1, ART_TERTIARY_FOPT, &body)
    }

    /// One cell: its rectangle within the group, and its text.
    fn table_cell(rect: (i32, i32, i32, i32), text: &str) -> Vec<u8> {
        let mut body = Vec::new();
        for value in [rect.0, rect.1, rect.2, rect.3] {
            body.extend_from_slice(&value.to_le_bytes());
        }
        let mut cell = art_atom(0, ART_CHILD_ANCHOR, &body);
        cell.extend(record(0, RT_TEXT_HEADER_ATOM, &[1]));
        cell.extend(text_chars(text));
        art(0, ART_SP_CONTAINER, &cell)
    }

    /// PowerPoint draws a table as a group of shapes and nothing else.
    ///
    /// There is no table record: the group's own shape carries a
    /// `tableProperties` property, each cell is a shape with a rectangle, and
    /// the rules between them are shapes of no width or no height. Read as
    /// loose shapes the cells arrive as a column of stray paragraphs, which is
    /// what the .pptx of the same deck disagreed with.
    #[test]
    fn rebuilds_a_table_from_the_shapes_that_draw_it() {
        let mut group = art(0, ART_SP_CONTAINER, &fopt(PID_TABLE_PROPERTIES, 1));
        group.extend(table_cell((400, 400, 2000, 800), "Region"));
        group.extend(table_cell((2000, 400, 3600, 800), "Growth"));
        group.extend(table_cell((400, 800, 2000, 1200), "EMEA"));
        group.extend(table_cell((2000, 800, 3600, 1200), "8%"));
        // A rule: no height, and no text.
        group.extend(table_cell((400, 800, 3600, 800), ""));

        let shapes = collect_shapes(&container(RT_SLIDE, &art(0, ART_SPGR_CONTAINER, &group)));
        let section = build_slide(shapes, Vec::new());
        let mut document = SemanticDoc::default();
        document.sections.push(section);
        assert_eq!(
            crate::doc::to_markdown(&document),
            "| Region | Growth |\n| --- | --- |\n| EMEA | 8% |\n"
        );
    }

    /// A group that is not a table is walked as shapes, exactly as before.
    #[test]
    fn a_group_without_the_table_property_stays_loose_shapes() {
        let mut group = art(0, ART_SP_CONTAINER, &fopt(0x0004, 1));
        group.extend(table_cell((400, 400, 2000, 800), "one"));
        group.extend(table_cell((2000, 400, 3600, 800), "two"));

        let shapes = collect_shapes(&container(RT_SLIDE, &art(0, ART_SPGR_CONTAINER, &group)));
        assert_eq!(shapes.len(), 2);
        assert!(shapes.iter().all(|shape| shape.table.is_none()));
    }

    /// A merged cell covers several boundaries, and that is its span.
    #[test]
    fn a_cell_spanning_columns_keeps_its_span() {
        let mut group = art(0, ART_SP_CONTAINER, &fopt(PID_TABLE_PROPERTIES, 1));
        group.extend(table_cell((400, 400, 3600, 800), "spans both"));
        group.extend(table_cell((400, 800, 2000, 1200), "left"));
        group.extend(table_cell((2000, 800, 3600, 1200), "right"));

        let shapes = collect_shapes(&container(RT_SLIDE, &art(0, ART_SPGR_CONTAINER, &group)));
        let section = build_slide(shapes, Vec::new());
        let Some(Block::Table(table)) = section.blocks.first() else {
            panic!("no table: {:?}", section.blocks);
        };
        assert_eq!(table.rows[0].cells[0].col_span, 2);
        assert_eq!(table.rows[1].cells.len(), 2);
        // Two columns, of the widths the rectangles give.
        assert_eq!(table.column_widths, vec![1600.0, 1600.0]);
    }

    #[test]
    fn the_stream_order_fallback_finds_slides_and_pairs_notes() {
        let mut slide = record(0, RT_TEXT_HEADER_ATOM, &[0]);
        slide.extend(text_chars("First"));
        let mut notes = record(0, RT_TEXT_HEADER_ATOM, &[2]);
        notes.extend(text_chars("a note"));

        let mut stream = container(RT_SLIDE, &slide);
        stream.extend(container(RT_NOTES, &notes));

        let sections = read_in_stream_order(&stream);
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].title.as_deref(), Some("First"));
        assert_eq!(sections[0].notes.len(), 1);
    }

    #[test]
    fn a_persist_fragment_maps_ids_to_offsets() {
        // One run of two entries starting at id 3.
        let header = 3u32 | (2u32 << 20);
        let mut body = header.to_le_bytes().to_vec();
        body.extend_from_slice(&100u32.to_le_bytes());
        body.extend_from_slice(&200u32.to_le_bytes());

        let mut persist = HashMap::new();
        read_persist_fragment(&body, &mut persist);
        assert_eq!(persist.get(&3), Some(&100));
        assert_eq!(persist.get(&4), Some(&200));
    }

    #[test]
    fn a_later_edit_wins_over_an_earlier_one_for_the_same_id() {
        // The chain is walked newest-first, so an id already present must not
        // be overwritten by the older fragment that follows.
        let mut persist = HashMap::new();
        let newest = {
            let mut body = (7u32 | (1u32 << 20)).to_le_bytes().to_vec();
            body.extend_from_slice(&999u32.to_le_bytes());
            body
        };
        let oldest = {
            let mut body = (7u32 | (1u32 << 20)).to_le_bytes().to_vec();
            body.extend_from_slice(&111u32.to_le_bytes());
            body
        };
        read_persist_fragment(&newest, &mut persist);
        read_persist_fragment(&oldest, &mut persist);
        assert_eq!(
            persist.get(&7),
            Some(&999),
            "stale offset overwrote current"
        );
    }

    #[test]
    fn reads_slide_persist_ids_in_list_order() {
        let mut list = record(0, RT_SLIDE_PERSIST_ATOM, &5u32.to_le_bytes());
        list.extend(record(0, RT_SLIDE_PERSIST_ATOM, &3u32.to_le_bytes()));
        assert_eq!(persist_ids(&list), vec![5, 3]);
    }

    #[test]
    fn cleaning_strips_control_characters_but_keeps_tabs() {
        assert_eq!(
            clean("a\u{0}b\tc "),
            "a b\tc".replace(' ', ""),
            "controls dropped"
        );
    }
}
