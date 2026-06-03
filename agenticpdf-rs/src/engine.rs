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
}

/// Decoded object stream: per-stream (object header offsets, decoded bytes).
type ObjStmCache = HashMap<u32, (Vec<(u32, usize)>, Vec<u8>)>;

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
        };
        doc.build_xref()?;
        if doc.xref.is_empty() || !doc.trailer.contains_key("Root") {
            // Recovery: scan the whole file for "n g obj" definitions.
            doc.recover_by_scan();
        }
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
            Xref::Offset(off) => self.parse_indirect_at(off)?,
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

fn decode_stream(dict: &Dict, raw: &[u8]) -> Result<Vec<u8>, PdfError> {
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
                let predictor = parm
                    .as_ref()
                    .and_then(|p| p.get("Predictor"))
                    .and_then(|o| o.as_int())
                    .map(|n| n as u8);
                let columns = parm
                    .as_ref()
                    .and_then(|p| p.get("Columns"))
                    .and_then(|o| o.as_int())
                    .map(|n| n as usize);
                data = PdfParser::decompress_stream(&data, predictor, columns)?;
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
        let (w, h) = media_box(&doc, pd);
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
        let (w, h) = media_box(&doc, pd);
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
        color: [f64; 4],
        font: String,
    },
    /// An image XObject placement; `name` is the page resource key.
    Image {
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        name: String,
    },
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
    let (width, height) = media_box(&doc, pd);
    let ops = build_display_ops(&doc, pd);
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

fn build_display_ops(doc: &Document, page: &Dict) -> Vec<RenderOp> {
    let fonts = build_fonts(doc, page);
    let content = page_contents(doc, page);
    if content.is_empty() {
        return Vec::new();
    }
    let mut ops = Vec::new();

    let mut ctm_stack: Vec<Matrix> = Vec::new();
    let mut ctm: Matrix = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
    let mut tm: Matrix = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
    let mut tlm: Matrix = tm;
    let mut font_size = 0.0f64;
    let mut leading = 0.0f64;
    let mut cur_font: Option<&Font> = None;
    let mut cur_font_name = String::new();
    let mut fill: [f64; 4] = [0.0, 0.0, 0.0, 1.0];
    let mut stroke: [f64; 4] = [0.0, 0.0, 0.0, 1.0];
    let mut line_width = 1.0f64;

    // Path state, in device space.
    let mut subpaths: Vec<Vec<[f64; 2]>> = Vec::new();
    let mut cur: Vec<[f64; 2]> = Vec::new();
    let mut start = [0.0f64, 0.0];
    let mut pt = [0.0f64, 0.0];

    let mut stack: Vec<Token> = Vec::new();
    let mut lex = ContentLexer::new(&content);

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
                    "q" => ctm_stack.push(ctm),
                    "Q" => {
                        if let Some(m) = ctm_stack.pop() {
                            ctm = m;
                        }
                    }
                    "cm" => {
                        if let Some(m) = last6(&stack) {
                            ctm = mat_mul(&m, &ctm);
                        }
                    }
                    "w" => {
                        if let Some(Token::Num(v)) = stack.last() {
                            let scale = (ctm[0] * ctm[3] - ctm[1] * ctm[2]).abs().sqrt();
                            line_width = (*v * scale).max(0.1);
                        }
                    }
                    "g" | "rg" | "k" => fill = color_from_stack(&stack),
                    "G" | "RG" | "K" => stroke = color_from_stack(&stack),
                    "sc" | "scn" => fill = color_from_stack(&stack),
                    "SC" | "SCN" => stroke = color_from_stack(&stack),
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
                            if fills {
                                ops.push(RenderOp::Fill {
                                    subpaths: paths.clone(),
                                    color: fill,
                                    even_odd: op.ends_with('*'),
                                });
                            }
                            if strokes {
                                ops.push(RenderOp::Stroke {
                                    subpaths: paths,
                                    color: stroke,
                                    width: line_width,
                                });
                            }
                        }
                    }
                    "n" => {
                        cur.clear();
                        subpaths.clear();
                    }
                    "Do" => {
                        if let Some(Token::Name(name)) = stack.last() {
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
                                name: name.clone(),
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
                            push_text_op(
                                bytes,
                                cur_font,
                                &cur_font_name,
                                font_size,
                                &tm,
                                &ctm,
                                fill,
                                &mut ops,
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
                                        if *adj < -120.0 && !combined.ends_with(' ') {
                                            combined.push(' ');
                                        }
                                    }
                                }
                            }
                            if !combined.trim().is_empty() {
                                emit_text_op(
                                    &combined,
                                    &cur_font_name,
                                    font_size,
                                    &tm,
                                    &ctm,
                                    fill,
                                    &mut ops,
                                );
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
        if ops.len() > 2_000_000 {
            break;
        }
    }
    ops
}

