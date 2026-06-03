//! Scanned-page detection and a pluggable OCR backend interface.
//!
//! Deterministic detection of image-dominated pages with little or no
//! extractable text (i.e. scans) is always available and useful on its own —
//! it tells an agent which pages need OCR. Recognition is delegated to a
//! caller-supplied [`OcrBackend`]; the `ocr` cargo feature adds an image-decode
//! pipeline (JPEG via `image`, CCITT G4 via `fax`, plus raw/indexed samples)
//! and a bundled [`TesseractCli`] backend. The default build stays a lean,
//! dependency-free static binary.

use crate::engine;
use crate::{PdfDocument, PdfError};
use serde::{Deserialize, Serialize};

/// Whether a bundled OCR engine was compiled in (the `ocr` feature).
pub const OCR_BUILTIN: bool = cfg!(feature = "ocr");

/// Per-page scan assessment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScannedPage {
    pub page_number: usize,
    /// Fraction of the page covered by raster images (0.0–1.0).
    pub image_coverage: f64,
    /// Extractable text characters on the page.
    pub text_chars: usize,
    /// Heuristic verdict: image-dominated with little text.
    pub likely_scanned: bool,
}

/// Result of running OCR over a page.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrResult {
    pub page_number: usize,
    pub text: String,
}

/// A pluggable OCR engine. Implement this to wire an external recognizer;
/// `ocr_scanned` will invoke it only for pages flagged as likely scanned.
pub trait OcrBackend {
    /// Recognize text for a page (1-based). `width`/`height` are PDF points.
    fn recognize_page(&self, page_number: usize, width: f64, height: f64)
    -> Result<String, String>;
}

/// A decoded 8-bit grayscale raster, ready to hand to an OCR engine.
#[derive(Debug, Clone)]
pub struct GrayImage {
    pub width: u32,
    pub height: u32,
    /// Row-major luminance, one byte per pixel (`width * height` bytes).
    pub pixels: Vec<u8>,
}

/// Decode an [`engine::ImageBlob`] to grayscale pixels. Handles JPEG
/// (DCTDecode) via the `image` crate and raw 8-bit Gray/RGB sample streams.
/// Only available with the `ocr` feature.
#[cfg(feature = "ocr")]
pub fn decode_blob(blob: &engine::ImageBlob) -> Option<GrayImage> {
    let (w, h) = (blob.width, blob.height);
    if w == 0 || h == 0 {
        return None;
    }
    if blob.filter.contains("DCTDecode") || blob.filter.contains("JPXDecode") {
        let img = image::load_from_memory(&blob.bytes).ok()?.to_luma8();
        return Some(GrayImage {
            width: img.width(),
            height: img.height(),
            pixels: img.into_raw(),
        });
    }

    // CCITT Group 4 (T.6) bilevel fax — the common scanned-page encoding.
    if blob.filter.contains("CCITTFaxDecode")
        && let Some(cc) = &blob.ccitt
        && cc.k < 0
    {
        let width = if cc.columns > 0 { cc.columns } else { w };
        let mut pixels: Vec<u8> = Vec::with_capacity(width as usize * h as usize);
        let mut row_count: u32 = 0;
        let ok = fax::decoder::decode_g4(
            blob.bytes.iter().copied(),
            width as u16,
            Some(h as u16),
            |line| {
                for color in fax::decoder::pels(line, width as u16) {
                    // Default CCITT: black is foreground (dark). /BlackIs1 inverts.
                    let dark = matches!(color, fax::Color::Black);
                    let dark = if cc.black_is_1 { !dark } else { dark };
                    pixels.push(if dark { 0 } else { 255 });
                }
                row_count += 1;
            },
        );
        if ok.is_some() && row_count > 0 && !pixels.is_empty() {
            return Some(GrayImage {
                width,
                height: row_count,
                pixels,
            });
        }
        return None;
    }

    // Raw sample stream (Flate already undone by extract_image_blobs).
    let n = (w as usize).checked_mul(h as usize)?;
    let cs = blob.color_space.as_str();
    let gray = matches!(cs, "DeviceGray" | "CalGray" | "G");
    let bpc = blob.bits_per_component;

    // Indexed / palette: each sample is an index into a color lookup table.
    if cs == "Indexed"
        && let Some(pal) = &blob.palette
        && let Some(indices) = unpack_samples(&blob.bytes, w as usize, h as usize, bpc)
    {
        let bc = pal.base_components as usize;
        let mut pixels = Vec::with_capacity(indices.len());
        for &idx in &indices {
            let off = idx as usize * bc;
            pixels.push(palette_luma(&pal.data, off, bc));
        }
        return Some(GrayImage {
            width: w,
            height: h,
            pixels,
        });
    }

    if gray {
        // Rows are padded to a byte boundary; unpack 1/2/4/8-bit samples,
        // scaling sub-byte depths up to the full 0–255 range. This covers
        // bilevel scans (the common 1-bit fax-style page image).
        if let Some(pixels) = unpack_gray(&blob.bytes, w as usize, h as usize, bpc) {
            return Some(GrayImage {
                width: w,
                height: h,
                pixels,
            });
        }
    }
    // Treat 8-bit, 3-samples/pixel as RGB and convert to luma.
    if bpc == 8 && blob.bytes.len() >= n * 3 {
        let mut pixels = Vec::with_capacity(n);
        for i in 0..n {
            let r = blob.bytes[i * 3] as u32;
            let g = blob.bytes[i * 3 + 1] as u32;
            let b = blob.bytes[i * 3 + 2] as u32;
            pixels.push(((r * 299 + g * 587 + b * 114) / 1000) as u8);
        }
        return Some(GrayImage {
            width: w,
            height: h,
            pixels,
        });
    }
    None
}

