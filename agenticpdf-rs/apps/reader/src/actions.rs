// SPDX-License-Identifier: AGPL-3.0-or-later
//! Agent actions, routed into the same session the user interface drives.
//!
//! Dewey's protocol gives an agent `query_ontology` to learn what exists and
//! `execute_action` to do something about it. This module is the second half.
//!
//! The rule it enforces: **every action here calls
//! [`Session`](crate::session::Session)** — the same methods the buttons call.
//! Nothing is reimplemented for agents. That is what keeps the ontology honest
//! as the app grows, because a capability cannot drift from its description
//! when there is only one implementation of it.
//!
//! Errors come back as data, not as a transport failure: an agent that asks for
//! page 900 of a 10-page document should be told the document has 10 pages, not
//! handed a protocol error it cannot interpret.

use serde_json::{Value, json};

use crate::session::{ACTOR_AGENT, Hit, Session};
use agenticpdf::adf::oplog::OpId;
use agenticpdf::doc::Block;

/// Every action name the app answers to.
///
/// Listed as data so `ontology::capabilities` and the dispatcher below cannot
/// disagree about what exists — a test asserts they match.
pub const ACTIONS: &[&str] = &[
    "open",
    "open_bytes",
    "search",
    "search_similar",
    "get_blocks",
    "insert_block",
    "replace_block",
    "delete_block",
    "goto_page",
    "save",
    "export",
    "capabilities",
];

