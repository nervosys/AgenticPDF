// SPDX-License-Identifier: AGPL-3.0-or-later
//! The ontology an agent discovers the app through.
//!
//! This is the app's contract with agents, and the reason it exists rather than
//! a chat panel: an agent asks the running program what it can do and gets a
//! machine-readable answer, instead of relying on a description that drifts
//! from the code. Dewey serves these over its JSON-Lines protocol
//! (`query_ontology`, `get_schema`, `get_tree`, `get_state`).
//!
//! The rule to hold to as this grows: **every capability here must be a call
//! into `session::Session`**, the same one the buttons make. An action that
//! exists only for agents is a second implementation waiting to disagree with
//! the first.

use dewey::ontology::{OntologyRegistry, WidgetSchema};
use dewey::prelude::SemanticRole;

/// Register the app's schemas.
pub fn register(registry: &mut OntologyRegistry) {
    registry.register_schema(WidgetSchema::new(
        "DocumentReader",
        "Reads 17 document formats and edits its own (ADF). Open a file, \
         navigate pages, search, edit blocks, and save or export.",
        SemanticRole::Container,
    ));

    registry.register_schema(WidgetSchema::new(
        "DocumentCanvas",
        "The typeset page. Reports the page number and its operation count. \
         Formats that carry no geometry report that rather than showing an \
         invented layout.",
        SemanticRole::Display,
    ));

    registry.register_schema(WidgetSchema::new(
        "DocumentOutline",
        "The document as an ordered list of editable blocks. Blocks written by \
         an agent are prefixed with the actor that wrote them.",
        SemanticRole::Selection,
    ));

    registry.register_schema(WidgetSchema::new(
        "DocumentSearch",
        "Full-text search. ADF documents answer from the index stored inside \
         the file; other formats are scanned. All query terms must match.",
        SemanticRole::Input,
    ));
}

/// A machine-readable description of what the app can be asked to do.
///
/// Dewey's registry describes the *widgets*; this describes the *operations*,
/// which is what an agent planning a change actually needs. Kept as data rather
/// than prose so it can be served verbatim and diffed when it changes.
pub fn capabilities() -> serde_json::Value {
    serde_json::json!({
        "application": "apdf-reader",
        "reads": agenticpdf::detect::Format::all()
            .iter()
            .map(|format| format.id())
            .collect::<Vec<_>>(),
        "writes": ["adf", "markdown", "html", "text"],
        // Taken from the dispatcher's own list, so this cannot advertise an
        // action the app does not implement.
        "actions": crate::actions::ACTIONS,
        "operations": [
            {
                "name": "open",
                "summary": "Open a document from a path.",
                "arguments": { "path": "string" }
            },
            {
                "name": "search",
                "summary": "Find blocks matching every term in a query.",
                "arguments": { "query": "string" },
                "returns": "hits with section, block, page and text"
            },
            {
                "name": "search_similar",
                "summary": "Rank blocks by embedding similarity. Requires a \
                            document that carries embeddings.",
                "arguments": { "vector": "array of f32", "limit": "integer" }
            },
            {
                "name": "insert_block",
                "summary": "Insert a block after another, or at the start.",
                "arguments": { "after": "op id or null", "markdown": "string" },
                "attribution": "recorded against the agent actor"
            },
            {
                "name": "replace_block",
                "summary": "Replace a block's content, last-writer-wins.",
                "arguments": { "target": "op id", "markdown": "string" }
            },
            { "name": "delete_block", "arguments": { "target": "op id" } },
            {
                "name": "save",
                "summary": "Write ADF. An ADF document is appended to rather \
                            than rewritten, so the cost is the size of the edit."
            },
            {
                "name": "export",
                "arguments": { "to": "markdown | html | text" }
            }
        ],
        "concurrency": {
            "model": "append-only CRDT log, block granularity",
            "merges": "edits to different blocks always merge",
            "conflicts": "edits to the same block resolve last-writer-wins by \
                          Lamport clock; one is discarded",
            "attribution": "every operation records its actor, including \
                            whether it was an agent"
        },
        "provenance": {
            "summary": "Imported blocks keep a content hash and their source \
                        page and bounding box.",
            "verify": "a quotation is reported as matching, drifted, or \
                       unrecorded — never guessed"
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_registry_describes_every_pane_the_app_shows() {
        let mut registry = OntologyRegistry::new();
        register(&mut registry);

        for widget in ["DocumentReader", "DocumentCanvas", "DocumentOutline", "DocumentSearch"] {
            assert!(
                registry.get_schema(widget).is_some(),
                "{widget} is not discoverable by an agent"
            );
        }
    }

    #[test]
    fn capabilities_advertise_only_formats_the_engine_really_reads() {
        let capabilities = capabilities();
        let reads = capabilities["reads"].as_array().unwrap();

        assert_eq!(reads.len(), agenticpdf::detect::Format::all().len());
        assert!(reads.iter().any(|f| f == "adf"));
        assert!(reads.iter().any(|f| f == "pdf"));
        // Writing is deliberately narrower than reading; advertising otherwise
        // would have agents attempt saves that cannot work.
        let writes = capabilities["writes"].as_array().unwrap();
        assert!(!writes.iter().any(|f| f == "docx"));
    }

    #[test]
    fn every_documented_operation_is_a_real_action() {
        // The two lists are written separately — the prose descriptions here
        // and the dispatcher's `ACTIONS` — so this is what stops the ontology
        // describing an operation the app cannot perform.
        let capabilities = capabilities();
        let documented: Vec<&str> = capabilities["operations"]
            .as_array()
            .unwrap()
            .iter()
            .map(|op| op["name"].as_str().unwrap())
            .collect();

        for name in documented {
            assert!(
                crate::actions::ACTIONS.contains(&name),
                "'{name}' is documented but not dispatchable"
            );
        }
    }

    #[test]
    fn the_conflict_model_is_stated_rather_than_implied() {
        // An agent needs to know that same-block edits lose data. If this key
        // ever disappears, the ontology has started overpromising.
        let capabilities = capabilities();
        let conflicts = capabilities["concurrency"]["conflicts"].as_str().unwrap();
        assert!(conflicts.contains("discarded"));
    }
}
