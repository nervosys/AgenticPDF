// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Derived from anydoc (https://github.com/firecrawl/anydoc), MIT licensed,
// Copyright (c) 2026 Sideguide Technologies Inc. See LICENSE-MIT-anydoc.txt.
//
//! Sprm (single property modifier) walking and application, per [MS-DOC].
//!
//! A `.doc` does not store formatting as properties on runs; it stores *deltas*
//! — grpprls, which are byte strings of sprms — that are applied in a defined
//! order over the style chain. Getting that order wrong is what makes a naive
//! reader emit documents that are entirely bold or entirely plain.
//!
//! The subtlety is toggles. `sprmCFBold` does not mean "bold"; its operand
//! selects among *off*, *on*, *whatever the style says*, and *the opposite of
//! what the style says*. Resolving the last two needs the style chain's value
//! as a base, which is why every application here takes one.

use crate::container::ole::{u16_at, u32_at};

/// Length of an sprm's operand, from its `spra` field (the top three bits).
///
/// Two sprms carry variable-length operands whose size is a prefix of the
/// operand itself; `sprmTDefTable` uses a two-byte prefix, everything else in
/// that class uses one byte.
fn operand_len(sprm: u16, operand: &[u8]) -> usize {
    match sprm >> 13 {
        0 | 1 => 1,
        2 | 4 | 5 => 2,
        3 => 4,
        7 => 3,
        _ => {
            if sprm == 0xD608 {
                u16_at(operand, 0).map(|len| len as usize + 1).unwrap_or(0)
            } else {
                operand.first().map(|&len| len as usize + 1).unwrap_or(0)
            }
        }
    }
}

/// Walk a grpprl, calling `visit` with each sprm and its operand.
pub fn walk(grpprl: &[u8], mut visit: impl FnMut(u16, &[u8])) {
    let mut pos = 0usize;
    while pos + 2 <= grpprl.len() {
        let sprm = u16::from_le_bytes([grpprl[pos], grpprl[pos + 1]]);
        pos += 2;
        let len = operand_len(sprm, &grpprl[pos..]);
        let Some(operand) = grpprl.get(pos..).and_then(|rest| rest.get(..len)) else {
            break;
        };
        visit(sprm, operand);
        pos += len;
    }
}

/// Resolve a toggle operand against the style chain's value.
///
/// `0x80` means "inherit" and `0x81` means "invert" — the two cases that make a
/// character sprm meaningless without knowing what the style already said.
fn toggle(operand: &[u8], base: bool) -> Option<bool> {
    match operand.first() {
        Some(0) => Some(false),
        Some(1) => Some(true),
        Some(0x80) => Some(base),
        Some(0x81) => Some(!base),
        _ => None,
    }
}

/// Character formatting a CHPX contributes.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CharProps {
    pub bold: bool,
    pub italic: bool,
    pub strike: bool,
    pub underline: bool,
    /// `sprmCFVanish`: hidden text — present in the file, painted nowhere.
    pub hidden: bool,
    /// Half-points, as everywhere in Word's binary format.
    pub half_points: Option<u16>,
}

/// The `sprmCIstd` character-style reference in a CHPX, if any.
pub fn chpx_istd(grpprl: &[u8]) -> Option<u16> {
    let mut istd = None;
    walk(grpprl, |sprm, operand| {
        if sprm == 0x4A30 {
            istd = u16_at(operand, 0);
        }
    });
    istd
}

/// Apply a CHPX over `current`, resolving toggles against `style_base`.
pub fn apply_chpx(grpprl: &[u8], current: CharProps, style_base: CharProps) -> CharProps {
    let mut props = current;
    walk(grpprl, |sprm, operand| match sprm {
        // The toggle sprms: bold, italic, strikethrough, hidden.
        0x0835 => {
            if let Some(value) = toggle(operand, style_base.bold) {
                props.bold = value;
            }
        }
        0x0836 => {
            if let Some(value) = toggle(operand, style_base.italic) {
                props.italic = value;
            }
        }
        0x0837 => {
            if let Some(value) = toggle(operand, style_base.strike) {
                props.strike = value;
            }
        }
        // sprmCFVanish — Word's hidden text, and the reason this format needs
        // the same injection scan the others get. Note 0x083C, not 0x083A:
        // the neighbouring codes are small-caps and all-caps, and picking the
        // wrong one silently reports every document as clean.
        0x083C => {
            if let Some(value) = toggle(operand, style_base.hidden) {
                props.hidden = value;
            }
        }
        // sprmCKul: underline kind, where 0 means none.
        0x2A3E => {
            props.underline = operand.first().is_some_and(|&kind| kind != 0);
        }
        // sprmCHps: font size in half-points.
        0x4A43 => props.half_points = u16_at(operand, 0),
        _ => {}
    });
    props
}

