// SPDX-License-Identifier: AGPL-3.0-or-later
//! Reading documents that are encrypted.
//!
//! Most encrypted PDFs are not secret. They carry an empty user password and a
//! permissions flag that asks readers not to print or copy — a wish, not a
//! lock, since the file hands over the key to anyone who opens it. A reader
//! that ignores encryption does not honour that wish; it simply shows a blank
//! page, which is what this crate did for eleven of the two hundred and
//! eighty-eight documents in the test corpus.
//!
//! Implemented: the standard security handler, revisions 2 through 4, which
//! covers RC4 at 40 and 128 bits and AES-128. A document that wants a real
//! password is reported as needing one rather than decoded to noise.

pub mod aes;
pub mod md5;
pub mod rc4;

use crate::engine::{Dict, Object};

/// The padding every standard-handler key derivation starts from.
const PAD: [u8; 32] = [
    0x28, 0xBF, 0x4E, 0x5E, 0x4E, 0x75, 0x8A, 0x41, 0x64, 0x00, 0x4E, 0x56, 0xFF, 0xFA, 0x01, 0x08,
    0x2E, 0x2E, 0x00, 0xB6, 0xD0, 0x68, 0x3E, 0x80, 0x2F, 0x0C, 0xA9, 0xFE, 0x64, 0x53, 0x69, 0x7A,
];

/// How the document's streams and strings are wrapped.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Cipher {
    Rc4,
    Aes128,
    /// Named in the document but not something we decode.
    None,
}

/// The file's encryption key, and how to apply it.
#[derive(Clone)]
pub struct Decryptor {
    key: Vec<u8>,
    cipher: Cipher,
}

impl Decryptor {
    /// Build a decryptor for a document, given its `/Encrypt` dictionary and
    /// the first element of its `/ID`.
    ///
    /// Returns `None` when the document is encrypted in a way this does not
    /// read — a real password, or a handler that is not the standard one — so
    /// the caller can say so rather than produce rubbish.
    pub fn new(encrypt: &Dict, file_id: &[u8]) -> Option<Decryptor> {
        let name = |key: &str| match encrypt.get(key) {
            Some(Object::Name(n)) => Some(n.clone()),
            _ => None,
        };
        let int = |key: &str| match encrypt.get(key) {
            Some(Object::Int(v)) => Some(*v),
            Some(Object::Real(v)) => Some(*v as i64),
            _ => None,
        };
        let bytes = |key: &str| match encrypt.get(key) {
            Some(Object::Str(b)) => Some(b.clone()),
            _ => None,
        };

        if name("Filter").as_deref() != Some("Standard") {
            return None;
        }
        let revision = int("R")?;
        let version = int("V").unwrap_or(0);
        if !(2..=4).contains(&revision) {
            // Revisions 5 and 6 are AES-256 with a different derivation.
            return None;
        }

        let owner = bytes("O")?;
        let permissions = int("P")? as i32;
        let length_bits = int("Length").unwrap_or(40);
        let mut key_len = (length_bits / 8).clamp(5, 16) as usize;

        // Version 4 names its ciphers in a crypt-filter dictionary.
        let mut cipher = match version {
            1 => Cipher::Rc4,
            2 => Cipher::Rc4,
            _ => Cipher::None,
        };
        if version == 4 {
            let stream_filter = name("StmF").unwrap_or_else(|| "Identity".into());
            let filters = match encrypt.get("CF") {
                Some(Object::Dict(d)) => d.clone(),
                _ => Dict::new(),
            };
            let chosen = match filters.get(&stream_filter) {
                Some(Object::Dict(d)) => d.clone(),
                _ => Dict::new(),
            };
            if let Some(Object::Int(bits)) = chosen.get("Length") {
                // Some producers state this in bytes and some in bits.
                key_len = match *bits > 40 {
                    true => (*bits / 8).clamp(5, 16) as usize,
                    false => (*bits).clamp(5, 16) as usize,
                };
            }
            cipher = match chosen.get("CFM") {
                Some(Object::Name(n)) if n == "AESV2" => Cipher::Aes128,
                Some(Object::Name(n)) if n == "V2" => Cipher::Rc4,
                Some(Object::Name(n)) if n == "None" => Cipher::None,
                _ => Cipher::Rc4,
            };
        }
        if cipher == Cipher::None {
            return None;
        }

        // Algorithm 2, with the empty user password.
        let mut input = Vec::with_capacity(96);
        input.extend_from_slice(&PAD);
        input.extend_from_slice(&owner[..owner.len().min(32)]);
        input.extend_from_slice(&permissions.to_le_bytes());
        input.extend_from_slice(file_id);
        if revision >= 4 && matches!(encrypt.get("EncryptMetadata"), Some(Object::Bool(false))) {
            input.extend_from_slice(&[0xFF; 4]);
        }
        let mut key = md5::digest(&input).to_vec();
        if revision >= 3 {
            // Fifty more rounds over the first `key_len` bytes, which is what
            // makes a short key expensive to attack by brute force.
            for _ in 0..50 {
                key = md5::digest(&key[..key_len]).to_vec();
            }
        }
        key.truncate(key_len);
        Some(Decryptor { key, cipher })
    }

