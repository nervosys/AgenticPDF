// SPDX-License-Identifier: AGPL-3.0-or-later
//! The pipeline must not panic, hang or exhaust memory on damaged input.
//!
//! Every other test in this repository feeds the readers documents that are
//! *correct* — hand-written fixtures, or files a real producer wrote. Neither
//! says what happens when a file is truncated by a failed download, corrupted in
//! transit, or built to be hostile. For a library that opens documents an agent
//! fetched from somewhere, that is the case that matters most: a panic in a
//! parser is a denial of service, and an unbounded loop is worse because it
//! takes the host with it.
//!
//! So this damages known-good documents in the ways real files get damaged and
//! asserts only that the pipeline *returns* — cleanly or as an error, but
//! within a budget and without unwinding.
//!
//! The seeds are synthetic and committed, so this runs anywhere. Any
//! real-producer files present in `tests/fixtures/` are damaged too, which is
//! where the interesting shapes come from.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::time::{Duration, Instant};

use agenticpdf::document::Document;

/// Nothing may take longer than this on input this small. Generous by two
/// orders of magnitude: the point is to catch a loop that does not terminate,
/// not to measure performance.
const BUDGET: Duration = Duration::from_secs(5);

/// A minimal PDF, a ZIP-shaped package, an RTF and the text formats, so the
/// test has something to damage without any fixture present.
fn seeds() -> Vec<(&'static str, Vec<u8>)> {
    let mut out: Vec<(&'static str, Vec<u8>)> = vec![
        (
            "pdf",
            b"%PDF-1.4\n1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj\n\
              2 0 obj<</Type/Pages/Kids[3 0 R]/Count 1>>endobj\n\
              3 0 obj<</Type/Page/Parent 2 0 R/MediaBox[0 0 99 99]/Contents 4 0 R>>endobj\n\
              4 0 obj<</Length 44>>stream\nBT /F1 12 Tf 10 50 Td (hello) Tj ET\nendstream endobj\n\
              startxref\n0\n%%EOF"
                .to_vec(),
        ),
        (
            "html",
            b"<html><body><h1>Title</h1><p>Body &amp; more</p>\
              <table><tr><td>a</td><td>b</td></tr></table></body></html>"
                .to_vec(),
        ),
        (
            "md",
            b"# Title\n\nBody text.\n\n- one\n- two\n\n| a | b |\n| --- | --- |\n| 1 | 2 |\n"
                .to_vec(),
        ),
        ("csv", b"a,b,c\n1,2,3\n4,5,6\n".to_vec()),
        (
            "rtf",
            br"{\rtf1\ansi{\stylesheet{\s1 heading 1;}}\s1 Title\par Body\par}".to_vec(),
        ),
    ];

    // Whatever real files happen to be present, which is where the shapes that
    // matter come from. Absent is fine; the seeds above stand alone.
    if let Ok(entries) = std::fs::read_dir("tests/fixtures") {
        for entry in entries.flatten() {
            let path = entry.path();
            let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
                continue;
            };
            // Leaked deliberately: the table wants a `&'static str` and there
            // are at most a few dozen of these, once, in a test binary.
            let ext: &'static str = Box::leak(ext.to_lowercase().into_boxed_str());
            if let Ok(bytes) = std::fs::read(&path) {
                out.push((ext, bytes));
            }
        }
    }
    out
}

/// The ways a document actually arrives damaged.
fn damage(seed: &[u8]) -> Vec<(String, Vec<u8>)> {
    let mut out = Vec::new();
    let len = seed.len();
    if len < 8 {
        return out;
    }

    // A failed download, cut at every eighth.
    for k in 1..8 {
        let cut = len * k / 8;
        out.push((format!("truncated to {k}/8"), seed[..cut].to_vec()));
    }

    // A run of bytes lost or overwritten, at the head, the middle and the tail.
    // The head damages the header a reader dispatches on; the tail damages the
    // trailer or central directory it seeks from.
    for (name, at) in [("head", 0), ("middle", len / 2), ("tail", len * 7 / 8)] {
        let mut zeroed = seed.to_vec();
        let end = (at + 64).min(len);
        zeroed[at..end].fill(0);
        out.push((format!("zeroed 64 bytes at the {name}"), zeroed));

        let mut flipped = seed.to_vec();
        for byte in &mut flipped[at..end] {
            *byte ^= 0xFF;
        }
        out.push((format!("inverted 64 bytes at the {name}"), flipped));
    }

    // A length field claiming far more than the file holds is the classic way
    // to make a reader allocate or loop; sweep a few plausible offsets.
    for at in [4usize, 16, 64] {
        if at + 4 <= len {
            let mut huge = seed.to_vec();
            huge[at..at + 4].copy_from_slice(&u32::MAX.to_le_bytes());
            out.push((format!("u32::MAX written at {at}"), huge));
        }
    }

    out
}

