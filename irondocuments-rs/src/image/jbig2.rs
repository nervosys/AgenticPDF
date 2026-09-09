//! JBIG2 (ITU-T T.88) decoding, in the profile PDF embeds.
//!
//! Scanned pages arrive as mixed raster content: a photographic background
//! and foreground in JPEG or JPEG 2000, and a bilevel JBIG2 stencil saying
//! which of the two shows through at each pixel. Without the stencil the
//! foreground paints as an opaque rectangle and hides the page.
//!
//! What is here is the arithmetic path -- generic regions, generic
//! refinement, symbol dictionaries and text regions -- which is what every
//! scanner and every PDF producer emits. The Huffman path exists in the
//! standard and not in the wild; it is declined rather than guessed at.

use super::mq::Mq;

/// A bilevel image, one byte per pixel, `1` where the bit is set.
pub struct Bitmap {
    pub w: usize,
    pub h: usize,
    pub bits: Vec<u8>,
}

impl Bitmap {
    fn new(w: usize, h: usize, fill: u8) -> Bitmap {
        Bitmap {
            w,
            h,
            bits: vec![fill; w * h],
        }
    }

    fn get(&self, x: i64, y: i64) -> u8 {
        if x < 0 || y < 0 || x >= self.w as i64 || y >= self.h as i64 {
            return 0;
        }
        self.bits[y as usize * self.w + x as usize]
    }

    /// Combine `src` into this bitmap with its top-left corner at `(x, y)`.
    fn compose(&mut self, src: &Bitmap, x: i64, y: i64, op: u8) {
        for sy in 0..src.h {
            let dy = y + sy as i64;
            if dy < 0 || dy >= self.h as i64 {
                continue;
            }
            for sx in 0..src.w {
                let dx = x + sx as i64;
                if dx < 0 || dx >= self.w as i64 {
                    continue;
                }
                let i = dy as usize * self.w + dx as usize;
                let s = src.bits[sy * src.w + sx];
                self.bits[i] = match op {
                    0 => self.bits[i] | s,
                    1 => self.bits[i] & s,
                    2 => self.bits[i] ^ s,
                    3 => !(self.bits[i] ^ s) & 1,
                    _ => s,
                };
            }
        }
    }
}

/// Decode an embedded JBIG2 image of the given size.
///
/// `globals` is the `/JBIG2Globals` stream, empty when there is none. The
/// PDF gives the page size, because an embedded stream's page information
/// segment is allowed to leave the height unknown.
pub fn decode_embedded(globals: &[u8], data: &[u8], w: usize, h: usize) -> Option<Bitmap> {
    if w == 0 || h == 0 || w.checked_mul(h)? > 64_000_000 {
        return None;
    }
    let mut state = Decoder {
        page: Bitmap::new(w, h, 0),
        symbols: Vec::new(),
        page_default_op: 0,
    };
    state.run(globals);
    state.run(data);
    Some(state.page)
}

struct Segment<'a> {
    number: u32,
    kind: u8,
    referred: Vec<u32>,
    data: &'a [u8],
}

/// Symbols exported by one dictionary segment.
struct SymbolSet {
    segment: u32,
    symbols: Vec<Bitmap>,
}

struct Decoder {
    page: Bitmap,
    symbols: Vec<SymbolSet>,
    page_default_op: u8,
}

impl Decoder {
    fn run(&mut self, data: &[u8]) {
        let mut off = 0usize;
        while let Some((segment, next)) = parse_segment(data, off) {
            match segment.kind {
                // Symbol dictionary.
                0 => {
                    if let Some(symbols) = self.symbol_dictionary(&segment) {
                        self.symbols.push(SymbolSet {
                            segment: segment.number,
                            symbols,
                        });
                    }
                }
                // Text region, intermediate or immediate.
                4 | 6 | 7 => self.text_region(&segment),
                // Generic region, intermediate or immediate.
                36 | 38 | 39 => self.generic_region(&segment),
                // Page information.
                48 => {
                    if let Some(&flags) = segment.data.get(16) {
                        let default = (flags >> 2) & 1;
                        if default != 0 {
                            self.page.bits.fill(1);
                        }
                        self.page_default_op = (flags >> 3) & 3;
                    }
                }
                _ => {}
            }
            if next <= off {
                return;
            }
            off = next;
        }
    }

