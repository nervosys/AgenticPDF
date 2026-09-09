//! A machine-readable model of what a system knows, does, and refuses.
//!
//! This crate is the vocabulary. It contains **no concepts, no capabilities and
//! no units of its own**, and it never will: everything here is a type for a
//! product to fill in.
//!
//! # Two ontologies, one company
//!
//! NERVOSYS has a formal company ontology, **NERVOSYS-ONTOLOGY-001**
//! (NS-ONTO-001) — OWL classes in Turtle, SHACL shapes, a JSON-LD context,
//! SPARQL over the result. That model answers *what things are*, across domains
//! that will never share a binary. It is specified in `docs/NS-ONTO-001.md`.
//!
//! **EXAMPLE-ONTO-000**, under `example-architecture/`, is the worked example
//! that informed it. The two names are kept apart deliberately: where this
//! crate cites `EXAMPLE-ONTO-000 §6` the rule came from there, and where it
//! cites NS-ONTO-001 it means the vocabulary this crate emits into. The
//! distinction earns its keep — the example's *modelling* is sound, and its
//! *compiler* validates a graph it never loads. Three of the rules below exist
//! because of the second.
//!
//! This crate answers a different question: *what does one running system do,
//! what must a caller hold to ask, and how does it decline.* It is compiled
//! into the product it describes, so its tests can hold the description to the
//! running code.
//!
//! Neither replaces the other, and the seam between them is deliberate:
//! [`emit`] projects a [`Manifest`] into RDF **using NS-ONTO-001's own IRIs**.
//! A capability becomes an `agent:Action` carrying `agent:sideEffect`,
//! `agent:reversible` and `agent:requiresAuthority` — the company's terms, not
//! this crate's — so a query across the whole company graph reaches products
//! written in Rust.
//!
//! # Three lessons this design is built on
//!
//! Each cost a real defect, so each is enforced by a test rather than a
//! convention.
//!
//! **1. A check that cannot fail is worse than no check.** The EXAMPLE-ONTO-000
//! compiler skeleton validated a graph it never loaded: corrupting a source
//! file and deleting an entire module both produced the same cheerful
//! `validation pipeline completed` and the same digest. Every validating
//! function in this crate therefore has a paired test proving it *can* fail —
//! see [`fingerprint::tests::the_fingerprint_can_fail_to_match`] — and
//! [`check::Conformance`] reports how many invariants it actually evaluated so
//! a run that checked nothing cannot read like a run that checked everything.
//!
//! **2. Never model "unknown" as a value that looks like an answer.** An
//! earlier version of this crate wrote authority as `Option<&str>`, so `None`
//! rendered as "public" — and an endpoint that was closed by deployment
//! printed as public directly above the refusal saying it was closed.
//! [`Authority`] splits those apart and adds [`Authority::Unaudited`], whose
//! `is_open()` returns `None` rather than a boolean.
//!
//! **3. Do not discard what you were given.** The example compiler parsed
//! literals with datatype and language and compiled them to bare strings,
//! after which no datatype constraint downstream could be checked. [`term`]
//! keeps both for the life of a term, and offers no lossy conversion.
//!
//! # Three components, three visibility rules
//!
//! The company ontology is not one thing, and treating it as one produces a
//! rule that is either too weak or too strong. It has three parts:
//!
//! | Component | Here | May a project see it? |
//! |---|---|---|
//! | **spec / schema** | [`spec`], [`shapes`], `docs/NS-ONTO-001.md` | **yes** |
//! | **interface / code** | the rest of this crate | **yes** |
//! | **datastore / data** | the aggregated company graph, the project registry | **never** |
//!
//! The boundary is *data*, not direction. A project reading the specification
//! learns what is required of it and nothing about any sibling. A project
//! reading the datastore learns what every sibling declares — the company's
//! aggregate view, which belongs to the hub alone.
//!
//! This crate is entirely spec and interface, and therefore entirely
//! publishable: it declares no concepts, capabilities or units, so there is
//! nothing in it to leak. [`spec::descriptor`] is the publishable spec in
//! serialised form, and `spec::tests::the_descriptor_carries_no_company_data`
//! holds it to that rather than trusting review.
//!
//! # Conforming to the latest, without the datastore
//!
//! - **What the standard requires** ships inside the vendored copy.
//!   [`spec::CLAUSES`] names each obligation with its section and
//!   [`spec::certify`] evaluates every one against the product's own
//!   declaration, offline. Re-vendoring delivers new obligations as *failing
//!   tests*, which is the earliest a project can learn of them.
//! - **Whether the copy is current** is checked from both ends, neither of
//!   which touches the aggregate. A project may fetch the published
//!   [`spec::Descriptor`] and call [`spec::currency`]; independently, the hub
//!   compares the `ns:standardRevision` every projection carries.
//!
//! # Declared, then verified — never reflected
//!
//! Nothing here inspects a running system. Reflection produces a description
//! that is always accurate and never meaningful: it can report that a route
//! exists, never what it asserts or how it refuses. Declare by hand; let tests
//! prove the declaration matches reality.
//!
//! # Keeping the domain out
//!
//! - **Units.** A product defines its own unit enum and implements
//!   [`UnitKind`]; this crate stores the flattened [`UnitRef`] that checking
//!   needs. Adding a domain unit here is the first step back
//!   toward a shared crate that belongs to one product.
//! - **The model is passed, never global.** [`check::check_value`] takes the
//!   concepts as an argument. A crate-level `ontology()` would bind this
//!   vocabulary to whichever product declared its model first.

pub mod capability;
pub mod check;
pub mod concept;
pub mod conform;
pub mod emit;
pub mod fingerprint;
pub mod invariant;
pub mod iri;
pub mod parse;
pub mod progress;
pub mod shapes;
pub mod spec;
pub mod surface;
pub mod term;
pub mod unit;

pub use capability::{
    Absence, Authority, Capability, Effect, Evidence, Manifest, Refusal, RefusalStatus, Unmapped,
};
pub use check::{check_value, Conformance, Violation};
pub use concept::{Concept, Property};
pub use conform::{check as check_conformance, Report};
pub use emit::{jsonld_context, to_jsonld, to_nquads, to_quads};
pub use fingerprint::{fingerprint, Fingerprint};
pub use invariant::Invariant;
pub use iri::{Iri, Namespace};
pub use parse::{parse_nquads, ParseError};
pub use progress::{Goal, Milestone, MilestoneView, Programme, Provenance, Stage};
pub use shapes::{shapes, STRUCTURAL};
pub use spec::{
    certify, currency, descriptor, Certification, Clause, Conformant, Currency, Descriptor, Route,
};
pub use surface::{Affordance, Condition, Control, Surface, SurfaceKind, View};
pub use term::{BlankNode, Literal, Quad, Subject, Term};
pub use unit::{UnitKind, UnitRef};

/// Fixtures shared between this crate's test modules.
#[cfg(test)]
pub(crate) mod test_support {
    use super::*;

    /// A minimal manifest wrapping the given capabilities.
    pub fn manifest(capabilities: Vec<Capability>) -> Manifest {
        Manifest {
            schema: 1,
            product: "Test Product",
            base: Iri::parse("https://ontology.nervosys.com/entity/test-product").unwrap(),
            purpose: "a fixture",
            principles: Vec::new(),
            capabilities,
            concepts: Vec::new(),
            surfaces: Vec::new(),
            refusal_statuses: Vec::new(),
            unmapped: Vec::new(),
        }
    }
}
