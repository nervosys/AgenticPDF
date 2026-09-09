// SPDX-License-Identifier: AGPL-3.0-or-later
//! End-to-end tests for the spreadsheet profile: reading a workbook as cells,
//! storing it in ADF, and reading it back.
//!
//! The point being tested throughout is that a spreadsheet survives as a
//! spreadsheet. Every assertion here fails against the old behaviour, where a
//! workbook became a table of strings and the type, the formula and the format
//! were gone by the time anything could ask about them.
//!
//! The fixture is synthesised rather than committed, following `real_docs.rs`:
//! an `.xlsx` is a ZIP of XML, so building one is a few lines and the tests run
//! anywhere.

use irondocuments::adf::{AdfDoc, AdfWriter, Profile};
use irondocuments::detect::Format;
use irondocuments::document::Document;
use irondocuments::sheet::{CellError, Value};

const OOXML_RELS: &str = r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="MAIN"/>
</Relationships>"#;

/// A workbook holding every case the model exists to keep: a formula with a
/// cached result, a date that is a number with a format, a text cell that looks
/// like a number, a boolean, an error, a hidden sheet, a hidden row, a hidden
/// column, a merge and a defined name.
fn sample_xlsx() -> Vec<u8> {
    let workbook = r#"<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
        <sheets>
          <sheet name="Ledger" sheetId="1" r:id="rId1"/>
          <sheet name="Secret" sheetId="2" state="hidden" r:id="rId2"/>
        </sheets>
        <definedNames><definedName name="Tax_Rate">Ledger!$D$1</definedName></definedNames>
      </workbook>"#;
    let rels = r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
        <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/>
        <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet2.xml"/>
      </Relationships>"#;
    let shared = r#"<sst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
        <si><t>Item</t></si><si><t>Cost</t></si><si><t>Rent</t></si>
        <si><t>Total</t></si><si><t>007</t></si><si><t>HIDDEN ROW PAYLOAD</t></si>
        <si><t>HIDDEN COLUMN PAYLOAD</t></si><si><t>HIDDEN SHEET PAYLOAD</t></si>
      </sst>"#;
    // Style 1 is a date format, style 2 a percentage; style 0 is General.
    let styles = r#"<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
        <numFmts><numFmt numFmtId="164" formatCode="yyyy-mm-dd"/><numFmt numFmtId="165" formatCode="0.0%"/></numFmts>
        <cellXfs count="3"><xf numFmtId="0"/><xf numFmtId="164"/><xf numFmtId="165"/></cellXfs>
      </styleSheet>"#;
    let sheet1 = r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
        <sheetViews><sheetView><pane xSplit="0" ySplit="1" state="frozen"/></sheetView></sheetViews>
        <cols><col min="3" max="3" width="14.5" hidden="1"/></cols>
        <sheetData>
          <row r="1"><c r="A1" t="s"><v>0</v></c><c r="B1" t="s"><v>1</v></c><c r="C1" t="s"><v>6</v></c></row>
          <row r="2"><c r="A2" t="s"><v>2</v></c><c r="B2"><v>1200</v></c></row>
          <row r="3" hidden="1"><c r="A3" t="s"><v>5</v></c></row>
          <row r="4"><c r="A4" t="s"><v>3</v></c><c r="B4"><f>SUM(B2:B2)</f><v>1200</v></c></row>
          <row r="5"><c r="A5" s="1"><v>46095</v></c><c r="B5" s="2"><v>0.085</v></c></row>
          <row r="6"><c r="A6" t="s"><v>4</v></c><c r="B6" t="b"><v>1</v></c><c r="D6" t="e"><v>#DIV/0!</v></c></row>
        </sheetData>
        <mergeCells><mergeCell ref="A1:B1"/></mergeCells>
      </worksheet>"#;
    let sheet2 = r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData>
        <row r="1"><c r="A1" t="s"><v>7</v></c></row>
      </sheetData></worksheet>"#;
    let root = OOXML_RELS.replace("MAIN", "xl/workbook.xml");

    irondocuments::testing::build_zip(&[
        ("_rels/.rels", root.as_bytes(), true),
        ("xl/workbook.xml", workbook.as_bytes(), true),
        ("xl/_rels/workbook.xml.rels", rels.as_bytes(), true),
        ("xl/sharedStrings.xml", shared.as_bytes(), true),
        ("xl/styles.xml", styles.as_bytes(), true),
        ("xl/worksheets/sheet1.xml", sheet1.as_bytes(), true),
        ("xl/worksheets/sheet2.xml", sheet2.as_bytes(), true),
    ])
}

