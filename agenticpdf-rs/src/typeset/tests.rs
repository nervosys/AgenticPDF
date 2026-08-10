// SPDX-License-Identifier: AGPL-3.0-or-later
//! Tests for the typesetter.
//!
//! These assert on *geometry*, because geometry is the whole point of the
//! module: that lines stay inside their margins, that a baseline sits below the
//! one above it, that a table's rulings land on its cell boundaries, and that
//! the display list a renderer receives describes the same page the text blocks
//! do.

use super::*;
use crate::doc::{
    Block, Cell, Inline, List, ListItem, Row, Section, SectionKind, SemanticDoc, Table, TextStyle,
    to_markdown,
};

fn doc_with(blocks: Vec<Block>) -> SemanticDoc {
    SemanticDoc {
        sections: vec![Section {
            blocks,
            ..Section::default()
        }],
        ..SemanticDoc::default()
    }
}

fn para(text: &str) -> Block {
    Block::paragraph(text)
}

/// All text blocks across all pages, in order.
fn all_text(output: &Typeset) -> Vec<&TextBlock> {
    output
        .pages
        .iter()
        .flat_map(|page| page.text_content.iter())
        .collect()
}

fn text_ops(list: &DisplayList) -> Vec<&RenderOp> {
    list.ops
        .iter()
        .filter(|op| matches!(op, RenderOp::Text { .. }))
        .collect()
}

// ============================================================================
// Page structure
// ============================================================================

#[test]
fn produces_one_page_per_output_model() {
    let output = typeset(&doc_with(vec![Block::heading(1, "Title"), para("Body.")]));
    // Every model describes the same number of pages, or a consumer of one will
    // index past the end of another.
    assert_eq!(output.pages.len(), 1);
    assert_eq!(output.graphics.len(), 1);
    assert_eq!(output.display.len(), 1);
    assert_eq!(output.pages[0].width, 612.0);
    assert_eq!(output.pages[0].height, 792.0);
    assert_eq!(output.display[0].page_number, 1);
    assert_eq!(output.pages[0].index, 0, "page index is 0-based");
}

#[test]
fn an_empty_document_still_has_a_page() {
    let output = typeset(&SemanticDoc::default());
    assert_eq!(output.pages.len(), 1);
    assert!(output.pages[0].text_content.is_empty());
}

#[test]
fn text_stays_inside_the_page_margins() {
    // Long prose is the case that catches an off-by-one in the content width.
    let prose = "The quick brown fox jumps over the lazy dog. ".repeat(40);
    let output = typeset(&doc_with(vec![para(&prose)]));

    for block in all_text(&output) {
        assert!(block.x >= 72.0 - 0.01, "left margin breached: x={}", block.x);
        assert!(
            block.x + block.width <= 612.0 - 72.0 + 0.5,
            "right margin breached: x={} width={}",
            block.x,
            block.width
        );
        assert!(block.y > 0.0 && block.y < 792.0, "off page: y={}", block.y);
    }
}

#[test]
fn baselines_descend_down_the_page() {
    let output = typeset(&doc_with(vec![
        para("First line of the document."),
        para("Second line of the document."),
        para("Third line of the document."),
    ]));
    let blocks = all_text(&output);
    assert_eq!(blocks.len(), 3);
    // PDF space is y-up, so each successive line has a *smaller* y.
    for pair in blocks.windows(2) {
        assert!(
            pair[1].y < pair[0].y,
            "line {:?} is not below {:?}",
            pair[1].text,
            pair[0].text
        );
    }
}

#[test]
fn long_documents_break_across_pages() {
    let blocks: Vec<Block> = (0..200).map(|i| para(&format!("Paragraph number {i}."))).collect();
    let output = typeset(&doc_with(blocks));
    assert!(output.pages.len() > 1, "expected multiple pages");
    assert_eq!(output.pages.len(), output.display.len());

    // Page indices are dense and 0-based, and every page holds content.
    for (index, page) in output.pages.iter().enumerate() {
        assert_eq!(page.index, index);
        assert!(!page.text_content.is_empty(), "page {index} is empty");
    }
    // Nothing overflows the bottom margin.
    for block in all_text(&output) {
        assert!(block.y >= 72.0 - BODY_SIZE, "below bottom margin: {}", block.y);
    }
}

