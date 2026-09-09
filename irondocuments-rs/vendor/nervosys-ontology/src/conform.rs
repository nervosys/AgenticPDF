//! Checking an emitted graph against the SHACL shapes.
//!
//! This is the conformance half of hub-and-spoke. A project owns its ontology
//! and depends on nothing upstream; what makes that safe rather than merely
//! decoupled is that the hub can check what the project *emits* against the
//! published shapes. Without this, "conforms to the standard" is an assertion
//! nobody tests.
//!
//! # A subset, and it says which
//!
//! This evaluates the constraints [`crate::shapes`] actually uses:
//! `sh:targetClass`, `sh:path`, `sh:minCount`, `sh:maxCount`, `sh:datatype`,
//! `sh:class`, `sh:minInclusive`, `sh:maxInclusive`. It is **not** a SHACL
//! engine — no property paths, no logical constraint components, no SPARQL
//! constraints.
//!
//! Anything it meets and cannot evaluate is counted in
//! [`Report::unsupported`] and named. That is the whole difference between this
//! and the validator it replaces: EXAMPLE-ONTO-000's `validate_shacl` took the
//! shape files, ignored them, and returned `Ok(())`, so a caller could not tell
//! a clean graph from an unread one. A report here always says how many
//! constraints it evaluated, and a run that evaluated none is not a pass.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::iri::{rdf, Iri};
use crate::shapes::SH;
use crate::term::{Quad, Subject, Term};

/// One constraint that did not hold.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Violation {
    /// The node that failed.
    pub focus: String,
    /// The property path.
    pub path: String,
    /// What the shape required.
    pub expected: String,
    /// What was found.
    pub found: String,
}

impl std::fmt::Display for Violation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} [{}]: expected {}, found {}",
            self.focus, self.path, self.expected, self.found
        )
    }
}

/// The outcome of a conformance run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Report {
    pub violations: Vec<Violation>,
    /// Constraints actually evaluated.
    pub evaluated: usize,
    /// Focus nodes the shapes targeted and found.
    pub focus_nodes: usize,
    /// Constraint components met and not understood, named.
    pub unsupported: Vec<String>,
}

impl Report {
    /// Whether the graph conformed **and** the run was meaningful.
    ///
    /// A run that evaluated nothing is not a pass. This is the distinction the
    /// validator this replaces could not make.
    pub fn conforms(&self) -> bool {
        self.violations.is_empty() && self.evaluated > 0
    }

    /// True when nothing was checked at all — no shapes matched any data.
    pub fn vacuous(&self) -> bool {
        self.evaluated == 0
    }

    pub fn summary(&self) -> String {
        if self.vacuous() {
            return "VACUOUS — no shape matched any node; nothing was checked".into();
        }
        if self.violations.is_empty() {
            format!(
                "conforms ({} constraints over {} nodes, {} unsupported)",
                self.evaluated,
                self.focus_nodes,
                self.unsupported.len()
            )
        } else {
            format!(
                "{} violation(s) ({} constraints over {} nodes)\n  {}",
                self.violations.len(),
                self.evaluated,
                self.focus_nodes,
                self.violations
                    .iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join("\n  ")
            )
        }
    }
}

/// A property constraint lifted out of the shape graph.
#[derive(Debug, Default)]
struct PropShape {
    path: Option<Iri>,
    min: Option<u32>,
    max: Option<u32>,
    datatype: Option<Iri>,
    class: Option<Iri>,
    min_inclusive: Option<f64>,
    max_inclusive: Option<f64>,
    other: Vec<String>,
}

fn object_of<'a>(quads: &'a [Quad], subject: &Subject, predicate: &Iri) -> Option<&'a Term> {
    quads
        .iter()
        .find(|q| q.subject == *subject && q.predicate == *predicate)
        .map(|q| &q.object)
}

fn literal_f64(t: &Term) -> Option<f64> {
    t.as_literal().and_then(|l| l.lexical.parse().ok())
}

