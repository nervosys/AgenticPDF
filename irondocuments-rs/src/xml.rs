// SPDX-License-Identifier: AGPL-3.0-or-later
//! Minimal namespace-aware pull XML parser.
//!
//! Every ZIP-container document format in this crate is a bag of XML parts:
//! OOXML (`word/document.xml`, `xl/worksheets/sheet1.xml`, `ppt/slides/*.xml`),
//! OpenDocument (`content.xml`, `styles.xml`) and EPUB (XHTML). They all need
//! the same thing — stream through elements, read attributes, collect text —
//! so that is all this provides.
//!
//! Namespaces are resolved properly rather than assumed from prefixes, because
//! prefix collisions are real: inside `word/document.xml` a `<w:t>` (Word text)
//! and an `<a:t>` (DrawingML text inside a chart or shape) both have the local
//! name `t`, and only the namespace URI tells them apart. [`Element::in_ns`]
//! and the `ns::*` constants make that check cheap.
//!
//! ## Security posture
//!
//! Only the five predefined entities and numeric character references are
//! expanded. Custom `<!ENTITY>` declarations are **not** processed at all, so
//! entity-expansion ("billion laughs") and external-entity (XXE) attacks are
//! structurally impossible rather than merely mitigated. Element nesting is
//! capped by [`MAX_DEPTH`].
//!
//! ## Error posture
//!
//! Parsing is lenient, matching how the rest of the engine treats real-world
//! documents: malformed markup ends the event stream instead of discarding
//! everything read so far, and [`Reader::error`] reports whether that happened.
//! A truncated document still yields the text it did contain.

/// Well-known namespace URIs, so format parsers can disambiguate local names.
pub mod ns {
    /// WordprocessingML — `word/document.xml` body content.
    pub const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
    /// SpreadsheetML — `xl/workbook.xml`, worksheets, shared strings.
    pub const X: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
    /// PresentationML — `ppt/presentation.xml`, slides.
    pub const P: &str = "http://schemas.openxmlformats.org/presentationml/2006/main";
    /// DrawingML — shapes, charts, and the text inside them.
    pub const A: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";
    /// OPC relationships — `_rels/*.rels`.
    pub const RELS: &str = "http://schemas.openxmlformats.org/package/2006/relationships";
    /// OPC content types — `[Content_Types].xml`.
    pub const CONTENT_TYPES: &str = "http://schemas.openxmlformats.org/package/2006/content-types";
    /// Relationship references (`r:id` attributes).
    pub const R: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
    /// Dublin Core — document metadata in both OOXML and ODF.
    pub const DC: &str = "http://purl.org/dc/elements/1.1/";
    /// OOXML core properties — `docProps/core.xml`.
    pub const CP: &str = "http://schemas.openxmlformats.org/package/2006/metadata/core-properties";

    /// OpenDocument text content.
    pub const ODF_TEXT: &str = "urn:oasis:names:tc:opendocument:xmlns:text:1.0";
    /// OpenDocument tables (also used for spreadsheet cells).
    pub const ODF_TABLE: &str = "urn:oasis:names:tc:opendocument:xmlns:table:1.0";
    /// OpenDocument office-level elements (`office:body`, `office:text`).
    pub const ODF_OFFICE: &str = "urn:oasis:names:tc:opendocument:xmlns:office:1.0";
    /// OpenDocument drawing elements (frames, images, slide shapes).
    pub const ODF_DRAW: &str = "urn:oasis:names:tc:opendocument:xmlns:drawing:1.0";
    /// OpenDocument style definitions.
    pub const ODF_STYLE: &str = "urn:oasis:names:tc:opendocument:xmlns:style:1.0";
    /// OpenDocument presentation elements (slide notes, placeholder classes).
    pub const ODF_PRESENTATION: &str = "urn:oasis:names:tc:opendocument:xmlns:presentation:1.0";
    /// OpenDocument document metadata (`meta.xml`).
    pub const ODF_META: &str = "urn:oasis:names:tc:opendocument:xmlns:meta:1.0";
    /// SVG-compatible attributes reused by ODF for geometry.
    pub const SVG: &str = "urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0";
    /// XSL-FO attributes reused by ODF for formatting.
    pub const FO: &str = "urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0";
    /// XLink — ODF hyperlink and image references.
    pub const XLINK: &str = "http://www.w3.org/1999/xlink";
    /// XHTML — EPUB content documents.
    pub const XHTML: &str = "http://www.w3.org/1999/xhtml";
}

