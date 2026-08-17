// SPDX-License-Identifier: AGPL-3.0-or-later
//! CFF font programs: `/FontFile3` streams, read into glyph outlines.
//!
//! CFF is what Type 1 became. A modern producer subsetting a PostScript
//! typeface emits `/Subtype /Type1C`, and an OpenType font with PostScript
//! outlines carries the same structure inside a `CFF ` table. Between this and
//! [`super::truetype`], the embedded fonts a PDF is likely to carry are
//! covered.
//!
//! The container is a stack of INDEX structures -- length-prefixed arrays --
//! and DICTs whose operands precede their operators. The outlines are Type 2
//! charstrings: the same idea as Type 1 but a denser encoding, with the
//! width folded into the first stack-clearing operator and hints carried in
//! bitmasks.
//!
//! Outlines come out in font units, flattened, as the other readers do.

use std::collections::HashMap;

use super::Glyph;

/// Curve flattening, matching the other font readers.
const CURVE_STEPS: usize = 8;

/// How deep `callsubr` may nest before this is a damaged font rather than a
/// clever one.
const MAX_SUBR_DEPTH: usize = 32;

/// A parsed CFF font.
#[derive(Debug, Clone)]
pub struct CffFont {
    charstrings: Vec<Vec<u8>>,
    global_subrs: Vec<Vec<u8>>,
    local_subrs: Vec<Vec<u8>>,
    /// Glyph name to glyph index, from the charset.
    by_name: HashMap<String, u16>,
    /// Character code to glyph index, from the font's own encoding.
    by_code: HashMap<u8, u16>,
    /// Usually a 1000-unit em, but a CFF may state otherwise and then every
    /// outline is the wrong size.
    pub font_matrix: [f64; 6],
    default_width: f32,
    nominal_width: f32,
}

impl CffFont {
    /// Parse a `/FontFile3` stream, or the contents of a `CFF ` table.
    ///
    /// Returns `None` rather than erroring: a font that will not parse should
    /// cost the caller a fallback, not a failed page.
    pub fn parse(data: &[u8]) -> Option<CffFont> {
        let header_size = *data.get(2)? as usize;
        let mut at = header_size;

        let (_names, next) = read_index(data, at)?;
        at = next;
        let (top_dicts, next) = read_index(data, at)?;
        at = next;
        let (strings, next) = read_index(data, at)?;
        at = next;
        let (global_subrs, _) = read_index(data, at)?;

        let top = parse_dict(top_dicts.first()?);

        let charstrings_at = *top.get(&17u16)?.first()? as usize;
        let (charstrings, _) = read_index(data, charstrings_at)?;
        if charstrings.is_empty() {
            return None;
        }

        // The private DICT carries the local subroutines, and its `Subrs`
        // offset is relative to the private DICT itself rather than the file.
        let mut local_subrs = Vec::new();
        let mut default_width = 0.0f32;
        let mut nominal_width = 0.0f32;
        if let Some(private) = top.get(&18u16)
            && private.len() >= 2
        {
            let size = private[0] as usize;
            let offset = private[1] as usize;
            if let Some(bytes) = data.get(offset..offset.saturating_add(size)) {
                let dict = parse_dict(bytes);
                default_width = dict
                    .get(&20u16)
                    .and_then(|v| v.first())
                    .copied()
                    .unwrap_or(0.0) as f32;
                nominal_width = dict
                    .get(&21u16)
                    .and_then(|v| v.first())
                    .copied()
                    .unwrap_or(0.0) as f32;
                if let Some(subrs_at) = dict.get(&19u16).and_then(|v| v.first()) {
                    let absolute = offset.saturating_add(*subrs_at as usize);
                    if let Some((subrs, _)) = read_index(data, absolute) {
                        local_subrs = subrs;
                    }
                }
            }
        }

        let font_matrix = top
            .get(&0x0C07)
            .filter(|values| values.len() == 6)
            .map(|v| [v[0], v[1], v[2], v[3], v[4], v[5]])
            .unwrap_or([0.001, 0.0, 0.0, 0.001, 0.0, 0.0]);

        let by_name = read_charset(
            data,
            top.get(&15u16)
                .and_then(|v| v.first())
                .copied()
                .unwrap_or(0.0) as usize,
            charstrings.len(),
            &strings,
        );
        let by_code = read_encoding(
            data,
            top.get(&16u16)
                .and_then(|v| v.first())
                .copied()
                .unwrap_or(0.0) as usize,
            &by_name,
        );

        Some(CffFont {
            charstrings,
            global_subrs,
            local_subrs,
            by_name,
            by_code,
            font_matrix,
            default_width,
            nominal_width,
        })
    }

    pub fn glyph_count(&self) -> usize {
        self.charstrings.len()
    }

    /// The glyph index a name selects.
    pub fn index_for_name(&self, name: &str) -> Option<u16> {
        self.by_name.get(name).copied()
    }

    /// The glyph index the font's own encoding gives a code.
    pub fn index_for_code(&self, code: u8) -> Option<u16> {
        self.by_code.get(&code).copied()
    }

    pub fn glyph_names(&self) -> impl Iterator<Item = String> + '_ {
        self.by_name.keys().cloned()
    }

    /// The outline of a glyph, by index.
    pub fn outline(&self, index: u16) -> Option<Glyph> {
        let charstring = self.charstrings.get(index as usize)?;
        let mut state = Type2 {
            font: self,
            stack: Vec::new(),
            contours: Vec::new(),
            current: Vec::new(),
            x: 0.0,
            y: 0.0,
            stems: 0,
            width: None,
            transient: [0.0; 32],
            done: false,
        };
        state.run(charstring, 0);
        state.close();
        let width = state
            .width
            .map(|extra| self.nominal_width + extra)
            .unwrap_or(self.default_width);
        Some(Glyph {
            contours: state.contours,
            advance: width,
        })
    }
}

