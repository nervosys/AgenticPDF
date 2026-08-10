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
    Align, Block, Cell, Inline, List, ListItem, Row, Run, Section, SemanticDoc, Table, TextStyle,
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

    // Character counts per document part; only the main text is converted.
    let ccp_text = u32_at(&word, 0x4C).unwrap_or(0) as usize;
    let total_cp = ccp_text
        + [0x50, 0x54, 0x58, 0x5C, 0x60]
            .iter()
            .map(|&offset| u32_at(&word, offset).unwrap_or(0) as usize)
            .sum::<usize>();

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

    let main_end = text.index_of_cp(ccp_text);
    let assembler = Assembler {
        text,
        chpx,
        papx,
        stylesheet,
        lists,
        prcs,
        piece_prcs: pieces.iter().map(|piece| piece.prm_prc).collect(),
        counters: HashMap::new(),
    };

    let blocks = assembler.build(0, main_end);
    Ok(SemanticDoc {
        sections: vec![Section {
            blocks,
            ..Section::default()
        }],
        ..SemanticDoc::default()
    })
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
                .chunks_exact(2)
                .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
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
    let cb_grpprl_chpx = *table.get(at + 25)? as usize;
    let cb_grpprl_papx = *table.get(at + 26)? as usize;

    // Then a length-prefixed number text.
    let after_grpprls = at + 28 + cb_grpprl_papx + cb_grpprl_chpx;
    let text_len = u16_at(table, after_grpprls)? as usize;
    let next = after_grpprls + 2 + text_len;

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
}

/// A paragraph's resolved properties.
struct EffectivePap {
    istd: u16,
    props: PapProps,
}

impl Assembler {
    /// Walk the character range, emitting blocks.
    fn build(mut self, lo: usize, hi: usize) -> Vec<Block> {
        let mut blocks: Vec<Block> = Vec::new();
        let mut content: Vec<Inline> = Vec::new();

        // Table state: cells accumulate into a row, rows into a table.
        let mut cell_blocks: Vec<Block> = Vec::new();
        let mut row: Vec<Cell> = Vec::new();
        let mut rows: Vec<Row> = Vec::new();
        let mut header_rows = 0usize;

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
                        );
                        self.emit_paragraph(&pap, inlines, &mut blocks);
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
                // Field markers, picture placeholders and other control
                // characters carry no text of their own.
                character if character.is_control() => {}
                character => {
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
    fn char_style(&self, fc: u32, index: usize) -> TextStyle {
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
        to_text_style(props)
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
        if inline_text(&content).trim().is_empty() {
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
            blocks.push(Block::Quote(vec![paragraph]));
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
            let level = (pap.props.ilvl.unwrap_or(0) as usize).min(LEVELS - 1);
            let fallback = ListDef::unknown();
            let list = self.lists.by_ilfo.get(&ilfo).unwrap_or(&fallback).clone();
            let ordered = list.ordered[level];
            let start = self.next_number(&list, level);

            let item = ListItem {
                blocks: vec![paragraph],
                checked: None,
            };
            append_list_item(blocks, item, level, ordered, start);
            return;
        }

        blocks.push(paragraph);
    }

    /// Advance a list's counter and return the number this item takes.
    ///
    /// State is keyed by list identity rather than by the reference, so every
    /// override of the same list continues one sequence — which is what makes
    /// a list interrupted by a paragraph resume at the right number.
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
        if inline_text(&content).trim().is_empty() {
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
            return;
        }
        let rows = std::mem::take(rows);
        let header = std::mem::take(header_rows).min(rows.len());
        blocks.push(Block::Table(Table {
            caption: None,
            header_rows: header,
            rows,
            column_widths: Vec::new(),
        }));
    }
}

/// Append a list item, merging with the run before it and nesting by level.
fn append_list_item(
    blocks: &mut Vec<Block>,
    item: ListItem,
    level: usize,
    ordered: bool,
    start: u64,
) {
    if level > 0
        && let Some(Block::List(list)) = blocks.last_mut()
        && let Some(last) = list.items.last_mut()
    {
        append_list_item(&mut last.blocks, item, level - 1, ordered, start);
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
        start: if ordered { start } else { 1 },
        items: vec![item],
    }));
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
        hidden: props.hidden,
        size: props.half_points.map(|value| value as f64 / 2.0),
        ..TextStyle::default()
    }
}
