// SPDX-License-Identifier: AGPL-3.0-or-later
//! Excel 97-2003 binary (`.xls`), reading the BIFF8 record stream.
//!
//! The `Workbook` stream is a flat sequence of records — a 16-bit type, a
//! 16-bit length, then the payload — with no nesting and no index. Everything
//! is found by reading forward and remembering what has been seen.
//!
//! Three things make it awkward:
//!
//! - **Strings are shared.** Cell records mostly hold an index into a Shared
//!   String Table that appears once, near the start. A reader that prints the
//!   cell payload emits a spreadsheet full of integers.
//! - **Records have a maximum size.** A long string table spills into
//!   `CONTINUE` records, and a string may be split across the boundary — with
//!   its compression flag restated in the continuation, so the second half can
//!   have a different width from the first.
//! - **Numbers are compressed.** `RK` values pack a float into 32 bits with two
//!   flag bits: one selects a 100× scale, the other an integer encoding.
//!
//! This module is written from the format rather than ported: anydoc delegates
//! `.xls` entirely to the `calamine` crate, so there was nothing to adapt.

use crate::PdfError;
use crate::container::ole::{Ole2, decode_utf16le, u16_at, u32_at};
use crate::doc::{Block, Cell, Row, Section, SectionKind, SemanticDoc, Table};

// Record types, from the BIFF8 specification.
const BOF: u16 = 0x0809;
const EOF_RECORD: u16 = 0x000A;
const BOUNDSHEET: u16 = 0x0085;
const SST: u16 = 0x00FC;
const CONTINUE: u16 = 0x003C;
const LABELSST: u16 = 0x00FD;
const LABEL: u16 = 0x0204;
const RK: u16 = 0x027E;
const MULRK: u16 = 0x00BD;
const NUMBER: u16 = 0x0203;
const BOOLERR: u16 = 0x0205;
const FORMULA: u16 = 0x0006;
const STRING: u16 = 0x0207;
const RSTRING: u16 = 0x00D6;
/// Cell formatting: its number-format index says whether a number is a date.
const XF: u16 = 0x00E0;
/// A custom number-format string, for indices at 164 and above.
const FORMAT: u16 = 0x041E;
const FILEPASS: u16 = 0x002F;

/// Caps on the grid a single sheet may produce.
const MAX_COLUMNS: usize = 1_024;
const MAX_ROWS: usize = 65_536;

/// Parse an Excel 97-2003 workbook.
pub fn parse(data: &[u8]) -> Result<SemanticDoc, PdfError> {
    let mut ole = Ole2::open(data)?;
    // Excel 97+ calls it "Workbook"; Excel 5/95 called it "Book".
    let stream = ole
        .read_any(&["Workbook", "Book"])
        .ok_or_else(|| PdfError::MissingPart("Workbook".into()))?;

    let records = Records::new(&stream);
    let mut sheets: Vec<SheetRef> = Vec::new();
    let mut strings: Vec<String> = Vec::new();
    let mut formats = Formats::default();

    // First pass: the workbook globals, which hold the sheet directory and the
    // shared strings both needed before any cell can be read.
    for record in records.clone() {
        match record.kind {
            FILEPASS => return Err(PdfError::Encrypted),
            BOUNDSHEET => {
                if let Some(sheet) = parse_boundsheet(record.data) {
                    sheets.push(sheet);
                }
            }
            SST => strings = parse_sst(record.data, &record.continuations),
            // The format table: an `XF` names a number format by index, and a
            // `FORMAT` gives the code for the custom ones. Both are read here
            // because the globals precede every sheet and apply to all of them.
            XF => {
                if let Some(id) = u16_at(record.data, 2) {
                    formats.xf.push(id);
                }
            }
            FORMAT => {
                if let Some(id) = u16_at(record.data, 0)
                    && let Some(len) = u16_at(record.data, 2)
                    && let Some(&flags) = record.data.get(4)
                    && let Some((code, _)) =
                        read_string_body(&record.data[5..], len as usize, flags)
                {
                    formats.codes.insert(id, code);
                }
            }
            // The globals end at the first EOF; sheet substreams follow.
            EOF_RECORD => break,
            _ => {}
        }
    }

    let mut document = SemanticDoc::default();
    for sheet in &sheets {
        // A hidden sheet is content the author chose not to show.
        if sheet.hidden {
            continue;
        }
        let grid = read_sheet(&stream, sheet.offset, &strings, &formats);
        let blocks = if grid.is_empty() {
            Vec::new()
        } else {
            let rows: Vec<Row> = grid
                .into_iter()
                .map(|cells| Row {
                    cells: cells.into_iter().map(Cell::text).collect(),
                })
                .collect();
            vec![Block::Table(Table {
                header_rows: 1.min(rows.len()),
                rows,
                ..Table::default()
            })]
        };
        document.sections.push(Section {
            kind: SectionKind::Sheet,
            title: Some(sheet.name.clone()),
            blocks,
            ..Section::default()
        });
    }

    if document.sections.is_empty() {
        return Err(PdfError::MissingPart("xls contains no worksheets".into()));
    }
    Ok(document)
}