// ============================================================================
// Container
// ============================================================================

/// An INDEX: a count, an offset size, offsets, then the data they point into.
fn read_index(data: &[u8], at: usize) -> Option<(Vec<Vec<u8>>, usize)> {
    let count = u16::from_be_bytes(data.get(at..at + 2)?.try_into().ok()?) as usize;
    if count == 0 {
        return Some((Vec::new(), at + 2));
    }
    let offset_size = *data.get(at + 2)? as usize;
    if !(1..=4).contains(&offset_size) {
        return None;
    }
    let offsets_at = at + 3;
    let read_offset = |index: usize| -> Option<usize> {
        let from = offsets_at + index * offset_size;
        let bytes = data.get(from..from + offset_size)?;
        Some(bytes.iter().fold(0usize, |acc, &b| (acc << 8) | b as usize))
    };

    let data_at = offsets_at + (count + 1) * offset_size - 1;
    let mut out = Vec::with_capacity(count.min(65_536));
    for index in 0..count {
        let start = read_offset(index)?;
        let end = read_offset(index + 1)?;
        if end < start {
            return None;
        }
        let from = data_at.checked_add(start)?;
        let to = data_at.checked_add(end)?;
        out.push(data.get(from..to)?.to_vec());
    }
    Some((out, data_at + read_offset(count)?))
}

/// A DICT: operands then an operator, repeatedly. Two-byte operators are
/// prefixed with 12 and stored here as `0x0C00 | second`.
fn parse_dict(data: &[u8]) -> HashMap<u16, Vec<f64>> {
    let mut out = HashMap::new();
    let mut operands: Vec<f64> = Vec::new();
    let mut at = 0;
    while at < data.len() {
        let byte = data[at];
        match byte {
            0..=21 => {
                let key = match byte {
                    12 => {
                        at += 1;
                        0x0C00u16 | *data.get(at).unwrap_or(&0) as u16
                    }
                    _ => byte as u16,
                };
                out.insert(key, std::mem::take(&mut operands));
                at += 1;
            }
            28 => {
                let value = i16::from_be_bytes([
                    *data.get(at + 1).unwrap_or(&0),
                    *data.get(at + 2).unwrap_or(&0),
                ]);
                operands.push(value as f64);
                at += 3;
            }
            29 => {
                let value = i32::from_be_bytes([
                    *data.get(at + 1).unwrap_or(&0),
                    *data.get(at + 2).unwrap_or(&0),
                    *data.get(at + 3).unwrap_or(&0),
                    *data.get(at + 4).unwrap_or(&0),
                ]);
                operands.push(value as f64);
                at += 5;
            }
            30 => {
                // A real, encoded as packed nibbles.
                let (value, next) = parse_real(data, at + 1);
                operands.push(value);
                at = next;
            }
            32..=246 => {
                operands.push(byte as f64 - 139.0);
                at += 1;
            }
            247..=250 => {
                let next = *data.get(at + 1).unwrap_or(&0) as f64;
                operands.push((byte as f64 - 247.0) * 256.0 + next + 108.0);
                at += 2;
            }
            251..=254 => {
                let next = *data.get(at + 1).unwrap_or(&0) as f64;
                operands.push(-((byte as f64 - 251.0) * 256.0) - next - 108.0);
                at += 2;
            }
            _ => at += 1,
        }
        // A DICT with an absurd operand run is damaged, not expressive.
        if operands.len() > 48 {
            operands.clear();
        }
    }
    out
}

fn parse_real(data: &[u8], mut at: usize) -> (f64, usize) {
    let mut text = String::new();
    'outer: while at < data.len() {
        let byte = data[at];
        at += 1;
        for nibble in [byte >> 4, byte & 0x0F] {
            match nibble {
                0..=9 => text.push((b'0' + nibble) as char),
                0x0a => text.push('.'),
                0x0b => text.push('E'),
                0x0c => text.push_str("E-"),
                0x0e => text.push('-'),
                0x0f => break 'outer,
                _ => {}
            }
        }
    }
    (text.parse().unwrap_or(0.0), at)
}

/// The charset: glyph index to name, via string ids.
fn read_charset(
    data: &[u8],
    offset: usize,
    glyph_count: usize,
    strings: &[Vec<u8>],
) -> HashMap<String, u16> {
    let mut out = HashMap::new();
    let name_of = |sid: usize| -> Option<String> {
        match sid < STANDARD_STRINGS.len() {
            true => Some(STANDARD_STRINGS[sid].to_string()),
            false => strings
                .get(sid - STANDARD_STRINGS.len())
                .and_then(|bytes| String::from_utf8(bytes.clone()).ok()),
        }
    };
    // Glyph 0 is always .notdef and is not listed.
    out.insert(".notdef".to_string(), 0);

    // Offsets 0, 1 and 2 name the predefined charsets rather than a table.
    if offset <= 2 {
        for gid in 1..glyph_count {
            if let Some(name) = name_of(gid) {
                out.insert(name, gid as u16);
            }
        }
        return out;
    }

    let Some(&format) = data.get(offset) else {
        return out;
    };
    let mut gid = 1usize;
    let mut at = offset + 1;
    match format {
        0 => {
            while gid < glyph_count {
                let Some(bytes) = data.get(at..at + 2) else {
                    break;
                };
                let sid = u16::from_be_bytes([bytes[0], bytes[1]]) as usize;
                if let Some(name) = name_of(sid) {
                    out.insert(name, gid as u16);
                }
                gid += 1;
                at += 2;
            }
        }
        1 | 2 => {
            let run_bytes = if format == 1 { 1 } else { 2 };
            while gid < glyph_count {
                let Some(first) = data.get(at..at + 2) else {
                    break;
                };
                let sid = u16::from_be_bytes([first[0], first[1]]) as usize;
                let left = match run_bytes {
                    1 => *data.get(at + 2).unwrap_or(&0) as usize,
                    _ => match data.get(at + 2..at + 4) {
                        Some(b) => u16::from_be_bytes([b[0], b[1]]) as usize,
                        None => break,
                    },
                };
                for step in 0..=left {
                    if gid >= glyph_count {
                        break;
                    }
                    if let Some(name) = name_of(sid + step) {
                        out.insert(name, gid as u16);
                    }
                    gid += 1;
                }
                at += 2 + run_bytes;
            }
        }
        _ => {}
    }
    out
}

