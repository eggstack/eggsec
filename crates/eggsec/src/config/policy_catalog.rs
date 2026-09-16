//! Operation catalog facade.
//!
//! Canonical owner is `eggsec-policy` (Phase C). Re-exported here so
//! `crate::config::policy_catalog::{...}` paths keep working. The engine-only
//! `derive_operation_integration` helper (which couples catalog metadata to
//! `crate::domain`) lives here as an extension, not in the policy kernel.

// Compatibility shim: this deep path is intentionally preserved for
// downstream users even when no in-tree code references it directly.
#[allow(unused_imports)]
pub use eggsec_policy::catalog::{
    all_operation_metadata, metadata_for_tool_id, operation_matches_tool_id, operation_metadata,
    OperationMetadata, ALL_OPERATION_METADATA, ALL_OPERATION_METADATA_ALIASES,
};

/// Derive an [`crate::domain::OperationIntegration`] for domain descriptor construction.
///
/// Engine-side extension: mirrors metadata mode, risk, capabilities, features,
/// and target policy. Kept out of the policy kernel because it couples the
/// catalog to engine domain types.
pub fn derive_operation_integration(
    metadata: &'static OperationMetadata,
) -> crate::domain::OperationIntegration {
    use eggsec_policy::TargetPolicyKind;
    crate::domain::OperationIntegration {
        operation_id: metadata.id,
        display_name: metadata.display_name,
        mode: metadata.mode,
        risk: metadata.risk,
        capabilities: metadata.required_capabilities,
        intended_uses: metadata.intended_uses,
        required_features: metadata.required_features,
        requires_explicit_scope: matches!(
            metadata.target_policy,
            TargetPolicyKind::ExplicitScopeRequired | TargetPolicyKind::PrivateOrLocalRequired
        ),
        requires_private_or_local_target: matches!(
            metadata.target_policy,
            TargetPolicyKind::PrivateOrLocalRequired
        ),
    }
}