#[test]
fn an_explicit_page_break_starts_a_new_page() {
    let output = typeset(&doc_with(vec![
        para("Before the break."),
        Block::PageBreak,
        para("After the break."),
    ]));
    assert_eq!(output.pages.len(), 2);
    assert!(output.pages[0].text_content[0].text.contains("Before"));
    assert!(output.pages[1].text_content[0].text.contains("After"));
}

#[test]
fn slides_are_one_page_each_at_their_own_size() {
    let slide = |title: &str| Section {
        kind: SectionKind::Slide,
        title: Some(title.into()),
        blocks: vec![para("a point")],
        page_size: Some(PageSize {
            width: 720.0,
            height: 540.0,
            margin_left: 36.0,
            margin_right: 36.0,
            margin_top: 36.0,
            margin_bottom: 36.0,
        }),
        ..Section::default()
    };
    let document = SemanticDoc {
        sections: vec![slide("One"), slide("Two")],
        ..SemanticDoc::default()
    };
    let output = typeset(&document);

    assert_eq!(output.pages.len(), 2);
    for page in &output.pages {
        assert_eq!((page.width, page.height), (720.0, 540.0));
    }
    // The slide title is drawn on the slide, not just held as metadata.
    assert!(page_text(&output, 0).contains("One"));
    assert!(page_text(&output, 1).contains("Two"));
}

#[test]
fn a_slide_never_spills_onto_a_second_page() {
    // An overfull slide is the author's problem; silently paginating it would
    // desynchronise slide numbers from page numbers.
    let many: Vec<Block> = (0..200).map(|i| para(&format!("point {i}"))).collect();
    let document = SemanticDoc {
        sections: vec![Section {
            kind: SectionKind::Slide,
            blocks: many,
            page_size: Some(PageSize::default()),
            ..Section::default()
        }],
        ..SemanticDoc::default()
    };
    assert_eq!(typeset(&document).pages.len(), 1);
}

fn page_text(output: &Typeset, index: usize) -> String {
    output.pages[index]
        .text_content
        .iter()
        .map(|block| block.text.as_str())
        .collect::<Vec<_>>()
        .join(" ")
}

// ============================================================================
// Line breaking
// ============================================================================

#[test]
fn wraps_prose_into_multiple_lines() {
    let prose = "word ".repeat(200);
    let output = typeset(&doc_with(vec![para(&prose)]));
    let blocks = all_text(&output);
    assert!(blocks.len() > 5, "expected wrapping, got {} lines", blocks.len());
}

#[test]
fn a_single_word_too_long_to_fit_still_gets_a_line() {
    // No break opportunity exists; the line must be emitted rather than lost.
    let output = typeset(&doc_with(vec![para(&"x".repeat(500))]));
    assert!(!all_text(&output).is_empty());
}

#[test]
fn breaks_cjk_between_characters() {
    // CJK has no spaces, so a reader that only breaks at whitespace produces
    // one enormous line running off the page.
    let cjk = "\u{4E2D}\u{6587}".repeat(200);
    let output = typeset(&doc_with(vec![para(&cjk)]));
    let blocks = all_text(&output);
    assert!(blocks.len() > 5, "CJK did not wrap: {} lines", blocks.len());
    for block in blocks {
        assert!(
            block.x + block.width <= 612.0 - 72.0 + 0.5,
            "CJK line overflows the margin"
        );
    }
}

#[test]
fn a_hard_break_ends_the_line_without_ending_the_paragraph() {
    let content = vec![
        Inline::Run(Run::plain("first")),
        Inline::Break,
        Inline::Run(Run::plain("second")),
    ];
    let output = typeset(&doc_with(vec![Block::Paragraph {
        content,
        align: Align::Left,
        indent: 0.0,
    }]));
    let blocks = all_text(&output);
    assert_eq!(blocks.len(), 2);
    assert_eq!(blocks[0].text.trim(), "first");
    assert_eq!(blocks[1].text.trim(), "second");
}

