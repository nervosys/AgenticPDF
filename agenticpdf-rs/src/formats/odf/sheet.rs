// SPDX-License-Identifier: AGPL-3.0-or-later
//! OpenDocument Spreadsheet (`.ods`) reader.
//!
//! ODS stores a grid where SpreadsheetML stores a sparse map, and it compresses
//! that grid with repeat counts: `table:number-columns-repeated` and
//! `table:number-rows-repeated`. A row of one value followed by empty cells to
//! the sheet's edge is written as two cells, the second repeated 16,383 times.
//!
//! That encoding is the whole difficulty. Expanding it naively turns a
//! three-cell sheet into a million-cell grid — LibreOffice routinely writes a
//! trailing repeat covering the entire remaining width and height — so the
//! expansion has to be *lazy*: repeats of empty cells are recorded as position
//! advances rather than materialised, and only cells that actually carry a
//! value are stored.

use crate::doc::{Block, Cell, Row, Section, SectionKind, SemanticDoc, Table};
use crate::xml::{Element, Event, Reader, ns};


/// Largest repeat count honoured for a cell or row carrying content.
const MAX_REPEAT: usize = 4_096;
/// Cap on the grid a single sheet may expand to.
const MAX_COLUMNS: usize = 1_024;
const MAX_ROWS: usize = 65_536;
/// Total cells one sheet may materialise.
///
/// The per-axis caps alone are not enough: a *non-empty* cell repeated across
/// the full width, on a row repeated down the full height, multiplies out to
/// millions of cells and tens of megabytes of text. This bounds the product,
/// not just the factors.
const MAX_CELLS: usize = 250_000;

/// Read `<office:spreadsheet>`: one section per sheet.
pub fn read(content: &[u8], document: &mut SemanticDoc) {
    let mut reader = Reader::new(content);

    while let Some(event) = reader.read_event() {
        let Event::Start(element) = event else {
            continue;
        };
        if !element.is(ns::ODF_TABLE, "table") {
            continue;
        }

        let name = element
            .attr_local("name")
            .filter(|value| !value.trim().is_empty())
            .unwrap_or("Sheet")
            .to_string();
        let grid = read_sheet(&mut reader, &element);

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
                // As with every spreadsheet format, nothing marks a header row;
                // the first is one by convention and GFM requires one.
                header_rows: 1.min(rows.len()),
                rows,
                ..Table::default()
            })]
        };

        document.sections.push(Section {
            kind: SectionKind::Sheet,
            title: Some(name),
            blocks,
            ..Section::default()
        });
    }
}

/// Read one `<table:table>` into a dense, rectangular grid.
fn read_sheet(reader: &mut Reader, start: &Element) -> Vec<Vec<String>> {
    let mut grid: Vec<Vec<String>> = Vec::new();
    let mut nesting = 1usize;
    // Empty rows seen since the last row with content. They are only worth
    // keeping if something follows: an interior gap is part of the sheet's
    // shape, while the trailing run — which spans the rest of the million-row
    // grid — is padding.
    let mut pending_empty = 0usize;
    // Cells materialised so far, bounding the product of the repeat counts.
    let mut cells_used = 0usize;

    while let Some(event) = reader.read_event() {
        match event {
            Event::End(name) if name == start.qname => {
                nesting -= 1;
                if nesting == 0 {
                    break;
                }
            }
            Event::Start(element) if element.qname == start.qname => nesting += 1,
            Event::Start(element) if element.is(ns::ODF_TABLE, "table-row") => {
                let cells = read_row(reader, &element);
                let repeat = repeat_count(&element, "number-rows-repeated");

                if cells.iter().all(String::is_empty) {
                    pending_empty = pending_empty.saturating_add(repeat);
                    continue;
                }

                for _ in 0..pending_empty.min(MAX_REPEAT) {
                    if grid.len() >= MAX_ROWS {
                        break;
                    }
                    grid.push(Vec::new());
                }
                pending_empty = 0;

                let width = cells.len().max(1);
                for _ in 0..repeat.min(MAX_REPEAT) {
                    if grid.len() >= MAX_ROWS || cells_used + width > MAX_CELLS {
                        break;
                    }
                    cells_used += width;
                    grid.push(cells.clone());
                }
            }
            _ => {}
        }
    }

    // Trim trailing empty rows and columns, then square the grid.
    while grid.last().is_some_and(|row| row.iter().all(String::is_empty)) {
        grid.pop();
    }
    let width = grid
        .iter()
        .map(|row| {
            row.iter()
                .rposition(|cell| !cell.is_empty())
                .map_or(0, |at| at + 1)
        })
        .max()
        .unwrap_or(0);
    for row in &mut grid {
        row.resize(width, String::new());
    }
    grid
}

