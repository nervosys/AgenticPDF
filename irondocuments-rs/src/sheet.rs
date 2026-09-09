// SPDX-License-Identifier: AGPL-3.0-or-later
//! The spreadsheet model.
//!
//! Every spreadsheet this crate could previously read was flattened into a
//! [`Table`](crate::doc::Table) of strings: enough to print, useless to compute
//! with. A cell holding `=SUM(B2:B9)` arrived as the text `1783.5`, and the
//! question "what does this number depend on?" had no answer left in the file.
//!
//! This model keeps the three things that flattening destroys:
//!
//! - **The type.** A cell is a number, a string, a boolean or an error, not a
//!   rendering of one. `42` and `"42"` are different cells, and only the first
//!   can be summed.
//! - **The formula.** Both the expression and its cached result are stored, so
//!   a reader that cannot evaluate still shows the right value, and one that
//!   can knows what to recompute.
//! - **The address.** Cells carry their `(row, column)` rather than their
//!   position in a dense array, so a sheet with one cell at `ZZ10000` costs one
//!   cell rather than ten million.
//!
//! # Sparse, sorted, seekable
//!
//! `Sheet::cells` is sorted by `(row, column)` and held as a flat vector. That
//! ordering is the whole data structure: lookup is a binary search, iteration
//! is in reading order without a sort, and the on-disk form is the in-memory
//! form, so loading a sheet out of an ADF chunk is a decode rather than an
//! index rebuild. A `HashMap<(u32, u32), Cell>` would beat it on random writes
//! and lose on everything else a document format does.

use serde::{Deserialize, Serialize};

/// A workbook: the sheets, plus the names that span them.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Workbook {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sheets: Vec<Sheet>,
    /// Workbook- or sheet-scoped names, such as `Tax_Rate` pointing at
    /// `Rates!$B$2`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub defined_names: Vec<DefinedName>,
}

impl Workbook {
    pub fn sheet_by_name(&self, name: &str) -> Option<&Sheet> {
        self.sheets.iter().find(|sheet| sheet.name == name)
    }

    /// Total non-empty cells across every sheet.
    pub fn cell_count(&self) -> usize {
        self.sheets.iter().map(|sheet| sheet.cells.len()).sum()
    }
}

/// One worksheet.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Sheet {
    pub name: String,
    /// Non-empty cells, sorted by `(row, column)`. See the module docs.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cells: Vec<Cell>,
    /// Merged regions. A merge is a property of the sheet rather than of its
    /// anchor cell, because the cells it covers still exist and still hold
    /// values that a reader must not attribute to the anchor.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub merges: Vec<Range>,
    /// Column widths in characters, where the source states them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub column_widths: Vec<f64>,
    /// Rows and columns frozen at the top-left, as `(rows, columns)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frozen: Option<(u32, u32)>,
    /// The author hid this sheet.
    ///
    /// Hidden content is *kept* here and dropped by [`Sheet::to_section`]
    /// instead. This is a format rather than an extraction: silently losing a
    /// hidden sheet would corrupt a workbook on the way through, while
    /// surfacing its text would show a reader what the author chose to
    /// conceal. Storing it and filtering the view does neither.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub hidden: bool,
    /// Rows the author hid, by index.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hidden_rows: Vec<u32>,
    /// Columns the author hid, by index.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hidden_columns: Vec<u32>,
}

impl Sheet {
    pub fn new(name: impl Into<String>) -> Sheet {
        Sheet {
            name: name.into(),
            ..Sheet::default()
        }
    }

    /// The cell at `(row, column)`, if it holds anything.
    pub fn cell(&self, row: u32, column: u32) -> Option<&Cell> {
        self.cells
            .binary_search_by_key(&(row, column), |cell| (cell.row, cell.column))
            .ok()
            .map(|at| &self.cells[at])
    }

    /// Insert or replace a cell, keeping `cells` sorted.
    ///
    /// Importers append in reading order, which is already sorted, so the
    /// common path is a push after a single comparison.
    pub fn set(&mut self, cell: Cell) {
        let key = (cell.row, cell.column);
        match self.cells.last() {
            Some(last) if (last.row, last.column) < key => self.cells.push(cell),
            _ => match self
                .cells
                .binary_search_by_key(&key, |cell| (cell.row, cell.column))
            {
                Ok(at) => self.cells[at] = cell,
                Err(at) => self.cells.insert(at, cell),
            },
        }
    }