/// The font's own encoding: character code to glyph index.
fn read_encoding(data: &[u8], offset: usize, by_name: &HashMap<String, u16>) -> HashMap<u8, u16> {
    let mut out = HashMap::new();
    // 0 is the standard encoding, 1 the expert one. Standard is worth
    // honouring because a font used as it ships relies on it.
    if offset <= 1 {
        for (code, name) in STANDARD_ENCODING.iter().enumerate() {
            if !name.is_empty()
                && let Some(&gid) = by_name.get(*name)
            {
                out.insert(code as u8, gid);
            }
        }
        return out;
    }
    let Some(&format) = data.get(offset) else {
        return out;
    };
    match format & 0x7F {
        0 => {
            let count = *data.get(offset + 1).unwrap_or(&0) as usize;
            for index in 0..count {
                if let Some(&code) = data.get(offset + 2 + index) {
                    out.insert(code, index as u16 + 1);
                }
            }
        }
        1 => {
            let ranges = *data.get(offset + 1).unwrap_or(&0) as usize;
            let mut gid = 1u16;
            for range in 0..ranges {
                let at = offset + 2 + range * 2;
                let (Some(&first), Some(&left)) = (data.get(at), data.get(at + 1)) else {
                    break;
                };
                for step in 0..=left as u16 {
                    let code = first as u16 + step;
                    if code <= 255 {
                        out.insert(code as u8, gid);
                    }
                    gid += 1;
                }
            }
        }
        _ => {}
    }
    out
}

// ============================================================================
// Type 2 charstrings
// ============================================================================

struct Type2<'a> {
    font: &'a CffFont,
    stack: Vec<f32>,
    contours: Vec<Vec<[f32; 2]>>,
    current: Vec<[f32; 2]>,
    x: f32,
    y: f32,
    stems: usize,
    /// The optional width, taken from the first stack-clearing operator.
    width: Option<f32>,
    transient: [f32; 32],
    done: bool,
}

/// The bias a subroutine index is offset by, which depends on how many there
/// are. Getting this wrong calls the wrong subroutine, which draws a different
/// letter.
fn bias(count: usize) -> i32 {
    match count {
        0..=1238 => 107,
        1239..=33899 => 1131,
        _ => 32768,
    }
}

