//! AgenticPDF CLI — Command-line interface for PDF processing.
//!
//! Usage:
//!   agenticpdf text   <file>  [--pages 1-5] [--format json|text]
//!   agenticpdf meta   <file>  [--format json|text]
//!   agenticpdf images <file>  [--pages 1-5]
//!   agenticpdf chunk  <file>  [--size 500] [--overlap 50] [--format json|text]
//!   agenticpdf info

#[cfg(feature = "cli")]
use clap::{Parser, Subcommand};

use agenticpdf::{PdfDocument, PdfError};
use std::fs;
use std::process;

#[cfg(feature = "cli")]
#[derive(Parser)]
#[command(
    name = "agenticpdf",
    version = "1.0.0",
    about = "High-performance PDF processing CLI for agentic AI workflows",
    long_about = "AgenticPDF CLI — Extract text, metadata, images, and semantic chunks from PDF documents.\nOptimized for AI agent integration with structured JSON output."
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
    /// List images in a PDF
    Images {
        /// Path to the PDF file
        file: String,
        /// Page range
        #[arg(short, long)]
        pages: Option<String>,
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
    /// Show library info and capabilities
    Info,
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
            Commands::Images { file, pages } => cmd_images(&file, pages.as_deref()),
            Commands::Chunk {
                file,
                size,
                overlap,
                format,
                output,
            } => cmd_chunk(&file, size, overlap, &format, output.as_deref()),
            Commands::Info => cmd_info(),
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
            println!("PDF Version:   {}", meta.pdf_version);
            println!("Pages:         {}", meta.page_count);
            println!("File Size:     {} bytes", meta.file_size);
            println!("Encrypted:     {}", meta.encrypted);
            if let Some(ref title) = meta.title {
                println!("Title:         {}", title);
            }
            if let Some(ref author) = meta.author {
                println!("Author:        {}", author);
            }
        }
    }

    Ok(())
}

#[cfg(feature = "cli")]
fn cmd_images(file: &str, _pages: Option<&str>) -> Result<(), PdfError> {
    let doc = load_pdf(file)?;

    println!("Document: {}", file);
    println!("Pages:    {}", doc.pages.len());
    println!(
        "(Image enumeration requires full object graph traversal — coming in a future release)"
    );

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
fn cmd_info() -> Result<(), PdfError> {
    println!("AgenticPDF CLI v1.0.0");
    println!("=====================");
    println!("High-performance PDF processing for agentic AI workflows.");
    println!();
    println!("Capabilities:");
    println!("  - Text extraction with positioning and font metadata");
    println!("  - Document metadata parsing");
    println!("  - Semantic chunking for RAG pipelines");
    println!("  - FlateDecode / PNG predictor decompression");
    println!("  - Cross-reference table and stream parsing");
    println!("  - WASM compilation target for browser integration");
    println!();
    println!("Formats:  text, json");
    println!("License:  AGPL-3.0-or-later");
    println!("Source:   https://github.com/nervosys/AgenticPDF");

    Ok(())
}