// ============================================================================
// Record stream
// ============================================================================

/// One record, with any `CONTINUE` payloads that follow it.
struct Record<'a> {
    kind: u16,
    data: &'a [u8],
    continuations: Vec<&'a [u8]>,
}

/// An iterator over the record stream that folds `CONTINUE` records into the
/// record they extend.
#[derive(Clone)]
struct Records<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Records<'a> {
    fn new(bytes: &'a [u8]) -> Records<'a> {
        Records { bytes, pos: 0 }
    }

    /// Read the header and payload at `pos`, without interpreting it.
    fn raw_at(&self, pos: usize) -> Option<(u16, &'a [u8], usize)> {
        let kind = u16_at(self.bytes, pos)?;
        let len = u16_at(self.bytes, pos + 2)? as usize;
        let data = self.bytes.get(pos + 4..)?.get(..len)?;
        Some((kind, data, pos + 4 + len))
    }
}

impl<'a> Iterator for Records<'a> {
    type Item = Record<'a>;

    fn next(&mut self) -> Option<Record<'a>> {
        let (kind, data, mut next) = self.raw_at(self.pos)?;

        // Gather the continuations now, so consumers never have to know that a
        // record was split.
        let mut continuations = Vec::new();
        while let Some((CONTINUE, extra, after)) = self.raw_at(next) {
            continuations.push(extra);
            next = after;
        }

        self.pos = next;
        Some(Record {
            kind,
            data,
            continuations,
        })
    }
}

// ============================================================================
// Workbook globals
// ============================================================================

struct SheetRef {
    name: String,
    /// Byte offset of the sheet's substream in the Workbook stream.
    offset: usize,
    hidden: bool,
}

fn parse_boundsheet(data: &[u8]) -> Option<SheetRef> {
    let offset = u32_at(data, 0)? as usize;
    let state = *data.get(4)?;
    // 0 = visible, 1 = hidden, 2 = very hidden.
    let hidden = state & 0x03 != 0;
    let name = parse_short_string(data.get(6..)?)?.0;
    Some(SheetRef {
        name,
        offset,
        hidden,
    })
}

/// Parse a BIFF8 string with a one-byte length prefix.
fn parse_short_string(data: &[u8]) -> Option<(String, usize)> {
    let len = *data.first()? as usize;
    let flags = *data.get(1)?;
    read_string_body(data.get(2..)?, len, flags).map(|(text, used)| (text, 2 + used))
}

/// Decode a string body of `len` characters.
///
/// The low bit of `flags` selects the width: clear means one byte per character
/// (the low half of Latin-1), set means UTF-16.
fn read_string_body(data: &[u8], len: usize, flags: u8) -> Option<(String, usize)> {
    if flags & 0x01 == 0 {
        let bytes = data.get(..len)?;
        // Compressed text is Latin-1's low half, one byte per character.
        Some((bytes.iter().map(|&b| b as char).collect(), len))
    } else {
        let bytes = data.get(..len.checked_mul(2)?)?;
        Some((decode_utf16le(bytes), len * 2))
    }
}

