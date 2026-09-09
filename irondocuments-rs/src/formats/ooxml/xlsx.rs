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
use std::collections::HashMap;

use crate::container::zip::ZipArchive;
use crate::doc::{Block, Section, SemanticDoc, Table};
use crate::formats::SheetCell;
use crate::formats::ooxml::{Package, Rels, attr_i64, resolve_path};
use crate::sheet::{Cell, CellError, DefinedName, Range, Sheet, Value, Workbook};
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
    let styles = CellStyles::read(archive, &base);

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

        // A cell's hyperlink is stated away from the cell, at the end of the
        // sheet and through a relationship of the sheet's own -- so reading
        // the cells alone gave the link's text and lost where it points.
        let links = read_hyperlinks(&bytes, &Rels::for_part(archive, &path), &base);
        let grid = read_sheet(&bytes, &shared, &styles, &links);
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

        let rows = crate::formats::sheet_rows(grid);

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

/// The hyperlink on each cell, by `(column, row)`.
///
/// `<hyperlink ref="B11" r:id="rId1"/>` sits after the cells and names a
/// relationship of the worksheet part, not of the workbook. A `location`
/// without a relationship is a reference within the workbook rather than a
/// URL, and is kept as a fragment so it is not mistaken for one.
fn read_hyperlinks(xml: &[u8], rels: &Rels, base: &str) -> HashMap<(usize, usize), String> {
    /// A sheet naming more links than it has cells is not a sheet.
    const MAX_LINKS: usize = 1 << 16;

    let mut links = HashMap::new();
    let mut reader = Reader::new(xml);

    while let Some(event) = reader.read_event() {
        let Event::Start(element) = event else {
            continue;
        };
        if element.local != "hyperlink" || links.len() >= MAX_LINKS {
            continue;
        }
        let href = element
            .attr_local("id")
            .and_then(|id| rels.resolve(base, id))
            .or_else(|| {
                element
                    .attr_local("location")
                    .filter(|value| !value.trim().is_empty())
                    .map(|value| format!("#{value}"))
            });
        let (Some(href), Some(reference)) = (href, element.attr_local("ref")) else {
            continue;
        };
        // The reference may be a range, and every cell in it carries the link.
        for (column, row) in reference_cells(reference) {
            if links.len() >= MAX_LINKS {
                break;
            }
            links.insert((column, row), href.clone());
        }
    }
    links
}

/// Every cell a reference names, whether it is one cell or a range.
fn reference_cells(reference: &str) -> Vec<(usize, usize)> {
    /// A range wider or taller than this is a whole-sheet link, and marking
    /// every cell of it says nothing.
    const MAX_SPAN: usize = 4096;

    let (from, to) = match reference.split_once(':') {
        Some((from, to)) => (from, to),
        None => (reference, reference),
    };
    let (Some(from), Some(to)) = (parse_reference(from), parse_reference(to)) else {
        return Vec::new();
    };
    let columns = from.0.min(to.0)..=from.0.max(to.0);
    let rows = from.1.min(to.1)..=from.1.max(to.1);
    if columns.clone().count() * rows.clone().count() > MAX_SPAN {
        return Vec::new();
    }
    rows.flat_map(|row| columns.clone().map(move |column| (column, row)))
        .collect()
}

