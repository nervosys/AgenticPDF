// SPDX-License-Identifier: AGPL-3.0-or-later
//! Concealed text is reported, and `--sanitize` actually removes it.
//!
//! The threat is specific: text a person cannot see and a model reads anyway.
//! A document can carry an instruction in white-on-white, at two-point type, in
//! a hidden run, on a slide dropped from the show — all of it perfectly
//! extractable, none of it visible to whoever approved the file.
//!
//! Two properties, both conservation-shaped and both one-directional:
//!
//! - Every run a reader marked hidden is named by `scan`. A reader that can
//!   tell text is concealed and a scan that does not say so is worse than
//!   neither, because the document is then reported clean.
//! - `--sanitize` leaves none of that text in the output. The switch exists so
//!   an agent can be handed a document with the payload gone; text that
//!   survives it is a payload delivered under a promise it was not.
//!
//! Skips when the corpus is absent: the files are gitignored, since Office
//! stamps the author's name into everything it writes.

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

/// A concealed run reduced to its words, joined, so two runs that share a
/// word are still told apart.
///
/// Matching word by word cannot: a document concealing `TINY TEXT PAYLOAD`
/// beside a visible `WHITE TEXT PAYLOAD` shares two words with it, and asking
/// whether "payload" survived sanitising answers yes for the wrong reason.
/// This nearly had me report a hole in the injection defence that was not
/// there.
fn phrase(text: &str) -> String {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

#[test]
fn concealed_text_is_reported_and_sanitising_removes_it() {
    let corpus = corpus();
    if corpus.is_empty() {
        eprintln!("no corpus present; skipping");
        return;
    }

    let mut checked = 0usize;
    let mut concealed = 0usize;
    let mut failures: Vec<String> = Vec::new();

    for path in &corpus {
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        let Ok(document) = Document::open(&bytes) else {
            continue;
        };
        let Some(semantic) = document.semantic() else {
            continue;
        };

        let hidden = semantic.hidden_text();
        if hidden.is_empty() {
            checked += 1;
            continue;
        }
        concealed += hidden.len();

        let report = document.scan();
        let reported = report
            .findings
            .iter()
            .map(|finding| format!("{finding:?}").to_lowercase())
            .collect::<Vec<_>>()
            .join(" ");
        let sanitised = document.sanitized().to_markdown().to_lowercase();

        for (_, text) in &hidden {
            let concealed = phrase(text);
            // Short runs are a styled word inside a sentence, not a payload,
            // and a two-word phrase collides with ordinary prose.
            if concealed.split_whitespace().count() < 3 {
                continue;
            }
            if report.clean || !phrase(&reported).contains(&concealed) {
                failures.push(format!(
                    "{}: concealed {concealed:?} is not in the scan",
                    path.display()
                ));
            }
            if phrase(&sanitised).contains(&concealed) {
                failures.push(format!(
                    "{}: concealed {concealed:?} survived --sanitize",
                    path.display()
                ));
            }
        }
        checked += 1;
    }

    assert!(checked > 0, "the corpus is present but nothing opened");
    assert!(concealed > 0, "no document in the corpus conceals anything");
    failures.sort();
    failures.dedup();
    assert!(
        failures.is_empty(),
        "{} failures:\n{}",
        failures.len(),
        failures
            .iter()
            .take(10)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
    eprintln!("{concealed} concealed runs across {checked} documents, all reported and removed");
}
