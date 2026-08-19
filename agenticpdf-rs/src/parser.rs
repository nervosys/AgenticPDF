// SPDX-License-Identifier: AGPL-3.0-or-later
//! Stream decoding helpers and a thin parser facade.
//!
//! The full parsing pipeline lives in [`crate::engine`]. This module keeps the
//! stable `PdfParser` entry point (`new` + `parse`) and the FlateDecode /
//! PNG-predictor primitives that the engine and the WASM bindings rely on.

use crate::{PdfDocument, PdfError};
use miniz_oxide::inflate::decompress_to_vec_zlib;

/// Maximum stream length to decompress (256 MB).
const MAX_STREAM_SIZE: usize = 256 * 1024 * 1024;

/// Thin parser facade that delegates to the engine.
pub struct PdfParser<'a> {
    data: &'a [u8],
}

impl<'a> PdfParser<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data }
    }

    /// Parse the entire PDF into a [`PdfDocument`] via the engine.
    pub fn parse(&mut self) -> Result<PdfDocument, PdfError> {
        crate::engine::parse_document(self.data)
    }

    // ========================================================================
    // FlateDecode + PNG predictor
    // ========================================================================

    /// Decompress a FlateDecode stream, then apply PNG predictor un-filtering
    /// if a predictor >= 10 is specified.
    pub fn decompress_stream(
        data: &[u8],
        predictor: Option<u8>,
        columns: Option<usize>,
    ) -> Result<Vec<u8>, PdfError> {
        if data.len() > MAX_STREAM_SIZE {
            return Err(PdfError::StreamError("stream too large".into()));
        }

        let decompressed = match decompress_to_vec_zlib(data) {
            Ok(out) => out,
            Err(zlib) => match miniz_oxide::inflate::decompress_to_vec(data) {
                // Some producers omit the zlib header; try raw inflate.
                Ok(out) => out,
                Err(raw) => {
                    // Damaged part way through. Every reader keeps what came
                    // out before the fault rather than dropping the stream:
                    // half a page of content is a page with a gap in it, and
                    // none is a blank sheet. One document in the test corpus
                    // is exactly this -- a form whose entire printed template
                    // lives in a stream that stops early.
                    let salvaged = match zlib.output.len() >= raw.output.len() {
                        true => zlib.output,
                        false => raw.output,
                    };
                    if salvaged.is_empty() {
                        return Err(PdfError::DecompressError(format!("{:?}", zlib.status)));
                    }
                    salvaged
                }
            },
        };

        match predictor {
            Some(p) if p >= 10 => {
                let cols = columns.unwrap_or(1);
                Self::png_unfilter(&decompressed, cols)
            }
            _ => Ok(decompressed),
        }
    }

    /// Apply PNG un-filtering (predictors 10-14).
    fn png_unfilter(data: &[u8], columns: usize) -> Result<Vec<u8>, PdfError> {
        let row_bytes = columns + 1; // +1 for the per-row filter byte
        if row_bytes == 0 || !data.len().is_multiple_of(row_bytes) {
            return Err(PdfError::DecompressError(
                "data length not aligned to row size".into(),
            ));
        }

        let num_rows = data.len() / row_bytes;
        let mut output = Vec::with_capacity(num_rows * columns);
        let mut prev_row = vec![0u8; columns];

        for row_idx in 0..num_rows {
            let row_start = row_idx * row_bytes;
            let filter_type = data[row_start];
            let row_data = &data[row_start + 1..row_start + row_bytes];

            let mut current_row = vec![0u8; columns];
            for i in 0..columns {
                let raw = row_data[i];
                let a = if i > 0 { current_row[i - 1] } else { 0 };
                let b = prev_row[i];
                let c = if i > 0 { prev_row[i - 1] } else { 0 };

                current_row[i] = match filter_type {
                    0 => raw,
                    1 => raw.wrapping_add(a),
                    2 => raw.wrapping_add(b),
                    3 => raw.wrapping_add(((a as u16 + b as u16) / 2) as u8),
                    4 => raw.wrapping_add(Self::paeth(a, b, c)),
                    _ => raw,
                };
            }

            output.extend_from_slice(&current_row);
            prev_row = current_row;
        }

        Ok(output)
    }

    /// Paeth predictor function.
    fn paeth(a: u8, b: u8, c: u8) -> u8 {
        let p = a as i16 + b as i16 - c as i16;
        let pa = (p - a as i16).unsigned_abs();
        let pb = (p - b as i16).unsigned_abs();
        let pc = (p - c as i16).unsigned_abs();

        if pa <= pb && pa <= pc {
            a
        } else if pb <= pc {
            b
        } else {
            c
        }
    }
}

