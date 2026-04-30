use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub use crate::tool::finding::{Finding, FindingType, ResponseSeverity};
pub use crate::tool::tool_error::{ToolError, ToolErrorType};

/// Response returned by a tool after execution.
///
/// Contains the request ID, tool ID, status, results, metadata,
/// any errors encountered, and findings discovered.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResponse {
    /// Unique identifier for the original request
    pub request_id: String,
    /// Identifier of the tool that generated this response
    pub tool_id: String,
    /// Overall status of the execution
    pub status: ResponseStatus,
    /// Tool-specific results as JSON
    pub results: serde_json::Value,
    /// Execution metadata (timing, counts, etc.)
    pub metadata: ResponseMetadata,
    /// Errors encountered during execution
    pub errors: Vec<ToolError>,
    /// Security findings discovered during execution
    pub findings: Vec<Finding>,
}

impl ToolResponse {
    /// Creates a successful response with the given results.
    ///
    /// # Arguments
    ///
    /// * `request_id` - The request ID from the original ToolRequest
    /// * `tool_id` - The tool identifier
    /// * `results` - The tool-specific results as JSON
    ///
    /// # Example
    ///
    /// ```rust
    /// use slapper::tool::response::ToolResponse;
    ///
    /// let response = ToolResponse::success(
    ///     "req-123",
    ///     "scanner",
    ///     serde_json::json!({"ports": [80, 443]})
    /// );
    /// ```
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

    /// Adds execution metadata to the response.
    ///
    /// # Arguments
    ///
    /// * `metadata` - The response metadata to attach
    pub fn with_metadata(mut self, metadata: ResponseMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    /// Adds findings to the response.
    ///
    /// Also updates the `findings_count` in metadata.
    ///
    /// # Arguments
    ///
    /// * `findings` - List of findings to attach
    pub fn with_findings(mut self, findings: Vec<Finding>) -> Self {
        self.findings = findings;
        self.metadata.findings_count = self.findings.len();
        self
    }

    /// Adds errors to the response.
    ///
    /// If errors are added and the status is currently `Success`,
    /// the status is changed to `PartialSuccess`.
    ///
    /// # Arguments
    ///
    /// * `errors` - List of errors to attach
    pub fn with_errors(mut self, errors: Vec<ToolError>) -> Self {
        self.errors = errors.clone();
        if !errors.is_empty() && self.status == ResponseStatus::Success {
            self.status = ResponseStatus::PartialSuccess;
        }
        self
    }

    /// Returns `true` if the status indicates success.
    ///
    /// This is true for both `Success` and `PartialSuccess` statuses.
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

    pub fn to_json_line(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self).map(|s| s + "\n")
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fuzzer::engine::FuzzResult;
    use crate::fuzzer::payloads::{Payload, PayloadType};
    use crate::types::Severity;

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
        assert!(finding.location.contains("SQL Injection"));
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
