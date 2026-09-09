//! The specification, as an interface a project implements.
//!
//! # The problem this solves
//!
//! Under the hub-and-spoke model the company ontology reads projects, and
//! projects never read the company ontology. That direction is not a policy —
//! it is what the transport does, and [`crate::conform`] plus the hub enforce
//! it from the outside.
//!
//! # What a project may and may not see
//!
//! The company ontology has three components, and they do not share a
//! visibility rule. Collapsing them into one — "projects never see Ontology" —
//! is the mistake this module was first written against, and it is too strong:
//!
//! | Component | Example | May a project see it? |
//! |---|---|---|
//! | **spec / schema** | this module, [`CLAUSES`], the SHACL shapes | **yes** — it holds no company data |
//! | **interface / code** | the rest of this crate | **yes** — vendored or depended on |
//! | **datastore / data** | the aggregated company graph, the project registry | **never** |
//!
//! The line is *data*, not direction. A project reading the specification
//! learns what is required of it and nothing about any other project. A project
//! reading the datastore learns what every sibling declares, which is the
//! company's aggregate view and belongs to the hub alone.
//!
//! So the specification is publishable, and [`Descriptor`] is the publishable
//! form of it: revision, obligations, shape digest, and no fact about any
//! product. [`tests::the_descriptor_carries_no_company_data`] holds it to that.
//!
//! So the standard travels **inside the vendored vocabulary**. This module is
//! the spec restated as things a compiler and a test can check: [`CLAUSES`]
//! names each normative obligation with the section it comes from, and
//! [`certify`] evaluates every one of them against a product's own
//! declaration. A project runs it in its own test suite, offline, and learns
//! precisely which clause it fails and what the spec says about it.
//!
//! ```ignore
//! #[test]
//! fn this_service_conforms_to_the_standard_it_vendors() {
//!     nervosys_ontology::spec::assert_conformant::<SovereignFrontier>();
//! }
//! ```
//!
//! Nothing there reaches the hub, and nothing needs to: the obligations arrived
//! with the copy.
//!
//! # Two independent ways to catch a stale copy
//!
//! A vendored copy cannot tell on its own that a newer one exists, so currency
//! is checked from both ends. Neither end requires the datastore.
//!
//! **From the project.** It fetches the published [`Descriptor`] — spec only,
//! no graph — and calls [`currency`]. That is a project learning it is behind,
//! by reading a document that names no other project. It is optional, because
//! it needs the network; when it is skipped, nothing is silently assumed.
//!
//! **From the hub.** Every projection carries [`REVISION`] as
//! `ns:standardRevision`. The hub compares it against the revision it holds and
//! reports anything behind as stale. This one always runs, needs nothing of the
//! project, and is the backstop for a project that never fetches.
//!
//! The project declares what it holds; the side with visibility judges whether
//! that is current. Neither path exposes the aggregate.
//!
//! # Why the checks live here and not in each project
//!
//! Every obligation in [`CLAUSES`] is generic to NS-ONTO-001, so a project
//! that hand-writes them writes the same test as every other project and
//! discovers a new clause only when someone tells it. Shipping them with the
//! vocabulary means a re-vendor delivers the new obligations as failing tests,
//! which is the earliest a project can possibly learn of them without
//! observing the hub.

use crate::capability::Manifest;
use crate::iri::{Iri, CORE};
use crate::surface::SurfaceKind;
use crate::{conform, emit, fingerprint, shapes};

/// The short name of the standard this vocabulary implements.
pub const NAME: &str = "NS-ONTO-001";

/// The revision of NS-ONTO-001 that this copy of the vocabulary implements.
///
/// Emitted into every product's graph as `ns:standardRevision`, so the hub can
/// tell a current projection from one built against an older copy. A project
/// never compares this against anything — it has nothing to compare it to.
///
/// Bumped whenever an obligation changes.
/// [`tests::changing_the_emitted_form_requires_bumping_the_revision`] forces
/// that: it pins the digest of the shape graph, so altering what the emitter
/// writes fails the build until the revision moves with it.
pub const REVISION: &str = "2026-09-09";

/// The digest of the shape graph as of [`REVISION`].
///
/// Not a second identity for the standard — a tripwire on the first. Its only
/// job is to make a form change that forgot to bump [`REVISION`] impossible to
/// commit.
#[cfg(test)]
const FORM_DIGEST_AT_REVISION: &str =
    "8c8d943010f56dfd132ea73a15c54714e660e5f8079fdfc7e05805823a61e816";

