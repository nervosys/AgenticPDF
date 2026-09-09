// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Derived from anydoc (https://github.com/firecrawl/anydoc), MIT licensed,
// Copyright (c) 2026 Sideguide Technologies Inc. See LICENSE-MIT-anydoc.txt.
//
//! Word 97-2003 binary (`.doc`), per [MS-DOC].
//!
//! Nothing about this format is laid out the way a reader would like. The text
//! is not contiguous: a **piece table** maps character positions to byte offsets,
//! and each piece may be 8-bit code page text or UTF-16. Formatting is not
//! attached to the text either; it lives in **FKP pages** keyed by byte offset,
//! holding grpprls that are *deltas* over a style chain. Paragraph structure is
//! implied by control characters in the text stream — `\r` ends a paragraph,
//! `\u{7}` ends a table cell or row.
//!
//! So reading one is four passes that have to agree with each other:
//!
//! 1. **Piece table** → a character stream, remembering each character's byte
//!    offset and originating piece.
//! 2. **FKP pages** → runs of character and paragraph properties, keyed by byte
//!    offset, which is why step 1 has to keep them.
//! 3. **Style sheet** → the base each grpprl is a delta over.
//! 4. **Assembly** → walk the characters, resolving properties per character and
//!    splitting on the control characters.
//!
//! Character positions count UTF-16 units, and in a compressed piece a
//! double-byte character occupies two of them — a detail that silently
//! misaligns every formatting run if it is ignored.

use std::collections::HashMap;

use crate::PdfError;
use crate::container::ole::{Ole2, encoding_for_lid, is_lead_byte, u16_at, u32_at};
use crate::doc::{
    Align, Block, Cell, Inline, ListItem, Row, Run, Section, SemanticDoc, Table, TextStyle,
    inline_text,
};

use super::sprm::{CharProps, PapProps, apply_chpx, apply_pap, chpx_istd};
use super::stsh::{self, Stylesheet};

/// Number of list levels Word defines.
const LEVELS: usize = 9;

/// Parse a Word 97-2003 document.
pub fn parse(data: &[u8]) -> Result<SemanticDoc, PdfError> {
    let mut ole = Ole2::open(data)?;
    let word = ole.read("WordDocument")?;

    // The FIB's magic; anything else is not a Word 97 document.
    if u16_at(&word, 0) != Some(0xA5EC) {
        return Err(PdfError::Malformed("WordDocument: bad FIB magic".into()));
    }
    let flags = u16_at(&word, 0x0A).unwrap_or(0);
    if flags & 0x0100 != 0 {
        return Err(PdfError::Encrypted);
    }

    // Which of the two table streams is current is a flag, not a convention.
    let table = if flags & 0x0200 != 0 {
        ole.read_any(&["1Table", "0Table"])
    } else {
        ole.read_any(&["0Table", "1Table"])
    }
    .unwrap_or_default();
    let data_stream = ole.read_optional("Data").unwrap_or_default();

    // A document is one run of characters divided into parts, each counted in
    // the header: the main text, then the footnotes, headers, macros,
    // annotations, endnotes and finally the text boxes. The text-box count was
    // missing from this total, so a text box's text was never even extracted --
    // no amount of looking at the main text would have found it.
    let counts: Vec<usize> = [0x4C, 0x50, 0x54, 0x58, 0x5C, 0x60, 0x64, 0x68]
        .iter()
        .map(|&offset| u32_at(&word, offset).unwrap_or(0) as usize)
        .collect();
    let ccp_text = counts[0];
    let total_cp: usize = counts.iter().sum();
    // The text boxes begin after every part before them.
    let textbox_start: usize = counts[..6].iter().sum();
    let textbox_end = textbox_start + counts[6] + counts[7];

    let fc_clx = u32_at(&word, 0x1A2).unwrap_or(0) as usize;
    let lcb_clx = u32_at(&word, 0x1A6).unwrap_or(0) as usize;
    let (pieces, prcs) = if lcb_clx > 0 {
        parse_clx(&table, fc_clx, lcb_clx)?
    } else {
        (single_piece(&word), Vec::new())
    };

    // 8-bit text decodes in the document's own code page. For Far East
    // documents the FIB carries a second language id that takes precedence.
    let lid = if flags & 0x4000 != 0 {
        u16_at(&word, 0x3C)
            .filter(|&value| value != 0)
            .or_else(|| u16_at(&word, 0x06))
    } else {
        u16_at(&word, 0x06)
    };
    let encoding = encoding_for_lid(lid.unwrap_or(0));

    let text = extract_text(&word, &pieces, total_cp, encoding);
    let chpx = Runs::new(parse_fkps(&word, &table, 0xFA, FkpKind::Chpx, &data_stream));
    let papx = Runs::new(parse_fkps(
        &word,
        &table,
        0x102,
        FkpKind::Papx,
        &data_stream,
    ));
    let stylesheet = stsh::parse(&word, &table);
    let lists = parse_lists(&word, &table);

    // A note's reference is a 0x02 in the main text and its body is a
    // paragraph in the note subdocument, also beginning with 0x02. What says
    // which of the two kinds a reference is -- the subdocuments are separate,
    // and the references are mixed together in reading order -- is a table of
    // the character positions of each kind.
    //
    // Both offsets were found by looking for the table naming the positions
    // this document's references actually sit at, rather than taken from
    // memory: the footnote reference is at character 30 and the endnote at 57,
    // and exactly one table names each.
    let footnote_refs = reference_positions(&word, &table, 0x00AA);
    let endnote_refs = reference_positions(&word, &table, 0x020A);

    let footnote_start: usize = counts[0];
    let endnote_start: usize = counts[..5].iter().sum();
    let footnote_bodies = note_bodies(&text, footnote_start, counts[1]);
    let endnote_bodies = note_bodies(&text, endnote_start, counts[5]);

    let main_end = text.index_of_cp(ccp_text);
    let textbox_range = match counts[6] + counts[7] > 0 {
        true => Some((
            text.index_of_cp(textbox_start),
            text.index_of_cp(textbox_end),
        )),
        false => None,
    };
    let mut assembler = Assembler {
        text,
        chpx,
        papx,
        stylesheet,
        lists,
        prcs,
        piece_prcs: pieces.iter().map(|piece| piece.prm_prc).collect(),
        counters: HashMap::new(),
        footnote_refs,
        endnote_refs,
        note_bodies: Vec::new(),
        notes: Vec::new(),
        boxes: Vec::new(),
        data: data_stream.clone(),
        assets: Vec::new(),
        seen_pictures: HashMap::new(),
    };

    // Built before the body, so a reference found there resolves to one.
    let mut bodies: Vec<(bool, Vec<Block>)> = Vec::new();
    for (endnote, ranges) in [(false, footnote_bodies), (true, endnote_bodies)] {
        for (lo, hi) in ranges {
            let blocks = assembler.build(lo, hi);
            bodies.push((endnote, blocks));
        }
    }
    assembler.note_bodies = bodies;

    // A text box's text is not where the box is: it lives in a subdocument of
    // its own, and the drawing that anchors it says where it belongs. Two
    // tables state that between them -- the shapes, by position in the main
    // text, and the stories, by the shape each one fills.
    let anchors: HashMap<u32, u32> = shape_anchors(&word, &table).into_iter().collect();
    let mut placed: Vec<(usize, usize, usize)> = Vec::new();
    for (from, to, shape) in textbox_stories(&word, &table) {
        let Some(&cp) = anchors.get(&shape) else {
            continue;
        };
        placed.push((
            assembler.text.index_of_cp(cp as usize),
            assembler.text.index_of_cp(textbox_start + from as usize),
            assembler.text.index_of_cp(textbox_start + to as usize),
        ));
    }
    placed.sort_unstable();
    let mut boxes: Vec<(usize, Vec<Block>)> = Vec::new();
    for (anchor, lo, hi) in placed {
        let blocks = assembler.build(lo, hi);
        if !blocks.is_empty() {
            boxes.push((anchor, blocks));
        }
    }
    let anchored = !boxes.is_empty();
    assembler.boxes = boxes;

    let mut blocks = assembler.build(0, main_end);
    // Whatever the anchors did not account for still belongs in the document,
    // even if this reader cannot say where; better after the body than lost.
    let unplaced = std::mem::take(&mut assembler.boxes);
    for (_, box_blocks) in unplaced {
        blocks.extend(box_blocks);
    }
    if !anchored && let Some((lo, hi)) = textbox_range {
        blocks.extend(assembler.build(lo, hi));
    }
    let mut document = SemanticDoc {
        sections: vec![Section {
            blocks,
            ..Section::default()
        }],
        footnotes: std::mem::take(&mut assembler.notes),
        ..SemanticDoc::default()
    };
    // Registered in the order they were met, which is the order the ids handed
    // out while reading assumed.
    for (media, bytes) in std::mem::take(&mut assembler.assets) {
        document.add_asset(media, bytes);
    }
    Ok(document)
}

