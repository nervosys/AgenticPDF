//! What a system can be asked to do, what a caller must hold to ask, and how
//! it declines.
//!
//! # CAN is not MAY
//!
//! EXAMPLE-ONTO-000 §6 states the rule this module is built around: capability and
//! authority are orthogonal. A system may possess a capability it is not
//! authorised to exercise, and the company ontology models them as separate
//! things for that reason.
//!
//! An earlier version of this crate collapsed them into
//! `requires_scope: Option<&str>`, which was wrong in a way that produced real
//! error. `None` rendered as "public", so an endpoint closed by deployment
//! configuration — holding no scope, and also not callable — printed as
//! `[public]` directly above the refusal explaining it was closed. Two
//! different facts had been given one field.
//!
//! [`Authority`] is that field split apart. Its variants are not invented:
//! each is a mechanism found while auditing a real service by calling every
//! mutating route without a credential and recording the outcome. The audit
//! also produced the variant that matters most, [`Authority::Unaudited`] —
//! because the failure there was a classifier that defaulted to "open" when it
//! could not tell, which is the one answer a reader must never be given
//! silently.

use serde::{Deserialize, Serialize};

use crate::{Concept, Surface};

/// What a caller must hold in order to invoke a capability.
///
/// Distinct from what the capability *does* ([`Effect`]) and from whether it
/// is safe to hand to an agent (`agent_safe`). Those three are independent,
/// and every attempt to derive one from another has been wrong.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Authority {
    /// Anyone may invoke this, by design and on purpose.
    ///
    /// Reserve for capabilities that are *meant* to be open. Never the answer
    /// to "we did not check" — that is [`Authority::Unaudited`].
    Public,

    /// A named scope in the product's own vocabulary, carried by a token.
    ///
    /// A struct variant rather than a newtype: serde cannot internally-tag a
    /// newtype variant holding a primitive, and the tagged form is what makes
    /// this enum legible in the emitted JSON.
    Scope { name: &'static str },

    /// A second factor proven in the request itself.
    ///
    /// A capability can require no scope and still be firmly closed: an
    /// operator-token endpoint verifying a TOTP holds no bearer scope and is
    /// not remotely public.
    SecondFactor { method: &'static str },

    /// The payload's own signature is the gate.
    ///
    /// Common where the credential is the artefact: a revocation is accepted
    /// because it verifies against a trusted signer, not because the caller
    /// authenticated.
    PayloadSignature { signer: &'static str },

    /// A shared secret proves the caller, typically a webhook.
    SharedSecret { source: &'static str },

    /// Reachable only when a deployment flag opens it.
    ///
    /// The case that broke the old model: no scope, and yet closed.
    ClosedByDeployment {
        flag: &'static str,
        open_by_default: bool,
    },

    /// Nobody has established what protects this.
    ///
    /// Serialises as `unaudited` and is never rendered as "public". An agent
    /// must be able to tell "anyone may" from "we do not know", and a checker
    /// must be able to fail on the second.
    Unaudited { note: &'static str },
}

impl Authority {
    /// Whether this is open to an unauthenticated caller.
    ///
    /// `None` for [`Authority::Unaudited`] — the honest answer is not a
    /// boolean, and forcing one is how "unknown" becomes "open".
    pub fn is_open(&self) -> Option<bool> {
        match self {
            Authority::Public => Some(true),
            Authority::ClosedByDeployment {
                open_by_default, ..
            } => Some(*open_by_default),
            Authority::Scope { .. }
            | Authority::SecondFactor { .. }
            | Authority::PayloadSignature { .. }
            | Authority::SharedSecret { .. } => Some(false),
            Authority::Unaudited { .. } => None,
        }
    }

    /// The scope name, when authority is scope-based.
    pub fn scope(&self) -> Option<&'static str> {
        match self {
            Authority::Scope { name } => Some(name),
            _ => None,
        }
    }

    /// A short human label. Never "public" unless it truly is.
    pub fn label(&self) -> String {
        match self {
            Authority::Public => "public".into(),
            Authority::Scope { name } => format!("scope:{name}"),
            Authority::SecondFactor { method } => format!("second-factor:{method}"),
            Authority::PayloadSignature { signer } => format!("signed-by:{signer}"),
            Authority::SharedSecret { source } => format!("shared-secret:{source}"),
            Authority::ClosedByDeployment {
                flag,
                open_by_default,
            } => {
                if *open_by_default {
                    format!("open unless {flag} disables it")
                } else {
                    format!("closed unless {flag} enables it")
                }
            }
            Authority::Unaudited { .. } => "UNAUDITED".into(),
        }
    }
}

/// A declared way an operation says no.
///
/// An agent that learns a refusal by *being refused* has already spent a call,
/// and on a mutating operation possibly one it cannot take back. That is the
/// difference between "call and see" and "plan and execute", and it is why
/// `because` is not optional.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Refusal {
    pub when: &'static str,
    pub response: &'static str,
    pub because: &'static str,
}

/// Whether an operation changes state.
///
/// Stated, never inferred. An HTTP method is a convention, not a guarantee.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Effect {
    ReadOnly,
    Mutating,
}

impl Effect {
    pub fn mutates(self) -> bool {
        matches!(self, Effect::Mutating)
    }
}

/// A field that legitimately may not appear, and what its absence means.
///
/// Distinguishes "we looked and there is none" from "we could not look".
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Absence {
    pub concept: &'static str,
    pub when: &'static str,
}

/// How a caller can check a claim without trusting the answer.
///
/// EXAMPLE-ONTO-000 §7 makes evidence and provenance first-class. For a system
/// whose product *is* provenance, the useful form is not "here is a receipt"
/// but "here is the command that re-derives it independently".
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Evidence {
    /// What is recorded — `"transparency-log index"`, `"signed checkpoint"`.
    pub records: &'static str,
    /// The command that verifies it offline, if one exists.
    pub verify_command: Option<&'static str>,
}

/// One place a capability can be invoked, on a surface other than the
/// product's primary one.
///
/// A capability has a single meaning and may have several front doors: the
/// same operation reachable from a command line and from an MCP tool is one
/// capability with two invocations, not two capabilities. Declaring them
/// separately is how the drift this vocabulary exists to catch gets in — a
/// capability added to one door and forgotten at another is invisible to
/// exactly the agents that reach the product through the forgotten one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Invocation {
    pub surface: crate::surface::SurfaceKind,
    /// The verb slot for the surface: an HTTP method, a CLI subcommand's
    /// parent, or the empty string where the surface has no such notion.
    pub method: &'static str,
    /// The address on that surface: a path, a subcommand, a tool name.
    pub path: &'static str,
}