/// The IRI naming the standard itself.
///
/// Minted in the company's `core` namespace rather than a namespace of its
/// own: the pinned set in [`crate::iri::ALL`] comes verbatim from the
/// published company context, and adding a prefix that context does not
/// declare would detach this crate from the graph it emits into — quietly,
/// since the symptom is an empty query rather than an error.
pub fn iri() -> Iri {
    CORE.term(NAME)
}

/// One normative obligation, carrying the section it comes from.
///
/// The citation is the useful part. A project that fails a check should be
/// able to read the rule rather than infer it from an assertion message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Clause {
    /// Stable identifier, used in [`Finding::clause`].
    pub id: &'static str,
    /// The section of `docs/NS-ONTO-001.md` this comes from.
    pub section: &'static str,
    /// What the section requires, in one sentence.
    pub requires: &'static str,
}

/// Every obligation [`certify`] evaluates.
///
/// Exhaustive by construction: [`tests::every_clause_is_actually_evaluated`]
/// asserts that a certification reports exactly this many checks, so a clause
/// added here without a check to back it fails the build. A clause list that
/// over-reports what was verified is the same defect as a validator that
/// returns `Ok` without reading anything.
pub const CLAUSES: &[Clause] = &[
    Clause {
        id: "unmapped-stated",
        section: "§2.5",
        requires: "A manifest states what it omits; `unmapped` is not optional.",
    },
    Clause {
        id: "no-unaudited-authority",
        section: "§3.1",
        requires: "No capability ships with authority nobody has established.",
    },
    Clause {
        id: "refusal-carries-because",
        section: "§3.2",
        requires: "Every refusal carries `because`, not only a status code.",
    },
    Clause {
        id: "absence-explained",
        section: "§3.3",
        requires: "Every field that may be missing declares when and why.",
    },
    Clause {
        id: "evidence-is-not-empty",
        section: "§4",
        requires: "Evidence that is declared records something; absence stays absent.",
    },
    Clause {
        id: "capability-names-unique",
        section: "§5",
        requires: "Capability names are unique, since each mints an IRI.",
    },
    Clause {
        id: "projection-conforms-to-shapes",
        section: "§5",
        requires: "The emitted graph satisfies the published SHACL shapes.",
    },
    Clause {
        id: "graph-fingerprints",
        section: "§6",
        requires: "The emitted graph is non-empty and produces a digest.",
    },
    Clause {
        id: "declared-paths-are-served",
        section: "§7.4",
        requires: "Every declared invocation is reachable on the surface it claims.",
    },
    Clause {
        id: "served-entry-points-are-declared",
        section: "§7.4",
        requires: "Every entry point the system serves is declared or named in `unmapped`.",
    },
];

/// A clause a product failed, and what it was.
#[derive(Debug, Clone, PartialEq)]
pub struct Finding {
    /// The [`Clause::id`] that failed.
    pub clause: &'static str,
    pub section: &'static str,
    /// What was found, specifically enough to fix without re-deriving it.
    pub detail: String,
}

impl std::fmt::Display for Finding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} [{} {}]", self.detail, NAME, self.section)
    }
}

/// The result of evaluating every clause against a product.
#[derive(Debug, Clone, PartialEq)]
pub struct Certification {
    /// The revision certified against — the one this copy implements.
    pub revision: &'static str,
    /// How many clauses were actually evaluated.
    ///
    /// Reported for the reason [`crate::check::Conformance`] reports it: a run
    /// that checked nothing must not read like a run that checked everything.
    pub checked: usize,
    pub findings: Vec<Finding>,
}

impl Certification {
    /// Whether the product conforms — and actually proved it.
    ///
    /// A certification that evaluated no clauses does not conform, however
    /// empty its findings.
    pub fn conforms(&self) -> bool {
        self.findings.is_empty() && self.checked == CLAUSES.len()
    }

    /// True when nothing was evaluated. Never a pass.
    pub fn vacuous(&self) -> bool {
        self.checked == 0
    }

    pub fn summary(&self) -> String {
        if self.vacuous() {
            return format!("VACUOUS — no clause of {NAME} was evaluated");
        }
        if self.findings.is_empty() {
            format!(
                "conforms to {NAME} revision {} ({}/{} clauses)",
                self.revision,
                self.checked,
                CLAUSES.len()
            )
        } else {
            format!(
                "{} finding(s) against {NAME} revision {} ({}/{} clauses)",
                self.findings.len(),
                self.revision,
                self.checked,
                CLAUSES.len()
            )
        }
    }

