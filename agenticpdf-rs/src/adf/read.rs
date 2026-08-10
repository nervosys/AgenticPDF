// SPDX-License-Identifier: AGPL-3.0-or-later
//! Reading ADF files.
//!
//! [`AdfDoc`] borrows the file rather than owning it, so the caller decides
//! whether the bytes came from a read, a memory map or a network buffer. Opening
//! parses the header and the chunk table and nothing else — for a large
//! document that is a few kilobytes regardless of its size — and every accessor
//! decodes only the chunk it was asked for.

use std::collections::HashMap;

use crate::doc::{Asset, Footnote, Section, SemanticDoc};

use super::codec::{self, Meta, StringHeap};
use super::index::{Embeddings, RetrievalChunk, TermIndex};
use super::oplog::OpLog;
use super::provenance::ProvenanceTable;
use super::wire::Reader;
use super::{AdfError, ChunkEntry, ChunkKind, ENTRY_LEN, Header};

/// A document opened from ADF bytes.
#[derive(Debug)]
pub struct AdfDoc<'a> {
    bytes: &'a [u8],
    header: Header,
    entries: Vec<ChunkEntry>,
    /// Chunks whose payload had to be inflated, kept so accessors can hand out
    /// slices with the same lifetime regardless of whether they were stored
    /// compressed.
    inflated: HashMap<(u16, u32), Vec<u8>>,
    strings_key: Option<(u16, u32)>,
}

impl<'a> AdfDoc<'a> {
    /// Whether `bytes` looks like an ADF document.
    pub fn sniff(bytes: &[u8]) -> bool {
        bytes.starts_with(&super::MAGIC)
    }

    /// Open a document, reading only the header and chunk table.
    pub fn open(bytes: &'a [u8]) -> Result<AdfDoc<'a>, AdfError> {
        let header = Header::read(bytes)?;

        let start = header.chunk_table_offset as usize;
        let len = (header.chunk_count as usize)
            .checked_mul(ENTRY_LEN)
            .ok_or(AdfError::Malformed("chunk table length overflows"))?;
        let table = bytes
            .get(start..start.checked_add(len).ok_or(AdfError::Truncated)?)
            .ok_or(AdfError::Truncated)?;

        let mut reader = Reader::new(table);
        let mut entries = Vec::with_capacity(header.chunk_count as usize);
        for _ in 0..header.chunk_count {
            let entry = ChunkEntry::read(&mut reader)?;
            // Validate the span now. Every accessor below then works from an
            // entry known to be in range, instead of re-checking each time.
            let end = (entry.offset as usize)
                .checked_add(entry.len as usize)
                .ok_or(AdfError::Malformed("chunk extends past usize"))?;
            if end > bytes.len() {
                return Err(AdfError::Truncated);
            }
            entries.push(entry);
        }

        let mut document = AdfDoc {
            bytes,
            header,
            entries,
            inflated: HashMap::new(),
            strings_key: None,
        };
        document.inflate_all()?;
        document.strings_key = document
            .find(ChunkKind::Strings, 0)
            .map(|entry| (entry.kind.to_u16(), entry.id));
        Ok(document)
    }

    /// Inflate every compressed chunk up front.
    ///
    /// Compressed payloads cannot be borrowed from the file, and returning
    /// owned bytes from some accessors and borrowed from others would push that
    /// distinction onto every caller. Inflating here keeps one shape; the
    /// uncompressed path — which is the one that matters for large assets and
    /// embeddings — still borrows directly.
    fn inflate_all(&mut self) -> Result<(), AdfError> {
        for entry in &self.entries {
            if !entry.is_compressed() {
                continue;
            }
            let raw = self
                .bytes
                .get(entry.offset as usize..(entry.offset as usize + entry.len as usize))
                .ok_or(AdfError::Truncated)?;
            let payload =
                miniz_oxide::inflate::decompress_to_vec_with_limit(raw, entry.raw_len as usize)
                    .map_err(|_| AdfError::Malformed("chunk failed to inflate"))?;
            self.inflated
                .insert((entry.kind.to_u16(), entry.id), payload);
        }
        Ok(())
    }

    pub fn header(&self) -> &Header {
        &self.header
    }

    pub fn doc_id(&self) -> [u8; 16] {
        self.header.doc_id
    }

    pub fn chunks(&self) -> &[ChunkEntry] {
        &self.entries
    }

    fn find(&self, kind: ChunkKind, id: u32) -> Option<&ChunkEntry> {
        self.entries
            .iter()
            .find(|entry| entry.kind == kind && entry.id == id)
    }

    /// Borrow a chunk's payload, inflating transparently.
    pub fn chunk(&self, kind: ChunkKind, id: u32) -> Option<&[u8]> {
        let entry = self.find(kind, id)?;
        if entry.is_compressed() {
            return self.inflated.get(&(kind.to_u16(), id)).map(Vec::as_slice);
        }
        self.bytes
            .get(entry.offset as usize..(entry.offset as usize + entry.len as usize))
    }

    /// How many chunks of a kind the file holds.
    pub fn count_of(&self, kind: ChunkKind) -> usize {
        self.entries.iter().filter(|e| e.kind == kind).count()
    }

    /// Number of sections, without decoding any of them.
    pub fn section_count(&self) -> usize {
        self.count_of(ChunkKind::Section)
    }

    /// The string heap.
    pub fn strings(&self) -> Result<StringHeap<'_>, AdfError> {
        let bytes = self
            .chunk(ChunkKind::Strings, 0)
            .ok_or(AdfError::MissingChunk(ChunkKind::Strings))?;
        StringHeap::parse(bytes)
    }