impl Invocation {
    pub fn new(
        surface: crate::surface::SurfaceKind,
        method: &'static str,
        path: &'static str,
    ) -> Self {
        Self {
            surface,
            method,
            path,
        }
    }

    /// A CLI subcommand.
    pub fn cli(subcommand: &'static str) -> Self {
        Self::new(crate::surface::SurfaceKind::Cli, "", subcommand)
    }

    /// A machine interface with no human view — an MCP tool, a webhook.
    pub fn machine(name: &'static str) -> Self {
        Self::new(crate::surface::SurfaceKind::Machine, "", name)
    }
}

/// Something the system can be asked to do.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Capability {
    pub name: &'static str,
    pub method: &'static str,
    pub path: &'static str,
    pub purpose: &'static str,
    pub inputs: Vec<&'static str>,
    pub returns: Vec<&'static str>,
    pub may_be_absent: Vec<Absence>,

    /// What it does to state.
    pub effect: Effect,
    /// What a caller must hold. Orthogonal to `effect`.
    pub authority: Authority,
    /// Whether it is safe to expose to an autonomous agent.
    ///
    /// Independent of both fields above: some read-only operations are too
    /// expensive to run unattended, and some mutations are perfectly safe.
    pub agent_safe: bool,
    /// Whether the effect can be undone. Emitted as `agent:reversible`.
    pub reversible: bool,

    pub refusals: Vec<Refusal>,
    pub evidence: Option<Evidence>,

    /// Which surface `method`/`path` names.
    ///
    /// Defaults to [`SurfaceKind::Web`], so a product with an HTTP router
    /// declares what it always declared. A product that has no web surface at
    /// all — a command-line tool, an MCP server — names the one it does have
    /// rather than inventing a route to satisfy the shape of this field.
    #[serde(default)]
    pub primary_surface: crate::surface::SurfaceKind,

    /// Other surfaces this same capability is reachable on.
    ///
    /// Empty for a product with one front door, which is why adding this field
    /// left every existing declaration conformant. `method`/`path` remain the
    /// primary invocation; these are the rest.
    ///
    /// Not projected into RDF. The company context declares no term for
    /// "reachable on", and minting one here would detach this crate from the
    /// graph it emits into — silently, since the symptom is an empty query
    /// rather than an error (§5).
    #[serde(default)]
    pub invocations: Vec<Invocation>,
}

