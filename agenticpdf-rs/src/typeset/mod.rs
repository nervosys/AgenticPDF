// SPDX-License-Identifier: AGPL-3.0-or-later
//! The typesetter: semantic structure in, page geometry out.
//!
//! This is the mirror image of [`crate::layout`]. That module takes a PDF's
//! coordinates and *infers* structure from them; this one takes a document's
//! authored structure and *computes* coordinates for it. Between them, both
//! kinds of source end up with both models.
//!
//! The output is deliberately the crate's existing geometric types — not a new
//! parallel set:
//!
//! - [`PdfPage`] of [`TextBlock`]s, so [`crate::layout`], [`crate::tables`],
//!   [`crate::sanitize`] and chunking work on a `.docx` through the same code
//!   they use for a PDF.
//! - [`DisplayList`] of [`RenderOp`]s, so `render/webgl-renderer.ts` draws a
//!   `.docx` with no change to the renderer at all.
//! - [`PageGraphics`] rulings, so table *reconstruction* round-trips against
//!   the tables the typesetter drew.
//!
//! ## Two coordinate systems
//!
//! Layout runs top-down — that is how documents are written and how page
//! breaking is naturally expressed — while PDF space is y-up with the origin at
//! the bottom left. The flow phase works in top-down "page-relative" units and
//! [`Page::flip`] converts once, at emission. Keeping the conversion to a single
//! place is what stops sign errors leaking into every block type.
//!
//! ## Scope
//!
//! Greedy first-fit line breaking, not Knuth–Plass. Widths come from the
//! standard-14 metrics in [`fonts`], so a document asking for Calibri is laid
//! out in Helvetica's metrics and rendered in the browser's Helvetica — an
//! approximation, but a *self-consistent* one: the same widths decide the line
//! breaks and position the glyphs, so text never overflows the margin it was
//! broken to.

pub mod fonts;

#[cfg(test)]
mod tests;

use crate::PdfAnnotation;
use crate::doc::{
    Align, Block, ImageRef, Inline, PageSize, Run, SectionKind, SemanticDoc, Table, TextStyle,
};
use crate::engine::{DisplayList, PageGraphics, PageImage, RenderOp, Seg};
use crate::{PdfPage, TextBlock};
use fonts::{Font, measure};

/// Body text size when the source states none.
const BODY_SIZE: f64 = 11.0;
/// Heading sizes for levels 1-6.
const HEADING_SIZES: [f64; 6] = [22.0, 17.0, 14.0, 12.0, 11.0, 10.0];
/// Preformatted text size.
const CODE_SIZE: f64 = 9.5;

/// Horizontal space reserved for a list marker, in points.
const MARKER_WIDTH: f64 = 18.0;
/// Indent added per list nesting level.
const LIST_INDENT: f64 = 18.0;
/// Indent added by a block quote.
const QUOTE_INDENT: f64 = 24.0;
/// Padding inside a table cell.
const CELL_PADDING: f64 = 4.0;
/// Ruling and rule stroke width.
const RULE_WIDTH: f64 = 0.6;

/// Cap on generated pages, bounding a pathological document.
const MAX_PAGES: usize = 20_000;

/// Geometry computed for a document.
///
/// All four page-indexed vectors are the same length, so a consumer holding a
/// page number can index any of them.
#[derive(Debug, Default)]
pub struct Typeset {
    pub pages: Vec<PdfPage>,
    pub graphics: Vec<PageGraphics>,
    /// Decoded image placements, per page.
    pub images: Vec<Vec<PageImage>>,
    pub display: Vec<DisplayList>,
    /// Hyperlinks, as clickable rectangles.
    pub annotations: Vec<PdfAnnotation>,
}

impl Typeset {
    /// Image placements in the form figure detection expects.
    ///
    /// Lets `apdf figures` link a `.docx` image to its caption through the same
    /// proximity matching it uses on a PDF.
    pub fn placed_images(&self) -> Vec<crate::engine::PlacedImage> {
        let mut placed = Vec::new();
        for (index, list) in self.display.iter().enumerate() {
            let page_number = index + 1;
            for op in &list.ops {
                let RenderOp::Image { x, y, w, h, name } = op else {
                    continue;
                };
                let pixels = self.images[index]
                    .iter()
                    .find(|image| &image.name == name)
                    .map(|image| (image.width, image.height))
                    .unwrap_or((0, 0));
                placed.push(crate::engine::PlacedImage {
                    page_number,
                    name: name.clone(),
                    bbox: [*x, *y, *x + *w, *y + *h],
                    width: pixels.0,
                    height: pixels.1,
                    color_space: String::new(),
                });
            }
        }
        placed
    }
}