/// Parse the shared string table, following it across `CONTINUE` records.
///
/// A string may straddle a record boundary, and the continuation restates the
/// compression flag — so the two halves can differ in width. Splicing the
/// payloads together and parsing once would corrupt exactly those strings, so
/// this walks the segments explicitly.
fn parse_sst(first: &[u8], continuations: &[&[u8]]) -> Vec<String> {
    let Some(count) = u32_at(first, 4) else {
        return Vec::new();
    };
    let mut strings = Vec::with_capacity((count as usize).min(1 << 16));

    let mut segments: Vec<&[u8]> = Vec::with_capacity(1 + continuations.len());
    segments.push(&first[8.min(first.len())..]);
    segments.extend_from_slice(continuations);

    let mut segment = 0usize;
    let mut pos = 0usize;

    for _ in 0..count {
        // Move to a segment with room for a header.
        while segment < segments.len() && pos + 3 > segments[segment].len() {
            segment += 1;
            pos = 0;
        }
        if segment >= segments.len() {
            break;
        }
        let data = segments[segment];

        let Some(len) = u16_at(data, pos).map(|value| value as usize) else {
            break;
        };
        let flags = data[pos + 2];
        pos += 3;

        // Rich-text and Far East runs add trailing arrays that are not text.
        let rich_runs = if flags & 0x08 != 0 {
            let runs = u16_at(data, pos).unwrap_or(0) as usize;
            pos += 2;
            runs
        } else {
            0
        };
        let extended = if flags & 0x04 != 0 {
            let size = u32_at(data, pos).unwrap_or(0) as usize;
            pos += 4;
            size
        } else {
            0
        };

        // Read the characters, crossing segments when the string is split.
        let mut text = String::with_capacity(len);
        let mut remaining = len;
        let mut wide = flags & 0x01 != 0;
        loop {
            let data = segments[segment];
            let available = data.len().saturating_sub(pos);
            let per_char = if wide { 2 } else { 1 };
            let fits = (available / per_char).min(remaining);

            if fits > 0
                && let Some((part, used)) =
                    read_string_body(&data[pos..], fits, if wide { 1 } else { 0 })
            {
                text.push_str(&part);
                pos += used;
                remaining -= fits;
            }
            if remaining == 0 {
                break;
            }

            // The rest is in the next segment, which restates the width.
            segment += 1;
            if segment >= segments.len() {
                break;
            }
            pos = 0;
            let Some(&next_flags) = segments[segment].first() else {
                break;
            };
            wide = next_flags & 0x01 != 0;
            pos = 1;
        }

        // Skip the formatting runs and extended data that follow the text.
        pos += rich_runs * 4 + extended;
        strings.push(text);
    }
    strings
}

// ============================================================================
// Worksheets
// ============================================================================

/// Read one sheet substream into a dense grid.
/// Which cell formats show a date, indexed as the cell records index them.
///
/// A cell record carries its format index two bytes after the column; that
/// selects an `XF`, which names a number format, which decides whether the
/// stored number is a quantity or a moment. Without the chain a date reports
/// its serial -- `46095` for 2026-03-14 -- exactly as the OOXML reader did
/// before it learned to read `styles.xml`.
#[derive(Debug, Default)]
struct Formats {
    /// XF index -> number format id, in record order.
    xf: Vec<u16>,
    /// Custom format id -> its code.
    codes: std::collections::HashMap<u16, String>,
}

impl Formats {
    fn shows_a_date(&self, xf_index: u16) -> bool {
        let Some(&id) = self.xf.get(xf_index as usize) else {
            return false;
        };
        crate::formats::is_date_format(id as u32, self.codes.get(&id).map(String::as_str))
    }
}

