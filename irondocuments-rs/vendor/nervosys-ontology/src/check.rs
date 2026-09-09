//! Validating a real response against what was declared.
//!
//! The model is only worth having if something enforces it. This walks a
//! response against a concept's declared properties and reports what does not
//! hold — the test nobody has to write per field.
//!
//! # What it will not do
//!
//! It does not evaluate [`Invariant::Unmodelled`], and it does not silently
//! treat an unevaluable invariant as satisfied. Those are counted as `skipped`
//! and reported, because a conformance run that cannot fail is worse than no
//! conformance run: it reads exactly like a passing one.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{Concept, Invariant, Property};

/// One thing that did not hold.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Violation {
    pub concept: String,
    pub property: String,
    /// What was expected, from [`Invariant::describe`].
    pub expected: String,
    /// What was found.
    pub found: String,
}

impl Violation {
    pub fn describe(&self) -> String {
        format!(
            "{}.{}: expected {}, found {}",
            self.concept, self.property, self.expected, self.found
        )
    }
}

/// The result of checking one response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Conformance {
    pub concept: String,
    pub violations: Vec<Violation>,
    /// Invariants actually evaluated.
    pub checked: usize,
    /// Invariants that could not be evaluated, with the reason.
    ///
    /// Surfaced rather than hidden: a run that checked nothing and a run that
    /// checked everything must not look alike.
    pub skipped: Vec<String>,
}

impl Conformance {
    pub fn conforms(&self) -> bool {
        self.violations.is_empty()
    }

    pub fn summary(&self) -> String {
        if self.violations.is_empty() {
            format!(
                "{}: conforms ({} checked, {} skipped)",
                self.concept,
                self.checked,
                self.skipped.len()
            )
        } else {
            format!(
                "{}: {} violation(s) ({} checked, {} skipped)\n  {}",
                self.concept,
                self.violations.len(),
                self.checked,
                self.skipped.len(),
                self.violations
                    .iter()
                    .map(Violation::describe)
                    .collect::<Vec<_>>()
                    .join("\n  ")
            )
        }
    }
}

/// Check a response against a declared concept.
///
/// The concepts are passed rather than looked up in a global, so this
/// vocabulary stays unbound to whichever product declared its model first.
pub fn check_value(concepts: &[Concept], concept_name: &str, response: &Value) -> Conformance {
    let mut out = Conformance {
        concept: concept_name.to_string(),
        violations: Vec::new(),
        checked: 0,
        skipped: Vec::new(),
    };

    let Some(concept) = concepts.iter().find(|c| c.name == concept_name) else {
        out.skipped
            .push(format!("concept `{concept_name}` is not declared"));
        return out;
    };

    for prop in &concept.properties {
        match response.get(prop.name) {
            None | Some(Value::Null) => {
                if !prop.optional {
                    out.violations.push(Violation {
                        concept: concept_name.into(),
                        property: prop.name.into(),
                        expected: "present".into(),
                        found: "absent".into(),
                    });
                }
                // An absent optional field has nothing to check against. That
                // is not a skip, it is the declared shape.
            }
            Some(v) => check_property(concept_name, prop, v, response, &mut out),
        }
    }
    out
}

fn check_property(
    concept: &str,
    prop: &Property,
    value: &Value,
    whole: &Value,
    out: &mut Conformance,
) {
    // A unit that declares non-negativity is a fact even with no range stated.
    if prop.unit.numeric && !prop.unit.allows_negative {
        if let Some(n) = value.as_f64() {
            out.checked += 1;
            if n < 0.0 {
                out.violations.push(Violation {
                    concept: concept.into(),
                    property: prop.name.into(),
                    expected: format!("not negative ({})", prop.unit.symbol),
                    found: n.to_string(),
                });
            }
        }
    }

    for inv in &prop.invariants {
        if !inv.is_checkable() {
            out.skipped
                .push(format!("{}.{}: {}", concept, prop.name, inv.describe()));
            continue;
        }
        check_invariant(concept, prop, inv, value, whole, out);
    }
}

