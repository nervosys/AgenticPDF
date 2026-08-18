// SPDX-License-Identifier: AGPL-3.0-or-later
//! TrueType font programs: `/FontFile2` streams, read into glyph outlines.
//!
//! Type 1 covers what TeX emits; TrueType covers nearly everything else. A
//! `.docx` exported to PDF, anything from Word or a browser's print path,
//! most CID fonts for CJK -- all arrive here.
//!
//! The format is a table directory rather than a program: `loca` says where
//! each glyph's outline lives inside `glyf`, and the outline is a list of
//! closed contours of quadratic B-spline points. Quadratics are flattened on
//! the way out, because every consumer here fills polygons.
//!
//! Glyphs are addressed by index, not by name, which is the substantive
//! difference from Type 1. A PDF simple font gives character codes, so a
//! `cmap` subtable translates; a CID font addresses glyphs directly.

use std::collections::HashMap;

use super::Glyph;

/// Curve flattening, matching the Type 1 reader.
const CURVE_STEPS: usize = 8;

/// A parsed TrueType font.
#[derive(Debug, Clone)]
pub struct TrueTypeFont {
    /// The whole font, kept because glyph outlines are read on demand.
    data: Vec<u8>,
    tables: HashMap<[u8; 4], (usize, usize)>,
    /// Glyph offsets from `loca`, one more than the glyph count.
    loca: Vec<u32>,
    glyf: (usize, usize),
    /// Design units per em, from `head`. 2048 is usual, 1000 happens.
    pub units_per_em: f64,
    /// Unicode to glyph index, from the best available `cmap` subtable.
    unicode_map: HashMap<u32, u16>,
    /// Raw byte codes to glyph index, from a `(3,0)` symbol or `(1,0)` Mac
    /// subtable. A PDF simple font addresses glyphs this way.
    byte_map: HashMap<u32, u16>,
}

/// How deep a composite glyph may nest. Composites reference other glyphs and
/// a damaged font can make that a cycle.
const MAX_COMPOSITE_DEPTH: usize = 8;

impl TrueTypeFont {
    /// Parse a `/FontFile2` stream.
    ///
    /// Returns `None` rather than erroring: a font that will not parse should
    /// cost the caller a fallback, not a failed page.
    pub fn parse(data: &[u8]) -> Option<TrueTypeFont> {
        let tables = read_table_directory(data)?;

        let head = table(data, &tables, b"head")?;
        let units_per_em = be_u16(head, 18)? as f64;
        // A zero here would divide every coordinate by nothing.
        let units_per_em = match units_per_em {
            0.0 => 1000.0,
            value => value,
        };
        let long_loca = be_i16(head, 50)? != 0;

        let maxp = table(data, &tables, b"maxp")?;
        let glyph_count = be_u16(maxp, 4)? as usize;

        let loca_range = *tables.get(b"loca")?;
        let loca = read_loca(data, loca_range, glyph_count, long_loca)?;
        let glyf = *tables.get(b"glyf")?;

        let (unicode_map, byte_map) = match tables.get(b"cmap") {
            Some(&range) => read_cmap(data, range),
            None => (HashMap::new(), HashMap::new()),
        };

        Some(TrueTypeFont {
            data: data.to_vec(),
            tables,
            loca,
            glyf,
            units_per_em,
            unicode_map,
            byte_map,
        })
    }

    /// The scale that takes design units to text space, as a `/FontMatrix`.
    pub fn font_matrix(&self) -> [f64; 6] {
        let scale = 1.0 / self.units_per_em;
        [scale, 0.0, 0.0, scale, 0.0, 0.0]
    }

    pub fn glyph_count(&self) -> usize {
        self.loca.len().saturating_sub(1)
    }

    /// The glyph index a character code selects in a simple font.
    ///
    /// The order is the one every PDF reader ends up with: a symbol font maps
    /// codes in the `0xF000` private-use block, a Mac subtable maps the raw
    /// byte, and otherwise the code is Unicode. A font that answers to none of
    /// these is usually addressed by index anyway.
    pub fn glyph_index(&self, code: u32, unicode: Option<char>) -> Option<u16> {
        self.glyph_candidates(code, unicode).into_iter().next()
    }

