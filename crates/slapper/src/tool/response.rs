use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::fuzzer::{FuzzResult, Payload};
use crate::types::Severity;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResponse {
    pub request_id: String,
    pub tool_id: String,
    pub status: ResponseStatus,
    pub results: serde_json::Value,
    pub metadata: ResponseMetadata,
    pub errors: Vec<ToolError>,
    pub findings: Vec<Finding>,
}

impl ToolResponse {
    pub fn success(
        request_id: impl Into<String>,
        tool_id: impl Into<String>,
        results: serde_json::Value,
    ) -> Self {
        let now = Utc::now();
        Self {
            request_id: request_id.into(),
            tool_id: tool_id.into(),
            status: ResponseStatus::Success,
            results,
            metadata: ResponseMetadata {
                started_at: now,
                completed_at: now,
                duration_ms: 0,
                targets_scanned: 0,
                findings_count: 0,
            },
            errors: vec![],
            findings: vec![],
        }
    }

    pub fn with_metadata(mut self, metadata: ResponseMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    pub fn with_findings(mut self, findings: Vec<Finding>) -> Self {
        self.findings = findings;
        self.metadata.findings_count = self.findings.len();
        self
    }

    pub fn with_errors(mut self, errors: Vec<ToolError>) -> Self {
        self.errors = errors.clone();
        if !errors.is_empty() && self.status == ResponseStatus::Success {
            self.status = ResponseStatus::PartialSuccess;
        }
        self
    }

    pub fn is_success(&self) -> bool {
        matches!(
            self.status,
            ResponseStatus::Success | ResponseStatus::PartialSuccess
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResponseStatus {
    Success,
    PartialSuccess,
    Failed,
    Timeout,
    ScopeViolation,
    Cancelled,
}

impl std::fmt::Display for ResponseStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResponseStatus::Success => write!(f, "success"),
            ResponseStatus::PartialSuccess => write!(f, "partial_success"),
            ResponseStatus::Failed => write!(f, "failed"),
            ResponseStatus::Timeout => write!(f, "timeout"),
            ResponseStatus::ScopeViolation => write!(f, "scope_violation"),
            ResponseStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseMetadata {
    pub started_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
    pub duration_ms: u64,
    pub targets_scanned: usize,
    pub findings_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolError {
    pub code: String,
    pub message: String,
    pub details: Option<serde_json::Value>,
    pub target: Option<String>,
    pub recoverable: bool,
    pub error_type: ToolErrorType,
    pub retry_after_ms: Option<u64>,
}

impl ToolError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            details: None,
            target: None,
            recoverable: false,
            error_type: ToolErrorType::Internal,
            retry_after_ms: None,
        }
    }

    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }

    pub fn at_target(mut self, target: impl Into<String>) -> Self {
        self.target = Some(target.into());
        self
    }

    pub fn with_error_type(mut self, error_type: ToolErrorType) -> Self {
        self.error_type = error_type;
        self.recoverable = error_type.is_recoverable();
        self
    }

    pub fn with_retry_after(mut self, ms: u64) -> Self {
        self.retry_after_ms = Some(ms);
        self.recoverable = true;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolErrorType {
    Validation,
    Authentication,
    Authorization,
    RateLimit,
    Network,
    Timeout,
    ScopeViolation,
    NotFound,
    Configuration,
    Internal,
    ToolNotFound,
}

impl ToolErrorType {
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            ToolErrorType::RateLimit
                | ToolErrorType::Timeout
                | ToolErrorType::Network
                | ToolErrorType::Internal
        )
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            ToolErrorType::Validation => "validation",
            ToolErrorType::Authentication => "authentication",
            ToolErrorType::Authorization => "authorization",
            ToolErrorType::RateLimit => "rate_limit",
            ToolErrorType::Network => "network",
            ToolErrorType::Timeout => "timeout",
            ToolErrorType::ScopeViolation => "scope_violation",
            ToolErrorType::NotFound => "not_found",
            ToolErrorType::Configuration => "configuration",
            ToolErrorType::Internal => "internal",
            ToolErrorType::ToolNotFound => "tool_not_found",
        }
    }
}

