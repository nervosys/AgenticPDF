// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADF — the Agentic Document Format.
//!
//! Every other format this crate reads was designed for a word processor, a
//! printer or a browser. ADF is designed for an agent, and the difference shows
//! up in four places:
//!
//! - **Seek, don't scan.** The file opens by reading a 64-byte header and a
//!   fixed-stride chunk table. Everything else — a section, a page's geometry,
//!   an image, an embedding matrix — is found by offset. Opening a 500 MB
//!   document to answer a question about page 400 touches three chunks, so the
//!   cost of a question is the size of its answer rather than the size of the
//!   document.
//! - **The index travels with the document.** Retrieval chunks, their
//!   embeddings and a term index live *in* the file. An agent can search a
//!   document it has never seen without an embedding pass or a side database,
//!   and the index cannot drift from the content it describes because saving
//!   the content rewrites it.
//! - **Every span can be traced.** Each node carries a content hash and,
//!   for imported documents, the source file, page and bounding box it came
//!   from. A citation is checkable rather than plausible — which is the whole
//!   difference between an agent that quotes and an agent that confabulates.
//! - **Edits are an append-only log.** Agents and humans edit the same document
//!   concurrently, so the file stores operations rather than only their result.
//!   Saving appends; history and attribution — including *which model* made a
//!   change — are inherent rather than bolted on.
//!
//! # Layout
//!
//! ```text
//! ┌──────────────────────────────────────────────┐
//! │ Header — 64 bytes, fixed                     │
//! ├──────────────────────────────────────────────┤
//! │ Chunk payloads, each 64-byte aligned         │
//! │   strings · sections · assets · geometry     │
//! │   embeddings · term index · provenance       │
//! ├──────────────────────────────────────────────┤
//! │ Chunk table — `chunk_count` × 32 bytes       │
//! ├──────────────────────────────────────────────┤
//! │ Op log — append-only, grows without rewrite  │
//! └──────────────────────────────────────────────┘
//! ```
//!
//! The chunk table sits after the payloads so that writing is a single forward
//! pass: payload offsets are only known once the payloads are written. The op
//! log sits last so an edit can be appended without moving anything.
//!
//! # What "zero-copy" does and does not mean here
//!
//! Chunk lookup, string access, assets, provenance rows and embeddings are
//! genuinely copy-free: they are slices into the mapped file. Block content is
//! *decoded* on access, because the semantic model is a recursive Rust enum
//! rather than a flat record, and pretending otherwise would mean either
//! `unsafe` transmutes or a model shaped for the disk instead of for the
//! program. Decoding is per-section and on demand, so the property that
//! matters — never paying for what you did not ask for — holds.

pub mod codec;
pub mod index;
pub mod oplog;
pub mod provenance;
pub mod read;
pub mod sha256;
pub mod wire;
pub mod write;

#[cfg(test)]
mod tests;

pub use read::AdfDoc;
pub use write::AdfWriter;

use crate::PdfError;

/// Magic bytes. `\x89` first, matching PNG's trick: a file transferred as text
/// loses the high bit and stops matching, so corruption is caught at open.
pub const MAGIC: [u8; 4] = [0x89, b'A', b'D', b'F'];

/// Format version written by this build.
pub const VERSION_MAJOR: u16 = 1;
/// Minor 2 added the profile word and the workbook chunks. A v1.1 reader
/// opens a v1.2 file unharmed: it ignores the reserved word the profile lives
/// in and skips the chunk kinds it does not know, which is what the forward
/// compatibility rules already promised.
pub const VERSION_MINOR: u16 = 2;

/// Size of the fixed header, in bytes.
pub const HEADER_LEN: usize = 64;

/// Size of one chunk-table entry, in bytes.
pub const ENTRY_LEN: usize = 32;

/// What a chunk holds.
///
/// Stored as a `u16`, so a reader that meets a kind from a newer writer can
/// skip it by length rather than failing — which is what makes the format
/// forward-compatible.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u16)]
pub enum ChunkKind {
    /// Document metadata: title, author, dates, source format.
    Meta = 1,
    /// The string heap every other chunk indexes into.
    Strings = 2,
    /// One section's block stream. `id` is the section index.
    Section = 3,
    /// Footnotes and endnotes.
    Footnotes = 4,
    /// One embedded asset's bytes. `id` is the asset index.
    Asset = 5,
    /// Asset directory: id, media type and dimensions for each asset.
    AssetTable = 6,
    /// Provenance rows, fixed stride.
    Provenance = 7,
    /// Retrieval chunk table: text spans an agent can cite.
    Retrieval = 8,
    /// Embedding matrix, one row per retrieval chunk.
    Embeddings = 9,
    /// Inverted term index over the retrieval chunks.
    TermIndex = 10,
    /// One typeset page's display list. `id` is the page index.
    Geometry = 11,
    /// A spreadsheet workbook directory: sheet names and defined names.
    Workbook = 12,
    /// One worksheet's cells. `id` is the sheet index.
    Sheet = 13,
    /// Unrecognised — carried through untouched so that round-tripping a file
    /// written by a newer version does not silently discard its data.
    Unknown(u16),
}