    /// Every symbol this segment may refer to, in referred-to order.
    fn input_symbols(&self, segment: &Segment) -> Vec<&Bitmap> {
        let mut out = Vec::new();
        for &r in &segment.referred {
            if let Some(set) = self.symbols.iter().find(|s| s.segment == r) {
                out.extend(set.symbols.iter());
            }
        }
        out
    }

    fn generic_region(&mut self, segment: &Segment) {
        let Some(info) = region_info(segment.data) else {
            return;
        };
        let body = &segment.data[17..];
        let flags = match body.first() {
            Some(&f) => f,
            None => return,
        };
        if flags & 1 != 0 {
            // MMR-coded: the fax coder, which lives in `ccitt`, but no
            // producer of scanned pages reaches for it here.
            return;
        }
        let template = (flags >> 1) & 3;
        let tpgdon = (flags >> 3) & 1 != 0;
        let nat = if template == 0 { 4 } else { 1 };
        let mut at = [(0i8, 0i8); 4];
        for (k, slot) in at.iter_mut().enumerate().take(nat) {
            let Some(&x) = body.get(1 + 2 * k) else {
                return;
            };
            let Some(&y) = body.get(2 + 2 * k) else {
                return;
            };
            *slot = (x as i8, y as i8);
        }
        let start = 1 + 2 * nat;
        let Some(coded) = body.get(start..) else {
            return;
        };
        let mut mq = Mq::new(coded);
        let mut ctx = vec![0u8; 1 << 16];
        let Some(bitmap) = decode_generic(
            &mut mq, &mut ctx, info.w, info.h, template, &at, tpgdon, None,
        ) else {
            return;
        };
        self.page
            .compose(&bitmap, info.x as i64, info.y as i64, info.op);
    }

