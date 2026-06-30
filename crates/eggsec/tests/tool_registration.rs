//! Metadata consistency tests for tool registration (Phase 7).
//!
//! Validates that the canonical registration builder stays consistent
//! with OperationMetadata, DomainDescriptor, and protocol exposure rules.

use eggsec::config::{metadata_for_tool_id, OperationRisk};
use eggsec::domain::{all_domain_descriptors, DomainCategory};
use eggsec::tool::registration::{
    agent_tool_registrations, all_tool_registrations, grpc_tool_registrations,
    mcp_tool_registrations, rest_tool_registrations, ToolRegistrationSource,
};

// ─── Registration → OperationMetadata Cross-Validation ───────────────────

/// Every registration's operation_id must resolve via metadata_for_tool_id().
#[test]
fn every_tool_registration_resolves_to_operation_metadata() {
    for reg in all_tool_registrations() {
        assert!(
            metadata_for_tool_id(reg.operation_id).is_some(),
            "registration '{}' has operation_id '{}' with no matching OperationMetadata",
            reg.tool_id,
            reg.operation_id
        );
    }
}

/// All registrations with mcp_exposed_by_default == true must have
/// metadata.mcp_exposable == true (cross-check registration vs OperationMetadata).
#[test]
fn default_mcp_exposed_tools_have_metadata_flag() {
    for reg in all_tool_registrations() {
        if reg.mcp_exposed_by_default {
            let meta = metadata_for_tool_id(reg.operation_id)
                .expect("MCP-exposed registration should have metadata");
            assert!(
                meta.mcp_exposable,
                "registration '{}' has mcp_exposed_by_default=true but metadata has mcp_exposable=false",
                reg.tool_id
            );
        }
    }
}

/// Tools with required_mcp_feature.is_some() must have mcp_exposed_by_default == false.
#[test]
fn opt_in_mcp_tools_not_default_exposed() {
    for reg in all_tool_registrations() {
        if reg.required_mcp_feature.is_some() {
            assert!(
                !reg.mcp_exposed_by_default,
                "registration '{}' requires MCP feature '{}' but is MCP-exposed by default",
                reg.tool_id,
                reg.required_mcp_feature.unwrap()
            );
        }
    }
}

/// Domain-sourced registrations from HazardousLab domains must have
/// mcp_exposed_by_default == false.
#[test]
fn hazardous_domains_never_default_mcp_exposed() {
    let domains = all_domain_descriptors();
    for reg in all_tool_registrations() {
        if let ToolRegistrationSource::Domain(domain_id) = reg.source {
            let domain = domains
                .iter()
                .find(|d| d.id == domain_id)
                .expect("domain source should exist");
            if domain.category == DomainCategory::HazardousLab {
                assert!(
                    !reg.mcp_exposed_by_default,
                    "HazardousLab domain '{}' registration '{}' must not be MCP-exposed by default",
                    domain_id, reg.tool_id
                );
            }
        }
    }
}

/// Agent-exposable registrations with high risk must have required capabilities
/// in their metadata.
#[test]
fn high_risk_agent_exposable_ops_declare_capabilities() {
    for reg in all_tool_registrations() {
        if reg.agent_exposable {
            let meta = metadata_for_tool_id(reg.operation_id)
                .expect("agent-exposable registration should have metadata");
            if meta.risk > OperationRisk::SafeActive {
                assert!(
                    !meta.required_capabilities.is_empty(),
                    "high-risk agent-exposable registration '{}' (risk {:?}) must declare required capabilities",
                    reg.tool_id,
                    meta.risk
                );
            }
        }
    }
}

/// Registrations with source == FeatureGated(f) must have feature == Some(f)
/// and f must be non-empty.
#[test]
fn feature_gated_registrations_declare_nonempty_features() {
    for reg in all_tool_registrations() {
        if let ToolRegistrationSource::FeatureGated(f) = reg.source {
            assert!(
                !f.is_empty(),
                "registration '{}' has FeatureGated source with empty feature string",
                reg.tool_id
            );
            assert_eq!(
                reg.feature,
                Some(f),
                "registration '{}' has FeatureGated('{}') but feature field is {:?}",
                reg.tool_id,
                f,
                reg.feature
            );
        }
    }
}

/// The number of Base-source registrations should approximate the number
/// of tools in create_default_registry() (11 base tools).
#[test]
fn base_tool_count_matches_registry() {
    let base_count = all_tool_registrations()
        .iter()
        .filter(|r| r.source == ToolRegistrationSource::Base)
        .count();
    // create_default_registry() registers 11 base tools (always present).
    assert!(
        base_count >= 10,
        "expected at least 10 Base-source registrations, got {}",
        base_count
    );
}

/// rest_tool_registrations(), grpc_tool_registrations(), agent_tool_registrations()
/// must all be subsets of all_tool_registrations().
#[test]
fn all_protocol_registrations_are_subsets_of_all() {
    let all_ids: rustc_hash::FxHashSet<&str> =
        all_tool_registrations().iter().map(|r| r.tool_id).collect();

    for (label, regs) in [
        ("REST", rest_tool_registrations()),
        ("gRPC", grpc_tool_registrations()),
        ("Agent", agent_tool_registrations()),
    ] {
        for reg in regs {
            assert!(
                all_ids.contains(reg.tool_id),
                "{} registration '{}' not found in all_tool_registrations()",
                label,
                reg.tool_id
            );
        }
    }
}

/// mcp_tool_registrations("coding-agent") must be a subset of
/// mcp_tool_registrations("ops-agent").
#[test]
fn coding_agent_registrations_are_subset_of_mcp() {
    let ops_ids: rustc_hash::FxHashSet<&str> = mcp_tool_registrations("ops-agent")
        .iter()
        .map(|r| r.tool_id)
        .collect();

    for reg in mcp_tool_registrations("coding-agent") {
        assert!(
            ops_ids.contains(reg.tool_id),
            "coding-agent registration '{}' not found in ops-agent registrations",
            reg.tool_id
        );
    }
}

/// all_tool_registrations() must have unique tool_ids.
#[test]
fn no_duplicate_tool_ids_in_registrations() {
    let mut seen = rustc_hash::FxHashSet::default();
    for reg in all_tool_registrations() {
        assert!(
            seen.insert(reg.tool_id),
            "duplicate tool_id in registrations: '{}'",
            reg.tool_id
        );
    }
}