    /// The extent of the used range, as `(rows, columns)`. Both are exclusive
    /// upper bounds, so an empty sheet is `(0, 0)`.
    pub fn extent(&self) -> (u32, u32) {
        let mut rows = 0;
        let mut columns = 0;
        for cell in &self.cells {
            rows = rows.max(cell.row + 1);
            columns = columns.max(cell.column + 1);
        }
        (rows, columns)
    }

    /// The used rows, each as the slice of cells it holds. Rows with no cells
    /// are not yielded — a sheet is sparse in rows as well as in columns.
    pub fn rows(&self) -> impl Iterator<Item = (u32, &[Cell])> {
        let mut at = 0usize;
        std::iter::from_fn(move || {
            let first = self.cells.get(at)?;
            let row = first.row;
            let start = at;
            while self.cells.get(at).is_some_and(|cell| cell.row == row) {
                at += 1;
            }
            Some((row, &self.cells[start..at]))
        })
    }

    /// Cells holding a formula, in reading order.
    pub fn formulas(&self) -> impl Iterator<Item = &Cell> {
        self.cells.iter().filter(|cell| cell.formula.is_some())
    }
}

/// The text of one row, as the retrieval index and the provenance table both
/// see it.
///
/// One definition rather than two: the index stores this text and the
/// provenance row stores its hash, so if the two ever disagreed every citation
/// out of a spreadsheet would fail to verify.
pub fn row_text(cells: &[Cell]) -> String {
    let mut text = String::new();
    for cell in cells {
        let value = cell.value.to_text();
        if value.is_empty() {
            continue;
        }
        if !text.is_empty() {
            text.push(' ');
        }
        text.push_str(&value);
    }
    text
}

/// One cell: where it is, what it holds, and how it got there.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Cell {
    /// Zero-based row.
    pub row: u32,
    /// Zero-based column.
    pub column: u32,
    pub value: Value,
    /// The formula source without its leading `=`, when the cell is computed.
    /// The cached result stays in `value`, so a reader that does not evaluate
    /// still shows what the author saw.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub formula: Option<String>,
    /// The number-format code, such as `0.00` or `yyyy-mm-dd`. Kept because it
    /// is the only thing that distinguishes a date from the number it is
    /// stored as.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub number_format: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub href: Option<String>,
}

impl Cell {
    /// A cell holding a literal value.
    pub fn new(row: u32, column: u32, value: Value) -> Cell {
        Cell {
            row,
            column,
            value,
            ..Cell::default()
        }
    }

    /// A computed cell: the expression and the result the source cached for it.
    pub fn computed(row: u32, column: u32, formula: impl Into<String>, cached: Value) -> Cell {
        Cell {
            row,
            column,
            value: cached,
            formula: Some(formula.into()),
            ..Cell::default()
        }
    }

    /// This cell's A1 reference, such as `B7`.
    pub fn reference(&self) -> String {
        a1(self.row, self.column)
    }
}

/// What a cell holds.
///
/// Adjacently tagged so the JSON an agent reads says which of these it got —
/// `{"type":"number","value":42}` rather than a bare `42` that could equally
/// have been text.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "lowercase")]
pub enum Value {
    /// Present but empty: a cell that carries a format or a merge, no content.
    #[default]
    Empty,
    Number(f64),
    Text(String),
    Bool(bool),
    /// A spreadsheet error, which is a value in its own right: it propagates
    /// through formulas and must survive a round trip.
    Error(CellError),
}

impl Value {
    /// How the cell reads to a human, ignoring number formatting.
    pub fn to_text(&self) -> String {
        match self {
            Value::Empty => String::new(),
            Value::Number(number) => crate::formats::format_number_text(&number.to_string()),
            Value::Text(text) => text.clone(),
            Value::Bool(true) => "TRUE".to_string(),
            Value::Bool(false) => "FALSE".to_string(),
            Value::Error(error) => error.code().to_string(),
        }
    }

    pub fn is_empty(&self) -> bool {
        matches!(self, Value::Empty)
    }

