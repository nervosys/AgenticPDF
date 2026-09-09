// SPDX-License-Identifier: AGPL-3.0-or-later
//! Type 1 font programs: `/FontFile` streams, decrypted and interpreted into
//! glyph outlines.
//!
//! Rendering text with a substitute font can only ever approximate a document.
//! Widths differ, so runs collide or have to be squeezed to fit, and the result
//! is visibly not what the author saw. The fix is to draw the glyphs the
//! document carries, which is what PDF.js and Okular do. This module turns an
//! embedded Type 1 program into outlines a painter can fill.
//!
//! Type 1 is still the common case for anything produced by TeX — a paper's
//! body font and every mathematical symbol in it arrive this way.
//!
//! The format is a PostScript program wrapped in two layers of a very weak
//! cipher: `eexec` over the private portion, and the same cipher again over
//! each charstring. Neither is a security measure and both are fixed-key, so
//! "decrypt" here means "decode".
//!
//! Outlines come out in font units — a 1000-unit em, as `/FontMatrix` almost
//! always says — with curves already flattened, because every consumer here
//! wants polygons.

use std::collections::HashMap;

/// One glyph's filled outline, in font units.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Glyph {
    /// Closed contours. A glyph with counters (an `o`) has more than one, and
    /// they are filled with the non-zero winding rule, as PostScript does.
    pub contours: Vec<Vec<[f32; 2]>>,
    /// The advance the font itself declares, in font units. The PDF's `/Widths`
    /// overrides this, and does so on purpose: a subset font can disagree.
    pub advance: f32,
}

/// A parsed Type 1 font program.
#[derive(Debug, Clone)]
pub struct Type1Font {
    charstrings: HashMap<String, Vec<u8>>,
    subrs: Vec<Vec<u8>>,
    /// The font's own encoding: code to glyph name. A PDF `/Differences` array
    /// overrides this, but many fonts are used exactly as they ship.
    encoding: HashMap<u8, String>,
    /// Usually `[0.001, 0, 0, 0.001, 0, 0]`, i.e. a 1000-unit em, but a TeX
    /// font can ship something else and then every outline is the wrong size.
    pub font_matrix: [f64; 6],
}

/// How deep `callsubr` may nest. The specification says ten; a damaged or
/// hostile font can say otherwise, and without a bound it recurses forever.
const MAX_SUBR_DEPTH: usize = 32;

/// Curve flattening. Type 1 curves are small and this runs per glyph per page,
/// so a fixed subdivision beats an adaptive one that has to measure first.
const CURVE_STEPS: usize = 8;

impl Type1Font {
    /// Parse a `/FontFile` stream. `clear_len` is `/Length1`, the cleartext
    /// portion; `encrypted_len` is `/Length2`.
    ///
    /// Returns `None` rather than erroring: a font that will not parse should
    /// cost the caller a fallback, not a failed page.
    pub fn parse(data: &[u8], clear_len: usize, encrypted_len: usize) -> Option<Type1Font> {
        // A PFB wrapper segments the file with 0x80 markers. PDF streams are
        // usually raw, but a producer that embedded the PFB verbatim is not
        // rare enough to ignore.
        let owned = strip_pfb(data);
        let data: &[u8] = owned.as_ref();

        let clear_end = clear_len.min(data.len());
        let clear = &data[..clear_end];
        // Trust `eexec` in the bytes over `/Length1` when they disagree: a
        // wrong length is a common producer bug and the marker is unambiguous.
        let start = find(clear, b"eexec")
            .map(|at| skip_eol(data, at + 5))
            .unwrap_or(clear_end);

        let end = match encrypted_len {
            0 => data.len(),
            len => (start + len).min(data.len()),
        };
        let encrypted = data.get(start..end)?;

        // Some producers write the private portion as hex rather than binary.
        let binary = match looks_hex(encrypted) {
            true => unhex(encrypted),
            false => encrypted.to_vec(),
        };
        let private = decrypt(&binary, 55665, 4);

        let len_iv = find(&private, b"/lenIV")
            .and_then(|at| read_int(&private[at + 6..]))
            .unwrap_or(4)
            .clamp(0, 16) as usize;

        let subrs = parse_subrs(&private, len_iv);
        let charstrings = parse_charstrings(&private, len_iv);
        if charstrings.is_empty() {
            return None;
        }

        Some(Type1Font {
            charstrings,
            subrs,
            encoding: parse_encoding(clear),
            font_matrix: parse_font_matrix(clear).unwrap_or([0.001, 0.0, 0.0, 0.001, 0.0, 0.0]),
        })
    }

    /// Whether the font carries a glyph under this name.
    pub fn has(&self, name: &str) -> bool {
        self.charstrings.contains_key(name)
    }

    /// The glyph name this font's own encoding gives a code.
    pub fn glyph_name(&self, code: u8) -> Option<&str> {
        self.encoding.get(&code).map(String::as_str)
    }