/// Run everything a caller can ask of a document, and return without panicking.
///
/// Reports whether the document opened, because an attack turned away at the
/// door proves nothing about the code it was aimed at: the first zip bomb here
/// was rejected for a missing part and "passed" in 38 microseconds.
fn exercise(bytes: &[u8], ext: &str) -> String {
    let document = match Document::open_with_hint(bytes, Some(ext)) {
        Ok(document) => document,
        Err(e) => return format!("rejected: {e}"),
    };
    let _ = document.extract_text();
    let _ = document.to_markdown();
    let _ = document.to_html();
    let _ = document.tables();
    let _ = document.generate_chunks(512, 64);
    let _ = document.scan();
    let _ = document.structure();
    // Rendering, which is the other half of the pipeline and has its own
    // budgets to respect.
    for page in 1..=document.page_count().min(3) {
        let _ = document.display_list(page);
    }
    format!("opened, {} chars", document.extract_text().len())
}

#[test]
fn damaged_documents_return_rather_than_panic() {
    let mut cases = 0usize;
    let mut panics: Vec<String> = Vec::new();
    let mut slow: Vec<(Duration, String)> = Vec::new();

    for (ext, seed) in seeds() {
        // The undamaged file first: a seed that panics makes every result below
        // it meaningless.
        let what = format!("{ext}: undamaged");
        let started = Instant::now();
        if catch_unwind(AssertUnwindSafe(|| exercise(&seed, ext))).is_err() {
            panics.push(what.clone());
        }
        cases += 1;
        if started.elapsed() > BUDGET {
            slow.push((started.elapsed(), what));
        }

        for (how, bytes) in damage(&seed) {
            let what = format!("{ext}: {how}");
            let started = Instant::now();
            if catch_unwind(AssertUnwindSafe(|| exercise(&bytes, ext))).is_err() {
                panics.push(what.clone());
            }
            cases += 1;
            let took = started.elapsed();
            if took > BUDGET {
                slow.push((took, what));
            }
        }
    }

    eprintln!("{cases} damaged documents exercised");
    slow.sort_by_key(|(took, _)| std::cmp::Reverse(*took));
    for (took, what) in slow.iter().take(8) {
        eprintln!("  slow: {took:?}  {what}");
    }

    assert!(
        panics.is_empty(),
        "{} of {cases} panicked:\n  {}",
        panics.len(),
        panics.join("\n  ")
    );
    assert!(
        slow.is_empty(),
        "{} of {cases} took longer than {BUDGET:?}:\n  {}",
        slow.len(),
        slow.iter()
            .map(|(d, w)| format!("{d:?} {w}"))
            .collect::<Vec<_>>()
            .join("\n  ")
    );
}

// ============================================================================
// Structural attacks
// ============================================================================
//
// Damaging bytes finds a reader that trusts a length or an offset. It does not
// find a reader that trusts a *structure*: an archive that expands to more than
// it claims, a nesting that never ends, a dimension nobody could draw. Those
// have to be built on purpose.

/// A megabyte of one byte compresses to almost nothing, which is what makes a
/// bomb. Repeated across members it is how a small archive becomes a large
/// allocation.
fn compressible(bytes: usize) -> Vec<u8> {
    vec![b'A'; bytes]
}

/// A .docx the reader will actually open, plus whatever extra parts are given.
///
/// The relationships part matters: without it the package is rejected as
/// "missing required part" before a single byte is decompressed, and an attack
/// turned away at the door proves nothing about the code it was aimed at. The
/// first version of the bomb below was rejected exactly that way and "passed"
/// in 38 microseconds.
fn package(document_xml: Vec<u8>, extra: &[(&str, Vec<u8>, bool)]) -> Vec<u8> {
    const RELS: &[u8] = br#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
         <Relationship Id="rId1" Target="word/document.xml"
           Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument"/>
       </Relationships>"#;
    let mut members: Vec<(&str, &[u8], bool)> = vec![
        (
            "[Content_Types].xml",
            br#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"/>"#,
            false,
        ),
        ("_rels/.rels", RELS, false),
        ("word/document.xml", document_xml.as_slice(), true),
    ];
    for (name, body, deflate) in extra {
        members.push((name, body.as_slice(), *deflate));
    }
    agenticpdf::testing::build_zip(&members)
}

/// A WordprocessingML body wrapping the given inner markup.
fn document_xml(inner: &str) -> Vec<u8> {
    format!(
        "<w:document xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\">\
         <w:body>{inner}</w:body></w:document>"
    )
    .into_bytes()
}

