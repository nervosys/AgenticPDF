// SPDX-License-Identifier: AGPL-3.0-or-later
//! Every real document survives the archival format.
//!
//! ADF is where a document goes to be kept, so anything it loses is lost for
//! good. The model-level round trip in `src/adf/tests.rs` builds a document
//! holding one of everything; this one takes whatever real producers actually
//! wrote, which is where the constructs nobody thought to hand-build turn up.
//!
//! The same file checks the JSON surface, for the same reason and with the
//! same corpus.
//!
//! Skips when the corpus is absent: the files are gitignored, since Office
//! stamps the author's name into everything it writes.

use agenticpdf::adf::{AdfDoc, AdfWriter};
use agenticpdf::doc::{to_html, to_markdown};
use agenticpdf::document::Document;

/// Every corpus file, ignoring the ones that are not documents.
fn corpus() -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    for directory in ["tests/fixtures", "tests/fixtures/lo"] {
        let Ok(entries) = std::fs::read_dir(directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let extension = path
                .extension()
                .and_then(|value| value.to_str())
                .unwrap_or_default()
                .to_ascii_lowercase();
            if path.is_file() && !matches!(extension.as_str(), "png" | "gif" | "") {
                out.push(path);
            }
        }
    }
    out.sort();
    out
}

#[test]
fn every_corpus_document_survives_adf_and_json() {
    let corpus = corpus();
    if corpus.is_empty() {
        eprintln!("no corpus present; skipping");
        return;
    }

    let mut checked = 0usize;
    for path in &corpus {
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        let Ok(document) = Document::open(&bytes) else {
            continue;
        };
        let Some(original) = document.semantic() else {
            continue;
        };

        let written = AdfWriter::new().write(original, "test");
        let reopened = AdfDoc::open(&written)
            .unwrap_or_else(|error| panic!("{}: reading back failed: {error}", path.display()));
        let restored = reopened
            .to_semantic()
            .unwrap_or_else(|error| panic!("{}: decoding failed: {error}", path.display()));

        // Two renderings rather than one: Markdown compares the block tree in
        // a form a person can read a diff of, and HTML carries what Markdown
        // has no mark for -- which is exactly where a codec quietly drops a
        // property nobody looks at.
        assert_eq!(
            to_markdown(&restored),
            to_markdown(original),
            "{}: Markdown differs after a round trip",
            path.display()
        );
        assert_eq!(
            to_html(&restored),
            to_html(original),
            "{}: HTML differs after a round trip",
            path.display()
        );
        assert_eq!(
            restored.assets.len(),
            original.assets.len(),
            "{}: assets differ",
            path.display()
        );
        for (after, before) in restored.assets.iter().zip(&original.assets) {
            assert_eq!(after.bytes, before.bytes, "{}: asset bytes", path.display());
        }
        // The JSON surface is what an agent consumes, and a field left out of
        // it is a field the agent never sees. `skip_serializing_if` is what
        // makes that easy to do by accident.
        let json = serde_json::to_string(original)
            .unwrap_or_else(|error| panic!("{}: serialising failed: {error}", path.display()));
        let from_json: agenticpdf::doc::SemanticDoc = serde_json::from_str(&json)
            .unwrap_or_else(|error| panic!("{}: parsing back failed: {error}", path.display()));
        assert_eq!(
            to_html(&from_json),
            to_html(original),
            "{}: HTML differs after a round trip through JSON",
            path.display()
        );

        checked += 1;
    }
    assert!(checked > 0, "the corpus is present but nothing opened");
    eprintln!("{checked} documents survived a round trip through ADF and JSON");
}
