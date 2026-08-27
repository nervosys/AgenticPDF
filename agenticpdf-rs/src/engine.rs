// SPDX-License-Identifier: AGPL-3.0-or-later
//! AgenticPDF parsing engine — a real PDF object model.
//!
//! This module replaces the previous heuristic byte-scanner with a proper
//! lexer → object model → cross-reference resolver → page-tree walker, plus
//! content-stream text extraction with font/encoding decoding (ToUnicode
//! CMaps, WinAnsi base encoding + Differences, and Identity-H composite fonts).
//!
//! It supports both classic `xref` tables and PDF 1.5+ cross-reference streams
//! with compressed object streams (`/ObjStm`) — the common case for modern
//! documents that the old scanner could not handle at all.

use crate::parser::PdfParser;
use crate::{
    ImageInfo, OutlineItem, PdfAnnotation, PdfDocument, PdfError, PdfMetadata, PdfPage, TextBlock,
};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::HashMap;

const MAX_DEPTH: usize = 64;
const MAX_OBJECTS: usize = 2_000_000;

// ============================================================================
// Object model
// ============================================================================

/// A PDF object dictionary, preserving insertion is unnecessary; key lookup only.
pub type Dict = HashMap<String, Object>;

/// A fully-parsed PDF object.
#[derive(Debug, Clone)]
pub enum Object {
    Null,
    Bool(bool),
    Int(i64),
    Real(f64),
    /// Decoded string bytes (literal `(...)` or hex `<...>`).
    Str(Vec<u8>),
    /// Name without the leading slash.
    Name(String),
    Array(Vec<Object>),
    Dict(Dict),
    /// Stream: dictionary plus raw (still-encoded) bytes.
    Stream(Dict, Vec<u8>),
    /// Indirect reference: (object number, generation).
    Ref(u32, u16),
}

impl Object {
    pub fn as_int(&self) -> Option<i64> {
        match self {
            Object::Int(n) => Some(*n),
            Object::Real(r) => Some(*r as i64),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Object::Int(n) => Some(*n as f64),
            Object::Real(r) => Some(*r),
            _ => None,
        }
    }

    pub fn as_name(&self) -> Option<&str> {
        match self {
            Object::Name(s) => Some(s.as_str()),
            _ => None,
        }
    }

    pub fn as_str_bytes(&self) -> Option<&[u8]> {
        match self {
            Object::Str(b) => Some(b),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[Object]> {
        match self {
            Object::Array(a) => Some(a),
            _ => None,
        }
    }

    pub fn as_dict(&self) -> Option<&Dict> {
        match self {
            Object::Dict(d) => Some(d),
            Object::Stream(d, _) => Some(d),
            _ => None,
        }
    }

    pub fn as_ref(&self) -> Option<(u32, u16)> {
        match self {
            Object::Ref(n, g) => Some((*n, *g)),
            _ => None,
        }
    }
}

// ============================================================================
// Lexer / recursive-descent object parser
// ============================================================================

/// Parses PDF objects from a byte buffer at a given position.
struct Lexer<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Lexer<'a> {
    fn new(buf: &'a [u8], pos: usize) -> Self {
        Self { buf, pos }
    }

    fn at_end(&self) -> bool {
        self.pos >= self.buf.len()
    }

    fn peek(&self) -> u8 {
        self.buf.get(self.pos).copied().unwrap_or(0)
    }

    fn is_ws(b: u8) -> bool {
        matches!(b, b' ' | b'\t' | b'\r' | b'\n' | 0x0c | 0x00)
    }

    fn is_delim(b: u8) -> bool {
        matches!(
            b,
            b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'%'
        )
    }

    fn skip_ws(&mut self) {
        while self.pos < self.buf.len() {
            let b = self.buf[self.pos];
            if b == b'%' {
                // comment to end of line
                while self.pos < self.buf.len()
                    && self.buf[self.pos] != b'\n'
                    && self.buf[self.pos] != b'\r'
                {
                    self.pos += 1;
                }
            } else if Self::is_ws(b) {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    /// Read a bare token (keyword / number characters) without leading ws.
    fn read_token(&mut self) -> &'a [u8] {
        let start = self.pos;
        while self.pos < self.buf.len() {
            let b = self.buf[self.pos];
            if Self::is_ws(b) || Self::is_delim(b) {
                break;
            }
            self.pos += 1;
        }
        &self.buf[start..self.pos]
    }

    /// Parse a single object, resolving `n g R` references and `n g obj`.
    fn parse_object(&mut self, depth: usize) -> Result<Object, PdfError> {
        if depth > MAX_DEPTH {
            return Err(PdfError::ObjectParseError("recursion limit".into()));
        }
        self.skip_ws();
        if self.at_end() {
            return Ok(Object::Null);
        }
        let b = self.peek();
        match b {
            b'/' => Ok(self.parse_name()),
            b'(' => Ok(Object::Str(self.parse_literal_string())),
            b'<' => {
                if self.buf.get(self.pos + 1) == Some(&b'<') {
                    self.parse_dict_or_stream(depth)
                } else {
                    Ok(Object::Str(self.parse_hex_string()))
                }
            }
            b'[' => self.parse_array(depth),
            b'0'..=b'9' | b'+' | b'-' | b'.' => self.parse_number_or_ref(),
            _ => {
                // keyword: true / false / null / R / obj / endobj / stream
                let tok = self.read_token();
                match tok {
                    b"true" => Ok(Object::Bool(true)),
                    b"false" => Ok(Object::Bool(false)),
                    b"null" => Ok(Object::Null),
                    b"" => {
                        // Unknown delimiter we don't handle; advance to avoid loop.
                        self.pos += 1;
                        Ok(Object::Null)
                    }
                    _ => Ok(Object::Null),
                }
            }
        }
    }

    fn parse_name(&mut self) -> Object {
        self.pos += 1; // skip '/'
        let mut s = String::new();
        while self.pos < self.buf.len() {
            let b = self.buf[self.pos];
            if Self::is_ws(b) || Self::is_delim(b) {
                break;
            }
            if b == b'#' && self.pos + 2 < self.buf.len() {
                let hi = hex_val(self.buf[self.pos + 1]);
                let lo = hex_val(self.buf[self.pos + 2]);
                if let (Some(h), Some(l)) = (hi, lo) {
                    s.push((h * 16 + l) as char);
                    self.pos += 3;
                    continue;
                }
            }
            s.push(b as char);
            self.pos += 1;
        }
        Object::Name(s)
    }

    fn parse_literal_string(&mut self) -> Vec<u8> {
        self.pos += 1; // skip '('
        let mut out = Vec::new();
        let mut depth = 1usize;
        while self.pos < self.buf.len() {
            let b = self.buf[self.pos];
            self.pos += 1;
            match b {
                b'\\' => {
                    if self.pos >= self.buf.len() {
                        break;
                    }
                    let e = self.buf[self.pos];
                    self.pos += 1;
                    match e {
                        b'n' => out.push(b'\n'),
                        b'r' => out.push(b'\r'),
                        b't' => out.push(b'\t'),
                        b'b' => out.push(0x08),
                        b'f' => out.push(0x0c),
                        b'(' => out.push(b'('),
                        b')' => out.push(b')'),
                        b'\\' => out.push(b'\\'),
                        b'\r' => {
                            // line continuation; consume optional \n
                            if self.peek() == b'\n' {
                                self.pos += 1;
                            }
                        }
                        b'\n' => {}
                        b'0'..=b'7' => {
                            // up to 3 octal digits
                            let mut val = (e - b'0') as u32;
                            for _ in 0..2 {
                                let c = self.peek();
                                if (b'0'..=b'7').contains(&c) {
                                    val = val * 8 + (c - b'0') as u32;
                                    self.pos += 1;
                                } else {
                                    break;
                                }
                            }
                            out.push(val as u8);
                        }
                        other => out.push(other),
                    }
                }
                b'(' => {
                    depth += 1;
                    out.push(b'(');
                }
                b')' => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                    out.push(b')');
                }
                _ => out.push(b),
            }
        }
        out
    }

    fn parse_hex_string(&mut self) -> Vec<u8> {
        self.pos += 1; // skip '<'
        let mut out = Vec::new();
        let mut hi: Option<u8> = None;
        while self.pos < self.buf.len() {
            let b = self.buf[self.pos];
            self.pos += 1;
            if b == b'>' {
                break;
            }
            if let Some(v) = hex_val(b) {
                match hi {
                    None => hi = Some(v),
                    Some(h) => {
                        out.push(h * 16 + v);
                        hi = None;
                    }
                }
            }
        }
        if let Some(h) = hi {
            out.push(h * 16); // odd digit: pad low nibble with 0
        }
        out
    }

    fn parse_array(&mut self, depth: usize) -> Result<Object, PdfError> {
        self.pos += 1; // skip '['
        let mut items = Vec::new();
        loop {
            self.skip_ws();
            if self.at_end() {
                break;
            }
            if self.peek() == b']' {
                self.pos += 1;
                break;
            }
            let before = self.pos;
            items.push(self.parse_object(depth + 1)?);
            if self.pos == before {
                self.pos += 1; // guarantee progress
            }
        }
        Ok(Object::Array(items))
    }

    fn parse_dict_or_stream(&mut self, depth: usize) -> Result<Object, PdfError> {
        self.pos += 2; // skip '<<'
        let mut dict = Dict::new();
        loop {
            self.skip_ws();
            if self.at_end() {
                break;
            }
            if self.peek() == b'>' {
                self.pos += 1;
                if self.peek() == b'>' {
                    self.pos += 1;
                }
                break;
            }
            if self.peek() != b'/' {
                // malformed; bail to avoid infinite loop
                self.pos += 1;
                continue;
            }
            let key = match self.parse_name() {
                Object::Name(n) => n,
                _ => break,
            };
            let before = self.pos;
            let val = self.parse_object(depth + 1)?;
            if self.pos == before {
                self.pos += 1;
            }
            dict.insert(key, val);
        }
        // Is this a stream?
        self.skip_ws();
        if self.buf[self.pos..].starts_with(b"stream") {
            self.pos += 6;
            // EOL after 'stream': either \r\n or \n
            if self.peek() == b'\r' {
                self.pos += 1;
            }
            if self.peek() == b'\n' {
                self.pos += 1;
            }
            let data_start = self.pos;
            // Determine length from /Length if it is a direct integer.
            let len = dict.get("Length").and_then(|o| o.as_int());
            let (raw, after) = match len {
                Some(n) if n >= 0 && data_start + n as usize <= self.buf.len() => {
                    let end = data_start + n as usize;
                    // Validate that endstream follows (allow some slack).
                    (self.buf[data_start..end].to_vec(), end)
                }
                _ => {
                    // Fallback: scan for 'endstream'.
                    let end = find_sub(&self.buf[data_start..], b"endstream")
                        .map(|p| data_start + p)
                        .unwrap_or(self.buf.len());
                    // Trim a single trailing EOL before endstream.
                    let mut e = end;
                    if e > data_start && self.buf[e - 1] == b'\n' {
                        e -= 1;
                    }
                    if e > data_start && self.buf[e - 1] == b'\r' {
                        e -= 1;
                    }
                    (self.buf[data_start..e].to_vec(), end)
                }
            };
            self.pos = after;
            Ok(Object::Stream(dict, raw))
        } else {
            Ok(Object::Dict(dict))
        }
    }

    fn parse_number_or_ref(&mut self) -> Result<Object, PdfError> {
        let start = self.pos;
        let tok = self.read_token();
        let tok_str = std::str::from_utf8(tok).unwrap_or("");
        // Could be "n g R" or "n g obj"; look ahead.
        if !tok_str.contains('.') && tok_str.parse::<i64>().is_ok() {
            let save = self.pos;
            self.skip_ws();
            let tok2_start = self.pos;
            let tok2 = self.read_token();
            if let Ok(gen_num) = std::str::from_utf8(tok2).unwrap_or("x").parse::<u16>() {
                self.skip_ws();
                let kw = self.read_token();
                if kw == b"R" {
                    let num = tok_str.parse::<u32>().unwrap_or(0);
                    return Ok(Object::Ref(num, gen_num));
                } else if kw == b"obj" {
                    // Indirect object definition; parse the inner object.
                    return self.parse_object(0);
                }
                // not a ref/obj; rewind to right after first number
                let _ = tok2_start;
                self.pos = save;
            } else {
                self.pos = save;
            }
        }
        // Plain number
        let _ = start;
        if let Ok(i) = tok_str.parse::<i64>() {
            Ok(Object::Int(i))
        } else if let Ok(r) = tok_str.parse::<f64>() {
            Ok(Object::Real(r))
        } else {
            // tolerate forms like ".5" or "-.5"
            let cleaned: String = tok_str.to_string();
            cleaned
                .parse::<f64>()
                .map(Object::Real)
                .or(Ok(Object::Int(0)))
        }
    }
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

fn find_sub(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || needle.len() > haystack.len() {
        return None;
    }
    haystack.windows(needle.len()).position(|w| w == needle)
}

fn rfind_sub(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || needle.len() > haystack.len() {
        return None;
    }
    (0..=haystack.len() - needle.len())
        .rev()
        .find(|&i| &haystack[i..i + needle.len()] == needle)
}

// ============================================================================
// Cross-reference & document
// ============================================================================

#[derive(Debug, Clone, Copy)]
enum Xref {
    /// Uncompressed object at byte offset.
    Offset(usize),
    /// Object stored in an object stream: (stream object number, index).
    InStream(u32, usize),
}

/// A parsed document with resolved cross-references and an object cache.
pub struct Document<'a> {
    data: &'a [u8],
    xref: HashMap<u32, Xref>,
    trailer: Dict,
    cache: RefCell<HashMap<u32, Object>>,
    /// Cache of decoded object streams: stream obj num -> (offsets, bytes).
    objstm_cache: RefCell<ObjStmCache>,
    /// Set when the document is encrypted and we hold its key. Streams and
    /// strings are then unwrapped as they are fetched, so nothing downstream
    /// needs to know.
    crypt: Option<crate::crypt::Decryptor>,
}

/// Decoded object stream: per-stream (object header offsets, decoded bytes).
type ObjStmCache = HashMap<u32, (Vec<(u32, usize)>, Vec<u8>)>;

impl Document<'static> {
    /// A document with no objects, for exercising code that only needs the
    /// resolver to answer "nothing here".
    #[cfg(test)]
    pub fn empty() -> Document<'static> {
        Document {
            data: &[],
            xref: HashMap::new(),
            trailer: Dict::new(),
            cache: RefCell::new(HashMap::new()),
            objstm_cache: RefCell::new(HashMap::new()),
            crypt: None,
        }
    }
}

impl<'a> Document<'a> {
    pub fn parse(data: &'a [u8]) -> Result<Self, PdfError> {
        if !data.starts_with(b"%PDF-") {
            // Some files have leading junk; tolerate up to 1KB.
            if find_sub(&data[..data.len().min(1024)], b"%PDF-").is_none() {
                return Err(PdfError::InvalidHeader);
            }
        }
        let mut doc = Document {
            data,
            xref: HashMap::new(),
            trailer: Dict::new(),
            cache: RefCell::new(HashMap::new()),
            objstm_cache: RefCell::new(HashMap::new()),
            crypt: None,
        };
        doc.build_xref()?;
        if doc.xref.is_empty() || !doc.trailer.contains_key("Root") {
            // Recovery: scan the whole file for "n g obj" definitions.
            doc.recover_by_scan();
        }
        doc.unlock();
        Ok(doc)
    }

    fn version(&self) -> String {
        let head = &self.data[..self.data.len().min(16)];
        if let Some(p) = find_sub(head, b"%PDF-") {
            let v = &head[p + 5..];
            let end = v
                .iter()
                .position(|&b| Lexer::is_ws(b))
                .unwrap_or(v.len().min(3));
            return std::str::from_utf8(&v[..end.min(v.len())])
                .unwrap_or("1.4")
                .to_string();
        }
        "1.4".to_string()
    }

    // ---- xref construction -------------------------------------------------

    fn build_xref(&mut self) -> Result<(), PdfError> {
        let sx = rfind_sub(self.data, b"startxref").ok_or(PdfError::XRefNotFound)?;
        let mut lex = Lexer::new(self.data, sx + 9);
        lex.skip_ws();
        let off = lex.read_token();
        let start_off = std::str::from_utf8(off)
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .ok_or_else(|| PdfError::InvalidXRef("bad startxref".into()))?;

        let mut visited = std::collections::HashSet::new();
        let mut next = Some(start_off);
        while let Some(o) = next {
            if o >= self.data.len() || !visited.insert(o) {
                break;
            }
            next = self.read_xref_section(o)?;
        }
        Ok(())
    }

    /// Reads one xref section (table or stream) and returns the /Prev offset.
    fn read_xref_section(&mut self, offset: usize) -> Result<Option<usize>, PdfError> {
        let mut lex = Lexer::new(self.data, offset);
        lex.skip_ws();
        if self.data[lex.pos..].starts_with(b"xref") {
            self.read_xref_table(offset)
        } else {
            self.read_xref_stream(offset)
        }
    }

    fn read_xref_table(&mut self, offset: usize) -> Result<Option<usize>, PdfError> {
        let mut lex = Lexer::new(self.data, offset + 4); // skip "xref"
        loop {
            lex.skip_ws();
            if lex.at_end() || self.data[lex.pos..].starts_with(b"trailer") {
                break;
            }
            // subsection header: start count
            let start_tok = lex.read_token();
            let start = match std::str::from_utf8(start_tok)
                .ok()
                .and_then(|s| s.parse::<u32>().ok())
            {
                Some(n) => n,
                None => break,
            };
            lex.skip_ws();
            let count_tok = lex.read_token();
            let count = std::str::from_utf8(count_tok)
                .ok()
                .and_then(|s| s.parse::<u32>().ok())
                .unwrap_or(0);
            lex.skip_ws();
            for i in 0..count {
                // each entry: 10-digit offset, 5-digit gen, type char
                lex.skip_ws();
                let off_tok = lex.read_token();
                lex.skip_ws();
                let _gen_tok = lex.read_token();
                lex.skip_ws();
                let type_tok = lex.read_token();
                let num = start + i;
                if type_tok == b"n"
                    && let Some(off) = std::str::from_utf8(off_tok)
                        .ok()
                        .and_then(|s| s.parse::<usize>().ok())
                {
                    self.xref.entry(num).or_insert(Xref::Offset(off));
                }
            }
        }
        // trailer dictionary
        lex.skip_ws();
        if self.data[lex.pos..].starts_with(b"trailer") {
            lex.pos += 7;
            let trailer = lex.parse_object(0)?;
            if let Object::Dict(d) = trailer {
                let prev = d.get("Prev").and_then(|o| o.as_int()).map(|n| n as usize);
                // A hybrid file may also carry /XRefStm.
                let xrefstm = d
                    .get("XRefStm")
                    .and_then(|o| o.as_int())
                    .map(|n| n as usize);
                self.merge_trailer(d);
                if let Some(xs) = xrefstm {
                    let _ = self.read_xref_stream(xs);
                }
                return Ok(prev);
            }
        }
        Ok(None)
    }

    fn read_xref_stream(&mut self, offset: usize) -> Result<Option<usize>, PdfError> {
        let obj = self.parse_indirect_at(offset)?;
        let (dict, raw) = match obj {
            Object::Stream(d, r) => (d, r),
            _ => return Ok(None),
        };
        let decoded = decode_stream(&dict, &raw)?;
        let w = dict
            .get("W")
            .and_then(|o| o.as_array())
            .ok_or_else(|| PdfError::InvalidXRef("xref stream missing /W".into()))?;
        let w: Vec<usize> = w
            .iter()
            .filter_map(|o| o.as_int().map(|n| n as usize))
            .collect();
        if w.len() < 3 {
            return Err(PdfError::InvalidXRef("bad /W".into()));
        }
        let (w0, w1, w2) = (w[0], w[1], w[2]);
        let row = w0 + w1 + w2;
        if row == 0 {
            return Err(PdfError::InvalidXRef("zero-width xref row".into()));
        }
        let size = dict.get("Size").and_then(|o| o.as_int()).unwrap_or(0) as u32;

        // /Index pairs [start count ...]; default [0 Size].
        let index: Vec<i64> = dict
            .get("Index")
            .and_then(|o| o.as_array())
            .map(|a| a.iter().filter_map(|o| o.as_int()).collect())
            .unwrap_or_else(|| vec![0, size as i64]);

        let mut pos = 0usize;
        let read_field = |bytes: &[u8], width: usize| -> u64 {
            let mut v = 0u64;
            for &b in bytes.iter().take(width) {
                v = (v << 8) | b as u64;
            }
            v
        };

        for chunk in index.chunks(2) {
            let start = chunk[0];
            let count = if chunk.len() == 2 { chunk[1] } else { 0 };
            for k in 0..count {
                if pos + row > decoded.len() {
                    break;
                }
                let f0 = if w0 == 0 {
                    1
                } else {
                    read_field(&decoded[pos..], w0)
                };
                let f1 = read_field(&decoded[pos + w0..], w1);
                let f2 = read_field(&decoded[pos + w0 + w1..], w2);
                pos += row;
                let num = (start + k) as u32;
                match f0 {
                    1 => {
                        self.xref.entry(num).or_insert(Xref::Offset(f1 as usize));
                    }
                    2 => {
                        self.xref
                            .entry(num)
                            .or_insert(Xref::InStream(f1 as u32, f2 as usize));
                    }
                    _ => {}
                }
            }
        }

        let prev = dict
            .get("Prev")
            .and_then(|o| o.as_int())
            .map(|n| n as usize);
        self.merge_trailer(dict);
        Ok(prev)
    }

    fn merge_trailer(&mut self, d: Dict) {
        for (k, v) in d {
            self.trailer.entry(k).or_insert(v);
        }
    }

    /// Last-resort recovery: scan the file for `N G obj` and index offsets.
    fn recover_by_scan(&mut self) {
        let data = self.data;
        let mut i = 0;
        let mut count = 0;
        while i + 3 < data.len() && count < MAX_OBJECTS {
            if &data[i..i + 3] == b"obj"
                && (i + 3 >= data.len()
                    || Lexer::is_ws(data[i + 3])
                    || Lexer::is_delim(data[i + 3]))
            {
                // walk back over: ws gen ws num
                let mut j = i;
                // skip back ws
                while j > 0 && Lexer::is_ws(data[j - 1]) {
                    j -= 1;
                }
                let gen_end = j;
                while j > 0 && data[j - 1].is_ascii_digit() {
                    j -= 1;
                }
                let gen_start = j;
                while j > 0 && Lexer::is_ws(data[j - 1]) {
                    j -= 1;
                }
                let num_end = j;
                while j > 0 && data[j - 1].is_ascii_digit() {
                    j -= 1;
                }
                let num_start = j;
                if num_start < num_end
                    && gen_start < gen_end
                    && let Ok(num) = std::str::from_utf8(&data[num_start..num_end])
                        .unwrap_or("")
                        .parse::<u32>()
                {
                    self.xref.insert(num, Xref::Offset(num_start));
                    count += 1;
                }
            }
            i += 1;
        }
        // Find trailer / Root if not set.
        if !self.trailer.contains_key("Root")
            && let Some(tp) = rfind_sub(data, b"trailer")
        {
            let mut lex = Lexer::new(data, tp + 7);
            if let Ok(Object::Dict(d)) = lex.parse_object(0) {
                self.merge_trailer(d);
            }
        }
        if !self.trailer.contains_key("Root") {
            // Scan objects for /Type /Catalog.
            let keys: Vec<u32> = self.xref.keys().copied().collect();
            for num in keys {
                if let Ok(obj) = self.fetch(num)
                    && let Some(d) = obj.as_dict()
                    && d.get("Type").and_then(|o| o.as_name()) == Some("Catalog")
                {
                    self.trailer.insert("Root".into(), Object::Ref(num, 0));
                    break;
                }
            }
        }
    }

    // ---- object fetching ---------------------------------------------------

    fn parse_indirect_at(&self, offset: usize) -> Result<Object, PdfError> {
        if offset >= self.data.len() {
            return Err(PdfError::ObjectParseError("offset out of bounds".into()));
        }
        let mut lex = Lexer::new(self.data, offset);
        lex.skip_ws();
        // Expect "num gen obj"
        let _n = lex.read_token();
        lex.skip_ws();
        let _g = lex.read_token();
        lex.skip_ws();
        let kw = lex.read_token();
        if kw != b"obj" {
            // Maybe offset already points at the object body.
            let mut l2 = Lexer::new(self.data, offset);
            return l2.parse_object(0);
        }
        lex.parse_object(0)
    }

    /// Work out the document's encryption key, if it has one.
    ///
    /// Called once, before anything is fetched, because the `/Encrypt`
    /// dictionary and the file identifier are themselves read from the
    /// trailer -- and, unlike everything else, are not encrypted.
    fn unlock(&mut self) {
        let Some(encrypt) = self.trailer.get("Encrypt").cloned() else {
            return;
        };
        // The dictionary may be indirect; resolving it now is safe because
        // `crypt` is still `None` and so nothing is unwrapped.
        let encrypt = match encrypt {
            Object::Dict(d) => d,
            Object::Ref(n, _) => match self.fetch(n) {
                Ok(Object::Dict(d)) => d,
                _ => return,
            },
            _ => return,
        };
        let id = match self.trailer.get("ID") {
            Some(Object::Array(a)) => match a.first() {
                Some(Object::Str(b)) => b.clone(),
                _ => Vec::new(),
            },
            _ => Vec::new(),
        };
        self.crypt = crate::crypt::Decryptor::new(&encrypt, &id);
        // The `/Encrypt` dictionary itself was read before the key existed;
        // drop the cache so everything is read again through it.
        self.cache.borrow_mut().clear();
        self.objstm_cache.borrow_mut().clear();
    }

    /// Whether the document is encrypted in a way this cannot read.
    ///
    /// Encrypted and readable is the common case -- an empty user password
    /// with permissions the file cannot enforce -- and says nothing here.
    pub fn is_locked(&self) -> bool {
        self.trailer.contains_key("Encrypt") && self.crypt.is_none()
    }

    /// Unwrap the strings and stream of a freshly parsed object.
    fn decrypt_object(&self, number: u32, obj: Object) -> Object {
        let Some(crypt) = &self.crypt else {
            return obj;
        };
        fn walk(crypt: &crate::crypt::Decryptor, number: u32, obj: Object) -> Object {
            match obj {
                Object::Str(bytes) => Object::Str(crypt.decrypt(number, 0, &bytes)),
                Object::Array(items) => Object::Array(
                    items
                        .into_iter()
                        .map(|item| walk(crypt, number, item))
                        .collect(),
                ),
                Object::Dict(dict) => Object::Dict(
                    dict.into_iter()
                        .map(|(k, v)| (k, walk(crypt, number, v)))
                        .collect(),
                ),
                Object::Stream(dict, raw) => {
                    // An `XRef` stream is never encrypted: it has to be
                    // readable to find the key in the first place.
                    let plain = matches!(
                        dict.get("Type"),
                        Some(Object::Name(name)) if name == "XRef"
                    );
                    let bytes = match plain {
                        true => raw,
                        false => crypt.decrypt(number, 0, &raw),
                    };
                    let dict = match walk(crypt, number, Object::Dict(dict)) {
                        Object::Dict(d) => d,
                        _ => Dict::new(),
                    };
                    Object::Stream(dict, bytes)
                }
                other => other,
            }
        }
        walk(crypt, number, obj)
    }

    /// Fetch an object by number, resolving object-stream membership, cached.
    pub fn fetch(&self, num: u32) -> Result<Object, PdfError> {
        if let Some(o) = self.cache.borrow().get(&num) {
            return Ok(o.clone());
        }
        let entry = match self.xref.get(&num) {
            Some(e) => *e,
            None => return Ok(Object::Null),
        };
        let obj = match entry {
            Xref::Offset(off) => {
                let parsed = self.parse_indirect_at(off)?;
                // Objects inside an object stream are not wrapped separately:
                // the stream that carried them was.
                self.decrypt_object(num, parsed)
            }
            Xref::InStream(snum, idx) => self.fetch_from_objstm(snum, idx)?,
        };
        self.cache.borrow_mut().insert(num, obj.clone());
        Ok(obj)
    }

    fn fetch_from_objstm(&self, snum: u32, idx: usize) -> Result<Object, PdfError> {
        // Build/lookup decoded object stream.
        if !self.objstm_cache.borrow().contains_key(&snum) {
            let stream_obj = match self.xref.get(&snum) {
                Some(Xref::Offset(off)) => self.parse_indirect_at(*off)?,
                _ => return Ok(Object::Null),
            };
            let (dict, raw) = match stream_obj {
                Object::Stream(d, r) => (d, r),
                _ => return Ok(Object::Null),
            };
            let decoded = decode_stream(&dict, &raw)?;
            let n = dict.get("N").and_then(|o| o.as_int()).unwrap_or(0) as usize;
            let first = dict.get("First").and_then(|o| o.as_int()).unwrap_or(0) as usize;
            // Header: N pairs of "objnum offset".
            let mut header = Vec::with_capacity(n);
            let mut hl = Lexer::new(&decoded, 0);
            for _ in 0..n {
                hl.skip_ws();
                let a = hl.read_token();
                hl.skip_ws();
                let b = hl.read_token();
                let onum = std::str::from_utf8(a)
                    .ok()
                    .and_then(|s| s.parse::<u32>().ok());
                let ooff = std::str::from_utf8(b)
                    .ok()
                    .and_then(|s| s.parse::<usize>().ok());
                if let (Some(on), Some(of)) = (onum, ooff) {
                    header.push((on, first + of));
                }
            }
            self.objstm_cache
                .borrow_mut()
                .insert(snum, (header, decoded));
        }
        let cache = self.objstm_cache.borrow();
        let (header, decoded) = cache.get(&snum).unwrap();
        if idx >= header.len() {
            return Ok(Object::Null);
        }
        let (_onum, start) = header[idx];
        let mut lex = Lexer::new(decoded, start);
        lex.parse_object(0)
    }

    /// Resolve one level of indirection.
    /// Every object number the cross-reference table knows.
    ///
    /// Walking objects directly is how font programs are found: a font can be
    /// shared between pages, reached through nested form XObjects, or sit in a
    /// resource dictionary nothing else has reason to visit.
    pub fn object_numbers(&self) -> Vec<u32> {
        let mut numbers: Vec<u32> = self.xref.keys().copied().collect();
        // Sorted so results do not depend on hash iteration order.
        numbers.sort_unstable();
        numbers
    }

    pub fn resolve(&self, obj: &Object) -> Object {
        match obj {
            Object::Ref(n, _) => self.fetch(*n).unwrap_or(Object::Null),
            other => other.clone(),
        }
    }

    /// Resolve a dict entry, dereferencing if it is a reference.
    pub fn get(&self, dict: &Dict, key: &str) -> Option<Object> {
        dict.get(key).map(|o| self.resolve(o))
    }
}

// ============================================================================
// Stream decoding (Flate + predictors; ASCIIHex/ASCII85 passthrough-ish)
// ============================================================================

pub fn decode_stream(dict: &Dict, raw: &[u8]) -> Result<Vec<u8>, PdfError> {
    let filters: Vec<String> = match dict.get("Filter") {
        Some(Object::Name(n)) => vec![n.clone()],
        Some(Object::Array(a)) => a
            .iter()
            .filter_map(|o| o.as_name().map(String::from))
            .collect(),
        _ => Vec::new(),
    };
    if filters.is_empty() {
        return Ok(raw.to_vec());
    }
    // Collect DecodeParms (may be single dict or array).
    let parms: Vec<Option<Dict>> = match dict.get("DecodeParms").or_else(|| dict.get("DP")) {
        Some(Object::Dict(d)) => vec![Some(d.clone())],
        Some(Object::Array(a)) => a
            .iter()
            .map(|o| match o {
                Object::Dict(d) => Some(d.clone()),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    };

    let mut data = raw.to_vec();
    for (i, f) in filters.iter().enumerate() {
        let parm = parms.get(i).cloned().flatten();
        match f.as_str() {
            "FlateDecode" | "Fl" => {
                // `/Colors` and `/BitsPerComponent` describe the row as much
                // as `/Columns` does: a row is Columns *samples* wide. They
                // travel together so a caller cannot read one and forget the
                // rest, which is how every predicted colour image used to be
                // thrown away.
                let number = |key: &str| {
                    parm.as_ref()
                        .and_then(|p| p.get(key))
                        .and_then(|o| o.as_int())
                };
                let predictor = number("Predictor").map(|n| crate::parser::Predictor {
                    predictor: n as u8,
                    columns: number("Columns").unwrap_or(1).max(1) as usize,
                    colors: number("Colors").unwrap_or(1).max(1) as usize,
                    bits: number("BitsPerComponent").unwrap_or(8).max(1) as u8,
                });
                data = PdfParser::decompress_stream(&data, predictor)?;
            }
            "ASCIIHexDecode" | "AHx" => {
                data = ascii_hex_decode(&data);
            }
            "ASCII85Decode" | "A85" => {
                data = ascii85_decode(&data);
            }
            // LZW/DCT/CCITT/JPX not handled here; leave bytes as-is so callers
            // that only need text from Flate streams still work.
            _ => {}
        }
    }
    Ok(data)
}

fn ascii_hex_decode(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut hi = None;
    for &b in data {
        if b == b'>' {
            break;
        }
        if let Some(v) = hex_val(b) {
            match hi {
                None => hi = Some(v),
                Some(h) => {
                    out.push(h * 16 + v);
                    hi = None;
                }
            }
        }
    }
    if let Some(h) = hi {
        out.push(h * 16);
    }
    out
}

fn ascii85_decode(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut tuple = [0u8; 5];
    let mut count = 0;
    let mut i = 0;
    // Skip optional <~ prefix
    if data.starts_with(b"<~") {
        i = 2;
    }
    while i < data.len() {
        let b = data[i];
        i += 1;
        if b == b'~' {
            break;
        }
        if Lexer::is_ws(b) {
            continue;
        }
        if b == b'z' && count == 0 {
            out.extend_from_slice(&[0, 0, 0, 0]);
            continue;
        }
        if !(b'!'..=b'u').contains(&b) {
            continue;
        }
        tuple[count] = b - b'!';
        count += 1;
        if count == 5 {
            let mut val = 0u32;
            for &t in &tuple {
                val = val.wrapping_mul(85).wrapping_add(t as u32);
            }
            out.extend_from_slice(&val.to_be_bytes());
            count = 0;
        }
    }
    if count > 0 {
        tuple[count..5].fill(84);
        let mut val = 0u32;
        for &t in &tuple {
            val = val.wrapping_mul(85).wrapping_add(t as u32);
        }
        let bytes = val.to_be_bytes();
        out.extend_from_slice(&bytes[..count - 1]);
    }
    out
}

// ============================================================================
// Public entry point: build a PdfDocument
// ============================================================================

/// Parse raw bytes into the public `PdfDocument` model.
pub fn parse_document(data: &[u8]) -> Result<PdfDocument, PdfError> {
    let doc = Document::parse(data)?;
    let version = doc.version();

    // Resolve catalog.
    let root = doc
        .trailer
        .get("Root")
        .map(|o| doc.resolve(o))
        .and_then(|o| o.as_dict().cloned())
        .unwrap_or_default();

    // Build page list by walking the tree.
    let mut page_dicts: Vec<Dict> = Vec::new();
    if let Some(pages_obj) = doc.get(&root, "Pages")
        && let Some(pages_dict) = pages_obj.as_dict()
    {
        let mut visited = std::collections::HashSet::new();
        collect_pages(
            &doc,
            pages_dict,
            &mut page_dicts,
            &Inherited::default(),
            0,
            &mut visited,
        );
    }

    // Extract per-page content.
    let mut pages = Vec::with_capacity(page_dicts.len());
    let mut annotations = Vec::new();
    for (idx, pd) in page_dicts.iter().enumerate() {
        let (w, h) = view_size(&doc, pd);
        let content = extract_page_content(&doc, pd, idx + 1);
        let mut text = content.text;
        crate::text_norm::merge_positional_accents(&mut text);
        pages.push(PdfPage {
            index: idx,
            width: w,
            height: h,
            text_content: text,
        });
        collect_annotations(&doc, pd, idx + 1, &mut annotations);
    }

    // Metadata.
    let mut metadata = PdfMetadata {
        pdf_version: version.clone(),
        file_size: data.len(),
        page_count: pages.len(),
        encrypted: doc.trailer.contains_key("Encrypt"),
        has_annotations: !annotations.is_empty(),
        ..Default::default()
    };
    if let Some(info_obj) = doc.trailer.get("Info")
        && let Some(info) = doc.resolve(info_obj).as_dict()
    {
        metadata.title = text_string(&doc, info, "Title");
        metadata.author = text_string(&doc, info, "Author");
        metadata.subject = text_string(&doc, info, "Subject");
        metadata.creator = text_string(&doc, info, "Creator");
        metadata.producer = text_string(&doc, info, "Producer");
        metadata.creation_date = text_string(&doc, info, "CreationDate");
        metadata.modification_date = text_string(&doc, info, "ModDate");
    }
    metadata.has_forms = doc.get(&root, "AcroForm").is_some();
    let outline = parse_outline(&doc, &root);
    metadata.has_outlines = !outline.is_empty();

    Ok(PdfDocument::from_parts(
        version,
        pages,
        metadata,
        annotations,
        outline,
    ))
}

/// Enumerate image XObjects across all pages with dimensions and encoding.
pub fn extract_images(data: &[u8]) -> Result<Vec<ImageInfo>, PdfError> {
    let doc = Document::parse(data)?;
    let root = doc
        .trailer
        .get("Root")
        .map(|o| doc.resolve(o))
        .and_then(|o| o.as_dict().cloned())
        .unwrap_or_default();
    let mut page_dicts: Vec<Dict> = Vec::new();
    if let Some(pages_obj) = doc.get(&root, "Pages")
        && let Some(pages_dict) = pages_obj.as_dict()
    {
        let mut visited = std::collections::HashSet::new();
        collect_pages(
            &doc,
            pages_dict,
            &mut page_dicts,
            &Inherited::default(),
            0,
            &mut visited,
        );
    }

    let mut images = Vec::new();
    let mut counter = 0usize;
    for (idx, pd) in page_dicts.iter().enumerate() {
        let resources = match doc.get(pd, "Resources").and_then(|o| o.as_dict().cloned()) {
            Some(r) => r,
            None => continue,
        };
        let xobjects = match doc
            .get(&resources, "XObject")
            .and_then(|o| o.as_dict().cloned())
        {
            Some(x) => x,
            None => continue,
        };
        for xref in xobjects.values() {
            let obj = doc.resolve(xref);
            if let Object::Stream(d, raw) = &obj {
                if doc
                    .get(d, "Subtype")
                    .and_then(|o| o.as_name().map(String::from))
                    .as_deref()
                    != Some("Image")
                {
                    continue;
                }
                let width = doc.get(d, "Width").and_then(|o| o.as_int()).unwrap_or(0) as u32;
                let height = doc.get(d, "Height").and_then(|o| o.as_int()).unwrap_or(0) as u32;
                let bpc = doc
                    .get(d, "BitsPerComponent")
                    .and_then(|o| o.as_int())
                    .unwrap_or(8) as u8;
                let color_space = color_space_name(&doc, d);
                let filter = filter_names(d);
                images.push(ImageInfo {
                    id: format!("img_{}", counter),
                    width,
                    height,
                    color_space,
                    bits_per_component: bpc,
                    filter,
                    page_number: idx + 1,
                    data_offset: 0,
                    data_length: raw.len(),
                });
                counter += 1;
            }
        }
    }
    Ok(images)
}

/// A node in a tagged PDF's logical structure tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructNode {
    /// Structure type from `/S` (e.g. "Document", "H1", "P", "Table", "Figure").
    pub kind: String,
    /// Author-provided text: `/ActualText`, else `/Alt`, else `/T` (title).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// 1-based page the element is on (`/Pg`), if known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_number: Option<usize>,
    pub children: Vec<StructNode>,
}

/// Extract the tagged-PDF logical structure tree from `/StructTreeRoot`.
///
/// This is the author-provided structure (headings, lists, tables, reading
/// order) — the highest-accuracy source when present, requiring no heuristics.
/// Returns an empty vec for untagged documents.
/// An AcroForm field (interactive form), à la PDF.js `getFieldObjects`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormField {
    /// Fully-qualified field name (parent.child).
    pub name: String,
    /// "text" | "button" | "choice" | "signature" | "unknown".
    pub field_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_value: Option<String>,
    /// Widget rectangle [x1, y1, x2, y2], if the field has one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rect: Option<[f64; 4]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_number: Option<usize>,
    /// Selectable options for choice fields.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<String>,
    pub required: bool,
    pub read_only: bool,
}

