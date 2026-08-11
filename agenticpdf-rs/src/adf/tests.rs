// SPDX-License-Identifier: AGPL-3.0-or-later
//! Tests for the Agentic Document Format.

use crate::doc::{
    Align, Asset, Block, Cell, Footnote, ImageRef, Inline, List, ListItem, Row, Section,
    SectionKind, SemanticDoc, Table, TextStyle,
};

use super::oplog::{Actor, Change, OpId, OpLog};
use super::provenance::{Provenance, Verification};
use super::wire::hash64;
use super::{AdfDoc, AdfError, AdfWriter, ChunkKind, Header};

/// A document exercising every block and inline variant, so a round-trip test
/// is a statement about the whole model rather than about paragraphs.
fn rich_document() -> SemanticDoc {
    SemanticDoc {
        title: Some("Quarterly Report".into()),
        author: Some("NERVOSYS".into()),
        subject: None,
        creator: Some("agenticpdf".into()),
        created: Some("2026-08-10".into()),
        modified: None,
        sections: vec![Section {
            kind: SectionKind::Flow,
            title: Some("Body".into()),
            page_size: Some(crate::doc::PageSize::default()),
            blocks: vec![
                Block::Heading {
                    level: 1,
                    content: vec![Inline::Run(crate::doc::Run::plain("Quarterly Report"))],
                },
                Block::Paragraph {
                    content: vec![
                        Inline::Run(crate::doc::Run::styled(
                            "Revenue",
                            TextStyle {
                                bold: true,
                                size: Some(12.5),
                                color: Some([0.1, 0.2, 0.3]),
                                font: Some("Helvetica".into()),
                                ..TextStyle::default()
                            },
                        )),
                        Inline::Break,
                        Inline::Link {
                            href: "https://example.invalid".into(),
                            runs: vec![crate::doc::Run::plain("details")],
                        },
                        Inline::FootnoteRef { index: 0 },
                        Inline::Image(ImageRef {
                            asset_id: "img1".into(),
                            alt: Some("chart".into()),
                            width: Some(100.0),
                            height: None,
                        }),
                    ],
                    align: Align::Justify,
                    indent: 18.0,
                },
                Block::List(List {
                    ordered: true,
                    start: 3,
                    items: vec![ListItem {
                        checked: Some(true),
                        blocks: vec![Block::paragraph("done")],
                    }],
                }),
                Block::Table(Table {
                    caption: Some("Growth".into()),
                    header_rows: 1,
                    column_widths: vec![1.0, 2.0],
                    rows: vec![Row {
                        cells: vec![
                            Cell::text("EMEA"),
                            Cell {
                                blocks: vec![Block::paragraph("8%")],
                                col_span: 2,
                                row_span: 1,
                            },
                        ],
                    }],
                }),
                Block::Quote(vec![Block::paragraph("Best quarter on record.")]),
                Block::Code {
                    language: Some("rust".into()),
                    text: "fn main() {}".into(),
                },
                Block::Figure {
                    image: ImageRef {
                        asset_id: "img1".into(),
                        alt: None,
                        width: None,
                        height: None,
                    },
                    caption: Some("Figure 1".into()),
                },
                Block::Divider,
                Block::PageBreak,
            ],
            notes: vec![Block::paragraph("speaker note")],
        }],
        footnotes: vec![Footnote {
            label: Some("1".into()),
            blocks: vec![Block::paragraph("A footnote.")],
        }],
        assets: vec![Asset {
            id: "img1".into(),
            media_type: "image/png".into(),
            bytes: crate::testing::png_header(4, 4).to_vec(),
            width: 4,
            height: 4,
        }],
    }
}

#[test]
fn every_part_of_the_model_survives_a_round_trip() {
    let original = rich_document();
    let bytes = AdfWriter::new().write(&original, "docx");
    let document = AdfDoc::open(&bytes).unwrap();
    let restored = document.to_semantic().unwrap();

    // Comparing the Markdown rendering compares the whole tree in one assert,
    // and fails with a readable diff rather than a wall of struct debug.
    assert_eq!(
        crate::doc::to_markdown(&restored),
        crate::doc::to_markdown(&original)
    );
    assert_eq!(restored.title, original.title);
    assert_eq!(restored.author, original.author);
    assert_eq!(restored.footnotes.len(), 1);
    assert_eq!(restored.assets.len(), 1);
    assert_eq!(restored.assets[0].bytes, original.assets[0].bytes);
    assert_eq!(restored.sections[0].notes.len(), 1);
    assert_eq!(document.meta().unwrap().source_format, "docx");
}

