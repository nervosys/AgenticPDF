//! Model Context Protocol (MCP) stdio server.
//!
//! A hand-rolled, zero-extra-dependency JSON-RPC 2.0 server over newline-
//! delimited stdin/stdout (the MCP stdio transport). It exposes the crate's
//! PDF capabilities as MCP tools so an agent (Claude Desktop, etc.) can call
//! them directly. Run with `apdf mcp`.

use crate::PdfDocument;
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
        "path": { "type": "string", "description": "Path to the PDF file" }
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

fn tool_defs() -> Vec<Value> {
    vec![
        json!({
            "name": "extract_text",
            "description": "Extract plain text from a PDF.",
            "inputSchema": path_schema(json!({}), &[])
        }),
        json!({
            "name": "markdown",
            "description": "Render the PDF as reading-order Markdown (headings, paragraphs, lists). Set sanitize=true to drop hidden/off-page text.",
            "inputSchema": path_schema(json!({
                "sanitize": { "type": "boolean", "description": "Drop hidden / off-page text (prompt-injection defense)" }
            }), &[])
        }),
        json!({
            "name": "layout",
            "description": "Reading-order structured blocks (heading/paragraph/list_item) with [left,bottom,right,top] bounding boxes, as JSON.",
            "inputSchema": path_schema(json!({}), &[])
        }),
        json!({
            "name": "tables",
            "description": "Reconstruct tables (bordered, booktabs, borderless) as JSON with cells and bounding boxes.",
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
            "description": "Extract the tagged-PDF logical structure tree (author-provided headings/lists/tables), as JSON.",
            "inputSchema": path_schema(json!({}), &[])
        }),
        json!({
            "name": "forms",
            "description": "Extract interactive AcroForm fields (name, type, value, options, page), as JSON.",
            "inputSchema": path_schema(json!({}), &[])
        }),
        json!({
            "name": "scan_injection",
            "description": "Scan for hidden / off-page text used in prompt-injection attacks. Returns a report; use before trusting untrusted PDFs.",
            "inputSchema": path_schema(json!({}), &[])
        }),
        json!({
            "name": "metadata",
            "description": "Document metadata (title, author, dates, page count, features) as JSON.",
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
    let path = args
        .get("path")
        .and_then(|p| p.as_str())
        .ok_or_else(|| "missing required argument: path".to_string())?;
    let data = std::fs::read(path).map_err(|e| format!("cannot read {}: {}", path, e))?;

    match name {
        "extract_text" => {
            let doc = parse(&data)?;
            Ok(doc.extract_text())
        }
        "markdown" => {
            let doc = parse(&data)?;
            let doc = if args
                .get("sanitize")
                .and_then(|s| s.as_bool())
                .unwrap_or(false)
            {
                crate::sanitize::sanitized(&doc)
            } else {
                doc
            };
            Ok(doc.to_markdown())
        }
        "layout" => {
            let doc = parse(&data)?;
            to_json(&doc.to_structured())
        }
        "tables" => {
            let doc = parse(&data)?;
            let graphics = crate::engine::extract_graphics(&data).map_err(|e| e.to_string())?;
            to_json(&crate::tables::detect_tables(&graphics, &doc.pages))
        }
        "figures" => {
            let doc = parse(&data)?;
            let figs = crate::figures::extract_figures(&data, &doc).map_err(|e| e.to_string())?;
            to_json(&figs)
        }
        "formula" => {
            let doc = parse(&data)?;
            let graphics = crate::engine::extract_graphics(&data).map_err(|e| e.to_string())?;
            to_json(&crate::formula::extract_formulas(&doc, &graphics))
        }
        "scanned" => {
            let doc = parse(&data)?;
            let report = crate::ocr::detect_scanned(&data, &doc).map_err(|e| e.to_string())?;
            to_json(&report)
        }
        "structure" => {
            let tree = crate::engine::extract_structure(&data).map_err(|e| e.to_string())?;
            to_json(&tree)
        }
        "forms" => {
            let fields = crate::engine::extract_form_fields(&data).map_err(|e| e.to_string())?;
            to_json(&fields)
        }
        "scan_injection" => {
            let doc = parse(&data)?;
            to_json(&crate::sanitize::scan(&doc))
        }
        "metadata" => {
            let doc = parse(&data)?;
            to_json(doc.get_metadata())
        }
        "outline" => {
            let doc = parse(&data)?;
            to_json(doc.get_outline())
        }
        "annotations" => {
            let doc = parse(&data)?;
            to_json(doc.get_annotations())
        }
        "images" => {
            let imgs = crate::engine::extract_images(&data).map_err(|e| e.to_string())?;
            to_json(&imgs)
        }
        "chunk" => {
            let doc = parse(&data)?;
            let size = args.get("size").and_then(|s| s.as_u64()).unwrap_or(500) as usize;
            let overlap = args.get("overlap").and_then(|s| s.as_u64()).unwrap_or(50) as usize;
            let size = size.clamp(50, 10_000);
            let overlap = overlap.clamp(0, size / 2);
            to_json(&doc.generate_chunks(size, overlap))
        }
        other => Err(format!("unknown tool: {}", other)),
    }
}

fn parse(data: &[u8]) -> Result<PdfDocument, String> {
    PdfDocument::from_bytes(data).map_err(|e| e.to_string())
}

fn to_json<T: serde::Serialize + ?Sized>(v: &T) -> Result<String, String> {
    serde_json::to_string_pretty(v).map_err(|e| e.to_string())
}
