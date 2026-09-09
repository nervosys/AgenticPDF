// SPDX-License-Identifier: AGPL-3.0-or-later
//! Wire primitives for the Agentic Document Format.
//!
//! Everything is little-endian, because every platform the app targets is
//! little-endian and byte-swapping on read would defeat the point of mapping
//! the file. Lengths and indices are LEB128 varints: documents have many small
//! counts, and a varint spends one byte on each of them instead of four.
//!
//! The reader never panics and never trusts a length. Every read is bounds
//! checked against the remaining slice, so a truncated or hostile file yields
//! `None` rather than a slice out of range — this module is the only place that
//! indexes into untrusted bytes, so the checks live here once.

use super::AdfError;

/// Alignment for chunk payloads.
///
/// A cache line. Chunks that are read as numeric arrays — embeddings above all
/// — start on a boundary that suits the loads performed on them, and the cost
/// is at most 63 bytes per chunk.
pub const ALIGN: usize = 64;

/// Round `value` up to the next multiple of [`ALIGN`].
pub fn align_up(value: usize) -> usize {
    value.div_ceil(ALIGN) * ALIGN
}

// ============================================================================
// Writing
// ============================================================================

/// A growable little-endian byte sink.
#[derive(Debug, Default)]
pub struct Writer {
    pub bytes: Vec<u8>,
}

impl Writer {
    pub fn new() -> Writer {
        Writer { bytes: Vec::new() }
    }

    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    pub fn u8(&mut self, value: u8) {
        self.bytes.push(value);
    }

    pub fn u16(&mut self, value: u16) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    pub fn u32(&mut self, value: u32) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    pub fn u64(&mut self, value: u64) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    pub fn f32(&mut self, value: f32) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    /// Store an f64 by its bit pattern, so NaN and infinity survive a round
    /// trip unchanged rather than being normalised by a decimal detour.
    pub fn f64(&mut self, value: f64) {
        self.bytes.extend_from_slice(&value.to_bits().to_le_bytes());
    }

    /// LEB128, seven bits per byte with the high bit as the continuation flag.
    pub fn varint(&mut self, mut value: u64) {
        loop {
            let byte = (value & 0x7F) as u8;
            value >>= 7;
            if value == 0 {
                self.bytes.push(byte);
                return;
            }
            self.bytes.push(byte | 0x80);
        }
    }

    pub fn usize(&mut self, value: usize) {
        self.varint(value as u64);
    }

    /// A length-prefixed byte string.
    pub fn bytes_with_len(&mut self, value: &[u8]) {
        self.varint(value.len() as u64);
        self.bytes.extend_from_slice(value);
    }

    pub fn raw(&mut self, value: &[u8]) {
        self.bytes.extend_from_slice(value);
    }

    /// An optional value, as a presence byte then the value.
    pub fn option<T>(&mut self, value: Option<T>, mut write: impl FnMut(&mut Writer, T)) {
        match value {
            Some(inner) => {
                self.u8(1);
                write(self, inner);
            }
            None => self.u8(0),
        }
    }

    /// Pad to the next [`ALIGN`] boundary.
    pub fn pad_to_align(&mut self) {
        self.bytes.resize(align_up(self.bytes.len()), 0);
    }
}

// ============================================================================
// Reading
// ============================================================================