fn read_sheet(
    stream: &[u8],
    offset: usize,
    strings: &[String],
    formats: &Formats,
) -> Vec<Vec<String>> {
    let mut grid: Vec<Vec<String>> = Vec::new();
    if offset >= stream.len() {
        return grid;
    }

    let mut records = Records::new(stream);
    records.pos = offset;
    let mut started = false;

    for record in records {
        match record.kind {
            BOF if started => break,
            BOF => started = true,
            EOF_RECORD if started => break,

            LABELSST => {
                if let (Some(row), Some(column), Some(index)) = (
                    u16_at(record.data, 0),
                    u16_at(record.data, 2),
                    u32_at(record.data, 6),
                ) {
                    let value = strings.get(index as usize).cloned().unwrap_or_default();
                    place(&mut grid, row as usize, column as usize, value);
                }
            }
            LABEL | RSTRING => {
                if let (Some(row), Some(column)) = (u16_at(record.data, 0), u16_at(record.data, 2))
                    && let Some(len) = u16_at(record.data, 6).map(|v| v as usize)
                    && let Some(flags) = record.data.get(8)
                    && let Some((text, _)) = read_string_body(&record.data[9..], len, *flags)
                {
                    place(&mut grid, row as usize, column as usize, text);
                }
            }
            RK => {
                if let (Some(row), Some(column), Some(raw)) = (
                    u16_at(record.data, 0),
                    u16_at(record.data, 2),
                    u32_at(record.data, 6),
                ) {
                    let xf = u16_at(record.data, 4).unwrap_or(0);
                    place(
                        &mut grid,
                        row as usize,
                        column as usize,
                        render_number(decode_rk(raw), xf, formats),
                    );
                }
            }
            // A run of RK values sharing one row, ending with the last column.
            MULRK => {
                let (Some(row), Some(first)) = (u16_at(record.data, 0), u16_at(record.data, 2))
                else {
                    continue;
                };
                let mut at = 4usize;
                let mut column = first as usize;
                while at + 6 <= record.data.len().saturating_sub(2) {
                    if let Some(raw) = u32_at(record.data, at + 2) {
                        // Each entry in the run is its own format index
                        // and value: two bytes then four.
                        let xf = u16_at(record.data, at).unwrap_or(0);
                        place(
                            &mut grid,
                            row as usize,
                            column,
                            render_number(decode_rk(raw), xf, formats),
                        );
                    }
                    at += 6;
                    column += 1;
                }
            }
            NUMBER => {
                if let (Some(row), Some(column)) = (u16_at(record.data, 0), u16_at(record.data, 2))
                    && let Some(bytes) = record.data.get(6..14)
                {
                    let value = f64::from_le_bytes(bytes.try_into().unwrap_or([0; 8]));
                    let xf = u16_at(record.data, 4).unwrap_or(0);
                    place(
                        &mut grid,
                        row as usize,
                        column as usize,
                        render_number(value, xf, formats),
                    );
                }
            }
            BOOLERR => {
                if let (Some(row), Some(column), Some(&value), Some(&is_error)) = (
                    u16_at(record.data, 0),
                    u16_at(record.data, 2),
                    record.data.get(6),
                    record.data.get(7),
                ) {
                    let text = if is_error != 0 {
                        error_text(value).to_string()
                    } else if value != 0 {
                        "TRUE".to_string()
                    } else {
                        "FALSE".to_string()
                    };
                    place(&mut grid, row as usize, column as usize, text);
                }
            }
            // A formula's cached result: a number, or a marker saying the
            // result is the string in the record that follows.
            FORMULA => {
                let (Some(row), Some(column)) = (u16_at(record.data, 0), u16_at(record.data, 2))
                else {
                    continue;
                };
                let Some(bytes) = record.data.get(6..14) else {
                    continue;
                };
                // A leading 0xFFFF in the high word marks a non-numeric result.
                if u16_at(record.data, 12) == Some(0xFFFF) {
                    match bytes[0] {
                        1 => place(
                            &mut grid,
                            row as usize,
                            column as usize,
                            if bytes[2] != 0 { "TRUE" } else { "FALSE" }.to_string(),
                        ),
                        2 => place(
                            &mut grid,
                            row as usize,
                            column as usize,
                            error_text(bytes[2]).to_string(),
                        ),
                        // 0 means the value is in the next STRING record; 3 is
                        // an empty string.
                        _ => {}
                    }
                    continue;
                }
                let value = f64::from_le_bytes(bytes.try_into().unwrap_or([0; 8]));
                place(
                    &mut grid,
                    row as usize,
                    column as usize,
                    format_number(value),
                );
            }
            // The string result of the formula immediately before it.
            STRING => {
                if let Some(len) = u16_at(record.data, 0).map(|v| v as usize)
                    && let Some(&flags) = record.data.get(2)
                    && let Some((text, _)) = read_string_body(&record.data[3..], len, flags)
                    && let Some((row, column)) = last_position(&grid)
                {
                    place(&mut grid, row, column, text);
                }
            }
            _ => {}
        }
    }

    square(&mut grid);
    grid
}

/// The most recently written position, for attaching a formula's string result.
fn last_position(grid: &[Vec<String>]) -> Option<(usize, usize)> {
    let row = grid.len().checked_sub(1)?;
    let column = grid[row].len().checked_sub(1)?;
    Some((row, column))
}

/// Write a value at its true coordinates, growing the grid as needed.
fn place(grid: &mut Vec<Vec<String>>, row: usize, column: usize, value: String) {
    if row >= MAX_ROWS || column >= MAX_COLUMNS || value.is_empty() {
        return;
    }
    if grid.len() <= row {
        grid.resize(row + 1, Vec::new());
    }
    if grid[row].len() <= column {
        grid[row].resize(column + 1, String::new());
    }
    grid[row][column] = value;
}