/// Validate `data` against `shapes`.
pub fn check(shapes: &[Quad], data: &[Quad]) -> Report {
    let mut report = Report {
        violations: Vec::new(),
        evaluated: 0,
        focus_nodes: 0,
        unsupported: Vec::new(),
    };

    let sh_target = SH.term("targetClass");
    let sh_property = SH.term("property");
    let sh_path = SH.term("path");

    // Which node shapes exist, and what class each targets.
    let node_shapes: Vec<(Subject, Iri)> = shapes
        .iter()
        .filter(|q| q.predicate == sh_target)
        .filter_map(|q| q.object.as_iri().map(|c| (q.subject.clone(), c.clone())))
        .collect();

    for (shape, target) in &node_shapes {
        // Focus nodes: everything with rdf:type == the targeted class.
        let focus: Vec<Subject> = data
            .iter()
            .filter(|q| q.predicate == *rdf::TYPE && q.object.as_iri() == Some(target))
            .map(|q| q.subject.clone())
            .collect();
        report.focus_nodes += focus.len();

        // The property shapes hanging off this node shape.
        let props: Vec<PropShape> = shapes
            .iter()
            .filter(|q| q.subject == *shape && q.predicate == sh_property)
            .filter_map(|q| match &q.object {
                Term::Blank(b) => Some(Subject::Blank(b.clone())),
                Term::Iri(i) => Some(Subject::Iri(i.clone())),
                Term::Literal(_) => None,
            })
            .map(|ps| {
                let mut p = PropShape {
                    path: object_of(shapes, &ps, &sh_path).and_then(|t| t.as_iri().cloned()),
                    ..Default::default()
                };
                for q in shapes.iter().filter(|q| q.subject == ps) {
                    let local = q.predicate.as_str().rsplit('#').next().unwrap_or("");
                    match local {
                        "path" => {}
                        "minCount" => p.min = literal_f64(&q.object).map(|v| v as u32),
                        "maxCount" => p.max = literal_f64(&q.object).map(|v| v as u32),
                        "datatype" => p.datatype = q.object.as_iri().cloned(),
                        "class" => p.class = q.object.as_iri().cloned(),
                        "minInclusive" => p.min_inclusive = literal_f64(&q.object),
                        "maxInclusive" => p.max_inclusive = literal_f64(&q.object),
                        // Named rather than ignored. An unevaluated constraint
                        // reported as satisfied is the defect being avoided.
                        other => p.other.push(format!("sh:{other}")),
                    }
                }
                p
            })
            .collect();

        for node in &focus {
            for p in &props {
                let Some(path) = &p.path else {
                    report
                        .unsupported
                        .push("property shape with no sh:path".into());
                    continue;
                };
                for u in &p.other {
                    report
                        .unsupported
                        .push(format!("{u} on {} (not evaluated)", path.as_str()));
                }

                let values: Vec<&Term> = data
                    .iter()
                    .filter(|q| q.subject == *node && q.predicate == *path)
                    .map(|q| &q.object)
                    .collect();

                let focus_label = subject_label(node);

                if let Some(min) = p.min {
                    report.evaluated += 1;
                    if (values.len() as u32) < min {
                        report.violations.push(Violation {
                            focus: focus_label.clone(),
                            path: path.as_str().into(),
                            expected: format!("at least {min} value(s)"),
                            found: format!("{}", values.len()),
                        });
                    }
                }
                if let Some(max) = p.max {
                    report.evaluated += 1;
                    if (values.len() as u32) > max {
                        report.violations.push(Violation {
                            focus: focus_label.clone(),
                            path: path.as_str().into(),
                            expected: format!("at most {max} value(s)"),
                            found: format!("{}", values.len()),
                        });
                    }
                }

                for v in &values {
                    if let Some(dt) = &p.datatype {
                        report.evaluated += 1;
                        let actual = v.as_literal().and_then(|l| l.datatype.clone());
                        if actual.as_ref() != Some(dt) {
                            report.violations.push(Violation {
                                focus: focus_label.clone(),
                                path: path.as_str().into(),
                                expected: format!("datatype {}", dt.as_str()),
                                found: actual
                                    .map(|d| d.as_str().to_string())
                                    .unwrap_or_else(|| "no datatype".into()),
                            });
                        }
                    }
                    if let Some(class) = &p.class {
                        report.evaluated += 1;
                        let ok = v.as_iri().is_some_and(|i| {
                            data.iter().any(|q| {
                                q.subject == Subject::Iri(i.clone())
                                    && q.predicate == *rdf::TYPE
                                    && q.object.as_iri() == Some(class)
                            })
                        });
                        if !ok {
                            report.violations.push(Violation {
                                focus: focus_label.clone(),
                                path: path.as_str().into(),
                                expected: format!("a node typed {}", class.as_str()),
                                found: term_label(v),
                            });
                        }
                    }
                    if let (Some(lo), Some(n)) = (p.min_inclusive, literal_f64(v)) {
                        report.evaluated += 1;
                        if n < lo {
                            report.violations.push(Violation {
                                focus: focus_label.clone(),
                                path: path.as_str().into(),
                                expected: format!("at least {lo}"),
                                found: n.to_string(),
                            });
                        }
                    }
                    if let (Some(hi), Some(n)) = (p.max_inclusive, literal_f64(v)) {
                        report.evaluated += 1;
                        if n > hi {
                            report.violations.push(Violation {
                                focus: focus_label.clone(),
                                path: path.as_str().into(),
                                expected: format!("at most {hi}"),
                                found: n.to_string(),
                            });
                        }
                    }
                }
            }
        }
    }

    report.unsupported.sort();
    report.unsupported.dedup();
    report
}

