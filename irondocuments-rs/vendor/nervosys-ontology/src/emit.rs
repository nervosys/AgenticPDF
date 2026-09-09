//! Projecting a manifest into the company graph.
//!
//! This is the join. A product declares itself in Rust, where the compiler and
//! its tests hold the declaration to the running system; this module turns that
//! declaration into RDF using the IRIs NS-ONTO-001 already defines, so the
//! result answers SPARQL queries across every product rather than only its own.
//!
//! The mapping is deliberately thin:
//!
//! | Rust | Company ontology |
//! |---|---|
//! | [`Capability`] | `agent:Action` |
//! | `effect == Mutating` | `agent:sideEffect true` |
//! | `reversible` | `agent:reversible` |
//! | [`Authority`] | `agent:requiresAuthority` → an `agent:Authority` node |
//! | `risk_score()` | `agent:riskScore` |
//! | [`Evidence`] | `ns:hasEvidence` → an `ns:Evidence` node |
//!
//! Because those are the company's terms, a question like *"which capabilities
//! across all products are irreversible, change state, and require no
//! authority?"* is one query instead of a survey. That query is the reason to
//! emit at all.
//!
//! # What is not emitted
//!
//! Nothing is invented to fill a gap. A capability with no evidence emits no
//! evidence node — it does not emit an empty one — because a graph that
//! represents absence as a present-but-null node teaches every consumer to
//! misread it.

use crate::capability::{Authority, Capability, Manifest};
use crate::iri::{agent, core_terms, rdf, xsd, Iri};
use crate::term::{Literal, Quad};

/// Mint a stable IRI for a capability beneath the manifest's base.
fn capability_iri(m: &Manifest, c: &Capability) -> Iri {
    Iri::parse(format!(
        "{}/capability/{}",
        m.base.as_str().trim_end_matches('/'),
        c.name
    ))
    .expect("manifest base is a valid IRI and capability names are path-safe")
}

fn authority_iri(m: &Manifest, c: &Capability) -> Iri {
    Iri::parse(format!(
        "{}/authority/{}",
        m.base.as_str().trim_end_matches('/'),
        c.name
    ))
    .expect("valid")
}

fn evidence_iri(m: &Manifest, c: &Capability) -> Iri {
    Iri::parse(format!(
        "{}/evidence/{}",
        m.base.as_str().trim_end_matches('/'),
        c.name
    ))
    .expect("valid")
}

fn boolean(b: bool) -> Literal {
    Literal::typed(if b { "true" } else { "false" }, xsd::BOOLEAN.clone())
}

fn decimal(v: f64) -> Literal {
    Literal::typed(format!("{v:.2}"), xsd::DECIMAL.clone())
}