    /// Every glyph name the font defines.
    pub fn glyph_names(&self) -> impl Iterator<Item = String> + '_ {
        self.charstrings.keys().cloned()
    }

    /// Interpret a glyph's charstring into an outline.
    pub fn outline(&self, name: &str) -> Option<Glyph> {
        let charstring = self.charstrings.get(name)?;
        let mut state = Interpreter::new(self);
        state.run(charstring, 0);
        state.finish();
        Some(Glyph {
            contours: state.contours,
            advance: state.advance,
        })
    }
}

// ============================================================================
// Container and cipher
// ============================================================================

/// Strip PFB segment headers if present.
fn strip_pfb(data: &[u8]) -> std::borrow::Cow<'_, [u8]> {
    if data.first() != Some(&0x80) {
        return std::borrow::Cow::Borrowed(data);
    }
    let mut out = Vec::with_capacity(data.len());
    let mut at = 0;
    while at + 6 <= data.len() && data[at] == 0x80 {
        let kind = data[at + 1];
        if kind == 3 {
            break; // end-of-file segment
        }
        let len =
            u32::from_le_bytes([data[at + 2], data[at + 3], data[at + 4], data[at + 5]]) as usize;
        let from = at + 6;
        let to = from.saturating_add(len).min(data.len());
        out.extend_from_slice(&data[from..to]);
        at = to;
    }
    std::borrow::Cow::Owned(out)
}

/// The Type 1 cipher, used for both `eexec` and individual charstrings. Fixed
/// key, four discarded plaintext bytes; it obfuscates, it does not protect.
fn decrypt(data: &[u8], key: u16, skip: usize) -> Vec<u8> {
    let mut r = key;
    let mut out = Vec::with_capacity(data.len().saturating_sub(skip));
    for (index, &byte) in data.iter().enumerate() {
        let plain = byte ^ (r >> 8) as u8;
        r = (byte as u16)
            .wrapping_add(r)
            .wrapping_mul(52845)
            .wrapping_add(22719);
        if index >= skip {
            out.push(plain);
        }
    }
    out
}

fn looks_hex(data: &[u8]) -> bool {
    data.iter()
        .take(4)
        .all(|b| b.is_ascii_hexdigit() || b.is_ascii_whitespace())
}

fn unhex(data: &[u8]) -> Vec<u8> {
    let digits: Vec<u8> = data.iter().copied().filter(u8::is_ascii_hexdigit).collect();
    digits
        .chunks(2)
        .filter(|pair| pair.len() == 2)
        .filter_map(|pair| {
            let hi = (pair[0] as char).to_digit(16)?;
            let lo = (pair[1] as char).to_digit(16)?;
            Some(((hi << 4) | lo) as u8)
        })
        .collect()
}

// ============================================================================
// Parsing the decrypted private portion
// ============================================================================

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn skip_eol(data: &[u8], mut at: usize) -> usize {
    while matches!(data.get(at), Some(b'\r' | b'\n' | b' ' | b'\t')) {
        at += 1;
    }
    at
}

fn read_int(data: &[u8]) -> Option<i32> {
    read_int_at(data).map(|(value, _)| value)
}

/// An integer and the offset just past it, so a caller can keep reading from
/// where it stopped.
fn read_int_at(data: &[u8]) -> Option<(i32, usize)> {
    let mut at = 0;
    while matches!(data.get(at), Some(b' ' | b'\r' | b'\n' | b'\t')) {
        at += 1;
    }
    let start = at;
    if data.get(at) == Some(&b'-') {
        at += 1;
    }
    while data.get(at).is_some_and(u8::is_ascii_digit) {
        at += 1;
    }
    let value = std::str::from_utf8(data.get(start..at)?)
        .ok()?
        .parse()
        .ok()?;
    Some((value, at))
}

/// `/Subrs N array dup I L RD <L bytes> NP ...`
fn parse_subrs(private: &[u8], len_iv: usize) -> Vec<Vec<u8>> {
    let Some(start) = find(private, b"/Subrs") else {
        return Vec::new();
    };
    let count = read_int(&private[start + 6..]).unwrap_or(0).max(0) as usize;
    // A count is a hint, not a promise: allocate lazily so a corrupt one
    // cannot ask for gigabytes before a single subroutine is read.
    let mut subrs: Vec<Vec<u8>> = vec![Vec::new(); count.min(65_536)];

    let mut at = start;
    while let Some(rel) = find(&private[at..], b"dup ") {
        let cursor = at + rel + 4;
        // Stop at the charstrings: `dup` appears there too.
        if let Some(cs) = find(private, b"/CharStrings")
            && cursor > cs
        {
            break;
        }
        // `dup <index> <length> RD <bytes> NP`. The length follows the index,
        // so the cursor must step past the index first: reading from the
        // cursor takes the index *as* the length, which yields an empty
        // subroutine 0 and a wrong-sized one everywhere else. Outlines then
        // come out empty for exactly the fonts that keep their drawing in
        // subroutines, which is most text fonts.
        let Some((index, index_end)) = read_int_at(&private[cursor..]) else {
            at = cursor;
            continue;
        };
        let Some((bytes, next)) = read_binary(private, cursor + index_end) else {
            at = cursor + index_end;
            continue;
        };
        let index = index.max(0) as usize;
        if index < subrs.len() {
            subrs[index] = decrypt(bytes, 4330, len_iv);
        }
        at = next;
    }
    subrs
}

