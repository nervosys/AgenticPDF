// SPDX-License-Identifier: AGPL-3.0-or-later
//! AES decryption in CBC mode, for documents encrypted with `AESV2`.
//!
//! Only the inverse cipher is here: a reader never has to encrypt. The
//! initialisation vector is the first sixteen bytes of the stream, which is
//! where the PDF specification puts it.

const SBOX: [u8; 256] = [
    0x63, 0x7c, 0x77, 0x7b, 0xf2, 0x6b, 0x6f, 0xc5, 0x30, 0x01, 0x67, 0x2b, 0xfe, 0xd7, 0xab, 0x76,
    0xca, 0x82, 0xc9, 0x7d, 0xfa, 0x59, 0x47, 0xf0, 0xad, 0xd4, 0xa2, 0xaf, 0x9c, 0xa4, 0x72, 0xc0,
    0xb7, 0xfd, 0x93, 0x26, 0x36, 0x3f, 0xf7, 0xcc, 0x34, 0xa5, 0xe5, 0xf1, 0x71, 0xd8, 0x31, 0x15,
    0x04, 0xc7, 0x23, 0xc3, 0x18, 0x96, 0x05, 0x9a, 0x07, 0x12, 0x80, 0xe2, 0xeb, 0x27, 0xb2, 0x75,
    0x09, 0x83, 0x2c, 0x1a, 0x1b, 0x6e, 0x5a, 0xa0, 0x52, 0x3b, 0xd6, 0xb3, 0x29, 0xe3, 0x2f, 0x84,
    0x53, 0xd1, 0x00, 0xed, 0x20, 0xfc, 0xb1, 0x5b, 0x6a, 0xcb, 0xbe, 0x39, 0x4a, 0x4c, 0x58, 0xcf,
    0xd0, 0xef, 0xaa, 0xfb, 0x43, 0x4d, 0x33, 0x85, 0x45, 0xf9, 0x02, 0x7f, 0x50, 0x3c, 0x9f, 0xa8,
    0x51, 0xa3, 0x40, 0x8f, 0x92, 0x9d, 0x38, 0xf5, 0xbc, 0xb6, 0xda, 0x21, 0x10, 0xff, 0xf3, 0xd2,
    0xcd, 0x0c, 0x13, 0xec, 0x5f, 0x97, 0x44, 0x17, 0xc4, 0xa7, 0x7e, 0x3d, 0x64, 0x5d, 0x19, 0x73,
    0x60, 0x81, 0x4f, 0xdc, 0x22, 0x2a, 0x90, 0x88, 0x46, 0xee, 0xb8, 0x14, 0xde, 0x5e, 0x0b, 0xdb,
    0xe0, 0x32, 0x3a, 0x0a, 0x49, 0x06, 0x24, 0x5c, 0xc2, 0xd3, 0xac, 0x62, 0x91, 0x95, 0xe4, 0x79,
    0xe7, 0xc8, 0x37, 0x6d, 0x8d, 0xd5, 0x4e, 0xa9, 0x6c, 0x56, 0xf4, 0xea, 0x65, 0x7a, 0xae, 0x08,
    0xba, 0x78, 0x25, 0x2e, 0x1c, 0xa6, 0xb4, 0xc6, 0xe8, 0xdd, 0x74, 0x1f, 0x4b, 0xbd, 0x8b, 0x8a,
    0x70, 0x3e, 0xb5, 0x66, 0x48, 0x03, 0xf6, 0x0e, 0x61, 0x35, 0x57, 0xb9, 0x86, 0xc1, 0x1d, 0x9e,
    0xe1, 0xf8, 0x98, 0x11, 0x69, 0xd9, 0x8e, 0x94, 0x9b, 0x1e, 0x87, 0xe9, 0xce, 0x55, 0x28, 0xdf,
    0x8c, 0xa1, 0x89, 0x0d, 0xbf, 0xe6, 0x42, 0x68, 0x41, 0x99, 0x2d, 0x0f, 0xb0, 0x54, 0xbb, 0x16,
];

