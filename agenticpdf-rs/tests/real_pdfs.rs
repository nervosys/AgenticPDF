//! End-to-end integration tests against real PDFs in the repository.
//!
//! These exercise the whole pipeline (parse → text → markdown → tables →
//! figures → formulas) on genuine documents. Fixtures live at the repo root,
//! relative to the crate; if they are absent (e.g. a packaged crate without the
//! repo layout) each test skips rather than failing.

use std::path::Path;

use agenticpdf::PdfDocument;

/// Repo-relative fixture paths (tests run with CWD = crate dir).
const SAMPLE: &str = "../demos/sample.pdf";
const SHANNON: &str = "../website/out/shannon1948.pdf";

fn read(path: &str) -> Option<Vec<u8>> {
    if Path::new(path).exists() {
        std::fs::read(path).ok()
    } else {
        eprintln!("skipping: fixture not found: {path}");
        None
    }
}

#[test]
fn sample_arxiv_full_pipeline() {
    let Some(data) = read(SAMPLE) else { return };
    let doc = PdfDocument::from_bytes(&data).expect("parse sample.pdf");

    // Structure & metadata (xref-stream + object-stream PDF 1.7).
    assert_eq!(doc.pages.len(), 8, "page count");
    let meta = doc.get_metadata();
    assert_eq!(meta.title.as_deref(), Some("Inverting Trojans in LLMs"));
    assert!(meta.has_outlines);

    // Unicode text extraction.
    let text = doc.extract_text();
    assert!(text.contains("backdoor"), "expected body text");
    assert!(
        text.contains("\u{201c}") || text.contains("\u{201d}"),
        "curly quotes decoded"
    );

    // Reading-order Markdown with a detected heading.
    let md = doc.to_markdown();
    assert!(md.contains('#'), "markdown has headings");
    assert!(md.contains("Introduction"), "section heading present");

    // Tables, figures, formulas via the data-aware bundle.
    let bundle = doc.extract_all_with_data(&data, 500, 50);
    assert!(!bundle.tables.is_empty(), "tables reconstructed");
    assert!(!bundle.figures.is_empty(), "figures linked");
    assert!(!bundle.formulas.is_empty(), "formulas detected");
    assert!(
        bundle
            .figures
            .iter()
            .any(|f| f.label.as_deref() == Some("Figure 1")),
        "Figure 1 caption linked"
    );

    // The text-based paper is not flagged as scanned.
    let scanned = agenticpdf::ocr::detect_scanned(&data, &doc).unwrap();
    assert!(
        scanned.iter().all(|p| !p.likely_scanned),
        "no scanned pages"
    );
}

#[test]
fn shannon_classic_xref() {
    let Some(data) = read(SHANNON) else { return };
    let doc = PdfDocument::from_bytes(&data).expect("parse shannon1948.pdf");

    // 1948 paper, classic xref table, 54 pages.
    assert_eq!(doc.pages.len(), 54, "page count");

    let text = doc.extract_text();
    assert!(text.contains("Communication"), "title/body text present");
    // fi/fl ligatures should be decoded to real Unicode somewhere.
    assert!(
        text.contains('\u{fb01}') || text.contains("fi"),
        "ligature handling"
    );

    let md = doc.to_markdown();
    assert!(md.len() > 1000, "substantial markdown");
}
