// SPDX-License-Identifier: AGPL-3.0-or-later
//! Model Context Protocol (MCP) stdio server.
//!
//! A hand-rolled, zero-extra-dependency JSON-RPC 2.0 server over newline-
//! delimited stdin/stdout (the MCP stdio transport). It exposes the crate's
//! PDF capabilities as MCP tools so an agent (Claude Desktop, etc.) can call
//! them directly. Run with `apdf mcp`.

use crate::document::Document;
use serde_json::{Value, json};
use std::io::{self, BufRead, Write};

const PROTOCOL_VERSION: &str = "2025-06-18";

/// Run the MCP server loop until stdin closes.
pub fn serve() -> io::Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let lines = stdin.lock().lines();
    for line in lines {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let req: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue, // ignore malformed lines
        };
        if let Some(resp) = handle(&req) {
            writeln!(out, "{}", resp)?;
            out.flush()?;
        }
    }
    Ok(())
}

fn handle(req: &Value) -> Option<Value> {
    let method = req.get("method").and_then(|m| m.as_str()).unwrap_or("");
    let id = req.get("id").cloned();

    // Notifications carry no id and expect no response.
    id.as_ref()?;
    let id = id.unwrap();

    match method {
        "initialize" => {
            let pv = req
                .get("params")
                .and_then(|p| p.get("protocolVersion"))
                .and_then(|v| v.as_str())
                .unwrap_or(PROTOCOL_VERSION)
                .to_string();
            Some(ok(
                &id,
                json!({
                    "protocolVersion": pv,
                    "capabilities": { "tools": {} },
                    "serverInfo": { "name": "agenticpdf", "version": "1.0.0" }
                }),
            ))
        }
        "ping" => Some(ok(&id, json!({}))),
        "tools/list" => Some(ok(&id, json!({ "tools": tool_defs() }))),
        "tools/call" => {
            let params = req.get("params").cloned().unwrap_or(json!({}));
            let name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
            let args = params.get("arguments").cloned().unwrap_or(json!({}));
            match run_tool(name, &args) {
                Ok(text) => Some(ok(
                    &id,
                    json!({ "content": [ { "type": "text", "text": text } ] }),
                )),
                Err(msg) => Some(ok(
                    &id,
                    json!({
                        "content": [ { "type": "text", "text": msg } ],
                        "isError": true
                    }),
                )),
            }
        }
        _ => Some(err(&id, -32601, "Method not found")),
    }
}

