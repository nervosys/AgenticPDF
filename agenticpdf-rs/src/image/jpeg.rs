// SPDX-License-Identifier: AGPL-3.0-or-later
//! A baseline JPEG decoder.
//!
//! `DCTDecode` is how a PDF carries a photograph, and there is no way to draw
//! one without decoding it. A browser has a decoder built in and the web shell
//! uses it; a desktop reader has to bring its own, and this crate takes no
//! image-codec dependency for the same reason it hand-rolls its ZIP and its
//! CMaps: the wasm bundle is measured in hundreds of kilobytes, and every
//! dependency is a security surface.
//!
//! Baseline and extended sequential (SOF0/SOF1) are what PDF producers emit.
//! Progressive is a web format and is declined rather than guessed at. Adobe's
//! APP14 transform is honoured, inverted CMYK included, because print-ready
//! PDFs are full of it.

/// A decoded image: 8-bit RGB, row-major, no padding.
pub struct Image {
    pub width: u32,
    pub height: u32,
    /// `width * height * 3` bytes.
    pub rgb: Vec<u8>,
}

/// One component of the frame.
#[derive(Clone, Copy, Default)]
struct Component {
    id: u8,
    /// Sampling factors: how many blocks of this component sit in one MCU.
    h: usize,
    v: usize,
    quant: usize,
    dc_table: usize,
    ac_table: usize,
    /// Running DC predictor: each block's DC is a difference from the last.
    pred: i32,
}

/// A Huffman table as the decoder reads it: canonical codes bounded per
/// length, with the index of the first value of each length.
#[derive(Clone, Default)]
struct Huffman {
    min_code: [i32; 16],
    max_code: [i32; 16],
    val_start: [i32; 16],
    values: Vec<u8>,
}

impl Huffman {
    fn build(counts: &[u8; 16], values: Vec<u8>) -> Huffman {
        let mut table = Huffman {
            values,
            ..Default::default()
        };
        let mut code = 0i32;
        let mut index = 0i32;
        for (length, count) in counts.iter().enumerate() {
            let n = *count as i32;
            if n == 0 {
                // No code of this length. `max_code` under `min_code` says so.
                table.max_code[length] = -1;
            } else {
                table.val_start[length] = index - code;
                table.min_code[length] = code;
                table.max_code[length] = code + n - 1;
                index += n;
                code += n;
            }
            code <<= 1;
        }
        table
    }
}

/// The entropy-coded bit stream, which is byte-stuffed: a 0xFF in the data is
/// written `FF 00`, and any other `FF xx` is a marker and ends the scan.
struct BitReader<'a> {
    data: &'a [u8],
    pos: usize,
    bits: u32,
    count: u32,
    /// Set once a marker is reached. Further reads yield zeroes rather than
    /// running on into the next segment: a truncated file should decode to a
    /// partial picture, not to nothing.
    ended: bool,
}