impl Type2<'_> {
    fn close(&mut self) {
        if self.current.len() > 2 {
            let contour = std::mem::take(&mut self.current);
            self.contours.push(contour);
        } else {
            self.current.clear();
        }
    }

    fn move_to(&mut self, x: f32, y: f32) {
        self.close();
        self.current.push([x, y]);
        self.x = x;
        self.y = y;
    }

    fn line_to(&mut self, x: f32, y: f32) {
        if self.current.is_empty() {
            self.current.push([self.x, self.y]);
        }
        self.current.push([x, y]);
        self.x = x;
        self.y = y;
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x3: f32, y3: f32) {
        if self.current.is_empty() {
            self.current.push([self.x, self.y]);
        }
        let (x0, y0) = (self.x, self.y);
        for step in 1..=CURVE_STEPS {
            let t = step as f32 / CURVE_STEPS as f32;
            let u = 1.0 - t;
            self.current.push([
                u * u * u * x0 + 3.0 * u * u * t * x1 + 3.0 * u * t * t * x2 + t * t * t * x3,
                u * u * u * y0 + 3.0 * u * u * t * y1 + 3.0 * u * t * t * y2 + t * t * t * y3,
            ]);
        }
        self.x = x3;
        self.y = y3;
    }

    /// Take the width off the front of the stack if this operator is the first
    /// stack-clearing one and the count is odd for it.
    fn take_width(&mut self, even: bool) {
        if self.width.is_some() {
            return;
        }
        let odd = self.stack.len() % 2 == 1;
        let extra = match even {
            true => odd,
            false => !odd,
        };
        match extra && !self.stack.is_empty() {
            true => self.width = Some(self.stack.remove(0)),
            // Recording zero marks the width as decided, so a later operator
            // does not take an operand that belongs to it.
            false => self.width = Some(0.0),
        }
    }

    fn run(&mut self, code: &[u8], depth: usize) {
        if depth > MAX_SUBR_DEPTH || self.done {
            return;
        }
        let mut at = 0;
        while at < code.len() {
            let byte = code[at];
            at += 1;

            if byte >= 32 || byte == 28 {
                let value = match byte {
                    28 => {
                        let Some(bytes) = code.get(at..at + 2) else {
                            return;
                        };
                        at += 2;
                        i16::from_be_bytes([bytes[0], bytes[1]]) as f32
                    }
                    32..=246 => byte as f32 - 139.0,
                    247..=250 => {
                        let Some(&next) = code.get(at) else { return };
                        at += 1;
                        (byte as f32 - 247.0) * 256.0 + next as f32 + 108.0
                    }
                    251..=254 => {
                        let Some(&next) = code.get(at) else { return };
                        at += 1;
                        -((byte as f32 - 251.0) * 256.0) - next as f32 - 108.0
                    }
                    _ => {
                        // 255: a 16.16 fixed-point number.
                        let Some(bytes) = code.get(at..at + 4) else {
                            return;
                        };
                        at += 4;
                        i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as f32
                            / 65536.0
                    }
                };
                if self.stack.len() < 48 {
                    self.stack.push(value);
                }
                continue;
            }

            match byte {
                1 | 3 | 18 | 23 => {
                    // Stem hints: only the count matters, for the mask width.
                    self.take_width(true);
                    self.stems += self.stack.len() / 2;
                    self.stack.clear();
                }
                19 | 20 => {
                    // hintmask: implies any pending stems, then skips a mask.
                    self.take_width(true);
                    self.stems += self.stack.len() / 2;
                    self.stack.clear();
                    at += self.stems.div_ceil(8);
                }
                21 => {
                    self.take_width(true);
                    let dy = self.stack.pop().unwrap_or(0.0);
                    let dx = self.stack.pop().unwrap_or(0.0);
                    let (x, y) = (self.x + dx, self.y + dy);
                    self.move_to(x, y);
                    self.stack.clear();
                }
                22 => {
                    self.take_width(false);
                    let dx = self.stack.pop().unwrap_or(0.0);
                    let (x, y) = (self.x + dx, self.y);
                    self.move_to(x, y);
                    self.stack.clear();
                }
                4 => {
                    self.take_width(false);
                    let dy = self.stack.pop().unwrap_or(0.0);
                    let (x, y) = (self.x, self.y + dy);
                    self.move_to(x, y);
                    self.stack.clear();
                }
                5 => {
                    let values = std::mem::take(&mut self.stack);
                    for pair in values.chunks(2) {
                        if pair.len() < 2 {
                            break;
                        }
                        let (x, y) = (self.x + pair[0], self.y + pair[1]);
                        self.line_to(x, y);
                    }
                }
                6 | 7 => {
                    // hlineto / vlineto: alternating axes.
                    let values = std::mem::take(&mut self.stack);
                    let mut horizontal = byte == 6;
                    for delta in values {
                        let (x, y) = match horizontal {
                            true => (self.x + delta, self.y),
                            false => (self.x, self.y + delta),
                        };
                        self.line_to(x, y);
                        horizontal = !horizontal;
                    }
                }
                8 => {
                    let values = std::mem::take(&mut self.stack);
                    for six in values.chunks(6) {
                        if six.len() < 6 {
                            break;
                        }
                        self.relative_curve(six[0], six[1], six[2], six[3], six[4], six[5]);
                    }
                }
                24 => {
                    // rcurveline: curves then one line.
                    let values = std::mem::take(&mut self.stack);
                    let curves = (values.len().saturating_sub(2)) / 6;
                    for index in 0..curves {
                        let six = &values[index * 6..index * 6 + 6];
                        self.relative_curve(six[0], six[1], six[2], six[3], six[4], six[5]);
                    }
                    if let Some(rest) = values.get(curves * 6..curves * 6 + 2) {
                        let (x, y) = (self.x + rest[0], self.y + rest[1]);
                        self.line_to(x, y);
                    }
                }
                25 => {
                    // rlinecurve: lines then one curve.
                    let values = std::mem::take(&mut self.stack);
                    let lines = (values.len().saturating_sub(6)) / 2;
                    for index in 0..lines {
                        let pair = &values[index * 2..index * 2 + 2];
                        let (x, y) = (self.x + pair[0], self.y + pair[1]);
                        self.line_to(x, y);
                    }
                    if let Some(six) = values.get(lines * 2..lines * 2 + 6) {
                        self.relative_curve(six[0], six[1], six[2], six[3], six[4], six[5]);
                    }
                }
                26 | 27 => {
                    // vvcurveto / hhcurveto, with an optional leading delta on
                    // the other axis.
                    let mut values = std::mem::take(&mut self.stack);
                    let mut lead = 0.0;
                    if values.len() % 4 == 1 {
                        lead = values.remove(0);
                    }
                    for four in values.chunks(4) {
                        if four.len() < 4 {
                            break;
                        }
                        match byte == 26 {
                            true => {
                                self.relative_curve(lead, four[0], four[1], four[2], 0.0, four[3])
                            }
                            false => {
                                self.relative_curve(four[0], lead, four[1], four[2], four[3], 0.0)
                            }
                        }
                        lead = 0.0;
                    }
                }
                30 | 31 => {
                    // vhcurveto / hvcurveto: curves alternating start axis,
                    // with an optional final delta on the closing axis.
                    let values = std::mem::take(&mut self.stack);
                    let mut horizontal = byte == 31;
                    let mut index = 0;
                    while index + 4 <= values.len() {
                        let last = index + 8 > values.len();
                        let extra = match last && values.len() - index == 5 {
                            true => values[index + 4],
                            false => 0.0,
                        };
                        let four = &values[index..index + 4];
                        match horizontal {
                            true => {
                                self.relative_curve(four[0], 0.0, four[1], four[2], extra, four[3])
                            }
                            false => {
                                self.relative_curve(0.0, four[0], four[1], four[2], four[3], extra)
                            }
                        }
                        horizontal = !horizontal;
                        index += 4;
                    }
                }
                10 | 29 => {
                    // callsubr / callgsubr
                    let Some(raw) = self.stack.pop() else {
                        continue;
                    };
                    let subrs = match byte == 10 {
                        true => &self.font.local_subrs,
                        false => &self.font.global_subrs,
                    };
                    let index = raw as i32 + bias(subrs.len());
                    if index >= 0
                        && let Some(subr) = subrs.get(index as usize)
                    {
                        let subr = subr.clone();
                        self.run(&subr, depth + 1);
                    }
                }
                11 => return,
                14 => {
                    self.take_width(true);
                    self.close();
                    self.done = true;
                    return;
                }
                12 => {
                    let Some(&second) = code.get(at) else { return };
                    at += 1;
                    self.escape(second);
                }
                _ => self.stack.clear(),
            }
        }
    }

    fn escape(&mut self, op: u8) {
        match op {
            35 => {
                // flex: two curves, then a flex depth that does not affect the
                // outline at this resolution.
                let values = std::mem::take(&mut self.stack);
                if values.len() >= 12 {
                    self.relative_curve(
                        values[0], values[1], values[2], values[3], values[4], values[5],
                    );
                    self.relative_curve(
                        values[6], values[7], values[8], values[9], values[10], values[11],
                    );
                }
            }
            34 => {
                // hflex
                let v = std::mem::take(&mut self.stack);
                if v.len() >= 7 {
                    let y = self.y;
                    self.relative_curve(v[0], 0.0, v[1], v[2], v[3], 0.0);
                    self.relative_curve(v[4], 0.0, v[5], y - self.y, v[6], 0.0);
                }
            }
            36 => {
                // hflex1
                let v = std::mem::take(&mut self.stack);
                if v.len() >= 9 {
                    let y = self.y;
                    self.relative_curve(v[0], v[1], v[2], v[3], v[4], 0.0);
                    self.relative_curve(v[5], 0.0, v[6], v[7], v[8], y - self.y);
                }
            }
            37 => {
                // flex1: the final point returns to the start of the pair.
                let v = std::mem::take(&mut self.stack);
                if v.len() >= 11 {
                    let (x0, y0) = (self.x, self.y);
                    let dx: f32 = v[0] + v[2] + v[4] + v[6] + v[8];
                    let dy: f32 = v[1] + v[3] + v[5] + v[7] + v[9];
                    self.relative_curve(v[0], v[1], v[2], v[3], v[4], v[5]);
                    let x1 = self.x + v[6];
                    let y1 = self.y + v[7];
                    let x2 = x1 + v[8];
                    let y2 = y1 + v[9];
                    let (x3, y3) = match dx.abs() > dy.abs() {
                        true => (x2 + v[10], y0),
                        false => (x0, y2 + v[10]),
                    };
                    self.curve_to(x1, y1, x2, y2, x3, y3);
                }
            }
            3 | 4 | 5 | 9 | 10 | 11 | 12 | 14 | 15 | 18 | 21 | 22 | 23 | 24 | 26 | 27 | 28 | 29
            | 30 => {
                // The arithmetic and storage operators. Fonts in the wild do
                // not use them for outlines; clearing keeps a stray one from
                // corrupting the operand stack.
                let _ = &self.transient;
                self.stack.clear();
            }
            _ => self.stack.clear(),
        }
    }

    fn relative_curve(&mut self, dx1: f32, dy1: f32, dx2: f32, dy2: f32, dx3: f32, dy3: f32) {
        let x1 = self.x + dx1;
        let y1 = self.y + dy1;
        let x2 = x1 + dx2;
        let y2 = y1 + dy2;
        let x3 = x2 + dx3;
        let y3 = y2 + dy3;
        self.curve_to(x1, y1, x2, y2, x3, y3);
    }
}

