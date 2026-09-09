//! A content fingerprint that actually depends on the content.
//!
//! # The failure this is written against
//!
//! The EXAMPLE-ONTO-000 compiler skeleton computed a SHA-256 over a graph that was
//! never loaded. Measured: appending invalid Turtle to a source file and then
//! deleting an entire source module both produced the identical digest,
//! `55c6d543…`. A fingerprint exists to certify that two parties hold the same
//! graph; that one certified that any two ontologies were the same ontology.
//!
//! Two rules follow, and both are enforced by tests in this module rather than
//! by intention:
//!
//! 1. **Different content must produce a different digest.** Trivial to state,
//!    and the thing nobody checked.
//! 2. **A fingerprint over nothing must be impossible to mistake for a
//!    fingerprint over something.** [`fingerprint`] returns
//!    [`Fingerprint::Empty`] rather than a hex string for an empty graph, so a
//!    caller cannot print it beside a real one and see the same shape.
//!
//! # Canonicalisation, and the limit of it
//!
//! Hashing `serde_json` output — what the example did — is not canonical for
//! RDF: statement order and blank-node labels vary between runs that mean the
//! same thing. Here, quads are serialised in a stable N-Quads-like form and
//! sorted, and blank nodes are relabelled by their position in that sort.
//!
//! **This is not RDFC-1.0.** It is deterministic for graphs whose blank nodes
//! are distinguishable by their surrounding statements, and it does not
//! resolve the hard cases full canonicalisation exists for — automorphic
//! blank-node cycles. [`Fingerprint::canonicalisation`] says which algorithm
//! produced a digest so two parties can tell whether they are comparing like
//! with like, and the limitation is named here rather than discovered later.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::term::{Quad, Subject, Term};

/// The algorithm used, carried alongside the digest.
///
/// Present so that adopting RDFC-1.0 later is a visible change rather than a
/// silent one that makes old and new digests incomparable without warning.
pub const CANONICALISATION: &str = "nervosys-sorted-nquads-v1";

/// The result of fingerprinting a graph.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum Fingerprint {
    /// There were no quads. Deliberately not a digest.
    Empty,
    /// A digest over a non-empty graph.
    Digest {
        sha256: String,
        canonicalisation: &'static str,
        quads: usize,
    },
}

impl Fingerprint {
    /// The hex digest, if there was anything to digest.
    pub fn hex(&self) -> Option<&str> {
        match self {
            Fingerprint::Empty => None,
            Fingerprint::Digest { sha256, .. } => Some(sha256),
        }
    }

    pub fn is_empty(&self) -> bool {
        matches!(self, Fingerprint::Empty)
    }

    /// How it was canonicalised, for comparison across parties.
    pub fn canonicalisation(&self) -> Option<&'static str> {
        match self {
            Fingerprint::Empty => None,
            Fingerprint::Digest {
                canonicalisation, ..
            } => Some(canonicalisation),
        }
    }
}

impl std::fmt::Display for Fingerprint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // Loud on purpose. An empty graph should never scroll past looking
            // like a successful build.
            Fingerprint::Empty => write!(f, "EMPTY-GRAPH (nothing was loaded)"),
            Fingerprint::Digest { sha256, quads, .. } => write!(f, "{sha256} ({quads} quads)"),
        }
    }
}

/// Serialise one quad in a stable, unambiguous form.
///
/// Datatype and language are included. Dropping them — as the example's
/// compile step did — would make `"1.0"^^xsd:decimal` and the string `"1.0"`
/// hash identically, so a datatype change would not move the fingerprint.
fn write_quad(q: &Quad, label: impl Fn(&str) -> usize) -> String {
    let subject = match &q.subject {
        Subject::Iri(i) => format!("<{i}>"),
        Subject::Blank(b) => format!("_:c14n{}", label(&b.0)),
    };
    let object = match &q.object {
        Term::Iri(i) => format!("<{i}>"),
        Term::Blank(b) => format!("_:c14n{}", label(&b.0)),
        Term::Literal(l) => {
            let mut s = format!("{:?}", l.lexical);
            if let Some(lang) = &l.language {
                s.push('@');
                s.push_str(lang);
            } else if let Some(dt) = &l.datatype {
                s.push_str("^^<");
                s.push_str(dt.as_str());
                s.push('>');
            }
            s
        }
    };
    let graph = q
        .graph
        .as_ref()
        .map(|g| format!(" <{g}>"))
        .unwrap_or_default();
    format!("{subject} <{}> {object}{graph} .", q.predicate)
}

