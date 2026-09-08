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

use std::collections::HashSet;

use crate::doc::{Block, Section, SectionKind, SemanticDoc, Table};
use crate::formats::SheetCell;
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
    // A sheet is hidden by its table style rather than by an attribute of its
    // own, so the styles have to be gathered on the way past. They are declared
    // before the body, so one forward pass is enough.
    let mut hidden_styles: HashSet<String> = HashSet::new();
    let mut style_name: Option<String> = None;

    while let Some(event) = reader.read_event() {
        let Event::Start(element) = event else {
            continue;
        };
        if element.is(ns::ODF_STYLE, "style") {
            style_name = match element.attr_local("family") {
                Some("table") => element.attr_local("name").map(str::to_string),
                _ => None,
            };
            continue;
        }
        if element.is(ns::ODF_STYLE, "table-properties") {
            if element.attr_local("display") == Some("false")
                && let Some(name) = style_name.take()
            {
                hidden_styles.insert(name);
            }
            continue;
        }
        if !element.is(ns::ODF_TABLE, "table") {
            continue;
        }

        // The .xlsx and .xls readers of the same workbook both drop a hidden
        // sheet; this one emitted its contents as an ordinary section, so a
        // payload two readers excluded came through the third.
        let sheet_hidden = element
            .attr_local("style-name")
            .is_some_and(|name| hidden_styles.contains(name));

        let name = element
            .attr_local("name")
            .filter(|value| !value.trim().is_empty())
            .unwrap_or("Sheet")
            .to_string();
        let grid = read_sheet(&mut reader, &element);
        if sheet_hidden {
            continue;
        }

        let blocks = if grid.is_empty() {
            Vec::new()
        } else {
            let rows = crate::formats::sheet_rows(grid);
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
fn read_sheet(reader: &mut Reader, start: &Element) -> Vec<Vec<SheetCell>> {
    let mut grid: Vec<Vec<SheetCell>> = Vec::new();
    let mut nesting = 1usize;
    // Columns are declared before the rows; the hidden ones are removed once
    // the grid is built.
    let mut column = 0usize;
    let mut hidden_columns: Vec<usize> = Vec::new();
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
            Event::Start(element) if element.is(ns::ODF_TABLE, "table-column") => {
                let repeat = repeat_count(&element, "number-columns-repeated");
                if is_collapsed(&element) {
                    for offset in 0..repeat.min(MAX_REPEAT) {
                        hidden_columns.push(column.saturating_add(offset));
                    }
                }
                column = column.saturating_add(repeat);
            }
            Event::Start(element) if element.is(ns::ODF_TABLE, "table-row") => {
                let cells = read_row(reader, &element);
                let repeat = repeat_count(&element, "number-rows-repeated");

                // A hidden row is content the author chose not to show, as a
                // hidden sheet is.
                if is_collapsed(&element) {
                    continue;
                }

                if cells.iter().all(SheetCell::is_empty) {
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

    for row in &mut grid {
        for at in hidden_columns.iter().rev() {
            if *at < row.len() {
                row.remove(*at);
            }
        }
    }

    crate::formats::square_grid(&mut grid);
    grid
}

/// Whether a row or column states that it is not displayed.
///
/// `collapse` is hidden outright; `filter` is hidden by a filter, which is the
/// same thing as far as a reader is concerned.
fn is_collapsed(element: &Element) -> bool {
    matches!(
        element.attr_local("visibility"),
        Some("collapse") | Some("filter")
    )
}

/// Read one `<table:table-row>` into its cell values.
fn read_row(reader: &mut Reader, start: &Element) -> Vec<SheetCell> {
    let mut cells: Vec<SheetCell> = Vec::new();

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
                    cells.resize(target, SheetCell::default());
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

/// Read a cell's value.
///
/// ODF stores both a typed value and the formatted text the user sees, and
/// which one a reader wants depends on what the cell holds.
///
/// For a **number** it is the value. The displayed form is a rendering of it —
/// `1,234.50` for 1234.5, `8.5%` for 0.085 — and a thousands separator makes
/// the text harder for a consumer to parse than the number it stands for. The
/// two OOXML readers of the same workbook report the value, so preferring the
/// text here also made one document give two answers depending on which format
/// it had been saved as.
///
/// For **text** it is the paragraphs: there the displayed form *is* the value,
/// and `string-value` is often absent entirely.
///
/// Dates and times keep their typed value for the same reason as numbers, and
/// it is already ISO 8601 — which is what the OOXML readers now produce from a
/// serial.
fn read_cell(reader: &mut Reader, start: &Element) -> SheetCell {
    let typed = typed_value(start);
    let mut text = String::new();
    let mut href: Option<String> = None;
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
            // A comment on a cell is a remark about it, not its contents. Its
            // paragraphs sit inside the cell, so reading every paragraph read
            // the comment as part of the value -- where the .xlsx and .xls of
            // the same workbook leave it out.
            Event::Start(element) if element.is(ns::ODF_OFFICE, "annotation") => {
                let _ = crate::xml::text_of(reader, &element.qname);
            }
            // Read a paragraph at a time rather than taking its text
            // wholesale, because a cell's hyperlink is written inside it: the
            // text of a `<text:a>` is the cell's text, and its target is where
            // the cell points -- which the .xlsx of the same workbook states
            // beside the sheet and this reader was losing here.
            Event::Start(element) if element.is(ns::ODF_TEXT, "p") => {
                let (paragraph, target) = read_cell_paragraph(reader, &element.qname);
                href = href.or(target);
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

    // A machine-readable value wins where the cell has one; otherwise the
    // displayed text, which for a string cell is the whole of it.
    //
    // Except for an error. ODF has no error type: a cell holding `#DIV/0!` is
    // written as a float of zero with the error only in the displayed text, so
    // preferring the value turns a failed formula into a plausible number --
    // which is worse than either answer alone.
    let machine_readable = matches!(
        start.attr_local("value-type").unwrap_or(""),
        "float" | "percentage" | "currency" | "date" | "time" | "boolean"
    ) && !is_error_code(&text);
    let value = match machine_readable && !typed.is_empty() {
        true => typed,
        false => match text.is_empty() {
            true => typed,
            false => text,
        },
    };
    SheetCell { text: value, href }
}

/// A cell's paragraph: its text, and the target of the first link in it.
///
/// A cell holds one link at most in practice, and a table cell has one place
/// to put it, so the first is the one that counts.
fn read_cell_paragraph(reader: &mut Reader, closer: &str) -> (String, Option<String>) {
    let mut text = String::new();
    let mut href: Option<String> = None;
    let mut depth = 1usize;

    while let Some(event) = reader.read_event() {
        match event {
            Event::End(name) if name == closer => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            Event::Start(element) if element.qname == closer => depth += 1,
            Event::Start(element) if element.is(ns::ODF_TEXT, "a") => {
                if href.is_none()
                    && let Some(target) = element.attr_local("href")
                    && !target.trim().is_empty()
                {
                    href = Some(target.to_string());
                }
            }
            Event::Text(chunk) => text.push_str(&chunk),
            _ => {}
        }
    }
    (text, href)
}

/// Whether displayed text is one of the spreadsheet error codes.
///
/// Matched against the closed set rather than by the leading `#`, because a
/// column too narrow for its number displays as `######` and that is a
/// rendering artefact, not an error: there the stored value is the answer.
fn is_error_code(text: &str) -> bool {
    matches!(
        text.trim(),
        "#DIV/0!"
            | "#N/A"
            | "#NAME?"
            | "#NULL!"
            | "#NUM!"
            | "#REF!"
            | "#VALUE!"
            | "#GETTING_DATA"
            | "#SPILL!"
            | "#CALC!"
            | "#FIELD!"
            | "#BLOCKED!"
            | "#CONNECT!"
            | "#UNKNOWN!"
    )
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
        // ODF writes a date as a full timestamp even when the cell shows only
        // the date. Midnight means the cell holds a date, so the time is
        // dropped -- which is also what the OOXML readers produce from a
        // serial with no fractional part.
        "date" => {
            let value = element.attr_local("date-value").unwrap_or("");
            match value.split_once('T') {
                Some((day, time)) if time.trim_matches(['0', ':', '.']).is_empty() => {
                    day.to_string()
                }
                _ => value.replace('T', " "),
            }
        }
        "time" => element.attr_local("time-value").unwrap_or("").to_string(),
        "string" => element.attr_local("string-value").unwrap_or("").to_string(),
        _ => String::new(),
    }
}

use crate::formats::format_number_text as trim_float;

fn repeat_count(element: &Element, attribute: &str) -> usize {
    element
        .attr_local(attribute)
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(1)
        .max(1)
}