    /// The full report, one finding per line.
    pub fn report(&self) -> String {
        let mut s = self.summary();
        for f in &self.findings {
            s.push_str(&format!("\n  - {f}"));
        }
        s
    }
}

/// A route the running service actually serves.
///
/// Supplied by the product, because only the product can see its own router.
/// This is the half of §7.6 that no vocabulary can check on a project's
/// behalf, and the reason [`Conformant`] is a trait rather than a function
/// over a [`Manifest`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Route {
    pub method: String,
    pub path: String,
    /// Which front door this was read from.
    ///
    /// Defaults to [`SurfaceKind::Web`] via [`Route::new`], so a product with
    /// one HTTP router declares exactly what it declared before this field
    /// existed.
    pub surface: SurfaceKind,
}

impl Route {
    /// A route served over HTTP.
    pub fn new(method: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            method: method.into(),
            path: path.into(),
            surface: SurfaceKind::Web,
        }
    }

    /// An entry point on some other surface — a CLI subcommand, an MCP tool.
    pub fn on(surface: SurfaceKind, method: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            method: method.into(),
            path: path.into(),
            surface,
        }
    }

    fn matches(&self, surface: SurfaceKind, method: &str, path: &str) -> bool {
        self.surface == surface && self.method.eq_ignore_ascii_case(method) && self.path == path
    }
}

/// What a product supplies in order to be certified.
///
/// Two methods, and the second is the one that matters. A manifest alone can
/// be checked for internal consistency, which is worth little: the failure
/// NS-ONTO-001 exists to prevent is a declaration that drifts from the running
/// system, and catching that needs something only the product can produce.
pub trait Conformant {
    /// The product's self-description.
    fn manifest() -> Manifest;

    /// Every entry point the running system serves, on every surface.
    ///
    /// Read from the router, the command parser and the tool table themselves,
    /// not from a list written alongside the declaration — two hand-maintained
    /// lists agreeing proves only that someone updated both.
    ///
    /// A product with one HTTP router returns what it always did. A product
    /// with several front doors returns all of them, tagged with
    /// [`Route::on`], and the two clauses at §7.4 then hold the declaration and
    /// the surfaces to each other in both directions.
    fn served_routes() -> Vec<Route>;
}

/// Evaluate every clause of NS-ONTO-001 against a product.
pub fn certify<P: Conformant>() -> Certification {
    certify_manifest(&P::manifest(), &P::served_routes())
}

