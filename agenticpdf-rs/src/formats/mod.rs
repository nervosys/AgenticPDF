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
