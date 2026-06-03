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

/// Render the PDF as reading-order Markdown.
#[cfg(feature = "wasm")]
#[wasm_bindgen(js_name = "toMarkdown")]
pub fn to_markdown(data: &[u8]) -> Result<String, JsValue> {
    let doc = PdfDocument::from_bytes(data).map_err(|e| JsValue::from_str(&e.to_string()))?;
    Ok(doc.to_markdown())
}

/// Produce reading-order structured layout as JSON (blocks with type/bbox).
#[cfg(feature = "wasm")]
#[wasm_bindgen(js_name = "toLayout")]
pub fn to_layout(data: &[u8]) -> Result<String, JsValue> {
    let doc = PdfDocument::from_bytes(data).map_err(|e| JsValue::from_str(&e.to_string()))?;
    serde_json::to_string(&doc.to_structured()).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Enumerate image XObjects as JSON.
#[cfg(feature = "wasm")]
#[wasm_bindgen(js_name = "listImages")]
pub fn list_images(data: &[u8]) -> Result<String, JsValue> {
    let images =
        crate::engine::extract_images(data).map_err(|e| JsValue::from_str(&e.to_string()))?;
    serde_json::to_string(&images).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Reconstruct bordered tables as JSON.
#[cfg(feature = "wasm")]
#[wasm_bindgen(js_name = "extractTables")]
pub fn extract_tables(data: &[u8]) -> Result<String, JsValue> {
    let doc = PdfDocument::from_bytes(data).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let graphics =
        crate::engine::extract_graphics(data).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let tables = crate::tables::detect_tables(&graphics, &doc.pages);
    serde_json::to_string(&tables).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Scan for hidden / off-page text (prompt-injection signals) as JSON.
#[cfg(feature = "wasm")]
#[wasm_bindgen(js_name = "scanInjection")]
pub fn scan_injection(data: &[u8]) -> Result<String, JsValue> {
    let doc = PdfDocument::from_bytes(data).map_err(|e| JsValue::from_str(&e.to_string()))?;
    serde_json::to_string(&crate::sanitize::scan(&doc))
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Detect figures and link them to captions as JSON.
#[cfg(feature = "wasm")]
#[wasm_bindgen(js_name = "extractFigures")]
pub fn extract_figures(data: &[u8]) -> Result<String, JsValue> {
    let doc = PdfDocument::from_bytes(data).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let figures = crate::figures::extract_figures(data, &doc)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    serde_json::to_string(&figures).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Detect formulas and reconstruct best-effort LaTeX as JSON.
#[cfg(feature = "wasm")]
#[wasm_bindgen(js_name = "extractFormulas")]
pub fn extract_formulas(data: &[u8]) -> Result<String, JsValue> {
    let doc = PdfDocument::from_bytes(data).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let graphics =
        crate::engine::extract_graphics(data).map_err(|e| JsValue::from_str(&e.to_string()))?;
    serde_json::to_string(&crate::formula::extract_formulas(&doc, &graphics))
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Detect likely-scanned pages (image-dominated, low text) as JSON.
#[cfg(feature = "wasm")]
#[wasm_bindgen(js_name = "detectScanned")]
pub fn detect_scanned(data: &[u8]) -> Result<String, JsValue> {
    let doc = PdfDocument::from_bytes(data).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let report =
        crate::ocr::detect_scanned(data, &doc).map_err(|e| JsValue::from_str(&e.to_string()))?;
    serde_json::to_string(&report).map_err(|e| JsValue::from_str(&e.to_string()))
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
