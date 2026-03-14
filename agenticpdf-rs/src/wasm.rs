//! WASM bindings for AgenticPDF.
//!
//! Exposes PDF processing functions to JavaScript via wasm-bindgen.
//! Build with: `wasm-pack build --target web --features wasm`

#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

#[cfg(feature = "wasm")]
use crate::PdfDocument;

/// Parse a PDF from bytes and return metadata as JSON.
#[cfg(feature = "wasm")]
#[wasm_bindgen(js_name = "parsePdfMetadata")]
pub fn parse_pdf_metadata(data: &[u8]) -> Result<String, JsValue> {
    let doc = PdfDocument::from_bytes(data).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let meta = doc.get_metadata();
    serde_json::to_string(meta).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Extract all text from a PDF, returned as a single string.
#[cfg(feature = "wasm")]
#[wasm_bindgen(js_name = "extractText")]
pub fn extract_text(data: &[u8]) -> Result<String, JsValue> {
    let doc = PdfDocument::from_bytes(data).map_err(|e| JsValue::from_str(&e.to_string()))?;
    Ok(doc.extract_text())
}

/// Extract text from a specific page (0-indexed).
#[cfg(feature = "wasm")]
#[wasm_bindgen(js_name = "extractPageText")]
pub fn extract_page_text(data: &[u8], page_index: usize) -> Result<String, JsValue> {
    let doc = PdfDocument::from_bytes(data).map_err(|e| JsValue::from_str(&e.to_string()))?;
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
    let doc = PdfDocument::from_bytes(data).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let chunks = doc.generate_chunks(max_chunk_size, overlap);
    serde_json::to_string(&chunks).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Decompress a FlateDecode stream (for performance-critical hot paths).
#[cfg(feature = "wasm")]
#[wasm_bindgen(js_name = "decompressFlate")]
pub fn decompress_flate(data: &[u8]) -> Result<Vec<u8>, JsValue> {
    crate::parser::PdfParser::decompress_stream(data, None, None)
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Get the page count from a PDF without full parsing.
#[cfg(feature = "wasm")]
#[wasm_bindgen(js_name = "getPageCount")]
pub fn get_page_count(data: &[u8]) -> Result<usize, JsValue> {
    let doc = PdfDocument::from_bytes(data).map_err(|e| JsValue::from_str(&e.to_string()))?;
    Ok(doc.get_metadata().page_count)
}
