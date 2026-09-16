//! Policy vocabulary facade.
//!
//! Canonical owner is `eggsec-policy` (Phase C). This module re-exports the
//! pure vocabulary, [`ExecutionPolicy`], and [`OperationDescriptor`] so
//! existing `crate::config::policy::{...}` paths keep working. New engine
//! code should import from `eggsec_policy` directly.

pub use eggsec_policy::policy::{
    baseline_allowed_capability, normalize_target, Capability, DenialClass, DescriptorError,
    ExecutionPolicy, ExecutionProfile, ExecutionSurface, IntendedUse, OperationDescriptor,
    OperationMode, OperationRisk, OperationTarget, TargetHint, TargetPolicyKind,
};
pub use eggsec_policy::{
    all_operation_metadata, metadata_for_tool_id, operation_matches_tool_id, operation_metadata,
    OperationMetadata, ALL_OPERATION_METADATA, ALL_OPERATION_METADATA_ALIASES,
};
