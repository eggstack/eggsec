//! FTP authentication testing
//!
//! Tests FTP authentication with password-based credentials.

use super::super::{AuthTestResult};
use crate::error::{Result, SlapperError};
use crate::types::Severity;
use std::time::Duration;

pub async fn test_ftp_auth(
    target: &str,
    port: u16,
    credentials: &[(String, String)],
    timeout: Duration,
) -> Result<Vec<AuthTestResult>> {
    let mut results = Vec::new();

    for (username, password) in credentials {
        let result = test_single_credential(target, port, username, password, timeout).await;
        results.push(result);
    }

    Ok(results)
}

async fn test_single_credential(
    target: &str,
    port: u16,
    username: &str,
    password: &str,
    timeout: Duration,
) -> AuthTestResult {
    let addr = format!("{}:{}", target, port);

    let result = tokio::time::timeout(
        timeout,
        ftp_auth_attempt(&addr, username, password),
    )
    .await;

    match result {
        Ok(Ok(success)) => AuthTestResult {
            protocol: "FTP".to_string(),
            target: target.to_string(),
            port,
            success,
            credentials_used: if success {
                Some((username.to_string(), password.to_string()))
            } else {
                None
            },
            severity: if success { Severity::Critical } else { Severity::Info },
            message: if success {
                "Authentication successful".to_string()
            } else {
                "Authentication failed".to_string()
            },
        },
        Ok(Err(e)) => AuthTestResult {
            protocol: "FTP".to_string(),
            target: target.to_string(),
            port,
            success: false,
            credentials_used: None,
            severity: Severity::Info,
            message: format!("Connection error: {}", e),
        },
        Err(_) => AuthTestResult {
            protocol: "FTP".to_string(),
            target: target.to_string(),
            port,
            success: false,
            credentials_used: None,
            severity: Severity::Info,
            message: "Connection timeout".to_string(),
        },
    }
}

async fn ftp_auth_attempt(addr: &str, username: &str, password: &str) -> Result<bool> {
    use std::net::TcpStream;
    use std::io::{Read, Write};

    let mut stream = TcpStream::connect(addr)
        .map_err(|e| SlapperError::Network(format!("TCP connection failed: {}", e)))?;

    stream.set_read_timeout(Some(Duration::from_secs(10)))
        .map_err(|e| SlapperError::Network(format!("Timeout set failed: {}", e)))?;

    let mut response = [0u8; 1024];
    stream.read(&mut response)
        .map_err(|e| SlapperError::Network(format!("Read failed: {}", e)))?;

    let response_str = String::from_utf8_lossy(&response);

    if !response_str.contains("220") {
        return Err(SlapperError::Network("Invalid FTP banner".to_string()));
    }

    let user_cmd = format!("USER {}\r\n", username);
    stream.write_all(user_cmd.as_bytes())
        .map_err(|e| SlapperError::Network(format!("Write failed: {}", e)))?;

    let mut response = [0u8; 1024];
    stream.read(&mut response)
        .map_err(|e| SlapperError::Network(format!("Read failed: {}", e)))?;

    let pass_cmd = format!("PASS {}\r\n", password);
    stream.write_all(pass_cmd.as_bytes())
        .map_err(|e| SlapperError::Network(format!("Write failed: {}", e)))?;

    let mut response = [0u8; 1024];
    let n = stream.read(&mut response)
        .map_err(|e| SlapperError::Network(format!("Read failed: {}", e)))?;

    let response_str = String::from_utf8_lossy(&response[..n]);
    Ok(response_str.contains("230"))
}

pub fn check_ftp_banner(address: &str, port: u16) -> Result<Option<String>> {
    use std::net::TcpStream;
    use std::io::{BufRead, BufReader};

    let addr = format!("{}:{}", address, port);
    let mut stream = TcpStream::connect(&addr)
        .map_err(|e| SlapperError::Network(format!("TCP connection failed: {}", e)))?;

    stream.set_read_timeout(Some(Duration::from_secs(5)))
        .map_err(|e| SlapperError::Network(format!("Timeout set failed: {}", e)))?;

    let reader = BufReader::new(stream);
    let mut lines = reader.lines();

    if let Some(Ok(line)) = lines.next() {
        if line.starts_with("220") {
            return Ok(Some(line));
        }
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_ftp_auth_result_structure() {
        let result = AuthTestResult {
            protocol: "FTP".to_string(),
            target: "example.com".to_string(),
            port: 21,
            success: false,
            credentials_used: None,
            severity: Severity::Info,
            message: "Connection refused".to_string(),
        };

        assert!(!result.success);
        assert_eq!(result.protocol, "FTP");
    }
}
