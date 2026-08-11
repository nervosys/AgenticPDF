// SPDX-License-Identifier: AGPL-3.0-or-later
//! AgenticPDF CLI — Command-line interface for PDF processing.
//!
//! A self-describing CLI that agentic LLMs (ChatGPT, Claude, Gemini, etc.)
//! can discover and invoke programmatically. Run `apdf describe` to get a
//! machine-readable JSON-LD ontology of all commands, parameters, output
//! schemas, and workflows.
//!
//! Usage:
//!   apdf text        <file>  [--pages 1-5] [--format json|text] [--output path]
//!   apdf meta        <file>  [--format json|text]
//!   apdf annotations <file>  [--pages 1-5] [--format json|text]
//!   apdf outline     <file>  [--format json|text]
//!   apdf images      <file>  [--pages 1-5] [--format json|text]
//!   apdf chunk       <file>  [--size 500] [--overlap 50] [--format json|text]
//!   apdf all         <file>  [--chunk-size 500] [--chunk-overlap 50] [--output path]
//!   apdf describe
//!   apdf info        [--format json|text]

#[cfg(feature = "cli")]
use clap::{Parser, Subcommand};

#[cfg(feature = "cli")]
use agenticpdf::{PdfError, document::Document};
#[cfg(feature = "cli")]
use std::fs;
use std::process;

#[cfg(feature = "cli")]
#[derive(Parser)]
#[command(
    name = "apdf",
    version = "1.0.0",
    about = "High-performance PDF processing CLI for agentic AI workflows",
    long_about = "AgenticPDF (apdf) — Extract text, metadata, annotations, outlines, images, and semantic chunks from PDF documents.\nOptimized for AI agent integration with structured JSON output.\n\nRun `apdf describe` to get a machine-readable JSON-LD ontology for autonomous LLM discovery."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[cfg(feature = "cli")]
