use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Direction of a captured flow relative to the client.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProxyFlowDirection {
    /// Client → upstream proxy
    Request,
    /// Upstream → client
    Response,
}

/// A single captured HTTP/HTTPS request-response flow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyFlow {
    /// Monotonically increasing flow index within the session.
    pub index: u64,
    /// HTTP method (GET, POST, CONNECT, etc.)
    pub method: String,
    /// Full URL or host+path for the request.
    pub url: String,
    /// Host header value.
    pub host: String,
    /// Request path.
    pub path: String,
    /// Request headers (key-value pairs).
    pub request_headers: HashMap<String, String>,
    /// Request body (truncated/redacted).
    pub request_body: Option<String>,
    /// Response status code (0 if not yet received).
    pub response_status: u16,
    /// Response headers.
    pub response_headers: HashMap<String, String>,
    /// Response body (truncated/redacted).
    pub response_body: Option<String>,
    /// Whether this was an HTTPS CONNECT tunnel.
    pub is_https: bool,
    /// Flow duration in milliseconds.
    pub duration_ms: u64,
    /// Request body size in bytes (before truncation).
    pub request_body_size: u64,
    /// Response body size in bytes (before truncation).
    pub response_body_size: u64,
    /// Timestamp when the flow started (RFC 3339).
    pub started_at: String,
    /// Timestamp when the flow completed (RFC 3339).
    pub completed_at: String,
    /// Redaction applied to this flow (if any).
    pub redaction_applied: Option<String>,
}

/// Budget usage tracking for a proxy session.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BudgetUsage {
    /// Maximum number of flows allowed.
    pub max_flows: Option<u64>,
    /// Number of flows captured so far.
    pub flows_captured: u64,
    /// Maximum bytes per flow body.
    pub max_bytes_per_flow: Option<u64>,
    /// Maximum session duration in seconds.
    pub max_duration_secs: Option<u64>,
    /// Elapsed session duration in seconds.
    pub elapsed_secs: u64,
    /// Maximum concurrent connections.
    pub max_concurrent: Option<u32>,
    /// Peak concurrent connections observed.
    pub peak_concurrent: u32,
}

/// Complete session report for an interactive web proxy capture session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebProxySessionReport {
    /// Listen address (e.g. "127.0.0.1:8080").
    pub listen_addr: String,
    /// CA certificate fingerprint (SHA-256 hex).
    pub ca_fingerprint: String,
    /// Whether this was a dry-run session.
    pub dry_run: bool,
    /// Captured flows.
    pub flows: Vec<ProxyFlow>,
    /// Budget usage.
    pub budget: BudgetUsage,
    /// Policy decision record (serialized).
    pub policy_decision: Option<serde_json::Value>,
    /// Actions performed during the session (audit trail).
    pub actions_performed: Vec<String>,
    /// Whether a scope manifest was matched.
    pub manifest_matched: bool,
    /// Session start timestamp (RFC 3339).
    pub started_at: String,
    /// Session end timestamp (RFC 3339).
    pub ended_at: String,
    /// Total session duration in milliseconds.
    pub duration_ms: u64,
    /// Number of HTTPS flows intercepted.
    pub https_intercepted: u64,
    /// Number of HTTP flows logged.
    pub http_logged: u64,
    /// Number of flows blocked by rules.
    pub blocked: u64,
    /// Number of flows where redaction was applied.
    pub redacted: u64,
    /// Error messages encountered during the session.
    pub errors: Vec<String>,
}