/// Certify a manifest against routes gathered separately.
///
/// The form to use when the manifest and the router are reachable but
/// implementing [`Conformant`] would mean threading a type through a test for
/// no other purpose.
pub fn certify_manifest(m: &Manifest, routes: &[Route]) -> Certification {
    let mut findings = Vec::new();
    let mut checked = 0usize;

    // §2.5 — a model that cannot say what it omits invites the reader to
    // assume it is complete.
    checked += 1;
    if m.unmapped.is_empty() {
        findings.push(finding(
            "unmapped-stated",
            "`unmapped` is empty, which claims the model is complete".to_string(),
        ));
    }

    // §3.1 — unaudited is an honest state to pass through, never one to ship.
    checked += 1;
    for c in m.unaudited() {
        findings.push(finding(
            "no-unaudited-authority",
            format!(
                "capability `{}` has authority nobody has established",
                c.name
            ),
        ));
    }

    // §3.2 — `because` is what turns "call and see" into "plan and execute".
    checked += 1;
    for c in &m.capabilities {
        for r in &c.refusals {
            if r.because.trim().is_empty() {
                findings.push(finding(
                    "refusal-carries-because",
                    format!(
                        "capability `{}` declares a refusal ({}) with no reason",
                        c.name, r.response
                    ),
                ));
            }
        }
    }

    // §3.3 — "we looked and there is none" is not "we could not look".
    checked += 1;
    for c in &m.capabilities {
        for a in &c.may_be_absent {
            if a.when.trim().is_empty() {
                findings.push(finding(
                    "absence-explained",
                    format!(
                        "capability `{}` says `{}` may be absent without saying when",
                        c.name, a.concept
                    ),
                ));
            }
        }
    }

    // §4 — an evidence node that records nothing is the sentinel §4 forbids,
    // wearing a different hat.
    checked += 1;
    for c in &m.capabilities {
        if let Some(e) = &c.evidence {
            if e.records.trim().is_empty() {
                findings.push(finding(
                    "evidence-is-not-empty",
                    format!(
                        "capability `{}` declares evidence that records nothing",
                        c.name
                    ),
                ));
            }
        }
    }

    // §5 — each capability mints an IRI from its name, so a duplicate name
    // silently merges two capabilities into one node.
    checked += 1;
    {
        let mut names: Vec<&str> = m.capabilities.iter().map(|c| c.name).collect();
        names.sort_unstable();
        let before = names.len();
        names.dedup();
        if names.len() != before {
            findings.push(finding(
                "capability-names-unique",
                "two capabilities share a name and would mint one IRI".to_string(),
            ));
        }
    }

    // §5 — the shapes this copy carries, against the graph this copy emits.
    let quads = emit::to_quads(m);
    checked += 1;
    {
        let report = conform::check(&shapes::shapes(), &quads);
        if report.vacuous() {
            findings.push(finding(
                "projection-conforms-to-shapes",
                "the shape check evaluated no constraints, which is not a pass".to_string(),
            ));
        }
        for v in &report.violations {
            findings.push(finding("projection-conforms-to-shapes", v.to_string()));
        }
    }

    // §6 — an empty graph must not produce a digest, so a manifest that emits
    // nothing cannot be certified.
    checked += 1;
    if fingerprint::fingerprint(&quads).is_empty() {
        findings.push(finding(
            "graph-fingerprints",
            "the manifest emitted an empty graph".to_string(),
        ));
    }

    // §7.4 — the clause that needs the product, and the only one that can
    // catch a declaration drifting from the running system.
    //
    // Both directions are checked, because they catch opposite defects. A
    // declared invocation nobody serves is a promise the product does not
    // keep. A served entry point nobody declared is worse: it is invisible to
    // every agent that reads the manifest, which is the failure that does not
    // announce itself.
    checked += 1;
    for c in &m.capabilities {
        for (surface, method, path) in c.all_invocations() {
            let served = routes.iter().any(|r| r.matches(surface, method, path));
            if !served {
                findings.push(finding(
                    "declared-paths-are-served",
                    format!(
                        "capability `{}` declares {} {} on the {} surface, which the product does not serve",
                        c.name,
                        method,
                        path,
                        surface.label()
                    ),
                ));
            }
        }
    }

    // §7.4, the converse. A route is excused only by being named in
    // `unmapped` — §2.5 already requires a model to say what it leaves out,
    // and this is where that requirement acquires teeth.
    checked += 1;
    for route in routes {
        let declared = m.capabilities.iter().any(|c| {
            c.all_invocations()
                .into_iter()
                .any(|(surface, method, path)| route.matches(surface, method, path))
        });
        if declared {
            continue;
        }
        let excused = m
            .unmapped
            .iter()
            .any(|u| u.area == route.path || u.area == format!("{} {}", route.method, route.path));
        if !excused {
            findings.push(finding(
                "served-entry-points-are-declared",
                format!(
                    "the product serves {} {} on the {} surface, which no capability declares and `unmapped` does not name",
                    route.method,
                    route.path,
                    route.surface.label()
                ),
            ));
        }
    }

    Certification {
        revision: REVISION,
        checked,
        findings,
    }
}

fn finding(id: &'static str, detail: String) -> Finding {
    let clause = CLAUSES
        .iter()
        .find(|c| c.id == id)
        .expect("every finding cites a declared clause");
    Finding {
        clause: clause.id,
        section: clause.section,
        detail,
    }
}

/// Assert conformance, panicking with the full report if it fails.
///
/// The form a project puts in its own test suite. Runs offline against the
/// vendored copy and reaches nothing.
#[track_caller]
pub fn assert_conformant<P: Conformant>() {
    let c = certify::<P>();
    assert!(c.conforms(), "{}", c.report());
}