// ============================================================================
// Predefined tables
// ============================================================================

/// The 391 standard strings, which a charset refers to by index before any of
/// the font's own strings begin.
const STANDARD_STRINGS: [&str; 391] = [
    ".notdef",
    "space",
    "exclam",
    "quotedbl",
    "numbersign",
    "dollar",
    "percent",
    "ampersand",
    "quoteright",
    "parenleft",
    "parenright",
    "asterisk",
    "plus",
    "comma",
    "hyphen",
    "period",
    "slash",
    "zero",
    "one",
    "two",
    "three",
    "four",
    "five",
    "six",
    "seven",
    "eight",
    "nine",
    "colon",
    "semicolon",
    "less",
    "equal",
    "greater",
    "question",
    "at",
    "A",
    "B",
    "C",
    "D",
    "E",
    "F",
    "G",
    "H",
    "I",
    "J",
    "K",
    "L",
    "M",
    "N",
    "O",
    "P",
    "Q",
    "R",
    "S",
    "T",
    "U",
    "V",
    "W",
    "X",
    "Y",
    "Z",
    "bracketleft",
    "backslash",
    "bracketright",
    "asciicircum",
    "underscore",
    "quoteleft",
    "a",
    "b",
    "c",
    "d",
    "e",
    "f",
    "g",
    "h",
    "i",
    "j",
    "k",
    "l",
    "m",
    "n",
    "o",
    "p",
    "q",
    "r",
    "s",
    "t",
    "u",
    "v",
    "w",
    "x",
    "y",
    "z",
    "braceleft",
    "bar",
    "braceright",
    "asciitilde",
    "exclamdown",
    "cent",
    "sterling",
    "fraction",
    "yen",
    "florin",
    "section",
    "currency",
    "quotesingle",
    "quotedblleft",
    "guillemotleft",
    "guilsinglleft",
    "guilsinglright",
    "fi",
    "fl",
    "endash",
    "dagger",
    "daggerdbl",
    "periodcentered",
    "paragraph",
    "bullet",
    "quotesinglbase",
    "quotedblbase",
    "quotedblright",
    "guillemotright",
    "ellipsis",
    "perthousand",
    "questiondown",
    "grave",
    "acute",
    "circumflex",
    "tilde",
    "macron",
    "breve",
    "dotaccent",
    "dieresis",
    "ring",
    "cedilla",
    "hungarumlaut",
    "ogonek",
    "caron",
    "emdash",
    "AE",
    "ordfeminine",
    "Lslash",
    "Oslash",
    "OE",
    "ordmasculine",
    "ae",
    "dotlessi",
    "lslash",
    "oslash",
    "oe",
    "germandbls",
    "onesuperior",
    "logicalnot",
    "mu",
    "trademark",
    "Eth",
    "onehalf",
    "plusminus",
    "Thorn",
    "onequarter",
    "divide",
    "brokenbar",
    "degree",
    "thorn",
    "threequarters",
    "twosuperior",
    "registered",
    "minus",
    "eth",
    "multiply",
    "threesuperior",
    "copyright",
    "Aacute",
    "Acircumflex",
    "Adieresis",
    "Agrave",
    "Aring",
    "Atilde",
    "Ccedilla",
    "Eacute",
    "Ecircumflex",
    "Edieresis",
    "Egrave",
    "Iacute",
    "Icircumflex",
    "Idieresis",
    "Igrave",
    "Ntilde",
    "Oacute",
    "Ocircumflex",
    "Odieresis",
    "Ograve",
    "Otilde",
    "Scaron",
    "Uacute",
    "Ucircumflex",
    "Udieresis",
    "Ugrave",
    "Yacute",
    "Ydieresis",
    "Zcaron",
    "aacute",
    "acircumflex",
    "adieresis",
    "agrave",
    "aring",
    "atilde",
    "ccedilla",
    "eacute",
    "ecircumflex",
    "edieresis",
    "egrave",
    "iacute",
    "icircumflex",
    "idieresis",
    "igrave",
    "ntilde",
    "oacute",
    "ocircumflex",
    "odieresis",
    "ograve",
    "otilde",
    "scaron",
    "uacute",
    "ucircumflex",
    "udieresis",
    "ugrave",
    "yacute",
    "ydieresis",
    "zcaron",
    "exclamsmall",
    "Hungarumlautsmall",
    "dollaroldstyle",
    "dollarsuperior",
    "ampersandsmall",
    "Acutesmall",
    "parenleftsuperior",
    "parenrightsuperior",
    "twodotenleader",
    "onedotenleader",
    "zerooldstyle",
    "oneoldstyle",
    "twooldstyle",
    "threeoldstyle",
    "fouroldstyle",
    "fiveoldstyle",
    "sixoldstyle",
    "sevenoldstyle",
    "eightoldstyle",
    "nineoldstyle",
    "commasuperior",
    "threequartersemdash",
    "periodsuperior",
    "questionsmall",
    "asuperior",
    "bsuperior",
    "centsuperior",
    "dsuperior",
    "esuperior",
    "isuperior",
    "lsuperior",
    "msuperior",
    "nsuperior",
    "osuperior",
    "rsuperior",
    "ssuperior",
    "tsuperior",
    "ff",
    "ffi",
    "ffl",
    "parenleftinferior",
    "parenrightinferior",
    "Circumflexsmall",
    "hyphensuperior",
    "Gravesmall",
    "Asmall",
    "Bsmall",
    "Csmall",
    "Dsmall",
    "Esmall",
    "Fsmall",
    "Gsmall",
    "Hsmall",
    "Ismall",
    "Jsmall",
    "Ksmall",
    "Lsmall",
    "Msmall",
    "Nsmall",
    "Osmall",
    "Psmall",
    "Qsmall",
    "Rsmall",
    "Ssmall",
    "Tsmall",
    "Usmall",
    "Vsmall",
    "Wsmall",
    "Xsmall",
    "Ysmall",
    "Zsmall",
    "colonmonetary",
    "onefitted",
    "rupiah",
    "Tildesmall",
    "exclamdownsmall",
    "centoldstyle",
    "Lslashsmall",
    "Scaronsmall",
    "Zcaronsmall",
    "Dieresissmall",
    "Brevesmall",
    "Caronsmall",
    "Dotaccentsmall",
    "Macronsmall",
    "figuredash",
    "hypheninferior",
    "Ogoneksmall",
    "Ringsmall",
    "Cedillasmall",
    "questiondownsmall",
    "oneeighth",
    "threeeighths",
    "fiveeighths",
    "seveneighths",
    "onethird",
    "twothirds",
    "zerosuperior",
    "foursuperior",
    "fivesuperior",
    "sixsuperior",
    "sevensuperior",
    "eightsuperior",
    "ninesuperior",
    "zeroinferior",
    "oneinferior",
    "twoinferior",
    "threeinferior",
    "fourinferior",
    "fiveinferior",
    "sixinferior",
    "seveninferior",
    "eightinferior",
    "nineinferior",
    "centinferior",
    "dollarinferior",
    "periodinferior",
    "commainferior",
    "Agravesmall",
    "Aacutesmall",
    "Acircumflexsmall",
    "Atildesmall",
    "Adieresissmall",
    "Aringsmall",
    "AEsmall",
    "Ccedillasmall",
    "Egravesmall",
    "Eacutesmall",
    "Ecircumflexsmall",
    "Edieresissmall",
    "Igravesmall",
    "Iacutesmall",
    "Icircumflexsmall",
    "Idieresissmall",
    "Ethsmall",
    "Ntildesmall",
    "Ogravesmall",
    "Oacutesmall",
    "Ocircumflexsmall",
    "Otildesmall",
    "Odieresissmall",
    "OEsmall",
    "Oslashsmall",
    "Ugravesmall",
    "Uacutesmall",
    "Ucircumflexsmall",
    "Udieresissmall",
    "Yacutesmall",
    "Thornsmall",
    "Ydieresissmall",
    "001.000",
    "001.001",
    "001.002",
    "001.003",
    "Black",
    "Bold",
    "Book",
    "Light",
    "Medium",
    "Regular",
    "Roman",
    "Semibold",
];

