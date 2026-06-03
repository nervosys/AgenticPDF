//! AgenticPDF — High-performance PDF processing library in Rust.
//!
//! Core PDF parsing, text extraction, and WASM-exportable operations
//! optimized for agentic AI workflows. Includes a machine-readable ontology
//! for autonomous discovery by agentic LLMs (ChatGPT, Claude, etc.).

#![recursion_limit = "512"]

pub mod engine;
pub mod figures;
pub mod formula;
pub mod layout;
#[cfg(feature = "cli")]
pub mod mcp;
pub mod ocr;
pub mod parser;
pub mod sanitize;
pub mod tables;
pub mod text_norm;

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
    /// Reading-order Markdown of the whole document.
    #[serde(default)]
    pub markdown: String,
    /// Reconstructed tables.
    #[serde(default)]
    pub tables: Vec<tables::Table>,
    /// Detected figures linked to captions.
    #[serde(default)]
    pub figures: Vec<figures::Figure>,
    /// Best-effort LaTeX formulas.
    #[serde(default)]
    pub formulas: Vec<formula::Formula>,
    /// Prompt-injection / hidden-text scan report.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scan: Option<sanitize::ScanReport>,
}

// ============================================================================
// PDF Document implementation
// ============================================================================

impl PdfDocument {
    /// Parse a PDF document from raw bytes.
    pub fn from_bytes(data: &[u8]) -> Result<Self, PdfError> {
        engine::parse_document(data)
    }

    /// Construct a document from already-extracted parts (used by the engine).
    pub fn from_parts(
        version: String,
        pages: Vec<PdfPage>,
        metadata: PdfMetadata,
        annotations: Vec<PdfAnnotation>,
        outline: Vec<OutlineItem>,
    ) -> Self {
        PdfDocument {
            version,
            pages,
            metadata,
            annotations,
            outline,
            xref: Vec::new(),
            data: Vec::new(),
        }
    }

    /// Render the document as Markdown with reading-order, headings, and lists.
    pub fn to_markdown(&self) -> String {
        layout::to_markdown(self)
    }

