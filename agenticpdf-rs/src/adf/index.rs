// SPDX-License-Identifier: AGPL-3.0-or-later
//! The retrieval index that ships inside the document.
//!
//! Normally, answering a question about a document means running it through a
//! chunker, an embedding model and a vector store first, and keeping that store
//! in sync afterwards. Putting the index in the file removes both problems: a
//! document is searchable the moment it is opened, and the index cannot go
//! stale relative to its content, because the only way to change the content is
//! to save — which rewrites the index.
//!
//! Two indexes are kept, because they fail in opposite directions. The term
//! index finds exact words — names, identifiers, error codes — which is what
//! embeddings are worst at. The embeddings find paraphrases, which is what term
//! matching is worst at. Both are optional: a document with neither is still a
//! valid document, just one an agent has to read linearly.
//!
//! Embeddings are stored as little-endian `f32` and read back by decoding
//! rather than by casting the mapped bytes. Casting would need `unsafe` and an
//! alignment the file cannot guarantee on every platform; decoding costs a load
//! and a byte-swap that never happens on the targets we build for, and scoring
//! still allocates nothing.

use std::collections::BTreeMap;

use super::AdfError;
use super::wire::{Reader, Writer};

// ============================================================================
// Retrieval chunks
// ============================================================================

/// A span of text an agent can retrieve and cite.
#[derive(Debug, Clone, PartialEq)]
pub struct RetrievalChunk {
    /// Section this chunk's text came from.
    pub section: u32,
    /// Range of top-level blocks covered, as `[first, last]` inclusive.
    pub blocks: [u32; 2],
    /// Typeset page the chunk starts on, 1-based; 0 when the document has not
    /// been laid out.
    pub page: u32,
    /// String id of the chunk's text.
    pub text: u32,
}

/// Encode the retrieval chunk table.
pub fn encode_chunks(chunks: &[RetrievalChunk]) -> Vec<u8> {
    let mut out = Writer::new();
    out.usize(chunks.len());
    for chunk in chunks {
        out.u32(chunk.section);
        out.u32(chunk.blocks[0]);
        out.u32(chunk.blocks[1]);
        out.u32(chunk.page);
        out.u32(chunk.text);
    }
    out.bytes
}

/// Decode the retrieval chunk table.
pub fn decode_chunks(bytes: &[u8]) -> Result<Vec<RetrievalChunk>, AdfError> {
    let mut reader = Reader::new(bytes);
    let count = reader.count()?;
    let mut chunks = Vec::with_capacity(count.min(1 << 16));
    for _ in 0..count {
        chunks.push(RetrievalChunk {
            section: reader.u32()?,
            blocks: [reader.u32()?, reader.u32()?],
            page: reader.u32()?,
            text: reader.u32()?,
        });
    }
    Ok(chunks)
}

// ============================================================================
// Embeddings
// ============================================================================

/// An embedding matrix: one row per retrieval chunk.
#[derive(Debug, Clone, Copy, Default)]
pub struct Embeddings<'a> {
    dim: usize,
    rows: usize,
    data: &'a [u8],
}

/// Encode an embedding matrix. Every row must have the same width.
pub fn encode_embeddings(dim: usize, rows: &[Vec<f32>]) -> Vec<u8> {
    let mut out = Writer::new();
    out.u32(dim as u32);
    out.u32(rows.len() as u32);
    for row in rows {
        for index in 0..dim {
            // Short rows are padded rather than rejected, so that a partially
            // embedded document still opens; a zero vector simply never wins a
            // similarity search.
            out.f32(row.get(index).copied().unwrap_or(0.0));
        }
    }
    out.bytes
}

impl<'a> Embeddings<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Embeddings<'a>, AdfError> {
        let mut reader = Reader::new(bytes);
        let dim = reader.u32()? as usize;
        let rows = reader.u32()? as usize;

        let needed = dim
            .checked_mul(rows)
            .and_then(|cells| cells.checked_mul(4))
            .ok_or(AdfError::Malformed("embedding matrix too large"))?;
        let data = reader.take(needed)?;

