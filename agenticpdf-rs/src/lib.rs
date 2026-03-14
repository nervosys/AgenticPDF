//! AgenticPDF — High-performance PDF processing library in Rust.
//!
//! Core PDF parsing, text extraction, and WASM-exportable operations
//! optimized for agentic AI workflows.

pub mod parser;

#[cfg(feature = "wasm")]
pub mod wasm;

use serde::{Deserialize, Serialize};

// ============================================================================
// Core data types
// ============================================================================

/// Parsed PDF document with pages, metadata, and cross-reference table.
#[derive(Debug)]
pub struct PdfDocument {
    pub version: String,
    pub pages: Vec<PdfPage>,
    pub metadata: PdfMetadata,
    xref: Vec<XRefEntry>,
    data: Vec<u8>,
}

/// A single page in the PDF document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfPage {
    pub index: usize,
    pub width: f64,
    pub height: f64,
    pub text_content: Vec<TextBlock>,
}

/// A block of extracted text with position and style.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextBlock {
    pub text: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub font_size: f64,
    pub font_name: String,
    pub page_number: usize,
}

/// Document metadata.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PdfMetadata {
    pub title: Option<String>,
    pub author: Option<String>,
    pub subject: Option<String>,
    pub creator: Option<String>,
    pub producer: Option<String>,
    pub creation_date: Option<String>,
    pub page_count: usize,
    pub file_size: usize,
    pub pdf_version: String,
    pub encrypted: bool,
}

/// A semantic chunk for RAG pipelines.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticChunk {
    pub id: String,
    pub content: String,
    pub page_numbers: Vec<usize>,
    pub token_count: usize,
    pub importance: f64,
}

/// An image extracted from the PDF.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageInfo {
    pub id: String,
    pub width: u32,
    pub height: u32,
    pub color_space: String,
    pub bits_per_component: u8,
    pub filter: String,
    pub page_number: usize,
    pub data_offset: usize,
    pub data_length: usize,
}

/// Cross-reference table entry.
#[derive(Debug, Clone)]
pub struct XRefEntry {
    pub obj_num: u32,
    pub offset: usize,
    pub generation: u16,
    pub in_use: bool,
}

// ============================================================================
// PDF Document implementation
// ============================================================================

impl PdfDocument {
    /// Parse a PDF document from raw bytes.
    pub fn from_bytes(data: &[u8]) -> Result<Self, PdfError> {
        let mut parser = parser::PdfParser::new(data);
        parser.parse()
    }

