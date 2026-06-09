use serde::{Deserialize, Serialize};
use slapper_core::types::Severity;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiOutput {
    pub findings: Vec<AiFinding>,
    pub summary: AiSummary,
}

impl AiOutput {
    pub fn from_findings(findings: Vec<AiFinding>) -> Self {
        let total = findings.len();
        let critical = findings
            .iter()
            .filter(|f| f.severity == Severity::Critical)
            .count();
        let high = findings
            .iter()
            .filter(|f| f.severity == Severity::High)
            .count();
        let medium = findings
            .iter()
            .filter(|f| f.severity == Severity::Medium)
            .count();
        let low = findings
            .iter()
            .filter(|f| f.severity == Severity::Low)
            .count();
        let info = findings
            .iter()
            .filter(|f| f.severity == Severity::Info)
            .count();
        let risk_score = if total == 0 {
            0.0
        } else {
            findings
                .iter()
                .map(|f| f.severity.as_int() as f32 * f.confidence)
                .sum::<f32>()
                / total as f32
                * 2.0
        };
        let risk_score = risk_score.min(10.0);

        Self {
            findings,
            summary: AiSummary {
                total_findings: total,
                critical_count: critical,
                high_count: high,
                medium_count: medium,
                low_count: low,
                info_count: info,
                risk_score,
                executive_summary: if critical > 0 {
                    format!(
                        "{} critical finding(s) require immediate attention.",
                        critical
                    )
                } else if high > 0 {
                    format!(
                        "{} high-severity finding(s) should be addressed promptly.",
                        high
                    )
                } else if total > 0 {
                    format!("{} finding(s) identified, no critical issues.", total)
                } else {
                    "No findings to report.".to_string()
                },
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiFinding {
    pub title: String,
    pub severity: Severity,
    pub description: String,
    pub evidence: Vec<AiEvidence>,
    pub remediation: Vec<AiRemediation>,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiEvidence {
    pub source: String,
    pub content: String,
    pub relevance: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiRemediation {
    pub priority: u8,
    pub action: String,
    pub effort: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiSummary {
    pub total_findings: usize,
    pub critical_count: usize,
    pub high_count: usize,
    pub medium_count: usize,
    pub low_count: usize,
    pub info_count: usize,
    pub risk_score: f32,
    pub executive_summary: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_finding(title: &str, severity: Severity, confidence: f32) -> AiFinding {
        AiFinding {
            title: title.to_string(),
            severity,
            description: "test".to_string(),
            evidence: vec![AiEvidence {
                source: "test".to_string(),
                content: "test".to_string(),
                relevance: 1.0,
            }],
            remediation: vec![AiRemediation {
                priority: 1,
                action: "fix".to_string(),
                effort: "low".to_string(),
            }],
            confidence,
        }
    }

    #[test]
    fn test_from_findings_empty() {
        let output = AiOutput::from_findings(vec![]);
        assert_eq!(output.summary.total_findings, 0);
        assert_eq!(output.summary.critical_count, 0);
        assert_eq!(output.summary.high_count, 0);
        assert_eq!(output.summary.medium_count, 0);
        assert_eq!(output.summary.low_count, 0);
        assert_eq!(output.summary.info_count, 0);
        assert_eq!(output.summary.risk_score, 0.0);
        assert!(output.summary.executive_summary.contains("No findings"));
    }

    #[test]
    fn test_from_findings_critical() {
        let findings = vec![make_finding("RCE", Severity::Critical, 0.9)];
        let output = AiOutput::from_findings(findings);
        assert_eq!(output.summary.total_findings, 1);
        assert_eq!(output.summary.critical_count, 1);
        assert!(output.summary.executive_summary.contains("critical"));
    }

    #[test]
    fn test_from_findings_high_no_critical() {
        let findings = vec![
            make_finding("XSS", Severity::High, 0.8),
            make_finding("SQLi", Severity::High, 0.7),
        ];
        let output = AiOutput::from_findings(findings);
        assert_eq!(output.summary.total_findings, 2);
        assert_eq!(output.summary.critical_count, 0);
        assert_eq!(output.summary.high_count, 2);
        assert!(output.summary.executive_summary.contains("high"));
    }

    #[test]
    fn test_from_findings_mixed_severities() {
        let findings = vec![
            make_finding("RCE", Severity::Critical, 0.9),
            make_finding("XSS", Severity::High, 0.8),
            make_finding("Info Leak", Severity::Medium, 0.6),
            make_finding("Header", Severity::Low, 0.5),
            make_finding("Version", Severity::Info, 0.4),
        ];
        let output = AiOutput::from_findings(findings);
        assert_eq!(output.summary.total_findings, 5);
        assert_eq!(output.summary.critical_count, 1);
        assert_eq!(output.summary.high_count, 1);
        assert_eq!(output.summary.medium_count, 1);
        assert_eq!(output.summary.low_count, 1);
        assert_eq!(output.summary.info_count, 1);
    }

    #[test]
    fn test_risk_score_capped_at_10() {
        let findings = vec![
            make_finding("RCE", Severity::Critical, 1.0),
            make_finding("RCE2", Severity::Critical, 1.0),
            make_finding("RCE3", Severity::Critical, 1.0),
        ];
        let output = AiOutput::from_findings(findings);
        assert!(output.summary.risk_score <= 10.0);
    }

    #[test]
    fn test_from_findings_low_severity_only() {
        let findings = vec![make_finding("Info", Severity::Info, 0.5)];
        let output = AiOutput::from_findings(findings);
        assert_eq!(output.summary.critical_count, 0);
        assert_eq!(output.summary.high_count, 0);
        assert_eq!(output.summary.medium_count, 0);
        assert_eq!(output.summary.low_count, 0);
        assert_eq!(output.summary.info_count, 1);
        assert!(output.summary.executive_summary.contains("no critical"));
    }

    #[test]
    fn test_ai_finding_serialization() {
        let finding = make_finding("Test", Severity::High, 0.8);
        let json = serde_json::to_string(&finding).unwrap();
        let deserialized: AiFinding = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.title, "Test");
        assert_eq!(deserialized.severity, Severity::High);
        assert_eq!(deserialized.confidence, 0.8);
    }

    #[test]
    fn test_ai_output_serialization() {
        let findings = vec![make_finding("Test", Severity::Medium, 0.7)];
        let output = AiOutput::from_findings(findings);
        let json = serde_json::to_string(&output).unwrap();
        let deserialized: AiOutput = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.summary.total_findings, 1);
    }

    #[test]
    fn test_risk_score_calculation() {
        let findings = vec![make_finding("A", Severity::Critical, 1.0)];
        let output = AiOutput::from_findings(findings);
        let expected = (Severity::Critical.as_int() as f32 * 1.0) / 1.0 * 2.0;
        let expected = expected.min(10.0);
        assert!((output.summary.risk_score - expected).abs() < f32::EPSILON);
    }
}