fn ledger() -> irondocuments::sheet::Sheet {
    let (workbook, _) =
        irondocuments::formats::parse_workbook(&sample_xlsx(), Format::Xlsx).unwrap();
    workbook.sheets.into_iter().next().unwrap()
}

#[test]
fn a_cell_keeps_the_type_it_had() {
    let sheet = ledger();

    assert_eq!(sheet.cell(1, 1).unwrap().value, Value::Number(1200.0));
    assert_eq!(sheet.cell(5, 1).unwrap().value, Value::Bool(true));
    assert_eq!(
        sheet.cell(5, 3).unwrap().value,
        Value::Error(CellError::Div0)
    );

    // The case flattening always got wrong: a text cell that looks like a
    // number stays text, so a part number does not become the integer seven.
    assert_eq!(sheet.cell(5, 0).unwrap().value, Value::Text("007".into()));
    assert!(sheet.cell(5, 0).unwrap().value.as_number().is_none());
}

#[test]
fn a_formula_arrives_with_its_cached_result() {
    let sheet = ledger();
    let total = sheet.cell(3, 1).unwrap();

    assert_eq!(total.formula.as_deref(), Some("SUM(B2:B2)"));
    assert_eq!(total.value, Value::Number(1200.0));
    assert_eq!(sheet.formulas().count(), 1);
}

#[test]
fn a_date_stays_a_number_and_says_how_to_read_itself() {
    let sheet = ledger();
    let date = sheet.cell(4, 0).unwrap();

    // The serial is the truth of the file; the format is what makes it a date.
    // Rendering it to "2026-03-14" here would be the flattening this model
    // exists to avoid.
    assert_eq!(date.value, Value::Number(46095.0));
    assert_eq!(date.number_format.as_deref(), Some("yyyy-mm-dd"));
    assert_eq!(
        sheet.cell(4, 1).unwrap().number_format.as_deref(),
        Some("0.0%")
    );
    // General is the absence of a format, not a format.
    assert_eq!(sheet.cell(1, 1).unwrap().number_format, None);
}

#[test]
fn geometry_that_is_not_content_survives_too() {
    let sheet = ledger();

    assert_eq!(sheet.frozen, Some((1, 0)));
    assert_eq!(sheet.merges.len(), 1);
    assert_eq!(sheet.merges[0].reference(), "A1:B1");

    let (_, defined) = {
        let (workbook, _) =
            irondocuments::formats::parse_workbook(&sample_xlsx(), Format::Xlsx).unwrap();
        (workbook.sheets.len(), workbook.defined_names)
    };
    assert_eq!(defined.len(), 1);
    assert_eq!(defined[0].name, "Tax_Rate");
    assert_eq!(defined[0].refers_to, "Ledger!$D$1");
}

#[test]
fn hidden_content_is_kept_in_the_cells_and_kept_out_of_the_text() {
    let (workbook, metadata) =
        irondocuments::formats::parse_workbook(&sample_xlsx(), Format::Xlsx).unwrap();

    // Kept: a conversion that dropped the author's hidden sheet would be
    // writing out a workbook that is missing part of the one it read.
    assert_eq!(workbook.sheets.len(), 2);
    assert!(workbook.sheets[1].hidden);
    assert_eq!(workbook.sheets[0].hidden_rows, vec![2]);
    assert_eq!(workbook.sheets[0].hidden_columns, vec![2]);
    let cells = format!("{:?}", workbook.sheets);
    for payload in ["HIDDEN ROW", "HIDDEN COLUMN", "HIDDEN SHEET"] {
        assert!(cells.contains(payload), "{payload} should be in the cells");
    }

    // Kept out: the text a reader is shown is what the author left visible,
    // which is how every reader of the source formats has always behaved.
    let bytes = AdfWriter::new().write_workbook(&workbook, &metadata, "xlsx");
    let markdown = Document::open(&bytes).unwrap().to_markdown();
    for payload in ["HIDDEN ROW", "HIDDEN COLUMN", "HIDDEN SHEET"] {
        assert!(
            !markdown.contains(payload),
            "{payload} leaked into text:\n{markdown}"
        );
    }
    assert!(markdown.contains("Rent"), "{markdown}");
}