impl<'a> BitReader<'a> {
    fn new(data: &'a [u8]) -> BitReader<'a> {
        BitReader {
            data,
            pos: 0,
            bits: 0,
            count: 0,
            ended: false,
        }
    }

    fn fill(&mut self) {
        while self.count <= 24 {
            if self.ended || self.pos >= self.data.len() {
                self.count += 8;
                continue;
            }
            let mut byte = self.data[self.pos];
            self.pos += 1;
            if byte == 0xFF {
                let next = self.data.get(self.pos).copied().unwrap_or(0xD9);
                if next == 0x00 {
                    self.pos += 1;
                } else {
                    // A marker, restart included: leave it for `restart` to
                    // consume and stop feeding real bits.
                    self.pos -= 1;
                    self.ended = true;
                    byte = 0;
                }
            }
            self.bits |= (byte as u32) << (24 - self.count);
            self.count += 8;
        }
    }

    fn bit(&mut self) -> u32 {
        if self.count == 0 {
            self.fill();
        }
        let out = self.bits >> 31;
        self.bits <<= 1;
        self.count -= 1;
        out
    }

    fn take(&mut self, n: u32) -> u32 {
        let mut out = 0;
        for _ in 0..n {
            out = (out << 1) | self.bit();
        }
        out
    }

    fn decode(&mut self, table: &Huffman) -> Option<u8> {
        let mut code = self.bit() as i32;
        for length in 0..16 {
            if table.max_code[length] >= code && code >= table.min_code[length] {
                let index = table.val_start[length] + code;
                return table.values.get(index as usize).copied();
            }
            code = (code << 1) | self.bit() as i32;
        }
        None
    }

    /// Skip past the next restart marker and start a fresh bit buffer.
    fn restart(&mut self) {
        self.bits = 0;
        self.count = 0;
        self.ended = false;
        while self.pos + 1 < self.data.len() {
            if self.data[self.pos] == 0xFF && (0xD0..=0xD7).contains(&self.data[self.pos + 1]) {
                self.pos += 2;
                return;
            }
            self.pos += 1;
        }
        self.pos = self.data.len();
    }
}

/// Sign-extend an `n`-bit JPEG coefficient.
fn extend(value: u32, n: u32) -> i32 {
    if n == 0 {
        return 0;
    }
    let v = value as i32;
    if v < (1 << (n - 1)) {
        v - (1 << n) + 1
    } else {
        v
    }
}

/// The order coefficients are stored in: a zig-zag out from the top left.
const ZIGZAG: [usize; 64] = [
    0, 1, 8, 16, 9, 2, 3, 10, 17, 24, 32, 25, 18, 11, 4, 5, 12, 19, 26, 33, 40, 48, 41, 34, 27, 20,
    13, 6, 7, 14, 21, 28, 35, 42, 49, 56, 57, 50, 43, 36, 29, 22, 15, 23, 30, 37, 44, 51, 58, 59,
    52, 45, 38, 31, 39, 46, 53, 60, 61, 54, 47, 55, 62, 63,
];

/// The IDCT basis: `cos((2x+1) u pi / 16) * c(u) / 2`, written out because
/// `cos` cannot be called in a constant.
const COS: [[f32; 8]; 8] = [
    [
        0.353_553_4,
        0.490_392_6,
        0.461_939_8,
        0.415_734_8,
        0.353_553_4,
        0.277_785_1,
        0.191_341_7,
        0.097_545_16,
    ],
    [
        0.353_553_4,
        0.415_734_8,
        0.191_341_7,
        -0.097_545_16,
        -0.353_553_4,
        -0.490_392_6,
        -0.461_939_8,
        -0.277_785_1,
    ],
    [
        0.353_553_4,
        0.277_785_1,
        -0.191_341_7,
        -0.490_392_6,
        -0.353_553_4,
        0.097_545_16,
        0.461_939_8,
        0.415_734_8,
    ],
    [
        0.353_553_4,
        0.097_545_16,
        -0.461_939_8,
        -0.277_785_1,
        0.353_553_4,
        0.415_734_8,
        -0.191_341_7,
        -0.490_392_6,
    ],
    [
        0.353_553_4,
        -0.097_545_16,
        -0.461_939_8,
        0.277_785_1,
        0.353_553_4,
        -0.415_734_8,
        -0.191_341_7,
        0.490_392_6,
    ],
    [
        0.353_553_4,
        -0.277_785_1,
        -0.191_341_7,
        0.490_392_6,
        -0.353_553_4,
        -0.097_545_16,
        0.461_939_8,
        -0.415_734_8,
    ],
    [
        0.353_553_4,
        -0.415_734_8,
        0.191_341_7,
        0.097_545_16,
        -0.353_553_4,
        0.490_392_6,
        -0.461_939_8,
        0.277_785_1,
    ],
    [
        0.353_553_4,
        -0.490_392_6,
        0.461_939_8,
        -0.415_734_8,
        0.353_553_4,
        -0.277_785_1,
        0.191_341_7,
        -0.097_545_16,
    ],
];

/// An 8x8 inverse DCT on dequantised coefficients.
///
/// The plain separable float transform rather than one of the integer
/// approximations: it costs microseconds a block, the result is cached as a
/// texture, and being obviously right is worth more here than being fast.
fn idct(block: &[f32; 64], out: &mut [u8; 64]) {
    let mut tmp = [0f32; 64];
    for y in 0..8 {
        for x in 0..8 {
            let mut sum = 0f32;
            for u in 0..8 {
                sum += COS[x][u] * block[y * 8 + u];
            }
            tmp[y * 8 + x] = sum;
        }
    }
    for x in 0..8 {
        for y in 0..8 {
            let mut sum = 0f32;
            for v in 0..8 {
                sum += COS[y][v] * tmp[v * 8 + x];
            }
            out[y * 8 + x] = (sum + 128.0).clamp(0.0, 255.0) as u8;
        }
    }
}

/// Decode a baseline JPEG to RGB.
///
/// Returns `None` for anything it does not handle -- progressive, arithmetic
/// coding, a truncated header -- so the caller can draw the image's frame
/// rather than the wrong pixels.
pub fn decode(data: &[u8]) -> Option<Image> {
    let mut quant = [[0u16; 64]; 4];
    let mut dc_tables: Vec<Huffman> = vec![Huffman::default(); 4];
    let mut ac_tables: Vec<Huffman> = vec![Huffman::default(); 4];
    let mut components: Vec<Component> = Vec::new();
    let mut width = 0usize;
    let mut height = 0usize;
    let mut restart_interval = 0usize;
    let mut adobe_transform: Option<u8> = None;

    if data.len() < 4 || data[0] != 0xFF || data[1] != 0xD8 {
        return None;
    }
    let mut i = 2usize;

    while i + 1 < data.len() {
        if data[i] != 0xFF {
            i += 1;
            continue;
        }
        let marker = data[i + 1];
        i += 2;
        match marker {
            // Padding and standalone markers carry no payload.
            0xFF | 0x01 | 0xD0..=0xD7 => continue,
            0xD9 => break,
            _ => {}
        }
        if i + 1 >= data.len() {
            return None;
        }
        let length = u16::from_be_bytes([data[i], data[i + 1]]) as usize;
        if length < 2 || i + length > data.len() {
            return None;
        }
        let segment = &data[i + 2..i + length];
        match marker {
            // Quantisation tables.
            0xDB => {
                let mut p = 0;
                while p < segment.len() {
                    let precision = segment[p] >> 4;
                    let id = (segment[p] & 15) as usize;
                    p += 1;
                    if id >= 4 {
                        return None;
                    }
                    for k in ZIGZAG.iter() {
                        let value = if precision == 0 {
                            let v = *segment.get(p)? as u16;
                            p += 1;
                            v
                        } else {
                            let v = u16::from_be_bytes([*segment.get(p)?, *segment.get(p + 1)?]);
                            p += 2;
                            v
                        };
                        quant[id][*k] = value;
                    }
                }
            }
            // Baseline and extended sequential frames.
            0xC0 | 0xC1 => {
                if segment.len() < 6 {
                    return None;
                }
                height = u16::from_be_bytes([segment[1], segment[2]]) as usize;
                width = u16::from_be_bytes([segment[3], segment[4]]) as usize;
                let count = segment[5] as usize;
                if width == 0 || height == 0 || count == 0 || count > 4 {
                    return None;
                }
                components.clear();
                for c in 0..count {
                    let base = 6 + c * 3;
                    let sampling = *segment.get(base + 1)?;
                    components.push(Component {
                        id: *segment.get(base)?,
                        h: (sampling >> 4) as usize,
                        v: (sampling & 15) as usize,
                        quant: *segment.get(base + 2)? as usize,
                        ..Default::default()
                    });
                }
                if components
                    .iter()
                    .any(|c| c.h == 0 || c.v == 0 || c.h > 4 || c.v > 4 || c.quant >= 4)
                {
                    return None;
                }
            }
            // Progressive, lossless and arithmetic frames are not handled.
            0xC2 | 0xC3 | 0xC5..=0xC7 | 0xC9..=0xCB | 0xCD..=0xCF => return None,
            // Huffman tables.
            0xC4 => {
                let mut p = 0;
                while p + 17 <= segment.len() {
                    let class = segment[p] >> 4;
                    let id = (segment[p] & 15) as usize;
                    p += 1;
                    let mut counts = [0u8; 16];
                    counts.copy_from_slice(&segment[p..p + 16]);
                    p += 16;
                    let total: usize = counts.iter().map(|c| *c as usize).sum();
                    if id >= 4 || p + total > segment.len() {
                        return None;
                    }
                    let table = Huffman::build(&counts, segment[p..p + total].to_vec());
                    p += total;
                    if class == 0 {
                        dc_tables[id] = table;
                    } else {
                        ac_tables[id] = table;
                    }
                }
            }
            // Restart interval.
            0xDD => {
                if segment.len() >= 2 {
                    restart_interval = u16::from_be_bytes([segment[0], segment[1]]) as usize;
                }
            }
            // Adobe's marker, which records the colour transform.
            0xEE => {
                if segment.len() >= 12 && &segment[..5] == b"Adobe" {
                    adobe_transform = Some(segment[11]);
                }
            }
            // The scan header, followed by the entropy-coded data itself.
            0xDA => {
                if segment.is_empty() {
                    return None;
                }
                let count = segment[0] as usize;
                for c in 0..count {
                    let id = *segment.get(1 + c * 2)?;
                    let tables = *segment.get(2 + c * 2)?;
                    for comp in components.iter_mut() {
                        if comp.id == id {
                            comp.dc_table = (tables >> 4) as usize;
                            comp.ac_table = (tables & 15) as usize;
                        }
                    }
                }
                if components.is_empty()
                    || components
                        .iter()
                        .any(|c| c.dc_table >= 4 || c.ac_table >= 4)
                {
                    return None;
                }
                return decode_scan(
                    &data[i + length..],
                    &mut components,
                    &quant,
                    &dc_tables,
                    &ac_tables,
                    Frame {
                        width,
                        height,
                        restart_interval,
                        adobe_transform,
                    },
                );
            }
            _ => {}
        }
        i += length;
    }
    None
}

/// What the frame and scan headers said about the picture as a whole.
struct Frame {
    width: usize,
    height: usize,
    restart_interval: usize,
    /// Adobe's APP14 colour transform: 0 none, 1 YCbCr, 2 YCCK.
    adobe_transform: Option<u8>,
}

/// Decode the entropy-coded scan into one plane per component, then upsample
/// and convert to RGB.
fn decode_scan(
    scan: &[u8],
    components: &mut [Component],
    quant: &[[u16; 64]; 4],
    dc_tables: &[Huffman],
    ac_tables: &[Huffman],
    frame: Frame,
) -> Option<Image> {
    let (width, height) = (frame.width, frame.height);
    let hmax = components.iter().map(|c| c.h).max()?;
    let vmax = components.iter().map(|c| c.v).max()?;
    let mcus_x = width.div_ceil(hmax * 8);
    let mcus_y = height.div_ceil(vmax * 8);

    // A plane per component, at that component's own sampling resolution.
    let mut planes: Vec<Vec<u8>> = Vec::new();
    let mut strides: Vec<usize> = Vec::new();
    for comp in components.iter() {
        let stride = mcus_x * comp.h * 8;
        let rows = mcus_y * comp.v * 8;
        if stride.saturating_mul(rows) > 256_000_000 {
            return None;
        }
        planes.push(vec![0u8; stride * rows]);
        strides.push(stride);
    }

    let mut reader = BitReader::new(scan);
    let mut pixels = [0u8; 64];
    let mut since_restart = 0usize;

    for my in 0..mcus_y {
        for mx in 0..mcus_x {
            if frame.restart_interval > 0 && since_restart == frame.restart_interval {
                reader.restart();
                since_restart = 0;
                for comp in components.iter_mut() {
                    comp.pred = 0;
                }
            }
            since_restart += 1;

            for (index, comp) in components.iter_mut().enumerate() {
                for by in 0..comp.v {
                    for bx in 0..comp.h {
                        let mut block = [0f32; 64];
                        let table = &quant[comp.quant];

                        // The DC coefficient is a difference from the previous
                        // block of the same component.
                        let t = reader.decode(&dc_tables[comp.dc_table])?;
                        if t > 15 {
                            return None;
                        }
                        comp.pred += extend(reader.take(t as u32), t as u32);
                        block[0] = comp.pred as f32 * table[0] as f32;

                        // Then runs of zeroes, each followed by a coefficient.
                        let mut k = 1usize;
                        while k < 64 {
                            let rs = reader.decode(&ac_tables[comp.ac_table])?;
                            let run = (rs >> 4) as usize;
                            let size = (rs & 15) as u32;
                            if size == 0 {
                                if run == 15 {
                                    // Sixteen zeroes and no coefficient.
                                    k += 16;
                                    continue;
                                }
                                // End of block: the rest are zero.
                                break;
                            }
                            k += run;
                            if k >= 64 {
                                break;
                            }
                            let at = ZIGZAG[k];
                            block[at] = extend(reader.take(size), size) as f32 * table[at] as f32;
                            k += 1;
                        }

                        idct(&block, &mut pixels);
                        let stride = strides[index];
                        let ox = (mx * comp.h + bx) * 8;
                        let oy = (my * comp.v + by) * 8;
                        for row in 0..8 {
                            let start = (oy + row) * stride + ox;
                            planes[index][start..start + 8]
                                .copy_from_slice(&pixels[row * 8..row * 8 + 8]);
                        }
                    }
                }
            }
        }
    }

    let n = components.len();
    // With no Adobe marker, three components are YCbCr and one is grey.
    let transform = frame.adobe_transform.unwrap_or(u8::from(n == 3));
    let mut rgb = vec![0u8; width * height * 3];
    for y in 0..height {
        for x in 0..width {
            let mut sample = [0u8; 4];
            for (index, comp) in components.iter().enumerate() {
                // Nearest-neighbour upsampling of the subsampled planes.
                let sx = x * comp.h / hmax;
                let sy = y * comp.v / vmax;
                sample[index] = planes[index][sy * strides[index] + sx];
            }
            let at = (y * width + x) * 3;
            let out = &mut rgb[at..at + 3];
            match n {
                1 => out.fill(sample[0]),
                3 => {
                    let (r, g, b) = if transform == 0 {
                        (sample[0], sample[1], sample[2])
                    } else {
                        ycbcr(sample[0], sample[1], sample[2])
                    };
                    out[0] = r;
                    out[1] = g;
                    out[2] = b;
                }
                4 => {
                    // YCCK carries YCbCr in its first three channels; plain
                    // CMYK does not.
                    let (c, m, ye) = if transform == 2 {
                        ycbcr(sample[0], sample[1], sample[2])
                    } else {
                        (sample[0], sample[1], sample[2])
                    };
                    let k = sample[3];
                    // Ink coverage to light: no ink is white, full ink is
                    // black. Decoders are widely said to have to invert
                    // Adobe's CMYK, and this one did until the pictures came
                    // out as negatives -- the print-ready PDFs in the test
                    // corpus store coverage the plain way round. Taking the
                    // samples as coverage is what matches the reference
                    // renderer on them.
                    // After the transform above, the first three channels
                    // hold light rather than ink -- 255 is no colorant --
                    // while the black channel holds coverage, 0 being none.
                    // Reading the black channel the same way round as the
                    // others is what turned every four-channel picture into a
                    // near-black rectangle.
                    let light = 255 - k as u32;
                    out[0] = (c as u32 * light / 255) as u8;
                    out[1] = (m as u32 * light / 255) as u8;
                    out[2] = (ye as u32 * light / 255) as u8;
                }
                _ => return None,
            }
        }
    }

    Some(Image {
        width: width as u32,
        height: height as u32,
        rgb,
    })
}

/// YCbCr to RGB, the JFIF conversion.
fn ycbcr(y: u8, cb: u8, cr: u8) -> (u8, u8, u8) {
    let y = y as f32;
    let cb = cb as f32 - 128.0;
    let cr = cr as f32 - 128.0;
    (
        (y + 1.402 * cr).clamp(0.0, 255.0) as u8,
        (y - 0.344_136 * cb - 0.714_136 * cr).clamp(0.0, 255.0) as u8,
        (y + 1.772 * cb).clamp(0.0, 255.0) as u8,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 16x16 image with four known quadrants -- red, green, blue, white --
    /// encoded by a browser at quality 0.92, so the bytes come from an
    /// encoder this decoder's author had no hand in.
    #[rustfmt::skip]
    const QUADRANTS: &[u8] = &[
        255, 216, 255, 224, 0, 16, 74, 70, 73, 70, 0, 1, 1, 0, 0, 1, 0, 1, 0, 0, 255, 219, 0,
        67, 0, 3, 2, 2, 2, 2, 2, 3, 2, 2, 2, 3, 3, 3, 3, 4, 6, 4, 4, 4, 4, 4, 8, 6, 6, 5, 6, 9,
        8, 10, 10, 9, 8, 9, 9, 10, 12, 15, 12, 10, 11, 14, 11, 9, 9, 13, 17, 13, 14, 15, 16, 16,
        17, 16, 10, 12, 18, 19, 18, 16, 19, 15, 16, 16, 16, 255, 219, 0, 67, 1, 3, 3, 3, 4, 3,
        4, 8, 4, 4, 8, 16, 11, 9, 11, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16,
        16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16,
        16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 255, 192, 0, 17, 8, 0, 16, 0,
        16, 3, 1, 17, 0, 2, 17, 1, 3, 17, 1, 255, 196, 0, 31, 0, 0, 1, 5, 1, 1, 1, 1, 1, 1, 0,
        0, 0, 0, 0, 0, 0, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 255, 196, 0, 181, 16, 0, 2, 1,
        3, 3, 2, 4, 3, 5, 5, 4, 4, 0, 0, 1, 125, 1, 2, 3, 0, 4, 17, 5, 18, 33, 49, 65, 6, 19,
        81, 97, 7, 34, 113, 20, 50, 129, 145, 161, 8, 35, 66, 177, 193, 21, 82, 209, 240, 36,
        51, 98, 114, 130, 9, 10, 22, 23, 24, 25, 26, 37, 38, 39, 40, 41, 42, 52, 53, 54, 55, 56,
        57, 58, 67, 68, 69, 70, 71, 72, 73, 74, 83, 84, 85, 86, 87, 88, 89, 90, 99, 100, 101,
        102, 103, 104, 105, 106, 115, 116, 117, 118, 119, 120, 121, 122, 131, 132, 133, 134,
        135, 136, 137, 138, 146, 147, 148, 149, 150, 151, 152, 153, 154, 162, 163, 164, 165,
        166, 167, 168, 169, 170, 178, 179, 180, 181, 182, 183, 184, 185, 186, 194, 195, 196,
        197, 198, 199, 200, 201, 202, 210, 211, 212, 213, 214, 215, 216, 217, 218, 225, 226,
        227, 228, 229, 230, 231, 232, 233, 234, 241, 242, 243, 244, 245, 246, 247, 248, 249,
        250, 255, 196, 0, 31, 1, 0, 3, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 1, 2, 3, 4,
        5, 6, 7, 8, 9, 10, 11, 255, 196, 0, 181, 17, 0, 2, 1, 2, 4, 4, 3, 4, 7, 5, 4, 4, 0, 1,
        2, 119, 0, 1, 2, 3, 17, 4, 5, 33, 49, 6, 18, 65, 81, 7, 97, 113, 19, 34, 50, 129, 8, 20,
        66, 145, 161, 177, 193, 9, 35, 51, 82, 240, 21, 98, 114, 209, 10, 22, 36, 52, 225, 37,
        241, 23, 24, 25, 26, 38, 39, 40, 41, 42, 53, 54, 55, 56, 57, 58, 67, 68, 69, 70, 71, 72,
        73, 74, 83, 84, 85, 86, 87, 88, 89, 90, 99, 100, 101, 102, 103, 104, 105, 106, 115, 116,
        117, 118, 119, 120, 121, 122, 130, 131, 132, 133, 134, 135, 136, 137, 138, 146, 147,
        148, 149, 150, 151, 152, 153, 154, 162, 163, 164, 165, 166, 167, 168, 169, 170, 178,
        179, 180, 181, 182, 183, 184, 185, 186, 194, 195, 196, 197, 198, 199, 200, 201, 202,
        210, 211, 212, 213, 214, 215, 216, 217, 218, 226, 227, 228, 229, 230, 231, 232, 233,
        234, 242, 243, 244, 245, 246, 247, 248, 249, 250, 255, 218, 0, 12, 3, 1, 0, 2, 17, 3,
        17, 0, 63, 0, 249, 210, 191, 12, 63, 213, 51, 236, 106, 252, 72, 255, 0, 152, 243, 242,
        242, 191, 232, 204, 254, 229, 63, 165, 186, 255, 0, 43, 15, 184, 63, 255, 217
    ];

    #[test]
    fn a_baseline_jpeg_decodes_to_its_colours() {
        let image = decode(QUADRANTS).expect("a baseline JPEG should decode");
        assert_eq!((image.width, image.height), (16, 16));
        assert_eq!(image.rgb.len(), 16 * 16 * 3);

        // Sampled in the middle of each quadrant, away from the edges where
        // the transform rings. JPEG is lossy, so the tolerance is generous:
        // what is checked is that each quadrant is the right colour, not that
        // the codec is exact.
        let at = |x: usize, y: usize| {
            let i = (y * 16 + x) * 3;
            (image.rgb[i], image.rgb[i + 1], image.rgb[i + 2])
        };
        let near = |got: (u8, u8, u8), want: (u8, u8, u8)| {
            let d = |a: u8, b: u8| (a as i32 - b as i32).abs();
            d(got.0, want.0) < 48 && d(got.1, want.1) < 48 && d(got.2, want.2) < 48
        };
        assert!(near(at(3, 3), (255, 0, 0)), "top left red, got {:?}", at(3, 3));
        assert!(near(at(12, 3), (0, 255, 0)), "top right green, got {:?}", at(12, 3));
        assert!(near(at(3, 12), (0, 0, 255)), "bottom left blue, got {:?}", at(3, 12));
        assert!(
            near(at(12, 12), (255, 255, 255)),
            "bottom right white, got {:?}",
            at(12, 12)
        );
    }

    /// Progressive JPEG is declined rather than decoded wrongly, so the caller
    /// can draw the image's frame instead of the wrong pixels.
    #[test]
    fn a_progressive_jpeg_is_declined() {
        let mut data = QUADRANTS.to_vec();
        for i in 0..data.len() - 1 {
            if data[i] == 0xFF && data[i + 1] == 0xC0 {
                data[i + 1] = 0xC2;
                break;
            }
        }
        assert!(decode(&data).is_none());
    }

    /// Nothing here may panic on a damaged file: these bytes come from
    /// documents nobody vouched for.
    #[test]
    fn rubbish_is_declined_rather_than_panicking() {
        assert!(decode(&[]).is_none());
        assert!(decode(&[0xFF, 0xD8, 1, 2, 3, 4, 5]).is_none());
        for cut in [4usize, 40, 200, QUADRANTS.len() / 2] {
            if cut < QUADRANTS.len() {
                let _ = decode(&QUADRANTS[..cut]);
            }
        }
    }
}
