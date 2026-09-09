//! The things a product handles, and the properties that describe them.
//!
//! A concept is whatever the product would name in a sentence about its own
//! work — a building, an artifact, an attestation, a draw request. Declaring it
//! gives an agent the vocabulary before it sees a single response, and gives
//! [`crate::check`] something to validate against.

use serde::{Deserialize, Serialize};

use crate::{Invariant, UnitRef};

/// One field of a concept.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Property {
    /// The field name as it appears on the wire.
    pub name: &'static str,
    pub unit: UnitRef,
    /// What it means, in a sentence an agent can use.
    pub meaning: &'static str,
    /// Whether the field may be absent.
    ///
    /// Absence is meaningful in both products this vocabulary serves: a missing
    /// ratio means unverified, not zero, and a missing parameter count means
    /// nobody could reach the weights — not a model with no parameters.
    pub optional: bool,
    pub invariants: Vec<Invariant>,
}

/// Something the product handles.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Concept {
    pub name: &'static str,
    /// What it is, in a sentence.
    pub meaning: &'static str,
    /// Which response object it appears in, so a checker knows where to look.
    /// Empty means it is nested inside another concept.
    pub found_in: &'static str,
    pub properties: Vec<Property>,
}

impl Concept {
    /// The property by wire name, if declared.
    pub fn property(&self, name: &str) -> Option<&Property> {
        self.properties.iter().find(|p| p.name == name)
    }
}
