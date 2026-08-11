// SPDX-License-Identifier: AGPL-3.0-or-later
//! Minimal read-only ZIP archive reader.
//!
//! OOXML (`.docx` / `.pptx` / `.xlsx`), OpenDocument (`.odt` / `.ods` / `.odp`)
//! and EPUB are all ZIP containers, so every one of those parsers needs to pull
//! named members out of an archive. This module provides exactly that and
//! nothing more: locate the end-of-central-directory record, walk the central
//! directory, and inflate a member on demand.
//!
//! Consistent with the rest of the crate (which hand-rolls PDF lexing, CMaps,
//! base64 and JSON-RPC), this avoids pulling in the `zip` crate — keeping the
//! dependency set at three and the WASM blob small. Decompression reuses the
//! existing `miniz_oxide` dependency: a ZIP member stored with method 8 is raw
//! DEFLATE, which is what [`miniz_oxide::inflate::decompress_to_vec_with_limit`]
//! consumes.
//!
//! Deliberately unsupported: writing, encryption, and compression methods other
//! than *stored* (0) and *deflate* (8). Real-world Office and OpenDocument
//! producers use only those two.

use crate::PdfError;

/// Largest member we will inflate (128 MB), and the cap on the whole archive's
/// uncompressed size. Bounds decompression-bomb blowup.
const MAX_ENTRY_SIZE: usize = 128 * 1024 * 1024;
const MAX_TOTAL_SIZE: u64 = 512 * 1024 * 1024;

/// Largest number of members we will index.
const MAX_ENTRIES: usize = 65_536;

/// A ZIP end-of-central-directory record is at most 22 bytes plus a 64 KB
/// comment, so scanning back this far from EOF always finds it.
const MAX_EOCD_SEARCH: usize = 22 + 0xFFFF;

const SIG_EOCD: u32 = 0x0605_4B50;
const SIG_EOCD64: u32 = 0x0606_4B50;
const SIG_EOCD64_LOCATOR: u32 = 0x0706_4B50;
const SIG_CENTRAL: u32 = 0x0201_4B50;
const SIG_LOCAL: u32 = 0x0403_4B50;

const METHOD_STORE: u16 = 0;
const METHOD_DEFLATE: u16 = 8;

/// Marker in a 32-bit field meaning "the real value is in the Zip64 extra field".
const ZIP64_SENTINEL_32: u32 = 0xFFFF_FFFF;
const ZIP64_SENTINEL_16: u16 = 0xFFFF;

/// One member of the archive, as described by its central-directory record.
#[derive(Debug, Clone)]
pub struct ZipEntry {
    /// Member path within the archive, e.g. `word/document.xml`.
    pub name: String,
    /// Compression method (0 = stored, 8 = deflate).
    pub method: u16,
    /// CRC-32 of the uncompressed data, as recorded in the archive.
    pub crc32: u32,
    pub compressed_size: u64,
    pub uncompressed_size: u64,
    /// Byte offset of this member's *local* file header.
    pub local_header_offset: u64,
}

/// A parsed ZIP archive borrowing the caller's bytes.
///
/// Construction indexes the central directory only; member data is inflated
/// lazily by [`ZipArchive::read`].
pub struct ZipArchive<'a> {
    data: &'a [u8],
    entries: Vec<ZipEntry>,
}

impl<'a> ZipArchive<'a> {
    /// Index an archive's central directory.
    pub fn open(data: &'a [u8]) -> Result<Self, PdfError> {
        let eocd = find_eocd(data)?;
        let (cd_offset, cd_entries) = central_directory_location(data, eocd)?;

        if cd_entries > MAX_ENTRIES {
            return Err(PdfError::ResourceLimit(format!(
                "zip has {cd_entries} entries (limit {MAX_ENTRIES})"
            )));
        }

        let mut entries = Vec::with_capacity(cd_entries.min(1024));
        let mut pos = cd_offset;
        let mut total_uncompressed: u64 = 0;

        for _ in 0..cd_entries {
            let (entry, next) = parse_central_record(data, pos)?;
            total_uncompressed = total_uncompressed.saturating_add(entry.uncompressed_size);
            if total_uncompressed > MAX_TOTAL_SIZE {
                return Err(PdfError::ResourceLimit(
                    "zip uncompressed size exceeds limit".into(),
                ));
            }
            entries.push(entry);
            pos = next;
        }

        Ok(ZipArchive { data, entries })
    }

