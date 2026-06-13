//! Protocol-specific types for WebSocket, HTTP/2, and gRPC interception.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Supported proxy protocol types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProxyProtocol {
    Http1,
    Http2,
    WebSocket,
    Grpc,
}

/// WebSocket opcode types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WebSocketOpcode {
    /// Continuation frame
    Continuation,
    /// Text frame
    Text,
    /// Binary frame
    Binary,
    /// Connection close
    Close,
    /// Ping
    Ping,
    /// Pong
    Pong,
}

impl WebSocketOpcode {
    pub fn from_byte(b: u8) -> Option<Self> {
        match b & 0x0F {
            0x0 => Some(Self::Continuation),
            0x1 => Some(Self::Text),
            0x2 => Some(Self::Binary),
            0x8 => Some(Self::Close),
            0x9 => Some(Self::Ping),
            0xA => Some(Self::Pong),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Continuation => "continuation",
            Self::Text => "text",
            Self::Binary => "binary",
            Self::Close => "close",
            Self::Ping => "ping",
            Self::Pong => "pong",
        }
    }
}

/// A single WebSocket message captured during interception.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketMessage {
    /// Direction of the message.
    pub direction: super::types::ProxyFlowDirection,
    /// WebSocket opcode.
    pub opcode: WebSocketOpcode,
    /// Message payload (text content for text frames, hex for binary).
    pub payload: String,
    /// Whether the payload was masked (client→server messages are masked per RFC 6455).
    pub masked: bool,
    /// Original payload size in bytes.
    pub payload_size: u64,
    /// Timestamp when the message was captured (RFC 3339).
    pub timestamp: String,
    /// Manipulation applied to this message (if any).
    pub manipulation: Option<WebSocketManipulation>,
}

/// Record of a WebSocket message manipulation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketManipulation {
    /// Original payload before modification.
    pub original_payload: String,
    /// New payload after modification.
    pub new_payload: String,
    /// Reason for the modification.
    pub reason: String,
    /// Timestamp of the manipulation.
    pub timestamp: String,
}

/// A complete WebSocket session with all captured messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketSession {
    /// The HTTP/1.1 upgrade request URL.
    pub url: String,
    /// Host header from the upgrade request.
    pub host: String,
    /// Path from the upgrade request.
    pub path: String,
    /// Whether the connection was over TLS.
    pub is_secure: bool,
    /// All captured messages in order.
    pub messages: Vec<WebSocketMessage>,
    /// Connection opened timestamp (RFC 3339).
    pub opened_at: String,
    /// Connection closed timestamp (RFC 3339), None if still open.
    pub closed_at: Option<String>,
    /// Close code (if a close frame was received).
    pub close_code: Option<u16>,
    /// Close reason (if provided in the close frame).
    pub close_reason: Option<String>,
    /// Total messages sent by client.
    pub client_message_count: u64,
    /// Total messages sent by server.
    pub server_message_count: u64,
    /// Total bytes transferred (client + server).
    pub total_bytes: u64,
}

impl WebSocketSession {
    pub fn new(url: &str, host: &str, path: &str, is_secure: bool) -> Self {
        Self {
            url: url.to_string(),
            host: host.to_string(),
            path: path.to_string(),
            is_secure,
            messages: Vec::new(),
            opened_at: chrono::Utc::now().to_rfc3339(),
            closed_at: None,
            close_code: None,
            close_reason: None,
            client_message_count: 0,
            server_message_count: 0,
            total_bytes: 0,
        }
    }

    pub fn add_message(&mut self, msg: WebSocketMessage) {
        use super::types::ProxyFlowDirection;
        match msg.direction {
            ProxyFlowDirection::Request => self.client_message_count += 1,
            ProxyFlowDirection::Response => self.server_message_count += 1,
        }
        self.total_bytes += msg.payload_size;
        self.messages.push(msg);
    }

    pub fn close(&mut self, code: Option<u16>, reason: Option<String>) {
        self.closed_at = Some(chrono::Utc::now().to_rfc3339());
        self.close_code = code;
        self.close_reason = reason;
    }
}

/// HTTP/2 stream state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Http2StreamState {
    Idle,
    Open,
    HalfClosedLocal,
    HalfClosedRemote,
    Closed,
}

