// SPDX-License-Identifier: AGPL-3.0-or-later
//! Reading-order analysis, block classification, and Markdown rendering.
//!
//! Takes the positioned text fragments produced by [`crate::engine`] and turns
//! them into reading-order blocks (headings, paragraphs, list items) with
//! bounding boxes — the structured, citation-ready output that agentic RAG
//! pipelines need. Includes a lightweight XY-cut column detector so two-column
//! layouts (papers, reports) are read in the correct order.

use crate::{PdfDocument, PdfPage, TextBlock};
use serde::{Deserialize, Serialize};

/// Classification of a reading-order block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockKind {
    Heading,
    Paragraph,
    ListItem,
}

/// A reading-order block with provenance for citations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub kind: BlockKind,
    pub text: String,
    /// Heading level (1-6); 0 for non-headings.
    pub level: u8,
    pub page_number: usize,
    /// Bounding box in PDF points: [left, bottom, right, top].
    pub bbox: [f64; 4],
    /// Representative font size in points.
    pub font_size: f64,
}

/// A page of reading-order blocks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredPage {
    pub page_number: usize,
    pub width: f64,
    pub height: f64,
    pub blocks: Vec<Block>,
}

/// A structured document: reading-order blocks for every page.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredDoc {
    pub pages: Vec<StructuredPage>,
}

// ----------------------------------------------------------------------------
// Internal line representation
// ----------------------------------------------------------------------------

struct Line {
    text: String,
    x0: f64,
    x1: f64,
    y_top: f64,
    y_bottom: f64,
    font_size: f64,
}

/// Analyze a document into reading-order structured pages.
pub fn analyze(doc: &PdfDocument) -> StructuredDoc {
    let pages = doc.pages.iter().map(analyze_page).collect();
    StructuredDoc { pages }
}

fn analyze_page(page: &PdfPage) -> StructuredPage {
    let frags: Vec<&TextBlock> = page
        .text_content
        .iter()
        .filter(|f| !f.text.trim().is_empty())
        .collect();

    // XY-cut: split into columns first, then cluster lines within each column,
    // ordering columns left-to-right and lines top-to-bottom.
    let mut ordered: Vec<Line> = Vec::new();
    for column in split_into_columns(frags, page.width) {
        let mut lines = build_lines(&column);
        lines.sort_by(|a, b| {
            b.y_top
                .partial_cmp(&a.y_top)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        ordered.extend(lines);
    }

    let blocks = group_blocks(ordered, page.page_number());
    StructuredPage {
        page_number: page.page_number(),
        width: page.width,
        height: page.height,
        blocks,
    }
}

impl PdfPage {
    fn page_number(&self) -> usize {
        self.index + 1
    }
}

/// Cluster fragments that share a baseline into lines.
fn build_lines(frags: &[&TextBlock]) -> Vec<Line> {
    let mut frags: Vec<&TextBlock> = frags.to_vec();
    if frags.is_empty() {
        return Vec::new();
    }
    // Sort top-to-bottom (PDF y grows upward), then left-to-right.
    frags.sort_by(|a, b| {
        b.y.partial_cmp(&a.y)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal))
    });

    let mut lines: Vec<Line> = Vec::new();
    for f in frags {
        let tol = (f.font_size.max(1.0)) * 0.5;
        // Find an existing line on roughly the same baseline.
        let placed = lines
            .iter_mut()
            .rev()
            .take(4)
            .find(|l| (l.y_bottom - f.y).abs() <= tol && f.x >= l.x0 - 1.0);
        match placed {
            Some(line) => {
                // Insert a space if there is a horizontal gap.
                let gap = f.x - line.x1;
                let needs_space = gap > f.font_size.max(1.0) * 0.25
                    && !line.text.ends_with(' ')
                    && !f.text.starts_with(' ');
                if needs_space {
                    line.text.push(' ');
                }
                line.text.push_str(&f.text);
                line.x1 = line.x1.max(f.x + f.width);
                line.x0 = line.x0.min(f.x);
                line.font_size = line.font_size.max(f.font_size);
                line.y_top = line.y_top.max(f.y + f.height);
            }
            None => lines.push(Line {
                text: f.text.clone(),
                x0: f.x,
                x1: f.x + f.width,
                y_top: f.y + f.height,
                y_bottom: f.y,
                font_size: f.font_size,
            }),
        }
    }
    lines
}