/// Apply a CHPX as a style *definition* layer: the parent's value is the base
/// its toggles resolve against.
pub fn apply_style_chpx(grpprl: &[u8], parent: CharProps) -> CharProps {
    apply_chpx(grpprl, parent, parent)
}

/// One row's table properties, from `sprmTDefTable` and its companions in the
/// row-terminating paragraph's PAPX.
#[derive(Debug, Clone, Default)]
pub struct Tap {
    /// Cell boundary positions in twips, one more than the cell count.
    pub boundaries: Vec<i16>,
    pub cells: Vec<TapCell>,
    /// `sprmTTableHeader`: this row repeats as a header row.
    pub header: bool,
}

/// Merge flags of one cell.
#[derive(Debug, Clone, Copy, Default)]
pub struct TapCell {
    pub horz_first: bool,
    pub horz_cont: bool,
    pub vert_restart: bool,
    pub vert_cont: bool,
}

/// Paragraph properties a PAPX (or a style's paragraph UPX) contributes.
///
/// Every field is optional because a grpprl is a delta: absent means "inherit",
/// which is not the same as "false".
#[derive(Debug, Clone, Default)]
pub struct PapProps {
    pub in_table: Option<bool>,
    /// This paragraph mark terminates a table row.
    pub ttp: Option<bool>,
    pub outline: Option<Option<u8>>,
    /// List format override index; 0 means unnumbered.
    pub ilfo: Option<u16>,
    pub ilvl: Option<u8>,
    /// Table nesting depth; 1 is a regular table.
    pub itap: Option<i32>,
    pub inner_cell: Option<bool>,
    pub inner_ttp: Option<bool>,
    pub justify: Option<u8>,
    pub tap: Option<Tap>,
}

impl PapProps {
    /// Overlay `over` onto `self`, with `over` winning where it states a value.
    pub fn merge(self, over: PapProps) -> PapProps {
        PapProps {
            in_table: over.in_table.or(self.in_table),
            ttp: over.ttp.or(self.ttp),
            outline: over.outline.or(self.outline),
            ilfo: over.ilfo.or(self.ilfo),
            ilvl: over.ilvl.or(self.ilvl),
            itap: over.itap.or(self.itap),
            inner_cell: over.inner_cell.or(self.inner_cell),
            inner_ttp: over.inner_ttp.or(self.inner_ttp),
            justify: over.justify.or(self.justify),
            tap: over.tap.or(self.tap),
        }
    }
}

/// Apply a paragraph grpprl into `props`.
///
/// `data` is the `Data` stream, needed only for `sprmPHugePapx`, which stores
/// its real grpprl there when it will not fit in an FKP page.
pub fn apply_pap(grpprl: &[u8], data: &[u8], props: &mut PapProps) {
    walk(grpprl, |sprm, operand| match sprm {
        0x2416 => props.in_table = Some(operand.first().is_some_and(|&v| v != 0)),
        0x2417 => props.ttp = Some(operand.first().is_some_and(|&v| v != 0)),
        // sprmPHugePapx: an indirection into the Data stream.
        0x6646 => {
            if let Some(offset) = u32_at(operand, 0).map(|v| v as usize)
                && let Some(len) = u16_at(data, offset).map(|v| v as usize)
                && let Some(huge) = data
                    .get(offset..)
                    .and_then(|rest| rest.get(2..))
                    .and_then(|rest| rest.get(..len))
            {
                apply_pap(huge, &[], props);
            }
        }
        // sprmPOutLvl: 0-8 are heading levels 1-9; 9 means body text.
        0x2640 => {
            if let Some(&value) = operand.first() {
                props.outline = Some(if value < 9 { Some(value + 1) } else { None });
            }
        }
        0x260A => props.ilvl = operand.first().copied(),
        0x460B => props.ilfo = u16_at(operand, 0),
        // sprmPJc / sprmPJc80: paragraph justification.
        0x2403 | 0x2461 => props.justify = operand.first().copied(),
        // Table nesting depth: absolute, then relative.
        0x6649 => props.itap = u32_at(operand, 0).map(|v| v as i32),
        0x664A => {
            if let Some(delta) = u32_at(operand, 0).map(|v| v as i32) {
                props.itap = Some(props.itap.unwrap_or(0).saturating_add(delta));
            }
        }
        0x244B => props.inner_cell = Some(operand.first().is_some_and(|&v| v != 0)),
        0x244C => props.inner_ttp = Some(operand.first().is_some_and(|&v| v != 0)),
        // sprmTDefTable: the row's cell boundaries and merge flags.
        0xD608 => {
            if let Some(tap) = parse_tdef_table(operand) {
                let header = props.tap.as_ref().is_some_and(|existing| existing.header);
                props.tap = Some(Tap { header, ..tap });
            }
        }
        0x3404 => {
            let on = operand.first().is_some_and(|&v| v != 0);
            match &mut props.tap {
                Some(tap) => tap.header = on,
                None if on => {
                    props.tap = Some(Tap {
                        header: true,
                        ..Tap::default()
                    })
                }
                None => {}
            }
        }
        // sprmTVertMerge: one cell's vertical-merge state.
        0xD62B => {
            if let (Some(&index), Some(&flag)) = (operand.get(1), operand.get(2))
                && let Some(tap) = &mut props.tap
                && let Some(cell) = tap.cells.get_mut(index as usize)
            {
                cell.vert_cont = flag == 0x01;
                cell.vert_restart = flag == 0x03;
            }
        }
        _ => {}
    });
}