/// StandardEncoding, as a code-to-name table. Empty entries are unmapped.
const STANDARD_ENCODING: [&str; 256] = {
    let mut table = [""; 256];
    table[32] = "space";
    table[33] = "exclam";
    table[34] = "quotedbl";
    table[35] = "numbersign";
    table[36] = "dollar";
    table[37] = "percent";
    table[38] = "ampersand";
    table[39] = "quoteright";
    table[40] = "parenleft";
    table[41] = "parenright";
    table[42] = "asterisk";
    table[43] = "plus";
    table[44] = "comma";
    table[45] = "hyphen";
    table[46] = "period";
    table[47] = "slash";
    table[48] = "zero";
    table[49] = "one";
    table[50] = "two";
    table[51] = "three";
    table[52] = "four";
    table[53] = "five";
    table[54] = "six";
    table[55] = "seven";
    table[56] = "eight";
    table[57] = "nine";
    table[58] = "colon";
    table[59] = "semicolon";
    table[60] = "less";
    table[61] = "equal";
    table[62] = "greater";
    table[63] = "question";
    table[64] = "at";
    table[65] = "A";
    table[66] = "B";
    table[67] = "C";
    table[68] = "D";
    table[69] = "E";
    table[70] = "F";
    table[71] = "G";
    table[72] = "H";
    table[73] = "I";
    table[74] = "J";
    table[75] = "K";
    table[76] = "L";
    table[77] = "M";
    table[78] = "N";
    table[79] = "O";
    table[80] = "P";
    table[81] = "Q";
    table[82] = "R";
    table[83] = "S";
    table[84] = "T";
    table[85] = "U";
    table[86] = "V";
    table[87] = "W";
    table[88] = "X";
    table[89] = "Y";
    table[90] = "Z";
    table[91] = "bracketleft";
    table[92] = "backslash";
    table[93] = "bracketright";
    table[94] = "asciicircum";
    table[95] = "underscore";
    table[96] = "quoteleft";
    table[97] = "a";
    table[98] = "b";
    table[99] = "c";
    table[100] = "d";
    table[101] = "e";
    table[102] = "f";
    table[103] = "g";
    table[104] = "h";
    table[105] = "i";
    table[106] = "j";
    table[107] = "k";
    table[108] = "l";
    table[109] = "m";
    table[110] = "n";
    table[111] = "o";
    table[112] = "p";
    table[113] = "q";
    table[114] = "r";
    table[115] = "s";
    table[116] = "t";
    table[117] = "u";
    table[118] = "v";
    table[119] = "w";
    table[120] = "x";
    table[121] = "y";
    table[122] = "z";
    table[123] = "braceleft";
    table[124] = "bar";
    table[125] = "braceright";
    table[126] = "asciitilde";
    table
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rubbish_is_refused_rather_than_half_parsed() {
        assert!(CffFont::parse(b"").is_none());
        assert!(CffFont::parse(&[0u8; 32]).is_none());
    }

    #[test]
    fn the_standard_strings_are_the_right_length() {
        // A short table shifts every glyph name past the gap, so the count is
        // worth asserting rather than trusting.
        assert_eq!(STANDARD_STRINGS.len(), 391);
        assert_eq!(STANDARD_STRINGS[0], ".notdef");
        assert_eq!(STANDARD_STRINGS[1], "space");
        assert_eq!(STANDARD_STRINGS[34], "A");
        assert_eq!(STANDARD_STRINGS[66], "a");
    }

    #[test]
    fn subroutine_bias_follows_the_count() {
        assert_eq!(bias(0), 107);
        assert_eq!(bias(1238), 107);
        assert_eq!(bias(1239), 1131);
        assert_eq!(bias(33899), 1131);
        assert_eq!(bias(33900), 32768);
    }

    /// Operands are the part of the encoding easiest to get subtly wrong.
    #[test]
    fn dict_operands_decode_across_every_range() {
        // 139 -> 0 with operator 1; then 247 0 -> 108 with operator 2.
        let dict = parse_dict(&[139, 1, 247, 0, 2, 251, 0, 3]);
        assert_eq!(dict.get(&1u16), Some(&vec![0.0]));
        assert_eq!(dict.get(&2u16), Some(&vec![108.0]));
        assert_eq!(dict.get(&3u16), Some(&vec![-108.0]));
    }

    #[test]
    fn a_two_byte_operator_is_keyed_separately() {
        // 12 7 is FontMatrix.
        let dict = parse_dict(&[139, 12, 7]);
        assert!(dict.contains_key(&0x0C07));
    }

    /// An INDEX is the container everything else sits in.
    #[test]
    fn an_index_round_trips() {
        // Two entries, "ab" and "cde", with one-byte offsets.
        let bytes = [0, 2, 1, 1, 3, 6, b'a', b'b', b'c', b'd', b'e'];
        let (entries, end) = read_index(&bytes, 0).expect("parses");
        assert_eq!(entries, vec![b"ab".to_vec(), b"cde".to_vec()]);
        assert_eq!(end, bytes.len());
    }

    #[test]
    fn an_empty_index_is_two_bytes() {
        let (entries, end) = read_index(&[0, 0], 0).expect("parses");
        assert!(entries.is_empty());
        assert_eq!(end, 2);
    }
}

