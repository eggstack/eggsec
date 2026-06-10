use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

use crate::tool::finding::{Finding, ResponseSeverity};

/// Top-level report structure for the coding-agent profile output.
///
/// This is the stable, typed schema returned by `build_coding_agent_output()`.
/// The JSON serialization is backwards-compatible with the previous inline
/// `serde_json::Value` implementation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodingAgentFindingReport {
    /// Schema version for forward compatibility
    pub schema_version: String,
    /// Target that was scanned
    pub target: String,
    /// Always "coding-agent"
    pub profile: String,
    /// Unique request identifier
    pub run_id: String,
    /// Run status: "completed" or "failed"
    pub status: String,
    /// Individual findings
    pub findings: Vec<CodingAgentFinding>,
    /// Aggregated summary
    pub summary: CodingAgentSummary,
}

/// A single finding formatted for the coding-agent profile.
///
/// Omits exploit payload dumps by default to keep output safe for
/// embedding in issue trackers and code review comments.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodingAgentFinding {
    /// Stable finding identifier (UUID)
    pub id: String,
    /// Short human-readable title
    pub title: String,
    /// Finding category (e.g., "vulnerability", "open_port", "endpoint")
    pub category: String,
    /// Severity level
    pub severity: String,
    /// Confidence assessment
    pub confidence: String,
    /// What was observed during the scan
    pub observed_behavior: String,
    /// Evidence snippets (no raw exploit payloads)
    pub evidence: Vec<CodingAgentEvidence>,
    /// How this finding relates to merge readiness
    pub patch_relevance: String,
}

/// An evidence snippet attached to a finding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodingAgentEvidence {
    /// Evidence type (e.g., "raw")
    #[serde(rename = "type")]
    pub evidence_type: String,
    /// Evidence content text
    pub content: String,
}

/// Aggregated summary counts for the report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodingAgentSummary {
    /// Total number of findings
    pub total_findings: usize,
    /// Counts per severity level
    pub by_severity: FxHashMap<String, usize>,
}

impl CodingAgentFindingReport {
    /// Determine the patch-relevance label from a severity level.
    pub fn patch_relevance_for_severity(severity: &ResponseSeverity) -> &'static str {
        match severity {
            ResponseSeverity::Critical | ResponseSeverity::High => "blocks_merge",
            ResponseSeverity::Medium => "should_fix",
            ResponseSeverity::Low => "review_manually",
            _ => "informational",
        }
    }

    /// Determine the confidence label from a severity level.
    pub fn confidence_for_severity(severity: &ResponseSeverity) -> &'static str {
        match severity {
            ResponseSeverity::Critical | ResponseSeverity::High => "high",
            ResponseSeverity::Medium => "medium",
            _ => "low",
        }
    }

    /// Build the summary from a slice of [`CodingAgentFinding`]s.
    pub fn build_summary(findings: &[CodingAgentFinding]) -> CodingAgentSummary {
        let mut by_severity: FxHashMap<String, usize> = FxHashMap::default();
        for f in findings {
            *by_severity.entry(f.severity.clone()).or_insert(0) += 1;
        }
        CodingAgentSummary {
            total_findings: findings.len(),
            by_severity,
        }
    }
}

