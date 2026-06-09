use serde::{Deserialize, Serialize};

use crate::config::{ExecutionBudget, PolicyDecision};
use crate::output::PolicySummary;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabDefenseReportSection {
    pub policy_summary: PolicyDecision,
    pub scope_summary: ScopeSummary,
    pub feature_flags_used: Vec<String>,
    pub risk_tiers_executed: Vec<String>,
    pub budget_summary: BudgetSummary,
    pub target_resolution: TargetResolutionSummary,
    pub skipped_operations: Vec<SkippedOperation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScopeSummary {
    pub scope_file: Option<String>,
    pub allowed_rules: Vec<String>,
    pub excluded_rules: Vec<String>,
    pub require_explicit_scope: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetSummary {
    pub max_duration_secs: u64,
    pub max_requests: Option<u64>,
    pub max_packets: Option<u64>,
    pub max_bytes: Option<u64>,
    pub max_concurrency: usize,
    pub duration_consumed_secs: Option<f64>,
    pub requests_consumed: Option<u64>,
    pub termination_reason: Option<String>,
}

impl From<&ExecutionBudget> for BudgetSummary {
    fn from(budget: &ExecutionBudget) -> Self {
        Self {
            max_duration_secs: budget.max_duration_secs,
            max_requests: budget.max_requests,
            max_packets: budget.max_packets,
            max_bytes: budget.max_bytes,
            max_concurrency: budget.max_concurrency,
            duration_consumed_secs: None,
            requests_consumed: None,
            termination_reason: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetResolutionSummary {
    pub original: Option<String>,
    pub normalized: Option<String>,
    pub resolved_addresses: Vec<String>,
    pub matched_scope_rules: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkippedOperation {
    pub operation: String,
    pub reason: String,
    pub required_risk: Option<String>,
    pub budget_remaining: Option<String>,
}

impl LabDefenseReportSection {
    pub fn to_human_readable(&self) -> String {
        let mut lines = Vec::new();
        lines.push("Lab Defense Report Section".to_string());
        lines.push(String::new());

        let status = if self.policy_summary.allowed {
            "ALLOWED"
        } else {
            "DENIED"
        };
        lines.push(format!(
            "Policy: {} ({})",
            status, self.policy_summary.operation_mode
        ));
        lines.push(format!("  Risk: {}", self.policy_summary.operation_risk));

        if !self.feature_flags_used.is_empty() {
            lines.push(format!(
                "  Features: {}",
                self.feature_flags_used.join(", ")
            ));
        }
        if !self.risk_tiers_executed.is_empty() {
            lines.push(format!(
                "  Risk tiers: {}",
                self.risk_tiers_executed.join(", ")
            ));
        }

        lines.push(format!(
            "Budget: {}s max, {} reqs max",
            self.budget_summary.max_duration_secs,
            self.budget_summary
                .max_requests
                .map_or("unlimited".to_string(), |r| r.to_string())
        ));

        if !self.skipped_operations.is_empty() {
            lines.push("Skipped operations:".to_string());
            for op in &self.skipped_operations {
                lines.push(format!("  - {}: {}", op.operation, op.reason));
            }
        }

        lines.join("\n")
    }
}

impl From<&PolicyDecision> for PolicySummary {
    fn from(decision: &PolicyDecision) -> Self {
        Self {
            operation_mode: decision.operation_mode.to_string(),
            max_risk: decision.operation_risk.to_string(),
            total_decisions: 1,
            denied_count: if decision.allowed { 0 } else { 1 },
            warning_count: decision.warnings.len(),
            denied_reasons: decision.denied_reasons.clone(),
            warnings: decision.warnings.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{IntendedUse, OperationMode, OperationRisk};

    #[test]
    fn budget_summary_from_execution_budget() {
        let budget = ExecutionBudget::defense_lab_default();
        let summary = BudgetSummary::from(&budget);
        assert_eq!(summary.max_duration_secs, 300);
        assert!(summary.max_requests.is_some());
    }

    #[test]
    fn lab_report_human_readable() {
        let policy = PolicyDecision::allowed(
            "test",
            OperationMode::DefenseLab,
            OperationRisk::Intrusive,
            vec![IntendedUse::WafRegression],
        );
        let budget = ExecutionBudget::defense_lab_default();
        let section = LabDefenseReportSection {
            policy_summary: policy,
            scope_summary: ScopeSummary {
                scope_file: Some("test.toml".to_string()),
                allowed_rules: vec!["127.0.0.1".to_string()],
                excluded_rules: vec![],
                require_explicit_scope: true,
            },
            feature_flags_used: vec!["stress-testing".to_string()],
            risk_tiers_executed: vec!["intrusive".to_string()],
            budget_summary: BudgetSummary::from(&budget),
            target_resolution: TargetResolutionSummary {
                original: Some("127.0.0.1".to_string()),
                normalized: Some("127.0.0.1".to_string()),
                resolved_addresses: vec!["127.0.0.1".to_string()],
                matched_scope_rules: vec!["127.0.0.1/32".to_string()],
            },
            skipped_operations: vec![],
        };
        let text = section.to_human_readable();
        assert!(text.contains("Lab Defense Report Section"));
        assert!(text.contains("ALLOWED"));
    }
}
