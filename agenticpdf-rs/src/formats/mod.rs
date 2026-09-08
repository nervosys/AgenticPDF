// SPDX-License-Identifier: AGPL-3.0-or-later
//! Per-format parsers producing the shared semantic model.
//!
//! Each submodule turns one family of bytes into a [`crate::doc::SemanticDoc`].
//! Nothing here knows about geometry, rendering or PDF — the typesetter takes
//! the semantic model from here and computes pages from it.

pub mod epub;
pub mod html;
pub mod legacy;
pub mod odf;
pub mod ooxml;
pub mod rtf;
pub mod text;

use crate::PdfError;
use crate::detect::Format;
use crate::doc::SemanticDoc;

/// Parse bytes of a known format into the semantic model.
///
/// PDF is absent by design: it has no authored structure to read, so it takes
/// the geometric path through [`crate::engine`] and has its structure inferred
/// by [`crate::layout`] instead.
pub fn parse(data: &[u8], format: Format) -> Result<SemanticDoc, PdfError> {
    let mut document = read(data, format)?;
    flag_unreadable_text(&mut document);
    Ok(document)
}

/// Flag text too small to read as hidden.
///
/// Type at a point or less is not small print, it is text put in a document to
/// be extracted rather than read -- the same trick as `font-size:0` in HTML,
/// which this reader has always caught, written in a way every other format
/// allows too. A document saved by Word as .docx, .doc, .rtf and .odt reported
/// the size faithfully through all four readers and flagged it in none.
///
/// Applied here rather than in each reader, so every format gets it and gets
/// the same rule. This is the one path documents are opened through.
///
/// The threshold is where it is because a point is the smallest size Word will
/// set and nothing at or below it can be read, while raising it starts catching
/// legitimate fine print.
///
/// Colour is deliberately not judged here. White text is invisible on a white
/// page and perfectly ordinary on a dark one, and the model carries no
/// background to tell those apart: in the same document above, the white text
/// of a table header styled by Word itself is indistinguishable from the white
/// text of a payload. Calling both hidden would report every such table as an
/// injection.
fn flag_unreadable_text(document: &mut SemanticDoc) {
    /// Points at or below which text cannot be read at all.
    const UNREADABLE: f64 = 1.0;

    for section in &mut document.sections {
        let mut flag = |run: &mut crate::doc::Run| {
            if run.style.size.is_some_and(|size| size <= UNREADABLE) {
                run.style.hidden = true;
            }
        };
        crate::doc::walk_runs_mut(&mut section.blocks, &mut flag);
        crate::doc::walk_runs_mut(&mut section.notes, &mut flag);
    }
}

/// Parse bytes of a known format, before the checks every format shares.
fn read(data: &[u8], format: Format) -> Result<SemanticDoc, PdfError> {
    match format {
        Format::Text => Ok(text::parse_text(data)),
        Format::Csv => Ok(text::parse_csv(data, None)),
        Format::Markdown => Ok(text::parse_markdown(data)),
        Format::Html => Ok(html::parse_html(data)),
        Format::Epub => epub::parse_epub(data),
        Format::Rtf => Ok(rtf::parse_rtf(data)),
        Format::Docx | Format::Xlsx | Format::Pptx => ooxml::parse(data, format),
        Format::Odt | Format::Ods | Format::Odp => odf::parse(data, format),
        Format::Doc | Format::Xls | Format::Ppt => legacy::parse(data, format),
        Format::Adf => Ok(crate::adf::AdfDoc::open(data)?.to_semantic()?),
        Format::Pdf => Err(PdfError::Unsupported(
            "PDF is parsed by the engine, not the semantic pipeline".into(),
        )),
    }
    // Note the absence of a catch-all: every `Format` now has a parser, and the
    // compiler enforces that a new one cannot be added without wiring it up.
}