/// Lay out a semantic document into pages.
pub fn typeset(document: &SemanticDoc) -> Typeset {
    let mut output = Typeset::default();

    for section in &document.sections {
        let geometry = section.page_size.unwrap_or_default();
        let mut flow = Flow::new(document, geometry.content_width());

        // A slide or sheet title is the page's heading; a flow section's title
        // is metadata that has already been used elsewhere.
        if section.kind != SectionKind::Flow
            && let Some(title) = &section.title
            && !title.trim().is_empty()
        {
            flow.heading(2, title);
        }
        flow.blocks(&section.blocks, 0.0);
        if !section.notes.is_empty() {
            flow.gap(BODY_SIZE);
            flow.heading(3, "Speaker notes");
            flow.blocks(&section.notes, QUOTE_INDENT);
        }

        let items = flow.finish();
        // A slide or worksheet is one authored page; only a reflowable section
        // is broken across pages by the typesetter.
        let single_page = section.kind != SectionKind::Flow;
        paginate(items, geometry, single_page, document, &mut output);
    }

    // A document with no content still has one page, so page counts and
    // renderers have something coherent to work with.
    if output.pages.is_empty() {
        let geometry = PageSize::default();
        emit_page(Page::new(geometry), document, &mut output);
    }

    output
}

// ============================================================================
// Flowed items
// ============================================================================

/// One laid-out line of text.
#[derive(Debug, Clone)]
struct Line {
    pieces: Vec<Piece>,
    /// Total height of the line box.
    height: f64,
    /// Distance from the top of the line box to the baseline.
    ascent: f64,
}

/// A run of characters on a line, sharing one font and style.
#[derive(Debug, Clone)]
struct Piece {
    text: String,
    /// Offset from the content box's left edge.
    x: f64,
    width: f64,
    advances: Vec<f64>,
    font: Font,
    size: f64,
    color: [f64; 4],
    underline: bool,
    strikethrough: bool,
    /// Hidden runs are laid out but not painted, so a sanitised and an
    /// unsanitised render of the same document share their geometry.
    hidden: bool,
    link: Option<String>,
}

/// A unit of flowed content, before pagination assigns it a page and a `y`.
#[derive(Debug, Clone)]
enum Item {
    Line(Line),
    /// Vertical space.
    Gap(f64),
    /// A horizontal rule spanning the content width.
    Rule,
    Image {
        asset_id: String,
        width: f64,
        height: f64,
    },
    Table(TableBox),
    /// An author-requested page break.
    PageBreak,
}

impl Item {
    /// Height this item occupies.
    fn height(&self) -> f64 {
        match self {
            Item::Line(line) => line.height,
            Item::Gap(height) => *height,
            Item::Rule => RULE_WIDTH * 4.0,
            Item::Image { height, .. } => *height,
            Item::Table(table) => table.height(),
            Item::PageBreak => 0.0,
        }
    }
}

/// A laid-out table: column edges plus per-row cell content.
#[derive(Debug, Clone)]
struct TableBox {
    /// Column boundary offsets from the content left edge; `columns.len()` is
    /// the column count plus one.
    columns: Vec<f64>,
    rows: Vec<TableRowBox>,
    header_rows: usize,
}

#[derive(Debug, Clone)]
struct TableRowBox {
    height: f64,
    cells: Vec<TableCellBox>,
}

#[derive(Debug, Clone)]
struct TableCellBox {
    /// Index of the column this cell starts at.
    column: usize,
    col_span: usize,
    lines: Vec<Line>,
}

impl TableBox {
    fn height(&self) -> f64 {
        self.rows.iter().map(|row| row.height).sum()
    }

    fn width(&self) -> f64 {
        self.columns.last().copied().unwrap_or(0.0)
    }
}

// ============================================================================
// Flow: blocks to items
// ============================================================================

struct Flow<'a> {
    document: &'a SemanticDoc,
    /// Width available to content.
    width: f64,
    items: Vec<Item>,
}