/// Extract interactive AcroForm fields (names, types, values, positions).
pub fn extract_form_fields(data: &[u8]) -> Result<Vec<FormField>, PdfError> {
    let doc = Document::parse(data)?;
    let root = doc
        .trailer
        .get("Root")
        .map(|o| doc.resolve(o))
        .and_then(|o| o.as_dict().cloned())
        .unwrap_or_default();
    let acro = match doc
        .get(&root, "AcroForm")
        .and_then(|o| o.as_dict().cloned())
    {
        Some(a) => a,
        None => return Ok(Vec::new()),
    };
    let fields = match doc
        .get(&acro, "Fields")
        .and_then(|o| o.as_array().map(|a| a.to_vec()))
    {
        Some(f) => f,
        None => return Ok(Vec::new()),
    };
    let pages = build_page_index(&doc, &root);
    let mut out = Vec::new();
    let mut visited = std::collections::HashSet::new();
    for f in &fields {
        walk_field(&doc, f, "", None, &pages, &mut out, &mut visited, 0);
    }
    Ok(out)
}

#[allow(clippy::too_many_arguments)]
fn walk_field(
    doc: &Document,
    field: &Object,
    parent_name: &str,
    inherited_ft: Option<&str>,
    pages: &HashMap<u32, usize>,
    out: &mut Vec<FormField>,
    visited: &mut std::collections::HashSet<u32>,
    depth: usize,
) {
    if depth > MAX_DEPTH {
        return;
    }
    if let Object::Ref(n, _) = field
        && !visited.insert(*n)
    {
        return;
    }
    let d = match doc.resolve(field).as_dict().cloned() {
        Some(d) => d,
        None => return,
    };
    let partial = text_string(doc, &d, "T");
    let full = match &partial {
        Some(p) if parent_name.is_empty() => p.clone(),
        Some(p) => format!("{parent_name}.{p}"),
        None => parent_name.to_string(),
    };
    let ft = doc
        .get(&d, "FT")
        .and_then(|o| o.as_name().map(String::from))
        .or_else(|| inherited_ft.map(String::from));

    // Kids that are themselves fields (have /T) make this a non-terminal node.
    let kids = doc
        .get(&d, "Kids")
        .and_then(|o| o.as_array().map(|a| a.to_vec()));
    let field_kids: Vec<Object> = kids
        .as_ref()
        .map(|ks| {
            ks.iter()
                .filter(|k| {
                    doc.resolve(k)
                        .as_dict()
                        .map(|kd| kd.contains_key("T"))
                        .unwrap_or(false)
                })
                .cloned()
                .collect()
        })
        .unwrap_or_default();

    if !field_kids.is_empty() {
        for k in &field_kids {
            walk_field(doc, k, &full, ft.as_deref(), pages, out, visited, depth + 1);
        }
        return;
    }

    // Terminal field: emit if it looks like one (has a type or a name).
    if ft.is_none() && partial.is_none() {
        return;
    }
    let field_type = match ft.as_deref() {
        Some("Tx") => "text",
        Some("Btn") => "button",
        Some("Ch") => "choice",
        Some("Sig") => "signature",
        _ => "unknown",
    }
    .to_string();

    let value = field_value(doc, &d, "V");
    let default_value = field_value(doc, &d, "DV");
    // Rect from the field, else from a widget kid.
    let rect = rect4(doc, &d, "Rect").or_else(|| {
        kids.as_ref().and_then(|ks| {
            ks.iter().find_map(|k| {
                doc.resolve(k)
                    .as_dict()
                    .and_then(|kd| rect4(doc, kd, "Rect"))
            })
        })
    });
    let page_number = d
        .get("P")
        .and_then(|o| o.as_ref())
        .or_else(|| {
            kids.as_ref().and_then(|ks| {
                ks.iter().find_map(|k| {
                    doc.resolve(k)
                        .as_dict()
                        .and_then(|kd| kd.get("P").and_then(|o| o.as_ref()))
                })
            })
        })
        .and_then(|(n, _)| pages.get(&n).map(|i| i + 1));
    let options = doc
        .get(&d, "Opt")
        .and_then(|o| o.as_array().map(|a| a.to_vec()))
        .map(|arr| {
            arr.iter()
                .filter_map(|o| match doc.resolve(o) {
                    Object::Str(b) => Some(decode_text_string(&b)),
                    // [export, display] pair → take the display string.
                    Object::Array(pair) => pair
                        .last()
                        .and_then(|x| x.as_str_bytes().map(decode_text_string)),
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default();
    let flags = doc.get(&d, "Ff").and_then(|o| o.as_int()).unwrap_or(0);

    out.push(FormField {
        name: full,
        field_type,
        value,
        default_value,
        rect,
        page_number,
        options,
        read_only: flags & 1 != 0,
        required: flags & 2 != 0,
    });
}

/// A form field value: a text string or a button/checkbox state name.
fn field_value(doc: &Document, d: &Dict, key: &str) -> Option<String> {
    match doc.get(d, key)? {
        Object::Str(b) => Some(decode_text_string(&b)),
        Object::Name(n) => Some(n),
        _ => None,
    }
}

pub fn extract_structure(data: &[u8]) -> Result<Vec<StructNode>, PdfError> {
    let doc = Document::parse(data)?;
    let root = doc
        .trailer
        .get("Root")
        .map(|o| doc.resolve(o))
        .and_then(|o| o.as_dict().cloned())
        .unwrap_or_default();
    let str_root = match doc
        .get(&root, "StructTreeRoot")
        .and_then(|o| o.as_dict().cloned())
    {
        Some(d) => d,
        None => return Ok(Vec::new()),
    };
    let pages = build_page_index(&doc, &root);
    let mut visited = std::collections::HashSet::new();
    let k = match str_root.get("K") {
        Some(k) => k.clone(),
        None => return Ok(Vec::new()),
    };
    Ok(walk_struct_kids(&doc, &k, &pages, &mut visited, 0))
}

/// Map page object numbers to 0-based page indices.
fn build_page_index(doc: &Document, root: &Dict) -> HashMap<u32, usize> {
    let mut map = HashMap::new();
    if let Some(pages) = doc.get(root, "Pages").and_then(|o| o.as_dict().cloned()) {
        let mut counter = 0usize;
        let mut visited = std::collections::HashSet::new();
        page_index_walk(doc, &pages, &mut map, &mut counter, &mut visited, 0);
    }
    map
}

fn page_index_walk(
    doc: &Document,
    node: &Dict,
    map: &mut HashMap<u32, usize>,
    counter: &mut usize,
    visited: &mut std::collections::HashSet<u32>,
    depth: usize,
) {
    if depth > MAX_DEPTH {
        return;
    }
    let kids = match doc
        .get(node, "Kids")
        .and_then(|o| o.as_array().map(|a| a.to_vec()))
    {
        Some(k) => k,
        None => return,
    };
    for kid in &kids {
        if let Object::Ref(n, _) = kid {
            if !visited.insert(*n) {
                continue;
            }
            let kd = match doc.resolve(kid).as_dict().cloned() {
                Some(d) => d,
                None => continue,
            };
            let is_page = kd.get("Type").and_then(|o| o.as_name()) == Some("Page")
                || !kd.contains_key("Kids");
            if is_page {
                map.insert(*n, *counter);
                *counter += 1;
            } else {
                page_index_walk(doc, &kd, map, counter, visited, depth + 1);
            }
        }
    }
}

/// Walk the `/K` contents of a structure element or the tree root.
fn walk_struct_kids(
    doc: &Document,
    k: &Object,
    pages: &HashMap<u32, usize>,
    visited: &mut std::collections::HashSet<u32>,
    depth: usize,
) -> Vec<StructNode> {
    if depth > MAX_DEPTH {
        return Vec::new();
    }
    let mut out = Vec::new();
    match k {
        Object::Array(items) => {
            for item in items {
                out.extend(walk_struct_kids(doc, item, pages, visited, depth));
            }
        }
        Object::Ref(n, _) if visited.insert(*n) => {
            let resolved = doc.resolve(k);
            if let Some(node) = struct_elem(doc, &resolved, pages, visited, depth) {
                out.push(node);
            }
        }
        Object::Dict(_) => {
            if let Some(node) = struct_elem(doc, k, pages, visited, depth) {
                out.push(node);
            }
        }
        // Integers / MCR / OBJR are content leaves, not structural elements.
        _ => {}
    }
    out
}

/// Build a `StructNode` from a structure-element object, if it is one.
fn struct_elem(
    doc: &Document,
    obj: &Object,
    pages: &HashMap<u32, usize>,
    visited: &mut std::collections::HashSet<u32>,
    depth: usize,
) -> Option<StructNode> {
    let d = obj.as_dict()?;
    // A marked-content (/MCR) or object (/OBJR) reference is a leaf, not an elem.
    let ty = d.get("Type").and_then(|o| o.as_name());
    if matches!(ty, Some("MCR") | Some("OBJR")) {
        return None;
    }
    let kind = doc
        .get(d, "S")
        .and_then(|o| o.as_name().map(String::from))?;
    let text = text_string(doc, d, "ActualText")
        .or_else(|| text_string(doc, d, "Alt"))
        .or_else(|| text_string(doc, d, "T"));
    let page_number = d
        .get("Pg")
        .and_then(|o| o.as_ref())
        .and_then(|(n, _)| pages.get(&n).map(|i| i + 1));
    let children = match d.get("K") {
        Some(k) => walk_struct_kids(doc, k, pages, visited, depth + 1),
        None => Vec::new(),
    };
    Some(StructNode {
        kind,
        text,
        page_number,
        children,
    })
}

/// Extract per-page ruling-line geometry (for table reconstruction).
pub fn extract_graphics(data: &[u8]) -> Result<Vec<PageGraphics>, PdfError> {
    let doc = Document::parse(data)?;
    let root = doc
        .trailer
        .get("Root")
        .map(|o| doc.resolve(o))
        .and_then(|o| o.as_dict().cloned())
        .unwrap_or_default();
    let mut page_dicts: Vec<Dict> = Vec::new();
    if let Some(pages_obj) = doc.get(&root, "Pages")
        && let Some(pages_dict) = pages_obj.as_dict()
    {
        let mut visited = std::collections::HashSet::new();
        collect_pages(
            &doc,
            pages_dict,
            &mut page_dicts,
            &Inherited::default(),
            0,
            &mut visited,
        );
    }
    let mut out = Vec::with_capacity(page_dicts.len());
    for (idx, pd) in page_dicts.iter().enumerate() {
        let (w, h) = view_size(&doc, pd);
        let content = extract_page_content(&doc, pd, idx + 1);
        out.push(PageGraphics {
            page_number: idx + 1,
            width: w,
            height: h,
            h_lines: content.h_lines,
            v_lines: content.v_lines,
        });
    }
    Ok(out)
}

// ============================================================================
// Display list — device-space draw primitives for a GPU renderer
// ============================================================================

/// A single drawing primitive in device (PDF point) space, ready for a
/// rasterizer. Beziers are pre-flattened to polylines so a GPU renderer only
/// has to handle polygons (fills) and polylines (strokes).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "lowercase")]
pub enum RenderOp {
    /// Fill closed subpaths (each a polyline of [x, y] points).
    Fill {
        subpaths: Vec<Vec<[f64; 2]>>,
        color: [f64; 4],
        even_odd: bool,
    },
    /// Stroke subpaths with `width` (device units) and `color`.
    Stroke {
        subpaths: Vec<Vec<[f64; 2]>>,
        color: [f64; 4],
        width: f64,
    },
    /// A text run anchored at its baseline origin.
    Text {
        text: String,
        x: f64,
        y: f64,
        size: f64,
        /// Target advance width in device units (fit rendered glyphs to this).
        width: f64,
        /// Per-character advance in device units (PDF glyph widths), parallel to
        /// the Unicode chars of `text`. Lets a renderer place each glyph at the
        /// exact PDF cumulative position (matching the reference renderer).
        advances: Vec<f64>,
        /// The character code that produced each character of `text`, parallel
        /// to it. A glyph is selected by code, not by the Unicode the code
        /// happens to mean, so a renderer drawing the document's own embedded
        /// font needs this rather than the text.
        #[serde(default)]
        codes: Vec<u32>,
        /// True when the PDF lacked explicit glyph widths (advances are a flat
        /// default); a renderer should measure real glyph widths instead.
        measured: bool,
        /// Baseline rotation in radians (0 for horizontal text).
        rot: f64,
        color: [f64; 4],
        font: String,
        /// The object number of the font dictionary this run was set in.
        ///
        /// `/BaseFont` is not an identity: one document embeds two subsets of
        /// PTSans-Bold under the same name, and a renderer that picks between
        /// them by name draws whichever answers first. One of those maps `T`
        /// to a glyph that draws `q`. Zero where the font was written inline
        /// rather than referenced, which no producer does.
        #[serde(default)]
        face: u32,
    },
    /// An image XObject placement; `name` is the page resource key.
    Image {
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        name: String,
        /// The constant alpha the image is painted with (the fill alpha in
        /// force, `ca`). A picture at `ca 0.5` is a watermark, and drawing it
        /// opaque puts a photograph over the text it was meant to sit behind.
        ///
        /// Defaults to 1 so a display list written before this field existed
        /// still reads as fully opaque rather than invisible.
        #[serde(default = "opaque")]
        alpha: f64,
        /// The colour to paint this image in, for an `/ImageMask`.
        ///
        /// A mask is a stencil rather than a picture: one bit a sample, no
        /// colour space, and what it paints is the fill colour in force at the
        /// `Do`. The texture cannot carry that -- the same stencil may be
        /// painted twice on one page in two colours -- so it carries coverage
        /// in its alpha and the colour rides on the op. `None` for an ordinary
        /// image, which is every image that is not a mask.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tint: Option<[f64; 4]>,
        /// The `[a b c d]` of the transform that placed the image, in device
        /// space, where `x`/`y`/`w`/`h` are only its bounding box.
        ///
        /// An image is painted into the *unit square* under the current
        /// transform, so that transform is free to turn it, mirror it or shear
        /// it. A bounding box keeps none of that: a photograph placed with a
        /// quarter turn comes out lying on its side and stretched to the box
        /// it would have occupied. `None` where a display list predates the
        /// field, and it is then drawn upright as it always was.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        mat: Option<[f64; 4]>,
    },
    /// Save graphics state (push the clip/scissor stack).
    Save,
    /// Restore graphics state (pop the clip/scissor stack).
    Restore,
    /// Intersect the clip region with a path: `rect` is its bounding box (a
    /// fast scissor pre-clip) and `subpaths` the polygon(s) for an exact
    /// stencil clip.
    Clip {
        rect: [f64; 4],
        subpaths: Vec<Vec<[f64; 2]>>,
    },
}

/// The alpha an image is assumed to carry when a display list does not say.
///
/// `#[serde(default)]` on an `f64` is zero, and zero alpha means "do not draw
/// it at all" — so an older display list, or one written by a producer that
/// never set the field, would lose every picture on the page.
fn opaque() -> f64 {
    1.0
}

/// A page's display list in device space (origin bottom-left, y up — PDF
/// convention; a renderer flips to its own coordinate system).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayList {
    pub page_number: usize,
    pub width: f64,
    pub height: f64,
    pub ops: Vec<RenderOp>,
}

/// Extract a device-space display list for one page (1-based).
pub fn extract_display_list(data: &[u8], page_number: usize) -> Result<DisplayList, PdfError> {
    let doc = Document::parse(data)?;
    let root = doc
        .trailer
        .get("Root")
        .map(|o| doc.resolve(o))
        .and_then(|o| o.as_dict().cloned())
        .unwrap_or_default();
    let mut page_dicts: Vec<Dict> = Vec::new();
    if let Some(pages_obj) = doc.get(&root, "Pages")
        && let Some(pages_dict) = pages_obj.as_dict()
    {
        let mut visited = std::collections::HashSet::new();
        collect_pages(
            &doc,
            pages_dict,
            &mut page_dicts,
            &Inherited::default(),
            0,
            &mut visited,
        );
    }
    let pd = page_dicts
        .get(page_number.saturating_sub(1))
        .ok_or_else(|| PdfError::ObjectParseError("page out of range".into()))?;
    // The page as shown, not the sheet it was imposed on.
    let view = page_box(&doc, pd);
    let (width, height) = (view[2] - view[0], view[3] - view[1]);
    let ops = build_display_ops(&doc, pd, view);
    Ok(DisplayList {
        page_number,
        width,
        height,
        ops,
    })
}

/// Flatten a cubic Bézier (already in device space) into line points.
fn flatten_cubic(p0: [f64; 2], p1: [f64; 2], p2: [f64; 2], p3: [f64; 2], out: &mut Vec<[f64; 2]>) {
    const N: usize = 12;
    for i in 1..=N {
        let t = i as f64 / N as f64;
        let mt = 1.0 - t;
        let a = mt * mt * mt;
        let b = 3.0 * mt * mt * t;
        let c = 3.0 * mt * t * t;
        let d = t * t * t;
        out.push([
            a * p0[0] + b * p1[0] + c * p2[0] + d * p3[0],
            a * p0[1] + b * p1[1] + c * p2[1] + d * p3[1],
        ]);
    }
}

/// Read the last `n` numeric operands as an RGBA color (gray/rgb/cmyk by count).
fn color_from_stack(stack: &[Token]) -> [f64; 4] {
    let nums: Vec<f64> = stack.iter().rev().map_while(|t| t.as_num()).collect();
    match nums.len() {
        1 => {
            let g = nums[0];
            [g, g, g, 1.0]
        }
        3 => [nums[2], nums[1], nums[0], 1.0],
        4 => {
            // nums is reversed: [k, y, m, c]
            let (c, m, y, k) = (nums[3], nums[2], nums[1], nums[0]);
            [
                (1.0 - c) * (1.0 - k),
                (1.0 - m) * (1.0 - k),
                (1.0 - y) * (1.0 - k),
                1.0,
            ]
        }
        _ => [0.0, 0.0, 0.0, 1.0],
    }
}

/// How to read the operands of `sc`/`scn` for the space currently selected.
///
/// Arity alone is not enough. A single number is a grey level in DeviceGray
/// but a *tint* in a Separation or DeviceN space, and a tint runs the other
/// way: 1 means the colorant at full strength, which is usually black ink.
/// Read as grey, `1 scn` paints white, and a page of white text on white
/// paper looks exactly like a page that failed to render.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
enum ColorKind {
    Gray,
    Rgb,
    Cmyk,
    /// Separation or DeviceN: the operands are colorant tints.
    Tint,
    /// Indexed: the operand is a row of the space's own lookup table, not a
    /// colour at all. Read as a grey level, `1 scn` in such a space paints
    /// white, and a page's whole background panel disappears.
    Indexed,
    /// The `scn` operand is the *name* of a pattern, not a colour at all.
    /// Read as a colour it has no numbers to read, which falls through to
    /// black -- so a photograph placed as a tiling pattern paints as a solid
    /// black rectangle, which is the worst available way to be wrong.
    Pattern,
    /// Something unrecognised; fall back to operand count.
    #[default]
    Unknown,
}

impl ColorKind {
    /// Classify a colour space object, resolving `ICCBased` by its component
    /// count and named spaces by name.
    fn of(doc: &Document, cs: &Object) -> ColorKind {
        match cs {
            Object::Name(n) => match n.as_str() {
                "DeviceGray" | "CalGray" | "G" => ColorKind::Gray,
                "DeviceCMYK" | "CMYK" => ColorKind::Cmyk,
                "DeviceRGB" | "CalRGB" | "RGB" => ColorKind::Rgb,
                "Pattern" => ColorKind::Pattern,
                _ => ColorKind::Unknown,
            },
            Object::Array(a) => match a.first().and_then(|o| o.as_name()) {
                Some("Separation") | Some("DeviceN") => ColorKind::Tint,
                Some("Indexed") => ColorKind::Indexed,
                Some("Pattern") => ColorKind::Pattern,
                Some("CalGray") => ColorKind::Gray,
                Some("CalRGB") | Some("Lab") => ColorKind::Rgb,
                Some("ICCBased") => match color_space_components(doc, cs) {
                    1 => ColorKind::Gray,
                    4 => ColorKind::Cmyk,
                    _ => ColorKind::Rgb,
                },
                _ => ColorKind::Unknown,
            },
            _ => ColorKind::Unknown,
        }
    }
}

/// The colour an `sc`/`scn` names, given the space in force.
///
/// A tint is treated as ink coverage on white: full tint is black. Evaluating
/// the space's tint transform into its alternate would be exact, but coverage
/// is the right sense of the number, and the sense is what matters -- the
/// alternative is text painted in the inverse of its own colour.
fn color_in_space(stack: &[Token], space: &Space) -> [f64; 4] {
    let nums: Vec<f64> = stack.iter().rev().map_while(|t| t.as_num()).collect();
    if space.kind == ColorKind::Indexed
        && let Some(palette) = &space.palette
        && let Some(index) = nums.last()
    {
        // The operand names a row of the space's own table.
        return palette.entry(*index as usize);
    }
    let kind = space.kind;
    if kind == ColorKind::Tint && !nums.is_empty() {
        // DeviceN carries one tint per colorant; the heaviest governs.
        let ink = nums.iter().cloned().fold(0.0f64, f64::max).clamp(0.0, 1.0);
        let level = 1.0 - ink;
        return [level, level, level, 1.0];
    }
    color_from_stack(stack)
}

/// The colour spaces a resource dictionary names.
fn build_color_spaces_in(doc: &Document, rd: &Dict) -> HashMap<String, Space> {
    let mut out = HashMap::new();
    let Some(spaces) = doc.get(rd, "ColorSpace").map(|o| doc.resolve(&o)) else {
        return out;
    };
    let Some(sd) = spaces.as_dict().cloned() else {
        return out;
    };
    for (name, obj) in sd.iter() {
        let resolved = doc.resolve(obj);
        out.insert(
            name.clone(),
            Space {
                kind: ColorKind::of(doc, &resolved),
                palette: palette_from_space(doc, &resolved),
            },
        );
    }
    out
}

/// A named colour space: how to read its operands, and its lookup table where
/// it has one.
#[derive(Clone, Default)]
struct Space {
    kind: ColorKind,
    palette: Option<Palette>,
}

/// Resolve the name `cs` was given against the page's spaces, treating the
/// device names as themselves.
fn named_space(stack: &[Token], spaces: &HashMap<String, Space>) -> Space {
    let Some(Token::Name(name)) = stack.last() else {
        return Space::default();
    };
    let kind = match name.as_str() {
        "DeviceGray" | "CalGray" | "G" => ColorKind::Gray,
        "DeviceRGB" | "CalRGB" | "RGB" => ColorKind::Rgb,
        "DeviceCMYK" | "CMYK" => ColorKind::Cmyk,
        // `/Pattern cs` names the space directly rather than through the
        // resource dictionary, which is where every other space is looked up.
        // Falling through to that lookup finds nothing and lands on the
        // default, and a pattern read as a default space paints black.
        "Pattern" => ColorKind::Pattern,
        other => return spaces.get(other).cloned().unwrap_or_default(),
    };
    Space {
        kind,
        palette: None,
    }
}

/// What `q` saves and `Q` puts back.
///
/// Colour rides along with the transform: a form or an annotation that sets
/// its own colour inside `q`/`Q` must not leave it set for the rest of the
/// page.
/// The blend modes we distinguish.
///
/// Only the two with a colour that acts as an identity matter for correctness
/// on a page rendered without a real compositor; everything else is treated as
/// normal painting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Blend {
    Normal,
    Multiply,
    Screen,
    Other,
}

impl Blend {
    /// The value is a name, or an array of names in order of preference.
    fn from_object(doc: &Document, obj: &Object) -> Blend {
        let resolved = doc.resolve(obj);
        let name = match resolved.as_name() {
            Some(n) => n.to_string(),
            None => match resolved.as_array().and_then(|a| a.first()) {
                Some(first) => match doc.resolve(first).as_name() {
                    Some(n) => n.to_string(),
                    None => return Blend::Other,
                },
                None => return Blend::Other,
            },
        };
        match name.as_str() {
            "Normal" | "Compatible" => Blend::Normal,
            "Multiply" => Blend::Multiply,
            "Screen" => Blend::Screen,
            _ => Blend::Other,
        }
    }
}

/// The `/ExtGState` entry a `gs` operator names, if it resolves to a dictionary.
fn ext_gstate(doc: &Document, resources: &Dict, name: &str) -> Option<Dict> {
    let states = doc
        .get(resources, "ExtGState")
        .and_then(|o| o.as_dict().cloned())?;
    let resolved = doc.resolve(states.get(name)?);
    resolved.as_dict().cloned()
}

/// Fold a constant alpha into a colour's own alpha channel.
fn with_alpha(mut color: [f64; 4], alpha: f64) -> [f64; 4] {
    color[3] *= alpha.clamp(0.0, 1.0);
    color
}

/// True when painting `color` under `blend` cannot change the page.
///
/// White is the identity of multiply and black is the identity of screen: a
/// producer that paints one is saying "leave the backdrop alone", and often
/// does so over a large area. Painting it normally instead erases whatever is
/// underneath -- which is exactly how a tinted panel behind a stack of white
/// boxes vanishes. Fully transparent paint is the same case.
fn blend_is_noop(color: [f64; 4], blend: Blend) -> bool {
    if color[3] <= 0.0 {
        return true;
    }
    match blend {
        Blend::Multiply => color[0] >= 0.999 && color[1] >= 0.999 && color[2] >= 0.999,
        Blend::Screen => color[0] <= 0.001 && color[1] <= 0.001 && color[2] <= 0.001,
        _ => false,
    }
}

#[derive(Clone)]
struct GState<'a> {
    ctm: Matrix,
    text: TextState,
    fill: [f64; 4],
    stroke: [f64; 4],
    fill_space: Space,
    stroke_space: Space,
    /// The pattern `scn` named, when the fill space is a pattern space. It is
    /// graphics state like any other colour, so `q`/`Q` must restore it.
    fill_pattern: Option<String>,
    /// The font is text state, and text state is graphics state: `Tf` inside
    /// `q`/`Q` lasts only until the `Q`.
    font: Option<&'a Font>,
    font_name: String,
    font_size: f64,
    leading: f64,
    /// Constant alpha and blend mode, set by `gs` from an `/ExtGState`.
    fill_alpha: f64,
    stroke_alpha: f64,
    blend: Blend,
    /// In user space, as `w` gives it. The CTM that scales it is the one in
    /// effect when the path is painted, not when the width was set.
    line_width: f64,
    /// The device rectangle painting is confined to. `sh` fills exactly this,
    /// so it has to be state rather than only an emitted op.
    clip: [f64; 4],
    /// The luminosity mask a `gs` set, evaluated into regions.
    soft_mask: Option<SoftMask>,
}

fn build_display_ops(doc: &Document, page: &Dict, view: [f64; 4]) -> Vec<RenderOp> {
    let content = page_contents(doc, page);
    if content.is_empty() {
        return Vec::new();
    }
    let resources = doc
        .get(page, "Resources")
        .and_then(|o| o.as_dict().cloned())
        .unwrap_or_default();
    let mut ops = Vec::new();
    // Content is placed in the media box's coordinates; the display list is in
    // the visible box's. Where the two differ the whole page shifts.
    run_content(
        doc,
        &content,
        &resources,
        [1.0, 0.0, 0.0, 1.0, -view[0], -view[1]],
        &mut ops,
        0,
        &FormState {
            // `sh` paints the current clip, and on a page with none of its own
            // that is the page. Without this the gradient has no bounds to fill.
            clip: [0.0, 0.0, view[2] - view[0], view[3] - view[1]],
            ..FormState::default()
        },
    );
    ops
}

/// A luminosity mask, evaluated: device-space regions and the coverage each
/// gives. Shared rather than copied, because `q` saves it on every push.
type SoftMask = std::rc::Rc<Vec<(Vec<[f64; 2]>, f64)>>;

/// How many pieces a masked group may emit before the mask is abandoned.
///
/// A mask multiplies every fill under it by the number of regions it has, so a
/// group holding a page of artwork could otherwise turn a few hundred ops into
/// tens of thousands.
const MAX_MASKED_PIECES: usize = 8192;

/// Evaluate a luminosity mask into device-space regions and their coverage.
///
/// The mask's group is a form, so it is run the way any form is run, and
/// whatever it filled becomes the mask: brightness is coverage. Only fills are
/// read. A mask drawn with text or an image cannot be turned into regions this
/// way, and returning nothing for it means the content paints unmasked -- which
/// is what happened before any of this existed.
fn build_soft_mask(doc: &Document, mask: &Dict, ctm: Matrix, depth: usize) -> Option<SoftMask> {
    if depth >= MAX_FORM_DEPTH {
        return None;
    }
    // An alpha mask takes coverage from the group's alpha rather than its
    // colour, and nothing here records alpha per region.
    if doc
        .get(mask, "S")
        .and_then(|o| o.as_name().map(String::from))
        != Some("Luminosity".into())
    {
        return None;
    }
    let Object::Stream(dict, raw) = doc.get(mask, "G").map(|o| doc.resolve(&o))? else {
        return None;
    };
    let content = decode_stream(&dict, &raw).ok()?;
    let resources = doc
        .get(&dict, "Resources")
        .and_then(|o| doc.resolve(&o).as_dict().cloned())
        .unwrap_or_default();
    let mut ops = Vec::new();
    run_form(
        doc,
        &(dict, content),
        &resources,
        ctm,
        &mut ops,
        depth + 1,
        &FormState::default(),
    );
    let regions: Vec<(Vec<[f64; 2]>, f64)> = ops
        .into_iter()
        .filter_map(|op| match op {
            RenderOp::Fill {
                subpaths, color, ..
            } => subpaths
                .into_iter()
                .find(|p| p.len() >= 3)
                .map(|p| (p, crate::shading::luminosity(color))),
            _ => None,
        })
        .collect();
    if regions.is_empty() {
        return None;
    }
    Some(std::rc::Rc::new(regions))
}

/// Split what a transparency group painted across a mask's regions.
///
/// The mask belongs to the group as a whole, not to the operations inside it:
/// a `gs` within the group sets the state for the group's own painting and does
/// not undo the mask the group is composited through. So this runs over the
/// finished ops rather than being threaded into the interpreter -- which is the
/// difference between working and doing nothing at all.
///
/// Only fills are split. Text and images inside a masked group still paint at
/// full strength.
fn apply_mask_to_group(ops: Vec<RenderOp>, mask: &[(Vec<[f64; 2]>, f64)]) -> Vec<RenderOp> {
    let fills = ops
        .iter()
        .filter(|op| matches!(op, RenderOp::Fill { .. }))
        .count();
    if fills.saturating_mul(mask.len()) > MAX_MASKED_PIECES {
        return ops;
    }
    let mut out = Vec::with_capacity(ops.len());
    for op in ops {
        let RenderOp::Fill {
            subpaths,
            color,
            even_odd,
        } = op
        else {
            out.push(op);
            continue;
        };
        for (region, coverage) in mask {
            // Under a hundredth of coverage nothing survives rounding to a
            // byte of alpha, and the piece costs an op either way.
            if *coverage <= 0.004 {
                continue;
            }
            let pieces: Vec<Vec<[f64; 2]>> = subpaths
                .iter()
                .map(|sp| crate::shading::clip_to_convex(sp, region))
                .filter(|p| p.len() >= 3)
                .collect();
            if pieces.is_empty() {
                continue;
            }
            let mut tinted = color;
            tinted[3] *= coverage;
            out.push(RenderOp::Fill {
                subpaths: pieces,
                color: tinted,
                even_odd,
            });
        }
    }
    out
}

/// The shading a name refers to in the resource dictionary.
fn shading_in(doc: &Document, resources: &Dict, name: &str) -> Option<(Dict, Vec<u8>)> {
    let shadings = doc.get(resources, "Shading").map(|o| doc.resolve(&o))?;
    let entry = doc
        .get(shadings.as_dict()?, name)
        .map(|o| doc.resolve(&o))?;
    match entry {
        // A shading is a stream when it carries mesh data, and the mesh *is*
        // that data: a patch's geometry and its corner colours are nowhere in
        // the dictionary. Keeping only the dictionary, as this did, leaves a
        // mesh with nothing to be drawn from.
        Object::Stream(d, raw) => Some((d, raw)),
        other => other.as_dict().cloned().map(|d| (d, Vec::new())),
    }
}

/// The form XObject a name refers to, if it is one.
///
/// An image XObject is drawn; a form is executed. Anything else -- a
/// PostScript XObject, a broken reference -- is neither.
fn form_xobject(doc: &Document, resources: &Dict, name: &str) -> Option<(Dict, Vec<u8>)> {
    let xobjects = doc
        .get(resources, "XObject")
        .and_then(|o| o.as_dict().cloned())?;
    let resolved = doc.resolve(xobjects.get(name)?);
    let Object::Stream(d, raw) = &resolved else {
        return None;
    };
    if doc
        .get(d, "Subtype")
        .and_then(|o| o.as_name().map(String::from))
        .as_deref()
        != Some("Form")
    {
        return None;
    }
    let content = decode_stream(d, raw).ok()?;
    Some((d.clone(), content))
}

/// Paint a fill whose colour is a pattern, clipped to the path it drew.
///
/// A pattern is content, not a colour, and `scn` names it rather than giving
/// numbers. Read as a colour there are no numbers to read, so it lands on
/// black -- which is how four photographs on one cover came out as solid black
/// rectangles inside their white frames.
///
/// A tiling pattern is structurally a form: `/BBox`, `/Matrix`, `/Resources`
/// and a content stream, so it is run as one. Its `/Matrix` is anchored to the
/// space the *enclosing* stream started in, not to the CTM at the fill -- a
/// pattern does not move when the thing painted with it is transformed, which
/// is the whole point of one.
///
/// A shading pattern has no stream: it is a gradient poured into the shape,
/// and is banded the way `sh` bands one. Anything else -- a broken reference,
/// a type nobody has written -- paints nothing, which leaves a hole, and a
/// hole is the safe direction where black is not.
struct PatternFill<'a> {
    /// The path the fill drew, in device space; the pattern is clipped to it.
    paths: &'a [Vec<[f64; 2]>],
    /// The transform the *enclosing* stream started in. A pattern's `/Matrix`
    /// is anchored to that, not to the CTM at the fill: a pattern does not
    /// move when the shape painted with it is transformed.
    base: Matrix,
}