#[test]
fn wrapped_lines_do_not_begin_with_a_space() {
    let output = typeset(&doc_with(vec![para(&"alpha beta gamma ".repeat(40))]));
    for block in all_text(&output) {
        assert!(
            !block.text.starts_with(' '),
            "line starts with a space: {:?}",
            block.text
        );
    }
}

// ============================================================================
// Alignment
// ============================================================================

#[test]
fn honours_paragraph_alignment() {
    let make = |align: Align| {
        typeset(&doc_with(vec![Block::Paragraph {
            content: vec![Inline::Run(Run::plain("short line"))],
            align,
            indent: 0.0,
        }]))
    };

    let left = make(Align::Left).pages[0].text_content[0].x;
    let centre = make(Align::Center).pages[0].text_content[0].x;
    let right = make(Align::Right).pages[0].text_content[0].x;

    assert!(left < centre && centre < right, "{left} {centre} {right}");
    assert!((left - 72.0).abs() < 0.01, "left-aligned text starts at the margin");
    // A right-aligned line ends at the right margin.
    let block = &make(Align::Right).pages[0].text_content[0];
    assert!((block.x + block.width - (612.0 - 72.0)).abs() < 0.5);
}

#[test]
fn justification_stretches_all_but_the_last_line() {
    let output = typeset(&doc_with(vec![Block::Paragraph {
        content: vec![Inline::Run(Run::plain("alpha beta gamma delta ".repeat(20)))],
        align: Align::Justify,
        indent: 0.0,
    }]));
    let blocks = all_text(&output);
    assert!(blocks.len() > 2);

    let right_edge = 612.0 - 72.0;
    for block in &blocks[..blocks.len() - 1] {
        assert!(
            (block.x + block.width - right_edge).abs() < 1.0,
            "justified line does not reach the right margin: {}",
            block.x + block.width
        );
    }
    // The last line keeps its natural width.
    let last = blocks.last().unwrap();
    assert!(last.x + last.width < right_edge - 1.0);
}

// ============================================================================
// Lists, quotes, headings
// ============================================================================

#[test]
fn list_markers_hang_to_the_left_of_their_text() {
    let output = typeset(&doc_with(vec![Block::List(List {
        ordered: false,
        start: 1,
        items: vec![ListItem {
            blocks: vec![para("an item")],
            checked: None,
        }],
    })]));
    let blocks = all_text(&output);
    let marker = blocks.iter().find(|b| b.text.contains('\u{2022}')).expect("bullet");
    let text = blocks.iter().find(|b| b.text.contains("item")).expect("text");
    assert!(marker.x < text.x, "marker is not in the margin");
    // Both sit on the same baseline.
    assert!((marker.y - text.y).abs() < 0.01);
}

#[test]
fn ordered_lists_number_from_their_start() {
    let output = typeset(&doc_with(vec![Block::List(List {
        ordered: true,
        start: 5,
        items: vec![
            ListItem {
                blocks: vec![para("five")],
                checked: None,
            },
            ListItem {
                blocks: vec![para("six")],
                checked: None,
            },
        ],
    })]));
    let text = page_text(&output, 0);
    assert!(text.contains("5."), "{text}");
    assert!(text.contains("6."), "{text}");
}

#[test]
fn nested_lists_indent_further_than_their_parent() {
    let inner = Block::List(List {
        ordered: false,
        start: 1,
        items: vec![ListItem {
            blocks: vec![para("child")],
            checked: None,
        }],
    });
    let output = typeset(&doc_with(vec![Block::List(List {
        ordered: false,
        start: 1,
        items: vec![ListItem {
            blocks: vec![para("parent"), inner],
            checked: None,
        }],
    })]));

    let blocks = all_text(&output);
    let parent = blocks.iter().find(|b| b.text.contains("parent")).expect("parent");
    let child = blocks.iter().find(|b| b.text.contains("child")).expect("child");
    assert!(child.x > parent.x, "nested item is not indented");
}

