// SPDX-License-Identifier: AGPL-3.0-or-later
//! What this library hands an agent, it will vouch for.
//!
//! The pipeline an agent actually follows is: search the document, quote what
//! came back, cite where it came from. So the property is end to end — every
//! hit `search` returns must verify as `matched` at the locator `search` gave
//! for it. Anything else means the library handed over a line and then refused
//! to stand behind it.
//!
//! `verify` answers matched, drifted or unrecorded, and the wrong one is worse
//! than none: `drifted` says the source has been edited since import, which is
//! a claim about the document's history, and a locator naming two different
//! pieces of text makes that claim falsely.
//!
//! Skips when the corpus is absent: the files are gitignored, since Office
//! stamps the author's name into everything it writes.

use std::collections::BTreeSet;

use agenticpdf::agent_ops;
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

/// The words a searcher would type, as a set so each is tried once.
fn words(text: &str) -> BTreeSet<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|word| word.chars().count() >= 4)
        .map(str::to_lowercase)
        .collect()
}

#[test]
fn every_line_search_hands_over_can_then_be_cited() {
    let corpus = corpus();
    if corpus.is_empty() {
        eprintln!("no corpus present; skipping");
        return;
    }

    let mut checked = 0usize;
    let mut citations = 0usize;
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
        // Converted to the format that carries provenance, which is what an
        // agent meaning to cite anything would do first.
        let semantic = document.semantic_view().into_owned();
        let adf = agent_ops::write_adf(&semantic, "corpus", document.format());

        for word in words(&text) {
            let Ok(found) = agent_ops::search(&bytes, &document, &word) else {
                continue;
            };
            let Some(hits) = found["hits"].as_array() else {
                continue;
            };
            for hit in hits {
                let (Some(section), Some(block), Some(quotation)) = (
                    hit["section"].as_u64(),
                    hit["block"].as_u64(),
                    hit["text"].as_str(),
                ) else {
                    continue;
                };
                if quotation.trim().is_empty() {
                    continue;
                }
                citations += 1;

                let answer =
                    agent_ops::verify(&adf, quotation, section as u32, block as u32).unwrap();
                if answer["status"] != "matched" {
                    failures.push(format!(
                        "{} [{section}:{block}] {} {}: {:?}",
                        path.display(),
                        hit["kind"],
                        answer["status"],
                        quotation.chars().take(44).collect::<String>()
                    ));
                }
            }
        }
        checked += 1;
    }

    assert!(checked > 0, "the corpus is present but nothing opened");
    failures.sort();
    failures.dedup();
    assert!(
        failures.is_empty(),
        "{} citations were refused by the library that gave them:\n{}",
        failures.len(),
        failures
            .iter()
            .take(10)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
    eprintln!("{citations} citations from {checked} documents all verified");
}
