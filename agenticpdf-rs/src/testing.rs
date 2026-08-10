// SPDX-License-Identifier: AGPL-3.0-or-later
//! Fixture builders for tests.
//!
//! Every ZIP-container format this crate reads — OOXML, OpenDocument, EPUB — is
//! an archive of text parts, so a valid fixture can be assembled in a few lines
//! instead of committed as an opaque binary. That keeps the repository free of
//! blobs no one can diff, and lets a test state exactly which parts it is
//! exercising.
//!
//! This module is compiled into the library rather than gated behind `cfg(test)`
//! so the integration tests in `tests/` can use it too; nothing in the shipping
//! code paths calls it.

use crate::container::zip::crc32;

const SIG_LOCAL: u32 = 0x0403_4B50;
const SIG_CENTRAL: u32 = 0x0201_4B50;
const SIG_EOCD: u32 = 0x0605_4B50;

/// Build a ZIP archive in memory.
///
/// Each member is `(path, contents, deflate)`. Set `deflate` to false for
/// members a spec requires to be stored uncompressed — notably the `mimetype`
/// entry that identifies OpenDocument and EPUB archives.
pub fn build_zip(members: &[(&str, &[u8], bool)]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut central = Vec::new();

    for (name, body, deflate) in members {
        let offset = out.len() as u32;
        let (method, payload) = if *deflate {
            (8u16, miniz_oxide::deflate::compress_to_vec(body, 6))
        } else {
            (0u16, body.to_vec())
        };
        let crc = crc32(body);

        for chunk in [
            &SIG_LOCAL.to_le_bytes()[..],
            &20u16.to_le_bytes(),
            &0u16.to_le_bytes(),
            &method.to_le_bytes(),
            &0u16.to_le_bytes(),
            &0u16.to_le_bytes(),
            &crc.to_le_bytes(),
            &(payload.len() as u32).to_le_bytes(),
            &(body.len() as u32).to_le_bytes(),
            &(name.len() as u16).to_le_bytes(),
            &0u16.to_le_bytes(),
        ] {
            out.extend_from_slice(chunk);
        }
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(&payload);

        for chunk in [
            &SIG_CENTRAL.to_le_bytes()[..],
            &20u16.to_le_bytes(),
            &20u16.to_le_bytes(),
            &0u16.to_le_bytes(),
            &method.to_le_bytes(),
            &0u16.to_le_bytes(),
            &0u16.to_le_bytes(),
            &crc.to_le_bytes(),
            &(payload.len() as u32).to_le_bytes(),
            &(body.len() as u32).to_le_bytes(),
            &(name.len() as u16).to_le_bytes(),
            &0u16.to_le_bytes(),
            &0u16.to_le_bytes(),
            &0u16.to_le_bytes(),
            &0u16.to_le_bytes(),
            &0u32.to_le_bytes(),
            &offset.to_le_bytes(),
        ] {
            central.extend_from_slice(chunk);
        }
        central.extend_from_slice(name.as_bytes());
    }

    let cd_offset = out.len() as u32;
    let cd_size = central.len() as u32;
    let count = members.len() as u16;
    out.extend_from_slice(&central);
    for chunk in [
        &SIG_EOCD.to_le_bytes()[..],
        &0u16.to_le_bytes(),
        &0u16.to_le_bytes(),
        &count.to_le_bytes(),
        &count.to_le_bytes(),
        &cd_size.to_le_bytes(),
        &cd_offset.to_le_bytes(),
        &0u16.to_le_bytes(),
    ] {
        out.extend_from_slice(chunk);
    }
    out
}

/// A minimal but valid PNG header, enough for [`crate::doc::image_dimensions`]
/// to report the given size. The pixel data is deliberately absent — nothing in
/// this crate decodes PNG pixels outside the `ocr` feature.
pub fn png_header(width: u32, height: u32) -> Vec<u8> {
    let mut bytes = b"\x89PNG\r\n\x1a\n".to_vec();
    bytes.extend_from_slice(&[0, 0, 0, 13]);
    bytes.extend_from_slice(b"IHDR");
    bytes.extend_from_slice(&width.to_be_bytes());
    bytes.extend_from_slice(&height.to_be_bytes());
    bytes
}
