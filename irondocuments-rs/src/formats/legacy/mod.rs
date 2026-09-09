// SPDX-License-Identifier: AGPL-3.0-or-later
//
// The `.doc` and `.ppt` readers are derived from anydoc
// (https://github.com/firecrawl/anydoc), MIT licensed, Copyright (c) 2026
// Sideguide Technologies Inc. See LICENSE-MIT-anydoc.txt.
//
//! Legacy binary Office formats (`.doc`, `.xls`, `.ppt`).
//!
//! These predate both XML and Unicode. Each is an OLE2 compound file (see
//! [`crate::container::ole`]) holding packed binary records, with text in a
//! code page chosen by the document's language and structure expressed through
//! offsets rather than nesting.
//!
//! Because the formats share nothing but their container, each reader is
//! separate. What they do share is [`sprm`] and [`stsh`] — Word's property and
//! style machinery, which PowerPoint borrows nothing of.

pub mod doc;
pub mod ppt;
pub mod sprm;

/// The bytes an OfficeArt `BLIP` record holds, and the type they are.
///
/// The record opens with one or two 16-byte identifiers -- the odd instance
/// values are the ones that write it twice -- and then a tag byte, before the
/// file itself. Only the forms a reader can hand on: a metafile is a drawing
/// to be executed rather than an image, and the two formats that embed one
/// carry a larger header this reader has no use for either way.
pub(crate) fn blip_payload(kind: u16, instance: u16, body: &[u8]) -> Option<(&'static str, &[u8])> {
    let media = match kind {
        0xF01D | 0xF018 => "image/jpeg",
        0xF01E => "image/png",
        0xF01F => "image/bmp",
        _ => return None,
    };
    let identifiers = match instance & 1 == 1 {
        true => 2,
        false => 1,
    };
    Some((media, body.get(identifiers * 16 + 1..)?))
}
pub mod stsh;
pub mod xls;

#[cfg(test)]
mod tests;

use crate::PdfError;
use crate::detect::Format;
use crate::doc::SemanticDoc;

/// Parse a legacy binary Office document.
pub fn parse(data: &[u8], format: Format) -> Result<SemanticDoc, PdfError> {
    match format {
        Format::Doc => doc::parse(data),
        Format::Xls => xls::parse(data),
        Format::Ppt => ppt::parse(data),
        other => Err(PdfError::Unsupported(format!(
            "{} is not a legacy binary Office format",
            other.label()
        ))),
    }
}
