//! Pure serializable summary DTOs.
//!
//! Data-only summaries moved verbatim from `eggsec-output` (Phase B).
//! Analysis and comparison behavior stays in `eggsec-output`:
//! - `baseline::BaselineComparison` (finding-set comparison over agent findings);
//! - `trend::{ResultComparator, TrendAnalyzer}` (trend computation + LRU history);
//! - `audit_summary::{AuditSummary::from_values, from_serde_value}` (event aggregation).
//!
//! Only [`PolicySummary`] and [`DiffSummary`] move here because they are pure
//! data with no behavior. [`AuditSummary`]'s struct is data-only, but its only
//! constructors aggregate audit events, so the whole module stays in
//! `eggsec-output` to avoid splitting struct from behavior.

use serde::{Deserialize, Serialize};

/// Summary of policy decisions for a scan run.
///
/// This is a standalone struct that can be populated by the `eggsec` crate
/// from its internal `PolicyDecision` types. It lives in the report model so
/// that report formats can include policy context without depending on the
/// engine crate.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PolicySummary {
    /// The operation mode (e.g. "standard-assessment", "defense-lab").
    pub operation_mode: String,
    /// The maximum risk tier allowed (e.g. "safe-active", "intrusive").
    pub max_risk: String,
    /// Total number of policy decisions evaluated.
    pub total_decisions: usize,
    /// Number of decisions that resulted in denial.
    pub denied_count: usize,
    /// Number of decisions that generated warnings.
    pub warning_count: usize,
    /// Reasons for any denials.
    pub denied_reasons: Vec<String>,
    /// Warning messages from policy evaluation.
    pub warnings: Vec<String>,
}

/// Lightweight diff envelope for run manifests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffSummary {
    pub total_new: usize,
    pub total_resolved: usize,
    pub total_escalated: usize,
    pub total_deescalated: usize,
    pub net_change: i32,
}
