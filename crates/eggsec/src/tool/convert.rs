use crate::output::agent::{
    AgentFinding, AttackSurface, Confidence, Evidence, Remediation, Severity as AgentSeverity,
};
use crate::tool::response::{ResponseSeverity, ToolResponse};

/// Extension trait for [`ToolResponse`] to convert findings to agent findings.
pub trait ToolResponseExt {
    fn to_findings(&self) -> Vec<AgentFinding>;
    fn to_single_finding(&self, vuln_type: &str, title: &str, description: &str) -> AgentFinding;
}

impl ToolResponseExt for ToolResponse {
    fn to_findings(&self) -> Vec<AgentFinding> {
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

    fn to_single_finding(&self, vuln_type: &str, title: &str, description: &str) -> AgentFinding {
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