/// Maximum element nesting. Deeply nested markup is either a bug or an attack;
/// legitimate documents stay far below this.
pub const MAX_DEPTH: usize = 256;

/// A start tag with its resolved namespace and attributes.
#[derive(Debug, Clone, Default)]
pub struct Element {
    /// Qualified name as written, e.g. `w:p`.
    pub qname: String,
    /// Local name with any prefix stripped, e.g. `p`.
    pub local: String,
    /// Resolved namespace URI, or empty when the element is un-namespaced.
    pub ns: String,
    /// Attributes as `(qualified name, value)`, in document order.
    pub attrs: Vec<(String, String)>,
    /// Whether the tag was written in self-closing form (`<w:br/>`).
    pub self_closing: bool,
}

impl Element {
    /// Attribute by qualified name, e.g. `w:val` or `xml:space`.
    pub fn attr(&self, qname: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(k, _)| k == qname)
            .map(|(_, v)| v.as_str())
    }

    /// Attribute by local name, ignoring any prefix.
    ///
    /// Convenient when the prefix varies between producers (`r:id` vs `rel:id`)
    /// and the local name is unambiguous on that element.
    pub fn attr_local(&self, local: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(k, _)| local_name(k) == local)
            .map(|(_, v)| v.as_str())
    }

    /// Whether this element is `local` in namespace `uri`.
    pub fn is(&self, uri: &str, local: &str) -> bool {
        self.local == local && self.ns == uri
    }

    /// Whether this element belongs to namespace `uri`.
    pub fn in_ns(&self, uri: &str) -> bool {
        self.ns == uri
    }
}

/// One item from the document stream.
#[derive(Debug, Clone)]
pub enum Event {
    /// An element opened. Self-closing tags emit `Start` then `End`, so
    /// consumers need only one balanced shape.
    Start(Element),
    /// An element closed; carries the qualified name.
    End(String),
    /// Character data, with entities already expanded.
    Text(String),
}

/// A streaming XML reader over an in-memory document.
pub struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
    /// Set when a self-closing tag has emitted its `Start` and owes an `End`.
    pending_end: Option<String>,
    /// Namespace scopes: one frame per open element, each holding the
    /// `(prefix, uri)` bindings that element declared.
    scopes: Vec<Vec<(String, String)>>,
    /// Open element names, for depth tracking and implicit closing.
    open: Vec<String>,
    error: Option<String>,
}

impl<'a> Reader<'a> {
    /// Create a reader over raw part bytes. A leading BOM is skipped.
    pub fn new(data: &'a [u8]) -> Self {
        let data = data.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(data);
        Reader {
            data,
            pos: 0,
            pending_end: None,
            scopes: Vec::new(),
            open: Vec::new(),
            error: None,
        }
    }

    /// The message describing why parsing stopped early, if it did.
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// Current element nesting depth.
    pub fn depth(&self) -> usize {
        self.open.len()
    }

    /// Resolve a namespace prefix against the current scope stack.
    ///
    /// Inner scopes shadow outer ones, so the search runs innermost-first.
    fn resolve(&self, prefix: &str) -> String {
        for frame in self.scopes.iter().rev() {
            for (p, uri) in frame.iter().rev() {
                if p == prefix {
                    return uri.clone();
                }
            }
        }
        String::new()
    }

    fn fail(&mut self, msg: impl Into<String>) -> Option<Event> {
        if self.error.is_none() {
            self.error = Some(msg.into());
        }
        self.pos = self.data.len();
        None
    }

