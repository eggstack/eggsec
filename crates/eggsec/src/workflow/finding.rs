use crate::error::{Result, EggsecError};
use crate::types::Severity;
use crate::workflow::status::StatusWorkflow;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub id: String,
    pub title: String,
    pub description: String,
    pub severity: Severity,
    pub status: FindingStatus,
    pub assignee: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FindingStatus {
    #[default]
    Open,
    InProgress,
    Resolved,
    Verified,
    FalsePositive,
}

impl std::fmt::Display for FindingStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Open => write!(f, "open"),
            Self::InProgress => write!(f, "in_progress"),
            Self::Resolved => write!(f, "resolved"),
            Self::Verified => write!(f, "verified"),
            Self::FalsePositive => write!(f, "false_positive"),
        }
    }
}

impl Finding {
    pub fn new(title: &str, severity: Severity) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            title: title.to_string(),
            description: String::new(),
            severity,
            status: FindingStatus::Open,
            assignee: None,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn assign(&mut self, user: &str) {
        self.assignee = Some(user.to_string());
        self.updated_at = chrono::Utc::now();
    }

    pub fn update_status(&mut self, new_status: FindingStatus) -> Result<()> {
        if !StatusWorkflow::can_transition(&self.status, &new_status) {
            return Err(EggsecError::Validation(format!(
                "Invalid transition from {:?} to {:?}",
                self.status, new_status
            )));
        }
        self.status = new_status;
        self.updated_at = chrono::Utc::now();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_finding_creation() {
        let finding = Finding::new("Test Finding", Severity::High);
        assert_eq!(finding.status, FindingStatus::Open);
        assert!(finding.assignee.is_none());
    }

    #[test]
    fn test_finding_assignment() {
        let mut finding = Finding::new("Test", Severity::Medium);
        finding.assign("analyst@example.com");
        assert_eq!(finding.assignee, Some("analyst@example.com".to_string()));
    }

    #[test]
    fn test_update_status_valid() {
        let mut finding = Finding::new("Test", Severity::High);
        assert!(finding.update_status(FindingStatus::InProgress).is_ok());
        assert_eq!(finding.status, FindingStatus::InProgress);
    }

    #[test]
    fn test_update_status_invalid() {
        let mut finding = Finding::new("Test", Severity::High);
        let result = finding.update_status(FindingStatus::Verified);
        assert!(result.is_err());
        assert_eq!(finding.status, FindingStatus::Open);
    }

    #[test]
    fn test_finding_status_display() {
        assert_eq!(FindingStatus::Open.to_string(), "open");
        assert_eq!(FindingStatus::InProgress.to_string(), "in_progress");
        assert_eq!(FindingStatus::Resolved.to_string(), "resolved");
        assert_eq!(FindingStatus::Verified.to_string(), "verified");
        assert_eq!(FindingStatus::FalsePositive.to_string(), "false_positive");
    }

    #[test]
    fn test_finding_status_default() {
        assert_eq!(FindingStatus::default(), FindingStatus::Open);
    }

    #[test]
    fn test_finding_has_uuid() {
        let finding = Finding::new("Test", Severity::High);
        assert!(!finding.id.is_empty());
        let finding2 = Finding::new("Test", Severity::High);
        assert_ne!(finding.id, finding2.id);
    }

    #[test]
    fn test_finding_timestamps() {
        let before = chrono::Utc::now();
        let finding = Finding::new("Test", Severity::High);
        let after = chrono::Utc::now();
        assert!(finding.created_at >= before && finding.created_at <= after);
        assert!(finding.updated_at >= before && finding.updated_at <= after);
    }

    #[test]
    fn test_finding_assign_updates_timestamp() {
        let mut finding = Finding::new("Test", Severity::High);
        let original_updated = finding.updated_at;
        std::thread::sleep(std::time::Duration::from_millis(10));
        finding.assign("user@example.com");
        assert!(finding.updated_at > original_updated);
    }

    #[test]
    fn test_finding_update_status_updates_timestamp() {
        let mut finding = Finding::new("Test", Severity::High);
        let original_updated = finding.updated_at;
        std::thread::sleep(std::time::Duration::from_millis(10));
        finding.update_status(FindingStatus::InProgress).unwrap();
        assert!(finding.updated_at > original_updated);
    }

    #[test]
    fn test_full_status_workflow() {
        let mut finding = Finding::new("Test", Severity::High);
        assert_eq!(finding.status, FindingStatus::Open);

        finding.update_status(FindingStatus::InProgress).unwrap();
        assert_eq!(finding.status, FindingStatus::InProgress);

        finding.update_status(FindingStatus::Resolved).unwrap();
        assert_eq!(finding.status, FindingStatus::Resolved);

        finding.update_status(FindingStatus::Verified).unwrap();
        assert_eq!(finding.status, FindingStatus::Verified);

        finding.update_status(FindingStatus::Open).unwrap();
        assert_eq!(finding.status, FindingStatus::Open);
    }

    #[test]
    fn test_false_positive_workflow() {
        let mut finding = Finding::new("Test", Severity::High);
        finding.update_status(FindingStatus::FalsePositive).unwrap();
        assert_eq!(finding.status, FindingStatus::FalsePositive);

        finding.update_status(FindingStatus::Open).unwrap();
        assert_eq!(finding.status, FindingStatus::Open);
    }
}
