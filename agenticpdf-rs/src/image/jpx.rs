//! A JPEG 2000 (ITU-T T.800 Part 1) decoder.
//!
//! Scanned books reach us as JPEG 2000 far more often than the format's
//! reputation suggests, and a page whose only content is one JPX image
//! renders as blank paper without this. No browser but Safari decodes it
//! either, so handing the bytes onward is not a fallback.
//!
//! The whole chain is here: JP2 boxes, codestream markers, packet headers
//! with their tag trees, the MQ arithmetic coder, EBCOT tier-1 coefficient
//! modelling, dequantization, the inverse wavelet transform, and the
//! component transform. No dependencies, in keeping with the rest of the
//! crate.

/// A decoded image: `components` interleaved 8-bit samples per pixel.
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub components: usize,
    pub data: Vec<u8>,
}

/// Decode a JP2 file or a raw codestream. `None` for anything malformed or
/// outside the Part 1 baseline this implements.
pub fn decode(data: &[u8]) -> Option<Image> {
    decode_codestream(codestream(data)?)
}

// ============================================================================
// JP2 container
// ============================================================================

/// A JP2 file wraps the codestream in boxes; a raw codestream opens with SOC.
fn codestream(data: &[u8]) -> Option<&[u8]> {
    if data.starts_with(&[0xFF, 0x4F, 0xFF, 0x51]) {
        return Some(data);
    }
    let mut off = 0usize;
    while off + 8 <= data.len() {
        let declared = u32::from_be_bytes(data[off..off + 4].try_into().ok()?) as usize;
        let kind = &data[off + 4..off + 8];
        let (start, len) = if declared == 1 {
            if off + 16 > data.len() {
                return None;
            }
            let l = u64::from_be_bytes(data[off + 8..off + 16].try_into().ok()?) as usize;
            (off + 16, l)
        } else if declared == 0 {
            (off + 8, data.len() - off)
        } else {
            (off + 8, declared)
        };
        if start > data.len() || len < start - off {
            return None;
        }
        let end = off.saturating_add(len).min(data.len());
        if kind == b"jp2c" {
            return data.get(start..end);
        }
        off = end.max(off + 8);
    }
    None
}

// ============================================================================
// Codestream headers
// ============================================================================

#[derive(Clone, Copy)]
struct CompSpec {
    depth: u8,
    signed: bool,
    dx: u32,
    dy: u32,
}

#[derive(Clone)]
struct Siz {
    xsiz: i64,
    ysiz: i64,
    xosiz: i64,
    yosiz: i64,
    xtsiz: i64,
    ytsiz: i64,
    xtosiz: i64,
    ytosiz: i64,
    comps: Vec<CompSpec>,
}

#[derive(Clone)]
struct Cod {
    prog: u8,
    layers: u16,
    mct: u8,
    levels: u8,
    xcb: u8,
    ycb: u8,
    cbsty: u8,
    reversible: bool,
    /// `(PPx, PPy)` per resolution, lowest first.
    precincts: Vec<(u8, u8)>,
    sop: bool,
    eph: bool,
}

#[derive(Clone)]
struct Qcd {
    style: u8,
    guard: u8,
    /// `(exponent, mantissa)` per band, in codestream order.
    steps: Vec<(u8, u16)>,
}

fn be16(b: &[u8], i: usize) -> Option<u16> {
    Some(u16::from_be_bytes(b.get(i..i + 2)?.try_into().ok()?))
}

fn be32(b: &[u8], i: usize) -> Option<u32> {
    Some(u32::from_be_bytes(b.get(i..i + 4)?.try_into().ok()?))
}

fn parse_siz(seg: &[u8]) -> Option<Siz> {
    let ncomp = be16(seg, 34)? as usize;
    if ncomp == 0 || ncomp > 16 || seg.len() < 36 + 3 * ncomp {
        return None;
    }
    let mut comps = Vec::with_capacity(ncomp);
    for k in 0..ncomp {
        let s = seg[36 + 3 * k];
        comps.push(CompSpec {
            depth: (s & 0x7F) + 1,
            signed: s & 0x80 != 0,
            dx: seg[37 + 3 * k].max(1) as u32,
            dy: seg[38 + 3 * k].max(1) as u32,
        });
    }
    Some(Siz {
        xsiz: be32(seg, 2)? as i64,
        ysiz: be32(seg, 6)? as i64,
        xosiz: be32(seg, 10)? as i64,
        yosiz: be32(seg, 14)? as i64,
        xtsiz: be32(seg, 18)? as i64,
        ytsiz: be32(seg, 22)? as i64,
        xtosiz: be32(seg, 26)? as i64,
        ytosiz: be32(seg, 30)? as i64,
        comps,
    })
}

/// The style byte and everything after it, shared by COD and COC.
fn parse_cod_body(scod: u8, b: &[u8], with_prog: bool) -> Option<Cod> {
    let (prog, layers, mct, rest) = if with_prog {
        (*b.first()?, be16(b, 1)?, *b.get(3)?, b.get(4..)?)
    } else {
        (0, 1, 0, b)
    };
    let levels = *rest.first()?;
    if levels > 32 {
        return None;
    }
    let xcb = (rest.get(1)? & 0x0F) + 2;
    let ycb = (rest.get(2)? & 0x0F) + 2;
    let cbsty = *rest.get(3)?;
    let reversible = *rest.get(4)? == 1;
    let nres = levels as usize + 1;
    let precincts = if scod & 1 != 0 {
        let mut v = Vec::with_capacity(nres);
        for r in 0..nres {
            let byte = *rest.get(5 + r)?;
            v.push((byte & 0x0F, byte >> 4));
        }
        v
    } else {
        vec![(15, 15); nres]
    };
    Some(Cod {
        prog,
        layers: layers.max(1),
        mct,
        levels,
        xcb,
        ycb,
        cbsty,
        reversible,
        precincts,
        sop: scod & 2 != 0,
        eph: scod & 4 != 0,
    })
}

fn parse_qcd_body(seg: &[u8]) -> Option<Qcd> {
    let sq = *seg.first()?;
    let style = sq & 0x1F;
    let guard = sq >> 5;
    let body = &seg[1..];
    let mut steps = Vec::new();
    match style {
        0 => {
            for &b in body {
                steps.push((b >> 3, 0));
            }
        }
        1 | 2 => {
            let mut i = 0;
            while i + 2 <= body.len() {
                let v = be16(body, i)?;
                steps.push(((v >> 11) as u8, v & 0x07FF));
                i += 2;
            }
        }
        _ => return None,
    }
    if steps.is_empty() {
        return None;
    }
    Some(Qcd {
        style,
        guard,
        steps,
    })
}

// ============================================================================
// Geometry
// ============================================================================

fn ceil_div(a: i64, b: i64) -> i64 {
    debug_assert!(b > 0);
    (a + b - 1).div_euclid(b)
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
struct Rect {
    x0: i64,
    y0: i64,
    x1: i64,
    y1: i64,
}

impl Rect {
    fn w(&self) -> usize {
        (self.x1 - self.x0).max(0) as usize
    }
    fn h(&self) -> usize {
        (self.y1 - self.y0).max(0) as usize
    }
    fn area(&self) -> usize {
        self.w() * self.h()
    }
}

// ============================================================================
// Bit reader for packet headers (with the mandatory bit stuffing)
// ============================================================================

struct Bits<'a> {
    d: &'a [u8],
    pos: usize,
    cur: u8,
    ct: u8,
}

