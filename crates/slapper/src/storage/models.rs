use crate::error::Result;
use crate::storage::models::*;
use crate::types::Severity;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredScan {
    pub id: String,
    pub target: String,
    pub scan_type: String,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub status: ScanStatus,
    pub findings_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ScanStatus {
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredFinding {
    pub id: String,
    pub scan_id: String,
    pub title: String,
    pub description: String,
    pub severity: Severity,
    pub status: FindingStatus,
    pub cvss_score: Option<f32>,
    pub cve_ids: Vec<String>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredUser {
    pub id: String,
    pub username: String,
    pub email: String,
    pub role: UserRole,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum UserRole {
    Admin,
    Analyst,
    Viewer,
}

impl StoredScan {
    pub fn new(target: &str, scan_type: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            target: target.to_string(),
            scan_type: scan_type.to_string(),
            started_at: chrono::Utc::now(),
            completed_at: None,
            status: ScanStatus::Running,
            findings_count: 0,
        }
    }

    pub fn complete(&mut self) {
        self.completed_at = Some(chrono::Utc::now());
        self.status = ScanStatus::Completed;
    }
}

impl StoredFinding {
    pub fn new(scan_id: &str, title: &str, severity: Severity) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            scan_id: scan_id.to_string(),
            title: title.to_string(),
            description: String::new(),
            severity,
            status: FindingStatus::Open,
            cvss_score: None,
            cve_ids: vec![],
            created_at: now,
            updated_at: now,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_creation() {
        let scan = StoredScan::new("http://example.com", "recon");
        assert_eq!(scan.status, ScanStatus::Running);
        assert!(scan.completed_at.is_none());
    }

    #[test]
    fn test_finding_creation() {
        let finding = StoredFinding::new("scan-1", "Test Finding", Severity::High);
        assert_eq!(finding.status, FindingStatus::Open);
        assert_eq!(finding.severity, Severity::High);
    }
}
