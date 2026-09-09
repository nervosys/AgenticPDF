//! Reading N-Quads back into a graph.
//!
//! The hub ingests what spokes emit, so the vocabulary needs a reader as well
//! as a writer. N-Quads is the ingest format on purpose: it is line-oriented
//! and self-contained, where JSON-LD would require context resolution and
//! expansion before a single statement could be trusted.
//!
//! # Honest about its limits
//!
//! This handles the profile [`crate::emit::to_nquads`] produces and the common
//! subset beyond it: IRIs, blank nodes, plain literals, typed literals,
//! language-tagged literals, an optional graph name, and comments. It does
//! **not** implement every escape sequence in the RDF 1.1 grammar.
//!
//! Where it cannot parse a line it returns an error naming the line number,
//! rather than skipping it. A parser that silently drops what it does not
//! understand turns a malformed graph into a smaller valid-looking one — which
//! is the same failure as a validator that cannot fail, arriving one layer
//! earlier.

use crate::iri::Iri;
use crate::term::{BlankNode, Literal, Quad, Subject, Term};

/// Why a line could not be parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    /// 1-indexed line number in the input.
    pub line: usize,
    pub message: String,
    /// The offending line, truncated.
    pub text: String,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "line {}: {} — {}", self.line, self.message, self.text)
    }
}

impl std::error::Error for ParseError {}

/// Parse an N-Quads document.
///
/// Returns the first error rather than a partial graph: a caller that receives
/// `Ok` holds every statement in the input, and one that receives `Err` knows
/// not to reason about what did parse.
pub fn parse_nquads(input: &str) -> Result<Vec<Quad>, ParseError> {
    let mut out = Vec::new();
    for (i, raw) in input.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        out.push(parse_line(line).map_err(|message| ParseError {
            line: i + 1,
            message,
            text: raw.chars().take(120).collect(),
        })?);
    }
    Ok(out)
}

fn parse_line(line: &str) -> Result<Quad, String> {
    let body = line
        .strip_suffix('.')
        .ok_or_else(|| "statement does not end with '.'".to_string())?
        .trim_end();

    let mut cur = body.trim_start();

    let (subject, rest) = take_term(cur)?;
    cur = rest.trim_start();
    let (predicate, rest) = take_term(cur)?;
    cur = rest.trim_start();
    let (object, rest) = take_term(cur)?;
    cur = rest.trim_start();

    let graph = if cur.is_empty() {
        None
    } else {
        let (g, rest) = take_term(cur)?;
        if !rest.trim().is_empty() {
            return Err(format!("trailing content after graph name: {rest:?}"));
        }
        match g {
            Raw::Iri(i) => Some(i),
            _ => return Err("graph name must be an IRI".into()),
        }
    };

    let subject = match subject {
        Raw::Iri(i) => Subject::Iri(i),
        Raw::Blank(b) => Subject::Blank(b),
        Raw::Literal(_) => return Err("a literal cannot be a subject".into()),
    };
    let predicate = match predicate {
        Raw::Iri(i) => i,
        _ => return Err("a predicate must be an IRI".into()),
    };
    let object = match object {
        Raw::Iri(i) => Term::Iri(i),
        Raw::Blank(b) => Term::Blank(b),
        Raw::Literal(l) => Term::Literal(l),
    };

    Ok(Quad {
        subject,
        predicate,
        object,
        graph,
    })
}

enum Raw {
    Iri(Iri),
    Blank(BlankNode),
    Literal(Literal),
}

/// Consume one term from the front, returning it and the remainder.
fn take_term(s: &str) -> Result<(Raw, &str), String> {
    let s = s.trim_start();
    let mut chars = s.char_indices();
    match chars.next() {
        Some((_, '<')) => {
            let end = s.find('>').ok_or("unterminated IRI: no closing '>'")?;
            let iri = Iri::parse(&s[1..end]).map_err(|e| e.to_string())?;
            Ok((Raw::Iri(iri), &s[end + 1..]))
        }
        Some((_, '_')) => {
            let rest = s
                .strip_prefix("_:")
                .ok_or("expected '_:' for a blank node")?;
            let end = rest.find(|c: char| c.is_whitespace()).unwrap_or(rest.len());
            if end == 0 {
                return Err("blank node has no label".into());
            }
            Ok((Raw::Blank(BlankNode::new(&rest[..end])), &rest[end..]))
        }
        Some((_, '"')) => {
            let (lexical, after) = take_quoted(s)?;
            // Optional language tag or datatype.
            if let Some(rest) = after.strip_prefix("^^") {
                let (dt, rest) = take_term(rest)?;
                match dt {
                    Raw::Iri(i) => Ok((Raw::Literal(Literal::typed(lexical, i)), rest)),
                    _ => Err("datatype must be an IRI".into()),
                }
            } else if let Some(rest) = after.strip_prefix('@') {
                let end = rest.find(|c: char| c.is_whitespace()).unwrap_or(rest.len());
                if end == 0 {
                    return Err("empty language tag".into());
                }
                Ok((
                    Raw::Literal(Literal::lang(lexical, &rest[..end])),
                    &rest[end..],
                ))
            } else {
                Ok((Raw::Literal(Literal::plain(lexical)), after))
            }
        }
        Some((_, c)) => Err(format!("unexpected character {c:?} at start of term")),
        None => Err("expected a term, found end of line".into()),
    }
}