        Ok(Embeddings { dim, rows, data })
    }

    pub fn dim(&self) -> usize {
        self.dim
    }

    pub fn rows(&self) -> usize {
        self.rows
    }

    pub fn is_empty(&self) -> bool {
        self.rows == 0 || self.dim == 0
    }

    /// Row `index` as an iterator, decoded lazily and without allocating.
    pub fn row(&self, index: usize) -> Option<impl Iterator<Item = f32> + 'a> {
        if index >= self.rows {
            return None;
        }
        let start = index * self.dim * 4;
        let slice = self.data.get(start..start + self.dim * 4)?;
        Some(
            slice
                .chunks_exact(4)
                .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]])),
        )
    }

    /// Cosine similarity between `query` and row `index`.
    ///
    /// Returns `None` for a dimension mismatch rather than silently comparing
    /// truncated vectors — a query embedded by a different model than the
    /// document is a mistake worth surfacing, not one worth scoring.
    pub fn similarity(&self, index: usize, query: &[f32]) -> Option<f32> {
        if query.len() != self.dim {
            return None;
        }
        let row = self.row(index)?;

        let (mut dot, mut norm_row, mut norm_query) = (0.0f32, 0.0f32, 0.0f32);
        for (value, &q) in row.zip(query) {
            dot += value * q;
            norm_row += value * value;
            norm_query += q * q;
        }
        let magnitude = (norm_row.sqrt()) * (norm_query.sqrt());
        if magnitude == 0.0 {
            return Some(0.0);
        }
        Some(dot / magnitude)
    }

    /// The `limit` best-matching rows, most similar first.
    pub fn search(&self, query: &[f32], limit: usize) -> Vec<(usize, f32)> {
        let mut scored: Vec<(usize, f32)> = (0..self.rows)
            .filter_map(|index| self.similarity(index, query).map(|score| (index, score)))
            .collect();
        // Descending by score; NaN sorts last rather than poisoning the order.
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(limit);
        scored
    }
}

// ============================================================================
// Term index
// ============================================================================

/// Builds an inverted index from chunk text.
#[derive(Debug, Default)]
pub struct TermIndexBuilder {
    /// term -> sorted chunk ids
    postings: BTreeMap<String, Vec<u32>>,
}

impl TermIndexBuilder {
    pub fn new() -> TermIndexBuilder {
        TermIndexBuilder::default()
    }

    /// Add a chunk's text under `chunk` id.
    pub fn add(&mut self, chunk: u32, text: &str) {
        for term in tokenize(text) {
            let postings = self.postings.entry(term).or_default();
            // Text is added one chunk at a time and in order, so a repeated
            // term within the same chunk is always the last entry.
            if postings.last() != Some(&chunk) {
                postings.push(chunk);
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        self.postings.is_empty()
    }

    /// Serialise. Terms are sorted, so lookup is a binary search and a prefix
    /// query is a range — both without decoding the postings.
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Writer::new();
        out.usize(self.postings.len());
        for (term, chunks) in &self.postings {
            out.bytes_with_len(term.as_bytes());
            out.usize(chunks.len());
            // Ids ascend, so store the gaps: most are small and cost one byte.
            let mut previous = 0u32;
            for &chunk in chunks {
                out.varint(u64::from(chunk - previous));
                previous = chunk;
            }
        }
        out.bytes
    }
}

/// Split text into lowercase alphanumeric terms.
///
/// Deliberately simple: no stemming and no stop-word list. Stemming would make
/// the index disagree with a user searching for the exact word they can see on
/// the page, and stop words are exactly what a code identifier or a legal
/// citation is made of.
pub fn tokenize(text: &str) -> impl Iterator<Item = String> + '_ {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(|word| word.to_lowercase())
}

/// A decoded term index.
#[derive(Debug, Clone, Default)]
pub struct TermIndex {
    postings: BTreeMap<String, Vec<u32>>,
}