/// The character positions a reference table names.
///
/// A `PLC` is an array of positions followed by one entry each; only the
/// positions are wanted here, and the last is the end marker rather than a
/// reference.
fn reference_positions(word: &[u8], table: &[u8], fib_offset: usize) -> Vec<u32> {
    let fc = u32_at(word, fib_offset).unwrap_or(0) as usize;
    let lcb = u32_at(word, fib_offset + 4).unwrap_or(0) as usize;
    // Each entry is two bytes, so `lcb = 4 * (n + 1) + 2 * n`.
    if lcb < 10 || !(lcb - 4).is_multiple_of(6) || fc + lcb > table.len() {
        return Vec::new();
    }
    let count = (lcb - 4) / 6;
    (0..count)
        .filter_map(|index| u32_at(table, fc + index * 4))
        .collect()
}

/// The picture at an offset in the `Data` stream.
///
/// A `PICF` header says how long the whole record is and how long the header
/// itself is; what follows is OfficeArt, the same drawing format PowerPoint
/// stores its pictures in. The bytes sit inside a `BLIP` record, which in a
/// document is wrapped in the store entry that would name it if the file kept
/// its pictures in one place.
fn picture_at(data: &[u8], offset: usize) -> Option<(&'static str, Vec<u8>)> {
    let length = u32_at(data, offset)? as usize;
    let header = u16_at(data, offset + 4)? as usize;
    let art = data.get(offset + header..offset.checked_add(length)?)?;
    find_blip(art, 0).map(|(media, bytes)| (media, bytes.to_vec()))
}

/// Walk OfficeArt records for the first `BLIP`.
fn find_blip(data: &[u8], depth: usize) -> Option<(&'static str, &[u8])> {
    /// `msofbtBSE`: a store entry whose fixed header precedes the blip itself.
    const ART_BSE: u16 = 0xF007;
    const BSE_HEADER: usize = 36;

    if depth > 8 {
        return None;
    }
    let mut at = 0usize;
    while at + 8 <= data.len() {
        let version = u16_at(data, at)?;
        let kind = u16_at(data, at + 2)?;
        let length = u32_at(data, at + 4)? as usize;
        let body = data.get(at + 8..at + 8 + length)?;

        if let Some(found) = super::blip_payload(kind, version >> 4, body) {
            return Some(found);
        }
        // A container holds records; a store entry holds one behind a header
        // of its own, which is why its body is entered at an offset.
        let inner = match (version & 0x0F == 0x0F, kind == ART_BSE) {
            (_, true) => body.get(BSE_HEADER..),
            (true, false) => Some(body),
            _ => None,
        };
        if let Some(inner) = inner
            && let Some(found) = find_blip(inner, depth + 1)
        {
            return Some(found);
        }
        at += 8 + length;
    }
    None
}

/// The shapes anchored in the main text, as position and shape id.
///
/// `PlcfSpaMom` is a `PLC` of 26-byte `FSPA` entries whose first four bytes are
/// the shape id; the positions are in the main document.
fn shape_anchors(word: &[u8], table: &[u8]) -> Vec<(u32, u32)> {
    plc(word, table, 0x01DA, 26)
        .into_iter()
        .filter_map(|(cp, _, entry)| Some((u32_at(entry, 0)?, cp)))
        .collect()
}

/// The text-box stories, as a range in the text-box subdocument and the shape
/// each one fills.
///
/// `PlcfTxbxTxt` is a `PLC` of 22-byte `FTXBXS` entries whose `lid`, fourteen
/// bytes in, is the shape id. The last entry is a reserved one whose id names
/// no shape, and is dropped by not matching one.
fn textbox_stories(word: &[u8], table: &[u8]) -> Vec<(u32, u32, u32)> {
    plc(word, table, 0x025A, 22)
        .into_iter()
        .filter_map(|(from, to, entry)| Some((from, to, u32_at(entry, 14)?)))
        .collect()
}

/// Read a `PLC`: `n + 1` positions followed by `n` entries of `size` bytes.
fn plc<'a>(
    word: &[u8],
    table: &'a [u8],
    fib_offset: usize,
    size: usize,
) -> Vec<(u32, u32, &'a [u8])> {
    let fc = u32_at(word, fib_offset).unwrap_or(0) as usize;
    let lcb = u32_at(word, fib_offset + 4).unwrap_or(0) as usize;
    if lcb < 4 + 4 + size || !(lcb - 4).is_multiple_of(4 + size) || fc + lcb > table.len() {
        return Vec::new();
    }
    let count = (lcb - 4) / (4 + size);
    let entries = fc + 4 * (count + 1);
    (0..count)
        .filter_map(|index| {
            let from = u32_at(table, fc + index * 4)?;
            let to = u32_at(table, fc + index * 4 + 4)?;
            let at = entries + index * size;
            Some((from, to, table.get(at..at + size)?))
        })
        .collect()
}