/// A bounds-checked cursor over a borrowed slice.
///
/// Borrowed, not owned: the whole point of the format is that a reader can hold
/// a memory map and hand out views into it without copying.
#[derive(Debug, Clone)]
pub struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    pub fn new(bytes: &'a [u8]) -> Reader<'a> {
        Reader { bytes, pos: 0 }
    }

    pub fn position(&self) -> usize {
        self.pos
    }

    pub fn remaining(&self) -> usize {
        self.bytes.len().saturating_sub(self.pos)
    }

    pub fn is_empty(&self) -> bool {
        self.remaining() == 0
    }

    /// Take `count` bytes, or fail if the slice is short.
    pub fn take(&mut self, count: usize) -> Result<&'a [u8], AdfError> {
        let end = self.pos.checked_add(count).ok_or(AdfError::Truncated)?;
        let slice = self.bytes.get(self.pos..end).ok_or(AdfError::Truncated)?;
        self.pos = end;
        Ok(slice)
    }

    pub fn u8(&mut self) -> Result<u8, AdfError> {
        Ok(self.take(1)?[0])
    }

    pub fn u16(&mut self) -> Result<u16, AdfError> {
        let bytes = self.take(2)?;
        Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    pub fn u32(&mut self) -> Result<u32, AdfError> {
        let bytes = self.take(4)?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    pub fn u64(&mut self) -> Result<u64, AdfError> {
        let bytes = self.take(8)?;
        let mut array = [0u8; 8];
        array.copy_from_slice(bytes);
        Ok(u64::from_le_bytes(array))
    }

    pub fn f32(&mut self) -> Result<f32, AdfError> {
        let bytes = self.take(4)?;
        Ok(f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    /// A 32-byte SHA-256 digest. Stored as bytes rather than as words, so
    /// there is no endianness to get wrong.
    pub fn digest(&mut self) -> Result<[u8; 32], AdfError> {
        let bytes = self.take(32)?;
        let mut array = [0u8; 32];
        array.copy_from_slice(bytes);
        Ok(array)
    }

    pub fn f64(&mut self) -> Result<f64, AdfError> {
        Ok(f64::from_bits(self.u64()?))
    }

    /// LEB128. Bounded at ten bytes, which is the most a `u64` can occupy —
    /// without the bound a run of 0x80 bytes would spin to the end of the file.
    pub fn varint(&mut self) -> Result<u64, AdfError> {
        let mut value = 0u64;
        for step in 0..10 {
            let byte = self.u8()?;
            value |= u64::from(byte & 0x7F) << (step * 7);
            if byte & 0x80 == 0 {
                return Ok(value);
            }
        }
        Err(AdfError::Malformed("varint longer than 10 bytes"))
    }

    /// A varint used as a count, rejected if it exceeds what remains.
    ///
    /// This is the guard that keeps a corrupt length from causing a huge
    /// speculative allocation: a count can never exceed the bytes left, because
    /// even a one-byte element would not fit.
    pub fn count(&mut self) -> Result<usize, AdfError> {
        let value = self.varint()? as usize;
        if value > self.remaining() {
            return Err(AdfError::Malformed("count exceeds remaining bytes"));
        }
        Ok(value)
    }

    pub fn bytes_with_len(&mut self) -> Result<&'a [u8], AdfError> {
        let len = self.varint()? as usize;
        self.take(len)
    }

    /// Read a value that was written by [`Writer::option`].
    pub fn option<T>(
        &mut self,
        mut read: impl FnMut(&mut Reader<'a>) -> Result<T, AdfError>,
    ) -> Result<Option<T>, AdfError> {
        match self.u8()? {
            0 => Ok(None),
            1 => Ok(Some(read(self)?)),
            _ => Err(AdfError::Malformed("option tag is not 0 or 1")),
        }
    }
}

// FNV-1a used to live here, for content addressing and for the provenance
// hash. It was removed rather than kept for the cheap cases: the comment said
// "nothing authenticates with it", and then provenance did. Leaving a fast
// non-cryptographic hash in a module about file integrity is an invitation to
// make that mistake again. Everything now goes through `super::sha256`.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn varints_round_trip_across_their_whole_range() {
        let values = [0u64, 1, 127, 128, 300, u32::MAX as u64, u64::MAX];
        let mut writer = Writer::new();
        for value in values {
            writer.varint(value);
        }
        let mut reader = Reader::new(&writer.bytes);
        for value in values {
            assert_eq!(reader.varint().unwrap(), value);
        }
        assert!(reader.is_empty());
    }

    #[test]
    fn a_varint_of_all_continuation_bytes_is_rejected() {
        // Without the ten-byte bound this walks to the end of the input.
        let bytes = vec![0x80u8; 64];
        assert!(Reader::new(&bytes).varint().is_err());
    }

    #[test]
    fn a_count_larger_than_the_input_is_rejected_before_allocating() {
        let mut writer = Writer::new();
        writer.varint(1_000_000);
        let mut reader = Reader::new(&writer.bytes);
        assert!(reader.count().is_err());
    }

    #[test]
    fn reads_past_the_end_fail_rather_than_panic() {
        let bytes = [1u8, 2, 3];
        let mut reader = Reader::new(&bytes);
        assert!(reader.u16().is_ok());
        assert!(reader.u32().is_err());
    }

    #[test]
    fn floats_keep_their_exact_bits() {
        let values = [0.0f64, -0.0, 1.5, f64::MIN, f64::MAX, f64::INFINITY];
        let mut writer = Writer::new();
        for value in values {
            writer.f64(value);
        }
        let mut reader = Reader::new(&writer.bytes);
        for value in values {
            assert_eq!(reader.f64().unwrap().to_bits(), value.to_bits());
        }

        // NaN separately: it is never equal to itself, so compare bit patterns.
        let mut writer = Writer::new();
        writer.f64(f64::NAN);
        assert!(Reader::new(&writer.bytes).f64().unwrap().is_nan());
    }

    #[test]
    fn alignment_rounds_up_without_moving_already_aligned_values() {
        assert_eq!(align_up(0), 0);
        assert_eq!(align_up(1), ALIGN);
        assert_eq!(align_up(ALIGN), ALIGN);
        assert_eq!(align_up(ALIGN + 1), ALIGN * 2);
    }
}