/// The clause a given finding cites, for a caller that wants the rule text.
pub fn clause(id: &str) -> Option<&'static Clause> {
    CLAUSES.iter().find(|c| c.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability::{
        Absence, Authority, Capability, Evidence, Invocation, Refusal, Unmapped,
    };
    use crate::test_support::manifest;

    fn routes_for(m: &Manifest) -> Vec<Route> {
        m.capabilities
            .iter()
            .flat_map(|c| c.all_invocations())
            .map(|(surface, method, path)| Route::on(surface, method, path))
            .collect()
    }

    fn conformant_manifest() -> Manifest {
        let mut m = manifest(vec![Capability::read(
            "thing.get",
            "GET",
            "/thing",
            "reads a thing",
        )]);
        m.unmapped.push(Unmapped {
            area: "admin",
            why: "internal, deliberately not modelled",
        });
        m
    }

    #[test]
    fn a_well_formed_product_certifies() {
        let m = conformant_manifest();
        let c = certify_manifest(&m, &routes_for(&m));
        assert!(c.conforms(), "{}", c.report());
        assert_eq!(c.checked, CLAUSES.len());
        assert_eq!(c.revision, REVISION);
    }

    /// The paired failure test §2.1 requires. A certification that cannot fail
    /// is the defect this whole crate is written against.
    #[test]
    fn certification_can_fail() {
        let mut m = conformant_manifest();
        m.unmapped.clear();
        let c = certify_manifest(&m, &routes_for(&m));
        assert!(!c.conforms());
        assert!(c.findings.iter().any(|f| f.clause == "unmapped-stated"));
    }

    #[test]
    fn a_declared_path_the_router_does_not_serve_is_caught() {
        // The drift NS-ONTO-001 §7.4 exists for: the declaration still reads
        // correctly, and the route is gone.
        let m = conformant_manifest();
        let c = certify_manifest(&m, &[Route::new("GET", "/something-else")]);
        assert!(!c.conforms());
        let f = c
            .findings
            .iter()
            .find(|f| f.clause == "declared-paths-are-served")
            .expect("the missing route is reported");
        assert!(f.detail.contains("/thing"));
        assert_eq!(f.section, "§7.4");
    }

    /// The guarantee that made this an additive change: a product with one
    /// HTTP router declares what it always declared, and still certifies.
    #[test]
    fn a_single_surface_product_is_unaffected_by_the_surface_field() {
        let m = conformant_manifest();
        let c = certify_manifest(&m, &[Route::new("GET", "/thing")]);
        assert!(c.conforms(), "{}", c.report());
    }

    #[test]
    fn a_capability_unreachable_on_a_surface_it_claims_is_caught() {
        let mut m = conformant_manifest();
        m.capabilities[0] = Capability::read("thing.get", "GET", "/thing", "reads a thing")
            .reachable(Invocation::cli("thing"))
            .reachable(Invocation::machine("get_thing"));

        // The HTTP door is served; the other two are not. This is the shape of
        // the defect exactly: a capability that exists, and agents reaching it
        // through the other doors cannot see.
        let c = certify_manifest(&m, &[Route::new("GET", "/thing")]);
        assert!(!c.conforms());

        let missed: Vec<&str> = c
            .findings
            .iter()
            .filter(|f| f.clause == "declared-paths-are-served")
            .map(|f| f.detail.as_str())
            .collect();
        assert_eq!(missed.len(), 2, "{missed:?}");
        assert!(missed.iter().any(|d| d.contains("cli")), "{missed:?}");
        assert!(missed.iter().any(|d| d.contains("machine")), "{missed:?}");
    }

    #[test]
    fn a_served_entry_point_nobody_declared_is_caught() {
        // The converse, and the one that catches a capability added to a
        // surface but never written down: it is reachable, and invisible.
        let m = conformant_manifest();
        let mut routes = routes_for(&m);
        routes.push(Route::on(SurfaceKind::Machine, "", "undeclared_tool"));

        let c = certify_manifest(&m, &routes);
        assert!(!c.conforms());
        let f = c
            .findings
            .iter()
            .find(|f| f.clause == "served-entry-points-are-declared")
            .expect("the undeclared entry point is reported");
        assert!(f.detail.contains("undeclared_tool"), "{}", f.detail);
        assert!(f.detail.contains("machine"), "{}", f.detail);
    }

    /// A product with no HTTP surface at all. Before `primary_surface` it had
    /// to declare a route it did not serve in order to be describable, which
    /// is a declaration that lies to satisfy a field.
    #[test]
    fn a_product_with_no_web_surface_declares_the_surface_it_has() {
        let mut m = conformant_manifest();
        m.capabilities[0] = Capability::cli_read("thing.get", "thing", "reads a thing")
            .reachable(Invocation::machine("get_thing"));

        let served = vec![
            Route::on(SurfaceKind::Cli, "", "thing"),
            Route::on(SurfaceKind::Machine, "", "get_thing"),
        ];
        let c = certify_manifest(&m, &served);
        assert!(c.conforms(), "{}", c.report());

        // And the web route it never served is not silently accepted for it.
        let wrong = certify_manifest(&m, &[Route::new("GET", "thing")]);
        assert!(!wrong.conforms());
    }

    #[test]
    fn a_served_entry_point_named_in_unmapped_is_excused() {
        // §2.5 already requires a model to say what it leaves out. This is
        // where that requirement acquires teeth: naming it is the way past
        // this clause, and the naming is the point.
        let mut m = conformant_manifest();
        m.unmapped.push(Unmapped {
            area: "healthz",
            why: "liveness probe; carries no product meaning",
        });
        let mut routes = routes_for(&m);
        routes.push(Route::new("GET", "healthz"));

        let c = certify_manifest(&m, &routes);
        assert!(c.conforms(), "{}", c.report());
    }

    #[test]
    fn shipping_unaudited_authority_is_a_finding() {
        let mut m = conformant_manifest();
        m.capabilities.push(
            Capability::read("thing.wipe", "POST", "/thing/wipe", "removes a thing")
                .mutating(Authority::Unaudited { note: "not probed" }, false),
        );
        let c = certify_manifest(&m, &routes_for(&m));
        assert!(c
            .findings
            .iter()
            .any(|f| f.clause == "no-unaudited-authority"));
    }

    #[test]
    fn a_refusal_without_a_reason_is_a_finding() {
        let mut m = conformant_manifest();
        m.capabilities[0].refusals.push(Refusal {
            when: "rate limited",
            response: "429",
            because: "   ",
        });
        let c = certify_manifest(&m, &routes_for(&m));
        assert!(c
            .findings
            .iter()
            .any(|f| f.clause == "refusal-carries-because"));
    }

    #[test]
    fn an_unexplained_absence_is_a_finding() {
        let mut m = conformant_manifest();
        m.capabilities[0].may_be_absent.push(Absence {
            concept: "ratio",
            when: "",
        });
        let c = certify_manifest(&m, &routes_for(&m));
        assert!(c.findings.iter().any(|f| f.clause == "absence-explained"));
    }

    #[test]
    fn evidence_that_records_nothing_is_a_finding() {
        let mut m = conformant_manifest();
        m.capabilities[0].evidence = Some(Evidence {
            records: "",
            verify_command: Some("sf verify"),
        });
        let c = certify_manifest(&m, &routes_for(&m));
        assert!(c
            .findings
            .iter()
            .any(|f| f.clause == "evidence-is-not-empty"));
    }

    #[test]
    fn two_capabilities_sharing_a_name_are_caught() {
        let mut m = conformant_manifest();
        m.capabilities.push(Capability::read(
            "thing.get",
            "GET",
            "/thing",
            "the same name again",
        ));
        let c = certify_manifest(&m, &routes_for(&m));
        assert!(c
            .findings
            .iter()
            .any(|f| f.clause == "capability-names-unique"));
    }

    #[test]
    fn a_manifest_emitting_nothing_cannot_certify() {
        // `to_quads` always writes the product node, so an empty graph is not
        // reachable through the public API. The clause is still evaluated, and
        // the check that proves it can fail lives in `fingerprint`.
        let m = manifest(Vec::new());
        let c = certify_manifest(&m, &[]);
        assert!(
            !c.conforms(),
            "an empty manifest declares nothing and omits nothing"
        );
        assert!(c.findings.iter().any(|f| f.clause == "unmapped-stated"));
    }

    #[test]
    fn a_certification_that_evaluated_nothing_is_not_a_pass() {
        let vacuous = Certification {
            revision: REVISION,
            checked: 0,
            findings: Vec::new(),
        };
        assert!(vacuous.vacuous());
        assert!(
            !vacuous.conforms(),
            "no findings over no checks is not conformance"
        );
        assert!(vacuous.summary().contains("VACUOUS"));
    }

    #[test]
    fn every_clause_is_actually_evaluated() {
        // Closes the loop on CLAUSES: a clause documented but never checked
        // would let this module over-report what it verified, which is the
        // failure mode of the compiler in `example-architecture/`.
        let m = conformant_manifest();
        let c = certify_manifest(&m, &routes_for(&m));
        assert_eq!(
            c.checked,
            CLAUSES.len(),
            "every declared clause must be evaluated, and none twice"
        );
    }

    #[test]
    fn every_clause_id_is_unique_and_cited() {
        let mut ids: Vec<&str> = CLAUSES.iter().map(|c| c.id).collect();
        let before = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), before, "duplicate clause id");
        for c in CLAUSES {
            assert!(
                c.section.starts_with('§'),
                "clause `{}` cites no section",
                c.id
            );
            assert!(!c.requires.is_empty());
        }
    }

    #[test]
    fn the_trait_form_and_the_function_form_agree() {
        struct P;
        impl Conformant for P {
            fn manifest() -> Manifest {
                conformant_manifest()
            }
            fn served_routes() -> Vec<Route> {
                routes_for(&conformant_manifest())
            }
        }
        assert_eq!(
            certify::<P>(),
            certify_manifest(&P::manifest(), &P::served_routes())
        );
        assert_conformant::<P>();
    }

    /// A form change that forgot to move the revision would leave projects
    /// declaring a revision whose obligations they do not implement, and the
    /// hub unable to tell. Pinning the digest makes that fail here instead.
    #[test]
    fn changing_the_emitted_form_requires_bumping_the_revision() {
        let digest = fingerprint::fingerprint(&shapes::shapes())
            .hex()
            .expect("the shape graph is not empty")
            .to_string();
        assert_eq!(
            digest, FORM_DIGEST_AT_REVISION,
            "the emitted form changed: bump spec::REVISION and set \
             FORM_DIGEST_AT_REVISION to {digest}"
        );
    }

    /// The structural guarantee behind the three-component split: the spec is
    /// publishable *because* it contains no company data. Reviewing for that
    /// each time is exactly the kind of check that stops happening.
    #[test]
    fn the_descriptor_carries_no_company_data() {
        let json = serde_json::to_string(&descriptor()).expect("descriptor serialises");

        // Company facts are minted under `entity:`. Nothing in the spec may
        // reach into it — that namespace is the datastore's, not the spec's.
        assert!(
            !json.contains(crate::iri::ENTITY.base),
            "the published spec names an entity IRI, which is company data"
        );

        // Nor may it carry a product's own vocabulary. This crate declares no
        // concepts, capabilities or units, and the descriptor is the surface
        // where a lapse would escape the repository.
        for leak in ["sovereign", "buildingops", "frontier"] {
            assert!(
                !json.to_lowercase().contains(leak),
                "the published spec mentions `{leak}`, which belongs to a product"
            );
        }
    }

    #[test]
    fn the_shape_graph_names_no_product_either() {
        // The shapes ship alongside the descriptor, so they inherit the same
        // obligation: they constrain classes, never instances.
        let nq = crate::emit::to_nquads(&shapes::shapes());
        assert!(
            !nq.contains(crate::iri::ENTITY.base),
            "a shape names a company entity"
        );
    }

    #[test]
    fn a_descriptor_round_trips() {
        let d = descriptor();
        let json = serde_json::to_string(&d).unwrap();
        let back: Descriptor = serde_json::from_str(&json).unwrap();
        assert_eq!(d, back);
        assert_eq!(back.clauses.len(), CLAUSES.len());
        assert_eq!(back.revision, REVISION);
    }

    #[test]
    fn the_descriptor_cannot_advertise_an_unchecked_obligation() {
        // Built from the same constant `certify` walks, so the published list
        // and the evaluated list are one thing rather than two that agree.
        let d = descriptor();
        let m = conformant_manifest();
        assert_eq!(
            d.clauses.len(),
            certify_manifest(&m, &routes_for(&m)).checked
        );
    }

    #[test]
    fn a_project_on_the_published_revision_is_current() {
        assert_eq!(currency(&descriptor()), Currency::Current);
        assert!(currency(&descriptor()).is_current());
    }

    /// The case the whole descriptor exists for: a project fetches the spec,
    /// finds it has moved, and learns that without reading one company fact.
    #[test]
    fn a_project_holding_an_older_revision_learns_it_is_behind() {
        let mut published = descriptor();
        published.revision = "9999-01-01".to_string();
        let c = currency(&published);
        assert!(!c.is_current());
        assert!(matches!(c, Currency::Behind { .. }), "{c:?}");
        assert!(c.explain().contains("re-vendor"));
    }

    #[test]
    fn a_copy_ahead_of_the_publisher_is_not_reported_as_current() {
        let mut published = descriptor();
        published.revision = "1970-01-01".to_string();
        let c = currency(&published);
        assert!(
            !c.is_current(),
            "disagreeing with the publisher is not a pass"
        );
        assert!(matches!(c, Currency::Diverged { .. }), "{c:?}");
    }

    #[test]
    fn a_descriptor_for_another_standard_is_rejected() {
        let mut published = descriptor();
        published.standard = "SOME-OTHER-SPEC".to_string();
        assert!(matches!(
            currency(&published),
            Currency::NotTheSameStandard { .. }
        ));
    }

    #[test]
    fn the_standard_iri_sits_in_a_pinned_namespace() {
        assert_eq!(
            iri().as_str(),
            "https://ontology.nervosys.com/core/NS-ONTO-001"
        );
        assert!(crate::iri::ALL
            .iter()
            .any(|ns| iri().as_str().starts_with(ns.base)));
    }
}

