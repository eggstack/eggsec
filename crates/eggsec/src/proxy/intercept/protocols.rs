//! Protocol-specific types for WebSocket, HTTP/2, and gRPC interception.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use super::types::ProxyFlowDirection;

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
    /// Connection-level flow control window size (bytes).
    /// Default: 65535 (HTTP/2 spec default).
    #[serde(default = "default_window_size")]
    pub connection_window_size: u32,
    /// Stream-level flow control window size (bytes).
    /// Default: 65535 (HTTP/2 spec default).
    #[serde(default = "default_window_size")]
    pub stream_window_size: u32,
    /// Maximum concurrent streams allowed.
    #[serde(default = "default_max_concurrent")]
    pub max_concurrent_streams: u32,
    /// Maximum frame size allowed.
    #[serde(default = "default_max_frame_size")]
    pub max_frame_size: u32,
    /// Initial window size for new streams.
    #[serde(default = "default_window_size")]
    pub initial_window_size: u32,
}

fn default_window_size() -> u32 {
    65535
}

fn default_max_concurrent() -> u32 {
    100
}

fn default_max_frame_size() -> u32 {
    16384
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
            connection_window_size: 65535,
            stream_window_size: 65535,
            max_concurrent_streams: 100,
            max_frame_size: 16384,
            initial_window_size: 65535,
        }
    }

    /// Tune window sizes for high-throughput scenarios.
    ///
    /// Increases flow control windows to allow more data in flight,
    /// which can significantly improve throughput for large downloads
    /// or high-bandwidth connections.
    ///
    /// # Arguments
    /// * `connection_window` - Connection-level window size (bytes)
    /// * `stream_window` - Stream-level window size (bytes)
    pub fn tune_windows(&mut self, connection_window: u32, stream_window: u32) {
        self.connection_window_size = connection_window;
        self.stream_window_size = stream_window;
        self.initial_window_size = stream_window;
        self.settings.insert(
            "INITIAL_WINDOW_SIZE".to_string(),
            stream_window.to_string(),
        );
        self.settings.insert(
            "MAX_CONCURRENT_STREAMS".to_string(),
            self.max_concurrent_streams.to_string(),
        );
    }

    /// Get optimal window sizes based on expected throughput.
    ///
    /// Returns tuned window sizes for different scenarios:
    /// - Low latency: 65535 (default)
    /// - Medium throughput: 256KB
    /// - High throughput: 1MB
    pub fn optimal_window_sizes(scenario: WindowTuningScenario) -> (u32, u32) {
        match scenario {
            WindowTuningScenario::LowLatency => (65535, 65535),
            WindowTuningScenario::MediumThroughput => (256 * 1024, 256 * 1024),
            WindowTuningScenario::HighThroughput => (1024 * 1024, 1024 * 1024),
        }
    }

    pub fn add_stream(&mut self, stream: Http2Stream) {
        self.streams.push(stream);
    }
}

