// SPDX-License-Identifier: AGPL-3.0-or-later
//! A word in the document can be found in the document.
//!
//! Search is the other half of what an agent does with a document: chunking
//! decides what it can retrieve in bulk, search decides what it can look up.
//! A word that is present and unfindable is worse than one that is absent,
//! because the answer comes back confidently empty.
//!
//! This asserts recall rather than ranking: every word the document holds must
//! bring back at least one result. Nothing here says which result, or in what
//! order — only that the document does not deny holding what it holds.
//!
//! Skips when the corpus is absent: the files are gitignored, since Office
//! stamps the author's name into everything it writes.

use std::collections::BTreeSet;

use irondocuments::document::Document;

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

/// The distinct words a document holds, as a searcher would type them.
fn words(text: &str) -> BTreeSet<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|word| word.chars().count() >= 3)
        .map(str::to_lowercase)
        .collect()
}

#[test]
fn every_word_a_document_holds_can_be_found_in_it() {
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

        let mut missing: Vec<String> = Vec::new();
        for word in words(&text) {
            let hits = irondocuments::agent_ops::search(&bytes, &document, &word)
                .ok()
                .and_then(|value| value.get("hits").cloned())
                .and_then(|hits| hits.as_array().map(|array| array.len()))
                .unwrap_or(0);
            if hits == 0 {
                missing.push(word);
            }
        }
        if !missing.is_empty() {
            failures.push(format!(
                "{}: {} words present but unfindable, e.g. {:?}",
                path.display(),
                missing.len(),
                &missing[..missing.len().min(5)]
            ));
        }
        checked += 1;
    }

    assert!(checked > 0, "the corpus is present but nothing opened");
    assert!(
        failures.is_empty(),
        "search denied holding what the document holds, in {} documents:\n{}",
        failures.len(),
        failures.join("\n")
    );
    eprintln!("{checked} documents answer for every word they hold");
}
