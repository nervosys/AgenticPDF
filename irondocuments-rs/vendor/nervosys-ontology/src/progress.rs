//! **NS-PROG-001** — goals, milestones and how they are known.
//!
//! # Why this is a sibling specification rather than part of NS-ONTO-001
//!
//! NS-ONTO-001 describes *what a running system does*: what it can be asked,
//! what a caller must hold, how it declines. That model is held to reality by
//! tests inside the product, because its failure mode is drifting from the
//! running code.
//!
//! A milestone is not about the running system. It is about the team building
//! it, and it has the opposite failure mode: it drifts from *the world*, and no
//! test inside the product can catch that. Folding the two together would give
//! one specification two subjects and two contradictory notions of what makes a
//! statement true — so the names are kept apart, exactly as NS-ONTO-001 and
//! EXAMPLE-ONTO-000 are.
//!
//! What they share is the graph. A [`Programme`] emits into the same company
//! IRIs as a [`crate::Manifest`], so one query reaches both: *which projects
//! have an unaudited capability and an in-progress milestone that claims to fix
//! it* is a join, not a meeting.
//!
//! # Declared and observed are different facts
//!
//! There are two ways to know a milestone, and they are not interchangeable:
//!
//! - A project **declares** it, compiled into the binary. Trustworthy about
//!   intent, and stale the moment the board moves.
//! - The hub **observes** it from a tracker. Current, and reflection — which
//!   NS-ONTO-001 §2.4 rejects for capabilities, because reflection can report
//!   that a route exists but never what it asserts.
//!
//! Both are legitimate here, and the resolution is not to choose: it is to
//! refuse to let them share a field. [`Provenance`] is required on every
//! milestone, so "the team says this is done" and "the tracker said this was
//! done at 14:02" are never rendered as the same claim. That is §2.2 applied to
//! progress — the defect being avoided is a dashboard that shows a green card
//! without saying who said so.
//!
//! # There is no percentage
//!
//! Deliberately. A hand-entered "70% complete" is the purest form of the value
//! that looks like an answer: it is precise, it is unfalsifiable, and it is
//! usually wrong. Progress here is a [`Stage`] plus the history of stage
//! changes, which the datastore already keeps per run — a number that can be
//! derived and audited rather than asserted.

// Serialize only. These types hold `&'static str`, so a programme is emitted
// rather than round-tripped; a consumer parses the JSON or the RDF, not this
// type. Deriving Deserialize would require the deserializer's data to outlive
// the program, which is not a thing a caller can supply.
use serde::Serialize;

use crate::capability::Evidence;
use crate::iri::{core_terms, rdf, xsd, Iri};
use crate::term::{Literal, Quad};

/// The short name of this specification.
pub const NAME: &str = "NS-PROG-001";

/// The revision this copy implements.
///
/// Versioned separately from [`crate::spec::REVISION`] on purpose: a change to
/// how capabilities are described should not invalidate every project's
/// milestones, and vice versa. Two subjects, two clocks.
pub const REVISION: &str = "2026-09-02";

/// Where a milestone sits — the columns of the board.
///
/// A closed set, so a query can filter on it and a dashboard cannot invent a
/// column. Two of the variants carry a reason, for the same rule that makes
/// [`crate::Refusal::because`] mandatory: the states that stop work are exactly
/// the ones where "why" is the only useful part.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "stage", rename_all = "kebab-case")]
pub enum Stage {
    /// Written down, not yet committed to.
    Proposed,
    /// Committed to, not yet started.
    Accepted,
    InProgress,
    /// Started and stopped. Carries what is blocking it, because a blocked card
    /// with no reason is a card nobody can act on — and it will sit there.
    Blocked {
        because: &'static str,
    },
    Done,
    /// Deliberately not done. Distinct from [`Stage::Done`] and from deletion:
    /// a milestone that vanishes leaves the reader to assume it shipped.
    Abandoned {
        because: &'static str,
    },
}

impl Stage {
    /// The board column, as a stable string.
    pub fn column(&self) -> &'static str {
        match self {
            Stage::Proposed => "proposed",
            Stage::Accepted => "accepted",
            Stage::InProgress => "in-progress",
            Stage::Blocked { .. } => "blocked",
            Stage::Done => "done",
            Stage::Abandoned { .. } => "abandoned",
        }
    }

    /// Why work is not proceeding, where that applies.
    pub fn because(&self) -> Option<&'static str> {
        match self {
            Stage::Blocked { because } | Stage::Abandoned { because } => Some(because),
            _ => None,
        }
    }

    /// Whether this stage is a resting state rather than work in flight.
    pub fn is_settled(&self) -> bool {
        matches!(self, Stage::Done | Stage::Abandoned { .. })
    }

    /// Whether work is stopped and should be visible as such.
    pub fn needs_attention(&self) -> bool {
        matches!(self, Stage::Blocked { .. })
    }
}

