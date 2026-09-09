//! RDF terms, modelled without losing anything.
//!
//! This exists because of two defects observed in the EXAMPLE-ONTO-000 compiler
//! skeleton, both of which are cheap to avoid at the start and expensive to
//! retrofit:
//!
//! 1. **Its term model had no blank nodes.** `Term` was `Iri | Literal`. Every
//!    SHACL shape in that repository is built from blank nodes —
//!    `sh:property [ sh:path … ]` — so the model could not represent the files
//!    it existed to validate.
//!
//! 2. **Compilation discarded datatype and language.** A parsed
//!    `Literal { lexical, datatype, language }` was compiled down to a bare
//!    `String`, after which `"1.0"^^xsd:decimal` and the string `"1.0"` were
//!    indistinguishable and every datatype constraint downstream became
//!    uncheckable.
//!
//! So: blank nodes exist here from the first commit, and a literal keeps its
//! datatype and language for its whole life. A lossy conversion is not
//! provided, because the way that defect arrives is someone reaching for the
//! convenient one.

use serde::{Deserialize, Serialize};

use crate::iri::Iri;

/// A label for a node that has no IRI.
///
/// Scoped to the graph that produced it. Two blank nodes with the same label
/// from different documents are different nodes, which is why canonicalisation
/// ([`crate::fingerprint`]) relabels rather than trusting these.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct BlankNode(pub String);

impl BlankNode {
    pub fn new(label: impl Into<String>) -> Self {
        Self(label.into())
    }
}

/// A literal value, with everything that distinguishes it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Literal {
    pub lexical: String,
    /// The datatype IRI. `None` means a plain string (`xsd:string`).
    pub datatype: Option<Iri>,
    /// BCP-47 language tag, only ever present on plain strings.
    pub language: Option<String>,
}

impl Literal {
    /// A plain string literal.
    pub fn plain(s: impl Into<String>) -> Self {
        Self {
            lexical: s.into(),
            datatype: None,
            language: None,
        }
    }

    /// A typed literal.
    pub fn typed(s: impl Into<String>, datatype: Iri) -> Self {
        Self {
            lexical: s.into(),
            datatype: Some(datatype),
            language: None,
        }
    }

    /// A language-tagged string.
    ///
    /// RDF forbids carrying both a language tag and a datatype other than
    /// `rdf:langString`, so this constructor takes no datatype rather than
    /// letting a caller build a term that cannot exist.
    pub fn lang(s: impl Into<String>, tag: impl Into<String>) -> Self {
        Self {
            lexical: s.into(),
            datatype: None,
            language: Some(tag.into()),
        }
    }

    /// Whether this is a number, by declared datatype.
    ///
    /// Answers from the datatype rather than by trying to parse the lexical
    /// form: `"12"` typed as `xsd:string` is not a number, and treating it as
    /// one is how a string field silently satisfies a range constraint.
    pub fn is_numeric(&self) -> bool {
        self.datatype.as_ref().is_some_and(|d| {
            matches!(
                d.as_str(),
                "http://www.w3.org/2001/XMLSchema#decimal"
                    | "http://www.w3.org/2001/XMLSchema#double"
                    | "http://www.w3.org/2001/XMLSchema#float"
                    | "http://www.w3.org/2001/XMLSchema#integer"
                    | "http://www.w3.org/2001/XMLSchema#long"
                    | "http://www.w3.org/2001/XMLSchema#int"
                    | "http://www.w3.org/2001/XMLSchema#nonNegativeInteger"
            )
        })
    }

    /// The numeric value, if this literal is a number and parses as one.
    pub fn as_f64(&self) -> Option<f64> {
        self.is_numeric()
            .then(|| self.lexical.parse().ok())
            .flatten()
    }
}

/// Anything that can stand in the object position of a statement.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Term {
    Iri(Iri),
    Blank(BlankNode),
    Literal(Literal),
}

