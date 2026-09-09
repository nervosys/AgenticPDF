//! SHACL shapes for the graph [`crate::emit`] produces.
//!
//! # Why this exists
//!
//! Everything else in this crate is enforced by the Rust type system, which
//! only helps people inside our build. A consumer who receives
//! `/api/v1/ontology.jsonld` from a NERVOSYS service has no way to check that
//! it is well-formed without trusting us — and for a company whose product is
//! provenance, "trust us" is the wrong answer.
//!
//! These shapes are that check, in a standard third parties already run.
//!
//! # Generated, not authored
//!
//! The shapes are built from the same module that knows what the emitter
//! writes, because a hand-authored shape file is the textbook instance of the
//! failure this codebase keeps hitting: two things that must agree, kept in two
//! places. [`tests::every_emitted_predicate_is_constrained`] closes the loop by
//! walking a real emitted graph and asserting each predicate is either
//! constrained by a shape or named in [`STRUCTURAL`] — no third option.
//!
//! # What these shapes do not do
//!
//! They constrain the *shape* of a capability description: cardinalities,
//! datatypes, the 0..1 bound on risk. They cannot check that a declaration is
//! *true* of the running system — that a path exists, that an authority claim
//! matches the code. Only a test inside the product can do that, which is why
//! NS-ONTO-001 §7.6 requires one. Publishing shapes does not retire that
//! obligation, and a reader should not infer that it does.

use crate::iri::{agent, core_terms, rdf, xsd, Iri, Namespace};
use crate::term::{BlankNode, Literal, Quad};

/// The SHACL vocabulary.
pub const SH: Namespace = Namespace {
    prefix: "sh",
    base: "http://www.w3.org/ns/shacl#",
};

/// Predicates the emitter writes that carry no property shape, with the reason.
///
/// Required to be exhaustive by the coverage test. Adding a predicate to the
/// emitter without either constraining it or listing it here fails the build,
/// which is the whole mechanism.
pub const STRUCTURAL: &[(&str, &str)] = &[
    (
        "http://www.w3.org/1999/02/22-rdf-syntax-ns#type",
        "class membership is what the shapes target; constraining it with a \
         shape would be circular",
    ),
    (
        "https://ontology.nervosys.com/core/providesCapability",
        "asserted from the product node outward, so it is constrained by the \
         product shape rather than the capability shape",
    ),
];

pub(crate) fn sh(local: &str) -> Iri {
    SH.term(local)
}

/// One `sh:property` constraint, as a blank node hung off a node shape.
pub(crate) struct PropertyShape {
    path: Iri,
    min: Option<u32>,
    max: Option<u32>,
    datatype: Option<Iri>,
    class: Option<Iri>,
    min_inclusive: Option<f64>,
    max_inclusive: Option<f64>,
}

impl PropertyShape {
    pub(crate) fn on(path: Iri) -> Self {
        Self {
            path,
            min: None,
            max: None,
            datatype: None,
            class: None,
            min_inclusive: None,
            max_inclusive: None,
        }
    }
    pub(crate) fn exactly_one(mut self) -> Self {
        self.min = Some(1);
        self.max = Some(1);
        self
    }
    pub(crate) fn at_most_one(mut self) -> Self {
        self.max = Some(1);
        self
    }
    pub(crate) fn datatype(mut self, d: Iri) -> Self {
        self.datatype = Some(d);
        self
    }
    pub(crate) fn class(mut self, c: Iri) -> Self {
        self.class = Some(c);
        self
    }
    fn between(mut self, lo: f64, hi: f64) -> Self {
        self.min_inclusive = Some(lo);
        self.max_inclusive = Some(hi);
        self
    }