fn paint_pattern(
    doc: &Document,
    resources: &Dict,
    name: &str,
    fill: &PatternFill<'_>,
    ops: &mut Vec<RenderOp>,
    depth: usize,
    state: &FormState,
) {
    let (paths, base) = (fill.paths, fill.base);
    if depth >= MAX_FORM_DEPTH {
        return;
    }
    let Some(pattern) = doc
        .get(resources, "Pattern")
        .and_then(|o| o.as_dict().cloned())
        .and_then(|d| d.get(name).map(|o| doc.resolve(o)))
    else {
        return;
    };
    // A tiling pattern is a stream -- it has content to run. A shading pattern
    // is a plain dictionary: the gradient is its whole definition. Requiring a
    // stream rejects every shading pattern before its type is even read, and
    // an unresolved pattern paints nothing, so a page's masthead simply
    // vanishes.
    let (dict, raw): (&Dict, &[u8]) = match &pattern {
        Object::Stream(d, raw) => (d, raw),
        Object::Dict(d) => (d, &[]),
        _ => return,
    };
    let kind = doc.get(dict, "PatternType").and_then(|o| o.as_int());

    // The device-space box the fill covers: nothing outside it can show.
    let points = paths.iter().flatten();
    let (mut minx, mut miny) = (f64::INFINITY, f64::INFINITY);
    let (mut maxx, mut maxy) = (f64::NEG_INFINITY, f64::NEG_INFINITY);
    for p in points {
        minx = minx.min(p[0]);
        miny = miny.min(p[1]);
        maxx = maxx.max(p[0]);
        maxy = maxy.max(p[1]);
    }
    if !(maxx > minx && maxy > miny) {
        return;
    }

    let matrix = rect6(doc, dict, "Matrix").unwrap_or([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
    let anchored = mat_mul(&matrix, &base);

    // A shading pattern has no stream to run: it is a gradient poured into the
    // shape, which is the same thing `sh` does to the clip. Sliced into flat
    // bands here for the same reason -- three renderers would otherwise have
    // to learn to draw a gradient.
    if kind == Some(2) {
        let Some((shading, mesh)) =
            doc.get(dict, "Shading")
                .map(|o| doc.resolve(&o))
                .and_then(|o| match o {
                    Object::Stream(d, raw) => Some((d, raw)),
                    other => other.as_dict().cloned().map(|d| (d, Vec::new())),
                })
        else {
            return;
        };
        ops.push(RenderOp::Save);
        ops.push(RenderOp::Clip {
            rect: [minx, miny, maxx, maxy],
            subpaths: paths.iter().filter(|s| s.len() >= 3).cloned().collect(),
        });
        for band in
            crate::shading::bands_of(doc, &shading, &mesh, anchored, [minx, miny, maxx, maxy])
        {
            ops.push(RenderOp::Fill {
                subpaths: vec![band.points],
                color: with_alpha(band.color, state.fill_alpha),
                even_odd: false,
            });
        }
        ops.push(RenderOp::Restore);
        return;
    }

    // Anything that is not a tiling pattern from here on has no content to
    // run, so it paints nothing rather than being approximated.
    if kind != Some(1) {
        return;
    }
    let Ok(content) = decode_stream(dict, raw) else {
        return;
    };

    ops.push(RenderOp::Save);
    ops.push(RenderOp::Clip {
        rect: [minx, miny, maxx, maxy],
        subpaths: paths.iter().filter(|s| s.len() >= 3).cloned().collect(),
    });

    let bbox = rect4(doc, dict, "BBox").unwrap_or([0.0, 0.0, 1.0, 1.0]);
    let step_x = doc
        .get(dict, "XStep")
        .and_then(|o| o.as_f64())
        .filter(|v| v.abs() > f64::EPSILON)
        .unwrap_or(bbox[2] - bbox[0]);
    let step_y = doc
        .get(dict, "YStep")
        .and_then(|o| o.as_f64())
        .filter(|v| v.abs() > f64::EPSILON)
        .unwrap_or(bbox[3] - bbox[1]);

    // How far the tile has to repeat to cover the fill, measured in pattern
    // space. A tile the size of its own step covers the area in one go, which
    // is what a photograph placed as a pattern does; a small tile repeats.
    let scale = (anchored[0] * anchored[3] - anchored[1] * anchored[2])
        .abs()
        .sqrt()
        .max(f64::EPSILON);
    let span_x = (maxx - minx) / scale;
    let span_y = (maxy - miny) / scale;
    // A cap rather than a count: a tile a fraction of a point wide against a
    // page-sized fill is millions of repetitions, and a document that asks for
    // that is asking by accident.
    const MAX_TILES: usize = 4096;
    let across = ((span_x / step_x.abs()).ceil() as usize + 2).min(MAX_TILES);
    let down = ((span_y / step_y.abs()).ceil() as usize + 2).min(MAX_TILES);
    let tiles = across.saturating_mul(down).min(MAX_TILES);

    let inner_resources = doc
        .get(dict, "Resources")
        .and_then(|o| o.as_dict().cloned())
        .unwrap_or_else(|| resources.clone());

    // Where the fill's own corner lands in pattern space, so the tiling starts
    // at the first tile that can reach it rather than always at the origin.
    let Some(back) = mat_invert(&anchored) else {
        ops.push(RenderOp::Restore);
        return;
    };
    let corner = transform_point(&back, minx, miny);
    let first_i = ((corner.0 - bbox[0]) / step_x).floor();
    let first_j = ((corner.1 - bbox[1]) / step_y).floor();

    let mut drawn = 0usize;
    'tiling: for j in 0..down {
        for i in 0..across {
            if drawn >= tiles {
                break 'tiling;
            }
            drawn += 1;
            let offset = [
                1.0,
                0.0,
                0.0,
                1.0,
                (first_i + i as f64) * step_x,
                (first_j + j as f64) * step_y,
            ];
            run_content(
                doc,
                &content,
                &inner_resources,
                mat_mul(&offset, &anchored),
                ops,
                depth + 1,
                state,
            );
        }
    }
    ops.push(RenderOp::Restore);
}

/// Run a form XObject: its own transform, clipped to its own box, with its own
/// resources.
///
/// The form's `/Matrix` maps form space into the space of whoever drew it, and
/// its `/BBox` bounds what it may paint -- a form is entitled to draw outside
/// its box and expect to be cut off there.
fn run_form(
    doc: &Document,
    form: &(Dict, Vec<u8>),
    outer: &Dict,
    ctm: Matrix,
    ops: &mut Vec<RenderOp>,
    depth: usize,
    state: &FormState,
) {
    let (dict, content) = form;
    let matrix = rect6(doc, dict, "Matrix").unwrap_or([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
    let inner = mat_mul(&matrix, &ctm);

    ops.push(RenderOp::Save);
    let mut state = state.clone();
    if let Some(bbox) = rect4(doc, dict, "BBox") {
        // The box is in form space, so its corners go through the form's own
        // transform; a rotated form has a rotated box and needs the polygon,
        // not just its extent.
        let corners = [
            transform_point(&inner, bbox[0], bbox[1]),
            transform_point(&inner, bbox[2], bbox[1]),
            transform_point(&inner, bbox[2], bbox[3]),
            transform_point(&inner, bbox[0], bbox[3]),
        ];
        let xs: Vec<f64> = corners.iter().map(|c| c.0).collect();
        let ys: Vec<f64> = corners.iter().map(|c| c.1).collect();
        let rect = [
            xs.iter().cloned().fold(f64::INFINITY, f64::min),
            ys.iter().cloned().fold(f64::INFINITY, f64::min),
            xs.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
            ys.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
        ];
        // The box bounds what the form may paint, so it narrows the tracked
        // clip as well as emitting one. Without this a gradient inside a form
        // -- which is where a document puts one -- floods the whole page
        // instead of the box it was drawn in.
        state.clip = [
            state.clip[0].max(rect[0]),
            state.clip[1].max(rect[1]),
            state.clip[2].min(rect[2]),
            state.clip[3].min(rect[3]),
        ];
        ops.push(RenderOp::Clip {
            rect,
            subpaths: vec![corners.iter().map(|c| [c.0, c.1]).collect()],
        });
    }
    // A form without resources of its own inherits the ones around it, which
    // the specification allows and producers rely on.
    let inner_resources = doc
        .get(dict, "Resources")
        .and_then(|o| o.as_dict().cloned())
        .unwrap_or_else(|| outer.clone());
    run_content(
        doc,
        content,
        &inner_resources,
        inner,
        ops,
        depth + 1,
        &state,
    );
    ops.push(RenderOp::Restore);
}

/// The graphics state a form XObject starts from.
///
/// A form is not a fresh page: the specification hands it the state in force
/// where it is drawn, and producers rely on that. One white paper sets `ca 0`
/// and then draws a form; starting the form from the defaults instead paints
/// two opaque figures over the page that no other reader shows.
#[derive(Clone)]
struct FormState {
    fill: [f64; 4],
    stroke: [f64; 4],
    fill_space: Space,
    stroke_space: Space,
    fill_alpha: f64,
    stroke_alpha: f64,
    blend: Blend,
    line_width: f64,
    /// The device rectangle painting is confined to. A form inherits it, and
    /// `sh` fills exactly it.
    clip: [f64; 4],
}

impl Default for FormState {
    fn default() -> FormState {
        FormState {
            fill: [0.0, 0.0, 0.0, 1.0],
            stroke: [0.0, 0.0, 0.0, 1.0],
            fill_space: Space::default(),
            stroke_space: Space::default(),
            fill_alpha: 1.0,
            stroke_alpha: 1.0,
            blend: Blend::Normal,
            line_width: 1.0,
            clip: [
                f64::NEG_INFINITY,
                f64::NEG_INFINITY,
                f64::INFINITY,
                f64::INFINITY,
            ],
        }
    }
}

/// How deep a form may nest before we stop following it.
///
/// Forms refer to forms legitimately -- a logo inside a letterhead inside a
/// page -- but a malformed file can make one refer to itself, and the reader
/// must come back either way.
const MAX_FORM_DEPTH: usize = 12;

/// Interpret one content stream, appending to `ops`.
///
/// A page and a form XObject are the same thing here: a stream of operators
/// with a resource dictionary and a starting transform. Keeping that in one
/// function is what lets a form be drawn by running it, rather than by
/// pretending it is an image and drawing a box.
fn run_content(
    doc: &Document,
    content: &[u8],
    resources: &Dict,
    start_ctm: Matrix,
    ops: &mut Vec<RenderOp>,
    depth: usize,
    inherited: &FormState,
) {
    let fonts = build_fonts_in(doc, resources);
    let color_spaces = build_color_spaces_in(doc, resources);

    // The text rendering mode rides along with the transform: it is graphics
    // state, so `q`/`Q` must restore it. Otherwise an invisible run inside a
    // saved state leaks out and blanks the real text that follows.
    let mut ctm_stack: Vec<GState<'_>> = Vec::new();
    let mut ctm: Matrix = start_ctm;
    let mut tm: Matrix = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
    let mut tlm: Matrix = tm;
    let mut font_size = 0.0f64;
    let mut leading = 0.0f64;
    let mut cur_font: Option<&Font> = None;
    let mut cur_font_name = String::new();
    // Text rendering mode. 3 is invisible and 7 clips without painting, which
    // is how an OCR layer sits over a scanned page: the words are there to be
    // searched and selected, not to be seen. Drawing them puts the
    // transcription on top of the picture of the same words.
    let mut ts = TextState::default();
    // Colour, alpha, blend and line width come from the caller: for a page
    // those are the defaults, for a form they are the state it is drawn under.
    let mut fill: [f64; 4] = inherited.fill;
    let mut stroke: [f64; 4] = inherited.stroke;
    // Which space those numbers are in, so `1 scn` can mean full ink rather
    // than white.
    let mut fill_space = inherited.fill_space.clone();
    // Not inherited: a pattern names a resource, and a form's resources are
    // its own, so a name from outside would mean a different thing inside.
    let mut fill_pattern: Option<String> = None;
    let mut stroke_space = inherited.stroke_space.clone();
    let mut line_width = inherited.line_width;
    let mut clip = inherited.clip;
    // Set by `gs`, and applied where a transparency group is composited.
    let mut soft_mask: Option<SoftMask> = None;
    // Constant alpha and blend mode, both graphics state, both set by `gs`.
    let mut fill_alpha = inherited.fill_alpha;
    let mut stroke_alpha = inherited.stroke_alpha;
    let mut blend = inherited.blend;

    // Path state, in device space.
    let mut subpaths: Vec<Vec<[f64; 2]>> = Vec::new();
    let mut cur: Vec<[f64; 2]> = Vec::new();
    let mut start = [0.0f64, 0.0];
    let mut pt = [0.0f64, 0.0];

    let mut stack: Vec<Token> = Vec::new();
    let mut lex = ContentLexer::new(content);

    let close_cur = |subpaths: &mut Vec<Vec<[f64; 2]>>, cur: &mut Vec<[f64; 2]>| {
        if cur.len() > 1 {
            subpaths.push(std::mem::take(cur));
        } else {
            cur.clear();
        }
    };

    while let Some(tok) = lex.next_token() {
        match tok {
            Token::Op(op) => {
                match op.as_str() {
                    "q" => {
                        ctm_stack.push(GState {
                            ctm,
                            text: ts,
                            fill,
                            stroke,
                            fill_space: fill_space.clone(),
                            fill_pattern: fill_pattern.clone(),
                            stroke_space: stroke_space.clone(),
                            font: cur_font,
                            font_name: cur_font_name.clone(),
                            font_size,
                            leading,
                            fill_alpha,
                            stroke_alpha,
                            blend,
                            line_width,
                            clip,
                            soft_mask: soft_mask.clone(),
                        });
                        ops.push(RenderOp::Save);
                    }
                    "Q" => {
                        if let Some(state) = ctm_stack.pop() {
                            ctm = state.ctm;
                            ts = state.text;
                            fill = state.fill;
                            stroke = state.stroke;
                            fill_space = state.fill_space;
                            fill_pattern = state.fill_pattern;
                            stroke_space = state.stroke_space;
                            // A `Tf` inside the saved state does not outlive
                            // it. Documents rely on this: one sets a symbol
                            // font for a separator, restores, and shows the
                            // next words with no `Tf` at all -- decoded with
                            // the symbol font, those words are gibberish or
                            // nothing.
                            cur_font = state.font;
                            cur_font_name = state.font_name;
                            font_size = state.font_size;
                            leading = state.leading;
                            fill_alpha = state.fill_alpha;
                            stroke_alpha = state.stroke_alpha;
                            blend = state.blend;
                            line_width = state.line_width;
                            clip = state.clip;
                            soft_mask = state.soft_mask;
                        }
                        ops.push(RenderOp::Restore);
                    }
                    "W" | "W*" => {
                        let mut minx = f64::INFINITY;
                        let mut miny = f64::INFINITY;
                        let mut maxx = f64::NEG_INFINITY;
                        let mut maxy = f64::NEG_INFINITY;
                        for sp in subpaths.iter().chain(std::iter::once(&cur)) {
                            for p in sp {
                                minx = minx.min(p[0]);
                                miny = miny.min(p[1]);
                                maxx = maxx.max(p[0]);
                                maxy = maxy.max(p[1]);
                            }
                        }
                        if minx.is_finite() && maxx > minx && maxy > miny {
                            let mut clip_subpaths: Vec<Vec<[f64; 2]>> =
                                subpaths.iter().filter(|s| s.len() >= 3).cloned().collect();
                            if cur.len() >= 3 {
                                clip_subpaths.push(cur.clone());
                            }
                            clip = [
                                clip[0].max(minx),
                                clip[1].max(miny),
                                clip[2].min(maxx),
                                clip[3].min(maxy),
                            ];
                            ops.push(RenderOp::Clip {
                                rect: [minx, miny, maxx, maxy],
                                subpaths: clip_subpaths,
                            });
                        }
                    }
                    // A shading painted straight onto the page, filling the
                    // current clip. It has no path and no fill colour of its
                    // own: the gradient *is* the paint.
                    "sh" => {
                        if let Some(Token::Name(name)) = stack.last()
                            && let Some((sh, raw)) = shading_in(doc, resources, name)
                        {
                            let region = [clip[0].max(0.0), clip[1].max(0.0), clip[2], clip[3]];
                            if region[2] > region[0] && region[3] > region[1] {
                                for band in crate::shading::bands_of(doc, &sh, &raw, ctm, region) {
                                    ops.push(RenderOp::Fill {
                                        subpaths: vec![band.points],
                                        color: with_alpha(band.color, fill_alpha),
                                        even_odd: false,
                                    });
                                }
                            }
                        }
                    }
                    "cm" => {
                        if let Some(m) = last6(&stack) {
                            ctm = mat_mul(&m, &ctm);
                        }
                    }
                    "w" => {
                        if let Some(Token::Num(v)) = stack.last() {
                            line_width = *v;
                        }
                    }
                    "g" | "rg" | "k" => {
                        fill_space = Space::default();
                        fill_pattern = None;
                        fill = color_from_stack(&stack);
                    }
                    "G" | "RG" | "K" => {
                        stroke_space = Space::default();
                        stroke = color_from_stack(&stack);
                    }
                    // Selecting a space also resets the colour to that space's
                    // initial value, which is black for every space here.
                    "cs" => {
                        fill_space = named_space(&stack, &color_spaces);
                        fill_pattern = None;
                        fill = [0.0, 0.0, 0.0, 1.0];
                    }
                    "CS" => {
                        stroke_space = named_space(&stack, &color_spaces);
                        stroke = [0.0, 0.0, 0.0, 1.0];
                    }
                    "sc" | "scn" => match fill_space.kind {
                        // In a pattern space the operand is a name. Reading it
                        // as a colour finds no numbers and lands on black.
                        ColorKind::Pattern => {
                            fill_pattern = match stack.last() {
                                Some(Token::Name(n)) => Some(n.clone()),
                                _ => None,
                            }
                        }
                        _ => {
                            fill_pattern = None;
                            fill = color_in_space(&stack, &fill_space);
                        }
                    },
                    "SC" | "SCN" => stroke = color_in_space(&stack, &stroke_space),
                    "m" => {
                        if let Some([x, y]) = last2(&stack) {
                            close_cur(&mut subpaths, &mut cur);
                            let p = transform_point(&ctm, x, y);
                            pt = [p.0, p.1];
                            start = pt;
                            cur.push(pt);
                        }
                    }
                    "l" => {
                        if let Some([x, y]) = last2(&stack) {
                            let p = transform_point(&ctm, x, y);
                            pt = [p.0, p.1];
                            cur.push(pt);
                        }
                    }
                    "c" if stack.len() >= 6 => {
                        // Full cubic: two control points + endpoint.
                        let n = stack.len();
                        let g = |k: usize| {
                            let p = transform_point(
                                &ctm,
                                stack[n - k].as_num().unwrap_or(0.0),
                                stack[n - k + 1].as_num().unwrap_or(0.0),
                            );
                            [p.0, p.1]
                        };
                        let (p1, p2, p3) = (g(6), g(4), g(2));
                        flatten_cubic(pt, p1, p2, p3, &mut cur);
                        pt = p3;
                    }
                    "v" | "y" => {
                        // One control point implicit; approximate as a curve to end.
                        if let Some([x, y]) = last2(&stack) {
                            let p = transform_point(&ctm, x, y);
                            flatten_cubic(pt, pt, [p.0, p.1], [p.0, p.1], &mut cur);
                            pt = [p.0, p.1];
                        }
                    }
                    "re" => {
                        if let Some([x, y, rw, rh]) = last4(&stack) {
                            close_cur(&mut subpaths, &mut cur);
                            let c0 = transform_point(&ctm, x, y);
                            let c1 = transform_point(&ctm, x + rw, y);
                            let c2 = transform_point(&ctm, x + rw, y + rh);
                            let c3 = transform_point(&ctm, x, y + rh);
                            subpaths.push(vec![
                                [c0.0, c0.1],
                                [c1.0, c1.1],
                                [c2.0, c2.1],
                                [c3.0, c3.1],
                                [c0.0, c0.1],
                            ]);
                            pt = [c0.0, c0.1];
                            start = pt;
                        }
                    }
                    "h" if !cur.is_empty() => {
                        cur.push(start);
                    }
                    "f" | "F" | "f*" | "b" | "b*" | "B" | "B*" | "S" | "s" => {
                        if matches!(op.as_str(), "s" | "b" | "b*") && !cur.is_empty() {
                            cur.push(start);
                        }
                        close_cur(&mut subpaths, &mut cur);
                        let paths = std::mem::take(&mut subpaths);
                        let fills =
                            matches!(op.as_str(), "f" | "F" | "f*" | "B" | "B*" | "b" | "b*");
                        let strokes = matches!(op.as_str(), "S" | "s" | "B" | "B*" | "b" | "b*");
                        if !paths.is_empty() {
                            let fill_color = with_alpha(fill, fill_alpha);
                            let stroke_color = with_alpha(stroke, stroke_alpha);
                            if fills && !blend_is_noop(fill_color, blend) {
                                match &fill_pattern {
                                    // A pattern is content, not a colour: run
                                    // it clipped to the path this fill drew.
                                    // Falling through to `fill` here paints
                                    // the black that a nameless `scn` leaves.
                                    Some(name) => paint_pattern(
                                        doc,
                                        resources,
                                        name,
                                        &PatternFill {
                                            paths: &paths,
                                            base: start_ctm,
                                        },
                                        ops,
                                        depth,
                                        &FormState {
                                            fill,
                                            stroke,
                                            fill_space: fill_space.clone(),
                                            stroke_space: stroke_space.clone(),
                                            fill_alpha,
                                            stroke_alpha,
                                            blend,
                                            line_width,
                                            clip,
                                        },
                                    ),
                                    None => ops.push(RenderOp::Fill {
                                        subpaths: paths.clone(),
                                        color: fill_color,
                                        even_odd: op.ends_with('*'),
                                    }),
                                }
                            }
                            if strokes && !blend_is_noop(stroke_color, blend) {
                                // The width is in user space and the CTM that
                                // scales it is this one -- the state when the
                                // path is painted. Reading it at `w` instead
                                // gets a document like
                                //   9525 w  q 0.00003 0 0 0.00003 cm ... S
                                // wrong by four orders of magnitude, which is
                                // a stroke wider than the page.
                                let scale = (ctm[0] * ctm[3] - ctm[1] * ctm[2]).abs().sqrt();
                                ops.push(RenderOp::Stroke {
                                    subpaths: paths,
                                    color: stroke_color,
                                    width: (line_width * scale).max(0.1),
                                });
                            }
                        }
                    }
                    "n" => {
                        cur.clear();
                        subpaths.clear();
                    }
                    "gs" => {
                        if let Some(Token::Name(name)) = stack.last()
                            && let Some(state) = ext_gstate(doc, resources, name)
                        {
                            if let Some(v) = doc.get(&state, "ca").and_then(|o| o.as_f64()) {
                                fill_alpha = v.clamp(0.0, 1.0);
                            }
                            if let Some(v) = doc.get(&state, "CA").and_then(|o| o.as_f64()) {
                                stroke_alpha = v.clamp(0.0, 1.0);
                            }
                            if let Some(v) = doc.get(&state, "LW").and_then(|o| o.as_f64()) {
                                line_width = v;
                            }
                            if let Some(v) = state.get("BM") {
                                blend = Blend::from_object(doc, v);
                            }
                            if let Some(v) = state.get("SMask") {
                                soft_mask = match doc.resolve(v) {
                                    Object::Dict(m) => build_soft_mask(doc, &m, ctm, depth),
                                    // `/None`, or a mask shape nothing here can
                                    // evaluate: paint at full strength.
                                    _ => None,
                                };
                            }
                        }
                    }
                    "Do" => {
                        if let Some(Token::Name(name)) = stack.last() {
                            // A form is a content stream, not a picture: run
                            // it. Drawing a box where one should be executed
                            // leaves a blank page for every document that
                            // wraps its content in one, which is what any
                            // number of imposition and archiving tools emit.
                            if let Some(form) = form_xobject(doc, resources, name)
                                && depth < MAX_FORM_DEPTH
                            {
                                let state = FormState {
                                    fill,
                                    stroke,
                                    fill_space: fill_space.clone(),
                                    stroke_space: stroke_space.clone(),
                                    fill_alpha,
                                    stroke_alpha,
                                    blend,
                                    line_width,
                                    clip,
                                };
                                // A soft mask belongs to the group as a whole.
                                // Run the form aside, then fold the mask into
                                // what it painted -- a `gs` inside the group
                                // must not be able to cancel it.
                                let group = doc
                                    .get(&form.0, "Group")
                                    .map(|o| doc.resolve(&o))
                                    .and_then(|g| g.as_dict().cloned());
                                match (&soft_mask, group) {
                                    (Some(mask), Some(_)) => {
                                        let mut inner = Vec::new();
                                        run_form(
                                            doc, &form, resources, ctm, &mut inner, depth, &state,
                                        );
                                        ops.extend(apply_mask_to_group(inner, mask));
                                    }
                                    _ => run_form(doc, &form, resources, ctm, ops, depth, &state),
                                }
                                stack.clear();
                                continue;
                            }
                            // An image is painted with the fill alpha, and at
                            // zero it cannot mark the page. It is dropped here
                            // rather than handed to a renderer that would only
                            // discover it is invisible -- the same reason a
                            // blend that cannot change the page suppresses a
                            // fill. Any other alpha travels on the op.
                            if fill_alpha <= 0.0 {
                                stack.clear();
                                continue;
                            }
                            let c0 = transform_point(&ctm, 0.0, 0.0);
                            let c1 = transform_point(&ctm, 1.0, 0.0);
                            let c2 = transform_point(&ctm, 1.0, 1.0);
                            let c3 = transform_point(&ctm, 0.0, 1.0);
                            let xs = [c0.0, c1.0, c2.0, c3.0];
                            let ys = [c0.1, c1.1, c2.1, c3.1];
                            let left = xs.iter().cloned().fold(f64::INFINITY, f64::min);
                            let right = xs.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                            let bottom = ys.iter().cloned().fold(f64::INFINITY, f64::min);
                            let top = ys.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                            ops.push(RenderOp::Image {
                                x: left,
                                y: bottom,
                                w: right - left,
                                h: top - bottom,
                                name: image_key_in(doc, resources, name),
                                alpha: fill_alpha.clamp(0.0, 1.0),
                                tint: match is_image_mask(doc, resources, name) {
                                    true => Some(fill),
                                    false => None,
                                },
                                mat: Some([ctm[0], ctm[1], ctm[2], ctm[3]]),
                            });
                        }
                    }
                    "BT" => {
                        tm = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
                        tlm = tm;
                    }
                    "Tf" if stack.len() >= 2 => {
                        if let Token::Num(sz) = stack[stack.len() - 1] {
                            font_size = sz;
                        }
                        if let Token::Name(n) = &stack[stack.len() - 2] {
                            cur_font = fonts.get(n);
                            cur_font_name = match cur_font {
                                Some(f) if !f.base_font.is_empty() => f.base_font.clone(),
                                _ => n.clone(),
                            };
                        }
                    }
                    "TL" => {
                        if let Some(Token::Num(v)) = stack.last() {
                            leading = *v;
                        }
                    }
                    "Tr" => {
                        if let Some(Token::Num(v)) = stack.last() {
                            ts.mode = *v as i64;
                        }
                    }
                    "Tc" => {
                        if let Some(Token::Num(v)) = stack.last() {
                            ts.char_spacing = *v;
                        }
                    }
                    "Tw" => {
                        if let Some(Token::Num(v)) = stack.last() {
                            ts.word_spacing = *v;
                        }
                    }
                    "Tz" => {
                        if let Some(Token::Num(v)) = stack.last() {
                            // Given as a percentage.
                            ts.h_scale = *v / 100.0;
                        }
                    }
                    "Ts" => {
                        if let Some(Token::Num(v)) = stack.last() {
                            ts.rise = *v;
                        }
                    }
                    "Td" | "TD" => {
                        if let Some([x, y]) = last2(&stack) {
                            if op == "TD" {
                                leading = -y;
                            }
                            tlm = mat_mul(&[1.0, 0.0, 0.0, 1.0, x, y], &tlm);
                            tm = tlm;
                        }
                    }
                    "Tm" => {
                        if let Some(m) = last6(&stack) {
                            tm = m;
                            tlm = m;
                        }
                    }
                    "T*" => {
                        tlm = mat_mul(&[1.0, 0.0, 0.0, 1.0, 0.0, -leading], &tlm);
                        tm = tlm;
                    }
                    "Tj" | "'" | "\"" => {
                        if op != "Tj" {
                            tlm = mat_mul(&[1.0, 0.0, 0.0, 1.0, 0.0, -leading], &tlm);
                            tm = tlm;
                        }
                        if let Some(Token::Str(bytes)) = stack.last() {
                            let adv = push_text_op(
                                bytes,
                                cur_font,
                                &cur_font_name,
                                font_size,
                                &tm,
                                &ctm,
                                fill,
                                &ts,
                                ops,
                            );
                            // Advance the text matrix by the run width so the
                            // next show on the same line is positioned correctly.
                            tm = mat_mul(&[1.0, 0.0, 0.0, 1.0, adv * font_size, 0.0], &tm);
                        }
                    }
                    "TJ" => {
                        if let Some(Token::ArrStr(parts)) = stack.last() {
                            // Split the run into segments at column-sized gaps so
                            // each lands at its true x; normal kerning/word spaces
                            // (small adjustments) stay merged into one segment.
                            const GAP_EM: f64 = 1.0;
                            // (start_em_from_run, text, per-char advances em)
                            let mut segs: Vec<(f64, String, Vec<f64>, Vec<u32>)> = Vec::new();
                            let mut cursor = 0.0f64;
                            let mut seg = String::new();
                            let mut seg_advs: Vec<f64> = Vec::new();
                            let mut seg_codes: Vec<u32> = Vec::new();
                            let mut seg_start = 0.0f64;
                            for part in parts {
                                match part {
                                    ArrPart::Str(bytes) => {
                                        if let Some(f) = cur_font {
                                            let before = seg_advs.iter().sum::<f64>();
                                            let from = seg_advs.len();
                                            f.decode_with_advances(
                                                bytes,
                                                &mut seg,
                                                &mut seg_advs,
                                                &mut seg_codes,
                                            );
                                            ts.space(
                                                &mut seg_advs,
                                                &seg_codes,
                                                from,
                                                font_size,
                                                f.two_byte,
                                            );
                                            cursor += seg_advs.iter().sum::<f64>() - before;
                                        } else {
                                            for &b in bytes {
                                                if let Some(c) = winansi(b) {
                                                    seg.push(c);
                                                    seg_advs.push(0.5);
                                                    seg_codes.push(b as u32);
                                                    cursor += 0.5;
                                                }
                                            }
                                        }
                                    }
                                    ArrPart::Num(adj) => {
                                        // TJ adjustment: forward advance (em).
                                        let gap = -adj / 1000.0;
                                        cursor += gap;
                                        if gap >= GAP_EM {
                                            // Column break: flush and jump.
                                            if !seg.trim().is_empty() {
                                                segs.push((
                                                    seg_start,
                                                    std::mem::take(&mut seg),
                                                    std::mem::take(&mut seg_advs),
                                                    std::mem::take(&mut seg_codes),
                                                ));
                                            } else {
                                                seg.clear();
                                                seg_advs.clear();
                                                seg_codes.clear();
                                            }
                                            seg_start = cursor;
                                        } else if *adj < -120.0 && !seg.ends_with(' ') {
                                            // Inter-word space: a space glyph carries the gap.
                                            seg.push(' ');
                                            seg_advs.push(gap);
                                            // The gap stands in for a space the
                                            // string never contained; code 32
                                            // keeps `codes` aligned with `text`.
                                            seg_codes.push(32);
                                        } else if let Some(idx) =
                                            seg_advs.iter().rposition(|&a| a != 0.0)
                                        {
                                            // Small kern: fold into the current
                                            // cluster's advance-bearing char,
                                            // skipping zero-advance ligature
                                            // continuations so clusters stay intact.
                                            seg_advs[idx] += gap;
                                        } else {
                                            // Leading kern before any glyph.
                                            seg_start += gap;
                                        }
                                    }
                                }
                            }
                            if !seg.trim().is_empty() {
                                segs.push((seg_start, seg, seg_advs, seg_codes));
                            }
                            let measured = cur_font.map(|f| !f.has_widths).unwrap_or(true);
                            // Read before the loop: `text` below shadows the
                            // state binding.
                            let visible = ts.visible();
                            let h_scale = ts.h_scale;
                            let rise = ts.rise;
                            for (start, text, mut advs, codes) in segs {
                                if !visible {
                                    continue;
                                }
                                // Horizontal scaling applies to the whole run:
                                // the offset to its start, and every advance
                                // within it.
                                if h_scale != 1.0 {
                                    for a in &mut advs {
                                        *a *= h_scale;
                                    }
                                }
                                let seg_tm = mat_mul(
                                    &[1.0, 0.0, 0.0, 1.0, start * font_size * h_scale, rise],
                                    &tm,
                                );
                                emit_text_op(
                                    &text,
                                    &cur_font_name,
                                    cur_font.map(|f| f.object).unwrap_or(0),
                                    font_size,
                                    &advs,
                                    &codes,
                                    measured,
                                    &seg_tm,
                                    &ctm,
                                    with_alpha(fill, fill_alpha),
                                    ops,
                                );
                            }
                            // Advance the text matrix by the total run width so a
                            // following show on the same line lands after it.
                            // The whole displacement scales, kerns included.
                            tm = mat_mul(
                                &[1.0, 0.0, 0.0, 1.0, cursor * font_size * h_scale, 0.0],
                                &tm,
                            );
                        }
                    }
                    _ => {}
                }
                stack.clear();
            }
            Token::Inline(data) => {
                // The dictionary arrived as the tokens between `BI` and `ID`.
                let dict = inline_image_dict(&stack);
                stack.clear();
                if fill_alpha <= 0.0 {
                    continue;
                }
                let corners = [
                    transform_point(&ctm, 0.0, 0.0),
                    transform_point(&ctm, 1.0, 0.0),
                    transform_point(&ctm, 1.0, 1.0),
                    transform_point(&ctm, 0.0, 1.0),
                ];
                let xs = corners.map(|c| c.0);
                let ys = corners.map(|c| c.1);
                let left = xs.iter().cloned().fold(f64::INFINITY, f64::min);
                let right = xs.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                let bottom = ys.iter().cloned().fold(f64::INFINITY, f64::min);
                let top = ys.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                let stencil = matches!(dict.get("ImageMask"), Some(Object::Bool(true)));
                ops.push(RenderOp::Image {
                    x: left,
                    y: bottom,
                    w: right - left,
                    h: top - bottom,
                    name: inline_image_key(&dict, &data),
                    alpha: fill_alpha.clamp(0.0, 1.0),
                    tint: match stencil {
                        true => Some(fill),
                        false => None,
                    },
                    mat: Some([ctm[0], ctm[1], ctm[2], ctm[3]]),
                });
            }
            other => {
                stack.push(other);
                if stack.len() > 64 {
                    stack.remove(0);
                }
            }
        }
        if ops.len() > 2_000_000 {
            break;
        }
    }
}

/// The text-state parameters that move the pen but are not the font or the
/// matrix: `Tc`, `Tw`, `Tz`, `Ts`, and `Tr`.
///
/// Justified text is the common case. Many producers justify a line by setting
/// a word spacing rather than by emitting `TJ` kerns, so a reader that ignores
/// `Tw` packs every word against the next and the line ends short.
#[derive(Debug, Clone, Copy)]
struct TextState {
    /// `Tc`, added to every glyph advance, in unscaled text units.
    char_spacing: f64,
    /// `Tw`, added to single-byte code 32 only, in unscaled text units.
    word_spacing: f64,
    /// `Tz` as a factor: it scales every horizontal displacement.
    h_scale: f64,
    /// `Ts`, a baseline offset in unscaled text units.
    rise: f64,
    /// `Tr`, the rendering mode.
    mode: i64,
}

impl Default for TextState {
    fn default() -> Self {
        Self {
            char_spacing: 0.0,
            word_spacing: 0.0,
            h_scale: 1.0,
            rise: 0.0,
            mode: 0,
        }
    }
}

impl TextState {
    /// Whether this mode paints anything.
    ///
    /// Mode 3 is invisible and mode 7 clips without painting. That is how an
    /// OCR layer sits over a scanned page: the words are there to be searched
    /// and selected, not to be seen. Drawing them lays the transcription on
    /// top of the picture of the same words. Extraction still wants them, so
    /// only the display list skips them.
    fn visible(&self) -> bool {
        self.mode != 3 && self.mode != 7
    }

    /// Add the per-glyph spacing to advances the font just appended.
    ///
    /// `Tc` and `Tw` are in unscaled text units while advances are em
    /// fractions, so they divide by the font size to match. Zero-advance
    /// entries are the continuation characters of a decomposed ligature --
    /// one glyph, several chars -- and spacing is per glyph, so they are
    /// left alone.
    fn space(&self, advs: &mut [f64], codes: &[u32], from: usize, font_size: f64, two_byte: bool) {
        if font_size == 0.0 || (self.char_spacing == 0.0 && self.word_spacing == 0.0) {
            return;
        }
        for (adv, code) in advs.iter_mut().zip(codes.iter()).skip(from) {
            if *adv == 0.0 {
                continue;
            }
            let mut extra = self.char_spacing;
            // Word spacing applies to the single-byte code 32 and to nothing
            // else -- notably not to a two-byte code that happens to be 32.
            if !two_byte && *code == 32 {
                extra += self.word_spacing;
            }
            *adv += extra / font_size;
        }
    }
}

/// Decodes and emits a single text show, returning its advance (em fractions)
/// so the caller can move the text matrix forward by the run width.
#[allow(clippy::too_many_arguments)]
fn push_text_op(
    bytes: &[u8],
    font: Option<&Font>,
    font_name: &str,
    font_size: f64,
    tm: &Matrix,
    ctm: &Matrix,
    color: [f64; 4],
    ts: &TextState,
    ops: &mut Vec<RenderOp>,
) -> f64 {
    let mut s = String::new();
    let mut advs: Vec<f64> = Vec::new();
    let mut codes: Vec<u32> = Vec::new();
    if let Some(f) = font {
        f.decode_with_advances(bytes, &mut s, &mut advs, &mut codes);
    } else {
        for &b in bytes {
            if let Some(c) = winansi(b) {
                s.push(c);
                advs.push(0.5);
                codes.push(b as u32);
            }
        }
    }
    ts.space(
        &mut advs,
        &codes,
        0,
        font_size,
        font.map(|f| f.two_byte).unwrap_or(false),
    );
    // Horizontal scaling multiplies every horizontal displacement, spacing
    // included.
    if ts.h_scale != 1.0 {
        for a in &mut advs {
            *a *= ts.h_scale;
        }
    }
    let advance: f64 = advs.iter().sum();
    let measured = font.map(|f| !f.has_widths).unwrap_or(true);
    // The advance is returned either way: invisible text still moves the pen,
    // and anything drawn after it on the line depends on that.
    if ts.visible() && !s.trim().is_empty() {
        // Rise lifts the run off the baseline -- superscripts and footnote
        // markers -- without disturbing the line it sits on.
        let placed = mat_mul(&[1.0, 0.0, 0.0, 1.0, 0.0, ts.rise], tm);
        let face = font.map(|f| f.object).unwrap_or(0);
        emit_text_op(
            &s, font_name, face, font_size, &advs, &codes, measured, &placed, ctm, color, ops,
        );
    }
    advance
}

/// `advs_em` holds per-character advances in em fractions (× font size × CTM
/// scale = device width), parallel to the chars of `text`.
#[allow(clippy::too_many_arguments)]
fn emit_text_op(
    text: &str,
    font_name: &str,
    face: u32,
    font_size: f64,
    advs_em: &[f64],
    codes: &[u32],
    measured: bool,
    tm: &Matrix,
    ctm: &Matrix,
    color: [f64; 4],
    ops: &mut Vec<RenderOp>,
) {
    let trm = mat_mul(tm, ctm);
    let scale = (trm[0] * trm[3] - trm[1] * trm[2]).abs().sqrt();
    // A negative `Tf` size is legal and means mirrored text. Producers pair it
    // with a flipped matrix so the glyphs come out upright, and one whole
    // family of tax forms is written that way. Treating it as "no size given"
    // fell back to the matrix scale alone: every run on the page was laid out
    // at 1.3 points instead of 5.9, which on a rendered page is nothing at
    // all. The orientation is already carried by the matrix, so what is wanted
    // here is the magnitude.
    let magnitude = font_size.abs();
    let size = if magnitude > 0.0 {
        magnitude * scale.max(0.01)
    } else {
        scale.max(1.0)
    };
    // The advance keeps the sign, and the glyph size does not. A producer that
    // writes a negative size writes negative widths to go with it, so the two
    // cancel and the pen still moves forward -- taking the magnitude here as
    // well would send every run backwards along its own line, piling the whole
    // page into its left margin.
    let fsz = if font_size != 0.0 { font_size } else { 1.0 };
    // Device advance = text-space width (em × fontSize) × CTM scale.
    let factor = fsz * scale.max(0.01);
    let advances: Vec<f64> = advs_em.iter().map(|a| a * factor).collect();
    let width = advances.iter().sum();
    let rot = trm[1].atan2(trm[0]);
    ops.push(RenderOp::Text {
        text: text.to_string(),
        x: trm[4],
        y: trm[5],
        size,
        width,
        advances,
        codes: codes.to_vec(),
        measured,
        rot,
        color,
        font: font_name.to_string(),
        face,
    });
}

/// An image XObject with its filter-decoded bytes (Flate/ASCII undone; image
/// codecs like DCTDecode left encoded so a decoder can consume them).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageBlob {
    pub page_number: usize,
    pub width: u32,
    pub height: u32,
    pub bits_per_component: u8,
    pub color_space: String,
    pub filter: String,
    pub bytes: Vec<u8>,
    /// Color lookup table for an Indexed color space, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub palette: Option<Palette>,
    /// CCITT fax parameters, if the image uses CCITTFaxDecode.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ccitt: Option<CcittParams>,
}

/// CCITTFaxDecode parameters from the image's `/DecodeParms`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CcittParams {
    /// `/K`: <0 = Group 4 (T.6), 0 = Group 3 1-D, >0 = Group 3 2-D.
    pub k: i64,
    /// `/Columns` (pixels per row); defaults to the image width.
    pub columns: u32,
    /// `/Rows`; defaults to the image height.
    pub rows: u32,
    /// `/BlackIs1`: if true, 1 bits are black (inverts the default).
    pub black_is_1: bool,
}

impl Palette {
    /// The colour a row of the table stands for, as RGBA.
    ///
    /// An index past the end is black, which is what a reader shows for a
    /// table shorter than the content claims.
    fn entry(&self, index: usize) -> [f64; 4] {
        let n = self.base_components.max(1) as usize;
        let at = index * n;
        let Some(row) = self.data.get(at..at + n) else {
            return [0.0, 0.0, 0.0, 1.0];
        };
        let v = |i: usize| row.get(i).map(|b| *b as f64 / 255.0).unwrap_or(0.0);
        match n {
            1 => [v(0), v(0), v(0), 1.0],
            4 => {
                let k = v(3);
                [
                    (1.0 - v(0)) * (1.0 - k),
                    (1.0 - v(1)) * (1.0 - k),
                    (1.0 - v(2)) * (1.0 - k),
                    1.0,
                ]
            }
            _ => [v(0), v(1), v(2), 1.0],
        }
    }
}

/// An Indexed color-space palette: each entry is `base_components` bytes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Palette {
    /// Components per entry in the base color space (3=RGB, 1=Gray, 4=CMYK).
    pub base_components: u8,
    /// Flattened lookup table: `(hival + 1) * base_components` bytes.
    pub data: Vec<u8>,
}