    /// The number this cell contributes to a sum; `None` for anything that is
    /// not a number, which is what keeps text out of arithmetic.
    pub fn as_number(&self) -> Option<f64> {
        match self {
            Value::Number(number) => Some(*number),
            _ => None,
        }
    }
}

/// The seven spreadsheet errors, which are identical across Excel, OpenDocument
/// and Google Sheets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CellError {
    /// Division by zero.
    Div0,
    /// A lookup found nothing.
    NotAvailable,
    /// An unrecognised name.
    Name,
    /// An empty intersection of two ranges.
    Null,
    /// A number too large, or undefined for the operation.
    Num,
    /// A reference to a cell that no longer exists.
    Ref,
    /// An operand of the wrong type.
    Value,
}

impl CellError {
    pub fn code(self) -> &'static str {
        match self {
            CellError::Div0 => "#DIV/0!",
            CellError::NotAvailable => "#N/A",
            CellError::Name => "#NAME?",
            CellError::Null => "#NULL!",
            CellError::Num => "#NUM!",
            CellError::Ref => "#REF!",
            CellError::Value => "#VALUE!",
        }
    }

    /// Parse the code a file stores. Unknown codes are not guessed at: a cell
    /// whose error this build does not know is better kept as text than
    /// silently turned into the wrong error.
    pub fn from_code(code: &str) -> Option<CellError> {
        match code.trim().to_ascii_uppercase().as_str() {
            "#DIV/0!" => Some(CellError::Div0),
            "#N/A" => Some(CellError::NotAvailable),
            "#NAME?" => Some(CellError::Name),
            "#NULL!" => Some(CellError::Null),
            "#NUM!" => Some(CellError::Num),
            "#REF!" => Some(CellError::Ref),
            "#VALUE!" => Some(CellError::Value),
            _ => None,
        }
    }

    pub fn to_u8(self) -> u8 {
        match self {
            CellError::Div0 => 0,
            CellError::NotAvailable => 1,
            CellError::Name => 2,
            CellError::Null => 3,
            CellError::Num => 4,
            CellError::Ref => 5,
            CellError::Value => 6,
        }
    }

    pub fn from_u8(value: u8) -> Option<CellError> {
        match value {
            0 => Some(CellError::Div0),
            1 => Some(CellError::NotAvailable),
            2 => Some(CellError::Name),
            3 => Some(CellError::Null),
            4 => Some(CellError::Num),
            5 => Some(CellError::Ref),
            6 => Some(CellError::Value),
            _ => None,
        }
    }
}

/// A rectangular region, inclusive at both ends.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Range {
    pub first_row: u32,
    pub first_column: u32,
    pub last_row: u32,
    pub last_column: u32,
}

impl Range {
    pub fn contains(&self, row: u32, column: u32) -> bool {
        (self.first_row..=self.last_row).contains(&row)
            && (self.first_column..=self.last_column).contains(&column)
    }

    /// `A1:C3` form.
    pub fn reference(&self) -> String {
        format!(
            "{}:{}",
            a1(self.first_row, self.first_column),
            a1(self.last_row, self.last_column)
        )
    }

    /// Parse `A1:C3`, or a single `B2` as a one-cell range.
    pub fn parse(reference: &str) -> Option<Range> {
        let (from, to) = reference.split_once(':').unwrap_or((reference, reference));
        let (first_row, first_column) = parse_a1(from)?;
        let (last_row, last_column) = parse_a1(to)?;
        Some(Range {
            first_row: first_row.min(last_row),
            first_column: first_column.min(last_column),
            last_row: first_row.max(last_row),
            last_column: first_column.max(last_column),
        })
    }
}

/// A named reference, such as `Tax_Rate` standing for `Rates!$B$2`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DefinedName {
    pub name: String,
    /// The formula the name expands to, verbatim.
    pub refers_to: String,
    /// The sheet the name is scoped to; `None` for workbook scope.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sheet: Option<u32>,
}

/// The A1 reference for a zero-based `(row, column)`.
pub fn a1(row: u32, column: u32) -> String {
    format!("{}{}", column_name(column), row + 1)
}

