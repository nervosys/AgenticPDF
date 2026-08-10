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
