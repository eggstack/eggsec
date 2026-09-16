//! Explicit feature-availability input for deterministic policy evaluation.
//!
//! The extracted policy kernel must not query engine `cfg!(feature = ...)`
//! state through a hidden global. Callers construct an [`EnabledFeatures`]
//! value from the engine [`feature_registry`](https://github.com/eggstack/eggsec)
//! (or any other source, including tests) and pass it explicitly to
//! evaluation. The same operation can therefore be evaluated against
//! different supplied feature sets without recompiling this crate.
//!
//! Fail-closed: unknown names are simply absent from the set, so any
//! `required_features` entry not present in [`EnabledFeatures`] denies.

use std::collections::HashSet;

/// Immutable set of enabled feature identifiers supplied by the caller.
///
/// Constructed by the engine from its compile-time feature registry before
/// evaluation; queried by pure policy evaluation. No Cargo-feature macros
/// live in this crate.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EnabledFeatures {
    features: HashSet<String>,
}

impl EnabledFeatures {
    /// Create an empty set (no features enabled; every gated operation denies).
    pub fn empty() -> Self {
        Self {
            features: HashSet::new(),
        }
    }

    /// Build from any iterator of feature names.
    pub fn from_names<I, S>(iter: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            features: iter.into_iter().map(Into::into).collect(),
        }
    }

    /// Returns `true` when `feature` is in the enabled set.
    ///
    /// Fail-closed: unknown or absent names return `false`.
    pub fn contains(&self, feature: &str) -> bool {
        self.features.contains(feature)
    }

    /// Returns `true` when the set contains no features.
    pub fn is_empty(&self) -> bool {
        self.features.is_empty()
    }

    /// Number of enabled features in the set.
    pub fn len(&self) -> usize {
        self.features.len()
    }

    /// Iterate over enabled feature names.
    pub fn iter(&self) -> impl Iterator<Item = &str> {
        self.features.iter().map(String::as_str)
    }
}

impl<I, S> From<I> for EnabledFeatures
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    fn from(iter: I) -> Self {
        Self::from_names(iter)
    }
}

impl<S> std::iter::FromIterator<S> for EnabledFeatures
where
    S: Into<String>,
{
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = S>,
    {
        Self::from_names(iter)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_denies_everything() {
        let features = EnabledFeatures::empty();
        assert!(!features.contains("nse"));
        assert!(features.is_empty());
        assert_eq!(features.len(), 0);
    }

    #[test]
    fn from_iter_enables_listed_features_only() {
        let features = EnabledFeatures::from_names(["nse", "wireless"]);
        assert!(features.contains("nse"));
        assert!(features.contains("wireless"));
        assert!(!features.contains("db-pentest"));
        assert!(!features.contains("totally-fake-feature"));
        assert_eq!(features.len(), 2);
    }

    #[test]
    fn same_operation_evaluates_against_different_sets() {
        // Core Phase C invariant: feature availability is caller input, so the
        // same query yields different answers without recompiling this crate.
        let with_nse = EnabledFeatures::from_names(["nse"]);
        let without = EnabledFeatures::empty();
        assert!(with_nse.contains("nse"));
        assert!(!without.contains("nse"));
    }
}
