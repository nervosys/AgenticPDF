// SPDX-License-Identifier: AGPL-3.0-or-later
//! Operations shared by every agent-facing surface.
//!
//! Search, provenance checking and conversion are reachable three ways — the
//! CLI, the MCP server, and the reader app — and they must give the same
//! answers through all three. So they live here once rather than being
//! implemented per entry point. The alternative, which this module exists to
//! prevent, is the failure mode where a capability quietly means something
//! different depending on how it was invoked, and the ontology describing it
//! becomes wrong without anyone editing it.
//!
//! Results are `serde_json::Value` because two of the three callers serialise
//! them; the CLI renders the same data as text.

use serde_json::{Value, json};

use crate::adf::provenance::Verification;
use crate::adf::{AdfDoc, AdfWriter};
use crate::detect::Format;
use crate::document::Document;
use crate::{PdfError, doc};

/// Find blocks matching every term in `query`.
///
/// ADF answers from the index inside the file; every other format is scanned.
/// The distinction is invisible on purpose — an agent should not have to
/// establish what kind of file it holds before it can look something up.
pub fn search(data: &[u8], document: &Document, query: &str) -> Result<Value, PdfError> {
    let hits: Vec<Value> = if AdfDoc::sniff(data) {
        AdfDoc::open(data)?
            .search(query)?
            .into_iter()
            .map(|(chunk, text)| {
                json!({ "section": chunk.section, "block": chunk.blocks[0], "text": text })
            })
            .collect()
    } else {
        let Some(semantic) = document.semantic() else {
            return Err(PdfError::Unsupported(format!(
                "{} has no semantic model to search",
                document.format().label()
            )));
        };
        let terms: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
        if terms.is_empty() {
            return Ok(json!({ "hits": [] }));
        }

        let mut found = Vec::new();
        for (section, division) in semantic.sections.iter().enumerate() {
            for (block, content) in division.blocks.iter().enumerate() {
                let mut text = String::new();
                doc::block_text_into(content, &mut text);
                let haystack = text.to_lowercase();
                if terms.iter().all(|term| haystack.contains(term.as_str())) {
                    found.push(json!({
                        "section": section,
                        "block": block,
                        "text": text.trim(),
                    }));
                }
            }
        }
        found
    };

    Ok(json!({ "hits": hits }))
}

/// Check a quotation against recorded provenance.
pub fn verify(data: &[u8], text: &str, section: u32, block: u32) -> Result<Value, PdfError> {
    if !AdfDoc::sniff(data) {
        return Err(PdfError::Unsupported(
            "only ADF documents carry provenance; convert to adf first".into(),
        ));
    }
    let adf = AdfDoc::open(data)?;

    Ok(match adf.provenance().verify(section, block, text) {
        Verification::Matches(row) => json!({
            "status": "matched",
            "source_page": row.page,
            "authored_here": row.is_authored(),
        }),
        Verification::Drifted(row) => json!({
            "status": "drifted",
            "source_page": row.page,
            "detail": "the block exists but its text has changed since import",
        }),
        Verification::Unrecorded => json!({
            "status": "unrecorded",
            "detail": "no provenance was stored for this block",
        }),
    })
}

/// Convert to another format, returning bytes.
///
/// Bytes rather than a string because ADF is binary; every other target is
/// UTF-8 text that a caller can decode.
pub fn convert(document: &Document, source: &str, to: &str) -> Result<Vec<u8>, PdfError> {
    Ok(match to.to_ascii_lowercase().as_str() {
        "markdown" | "md" | "gfm" => document.to_markdown().into_bytes(),
        "html" => document.to_html().into_bytes(),
        "text" | "txt" => document.extract_text().into_bytes(),
        // Derived where the format has no authored structure, so a PDF -- the
        // format most likely to be imported for retrieval -- converts rather
        // than being refused for lacking a model it never had.
        "adf" => write_adf(&document.semantic_view(), source, document.format()),
        other => {
            return Err(PdfError::Unsupported(format!(
                "target format '{other}' (expected adf, markdown, html or text)"
            )));
        }
    })
}