impl Capability {
    /// A read-only, public capability — the common shape, so declarations do
    /// not bury the interesting fields in boilerplate.
    pub fn read(
        name: &'static str,
        method: &'static str,
        path: &'static str,
        purpose: &'static str,
    ) -> Self {
        Self {
            name,
            method,
            path,
            purpose,
            inputs: Vec::new(),
            returns: Vec::new(),
            may_be_absent: Vec::new(),
            effect: Effect::ReadOnly,
            authority: Authority::Public,
            agent_safe: true,
            reversible: true,
            refusals: Vec::new(),
            evidence: None,
            primary_surface: crate::surface::SurfaceKind::Web,
            invocations: Vec::new(),
        }
    }

    /// A read-only capability whose primary door is a command-line
    /// subcommand.
    pub fn cli_read(name: &'static str, subcommand: &'static str, purpose: &'static str) -> Self {
        let mut c = Self::read(name, "", subcommand, purpose);
        c.primary_surface = crate::surface::SurfaceKind::Cli;
        c
    }

    /// Name the surface `method`/`path` refers to.
    pub fn on_surface(mut self, surface: crate::surface::SurfaceKind) -> Self {
        self.primary_surface = surface;
        self
    }

    pub fn taking(mut self, inputs: &[&'static str]) -> Self {
        self.inputs = inputs.to_vec();
        self
    }

    pub fn returning(mut self, concepts: &[&'static str]) -> Self {
        self.returns = concepts.to_vec();
        self
    }

    pub fn refusing(mut self, r: Refusal) -> Self {
        self.refusals.push(r);
        self
    }

    pub fn absent(mut self, a: Absence) -> Self {
        self.may_be_absent.push(a);
        self
    }

    pub fn with_evidence(mut self, e: Evidence) -> Self {
        self.evidence = Some(e);
        self
    }

    /// Every place this capability can be invoked: the primary one, then any
    /// further surfaces.
    ///
    /// One list rather than two so that a caller checking reachability cannot
    /// check the primary door and forget the rest — which is the mistake the
    /// field was added to prevent.
    pub fn all_invocations(
        &self,
    ) -> Vec<(crate::surface::SurfaceKind, &'static str, &'static str)> {
        let mut all = vec![(self.primary_surface, self.method, self.path)];
        all.extend(
            self.invocations
                .iter()
                .map(|i| (i.surface, i.method, i.path)),
        );
        all
    }

    /// Declare another surface this capability is reachable on.
    pub fn reachable(mut self, invocation: Invocation) -> Self {
        self.invocations.push(invocation);
        self
    }

    pub fn requiring(mut self, authority: Authority) -> Self {
        self.authority = authority;
        self
    }

    /// Declare this as state-changing, with the authority required and whether
    /// it can be undone.
    ///
    /// Takes all three together on purpose: a mutation declared without saying
    /// what guards it and whether it is recoverable is the shape of
    /// declaration that has been wrong every time.
    pub fn mutating(mut self, authority: Authority, reversible: bool) -> Self {
        self.effect = Effect::Mutating;
        self.authority = authority;
        self.reversible = reversible;
        // A mutation is not agent-safe until someone says so explicitly.
        self.agent_safe = false;
        self
    }

    /// Mark as safe for autonomous invocation. Verbose on purpose.
    pub fn agent_safe_because(mut self, _reason: &'static str) -> Self {
        self.agent_safe = true;
        self
    }

    /// A coarse risk score for `agent:riskScore`, derived rather than invented.
    ///
    /// Meaningful only relative to other capabilities in the same manifest. It
    /// is not a probability. Unaudited scores *higher* than known-open,
    /// because not knowing is worse than knowing the answer is bad.
    pub fn risk_score(&self) -> f64 {
        let mut r: f64 = 0.0;
        if self.effect.mutates() {
            r += 0.4;
        }
        if !self.reversible {
            r += 0.3;
        }
        match self.authority.is_open() {
            Some(true) => r += 0.2,
            None => r += 0.3,
            Some(false) => {}
        }
        if self.evidence.is_none() {
            r += 0.1;
        }
        r.min(1.0)
    }
}

/// A status a system returns when declining, and what it means.
///
/// Collected separately from individual refusals so retry logic is written
/// once rather than per operation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RefusalStatus {
    pub code: u16,
    pub meaning: &'static str,
    /// Whether retrying the identical request could succeed.
    pub retryable: bool,
}

/// A surface that exists and is deliberately not modelled.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Unmapped {
    pub area: &'static str,
    pub why: &'static str,
}