#[test]
fn quotes_indent_their_content() {
    let plain = typeset(&doc_with(vec![para("text")])).pages[0].text_content[0].x;
    let quoted = typeset(&doc_with(vec![Block::Quote(vec![para("text")])])).pages[0].text_content[0].x;
    assert!(quoted > plain, "quote is not indented");
}

#[test]
fn headings_are_larger_than_body_text_and_ordered_by_level() {
    let output = typeset(&doc_with(vec![
        Block::heading(1, "H1"),
        Block::heading(2, "H2"),
        Block::heading(3, "H3"),
        para("body"),
    ]));
    let blocks = all_text(&output);
    let size = |needle: &str| {
        blocks
            .iter()
            .find(|b| b.text.contains(needle))
            .expect(needle)
            .font_size
    };
    assert!(size("H1") > size("H2"));
    assert!(size("H2") > size("H3"));
    assert!(size("H3") > size("body"));
}

#[test]
fn a_runs_own_size_overrides_the_paragraph_default() {
    let content = vec![Inline::Run(Run::styled(
        "big",
        TextStyle {
            size: Some(30.0),
            ..TextStyle::default()
        },
    ))];
    let output = typeset(&doc_with(vec![Block::Paragraph {
        content,
        align: Align::Left,
        indent: 0.0,
    }]));
    assert_eq!(output.pages[0].text_content[0].font_size, 30.0);
}

// ============================================================================
// Fonts and styles in the output
// ============================================================================

#[test]
fn styled_runs_carry_the_matching_postscript_font() {
    let content = vec![
        Inline::Run(Run::plain("plain ")),
        Inline::Run(Run::styled(
            "bold",
            TextStyle {
                bold: true,
                ..TextStyle::default()
            },
        )),
    ];
    let output = typeset(&doc_with(vec![Block::Paragraph {
        content,
        align: Align::Left,
        indent: 0.0,
    }]));
    let fonts: Vec<&str> = output.pages[0]
        .text_content
        .iter()
        .map(|b| b.font_name.as_str())
        .collect();
    assert!(fonts.contains(&"Helvetica"), "{fonts:?}");
    assert!(fonts.contains(&"Helvetica-Bold"), "{fonts:?}");
}

#[test]
fn code_blocks_are_monospaced_and_unwrapped() {
    let output = typeset(&doc_with(vec![Block::Code {
        language: None,
        text: "let a = 1;\nlet b = 2;".into(),
    }]));
    let blocks = all_text(&output);
    assert_eq!(blocks.len(), 2, "one line per source line");
    assert!(blocks.iter().all(|b| b.font_name.starts_with("Courier")));
}

#[test]
fn display_list_advances_sum_to_the_reported_width() {
    // The renderer positions glyphs by these advances; if they disagreed with
    // the width, the layout and the paint would drift apart.
    let output = typeset(&doc_with(vec![para("Advance widths must agree.")]));
    for op in text_ops(&output.display[0]) {
        let RenderOp::Text {
            text,
            width,
            advances,
            measured,
            ..
        } = op
        else {
            unreachable!()
        };
        assert!(!measured, "our own metrics must be authoritative");
        assert_eq!(advances.len(), text.chars().count());
        let summed: f64 = advances.iter().sum();
        assert!((summed - width).abs() < 0.01, "{summed} vs {width}");
    }
}

#[test]
fn every_text_block_has_a_matching_draw_operation() {
    let output = typeset(&doc_with(vec![
        Block::heading(1, "Title"),
        para("Some body text here."),
    ]));
    let drawn: Vec<String> = text_ops(&output.display[0])
        .iter()
        .map(|op| match op {
            RenderOp::Text { text, .. } => text.clone(),
            _ => unreachable!(),
        })
        .collect();
    for block in &output.pages[0].text_content {
        assert!(
            drawn.contains(&block.text),
            "text block {:?} is not drawn",
            block.text
        );
    }
}

#[test]
fn underline_and_strikethrough_are_drawn_as_strokes() {
    let content = vec![Inline::Run(Run::styled(
        "marked",
        TextStyle {
            underline: true,
            strikethrough: true,
            ..TextStyle::default()
        },
    ))];
    let output = typeset(&doc_with(vec![Block::Paragraph {
        content,
        align: Align::Left,
        indent: 0.0,
    }]));
    let strokes = output.display[0]
        .ops
        .iter()
        .filter(|op| matches!(op, RenderOp::Stroke { .. }))
        .count();
    assert_eq!(strokes, 2, "expected one underline and one strikethrough");
}