/// Split a note subdocument into one range per note.
///
/// Each note's text begins with the same 0x02 that marks its reference, so the
/// subdocument divides at those without needing the table that also states it
/// -- which is as well, since that table's positions are stated relative to a
/// base this reader would have to guess at.
fn note_bodies(text: &TextStream, start_cp: usize, count: usize) -> Vec<(usize, usize)> {
    if count == 0 {
        return Vec::new();
    }
    let lo = text.index_of_cp(start_cp);
    let hi = text.index_of_cp(start_cp + count);
    let mut starts: Vec<usize> = (lo..hi.min(text.chars.len()))
        .filter(|&at| text.chars[at] == '\u{02}')
        .collect();
    if starts.is_empty() {
        return Vec::new();
    }
    starts.push(hi);
    starts.windows(2).map(|pair| (pair[0], pair[1])).collect()
}

// ============================================================================
// Piece table
// ============================================================================

struct Piece {
    cp_start: usize,
    cp_end: usize,
    /// Byte offset of the piece's text in the WordDocument stream.
    fc: usize,
    /// 8-bit code page text rather than UTF-16.
    compressed: bool,
    /// Index into the Clx's property-modifier array, when the piece has one.
    prm_prc: Option<usize>,
}

/// Parse the Clx: a run of property modifiers followed by the piece table.
fn parse_clx(table: &[u8], fc: usize, lcb: usize) -> Result<(Vec<Piece>, Vec<Vec<u8>>), PdfError> {
    let clx = table
        .get(fc..)
        .and_then(|rest| rest.get(..lcb))
        .ok_or_else(|| PdfError::Malformed("Clx out of bounds".into()))?;

    let mut prcs: Vec<Vec<u8>> = Vec::new();
    let mut pos = 0usize;
    loop {
        let rest = clx
            .get(pos..)
            .ok_or_else(|| PdfError::Malformed("malformed Clx".into()))?;
        match rest.first() {
            // A property modifier.
            Some(1) => {
                let cb =
                    u16_at(rest, 1).ok_or_else(|| PdfError::Malformed("bad Prc".into()))? as usize;
                if let Some(grpprl) = rest.get(3..).and_then(|payload| payload.get(..cb)) {
                    prcs.push(grpprl.to_vec());
                }
                pos += 3 + cb;
            }
            // The piece table itself, which ends the Clx.
            Some(2) => {
                let len = u32_at(rest, 1)
                    .ok_or_else(|| PdfError::Malformed("bad piece table".into()))?
                    as usize;
                let plc = rest
                    .get(5..)
                    .and_then(|payload| payload.get(..len))
                    .ok_or_else(|| PdfError::Malformed("piece table out of bounds".into()))?;
                return Ok((parse_piece_table(plc, &mut prcs)?, prcs));
            }
            _ => return Err(PdfError::Malformed("malformed Clx".into())),
        }
    }
}

/// Parse the piece table: character positions, then one descriptor per piece.
fn parse_piece_table(plc: &[u8], prcs: &mut Vec<Vec<u8>>) -> Result<Vec<Piece>, PdfError> {
    if plc.len() < 4 + 8 {
        return Err(PdfError::Malformed("empty piece table".into()));
    }
    let count = (plc.len() - 4) / 12;
    let mut pieces = Vec::with_capacity(count);

    for index in 0..count {
        let cp_start = u32_at(plc, index * 4)
            .ok_or_else(|| PdfError::Malformed("bad character position".into()))?
            as usize;
        let cp_end = u32_at(plc, (index + 1) * 4)
            .ok_or_else(|| PdfError::Malformed("bad character position".into()))?
            as usize;

        let descriptor = (count + 1) * 4 + index * 8;
        let raw = u32_at(plc, descriptor + 2)
            .ok_or_else(|| PdfError::Malformed("bad piece descriptor".into()))?;
        let prm = u16_at(plc, descriptor + 6).unwrap_or(0);

        // Bit 30 marks 8-bit text, and the offset is then doubled.
        let compressed = raw & 0x4000_0000 != 0;
        let fc = (raw & 0x3FFF_FFFF) as usize;
        let fc = if compressed { fc / 2 } else { fc };

        // A modifier is either an index into the Prc array or, in its
        // compressed form, one property encoded inline.
        let prm_prc = if prm & 1 != 0 {
            let index = (prm >> 1) as usize;
            (index < prcs.len()).then_some(index)
        } else if prm != 0 {
            prm0_grpprl(prm).map(|grpprl| {
                prcs.push(grpprl);
                prcs.len() - 1
            })
        } else {
            None
        };

        pieces.push(Piece {
            cp_start,
            cp_end,
            fc,
            compressed,
            prm_prc,
        });
    }
    Ok(pieces)
}

/// Expand a compressed piece modifier into a one-sprm grpprl.
///
/// Only the properties this reader models are worth materialising; the rest
/// have no representable effect.
fn prm0_grpprl(prm: u16) -> Option<Vec<u8>> {
    let selector = (prm >> 1) & 0x7F;
    let value = (prm >> 8) as u8;
    let sprm: u16 = match selector {
        0x0C => 0x260A, // list level
        0x18 => 0x2416, // in table
        0x19 => 0x2417, // row terminator
        0x55 => 0x0835, // bold
        0x56 => 0x0836, // italic
        0x57 => 0x0837, // strikethrough
        0x78 => 0x2640, // outline level
        _ => return None,
    };
    Some(vec![(sprm & 0xFF) as u8, (sprm >> 8) as u8, value])
}

/// The single-piece fallback for documents with no piece table.
fn single_piece(word: &[u8]) -> Vec<Piece> {
    let start = u32_at(word, 0x18).unwrap_or(0) as usize;
    let end = u32_at(word, 0x1C).unwrap_or(0) as usize;
    if end <= start {
        return Vec::new();
    }
    vec![Piece {
        cp_start: 0,
        cp_end: end - start,
        fc: start,
        compressed: true,
        prm_prc: None,
    }]
}

// ============================================================================
// Text extraction
// ============================================================================

/// The document's characters, with the provenance each later pass needs.
struct TextStream {
    chars: Vec<char>,
    /// Byte offset of each character, for looking up formatting runs.
    fcs: Vec<u32>,
    /// Character position of each character, for the position-indexed tables.
    cps: Vec<u32>,
    /// Originating piece, for applying piece-level modifiers.
    piece_of: Vec<u32>,
}

impl TextStream {
    /// The first character index at or after a character position.
    fn index_of_cp(&self, cp: usize) -> usize {
        self.cps.partition_point(|&value| (value as usize) < cp)
    }
}

