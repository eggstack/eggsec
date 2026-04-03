use crate::types::Severity;
use crate::vuln::asset::AssetCriticality;
use crate::vuln::exploit::ExploitInfo;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskScore {
    pub cvss_score: f32,
    pub exploitability_score: f32,
    pub asset_criticality: f32,
    pub combined_score: f32,
    pub priority_level: PriorityLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum PriorityLevel {
    P0,
    P1,
    P2,
    P3,
}

impl RiskScore {
    pub fn calculate(cvss: f32, exploitability: f32, asset_criticality: f32) -> Self {
        let combined = cvss * 0.4 + exploitability * 0.3 + asset_criticality * 0.3;

        let priority_level = match cvss {
            s if s >= 9.0 => PriorityLevel::P0,
            s if s >= 7.0 => PriorityLevel::P1,
            s if s >= 4.0 => PriorityLevel::P2,
            _ => PriorityLevel::P3,
        };

        Self {
            cvss_score: cvss,
            exploitability_score: exploitability,
            asset_criticality,
            combined_score: combined,
            priority_level,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrioritizedFinding {
    pub finding_id: String,
    pub title: String,
    pub severity: Severity,
    pub risk_score: RiskScore,
    pub exploit_info: Option<ExploitInfo>,
    pub asset_criticality: Option<AssetCriticality>,
    pub priority_rank: usize,
}

impl PrioritizedFinding {
    pub fn prioritize(findings: Vec<PrioritizedFinding>) -> Vec<PrioritizedFinding> {
        let mut sorted = findings;
        sorted.sort_by(|a, b| {
            b.risk_score
                .combined_score
                .partial_cmp(&a.risk_score.combined_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        for (i, finding) in sorted.iter_mut().enumerate() {
            finding.priority_rank = i + 1;
        }

        sorted
    }
}

pub fn prioritize_findings(
    findings: &[(String, String, Severity, Option<f32>)],
) -> Vec<PrioritizedFinding> {
    let prioritized: Vec<PrioritizedFinding> = findings
        .iter()
        .map(|(id, title, severity, cvss)| {
            let risk_score = RiskScore::calculate(
                cvss.unwrap_or_else(|| match severity {
                    Severity::Critical => 9.0,
                    Severity::High => 7.5,
                    Severity::Medium => 5.0,
                    Severity::Low => 2.5,
                    Severity::Info => 0.1,
                }),
                5.0,
                5.0,
            );

            PrioritizedFinding {
                finding_id: id.clone(),
                title: title.clone(),
                severity: *severity,
                risk_score,
                exploit_info: None,
                asset_criticality: None,
                priority_rank: 0,
            }
        })
        .collect();

    PrioritizedFinding::prioritize(prioritized)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prioritization() {
        let findings = vec![
            (
                "f1".to_string(),
                "Low finding".to_string(),
                Severity::Low,
                Some(2.5),
            ),
            (
                "f2".to_string(),
                "Critical finding".to_string(),
                Severity::Critical,
                Some(9.5),
            ),
            (
                "f3".to_string(),
                "High finding".to_string(),
                Severity::High,
                Some(8.0),
            ),
        ];

        let prioritized = prioritize_findings(&findings);
        assert_eq!(prioritized[0].finding_id, "f2");
        assert_eq!(prioritized[1].finding_id, "f3");
        assert_eq!(prioritized[2].finding_id, "f1");
    }
}