/// How a milestone came to be known.
///
/// Required, never defaulted. A dashboard that cannot say whether a card
/// reflects the team's declaration or a tracker's state is showing two
/// different facts in one colour.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "provenance", rename_all = "kebab-case")]
pub enum Provenance {
    /// The project declared it, compiled into its own binary.
    ///
    /// Authoritative about intent. Says nothing about currency: it is exactly
    /// as fresh as the last deploy, which is why [`Milestone::is_current_as_of`]
    /// refuses to answer for it.
    Declared,

    /// Read by the hub from somewhere that tracks the work.
    ///
    /// Reflection, and named as such. `source` identifies the system; `at` is
    /// when it was read, so a stale board is visibly stale rather than
    /// confidently wrong.
    Observed {
        source: &'static str,
        /// ISO-8601 instant the observation was made.
        at: &'static str,
    },
}

impl Provenance {
    pub fn label(&self) -> String {
        match self {
            Provenance::Declared => "declared".into(),
            Provenance::Observed { source, .. } => format!("observed:{source}"),
        }
    }

    /// The moment this was known to be true, if that is knowable.
    ///
    /// `None` for [`Provenance::Declared`] — not because nothing is known, but
    /// because a declaration's timestamp would be the build date, and rendering
    /// a build date as "last updated" is the kind of plausible number that
    /// stops anyone asking.
    pub fn observed_at(&self) -> Option<&'static str> {
        match self {
            Provenance::Observed { at, .. } => Some(at),
            Provenance::Declared => None,
        }
    }
}

/// Something a project is trying to achieve.
///
/// Coarser than a milestone and longer-lived. Carries `why` for the same reason
/// a refusal does: a goal nobody can motivate is one nobody will defend when it
/// competes with something urgent.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Goal {
    pub id: &'static str,
    pub statement: &'static str,
    pub why: &'static str,
}

/// A unit of work with a state.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Milestone {
    pub id: &'static str,
    pub title: &'static str,
    pub stage: Stage,
    /// The [`Goal::id`] this advances, if any. Optional because not all work
    /// serves a stated goal, and pretending otherwise invites invented goals.
    pub advances: Option<&'static str>,
    /// The id this work carries on the tracker, if it is tracked.
    ///
    /// The link that makes declared and observed claims about the same work
    /// comparable. Without it a board shows both and looks like it is
    /// double-counting, and a reader cannot tell whether the two agree —
    /// which is the single most useful thing two sources can tell you.
    ///
    /// Deliberately not a merge. NS-PROG-001 §2.1 keeps declared and observed
    /// apart; this pairs them so a disagreement is visible, rather than
    /// collapsing them so it is not.
    pub tracked_as: Option<&'static str>,
    /// The name of a NS-ONTO-001 capability this work is about.
    ///
    /// The join between the two specifications, and the reason it is a field
    /// rather than something a query infers. Without it, "risk and the work on
    /// it" can only be answered by pairing every risk in a project with every
    /// milestone in that project — which produces rows that look like findings
    /// and correlate nothing. A milestone that claims to address a capability
    /// says which one, or the claim is not made.
    pub addresses: Option<&'static str>,
    /// Milestone ids this waits on.
    pub blocked_by: Vec<&'static str>,
    pub provenance: Provenance,
    /// How a reader could check the claim — a PR, a released tag, a run.
    ///
    /// Reused from NS-ONTO-001 deliberately: "done" and "declines with a 403"
    /// are both claims, and both are worth more when they name what would
    /// settle the question.
    pub evidence: Option<Evidence>,
}

impl Milestone {
    /// A declared milestone in the given stage.
    pub fn declared(id: &'static str, title: &'static str, stage: Stage) -> Self {
        Self {
            id,
            title,
            stage,
            advances: None,
            addresses: None,
            tracked_as: None,
            blocked_by: Vec::new(),
            provenance: Provenance::Declared,
            evidence: None,
        }
    }

    /// A milestone the hub read from a tracker.
    pub fn observed(
        id: &'static str,
        title: &'static str,
        stage: Stage,
        source: &'static str,
        at: &'static str,
    ) -> Self {
        Self {
            provenance: Provenance::Observed { source, at },
            ..Self::declared(id, title, stage)
        }
    }