/// Read a worksheet into a dense, rectangular grid of strings.
fn read_sheet(
    xml: &[u8],
    shared: &SharedStrings,
    styles: &CellStyles,
    links: &HashMap<(usize, usize), String>,
) -> Vec<Vec<SheetCell>> {
    let mut grid: Vec<Vec<SheetCell>> = Vec::new();
    let mut reader = Reader::new(xml);
    let mut cells_read = 0usize;
    let mut row_index = 0usize;
    // A hidden row or column is content the author chose not to show, and its
    // text is as extractable as any other -- the same reasoning that already
    // skips a hidden sheet, one division down. Columns are stated before the
    // rows and removed once the grid is built.
    let mut row_hidden = false;
    let mut hidden_columns: Vec<usize> = Vec::new();
    let mut hidden_rows: Vec<usize> = Vec::new();

    while let Some(event) = reader.read_event() {
        let Event::Start(element) = event else {
            continue;
        };
        match element.local.as_str() {
            "col" => {
                if matches!(element.attr_local("hidden"), Some("1") | Some("true")) {
                    let first = attr_i64(&element, "min").unwrap_or(1).max(1) as usize;
                    let last = attr_i64(&element, "max").unwrap_or(0).max(1) as usize;
                    // The range is inclusive and 1-based, and may span the
                    // whole sheet, so it is bounded before being materialised.
                    for column in first..=last.min(MAX_COLUMNS) {
                        hidden_columns.push(column - 1);
                    }
                }
            }
            "row" => {
                // `r` is 1-based; a missing one means "the next row".
                row_index = attr_i64(&element, "r")
                    .map(|r| (r.max(1) as usize) - 1)
                    .unwrap_or(grid.len());
                row_hidden = matches!(element.attr_local("hidden"), Some("1") | Some("true"));
                if row_hidden {
                    hidden_rows.push(row_index);
                }
            }
            "c" => {
                if cells_read >= MAX_CELLS_PER_SHEET {
                    break;
                }
                cells_read += 1;
                if row_hidden {
                    continue;
                }

                let (column, row) = cell_position(&element, row_index, &grid);
                if column >= MAX_COLUMNS {
                    continue;
                }
                let value = read_cell_value(&mut reader, &element, shared, styles);
                if value.is_empty() {
                    continue;
                }

                if grid.len() <= row {
                    grid.resize(row + 1, Vec::new());
                }
                if grid[row].len() <= column {
                    grid[row].resize(column + 1, SheetCell::default());
                }
                grid[row][column] = SheetCell {
                    text: value,
                    href: links.get(&(column, row)).cloned(),
                };
            }
            _ => {}
        }
    }

    for row in &mut grid {
        for column in hidden_columns.iter().rev() {
            if *column < row.len() {
                row.remove(*column);
            }
        }
    }
    // Dropped outright rather than left blank: skipping only the cells would
    // leave a gap where the row was, and the other two readers of the same
    // workbook remove the row itself.
    let mut index = 0usize;
    grid.retain(|_| {
        let keep = !hidden_rows.contains(&index);
        index += 1;
        keep
    });

    crate::formats::square_grid(&mut grid);
    grid
}