#[test]
fn opening_reads_only_the_header_and_table() {
    let bytes = AdfWriter::new().write(&rich_document(), "adf");
    let document = AdfDoc::open(&bytes).unwrap();

    // The claim under test is structural: section count is answerable without
    // decoding a single section.
    assert_eq!(document.section_count(), 1);
    assert!(document.chunks().len() >= 4);
    assert!(
        document
            .chunks()
            .iter()
            .any(|c| c.kind == ChunkKind::Strings)
    );
    assert!(document.chunks().iter().any(|c| c.kind == ChunkKind::Meta));
}

#[test]
fn a_section_is_reachable_without_decoding_its_neighbours() {
    let mut document = SemanticDoc::default();
    for index in 0..50 {
        document.sections.push(Section {
            kind: SectionKind::Slide,
            title: Some(format!("Slide {index}")),
            blocks: vec![Block::paragraph(format!("content {index}"))],
            ..Section::default()
        });
    }
    let bytes = AdfWriter::new().write(&document, "pptx");
    let adf = AdfDoc::open(&bytes).unwrap();

    assert_eq!(adf.section_count(), 50);
    let section = adf.section(37).unwrap();
    assert_eq!(section.title.as_deref(), Some("Slide 37"));
    assert!(
        crate::doc::to_markdown(&SemanticDoc {
            sections: vec![section],
            ..SemanticDoc::default()
        })
        .contains("content 37")
    );
}

#[test]
fn strings_are_interned_once_however_often_they_repeat() {
    // Measured against the same document with distinct text rather than
    // against an absolute size: the block stream costs the same either way, so
    // the difference between the two files is exactly what interning saved.
    let sentence = "the same sentence over and over again, at some length";
    let build = |text: &dyn Fn(usize) -> String| {
        let mut document = SemanticDoc::default();
        document.sections.push(Section {
            blocks: (0..1000).map(|i| Block::paragraph(text(i))).collect(),
            ..Section::default()
        });
        AdfWriter::new().uncompressed().write(&document, "md")
    };

    let repeated = build(&|_| sentence.to_string());
    let distinct = build(&|i| format!("{sentence} {i:04}"));

    assert!(
        repeated.len() + sentence.len() * 900 < distinct.len(),
        "interning saved almost nothing: {} vs {} bytes",
        repeated.len(),
        distinct.len()
    );
    let restored = AdfDoc::open(&repeated).unwrap().to_semantic().unwrap();
    assert_eq!(restored.sections[0].blocks.len(), 1000);
}

// ============================================================================
// Robustness
// ============================================================================

#[test]
fn a_non_adf_file_is_rejected_by_its_magic() {
    assert!(!AdfDoc::sniff(b"%PDF-1.7"));
    assert_eq!(
        AdfDoc::open(b"%PDF-1.7 and then some").unwrap_err(),
        AdfError::NotAdf
    );
}

#[test]
fn an_empty_or_tiny_file_fails_without_panicking() {
    for len in 0..super::HEADER_LEN {
        let bytes = vec![0u8; len];
        assert!(AdfDoc::open(&bytes).is_err(), "len {len} should not open");
    }
}

#[test]
fn a_corrupted_header_is_caught_by_its_checksum() {
    let mut bytes = AdfWriter::new().write(&rich_document(), "adf");
    // Corrupt the chunk-table offset. Without the checksum this is read as a
    // valid-looking offset and the table is parsed out of arbitrary bytes.
    bytes[16] ^= 0xFF;
    assert!(matches!(AdfDoc::open(&bytes), Err(AdfError::Malformed(_))));
}

#[test]
fn truncation_at_every_length_is_an_error_not_a_panic() {
    let bytes = AdfWriter::new().write(&rich_document(), "adf");
    // Step rather than every byte: the file is tens of kilobytes and the
    // failure modes cluster at structure boundaries.
    for len in (0..bytes.len()).step_by(7) {
        let _ = AdfDoc::open(&bytes[..len]).map(|d| d.to_semantic());
    }
}

