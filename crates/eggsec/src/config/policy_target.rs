//! Target normalization facade.
//!
//! Canonical owner is `eggsec-policy` (Phase C). Re-exported here so
//! `crate::config::policy_target::{...}` paths keep working.

// Compatibility shim: this deep path is intentionally preserved for
// downstream users even when no in-tree code references it directly.
#[allow(unused_imports)]
pub use eggsec_policy::target::{
    normalize_target, DescriptorError, OperationTarget, TargetHint, TargetPolicyKind,
};