fn extract_text(
    word: &[u8],
    pieces: &[Piece],
    total_cp: usize,
    encoding: &'static encoding_rs::Encoding,
) -> TextStream {
    let mut text = TextStream {
        chars: Vec::new(),
        fcs: Vec::new(),
        cps: Vec::new(),
        piece_of: Vec::new(),
    };
    let mut cp = 0usize;

    for (index, piece) in pieces.iter().enumerate() {
        if cp >= total_cp {
            break;
        }
        let len = piece
            .cp_end
            .saturating_sub(piece.cp_start)
            .min(total_cp - cp);

        if piece.compressed {
            let Some(bytes) = word.get(piece.fc..).and_then(|rest| rest.get(..len)) else {
                continue;
            };
            // One character position per *byte*, so a double-byte character
            // spans two positions and its offset is the lead byte's. Counting
            // by character instead misaligns every formatting run after it.
            let mut at = 0usize;
            while at < bytes.len() {
                let width = if is_lead_byte(encoding, bytes[at]) && at + 1 < bytes.len() {
                    2
                } else {
                    1
                };
                let (decoded, _) = encoding.decode_without_bom_handling(&bytes[at..at + width]);
                for character in decoded.chars() {
                    text.chars.push(character);
                    text.fcs.push((piece.fc + at) as u32);
                    text.cps.push(cp as u32);
                    text.piece_of.push(index as u32);
                }
                cp += width;
                at += width;
            }
        } else {
            let Some(byte_len) = len.checked_mul(2) else {
                continue;
            };
            let Some(bytes) = word.get(piece.fc..).and_then(|rest| rest.get(..byte_len)) else {
                continue;
            };
            let units: Vec<u16> = bytes
                .as_chunks::<2>()
                .0
                .iter()
                .map(|pair| u16::from_le_bytes(*pair))
                .collect();
            let mut unit = 0usize;
            for result in char::decode_utf16(units.iter().copied()) {
                let character = result.unwrap_or('\u{FFFD}');
                text.chars.push(character);
                text.fcs.push((piece.fc + unit * 2) as u32);
                text.cps.push(cp as u32);
                text.piece_of.push(index as u32);
                unit += character.len_utf16();
                cp += character.len_utf16();
            }
        }
    }
    text
}

// ============================================================================
// Formatting runs
// ============================================================================

#[derive(Clone, Copy, PartialEq)]
enum FkpKind {
    Chpx,
    Papx,
}

#[derive(Default)]
struct RunProps {
    /// The raw character grpprl; applying it needs the style base.
    chpx: Vec<u8>,
    istd: u16,
    pap: PapProps,
}

struct Run_ {
    fc_start: u32,
    fc_end: u32,
    props: RunProps,
}

/// Formatting runs, searchable by byte offset.
struct Runs {
    runs: Vec<Run_>,
}

impl Runs {
    fn new(mut runs: Vec<Run_>) -> Runs {
        runs.sort_by_key(|run| run.fc_start);
        Runs { runs }
    }

    fn lookup(&self, fc: u32) -> Option<&RunProps> {
        let index = self.runs.partition_point(|run| run.fc_start <= fc);
        if index == 0 {
            return None;
        }
        let run = &self.runs[index - 1];
        (fc < run.fc_end).then_some(&run.props)
    }
}

/// Read the FKP pages a position table points at.
fn parse_fkps(
    word: &[u8],
    table: &[u8],
    fib_offset: usize,
    kind: FkpKind,
    data: &[u8],
) -> Vec<Run_> {
    let mut runs = Vec::new();
    let fc = u32_at(word, fib_offset).unwrap_or(0) as usize;
    let lcb = u32_at(word, fib_offset + 4).unwrap_or(0) as usize;
    let Some(plc) = table.get(fc..fc.saturating_add(lcb)) else {
        return runs;
    };
    if plc.len() < 8 {
        return runs;
    }

    let count = (plc.len() - 4) / 8;
    for index in 0..count {
        let Some(raw) = u32_at(plc, (count + 1) * 4 + index * 4) else {
            continue;
        };
        // FKP pages are 512-byte pages of the WordDocument stream.
        let page_number = (raw & 0x3F_FFFF) as usize;
        let Some(offset) = page_number.checked_mul(512) else {
            continue;
        };
        let Some(page) = word.get(offset..).and_then(|rest| rest.get(..512)) else {
            continue;
        };
        parse_fkp_page(page, kind, data, &mut runs);
    }
    runs
}

/// Read one 512-byte FKP page.
///
/// The last byte is the entry count; offsets to the grpprls are stored in
/// half-words counted from the page start, which is why they are doubled.
fn parse_fkp_page(page: &[u8], kind: FkpKind, data: &[u8], runs: &mut Vec<Run_>) {
    let count = page[511] as usize;
    if count == 0 {
        return;
    }
    let entry_size = if kind == FkpKind::Papx { 13 } else { 1 };

    for index in 0..count {
        let (Some(fc_start), Some(fc_end)) =
            (u32_at(page, index * 4), u32_at(page, (index + 1) * 4))
        else {
            continue;
        };
        let Some(&word_offset) = page.get((count + 1) * 4 + index * entry_size) else {
            continue;
        };

        let mut props = RunProps::default();
        if word_offset != 0 {
            let at = word_offset as usize * 2;
            match kind {
                FkpKind::Chpx => {
                    if let Some(&cb) = page.get(at)
                        && let Some(grpprl) = page
                            .get(at..)
                            .and_then(|rest| rest.get(1..))
                            .and_then(|rest| rest.get(..cb as usize))
                    {
                        props.chpx = grpprl.to_vec();
                    }
                }
                FkpKind::Papx => {
                    if let Some(&cb) = page.get(at) {
                        // A zero first byte means the real length is in the
                        // next byte and the payload is a word longer.
                        let (start, len) = if cb == 0 {
                            let extended = page.get(at + 1).copied().unwrap_or(0) as usize;
                            (at + 2, extended * 2)
                        } else {
                            (at + 1, cb as usize * 2 - 1)
                        };
                        if let Some(grpprl) = page.get(start..).and_then(|rest| rest.get(..len))
                            && grpprl.len() >= 2
                        {
                            props.istd = u16::from_le_bytes([grpprl[0], grpprl[1]]);
                            apply_pap(&grpprl[2..], data, &mut props.pap);
                        }
                    }
                }
            }
        }
        runs.push(Run_ {
            fc_start,
            fc_end,
            props,
        });
    }
}

// ============================================================================
// List tables
// ============================================================================

/// One list's per-level numbering formats.
#[derive(Debug, Clone)]
struct ListDef {
    /// List identity: numbering state is shared by every override of it.
    lsid: u32,
    ordered: [bool; LEVELS],
    start: [u64; LEVELS],
}

