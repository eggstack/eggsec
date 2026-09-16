//! Deterministic authorization/enforcement semantics for Eggsec.
//!
//! `eggsec-policy` owns the complete authorization semantic domain as pure,
//! I/O-free data types and algorithms:
//!
//! - policy vocabulary (`OperationRisk`, `OperationMode`,
//!   `ExecutionProfile`/`ExecutionSurface`, `IntendedUse`, `Capability`,
//!   denial classes);
//! - serializable [`ExecutionPolicy`];
//! - operation descriptors, target normalization, and the operation catalog;
//! - scope data model and pure matching over caller-supplied facts;
//! - address classification;
//! - policy decisions, preflight/enforcement outcomes, confirmation classes,
//!   manual overrides;
//! - approval-token binding ([`ApprovedOperation`]);
//! - deterministic evaluation over explicit inputs ([`EnabledFeatures`] +
//!   [`TargetScope`] facts).
//!
//! What this crate deliberately does **not** own:
//!
//! - config-file discovery/loading and environment overrides (engine);
//! - compile-time feature discovery (engine `feature_registry`; the engine
//!   maps it to [`EnabledFeatures`] before evaluation);
//! - concrete DNS resolution (engine resolver bridge; policy consumes
//!   [`TargetScope`] facts but never acquires them);
//! - the `NetworkAuthority` transport checkpoint adapter (`ScopeAuthority`
//!   lives in the engine policy bridge over `eggsec-transport`);
//! - audit emission and report-summary conversion (engine/output over
//!   `eggsec-report-model`);
//! - network I/O, filesystems, Tokio, HTTP/TLS clients, or frontends.
//!
//! This is not the rejected `eggsec-net` middle layer: it owns no network
//! stack, only the authorization semantics. See
//! `architecture/capability_segregation.md`.

pub mod address;
pub mod approval;
pub mod catalog;
pub mod decision;
pub mod features;
pub mod policy;
pub mod scope;
pub mod target;

// Canonical re-exports: new code imports policy types from the crate root.
pub use address::{classify_address, is_private_ip, AddressClass};
pub use approval::ApprovedOperation;
pub use catalog::{
    all_operation_metadata, metadata_for_tool_id, operation_matches_tool_id, operation_metadata,
    OperationMetadata, ALL_OPERATION_METADATA, ALL_OPERATION_METADATA_ALIASES,
};
pub use decision::{
    classify_denial_reasons, confirmation_class_strings, confirmation_classes_for,
    evaluate_enforcement, evaluate_operation_policy, may_downgrade_to_warning, preflight_operation,
    ConfirmationClass, EnforcementContext, EnforcementError, EnforcementOutcome, ManualOverride,
    PolicyDecision, PreflightOutcomeKind, PreflightResult,
};
pub use features::EnabledFeatures;
pub use policy::{
    baseline_allowed_capability, normalize_target, Capability, DenialClass, DescriptorError,
    ExecutionPolicy, ExecutionProfile, ExecutionSurface, IntendedUse, OperationDescriptor,
    OperationMode, OperationRisk, OperationTarget, TargetHint, TargetPolicyKind,
};
pub use scope::{LoadedScope, Scope, ScopeError, ScopeRule, ScopeSource, TargetScope};