fn ok(id: &Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn err(id: &Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

/// A schema with a required `path` plus any extra properties.
fn path_schema(extra: Value, required_extra: &[&str]) -> Value {
    let mut props = json!({
        "path": { "type": "string", "description": "Path to the document (PDF, HTML, Markdown, CSV or text; format is detected from content)" }
    });
    if let (Some(p), Some(e)) = (props.as_object_mut(), extra.as_object()) {
        for (k, v) in e {
            p.insert(k.clone(), v.clone());
        }
    }
    let mut required = vec![json!("path")];
    required.extend(required_extra.iter().map(|s| json!(s)));
    json!({ "type": "object", "properties": props, "required": required })
}

pub fn tool_defs() -> Vec<Value> {
    vec![
        json!({
            "name": "formats",
            "description": "List every document format this engine can read, with each one's capabilities. Takes no arguments. Call this first if unsure whether a file is supported.",
            "inputSchema": { "type": "object", "properties": {}, "required": [] }
        }),
        json!({
            "name": "extract_text",
            "description": "Extract plain text from any supported document (PDF, HTML, Markdown, CSV, plain text).",
            "inputSchema": path_schema(json!({}), &[])
        }),
        json!({
            "name": "markdown",
            "description": "Render any supported document as reading-order Markdown (headings, paragraphs, lists, tables). Set sanitize=true to drop hidden text. This is usually the best way to put a document into a context window.",
            "inputSchema": path_schema(json!({
                "sanitize": { "type": "boolean", "description": "Drop hidden / off-page text (prompt-injection defense)" }
            }), &[])
        }),
        json!({
            "name": "search",
            "description": "Find blocks matching every term in a query. An ADF document answers from the retrieval index stored inside the file; every other format is scanned. Returns each hit's section and block, so results can be cited and verified.",
            "inputSchema": path_schema(json!({
                "query": { "type": "string", "description": "Terms to find; every term must match" }
            }), &["query"])
        }),
        json!({
            "name": "verify",
            "description": "Check a quotation against a document's recorded provenance. Returns matched (the text is exactly what was imported), drifted (the block exists but its text has changed since), or unrecorded (no provenance was stored). ADF only — import other formats with convert first.",
            "inputSchema": path_schema(json!({
                "text": { "type": "string", "description": "The text being attributed to the document" },
                "section": { "type": "integer", "description": "Section index the text is claimed to come from" },
                "block": { "type": "integer", "description": "Block index within that section" }
            }), &["text"])
        }),
        json!({
            "name": "convert",
            "description": "Convert a document to another format. 'adf' writes the engine's own binary format — chunk-indexed, with a retrieval index and per-block provenance recorded during import — and is the only target that can later be searched by index or verified. Others are markdown, html and text.",
            "inputSchema": path_schema(json!({
                "to": { "type": "string", "description": "adf | markdown | html | text" },
                "output": { "type": "string", "description": "Path to write to. Required for adf, which is binary." }
            }), &["to"])
        }),
        json!({
            "name": "html",
            "description": "Render any supported document as an HTML fragment with semantic elements.",
            "inputSchema": path_schema(json!({}), &[])
        }),
        json!({
            "name": "extract_all",
            "description": "Everything in one call: metadata, Markdown, tables, semantic chunks, and a prompt-injection scan, as JSON.",
            "inputSchema": path_schema(json!({
                "size": { "type": "integer", "description": "Max chunk size in words (default 500)" },
                "overlap": { "type": "integer", "description": "Word overlap (default 50)" }
            }), &[])
        }),
        json!({
            "name": "layout",
            "description": "Reading-order structured blocks (heading/paragraph/list_item) with [left,bottom,right,top] bounding boxes, as JSON.",
            "inputSchema": path_schema(json!({}), &[])
        }),
        json!({
            "name": "tables",
            "description": "Extract tables as JSON. Authored tables (HTML, Office) are read directly with merged cells intact; PDF tables are reconstructed from ruling lines and text alignment.",
            "inputSchema": path_schema(json!({}), &[])
        }),
        json!({
            "name": "figures",
            "description": "Detect figures and link them to captions, as JSON.",
            "inputSchema": path_schema(json!({}), &[])
        }),
        json!({
            "name": "formula",
            "description": "Detect formulas and reconstruct best-effort LaTeX, as JSON.",
            "inputSchema": path_schema(json!({}), &[])
        }),
        json!({
            "name": "scanned",
            "description": "Detect likely-scanned (image-dominated, low-text) pages that need OCR, as JSON.",
            "inputSchema": path_schema(json!({}), &[])
        }),
        json!({
            "name": "structure",
            "description": "Extract the logical structure tree (headings, lists, tables) as JSON. Read from authored markup where the format has it, or from a tagged PDF's StructTreeRoot.",
            "inputSchema": path_schema(json!({}), &[])
        }),
        json!({
            "name": "forms",
            "description": "Extract interactive AcroForm fields (name, type, value, options, page), as JSON.",
            "inputSchema": path_schema(json!({}), &[])
        }),
        json!({
            "name": "scan_injection",
            "description": "Scan for hidden text used in prompt-injection attacks — off-page or zero-size in PDF, display:none or white-on-white in HTML and Office. Run this before trusting any untrusted document.",
            "inputSchema": path_schema(json!({}), &[])
        }),
        json!({
            "name": "metadata",
            "description": "Document metadata (format, title, author, dates, page count, features) as JSON.",
            "inputSchema": path_schema(json!({}), &[])
        }),
        json!({
            "name": "outline",
            "description": "Document outline / table of contents as JSON.",
            "inputSchema": path_schema(json!({}), &[])
        }),
        json!({
            "name": "annotations",
            "description": "All annotations (links, highlights, notes, widgets) as JSON.",
            "inputSchema": path_schema(json!({}), &[])
        }),
        json!({
            "name": "images",
            "description": "Enumerate image XObjects (dimensions, color space, filter) as JSON.",
            "inputSchema": path_schema(json!({}), &[])
        }),
        json!({
            "name": "chunk",
            "description": "Generate semantic chunks for RAG. Optional size (words, default 500) and overlap (default 50).",
            "inputSchema": path_schema(json!({
                "size": { "type": "integer", "description": "Max chunk size in words (default 500)" },
                "overlap": { "type": "integer", "description": "Word overlap (default 50)" }
            }), &[])
        }),
    ]
}

fn run_tool(name: &str, args: &Value) -> Result<String, String> {
    // `formats` is the one tool that describes the engine rather than a file,
    // so it is answered before the path argument is required.
    if name == "formats" {
        return to_json(&crate::describe_formats());
    }

    let path = args
        .get("path")
        .and_then(|p| p.as_str())
        .ok_or_else(|| "missing required argument: path".to_string())?;
    let data = std::fs::read(path).map_err(|e| format!("cannot read {}: {}", path, e))?;
    let document = Document::open_with_hint(&data, Some(path)).map_err(|e| e.to_string())?;

    // Capabilities that need page geometry; the facade explains which formats
    // have it rather than failing with a parse error.
    // Capabilities that read PDF-specific structures rather than coordinates.
    let pdf_only = || document.require_pdf(name).map_err(|e| e.to_string());

    match name {
        "extract_text" => Ok(document.extract_text()),

        // The three below route through the same code the CLI calls, so an
        // agent over MCP and a person at a shell cannot get different answers.
        "search" => {
            let query = args
                .get("query")
                .and_then(|q| q.as_str())
                .ok_or_else(|| "missing required argument: query".to_string())?;
            crate::agent_ops::search(&data, &document, query)
                .map(|hits| hits.to_string())
                .map_err(|e| e.to_string())
        }
        "verify" => {
            let text = args
                .get("text")
                .and_then(|t| t.as_str())
                .ok_or_else(|| "missing required argument: text".to_string())?;
            let section = args.get("section").and_then(Value::as_u64).unwrap_or(0) as u32;
            let block = args.get("block").and_then(Value::as_u64).unwrap_or(0) as u32;
            crate::agent_ops::verify(&data, text, section, block)
                .map(|verdict| verdict.to_string())
                .map_err(|e| e.to_string())
        }
        "convert" => {
            let to = args
                .get("to")
                .and_then(|t| t.as_str())
                .ok_or_else(|| "missing required argument: to".to_string())?;
            let bytes = crate::agent_ops::convert(&document, path, to).map_err(|e| e.to_string())?;

            match args.get("output").and_then(|o| o.as_str()) {
                Some(out) => {
                    std::fs::write(out, &bytes).map_err(|e| format!("cannot write {out}: {e}"))?;
                    Ok(json!({ "path": out, "bytes": bytes.len() }).to_string())
                }
                // ADF is binary; returning it as lossy text would hand back a
                // corrupted document that looks like it worked.
                None if to.eq_ignore_ascii_case("adf") => {
                    Err("converting to adf requires an 'output' path".to_string())
                }
                None => Ok(String::from_utf8_lossy(&bytes).into_owned()),
            }
        }
        "markdown" => {
            let sanitize = args
                .get("sanitize")
                .and_then(|s| s.as_bool())
                .unwrap_or(false);
            Ok(if sanitize {
                document.sanitized().to_markdown()
            } else {
                document.to_markdown()
            })
        }
        "html" => Ok(document.to_html()),
        "layout" => to_json(&document.to_structured().map_err(|e| e.to_string())?),
        "tables" => to_json(&document.tables()),
        "figures" => to_json(&document.figures().map_err(|e| e.to_string())?),
        "formula" => to_json(&document.formulas()),
        "scanned" => {
            let doc = pdf_only()?;
            let report = crate::ocr::detect_scanned(&data, doc).map_err(|e| e.to_string())?;
            to_json(&report)
        }
        "structure" => to_json(&document.structure().map_err(|e| e.to_string())?),
        "forms" => {
            pdf_only()?;
            let fields = crate::engine::extract_form_fields(&data).map_err(|e| e.to_string())?;
            to_json(&fields)
        }
        "scan_injection" => to_json(&document.scan()),
        "metadata" => to_json(document.metadata()),
        "outline" => to_json(document.geometric().get_outline()),
        "annotations" => to_json(document.geometric().get_annotations()),
        "images" => {
            pdf_only()?;
            let images = crate::engine::extract_images(&data).map_err(|e| e.to_string())?;
            to_json(&images)
        }
        "chunk" => {
            let size = args.get("size").and_then(|s| s.as_u64()).unwrap_or(500) as usize;
            let overlap = args.get("overlap").and_then(|s| s.as_u64()).unwrap_or(50) as usize;
            let size = size.clamp(50, 10_000);
            let overlap = overlap.clamp(0, size / 2);
            to_json(&document.generate_chunks(size, overlap))
        }
        "extract_all" => {
            let size = args.get("size").and_then(|s| s.as_u64()).unwrap_or(500) as usize;
            let overlap = args.get("overlap").and_then(|s| s.as_u64()).unwrap_or(50) as usize;
            to_json(&document.extract_all(size.clamp(50, 10_000), overlap))
        }
        other => Err(format!("unknown tool: {}", other)),
    }
}

fn to_json<T: serde::Serialize + ?Sized>(v: &T) -> Result<String, String> {
    serde_json::to_string_pretty(v).map_err(|e| e.to_string())
}