    pub fn advancing(mut self, goal: &'static str) -> Self {
        self.advances = Some(goal);
        self
    }

    /// Declare that this work is about a named capability.
    pub fn addressing(mut self, capability: &'static str) -> Self {
        self.addresses = Some(capability);
        self
    }

    /// Declare the tracker id this work appears under — `"issue-14"`.
    pub fn tracked_as(mut self, id: &'static str) -> Self {
        self.tracked_as = Some(id);
        self
    }

    pub fn waiting_on(mut self, ids: &[&'static str]) -> Self {
        self.blocked_by = ids.to_vec();
        self
    }

    pub fn with_evidence(mut self, e: Evidence) -> Self {
        self.evidence = Some(e);
        self
    }

    /// Whether this was known to be true at or after the given instant.
    ///
    /// `None` for a declared milestone, whose currency is not a fact this crate
    /// holds. Returning `false` there would assert staleness nobody measured;
    /// returning `true` would assert freshness nobody measured. §2.2: the
    /// honest answer is not a boolean.
    pub fn is_current_as_of(&self, instant: &str) -> Option<bool> {
        self.provenance.observed_at().map(|at| at >= instant)
    }
}

/// What a project is working on, as a whole.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Programme {
    /// The same base IRI the product's [`crate::Manifest`] uses, so goals and
    /// capabilities hang off one entity rather than two that look alike.
    pub base: Iri,
    pub goals: Vec<Goal>,
    pub milestones: Vec<Milestone>,
}

impl Programme {
    pub fn milestone(&self, id: &str) -> Option<&Milestone> {
        self.milestones.iter().find(|m| m.id == id)
    }

    /// Milestones in one board column.
    pub fn column(&self, column: &str) -> Vec<&Milestone> {
        self.milestones
            .iter()
            .filter(|m| m.stage.column() == column)
            .collect()
    }

    /// Everything stopped, which is the list a standup should start from.
    pub fn blocked(&self) -> Vec<&Milestone> {
        self.milestones
            .iter()
            .filter(|m| m.stage.needs_attention())
            .collect()
    }

    /// Milestones naming a goal this programme does not declare.
    ///
    /// A dangling reference renders as a card attached to nothing, which reads
    /// as "no goal" rather than as the error it is.
    pub fn dangling_goals(&self) -> Vec<&Milestone> {
        self.milestones
            .iter()
            .filter(|m| {
                m.advances
                    .is_some_and(|g| !self.goals.iter().any(|goal| goal.id == g))
            })
            .collect()
    }

    /// Milestones waiting on an id this programme does not declare.
    pub fn dangling_blockers(&self) -> Vec<&Milestone> {
        self.milestones
            .iter()
            .filter(|m| {
                m.blocked_by
                    .iter()
                    .any(|b| !self.milestones.iter().any(|o| o.id == *b))
            })
            .collect()
    }
}

fn goal_iri(p: &Programme, id: &str) -> Iri {
    Iri::parse(format!(
        "{}/goal/{}",
        p.base.as_str().trim_end_matches('/'),
        id
    ))
    .expect("base is valid and ids are path-safe")
}

/// Emit a programme as RDF, into the same company vocabulary as a manifest.
/// A milestone flattened to borrowed strings.
///
/// One emitter, two sources. A declaration holds `&'static str` because it is
/// compiled in; an observation holds `String` because it came off the wire a
/// moment ago. Without this both would need their own emission code — two
/// things that must agree, kept in two places, which is the defect this
/// codebase keeps paying for.
///
/// Constructing one directly is how the hub emits what it observed. Everything
/// a projection needs is here and nothing is optional-by-omission: a field that
/// does not apply is `None`, and `None` emits no statement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MilestoneView<'a> {
    pub id: &'a str,
    pub title: &'a str,
    /// The board column — one of [`Stage::column`]'s values.
    pub stage: &'a str,
    /// Present exactly when the stage stopped the work.
    pub blocked_because: Option<&'a str>,
    /// `declared` or `observed:<source>`. Never absent.
    pub provenance: &'a str,
    /// ISO-8601, and only ever on an observation.
    pub observed_at: Option<&'a str>,
    pub advances: Option<&'a str>,
    pub addresses: Option<&'a str>,
    /// The tracker-side milestone id this one corresponds to.
    pub tracked_as: Option<&'a str>,
    pub blocked_by: &'a [&'a str],
    /// `(records, verify_command)`.
    pub evidence: Option<(&'a str, Option<&'a str>)>,
}

fn base_of(base: &Iri) -> &str {
    base.as_str().trim_end_matches('/')
}

