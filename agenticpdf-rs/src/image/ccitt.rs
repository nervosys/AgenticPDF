// SPDX-License-Identifier: AGPL-3.0-or-later
//! CCITT Group 3 and Group 4 fax decoding.
//!
//! This is how a scanned page is stored: one bit per pixel, run lengths
//! Huffman-coded, each row usually described as a set of differences from the
//! row above. It is the most common codec in the scanned documents this reader
//! is asked to open, and without it such a page is a grey frame.
//!
//! The output is one bit per pixel, `1` white and `0` black -- ordinary
//! `DeviceGray` at one bit -- whatever `/BlackIs1` said, since that describes
//! the filter's own output encoding and is normalised away here.

/// What `/DecodeParms` said about the encoding.
pub struct Params {
    /// `/K`: negative is Group 4, zero is Group 3 one-dimensional, positive is
    /// Group 3 with a mode bit per row.
    pub k: i64,
    pub columns: usize,
    pub rows: usize,
    /// `/EncodedByteAlign`: each row starts on a byte boundary.
    pub byte_align: bool,
}

/// A run-length code: the bit pattern, its length, and the run it stands for.
struct Code {
    bits: u16,
    len: u8,
    run: u16,
}

/// Build a table from `(pattern, run)` pairs written as bit strings, which is
/// how the specification prints them and the only form in which a human can
/// check them against it.
fn table(rows: &[(&str, u16)]) -> Vec<Code> {
    rows.iter()
        .map(|(bits, run)| Code {
            bits: u16::from_str_radix(bits, 2).unwrap_or(0),
            len: bits.len() as u8,
            run: *run,
        })
        .collect()
}

const WHITE: &[(&str, u16)] = &[
    // Terminating codes, runs 0-63.
    ("00110101", 0), ("000111", 1), ("0111", 2), ("1000", 3),
    ("1011", 4), ("1100", 5), ("1110", 6), ("1111", 7),
    ("10011", 8), ("10100", 9), ("00111", 10), ("01000", 11),
    ("001000", 12), ("000011", 13), ("110100", 14), ("110101", 15),
    ("101010", 16), ("101011", 17), ("0100111", 18), ("0001100", 19),
    ("0001000", 20), ("0010111", 21), ("0000011", 22), ("0000100", 23),
    ("0101000", 24), ("0101011", 25), ("0010011", 26), ("0100100", 27),
    ("0011000", 28), ("00000010", 29), ("00000011", 30), ("00011010", 31),
    ("00011011", 32), ("00010010", 33), ("00010011", 34), ("00010100", 35),
    ("00010101", 36), ("00010110", 37), ("00010111", 38), ("00101000", 39),
    ("00101001", 40), ("00101010", 41), ("00101011", 42), ("00101100", 43),
    ("00101101", 44), ("00000100", 45), ("00000101", 46), ("00001010", 47),
    ("00001011", 48), ("01010010", 49), ("01010011", 50), ("01010100", 51),
    ("01010101", 52), ("00100100", 53), ("00100101", 54), ("01011000", 55),
    ("01011001", 56), ("01011010", 57), ("01011011", 58), ("01001010", 59),
    ("01001011", 60), ("00110010", 61), ("00110011", 62), ("00110100", 63),
    // Make-up codes, multiples of 64.
    ("11011", 64), ("10010", 128), ("010111", 192), ("0110111", 256),
    ("00110110", 320), ("00110111", 384), ("01100100", 448), ("01100101", 512),
    ("01101000", 576), ("01100111", 640), ("011001100", 704), ("011001101", 768),
    ("011010010", 832), ("011010011", 896), ("011010100", 960), ("011010101", 1024),
    ("011010110", 1088), ("011010111", 1152), ("011011000", 1216), ("011011001", 1280),
    ("011011010", 1344), ("011011011", 1408), ("010011000", 1472), ("010011001", 1536),
    ("010011010", 1600), ("011000", 1664), ("010011011", 1728),
];