    /// Every glyph index a code might select, best first.
    ///
    /// A subset font can carry a `cmap` entry that points at a blank glyph
    /// while another route reaches the real one -- Word's subsets do this for
    /// the WinAnsi punctuation range. Taking the first match and stopping
    /// loses those characters, so the caller tries each in turn and keeps the
    /// one that actually has an outline.
    pub fn glyph_candidates(&self, code: u32, unicode: Option<char>) -> Vec<u16> {
        // Ordered best first: a symbol font's private-use block, the raw
        // byte, then the character the document decoded the code to.
        let candidates = [
            self.byte_map.get(&(0xF000 + code)).copied(),
            self.byte_map.get(&code).copied(),
            unicode.and_then(|ch| self.unicode_map.get(&(ch as u32)).copied()),
            unicode.and_then(|ch| self.byte_map.get(&(0xF000 + ch as u32)).copied()),
            self.unicode_map.get(&code).copied(),
        ];
        let mut out: Vec<u16> = Vec::new();
        for index in candidates.into_iter().flatten() {
            if !out.contains(&index) {
                out.push(index);
            }
        }
        // A subset with no usable character map addresses glyphs by index,
        // which is the convention every reader falls back to.
        if out.is_empty() && self.unicode_map.is_empty() && self.byte_map.is_empty() {
            out.push(code as u16);
        }
        out
    }

    /// The outline of a glyph, by index.
    pub fn outline(&self, index: u16) -> Option<Glyph> {
        let mut contours = Vec::new();
        self.append_outline(index, [1.0, 0.0, 0.0, 1.0, 0.0, 0.0], &mut contours, 0);
        Some(Glyph {
            contours,
            advance: self.advance(index).unwrap_or(0.0),
        })
    }

    /// The advance width from `hmtx`, in design units.
    fn advance(&self, index: u16) -> Option<f32> {
        let hhea = table(&self.data, &self.tables, b"hhea")?;
        let long_metrics = be_u16(hhea, 34)? as usize;
        let hmtx = table(&self.data, &self.tables, b"hmtx")?;
        let at = match (index as usize) < long_metrics {
            true => index as usize * 4,
            // Past the long metrics every glyph shares the last advance.
            false => long_metrics.saturating_sub(1) * 4,
        };
        be_u16(hmtx, at).map(|value| value as f32)
    }

    fn append_outline(
        &self,
        index: u16,
        transform: [f32; 6],
        out: &mut Vec<Vec<[f32; 2]>>,
        depth: usize,
    ) {
        if depth > MAX_COMPOSITE_DEPTH {
            return;
        }
        let Some(bytes) = self.glyph_bytes(index) else {
            return;
        };
        // An empty entry is a blank glyph, which `space` legitimately is.
        if bytes.len() < 10 {
            return;
        }
        let Some(contour_count) = be_i16(bytes, 0) else {
            return;
        };

        match contour_count >= 0 {
            true => self.simple_outline(bytes, contour_count as usize, transform, out),
            false => self.composite_outline(bytes, transform, out, depth),
        }
    }

    fn glyph_bytes(&self, index: u16) -> Option<&[u8]> {
        let start = *self.loca.get(index as usize)? as usize;
        let end = *self.loca.get(index as usize + 1)? as usize;
        if end <= start {
            return Some(&[]);
        }
        let (offset, length) = self.glyf;
        let from = offset.checked_add(start)?;
        let to = offset.checked_add(end)?;
        if end > length {
            return None;
        }
        self.data.get(from..to)
    }