/// Run an action against the session, returning the agent's result.
///
/// `session` is `None` before a document is open; actions needing one say so
/// rather than silently doing nothing.
pub fn execute(session: &mut Option<Session>, action: &str, params: &Value) -> Value {
    // `open` is the only action that can run without a document.
    if action == "open" {
        let Some(path) = params.get("path").and_then(Value::as_str) else {
            return error("open requires a 'path' parameter");
        };
        return match read_document(path) {
            Ok(bytes) => match Session::open(bytes) {
                Ok(opened) => {
                    let summary = describe(&opened);
                    *session = Some(opened);
                    summary
                }
                Err(why) => error(&format!("could not open {path}: {why}")),
            },
            Err(why) => error(&why),
        };
    }
    // Opening from bytes works everywhere, and is the only way in on platforms
    // with no filesystem — a browser, or Android handing back a content URI.
    if action == "open_bytes" {
        let Some(values) = params.get("bytes").and_then(Value::as_array) else {
            return error("open_bytes requires a 'bytes' array");
        };
        let bytes: Vec<u8> = values
            .iter()
            .filter_map(|value| value.as_u64().map(|byte| byte as u8))
            .collect();
        if bytes.len() != values.len() {
            return error("'bytes' must contain only integers in 0..=255");
        }
        return match Session::open(bytes) {
            Ok(opened) => {
                let summary = describe(&opened);
                *session = Some(opened);
                summary
            }
            Err(why) => error(&format!("could not open the supplied bytes: {why}")),
        };
    }
    if action == "capabilities" {
        return crate::ontology::capabilities();
    }

    let Some(session) = session.as_mut() else {
        return error("no document is open; call 'open' first");
    };

    match action {
        "search" => match params.get("query").and_then(Value::as_str) {
            Some(query) => json!({ "hits": hits_to_json(&session.search(query)) }),
            None => error("search requires a 'query' parameter"),
        },

        "search_similar" => {
            let Some(vector) = params.get("vector").and_then(Value::as_array) else {
                return error("search_similar requires a 'vector' parameter");
            };
            let vector: Vec<f32> = vector
                .iter()
                .filter_map(|value| value.as_f64().map(|v| v as f32))
                .collect();
            let limit = params
                .get("limit")
                .and_then(Value::as_u64)
                .unwrap_or(10)
                .min(1000) as usize;

            let hits = session.search_similar(&vector, limit);
            if hits.is_empty() {
                // Distinguish "nothing matched" from "this document has no
                // embeddings", because the fix differs completely.
                return json!({
                    "hits": [],
                    "note": "no matches, or this document carries no embeddings"
                });
            }
            json!({ "hits": hits_to_json(&hits) })
        }

        "get_blocks" => json!({
            "blocks": session
                .blocks()
                .iter()
                .enumerate()
                .map(|(index, (id, block))| {
                    let mut text = String::new();
                    agenticpdf::doc::block_text_into(block, &mut text);
                    let (author, is_agent) = session
                        .attribution(*id)
                        .map(|(name, is_agent, _)| (name, is_agent))
                        .unwrap_or_else(|| ("unknown".to_string(), false));
                    json!({
                        "index": index,
                        "id": op_id_to_json(*id),
                        "text": text.trim(),
                        "author": author,
                        "by_agent": is_agent,
                    })
                })
                .collect::<Vec<_>>()
        }),

        "insert_block" => {
            let Some(markdown) = params.get("markdown").and_then(Value::as_str) else {
                return error("insert_block requires a 'markdown' parameter");
            };
            let after = params.get("after").and_then(op_id_from_json);
            // Blocks arrive as Markdown because that is what a model writes
            // fluently; parsing it here means an agent gets real headings,
            // lists and tables rather than one flat paragraph.
            let blocks = parse_markdown_blocks(markdown);
            if blocks.is_empty() {
                return error("'markdown' parsed to no blocks");
            }

            let mut left = after;
            let mut inserted = Vec::new();
            for block in blocks {
                let id = session.insert_block(ACTOR_AGENT, left, block);
                inserted.push(op_id_to_json(id));
                left = Some(id);
            }
            json!({ "inserted": inserted })
        }

        "replace_block" => {
            let Some(target) = params.get("target").and_then(op_id_from_json) else {
                return error("replace_block requires a 'target' op id");
            };
            let Some(markdown) = params.get("markdown").and_then(Value::as_str) else {
                return error("replace_block requires a 'markdown' parameter");
            };
            let Some(block) = parse_markdown_blocks(markdown).into_iter().next() else {
                return error("'markdown' parsed to no blocks");
            };
            json!({ "replaced": op_id_to_json(session.replace_block(ACTOR_AGENT, target, block)) })
        }

        "delete_block" => match params.get("target").and_then(op_id_from_json) {
            Some(target) => {
                json!({ "deleted": op_id_to_json(session.delete_block(ACTOR_AGENT, target)) })
            }
            None => error("delete_block requires a 'target' op id"),
        },

        "goto_page" => match params.get("page").and_then(Value::as_u64) {
            Some(page) => {
                session.go_to_page(page as usize);
                json!({ "page": session.page(), "page_count": session.page_count() })
            }
            None => error("goto_page requires a 'page' parameter"),
        },

        "save" => {
            let bytes = match session.save() {
                Ok(bytes) => bytes,
                Err(why) => return error(&format!("could not save: {why}")),
            };
            let path = params
                .get("path")
                .and_then(Value::as_str)
                .unwrap_or("document.adf");
            write_document(path, &bytes)
        }

        "export" => {
            let to = params
                .get("to")
                .and_then(Value::as_str)
                .unwrap_or("markdown");
            match session.export(to) {
                Ok(text) => json!({ "format": to, "content": text }),
                Err(why) => error(&why.to_string()),
            }
        }

        other => error(&format!(
            "unknown action '{other}'; call 'capabilities' for the list"
        )),
    }
}

// Path-based I/O is the only part of this module that is not portable, so it is
// isolated to these two functions rather than spread through the dispatcher. A
// browser has no paths and Android hands back content URIs, so on those
// platforms the shell reads and writes, and the core moves bytes.

#[cfg(not(target_arch = "wasm32"))]
fn read_document(path: &str) -> Result<Vec<u8>, String> {
    std::fs::read(path).map_err(|why| format!("could not read {path}: {why}"))
}

#[cfg(target_arch = "wasm32")]
fn read_document(_path: &str) -> Result<Vec<u8>, String> {
    Err("this platform has no filesystem; use 'open_bytes' instead".to_string())
}