    fn symbol_dictionary(&mut self, segment: &Segment) -> Option<Vec<Bitmap>> {
        let d = segment.data;
        let flags = u16::from_be_bytes(d.get(..2)?.try_into().ok()?);
        if flags & 1 != 0 {
            // Huffman-coded symbol dictionary; see the module note.
            return None;
        }
        let refagg = (flags >> 1) & 1 != 0;
        let template = ((flags >> 10) & 3) as u8;
        let rtemplate = ((flags >> 12) & 1) as u8;
        let mut off = 2usize;
        let nat = if template == 0 { 4 } else { 1 };
        let mut at = [(0i8, 0i8); 4];
        for slot in at.iter_mut().take(nat) {
            *slot = (*d.get(off)? as i8, *d.get(off + 1)? as i8);
            off += 2;
        }
        let mut rat = [(0i8, 0i8); 2];
        if refagg && rtemplate == 0 {
            for slot in rat.iter_mut() {
                *slot = (*d.get(off)? as i8, *d.get(off + 1)? as i8);
                off += 2;
            }
        }
        let nexported = u32::from_be_bytes(d.get(off..off + 4)?.try_into().ok()?) as usize;
        let nnew = u32::from_be_bytes(d.get(off + 4..off + 8)?.try_into().ok()?) as usize;
        off += 8;
        if nnew > 100_000 || nexported > 100_000 {
            return None;
        }

        let input: Vec<Bitmap> = self
            .input_symbols(segment)
            .into_iter()
            .map(|b| Bitmap {
                w: b.w,
                h: b.h,
                bits: b.bits.clone(),
            })
            .collect();
        let total = input.len() + nnew;
        let code_len = code_length(total);

        let mut mq = Mq::new(d.get(off..)?);
        let mut arith = Arith::new(code_len);
        let mut gb = vec![0u8; 1 << 16];
        let mut gr = vec![0u8; 1 << 13];

        let mut new_symbols: Vec<Bitmap> = Vec::with_capacity(nnew);
        let mut height = 0i32;
        while new_symbols.len() < nnew {
            let dh = arith.int(&mut mq, IADH)?;
            height += dh;
            if height <= 0 || height > 10_000 {
                return None;
            }
            let mut width = 0i32;
            // An out-of-band width ends the height class.
            while let Some(dw) = arith.int(&mut mq, IADW) {
                width += dw;
                if width <= 0 || width > 10_000 || new_symbols.len() >= nnew {
                    return None;
                }
                let (w, h) = (width as usize, height as usize);
                let bitmap = if !refagg {
                    decode_generic(&mut mq, &mut gb, w, h, template, &at, false, None)?
                } else {
                    let instances = arith.int(&mut mq, IAAI)?;
                    if instances == 1 {
                        // One instance is a plain refinement of an existing
                        // symbol, which T.88 6.5.8.2.2 spells out directly
                        // rather than going through the text region.
                        let id = arith.iaid(&mut mq) as usize;
                        let rdx = arith.int(&mut mq, IARDX)?;
                        let rdy = arith.int(&mut mq, IARDY)?;
                        let reference = input
                            .get(id)
                            .or_else(|| new_symbols.get(id.wrapping_sub(input.len())))?;
                        decode_refinement(
                            &mut mq, &mut gr, w, h, rtemplate, reference, rdx, rdy, &rat,
                        )?
                    } else {
                        // More than one: the symbol is itself a little text
                        // region placing symbols already known, decoded with
                        // the parameters 6.5.8.2 fixes -- one strip, top-left
                        // corner, OR, no offset -- and sharing this coder.
                        if instances < 0 {
                            return None;
                        }
                        let known: Vec<&Bitmap> = input.iter().chain(new_symbols.iter()).collect();
                        let params = TextParams {
                            instances: instances as u32,
                            log_strips: 0,
                            corner: 1,
                            transposed: false,
                            comb_op: 0,
                            default_pixel: 0,
                            ds_offset: 0,
                            refine: true,
                            rtemplate,
                            rat,
                        };
                        decode_text(&mut mq, &mut arith, &mut gr, &known, &params, w, h)?
                    }
                };
                new_symbols.push(bitmap);
            }
        }

        // Export flags select, by runs, which of the input and new symbols
        // this dictionary passes on.
        let mut all: Vec<Bitmap> = input;
        all.extend(new_symbols);
        let mut exported = Vec::with_capacity(nexported);
        let mut index = 0usize;
        let mut current = false;
        while index < all.len() && exported.len() < nexported {
            let run = arith.int(&mut mq, IAEX)?;
            if run < 0 {
                return None;
            }
            let run = run as usize;
            if current {
                for b in all.iter().skip(index).take(run) {
                    exported.push(Bitmap {
                        w: b.w,
                        h: b.h,
                        bits: b.bits.clone(),
                    });
                }
            }
            index += run;
            current = !current;
            if run == 0 && index >= all.len() {
                break;
            }
        }
        Some(exported)
    }

    fn text_region(&mut self, segment: &Segment) {
        let Some(info) = region_info(segment.data) else {
            return;
        };
        let d = segment.data;
        let Some(flags) = d
            .get(17..19)
            .and_then(|b| b.try_into().ok())
            .map(u16::from_be_bytes)
        else {
            return;
        };
        if flags & 1 != 0 {
            // Huffman-coded text region; see the module note.
            return;
        }
        let mut params = TextParams {
            instances: 0,
            log_strips: ((flags >> 2) & 3) as u32,
            corner: ((flags >> 4) & 3) as u8,
            transposed: (flags >> 6) & 1 != 0,
            comb_op: ((flags >> 7) & 3) as u8,
            default_pixel: ((flags >> 9) & 1) as u8,
            // A five-bit signed field.
            ds_offset: {
                let v = ((flags >> 10) & 0x1F) as i32;
                if v > 15 { v - 32 } else { v }
            },
            refine: (flags >> 1) & 1 != 0,
            rtemplate: ((flags >> 15) & 1) as u8,
            rat: [(0i8, 0i8); 2],
        };
        let mut off = 19usize;
        if params.refine && params.rtemplate == 0 {
            for slot in params.rat.iter_mut() {
                let (Some(&x), Some(&y)) = (d.get(off), d.get(off + 1)) else {
                    return;
                };
                *slot = (x as i8, y as i8);
                off += 2;
            }
        }
        let Some(instances) = d
            .get(off..off + 4)
            .and_then(|b| b.try_into().ok())
            .map(u32::from_be_bytes)
        else {
            return;
        };
        off += 4;
        if instances > 1_000_000 {
            return;
        }
        params.instances = instances;

        let symbols = self.input_symbols(segment);
        if symbols.is_empty() {
            return;
        }
        let Some(coded) = d.get(off..) else { return };
        let mut mq = Mq::new(coded);
        let mut arith = Arith::new(code_length(symbols.len()));
        let mut gr = vec![0u8; 1 << 13];
        let Some(region) = decode_text(
            &mut mq, &mut arith, &mut gr, &symbols, &params, info.w, info.h,
        ) else {
            return;
        };
        self.page
            .compose(&region, info.x as i64, info.y as i64, info.op);
    }
}