/// Unpack a row-padded sample stream (1/2/4/8 bpc) to raw integer values.
#[cfg(feature = "ocr")]
fn unpack_samples(bytes: &[u8], w: usize, h: usize, bpc: u8) -> Option<Vec<u32>> {
    let bits = bpc as usize;
    if !matches!(bits, 1 | 2 | 4 | 8) {
        return None;
    }
    let stride = (w * bits).div_ceil(8); // bytes per row, padded to a byte
    if bytes.len() < stride.checked_mul(h)? {
        return None;
    }
    let max = (1u32 << bits) - 1;
    let mut out = Vec::with_capacity(w * h);
    for row in 0..h {
        let base = row * stride;
        for col in 0..w {
            let bit_pos = col * bits;
            let byte = bytes[base + bit_pos / 8];
            let shift = 8 - bits - (bit_pos % 8);
            out.push((byte >> shift) as u32 & max);
        }
    }
    Some(out)
}

/// Unpack a row-padded grayscale sample stream (1/2/4/8 bpc) to 8-bit luma.
#[cfg(feature = "ocr")]
fn unpack_gray(bytes: &[u8], w: usize, h: usize, bpc: u8) -> Option<Vec<u8>> {
    let max = (1u32 << bpc as usize) - 1;
    let samples = unpack_samples(bytes, w, h, bpc)?;
    // Scale each sample to the full 0–255 range (1-bit: 0→0, 1→255).
    Some(samples.iter().map(|&v| ((v * 255) / max) as u8).collect())
}

/// Luminance of a palette entry at byte offset `off` with `bc` components.
#[cfg(feature = "ocr")]
fn palette_luma(data: &[u8], off: usize, bc: usize) -> u8 {
    let get = |i: usize| data.get(off + i).copied().unwrap_or(0) as u32;
    match bc {
        1 => get(0) as u8,
        4 => {
            // CMYK → RGB → luma.
            let (c, m, y, k) = (get(0), get(1), get(2), get(3));
            let r = ((255 - c) * (255 - k)) / 255;
            let g = ((255 - m) * (255 - k)) / 255;
            let b = ((255 - y) * (255 - k)) / 255;
            ((r * 299 + g * 587 + b * 114) / 1000) as u8
        }
        _ => {
            // RGB (or first three components).
            ((get(0) * 299 + get(1) * 587 + get(2) * 114) / 1000) as u8
        }
    }
}

