//! AgenticPDF — High-performance PDF processing library in Rust.
//!
//! Core PDF parsing, text extraction, and WASM-exportable operations
//! optimized for agentic AI workflows. Includes a machine-readable ontology
//! for autonomous discovery by agentic LLMs (ChatGPT, Claude, etc.).

#![recursion_limit = "512"]

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
    pub annotations: Vec<PdfAnnotation>,
    pub outline: Vec<OutlineItem>,
    #[allow(dead_code)]
    xref: Vec<XRefEntry>,
    #[allow(dead_code)]
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
    pub modification_date: Option<String>,
    pub page_count: usize,
    pub file_size: usize,
    pub pdf_version: String,
    pub encrypted: bool,
    pub has_forms: bool,
    pub has_annotations: bool,
    pub has_outlines: bool,
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

/// A PDF annotation (link, highlight, note, widget, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfAnnotation {
    pub subtype: String,
    pub page_number: usize,
    pub rect: [f64; 4],
    pub contents: Option<String>,
    pub uri: Option<String>,
    pub dest: Option<String>,
    pub title: Option<String>,
    pub color: Option<[f64; 3]>,
}

/// A bookmark / outline entry in the document's table of contents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutlineItem {
    pub title: String,
    pub page_number: Option<usize>,
    pub dest: Option<String>,
    pub children: Vec<OutlineItem>,
}

/// Cross-reference table entry.
#[derive(Debug, Clone)]
pub struct XRefEntry {
    pub obj_num: u32,
    pub offset: usize,
    pub generation: u16,
    pub in_use: bool,
}

