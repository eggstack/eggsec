//! HTTP/HTTPS intercepting proxy with dynamic SSL certificate generation
//!
//! Provides an intercepting proxy for security testing with:
//! - Dynamic SSL certificate generation for HTTPS interception
//! - Request/response interception and modification
//! - Monitor mode for passive traffic logging
//! - Configurable interception rules

mod bridge;
mod bundle;
mod cert;
mod interceptor;
pub mod narrative;
pub mod plugins;
pub mod protocols;
mod redteam;
mod rules;
pub mod types;
pub mod correlation;
#[cfg(feature = "transparent-proxy")]
pub mod transparent;

pub use bridge::to_scan_report_data_proxy;
pub use bundle::{EvidenceBundle, BundleManifest, BundleDiff, export_evidence_bundle, export_signed_evidence_bundle, import_evidence_bundle, compare_bundles};
pub use cert::{CertGenerator, CertMaterial};
pub use interceptor::{InterceptConfig, InterceptMode, InterceptProxy};
pub use rules::{
    InterceptRule, RuleAction, RuleSet,
    EnhancedRule, EnhancedRuleSet, RuleCondition, RuleContext, RuleId, InjectResponseConfig,
};

pub use protocols::{
    ProxyProtocol, WebSocketMessage, WebSocketSession, WebSocketOpcode,
    Http2Stream, Http2Session, Http2StreamState,
    GrpcCall, GrpcSession, GrpcMethodType, ProtocolDetection,
    GrpcStreamFrame, GrpcStreamingState, GrpcSecurityFinding,
    detect_grpc_security_issues,
};

pub use correlation::{
    CorrelationContext, CorrelationReference, CorrelationSource,
    CorrelationHook, CorrelationSummary,
    CorrelationEngine, TemporalCorrelation, BehavioralPattern,
};

pub use narrative::{AttackNarrative, NarrativeEvent, build_narrative};

pub use plugins::{
    ProtocolHandler, PluginRegistry, PluginInfo, PluginError,
    DetectionResult, HandleResult, PluginFinding,
};

use crate::error::{EggsecError, Result};
use bytes::Bytes;
use parking_lot::RwLock;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::{timeout, Duration};
use tokio_rustls::rustls::ServerConfig;
use tokio_rustls::TlsAcceptor;

pub struct ProxyServer {
    addr: SocketAddr,
    cert_generator: CertGenerator,
    rules: Arc<RwLock<RuleSet>>,
    enhanced_rules: Arc<RwLock<EnhancedRuleSet>>,
    mode: InterceptMode,
    proxy_http2_live: bool,
}

impl ProxyServer {
    pub fn new(addr: SocketAddr) -> Result<Self> {
        Ok(Self {
            addr,
            cert_generator: CertGenerator::new(),
            rules: Arc::new(RwLock::new(RuleSet::default())),
            enhanced_rules: Arc::new(RwLock::new(EnhancedRuleSet::new())),
            mode: InterceptMode::Monitor,
            proxy_http2_live: false,
        })
    }

    pub fn with_mode(mut self, mode: InterceptMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn with_cert_generator(mut self, gen: CertGenerator) -> Self {
        self.cert_generator = gen;
        self
    }

    pub fn with_proxy_http2_live(mut self, live: bool) -> Self {
        self.proxy_http2_live = live;
        self
    }

    pub fn add_rule(&self, rule: InterceptRule) {
        let mut rules = self.rules.write();
        rules.add(rule);
    }

    pub fn add_enhanced_rule(&self, rule: EnhancedRule) {
        let mut rules = self.enhanced_rules.write();
        rules.add(rule);
    }

    pub async fn start(&self) -> Result<()> {
        let listener = TcpListener::bind(self.addr)
            .await
            .map_err(|e| EggsecError::Network(format!("Failed to bind proxy: {}", e)))?;

        tracing::info!("Proxy listening on {}", self.addr);

        loop {
            match listener.accept().await {
                Ok((stream, client_addr)) => {
                    let rules = Arc::clone(&self.rules);
                    let enhanced_rules = Arc::clone(&self.enhanced_rules);
                    let mode = self.mode;
                    let cert_gen = self.cert_generator.clone();
                    let http2_live = self.proxy_http2_live;

                    tokio::spawn(async move {
                        if let Err(e) =
                            handle_connection(stream, client_addr, rules, enhanced_rules, mode, cert_gen, http2_live).await
                        {
                            tracing::debug!("Connection error: {}", e);
                        }
                    });
                }
                Err(e) => {
                    tracing::warn!("Accept error: {}", e);
                }
            }
        }
    }
}

fn is_private_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ipv4) => {
            let octets = ipv4.octets();
            octets[0] == 10
                || (octets[0] == 172 && (16..=31).contains(&octets[1]))
                || (octets[0] == 192 && octets[1] == 168)
                || octets[0] == 127
                || (octets[0] >= 224 && octets[0] <= 239)
                || octets.iter().all(|&o| o == 255)
        }
        IpAddr::V6(ipv6) => {
            let segments = ipv6.segments();
            (segments[0] & 0xfe00) == 0xfc00
                || ipv6.is_loopback()
                || (segments[0] & 0xff00) == 0xff00
                || ipv6.is_unspecified()
                || (segments[0] & 0xffc0) == 0xfe80
        }
    }
}

