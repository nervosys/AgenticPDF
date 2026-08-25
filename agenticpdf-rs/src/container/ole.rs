// SPDX-License-Identifier: AGPL-3.0-or-later
//! OLE2 compound-file access for the legacy Office formats.
//!
//! `.doc`, `.xls` and `.ppt` are not single byte streams: each is a compound
//! file — a FAT-like filesystem in a file — holding several named streams. A
//! Word document keeps its text in `WordDocument` and its formatting tables in
//! `0Table` or `1Table`; a workbook keeps everything in `Workbook`; a
//! presentation keeps records in `PowerPoint Document`. Getting at any of them
//! means walking the container first.
//!
//! Reads are capped: a corrupt sector chain can otherwise describe a stream far
//! larger than the file that contains it.

use std::io::{Cursor, Read};

use crate::PdfError;

/// Largest stream this reader will materialise (128 MB), matching the ZIP
/// reader's per-entry cap.
const MAX_STREAM_BYTES: u64 = 128 * 1024 * 1024;

/// An open compound file.
pub struct Ole2<'a> {
    inner: cfb::CompoundFile<Cursor<&'a [u8]>>,
}

impl<'a> Ole2<'a> {
    /// Open a compound file over the caller's bytes.
    pub fn open(data: &'a [u8]) -> Result<Ole2<'a>, PdfError> {
        let inner = cfb::CompoundFile::open(Cursor::new(data))
            .map_err(|e| PdfError::Malformed(format!("not an OLE2 compound file: {e}")))?;
        Ok(Ole2 { inner })
    }

    /// Read a named stream in full.
    pub fn read(&mut self, name: &str) -> Result<Vec<u8>, PdfError> {
        let stream = self
            .inner
            .open_stream(name)
            .map_err(|_| PdfError::MissingPart(name.to_string()))?;

        let mut bytes = Vec::new();
        let read = stream
            .take(MAX_STREAM_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| PdfError::Malformed(format!("unreadable stream {name}: {e}")))?
            as u64;
        if read > MAX_STREAM_BYTES {
            return Err(PdfError::ResourceLimit(format!(
                "{name} stream exceeds {MAX_STREAM_BYTES} bytes"
            )));
        }
        Ok(bytes)
    }

    /// Read a named stream, or `None` if it is absent.
    pub fn read_optional(&mut self, name: &str) -> Option<Vec<u8>> {
        self.read(name).ok()
    }

    /// Read the first of `names` that exists.
    ///
    /// Word picks between `0Table` and `1Table` by a flag, and a damaged file
    /// may disagree with itself about which; trying both is what makes those
    /// still readable.
    pub fn read_any(&mut self, names: &[&str]) -> Option<Vec<u8>> {
        names.iter().find_map(|name| self.read_optional(name))
    }
}

// ============================================================================
// Little-endian primitives
// ============================================================================
//
// Every legacy Office structure is a packed little-endian record read at
// computed offsets, most of them derived from other fields in the same file.
// These accessors return `None` rather than panicking so a malformed offset
// degrades to a missing property instead of taking the process down.

/// Little-endian `u16` at `offset`.
pub fn u16_at(bytes: &[u8], offset: usize) -> Option<u16> {
    let slice = bytes.get(offset..)?.get(..2)?;
    Some(u16::from_le_bytes([slice[0], slice[1]]))
}

/// Little-endian `u32` at `offset`.
pub fn u32_at(bytes: &[u8], offset: usize) -> Option<u32> {
    let slice = bytes.get(offset..)?.get(..4)?;
    Some(u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

/// Little-endian `i16` at `offset`.
pub fn i16_at(bytes: &[u8], offset: usize) -> Option<i16> {
    u16_at(bytes, offset).map(|value| value as i16)
}

// ============================================================================
// Code pages
// ============================================================================

/// The ANSI code page a Word language identifier implies.
///
/// Text in these formats predates Unicode: an 8-bit run means whatever the
/// document's language says it means, and decoding Cyrillic as Latin-1 produces
/// confident mojibake rather than an error. The mapping is by MS-LCID primary
/// language, except Chinese, where the region decides Simplified versus
/// Traditional.
pub fn encoding_for_lid(lid: u16) -> &'static encoding_rs::Encoding {
    use encoding_rs::*;
    match lid & 0x03FF {
        0x11 => SHIFT_JIS, // Japanese, 932
        0x12 => EUC_KR,    // Korean, 949
        0x04 => match lid {
            0x0404 | 0x0C04 | 0x1404 | 0x7C04 => BIG5, // Traditional, 950
            _ => GBK,                                  // Simplified, 936
        },
        0x01 | 0x20 | 0x29 => WINDOWS_1256, // Arabic script
        0x02 | 0x19 | 0x22 | 0x23 => WINDOWS_1251, // Cyrillic
        0x05 | 0x0E | 0x15 | 0x18 | 0x1A | 0x1B | 0x24 => WINDOWS_1250, // Central European
        0x08 => WINDOWS_1253,               // Greek
        0x0D => WINDOWS_1255,               // Hebrew
        0x1E => WINDOWS_874,                // Thai
        0x1F | 0x2C => WINDOWS_1254,        // Turkic
        0x25..=0x27 => WINDOWS_1257,        // Baltic
        0x2A => WINDOWS_1258,               // Vietnamese
        _ => WINDOWS_1252,                  // Western European
    }
}

/// Whether `byte` begins a two-byte sequence in a double-byte code page.
///
/// The double-byte code pages are variable width, so an 8-bit run cannot be
/// decoded one byte at a time — and character positions in these formats are
/// counted in bytes, so the lead byte's position is what the format's tables
/// refer to.
pub fn is_lead_byte(encoding: &'static encoding_rs::Encoding, byte: u8) -> bool {
    use encoding_rs::*;
    if encoding == SHIFT_JIS {
        matches!(byte, 0x81..=0x9F | 0xE0..=0xFC)
    } else if encoding == GBK || encoding == BIG5 || encoding == EUC_KR {
        (0x81..=0xFE).contains(&byte)
    } else {
        false
    }
}

/// Decode a UTF-16LE run into a string, replacing unpaired surrogates.
pub fn decode_utf16le(bytes: &[u8]) -> String {
    let units: Vec<u16> = bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u16::from_le_bytes(*pair))
        .collect();
    char::decode_utf16(units)
        .map(|result| result.unwrap_or('\u{FFFD}'))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integer_reads_reject_out_of_bounds_offsets() {
        let bytes = [1u8, 2, 3, 4];
        assert_eq!(u16_at(&bytes, 0), Some(0x0201));
        assert_eq!(u32_at(&bytes, 0), Some(0x0403_0201));
        // An offset derived from a corrupt field must not panic.
        assert_eq!(u16_at(&bytes, usize::MAX), None);
        assert_eq!(u32_at(&bytes, usize::MAX), None);
        assert_eq!(u16_at(&bytes, 3), None, "straddles the end");
    }

    #[test]
    fn opening_a_non_compound_file_is_malformed() {
        assert!(matches!(
            Ole2::open(b"%PDF-1.7\nnot a compound file"),
            Err(PdfError::Malformed(_))
        ));
    }

    #[test]
    fn maps_language_identifiers_to_code_pages() {
        use encoding_rs::*;
        assert_eq!(encoding_for_lid(0x0409), WINDOWS_1252, "US English");
        assert_eq!(encoding_for_lid(0x0419), WINDOWS_1251, "Russian");
        assert_eq!(encoding_for_lid(0x0411), SHIFT_JIS, "Japanese");
        assert_eq!(encoding_for_lid(0x0412), EUC_KR, "Korean");
        // Chinese needs the region, not just the primary language.
        assert_eq!(encoding_for_lid(0x0804), GBK, "Simplified");
        assert_eq!(encoding_for_lid(0x0404), BIG5, "Traditional");
        assert_eq!(encoding_for_lid(0x040D), WINDOWS_1255, "Hebrew");
        assert_eq!(encoding_for_lid(0x041F), WINDOWS_1254, "Turkish");
    }

    #[test]
    fn identifies_double_byte_lead_bytes() {
        use encoding_rs::*;
        assert!(is_lead_byte(SHIFT_JIS, 0x82));
        assert!(!is_lead_byte(SHIFT_JIS, 0x41), "ASCII is single-byte");
        assert!(is_lead_byte(GBK, 0xB0));
        // A single-byte code page has no lead bytes at all.
        assert!(!is_lead_byte(WINDOWS_1252, 0xE9));
    }

    #[test]
    fn decodes_utf16_including_surrogate_pairs() {
        // "hi" followed by U+1F600, which is a surrogate pair.
        let bytes = [b'h', 0, b'i', 0, 0x3D, 0xD8, 0x00, 0xDE];
        assert_eq!(decode_utf16le(&bytes), "hi\u{1F600}");
        // An unpaired surrogate becomes the replacement character.
        assert_eq!(decode_utf16le(&[0x00, 0xD8]), "\u{FFFD}");
    }
}