impl ListDef {
    fn unknown() -> ListDef {
        ListDef {
            lsid: u32::MAX,
            ordered: [false; LEVELS],
            start: [1; LEVELS],
        }
    }
}

/// The document's list definitions, keyed by the index paragraphs reference.
#[derive(Debug, Default)]
struct Lists {
    by_ilfo: HashMap<u16, ListDef>,
}

/// Parse the list definition and override tables.
fn parse_lists(word: &[u8], table: &[u8]) -> Lists {
    let mut lists = Lists::default();
    let lst_fc = u32_at(word, 0x2E2).unwrap_or(0) as usize;
    let lst_lcb = u32_at(word, 0x2E6).unwrap_or(0) as usize;
    let lfo_fc = u32_at(word, 0x2EA).unwrap_or(0) as usize;
    let lfo_lcb = u32_at(word, 0x2EE).unwrap_or(0) as usize;
    if lst_lcb == 0 {
        return lists;
    }

    let by_lsid = parse_list_definitions(table, lst_fc, lst_lcb);
    let Some(lfo) = table.get(lfo_fc..lfo_fc.saturating_add(lfo_lcb)) else {
        return lists;
    };

    // The override table maps the 1-based index paragraphs use onto a list.
    let count = u32_at(lfo, 0).unwrap_or(0) as usize;
    for index in 0..count.min(0x1000) {
        let Some(lsid) = u32_at(lfo, 4 + index * 16) else {
            break;
        };
        if let Some(def) = by_lsid.get(&lsid) {
            lists.by_ilfo.insert((index + 1) as u16, def.clone());
        }
    }
    lists
}

/// Parse the list definitions and the level structures that follow them.
fn parse_list_definitions(table: &[u8], fc: usize, lcb: usize) -> HashMap<u32, ListDef> {
    const LSTF_SIZE: usize = 28;
    let mut out = HashMap::new();
    let Some(plf) = table.get(fc..) else {
        return out;
    };
    let Some(count) = u16_at(plf, 0).map(|value| value as usize) else {
        return out;
    };

    // The declared length covers only the fixed records; the variable level
    // structures follow it, uncounted.
    let mut simple_flags: Vec<(u32, bool)> = Vec::with_capacity(count);
    let mut pos = 2usize;
    for _ in 0..count {
        let Some(record) = plf.get(pos..).and_then(|rest| rest.get(..LSTF_SIZE)) else {
            return out;
        };
        let Some(lsid) = u32_at(record, 0) else {
            return out;
        };
        simple_flags.push((lsid, record[26] & 0x01 != 0));
        pos += LSTF_SIZE;
    }

    // Level structures begin after the fixed array.
    let mut at = fc + lcb;
    for (lsid, simple) in simple_flags {
        let levels = if simple { 1 } else { LEVELS };
        let mut def = ListDef {
            lsid,
            ordered: [false; LEVELS],
            start: [1; LEVELS],
        };
        for level in 0..levels {
            let Some((ordered, start, next)) = parse_level(table, at) else {
                break;
            };
            if level < LEVELS {
                def.ordered[level] = ordered;
                def.start[level] = start;
            }
            at = next;
        }
        // A simple list uses its single level everywhere.
        if simple {
            let (ordered, start) = (def.ordered[0], def.start[0]);
            def.ordered = [ordered; LEVELS];
            def.start = [start; LEVELS];
        }
        out.insert(lsid, def);
    }
    out
}

/// Parse one level structure, returning its format and where the next begins.
fn parse_level(table: &[u8], at: usize) -> Option<(bool, u64, usize)> {
    // The fixed part is 28 bytes: start-at, number format, then flags and the
    // two variable-length grpprls whose sizes are at offsets 25 and 26.
    let start = u32_at(table, at)? as u64;
    let format = *table.get(at + 4)?;
    // The two grpprl sizes sit at 24 and 25, after `dxaIndentSav` and the four
    // unused bytes. Reading them at 25 and 26 drifts the walk by one byte per
    // level, and the drift accumulates across every list in the file: the third
    // list in one document read its number format out of the middle of another
    // structure and came out as bullets where it should have been "1." and "2.".
    let cb_grpprl_chpx = *table.get(at + 24)? as usize;
    let cb_grpprl_papx = *table.get(at + 25)? as usize;

    // Then a length-prefixed number text.
    let after_grpprls = at + 28 + cb_grpprl_papx + cb_grpprl_chpx;
    // The number text is a length-prefixed *Unicode* string: the count is in
    // characters and each takes two bytes. Advancing by the count alone drifts
    // one byte per character, and the drift carries into every list after it --
    // a one-character bullet was enough to make the next list's start-at read
    // 496 instead of 1.
    let text_len = u16_at(table, after_grpprls)? as usize;
    let next = after_grpprls + 2 + text_len * 2;

    // Format 23 is a bullet; everything else numbers.
    Some((format != 23, start, next))
}

// ============================================================================
// Assembly
// ============================================================================

struct Assembler {
    text: TextStream,
    chpx: Runs,
    papx: Runs,
    stylesheet: Stylesheet,
    lists: Lists,
    prcs: Vec<Vec<u8>>,
    piece_prcs: Vec<Option<usize>>,
    counters: HashMap<u32, [u64; LEVELS]>,
    /// Character positions of the footnote and endnote references, which say
    /// which of the two kinds each 0x02 in the main text is.
    footnote_refs: Vec<u32>,
    endnote_refs: Vec<u32>,
    /// Note bodies, footnotes first, each with whether it is an endnote.
    note_bodies: Vec<(bool, Vec<Block>)>,
    /// Notes placed so far, in the order their references were met.
    notes: Vec<crate::doc::Footnote>,
    /// Text boxes waiting for the paragraph they are anchored in to end,
    /// ordered by that anchor.
    boxes: Vec<(usize, Vec<Block>)>,
    /// The `Data` stream, where a picture's bytes are.
    data: Vec<u8>,
    /// Pictures registered so far, and the offset each was read from, so a
    /// picture used twice is stored once.
    assets: Vec<(String, Vec<u8>)>,
    seen_pictures: HashMap<usize, String>,
}

/// A paragraph's resolved properties.
struct EffectivePap {
    istd: u16,
    props: PapProps,
}

impl Assembler {
    /// Place the note a reference at this character names.
    ///
    /// The references are met in reading order and the bodies are held in that
    /// same order within each kind, so each kind is taken in turn. A reference
    /// whose body is missing yields nothing rather than a number pointing at
    /// no note.
    fn place_note(&mut self, index: usize) -> Option<usize> {
        let cp = *self.text.cps.get(index)?;
        let endnote = match (
            self.footnote_refs.contains(&cp),
            self.endnote_refs.contains(&cp),
        ) {
            (true, _) => false,
            (_, true) => true,
            // A 0x02 the tables do not name is not a reference.
            _ => return None,
        };
        let at = self
            .note_bodies
            .iter()
            .position(|(kind, _)| *kind == endnote)?;
        let (_, blocks) = self.note_bodies.remove(at);
        if blocks.is_empty() {
            return None;
        }
        let number = self.notes.len();
        self.notes.push(crate::doc::Footnote {
            label: None,
            blocks,
        });
        Some(number)
    }