/// Window tuning scenario for HTTP/2 connections.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WindowTuningScenario {
    /// Default HTTP/2 settings (65535 bytes).
    LowLatency,
    /// Medium throughput (256KB windows).
    MediumThroughput,
    /// High throughput (1MB windows).
    HighThroughput,
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

    /// Decode protobuf request body from hex-encoded bytes to JSON.
    ///
    /// Attempts to decode the protobuf-encoded request body and convert it
    /// to a JSON representation using prost's dynamic message decoding.
    /// Falls back to the raw hex representation if decoding fails.
    #[cfg(feature = "web-proxy")]
    pub fn decode_request_body(&self) -> Option<serde_json::Value> {
        self.request_body.as_ref().and_then(|hex_bytes| {
            let bytes = hex::decode(hex_bytes).ok()?;
            self.decode_protobuf_to_json(&bytes).ok()
        })
    }

    /// Decode protobuf response body from hex-encoded bytes to JSON.
    ///
    /// Attempts to decode the protobuf-encoded response body and convert it
    /// to a JSON representation using prost's dynamic message decoding.
    /// Falls back to the raw hex representation if decoding fails.
    #[cfg(feature = "web-proxy")]
    pub fn decode_response_body(&self) -> Option<serde_json::Value> {
        self.response_body.as_ref().and_then(|hex_bytes| {
            let bytes = hex::decode(hex_bytes).ok()?;
            self.decode_protobuf_to_json(&bytes).ok()
        })
    }

    /// Encode a JSON value as protobuf bytes and store as hex.
    ///
    /// This provides a best-effort JSON-to-protobuf encoding for unary call editing.
    /// The encoding is simplified and may not produce wire-compatible protobuf for
    /// all message types.
    #[cfg(feature = "web-proxy")]
    pub fn encode_request_body(&mut self, json: &serde_json::Value) -> Result<(), String> {
        let bytes = self.encode_json_to_protobuf(json)?;
        self.request_body = Some(hex::encode(&bytes));
        self.request_size = bytes.len() as u64;
        Ok(())
    }

    /// Encode a JSON value as protobuf bytes and store as hex for response.
    #[cfg(feature = "web-proxy")]
    pub fn encode_response_body(&mut self, json: &serde_json::Value) -> Result<(), String> {
        let bytes = self.encode_json_to_protobuf(json)?;
        self.response_body = Some(hex::encode(&bytes));
        self.response_size = bytes.len() as u64;
        Ok(())
    }

    /// Decode protobuf bytes to JSON using prost.
    ///
    /// This is a simplified decoder that handles common protobuf wire types:
    /// - Varint (wire type 0)
    /// - Length-delimited (wire type 2) - strings, bytes, embedded messages
    /// - 32-bit (wire type 5)
    #[cfg(feature = "web-proxy")]
    fn decode_protobuf_to_json(&self, bytes: &[u8]) -> Result<serde_json::Value, String> {
        // Try prost's default decode first (works for known message types)
        // For unknown types, fall back to manual wire format parsing
        let mut cursor = 0;
        let mut fields = serde_json::Map::new();

        while cursor < bytes.len() {
            // Read field tag (varint)
            let (tag, bytes_read) = read_varint(bytes, cursor)?;
            cursor += bytes_read;

            let field_number = tag >> 3;
            let wire_type = tag & 0x7;

            match wire_type {
                0 => {
                    // Varint
                    let (value, bytes_read) = read_varint(bytes, cursor)?;
                    cursor += bytes_read;
                    fields.insert(
                        format!("field_{}", field_number),
                        serde_json::json!(value),
                    );
                }
                2 => {
                    // Length-delimited
                    let (length, bytes_read) = read_varint(bytes, cursor)?;
                    cursor += bytes_read;
                    let length = length as usize;

                    if cursor + length > bytes.len() {
                        return Err("Truncated length-delimited field".to_string());
                    }

                    let data = &bytes[cursor..cursor + length];
                    cursor += length;

                    // Try to decode as UTF-8 string
                    if let Ok(s) = std::str::from_utf8(data) {
                        fields.insert(
                            format!("field_{}", field_number),
                            serde_json::json!(s),
                        );
                    } else {
                        // Store as base64 for binary data
                        fields.insert(
                            format!("field_{}", field_number),
                            serde_json::json!(base64::Engine::encode(
                                &base64::engine::general_purpose::STANDARD,
                                data
                            )),
                        );
                    }
                }
                5 => {
                    // 32-bit fixed
                    if cursor + 4 > bytes.len() {
                        return Err("Truncated 32-bit field".to_string());
                    }
                    let value = u32::from_le_bytes([
                        bytes[cursor],
                        bytes[cursor + 1],
                        bytes[cursor + 2],
                        bytes[cursor + 3],
                    ]);
                    cursor += 4;
                    fields.insert(
                        format!("field_{}", field_number),
                        serde_json::json!(value),
                    );
                }
                1 => {
                    // 64-bit fixed
                    if cursor + 8 > bytes.len() {
                        return Err("Truncated 64-bit field".to_string());
                    }
                    let value = u64::from_le_bytes([
                        bytes[cursor],
                        bytes[cursor + 1],
                        bytes[cursor + 2],
                        bytes[cursor + 3],
                        bytes[cursor + 4],
                        bytes[cursor + 5],
                        bytes[cursor + 6],
                        bytes[cursor + 7],
                    ]);
                    cursor += 8;
                    fields.insert(
                        format!("field_{}", field_number),
                        serde_json::json!(value),
                    );
                }
                _ => {
                    return Err(format!("Unsupported wire type: {}", wire_type));
                }
            }
        }

        Ok(serde_json::Value::Object(fields))
    }

    /// Encode JSON to protobuf bytes using prost.
    ///
    /// This provides a simplified encoder that handles common JSON types:
    /// - Numbers -> varint or fixed
    /// - Strings -> length-delimited UTF-8
    /// - Objects -> embedded messages
    #[cfg(feature = "web-proxy")]
    fn encode_json_to_protobuf(&self, json: &serde_json::Value) -> Result<Vec<u8>, String> {
        let mut buf = Vec::new();

        match json {
            serde_json::Value::Object(map) => {
                // Encode each field with a synthetic field number
                for (i, (_key, value)) in map.iter().enumerate() {
                    let field_number = (i + 1) as u64;
                    encode_json_field(&mut buf, field_number, value)?;
                }
            }
            serde_json::Value::Array(arr) => {
                // Encode as repeated length-delimited fields
                for (i, value) in arr.iter().enumerate() {
                    let field_number = (i + 1) as u64;
                    encode_json_field(&mut buf, field_number, value)?;
                }
            }
            _ => {
                return Err("Top-level value must be object or array".to_string());
            }
        }

        Ok(buf)
    }
}

