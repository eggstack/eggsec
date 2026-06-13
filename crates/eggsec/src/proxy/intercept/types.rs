use crate::error;
use crate::proxy::intercept::correlation::{CorrelationContext, CorrelationReference};
use crate::proxy::intercept::protocols::{GrpcSession, Http2Session, WebSocketSession};
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
    /// Detected protocol for this flow.
    #[serde(default = "default_protocol")]
    pub protocol: String,
}

fn default_protocol() -> String {
    "http1".to_string()
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
    /// Maximum WebSocket messages per session.
    #[serde(default)]
    pub max_ws_messages: Option<u64>,
    /// WebSocket messages captured so far.
    #[serde(default)]
    pub ws_messages_captured: u64,
    /// Maximum HTTP/2 streams per session.
    #[serde(default)]
    pub max_http2_streams: Option<u64>,
    /// HTTP/2 streams captured so far.
    #[serde(default)]
    pub http2_streams_captured: u64,
    /// Maximum gRPC calls per session.
    #[serde(default)]
    pub max_grpc_calls: Option<u64>,
    /// gRPC calls captured so far.
    #[serde(default)]
    pub grpc_calls_captured: u64,
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
    /// Manipulation audit trail from interactive session.
    #[serde(default)]
    pub manipulations: Vec<ManipulationRecord>,
    /// Captured WebSocket sessions.
    #[serde(default)]
    pub ws_sessions: Vec<WebSocketSession>,
    /// Captured HTTP/2 sessions.
    #[serde(default)]
    pub http2_sessions: Vec<Http2Session>,
    /// Captured gRPC sessions.
    #[serde(default)]
    pub grpc_sessions: Vec<GrpcSession>,
    /// Cross-loadout correlation context.
    #[serde(default)]
    pub correlation: Option<CorrelationContext>,
    /// Cross-loadout correlation references for evidence bundles.
    #[serde(default)]
    pub correlation_refs: Vec<CorrelationReference>,
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
            manipulations: Vec::new(),
            ws_sessions: Vec::new(),
            http2_sessions: Vec::new(),
            grpc_sessions: Vec::new(),
            correlation: None,
            correlation_refs: Vec::new(),
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

    /// Add a manipulation record to the session.
    pub fn add_manipulation(&mut self, record: ManipulationRecord) {
        self.manipulations.push(record);
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

/// An immutable record of a request or response manipulation performed during an interactive session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManipulationRecord {
    /// Flow index this manipulation was applied to.
    pub flow_index: u64,
    /// Whether this was a request or response modification.
    pub direction: ProxyFlowDirection,
    /// The field that was modified (e.g. "header:Authorization", "body", "path").
    pub field: String,
    /// Original value before modification (None for additions).
    pub before: Option<String>,
    /// New value after modification.
    pub after: Option<String>,
    /// Human-readable reason for the modification.
    pub reason: String,
    /// Timestamp when the manipulation occurred (RFC 3339).
    pub timestamp: String,
}

/// Action taken on a captured flow during interactive inspection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FlowAction {
    /// Forward the (possibly modified) request to the upstream server.
    Forward,
    /// Drop the request without forwarding.
    Drop,
    /// Replay the original unmodified request.
    Replay,
    /// The flow is paused at a breakpoint, awaiting operator decision.
    Paused,
}

/// A complete interactive intercept session that can be saved/loaded.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterceptSession {
    /// Session metadata.
    pub listen_addr: String,
    pub ca_fingerprint: String,
    pub dry_run: bool,
    pub started_at: String,
    pub ended_at: String,
    pub target: Option<String>,
    /// All captured flows.
    pub flows: Vec<ProxyFlow>,
    /// Manipulations performed during the session.
    pub manipulations: Vec<ManipulationRecord>,
    /// Actions taken on each flow (indexed by flow index).
    pub flow_actions: Vec<FlowActionRecord>,
    /// Budget usage at session end.
    pub budget: BudgetUsage,
}

/// Records the action taken on a specific flow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowActionRecord {
    /// Flow index.
    pub flow_index: u64,
    /// Action taken.
    pub action: FlowAction,
    /// Timestamp of the action.
    pub timestamp: String,
}