/// The inverse substitution, built from `SBOX` so the two cannot disagree.
fn inv_sbox() -> [u8; 256] {
    let mut inv = [0u8; 256];
    for (i, value) in SBOX.iter().enumerate() {
        inv[*value as usize] = i as u8;
    }
    inv
}

/// Multiply in GF(2^8), the field the mix step is defined over.
fn xtime(a: u8) -> u8 {
    match a & 0x80 {
        0 => a << 1,
        _ => (a << 1) ^ 0x1b,
    }
}

fn mul(a: u8, b: u8) -> u8 {
    let mut result = 0u8;
    let mut a = a;
    let mut b = b;
    while b != 0 {
        if b & 1 != 0 {
            result ^= a;
        }
        a = xtime(a);
        b >>= 1;
    }
    result
}

/// Expand a 16, 24 or 32 byte key into one round key per round.
fn expand(key: &[u8]) -> Option<Vec<[u8; 16]>> {
    let nk = key.len() / 4;
    let rounds = match key.len() {
        16 => 10,
        24 => 12,
        32 => 14,
        _ => return None,
    };
    let words = 4 * (rounds + 1);
    let mut w: Vec<[u8; 4]> = Vec::with_capacity(words);
    for chunk in key.chunks_exact(4) {
        w.push([chunk[0], chunk[1], chunk[2], chunk[3]]);
    }
    let mut rcon = 1u8;
    for i in nk..words {
        let mut temp = w[i - 1];
        if i % nk == 0 {
            temp = [
                SBOX[temp[1] as usize] ^ rcon,
                SBOX[temp[2] as usize],
                SBOX[temp[3] as usize],
                SBOX[temp[0] as usize],
            ];
            rcon = xtime(rcon);
        } else if nk > 6 && i % nk == 4 {
            temp = [
                SBOX[temp[0] as usize],
                SBOX[temp[1] as usize],
                SBOX[temp[2] as usize],
                SBOX[temp[3] as usize],
            ];
        }
        let prev = w[i - nk];
        w.push([
            prev[0] ^ temp[0],
            prev[1] ^ temp[1],
            prev[2] ^ temp[2],
            prev[3] ^ temp[3],
        ]);
    }
    let mut keys = Vec::with_capacity(rounds + 1);
    for round in 0..=rounds {
        let mut block = [0u8; 16];
        for c in 0..4 {
            block[c * 4..c * 4 + 4].copy_from_slice(&w[round * 4 + c]);
        }
        keys.push(block);
    }
    Some(keys)
}

/// The inverse cipher on one sixteen-byte block, in place.
fn decrypt_block(block: &mut [u8; 16], keys: &[[u8; 16]], inv: &[u8; 256]) {
    let rounds = keys.len() - 1;
    for (a, b) in block.iter_mut().zip(keys[rounds].iter()) {
        *a ^= b;
    }
    for round in (0..rounds).rev() {
        // Inverse shift rows: row r moves right by r.
        let mut shifted = *block;
        for row in 1..4 {
            for col in 0..4 {
                shifted[((col + row) % 4) * 4 + row] = block[col * 4 + row];
            }
        }
        // Inverse substitution.
        for byte in shifted.iter_mut() {
            *byte = inv[*byte as usize];
        }
        // Add the round key.
        for (a, b) in shifted.iter_mut().zip(keys[round].iter()) {
            *a ^= b;
        }
        // Inverse mix columns, except on the last round performed.
        if round > 0 {
            let mut mixed = [0u8; 16];
            for col in 0..4 {
                let s = &shifted[col * 4..col * 4 + 4];
                mixed[col * 4] = mul(s[0], 14) ^ mul(s[1], 11) ^ mul(s[2], 13) ^ mul(s[3], 9);
                mixed[col * 4 + 1] = mul(s[0], 9) ^ mul(s[1], 14) ^ mul(s[2], 11) ^ mul(s[3], 13);
                mixed[col * 4 + 2] = mul(s[0], 13) ^ mul(s[1], 9) ^ mul(s[2], 14) ^ mul(s[3], 11);
                mixed[col * 4 + 3] = mul(s[0], 11) ^ mul(s[1], 13) ^ mul(s[2], 9) ^ mul(s[3], 14);
            }
            shifted = mixed;
        }
        *block = shifted;
    }
}

