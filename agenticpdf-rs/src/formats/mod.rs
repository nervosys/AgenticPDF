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