/// Parse a `TDefTableOperand`: column count, boundary positions, then the TC80
/// records — of which there may be fewer than there are columns.
fn parse_tdef_table(operand: &[u8]) -> Option<Tap> {
    const TC80_SIZE: usize = 20;
    let columns = *operand.get(2)? as usize;
    // Word's own limit; a larger value means the operand is corrupt.
    if columns > 63 {
        return None;
    }

    let mut boundaries = Vec::with_capacity(columns + 1);
    for index in 0..=columns {
        boundaries.push(crate::container::ole::i16_at(operand, 3 + index * 2)?);
    }

    let mut cells = vec![TapCell::default(); columns];
    let base = 3 + (columns + 1) * 2;
    for (index, cell) in cells.iter_mut().enumerate() {
        let Some(flags) = u16_at(operand, base + index * TC80_SIZE) else {
            break; // Fewer TC80 records than columns: the defaults stand.
        };
        let horizontal = flags & 0x3;
        cell.horz_cont = horizontal == 1;
        cell.horz_first = horizontal >= 2;
        let vertical = (flags >> 5) & 0x3;
        cell.vert_cont = vertical == 1;
        cell.vert_restart = vertical == 3;
    }
    Some(Tap {
        boundaries,
        cells,
        header: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a grpprl from `(sprm, operand)` pairs.
    fn grpprl(entries: &[(u16, &[u8])]) -> Vec<u8> {
        let mut out = Vec::new();
        for (sprm, operand) in entries {
            out.extend_from_slice(&sprm.to_le_bytes());
            out.extend_from_slice(operand);
        }
        out
    }

    #[test]
    fn walks_operands_of_every_length_class() {
        let bytes = grpprl(&[
            (0x0835, &[1]),                    // spra 0: one byte
            (0x4A43, &[0x18, 0x00]),           // spra 2: two bytes
            (0x6649, &[1, 0, 0, 0]),           // spra 3: four bytes
        ]);
        let mut seen = Vec::new();
        walk(&bytes, |sprm, operand| seen.push((sprm, operand.len())));
        assert_eq!(seen, vec![(0x0835, 1), (0x4A43, 2), (0x6649, 4)]);
    }

    #[test]
    fn a_truncated_grpprl_stops_rather_than_reading_past_the_end() {
        // The operand is declared but absent.
        let bytes = vec![0x43, 0x4A];
        let mut seen = 0;
        walk(&bytes, |_, _| seen += 1);
        assert_eq!(seen, 0);
    }

    #[test]
    fn toggles_resolve_against_the_style_base() {
        let base = CharProps {
            bold: true,
            ..CharProps::default()
        };
        // 0x80 means "whatever the style says" — here, bold.
        let inherit = apply_chpx(&grpprl(&[(0x0835, &[0x80])]), CharProps::default(), base);
        assert!(inherit.bold, "0x80 should inherit the style's bold");

        // 0x81 means "the opposite of what the style says".
        let invert = apply_chpx(&grpprl(&[(0x0835, &[0x81])]), CharProps::default(), base);
        assert!(!invert.bold, "0x81 should invert the style's bold");

        // 0 and 1 are absolute.
        assert!(!apply_chpx(&grpprl(&[(0x0835, &[0])]), base, base).bold);
        assert!(apply_chpx(&grpprl(&[(0x0835, &[1])]), CharProps::default(), base).bold);
    }

    #[test]
    fn reads_the_character_property_sprms() {
        let bytes = grpprl(&[
            (0x0836, &[1]),          // italic
            (0x0837, &[1]),          // strike
            (0x083C, &[1]),          // vanish
            (0x2A3E, &[1]),          // underline, kind 1
            (0x4A43, &[0x18, 0x00]), // 24 half-points = 12pt
        ]);
        let props = apply_chpx(&bytes, CharProps::default(), CharProps::default());
        assert!(props.italic && props.strike && props.hidden && props.underline);
        assert_eq!(props.half_points, Some(24));
    }

    #[test]
    fn underline_kind_zero_means_no_underline() {
        let props = apply_chpx(
            &grpprl(&[(0x2A3E, &[0])]),
            CharProps::default(),
            CharProps::default(),
        );
        assert!(!props.underline);
    }

    #[test]
    fn reads_the_paragraph_property_sprms() {
        let bytes = grpprl(&[
            (0x2416, &[1]),          // in table
            (0x2417, &[1]),          // row terminator
            (0x2640, &[1]),          // outline level 1 -> heading 2
            (0x260A, &[2]),          // list level 2
            (0x460B, &[3, 0]),       // ilfo 3
            (0x6649, &[1, 0, 0, 0]), // table depth 1
        ]);
        let mut props = PapProps::default();
        apply_pap(&bytes, &[], &mut props);
        assert_eq!(props.in_table, Some(true));
        assert_eq!(props.ttp, Some(true));
        assert_eq!(props.outline, Some(Some(2)));
        assert_eq!(props.ilvl, Some(2));
        assert_eq!(props.ilfo, Some(3));
        assert_eq!(props.itap, Some(1));
    }

    #[test]
    fn outline_level_nine_means_body_text_not_a_heading() {
        let mut props = PapProps::default();
        apply_pap(&grpprl(&[(0x2640, &[9])]), &[], &mut props);
        assert_eq!(props.outline, Some(None), "level 9 is body text");
    }

    #[test]
    fn merge_lets_the_overlay_win_only_where_it_states_a_value() {
        let base = PapProps {
            ilfo: Some(1),
            ilvl: Some(0),
            ..PapProps::default()
        };
        let over = PapProps {
            ilvl: Some(2),
            ..PapProps::default()
        };
        let merged = base.merge(over);
        assert_eq!(merged.ilfo, Some(1), "not stated by the overlay");
        assert_eq!(merged.ilvl, Some(2), "stated by the overlay");
    }

    #[test]
    fn parses_a_table_definition_with_its_boundaries() {
        // Two columns: cb, count, three boundaries, then two TC80 records.
        let mut operand = vec![0u8, 0];
        operand.push(2);
        for boundary in [0i16, 4000, 8000] {
            operand.extend_from_slice(&boundary.to_le_bytes());
        }
        // First cell starts a horizontal merge; second continues it.
        let mut tc80 = vec![0u8; 20];
        tc80[0] = 0x02;
        operand.extend_from_slice(&tc80);
        let mut tc80b = vec![0u8; 20];
        tc80b[0] = 0x01;
        operand.extend_from_slice(&tc80b);

        let tap = parse_tdef_table(&operand).expect("parsed");
        assert_eq!(tap.boundaries, vec![0, 4000, 8000]);
        assert_eq!(tap.cells.len(), 2);
        assert!(tap.cells[0].horz_first);
        assert!(tap.cells[1].horz_cont);
    }

    #[test]
    fn rejects_an_implausible_column_count() {
        assert!(parse_tdef_table(&[0, 0, 200]).is_none());
    }

    #[test]
    fn huge_papx_is_read_from_the_data_stream() {
        // The real grpprl lives in Data, length-prefixed, when it will not fit
        // in an FKP page.
        let inner = grpprl(&[(0x2640, &[0])]);
        let mut data = (inner.len() as u16).to_le_bytes().to_vec();
        data.extend_from_slice(&inner);

        let mut props = PapProps::default();
        apply_pap(&grpprl(&[(0x6646, &[0, 0, 0, 0])]), &data, &mut props);
        assert_eq!(props.outline, Some(Some(1)), "indirect grpprl not applied");
    }

    #[test]
    fn finds_the_character_style_reference() {
        assert_eq!(chpx_istd(&grpprl(&[(0x4A30, &[5, 0])])), Some(5));
        assert_eq!(chpx_istd(&grpprl(&[(0x0835, &[1])])), None);
    }
}