// ============================================================================
// Hidden text
// ============================================================================

#[test]
fn hidden_text_is_laid_out_but_not_painted() {
    let content = vec![
        Inline::Run(Run::plain("visible ")),
        Inline::Run(Run::styled(
            "concealed",
            TextStyle {
                hidden: true,
                ..TextStyle::default()
            },
        )),
    ];
    let output = typeset(&doc_with(vec![Block::Paragraph {
        content,
        align: Align::Left,
        indent: 0.0,
    }]));

    // The text block exists, so extraction and the injection scan can see it.
    let blocks = all_text(&output);
    assert!(blocks.iter().any(|b| b.text.contains("concealed")));

    // The draw list does not, so a render shows what a human would see.
    let drawn: String = text_ops(&output.display[0])
        .iter()
        .map(|op| match op {
            RenderOp::Text { text, .. } => text.clone(),
            _ => unreachable!(),
        })
        .collect();
    assert!(drawn.contains("visible"));
    assert!(!drawn.contains("concealed"), "hidden text was painted");
}

// ============================================================================
// Tables
// ============================================================================

fn sample_table() -> Table {
    Table {
        header_rows: 1,
        rows: vec![
            Row {
                cells: vec![Cell::text("Region"), Cell::text("Growth")],
            },
            Row {
                cells: vec![Cell::text("EMEA"), Cell::text("8%")],
            },
            Row {
                cells: vec![Cell::text("APAC"), Cell::text("17%")],
            },
        ],
        ..Table::default()
    }
}

#[test]
fn tables_place_cells_in_columns_and_rows() {
    let output = typeset(&doc_with(vec![Block::Table(sample_table())]));
    let blocks = all_text(&output);
    let find = |needle: &str| blocks.iter().find(|b| b.text.contains(needle)).expect(needle);

    // Same column: same x. Same row: same baseline.
    assert!((find("Region").x - find("EMEA").x).abs() < 0.01);
    assert!((find("Growth").x - find("8%").x).abs() < 0.01);
    assert!((find("Region").y - find("Growth").y).abs() < 0.01);
    // Second column is to the right of the first, and rows descend.
    assert!(find("Growth").x > find("Region").x);
    assert!(find("EMEA").y < find("Region").y);
}

#[test]
fn table_rulings_are_emitted_as_page_graphics() {
    let output = typeset(&doc_with(vec![Block::Table(sample_table())]));
    let graphics = &output.graphics[0];
    // Three rows means four horizontal edges; two columns means three vertical
    // edges on each of three rows.
    assert_eq!(graphics.h_lines.len(), 4, "row rulings");
    assert_eq!(graphics.v_lines.len(), 9, "column rulings per row");
}

#[test]
fn drawn_tables_are_recovered_by_the_pdf_table_reconstructor() {
    // The strongest check available on the table geometry: feed the rulings and
    // text this module produced into the detector written for PDFs, and require
    // it to find the same table back.
    let output = typeset(&doc_with(vec![Block::Table(sample_table())]));
    let recovered = crate::tables::detect_tables(&output.graphics, &output.pages);

    assert_eq!(recovered.len(), 1, "table not recovered from its own rulings");
    assert_eq!((recovered[0].rows, recovered[0].cols), (3, 2));
    assert_eq!(recovered[0].cells[0], vec!["Region", "Growth"]);
    assert_eq!(recovered[0].cells[2], vec!["APAC", "17%"]);
}

