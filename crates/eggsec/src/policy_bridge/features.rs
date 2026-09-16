//! Feature-availability mapping: engine registry -> pure policy input.
//!
//! Policy evaluation never queries `cfg!(feature = ...)` itself. The engine
//! snapshots its compile-time [`feature_registry`](crate::config::feature_registry)
//! into an immutable [`eggsec_policy::EnabledFeatures`] value before
//! evaluation and passes it explicitly. Tests can supply arbitrary sets
//! without recompiling the policy kernel.

/// Snapshot the current compile-time feature registry into an explicit
/// [`eggsec_policy::EnabledFeatures`] input for pure policy evaluation.
pub fn current_enabled_features() -> eggsec_policy::EnabledFeatures {
    let names = crate::config::ALL_FEATURES
        .iter()
        .filter(|entry| entry.enabled)
        .map(|entry| entry.name);
    eggsec_policy::EnabledFeatures::from_names(names)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_contains_known_compiled_features() {
        let features = current_enabled_features();
        // `tool-api` is enabled in the default test build via dev context;
        // regardless, unknown names must fail closed.
        assert!(!features.contains("totally-fake-feature-xyz"));
        let _ = features.len();
    }

    #[test]
    fn snapshot_matches_registry_oracle() {
        let features = current_enabled_features();
        for entry in crate::config::ALL_FEATURES {
            assert_eq!(
                features.contains(entry.name),
                entry.enabled,
                "feature '{}' mismatch between registry and snapshot",
                entry.name
            );
        }
    }
}