/// Emit one milestone beneath `base`.
///
/// The single place NS-PROG-001's projection is written. `crate::progress`
/// calls it for declared milestones and the hub calls it for observed ones, so
/// a change to the projection reaches both by construction.
pub fn milestone_quads(base: &Iri, m: &MilestoneView<'_>) -> Vec<Quad> {
    let mut out = Vec::new();
    let mi = Iri::parse(format!("{}/milestone/{}", base_of(base), m.id))
        .expect("base is valid and ids are path-safe");

    out.push(Quad::new(
        mi.clone(),
        rdf::TYPE.clone(),
        core_terms::MILESTONE.clone(),
    ));
    out.push(Quad::new(
        mi.clone(),
        core_terms::NAME.clone(),
        Literal::plain(m.title),
    ));
    out.push(Quad::new(
        mi.clone(),
        core_terms::STAGE.clone(),
        Literal::plain(m.stage),
    ));
    out.push(Quad::new(
        base.clone(),
        core_terms::HAS_MILESTONE.clone(),
        mi.clone(),
    ));

    // Required, so a consumer never has to guess how a card was learned.
    out.push(Quad::new(
        mi.clone(),
        core_terms::PROVENANCE.clone(),
        Literal::plain(m.provenance),
    ));
    if let Some(at) = m.observed_at {
        out.push(Quad::new(
            mi.clone(),
            core_terms::OBSERVED_AT.clone(),
            Literal::typed(at, xsd::DATE_TIME.clone()),
        ));
    }

    // Only where the stage has one. A blocked card always does; the others
    // emit nothing rather than an empty reason.
    if let Some(why) = m.blocked_because {
        out.push(Quad::new(
            mi.clone(),
            core_terms::BLOCKED_BECAUSE.clone(),
            Literal::plain(why),
        ));
    }

    if let Some(goal) = m.advances {
        let gi = Iri::parse(format!("{}/goal/{}", base_of(base), goal)).expect("valid");
        out.push(Quad::new(mi.clone(), core_terms::ADVANCES_GOAL.clone(), gi));
    }
    // Minted the way `crate::emit` mints it, so the two specifications
    // reference one node rather than two strings that happen to match.
    if let Some(cap) = m.addresses {
        let ci = Iri::parse(format!("{}/capability/{}", base_of(base), cap)).expect("valid");
        out.push(Quad::new(mi.clone(), core_terms::ADDRESSES.clone(), ci));
    }
    // An edge, not a label. The observed milestone is a node in this same
    // graph, so pointing at it lets a query join rather than string-match.
    if let Some(t) = m.tracked_as {
        let ti = Iri::parse(format!("{}/milestone/{}", base_of(base), t)).expect("valid");
        out.push(Quad::new(mi.clone(), core_terms::TRACKED_AS.clone(), ti));
    }

    for b in m.blocked_by {
        let bi = Iri::parse(format!("{}/milestone/{}", base_of(base), b)).expect("valid");
        out.push(Quad::new(mi.clone(), core_terms::BLOCKED_BY.clone(), bi));
    }

    if let Some((records, verify)) = m.evidence {
        let ei =
            Iri::parse(format!("{}/milestone/{}/evidence", base_of(base), m.id)).expect("valid");
        out.push(Quad::new(
            ei.clone(),
            rdf::TYPE.clone(),
            core_terms::EVIDENCE.clone(),
        ));
        out.push(Quad::new(
            ei.clone(),
            core_terms::DESCRIPTION.clone(),
            Literal::plain(records),
        ));
        if let Some(cmd) = verify {
            out.push(Quad::new(
                ei.clone(),
                core_terms::NAME.clone(),
                Literal::plain(cmd),
            ));
        }
        out.push(Quad::new(mi.clone(), core_terms::HAS_EVIDENCE.clone(), ei));
    }

    out
}

impl Milestone {
    /// Borrow this milestone in the shape the emitter takes.
    ///
    /// `provenance` is returned by value because `Provenance::label` builds a
    /// string; the caller holds it for the borrow's lifetime.
    pub fn view<'a>(&'a self, provenance: &'a str) -> MilestoneView<'a> {
        MilestoneView {
            id: self.id,
            title: self.title,
            stage: self.stage.column(),
            blocked_because: self.stage.because(),
            provenance,
            observed_at: self.provenance.observed_at(),
            advances: self.advances,
            addresses: self.addresses,
            tracked_as: self.tracked_as,
            blocked_by: &self.blocked_by,
            evidence: self
                .evidence
                .as_ref()
                .map(|e| (e.records, e.verify_command)),
        }
    }
}