/// `/CharStrings N dict dup begin /name L RD <L bytes> ND ...`
fn parse_charstrings(private: &[u8], len_iv: usize) -> HashMap<String, Vec<u8>> {
    let mut out = HashMap::new();
    let Some(start) = find(private, b"/CharStrings") else {
        return out;
    };
    let mut at = start + 12;
    while at < private.len() {
        let Some(rel) = private[at..].iter().position(|&b| b == b'/') else {
            break;
        };
        let name_start = at + rel + 1;
        let mut name_end = name_start;
        while private
            .get(name_end)
            .is_some_and(|b| !b.is_ascii_whitespace() && *b != b'(' && *b != b'{')
        {
            name_end += 1;
        }
        let Ok(name) = std::str::from_utf8(&private[name_start..name_end]) else {
            at = name_end.max(at + 1);
            continue;
        };
        let name = name.to_string();
        match read_binary(private, name_end) {
            Some((bytes, next)) => {
                out.insert(name, decrypt(bytes, 4330, len_iv));
                at = next;
            }
            None => at = name_end.max(at + 1),
        }
    }
    out
}

/// Read `<int> RD <bytes>` starting at `at`, returning the bytes and the
/// position after them. `RD` is conventionally `RD` or `-|`.
fn read_binary(data: &[u8], at: usize) -> Option<(&[u8], usize)> {
    let len = read_int(&data[at..])?;
    if len < 0 || len as usize > data.len() {
        return None;
    }
    // Find the token that introduces the binary, then exactly one space.
    let window_end = (at + 40).min(data.len());
    let window = data.get(at..window_end)?;
    let rel = window
        .windows(2)
        .position(|pair| pair == b"RD" || pair == b"-|")?;
    let start = at + rel + 3; // token plus the single separating space
    let end = start.checked_add(len as usize)?;
    Some((data.get(start..end)?, end))
}

/// The font's built-in encoding, from `dup <code> /<name> put` in the
/// cleartext portion.
fn parse_encoding(clear: &[u8]) -> HashMap<u8, String> {
    let mut out = HashMap::new();
    let mut at = 0;
    while let Some(rel) = find(&clear[at..], b"dup ") {
        let cursor = at + rel + 4;
        at = cursor;
        let Some(code) = read_int(&clear[cursor..]) else {
            continue;
        };
        if !(0..=255).contains(&code) {
            continue;
        }
        let Some(slash) = clear[cursor..].iter().position(|&b| b == b'/') else {
            continue;
        };
        let name_start = cursor + slash + 1;
        let mut name_end = name_start;
        while clear
            .get(name_end)
            .is_some_and(|b| !b.is_ascii_whitespace())
        {
            name_end += 1;
        }
        // Guard against matching a `dup` from some unrelated construct far
        // away: a real entry has its name close behind.
        if name_start - cursor > 12 {
            continue;
        }
        if let Ok(name) = std::str::from_utf8(&clear[name_start..name_end]) {
            out.insert(code as u8, name.to_string());
        }
    }
    out
}

fn parse_font_matrix(clear: &[u8]) -> Option<[f64; 6]> {
    let at = find(clear, b"/FontMatrix")?;
    let open = clear[at..].iter().position(|&b| b == b'[')? + at + 1;
    let close = clear[open..].iter().position(|&b| b == b']')? + open;
    let text = std::str::from_utf8(clear.get(open..close)?).ok()?;
    let values: Vec<f64> = text
        .split_whitespace()
        .filter_map(|token| token.parse().ok())
        .collect();
    match values.len() {
        6 => Some([
            values[0], values[1], values[2], values[3], values[4], values[5],
        ]),
        _ => None,
    }
}

// ============================================================================
// Charstring interpreter
// ============================================================================

struct Interpreter<'a> {
    font: &'a Type1Font,
    stack: Vec<f32>,
    /// The PostScript operand stack `callothersubr` and `pop` communicate over.
    ps_stack: Vec<f32>,
    contours: Vec<Vec<[f32; 2]>>,
    current: Vec<[f32; 2]>,
    x: f32,
    y: f32,
    advance: f32,
    left_side_bearing: f32,
    /// Flex collects seven points across seven `rmoveto`s and then draws two
    /// curves; while it is running, moves accumulate instead of breaking the
    /// contour.
    flex: Option<Vec<[f32; 2]>>,
    done: bool,
}

