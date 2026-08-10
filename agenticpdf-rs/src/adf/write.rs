// SPDX-License-Identifier: AGPL-3.0-or-later
//! Writing ADF files.
//!
//! Writing is a single forward pass: payloads are appended in order, each
//! recording where it landed, and the chunk table is written afterwards from
//! those recorded positions. That ordering is why the table lives at the end of
//! the file rather than the beginning — offsets are not known until the bytes
//! they point at have been written, and the alternative is either two passes or
//! a reserved gap that has to be guessed.

use crate::doc::SemanticDoc;

use super::codec::{self, StringTable};
use super::index::{self, RetrievalChunk, TermIndexBuilder};
use super::oplog::OpLog;
use super::provenance::{self, Provenance};
use super::wire::{self, Writer, hash64};
use super::{ChunkEntry, ChunkKind, Header, VERSION_MAJOR, VERSION_MINOR, chunk_flags};

/// Builds an ADF file.
#[derive(Debug, Default)]
pub struct AdfWriter {
    strings: StringTable,
    chunks: Vec<PendingChunk>,
    provenance: Vec<Provenance>,
    retrieval: Vec<RetrievalChunk>,
    terms: TermIndexBuilder,
    embeddings: Option<(usize, Vec<Vec<f32>>)>,
    oplog: Option<OpLog>,
    doc_id: [u8; 16],
    compress: bool,
}

#[derive(Debug)]
struct PendingChunk {
    kind: ChunkKind,
    id: u32,
    payload: Vec<u8>,
}

impl AdfWriter {
    pub fn new() -> AdfWriter {
        AdfWriter {
            // Deflate is on by default: block streams are highly repetitive and
            // the string heap even more so, and the crate already carries the
            // codec for PDF and ZIP.
            compress: true,
            ..AdfWriter::default()
        }
    }

    /// Turn compression off, for a file meant to be inspected by hand.
    pub fn uncompressed(mut self) -> AdfWriter {
        self.compress = false;
        self
    }

    /// Set the document's stable identity. Defaults to a hash of its content.
    pub fn with_doc_id(mut self, id: [u8; 16]) -> AdfWriter {
        self.doc_id = id;
        self
    }

    /// Attach an edit log.
    pub fn with_oplog(mut self, log: OpLog) -> AdfWriter {
        self.oplog = Some(log);
        self
    }

    /// Attach embeddings, one row per retrieval chunk.
    pub fn with_embeddings(mut self, dim: usize, rows: Vec<Vec<f32>>) -> AdfWriter {
        self.embeddings = Some((dim, rows));
        self
    }

    /// Record where a block came from.
    pub fn add_provenance(&mut self, row: Provenance) {
        self.provenance.push(row);
    }

    /// Intern a source identifier — a path, URL or hash — for [`Provenance`].
    pub fn intern_source(&mut self, source: &str) -> u32 {
        self.strings.intern(source)
    }

    /// Encode a document, building the retrieval index as it goes.
    pub fn write(mut self, document: &SemanticDoc, source_format: &str) -> Vec<u8> {
        let meta = codec::encode_meta(document, source_format, &mut self.strings);
        self.chunks.push(PendingChunk {
            kind: ChunkKind::Meta,
            id: 0,
            payload: meta,
        });

        for (index, section) in document.sections.iter().enumerate() {
            let payload = codec::encode_section(section, &mut self.strings);
            self.chunks.push(PendingChunk {
                kind: ChunkKind::Section,
                id: index as u32,
                payload,
            });
            self.index_section(index as u32, section);
        }

        if !document.footnotes.is_empty() {
            let payload = codec::encode_footnotes(&document.footnotes, &mut self.strings);
            self.chunks.push(PendingChunk {
                kind: ChunkKind::Footnotes,
                id: 0,
                payload,
            });
        }

        self.write_assets(document);
        self.write_index_chunks();
        self.finish()
    }

    /// Split a section into retrieval chunks, one per top-level block.
    ///
    /// Per block rather than per fixed token count: a block is already the
    /// unit an author chose and the unit a citation names, so chunk boundaries
    /// land where meaning changes instead of mid-sentence at token 512.
    fn index_section(&mut self, section: u32, doc_section: &crate::doc::Section) {
        for (index, block) in doc_section.blocks.iter().enumerate() {
            let mut text = String::new();
            crate::doc::block_text_into(block, &mut text);
            let text = text.trim();
            if text.is_empty() {
                continue;
            }

            let chunk_id = self.retrieval.len() as u32;
            self.terms.add(chunk_id, text);
            let text_id = self.strings.intern(text);
            self.retrieval.push(RetrievalChunk {
                section,
                blocks: [index as u32, index as u32],
                page: 0,
                text: text_id,
            });
        }
    }

    fn write_assets(&mut self, document: &SemanticDoc) {
        if document.assets.is_empty() {
            return;
        }

        let mut table = Writer::new();
        table.usize(document.assets.len());
        for (index, asset) in document.assets.iter().enumerate() {
            table.u32(self.strings.intern(&asset.id));
            table.u32(self.strings.intern(&asset.media_type));
            table.u32(asset.width);
            table.u32(asset.height);
            table.u32(asset.bytes.len() as u32);

            // Asset bytes get their own chunk each, so a page showing one image
            // reads one image rather than the document's whole media library.
            self.chunks.push(PendingChunk {
                kind: ChunkKind::Asset,
                id: index as u32,
                payload: asset.bytes.clone(),
            });
        }
        self.chunks.push(PendingChunk {
            kind: ChunkKind::AssetTable,
            id: 0,
            payload: table.bytes,
        });
    }