/// Everything the text region decoding procedure needs beyond the arithmetic
/// state, the symbols and the size.
///
/// A struct because a symbol dictionary invokes the same procedure for an
/// aggregate symbol (T.88 6.5.8.2) with its own fixed set of these, and two
/// callers passing ten positional arguments is a bug waiting to happen.
struct TextParams {
    instances: u32,
    log_strips: u32,
    corner: u8,
    transposed: bool,
    comb_op: u8,
    default_pixel: u8,
    ds_offset: i32,
    refine: bool,
    rtemplate: u8,
    rat: [(i8, i8); 2],
}

/// The text region decoding procedure (T.88 6.4).
///
/// The arithmetic decoder and the integer contexts are borrowed rather than
/// made here: an aggregate symbol is decoded in the middle of a dictionary and
/// has to carry on with the same coder and the same adaptive state, not start
/// a fresh one.
fn decode_text(
    mq: &mut Mq,
    arith: &mut Arith,
    gr: &mut [u8],
    symbols: &[&Bitmap],
    p: &TextParams,
    w: usize,
    h: usize,
) -> Option<Bitmap> {
    let strips = 1i32 << p.log_strips;
    let mut region = Bitmap::new(w, h, p.default_pixel);
    let first = arith.int(mq, IADT)?;
    let mut stript = -first * strips;
    let mut firsts = 0i32;
    let mut placed = 0u32;
    while placed < p.instances {
        let dt = arith.int(mq, IADT)?;
        stript += dt * strips;
        let dfs = arith.int(mq, IAFS)?;
        firsts += dfs;
        let mut curs = firsts;
        let mut first_of_strip = true;
        loop {
            if !first_of_strip {
                match arith.int(mq, IADS) {
                    Some(ids) => curs += ids + p.ds_offset,
                    None => break, // out of band: the strip ends
                }
            }
            first_of_strip = false;
            if placed >= p.instances {
                return Some(region);
            }
            let curt = if strips == 1 { 0 } else { arith.int(mq, IAIT)? };
            let t = stript + curt;
            let id = arith.iaid(mq) as usize;
            let symbol = symbols.get(id)?;
            let mut refined: Option<Bitmap> = None;
            if p.refine {
                let ri = arith.int(mq, IARI)?;
                if ri != 0 {
                    let rdw = arith.int(mq, IARDW)?;
                    let rdh = arith.int(mq, IARDH)?;
                    let rdx = arith.int(mq, IARDX)?;
                    let rdy = arith.int(mq, IARDY)?;
                    let rw = symbol.w as i32 + rdw;
                    let rh = symbol.h as i32 + rdh;
                    if rw <= 0 || rh <= 0 || rw > 10_000 || rh > 10_000 {
                        return None;
                    }
                    refined = Some(decode_refinement(
                        mq,
                        gr,
                        rw as usize,
                        rh as usize,
                        p.rtemplate,
                        symbol,
                        rdw.div_euclid(2) + rdx,
                        rdh.div_euclid(2) + rdy,
                        &p.rat,
                    )?);
                }
            }
            let bitmap: &Bitmap = refined.as_ref().unwrap_or(symbol);
            let (bw, bh) = (bitmap.w as i32, bitmap.h as i32);
            let (x, y) = if p.transposed {
                let x = if p.corner == 2 || p.corner == 3 {
                    t - bw + 1
                } else {
                    t
                };
                (x, curs)
            } else {
                let y = if p.corner == 0 || p.corner == 2 {
                    t - bh + 1
                } else {
                    t
                };
                (curs, y)
            };
            region.compose(bitmap, x as i64, y as i64, p.comb_op);
            curs += if p.transposed { bh - 1 } else { bw - 1 };
            placed += 1;
        }
    }
    Some(region)
}

