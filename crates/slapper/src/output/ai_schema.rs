use crate::types::Severity;
use serde::{Deserialize, Serialize};

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
    pub risk_score: f32,
    pub executive_summary: String,
}
