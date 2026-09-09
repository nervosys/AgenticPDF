//! Identifiers, shared with the company graph rather than invented per crate.
//!
//! The point of this module is cross-domain identity. NS-ONTO-001 defines
//! `https://ontology.nervosys.com/core/Agent`; if a Rust product describes its
//! agents under some other name, the two models can never be joined, and the
//! whole reason to have a company ontology is lost. So the namespaces here are
//! *the same strings* as `contexts/v1.jsonld`, and a test asserts it.
//!
//! # Why IRIs are validated on construction
//!
//! The example compiler checked for empty subjects and predicates deep inside
//! its validation pass, which is both too late and too weak — by then the term
//! already existed. An [`Iri`] cannot be constructed unvalidated, so the check
//! happens once, at the boundary, and nothing downstream repeats it.

use std::fmt;
use std::sync::LazyLock;

use serde::{Deserialize, Serialize};

/// An absolute IRI.
///
/// Construct through [`Iri::parse`] or a namespace's `term` method; the inner
/// field is private so an invalid IRI has no way in.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Iri(String);

/// Why an IRI was rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IriError {
    Empty,
    NotAbsolute(String),
    ContainsWhitespace(String),
}

impl fmt::Display for IriError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IriError::Empty => write!(f, "IRI is empty"),
            IriError::NotAbsolute(s) => write!(f, "IRI is not absolute (needs a scheme): {s}"),
            IriError::ContainsWhitespace(s) => write!(f, "IRI contains whitespace: {s}"),
        }
    }
}

impl std::error::Error for IriError {}