#[test]
fn every_single_byte_mutation_is_survivable() {
    let original = AdfWriter::new().write(&rich_document(), "adf");

    for index in (0..original.len()).step_by(11) {
        let mut bytes = original.clone();
        bytes[index] ^= 0xFF;
        // The only requirement is that it does not panic or hang: a mutated
        // file may legitimately still parse, or fail — both are fine.
        if let Ok(document) = AdfDoc::open(&bytes) {
            let _ = document.to_semantic();
            let _ = document.search("revenue");
            let _ = document.oplog();
        }
    }
}

#[test]
fn a_chunk_pointing_outside_the_file_is_rejected_at_open() {
    let mut bytes = AdfWriter::new().write(&rich_document(), "adf");
    let header = Header::read(&bytes).unwrap();

    // Push the first entry's offset past the end of the file.
    let at = header.chunk_table_offset as usize + 8;
    bytes[at..at + 8].copy_from_slice(&u64::MAX.to_le_bytes());
    assert!(AdfDoc::open(&bytes).is_err());
}

#[test]
fn deeply_nested_blocks_are_rejected_rather_than_overflowing_the_stack() {
    // Build nesting past the decoder's limit by hand: quotes wrapping quotes.
    let mut block = Block::paragraph("deep");
    for _ in 0..500 {
        block = Block::Quote(vec![block]);
    }
    let encoded = super::codec::encode_block_standalone(&block);

    // Encoding is iterative on the way in only because the value already
    // exists; decoding is what recurses, and it must refuse rather than abort.
    match super::codec::decode_block_standalone(&encoded) {
        Err(AdfError::Malformed(_)) => {}
        other => panic!("expected a depth error, got {other:?}"),
    }
}

// ============================================================================
// Provenance
// ============================================================================

#[test]
fn provenance_distinguishes_a_match_from_drift_from_no_record() {
    let mut writer = AdfWriter::new();
    let source = writer.intern_source("report.pdf");
    writer.add_provenance(Provenance {
        section: 0,
        block: 0,
        source,
        page: 4,
        bbox: [72.0, 700.0, 540.0, 720.0],
        hash: hash64(b"Revenue grew 12%"),
    });

    let mut document = SemanticDoc::default();
    document.sections.push(Section {
        blocks: vec![Block::paragraph("Revenue grew 12%")],
        ..Section::default()
    });

    let bytes = writer.write(&document, "pdf");
    let adf = AdfDoc::open(&bytes).unwrap();
    let table = adf.provenance();

    assert_eq!(table.len(), 1);
    assert!(table.verify(0, 0, "Revenue grew 12%").is_match());
    assert!(matches!(
        table.verify(0, 0, "Revenue grew 120%"),
        Verification::Drifted(_)
    ));
    assert!(matches!(
        table.verify(9, 9, "anything"),
        Verification::Unrecorded
    ));

    let row = table.get(0).unwrap();
    assert_eq!(row.page, 4);
    assert!(row.has_geometry());
    assert!(!row.is_authored());
}

#[test]
fn authored_content_reports_no_source_and_no_geometry() {
    let row = Provenance {
        section: 0,
        block: 0,
        source: Provenance::AUTHORED,
        page: 0,
        bbox: [0.0; 4],
        hash: 0,
    };
    assert!(row.is_authored());
    assert!(
        !row.has_geometry(),
        "a zero box must not read as a location"
    );
}

// ============================================================================
// Retrieval
// ============================================================================

#[test]
fn a_document_is_searchable_the_moment_it_is_opened() {
    let mut document = SemanticDoc::default();
    document.sections.push(Section {
        blocks: vec![
            Block::paragraph("Revenue grew by 12% across EMEA"),
            Block::paragraph("Hiring is on plan"),
            Block::paragraph("Churn is down in APAC"),
        ],
        ..Section::default()
    });

    let bytes = AdfWriter::new().write(&document, "md");
    let adf = AdfDoc::open(&bytes).unwrap();

    let hits = adf.search("revenue emea").unwrap();
    assert_eq!(hits.len(), 1);
    assert!(hits[0].1.contains("EMEA"));
    assert_eq!(
        hits[0].0.blocks,
        [0, 0],
        "the hit names the block it came from"
    );

    assert!(adf.search("revenue hiring").unwrap().is_empty());
    assert_eq!(adf.search("plan").unwrap().len(), 1);
}