impl std::fmt::Display for ToolErrorType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub id: String,
    pub finding_type: FindingType,
    pub severity: ResponseSeverity,
    pub title: String,
    pub description: String,
    pub location: String,
    pub evidence: Option<String>,
    pub cve_ids: Vec<String>,
    pub remediation: Option<String>,
    pub references: Vec<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Finding {
    pub fn new(
        finding_type: FindingType,
        severity: ResponseSeverity,
        title: impl Into<String>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            finding_type,
            severity,
            title: title.into(),
            description: String::new(),
            location: String::new(),
            evidence: None,
            cve_ids: vec![],
            remediation: None,
            references: vec![],
            metadata: HashMap::new(),
        }
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    pub fn at_location(mut self, location: impl Into<String>) -> Self {
        self.location = location.into();
        self
    }

    pub fn with_evidence(mut self, evidence: impl Into<String>) -> Self {
        self.evidence = Some(evidence.into());
        self
    }

    pub fn with_cve(mut self, cve: impl Into<String>) -> Self {
        self.cve_ids.push(cve.into());
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FindingType {
    Vulnerability,
    Information,
    Weakness,
    Configuration,
    Misconfiguration,
    SensitiveData,
    Banner,
    Technology,
    Service,
    Endpoint,
    Subdomain,
    OpenPort,
}

impl std::fmt::Display for FindingType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FindingType::Vulnerability => write!(f, "vulnerability"),
            FindingType::Information => write!(f, "information"),
            FindingType::Weakness => write!(f, "weakness"),
            FindingType::Configuration => write!(f, "configuration"),
            FindingType::Misconfiguration => write!(f, "misconfiguration"),
            FindingType::SensitiveData => write!(f, "sensitive_data"),
            FindingType::Banner => write!(f, "banner"),
            FindingType::Technology => write!(f, "technology"),
            FindingType::Service => write!(f, "service"),
            FindingType::Endpoint => write!(f, "endpoint"),
            FindingType::Subdomain => write!(f, "subdomain"),
            FindingType::OpenPort => write!(f, "open_port"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResponseSeverity {
    Critical,
    High,
    Medium,
    Low,
    Info,
    None,
}

impl ResponseSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            ResponseSeverity::Critical => "critical",
            ResponseSeverity::High => "high",
            ResponseSeverity::Medium => "medium",
            ResponseSeverity::Low => "low",
            ResponseSeverity::Info => "info",
            ResponseSeverity::None => "none",
        }
    }

    fn as_int(&self) -> u8 {
        match self {
            ResponseSeverity::Critical => 5,
            ResponseSeverity::High => 4,
            ResponseSeverity::Medium => 3,
            ResponseSeverity::Low => 2,
            ResponseSeverity::Info => 1,
            ResponseSeverity::None => 0,
        }
    }
}

impl Ord for ResponseSeverity {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.as_int().cmp(&other.as_int())
    }
}

impl PartialOrd for ResponseSeverity {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl std::str::FromStr for ResponseSeverity {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "critical" => Ok(ResponseSeverity::Critical),
            "high" => Ok(ResponseSeverity::High),
            "medium" | "moderate" => Ok(ResponseSeverity::Medium),
            "low" => Ok(ResponseSeverity::Low),
            "info" | "informational" => Ok(ResponseSeverity::Info),
            _ => Ok(ResponseSeverity::None),
        }
    }
}