#[test]
fn a_workbook_round_trips_through_adf() {
    let (workbook, metadata) =
        irondocuments::formats::parse_workbook(&sample_xlsx(), Format::Xlsx).unwrap();
    let bytes = AdfWriter::new().write_workbook(&workbook, &metadata, "xlsx");

    let file = AdfDoc::open(&bytes).unwrap();
    assert_eq!(file.profile(), Profile::Spreadsheet);
    assert_eq!(file.sheet_count(), 2);

    let read = file.workbook().unwrap();
    assert_eq!(read.sheets.len(), workbook.sheets.len());
    for (before, after) in workbook.sheets.iter().zip(&read.sheets) {
        assert_eq!(before.name, after.name);
        assert_eq!(before.cells, after.cells);
        assert_eq!(before.merges, after.merges);
        assert_eq!(before.frozen, after.frozen);
        assert_eq!(before.hidden, after.hidden);
        assert_eq!(before.hidden_rows, after.hidden_rows);
        assert_eq!(before.hidden_columns, after.hidden_columns);
    }
    assert_eq!(read.defined_names.len(), 1);
}

#[test]
fn an_imported_workbook_is_searchable_and_citable() {
    let (workbook, metadata) =
        irondocuments::formats::parse_workbook(&sample_xlsx(), Format::Xlsx).unwrap();
    let bytes = irondocuments::agent_ops::write_adf_workbook(
        &workbook,
        &metadata,
        "ledger.xlsx",
        Format::Xlsx,
    );

    let file = AdfDoc::open(&bytes).unwrap();
    let hits = file.search("Rent").unwrap();
    assert_eq!(hits.len(), 1);
    // Sheet zero, row one: a hit points at a place, not at the file.
    assert_eq!(hits[0].0.section, 0);
    assert_eq!(hits[0].0.blocks, [1, 1]);

    // The quotation the search returned is the one provenance verifies, which
    // is the whole point of recording it at the same address.
    let quoted = hits[0].1.clone();
    let checked = irondocuments::agent_ops::verify(&bytes, &quoted, 0, 1).unwrap();
    assert_eq!(checked["status"], "matched");

    let invented = irondocuments::agent_ops::verify(&bytes, "Rent 999999", 0, 1).unwrap();
    assert_eq!(invented["status"], "drifted");
}

#[test]
fn a_spreadsheet_adf_still_answers_every_question_asked_of_a_document() {
    let (workbook, metadata) =
        irondocuments::formats::parse_workbook(&sample_xlsx(), Format::Xlsx).unwrap();
    let bytes = AdfWriter::new().write_workbook(&workbook, &metadata, "xlsx");
    let document = Document::open(&bytes).unwrap();

    // Nothing below knows what a cell is; they all go through the rendered
    // view, which is why adding a profile did not mean touching any of them.
    assert!(document.to_markdown().contains("| Item |"));
    assert!(document.extract_text().contains("Rent"));
    assert_eq!(document.tables().len(), 1);
    assert!(!document.generate_chunks(50, 0).is_empty());
}

#[test]
fn asking_a_report_for_its_cells_says_so_rather_than_inventing_a_grid() {
    let markdown = b"# Report\n\nRevenue grew.\n";
    let error = irondocuments::formats::parse_workbook(markdown, Format::Markdown).unwrap_err();
    assert!(
        format!("{error}").contains("no cell model"),
        "got {error:?}"
    );
}