    /// All member paths, in central-directory order.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.entries.iter().map(|e| e.name.as_str())
    }

    /// Indexed members.
    pub fn entries(&self) -> &[ZipEntry] {
        &self.entries
    }

    /// Whether a member exists.
    pub fn contains(&self, name: &str) -> bool {
        self.find(name).is_some()
    }

    /// Look up a member by exact path.
    pub fn find(&self, name: &str) -> Option<&ZipEntry> {
        self.entries.iter().find(|e| e.name == name)
    }

    /// Read and decompress a member by path.
    pub fn read(&self, name: &str) -> Result<Vec<u8>, PdfError> {
        let entry = self
            .find(name)
            .ok_or_else(|| PdfError::MissingPart(name.to_string()))?;
        self.read_entry(entry)
    }

    /// Read a member as UTF-8 text, stripping a leading byte-order mark.
    ///
    /// Invalid sequences are replaced rather than rejected: a malformed byte in
    /// one paragraph should not fail the whole document.
    pub fn read_to_string(&self, name: &str) -> Result<String, PdfError> {
        let bytes = self.read(name)?;
        Ok(decode_utf8_lossy(&bytes))
    }

    /// Read a member if present, returning `None` rather than erroring when it
    /// is absent. Useful for the many optional OOXML/ODF parts.
    pub fn read_optional(&self, name: &str) -> Option<Vec<u8>> {
        self.find(name).and_then(|e| self.read_entry(e).ok())
    }

    /// Read and decompress a specific member.
    pub fn read_entry(&self, entry: &ZipEntry) -> Result<Vec<u8>, PdfError> {
        if entry.uncompressed_size as usize > MAX_ENTRY_SIZE {
            return Err(PdfError::ResourceLimit(format!(
                "zip member {} is {} bytes (limit {MAX_ENTRY_SIZE})",
                entry.name, entry.uncompressed_size
            )));
        }

        let start = self.data_offset(entry)?;
        let end = start
            .checked_add(entry.compressed_size as usize)
            .filter(|&e| e <= self.data.len())
            .ok_or_else(|| {
                PdfError::Malformed(format!("member {} runs past end of file", entry.name))
            })?;
        let raw = &self.data[start..end];

        let out = match entry.method {
            METHOD_STORE => raw.to_vec(),
            METHOD_DEFLATE => {
                // A ZIP deflate member is a bare DEFLATE stream (no zlib header).
                // Cap at the declared size when we have one, so a lying header
                // cannot make us allocate without bound.
                let limit = if entry.uncompressed_size > 0 {
                    (entry.uncompressed_size as usize).min(MAX_ENTRY_SIZE)
                } else {
                    MAX_ENTRY_SIZE
                };
                miniz_oxide::inflate::decompress_to_vec_with_limit(raw, limit).map_err(|e| {
                    PdfError::DecompressError(format!("zip member {}: {:?}", entry.name, e.status))
                })?
            }
            other => {
                return Err(PdfError::Unsupported(format!(
                    "zip compression method {other} for member {}",
                    entry.name
                )));
            }
        };

        // The CRC is the archive's own integrity claim; a mismatch means the
        // bytes are not what the producer wrote. Entries with a zero CRC and
        // zero length (directories, empty files) are exempt.
        if entry.crc32 != 0 && crc32(&out) != entry.crc32 {
            return Err(PdfError::Malformed(format!(
                "crc32 mismatch for zip member {}",
                entry.name
            )));
        }

        Ok(out)
    }

    /// Resolve where a member's data actually begins.
    ///
    /// The local header repeats the name and extra fields but their lengths may
    /// differ from the central directory's copy, so the offset must be read from
    /// the local header rather than computed from central-directory values.
    fn data_offset(&self, entry: &ZipEntry) -> Result<usize, PdfError> {
        let at = entry.local_header_offset as usize;
        let header = self
            .data
            .get(at..at + 30)
            .ok_or_else(|| PdfError::Malformed(format!("bad local header for {}", entry.name)))?;

        if read_u32(header, 0) != SIG_LOCAL {
            return Err(PdfError::Malformed(format!(
                "missing local header signature for {}",
                entry.name
            )));
        }

        let name_len = read_u16(header, 26) as usize;
        let extra_len = read_u16(header, 28) as usize;
        Ok(at + 30 + name_len + extra_len)
    }
}

