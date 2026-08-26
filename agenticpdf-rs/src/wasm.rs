// SPDX-License-Identifier: AGPL-3.0-or-later
//! WASM bindings for AgenticPDF.
//!
//! Exposes document processing to JavaScript via wasm-bindgen.
//! Build with: `wasm-pack build --target web --features wasm`
//!
//! Every export takes raw bytes and routes through [`crate::document::Document`],
//! so the format is detected from content and the browser path supports the same
//! formats the CLI does. Export names and JSON shapes are unchanged from the
//! PDF-only versions, so existing consumers — including
//! `render/webgl-renderer.ts`, which calls `displayList` and `pageImages` —
//! keep working untouched.

#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

#[cfg(feature = "wasm")]
use crate::document::Document;

/// Open a document, mapping engine errors into JS exceptions.
#[cfg(feature = "wasm")]
fn open(data: &[u8]) -> Result<Document, JsValue> {
    Document::open(data).map_err(|e| JsValue::from_str(&e.to_string()))
}

#[cfg(feature = "wasm")]
fn json<T: serde::Serialize + ?Sized>(value: &T) -> Result<String, JsValue> {
    serde_json::to_string(value).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Identify a document's format from its bytes, without fully parsing it.
#[cfg(feature = "wasm")]
#[wasm_bindgen(js_name = "detectFormat")]
pub fn detect_format(data: &[u8]) -> Result<String, JsValue> {
    crate::detect::detect(data, None)
        .map(|format| format.id().to_string())
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

/// List every format the engine can open, as JSON.
#[cfg(feature = "wasm")]
#[wasm_bindgen(js_name = "supportedFormats")]
pub fn supported_formats() -> Result<String, JsValue> {
    json(&crate::describe_formats())
}

/// Parse a document from bytes and return metadata as JSON.
#[cfg(feature = "wasm")]
#[wasm_bindgen(js_name = "parsePdfMetadata")]
pub fn parse_pdf_metadata(data: &[u8]) -> Result<String, JsValue> {
    json(open(data)?.metadata())
}

/// Extract all text, returned as a single string.
#[cfg(feature = "wasm")]
#[wasm_bindgen(js_name = "extractText")]
pub fn extract_text(data: &[u8]) -> Result<String, JsValue> {
    Ok(open(data)?.extract_text())
}

/// Extract text from a specific page (0-indexed).
#[cfg(feature = "wasm")]
#[wasm_bindgen(js_name = "extractPageText")]
pub fn extract_page_text(data: &[u8], page_index: usize) -> Result<String, JsValue> {
    let document = open(data)?;
    let doc = document
        .require_geometry("per-page text")
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    doc.extract_page_text(page_index)
        .ok_or_else(|| JsValue::from_str("Page not found"))
}

/// Generate semantic chunks as JSON.
#[cfg(feature = "wasm")]
#[wasm_bindgen(js_name = "generateChunks")]
pub fn generate_chunks(
    data: &[u8],
    max_chunk_size: usize,
    overlap: usize,
) -> Result<String, JsValue> {
    json(&open(data)?.generate_chunks(max_chunk_size, overlap))
}

/// Render the document as reading-order Markdown.
#[cfg(feature = "wasm")]
#[wasm_bindgen(js_name = "toMarkdown")]
pub fn to_markdown(data: &[u8]) -> Result<String, JsValue> {
    Ok(open(data)?.to_markdown())
}

/// Produce reading-order structured layout as JSON (blocks with type/bbox).
#[cfg(feature = "wasm")]
#[wasm_bindgen(js_name = "toLayout")]
pub fn to_layout(data: &[u8]) -> Result<String, JsValue> {
    let structured = open(data)?
        .to_structured()
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    json(&structured)
}

/// Enumerate image XObjects as JSON.
#[cfg(feature = "wasm")]
#[wasm_bindgen(js_name = "listImages")]
pub fn list_images(data: &[u8]) -> Result<String, JsValue> {
    let document = open(data)?;
    document
        .require_pdf("image enumeration")
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    let images =
        crate::engine::extract_images(data).map_err(|e| JsValue::from_str(&e.to_string()))?;
    json(&images)
}

/// Reconstruct tables as JSON.
#[cfg(feature = "wasm")]
#[wasm_bindgen(js_name = "extractTables")]
pub fn extract_tables(data: &[u8]) -> Result<String, JsValue> {
    json(&open(data)?.tables())
}

/// Scan for hidden / off-page text (prompt-injection signals) as JSON.
#[cfg(feature = "wasm")]
#[wasm_bindgen(js_name = "scanInjection")]
pub fn scan_injection(data: &[u8]) -> Result<String, JsValue> {
    json(&open(data)?.scan())
}

/// Detect figures and link them to captions as JSON.
#[cfg(feature = "wasm")]
#[wasm_bindgen(js_name = "extractFigures")]
pub fn extract_figures(data: &[u8]) -> Result<String, JsValue> {
    let figures = open(data)?
        .figures()
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    json(&figures)
}

/// Detect formulas and reconstruct best-effort LaTeX as JSON.
#[cfg(feature = "wasm")]
#[wasm_bindgen(js_name = "extractFormulas")]
pub fn extract_formulas(data: &[u8]) -> Result<String, JsValue> {
    json(&open(data)?.formulas())
}

/// Extract the document's logical structure tree as JSON.
#[cfg(feature = "wasm")]
#[wasm_bindgen(js_name = "extractStructure")]
pub fn extract_structure(data: &[u8]) -> Result<String, JsValue> {
    let tree = open(data)?
        .structure()
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    json(&tree)
}

/// Extract interactive AcroForm fields as JSON.
#[cfg(feature = "wasm")]
#[wasm_bindgen(js_name = "extractFormFields")]
pub fn extract_form_fields(data: &[u8]) -> Result<String, JsValue> {
    let document = open(data)?;
    document
        .require_pdf("form fields")
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    let fields =
        crate::engine::extract_form_fields(data).map_err(|e| JsValue::from_str(&e.to_string()))?;
    json(&fields)
}

/// Extract a device-space display list (render ops) for a page, as JSON. This
/// is the draw list the hardware-accelerated WebGL renderer consumes.
#[cfg(feature = "wasm")]
#[wasm_bindgen(js_name = "displayList")]
pub fn display_list(data: &[u8], page_number: usize) -> Result<String, JsValue> {
    let list = open(data)?
        .display_list(page_number)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    json(&list)
}

/// Decoded placed images for a page (JPEG passthrough or base64 RGBA), as JSON,
/// for the WebGL renderer to upload as textures.
#[cfg(feature = "wasm")]
#[wasm_bindgen(js_name = "pageImages")]
pub fn page_images(data: &[u8], page_number: usize) -> Result<String, JsValue> {
    let images = open(data)?
        .page_images(page_number)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    json(&images)
}

/// Detect likely-scanned pages (image-dominated, low text) as JSON.
#[cfg(feature = "wasm")]
#[wasm_bindgen(js_name = "detectScanned")]
pub fn detect_scanned(data: &[u8]) -> Result<String, JsValue> {
    let document = open(data)?;
    let doc = document
        .require_pdf("scanned-page detection")
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    let report =
        crate::ocr::detect_scanned(data, doc).map_err(|e| JsValue::from_str(&e.to_string()))?;
    json(&report)
}

/// Everything in one pass — metadata, text, chunks, Markdown, tables, and a
/// prompt-injection scan — as JSON.
#[cfg(feature = "wasm")]
#[wasm_bindgen(js_name = "extractAll")]
pub fn extract_all(data: &[u8], chunk_size: usize, overlap: usize) -> Result<String, JsValue> {
    json(&open(data)?.extract_all(chunk_size, overlap))
}

/// Decompress a FlateDecode stream (for performance-critical hot paths).
#[cfg(feature = "wasm")]
#[wasm_bindgen(js_name = "decompressFlate")]
pub fn decompress_flate(data: &[u8]) -> Result<Vec<u8>, JsValue> {
    crate::parser::PdfParser::decompress_stream(data, None)
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Get the page, slide or sheet count without a full extraction.
#[cfg(feature = "wasm")]
#[wasm_bindgen(js_name = "getPageCount")]
pub fn get_page_count(data: &[u8]) -> Result<usize, JsValue> {
    Ok(open(data)?.page_count())
}
