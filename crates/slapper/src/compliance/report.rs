use crate::compliance::ComplianceReport;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceSummary {
    pub framework: String,
    pub score: f32,
    pub risk_level: RiskLevel,
    pub top_findings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

impl ComplianceReport {
    pub fn summarize(&self) -> ComplianceSummary {
        let risk_level = match self.overall_score {
            s if s >= 90.0 => RiskLevel::Low,
            s if s >= 70.0 => RiskLevel::Medium,
            s if s >= 50.0 => RiskLevel::High,
            _ => RiskLevel::Critical,
        };

        let top_findings: Vec<String> = self
            .findings
            .iter()
            .filter(|f| {
                f.severity == crate::types::Severity::Critical
                    || f.severity == crate::types::Severity::High
            })
            .map(|f| f.requirement_id.clone())
            .take(5)
            .collect();

        ComplianceSummary {
            framework: self.framework.clone(),
            score: self.overall_score,
            risk_level,
            top_findings,
        }
    }

    pub fn to_html(&self) -> String {
        let summary = self.summarize();
        format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <title>{} Compliance Report</title>
    <style>
        body {{ font-family: Arial, sans-serif; margin: 40px; }}
        .score {{ font-size: 48px; color: #{}; }}
        .finding {{ border: 1px solid #ddd; padding: 10px; margin: 10px 0; }}
        .critical {{ border-left: 4px solid #d32f2f; }}
        .high {{ border-left: 4px solid #f57c00; }}
    </style>
</head>
<body>
    <h1>{} Compliance Report</h1>
    <div class="score">{}%</div>
    <p>Risk Level: {:?}</p>
    <h2>Key Findings</h2>
    {}
</body>
</html>"#,
            self.framework,
            match summary.risk_level {
                RiskLevel::Low => "4caf50",
                RiskLevel::Medium => "ff9800",
                RiskLevel::High => "f57c00",
                RiskLevel::Critical => "d32f2f",
            },
            self.framework,
            self.overall_score,
            summary.risk_level,
            self.findings
                .iter()
                .map(|f| format!(
                    r#"<div class="finding {}"><strong>{}</strong>: {}</div>"#,
                    match f.severity {
                        crate::types::Severity::Critical => "critical",
                        crate::types::Severity::High => "high",
                        _ => "medium",
                    },
                    f.requirement_id,
                    f.description
                ))
                .collect::<Vec<_>>()
                .join("\n")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_risk_level_from_score() {
        let summary = crate::compliance::ComplianceReport {
            framework: "Test".to_string(),
            target: "http://test.com".to_string(),
            overall_score: 85.0,
            total_requirements: 10,
            passed: 8,
            failed: 2,
            findings: vec![],
        }
        .summarize();

        assert_eq!(summary.risk_level, RiskLevel::Medium);
    }
}
