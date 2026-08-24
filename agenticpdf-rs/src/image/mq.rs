//! The MQ arithmetic decoder, shared by JPEG 2000 and JBIG2.
//!
//! Both standards specify the same coder (T.800 Annex C and T.88 Annex E are
//! the same procedure); they differ only in how many contexts they keep and
//! what those contexts mean, so the state lives with the caller.

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

pub struct Mq<'a> {
    d: &'a [u8],
    bp: usize,
    chigh: u32,
    clow: u32,
    ct: i32,
    a: u32,
}

impl<'a> Mq<'a> {
    pub fn new(d: &'a [u8]) -> Mq<'a> {
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
    pub fn bit(&mut self, ctx: &mut [u8], cx: usize) -> u32 {
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