/// Emit a manifest as RDF quads in the company vocabulary.
pub fn to_quads(m: &Manifest) -> Vec<Quad> {
    // The product itself, and the standard it was built against.
    let mut out = vec![
        Quad::new(
            m.base.clone(),
            rdf::TYPE.clone(),
            core_terms::ENTITY.clone(),
        ),
        Quad::new(
            m.base.clone(),
            core_terms::NAME.clone(),
            Literal::plain(m.product),
        ),
        Quad::new(
            m.base.clone(),
            core_terms::DESCRIPTION.clone(),
            Literal::plain(m.purpose),
        ),
        // A project cannot see NS-ONTO-001 and so cannot know whether its copy
        // is current. It states what it has; the hub, which has visibility,
        // decides whether that is the latest. Emitting the revision is what
        // makes that judgement possible without pointing a project upstream.
        Quad::new(
            m.base.clone(),
            core_terms::CONFORMS_TO.clone(),
            crate::spec::iri(),
        ),
        Quad::new(
            m.base.clone(),
            core_terms::STANDARD_REVISION.clone(),
            Literal::plain(crate::spec::REVISION),
        ),
    ];

    for c in &m.capabilities {
        let ci = capability_iri(m, c);

        out.push(Quad::new(
            ci.clone(),
            rdf::TYPE.clone(),
            agent::ACTION.clone(),
        ));
        out.push(Quad::new(
            ci.clone(),
            core_terms::NAME.clone(),
            Literal::plain(c.name),
        ));
        out.push(Quad::new(
            ci.clone(),
            core_terms::DESCRIPTION.clone(),
            Literal::plain(c.purpose),
        ));
        // The product provides the capability; the capability targets nothing
        // in particular until it is invoked, so no agent:target is emitted.
        out.push(Quad::new(
            m.base.clone(),
            core_terms::PROVIDES_CAPABILITY.clone(),
            ci.clone(),
        ));

        out.push(Quad::new(
            ci.clone(),
            agent::SIDE_EFFECT.clone(),
            boolean(c.effect.mutates()),
        ));
        out.push(Quad::new(
            ci.clone(),
            agent::REVERSIBLE.clone(),
            boolean(c.reversible),
        ));
        out.push(Quad::new(
            ci.clone(),
            agent::RISK_SCORE.clone(),
            decimal(c.risk_score()),
        ));

        // Authority is a node, not a string, because NS-ONTO-001 models it as
        // a class and because "scope:admin" as a literal cannot be joined
        // against anything.
        //
        // Public is the one case that emits no authority node: requiring
        // nothing is the absence of a requirement, and minting an
        // "authority: none" node would make `MISSING agent:requiresAuthority`
        // untrue for every capability and so useless as a query.
        if !matches!(c.authority, Authority::Public) {
            let ai = authority_iri(m, c);
            out.push(Quad::new(
                ai.clone(),
                rdf::TYPE.clone(),
                agent::AUTHORITY.clone(),
            ));
            out.push(Quad::new(
                ai.clone(),
                core_terms::NAME.clone(),
                Literal::plain(c.authority.label()),
            ));
            out.push(Quad::new(ci.clone(), agent::REQUIRES_AUTHORITY.clone(), ai));
        }

        if let Some(e) = &c.evidence {
            let ei = evidence_iri(m, c);
            out.push(Quad::new(
                ei.clone(),
                rdf::TYPE.clone(),
                core_terms::EVIDENCE.clone(),
            ));
            out.push(Quad::new(
                ei.clone(),
                core_terms::DESCRIPTION.clone(),
                Literal::plain(e.records),
            ));
            if let Some(cmd) = e.verify_command {
                out.push(Quad::new(
                    ei.clone(),
                    core_terms::NAME.clone(),
                    Literal::plain(cmd),
                ));
            }
            out.push(Quad::new(ci.clone(), core_terms::HAS_EVIDENCE.clone(), ei));
        }
    }

    out
}

/// Serialise quads as N-Quads.
pub fn to_nquads(quads: &[Quad]) -> String {
    use crate::term::{Subject, Term};
    let mut lines: Vec<String> = quads
        .iter()
        .map(|q| {
            let s = match &q.subject {
                Subject::Iri(i) => format!("<{i}>"),
                Subject::Blank(b) => format!("_:{}", b.0),
            };
            let o = match &q.object {
                Term::Iri(i) => format!("<{i}>"),
                Term::Blank(b) => format!("_:{}", b.0),
                Term::Literal(l) => {
                    let mut t = format!("{:?}", l.lexical);
                    if let Some(lang) = &l.language {
                        t.push('@');
                        t.push_str(lang);
                    } else if let Some(dt) = &l.datatype {
                        t.push_str("^^<");
                        t.push_str(dt.as_str());
                        t.push('>');
                    }
                    t
                }
            };
            let g = q
                .graph
                .as_ref()
                .map(|g| format!(" <{g}>"))
                .unwrap_or_default();
            format!("{s} <{}> {o}{g} .", q.predicate)
        })
        .collect();
    lines.sort();
    lines.join("\n")
}

/// The JSON-LD context this crate's output uses.
///
/// Generated from [`crate::iri::ALL`] so it cannot drift from the IRIs the
/// emitter actually writes.
pub fn jsonld_context() -> serde_json::Value {
    let mut ctx = serde_json::Map::new();
    for ns in crate::iri::ALL {
        ctx.insert(ns.prefix.into(), serde_json::Value::String(ns.base.into()));
    }
    serde_json::json!({ "@context": ctx })
}

/// Shorten an IRI to a CURIE when a known namespace covers it.
///
/// Purely cosmetic — the full IRI is always correct, and a consumer that
/// resolves the context gets the same graph either way.
fn curie(i: &Iri) -> String {
    for ns in crate::iri::ALL {
        if let Some(local) = i.as_str().strip_prefix(ns.base) {
            if !local.contains('/') {
                return format!("{}:{local}", ns.prefix);
            }
        }
    }
    for ns in [
        crate::iri::xsd::NS,
        crate::iri::rdf::NS,
        crate::iri::rdf::RDFS,
    ] {
        if let Some(local) = i.as_str().strip_prefix(ns.base) {
            return format!("{}:{local}", ns.prefix);
        }
    }
    i.as_str().to_string()
}

