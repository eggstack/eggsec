use crate::output::agent::{
    AgentFinding, AttackSurface, Confidence, Evidence, Remediation, Severity as AgentSeverity,
};
use crate::tool::response::{ResponseSeverity, ToolResponse};

impl ToolResponse {
    pub fn to_findings(&self) -> Vec<AgentFinding> {
        self.findings
            .iter()
            .map(|f| {
                let evidence = f.evidence.as_deref().unwrap_or("");
                let remediation_text = f.remediation.as_deref().unwrap_or("See documentation");

                AgentFinding::new(
                    format!("{:?}", f.finding_type).to_lowercase(),
                    severity_from_response(&f.severity),
                    &f.title,
                    &self.request_id,
                    &f.location,
                )
                .with_description(&f.description)
                .with_evidence(Evidence::new().with_response(evidence))
                .with_remediation(Remediation::new(remediation_text))
                .with_confidence(Confidence::Possible)
                .with_attack_surface(AttackSurface::Web)
                .with_tool_id(&self.tool_id)
            })
            .collect()
    }

    pub fn to_single_finding(
        &self,
        vuln_type: &str,
        title: &str,
        description: &str,
    ) -> AgentFinding {
        AgentFinding::new(
            vuln_type,
            severity_from_status(&self.status),
            title,
            &self.request_id,
            "N/A",
        )
        .with_description(description)
        .with_evidence(Evidence::new())
        .with_remediation(Remediation::new("See documentation"))
        .with_confidence(confidence_from_status(&self.status))
        .with_tool_id(&self.tool_id)
    }
}

fn severity_from_response(severity: &ResponseSeverity) -> AgentSeverity {
    match severity {
        ResponseSeverity::Critical => AgentSeverity::Critical,
        ResponseSeverity::High => AgentSeverity::High,
        ResponseSeverity::Medium => AgentSeverity::Medium,
        ResponseSeverity::Low => AgentSeverity::Low,
        ResponseSeverity::Info => AgentSeverity::Info,
        ResponseSeverity::None => AgentSeverity::Info,
    }
}

fn severity_from_status(status: &crate::tool::ResponseStatus) -> AgentSeverity {
    match status {
        crate::tool::ResponseStatus::Success => AgentSeverity::Info,
        crate::tool::ResponseStatus::PartialSuccess => AgentSeverity::Medium,
        crate::tool::ResponseStatus::Failed => AgentSeverity::Low,
        crate::tool::ResponseStatus::Timeout => AgentSeverity::Low,
        crate::tool::ResponseStatus::ScopeViolation => AgentSeverity::High,
        crate::tool::ResponseStatus::Cancelled => AgentSeverity::Info,
    }
}

fn confidence_from_status(status: &crate::tool::ResponseStatus) -> Confidence {
    match status {
        crate::tool::ResponseStatus::Success => Confidence::Confirmed,
        crate::tool::ResponseStatus::PartialSuccess => Confidence::Likely,
        crate::tool::ResponseStatus::Failed => Confidence::Unlikely,
        _ => Confidence::Possible,
    }
}

impl AgentFinding {
    pub fn from_scan_result(
        scan_type: &str,
        target: &str,
        endpoint: &str,
        results: &serde_json::Value,
    ) -> Vec<AgentFinding> {
        let mut findings = Vec::new();

        if let Some(items) = results.as_array() {
            for item in items {
                let vuln_type = item
                    .get("type")
                    .and_then(|v| v.as_str())
                    .unwrap_or(scan_type);

                let severity = item
                    .get("severity")
                    .and_then(|v| v.as_str())
                    .map(Self::parse_severity)
                    .unwrap_or(AgentSeverity::Medium);

                let title = item
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Finding");

                let mut finding = AgentFinding::new(vuln_type, severity, title, target, endpoint);

                if let Some(desc) = item.get("description").and_then(|v| v.as_str()) {
                    finding = finding.with_description(desc);
                }

                if let Some(cwe) = item.get("cwe").and_then(|v| v.as_str()) {
                    finding = finding.with_cwe(cwe);
                }

                if let Some(cvss) = item.get("cvss").and_then(|v| v.as_f64()) {
                    finding = finding.with_cvss(cvss as f32);
                }

                if let Some(param) = item.get("parameter").and_then(|v| v.as_str()) {
                    finding = finding.with_parameter(param);
                }

                if let Some(surface) = item.get("attack_surface").and_then(|v| v.as_str()) {
                    if let Some(as_) = Self::parse_attack_surface(surface) {
                        finding = finding.with_attack_surface(as_);
                    }
                }

                finding = finding.with_confidence(Confidence::Possible);

                findings.push(finding);
            }
        }

        findings
    }

