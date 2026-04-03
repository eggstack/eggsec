use crate::types::Severity;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Remediation {
    pub finding_id: String,
    pub title: String,
    pub severity: Severity,
    pub effort_hours: f32,
    pub steps: Vec<String>,
    pub references: Vec<String>,
    pub priority: RemediationPriority,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum RemediationPriority {
    Critical,
    High,
    Medium,
    Low,
}

impl Remediation {
    pub fn for_finding(finding_id: &str, title: &str, severity: Severity) -> Self {
        let (effort_hours, steps, references, priority) = match severity {
            Severity::Critical => (
                4.0,
                vec![
                    "Immediately isolate affected system".to_string(),
                    "Apply emergency patch or mitigation".to_string(),
                    "Verify remediation".to_string(),
                    "Monitor for exploitation attempts".to_string(),
                ],
                vec![
                    "CVE Database".to_string(),
                    "Vendor Security Advisory".to_string(),
                ],
                RemediationPriority::Critical,
            ),
            Severity::High => (
                8.0,
                vec![
                    "Plan remediation window".to_string(),
                    "Test patch in staging environment".to_string(),
                    "Apply patch during maintenance window".to_string(),
                    "Verify and monitor".to_string(),
                ],
                vec!["CVE Database".to_string(), "OWASP Guidelines".to_string()],
                RemediationPriority::High,
            ),
            Severity::Medium => (
                16.0,
                vec![
                    "Schedule remediation in next sprint".to_string(),
                    "Develop and test fix".to_string(),
                    "Deploy to production".to_string(),
                    "Document lessons learned".to_string(),
                ],
                vec!["Security Best Practices".to_string()],
                RemediationPriority::Medium,
            ),
            Severity::Low => (
                40.0,
                vec![
                    "Add to technical debt backlog".to_string(),
                    "Address in next release cycle".to_string(),
                ],
                vec![],
                RemediationPriority::Low,
            ),
            Severity::Info => (
                0.0,
                vec!["No action required".to_string()],
                vec![],
                RemediationPriority::Low,
            ),
        };

        Self {
            finding_id: finding_id.to_string(),
            title: title.to_string(),
            severity,
            effort_hours,
            steps,
            references,
            priority,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remediation_for_critical() {
        let rem = Remediation::for_finding("f1", "RCE Vulnerability", Severity::Critical);
        assert_eq!(rem.effort_hours, 4.0);
        assert_eq!(rem.priority, RemediationPriority::Critical);
    }

    #[test]
    fn test_remediation_for_info() {
        let rem = Remediation::for_finding("f2", "Info finding", Severity::Info);
        assert_eq!(rem.effort_hours, 0.0);
    }
}