impl<'a> Interpreter<'a> {
    fn new(font: &'a Type1Font) -> Interpreter<'a> {
        Interpreter {
            font,
            stack: Vec::new(),
            ps_stack: Vec::new(),
            contours: Vec::new(),
            current: Vec::new(),
            x: 0.0,
            y: 0.0,
            advance: 0.0,
            left_side_bearing: 0.0,
            flex: None,
            done: false,
        }
    }

    fn finish(&mut self) {
        self.close();
    }

    fn close(&mut self) {
        if self.current.len() > 2 {
            let contour = std::mem::take(&mut self.current);
            self.contours.push(contour);
        } else {
            self.current.clear();
        }
    }

    fn move_to(&mut self, x: f32, y: f32) {
        match &mut self.flex {
            // During flex, a move is a curve control point, not a new contour.
            Some(points) => points.push([x, y]),
            None => {
                self.close();
                self.current.push([x, y]);
            }
        }
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
            let x = u * u * u * x0 + 3.0 * u * u * t * x1 + 3.0 * u * t * t * x2 + t * t * t * x3;
            let y = u * u * u * y0 + 3.0 * u * u * t * y1 + 3.0 * u * t * t * y2 + t * t * t * y3;
            self.current.push([x, y]);
        }
        self.x = x3;
        self.y = y3;
    }

