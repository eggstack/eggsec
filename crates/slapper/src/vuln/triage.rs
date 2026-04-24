use crate::types::Severity;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriageResult {
    pub finding_id: String,
    pub triage_status: TriageStatus,
    pub confidence: f32,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TriageStatus {
    New,
    TruePositive,
    FalsePositive,
    NeedsReview,
    Duplicate,
}

impl TriageResult {
    pub fn new(finding_id: Option<String>, status: TriageStatus) -> Self {
        Self {
            finding_id: finding_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
            triage_status: status,
            confidence: 0.5,
            reason: "Initial triage".to_string(),
        }
    }

    pub fn status(&self) -> &TriageStatus {
        &self.triage_status
    }
}

pub fn triage_finding(
    finding_id: &str,
    title: &str,
    description: &str,
    severity: Severity,
    cvss_score: Option<f32>,
) -> TriageResult {
    let duplicate_keywords = ["test", "example", "demo", "sample", "localhost"];
    let false_positive_keywords = ["information", "low", "informational", "no risk"];

    let title_lower = title.to_lowercase();
    let description_lower = description.to_lowercase();

    let is_duplicate = duplicate_keywords
        .iter()
        .any(|kw| title_lower.contains(kw) || description_lower.contains(kw));

    let is_false_positive = false_positive_keywords
        .iter()
        .any(|kw| title_lower.contains(kw) && severity == Severity::Info);

    let (status, confidence, reason) = if is_duplicate {
        (
            TriageStatus::Duplicate,
            0.95,
            "Finding matches duplicate pattern".to_string(),
        )
    } else if is_false_positive {
        (
            TriageStatus::FalsePositive,
            0.85,
            "Finding matches false positive pattern".to_string(),
        )
    } else if cvss_score.is_some() && cvss_score.unwrap() >= 9.0 {
        (
            TriageStatus::TruePositive,
            0.99,
            "Critical CVSS score confirms true positive".to_string(),
        )
    } else {
        (
            TriageStatus::NeedsReview,
            0.5,
            "Manual review required".to_string(),
        )
    };

    TriageResult {
        finding_id: finding_id.to_string(),
        triage_status: status,
        confidence,
        reason,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_triage_duplicate() {
        let result = triage_finding(
            "f1",
            "Test vulnerability",
            "This is a test",
            Severity::High,
            Some(7.0),
        );
        assert_eq!(result.triage_status, TriageStatus::Duplicate);
    }

    #[test]
    fn test_triage_true_positive() {
        let result = triage_finding(
            "f2",
            "Remote code execution",
            "RCE vulnerability",
            Severity::Critical,
            Some(9.8),
        );
        assert_eq!(result.triage_status, TriageStatus::TruePositive);
    }
}