impl<'a> Flow<'a> {
    fn new(document: &'a SemanticDoc, width: f64) -> Flow<'a> {
        Flow {
            document,
            width: width.max(24.0),
            items: Vec::new(),
        }
    }

    fn finish(self) -> Vec<Item> {
        self.items
    }

    fn gap(&mut self, height: f64) {
        if height > 0.0 {
            self.items.push(Item::Gap(height));
        }
    }

    fn heading(&mut self, level: u8, text: &str) {
        let content = vec![Inline::Run(Run::plain(text))];
        self.paragraph(&content, ParagraphStyle::heading(level), 0.0, None);
    }

    fn blocks(&mut self, blocks: &[Block], indent: f64) {
        for block in blocks {
            self.block(block, indent);
        }
    }

    fn block(&mut self, block: &Block, indent: f64) {
        match block {
            Block::Heading { level, content } => {
                let style = ParagraphStyle::heading(*level);
                self.gap(style.size * 0.7);
                self.paragraph(content, style, indent, None);
            }
            Block::Paragraph {
                content,
                align,
                indent: extra,
            } => {
                let style = ParagraphStyle {
                    align: *align,
                    ..ParagraphStyle::body()
                };
                self.paragraph(content, style, indent + extra, None);
                self.gap(BODY_SIZE * 0.45);
            }
            Block::List(list) => {
                for (offset, item) in list.items.iter().enumerate() {
                    let marker = match (list.ordered, item.checked) {
                        (_, Some(true)) => "[x]".to_string(),
                        (_, Some(false)) => "[ ]".to_string(),
                        (true, None) => format!("{}.", list.start + offset as u64),
                        (false, None) => "\u{2022}".to_string(),
                    };
                    self.list_item(&item.blocks, indent, &marker);
                }
                self.gap(BODY_SIZE * 0.4);
            }
            Block::Table(table) => {
                if let Some(laid_out) = self.table(table, indent) {
                    self.items.push(Item::Table(laid_out));
                    self.gap(BODY_SIZE * 0.5);
                }
            }
            Block::Quote(inner) => {
                self.blocks(inner, indent + QUOTE_INDENT);
            }
            Block::Code { text, .. } => {
                self.code(text, indent);
                self.gap(BODY_SIZE * 0.45);
            }
            Block::Figure { image, caption } => {
                self.image(image, indent);
                if let Some(caption) = caption
                    && !caption.trim().is_empty()
                {
                    let content = vec![Inline::Run(Run::styled(
                        caption.trim(),
                        TextStyle {
                            italic: true,
                            ..TextStyle::default()
                        },
                    ))];
                    self.paragraph(
                        &content,
                        ParagraphStyle {
                            align: Align::Center,
                            size: BODY_SIZE - 1.0,
                            ..ParagraphStyle::body()
                        },
                        indent,
                        None,
                    );
                }
                self.gap(BODY_SIZE * 0.5);
            }
            Block::Divider => {
                self.gap(BODY_SIZE * 0.4);
                self.items.push(Item::Rule);
                self.gap(BODY_SIZE * 0.4);
            }
            Block::PageBreak => self.items.push(Item::PageBreak),
        }
    }

    /// Flow a list item: its first block gets the marker, the rest are indented
    /// under it.
    fn list_item(&mut self, blocks: &[Block], indent: f64, marker: &str) {
        let inner = indent + LIST_INDENT;
        let mut first = true;
        for block in blocks {
            match block {
                Block::Paragraph { content, align, .. } if first => {
                    first = false;
                    let style = ParagraphStyle {
                        align: *align,
                        ..ParagraphStyle::body()
                    };
                    self.paragraph(content, style, inner, Some(marker));
                }
                // A nested list indents relative to this item's text.
                other => self.block(other, inner),
            }
        }
        if first {
            // An item whose first block was not a paragraph still needs its
            // marker drawn.
            let content = vec![Inline::Run(Run::plain(""))];
            self.paragraph(&content, ParagraphStyle::body(), inner, Some(marker));
        }
    }

    /// Break inline content into lines and append them.
    fn paragraph(
        &mut self,
        content: &[Inline],
        style: ParagraphStyle,
        indent: f64,
        marker: Option<&str>,
    ) {
        let available = (self.width - indent).max(24.0);
        let tokens = self.tokenize(content, &style);
        let lines = break_lines(&tokens, available, style.align);

        for (index, mut line) in lines.into_iter().enumerate() {
            // Shift the whole line right by the indent.
            for piece in &mut line.pieces {
                piece.x += indent;
            }
            // The marker hangs in the margin to the left of the first line.
            if index == 0
                && let Some(marker) = marker
            {
                let font = Font::resolve(&TextStyle::default());
                let (width, advances) = measure(marker, font, style.size);
                line.pieces.insert(
                    0,
                    Piece {
                        text: marker.to_string(),
                        x: indent - MARKER_WIDTH,
                        width,
                        advances,
                        font,
                        size: style.size,
                        color: [0.0, 0.0, 0.0, 1.0],
                        underline: false,
                        strikethrough: false,
                        hidden: false,
                        link: None,
                    },
                );
            }
            self.items.push(Item::Line(line));
        }
    }

    /// Lay out preformatted text: no wrapping, one line per source line.
    fn code(&mut self, text: &str, indent: f64) {
        let font = Font {
            family: fonts::Family::Monospace,
            bold: false,
            italic: false,
        };
        for source in text.lines() {
            let (width, advances) = measure(source, font, CODE_SIZE);
            let piece = Piece {
                text: source.to_string(),
                x: indent + CELL_PADDING,
                width,
                advances,
                font,
                size: CODE_SIZE,
                color: [0.1, 0.1, 0.1, 1.0],
                underline: false,
                strikethrough: false,
                hidden: false,
                link: None,
            };
            self.items.push(Item::Line(Line {
                pieces: vec![piece],
                height: font.line_height(CODE_SIZE),
                ascent: font.ascent(CODE_SIZE),
            }));
        }
    }

    /// Place an image, scaled to fit the content width.
    fn image(&mut self, image: &ImageRef, indent: f64) {
        let available = (self.width - indent).max(24.0);
        let asset = self.document.asset(&image.asset_id);

        // Prefer the display size the document asked for; fall back to the
        // image's intrinsic pixels at 96 dpi, which is what Office assumes.
        let (mut width, mut height) = match (image.width, image.height) {
            (Some(width), Some(height)) if width > 0.0 && height > 0.0 => (width, height),
            _ => match asset {
                Some(asset) if asset.width > 0 && asset.height > 0 => (
                    asset.width as f64 * 72.0 / 96.0,
                    asset.height as f64 * 72.0 / 96.0,
                ),
                _ => (available * 0.5, available * 0.3),
            },
        };

        if width > available {
            height *= available / width;
            width = available;
        }

        self.items.push(Item::Image {
            asset_id: image.asset_id.clone(),
            width,
            height,
        });
    }

    /// Lay out a table: resolve column widths, then flow each cell.
    fn table(&mut self, table: &Table, indent: f64) -> Option<TableBox> {
        if table.rows.is_empty() {
            return None;
        }
        let available = (self.width - indent).max(48.0);

        // The column count is the widest row once spans are counted, so a
        // ragged table still gets a rectangular grid.
        let column_count = table
            .rows
            .iter()
            .map(|row| {
                row.cells
                    .iter()
                    .map(|cell| cell.col_span.max(1))
                    .sum::<usize>()
            })
            .max()
            .unwrap_or(0)
            .max(1);

        let widths = column_widths(table, column_count, available);
        let mut columns = Vec::with_capacity(column_count + 1);
        let mut edge = indent;
        columns.push(edge);
        for width in &widths {
            edge += width;
            columns.push(edge);
        }

        let mut rows = Vec::with_capacity(table.rows.len());
        for source in &table.rows {
            let mut cells = Vec::with_capacity(source.cells.len());
            let mut column = 0usize;
            let mut tallest: f64 = 0.0;

            for cell in &source.cells {
                if column >= column_count {
                    break;
                }
                let span = cell.col_span.max(1).min(column_count - column);
                let cell_width = (widths[column..column + span].iter().sum::<f64>()
                    - CELL_PADDING * 2.0)
                    .max(12.0);

                // A cell is its own flow context: its content wraps to the
                // column, not to the page.
                let mut inner = Flow::new(self.document, cell_width);
                inner.blocks(&cell.blocks, 0.0);
                let lines: Vec<Line> = inner
                    .finish()
                    .into_iter()
                    .filter_map(|item| match item {
                        Item::Line(line) => Some(line),
                        _ => None,
                    })
                    .collect();

                let content_height: f64 = lines.iter().map(|line| line.height).sum();
                tallest = tallest.max(content_height);
                cells.push(TableCellBox {
                    column,
                    col_span: span,
                    lines,
                });
                column += span;
            }

            rows.push(TableRowBox {
                height: tallest.max(BODY_SIZE) + CELL_PADDING * 2.0,
                cells,
            });
        }

        Some(TableBox {
            columns,
            rows,
            header_rows: table.header_rows.min(table.rows.len()),
        })
    }

    /// Convert inline content into measured tokens.
    fn tokenize(&self, content: &[Inline], style: &ParagraphStyle) -> Vec<Token> {
        let mut tokens = Vec::new();
        for inline in content {
            match inline {
                Inline::Run(run) => push_run_tokens(run, style, None, &mut tokens),
                Inline::Link { href, runs } => {
                    for run in runs {
                        push_run_tokens(run, style, Some(href.as_str()), &mut tokens);
                    }
                }
                Inline::Break => tokens.push(Token::hard_break()),
                // An inline image is approximated by its alt text; a figure
                // block is the path that places actual pixels.
                Inline::Image(image) => {
                    if let Some(alt) = &image.alt {
                        let run = Run::styled(
                            format!("[{alt}]"),
                            TextStyle {
                                italic: true,
                                ..TextStyle::default()
                            },
                        );
                        push_run_tokens(&run, style, None, &mut tokens);
                    }
                }
                Inline::FootnoteRef { index } => {
                    let run = Run::styled(
                        format!("[{}]", index + 1),
                        TextStyle {
                            superscript: true,
                            ..TextStyle::default()
                        },
                    );
                    push_run_tokens(&run, style, None, &mut tokens);
                }
            }
        }
        tokens
    }
}

/// Paragraph-level formatting resolved before flowing.
#[derive(Debug, Clone, Copy)]
struct ParagraphStyle {
    size: f64,
    bold: bool,
    align: Align,
}

impl ParagraphStyle {
    fn body() -> ParagraphStyle {
        ParagraphStyle {
            size: BODY_SIZE,
            bold: false,
            align: Align::Left,
        }
    }