/// An OCR engine that recognizes decoded raster images. Implement this and call
/// [`ocr_scanned_images`] to OCR scanned pages. Only available with `ocr`.
#[cfg(feature = "ocr")]
pub trait ImageOcrBackend {
    fn recognize_image(&self, image: &GrayImage) -> Result<String, String>;
}

/// Decode each likely-scanned page's dominant image and run an
/// [`ImageOcrBackend`] over it. Only available with the `ocr` feature.
#[cfg(feature = "ocr")]
pub fn ocr_scanned_images<B: ImageOcrBackend>(
    data: &[u8],
    doc: &PdfDocument,
    backend: &B,
) -> Result<Vec<OcrResult>, PdfError> {
    let scanned = detect_scanned(data, doc)?;
    let blobs = engine::extract_image_blobs(data).unwrap_or_default();
    let mut out = Vec::new();
    for p in scanned.iter().filter(|p| p.likely_scanned) {
        let dominant = blobs
            .iter()
            .filter(|b| b.page_number == p.page_number)
            .max_by_key(|b| b.width as u64 * b.height as u64);
        if let Some(blob) = dominant
            && let Some(img) = decode_blob(blob)
            && let Ok(text) = backend.recognize_image(&img)
        {
            out.push(OcrResult {
                page_number: p.page_number,
                text,
            });
        }
    }
    Ok(out)
}

const COVERAGE_THRESHOLD: f64 = 0.5;
const TEXT_THRESHOLD: usize = 100;

/// Detect likely-scanned pages from image coverage vs. extractable text.
pub fn detect_scanned(data: &[u8], doc: &PdfDocument) -> Result<Vec<ScannedPage>, PdfError> {
    // If image placement can't be recovered, assume no images (coverage 0)
    // rather than failing the whole assessment.
    let placed = engine::extract_placed_images(data).unwrap_or_default();
    let mut out = Vec::with_capacity(doc.pages.len());
    for page in &doc.pages {
        let page_no = page.index + 1;
        let area = (page.width * page.height).max(1.0);
        let img_area: f64 = placed
            .iter()
            .filter(|p| p.page_number == page_no)
            .map(|p| {
                let w = (p.bbox[2] - p.bbox[0]).abs().min(page.width);
                let h = (p.bbox[3] - p.bbox[1]).abs().min(page.height);
                w * h
            })
            .sum();
        let coverage = (img_area / area).min(1.0);
        let text_chars: usize = page
            .text_content
            .iter()
            .map(|t| t.text.chars().filter(|c| !c.is_whitespace()).count())
            .sum();
        let likely_scanned = coverage >= COVERAGE_THRESHOLD && text_chars < TEXT_THRESHOLD;
        out.push(ScannedPage {
            page_number: page_no,
            image_coverage: (coverage * 1000.0).round() / 1000.0,
            text_chars,
            likely_scanned,
        });
    }
    Ok(out)
}

/// Run a caller-supplied OCR backend over every likely-scanned page.
pub fn ocr_scanned<B: OcrBackend>(
    data: &[u8],
    doc: &PdfDocument,
    backend: &B,
) -> Result<Vec<OcrResult>, PdfError> {
    let pages = detect_scanned(data, doc)?;
    let mut out = Vec::new();
    for p in pages.iter().filter(|p| p.likely_scanned) {
        if let Some(page) = doc.pages.get(p.page_number - 1)
            && let Ok(text) = backend.recognize_page(p.page_number, page.width, page.height)
        {
            out.push(OcrResult {
                page_number: p.page_number,
                text,
            });
        }
    }
    Ok(out)
}

