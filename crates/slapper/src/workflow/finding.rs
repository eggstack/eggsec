use crate::error::Result;
use crate::types::Severity;
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum FindingStatus {
    Open,
    InProgress,
    Resolved,
    Verified,
    FalsePositive,
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
}
