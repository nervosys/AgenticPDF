// SPDX-License-Identifier: AGPL-3.0-or-later
//! The machine-readable surface must describe the tool that actually exists.
//!
//! An agent discovers this tool through `apdf describe` and `apdf formats`
//! rather than by reading `--help`, and it has no way to tell a stale ontology
//! from an accurate one. A command listed but absent is a failed call; a
//! command present but unlisted is a capability nobody finds. Both are worse
//! than a smaller tool honestly described.
//!
//! These tests pin the ontology to the binary so the two cannot drift apart
//! silently.

use std::collections::BTreeSet;

/// Every command the CLI actually exposes, from its own `--help`.
///
/// Parsed rather than hard-coded: a list written here would be a third thing
/// to keep in step, and the point is to have one fewer.
fn commands_from_help() -> BTreeSet<String> {
    let help = command_help();
    let mut out = BTreeSet::new();
    let mut in_commands = false;
    for line in help.lines() {
        if line.starts_with("Commands:") {
            in_commands = true;
            continue;
        }
        if in_commands {
            if line.starts_with("Options:") || line.trim().is_empty() {
                if line.starts_with("Options:") {
                    break;
                }
                continue;
            }
            // "  name    description"
            let trimmed = line.trim_start();
            if line.starts_with("    ") && !line.starts_with("  ") {
                continue;
            }
            if let Some(name) = trimmed.split_whitespace().next()
                && name.chars().all(|c| c.is_ascii_lowercase())
                && line.starts_with("  ")
                && !line.starts_with("   ")
            {
                out.insert(name.to_string());
            }
        }
    }
    out.remove("help");
    out
}

fn command_help() -> String {
    let exe = env!("CARGO_BIN_EXE_apdf");
    let output = std::process::Command::new(exe)
        .arg("--help")
        .output()
        .expect("run apdf --help");
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn describe() -> serde_json::Value {
    let exe = env!("CARGO_BIN_EXE_apdf");
    let output = std::process::Command::new(exe)
        .arg("describe")
        .output()
        .expect("run apdf describe");
    serde_json::from_slice(&output.stdout).expect("describe emits JSON")
}

#[test]
fn the_ontology_lists_exactly_the_commands_the_cli_has() {
    let ontology: BTreeSet<String> = describe()["commands"]
        .as_array()
        .expect("commands is an array")
        .iter()
        .filter_map(|c| c["name"].as_str().map(str::to_string))
        .collect();
    let real = commands_from_help();

    assert!(!real.is_empty(), "parsed no commands from --help");
    let missing: Vec<_> = real.difference(&ontology).collect();
    let extra: Vec<_> = ontology.difference(&real).collect();
    assert!(
        missing.is_empty() && extra.is_empty(),
        "the ontology and the CLI disagree.\n  \
         in --help but not described (a capability nobody finds): {missing:?}\n  \
         described but not in --help (a call that will fail): {extra:?}"
    );
}

#[test]
fn the_ontology_lists_exactly_the_formats_the_build_reads() {
    let described: BTreeSet<String> = describe()["supportedFormats"]
        .as_array()
        .expect("supportedFormats is an array")
        .iter()
        .filter_map(|f| {
            f.get("id")
                .and_then(|v| v.as_str())
                .or_else(|| f.as_str())
                .map(str::to_string)
        })
        .collect();

    let supported: BTreeSet<String> = agenticpdf::detect::Format::all()
        .iter()
        .filter(|format| format.is_supported())
        .map(|format| format.id().to_string())
        .collect();

    assert_eq!(
        described, supported,
        "the ontology's format list does not match what the build can read"
    );
}

/// The commands that work on any supported document must say so, and the ones
/// that do not must say that instead.
///
/// Checked by running each against a non-PDF document rather than by reading
/// the text: a description is a claim, and this is the claim being true.
#[test]
fn every_command_either_reads_any_document_or_says_it_is_pdf_only() {
    // A minimal Markdown file stands in for "not a PDF"; every reader path
    // reaches the same semantic model, so the format hardly matters.
    let dir = std::env::temp_dir().join("apdf-agent-surface");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("sample.md");
    std::fs::write(&path, b"# Title\n\nBody text.\n\n- one\n- two\n").expect("write sample");

    let exe = env!("CARGO_BIN_EXE_apdf");
    let help = command_help();
    let mut wrong: Vec<String> = Vec::new();

    for name in commands_from_help() {
        // These take no document, or take one only in a mode this cannot drive.
        if matches!(
            name.as_str(),
            "formats" | "describe" | "info" | "mcp" | "verify" | "search" | "displaylist"
        ) {
            continue;
        }
        let output = std::process::Command::new(exe)
            .arg(&name)
            .arg(&path)
            .output()
            .expect("run command");
        let combined = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let refused = combined.contains("Unsupported");

        // What the help promises, from the command's own line.
        let line = help
            .lines()
            .find(|l| l.trim_start().starts_with(&format!("{name} ")))
            .unwrap_or_default();
        let claims_pdf_only = line.contains("PDF only");

        if refused != claims_pdf_only {
            wrong.push(format!(
                "{name}: help {} but the command {}",
                match claims_pdf_only {
                    true => "says PDF only",
                    false => "offers any document",
                },
                match refused {
                    true => "refused a Markdown file",
                    false => "accepted one",
                }
            ));
        }
    }

    let _ = std::fs::remove_dir_all(&dir);
    assert!(wrong.is_empty(), "{}", wrong.join("\n  "));
}
