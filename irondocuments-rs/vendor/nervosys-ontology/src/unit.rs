//! The seam that keeps this crate free of any one product's domain.
//!
//! One product measures in structural loads and water volumes; another in
//! bytes, tokens and log indices. Neither belongs here. A product declares its
//! own unit enum, implements [`UnitKind`], and this crate stores the flattened
//! [`UnitRef`] — the three facts conformance checking actually needs.
//!
//! The examples are deliberately unattributed. This crate is vendored into
//! products with different confidentiality settings, so a doc comment naming a
//! sibling product travels into a repository that may become public while the
//! product it names is not meant to. The principle does not need a proper noun.

use serde::{Deserialize, Serialize};

/// A product's unit type.
///
/// Implement this on a domain enum and convert with [`UnitRef::of`].
pub trait UnitKind: Copy {
    /// The symbol as it should appear to a reader — `"kip-ft"`, `"tokens/s"`.
    fn as_str(self) -> &'static str;
    /// Whether values carrying this unit are numbers.
    ///
    /// Range and ordering invariants are only meaningful when this is true;
    /// [`crate::check`] skips them rather than failing when it is not.
    fn is_numeric(self) -> bool;
    /// Whether a negative value is legitimate.
    ///
    /// Distinct from a declared range: a count is never negative even when no
    /// range was declared, and reporting that as "no invariant" would lose a
    /// fact the product knows.
    fn allows_negative(self) -> bool;
}

/// A unit, flattened to what a checker and an agent need.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnitRef {
    pub symbol: &'static str,
    pub numeric: bool,
    pub allows_negative: bool,
}

impl UnitRef {
    /// Flatten a product's unit.
    pub fn of<U: UnitKind>(u: U) -> Self {
        Self {
            symbol: u.as_str(),
            numeric: u.is_numeric(),
            allows_negative: u.allows_negative(),
        }
    }

    /// A dimensionless value — an identifier, a hash, an enumerated state.
    pub const fn none() -> Self {
        Self {
            symbol: "",
            numeric: false,
            allows_negative: false,
        }
    }

    /// A non-negative count.
    pub const fn count() -> Self {
        Self {
            symbol: "count",
            numeric: true,
            allows_negative: false,
        }
    }

    /// Free text.
    pub const fn text() -> Self {
        Self {
            symbol: "text",
            numeric: false,
            allows_negative: false,
        }
    }

    /// A boolean.
    pub const fn boolean() -> Self {
        Self {
            symbol: "bool",
            numeric: false,
            allows_negative: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Copy)]
    enum Demo {
        Bytes,
    }
    impl UnitKind for Demo {
        fn as_str(self) -> &'static str {
            "bytes"
        }
        fn is_numeric(self) -> bool {
            true
        }
        fn allows_negative(self) -> bool {
            false
        }
    }

    #[test]
    fn a_product_unit_flattens_without_this_crate_knowing_it() {
        let u = UnitRef::of(Demo::Bytes);
        assert_eq!(u.symbol, "bytes");
        assert!(u.numeric && !u.allows_negative);
    }

    #[test]
    fn the_builtin_refs_are_consistent() {
        assert!(!UnitRef::none().numeric);
        assert!(UnitRef::count().numeric && !UnitRef::count().allows_negative);
    }
}
