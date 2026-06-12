//! HTTP/HTTPS intercepting proxy with dynamic SSL certificate generation
//!
//! Provides an intercepting proxy for security testing with:
//! - Dynamic SSL certificate generation for HTTPS interception
//! - Request/response interception and modification
//! - Monitor mode for passive traffic logging
//! - Configurable interception rules

mod bridge;
mod cert;
mod interceptor;
mod rules;
pub mod types;

pub use bridge::to_scan_report_data_proxy;
pub use cert::{CertGenerator, CertMaterial};
pub use interceptor::{InterceptConfig, InterceptMode, InterceptProxy};
pub use rules::{InterceptRule, RuleAction, RuleSet};

use crate::error::{EggsecError, Result};
use parking_lot::RwLock;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
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
    mode: InterceptMode,
}

impl ProxyServer {
    pub fn new(addr: SocketAddr) -> Result<Self> {
        Ok(Self {
            addr,
            cert_generator: CertGenerator::new(),
            rules: Arc::new(RwLock::new(RuleSet::default())),
            mode: InterceptMode::Monitor,
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

    pub fn add_rule(&self, rule: InterceptRule) {
        let mut rules = self.rules.write();
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
                    let mode = self.mode;
                    let cert_gen = self.cert_generator.clone();

                    tokio::spawn(async move {
                        if let Err(e) =
                            handle_connection(stream, client_addr, rules, mode, cert_gen).await
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

async fn handle_connection(
    mut stream: TcpStream,
    _client_addr: SocketAddr,
    rules: Arc<RwLock<RuleSet>>,
    _mode: InterceptMode,
    cert_gen: CertGenerator,
) -> Result<()> {
    let mut buf = [0u8; 8192];
    let n = stream.read(&mut buf).await?;

    let request = String::from_utf8_lossy(&buf[..n]);

    if request.starts_with("CONNECT") {
        handle_connect_request(stream, &request, rules, cert_gen).await
    } else {
        handle_http_request(stream, &buf[..n], rules).await
    }
}

async fn handle_connect_request(
    mut stream: TcpStream,
    request: &str,
    rules: Arc<RwLock<RuleSet>>,
    cert_gen: CertGenerator,
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
        RuleAction::Modify | RuleAction::Intercept | RuleAction::Monitor | RuleAction::Allow => {
            let response = b"HTTP/1.1 200 Connection Established\r\n\r\n";
            stream.write_all(response).await?;
        }
    }

    let upstream = timeout(
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
        RuleAction::Intercept | RuleAction::Monitor => {
            tracing::debug!("HTTP {} {} - {:?}", host, path, rule_action);
        }
        RuleAction::Allow => {}
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

    let config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![cert_der], key_der)
        .map_err(|e| EggsecError::Proxy(format!("TLS config failed: {}", e)))?;

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
}