    /// Advance to the next event, or `None` at end of document (or on the first
    /// malformed construct).
    pub fn read_event(&mut self) -> Option<Event> {
        if let Some(name) = self.pending_end.take() {
            self.scopes.pop();
            self.open.pop();
            return Some(Event::End(name));
        }

        loop {
            if self.pos >= self.data.len() {
                return None;
            }

            if self.data[self.pos] != b'<' {
                let text = self.read_text();
                // Ignore inter-element whitespace-only runs at depth 0; inside
                // an element they may be significant (xml:space="preserve").
                if text.is_empty() {
                    continue;
                }
                return Some(Event::Text(text));
            }

            // A markup construct. Dispatch on what follows '<'.
            match self.data.get(self.pos + 1) {
                Some(b'?') => {
                    // Processing instruction / XML declaration.
                    if !self.skip_until(b"?>") {
                        return self.fail("unterminated processing instruction");
                    }
                }
                Some(b'!') => {
                    if self.data[self.pos..].starts_with(b"<!--") {
                        self.pos += 4;
                        if !self.skip_until(b"-->") {
                            return self.fail("unterminated comment");
                        }
                    } else if self.data[self.pos..].starts_with(b"<![CDATA[") {
                        self.pos += 9;
                        let start = self.pos;
                        let Some(end) = find(self.data, self.pos, b"]]>") else {
                            return self.fail("unterminated CDATA section");
                        };
                        let text = String::from_utf8_lossy(&self.data[start..end]).into_owned();
                        self.pos = end + 3;
                        if !text.is_empty() {
                            return Some(Event::Text(text));
                        }
                    } else {
                        // DOCTYPE or an entity declaration. Skipped wholesale —
                        // see the module note on why entities are never expanded.
                        if !self.skip_doctype() {
                            return self.fail("unterminated declaration");
                        }
                    }
                }
                Some(b'/') => return self.read_end_tag(),
                Some(_) => return self.read_start_tag(),
                None => return self.fail("truncated markup"),
            }
        }
    }

    /// Read character data up to the next `<`, expanding entities.
    ///
    /// Whitespace-only runs are returned, not discarded. They are usually just
    /// pretty-printing between elements — but inside `<w:t xml:space="preserve">`
    /// a lone space is the word boundary between two runs, and dropping it
    /// welds the words together. Deciding which is which needs context the
    /// reader does not have, so it reports what is there and lets the format
    /// parser choose.
    fn read_text(&mut self) -> String {
        let start = self.pos;
        while self.pos < self.data.len() && self.data[self.pos] != b'<' {
            self.pos += 1;
        }
        decode_entities(&String::from_utf8_lossy(&self.data[start..self.pos]))
    }

    fn read_end_tag(&mut self) -> Option<Event> {
        self.pos += 2; // consume "</"
        let start = self.pos;
        let Some(close) = find(self.data, self.pos, b">") else {
            return self.fail("unterminated end tag");
        };
        let qname = String::from_utf8_lossy(&self.data[start..close])
            .trim()
            .to_string();
        self.pos = close + 1;

        // Tolerate mismatched nesting rather than aborting: pop to the matching
        // open element when there is one, otherwise pop a single frame.
        if let Some(at) = self.open.iter().rposition(|n| *n == qname) {
            self.open.truncate(at);
            self.scopes.truncate(at);
        } else {
            self.open.pop();
            self.scopes.pop();
        }
        Some(Event::End(qname))
    }