fn subject_label(s: &Subject) -> String {
    match s {
        Subject::Iri(i) => i.as_str().to_string(),
        Subject::Blank(b) => format!("_:{}", b.0),
    }
}

fn term_label(t: &Term) -> String {
    match t {
        Term::Iri(i) => i.as_str().to_string(),
        Term::Blank(b) => format!("_:{}", b.0),
        Term::Literal(l) => format!("{:?}", l.lexical),
    }
}

/// Group a report's violations by focus node, for readable output.
pub fn by_focus(report: &Report) -> BTreeMap<&str, Vec<&Violation>> {
    let mut m: BTreeMap<&str, Vec<&Violation>> = BTreeMap::new();
    for v in &report.violations {
        m.entry(v.focus.as_str()).or_default().push(v);
    }
    m
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability::{Authority, Capability, Evidence};
    use crate::iri::{agent, core_terms, xsd};
    use crate::term::Literal;
    use crate::test_support::manifest;

    fn sample_graph() -> Vec<Quad> {
        crate::emit::to_quads(&manifest(vec![
            Capability::read("a.get", "GET", "/a", "read an artifact"),
            Capability::read("a.demote", "POST", "/d", "lower a tier")
                .mutating(Authority::Scope { name: "demote" }, false)
                .with_evidence(Evidence {
                    records: "log index",
                    verify_command: Some("sf verify"),
                }),
        ]))
    }

    /// What a spoke emits must satisfy the published shapes. If this fails, the
    /// standard and the reference implementation disagree.
    #[test]
    fn a_real_emitted_manifest_conforms() {
        let r = check(&crate::shapes::shapes(), &sample_graph());
        assert!(r.conforms(), "{}", r.summary());
        assert!(
            r.evaluated > 20,
            "only {} constraints evaluated",
            r.evaluated
        );
        assert!(r.focus_nodes > 0);
    }

    /// The test the validator this replaces did not have.
    #[test]
    fn the_checker_can_fail() {
        let mut bad = sample_graph();
        // Drop a required predicate: every agent:Action needs sideEffect.
        bad.retain(|q| q.predicate != *agent::SIDE_EFFECT);
        let r = check(&crate::shapes::shapes(), &bad);
        assert!(!r.conforms(), "removing a required property must fail");
        assert!(r.violations.iter().any(|v| v.path.ends_with("sideEffect")));
    }

    #[test]
    fn a_wrong_datatype_is_caught() {
        let mut bad = sample_graph();
        for q in bad.iter_mut() {
            if q.predicate == *agent::REVERSIBLE {
                // A plain string where xsd:boolean is required — precisely the
                // distinction the example architecture destroyed at compile.
                q.object = Term::Literal(Literal::plain("true"));
            }
        }
        let r = check(&crate::shapes::shapes(), &bad);
        assert!(!r.conforms());
        assert!(r.violations.iter().any(|v| v.expected.contains("boolean")));
    }

    #[test]
    fn an_out_of_range_risk_score_is_caught() {
        let mut bad = sample_graph();
        for q in bad.iter_mut() {
            if q.predicate == *agent::RISK_SCORE {
                q.object = Term::Literal(Literal::typed("4.20", xsd::DECIMAL.clone()));
            }
        }
        let r = check(&crate::shapes::shapes(), &bad);
        assert!(!r.conforms());
        assert!(r
            .violations
            .iter()
            .any(|v| v.expected.contains("at most 1")));
    }

    #[test]
    fn an_authority_reference_pointing_at_nothing_is_caught() {
        let mut bad = sample_graph();
        // Keep the requiresAuthority link, delete the node it points at.
        bad.retain(|q| q.object.as_iri().map(|i| i.as_str()) != Some(agent::AUTHORITY.as_str()));
        let r = check(&crate::shapes::shapes(), &bad);
        assert!(!r.conforms(), "a dangling sh:class reference must fail");
    }

    /// An empty graph must not read as success.
    #[test]
    fn nothing_checked_is_not_a_pass() {
        let r = check(&crate::shapes::shapes(), &[]);
        assert!(r.vacuous());
        assert!(
            !r.conforms(),
            "a graph with no nodes must not report conformance"
        );
        assert!(r.summary().starts_with("VACUOUS"));
    }

    /// And so must an empty *shape* set — the EXAMPLE-ONTO-000 failure exactly.
    #[test]
    fn no_shapes_is_not_a_pass_either() {
        let r = check(&[], &sample_graph());
        assert!(r.vacuous());
        assert!(
            !r.conforms(),
            "validating against no shapes must not report conformance"
        );
    }

    #[test]
    fn public_capabilities_do_not_trip_the_optional_authority_constraint() {
        // a.get is public and emits no authority node; maxCount 1 with no
        // minCount must accept that.
        let r = check(&crate::shapes::shapes(), &sample_graph());
        assert!(!r
            .violations
            .iter()
            .any(|v| v.path.ends_with("requiresAuthority")));
    }

    #[test]
    fn unsupported_constraints_are_named_not_assumed_satisfied() {
        let mut shapes = crate::shapes::shapes();
        // Add a constraint component this checker does not implement.
        let target = shapes
            .iter()
            .find(|q| q.predicate == SH.term("path"))
            .map(|q| q.subject.clone())
            .unwrap();
        shapes.push(Quad::new(
            match target {
                Subject::Blank(b) => b,
                _ => unreachable!("property shapes are blank nodes"),
            },
            SH.term("pattern"),
            Literal::plain("^x"),
        ));
        let r = check(&shapes, &sample_graph());
        assert!(
            r.unsupported.iter().any(|u| u.contains("sh:pattern")),
            "an unimplemented constraint must be reported, not silently passed: {:?}",
            r.unsupported
        );
    }

    #[test]
    fn a_report_round_trips_through_serde() {
        let r = check(&crate::shapes::shapes(), &sample_graph());
        let json = serde_json::to_string(&r).unwrap();
        let back: Report = serde_json::from_str(&json).unwrap();
        assert_eq!(r, back);
    }

    #[test]
    fn violations_group_by_focus_node() {
        let mut bad = sample_graph();
        bad.retain(|q| q.predicate != *core_terms::NAME);
        let r = check(&crate::shapes::shapes(), &bad);
        assert!(!by_focus(&r).is_empty());
    }
}
