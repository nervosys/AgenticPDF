//! What must be true, stated as data so it can be checked rather than trusted.
//!
//! An invariant here corresponds to something a product actually guarantees,
//! and the useful ones usually correspond to a defect it actually shipped. An
//! ordering constraint exists because a low above a high is nonsense nobody was
//! checking. [`Invariant::NeverEquals`] exists because some states an API must
//! never reach are cheaper to state than to detect. A third enumerated member
//! like `undetermined` exists because collapsing three states into two is how a
//! check spends its whole life reporting the wrong one of the remaining two.
//!
//! Declaring them as data means one conformance run checks all of them over any
//! response, and an agent reading the model learns them without being refused
//! first.

use serde::{Deserialize, Serialize};

/// A condition that must hold over a response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Invariant {
    /// The value lies within an inclusive range.
    Range { min: f64, max: f64 },
    /// The value is at least this.
    AtLeast { min: f64 },
    /// The value is at most this.
    AtMost { max: f64 },
    /// The value is one of a closed set. Anything else is a defect, not an
    /// extension — a new state means a new invariant, deliberately.
    OneOf { allowed: Vec<String> },
    /// The value must never equal this.
    ///
    /// The refusals live here, and `because` is not optional: an agent that
    /// knows a value is forbidden but not why cannot choose a different plan.
    NeverEquals { forbidden: String, because: String },
    /// This property must be less than or equal to another on the same concept.
    NotAbove { other: String },
    /// Present only when another property has a given value.
    ///
    /// Models a system that refuses rather than nulls: a reason accompanies a
    /// determination, a governing constraint accompanies a result.
    RequiredWhen { other: String, equals: String },
    /// Every element of a list satisfies the invariant.
    EachElement { inner: Box<Invariant> },
    /// A free-text condition the schema cannot express, recorded so it is not
    /// mistaken for absent.
    ///
    /// Deliberately not checkable. The alternative — approximating it into
    /// something that is — turns an honest gap into a false pass.
    Unmodelled { statement: String },
}

impl Invariant {
    /// A short human-readable statement, for the model and for failures.
    pub fn describe(&self) -> String {
        match self {
            Invariant::Range { min, max } => format!("between {min} and {max}"),
            Invariant::AtLeast { min } => format!("at least {min}"),
            Invariant::AtMost { max } => format!("at most {max}"),
            Invariant::OneOf { allowed } => format!("one of: {}", allowed.join(", ")),
            Invariant::NeverEquals { forbidden, because } => {
                format!("never {forbidden} — {because}")
            }
            Invariant::NotAbove { other } => format!("not above {other}"),
            Invariant::RequiredWhen { other, equals } => {
                format!("present when {other} is {equals}")
            }
            Invariant::EachElement { inner } => format!("each element {}", inner.describe()),
            Invariant::Unmodelled { statement } => format!("not checked here: {statement}"),
        }
    }

    /// Whether a conformance run can evaluate this.
    ///
    /// `Unmodelled` cannot, by design. Reporting it as checked would be worse
    /// than omitting it.
    pub fn is_checkable(&self) -> bool {
        !matches!(self, Invariant::Unmodelled { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unmodelled_is_never_reported_as_checked() {
        let i = Invariant::Unmodelled {
            statement: "expert judgement".into(),
        };
        assert!(!i.is_checkable());
        assert!(i.describe().starts_with("not checked here"));
    }

    #[test]
    fn a_refusal_always_carries_its_reason() {
        let i = Invariant::NeverEquals {
            forbidden: "interdicted".into(),
            because: "terminal; checked before any tier comparison".into(),
        };
        assert!(i.describe().contains("terminal"));
    }
}