/// Trim trailing empty rows and make every row the same width.
fn square(grid: &mut Vec<Vec<String>>) {
    while grid
        .last()
        .is_some_and(|row| row.iter().all(String::is_empty))
    {
        grid.pop();
    }
    let width = grid.iter().map(Vec::len).max().unwrap_or(0);
    for row in grid.iter_mut() {
        row.resize(width, String::new());
    }
}

/// Decode an RK value.
///
/// Two flag bits ride in the low end: bit 0 selects an integer encoding rather
/// than the top 30 bits of an IEEE double, and bit 1 a division by 100.
fn decode_rk(raw: u32) -> f64 {
    let integer = raw & 0x02 != 0;
    let scaled = raw & 0x01 != 0;
    let value = if integer {
        ((raw as i32) >> 2) as f64
    } else {
        f64::from_bits(((raw & 0xFFFF_FFFC) as u64) << 32)
    };
    if scaled { value / 100.0 } else { value }
}

use crate::formats::format_number;

/// A number as its cell shows it: a date where the format says so, otherwise
/// the number itself.
fn render_number(value: f64, xf: u16, formats: &Formats) -> String {
    match formats.shows_a_date(xf) {
        true => crate::formats::format_date_serial(value).unwrap_or_else(|| format_number(value)),
        false => format_number(value),
    }
}