/// A reference [`ImageOcrBackend`] that shells out to the Tesseract CLI
/// (`tesseract` on `PATH`). No FFI and no model downloads — just the widely
/// available `tesseract` binary. Only available with the `ocr` feature.
#[cfg(feature = "ocr")]
pub struct TesseractCli {
    /// Language(s) passed to `-l` (e.g. "eng", "eng+deu"). Defaults to "eng".
    pub lang: String,
}

#[cfg(feature = "ocr")]
impl Default for TesseractCli {
    fn default() -> Self {
        Self {
            lang: "eng".to_string(),
        }
    }
}

#[cfg(feature = "ocr")]
impl ImageOcrBackend for TesseractCli {
    fn recognize_image(&self, image: &GrayImage) -> Result<String, String> {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);

        let buf = ::image::GrayImage::from_raw(image.width, image.height, image.pixels.clone())
            .ok_or_else(|| "invalid image buffer".to_string())?;
        let uniq = format!(
            "apdf_ocr_{}_{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        );
        let png_path = std::env::temp_dir().join(format!("{uniq}.png"));
        buf.save(&png_path)
            .map_err(|e| format!("png encode: {e}"))?;

        let result = std::process::Command::new("tesseract")
            .arg(&png_path)
            .arg("stdout")
            .arg("-l")
            .arg(&self.lang)
            .output();
        let _ = std::fs::remove_file(&png_path);

        match result {
            Ok(out) if out.status.success() => {
                Ok(String::from_utf8_lossy(&out.stdout).into_owned())
            }
            Ok(out) => Err(format!(
                "tesseract failed: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            )),
            Err(e) => Err(format!("could not run `tesseract` (is it installed?): {e}")),
        }
    }
}

/// Convenience: OCR every likely-scanned page with the bundled Tesseract CLI
/// backend. Only available with the `ocr` feature.
#[cfg(feature = "ocr")]
pub fn recognize_scanned(
    data: &[u8],
    doc: &PdfDocument,
    lang: &str,
) -> Result<Vec<OcrResult>, PdfError> {
    let backend = TesseractCli {
        lang: lang.to_string(),
    };
    ocr_scanned_images(data, doc, &backend)
}

/// A recognized word/line with position and confidence (from an HTTP backend).
#[cfg(feature = "ocr")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrWord {
    pub text: String,
    /// Bounding box [x1, y1, x2, y2] in the image's pixel space.
    pub bbox: [f64; 4],
    pub confidence: f64,
}

/// An [`ImageOcrBackend`] that delegates to an external OCR HTTP server —
/// e.g. **PaddleOCR**, EasyOCR, or any liteparse-compatible `/ocr` endpoint.
/// The server receives the page image (PNG) in the request body and returns
/// `{ "results": [ { "text", "bbox": [x1,y1,x2,y2], "confidence" } ] }`.
/// Only available with the `ocr` feature.
#[cfg(feature = "ocr")]
pub struct HttpOcrBackend {
    /// Full URL of the `/ocr` endpoint (e.g. "http://localhost:8868/ocr").
    pub endpoint: String,
    /// Request timeout in seconds.
    pub timeout_secs: u64,
}

#[cfg(feature = "ocr")]
impl HttpOcrBackend {
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            timeout_secs: 60,
        }
    }
}

/// Parse a liteparse/PaddleOCR-style `/ocr` JSON response into words.
#[cfg(feature = "ocr")]
pub fn parse_ocr_response(json: &str) -> Result<Vec<OcrWord>, String> {
    let v: serde_json::Value = serde_json::from_str(json).map_err(|e| e.to_string())?;
    // Accept either {"results":[...]} or a bare array of results.
    let arr = v
        .get("results")
        .and_then(|r| r.as_array())
        .or_else(|| v.as_array())
        .ok_or_else(|| "response missing `results` array".to_string())?;
    let mut out = Vec::new();
    for r in arr {
        let text = r
            .get("text")
            .and_then(|t| t.as_str())
            .unwrap_or("")
            .to_string();
        if text.trim().is_empty() {
            continue;
        }
        let mut bbox = [0.0; 4];
        if let Some(b) = r.get("bbox").and_then(|b| b.as_array()) {
            for (i, slot) in bbox.iter_mut().enumerate() {
                *slot = b.get(i).and_then(|x| x.as_f64()).unwrap_or(0.0);
            }
        }
        let confidence = r.get("confidence").and_then(|c| c.as_f64()).unwrap_or(1.0);
        out.push(OcrWord {
            text,
            bbox,
            confidence,
        });
    }
    Ok(out)
}