/// The column letters for a zero-based index: 0 is `A`, 25 is `Z`, 26 is `AA`.
///
/// Bijective base-26, which is *not* ordinary base-26: there is no zero digit,
/// so `Z` is followed by `AA` rather than by `BA`.
pub fn column_name(column: u32) -> String {
    let mut letters = Vec::new();
    let mut value = column as u64 + 1;
    while value > 0 {
        let digit = ((value - 1) % 26) as u8;
        letters.push(b'A' + digit);
        value = (value - 1) / 26;
    }
    letters.reverse();
    String::from_utf8(letters).unwrap_or_default()
}

/// Parse an A1 reference into a zero-based `(row, column)`.
///
/// Absolute markers are accepted and dropped: `$B$7` addresses the same cell as
/// `B7`, and the difference only matters when a formula is copied.
pub fn parse_a1(reference: &str) -> Option<(u32, u32)> {
    let reference = reference.trim();
    let body = reference.strip_prefix('$').unwrap_or(reference);
    let letters: String = body
        .chars()
        .take_while(|c| c.is_ascii_alphabetic())
        .collect();
    if letters.is_empty() {
        return None;
    }
    let rest = &body[letters.len()..];
    let digits = rest.strip_prefix('$').unwrap_or(rest);
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }

    let mut column = 0u32;
    for letter in letters.bytes() {
        let digit = letter.to_ascii_uppercase().checked_sub(b'A')? as u32 + 1;
        column = column.checked_mul(26)?.checked_add(digit)?;
    }
    let row: u32 = digits.parse().ok()?;
    Some((row.checked_sub(1)?, column.checked_sub(1)?))
}

// ---------------------------------------------------------------------------
// The bridge to the prose model
//
// Everything this crate already does — Markdown and text output, retrieval
// chunking, typesetting, the structure tree — is written against
// [`SemanticDoc`](crate::doc::SemanticDoc). Rendering a workbook into that
// model on demand is what lets all of it work on a spreadsheet without any of
// it learning what a cell is.
//
// The conversion is deliberately lossy and deliberately not stored: types and
// formulas do not survive it. The cells stay the file's truth, and this is the
// view.
// ---------------------------------------------------------------------------

impl Sheet {
    /// The worksheet as a document section holding one table.
    pub fn to_section(&self) -> crate::doc::Section {
        use crate::doc::{Block, Row, Section, SectionKind, Table};

        let (_, columns) = self.extent();
        let mut rows: Vec<Row> = Vec::new();

        // Hidden columns are removed rather than blanked, so the remaining
        // columns close up exactly as they did before cells were modelled.
        let visible: Vec<u32> = (0..columns)
            .filter(|column| !self.hidden_columns.contains(column))
            .collect();

        for (index, cells) in self.rows() {
            if self.hidden_rows.contains(&index) {
                continue;
            }
            let mut row = Row {
                cells: vec![crate::doc::Cell::default(); visible.len()],
            };
            for cell in cells {
                let text = cell.value.to_text();
                if text.is_empty() {
                    continue;
                }
                // A hidden column has no slot at all; the rest shift left by
                // however many hidden columns precede them.
                let Ok(at) = visible.binary_search(&cell.column) else {
                    continue;
                };
                if let Some(slot) = row.cells.get_mut(at) {
                    let run = crate::doc::Run {
                        text,
                        style: Default::default(),
                    };
                    // A cell's hyperlink is part of what it says, so it
                    // survives into the view rather than being flattened away
                    // with everything else.
                    let content = match &cell.href {
                        Some(href) => vec![crate::doc::Inline::Link {
                            href: href.clone(),
                            runs: vec![run],
                        }],
                        None => vec![crate::doc::Inline::Run(run)],
                    };
                    *slot = crate::doc::Cell {
                        blocks: vec![Block::Paragraph {
                            content,
                            align: Default::default(),
                            indent: 0.0,
                        }],
                        col_span: 1,
                        row_span: 1,
                    };
                }
            }
            rows.push(row);
        }

        let blocks = match rows.is_empty() {
            true => Vec::new(),
            false => vec![Block::Table(Table {
                // A spreadsheet carries no header marker. The first used row is
                // one by overwhelming convention, and the Markdown writer needs
                // one to emit a table at all.
                header_rows: 1.min(rows.len()),
                rows,
                ..Table::default()
            })],
        };

        Section {
            kind: SectionKind::Sheet,
            title: Some(self.name.clone()),
            blocks,
            ..Section::default()
        }
    }
}