#[cfg(test)]
pub(crate) mod built_font {
    use super::*;

    /// Assemble a minimal but real CFF font whose one glyph is a known
    /// rectangle.
    ///
    /// Hand-built rather than borrowed: no CFF font ships with this machine,
    /// and a fixture whose expected outline is known exactly tests the
    /// interpreter more sharply than a typeface whose true shape has to be
    /// eyeballed.
    pub(crate) fn build() -> Vec<u8> {
        // Offsets are written as 5-byte integers so the top DICT is a fixed
        // size and the layout can be computed in one pass.
        fn int32(value: usize) -> Vec<u8> {
            let mut out = vec![29u8];
            out.extend_from_slice(&(value as i32).to_be_bytes());
            out
        }
        fn index(entries: &[Vec<u8>]) -> Vec<u8> {
            if entries.is_empty() {
                return vec![0, 0];
            }
            let mut out = (entries.len() as u16).to_be_bytes().to_vec();
            out.push(1); // one-byte offsets
            let mut offset = 1usize;
            out.push(offset as u8);
            for entry in entries {
                offset += entry.len();
                out.push(offset as u8);
            }
            for entry in entries {
                out.extend_from_slice(entry);
            }
            out
        }

        let name_index = index(&[b"Test".to_vec()]);
        let string_index = index(&[]);
        let gsubr_index = index(&[]);

        // rmoveto 100 100; rlineto 200 0 0 200 -200 0; endchar
        let charstring: Vec<u8> = vec![
            239, 239, 21, // 100 100 rmoveto
            247, 92, 139, // 200 0
            139, 247, 92, // 0 200
            251, 92, 139, // -200 0
            5,   // rlineto
            14,  // endchar
        ];
        let charstrings_index = index(&[Vec::new(), charstring]);
        // Format 0, naming glyph 1 with SID 34, which is "A".
        let charset: Vec<u8> = vec![0, 0, 34];
        let private: Vec<u8> = Vec::new();

        // Top DICT: charset (15), CharStrings (17), Private (18).
        let top_len = 6 + 6 + 11;
        let header_len = 4;
        let top_index_len = 2 + 1 + 2 + top_len;
        let charset_at =
            header_len + name_index.len() + top_index_len + string_index.len() + gsubr_index.len();
        let charstrings_at = charset_at + charset.len();
        let private_at = charstrings_at + charstrings_index.len();

        let mut top: Vec<u8> = Vec::new();
        top.extend(int32(charset_at));
        top.push(15);
        top.extend(int32(charstrings_at));
        top.push(17);
        top.extend(int32(private.len()));
        top.extend(int32(private_at));
        top.push(18);
        assert_eq!(top.len(), top_len, "the top DICT must be the size assumed");

        let top_index = index(&[top]);
        assert_eq!(top_index.len(), top_index_len, "top INDEX size assumed");

        let mut out = vec![1u8, 0, header_len as u8, 1];
        out.extend(name_index);
        out.extend(top_index);
        out.extend(string_index);
        out.extend(gsubr_index);
        out.extend(charset);
        out.extend(charstrings_index);
        out.extend(private);
        out
    }