/// Encode a grayscale image to in-memory PNG bytes.
#[cfg(feature = "ocr")]
fn encode_png(image: &GrayImage) -> Result<Vec<u8>, String> {
    let buf = ::image::GrayImage::from_raw(image.width, image.height, image.pixels.clone())
        .ok_or_else(|| "invalid image buffer".to_string())?;
    let mut out = std::io::Cursor::new(Vec::new());
    ::image::DynamicImage::ImageLuma8(buf)
        .write_to(&mut out, ::image::ImageFormat::Png)
        .map_err(|e| format!("png encode: {e}"))?;
    Ok(out.into_inner())
}

#[cfg(feature = "ocr")]
impl ImageOcrBackend for HttpOcrBackend {
    fn recognize_image(&self, image: &GrayImage) -> Result<String, String> {
        let png = encode_png(image)?;
        let resp = ureq::post(&self.endpoint)
            .timeout(std::time::Duration::from_secs(self.timeout_secs))
            .set("Content-Type", "image/png")
            .send_bytes(&png)
            .map_err(|e| format!("OCR request to {} failed: {e}", self.endpoint))?;
        let body = resp.into_string().map_err(|e| e.to_string())?;
        let words = parse_ocr_response(&body)?;
        // Join recognized words/lines top-to-bottom, left-to-right.
        let mut words = words;
        words.sort_by(|a, b| {
            a.bbox[1]
                .partial_cmp(&b.bbox[1])
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(
                    a.bbox[0]
                        .partial_cmp(&b.bbox[0])
                        .unwrap_or(std::cmp::Ordering::Equal),
                )
        });
        Ok(words
            .iter()
            .map(|w| w.text.as_str())
            .collect::<Vec<_>>()
            .join("\n"))
    }
}

/// Convenience: OCR every likely-scanned page via an external HTTP OCR server
/// (PaddleOCR / EasyOCR / liteparse-compatible). Only with the `ocr` feature.
#[cfg(feature = "ocr")]
pub fn recognize_scanned_http(
    data: &[u8],
    doc: &PdfDocument,
    endpoint: &str,
) -> Result<Vec<OcrResult>, PdfError> {
    let backend = HttpOcrBackend::new(endpoint);
    ocr_scanned_images(data, doc, &backend)
}

/// Standard-alphabet base64 (for embedding images in JSON data URLs).
#[cfg(feature = "ocr")]
fn b64_encode(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = *chunk.get(1).unwrap_or(&0) as u32;
        let b2 = *chunk.get(2).unwrap_or(&0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(T[(n >> 18 & 63) as usize] as char);
        out.push(T[(n >> 12 & 63) as usize] as char);
        out.push(if chunk.len() > 1 {
            T[(n >> 6 & 63) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            T[(n & 63) as usize] as char
        } else {
            '='
        });
    }
    out
}

/// An [`ImageOcrBackend`] that calls a document-parsing **vision-language model**
/// over an OpenAI-compatible `/v1/chat/completions` endpoint — e.g.
/// **PaddleOCR-VL-1.6** served with vLLM (`vllm serve PaddlePaddle/PaddleOCR-VL-1.6`),
/// or any OpenAI-compatible VLM. The page image is sent as a base64 data URL with
/// a text prompt; the model's Markdown reply (`choices[0].message.content`) is
/// returned. Only available with the `ocr` feature.
#[cfg(feature = "ocr")]
pub struct VlmOcrBackend {
    /// Chat-completions URL (e.g. "http://localhost:8080/v1/chat/completions").
    pub endpoint: String,
    /// Model name the server expects.
    pub model: String,
    /// Instruction prompt sent alongside the image.
    pub prompt: String,
    /// Optional bearer token for the `Authorization` header.
    pub api_key: Option<String>,
    pub timeout_secs: u64,
}

#[cfg(feature = "ocr")]
impl VlmOcrBackend {
    pub fn new(endpoint: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            model: model.into(),
            prompt: "Convert this document image to Markdown, preserving \
                     reading order, tables, and formulas."
                .to_string(),
            api_key: None,
            timeout_secs: 120,
        }
    }
}