    fn heading(level: u8) -> ParagraphStyle {
        let index = (level.clamp(1, 6) - 1) as usize;
        ParagraphStyle {
            size: HEADING_SIZES[index],
            bold: true,
            align: Align::Left,
        }
    }
}

// ============================================================================
// Tokenizing and line breaking
// ============================================================================

/// One breakable unit of text.
#[derive(Debug, Clone)]
struct Token {
    text: String,
    width: f64,
    font: Font,
    size: f64,
    color: [f64; 4],
    underline: bool,
    strikethrough: bool,
    hidden: bool,
    link: Option<String>,
    /// Whitespace: collapsible at a line break, and the elastic part of a
    /// justified line.
    is_space: bool,
    /// An explicit `<br>`-style break.
    is_break: bool,
}

impl Token {
    fn hard_break() -> Token {
        Token {
            text: String::new(),
            width: 0.0,
            font: Font::resolve(&TextStyle::default()),
            size: BODY_SIZE,
            color: [0.0, 0.0, 0.0, 1.0],
            underline: false,
            strikethrough: false,
            hidden: false,
            link: None,
            is_space: false,
            is_break: true,
        }
    }
}

/// Split a styled run into tokens, measuring each.
fn push_run_tokens(
    run: &Run,
    paragraph: &ParagraphStyle,
    link: Option<&str>,
    into: &mut Vec<Token>,
) {
    if run.text.is_empty() {
        return;
    }

    let mut style = run.style.clone();
    style.bold |= paragraph.bold;
    let font = Font::resolve(&style);
    // A run's own size wins; otherwise the paragraph decides. Super- and
    // subscripts are drawn smaller, as they are everywhere else.
    let mut size = style.size.unwrap_or(paragraph.size);
    if style.superscript || style.subscript {
        size *= 0.72;
    }

    let color = style
        .color
        .map(|[r, g, b]| [r, g, b, 1.0])
        .unwrap_or([0.0, 0.0, 0.0, 1.0]);

    let make = |text: String, is_space: bool| {
        let width = fonts::width_of(&text, font, size);
        Token {
            text,
            width,
            font,
            size,
            color,
            underline: style.underline,
            strikethrough: style.strikethrough,
            hidden: style.hidden,
            link: link.map(str::to_string),
            is_space,
            is_break: false,
        }
    };

    // Split into runs of whitespace and runs of non-whitespace, and break CJK
    // between every character — those scripts wrap mid-"word" and have no
    // spaces to break at.
    let mut current = String::new();
    let mut in_space = false;

    for ch in run.text.chars() {
        let is_space = ch.is_whitespace();
        if fonts::is_wide(ch) {
            if !current.is_empty() {
                into.push(make(std::mem::take(&mut current), in_space));
            }
            into.push(make(ch.to_string(), false));
            in_space = false;
            continue;
        }
        if is_space != in_space && !current.is_empty() {
            into.push(make(std::mem::take(&mut current), in_space));
        }
        in_space = is_space;
        current.push(ch);
    }
    if !current.is_empty() {
        into.push(make(current, in_space));
    }
}

/// Greedy first-fit line breaking.
fn break_lines(tokens: &[Token], width: f64, align: Align) -> Vec<Line> {
    let mut lines = Vec::new();
    let mut current: Vec<Token> = Vec::new();
    let mut used = 0.0f64;

    let mut flush = |current: &mut Vec<Token>, used: &mut f64, last: bool| {
        // Trailing spaces never affect alignment or justification.
        while current.last().is_some_and(|token| token.is_space) {
            current.pop();
        }
        if !current.is_empty() {
            lines.push(assemble(current, width, align, last));
        }
        current.clear();
        *used = 0.0;
    };

    for token in tokens {
        if token.is_break {
            flush(&mut current, &mut used, true);
            continue;
        }
        // Leading whitespace on a wrapped line is dropped, not printed.
        if token.is_space && current.is_empty() {
            continue;
        }

        if used + token.width > width && !current.is_empty() {
            flush(&mut current, &mut used, false);
            if token.is_space {
                continue;
            }
        }
        used += token.width;
        current.push(token.clone());
    }
    flush(&mut current, &mut used, true);

    lines
}

/// Turn a line's tokens into positioned pieces, applying alignment.
fn assemble(tokens: &[Token], width: f64, align: Align, last_line: bool) -> Line {
    let content_width: f64 = tokens.iter().map(|token| token.width).sum();
    let slack = (width - content_width).max(0.0);

    // Justification stretches the spaces; the last line of a paragraph is left
    // alone, which is what every typesetter does and what looks right.
    let space_count = tokens.iter().filter(|token| token.is_space).count();
    let (start, extra_per_space) = match align {
        Align::Center => (slack / 2.0, 0.0),
        Align::Right => (slack, 0.0),
        Align::Justify if !last_line && space_count > 0 => (0.0, slack / space_count as f64),
        _ => (0.0, 0.0),
    };

    let mut pieces: Vec<Piece> = Vec::new();
    let mut x = start;
    for token in tokens {
        let stretch = if token.is_space { extra_per_space } else { 0.0 };
        let advance = token.width + stretch;

        let (_, mut advances) = measure(&token.text, token.font, token.size);
        // Justification widens the spaces themselves rather than inserting gaps
        // between pieces. Folding the stretch into the space's own advance is
        // what lets a justified line stay a single merged run: the following
        // glyphs are still positioned by the advance array, so they land where
        // the layout put them.
        if stretch != 0.0
            && let Some(last) = advances.last_mut()
        {
            *last += stretch;
        }

        // Merge into the previous piece when nothing about the styling changed,
        // so a paragraph is a handful of draw calls rather than one per word —
        // and so the spaces between words survive into the extracted text.
        let mergeable = pieces.last().is_some_and(|last| {
            last.font == token.font
                && last.size == token.size
                && last.color == token.color
                && last.underline == token.underline
                && last.strikethrough == token.strikethrough
                && last.hidden == token.hidden
                && last.link == token.link
        });

        if mergeable {
            let last = pieces.last_mut().expect("just checked");
            last.text.push_str(&token.text);
            last.advances.extend(advances);
            last.width += advance;
        } else {
            pieces.push(Piece {
                text: token.text.clone(),
                x,
                width: advance,
                advances,
                font: token.font,
                size: token.size,
                color: token.color,
                underline: token.underline,
                strikethrough: token.strikethrough,
                hidden: token.hidden,
                link: token.link.clone(),
            });
        }
        x += advance;
    }

    let size = tokens
        .iter()
        .map(|token| token.size)
        .fold(0.0f64, f64::max)
        .max(BODY_SIZE * 0.5);
    let font = tokens
        .first()
        .map(|token| token.font)
        .unwrap_or_else(|| Font::resolve(&TextStyle::default()));

    Line {
        pieces,
        height: font.line_height(size),
        ascent: font.ascent(size),
    }
}

/// Resolve column widths, honouring the source's proportions when it gives any.
fn column_widths(table: &Table, columns: usize, available: f64) -> Vec<f64> {
    let declared = &table.column_widths;
    if declared.len() == columns {
        let total: f64 = declared.iter().sum();
        if total > 0.0 {
            // The source's widths are proportions, not absolutes: a table
            // authored for a wider page must still fit this one.
            return declared
                .iter()
                .map(|width| width / total * available)
                .collect();
        }
    }
    vec![available / columns as f64; columns]
}

// ============================================================================
// Pagination
// ============================================================================

/// A page under construction, in top-down coordinates.
struct Page {
    geometry: PageSize,
    /// Distance from the top margin to the next free position.
    cursor: f64,
    placed: Vec<Placed>,
}

/// An item with its final position on a page.
struct Placed {
    /// Distance from the top of the content box to the item's top edge.
    top: f64,
    item: Item,
}

impl Page {
    fn new(geometry: PageSize) -> Page {
        Page {
            geometry,
            cursor: 0.0,
            placed: Vec::new(),
        }
    }