    fn simple_outline(
        &self,
        bytes: &[u8],
        contour_count: usize,
        transform: [f32; 6],
        out: &mut Vec<Vec<[f32; 2]>>,
    ) {
        let mut at = 10;
        let mut ends = Vec::with_capacity(contour_count);
        for _ in 0..contour_count {
            let Some(end) = be_u16(bytes, at) else { return };
            ends.push(end as usize);
            at += 2;
        }
        let point_count = match ends.last() {
            Some(&last) => last + 1,
            None => return,
        };
        // A contour list that claims more points than the glyph can hold is a
        // damaged font, not a large one.
        if point_count > 10_000 {
            return;
        }

        let Some(instruction_len) = be_u16(bytes, at) else {
            return;
        };
        at += 2 + instruction_len as usize;

        // Flags are run-length encoded: bit 3 means the next byte is a repeat
        // count for the flag just read.
        let mut flags = Vec::with_capacity(point_count);
        while flags.len() < point_count {
            let Some(&flag) = bytes.get(at) else { return };
            at += 1;
            flags.push(flag);
            if flag & 0x08 != 0 {
                let Some(&repeat) = bytes.get(at) else { return };
                at += 1;
                for _ in 0..repeat {
                    if flags.len() >= point_count {
                        break;
                    }
                    flags.push(flag);
                }
            }
        }

        let mut xs = Vec::with_capacity(point_count);
        let mut x = 0i32;
        for &flag in &flags {
            // Bit 1: the delta is one byte. Bit 4 then gives its sign, or, when
            // bit 1 is clear, says the coordinate repeats.
            if flag & 0x02 != 0 {
                let Some(&delta) = bytes.get(at) else { return };
                at += 1;
                x += match flag & 0x10 != 0 {
                    true => delta as i32,
                    false => -(delta as i32),
                };
            } else if flag & 0x10 == 0 {
                let Some(delta) = be_i16(bytes, at) else {
                    return;
                };
                at += 2;
                x += delta as i32;
            }
            xs.push(x);
        }

        let mut ys = Vec::with_capacity(point_count);
        let mut y = 0i32;
        for &flag in &flags {
            if flag & 0x04 != 0 {
                let Some(&delta) = bytes.get(at) else { return };
                at += 1;
                y += match flag & 0x20 != 0 {
                    true => delta as i32,
                    false => -(delta as i32),
                };
            } else if flag & 0x20 == 0 {
                let Some(delta) = be_i16(bytes, at) else {
                    return;
                };
                at += 2;
                y += delta as i32;
            }
            ys.push(y);
        }

        let mut start = 0usize;
        for &end in &ends {
            if end >= point_count || end < start {
                break;
            }
            let points: Vec<(f32, f32, bool)> = (start..=end)
                .map(|i| (xs[i] as f32, ys[i] as f32, flags[i] & 0x01 != 0))
                .collect();
            if let Some(contour) = flatten_quadratic(&points) {
                out.push(
                    contour
                        .into_iter()
                        .map(|[px, py]| apply(transform, px, py))
                        .collect(),
                );
            }
            start = end + 1;
        }
    }