    fn read_start_tag(&mut self) -> Option<Event> {
        self.pos += 1; // consume "<"
        let Some(close) = self.find_tag_end() else {
            return self.fail("unterminated start tag");
        };
        let body = &self.data[self.pos..close];
        self.pos = close + 1;

        let self_closing = body.last() == Some(&b'/');
        let body = if self_closing {
            &body[..body.len() - 1]
        } else {
            body
        };

        let text = String::from_utf8_lossy(body);
        let mut cursor = text.trim_start();
        let name_end = cursor
            .find(|c: char| c.is_whitespace())
            .unwrap_or(cursor.len());
        let qname = cursor[..name_end].to_string();
        cursor = &cursor[name_end..];

        if qname.is_empty() {
            return self.fail("empty element name");
        }
        if self.open.len() >= MAX_DEPTH {
            return self.fail(format!("element nesting deeper than {MAX_DEPTH}"));
        }

        let attrs = parse_attributes(cursor);

        // Namespace declarations on this element form a new scope, visible to
        // the element itself and its descendants.
        let mut frame = Vec::new();
        for (key, value) in &attrs {
            if key == "xmlns" {
                frame.push((String::new(), value.clone()));
            } else if let Some(prefix) = key.strip_prefix("xmlns:") {
                frame.push((prefix.to_string(), value.clone()));
            }
        }
        self.scopes.push(frame);
        self.open.push(qname.clone());

        let (prefix, local) = split_qname(&qname);
        let element = Element {
            ns: self.resolve(prefix),
            local: local.to_string(),
            qname: qname.clone(),
            attrs,
            self_closing,
        };

        if self_closing {
            self.pending_end = Some(qname);
        }
        Some(Event::Start(element))
    }

    /// Find the `>` that ends a start tag, ignoring any inside quoted attribute
    /// values (`<a href="a>b"/>` is legal).
    fn find_tag_end(&self) -> Option<usize> {
        let mut quote: Option<u8> = None;
        for at in self.pos..self.data.len() {
            let byte = self.data[at];
            match quote {
                Some(q) if byte == q => quote = None,
                Some(_) => {}
                None if byte == b'"' || byte == b'\'' => quote = Some(byte),
                None if byte == b'>' => return Some(at),
                None => {}
            }
        }
        None
    }

    fn skip_until(&mut self, needle: &[u8]) -> bool {
        match find(self.data, self.pos, needle) {
            Some(at) => {
                self.pos = at + needle.len();
                true
            }
            None => false,
        }
    }

    /// Skip `<!DOCTYPE ...>`, including any internal subset in `[ ... ]`.
    fn skip_doctype(&mut self) -> bool {
        let mut depth = 0usize;
        let mut at = self.pos;
        while at < self.data.len() {
            match self.data[at] {
                b'[' => depth += 1,
                b']' => depth = depth.saturating_sub(1),
                b'>' if depth == 0 => {
                    self.pos = at + 1;
                    return true;
                }
                _ => {}
            }
            at += 1;
        }
        false
    }
}

impl Iterator for Reader<'_> {
    type Item = Event;

    fn next(&mut self) -> Option<Event> {
        self.read_event()
    }
}

// ============================================================================
// Helpers
// ============================================================================

/// Split `prefix:local` into its parts; an unprefixed name yields `("", name)`.
pub fn split_qname(qname: &str) -> (&str, &str) {
    match qname.split_once(':') {
        Some((prefix, local)) => (prefix, local),
        None => ("", qname),
    }
}

/// The local part of a qualified name.
pub fn local_name(qname: &str) -> &str {
    split_qname(qname).1
}

/// Parse `key="value"` pairs from the remainder of a start tag.
fn parse_attributes(mut input: &str) -> Vec<(String, String)> {
    let mut attrs = Vec::new();
    loop {
        input = input.trim_start();
        if input.is_empty() {
            return attrs;
        }
        let Some(eq) = input.find('=') else {
            return attrs;
        };
        let key = input[..eq].trim().to_string();
        let rest = input[eq + 1..].trim_start();
        let Some(quote) = rest.chars().next() else {
            return attrs;
        };
        if quote != '"' && quote != '\'' {
            return attrs;
        }
        let rest = &rest[quote.len_utf8()..];
        let Some(end) = rest.find(quote) else {
            return attrs;
        };
        if !key.is_empty() {
            attrs.push((key, decode_entities(&rest[..end])));
        }
        input = &rest[end + quote.len_utf8()..];
    }
}