struct RegionInfo {
    w: usize,
    h: usize,
    x: u32,
    y: u32,
    op: u8,
}

fn region_info(d: &[u8]) -> Option<RegionInfo> {
    let n =
        |i: usize| -> Option<u32> { Some(u32::from_be_bytes(d.get(i..i + 4)?.try_into().ok()?)) };
    let (w, h) = (n(0)? as usize, n(4)? as usize);
    if w == 0 || h == 0 || w.checked_mul(h)? > 64_000_000 {
        return None;
    }
    Some(RegionInfo {
        w,
        h,
        x: n(8)?,
        y: n(12)?,
        op: *d.get(16)? & 7,
    })
}

/// Parse one segment header and return the segment with the next offset.
fn parse_segment(d: &[u8], off: usize) -> Option<(Segment<'_>, usize)> {
    let number = u32::from_be_bytes(d.get(off..off + 4)?.try_into().ok()?);
    let flags = *d.get(off + 4)?;
    let kind = flags & 0x3F;
    let page_assoc = if flags & 0x40 != 0 { 4 } else { 1 };
    let mut i = off + 5;
    let rt = *d.get(i)?;
    let count = (rt >> 5) as usize;
    let count = if count == 7 {
        let long = u32::from_be_bytes(d.get(i..i + 4)?.try_into().ok()?) & 0x1FFF_FFFF;
        let n = long as usize;
        if n > 1_000_000 {
            return None;
        }
        i += 4 + n.div_ceil(8) + 1;
        n
    } else {
        i += 1;
        count
    };
    let ref_size = if number <= 256 {
        1
    } else if number <= 65536 {
        2
    } else {
        4
    };
    let mut referred = Vec::with_capacity(count);
    for k in 0..count {
        let at = i + k * ref_size;
        let v = match ref_size {
            1 => *d.get(at)? as u32,
            2 => u16::from_be_bytes(d.get(at..at + 2)?.try_into().ok()?) as u32,
            _ => u32::from_be_bytes(d.get(at..at + 4)?.try_into().ok()?),
        };
        referred.push(v);
    }
    i += count * ref_size + page_assoc;
    let length = u32::from_be_bytes(d.get(i..i + 4)?.try_into().ok()?) as usize;
    i += 4;
    if length == 0xFFFF_FFFF {
        // An unknown-length generic region, which only an encoder writing a
        // stream it cannot seek in produces.
        return None;
    }
    let end = i.checked_add(length)?.min(d.len());
    Some((
        Segment {
            number,
            kind,
            referred,
            data: d.get(i..end)?,
        },
        end,
    ))
}

/// Bits needed to number `n` symbols, at least one.
fn code_length(n: usize) -> u32 {
    let mut bits = 0u32;
    while (1usize << bits) < n {
        bits += 1;
    }
    bits.max(1)
}

// ============================================================================
// Arithmetic integer decoding (T.88 Annex A)
// ============================================================================

const IADH: usize = 0;
const IADW: usize = 1;
const IAEX: usize = 2;
const IAAI: usize = 3;
const IADT: usize = 4;
const IAFS: usize = 5;
const IADS: usize = 6;
const IAIT: usize = 7;
const IARI: usize = 8;
const IARDW: usize = 9;
const IARDH: usize = 10;
const IARDX: usize = 11;
const IARDY: usize = 12;
const NCONTEXTS: usize = 13;

/// The integer and symbol-id decoders, each with its own context bank.
struct Arith {
    banks: Vec<Vec<u8>>,
    id: Vec<u8>,
    code_len: u32,
}

impl Arith {
    fn new(code_len: u32) -> Arith {
        Arith {
            banks: (0..NCONTEXTS).map(|_| vec![0u8; 512]).collect(),
            id: vec![0u8; 1 << (code_len + 1)],
            code_len,
        }
    }