    pub(crate) fn emit(&self, owner: &Iri, label: &str, out: &mut Vec<Quad>) {
        let b = BlankNode::new(label);
        out.push(Quad::new(owner.clone(), sh("property"), b.clone()));
        out.push(Quad::new(b.clone(), sh("path"), self.path.clone()));
        if let Some(n) = self.min {
            out.push(Quad::new(
                b.clone(),
                sh("minCount"),
                Literal::typed(n.to_string(), xsd::INTEGER.clone()),
            ));
        }
        if let Some(n) = self.max {
            out.push(Quad::new(
                b.clone(),
                sh("maxCount"),
                Literal::typed(n.to_string(), xsd::INTEGER.clone()),
            ));
        }
        if let Some(d) = &self.datatype {
            out.push(Quad::new(b.clone(), sh("datatype"), d.clone()));
        }
        if let Some(c) = &self.class {
            out.push(Quad::new(b.clone(), sh("class"), c.clone()));
        }
        if let Some(v) = self.min_inclusive {
            out.push(Quad::new(
                b.clone(),
                sh("minInclusive"),
                Literal::typed(format!("{v}"), xsd::DECIMAL.clone()),
            ));
        }
        if let Some(v) = self.max_inclusive {
            out.push(Quad::new(
                b.clone(),
                sh("maxInclusive"),
                Literal::typed(format!("{v}"), xsd::DECIMAL.clone()),
            ));
        }
    }
}

/// Shapes are named in a NERVOSYS namespace, not the SHACL one — `sh:` is the
/// vocabulary they are written in, not where they live.
pub(crate) fn node_shape(name: &str, target: Iri, props: Vec<PropertyShape>, out: &mut Vec<Quad>) {
    let shape = Iri::parse(format!("https://ontology.nervosys.com/shapes/{name}"))
        .expect("static shape IRI");
    out.push(Quad::new(shape.clone(), rdf::TYPE.clone(), sh("NodeShape")));
    out.push(Quad::new(shape.clone(), sh("targetClass"), target));
    for (i, p) in props.iter().enumerate() {
        p.emit(&shape, &format!("{name}-p{i}"), out);
    }
}

/// The shape graph for a NS-ONTO-001 capability projection.
pub fn shapes() -> Vec<Quad> {
    let mut out = Vec::new();

    node_shape(
        "CapabilityShape",
        agent::ACTION.clone(),
        vec![
            PropertyShape::on(core_terms::NAME.clone()).exactly_one(),
            PropertyShape::on(core_terms::DESCRIPTION.clone()).exactly_one(),
            // Declared, never inferred from the verb — so it must be present.
            PropertyShape::on(agent::SIDE_EFFECT.clone())
                .exactly_one()
                .datatype(xsd::BOOLEAN.clone()),
            PropertyShape::on(agent::REVERSIBLE.clone())
                .exactly_one()
                .datatype(xsd::BOOLEAN.clone()),
            PropertyShape::on(agent::RISK_SCORE.clone())
                .exactly_one()
                .datatype(xsd::DECIMAL.clone())
                .between(0.0, 1.0),
            // At most one, and optional: its *absence* is how "requires no
            // authority" is expressed. A minCount here would make that
            // inexpressible and force a sentinel, which §4 forbids.
            PropertyShape::on(agent::REQUIRES_AUTHORITY.clone())
                .at_most_one()
                .class(agent::AUTHORITY.clone()),
            PropertyShape::on(core_terms::HAS_EVIDENCE.clone())
                .at_most_one()
                .class(core_terms::EVIDENCE.clone()),
        ],
        &mut out,
    );

    node_shape(
        "AuthorityShape",
        agent::AUTHORITY.clone(),
        vec![PropertyShape::on(core_terms::NAME.clone()).exactly_one()],
        &mut out,
    );

    node_shape(
        "EvidenceShape",
        core_terms::EVIDENCE.clone(),
        vec![
            PropertyShape::on(core_terms::DESCRIPTION.clone()).exactly_one(),
            // The verification command. Optional: not every claim has one, and
            // inventing an empty string would assert a command that does not
            // exist.
            PropertyShape::on(core_terms::NAME.clone()).at_most_one(),
        ],
        &mut out,
    );

    node_shape(
        "ProductShape",
        core_terms::ENTITY.clone(),
        vec![
            PropertyShape::on(core_terms::NAME.clone()).exactly_one(),
            PropertyShape::on(core_terms::PROVIDES_CAPABILITY.clone()).class(agent::ACTION.clone()),
            // Which standard, and which revision of it, produced this graph.
            //
            // Constrained as form, not as currency. Whether a revision is the
            // *latest* is not a property of a well-formed projection — it is a
            // comparison against something only the hub can see, so the hub
            // makes it. A third party running these shapes offline can still
            // check that the claim is present and singular.
            PropertyShape::on(core_terms::CONFORMS_TO.clone()).exactly_one(),
            PropertyShape::on(core_terms::STANDARD_REVISION.clone()).exactly_one(),
        ],
        &mut out,
    );

    out
}