    /// Walk the character range, emitting blocks.
    fn build(&mut self, lo: usize, hi: usize) -> Vec<Block> {
        let mut blocks: Vec<Block> = Vec::new();
        let mut content: Vec<Inline> = Vec::new();

        // Table state: cells accumulate into a row, rows into a table.
        let mut cell_blocks: Vec<Block> = Vec::new();
        let mut row: Vec<Cell> = Vec::new();
        let mut rows: Vec<Row> = Vec::new();
        // One edge list per row, so a merged cell can be measured against the
        // columns the rest of the table uses.
        let mut edges: Vec<Vec<i32>> = Vec::new();
        let mut header_rows = 0usize;

        // Fields nest, so this is a stack rather than a flag.
        let mut fields: Vec<Field> = Vec::new();
        let mut index = lo;
        let end = hi.min(self.text.chars.len());
        while index < end {
            let character = self.text.chars[index];
            let fc = self.text.fcs[index];

            match character {
                // Paragraph, cell and row terminators.
                '\r' | '\u{7}' | '\u{c}' | '\u{e}' => {
                    let pap = self.effective_pap(fc, index);
                    let inlines = std::mem::take(&mut content);
                    let is_cell_mark = character == '\u{7}';

                    if pap.props.in_table.unwrap_or(false) || is_cell_mark {
                        // A row terminator carries the row's properties; a cell
                        // terminator closes a cell; anything else is a
                        // paragraph inside the current cell.
                        if is_cell_mark && pap.props.ttp.unwrap_or(false) {
                            Self::push_paragraph(&mut cell_blocks, inlines);
                            if !cell_blocks.is_empty() {
                                row.push(Cell {
                                    blocks: std::mem::take(&mut cell_blocks),
                                    ..Cell::default()
                                });
                            }
                            if !row.is_empty() {
                                if pap.props.tap.as_ref().is_some_and(|tap| tap.header)
                                    && header_rows == rows.len()
                                {
                                    header_rows += 1;
                                }
                                edges.push(
                                    pap.props
                                        .tap
                                        .as_ref()
                                        .map(|tap| {
                                            tap.boundaries.iter().map(|at| *at as i32).collect()
                                        })
                                        .unwrap_or_default(),
                                );
                                rows.push(Row {
                                    cells: std::mem::take(&mut row),
                                });
                            }
                        } else if is_cell_mark {
                            Self::push_paragraph(&mut cell_blocks, inlines);
                            row.push(Cell {
                                blocks: std::mem::take(&mut cell_blocks),
                                ..Cell::default()
                            });
                        } else {
                            Self::push_paragraph(&mut cell_blocks, inlines);
                        }
                    } else {
                        self.flush_table(
                            &mut blocks,
                            &mut rows,
                            &mut row,
                            &mut cell_blocks,
                            &mut header_rows,
                            &mut edges,
                        );
                        self.emit_paragraph(&pap, inlines, &mut blocks);
                        // A box anchored in the paragraph that just ended
                        // follows it, which is where the .docx, .odt and .rtf
                        // of the same document all put it.
                        while self.boxes.first().is_some_and(|(at, _)| *at <= index) {
                            let (_, box_blocks) = self.boxes.remove(0);
                            blocks.extend(box_blocks);
                        }
                    }
                }
                // The character a picture is anchored at. It stands in for
                // the picture and carries only the offset to it, so read as a
                // character it is nothing at all -- which is what the .doc of
                // a document with a figure in it reported.
                '\u{01}' => {
                    if let Some(image) = self.place_picture(fc, index) {
                        content.push(Inline::Image(image));
                    }
                }
                // A line break inside a paragraph.
                '\u{b}' => content.push(Inline::Break),
                // Tabs and non-breaking hyphens have printable equivalents.
                '\t' => {
                    let style = self.char_style(fc, index);
                    push_char(&mut content, ' ', style);
                }
                '\u{1e}' => {
                    let style = self.char_style(fc, index);
                    push_char(&mut content, '-', style);
                }
                // A note's reference. Its body is in a subdocument of its
                // own, and the tables say which of the two kinds it is.
                '\u{02}' => {
                    if let Some(note) = self.place_note(index) {
                        content.push(Inline::FootnoteRef { index: note });
                    }
                }
                // A field is written inline: 0x13 opens it, 0x14 separates
                // its instruction from its result, and 0x15 closes it. The
                // instruction is not text to show -- read as if it were, a
                // hyperlink came out as `HYPERLINK "https://..."link`, where
                // the .docx and .odt of the same document produced a link.
                '\u{13}' => {
                    fields.push(Field {
                        instruction: String::new(),
                        start: content.len(),
                        reading_result: false,
                    });
                }
                '\u{14}' => {
                    if let Some(field) = fields.last_mut() {
                        field.reading_result = true;
                        field.start = content.len();
                    }
                }
                '\u{15}' => {
                    if let Some(field) = fields.pop() {
                        finish_field(&mut content, field);
                    }
                }
                // Field markers, picture placeholders and other control
                // characters carry no text of their own.
                character if character.is_control() => {}
                // Struck out by a tracked change: still in the file, and not
                // part of what the document says.
                _ if self.is_deleted(fc, index) => {}
                character => {
                    // An instruction names the field rather than showing text.
                    if let Some(field) = fields.last_mut()
                        && !field.reading_result
                    {
                        // Bounded: a malformed document must not be able to
                        // grow one without end.
                        if field.instruction.len() < 4096 {
                            field.instruction.push(character);
                        }
                        index += 1;
                        continue;
                    }
                    let style = self.char_style(fc, index);
                    push_char(&mut content, character, style);
                }
            }
            index += 1;
        }

        self.flush_table(
            &mut blocks,
            &mut rows,
            &mut row,
            &mut cell_blocks,
            &mut header_rows,
            &mut edges,
        );
        if !inline_text(&content).trim().is_empty() {
            blocks.push(Block::Paragraph {
                content,
                align: Align::Left,
                indent: 0.0,
            });
        }
        blocks
    }

    /// Character formatting in specification order: style chain, then the
    /// character grpprl, then any piece-level modifier.
    /// Whether the character at this position was struck out by a tracked
    /// change. Such text is still in the file but is not what it says.
    fn is_deleted(&self, fc: u32, index: usize) -> bool {
        self.char_props(fc, index).deleted
    }

