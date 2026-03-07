use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
}

impl ToolError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            details: None,
            target: None,
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub id: String,
    pub finding_type: FindingType,
    pub severity: Severity,
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
    pub fn new(finding_type: FindingType, severity: Severity, title: impl Into<String>) -> Self {
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
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
    Info,
    None,
}

impl Severity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Severity::Critical => "critical",
            Severity::High => "high",
            Severity::Medium => "medium",
            Severity::Low => "low",
            Severity::Info => "info",
            Severity::None => "none",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "critical" => Severity::Critical,
            "high" => Severity::High,
            "medium" | "moderate" => Severity::Medium,
            "low" => Severity::Low,
            "info" | "informational" => Severity::Info,
            _ => Severity::None,
        }
    }
}

impl std::fmt::Display for Severity {
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