#[cfg(not(target_arch = "wasm32"))]
fn write_document(path: &str, bytes: &[u8]) -> Value {
    match std::fs::write(path, bytes) {
        Ok(()) => json!({ "path": path, "bytes": bytes.len() }),
        Err(why) => error(&format!("could not write {path}: {why}")),
    }
}

/// The document is still serialised — the shell hands the bytes to a download
/// or a share sheet. Reporting a path that was never written would be a lie.
#[cfg(target_arch = "wasm32")]
fn write_document(_path: &str, bytes: &[u8]) -> Value {
    json!({
        "bytes": bytes.len(),
        "note": "returned to the host; this platform has no filesystem"
    })
}

/// Parse Markdown into blocks, reusing the engine's own reader.
fn parse_markdown_blocks(markdown: &str) -> Vec<Block> {
    agenticpdf::formats::text::parse_markdown(markdown.as_bytes())
        .sections
        .into_iter()
        .flat_map(|section| section.blocks)
        .collect()
}

fn describe(session: &Session) -> Value {
    json!({
        "title": session.title(),
        "format": session.format().id(),
        "pages": session.page_count(),
        "blocks": session.blocks().len(),
    })
}

fn hits_to_json(hits: &[Hit]) -> Vec<Value> {
    hits.iter()
        .map(|hit| {
            json!({
                "section": hit.section,
                "block": hit.block,
                "page": hit.page,
                "text": hit.text,
                "score": hit.score,
            })
        })
        .collect()
}

/// Op ids cross the wire as a pair, not a string.
///
/// A string would need parsing on the way back, and a malformed one would be
/// indistinguishable from a node that no longer exists.
fn op_id_to_json(id: OpId) -> Value {
    json!({ "counter": id.counter, "actor": id.actor })
}

fn op_id_from_json(value: &Value) -> Option<OpId> {
    Some(OpId {
        counter: value.get("counter")?.as_u64()?,
        actor: value.get("actor")?.as_u64()?,
    })
}