/// Resolve a cell's `(column, row)` from its `r` reference.
fn cell_position(element: &Element, row_index: usize, grid: &[Vec<SheetCell>]) -> (usize, usize) {
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
fn read_cell_value(
    reader: &mut Reader,
    start: &Element,
    shared: &SharedStrings,
    styles: &CellStyles,
) -> String {
    let cell_type = start.attr_local("t").unwrap_or("n").to_string();
    let style_index = attr_i64(start, "s").and_then(|v| usize::try_from(v).ok());
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
        "n" => {
            let text = match inline.trim().is_empty() {
                false => inline.trim(),
                true => value.trim(),
            };
            // A date is a number with a format that says otherwise.
            match styles.shows_a_date(style_index) {
                true => text
                    .parse::<f64>()
                    .ok()
                    .and_then(crate::formats::format_date_serial)
                    .unwrap_or_else(|| crate::formats::format_number_text(text)),
                false => crate::formats::format_number_text(text),
            }
        }
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

/// Which cell styles show a date.
///
/// A `<c>` carries `s="N"`, an index into `<cellXfs>`; that entry names a
/// number format; and the format decides whether the stored number is a
/// quantity or a moment. Without reading the chain a date cell reports its
/// serial -- `46095` for 2026-03-14 -- which is the truth of the file and not
/// the answer to the question.
#[derive(Debug, Default)]
struct CellStyles {
    /// Style index -> number format id, in `<cellXfs>` order.
    formats: Vec<u32>,
    /// Custom format id -> its code, for anything at 164 or above.
    codes: HashMap<u32, String>,
}

impl CellStyles {
    fn read(archive: &ZipArchive, base: &str) -> CellStyles {
        let mut styles = CellStyles::default();
        let Some(bytes) = archive.read_optional(&resolve_path(base, "styles.xml")) else {
            return styles;
        };
        let mut reader = Reader::new(&bytes);
        // `<cellStyleXfs>` holds named-style definitions and `<cellXfs>` the
        // per-cell ones; only the latter is what `s` indexes.
        let mut in_cell_xfs = false;
        while let Some(event) = reader.read_event() {
            match event {
                // A self-closing element arrives as a Start followed by an
                // End, so matching Start alone reaches `<xf .../>` too.
                Event::Start(element) => match element.local.as_str() {
                    "cellXfs" => in_cell_xfs = true,
                    "xf" if in_cell_xfs => {
                        styles
                            .formats
                            .push(attr_i64(&element, "numFmtId").unwrap_or(0).max(0) as u32);
                    }
                    "numFmt" => {
                        if let (Some(id), Some(code)) = (
                            attr_i64(&element, "numFmtId"),
                            element.attr_local("formatCode"),
                        ) {
                            styles.codes.insert(id.max(0) as u32, code.to_string());
                        }
                    }
                    _ => {}
                },
                Event::End(name) if name.ends_with("cellXfs") => in_cell_xfs = false,
                _ => {}
            }
        }
        styles
    }

    /// The number-format code for a style, when it has a non-default one.
    ///
    /// `General` is the absence of a format rather than a format, so it is
    /// reported as `None`; storing it would put a meaningless string on the
    /// majority of cells in every workbook.
    fn format_code(&self, style_index: Option<usize>) -> Option<String> {
        let id = style_index.and_then(|at| self.formats.get(at).copied())?;
        if let Some(code) = self.codes.get(&id) {
            return match code.trim().is_empty() || code == "General" {
                true => None,
                false => Some(code.clone()),
            };
        }
        // A built-in id with no code of its own is only worth carrying when it
        // changes how the number reads; the date formats are that case.
        match crate::formats::is_date_format(id, None) {
            true => Some(format!("builtin:{id}")),
            false => None,
        }
    }

    fn shows_a_date(&self, style_index: Option<usize>) -> bool {
        let Some(id) = style_index.and_then(|at| self.formats.get(at).copied()) else {
            return false;
        };
        crate::formats::is_date_format(id, self.codes.get(&id).map(String::as_str))
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

// ---------------------------------------------------------------------------
// The cell reader
//
// `parse` above answers "what does this spreadsheet say?" and throws away
// everything that is not text. This half answers "what is this spreadsheet?"
// and keeps the parts a conversion has to carry: the type of each value, the
// formula behind it, the format that decides whether a number is a date, and
// the rows the author hid.
// ---------------------------------------------------------------------------

/// Parse a SpreadsheetML package into the cell model.
pub fn parse_workbook(archive: &ZipArchive, package: &Package) -> Result<Workbook, PdfError> {
    let workbook_xml = archive.read(&package.main_part)?;
    let base = package.base();
    let shared = SharedStrings::read(archive, &base);
    let styles = CellStyles::read(archive, &base);

    let mut workbook = Workbook {
        defined_names: read_defined_names(&workbook_xml),
        ..Workbook::default()
    };

    for sheet in read_sheet_list_including_hidden(&workbook_xml) {
        let Some(path) = package.main_rels.resolve(&base, &sheet.relationship) else {
            continue;
        };
        let Some(bytes) = archive.read_optional(&path) else {
            continue;
        };

        let links = read_hyperlinks(&bytes, &Rels::for_part(archive, &path), &base);
        let mut parsed = read_cells(&bytes, &shared, &styles, &links);
        parsed.name = sheet.name;
        parsed.hidden = sheet.hidden;
        workbook.sheets.push(parsed);
    }

    if workbook.sheets.is_empty() {
        return Err(PdfError::MissingPart("xlsx contains no worksheets".into()));
    }
    Ok(workbook)
}

/// A worksheet as named by the workbook, hidden ones included.
struct SheetEntry {
    name: String,
    relationship: String,
    hidden: bool,
}

/// Like [`read_sheet_list`], but keeps hidden sheets and says which they are.
///
/// The text reader drops them; a conversion cannot, or the workbook it writes
/// is missing sheets that were in the one it read.
fn read_sheet_list_including_hidden(xml: &[u8]) -> Vec<SheetEntry> {
    let mut sheets = Vec::new();
    let mut reader = Reader::new(xml);
    while let Some(event) = reader.read_event() {
        let Event::Start(element) = event else {
            continue;
        };
        if element.local != "sheet" {
            continue;
        }
        let Some(relationship) = element.attr_local("id") else {
            continue;
        };
        sheets.push(SheetEntry {
            name: element
                .attr_local("name")
                .unwrap_or("Sheet")
                .trim()
                .to_string(),
            relationship: relationship.to_string(),
            hidden: matches!(
                element.attr_local("state"),
                Some("hidden") | Some("veryHidden")
            ),
        });
    }
    sheets
}

/// Workbook-level names, such as `Tax_Rate` standing for `Rates!$A$1`.
fn read_defined_names(xml: &[u8]) -> Vec<DefinedName> {
    /// A workbook naming more ranges than this is not one a person wrote.
    const MAX_NAMES: usize = 1 << 14;

    let mut names = Vec::new();
    let mut reader = Reader::new(xml);
    while let Some(event) = reader.read_event() {
        let Event::Start(element) = event else {
            continue;
        };
        if element.local != "definedName" || names.len() >= MAX_NAMES {
            continue;
        }
        let Some(name) = element.attr_local("name").map(str::to_string) else {
            continue;
        };
        // `localSheetId` is what scopes a name to one sheet; without it the
        // name belongs to the workbook.
        let sheet = attr_i64(&element, "localSheetId").and_then(|id| u32::try_from(id).ok());
        let refers_to = crate::xml::text_of(&mut reader, &element.qname);
        if refers_to.trim().is_empty() {
            continue;
        }
        names.push(DefinedName {
            name,
            refers_to: refers_to.trim().to_string(),
            sheet,
        });
    }
    names
}

/// Read one worksheet into cells, keeping types, formulas and formats.
fn read_cells(
    xml: &[u8],
    shared: &SharedStrings,
    styles: &CellStyles,
    links: &HashMap<(usize, usize), String>,
) -> Sheet {
    let mut sheet = Sheet::default();
    let mut reader = Reader::new(xml);
    let mut cells_read = 0usize;
    let mut row_index = 0usize;
    let mut last_column = 0usize;

    while let Some(event) = reader.read_event() {
        let Event::Start(element) = event else {
            continue;
        };
        match element.local.as_str() {
            // `<pane>` states the split; only a frozen one is a frozen pane,
            // a split pane is a scrolling convenience with no such meaning.
            "pane" => {
                if matches!(
                    element.attr_local("state"),
                    Some("frozen") | Some("frozenSplit")
                ) {
                    let columns = attr_i64(&element, "xSplit").unwrap_or(0).max(0) as u32;
                    let rows = attr_i64(&element, "ySplit").unwrap_or(0).max(0) as u32;
                    if rows > 0 || columns > 0 {
                        sheet.frozen = Some((rows, columns));
                    }
                }
            }
            "col" => {
                let first = attr_i64(&element, "min").unwrap_or(1).max(1) as usize;
                let last = attr_i64(&element, "max").unwrap_or(0).max(1) as usize;
                let hidden = matches!(element.attr_local("hidden"), Some("1") | Some("true"));
                let width = element
                    .attr_local("width")
                    .and_then(|w| w.parse::<f64>().ok());
                for column in first..=last.min(MAX_COLUMNS) {
                    let index = column - 1;
                    if hidden {
                        sheet.hidden_columns.push(index as u32);
                    }
                    if let Some(width) = width {
                        if sheet.column_widths.len() <= index {
                            sheet.column_widths.resize(index + 1, 0.0);
                        }
                        sheet.column_widths[index] = width;
                    }
                }
            }
            "row" => {
                row_index = attr_i64(&element, "r")
                    .map(|r| (r.max(1) as usize) - 1)
                    .unwrap_or(row_index + 1);
                last_column = 0;
                if matches!(element.attr_local("hidden"), Some("1") | Some("true")) {
                    sheet.hidden_rows.push(row_index as u32);
                }
            }
            "mergeCell" => {
                if let Some(range) = element.attr_local("ref").and_then(Range::parse) {
                    sheet.merges.push(range);
                }
            }
            "c" => {
                if cells_read >= MAX_CELLS_PER_SHEET {
                    break;
                }
                cells_read += 1;

                let (column, row) = match element.attr_local("r").and_then(parse_reference) {
                    Some(position) => position,
                    // Without a reference the cell follows the previous one in
                    // its row, which is how a sheet written without `r` is
                    // meant to be read.
                    None => (last_column, row_index),
                };
                last_column = column + 1;
                if column >= MAX_COLUMNS {
                    continue;
                }

                let style = attr_i64(&element, "s").and_then(|v| usize::try_from(v).ok());
                let (value, formula) = read_typed_cell(&mut reader, &element, shared);

                // A cell with nothing in it and no formula is not a cell. One
                // that is empty but computed is: the formula is the content.
                if value.is_empty() && formula.is_none() {
                    continue;
                }

                sheet.set(Cell {
                    row: row as u32,
                    column: column as u32,
                    value,
                    formula,
                    number_format: styles.format_code(style),
                    href: links.get(&(column, row)).cloned(),
                });
            }
            _ => {}
        }
    }

    sheet.hidden_rows.sort_unstable();
    sheet.hidden_rows.dedup();
    sheet.hidden_columns.sort_unstable();
    sheet.hidden_columns.dedup();
    sheet
}

/// Read one `<c>` as a typed value and, if it has one, its formula.
///
/// The difference from [`read_cell_value`] is what is kept rather than how it
/// is found: the type stays a type instead of becoming its rendering, and the
/// `<f>` element is read instead of skipped.
fn read_typed_cell(
    reader: &mut Reader,
    start: &Element,
    shared: &SharedStrings,
) -> (Value, Option<String>) {
    let cell_type = start.attr_local("t").unwrap_or("n").to_string();
    let mut stored = String::new();
    let mut inline = String::new();
    let mut formula = None;

    while let Some(event) = reader.read_event() {
        match event {
            Event::End(name) if name == start.qname => break,
            Event::Start(element) => match element.local.as_str() {
                "v" => stored = crate::xml::text_of(reader, &element.qname),
                "is" | "t" => {
                    let text = crate::xml::text_of(reader, &element.qname);
                    if !text.is_empty() {
                        inline.push_str(&text);
                    }
                }
                "f" => {
                    let text = crate::xml::text_of(reader, &element.qname);
                    // A shared formula is stated once and referenced by the
                    // cells that reuse it; those arrive with an empty `<f>`,
                    // and an empty string is not a formula.
                    if !text.trim().is_empty() {
                        formula = Some(text.trim().to_string());
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }

    let stored = stored.trim();
    let inline = inline.trim();

    let value = match cell_type.as_str() {
        "s" => stored
            .parse::<usize>()
            .ok()
            .and_then(|index| shared.get(index))
            .map(Value::Text)
            .unwrap_or_default(),
        "inlineStr" => match inline.is_empty() {
            true => Value::Empty,
            false => Value::Text(inline.to_string()),
        },
        // A formula's string result, already text.
        "str" => match stored.is_empty() {
            true => Value::Empty,
            false => Value::Text(stored.to_string()),
        },
        "b" => match stored {
            "1" => Value::Bool(true),
            "0" => Value::Bool(false),
            _ => Value::Empty,
        },
        "e" => match CellError::from_code(stored) {
            Some(error) => Value::Error(error),
            // An error code this build does not know is kept as what the file
            // said rather than guessed at.
            None if !stored.is_empty() => Value::Text(stored.to_string()),
            None => Value::Empty,
        },
        // A number, and the default when `t` is absent. A date stays the number
        // it is stored as; the format code on the cell is what says otherwise,
        // and turning it into text here would be the same flattening this model
        // exists to avoid.
        _ => {
            let text = match inline.is_empty() {
                false => inline,
                true => stored,
            };
            match text.parse::<f64>() {
                Ok(number) => Value::Number(number),
                Err(_) if text.is_empty() => Value::Empty,
                Err(_) => Value::Text(text.to_string()),
            }
        }
    };

    (value, formula)
}