const BLACK: &[(&str, u16)] = &[
    ("0000110111", 0), ("010", 1), ("11", 2), ("10", 3),
    ("011", 4), ("0011", 5), ("0010", 6), ("00011", 7),
    ("000101", 8), ("000100", 9), ("0000100", 10), ("0000101", 11),
    ("0000111", 12), ("00000100", 13), ("00000111", 14), ("000011000", 15),
    ("0000010111", 16), ("0000011000", 17), ("0000001000", 18), ("00001100111", 19),
    ("00001101000", 20), ("00001101100", 21), ("00000110111", 22), ("00000101000", 23),
    ("00000010111", 24), ("00000011000", 25), ("000011001010", 26), ("000011001011", 27),
    ("000011001100", 28), ("000011001101", 29), ("000001101000", 30), ("000001101001", 31),
    ("000001101010", 32), ("000001101011", 33), ("000011010010", 34), ("000011010011", 35),
    ("000011010100", 36), ("000011010101", 37), ("000011010110", 38), ("000011010111", 39),
    ("000001101100", 40), ("000001101101", 41), ("000011011010", 42), ("000011011011", 43),
    ("000001010100", 44), ("000001010101", 45), ("000001010110", 46), ("000001010111", 47),
    ("000001100100", 48), ("000001100101", 49), ("000001010010", 50), ("000001010011", 51),
    ("000000100100", 52), ("000000110111", 53), ("000000111000", 54), ("000000100111", 55),
    ("000000101000", 56), ("000001011000", 57), ("000001011001", 58), ("000000101011", 59),
    ("000000101100", 60), ("000001011010", 61), ("000001100110", 62), ("000001100111", 63),
    ("0000001111", 64), ("000011001000", 128), ("000011001001", 192), ("000001011011", 256),
    ("000000110011", 320), ("000000110100", 384), ("000000110101", 448),
    ("0000001101100", 512), ("0000001101101", 576), ("0000001001010", 640),
    ("0000001001011", 704), ("0000001001100", 768), ("0000001001101", 832),
    ("0000001110010", 896), ("0000001110011", 960), ("0000001110100", 1024),
    ("0000001110101", 1088), ("0000001110110", 1152), ("0000001110111", 1216),
    ("0000001010010", 1280), ("0000001010011", 1344), ("0000001010100", 1408),
    ("0000001010101", 1472), ("0000001011010", 1536), ("0000001011011", 1600),
    ("0000001100100", 1664), ("0000001100101", 1728),
];

/// Make-up codes above 1728, shared by both colours.
const EXTENDED: &[(&str, u16)] = &[
    ("00000001000", 1792), ("00000001100", 1856), ("00000001101", 1920),
    ("000000010010", 1984), ("000000010011", 2048), ("000000010100", 2112),
    ("000000010101", 2176), ("000000010110", 2240), ("000000010111", 2304),
    ("000000011100", 2368), ("000000011101", 2432), ("000000011110", 2496),
    ("000000011111", 2560),
];

struct Bits<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Bits<'a> {
    fn new(data: &'a [u8]) -> Bits<'a> {
        Bits { data, pos: 0 }
    }

    fn done(&self) -> bool {
        self.pos >= self.data.len() * 8
    }

    fn peek(&self, n: u8) -> u16 {
        let mut out = 0u16;
        for i in 0..n as usize {
            let at = self.pos + i;
            let bit = match self.data.get(at / 8) {
                Some(byte) => (byte >> (7 - (at % 8))) & 1,
                None => 0,
            };
            out = (out << 1) | bit as u16;
        }
        out
    }

    fn skip(&mut self, n: u8) {
        self.pos += n as usize;
    }

    fn align(&mut self) {
        self.pos = self.pos.div_ceil(8) * 8;
    }
}

/// Read one run length, following make-up codes to their terminating code.
fn read_run(bits: &mut Bits, white: bool, tables: &Tables) -> Option<usize> {
    let mut total = 0usize;
    loop {
        let table = match white {
            true => &tables.white,
            false => &tables.black,
        };
        let mut matched = None;
        // Codes are prefix-free, so the first length that matches is the code.
        for len in 2..=13u8 {
            let probe = bits.peek(len);
            if let Some(code) = table
                .iter()
                .chain(tables.extended.iter())
                .find(|c| c.len == len && c.bits == probe)
            {
                matched = Some((len, code.run));
                break;
            }
        }
        let (len, run) = matched?;
        bits.skip(len);
        total += run as usize;
        // A make-up code is a multiple of 64 and must be followed by a
        // terminating code; a terminating code ends the run.
        if run < 64 || run % 64 != 0 {
            return Some(total);
        }
        if total > 1 << 20 {
            return None;
        }
    }
}

