// SPDX-License-Identifier: AGPL-3.0-or-later
//! RC4, the stream cipher the standard security handler used before AES.
//!
//! As with MD5 next door, this is not a security choice: it is what the
//! documents in front of us were encrypted with.

/// Apply the keystream. RC4 is symmetric, so this both encrypts and decrypts.
pub fn apply(key: &[u8], data: &[u8]) -> Vec<u8> {
    if key.is_empty() {
        return data.to_vec();
    }
    let mut s: [u8; 256] = [0; 256];
    for (i, slot) in s.iter_mut().enumerate() {
        *slot = i as u8;
    }
    let mut j = 0u8;
    for i in 0..256 {
        j = j
            .wrapping_add(s[i])
            .wrapping_add(key[i % key.len()]);
        s.swap(i, j as usize);
    }

    let mut out = Vec::with_capacity(data.len());
    let (mut i, mut j) = (0u8, 0u8);
    for byte in data {
        i = i.wrapping_add(1);
        j = j.wrapping_add(s[i as usize]);
        s.swap(i as usize, j as usize);
        let k = s[(s[i as usize].wrapping_add(s[j as usize])) as usize];
        out.push(byte ^ k);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }

    /// The test vectors published with the cipher.
    #[test]
    fn the_published_vectors_hold() {
        assert_eq!(hex(&apply(b"Key", b"Plaintext")), "bbf316e8d940af0ad3");
        assert_eq!(hex(&apply(b"Wiki", b"pedia")), "1021bf0420");
        assert_eq!(
            hex(&apply(b"Secret", b"Attack at dawn")),
            "45a01f645fc35b383552544b9bf5"
        );
    }

    #[test]
    fn it_is_its_own_inverse() {
        let key = b"a rather long key for a change";
        let message = b"the quick brown fox jumps over the lazy dog";
        assert_eq!(apply(key, &apply(key, message)), message);
    }
}
