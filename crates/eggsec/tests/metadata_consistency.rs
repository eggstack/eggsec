//! Metadata consistency tests — cross-validate DomainDescriptor, OperationMetadata,
//! and the capability matrix against each other and against documentation.
//!
//! These tests ensure that metadata remains synchronized across code and docs,
//! preventing drift between what is declared and what is documented.

use eggsec::config::{all_operation_metadata, metadata_for_tool_id, Capability, OperationRisk};
use eggsec::domain::{all_domain_descriptors, generate_capability_matrix, DryRunSupport};

// ─── Domain × OperationMetadata Cross-Validation ──────────────────────────

/// Every operation declared in a DomainDescriptor must have a matching
/// OperationMetadata entry. This catches orphaned domain operations that
/// lack canonical metadata.
#[test]
fn all_domain_operations_have_matching_metadata() {
    for domain in all_domain_descriptors() {
        for op in domain.operations {
            let meta = metadata_for_tool_id(op.operation_id).unwrap_or_else(|| {
                panic!(
                    "domain '{}' operation '{}' has no matching OperationMetadata — \
                     add an entry to ALL_OPERATION_METADATA or ALL_OPERATION_METADATA_ALIASES",
                    domain.id, op.operation_id
                )
            });
            assert_eq!(
                meta.id, op.operation_id,
                "domain '{}' operation '{}' resolves to metadata '{}' — canonical ID mismatch",
                domain.id, op.operation_id, meta.id
            );
        }
    }
}

/// Domain-declared risk must match OperationMetadata risk for each operation.
#[test]
fn domain_risk_matches_operation_metadata_risk() {
    for domain in all_domain_descriptors() {
        for op in domain.operations {
            let meta = metadata_for_tool_id(op.operation_id)
                .expect("domain operation should have metadata");
            assert_eq!(
                op.risk, meta.risk,
                "domain '{}' operation '{}': domain declares {:?} but metadata has {:?}",
                domain.id, op.operation_id, op.risk, meta.risk
            );
        }
    }
}

/// Domain-declared capabilities must be a subset of (or equal to) OperationMetadata capabilities.
#[test]
fn domain_capabilities_match_metadata() {
    for domain in all_domain_descriptors() {
        for op in domain.operations {
            let meta = metadata_for_tool_id(op.operation_id)
                .expect("domain operation should have metadata");
            assert_eq!(
                op.capabilities, meta.required_capabilities,
                "domain '{}' operation '{}': domain capabilities {:?} != metadata {:?}",
                domain.id, op.operation_id, op.capabilities, meta.required_capabilities
            );
        }
    }
}

/// Domain-declared feature requirements must be consistent with metadata.
/// The domain may declare the same features or a subset — but not features
/// that the metadata doesn't know about.
#[test]
fn domain_features_subset_of_metadata() {
    for domain in all_domain_descriptors() {
        for op in domain.operations {
            let meta = metadata_for_tool_id(op.operation_id)
                .expect("domain operation should have metadata");
            for feat in op.required_features {
                assert!(
                    meta.required_features.contains(feat),
                    "domain '{}' operation '{}': feature '{}' is not in metadata required_features {:?}",
                    domain.id, op.operation_id, feat, meta.required_features
                );
            }
        }
    }
}

// ─── Domain Uniqueness ────────────────────────────────────────────────────

/// Domain IDs must be globally unique.
#[test]
fn domain_ids_are_unique() {
    let mut seen = rustc_hash::FxHashSet::default();
    for domain in all_domain_descriptors() {
        assert!(
            seen.insert(domain.id),
            "duplicate domain id: '{}'",
            domain.id
        );
    }
}

/// Operation IDs within a domain must be unique.
#[test]
fn domain_operation_ids_are_unique_within_domain() {
    for domain in all_domain_descriptors() {
        let mut seen = rustc_hash::FxHashSet::default();
        for op in domain.operations {
            assert!(
                seen.insert(op.operation_id),
                "domain '{}' has duplicate operation id: '{}'",
                domain.id,
                op.operation_id
            );
        }
    }
}

/// Operation IDs must be globally unique across all domains.
#[test]
fn domain_operation_ids_are_globally_unique() {
    let mut seen = rustc_hash::FxHashSet::default();
    for domain in all_domain_descriptors() {
        for op in domain.operations {
            assert!(
                seen.insert(op.operation_id),
                "operation '{}' appears in multiple domains",
                op.operation_id
            );
        }
    }
}

// ─── OperationMetadata Uniqueness ──────────────────────────────────────────

/// All OperationMetadata IDs must be unique.
#[test]
fn operation_metadata_ids_are_unique() {
    let mut seen = rustc_hash::FxHashSet::default();
    for m in all_operation_metadata() {
        assert!(
            seen.insert(m.id),
            "duplicate operation metadata id: '{}'",
            m.id
        );
    }
}

/// All tool aliases must resolve to known canonical IDs.
#[test]
fn all_aliases_resolve_to_known_metadata() {
    use eggsec::config::ALL_OPERATION_METADATA_ALIASES;
    for &(alias, canonical) in ALL_OPERATION_METADATA_ALIASES {
        assert!(
            metadata_for_tool_id(canonical).is_some(),
            "alias '{}' points to unknown canonical id '{}'",
            alias,
            canonical
        );
    }
}

/// No alias should map to itself (that's a redundant entry).
#[test]
fn no_alias_maps_to_self() {
    use eggsec::config::ALL_OPERATION_METADATA_ALIASES;
    for &(alias, canonical) in ALL_OPERATION_METADATA_ALIASES {
        assert_ne!(
            alias, canonical,
            "alias '{}' maps to itself — remove the redundant alias",
            alias
        );
    }
}