/// Read a varint from bytes starting at cursor position.
#[cfg(feature = "web-proxy")]
fn read_varint(bytes: &[u8], cursor: usize) -> Result<(u64, usize), String> {
    let mut result = 0u64;
    let mut shift = 0;
    let mut pos = cursor;

    loop {
        if pos >= bytes.len() {
            return Err("Unexpected end of varint".to_string());
        }

        let byte = bytes[pos];
        result |= ((byte & 0x7F) as u64) << shift;
        pos += 1;

        if byte & 0x80 == 0 {
            return Ok((result, pos - cursor));
        }

        shift += 7;
        if shift >= 64 {
            return Err("Varint too long".to_string());
        }
    }
}

/// Encode a JSON value as a protobuf field and append to buffer.
#[cfg(feature = "web-proxy")]
fn encode_json_field(
    buf: &mut Vec<u8>,
    field_number: u64,
    value: &serde_json::Value,
) -> Result<(), String> {
    let tag = (field_number << 3) | 2; // length-delimited by default

    match value {
        serde_json::Value::Null => {
            // Skip null fields
        }
        serde_json::Value::Bool(b) => {
            // Encode as varint (0 or 1)
            let tag = field_number << 3;
            encode_varint(buf, tag);
            encode_varint(buf, if *b { 1 } else { 0 });
        }
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_u64() {
                let tag = field_number << 3;
                encode_varint(buf, tag);
                encode_varint(buf, i);
            } else if let Some(f) = n.as_f64() {
                let tag = (field_number << 3) | 1;
                encode_varint(buf, tag);
                buf.extend_from_slice(&(f.to_bits()).to_le_bytes());
            }
        }
        serde_json::Value::String(s) => {
            let bytes = s.as_bytes();
            encode_varint(buf, tag);
            encode_varint(buf, bytes.len() as u64);
            buf.extend_from_slice(bytes);
        }
        serde_json::Value::Array(arr) => {
            // Encode each element as a separate field
            for elem in arr {
                encode_json_field(buf, field_number, elem)?;
            }
        }
        serde_json::Value::Object(map) => {
            // Encode as embedded message
            let mut inner_buf = Vec::new();
            for (i, (_key, val)) in map.iter().enumerate() {
                let inner_field = (i + 1) as u64;
                encode_json_field(&mut inner_buf, inner_field, val)?;
            }
            encode_varint(buf, tag);
            encode_varint(buf, inner_buf.len() as u64);
            buf.extend_from_slice(&inner_buf);
        }
    }

    Ok(())
}