/// Split fragments into reading-order columns using a lightweight XY-cut: find
/// a vertical whitespace gap that cleanly divides the page, then return the
/// column fragment-sets ordered left-to-right. Returns a single group when the
/// page is single-column.
fn split_into_columns(frags: Vec<&TextBlock>, page_width: f64) -> Vec<Vec<&TextBlock>> {
    if frags.len() < 8 {
        return vec![frags];
    }
    let center = |f: &TextBlock| f.x + f.width / 2.0;
    match find_split(&frags, page_width) {
        Some(split) => {
            let (left, right): (Vec<&TextBlock>, Vec<&TextBlock>) =
                frags.into_iter().partition(|f| center(f) < split);
            vec![left, right]
        }
        None => vec![frags],
    }
}

/// Find a vertical whitespace gap that cleanly splits the fragments into two
/// reasonably-sized columns. Returns the x coordinate of the split, if any.
fn find_split(frags: &[&TextBlock], page_width: f64) -> Option<f64> {
    let min_gap = (page_width * 0.04).max(18.0);
    let mut intervals: Vec<(f64, f64)> = frags.iter().map(|f| (f.x, f.x + f.width)).collect();
    intervals.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    let mut cover_end = intervals[0].1;
    let mut best: Option<(f64, f64)> = None; // (gap_width, split_x)
    for &(x0, x1) in &intervals[1..] {
        if x0 > cover_end + min_gap {
            let gap = x0 - cover_end;
            let split = (cover_end + x0) / 2.0;
            if best.map(|(g, _)| gap > g).unwrap_or(true) {
                best = Some((gap, split));
            }
        }
        cover_end = cover_end.max(x1);
    }

    let (_, split) = best?;
    let center = |f: &TextBlock| f.x + f.width / 2.0;
    let left = frags.iter().filter(|f| center(f) < split).count();
    let right = frags.len() - left;
    let min_each = ((frags.len() as f64 * 0.15).ceil() as usize).max(3);
    if left >= min_each && right >= min_each {
        Some(split)
    } else {
        None
    }
}

/// Group ordered lines into classified blocks.
fn group_blocks(lines: Vec<Line>, page_number: usize) -> Vec<Block> {
    if lines.is_empty() {
        return Vec::new();
    }
    let body_size = mode_font_size(&lines);
    let mut blocks: Vec<Block> = Vec::new();
    let mut para: Option<Block> = None;

    let flush = |para: &mut Option<Block>, blocks: &mut Vec<Block>| {
        if let Some(b) = para.take() {
            blocks.push(b);
        }
    };

    let mut prev_bottom: Option<f64> = None;
    for line in &lines {
        let is_heading = line.font_size > body_size * 1.18;
        let bullet = detect_list_marker(&line.text);
        let line_height = line.font_size.max(1.0);
        let big_gap = prev_bottom
            .map(|pb| (pb - line.y_top) > line_height * 1.4)
            .unwrap_or(false);
        prev_bottom = Some(line.y_bottom);

        if is_heading {
            flush(&mut para, &mut blocks);
            blocks.push(Block {
                kind: BlockKind::Heading,
                text: line.text.trim().to_string(),
                level: heading_level(line.font_size, body_size),
                page_number,
                bbox: [line.x0, line.y_bottom, line.x1, line.y_top],
                font_size: line.font_size,
            });
            continue;
        }

        if let Some(stripped) = bullet {
            flush(&mut para, &mut blocks);
            blocks.push(Block {
                kind: BlockKind::ListItem,
                text: stripped,
                level: 0,
                page_number,
                bbox: [line.x0, line.y_bottom, line.x1, line.y_top],
                font_size: line.font_size,
            });
            continue;
        }

        // Paragraph accumulation.
        match &mut para {
            Some(b) if !big_gap => {
                if !b.text.ends_with(' ') {
                    b.text.push(' ');
                }
                b.text.push_str(line.text.trim());
                b.bbox[0] = b.bbox[0].min(line.x0);
                b.bbox[1] = b.bbox[1].min(line.y_bottom);
                b.bbox[2] = b.bbox[2].max(line.x1);
                b.bbox[3] = b.bbox[3].max(line.y_top);
            }
            _ => {
                flush(&mut para, &mut blocks);
                para = Some(Block {
                    kind: BlockKind::Paragraph,
                    text: line.text.trim().to_string(),
                    level: 0,
                    page_number,
                    bbox: [line.x0, line.y_bottom, line.x1, line.y_top],
                    font_size: line.font_size,
                });
            }
        }
    }
    flush(&mut para, &mut blocks);
    blocks
}

/// Most common rounded font size (the body text size).
fn mode_font_size(lines: &[Line]) -> f64 {
    use std::collections::HashMap;
    let mut counts: HashMap<i64, usize> = HashMap::new();
    for l in lines {
        *counts.entry(l.font_size.round() as i64).or_insert(0) += 1;
    }
    counts
        .into_iter()
        .max_by_key(|(_, c)| *c)
        .map(|(s, _)| s as f64)
        .unwrap_or(12.0)
        .max(1.0)
}