/// A single HTTP/2 stream captured during interception.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Http2Stream {
    /// Stream ID (odd for client-initiated, even for server-initiated).
    pub stream_id: u32,
    /// Stream state.
    pub state: Http2StreamState,
    /// HTTP method.
    pub method: String,
    /// Request path.
    pub path: String,
    /// Request headers.
    pub request_headers: HashMap<String, String>,
    /// Request body.
    pub request_body: Option<String>,
    /// Response status code.
    pub response_status: u16,
    /// Response headers.
    pub response_headers: HashMap<String, String>,
    /// Response body.
    pub response_body: Option<String>,
    /// Request body size in bytes.
    pub request_body_size: u64,
    /// Response body size in bytes.
    pub response_body_size: u64,
    /// Stream priority (if set).
    pub priority: Option<u32>,
    /// Timestamp when the stream was opened.
    pub opened_at: String,
    /// Timestamp when the stream was closed.
    pub closed_at: Option<String>,
}

impl Http2Stream {
    pub fn new(stream_id: u32, method: &str, path: &str) -> Self {
        Self {
            stream_id,
            state: Http2StreamState::Open,
            method: method.to_string(),
            path: path.to_string(),
            request_headers: HashMap::new(),
            request_body: None,
            response_status: 0,
            response_headers: HashMap::new(),
            response_body: None,
            request_body_size: 0,
            response_body_size: 0,
            priority: None,
            opened_at: chrono::Utc::now().to_rfc3339(),
            closed_at: None,
        }
    }
}

/// An HTTP/2 connection with all its multiplexed streams.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Http2Session {
    /// Target host.
    pub host: String,
    /// Whether the connection is over TLS.
    pub is_secure: bool,
    /// All streams in this connection.
    pub streams: Vec<Http2Stream>,
    /// Connection opened timestamp.
    pub opened_at: String,
    /// Connection closed timestamp.
    pub closed_at: Option<String>,
    /// SETTINGS frame parameters received.
    pub settings: HashMap<String, String>,
}

impl Http2Session {
    pub fn new(host: &str, is_secure: bool) -> Self {
        Self {
            host: host.to_string(),
            is_secure,
            streams: Vec::new(),
            opened_at: chrono::Utc::now().to_rfc3339(),
            closed_at: None,
            settings: HashMap::new(),
        }
    }

    pub fn add_stream(&mut self, stream: Http2Stream) {
        self.streams.push(stream);
    }
}

/// gRPC method type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GrpcMethodType {
    Unary,
    ServerStreaming,
    ClientStreaming,
    Bidirectional,
}

/// A gRPC call captured during interception.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrpcCall {
    /// The full service path (e.g. "/package.ServiceName/MethodName").
    pub path: String,
    /// gRPC method type.
    pub method_type: GrpcMethodType,
    /// Request metadata (headers).
    pub request_metadata: HashMap<String, String>,
    /// Request body (protobuf bytes as hex, or JSON if transcoded).
    pub request_body: Option<String>,
    /// Response status code (gRPC status).
    pub response_status: u32,
    /// Response status message.
    pub response_message: Option<String>,
    /// Response metadata.
    pub response_metadata: HashMap<String, String>,
    /// Response body (protobuf bytes as hex, or JSON if transcoded).
    pub response_body: Option<String>,
    /// Request size in bytes.
    pub request_size: u64,
    /// Response size in bytes.
    pub response_size: u64,
    /// Timestamp when the call started.
    pub started_at: String,
    /// Timestamp when the call completed.
    pub completed_at: String,
    /// Duration in milliseconds.
    pub duration_ms: u64,
}

impl GrpcCall {
    pub fn new(path: &str, method_type: GrpcMethodType) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            path: path.to_string(),
            method_type,
            request_metadata: HashMap::new(),
            request_body: None,
            response_status: 0,
            response_message: None,
            response_metadata: HashMap::new(),
            response_body: None,
            request_size: 0,
            response_size: 0,
            started_at: now.clone(),
            completed_at: now,
            duration_ms: 0,
        }
    }
}

/// A complete gRPC session with all captured calls.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrpcSession {
    /// Target host.
    pub host: String,
    /// Whether the connection is over TLS.
    pub is_secure: bool,
    /// All captured calls.
    pub calls: Vec<GrpcCall>,
    /// Session opened timestamp.
    pub opened_at: String,
    /// Session closed timestamp.
    pub closed_at: Option<String>,
}

impl GrpcSession {
    pub fn new(host: &str, is_secure: bool) -> Self {
        Self {
            host: host.to_string(),
            is_secure,
            calls: Vec::new(),
            opened_at: chrono::Utc::now().to_rfc3339(),
            closed_at: None,
        }
    }

    pub fn add_call(&mut self, call: GrpcCall) {
        self.calls.push(call);
    }
}