impl InterceptSession {
    /// Create a new empty session.
    pub fn new(listen_addr: &str, dry_run: bool) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            listen_addr: listen_addr.to_string(),
            ca_fingerprint: String::new(),
            dry_run,
            started_at: now.clone(),
            ended_at: now,
            target: None,
            flows: Vec::new(),
            manipulations: Vec::new(),
            flow_actions: Vec::new(),
            budget: BudgetUsage::default(),
        }
    }

    /// Add a flow to the session.
    pub fn add_flow(&mut self, flow: ProxyFlow) {
        self.flows.push(flow);
    }

    /// Record a manipulation.
    pub fn record_manipulation(&mut self, record: ManipulationRecord) {
        self.manipulations.push(record);
    }

    /// Record an action on a flow.
    pub fn record_action(&mut self, flow_index: u64, action: FlowAction) {
        self.flow_actions.push(FlowActionRecord {
            flow_index,
            action,
            timestamp: chrono::Utc::now().to_rfc3339(),
        });
    }

    /// Finalize the session.
    pub fn finalize(&mut self) {
        self.ended_at = chrono::Utc::now().to_rfc3339();
    }

    /// Save the session to a JSON file.
    pub fn save_to_file(&self, path: &str) -> error::Result<()> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| error::EggsecError::Proxy(format!("Failed to serialize session: {}", e)))?;
        std::fs::write(path, json)
            .map_err(|e| error::EggsecError::Proxy(format!("Failed to write session file: {}", e)))?;
        Ok(())
    }

    /// Load a session from a JSON file.
    pub fn load_from_file(path: &str) -> error::Result<Self> {
        let json = std::fs::read_to_string(path)
            .map_err(|e| error::EggsecError::Proxy(format!("Failed to read session file: {}", e)))?;
        serde_json::from_str(&json)
            .map_err(|e| error::EggsecError::Proxy(format!("Failed to deserialize session: {}", e)))
    }

    /// Export the session as HAR 1.2 format.
    pub fn to_har(&self) -> HarExport {
        let entries: Vec<HarEntry> = self.flows.iter().map(|flow| {
            HarEntry {
                started_date_time: flow.started_at.clone(),
                time_ms: flow.duration_ms,
                request: HarRequest {
                    method: flow.method.clone(),
                    url: flow.url.clone(),
                    http_version: "HTTP/1.1".to_string(),
                    cookies: Vec::new(),
                    headers: flow.request_headers.iter().map(|(k, v)| HarNameValuePair {
                        name: k.clone(),
                        value: v.clone(),
                    }).collect(),
                    query_string: Vec::new(),
                    headers_size: -1,
                    body_size: flow.request_body_size as i64,
                    comment: None,
                },
                response: HarResponse {
                    status: flow.response_status,
                    status_text: String::new(),
                    http_version: "HTTP/1.1".to_string(),
                    cookies: Vec::new(),
                    headers: flow.response_headers.iter().map(|(k, v)| HarNameValuePair {
                        name: k.clone(),
                        value: v.clone(),
                    }).collect(),
                    content: HarContent {
                        size: flow.response_body_size as i64,
                        mime_type: "application/octet-stream".to_string(),
                        text: flow.response_body.clone(),
                        encoding: None,
                        comment: None,
                    },
                    redirect_url: String::new(),
                    headers_size: -1,
                    body_size: flow.response_body_size as i64,
                    comment: None,
                },
                cache: HarCache { before_request: None, after_request: None },
                timings: HarTimings {
                    send: 0.0,
                    wait: flow.duration_ms as f64,
                    receive: 0.0,
                    comment: None,
                },
                server_ip_address: None,
                connection: None,
                comment: None,
            }
        }).collect();

        HarExport {
            log: HarLog {
                version: "1.2".to_string(),
                creator: HarCreator {
                    name: "eggsec-web-proxy".to_string(),
                    version: env!("CARGO_PKG_VERSION").to_string(),
                    comment: None,
                },
                browsers: Vec::new(),
                entries,
                pages: Vec::new(),
                comment: None,
            },
        }
    }
}

