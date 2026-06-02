use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[allow(unused_imports)]
pub use crate::findings::lifecycle::{FindingStatus, StoredFinding, StatusChange};

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_creation() {
        let scan = StoredScan::new("http://example.com", "recon");
        assert_eq!(scan.status, ScanStatus::Running);
        assert!(scan.completed_at.is_none());
    }
}
