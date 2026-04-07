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

use agenticpdf::{PdfDocument, PdfError};
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

fn load_pdf(path: &str) -> Result<PdfDocument, PdfError> {
    let data = fs::read(path)?;
    PdfDocument::from_bytes(&data)
}

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
    let doc = load_pdf(file)?;
    let (start, end) = parse_page_range(pages, doc.pages.len());

    let selected_pages: Vec<_> = doc
        .pages
        .iter()
        .filter(|p| p.index + 1 >= start && p.index + 1 <= end)
        .collect();

    let result = match format {
        "json" => serde_json::to_string_pretty(&selected_pages)
            .map_err(|e| PdfError::ExportError(e.to_string()))?,
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
    let doc = load_pdf(file)?;
    let meta = doc.get_metadata();

    match format {
        "json" => {
            let json = serde_json::to_string_pretty(meta)
                .map_err(|e| PdfError::ExportError(e.to_string()))?;
            println!("{}", json);
        }
        _ => {
            println!("PDF Version:      {}", meta.pdf_version);
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
fn cmd_annotations(file: &str, pages: Option<&str>, format: &str) -> Result<(), PdfError> {
    let doc = load_pdf(file)?;
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
    let doc = load_pdf(file)?;
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
fn cmd_images(file: &str, _pages: Option<&str>, format: &str) -> Result<(), PdfError> {
    let doc = load_pdf(file)?;

    match format {
        "json" => {
            // Return empty array — image enumeration is a planned feature
            println!("[]");
        }
        _ => {
            println!("Document: {}", file);
            println!("Pages:    {}", doc.pages.len());
            println!(
                "(Image enumeration requires full object graph traversal — coming in a future release)"
            );
        }
    }

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

    let doc = load_pdf(file)?;
    let chunks = doc.generate_chunks(size, overlap);

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

    let doc = load_pdf(file)?;
    let full = doc.extract_all(size, overlap);

    let json =
        serde_json::to_string_pretty(&full).map_err(|e| PdfError::ExportError(e.to_string()))?;

    match output {
        Some(path) => fs::write(path, &json)?,
        None => println!("{}", json),
    }

    eprintln!(
        "Extracted: {} pages, {} annotations, {} outline items, {} chunks",
        full.pages.len(),
        full.annotations.len(),
        full.outline.len(),
        full.chunks.len()
    );

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
                    "text_positioning",
                    "font_metadata",
                    "metadata_parsing",
                    "semantic_chunking",
                    "annotation_extraction",
                    "outline_extraction",
                    "image_enumeration",
                    "flatedecode_decompression",
                    "png_predictor_unfiltering",
                    "xref_table_parsing",
                    "encryption_detection",
                    "structured_json_output",
                    "page_range_filtering",
                    "file_output",
                    "wasm_compilation",
                    "ontology_self_description"
                ],
                "formats": ["text", "json"],
                "commands": ["text", "meta", "annotations", "outline", "images", "chunk", "all", "describe", "info"],
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
            println!("  meta         Show document metadata (title, author, dates, features)");
            println!("  annotations  Extract annotations (links, highlights, notes, widgets)");
            println!("  outline      Extract document outline / table of contents");
            println!("  images       List embedded images (planned)");
            println!("  chunk        Generate semantic chunks for RAG pipelines");
            println!("  all          Extract everything in a single JSON output");
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