fn validate_target(host: &str, port: u16) -> Result<()> {
    let ip: IpAddr = host
        .parse()
        .map_err(|_| EggsecError::ScopeViolation(format!("Invalid host address: {}", host)))?;

    if is_private_ip(ip) {
        return Err(EggsecError::ScopeViolation(format!(
            "Connection to private/internal address blocked: {}:{}",
            host, port
        )));
    }

    Ok(())
}

fn is_websocket_upgrade(headers: &HashMap<String, String>) -> bool {
    headers
        .iter()
        .any(|(k, v)| k.eq_ignore_ascii_case("upgrade") && v.eq_ignore_ascii_case("websocket"))
}

fn parse_ws_http_headers(request_str: &str) -> HashMap<String, String> {
    let mut headers = HashMap::new();
    for line in request_str.lines().skip(1) {
        if line.is_empty() {
            break;
        }
        if let Some(colon_idx) = line.find(':') {
            let key = line[..colon_idx].trim().to_string();
            let value = line[colon_idx + 1..].trim().to_string();
            headers.insert(key, value);
        }
    }
    headers
}

fn extract_ws_path(request_str: &str) -> &str {
    request_str
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .map(|uri| {
            let after_scheme = if let Some(idx) = uri.find("://") {
                &uri[idx + 3..]
            } else {
                uri
            };
            if let Some(slash_idx) = after_scheme.find('/') {
                &after_scheme[slash_idx..]
            } else {
                "/"
            }
        })
        .unwrap_or("/")
}

async fn read_http_headers(
    stream: &mut (impl tokio::io::AsyncRead + Unpin),
) -> Result<Vec<u8>> {
    let mut request = Vec::with_capacity(1024);
    let mut buf = [0u8; 4096];

    loop {
        let n = stream
            .read(&mut buf)
            .await
            .map_err(|e| EggsecError::Proxy(format!("Failed to read HTTP headers: {}", e)))?;
        if n == 0 {
            break;
        }
        request.extend_from_slice(&buf[..n]);
        if request.windows(4).any(|w| w == b"\r\n\r\n") {
            break;
        }
    }

    Ok(request)
}

#[cfg(feature = "web-proxy")]
async fn handle_websocket_interception(
    client_stream: tokio_rustls::server::TlsStream<TcpStream>,
    upstream: TcpStream,
    host: &str,
    path: &str,
) -> Result<()> {
    use futures::{SinkExt, StreamExt};
    use tokio_tungstenite::tungstenite::protocol::Role;
    use tokio_tungstenite::tungstenite::Message as TungsteniteMessage;

    let ws_url = format!("wss://{}{}", host, path);

    let ws_client = tokio_tungstenite::WebSocketStream::from_raw_socket(
        client_stream,
        Role::Server,
        None,
    )
    .await;

    let ws_upstream = tokio_tungstenite::WebSocketStream::from_raw_socket(
        upstream,
        Role::Client,
        None,
    )
    .await;

    let mut session = WebSocketSession::new(&ws_url, host, path, true);

    let (mut client_sink, mut client_source) = ws_client.split();
    let (mut upstream_sink, mut upstream_source) = ws_upstream.split();

    let client_to_upstream = tokio::spawn(async move {
        while let Some(msg) = client_source.next().await {
            match msg {
                Ok(TungsteniteMessage::Text(text)) => {
                    let _ = upstream_sink.send(TungsteniteMessage::Text(text)).await;
                }
                Ok(TungsteniteMessage::Binary(data)) => {
                    let _ = upstream_sink.send(TungsteniteMessage::Binary(data)).await;
                }
                Ok(TungsteniteMessage::Close(frame)) => {
                    let _ = upstream_sink.send(TungsteniteMessage::Close(frame)).await;
                    break;
                }
                Ok(TungsteniteMessage::Ping(data)) => {
                    let _ = upstream_sink.send(TungsteniteMessage::Ping(data)).await;
                }
                Ok(TungsteniteMessage::Pong(data)) => {
                    let _ = upstream_sink.send(TungsteniteMessage::Pong(data)).await;
                }
                Ok(TungsteniteMessage::Frame(_)) => {}
                Err(_) => break,
            }
        }
    });

    let upstream_to_client = tokio::spawn(async move {
        while let Some(msg) = upstream_source.next().await {
            match msg {
                Ok(TungsteniteMessage::Text(text)) => {
                    let _ = client_sink.send(TungsteniteMessage::Text(text)).await;
                }
                Ok(TungsteniteMessage::Binary(data)) => {
                    let _ = client_sink.send(TungsteniteMessage::Binary(data)).await;
                }
                Ok(TungsteniteMessage::Close(frame)) => {
                    let _ = client_sink.send(TungsteniteMessage::Close(frame)).await;
                    break;
                }
                Ok(TungsteniteMessage::Ping(data)) => {
                    let _ = client_sink.send(TungsteniteMessage::Ping(data)).await;
                }
                Ok(TungsteniteMessage::Pong(data)) => {
                    let _ = client_sink.send(TungsteniteMessage::Pong(data)).await;
                }
                Ok(TungsteniteMessage::Frame(_)) => {}
                Err(_) => break,
            }
        }
    });

    tokio::select! {
        result = client_to_upstream => {
            if let Err(e) = result {
                tracing::debug!("WebSocket client->upstream task error: {}", e);
            }
        }
        result = upstream_to_client => {
            if let Err(e) = result {
                tracing::debug!("WebSocket upstream->client task error: {}", e);
            }
        }
    };

    session.close(None, None);
    tracing::debug!(
        "WebSocket session closed: client={} msgs, server={} msgs, {} total bytes",
        session.client_message_count,
        session.server_message_count,
        session.total_bytes,
    );

    Ok(())
}