/// Render a spreadsheet number the way the cell displays it.
///
/// The three spreadsheet formats store the same value three ways, and each
/// reader used to render it its own way. A workbook written by Excel holds
/// `0.28` as `<v>0.28000000000000003</v>` — seventeen significant digits, which
/// is how a producer guarantees the double round-trips — while the binary `.xls`
/// holds the double itself and ODF writes `office:value="0.28"`. Echoing the
/// stored text made one file out of three report a margin of
/// `0.28000000000000003`, which is the same number and not the same answer.
///
/// Formatting the parsed double gives the shortest form that round-trips, which
/// is what the cell shows and what a reader of the text expects.
/// The nesting level a built-in list style's name carries, zero-based.
///
/// Word records a list's depth here and nowhere else. A "List Bullet 2"
/// paragraph gets its own style with its own numbering and no explicit level,
/// in every format Word writes: `ListBullet2` in OOXML, the same id in
/// OpenDocument, `MsoListBullet2` in HTML, and the style name itself in the
/// binary formats. A reader that looks only at the numbering sees every item at
/// the top level and returns a flat list.
///
/// Returns `None` for anything that is not one of the three built-in families,
/// and for the unsuffixed first level, which needs no adjustment.
pub(crate) fn list_style_level(name: &str) -> Option<u8> {
    // OpenDocument escapes a space in a style name as `_20_`, so LibreOffice
    // writes `List_20_Bullet_20_2` where Word writes `ListBullet2`.
    let compact: String = name
        .to_ascii_lowercase()
        .replace("_20_", "")
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    // The two producers name the same depth differently. Word writes
    // `ListBullet2` for the second level and `ListBullet` for the first, so the
    // number is one more than the depth. LibreOffice writes `List 2` and
    // `List 1`, which comes to the same arithmetic. Longer prefixes are tried
    // first, or `list` would swallow `listbullet2` and find no number after it.
    let rest = [
        "listbullet",
        "listnumber",
        "listcontinue",
        "numbering",
        "list",
    ]
    .iter()
    .find_map(|prefix| {
        compact
            .strip_prefix(prefix)
            .and_then(|rest| rest.parse::<u8>().ok())
    })?;
    match (2..=9).contains(&rest) {
        true => Some(rest - 1),
        false => None,
    }
}

/// Append a list item at `level`, creating or nesting lists as needed.
///
/// Every format states a list as a flat run of items each carrying a depth, so
/// every reader has to rebuild the nesting the same way: a deeper item goes
/// inside the item above it, and a change of kind starts a new list rather than
/// continuing the last one.
///
/// `start` is the number the source gave, and is used only where this call
/// creates a new ordered list; a bulleted one has nothing to count.
pub(crate) fn append_list_item(
    blocks: &mut Vec<crate::doc::Block>,
    item: crate::doc::ListItem,
    level: u8,
    ordered: bool,
    start: u64,
) {
    use crate::doc::{Block, List};

    if level > 0
        && let Some(Block::List(list)) = blocks.last_mut()
        && let Some(parent) = list.items.last_mut()
    {
        append_list_item(&mut parent.blocks, item, level - 1, ordered, start);
        return;
    }
    // Falling through with `level > 0` is a depth with nothing above it to hang
    // from: a document that starts below the top level, or one whose first item
    // was empty. Flattening it to this level keeps the text; dropping it would
    // not.

    if let Some(Block::List(list)) = blocks.last_mut()
        && list.ordered == ordered
    {
        list.items.push(item);
        return;
    }

    blocks.push(Block::List(List {
        ordered,
        start: match ordered {
            true => start,
            false => 1,
        },
        items: vec![item],
    }));
}

/// Trim a sheet's empty edges and make it rectangular.
///
/// Every spreadsheet reader produces a ragged grid and has to square it before
/// it can be a table, and all three had their own copy. Two of them measured
/// the width as the longest row, which counts a trailing empty column as a
/// column: removing a hidden column at the right-hand edge left the same
/// workbook one column wider through `.xlsx` and `.xls` than through `.ods`.
/// The target of a `HYPERLINK` field instruction.
///
/// Word writes a field as an instruction and a result: the instruction names
/// what the field is, the result is the text to show. Both the .doc and the
/// .rtf state a hyperlink this way, so both readers ask the same question of
/// the same syntax.
///
/// Anything else -- a page number, a cross-reference, a date -- has a result
/// worth showing but no target, so it is read as ordinary text.
pub(crate) fn hyperlink_target(instruction: &str) -> Option<String> {
    let trimmed = instruction.trim_start();
    let rest = trimmed
        .strip_prefix("HYPERLINK")
        .or_else(|| trimmed.strip_prefix("hyperlink"))?
        .trim_start();
    // The target is quoted when it holds spaces, and bare when it does not.
    let target = match rest.strip_prefix('"') {
        Some(quoted) => quoted.split('"').next().unwrap_or_default(),
        None => rest.split_whitespace().next().unwrap_or_default(),
    };
    match target.is_empty() {
        true => None,
        false => Some(target.to_string()),
    }
}

#[cfg(test)]
mod field_tests {
    use super::hyperlink_target;

