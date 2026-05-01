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

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkflowReport {
    pub total_findings: usize,
    pub open_findings: usize,
    pub in_progress_findings: usize,
    pub resolved_findings: usize,
    pub sla_violations: usize,
}

impl WorkflowReport {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn calculate_metrics(&mut self) {
        self.sla_violations = self.open_findings;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_report() {
        let mut report = WorkflowReport::new();
        report.total_findings = 10;
        report.open_findings = 5;
        report.calculate_metrics();
        assert_eq!(report.sla_violations, 5);
    }
}