#[test]
fn column_widths_follow_the_sources_proportions() {
    let table = Table {
        header_rows: 0,
        rows: vec![Row {
            cells: vec![Cell::text("narrow"), Cell::text("wide")],
        }],
        // A 1:3 split, declared in whatever units the source used.
        column_widths: vec![100.0, 300.0],
        ..Table::default()
    };
    let output = typeset(&doc_with(vec![Block::Table(table)]));
    let blocks = all_text(&output);
    let first = blocks.iter().find(|b| b.text.contains("narrow")).unwrap();
    let second = blocks.iter().find(|b| b.text.contains("wide")).unwrap();

    // The second column starts a quarter of the way across the content box.
    let content_width = 612.0 - 144.0;
    let offset = second.x - first.x;
    assert!(
        (offset - content_width * 0.25).abs() < 1.0,
        "columns not proportional: offset {offset}"
    );
}

#[test]
fn a_merged_cell_is_not_struck_through_by_a_column_ruling() {
    let table = Table {
        header_rows: 0,
        rows: vec![
            Row {
                cells: vec![Cell {
                    blocks: vec![para("spans both")],
                    col_span: 2,
                    row_span: 1,
                }],
            },
            Row {
                cells: vec![Cell::text("a"), Cell::text("b")],
            },
        ],
        ..Table::default()
    };
    let output = typeset(&doc_with(vec![Block::Table(table)]));
    // Row one has two edges (its own sides); row two has three.
    assert_eq!(output.graphics[0].v_lines.len(), 5);
}

