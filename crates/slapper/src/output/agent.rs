use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub use crate::types::Severity;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    Confirmed,
    Likely,
    Possible,
    Unlikely,
}

impl Confidence {
    pub fn score(&self) -> f32 {
        match self {
            Confidence::Confirmed => 1.0,
            Confidence::Likely => 0.75,
            Confidence::Possible => 0.5,
            Confidence::Unlikely => 0.25,
        }
    }

    pub fn from_ratio(found: usize, tested: usize) -> Self {
        if tested == 0 {
            return Confidence::Possible;
        }

        let ratio = found as f32 / tested as f32;
        match ratio {
            r if r >= 0.9 => Confidence::Confirmed,
            r if r >= 0.6 => Confidence::Likely,
            r if r >= 0.3 => Confidence::Possible,
            _ => Confidence::Unlikely,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttackSurface {
    Web,
    Api,
    Network,
    Authentication,
    Session,
    FileSystem,
    Internal,
    Cloud,
    Cdn,
}

impl AttackSurface {
    pub fn display_name(&self) -> &'static str {
        match self {
            AttackSurface::Web => "Web Application",
            AttackSurface::Api => "API Endpoint",
            AttackSurface::Network => "Network Service",
            AttackSurface::Authentication => "Authentication System",
            AttackSurface::Session => "Session Management",
            AttackSurface::FileSystem => "File System",
            AttackSurface::Internal => "Internal Service",
            AttackSurface::Cloud => "Cloud Infrastructure",
            AttackSurface::Cdn => "CDN/Proxy",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentFinding {
    pub id: String,
    pub tool_id: String,
    pub vulnerability_type: String,
    pub severity: Severity,
    pub title: String,
    pub description: String,
    pub evidence: Evidence,
    pub remediation: Remediation,
    pub confidence: Confidence,
    pub cvss: Option<f32>,
    pub cwe_ids: Vec<String>,
    pub target: String,
    pub endpoint: String,
    pub parameter: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub attack_surface: AttackSurface,
    pub status: FindingStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingStatus {
    New,
    Confirmed,
    FalsePositive,
    Ignored,
    Remediated,
}

impl AgentFinding {
    pub fn new(
        vulnerability_type: impl Into<String>,
        severity: Severity,
        title: impl Into<String>,
        target: impl Into<String>,
        endpoint: impl Into<String>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            tool_id: String::new(),
            vulnerability_type: vulnerability_type.into(),
            severity,
            title: title.into(),
            description: String::new(),
            evidence: Evidence::default(),
            remediation: Remediation::default(),
            confidence: Confidence::Possible,
            cvss: None,
            cwe_ids: Vec::new(),
            target: target.into(),
            endpoint: endpoint.into(),
            parameter: None,
            timestamp: Utc::now(),
            attack_surface: AttackSurface::Web,
            status: FindingStatus::New,
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    pub fn with_evidence(mut self, evidence: Evidence) -> Self {
        self.evidence = evidence;
        self
    }

    pub fn with_remediation(mut self, remediation: Remediation) -> Self {
        self.remediation = remediation;
        self
    }

    pub fn with_cvss(mut self, cvss: f32) -> Self {
        self.cvss = Some(cvss);
        if self.cvss.is_some() {
            self.severity = Severity::from_cvss(cvss);
        }
        self
    }

    pub fn with_cwe(mut self, cwe: impl Into<String>) -> Self {
        self.cwe_ids.push(cwe.into());
        self
    }

    pub fn with_parameter(mut self, param: impl Into<String>) -> Self {
        self.parameter = Some(param.into());
        self
    }

    pub fn with_confidence(mut self, confidence: Confidence) -> Self {
        self.confidence = confidence;
        self
    }

    pub fn with_attack_surface(mut self, surface: AttackSurface) -> Self {
        self.attack_surface = surface;
        self
    }

    pub fn with_tool_id(mut self, tool_id: impl Into<String>) -> Self {
        self.tool_id = tool_id.into();
        self
    }

    pub fn short_summary(&self) -> String {
        format!(
            "[{}] {} - {} ({})",
            self.severity, self.title, self.target, self.endpoint
        )
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Evidence {
    pub request: Option<String>,
    pub response_snippet: Option<String>,
    pub diff_indicator: Option<String>,
    pub matched_pattern: Option<String>,
    pub timing_ms: Option<u64>,
    pub status_code: Option<u16>,
}

impl Evidence {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_request(mut self, request: impl Into<String>) -> Self {
        self.request = Some(request.into());
        self
    }

    pub fn with_response(mut self, response: impl Into<String>) -> Self {
        self.response_snippet = Some(response.into());
        self
    }

    pub fn with_pattern(mut self, pattern: impl Into<String>) -> Self {
        self.matched_pattern = Some(pattern.into());
        self
    }

    pub fn with_timing(mut self, ms: u64) -> Self {
        self.timing_ms = Some(ms);
        self
    }

    pub fn with_status_code(mut self, code: u16) -> Self {
        self.status_code = Some(code);
        self
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Remediation {
    pub summary: String,
    pub references: Vec<String>,
    pub code_example: Option<String>,
    pub priority: u8,
    pub effort: Option<RemediationEffort>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RemediationEffort {
    Low,
    Medium,
    High,
}

impl Remediation {
    pub fn new(summary: impl Into<String>) -> Self {
        Self {
            summary: summary.into(),
            ..Default::default()
        }
    }

    pub fn with_reference(mut self, reference: impl Into<String>) -> Self {
        self.references.push(reference.into());
        self
    }

    pub fn with_code_example(mut self, example: impl Into<String>) -> Self {
        self.code_example = Some(example.into());
        self
    }

    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority.min(5);
        self
    }

    pub fn with_effort(mut self, effort: RemediationEffort) -> Self {
        self.effort = Some(effort);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindingSummary {
    pub total: usize,
    pub by_severity: FxHashMap<Severity, usize>,
    pub by_confidence: FxHashMap<Confidence, usize>,
    pub by_attack_surface: FxHashMap<AttackSurface, usize>,
    pub by_type: FxHashMap<String, usize>,
}

impl FindingSummary {
    pub fn from_findings(findings: &[AgentFinding]) -> Self {
        let mut by_severity = FxHashMap::default();
        let mut by_confidence = FxHashMap::default();
        let mut by_attack_surface = FxHashMap::default();
        let mut by_type = FxHashMap::default();

        for finding in findings {
            *by_severity.entry(finding.severity).or_insert(0) += 1;
            *by_confidence.entry(finding.confidence).or_insert(0) += 1;
            *by_attack_surface.entry(finding.attack_surface).or_insert(0) += 1;
            *by_type
                .entry(finding.vulnerability_type.clone())
                .or_insert(0) += 1;
        }

        Self {
            total: findings.len(),
            by_severity,
            by_confidence,
            by_attack_surface,
            by_type,
        }
    }

    pub fn risk_score(&self) -> f32 {
        let critical = self.by_severity.get(&Severity::Critical).unwrap_or(&0);
        let high = self.by_severity.get(&Severity::High).unwrap_or(&0);
        let medium = self.by_severity.get(&Severity::Medium).unwrap_or(&0);
        let low = self.by_severity.get(&Severity::Low).unwrap_or(&0);

        let weighted = (*critical as f32 * 10.0)
            + (*high as f32 * 7.0)
            + (*medium as f32 * 4.0)
            + (*low as f32 * 1.0);

        (weighted / (self.total.max(1) as f32) * 10.0).min(10.0)
    }
}

use rustc_hash::FxHashMap;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_finding_creation() {
        let finding = AgentFinding::new(
            "sqli",
            Severity::Critical,
            "SQL Injection Found",
            "https://example.com",
            "/api/users",
        )
        .with_cwe("CWE-89")
        .with_cvss(9.8)
        .with_parameter("id");

        assert_eq!(finding.severity, Severity::Critical);
        assert!(finding.cvss.is_some());
    }

    #[test]
    fn test_finding_summary() {
        let findings = vec![
            AgentFinding::new(
                "sqli",
                Severity::Critical,
                "SQLi",
                "https://example.com",
                "/",
            ),
            AgentFinding::new("xss", Severity::High, "XSS", "https://example.com", "/"),
            AgentFinding::new("sqli", Severity::Medium, "SQLi", "https://example.com", "/"),
        ];

        let summary = FindingSummary::from_findings(&findings);
        assert_eq!(summary.total, 3);
        assert_eq!(summary.by_severity[&Severity::Critical], 1);
        assert_eq!(summary.by_type["sqli"], 2);
    }
}