/// The displayed form of a BIFF error code.
fn error_text(code: u8) -> &'static str {
    match code {
        0x00 => "#NULL!",
        0x07 => "#DIV/0!",
        0x0F => "#VALUE!",
        0x17 => "#REF!",
        0x1D => "#NAME?",
        0x24 => "#NUM!",
        0x2A => "#N/A",
        _ => "#ERR",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a BIFF record.
    fn record(kind: u16, payload: &[u8]) -> Vec<u8> {
        let mut out = kind.to_le_bytes().to_vec();
        out.extend_from_slice(&(payload.len() as u16).to_le_bytes());
        out.extend_from_slice(payload);
        out
    }

    #[test]
    fn decodes_all_four_rk_encodings() {
        // An integer, shifted left two with the integer flag set.
        assert_eq!(decode_rk((42i32 << 2) as u32 | 0x02), 42.0);
        // The same, scaled by 100.
        assert_eq!(decode_rk((4250i32 << 2) as u32 | 0x03), 42.5);
        // A double, keeping only its top 30 bits.
        let bits = (1.5f64.to_bits() >> 32) as u32 & 0xFFFF_FFFC;
        assert_eq!(decode_rk(bits), 1.5);
        assert_eq!(decode_rk(bits | 0x01), 0.015);
    }

    #[test]
    fn formats_numbers_without_spurious_decimals() {
        assert_eq!(format_number(42.0), "42");
        assert_eq!(format_number(42.5), "42.5");
        assert_eq!(format_number(3.50), "3.5");
        assert_eq!(format_number(f64::NAN), "");
    }

    #[test]
    fn reads_compressed_and_wide_strings() {
        let (text, used) = read_string_body(b"hello", 5, 0).unwrap();
        assert_eq!((text.as_str(), used), ("hello", 5));

        let wide: Vec<u8> = "hi".encode_utf16().flat_map(u16::to_le_bytes).collect();
        let (text, used) = read_string_body(&wide, 2, 1).unwrap();
        assert_eq!((text.as_str(), used), ("hi", 4));
    }

    #[test]
    fn the_record_iterator_folds_continuations_into_their_owner() {
        let mut stream = record(SST, b"aaaa");
        stream.extend(record(CONTINUE, b"bbbb"));
        stream.extend(record(CONTINUE, b"cccc"));
        stream.extend(record(EOF_RECORD, b""));

        let records: Vec<_> = Records::new(&stream).collect();
        assert_eq!(records.len(), 2, "continuations are not separate records");
        assert_eq!(records[0].kind, SST);
        assert_eq!(records[0].continuations.len(), 2);
        assert_eq!(records[1].kind, EOF_RECORD);
    }

    #[test]
    fn a_truncated_record_ends_the_stream_rather_than_panicking() {
        // A header claiming more payload than exists.
        let stream = vec![0x09, 0x08, 0xFF, 0xFF, 0x01];
        assert_eq!(Records::new(&stream).count(), 0);
    }

    #[test]
    fn parses_a_shared_string_table() {
        let mut payload = 2u32.to_le_bytes().to_vec(); // total
        payload.extend_from_slice(&2u32.to_le_bytes()); // unique
        for text in ["Region", "Revenue"] {
            payload.extend_from_slice(&(text.len() as u16).to_le_bytes());
            payload.push(0); // compressed
            payload.extend_from_slice(text.as_bytes());
        }
        assert_eq!(parse_sst(&payload, &[]), vec!["Region", "Revenue"]);
    }

    #[test]
    fn a_shared_string_split_across_a_continue_is_rejoined() {
        // "abcdef" with the tail in a continuation that restates the width.
        let mut first = 1u32.to_le_bytes().to_vec();
        first.extend_from_slice(&1u32.to_le_bytes());
        first.extend_from_slice(&6u16.to_le_bytes());
        first.push(0);
        first.extend_from_slice(b"abc");

        let mut continuation = vec![0u8]; // compressed again
        continuation.extend_from_slice(b"def");

        assert_eq!(parse_sst(&first, &[&continuation]), vec!["abcdef"]);
    }

    #[test]
    fn a_split_string_may_change_width_at_the_boundary() {
        // The first half is compressed; the continuation is UTF-16. Splicing
        // the payloads and parsing once would garble it.
        let mut first = 1u32.to_le_bytes().to_vec();
        first.extend_from_slice(&1u32.to_le_bytes());
        first.extend_from_slice(&4u16.to_le_bytes());
        first.push(0);
        first.extend_from_slice(b"ab");

        let mut continuation = vec![1u8]; // now wide
        continuation.extend_from_slice(&[0x63, 0x00, 0x64, 0x00]); // "cd"

        assert_eq!(parse_sst(&first, &[&continuation]), vec!["abcd"]);
    }

    #[test]
    fn reads_a_boundsheet_and_its_hidden_flag() {
        let mut payload = 0x1234u32.to_le_bytes().to_vec();
        payload.push(0x00); // visible
        payload.push(0x00); // worksheet
        payload.push(5);
        payload.push(0);
        payload.extend_from_slice(b"Sales");

        let sheet = parse_boundsheet(&payload).expect("parsed");
        assert_eq!(sheet.name, "Sales");
        assert_eq!(sheet.offset, 0x1234);
        assert!(!sheet.hidden);

        payload[4] = 0x01;
        assert!(parse_boundsheet(&payload).unwrap().hidden);
    }

    #[test]
    fn places_cells_at_their_true_coordinates() {
        let mut grid = Vec::new();
        place(&mut grid, 4, 2, "far".into());
        place(&mut grid, 0, 0, "near".into());
        square(&mut grid);

        assert_eq!(grid.len(), 5, "the gap rows are kept");
        assert_eq!(grid[0][0], "near");
        assert_eq!(grid[4][2], "far");
        assert!(grid.iter().all(|row| row.len() == 3), "grid is rectangular");
    }

    /// Wrap a workbook stream in a real compound file, so tests drive `parse`
    /// itself rather than a stand-in for it.
    fn workbook_file(stream: &[u8]) -> Vec<u8> {
        use std::io::Write;

        let mut file = cfb::CompoundFile::create(std::io::Cursor::new(Vec::new()))
            .expect("create compound file");
        file.create_stream("Workbook")
            .expect("create stream")
            .write_all(stream)
            .expect("write stream");
        file.flush().expect("flush");
        file.into_inner().into_inner()
    }

    #[test]
    fn an_encrypted_workbook_is_reported_as_such() {
        let mut stream = record(BOF, &[0; 16]);
        stream.extend(record(FILEPASS, &[0; 4]));
        stream.extend(record(EOF_RECORD, b""));

        let error = parse(&workbook_file(&stream)).unwrap_err();
        assert!(matches!(error, PdfError::Encrypted), "got {error:?}");
    }

    #[test]
    fn a_workbook_without_a_filepass_record_is_not_reported_as_encrypted() {
        // The negative half: without this, a reader that returned `Encrypted`
        // unconditionally would pass the test above.
        let mut stream = record(BOF, &[0; 16]);
        stream.extend(record(EOF_RECORD, b""));

        let result = parse(&workbook_file(&stream));
        assert!(
            !matches!(result, Err(PdfError::Encrypted)),
            "got {result:?}"
        );
    }

    #[test]
    fn maps_error_codes_to_their_displayed_form() {
        assert_eq!(error_text(0x07), "#DIV/0!");
        assert_eq!(error_text(0x2A), "#N/A");
        assert_eq!(error_text(0xFF), "#ERR");
    }
}
