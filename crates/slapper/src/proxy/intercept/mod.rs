//! HTTP/HTTPS intercepting proxy with dynamic SSL certificate generation
//!
//! Provides an intercepting proxy for security testing with:
//! - Dynamic SSL certificate generation for HTTPS interception
//! - Request/response interception and modification
//! - Monitor mode for passive traffic logging
//! - Configurable interception rules

mod cert;
mod interceptor;
mod rules;

pub use cert::CertGenerator;
pub use interceptor::{InterceptConfig, InterceptMode, InterceptProxy};
pub use rules::{InterceptRule, RuleAction, RuleSet};

use crate::error::{Result, SlapperError};
use parking_lot::RwLock;
use rcgen::{Certificate, KeyPair};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_rustls::rustls::{ServerConfig, NoClientAuth};
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
        let listener = TcpListener::bind(self.addr).await
            .map_err(|e| SlapperError::Network(format!("Failed to bind proxy: {}", e)))?;

        tracing::info!("Proxy listening on {}", self.addr);

        loop {
            match listener.accept().await {
                Ok((stream, client_addr)) => {
                    let rules = Arc::clone(&self.rules);
                    let mode = self.mode.clone();
                    let cert_gen = self.cert_generator.clone();

                    tokio::spawn(async move {
                        if let Err(e) = handle_connection(stream, client_addr, rules, mode, cert_gen).await {
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

async fn handle_connection(
    stream: TcpStream,
    client_addr: SocketAddr,
    rules: Arc<RwLock<RuleSet>>,
    mode: InterceptMode,
    cert_gen: CertGenerator,
) -> Result<()> {
    let mut buf = [0u8; 8192];
    let n = tokio::io::BufReader::new(&stream).read(&mut buf).await?;

    let request = String::from_utf8_lossy(&buf[..n]);

    if request.starts_with("CONNECT") {
        handle_connect_request(stream, &request, rules, cert_gen).await
    } else {
        handle_http_request(stream, &buf[..n], rules).await
    }
}

async fn handle_connect_request(
    stream: TcpStream,
    request: &str,
    rules: Arc<RwLock<RuleSet>>,
    cert_gen: CertGenerator,
) -> Result<()> {
    let host_port = request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .ok_or_else(|| SlapperError::Proxy("Invalid CONNECT request".to_string()))?;

    let (host, port) = if let Some(idx) = host_port.rfind(':') {
        (&host_port[..idx], host_port[idx + 1..].parse().unwrap_or(443))
    } else {
        (host_port, 443u16)
    };

    let rule_action = {
        let rules = rules.read();
        rules.evaluate(host, "", "")
    };

    match rule_action {
        RuleAction::Block => {
            let response = b"HTTP/1.1 403 Forbidden\r\n\r\n";
            stream.write_all(response).await?;
            return Ok(());
        }
        RuleAction::Intercept => {
            let response = b"HTTP/1.1 200 Connection Established\r\n\r\n";
            stream.write_all(response).await?;
        }
        RuleAction::Monitor | RuleAction::Allow => {}
    }

    let upstream = TcpStream::connect(format!("{}:{}", host, port)).await
        .map_err(|e| SlapperError::Network(format!("Failed to connect to upstream: {}", e)))?;

    let cert = cert_gen.generate_for_host(host)
        .map_err(|e| SlapperError::Proxy(format!("Cert generation failed: {}", e)))?;

    let tls_acceptor = create_tls_acceptor(&cert)
        .map_err(|e| SlapperError::Proxy(format!("TLS config failed: {}", e)))?;

    let mut client_stream = match tls_acceptor.accept(stream).await {
        Ok(s) => s,
        Err(e) => {
            tracing::debug!("TLS accept failed: {}", e);
            return Ok(());
        }
    };

    let (mut client_read, mut client_write) = client_stream.split();
    let (mut upstream_read, mut upstream_write) = tokio::io::split(upstream);

    tokio::io::copy(&mut client_read, &mut upstream_write).await?;
    tokio::io::copy(&mut upstream_read, &mut client_write).await?;

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
        rules.evaluate(host, path, &request_str)
    };

    match rule_action {
        RuleAction::Block => {
            let response = b"HTTP/1.1 403 Forbidden\r\n\r\n";
            stream.write_all(response).await?;
            return Ok(());
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
                Some((parts.get(1).unwrap_or(&""), parts.get(1).unwrap_or(&"")))
            } else {
                None
            }
        })
        .unwrap_or(("", ""))
}

fn create_tls_acceptor(cert: &Certificate) -> Result<TlsAcceptor> {
    let cert_der = cert.serialize_der()
        .map_err(|e| SlapperError::Proxy(format!("Cert serialization failed: {}", e)))?;

    let key_der = cert.serialize_private_key_der();
    let key_pair = KeyPair::from_der(&key_der)
        .map_err(|e| SlapperError::Proxy(format!("Key pair creation failed: {}", e)))?;

    let cert_parsed = rustls::Certificate(cert_der);
    let key_parsed = rustls::PrivateKey(key_pair.serialize_der());

    let mut config = ServerConfig::new(NoClientAuth::new());
    config.set_single_cert(vec![cert_parsed], key_parsed)
        .map_err(|e| SlapperError::Proxy(format!("Cert configuration failed: {}", e)))?;

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
}