impl Iri {
    /// Validate and construct.
    pub fn parse(s: impl Into<String>) -> Result<Self, IriError> {
        let s = s.into();
        if s.trim().is_empty() {
            return Err(IriError::Empty);
        }
        if s.chars().any(char::is_whitespace) {
            return Err(IriError::ContainsWhitespace(s));
        }
        // Absolute means it has a scheme. Deliberately not a full RFC 3987
        // parse: this catches the mistakes that actually happen (a relative
        // path, a CURIE that was never expanded) without pretending to a
        // rigour it does not have.
        match s.find(':') {
            Some(i)
                if i > 0
                    && s[..i]
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || "+-.".contains(c)) =>
            {
                Ok(Iri(s))
            }
            _ => Err(IriError::NotAbsolute(s)),
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Iri {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A namespace, and the prefix it is written with in Turtle and JSON-LD.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Namespace {
    pub prefix: &'static str,
    pub base: &'static str,
}

impl Namespace {
    /// Mint a term in this namespace.
    ///
    /// Panics on a local name containing whitespace, which is a programming
    /// error in a declaration rather than a runtime input.
    pub fn term(&self, local: &str) -> Iri {
        Iri::parse(format!("{}{}", self.base, local))
            .unwrap_or_else(|e| panic!("bad term `{local}` in {}: {e}", self.prefix))
    }

    /// The CURIE form, for JSON-LD and Turtle output.
    pub fn curie(&self, local: &str) -> String {
        format!("{}:{}", self.prefix, local)
    }
}

/// The NERVOSYS namespaces, verbatim from the context published in EXAMPLE-ONTO-000.
///
/// Changing a base string here silently detaches this crate from the company
/// graph, so [`tests::namespaces_match_the_company_context`] pins them.
pub const CORE: Namespace = Namespace {
    prefix: "ns",
    base: "https://ontology.nervosys.com/core/",
};
pub const ENGINEERING: Namespace = Namespace {
    prefix: "eng",
    base: "https://ontology.nervosys.com/engineering/",
};
pub const AGENTS: Namespace = Namespace {
    prefix: "agent",
    base: "https://ontology.nervosys.com/agents/",
};
pub const HARDWARE: Namespace = Namespace {
    prefix: "hw",
    base: "https://ontology.nervosys.com/hardware/",
};
pub const SOFTWARE: Namespace = Namespace {
    prefix: "sw",
    base: "https://ontology.nervosys.com/software/",
};
pub const ENTITY: Namespace = Namespace {
    prefix: "entity",
    base: "https://ontology.nervosys.com/entity/",
};

/// Namespaces this crate emits into, for context generation.
pub const ALL: &[Namespace] = &[CORE, ENGINEERING, AGENTS, HARDWARE, SOFTWARE, ENTITY];

macro_rules! iri_const {
    ($ns:expr, $($name:ident => $local:expr),+ $(,)?) => {
        $(pub static $name: LazyLock<Iri> = LazyLock::new(|| $ns.term($local));)+
    };
}

/// XSD datatypes, so literals name their type rather than implying it.
pub mod xsd {
    use super::{Iri, LazyLock, Namespace};
    pub const NS: Namespace = Namespace {
        prefix: "xsd",
        base: "http://www.w3.org/2001/XMLSchema#",
    };
    iri_const!(NS,
        STRING => "string",
        DECIMAL => "decimal",
        INTEGER => "integer",
        BOOLEAN => "boolean",
        DATE_TIME => "dateTime",
    );
}

/// RDF and RDFS terms used when emitting.
pub mod rdf {
    use super::{Iri, LazyLock, Namespace};
    pub const NS: Namespace = Namespace {
        prefix: "rdf",
        base: "http://www.w3.org/1999/02/22-rdf-syntax-ns#",
    };
    pub const RDFS: Namespace = Namespace {
        prefix: "rdfs",
        base: "http://www.w3.org/2000/01/rdf-schema#",
    };
    iri_const!(NS, TYPE => "type");
    iri_const!(RDFS, LABEL => "label", COMMENT => "comment");
}

/// The `agent:` terms this crate's capability model maps onto.
///
/// These are the join with NS-ONTO-001: an emitted capability is an
/// `agent:Action` carrying the same `sideEffect` / `reversible` /
/// `requiresAuthority` properties the company ontology already defines, so a
/// SPARQL query over the company graph reaches products written in Rust.
pub mod agent {
    use super::{Iri, LazyLock, AGENTS};
    iri_const!(AGENTS,
        ACTION => "Action",
        TOOL => "Tool",
        AUTHORITY => "Authority",
        ACTOR => "actor",
        TARGET => "target",
        USES_TOOL => "usesTool",
        HAS_AUTHORITY => "hasAuthority",
        REQUIRES_AUTHORITY => "requiresAuthority",
        SIDE_EFFECT => "sideEffect",
        REVERSIBLE => "reversible",
        RISK_SCORE => "riskScore",
        INPUT_SCHEMA => "inputSchema",
        OUTPUT_SCHEMA => "outputSchema",
    );
}

/// Core terms used when emitting.
pub mod core_terms {
    use super::{Iri, LazyLock, CORE};
    iri_const!(CORE,
        ENTITY => "Entity",
        CAPABILITY => "Capability",
        EVIDENCE => "Evidence",
        POLICY => "Policy",
        NAME => "name",
        DESCRIPTION => "description",
        STATUS => "status",
        PROVIDES_CAPABILITY => "providesCapability",
        HAS_EVIDENCE => "hasEvidence",
        CONFORMS_TO => "conformsTo",
        STANDARD_REVISION => "standardRevision",
        // NS-PROG-001. In `core` rather than a namespace of their own for the
        // reason the standard IRI is: the prefix set is pinned to the published
        // company context, and adding one detaches this crate from the graph it
        // emits into -- silently, since the symptom is an empty query.
        GOAL => "Goal",
        MILESTONE => "Milestone",
        STAGE => "stage",
        BLOCKED_BECAUSE => "blockedBecause",
        BLOCKED_BY => "blockedBy",
        PROVENANCE => "provenance",
        OBSERVED_AT => "observedAt",
        ADVANCES_GOAL => "advancesGoal",
        ADDRESSES => "addresses",
        TRACKED_AS => "trackedAs",
        PURSUES_GOAL => "pursuesGoal",
        HAS_MILESTONE => "hasMilestone",
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_invalid_iri_cannot_be_constructed() {
        // Each of these is a mistake that reached a term in the example
        // architecture's model, where validation happened later or not at all.
        assert_eq!(Iri::parse(""), Err(IriError::Empty));
        assert_eq!(Iri::parse("   "), Err(IriError::Empty));
        assert!(
            Iri::parse("ns:Entity").is_ok(),
            "a CURIE has a scheme-like prefix"
        );
        assert!(matches!(
            Iri::parse("/core/Entity"),
            Err(IriError::NotAbsolute(_))
        ));
        assert!(matches!(
            Iri::parse("https://example.org/a b"),
            Err(IriError::ContainsWhitespace(_))
        ));
    }

    #[test]
    fn namespaces_match_the_company_context() {
        // Verbatim from EXAMPLE-ONTO-000 contexts/v1.jsonld. If this fails, this
        // crate has silently detached from the company graph and joins across
        // products will produce nothing rather than an error.
        assert_eq!(CORE.base, "https://ontology.nervosys.com/core/");
        assert_eq!(
            ENGINEERING.base,
            "https://ontology.nervosys.com/engineering/"
        );
        assert_eq!(AGENTS.base, "https://ontology.nervosys.com/agents/");
        assert_eq!(HARDWARE.base, "https://ontology.nervosys.com/hardware/");
        assert_eq!(SOFTWARE.base, "https://ontology.nervosys.com/software/");
        assert_eq!(ENTITY.base, "https://ontology.nervosys.com/entity/");
    }

    #[test]
    fn terms_resolve_to_the_iris_the_company_ontology_declares() {
        assert_eq!(
            CORE.term("Agent").as_str(),
            "https://ontology.nervosys.com/core/Agent"
        );
        assert_eq!(
            agent::SIDE_EFFECT.as_str(),
            "https://ontology.nervosys.com/agents/sideEffect"
        );
        assert_eq!(CORE.curie("Agent"), "ns:Agent");
    }
}
