use crate::operation_registry::{OperationExecutorRegistry, StableOperation};

/// Generate the operation list for Engine.list_operations().
/// Derived from the registry, not a separate list.
pub fn operation_id_list() -> Vec<&'static str> {
    StableOperation::ALL.iter().map(|op| op.id()).collect()
}

/// Generate feature requirements map.
/// Returns Vec of (operation_id, feature_name) tuples.
pub fn feature_requirements_map() -> Vec<(&'static str, &'static str)> {
    StableOperation::ALL
        .iter()
        .filter_map(|op| op.feature_required().map(|f| (op.id(), f)))
        .collect()
}

/// Generate risk classification map.
pub fn risk_classification_map() -> Vec<(&'static str, &'static str)> {
    let registry = OperationExecutorRegistry::default_stable();
    StableOperation::ALL
        .iter()
        .map(|op| {
            let desc = registry.descriptor_for(*op);
            (
                op.id(),
                match desc.risk {
                    eggsec::config::OperationRisk::SafeActive => "safe_active",
                    eggsec::config::OperationRisk::Intrusive => "intrusive",
                    eggsec::config::OperationRisk::DbPentest => "db_pentest",
                    eggsec::config::OperationRisk::LoadTest => "load_test",
                    _ => "unknown",
                },
            )
        })
        .collect()
}

/// Generate confirmation requirements map.
pub fn confirmation_requirements_map() -> Vec<(&'static str, bool)> {
    let registry = OperationExecutorRegistry::default_stable();
    StableOperation::ALL
        .iter()
        .map(|op| {
            let desc = registry.descriptor_for(*op);
            (op.id(), desc.confirmation_required)
        })
        .collect()
}

/// Generate daemon task kind mapping.
pub fn daemon_task_kind_map() -> Vec<(&'static str, &'static str)> {
    let registry = OperationExecutorRegistry::default_stable();
    StableOperation::ALL
        .iter()
        .map(|op| {
            let desc = registry.descriptor_for(*op);
            (op.id(), desc.daemon_task_kind)
        })
        .collect()
}

/// Generate aliases map for all operations.
pub fn aliases_map() -> Vec<(&'static str, &'static [&'static str])> {
    let registry = OperationExecutorRegistry::default_stable();
    StableOperation::ALL
        .iter()
        .map(|op| {
            let desc = registry.descriptor_for(*op);
            (op.id(), desc.aliases)
        })
        .collect()
}

/// Validate that all operations have consistent metadata.
/// Returns a list of inconsistencies (empty if all consistent).
pub fn validate_metadata_consistency() -> Vec<String> {
    let mut issues = Vec::new();
    let registry = OperationExecutorRegistry::default_stable();

    for &op in StableOperation::ALL {
        let desc = registry.descriptor_for(op);

        // Check: every operation must have a description
        if desc.description.is_empty() {
            issues.push(format!("{:?}: missing description", op));
        }

        // Check: all operations must be locally available
        if !desc.local_available {
            issues.push(format!("{:?}: not locally available", op));
        }

        // Check: sync and async must both be available
        if !desc.sync_available || !desc.async_available {
            issues.push(format!(
                "{:?}: sync={} async={}",
                op, desc.sync_available, desc.async_available
            ));
        }
    }

    issues
}

/// Generate a versioned metadata manifest for CI validation.
/// Includes a version string and commit identity placeholder.
pub fn metadata_manifest() -> serde_json::Value {
    serde_json::json!({
        "version": "1.0",
        "operation_count": StableOperation::ALL.len(),
        "operations": operation_id_list(),
        "feature_requirements": feature_requirements_map(),
        "risk_classifications": risk_classification_map(),
        "confirmation_requirements": confirmation_requirements_map(),
        "daemon_task_kinds": daemon_task_kind_map(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata_consistency() {
        let issues = validate_metadata_consistency();
        assert!(issues.is_empty(), "Metadata inconsistencies: {:?}", issues);
    }

    #[test]
    fn test_manifest_generation() {
        let manifest = metadata_manifest();
        assert_eq!(manifest["operation_count"], 22);
        assert!(manifest["operations"].as_array().unwrap().len() == 22);
    }

    #[test]
    fn test_feature_map_covers_all_gated_ops() {
        let map = feature_requirements_map();
        // 8 feature-gated operations (4 features × 2 ops each for git-secrets, sbom, mobile, container;
        // plus db-pentest and nse which are 1 each) = 8 total
        assert_eq!(map.len(), 8);
        // Verify specific entries
        let git_secrets = map.iter().find(|(id, _)| *id == "scan_git_secrets");
        assert!(git_secrets.is_some());
        assert_eq!(git_secrets.unwrap().1, "git-secrets");
    }

    #[test]
    fn test_risk_classification_map_has_all_ops() {
        let map = risk_classification_map();
        assert_eq!(map.len(), 22);
        for op in StableOperation::ALL {
            assert!(
                map.iter().any(|(id, _)| *id == op.id()),
                "missing risk for {:?}",
                op
            );
        }
    }

    #[test]
    fn test_confirmation_map_covers_intrusive_ops() {
        let map = confirmation_requirements_map();
        let mut confirmed: Vec<&str> = map
            .iter()
            .filter(|(_, required)| *required)
            .map(|(id, _)| *id)
            .collect();
        confirmed.sort();
        assert_eq!(
            confirmed,
            vec!["db_probe", "fuzz_http", "load_test", "nse_run"]
        );
    }

    #[test]
    fn test_daemon_task_kind_map_has_all_ops() {
        let map = daemon_task_kind_map();
        assert_eq!(map.len(), 22);
        for (id, kind) in &map {
            assert!(!kind.is_empty(), "empty daemon_task_kind for {}", id);
        }
    }

    #[test]
    fn test_aliases_map_has_all_ops() {
        let map = aliases_map();
        assert_eq!(map.len(), 22);
        // FingerprintServices should have "fingerprint" alias
        let fp = map.iter().find(|(id, _)| *id == "fingerprint_services");
        assert!(fp.is_some());
        assert!(fp.unwrap().1.contains(&"fingerprint"));
    }
}