    /// Get all text content as a single string.
    pub fn extract_text(&self) -> String {
        self.pages
            .iter()
            .flat_map(|p| p.text_content.iter())
            .map(|t| t.text.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Get text from a specific page (0-indexed).
    pub fn extract_page_text(&self, page_index: usize) -> Option<String> {
        self.pages.get(page_index).map(|p| {
            p.text_content
                .iter()
                .map(|t| t.text.as_str())
                .collect::<Vec<_>>()
                .join("\n")
        })
    }

    /// Generate semantic chunks for RAG processing.
    pub fn generate_chunks(&self, max_chunk_size: usize, overlap: usize) -> Vec<SemanticChunk> {
        let mut chunks = Vec::new();
        let mut current_text = String::new();
        let mut current_pages: Vec<usize> = Vec::new();
        let mut chunk_id = 0usize;

        for page in &self.pages {
            for block in &page.text_content {
                let word_count = current_text.split_whitespace().count();

                if word_count + block.text.split_whitespace().count() > max_chunk_size
                    && !current_text.is_empty()
                {
                    // Emit chunk
                    let token_count = current_text.split_whitespace().count();
                    chunks.push(SemanticChunk {
                        id: format!("chunk_{}", chunk_id),
                        content: current_text.clone(),
                        page_numbers: current_pages.clone(),
                        token_count,
                        importance: Self::calculate_importance(token_count, &current_pages),
                    });
                    chunk_id += 1;

                    // Keep overlap
                    if overlap > 0 {
                        let words: Vec<&str> = current_text.split_whitespace().collect();
                        let keep = words.len().saturating_sub(overlap);
                        current_text = words[keep..].join(" ");
                    } else {
                        current_text.clear();
                    }
                    current_pages.clear();
                }

                if !current_text.is_empty() {
                    current_text.push(' ');
                }
                current_text.push_str(&block.text);

                if !current_pages.contains(&block.page_number) {
                    current_pages.push(block.page_number);
                }
            }
        }

        // Final chunk
        if !current_text.is_empty() {
            let token_count = current_text.split_whitespace().count();
            chunks.push(SemanticChunk {
                id: format!("chunk_{}", chunk_id),
                content: current_text,
                page_numbers: current_pages,
                token_count,
                importance: Self::calculate_importance(token_count, &[]),
            });
        }

        chunks
    }

    /// Get document metadata.
    pub fn get_metadata(&self) -> &PdfMetadata {
        &self.metadata
    }

    /// Export all text as JSON.
    pub fn export_json(&self) -> Result<String, PdfError> {
        serde_json::to_string_pretty(&self.pages).map_err(|e| PdfError::ExportError(e.to_string()))
    }

    fn calculate_importance(token_count: usize, pages: &[usize]) -> f64 {
        let mut score: f64 = 0.5;
        if pages.contains(&1) {
            score += 0.15;
        }
        if token_count > 200 {
            score += 0.1;
        }
        score.min(1.0)
    }
}

// ============================================================================
// Error types
// ============================================================================

/// Errors that can occur during PDF processing.
#[derive(Debug)]
pub enum PdfError {
    InvalidHeader,
    XRefNotFound,
    InvalidXRef(String),
    ObjectParseError(String),
    StreamError(String),
    DecompressError(String),
    ExportError(String),
    IoError(std::io::Error),
}

impl std::fmt::Display for PdfError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PdfError::InvalidHeader => write!(f, "Invalid PDF header"),
            PdfError::XRefNotFound => write!(f, "Cross-reference table not found"),
            PdfError::InvalidXRef(msg) => write!(f, "Invalid xref: {}", msg),
            PdfError::ObjectParseError(msg) => write!(f, "Object parse error: {}", msg),
            PdfError::StreamError(msg) => write!(f, "Stream error: {}", msg),
            PdfError::DecompressError(msg) => write!(f, "Decompression error: {}", msg),
            PdfError::ExportError(msg) => write!(f, "Export error: {}", msg),
            PdfError::IoError(e) => write!(f, "I/O error: {}", e),
        }
    }
}

impl std::error::Error for PdfError {}

impl From<std::io::Error> for PdfError {
    fn from(e: std::io::Error) -> Self {
        PdfError::IoError(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata_default() {
        let meta = PdfMetadata::default();
        assert_eq!(meta.page_count, 0);
        assert!(!meta.encrypted);
    }

    #[test]
    fn test_chunk_generation() {
        let doc = PdfDocument {
            version: "1.7".to_string(),
            pages: vec![PdfPage {
                index: 0,
                width: 612.0,
                height: 792.0,
                text_content: vec![
                    TextBlock {
                        text: "Hello world this is a test document with several words".to_string(),
                        x: 0.0,
                        y: 0.0,
                        width: 100.0,
                        height: 12.0,
                        font_size: 12.0,
                        font_name: "Helvetica".to_string(),
                        page_number: 1,
                    },
                    TextBlock {
                        text: "Second paragraph with more content for chunking".to_string(),
                        x: 0.0,
                        y: 20.0,
                        width: 100.0,
                        height: 12.0,
                        font_size: 12.0,
                        font_name: "Helvetica".to_string(),
                        page_number: 1,
                    },
                ],
            }],
            metadata: PdfMetadata::default(),
            xref: vec![],
            data: vec![],
        };

        let chunks = doc.generate_chunks(10, 2);
        assert!(!chunks.is_empty());
        assert!(chunks[0].content.contains("Hello"));
    }

    #[test]
    fn test_extract_text() {
        let doc = PdfDocument {
            version: "1.7".to_string(),
            pages: vec![PdfPage {
                index: 0,
                width: 612.0,
                height: 792.0,
                text_content: vec![TextBlock {
                    text: "Test content".to_string(),
                    x: 0.0,
                    y: 0.0,
                    width: 50.0,
                    height: 12.0,
                    font_size: 12.0,
                    font_name: "Helvetica".to_string(),
                    page_number: 1,
                }],
            }],
            metadata: PdfMetadata::default(),
            xref: vec![],
            data: vec![],
        };

        assert_eq!(doc.extract_text(), "Test content");
    }
}