/// Fingerprint a graph.
pub fn fingerprint(quads: &[Quad]) -> Fingerprint {
    if quads.is_empty() {
        return Fingerprint::Empty;
    }

    // Pass one: serialise with blank labels held as-is, to derive a stable
    // ordering that does not depend on the order the caller supplied.
    let mut provisional: Vec<(String, &Quad)> = quads
        .iter()
        .map(|q| (write_quad(q, |b| b.len()), q))
        .collect();
    provisional.sort_by(|a, b| a.0.cmp(&b.0));

    // Pass two: assign canonical blank-node labels by first appearance in the
    // sorted order, then re-serialise so the labels the caller chose cannot
    // affect the digest.
    let mut order: Vec<String> = Vec::new();
    let mut note = |b: &str| {
        if !order.iter().any(|x| x == b) {
            order.push(b.to_string());
        }
    };
    for (_, q) in &provisional {
        if let Subject::Blank(b) = &q.subject {
            note(&b.0);
        }
        if let Term::Blank(b) = &q.object {
            note(&b.0);
        }
    }
    let label = |b: &str| order.iter().position(|x| x == b).unwrap_or(usize::MAX);

    let mut lines: Vec<String> = quads.iter().map(|q| write_quad(q, label)).collect();
    lines.sort();
    lines.dedup(); // a graph is a set; duplicates carry no meaning

    let mut h = Sha256::new();
    for line in &lines {
        h.update(line.as_bytes());
        h.update(b"\n");
    }

    Fingerprint::Digest {
        sha256: hex_encode(&h.finalize()),
        canonicalisation: CANONICALISATION,
        quads: lines.len(),
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::iri::{xsd, Iri, CORE};
    use crate::term::{BlankNode, Literal};

    fn iri(s: &str) -> Iri {
        Iri::parse(s).unwrap()
    }

    fn sample() -> Vec<Quad> {
        vec![
            Quad::new(
                iri("https://example.org/a"),
                CORE.term("name"),
                Literal::plain("Atlas"),
            ),
            Quad::new(
                iri("https://example.org/a"),
                CORE.term("status"),
                Literal::plain("operational"),
            ),
        ]
    }

    /// The test the example architecture did not have. Everything else here is
    /// secondary to this one.
    #[test]
    fn the_fingerprint_can_fail_to_match() {
        let a = fingerprint(&sample());
        let mut changed = sample();
        changed[0] = Quad::new(
            iri("https://example.org/a"),
            CORE.term("name"),
            Literal::plain("Atlas II"),
        );
        let b = fingerprint(&changed);
        assert_ne!(a, b, "changing a literal must change the digest");
        assert_ne!(a.hex(), b.hex());
    }

    #[test]
    fn removing_a_statement_changes_the_digest() {
        // The example survived deletion of an entire ontology module unchanged.
        let full = fingerprint(&sample());
        let partial = fingerprint(&sample()[..1]);
        assert_ne!(full, partial);
    }

    #[test]
    fn an_empty_graph_is_not_given_a_digest() {
        let f = fingerprint(&[]);
        assert!(f.is_empty());
        assert_eq!(
            f.hex(),
            None,
            "an empty graph must not produce a hex digest"
        );
        assert!(
            f.to_string().contains("EMPTY-GRAPH"),
            "and it must be loud: {f}"
        );
    }

    #[test]
    fn statement_order_does_not_affect_the_digest() {
        let mut reversed = sample();
        reversed.reverse();
        assert_eq!(fingerprint(&sample()), fingerprint(&reversed));
    }

    #[test]
    fn duplicate_statements_do_not_affect_the_digest() {
        // An RDF graph is a set. The example's validator *errored* on
        // duplicates; the correct handling is to ignore them.
        let mut dup = sample();
        dup.push(sample()[0].clone());
        assert_eq!(fingerprint(&sample()), fingerprint(&dup));
    }

    #[test]
    fn blank_node_labels_do_not_affect_the_digest() {
        let build = |label: &str| {
            vec![Quad::new(
                BlankNode::new(label),
                CORE.term("name"),
                Literal::plain("x"),
            )]
        };
        assert_eq!(
            fingerprint(&build("b0")),
            fingerprint(&build("someOtherLabel")),
            "the same graph written with different blank labels is the same graph"
        );
    }

    #[test]
    fn datatype_is_part_of_the_digest() {
        // The example dropped datatype during compilation, so this distinction
        // vanished before hashing.
        let plain = vec![Quad::new(
            iri("https://example.org/a"),
            CORE.term("confidence"),
            Literal::plain("1.0"),
        )];
        let typed = vec![Quad::new(
            iri("https://example.org/a"),
            CORE.term("confidence"),
            Literal::typed("1.0", xsd::DECIMAL.clone()),
        )];
        assert_ne!(fingerprint(&plain), fingerprint(&typed));
    }

    #[test]
    fn the_graph_name_is_part_of_the_digest() {
        let default = sample();
        let named: Vec<_> = sample()
            .into_iter()
            .map(|q| q.in_graph(iri("https://example.org/g")))
            .collect();
        assert_ne!(fingerprint(&default), fingerprint(&named));
    }

    #[test]
    fn a_digest_reports_how_it_was_canonicalised() {
        let f = fingerprint(&sample());
        assert_eq!(f.canonicalisation(), Some(CANONICALISATION));
    }
}