    fn composite_outline(
        &self,
        bytes: &[u8],
        transform: [f32; 6],
        out: &mut Vec<Vec<[f32; 2]>>,
        depth: usize,
    ) {
        let mut at = 10;
        loop {
            let (Some(flags), Some(index)) = (be_u16(bytes, at), be_u16(bytes, at + 2)) else {
                return;
            };
            at += 4;

            // ARG_1_AND_2_ARE_WORDS
            let (dx, dy) = match flags & 0x0001 != 0 {
                true => {
                    let (Some(a), Some(b)) = (be_i16(bytes, at), be_i16(bytes, at + 2)) else {
                        return;
                    };
                    at += 4;
                    (a as f32, b as f32)
                }
                false => {
                    let (Some(&a), Some(&b)) = (bytes.get(at), bytes.get(at + 1)) else {
                        return;
                    };
                    at += 2;
                    (a as i8 as f32, b as i8 as f32)
                }
            };

            let mut component = [1.0f32, 0.0, 0.0, 1.0, dx, dy];
            if flags & 0x0008 != 0 {
                // WE_HAVE_A_SCALE
                let Some(scale) = be_f2dot14(bytes, at) else {
                    return;
                };
                at += 2;
                component[0] = scale;
                component[3] = scale;
            } else if flags & 0x0040 != 0 {
                // X_AND_Y_SCALE
                let (Some(sx), Some(sy)) = (be_f2dot14(bytes, at), be_f2dot14(bytes, at + 2))
                else {
                    return;
                };
                at += 4;
                component[0] = sx;
                component[3] = sy;
            } else if flags & 0x0080 != 0 {
                // TWO_BY_TWO
                let values: Option<Vec<f32>> = (0..4)
                    .map(|i| be_f2dot14(bytes, at + i * 2))
                    .collect::<Option<Vec<f32>>>();
                let Some(values) = values else { return };
                at += 8;
                component[0] = values[0];
                component[1] = values[1];
                component[2] = values[2];
                component[3] = values[3];
            }

            self.append_outline(index, compose(transform, component), out, depth + 1);

            // MORE_COMPONENTS
            if flags & 0x0020 == 0 {
                return;
            }
        }
    }
}

// ============================================================================
// Table plumbing
// ============================================================================

fn read_table_directory(data: &[u8]) -> Option<HashMap<[u8; 4], (usize, usize)>> {
    // A TrueType collection points at several fonts; take the first, which is
    // what a PDF embedding one means.
    let base = match data.get(..4)? {
        b"ttcf" => be_u32(data, 12)? as usize,
        _ => 0,
    };
    let tag = data.get(base..base + 4)?;
    // 0x00010000 is TrueType outlines; `true` is the Mac spelling; `OTTO` is
    // CFF outlines, which this reader cannot draw.
    if !(tag == [0x00, 0x01, 0x00, 0x00] || tag == b"true" || tag == b"ttcf") {
        return None;
    }
    let count = be_u16(data, base + 4)? as usize;
    if count > 512 {
        return None;
    }
    let mut tables = HashMap::with_capacity(count);
    for entry in 0..count {
        let at = base + 12 + entry * 16;
        let name: [u8; 4] = data.get(at..at + 4)?.try_into().ok()?;
        let offset = be_u32(data, at + 8)? as usize;
        let length = be_u32(data, at + 12)? as usize;
        if offset <= data.len() {
            tables.insert(name, (offset, length.min(data.len() - offset)));
        }
    }
    Some(tables)
}

fn table<'a>(
    data: &'a [u8],
    tables: &HashMap<[u8; 4], (usize, usize)>,
    name: &[u8; 4],
) -> Option<&'a [u8]> {
    let &(offset, length) = tables.get(name)?;
    data.get(offset..offset + length)
}

fn read_loca(
    data: &[u8],
    (offset, length): (usize, usize),
    glyph_count: usize,
    long: bool,
) -> Option<Vec<u32>> {
    let bytes = data.get(offset..offset + length)?;
    let stride = if long { 4 } else { 2 };
    let entries = (glyph_count + 1).min(bytes.len() / stride);
    let mut out = Vec::with_capacity(entries);
    for index in 0..entries {
        let value = match long {
            true => be_u32(bytes, index * 4)?,
            // The short format stores halved offsets.
            false => be_u16(bytes, index * 2)? as u32 * 2,
        };
        out.push(value);
    }
    Some(out)
}