/// A product's complete self-description.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Manifest {
    pub schema: u32,
    pub product: &'static str,
    /// The IRI this product's entities live under, joining it to the company
    /// graph. Emitted subjects are minted beneath it.
    pub base: crate::iri::Iri,
    pub purpose: &'static str,
    pub principles: Vec<&'static str>,
    pub capabilities: Vec<Capability>,
    pub concepts: Vec<Concept>,
    pub surfaces: Vec<Surface>,
    pub refusal_statuses: Vec<RefusalStatus>,
    /// Required, not optional. A model that cannot say what it omits invites
    /// the reader to assume it is complete.
    pub unmapped: Vec<Unmapped>,
}

impl Manifest {
    pub fn capability(&self, name: &str) -> Option<&Capability> {
        self.capabilities.iter().find(|c| c.name == name)
    }

    pub fn declares(&self, method: &str, path: &str) -> bool {
        self.capabilities
            .iter()
            .any(|c| c.method.eq_ignore_ascii_case(method) && c.path == path)
    }

    pub fn agent_tools(&self) -> impl Iterator<Item = &Capability> {
        self.capabilities.iter().filter(|c| c.agent_safe)
    }

    /// Every scope referenced, deduplicated.
    pub fn scopes(&self) -> Vec<&'static str> {
        let mut s: Vec<_> = self
            .capabilities
            .iter()
            .filter_map(|c| c.authority.scope())
            .collect();
        s.sort_unstable();
        s.dedup();
        s
    }

    /// Capabilities whose protection nobody has established.
    ///
    /// The list a reviewer should see first, and one a release check can
    /// assert is empty.
    pub fn unaudited(&self) -> Vec<&Capability> {
        self.capabilities
            .iter()
            .filter(|c| matches!(c.authority, Authority::Unaudited { .. }))
            .collect()
    }

    /// Mutating capabilities reachable without a credential.
    pub fn open_mutations(&self) -> Vec<&Capability> {
        self.capabilities
            .iter()
            .filter(|c| c.effect.mutates() && c.authority.is_open() == Some(true))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_authority_never_reads_as_public() {
        let u = Authority::Unaudited {
            note: "not yet probed",
        };
        assert_eq!(u.is_open(), None, "unknown must not collapse to a boolean");
        assert_eq!(u.label(), "UNAUDITED");
        assert_ne!(u.label(), Authority::Public.label());
    }

    #[test]
    fn a_capability_with_no_scope_can_still_be_closed() {
        // The exact case the old `requires_scope: Option<&str>` got wrong: it
        // printed "[public]" above a refusal saying the endpoint was closed.
        let a = Authority::ClosedByDeployment {
            flag: "SF_ALLOW_ANONYMOUS_SUBMIT",
            open_by_default: false,
        };
        assert_eq!(a.scope(), None);
        assert_eq!(a.is_open(), Some(false));
        assert!(a.label().contains("closed unless"));
    }

    #[test]
    fn second_factor_holds_no_scope_and_is_not_open() {
        let a = Authority::SecondFactor { method: "TOTP" };
        assert_eq!(a.scope(), None);
        assert_eq!(
            a.is_open(),
            Some(false),
            "no scope is not the same as no protection"
        );
    }

    #[test]
    fn declaring_a_mutation_makes_it_agent_unsafe_by_default() {
        let c = Capability::read("x", "POST", "/x", "p")
            .mutating(Authority::Scope { name: "admin" }, false);
        assert!(
            !c.agent_safe,
            "opting an agent in must be explicit, never inherited"
        );
        assert!(c.effect.mutates());
        assert!(!c.reversible);
    }

    #[test]
    fn risk_rises_with_irreversibility_and_falls_with_evidence() {
        let base = Capability::read("r", "GET", "/r", "p");
        let bad = Capability::read("w", "POST", "/w", "p").mutating(Authority::Public, false);
        let better = Capability::read("w", "POST", "/w", "p")
            .mutating(Authority::Scope { name: "admin" }, false)
            .with_evidence(Evidence {
                records: "log index",
                verify_command: Some("sf verify"),
            });
        assert!(base.risk_score() < better.risk_score());
        assert!(better.risk_score() < bad.risk_score());
        assert!(bad.risk_score() <= 1.0);
    }

    #[test]
    fn unaudited_outranks_known_open_in_risk() {
        let open = Capability::read("a", "POST", "/a", "p").mutating(Authority::Public, true);
        let unknown = Capability::read("b", "POST", "/b", "p")
            .mutating(Authority::Unaudited { note: "not probed" }, true);
        assert!(
            unknown.risk_score() > open.risk_score(),
            "not knowing must rank worse than knowing the answer is bad"
        );
    }
}