#[cfg(test)]
mod tests {

    /// A Flate stream that stops part way keeps what it produced.
    ///
    /// Dropping it instead loses whole pages: one document in the corpus
    /// keeps its entire printed form template in a stream that ends early,
    /// and without the salvage the page renders as a handful of signatures
    /// floating on white.
    #[test]
    fn a_truncated_flate_stream_keeps_what_it_had() {
        let body = b"BT /F1 12 Tf 10 10 Td (recoverable) Tj ET 0 0 100 100 re f";
        let whole = miniz_oxide::deflate::compress_to_vec_zlib(body, 6);
        // Cut the tail off, as a damaged or truncated file has.
        let cut = &whole[..whole.len() * 3 / 4];
        let out = PdfParser::decompress_stream(cut, None, None);
        let out = out.expect("a truncated stream should still yield its start");
        assert!(!out.is_empty(), "something should survive");
        assert!(
            body.starts_with(&out[..out.len().min(16)]),
            "what survives should be the start of the stream"
        );
    }

    /// Nothing recoverable is still an error, not an empty success.
    #[test]
    fn rubbish_is_still_an_error() {
        assert!(PdfParser::decompress_stream(b"not deflate at all", None, None).is_err());
    }
    use super::*;

    #[test]
    fn test_parse_minimal_pdf() {
        let pdf_bytes = b"%PDF-1.4\n\
            1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
            2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
            3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] >>\nendobj\n\
            xref\n0 4\n\
            0000000000 65535 f \n\
            0000000009 00000 n \n\
            0000000058 00000 n \n\
            0000000115 00000 n \n\
            trailer\n<< /Size 4 /Root 1 0 R >>\n\
            startxref\n195\n%%EOF";

        let mut parser = PdfParser::new(pdf_bytes);
        let result = parser.parse();
        assert!(result.is_ok());

        let doc = result.unwrap();
        assert_eq!(doc.version, "1.4");
        assert!(!doc.pages.is_empty());
    }

    #[test]
    fn test_invalid_header() {
        let result = PdfParser::new(b"NOT A PDF").parse();
        assert!(result.is_err());
    }

    #[test]
    fn test_decompress_stream() {
        let empty = miniz_oxide::deflate::compress_to_vec_zlib(&[], 6);
        let result = PdfParser::decompress_stream(&empty, None, None);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_png_unfilter_none() {
        let data = vec![0, 10, 20, 30, 0, 40, 50, 60]; // 2 rows × 3 cols, filter=0
        let result = PdfParser::png_unfilter(&data, 3).unwrap();
        assert_eq!(result, vec![10, 20, 30, 40, 50, 60]);
    }

    #[test]
    fn test_paeth() {
        assert_eq!(PdfParser::paeth(0, 0, 0), 0);
        assert_eq!(PdfParser::paeth(10, 20, 5), 20);
    }

    #[test]
    fn test_max_stream_size() {
        let huge = vec![0u8; MAX_STREAM_SIZE + 1];
        let result = PdfParser::decompress_stream(&huge, None, None);
        assert!(result.is_err());
    }
}