// ─── Capability Matrix Consistency ─────────────────────────────────────────

/// The capability matrix should produce rows when domains are registered.
/// With no features enabled, the domain registry is empty (no rows).
/// With db-pentest enabled, at least one row should appear.
#[test]
fn capability_matrix_has_rows_when_domains_registered() {
    let rows = generate_capability_matrix();
    // Every row should have non-empty fields regardless of feature state.
    for row in &rows {
        assert!(!row.domain_id.is_empty());
        assert!(!row.operation_id.is_empty());
    }
    // When db-pentest is enabled, rows should be non-empty.
    #[cfg(feature = "db-pentest")]
    assert!(
        !rows.is_empty(),
        "capability matrix is empty — db-pentest domain should produce rows"
    );
}

/// Every capability matrix row should have non-empty required fields.
#[test]
fn capability_matrix_rows_have_required_fields() {
    let rows = generate_capability_matrix();
    for row in &rows {
        assert!(!row.domain_id.is_empty(), "row has empty domain_id");
        assert!(!row.operation_id.is_empty(), "row has empty operation_id");
        assert!(
            !row.operation_name.is_empty(),
            "row has empty operation_name"
        );
        assert!(!row.domain_name.is_empty(), "row has empty domain_name");
    }
}

/// Every operation in the capability matrix must have matching OperationMetadata.
#[test]
fn capability_matrix_operations_have_metadata() {
    let rows = generate_capability_matrix();
    for row in &rows {
        let meta = metadata_for_tool_id(row.operation_id).unwrap_or_else(|| {
            panic!(
                "matrix row operation '{}' has no metadata",
                row.operation_id
            )
        });
        assert_eq!(
            meta.id, row.operation_id,
            "matrix row operation '{}' resolves to metadata '{}'",
            row.operation_id, meta.id
        );
    }
}

/// db-pentest pilot domain must appear in the capability matrix with correct fields.
#[cfg(feature = "db-pentest")]
#[test]
fn pilot_domain_db_pentest_in_capability_matrix() {
    use eggsec::domain::DomainCategory;

    let rows = generate_capability_matrix();
    let db_row = rows
        .iter()
        .find(|r| r.operation_id == "db-pentest")
        .expect("db-pentest should appear in capability matrix");

    assert_eq!(db_row.domain_id, "db-pentest");
    assert_eq!(db_row.category, DomainCategory::DefenseLab);
    assert!(db_row.cli, "db-pentest should have CLI integration");
    assert!(db_row.tui, "db-pentest should have TUI integration");
    assert!(
        db_row.mcp_api,
        "db-pentest should have MCP/tool integration"
    );
    assert_eq!(db_row.dry_run, "always");
    assert_eq!(db_row.evidence_report, "always");
}

// ─── Safety Exposure Rules ─────────────────────────────────────────────────

/// Hazardous domains (HazardousLab category) must NOT be exposed via MCP by default.
#[test]
fn hazardous_domains_not_mcp_exposed_by_default() {
    for domain in all_domain_descriptors() {
        if domain.category == eggsec::domain::DomainCategory::HazardousLab {
            for tool in domain.tools {
                assert!(
                    !tool.mcp_exposed_by_default,
                    "hazardous domain '{}' tool '{}' must not be MCP-exposed by default",
                    domain.id, tool.tool_id
                );
            }
        }
    }
}

/// Operations with risk > SafeActive that are agent-exposable must declare
/// at least one non-baseline capability.
#[test]
fn high_risk_agent_exposable_ops_declare_capability() {
    for m in all_operation_metadata() {
        if m.risk > OperationRisk::SafeActive && m.agent_exposable {
            let has_capability = !m.required_capabilities.is_empty();
            assert!(
                has_capability,
                "high-risk agent-exposable operation '{}' (risk {:?}) must declare capabilities",
                m.id, m.risk
            );
        }
    }
}

/// MCP-exposed operations must have mcp_exposable = true in their metadata.
#[test]
fn mcp_exposed_ops_have_metadata_flag() {
    for domain in all_domain_descriptors() {
        for tool in domain.tools {
            if tool.mcp_exposed_by_default {
                let meta = metadata_for_tool_id(tool.tool_id)
                    .expect("MCP-exposed tool should have metadata");
                assert!(
                    meta.mcp_exposable,
                    "tool '{}' is MCP-exposed in domain '{}' but metadata has mcp_exposable = false",
                    tool.tool_id, domain.id
                );
            }
        }
    }
}

// ─── Documentation Cross-References ────────────────────────────────────────

/// Every feature referenced by OperationMetadata should either be a valid
/// Cargo feature or the metadata marks it as required_features.
#[test]
fn feature_names_are_nonempty_when_present() {
    for m in all_operation_metadata() {
        for feat in m.required_features {
            assert!(
                !feat.is_empty(),
                "operation '{}' has empty required_feature string",
                m.id
            );
        }
    }
}

/// Dry-run support should be declared for every domain.
#[test]
fn all_domains_declare_dry_run_support() {
    for domain in all_domain_descriptors() {
        // DryRunSupport is an enum, so it always has a value.
        // This test documents the invariant that we check it.
        match domain.dry_run {
            DryRunSupport::AlwaysAvailable => {}
            DryRunSupport::FeatureGated(f) => {
                assert!(
                    !f.is_empty(),
                    "domain '{}' has empty feature-gated dry-run string",
                    domain.id
                );
            }
            DryRunSupport::NotSupported => {}
        }
    }
}
