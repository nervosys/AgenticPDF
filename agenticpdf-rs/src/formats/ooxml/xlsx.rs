// SPDX-License-Identifier: AGPL-3.0-or-later
//! SpreadsheetML (`.xlsx`) reader.
//!
//! Each worksheet becomes one [`Section`] holding a single table, so sheet
//! boundaries survive into Markdown, chunk provenance and the structure tree.
//!
//! A worksheet is not a dense grid. Rows and cells are *sparse*: `<row r="7">`
//! may follow `<row r="2">`, and within a row `<c r="D3">` may follow
//! `<c r="A3">`. Reading cells in document order and appending them produces a
//! table whose columns do not line up with anything. This reader parses the
//! `r` reference on each cell and places it at its true coordinates, padding
//! the gaps — which is what makes a column of figures stay a column.
//!
//! Cell *values* are equally indirect. Most text is not in the worksheet at
//! all: `<c t="s"><v>4</v></c>` means "the string at index 4 of
//! `xl/sharedStrings.xml`", and a reader that prints the `<v>` verbatim emits
//! a spreadsheet full of integers.

use crate::PdfError;
use crate::container::zip::ZipArchive;
use crate::doc::{Block, Cell, Row, Section, SemanticDoc, Table};
use crate::formats::ooxml::{Package, attr_i64, resolve_path};
use crate::xml::{Element, Event, Reader};

/// Cap on cells read per sheet, bounding a hostile or corrupt dimension.
const MAX_CELLS_PER_SHEET: usize = 2_000_000;
/// Cap on columns, so a stray reference like `XFD1` cannot allocate a huge row.
const MAX_COLUMNS: usize = 16_384;

/// Parse a SpreadsheetML package.
pub fn parse(archive: &ZipArchive, package: &Package) -> Result<SemanticDoc, PdfError> {
    let workbook = archive.read(&package.main_part)?;
    let base = package.base();
    let shared = SharedStrings::read(archive, &base);

    let mut document = SemanticDoc::default();
    for sheet in read_sheet_list(&workbook) {
        // Sheets are located through relationships, not by convention: the
        // order in `workbook.xml` is the tab order, and the paths need not be
        // `sheet1.xml`, `sheet2.xml` at all.
        let Some(path) = package.main_rels.resolve(&base, &sheet.relationship) else {
            continue;
        };
        let Some(bytes) = archive.read_optional(&path) else {
            continue;
        };

        let grid = read_sheet(&bytes, &shared);
        if grid.is_empty() {
            // An empty sheet still exists; recording it keeps sheet indices
            // aligned with the workbook a user is looking at.
            document.sections.push(Section {
                kind: crate::doc::SectionKind::Sheet,
                title: Some(sheet.name),
                ..Section::default()
            });
            continue;
        }

        let rows: Vec<Row> = grid
            .into_iter()
            .map(|cells| Row {
                cells: cells.into_iter().map(Cell::text).collect(),
            })
            .collect();

        document.sections.push(Section {
            kind: crate::doc::SectionKind::Sheet,
            title: Some(sheet.name),
            blocks: vec![Block::Table(Table {
                // Spreadsheets carry no header marker; the first row is one by
                // overwhelming convention, and a GFM table requires one.
                header_rows: 1.min(rows.len()),
                rows,
                ..Table::default()
            })],
            ..Section::default()
        });
    }

    if document.sections.is_empty() {
        return Err(PdfError::MissingPart("xlsx contains no worksheets".into()));
    }
    Ok(document)
}

/// A worksheet as named by the workbook.
struct SheetRef {
    name: String,
    relationship: String,
}

/// Read `<sheets>` from `xl/workbook.xml`, in tab order.
fn read_sheet_list(xml: &[u8]) -> Vec<SheetRef> {
    let mut sheets = Vec::new();
    let mut reader = Reader::new(xml);
    while let Some(event) = reader.read_event() {
        let Event::Start(element) = event else {
            continue;
        };
        if element.local != "sheet" {
            continue;
        }
        // Hidden sheets are deliberately not shown to the reader of a workbook;
        // extracting them anyway would surface content the author concealed.
        if matches!(
            element.attr_local("state"),
            Some("hidden") | Some("veryHidden")
        ) {
            continue;
        }
        let Some(relationship) = element.attr_local("id") else {
            continue;
        };
        sheets.push(SheetRef {
            name: element
                .attr_local("name")
                .unwrap_or("Sheet")
                .trim()
                .to_string(),
            relationship: relationship.to_string(),
        });
    }
    sheets
}

/// Read a worksheet into a dense, rectangular grid of strings.
fn read_sheet(xml: &[u8], shared: &SharedStrings) -> Vec<Vec<String>> {
    let mut grid: Vec<Vec<String>> = Vec::new();
    let mut reader = Reader::new(xml);
    let mut cells_read = 0usize;
    let mut row_index = 0usize;

    while let Some(event) = reader.read_event() {
        let Event::Start(element) = event else {
            continue;
        };
        match element.local.as_str() {
            "row" => {
                // `r` is 1-based; a missing one means "the next row".
                row_index = attr_i64(&element, "r")
                    .map(|r| (r.max(1) as usize) - 1)
                    .unwrap_or(grid.len());
            }
            "c" => {
                if cells_read >= MAX_CELLS_PER_SHEET {
                    break;
                }
                cells_read += 1;

                let (column, row) = cell_position(&element, row_index, &grid);
                if column >= MAX_COLUMNS {
                    continue;
                }
                let value = read_cell_value(&mut reader, &element, shared);
                if value.is_empty() {
                    continue;
                }

                if grid.len() <= row {
                    grid.resize(row + 1, Vec::new());
                }
                if grid[row].len() <= column {
                    grid[row].resize(column + 1, String::new());
                }
                grid[row][column] = value;
            }
            _ => {}
        }
    }

    // Trim wholly empty trailing rows, then square the grid so every row has
    // the same width — a GFM table has to be rectangular.
    while grid
        .last()
        .is_some_and(|row| row.iter().all(|c| c.is_empty()))
    {
        grid.pop();
    }
    let width = grid.iter().map(Vec::len).max().unwrap_or(0);
    for row in &mut grid {
        row.resize(width, String::new());
    }
    grid
}

