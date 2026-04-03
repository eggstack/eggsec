use crate::output::agent::AgentFinding;
use crate::types::Severity;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffResult {
    pub new_findings: Vec<DiffFinding>,
    pub resolved_findings: Vec<DiffFinding>,
    pub escalated_findings: Vec<DiffFinding>,
    pub deescalated_findings: Vec<DiffFinding>,
    pub unchanged_findings: Vec<DiffFinding>,
    pub summary: DiffSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffFinding {
    pub id: String,
    pub title: String,
    pub severity: Severity,
    pub description: String,
    pub first_seen: String,
    pub last_seen: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffSummary {
    pub total_new: usize,
    pub total_resolved: usize,
    pub total_escalated: usize,
    pub total_deescalated: usize,
    pub net_change: i32,
}

pub struct DiffEngine;

impl DiffEngine {
    pub fn compare(old_findings: &[AgentFinding], new_findings: &[AgentFinding]) -> DiffResult {
        let old_map: HashMap<_, _> = old_findings
            .iter()
            .map(|f| (f.id.clone(), f.clone()))
            .collect();
        let new_map: HashMap<_, _> = new_findings
            .iter()
            .map(|f| (f.id.clone(), f.clone()))
            .collect();

        let old_ids: HashSet<_> = old_map.keys().cloned().collect();
        let new_ids: HashSet<_> = new_map.keys().cloned().collect();

        let new_findings: Vec<DiffFinding> = new_findings
            .iter()
            .filter(|f| !old_ids.contains(&f.id))
            .map(|f| DiffFinding {
                id: f.id.clone(),
                title: f.title.clone(),
                severity: f.severity,
                description: f.description.clone(),
                first_seen: chrono::Utc::now().to_rfc3339(),
                last_seen: chrono::Utc::now().to_rfc3339(),
            })
            .collect();

        let resolved_findings: Vec<DiffFinding> = old_findings
            .iter()
            .filter(|f| !new_ids.contains(&f.id))
            .map(|f| DiffFinding {
                id: f.id.clone(),
                title: f.title.clone(),
                severity: f.severity,
                description: f.description.clone(),
                first_seen: chrono::Utc::now().to_rfc3339(),
                last_seen: chrono::Utc::now().to_rfc3339(),
            })
            .collect();

        let mut escalated_findings = Vec::new();
        let mut deescalated_findings = Vec::new();
        let mut unchanged_findings = Vec::new();

        for (id, new_finding) in &new_map {
            if let Some(old_finding) = old_map.get(id) {
                let severity_change = new_finding.severity.as_int() - old_finding.severity.as_int();
                if severity_change > 0 {
                    escalated_findings.push(DiffFinding {
                        id: new_finding.id.clone(),
                        title: new_finding.title.clone(),
                        severity: new_finding.severity,
                        description: new_finding.description.clone(),
                        first_seen: chrono::Utc::now().to_rfc3339(),
                        last_seen: chrono::Utc::now().to_rfc3339(),
                    });
                } else if severity_change < 0 {
                    deescalated_findings.push(DiffFinding {
                        id: new_finding.id.clone(),
                        title: new_finding.title.clone(),
                        severity: new_finding.severity,
                        description: new_finding.description.clone(),
                        first_seen: chrono::Utc::now().to_rfc3339(),
                        last_seen: chrono::Utc::now().to_rfc3339(),
                    });
                } else {
                    unchanged_findings.push(DiffFinding {
                        id: new_finding.id.clone(),
                        title: new_finding.title.clone(),
                        severity: new_finding.severity,
                        description: new_finding.description.clone(),
                        first_seen: chrono::Utc::now().to_rfc3339(),
                        last_seen: chrono::Utc::now().to_rfc3339(),
                    });
                }
            }
        }

        let total_new = new_findings.len();
        let total_resolved = resolved_findings.len();
        let total_escalated = escalated_findings.len();
        let total_deescalated = deescalated_findings.len();

        DiffResult {
            new_findings,
            resolved_findings,
            escalated_findings,
            deescalated_findings,
            unchanged_findings,
            summary: DiffSummary {
                total_new,
                total_resolved,
                total_escalated,
                total_deescalated,
                net_change: (total_new as i32) - (total_resolved as i32),
            },
        }
    }

    pub fn has_regressions(diff: &DiffResult) -> bool {
        diff.escalated_findings
            .iter()
            .any(|f| f.severity == Severity::Critical)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diff_summary() {
        let summary = DiffSummary {
            total_new: 5,
            total_resolved: 3,
            total_escalated: 1,
            total_deescalated: 2,
            net_change: 2,
        };
        assert_eq!(summary.net_change, 2);
    }
}