    fn parse_severity(s: &str) -> AgentSeverity {
        match s.to_lowercase().as_str() {
            "critical" | "crit" => AgentSeverity::Critical,
            "high" => AgentSeverity::High,
            "medium" | "moderate" => AgentSeverity::Medium,
            "low" => AgentSeverity::Low,
            _ => AgentSeverity::Info,
        }
    }

    fn parse_attack_surface(s: &str) -> Option<AttackSurface> {
        match s.to_lowercase().as_str() {
            "web" | "http" | "https" => Some(AttackSurface::Web),
            "api" | "graphql" | "rest" => Some(AttackSurface::Api),
            "network" | "tcp" | "udp" => Some(AttackSurface::Network),
            "auth" | "authentication" | "login" => Some(AttackSurface::Authentication),
            "session" | "cookie" | "jwt" => Some(AttackSurface::Session),
            "file" | "filesystem" | "path" => Some(AttackSurface::FileSystem),
            "internal" | "ssrf" => Some(AttackSurface::Internal),
            "cloud" | "aws" | "azure" | "gcp" => Some(AttackSurface::Cloud),
            "cdn" | "proxy" | "waf" => Some(AttackSurface::Cdn),
            _ => None,
        }
    }

    pub fn summarize(&self) -> String {
        format!(
            "[{:?}/{:?}] {} on {} at {}",
            self.severity, self.confidence, self.vulnerability_type, self.target, self.endpoint
        )
    }

