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
    Align, Block, Inline, List, ListItem, Run, Section, SectionKind, SemanticDoc, TextStyle,
};

// Record types.
const RT_DOCUMENT: u16 = 0x03E8;
const RT_SLIDE: u16 = 0x03EE;
const RT_NOTES: u16 = 0x03F0;
const RT_USER_EDIT_ATOM: u16 = 0x0FF5;
const RT_PERSIST_DIRECTORY_ATOM: u16 = 0x1772;
const RT_SLIDE_LIST_WITH_TEXT: u16 = 0x0FF0;
const RT_SLIDE_PERSIST_ATOM: u16 = 0x03F3;
const RT_TEXT_HEADER_ATOM: u16 = 0x0F9F;
const RT_TEXT_CHARS_ATOM: u16 = 0x0FA0;
const RT_TEXT_BYTES_ATOM: u16 = 0x0FA8;
const RT_CRYPT_SESSION_10: u16 = 0x2F14;

// Text types, which say what role a text shape plays on its page.
const TEXT_TYPE_TITLE: u8 = 0;
const TEXT_TYPE_BODY: u8 = 1;
const TEXT_TYPE_NOTES: u8 = 2;
const TEXT_TYPE_CENTER_BODY: u8 = 5;
const TEXT_TYPE_CENTER_TITLE: u8 = 6;

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
    children(list)
        .filter(|record| record.kind == RT_SLIDE_PERSIST_ATOM)
        .filter_map(|record| u32_at(record.body, 0))
        .collect()
}

// ============================================================================
// Reading slides
// ============================================================================

/// Read slides through the resolved persist directory.
fn read_in_presentation_order(stream: &[u8], layout: &Layout) -> Vec<Section> {
    // Notes are listed alongside slides; pair them by position.
    let notes_ids = layout.notes_list.map(persist_ids).unwrap_or_default();

    let mut sections = Vec::new();
    for (index, id) in persist_ids(layout.slide_list).into_iter().enumerate() {
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
        let notes = notes_ids
            .get(index)
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
                }),
                // UTF-16 text.
                RT_TEXT_CHARS_ATOM => push_text(shapes, decode_utf16le(record.body)),
                // The low bytes of UTF-16 code units, for text that fits in
                // the Latin-1 range.
                RT_TEXT_BYTES_ATOM => {
                    push_text(shapes, record.body.iter().map(|&b| b as char).collect())
                }
                _ if record.is_container() => walk(record.body, depth + 1, budget, shapes),
                _ => {}
            }
        }
    }

    walk(data, 0, &mut budget, &mut shapes);
    shapes.retain(|shape| !shape.text.trim().is_empty());
    shapes
}

/// Append text to the shape being gathered, starting one if none is open.
fn push_text(shapes: &mut Vec<Shape>, text: String) {
    match shapes.last_mut() {
        Some(shape) => shape.text.push_str(&text),
        None => shapes.push(Shape { kind: 1, text }),
    }
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
        // A body placeholder reads as a bulleted list, as it is drawn.
        let bulleted = matches!(shape.kind, TEXT_TYPE_BODY | TEXT_TYPE_CENTER_BODY);
        push_shape_blocks(&mut blocks, &shape.text, bulleted);
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
        push_shape_blocks(&mut blocks, &shape.text, false);
    }
    blocks
}

/// Split a shape's text into paragraphs and file them.
///
/// PowerPoint separates paragraphs with a carriage return and uses a vertical
/// tab for a line break inside one.
fn push_shape_blocks(blocks: &mut Vec<Block>, text: &str, bulleted: bool) {
    for paragraph in text.split(['\r', '\u{0B}']) {
        let cleaned = clean(paragraph);
        if cleaned.is_empty() {
            continue;
        }
        let block = Block::Paragraph {
            content: vec![Inline::Run(Run::styled(cleaned, TextStyle::default()))],
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
        match blocks.last_mut() {
            Some(Block::List(list)) if !list.ordered => list.items.push(item),
            _ => blocks.push(Block::List(List {
                ordered: false,
                start: 1,
                items: vec![item],
            })),
        }
    }
}

/// Strip the control characters PowerPoint embeds in shape text.
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