    fn char_style(&self, fc: u32, index: usize) -> TextStyle {
        to_text_style(self.char_props(fc, index))
    }

    /// The character properties in force, before they become a text style.
    fn char_props(&self, fc: u32, index: usize) -> CharProps {
        let para_istd = self.papx.lookup(fc).map(|props| props.istd).unwrap_or(0);
        let chpx = self
            .chpx
            .lookup(fc)
            .map(|props| props.chpx.as_slice())
            .unwrap_or(&[]);
        // A character style reference in the grpprl overrides the paragraph's.
        let istd = chpx_istd(chpx).unwrap_or(para_istd);
        let base = self.stylesheet.get(istd).chp;

        let mut props = apply_chpx(chpx, base, base);
        if let Some(&piece) = self.text.piece_of.get(index)
            && let Some(prm) = self.piece_prm(piece as usize)
        {
            props = apply_chpx(prm, props, base);
        }
        props
    }

    fn piece_prm(&self, piece: usize) -> Option<&[u8]> {
        let index = (*self.piece_prcs.get(piece)?)?;
        self.prcs.get(index).map(Vec::as_slice)
    }

    /// Paragraph properties in specification order.
    fn effective_pap(&self, fc: u32, index: usize) -> EffectivePap {
        let (istd, delta) = match self.papx.lookup(fc) {
            Some(props) => (props.istd, props.pap.clone()),
            None => (0, PapProps::default()),
        };
        let mut props = self.stylesheet.get(istd).pap.clone().merge(delta);

        if let Some(&piece) = self.text.piece_of.get(index)
            && let Some(prm) = self.piece_prm(piece as usize)
        {
            let mut piece_delta = PapProps::default();
            apply_pap(prm, &[], &mut piece_delta);
            props = props.merge(piece_delta);
        }
        EffectivePap { istd, props }
    }

    /// File a finished paragraph as a heading, list item or paragraph.
    fn emit_paragraph(
        &mut self,
        pap: &EffectivePap,
        content: Vec<Inline>,
        blocks: &mut Vec<Block>,
    ) {
        if crate::doc::inlines_are_empty(&content) {
            return;
        }
        let style = self.stylesheet.get(pap.istd);

        // A heading style names its level; an outline level states it directly.
        if let Some(level) = style.heading.or(pap.props.outline.flatten()) {
            blocks.push(Block::Heading {
                level: level.clamp(1, 6),
                content,
            });
            return;
        }

        let align = match pap.props.justify {
            Some(1) => Align::Center,
            Some(2) => Align::Right,
            Some(3) | Some(4) => Align::Justify,
            _ => Align::Left,
        };
        let paragraph = Block::Paragraph {
            content,
            align,
            indent: 0.0,
        };

        if style.quote {
            blocks.push(Block::Quote {
                blocks: vec![paragraph],
            });
            return;
        }
        if style.code
            && let Block::Paragraph { content, .. } = &paragraph
        {
            blocks.push(Block::Code {
                language: None,
                text: inline_text(content),
            });
            return;
        }

        // 0xF801 marks a paragraph whose numbering is suppressed.
        let ilfo = pap.props.ilfo.unwrap_or(0);
        if ilfo != 0 && ilfo != 0xF801 {
            // Word records a list's depth in the style's name when the
            // paragraph carries no level of its own -- "List Bullet 2" is the
            // second level, and `ilvl` stays at zero. Read by `ilvl` alone a
            // two-level list came back flat, which is what the .docx of the
            // same document disagreed with.
            let stated = (pap.props.ilvl.unwrap_or(0) as usize).min(LEVELS - 1);
            let level = match stated {
                0 => (style.list_level.unwrap_or(0) as usize).min(LEVELS - 1),
                deeper => deeper,
            };
            let fallback = ListDef::unknown();
            let list = self.lists.by_ilfo.get(&ilfo).unwrap_or(&fallback).clone();
            // The marker and the count come from the level the file states,
            // which is the level within that list's own definition; the name
            // says only where the item sits among the lists around it. Asked
            // at the deeper level, a bulleted sub-list came back numbered.
            let ordered = list.ordered[stated];
            let start = self.next_number(&list, stated);

            let item = ListItem {
                blocks: vec![paragraph],
                checked: None,
            };
            crate::formats::append_list_item(blocks, item, level as u8, ordered, start);
            return;
        }

        blocks.push(paragraph);
    }

    /// Advance a list's counter and return the number this item takes.
    ///
    /// State is keyed by list identity rather than by the reference, so every
    /// override of the same list continues one sequence — which is what makes
    /// a list interrupted by a paragraph resume at the right number.
    /// The picture a character stands in for, registered once per offset.
    fn place_picture(&mut self, fc: u32, index: usize) -> Option<crate::doc::ImageRef> {
        let offset = self.char_props(fc, index).picture? as usize;
        let reference = |id: String| crate::doc::ImageRef {
            asset_id: id,
            ..crate::doc::ImageRef::default()
        };
        if let Some(id) = self.seen_pictures.get(&offset) {
            return Some(reference(id.clone()));
        }
        let (media, bytes) = picture_at(&self.data, offset)?;
        // The ids the document will hand out, in the order these are added.
        let id = format!("asset{}", self.assets.len() + 1);
        self.assets.push((media.to_string(), bytes));
        self.seen_pictures.insert(offset, id.clone());
        Some(reference(id))
    }

    fn next_number(&mut self, list: &ListDef, level: usize) -> u64 {
        let values = self.counters.entry(list.lsid).or_insert([0; LEVELS]);
        let value = if values[level] == 0 {
            list.start[level]
        } else {
            values[level].saturating_add(1)
        };
        values[level] = value;
        // Using a level restarts every deeper one.
        for deeper in values.iter_mut().skip(level + 1) {
            *deeper = 0;
        }
        value
    }

    fn push_paragraph(blocks: &mut Vec<Block>, content: Vec<Inline>) {
        if crate::doc::inlines_are_empty(&content) {
            return;
        }
        blocks.push(Block::Paragraph {
            content,
            align: Align::Left,
            indent: 0.0,
        });
    }

    /// Emit the accumulated table, if any.
    fn flush_table(
        &self,
        blocks: &mut Vec<Block>,
        rows: &mut Vec<Row>,
        row: &mut Vec<Cell>,
        cell_blocks: &mut Vec<Block>,
        header_rows: &mut usize,
        edges: &mut Vec<Vec<i32>>,
    ) {
        if !cell_blocks.is_empty() {
            row.push(Cell {
                blocks: std::mem::take(cell_blocks),
                ..Cell::default()
            });
        }
        if !row.is_empty() {
            rows.push(Row {
                cells: std::mem::take(row),
            });
        }
        if rows.is_empty() {
            edges.clear();
            return;
        }
        let mut rows = std::mem::take(rows);
        let edges = std::mem::take(edges);
        crate::formats::apply_column_spans(&mut rows, &edges);
        let rows = rows;
        let header = std::mem::take(header_rows).min(rows.len());
        blocks.push(Block::Table(Table {
            caption: None,
            header_rows: header,
            rows,
            column_widths: Vec::new(),
        }));
    }
}