#[derive(Subcommand)]
enum Commands {
    /// Extract text content from a PDF
    Text {
        /// Path to the PDF file
        file: String,
        /// Page range (e.g., "1-5" or "3")
        #[arg(short, long)]
        pages: Option<String>,
        /// Output format: text or json
        #[arg(short, long, default_value = "text")]
        format: String,
        /// Output file path (stdout if omitted)
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Show PDF metadata
    Meta {
        /// Path to the PDF file
        file: String,
        /// Output format: text or json
        #[arg(short, long, default_value = "text")]
        format: String,
    },
    /// Extract annotations (links, highlights, notes, widgets) from a PDF
    Annotations {
        /// Path to the PDF file
        file: String,
        /// Page range (e.g., "1-5" or "3")
        #[arg(short, long)]
        pages: Option<String>,
        /// Output format: text or json
        #[arg(short, long, default_value = "json")]
        format: String,
    },
    /// Extract the document outline (bookmarks / table of contents)
    Outline {
        /// Path to the PDF file
        file: String,
        /// Output format: text or json
        #[arg(short, long, default_value = "json")]
        format: String,
    },
    /// List images in a PDF
    Images {
        /// Path to the PDF file
        file: String,
        /// Page range
        #[arg(short, long)]
        pages: Option<String>,
        /// Output format: text or json
        #[arg(short, long, default_value = "json")]
        format: String,
    },
    /// Render the PDF as Markdown with reading order, headings, and lists
    Markdown {
        /// Path to the PDF file
        file: String,
        /// Page range (e.g., "1-5" or "3")
        #[arg(short, long)]
        pages: Option<String>,
        /// Drop hidden / off-page text (prompt-injection defense)
        #[arg(long)]
        sanitize: bool,
        /// Output file path (stdout if omitted)
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Reconstruct bordered tables (Markdown or JSON)
    Table {
        /// Path to the PDF file
        file: String,
        /// Page range (e.g., "1-5" or "3")
        #[arg(short, long)]
        pages: Option<String>,
        /// Output format: markdown or json
        #[arg(short, long, default_value = "markdown")]
        format: String,
        /// Output file path (stdout if omitted)
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Scan for hidden / off-page text (prompt-injection signals)
    Scan {
        /// Path to the PDF file
        file: String,
        /// Output format: text or json
        #[arg(short, long, default_value = "text")]
        format: String,
    },
    /// Detect figures and link them to captions
    Figures {
        /// Path to the PDF file
        file: String,
        /// Page range (e.g., "1-5" or "3")
        #[arg(short, long)]
        pages: Option<String>,
        /// Output format: text or json
        #[arg(short, long, default_value = "json")]
        format: String,
    },
    /// Extract the tagged-PDF logical structure tree (author-provided)
    Structure {
        /// Path to the PDF file
        file: String,
        /// Output format: text or json
        #[arg(short, long, default_value = "text")]
        format: String,
    },
    /// Extract interactive AcroForm fields (names, types, values)
    Forms {
        /// Path to the PDF file
        file: String,
        /// Output format: text or json
        #[arg(short, long, default_value = "json")]
        format: String,
    },
    /// Emit a page's device-space display list (render ops) as JSON, for the
    /// hardware-accelerated WebGL renderer
    Displaylist {
        /// Path to the PDF file
        file: String,
        /// Page number (1-based)
        #[arg(short, long, default_value = "1")]
        page: usize,
    },
    /// Detect formulas and reconstruct best-effort LaTeX
    Formula {
        /// Path to the PDF file
        file: String,
        /// Page range (e.g., "1-5" or "3")
        #[arg(short, long)]
        pages: Option<String>,
        /// Output format: text or json
        #[arg(short, long, default_value = "text")]
        format: String,
    },
    /// Detect likely-scanned (image-dominated, low-text) pages needing OCR
    Scanned {
        /// Path to the PDF file
        file: String,
        /// Output format: text or json
        #[arg(short, long, default_value = "text")]
        format: String,
    },
    /// OCR likely-scanned pages (Tesseract CLI, or a PaddleOCR/EasyOCR server)
    #[cfg(feature = "ocr")]
    Ocr {
        /// Path to the PDF file
        file: String,
        /// Tesseract language(s), e.g. "eng" or "eng+deu" (CLI backend)
        #[arg(short, long, default_value = "eng")]
        lang: String,
        /// Use an HTTP OCR server (PaddleOCR/EasyOCR /ocr endpoint) instead
        #[arg(short, long)]
        server: Option<String>,
        /// Use a VLM via an OpenAI-compatible /v1/chat/completions URL
        /// (e.g. PaddleOCR-VL-1.6 served with vLLM)
        #[arg(long)]
        vlm: Option<String>,
        /// Model name for the VLM backend
        #[arg(long, default_value = "PaddleOCR-VL-1.6")]
        model: String,
    },
    /// Produce reading-order structured layout (blocks with type/level/bbox)
    Layout {
        /// Path to the PDF file
        file: String,
        /// Page range (e.g., "1-5" or "3")
        #[arg(short, long)]
        pages: Option<String>,
        /// Output file path (stdout if omitted)
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Generate semantic chunks for RAG pipelines
    Chunk {
        /// Path to the PDF file
        file: String,
        /// Maximum chunk size in tokens
        #[arg(short, long, default_value = "500")]
        size: usize,
        /// Overlap between chunks
        #[arg(short, long, default_value = "50")]
        overlap: usize,
        /// Output format: text or json
        #[arg(short, long, default_value = "json")]
        format: String,
        /// Output file path
        #[arg(long)]
        output: Option<String>,
    },
    /// Extract everything from a PDF: metadata, text, annotations, outline, and chunks
    All {
        /// Path to the PDF file
        file: String,
        /// Maximum chunk size in tokens
        #[arg(long, default_value = "500")]
        chunk_size: usize,
        /// Overlap between chunks
        #[arg(long, default_value = "50")]
        chunk_overlap: usize,
        /// Output file path (stdout if omitted)
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Convert any supported document to Markdown, HTML or JSON
    /// Search a document. ADF answers from the index inside the file; other
    /// formats are scanned.
    Search {
        /// Path to the document
        file: String,
        /// Terms to find; every term must match
        query: String,
        /// Output as JSON rather than text
        #[arg(long)]
        json: bool,
    },
    /// Check a quotation against a document's recorded provenance.
    ///
    /// Answers matched / drifted / unrecorded — the difference between a
    /// citation that is still accurate, one whose source has been edited since,
    /// and one that was never recorded at all.
    Verify {
        /// Path to an ADF document
        file: String,
        /// The text being attributed to the document
        text: String,
        /// Section index the text is claimed to come from
        #[arg(long, default_value_t = 0)]
        section: u32,
        /// Block index within that section
        #[arg(long, default_value_t = 0)]
        block: u32,
        /// Output as JSON rather than text
        #[arg(long)]
        json: bool,
    },
    Convert {
        /// Path to the document
        file: String,
        /// Target format: markdown, html, json or text
        #[arg(short, long, default_value = "markdown")]
        to: String,
        /// Drop hidden text (prompt-injection defense)
        #[arg(long)]
        sanitize: bool,
        /// Output file path (stdout if omitted)
        #[arg(short, long)]
        output: Option<String>,
    },
    /// List every document format this build can read
    Formats {
        /// Output format: text or json
        #[arg(short, long, default_value = "text")]
        format: String,
    },
    /// Run as an MCP (Model Context Protocol) stdio server for agents
    Mcp,
    /// Output machine-readable JSON-LD ontology for LLM agent discovery
    #[command(alias = "ontology")]
    Describe,
    /// Show library info and capabilities
    Info {
        /// Output format: text or json
        #[arg(short, long, default_value = "text")]
        format: Option<String>,
    },
}

fn main() {
    #[cfg(feature = "cli")]
    {
        let cli = Cli::parse();

        let result = match cli.command {
            Commands::Text {
                file,
                pages,
                format,
                output,
            } => cmd_text(&file, pages.as_deref(), &format, output.as_deref()),
            Commands::Meta { file, format } => cmd_meta(&file, &format),
            Commands::Markdown {
                file,
                pages,
                sanitize,
                output,
            } => cmd_markdown(&file, pages.as_deref(), sanitize, output.as_deref()),
            Commands::Layout {
                file,
                pages,
                output,
            } => cmd_layout(&file, pages.as_deref(), output.as_deref()),
            Commands::Table {
                file,
                pages,
                format,
                output,
            } => cmd_table(&file, pages.as_deref(), &format, output.as_deref()),
            Commands::Scan { file, format } => cmd_scan(&file, &format),
            Commands::Structure { file, format } => cmd_structure(&file, &format),
            Commands::Forms { file, format } => cmd_forms(&file, &format),
            Commands::Displaylist { file, page } => cmd_displaylist(&file, page),
            Commands::Figures {
                file,
                pages,
                format,
            } => cmd_figures(&file, pages.as_deref(), &format),
            Commands::Formula {
                file,
                pages,
                format,
            } => cmd_formula(&file, pages.as_deref(), &format),
            Commands::Scanned { file, format } => cmd_scanned(&file, &format),
            #[cfg(feature = "ocr")]
            Commands::Ocr {
                file,
                lang,
                server,
                vlm,
                model,
            } => cmd_ocr(&file, &lang, server.as_deref(), vlm.as_deref(), &model),
            Commands::Annotations {
                file,
                pages,
                format,
            } => cmd_annotations(&file, pages.as_deref(), &format),
            Commands::Outline { file, format } => cmd_outline(&file, &format),
            Commands::Images {
                file,
                pages,
                format,
            } => cmd_images(&file, pages.as_deref(), &format),
            Commands::Chunk {
                file,
                size,
                overlap,
                format,
                output,
            } => cmd_chunk(&file, size, overlap, &format, output.as_deref()),
            Commands::All {
                file,
                chunk_size,
                chunk_overlap,
                output,
            } => cmd_all(&file, chunk_size, chunk_overlap, output.as_deref()),
            Commands::Search { file, query, json } => cmd_search(&file, &query, json),
            Commands::Verify {
                file,
                text,
                section,
                block,
                json,
            } => cmd_verify(&file, &text, section, block, json),
            Commands::Convert {
                file,
                to,
                sanitize,
                output,
            } => cmd_convert(&file, &to, sanitize, output.as_deref()),
            Commands::Formats { format } => cmd_formats(&format),
            Commands::Mcp => agenticpdf::mcp::serve().map_err(PdfError::from),
            Commands::Describe => cmd_describe(),
            Commands::Info { format } => cmd_info(format.as_deref().unwrap_or("text")),
        };

        if let Err(e) = result {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
    }

    #[cfg(not(feature = "cli"))]
    {
        eprintln!("CLI feature not enabled. Build with: cargo build --features cli");
        process::exit(1);
    }
}

/// Read a file and parse it as whatever format its bytes say it is.
///
/// The path is passed as a hint, which only breaks ties inside the plain-text
/// family — a renamed `.docx` is still read as a `.docx`.
#[cfg(feature = "cli")]
fn load_document(path: &str) -> Result<Document, PdfError> {
    let data = fs::read(path)?;
    Document::open_with_hint(&data, Some(path))
}

#[cfg(feature = "cli")]
fn parse_page_range(range: Option<&str>, max_pages: usize) -> (usize, usize) {
    match range {
        Some(r) => {
            if let Some((start, end)) = r.split_once('-') {
                let s = start.parse::<usize>().unwrap_or(1).max(1);
                let e = end.parse::<usize>().unwrap_or(max_pages).min(max_pages);
                (s, e)
            } else if let Ok(page) = r.parse::<usize>() {
                (page.max(1), page.min(max_pages))
            } else {
                (1, max_pages)
            }
        }
        None => (1, max_pages),
    }
}

#[cfg(feature = "cli")]
fn cmd_text(
    file: &str,
    pages: Option<&str>,
    format: &str,
    output: Option<&str>,
) -> Result<(), PdfError> {
    let document = load_document(file)?;
    let doc = document.geometric();
    let (start, end) = parse_page_range(pages, doc.pages.len());

    let selected_pages: Vec<_> = doc
        .pages
        .iter()
        .filter(|p| p.index + 1 >= start && p.index < end)
        .collect();

    let result = match format {
        // JSON reports positioned text, page by page. For a reflowable source
        // those are the typesetter's coordinates rather than the author's, but
        // they are the same ones a render of the document would use.
        "json" => serde_json::to_string_pretty(&selected_pages)
            .map_err(|e| PdfError::ExportError(e.to_string()))?,
        // Plain text comes from the semantic model where there is one: it keeps
        // paragraphs whole instead of returning them broken into typeset lines.
        _ if document.semantic().is_some() && pages.is_none() => document.extract_text(),
        _ => selected_pages
            .iter()
            .flat_map(|p| p.text_content.iter())
            .map(|t| t.text.as_str())
            .collect::<Vec<_>>()
            .join("\n"),
    };

    match output {
        Some(path) => fs::write(path, &result)?,
        None => println!("{}", result),
    }

    Ok(())
}

#[cfg(feature = "cli")]
fn cmd_meta(file: &str, format: &str) -> Result<(), PdfError> {
    let document = load_document(file)?;
    let meta = document.metadata();

    match format {
        "json" => {
            let json = serde_json::to_string_pretty(meta)
                .map_err(|e| PdfError::ExportError(e.to_string()))?;
            println!("{}", json);
        }
        _ => {
            println!("Format:           {}", document.format().label());
            if !meta.pdf_version.is_empty() {
                println!("PDF Version:      {}", meta.pdf_version);
            }
            println!("Pages:            {}", meta.page_count);
            println!("File Size:        {} bytes", meta.file_size);
            println!("Encrypted:        {}", meta.encrypted);
            if let Some(ref title) = meta.title {
                println!("Title:            {}", title);
            }
            if let Some(ref author) = meta.author {
                println!("Author:           {}", author);
            }
            if let Some(ref subject) = meta.subject {
                println!("Subject:          {}", subject);
            }
            if let Some(ref creator) = meta.creator {
                println!("Creator:          {}", creator);
            }
            if let Some(ref producer) = meta.producer {
                println!("Producer:         {}", producer);
            }
            if let Some(ref date) = meta.creation_date {
                println!("Creation Date:    {}", date);
            }
            if let Some(ref date) = meta.modification_date {
                println!("Modification Date: {}", date);
            }
            println!("Has Forms:        {}", meta.has_forms);
            println!("Has Annotations:  {}", meta.has_annotations);
            println!("Has Outlines:     {}", meta.has_outlines);
        }
    }

    Ok(())
}

#[cfg(feature = "cli")]
fn cmd_markdown(
    file: &str,
    pages: Option<&str>,
    sanitize: bool,
    output: Option<&str>,
) -> Result<(), PdfError> {
    let document = load_document(file)?;
    let document = if sanitize {
        document.sanitized()
    } else {
        document
    };
    let (start, end) = parse_page_range(pages, document.page_count());
    let md = document.to_markdown_range(start, end);
    match output {
        Some(path) => fs::write(path, &md)?,
        None => println!("{}", md),
    }
    Ok(())
}

#[cfg(feature = "cli")]
fn cmd_table(
    file: &str,
    pages: Option<&str>,
    format: &str,
    output: Option<&str>,
) -> Result<(), PdfError> {
    let document = load_document(file)?;
    let (start, end) = parse_page_range(pages, document.page_count());
    let tables: Vec<_> = document
        .tables()
        .into_iter()
        .filter(|t| t.page_number >= start && t.page_number <= end)
        .collect();

    let result = match format {
        "json" => serde_json::to_string_pretty(&tables)
            .map_err(|e| PdfError::ExportError(e.to_string()))?,
        _ => agenticpdf::tables::to_markdown(&tables),
    };
    match output {
        Some(path) => fs::write(path, &result)?,
        None => println!("{}", result),
    }
    eprintln!("Found {} tables", tables.len());
    Ok(())
}

#[cfg(feature = "cli")]
fn cmd_figures(file: &str, pages: Option<&str>, format: &str) -> Result<(), PdfError> {
    let document = load_document(file)?;
    let (start, end) = parse_page_range(pages, document.page_count());
    let figures: Vec<_> = document
        .figures()?
        .into_iter()
        .filter(|f| f.page_number >= start && f.page_number <= end)
        .collect();

    match format {
        "json" => {
            let json = serde_json::to_string_pretty(&figures)
                .map_err(|e| PdfError::ExportError(e.to_string()))?;
            println!("{}", json);
        }
        _ => {
            if figures.is_empty() {
                println!("No figures found.");
            } else {
                for f in &figures {
                    let label = f.label.as_deref().unwrap_or("(uncaptioned)");
                    print!(
                        "  [{}] {} page {} [{:.0} {:.0} {:.0} {:.0}]",
                        f.kind, label, f.page_number, f.bbox[0], f.bbox[1], f.bbox[2], f.bbox[3]
                    );
                    if f.width > 0 {
                        print!(" {}x{}px", f.width, f.height);
                    }
                    if let Some(cap) = &f.caption {
                        let short: String = cap.chars().take(60).collect();
                        print!(" \"{}\"", short);
                    }
                    println!();
                }
            }
        }
    }
    eprintln!("Found {} figures", figures.len());
    Ok(())
}

#[cfg(feature = "cli")]
fn cmd_formula(file: &str, pages: Option<&str>, format: &str) -> Result<(), PdfError> {
    let document = load_document(file)?;
    let (start, end) = parse_page_range(pages, document.page_count());
    let formulas: Vec<_> = document
        .formulas()
        .into_iter()
        .filter(|f| f.page_number >= start && f.page_number <= end)
        .collect();

    match format {
        "json" => {
            let json = serde_json::to_string_pretty(&formulas)
                .map_err(|e| PdfError::ExportError(e.to_string()))?;
            println!("{}", json);
        }
        _ => {
            if formulas.is_empty() {
                println!("No formulas detected.");
            } else {
                for f in &formulas {
                    println!(
                        "  page {} [{:.0} {:.0} {:.0} {:.0}]",
                        f.page_number, f.bbox[0], f.bbox[1], f.bbox[2], f.bbox[3]
                    );
                    println!("    {}", f.latex);
                }
            }
        }
    }
    eprintln!("Found {} formulas", formulas.len());
    Ok(())
}

#[cfg(feature = "cli")]
fn cmd_scanned(file: &str, format: &str) -> Result<(), PdfError> {
    let document = load_document(file)?;
    let doc = document.require_pdf("scanned-page detection")?;
    let data = document.data();
    let report = agenticpdf::ocr::detect_scanned(data, doc)?;
    let scanned: Vec<_> = report.iter().filter(|p| p.likely_scanned).collect();

    match format {
        "json" => {
            let json = serde_json::to_string_pretty(&report)
                .map_err(|e| PdfError::ExportError(e.to_string()))?;
            println!("{}", json);
        }
        _ => {
            if scanned.is_empty() {
                println!("No likely-scanned pages (all pages have extractable text).");
            } else {
                println!("Likely-scanned pages needing OCR:");
                for p in &scanned {
                    println!(
                        "  page {} — {:.0}% image coverage, {} text chars",
                        p.page_number,
                        p.image_coverage * 100.0,
                        p.text_chars
                    );
                }
                if !agenticpdf::ocr::OCR_BUILTIN {
                    println!(
                        "(build with --features ocr to enable a bundled OCR engine; or wire an OcrBackend)"
                    );
                }
            }
        }
    }
    Ok(())
}

#[cfg(all(feature = "cli", feature = "ocr"))]
fn cmd_ocr(
    file: &str,
    lang: &str,
    server: Option<&str>,
    vlm: Option<&str>,
    model: &str,
) -> Result<(), PdfError> {
    let document = load_document(file)?;
    let doc = document.require_pdf("OCR")?;
    let data = document.data();
    let results = match (vlm, server) {
        (Some(url), _) => agenticpdf::ocr::recognize_scanned_vlm(data, doc, url, model)?,
        (None, Some(url)) => agenticpdf::ocr::recognize_scanned_http(data, doc, url)?,
        (None, None) => agenticpdf::ocr::recognize_scanned(data, doc, lang)?,
    };
    if results.is_empty() {
        eprintln!("No likely-scanned pages to OCR.");
        return Ok(());
    }
    for r in &results {
        println!("===== page {} =====", r.page_number);
        println!("{}", r.text.trim());
    }
    eprintln!("OCR'd {} page(s)", results.len());
    Ok(())
}

#[cfg(feature = "cli")]
fn cmd_displaylist(file: &str, page: usize) -> Result<(), PdfError> {
    let document = load_document(file)?;
    let dl = document.display_list(page)?;
    let json =
        serde_json::to_string_pretty(&dl).map_err(|e| PdfError::ExportError(e.to_string()))?;
    println!("{}", json);
    eprintln!("Page {}: {} render ops", page, dl.ops.len());
    Ok(())
}

#[cfg(feature = "cli")]
fn cmd_forms(file: &str, format: &str) -> Result<(), PdfError> {
    let document = load_document(file)?;
    let data = document
        .require_pdf("form fields")
        .map(|_| document.data())?;
    let fields = agenticpdf::engine::extract_form_fields(data)?;
    match format {
        "json" => {
            let json = serde_json::to_string_pretty(&fields)
                .map_err(|e| PdfError::ExportError(e.to_string()))?;
            println!("{}", json);
        }
        _ => {
            if fields.is_empty() {
                println!("No interactive form fields (document has no AcroForm).");
            } else {
                for f in &fields {
                    let val = f.value.as_deref().unwrap_or("");
                    print!("  [{}] {} = \"{}\"", f.field_type, f.name, val);
                    if f.required {
                        print!(" (required)");
                    }
                    if f.read_only {
                        print!(" (read-only)");
                    }
                    println!();
                }
            }
        }
    }
    eprintln!("Found {} form fields", fields.len());
    Ok(())
}

#[cfg(feature = "cli")]
fn cmd_structure(file: &str, format: &str) -> Result<(), PdfError> {
    let document = load_document(file)?;
    let tree = document.structure()?;
    match format {
        "json" => {
            let json = serde_json::to_string_pretty(&tree)
                .map_err(|e| PdfError::ExportError(e.to_string()))?;
            println!("{}", json);
        }
        _ => {
            if tree.is_empty() {
                println!("No tagged structure (document is not a Tagged PDF).");
            } else {
                fn print_nodes(nodes: &[agenticpdf::engine::StructNode], depth: usize) {
                    for n in nodes {
                        let indent = "  ".repeat(depth);
                        let text = n
                            .text
                            .as_deref()
                            .map(|t| format!(" — {t}"))
                            .unwrap_or_default();
                        let pg = n
                            .page_number
                            .map(|p| format!(" (p{p})"))
                            .unwrap_or_default();
                        println!("{indent}{}{pg}{text}", n.kind);
                        print_nodes(&n.children, depth + 1);
                    }
                }
                print_nodes(&tree, 0);
            }
        }
    }
    Ok(())
}

#[cfg(feature = "cli")]
fn cmd_scan(file: &str, format: &str) -> Result<(), PdfError> {
    let document = load_document(file)?;
    let report = document.scan();
    match format {
        "json" => {
            let json = serde_json::to_string_pretty(&report)
                .map_err(|e| PdfError::ExportError(e.to_string()))?;
            println!("{}", json);
        }
        _ => {
            if report.clean {
                println!("Clean: no hidden or off-page text detected.");
            } else {
                println!(
                    "⚠ {} suspicious fragment(s) detected:",
                    report.suspicious_fragments
                );
                for f in &report.findings {
                    let reason = match f.reason {
                        agenticpdf::sanitize::Reason::OffPage => "off-page",
                        agenticpdf::sanitize::Reason::TinyText => "tiny-text",
                        agenticpdf::sanitize::Reason::Hidden => "hidden",
                    };
                    let short: String = f.text.chars().take(70).collect();
                    println!(
                        "  [{}] page {} ({:.1}pt) \"{}\"",
                        reason, f.page_number, f.font_size, short
                    );
                }
            }
        }
    }
    Ok(())
}

#[cfg(feature = "cli")]
fn cmd_layout(file: &str, pages: Option<&str>, output: Option<&str>) -> Result<(), PdfError> {
    let document = load_document(file)?;
    let (start, end) = parse_page_range(pages, document.page_count());
    let mut structured = document.to_structured()?;
    structured
        .pages
        .retain(|p| p.page_number >= start && p.page_number <= end);
    let json = serde_json::to_string_pretty(&structured)
        .map_err(|e| PdfError::ExportError(e.to_string()))?;
    match output {
        Some(path) => fs::write(path, &json)?,
        None => println!("{}", json),
    }
    Ok(())
}

#[cfg(feature = "cli")]
fn cmd_annotations(file: &str, pages: Option<&str>, format: &str) -> Result<(), PdfError> {
    let document = load_document(file)?;
    let doc = document.require_geometry("annotations")?;
    let (start, end) = parse_page_range(pages, doc.pages.len());

    let annotations: Vec<_> = doc
        .get_annotations()
        .iter()
        .filter(|a| a.page_number >= start && a.page_number <= end)
        .cloned()
        .collect();

    match format {
        "json" => {
            let json = serde_json::to_string_pretty(&annotations)
                .map_err(|e| PdfError::ExportError(e.to_string()))?;
            println!("{}", json);
        }
        _ => {
            if annotations.is_empty() {
                println!("No annotations found.");
            } else {
                for a in &annotations {
                    print!(
                        "  [{}] page {} rect [{:.0} {:.0} {:.0} {:.0}]",
                        a.subtype, a.page_number, a.rect[0], a.rect[1], a.rect[2], a.rect[3]
                    );
                    if let Some(ref uri) = a.uri {
                        print!(" uri={}", uri);
                    }
                    if let Some(ref contents) = a.contents {
                        let short: String = contents.chars().take(60).collect();
                        print!(" \"{}\"", short);
                    }
                    println!();
                }
            }
        }
    }

    eprintln!("Found {} annotations", annotations.len());
    Ok(())
}

#[cfg(feature = "cli")]
fn cmd_outline(file: &str, format: &str) -> Result<(), PdfError> {
    let document = load_document(file)?;
    let doc = document.require_geometry("outlines")?;
    let outline = doc.get_outline();

    match format {
        "json" => {
            let json = serde_json::to_string_pretty(&outline)
                .map_err(|e| PdfError::ExportError(e.to_string()))?;
            println!("{}", json);
        }
        _ => {
            if outline.is_empty() {
                println!("No outline/bookmarks found.");
            } else {
                fn print_items(items: &[agenticpdf::OutlineItem], depth: usize) {
                    for item in items {
                        let indent = "  ".repeat(depth);
                        println!("{}{}", indent, item.title);
                        print_items(&item.children, depth + 1);
                    }
                }
                print_items(outline, 0);
            }
        }
    }

    Ok(())
}

#[cfg(feature = "cli")]
fn cmd_images(file: &str, pages: Option<&str>, format: &str) -> Result<(), PdfError> {
    let document = load_document(file)?;
    let doc = document.require_pdf("image enumeration")?;
    let data = document.data();
    let (start, end) = parse_page_range(pages, doc.pages.len());

    let images: Vec<_> = agenticpdf::engine::extract_images(data)?
        .into_iter()
        .filter(|i| i.page_number >= start && i.page_number <= end)
        .collect();

    match format {
        "json" => {
            let json = serde_json::to_string_pretty(&images)
                .map_err(|e| PdfError::ExportError(e.to_string()))?;
            println!("{}", json);
        }
        _ => {
            if images.is_empty() {
                println!("No images found.");
            } else {
                for img in &images {
                    println!(
                        "  [{}] page {} {}x{} {} {}bpc {}  ({} bytes)",
                        img.id,
                        img.page_number,
                        img.width,
                        img.height,
                        img.color_space,
                        img.bits_per_component,
                        img.filter,
                        img.data_length
                    );
                }
            }
        }
    }

    eprintln!("Found {} images", images.len());
    Ok(())
}

#[cfg(feature = "cli")]
fn cmd_chunk(
    file: &str,
    size: usize,
    overlap: usize,
    format: &str,
    output: Option<&str>,
) -> Result<(), PdfError> {
    // Clamp chunk size to reasonable bounds
    let size = size.clamp(50, 10_000);
    let overlap = overlap.clamp(0, size / 2);

    let document = load_document(file)?;
    let chunks = document.generate_chunks(size, overlap);

    let result = match format {
        "json" => serde_json::to_string_pretty(&chunks)
            .map_err(|e| PdfError::ExportError(e.to_string()))?,
        _ => chunks
            .iter()
            .map(|c| {
                format!(
                    "--- Chunk {} (pages {:?}, {} tokens) ---\n{}",
                    c.id, c.page_numbers, c.token_count, c.content
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n"),
    };

    match output {
        Some(path) => fs::write(path, &result)?,
        None => println!("{}", result),
    }

    eprintln!(
        "Generated {} chunks (max_size={}, overlap={})",
        chunks.len(),
        size,
        overlap
    );

    Ok(())
}

#[cfg(feature = "cli")]
fn cmd_all(
    file: &str,
    chunk_size: usize,
    chunk_overlap: usize,
    output: Option<&str>,
) -> Result<(), PdfError> {
    let size = chunk_size.clamp(50, 10_000);
    let overlap = chunk_overlap.clamp(0, size / 2);

    let document = load_document(file)?;
    let full = document.extract_all(size, overlap);

    let json =
        serde_json::to_string_pretty(&full).map_err(|e| PdfError::ExportError(e.to_string()))?;

    match output {
        Some(path) => fs::write(path, &json)?,
        None => println!("{}", json),
    }

    eprintln!(
        "Extracted: {} pages, {} annotations, {} outline items, {} chunks, {} tables, {} figures, {} formulas",
        full.pages.len(),
        full.annotations.len(),
        full.outline.len(),
        full.chunks.len(),
        full.tables.len(),
        full.figures.len(),
        full.formulas.len()
    );

    Ok(())
}

/// Convert any supported document to another representation.
#[cfg(feature = "cli")]
/// Search a document, using its embedded index where it has one.
///
/// The implementation lives in `agent_ops` so that the CLI, the MCP server and
/// the reader app cannot answer the same question differently.
fn cmd_search(file: &str, query: &str, json: bool) -> Result<(), PdfError> {
    let data = fs::read(file)?;
    let document = load_document(file)?;
    let result = agenticpdf::agent_ops::search(&data, &document, query)?;
    let hits = result["hits"].as_array().cloned().unwrap_or_default();

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&result)
                .map_err(|e| PdfError::ExportError(e.to_string()))?
        );
    } else {
        for hit in &hits {
            println!(
                "[{}:{}] {}",
                hit["section"],
                hit["block"],
                hit["text"].as_str().unwrap_or("")
            );
        }
        eprintln!("{} result(s)", hits.len());
    }
    Ok(())
}

/// Check a quotation against recorded provenance.
fn cmd_verify(
    file: &str,
    text: &str,
    section: u32,
    block: u32,
    json: bool,
) -> Result<(), PdfError> {
    let data = fs::read(file)?;
    if !agenticpdf::adf::AdfDoc::sniff(&data) {
        return Err(PdfError::Unsupported(
            "only ADF documents carry provenance; import with `convert --to adf` first".into(),
        ));
    }
    let verdict = agenticpdf::agent_ops::verify(&data, text, section, block)?;
    let status = verdict["status"].as_str().unwrap_or("unrecorded");

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&verdict)
                .map_err(|e| PdfError::ExportError(e.to_string()))?
        );
    } else {
        match status {
            "matched" => {
                println!("matched — this text is what was imported");
                let page = verdict["source_page"].as_u64().unwrap_or(0);
                if page > 0 {
                    println!("  source page: {page}");
                }
            }
            "drifted" => {
                println!("drifted — the block exists but its text has changed since import")
            }
            _ => println!(
                "unrecorded — no provenance was stored for section {section}, block {block}"
            ),
        }
    }

    // A failed check exits non-zero, so a script or an agent can gate on it
    // without parsing the output.
    if status != "matched" {
        std::process::exit(1);
    }
    Ok(())
}

fn cmd_convert(file: &str, to: &str, sanitize: bool, output: Option<&str>) -> Result<(), PdfError> {
    let document = load_document(file)?;
    let document = if sanitize {
        document.sanitized()
    } else {
        document
    };

    // Bytes rather than a String: every target but one is text, but ADF is
    // binary, and widening the type is cleaner than a parallel code path.
    let result: Vec<u8> = match to.to_ascii_lowercase().as_str() {
        "markdown" | "md" | "gfm" => document.to_markdown().into_bytes(),
        "html" => document.to_html().into_bytes(),
        "text" | "txt" => document.extract_text().into_bytes(),
        "json" => serde_json::to_string_pretty(&document.extract_all(500, 50))
            .map_err(|e| PdfError::ExportError(e.to_string()))?
            .into_bytes(),
        "adf" => {
            let semantic = document.semantic().ok_or_else(|| {
                PdfError::Unsupported("this format has no semantic model to convert from".into())
            })?;

            // Provenance is recorded during import by `agent_ops::write_adf`;
            // without it every later citation reports as "unrecorded".
            agenticpdf::agent_ops::write_adf(semantic, file, document.format())
        }
        other => {
            return Err(PdfError::Unsupported(format!(
                "target format '{other}' (expected adf, markdown, html, text or json)"
            )));
        }
    };

    match output {
        Some(path) => fs::write(path, &result)?,
        // Binary on stdout would corrupt a terminal, and piping it is what
        // `--output` is for.
        None if to.eq_ignore_ascii_case("adf") => {
            return Err(PdfError::Unsupported(
                "ADF is binary; use --output to write it to a file".into(),
            ));
        }
        None => println!("{}", String::from_utf8_lossy(&result)),
    }
    eprintln!("Converted {} → {}", document.format().label(), to);
    Ok(())
}

/// List the formats this build can read.
#[cfg(feature = "cli")]
fn cmd_formats(format: &str) -> Result<(), PdfError> {
    let formats = agenticpdf::describe_formats();
    match format {
        "json" => {
            let json = serde_json::to_string_pretty(&formats)
                .map_err(|e| PdfError::ExportError(e.to_string()))?;
            println!("{}", json);
        }
        _ => {
            println!(
                "{:<10} {:<26} {:<10} EXTENSIONS",
                "ID", "FORMAT", "GEOMETRY"
            );
            for entry in formats.as_array().into_iter().flatten() {
                let extensions: Vec<&str> = entry["extensions"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|v| v.as_str())
                    .collect();
                println!(
                    "{:<10} {:<26} {:<10} {}",
                    entry["id"].as_str().unwrap_or(""),
                    entry["name"].as_str().unwrap_or(""),
                    // Whether the coordinates come from the file or from the
                    // typesetter — the one real difference left between formats.
                    if entry["authoredGeometry"].as_bool().unwrap_or(false) {
                        "authored"
                    } else {
                        "typeset"
                    },
                    extensions
                        .iter()
                        .map(|e| format!(".{e}"))
                        .collect::<Vec<_>>()
                        .join(" ")
                );
            }
            println!(
                "\nEvery listed format can be parsed. \"geometry\" says whether page\n\
                 coordinates are read from the file or computed by the typesetter."
            );
            println!("Format is detected from content; the file extension is only a hint.");
        }
    }
    Ok(())
}

#[cfg(feature = "cli")]
fn cmd_describe() -> Result<(), PdfError> {
    println!("{}", agenticpdf::describe_ontology());
    Ok(())
}

#[cfg(feature = "cli")]
fn cmd_info(format: &str) -> Result<(), PdfError> {
    match format {
        "json" => {
            let info = serde_json::json!({
                "name": "apdf",
                "version": "1.0.0",
                "description": "High-performance PDF processing CLI for agentic AI workflows",
                "license": "AGPL-3.0-or-later",
                "source": "https://github.com/nervosys/AgenticPDF",
                "capabilities": [
                    "text_extraction",
                    "unicode_decoding",
                    "markdown_export",
                    "reading_order_analysis",
                    "column_detection_xy_cut",
                    "heading_detection",
                    "table_reconstruction",
                    "figure_detection",
                    "caption_linking",
                    "prompt_injection_scan",
                    "sanitized_extraction",
                    "mcp_server",
                    "structured_layout",
                    "bounding_boxes",
                    "metadata_parsing",
                    "semantic_chunking",
                    "annotation_extraction",
                    "outline_extraction",
                    "image_enumeration",
                    "xref_stream_parsing",
                    "object_stream_parsing",
                    "encryption_detection",
                    "structured_json_output",
                    "page_range_filtering",
                    "file_output",
                    "wasm_compilation",
                    "ontology_self_description"
                ],
                "formats": ["text", "json", "markdown"],
                "commands": ["text", "markdown", "layout", "table", "structure", "forms", "displaylist", "scan", "figures", "formula", "scanned", "meta", "annotations", "outline", "images", "chunk", "all", "mcp", "describe", "info"],
                "discovery": "Run `apdf describe` for full JSON-LD ontology with output schemas and workflow templates."
            });
            println!(
                "{}",
                serde_json::to_string_pretty(&info)
                    .map_err(|e| PdfError::ExportError(e.to_string()))?
            );
        }
        _ => {
            println!("AgenticPDF CLI v1.0.0");
            println!("=====================");
            println!("High-performance PDF processing for agentic AI workflows.");
            println!();
            println!("Commands:");
            println!("  text         Extract text content with positioning and font metadata");
            println!("  markdown     Render reading-order Markdown (headings, paragraphs, lists)");
            println!("  layout       Reading-order structured blocks with bounding boxes (JSON)");
            println!("  table        Reconstruct tables (Markdown or JSON)");
            println!("  scan         Detect hidden / off-page text (prompt-injection signals)");
            println!("  structure    Extract tagged-PDF logical structure tree");
            println!("  forms        Extract interactive AcroForm fields");
            println!("  displaylist  Emit a page's render display list (for the WebGL renderer)");
            println!("  figures      Detect figures and link them to captions");
            println!("  formula      Detect formulas and reconstruct best-effort LaTeX");
            println!("  scanned      Detect likely-scanned pages that need OCR");
            println!("  meta         Show document metadata (title, author, dates, features)");
            println!("  annotations  Extract annotations (links, highlights, notes, widgets)");
            println!("  outline      Extract document outline / table of contents");
            println!("  images       List embedded images with dimensions and encoding");
            println!("  chunk        Generate semantic chunks for RAG pipelines");
            println!("  all          Extract everything in a single JSON output");
            println!("  mcp          Run as an MCP (Model Context Protocol) stdio server");
            println!(
                "  describe     Output machine-readable JSON-LD ontology for LLM discovery (alias: ontology)"
            );
            println!("  info         Show this info");
            println!();
            println!("Formats:  text, json");
            println!("License:  AGPL-3.0-or-later");
            println!("Source:   https://github.com/nervosys/AgenticPDF");
            println!();
            println!("Tip: Run `apdf describe` for the full machine-readable ontology.");
        }
    }

    Ok(())
}

#[cfg(all(test, feature = "cli"))]
mod surface_tests {
    use super::*;
    use clap::CommandFactory;

    /// Subcommands the CLI actually offers, excluding clap's built-in `help`.
    fn cli_commands() -> Vec<String> {
        Cli::command()
            .get_subcommands()
            .map(|sub| sub.get_name().to_string())
            .filter(|name| name != "help")
            .collect()
    }

    /// Command names the ontology advertises.
    fn ontology_commands() -> Vec<String> {
        let ontology = agenticpdf::build_ontology();
        ontology["commands"]
            .as_array()
            .expect("the ontology has no commands array")
            .iter()
            .filter_map(|entry| entry["name"].as_str().map(str::to_string))
            .collect()
    }

    #[test]
    fn the_ontology_describes_every_command_the_cli_offers() {
        // This has drifted twice: commands were added to the CLI while the
        // ontology kept describing the older program. An agent that trusts
        // `apdf describe` then cannot find capabilities that exist. Asserting
        // it here is the only thing that makes the ontology's accuracy a
        // property of the build rather than of whoever last remembered.
        let ontology = ontology_commands();
        let commands = cli_commands();
        let missing: Vec<&String> = commands
            .iter()
            .filter(|name| !ontology.contains(name))
            .collect();

        assert!(
            missing.is_empty(),
            "these commands exist but the ontology does not describe them: {missing:?}"
        );
    }

    #[test]
    fn the_ontology_does_not_advertise_commands_that_do_not_exist() {
        // The opposite failure, and the worse one: an agent planning around a
        // capability that was renamed or removed.
        let cli = cli_commands();
        let phantom: Vec<String> = ontology_commands()
            .into_iter()
            // `describe` and `info` are self-description commands that the
            // ontology lists for discovery; everything else must be real.
            .filter(|name| !cli.contains(name) && name != "describe" && name != "info")
            .collect();

        assert!(
            phantom.is_empty(),
            "the ontology advertises commands the CLI does not have: {phantom:?}"
        );
    }

    #[test]
    fn every_mcp_tool_maps_to_a_real_capability() {
        // MCP had no search, verify or convert tool at all until these were
        // reconciled — an agent over MCP could not reach the format's headline
        // features. Tool names are not identical to CLI names by design
        // (`extract_text` vs `text`), so this checks dispatch rather than
        // spelling: an unknown tool must be rejected, and a known one must not
        // report itself unknown.
        for tool in agenticpdf::mcp::tool_defs() {
            let name = tool["name"].as_str().expect("a tool with no name");
            assert!(
                !name.is_empty() && name.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
                "tool name {name:?} is not a usable identifier"
            );
            assert!(
                tool["description"].as_str().is_some_and(|d| d.len() > 30),
                "tool {name:?} has no usable description for an agent to choose it by"
            );
            assert!(
                tool["inputSchema"]["properties"].is_object(),
                "tool {name:?} has no input schema"
            );
        }
    }
}