#[cfg(not(feature = "web-proxy"))]
async fn handle_websocket_interception(
    _client_stream: tokio_rustls::server::TlsStream<TcpStream>,
    _upstream: TcpStream,
    _host: &str,
    _path: &str,
) -> Result<()> {
    tracing::debug!("WebSocket interception requires web-proxy feature; falling back to passthrough");
    Ok(())
}

#[cfg(feature = "web-proxy")]
async fn handle_http2_interception(
    client_stream: tokio_rustls::server::TlsStream<TcpStream>,
    upstream: TcpStream,
    host: &str,
) -> Result<()> {
    use http::header;

    async fn read_h2_body(body: &mut h2::RecvStream) -> Vec<u8> {
        let mut buf = Vec::new();
        while let Some(chunk) = body.data().await {
            if let Ok(data) = chunk {
                buf.extend_from_slice(&data);
            } else {
                break;
            }
        }
        buf
    }

    // Perform HTTP/2 server handshake with the client
    let mut h2_client = match h2::server::handshake(client_stream).await {
        Ok(conn) => conn,
        Err(e) => {
            tracing::debug!("HTTP/2 client handshake failed for {}: {}", host, e);
            return Ok(());
        }
    };

    // Connect to upstream with HTTP/2
    let (mut h2_upstream_send, h2_upstream_conn) = match h2::client::handshake(upstream).await {
        Ok(parts) => parts,
        Err(e) => {
            tracing::debug!("HTTP/2 upstream handshake failed for {}: {}", host, e);
            return Ok(());
        }
    };

    // Drive the upstream connection in the background
    let host_clone = host.to_string();
    tokio::spawn(async move {
        if let Err(e) = h2_upstream_conn.await {
            tracing::debug!("HTTP/2 upstream connection error for {}: {}", host_clone, e);
        }
    });

    let mut session = Http2Session::new(host, true);

    // Proxy streams: accept from client, forward to upstream, relay responses back
    loop {
        tokio::select! {
            accepted = h2_client.accept() => {
                match accepted {
                    Some(Ok((req, mut client_respond))) => {
                        let method = req.method().clone();
                        let uri = req.uri().clone();
                        let path = uri.path().to_string();
                        let stream_id = client_respond.stream_id().as_u32();

                        // Collect request headers
                        let mut request_headers = HashMap::new();
                        for (name, value) in req.headers() {
                            request_headers.insert(
                                name.as_str().to_string(),
                                String::from_utf8_lossy(value.as_bytes()).to_string(),
                            );
                        }

                        // Read request body
                        let mut req_body = req.into_body();
                        let body_bytes = read_h2_body(&mut req_body).await;
                        let request_body = if body_bytes.is_empty() {
                            None
                        } else {
                            Some(String::from_utf8_lossy(&body_bytes).to_string())
                        };

                        // Build Http2Stream record
                        let mut http2_stream = Http2Stream::new(stream_id, method.as_str(), &path);
                        http2_stream.request_headers = request_headers.clone();
                        http2_stream.request_body = request_body;
                        http2_stream.request_body_size = body_bytes.len() as u64;

                        // Build upstream request
                        let mut upstream_request = http::Request::builder()
                            .method(method.clone())
                            .uri(uri.clone());

                        for (name, value) in &request_headers {
                            if !name.eq_ignore_ascii_case("host") {
                                if let Ok(header_name) = header::HeaderName::from_bytes(name.as_bytes()) {
                                    if let Ok(header_value) = http::HeaderValue::from_str(value) {
                                        upstream_request = upstream_request.header(header_name, header_value);
                                    }
                                }
                            }
                        }

                        // Add host header if not present
                        if !request_headers.contains_key("host") && !request_headers.contains_key("Host") {
                            upstream_request = upstream_request.header(header::HOST, host);
                        }

                        let upstream_req = match upstream_request.body(()) {
                            Ok(r) => r,
                            Err(e) => {
                                tracing::debug!("Failed to build upstream request: {}", e);
                                let _ = client_respond.send_response(
                                    http::Response::builder()
                                        .status(502)
                                        .body(())
                                        .unwrap(),
                                    true,
                                );
                                continue;
                            }
                        };

                        // Send request to upstream
                        match h2_upstream_send.ready().await {
                            Ok(ready_send) => { h2_upstream_send = ready_send; }
                            Err(e) => {
                                tracing::debug!("HTTP/2 upstream not ready: {}", e);
                                let _ = client_respond.send_response(
                                    http::Response::builder()
                                        .status(502)
                                        .body(())
                                        .unwrap(),
                                    true,
                                );
                                break;
                            }
                        }

                        let has_body = !body_bytes.is_empty();
                        match h2_upstream_send.send_request(upstream_req, !has_body) {
                            Ok((upstream_response, mut upstream_body)) => {
                                // If there was a request body, send it
                                if has_body {
                                    let _ = upstream_body.send_data(Bytes::from(body_bytes), true);
                                }

                                // Read upstream response
                                let upstream_resp = match upstream_response.await {
                                    Ok(r) => r,
                                    Err(e) => {
                                        tracing::debug!("HTTP/2 upstream response error on stream {}: {}", stream_id, e);
                                        let _ = client_respond.send_response(
                                            http::Response::builder()
                                                .status(502)
                                                .body(())
                                                .unwrap(),
                                            true,
                                        );
                                        continue;
                                    }
                                };

                                // Collect response headers
                                let response_status = upstream_resp.status().as_u16();
                                let mut response_headers_map = HashMap::new();
                                for (name, value) in upstream_resp.headers() {
                                    response_headers_map.insert(
                                        name.as_str().to_string(),
                                        String::from_utf8_lossy(value.as_bytes()).to_string(),
                                    );
                                }

                                // Read response body from upstream
                                let mut resp_body = upstream_resp.into_body();
                                let resp_bytes = read_h2_body(&mut resp_body).await;

                                // Build client response
                                let mut client_response = http::Response::builder()
                                    .status(response_status);

                                for (name, value) in &response_headers_map {
                                    if let Ok(header_name) = header::HeaderName::from_bytes(name.as_bytes()) {
                                        if let Ok(header_value) = http::HeaderValue::from_str(value) {
                                            client_response = client_response.header(header_name, header_value);
                                        }
                                    }
                                }

                                let has_resp_body = !resp_bytes.is_empty();
                                match client_response.body(()) {
                                    Ok(client_resp) => {
                                        match client_respond.send_response(client_resp, !has_resp_body) {
                                            Ok(mut send_stream) => {
                                                if has_resp_body {
                                                    if let Err(e) = send_stream.send_data(Bytes::from(resp_bytes.clone()), true) {
                                                        tracing::debug!("HTTP/2 client send_data error on stream {}: {}", stream_id, e);
                                                    }
                                                }

                                                // Update Http2Stream record
                                                http2_stream.response_status = response_status;
                                                http2_stream.response_headers = response_headers_map;
                                                http2_stream.response_body = Some(String::from_utf8_lossy(&resp_bytes).to_string());
                                                http2_stream.response_body_size = resp_bytes.len() as u64;
                                                http2_stream.state = Http2StreamState::Closed;
                                                http2_stream.closed_at = Some(chrono::Utc::now().to_rfc3339());

                                                session.add_stream(http2_stream);

                                                tracing::debug!(
                                                    "HTTP/2 stream {} proxied: {} {} -> {} ({} bytes)",
                                                    stream_id, method, path, response_status, resp_bytes.len()
                                                );
                                            }
                                            Err(e) => {
                                                tracing::debug!("HTTP/2 client send_response error on stream {}: {}", stream_id, e);
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        tracing::debug!("Failed to build client response: {}", e);
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::debug!("HTTP/2 upstream send_request error on stream {}: {}", stream_id, e);
                                let _ = client_respond.send_response(
                                    http::Response::builder()
                                        .status(502)
                                        .body(())
                                        .unwrap(),
                                    true,
                                );
                            }
                        }
                    }
                    Some(Err(e)) => {
                        tracing::debug!("HTTP/2 client accept error for {}: {}", host, e);
                        break;
                    }
                    None => {
                        tracing::debug!("HTTP/2 client connection closed for {}", host);
                        break;
                    }
                }
            }
        }
    }

    session.closed_at = Some(chrono::Utc::now().to_rfc3339());
    tracing::debug!(
        "HTTP/2 session closed for {}: {} streams",
        host,
        session.streams.len()
    );

    Ok(())
}

#[cfg(not(feature = "web-proxy"))]
async fn handle_http2_interception(
    _client_stream: tokio_rustls::server::TlsStream<TcpStream>,
    _upstream: TcpStream,
    _host: &str,
) -> Result<()> {
    tracing::debug!("HTTP/2 interception requires web-proxy feature; falling back to passthrough");
    Ok(())
}

fn build_rule_context(
    host: &str,
    path: &str,
    method: &str,
    headers: &HashMap<String, String>,
    body: Option<&str>,
    protocol: &str,
) -> RuleContext {
    RuleContext {
        host: host.to_string(),
        path: path.to_string(),
        method: method.to_string(),
        headers: headers.clone(),
        body: body.map(String::from),
        body_size: body.map(|b| b.len() as u64),
        protocol: protocol.to_string(),
        ws_opcode: None,
        grpc_method: None,
    }
}

async fn handle_connection(
    mut stream: TcpStream,
    _client_addr: SocketAddr,
    rules: Arc<RwLock<RuleSet>>,
    enhanced_rules: Arc<RwLock<EnhancedRuleSet>>,
    _mode: InterceptMode,
    cert_gen: CertGenerator,
    http2_live: bool,
) -> Result<()> {
    let mut buf = [0u8; 8192];
    let n = stream.read(&mut buf).await?;

    let request = String::from_utf8_lossy(&buf[..n]);

    if request.starts_with("CONNECT") {
        handle_connect_request(stream, &request, rules, enhanced_rules, cert_gen, http2_live).await
    } else {
        handle_http_request(stream, &buf[..n], rules, enhanced_rules).await
    }
}

async fn handle_connect_request(
    mut stream: TcpStream,
    request: &str,
    rules: Arc<RwLock<RuleSet>>,
    _enhanced_rules: Arc<RwLock<EnhancedRuleSet>>,
    cert_gen: CertGenerator,
    http2_live: bool,
) -> Result<()> {
    let host_port = request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .ok_or_else(|| EggsecError::Proxy("Invalid CONNECT request".to_string()))?;

    let (host, port) = if let Some(idx) = host_port.rfind(':') {
        (
            &host_port[..idx],
            host_port[idx + 1..].parse().unwrap_or(443),
        )
    } else {
        (host_port, 443u16)
    };

    validate_target(host, port)?;

    let rule_action = {
        let rules = rules.read();
        rules.evaluate(host, "/")
    };

    match rule_action {
        RuleAction::Block => {
            let response = b"HTTP/1.1 403 Forbidden\r\n\r\n";
            stream.write_all(response).await?;
            return Ok(());
        }
        RuleAction::Modify
        | RuleAction::Intercept
        | RuleAction::Monitor
        | RuleAction::Allow
        | RuleAction::InjectResponse
        | RuleAction::Delay
        | RuleAction::Tag => {
            let response = b"HTTP/1.1 200 Connection Established\r\n\r\n";
            stream.write_all(response).await?;
        }
    }

    let mut upstream = timeout(
        Duration::from_secs(30),
        TcpStream::connect(format!("{}:{}", host, port)),
    )
    .await
    .map_err(|e| EggsecError::Network(format!("Connection timeout to upstream: {}", e)))?
    .map_err(|e| EggsecError::Network(format!("Failed to connect to upstream: {}", e)))?;

    let material = cert_gen
        .generate_for_host(host)
        .map_err(|e| EggsecError::Proxy(format!("Cert generation failed: {}", e)))?;

    let tls_acceptor = create_tls_acceptor(&material)
        .map_err(|e| EggsecError::Proxy(format!("TLS config failed: {}", e)))?;

    let mut client_stream =
        match timeout(Duration::from_secs(30), tls_acceptor.accept(stream)).await {
            Ok(Ok(s)) => s,
            Ok(Err(e)) => {
                tracing::debug!("TLS accept failed: {}", e);
                return Ok(());
            }
            Err(_) => {
                tracing::debug!("TLS accept timeout");
                return Ok(());
            }
        };

    // Check if HTTP/2 was negotiated via ALPN
    let alpn = client_stream.get_ref().1.alpn_protocol();
    if http2_live && alpn == Some(b"h2") {
        tracing::debug!("HTTP/2 ALPN negotiated for {}:{}, dispatching to h2 interceptor", host, port);
        return handle_http2_interception(client_stream, upstream, host).await;
    }

    // Fall through to HTTP/1.1 handling
    let request_bytes = read_http_headers(&mut client_stream).await?;
    let request_str = String::from_utf8_lossy(&request_bytes);
    let headers = parse_ws_http_headers(&request_str);
    let ws_path = extract_ws_path(&request_str);

    if is_websocket_upgrade(&headers) {
        tracing::debug!("WebSocket upgrade detected for {}{}", host, ws_path);
        return handle_websocket_interception(client_stream, upstream, host, ws_path).await;
    }

    upstream
        .write_all(&request_bytes)
        .await
        .map_err(|e| EggsecError::Proxy(format!("Failed to forward request to upstream: {}", e)))?;

    let (mut client_read, mut client_write) = tokio::io::split(&mut client_stream);
    let (mut upstream_read, mut upstream_write) = tokio::io::split(upstream);

    let client_to_upstream = tokio::io::copy(&mut client_read, &mut upstream_write);
    let upstream_to_client = tokio::io::copy(&mut upstream_read, &mut client_write);

    tokio::select! {
        result = client_to_upstream => {
            if let Err(e) = result {
                tracing::debug!("Client to upstream copy error: {}", e);
            }
        }
        result = upstream_to_client => {
            if let Err(e) = result {
                tracing::debug!("Upstream to client copy error: {}", e);
            }
        }
    }

    Ok(())
}

async fn handle_http_request(
    mut stream: TcpStream,
    data: &[u8],
    rules: Arc<RwLock<RuleSet>>,
    enhanced_rules: Arc<RwLock<EnhancedRuleSet>>,
) -> Result<()> {
    let request_str = String::from_utf8_lossy(data);

    let (host, path) = parse_request_line(&request_str);

    let rule_action = {
        let rules = rules.read();
        rules.evaluate(host, path)
    };

    match rule_action {
        RuleAction::Block => {
            let response = b"HTTP/1.1 403 Forbidden\r\n\r\n";
            stream.write_all(response).await?;
            return Ok(());
        }
        RuleAction::Modify => {
            tracing::debug!("HTTP {} {} - Modify", host, path);
        }
        RuleAction::Intercept | RuleAction::Monitor | RuleAction::InjectResponse | RuleAction::Delay | RuleAction::Tag => {
            tracing::debug!("HTTP {} {} - {:?}", host, path, rule_action);
        }
        RuleAction::Allow => {}
    }

    let method = request_str
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().next())
        .unwrap_or("GET");

    let mut headers = HashMap::new();
    let mut body_start = None;
    for (i, line) in request_str.lines().enumerate() {
        if i == 0 {
            continue;
        }
        if line.is_empty() {
            body_start = Some(i + 1);
            break;
        }
        if let Some(colon_idx) = line.find(':') {
            let key = line[..colon_idx].trim().to_string();
            let value = line[colon_idx + 1..].trim().to_string();
            headers.insert(key, value);
        }
    }

    let request_body = body_start.map(|start| {
        request_str
            .lines()
            .skip(start)
            .collect::<Vec<_>>()
            .join("\n")
    });

    let delay_ms = {
        let enhanced_rules_guard = enhanced_rules.read();
        let ctx = build_rule_context(host, path, method, &headers, request_body.as_deref(), "http1");
        enhanced_rules_guard
            .evaluate_first(&ctx)
            .and_then(|r| r.delay_ms)
    };

    if let Some(ms) = delay_ms {
        tracing::debug!("Enhanced rule delay: {}ms", ms);
        tokio::time::sleep(Duration::from_millis(ms)).await;
    }

    let response = b"HTTP/1.1 400 Bad Request\r\n\r\n";
    stream.write_all(response).await?;
    Ok(())
}

fn parse_request_line(request: &str) -> (&str, &str) {
    request
        .lines()
        .next()
        .and_then(|line| {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let uri = parts[1];
                if let Some(slash_idx) = uri.find("://") {
                    let after_scheme = &uri[slash_idx + 3..];
                    if let Some(path_start) = after_scheme.find('/') {
                        let host = &after_scheme[..path_start];
                        let path = &after_scheme[path_start..];
                        return Some((host, path));
                    } else {
                        return Some((after_scheme, "/"));
                    }
                }
                let path = if uri.starts_with('/') { uri } else { "/" };
                Some(("", path))
            } else {
                None
            }
        })
        .unwrap_or(("", ""))
}

fn create_tls_acceptor(material: &CertMaterial) -> Result<TlsAcceptor> {
    let cert_der = CertificateDer::from(material.cert_der.clone());
    let key_der = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(material.key_der.clone()));

    let mut config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![cert_der], key_der)
        .map_err(|e| EggsecError::Proxy(format!("TLS config failed: {}", e)))?;

    // Enable ALPN for HTTP/2 negotiation (h2 preferred over http/1.1)
    config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];

    Ok(TlsAcceptor::from(Arc::new(config)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proxy_server_creation() {
        let server = ProxyServer::new("127.0.0.1:8080".parse().unwrap());
        assert!(server.is_ok());
    }

    #[test]
    fn test_rule_evaluation() {
        let rule = InterceptRule::new(
            "example.com".to_string(),
            Some("/admin".to_string()),
            RuleAction::Block,
        );
        assert!(matches!(rule.action, RuleAction::Block));
    }

    #[test]
    fn test_parse_request_line_absolute_uri() {
        let (host, path) = parse_request_line("GET http://example.com/path HTTP/1.1\r\n");
        assert_eq!(host, "example.com");
        assert_eq!(path, "/path");
    }

    #[test]
    fn test_parse_request_line_relative_uri() {
        let (host, path) = parse_request_line("GET /admin HTTP/1.1\r\n");
        assert_eq!(host, "");
        assert_eq!(path, "/admin");
    }

    #[test]
    fn test_parse_request_line_no_uri() {
        let (host, path) = parse_request_line("INVALID");
        assert_eq!(host, "");
        assert_eq!(path, "");
    }

    #[test]
    fn test_parse_request_line_absolute_uri_no_path() {
        let (host, path) = parse_request_line("GET http://example.com HTTP/1.1\r\n");
        assert_eq!(host, "example.com");
        assert_eq!(path, "/");
    }

    #[test]
    fn test_is_private_ip() {
        assert!(is_private_ip("127.0.0.1".parse().unwrap()));
        assert!(is_private_ip("10.0.0.1".parse().unwrap()));
        assert!(is_private_ip("172.16.0.1".parse().unwrap()));
        assert!(is_private_ip("192.168.1.1".parse().unwrap()));
        assert!(!is_private_ip("8.8.8.8".parse().unwrap()));
        assert!(is_private_ip("::1".parse().unwrap()));
        assert!(is_private_ip("fc00::1".parse().unwrap()));
        assert!(is_private_ip("fd00::1".parse().unwrap()));
    }

    #[test]
    fn test_build_rule_context() {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        let ctx = build_rule_context(
            "example.com",
            "/api/test",
            "POST",
            &headers,
            Some("{\"key\":\"value\"}"),
            "http1",
        );
        assert_eq!(ctx.host, "example.com");
        assert_eq!(ctx.path, "/api/test");
        assert_eq!(ctx.method, "POST");
        assert_eq!(ctx.protocol, "http1");
        assert_eq!(ctx.headers.get("Content-Type").unwrap(), "application/json");
        assert_eq!(ctx.body, Some("{\"key\":\"value\"}".to_string()));
        assert_eq!(ctx.body_size, Some(15));
    }

    #[test]
    fn test_build_rule_context_no_body() {
        let headers = HashMap::new();
        let ctx = build_rule_context("example.com", "/", "GET", &headers, None, "http1");
        assert_eq!(ctx.body, None);
        assert_eq!(ctx.body_size, None);
    }

    #[test]
    fn test_proxy_server_with_enhanced_rules() {
        use crate::proxy::intercept::rules::{RuleCondition, RuleAction};

        let server = ProxyServer::new("127.0.0.1:8080".parse().unwrap()).unwrap();
        server.add_enhanced_rule(
            EnhancedRule::new(
                "test-rule",
                "Test Rule",
                RuleCondition::HostMatches("example.com".to_string()),
                RuleAction::Block,
            )
        );
        let rules = server.enhanced_rules.read();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules.get_by_id("test-rule").unwrap().name, "Test Rule");
    }

    #[test]
    fn test_enhanced_rule_evaluation_in_context() {
        use crate::proxy::intercept::rules::{RuleCondition, RuleAction};

        let mut headers = HashMap::new();
        headers.insert("Host".to_string(), "example.com".to_string());

        let ctx = build_rule_context(
            "example.com",
            "/admin/api",
            "POST",
            &headers,
            Some("secret=data"),
            "http1",
        );

        let mut rule_set = EnhancedRuleSet::new();
        rule_set.add(
            EnhancedRule::new(
                "body-inspector",
                "Body Inspector",
                RuleCondition::BodyContains("secret".to_string()),
                RuleAction::Monitor,
            )
        );
        rule_set.add(
            EnhancedRule::new(
                "admin-block",
                "Admin Blocker",
                RuleCondition::And(vec![
                    RuleCondition::HostMatches("example.com".to_string()),
                    RuleCondition::PathMatches("/admin/*".to_string()),
                ]),
                RuleAction::Block,
            )
        );

        let matches = rule_set.evaluate(&ctx);
        assert_eq!(matches.len(), 2);

        let first = rule_set.evaluate_first(&ctx).unwrap();
        assert_eq!(first.id.as_str(), "body-inspector");
    }

    #[test]
    fn test_is_websocket_upgrade_with_header() {
        let mut headers = HashMap::new();
        headers.insert("upgrade".to_string(), "websocket".to_string());
        headers.insert("connection".to_string(), "Upgrade".to_string());
        assert!(is_websocket_upgrade(&headers));
    }

    #[test]
    fn test_is_websocket_upgrade_case_insensitive() {
        let mut headers = HashMap::new();
        headers.insert("Upgrade".to_string(), "WebSocket".to_string());
        assert!(is_websocket_upgrade(&headers));
    }

    #[test]
    fn test_is_websocket_upgrade_missing() {
        let headers = HashMap::new();
        assert!(!is_websocket_upgrade(&headers));
    }

    #[test]
    fn test_is_websocket_upgrade_wrong_value() {
        let mut headers = HashMap::new();
        headers.insert("upgrade".to_string(), "h2c".to_string());
        assert!(!is_websocket_upgrade(&headers));
    }

    #[test]
    fn test_parse_ws_http_headers() {
        let request = "GET /chat HTTP/1.1\r\nHost: example.com\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nSec-WebSocket-Version: 13\r\n\r\n";
        let headers = parse_ws_http_headers(request);
        assert_eq!(headers.get("Host").unwrap(), "example.com");
        assert_eq!(headers.get("Upgrade").unwrap(), "websocket");
        assert_eq!(headers.get("Connection").unwrap(), "Upgrade");
        assert_eq!(
            headers.get("Sec-WebSocket-Key").unwrap(),
            "dGhlIHNhbXBsZSBub25jZQ=="
        );
    }

    #[test]
    fn test_extract_ws_path() {
        assert_eq!(
            extract_ws_path("GET /chat HTTP/1.1\r\n"),
            "/chat"
        );
        assert_eq!(
            extract_ws_path("GET http://example.com/ws HTTP/1.1\r\n"),
            "/ws"
        );
        assert_eq!(
            extract_ws_path("GET / HTTP/1.1\r\n"),
            "/"
        );
    }

    #[test]
    fn test_tls_acceptor_has_alpn() {
        use crate::proxy::intercept::cert::CertGenerator;
        let _ = rustls::crypto::ring::default_provider().install_default();

        let gen = CertGenerator::new();
        let material = gen.generate_for_host("example.com").unwrap();
        let acceptor = create_tls_acceptor(&material).unwrap();
        let config = acceptor.config();

        assert_eq!(config.alpn_protocols.len(), 2);
        assert_eq!(config.alpn_protocols[0], b"h2");
        assert_eq!(config.alpn_protocols[1], b"http/1.1");
    }

    #[test]
    fn test_proxy_server_http2_live_default() {
        let server = ProxyServer::new("127.0.0.1:8080".parse().unwrap()).unwrap();
        assert!(!server.proxy_http2_live);
    }

    #[test]
    fn test_proxy_server_with_http2_live() {
        let server = ProxyServer::new("127.0.0.1:8080".parse().unwrap())
            .unwrap()
            .with_proxy_http2_live(true);
        assert!(server.proxy_http2_live);
    }

    #[test]
    fn test_http2_session_tracking() {
        let mut session = Http2Session::new("api.example.com", true);
        assert_eq!(session.host, "api.example.com");
        assert!(session.is_secure);
        assert!(session.streams.is_empty());

        let stream1 = Http2Stream::new(1, "GET", "/api/data");
        let stream2 = Http2Stream::new(3, "POST", "/api/submit");
        session.add_stream(stream1);
        session.add_stream(stream2);

        assert_eq!(session.streams.len(), 2);
        assert_eq!(session.streams[0].stream_id, 1);
        assert_eq!(session.streams[1].stream_id, 3);
    }

    #[test]
    fn test_http2_stream_request_headers() {
        let mut stream = Http2Stream::new(1, "GET", "/api/data");
        stream
            .request_headers
            .insert("accept".to_string(), "application/json".to_string());
        stream
            .request_headers
            .insert("authorization".to_string(), "Bearer token123".to_string());

        assert_eq!(stream.request_headers.len(), 2);
        assert_eq!(
            stream.request_headers.get("accept").unwrap(),
            "application/json"
        );
    }

    #[test]
    fn test_http2_stream_response_tracking() {
        let mut stream = Http2Stream::new(1, "GET", "/api/data");
        stream.response_status = 200;
        stream
            .response_headers
            .insert("content-type".to_string(), "application/json".to_string());
        stream.response_body = Some("{\"key\":\"value\"}".to_string());
        stream.response_body_size = 17;
        stream.state = Http2StreamState::Closed;

        assert_eq!(stream.response_status, 200);
        assert_eq!(stream.response_body_size, 17);
        assert_eq!(stream.state, Http2StreamState::Closed);
    }
}
