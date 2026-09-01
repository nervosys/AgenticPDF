// SPDX-License-Identifier: AGPL-3.0-or-later
//! Does the pipeline scale linearly with the size of a document?
//!
//! Ignored by default: it is a measurement, not an assertion, and it builds
//! documents large enough to be slow on purpose.
//!
//! `cargo test --release --test scaling -- --ignored --nocapture`

use std::time::Instant;

use agenticpdf::document::Document;

fn package(document_xml: Vec<u8>) -> Vec<u8> {
    const RELS: &[u8] = br#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
         <Relationship Id="rId1" Target="word/document.xml"
           Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument"/>
       </Relationships>"#;
    agenticpdf::testing::build_zip(&[
        (
            "[Content_Types].xml",
            br#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"/>"#,
            false,
        ),
        ("_rels/.rels", RELS, false),
        ("word/document.xml", document_xml.as_slice(), true),
    ])
}

fn body(paragraphs: usize) -> Vec<u8> {
    let one = "<w:p><w:r><w:t>AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA</w:t></w:r></w:p>";
    format!(
        "<w:document xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\">\
         <w:body>{}</w:body></w:document>",
        one.repeat(paragraphs)
    )
    .into_bytes()
}

#[test]
#[ignore = "a measurement, and deliberately slow"]
fn docx_time_against_size() {
    eprintln!(
        "  {:>9}  {:>10}  {:>10}  {:>8}",
        "paras", "open", "text", "ratio"
    );
    let mut previous: Option<(usize, f64)> = None;
    for paragraphs in [80_000usize, 160_000, 320_000, 640_000] {
        let zip = package(body(paragraphs));

        let started = Instant::now();
        let document = Document::open_with_hint(&zip, Some("docx")).expect("opens");
        let open = started.elapsed().as_secs_f64();

        let started = Instant::now();
        let chars = document.extract_text().len();
        let text = started.elapsed().as_secs_f64();

        // Doubling the input should roughly double the time. Much more than
        // that is superlinear, and on input a sender controls that is the
        // difference between slow and unusable.
        let ratio = match previous {
            Some((was, then)) if was * 2 == paragraphs => format!("{:.2}x", (open + text) / then),
            _ => "-".to_string(),
        };
        eprintln!(
            "  {paragraphs:>9}  {open:>9.3}s  {text:>9.3}s  {ratio:>8}   {chars} chars,              {} pages, {} sections",
            document.page_count(),
            document.section_count()
        );
        previous = Some((paragraphs, open + text));
    }
}

/// How long a *real* document takes to open, so the cost of laying one out
/// eagerly can be judged against documents that exist rather than only against
/// the pathological one.
#[test]
#[ignore = "reads tests/fixtures if present"]
fn real_documents_open_quickly() {
    let Ok(entries) = std::fs::read_dir("tests/fixtures") else {
        eprintln!("no fixtures present");
        return;
    };
    let mut rows: Vec<(f64, f64, String, usize)> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(ext) = path
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_string)
        else {
            continue;
        };
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let started = Instant::now();
        let Ok(document) = Document::open_with_hint(&bytes, Some(&ext)) else {
            continue;
        };
        let open = started.elapsed().as_secs_f64() * 1000.0;
        let started = Instant::now();
        let chars = document.extract_text().len();
        let text = started.elapsed().as_secs_f64() * 1000.0;
        rows.push((
            open,
            text,
            path.file_name().unwrap().to_string_lossy().into(),
            chars,
        ));
    }
    rows.sort_by(|a, b| b.0.total_cmp(&a.0));
    eprintln!("  {:>9}  {:>9}  file", "open ms", "text ms");
    for (open, text, name, chars) in &rows {
        eprintln!("  {open:>9.2}  {text:>9.2}  {name}  ({chars} chars)");
    }
}

/// Where the time goes on a large document, phase by phase.
#[test]
#[ignore = "a measurement, and deliberately slow"]
fn phases_of_a_large_document() {
    let zip = package(body(60_000));
    let started = Instant::now();
    let document = Document::open_with_hint(&zip, Some("docx")).expect("opens");
    eprintln!("  {:>9.3?}  open (parse + typeset)", started.elapsed());

    macro_rules! phase {
        ($name:expr, $body:expr) => {{
            let started = Instant::now();
            let value = $body;
            eprintln!("  {:>9.3?}  {}", started.elapsed(), $name);
            value
        }};
    }

    let text = phase!("extract_text", document.extract_text());
    eprintln!("             ({} chars)", text.len());
    phase!("to_markdown", document.to_markdown());
    phase!("to_html", document.to_html());
    phase!("tables", document.tables());
    phase!(
        "generate_chunks(512, 64)",
        document.generate_chunks(512, 64)
    );
    phase!("scan", document.scan());
    let _ = phase!("structure", document.structure());
    phase!("display_list(1..=3)", {
        for page in 1..=document.page_count().min(3) {
            let _ = document.display_list(page);
        }
    });
}

/// Does chunking scale with the document, or with its square?
#[test]
#[ignore = "a measurement"]
fn chunking_against_size() {
    eprintln!(
        "  {:>9}  {:>11}  {:>8}  {:>8}",
        "paras", "chunk", "ratio", "chunks"
    );
    let mut previous: Option<f64> = None;
    for paragraphs in [10_000usize, 20_000, 40_000, 80_000] {
        let zip = package(body(paragraphs));
        let document = Document::open_with_hint(&zip, Some("docx")).expect("opens");
        let started = Instant::now();
        let chunks = document.generate_chunks(512, 64);
        let took = started.elapsed().as_secs_f64();
        let ratio = previous
            .map(|then| format!("{:.2}x", took / then))
            .unwrap_or_else(|| "-".into());
        eprintln!(
            "  {paragraphs:>9}  {took:>10.3}s  {ratio:>8}  {:>8}",
            chunks.len()
        );
        previous = Some(took);
    }
}
