//! Workflow management module
//!
//! Provides finding management, status workflow, assignment, comments, and SLA tracking.
//!
//! ## Modules
//!
//! - [`finding`] - Finding management
//! - [`status`] - Status workflow transitions
//! - [`assignment`] - Finding assignment
//! - [`comments`] - Finding comments
//! - [`sla`] - SLA tracking

pub mod assignment;
pub mod comments;
pub mod finding;
pub mod sla;
pub mod status;

use crate::workflow::finding::{Finding, FindingStatus};
use crate::workflow::sla::calculate_sla;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkflowReport {
    pub total_findings: usize,
    pub open_findings: usize,
    pub in_progress_findings: usize,
    pub resolved_findings: usize,
    pub sla_violations: usize,
    pub findings: Vec<Finding>,
}

impl WorkflowReport {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn calculate_metrics(&mut self) {
        self.total_findings = self.findings.len();
        self.open_findings = 0;
        self.in_progress_findings = 0;
        self.resolved_findings = 0;
        self.sla_violations = 0;

        for finding in &self.findings {
            match finding.status {
                FindingStatus::Open => {
                    self.open_findings += 1;
                    let sla = calculate_sla(&finding.id, finding.severity, finding.created_at);
                    if sla.is_violated {
                        self.sla_violations += 1;
                    }
                }
                FindingStatus::InProgress => {
                    self.in_progress_findings += 1;
                }
                FindingStatus::Resolved | FindingStatus::Verified => {
                    self.resolved_findings += 1;
                }
                FindingStatus::FalsePositive => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow::finding::FindingStatus;
    use crate::types::Severity;

    fn make_finding(id: &str, severity: Severity, status: FindingStatus, hours_ago: i64) -> Finding {
        Finding {
            id: id.to_string(),
            title: format!("Finding {id}"),
            description: String::new(),
            severity,
            status,
            assignee: None,
            created_at: chrono::Utc::now() - chrono::Duration::hours(hours_ago),
            updated_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn test_workflow_report_default() {
        let report = WorkflowReport::new();
        assert_eq!(report.sla_violations, 0);
        assert!(report.findings.is_empty());
    }

    #[test]
    fn test_sla_violations_count_only_open_findings() {
        let mut report = WorkflowReport::new();
        // Critical SLA is 24h. Created 48h ago -> violated.
        report.findings.push(make_finding("f1", Severity::Critical, FindingStatus::Open, 48));
        // High SLA is 168h. Created 200h ago -> violated.
        report.findings.push(make_finding("f2", Severity::High, FindingStatus::Open, 200));
        // This one is resolved, should be ignored even if overdue.
        report.findings.push(make_finding("f3", Severity::Critical, FindingStatus::Resolved, 100));
        // InProgress, should be ignored.
        report.findings.push(make_finding("f4", Severity::Medium, FindingStatus::InProgress, 800));

        report.calculate_metrics();

        assert_eq!(report.total_findings, 4);
        assert_eq!(report.open_findings, 2);
        assert_eq!(report.in_progress_findings, 1);
        assert_eq!(report.resolved_findings, 1);
        assert_eq!(report.sla_violations, 2);
    }

    #[test]
    fn test_sla_violations_zero_when_all_within_sla() {
        let mut report = WorkflowReport::new();
        // Critical SLA is 24h. Created 1h ago -> not violated.
        report.findings.push(make_finding("f1", Severity::Critical, FindingStatus::Open, 1));
        // High SLA is 168h. Created 10h ago -> not violated.
        report.findings.push(make_finding("f2", Severity::High, FindingStatus::Open, 10));

        report.calculate_metrics();

        assert_eq!(report.total_findings, 2);
        assert_eq!(report.open_findings, 2);
        assert_eq!(report.sla_violations, 0);
    }

    #[test]
    fn test_sla_violations_mixed_statuses() {
        let mut report = WorkflowReport::new();
        // Open and violated
        report.findings.push(make_finding("f1", Severity::Critical, FindingStatus::Open, 48));
        // Open and NOT violated
        report.findings.push(make_finding("f2", Severity::High, FindingStatus::Open, 10));
        // Resolved and violated (should not count)
        report.findings.push(make_finding("f3", Severity::Critical, FindingStatus::Resolved, 48));
        // FalsePositive and violated (should not count)
        report.findings.push(make_finding("f4", Severity::High, FindingStatus::FalsePositive, 200));
        // Verified and violated (should not count as open)
        report.findings.push(make_finding("f5", Severity::Medium, FindingStatus::Verified, 800));

        report.calculate_metrics();

        assert_eq!(report.total_findings, 5);
        assert_eq!(report.open_findings, 2);
        assert_eq!(report.resolved_findings, 2); // Resolved + Verified
        assert_eq!(report.sla_violations, 1);
    }

    #[test]
    fn test_sla_violations_empty_findings() {
        let mut report = WorkflowReport::new();
        report.calculate_metrics();
        assert_eq!(report.total_findings, 0);
        assert_eq!(report.sla_violations, 0);
    }

    #[test]
    fn test_calculate_metrics_computes_all_fields() {
        let mut report = WorkflowReport::new();
        report.findings.push(make_finding("f1", Severity::Critical, FindingStatus::Open, 1));
        report.findings.push(make_finding("f2", Severity::High, FindingStatus::InProgress, 10));
        report.findings.push(make_finding("f3", Severity::Medium, FindingStatus::Resolved, 20));
        report.findings.push(make_finding("f4", Severity::Low, FindingStatus::Verified, 30));
        report.findings.push(make_finding("f5", Severity::Info, FindingStatus::FalsePositive, 40));

        report.calculate_metrics();

        assert_eq!(report.total_findings, 5);
        assert_eq!(report.open_findings, 1);
        assert_eq!(report.in_progress_findings, 1);
        assert_eq!(report.resolved_findings, 2);
        assert_eq!(report.sla_violations, 0);
    }
}