/// Extract image XObjects with their (partially) decoded byte payloads.
pub fn extract_image_blobs(data: &[u8]) -> Result<Vec<ImageBlob>, PdfError> {
    let doc = Document::parse(data)?;
    let root = doc
        .trailer
        .get("Root")
        .map(|o| doc.resolve(o))
        .and_then(|o| o.as_dict().cloned())
        .unwrap_or_default();
    let mut page_dicts: Vec<Dict> = Vec::new();
    if let Some(pages_obj) = doc.get(&root, "Pages")
        && let Some(pages_dict) = pages_obj.as_dict()
    {
        let mut visited = std::collections::HashSet::new();
        collect_pages(
            &doc,
            pages_dict,
            &mut page_dicts,
            &Inherited::default(),
            0,
            &mut visited,
        );
    }
    let mut out = Vec::new();
    for (idx, pd) in page_dicts.iter().enumerate() {
        let xobjects = doc
            .get(pd, "Resources")
            .and_then(|o| o.as_dict().cloned())
            .and_then(|r| doc.get(&r, "XObject"))
            .and_then(|o| o.as_dict().cloned())
            .unwrap_or_default();
        for xref in xobjects.values() {
            let obj = doc.resolve(xref);
            if let Object::Stream(d, raw) = &obj {
                if doc
                    .get(d, "Subtype")
                    .and_then(|o| o.as_name().map(String::from))
                    .as_deref()
                    != Some("Image")
                {
                    continue;
                }
                let bytes = decode_stream(d, raw).unwrap_or_default();
                out.push(ImageBlob {
                    page_number: idx + 1,
                    width: doc.get(d, "Width").and_then(|o| o.as_int()).unwrap_or(0) as u32,
                    height: doc.get(d, "Height").and_then(|o| o.as_int()).unwrap_or(0) as u32,
                    bits_per_component: doc
                        .get(d, "BitsPerComponent")
                        .and_then(|o| o.as_int())
                        .unwrap_or(8) as u8,
                    color_space: color_space_name(&doc, d),
                    filter: filter_names(d),
                    bytes,
                    palette: extract_palette(&doc, d),
                    ccitt: extract_ccitt(&doc, d),
                });
            }
        }
    }
    Ok(out)
}

/// An image placed on a page, decoded for a renderer. `data` is base64: either
/// the original JPEG bytes (`format == "jpeg"`, the browser decodes) or raw
/// RGBA (`format == "rgba"`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageImage {
    pub name: String,
    /// On-page placement [x, y, w, h] in device (PDF point) space.
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub format: String,
    pub width: u32,
    pub height: u32,
    pub data: String,
}

/// A page image decoded all the way to pixels a painter can draw.
///
/// [`PageImage`] hands JPEG bytes on untouched, which is right for a browser
/// -- it has a decoder and base64 of an already-compressed photograph is far
/// smaller than base64 of its pixels. A native painter has neither luxury and
/// wants the pixels.
pub struct PageTexture {
    /// The resource name a [`RenderOp::Image`] refers to.
    pub name: String,
    pub width: u32,
    pub height: u32,
    /// `width * height * 4` bytes, straight RGBA.
    pub rgba: Vec<u8>,
}

/// Gather the image XObjects a resource dictionary can reach, following forms.
///
/// Outer names win: where a form gives a different image the same name as the
/// page does, the page's is the one the page's own operators mean.
/// A key for an image resource that is unique across the document.
///
/// A resource name is only unique inside one resource dictionary. A page and a
/// form it draws may each call a different picture `/Im0`, and keying the
/// texture table by name alone hands one of them the other's image -- which is
/// how a page banner ends up wearing a photograph from the middle of the page.
/// The object number is unique across the file, so it is preferred; a picture
/// written inline in the resource dictionary has no number and keeps its name.
fn image_key(entry: &Object, name: &str) -> String {
    match entry.as_ref() {
        Some((num, generation)) => format!("{num} {generation} R"),
        None => name.to_string(),
    }
}

/// The same key, looked up by the name a `Do` operator gives.
/// The keys an inline image abbreviates, spelled out.
///
/// An inline image writes `/W` where an XObject writes `/Width`. Both spellings
/// are legal in either place, so the dictionary is normalised here and every
/// reader of it downstream -- the decoder especially -- sees one vocabulary.
fn inline_key(abbrev: &str) -> &str {
    match abbrev {
        "W" => "Width",
        "H" => "Height",
        "BPC" => "BitsPerComponent",
        "CS" => "ColorSpace",
        "F" => "Filter",
        "D" => "Decode",
        "DP" => "DecodeParms",
        "IM" => "ImageMask",
        "I" => "Interpolate",
        other => other,
    }
}

/// The colour-space names an inline image abbreviates.
fn inline_space(name: &str) -> &str {
    match name {
        "G" => "DeviceGray",
        "RGB" => "DeviceRGB",
        "CMYK" => "DeviceCMYK",
        "I" => "Indexed",
        other => other,
    }
}

/// Rebuild an inline image's dictionary from the tokens before its `ID`.
fn inline_image_dict(stack: &[Token]) -> Dict {
    let mut dict = Dict::new();
    let mut at = 0;
    while at < stack.len() {
        let Token::Name(key) = &stack[at] else {
            at += 1;
            continue;
        };
        let key = inline_key(key).to_string();
        let value = match stack.get(at + 1) {
            Some(Token::Num(n)) => Object::Int(*n as i64),
            Some(Token::Bool(v)) => Object::Bool(*v),
            Some(Token::Name(n)) => Object::Name(inline_space(n).to_string()),
            // `/D [1 0]` and `/F [/Fl]` arrive as an array token.
            Some(Token::ArrStr(parts)) => Object::Array(
                parts
                    .iter()
                    .filter_map(|p| match p {
                        ArrPart::Num(n) => Some(Object::Int(*n as i64)),
                        ArrPart::Str(_) => None,
                    })
                    .collect(),
            ),
            _ => {
                at += 1;
                continue;
            }
        };
        dict.insert(key, value);
        at += 2;
    }
    dict
}

/// A texture name for an inline image, stable across the two passes.
///
/// An inline image has no resource name -- it *is* its bytes. The display list
/// and the texture walk each meet it separately and must agree on what to call
/// it, so the name is derived from the samples and the shape they decode to
/// rather than from where it sits on the page. Two identical inline images
/// then also share one texture, which the documents that use hundreds of them
/// very much want.
fn inline_image_key(dict: &Dict, data: &[u8]) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    let mut eat = |bytes: &[u8]| {
        for b in bytes {
            hash ^= *b as u64;
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    };
    eat(data);
    for key in ["Width", "Height", "BitsPerComponent", "ImageMask", "Decode"] {
        eat(key.as_bytes());
        if let Some(v) = dict.get(key) {
            eat(format!("{v:?}").as_bytes());
        }
    }
    format!("inline {hash:016x}")
}

/// Gather every inline image a content stream draws, following what it draws.
///
/// An inline image belongs to the stream rather than to a resource dictionary,
/// so the walk that finds XObjects cannot see one. This walk mirrors it: the
/// page's own content, then every form it draws and every tiling pattern it
/// fills with, to the same depth limit.
fn collect_inline_images(
    doc: &Document,
    content: &[u8],
    resources: &Dict,
    depth: usize,
    out: &mut Vec<(Dict, Vec<u8>)>,
) {
    if depth > MAX_FORM_DEPTH {
        return;
    }
    let mut lex = ContentLexer::new(content);
    let mut stack: Vec<Token> = Vec::new();
    while let Some(tok) = lex.next_token() {
        match tok {
            Token::Inline(data) => {
                out.push((inline_image_dict(&stack), data));
                stack.clear();
            }
            Token::Op(op) => {
                if op == "Do"
                    && let Some(Token::Name(name)) = stack.last()
                    && let Some((dict, inner)) = form_xobject(doc, resources, name)
                {
                    let inner_resources = doc
                        .get(&dict, "Resources")
                        .and_then(|o| o.as_dict().cloned())
                        .unwrap_or_else(|| resources.clone());
                    collect_inline_images(doc, &inner, &inner_resources, depth + 1, out);
                }
                stack.clear();
            }
            other => {
                stack.push(other);
                if stack.len() > 64 {
                    stack.remove(0);
                }
            }
        }
    }
    // A tiling pattern is a content stream too, and a page that fills with one
    // never names what the tile draws.
    if let Some(patterns) = doc
        .get(resources, "Pattern")
        .and_then(|o| o.as_dict().cloned())
    {
        for entry in patterns.values() {
            if let Object::Stream(d, raw) = doc.resolve(entry)
                && let Ok(tile) = decode_stream(&d, &raw)
            {
                let inner = doc
                    .get(&d, "Resources")
                    .and_then(|o| o.as_dict().cloned())
                    .unwrap_or_else(|| resources.clone());
                collect_inline_images(doc, &tile, &inner, depth + 1, out);
            }
        }
    }
}

/// Whether the named image XObject is a stencil rather than a picture.
fn is_image_mask(doc: &Document, resources: &Dict, name: &str) -> bool {
    doc.get(resources, "XObject")
        .and_then(|o| o.as_dict().cloned())
        .and_then(|xobjects| xobjects.get(name).map(|entry| doc.resolve(entry)))
        .and_then(|resolved| match resolved {
            Object::Stream(d, _) => Some(d),
            _ => None,
        })
        .is_some_and(|d| matches!(doc.get(&d, "ImageMask"), Some(Object::Bool(true))))
}

fn image_key_in(doc: &Document, resources: &Dict, name: &str) -> String {
    doc.get(resources, "XObject")
        .and_then(|o| o.as_dict().cloned())
        .and_then(|xobjects| xobjects.get(name).map(|entry| image_key(entry, name)))
        .unwrap_or_else(|| name.to_string())
}

fn collect_image_xobjects(doc: &Document, resources: &Dict, out: &mut Dict, depth: usize) {
    if depth > MAX_FORM_DEPTH {
        return;
    }
    // A tiling pattern is a content stream with resources of its own, and what
    // it draws is very often one image. Reaching it only through `/XObject`
    // finds the pattern's *users* and never its pictures, so a page painted
    // with one names a texture nobody has.
    if let Some(patterns) = doc
        .get(resources, "Pattern")
        .and_then(|o| o.as_dict().cloned())
    {
        for entry in patterns.values() {
            if let Object::Stream(d, _) = doc.resolve(entry)
                && let Some(inner) = doc.get(&d, "Resources").and_then(|o| o.as_dict().cloned())
            {
                collect_image_xobjects(doc, &inner, out, depth + 1);
            }
        }
    }
    let Some(xobjects) = doc
        .get(resources, "XObject")
        .and_then(|o| o.as_dict().cloned())
    else {
        return;
    };
    for (name, entry) in xobjects.iter() {
        let resolved = doc.resolve(entry);
        let Object::Stream(d, _) = &resolved else {
            continue;
        };
        match doc
            .get(d, "Subtype")
            .and_then(|o| o.as_name().map(String::from))
            .as_deref()
        {
            Some("Image") => {
                let key = image_key(entry, name);
                out.entry(key).or_insert_with(|| entry.clone());
            }
            Some("Form") => {
                if let Some(inner) = doc.get(d, "Resources").and_then(|o| o.as_dict().cloned()) {
                    collect_image_xobjects(doc, &inner, out, depth + 1);
                }
            }
            _ => {}
        }
    }
}

/// Decode every image a page draws, by resource name.
pub fn extract_page_textures(
    data: &[u8],
    page_number: usize,
) -> Result<Vec<PageTexture>, PdfError> {
    let doc = Document::parse(data)?;
    let root = doc
        .trailer
        .get("Root")
        .map(|o| doc.resolve(o))
        .and_then(|o| o.as_dict().cloned())
        .unwrap_or_default();
    let mut page_dicts: Vec<Dict> = Vec::new();
    if let Some(pages_obj) = doc.get(&root, "Pages")
        && let Some(pages_dict) = pages_obj.as_dict()
    {
        let mut visited = std::collections::HashSet::new();
        collect_pages(
            &doc,
            pages_dict,
            &mut page_dicts,
            &Inherited::default(),
            0,
            &mut visited,
        );
    }
    let Some(pd) = page_dicts.get(page_number.saturating_sub(1)) else {
        return Ok(Vec::new());
    };
    let resources = doc
        .get(pd, "Resources")
        .and_then(|o| o.as_dict().cloned())
        .unwrap_or_default();
    // A form's images are drawn under the form's own names, so they have to be
    // collected the way the interpreter reaches them. Without this a page that
    // wraps its artwork in a form draws an op naming an image nobody has, and
    // a photograph the size of the page simply does not appear.
    let mut xobjects = Dict::new();
    collect_image_xobjects(&doc, &resources, &mut xobjects, 0);

    let mut out = Vec::new();
    for (name, obj) in xobjects.iter() {
        let resolved = doc.resolve(obj);
        let Object::Stream(d, raw) = &resolved else {
            continue;
        };
        if doc
            .get(d, "Subtype")
            .and_then(|o| o.as_name().map(String::from))
            .as_deref()
            != Some("Image")
        {
            continue;
        }
        let Some((format, width, height, bytes)) = image_to_rgba(&doc, d, raw) else {
            continue;
        };
        // A Separation or DeviceN image carries colorant *tints*, not colour:
        // full tint is full ink, which is dark. Read as intensity, every such
        // image comes out as its own negative. Inverting is the same
        // approximation the fill colours use, and for the one-ink and
        // process-ink spaces these documents use it lands on the right side of
        // black and white.
        let tint = matches!(color_space_name(&doc, d).as_str(), "Separation" | "DeviceN");
        let mut rgba = match format.as_str() {
            "rgba" => bytes,
            "jpeg" => {
                // A codec the platform may or may not have. Where it cannot be
                // decoded the image is left out and the painter draws its
                // frame, which is honest about there being something there.
                let Some(image) = crate::image::jpeg::decode(&bytes) else {
                    continue;
                };
                if image.width != width || image.height != height {
                    continue;
                }
                let mut rgba = Vec::with_capacity(image.rgb.len() / 3 * 4);
                for pixel in image.rgb.as_chunks::<3>().0 {
                    rgba.extend_from_slice(pixel);
                    rgba.push(255);
                }
                // The soft mask is a separate image and applies either way.
                apply_smask(&doc, d, &mut rgba, width as usize, height as usize);
                rgba
            }
            _ => continue,
        };
        if rgba.len() != width as usize * height as usize * 4 {
            rgba.resize(width as usize * height as usize * 4, 0);
        }
        if tint {
            for pixel in rgba.as_chunks_mut::<4>().0 {
                pixel[0] = 255 - pixel[0];
                pixel[1] = 255 - pixel[1];
                pixel[2] = 255 - pixel[2];
            }
        }
        out.push(PageTexture {
            name: name.clone(),
            width,
            height,
            rgba,
        });
    }

    // Inline images are not in any resource dictionary: they are the content
    // stream. The display list names them by a hash of their own samples, so
    // the two walks agree without either knowing where the other looked.
    let mut inline = Vec::new();
    collect_inline_images(&doc, &page_contents(&doc, pd), &resources, 0, &mut inline);
    for (dict, data) in &inline {
        let name = inline_image_key(dict, data);
        if out.iter().any(|t| t.name == name) {
            continue;
        }
        let Some((format, width, height, bytes)) = image_to_rgba(&doc, dict, data) else {
            continue;
        };
        // The `jpeg` form is a stream of coded bytes for a host that decodes
        // them; a texture is pixels, so only the decoded form is one.
        if format != "rgba" {
            continue;
        }
        out.push(PageTexture {
            name,
            width,
            height,
            rgba: bytes,
        });
    }
    Ok(out)
}

/// Extract a page's placed images with decoded pixels, for the renderer.
pub fn extract_page_images(data: &[u8], page_number: usize) -> Result<Vec<PageImage>, PdfError> {
    let doc = Document::parse(data)?;
    let root = doc
        .trailer
        .get("Root")
        .map(|o| doc.resolve(o))
        .and_then(|o| o.as_dict().cloned())
        .unwrap_or_default();
    let mut page_dicts: Vec<Dict> = Vec::new();
    if let Some(pages_obj) = doc.get(&root, "Pages")
        && let Some(pages_dict) = pages_obj.as_dict()
    {
        let mut visited = std::collections::HashSet::new();
        collect_pages(
            &doc,
            pages_dict,
            &mut page_dicts,
            &Inherited::default(),
            0,
            &mut visited,
        );
    }
    let pd = match page_dicts.get(page_number.saturating_sub(1)) {
        Some(p) => p,
        None => return Ok(Vec::new()),
    };
    let xobjects = doc
        .get(pd, "Resources")
        .and_then(|o| o.as_dict().cloned())
        .and_then(|r| doc.get(&r, "XObject"))
        .and_then(|o| o.as_dict().cloned())
        .unwrap_or_default();
    let content = extract_page_content(&doc, pd, page_number);
    let mut out = Vec::new();
    for (name, bbox) in content.placed {
        let xobj = match xobjects.get(&name) {
            Some(o) => doc.resolve(o),
            None => continue,
        };
        if let Object::Stream(d, raw) = &xobj
            && doc
                .get(d, "Subtype")
                .and_then(|o| o.as_name().map(String::from))
                .as_deref()
                == Some("Image")
            && let Some((format, width, height, bytes)) = image_to_rgba(&doc, d, raw)
        {
            out.push(PageImage {
                name,
                x: bbox[0],
                y: bbox[1],
                w: bbox[2] - bbox[0],
                h: bbox[3] - bbox[1],
                format,
                width,
                height,
                data: b64e(&bytes),
            });
        }
    }
    Ok(out)
}

/// Decode an image to RGBA bytes, or pass JPEG/JPX through for browser decode.
/// Returns (format, width, height, bytes). Pure Rust (no image codec deps).
fn image_to_rgba(doc: &Document, d: &Dict, raw: &[u8]) -> Option<(String, u32, u32, Vec<u8>)> {
    let w = doc.get(d, "Width").and_then(|o| o.as_int())? as usize;
    let h = doc.get(d, "Height").and_then(|o| o.as_int())? as usize;
    if w == 0 || h == 0 || w * h > 64_000_000 {
        return None;
    }
    let filter = filter_names(d);
    if filter.contains("JPXDecode") {
        // JPEG 2000 is the one codec no browser but Safari decodes, so the
        // bytes have to become pixels here or the page renders blank.
        let bytes = decode_stream(d, raw).ok()?;
        let image = crate::image::jpx::decode(&bytes)?;
        if image.width as usize != w || image.height as usize != h {
            return None;
        }
        let mut rgba = Vec::with_capacity(w * h * 4);
        for p in image.data.chunks_exact(image.components) {
            if image.components >= 3 {
                rgba.extend_from_slice(&[p[0], p[1], p[2], 255]);
            } else {
                rgba.extend_from_slice(&[p[0], p[0], p[0], 255]);
            }
        }
        apply_smask(doc, d, &mut rgba, w, h);
        apply_stencil_mask(doc, d, &mut rgba, w, h);
        return Some(("rgba".into(), w as u32, h as u32, rgba));
    }
    if filter.contains("DCTDecode") {
        // Hand the JPEG bytes to the browser, which decodes them natively.
        let bytes = decode_stream(d, raw).ok()?;
        return Some(("jpeg".into(), w as u32, h as u32, bytes));
    }
    let mut decoded = decode_stream(d, raw).ok()?;
    // A fax image arrives run-length coded. Decoding it here turns it into an
    // ordinary one-bit grey image, which the rest of this function already
    // knows how to paint.
    if filter.contains("CCITTFaxDecode") {
        let params = extract_ccitt(doc, d)?;
        let byte_align = match doc.get(d, "DecodeParms").or_else(|| doc.get(d, "DP")) {
            Some(Object::Dict(p)) => {
                matches!(doc.get(&p, "EncodedByteAlign"), Some(Object::Bool(true)))
            }
            Some(Object::Array(a)) => a.iter().any(|o| match doc.resolve(o) {
                Object::Dict(p) => {
                    matches!(doc.get(&p, "EncodedByteAlign"), Some(Object::Bool(true)))
                }
                _ => false,
            }),
            _ => false,
        };
        decoded = crate::image::ccitt::decode(
            &decoded,
            &crate::image::ccitt::Params {
                k: params.k,
                columns: params.columns as usize,
                rows: if params.rows == 0 {
                    h
                } else {
                    params.rows as usize
                },
                byte_align,
            },
        )?;
    }
    // A JBIG2 image arrives as its own little document of segments. Decoding
    // it here turns it into an ordinary one-bit grey image.
    if filter.contains("JBIG2Decode") {
        decoded = decode_jbig2(doc, d, &decoded, w, h)?;
    }
    let bpc = doc
        .get(d, "BitsPerComponent")
        .and_then(|o| o.as_int())
        .unwrap_or(8) as u8;
    let cs = color_space_name(doc, d);
    let n = w * h;

    // An image mask is a stencil, not a picture: one bit a sample, no colour
    // space at all, and the colour it paints is whatever the fill colour was
    // at the `Do`. That colour is not here to be had, so the texture carries
    // coverage in its alpha and leaves the tint to the renderer, which gets
    // it from the op. Without this the whole class decodes to nothing and the
    // painter draws its frame -- a grey box where a journal masthead goes.
    if matches!(doc.get(d, "ImageMask"), Some(Object::Bool(true))) {
        let samples = unpack_samples(&decoded, w, h, 1)?;
        // `/Decode [1 0]` swaps which sample value marks the page. The
        // default is that a zero paints, which is the opposite of the
        // intuition and the reason this is read rather than assumed.
        let invert = matches!(doc.get(d, "Decode"), Some(Object::Array(a))
            if a.first().and_then(|o| doc.resolve(o).as_f64()) == Some(1.0));
        let mut rgba = Vec::with_capacity(n * 4);
        for sample in samples {
            let paints = (sample == 0) != invert;
            // White under the coverage: a renderer that ignores the tint
            // draws nothing visible rather than a black rectangle.
            rgba.extend_from_slice(&[255, 255, 255, if paints { 255 } else { 0 }]);
        }
        return Some(("rgba".into(), w as u32, h as u32, rgba));
    }

    let gray = matches!(cs.as_str(), "DeviceGray" | "CalGray" | "G");
    let mut rgba = Vec::with_capacity(n * 4);

    if cs == "Indexed"
        && let Some(pal) = extract_palette(doc, d)
        && let Some(idx) = unpack_samples(&decoded, w, h, bpc)
    {
        let bc = pal.base_components as usize;
        for i in idx {
            let off = i as usize * bc;
            let g = |k: usize| pal.data.get(off + k).copied().unwrap_or(0);
            if bc >= 3 {
                rgba.extend_from_slice(&[g(0), g(1), g(2), 255]);
            } else {
                let v = g(0);
                rgba.extend_from_slice(&[v, v, v, 255]);
            }
        }
    } else if gray && let Some(samples) = unpack_samples(&decoded, w, h, bpc) {
        let max = ((1u32 << bpc as usize) - 1).max(1);
        for s in samples {
            let v = ((s * 255) / max) as u8;
            rgba.extend_from_slice(&[v, v, v, 255]);
        }
    } else if bpc == 8 && decoded.len() >= n * 3 {
        for p in decoded.as_chunks::<3>().0.iter().take(n) {
            rgba.extend_from_slice(&[p[0], p[1], p[2], 255]);
        }
    }

    if rgba.len() != n * 4 {
        return None;
    }
    apply_smask(doc, d, &mut rgba, w, h);
    apply_stencil_mask(doc, d, &mut rgba, w, h);
    Some(("rgba".into(), w as u32, h as u32, rgba))
}

/// Apply an image's soft-mask (`/SMask`, a grayscale image) as the alpha
/// channel of `rgba`, nearest-sampling if the mask resolution differs.
fn apply_smask(doc: &Document, d: &Dict, rgba: &mut [u8], w: usize, h: usize) {
    let smask = match doc.get(d, "SMask") {
        Some(Object::Stream(sd, sraw)) => (sd, sraw),
        _ => return,
    };
    let (sd, sraw) = smask;
    let sfilter = filter_names(&sd);
    let sw = doc.get(&sd, "Width").and_then(|o| o.as_int()).unwrap_or(0) as usize;
    let sh = doc.get(&sd, "Height").and_then(|o| o.as_int()).unwrap_or(0) as usize;
    let sbpc = doc
        .get(&sd, "BitsPerComponent")
        .and_then(|o| o.as_int())
        .unwrap_or(8) as u8;
    if sw == 0 || sh == 0 {
        return;
    }
    let decoded = match decode_stream(&sd, &sraw) {
        Ok(b) => b,
        Err(_) => return,
    };
    // A mask coded as JPEG or JPEG 2000 arrives still coded -- `decode_stream`
    // undoes the wrappers and leaves the codec alone -- so it decodes here,
    // down to one byte a pixel.
    let (samples, max) = if sfilter.contains("DCTDecode") {
        let Some(image) = crate::image::jpeg::decode(&decoded) else {
            return;
        };
        if image.width as usize != sw || image.height as usize != sh {
            return;
        }
        // The mask is grey: every channel of the decoded pixel is the same.
        (
            image
                .rgb
                .as_chunks::<3>()
                .0
                .iter()
                .map(|p| p[0] as u32)
                .collect(),
            255u32,
        )
    } else if sfilter.contains("JPXDecode") {
        let Some(image) = crate::image::jpx::decode(&decoded) else {
            return;
        };
        if image.width as usize != sw || image.height as usize != sh {
            return;
        }
        (
            image
                .data
                .chunks_exact(image.components)
                .map(|p| p[0] as u32)
                .collect(),
            255u32,
        )
    } else {
        let Some(samples) = unpack_samples(&decoded, sw, sh, sbpc) else {
            return;
        };
        (samples, ((1u32 << sbpc as usize) - 1).max(1))
    };
    for y in 0..h {
        let sy = y * sh / h;
        for x in 0..w {
            let sx = x * sw / w;
            let a = (samples[sy * sw + sx] * 255 / max) as u8;
            rgba[(y * w + x) * 4 + 3] = a;
        }
    }
}

/// Unpack a row-padded sample stream (1/2/4/8 bpc) to raw integer values.
pub(crate) fn unpack_samples(bytes: &[u8], w: usize, h: usize, bpc: u8) -> Option<Vec<u32>> {
    let bits = bpc as usize;
    if !matches!(bits, 1 | 2 | 4 | 8) {
        return None;
    }
    let stride = (w * bits).div_ceil(8);
    if bytes.len() < stride.checked_mul(h)? {
        return None;
    }
    let max = (1u32 << bits) - 1;
    let mut out = Vec::with_capacity(w * h);
    for row in 0..h {
        let base = row * stride;
        for col in 0..w {
            let bit_pos = col * bits;
            let byte = bytes[base + bit_pos / 8];
            let shift = 8 - bits - (bit_pos % 8);
            out.push((byte >> shift) as u32 & max);
        }
    }
    Some(out)
}

/// Standard-alphabet base64 (no dependency).
/// Base64, for embedding bytes in JSON. Public because a recording inlines
/// glyph masks for a host that has no other way to receive them.
pub fn b64e(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for c in data.chunks(3) {
        let n = (c[0] as u32) << 16
            | (*c.get(1).unwrap_or(&0) as u32) << 8
            | (*c.get(2).unwrap_or(&0) as u32);
        out.push(T[(n >> 18 & 63) as usize] as char);
        out.push(T[(n >> 12 & 63) as usize] as char);
        out.push(if c.len() > 1 {
            T[(n >> 6 & 63) as usize] as char
        } else {
            '='
        });
        out.push(if c.len() > 2 {
            T[(n & 63) as usize] as char
        } else {
            '='
        });
    }
    out
}

/// Parse an Indexed color-space palette from an image dict, if present.
fn extract_palette(doc: &Document, d: &Dict) -> Option<Palette> {
    let cs = doc.get(d, "ColorSpace").or_else(|| doc.get(d, "CS"))?;
    palette_from_space(doc, &cs)
}

/// The lookup table of an `Indexed` colour space, whatever names it.
fn palette_from_space(doc: &Document, cs: &Object) -> Option<Palette> {
    let arr = cs.as_array()?;
    let head = arr.first().and_then(|o| o.as_name())?;
    if head != "Indexed" && head != "I" {
        return None;
    }
    if arr.len() < 4 {
        return None;
    }
    let base_components = color_space_components(doc, &doc.resolve(&arr[1]));
    let lookup = doc.resolve(&arr[3]);
    let data = match lookup {
        Object::Str(b) => b,
        Object::Stream(sd, raw) => decode_stream(&sd, &raw).ok()?,
        _ => return None,
    };
    Some(Palette {
        base_components,
        data,
    })
}

/// Parse CCITTFaxDecode parameters from an image dict, if it uses CCITT.
fn extract_ccitt(doc: &Document, d: &Dict) -> Option<CcittParams> {
    if !filter_names(d).contains("CCITTFaxDecode") {
        return None;
    }
    let width = doc.get(d, "Width").and_then(|o| o.as_int()).unwrap_or(1728) as u32;
    let height = doc.get(d, "Height").and_then(|o| o.as_int()).unwrap_or(0) as u32;
    let parms = decode_parms(doc, d);
    let g = |key: &str| parms.as_ref().and_then(|p| doc.get(p, key));
    Some(CcittParams {
        k: g("K").and_then(|o| o.as_int()).unwrap_or(0),
        columns: g("Columns")
            .and_then(|o| o.as_int())
            .map(|n| n as u32)
            .unwrap_or(width),
        rows: g("Rows")
            .and_then(|o| o.as_int())
            .map(|n| n as u32)
            .unwrap_or(height),
        black_is_1: matches!(g("BlackIs1"), Some(Object::Bool(true))),
    })
}

/// The `/DecodeParms` dictionary, whether it was written as one dict or as
/// an array parallel to a filter array.
fn decode_parms(doc: &Document, d: &Dict) -> Option<Dict> {
    match doc.get(d, "DecodeParms").or_else(|| doc.get(d, "DP")) {
        Some(Object::Dict(p)) => Some(p),
        Some(Object::Array(a)) => a.iter().find_map(|o| match doc.resolve(o) {
            Object::Dict(p) => Some(p),
            _ => None,
        }),
        _ => None,
    }
}

/// Decode a JBIG2 image to packed one-bit rows, in PDF's sense of the bits.
///
/// JBIG2 counts a set bit as black; a PDF image sample counts zero as black,
/// as every other filter's output does. The inversion here is what makes a
/// JBIG2 stencil behave like any other one-bit image downstream.
fn decode_jbig2(doc: &Document, d: &Dict, coded: &[u8], w: usize, h: usize) -> Option<Vec<u8>> {
    let globals = decode_parms(doc, d)
        .and_then(|p| match doc.get(&p, "JBIG2Globals") {
            Some(Object::Stream(gd, graw)) => decode_stream(&gd, &graw).ok(),
            _ => None,
        })
        .unwrap_or_default();
    let bitmap = crate::image::jbig2::decode_embedded(&globals, coded, w, h)?;
    let stride = w.div_ceil(8);
    let mut out = vec![0u8; stride * h];
    for y in 0..h {
        for x in 0..w {
            if bitmap.bits[y * w + x] == 0 {
                out[y * stride + x / 8] |= 0x80 >> (x % 8);
            }
        }
    }
    Some(out)
}