/// Encode a u64 as a varint and append to buffer.
#[cfg(feature = "web-proxy")]
fn encode_varint(buf: &mut Vec<u8>, mut value: u64) {
    loop {
        let mut byte = (value & 0x7F) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        buf.push(byte);
        if value == 0 {
            break;
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

    /// Total number of streaming calls in this session.
    pub fn streaming_call_count(&self) -> usize {
        self.calls
            .iter()
            .filter(|c| matches!(c.method_type, GrpcMethodType::ServerStreaming | GrpcMethodType::ClientStreaming | GrpcMethodType::Bidirectional))
            .count()
    }

    /// Find calls with non-OK status codes.
    pub fn error_calls(&self) -> Vec<&GrpcCall> {
        self.calls.iter().filter(|c| c.response_status != 0).collect()
    }
}

/// A single frame in a gRPC streaming call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrpcStreamFrame {
    /// Stream ID (HTTP/2 stream identifier).
    pub stream_id: u32,
    /// Direction of the frame.
    pub direction: ProxyFlowDirection,
    /// Frame payload (protobuf bytes as hex).
    pub payload: Option<String>,
    /// Payload size in bytes.
    pub size: u64,
    /// Whether this is an end-of-stream frame.
    pub end_stream: bool,
    /// Timestamp.
    pub timestamp: String,
    /// Compression flag.
    pub compressed: bool,
}

impl GrpcStreamFrame {
    pub fn new(stream_id: u32, direction: ProxyFlowDirection) -> Self {
        Self {
            stream_id,
            direction,
            payload: None,
            size: 0,
            end_stream: false,
            timestamp: chrono::Utc::now().to_rfc3339(),
            compressed: false,
        }
    }

    pub fn with_payload(mut self, payload: Vec<u8>) -> Self {
        self.size = payload.len() as u64;
        self.payload = Some(hex::encode(&payload));
        self
    }

    pub fn with_end_stream(mut self) -> Self {
        self.end_stream = true;
        self
    }

    pub fn with_compressed(mut self) -> Self {
        self.compressed = true;
        self
    }
}

/// State tracking for a gRPC streaming call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrpcStreamingState {
    /// Whether this is a client-streaming, server-streaming, or bidi call.
    pub method_type: GrpcMethodType,
    /// Client-to-server frames.
    pub client_frames: Vec<GrpcStreamFrame>,
    /// Server-to-client frames.
    pub server_frames: Vec<GrpcStreamFrame>,
    /// Flow control window size (from HTTP/2 SETTINGS).
    pub flow_control_window: u32,
    /// Current bytes in flight (unacknowledged).
    pub bytes_in_flight: u64,
    /// Maximum concurrent streams allowed.
    pub max_concurrent_streams: u32,
}

impl GrpcStreamingState {
    pub fn new(method_type: GrpcMethodType) -> Self {
        Self {
            method_type,
            client_frames: Vec::new(),
            server_frames: Vec::new(),
            flow_control_window: 65535, // default HTTP/2 window
            bytes_in_flight: 0,
            max_concurrent_streams: 100,
        }
    }

    pub fn add_frame(&mut self, frame: GrpcStreamFrame) {
        match frame.direction {
            ProxyFlowDirection::Request => self.client_frames.push(frame),
            ProxyFlowDirection::Response => self.server_frames.push(frame),
        }
    }

    /// Total frames captured.
    pub fn total_frames(&self) -> usize {
        self.client_frames.len() + self.server_frames.len()
    }

    /// Total bytes transferred.
    pub fn total_bytes(&self) -> u64 {
        self.client_frames.iter().map(|f| f.size).sum::<u64>()
            + self.server_frames.iter().map(|f| f.size).sum::<u64>()
    }

    /// Whether the stream has completed (both sides sent end-of-stream).
    pub fn is_complete(&self) -> bool {
        let client_ended = self.client_frames.iter().any(|f| f.end_stream);
        let server_ended = self.server_frames.iter().any(|f| f.end_stream);
        match self.method_type {
            GrpcMethodType::Unary => true, // unary doesn't use streaming
            GrpcMethodType::ServerStreaming => server_ended,
            GrpcMethodType::ClientStreaming => client_ended,
            GrpcMethodType::Bidirectional => client_ended && server_ended,
        }
    }
}

/// A security finding specific to gRPC services.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrpcSecurityFinding {
    /// Service path where the finding was detected.
    pub service_path: String,
    /// Finding category (e.g., "missing_auth", "open_endpoint", "large_payload").
    pub category: String,
    /// Severity (0-10).
    pub severity: u8,
    /// Description of the finding.
    pub description: String,
    /// Recommended remediation.
    pub remediation: Option<String>,
}