// ============================================================================
// Central directory / EOCD parsing
// ============================================================================

/// Scan backwards for the end-of-central-directory signature.
fn find_eocd(data: &[u8]) -> Result<usize, PdfError> {
    if data.len() < 22 {
        return Err(PdfError::Malformed("file too short to be a zip".into()));
    }
    let search_start = data.len().saturating_sub(MAX_EOCD_SEARCH);
    // Search from the end so a comment that happens to contain the signature
    // does not win over the real record.
    for at in (search_start..=data.len() - 22).rev() {
        if read_u32(data, at) == SIG_EOCD {
            return Ok(at);
        }
    }
    Err(PdfError::Malformed(
        "end of central directory not found".into(),
    ))
}

/// Resolve `(central directory offset, entry count)`, following the Zip64
/// records when the 32-bit fields are saturated.
fn central_directory_location(data: &[u8], eocd: usize) -> Result<(usize, usize), PdfError> {
    let entries_16 = read_u16(data, eocd + 10);
    let offset_32 = read_u32(data, eocd + 16);

    if entries_16 != ZIP64_SENTINEL_16 && offset_32 != ZIP64_SENTINEL_32 {
        let offset = offset_32 as usize;
        if offset >= data.len() {
            return Err(PdfError::Malformed(
                "central directory offset past end of file".into(),
            ));
        }
        return Ok((offset, entries_16 as usize));
    }

    // Zip64: a 20-byte locator sits immediately before the EOCD and points at
    // the Zip64 EOCD record, which carries the real 64-bit values.
    let locator = eocd
        .checked_sub(20)
        .ok_or_else(|| PdfError::Malformed("missing zip64 locator".into()))?;
    if read_u32(data, locator) != SIG_EOCD64_LOCATOR {
        return Err(PdfError::Malformed("bad zip64 locator signature".into()));
    }

    let eocd64 = read_u64(data, locator + 8) as usize;
    if eocd64 + 56 > data.len() || read_u32(data, eocd64) != SIG_EOCD64 {
        return Err(PdfError::Malformed(
            "bad zip64 end of central directory".into(),
        ));
    }

    let entries = read_u64(data, eocd64 + 32) as usize;
    let offset = read_u64(data, eocd64 + 48) as usize;
    if offset >= data.len() {
        return Err(PdfError::Malformed(
            "zip64 central directory offset past end of file".into(),
        ));
    }
    Ok((offset, entries))
}

