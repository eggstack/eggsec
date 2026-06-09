use serde::{Deserialize, Serialize};

/// Summary of policy decisions for a scan run.
///
/// This is a standalone struct that can be populated by the `eggsec` crate
/// from its internal `PolicyDecision` types. It lives in `eggsec-output`
/// so that report formats can include policy context without depending on
/// the engine crate.
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