/// The publishable description of the standard.
///
/// This is the **spec component** in serialisable form: what a project needs in
/// order to know what is required of it and whether its copy is current. It
/// carries the revision, the obligations, and the digest of the shape graph.
///
/// It carries no fact about any product — no project name, no capability, no
/// endpoint, nothing minted under the `entity:` namespace where company data
/// lives. That is what makes it safe to serve publicly while the aggregated
/// graph stays behind the hub, and
/// [`tests::the_descriptor_carries_no_company_data`] enforces it rather than
/// leaving it to review.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Descriptor {
    /// The standard's short name — `"NS-ONTO-001"`.
    pub standard: String,
    /// The revision this describes.
    pub revision: String,
    /// Digest of the shape graph at this revision, so a consumer can tell that
    /// the shapes it holds are the ones this revision means.
    pub shape_digest: String,
    /// Every obligation, with the section it comes from.
    pub clauses: Vec<DescribedClause>,
}

/// A [`Clause`] in owned, serialisable form.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DescribedClause {
    pub id: String,
    pub section: String,
    pub requires: String,
}

/// The descriptor for the revision this copy implements.
///
/// A hub serves this; a project fetches it and compares. Deliberately built
/// from the same constants [`certify`] evaluates, so a descriptor cannot
/// advertise an obligation the implementation does not check.
pub fn descriptor() -> Descriptor {
    Descriptor {
        standard: NAME.to_string(),
        revision: REVISION.to_string(),
        shape_digest: fingerprint::fingerprint(&shapes::shapes())
            .hex()
            .expect("the shape graph is never empty")
            .to_string(),
        clauses: CLAUSES
            .iter()
            .map(|c| DescribedClause {
                id: c.id.to_string(),
                section: c.section.to_string(),
                requires: c.requires.to_string(),
            })
            .collect(),
    }
}