    #[test]
    fn a_built_font_parses_and_draws_its_glyph() {
        let font = CffFont::parse(&build()).expect("the built font should parse");
        assert_eq!(font.glyph_count(), 2, ".notdef plus one glyph");

        let index = font
            .index_for_name("A")
            .expect("the charset names glyph 1 A");
        assert_eq!(index, 1);

        let glyph = font.outline(index).expect("the glyph draws");
        assert_eq!(glyph.contours.len(), 1, "one closed contour");

        let points = &glyph.contours[0];
        let min_x = points.iter().map(|p| p[0]).fold(f32::MAX, f32::min);
        let max_x = points.iter().map(|p| p[0]).fold(f32::MIN, f32::max);
        let min_y = points.iter().map(|p| p[1]).fold(f32::MAX, f32::min);
        let max_y = points.iter().map(|p| p[1]).fold(f32::MIN, f32::max);

        // The charstring draws exactly this rectangle; anything else means the
        // operand decoding or an operator is wrong.
        assert_eq!(min_x, 100.0, "left edge");
        assert_eq!(max_x, 300.0, "right edge");
        assert_eq!(min_y, 100.0, "bottom edge");
        assert_eq!(max_y, 300.0, "top edge");
    }

    /// The font's own encoding must reach the glyph, since a PDF simple font
    /// with no `/Differences` relies on it.
    #[test]
    fn the_standard_encoding_reaches_the_glyph() {
        let font = CffFont::parse(&build()).expect("parses");
        assert_eq!(font.index_for_code(b'A'), Some(1));
        assert_eq!(font.index_for_code(b'B'), None, "only A is in this font");
    }
}