#[allow(clippy::too_many_arguments)]
fn push_text_op(
    bytes: &[u8],
    font: Option<&Font>,
    font_name: &str,
    font_size: f64,
    tm: &Matrix,
    ctm: &Matrix,
    color: [f64; 4],
    ops: &mut Vec<RenderOp>,
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
    emit_text_op(&s, font_name, font_size, tm, ctm, color, ops);
}

#[allow(clippy::too_many_arguments)]
fn emit_text_op(
    text: &str,
    font_name: &str,
    font_size: f64,
    tm: &Matrix,
    ctm: &Matrix,
    color: [f64; 4],
    ops: &mut Vec<RenderOp>,
) {
    let trm = mat_mul(tm, ctm);
    let scale = (trm[0] * trm[3] - trm[1] * trm[2]).abs().sqrt();
    let size = if font_size > 0.0 {
        font_size * scale.max(0.01)
    } else {
        scale.max(1.0)
    };
    ops.push(RenderOp::Text {
        text: text.to_string(),
        x: trm[4],
        y: trm[5],
        size,
        color,
        font: font_name.to_string(),
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

/// Parse an Indexed color-space palette from an image dict, if present.
fn extract_palette(doc: &Document, d: &Dict) -> Option<Palette> {
    let cs = doc.get(d, "ColorSpace").or_else(|| doc.get(d, "CS"))?;
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
    // /DecodeParms may be a dict or (parallel to a filter array) an array.
    let parms = match doc.get(d, "DecodeParms").or_else(|| doc.get(d, "DP")) {
        Some(Object::Dict(p)) => Some(p),
        Some(Object::Array(a)) => a.iter().find_map(|o| match doc.resolve(o) {
            Object::Dict(p) => Some(p),
            _ => None,
        }),
        _ => None,
    };
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

fn media_box(doc: &Document, page: &Dict) -> (f64, f64) {
    if let Some(mb) = rect4(doc, page, "MediaBox") {
        ((mb[2] - mb[0]).abs(), (mb[3] - mb[1]).abs())
    } else {
        (612.0, 792.0)
    }
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
    base_font: String,
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
}

fn build_fonts(doc: &Document, page: &Dict) -> HashMap<String, Font> {
    let mut fonts = HashMap::new();
    let resources = match doc.get(page, "Resources") {
        Some(o) => o,
        None => return fonts,
    };
    let rd = match resources.as_dict() {
        Some(d) => d.clone(),
        None => return fonts,
    };
    let font_dict = match doc.get(&rd, "Font") {
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
            fonts.insert(name.clone(), build_one_font(doc, font));
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

    Font {
        two_byte,
        to_unicode,
        simple,
        codespace,
        cmap_unicode,
        base_font,
    }
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

/// Parse a ToUnicode CMap: bfchar and bfrange sections.
fn parse_tounicode(data: &[u8], map: &mut HashMap<u32, String>) {
    let text = String::from_utf8_lossy(data);
    let bytes = text.as_bytes();

    // bfchar
    let mut search_from = 0;
    while let Some(rel) = find_sub(&bytes[search_from..], b"beginbfchar") {
        let start = search_from + rel + b"beginbfchar".len();
        let end = find_sub(&bytes[start..], b"endbfchar")
            .map(|p| start + p)
            .unwrap_or(bytes.len());
        let section = &text[start..end];
        for line in section.lines() {
            let toks = tokenize_cmap(line);
            if toks.len() >= 2
                && let (Some(code), Some(dst)) =
                    (hex_to_u32(&toks[0]), Some(hex_to_string(&toks[1])))
            {
                map.insert(code, dst);
            }
        }
        search_from = end + b"endbfchar".len();
        if search_from >= bytes.len() {
            break;
        }
    }

    // bfrange
    let mut search_from = 0;
    while let Some(rel) = find_sub(&bytes[search_from..], b"beginbfrange") {
        let start = search_from + rel + b"beginbfrange".len();
        let end = find_sub(&bytes[start..], b"endbfrange")
            .map(|p| start + p)
            .unwrap_or(bytes.len());
        let section = &text[start..end];
        for line in section.lines() {
            let toks = tokenize_cmap(line);
            if toks.len() >= 3 {
                let lo = hex_to_u32(&toks[0]);
                let hi = hex_to_u32(&toks[1]);
                if let (Some(lo), Some(hi)) = (lo, hi) {
                    if toks[2].starts_with('[') {
                        // array form handled separately below; skip simple path
                    } else if let Some(base) = hex_to_u32(&toks[2]) {
                        let count = hi.saturating_sub(lo);
                        for k in 0..=count.min(65535) {
                            if let Some(c) = char::from_u32(base + k) {
                                map.insert(lo + k, c.to_string());
                            }
                        }
                    }
                }
            }
        }
        search_from = end + b"endbfrange".len();
        if search_from >= bytes.len() {
            break;
        }
    }
}

/// Split a CMap line into `<...>` / `[...]` / bare tokens.
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
fn extract_page_content(doc: &Document, page: &Dict, page_number: usize) -> PageContent {
    let fonts = build_fonts(doc, page);
    let content = page_contents(doc, page);
    if content.is_empty() {
        return PageContent {
            text: Vec::new(),
            h_lines: Vec::new(),
            v_lines: Vec::new(),
            placed: Vec::new(),
        };
    }

    let mut blocks = Vec::new();
    let mut h_lines: Vec<Seg> = Vec::new();
    let mut v_lines: Vec<Seg> = Vec::new();
    let mut placed: Vec<(String, [f64; 4])> = Vec::new();

    // Graphics + text state.
    let mut ctm_stack: Vec<Matrix> = Vec::new();
    let mut ctm: Matrix = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
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

    let mut lex = ContentLexer::new(&content);
    while let Some(tok) = lex.next_token() {
        match tok {
            Token::Op(op) => {
                match op.as_str() {
                    "q" => ctm_stack.push(ctm),
                    "Q" => {
                        if let Some(m) = ctm_stack.pop() {
                            ctm = m;
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

    PageContent {
        text: blocks,
        h_lines,
        v_lines,
        placed,
    }
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
            two_byte: true,
            to_unicode: tu,
            simple: None,
            codespace: vec![(0x00, 0x80, 1), (0x8140, 0x9ffc, 2)],
            cmap_unicode: false,
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
            two_byte: true,
            to_unicode: tu,
            simple: None,
            codespace: vec![], // Identity-H
            cmap_unicode: false,
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
            two_byte: true,
            to_unicode: HashMap::new(),
            simple: None,
            codespace: vec![],
            cmap_unicode: true,
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
            two_byte: true,
            to_unicode: HashMap::new(),
            simple: None,
            codespace: vec![],
            cmap_unicode: false,
            base_font: String::new(),
        };
        let mut out = String::new();
        font.decode(&[0x00, 0x03], &mut out);
        assert_eq!(out, "");
    }
}