    /// Document metadata.
    pub fn meta(&self) -> Result<Meta, AdfError> {
        let heap = self.strings()?;
        let bytes = self
            .chunk(ChunkKind::Meta, 0)
            .ok_or(AdfError::MissingChunk(ChunkKind::Meta))?;
        codec::decode_meta(bytes, &heap)
    }

    /// Decode one section. This is the operation the format exists to make
    /// cheap: it touches the header, the table, the heap and one chunk.
    pub fn section(&self, index: u32) -> Result<Section, AdfError> {
        let heap = self.strings()?;
        let bytes = self
            .chunk(ChunkKind::Section, index)
            .ok_or(AdfError::MissingChunk(ChunkKind::Section))?;
        codec::decode_section(bytes, &heap)
    }

    /// Footnotes, if the document has any.
    pub fn footnotes(&self) -> Result<Vec<Footnote>, AdfError> {
        let Some(bytes) = self.chunk(ChunkKind::Footnotes, 0) else {
            return Ok(Vec::new());
        };
        codec::decode_footnotes(bytes, &self.strings()?)
    }

    /// One asset's bytes, borrowed straight out of the file when stored raw.
    pub fn asset_bytes(&self, index: u32) -> Option<&[u8]> {
        self.chunk(ChunkKind::Asset, index)
    }

    /// The asset directory.
    pub fn assets(&self) -> Result<Vec<Asset>, AdfError> {
        let Some(bytes) = self.chunk(ChunkKind::AssetTable, 0) else {
            return Ok(Vec::new());
        };
        let heap = self.strings()?;
        let mut reader = Reader::new(bytes);
        let count = reader.count()?;

        let mut assets = Vec::with_capacity(count.min(4096));
        for index in 0..count {
            let id = heap
                .get(reader.u32()?)
                .ok_or(AdfError::Malformed("asset id string missing"))?
                .to_string();
            let media_type = heap
                .get(reader.u32()?)
                .ok_or(AdfError::Malformed("asset media type missing"))?
                .to_string();
            let width = reader.u32()?;
            let height = reader.u32()?;
            let _stored_len = reader.u32()?;

            assets.push(Asset {
                id,
                media_type,
                bytes: self
                    .asset_bytes(index as u32)
                    .map(<[u8]>::to_vec)
                    .unwrap_or_default(),
                width,
                height,
            });
        }
        Ok(assets)
    }