/// HAR 1.2 export structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarExport {
    pub log: HarLog,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarLog {
    pub version: String,
    pub creator: HarCreator,
    #[serde(default)]
    pub browsers: Vec<serde_json::Value>,
    pub entries: Vec<HarEntry>,
    #[serde(default)]
    pub pages: Vec<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarCreator {
    pub name: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarEntry {
    pub started_date_time: String,
    pub time_ms: u64,
    pub request: HarRequest,
    pub response: HarResponse,
    pub cache: HarCache,
    pub timings: HarTimings,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_ip_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarRequest {
    pub method: String,
    pub url: String,
    pub http_version: String,
    #[serde(default)]
    pub cookies: Vec<serde_json::Value>,
    pub headers: Vec<HarNameValuePair>,
    #[serde(default)]
    pub query_string: Vec<serde_json::Value>,
    pub headers_size: i64,
    pub body_size: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarResponse {
    pub status: u16,
    pub status_text: String,
    pub http_version: String,
    #[serde(default)]
    pub cookies: Vec<serde_json::Value>,
    pub headers: Vec<HarNameValuePair>,
    pub content: HarContent,
    pub redirect_url: String,
    pub headers_size: i64,
    pub body_size: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarNameValuePair {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarContent {
    pub size: i64,
    pub mime_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encoding: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HarCache {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before_request: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after_request: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarTimings {
    pub send: f64,
    pub wait: f64,
    pub receive: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
}

/// Configurable flow buffer with LRU eviction for high-volume sessions.
pub struct FlowBuffer {
    flows: Vec<ProxyFlow>,
    max_size: usize,
}

impl FlowBuffer {
    pub fn new(max_size: usize) -> Self {
        Self {
            flows: Vec::with_capacity(max_size.min(10000)),
            max_size,
        }
    }

    pub fn push(&mut self, flow: ProxyFlow) {
        if self.flows.len() >= self.max_size {
            self.flows.remove(0); // LRU eviction: remove oldest
        }
        self.flows.push(flow);
    }

    pub fn flows(&self) -> &[ProxyFlow] {
        &self.flows
    }

    pub fn len(&self) -> usize {
        self.flows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.flows.is_empty()
    }
}

/// Runtime performance telemetry for proxy sessions.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProxyMetrics {
    pub flows_per_second: f64,
    pub rule_eval_time_ms: f64,
    pub memory_usage_bytes: u64,
    pub active_connections: u32,
    pub total_rules_evaluated: u64,
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
            protocol: "http1".to_string(),
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
            protocol: "http1".to_string(),
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

    #[test]
    fn test_manipulation_record_roundtrip() {
        let record = ManipulationRecord {
            flow_index: 1,
            direction: ProxyFlowDirection::Request,
            field: "header:Authorization".to_string(),
            before: Some("Bearer old-token".to_string()),
            after: Some("Bearer new-token".to_string()),
            reason: "Token refresh".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        };
        let json = serde_json::to_string(&record).unwrap();
        let deserialized: ManipulationRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.field, "header:Authorization");
        assert_eq!(deserialized.before, Some("Bearer old-token".to_string()));
    }

    #[test]
    fn test_flow_action_roundtrip() {
        let action = FlowAction::Forward;
        let json = serde_json::to_string(&action).unwrap();
        let deserialized: FlowAction = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, FlowAction::Forward);
    }

    #[test]
    fn test_intercept_session_new() {
        let session = InterceptSession::new("127.0.0.1:8080", true);
        assert!(session.dry_run);
        assert!(session.flows.is_empty());
        assert!(session.manipulations.is_empty());
    }

    #[test]
    fn test_intercept_session_record_manipulation() {
        let mut session = InterceptSession::new("127.0.0.1:8080", false);
        let record = ManipulationRecord {
            flow_index: 0,
            direction: ProxyFlowDirection::Request,
            field: "body".to_string(),
            before: None,
            after: Some("injected".to_string()),
            reason: "SQLi test".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        };
        session.record_manipulation(record);
        assert_eq!(session.manipulations.len(), 1);
    }

    #[test]
    fn test_intercept_session_record_action() {
        let mut session = InterceptSession::new("127.0.0.1:8080", false);
        session.record_action(0, FlowAction::Drop);
        assert_eq!(session.flow_actions.len(), 1);
        assert_eq!(session.flow_actions[0].action, FlowAction::Drop);
    }

    #[test]
    fn test_intercept_session_to_har_empty() {
        let session = InterceptSession::new("127.0.0.1:8080", false);
        let har = session.to_har();
        assert!(har.log.entries.is_empty());
        assert_eq!(har.log.version, "1.2");
    }
}
