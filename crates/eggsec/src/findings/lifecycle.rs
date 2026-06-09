use serde::{Deserialize, Serialize};

/// Finding lifecycle status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingStatus {
    New,
    Confirmed,
    AcceptedRisk,
    FalsePositive,
    Remediated,
    Reopened,
}

impl FindingStatus {
    /// Returns the set of valid target statuses from `self`.
    pub fn valid_transitions(&self) -> &'static [FindingStatus] {
        use FindingStatus::*;
        match self {
            New => &[Confirmed, FalsePositive, AcceptedRisk],
            Confirmed => &[Remediated, AcceptedRisk, FalsePositive],
            AcceptedRisk => &[Reopened, FalsePositive],
            FalsePositive => &[Reopened],
            Remediated => &[Reopened],
            Reopened => &[Confirmed, FalsePositive, AcceptedRisk],
        }
    }

    /// Check whether a transition from `self` to `target` is allowed.
    pub fn can_transition_to(&self, target: &FindingStatus) -> bool {
        self.valid_transitions().contains(target)
    }
}

impl std::fmt::Display for FindingStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::New => write!(f, "new"),
            Self::Confirmed => write!(f, "confirmed"),
            Self::AcceptedRisk => write!(f, "accepted_risk"),
            Self::FalsePositive => write!(f, "false_positive"),
            Self::Remediated => write!(f, "remediated"),
            Self::Reopened => write!(f, "reopened"),
        }
    }
}

/// A stored finding with lifecycle metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredFinding {
    pub finding: super::Finding,
    pub scan_id: String,
    pub status: FindingStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub status_history: Vec<StatusChange>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusChange {
    pub from: FindingStatus,
    pub to: FindingStatus,
    pub changed_at: chrono::DateTime<chrono::Utc>,
    pub note: Option<String>,
}

impl StoredFinding {
    pub fn new(finding: super::Finding, scan_id: impl Into<String>) -> Self {
        let now = chrono::Utc::now();
        Self {
            finding,
            scan_id: scan_id.into(),
            status: FindingStatus::New,
            created_at: now,
            updated_at: now,
            status_history: vec![],
        }
    }

    pub fn change_status(
        &mut self,
        new_status: FindingStatus,
        note: Option<String>,
    ) -> anyhow::Result<()> {
        if !self.status.can_transition_to(&new_status) {
            anyhow::bail!(
                "Invalid status transition: {:?} -> {:?}",
                self.status,
                new_status
            );
        }
        let old_status = self.status;
        self.status_history.push(StatusChange {
            from: old_status,
            to: new_status,
            changed_at: chrono::Utc::now(),
            note,
        });
        self.status = new_status;
        self.updated_at = chrono::Utc::now();
        Ok(())
    }
}

/// A scan run record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanRun {
    pub id: String,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub target: String,
    pub findings_count: usize,
    pub new_findings_count: usize,
    pub resolved_findings_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::findings::*;
    use chrono::Utc;

    fn test_finding(fingerprint: &str) -> Finding {
        Finding {
            id: format!("test-{}", fingerprint),
            fingerprint: fingerprint.to_string(),
            title: "Test Finding".to_string(),
            description: "Test".to_string(),
            severity: crate::types::Severity::Medium,
            confidence: Confidence::High,
            finding_type: FindingType::Vulnerability,
            cwe: None,
            owasp: None,
            cve: None,
            affected_asset: AffectedAsset {
                asset_type: "web_application".to_string(),
                identifier: "https://example.com".to_string(),
                host: Some("example.com".to_string()),
                port: Some(443),
                protocol: Some("https".to_string()),
            },
            location: FindingLocation {
                url: None,
                path: None,
                parameter: None,
                header: None,
                method: None,
                line: None,
                file: None,
            },
            evidence: vec![],
            reproduction: None,
            remediation: None,
            discovered_at: Utc::now(),
            source: FindingSource {
                tool: "test".to_string(),
                module: "test".to_string(),
                run_id: None,
            },
            tags: vec![],
            metadata: serde_json::Value::Null,
        }
    }

    #[test]
    fn stored_finding_starts_as_new() {
        let stored = StoredFinding::new(test_finding("fp1"), "scan-1");
        assert_eq!(stored.status, FindingStatus::New);
        assert!(stored.status_history.is_empty());
        assert_eq!(stored.scan_id, "scan-1");
    }

    #[test]
    fn change_status_records_history() {
        let mut stored = StoredFinding::new(test_finding("fp1"), "scan-1");
        stored
            .change_status(FindingStatus::Confirmed, Some("Looks real".to_string()))
            .unwrap();

        assert_eq!(stored.status, FindingStatus::Confirmed);
        assert_eq!(stored.status_history.len(), 1);
        assert_eq!(stored.status_history[0].from, FindingStatus::New);
        assert_eq!(stored.status_history[0].to, FindingStatus::Confirmed);
        assert_eq!(stored.status_history[0].note.as_deref(), Some("Looks real"));
    }

    #[test]
    fn invalid_transition_is_rejected() {
        let mut stored = StoredFinding::new(test_finding("fp1"), "scan-1");
        let result = stored.change_status(FindingStatus::Remediated, None);
        assert!(result.is_err());
        assert_eq!(stored.status, FindingStatus::New);
    }

    #[test]
    fn valid_transitions_from_new() {
        assert!(FindingStatus::New.can_transition_to(&FindingStatus::Confirmed));
        assert!(FindingStatus::New.can_transition_to(&FindingStatus::FalsePositive));
        assert!(FindingStatus::New.can_transition_to(&FindingStatus::AcceptedRisk));
        assert!(!FindingStatus::New.can_transition_to(&FindingStatus::Remediated));
        assert!(!FindingStatus::New.can_transition_to(&FindingStatus::Reopened));
    }

    #[test]
    fn finding_status_display() {
        assert_eq!(FindingStatus::New.to_string(), "new");
        assert_eq!(FindingStatus::Confirmed.to_string(), "confirmed");
        assert_eq!(FindingStatus::AcceptedRisk.to_string(), "accepted_risk");
        assert_eq!(FindingStatus::FalsePositive.to_string(), "false_positive");
        assert_eq!(FindingStatus::Remediated.to_string(), "remediated");
        assert_eq!(FindingStatus::Reopened.to_string(), "reopened");
    }

    #[test]
    fn stored_finding_serializes() {
        let stored = StoredFinding::new(test_finding("fp1"), "scan-1");
        let json = serde_json::to_string(&stored).unwrap();
        assert!(json.contains("fp1"));
        assert!(json.contains("scan-1"));
        assert!(json.contains("new"));
    }

    #[test]
    fn scan_run_serializes() {
        let run = ScanRun {
            id: "run-1".to_string(),
            started_at: Utc::now(),
            completed_at: Some(Utc::now()),
            target: "https://example.com".to_string(),
            findings_count: 5,
            new_findings_count: 3,
            resolved_findings_count: 1,
        };
        let json = serde_json::to_string(&run).unwrap();
        assert!(json.contains("run-1"));
    }
}