/// Comprehensive extraction result for the `all` command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FullExtraction {
    pub metadata: PdfMetadata,
    pub pages: Vec<PdfPage>,
    pub annotations: Vec<PdfAnnotation>,
    pub outline: Vec<OutlineItem>,
    pub chunks: Vec<SemanticChunk>,
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

    /// Get all annotations.
    pub fn get_annotations(&self) -> &[PdfAnnotation] {
        &self.annotations
    }

    /// Get the document outline (bookmarks / table of contents).
    pub fn get_outline(&self) -> &[OutlineItem] {
        &self.outline
    }

    /// Extract everything: metadata, pages, annotations, outline, and chunks.
    pub fn extract_all(&self, chunk_size: usize, chunk_overlap: usize) -> FullExtraction {
        FullExtraction {
            metadata: self.metadata.clone(),
            pages: self.pages.clone(),
            annotations: self.annotations.clone(),
            outline: self.outline.clone(),
            chunks: self.generate_chunks(chunk_size, chunk_overlap),
        }
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
// Ontology — Machine-readable self-description for agentic LLM discovery
// ============================================================================

/// Returns the full CLI ontology as a JSON string.
/// This allows agentic LLMs (ChatGPT, Claude, Gemini, etc.) to programmatically
/// discover the CLI's commands, parameters, output schemas, and workflows.
pub fn describe_ontology() -> String {
    serde_json::to_string_pretty(&build_ontology()).unwrap_or_default()
}

/// Build the structured ontology object.
pub fn build_ontology() -> serde_json::Value {
    serde_json::json!({
        "@context": "https://schema.org",
        "@type": "SoftwareApplication",
        "name": "apdf",
        "version": "1.0.0",
        "license": "AGPL-3.0-or-later",
        "description": "High-performance Rust CLI for PDF text extraction, metadata parsing, semantic chunking, annotation reading, and outline extraction. Designed for agentic AI workflows with structured JSON output.",
        "applicationCategory": "DeveloperApplication",
        "operatingSystem": "Windows, macOS, Linux",
        "programmingLanguage": "Rust",
        "url": "https://github.com/nervosys/AgenticPDF",
        "installInstructions": "cargo install agenticpdf  OR  download pre-built binary from GitHub Releases",
        "invocation": {
            "binary": "apdf",
            "shell": "apdf <COMMAND> [OPTIONS]",
            "help": "apdf --help",
            "version": "apdf --version"
        },
        "commands": [
            {
                "name": "text",
                "description": "Extract text content from a PDF with positioning, font metadata, and page structure.",
                "usage": "apdf text <FILE> [--pages 1-5] [--format text|json] [--output path]",
                "parameters": [
                    { "name": "file", "type": "string", "required": true, "description": "Path to the PDF file" },
                    { "name": "--pages", "type": "string", "required": false, "description": "Page range (e.g. '1-5' or '3'). Defaults to all pages." },
                    { "name": "--format", "type": "enum", "values": ["text", "json"], "default": "text", "description": "Output format. Use 'json' for structured output with positioning." },
                    { "name": "--output", "type": "string", "required": false, "description": "Write output to file instead of stdout" }
                ],
                "outputSchema": {
                    "text": "Concatenated text strings separated by newlines",
                    "json": {
                        "type": "array",
                        "items": {
                            "type": "PdfPage",
                            "properties": {
                                "index": { "type": "integer", "description": "0-based page index" },
                                "width": { "type": "number", "description": "Page width in points" },
                                "height": { "type": "number", "description": "Page height in points" },
                                "text_content": {
                                    "type": "array",
                                    "items": {
                                        "type": "TextBlock",
                                        "properties": {
                                            "text": { "type": "string" },
                                            "x": { "type": "number", "description": "Horizontal position in points" },
                                            "y": { "type": "number", "description": "Vertical position in points" },
                                            "width": { "type": "number" },
                                            "height": { "type": "number" },
                                            "font_size": { "type": "number" },
                                            "font_name": { "type": "string" },
                                            "page_number": { "type": "integer", "description": "1-based page number" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                },
                "examples": [
                    "apdf text document.pdf",
                    "apdf text document.pdf --format json",
                    "apdf text document.pdf --pages 1-3 --format json --output extracted.json"
                ]
            },
            {
                "name": "meta",
                "description": "Show PDF metadata: title, author, subject, creator, producer, dates, page count, encryption status, and feature flags.",
                "usage": "apdf meta <FILE> [--format text|json]",
                "parameters": [
                    { "name": "file", "type": "string", "required": true, "description": "Path to the PDF file" },
                    { "name": "--format", "type": "enum", "values": ["text", "json"], "default": "text", "description": "Output format" }
                ],
                "outputSchema": {
                    "json": {
                        "type": "PdfMetadata",
                        "properties": {
                            "title": { "type": "string|null" },
                            "author": { "type": "string|null" },
                            "subject": { "type": "string|null" },
                            "creator": { "type": "string|null" },
                            "producer": { "type": "string|null" },
                            "creation_date": { "type": "string|null" },
                            "modification_date": { "type": "string|null" },
                            "page_count": { "type": "integer" },
                            "file_size": { "type": "integer", "description": "File size in bytes" },
                            "pdf_version": { "type": "string" },
                            "encrypted": { "type": "boolean" },
                            "has_forms": { "type": "boolean" },
                            "has_annotations": { "type": "boolean" },
                            "has_outlines": { "type": "boolean" }
                        }
                    }
                },
                "examples": [
                    "apdf meta document.pdf",
                    "apdf meta document.pdf --format json"
                ]
            },
            {
                "name": "annotations",
                "description": "Extract all annotations (links, highlights, notes, widgets, file attachments, etc.) from a PDF.",
                "usage": "apdf annotations <FILE> [--pages 1-5] [--format text|json]",
                "parameters": [
                    { "name": "file", "type": "string", "required": true, "description": "Path to the PDF file" },
                    { "name": "--pages", "type": "string", "required": false, "description": "Page range" },
                    { "name": "--format", "type": "enum", "values": ["text", "json"], "default": "json", "description": "Output format" }
                ],
                "outputSchema": {
                    "json": {
                        "type": "array",
                        "items": {
                            "type": "PdfAnnotation",
                            "properties": {
                                "subtype": { "type": "string", "description": "Annotation type: Link, Text, Highlight, Widget, FileAttachment, etc." },
                                "page_number": { "type": "integer", "description": "1-based page number" },
                                "rect": { "type": "array", "items": { "type": "number" }, "description": "[x1, y1, x2, y2] bounding box" },
                                "contents": { "type": "string|null", "description": "Text content of the annotation" },
                                "uri": { "type": "string|null", "description": "URL for Link annotations" },
                                "dest": { "type": "string|null", "description": "Internal destination reference" },
                                "title": { "type": "string|null", "description": "Author or title of the annotation" },
                                "color": { "type": "array|null", "description": "[r, g, b] color values 0-1" }
                            }
                        }
                    }
                },
                "examples": [
                    "apdf annotations document.pdf",
                    "apdf annotations document.pdf --pages 1-5 --format json"
                ]
            },
            {
                "name": "outline",
                "description": "Extract the document outline (bookmarks / table of contents) as a hierarchical tree.",
                "usage": "apdf outline <FILE> [--format text|json]",
                "parameters": [
                    { "name": "file", "type": "string", "required": true, "description": "Path to the PDF file" },
                    { "name": "--format", "type": "enum", "values": ["text", "json"], "default": "json", "description": "Output format" }
                ],
                "outputSchema": {
                    "json": {
                        "type": "array",
                        "items": {
                            "type": "OutlineItem",
                            "properties": {
                                "title": { "type": "string" },
                                "page_number": { "type": "integer|null", "description": "1-based destination page" },
                                "dest": { "type": "string|null" },
                                "children": { "type": "array", "items": { "$ref": "OutlineItem" } }
                            }
                        }
                    }
                },
                "examples": [
                    "apdf outline document.pdf",
                    "apdf outline document.pdf --format text"
                ]
            },
            {
                "name": "images",
                "description": "List images embedded in a PDF with dimensions, color space, compression filter, and byte offsets.",
                "usage": "apdf images <FILE> [--pages 1-5] [--format text|json]",
                "parameters": [
                    { "name": "file", "type": "string", "required": true, "description": "Path to the PDF file" },
                    { "name": "--pages", "type": "string", "required": false, "description": "Page range" },
                    { "name": "--format", "type": "enum", "values": ["text", "json"], "default": "json", "description": "Output format" }
                ],
                "outputSchema": {
                    "json": {
                        "type": "array",
                        "items": {
                            "type": "ImageInfo",
                            "properties": {
                                "id": { "type": "string" },
                                "width": { "type": "integer" },
                                "height": { "type": "integer" },
                                "color_space": { "type": "string" },
                                "bits_per_component": { "type": "integer" },
                                "filter": { "type": "string" },
                                "page_number": { "type": "integer" },
                                "data_offset": { "type": "integer" },
                                "data_length": { "type": "integer" }
                            }
                        }
                    }
                },
                "examples": [
                    "apdf images document.pdf",
                    "apdf images document.pdf --pages 1-3"
                ]
            },
            {
                "name": "chunk",
                "description": "Generate semantic chunks for RAG pipelines with configurable size and overlap. Chunks include importance scoring and page provenance.",
                "usage": "apdf chunk <FILE> [--size 500] [--overlap 50] [--format text|json] [--output path]",
                "parameters": [
                    { "name": "file", "type": "string", "required": true, "description": "Path to the PDF file" },
                    { "name": "--size", "type": "integer", "default": 500, "description": "Maximum chunk size in words (clamped to 50-10000)" },
                    { "name": "--overlap", "type": "integer", "default": 50, "description": "Word overlap between chunks (clamped to 0-size/2)" },
                    { "name": "--format", "type": "enum", "values": ["text", "json"], "default": "json", "description": "Output format" },
                    { "name": "--output", "type": "string", "required": false, "description": "Write output to file" }
                ],
                "outputSchema": {
                    "json": {
                        "type": "array",
                        "items": {
                            "type": "SemanticChunk",
                            "properties": {
                                "id": { "type": "string", "description": "Unique chunk identifier (chunk_0, chunk_1, ...)" },
                                "content": { "type": "string" },
                                "page_numbers": { "type": "array", "items": { "type": "integer" } },
                                "token_count": { "type": "integer" },
                                "importance": { "type": "number", "description": "0.0 to 1.0 importance score" }
                            }
                        }
                    }
                },
                "examples": [
                    "apdf chunk document.pdf",
                    "apdf chunk document.pdf --size 1000 --overlap 100 --format json --output chunks.json"
                ]
            },
            {
                "name": "all",
                "description": "Extract everything from a PDF in a single pass: metadata, all pages with text, annotations, outline, and semantic chunks. Ideal for agentic workflows that need comprehensive document understanding.",
                "usage": "apdf all <FILE> [--chunk-size 500] [--chunk-overlap 50] [--output path]",
                "parameters": [
                    { "name": "file", "type": "string", "required": true, "description": "Path to the PDF file" },
                    { "name": "--chunk-size", "type": "integer", "default": 500, "description": "Chunk size for semantic chunking" },
                    { "name": "--chunk-overlap", "type": "integer", "default": 50, "description": "Overlap for semantic chunking" },
                    { "name": "--output", "type": "string", "required": false, "description": "Write output to file" }
                ],
                "outputSchema": {
                    "json": {
                        "type": "FullExtraction",
                        "properties": {
                            "metadata": { "$ref": "PdfMetadata" },
                            "pages": { "type": "array", "items": { "$ref": "PdfPage" } },
                            "annotations": { "type": "array", "items": { "$ref": "PdfAnnotation" } },
                            "outline": { "type": "array", "items": { "$ref": "OutlineItem" } },
                            "chunks": { "type": "array", "items": { "$ref": "SemanticChunk" } }
                        }
                    }
                },
                "examples": [
                    "apdf all document.pdf",
                    "apdf all document.pdf --chunk-size 1000 --output full.json"
                ]
            },
            {
                "name": "describe",
                "description": "Output this machine-readable JSON-LD ontology describing all commands, parameters, output schemas, and workflows. Used by agentic LLMs for capability discovery.",
                "usage": "apdf describe",
                "parameters": [],
                "outputSchema": { "type": "LibraryOntology", "format": "JSON-LD" },
                "examples": ["apdf describe"]
            },
            {
                "name": "info",
                "description": "Show library version, capabilities summary, and supported formats.",
                "usage": "apdf info [--format text|json]",
                "parameters": [
                    { "name": "--format", "type": "enum", "values": ["text", "json"], "default": "text", "description": "Output format" }
                ],
                "examples": ["apdf info", "apdf info --format json"]
            }
        ],
        "workflows": [
            {
                "id": "basic-text-extraction",
                "name": "Basic Text Extraction",
                "description": "Extract readable text from a PDF",
                "steps": [
                    "apdf text document.pdf"
                ]
            },
            {
                "id": "structured-extraction",
                "name": "Structured JSON Extraction",
                "description": "Extract text with full positional and font metadata as JSON",
                "steps": [
                    "apdf text document.pdf --format json"
                ]
            },
            {
                "id": "rag-pipeline",
                "name": "RAG Pipeline Preparation",
                "description": "Generate semantic chunks for vector store ingestion",
                "steps": [
                    "apdf chunk document.pdf --size 1000 --overlap 100 --format json --output chunks.json"
                ]
            },
            {
                "id": "comprehensive-analysis",
                "name": "Comprehensive Document Analysis",
                "description": "Extract all data from a PDF in one pass for full document understanding",
                "steps": [
                    "apdf all document.pdf --output full.json"
                ]
            },
            {
                "id": "metadata-triage",
                "name": "Document Triage via Metadata",
                "description": "Quickly inspect document properties before full processing",
                "steps": [
                    "apdf meta document.pdf --format json"
                ]
            },
            {
                "id": "navigation-extraction",
                "name": "Table of Contents Extraction",
                "description": "Extract the document outline/bookmarks for navigation",
                "steps": [
                    "apdf outline document.pdf --format json"
                ]
            },
            {
                "id": "link-annotation-audit",
                "name": "Link & Annotation Audit",
                "description": "List all links, highlights, notes, and other annotations",
                "steps": [
                    "apdf annotations document.pdf --format json"
                ]
            }
        ],
        "capabilities": [
            "text_extraction",
            "text_positioning",
            "font_metadata",
            "metadata_parsing",
            "semantic_chunking",
            "annotation_extraction",
            "outline_extraction",
            "image_enumeration",
            "flatedecode_decompression",
            "png_predictor_unfiltering",
            "xref_table_parsing",
            "encryption_detection",
            "structured_json_output",
            "page_range_filtering",
            "file_output",
            "wasm_compilation"
        ],
        "outputFormats": ["text", "json"],
        "supportedPdfVersions": ["1.0", "1.1", "1.2", "1.3", "1.4", "1.5", "1.6", "1.7", "2.0"],
        "limits": {
            "maxXRefEntries": 1000000,
            "maxStreamSize": "256MB",
            "maxObjects": 500000,
            "maxRecursionDepth": 64,
            "chunkSizeRange": "50-10000 words"
        }
    })
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
            annotations: vec![],
            outline: vec![],
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
            annotations: vec![],
            outline: vec![],
            xref: vec![],
            data: vec![],
        };

        assert_eq!(doc.extract_text(), "Test content");
    }
}