/// Read the `cmap` subtables worth having: a Unicode map and a byte map.
fn read_cmap(
    data: &[u8],
    (offset, length): (usize, usize),
) -> (HashMap<u32, u16>, HashMap<u32, u16>) {
    let mut unicode = HashMap::new();
    let mut bytes_map = HashMap::new();
    let Some(cmap) = data.get(offset..offset + length) else {
        return (unicode, bytes_map);
    };
    let Some(count) = be_u16(cmap, 2) else {
        return (unicode, bytes_map);
    };

    for entry in 0..count as usize {
        let at = 4 + entry * 8;
        let (Some(platform), Some(encoding), Some(sub_offset)) =
            (be_u16(cmap, at), be_u16(cmap, at + 2), be_u32(cmap, at + 4))
        else {
            continue;
        };
        let Some(subtable) = cmap.get(sub_offset as usize..) else {
            continue;
        };
        let mapped = read_cmap_subtable(subtable);
        match (platform, encoding) {
            // Windows symbol and Mac Roman address raw byte codes.
            (3, 0) | (1, 0) => bytes_map.extend(mapped),
            // Windows Unicode BMP, Windows full Unicode, and any Unicode
            // platform table.
            (3, 1) | (3, 10) | (0, _) => unicode.extend(mapped),
            _ => {}
        }
    }
    (unicode, bytes_map)
}

fn read_cmap_subtable(data: &[u8]) -> HashMap<u32, u16> {
    let mut out = HashMap::new();
    let Some(format) = be_u16(data, 0) else {
        return out;
    };
    match format {
        0 => {
            for code in 0..256usize {
                if let Some(&index) = data.get(6 + code)
                    && index != 0
                {
                    out.insert(code as u32, index as u16);
                }
            }
        }
        4 => {
            let Some(seg_x2) = be_u16(data, 6) else {
                return out;
            };
            let segments = seg_x2 as usize / 2;
            for segment in 0..segments {
                let (Some(end), Some(start)) = (
                    be_u16(data, 14 + segment * 2),
                    be_u16(data, 16 + seg_x2 as usize + segment * 2),
                ) else {
                    continue;
                };
                let (Some(delta), Some(range_offset)) = (
                    be_i16(data, 16 + seg_x2 as usize * 2 + segment * 2),
                    be_u16(data, 16 + seg_x2 as usize * 3 + segment * 2),
                ) else {
                    continue;
                };
                if start > end || end == 0xFFFF && start == 0xFFFF {
                    continue;
                }
                for code in start..=end {
                    let index = match range_offset {
                        0 => (code as i32 + delta as i32) as u16,
                        _ => {
                            let at = 16
                                + seg_x2 as usize * 3
                                + segment * 2
                                + range_offset as usize
                                + (code - start) as usize * 2;
                            match be_u16(data, at) {
                                Some(0) | None => continue,
                                Some(raw) => (raw as i32 + delta as i32) as u16,
                            }
                        }
                    };
                    if index != 0 {
                        out.insert(code as u32, index);
                    }
                }
            }
        }
        6 => {
            let (Some(first), Some(count)) = (be_u16(data, 6), be_u16(data, 8)) else {
                return out;
            };
            for entry in 0..count as usize {
                if let Some(index) = be_u16(data, 10 + entry * 2)
                    && index != 0
                {
                    out.insert(first as u32 + entry as u32, index);
                }
            }
        }
        12 => {
            let Some(groups) = be_u32(data, 12) else {
                return out;
            };
            for group in 0..(groups as usize).min(100_000) {
                let at = 16 + group * 12;
                let (Some(start), Some(end), Some(glyph)) =
                    (be_u32(data, at), be_u32(data, at + 4), be_u32(data, at + 8))
                else {
                    break;
                };
                // A group spanning the whole plane is a damaged table.
                if end < start || end - start > 65_535 {
                    continue;
                }
                for code in start..=end {
                    out.insert(code, (glyph + (code - start)) as u16);
                }
            }
        }
        _ => {}
    }
    out
}

// ============================================================================
// Geometry
// ============================================================================