    fn write_index_chunks(&mut self) {
        if !self.provenance.is_empty() {
            let payload = provenance::encode(&self.provenance);
            self.chunks.push(PendingChunk {
                kind: ChunkKind::Provenance,
                id: 0,
                payload,
            });
        }
        if !self.retrieval.is_empty() {
            let payload = index::encode_chunks(&self.retrieval);
            self.chunks.push(PendingChunk {
                kind: ChunkKind::Retrieval,
                id: 0,
                payload,
            });
        }
        if !self.terms.is_empty() {
            let payload = self.terms.encode();
            self.chunks.push(PendingChunk {
                kind: ChunkKind::TermIndex,
                id: 0,
                payload,
            });
        }
        if let Some((dim, rows)) = self.embeddings.take() {
            let payload = index::encode_embeddings(dim, &rows);
            self.chunks.push(PendingChunk {
                kind: ChunkKind::Embeddings,
                id: 0,
                payload,
            });
        }
    }

    /// Lay out the file: header, payloads, chunk table, op log.
    fn finish(mut self) -> Vec<u8> {
        // The string heap is written last of the content chunks but must be
        // first in the table's eyes, because every other chunk indexes into it.
        // Interning is finished by now, so this is the point it can be frozen.
        let strings = self.strings.encode();
        self.chunks.insert(
            0,
            PendingChunk {
                kind: ChunkKind::Strings,
                id: 0,
                payload: strings,
            },
        );

        let mut out = Writer::new();
        out.bytes.resize(super::HEADER_LEN, 0);

        let mut entries = Vec::with_capacity(self.chunks.len());
        for chunk in &self.chunks {
            out.pad_to_align();
            let offset = out.len() as u64;
            let raw_len = chunk.payload.len() as u32;
            let hash = hash64(&chunk.payload);

            // Compression is per chunk and only kept when it wins. Asset bytes
            // are usually already-compressed JPEG or PNG, where deflate costs
            // time and adds bytes.
            let (bytes, flags) = match self.compress && chunk.payload.len() > 128 {
                true => {
                    let deflated = miniz_oxide::deflate::compress_to_vec(&chunk.payload, 6);
                    if deflated.len() < chunk.payload.len() {
                        (deflated, chunk_flags::COMPRESSED)
                    } else {
                        (chunk.payload.clone(), 0)
                    }
                }
                false => (chunk.payload.clone(), 0),
            };

            out.raw(&bytes);
            entries.push(ChunkEntry {
                kind: chunk.kind,
                flags,
                id: chunk.id,
                offset,
                len: bytes.len() as u32,
                raw_len,
                hash,
            });
        }

        out.pad_to_align();
        let chunk_table_offset = out.len() as u64;
        for entry in &entries {
            entry.write(&mut out);
        }

        let (oplog_offset, oplog_len) = match &self.oplog {
            Some(log) if !log.is_empty() => {
                out.pad_to_align();
                let offset = out.len() as u64;
                let bytes = log.encode();
                out.raw(&bytes);
                (offset, bytes.len() as u64)
            }
            _ => (0, 0),
        };

        // A document with no explicit identity gets one derived from its
        // content, so the same input twice is the same document rather than
        // two that merely look alike.
        if self.doc_id == [0u8; 16] {
            let content = hash64(&out.bytes[super::HEADER_LEN..]);
            self.doc_id[..8].copy_from_slice(&content.to_le_bytes());
            self.doc_id[8..].copy_from_slice(&(entries.len() as u64).to_le_bytes());
        }

        let header = Header {
            version_major: VERSION_MAJOR,
            version_minor: VERSION_MINOR,
            flags: 0,
            chunk_count: entries.len() as u32,
            chunk_table_offset,
            oplog_offset,
            oplog_len,
            doc_id: self.doc_id,
        };
        let mut head = Writer::new();
        header.write(&mut head);
        out.bytes[..super::HEADER_LEN].copy_from_slice(&head.bytes);

        out.bytes
    }
}

/// Append operations to an existing file without rewriting it.
///
/// The op log is the last thing in the file precisely so that this is possible:
/// a one-word edit to a 500 MB document writes the bytes of that edit plus a
/// new header, not 500 MB.
pub fn append_ops(file: &mut Vec<u8>, log: &OpLog, since: Option<super::oplog::OpId>) {
    let segment = log.encode_since(since);
    if segment.is_empty() {
        return;
    }

    let mut header = match Header::read(file) {
        Ok(header) => header,
        Err(_) => return,
    };

    // A file written without a log has no log region yet; start one at the end.
    if header.oplog_offset == 0 {
        let padded = wire::align_up(file.len());
        file.resize(padded, 0);
        header.oplog_offset = padded as u64;
        header.oplog_len = 0;
    }

    file.extend_from_slice(&segment);
    header.oplog_len += segment.len() as u64;

    let mut head = Writer::new();
    header.write(&mut head);
    file[..super::HEADER_LEN].copy_from_slice(&head.bytes);
}