/// Parse one central-directory record, returning it and the offset of the next.
fn parse_central_record(data: &[u8], at: usize) -> Result<(ZipEntry, usize), PdfError> {
    let header = data
        .get(at..at + 46)
        .ok_or_else(|| PdfError::Malformed("truncated central directory".into()))?;

    if read_u32(header, 0) != SIG_CENTRAL {
        return Err(PdfError::Malformed(
            "bad central directory signature".into(),
        ));
    }

    let flags = read_u16(header, 8);
    let method = read_u16(header, 10);
    let crc32 = read_u32(header, 16);
    let mut compressed_size = read_u32(header, 20) as u64;
    let mut uncompressed_size = read_u32(header, 24) as u64;
    let name_len = read_u16(header, 28) as usize;
    let extra_len = read_u16(header, 30) as usize;
    let comment_len = read_u16(header, 32) as usize;
    let mut local_header_offset = read_u32(header, 42) as u64;

    // Bit 0 of the general-purpose flags marks an encrypted member. We cannot
    // read those, and saying so beats returning garbage.
    if flags & 1 != 0 {
        return Err(PdfError::Encrypted);
    }

    let name_start = at + 46;
    let name_bytes = data
        .get(name_start..name_start + name_len)
        .ok_or_else(|| PdfError::Malformed("truncated member name".into()))?;
    // Bit 11 declares UTF-8 names; otherwise the spec says CP437. Every OOXML
    // and ODF part name is ASCII, which both encodings agree on, so a lossy
    // UTF-8 decode is correct for the formats we support.
    let name = decode_utf8_lossy(name_bytes);

    let extra_start = name_start + name_len;
    let extra = data
        .get(extra_start..extra_start + extra_len)
        .ok_or_else(|| PdfError::Malformed("truncated extra field".into()))?;

    if uncompressed_size == ZIP64_SENTINEL_32 as u64
        || compressed_size == ZIP64_SENTINEL_32 as u64
        || local_header_offset == ZIP64_SENTINEL_32 as u64
    {
        apply_zip64_extra(
            extra,
            &mut uncompressed_size,
            &mut compressed_size,
            &mut local_header_offset,
        );
    }

    let next = extra_start + extra_len + comment_len;
    Ok((
        ZipEntry {
            name,
            method,
            crc32,
            compressed_size,
            uncompressed_size,
            local_header_offset,
        },
        next,
    ))
}

/// Pull 64-bit sizes/offsets out of the Zip64 extended information extra field
/// (header id 0x0001). Fields appear in a fixed order but only for those whose
/// 32-bit counterpart was saturated.
fn apply_zip64_extra(extra: &[u8], uncompressed: &mut u64, compressed: &mut u64, offset: &mut u64) {
    let mut at = 0usize;
    while at + 4 <= extra.len() {
        let id = read_u16(extra, at);
        let size = read_u16(extra, at + 2) as usize;
        let body_start = at + 4;
        if body_start + size > extra.len() {
            return;
        }
        if id == 0x0001 {
            let body = &extra[body_start..body_start + size];
            let mut cursor = 0usize;
            let mut take = |slot: &mut u64| {
                if *slot == ZIP64_SENTINEL_32 as u64 && cursor + 8 <= body.len() {
                    *slot = read_u64(body, cursor);
                    cursor += 8;
                }
            };
            take(uncompressed);
            take(compressed);
            take(offset);
            return;
        }
        at = body_start + size;
    }
}

// ============================================================================
// Primitives
// ============================================================================

fn read_u16(data: &[u8], at: usize) -> u16 {
    match data.get(at..at + 2) {
        Some(b) => u16::from_le_bytes([b[0], b[1]]),
        None => 0,
    }
}

fn read_u32(data: &[u8], at: usize) -> u32 {
    match data.get(at..at + 4) {
        Some(b) => u32::from_le_bytes([b[0], b[1], b[2], b[3]]),
        None => 0,
    }
}

fn read_u64(data: &[u8], at: usize) -> u64 {
    match data.get(at..at + 8) {
        Some(b) => u64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]),
        None => 0,
    }
}

/// UTF-8 decode that replaces invalid sequences and strips a leading BOM.
pub(crate) fn decode_utf8_lossy(bytes: &[u8]) -> String {
    let body = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
    String::from_utf8_lossy(body).into_owned()
}