/// Extract the assistant message text from an OpenAI chat-completions response.
#[cfg(feature = "ocr")]
pub fn parse_vlm_response(json: &str) -> Result<String, String> {
    let v: serde_json::Value = serde_json::from_str(json).map_err(|e| e.to_string())?;
    if let Some(err) = v.get("error") {
        return Err(format!("VLM server error: {err}"));
    }
    let content = v
        .pointer("/choices/0/message/content")
        .ok_or_else(|| "response missing choices[0].message.content".to_string())?;
    match content {
        serde_json::Value::String(s) => Ok(s.clone()),
        // Some servers return content as an array of typed parts.
        serde_json::Value::Array(parts) => Ok(parts
            .iter()
            .filter_map(|p| p.get("text").and_then(|t| t.as_str()))
            .collect::<Vec<_>>()
            .join("")),
        _ => Err("unexpected content type".to_string()),
    }
}

#[cfg(feature = "ocr")]
impl ImageOcrBackend for VlmOcrBackend {
    fn recognize_image(&self, image: &GrayImage) -> Result<String, String> {
        let png = encode_png(image)?;
        let data_url = format!("data:image/png;base64,{}", b64_encode(&png));
        let body = serde_json::json!({
            "model": self.model,
            "temperature": 0,
            "max_tokens": 4096,
            "messages": [{
                "role": "user",
                "content": [
                    { "type": "text", "text": self.prompt },
                    { "type": "image_url", "image_url": { "url": data_url } }
                ]
            }]
        });
        let mut req = ureq::post(&self.endpoint)
            .timeout(std::time::Duration::from_secs(self.timeout_secs))
            .set("Content-Type", "application/json");
        if let Some(key) = &self.api_key {
            req = req.set("Authorization", &format!("Bearer {key}"));
        }
        let resp = req
            .send_string(&body.to_string())
            .map_err(|e| format!("VLM request to {} failed: {e}", self.endpoint))?;
        let text = resp.into_string().map_err(|e| e.to_string())?;
        parse_vlm_response(&text)
    }
}