#[test]
fn a_long_table_splits_across_pages_and_repeats_its_header() {
    let mut rows = vec![Row {
        cells: vec![Cell::text("Header A"), Cell::text("Header B")],
    }];
    for i in 0..120 {
        rows.push(Row {
            cells: vec![Cell::text(format!("row {i}")), Cell::text("value")],
        });
    }
    let output = typeset(&doc_with(vec![Block::Table(Table {
        header_rows: 1,
        rows,
        ..Table::default()
    })]));

    assert!(output.pages.len() > 1, "long table did not split");
    for (index, page) in output.pages.iter().enumerate() {
        let text: String = page
            .text_content
            .iter()
            .map(|b| b.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(
            text.contains("Header A"),
            "page {index} is missing the repeated header"
        );
    }
}

#[test]
fn cell_content_wraps_within_its_column() {
    let table = Table {
        header_rows: 0,
        rows: vec![Row {
            cells: vec![
                Cell::text("long cell content ".repeat(20)),
                Cell::text("short"),
            ],
        }],
        ..Table::default()
    };
    let output = typeset(&doc_with(vec![Block::Table(table)]));
    let blocks = all_text(&output);

    // The wrapped cell produced several lines, all inside its own column.
    let column_edge = 72.0 + (612.0 - 144.0) / 2.0;
    let wrapped: Vec<_> = blocks.iter().filter(|b| b.text.contains("long")).collect();
    assert!(wrapped.len() > 1, "cell content did not wrap");
    for block in wrapped {
        assert!(
            block.x + block.width <= column_edge + 0.5,
            "cell text escaped its column"
        );
    }
}

// ============================================================================
// Images and links
// ============================================================================

#[test]
fn images_are_placed_and_handed_to_the_renderer() {
    let mut document = SemanticDoc::new();
    let image = document.add_asset("image/png", crate::testing::png_header(400, 200));
    document.push(Block::Figure {
        image,
        caption: Some("A chart".into()),
    });

    let output = typeset(&document);
    let placements: Vec<&RenderOp> = output.display[0]
        .ops
        .iter()
        .filter(|op| matches!(op, RenderOp::Image { .. }))
        .collect();
    assert_eq!(placements.len(), 1);

    let RenderOp::Image { name, w, h, .. } = placements[0] else {
        unreachable!()
    };
    // 400x200 pixels at 96 dpi is 300x150 points, and the aspect ratio holds.
    assert!((w / h - 2.0).abs() < 0.01, "aspect ratio lost: {w}x{h}");

    // The renderer is given the bytes under the same key the draw op uses, on
    // the page the placement is on.
    assert_eq!(output.images.len(), output.pages.len());
    assert_eq!(output.images[0].len(), 1);
    assert_eq!(&output.images[0][0].name, name);
    assert_eq!(output.images[0][0].format, "png");
    assert!(!output.images[0][0].data.is_empty());

    // The caption is typeset below the image.
    assert!(page_text(&output, 0).contains("A chart"));
}

#[test]
fn oversized_images_are_scaled_to_the_content_width() {
    let mut document = SemanticDoc::new();
    let mut image = document.add_asset("image/png", crate::testing::png_header(4000, 2000));
    image.width = Some(4000.0);
    image.height = Some(2000.0);
    document.push(Block::Figure {
        image,
        caption: None,
    });

    let output = typeset(&document);
    let RenderOp::Image { w, h, .. } = output.display[0]
        .ops
        .iter()
        .find(|op| matches!(op, RenderOp::Image { .. }))
        .expect("image")
    else {
        unreachable!()
    };
    assert!((*w - (612.0 - 144.0)).abs() < 0.01, "not scaled to fit: {w}");
    assert!((w / h - 2.0).abs() < 0.01, "aspect ratio lost when scaling");
}

#[test]
fn links_become_annotations_over_their_text() {
    let content = vec![
        Inline::Run(Run::plain("see ")),
        Inline::Link {
            href: "https://example.com".into(),
            runs: vec![Run::plain("the report")],
        },
    ];
    let output = typeset(&doc_with(vec![Block::Paragraph {
        content,
        align: Align::Left,
        indent: 0.0,
    }]));

    assert_eq!(output.annotations.len(), 1);
    let link = &output.annotations[0];
    assert_eq!(link.uri.as_deref(), Some("https://example.com"));
    assert_eq!(link.page_number, 1);
    let [x0, y0, x1, y1] = link.rect;
    assert!(x1 > x0 && y1 > y0, "degenerate link rectangle");
    // The rectangle sits over the link text, not at the line start.
    assert!(x0 > 72.0, "link rect is at the margin, not over its text");
}

// ============================================================================
// Round trip against the inference pipeline
// ============================================================================

#[test]
fn typeset_geometry_reads_back_as_the_structure_it_came_from() {
    // The end-to-end invariant for the module: lay out authored structure, then
    // hand the resulting coordinates to the analyser written for PDFs. It must
    // infer the same document back.
    let document = doc_with(vec![
        Block::heading(1, "Quarterly Report"),
        para("Revenue grew across every region this quarter."),
        Block::heading(2, "Highlights"),
        Block::List(List {
            ordered: false,
            start: 1,
            items: vec![
                ListItem {
                    blocks: vec![para("Hiring on plan")],
                    checked: None,
                },
                ListItem {
                    blocks: vec![para("Churn down")],
                    checked: None,
                },
            ],
        }),
    ]);

    let output = typeset(&document);
    let geometric = crate::PdfDocument::from_parts(
        String::new(),
        output.pages.clone(),
        crate::PdfMetadata::default(),
        Vec::new(),
        Vec::new(),
    );
    let inferred = crate::layout::to_markdown(&geometric);

    assert!(inferred.contains("Quarterly Report"), "{inferred}");
    assert!(inferred.contains("Highlights"), "{inferred}");
    assert!(inferred.contains("Hiring on plan"), "{inferred}");
    assert!(
        inferred.contains("# Quarterly Report"),
        "heading not inferred from its size:\n{inferred}"
    );
    assert!(
        inferred.contains("- Hiring on plan"),
        "list not inferred from its bullet:\n{inferred}"
    );
}

#[test]
fn text_survives_the_round_trip_through_geometry() {
    // Whatever the semantic model says the words are, the laid-out page must
    // contain the same ones in the same order.
    let document = doc_with(vec![
        Block::heading(2, "Section"),
        para("First paragraph of the section."),
        para("Second paragraph, a little longer than the first one."),
    ]);
    let output = typeset(&document);

    let from_model: Vec<String> = to_markdown(&document)
        .split_whitespace()
        .map(|word| word.trim_matches(|c: char| !c.is_alphanumeric()).to_string())
        .filter(|word| !word.is_empty())
        .collect();
    let from_geometry: Vec<String> = all_text(&output)
        .iter()
        .flat_map(|block| block.text.split_whitespace())
        .map(|word| word.trim_matches(|c: char| !c.is_alphanumeric()).to_string())
        .filter(|word| !word.is_empty())
        .collect();

    assert_eq!(from_model, from_geometry);
}