/// Apply an image's stencil `/Mask` as the alpha channel of `rgba`.
///
/// A stencil mask is a one-bit image where a set sample masks the pixel out.
/// Scanned pages lean on it: the sharp foreground layer is drawn over the
/// photographic background through a stencil of the text, and without the
/// stencil the foreground covers the page as an opaque rectangle.
fn apply_stencil_mask(doc: &Document, d: &Dict, rgba: &mut [u8], w: usize, h: usize) {
    // The colour-key form of `/Mask` is an array of sample ranges, not a
    // stencil, and is a separate feature.
    let Some(Object::Stream(md, mraw)) = doc.get(d, "Mask") else {
        return;
    };
    let mw = doc.get(&md, "Width").and_then(|o| o.as_int()).unwrap_or(0) as usize;
    let mh = doc.get(&md, "Height").and_then(|o| o.as_int()).unwrap_or(0) as usize;
    if mw == 0 || mh == 0 {
        return;
    }
    let Ok(decoded) = decode_stream(&md, &mraw) else {
        return;
    };
    let decoded = if filter_names(&md).contains("JBIG2Decode") {
        match decode_jbig2(doc, &md, &decoded, mw, mh) {
            Some(bits) => bits,
            None => return,
        }
    } else {
        decoded
    };
    let Some(samples) = unpack_samples(&decoded, mw, mh, 1) else {
        return;
    };
    // `/Decode [1 0]` swaps which sample value masks.
    let invert = matches!(doc.get(&md, "Decode"), Some(Object::Array(a))
        if a.first().and_then(|o| doc.resolve(o).as_f64()) == Some(1.0));
    for y in 0..h {
        let sy = y * mh / h;
        for x in 0..w {
            let sx = x * mw / w;
            let masked = (samples[sy * mw + sx] != 0) != invert;
            if masked {
                rgba[(y * w + x) * 4 + 3] = 0;
            }
        }
    }
}

/// Number of color components for a base color space object.
fn color_space_components(doc: &Document, cs: &Object) -> u8 {
    match cs {
        Object::Name(n) => match n.as_str() {
            "DeviceGray" | "CalGray" | "G" => 1,
            "DeviceCMYK" => 4,
            _ => 3, // DeviceRGB / CalRGB / Lab / default
        },
        Object::Array(a) => match a.first().and_then(|o| o.as_name()) {
            Some("ICCBased") => a
                .get(1)
                .map(|o| doc.resolve(o))
                .and_then(|s| {
                    s.as_dict()
                        .and_then(|sd| doc.get(sd, "N"))
                        .and_then(|o| o.as_int())
                })
                .unwrap_or(3) as u8,
            Some("CalGray") => 1,
            Some("DeviceN") => a
                .get(1)
                .and_then(|o| o.as_array())
                .map(|names| names.len() as u8)
                .unwrap_or(1),
            Some("Separation") => 1,
            _ => 3,
        },
        _ => 3,
    }
}

fn color_space_name(doc: &Document, d: &Dict) -> String {
    match doc.get(d, "ColorSpace").or_else(|| doc.get(d, "CS")) {
        Some(Object::Name(n)) => n,
        Some(Object::Array(a)) => a
            .first()
            .and_then(|o| o.as_name().map(String::from))
            .unwrap_or_else(|| "Array".into()),
        _ => "Unknown".into(),
    }
}

fn filter_names(d: &Dict) -> String {
    match d.get("Filter").or_else(|| d.get("F")) {
        Some(Object::Name(n)) => n.clone(),
        Some(Object::Array(a)) => a
            .iter()
            .filter_map(|o| o.as_name())
            .collect::<Vec<_>>()
            .join("+"),
        _ => "None".into(),
    }
}

#[derive(Default, Clone)]
struct Inherited {
    media_box: Option<[f64; 4]>,
    /// The crop box is inheritable too, and a print-ready document commonly
    /// states it once on the page tree.
    crop_box: Option<[f64; 4]>,
    resources: Option<Object>,
}

fn collect_pages(
    doc: &Document,
    node: &Dict,
    out: &mut Vec<Dict>,
    inherited: &Inherited,
    depth: usize,
    visited: &mut std::collections::HashSet<usize>,
) {
    if depth > MAX_DEPTH || out.len() > 200_000 {
        return;
    }
    // Compute inheritance for this node.
    let mut inh = inherited.clone();
    if let Some(mb) = rect4(doc, node, "MediaBox") {
        inh.media_box = Some(mb);
    }
    if let Some(cb) = rect4(doc, node, "CropBox") {
        inh.crop_box = Some(cb);
    }
    if let Some(res) = node.get("Resources") {
        inh.resources = Some(res.clone());
    }

    let node_type = node.get("Type").and_then(|o| o.as_name());
    if node_type == Some("Page") || (node_type.is_none() && node.get("Kids").is_none()) {
        // Leaf page: fold inherited attributes in if missing.
        let mut page = node.clone();
        if !page.contains_key("MediaBox")
            && let Some(mb) = inh.media_box
        {
            page.insert(
                "MediaBox".into(),
                Object::Array(mb.iter().map(|v| Object::Real(*v)).collect()),
            );
        }
        if !page.contains_key("CropBox")
            && let Some(cb) = inh.crop_box
        {
            page.insert(
                "CropBox".into(),
                Object::Array(cb.iter().map(|v| Object::Real(*v)).collect()),
            );
        }
        if !page.contains_key("Resources")
            && let Some(res) = inh.resources.clone()
        {
            page.insert("Resources".into(), res);
        }
        out.push(page);
        return;
    }

    if let Some(kids_obj) = doc.get(node, "Kids")
        && let Some(kids) = kids_obj.as_array()
    {
        for kid in kids {
            if let Object::Ref(n, _) = kid
                && !visited.insert(*n as usize)
            {
                continue;
            }
            let kid_resolved = doc.resolve(kid);
            if let Some(kd) = kid_resolved.as_dict() {
                collect_pages(doc, kd, out, &inh, depth + 1, visited);
            }
        }
    }
}

/// A six-number array: a transformation matrix.
fn rect6(doc: &Document, dict: &Dict, key: &str) -> Option<Matrix> {
    let arr = doc.get(dict, key)?;
    let a = arr.as_array()?;
    if a.len() < 6 {
        return None;
    }
    let mut out = [0.0f64; 6];
    for (slot, value) in out.iter_mut().zip(a.iter()) {
        *slot = doc.resolve(value).as_f64()?;
    }
    Some(out)
}

fn rect4(doc: &Document, dict: &Dict, key: &str) -> Option<[f64; 4]> {
    let arr = doc.get(dict, key)?;
    let a = arr.as_array()?;
    if a.len() < 4 {
        return None;
    }
    Some([
        doc.resolve(&a[0]).as_f64()?,
        doc.resolve(&a[1]).as_f64()?,
        doc.resolve(&a[2]).as_f64()?,
        doc.resolve(&a[3]).as_f64()?,
    ])
}

/// The rectangle a reader shows, as `[x0, y0, x1, y1]`.
///
/// The media box is the sheet the page is printed on; the crop box is the
/// part meant to be seen. A print-ready file carries registration marks, a
/// timestamp and trim corners out in the margin of the media box, and every
/// viewer -- PDF.js, Okular, Acrobat -- shows the crop box instead, so a
/// reader that shows the media box shows a different page from everyone else.
///
/// The crop box is clamped to the media box, as the specification requires,
/// and ignored if it is degenerate.
fn page_box(doc: &Document, page: &Dict) -> [f64; 4] {
    let media = rect4(doc, page, "MediaBox").unwrap_or([0.0, 0.0, 612.0, 792.0]);
    let media = [
        media[0].min(media[2]),
        media[1].min(media[3]),
        media[0].max(media[2]),
        media[1].max(media[3]),
    ];
    let Some(crop) = rect4(doc, page, "CropBox") else {
        return media;
    };
    let crop = [
        crop[0].min(crop[2]).max(media[0]),
        crop[1].min(crop[3]).max(media[1]),
        crop[0].max(crop[2]).min(media[2]),
        crop[1].max(crop[3]).min(media[3]),
    ];
    if crop[2] - crop[0] < 1.0 || crop[3] - crop[1] < 1.0 {
        return media;
    }
    crop
}

/// The size of the page a reader shows, in points.
fn view_size(doc: &Document, page: &Dict) -> (f64, f64) {
    let view = page_box(doc, page);
    (view[2] - view[0], view[3] - view[1])
}

/// Decode a PDF text string (handles UTF-16BE BOM and PDFDocEncoding-ish).
fn text_string(doc: &Document, dict: &Dict, key: &str) -> Option<String> {
    let o = doc.get(dict, key)?;
    let bytes = o.as_str_bytes()?;
    Some(decode_text_string(bytes))
}

fn decode_text_string(bytes: &[u8]) -> String {
    if bytes.len() >= 2 && bytes[0] == 0xFE && bytes[1] == 0xFF {
        // UTF-16BE
        let mut s = String::new();
        let mut i = 2;
        while i + 1 < bytes.len() {
            let u = u16::from_be_bytes([bytes[i], bytes[i + 1]]);
            s.push(char::from_u32(u as u32).unwrap_or('\u{FFFD}'));
            i += 2;
        }
        s
    } else {
        bytes.iter().map(|&b| b as char).collect()
    }
}

// ============================================================================
// Annotations & outline
// ============================================================================

fn collect_annotations(
    doc: &Document,
    page: &Dict,
    page_number: usize,
    out: &mut Vec<PdfAnnotation>,
) {
    let annots = match doc.get(page, "Annots") {
        Some(o) => o,
        None => return,
    };
    let arr = match annots.as_array() {
        Some(a) => a,
        None => return,
    };
    for a in arr {
        let ad = doc.resolve(a);
        let d = match ad.as_dict() {
            Some(d) => d,
            None => continue,
        };
        let subtype = doc
            .get(d, "Subtype")
            .and_then(|o| o.as_name().map(String::from))
            .unwrap_or_default();
        let rect = rect4(doc, d, "Rect").unwrap_or([0.0; 4]);
        let contents = text_string(doc, d, "Contents");
        let title = text_string(doc, d, "T");
        let mut uri = None;
        if let Some(action) = doc.get(d, "A")
            && let Some(ad2) = action.as_dict()
            && let Some(u) = doc.get(ad2, "URI")
        {
            uri = u.as_str_bytes().map(decode_text_string);
        }
        let dest = doc
            .get(d, "Dest")
            .and_then(|o| o.as_name().map(String::from));
        let color = doc.get(d, "C").and_then(|o| {
            o.as_array().and_then(|c| {
                if c.len() >= 3 {
                    Some([
                        c[0].as_f64().unwrap_or(0.0),
                        c[1].as_f64().unwrap_or(0.0),
                        c[2].as_f64().unwrap_or(0.0),
                    ])
                } else {
                    None
                }
            })
        });
        out.push(PdfAnnotation {
            subtype,
            page_number,
            rect,
            contents,
            uri,
            dest,
            title,
            color,
        });
    }
}

fn parse_outline(doc: &Document, root: &Dict) -> Vec<OutlineItem> {
    let outlines = match doc.get(root, "Outlines") {
        Some(o) => o,
        None => return Vec::new(),
    };
    let od = match outlines.as_dict() {
        Some(d) => d,
        None => return Vec::new(),
    };
    let first = match od.get("First").and_then(|o| o.as_ref()) {
        Some((n, _)) => n,
        None => return Vec::new(),
    };
    let mut visited = std::collections::HashSet::new();
    walk_outline(doc, first, 0, &mut visited)
}

fn walk_outline(
    doc: &Document,
    start: u32,
    depth: usize,
    visited: &mut std::collections::HashSet<u32>,
) -> Vec<OutlineItem> {
    let mut items = Vec::new();
    if depth > MAX_DEPTH {
        return items;
    }
    let mut cur = Some(start);
    while let Some(num) = cur {
        if !visited.insert(num) || items.len() > 50_000 {
            break;
        }
        let obj = match doc.fetch(num) {
            Ok(o) => o,
            Err(_) => break,
        };
        let d = match obj.as_dict() {
            Some(d) => d.clone(),
            None => break,
        };
        let title = text_string(doc, &d, "Title").unwrap_or_default();
        let dest = d.get("Dest").and_then(|o| o.as_name().map(String::from));
        let children = match d.get("First").and_then(|o| o.as_ref()) {
            Some((c, _)) => walk_outline(doc, c, depth + 1, visited),
            None => Vec::new(),
        };
        items.push(OutlineItem {
            title,
            page_number: None,
            dest,
            children,
        });
        cur = d.get("Next").and_then(|o| o.as_ref()).map(|(n, _)| n);
    }
    items
}

// ============================================================================
// Fonts & encoding
// ============================================================================

struct Font {
    /// Composite (Type0) fonts use multi-byte codes.
    two_byte: bool,
    /// code -> Unicode string (from ToUnicode CMap).
    to_unicode: HashMap<u32, String>,
    /// Simple-font single-byte code -> char (base encoding + Differences).
    simple: Option<Box<[Option<char>; 256]>>,
    /// Codespace ranges (low, high, byte-width) from an embedded /Encoding
    /// CMap. Empty for Identity-H (which is a fixed 2-byte codespace).
    codespace: Vec<(u32, u32, u8)>,
    /// True when the encoding is a Unicode predefined CMap (UniXXX-UCS2/UTF16),
    /// so a character code maps directly to a Unicode scalar value.
    cmap_unicode: bool,
    /// Glyph advance widths per code, in em fractions (PDF width / 1000).
    widths: HashMap<u32, f64>,
    /// Default advance (em fraction) for codes absent from `widths`.
    default_width: f64,
    /// Whether the PDF supplied explicit glyph widths (/Widths, /W, /DW, or
    /// /MissingWidth). When false, advances are a flat default and a renderer
    /// should measure real glyph widths instead.
    has_widths: bool,
    base_font: String,
    /// The object number the font dictionary was fetched from, so a renderer
    /// can tell two subsets sharing a `/BaseFont` apart.
    object: u32,
}

impl Font {
    /// Determine the next character code and its byte width from a composite
    /// font's byte string, using codespace ranges (or 2-byte Identity default).
    fn next_code(&self, bytes: &[u8]) -> (u32, usize) {
        if self.codespace.is_empty() {
            // Identity-H / unspecified: fixed 2-byte codes.
            if bytes.len() >= 2 {
                return (u16::from_be_bytes([bytes[0], bytes[1]]) as u32, 2);
            }
            return (bytes[0] as u32, 1);
        }
        // Try each width 1..=4, matching the leading bytes against a codespace
        // range of that width.
        for w in 1..=4usize.min(bytes.len()) {
            let mut val = 0u32;
            for &b in &bytes[..w] {
                val = (val << 8) | b as u32;
            }
            if self
                .codespace
                .iter()
                .any(|&(lo, hi, nb)| nb as usize == w && val >= lo && val <= hi)
            {
                return (val, w);
            }
        }
        // Fall back to the shortest codespace width, else 1.
        let w = self
            .codespace
            .iter()
            .map(|&(_, _, nb)| nb as usize)
            .min()
            .unwrap_or(1)
            .min(bytes.len())
            .max(1);
        let mut val = 0u32;
        for &b in &bytes[..w] {
            val = (val << 8) | b as u32;
        }
        (val, w)
    }

    fn decode(&self, bytes: &[u8], out: &mut String) {
        if self.two_byte {
            let mut i = 0;
            while i < bytes.len() {
                let (code, w) = self.next_code(&bytes[i..]);
                i += w.max(1);
                if let Some(s) = self.to_unicode.get(&code) {
                    out.push_str(s);
                } else if self.cmap_unicode {
                    // Unicode predefined CMap: the code IS a Unicode scalar.
                    if let Some(c) = char::from_u32(code)
                        && !c.is_control()
                    {
                        out.push(c);
                    }
                }
                // Otherwise (Identity-H / CID without ToUnicode) we cannot
                // recover Unicode without a CID→Unicode table, so we skip
                // rather than emit a garbage glyph-id character.
            }
        } else {
            for &b in bytes {
                let code = b as u32;
                if let Some(s) = self.to_unicode.get(&code) {
                    out.push_str(s);
                } else if let Some(tbl) = &self.simple {
                    if let Some(c) = tbl[b as usize] {
                        out.push(c);
                    }
                } else if let Some(c) = winansi(b) {
                    out.push(c);
                }
            }
        }
    }

    /// Like `decode`, but also records a per-output-character advance (em
    /// fraction) parallel to the pushed characters, so a renderer can place each
    /// glyph at its PDF cumulative width. A code that produces N characters
    /// splits its width evenly; a skipped code's width folds into the next
    /// emitted glyph so the total run width is preserved.
    fn decode_with_advances(
        &self,
        bytes: &[u8],
        out: &mut String,
        advs: &mut Vec<f64>,
        codes_out: &mut Vec<u32>,
    ) {
        let mut pending = 0.0f64;
        // A code that decodes to several characters (e.g. an "ﬁ" ligature) is one
        // cluster: the full code width goes on the first character and 0 on the
        // rest, so a renderer draws the cluster as a unit and advances once — the
        // same way the Canvas2D reference draws the whole code string.
        // The character code travels alongside, because a glyph is selected by
        // code, not by the Unicode the code happens to mean. Rendering from the
        // document's own font needs the code; text extraction needs the
        // Unicode; they are not the same thing and a ligature proves it.
        let mut emit = |s: &str,
                        w: f64,
                        code: u32,
                        out: &mut String,
                        advs: &mut Vec<f64>,
                        codes_out: &mut Vec<u32>| {
            if s.is_empty() {
                pending += w;
                return;
            }
            let total = w + pending;
            pending = 0.0;
            for (i, c) in s.chars().enumerate() {
                out.push(c);
                advs.push(if i == 0 { total } else { 0.0 });
                codes_out.push(code);
            }
        };
        let mut tmp = String::new();
        if self.two_byte {
            let mut i = 0;
            while i < bytes.len() {
                let (code, bw) = self.next_code(&bytes[i..]);
                i += bw.max(1);
                let w = self
                    .widths
                    .get(&code)
                    .copied()
                    .unwrap_or(self.default_width);
                tmp.clear();
                if let Some(s) = self.to_unicode.get(&code) {
                    tmp.push_str(s);
                } else if self.cmap_unicode
                    && let Some(c) = char::from_u32(code)
                    && !c.is_control()
                {
                    tmp.push(c);
                }
                // No placeholder here, unlike the simple-font case below. A
                // composite code that no `/ToUnicode` covers is usually a
                // document that means something other than what it says --
                // one in the corpus shows plain ASCII bytes through an
                // Identity-H font -- and holding the slot open only puts
                // wrong glyphs, or a fallback face, where nothing was.
                emit(&tmp, w, code, out, advs, codes_out);
            }
        } else {
            for &b in bytes {
                let code = b as u32;
                let w = self
                    .widths
                    .get(&code)
                    .copied()
                    .unwrap_or(self.default_width);
                tmp.clear();
                #[allow(clippy::collapsible_if)]
                if let Some(s) = self.to_unicode.get(&code) {
                    tmp.push_str(s);
                } else if let Some(tbl) = &self.simple {
                    if let Some(c) = tbl[b as usize] {
                        tmp.push(c);
                    }
                } else if let Some(c) = winansi(b) {
                    tmp.push(c);
                }
                if tmp.is_empty() {
                    // A code the document never says the meaning of still has
                    // a glyph in the font it came from, and a slot on the
                    // page. TeX's f-ligatures are the common case: no
                    // `/ToUnicode`, no `/Differences`, just code 12 and a
                    // charstring named "fi". Dropping the code dropped the
                    // ligature and folded its width into the previous letter,
                    // so "efficiently" came out as "e ciently" with a hole in
                    // it. The placeholder holds the slot; the renderer picks
                    // the glyph by code and draws the right thing.
                    tmp.push(char::REPLACEMENT_CHARACTER);
                }
                emit(&tmp, w, code, out, advs, codes_out);
            }
        }
        // Fold any trailing skipped width into the last glyph.
        if pending > 0.0
            && let Some(last) = advs.last_mut()
        {
            *last += pending;
        }
    }
}

/// The fonts a resource dictionary names. A form XObject brings its own.
fn build_fonts_in(doc: &Document, rd: &Dict) -> HashMap<String, Font> {
    let mut fonts = HashMap::new();
    let font_dict = match doc.get(rd, "Font") {
        Some(o) => o,
        None => return fonts,
    };
    let fd = match font_dict.as_dict() {
        Some(d) => d.clone(),
        None => return fonts,
    };
    for (name, fref) in fd.iter() {
        let fobj = doc.resolve(fref);
        if let Some(font) = fobj.as_dict() {
            let object = match fref {
                Object::Ref(number, _) => *number,
                _ => 0,
            };
            let mut built = build_one_font(doc, font);
            built.object = object;
            fonts.insert(name.clone(), built);
        }
    }
    fonts
}

fn build_one_font(doc: &Document, font: &Dict) -> Font {
    let subtype = doc
        .get(font, "Subtype")
        .and_then(|o| o.as_name().map(String::from))
        .unwrap_or_default();
    let base_font = doc
        .get(font, "BaseFont")
        .and_then(|o| o.as_name().map(String::from))
        .unwrap_or_default();
    let two_byte = subtype == "Type0";

    // ToUnicode CMap.
    let mut to_unicode = HashMap::new();
    if let Some(tu) = doc.get(font, "ToUnicode")
        && let Object::Stream(d, raw) = tu
        && let Ok(decoded) = decode_stream(&d, &raw)
    {
        parse_tounicode(&decoded, &mut to_unicode);
    }

    // Simple-font encoding table.
    let mut simple = None;
    if !two_byte {
        let mut table: Box<[Option<char>; 256]> = Box::new([None; 256]);
        // base
        let base_name = match doc.get(font, "Encoding") {
            Some(Object::Name(n)) => Some(n),
            Some(Object::Dict(ed)) => doc
                .get(&ed, "BaseEncoding")
                .and_then(|o| o.as_name().map(String::from)),
            _ => None,
        };
        for i in 0..256u32 {
            table[i as usize] = match base_name.as_deref() {
                Some("WinAnsiEncoding") | None => winansi(i as u8),
                Some("MacRomanEncoding") => macroman(i as u8),
                _ => winansi(i as u8),
            };
        }
        // /Differences
        if let Some(Object::Dict(ed)) = doc.get(font, "Encoding")
            && let Some(diffs) = doc.get(&ed, "Differences")
            && let Some(arr) = diffs.as_array()
        {
            let mut code = 0usize;
            for item in arr {
                match doc.resolve(item) {
                    Object::Int(n) => code = n as usize,
                    Object::Name(gname) => {
                        if code < 256 {
                            table[code] = glyph_name_to_char(&gname);
                        }
                        code += 1;
                    }
                    _ => {}
                }
            }
        }
        simple = Some(table);
    }

    // Composite-font codespace ranges from an embedded /Encoding CMap stream,
    // so codes of mixed byte widths tokenize correctly. Named encodings
    // (Identity-H/V and predefined CMaps) leave this empty → 2-byte default.
    let mut codespace = Vec::new();
    let mut cmap_unicode = false;
    if two_byte {
        match doc.get(font, "Encoding") {
            Some(Object::Stream(d, raw)) => {
                if let Ok(decoded) = decode_stream(&d, &raw) {
                    codespace = parse_codespace(&decoded);
                }
            }
            Some(Object::Name(n)) => {
                // Unicode predefined CMaps map a 2-byte code straight to a
                // Unicode scalar (UCS-2 / UTF-16 BMP), so CJK text decodes
                // without a ToUnicode map or CID→Unicode tables.
                cmap_unicode = n.starts_with("Uni") && (n.contains("UCS2") || n.contains("UTF16"));
            }
            _ => {}
        }
    }

    // Glyph advance widths (em fractions) for fitting rendered text to layout.
    // The encoding is passed in because a width fallback has to follow it: a
    // code is not a character, and a font that puts its "fi" ligature at code
    // 2 needs the ligature's width, not a control character's.
    let (widths, default_width, has_widths) =
        parse_font_widths(doc, font, two_byte, simple.as_deref());

    Font {
        object: 0,
        two_byte,
        to_unicode,
        simple,
        codespace,
        cmap_unicode,
        widths,
        default_width,
        has_widths,
        base_font,
    }
}

/// The standard-14 font whose published metrics a base font name implies.
///
/// A PDF may name Times, Helvetica or Courier without embedding anything and
/// without a `/Widths` array, because every reader is required to know their
/// metrics. This is where that knowledge comes from; the tables are the same
/// ones the typesetter measures with, so a document laid out here and a
/// document read from a file agree.
fn standard_14_metrics(base_font: &str) -> crate::typeset::fonts::Font {
    let lower = base_font.to_ascii_lowercase();
    crate::typeset::fonts::Font {
        family: crate::typeset::fonts::Family::from_name(base_font),
        bold: lower.contains("bold") || lower.contains("black") || lower.contains("heavy"),
        italic: lower.contains("italic") || lower.contains("oblique"),
    }
}

/// Parse glyph advance widths: simple fonts use /Widths + /FirstChar; Type0
/// uses the descendant font's /W array + /DW. Returns (code→em-fraction,
/// default, has_explicit_widths). When `has_explicit_widths` is false (e.g. a
/// base-14 font with no /Widths), the advances are a flat default and a renderer
/// should measure real glyph widths instead — matching the reference renderer.
fn parse_font_widths(
    doc: &Document,
    font: &Dict,
    two_byte: bool,
    simple: Option<&[Option<char>; 256]>,
) -> (HashMap<u32, f64>, f64, bool) {
    let mut widths = HashMap::new();
    if !two_byte {
        // A Type 3 font measures its glyphs in its own space, and `/FontMatrix`
        // is what converts that to text space. Dividing by 1000 as every other
        // simple font requires makes a Type 3 advance about ten times too
        // small, so a line of text claims a fraction of the width it occupies
        // and everything after it on that line is drawn through it.
        let scale = match doc
            .get(font, "Subtype")
            .and_then(|o| o.as_name().map(String::from))
            .as_deref()
        {
            Some("Type3") => doc
                .get(font, "FontMatrix")
                .and_then(|o| o.as_array().and_then(|a| a.first().cloned()))
                .and_then(|o| doc.resolve(&o).as_f64())
                .filter(|value| *value > 0.0)
                .unwrap_or(0.001),
            _ => 0.001,
        };
        let first = doc
            .get(font, "FirstChar")
            .and_then(|o| o.as_int())
            .unwrap_or(0) as u32;
        let widths_arr = doc
            .get(font, "Widths")
            .and_then(|o| o.as_array().map(|a| a.to_vec()));
        if let Some(arr) = &widths_arr {
            for (i, wo) in arr.iter().enumerate() {
                if let Some(w) = doc.resolve(wo).as_f64() {
                    widths.insert(first + i as u32, w * scale);
                }
            }
        }
        let missing = doc
            .get(font, "FontDescriptor")
            .and_then(|o| o.as_dict().cloned())
            .and_then(|fd| doc.get(&fd, "MissingWidth"));
        let has = widths_arr.map(|a| !a.is_empty()).unwrap_or(false) || missing.is_some();
        let default_width = missing.and_then(|o| o.as_f64()).unwrap_or(500.0) * scale;
        if has {
            return (widths, default_width, has);
        }

        // No `/Widths`, which a standard-14 font is entitled to omit: every
        // reader is expected to know its metrics. Falling back to a flat half
        // em makes every run too wide -- Times' `i` is 0.278 em, not 0.5 --
        // so a line overruns the position the document explicitly sets for
        // what follows, and text overlaps. Use the published metrics instead.
        let base = doc
            .get(font, "BaseFont")
            .and_then(|o| o.as_name().map(String::from))
            .unwrap_or_default();
        let metrics = standard_14_metrics(&base);
        for code in 0u32..=255 {
            // The character the code stands for under this font's encoding.
            // Reading the code as a character instead gives a control
            // character zero width, which then reads as a cluster
            // continuation and the glyph is never drawn at all.
            let ch = simple
                .and_then(|table| table[code as usize])
                .or_else(|| char::from_u32(code));
            if let Some(ch) = ch {
                widths.insert(code, metrics.char_width(ch));
            }
        }
        // The widths are now real, so a renderer should place by them rather
        // than measure a substitute face.
        return (widths, metrics.char_width(' '), true);
    }

    // Type0: descendant CIDFont carries /W and /DW.
    let desc = doc
        .get(font, "DescendantFonts")
        .and_then(|o| o.as_array().and_then(|a| a.first().map(|x| doc.resolve(x))))
        .and_then(|o| o.as_dict().cloned())
        .unwrap_or_default();
    let dw_obj = doc.get(&desc, "DW");
    let dw = dw_obj.as_ref().and_then(|o| o.as_f64()).unwrap_or(1000.0) / 1000.0;
    let w_arr = doc
        .get(&desc, "W")
        .and_then(|o| o.as_array().map(|a| a.to_vec()));
    if let Some(w) = &w_arr {
        let mut i = 0;
        while i + 1 < w.len() {
            let c = match doc.resolve(&w[i]).as_int() {
                Some(c) => c as u32,
                None => break,
            };
            match doc.resolve(&w[i + 1]) {
                // c [w_1 w_2 ...] : consecutive CIDs starting at c.
                Object::Array(list) => {
                    for (j, wo) in list.iter().enumerate() {
                        if let Some(width) = doc.resolve(wo).as_f64() {
                            widths.insert(c + j as u32, width / 1000.0);
                        }
                    }
                    i += 2;
                }
                // c_first c_last w : a CID range with one width.
                _ => {
                    if i + 2 >= w.len() {
                        break;
                    }
                    let c2 = doc.resolve(&w[i + 1]).as_int().unwrap_or(c as i64) as u32;
                    let width = doc.resolve(&w[i + 2]).as_f64().unwrap_or(0.0) / 1000.0;
                    for cid in c..=c2.min(c.saturating_add(65535)) {
                        widths.insert(cid, width);
                    }
                    i += 3;
                }
            }
        }
    }
    let has = w_arr.map(|a| !a.is_empty()).unwrap_or(false) || dw_obj.is_some();
    (widths, dw, has)
}

/// Parse `begincodespacerange`/`endcodespacerange` from a CMap, returning
/// (low, high, byte-width) tuples.
fn parse_codespace(data: &[u8]) -> Vec<(u32, u32, u8)> {
    let text = String::from_utf8_lossy(data);
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(rel) = find_sub(&bytes[from..], b"begincodespacerange") {
        let start = from + rel + b"begincodespacerange".len();
        let end = find_sub(&bytes[start..], b"endcodespacerange")
            .map(|p| start + p)
            .unwrap_or(bytes.len());
        for line in text[start..end].lines() {
            let toks = tokenize_cmap(line);
            if toks.len() >= 2 {
                let lo_hex = toks[0].trim_start_matches('<').trim_end_matches('>');
                let width = (lo_hex.len() / 2).clamp(1, 4) as u8;
                if let (Some(lo), Some(hi)) = (hex_to_u32(&toks[0]), hex_to_u32(&toks[1])) {
                    out.push((lo, hi, width));
                }
            }
        }
        from = end + b"endcodespacerange".len();
        if from >= bytes.len() {
            break;
        }
    }
    out
}

/// Parse a ToUnicode CMap: `bfchar` and `bfrange` sections.
///
/// Read as a token stream rather than line by line. Producers pack several
/// pairs onto one line, and reading one pair per line silently dropped the
/// rest: a document whose subset font packs two letters into each two-byte
/// code lost most of its map, and with it whole runs of text -- a page title
/// and half a footer, gone from the render and from extraction alike, with no
/// error anywhere.
fn parse_tounicode(data: &[u8], map: &mut HashMap<u32, String>) {
    let text = String::from_utf8_lossy(data);
    let bytes = text.as_bytes();

    // `<src> <dst>` pairs, however they are laid out.
    let mut from = 0;
    while let Some(rel) = find_sub(&bytes[from..], b"beginbfchar") {
        let start = from + rel + b"beginbfchar".len();
        let end = find_sub(&bytes[start..], b"endbfchar")
            .map(|p| start + p)
            .unwrap_or(bytes.len());
        let toks = tokenize_cmap(&text[start..end]);
        for pair in toks.chunks(2) {
            if let [src, dst] = pair
                && let Some(code) = hex_to_u32(src)
            {
                map.insert(code, hex_to_string(dst));
            }
        }
        from = end + b"endbfchar".len();
        if from >= bytes.len() {
            break;
        }
    }

    // `<lo> <hi> <dst>`, or `<lo> <hi> [ <d0> <d1> ... ]` naming each code.
    let mut from = 0;
    while let Some(rel) = find_sub(&bytes[from..], b"beginbfrange") {
        let start = from + rel + b"beginbfrange".len();
        let end = find_sub(&bytes[start..], b"endbfrange")
            .map(|p| start + p)
            .unwrap_or(bytes.len());
        let toks = tokenize_cmap(&text[start..end]);
        let mut i = 0;
        while i + 2 < toks.len() {
            let (Some(lo), Some(hi)) = (hex_to_u32(&toks[i]), hex_to_u32(&toks[i + 1])) else {
                i += 1;
                continue;
            };
            if hi < lo || hi - lo > 65_535 {
                i += 3;
                continue;
            }
            if toks[i + 2] == "[" {
                let mut code = lo;
                let mut j = i + 3;
                while j < toks.len() && toks[j] != "]" {
                    if code <= hi {
                        map.insert(code, hex_to_string(&toks[j]));
                    }
                    code += 1;
                    j += 1;
                }
                i = j + 1;
            } else {
                let base = hex_to_string(&toks[i + 2]);
                for k in 0..=(hi - lo) {
                    // Consecutive codes take consecutive destinations, and it
                    // is the last unit that counts up: a range mapping to
                    // "ffi" walks the third letter, not the first.
                    let mut chars: Vec<char> = base.chars().collect();
                    if let Some(last) = chars.last_mut()
                        && let Some(next) = char::from_u32(*last as u32 + k)
                    {
                        *last = next;
                    } else {
                        break;
                    }
                    map.insert(lo + k, chars.into_iter().collect());
                }
                i += 3;
            }
        }
        from = end + b"endbfrange".len();
        if from >= bytes.len() {
            break;
        }
    }
}

#[cfg(test)]
mod tounicode {
    use std::collections::HashMap;

    /// Producers pack several pairs onto one line. Reading a pair per line
    /// kept the first and silently dropped the rest, which deletes text from
    /// the page: one document's subset font packs two letters into each
    /// two-byte code, and losing most of the map lost a page title and half a
    /// footer, from the render and from extraction alike.
    #[test]
    fn every_pair_on_a_line_is_read() {
        let cmap = b"begincmap
4 beginbfchar
<0041> <0041> <0042> <0042> <414E> <0041004E> <5343> <00530043>
endbfchar
endcmap";
        let mut map = HashMap::new();
        super::parse_tounicode(cmap, &mut map);
        assert_eq!(map.get(&0x41).map(String::as_str), Some("A"));
        assert_eq!(map.get(&0x42).map(String::as_str), Some("B"));
        // A code standing for two letters at once.
        assert_eq!(map.get(&0x414E).map(String::as_str), Some("AN"));
        assert_eq!(map.get(&0x5343).map(String::as_str), Some("SC"));
    }

    /// A range names consecutive destinations from a base.
    #[test]
    fn a_range_counts_up_from_its_base() {
        let cmap = b"begincmap
1 beginbfrange
<0003> <0005> <0041>
endbfrange
endcmap";
        let mut map = HashMap::new();
        super::parse_tounicode(cmap, &mut map);
        assert_eq!(map.get(&3).map(String::as_str), Some("A"));
        assert_eq!(map.get(&4).map(String::as_str), Some("B"));
        assert_eq!(map.get(&5).map(String::as_str), Some("C"));
    }

    /// The array form names each code in the range separately, and was
    /// previously skipped outright.
    #[test]
    fn a_range_may_name_each_destination() {
        let cmap = b"begincmap
1 beginbfrange
<0010> <0012> [<0058> <0059005A> <005B>]
endbfrange
endcmap";
        let mut map = HashMap::new();
        super::parse_tounicode(cmap, &mut map);
        assert_eq!(map.get(&0x10).map(String::as_str), Some("X"));
        assert_eq!(map.get(&0x11).map(String::as_str), Some("YZ"));
        assert_eq!(map.get(&0x12).map(String::as_str), Some("["));
    }

    /// A malformed map must not hang or panic: these come from documents
    /// nobody vouched for.
    #[test]
    fn damage_is_survivable() {
        for cmap in [
            &b"beginbfchar <0041>"[..],
            &b"beginbfrange <0005> <0001> <0041> endbfrange"[..],
            &b"beginbfrange <0000> <FFFFFFFF> <0041> endbfrange"[..],
            &b"beginbfrange <0010> <0012> [ endbfrange"[..],
        ] {
            let mut map = HashMap::new();
            super::parse_tounicode(cmap, &mut map);
        }
    }
}

fn tokenize_cmap(line: &str) -> Vec<String> {
    let mut toks = Vec::new();
    let mut chars = line.chars().peekable();
    while let Some(&c) = chars.peek() {
        match c {
            '<' => {
                let mut t = String::new();
                chars.next();
                for ch in chars.by_ref() {
                    if ch == '>' {
                        break;
                    }
                    t.push(ch);
                }
                toks.push(format!("<{}>", t));
            }
            '[' => {
                toks.push("[".to_string());
                chars.next();
            }
            ']' => {
                toks.push("]".to_string());
                chars.next();
            }
            c if c.is_whitespace() => {
                chars.next();
            }
            _ => {
                let mut t = String::new();
                while let Some(&ch) = chars.peek() {
                    if ch.is_whitespace() || ch == '<' || ch == '[' || ch == ']' {
                        break;
                    }
                    t.push(ch);
                    chars.next();
                }
                if !t.is_empty() {
                    toks.push(t);
                }
            }
        }
    }
    toks
}