struct Tables {
    white: Vec<Code>,
    black: Vec<Code>,
    extended: Vec<Code>,
}

/// Decode a CCITT fax image to one bit per pixel, `1` white and `0` black,
/// packed into rows of `columns` bits.
///
/// Returns `None` only when the parameters make no sense; a stream that runs
/// out or goes wrong part way yields the rows decoded so far, since half a
/// scanned page is more use than none.
pub fn decode(data: &[u8], params: &Params) -> Option<Vec<u8>> {
    let columns = params.columns;
    if columns == 0 || columns > 1 << 16 {
        return None;
    }
    let tables = Tables {
        white: table(WHITE),
        black: table(BLACK),
        extended: table(EXTENDED),
    };
    let stride = columns.div_ceil(8);
    let mut out: Vec<u8> = Vec::new();
    let mut bits = Bits::new(data);

    // Changing elements of the row above: the positions where colour flips.
    // The imaginary row before the first is all white.
    let mut reference: Vec<usize> = vec![columns, columns];
    let mut row_count = 0usize;

    while !bits.done() && (params.rows == 0 || row_count < params.rows) {
        if params.byte_align && params.k >= 0 {
            bits.align();
        }
        // An end-of-line code, which Group 3 may put before every row.
        while bits.peek(12) == 1 {
            bits.skip(12);
        }
        if bits.done() {
            break;
        }
        // Group 3 with K > 0 announces each row's coding with one bit.
        let two_dimensional = match params.k {
            k if k < 0 => true,
            0 => false,
            _ => {
                let one_d = bits.peek(1) == 1;
                bits.skip(1);
                !one_d
            }
        };

        let Some(current) = decode_row(&mut bits, &reference, columns, two_dimensional, &tables)
        else {
            break;
        };

        // Paint the row: runs alternate, starting white.
        let mut row = vec![0u8; stride];
        let mut colour_white = true;
        let mut at = 0usize;
        for &change in current.iter() {
            let end = change.min(columns);
            if colour_white {
                for x in at..end {
                    row[x / 8] |= 0x80 >> (x % 8);
                }
            }
            at = end;
            colour_white = !colour_white;
            if at >= columns {
                break;
            }
        }
        if colour_white {
            for x in at..columns {
                row[x / 8] |= 0x80 >> (x % 8);
            }
        }
        out.extend_from_slice(&row);
        row_count += 1;

        reference = current;
        reference.push(columns);
        reference.push(columns);
        if params.byte_align && params.k < 0 {
            bits.align();
        }
    }

    // A short stream leaves the rest of the page white rather than missing.
    if params.rows > 0 {
        out.resize(stride * params.rows, 0xFF);
    }
    Some(out)
}