impl ChunkKind {
    pub fn to_u16(self) -> u16 {
        match self {
            ChunkKind::Meta => 1,
            ChunkKind::Strings => 2,
            ChunkKind::Section => 3,
            ChunkKind::Footnotes => 4,
            ChunkKind::Asset => 5,
            ChunkKind::AssetTable => 6,
            ChunkKind::Provenance => 7,
            ChunkKind::Retrieval => 8,
            ChunkKind::Embeddings => 9,
            ChunkKind::TermIndex => 10,
            ChunkKind::Geometry => 11,
            ChunkKind::Workbook => 12,
            ChunkKind::Sheet => 13,
            ChunkKind::Unknown(value) => value,
        }
    }

    pub fn from_u16(value: u16) -> ChunkKind {
        match value {
            1 => ChunkKind::Meta,
            2 => ChunkKind::Strings,
            3 => ChunkKind::Section,
            4 => ChunkKind::Footnotes,
            5 => ChunkKind::Asset,
            6 => ChunkKind::AssetTable,
            7 => ChunkKind::Provenance,
            8 => ChunkKind::Retrieval,
            9 => ChunkKind::Embeddings,
            10 => ChunkKind::TermIndex,
            11 => ChunkKind::Geometry,
            12 => ChunkKind::Workbook,
            13 => ChunkKind::Sheet,
            other => ChunkKind::Unknown(other),
        }
    }
}

/// Chunk flags.
pub mod chunk_flags {
    /// Payload is deflate-compressed; `raw_len` gives its inflated size.
    pub const COMPRESSED: u16 = 1 << 0;
}

/// One chunk-table entry: 32 bytes, fixed stride so the table is an array.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChunkEntry {
    pub kind: ChunkKind,
    pub flags: u16,
    /// Ordinal within the kind — the section number, the page number, the
    /// asset index. Zero for kinds that occur once.
    pub id: u32,
    pub offset: u64,
    /// Bytes stored on disk.
    pub len: u32,
    /// Bytes after decompression; equal to `len` when not compressed.
    pub raw_len: u32,
    /// Content hash of the uncompressed payload.
    pub hash: u64,
}

impl ChunkEntry {
    pub fn is_compressed(&self) -> bool {
        self.flags & chunk_flags::COMPRESSED != 0
    }

    pub fn write(&self, out: &mut wire::Writer) {
        out.u16(self.kind.to_u16());
        out.u16(self.flags);
        out.u32(self.id);
        out.u64(self.offset);
        out.u32(self.len);
        out.u32(self.raw_len);
        out.u64(self.hash);
    }

    pub fn read(reader: &mut wire::Reader<'_>) -> Result<ChunkEntry, AdfError> {
        Ok(ChunkEntry {
            kind: ChunkKind::from_u16(reader.u16()?),
            flags: reader.u16()?,
            id: reader.u32()?,
            offset: reader.u64()?,
            len: reader.u32()?,
            raw_len: reader.u32()?,
            hash: reader.u64()?,
        })
    }
}

/// What kind of document an ADF file holds.
///
/// One container, several document types. The chunk kinds a file uses follow
/// from its profile, so a reader can tell a workbook from a report by looking
/// at the header rather than by opening chunks and guessing from what it
/// finds.
///
/// The value lives in the word the v1.1 header reserved, which has two
/// consequences worth stating plainly. A file written before profiles existed
/// reads back as `Document` — which is what it was. And the word sits outside
/// the header checksum, so a profile corrupted in transit is not detected;
/// that is the price of not invalidating every file already written, and it
/// costs a mis-chosen decoder rather than a bad offset.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Profile {
    /// Prose: sections of blocks. The original ADF, and still the default.
    #[default]
    Document,
    /// A workbook: sheets of typed cells, formulas kept alongside their
    /// cached results.
    Spreadsheet,
    /// Written by a build that knows a profile this one does not. The chunks
    /// this reader does recognise still read, so meeting a future profile
    /// degrades to "what I understand of it" rather than to an error.
    Other(u32),
}

impl Profile {
    pub fn to_u32(self) -> u32 {
        match self {
            Profile::Document => 0,
            Profile::Spreadsheet => 1,
            Profile::Other(value) => value,
        }
    }

    pub fn from_u32(value: u32) -> Profile {
        match value {
            0 => Profile::Document,
            1 => Profile::Spreadsheet,
            other => Profile::Other(other),
        }
    }