impl Workbook {
    /// The workbook as a document: one section per visible sheet.
    ///
    /// Hidden sheets are omitted for the same reason `to_section` omits hidden
    /// rows — they remain in the workbook, they just do not become text.
    pub fn to_semantic(&self) -> crate::doc::SemanticDoc {
        crate::doc::SemanticDoc {
            sections: self
                .sheets
                .iter()
                .filter(|sheet| !sheet.hidden)
                .map(Sheet::to_section)
                .collect(),
            ..crate::doc::SemanticDoc::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn column_names_are_bijective_base_26() {
        assert_eq!(column_name(0), "A");
        assert_eq!(column_name(25), "Z");
        // The case ordinary base-26 gets wrong: after Z comes AA, not BA.
        assert_eq!(column_name(26), "AA");
        assert_eq!(column_name(51), "AZ");
        assert_eq!(column_name(52), "BA");
        assert_eq!(column_name(16_383), "XFD");
    }

    #[test]
    fn a1_round_trips() {
        for (row, column) in [(0, 0), (6, 1), (9_999, 16_383), (41, 27)] {
            assert_eq!(parse_a1(&a1(row, column)), Some((row, column)));
        }
    }

    #[test]
    fn absolute_references_address_the_same_cell() {
        assert_eq!(parse_a1("$B$7"), parse_a1("B7"));
        assert_eq!(parse_a1("B$7"), parse_a1("B7"));
    }

    #[test]
    fn rubbish_references_are_rejected() {
        for bad in ["", "7", "B", "$", "B7B", "-1"] {
            assert_eq!(parse_a1(bad), None, "{bad} should not parse");
        }
    }

    #[test]
    fn cells_stay_sorted_however_they_are_inserted() {
        let mut sheet = Sheet::new("Sheet1");
        sheet.set(Cell::new(5, 0, Value::Number(1.0)));
        sheet.set(Cell::new(0, 3, Value::Number(2.0)));
        sheet.set(Cell::new(0, 1, Value::Number(3.0)));
        sheet.set(Cell::new(5, 0, Value::Number(9.0)));

        let keys: Vec<_> = sheet.cells.iter().map(|c| (c.row, c.column)).collect();
        assert_eq!(keys, vec![(0, 1), (0, 3), (5, 0)]);
        // The second write to (5, 0) replaced rather than duplicated.
        assert_eq!(sheet.cell(5, 0).unwrap().value, Value::Number(9.0));
    }

    #[test]
    fn rows_group_a_sparse_sheet() {
        let mut sheet = Sheet::new("Sheet1");
        sheet.set(Cell::new(0, 0, Value::Text("a".into())));
        sheet.set(Cell::new(0, 1, Value::Text("b".into())));
        sheet.set(Cell::new(4, 0, Value::Text("c".into())));

        let rows: Vec<_> = sheet
            .rows()
            .map(|(row, cells)| (row, cells.len()))
            .collect();
        // Rows 1 to 3 hold nothing and are not yielded at all.
        assert_eq!(rows, vec![(0, 2), (4, 1)]);
        assert_eq!(sheet.extent(), (5, 2));
    }

    #[test]
    fn ranges_parse_and_round_trip() {
        let range = Range::parse("B2:D10").unwrap();
        assert_eq!(range.reference(), "B2:D10");
        assert!(range.contains(5, 2));
        assert!(!range.contains(0, 2));
        // Reversed corners normalise rather than producing an empty range.
        assert_eq!(Range::parse("D10:B2"), Some(range));
        assert_eq!(Range::parse("C3").unwrap().reference(), "C3:C3");
    }

    #[test]
    fn error_codes_round_trip() {
        for error in [
            CellError::Div0,
            CellError::NotAvailable,
            CellError::Name,
            CellError::Null,
            CellError::Num,
            CellError::Ref,
            CellError::Value,
        ] {
            assert_eq!(CellError::from_code(error.code()), Some(error));
            assert_eq!(CellError::from_u8(error.to_u8()), Some(error));
        }
        assert_eq!(CellError::from_code("#WAT?"), None);
    }
}