/// Flatten a contour of quadratic B-spline points.
///
/// TrueType stores control points and on-curve points in one list, and allows
/// two consecutive control points, with an implied on-curve point midway
/// between them. Missing that rule rounds every such corner into the wrong
/// place.
fn flatten_quadratic(points: &[(f32, f32, bool)]) -> Option<Vec<[f32; 2]>> {
    if points.len() < 2 {
        return None;
    }
    // Start at an on-curve point; if the contour has none, the midpoint of the
    // first two controls serves, which is what the implied-point rule says.
    let start_index = points.iter().position(|p| p.2);
    let start = match start_index {
        Some(index) => [points[index].0, points[index].1],
        None => [
            (points[0].0 + points[1].0) / 2.0,
            (points[0].1 + points[1].1) / 2.0,
        ],
    };
    let offset = start_index.map(|i| i + 1).unwrap_or(0);

    let mut out = vec![start];
    let mut current = start;
    let mut control: Option<[f32; 2]> = None;

    for step in 0..points.len() {
        let point = points[(offset + step) % points.len()];
        let position = [point.0, point.1];
        match (point.2, control) {
            (true, None) => {
                out.push(position);
                current = position;
            }
            (true, Some(ctrl)) => {
                push_quadratic(&mut out, current, ctrl, position);
                current = position;
                control = None;
            }
            (false, None) => control = Some(position),
            (false, Some(ctrl)) => {
                // Two controls in a row: an on-curve point is implied halfway.
                let implied = [(ctrl[0] + position[0]) / 2.0, (ctrl[1] + position[1]) / 2.0];
                push_quadratic(&mut out, current, ctrl, implied);
                current = implied;
                control = Some(position);
            }
        }
    }
    if let Some(ctrl) = control {
        push_quadratic(&mut out, current, ctrl, start);
    }
    match out.len() >= 3 {
        true => Some(out),
        false => None,
    }
}

fn push_quadratic(out: &mut Vec<[f32; 2]>, from: [f32; 2], control: [f32; 2], to: [f32; 2]) {
    for step in 1..=CURVE_STEPS {
        let t = step as f32 / CURVE_STEPS as f32;
        let u = 1.0 - t;
        out.push([
            u * u * from[0] + 2.0 * u * t * control[0] + t * t * to[0],
            u * u * from[1] + 2.0 * u * t * control[1] + t * t * to[1],
        ]);
    }
}

fn apply(m: [f32; 6], x: f32, y: f32) -> [f32; 2] {
    [m[0] * x + m[2] * y + m[4], m[1] * x + m[3] * y + m[5]]
}

fn compose(outer: [f32; 6], inner: [f32; 6]) -> [f32; 6] {
    [
        inner[0] * outer[0] + inner[1] * outer[2],
        inner[0] * outer[1] + inner[1] * outer[3],
        inner[2] * outer[0] + inner[3] * outer[2],
        inner[2] * outer[1] + inner[3] * outer[3],
        inner[4] * outer[0] + inner[5] * outer[2] + outer[4],
        inner[4] * outer[1] + inner[5] * outer[3] + outer[5],
    ]
}

// ============================================================================
// Big-endian readers
// ============================================================================

fn be_u16(data: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_be_bytes(data.get(at..at + 2)?.try_into().ok()?))
}

fn be_i16(data: &[u8], at: usize) -> Option<i16> {
    be_u16(data, at).map(|value| value as i16)
}

fn be_u32(data: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_be_bytes(data.get(at..at + 4)?.try_into().ok()?))
}