    pub fn to_sarif_finding(&self) -> serde_json::Value {
        serde_json::json!({
            "id": self.id,
            "name": self.vulnerability_type,
            "shortDescription": {
                "text": self.title
            },
            "fullDescription": {
                "text": self.description
            },
            "severity": match self.severity {
                AgentSeverity::Critical => "error",
                AgentSeverity::High => "error",
                AgentSeverity::Medium => "warning",
                AgentSeverity::Low => "note",
                AgentSeverity::Info => "note",
            },
            "properties": {
                "confidence": format!("{:?}", self.confidence).to_lowercase(),
                "cvss": self.cvss,
                "cwe": self.cwe_ids,
                "attackSurface": format!("{:?}", self.attack_surface).to_lowercase(),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tool::response::{Finding as ResponseFinding, FindingType};
    use crate::tool::ResponseStatus;

    #[test]
    fn test_severity_from_response() {
        assert_eq!(
            severity_from_response(&ResponseSeverity::Critical),
            AgentSeverity::Critical
        );
        assert_eq!(
            severity_from_response(&ResponseSeverity::High),
            AgentSeverity::High
        );
        assert_eq!(
            severity_from_response(&ResponseSeverity::Medium),
            AgentSeverity::Medium
        );
        assert_eq!(
            severity_from_response(&ResponseSeverity::Low),
            AgentSeverity::Low
        );
        assert_eq!(
            severity_from_response(&ResponseSeverity::Info),
            AgentSeverity::Info
        );
    }

    #[test]
    fn test_severity_from_status() {
        assert_eq!(
            severity_from_status(&ResponseStatus::Success),
            AgentSeverity::Info
        );
        assert_eq!(
            severity_from_status(&ResponseStatus::PartialSuccess),
            AgentSeverity::Medium
        );
        assert_eq!(
            severity_from_status(&ResponseStatus::Failed),
            AgentSeverity::Low
        );
        assert_eq!(
            severity_from_status(&ResponseStatus::Timeout),
            AgentSeverity::Low
        );
    }

    #[test]
    fn test_confidence_from_status() {
        assert_eq!(
            confidence_from_status(&ResponseStatus::Success),
            Confidence::Confirmed
        );
        assert_eq!(
            confidence_from_status(&ResponseStatus::PartialSuccess),
            Confidence::Likely
        );
        assert_eq!(
            confidence_from_status(&ResponseStatus::Failed),
            Confidence::Unlikely
        );
    }

    #[test]
    fn test_parse_severity() {
        assert_eq!(
            AgentFinding::parse_severity("critical"),
            AgentSeverity::Critical
        );
        assert_eq!(AgentFinding::parse_severity("HIGH"), AgentSeverity::High);
        assert_eq!(
            AgentFinding::parse_severity("medium"),
            AgentSeverity::Medium
        );
        assert_eq!(AgentFinding::parse_severity("low"), AgentSeverity::Low);
        assert_eq!(AgentFinding::parse_severity("unknown"), AgentSeverity::Info);
    }

    #[test]
    fn test_parse_attack_surface() {
        assert_eq!(
            AgentFinding::parse_attack_surface("web"),
            Some(AttackSurface::Web)
        );
        assert_eq!(
            AgentFinding::parse_attack_surface("API"),
            Some(AttackSurface::Api)
        );
        assert_eq!(
            AgentFinding::parse_attack_surface("auth"),
            Some(AttackSurface::Authentication)
        );
        assert_eq!(AgentFinding::parse_attack_surface("unknown"), None);
    }

    #[test]
    fn test_from_scan_result() {
        let results = serde_json::json!([
            {
                "type": "sqli",
                "severity": "critical",
                "title": "SQL Injection Found",
                "description": "Blind SQL injection in user_id parameter",
                "cwe": "CWE-89",
                "cvss": 9.8,
                "parameter": "user_id"
            },
            {
                "type": "xss",
                "severity": "high",
                "title": "Reflected XSS",
                "description": "XSS in search parameter"
            }
        ]);

        let findings =
            AgentFinding::from_scan_result("fuzz", "https://example.com", "/api/users", &results);

        assert_eq!(findings.len(), 2);
        assert_eq!(findings[0].severity, AgentSeverity::Critical);
        assert_eq!(findings[0].vulnerability_type, "sqli");
        assert_eq!(findings[0].cwe_ids, vec!["CWE-89"]);
    }

    #[test]
    fn test_finding_to_agents() {
        let response = ToolResponse {
            request_id: "test-123".to_string(),
            tool_id: "recon".to_string(),
            status: ResponseStatus::Success,
            results: serde_json::json!({}),
            metadata: crate::tool::ResponseMetadata {
                started_at: chrono::Utc::now(),
                completed_at: chrono::Utc::now(),
                duration_ms: 100,
                targets_scanned: 1,
                findings_count: 1,
            },
            errors: vec![],
            findings: vec![ResponseFinding {
                id: "finding-1".to_string(),
                finding_type: FindingType::Vulnerability,
                severity: ResponseSeverity::High,
                title: "SQL Injection".to_string(),
                description: "SQL injection found".to_string(),
                location: "/api/user?id=1".to_string(),
                evidence: Some("Payload: ' OR 1=1--".to_string()),
                cve_ids: vec![],
                remediation: Some("Use parameterized queries".to_string()),
                references: vec![],
                metadata: rustc_hash::FxHashMap::default(),
            }],
        };

        let agents = response.to_findings();
        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0].vulnerability_type, "vulnerability");
        assert_eq!(agents[0].severity, AgentSeverity::High);
    }
}