/// Expand the five predefined entities and numeric character references.
///
/// Anything else — including declared custom entities — is left verbatim, which
/// is what makes entity-expansion attacks impossible here.
pub fn decode_entities(input: &str) -> String {
    if !input.contains('&') {
        return input.to_string();
    }
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        let tail = &rest[amp..];
        // An entity reference is short; a bare '&' in text is common, so bound
        // the search rather than scanning to end of document.
        //
        // The bound is a count of *bytes* and the text is UTF-8, so it can land
        // inside a character. Slicing there panics, and it takes only an
        // ampersand with a multi-byte character eleven bytes after it -- one
        // real HTML page, with a lambda in it, brought the parser down that
        // way. Walk back to a boundary instead.
        let mut window = tail.len().min(12);
        while !tail.is_char_boundary(window) {
            window -= 1;
        }
        let Some(semi) = tail[..window].find(';') else {
            out.push('&');
            rest = &tail[1..];
            continue;
        };
        let body = &tail[1..semi];
        match body {
            "amp" => out.push('&'),
            "lt" => out.push('<'),
            "gt" => out.push('>'),
            "quot" => out.push('"'),
            "apos" => out.push('\''),
            _ => {
                let parsed = body.strip_prefix('#').and_then(|digits| {
                    let code = match digits.strip_prefix(['x', 'X']) {
                        Some(hex) => u32::from_str_radix(hex, 16).ok()?,
                        None => digits.parse::<u32>().ok()?,
                    };
                    char::from_u32(code)
                });
                match parsed {
                    Some(ch) => out.push(ch),
                    // Unknown entity: keep the source text so nothing is lost.
                    None => out.push_str(&tail[..=semi]),
                }
            }
        }
        rest = &tail[semi + 1..];
    }
    out.push_str(rest);
    out
}

/// Escape text for inclusion in XML/HTML character data.
pub fn escape(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(ch),
        }
    }
    out
}

/// Byte-substring search from `from`.
fn find(haystack: &[u8], from: usize, needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || from >= haystack.len() {
        return None;
    }
    haystack[from..]
        .windows(needle.len())
        .position(|w| w == needle)
        .map(|at| at + from)
}