/// How this copy of the vocabulary stands against a published descriptor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Currency {
    /// Same revision. Nothing to do.
    Current,
    /// The published standard has moved on; re-vendor.
    Behind { latest: String, held: &'static str },
    /// This copy claims a revision the publisher does not — a local edit, or a
    /// descriptor fetched from somewhere that is not the hub.
    ///
    /// Not reported as "current": a copy that disagrees with the publisher
    /// about what exists is a fact worth surfacing, not a pass.
    Diverged {
        published: String,
        held: &'static str,
    },
    /// The descriptor is for a different standard entirely.
    NotTheSameStandard { published: String },
}

impl Currency {
    /// Whether this copy is the published revision.
    pub fn is_current(&self) -> bool {
        matches!(self, Currency::Current)
    }

    pub fn explain(&self) -> String {
        match self {
            Currency::Current => format!("current with {NAME} {REVISION}"),
            Currency::Behind { latest, held } => {
                format!("behind: holds {held}, published is {latest} — re-vendor the vocabulary")
            }
            Currency::Diverged { published, held } => {
                format!("diverged: holds {held}, publisher has {published}")
            }
            Currency::NotTheSameStandard { published } => {
                format!("descriptor is for {published}, not {NAME}")
            }
        }
    }
}

/// Compare a fetched [`Descriptor`] against the copy this code was built with.
///
/// The optional, project-side half of the currency check. The project supplies
/// the descriptor — this crate does no I/O, because it is vendored into
/// products and a vocabulary that opens sockets is not one.
///
/// Comparing revisions rather than digests on purpose: a digest mismatch cannot
/// distinguish "older" from "tampered with", and answering the first question
/// with evidence for the second is how a version check becomes a security
/// claim it cannot support.
pub fn currency(published: &Descriptor) -> Currency {
    if published.standard != NAME {
        return Currency::NotTheSameStandard {
            published: published.standard.clone(),
        };
    }
    if published.revision == REVISION {
        Currency::Current
    } else if published.revision.as_str() > REVISION {
        Currency::Behind {
            latest: published.revision.clone(),
            held: REVISION,
        }
    } else {
        Currency::Diverged {
            published: published.revision.clone(),
            held: REVISION,
        }
    }
}