#[test]
fn embeddings_ride_along_and_rank_semantically() {
    let mut document = SemanticDoc::default();
    document.sections.push(Section {
        blocks: vec![
            Block::paragraph("first block"),
            Block::paragraph("second block"),
        ],
        ..Section::default()
    });

    let bytes = AdfWriter::new()
        .with_embeddings(3, vec![vec![1.0, 0.0, 0.0], vec![0.0, 1.0, 0.0]])
        .write(&document, "md");
    let adf = AdfDoc::open(&bytes).unwrap();

    let hits = adf.search_similar(&[0.0, 1.0, 0.0], 1).unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].1, "second block");
    assert!(hits[0].2 > 0.99);
}

#[test]
fn a_document_without_an_index_searches_to_nothing_rather_than_failing() {
    let bytes = AdfWriter::new().write(&SemanticDoc::default(), "adf");
    let adf = AdfDoc::open(&bytes).unwrap();
    assert!(adf.search("anything").unwrap().is_empty());
    assert!(adf.search_similar(&[1.0], 5).unwrap().is_empty());
}

// ============================================================================
// Edit log
// ============================================================================

fn person() -> Actor {
    Actor {
        id: 1,
        name: "Adam".into(),
        is_agent: false,
    }
}

fn agent() -> Actor {
    Actor {
        id: 2,
        name: "claude-opus-5".into(),
        is_agent: true,
    }
}

#[test]
fn the_log_replays_into_the_document_it_describes() {
    let mut log = OpLog::new();
    log.register_actor(person());

    let first = log.push(
        1,
        100,
        Change::Insert {
            parent: OpId::ROOT,
            left: None,
            block: Block::paragraph("one"),
        },
    );
    log.push(
        1,
        101,
        Change::Insert {
            parent: OpId::ROOT,
            left: Some(first),
            block: Block::paragraph("two"),
        },
    );

    let blocks = log.materialize(OpId::ROOT);
    assert_eq!(blocks.len(), 2);
    assert_eq!(block_text(&blocks[0]), "one");
    assert_eq!(block_text(&blocks[1]), "two");
}

#[test]
fn concurrent_edits_converge_whatever_order_they_arrive_in() {
    // Two replicas start from the same document and edit independently.
    let mut base = OpLog::new();
    base.register_actor(person());
    base.register_actor(agent());
    let anchor = base.push(
        1,
        100,
        Change::Insert {
            parent: OpId::ROOT,
            left: None,
            block: Block::paragraph("shared"),
        },
    );

    let mut human = base.clone();
    human.push(
        1,
        200,
        Change::Insert {
            parent: OpId::ROOT,
            left: Some(anchor),
            block: Block::paragraph("from the human"),
        },
    );

    let mut model = base.clone();
    model.push(
        2,
        201,
        Change::Insert {
            parent: OpId::ROOT,
            left: Some(anchor),
            block: Block::paragraph("from the agent"),
        },
    );

    // Merge each into the other, in opposite orders.
    let mut a = human.clone();
    a.merge(&model);
    let mut b = model.clone();
    b.merge(&human);

    let left: Vec<String> = a.materialize(OpId::ROOT).iter().map(block_text).collect();
    let right: Vec<String> = b.materialize(OpId::ROOT).iter().map(block_text).collect();

    assert_eq!(left, right, "replicas diverged");
    assert_eq!(left.len(), 3, "no edit was lost");
    assert!(left.contains(&"from the human".to_string()));
    assert!(left.contains(&"from the agent".to_string()));
}

#[test]
fn merging_is_idempotent_and_order_independent() {
    let mut log = OpLog::new();
    log.register_actor(person());
    log.push(
        1,
        1,
        Change::Insert {
            parent: OpId::ROOT,
            left: None,
            block: Block::paragraph("x"),
        },
    );

    let snapshot = log.clone();
    let before = log.len();
    log.merge(&snapshot);
    log.merge(&snapshot);
    assert_eq!(log.len(), before, "merging the same log grew it");
}

#[test]
fn a_replace_wins_by_lamport_clock_not_by_wall_clock() {
    let mut log = OpLog::new();
    log.register_actor(person());
    log.register_actor(agent());

    let node = log.push(
        1,
        5_000,
        Change::Insert {
            parent: OpId::ROOT,
            left: None,
            block: Block::paragraph("original"),
        },
    );
    // The later operation carries an *earlier* timestamp, as it would from a
    // machine with a slow clock. Causal order must still decide.
    log.push(
        2,
        1,
        Change::Replace {
            target: node,
            block: Block::paragraph("edited by the agent"),
        },
    );

    let blocks = log.materialize(OpId::ROOT);
    assert_eq!(block_text(&blocks[0]), "edited by the agent");

    let (actor, _) = log.attribution(node).unwrap();
    assert_eq!(actor.name, "claude-opus-5");
    assert!(actor.is_agent, "attribution must say it was a model");
}