/// Collect the concatenated text of the element that is currently open,
/// consuming events through its matching end tag.
///
/// Used constantly by the format parsers, where a run of text is split across
/// several child elements (`<w:t>`, `<a:t>`, `<text:span>`).
pub fn text_of(reader: &mut Reader, qname: &str) -> String {
    let mut out = String::new();
    let mut depth = 1usize;
    while let Some(event) = reader.read_event() {
        match event {
            Event::Start(el) if el.qname == qname => depth += 1,
            Event::End(name) if name == qname => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            Event::Text(text) => out.push_str(&text),
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {

    /// A bare ampersand near a multi-byte character does not panic.
    ///
    /// The entity search is bounded to twelve *bytes*, and a boundary walk is
    /// what keeps that bound from landing inside a character. Here the lambda
    /// occupies bytes 11 and 12 of the tail, so a naive slice at 12 splits it.
    ///
    /// Found by opening real documents rather than fixtures: hand-written test
    /// packages are all ASCII, and the parsers had never met a Greek letter.
    #[test]
    fn an_ampersand_near_a_multibyte_character_does_not_panic() {
        // 1 byte of '&' + 10 of ASCII puts the lambda across the bound.
        let input = "&abcdefghijλ and more";
        assert_eq!(decode_entities(input), input);

        // The same hazard from either side of the bound, and with the
        // character sitting exactly on it.
        for pad in 0..14 {
            let text = format!("&{}λ;x", "a".repeat(pad));
            let _ = decode_entities(&text);
        }

        // Real entities still decode with a multi-byte character alongside.
        assert_eq!(decode_entities("λ &amp; μ"), "λ & μ");
        assert_eq!(decode_entities("&#955;"), "λ");
    }
    use super::*;

    fn events(xml: &str) -> Vec<Event> {
        Reader::new(xml.as_bytes()).collect()
    }

    #[test]
    fn parses_elements_attrs_and_text() {
        let evts = events(r#"<root a="1" b='two'>hello</root>"#);
        assert_eq!(evts.len(), 3);
        match &evts[0] {
            Event::Start(el) => {
                assert_eq!(el.qname, "root");
                assert_eq!(el.attr("a"), Some("1"));
                assert_eq!(el.attr("b"), Some("two"));
                assert!(!el.self_closing);
            }
            other => panic!("expected start, got {other:?}"),
        }
        assert!(matches!(&evts[1], Event::Text(t) if t == "hello"));
        assert!(matches!(&evts[2], Event::End(n) if n == "root"));
    }

    #[test]
    fn self_closing_emits_balanced_start_and_end() {
        let evts = events("<a><br/></a>");
        assert_eq!(evts.len(), 4);
        assert!(matches!(&evts[1], Event::Start(el) if el.self_closing && el.local == "br"));
        assert!(matches!(&evts[2], Event::End(n) if n == "br"));
    }

    #[test]
    fn resolves_namespaces_from_declarations() {
        let xml = format!(
            r#"<w:document xmlns:w="{}" xmlns:a="{}"><w:t>doc</w:t><a:t>art</a:t></w:document>"#,
            ns::W,
            ns::A
        );
        let mut reader = Reader::new(xml.as_bytes());
        let mut seen = Vec::new();
        while let Some(event) = reader.read_event() {
            if let Event::Start(el) = event {
                seen.push((el.ns.clone(), el.local.clone()));
            }
        }
        assert_eq!(
            seen,
            vec![
                (ns::W.to_string(), "document".to_string()),
                (ns::W.to_string(), "t".to_string()),
                (ns::A.to_string(), "t".to_string()),
            ]
        );
    }

    #[test]
    fn same_local_name_in_different_namespaces_is_distinguishable() {
        // The exact case that makes prefix-matching wrong: Word text and
        // DrawingML text both have local name "t".
        let xml = format!(
            r#"<r xmlns:w="{}" xmlns:a="{}"><w:t>body</w:t><a:t>shape</a:t></r>"#,
            ns::W,
            ns::A
        );
        let mut reader = Reader::new(xml.as_bytes());
        let mut word_text = String::new();
        while let Some(event) = reader.read_event() {
            if let Event::Start(el) = event
                && el.is(ns::W, "t")
            {
                word_text = text_of(&mut reader, &el.qname);
            }
        }
        assert_eq!(word_text, "body");
    }

    #[test]
    fn inner_scope_shadows_outer_prefix_binding() {
        let xml = r#"<a xmlns:p="urn:outer"><p:x/><b xmlns:p="urn:inner"><p:y/></b></a>"#;
        let mut reader = Reader::new(xml.as_bytes());
        let mut found = Vec::new();
        while let Some(event) = reader.read_event() {
            if let Event::Start(el) = event
                && (el.local == "x" || el.local == "y")
            {
                found.push((el.local.clone(), el.ns.clone()));
            }
        }
        assert_eq!(
            found,
            vec![
                ("x".to_string(), "urn:outer".to_string()),
                ("y".to_string(), "urn:inner".to_string()),
            ]
        );
    }

    #[test]
    fn default_namespace_applies_to_unprefixed_elements() {
        let xml = format!(r#"<html xmlns="{}"><p>hi</p></html>"#, ns::XHTML);
        let mut reader = Reader::new(xml.as_bytes());
        let Some(Event::Start(el)) = reader.read_event() else {
            panic!("expected start")
        };
        assert!(el.in_ns(ns::XHTML));
    }

    #[test]
    fn skips_comments_declarations_and_pis() {
        let xml = r#"<?xml version="1.0"?><!DOCTYPE t><!-- note --><t>x</t>"#;
        let evts = events(xml);
        assert_eq!(evts.len(), 3);
        assert!(matches!(&evts[1], Event::Text(t) if t == "x"));
    }

    #[test]
    fn reads_cdata_as_text() {
        let evts = events("<t><![CDATA[a < b & c]]></t>");
        assert!(matches!(&evts[1], Event::Text(t) if t == "a < b & c"));
    }

    #[test]
    fn decodes_entities_in_text_and_attributes() {
        let evts = events(r#"<t v="a&amp;b">&lt;tag&gt; &#65;&#x42;</t>"#);
        match &evts[0] {
            Event::Start(el) => assert_eq!(el.attr("v"), Some("a&b")),
            other => panic!("expected start, got {other:?}"),
        }
        assert!(matches!(&evts[1], Event::Text(t) if t == "<tag> AB"));
    }

    #[test]
    fn leaves_unknown_and_bare_ampersands_alone() {
        // A declared custom entity is never expanded — that is the XXE defence.
        assert_eq!(decode_entities("&xxe; a & b"), "&xxe; a & b");
    }

    #[test]
    fn tolerates_angle_bracket_inside_attribute_value() {
        let evts = events(r#"<a href="x?p=1&amp;q=a>b"/>"#);
        match &evts[0] {
            Event::Start(el) => assert_eq!(el.attr("href"), Some("x?p=1&q=a>b")),
            other => panic!("expected start, got {other:?}"),
        }
    }

    #[test]
    fn reports_whitespace_only_runs_rather_than_dropping_them() {
        // Between elements this is formatting noise, but inside
        // `xml:space="preserve"` it is a significant space. The reader reports
        // it either way; the caller decides.
        let evts = events("<a>\n  <b/>\n</a>");
        assert!(
            evts.iter()
                .any(|e| matches!(e, Event::Text(t) if t.trim().is_empty()))
        );
    }

    #[test]
    fn preserves_a_lone_space_as_element_content() {
        let mut reader = Reader::new(br#"<t xml:space="preserve"> </t>"#);
        let Some(Event::Start(el)) = reader.read_event() else {
            panic!("expected start")
        };
        assert_eq!(el.attr("xml:space"), Some("preserve"));
        assert_eq!(text_of(&mut reader, &el.qname), " ");
    }

    #[test]
    fn text_of_collects_across_nested_children() {
        let mut reader = Reader::new(b"<p>a<b>c</b>d</p>rest");
        let Some(Event::Start(el)) = reader.read_event() else {
            panic!("expected start")
        };
        assert_eq!(text_of(&mut reader, &el.qname), "acd");
    }

    #[test]
    fn attr_local_ignores_prefix() {
        let evts = events(r#"<img r:embed="rId7"/>"#);
        match &evts[0] {
            Event::Start(el) => {
                assert_eq!(el.attr_local("embed"), Some("rId7"));
                assert_eq!(el.attr("r:embed"), Some("rId7"));
            }
            other => panic!("expected start, got {other:?}"),
        }
    }

    #[test]
    fn truncated_markup_keeps_earlier_events_and_reports_error() {
        let mut reader = Reader::new(b"<a>text</a><b attr=");
        let collected: Vec<Event> = reader.by_ref().collect();
        assert_eq!(collected.len(), 3);
        assert!(reader.error().is_some());
    }

    #[test]
    fn caps_nesting_depth() {
        let deep = "<a>".repeat(MAX_DEPTH + 10);
        let mut reader = Reader::new(deep.as_bytes());
        let count = reader.by_ref().count();
        assert_eq!(count, MAX_DEPTH);
        assert!(reader.error().unwrap().contains("nesting"));
    }

    #[test]
    fn mismatched_end_tag_does_not_derail_parsing() {
        let evts = events("<a><b>x</a><c>y</c>");
        // The stray </a> closes both open elements; parsing continues.
        assert!(evts.iter().any(|e| matches!(e, Event::Text(t) if t == "y")));
    }

    #[test]
    fn strips_byte_order_mark() {
        let mut input = vec![0xEF, 0xBB, 0xBF];
        input.extend_from_slice(b"<t>x</t>");
        let mut reader = Reader::new(&input);
        assert!(matches!(reader.read_event(), Some(Event::Start(el)) if el.local == "t"));
    }

    #[test]
    fn escape_round_trips_through_decode() {
        let raw = r#"a & b < c > d " e"#;
        assert_eq!(decode_entities(&escape(raw)), raw);
    }
}