/// Encode a semantic document as ADF, recording provenance as it imports.
///
/// Recording here rather than leaving it to callers is what makes verification
/// meaningful: a document written without this step reports every quotation as
/// "unrecorded", which is indistinguishable from provenance being genuinely
/// absent.
pub fn write_adf(semantic: &doc::SemanticDoc, source: &str, format: Format) -> Vec<u8> {
    let mut writer = AdfWriter::new();
    let source_id = writer.intern_source(source);

    for (section, division) in semantic.sections.iter().enumerate() {
        for (block, content) in division.blocks.iter().enumerate() {
            let mut text = String::new();
            doc::block_text_into(content, &mut text);
            writer.add_provenance(crate::adf::provenance::Provenance {
                section: section as u32,
                block: block as u32,
                source: source_id,
                page: 0,
                // No geometry: the semantic model does not carry it, and an
                // invented bounding box is worse than an absent one.
                bbox: [0.0; 4],
                hash: crate::adf::provenance::Provenance::hash_text(text.trim()),
            });
        }
    }
    writer.write(semantic, format.id())
}

#[cfg(test)]
mod tests {
    use super::*;

    const MARKDOWN: &[u8] =
        b"# Report\n\nRevenue grew across EMEA.\n\n- Hiring on plan\n- Churn down in APAC\n";

    fn adf_bytes() -> Vec<u8> {
        let document = Document::open(MARKDOWN).unwrap();
        convert(&document, "report.md", "adf").unwrap()
    }

    #[test]
    fn search_gives_the_same_answer_through_the_index_and_the_scan() {
        // The property that matters: an agent must not get different results
        // depending on which format the document happens to be in.
        let source = Document::open(MARKDOWN).unwrap();
        let scanned = search(MARKDOWN, &source, "revenue emea").unwrap();

        let bytes = adf_bytes();
        let imported = Document::open(&bytes).unwrap();
        let indexed = search(&bytes, &imported, "revenue emea").unwrap();

        let text = |value: &Value| value["hits"][0]["text"].as_str().unwrap().to_string();
        assert_eq!(scanned["hits"].as_array().unwrap().len(), 1);
        assert_eq!(indexed["hits"].as_array().unwrap().len(), 1);
        assert_eq!(text(&scanned), text(&indexed));
    }

    #[test]
    fn every_query_term_must_match() {
        let document = Document::open(MARKDOWN).unwrap();
        let hits = search(MARKDOWN, &document, "revenue hiring").unwrap();
        assert!(hits["hits"].as_array().unwrap().is_empty());
    }

    #[test]
    fn a_hit_names_the_block_it_came_from() {
        let document = Document::open(MARKDOWN).unwrap();
        let hits = search(MARKDOWN, &document, "revenue").unwrap();
        // Without section and block a caller cannot verify what it just found.
        assert!(hits["hits"][0]["section"].is_number());
        assert!(hits["hits"][0]["block"].is_number());
    }

    #[test]
    fn importing_records_provenance_so_quotations_can_be_checked() {
        let bytes = adf_bytes();
        let verdict = verify(&bytes, "Revenue grew across EMEA.", 0, 1).unwrap();
        assert_eq!(verdict["status"], "matched");
    }

    #[test]
    fn an_altered_quotation_reads_as_drifted_not_as_absent() {
        let bytes = adf_bytes();
        let verdict = verify(&bytes, "Revenue grew across APAC.", 0, 1).unwrap();
        assert_eq!(verdict["status"], "drifted");
    }

    #[test]
    fn an_unknown_block_reads_as_unrecorded() {
        let bytes = adf_bytes();
        let verdict = verify(&bytes, "anything", 99, 99).unwrap();
        assert_eq!(verdict["status"], "unrecorded");
    }

    #[test]
    fn verifying_a_non_adf_document_says_why_rather_than_failing_obscurely() {
        let error = verify(MARKDOWN, "Revenue grew across EMEA.", 0, 1).unwrap_err();
        assert!(
            matches!(&error, PdfError::Unsupported(why) if why.contains("convert to adf")),
            "got {error:?}"
        );
    }

    #[test]
    fn convert_rejects_a_target_the_engine_cannot_write() {
        let document = Document::open(MARKDOWN).unwrap();
        assert!(convert(&document, "report.md", "docx").is_err());
        assert!(convert(&document, "report.md", "markdown").is_ok());
    }
}
