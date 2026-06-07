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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RemediationPriority {
    Critical,
    High,
    Medium,
    Low,
}

impl PartialOrd for RemediationPriority {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for RemediationPriority {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.as_int().cmp(&other.as_int())
    }
}

impl RemediationPriority {
    fn as_int(&self) -> i32 {
        match self {
            Self::Critical => 4,
            Self::High => 3,
            Self::Medium => 2,
            Self::Low => 1,
        }
    }
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
                24.0,
                vec![
                    "Review and understand the finding".to_string(),
                    "Plan fix for future release".to_string(),
                    "Implement and test".to_string(),
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

    pub fn from_severity(severity: &str) -> Self {
        let sev = match severity.to_lowercase().as_str() {
            "critical" => Severity::Critical,
            "high" => Severity::High,
            "medium" => Severity::Medium,
            "low" => Severity::Low,
            _ => Severity::Info,
        };
        Self::for_finding("default", "Finding", sev)
    }

    pub fn priority(&self) -> &RemediationPriority {
        &self.priority
    }

    pub fn effort(&self) -> f32 {
        self.effort_hours
    }

    pub fn steps(&self) -> &[String] {
        &self.steps
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

    #[test]
    fn test_remediation_priority_ordering() {
        assert!(RemediationPriority::Critical > RemediationPriority::High);
        assert!(RemediationPriority::High > RemediationPriority::Medium);
        assert!(RemediationPriority::Medium > RemediationPriority::Low);
    }

    #[test]
    fn test_remediation_priority_sort() {
        let mut priorities = vec![
            RemediationPriority::Low,
            RemediationPriority::Critical,
            RemediationPriority::Medium,
            RemediationPriority::High,
        ];
        priorities.sort();
        assert_eq!(
            priorities,
            vec![
                RemediationPriority::Low,
                RemediationPriority::Medium,
                RemediationPriority::High,
                RemediationPriority::Critical,
            ]
        );
    }
}