impl<'a> Bits<'a> {
    fn new(d: &'a [u8], pos: usize) -> Self {
        Bits {
            d,
            pos,
            cur: 0,
            ct: 0,
        }
    }

    /// A byte following 0xFF carries seven bits, not eight -- the stuffing
    /// that keeps a packet header from ever looking like a marker.
    fn bit(&mut self) -> u32 {
        if self.ct == 0 {
            let stuffed = self.cur == 0xFF;
            self.cur = self.d.get(self.pos).copied().unwrap_or(0);
            self.pos += 1;
            self.ct = if stuffed { 7 } else { 8 };
        }
        self.ct -= 1;
        ((self.cur >> self.ct) & 1) as u32
    }

    fn bits(&mut self, n: u32) -> u32 {
        let mut v = 0;
        for _ in 0..n.min(32) {
            v = (v << 1) | self.bit();
        }
        v
    }

    /// End the header: discard the part-byte, and the stuffed byte with it.
    fn align(&mut self) {
        if self.cur == 0xFF {
            self.pos += 1;
        }
        self.cur = 0;
        self.ct = 0;
    }
}

// ============================================================================
// Tag trees
// ============================================================================

struct TagLevel {
    w: usize,
    value: Vec<u32>,
    known: Vec<bool>,
}

struct TagTree {
    levels: Vec<TagLevel>,
}

impl TagTree {
    fn new(w: usize, h: usize) -> TagTree {
        let mut levels = Vec::new();
        let (mut lw, mut lh) = (w.max(1), h.max(1));
        loop {
            levels.push(TagLevel {
                w: lw,
                value: vec![0; lw * lh],
                known: vec![false; lw * lh],
            });
            if lw == 1 && lh == 1 {
                break;
            }
            lw = lw.div_ceil(2);
            lh = lh.div_ceil(2);
        }
        TagTree { levels }
    }

    /// Read until this leaf's value is known, or is proven to be at least
    /// `threshold`. The value returned is `>= threshold` in the latter case.
    fn decode(&mut self, r: &mut Bits, x: usize, y: usize, threshold: u32) -> u32 {
        let mut lo = 0u32;
        for l in (0..self.levels.len()).rev() {
            let lev = &mut self.levels[l];
            let idx = (y >> l) * lev.w + (x >> l);
            if idx >= lev.value.len() {
                return lo;
            }
            if lev.value[idx] < lo {
                lev.value[idx] = lo;
            }
            while !lev.known[idx] && lev.value[idx] < threshold {
                if r.bit() == 1 {
                    lev.known[idx] = true;
                } else {
                    lev.value[idx] += 1;
                }
            }
            lo = lev.value[idx];
            if !lev.known[idx] {
                return lo;
            }
        }
        lo
    }
}

// ============================================================================
// MQ arithmetic decoder (T.800 Annex C)
// ============================================================================

/// `(Qe, NMPS, NLPS, SWITCH)`.
const QE: [(u32, u8, u8, u8); 47] = [
    (0x5601, 1, 1, 1),
    (0x3401, 2, 6, 0),
    (0x1801, 3, 9, 0),
    (0x0AC1, 4, 12, 0),
    (0x0521, 5, 29, 0),
    (0x0221, 38, 33, 0),
    (0x5601, 7, 6, 1),
    (0x5401, 8, 14, 0),
    (0x4801, 9, 14, 0),
    (0x3801, 10, 14, 0),
    (0x3001, 11, 17, 0),
    (0x2401, 12, 18, 0),
    (0x1C01, 13, 20, 0),
    (0x1601, 29, 21, 0),
    (0x5601, 15, 14, 1),
    (0x5401, 16, 14, 0),
    (0x5101, 17, 15, 0),
    (0x4801, 18, 16, 0),
    (0x3801, 19, 17, 0),
    (0x3401, 20, 18, 0),
    (0x3001, 21, 19, 0),
    (0x2801, 22, 19, 0),
    (0x2401, 23, 20, 0),
    (0x2201, 24, 21, 0),
    (0x1C01, 25, 22, 0),
    (0x1801, 26, 23, 0),
    (0x1601, 27, 24, 0),
    (0x1401, 28, 25, 0),
    (0x1201, 29, 26, 0),
    (0x1101, 30, 27, 0),
    (0x0AC1, 31, 28, 0),
    (0x09C1, 32, 29, 0),
    (0x08A1, 33, 30, 0),
    (0x0521, 34, 31, 0),
    (0x0441, 35, 32, 0),
    (0x02A1, 36, 33, 0),
    (0x0221, 37, 34, 0),
    (0x0141, 38, 35, 0),
    (0x0111, 39, 36, 0),
    (0x0085, 40, 37, 0),
    (0x0049, 41, 38, 0),
    (0x0025, 42, 39, 0),
    (0x0015, 43, 40, 0),
    (0x0009, 44, 41, 0),
    (0x0005, 45, 42, 0),
    (0x0001, 45, 43, 0),
    (0x5601, 46, 46, 0),
];

struct Mq<'a> {
    d: &'a [u8],
    bp: usize,
    chigh: u32,
    clow: u32,
    ct: i32,
    a: u32,
}