/// Read one `<table:table-row>` into its cell values.
fn read_row(reader: &mut Reader, start: &Element) -> Vec<String> {
    let mut cells: Vec<String> = Vec::new();

    while let Some(event) = reader.read_event() {
        match event {
            Event::End(name) if name == start.qname => break,
            Event::Start(element)
                if element.is(ns::ODF_TABLE, "table-cell")
                    || element.is(ns::ODF_TABLE, "covered-table-cell") =>
            {
                let repeat = repeat_count(&element, "number-columns-repeated");
                let value = read_cell(reader, &element);

                if value.is_empty() {
                    // An empty run just advances the column cursor. Materialising
                    // it is what turns a small sheet into a huge one.
                    let target = (cells.len() + repeat).min(MAX_COLUMNS);
                    cells.resize(target, String::new());
                    continue;
                }
                for _ in 0..repeat.min(MAX_REPEAT) {
                    if cells.len() >= MAX_COLUMNS {
                        break;
                    }
                    cells.push(value.clone());
                }
            }
            _ => {}
        }
    }

    cells
}

/// Read a cell's displayed text.
///
/// ODF stores both a typed value and the formatted text the user sees. The
/// displayed text is what a reader wants — it carries the currency symbol, the
/// thousands separator and the date format — so the paragraphs win, with the
/// typed value as the fallback.
fn read_cell(reader: &mut Reader, start: &Element) -> String {
    let typed = typed_value(start);
    let mut text = String::new();
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
            Event::Start(element) if element.is(ns::ODF_TEXT, "p") => {
                let paragraph = crate::xml::text_of(reader, &element.qname);
                if !paragraph.trim().is_empty() {
                    if !text.is_empty() {
                        text.push(' ');
                    }
                    text.push_str(paragraph.trim());
                }
            }
            _ => {}
        }
    }


    if text.is_empty() { typed } else { text }
}

/// The cell's typed value, for cells with no displayed paragraph.
fn typed_value(element: &Element) -> String {
    match element.attr_local("value-type").unwrap_or("") {
        "float" | "percentage" | "currency" => element
            .attr_local("value")
            .map(trim_float)
            .unwrap_or_default(),
        "boolean" => match element.attr_local("boolean-value") {
            Some("true") => "TRUE".to_string(),
            Some("false") => "FALSE".to_string(),
            other => other.unwrap_or("").to_string(),
        },
        "date" => element.attr_local("date-value").unwrap_or("").to_string(),
        "time" => element.attr_local("time-value").unwrap_or("").to_string(),
        "string" => element.attr_local("string-value").unwrap_or("").to_string(),
        _ => String::new(),
    }
}

/// Render a stored float without the trailing zeros ODF writes.
fn trim_float(value: &str) -> String {
    let Ok(number) = value.parse::<f64>() else {
        return value.to_string();
    };
    if number.fract() == 0.0 && number.abs() < 1e15 {
        return format!("{}", number as i64);
    }
    let mut text = format!("{number}");
    if text.contains('.') {
        text = text.trim_end_matches('0').trim_end_matches('.').to_string();
    }
    text
}

fn repeat_count(element: &Element, attribute: &str) -> usize {
    element
        .attr_local(attribute)
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(1)
        .max(1)
}