impl Term {
    pub fn as_iri(&self) -> Option<&Iri> {
        match self {
            Term::Iri(i) => Some(i),
            _ => None,
        }
    }

    pub fn as_literal(&self) -> Option<&Literal> {
        match self {
            Term::Literal(l) => Some(l),
            _ => None,
        }
    }
}

impl From<Iri> for Term {
    fn from(i: Iri) -> Self {
        Term::Iri(i)
    }
}

impl From<Literal> for Term {
    fn from(l: Literal) -> Self {
        Term::Literal(l)
    }
}

impl From<BlankNode> for Term {
    fn from(b: BlankNode) -> Self {
        Term::Blank(b)
    }
}

/// What can stand in the subject position: an IRI or a blank node.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Subject {
    Iri(Iri),
    Blank(BlankNode),
}

impl From<Subject> for Term {
    fn from(s: Subject) -> Self {
        match s {
            Subject::Iri(i) => Term::Iri(i),
            Subject::Blank(b) => Term::Blank(b),
        }
    }
}

/// One statement, in a named graph.
///
/// The graph slot is present from the start because the config in the example
/// architecture declared `emit_nquads = true` over a triple type with nowhere
/// to put a graph name. N-Quads is quads; provenance is a stated first-class
/// concern; retrofitting a fourth position through every construction site is
/// the expensive version of this decision.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Quad {
    pub subject: Subject,
    pub predicate: Iri,
    pub object: Term,
    /// `None` is the default graph.
    pub graph: Option<Iri>,
}

impl Quad {
    /// A statement in the default graph.
    pub fn new(subject: impl Into<Subject>, predicate: Iri, object: impl Into<Term>) -> Self {
        Self {
            subject: subject.into(),
            predicate,
            object: object.into(),
            graph: None,
        }
    }

    /// Place this statement in a named graph.
    pub fn in_graph(mut self, g: Iri) -> Self {
        self.graph = Some(g);
        self
    }
}

impl From<Iri> for Subject {
    fn from(i: Iri) -> Self {
        Subject::Iri(i)
    }
}

impl From<BlankNode> for Subject {
    fn from(b: BlankNode) -> Self {
        Subject::Blank(b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::iri::xsd;

    #[test]
    fn a_typed_number_and_a_string_that_looks_like_one_are_different_terms() {
        let n = Literal::typed("1.0", xsd::DECIMAL.clone());
        let s = Literal::plain("1.0");
        assert_ne!(n, s);
        assert!(n.is_numeric());
        assert!(
            !s.is_numeric(),
            "a plain string must not satisfy a numeric constraint just because it parses"
        );
        assert_eq!(n.as_f64(), Some(1.0));
        assert_eq!(s.as_f64(), None);
    }

    #[test]
    fn blank_nodes_exist_and_can_be_subjects() {
        // The example architecture could not represent its own SHACL files for
        // want of this.
        let q = Quad::new(
            BlankNode::new("b0"),
            crate::iri::rdf::TYPE.clone(),
            Iri::parse("https://ontology.nervosys.com/core/Entity").unwrap(),
        );
        assert!(matches!(q.subject, Subject::Blank(_)));
    }

    #[test]
    fn a_quad_can_name_its_graph() {
        let g = Iri::parse("https://ontology.nervosys.com/graph/provenance").unwrap();
        let q = Quad::new(
            Iri::parse("https://example.org/a").unwrap(),
            crate::iri::rdf::TYPE.clone(),
            Iri::parse("https://example.org/B").unwrap(),
        )
        .in_graph(g.clone());
        assert_eq!(q.graph, Some(g));
    }

    #[test]
    fn a_language_tagged_literal_carries_no_datatype() {
        let l = Literal::lang("bonjour", "fr");
        assert_eq!(l.language.as_deref(), Some("fr"));
        assert!(
            l.datatype.is_none(),
            "langString and a datatype cannot coexist"
        );
    }
}