impl<'a> Mq<'a> {
    fn new(d: &'a [u8]) -> Mq<'a> {
        let mut mq = Mq {
            d,
            bp: 0,
            chigh: d.first().copied().unwrap_or(0xFF) as u32,
            clow: 0,
            ct: 0,
            a: 0,
        };
        mq.byte_in();
        mq.chigh = ((mq.chigh << 7) & 0xFFFF) | ((mq.clow >> 9) & 0x7F);
        mq.clow = (mq.clow << 7) & 0xFFFF;
        mq.ct -= 7;
        mq.a = 0x8000;
        mq
    }

    fn at(&self, i: usize) -> u32 {
        self.d.get(i).copied().unwrap_or(0xFF) as u32
    }

    fn byte_in(&mut self) {
        if self.at(self.bp) == 0xFF {
            if self.at(self.bp + 1) > 0x8F {
                self.clow += 0xFF00;
                self.ct = 8;
            } else {
                self.bp += 1;
                self.clow += self.at(self.bp) << 9;
                self.ct = 7;
            }
        } else {
            self.bp += 1;
            self.clow += if self.bp < self.d.len() {
                self.at(self.bp) << 8
            } else {
                0xFF00
            };
            self.ct = 8;
        }
        if self.clow > 0xFFFF {
            self.chigh += self.clow >> 16;
            self.clow &= 0xFFFF;
        }
    }

    /// Decode one bit against context `cx`, whose state lives in `ctx`.
    fn bit(&mut self, ctx: &mut [u8; 19], cx: usize) -> u32 {
        let mut index = (ctx[cx] >> 1) as usize;
        let mut mps = (ctx[cx] & 1) as u32;
        let (qe, nmps, nlps, switch) = QE[index];
        let d;
        let mut a = self.a.wrapping_sub(qe);
        if self.chigh < qe {
            if a < qe {
                a = qe;
                d = mps;
                index = nmps as usize;
            } else {
                a = qe;
                d = 1 ^ mps;
                if switch == 1 {
                    mps = d;
                }
                index = nlps as usize;
            }
        } else {
            self.chigh -= qe;
            if a & 0x8000 != 0 {
                self.a = a;
                return mps;
            }
            if a < qe {
                d = 1 ^ mps;
                if switch == 1 {
                    mps = d;
                }
                index = nlps as usize;
            } else {
                d = mps;
                index = nmps as usize;
            }
        }
        loop {
            if self.ct == 0 {
                self.byte_in();
            }
            a <<= 1;
            self.chigh = ((self.chigh << 1) & 0xFFFF) | ((self.clow >> 15) & 1);
            self.clow = (self.clow << 1) & 0xFFFF;
            self.ct -= 1;
            if a & 0x8000 != 0 {
                break;
            }
        }
        self.a = a;
        ctx[cx] = ((index as u8) << 1) | mps as u8;
        d
    }
}

// ============================================================================
// EBCOT tier-1: one code-block's coefficient bits
// ============================================================================

const CX_UNIFORM: usize = 17;
const CX_RUNLENGTH: usize = 18;

struct Block {
    w: usize,
    h: usize,
    sig: Vec<u8>,
    sign: Vec<u8>,
    mag: Vec<u32>,
    visit: Vec<u8>,
    refined: Vec<u8>,
    /// Bit-planes actually decoded, per coefficient.
    planes: Vec<u8>,
    ctx: [u8; 19],
}

impl Block {
    fn new(w: usize, h: usize) -> Block {
        let n = w * h;
        let mut ctx = [0u8; 19];
        // The three contexts the standard starts away from state zero.
        ctx[0] = 4 << 1;
        ctx[CX_UNIFORM] = 46 << 1;
        ctx[CX_RUNLENGTH] = 3 << 1;
        Block {
            w,
            h,
            sig: vec![0; n],
            sign: vec![0; n],
            mag: vec![0; n],
            visit: vec![0; n],
            refined: vec![0; n],
            planes: vec![0; n],
            ctx,
        }
    }

    fn sig_at(&self, x: i64, y: i64) -> u32 {
        if x < 0 || y < 0 || x >= self.w as i64 || y >= self.h as i64 {
            return 0;
        }
        self.sig[y as usize * self.w + x as usize] as u32
    }

    /// Signed contribution of a neighbour: +1 positive, -1 negative, 0 absent.
    fn sign_at(&self, x: i64, y: i64) -> i32 {
        if x < 0 || y < 0 || x >= self.w as i64 || y >= self.h as i64 {
            return 0;
        }
        let i = y as usize * self.w + x as usize;
        if self.sig[i] == 0 {
            0
        } else if self.sign[i] == 0 {
            1
        } else {
            -1
        }
    }

    /// Neighbour significance as `(horizontal, vertical, diagonal)` counts.
    fn neighbours(&self, x: usize, y: usize) -> (u32, u32, u32) {
        let (xi, yi) = (x as i64, y as i64);
        let h = self.sig_at(xi - 1, yi) + self.sig_at(xi + 1, yi);
        let v = self.sig_at(xi, yi - 1) + self.sig_at(xi, yi + 1);
        let d = self.sig_at(xi - 1, yi - 1)
            + self.sig_at(xi + 1, yi - 1)
            + self.sig_at(xi - 1, yi + 1)
            + self.sig_at(xi + 1, yi + 1);
        (h, v, d)
    }

    /// Table D.1: the significance context, which depends on the band.
    fn sig_context(&self, x: usize, y: usize, band: u8) -> usize {
        let (h, v, d) = self.neighbours(x, y);
        // HL reads the table with the two axes exchanged.
        let (h, v) = if band == 1 { (v, h) } else { (h, v) };
        if band == 3 {
            let hv = h + v;
            return match (d, hv) {
                (0, 0) => 0,
                (0, 1) => 1,
                (0, _) => 2,
                (1, 0) => 3,
                (1, 1) => 4,
                (1, _) => 5,
                (2, 0) => 6,
                (2, _) => 7,
                _ => 8,
            };
        }
        match (h, v, d) {
            (2, _, _) => 8,
            (1, 0, 0) => 5,
            (1, 0, _) => 6,
            (1, _, _) => 7,
            (0, 2, _) => 4,
            (0, 1, _) => 3,
            (0, 0, 0) => 0,
            (0, 0, 1) => 1,
            _ => 2,
        }
    }

    /// Table D.3: sign context, and the bit the decoded value is XORed with.
    fn sign_context(&self, x: usize, y: usize) -> (usize, u32) {
        let (xi, yi) = (x as i64, y as i64);
        let h = (self.sign_at(xi - 1, yi) + self.sign_at(xi + 1, yi)).clamp(-1, 1);
        let v = (self.sign_at(xi, yi - 1) + self.sign_at(xi, yi + 1)).clamp(-1, 1);
        match (h, v) {
            (1, 1) => (13, 0),
            (1, 0) => (12, 0),
            (1, -1) => (11, 0),
            (0, 1) => (10, 0),
            (0, 0) => (9, 0),
            (0, -1) => (10, 1),
            (-1, 1) => (11, 1),
            (-1, 0) => (12, 1),
            _ => (13, 1),
        }
    }

    fn decode_sign(&mut self, mq: &mut Mq, x: usize, y: usize) -> u8 {
        let (cx, xorbit) = self.sign_context(x, y);
        (mq.bit(&mut self.ctx, cx) ^ xorbit) as u8
    }

    fn significance_pass(&mut self, mq: &mut Mq, band: u8, plane: u32) {
        for y0 in (0..self.h).step_by(4) {
            for x in 0..self.w {
                for y in y0..(y0 + 4).min(self.h) {
                    let i = y * self.w + x;
                    if self.sig[i] != 0 {
                        continue;
                    }
                    let cx = self.sig_context(x, y, band);
                    if cx == 0 {
                        continue;
                    }
                    self.planes[i] = self.planes[i].saturating_add(1);
                    if mq.bit(&mut self.ctx, cx) == 1 {
                        self.sign[i] = self.decode_sign(mq, x, y);
                        self.sig[i] = 1;
                        self.mag[i] = 1 << plane;
                    }
                    self.visit[i] = 1;
                }
            }
        }
    }

    fn refinement_pass(&mut self, mq: &mut Mq, plane: u32) {
        for y0 in (0..self.h).step_by(4) {
            for x in 0..self.w {
                for y in y0..(y0 + 4).min(self.h) {
                    let i = y * self.w + x;
                    if self.sig[i] == 0 || self.visit[i] != 0 {
                        continue;
                    }
                    let cx = if self.refined[i] != 0 {
                        16
                    } else {
                        let (h, v, d) = self.neighbours(x, y);
                        if h + v + d > 0 { 15 } else { 14 }
                    };
                    let b = mq.bit(&mut self.ctx, cx);
                    self.refined[i] = 1;
                    self.planes[i] = self.planes[i].saturating_add(1);
                    self.mag[i] |= b << plane;
                }
            }
        }
    }

