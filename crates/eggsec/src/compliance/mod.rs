//! Compliance reporting module
//!
//! Provides compliance checking against security frameworks and standards.
//!
//! ## Modules
//!
//! - [`owasp`] - OWASP Top 10 mapping and compliance
//! - [`pci`] - PCI DSS compliance checks
//! - [`hipaa`] - HIPAA compliance checks
//! - [`soc2`] - SOC 2 compliance checks

pub mod hipaa;
pub mod owasp;
pub mod pci;
pub mod report;
pub mod soc2;

use crate::error::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ComplianceReport {
    pub framework: String,
    pub target: String,
    pub overall_score: f32,
    pub total_requirements: usize,
    pub passed: usize,
    pub failed: usize,
    pub findings: Vec<ComplianceFinding>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceFinding {
    pub requirement_id: String,
    pub description: String,
    pub severity: crate::types::Severity,
    pub status: ComplianceStatus,
    pub remediation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ComplianceStatus {
    Pass,
    Fail,
    NotApplicable,
    NeedsReview,
}

pub async fn generate_compliance_report(
    target: &str,
    framework: ComplianceFramework,
    findings: &[crate::types::Severity],
) -> Result<ComplianceReport> {
    match framework {
        ComplianceFramework::OWASP => owasp::generate_report(target, findings).await,
        ComplianceFramework::PCIDSS => pci::generate_report(target, findings).await,
        ComplianceFramework::HIPAA => hipaa::generate_report(target, findings).await,
        ComplianceFramework::SOC2 => soc2::generate_report(target, findings).await,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[allow(clippy::upper_case_acronyms)]
pub enum ComplianceFramework {
    OWASP,
    PCIDSS,
    HIPAA,
    SOC2,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_compliance_report_generation() {
        let findings = vec![];
        let report =
            generate_compliance_report("http://example.com", ComplianceFramework::OWASP, &findings)
                .await
                .unwrap();
        assert_eq!(report.framework, "OWASP Top 10");
    }
}