fn check_invariant(
    concept: &str,
    prop: &Property,
    inv: &Invariant,
    value: &Value,
    whole: &Value,
    out: &mut Conformance,
) {
    match inv {
        Invariant::Range { min, max } => match value.as_f64() {
            Some(n) => {
                out.checked += 1;
                if n < *min || n > *max {
                    push(out, concept, prop, inv, n.to_string());
                }
            }
            None => out.skipped.push(format!(
                "{concept}.{}: not a number, cannot range-check",
                prop.name
            )),
        },
        Invariant::AtLeast { min } => match value.as_f64() {
            Some(n) => {
                out.checked += 1;
                if n < *min {
                    push(out, concept, prop, inv, n.to_string());
                }
            }
            None => out.skipped.push(format!(
                "{concept}.{}: not a number, cannot bound-check",
                prop.name
            )),
        },
        Invariant::AtMost { max } => match value.as_f64() {
            Some(n) => {
                out.checked += 1;
                if n > *max {
                    push(out, concept, prop, inv, n.to_string());
                }
            }
            None => out.skipped.push(format!(
                "{concept}.{}: not a number, cannot bound-check",
                prop.name
            )),
        },
        Invariant::OneOf { allowed } => {
            out.checked += 1;
            let s = as_plain_string(value);
            if !allowed.contains(&s) {
                push(out, concept, prop, inv, s);
            }
        }
        Invariant::NeverEquals { forbidden, .. } => {
            out.checked += 1;
            let s = as_plain_string(value);
            if s == *forbidden {
                push(out, concept, prop, inv, s);
            }
        }
        Invariant::NotAbove { other } => {
            match (
                value.as_f64(),
                whole.get(other.as_str()).and_then(Value::as_f64),
            ) {
                (Some(a), Some(b)) => {
                    out.checked += 1;
                    if a > b {
                        push(out, concept, prop, inv, format!("{a} > {other}={b}"));
                    }
                }
                _ => out.skipped.push(format!(
                    "{concept}.{}: `{other}` absent or non-numeric, cannot order-check",
                    prop.name
                )),
            }
        }
        Invariant::RequiredWhen { other, equals } => {
            let triggered = whole
                .get(other.as_str())
                .map(as_plain_string)
                .is_some_and(|s| s == *equals);
            if triggered {
                out.checked += 1;
                if matches!(value, Value::Null) {
                    push(out, concept, prop, inv, "absent".into());
                }
            }
        }
        Invariant::EachElement { inner } => match value.as_array() {
            Some(items) => {
                for item in items {
                    check_invariant(concept, prop, inner, item, whole, out);
                }
            }
            None => out.skipped.push(format!(
                "{concept}.{}: not an array, cannot check elements",
                prop.name
            )),
        },
        Invariant::Unmodelled { .. } => unreachable!("filtered by is_checkable"),
    }
}

fn push(out: &mut Conformance, concept: &str, prop: &Property, inv: &Invariant, found: String) {
    out.violations.push(Violation {
        concept: concept.into(),
        property: prop.name.into(),
        expected: inv.describe(),
        found,
    });
}

/// A JSON scalar as the bare string a declaration would name.
///
/// `json!("hardened")` and the bare word `hardened` must compare equal, so
/// strings are unquoted rather than rendered back as JSON.
fn as_plain_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Concept, Property, UnitRef};
    use serde_json::json;

    fn artifact() -> Vec<Concept> {
        vec![Concept {
            name: "artifact",
            meaning: "a catalogued AI artifact",
            found_in: "artifact",
            properties: vec![
                Property {
                    name: "tier",
                    unit: UnitRef::none(),
                    meaning: "what the registry asserts about it",
                    optional: false,
                    invariants: vec![
                        Invariant::OneOf {
                            allowed: ["closed", "quarantine", "hardened", "interdicted"]
                                .map(String::from)
                                .to_vec(),
                        },
                        Invariant::NeverEquals {
                            forbidden: "sealed".into(),
                            because: "the Sealed tier was retired".into(),
                        },
                    ],
                },
                Property {
                    name: "pulls",
                    unit: UnitRef::count(),
                    meaning: "download count",
                    optional: true,
                    invariants: vec![],
                },
            ],
        }]
    }

    #[test]
    fn a_conforming_response_passes_and_reports_how_much_it_checked() {
        let c = check_value(
            &artifact(),
            "artifact",
            &json!({"tier":"hardened","pulls":12}),
        );
        assert!(c.conforms(), "{}", c.summary());
        assert!(
            c.checked > 0,
            "a run that checks nothing must not look like a pass"
        );
    }

    #[test]
    fn a_retired_tier_is_caught_by_the_declared_refusal() {
        let c = check_value(&artifact(), "artifact", &json!({"tier":"sealed"}));
        assert!(!c.conforms());
        assert!(c.summary().contains("retired"));
    }

    #[test]
    fn a_negative_count_fails_on_the_unit_alone() {
        let c = check_value(
            &artifact(),
            "artifact",
            &json!({"tier":"closed","pulls":-3}),
        );
        assert!(
            !c.conforms(),
            "unit non-negativity holds even with no range declared"
        );
    }

    #[test]
    fn a_missing_required_field_is_a_violation_and_a_missing_optional_is_not() {
        let c = check_value(&artifact(), "artifact", &json!({"pulls":1}));
        assert_eq!(c.violations.len(), 1);
        assert_eq!(c.violations[0].property, "tier");

        let c = check_value(&artifact(), "artifact", &json!({"tier":"closed"}));
        assert!(c.conforms(), "{}", c.summary());
    }

    #[test]
    fn an_undeclared_concept_is_reported_rather_than_silently_passed() {
        let c = check_value(&artifact(), "nonesuch", &json!({}));
        assert!(c.conforms(), "no violations, because nothing was checkable");
        assert!(!c.skipped.is_empty(), "but it must say it checked nothing");
    }

    #[test]
    fn unmodelled_is_skipped_and_reported() {
        let concepts = vec![Concept {
            name: "c",
            meaning: "",
            found_in: "c",
            properties: vec![Property {
                name: "f",
                unit: UnitRef::text(),
                meaning: "",
                optional: false,
                invariants: vec![Invariant::Unmodelled {
                    statement: "expert judgement".into(),
                }],
            }],
        }];
        let c = check_value(&concepts, "c", &json!({"f":"x"}));
        assert!(c.conforms());
        assert_eq!(c.skipped.len(), 1);
    }
}
