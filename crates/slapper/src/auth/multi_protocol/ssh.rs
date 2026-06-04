//! SSH authentication testing
//!
//! Tests SSH authentication with password-based credentials.

use super::AuthTestResult;
use crate::error::{Result, SlapperError};
use crate::types::Severity;
use std::time::Duration;

pub async fn test_ssh_auth(
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
        ssh_auth_attempt(&addr, username, password),
    )
    .await;

    match result {
        Ok(Ok(success)) => AuthTestResult {
            protocol: "SSH".to_string(),
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
            protocol: "SSH".to_string(),
            target: target.to_string(),
            port,
            success: false,
            credentials_used: None,
            severity: Severity::Info,
            message: format!("Connection error: {}", e),
        },
        Err(_) => AuthTestResult {
            protocol: "SSH".to_string(),
            target: target.to_string(),
            port,
            success: false,
            credentials_used: None,
            severity: Severity::Info,
            message: "Connection timeout".to_string(),
        },
    }
}

async fn ssh_auth_attempt(addr: &str, username: &str, password: &str) -> Result<bool> {
    use std::net::TcpStream;
    use ssh2::Session;

    let tcp = TcpStream::connect(addr)
        .map_err(|e| SlapperError::Network(format!("TCP connection failed: {}", e)))?;

    let mut session = Session::new()
        .map_err(|e| SlapperError::Network(format!("SSH session creation failed: {}", e)))?;

    session.set_tcp_stream(tcp);
    session.handshake()
        .map_err(|e| SlapperError::Network(format!("SSH handshake failed: {}", e)))?;

    match session.userauth_password(username, password) {
        Ok(_) => Ok(session.authenticated()),
        Err(e) => Ok(false),
    }
}

pub fn check_ssh_banner(address: &str, port: u16) -> Result<Option<String>> {
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
        if line.starts_with("SSH-") {
            return Ok(Some(line));
        }
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_ssh_auth_result_structure() {
        let result = AuthTestResult {
            protocol: "SSH".to_string(),
            target: "example.com".to_string(),
            port: 22,
            success: false,
            credentials_used: None,
            severity: Severity::Info,
            message: "Connection refused".to_string(),
        };

        assert!(!result.success);
        assert_eq!(result.protocol, "SSH");
    }
}
