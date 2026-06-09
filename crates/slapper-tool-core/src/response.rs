use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub use crate::finding::{Finding, FindingType, ResponseSeverity};
pub use crate::tool_error::{ToolError, ToolErrorType};

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
    /// use slapper_tool_core::response::ToolResponse;
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

    #[test]
    fn test_tool_response_success() {
        let response = ToolResponse::success(
            "req-123",
            "scanner",
            serde_json::json!({"ports": [80, 443]}),
        );
        assert!(response.is_success());
        assert_eq!(response.request_id, "req-123");
        assert_eq!(response.tool_id, "scanner");
    }

    #[test]
    fn test_tool_response_with_findings() {
        let finding = Finding::new(
            FindingType::OpenPort,
            ResponseSeverity::Info,
            "Open port 80",
        );
        let response = ToolResponse::success("req-1", "scanner", serde_json::json!({}))
            .with_findings(vec![finding]);
        assert_eq!(response.metadata.findings_count, 1);
    }

    #[test]
    fn test_tool_response_with_errors() {
        let error = ToolError::new("E001", "test error");
        let response = ToolResponse::success("req-1", "scanner", serde_json::json!({}))
            .with_errors(vec![error]);
        assert_eq!(response.status, ResponseStatus::PartialSuccess);
    }

    #[test]
    fn test_response_status_display() {
        assert_eq!(ResponseStatus::Success.to_string(), "success");
        assert_eq!(ResponseStatus::Failed.to_string(), "failed");
        assert_eq!(ResponseStatus::Timeout.to_string(), "timeout");
    }
}