/// Read a `"..."` literal, honouring backslash escapes.
fn take_quoted(s: &str) -> Result<(String, &str), String> {
    let bytes = s.as_bytes();
    debug_assert_eq!(bytes[0], b'"');
    let mut out = String::new();
    let mut i = 1usize;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => {
                i += 1;
                let e = *bytes.get(i).ok_or("string ends in a backslash")?;
                out.push(match e {
                    b'n' => '\n',
                    b'r' => '\r',
                    b't' => '\t',
                    b'"' => '"',
                    b'\\' => '\\',
                    other => return Err(format!("unsupported escape \\{}", other as char)),
                });
                i += 1;
            }
            b'"' => return Ok((out, &s[i + 1..])),
            _ => {
                // Step by whole characters so multi-byte UTF-8 survives.
                let ch = s[i..].chars().next().ok_or("malformed UTF-8 in literal")?;
                out.push(ch);
                i += ch.len_utf8();
            }
        }
    }
    Err("unterminated string literal".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::emit::to_nquads;
    use crate::iri::{xsd, CORE};

    fn iri(s: &str) -> Iri {
        Iri::parse(s).unwrap()
    }

    /// The property that matters: what the emitter writes, the parser reads
    /// back unchanged. Without this the hub and the spokes disagree silently.
    #[test]
    fn emit_then_parse_round_trips() {
        let original = vec![
            Quad::new(
                iri("https://example.org/a"),
                CORE.term("name"),
                Literal::plain("Atlas"),
            ),
            Quad::new(
                iri("https://example.org/a"),
                CORE.term("confidence"),
                Literal::typed("0.80", xsd::DECIMAL.clone()),
            ),
            Quad::new(
                BlankNode::new("b0"),
                CORE.term("status"),
                Literal::lang("prêt", "fr"),
            ),
            Quad::new(
                iri("https://example.org/a"),
                CORE.term("hasPart"),
                iri("https://example.org/b"),
            )
            .in_graph(iri("https://example.org/g")),
        ];
        let text = to_nquads(&original);
        let parsed = parse_nquads(&text).expect("round trip parses");

        let mut a = original.clone();
        let mut b = parsed;
        a.sort();
        b.sort();
        assert_eq!(a, b, "emit -> parse lost or altered a statement");
    }

    #[test]
    fn a_real_emitted_manifest_round_trips() {
        let m = crate::test_support::manifest(vec![crate::Capability::read(
            "x.get", "GET", "/x", "read",
        )]);
        let quads = crate::emit::to_quads(&m);
        let parsed = parse_nquads(&to_nquads(&quads)).expect("parses");
        assert_eq!(parsed.len(), quads.len());
        // And the fingerprint agrees, which is the check the hub actually runs.
        assert_eq!(
            crate::fingerprint::fingerprint(&quads),
            crate::fingerprint::fingerprint(&parsed)
        );
    }

    #[test]
    fn a_malformed_line_is_an_error_not_a_silent_drop() {
        // The failure this parser is written against: quietly discarding what
        // it cannot read turns a broken graph into a smaller valid-looking one.
        let bad = "<https://example.org/a> <https://example.org/p> \"unterminated .\n";
        let e = parse_nquads(bad).expect_err("must not silently skip");
        assert_eq!(e.line, 1);

        let no_dot = "<https://example.org/a> <https://example.org/p> <https://example.org/b>\n";
        assert!(parse_nquads(no_dot).is_err());
    }

    #[test]
    fn comments_and_blank_lines_are_skipped_but_statements_are_not() {
        let text = "# a comment\n\n<https://example.org/a> <https://example.org/p> \"v\" .\n";
        let q = parse_nquads(text).unwrap();
        assert_eq!(q.len(), 1);
    }

    #[test]
    fn a_literal_subject_is_rejected() {
        let bad = "\"nope\" <https://example.org/p> \"v\" .";
        assert!(parse_nquads(bad).is_err());
    }

    #[test]
    fn escapes_and_multibyte_survive() {
        let text = r#"<https://example.org/a> <https://example.org/p> "line\nbreak — é" ."#;
        let q = parse_nquads(text).unwrap();
        let l = q[0].object.as_literal().unwrap();
        assert_eq!(l.lexical, "line\nbreak — é");
    }
}
