use crate::error::{EggsecError, Result};
use crate::types::SensitiveString;
use base64::{engine::general_purpose, Engine as _};
use std::net::SocketAddr;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::time::timeout;

use super::config::{ProxyEntry, ProxyType};
use super::ProxiedConnection;

#[allow(dead_code)]
pub struct HttpConnectProxy {
    proxy_addr: SocketAddr,
    username: Option<SensitiveString>,
    password: Option<SensitiveString>,
    #[deprecated(
        note = "use_ssl is not implemented - HTTPS proxy type determines TLS in the caller"
    )]
    use_ssl: bool,
    timeout: Duration,
}

#[allow(dead_code)]
impl HttpConnectProxy {
    pub fn new(proxy_addr: SocketAddr) -> Self {
        Self {
            proxy_addr,
            username: None,
            password: None,
            use_ssl: false,
            timeout: Duration::from_secs(30),
        }
    }

    pub fn with_auth(mut self, username: String, password: String) -> Self {
        self.username = Some(SensitiveString::new(username));
        self.password = Some(SensitiveString::new(password));
        self
    }

    pub fn with_ssl(mut self, use_ssl: bool) -> Self {
        self.use_ssl = use_ssl;
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub async fn connect(&self, target: SocketAddr) -> Result<TcpStream> {
        let mut stream = timeout(self.timeout, TcpStream::connect(self.proxy_addr))
            .await
            .map_err(|e| EggsecError::Proxy(format!("Connection timeout: {}", e)))?
            .map_err(|e| EggsecError::Proxy(format!("Failed to connect to proxy: {}", e)))?;

        let connect_request = self.build_connect_request(target);
        stream.write_all(connect_request.as_bytes()).await?;

        let response = self.read_response(&mut stream).await?;
        self.parse_response(&response)?;

        Ok(stream)
    }

    pub async fn connect_with_host(&self, host: &str, port: u16) -> Result<TcpStream> {
        let mut stream = timeout(self.timeout, TcpStream::connect(self.proxy_addr))
            .await
            .map_err(|e| EggsecError::Proxy(format!("Connection timeout: {}", e)))?
            .map_err(|e| EggsecError::Proxy(format!("Failed to connect to proxy: {}", e)))?;

        let connect_request = self.build_connect_request_with_host(host, port);
        stream.write_all(connect_request.as_bytes()).await?;

        let response = self.read_response(&mut stream).await?;
        self.parse_response(&response)?;

        Ok(stream)
    }

    fn build_connect_request(&self, target: SocketAddr) -> String {
        let mut request = format!(
            "CONNECT {}:{} HTTP/1.1\r\nHost: {}:{}\r\n",
            target.ip(),
            target.port(),
            target.ip(),
            target.port()
        );

        if let (Some(user), Some(pass)) = (&self.username, &self.password) {
            let credentials = general_purpose::STANDARD.encode(format!(
                "{}:{}",
                user.expose_secret(),
                pass.expose_secret()
            ));
            request.push_str(&format!("Proxy-Authorization: Basic {}\r\n", credentials));
        }

        request.push_str("Proxy-Connection: Keep-Alive\r\n");
        request.push_str("\r\n");

        request
    }

    fn build_connect_request_with_host(&self, host: &str, port: u16) -> String {
        let mut request = format!(
            "CONNECT {}:{} HTTP/1.1\r\nHost: {}:{}\r\n",
            host, port, host, port
        );

        if let (Some(user), Some(pass)) = (&self.username, &self.password) {
            let credentials = general_purpose::STANDARD.encode(format!(
                "{}:{}",
                user.expose_secret(),
                pass.expose_secret()
            ));
            request.push_str(&format!("Proxy-Authorization: Basic {}\r\n", credentials));
        }

        request.push_str("Proxy-Connection: Keep-Alive\r\n");
        request.push_str("\r\n");

        request
    }

    async fn read_response(&self, stream: &mut TcpStream) -> Result<String> {
        let mut response = String::new();
        let mut reader = BufReader::with_capacity(8192, stream);
        let mut buf = [0u8; 8192];
        let mut seen_cr_lf_cr_lf = 0;

        let result = timeout(self.timeout, async {
            loop {
                let bytes_read = reader.read(&mut buf).await?;
                if bytes_read == 0 {
                    if response.is_empty() {
                        return Err(EggsecError::Proxy("Empty response from proxy".to_string()));
                    }
                    break;
                }

                for &byte in &buf[..bytes_read] {
                    let ch = byte as char;
                    response.push(ch);

                    match ch {
                        '\r' => {
                            if seen_cr_lf_cr_lf == 0 || seen_cr_lf_cr_lf == 2 {
                                seen_cr_lf_cr_lf += 1;
                            } else {
                                seen_cr_lf_cr_lf = 1;
                            }
                        }
                        '\n' => {
                            if seen_cr_lf_cr_lf == 1 {
                                seen_cr_lf_cr_lf = 2;
                            } else if seen_cr_lf_cr_lf == 3 {
                                return Ok(response);
                            }
                        }
                        _ => {
                            seen_cr_lf_cr_lf = 0;
                        }
                    }
                }
            }
            Ok(response)
        })
        .await;

        match result {
            Ok(Ok(response)) => Ok(response),
            Ok(Err(e)) => Err(e),
            Err(_) => Err(EggsecError::Proxy(
                "Timeout reading proxy response".to_string(),
            )),
        }
    }

    fn parse_response(&self, response: &str) -> Result<()> {
        let first_line = response
            .lines()
            .next()
            .ok_or_else(|| EggsecError::Proxy("Empty response from proxy".to_string()))?;

        let parts: Vec<&str> = first_line.splitn(3, ' ').collect();
        if parts.len() < 2 {
            return Err(EggsecError::Proxy(format!(
                "Invalid response from proxy: {}",
                first_line
            )));
        }

        let status_code: u16 = parts[1]
            .parse()
            .map_err(|e| EggsecError::Proxy(format!("Invalid status code: {}: {}", parts[1], e)))?;

        if !(200..300).contains(&status_code) {
            return Err(EggsecError::Proxy(format!(
                "Proxy returned error: {} {}",
                status_code,
                parts.get(2).unwrap_or(&"")
            )));
        }

        Ok(())
    }
}

pub async fn connect_through(proxy: ProxyEntry, target: SocketAddr) -> Result<ProxiedConnection> {
    let proxy_addr = proxy.socket_addr()?;
    let use_ssl = proxy.proxy_type == ProxyType::Https;

    let http_proxy = HttpConnectProxy::new(proxy_addr)
        .with_ssl(use_ssl)
        .with_timeout(Duration::from_millis(proxy.timeout_ms));

    let http_proxy = if let (Some(user), Some(pass)) = (&proxy.username, &proxy.password) {
        http_proxy.with_auth(user.clone(), pass.expose_secret().to_string())
    } else {
        http_proxy
    };

    let stream = http_proxy.connect(target).await?;
    let local_addr = stream.local_addr()?;

    Ok(ProxiedConnection {
        proxy_chain: vec![proxy],
        local_addr,
        target_addr: target,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_connect_request_without_auth() {
        let proxy = HttpConnectProxy::new("127.0.0.1:8080".parse().unwrap());
        let target: SocketAddr = "10.0.0.1:443".parse().unwrap();
        let request = proxy.build_connect_request(target);

        assert!(request.starts_with("CONNECT 10.0.0.1:443 HTTP/1.1\r\n"));
        assert!(request.contains("Host: 10.0.0.1:443\r\n"));
        assert!(request.contains("Proxy-Connection: Keep-Alive\r\n"));
        assert!(!request.contains("Proxy-Authorization"));
    }

    #[test]
    fn test_build_connect_request_with_auth() {
        let proxy = HttpConnectProxy::new("127.0.0.1:8080".parse().unwrap())
            .with_auth("user".to_string(), "pass".to_string());
        let target: SocketAddr = "10.0.0.1:443".parse().unwrap();
        let request = proxy.build_connect_request(target);

        assert!(request.contains("Proxy-Authorization: Basic "));
        let expected_creds = general_purpose::STANDARD.encode("user:pass");
        assert!(request.contains(&expected_creds));
    }

    #[test]
    fn test_build_connect_request_with_host() {
        let proxy = HttpConnectProxy::new("127.0.0.1:8080".parse().unwrap());
        let request = proxy.build_connect_request_with_host("example.com", 443);

        assert!(request.starts_with("CONNECT example.com:443 HTTP/1.1\r\n"));
        assert!(request.contains("Host: example.com:443\r\n"));
    }

    #[test]
    fn test_parse_response_success() {
        let proxy = HttpConnectProxy::new("127.0.0.1:8080".parse().unwrap());
        let response = "HTTP/1.1 200 Connection Established\r\n\r\n";
        assert!(proxy.parse_response(response).is_ok());
    }

    #[test]
    fn test_parse_response_201() {
        let proxy = HttpConnectProxy::new("127.0.0.1:8080".parse().unwrap());
        let response = "HTTP/1.1 201 Created\r\n\r\n";
        assert!(proxy.parse_response(response).is_ok());
    }

    #[test]
    fn test_parse_response_403() {
        let proxy = HttpConnectProxy::new("127.0.0.1:8080".parse().unwrap());
        let response = "HTTP/1.1 403 Forbidden\r\n\r\n";
        let result = proxy.parse_response(response);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("403"));
    }

    #[test]
    fn test_parse_response_500() {
        let proxy = HttpConnectProxy::new("127.0.0.1:8080".parse().unwrap());
        let response = "HTTP/1.1 500 Internal Server Error\r\n\r\n";
        let result = proxy.parse_response(response);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_response_empty() {
        let proxy = HttpConnectProxy::new("127.0.0.1:8080".parse().unwrap());
        let result = proxy.parse_response("");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Empty response"));
    }

    #[test]
    fn test_parse_response_invalid_format() {
        let proxy = HttpConnectProxy::new("127.0.0.1:8080".parse().unwrap());
        let result = proxy.parse_response("INVALID");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_response_invalid_status_code() {
        let proxy = HttpConnectProxy::new("127.0.0.1:8080".parse().unwrap());
        let result = proxy.parse_response("HTTP/1.1 abc Bad\r\n\r\n");
        assert!(result.is_err());
    }

    #[test]
    fn test_builder_pattern() {
        let proxy = HttpConnectProxy::new("127.0.0.1:3128".parse().unwrap())
            .with_auth("admin".to_string(), "secret".to_string())
            .with_ssl(true)
            .with_timeout(Duration::from_secs(10));

        assert_eq!(proxy.use_ssl, true);
        assert_eq!(proxy.timeout, Duration::from_secs(10));
        assert!(proxy.username.is_some());
        assert!(proxy.password.is_some());
    }
}