/// Convenience: parse every likely-scanned page with a document-parsing VLM
/// (e.g. PaddleOCR-VL-1.6) over an OpenAI-compatible endpoint. `ocr` feature.
#[cfg(feature = "ocr")]
pub fn recognize_scanned_vlm(
    data: &[u8],
    doc: &PdfDocument,
    endpoint: &str,
    model: &str,
) -> Result<Vec<OcrResult>, PdfError> {
    let backend = VlmOcrBackend::new(endpoint, model);
    ocr_scanned_images(data, doc, &backend)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PdfMetadata, PdfPage, TextBlock};

    fn doc_with(pages: Vec<PdfPage>) -> PdfDocument {
        PdfDocument::from_parts("1.7".into(), pages, PdfMetadata::default(), vec![], vec![])
    }

    #[test]
    fn text_page_not_scanned() {
        // A page full of text and no images is not scanned. (No placed images
        // because we pass empty data; detection still classifies by text.)
        let page = PdfPage {
            index: 0,
            width: 612.0,
            height: 792.0,
            text_content: (0..50)
                .map(|i| TextBlock {
                    text: "lots of words here ".into(),
                    x: 72.0,
                    y: 700.0 - i as f64,
                    width: 100.0,
                    height: 10.0,
                    font_size: 10.0,
                    font_name: "F".into(),
                    page_number: 1,
                })
                .collect(),
        };
        let doc = doc_with(vec![page]);
        // Empty PDF bytes => no placed images; coverage 0 => not scanned.
        let report = detect_scanned(b"%PDF-1.4", &doc).unwrap();
        assert_eq!(report.len(), 1);
        assert!(!report[0].likely_scanned);
        assert!(report[0].text_chars > TEXT_THRESHOLD);
    }

    struct StubOcr;
    impl OcrBackend for StubOcr {
        fn recognize_page(&self, n: usize, _w: f64, _h: f64) -> Result<String, String> {
            Ok(format!("ocr text page {n}"))
        }
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn decodes_raw_gray_blob() {
        let blob = engine::ImageBlob {
            page_number: 1,
            width: 2,
            height: 2,
            bits_per_component: 8,
            color_space: "DeviceGray".into(),
            filter: "FlateDecode".into(),
            bytes: vec![10, 20, 30, 40],
            palette: None,
            ccitt: None,
        };
        let img = decode_blob(&blob).expect("decoded");
        assert_eq!((img.width, img.height), (2, 2));
        assert_eq!(img.pixels, vec![10, 20, 30, 40]);
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn unpacks_1bit_bilevel() {
        // One row of 8 pixels: 0b10101010 → 255,0,255,0,...
        let px = unpack_gray(&[0b1010_1010], 8, 1, 1).unwrap();
        assert_eq!(px, vec![255, 0, 255, 0, 255, 0, 255, 0]);
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn unpacks_4bit_gray_with_row_padding() {
        // width 3 @ 4bpc = 12 bits → 2 bytes/row (padded). Values 0x0,0xF,0x8.
        let px = unpack_gray(&[0x0F, 0x80], 3, 1, 4).unwrap();
        assert_eq!(px, vec![0, 255, 136]); // 0x8*255/15 = 136
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn decodes_1bit_blob_via_decode_blob() {
        let blob = engine::ImageBlob {
            page_number: 1,
            width: 8,
            height: 1,
            bits_per_component: 1,
            color_space: "DeviceGray".into(),
            filter: "FlateDecode".into(),
            bytes: vec![0b1111_0000],
            palette: None,
            ccitt: None,
        };
        let img = decode_blob(&blob).expect("decoded");
        assert_eq!(img.pixels, vec![255, 255, 255, 255, 0, 0, 0, 0]);
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn decodes_rgb_blob_to_luma() {
        let blob = engine::ImageBlob {
            page_number: 1,
            width: 1,
            height: 1,
            bits_per_component: 8,
            color_space: "DeviceRGB".into(),
            filter: "FlateDecode".into(),
            bytes: vec![255, 0, 0], // pure red → luma ~76
            palette: None,
            ccitt: None,
        };
        let img = decode_blob(&blob).expect("decoded");
        assert_eq!(img.pixels.len(), 1);
        assert!((img.pixels[0] as i32 - 76).abs() <= 1);
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn decodes_indexed_palette() {
        // 2-entry RGB palette: index 0 = black, index 1 = white. 4 pixels @ 1bpc.
        let blob = engine::ImageBlob {
            page_number: 1,
            width: 4,
            height: 1,
            bits_per_component: 1,
            color_space: "Indexed".into(),
            filter: "FlateDecode".into(),
            bytes: vec![0b1010_0000], // indices 1,0,1,0
            palette: Some(engine::Palette {
                base_components: 3,
                data: vec![0, 0, 0, 255, 255, 255], // entry0 black, entry1 white
            }),
            ccitt: None,
        };
        let img = decode_blob(&blob).expect("decoded");
        assert_eq!(img.pixels, vec![255, 0, 255, 0]);
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn parses_http_ocr_response() {
        let json = r#"{"results":[
            {"text":"Hello","bbox":[10,20,60,40],"confidence":0.98},
            {"text":"world","bbox":[70,20,120,40],"confidence":0.95},
            {"text":"  ","bbox":[0,0,1,1],"confidence":0.1}
        ]}"#;
        let words = parse_ocr_response(json).unwrap();
        assert_eq!(words.len(), 2); // blank entry skipped
        assert_eq!(words[0].text, "Hello");
        assert_eq!(words[0].bbox, [10.0, 20.0, 60.0, 40.0]);
        assert!((words[1].confidence - 0.95).abs() < 1e-9);
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn parses_bare_array_ocr_response() {
        let json = r#"[{"text":"X","bbox":[1,2,3,4],"confidence":1.0}]"#;
        assert_eq!(parse_ocr_response(json).unwrap().len(), 1);
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn base64_known_vectors() {
        assert_eq!(b64_encode(b"Man"), "TWFu");
        assert_eq!(b64_encode(b"Ma"), "TWE=");
        assert_eq!(b64_encode(b"M"), "TQ==");
        assert_eq!(b64_encode(b""), "");
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn parses_vlm_chat_response() {
        let s = r##"{"choices":[{"message":{"role":"assistant","content":"# Title\n\nBody"}}]}"##;
        assert_eq!(parse_vlm_response(s).unwrap(), "# Title\n\nBody");
        // Array-of-parts content form.
        let a = r#"{"choices":[{"message":{"content":[{"type":"text","text":"Hello "},{"type":"text","text":"world"}]}}]}"#;
        assert_eq!(parse_vlm_response(a).unwrap(), "Hello world");
        // Server error surfaces.
        assert!(parse_vlm_response(r#"{"error":{"message":"bad"}}"#).is_err());
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn decodes_ccitt_g4_roundtrip() {
        use fax::Color;
        // Encode a known 8x2 bilevel image to CCITT G4, then decode via the
        // product path and confirm the pixels round-trip.
        let width: u16 = 8;
        // Row patterns (true = black pixel).
        let rows = [
            [true, true, false, false, true, true, false, false],
            [false, false, true, true, false, false, true, true],
        ];
        let mut enc = fax::encoder::Encoder::new(fax::VecWriter::new());
        for row in &rows {
            let pels = row
                .iter()
                .map(|&b| if b { Color::Black } else { Color::White });
            enc.encode_line(pels, width).unwrap();
        }
        let encoded = enc.finish().unwrap().finish();

        let blob = engine::ImageBlob {
            page_number: 1,
            width: width as u32,
            height: 2,
            bits_per_component: 1,
            color_space: "DeviceGray".into(),
            filter: "CCITTFaxDecode".into(),
            bytes: encoded,
            palette: None,
            ccitt: Some(engine::CcittParams {
                k: -1,
                columns: width as u32,
                rows: 2,
                black_is_1: false,
            }),
        };
        let img = decode_blob(&blob).expect("decoded");
        assert_eq!((img.width, img.height), (8, 2));
        // Black → 0, White → 255.
        let expected: Vec<u8> = rows
            .iter()
            .flatten()
            .map(|&b| if b { 0 } else { 255 })
            .collect();
        assert_eq!(img.pixels, expected);
    }

    #[test]
    fn ocr_backend_invoked_for_scanned() {
        // A page with no text -> we simulate scanned by checking the orchestrator
        // path; with no images coverage is 0, so force via empty text + manual.
        let page = PdfPage {
            index: 0,
            width: 612.0,
            height: 792.0,
            text_content: vec![],
        };
        let doc = doc_with(vec![page]);
        // No images => not flagged; backend should not run. Confirms gating.
        let results = ocr_scanned(b"%PDF-1.4", &doc, &StubOcr).unwrap();
        assert!(results.is_empty());
    }
}