    /// Decode one integer, or `None` for the out-of-band value.
    fn int(&mut self, mq: &mut Mq, which: usize) -> Option<i32> {
        /// One bit, with the context window sliding over the bits so far.
        fn bit(mq: &mut Mq, bank: &mut [u8], prev: &mut usize) -> i64 {
            let b = mq.bit(bank, *prev) as usize;
            *prev = if *prev < 256 {
                (*prev << 1) | b
            } else {
                (((*prev << 1) | b) & 511) | 256
            };
            b as i64
        }
        fn bits(mq: &mut Mq, bank: &mut [u8], prev: &mut usize, n: u32) -> i64 {
            let mut v = 0i64;
            for _ in 0..n {
                v = (v << 1) | bit(mq, bank, prev);
            }
            v
        }
        let bank = &mut self.banks[which];
        let mut prev = 1usize;
        let sign = bit(mq, bank, &mut prev);
        let value = if bit(mq, bank, &mut prev) == 0 {
            bits(mq, bank, &mut prev, 2)
        } else if bit(mq, bank, &mut prev) == 0 {
            bits(mq, bank, &mut prev, 4) + 4
        } else if bit(mq, bank, &mut prev) == 0 {
            bits(mq, bank, &mut prev, 6) + 20
        } else if bit(mq, bank, &mut prev) == 0 {
            bits(mq, bank, &mut prev, 8) + 84
        } else if bit(mq, bank, &mut prev) == 0 {
            bits(mq, bank, &mut prev, 12) + 340
        } else {
            bits(mq, bank, &mut prev, 32) + 4436
        };
        if sign == 1 {
            if value == 0 {
                return None; // out of band
            }
            return i32::try_from(-value).ok();
        }
        i32::try_from(value).ok()
    }

    /// Decode a symbol id: a fixed number of bits down a binary tree.
    fn iaid(&mut self, mq: &mut Mq) -> u32 {
        let mut prev = 1usize;
        for _ in 0..self.code_len {
            let b = mq.bit(&mut self.id, prev);
            prev = (prev << 1) | b as usize;
        }
        (prev - (1 << self.code_len)) as u32
    }
}

// ============================================================================
// Generic region decoding (T.88 6.2)
// ============================================================================

/// Template pixels, most significant bit first, with the adaptive slots
/// already filled in at their nominal positions.
fn template_pixels(template: u8, at: &[(i8, i8); 4]) -> Pixels {
    let a = |k: usize| at[k];
    match template {
        0 => vec![
            a(3),
            (-1, -2),
            (0, -2),
            (1, -2),
            a(2),
            a(1),
            (-2, -1),
            (-1, -1),
            (0, -1),
            (1, -1),
            (2, -1),
            a(0),
            (-4, 0),
            (-3, 0),
            (-2, 0),
            (-1, 0),
        ],
        1 => vec![
            (-1, -2),
            (0, -2),
            (1, -2),
            (2, -2),
            (-2, -1),
            (-1, -1),
            (0, -1),
            (1, -1),
            (2, -1),
            a(0),
            (-3, 0),
            (-2, 0),
            (-1, 0),
        ],
        2 => vec![
            (-1, -2),
            (0, -2),
            (1, -2),
            (-2, -1),
            (-1, -1),
            (0, -1),
            (1, -1),
            a(0),
            (-2, 0),
            (-1, 0),
        ],
        _ => vec![
            (-3, -1),
            (-2, -1),
            (-1, -1),
            (0, -1),
            (1, -1),
            a(0),
            (-4, 0),
            (-3, 0),
            (-2, 0),
            (-1, 0),
        ],
    }
}

/// The pseudo-pixel context that carries the typical-prediction bit.
fn tpgdon_context(template: u8) -> usize {
    match template {
        0 => 0x9B25,
        1 => 0x0795,
        2 => 0x00E5,
        _ => 0x0195,
    }
}

