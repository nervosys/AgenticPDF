// SPDX-License-Identifier: AGPL-3.0-or-later
//! Chunking must not lose the document.
//!
//! A retrieval pipeline never sees the document; it sees the chunks. So a word
//! that falls between two of them is a word that cannot be retrieved, cited or
//! quoted, and nothing downstream can tell that it is missing — the answer
//! simply comes back without it.
//!
//! This asserts conservation rather than any particular chunking: whatever the
//! splitter decides, every word of the document has to land in at least one
//! chunk. That holds for any sensible chunk size, so it is checked at several,
//! including sizes far smaller than a paragraph.
//!
//! Skips when the corpus is absent: the files are gitignored, since Office
//! stamps the author's name into everything it writes.

use std::collections::HashMap;

use agenticpdf::document::Document;

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

/// How many times each word occurs, so a missing word is visible even when
/// another copy of it survives elsewhere.
fn words(text: &str) -> HashMap<String, usize> {
    let mut counts = HashMap::new();
    // Split on punctuation as well as space, because a placeholder drawn for
    // an object -- `[Shape1]` where a picture sits -- brackets a word that a
    // reader is looking for, and brackets are not part of it.
    for word in text.split(|c: char| !c.is_alphanumeric()) {
        if !word.is_empty() {
            *counts.entry(word.to_string()).or_insert(0) += 1;
        }
    }
    counts
}

#[test]
fn every_word_of_a_document_lands_in_a_chunk() {
    let corpus = corpus();
    if corpus.is_empty() {
        eprintln!("no corpus present; skipping");
        return;
    }

    let mut checked = 0usize;
    let mut failures: Vec<String> = Vec::new();

    for path in &corpus {
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        let Ok(document) = Document::open(&bytes) else {
            continue;
        };
        let text = document.extract_text();
        if text.trim().is_empty() {
            continue;
        }
        let wanted = words(&text);

        // Several sizes, including ones smaller than a paragraph, because a
        // splitter that never has to split cannot drop anything at a seam.
        for size in [64usize, 256, 512, 4096] {
            let chunks = document.generate_chunks(size, size / 8);
            let joined: String = chunks
                .iter()
                .map(|chunk| chunk.content.as_str())
                .collect::<Vec<_>>()
                .join(" ");
            let got = words(&joined);

            let mut missing: Vec<&String> = wanted
                .iter()
                .filter(|(word, count)| got.get(*word).copied().unwrap_or(0) < **count)
                .map(|(word, _)| word)
                .collect();
            missing.sort();
            if !missing.is_empty() {
                failures.push(format!(
                    "{} at size {size}: {} of {} words short, e.g. {:?}",
                    path.display(),
                    missing.len(),
                    wanted.len(),
                    &missing[..missing.len().min(4)]
                ));
            }
        }
        checked += 1;
    }

    assert!(checked > 0, "the corpus is present but nothing opened");
    assert!(
        failures.is_empty(),
        "chunking lost text in {} cases:\n{}",
        failures.len(),
        failures.join("\n")
    );
    eprintln!("{checked} documents chunk without losing a word");
}