#[test]
fn structural_attacks_return_within_budget() {
    let mut cases: Vec<(String, Vec<u8>, &'static str)> = Vec::new();

    // The part the reader must read, made enormous. Thirty-two megabytes of
    // one repeated paragraph compresses to a few kilobytes, so this is cheap to
    // send and expensive to open -- which is the whole of the attack.
    let paragraph = "<w:p><w:r><w:t>AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA</w:t></w:r></w:p>";
    // Sized so the test stays inside a CI run while still decompressing and
    // parsing megabytes: the point is that the path is exercised, and the
    // budget below is what proves it stays bounded.
    let huge_body = paragraph.repeat(60_000);
    cases.push((
        "a 4 MB main part from a few KB".into(),
        package(document_xml(&huge_body), &[]),
        "docx",
    ));

    // The same weight spread over parts the reader does *not* need, which it
    // should therefore never decompress.
    let spare: Vec<(&str, Vec<u8>, bool)> = (0..16)
        .map(|i| {
            let name: &'static str = Box::leak(format!("word/spare{i}.xml").into_boxed_str());
            (name, compressible(8 * 1024 * 1024), true)
        })
        .collect();
    cases.push((
        "128 MB in parts nothing references".into(),
        package(
            document_xml("<w:p><w:r><w:t>small</w:t></w:r></w:p>"),
            &spare,
        ),
        "docx",
    ));

    // Nesting deeper than any document has, in the two places nesting is read:
    // an OOXML part and a bare HTML file.
    let deep_xml = format!("{}text{}", "<w:p>".repeat(20_000), "</w:p>".repeat(20_000));
    cases.push((
        "20,000 nested XML elements".into(),
        package(document_xml(&deep_xml), &[]),
        "docx",
    ));
    let deep_html = format!("{}deep{}", "<div>".repeat(50_000), "</div>".repeat(50_000));
    cases.push((
        "50,000 nested HTML elements".into(),
        deep_html.into_bytes(),
        "html",
    ));

    // Unclosed nesting, which is the same attack without the cost of closing.
    cases.push((
        "50,000 unclosed HTML elements".into(),
        "<div>".repeat(50_000).into_bytes(),
        "html",
    ));

    // A page and an image with dimensions no allocator should honour.
    let huge_pdf = b"%PDF-1.4\n\
        1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj\n\
        2 0 obj<</Type/Pages/Kids[3 0 R]/Count 1>>endobj\n\
        3 0 obj<</Type/Page/Parent 2 0 R/MediaBox[0 0 200000 200000]/Contents 4 0 R\
        /Resources<</XObject<</Im0 5 0 R>>>>>>endobj\n\
        4 0 obj<</Length 30>>stream\nq 1 0 0 1 0 0 cm /Im0 Do Q\nendstream endobj\n\
        5 0 obj<</Type/XObject/Subtype/Image/Width 2000000/Height 2000000\
        /ColorSpace/DeviceRGB/BitsPerComponent 8/Length 4>>stream\n\x00\x00\x00\x00\nendstream endobj\n\
        startxref\n0\n%%EOF"
        .to_vec();
    cases.push((
        "a 200000pt page holding a 4-terapixel image".into(),
        huge_pdf,
        "pdf",
    ));

    // A file whose extension lies. An agent naming a download by its URL gets
    // this for free, and the dispatcher must not be steered by the name into
    // reading bytes as something they are not.
    let a_pdf = seeds()
        .into_iter()
        .find(|(ext, _)| *ext == "pdf")
        .map(|(_, bytes)| bytes)
        .unwrap_or_default();
    cases.push(("a PDF named .docx".into(), a_pdf.clone(), "docx"));
    cases.push(("a PDF named .xlsx".into(), a_pdf.clone(), "xlsx"));
    cases.push(("an empty file named .pdf".into(), Vec::new(), "pdf"));
    cases.push((
        "an archive with no members named .docx".into(),
        agenticpdf::testing::build_zip(&[]),
        "docx",
    ));

    let mut panics: Vec<String> = Vec::new();
    let mut slow: Vec<(Duration, String)> = Vec::new();
    for (what, bytes, ext) in &cases {
        let started = Instant::now();
        let outcome = catch_unwind(AssertUnwindSafe(|| exercise(bytes, ext)));
        let took = started.elapsed();
        let status = match &outcome {
            Ok(status) => status.clone(),
            Err(_) => {
                panics.push(what.clone());
                "PANICKED".to_string()
            }
        };
        eprintln!("  {took:>10.3?}  {what}  [{status}]");
        if took > BUDGET {
            slow.push((took, what.clone()));
        }
    }

    assert!(panics.is_empty(), "panicked:\n  {}", panics.join("\n  "));
    assert!(
        slow.is_empty(),
        "over {BUDGET:?}:\n  {}",
        slow.iter()
            .map(|(d, w)| format!("{d:?} {w}"))
            .collect::<Vec<_>>()
            .join("\n  ")
    );
}