#[allow(clippy::too_many_arguments)]
fn decode_generic(
    mq: &mut Mq,
    ctx: &mut [u8],
    w: usize,
    h: usize,
    template: u8,
    at: &[(i8, i8); 4],
    tpgdon: bool,
    skip: Option<&Bitmap>,
) -> Option<Bitmap> {
    if w == 0 || h == 0 || w.checked_mul(h)? > 64_000_000 {
        return None;
    }
    let pixels = template_pixels(template, at);
    let mut bitmap = Bitmap::new(w, h, 0);
    let mut ltp = false;
    for y in 0..h {
        if tpgdon {
            if mq.bit(ctx, tpgdon_context(template)) == 1 {
                ltp = !ltp;
            }
            if ltp {
                if y > 0 {
                    let (prev, row) = bitmap.bits.split_at_mut(y * w);
                    row[..w].copy_from_slice(&prev[(y - 1) * w..y * w]);
                }
                continue;
            }
        }
        for x in 0..w {
            if let Some(skip) = skip
                && skip.get(x as i64, y as i64) != 0
            {
                continue;
            }
            let mut label = 0usize;
            for &(dx, dy) in &pixels {
                label =
                    (label << 1) | bitmap.get(x as i64 + dx as i64, y as i64 + dy as i64) as usize;
            }
            bitmap.bits[y * w + x] = mq.bit(ctx, label) as u8;
        }
    }
    Some(bitmap)
}

// ============================================================================
// Generic refinement region decoding (T.88 6.3)
// ============================================================================

/// A list of template offsets, most significant bit of the context first.
type Pixels = Vec<(i8, i8)>;

/// `(coding pixels, reference pixels)`, with the adaptive slots filled.
fn refinement_pixels(template: u8, at: &[(i8, i8); 2]) -> (Pixels, Pixels) {
    if template == 0 {
        (
            vec![(0, -1), (1, -1), (-1, 0), at[0]],
            vec![
                (0, -1),
                (1, -1),
                (-1, 0),
                (0, 0),
                (1, 0),
                (-1, 1),
                (0, 1),
                (1, 1),
                at[1],
            ],
        )
    } else {
        (
            vec![(-1, -1), (0, -1), (1, -1), (-1, 0)],
            vec![(0, -1), (-1, 0), (0, 0), (1, 0), (0, 1), (1, 1)],
        )
    }
}

#[allow(clippy::too_many_arguments)]
fn decode_refinement(
    mq: &mut Mq,
    ctx: &mut [u8],
    w: usize,
    h: usize,
    template: u8,
    reference: &Bitmap,
    dx: i32,
    dy: i32,
    at: &[(i8, i8); 2],
) -> Option<Bitmap> {
    if w == 0 || h == 0 || w.checked_mul(h)? > 64_000_000 {
        return None;
    }
    let (coding, refer) = refinement_pixels(template, at);
    let mut bitmap = Bitmap::new(w, h, 0);
    for y in 0..h {
        for x in 0..w {
            let mut label = 0usize;
            for &(cx, cy) in &coding {
                label =
                    (label << 1) | bitmap.get(x as i64 + cx as i64, y as i64 + cy as i64) as usize;
            }
            // The reference bitmap sits at an offset the caller worked out
            // from the size change, so the same feature lines up under it.
            let (rx, ry) = (x as i64 - dx as i64, y as i64 - dy as i64);
            for &(px, py) in &refer {
                label = (label << 1) | reference.get(rx + px as i64, ry + py as i64) as usize;
            }
            bitmap.bits[y * w + x] = mq.bit(ctx, label) as u8;
        }
    }
    Some(bitmap)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_symbol_count_needs_enough_bits_to_number_it() {
        assert_eq!(code_length(1), 1);
        assert_eq!(code_length(2), 1);
        assert_eq!(code_length(3), 2);
        assert_eq!(code_length(256), 8);
        assert_eq!(code_length(257), 9);
    }

    #[test]
    fn composing_off_the_edge_clips_rather_than_panics() {
        let mut page = Bitmap::new(4, 4, 0);
        let mut src = Bitmap::new(3, 3, 1);
        src.bits[0] = 1;
        page.compose(&src, -1, -1, 0);
        page.compose(&src, 3, 3, 0);
        assert_eq!(page.get(0, 0), 1);
        assert_eq!(page.get(3, 3), 1);
        assert_eq!(page.get(2, 0), 0);
    }

    #[test]
    fn garbage_is_rejected_rather_than_guessed_at() {
        // A page with no segments stays as the default: all white.
        let page = decode_embedded(&[], b"not a jbig2 stream", 8, 8).expect("a page");
        assert!(page.bits.iter().all(|&b| b == 0));
    }
}