impl TermIndex {
    pub fn parse(bytes: &[u8]) -> Result<TermIndex, AdfError> {
        let mut reader = Reader::new(bytes);
        let terms = reader.count()?;
        let mut postings = BTreeMap::new();

        for _ in 0..terms {
            let term = std::str::from_utf8(reader.bytes_with_len()?)
                .map_err(|_| AdfError::Malformed("term is not UTF-8"))?
                .to_string();
            let count = reader.count()?;
            let mut chunks = Vec::with_capacity(count.min(1 << 16));
            let mut previous = 0u32;
            for _ in 0..count {
                previous = previous
                    .checked_add(reader.varint()? as u32)
                    .ok_or(AdfError::Malformed("posting id overflow"))?;
                chunks.push(previous);
            }
            postings.insert(term, chunks);
        }
        Ok(TermIndex { postings })
    }

    pub fn is_empty(&self) -> bool {
        self.postings.is_empty()
    }

    pub fn terms(&self) -> usize {
        self.postings.len()
    }

    /// Chunks containing `term`.
    pub fn lookup(&self, term: &str) -> &[u32] {
        self.postings
            .get(&term.to_lowercase())
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    /// Chunks containing *every* term in the query.
    pub fn search_all(&self, query: &str) -> Vec<u32> {
        let terms: Vec<String> = tokenize(query).collect();
        let Some((first, rest)) = terms.split_first() else {
            return Vec::new();
        };

        let mut hits: Vec<u32> = self.lookup(first).to_vec();
        for term in rest {
            let next = self.lookup(term);
            hits.retain(|chunk| next.binary_search(chunk).is_ok());
            if hits.is_empty() {
                break;
            }
        }
        hits
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embeddings_round_trip_and_rank_by_similarity() {
        let rows = vec![
            vec![1.0, 0.0, 0.0],
            vec![0.0, 1.0, 0.0],
            vec![0.9, 0.1, 0.0],
        ];
        let bytes = encode_embeddings(3, &rows);
        let embeddings = Embeddings::parse(&bytes).unwrap();

        assert_eq!(embeddings.dim(), 3);
        assert_eq!(embeddings.rows(), 3);
        assert_eq!(embeddings.row(0).unwrap().collect::<Vec<_>>(), rows[0]);

        let hits = embeddings.search(&[1.0, 0.0, 0.0], 2);
        assert_eq!(hits[0].0, 0, "the identical vector ranks first");
        assert_eq!(hits[1].0, 2, "the near vector ranks second");
    }

    #[test]
    fn a_query_of_the_wrong_width_scores_nothing() {
        let bytes = encode_embeddings(3, &[vec![1.0, 0.0, 0.0]]);
        let embeddings = Embeddings::parse(&bytes).unwrap();
        assert_eq!(embeddings.similarity(0, &[1.0, 0.0]), None);
        assert!(embeddings.search(&[1.0, 0.0], 5).is_empty());
    }

    #[test]
    fn the_term_index_round_trips_and_intersects_queries() {
        let mut builder = TermIndexBuilder::new();
        builder.add(0, "the quarterly revenue report");
        builder.add(1, "revenue grew in EMEA");
        builder.add(2, "hiring is on plan");

        let index = TermIndex::parse(&builder.encode()).unwrap();
        assert_eq!(index.lookup("revenue"), &[0, 1]);
        assert_eq!(index.lookup("Revenue"), &[0, 1], "lookup is case-folded");
        assert_eq!(index.search_all("revenue emea"), vec![1]);
        assert!(index.search_all("revenue hiring").is_empty());
    }

    #[test]
    fn a_repeated_term_is_recorded_once_per_chunk() {
        let mut builder = TermIndexBuilder::new();
        builder.add(7, "budget budget budget");
        let index = TermIndex::parse(&builder.encode()).unwrap();
        assert_eq!(index.lookup("budget"), &[7]);
    }

    #[test]
    fn tokenizing_keeps_identifiers_and_drops_punctuation() {
        let terms: Vec<String> = tokenize("Error E0432: use of `std::fmt`!").collect();
        assert_eq!(terms, ["error", "e0432", "use", "of", "std", "fmt"]);
    }
}