/// The 2.14 fixed-point format composite glyphs scale with.
fn be_f2dot14(data: &[u8], at: usize) -> Option<f32> {
    be_i16(data, at).map(|value| value as f32 / 16384.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A real font from the system, so the parser is tested against what
    /// shipped rather than against a fixture written to match it. Skipped
    /// where none is present.
    fn system_font() -> Option<Vec<u8>> {
        for path in [
            "C:/Windows/Fonts/Candara.ttf",
            "C:/Windows/Fonts/arial.ttf",
            "C:/Windows/Fonts/times.ttf",
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            "/System/Library/Fonts/Helvetica.ttc",
        ] {
            if let Ok(bytes) = std::fs::read(path) {
                return Some(bytes);
            }
        }
        None
    }

    #[test]
    fn rubbish_is_refused_rather_than_half_parsed() {
        assert!(TrueTypeFont::parse(b"not a font").is_none());
        assert!(TrueTypeFont::parse(&[0u8; 64]).is_none());
    }

    #[test]
    fn a_system_font_parses() {
        let Some(bytes) = system_font() else {
            eprintln!("skipping: no system TrueType font");
            return;
        };
        let font = TrueTypeFont::parse(&bytes).expect("a system font should parse");
        assert!(font.glyph_count() > 100, "glyphs: {}", font.glyph_count());
        assert!(
            (500.0..=8192.0).contains(&font.units_per_em),
            "units per em: {}",
            font.units_per_em
        );
    }

    #[test]
    fn letters_have_believable_outlines() {
        let Some(bytes) = system_font() else {
            eprintln!("skipping: no system TrueType font");
            return;
        };
        let font = TrueTypeFont::parse(&bytes).expect("parses");
        let index = font
            .glyph_index(b'A' as u32, Some('A'))
            .expect("a font should map A");
        let glyph = font.outline(index).expect("A has an outline");
        assert!(!glyph.contours.is_empty(), "A should have contours");

        let points: Vec<[f32; 2]> = glyph.contours.iter().flatten().copied().collect();
        let min_y = points.iter().map(|p| p[1]).fold(f32::MAX, f32::min);
        let max_y = points.iter().map(|p| p[1]).fold(f32::MIN, f32::max);
        let min_x = points.iter().map(|p| p[0]).fold(f32::MAX, f32::min);
        let max_x = points.iter().map(|p| p[0]).fold(f32::MIN, f32::max);

        // A capital sits on the baseline and reaches cap height, in a design
        // grid whose size the font declares.
        assert!(min_y > -0.1 * font.units_per_em as f32, "A dips: {min_y}");
        assert!(
            max_y > 0.4 * font.units_per_em as f32,
            "A is too short: {max_y} of {}",
            font.units_per_em
        );
        assert!(
            max_x - min_x > 0.2 * font.units_per_em as f32,
            "A is too narrow"
        );
        assert!(glyph.advance > 0.0, "A should have an advance");
    }

    /// `o` has a counter, so two contours. A parser that drops the second
    /// produces a filled blob, which is the failure that matters here.
    #[test]
    fn a_counter_is_its_own_contour() {
        let Some(bytes) = system_font() else {
            eprintln!("skipping: no system TrueType font");
            return;
        };
        let font = TrueTypeFont::parse(&bytes).expect("parses");
        let Some(index) = font.glyph_index(b'o' as u32, Some('o')) else {
            eprintln!("skipping: no o in this font");
            return;
        };
        let glyph = font.outline(index).expect("o has an outline");
        assert_eq!(glyph.contours.len(), 2, "an o is a ring: outer and inner");
    }

    /// Curves must be flattened into points, or round letters come out as
    /// polygons with four corners.
    #[test]
    fn curves_are_flattened() {
        let Some(bytes) = system_font() else {
            eprintln!("skipping: no system TrueType font");
            return;
        };
        let font = TrueTypeFont::parse(&bytes).expect("parses");
        let Some(index) = font.glyph_index(b'O' as u32, Some('O')) else {
            return;
        };
        let glyph = font.outline(index).expect("O");
        let points: usize = glyph.contours.iter().map(Vec::len).sum();
        assert!(
            points > 40,
            "a round letter should be smooth: {points} points"
        );
    }

    /// Every glyph in a real font must terminate and stay in bounds. A font is
    /// untrusted input like any other file.
    #[test]
    fn every_glyph_terminates() {
        let Some(bytes) = system_font() else {
            eprintln!("skipping: no system TrueType font");
            return;
        };
        let font = TrueTypeFont::parse(&bytes).expect("parses");
        let mut drawn = 0;
        for index in 0..font.glyph_count().min(600) {
            if let Some(glyph) = font.outline(index as u16)
                && !glyph.contours.is_empty()
            {
                drawn += 1;
            }
        }
        assert!(drawn > 50, "expected most glyphs to draw, got {drawn}");
    }
}
