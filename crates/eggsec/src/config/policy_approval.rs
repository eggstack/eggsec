//! Approval token facade.
//!
//! Canonical owner is `eggsec-policy` (Phase C). Re-exported here so
//! `crate::config::policy_approval::ApprovedOperation` and
//! `crate::config::ApprovedOperation` remain stable paths.

// Compatibility shim: this deep path is intentionally preserved for
// downstream users even when no in-tree code references it directly.
#[allow(unused_imports)]
pub use eggsec_policy::approval::ApprovedOperation;