fn hex_to_u32(tok: &str) -> Option<u32> {
    let h = tok.trim_start_matches('<').trim_end_matches('>');
    if h.is_empty() {
        return None;
    }
    u32::from_str_radix(h, 16).ok()
}

/// Hex string of UTF-16BE code units -> Rust String.
fn hex_to_string(tok: &str) -> String {
    let h = tok.trim_start_matches('<').trim_end_matches('>');
    let mut bytes = Vec::new();
    let mut i = 0;
    let hb = h.as_bytes();
    while i + 1 < hb.len() {
        if let (Some(hi), Some(lo)) = (hex_val(hb[i]), hex_val(hb[i + 1])) {
            bytes.push(hi * 16 + lo);
        }
        i += 2;
    }
    // bytes are UTF-16BE
    let mut s = String::new();
    let mut j = 0;
    while j + 1 < bytes.len() {
        let u = u16::from_be_bytes([bytes[j], bytes[j + 1]]);
        s.push(char::from_u32(u as u32).unwrap_or('\u{FFFD}'));
        j += 2;
    }
    if bytes.len() == 1 {
        s.push(bytes[0] as char);
    }
    s
}

// ============================================================================
// Content stream text extraction
// ============================================================================

type Matrix = [f64; 6];

/// Invert an affine matrix, or `None` where it collapses a dimension.
///
/// Needed to ask where a device-space point lands in pattern space: a tiling
/// pattern is anchored to the page, so the first tile that can reach a fill is
/// found by mapping the fill's corner back through the pattern's own matrix.
fn mat_invert(m: &Matrix) -> Option<Matrix> {
    let det = m[0] * m[3] - m[1] * m[2];
    if det.abs() < 1e-12 {
        return None;
    }
    let (a, b, c, d, e, f) = (m[0], m[1], m[2], m[3], m[4], m[5]);
    Some([
        d / det,
        -b / det,
        -c / det,
        a / det,
        (c * f - d * e) / det,
        (b * e - a * f) / det,
    ])
}

fn mat_mul(a: &Matrix, b: &Matrix) -> Matrix {
    [
        a[0] * b[0] + a[1] * b[2],
        a[0] * b[1] + a[1] * b[3],
        a[2] * b[0] + a[3] * b[2],
        a[2] * b[1] + a[3] * b[3],
        a[4] * b[0] + a[5] * b[2] + b[4],
        a[4] * b[1] + a[5] * b[3] + b[5],
    ]
}

fn page_contents(doc: &Document, page: &Dict) -> Vec<u8> {
    let mut out = Vec::new();
    let contents = match doc.get(page, "Contents") {
        Some(o) => o,
        None => return out,
    };
    let push_stream = |obj: &Object, out: &mut Vec<u8>| {
        if let Object::Stream(d, raw) = obj
            && let Ok(dec) = decode_stream(d, raw)
        {
            out.extend_from_slice(&dec);
            out.push(b'\n');
        }
    };
    match &contents {
        Object::Stream(_, _) => push_stream(&contents, &mut out),
        Object::Array(arr) => {
            for item in arr {
                let resolved = doc.resolve(item);
                push_stream(&resolved, &mut out);
            }
        }
        _ => {}
    }
    out
}

/// Text fragments plus ruling-line geometry extracted from one page.
struct PageContent {
    text: Vec<TextBlock>,
    h_lines: Vec<Seg>,
    v_lines: Vec<Seg>,
    /// XObject placements: (resource name, device-space bbox [l,b,r,t]).
    placed: Vec<(String, [f64; 4])>,
}

/// An image XObject placed on a page, with its on-page bounding box.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlacedImage {
    pub page_number: usize,
    pub name: String,
    /// Bounding box [left, bottom, right, top] in PDF points.
    pub bbox: [f64; 4],
    pub width: u32,
    pub height: u32,
    pub color_space: String,
}

/// An axis-aligned line segment in device (page) space.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Seg {
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
}

/// Per-page ruling geometry, for table reconstruction and debugging.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageGraphics {
    pub page_number: usize,
    pub width: f64,
    pub height: f64,
    pub h_lines: Vec<Seg>,
    pub v_lines: Vec<Seg>,
}

fn transform_point(m: &Matrix, x: f64, y: f64) -> (f64, f64) {
    (m[0] * x + m[2] * y + m[4], m[1] * x + m[3] * y + m[5])
}

/// Classify collected path segments into horizontal / vertical rulings.
fn flush_path(path: &mut Vec<Seg>, h: &mut Vec<Seg>, v: &mut Vec<Seg>) {
    const TOL: f64 = 2.5;
    const MIN_LEN: f64 = 4.0;
    for seg in path.drain(..) {
        let dx = (seg.x1 - seg.x0).abs();
        let dy = (seg.y1 - seg.y0).abs();
        if dy <= TOL && dx >= MIN_LEN {
            let y = (seg.y0 + seg.y1) / 2.0;
            h.push(Seg {
                x0: seg.x0.min(seg.x1),
                y0: y,
                x1: seg.x0.max(seg.x1),
                y1: y,
            });
        } else if dx <= TOL && dy >= MIN_LEN {
            let x = (seg.x0 + seg.x1) / 2.0;
            v.push(Seg {
                x0: x,
                y0: seg.y0.min(seg.y1),
                x1: x,
                y1: seg.y0.max(seg.y1),
            });
        }
    }
}

/// Extract positioned text fragments and ruling lines from a page.
/// Text and rulings for one page, in the coordinates a reader shows.
///
/// The origin is the bottom-left of the crop box, not of the media box, so a
/// bounding box reported here names the same spot as the display list draws
/// it. On an imposed sheet the two differ by the trim margin, and a citation
/// that quotes media-box coordinates points off the page the reader sees.
fn extract_page_content(doc: &Document, page: &Dict, page_number: usize) -> PageContent {
    let content = page_contents(doc, page);
    let mut found = PageContent {
        text: Vec::new(),
        h_lines: Vec::new(),
        v_lines: Vec::new(),
        placed: Vec::new(),
    };
    if content.is_empty() {
        return found;
    }
    let resources = doc
        .get(page, "Resources")
        .and_then(|o| o.as_dict().cloned())
        .unwrap_or_default();
    let view = page_box(doc, page);
    read_content(
        doc,
        &content,
        &resources,
        [1.0, 0.0, 0.0, 1.0, -view[0], -view[1]],
        page_number,
        &mut found,
        0,
    );
    found
}