    fn run(&mut self, code: &[u8], depth: usize) {
        if depth > MAX_SUBR_DEPTH || self.done {
            return;
        }
        let mut at = 0;
        while at < code.len() {
            let byte = code[at];
            at += 1;

            // Operands.
            if byte >= 32 || byte == 255 {
                let value = match byte {
                    255 => {
                        let bytes = code.get(at..at + 4);
                        at += 4;
                        match bytes {
                            Some(b) => i32::from_be_bytes([b[0], b[1], b[2], b[3]]) as f32,
                            None => return,
                        }
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
                    _ => unreachable!("operand ranges are exhaustive"),
                };
                self.stack.push(value);
                continue;
            }

            match byte {
                // hstem, vstem: hinting, which a filled outline does not use.
                1 | 3 => self.stack.clear(),
                4 => {
                    // vmoveto
                    let dy = self.stack.last().copied().unwrap_or(0.0);
                    let (x, y) = (self.x, self.y + dy);
                    self.move_to(x, y);
                    self.stack.clear();
                }
                5 => {
                    // rlineto
                    let (dx, dy) = self.two();
                    let (x, y) = (self.x + dx, self.y + dy);
                    self.line_to(x, y);
                }
                6 => {
                    // hlineto
                    let dx = self.stack.first().copied().unwrap_or(0.0);
                    let (x, y) = (self.x + dx, self.y);
                    self.line_to(x, y);
                    self.stack.clear();
                }
                7 => {
                    // vlineto
                    let dy = self.stack.first().copied().unwrap_or(0.0);
                    let (x, y) = (self.x, self.y + dy);
                    self.line_to(x, y);
                    self.stack.clear();
                }
                8 => {
                    // rrcurveto
                    let a = self.take(6);
                    self.relative_curve(a[0], a[1], a[2], a[3], a[4], a[5]);
                }
                9 => {
                    // closepath
                    self.close();
                    self.stack.clear();
                }
                10 => {
                    // callsubr
                    let Some(index) = self.stack.pop() else {
                        continue;
                    };
                    let index = index as i32;
                    if index >= 0
                        && let Some(subr) = self.font.subrs.get(index as usize)
                    {
                        let subr = subr.clone();
                        self.run(&subr, depth + 1);
                    }
                }
                11 => return, // return
                13 => {
                    // hsbw: sidebearing and advance
                    let a = self.take(2);
                    self.left_side_bearing = a[0];
                    self.advance = a[1];
                    self.x = a[0];
                    self.y = 0.0;
                }
                14 => {
                    // endchar
                    self.close();
                    self.done = true;
                    return;
                }
                21 => {
                    // rmoveto
                    let (dx, dy) = self.two();
                    let (x, y) = (self.x + dx, self.y + dy);
                    self.move_to(x, y);
                }
                22 => {
                    // hmoveto
                    let dx = self.stack.first().copied().unwrap_or(0.0);
                    let (x, y) = (self.x + dx, self.y);
                    self.move_to(x, y);
                    self.stack.clear();
                }
                30 => {
                    // vhcurveto
                    let a = self.take(4);
                    self.relative_curve(0.0, a[0], a[1], a[2], a[3], 0.0);
                }
                31 => {
                    // hvcurveto
                    let a = self.take(4);
                    self.relative_curve(a[0], 0.0, a[1], a[2], 0.0, a[3]);
                }
                12 => {
                    let Some(&second) = code.get(at) else { return };
                    at += 1;
                    self.escape(second, depth);
                }
                _ => self.stack.clear(),
            }
        }
    }

    fn escape(&mut self, op: u8, depth: usize) {
        match op {
            0 => self.stack.clear(),     // dotsection
            1 | 2 => self.stack.clear(), // vstem3, hstem3
            6 => {
                // seac: an accented character built from two standard glyphs.
                let a = self.take(5);
                let (adx, ady, bchar, achar) = (a[1], a[2], a[3] as u8, a[4] as u8);
                self.done = true;
                let base = standard_encoding(bchar).and_then(|n| self.font.outline(n));
                let accent = standard_encoding(achar).and_then(|n| self.font.outline(n));
                if let Some(base) = base {
                    self.contours.extend(base.contours);
                    self.advance = base.advance;
                }
                if let Some(accent) = accent {
                    // The accent is placed relative to the base's sidebearing,
                    // not to the origin.
                    let dx = self.left_side_bearing - accent.advance.min(0.0) + adx
                        - self.left_side_bearing;
                    for contour in accent.contours {
                        self.contours.push(
                            contour
                                .into_iter()
                                .map(|[x, y]| [x + adx + dx - adx, y + ady])
                                .collect(),
                        );
                    }
                }
            }
            7 => {
                // sbw: sidebearing and advance, both axes
                let a = self.take(4);
                self.left_side_bearing = a[0];
                self.advance = a[2];
                self.x = a[0];
                self.y = a[1];
            }
            12 => {
                // div
                let b = self.stack.pop().unwrap_or(1.0);
                let a = self.stack.pop().unwrap_or(0.0);
                self.stack.push(match b {
                    0.0 => 0.0,
                    b => a / b,
                });
            }
            16 => {
                // callothersubr: flex and hint replacement
                let index = self.stack.pop().unwrap_or(0.0) as i32;
                let count = self.stack.pop().unwrap_or(0.0).max(0.0) as usize;
                let mut args = Vec::new();
                for _ in 0..count.min(self.stack.len()) {
                    args.push(self.stack.pop().unwrap_or(0.0));
                }
                args.reverse();
                match index {
                    1 => self.flex = Some(Vec::new()),
                    0 => {
                        // End of flex: seven collected points become two
                        // curves. The first is a reference point and is
                        // dropped.
                        if let Some(points) = self.flex.take()
                            && points.len() >= 7
                        {
                            let p = &points[1..7];
                            let (sx, sy) = (self.x, self.y);
                            self.x = points[0][0];
                            self.y = points[0][1];
                            // Rewind to the contour's real current point: the
                            // flex moves updated x/y as they went.
                            self.x = sx;
                            self.y = sy;
                            self.x = p[0][0];
                            self.y = p[0][1];
                            let start = self.current.last().copied().unwrap_or([sx, sy]);
                            self.x = start[0];
                            self.y = start[1];
                            self.curve_to(p[0][0], p[0][1], p[1][0], p[1][1], p[2][0], p[2][1]);
                            self.curve_to(p[3][0], p[3][1], p[4][0], p[4][1], p[5][0], p[5][1]);
                        }
                        // The interpreter expects two values back for the
                        // `pop pop setcurrentpoint` that follows.
                        self.ps_stack.push(self.y);
                        self.ps_stack.push(self.x);
                    }
                    3 => self.ps_stack.push(3.0),
                    _ => {
                        for arg in args.into_iter().rev() {
                            self.ps_stack.push(arg);
                        }
                    }
                }
                let _ = depth;
            }
            17 => {
                // pop: take a value the othersubr left behind
                let value = self.ps_stack.pop().unwrap_or(0.0);
                self.stack.push(value);
            }
            33 => {
                // setcurrentpoint
                let a = self.take(2);
                self.x = a[0];
                self.y = a[1];
            }
            _ => self.stack.clear(),
        }
    }

    /// The first two operands, then clear. Type 1 operators read from the
    /// bottom of the stack, unlike Type 2.
    fn two(&mut self) -> (f32, f32) {
        let a = self.take(2);
        (a[0], a[1])
    }

    fn take(&mut self, count: usize) -> Vec<f32> {
        let mut out = vec![0.0; count];
        for (slot, value) in out.iter_mut().zip(self.stack.iter()) {
            *slot = *value;
        }
        self.stack.clear();
        out
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

/// The subset of StandardEncoding `seac` needs: the accents and the letters
/// they sit on. A full table would be mostly unused here, since `seac` only
/// ever names glyphs from this range.
fn standard_encoding(code: u8) -> Option<&'static str> {
    Some(match code {
        b'A'..=b'Z' => return Some(LETTERS[(code - b'A') as usize]),
        b'a'..=b'z' => return Some(LOWER[(code - b'a') as usize]),
        193 => "grave",
        194 => "acute",
        195 => "circumflex",
        196 => "tilde",
        197 => "macron",
        198 => "breve",
        199 => "dotaccent",
        200 => "dieresis",
        202 => "ring",
        203 => "cedilla",
        205 => "hungarumlaut",
        206 => "ogonek",
        207 => "caron",
        _ => return None,
    })
}

const LETTERS: [&str; 26] = [
    "A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "O", "P", "Q", "R", "S",
    "T", "U", "V", "W", "X", "Y", "Z",
];
const LOWER: [&str; 26] = [
    "a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l", "m", "n", "o", "p", "q", "r", "s",
    "t", "u", "v", "w", "x", "y", "z",
];

#[cfg(test)]
mod tests {
    use super::*;

    /// Encode a number the way a charstring does, so the tests exercise the
    /// same decoder a real font does rather than a convenient shortcut.
    fn num(value: i32) -> Vec<u8> {
        match value {
            -107..=107 => vec![(value + 139) as u8],
            108..=1131 => {
                let v = value - 108;
                vec![(247 + (v >> 8)) as u8, (v & 0xFF) as u8]
            }
            -1131..=-108 => {
                let v = -value - 108;
                vec![(251 + (v >> 8)) as u8, (v & 0xFF) as u8]
            }
            _ => {
                let mut out = vec![255u8];
                out.extend_from_slice(&value.to_be_bytes());
                out
            }
        }
    }

    /// The cipher is its own inverse given the same key, which is the property
    /// every other test here depends on.
    #[test]
    fn the_cipher_round_trips() {
        let plain = b"hello type 1";
        // Encrypt: the inverse of `decrypt`, with four leading pad bytes.
        let mut r: u16 = 4330;
        let mut cipher = Vec::new();
        for &byte in [0u8; 4].iter().chain(plain.iter()) {
            let c = byte ^ (r >> 8) as u8;
            r = (c as u16)
                .wrapping_add(r)
                .wrapping_mul(52845)
                .wrapping_add(22719);
            cipher.push(c);
        }
        assert_eq!(decrypt(&cipher, 4330, 4), plain);
    }

    #[test]
    fn pfb_segments_are_stripped() {
        let mut pfb = vec![0x80, 0x01, 3, 0, 0, 0];
        pfb.extend_from_slice(b"abc");
        pfb.extend_from_slice(&[0x80, 0x02, 2, 0, 0, 0]);
        pfb.extend_from_slice(b"de");
        pfb.extend_from_slice(&[0x80, 0x03]);
        assert_eq!(strip_pfb(&pfb).as_ref(), b"abcde");
    }

    #[test]
    fn a_font_without_charstrings_is_refused_rather_than_half_parsed() {
        assert!(Type1Font::parse(b"not a font at all", 5, 5).is_none());
    }

    /// Operand encoding is the part of the charstring format that is easiest to
    /// get subtly wrong, and every outline depends on it.
    #[test]
    fn operands_decode_across_every_range() {
        let font = Type1Font {
            charstrings: HashMap::new(),
            subrs: Vec::new(),
            encoding: HashMap::new(),
            font_matrix: [0.001, 0.0, 0.0, 0.001, 0.0, 0.0],
        };
        let mut interp = Interpreter::new(&font);
        // 139 -> 0, 32 -> -107, 246 -> 107
        interp.run(&[139, 32, 246], 0);
        assert_eq!(interp.stack, vec![0.0, -107.0, 107.0]);

        interp.stack.clear();
        // 247 0 -> 108 ; 251 0 -> -108
        interp.run(&[247, 0, 251, 0], 0);
        assert_eq!(interp.stack, vec![108.0, -108.0]);

        interp.stack.clear();
        // 255 followed by a big-endian i32
        interp.run(&[255, 0x00, 0x00, 0x04, 0x00], 0);
        assert_eq!(interp.stack, vec![1024.0]);
    }

    /// A square drawn with the operators a real glyph uses.
    #[test]
    fn a_charstring_becomes_a_closed_contour() {
        let mut charstrings = HashMap::new();
        // hsbw 0 500 ; rmoveto 100 100 ; rlineto 200 0 ; rlineto 0 200 ;
        // rlineto -200 0 ; closepath ; endchar
        let mut code = Vec::new();
        code.extend(num(0));
        code.extend(num(500));
        code.push(13); // hsbw 0 500
        code.extend(num(100));
        code.extend(num(100));
        code.push(21); // rmoveto 100 100
        code.extend(num(200));
        code.extend(num(0));
        code.push(5); // rlineto 200 0
        code.extend(num(0));
        code.extend(num(200));
        code.push(5); // rlineto 0 200
        code.extend(num(-200));
        code.extend(num(0));
        code.push(5); // rlineto -200 0
        code.push(9); // closepath
        code.push(14); // endchar
        charstrings.insert("square".to_string(), code);
        let font = Type1Font {
            charstrings,
            subrs: Vec::new(),
            encoding: HashMap::new(),
            font_matrix: [0.001, 0.0, 0.0, 0.001, 0.0, 0.0],
        };
        let glyph = font.outline("square").expect("the glyph parses");
        assert_eq!(glyph.contours.len(), 1, "one closed contour");
        let contour = &glyph.contours[0];
        assert!(contour.len() >= 4, "a square has four corners: {contour:?}");
        // The outline must sit where the operators put it.
        let xs: Vec<f32> = contour.iter().map(|p| p[0]).collect();
        let ys: Vec<f32> = contour.iter().map(|p| p[1]).collect();
        assert_eq!(xs.iter().cloned().fold(f32::MAX, f32::min), 100.0);
        assert_eq!(xs.iter().cloned().fold(f32::MIN, f32::max), 300.0);
        assert_eq!(ys.iter().cloned().fold(f32::MAX, f32::min), 100.0);
        assert_eq!(ys.iter().cloned().fold(f32::MIN, f32::max), 300.0);
    }

    /// A curve must be flattened into points rather than dropped, or round
    /// letters come out as straight lines.
    #[test]
    fn a_curve_is_flattened() {
        let mut charstrings = HashMap::new();
        // hsbw ; rmoveto 0 0 ; rrcurveto 100 0 100 100 0 100 ; endchar
        let mut code = Vec::new();
        code.extend(num(0));
        code.extend(num(500));
        code.push(13); // hsbw
        code.extend(num(0));
        code.extend(num(0));
        code.push(21); // rmoveto
        for delta in [100, 0, 100, 100, 0, 100] {
            code.extend(num(delta));
        }
        code.push(8); // rrcurveto
        code.push(14); // endchar
        charstrings.insert("curve".to_string(), code);
        let font = Type1Font {
            charstrings,
            subrs: Vec::new(),
            encoding: HashMap::new(),
            font_matrix: [0.001, 0.0, 0.0, 0.001, 0.0, 0.0],
        };
        let glyph = font.outline("curve").expect("parses");
        let points = glyph.contours.first().map(Vec::len).unwrap_or(0);
        assert!(points > CURVE_STEPS, "a curve should add points: {points}");
    }

    /// A subroutine that calls itself must not hang the renderer.
    #[test]
    fn runaway_subroutine_recursion_is_bounded() {
        let mut charstrings = HashMap::new();
        // callsubr 0, where subr 0 calls subr 0
        charstrings.insert("loop".to_string(), vec![139, 10, 14]); // 139 == 0
        let font = Type1Font {
            charstrings,
            subrs: vec![vec![139, 10]], // subr 0 calls subr 0
            encoding: HashMap::new(),
            font_matrix: [0.001, 0.0, 0.0, 0.001, 0.0, 0.0],
        };
        // The assertion is that this returns at all.
        assert!(font.outline("loop").is_some());
    }
}

#[cfg(test)]
mod real_font_tests {

    /// Parse the Type 1 programs actually embedded in the sample PDF.
    ///
    /// The synthetic tests prove the interpreter obeys the operators; this
    /// proves it survives what a real TeX distribution emits, which is where
    /// flex, subroutines and unusual `/lenIV` values actually appear.
    #[test]
    fn real_embedded_fonts_yield_outlines() {
        let Ok(pdf) = std::fs::read("../demos/sample.pdf") else {
            eprintln!("skipping: demos/sample.pdf not present");
            return;
        };

        let fonts = crate::font::embedded_type1(&pdf);
        assert!(
            fonts.len() >= 5,
            "expected the sample's embedded fonts, found {}",
            fonts.len()
        );

        let mut with_outlines = 0;
        let mut total_points = 0usize;
        for font in &fonts {
            // Every font must yield at least one non-empty outline, or the
            // charstring interpreter is silently producing nothing.
            let names: Vec<String> = font.glyph_names().take(40).collect();
            let drawn = names
                .iter()
                .filter(|name| name.as_str() != ".notdef")
                .filter_map(|name| font.outline(name))
                .filter(|glyph| !glyph.contours.is_empty())
                .inspect(|glyph| total_points += glyph.contours.iter().map(Vec::len).sum::<usize>())
                .count();
            if drawn > 0 {
                with_outlines += 1;
            }
        }
        assert_eq!(
            with_outlines,
            fonts.len(),
            "every embedded font should produce outlines"
        );
        assert!(total_points > 500, "outlines look empty: {total_points}");
    }

    /// A letter's outline must sit in a plausible place: inside the em, above
    /// the baseline, and wide enough to be a glyph rather than a stray point.
    #[test]
    fn a_real_glyph_has_sane_geometry() {
        let Ok(pdf) = std::fs::read("../demos/sample.pdf") else {
            eprintln!("skipping: demos/sample.pdf not present");
            return;
        };
        let fonts = crate::font::embedded_type1(&pdf);
        let Some((font, name)) = fonts.iter().find_map(|font| {
            font.glyph_names()
                .find(|n| n == "e" || n == "o" || n == "n")
                .map(|n| (font, n))
        }) else {
            eprintln!("skipping: no lowercase letter in the subset fonts");
            return;
        };

        let glyph = font.outline(&name).expect("the glyph parses");
        let points: Vec<[f32; 2]> = glyph.contours.iter().flatten().copied().collect();
        assert!(!points.is_empty(), "{name} has no outline");

        let min_x = points.iter().map(|p| p[0]).fold(f32::MAX, f32::min);
        let max_x = points.iter().map(|p| p[0]).fold(f32::MIN, f32::max);
        let min_y = points.iter().map(|p| p[1]).fold(f32::MAX, f32::min);
        let max_y = points.iter().map(|p| p[1]).fold(f32::MIN, f32::max);

        assert!(
            max_x - min_x > 50.0,
            "{name} is too narrow: {min_x}..{max_x}"
        );
        assert!(
            max_y - min_y > 50.0,
            "{name} is too short: {min_y}..{max_y}"
        );
        // An x-height letter sits on the baseline and stays under the em.
        assert!(
            min_y > -250.0 && max_y < 1200.0,
            "{name} y range {min_y}..{max_y}"
        );
        assert!(
            min_x > -500.0 && max_x < 1500.0,
            "{name} x range {min_x}..{max_x}"
        );
    }
}

#[cfg(test)]
mod debug_real {
    /// Real letters must have believable metrics, not merely non-empty ones.
    ///
    /// An interpreter can produce contours that are the wrong size or in the
    /// wrong place and still look "parsed"; these bounds are what a Times-like
    /// text font actually measures, so a regression in the charstring maths
    /// shows up here rather than on screen.
    #[test]
    fn body_font_letters_have_believable_metrics() {
        let Ok(pdf) = std::fs::read("../demos/sample.pdf") else {
            eprintln!("skipping: demos/sample.pdf not present");
            return;
        };
        let fonts = crate::font::embedded_fonts(&pdf);
        let Some(font) = fonts
            .iter()
            .find(|f| f.base_font.contains("NimbusRomNo9L-Regu"))
        else {
            eprintln!("skipping: body font not embedded");
            return;
        };

        // `l` is a stem with foot serifs: tall, sitting on the baseline, and
        // inside its own advance. Bounds are stated against the advance rather
        // than as absolute units, because a serif face's `l` is wider than the
        // stem alone and guessing a number tests the guess.
        let mut narrow = 0.0f32;
        if let Some(glyph) = font.program.as_type1().expect("type 1").outline("l") {
            let (w, h, min_y) = extent(&glyph);
            narrow = w;
            assert!(
                w <= glyph.advance,
                "l should fit its advance: {w} vs {}",
                glyph.advance
            );
            assert!(h > 500.0, "l should be tall, got {h}");
            assert!(
                min_y.abs() < 30.0,
                "l should sit on the baseline, got {min_y}"
            );
            assert_eq!(glyph.advance, 278.0, "the font's own advance for l");
        }

        // `o` is round and has a counter, so two contours.
        if let Some(glyph) = font.program.as_type1().expect("type 1").outline("o") {
            let (w, h, min_y) = extent(&glyph);
            assert_eq!(glyph.contours.len(), 2, "o has an outer and inner contour");
            assert!((350.0..520.0).contains(&w), "o width {w}");
            assert!((350.0..520.0).contains(&h), "o height {h}");
            assert!(min_y > -30.0 && min_y < 10.0, "o baseline {min_y}");
        }

        // `A` is a cap: as tall as the cap height, wider than it is deep.
        if let Some(glyph) = font.program.as_type1().expect("type 1").outline("A") {
            let (w, h, _) = extent(&glyph);
            assert!(h > 600.0, "A should reach cap height, got {h}");
            assert!(w > 500.0, "A should be wide, got {w}");
            assert!(w > narrow, "A should be wider than l: {w} vs {narrow}");
            assert_eq!(glyph.advance, 722.0, "the font's own advance for A");
        }
    }

    fn extent(glyph: &super::Glyph) -> (f32, f32, f32) {
        let points: Vec<[f32; 2]> = glyph.contours.iter().flatten().copied().collect();
        let min_x = points.iter().map(|p| p[0]).fold(f32::MAX, f32::min);
        let max_x = points.iter().map(|p| p[0]).fold(f32::MIN, f32::max);
        let min_y = points.iter().map(|p| p[1]).fold(f32::MAX, f32::min);
        let max_y = points.iter().map(|p| p[1]).fold(f32::MIN, f32::max);
        (max_x - min_x, max_y - min_y, min_y)
    }
}
