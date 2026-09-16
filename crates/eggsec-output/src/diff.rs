//! Diff summary DTO (compatibility facade).
//!
//! The canonical [`DiffSummary`] owner is `eggsec-report-model` (Phase B).
//! Comparison behavior that consumes engine-internal reports stays in the
//! engine (`output::run_manifest`); this module only preserves the
//! `eggsec_output::diff::DiffSummary` import path.

pub use eggsec_report_model::DiffSummary;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diff_summary() {
        let summary = DiffSummary {
            total_new: 5,
            total_resolved: 3,
            total_escalated: 1,
            total_deescalated: 2,
            net_change: 2,
        };
        assert_eq!(summary.net_change, 2);
    }
}