    fn cleanup_pass(&mut self, mq: &mut Mq, band: u8, plane: u32) {
        for y0 in (0..self.h).step_by(4) {
            let stripe_end = (y0 + 4).min(self.h);
            for x in 0..self.w {
                let mut y = y0;
                // A full stripe column with no significant neighbour at all
                // is coded as one run-length symbol rather than four.
                if stripe_end - y0 == 4 {
                    let runnable = (y0..stripe_end).all(|yy| {
                        let i = yy * self.w + x;
                        self.sig[i] == 0 && self.visit[i] == 0 && self.sig_context(x, yy, band) == 0
                    });
                    if runnable {
                        if mq.bit(&mut self.ctx, CX_RUNLENGTH) == 0 {
                            for yy in y0..stripe_end {
                                let i = yy * self.w + x;
                                self.planes[i] = self.planes[i].saturating_add(1);
                            }
                            continue;
                        }
                        let k = (mq.bit(&mut self.ctx, CX_UNIFORM) << 1)
                            | mq.bit(&mut self.ctx, CX_UNIFORM);
                        let first = y0 + k as usize;
                        for yy in y0..=first {
                            let i = yy * self.w + x;
                            self.planes[i] = self.planes[i].saturating_add(1);
                        }
                        let i = first * self.w + x;
                        self.sign[i] = self.decode_sign(mq, x, first);
                        self.sig[i] = 1;
                        self.mag[i] = 1 << plane;
                        y = first + 1;
                    }
                }
                while y < stripe_end {
                    let i = y * self.w + x;
                    if self.visit[i] != 0 || self.sig[i] != 0 {
                        y += 1;
                        continue;
                    }
                    let cx = self.sig_context(x, y, band);
                    self.planes[i] = self.planes[i].saturating_add(1);
                    if mq.bit(&mut self.ctx, cx) == 1 {
                        self.sign[i] = self.decode_sign(mq, x, y);
                        self.sig[i] = 1;
                        self.mag[i] = 1 << plane;
                    }
                    y += 1;
                }
            }
        }
        // The visited flags belong to one bit-plane only.
        for v in self.visit.iter_mut() {
            *v = 0;
        }
    }

