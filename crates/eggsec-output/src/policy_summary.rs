//! Policy summary DTO (compatibility facade).
//!
//! The canonical [`PolicySummary`] owner is `eggsec-report-model` (Phase B).
//! This module re-exports it so existing `eggsec_output::policy_summary::*`
//! and `eggsec_output::PolicySummary` paths keep working.

pub use eggsec_report_model::PolicySummary;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_policy_summary_is_empty() {
        let summary = PolicySummary::default();
        assert_eq!(summary.total_decisions, 0);
        assert_eq!(summary.denied_count, 0);
        assert!(summary.denied_reasons.is_empty());
    }

    #[test]
    fn policy_summary_serializes() {
        let summary = PolicySummary {
            operation_mode: "defense-lab".to_string(),
            max_risk: "intrusive".to_string(),
            total_decisions: 1,
            denied_count: 0,
            warning_count: 1,
            denied_reasons: vec![],
            warnings: vec!["target is a private IP".to_string()],
        };
        let json = serde_json::to_string(&summary).unwrap();
        assert!(json.contains("\"defense-lab\""));
        assert!(json.contains("\"intrusive\""));
    }
}