impl CodingAgentFinding {
    /// Convert a tool [`Finding`] into a [`CodingAgentFinding`].
    ///
    /// This intentionally strips raw exploit payloads — only metadata and
    /// text descriptions are kept.
    pub fn from_finding(finding: &Finding) -> Self {
        let evidence = finding
            .evidence
            .as_ref()
            .map(|e| {
                vec![CodingAgentEvidence {
                    evidence_type: "raw".to_string(),
                    content: e.clone(),
                }]
            })
            .unwrap_or_default();

        CodingAgentFinding {
            id: finding.id.clone(),
            title: finding.title.clone(),
            category: format!("{}", finding.finding_type),
            severity: finding.severity.as_str().to_string(),
            confidence: CodingAgentFindingReport::confidence_for_severity(&finding.severity)
                .to_string(),
            observed_behavior: finding.description.clone(),
            evidence,
            patch_relevance: CodingAgentFindingReport::patch_relevance_for_severity(
                &finding.severity,
            )
            .to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tool::finding::{Finding, FindingType, ResponseSeverity};

    fn make_test_finding(severity: ResponseSeverity) -> Finding {
        Finding {
            id: "test-id-001".to_string(),
            finding_type: FindingType::Vulnerability,
            severity,
            title: "Test finding".to_string(),
            description: "Something was observed".to_string(),
            location: "/api/v1/test".to_string(),
            evidence: Some("evidence content".to_string()),
            cve_ids: vec![],
            remediation: None,
            references: vec![],
            metadata: Default::default(),
        }
    }

    #[test]
    fn test_from_finding_maps_fields() {
        let f = make_test_finding(ResponseSeverity::High);
        let caf = CodingAgentFinding::from_finding(&f);

        assert_eq!(caf.id, "test-id-001");
        assert_eq!(caf.severity, "high");
        assert_eq!(caf.confidence, "high");
        assert_eq!(caf.patch_relevance, "blocks_merge");
        assert_eq!(caf.category, "vulnerability");
        assert_eq!(caf.observed_behavior, "Something was observed");
        assert_eq!(caf.evidence.len(), 1);
        assert_eq!(caf.evidence[0].evidence_type, "raw");
        assert_eq!(caf.evidence[0].content, "evidence content");
    }

    #[test]
    fn test_from_finding_no_evidence() {
        let mut f = make_test_finding(ResponseSeverity::Medium);
        f.evidence = None;
        let caf = CodingAgentFinding::from_finding(&f);

        assert!(caf.evidence.is_empty());
        assert_eq!(caf.patch_relevance, "should_fix");
        assert_eq!(caf.confidence, "medium");
    }

    #[test]
    fn test_build_summary() {
        let findings = vec![
            CodingAgentFinding::from_finding(&make_test_finding(ResponseSeverity::High)),
            CodingAgentFinding::from_finding(&make_test_finding(ResponseSeverity::Medium)),
            CodingAgentFinding::from_finding(&make_test_finding(ResponseSeverity::Medium)),
        ];
        let summary = CodingAgentFindingReport::build_summary(&findings);

        assert_eq!(summary.total_findings, 3);
        assert_eq!(summary.by_severity.get("high"), Some(&1));
        assert_eq!(summary.by_severity.get("medium"), Some(&2));
    }

    #[test]
    fn test_serde_roundtrip() {
        let report = CodingAgentFindingReport {
            schema_version: "1.0".to_string(),
            target: "https://example.com".to_string(),
            profile: "coding-agent".to_string(),
            run_id: "req-123".to_string(),
            status: "completed".to_string(),
            findings: vec![CodingAgentFinding::from_finding(&make_test_finding(
                ResponseSeverity::Critical,
            ))],
            summary: CodingAgentSummary {
                total_findings: 1,
                by_severity: {
                    let mut m = FxHashMap::default();
                    m.insert("critical".to_string(), 1);
                    m
                },
            },
        };

        let json = serde_json::to_string(&report).unwrap();
        let de: CodingAgentFindingReport = serde_json::from_str(&json).unwrap();
        assert_eq!(de.schema_version, "1.0");
        assert_eq!(de.findings.len(), 1);
        assert_eq!(de.findings[0].severity, "critical");
        assert_eq!(de.findings[0].patch_relevance, "blocks_merge");
    }

    #[test]
    fn test_json_field_names_match_expected_schema() {
        let report = CodingAgentFindingReport {
            schema_version: "1.0".to_string(),
            target: "https://example.com".to_string(),
            profile: "coding-agent".to_string(),
            run_id: "req-456".to_string(),
            status: "completed".to_string(),
            findings: vec![],
            summary: CodingAgentSummary {
                total_findings: 0,
                by_severity: FxHashMap::default(),
            },
        };

        let json_value = serde_json::to_value(&report).unwrap();
        let obj = json_value.as_object().unwrap();

        assert!(obj.contains_key("schema_version"));
        assert!(obj.contains_key("target"));
        assert!(obj.contains_key("profile"));
        assert!(obj.contains_key("run_id"));
        assert!(obj.contains_key("status"));
        assert!(obj.contains_key("findings"));
        assert!(obj.contains_key("summary"));

        let summary = obj.get("summary").unwrap().as_object().unwrap();
        assert!(summary.contains_key("total_findings"));
        assert!(summary.contains_key("by_severity"));
    }
}