/// Resolve a cell's `(column, row)` from its `r` reference.
fn cell_position(element: &Element, row_index: usize, grid: &[Vec<String>]) -> (usize, usize) {
    match element.attr_local("r").and_then(parse_reference) {
        Some((column, row)) => (column, row),
        // Without a reference, the cell follows the previous one in its row.
        None => (grid.get(row_index).map_or(0, Vec::len), row_index),
    }
}

/// Parse an A1-style reference into zero-based `(column, row)`.
fn parse_reference(reference: &str) -> Option<(usize, usize)> {
    let letters: String = reference
        .chars()
        .take_while(|c| c.is_ascii_alphabetic())
        .collect();
    let digits = &reference[letters.len()..];
    if letters.is_empty() || digits.is_empty() {
        return None;
    }

    // Column letters are bijective base-26: A=1, Z=26, AA=27.
    let mut column = 0usize;
    for ch in letters.chars() {
        let value = (ch.to_ascii_uppercase() as u8).checked_sub(b'A')? as usize + 1;
        column = column.checked_mul(26)?.checked_add(value)?;
    }
    let row: usize = digits.parse().ok()?;
    Some((column.checked_sub(1)?, row.checked_sub(1)?))
}

/// Read one `<c>`'s value, resolving its type.
fn read_cell_value(reader: &mut Reader, start: &Element, shared: &SharedStrings) -> String {
    let cell_type = start.attr_local("t").unwrap_or("n").to_string();
    let mut value = String::new();
    let mut inline = String::new();

    while let Some(event) = reader.read_event() {
        match event {
            Event::End(name) if name == start.qname => break,
            Event::Start(element) => match element.local.as_str() {
                // `<v>` is the stored value: a number, a boolean, an error, or
                // an index into the shared string table.
                "v" => value = crate::xml::text_of(reader, &element.qname),
                // `<is>` holds a string stored inline rather than shared.
                "is" | "t" => {
                    let text = crate::xml::text_of(reader, &element.qname);
                    if !text.is_empty() {
                        inline.push_str(&text);
                    }
                }
                // `<f>` is the formula source; the cached result in `<v>` is
                // what a reader wants, so the formula itself is skipped.
                "f" => {
                    let _ = crate::xml::text_of(reader, &element.qname);
                }
                _ => {}
            },
            _ => {}
        }
    }

    match cell_type.as_str() {
        "s" => value
            .trim()
            .parse::<usize>()
            .ok()
            .and_then(|index| shared.get(index))
            .unwrap_or_default(),
        "inlineStr" => inline.trim().to_string(),
        "b" => match value.trim() {
            "1" => "TRUE".to_string(),
            "0" => "FALSE".to_string(),
            other => other.to_string(),
        },
        // A number, which is also the default when `t` is absent. Excel
        // writes seventeen significant digits so the double round-trips, so
        // the stored text is not the text the cell shows.
        "n" => match inline.trim().is_empty() {
            false => crate::formats::format_number_text(inline.trim()),
            true => crate::formats::format_number_text(value.trim()),
        },
        // "str" is a formula's string result; "e" is an error code such as
        // #DIV/0!. Both are already text, and neither is a number to reformat.
        _ => {
            if !inline.trim().is_empty() {
                inline.trim().to_string()
            } else {
                value.trim().to_string()
            }
        }
    }
}

/// The workbook's shared string table.
#[derive(Debug, Default)]
struct SharedStrings {
    entries: Vec<String>,
}

impl SharedStrings {
    fn read(archive: &ZipArchive, base: &str) -> SharedStrings {
        let Some(bytes) = archive.read_optional(&resolve_path(base, "sharedStrings.xml")) else {
            return SharedStrings::default();
        };

        let mut entries = Vec::new();
        let mut reader = Reader::new(&bytes);
        while let Some(event) = reader.read_event() {
            let Event::Start(element) = event else {
                continue;
            };
            if element.local != "si" {
                continue;
            }
            // An `<si>` may be a single `<t>` or a sequence of `<r><t>` runs
            // with different formatting; the text is the concatenation either
            // way.
            let mut text = String::new();
            let mut depth = 1usize;
            while let Some(event) = reader.read_event() {
                match event {
                    Event::End(name) if name == element.qname => {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    Event::Start(inner) if inner.qname == element.qname => depth += 1,
                    Event::Start(inner) if inner.local == "t" => {
                        text.push_str(&crate::xml::text_of(&mut reader, &inner.qname));
                    }
                    // `<rPh>` holds phonetic guides for CJK text, which are an
                    // annotation rather than content.
                    Event::Start(inner) if inner.local == "rPh" => {
                        let _ = crate::xml::text_of(&mut reader, &inner.qname);
                    }
                    _ => {}
                }
            }
            entries.push(text);
        }
        SharedStrings { entries }
    }

    fn get(&self, index: usize) -> Option<String> {
        self.entries.get(index).cloned()
    }
}