    /// Produce a structured, reading-order layout (blocks with type/bbox/page).
    pub fn to_structured(&self) -> layout::StructuredDoc {
        layout::analyze(self)
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
            markdown: self.to_markdown(),
            tables: Vec::new(),
            figures: Vec::new(),
            formulas: formula::extract_formulas(self, &[]),
            scan: Some(sanitize::scan(self)),
        }
    }

    /// Comprehensive single-pass extraction for agentic workflows: metadata,
    /// pages, annotations, outline, chunks, reading-order Markdown, tables,
    /// figures, formulas, and a prompt-injection scan. Needs the raw bytes to
    /// recover ruling geometry and image placements.
    pub fn extract_all_with_data(
        &self,
        data: &[u8],
        chunk_size: usize,
        chunk_overlap: usize,
    ) -> FullExtraction {
        let graphics = engine::extract_graphics(data).unwrap_or_default();
        FullExtraction {
            metadata: self.metadata.clone(),
            pages: self.pages.clone(),
            annotations: self.annotations.clone(),
            outline: self.outline.clone(),
            chunks: self.generate_chunks(chunk_size, chunk_overlap),
            markdown: self.to_markdown(),
            tables: tables::detect_tables(&graphics, &self.pages),
            figures: figures::extract_figures(data, self).unwrap_or_default(),
            formulas: formula::extract_formulas(self, &graphics),
            scan: Some(sanitize::scan(self)),
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
                "name": "markdown",
                "description": "Render the PDF as clean Markdown in correct reading order, with detected headings, paragraphs, and lists. Multi-column layouts are read column-by-column via XY-cut analysis. Ideal for feeding documents to an LLM context window.",
                "usage": "apdf markdown <FILE> [--pages 1-5] [--output path]",
                "parameters": [
                    { "name": "file", "type": "string", "required": true, "description": "Path to the PDF file" },
                    { "name": "--pages", "type": "string", "required": false, "description": "Page range (e.g. '1-5' or '3'). Defaults to all pages." },
                    { "name": "--sanitize", "type": "boolean", "required": false, "description": "Drop hidden / off-page text (prompt-injection defense)" },
                    { "name": "--output", "type": "string", "required": false, "description": "Write output to file instead of stdout" }
                ],
                "outputSchema": { "type": "string", "format": "Markdown" },
                "examples": [
                    "apdf markdown document.pdf",
                    "apdf markdown paper.pdf --pages 1-3 --output paper.md",
                    "apdf markdown untrusted.pdf --sanitize"
                ]
            },
            {
                "name": "layout",
                "description": "Produce a reading-order structured layout: an array of typed blocks (heading/paragraph/list_item) with heading level, font size, and [left, bottom, right, top] bounding boxes in PDF points for precise source citations.",
                "usage": "apdf layout <FILE> [--pages 1-5] [--output path]",
                "parameters": [
                    { "name": "file", "type": "string", "required": true, "description": "Path to the PDF file" },
                    { "name": "--pages", "type": "string", "required": false, "description": "Page range" },
                    { "name": "--output", "type": "string", "required": false, "description": "Write output to file" }
                ],
                "outputSchema": {
                    "json": {
                        "type": "StructuredDoc",
                        "properties": {
                            "pages": {
                                "type": "array",
                                "items": {
                                    "type": "StructuredPage",
                                    "properties": {
                                        "page_number": { "type": "integer" },
                                        "width": { "type": "number" },
                                        "height": { "type": "number" },
                                        "blocks": {
                                            "type": "array",
                                            "items": {
                                                "type": "Block",
                                                "properties": {
                                                    "kind": { "type": "enum", "values": ["heading", "paragraph", "list_item"] },
                                                    "text": { "type": "string" },
                                                    "level": { "type": "integer", "description": "Heading level 1-6; 0 for non-headings" },
                                                    "page_number": { "type": "integer" },
                                                    "bbox": { "type": "array", "items": { "type": "number" }, "description": "[left, bottom, right, top] in PDF points" },
                                                    "font_size": { "type": "number" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                },
                "examples": [
                    "apdf layout document.pdf --format json",
                    "apdf layout paper.pdf --pages 1 --output layout.json"
                ]
            },
            {
                "name": "table",
                "description": "Reconstruct tables (bordered, booktabs-style, and borderless) from ruling lines and text alignment. Rows come from baseline clustering; columns from vertical rules or text-coverage gaps. Output as GitHub-flavored Markdown or structured JSON with per-table bounding boxes.",
                "usage": "apdf table <FILE> [--pages 1-5] [--format markdown|json] [--output path]",
                "parameters": [
                    { "name": "file", "type": "string", "required": true, "description": "Path to the PDF file" },
                    { "name": "--pages", "type": "string", "required": false, "description": "Page range" },
                    { "name": "--format", "type": "enum", "values": ["markdown", "json"], "default": "markdown", "description": "Output format" },
                    { "name": "--output", "type": "string", "required": false, "description": "Write output to file" }
                ],
                "outputSchema": {
                    "json": {
                        "type": "array",
                        "items": {
                            "type": "Table",
                            "properties": {
                                "page_number": { "type": "integer" },
                                "rows": { "type": "integer" },
                                "cols": { "type": "integer" },
                                "bbox": { "type": "array", "items": { "type": "number" }, "description": "[left, bottom, right, top] in PDF points" },
                                "cells": { "type": "array", "description": "Row-major array of cell-text rows" }
                            }
                        }
                    }
                },
                "examples": [
                    "apdf table report.pdf",
                    "apdf table report.pdf --format json --output tables.json"
                ]
            },
            {
                "name": "scan",
                "description": "Scan for hidden / off-page text used in prompt-injection attacks: fragments positioned outside the visible page or rendered at a sub-perceptible font size. Use before feeding a PDF to an LLM; combine with `markdown --sanitize` to strip the hidden content.",
                "usage": "apdf scan <FILE> [--format text|json]",
                "parameters": [
                    { "name": "file", "type": "string", "required": true, "description": "Path to the PDF file" },
                    { "name": "--format", "type": "enum", "values": ["text", "json"], "default": "text", "description": "Output format" }
                ],
                "outputSchema": {
                    "json": {
                        "type": "ScanReport",
                        "properties": {
                            "clean": { "type": "boolean" },
                            "suspicious_fragments": { "type": "integer" },
                            "findings": {
                                "type": "array",
                                "items": {
                                    "type": "Finding",
                                    "properties": {
                                        "reason": { "type": "enum", "values": ["off_page", "tiny_text"] },
                                        "page_number": { "type": "integer" },
                                        "text": { "type": "string" },
                                        "x": { "type": "number" },
                                        "y": { "type": "number" },
                                        "font_size": { "type": "number" }
                                    }
                                }
                            }
                        }
                    }
                },
                "examples": [
                    "apdf scan suspicious.pdf",
                    "apdf scan suspicious.pdf --format json"
                ]
            },
            {
                "name": "figures",
                "description": "Detect figures (placed image XObjects) and link them to their captions ('Figure N', 'Chart N') by spatial proximity. Caption-only vector figures are also surfaced. Each record carries page, bounding box, label, caption, and pixel dimensions.",
                "usage": "apdf figures <FILE> [--pages 1-5] [--format text|json]",
                "parameters": [
                    { "name": "file", "type": "string", "required": true, "description": "Path to the PDF file" },
                    { "name": "--pages", "type": "string", "required": false, "description": "Page range" },
                    { "name": "--format", "type": "enum", "values": ["text", "json"], "default": "json", "description": "Output format" }
                ],
                "outputSchema": {
                    "json": {
                        "type": "array",
                        "items": {
                            "type": "Figure",
                            "properties": {
                                "id": { "type": "string" },
                                "kind": { "type": "enum", "values": ["image", "figure", "chart", "table"] },
                                "page_number": { "type": "integer" },
                                "bbox": { "type": "array", "items": { "type": "number" }, "description": "[left, bottom, right, top] in PDF points" },
                                "label": { "type": "string|null", "description": "e.g. 'Figure 3'" },
                                "caption": { "type": "string|null" },
                                "width": { "type": "integer", "description": "Image pixel width (0 for caption-only)" },
                                "height": { "type": "integer" }
                            }
                        }
                    }
                },
                "examples": [
                    "apdf figures paper.pdf",
                    "apdf figures paper.pdf --pages 1-5 --format json"
                ]
            },
            {
                "name": "formula",
                "description": "Detect mathematical formulas (by math fonts and Unicode math characters) and reconstruct best-effort LaTeX, mapping symbols to commands and rebuilding super/subscripts from baseline shifts. Symbol-level reconstruction — not full 2-D math layout.",
                "usage": "apdf formula <FILE> [--pages 1-5] [--format text|json]",
                "parameters": [
                    { "name": "file", "type": "string", "required": true, "description": "Path to the PDF file" },
                    { "name": "--pages", "type": "string", "required": false, "description": "Page range" },
                    { "name": "--format", "type": "enum", "values": ["text", "json"], "default": "text", "description": "Output format" }
                ],
                "outputSchema": {
                    "json": {
                        "type": "array",
                        "items": {
                            "type": "Formula",
                            "properties": {
                                "page_number": { "type": "integer" },
                                "bbox": { "type": "array", "items": { "type": "number" }, "description": "[left, bottom, right, top] in PDF points" },
                                "latex": { "type": "string", "description": "Best-effort LaTeX reconstruction" },
                                "text": { "type": "string", "description": "Raw extracted text of the span" }
                            }
                        }
                    }
                },
                "examples": [
                    "apdf formula paper.pdf",
                    "apdf formula paper.pdf --pages 2 --format json"
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
                "description": "Extract everything from a PDF in a single pass: metadata, all pages with text, annotations, outline, semantic chunks, reading-order Markdown, reconstructed tables, figures linked to captions, best-effort LaTeX formulas, and a prompt-injection scan. The one call an agent needs for comprehensive document understanding.",
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
                            "chunks": { "type": "array", "items": { "$ref": "SemanticChunk" } },
                            "markdown": { "type": "string", "description": "Reading-order Markdown of the whole document" },
                            "tables": { "type": "array", "items": { "$ref": "Table" } },
                            "figures": { "type": "array", "items": { "$ref": "Figure" } },
                            "formulas": { "type": "array", "items": { "$ref": "Formula" } },
                            "scan": { "$ref": "ScanReport", "description": "Prompt-injection / hidden-text report" }
                        }
                    }
                },
                "examples": [
                    "apdf all document.pdf",
                    "apdf all document.pdf --chunk-size 1000 --output full.json"
                ]
            },
            {
                "name": "scanned",
                "description": "Detect likely-scanned pages (image-dominated with little extractable text) that need OCR. Deterministic detection is built in; recognition is delegated to a pluggable OcrBackend (or a bundled engine via the 'ocr' build feature).",
                "usage": "apdf scanned <FILE> [--format text|json]",
                "parameters": [
                    { "name": "file", "type": "string", "required": true, "description": "Path to the PDF file" },
                    { "name": "--format", "type": "enum", "values": ["text", "json"], "default": "text", "description": "Output format" }
                ],
                "outputSchema": {
                    "json": {
                        "type": "array",
                        "items": {
                            "type": "ScannedPage",
                            "properties": {
                                "page_number": { "type": "integer" },
                                "image_coverage": { "type": "number", "description": "Fraction of page covered by raster images (0-1)" },
                                "text_chars": { "type": "integer" },
                                "likely_scanned": { "type": "boolean" }
                            }
                        }
                    }
                },
                "examples": ["apdf scanned doc.pdf", "apdf scanned doc.pdf --format json"]
            },
            {
                "name": "mcp",
                "description": "Run as a Model Context Protocol (MCP) stdio server, exposing the PDF capabilities as MCP tools (extract_text, markdown, layout, tables, figures, scan_injection, metadata, outline, annotations, images, chunk) over newline-delimited JSON-RPC 2.0 on stdin/stdout. Configure as an MCP server in an agent client.",
                "usage": "apdf mcp",
                "parameters": [],
                "outputSchema": { "type": "JSON-RPC", "transport": "stdio (newline-delimited)" },
                "examples": ["apdf mcp"]
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
                "id": "llm-context-markdown",
                "name": "Markdown for LLM Context",
                "description": "Render a PDF as reading-order Markdown to feed directly into an LLM context window",
                "steps": [
                    "apdf markdown document.pdf --output document.md"
                ]
            },
            {
                "id": "citation-grounded-layout",
                "name": "Citation-Grounded Layout Extraction",
                "description": "Extract typed blocks with bounding boxes so an agent can cite exact source regions",
                "steps": [
                    "apdf layout document.pdf --output layout.json"
                ]
            },
            {
                "id": "table-extraction",
                "name": "Table Extraction",
                "description": "Reconstruct tables as Markdown or structured JSON for analysis or RAG",
                "steps": [
                    "apdf table report.pdf --format json --output tables.json"
                ]
            },
            {
                "id": "untrusted-ingestion",
                "name": "Safe Ingestion of Untrusted PDFs",
                "description": "Scan for prompt-injection signals, then extract sanitized Markdown with hidden text removed",
                "steps": [
                    "apdf scan untrusted.pdf --format json",
                    "apdf markdown untrusted.pdf --sanitize --output clean.md"
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
            "unicode_decoding",
            "tounicode_cmap",
            "winansi_encoding",
            "composite_font_identity_h",
            "cid_codespace_cmap",
            "cid_unicode_predefined_cmap",
            "font_metadata",
            "metadata_parsing",
            "semantic_chunking",
            "annotation_extraction",
            "outline_extraction",
            "image_enumeration",
            "markdown_export",
            "reading_order_analysis",
            "column_detection_xy_cut",
            "heading_detection",
            "list_detection",
            "table_reconstruction",
            "ruling_line_detection",
            "borderless_table_inference",
            "figure_detection",
            "caption_linking",
            "image_placement_bbox",
            "formula_to_latex",
            "math_symbol_mapping",
            "fraction_reconstruction",
            "nested_fractions",
            "operator_limits",
            "matrix_reconstruction",
            "nested_radicals",
            "diacritic_normalization",
            "positional_accent_reconstruction",
            "scanned_page_detection",
            "ocr_backend_interface",
            "tesseract_cli_backend",
            "bilevel_image_decode",
            "mcp_server",
            "prompt_injection_scan",
            "hidden_text_detection",
            "sanitized_extraction",
            "structured_layout",
            "bounding_boxes",
            "flatedecode_decompression",
            "ascii_hex_decode",
            "ascii85_decode",
            "png_predictor_unfiltering",
            "xref_table_parsing",
            "xref_stream_parsing",
            "object_stream_parsing",
            "page_tree_traversal",
            "inherited_attributes",
            "encryption_detection",
            "structured_json_output",
            "page_range_filtering",
            "file_output",
            "wasm_compilation"
        ],
        "outputFormats": ["text", "json", "markdown"],
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
    fn extract_all_bundles_everything() {
        let doc = PdfDocument::from_parts(
            "1.7".into(),
            vec![PdfPage {
                index: 0,
                width: 612.0,
                height: 792.0,
                text_content: vec![TextBlock {
                    text: "Hello world this is content".into(),
                    x: 72.0,
                    y: 700.0,
                    width: 120.0,
                    height: 12.0,
                    font_size: 12.0,
                    font_name: "F".into(),
                    page_number: 1,
                }],
            }],
            PdfMetadata::default(),
            vec![],
            vec![],
        );
        let full = doc.extract_all(100, 10);
        assert!(!full.markdown.is_empty());
        assert!(full.scan.is_some());
        assert!(full.scan.unwrap().clean); // no hidden text
        // Doc-only path leaves data-dependent collections empty.
        assert!(full.tables.is_empty());
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