impl std::fmt::Display for ResponseSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortData {
    pub port: u16,
    pub protocol: String,
    pub state: PortState,
    pub service: Option<String>,
    pub version: Option<String>,
    pub banner: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PortState {
    Open,
    Closed,
    Filtered,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointData {
    pub url: String,
    pub status_code: Option<u16>,
    pub content_length: Option<u64>,
    pub content_type: Option<String>,
    pub discovered_at: chrono::DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TechnologyData {
    pub name: String,
    pub version: Option<String>,
    pub category: String,
    pub confidence: f32,
    pub website: Option<String>,
    pub cpe: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressUpdate {
    pub request_id: String,
    pub stage: String,
    pub progress: f32,
    pub message: String,
    pub items_found: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamEvent {
    pub event_type: StreamEventType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress: Option<ProgressUpdate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finding: Option<Finding>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<ToolResponse>,
}

impl StreamEvent {
    pub fn progress(
        request_id: &str,
        stage: &str,
        progress: f32,
        message: &str,
        items_found: usize,
    ) -> Self {
        Self {
            event_type: StreamEventType::Progress,
            request_id: Some(request_id.to_string()),
            progress: Some(ProgressUpdate {
                request_id: request_id.to_string(),
                stage: stage.to_string(),
                progress,
                message: message.to_string(),
                items_found,
            }),
            finding: None,
            result: None,
        }
    }

    pub fn finding(finding: Finding) -> Self {
        Self {
            event_type: StreamEventType::Finding,
            request_id: None,
            progress: None,
            finding: Some(finding),
            result: None,
        }
    }

    pub fn result(response: ToolResponse) -> Self {
        Self {
            event_type: StreamEventType::Result,
            request_id: Some(response.request_id.clone()),
            progress: None,
            finding: None,
            result: Some(response),
        }
    }

    pub fn to_json_line(&self) -> String {
        serde_json::to_string(self).unwrap_or_default() + "\n"
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StreamEventType {
    Progress,
    Finding,
    Result,
    Error,
}

impl std::fmt::Display for StreamEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StreamEventType::Progress => write!(f, "progress"),
            StreamEventType::Finding => write!(f, "finding"),
            StreamEventType::Result => write!(f, "result"),
            StreamEventType::Error => write!(f, "error"),
        }
    }
}

impl From<Severity> for ResponseSeverity {
    fn from(severity: Severity) -> Self {
        match severity {
            Severity::Critical => ResponseSeverity::Critical,
            Severity::High => ResponseSeverity::High,
            Severity::Medium => ResponseSeverity::Medium,
            Severity::Low => ResponseSeverity::Low,
            Severity::Info => ResponseSeverity::Info,
        }
    }
}

impl From<FuzzResult> for Finding {
    fn from(result: FuzzResult) -> Self {
        let severity = ResponseSeverity::from(result.detected_severity);
        let description = if result.leaks_found.is_empty() {
            String::new()
        } else {
            result.leaks_found.join(", ")
        };
        let location = format!(
            "{} - {}",
            result.payload.payload_type, result.payload.payload
        );
        let mut metadata = HashMap::new();
        metadata.insert(
            "status_code".to_string(),
            serde_json::Value::Number(result.status_code.into()),
        );
        metadata.insert(
            "response_time_ms".to_string(),
            serde_json::Value::Number(result.response_time_ms.into()),
        );
        metadata.insert(
            "is_waf_blocked".to_string(),
            serde_json::Value::Bool(result.is_waf_blocked),
        );
        metadata.insert(
            "is_anomaly".to_string(),
            serde_json::Value::Bool(result.is_anomaly),
        );
        metadata.insert(
            "payload".to_string(),
            serde_json::to_value(&result.payload).unwrap_or_default(),
        );

        Finding {
            id: uuid::Uuid::new_v4().to_string(),
            finding_type: FindingType::Vulnerability,
            severity,
            title: result.payload.description,
            description,
            location,
            evidence: None,
            cve_ids: vec![],
            remediation: None,
            references: vec![],
            metadata,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fuzzer::payloads::{Payload, PayloadType};

    fn make_fuzz_result(
        leaks: Vec<String>,
        anomaly: bool,
        waf: bool,
        severity: Severity,
    ) -> FuzzResult {
        FuzzResult {
            payload: Payload {
                payload_type: PayloadType::Sqli,
                payload: "test payload".to_string(),
                description: "SQL injection test".to_string(),
                severity: Severity::Medium,
                tags: vec!["test".to_string()],
            },
            status_code: 200,
            response_time_ms: 150,
            response_length: Some(500),
            is_waf_blocked: waf,
            is_anomaly: anomaly,
            is_redos_suspected: false,
            leaks_found: leaks,
            error: None,
            owasp_category: None,
            detected_severity: severity,
        }
    }

    #[test]
    fn test_fuzz_result_to_finding_with_leaks() {
        let result = make_fuzz_result(
            vec!["SQL injection detected".to_string()],
            false,
            false,
            Severity::High,
        );
        let finding = Finding::from(result);

        assert_eq!(finding.title, "SQL injection test");
        assert_eq!(finding.description, "SQL injection detected");
        assert_eq!(finding.severity, ResponseSeverity::High);
        assert!(finding.location.contains("Sqli"));
        assert!(finding.location.contains("test payload"));
        assert_eq!(
            finding.metadata.get("status_code").and_then(|v| v.as_u64()),
            Some(200)
        );
        assert_eq!(
            finding
                .metadata
                .get("response_time_ms")
                .and_then(|v| v.as_u64()),
            Some(150)
        );
        assert!(!finding
            .metadata
            .get("is_waf_blocked")
            .unwrap()
            .as_bool()
            .unwrap());
        assert!(!finding
            .metadata
            .get("is_anomaly")
            .unwrap()
            .as_bool()
            .unwrap());
    }

    #[test]
    fn test_fuzz_result_to_finding_with_anomaly() {
        let result = make_fuzz_result(vec![], true, false, Severity::Medium);
        let finding = Finding::from(result);

        assert_eq!(finding.title, "SQL injection test");
        assert!(finding.description.is_empty());
        assert_eq!(finding.severity, ResponseSeverity::Medium);
        assert!(finding
            .metadata
            .get("is_anomaly")
            .unwrap()
            .as_bool()
            .unwrap());
    }

    #[test]
    fn test_fuzz_result_to_finding_waf_blocked() {
        let result = make_fuzz_result(vec![], false, true, Severity::Critical);
        let finding = Finding::from(result);

        assert_eq!(finding.severity, ResponseSeverity::Critical);
        assert!(finding
            .metadata
            .get("is_waf_blocked")
            .unwrap()
            .as_bool()
            .unwrap());
    }

    #[test]
    fn test_fuzz_result_to_finding_multiple_leaks() {
        let result = make_fuzz_result(
            vec![
                "leak 1: admin credentials".to_string(),
                "leak 2: session token".to_string(),
            ],
            false,
            false,
            Severity::Critical,
        );
        let finding = Finding::from(result);

        assert!(finding.description.contains("leak 1"));
        assert!(finding.description.contains("leak 2"));
    }

    #[test]
    fn test_fuzz_result_to_finding_severity_mapping() {
        for severity_input in [
            Severity::Critical,
            Severity::High,
            Severity::Medium,
            Severity::Low,
            Severity::Info,
        ] {
            let result = make_fuzz_result(vec![], false, false, severity_input);
            let finding = Finding::from(result);
            let expected = ResponseSeverity::from(severity_input);
            assert_eq!(finding.severity, expected);
        }
    }

    #[test]
    fn test_fuzz_result_to_finding_payload_metadata() {
        let result = make_fuzz_result(vec![], false, false, Severity::Medium);
        let finding = Finding::from(result);

        let payload_meta = finding.metadata.get("payload").unwrap();
        assert!(payload_meta.is_object());
    }

    #[test]
    fn test_severity_to_response_severity() {
        assert_eq!(
            ResponseSeverity::from(Severity::Critical),
            ResponseSeverity::Critical
        );
        assert_eq!(
            ResponseSeverity::from(Severity::High),
            ResponseSeverity::High
        );
        assert_eq!(
            ResponseSeverity::from(Severity::Medium),
            ResponseSeverity::Medium
        );
        assert_eq!(ResponseSeverity::from(Severity::Low), ResponseSeverity::Low);
        assert_eq!(
            ResponseSeverity::from(Severity::Info),
            ResponseSeverity::Info
        );
    }
}