/// A field being read: its instruction, and where its result began.
struct Field {
    instruction: String,
    /// Index into the paragraph's inlines where the result's text starts.
    start: usize,
    reading_result: bool,
}

/// Turn the inlines a field's result produced into a link, where it is one.
fn finish_field(content: &mut Vec<Inline>, field: Field) {
    let Some(href) = crate::formats::hyperlink_target(&field.instruction) else {
        return;
    };
    if field.start > content.len() {
        return;
    }
    let runs: Vec<Run> = content
        .drain(field.start..)
        .filter_map(|inline| match inline {
            Inline::Run(run) => Some(run),
            _ => None,
        })
        .collect();
    match runs.is_empty() {
        // A field with no text of its own: the target is all there is, and
        // showing it is better than showing nothing.
        true => content.push(Inline::Link {
            runs: vec![Run::plain(href.clone())],
            href,
        }),
        false => content.push(Inline::Link { href, runs }),
    }
}

/// Append a character, extending the previous run when the style is unchanged.
fn push_char(content: &mut Vec<Inline>, character: char, style: TextStyle) {
    if let Some(Inline::Run(run)) = content.last_mut()
        && run.style == style
    {
        run.text.push(character);
        return;
    }
    content.push(Inline::Run(Run::styled(character.to_string(), style)));
}

/// Convert Word's character properties into the shared model's.
fn to_text_style(props: CharProps) -> TextStyle {
    TextStyle {
        bold: props.bold,
        italic: props.italic,
        strikethrough: props.strike,
        underline: props.underline,
        superscript: props.superscript,
        subscript: props.subscript,
        hidden: props.hidden,
        color: props.color.map(|[red, green, blue]| {
            [
                f64::from(red) / 255.0,
                f64::from(green) / 255.0,
                f64::from(blue) / 255.0,
            ]
        }),
        size: props.half_points.map(|value| value as f64 / 2.0),
        ..TextStyle::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a `PLC`: positions, then one entry each.
    fn plc_bytes(positions: &[u32], entries: &[Vec<u8>]) -> Vec<u8> {
        let mut out: Vec<u8> = positions.iter().flat_map(|p| p.to_le_bytes()).collect();
        for entry in entries {
            out.extend_from_slice(entry);
        }
        out
    }

    /// A FIB naming one table at `offset`, for a table stream of `len` bytes.
    fn fib(offset: usize, len: usize) -> Vec<u8> {
        let mut word = vec![0u8; 0x400];
        word[offset..offset + 4].copy_from_slice(&0u32.to_le_bytes());
        word[offset + 4..offset + 8].copy_from_slice(&(len as u32).to_le_bytes());
        word
    }

    /// A picture's bytes are behind a `PICF`, in the drawing format.
    ///
    /// The layout was read out of the same document written by Word and by
    /// LibreOffice: both put the blip 61 bytes into the store entry -- its own
    /// 36-byte header, then an 8-byte record header, one 16-byte identifier
    /// and a tag. Without following it the .doc of a document with a figure in
    /// it was the one form of that document reporting no figure.
    #[test]
    fn follows_a_picture_header_to_the_bytes_behind_it() {
        // A record: version and instance, type, length, body.
        let record = |version: u16, kind: u16, body: &[u8]| {
            let mut out = version.to_le_bytes().to_vec();
            out.extend_from_slice(&kind.to_le_bytes());
            out.extend_from_slice(&(body.len() as u32).to_le_bytes());
            out.extend_from_slice(body);
            out
        };

        // One identifier and a tag, because the instance is even.
        let mut blip_body = vec![0u8; 17];
        blip_body.extend_from_slice(b"PNGBYTES");
        let blip = record(0x6E0, 0xF01E, &blip_body);

        let mut store = vec![0u8; 36]; // the entry's own header
        store.extend_from_slice(&blip);
        let art = record(0x000F, 0xF002, &record(0x0000, 0xF007, &store));

        // The `PICF`: total length, then the length of the header itself.
        const HEADER: usize = 68;
        let mut data = vec![0u8; 8]; // the picture does not start at zero
        let offset = data.len();
        let total = HEADER + art.len();
        data.extend_from_slice(&(total as u32).to_le_bytes());
        data.extend_from_slice(&(HEADER as u16).to_le_bytes());
        data.resize(offset + HEADER, 0);
        data.extend_from_slice(&art);

        assert_eq!(
            picture_at(&data, offset),
            Some(("image/png", b"PNGBYTES".to_vec()))
        );
    }

    /// A header claiming more than the stream holds reads as no picture.
    #[test]
    fn a_picture_header_running_past_the_stream_is_not_read() {
        let mut data = 4096u32.to_le_bytes().to_vec();
        data.extend_from_slice(&68u16.to_le_bytes());
        data.resize(80, 0);
        assert_eq!(picture_at(&data, 0), None);
    }

    /// A text box says where it belongs in two tables, not one.
    ///
    /// The shapes are listed by their position in the main text, and the
    /// stories by the shape each one fills, so the two are joined on the shape
    /// id: the first table gives the anchor, the second the text. Without
    /// them a box's content could only be appended after the body, which is
    /// where this reader used to leave it while the .docx, .odt and .rtf of
    /// the same document all placed it beside its paragraph.
    #[test]
    fn joins_a_text_box_to_its_anchor_through_the_shape_id() {
        let mut shape = vec![0u8; 26];
        shape[..4].copy_from_slice(&2050u32.to_le_bytes());
        let shapes = plc_bytes(&[42, 353], &[shape]);

        let entry = |lid: u32| {
            let mut bytes = vec![0u8; 22];
            bytes[14..18].copy_from_slice(&lid.to_le_bytes());
            bytes
        };
        // The second story is the reserved entry, naming no shape.
        let stories = plc_bytes(&[0, 19, 353], &[entry(2050), entry(0)]);

        assert_eq!(
            shape_anchors(&fib(0x01DA, shapes.len()), &shapes),
            vec![(2050, 42)]
        );
        assert_eq!(
            textbox_stories(&fib(0x025A, stories.len()), &stories),
            vec![(0, 19, 2050), (19, 353, 0)]
        );
    }

    /// A table whose length does not divide into whole entries is not one.
    #[test]
    fn a_plc_of_the_wrong_shape_is_read_as_empty() {
        let stories = vec![0u8; 37];
        assert!(textbox_stories(&fib(0x025A, stories.len()), &stories).is_empty());
    }
}