    fn decode(&mut self, data: &[u8], band: u8, numbps: u32, npasses: u32) {
        if numbps == 0 || self.w == 0 || self.h == 0 {
            return;
        }
        let mut mq = Mq::new(data);
        let mut plane = numbps as i32 - 1;
        let mut kind = 2u8;
        for _ in 0..npasses {
            if plane < 0 {
                break;
            }
            match kind {
                0 => self.significance_pass(&mut mq, band, plane as u32),
                1 => self.refinement_pass(&mut mq, plane as u32),
                _ => self.cleanup_pass(&mut mq, band, plane as u32),
            }
            if kind == 2 {
                plane -= 1;
                kind = 0;
            } else {
                kind += 1;
            }
        }
    }
}

// ============================================================================
// Inverse wavelet transform
// ============================================================================

const ALPHA: f32 = -1.586_134_3;
const BETA: f32 = -0.052_980_118;
const GAMMA: f32 = 0.882_911_1;
const DELTA: f32 = 0.443_506_85;
const K: f32 = 1.230_174_1;

/// Reflect an out-of-range index back inside `[i0, i1)` about its endpoints.
fn mirror(i: i64, i0: i64, i1: i64) -> usize {
    let span = i1 - i0;
    if span <= 1 {
        return 0;
    }
    let period = 2 * (span - 1);
    let mut t = (i - i0).rem_euclid(period);
    if t >= span {
        t = period - t;
    }
    t as usize
}

/// One-dimensional synthesis over absolute indices `[i0, i1)`.
///
/// `a[k]` holds absolute index `i0 + k`, and even absolute indices are the
/// low-pass half. The lifting steps are the forward ones of T.800 F.4 run
/// backwards; the signal is symmetrically extended first, which is what lets
/// each step read its neighbours without a boundary special case.
fn synthesize(a: &mut [f32], i0: i64, i1: i64, reversible: bool, scratch: &mut Vec<f32>) {
    let n = (i1 - i0) as usize;
    if n == 0 {
        return;
    }
    if n == 1 {
        if i0 % 2 != 0 && reversible {
            a[0] /= 2.0;
        }
        return;
    }
    const PAD: usize = 4;
    scratch.clear();
    scratch.reserve(n + 2 * PAD);
    for k in 0..n + 2 * PAD {
        let idx = i0 - PAD as i64 + k as i64;
        scratch.push(a[mirror(idx, i0, i1)]);
    }
    let base = i0 - PAD as i64;
    let len = scratch.len();
    let even = |k: usize| (base + k as i64).rem_euclid(2) == 0;
    if reversible {
        for k in 1..len - 1 {
            if even(k) {
                scratch[k] -= ((scratch[k - 1] + scratch[k + 1] + 2.0) / 4.0).floor();
            }
        }
        for k in 2..len - 2 {
            if !even(k) {
                scratch[k] += ((scratch[k - 1] + scratch[k + 1]) / 2.0).floor();
            }
        }
    } else {
        for (k, v) in scratch.iter_mut().enumerate() {
            *v *= if (base + k as i64).rem_euclid(2) == 0 {
                K
            } else {
                1.0 / K
            };
        }
        for k in 1..len - 1 {
            if even(k) {
                scratch[k] -= DELTA * (scratch[k - 1] + scratch[k + 1]);
            }
        }
        for k in 2..len - 2 {
            if !even(k) {
                scratch[k] -= GAMMA * (scratch[k - 1] + scratch[k + 1]);
            }
        }
        for k in 3..len - 3 {
            if even(k) {
                scratch[k] -= BETA * (scratch[k - 1] + scratch[k + 1]);
            }
        }
        for k in 4..len - 4 {
            if !even(k) {
                scratch[k] -= ALPHA * (scratch[k - 1] + scratch[k + 1]);
            }
        }
    }
    a[..n].copy_from_slice(&scratch[PAD..PAD + n]);
}

// ============================================================================
// Whole-codestream decode
// ============================================================================

/// One code-block's state, accumulated across the layers that feed it.
#[derive(Default)]
struct CbState {
    rect: Rect,
    included: bool,
    lblock: u32,
    zbp: u32,
    npasses: u32,
    data: Vec<u8>,
}

struct Precinct {
    cw: usize,
    ch: usize,
    blocks: Vec<CbState>,
    incl: TagTree,
    imsb: TagTree,
}

struct BandState {
    kind: u8,
    rect: Rect,
    eps: u8,
    mu: u16,
    coeffs: Vec<f32>,
    precincts: Vec<Precinct>,
}

struct ResState {
    rect: Rect,
    npx: usize,
    npy: usize,
    bands: Vec<BandState>,
}

fn decode_codestream(cs: &[u8]) -> Option<Image> {
    let mut siz: Option<Siz> = None;
    let mut cod: Option<Cod> = None;
    let mut qcd: Option<Qcd> = None;
    let mut coc: Vec<Option<Cod>> = Vec::new();
    let mut qcc: Vec<Option<Qcd>> = Vec::new();
    // Tile index -> concatenated tile-part data.
    let mut tiles: Vec<Vec<u8>> = Vec::new();

    let mut i = 0usize;
    while i + 2 <= cs.len() {
        let marker = be16(cs, i)?;
        if marker == 0xFF4F {
            i += 2;
            continue;
        }
        if marker == 0xFFD9 {
            break;
        }
        if marker < 0xFF00 {
            return None;
        }
        let len = be16(cs, i + 2)? as usize;
        if len < 2 || i + 2 + len > cs.len() {
            return None;
        }
        let seg = &cs[i + 4..i + 2 + len];
        match marker {
            0xFF51 => {
                let s = parse_siz(seg)?;
                if s.xtsiz <= 0 || s.ytsiz <= 0 {
                    return None;
                }
                coc = vec![None; s.comps.len()];
                qcc = vec![None; s.comps.len()];
                let ntx = ceil_div(s.xsiz - s.xtosiz, s.xtsiz).max(0) as usize;
                let nty = ceil_div(s.ysiz - s.ytosiz, s.ytsiz).max(0) as usize;
                if ntx == 0 || nty == 0 || ntx * nty > 65535 {
                    return None;
                }
                tiles = (0..ntx * nty).map(|_| Vec::new()).collect();
                siz = Some(s);
            }
            0xFF52 => cod = parse_cod_body(*seg.first()?, seg.get(1..)?, true),
            0xFF5C => qcd = parse_qcd_body(seg),
            0xFF53 => {
                let ncomp = siz.as_ref()?.comps.len();
                let (c, rest) = if ncomp < 257 {
                    (*seg.first()? as usize, seg.get(1..)?)
                } else {
                    (be16(seg, 0)? as usize, seg.get(2..)?)
                };
                let base = cod.clone()?;
                if let Some(mut over) = parse_cod_body(*rest.first()?, rest.get(1..)?, false)
                    && c < ncomp
                {
                    over.prog = base.prog;
                    over.layers = base.layers;
                    over.mct = base.mct;
                    over.sop = base.sop;
                    over.eph = base.eph;
                    coc[c] = Some(over);
                }
            }
            0xFF5D => {
                let ncomp = siz.as_ref()?.comps.len();
                let (c, rest) = if ncomp < 257 {
                    (*seg.first()? as usize, seg.get(1..)?)
                } else {
                    (be16(seg, 0)? as usize, seg.get(2..)?)
                };
                if c < ncomp {
                    qcc[c] = parse_qcd_body(rest);
                }
            }
            0xFF90 => {
                // A tile-part: its own header markers, SOD, then the data.
                let isot = be16(seg, 0)? as usize;
                let psot = be32(seg, 2)? as usize;
                let part_end = if psot == 0 {
                    cs.len()
                } else {
                    i.saturating_add(psot).min(cs.len())
                };
                let mut j = i + 2 + len;
                while j + 2 <= part_end {
                    let m = be16(cs, j)?;
                    if m == 0xFF93 {
                        j += 2;
                        break;
                    }
                    let l = be16(cs, j + 2)? as usize;
                    if l < 2 {
                        return None;
                    }
                    j += 2 + l;
                }
                if let Some(t) = tiles.get_mut(isot)
                    && j <= part_end
                {
                    t.extend_from_slice(&cs[j..part_end]);
                }
                if part_end <= i {
                    return None;
                }
                i = part_end;
                continue;
            }
            _ => {}
        }
        i += 2 + len;
    }

    let siz = siz?;
    let cod = cod?;
    let qcd = qcd?;
    if cod.cbsty & 0x3F != 0 {
        // Bypass, reset, termall, vertically-causal contexts, predictable
        // termination and segmentation symbols each change the pass
        // structure. None of them appear in files from real producers.
        return None;
    }
    let ncomp = siz.comps.len();
    let img = Rect {
        x0: siz.xosiz,
        y0: siz.yosiz,
        x1: siz.xsiz,
        y1: siz.ysiz,
    };
    if img.w() == 0 || img.h() == 0 || img.area() > 64_000_000 {
        return None;
    }

    // Component sample grids, in component coordinates.
    let comp_rect: Vec<Rect> = siz
        .comps
        .iter()
        .map(|c| Rect {
            x0: ceil_div(img.x0, c.dx as i64),
            y0: ceil_div(img.y0, c.dy as i64),
            x1: ceil_div(img.x1, c.dx as i64),
            y1: ceil_div(img.y1, c.dy as i64),
        })
        .collect();
    let mut planes: Vec<Vec<f32>> = comp_rect.iter().map(|r| vec![0.0; r.area()]).collect();

    let ntx = ceil_div(siz.xsiz - siz.xtosiz, siz.xtsiz).max(0) as usize;
    for (t, data) in tiles.iter().enumerate() {
        if data.is_empty() {
            continue;
        }
        let (tp, tq) = (t % ntx, t / ntx);
        let tile = Rect {
            x0: (siz.xtosiz + tp as i64 * siz.xtsiz).max(img.x0),
            y0: (siz.ytosiz + tq as i64 * siz.ytsiz).max(img.y0),
            x1: (siz.xtosiz + (tp as i64 + 1) * siz.xtsiz).min(img.x1),
            y1: (siz.ytosiz + (tq as i64 + 1) * siz.ytsiz).min(img.y1),
        };
        decode_tile(
            &siz,
            &cod,
            &qcd,
            &coc,
            &qcc,
            tile,
            &comp_rect,
            data,
            &mut planes,
        )?;
    }

    // Component transform, then level shift and interleave to eight bits.
    let out_comps = if ncomp >= 3 { 3 } else { 1 };
    if cod.mct == 1 && ncomp >= 3 && comp_rect[0] == comp_rect[1] && comp_rect[1] == comp_rect[2] {
        let (first, rest) = planes.split_at_mut(1);
        let (second, third) = rest.split_at_mut(1);
        for ((a, b), c) in first[0]
            .iter_mut()
            .zip(second[0].iter_mut())
            .zip(third[0].iter_mut())
        {
            let (y, u, v) = (*a, *b, *c);
            let (r, g, bl) = if cod.reversible {
                let g = y - ((u + v) / 4.0).floor();
                (v + g, g, u + g)
            } else {
                (
                    y + 1.402 * v,
                    y - 0.344_136 * u - 0.714_136 * v,
                    y + 1.772 * u,
                )
            };
            *a = r;
            *b = g;
            *c = bl;
        }
    }

    let (w, h) = (img.w(), img.h());
    let mut out = vec![0u8; w * h * out_comps];
    for c in 0..out_comps {
        let spec = siz.comps[c];
        let rect = comp_rect[c];
        if rect.area() == 0 {
            continue;
        }
        let shift = if spec.signed {
            0.0
        } else {
            (1u32 << (spec.depth - 1)) as f32
        };
        let maxv = ((1u64 << spec.depth) - 1) as f32;
        let scale = 255.0 / maxv;
        for y in 0..h {
            let sy = ((y / spec.dy as usize).min(rect.h() - 1)) * rect.w();
            for x in 0..w {
                let sx = (x / spec.dx as usize).min(rect.w() - 1);
                let v = planes[c][sy + sx] + shift;
                out[(y * w + x) * out_comps + c] = (v.clamp(0.0, maxv) * scale).round() as u8;
            }
        }
    }
    Some(Image {
        width: w as u32,
        height: h as u32,
        components: out_comps,
        data: out,
    })
}

#[allow(clippy::too_many_arguments)]
fn decode_tile(
    siz: &Siz,
    cod: &Cod,
    qcd: &Qcd,
    coc: &[Option<Cod>],
    qcc: &[Option<Qcd>],
    tile: Rect,
    comp_rect: &[Rect],
    data: &[u8],
    planes: &mut [Vec<f32>],
) -> Option<()> {
    let ncomp = siz.comps.len();
    let cods: Vec<Cod> = (0..ncomp)
        .map(|c| coc[c].clone().unwrap_or_else(|| cod.clone()))
        .collect();
    let qcds: Vec<Qcd> = (0..ncomp)
        .map(|c| qcc[c].clone().unwrap_or_else(|| qcd.clone()))
        .collect();

    // Build the resolution / band / precinct skeleton for every component.
    let mut comps: Vec<Vec<ResState>> = Vec::with_capacity(ncomp);
    for c in 0..ncomp {
        let spec = siz.comps[c];
        let tc = Rect {
            x0: ceil_div(tile.x0, spec.dx as i64),
            y0: ceil_div(tile.y0, spec.dy as i64),
            x1: ceil_div(tile.x1, spec.dx as i64),
            y1: ceil_div(tile.y1, spec.dy as i64),
        };
        let cc = &cods[c];
        let qc = &qcds[c];
        let nres = cc.levels as usize + 1;
        let mut resolutions = Vec::with_capacity(nres);
        for r in 0..nres {
            let d = 1i64 << (nres - 1 - r);
            let rect = Rect {
                x0: ceil_div(tc.x0, d),
                y0: ceil_div(tc.y0, d),
                x1: ceil_div(tc.x1, d),
                y1: ceil_div(tc.y1, d),
            };
            let (ppx, ppy) = cc.precincts[r];
            let (px, py) = (1i64 << ppx, 1i64 << ppy);
            let (npx, npy) = if rect.w() == 0 || rect.h() == 0 {
                (0, 0)
            } else {
                (
                    (ceil_div(rect.x1, px) - rect.x0.div_euclid(px)) as usize,
                    (ceil_div(rect.y1, py) - rect.y0.div_euclid(py)) as usize,
                )
            };
            // A code-block never straddles a precinct, and above the lowest
            // resolution the precinct halves when it maps onto the band grid.
            let sub: u8 = u8::from(r > 0);
            let xcb = cc.xcb.min(ppx.saturating_sub(sub)).max(1);
            let ycb = cc.ycb.min(ppy.saturating_sub(sub)).max(1);
            let kinds: &[u8] = if r == 0 { &[0] } else { &[1, 2, 3] };
            let mut bands = Vec::with_capacity(kinds.len());
            for &kind in kinds {
                let brect = if r == 0 {
                    rect
                } else {
                    let bd = 1i64 << (nres - r);
                    let half = bd / 2;
                    let (xob, yob) = band_offsets(kind);
                    Rect {
                        x0: ceil_div(tc.x0 - half * xob, bd),
                        y0: ceil_div(tc.y0 - half * yob, bd),
                        x1: ceil_div(tc.x1 - half * xob, bd),
                        y1: ceil_div(tc.y1 - half * yob, bd),
                    }
                };
                let bi = if r == 0 {
                    0
                } else {
                    3 * (r - 1) + kind as usize
                };
                let (eps, mu) = step_for(qc, cc, bi, r);
                let mut precincts = Vec::with_capacity(npx * npy);
                for pj in 0..npy {
                    for pi in 0..npx {
                        // The precinct's slice of the resolution grid, then
                        // the band coordinates that slice maps onto.
                        let rx0 = (rect.x0.div_euclid(px) + pi as i64) * px;
                        let ry0 = (rect.y0.div_euclid(py) + pj as i64) * py;
                        let s = sub as i64;
                        let pb = Rect {
                            x0: (rx0.max(rect.x0) >> s).max(brect.x0),
                            y0: (ry0.max(rect.y0) >> s).max(brect.y0),
                            x1: ((rx0 + px).min(rect.x1) >> s).min(brect.x1),
                            y1: ((ry0 + py).min(rect.y1) >> s).min(brect.y1),
                        };
                        let (cbx, cby) = (1i64 << xcb, 1i64 << ycb);
                        let (cw, ch) = if pb.w() == 0 || pb.h() == 0 {
                            (0, 0)
                        } else {
                            (
                                (ceil_div(pb.x1, cbx) - pb.x0.div_euclid(cbx)) as usize,
                                (ceil_div(pb.y1, cby) - pb.y0.div_euclid(cby)) as usize,
                            )
                        };
                        let mut blocks = Vec::with_capacity(cw * ch);
                        for bj in 0..ch {
                            for bi2 in 0..cw {
                                let bx0 = (pb.x0.div_euclid(cbx) + bi2 as i64) * cbx;
                                let by0 = (pb.y0.div_euclid(cby) + bj as i64) * cby;
                                blocks.push(CbState {
                                    rect: Rect {
                                        x0: bx0.max(pb.x0),
                                        y0: by0.max(pb.y0),
                                        x1: (bx0 + cbx).min(pb.x1),
                                        y1: (by0 + cby).min(pb.y1),
                                    },
                                    lblock: 3,
                                    ..Default::default()
                                });
                            }
                        }
                        precincts.push(Precinct {
                            cw,
                            ch,
                            blocks,
                            incl: TagTree::new(cw, ch),
                            imsb: TagTree::new(cw, ch),
                        });
                    }
                }
                bands.push(BandState {
                    kind,
                    rect: brect,
                    eps,
                    mu,
                    coeffs: vec![0.0; brect.area()],
                    precincts,
                });
            }
            resolutions.push(ResState {
                rect,
                npx,
                npy,
                bands,
            });
        }
        comps.push(resolutions);
    }

    read_packets(&cod.clone(), &mut comps, data)?;

    for c in 0..ncomp {
        let cc = &cods[c];
        let qc = &qcds[c];
        let spec = siz.comps[c];
        for res in comps[c].iter_mut() {
            for band in res.bands.iter_mut() {
                dequantize(band, cc, qc.guard, spec.depth);
            }
        }

        // Resolution zero's LL band is the starting image; each level folds
        // in the three detail bands above it.
        let mut ll = std::mem::take(&mut comps[c][0].bands[0].coeffs);
        let mut ll_rect = comps[c][0].bands[0].rect;
        let mut scratch = Vec::new();
        for res in comps[c].iter().skip(1) {
            let rect = res.rect;
            let (w, h) = (rect.w(), rect.h());
            let mut a = vec![0.0f32; w * h];
            interleave(&mut a, rect, &ll, ll_rect, 0, 0);
            for band in res.bands.iter() {
                let (xob, yob) = band_offsets(band.kind);
                interleave(&mut a, rect, &band.coeffs, band.rect, xob, yob);
            }
            let mut line = vec![0.0f32; w.max(h)];
            for y in 0..h {
                line[..w].copy_from_slice(&a[y * w..y * w + w]);
                synthesize(
                    &mut line[..w],
                    rect.x0,
                    rect.x1,
                    cc.reversible,
                    &mut scratch,
                );
                a[y * w..y * w + w].copy_from_slice(&line[..w]);
            }
            for x in 0..w {
                for y in 0..h {
                    line[y] = a[y * w + x];
                }
                synthesize(
                    &mut line[..h],
                    rect.y0,
                    rect.y1,
                    cc.reversible,
                    &mut scratch,
                );
                for y in 0..h {
                    a[y * w + x] = line[y];
                }
            }
            ll = a;
            ll_rect = rect;
        }

        // Paste the tile-component into the whole-component plane.
        let dest = comp_rect[c];
        for y in 0..ll_rect.h() {
            let dy = ll_rect.y0 + y as i64 - dest.y0;
            if dy < 0 || dy >= dest.h() as i64 {
                continue;
            }
            for x in 0..ll_rect.w() {
                let dx = ll_rect.x0 + x as i64 - dest.x0;
                if dx < 0 || dx >= dest.w() as i64 {
                    continue;
                }
                planes[c][dy as usize * dest.w() + dx as usize] = ll[y * ll_rect.w() + x];
            }
        }
    }
    Some(())
}

/// Where a band's samples sit on the interleaved grid one resolution up.
fn band_offsets(kind: u8) -> (i64, i64) {
    match kind {
        1 => (1, 0),
        2 => (0, 1),
        3 => (1, 1),
        _ => (0, 0),
    }
}

/// Section F.3.3: scatter one band into the interleaved array it synthesizes
/// from, at every other sample in each direction.
fn interleave(a: &mut [f32], rect: Rect, src: &[f32], srect: Rect, xob: i64, yob: i64) {
    let (w, h) = (rect.w(), rect.h());
    let sw = srect.w();
    for y in 0..srect.h() {
        let ay = 2 * (srect.y0 + y as i64) + yob - rect.y0;
        if ay < 0 || ay >= h as i64 {
            continue;
        }
        for x in 0..sw {
            let ax = 2 * (srect.x0 + x as i64) + xob - rect.x0;
            if ax >= 0 && ax < w as i64 {
                a[ay as usize * w + ax as usize] = src[y * sw + x];
            }
        }
    }
}

/// Run tier-1 over every code-block of a band and write the coefficients.
fn dequantize(band: &mut BandState, cc: &Cod, guard: u8, depth: u8) {
    let gain = match band.kind {
        0 => 0i32,
        3 => 2,
        _ => 1,
    };
    let mb = (guard as i32 + band.eps as i32 - 1).max(0) as u32;
    let delta = if cc.reversible {
        1.0f32
    } else {
        (2.0f32).powi(depth as i32 + gain - band.eps as i32) * (1.0 + band.mu as f32 / 2048.0)
    };
    let bw = band.rect.w();
    let bh = band.rect.h();
    for precinct in band.precincts.iter() {
        for cb in precinct.blocks.iter() {
            if !cb.included || cb.npasses == 0 || cb.rect.area() == 0 {
                continue;
            }
            let (w, h) = (cb.rect.w(), cb.rect.h());
            let numbps = mb.saturating_sub(cb.zbp);
            let mut blk = Block::new(w, h);
            blk.decode(&cb.data, band.kind, numbps, cb.npasses);
            for y in 0..h {
                let py = (cb.rect.y0 - band.rect.y0) as usize + y;
                if py >= bh {
                    break;
                }
                for x in 0..w {
                    let i = y * w + x;
                    let n = blk.mag[i];
                    if n == 0 {
                        continue;
                    }
                    // The magnitude already sits at its true scale: each
                    // bit was written at the plane it was decoded on. What
                    // is unknown is everything below the last plane this
                    // coefficient reached, so reconstruct at the middle of
                    // the interval those missing bits span.
                    let last = numbps.saturating_sub(blk.planes[i] as u32);
                    let mut v = n as f32;
                    if last > 0 {
                        v += (1u32 << (last - 1)) as f32;
                    } else if !cc.reversible {
                        v += 0.5;
                    }
                    v *= delta;
                    if blk.sign[i] != 0 {
                        v = -v;
                    }
                    let px = (cb.rect.x0 - band.rect.x0) as usize + x;
                    if px < bw {
                        band.coeffs[py * bw + px] = v;
                    }
                }
            }
        }
    }
}

/// The quantization step for one band, expounded or derived.
fn step_for(qc: &Qcd, cc: &Cod, band_index: usize, r: usize) -> (u8, u16) {
    if qc.style == 1 {
        // Derived: one value for the whole component, shifted by the level
        // the band sits on.
        let (e0, mu) = qc.steps[0];
        let nb = if r == 0 { 0 } else { r - 1 };
        let eps = (e0 as i32 - cc.levels as i32 + nb as i32 + 1).clamp(0, 31) as u8;
        return (eps, mu);
    }
    *qc.steps.get(band_index).unwrap_or(&qc.steps[0])
}

/// Walk the packets in progression order, filling code-block data.
fn read_packets(cod: &Cod, comps: &mut [Vec<ResState>], data: &[u8]) -> Option<()> {
    let ncomp = comps.len();
    let maxres = comps.iter().map(|r| r.len()).max().unwrap_or(0);
    let layers = cod.layers as usize;
    let prec: Vec<usize> = (0..maxres)
        .map(|r| {
            comps
                .iter()
                .filter_map(|c| c.get(r))
                .map(|res| res.npx * res.npy)
                .max()
                .unwrap_or(0)
        })
        .collect();
    let mut order: Vec<(usize, usize, usize, usize)> = Vec::new();
    match cod.prog {
        // Layer-resolution-component-position.
        0 => {
            for l in 0..layers {
                for (r, &np) in prec.iter().enumerate() {
                    for c in 0..ncomp {
                        for p in 0..np {
                            order.push((l, r, c, p));
                        }
                    }
                }
            }
        }
        // Resolution-layer-component-position.
        1 => {
            for (r, &np) in prec.iter().enumerate() {
                for l in 0..layers {
                    for c in 0..ncomp {
                        for p in 0..np {
                            order.push((l, r, c, p));
                        }
                    }
                }
            }
        }
        // Resolution-position-component-layer.
        2 => {
            for (r, &np) in prec.iter().enumerate() {
                for p in 0..np {
                    for c in 0..ncomp {
                        for l in 0..layers {
                            order.push((l, r, c, p));
                        }
                    }
                }
            }
        }
        // The position- and component-major orders need a walk over the
        // image grid that no file from a real producer asks for.
        _ => return None,
    }

    let mut pos = 0usize;
    for (l, r, c, p) in order {
        if pos >= data.len() {
            break;
        }
        let Some(res) = comps[c].get(r) else { continue };
        if p >= res.npx * res.npy {
            continue;
        }
        if cod.sop && data[pos..].starts_with(&[0xFF, 0x91]) {
            pos += 6;
        }
        pos = read_packet(comps, c, r, p, l, data, pos, cod.eph)?;
    }
    Some(())
}

#[allow(clippy::too_many_arguments)]
fn read_packet(
    comps: &mut [Vec<ResState>],
    c: usize,
    r: usize,
    p: usize,
    layer: usize,
    data: &[u8],
    pos: usize,
    eph: bool,
) -> Option<usize> {
    let mut bits = Bits::new(data, pos);
    let nonempty = bits.bit() == 1;
    // (band, block, length, passes)
    let mut segments: Vec<(usize, usize, usize, u32)> = Vec::new();
    if nonempty {
        for b in 0..comps[c][r].bands.len() {
            let precinct = &mut comps[c][r].bands[b].precincts[p];
            let (cw, ch) = (precinct.cw, precinct.ch);
            for k in 0..cw * ch {
                let (bx, by) = (k % cw, k / cw);
                let included = if precinct.blocks[k].included {
                    bits.bit() == 1
                } else {
                    precinct.incl.decode(&mut bits, bx, by, layer as u32 + 1) <= layer as u32
                };
                if !included {
                    continue;
                }
                if !precinct.blocks[k].included {
                    // The zero bit-planes come from a tag tree read against
                    // rising thresholds until the value is pinned down.
                    let mut t = 1u32;
                    let zbp = loop {
                        let v = precinct.imsb.decode(&mut bits, bx, by, t);
                        if v < t {
                            break v;
                        }
                        t += 1;
                        if t > 74 {
                            return None;
                        }
                    };
                    precinct.blocks[k].zbp = zbp;
                    precinct.blocks[k].included = true;
                }
                let npasses = read_pass_count(&mut bits);
                let mut lblock = precinct.blocks[k].lblock;
                while bits.bit() == 1 {
                    lblock += 1;
                    if lblock > 32 {
                        return None;
                    }
                }
                precinct.blocks[k].lblock = lblock;
                let nbits = lblock + 31 - npasses.leading_zeros();
                let len = bits.bits(nbits) as usize;
                segments.push((b, k, len, npasses));
            }
        }
    }
    bits.align();
    let mut body = bits.pos;
    if eph && data.get(body..body + 2) == Some(&[0xFF, 0x92][..]) {
        body += 2;
    }
    for (b, k, len, npasses) in segments {
        let end = body.saturating_add(len).min(data.len());
        let cb = &mut comps[c][r].bands[b].precincts[p].blocks[k];
        cb.data.extend_from_slice(data.get(body..end)?);
        cb.npasses += npasses;
        body = end;
    }
    Some(body)
}

/// The number of coding passes this layer contributes, in its variable code.
fn read_pass_count(bits: &mut Bits) -> u32 {
    if bits.bit() == 0 {
        return 1;
    }
    if bits.bit() == 0 {
        return 2;
    }
    let v = bits.bits(2);
    if v < 3 {
        return 3 + v;
    }
    let v = bits.bits(5);
    if v < 31 {
        return 6 + v;
    }
    37 + bits.bits(7)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_mirrored_index_reflects_without_repeating_the_edge() {
        // [0, 5): ... 2 1 | 0 1 2 3 4 | 3 2 ...
        assert_eq!(mirror(-1, 0, 5), 1);
        assert_eq!(mirror(-2, 0, 5), 2);
        assert_eq!(mirror(0, 0, 5), 0);
        assert_eq!(mirror(4, 0, 5), 4);
        assert_eq!(mirror(5, 0, 5), 3);
        assert_eq!(mirror(6, 0, 5), 2);
    }

    /// The forward transform, for round-tripping only: the same lifting
    /// steps in the opposite order, so a failure points at the synthesis
    /// rather than at a hand-computed expectation.
    fn analyze(a: &mut [f32], i0: i64, i1: i64, reversible: bool) {
        const PAD: usize = 4;
        let n = (i1 - i0) as usize;
        let mut y: Vec<f32> = (0..n + 2 * PAD)
            .map(|k| a[mirror(i0 - PAD as i64 + k as i64, i0, i1)])
            .collect();
        let base = i0 - PAD as i64;
        let len = y.len();
        let even = |k: usize| (base + k as i64).rem_euclid(2) == 0;
        if reversible {
            for k in 1..len - 1 {
                if !even(k) {
                    y[k] -= ((y[k - 1] + y[k + 1]) / 2.0).floor();
                }
            }
            for k in 2..len - 2 {
                if even(k) {
                    y[k] += ((y[k - 1] + y[k + 1] + 2.0) / 4.0).floor();
                }
            }
        } else {
            for k in 1..len - 1 {
                if !even(k) {
                    y[k] += ALPHA * (y[k - 1] + y[k + 1]);
                }
            }
            for k in 2..len - 2 {
                if even(k) {
                    y[k] += BETA * (y[k - 1] + y[k + 1]);
                }
            }
            for k in 3..len - 3 {
                if !even(k) {
                    y[k] += GAMMA * (y[k - 1] + y[k + 1]);
                }
            }
            for k in 4..len - 4 {
                if even(k) {
                    y[k] += DELTA * (y[k - 1] + y[k + 1]);
                }
            }
            for (k, v) in y.iter_mut().enumerate() {
                *v *= if (base + k as i64).rem_euclid(2) == 0 {
                    1.0 / K
                } else {
                    K
                };
            }
        }
        a[..n].copy_from_slice(&y[PAD..PAD + n]);
    }

    /// Analysis then synthesis returns the signal it started from. This is
    /// the check that the lifting steps, their order, their signs and the
    /// even/odd parity all agree with the standard.
    #[test]
    fn the_wavelet_round_trips() {
        for reversible in [true, false] {
            for (i0, i1) in [(0i64, 16i64), (1, 17), (3, 12), (0, 2)] {
                let n = (i1 - i0) as usize;
                let want: Vec<f32> = (0..n).map(|i| ((i * 37) % 91) as f32 - 45.0).collect();
                let mut a = want.clone();
                analyze(&mut a, i0, i1, reversible);
                let mut scratch = Vec::new();
                synthesize(&mut a, i0, i1, reversible, &mut scratch);
                for (got, want) in a.iter().zip(&want) {
                    assert!(
                        (got - want).abs() < 1e-2,
                        "reversible={reversible} [{i0},{i1}): got {got} want {want}"
                    );
                }
            }
        }
    }

    #[test]
    fn a_tag_tree_reads_back_the_value_it_was_coded_with() {
        // A single-leaf tree: three zero bits then a one codes the value 3.
        let bytes = [0b0001_0000u8];
        let mut bits = Bits::new(&bytes, 0);
        let mut tree = TagTree::new(1, 1);
        assert_eq!(tree.decode(&mut bits, 0, 0, 32), 3);
    }

    #[test]
    fn garbage_is_rejected_rather_than_guessed_at() {
        assert!(decode(b"not a jpeg 2000 file at all").is_none());
        assert!(decode(&[0xFF, 0x4F, 0xFF, 0x51]).is_none());
    }
}