/// A manifest as a JSON-LD document.
///
/// Quads are grouped by subject into node objects, which is the form a JSON-LD
/// processor and most graph stores expect. Literals keep their datatype via
/// `@value`/`@type` rather than being flattened to strings — the lossy step the
/// example architecture took during compilation, after which no datatype
/// constraint downstream could be evaluated.
pub fn to_jsonld(m: &Manifest) -> serde_json::Value {
    use crate::term::{Subject, Term};
    use serde_json::{json, Map, Value};
    use std::collections::BTreeMap;

    let quads = to_quads(m);
    // BTreeMap so output ordering is stable across runs; an endpoint that
    // reshuffles its own bytes on every request cannot be cached or diffed.
    let mut nodes: BTreeMap<String, Map<String, Value>> = BTreeMap::new();

    for q in &quads {
        let sid = match &q.subject {
            Subject::Iri(i) => curie(i),
            Subject::Blank(b) => format!("_:{}", b.0),
        };
        let node = nodes.entry(sid.clone()).or_insert_with(|| {
            let mut m = Map::new();
            m.insert("@id".into(), Value::String(sid.clone()));
            m
        });

        let key = if q.predicate == *rdf::TYPE {
            "@type".to_string()
        } else {
            curie(&q.predicate)
        };

        let value = match &q.object {
            Term::Iri(i) => Value::String(curie(i)),
            Term::Blank(b) => Value::String(format!("_:{}", b.0)),
            Term::Literal(l) => match (&l.datatype, &l.language) {
                (_, Some(lang)) => json!({ "@value": l.lexical, "@language": lang }),
                (Some(dt), None) => json!({ "@value": l.lexical, "@type": curie(dt) }),
                (None, None) => Value::String(l.lexical.clone()),
            },
        };

        // A repeated predicate becomes an array rather than overwriting, which
        // would silently drop statements.
        match node.get_mut(&key) {
            None => {
                node.insert(key, value);
            }
            Some(Value::Array(a)) => a.push(value),
            Some(existing) => {
                let prev = existing.clone();
                *existing = Value::Array(vec![prev, value]);
            }
        }
    }

    let mut ctx = Map::new();
    for ns in crate::iri::ALL {
        ctx.insert(ns.prefix.into(), Value::String(ns.base.into()));
    }
    for ns in [
        crate::iri::xsd::NS,
        crate::iri::rdf::NS,
        crate::iri::rdf::RDFS,
    ] {
        ctx.insert(ns.prefix.into(), Value::String(ns.base.into()));
    }

    json!({
        "@context": ctx,
        "@graph": nodes.into_values().map(Value::Object).collect::<Vec<_>>(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability::{Effect, Evidence};
    use crate::test_support::manifest;

    fn sample() -> Manifest {
        manifest(vec![
            Capability::read("artifact.get", "GET", "/a/:id", "read an artifact"),
            Capability::read("artifact.demote", "POST", "/demote/:id", "lower a tier")
                .mutating(Authority::Scope { name: "demote" }, false)
                .with_evidence(Evidence {
                    records: "transparency-log index",
                    verify_command: Some("sf verify --bundle"),
                }),
            Capability::read("session.create", "POST", "/sessions", "open a session")
                .mutating(Authority::Public, true),
        ])
    }

    #[test]
    fn a_mutation_emits_the_company_side_effect_predicate() {
        let q = to_quads(&sample());
        let n = to_nquads(&q);
        assert!(
            n.contains("<https://ontology.nervosys.com/agents/sideEffect> \"true\""),
            "emitted graph does not use the company predicate:\n{n}"
        );
        assert!(n.contains("<https://ontology.nervosys.com/agents/Action>"));
    }

    #[test]
    fn public_capabilities_emit_no_authority_node() {
        // So that "capabilities requiring no authority" is expressible as a
        // missing predicate rather than a sentinel value.
        let q = to_quads(&sample());
        let auth: Vec<_> = q
            .iter()
            .filter(|x| x.predicate == *agent::REQUIRES_AUTHORITY)
            .collect();
        assert_eq!(
            auth.len(),
            1,
            "only the scoped capability should require authority"
        );
    }

    #[test]
    fn a_capability_without_evidence_emits_no_evidence_node() {
        let q = to_quads(&sample());
        let ev: Vec<_> = q
            .iter()
            .filter(|x| x.predicate == *core_terms::HAS_EVIDENCE)
            .collect();
        assert_eq!(ev.len(), 1, "absence must be absent, not a null node");
    }

    #[test]
    fn the_dangerous_capability_is_findable_by_query_shape() {
        // The join this whole module exists for: state-changing, reachable
        // with no authority. `session.create` is exactly that shape.
        let q = to_quads(&sample());
        let m = sample();
        let open: Vec<_> = m.open_mutations().iter().map(|c| c.name).collect();
        assert_eq!(open, vec!["session.create"]);
        // And it is visible in the emitted graph as sideEffect true with no
        // requiresAuthority statement.
        let ci = capability_iri(&m, m.capability("session.create").unwrap());
        let has_auth = q.iter().any(|x| {
            x.predicate == *agent::REQUIRES_AUTHORITY
                && matches!(&x.subject, crate::term::Subject::Iri(i) if *i == ci)
        });
        assert!(!has_auth);
    }

    #[test]
    fn the_context_covers_every_namespace_the_emitter_uses() {
        let ctx = jsonld_context();
        let map = ctx["@context"].as_object().unwrap();
        for ns in crate::iri::ALL {
            assert_eq!(map[ns.prefix].as_str(), Some(ns.base));
        }
        // And matches the company context as published in EXAMPLE-ONTO-000.
        assert_eq!(
            map["ns"].as_str(),
            Some("https://ontology.nervosys.com/core/")
        );
        assert_eq!(
            map["agent"].as_str(),
            Some("https://ontology.nervosys.com/agents/")
        );
    }

    #[test]
    fn emitting_then_fingerprinting_depends_on_the_manifest() {
        let a = crate::fingerprint::fingerprint(&to_quads(&sample()));
        let mut m = sample();
        m.capabilities[0].purpose = "something else";
        let b = crate::fingerprint::fingerprint(&to_quads(&m));
        assert_ne!(a, b);
        assert!(!a.is_empty());
    }

    #[test]
    fn jsonld_groups_by_subject_and_keeps_datatypes() {
        let doc = to_jsonld(&sample());
        let graph = doc["@graph"].as_array().expect("@graph is an array");
        assert!(
            !graph.is_empty(),
            "an empty projection is the defect being avoided"
        );

        let cap = graph
            .iter()
            .find(|n| {
                n["@id"]
                    .as_str()
                    .is_some_and(|s| s.ends_with("/capability/artifact.demote"))
            })
            .expect("the demote capability is a node");

        assert_eq!(cap["@type"], "agent:Action");
        // Booleans keep their datatype rather than becoming the string "true".
        assert_eq!(cap["agent:sideEffect"]["@value"], "true");
        assert_eq!(cap["agent:sideEffect"]["@type"], "xsd:boolean");
        assert_eq!(cap["agent:reversible"]["@value"], "false");
    }

    #[test]
    fn a_repeated_predicate_becomes_an_array_rather_than_overwriting() {
        // The product provides three capabilities; all three must survive.
        let doc = to_jsonld(&sample());
        let graph = doc["@graph"].as_array().unwrap();
        let product = graph
            .iter()
            .find(|n| n["@id"] == "entity:test-product")
            .expect("the product node, as a CURIE");
        let provides = product["ns:providesCapability"]
            .as_array()
            .expect("three capabilities must not collapse into one value");
        assert_eq!(provides.len(), 3);
    }

    #[test]
    fn the_context_resolves_every_prefix_the_document_uses() {
        let doc = to_jsonld(&sample());
        let ctx = doc["@context"].as_object().unwrap();
        let text = doc["@graph"].to_string();
        for prefix in ["ns", "agent", "entity", "xsd"] {
            if text.contains(&format!("\"{prefix}:")) {
                assert!(
                    ctx.contains_key(prefix),
                    "`{prefix}:` is used but not in @context"
                );
            }
        }
    }

    #[test]
    fn jsonld_output_is_byte_stable_across_runs() {
        // Subjects are held in a BTreeMap for this reason: a document that
        // reshuffles itself cannot be cached, diffed, or fingerprinted.
        assert_eq!(
            to_jsonld(&sample()).to_string(),
            to_jsonld(&sample()).to_string()
        );
    }

    #[test]
    fn effect_is_declared_not_inferred_from_the_verb() {
        // A POST that only previews changes nothing; the model must be able to
        // say so, and the emitter must follow the declaration.
        let m = manifest(vec![Capability::read(
            "route.preview",
            "POST",
            "/preview",
            "evaluate without acting",
        )]);
        assert_eq!(m.capabilities[0].effect, Effect::ReadOnly);
        let n = to_nquads(&to_quads(&m));
        assert!(n.contains("<https://ontology.nervosys.com/agents/sideEffect> \"false\""));
    }
}
