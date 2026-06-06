use crate::error::{Result, SlapperError};
use crate::types::SensitiveString;
use crate::utils::connect_with_nodelay_timeout;
use std::net::{IpAddr, SocketAddr};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::Duration;

use super::config::{ProxyEntry, ProxyType};
use super::ProxiedConnection;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum SocksVersion {
    V4,
    V4a,
    V5,
}

pub struct SocksProxy {
    version: SocksVersion,
    proxy_addr: SocketAddr,
    username: Option<SensitiveString>,
    password: Option<SensitiveString>,
    timeout: Duration,
}

impl SocksProxy {
    pub fn new(version: SocksVersion, proxy_addr: SocketAddr) -> Self {
        Self {
            version,
            proxy_addr,
            username: None,
            password: None,
            timeout: Duration::from_secs(30),
        }
    }

    pub fn with_auth(mut self, username: String, password: String) -> Self {
        self.username = Some(SensitiveString::new(username));
        self.password = Some(SensitiveString::new(password));
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub async fn connect(&self, target: SocketAddr) -> Result<TcpStream> {
        match self.version {
            SocksVersion::V4 | SocksVersion::V4a => self.connect_socks4(target).await,
            SocksVersion::V5 => self.connect_socks5(target).await,
        }
    }

    pub async fn connect_with_domain(&self, domain: &str, port: u16) -> Result<TcpStream> {
        match self.version {
            SocksVersion::V4a => self.connect_socks4a(domain, port).await,
            SocksVersion::V5 => self.connect_socks5_domain(domain, port).await,
            SocksVersion::V4 => Err(SlapperError::Proxy(
                "SOCKS4 requires IP address, use SOCKS4a for domain support".to_string(),
            )),
        }
    }

    async fn connect_socks4(&self, target: SocketAddr) -> Result<TcpStream> {
        let mut stream = connect_with_nodelay_timeout(&self.proxy_addr, self.timeout)
            .await
            .map_err(|e| SlapperError::Proxy(format!("Failed to connect to proxy: {}", e)))?;

        let ip = match target.ip() {
            IpAddr::V4(ip) => ip,
            IpAddr::V6(_) => {
                return Err(SlapperError::Proxy(
                    "SOCKS4 does not support IPv6".to_string(),
                ))
            }
        };

        let mut request = vec![
            0x04,
            0x01,
            (target.port() >> 8) as u8,
            (target.port() & 0xFF) as u8,
        ];
        request.extend_from_slice(&ip.octets());
        request.push(0x00);

        stream.write_all(&request).await?;

        let mut response = [0u8; 8];
        stream.read_exact(&mut response).await?;

        if response[1] != 0x5A {
            return Err(SlapperError::Proxy(format!(
                "SOCKS4 proxy rejected connection: status 0x{:02X}",
                response[1]
            )));
        }

        Ok(stream)
    }

    async fn connect_socks4a(&self, domain: &str, port: u16) -> Result<TcpStream> {
        let mut stream = connect_with_nodelay_timeout(&self.proxy_addr, self.timeout)
            .await
            .map_err(|e| SlapperError::Proxy(format!("Failed to connect to proxy: {}", e)))?;

        let mut request = vec![
            0x04,
            0x01,
            (port >> 8) as u8,
            (port & 0xFF) as u8,
            0x00,
            0x00,
            0x00,
            0x01,
            0x00,
        ];
        request.extend_from_slice(domain.as_bytes());
        request.push(0x00);

        stream.write_all(&request).await?;

        let mut response = [0u8; 8];
        stream.read_exact(&mut response).await?;

        if response[1] != 0x5A {
            return Err(SlapperError::Proxy(format!(
                "SOCKS4a proxy rejected connection: status 0x{:02X}",
                response[1]
            )));
        }

        Ok(stream)
    }

    async fn connect_socks5(&self, target: SocketAddr) -> Result<TcpStream> {
        let mut stream = connect_with_nodelay_timeout(&self.proxy_addr, self.timeout)
            .await
            .map_err(|e| SlapperError::Proxy(format!("Failed to connect to proxy: {}", e)))?;

        self.socks5_handshake(&mut stream).await?;
        self.socks5_connect(&mut stream, &target.ip(), target.port())
            .await?;

        Ok(stream)
    }

    async fn connect_socks5_domain(&self, domain: &str, port: u16) -> Result<TcpStream> {
        let mut stream = connect_with_nodelay_timeout(&self.proxy_addr, self.timeout)
            .await
            .map_err(|e| SlapperError::Proxy(format!("Failed to connect to proxy: {}", e)))?;

        self.socks5_handshake(&mut stream).await?;
        self.socks5_connect_domain(&mut stream, domain, port)
            .await?;

        Ok(stream)
    }

    async fn socks5_handshake(&self, stream: &mut TcpStream) -> Result<()> {
        let auth_methods = match (&self.username, &self.password) {
            (Some(_), Some(_)) => vec![0x00, 0x02],
            _ => vec![0x00],
        };

        let mut request = vec![0x05, auth_methods.len() as u8];
        request.extend(auth_methods);

        stream.write_all(&request).await?;

        let mut response = [0u8; 2];
        stream.read_exact(&mut response).await?;

        if response[0] != 0x05 {
            return Err(SlapperError::Proxy("Invalid SOCKS5 response".to_string()));
        }

        match response[1] {
            0x00 => {}
            0x02 => {
                self.socks5_auth(stream).await?;
            }
            0xFF => {
                return Err(SlapperError::Proxy(
                    "SOCKS5 proxy: no acceptable authentication method".to_string(),
                ));
            }
            method => {
                return Err(SlapperError::Proxy(format!(
                    "SOCKS5 proxy: unsupported auth method 0x{:02X}",
                    method
                )));
            }
        }

        Ok(())
    }

    async fn socks5_auth(&self, stream: &mut TcpStream) -> Result<()> {
        let username = self
            .username
            .as_ref()
            .ok_or_else(|| SlapperError::Proxy("Username required".to_string()))?;
        let password = self
            .password
            .as_ref()
            .ok_or_else(|| SlapperError::Proxy("Password required".to_string()))?;

        if username.len() > 255 || password.len() > 255 {
            return Err(SlapperError::Proxy(
                "Username or password too long".to_string(),
            ));
        }

        let mut request = vec![0x01, username.len() as u8];
        request.extend(username.as_bytes());
        request.push(password.len() as u8);
        request.extend(password.as_bytes());

        stream.write_all(&request).await?;

        let mut response = [0u8; 2];
        stream.read_exact(&mut response).await?;

        if response[1] != 0x00 {
            return Err(SlapperError::Proxy(
                "SOCKS5 authentication failed".to_string(),
            ));
        }

        Ok(())
    }

    async fn socks5_connect(&self, stream: &mut TcpStream, ip: &IpAddr, port: u16) -> Result<()> {
        let mut request = vec![0x05, 0x01, 0x00];

        match ip {
            IpAddr::V4(ip) => {
                request.push(0x01);
                request.extend_from_slice(&ip.octets());
            }
            IpAddr::V6(ip) => {
                request.push(0x04);
                request.extend_from_slice(&ip.octets());
            }
        }

        request.push((port >> 8) as u8);
        request.push((port & 0xFF) as u8);

        stream.write_all(&request).await?;

        let mut response = [0u8; 10];
        stream.read_exact(&mut response[..4]).await?;

        if response[1] != 0x00 {
            return Err(map_socks5_error(response[1]));
        }

        let bind_addr_len = match response[3] {
            0x01 => 4,
            0x04 => 16,
            0x03 => {
                let mut len = [0u8; 1];
                stream.read_exact(&mut len).await?;
                len[0] as usize + 1
            }
            _ => {
                return Err(SlapperError::Proxy(
                    "Invalid address type in SOCKS5 response".to_string(),
                ))
            }
        };

        let mut remaining = vec![0u8; bind_addr_len + 2];
        stream.read_exact(&mut remaining).await?;

        Ok(())
    }

    async fn socks5_connect_domain(
        &self,
        stream: &mut TcpStream,
        domain: &str,
        port: u16,
    ) -> Result<()> {
        if domain.len() > 255 {
            return Err(SlapperError::Proxy("Domain name too long".to_string()));
        }

        let mut request = vec![0x05, 0x01, 0x00, 0x03];
        request.push(domain.len() as u8);
        request.extend(domain.as_bytes());
        request.push((port >> 8) as u8);
        request.push((port & 0xFF) as u8);

        stream.write_all(&request).await?;

        let mut response = [0u8; 4];
        stream.read_exact(&mut response).await?;

        if response[1] != 0x00 {
            return Err(map_socks5_error(response[1]));
        }

        let bind_addr_len = match response[3] {
            0x01 => 4,
            0x04 => 16,
            0x03 => {
                let mut len = [0u8; 1];
                stream.read_exact(&mut len).await?;
                len[0] as usize + 1
            }
            _ => {
                return Err(SlapperError::Proxy(
                    "Invalid address type in SOCKS5 response".to_string(),
                ))
            }
        };

        let mut remaining = vec![0u8; bind_addr_len + 2];
        stream.read_exact(&mut remaining).await?;

        Ok(())
    }
}

fn map_socks5_error(code: u8) -> SlapperError {
    match code {
        0x01 => SlapperError::Proxy("SOCKS5: General failure".to_string()),
        0x02 => SlapperError::Proxy("SOCKS5: Connection not allowed by ruleset".to_string()),
        0x03 => SlapperError::Proxy("SOCKS5: Network unreachable".to_string()),
        0x04 => SlapperError::Proxy("SOCKS5: Host unreachable".to_string()),
        0x05 => SlapperError::Proxy("SOCKS5: Connection refused".to_string()),
        0x06 => SlapperError::Proxy("SOCKS5: TTL expired".to_string()),
        0x07 => SlapperError::Proxy("SOCKS5: Command not supported".to_string()),
        0x08 => SlapperError::Proxy("SOCKS5: Address type not supported".to_string()),
        _ => SlapperError::Proxy(format!("SOCKS5: Unknown error 0x{:02X}", code)),
    }
}

pub async fn connect_through(proxy: ProxyEntry, target: SocketAddr) -> Result<ProxiedConnection> {
    let version = match proxy.proxy_type {
        ProxyType::Socks4 => SocksVersion::V4,
        ProxyType::Socks5 | ProxyType::Tor => SocksVersion::V5,
        _ => return Err(SlapperError::Proxy("Not a SOCKS proxy".to_string())),
    };

    let proxy_addr = proxy.socket_addr()?;
    let socks = SocksProxy::new(version, proxy_addr);

    let socks = if let (Some(user), Some(pass)) = (&proxy.username, &proxy.password) {
        socks.with_auth(user.clone(), pass.expose_secret().to_string())
    } else {
        socks
    };

    let socks = socks.with_timeout(Duration::from_millis(proxy.timeout_ms));

    let stream = socks.connect(target).await?;
    let local_addr = stream.local_addr()?;

    Ok(ProxiedConnection {
        proxy_chain: vec![proxy],
        local_addr,
        target_addr: target,
    })
}

/// Connects through a Tor proxy using SOCKS5.
/// Currently a pass-through to [`connect_through`] since Tor handles .onion
/// address resolution via standard SOCKS5 domain resolution.
pub async fn connect_through_tor(
    proxy: ProxyEntry,
    target: SocketAddr,
) -> Result<ProxiedConnection> {
    connect_through(proxy, target).await
}

pub async fn connect_through_with_domain(
    proxy: &ProxyEntry,
    domain: &str,
    port: u16,
) -> Result<TcpStream> {
    match proxy.proxy_type {
        ProxyType::Socks5 | ProxyType::Tor => {}
        ProxyType::Socks4 => {
            return Err(SlapperError::Proxy(
                "SOCKS4 does not support domain resolution. Use an IP address or configure a SOCKS5 proxy.".to_string(),
            ));
        }
        _ => {
            return Err(SlapperError::Proxy(format!(
                "Domain resolution not supported for proxy type {:?}",
                proxy.proxy_type
            )));
        }
    }

    let proxy_addr = proxy.socket_addr()?;

    let socks = SocksProxy::new(SocksVersion::V5, proxy_addr)
        .with_timeout(Duration::from_millis(proxy.timeout_ms));

    let socks = if let (Some(user), Some(pass)) = (&proxy.username, &proxy.password) {
        socks.with_auth(user.clone(), pass.expose_secret().to_string())
    } else {
        socks
    };

    socks.connect_with_domain(domain, port).await
}

pub async fn chain_connect(proxies: &[ProxyEntry], target: SocketAddr) -> Result<TcpStream> {
    if proxies.is_empty() {
        return Err(SlapperError::Proxy("No proxies in chain".to_string()));
    }

    for proxy in proxies {
        if !matches!(proxy.proxy_type, ProxyType::Socks5 | ProxyType::Tor) {
            return Err(SlapperError::Proxy(
                "SOCKS proxy chaining currently supports only SOCKS5/Tor entries".to_string(),
            ));
        }
    }

    let first = &proxies[0];
    let first_addr = first.socket_addr()?;
    let mut stream =
        connect_with_nodelay_timeout(&first_addr, Duration::from_millis(first.timeout_ms))
            .await
            .map_err(|e| SlapperError::Proxy(format!("Failed to connect to proxy: {}", e)))?;

    for (i, proxy) in proxies.iter().enumerate() {
        let hop = SocksProxy::new(SocksVersion::V5, proxy.socket_addr()?)
            .with_timeout(Duration::from_millis(proxy.timeout_ms));
        let hop = if let (Some(user), Some(pass)) = (&proxy.username, &proxy.password) {
            hop.with_auth(user.clone(), pass.expose_secret().to_string())
        } else {
            hop
        };

        hop.socks5_handshake(&mut stream).await?;

        if i == proxies.len() - 1 {
            hop.socks5_connect(&mut stream, &target.ip(), target.port())
                .await?;
        } else {
            let next = proxies[i + 1].socket_addr()?;
            hop.socks5_connect(&mut stream, &next.ip(), next.port())
                .await?;
        }
    }

    Ok(stream)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_socks_proxy_new() {
        let addr: SocketAddr = "127.0.0.1:1080".parse().unwrap();
        let proxy = SocksProxy::new(SocksVersion::V5, addr);
        assert!(proxy.username.is_none());
        assert!(proxy.password.is_none());
        assert_eq!(proxy.timeout, Duration::from_secs(30));
    }

    #[test]
    fn test_socks_proxy_with_auth() {
        let addr: SocketAddr = "127.0.0.1:1080".parse().unwrap();
        let proxy = SocksProxy::new(SocksVersion::V5, addr)
            .with_auth("user".to_string(), "pass".to_string());
        assert!(proxy.username.is_some());
        assert!(proxy.password.is_some());
    }

    #[test]
    fn test_socks_proxy_builder_chaining() {
        let addr: SocketAddr = "127.0.0.1:1080".parse().unwrap();
        let proxy = SocksProxy::new(SocksVersion::V4, addr)
            .with_auth("u".to_string(), "p".to_string())
            .with_timeout(Duration::from_millis(5000));
        assert!(proxy.username.is_some());
        assert_eq!(proxy.timeout, Duration::from_millis(5000));
    }

    #[test]
    fn test_socks_proxy_with_timeout() {
        let addr: SocketAddr = "127.0.0.1:1080".parse().unwrap();
        let proxy = SocksProxy::new(SocksVersion::V5, addr).with_timeout(Duration::from_secs(10));
        assert_eq!(proxy.timeout, Duration::from_secs(10));
    }

    #[test]
    fn test_socks_version_equality() {
        assert_eq!(SocksVersion::V4, SocksVersion::V4);
        assert_ne!(SocksVersion::V4, SocksVersion::V5);
        assert_ne!(SocksVersion::V4a, SocksVersion::V5);
    }

    #[test]
    fn test_map_socks5_error_all_codes() {
        let err = map_socks5_error(0x01);
        assert!(err.to_string().contains("General failure"));

        let err = map_socks5_error(0x02);
        assert!(err.to_string().contains("not allowed"));

        let err = map_socks5_error(0x03);
        assert!(err.to_string().contains("Network unreachable"));

        let err = map_socks5_error(0x04);
        assert!(err.to_string().contains("Host unreachable"));

        let err = map_socks5_error(0x05);
        assert!(err.to_string().contains("refused"));

        let err = map_socks5_error(0x06);
        assert!(err.to_string().contains("TTL expired"));

        let err = map_socks5_error(0x07);
        assert!(err.to_string().contains("not supported"));

        let err = map_socks5_error(0x08);
        assert!(err.to_string().contains("not supported"));

        let err = map_socks5_error(0xFF);
        assert!(err.to_string().contains("Unknown error"));
    }

    #[tokio::test]
    async fn test_connect_through_wrong_proxy_type() {
        let proxy = ProxyEntry::new(ProxyType::Http, "proxy.com".to_string(), 8080);
        let target: SocketAddr = "93.184.216.34:80".parse().unwrap();

        let result = connect_through(proxy, target).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Not a SOCKS proxy"));
    }

    #[tokio::test]
    async fn test_connect_through_empty_chain() {
        let proxies: Vec<ProxyEntry> = vec![];
        let target: SocketAddr = "93.184.216.34:80".parse().unwrap();

        let result = chain_connect(&proxies, target).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("No proxies in chain"));
    }

    #[tokio::test]
    async fn test_connect_through_invalid_address() {
        let proxy = ProxyEntry::new(ProxyType::Socks5, "not-an-ip".to_string(), 1080);
        let target: SocketAddr = "93.184.216.34:80".parse().unwrap();

        let result = connect_through(proxy, target).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_connect_through_with_domain_invalid_address() {
        let proxy = ProxyEntry::new(ProxyType::Socks5, "not-an-ip".to_string(), 1080);

        let result = connect_through_with_domain(&proxy, "example.com", 80).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_chain_connect_invalid_proxy_address() {
        let proxy = ProxyEntry::new(ProxyType::Socks5, "invalid".to_string(), 1080);
        let target: SocketAddr = "93.184.216.34:80".parse().unwrap();

        let result = chain_connect(&[proxy], target).await;
        assert!(result.is_err());
    }
}