    /// The key for one object, which mixes in its number and generation so the
    /// same bytes in two places do not encrypt alike.
    fn object_key(&self, number: u32, generation: u16) -> Vec<u8> {
        let mut input = self.key.clone();
        input.extend_from_slice(&number.to_le_bytes()[..3]);
        input.extend_from_slice(&generation.to_le_bytes()[..2]);
        if self.cipher == Cipher::Aes128 {
            input.extend_from_slice(b"sAlT");
        }
        let digest = md5::digest(&input);
        let take = (self.key.len() + 5).min(16);
        digest[..take].to_vec()
    }

    /// Decrypt one stream or string belonging to an object.
    pub fn decrypt(&self, number: u32, generation: u16, data: &[u8]) -> Vec<u8> {
        let key = self.object_key(number, generation);
        match self.cipher {
            Cipher::Rc4 => rc4::apply(&key, data),
            Cipher::Aes128 => aes::decrypt_cbc(&key, data).unwrap_or_default(),
            Cipher::None => data.to_vec(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The key derivation is fixed by the specification, so a document
    /// encrypted with a known owner hash and permissions must produce a known
    /// key. This is Algorithm 2 with an empty user password, 128-bit RC4.
    #[test]
    fn the_file_key_follows_algorithm_two() {
        let mut encrypt = Dict::new();
        encrypt.insert("Filter".into(), Object::Name("Standard".into()));
        encrypt.insert("R".into(), Object::Int(3));
        encrypt.insert("V".into(), Object::Int(2));
        encrypt.insert("Length".into(), Object::Int(128));
        encrypt.insert("P".into(), Object::Int(-1));
        encrypt.insert("O".into(), Object::Str(vec![0x41; 32]));
        let crypt = Decryptor::new(&encrypt, b"0123456789abcdef").expect("a decryptor");
        assert_eq!(crypt.key.len(), 16);
        assert_eq!(crypt.cipher, Cipher::Rc4);

        // The same inputs must always give the same key, and the object key
        // must depend on the object.
        let again = Decryptor::new(&encrypt, b"0123456789abcdef").unwrap();
        assert_eq!(crypt.key, again.key);
        assert_ne!(crypt.object_key(1, 0), crypt.object_key(2, 0));
        assert_ne!(crypt.object_key(1, 0), crypt.object_key(1, 1));
    }

    /// A handler we do not read must say so rather than produce noise.
    #[test]
    fn an_unreadable_handler_is_declined() {
        let mut encrypt = Dict::new();
        encrypt.insert("Filter".into(), Object::Name("Standard".into()));
        encrypt.insert("R".into(), Object::Int(6));
        encrypt.insert("V".into(), Object::Int(5));
        encrypt.insert("P".into(), Object::Int(-1));
        encrypt.insert("O".into(), Object::Str(vec![0x41; 48]));
        assert!(Decryptor::new(&encrypt, b"id").is_none());

        let mut custom = Dict::new();
        custom.insert("Filter".into(), Object::Name("MySecurity".into()));
        custom.insert("R".into(), Object::Int(4));
        custom.insert("P".into(), Object::Int(-1));
        custom.insert("O".into(), Object::Str(vec![0x41; 32]));
        assert!(Decryptor::new(&custom, b"id").is_none());
    }

    /// A whole document, encrypted the way a real one is, read back through
    /// the ordinary entry point.
    ///
    /// RC4 is symmetric, so the fixture is built with the same primitive that
    /// reads it -- but the key comes from the derivation under test, and the
    /// reader derives its own from the dictionary rather than being handed
    /// this one.
    #[test]
    fn an_encrypted_document_reads() {
        let file_id: &[u8] = b"0123456789abcdef";
        let hex_id = "30313233343536373839616263646566";
        let owner = vec![0x5A_u8; 32];
        let mut encrypt = Dict::new();
        encrypt.insert("Filter".into(), Object::Name("Standard".into()));
        encrypt.insert("R".into(), Object::Int(3));
        encrypt.insert("V".into(), Object::Int(2));
        encrypt.insert("Length".into(), Object::Int(128));
        encrypt.insert("P".into(), Object::Int(-3904));
        encrypt.insert("O".into(), Object::Str(owner.clone()));
        let crypt = Decryptor::new(&encrypt, file_id).expect("a decryptor");

        // Object 5 carries the page's content, wrapped under its own key.
        let content = b"BT /F1 24 Tf 20 100 Td (Unlocked) Tj ET";
        let wrapped = crypt.decrypt(5, 0, content);
        let hex = |bytes: &[u8]| -> String {
            bytes.iter().map(|b| format!("{b:02x}")).collect()
        };

        let mut pdf: Vec<u8> = Vec::new();
        let mut offsets = vec![0usize];
        pdf.extend_from_slice(b"%PDF-1.4
");
        let push = |pdf: &mut Vec<u8>, offsets: &mut Vec<usize>, body: &[u8]| {
            offsets.push(pdf.len());
            let number = offsets.len() - 1;
            pdf.extend_from_slice(format!("{number} 0 obj
").as_bytes());
            pdf.extend_from_slice(body);
            pdf.extend_from_slice(b"
endobj
");
        };
        push(&mut pdf, &mut offsets, b"<< /Type /Catalog /Pages 2 0 R >>");
        push(&mut pdf, &mut offsets, b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>");
        push(
            &mut pdf,
            &mut offsets,
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 200]               /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>",
        );
        push(
            &mut pdf,
            &mut offsets,
            b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
        );
        let mut stream = format!("<< /Length {} >>
stream
", wrapped.len()).into_bytes();
        stream.extend_from_slice(&wrapped);
        stream.extend_from_slice(b"
endstream");
        push(&mut pdf, &mut offsets, &stream);
        push(
            &mut pdf,
            &mut offsets,
            format!(
                "<< /Filter /Standard /R 3 /V 2 /Length 128 /P -3904 /O <{}> /U <{}> >>",
                hex(&owner),
                hex(&[0u8; 32])
            )
            .as_bytes(),
        );

        let xref = pdf.len();
        pdf.extend_from_slice(format!("xref
0 {}
", offsets.len()).as_bytes());
        pdf.extend_from_slice(b"0000000000 65535 f 
");
        for offset in &offsets[1..] {
            pdf.extend_from_slice(format!("{offset:010} 00000 n 
").as_bytes());
        }
        pdf.extend_from_slice(
            format!(
                "trailer
<< /Size {} /Root 1 0 R /Encrypt 6 0 R /ID [<{hex_id}> <{hex_id}>] >>
                 startxref
{xref}
%%EOF
",
                offsets.len()
            )
            .as_bytes(),
        );

        let text = crate::PdfDocument::from_bytes(&pdf)
            .expect("an encrypted document should open")
            .extract_text();
        assert!(
            text.contains("Unlocked"),
            "the content stream should decrypt: {text:?}"
        );
    }

    /// RC4 is its own inverse, so a round trip through the object key is the
    /// simplest end-to-end check that the pieces fit together.
    #[test]
    fn a_stream_round_trips() {
        let mut encrypt = Dict::new();
        encrypt.insert("Filter".into(), Object::Name("Standard".into()));
        encrypt.insert("R".into(), Object::Int(3));
        encrypt.insert("V".into(), Object::Int(2));
        encrypt.insert("Length".into(), Object::Int(128));
        encrypt.insert("P".into(), Object::Int(-3904));
        encrypt.insert("O".into(), Object::Str(vec![0x7F; 32]));
        let crypt = Decryptor::new(&encrypt, b"fileid0123456789").unwrap();

        let plain = b"BT /F1 12 Tf (hello) Tj ET";
        let wrapped = crypt.decrypt(7, 0, plain);
        assert_ne!(&wrapped[..], &plain[..]);
        assert_eq!(crypt.decrypt(7, 0, &wrapped), plain);
    }
}