/// Every predicate a shape constrains, via `sh:path`.
pub fn constrained_predicates() -> Vec<String> {
    let path = sh("path");
    let mut v: Vec<String> = shapes()
        .iter()
        .filter(|q| q.predicate == path)
        .filter_map(|q| q.object.as_iri().map(|i| i.as_str().to_string()))
        .collect();
    v.sort();
    v.dedup();
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability::{Authority, Capability, Evidence};
    use crate::test_support::manifest;

    fn sample() -> crate::Manifest {
        manifest(vec![
            Capability::read("a.get", "GET", "/a", "read"),
            Capability::read("a.demote", "POST", "/d", "demote")
                .mutating(Authority::Scope { name: "demote" }, false)
                .with_evidence(Evidence {
                    records: "log index",
                    verify_command: Some("sf verify"),
                }),
            Capability::read("s.create", "POST", "/s", "open a session")
                .mutating(Authority::Public, true),
        ])
    }

    /// The loop-closing test. Without it these shapes are a document that
    /// happens to sit near the emitter rather than a constraint on it.
    #[test]
    fn every_emitted_predicate_is_constrained() {
        let emitted = crate::emit::to_quads(&sample());
        let constrained = constrained_predicates();
        let exempt: Vec<&str> = STRUCTURAL.iter().map(|(p, _)| *p).collect();

        let mut uncovered: Vec<String> = emitted
            .iter()
            .map(|q| q.predicate.as_str().to_string())
            .filter(|p| !constrained.contains(p) && !exempt.contains(&p.as_str()))
            .collect();
        uncovered.sort();
        uncovered.dedup();

        assert!(
            uncovered.is_empty(),
            "the emitter writes predicates no shape constrains and STRUCTURAL does not \
             exempt. Add a property shape, or list it in STRUCTURAL with a reason:\n  {uncovered:#?}"
        );
    }

    /// The inverse: a shape constraining something never emitted is dead
    /// weight, and worse, tells a consumer to expect data that never arrives.
    #[test]
    fn no_shape_constrains_a_predicate_the_emitter_never_writes() {
        let emitted: Vec<String> = crate::emit::to_quads(&sample())
            .iter()
            .map(|q| q.predicate.as_str().to_string())
            .collect();
        for p in constrained_predicates() {
            assert!(
                emitted.contains(&p),
                "a shape constrains `{p}`, which the emitter never writes"
            );
        }
    }

    #[test]
    fn the_shape_graph_is_not_empty() {
        // The guard NS-ONTO-001 §2.1 requires: a generator that silently
        // produces nothing must not look like one that worked.
        let s = shapes();
        assert!(
            s.len() > 30,
            "only {} quads of shapes; the generator is broken",
            s.len()
        );
        assert!(!crate::fingerprint::fingerprint(&s).is_empty());
    }

    #[test]
    fn authority_is_optional_so_that_public_is_expressible() {
        // If requiresAuthority were minCount 1, "requires no authority" would
        // need a sentinel node — which §4 forbids and which would make the
        // "capabilities requiring nothing" query impossible to write.
        let path = sh("path");
        let owner: Vec<_> = shapes()
            .iter()
            .filter(|q| {
                q.predicate == path && q.object.as_iri() == Some(&agent::REQUIRES_AUTHORITY)
            })
            .map(|q| q.subject.clone())
            .collect();
        assert_eq!(owner.len(), 1);
        let has_min = shapes()
            .iter()
            .any(|q| q.subject == owner[0] && q.predicate == sh("minCount"));
        assert!(!has_min, "requiresAuthority must not be mandatory");
    }

    #[test]
    fn risk_score_is_bounded_zero_to_one() {
        let n = crate::emit::to_nquads(&shapes());
        assert!(n.contains("<http://www.w3.org/ns/shacl#minInclusive> \"0\""));
        assert!(n.contains("<http://www.w3.org/ns/shacl#maxInclusive> \"1\""));
    }

    #[test]
    fn every_structural_exemption_carries_a_reason() {
        for (p, why) in STRUCTURAL {
            assert!(!p.is_empty());
            assert!(
                why.len() > 20,
                "`{p}` is exempt from shape coverage with no real explanation"
            );
        }
    }
}