impl WebProxySessionReport {
    /// Create a new empty session report.
    pub fn new(listen_addr: &str, dry_run: bool) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            listen_addr: listen_addr.to_string(),
            ca_fingerprint: String::new(),
            dry_run,
            flows: Vec::new(),
            budget: BudgetUsage::default(),
            policy_decision: None,
            actions_performed: Vec::new(),
            manifest_matched: false,
            started_at: now.clone(),
            ended_at: now,
            duration_ms: 0,
            https_intercepted: 0,
            http_logged: 0,
            blocked: 0,
            redacted: 0,
            errors: Vec::new(),
        }
    }

    /// Add a captured flow to the report.
    pub fn add_flow(&mut self, flow: ProxyFlow) {
        if flow.is_https {
            self.https_intercepted += 1;
        } else {
            self.http_logged += 1;
        }
        if flow.redaction_applied.is_some() {
            self.redacted += 1;
        }
        self.flows.push(flow);
    }

    /// Finalize the report with end timestamp and duration.
    pub fn finalize(&mut self) {
        let now = chrono::Utc::now();
        self.ended_at = now.to_rfc3339();
        if let Ok(start) = chrono::DateTime::parse_from_rfc3339(&self.started_at) {
            self.duration_ms = (now - start.with_timezone(&chrono::Utc)).num_milliseconds() as u64;
        }
    }
}

/// Redaction pattern for PII/tokens in captured traffic.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedactionPattern {
    /// Human-readable name for this pattern.
    pub name: String,
    /// Regex pattern to match.
    pub pattern: String,
    /// Replacement string (e.g. "[REDACTED]").
    pub replacement: String,
}

impl Default for RedactionPattern {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            pattern: String::new(),
            replacement: "[REDACTED]".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proxy_flow_roundtrip() {
        let flow = ProxyFlow {
            index: 1,
            method: "GET".to_string(),
            url: "https://example.com/path".to_string(),
            host: "example.com".to_string(),
            path: "/path".to_string(),
            request_headers: HashMap::new(),
            request_body: None,
            response_status: 200,
            response_headers: HashMap::new(),
            response_body: None,
            is_https: true,
            duration_ms: 150,
            request_body_size: 0,
            response_body_size: 1024,
            started_at: chrono::Utc::now().to_rfc3339(),
            completed_at: chrono::Utc::now().to_rfc3339(),
            redaction_applied: None,
        };
        let json = serde_json::to_string(&flow).unwrap();
        let deserialized: ProxyFlow = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.method, "GET");
        assert!(deserialized.is_https);
    }

    #[test]
    fn test_session_report_new() {
        let report = WebProxySessionReport::new("127.0.0.1:8080", true);
        assert!(report.dry_run);
        assert_eq!(report.listen_addr, "127.0.0.1:8080");
        assert!(report.flows.is_empty());
    }

    #[test]
    fn test_session_report_add_flow() {
        let mut report = WebProxySessionReport::new("127.0.0.1:8080", false);
        let flow = ProxyFlow {
            index: 1,
            method: "GET".to_string(),
            url: "https://example.com/".to_string(),
            host: "example.com".to_string(),
            path: "/".to_string(),
            request_headers: HashMap::new(),
            request_body: None,
            response_status: 200,
            response_headers: HashMap::new(),
            response_body: None,
            is_https: true,
            duration_ms: 100,
            request_body_size: 0,
            response_body_size: 512,
            started_at: chrono::Utc::now().to_rfc3339(),
            completed_at: chrono::Utc::now().to_rfc3339(),
            redaction_applied: Some("header".to_string()),
        };
        report.add_flow(flow);
        assert_eq!(report.flows.len(), 1);
        assert_eq!(report.https_intercepted, 1);
        assert_eq!(report.redacted, 1);
    }

    #[test]
    fn test_budget_usage_default() {
        let budget = BudgetUsage::default();
        assert!(budget.max_flows.is_none());
        assert_eq!(budget.flows_captured, 0);
    }

    #[test]
    fn test_session_report_roundtrip() {
        let report = WebProxySessionReport::new("127.0.0.1:9090", true);
        let json = serde_json::to_string(&report).unwrap();
        let deserialized: WebProxySessionReport = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.listen_addr, "127.0.0.1:9090");
        assert!(deserialized.dry_run);
    }
}