/// Read one content stream for text and rulings.
///
/// The companion to `run_content`: a page and a form XObject are the same kind
/// of thing, and text inside a form is text on the page. Without this a
/// document that wraps its content in a form -- which imposition and archiving
/// tools do routinely -- extracts as nothing at all.
fn read_content(
    doc: &Document,
    content: &[u8],
    resources: &Dict,
    start_ctm: Matrix,
    page_number: usize,
    found: &mut PageContent,
    depth: usize,
) {
    let fonts = build_fonts_in(doc, resources);

    let mut blocks = Vec::new();
    let mut h_lines: Vec<Seg> = Vec::new();
    let mut v_lines: Vec<Seg> = Vec::new();
    let mut placed: Vec<(String, [f64; 4])> = Vec::new();

    // Graphics + text state.
    // The font travels with the transform: `Tf` inside `q`/`Q` lasts only
    // until the `Q`, and text shown after it uses whatever was in force
    // before. Reading it with the wrong font yields gibberish or nothing.
    #[allow(clippy::type_complexity)]
    let mut ctm_stack: Vec<(Matrix, Option<&Font>, String, f64, f64)> = Vec::new();
    let mut ctm: Matrix = start_ctm;
    let mut tm: Matrix = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
    let mut tlm: Matrix = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
    let mut font_size = 0.0f64;
    let mut leading = 0.0f64;
    let mut cur_font: Option<&Font> = None;
    let mut cur_font_name = String::new();

    // Path-construction state (device space).
    let mut path: Vec<Seg> = Vec::new();
    let mut cur_pt = (0.0f64, 0.0f64);
    let mut start_pt = (0.0f64, 0.0f64);

    // Operand stack of content-stream tokens.
    let mut stack: Vec<Token> = Vec::new();

    let mut lex = ContentLexer::new(content);
    while let Some(tok) = lex.next_token() {
        match tok {
            Token::Op(op) => {
                match op.as_str() {
                    "q" => ctm_stack.push((
                        ctm,
                        cur_font,
                        cur_font_name.clone(),
                        font_size,
                        leading,
                    )),
                    "Q" => {
                        if let Some((m, font, name, size, lead)) = ctm_stack.pop() {
                            ctm = m;
                            cur_font = font;
                            cur_font_name = name;
                            font_size = size;
                            leading = lead;
                        }
                    }
                    "cm" => {
                        if let Some(m) = last6(&stack) {
                            ctm = mat_mul(&m, &ctm);
                        }
                    }
                    "m" => {
                        if let Some([x, y]) = last2(&stack) {
                            cur_pt = transform_point(&ctm, x, y);
                            start_pt = cur_pt;
                        }
                    }
                    "l" => {
                        if let Some([x, y]) = last2(&stack) {
                            let p = transform_point(&ctm, x, y);
                            path.push(Seg { x0: cur_pt.0, y0: cur_pt.1, x1: p.0, y1: p.1 });
                            cur_pt = p;
                        }
                    }
                    "re" => {
                        if let Some([x, y, w, hgt]) = last4(&stack) {
                            let c0 = transform_point(&ctm, x, y);
                            let c1 = transform_point(&ctm, x + w, y);
                            let c2 = transform_point(&ctm, x + w, y + hgt);
                            let c3 = transform_point(&ctm, x, y + hgt);
                            path.push(Seg { x0: c0.0, y0: c0.1, x1: c1.0, y1: c1.1 });
                            path.push(Seg { x0: c1.0, y0: c1.1, x1: c2.0, y1: c2.1 });
                            path.push(Seg { x0: c2.0, y0: c2.1, x1: c3.0, y1: c3.1 });
                            path.push(Seg { x0: c3.0, y0: c3.1, x1: c0.0, y1: c0.1 });
                            cur_pt = c0;
                            start_pt = c0;
                        }
                    }
                    "c" | "v" | "y" => {
                        // Bezier: only advance the current point to the endpoint.
                        if let Some([x, y]) = last2(&stack) {
                            cur_pt = transform_point(&ctm, x, y);
                        }
                    }
                    "h" => {
                        path.push(Seg { x0: cur_pt.0, y0: cur_pt.1, x1: start_pt.0, y1: start_pt.1 });
                        cur_pt = start_pt;
                    }
                    "S" | "F" | "f" | "f*" | "B" | "B*" => {
                        flush_path(&mut path, &mut h_lines, &mut v_lines);
                    }
                    "s" | "b" | "b*" => {
                        path.push(Seg { x0: cur_pt.0, y0: cur_pt.1, x1: start_pt.0, y1: start_pt.1 });
                        flush_path(&mut path, &mut h_lines, &mut v_lines);
                    }
                    "n" => {
                        path.clear();
                    }
                    "Do" => {
                        if let Some(Token::Name(n)) = stack.last() {
                            // A form holds text like any other content stream.
                            if let Some(form) = form_xobject(doc, resources, n)
                                && depth < MAX_FORM_DEPTH
                            {
                                let matrix = rect6(doc, &form.0, "Matrix")
                                    .unwrap_or([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
                                let inner_resources = doc
                                    .get(&form.0, "Resources")
                                    .and_then(|o| o.as_dict().cloned())
                                    .unwrap_or_else(|| resources.clone());
                                read_content(
                                    doc,
                                    &form.1,
                                    &inner_resources,
                                    mat_mul(&matrix, &ctm),
                                    page_number,
                                    found,
                                    depth + 1,
                                );
                                stack.clear();
                                continue;
                            }
                            // Image space is the unit square mapped by the CTM.
                            let c0 = transform_point(&ctm, 0.0, 0.0);
                            let c1 = transform_point(&ctm, 1.0, 0.0);
                            let c2 = transform_point(&ctm, 1.0, 1.0);
                            let c3 = transform_point(&ctm, 0.0, 1.0);
                            let xs = [c0.0, c1.0, c2.0, c3.0];
                            let ys = [c0.1, c1.1, c2.1, c3.1];
                            let left = xs.iter().cloned().fold(f64::INFINITY, f64::min);
                            let right = xs.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                            let bottom = ys.iter().cloned().fold(f64::INFINITY, f64::min);
                            let top = ys.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                            placed.push((n.clone(), [left, bottom, right, top]));
                        }
                    }
                    "BT" => {
                        tm = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
                        tlm = tm;
                    }
                    "ET" => {}
                    "Tf"
                        // /Name size Tf
                        if stack.len() >= 2 => {
                            if let Token::Num(sz) = stack[stack.len() - 1] {
                                font_size = sz;
                            }
                            if let Token::Name(n) = &stack[stack.len() - 2] {
                                cur_font = fonts.get(n);
                                cur_font_name = match cur_font {
                                    Some(f) if !f.base_font.is_empty() => f.base_font.clone(),
                                    _ => n.clone(),
                                };
                            }
                        }
                    "TL" => {
                        if let Some(Token::Num(v)) = stack.last() {
                            leading = *v;
                        }
                    }
                    "Td" => {
                        if let Some([x, y]) = last2(&stack) {
                            let t = [1.0, 0.0, 0.0, 1.0, x, y];
                            tlm = mat_mul(&t, &tlm);
                            tm = tlm;
                        }
                    }
                    "TD" => {
                        if let Some([x, y]) = last2(&stack) {
                            leading = -y;
                            let t = [1.0, 0.0, 0.0, 1.0, x, y];
                            tlm = mat_mul(&t, &tlm);
                            tm = tlm;
                        }
                    }
                    "Tm" => {
                        if let Some(m) = last6(&stack) {
                            tm = m;
                            tlm = m;
                        }
                    }
                    "T*" => {
                        let t = [1.0, 0.0, 0.0, 1.0, 0.0, -leading];
                        tlm = mat_mul(&t, &tlm);
                        tm = tlm;
                    }
                    "Tj" | "'" | "\"" => {
                        if op != "Tj" {
                            // ' and " imply T* first
                            let t = [1.0, 0.0, 0.0, 1.0, 0.0, -leading];
                            tlm = mat_mul(&t, &tlm);
                            tm = tlm;
                        }
                        if let Some(Token::Str(bytes)) = stack.last() {
                            emit_text(
                                bytes, cur_font, &cur_font_name, font_size, &tm, &ctm,
                                page_number, &mut blocks,
                            );
                        }
                    }
                    "TJ" => {
                        if let Some(Token::ArrStr(parts)) = stack.last() {
                            let mut combined = String::new();
                            for part in parts {
                                match part {
                                    ArrPart::Str(bytes) => {
                                        if let Some(f) = cur_font {
                                            f.decode(bytes, &mut combined);
                                        } else {
                                            for &b in bytes {
                                                if let Some(c) = winansi(b) {
                                                    combined.push(c);
                                                }
                                            }
                                        }
                                    }
                                    ArrPart::Num(adj) => {
                                        // Large negative adjustment => inter-word gap.
                                        if *adj < -120.0 && !combined.ends_with(' ') {
                                            combined.push(' ');
                                        }
                                    }
                                }
                            }
                            if !combined.trim().is_empty() {
                                push_block(&combined, &cur_font_name, font_size, &tm, &ctm, page_number, &mut blocks);
                            }
                        }
                    }
                    _ => {}
                }
                stack.clear();
            }
            other => {
                stack.push(other);
                if stack.len() > 64 {
                    stack.remove(0);
                }
            }
        }
    }

    found.text.append(&mut blocks);
    found.h_lines.append(&mut h_lines);
    found.v_lines.append(&mut v_lines);
    found.placed.append(&mut placed);
}

/// Extract image XObjects with their on-page placement bounding boxes.
pub fn extract_placed_images(data: &[u8]) -> Result<Vec<PlacedImage>, PdfError> {
    let doc = Document::parse(data)?;
    let root = doc
        .trailer
        .get("Root")
        .map(|o| doc.resolve(o))
        .and_then(|o| o.as_dict().cloned())
        .unwrap_or_default();
    let mut page_dicts: Vec<Dict> = Vec::new();
    if let Some(pages_obj) = doc.get(&root, "Pages")
        && let Some(pages_dict) = pages_obj.as_dict()
    {
        let mut visited = std::collections::HashSet::new();
        collect_pages(
            &doc,
            pages_dict,
            &mut page_dicts,
            &Inherited::default(),
            0,
            &mut visited,
        );
    }
    let mut out = Vec::new();
    for (idx, pd) in page_dicts.iter().enumerate() {
        // Map XObject resource name -> image dict (Image subtype only).
        let xobjects = doc
            .get(pd, "Resources")
            .and_then(|o| o.as_dict().cloned())
            .and_then(|r| doc.get(&r, "XObject"))
            .and_then(|o| o.as_dict().cloned())
            .unwrap_or_default();
        let content = extract_page_content(&doc, pd, idx + 1);
        for (name, bbox) in content.placed {
            let xobj = match xobjects.get(&name) {
                Some(o) => doc.resolve(o),
                None => continue,
            };
            let d = match &xobj {
                Object::Stream(d, _) => d,
                _ => continue,
            };
            if doc
                .get(d, "Subtype")
                .and_then(|o| o.as_name().map(String::from))
                .as_deref()
                != Some("Image")
            {
                continue;
            }
            out.push(PlacedImage {
                page_number: idx + 1,
                name,
                bbox,
                width: doc.get(d, "Width").and_then(|o| o.as_int()).unwrap_or(0) as u32,
                height: doc.get(d, "Height").and_then(|o| o.as_int()).unwrap_or(0) as u32,
                color_space: color_space_name(&doc, d),
            });
        }
    }
    Ok(out)
}

#[allow(clippy::too_many_arguments)]
fn emit_text(
    bytes: &[u8],
    font: Option<&Font>,
    font_name: &str,
    font_size: f64,
    tm: &Matrix,
    ctm: &Matrix,
    page_number: usize,
    blocks: &mut Vec<TextBlock>,
) {
    let mut s = String::new();
    if let Some(f) = font {
        f.decode(bytes, &mut s);
    } else {
        for &b in bytes {
            if let Some(c) = winansi(b) {
                s.push(c);
            }
        }
    }
    if s.trim().is_empty() {
        return;
    }
    push_block(&s, font_name, font_size, tm, ctm, page_number, blocks);
}

#[allow(clippy::too_many_arguments)]
fn push_block(
    text: &str,
    font_name: &str,
    font_size: f64,
    tm: &Matrix,
    ctm: &Matrix,
    page_number: usize,
    blocks: &mut Vec<TextBlock>,
) {
    let trm = mat_mul(tm, ctm);
    let x = trm[4];
    let y = trm[5];
    // Effective font size scales with the text/ctm vertical scale.
    let scale = (trm[0] * trm[3] - trm[1] * trm[2]).abs().sqrt();
    let eff_size = if font_size > 0.0 {
        font_size * scale.max(0.01)
    } else {
        scale.max(1.0)
    };
    let text = crate::text_norm::normalize_diacritics(text);
    blocks.push(TextBlock {
        text: text.clone(),
        x,
        y,
        width: text.chars().count() as f64 * eff_size * 0.5,
        height: eff_size,
        font_size: eff_size,
        font_name: font_name.to_string(),
        page_number,
    });
}

fn last2(stack: &[Token]) -> Option<[f64; 2]> {
    if stack.len() < 2 {
        return None;
    }
    let a = stack[stack.len() - 2].as_num()?;
    let b = stack[stack.len() - 1].as_num()?;
    Some([a, b])
}

fn last4(stack: &[Token]) -> Option<[f64; 4]> {
    if stack.len() < 4 {
        return None;
    }
    let n = stack.len();
    Some([
        stack[n - 4].as_num()?,
        stack[n - 3].as_num()?,
        stack[n - 2].as_num()?,
        stack[n - 1].as_num()?,
    ])
}

fn last6(stack: &[Token]) -> Option<Matrix> {
    if stack.len() < 6 {
        return None;
    }
    let n = stack.len();
    Some([
        stack[n - 6].as_num()?,
        stack[n - 5].as_num()?,
        stack[n - 4].as_num()?,
        stack[n - 3].as_num()?,
        stack[n - 2].as_num()?,
        stack[n - 1].as_num()?,
    ])
}

// ---- content-stream lexer --------------------------------------------------

#[derive(Debug, Clone)]
enum ArrPart {
    Str(Vec<u8>),
    Num(f64),
}

#[derive(Debug, Clone)]
enum Token {
    Num(f64),
    Str(Vec<u8>),
    /// The samples of an inline image, already stepped over by the lexer.
    /// Its dictionary arrives before it as ordinary name/value tokens.
    Inline(Vec<u8>),
    /// `true` or `false`. A keyword, never an operator: read as one it takes
    /// the operand-clearing path and an inline image's `/IM true` wipes the
    /// dictionary being accumulated in front of it.
    Bool(bool),
    Name(String),
    ArrStr(Vec<ArrPart>),
    Op(String),
}

impl Token {
    fn as_num(&self) -> Option<f64> {
        match self {
            Token::Num(n) => Some(*n),
            _ => None,
        }
    }
}

struct ContentLexer<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> ContentLexer<'a> {
    fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    fn next_token(&mut self) -> Option<Token> {
        loop {
            // skip ws & comments
            while self.pos < self.buf.len() {
                let b = self.buf[self.pos];
                if b == b'%' {
                    while self.pos < self.buf.len() && self.buf[self.pos] != b'\n' {
                        self.pos += 1;
                    }
                } else if Lexer::is_ws(b) {
                    self.pos += 1;
                } else {
                    break;
                }
            }
            if self.pos >= self.buf.len() {
                return None;
            }
            let b = self.buf[self.pos];
            match b {
                b'(' => return Some(Token::Str(self.read_literal())),
                b'<' => {
                    if self.buf.get(self.pos + 1) == Some(&b'<') {
                        // dict in content (e.g. BDC properties) — skip it
                        self.skip_dict();
                        continue;
                    } else {
                        return Some(Token::Str(self.read_hex()));
                    }
                }
                b'[' => return Some(self.read_tj_array()),
                b'/' => return Some(Token::Name(self.read_name())),
                b']' | b'>' | b'}' | b'{' | b')' => {
                    self.pos += 1;
                    continue;
                }
                b'0'..=b'9' | b'+' | b'-' | b'.' => {
                    if let Some(n) = self.read_number() {
                        return Some(Token::Num(n));
                    }
                    self.pos += 1;
                    continue;
                }
                _ => {
                    let op = self.read_op();
                    if op.is_empty() {
                        self.pos += 1;
                        continue;
                    }
                    // `ID` is followed by raw image bytes, not by operators.
                    // Lexing them is not merely useless: random binary is full
                    // of things that read as content. A `(` opens a literal
                    // string that swallows everything to the next unbalanced
                    // `)`, and the one- and two-letter operator names -- `f`,
                    // `re`, `q`, `Q`, `cm`, `rg`, `W`, `n`, `Do` -- turn up
                    // constantly, so a photograph can fill the page, clip away
                    // the text under it, or stroke a line as wide as the sheet.
                    if op == "ID" {
                        return Some(Token::Inline(self.take_inline_image_data()));
                    }
                    if op == "true" || op == "false" {
                        return Some(Token::Bool(op == "true"));
                    }
                    return Some(Token::Op(op));
                }
            }
        }
    }

    fn read_literal(&mut self) -> Vec<u8> {
        let mut lex = Lexer::new(self.buf, self.pos);
        let s = lex.parse_literal_string();
        self.pos = lex.pos;
        s
    }

    fn read_hex(&mut self) -> Vec<u8> {
        let mut lex = Lexer::new(self.buf, self.pos);
        let s = lex.parse_hex_string();
        self.pos = lex.pos;
        s
    }

    fn read_name(&mut self) -> String {
        let mut lex = Lexer::new(self.buf, self.pos);
        let n = match lex.parse_name() {
            Object::Name(n) => n,
            _ => String::new(),
        };
        self.pos = lex.pos;
        n
    }

    fn read_number(&mut self) -> Option<f64> {
        let start = self.pos;
        if self.buf[self.pos] == b'+' || self.buf[self.pos] == b'-' {
            self.pos += 1;
        }
        let mut seen = false;
        while self.pos < self.buf.len() {
            let b = self.buf[self.pos];
            if b.is_ascii_digit() || b == b'.' {
                self.pos += 1;
                seen = true;
            } else {
                break;
            }
        }
        if !seen {
            self.pos = start;
            return None;
        }
        std::str::from_utf8(&self.buf[start..self.pos])
            .ok()
            .and_then(|s| s.parse::<f64>().ok())
    }

    /// Take an inline image's samples, leaving `pos` after its `EI`.
    ///
    /// The data begins after exactly one whitespace byte and ends at an `EI`
    /// that stands as its own token. Nothing in the format says the samples
    /// cannot contain those two bytes, so this is a heuristic rather than a
    /// parse -- but it is the same one every reader uses, and being wrong here
    /// costs part of one image, where not skipping at all costs the page.
    fn take_inline_image_data(&mut self) -> Vec<u8> {
        if self.pos < self.buf.len() && Lexer::is_ws(self.buf[self.pos]) {
            self.pos += 1;
        }
        let start = self.pos;
        let mut at = self.pos;
        while at + 1 < self.buf.len() {
            let is_ei = self.buf[at] == b'E'
                && self.buf[at + 1] == b'I'
                && at > 0
                && Lexer::is_ws(self.buf[at - 1])
                && self
                    .buf
                    .get(at + 2)
                    .is_none_or(|&b| Lexer::is_ws(b) || Lexer::is_delim(b));
            if is_ei {
                self.pos = at + 2;
                // `at` is the `E`, and the whitespace before it delimits the
                // operator rather than belonging to the samples.
                return self.buf[start..at.saturating_sub(1)].to_vec();
            }
            at += 1;
        }
        // Unterminated: the rest of the stream is image data, not content.
        self.pos = self.buf.len();
        self.buf[start..].to_vec()
    }

    fn read_op(&mut self) -> String {
        let start = self.pos;
        while self.pos < self.buf.len() {
            let b = self.buf[self.pos];
            if Lexer::is_ws(b) || Lexer::is_delim(b) {
                break;
            }
            self.pos += 1;
        }
        String::from_utf8_lossy(&self.buf[start..self.pos]).to_string()
    }

    fn read_tj_array(&mut self) -> Token {
        self.pos += 1; // skip '['
        let mut parts = Vec::new();
        while self.pos < self.buf.len() {
            while self.pos < self.buf.len() && Lexer::is_ws(self.buf[self.pos]) {
                self.pos += 1;
            }
            if self.pos >= self.buf.len() {
                break;
            }
            let b = self.buf[self.pos];
            match b {
                b']' => {
                    self.pos += 1;
                    break;
                }
                b'(' => parts.push(ArrPart::Str(self.read_literal())),
                b'<' => parts.push(ArrPart::Str(self.read_hex())),
                b'0'..=b'9' | b'+' | b'-' | b'.' => {
                    if let Some(n) = self.read_number() {
                        parts.push(ArrPart::Num(n));
                    } else {
                        self.pos += 1;
                    }
                }
                _ => {
                    self.pos += 1;
                }
            }
        }
        Token::ArrStr(parts)
    }

    fn skip_dict(&mut self) {
        // skip a balanced << ... >>
        let mut depth = 0;
        while self.pos + 1 < self.buf.len() {
            if &self.buf[self.pos..self.pos + 2] == b"<<" {
                depth += 1;
                self.pos += 2;
            } else if &self.buf[self.pos..self.pos + 2] == b">>" {
                depth -= 1;
                self.pos += 2;
                if depth <= 0 {
                    break;
                }
            } else {
                self.pos += 1;
            }
        }
    }
}

// ============================================================================
// Encodings (WinAnsi / MacRoman) and glyph-name resolution
// ============================================================================

/// WinAnsiEncoding (CP1252) code -> char.
fn winansi(code: u8) -> Option<char> {
    let c = code;
    if (0x20..=0x7e).contains(&c) {
        return Some(c as char);
    }
    let ch = match c {
        0x80 => '\u{20AC}',
        0x82 => '\u{201A}',
        0x83 => '\u{0192}',
        0x84 => '\u{201E}',
        0x85 => '\u{2026}',
        0x86 => '\u{2020}',
        0x87 => '\u{2021}',
        0x88 => '\u{02C6}',
        0x89 => '\u{2030}',
        0x8A => '\u{0160}',
        0x8B => '\u{2039}',
        0x8C => '\u{0152}',
        0x8E => '\u{017D}',
        0x91 => '\u{2018}',
        0x92 => '\u{2019}',
        0x93 => '\u{201C}',
        0x94 => '\u{201D}',
        0x95 => '\u{2022}',
        0x96 => '\u{2013}',
        0x97 => '\u{2014}',
        0x98 => '\u{02DC}',
        0x99 => '\u{2122}',
        0x9A => '\u{0161}',
        0x9B => '\u{203A}',
        0x9C => '\u{0153}',
        0x9E => '\u{017E}',
        0x9F => '\u{0178}',
        0xA0..=0xFF => c as char, // Latin-1 supplement matches CP1252 here
        _ => return None,
    };
    Some(ch)
}

/// MacRomanEncoding — high range subset; ASCII identical to WinAnsi.
fn macroman(code: u8) -> Option<char> {
    if (0x20..=0x7e).contains(&code) {
        return Some(code as char);
    }
    // Minimal: fall back to latin-1 for the common accented range.
    if code >= 0xA0 {
        return char::from_u32(code as u32);
    }
    None
}

/// Resolve a PostScript glyph name to a character (AGL subset + uniXXXX).
fn glyph_name_to_char(name: &str) -> Option<char> {
    if let Some(hex) = name.strip_prefix("uni")
        && hex.len() >= 4
        && let Ok(v) = u32::from_str_radix(&hex[..4], 16)
    {
        return char::from_u32(v);
    }
    let c = match name {
        "space" => ' ',
        "exclam" => '!',
        "quotedbl" => '"',
        "numbersign" => '#',
        "dollar" => '$',
        "percent" => '%',
        "ampersand" => '&',
        "quotesingle" => '\'',
        "parenleft" => '(',
        "parenright" => ')',
        "asterisk" => '*',
        "plus" => '+',
        "comma" => ',',
        "hyphen" | "minus" => '-',
        "period" => '.',
        "slash" => '/',
        "zero" => '0',
        "one" => '1',
        "two" => '2',
        "three" => '3',
        "four" => '4',
        "five" => '5',
        "six" => '6',
        "seven" => '7',
        "eight" => '8',
        "nine" => '9',
        "colon" => ':',
        "semicolon" => ';',
        "less" => '<',
        "equal" => '=',
        "greater" => '>',
        "question" => '?',
        "at" => '@',
        "bracketleft" => '[',
        "backslash" => '\\',
        "bracketright" => ']',
        "underscore" => '_',
        "braceleft" => '{',
        "bar" => '|',
        "braceright" => '}',
        "quoteleft" => '\u{2018}',
        "quoteright" => '\u{2019}',
        "quotedblleft" => '\u{201C}',
        "quotedblright" => '\u{201D}',
        "bullet" => '\u{2022}',
        "endash" => '\u{2013}',
        "emdash" => '\u{2014}',
        "fi" => '\u{FB01}',
        "fl" => '\u{FB02}',
        "ellipsis" => '\u{2026}',
        _ => {
            // Single-letter glyph names map to themselves.
            if name.len() == 1 {
                name.chars().next().unwrap()
            } else {
                return None;
            }
        }
    };
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn winansi_basic() {
        assert_eq!(winansi(b'A'), Some('A'));
        assert_eq!(winansi(0x97), Some('\u{2014}'));
    }

    #[test]
    fn glyph_names() {
        assert_eq!(glyph_name_to_char("space"), Some(' '));
        assert_eq!(glyph_name_to_char("uni00E9"), Some('é'));
        assert_eq!(glyph_name_to_char("A"), Some('A'));
    }

    #[test]
    fn ascii85_roundtrip_simple() {
        // "Man " in ASCII85 is "9jqo^"
        let out = ascii85_decode(b"9jqo^~>");
        assert_eq!(&out[..4], b"Man ");
    }

    #[test]
    fn cmap_tokenize() {
        let toks = tokenize_cmap("<0041> <0041>");
        assert_eq!(toks, vec!["<0041>", "<0041>"]);
    }

    #[test]
    fn parse_object_dict() {
        let mut lex = Lexer::new(b"<< /Type /Page /Count 3 >>", 0);
        let obj = lex.parse_object(0).unwrap();
        let d = obj.as_dict().unwrap();
        assert_eq!(d.get("Type").and_then(|o| o.as_name()), Some("Page"));
        assert_eq!(d.get("Count").and_then(|o| o.as_int()), Some(3));
    }

    #[test]
    fn tagged_structure_tree() {
        // Minimal tagged PDF: Document > [H1 "Title" (p1), P (p1)].
        let pdf = b"%PDF-1.5\n\
            1 0 obj<</Type/Catalog/Pages 2 0 R/StructTreeRoot 5 0 R/MarkInfo<</Marked true>>>>endobj\n\
            2 0 obj<</Type/Pages/Kids[3 0 R]/Count 1>>endobj\n\
            3 0 obj<</Type/Page/Parent 2 0 R/MediaBox[0 0 612 792]>>endobj\n\
            5 0 obj<</Type/StructTreeRoot/K 6 0 R>>endobj\n\
            6 0 obj<</Type/StructElem/S/Document/P 5 0 R/K[7 0 R 8 0 R]>>endobj\n\
            7 0 obj<</Type/StructElem/S/H1/P 6 0 R/Pg 3 0 R/ActualText(Title)/K 0>>endobj\n\
            8 0 obj<</Type/StructElem/S/P/P 6 0 R/Pg 3 0 R/K 1>>endobj\n\
            startxref\n0\n%%EOF";
        let tree = extract_structure(pdf).unwrap();
        assert_eq!(tree.len(), 1);
        assert_eq!(tree[0].kind, "Document");
        assert_eq!(tree[0].children.len(), 2);
        assert_eq!(tree[0].children[0].kind, "H1");
        assert_eq!(tree[0].children[0].text.as_deref(), Some("Title"));
        assert_eq!(tree[0].children[0].page_number, Some(1));
        assert_eq!(tree[0].children[1].kind, "P");
    }

    #[test]
    fn acroform_fields() {
        // Minimal AcroForm: a text field "name" = "Jane", a checkbox "agree" = /Yes.
        let pdf = b"%PDF-1.5\n\
            1 0 obj<</Type/Catalog/Pages 2 0 R/AcroForm<</Fields[7 0 R 8 0 R]>>>>endobj\n\
            2 0 obj<</Type/Pages/Kids[3 0 R]/Count 1>>endobj\n\
            3 0 obj<</Type/Page/Parent 2 0 R/MediaBox[0 0 612 792]/Annots[7 0 R 8 0 R]>>endobj\n\
            7 0 obj<</FT/Tx/T(name)/V(Jane)/Rect[100 700 300 720]/P 3 0 R>>endobj\n\
            8 0 obj<</FT/Btn/T(agree)/V/Yes/Rect[100 660 120 680]/P 3 0 R/Ff 2>>endobj\n\
            startxref\n0\n%%EOF";
        let fields = extract_form_fields(pdf).unwrap();
        assert_eq!(fields.len(), 2);
        let name = fields.iter().find(|f| f.name == "name").unwrap();
        assert_eq!(name.field_type, "text");
        assert_eq!(name.value.as_deref(), Some("Jane"));
        assert_eq!(name.page_number, Some(1));
        let agree = fields.iter().find(|f| f.name == "agree").unwrap();
        assert_eq!(agree.field_type, "button");
        assert_eq!(agree.value.as_deref(), Some("Yes"));
        assert!(agree.required); // Ff bit 2
    }

    #[test]
    fn base64_encoder() {
        assert_eq!(b64e(b"Man"), "TWFu");
        assert_eq!(b64e(b"Ma"), "TWE=");
        assert_eq!(b64e(b""), "");
    }

    /// A text run says which font object it was set in.
    ///
    /// `/BaseFont` is not an identity: a document may embed two subsets of one
    /// typeface under the same name, and a renderer choosing between them by
    /// name draws whichever answers first. One such cover rendered as "qhe
    /// pingle-Board Computer e andbook" and still scored 0.010 against the
    /// reference, because a grid of ink cannot see which letter drew it.
    #[test]
    fn a_text_run_carries_the_font_object_it_was_set_in() {
        const PAGE: &str = "BT /F1 12 Tf 10 700 Td (Hi) Tj ET";
        let pdf = format!(
            concat!(
                "%PDF-1.5
",
                "1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj
",
                "2 0 obj<</Type/Pages/Kids[3 0 R]/Count 1>>endobj
",
                "3 0 obj<</Type/Page/Parent 2 0 R/MediaBox[0 0 612 792]/Contents 4 0 R",
                "/Resources<</Font<</F1 9 0 R>>>>>>endobj
",
                "4 0 obj<</Length {}>>stream
{}
endstream
endobj
",
                "9 0 obj<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>endobj
",
                "startxref
0
%%EOF"
            ),
            PAGE.len(),
            PAGE
        );
        let list = extract_display_list(pdf.as_bytes(), 1).expect("a display list");
        let face = list
            .ops
            .iter()
            .find_map(|op| match op {
                RenderOp::Text { face, .. } => Some(*face),
                _ => None,
            })
            .expect("the page shows text");
        assert_eq!(face, 9, "the run names the object its font came from");
    }

    /// A form XObject starts from the state it is drawn under.
    ///
    /// `ca 0` means nothing the form paints can mark the page. Running the
    /// form from the defaults instead puts its artwork on top of everything,
    /// which is how one white paper grew two opaque figures no other reader
    /// shows.
    #[test]
    fn a_form_is_drawn_in_the_state_it_was_reached_in() {
        const FORM: &str = "q 100 0 0 100 10 10 cm /Im Do Q";
        let make = |gs: &str| -> Vec<RenderOp> {
            let page = format!("q {gs} /Fm Do Q");
            let pdf = format!(
                concat!(
                    "%PDF-1.5
",
                    "1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj
",
                    "2 0 obj<</Type/Pages/Kids[3 0 R]/Count 1>>endobj
",
                    "3 0 obj<</Type/Page/Parent 2 0 R/MediaBox[0 0 200 200]/Contents 4 0 R",
                    "/Resources<</ExtGState<</G0 7 0 R>>/XObject<</Fm 5 0 R>>>>>>endobj
",
                    "4 0 obj<</Length {}>>stream
{}
endstream
endobj
",
                    "5 0 obj<</Type/XObject/Subtype/Form/BBox[0 0 200 200]",
                    "/Resources<</XObject<</Im 6 0 R>>>>/Length {}>>stream
{}
",
                    "endstream
endobj
",
                    "6 0 obj<</Type/XObject/Subtype/Image/Width 1/Height 1/ColorSpace",
                    "/DeviceRGB/BitsPerComponent 8/Length 3>>stream
RGB
endstream
endobj
",
                    "7 0 obj<</Type/ExtGState/ca 0>>endobj
",
                    "startxref
0
%%EOF"
                ),
                page.len(),
                page,
                FORM.len(),
                FORM
            );
            extract_display_list(pdf.as_bytes(), 1)
                .expect("a display list")
                .ops
        };
        let drawn = make("");
        assert!(
            drawn.iter().any(|op| matches!(op, RenderOp::Image { .. })),
            "without the state the form's image is drawn: {drawn:?}"
        );
        let hidden = make("/G0 gs");
        assert!(
            !hidden.iter().any(|op| matches!(op, RenderOp::Image { .. })),
            "under ca 0 nothing the form paints can mark the page: {hidden:?}"
        );
    }

    /// A partly transparent image is faded, not hidden and not opaque.
    ///
    /// A watermark is drawn exactly this way: one picture under `ca 0.15`,
    /// behind the text. Drawn opaque it is not a watermark, it is a photograph
    /// covering the page.
    #[test]
    fn an_image_carries_the_constant_alpha_it_was_drawn_under() {
        let alpha_of = |ca: &str| -> Option<f64> {
            const PAGE: &str = "q /G0 gs 100 0 0 100 10 10 cm /Im Do Q";
            let pdf = format!(
                concat!(
                    "%PDF-1.5
",
                    "1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj
",
                    "2 0 obj<</Type/Pages/Kids[3 0 R]/Count 1>>endobj
",
                    "3 0 obj<</Type/Page/Parent 2 0 R/MediaBox[0 0 200 200]/Contents 4 0 R",
                    "/Resources<</ExtGState<</G0 6 0 R>>/XObject<</Im 5 0 R>>>>>>endobj
",
                    "4 0 obj<</Length {}>>stream
{}
endstream
endobj
",
                    "5 0 obj<</Type/XObject/Subtype/Image/Width 1/Height 1/ColorSpace",
                    "/DeviceRGB/BitsPerComponent 8/Length 3>>stream
RGB
endstream
endobj
",
                    "6 0 obj<</Type/ExtGState/ca {}>>endobj
",
                    "startxref
0
%%EOF"
                ),
                PAGE.len(),
                PAGE,
                ca
            );
            extract_display_list(pdf.as_bytes(), 1)
                .expect("a display list")
                .ops
                .iter()
                .find_map(|op| match op {
                    RenderOp::Image { alpha, .. } => Some(*alpha),
                    _ => None,
                })
        };

        assert_eq!(alpha_of("1"), Some(1.0), "an opaque image is unchanged");
        assert_eq!(alpha_of("0.4"), Some(0.4), "the op carries ca");
        assert_eq!(
            alpha_of("0"),
            None,
            "at zero it cannot mark the page and is not emitted at all"
        );
    }

    /// A Coons patch mesh is drawn rather than declined.
    ///
    /// One infographic ebook draws its page background with tensor patches and
    /// sets its table of contents in *white* on top. Declining the mesh lost
    /// the background and thirty-four lines of text together, and the page
    /// scored 0.872 for losing both.
    #[test]
    fn a_patch_mesh_is_sliced_into_flat_quads() {
        // One patch covering a 100-point square: red along the top edge, blue
        // along the bottom. Coordinates are one byte each over a `/Decode` of
        // 0..100, so a byte of 0 is 0 and 255 is 100.
        let q = |v: f64| (v / 100.0 * 255.0).round() as u8;
        let mut mesh: Vec<u8> = vec![0]; // flag 0: a patch that shares no edge
        // Twelve boundary points clockwise from the top-left corner.
        let boundary: [(f64, f64); 12] = [
            (0.0, 100.0),
            (33.0, 100.0),
            (67.0, 100.0),
            (100.0, 100.0),
            (100.0, 67.0),
            (100.0, 33.0),
            (100.0, 0.0),
            (67.0, 0.0),
            (33.0, 0.0),
            (0.0, 0.0),
            (0.0, 33.0),
            (0.0, 67.0),
        ];
        for (x, y) in boundary {
            mesh.push(q(x));
            mesh.push(q(y));
        }
        // Corner colours, in the same order: red, red, blue, blue.
        for rgb in [[255u8, 0, 0], [255, 0, 0], [0, 0, 255], [0, 0, 255]] {
            mesh.extend_from_slice(&rgb);
        }

        const PAGE: &str = "q 0 0 100 100 re W n /Sh0 sh Q";
        let mut pdf: Vec<u8> = Vec::new();
        pdf.extend_from_slice(b"%PDF-1.5\n");
        pdf.extend_from_slice(b"1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj\n");
        pdf.extend_from_slice(b"2 0 obj<</Type/Pages/Kids[3 0 R]/Count 1>>endobj\n");
        pdf.extend_from_slice(
            b"3 0 obj<</Type/Page/Parent 2 0 R/MediaBox[0 0 100 100]/Contents 4 0 R\
/Resources<</Shading<</Sh0 5 0 R>>>>>>endobj\n",
        );
        pdf.extend_from_slice(
            format!(
                "4 0 obj<</Length {}>>stream\n{PAGE}\nendstream\nendobj\n",
                PAGE.len()
            )
            .as_bytes(),
        );
        pdf.extend_from_slice(
            format!(
                "5 0 obj<</ShadingType 6/ColorSpace/DeviceRGB/BitsPerCoordinate 8\
/BitsPerComponent 8/BitsPerFlag 8/Decode[0 100 0 100 0 1 0 1 0 1]/Length {}>>stream\n",
                mesh.len()
            )
            .as_bytes(),
        );
        pdf.extend_from_slice(&mesh);
        pdf.extend_from_slice(b"\nendstream\nendobj\nstartxref\n0\n%%EOF");

        let ops = extract_display_list(&pdf, 1).expect("a display list").ops;
        let fills: Vec<_> = ops
            .iter()
            .filter_map(|op| match op {
                RenderOp::Fill {
                    subpaths, color, ..
                } => Some((subpaths, *color)),
                _ => None,
            })
            .collect();
        assert!(
            fills.len() > 8,
            "a patch is diced into many quads, got {}: {ops:?}",
            fills.len()
        );

        // The mesh's own colours, not the black a declined shading leaves.
        let highest = fills
            .iter()
            .max_by(|a, b| {
                let ay = a.0.iter().flatten().map(|p| p[1]).fold(f64::MIN, f64::max);
                let by = b.0.iter().flatten().map(|p| p[1]).fold(f64::MIN, f64::max);
                ay.total_cmp(&by)
            })
            .expect("a topmost quad");
        let lowest = fills
            .iter()
            .min_by(|a, b| {
                let ay = a.0.iter().flatten().map(|p| p[1]).fold(f64::MAX, f64::min);
                let by = b.0.iter().flatten().map(|p| p[1]).fold(f64::MAX, f64::min);
                ay.total_cmp(&by)
            })
            .expect("a bottom quad");
        assert!(
            highest.1[0] > 0.6 && highest.1[2] < 0.4,
            "the top of the patch is its red corners, got {:?}",
            highest.1
        );
        assert!(
            lowest.1[2] > 0.6 && lowest.1[0] < 0.4,
            "the bottom of the patch is its blue corners, got {:?}",
            lowest.1
        );

        // And nothing is painted outside the patch's own square.
        for (subpaths, _) in &fills {
            for p in subpaths.iter().flatten() {
                assert!(
                    (-0.5..=100.5).contains(&p[0]) && (-0.5..=100.5).contains(&p[1]),
                    "a quad escaped the patch: {p:?}"
                );
            }
        }
    }

    /// A stencil carries the fill colour it was drawn in, and no picture does.
    ///
    /// An `/ImageMask` has no colour space and one bit a sample: what it
    /// paints is whatever `rg` last said. The texture cannot hold that -- the
    /// same stencil may be drawn twice on a page in two colours -- so the op
    /// does, and an ordinary image carries no tint at all.
    #[test]
    fn an_image_mask_carries_the_fill_colour_and_a_picture_does_not() {
        let tint_of = |extra: &str| -> Option<Option<[f64; 4]>> {
            const PAGE: &str = "q 1 0 0 rg 100 0 0 100 10 10 cm /Im Do Q";
            let pdf = format!(
                concat!(
                    "%PDF-1.5
",
                    "1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj
",
                    "2 0 obj<</Type/Pages/Kids[3 0 R]/Count 1>>endobj
",
                    "3 0 obj<</Type/Page/Parent 2 0 R/MediaBox[0 0 200 200]/Contents 4 0 R",
                    "/Resources<</XObject<</Im 5 0 R>>>>>>endobj
",
                    "4 0 obj<</Length {}>>stream
{}
endstream
endobj
",
                    "5 0 obj<</Type/XObject/Subtype/Image/Width 8/Height 1{}",
                    "/BitsPerComponent 1/Length 1>>stream
 
endstream
endobj
",
                    "startxref
0
%%EOF"
                ),
                PAGE.len(),
                PAGE,
                extra
            );
            extract_display_list(pdf.as_bytes(), 1)
                .expect("a display list")
                .ops
                .iter()
                .find_map(|op| match op {
                    RenderOp::Image { tint, .. } => Some(*tint),
                    _ => None,
                })
        };

        assert_eq!(
            tint_of("/ImageMask true"),
            Some(Some([1.0, 0.0, 0.0, 1.0])),
            "a stencil takes the fill colour that was in force"
        );
        assert_eq!(
            tint_of("/ColorSpace/DeviceGray"),
            Some(None),
            "an ordinary image has a colour of its own and needs no tint"
        );
    }

    /// A stencil's bits become coverage, and a zero is what marks the page.
    #[test]
    fn an_image_mask_decodes_to_coverage_rather_than_to_nothing() {
        // Eight samples in one byte: 1010_1010. A zero paints by default.
        let coverage = |extra: &str| -> Vec<u8> {
            let pdf = format!(
                concat!(
                    "%PDF-1.5
",
                    "1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj
",
                    "2 0 obj<</Type/Pages/Kids[3 0 R]/Count 1>>endobj
",
                    "3 0 obj<</Type/Page/Parent 2 0 R/MediaBox[0 0 200 200]/Contents 4 0 R",
                    "/Resources<</XObject<</Im 5 0 R>>>>>>endobj
",
                    "4 0 obj<</Length 2>>stream
q
endstream
endobj
",
                    "5 0 obj<</Type/XObject/Subtype/Image/Width 8/Height 1/ImageMask true{}",
                    "/BitsPerComponent 1/Length 1>>stream
ª
endstream
endobj
",
                    "startxref
0
%%EOF"
                ),
                extra
            );
            // Latin-1 so the 0xAA byte survives into the stream unchanged.
            let bytes: Vec<u8> = pdf.chars().map(|c| c as u8).collect();
            let textures = extract_page_textures(&bytes, 1).expect("textures");
            let mask = textures.first().expect("the stencil decodes");
            mask.rgba.as_chunks::<4>().0.iter().map(|p| p[3]).collect()
        };

        assert_eq!(
            coverage(""),
            vec![0, 255, 0, 255, 0, 255, 0, 255],
            "a zero sample paints and a one does not"
        );
        assert_eq!(
            coverage("/Decode[1 0]"),
            vec![255, 0, 255, 0, 255, 0, 255, 0],
            "/Decode [1 0] swaps which bit marks the page"
        );
    }

    /// An inline image is drawn, and its texture is found by the other walk.
    ///
    /// It has no resource name -- it *is* its bytes -- so the display list and
    /// the texture walk meet it separately and must agree on what to call it.
    /// They agree by hashing the samples, which also means two identical
    /// inline images cost one texture.
    #[test]
    fn an_inline_image_is_drawn_and_its_texture_is_found() {
        // Eight one-bit samples, `/D [1 0]` inverted, drawn as a stencil in
        // red over a 100-point square.
        const PAGE: &str = concat!(
            "1 0 0 rg q 80 0 0 40 10 10 cm ",
            "BI /W 8 /H 1 /IM true /BPC 1 /D [1 0] ID ª EI Q"
        );
        let pdf = format!(
            concat!(
                "%PDF-1.5
",
                "1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj
",
                "2 0 obj<</Type/Pages/Kids[3 0 R]/Count 1>>endobj
",
                "3 0 obj<</Type/Page/Parent 2 0 R/MediaBox[0 0 100 100]/Contents 4 0 R>>endobj
",
                "4 0 obj<</Length {}>>stream
{}
endstream
endobj
",
                "startxref
0
%%EOF"
            ),
            PAGE.len(),
            PAGE
        );
        let bytes: Vec<u8> = pdf.chars().map(|c| c as u8).collect();

        let ops = extract_display_list(&bytes, 1).expect("a display list").ops;
        let placed: Vec<_> = ops
            .iter()
            .filter_map(|op| match op {
                RenderOp::Image {
                    name, w, h, tint, ..
                } => Some((name.clone(), *w, *h, *tint)),
                _ => None,
            })
            .collect();
        assert_eq!(
            placed.len(),
            1,
            "the inline image should be placed: {ops:?}"
        );
        let (name, w, h, tint) = &placed[0];
        assert!(
            (*w - 80.0).abs() < 0.01 && (*h - 40.0).abs() < 0.01,
            "placed by the CTM"
        );
        assert_eq!(
            *tint,
            Some([1.0, 0.0, 0.0, 1.0]),
            "an inline stencil takes the fill colour, like any other"
        );

        let textures = extract_page_textures(&bytes, 1).expect("textures");
        let found = textures
            .iter()
            .find(|t| t.name == *name)
            .expect("the texture walk must reach the same image under the same name");
        assert_eq!((found.width, found.height), (8, 1));
        // `/D [1 0]` inverts, so the set bits of 0xAA are what paints.
        let coverage: Vec<u8> = found.rgba.as_chunks::<4>().0.iter().map(|p| p[3]).collect();
        assert_eq!(coverage, vec![255, 0, 255, 0, 255, 0, 255, 0]);
    }

    /// An inline image's bytes are samples, not operators.
    ///
    /// Random binary reads as content: `(` opens a string that runs to the
    /// next unbalanced `)`, and the short operator names turn up constantly.
    /// A payload that happens to spell `re f` paints a rectangle nobody asked
    /// for, at whatever numbers the neighbouring bytes decoded to.
    #[test]
    fn inline_image_samples_are_not_read_as_content() {
        // The payload spells out a full-page black fill and a clip. If it is
        // lexed, the page gains both; if it is skipped, the page has only the
        // small red rectangle drawn after the `EI`.
        const PAGE: &str = concat!(
            "BI /W 4 /H 1 /BPC 8 /CS /G ID ",
            "0 0 999 999 re f 0 0 999 999 re W n ",
            "EI 1 0 0 rg 10 10 20 20 re f"
        );
        let pdf = format!(
            concat!(
                "%PDF-1.5
",
                "1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj
",
                "2 0 obj<</Type/Pages/Kids[3 0 R]/Count 1>>endobj
",
                "3 0 obj<</Type/Page/Parent 2 0 R/MediaBox[0 0 200 200]/Contents 4 0 R>>endobj
",
                "4 0 obj<</Length {}>>stream
{}
endstream
endobj
",
                "startxref
0
%%EOF"
            ),
            PAGE.len(),
            PAGE
        );
        let ops = extract_display_list(pdf.as_bytes(), 1)
            .expect("a display list")
            .ops;

        let fills: Vec<_> = ops
            .iter()
            .filter_map(|op| match op {
                RenderOp::Fill { subpaths, .. } => Some(subpaths),
                _ => None,
            })
            .collect();
        assert_eq!(
            fills.len(),
            1,
            "only the fill after the EI is content: {ops:?}"
        );
        assert!(
            !ops.iter().any(|op| matches!(op, RenderOp::Clip { .. })),
            "the payload's clip must not narrow the page: {ops:?}"
        );
        // And the surviving fill is the small one, not the page-sized one.
        let widest = fills[0]
            .iter()
            .flatten()
            .map(|p| p[0])
            .fold(f64::NEG_INFINITY, f64::max);
        assert!(widest < 100.0, "the page-sized fill leaked through");
    }

    /// A pattern fill runs the pattern; it does not paint black.
    ///
    /// `scn` in a pattern space names a resource instead of giving numbers.
    /// Read as a colour there are no numbers to read, which lands on black --
    /// so four photographs on one cover came out as solid black rectangles
    /// inside their white frames.
    #[test]
    fn a_tiling_pattern_is_run_rather_than_read_as_a_colour() {
        // The tile paints a small red square; the page fills a box with it.
        const TILE: &str = "1 0 0 rg 0 0 20 20 re f";
        const PAGE: &str = "/Pattern cs /P0 scn 20 20 40 40 re f";
        let pdf = format!(
            concat!(
                "%PDF-1.5
",
                "1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj
",
                "2 0 obj<</Type/Pages/Kids[3 0 R]/Count 1>>endobj
",
                "3 0 obj<</Type/Page/Parent 2 0 R/MediaBox[0 0 100 100]/Contents 4 0 R",
                "/Resources<</Pattern<</P0 5 0 R>>>>>>endobj
",
                "4 0 obj<</Length {}>>stream
{}
endstream
endobj
",
                "5 0 obj<</Type/Pattern/PatternType 1/PaintType 1/TilingType 1",
                "/BBox[0 0 20 20]/XStep 20/YStep 20/Resources<<>>/Length {}>>stream
{}
",
                "endstream
endobj
startxref
0
%%EOF"
            ),
            PAGE.len(),
            PAGE,
            TILE.len(),
            TILE
        );
        let ops = extract_display_list(pdf.as_bytes(), 1)
            .expect("a display list")
            .ops;

        let fills: Vec<_> = ops
            .iter()
            .filter_map(|op| match op {
                RenderOp::Fill { color, .. } => Some(*color),
                _ => None,
            })
            .collect();
        assert!(
            !fills.is_empty(),
            "the pattern painted nothing at all: {ops:?}"
        );
        assert!(
            fills.iter().all(|c| c[0] > 0.9 && c[1] < 0.1),
            "the tile's own red is what should paint, not black: {fills:?}"
        );
        assert!(
            ops.iter().any(|op| matches!(op, RenderOp::Clip { .. })),
            "the pattern must be clipped to the path it filled: {ops:?}"
        );
    }

    /// A shading pattern is a dictionary, not a stream.
    ///
    /// A tiling pattern has content to run and so is a stream; a shading
    /// pattern's gradient is its whole definition. Requiring a stream rejects
    /// every shading pattern before its type is read, and an unresolved
    /// pattern paints nothing -- which took a masthead off one datasheet.
    #[test]
    fn a_shading_pattern_paints_its_gradient() {
        const PAGE: &str = "/Pattern cs /P0 scn 0 0 100 100 re f";
        let pdf = format!(
            concat!(
                "%PDF-1.5
",
                "1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj
",
                "2 0 obj<</Type/Pages/Kids[3 0 R]/Count 1>>endobj
",
                "3 0 obj<</Type/Page/Parent 2 0 R/MediaBox[0 0 100 100]/Contents 4 0 R",
                "/Resources<</Pattern<</P0 5 0 R>>>>>>endobj
",
                "4 0 obj<</Length {}>>stream
{}
endstream
endobj
",
                "5 0 obj<</PatternType 2/Shading<</ShadingType 2/ColorSpace/DeviceRGB",
                "/Coords[0 0 100 0]/Extend[true true]/Function 6 0 R>>>>endobj
",
                "6 0 obj<</FunctionType 2/Domain[0 1]/C0[1 0 0]/C1[0 0 1]/N 1>>endobj
",
                "startxref
0
%%EOF"
            ),
            PAGE.len(),
            PAGE
        );
        let ops = extract_display_list(pdf.as_bytes(), 1)
            .expect("a display list")
            .ops;
        let fills: Vec<_> = ops
            .iter()
            .filter_map(|op| match op {
                RenderOp::Fill { color, .. } => Some(*color),
                _ => None,
            })
            .collect();
        assert!(
            fills.len() > 4,
            "a gradient is many bands, not one fill: {ops:?}"
        );
        // Red at one end, blue at the other, and neither is the black a
        // nameless `scn` would have left.
        assert!(
            fills.iter().any(|c| c[0] > 0.8) && fills.iter().any(|c| c[2] > 0.8),
            "the gradient's own colours should appear: {fills:?}"
        );
    }

    /// Colour is graphics state too, and a form inherits it.
    #[test]
    fn a_form_inherits_the_colour_it_is_drawn_under() {
        const PAGE: &str = "1 0 0 rg /Fm Do";
        const FORM: &str = "10 10 m 100 10 l 100 100 l h f";
        let pdf = format!(
            concat!(
                "%PDF-1.5
",
                "1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj
",
                "2 0 obj<</Type/Pages/Kids[3 0 R]/Count 1>>endobj
",
                "3 0 obj<</Type/Page/Parent 2 0 R/MediaBox[0 0 200 200]/Contents 4 0 R",
                "/Resources<</XObject<</Fm 5 0 R>>>>>>endobj
",
                "4 0 obj<</Length {}>>stream
{}
endstream
endobj
",
                "5 0 obj<</Type/XObject/Subtype/Form/BBox[0 0 200 200]/Length {}>>",
                "stream
{}
endstream
endobj
",
                "startxref
0
%%EOF"
            ),
            PAGE.len(),
            PAGE,
            FORM.len(),
            FORM
        );
        let list = extract_display_list(pdf.as_bytes(), 1).expect("a display list");
        let fill = list
            .ops
            .iter()
            .find_map(|op| match op {
                RenderOp::Fill { color, .. } => Some(*color),
                _ => None,
            })
            .unwrap_or_else(|| panic!("the form fills a path: {:?}", list.ops));
        assert!(
            fill[0] > 0.9 && fill[1] < 0.1 && fill[2] < 0.1,
            "the form paints in the colour set outside it: {fill:?}"
        );
    }

    /// A stroke is as wide as the CTM in force when the path is *painted*.
    ///
    /// Setting the width and then scaling down is ordinary output from
    /// drawing tools, which express a hairline as a large number under a tiny
    /// matrix. Reading the width when `w` runs instead of when `S` runs makes
    /// such a stroke wider than the page: one real document asks for `9525 w`
    /// under a matrix scaled by 0.0000337, and painting that literally took
    /// over two minutes for a single page.
    #[test]
    fn a_stroke_is_scaled_by_the_matrix_in_force_when_it_is_painted() {
        let pdf: &[u8] = b"%PDF-1.5
            1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj
            2 0 obj<</Type/Pages/Kids[3 0 R]/Count 1>>endobj
            3 0 obj<</Type/Page/Parent 2 0 R/MediaBox[0 0 612 792]/Contents 4 0 R/Resources<<>>>>endobj
            4 0 obj<<>>stream
            100 w q 0.01 0 0 0.01 0 0 cm 0 0 m 1000 1000 l S Q
            0 0 m 10 10 l S
            endstream
endobj
            startxref
0
%%EOF";
        let list = extract_display_list(pdf, 1).expect("a display list");
        let widths: Vec<f64> = list
            .ops
            .iter()
            .filter_map(|op| match op {
                RenderOp::Stroke { width, .. } => Some(*width),
                _ => None,
            })
            .collect();
        assert_eq!(widths.len(), 2, "both paths are stroked: {widths:?}");
        // 100 user units under a matrix scaled by 0.01.
        assert!(
            (widths[0] - 1.0).abs() < 1e-6,
            "scaled by the painting matrix, not the setting one: {}",
            widths[0]
        );
        // And `w` set outside the q/Q still applies after it, unscaled.
        assert!(
            (widths[1] - 100.0).abs() < 1e-6,
            "the width outlives the q/Q it was set before: {}",
            widths[1]
        );
    }

    /// `w` is graphics state, so `q`/`Q` bounds it like everything else.
    #[test]
    fn a_line_width_set_inside_a_saved_state_does_not_outlive_it() {
        let pdf: &[u8] = b"%PDF-1.5
            1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj
            2 0 obj<</Type/Pages/Kids[3 0 R]/Count 1>>endobj
            3 0 obj<</Type/Page/Parent 2 0 R/MediaBox[0 0 612 792]/Contents 4 0 R/Resources<<>>>>endobj
            4 0 obj<<>>stream
            3 w q 40 w 0 0 m 10 10 l S Q 0 0 m 20 20 l S
            endstream
endobj
            startxref
0
%%EOF";
        let list = extract_display_list(pdf, 1).expect("a display list");
        let widths: Vec<f64> = list
            .ops
            .iter()
            .filter_map(|op| match op {
                RenderOp::Stroke { width, .. } => Some(*width),
                _ => None,
            })
            .collect();
        assert_eq!(widths.len(), 2, "{widths:?}");
        assert!((widths[0] - 40.0).abs() < 1e-6, "{}", widths[0]);
        assert!(
            (widths[1] - 3.0).abs() < 1e-6,
            "the Q restores the width the q saved: {}",
            widths[1]
        );
    }

    #[test]
    fn page_images_raw_rgb() {
        // A 2x1 DeviceRGB image (red, green) placed via `Do`, no filter.
        let pdf: &[u8] = b"%PDF-1.5\n\
            1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj\n\
            2 0 obj<</Type/Pages/Kids[3 0 R]/Count 1>>endobj\n\
            3 0 obj<</Type/Page/Parent 2 0 R/MediaBox[0 0 612 792]/Contents 4 0 R/Resources<</XObject<</Im0 6 0 R>>>>>>endobj\n\
            4 0 obj<<>>stream\nq 20 0 0 10 50 60 cm /Im0 Do Q\nendstream\nendobj\n\
            6 0 obj<</Type/XObject/Subtype/Image/Width 2/Height 1/ColorSpace/DeviceRGB/BitsPerComponent 8/Length 6>>stream\n\
\xff\x00\x00\x00\xff\x00\nendstream\nendobj\n\
            startxref\n0\n%%EOF";
        let imgs = extract_page_images(pdf, 1).unwrap();
        assert_eq!(imgs.len(), 1);
        let im = &imgs[0];
        assert_eq!(im.format, "rgba");
        assert_eq!((im.width, im.height), (2, 1));
        assert_eq!(im.name, "Im0");
        // Placement: cm 20 0 0 10 50 60 → bbox x 50..70, y 60..70.
        assert!((im.x - 50.0).abs() < 1.0 && (im.w - 20.0).abs() < 1.0);
        // RGBA bytes: red (255,0,0,255) then green (0,255,0,255).
        assert_eq!(b64e(&[255, 0, 0, 255, 0, 255, 0, 255]), im.data);
    }

    #[test]
    fn page_image_smask_alpha() {
        // 2x1 RGB image with a 2x1 grayscale SMask [255,0] -> alpha [255,0].
        let pdf: &[u8] = b"%PDF-1.5\n\
            1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj\n\
            2 0 obj<</Type/Pages/Kids[3 0 R]/Count 1>>endobj\n\
            3 0 obj<</Type/Page/Parent 2 0 R/MediaBox[0 0 612 792]/Contents 4 0 R/Resources<</XObject<</Im0 6 0 R>>>>>>endobj\n\
            4 0 obj<<>>stream\nq 20 0 0 10 50 60 cm /Im0 Do Q\nendstream\nendobj\n\
            6 0 obj<</Type/XObject/Subtype/Image/Width 2/Height 1/ColorSpace/DeviceRGB/BitsPerComponent 8/SMask 7 0 R/Length 6>>stream\n\
\xff\x00\x00\x00\xff\x00\nendstream\nendobj\n\
            7 0 obj<</Type/XObject/Subtype/Image/Width 2/Height 1/ColorSpace/DeviceGray/BitsPerComponent 8/Length 2>>stream\n\
\xff\x00\nendstream\nendobj\n\
            startxref\n0\n%%EOF";
        let imgs = extract_page_images(pdf, 1).unwrap();
        assert_eq!(imgs.len(), 1);
        // red pixel opaque, green pixel transparent (alpha from SMask).
        assert_eq!(b64e(&[255, 0, 0, 255, 0, 255, 0, 0]), imgs[0].data);
    }

    #[test]
    fn display_list_clip_emits_save_restore() {
        let pdf: &[u8] = b"%PDF-1.5\n\
            1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj\n\
            2 0 obj<</Type/Pages/Kids[3 0 R]/Count 1>>endobj\n\
            3 0 obj<</Type/Page/Parent 2 0 R/MediaBox[0 0 612 792]/Contents 4 0 R>>endobj\n\
            4 0 obj<<>>stream\nq 10 10 100 50 re W n 0 0 0 rg 20 20 40 20 re f Q\nendstream\nendobj\n\
            startxref\n0\n%%EOF";
        let dl = extract_display_list(pdf, 1).unwrap();
        assert!(matches!(dl.ops.first(), Some(RenderOp::Save)));
        assert!(matches!(dl.ops.last(), Some(RenderOp::Restore)));
        let clip = dl.ops.iter().find_map(|o| match o {
            RenderOp::Clip { rect, .. } => Some(*rect),
            _ => None,
        });
        assert_eq!(clip, Some([10.0, 10.0, 110.0, 60.0]));
    }

    #[test]
    fn display_list_fill_and_text() {
        let pdf = b"%PDF-1.5\n\
            1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj\n\
            2 0 obj<</Type/Pages/Kids[3 0 R]/Count 1>>endobj\n\
            3 0 obj<</Type/Page/Parent 2 0 R/MediaBox[0 0 612 792]/Contents 4 0 R/Resources<</Font<</F1 5 0 R>>>>>>endobj\n\
            4 0 obj<<>>stream\n\
1 0 0 rg\n100 100 50 40 re\nf\nBT /F1 12 Tf 100 200 Td (Hi) Tj ET\n\
endstream\nendobj\n\
            5 0 obj<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>endobj\n\
            startxref\n0\n%%EOF";
        let dl = extract_display_list(pdf, 1).unwrap();
        assert_eq!((dl.width, dl.height), (612.0, 792.0));
        let fill = dl.ops.iter().find_map(|o| match o {
            RenderOp::Fill {
                color, subpaths, ..
            } => Some((*color, subpaths.clone())),
            _ => None,
        });
        let (color, subpaths) = fill.expect("a fill op");
        assert!(
            (color[0] - 1.0).abs() < 1e-9 && color[1] < 1e-9 && color[2] < 1e-9,
            "red fill"
        );
        assert!(
            !subpaths.is_empty() && subpaths[0].len() >= 4,
            "rect polygon"
        );
        let text = dl.ops.iter().find_map(|o| match o {
            RenderOp::Text { text, x, y, .. } => Some((text.clone(), *x, *y)),
            _ => None,
        });
        let (t, x, y) = text.expect("a text op");
        assert_eq!(t, "Hi");
        assert!(
            (x - 100.0).abs() < 1.0 && (y - 200.0).abs() < 1.0,
            "baseline at (100,200)"
        );
    }

    #[test]
    fn no_acroform_empty() {
        let pdf = b"%PDF-1.4\n\
            1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj\n\
            2 0 obj<</Type/Pages/Kids[3 0 R]/Count 1>>endobj\n\
            3 0 obj<</Type/Page/Parent 2 0 R/MediaBox[0 0 612 792]>>endobj\n\
            startxref\n0\n%%EOF";
        assert!(extract_form_fields(pdf).unwrap().is_empty());
    }

    #[test]
    fn untagged_structure_empty() {
        let pdf = b"%PDF-1.4\n\
            1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj\n\
            2 0 obj<</Type/Pages/Kids[3 0 R]/Count 1>>endobj\n\
            3 0 obj<</Type/Page/Parent 2 0 R/MediaBox[0 0 612 792]>>endobj\n\
            startxref\n0\n%%EOF";
        assert!(extract_structure(pdf).unwrap().is_empty());
    }

    #[test]
    fn codespace_parsing() {
        let cmap = b"begincodespacerange\n<00> <80>\n<8140> <9ffc>\nendcodespacerange";
        let cs = parse_codespace(cmap);
        assert_eq!(cs, vec![(0x00, 0x80, 1), (0x8140, 0x9ffc, 2)]);
    }

    #[test]
    fn mixed_width_decode() {
        // Font with a 1-byte and a 2-byte codespace; ToUnicode maps both.
        let mut tu = HashMap::new();
        tu.insert(0x41u32, "A".to_string()); // 1-byte code 0x41
        tu.insert(0x8140u32, "—".to_string()); // 2-byte code
        let font = Font {
            object: 0,
            two_byte: true,
            to_unicode: tu,
            simple: None,
            codespace: vec![(0x00, 0x80, 1), (0x8140, 0x9ffc, 2)],
            cmap_unicode: false,
            widths: HashMap::new(),
            default_width: 0.5,
            has_widths: true,
            base_font: String::new(),
        };
        let mut out = String::new();
        font.decode(&[0x41, 0x81, 0x40, 0x41], &mut out);
        assert_eq!(out, "A—A");
    }

    #[test]
    fn identity_h_default_two_byte() {
        let mut tu = HashMap::new();
        tu.insert(0x0041u32, "A".to_string());
        let font = Font {
            object: 0,
            two_byte: true,
            to_unicode: tu,
            simple: None,
            codespace: vec![], // Identity-H
            cmap_unicode: false,
            widths: HashMap::new(),
            default_width: 0.5,
            has_widths: true,
            base_font: String::new(),
        };
        let mut out = String::new();
        font.decode(&[0x00, 0x41], &mut out);
        assert_eq!(out, "A");
    }

    #[test]
    fn unicode_cmap_decodes_without_tounicode() {
        // UniGB-UCS2-H style: 2-byte code is the Unicode scalar directly.
        let font = Font {
            object: 0,
            two_byte: true,
            to_unicode: HashMap::new(),
            simple: None,
            codespace: vec![],
            cmap_unicode: true,
            widths: HashMap::new(),
            default_width: 0.5,
            has_widths: true,
            base_font: String::new(),
        };
        let mut out = String::new();
        // U+4E2D 中, U+6587 文
        font.decode(&[0x4E, 0x2D, 0x65, 0x87], &mut out);
        assert_eq!(out, "中文");
    }

    #[test]
    fn identity_without_tounicode_skips_garbage() {
        // No ToUnicode, not a Unicode CMap → cannot recover Unicode, emit nothing
        // rather than a garbage glyph-id character.
        let font = Font {
            object: 0,
            two_byte: true,
            to_unicode: HashMap::new(),
            simple: None,
            codespace: vec![],
            cmap_unicode: false,
            widths: HashMap::new(),
            default_width: 0.5,
            has_widths: true,
            base_font: String::new(),
        };
        let mut out = String::new();
        font.decode(&[0x00, 0x03], &mut out);
        assert_eq!(out, "");
    }
}

#[cfg(test)]
mod type3_widths {
    /// A Type 3 font measures glyphs in its own space, and `/FontMatrix` is
    /// what converts that to text space. Dividing by 1000, as every other
    /// simple font requires, makes the advance about ten times too small: a
    /// line then claims a fraction of the width it occupies and the text after
    /// it is drawn straight through it.
    #[test]
    fn a_type3_advance_follows_the_font_matrix() {
        // `/Widths [50]` with `/FontMatrix [0.01 ...]` is half an em, not a
        // twentieth of one.
        let pdf = build(0.01, 50.0);
        let list = super::extract_display_list(&pdf, 1).expect("page 1");
        let run = list.ops.iter().find_map(|op| match op {
            super::RenderOp::Text { advances, size, .. } if !advances.is_empty() => {
                Some((advances.clone(), *size))
            }
            _ => None,
        });
        let Some((advances, size)) = run else {
            panic!("the page should draw text");
        };
        let em = advances[0] / size;
        assert!((em - 0.5).abs() < 0.01, "expected half an em, got {em:.3}");
    }

    /// The same font measured with a different matrix must scale with it, or
    /// the matrix is being ignored and a constant substituted.
    #[test]
    fn a_different_matrix_gives_a_different_advance() {
        let wide = build(0.02, 50.0);
        let narrow = build(0.005, 50.0);
        let advance = |pdf: &[u8]| -> f64 {
            let list = super::extract_display_list(pdf, 1).expect("page 1");
            list.ops
                .iter()
                .find_map(|op| match op {
                    super::RenderOp::Text { advances, size, .. } if !advances.is_empty() => {
                        Some(advances[0] / *size)
                    }
                    _ => None,
                })
                .expect("text")
        };
        let (wide, narrow) = (advance(&wide), advance(&narrow));
        assert!(
            (wide / narrow - 4.0).abs() < 0.05,
            "a matrix four times larger should give four times the advance:              {wide:.3} vs {narrow:.3}"
        );
    }

    /// A minimal document with one Type 3 font drawing one character.
    fn build(matrix: f64, width: f64) -> Vec<u8> {
        let mut pdf: Vec<u8> = Vec::new();
        let mut offsets = vec![0usize];
        pdf.extend_from_slice(
            b"%PDF-1.4
",
        );
        let push = |pdf: &mut Vec<u8>, offsets: &mut Vec<usize>, body: &[u8]| {
            offsets.push(pdf.len());
            let number = offsets.len() - 1;
            pdf.extend_from_slice(
                format!(
                    "{number} 0 obj
"
                )
                .as_bytes(),
            );
            pdf.extend_from_slice(body);
            pdf.extend_from_slice(
                b"
endobj
",
            );
        };

        push(&mut pdf, &mut offsets, b"<< /Type /Catalog /Pages 2 0 R >>");
        push(
            &mut pdf,
            &mut offsets,
            b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        );
        push(
            &mut pdf,
            &mut offsets,
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200]               /Resources << /Font << /F1 4 0 R >> >> /Contents 6 0 R >>",
        );
        push(
            &mut pdf,
            &mut offsets,
            format!(
                "<< /Type /Font /Subtype /Type3 /FontBBox [0 0 100 100]                  /FontMatrix [{matrix} 0 0 {matrix} 0 0] /CharProcs 5 0 R                  /Encoding << /Type /Encoding /Differences [65 /A] >>                  /FirstChar 65 /LastChar 65 /Widths [{width}] >>"
            )
            .as_bytes(),
        );
        push(&mut pdf, &mut offsets, b"<< >>");

        let content = "BT /F1 10 Tf 20 100 Td (AA) Tj ET";
        push(
            &mut pdf,
            &mut offsets,
            format!(
                "<< /Length {} >>
stream
{content}
endstream",
                content.len()
            )
            .as_bytes(),
        );

        let xref = pdf.len();
        pdf.extend_from_slice(
            format!(
                "xref
0 {}
",
                offsets.len()
            )
            .as_bytes(),
        );
        pdf.extend_from_slice(
            b"0000000000 65535 f 
",
        );
        for &offset in &offsets[1..] {
            pdf.extend_from_slice(
                format!(
                    "{offset:010} 00000 n 
"
                )
                .as_bytes(),
            );
        }
        pdf.extend_from_slice(
            format!(
                "trailer
<< /Size {} /Root 1 0 R >>
startxref
{xref}
%%EOF
",
                offsets.len()
            )
            .as_bytes(),
        );
        pdf
    }
}