/// Emit a programme as RDF, into the same company vocabulary as a manifest.
pub fn to_quads(p: &Programme) -> Vec<Quad> {
    let mut out = Vec::new();

    for g in &p.goals {
        let gi = goal_iri(p, g.id);
        out.push(Quad::new(
            gi.clone(),
            rdf::TYPE.clone(),
            core_terms::GOAL.clone(),
        ));
        out.push(Quad::new(
            gi.clone(),
            core_terms::NAME.clone(),
            Literal::plain(g.statement),
        ));
        out.push(Quad::new(
            gi.clone(),
            core_terms::DESCRIPTION.clone(),
            Literal::plain(g.why),
        ));
        out.push(Quad::new(
            p.base.clone(),
            core_terms::PURSUES_GOAL.clone(),
            gi,
        ));
    }

    for m in &p.milestones {
        let label = m.provenance.label();
        out.extend(milestone_quads(&p.base, &m.view(&label)));
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn programme(milestones: Vec<Milestone>) -> Programme {
        Programme {
            base: Iri::parse("https://ontology.nervosys.com/entity/test-product").unwrap(),
            goals: vec![Goal {
                id: "g1",
                statement: "every product describes itself",
                why: "an agent should not have to read source",
            }],
            milestones,
        }
    }

    #[test]
    fn a_blocked_milestone_cannot_exist_without_a_reason() {
        // Enforced by the type, not by a check that could be skipped: there is
        // no way to spell a blocked stage that carries no `because`.
        let s = Stage::Blocked {
            because: "waiting on the vendor",
        };
        assert_eq!(s.because(), Some("waiting on the vendor"));
        assert!(s.needs_attention());
        assert!(!s.is_settled());
        assert_eq!(Stage::InProgress.because(), None);
    }

    #[test]
    fn abandoned_is_not_done_and_neither_is_deletion() {
        let done = Stage::Done;
        let dropped = Stage::Abandoned {
            because: "the customer withdrew the requirement",
        };
        assert!(done.is_settled() && dropped.is_settled());
        assert_ne!(done.column(), dropped.column());
        assert!(
            dropped.because().is_some(),
            "abandoning something needs a reason"
        );
    }

    #[test]
    fn declared_and_observed_do_not_render_the_same() {
        let d = Milestone::declared("m1", "ship it", Stage::Done);
        let o = Milestone::observed(
            "m1",
            "ship it",
            Stage::Done,
            "github",
            "2026-09-02T14:02:00Z",
        );
        assert_ne!(d.provenance.label(), o.provenance.label());
        assert_eq!(d.provenance.observed_at(), None);
        assert_eq!(o.provenance.observed_at(), Some("2026-09-02T14:02:00Z"));
    }

    /// The §2.2 case: currency of a declared milestone is not a boolean.
    #[test]
    fn a_declared_milestone_will_not_claim_to_be_current() {
        let d = Milestone::declared("m1", "ship it", Stage::InProgress);
        assert_eq!(
            d.is_current_as_of("2026-09-01T00:00:00Z"),
            None,
            "a declaration's currency is not known, and must not be guessed either way"
        );

        let fresh = Milestone::observed(
            "m2",
            "ship it",
            Stage::InProgress,
            "github",
            "2026-09-02T00:00:00Z",
        );
        assert_eq!(fresh.is_current_as_of("2026-09-01T00:00:00Z"), Some(true));
        assert_eq!(fresh.is_current_as_of("2026-09-03T00:00:00Z"), Some(false));
    }

    #[test]
    fn a_blocked_milestone_emits_its_reason() {
        let p = programme(vec![Milestone::declared(
            "m1",
            "adopt the vocabulary",
            Stage::Blocked {
                because: "Property.unit is still the product's own enum",
            },
        )]);
        let nq = crate::emit::to_nquads(&to_quads(&p));
        assert!(nq.contains("blockedBecause"));
        assert!(nq.contains("Property.unit is still"));
    }

    #[test]
    fn an_unblocked_milestone_emits_no_reason_predicate() {
        // Absence stays absent: an empty reason would make "blocked with no
        // stated cause" unspellable and every consumer learn to ignore "".
        let p = programme(vec![Milestone::declared("m1", "x", Stage::InProgress)]);
        let nq = crate::emit::to_nquads(&to_quads(&p));
        assert!(!nq.contains("blockedBecause"));
    }

    #[test]
    fn provenance_is_always_emitted() {
        for m in [
            Milestone::declared("m1", "x", Stage::Done),
            Milestone::observed("m2", "y", Stage::Done, "github", "2026-09-02T00:00:00Z"),
        ] {
            let nq = crate::emit::to_nquads(&to_quads(&programme(vec![m])));
            assert!(
                nq.contains("provenance"),
                "every milestone must say how it is known"
            );
        }
    }

    #[test]
    fn an_observation_carries_a_typed_timestamp() {
        let p = programme(vec![Milestone::observed(
            "m1",
            "x",
            Stage::Done,
            "github",
            "2026-09-02T14:02:00Z",
        )]);
        let nq = crate::emit::to_nquads(&to_quads(&p));
        assert!(nq.contains("observedAt"));
        assert!(
            nq.contains("XMLSchema#dateTime"),
            "a timestamp must carry its datatype"
        );
    }

    #[test]
    fn a_declaration_emits_no_observation_time() {
        let p = programme(vec![Milestone::declared("m1", "x", Stage::Done)]);
        assert!(!crate::emit::to_nquads(&to_quads(&p)).contains("observedAt"));
    }

    #[test]
    fn the_board_groups_by_column() {
        let p = programme(vec![
            Milestone::declared("m1", "a", Stage::InProgress),
            Milestone::declared("m2", "b", Stage::InProgress),
            Milestone::declared(
                "m3",
                "c",
                Stage::Blocked {
                    because: "no reviewer",
                },
            ),
        ]);
        assert_eq!(p.column("in-progress").len(), 2);
        assert_eq!(p.column("blocked").len(), 1);
        assert_eq!(p.column("done").len(), 0);
        assert_eq!(p.blocked().len(), 1);
    }

    #[test]
    fn a_milestone_naming_an_undeclared_goal_is_findable() {
        let p = programme(vec![
            Milestone::declared("m1", "a", Stage::Done).advancing("g1"),
            Milestone::declared("m2", "b", Stage::Done).advancing("does-not-exist"),
        ]);
        let dangling = p.dangling_goals();
        assert_eq!(dangling.len(), 1);
        assert_eq!(dangling[0].id, "m2");
    }

    #[test]
    fn a_milestone_waiting_on_nothing_real_is_findable() {
        let p = programme(vec![
            Milestone::declared("m1", "a", Stage::Done),
            Milestone::declared("m2", "b", Stage::InProgress).waiting_on(&["m1", "ghost"]),
        ]);
        assert_eq!(p.dangling_blockers().len(), 1);
    }

    #[test]
    fn goals_and_milestones_hang_off_the_product_entity() {
        // The join with NS-ONTO-001: one entity carries both what it does and
        // what it is trying to do, so a query can reach across.
        let p = programme(vec![
            Milestone::declared("m1", "a", Stage::Done).advancing("g1")
        ]);
        let nq = crate::emit::to_nquads(&to_quads(&p));
        assert!(nq.contains("pursuesGoal"));
        assert!(nq.contains("hasMilestone"));
        assert!(nq.contains("advancesGoal"));
        assert!(nq.contains("/entity/test-product/milestone/m1"));
    }

    /// The join between the two specifications must reference the *same* node
    /// the capability emitter mints, not a string that resembles it.
    #[test]
    fn addressing_a_capability_points_at_the_capability_iri() {
        let base = "https://ontology.nervosys.com/entity/test-product";
        let p = Programme {
            base: Iri::parse(base).unwrap(),
            goals: Vec::new(),
            milestones: vec![Milestone::declared("m1", "fix it", Stage::InProgress)
                .addressing("artifact.submit")],
        };
        let nq = crate::emit::to_nquads(&to_quads(&p));
        assert!(nq.contains(&format!("<{base}/capability/artifact.submit>")));

        // And it is the IRI `crate::emit` would mint for that capability.
        let mut m = crate::test_support::manifest(vec![crate::Capability::read(
            "artifact.submit",
            "POST",
            "/submit",
            "p",
        )]);
        m.base = Iri::parse(base).unwrap();
        let cap_nq = crate::emit::to_nquads(&crate::emit::to_quads(&m));
        assert!(cap_nq.contains(&format!("<{base}/capability/artifact.submit>")));
    }

    #[test]
    fn tracking_points_at_the_milestone_the_tracker_would_mint() {
        // The hub mints observed milestones as `<base>/milestone/issue-N`, so a
        // declaration tracked as `issue-14` must resolve to that same node or
        // the reconciliation join silently matches nothing.
        let base = "https://ontology.nervosys.com/entity/test-product";
        let p = Programme {
            base: Iri::parse(base).unwrap(),
            goals: Vec::new(),
            milestones: vec![
                Milestone::declared("m1", "x", Stage::InProgress).tracked_as("issue-14")
            ],
        };
        let nq = crate::emit::to_nquads(&to_quads(&p));
        assert!(nq.contains("trackedAs"));
        assert!(nq.contains(&format!("<{base}/milestone/issue-14>")));
    }

    #[test]
    fn an_untracked_milestone_emits_no_tracking_link() {
        let p = programme(vec![Milestone::declared("m1", "x", Stage::Done)]);
        assert!(!crate::emit::to_nquads(&to_quads(&p)).contains("trackedAs"));
    }

    #[test]
    fn a_milestone_addressing_nothing_emits_no_link() {
        let p = programme(vec![Milestone::declared("m1", "x", Stage::Done)]);
        assert!(!crate::emit::to_nquads(&to_quads(&p)).contains("addresses"));
    }

    #[test]
    fn emitting_is_stable_and_content_dependent() {
        let a = programme(vec![Milestone::declared("m1", "a", Stage::InProgress)]);
        let b = programme(vec![Milestone::declared("m1", "a", Stage::Done)]);
        assert_eq!(
            crate::emit::to_nquads(&to_quads(&a)),
            crate::emit::to_nquads(&to_quads(&a))
        );
        assert_ne!(
            crate::fingerprint::fingerprint(&to_quads(&a)),
            crate::fingerprint::fingerprint(&to_quads(&b)),
            "moving a card must change the graph"
        );
    }

    #[test]
    fn a_programme_with_no_milestones_emits_no_milestone_statements() {
        let p = programme(Vec::new());
        let quads = to_quads(&p);
        assert!(!quads.is_empty(), "the goal is still emitted");
        assert!(!crate::emit::to_nquads(&quads).contains("Milestone"));
    }
}

/// SHACL shapes for the graph [`to_quads`] produces.
///
/// Separate from [`crate::shapes::shapes`] on purpose. Those describe
/// NS-ONTO-001, and folding these in would make every change to how milestones
/// are described bump the capability specification's revision — two subjects
/// sharing one clock, which is the thing this module exists to avoid.
///
/// A graph may of course carry both. SHACL targets by class, so the two shape
/// graphs compose without interfering: concatenate them and check once.
pub fn shapes() -> Vec<Quad> {
    use crate::shapes::{node_shape, PropertyShape};

    let mut out = Vec::new();

    node_shape(
        "MilestoneShape",
        core_terms::MILESTONE.clone(),
        vec![
            PropertyShape::on(core_terms::NAME.clone()).exactly_one(),
            PropertyShape::on(core_terms::STAGE.clone()).exactly_one(),
            // Required. A card that cannot say whether it was declared or
            // observed is the one thing this vocabulary refuses to represent.
            PropertyShape::on(core_terms::PROVENANCE.clone()).exactly_one(),
            // At most one, and optional: only an observation has a time, and a
            // declaration emitting one would be asserting a freshness nobody
            // measured.
            PropertyShape::on(core_terms::OBSERVED_AT.clone())
                .at_most_one()
                .datatype(xsd::DATE_TIME.clone()),
            // Optional for the same reason: present exactly when the stage
            // stopped the work, absent otherwise. Its absence is meaningful, so
            // a minCount here would force an empty-string sentinel.
            PropertyShape::on(core_terms::BLOCKED_BECAUSE.clone()).at_most_one(),
            PropertyShape::on(core_terms::ADVANCES_GOAL.clone())
                .at_most_one()
                .class(core_terms::GOAL.clone()),
            // Deliberately unclassed: the capability it points at is described
            // by NS-ONTO-001 and may not be in the same document. A class
            // constraint here would make a programme nonconformant merely for
            // being emitted separately from its manifest.
            PropertyShape::on(core_terms::ADDRESSES.clone()).at_most_one(),
            // Unclassed for the same reason as `addresses`: the milestone it
            // points at may be observed on a later run than this declaration.
            PropertyShape::on(core_terms::TRACKED_AS.clone()).at_most_one(),
            PropertyShape::on(core_terms::BLOCKED_BY.clone()).class(core_terms::MILESTONE.clone()),
            PropertyShape::on(core_terms::HAS_EVIDENCE.clone())
                .at_most_one()
                .class(core_terms::EVIDENCE.clone()),
        ],
        &mut out,
    );

    node_shape(
        "GoalShape",
        core_terms::GOAL.clone(),
        vec![
            PropertyShape::on(core_terms::NAME.clone()).exactly_one(),
            // The `why`. Mandatory, for the reason a refusal's `because` is.
            PropertyShape::on(core_terms::DESCRIPTION.clone()).exactly_one(),
        ],
        &mut out,
    );

    out
}

#[cfg(test)]
mod shape_tests {
    use super::*;
    use crate::term::{Subject, Term};

    fn sample() -> Programme {
        Programme {
            base: Iri::parse("https://ontology.nervosys.com/entity/p").unwrap(),
            goals: vec![Goal {
                id: "g",
                statement: "s",
                why: "w",
            }],
            milestones: vec![
                Milestone::observed("m1", "t", Stage::Done, "github", "2026-09-02T00:00:00Z")
                    .advancing("g")
                    .with_evidence(Evidence {
                        records: "PR #12",
                        verify_command: None,
                    }),
                Milestone::declared(
                    "m2",
                    "u",
                    Stage::Blocked {
                        because: "no reviewer",
                    },
                )
                .waiting_on(&["m1"])
                .addressing("x")
                .tracked_as("issue-9"),
            ],
        }
    }

    /// The same coverage rule NS-ONTO-001's shapes carry: a predicate the
    /// emitter writes is either constrained or explicitly exempt, never a third
    /// thing nobody noticed.
    #[test]
    fn every_emitted_predicate_is_constrained_or_structural() {
        let constrained: Vec<String> = shapes()
            .iter()
            .filter(|q| q.predicate == crate::shapes::sh("path"))
            .filter_map(|q| q.object.as_iri().map(|i| i.as_str().to_string()))
            .collect();

        // Asserted from the product node outward, so they are the product
        // shape's business rather than the milestone's; and rdf:type is what
        // the shapes target, so constraining it would be circular.
        let structural = [
            "http://www.w3.org/1999/02/22-rdf-syntax-ns#type",
            "https://ontology.nervosys.com/core/pursuesGoal",
            "https://ontology.nervosys.com/core/hasMilestone",
        ];

        for q in to_quads(&sample()) {
            let p = q.predicate.as_str();
            assert!(
                constrained.iter().any(|c| c == p) || structural.contains(&p),
                "`{p}` is emitted but neither constrained nor listed as structural"
            );
        }
    }

    #[test]
    fn no_shape_constrains_a_predicate_the_emitter_never_writes() {
        let emitted: Vec<String> = to_quads(&sample())
            .iter()
            .map(|q| q.predicate.as_str().to_string())
            .collect();
        for q in shapes() {
            if q.predicate == crate::shapes::sh("path") {
                if let Some(path) = q.object.as_iri() {
                    assert!(
                        emitted.contains(&path.as_str().to_string()),
                        "a shape constrains `{path}`, which nothing emits"
                    );
                }
            }
        }
    }

    #[test]
    fn the_emitted_graph_conforms_to_its_own_shapes() {
        let report = crate::conform::check(&shapes(), &to_quads(&sample()));
        assert!(report.conforms(), "{}", report.summary());
        assert!(
            !report.vacuous(),
            "a run that evaluated nothing is not a pass"
        );
    }

    /// The paired failure test: these shapes must be able to reject something.
    #[test]
    fn the_shapes_can_fail() {
        let mut quads = to_quads(&sample());
        // Strip provenance from every milestone -- the one thing NS-PROG-001
        // will not represent.
        quads.retain(|q| q.predicate != *core_terms::PROVENANCE);
        let report = crate::conform::check(&shapes(), &quads);
        assert!(
            !report.conforms(),
            "a milestone with no provenance must not conform"
        );
    }

    #[test]
    fn the_two_shape_graphs_compose_without_interfering() {
        // A project emits capabilities and milestones into one document, so the
        // combined shape graph must accept the combined data graph.
        let m = crate::test_support::manifest(vec![crate::Capability::read("x", "GET", "/x", "p")]);
        let mut both = crate::emit::to_quads(&m);
        both.extend(to_quads(&sample()));

        let mut all_shapes = crate::shapes::shapes();
        all_shapes.extend(shapes());

        let report = crate::conform::check(&all_shapes, &both);
        assert!(report.conforms(), "{}", report.summary());
        assert!(report.evaluated > 0);
    }

    #[test]
    fn a_milestone_subject_is_never_a_blank_node() {
        for q in to_quads(&sample()) {
            assert!(
                matches!(q.subject, Subject::Iri(_)),
                "milestones are addressable"
            );
            if let Term::Literal(l) = &q.object {
                assert!(!l.lexical.is_empty(), "no emitted literal is empty");
            }
        }
    }
}