    /// The provenance table, borrowed.
    pub fn provenance(&self) -> ProvenanceTable<'_> {
        ProvenanceTable::new(self.chunk(ChunkKind::Provenance, 0).unwrap_or(&[]))
    }

    /// The retrieval chunk table.
    pub fn retrieval_chunks(&self) -> Result<Vec<RetrievalChunk>, AdfError> {
        match self.chunk(ChunkKind::Retrieval, 0) {
            Some(bytes) => super::index::decode_chunks(bytes),
            None => Ok(Vec::new()),
        }
    }

    /// The embedding matrix, if one was stored.
    pub fn embeddings(&self) -> Option<Embeddings<'_>> {
        Embeddings::parse(self.chunk(ChunkKind::Embeddings, 0)?).ok()
    }

    /// The term index, if one was stored.
    pub fn term_index(&self) -> Option<TermIndex> {
        TermIndex::parse(self.chunk(ChunkKind::TermIndex, 0)?).ok()
    }

    /// The edit log.
    pub fn oplog(&self) -> Result<OpLog, AdfError> {
        if self.header.oplog_len == 0 {
            return Ok(OpLog::new());
        }
        let start = self.header.oplog_offset as usize;
        let end = start
            .checked_add(self.header.oplog_len as usize)
            .ok_or(AdfError::Truncated)?;
        let bytes = self.bytes.get(start..end).ok_or(AdfError::Truncated)?;
        OpLog::parse(bytes)
    }

    /// Full-text search over the embedded term index.
    ///
    /// Returns retrieval chunks with their text, so a caller gets something it
    /// can cite rather than a list of ids it has to resolve itself.
    pub fn search(&self, query: &str) -> Result<Vec<(RetrievalChunk, String)>, AdfError> {
        let Some(index) = self.term_index() else {
            return Ok(Vec::new());
        };
        let heap = self.strings()?;
        let chunks = self.retrieval_chunks()?;

        Ok(index
            .search_all(query)
            .into_iter()
            .filter_map(|id| {
                let chunk = chunks.get(id as usize)?;
                let text = heap.get(chunk.text)?;
                Some((chunk.clone(), text.to_string()))
            })
            .collect())
    }

    /// Semantic search, when the document carries embeddings.
    pub fn search_similar(
        &self,
        query: &[f32],
        limit: usize,
    ) -> Result<Vec<(RetrievalChunk, String, f32)>, AdfError> {
        let Some(embeddings) = self.embeddings() else {
            return Ok(Vec::new());
        };
        let heap = self.strings()?;
        let chunks = self.retrieval_chunks()?;

        Ok(embeddings
            .search(query, limit)
            .into_iter()
            .filter_map(|(id, score)| {
                let chunk = chunks.get(id)?;
                let text = heap.get(chunk.text)?;
                Some((chunk.clone(), text.to_string(), score))
            })
            .collect())
    }

    /// Decode the whole document.
    ///
    /// The escape hatch for callers that genuinely want everything — export,
    /// conversion, a full-text pass. Everything above exists so that this is
    /// not the only option.
    pub fn to_semantic(&self) -> Result<SemanticDoc, AdfError> {
        let meta = self.meta()?;
        let mut document = SemanticDoc {
            title: meta.title,
            author: meta.author,
            subject: meta.subject,
            creator: meta.creator,
            created: meta.created,
            modified: meta.modified,
            sections: Vec::with_capacity(self.section_count()),
            footnotes: self.footnotes()?,
            assets: self.assets()?,
        };
        for index in 0..self.section_count() as u32 {
            document.sections.push(self.section(index)?);
        }
        Ok(document)
    }
}
