// SPDX-License-Identifier: AGPL-3.0-or-later
//! Cross-cutting tests for the legacy binary readers.

use crate::detect::Format;

use super::parse;

#[test]
fn a_non_compound_file_is_reported_as_malformed() {
    for format in [Format::Doc, Format::Xls, Format::Ppt] {
        let error = parse(b"not an OLE2 compound file at all", format).unwrap_err();
        assert!(
            matches!(&error, crate::PdfError::Malformed(m) if m.contains("OLE2")),
            "{format:?}: got {error:?}"
        );
    }
}

#[test]
fn a_non_legacy_format_is_rejected() {
    assert!(matches!(
        parse(b"", Format::Docx),
        Err(crate::PdfError::Unsupported(_))
    ));
}