/// Protocol detection result from analyzing initial bytes/headers.
#[derive(Debug, Clone)]
pub struct ProtocolDetection {
    /// Detected protocol.
    pub protocol: ProxyProtocol,
    /// Confidence level (0.0 - 1.0).
    pub confidence: f64,
    /// Reason for detection.
    pub reason: String,
}

/// Detect protocol from the initial HTTP request headers.
pub fn detect_protocol(_method: &str, _path: &str, headers: &HashMap<String, String>) -> ProtocolDetection {
    // Check for WebSocket upgrade
    if let Some(upgrade) = headers.get("upgrade") {
        if upgrade.eq_ignore_ascii_case("websocket") {
            return ProtocolDetection {
                protocol: ProxyProtocol::WebSocket,
                confidence: 0.99,
                reason: "HTTP Upgrade: websocket header present".to_string(),
            };
        }
    }

    // Check for gRPC (uses HTTP/2 with application/grpc content type)
    if let Some(content_type) = headers.get("content-type") {
        if content_type.starts_with("application/grpc") {
            return ProtocolDetection {
                protocol: ProxyProtocol::Grpc,
                confidence: 0.95,
                reason: format!("Content-Type: {}", content_type),
            };
        }
    }

    // Check for HTTP/2 via the :method pseudo-header or protocol hint
    if let Some(protocol) = headers.get(":scheme") {
        if protocol == "https" || protocol == "http" {
            // The presence of :scheme pseudo-header indicates HTTP/2
            return ProtocolDetection {
                protocol: ProxyProtocol::Http2,
                confidence: 0.90,
                reason: "HTTP/2 pseudo-header :scheme present".to_string(),
            };
        }
    }

    // Default to HTTP/1.1
    ProtocolDetection {
        protocol: ProxyProtocol::Http1,
        confidence: 1.0,
        reason: "Default HTTP/1.1".to_string(),
    }
}

