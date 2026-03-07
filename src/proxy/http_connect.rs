#![allow(dead_code)]

use std::net::SocketAddr;
use std::time::Duration;
use anyhow::{Result, Context, anyhow};
use base64::{Engine as _, engine::general_purpose};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::timeout;

use super::config::{ProxyEntry, ProxyType};
use super::ProxiedConnection;

pub struct HttpConnectProxy {
    proxy_addr: SocketAddr,
    username: Option<String>,
    password: Option<String>,
    use_ssl: bool,
    timeout: Duration,
}

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
        self.username = Some(username);
        self.password = Some(password);
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
            .context("Connection timeout")?
            .context("Failed to connect to proxy")?;
        
        let connect_request = self.build_connect_request(target);
        stream.write_all(connect_request.as_bytes()).await?;
        
        let response = self.read_response(&mut stream).await?;
        self.parse_response(&response)?;
        
        Ok(stream)
    }
    
    pub async fn connect_with_host(&self, host: &str, port: u16) -> Result<TcpStream> {
        let mut stream = timeout(self.timeout, TcpStream::connect(self.proxy_addr))
            .await
            .context("Connection timeout")?
            .context("Failed to connect to proxy")?;
        
        let connect_request = self.build_connect_request_with_host(host, port);
        stream.write_all(connect_request.as_bytes()).await?;
        
        let response = self.read_response(&mut stream).await?;
        self.parse_response(&response)?;
        
        Ok(stream)
    }
    
    fn build_connect_request(&self, target: SocketAddr) -> String {
        let mut request = format!(
            "CONNECT {}:{} HTTP/1.1\r\nHost: {}:{}\r\n",
            target.ip(), target.port(), target.ip(), target.port()
        );
        
        if let (Some(user), Some(pass)) = (&self.username, &self.password) {
            let credentials = general_purpose::STANDARD
                .encode(format!("{}:{}", user, pass));
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
            let credentials = general_purpose::STANDARD
                .encode(format!("{}:{}", user, pass));
            request.push_str(&format!("Proxy-Authorization: Basic {}\r\n", credentials));
        }
        
        request.push_str("Proxy-Connection: Keep-Alive\r\n");
        request.push_str("\r\n");
        
        request
    }
    
    async fn read_response(&self, stream: &mut TcpStream) -> Result<String> {
        let mut response = String::new();
        let mut buf = [0u8; 1];
        let mut seen_cr_lf_cr_lf = 0;
        
        loop {
            stream.read_exact(&mut buf).await?;
            let ch = buf[0] as char;
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
                        break;
                    }
                }
                _ => {
                    seen_cr_lf_cr_lf = 0;
                }
            }
        }
        
        Ok(response)
    }
    
    fn parse_response(&self, response: &str) -> Result<()> {
        let first_line = response
            .lines()
            .next()
            .ok_or_else(|| anyhow!("Empty response from proxy"))?;
        
        let parts: Vec<&str> = first_line.splitn(3, ' ').collect();
        if parts.len() < 2 {
            anyhow::bail!("Invalid response from proxy: {}", first_line);
        }
        
        let status_code: u16 = parts[1].parse()
            .with_context(|| format!("Invalid status code: {}", parts[1]))?;
        
        if status_code < 200 || status_code >= 300 {
            anyhow::bail!("Proxy returned error: {} {}", status_code, parts.get(2).unwrap_or(&""));
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
        http_proxy.with_auth(user.clone(), pass.clone())
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
