// SPDX-License-Identifier: AGPL-3.0-or-later
//! Provenance: where each piece of content came from, and whether it still
//! says what it said.
//!
//! An agent that cites a document is only useful if the citation can be
//! checked. A page number is not enough — documents get edited, imported and
//! re-exported, and a quote that was true of last week's file is worse than no
//! quote at all. So every provenance row carries three things: *where* the
//! content sits now (section and block), *where it came from* (source document,
//! page, bounding box) and *what it said* (a content hash). A verifier that has
//! the row and the current content can tell the difference between a quote that
//! is still accurate, one that has been edited since, and one that never
//! existed.
//!
//! Rows are a fixed 48-byte stride so the table is an array: finding the
//! provenance of block *n* is an index, not a scan, and the table can be
//! searched without decoding any content at all.

use super::AdfError;
use super::wire::{Reader, Writer};

/// Bytes per row. Fixed so the table is randomly addressable.
pub const ROW_LEN: usize = 48;

/// Where one node's content came from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Provenance {
    /// Section holding the node.
    pub section: u32,
    /// Index of the block within that section's top-level block list.
    pub block: u32,
    /// String id of the source document's identifier — a path, URL or hash.
    /// `u32::MAX` means the content was authored here rather than imported.
    pub source: u32,
    /// 1-based page in the source document, or 0 if it had no pages.
    pub page: u32,
    /// Bounding box in the source, as `[left, bottom, right, top]` in points.
    /// All zero when the source carried no geometry — a Markdown file, say.
    pub bbox: [f32; 4],
    /// Hash of the node's text at import.
    pub hash: u64,
}

impl Provenance {
    /// The sentinel meaning "not imported from anywhere".
    pub const AUTHORED: u32 = u32::MAX;

    /// Whether this content was authored in the app rather than imported.
    pub fn is_authored(&self) -> bool {
        self.source == Provenance::AUTHORED
    }

    /// Whether the source recorded a usable bounding box.
    ///
    /// A zero box is not a location. Callers that would draw a citation
    /// highlight need to know the difference, and the standing rule in this
    /// crate is that an invented bounding box is worse than an absent one.
    pub fn has_geometry(&self) -> bool {
        self.bbox != [0.0; 4]
    }

    fn write(&self, out: &mut Writer) {
        out.u32(self.section);
        out.u32(self.block);
        out.u32(self.source);
        out.u32(self.page);
        for value in self.bbox {
            out.f32(value);
        }
        out.u64(self.hash);
        // Pad to the fixed stride. Reserved for a span offset within the block,
        // which is the obvious next refinement.
        out.u64(0);
    }

    fn read(reader: &mut Reader<'_>) -> Result<Provenance, AdfError> {
        let row = Provenance {
            section: reader.u32()?,
            block: reader.u32()?,
            source: reader.u32()?,
            page: reader.u32()?,
            bbox: [reader.f32()?, reader.f32()?, reader.f32()?, reader.f32()?],
            hash: reader.u64()?,
        };
        reader.u64()?; // reserved
        Ok(row)
    }
}

/// Encode a provenance table.
pub fn encode(rows: &[Provenance]) -> Vec<u8> {
    let mut out = Writer::new();
    for row in rows {
        row.write(&mut out);
    }
    debug_assert_eq!(out.len(), rows.len() * ROW_LEN);
    out.bytes
}

/// A provenance table borrowed from the mapped file.
#[derive(Debug, Clone, Copy, Default)]
pub struct ProvenanceTable<'a> {
    bytes: &'a [u8],
}

impl<'a> ProvenanceTable<'a> {
    pub fn new(bytes: &'a [u8]) -> ProvenanceTable<'a> {
        ProvenanceTable { bytes }
    }

    pub fn len(&self) -> usize {
        self.bytes.len() / ROW_LEN
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Row `index`, decoded on demand.
    pub fn get(&self, index: usize) -> Option<Provenance> {
        let start = index.checked_mul(ROW_LEN)?;
        let row = self.bytes.get(start..start.checked_add(ROW_LEN)?)?;
        Provenance::read(&mut Reader::new(row)).ok()
    }

    pub fn iter(&self) -> impl Iterator<Item = Provenance> + '_ {
        (0..self.len()).filter_map(move |index| self.get(index))
    }

    /// The row for a block, if one was recorded.
    pub fn find(&self, section: u32, block: u32) -> Option<Provenance> {
        self.iter()
            .find(|row| row.section == section && row.block == block)
    }

    /// Check a quotation against what the file says the content was.
    ///
    /// This is the operation the whole table exists for: given text an agent
    /// claims came from a node, say whether it matches, has drifted, or has no
    /// record at all.
    pub fn verify(&self, section: u32, block: u32, text: &str) -> Verification {
        match self.find(section, block) {
            None => Verification::Unrecorded,
            Some(row) if row.hash == super::wire::hash64(text.as_bytes()) => {
                Verification::Matches(row)
            }
            Some(row) => Verification::Drifted(row),
        }
    }
}

/// The result of checking a citation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Verification {
    /// The text is exactly what was imported.
    Matches(Provenance),
    /// The node exists and was imported, but its text has changed since.
    Drifted(Provenance),
    /// No provenance was recorded for this node.
    Unrecorded,
}

impl Verification {
    pub fn is_match(&self) -> bool {
        matches!(self, Verification::Matches(_))
    }
}