/// CRC-32 (IEEE 802.3), computed with a lazily built lookup table.
pub fn crc32(data: &[u8]) -> u32 {
    let table = crc_table();
    let mut crc = 0xFFFF_FFFFu32;
    for &byte in data {
        let idx = ((crc ^ byte as u32) & 0xFF) as usize;
        crc = table[idx] ^ (crc >> 8);
    }
    crc ^ 0xFFFF_FFFF
}

/// The CRC table is 1 KB and pure; build it once.
fn crc_table() -> &'static [u32; 256] {
    use std::sync::OnceLock;
    static TABLE: OnceLock<[u32; 256]> = OnceLock::new();
    TABLE.get_or_init(|| {
        let mut table = [0u32; 256];
        for (i, slot) in table.iter_mut().enumerate() {
            let mut c = i as u32;
            for _ in 0..8 {
                c = if c & 1 != 0 {
                    0xEDB8_8320 ^ (c >> 1)
                } else {
                    c >> 1
                };
            }
            *slot = c;
        }
        table
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::build_zip;

    #[test]
    fn reads_stored_member() {
        let zip = build_zip(&[(
            "mimetype",
            b"application/vnd.oasis.opendocument.text",
            false,
        )]);
        let archive = ZipArchive::open(&zip).unwrap();
        assert_eq!(archive.entries().len(), 1);
        assert_eq!(
            archive.read_to_string("mimetype").unwrap(),
            "application/vnd.oasis.opendocument.text"
        );
    }

    #[test]
    fn reads_deflated_member() {
        let body = "<w:document><w:body/></w:document>".repeat(50);
        let zip = build_zip(&[("word/document.xml", body.as_bytes(), true)]);
        let archive = ZipArchive::open(&zip).unwrap();
        assert_eq!(archive.read_to_string("word/document.xml").unwrap(), body);
    }

    #[test]
    fn indexes_multiple_members_in_order() {
        let zip = build_zip(&[
            ("[Content_Types].xml", b"<Types/>", true),
            ("word/document.xml", b"<w:document/>", true),
            ("word/_rels/document.xml.rels", b"<Relationships/>", false),
        ]);
        let archive = ZipArchive::open(&zip).unwrap();
        let names: Vec<&str> = archive.names().collect();
        assert_eq!(
            names,
            vec![
                "[Content_Types].xml",
                "word/document.xml",
                "word/_rels/document.xml.rels"
            ]
        );
        assert!(archive.contains("word/document.xml"));
        assert!(!archive.contains("xl/workbook.xml"));
    }

    #[test]
    fn missing_member_is_missing_part() {
        let zip = build_zip(&[("a.xml", b"<a/>", false)]);
        let archive = ZipArchive::open(&zip).unwrap();
        assert!(matches!(
            archive.read("nope.xml"),
            Err(PdfError::MissingPart(_))
        ));
        assert!(archive.read_optional("nope.xml").is_none());
    }

    #[test]
    fn detects_corrupted_payload() {
        let mut zip = build_zip(&[("a.txt", b"hello world", false)]);
        // Flip a byte inside the stored payload; the CRC must catch it.
        let at = zip.iter().position(|&b| b == b'h').unwrap();
        zip[at] = b'H';
        let archive = ZipArchive::open(&zip).unwrap();
        assert!(matches!(archive.read("a.txt"), Err(PdfError::Malformed(_))));
    }

    #[test]
    fn rejects_non_zip() {
        assert!(ZipArchive::open(b"%PDF-1.7\n").is_err());
        assert!(ZipArchive::open(b"").is_err());
    }

    #[test]
    fn strips_bom_when_decoding_text() {
        let zip = build_zip(&[("a.xml", b"\xEF\xBB\xBF<a/>", false)]);
        let archive = ZipArchive::open(&zip).unwrap();
        assert_eq!(archive.read_to_string("a.xml").unwrap(), "<a/>");
    }

    #[test]
    fn crc32_matches_known_vector() {
        // The canonical IEEE CRC-32 of "123456789".
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
        assert_eq!(crc32(b""), 0);
    }
}