#[cfg(test)]
mod standard_14_widths {
    /// A width fallback has to follow the font's encoding. This document puts
    /// its "fi" ligature at code 2; reading the code as a character gives a
    /// control character, whose width is zero, and a zero advance then reads
    /// downstream as a cluster continuation -- so the ligature was skipped
    /// entirely and "intensified" rendered as "intensi ed".
    #[test]
    fn a_width_fallback_follows_the_encoding() {
        let Ok(pdf) = std::fs::read("../website/public/shannon1948.pdf") else {
            eprintln!("skipping: shannon1948.pdf not present");
            return;
        };
        let list = super::extract_display_list(&pdf, 1).expect("page 1");
        let run = list.ops.iter().find_map(|op| match op {
            super::RenderOp::Text {
                text,
                advances,
                size,
                ..
            } if text.contains('\u{FB01}') => Some((text.clone(), advances.clone(), *size)),
            _ => None,
        });
        let Some((text, advances, size)) = run else {
            eprintln!("skipping: no ligature on this page");
            return;
        };
        let at = text.chars().position(|ch| ch == '\u{FB01}').expect("found");
        let em = advances[at] / size;
        assert!(
            em > 0.3,
            "the ligature should carry a real advance, got {em:.3} em"
        );
    }

    /// A standard-14 font may omit `/Widths`, because every reader is required
    /// to know its metrics. Falling back to a flat half em made every run too
    /// wide, so a line overran the position the document explicitly set for
    /// what followed and the text overlapped on screen.
    #[test]
    fn a_base_14_font_without_widths_gets_its_published_metrics() {
        let Ok(pdf) = std::fs::read("../website/public/shannon1948.pdf") else {
            eprintln!("skipping: shannon1948.pdf not present");
            return;
        };
        let list = super::extract_display_list(&pdf, 1).expect("page 1");
        let runs: Vec<_> = list
            .ops
            .iter()
            .filter_map(|op| match op {
                super::RenderOp::Text {
                    text,
                    x,
                    y,
                    width,
                    measured,
                    ..
                } => Some((text.clone(), *x, *y, *width, *measured)),
                _ => None,
            })
            .collect();

        assert!(!runs.is_empty(), "the page should have text");
        assert!(
            runs.iter().all(|run| !run.4),
            "widths should be known, not measured by the renderer"
        );

        // Narrow letters must be narrower than wide ones. A flat default is
        // what this is guarding against, and it makes every width identical.
        let widths: Vec<f64> = runs.iter().map(|run| run.3).collect();
        assert!(widths.iter().any(|w| *w > 0.0), "runs should have a width");

        // Runs on one baseline must not overlap: the document places each one
        // itself, so an overrun is a metric error.
        let mut line: Vec<(f64, f64, String)> = runs
            .iter()
            .filter(|run| (run.2 - 570.9).abs() < 0.5)
            .map(|run| (run.1, run.1 + run.3, run.0.clone()))
            .collect();
        if line.len() < 2 {
            eprintln!("skipping overlap check: expected line not found");
            return;
        }
        line.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        for pair in line.windows(2) {
            assert!(
                pair[0].1 <= pair[1].0 + 0.5,
                "runs overlap: {:?} ends at {:.1} but {:?} starts at {:.1}",
                pair[0].2,
                pair[0].1,
                pair[1].2,
                pair[1].0
            );
        }
    }
}

#[cfg(test)]
mod text_state {
    /// An OCR layer is invisible text over a picture of the same words. It has
    /// to be extractable -- that is the entire point of it -- and it must not
    /// be drawn, or the transcription appears on top of the scan.
    #[test]
    fn invisible_text_is_extracted_but_not_drawn() {
        let visible = build(0);
        let invisible = build(3);
        let clipping = build(7);

        let drawn = |pdf: &[u8]| -> usize {
            super::extract_display_list(pdf, 1)
                .expect("page 1")
                .ops
                .iter()
                .filter(|op| matches!(op, super::RenderOp::Text { .. }))
                .count()
        };

        assert!(drawn(&visible) > 0, "mode 0 should draw");
        assert_eq!(drawn(&invisible), 0, "mode 3 is invisible");
        assert_eq!(drawn(&clipping), 0, "mode 7 clips without painting");

        // Extraction is unaffected: the words are why the layer exists.
        for pdf in [&visible, &invisible, &clipping] {
            let text = crate::PdfDocument::from_bytes(pdf)
                .expect("parse")
                .extract_text();
            assert!(
                text.contains("Hidden"),
                "invisible text must still be extractable, got {text:?}"
            );
        }
    }

    /// Invisible text still moves the pen, so what follows it on the line
    /// stays where the document put it.
    #[test]
    fn invisible_text_still_advances_the_line() {
        let pdf = build_two_runs();
        let list = super::extract_display_list(&pdf, 1).expect("page 1");
        let runs: Vec<(f64, String)> = list
            .ops
            .iter()
            .filter_map(|op| match op {
                super::RenderOp::Text { x, text, .. } => Some((*x, text.clone())),
                _ => None,
            })
            .collect();
        assert_eq!(runs.len(), 1, "only the visible run should be drawn");
        // The visible run follows an invisible one, so it must start past it
        // rather than at the origin.
        assert!(
            runs[0].0 > 60.0,
            "the visible run should start after the invisible one, at {}",
            runs[0].0
        );
    }

    /// `Tr` is graphics state, so `Q` puts back whatever mode was in force
    /// before the matching `q`. A form or annotation that hides its own text
    /// must not hide the page's.
    #[test]
    fn q_restores_the_previous_render_mode() {
        let pdf = document(
            "BT /F1 12 Tf 20 100 Td ET              q BT /F1 12 Tf 3 Tr 20 80 Td (Hidden) Tj ET Q              BT /F1 12 Tf 20 60 Td (Shown) Tj ET",
        );
        let list = super::extract_display_list(&pdf, 1).expect("page 1");
        let drawn: Vec<String> = list
            .ops
            .iter()
            .filter_map(|op| match op {
                super::RenderOp::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(drawn, vec!["Shown".to_string()]);
    }

    /// `Tc` widens every glyph and `Tw` only the spaces. Both are in unscaled
    /// text units, so the effect is independent of the font size.
    #[test]
    fn character_and_word_spacing_widen_the_run() {
        let plain = run_width("BT /F1 10 Tf 20 100 Td (ab cd) Tj ET");
        let charred = run_width("BT /F1 10 Tf 5 Tc 20 100 Td (ab cd) Tj ET");
        let worded = run_width("BT /F1 10 Tf 5 Tw 20 100 Td (ab cd) Tj ET");

        // Five glyphs carry the character spacing: a, b, space, c, d.
        assert!(
            (charred - plain - 25.0).abs() < 0.5,
            "5 pt on 5 glyphs should add 25 pt, got {}",
            charred - plain
        );
        // Only the single space carries the word spacing.
        assert!(
            (worded - plain - 5.0).abs() < 0.5,
            "5 pt on 1 space should add 5 pt, got {}",
            worded - plain
        );
    }

    /// `Tz` scales horizontal displacement, spacing included.
    #[test]
    fn horizontal_scaling_stretches_the_run() {
        let plain = run_width("BT /F1 10 Tf 20 100 Td (abcd) Tj ET");
        let wide = run_width("BT /F1 10 Tf 200 Tz 20 100 Td (abcd) Tj ET");
        assert!(
            (wide - plain * 2.0).abs() < 0.5,
            "200% should double {plain}, got {wide}"
        );
    }

    /// `Ts` lifts the run off the baseline without moving the line.
    #[test]
    fn rise_lifts_the_baseline() {
        let base = first_text("BT /F1 10 Tf 20 100 Td (x) Tj ET").1;
        let lifted = first_text("BT /F1 10 Tf 6 Ts 20 100 Td (x) Tj ET").1;
        assert!(
            (lifted - base - 6.0).abs() < 0.5,
            "a 6 pt rise should lift 6 pt, got {}",
            lifted - base
        );
    }

    /// Spacing must survive `TJ` too, which is where justified text lives.
    #[test]
    fn spacing_applies_to_positioned_text() {
        let plain = run_width("BT /F1 10 Tf 20 100 Td [(ab) -200 (cd)] TJ ET");
        let spaced = run_width("BT /F1 10 Tf 5 Tc 20 100 Td [(ab) -200 (cd)] TJ ET");
        assert!(
            spaced > plain + 15.0,
            "character spacing should widen a TJ run: {plain} -> {spaced}"
        );
    }

    /// The font is graphics state: a `Tf` inside `q`/`Q` ends at the `Q`.
    ///
    /// Documents rely on it. One sets a symbol font for a separator, restores,
    /// and shows the next words with no `Tf` at all; read with the symbol
    /// font, those words decode to nothing and vanish from the page and from
    /// extraction. A page title and half a footer went that way in one
    /// document, a bulleted list in another.
    #[test]
    fn a_font_set_inside_a_saved_state_does_not_outlive_it() {
        let pdf = document(
            "BT /F1 12 Tf 20 150 Td (one) Tj ET              q BT /F1 24 Tf 20 100 Td (two) Tj ET Q              BT 20 50 Td (three) Tj ET",
        );
        let list = super::extract_display_list(&pdf, 1).expect("page 1");
        let sizes: Vec<(String, f64)> = list
            .ops
            .iter()
            .filter_map(|op| match op {
                super::RenderOp::Text { text, size, .. } => Some((text.trim().to_string(), *size)),
                _ => None,
            })
            .collect();
        assert_eq!(sizes.len(), 3, "three runs: {sizes:?}");
        assert_eq!(sizes[0].0, "one");
        assert!((sizes[0].1 - 12.0).abs() < 0.5, "{sizes:?}");
        assert!(
            (sizes[1].1 - 24.0).abs() < 0.5,
            "the saved state's own size"
        );
        // The third run names no font at all and must inherit the first's.
        assert_eq!(sizes[2].0, "three");
        assert!(
            (sizes[2].1 - 12.0).abs() < 0.5,
            "the size from before `q` should be back: {sizes:?}"
        );

        // And the face itself, not just its size.
        let pdf = assemble_with(
            "BT /F1 12 Tf 20 150 Td (one) Tj ET              q BT /F2 12 Tf 20 100 Td (two) Tj ET Q              BT 20 50 Td (three) Tj ET",
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 200]               /Resources << /Font << /F1 4 0 R /F2 6 0 R >> >> /Contents 5 0 R >>",
            b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
            b"<< /Type /Font /Subtype /Type1 /BaseFont /Courier >>",
        );
        let list = super::extract_display_list(&pdf, 1).expect("page 1");
        let faces: Vec<String> = list
            .ops
            .iter()
            .filter_map(|op| match op {
                super::RenderOp::Text { font, .. } => Some(font.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(
            faces,
            vec![
                "Helvetica".to_string(),
                "Courier".to_string(),
                "Helvetica".to_string()
            ],
            "the face from before `q` should be back"
        );
    }

    /// An image inside a form is still an image on the page.
    ///
    /// The textures are gathered by name from the page's resources, and a
    /// form brings its own; without following them a page that wraps its
    /// artwork in a form draws an operator naming an image nobody has, and a
    /// photograph the size of the page simply does not appear.
    #[test]
    fn an_image_inside_a_form_is_found() {
        let pdf = form_document(
            "/Fm0 Do",
            "/BBox [0 0 200 100]              /Resources << /XObject << /Im0 7 0 R >> >>",
            "q 100 0 0 50 10 10 cm /Im0 Do Q",
        );
        // The form names an image the page never mentions.
        let pdf = with_image(&pdf);
        let textures = super::extract_page_textures(&pdf, 1).expect("textures");
        // The contract is that every name the display list draws resolves to a
        // texture, not that the key is spelled any particular way -- the key is
        // private between the two and has to be unique across the document,
        // which a resource name is not.
        let list = super::extract_display_list(&pdf, 1).expect("geometry");
        let drawn: Vec<&String> = list
            .ops
            .iter()
            .filter_map(|op| match op {
                super::RenderOp::Image { name, .. } => Some(name),
                _ => None,
            })
            .collect();
        assert_eq!(drawn.len(), 1, "the form draws one image");
        assert!(
            textures.iter().any(|t| &&t.name == drawn.first().unwrap()),
            "the form's image should be collected: drew {drawn:?}, have {:?}",
            textures.iter().map(|t| &t.name).collect::<Vec<_>>()
        );
    }

    /// Two pictures both called `/Im0` are two pictures.
    ///
    /// A page and a form it draws each have their own resource dictionary, and
    /// producers reuse the short names freely. Keyed by name, the second one
    /// collected is dropped and whichever operator asks gets the first -- so a
    /// banner is drawn with a photograph from further down the page.
    #[test]
    fn images_with_the_same_name_keep_separate_keys() {
        let page_im0 = super::Object::Ref(7, 0);
        let form_im0 = super::Object::Ref(11, 0);
        assert_ne!(
            super::image_key(&page_im0, "Im0"),
            super::image_key(&form_im0, "Im0")
        );
        // The same picture reached by either name is still one picture.
        assert_eq!(
            super::image_key(&page_im0, "Im0"),
            super::image_key(&page_im0, "X")
        );
    }

    /// Append a tiny image as object 7 and repair the cross-reference table.
    fn with_image(pdf: &[u8]) -> Vec<u8> {
        let text = String::from_utf8_lossy(pdf).into_owned();
        let body = "<< /Type /XObject /Subtype /Image /Width 2 /Height 2                      /ColorSpace /DeviceGray /BitsPerComponent 8 /Length 4 >>
                    stream
 @ÿ
endstream";
        let xref_at = text.rfind("xref").expect("an xref");
        let mut out = pdf[..xref_at].to_vec();
        let offset = out.len();
        out.extend_from_slice(
            format!(
                "7 0 obj
{body}
endobj
"
            )
            .as_bytes(),
        );
        let start = out.len();
        // Rebuild the table with the extra entry.
        let mut offsets: Vec<usize> = Vec::new();
        let mut at = 0usize;
        for number in 1..=7 {
            let needle = format!(
                "
{number} 0 obj"
            );
            match text[at..].find(&needle) {
                Some(found) => {
                    offsets.push(at + found + 1);
                    at += found + 1;
                }
                None => offsets.push(offset),
            }
        }
        out.extend_from_slice(
            format!(
                "xref
0 {}
",
                offsets.len() + 1
            )
            .as_bytes(),
        );
        out.extend_from_slice(
            b"0000000000 65535 f 
",
        );
        for at in &offsets {
            out.extend_from_slice(
                format!(
                    "{at:010} 00000 n 
"
                )
                .as_bytes(),
            );
        }
        out.extend_from_slice(
            format!(
                "trailer
<< /Size {} /Root 1 0 R >>
startxref
{start}
%%EOF
",
                offsets.len() + 1
            )
            .as_bytes(),
        );
        out
    }

    /// A form XObject is a content stream and must be run, not drawn.
    ///
    /// A page whose whole content is one form is a real shape -- imposition
    /// and archiving tools emit it -- and treating the form as a picture left
    /// the page blank, in the renderer and in text extraction alike.
    #[test]
    fn a_form_xobject_is_executed() {
        let pdf = form_document(
            "q 1 0 0 1 50 20 cm /Fm0 Do Q",
            "/BBox [0 0 200 100] /Resources << /Font << /F1 4 0 R >> >>",
            "BT /F1 12 Tf 10 10 Td (Inside) Tj ET",
        );
        let list = super::extract_display_list(&pdf, 1).expect("page 1");
        let (text, x, y) = list
            .ops
            .iter()
            .find_map(|op| match op {
                super::RenderOp::Text { text, x, y, .. } => Some((text.clone(), *x, *y)),
                _ => None,
            })
            .expect("the form's text should be drawn");
        assert_eq!(text.trim(), "Inside");
        // Placed by the `cm` at the call site plus the offset inside the form.
        assert!((x - 60.0).abs() < 1.0, "x {x}");
        assert!((y - 30.0).abs() < 1.0, "y {y}");

        // And the same words come out of extraction.
        let extracted = crate::PdfDocument::from_bytes(&pdf)
            .expect("parse")
            .extract_text();
        assert!(
            extracted.contains("Inside"),
            "text inside a form is text on the page: {extracted:?}"
        );
    }

    /// A form's `/Matrix` maps its space into the caller's.
    #[test]
    fn a_form_matrix_places_its_content() {
        let pdf = form_document(
            "/Fm0 Do",
            "/BBox [0 0 200 100] /Matrix [1 0 0 1 100 40]              /Resources << /Font << /F1 4 0 R >> >>",
            "BT /F1 12 Tf 10 10 Td (Inside) Tj ET",
        );
        let list = super::extract_display_list(&pdf, 1).expect("page 1");
        let (x, y) = list
            .ops
            .iter()
            .find_map(|op| match op {
                super::RenderOp::Text { x, y, .. } => Some((*x, *y)),
                _ => None,
            })
            .expect("the form's text");
        assert!((x - 110.0).abs() < 1.0, "x {x}");
        assert!((y - 50.0).abs() < 1.0, "y {y}");
    }

    /// A form that draws itself must not take the reader with it.
    #[test]
    fn a_self_referential_form_terminates() {
        let pdf = form_document(
            "/Fm0 Do",
            "/BBox [0 0 200 100] /Resources << /XObject << /Fm0 6 0 R >> >>",
            "/Fm0 Do",
        );
        // The point is that this returns at all.
        let list = super::extract_display_list(&pdf, 1).expect("page 1");
        assert!(list.ops.len() < 10_000, "bounded: {}", list.ops.len());
    }

    /// A page whose content is one form, with the form as object 6.
    fn form_document(content: &str, form_dict: &str, form_content: &str) -> Vec<u8> {
        let mut pdf: Vec<u8> = Vec::new();
        let mut offsets = vec![0usize];
        pdf.extend_from_slice(
            b"%PDF-1.4
",
        );
        let push = |pdf: &mut Vec<u8>, offsets: &mut Vec<usize>, body: &[u8]| {
            offsets.push(pdf.len());
            let number = offsets.len() - 1;
            pdf.extend_from_slice(
                format!(
                    "{number} 0 obj
"
                )
                .as_bytes(),
            );
            pdf.extend_from_slice(body);
            pdf.extend_from_slice(
                b"
endobj
",
            );
        };
        push(&mut pdf, &mut offsets, b"<< /Type /Catalog /Pages 2 0 R >>");
        push(
            &mut pdf,
            &mut offsets,
            b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        );
        push(
            &mut pdf,
            &mut offsets,
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 200]               /Resources << /XObject << /Fm0 6 0 R >> /Font << /F1 4 0 R >> >>               /Contents 5 0 R >>",
        );
        push(
            &mut pdf,
            &mut offsets,
            b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
        );
        push(
            &mut pdf,
            &mut offsets,
            format!(
                "<< /Length {} >>
stream
{content}
endstream",
                content.len()
            )
            .as_bytes(),
        );
        push(
            &mut pdf,
            &mut offsets,
            {
                let header = format!(
                    "<< /Type /XObject /Subtype /Form {form_dict} /Length {} >>",
                    form_content.len()
                );
                format!(
                    "{header}
stream
{form_content}
endstream"
                )
            }
            .as_bytes(),
        );
        let xref = pdf.len();
        pdf.extend_from_slice(
            format!(
                "xref
0 {}
",
                offsets.len()
            )
            .as_bytes(),
        );
        pdf.extend_from_slice(
            b"0000000000 65535 f 
",
        );
        for &offset in &offsets[1..] {
            pdf.extend_from_slice(
                format!(
                    "{offset:010} 00000 n 
"
                )
                .as_bytes(),
            );
        }
        pdf.extend_from_slice(
            format!(
                "trailer
<< /Size {} /Root 1 0 R >>
startxref
{xref}
%%EOF
",
                offsets.len()
            )
            .as_bytes(),
        );
        pdf
    }

    /// A reader shows the crop box, not the sheet the page was printed on.
    ///
    /// Print-ready files put registration marks, trim corners and a plate
    /// timestamp out in the margin of the media box. Showing the media box
    /// shows all of that, at a different size and offset from every other
    /// viewer.
    #[test]
    fn the_page_shown_is_the_crop_box() {
        // A 300x200 sheet cropped to the middle 100x100.
        let pdf = document_with_page(
            "BT /F1 12 Tf 120 60 Td (in) Tj ET",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 200]               /CropBox [100 50 200 150]               /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>",
        );
        let list = super::extract_display_list(&pdf, 1).expect("page 1");
        assert_eq!((list.width, list.height), (100.0, 100.0), "the crop box");

        let (x, y) = list
            .ops
            .iter()
            .find_map(|op| match op {
                super::RenderOp::Text { x, y, .. } => Some((*x, *y)),
                _ => None,
            })
            .expect("a text op");
        // Placed at (120, 60) on the sheet, which is (20, 10) inside the crop.
        assert!((x - 20.0).abs() < 0.5, "x should be crop-relative: {x}");
        assert!((y - 10.0).abs() < 0.5, "y should be crop-relative: {y}");
    }

    /// Extraction reports the same coordinates rendering draws at.
    ///
    /// A citation is a page number and a box. If extraction measures from the
    /// sheet while the renderer measures from the crop, the box a search hit
    /// hands back is offset by the trim margin and highlights empty paper.
    #[test]
    fn extracted_boxes_share_the_rendered_origin() {
        let pdf = document_with_page(
            "BT /F1 12 Tf 120 60 Td (in) Tj ET",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 200]               /CropBox [100 50 200 150]               /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>",
        );
        let doc = super::parse_document(&pdf).expect("parse");
        let page = &doc.pages[0];
        assert_eq!((page.width, page.height), (100.0, 100.0), "the crop box");
        let block = page
            .text_content
            .iter()
            .find(|b| b.text.contains("in"))
            .expect("the text");
        assert!(
            (block.x - 20.0).abs() < 0.5,
            "x should be crop-relative: {}",
            block.x
        );
    }

    /// A crop box outside the sheet is clamped to it rather than believed.
    #[test]
    fn a_crop_box_is_clamped_to_the_sheet() {
        let pdf = document_with_page(
            "BT /F1 12 Tf 10 10 Td (x) Tj ET",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 200]               /CropBox [-50 -50 900 900]               /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>",
        );
        let list = super::extract_display_list(&pdf, 1).expect("page 1");
        assert_eq!((list.width, list.height), (300.0, 200.0));
    }

    /// A code the document never gives a meaning for still occupies a slot.
    ///
    /// TeX's f-ligatures arrive with no `/ToUnicode` and no `/Differences`:
    /// just a byte and a charstring in the embedded font. Dropping the code
    /// dropped the ligature and folded its width into the letter before it,
    /// which is how "efficiently" rendered as "e ciently" with a gap.
    #[test]
    fn an_unnamed_code_keeps_its_place() {
        // Byte 12 is a form feed in WinAnsi -- no character -- and is where
        // Computer Modern keeps "fi". The font gives it a width, as a font
        // with a glyph there does.
        let pdf = document_with_widths("BT /F1 12 Tf 20 100 Td (ab) Tj ET");
        let list = super::extract_display_list(&pdf, 1).expect("page 1");
        let (text, advances, codes) = list
            .ops
            .iter()
            .find_map(|op| match op {
                super::RenderOp::Text {
                    text,
                    advances,
                    codes,
                    ..
                } => Some((text.clone(), advances.clone(), codes.clone())),
                _ => None,
            })
            .expect("a text op");

        assert_eq!(
            text.chars().count(),
            3,
            "three codes, three slots: {text:?}"
        );
        assert_eq!(codes[1], 12, "the middle slot keeps the document's code");
        assert!(
            (advances[1] - 12.0).abs() < 0.5,
            "the unnamed code carries its own width rather than swelling its              neighbour: {advances:?}"
        );
    }

    /// As `document`, but the font states a width for code 12.
    fn document_with_widths(content: &str) -> Vec<u8> {
        document_with_font(
            content,
            b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica               /FirstChar 12 /LastChar 12 /Widths [1000] >>",
        )
    }

    /// In an Indexed space the operand is a row of the space's own table, not
    /// a colour. Read as a grey level, `1 scn` paints white.
    #[test]
    fn an_indexed_colour_is_a_row_of_its_table() {
        // Two entries: black, then a mid grey.
        let pdf = document_with_page(
            "/CS0 cs 1 scn BT /F1 12 Tf 20 100 Td (indexed) Tj ET",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 200]               /Resources << /Font << /F1 4 0 R >>               /ColorSpace << /CS0 [/Indexed /DeviceRGB 1 <000000808080>] >> >>               /Contents 5 0 R >>",
        );
        let list = super::extract_display_list(&pdf, 1).expect("page 1");
        let colour = list
            .ops
            .iter()
            .find_map(|op| match op {
                super::RenderOp::Text { color, .. } => Some(*color),
                _ => None,
            })
            .expect("a text op");
        assert!(
            (colour[0] - 0.5).abs() < 0.02 && (colour[1] - 0.5).abs() < 0.02,
            "index 1 is the table's second entry, a mid grey: {colour:?}"
        );
    }

    /// A Separation tint of 1 is full ink, not white. Read as a grey level it
    /// paints white text on white paper -- a page that looks blank while the
    /// display list insists it drew thousands of glyphs.
    #[test]
    fn a_full_tint_is_ink_not_white() {
        let pdf = spot_color_document("/CS0 cs 1 scn");
        let color = first_fill(&pdf);
        assert!(
            color[0] < 0.1 && color[1] < 0.1 && color[2] < 0.1,
            "a full tint should be dark, got {color:?}"
        );

        // And no tint is the paper.
        let pdf = spot_color_document("/CS0 cs 0 scn");
        let color = first_fill(&pdf);
        assert!(color[0] > 0.9, "no tint should be white, got {color:?}");

        // A grey space still reads the old way round.
        let pdf = spot_color_document("/DeviceGray cs 1 scn");
        let color = first_fill(&pdf);
        assert!(color[0] > 0.9, "grey 1 is white, got {color:?}");
    }

    /// `Q` restores the colour as well as the transform.
    #[test]
    fn q_restores_the_fill_colour() {
        let pdf = spot_color_document("1 0 0 rg q /CS0 cs 1 scn Q");
        let color = first_fill(&pdf);
        assert!(
            color[0] > 0.9 && color[1] < 0.1,
            "the red set before `q` should survive `Q`, got {color:?}"
        );
    }

    fn first_fill(pdf: &[u8]) -> [f64; 4] {
        let list = super::extract_display_list(pdf, 1).expect("page 1");
        list.ops
            .iter()
            .find_map(|op| match op {
                super::RenderOp::Text { color, .. } => Some(*color),
                _ => None,
            })
            .expect("a text op")
    }

    /// A page whose `/CS0` is a Separation over DeviceCMYK, as a press-ready
    /// document's black text usually is.
    fn spot_color_document(prelude: &str) -> Vec<u8> {
        let content = format!("{prelude} BT /F1 12 Tf 20 100 Td (Ink) Tj ET");
        let mut pdf: Vec<u8> = Vec::new();
        let mut offsets = vec![0usize];
        pdf.extend_from_slice(
            b"%PDF-1.4
",
        );
        let push = |pdf: &mut Vec<u8>, offsets: &mut Vec<usize>, body: &[u8]| {
            offsets.push(pdf.len());
            let number = offsets.len() - 1;
            pdf.extend_from_slice(
                format!(
                    "{number} 0 obj
"
                )
                .as_bytes(),
            );
            pdf.extend_from_slice(body);
            pdf.extend_from_slice(
                b"
endobj
",
            );
        };
        push(&mut pdf, &mut offsets, b"<< /Type /Catalog /Pages 2 0 R >>");
        push(
            &mut pdf,
            &mut offsets,
            b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        );
        push(
            &mut pdf,
            &mut offsets,
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 200]               /Resources << /Font << /F1 4 0 R >>               /ColorSpace << /CS0 [/Separation /Black /DeviceCMYK 6 0 R] >> >>               /Contents 5 0 R >>",
        );
        push(
            &mut pdf,
            &mut offsets,
            b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
        );
        push(
            &mut pdf,
            &mut offsets,
            format!(
                "<< /Length {} >>
stream
{content}
endstream",
                content.len()
            )
            .as_bytes(),
        );
        // The tint transform. Never evaluated -- the tint is read as coverage
        // -- but a Separation is malformed without one.
        push(
            &mut pdf,
            &mut offsets,
            b"<< /FunctionType 2 /Domain [0 1] /C0 [0 0 0 0] /C1 [0 0 0 1] /N 1 >>",
        );
        let xref = pdf.len();
        pdf.extend_from_slice(
            format!(
                "xref
0 {}
",
                offsets.len()
            )
            .as_bytes(),
        );
        pdf.extend_from_slice(
            b"0000000000 65535 f 
",
        );
        for &offset in &offsets[1..] {
            pdf.extend_from_slice(
                format!(
                    "{offset:010} 00000 n 
"
                )
                .as_bytes(),
            );
        }
        pdf.extend_from_slice(
            format!(
                "trailer
<< /Size {} /Root 1 0 R >>
startxref
{xref}
%%EOF
",
                offsets.len()
            )
            .as_bytes(),
        );
        pdf
    }

    /// The advance is measured from where the first run starts to where the
    /// text matrix leaves the pen, which is what the next `Tj` on the line
    /// would use.
    fn run_width(content: &str) -> f64 {
        let pdf = document(&format!("{content} BT /F1 10 Tf 0 0 Td (|) Tj ET"));
        let list = super::extract_display_list(&pdf, 1).expect("page 1");
        let xs: Vec<f64> = list
            .ops
            .iter()
            .filter_map(|op| match op {
                super::RenderOp::Text { x, width, .. } => Some(x + width),
                _ => None,
            })
            .collect();
        let start = first_text(content).0;
        xs[0] - start
    }

    fn first_text(content: &str) -> (f64, f64) {
        let pdf = document(content);
        let list = super::extract_display_list(&pdf, 1).expect("page 1");
        list.ops
            .iter()
            .find_map(|op| match op {
                super::RenderOp::Text { x, y, .. } => Some((*x, *y)),
                _ => None,
            })
            .expect("a text op")
    }

    fn build(mode: i64) -> Vec<u8> {
        document(&format!(
            "BT /F1 12 Tf {mode} Tr 20 100 Td (Hidden words) Tj ET"
        ))
    }

    fn build_two_runs() -> Vec<u8> {
        document("BT /F1 12 Tf 20 100 Td 3 Tr (Invisible) Tj 0 Tr (Shown) Tj ET")
    }

    fn document(content: &str) -> Vec<u8> {
        document_with_font(
            content,
            b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
        )
    }

    fn document_with_page(content: &str, page: &str) -> Vec<u8> {
        assemble(
            content,
            page.as_bytes(),
            b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
        )
    }

    fn document_with_font(content: &str, font: &[u8]) -> Vec<u8> {
        assemble(
            content,
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 200]               /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>",
            font,
        )
    }

    fn assemble(content: &str, page: &[u8], font: &[u8]) -> Vec<u8> {
        assemble_with(content, page, font, b"")
    }

    /// As `assemble`, with one more object appended as number 6.
    fn assemble_with(content: &str, page: &[u8], font: &[u8], extra: &[u8]) -> Vec<u8> {
        let mut pdf: Vec<u8> = Vec::new();
        let mut offsets = vec![0usize];
        pdf.extend_from_slice(b"%PDF-1.4\n");
        let push = |pdf: &mut Vec<u8>, offsets: &mut Vec<usize>, body: &[u8]| {
            offsets.push(pdf.len());
            let number = offsets.len() - 1;
            pdf.extend_from_slice(format!("{number} 0 obj\n").as_bytes());
            pdf.extend_from_slice(body);
            pdf.extend_from_slice(b"\nendobj\n");
        };
        push(&mut pdf, &mut offsets, b"<< /Type /Catalog /Pages 2 0 R >>");
        push(
            &mut pdf,
            &mut offsets,
            b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        );
        push(&mut pdf, &mut offsets, page);
        push(&mut pdf, &mut offsets, font);
        push(
            &mut pdf,
            &mut offsets,
            format!(
                "<< /Length {} >>\nstream\n{content}\nendstream",
                content.len()
            )
            .as_bytes(),
        );
        if !extra.is_empty() {
            push(&mut pdf, &mut offsets, extra);
        }
        let xref = pdf.len();
        pdf.extend_from_slice(format!("xref\n0 {}\n", offsets.len()).as_bytes());
        pdf.extend_from_slice(b"0000000000 65535 f \n");
        for &offset in &offsets[1..] {
            pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
        }
        pdf.extend_from_slice(
            format!(
                "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
                offsets.len()
            )
            .as_bytes(),
        );
        pdf
    }
}

#[cfg(test)]
mod transparency {
    use super::{Blend, blend_is_noop, with_alpha};

    /// Multiplying by white and screening by black leave the page exactly as
    /// it was. A producer that paints one means "keep the backdrop", and often
    /// covers a whole panel doing it -- so painting it normally instead is not
    /// a small error, it erases everything underneath.
    #[test]
    fn identity_blends_paint_nothing() {
        let white = [1.0, 1.0, 1.0, 1.0];
        let black = [0.0, 0.0, 0.0, 1.0];
        assert!(blend_is_noop(white, Blend::Multiply));
        assert!(blend_is_noop(black, Blend::Screen));
        // Only the identity colour of each mode is a no-op.
        assert!(!blend_is_noop(black, Blend::Multiply));
        assert!(!blend_is_noop(white, Blend::Screen));
        // Under normal painting white covers the page, and must be drawn.
        assert!(!blend_is_noop(white, Blend::Normal));
    }

    /// Fully transparent paint changes nothing whatever the mode.
    #[test]
    fn zero_alpha_paints_nothing() {
        for blend in [Blend::Normal, Blend::Multiply, Blend::Screen, Blend::Other] {
            assert!(blend_is_noop([0.2, 0.4, 0.6, 0.0], blend));
        }
    }

    /// `ca` multiplies the colour's own alpha rather than replacing it, and is
    /// held to the unit range however the document writes it.
    #[test]
    fn constant_alpha_multiplies_and_clamps() {
        assert_eq!(with_alpha([0.0, 0.0, 0.0, 1.0], 0.5)[3], 0.5);
        assert_eq!(with_alpha([0.0, 0.0, 0.0, 0.5], 0.5)[3], 0.25);
        assert_eq!(with_alpha([0.0, 0.0, 0.0, 1.0], 4.0)[3], 1.0);
        assert_eq!(with_alpha([0.0, 0.0, 0.0, 1.0], -1.0)[3], 0.0);
    }
}