fn error(message: &str) -> Value {
    json!({ "error": message })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opened() -> Option<Session> {
        Some(
            Session::open(b"# Report\n\nRevenue grew across EMEA.\n\n- Hiring on plan\n".to_vec())
                .unwrap(),
        )
    }

    #[test]
    fn actions_needing_a_document_say_so_instead_of_doing_nothing() {
        let mut none = None;
        let result = execute(&mut none, "search", &json!({ "query": "x" }));
        assert!(result["error"].as_str().unwrap().contains("no document"));
    }

    #[test]
    fn a_document_can_be_opened_from_bytes_on_any_platform() {
        let mut none = None;
        let bytes: Vec<serde_json::Value> = b"# Title

Body text.
"
        .iter()
        .map(|b| json!(b))
        .collect();

        let result = execute(&mut none, "open_bytes", &json!({ "bytes": bytes }));
        assert_eq!(result["title"], "Title");
        assert_eq!(result["format"], "markdown");
        assert!(none.is_some(), "the session was not opened");
    }

    #[test]
    fn open_bytes_rejects_a_non_byte_array() {
        let mut none = None;
        let result = execute(&mut none, "open_bytes", &json!({ "bytes": [1, "two", 3] }));
        assert!(result["error"].as_str().unwrap().contains("integers"));
    }

    #[test]
    fn capabilities_answers_without_a_document() {
        let mut none = None;
        let result = execute(&mut none, "capabilities", &json!({}));
        assert_eq!(result["application"], "apdf-reader");
    }

    #[test]
    fn an_unknown_action_names_the_way_to_discover_the_real_ones() {
        let mut session = opened();
        let result = execute(&mut session, "frobnicate", &json!({}));
        assert!(result["error"].as_str().unwrap().contains("capabilities"));
    }

    #[test]
    fn a_missing_parameter_is_reported_rather_than_defaulted() {
        let mut session = opened();
        for (action, params) in [
            ("search", json!({})),
            ("insert_block", json!({})),
            ("goto_page", json!({})),
            ("delete_block", json!({})),
        ] {
            let result = execute(&mut session, action, &params);
            assert!(
                result.get("error").is_some(),
                "{action} accepted missing parameters"
            );
        }
    }

    #[test]
    fn search_returns_citable_hits() {
        let mut session = opened();
        let result = execute(&mut session, "search", &json!({ "query": "revenue emea" }));

        let hits = result["hits"].as_array().unwrap();
        assert_eq!(hits.len(), 1);
        assert!(hits[0]["text"].as_str().unwrap().contains("EMEA"));
        assert!(hits[0]["block"].is_number(), "a hit must name its block");
    }

    #[test]
    fn an_agent_inserts_real_structure_not_flat_text() {
        let mut session = opened();
        let result = execute(
            &mut session,
            "insert_block",
            &json!({ "markdown": "## New Section\n\n- one\n- two\n" }),
        );

        // A heading and a list, not a single paragraph of literal Markdown.
        assert_eq!(result["inserted"].as_array().unwrap().len(), 2);
        let exported = session.as_ref().unwrap().export("markdown").unwrap();
        assert!(exported.contains("## New Section"));
        assert!(exported.contains("- one"));
    }

    #[test]
    fn an_agents_edits_are_attributed_to_it_in_get_blocks() {
        let mut session = opened();
        execute(
            &mut session,
            "insert_block",
            &json!({ "markdown": "written by a model" }),
        );

        let blocks = execute(&mut session, "get_blocks", &json!({}));
        let blocks = blocks["blocks"].as_array().unwrap();
        let agent_written: Vec<_> = blocks.iter().filter(|b| b["by_agent"] == true).collect();

        assert_eq!(agent_written.len(), 1);
        assert_eq!(agent_written[0]["author"], "agent");
    }

    #[test]
    fn a_block_can_be_addressed_replaced_and_deleted_by_its_id() {
        let mut session = opened();
        let blocks = execute(&mut session, "get_blocks", &json!({}));
        let target = blocks["blocks"][0]["id"].clone();

        let replaced = execute(
            &mut session,
            "replace_block",
            &json!({ "target": target, "markdown": "replaced heading" }),
        );
        assert!(replaced["replaced"].is_object());
        assert!(
            session
                .as_ref()
                .unwrap()
                .export("markdown")
                .unwrap()
                .contains("replaced heading")
        );

        let before = session.as_ref().unwrap().blocks().len();
        execute(&mut session, "delete_block", &json!({ "target": target }));
        assert_eq!(session.as_ref().unwrap().blocks().len(), before - 1);
    }

    #[test]
    fn navigation_reports_where_it_actually_landed() {
        let mut session = opened();
        let result = execute(&mut session, "goto_page", &json!({ "page": 9999 }));
        // Clamped, and the agent is told the real number rather than its ask.
        assert_eq!(result["page"], result["page_count"]);
    }

    #[test]
    fn export_rejects_a_format_the_app_cannot_write() {
        let mut session = opened();
        let ok = execute(&mut session, "export", &json!({ "to": "markdown" }));
        assert!(ok["content"].as_str().unwrap().contains("Report"));

        let bad = execute(&mut session, "export", &json!({ "to": "docx" }));
        assert!(bad.get("error").is_some(), "we do not write OOXML");
    }

    #[test]
    fn semantic_search_distinguishes_no_matches_from_no_embeddings() {
        let mut session = opened();
        let result = execute(
            &mut session,
            "search_similar",
            &json!({ "vector": [1.0, 0.0, 0.0], "limit": 5 }),
        );
        // Markdown carries no embeddings; the answer must say so rather than
        // implying the document was searched and found wanting.
        assert!(result["note"].as_str().unwrap().contains("no embeddings"));
    }

    #[test]
    fn every_advertised_action_is_dispatchable() {
        // The guard against the ontology describing an app that does not exist.
        let mut session = opened();
        for action in ACTIONS {
            let result = execute(&mut session, action, &json!({}));
            if let Some(message) = result.get("error").and_then(Value::as_str) {
                assert!(
                    !message.contains("unknown action"),
                    "{action} is advertised but not dispatchable"
                );
            }
        }
    }
}
