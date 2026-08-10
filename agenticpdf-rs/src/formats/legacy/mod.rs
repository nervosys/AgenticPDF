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