fn heading_level(size: f64, body: f64) -> u8 {
    let ratio = size / body.max(1.0);
    if ratio >= 2.0 {
        1
    } else if ratio >= 1.6 {
        2
    } else if ratio >= 1.35 {
        3
    } else {
        4
    }
}

/// If the line begins with a list marker, return the text without it.
fn detect_list_marker(text: &str) -> Option<String> {
    let t = text.trim_start();
    let mut chars = t.chars();
    let first = chars.next()?;
    if matches!(first, '•' | '◦' | '▪' | '‣' | '·' | '–' | '-' | '*') {
        let rest = chars.as_str().trim_start();
        if !rest.is_empty() {
            return Some(rest.to_string());
        }
    }
    // Numbered: "1." "1)" "12." up to a couple digits.
    let digits: String = t.chars().take_while(|c| c.is_ascii_digit()).collect();
    if !digits.is_empty() && digits.len() <= 3 {
        let after = &t[digits.len()..];
        if let Some(rest) = after.strip_prefix('.').or_else(|| after.strip_prefix(')')) {
            let rest = rest.trim_start();
            if !rest.is_empty() {
                return Some(rest.to_string());
            }
        }
    }
    None
}

// ----------------------------------------------------------------------------
// Markdown rendering
// ----------------------------------------------------------------------------

/// Render the whole document as Markdown.
pub fn to_markdown(doc: &PdfDocument) -> String {
    render_markdown(&analyze(doc))
}

/// Render an already-analyzed structured document as Markdown.
pub fn render_markdown(structured: &StructuredDoc) -> String {
    let mut out = String::new();
    for (i, page) in structured.pages.iter().enumerate() {
        if i > 0 {
            out.push_str("\n---\n\n");
        }
        for block in &page.blocks {
            match block.kind {
                BlockKind::Heading => {
                    let level = block.level.clamp(1, 6) as usize;
                    out.push_str(&"#".repeat(level));
                    out.push(' ');
                    out.push_str(&block.text);
                    out.push_str("\n\n");
                }
                BlockKind::ListItem => {
                    out.push_str("- ");
                    out.push_str(&block.text);
                    out.push('\n');
                }
                BlockKind::Paragraph => {
                    out.push_str(&block.text);
                    out.push_str("\n\n");
                }
            }
        }
    }
    // Collapse excessive blank lines.
    while out.contains("\n\n\n") {
        out = out.replace("\n\n\n", "\n\n");
    }
    out.trim_end().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frag(text: &str, x: f64, y: f64, size: f64) -> TextBlock {
        TextBlock {
            text: text.to_string(),
            x,
            y,
            width: text.len() as f64 * size * 0.5,
            height: size,
            font_size: size,
            font_name: "F1".into(),
            page_number: 1,
        }
    }

    #[test]
    fn heading_and_paragraph() {
        let page = PdfPage {
            index: 0,
            width: 612.0,
            height: 792.0,
            text_content: vec![
                frag("Big Title", 72.0, 700.0, 24.0),
                frag("Body line one.", 72.0, 670.0, 12.0),
                frag("Body line two.", 72.0, 656.0, 12.0),
            ],
        };
        let s = analyze_page(&page);
        assert_eq!(s.blocks[0].kind, BlockKind::Heading);
        assert_eq!(s.blocks[0].level, 1);
        assert_eq!(s.blocks[1].kind, BlockKind::Paragraph);
        assert!(s.blocks[1].text.contains("one"));
        assert!(s.blocks[1].text.contains("two"));
    }

    #[test]
    fn list_marker() {
        assert_eq!(detect_list_marker("• item").as_deref(), Some("item"));
        assert_eq!(detect_list_marker("1. first").as_deref(), Some("first"));
        assert_eq!(detect_list_marker("plain text"), None);
    }

    #[test]
    fn two_column_ordering() {
        // Left column lines high x0~50, right column x0~350.
        let mut frags = Vec::new();
        for i in 0..5 {
            frags.push(frag(
                &format!("L{}", i),
                50.0,
                700.0 - i as f64 * 20.0,
                12.0,
            ));
            frags.push(frag(
                &format!("R{}", i),
                350.0,
                700.0 - i as f64 * 20.0,
                12.0,
            ));
        }
        let page = PdfPage {
            index: 0,
            width: 612.0,
            height: 792.0,
            text_content: frags,
        };
        let s = analyze_page(&page);
        let joined: String = s
            .blocks
            .iter()
            .map(|b| b.text.clone())
            .collect::<Vec<_>>()
            .join(" ");
        // All left column should be read before right column.
        let l4 = joined.find("L4").unwrap();
        let r0 = joined.find("R0").unwrap();
        assert!(l4 < r0, "left column should precede right column: {joined}");
    }
}