    fn remaining(&self) -> f64 {
        self.geometry.content_height() - self.cursor
    }

    fn is_empty(&self) -> bool {
        self.placed.is_empty()
    }

    /// Convert a top-down offset within the content box into a PDF y.
    fn flip(&self, top: f64) -> f64 {
        self.geometry.height - self.geometry.margin_top - top
    }

    /// The PDF x of a content-box offset.
    fn left(&self, x: f64) -> f64 {
        self.geometry.margin_left + x
    }
}

/// Assign items to pages.
fn paginate(
    items: Vec<Item>,
    geometry: PageSize,
    single_page: bool,
    document: &SemanticDoc,
    output: &mut Typeset,
) {
    let mut page = Page::new(geometry);

    for item in items {
        if output.pages.len() >= MAX_PAGES {
            break;
        }

        match item {
            Item::PageBreak if !single_page => {
                if !page.is_empty() {
                    emit_page(
                        std::mem::replace(&mut page, Page::new(geometry)),
                        document,
                        output,
                    );
                }
                continue;
            }
            Item::PageBreak => continue,

            // A table is placed row by row so a long one can span pages,
            // repeating its header on each.
            Item::Table(table) if !single_page => {
                place_table(table, &mut page, geometry, document, output);
                continue;
            }

            item => {
                let height = item.height();
                // Never break a page for a gap: whitespace at a page boundary
                // is simply dropped.
                if !single_page && height > page.remaining() && !page.is_empty() {
                    if matches!(item, Item::Gap(_)) {
                        continue;
                    }
                    emit_page(
                        std::mem::replace(&mut page, Page::new(geometry)),
                        document,
                        output,
                    );
                }
                let top = page.cursor;
                page.cursor += height;
                page.placed.push(Placed { top, item });
            }
        }
    }

    if !page.is_empty() || output.pages.is_empty() {
        emit_page(page, document, output);
    }
}

/// Place a table, splitting it across pages by row.
fn place_table(
    table: TableBox,
    page: &mut Page,
    geometry: PageSize,
    document: &SemanticDoc,
    output: &mut Typeset,
) {
    let header: Vec<TableRowBox> = table.rows[..table.header_rows.min(table.rows.len())].to_vec();
    let mut pending: Vec<TableRowBox> = Vec::new();
    let mut repeat_header = false;

    let flush = |pending: &mut Vec<TableRowBox>, page: &mut Page, repeat: bool| {
        if pending.is_empty() {
            return;
        }
        let rows = std::mem::take(pending);
        let header_rows = if repeat {
            header.len()
        } else {
            table.header_rows
        };
        let block = TableBox {
            columns: table.columns.clone(),
            header_rows: header_rows.min(rows.len()),
            rows,
        };
        let top = page.cursor;
        page.cursor += block.height();
        page.placed.push(Placed {
            top,
            item: Item::Table(block),
        });
    };

    for row in &table.rows {
        let pending_height: f64 = pending.iter().map(|row| row.height).sum();
        if pending_height + row.height > page.remaining()
            && !(page.is_empty() && pending.is_empty())
        {
            flush(&mut pending, page, repeat_header);
            emit_page(
                std::mem::replace(page, Page::new(geometry)),
                document,
                output,
            );
            // Continuation pages restate the header, so a split table still
            // reads as a table.
            if !header.is_empty() {
                pending.extend(header.iter().cloned());
                repeat_header = true;
            }
        }
        pending.push(row.clone());
    }
    flush(&mut pending, page, repeat_header);
}

// ============================================================================
// Emission
// ============================================================================

/// Turn a finished page into the crate's geometric types.
fn emit_page(page: Page, document: &SemanticDoc, output: &mut Typeset) {
    let page_number = output.pages.len() + 1;
    let mut text_content: Vec<TextBlock> = Vec::new();
    let mut ops: Vec<RenderOp> = Vec::new();
    let mut h_lines: Vec<Seg> = Vec::new();
    let mut v_lines: Vec<Seg> = Vec::new();
    let mut images: Vec<PageImage> = Vec::new();

    for placed in &page.placed {
        match &placed.item {
            Item::Line(line) => emit_line(
                line,
                &page,
                placed.top,
                page_number,
                &mut text_content,
                &mut ops,
                &mut output.annotations,
            ),
            Item::Gap(_) | Item::PageBreak => {}
            Item::Rule => {
                let y = page.flip(placed.top + RULE_WIDTH * 2.0);
                let left = page.left(0.0);
                let right = page.left(page.geometry.content_width());
                ops.push(RenderOp::Stroke {
                    subpaths: vec![vec![[left, y], [right, y]]],
                    color: [0.6, 0.6, 0.6, 1.0],
                    width: RULE_WIDTH,
                });
                h_lines.push(Seg {
                    x0: left,
                    y0: y,
                    x1: right,
                    y1: y,
                });
            }
            Item::Image {
                asset_id,
                width,
                height,
            } => {
                let x = page.left(0.0);
                let y = page.flip(placed.top + height);
                ops.push(RenderOp::Image {
                    x,
                    y,
                    w: *width,
                    h: *height,
                    name: asset_id.clone(),
                });
                if let Some(image) = page_image(document, asset_id, x, y, *width, *height)
                    && !images.iter().any(|existing| existing.name == image.name)
                {
                    images.push(image);
                }
            }
            Item::Table(table) => emit_table(
                table,
                &page,
                placed.top,
                page_number,
                &mut text_content,
                &mut ops,
                &mut h_lines,
                &mut v_lines,
                &mut output.annotations,
            ),
        }
    }

    output.pages.push(PdfPage {
        index: page_number - 1,
        width: page.geometry.width,
        height: page.geometry.height,
        text_content,
    });
    output.graphics.push(PageGraphics {
        page_number,
        width: page.geometry.width,
        height: page.geometry.height,
        h_lines,
        v_lines,
    });
    output.display.push(DisplayList {
        page_number,
        width: page.geometry.width,
        height: page.geometry.height,
        ops,
    });
    output.images.push(images);
}

/// Emit one line's text blocks and draw operations.
fn emit_line(
    line: &Line,
    page: &Page,
    top: f64,
    page_number: usize,
    text_content: &mut Vec<TextBlock>,
    ops: &mut Vec<RenderOp>,
    annotations: &mut Vec<PdfAnnotation>,
) {
    let baseline = page.flip(top + line.ascent);

    for piece in &line.pieces {
        if piece.text.trim().is_empty() {
            continue;
        }
        let x = page.left(piece.x);

        // The text block is emitted even for hidden runs: that is what lets
        // `sanitize` see them and what keeps extracted text complete.
        text_content.push(TextBlock {
            text: piece.text.clone(),
            x,
            y: baseline,
            width: piece.width,
            height: piece.size,
            font_size: piece.size,
            font_name: piece.font.postscript_name().to_string(),
            page_number,
        });

        if piece.hidden {
            continue;
        }

        ops.push(RenderOp::Text {
            text: piece.text.clone(),
            x,
            y: baseline,
            size: piece.size,
            width: piece.width,
            advances: piece.advances.clone(),
            // The advances are our own metrics, and they are the same ones the
            // line was broken with — so the renderer must use them rather than
            // re-measuring, or the paint would drift from the layout.
            // Typeset formats have no PDF character codes; the renderer
            // falls back to laying the run out with its own font.
            codes: Vec::new(),
            measured: false,
            rot: 0.0,
            color: piece.color,
            font: piece.font.postscript_name().to_string(),
            face: 0,
        });

        if piece.underline {
            let y = baseline - piece.size * 0.12;
            ops.push(RenderOp::Stroke {
                subpaths: vec![vec![[x, y], [x + piece.width, y]]],
                color: piece.color,
                width: piece.size * 0.05,
            });
        }
        if piece.strikethrough {
            let y = baseline + piece.size * 0.26;
            ops.push(RenderOp::Stroke {
                subpaths: vec![vec![[x, y], [x + piece.width, y]]],
                color: piece.color,
                width: piece.size * 0.05,
            });
        }

        if let Some(href) = &piece.link {
            annotations.push(PdfAnnotation {
                subtype: "Link".to_string(),
                page_number,
                rect: [
                    x,
                    baseline - piece.size * 0.25,
                    x + piece.width,
                    baseline + piece.size * 0.85,
                ],
                contents: None,
                uri: Some(href.clone()),
                dest: None,
                title: None,
                color: None,
            });
        }
    }
}

/// Emit a table's cell text and its rulings.
#[allow(clippy::too_many_arguments)]
fn emit_table(
    table: &TableBox,
    page: &Page,
    top: f64,
    page_number: usize,
    text_content: &mut Vec<TextBlock>,
    ops: &mut Vec<RenderOp>,
    h_lines: &mut Vec<Seg>,
    v_lines: &mut Vec<Seg>,
    annotations: &mut Vec<PdfAnnotation>,
) {
    let table_top = top;
    let table_bottom = top + table.height();
    let left = page.left(table.columns.first().copied().unwrap_or(0.0));
    let right = page.left(table.width());

    let mut row_top = table_top;
    for row in &table.rows {
        for cell in &row.cells {
            let cell_x = table.columns.get(cell.column).copied().unwrap_or(0.0) + CELL_PADDING;
            let mut line_top = row_top + CELL_PADDING;
            for line in &cell.lines {
                // Cell lines were flowed relative to the cell, so shift them
                // into the column before emitting.
                let shifted = Line {
                    pieces: line
                        .pieces
                        .iter()
                        .map(|piece| Piece {
                            x: piece.x + cell_x,
                            ..piece.clone()
                        })
                        .collect(),
                    ..line.clone()
                };
                emit_line(
                    &shifted,
                    page,
                    line_top,
                    page_number,
                    text_content,
                    ops,
                    annotations,
                );
                line_top += line.height;
            }
        }
        row_top += row.height;

        // A ruling under every row but the last; the table's own bottom edge
        // is drawn once, below.
        if row_top < table_bottom - 0.01 {
            push_h_line(page, row_top, left, right, ops, h_lines);
        }
    }

    // Outer edges.
    push_h_line(page, table_top, left, right, ops, h_lines);
    push_h_line(page, table_bottom, left, right, ops, h_lines);

    // Column rulings are drawn per row rather than the full height of the
    // table, so an edge interior to a merged cell is skipped instead of being
    // struck through the middle of it.
    let mut row_top = table_top;
    for row in &table.rows {
        let row_bottom = row_top + row.height;
        let y_top = page.flip(row_top);
        let y_bottom = page.flip(row_bottom);

        for (index, edge) in table.columns.iter().enumerate() {
            if spans_across(row, index) {
                continue;
            }
            let x = page.left(*edge);
            ops.push(RenderOp::Stroke {
                subpaths: vec![vec![[x, y_top], [x, y_bottom]]],
                color: [0.4, 0.4, 0.4, 1.0],
                width: RULE_WIDTH,
            });
            v_lines.push(Seg {
                x0: x,
                y0: y_bottom,
                x1: x,
                y1: y_top,
            });
        }
        row_top = row_bottom;
    }
}

/// Whether a merged cell in `row` crosses the column boundary at `edge`.
///
/// Edge 0 and the final edge are the table's own sides and are always drawn.
fn spans_across(row: &TableRowBox, edge: usize) -> bool {
    row.cells
        .iter()
        .any(|cell| cell.col_span > 1 && edge > cell.column && edge < cell.column + cell.col_span)
}

fn push_h_line(
    page: &Page,
    top: f64,
    left: f64,
    right: f64,
    ops: &mut Vec<RenderOp>,
    h_lines: &mut Vec<Seg>,
) {
    let y = page.flip(top);
    ops.push(RenderOp::Stroke {
        subpaths: vec![vec![[left, y], [right, y]]],
        color: [0.4, 0.4, 0.4, 1.0],
        width: RULE_WIDTH,
    });
    h_lines.push(Seg {
        x0: left,
        y0: y,
        x1: right,
        y1: y,
    });
}

/// Build the renderer's image record for a placed asset.
fn page_image(
    document: &SemanticDoc,
    asset_id: &str,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
) -> Option<PageImage> {
    let asset = document.asset(asset_id)?;
    if asset.bytes.is_empty() {
        return None;
    }
    // The bytes are handed over in their original encoding for the renderer to
    // decode; this crate does not decode image pixels outside the `ocr` feature.
    let format = match asset.media_type.as_str() {
        "image/jpeg" => "jpeg",
        "image/png" => "png",
        "image/gif" => "gif",
        _ => return None,
    };
    Some(PageImage {
        name: asset_id.to_string(),
        x,
        y,
        w,
        h,
        format: format.to_string(),
        width: asset.width,
        height: asset.height,
        data: crate::engine::b64e(&asset.bytes),
    })
}