    #[test]
    fn reads_the_target_of_a_hyperlink_field() {
        assert_eq!(
            hyperlink_target(r#"HYPERLINK "https://example.invalid/a b" "#).as_deref(),
            Some("https://example.invalid/a b"),
            "quoted, and the quotes are what allow the space"
        );
        assert_eq!(
            hyperlink_target(r"HYPERLINK https://example.invalid/p \l anchor").as_deref(),
            Some("https://example.invalid/p"),
            "bare, ending at the switch that follows it"
        );
        assert_eq!(
            hyperlink_target("  hyperlink \"mailto:a@b.invalid\"").as_deref(),
            Some("mailto:a@b.invalid"),
            "leading space, and the name is not case sensitive"
        );
    }

    /// Every other field has a result worth showing but no target.
    #[test]
    fn a_field_that_is_not_a_hyperlink_has_no_target() {
        for instruction in ["PAGE", r"REF _Ref123 \h", r#"DATE \@ "d MMMM yyyy""#, ""] {
            assert_eq!(hyperlink_target(instruction), None, "{instruction}");
        }
    }

    /// A hyperlink naming nothing is not a link.
    #[test]
    fn a_hyperlink_without_a_target_is_none() {
        assert_eq!(hyperlink_target("HYPERLINK"), None);
        assert_eq!(hyperlink_target("HYPERLINK   "), None);
        assert_eq!(hyperlink_target(r#"HYPERLINK """#), None);
    }
}

pub(crate) fn square_grid(grid: &mut Vec<Vec<String>>) {
    while grid
        .last()
        .is_some_and(|row| row.iter().all(String::is_empty))
    {
        grid.pop();
    }
    // The last column holding anything, rather than the longest row.
    let width = grid
        .iter()
        .map(|row| {
            row.iter()
                .rposition(|cell| !cell.is_empty())
                .map_or(0, |at| at + 1)
        })
        .max()
        .unwrap_or(0);
    for row in grid.iter_mut() {
        row.resize(width, String::new());
    }
}

pub(crate) fn format_number(value: f64) -> String {
    if !value.is_finite() {
        return String::new();
    }
    if value.fract() == 0.0 && value.abs() < 1e15 {
        return format!("{}", value as i64);
    }
    let mut text = format!("{value}");
    if text.contains('.') {
        text = text.trim_end_matches('0').trim_end_matches('.').to_string();
    }
    text
}

/// The same, from stored text. Text that is not a number is returned unchanged,
/// which is what keeps an error code like `#DIV/0!` intact.
pub(crate) fn format_number_text(text: &str) -> String {
    match text.parse::<f64>() {
        Ok(value) => format_number(value),
        Err(_) => text.to_string(),
    }
}

/// Render a spreadsheet date serial as an ISO 8601 date, or date and time.
///
/// Excel counts days from 1899-12-30. The epoch looks wrong by two days and is
/// not: the format deliberately reproduces a 1900 leap year that never
/// happened, so serial 60 is a date that does not exist and everything after it
/// is shifted. Counting from the 30th absorbs both.
///
/// Without this a ledger reports `46095` where the cell reads `2026-03-14`.
/// The number is the truth of the file and not the answer to the question, and
/// the OpenDocument reader of the same workbook said the date all along —
/// ODF stores a formatted value beside the number, and the two OOXML formats
/// store only a number and a format to apply to it.
pub(crate) fn format_date_serial(serial: f64) -> Option<String> {
    if !serial.is_finite() || !(0.0..2_958_466.0).contains(&serial) {
        return None;
    }
    let fraction = serial - serial.trunc();
    // Serial 60 is 1900-02-29, a date that never happened: the format keeps it
    // so that files written by its predecessors still agree. Everything from
    // 61 on is therefore one day ahead of a real calendar counted from
    // 1899-12-30, and everything below 60 is not -- so the two halves need
    // different epochs. Using one gives the right answer for every modern date
    // and is a day out for January and February 1900.
    let days = match serial < 60.0 {
        true => serial.trunc() as i64 + 1,
        false => serial.trunc() as i64,
    };

    // Days from 1899-12-30 to the civil date, by Howard Hinnant's algorithm.
    // 719_468 shifts a 1970 epoch to a 0000-03-01 era; 25_569 is 1899-12-30 in
    // days before 1970.
    let z = days - 25_569 + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };

    // A whole day is a date; a fraction of one carries a time.
    if fraction.abs() < 1e-9 {
        return Some(format!("{y:04}-{m:02}-{d:02}"));
    }
    let seconds = (fraction * 86_400.0).round() as i64;
    let (hour, minute, second) = (seconds / 3600, (seconds / 60) % 60, seconds % 60);
    Some(format!(
        "{y:04}-{m:02}-{d:02} {hour:02}:{minute:02}:{second:02}"
    ))
}

/// Whether a spreadsheet number format shows a date or a time.
///
/// The built-in identifiers are fixed by the format; anything at 164 or above
/// is a custom code, and the code itself has to be read. Quoted literals and
/// colour or condition brackets are skipped so that a currency format spelling
/// out "[Red]" or a month name in quotes is not mistaken for a date.
pub(crate) fn is_date_format(id: u32, code: Option<&str>) -> bool {
    if (14..=22).contains(&id) || (45..=47).contains(&id) || (27..=36).contains(&id) {
        return true;
    }
    let Some(code) = code else {
        return false;
    };
    let mut in_quote = false;
    let mut in_bracket = false;
    let mut previous = ' ';
    for ch in code.chars() {
        match ch {
            '"' => in_quote = !in_quote,
            '[' => in_bracket = true,
            ']' => in_bracket = false,
            _ if in_quote || in_bracket => {}
            // An escaped character is a literal, not a token.
            _ if previous == '\\' => {}
            'y' | 'Y' | 'd' | 'D' => return true,
            // `h` implies a time, and `m` after `h` is minutes rather than a
            // month -- but either way the cell shows a moment, not a number.
            'h' | 'H' | 's' | 'S' => return true,
            _ => {}
        }
        previous = ch;
    }
    false
}

#[cfg(test)]
mod date_tests {
    use super::{format_date_serial, is_date_format};

    /// The serials a real workbook produced, against the dates its cells show.
    ///
    /// Excel counts from 1899-12-30 rather than the 31st because it reproduces
    /// a 1900 leap year that never happened. Getting the epoch wrong by a day
    /// is invisible on any single value and wrong on all of them, so these come
    /// from a workbook Excel wrote rather than from arithmetic.
    #[test]
    fn a_serial_becomes_the_date_the_cell_shows() {
        assert_eq!(format_date_serial(46095.0).as_deref(), Some("2026-03-14"));
        assert_eq!(format_date_serial(46203.0).as_deref(), Some("2026-06-30"));
        // The epoch itself, and the first day anyone writes.
        assert_eq!(format_date_serial(1.0).as_deref(), Some("1900-01-01"));
        // 1900-02-28 is serial 59; 60 is the day that never existed, and 61 is
        // the 1st of March. The shift is why the epoch is the 30th.
        assert_eq!(format_date_serial(59.0).as_deref(), Some("1900-02-28"));
        assert_eq!(format_date_serial(61.0).as_deref(), Some("1900-03-01"));
        // A leap day that did happen, and the day after it.
        assert_eq!(format_date_serial(45351.0).as_deref(), Some("2024-02-29"));
        assert_eq!(format_date_serial(45352.0).as_deref(), Some("2024-03-01"));
    }

    /// A fraction of a day is a time of day.
    #[test]
    fn a_fractional_serial_carries_the_time() {
        assert_eq!(
            format_date_serial(46095.5).as_deref(),
            Some("2026-03-14 12:00:00")
        );
    }

    /// Out of range returns nothing rather than a wrong date.
    #[test]
    fn an_impossible_serial_is_declined() {
        assert!(format_date_serial(-1.0).is_none());
        assert!(format_date_serial(f64::NAN).is_none());
        assert!(format_date_serial(1e12).is_none());
    }

    /// Built-in identifiers are fixed; custom codes have to be read.
    #[test]
    fn a_format_is_a_date_when_its_code_says_so() {
        assert!(is_date_format(14, None), "built-in short date");
        assert!(is_date_format(22, None), "built-in date and time");
        assert!(is_date_format(165, Some("yyyy-mm-dd")));
        assert!(is_date_format(166, Some("d mmm yyyy")));
        assert!(is_date_format(167, Some("hh:mm:ss")));

        assert!(!is_date_format(0, None), "general");
        assert!(!is_date_format(4, None), "#,##0.00");
        assert!(!is_date_format(164, Some("#,##0.00")));
        assert!(!is_date_format(168, Some("0.0%")));
        // A month name in a literal is text, not a token; and a colour or
        // condition in brackets is neither.
        assert!(!is_date_format(169, Some(r#""May" #,##0"#)));
        assert!(!is_date_format(170, Some("[Red]-#,##0.00")));
    }
}