/// Decrypt a stream whose first sixteen bytes are the initialisation vector,
/// removing the PKCS#7 padding.
///
/// Returns `None` when the input is not a whole number of blocks, or is too
/// short to hold an initialisation vector and one block of content.
pub fn decrypt_cbc(key: &[u8], data: &[u8]) -> Option<Vec<u8>> {
    let keys = expand(key)?;
    let inv = inv_sbox();
    if data.len() < 32 || !data.len().is_multiple_of(16) {
        return None;
    }
    let mut previous: [u8; 16] = data[..16].try_into().ok()?;
    let mut out = Vec::with_capacity(data.len() - 16);
    for chunk in data[16..].chunks_exact(16) {
        let cipher: [u8; 16] = chunk.try_into().ok()?;
        let mut block = cipher;
        decrypt_block(&mut block, &keys, &inv);
        for (a, b) in block.iter_mut().zip(previous.iter()) {
            *a ^= b;
        }
        out.extend_from_slice(&block);
        previous = cipher;
    }
    // PKCS#7: the last byte says how many were added.
    let pad = *out.last()? as usize;
    if (1..=16).contains(&pad) && pad <= out.len() {
        out.truncate(out.len() - pad);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The appendix C vector from FIPS-197: a known key and ciphertext with a
    /// known plaintext. Without this the cipher is unverifiable, and a wrong
    /// one decrypts every document to noise with no error to show for it.
    #[test]
    fn the_fips_vector_decrypts() {
        let key: [u8; 16] = [
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
            0x0e, 0x0f,
        ];
        let cipher: [u8; 16] = [
            0x69, 0xc4, 0xe0, 0xd8, 0x6a, 0x7b, 0x04, 0x30, 0xd8, 0xcd, 0xb7, 0x80, 0x70, 0xb4,
            0xc5, 0x5a,
        ];
        let want: [u8; 16] = [
            0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd,
            0xee, 0xff,
        ];
        let keys = expand(&key).expect("a 128-bit key expands");
        let inv = inv_sbox();
        let mut block = cipher;
        decrypt_block(&mut block, &keys, &inv);
        assert_eq!(block, want);
    }

    /// The 256-bit vector from the same appendix, so the key schedule is
    /// exercised beyond its shortest form.
    #[test]
    fn the_long_key_vector_decrypts() {
        let key: [u8; 32] = [
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
            0x0e, 0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b,
            0x1c, 0x1d, 0x1e, 0x1f,
        ];
        let cipher: [u8; 16] = [
            0x8e, 0xa2, 0xb7, 0xca, 0x51, 0x67, 0x45, 0xbf, 0xea, 0xfc, 0x49, 0x90, 0x4b, 0x49,
            0x60, 0x89,
        ];
        let want: [u8; 16] = [
            0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd,
            0xee, 0xff,
        ];
        let keys = expand(&key).expect("a 256-bit key expands");
        let inv = inv_sbox();
        let mut block = cipher;
        decrypt_block(&mut block, &keys, &inv);
        assert_eq!(block, want);
    }

    #[test]
    fn a_short_or_ragged_stream_is_declined() {
        let key = [0u8; 16];
        assert!(decrypt_cbc(&key, &[0u8; 16]).is_none());
        assert!(decrypt_cbc(&key, &[0u8; 40]).is_none());
        assert!(decrypt_cbc(&[0u8; 7], &[0u8; 32]).is_none());
    }
}