/// Detect gRPC method type from the path and headers.
pub fn detect_grpc_method_type(_path: &str, headers: &HashMap<String, String>) -> GrpcMethodType {
    // Check for streaming indicators in grpc-status or TE header
    let te = headers.get("te").map(|v| v.as_str()).unwrap_or("");
    let grpc_encoding = headers.get("grpc-encoding").map(|v| v.as_str()).unwrap_or("");

    // Check the request content-type for streaming hints
    let is_trailers_only = headers.contains_key("grpc-status");

    if te.contains("trailers") || grpc_encoding.contains("grpc") {
        // Could be server streaming or bidirectional
        // Without deep inspection, assume server streaming
        GrpcMethodType::ServerStreaming
    } else if is_trailers_only && headers.get("content-length").is_none() {
        GrpcMethodType::Unary
    } else {
        GrpcMethodType::Unary
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_websocket_opcode_from_byte() {
        assert_eq!(WebSocketOpcode::from_byte(0x1), Some(WebSocketOpcode::Text));
        assert_eq!(WebSocketOpcode::from_byte(0x2), Some(WebSocketOpcode::Binary));
        assert_eq!(WebSocketOpcode::from_byte(0x8), Some(WebSocketOpcode::Close));
        assert_eq!(WebSocketOpcode::from_byte(0x9), Some(WebSocketOpcode::Ping));
        assert_eq!(WebSocketOpcode::from_byte(0xA), Some(WebSocketOpcode::Pong));
        assert_eq!(WebSocketOpcode::from_byte(0x0), Some(WebSocketOpcode::Continuation));
        assert_eq!(WebSocketOpcode::from_byte(0x5), None);
    }

    #[test]
    fn test_websocket_opcode_as_str() {
        assert_eq!(WebSocketOpcode::Text.as_str(), "text");
        assert_eq!(WebSocketOpcode::Binary.as_str(), "binary");
        assert_eq!(WebSocketOpcode::Close.as_str(), "close");
    }

    #[test]
    fn test_websocket_session_new() {
        let session = WebSocketSession::new("wss://example.com/chat", "example.com", "/chat", true);
        assert_eq!(session.host, "example.com");
        assert!(session.is_secure);
        assert!(session.messages.is_empty());
        assert_eq!(session.client_message_count, 0);
        assert_eq!(session.server_message_count, 0);
        assert_eq!(session.total_bytes, 0);
    }

    #[test]
    fn test_websocket_session_add_message() {
        let mut session = WebSocketSession::new("wss://example.com/chat", "example.com", "/chat", true);
        let msg = WebSocketMessage {
            direction: super::super::types::ProxyFlowDirection::Request,
            opcode: WebSocketOpcode::Text,
            payload: "hello".to_string(),
            masked: true,
            payload_size: 5,
            timestamp: chrono::Utc::now().to_rfc3339(),
            manipulation: None,
        };
        session.add_message(msg);
        assert_eq!(session.client_message_count, 1);
        assert_eq!(session.total_bytes, 5);
        assert_eq!(session.messages.len(), 1);
    }

    #[test]
    fn test_websocket_session_close() {
        let mut session = WebSocketSession::new("wss://example.com/chat", "example.com", "/chat", true);
        session.close(Some(1000), Some("normal closure".to_string()));
        assert!(session.closed_at.is_some());
        assert_eq!(session.close_code, Some(1000));
        assert_eq!(session.close_reason, Some("normal closure".to_string()));
    }

    #[test]
    fn test_http2_stream_new() {
        let stream = Http2Stream::new(1, "GET", "/api/data");
        assert_eq!(stream.stream_id, 1);
        assert_eq!(stream.method, "GET");
        assert_eq!(stream.state, Http2StreamState::Open);
        assert!(stream.request_body.is_none());
    }

    #[test]
    fn test_http2_session_new() {
        let mut session = Http2Session::new("api.example.com", true);
        assert_eq!(session.host, "api.example.com");
        assert!(session.is_secure);
        assert!(session.streams.is_empty());

        let stream = Http2Stream::new(1, "POST", "/api/data");
        session.add_stream(stream);
        assert_eq!(session.streams.len(), 1);
    }

    #[test]
    fn test_grpc_call_new() {
        let call = GrpcCall::new("/package.Service/Method", GrpcMethodType::Unary);
        assert_eq!(call.path, "/package.Service/Method");
        assert_eq!(call.method_type, GrpcMethodType::Unary);
        assert_eq!(call.response_status, 0);
    }

    #[test]
    fn test_grpc_session_new() {
        let mut session = GrpcSession::new("grpc.example.com", true);
        assert_eq!(session.host, "grpc.example.com");
        assert!(session.is_secure);

        let call = GrpcCall::new("/package.Service/Method", GrpcMethodType::Unary);
        session.add_call(call);
        assert_eq!(session.calls.len(), 1);
    }

    #[test]
    fn test_detect_protocol_websocket() {
        let mut headers = HashMap::new();
        headers.insert("upgrade".to_string(), "websocket".to_string());
        headers.insert("connection".to_string(), "Upgrade".to_string());
        let det = detect_protocol("GET", "/chat", &headers);
        assert_eq!(det.protocol, ProxyProtocol::WebSocket);
        assert!(det.confidence > 0.9);
    }

    #[test]
    fn test_detect_protocol_grpc() {
        let mut headers = HashMap::new();
        headers.insert("content-type".to_string(), "application/grpc+proto".to_string());
        let det = detect_protocol("POST", "/package.Service/Method", &headers);
        assert_eq!(det.protocol, ProxyProtocol::Grpc);
        assert!(det.confidence > 0.9);
    }

    #[test]
    fn test_detect_protocol_http2() {
        let mut headers = HashMap::new();
        headers.insert(":scheme".to_string(), "https".to_string());
        let det = detect_protocol("GET", "/api/data", &headers);
        assert_eq!(det.protocol, ProxyProtocol::Http2);
        assert!(det.confidence > 0.85);
    }

    #[test]
    fn test_detect_protocol_default() {
        let headers = HashMap::new();
        let det = detect_protocol("GET", "/index.html", &headers);
        assert_eq!(det.protocol, ProxyProtocol::Http1);
        assert_eq!(det.confidence, 1.0);
    }

    #[test]
    fn test_detect_grpc_method_type_unary() {
        let headers = HashMap::new();
        assert_eq!(detect_grpc_method_type("/pkg.Svc/Method", &headers), GrpcMethodType::Unary);
    }

    #[test]
    fn test_detect_grpc_method_type_streaming() {
        let mut headers = HashMap::new();
        headers.insert("te".to_string(), "trailers".to_string());
        assert_eq!(detect_grpc_method_type("/pkg.Svc/Stream", &headers), GrpcMethodType::ServerStreaming);
    }

    #[test]
    fn test_grpc_method_type_serialization() {
        let mt = GrpcMethodType::Bidirectional;
        let json = serde_json::to_string(&mt).unwrap();
        assert_eq!(json, "\"bidirectional\"");
        let back: GrpcMethodType = serde_json::from_str(&json).unwrap();
        assert_eq!(back, GrpcMethodType::Bidirectional);
    }
}