/// Decode one row into its changing-element positions.
fn decode_row(
    bits: &mut Bits,
    reference: &[usize],
    columns: usize,
    two_dimensional: bool,
    tables: &Tables,
) -> Option<Vec<usize>> {
    let mut current: Vec<usize> = Vec::new();
    let mut a0: isize = -1;
    let mut white = true;

    while a0 < columns as isize {
        if bits.done() {
            return if current.is_empty() { None } else { Some(current) };
        }
        if !two_dimensional {
            let run = read_run(bits, white, tables)?;
            let a1 = (a0.max(0) as usize + run).min(columns);
            current.push(a1);
            a0 = a1 as isize;
            white = !white;
            continue;
        }

        // b1: the first changing element on the reference line to the right of
        // a0 with the opposite colour to a0's; b2 the one after it.
        let mut b1 = columns;
        let mut index = 0;
        while index < reference.len() {
            let candidate = reference[index];
            if (candidate as isize) > a0 && (index % 2 == 0) == white {
                b1 = candidate;
                break;
            }
            index += 1;
        }
        let b2 = reference.get(index + 1).copied().unwrap_or(columns).min(columns);

        // Modes, longest patterns first so a prefix cannot win.
        if bits.peek(4) == 0b0001 {
            // Pass: the run continues past b2.
            bits.skip(4);
            a0 = b2 as isize;
            continue;
        }
        if bits.peek(3) == 0b001 {
            // Horizontal: two runs follow, in this row's own colours.
            bits.skip(3);
            let first = read_run(bits, white, tables)?;
            let second = read_run(bits, !white, tables)?;
            let start = a0.max(0) as usize;
            let a1 = (start + first).min(columns);
            let a2 = (a1 + second).min(columns);
            current.push(a1);
            current.push(a2);
            a0 = a2 as isize;
            continue;
        }
        // Vertical modes: a1 sits within three pixels of b1.
        let (offset, len) = if bits.peek(1) == 0b1 {
            (0isize, 1u8)
        } else if bits.peek(3) == 0b011 {
            (1, 3)
        } else if bits.peek(3) == 0b010 {
            (-1, 3)
        } else if bits.peek(6) == 0b000011 {
            (2, 6)
        } else if bits.peek(6) == 0b000010 {
            (-2, 6)
        } else if bits.peek(7) == 0b0000011 {
            (3, 7)
        } else if bits.peek(7) == 0b0000010 {
            (-3, 7)
        } else {
            // An extension or end-of-block: stop with what we have.
            return if current.is_empty() { None } else { Some(current) };
        };
        bits.skip(len);
        let a1 = (b1 as isize + offset).clamp(0, columns as isize) as usize;
        current.push(a1);
        a0 = a1 as isize;
        white = !white;
    }
    Some(current)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every code in a table must be readable: no code may be a prefix of
    /// another, or the decoder would take the short one and desynchronise.
    /// This is what catches a mistyped bit pattern, which is otherwise
    /// invisible until a scan decodes to noise.
    #[test]
    fn the_code_tables_are_prefix_free() {
        for (name, rows) in [("white", WHITE), ("black", BLACK)] {
            let codes = table(rows);
            let ext = table(EXTENDED);
            let all: Vec<&Code> = codes.iter().chain(ext.iter()).collect();
            for a in &all {
                for b in &all {
                    if std::ptr::eq(*a, *b) || a.len > b.len {
                        continue;
                    }
                    // Is `a` a prefix of `b`?
                    let shifted = b.bits >> (b.len - a.len);
                    assert!(
                        !(shifted == a.bits && !(a.len == b.len && a.run == b.run)),
                        "{name}: run {} ({} bits) is a prefix of run {} ({} bits)",
                        a.run,
                        a.len,
                        b.run,
                        b.len
                    );
                }
            }
        }
    }

    /// Each table must name every terminating run and the make-up runs.
    #[test]
    fn the_tables_are_complete() {
        for (name, rows) in [("white", WHITE), ("black", BLACK)] {
            let runs: std::collections::HashSet<u16> = rows.iter().map(|(_, r)| *r).collect();
            for run in 0..=63u16 {
                assert!(runs.contains(&run), "{name} is missing run {run}");
            }
            for run in (64..=1728).step_by(64) {
                assert!(runs.contains(&(run as u16)), "{name} is missing make-up {run}");
            }
        }
    }

    /// A row of alternating runs, coded one-dimensionally, comes back as it
    /// went in.
    #[test]
    fn a_one_dimensional_row_round_trips() {
        // White 4, black 2, white 2: codes "1011" "11" "0111".
        let mut data = Vec::new();
        let bits = "1011110111";
        let mut byte = 0u8;
        let mut n = 0;
        for c in bits.chars() {
            byte = (byte << 1) | (c == '1') as u8;
            n += 1;
            if n == 8 {
                data.push(byte);
                byte = 0;
                n = 0;
            }
        }
        if n > 0 {
            data.push(byte << (8 - n));
        }
        let out = decode(
            &data,
            &Params {
                k: 0,
                columns: 8,
                rows: 1,
                byte_align: false,
            },
        )
        .expect("a row");
        // 1 = white: 1111 00 11
        assert_eq!(out[0], 0b1111_0011);
    }

    #[test]
    fn rubbish_is_declined_rather_than_panicking() {
        for bad in [&b""[..], &b"\xff\xff\xff"[..], &b"\x00\x00\x00\x00"[..]] {
            let _ = decode(
                bad,
                &Params {
                    k: -1,
                    columns: 64,
                    rows: 4,
                    byte_align: false,
                },
            );
        }
        assert!(
            decode(
                b"\x00",
                &Params {
                    k: -1,
                    columns: 0,
                    rows: 1,
                    byte_align: false
                }
            )
            .is_none()
        );
    }
}