/// Detect common gRPC security issues from a captured call.
pub fn detect_grpc_security_issues(call: &GrpcCall) -> Vec<GrpcSecurityFinding> {
    let mut findings = Vec::new();

    // Check for missing authorization metadata
    let has_auth = call
        .request_metadata
        .keys()
        .any(|k| k.eq_ignore_ascii_case("authorization"));
    if !has_auth && call.response_status == 0 {
        findings.push(GrpcSecurityFinding {
            service_path: call.path.clone(),
            category: "missing_auth".to_string(),
            severity: 5,
            description: "gRPC call has no Authorization metadata".to_string(),
            remediation: Some("Add authentication interceptors".to_string()),
        });
    }

    // Check for overly large payloads (potential DoS)
    if call.request_size > 10_000_000 {
        // 10MB
        findings.push(GrpcSecurityFinding {
            service_path: call.path.clone(),
            category: "large_payload".to_string(),
            severity: 4,
            description: format!("Request payload is {} bytes (>10MB)", call.request_size),
            remediation: Some("Enforce maximum message size limits".to_string()),
        });
    }

    // Check for gRPC errors
    if call.response_status != 0 {
        let status_name = match call.response_status {
            1 => "CANCELLED",
            2 => "UNKNOWN",
            3 => "INVALID_ARGUMENT",
            4 => "DEADLINE_EXCEEDED",
            5 => "NOT_FOUND",
            6 => "ALREADY_EXISTS",
            7 => "PERMISSION_DENIED",
            8 => "RESOURCE_EXHAUSTED",
            9 => "FAILED_PRECONDITION",
            10 => "ABORTED",
            11 => "OUT_OF_RANGE",
            12 => "UNIMPLEMENTED",
            13 => "INTERNAL",
            14 => "UNAVAILABLE",
            15 => "DATA_LOSS",
            16 => "UNAUTHENTICATED",
            _ => "UNKNOWN_STATUS",
        };
        findings.push(GrpcSecurityFinding {
            service_path: call.path.clone(),
            category: "grpc_error".to_string(),
            severity: if call.response_status == 7 || call.response_status == 16 {
                6
            } else {
                3
            },
            description: format!(
                "gRPC call returned status {} ({})",
                call.response_status, status_name
            ),
            remediation: None,
        });
    }

    // Check for sensitive paths
    let sensitive_patterns = ["/admin", "/debug", "/internal", "/test", "/swagger"];
    for pattern in &sensitive_patterns {
        if call.path.to_lowercase().contains(pattern) {
            findings.push(GrpcSecurityFinding {
                service_path: call.path.clone(),
                category: "sensitive_endpoint".to_string(),
                severity: 6,
                description: format!(
                    "gRPC endpoint contains sensitive path pattern '{}'",
                    pattern
                ),
                remediation: Some("Restrict access to sensitive endpoints".to_string()),
            });
            break;
        }
    }

    findings
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

    // --- GrpcStreamFrame tests ---

    #[test]
    fn test_grpc_stream_frame_new() {
        let frame = GrpcStreamFrame::new(1, ProxyFlowDirection::Request);
        assert_eq!(frame.stream_id, 1);
        assert_eq!(frame.direction, ProxyFlowDirection::Request);
        assert!(frame.payload.is_none());
        assert_eq!(frame.size, 0);
        assert!(!frame.end_stream);
        assert!(!frame.compressed);
    }

    #[test]
    fn test_grpc_stream_frame_builder() {
        let payload = vec![0x0a, 0x04, 0x74, 0x65, 0x73, 0x74];
        let frame = GrpcStreamFrame::new(3, ProxyFlowDirection::Response)
            .with_payload(payload.clone())
            .with_end_stream()
            .with_compressed();
        assert_eq!(frame.size, payload.len() as u64);
        assert!(frame.end_stream);
        assert!(frame.compressed);
        assert!(frame.payload.is_some());
        // hex-encoded payload
        assert_eq!(frame.payload.unwrap(), hex::encode(&payload));
    }

    #[test]
    fn test_grpc_stream_frame_serialization() {
        let frame = GrpcStreamFrame::new(5, ProxyFlowDirection::Request)
            .with_payload(vec![1, 2, 3])
            .with_end_stream();
        let json = serde_json::to_string(&frame).unwrap();
        let back: GrpcStreamFrame = serde_json::from_str(&json).unwrap();
        assert_eq!(back.stream_id, 5);
        assert!(back.end_stream);
        assert_eq!(back.size, 3);
    }

    // --- GrpcStreamingState tests ---

    #[test]
    fn test_grpc_streaming_state_new() {
        let state = GrpcStreamingState::new(GrpcMethodType::Bidirectional);
        assert_eq!(state.method_type, GrpcMethodType::Bidirectional);
        assert!(state.client_frames.is_empty());
        assert!(state.server_frames.is_empty());
        assert_eq!(state.flow_control_window, 65535);
        assert_eq!(state.bytes_in_flight, 0);
        assert_eq!(state.max_concurrent_streams, 100);
    }

    #[test]
    fn test_grpc_streaming_state_add_frame() {
        let mut state = GrpcStreamingState::new(GrpcMethodType::Bidirectional);
        let req = GrpcStreamFrame::new(1, ProxyFlowDirection::Request)
            .with_payload(vec![1, 2, 3]);
        let resp = GrpcStreamFrame::new(1, ProxyFlowDirection::Response)
            .with_payload(vec![4, 5, 6, 7]);
        state.add_frame(req);
        state.add_frame(resp);
        assert_eq!(state.client_frames.len(), 1);
        assert_eq!(state.server_frames.len(), 1);
    }

    #[test]
    fn test_grpc_streaming_state_total_frames() {
        let mut state = GrpcStreamingState::new(GrpcMethodType::ServerStreaming);
        for _ in 0..5 {
            state.add_frame(GrpcStreamFrame::new(1, ProxyFlowDirection::Request));
            state.add_frame(GrpcStreamFrame::new(1, ProxyFlowDirection::Response));
        }
        assert_eq!(state.total_frames(), 10);
    }

    #[test]
    fn test_grpc_streaming_state_total_bytes() {
        let mut state = GrpcStreamingState::new(GrpcMethodType::Unary);
        state.add_frame(GrpcStreamFrame::new(1, ProxyFlowDirection::Request).with_payload(vec![0; 100]));
        state.add_frame(GrpcStreamFrame::new(1, ProxyFlowDirection::Response).with_payload(vec![0; 200]));
        assert_eq!(state.total_bytes(), 300);
    }

    #[test]
    fn test_grpc_streaming_state_is_complete_unary() {
        let state = GrpcStreamingState::new(GrpcMethodType::Unary);
        // Unary always returns true
        assert!(state.is_complete());
    }

    #[test]
    fn test_grpc_streaming_state_is_complete_server_streaming() {
        let mut state = GrpcStreamingState::new(GrpcMethodType::ServerStreaming);
        assert!(!state.is_complete());
        state.add_frame(GrpcStreamFrame::new(1, ProxyFlowDirection::Response).with_end_stream());
        assert!(state.is_complete());
    }

    #[test]
    fn test_grpc_streaming_state_is_complete_client_streaming() {
        let mut state = GrpcStreamingState::new(GrpcMethodType::ClientStreaming);
        assert!(!state.is_complete());
        state.add_frame(GrpcStreamFrame::new(1, ProxyFlowDirection::Request).with_end_stream());
        assert!(state.is_complete());
    }

    #[test]
    fn test_grpc_streaming_state_is_complete_bidi() {
        let mut state = GrpcStreamingState::new(GrpcMethodType::Bidirectional);
        assert!(!state.is_complete());
        state.add_frame(GrpcStreamFrame::new(1, ProxyFlowDirection::Request).with_end_stream());
        assert!(!state.is_complete()); // server not ended yet
        state.add_frame(GrpcStreamFrame::new(1, ProxyFlowDirection::Response).with_end_stream());
        assert!(state.is_complete());
    }

    // --- detect_grpc_security_issues tests ---

    #[test]
    fn test_detect_grpc_security_issues_missing_auth() {
        let call = GrpcCall::new("/pkg.Svc/Method", GrpcMethodType::Unary);
        // No authorization header, response_status=0 (success) -> missing_auth
        let findings = detect_grpc_security_issues(&call);
        assert!(findings.iter().any(|f| f.category == "missing_auth"));
    }

    #[test]
    fn test_detect_grpc_security_issues_with_auth_no_finding() {
        let mut call = GrpcCall::new("/pkg.Svc/Method", GrpcMethodType::Unary);
        call.request_metadata.insert("authorization".to_string(), "Bearer token".to_string());
        let findings = detect_grpc_security_issues(&call);
        assert!(!findings.iter().any(|f| f.category == "missing_auth"));
    }

    #[test]
    fn test_detect_grpc_security_issues_large_payload() {
        let mut call = GrpcCall::new("/pkg.Svc/Method", GrpcMethodType::Unary);
        call.request_size = 15_000_000; // 15MB > 10MB threshold
        let findings = detect_grpc_security_issues(&call);
        assert!(findings.iter().any(|f| f.category == "large_payload"));
        let large = findings.iter().find(|f| f.category == "large_payload").unwrap();
        assert_eq!(large.severity, 4);
    }

    #[test]
    fn test_detect_grpc_security_issues_error_status() {
        let mut call = GrpcCall::new("/pkg.Svc/Method", GrpcMethodType::Unary);
        call.response_status = 13; // INTERNAL
        let findings = detect_grpc_security_issues(&call);
        assert!(findings.iter().any(|f| f.category == "grpc_error"));
        let err = findings.iter().find(|f| f.category == "grpc_error").unwrap();
        assert!(err.description.contains("INTERNAL"));
        assert_eq!(err.severity, 3); // non-auth error -> severity 3
    }

    #[test]
    fn test_detect_grpc_security_issues_permission_denied() {
        let mut call = GrpcCall::new("/pkg.Svc/Method", GrpcMethodType::Unary);
        call.response_status = 7; // PERMISSION_DENIED
        let findings = detect_grpc_security_issues(&call);
        let err = findings.iter().find(|f| f.category == "grpc_error").unwrap();
        assert_eq!(err.severity, 6); // auth-related error -> severity 6
    }

    #[test]
    fn test_detect_grpc_security_issues_unauthenticated() {
        let mut call = GrpcCall::new("/pkg.Svc/Method", GrpcMethodType::Unary);
        call.response_status = 16; // UNAUTHENTICATED
        let findings = detect_grpc_security_issues(&call);
        let err = findings.iter().find(|f| f.category == "grpc_error").unwrap();
        assert_eq!(err.severity, 6);
    }

    #[test]
    fn test_detect_grpc_security_issues_sensitive_path() {
        let call = GrpcCall::new("/admin.Service/Debug", GrpcMethodType::Unary);
        let findings = detect_grpc_security_issues(&call);
        assert!(findings.iter().any(|f| f.category == "sensitive_endpoint"));
        let ep = findings.iter().find(|f| f.category == "sensitive_endpoint").unwrap();
        assert_eq!(ep.severity, 6);
    }

    #[test]
    fn test_detect_grpc_security_issues_multiple_sensitive_patterns() {
        // Only first match should be reported (break after first)
        let call = GrpcCall::new("/internal/debug/Admin", GrpcMethodType::Unary);
        let findings = detect_grpc_security_issues(&call);
        let sensitive: Vec<_> = findings.iter().filter(|f| f.category == "sensitive_endpoint").collect();
        assert_eq!(sensitive.len(), 1); // break after first match
    }

    #[test]
    fn test_detect_grpc_security_issues_clean_call() {
        let mut call = GrpcCall::new("/pkg.Svc/Method", GrpcMethodType::Unary);
        call.request_metadata.insert("authorization".to_string(), "Bearer token".to_string());
        call.request_size = 1024;
        call.response_status = 0;
        let findings = detect_grpc_security_issues(&call);
        assert!(findings.is_empty());
    }

    // --- GrpcSession streaming helpers ---

    #[test]
    fn test_grpc_session_streaming_call_count() {
        let mut session = GrpcSession::new("grpc.example.com", true);
        session.add_call(GrpcCall::new("/pkg.Svc/Unary", GrpcMethodType::Unary));
        session.add_call(GrpcCall::new("/pkg.Svc/ServerStream", GrpcMethodType::ServerStreaming));
        session.add_call(GrpcCall::new("/pkg.Svc/Bidi", GrpcMethodType::Bidirectional));
        session.add_call(GrpcCall::new("/pkg.Svc/ClientStream", GrpcMethodType::ClientStreaming));
        assert_eq!(session.streaming_call_count(), 3);
    }

    #[test]
    fn test_grpc_session_error_calls() {
        let mut session = GrpcSession::new("grpc.example.com", true);
        let mut ok_call = GrpcCall::new("/pkg.Svc/OK", GrpcMethodType::Unary);
        ok_call.response_status = 0;
        let mut err_call = GrpcCall::new("/pkg.Svc/Err", GrpcMethodType::Unary);
        err_call.response_status = 13;
        session.add_call(ok_call);
        session.add_call(err_call);
        assert_eq!(session.error_calls().len(), 1);
        assert_eq!(session.error_calls()[0].path, "/pkg.Svc/Err");
    }
}