    /// The name used on the command line and in JSON output.
    pub fn name(self) -> &'static str {
        match self {
            Profile::Document => "document",
            Profile::Spreadsheet => "spreadsheet",
            Profile::Other(_) => "unknown",
        }
    }
}

/// The fixed 64-byte header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Header {
    pub version_major: u16,
    pub version_minor: u16,
    pub flags: u32,
    pub chunk_count: u32,
    pub chunk_table_offset: u64,
    pub oplog_offset: u64,
    pub oplog_len: u64,
    /// Stable identity for this document, preserved across saves. Actors in the
    /// op log are scoped to it.
    pub doc_id: [u8; 16],
    /// Which kind of document this file holds. See [`Profile`].
    pub profile: Profile,
}

impl Header {
    pub fn write(&self, out: &mut wire::Writer) {
        let start = out.len();
        out.raw(&MAGIC);
        out.u16(self.version_major);
        out.u16(self.version_minor);
        out.u32(self.flags);
        out.u32(self.chunk_count);
        out.u64(self.chunk_table_offset);
        out.u64(self.oplog_offset);
        out.u64(self.oplog_len);
        out.raw(&self.doc_id);
        // Checksum of everything above, so a header damaged in transit is
        // caught at open rather than misread as valid offsets.
        let checksum = sha256::sha256_u64(&out.bytes[start..]) as u32;
        out.u32(checksum);
        // The word v1.1 reserved. It follows the checksum rather than
        // preceding it so that the bytes a v1.1 reader checksums are
        // unchanged, which is what lets it open a v1.2 file at all.
        out.u32(self.profile.to_u32());
        debug_assert_eq!(out.len() - start, HEADER_LEN);
    }

    pub fn read(bytes: &[u8]) -> Result<Header, AdfError> {
        // Magic before length. A short file that is not ADF at all should say
        // so; reporting it as truncated would send a caller looking for a
        // damaged ADF document instead of a `.pdf` handed to the wrong reader.
        if !bytes.starts_with(&MAGIC) {
            return Err(AdfError::NotAdf);
        }
        let head = bytes.get(..HEADER_LEN).ok_or(AdfError::Truncated)?;
        let mut reader = wire::Reader::new(head);
        reader.take(4)?;

        let mut header = Header {
            version_major: reader.u16()?,
            version_minor: reader.u16()?,
            flags: reader.u32()?,
            chunk_count: reader.u32()?,
            chunk_table_offset: reader.u64()?,
            oplog_offset: reader.u64()?,
            oplog_len: reader.u64()?,
            doc_id: {
                let mut id = [0u8; 16];
                id.copy_from_slice(reader.take(16)?);
                id
            },
            // Filled in below; it is stored after the checksum.
            profile: Profile::Document,
        };

        let expected = sha256::sha256_u64(&head[..HEADER_LEN - 8]) as u32;
        if reader.u32()? != expected {
            return Err(AdfError::Malformed("header checksum mismatch"));
        }
        header.profile = Profile::from_u32(reader.u32()?);

        // Major version is a compatibility barrier; minor is not. A v1.7 file
        // opens in a v1.0 reader with its unknown chunks skipped.
        if header.version_major != VERSION_MAJOR {
            return Err(AdfError::UnsupportedVersion(header.version_major));
        }
        Ok(header)
    }
}

/// What can go wrong reading an ADF file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdfError {
    /// The magic bytes do not match.
    NotAdf,
    /// The file ends inside a structure that should have continued.
    Truncated,
    /// Structurally invalid: a bad tag, an impossible count, a failed checksum.
    Malformed(&'static str),
    /// Written by a major version this build does not implement.
    UnsupportedVersion(u16),
    /// A chunk referenced by the table is not present.
    MissingChunk(ChunkKind),
}

impl std::fmt::Display for AdfError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AdfError::NotAdf => write!(f, "not an ADF document"),
            AdfError::Truncated => write!(f, "ADF document is truncated"),
            AdfError::Malformed(why) => write!(f, "malformed ADF document: {why}"),
            AdfError::UnsupportedVersion(major) => {
                write!(
                    f,
                    "ADF major version {major} is not supported by this build"
                )
            }
            AdfError::MissingChunk(kind) => write!(f, "ADF document has no {kind:?} chunk"),
        }
    }
}

impl std::error::Error for AdfError {}

impl From<AdfError> for PdfError {
    fn from(error: AdfError) -> PdfError {
        match error {
            AdfError::UnsupportedVersion(major) => {
                PdfError::Unsupported(format!("ADF major version {major}"))
            }
            AdfError::MissingChunk(kind) => PdfError::MissingPart(format!("{kind:?}")),
            other => PdfError::Malformed(other.to_string()),
        }
    }
}