#[test]
fn deleting_removes_a_node_but_keeps_it_distinguishable_from_absent() {
    let mut log = OpLog::new();
    log.register_actor(person());
    let node = log.push(
        1,
        1,
        Change::Insert {
            parent: OpId::ROOT,
            left: None,
            block: Block::paragraph("doomed"),
        },
    );
    log.push(1, 2, Change::Delete { target: node });

    assert!(log.materialize(OpId::ROOT).is_empty());
    assert!(log.deleted().contains(&node));
}

#[test]
fn the_log_round_trips_through_bytes() {
    let mut log = OpLog::new();
    log.register_actor(person());
    log.register_actor(agent());
    let first = log.push(
        1,
        100,
        Change::Insert {
            parent: OpId::ROOT,
            left: None,
            block: Block::Heading {
                level: 2,
                content: vec![Inline::Run(crate::doc::Run::plain("Heading"))],
            },
        },
    );
    log.push(
        2,
        101,
        Change::Replace {
            target: first,
            block: Block::paragraph("replaced"),
        },
    );
    log.push(2, 102, Change::Delete { target: first });

    let restored = OpLog::parse(&log.encode()).unwrap();
    assert_eq!(restored.len(), log.len());
    assert_eq!(restored.actor(2).unwrap().name, "claude-opus-5");
    assert!(restored.actor(2).unwrap().is_agent);
    assert_eq!(
        restored.materialize(OpId::ROOT).len(),
        log.materialize(OpId::ROOT).len()
    );
}

#[test]
fn a_log_stored_in_a_file_comes_back_out() {
    let mut log = OpLog::new();
    log.register_actor(agent());
    log.push(
        2,
        1,
        Change::Insert {
            parent: OpId::ROOT,
            left: None,
            block: Block::paragraph("written by an agent"),
        },
    );

    let bytes = AdfWriter::new()
        .with_oplog(log)
        .write(&SemanticDoc::default(), "adf");
    let adf = AdfDoc::open(&bytes).unwrap();

    let restored = adf.oplog().unwrap();
    assert_eq!(restored.len(), 1);
    assert_eq!(
        block_text(&restored.materialize(OpId::ROOT)[0]),
        "written by an agent"
    );
}

#[test]
fn appending_an_edit_does_not_rewrite_the_document() {
    let document = rich_document();
    let mut log = OpLog::new();
    log.register_actor(person());
    log.push(
        1,
        1,
        Change::Insert {
            parent: OpId::ROOT,
            left: None,
            block: Block::paragraph("first"),
        },
    );

    let mut bytes = AdfWriter::new()
        .with_oplog(log.clone())
        .write(&document, "adf");
    let original_len = bytes.len();
    let mark = log.ops().next_back().map(|op| op.id);

    log.push(
        1,
        2,
        Change::Insert {
            parent: OpId::ROOT,
            left: mark,
            block: Block::paragraph("appended later"),
        },
    );
    super::write::append_ops(&mut bytes, &log, mark);

    // The document is unchanged and the file only grew by the edit.
    let grew = bytes.len() - original_len;
    assert!(grew < 512, "append wrote {grew} bytes for a one-block edit");

    let adf = AdfDoc::open(&bytes).unwrap();
    assert_eq!(
        crate::doc::to_markdown(&adf.to_semantic().unwrap()),
        crate::doc::to_markdown(&document)
    );

    let restored = adf.oplog().unwrap();
    assert_eq!(restored.len(), 2, "the appended operation is not readable");
    let blocks = restored.materialize(OpId::ROOT);
    assert_eq!(block_text(&blocks[1]), "appended later");
}

#[test]
fn a_document_can_be_seeded_into_a_log_for_editing() {
    let blocks = vec![Block::paragraph("one"), Block::paragraph("two")];
    let log = super::oplog::from_blocks(&blocks, 1, 0);

    let replayed: Vec<String> = log.materialize(OpId::ROOT).iter().map(block_text).collect();
    assert_eq!(replayed, ["one", "two"], "seeding changed the order");
}

/// The plain text of a block, for readable assertions.
fn block_text(block: &Block) -> String {
    let mut text = String::new();
    crate::doc::block_text_into(block, &mut text);
    text.trim().to_string()
}
