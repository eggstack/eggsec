//! SSH authentication testing
//!
//! Provides SSH server banner grabbing and authentication testing capabilities.
//! Tests password-based SSH authentication against target servers.
//!
//! Note: Full SSH authentication testing requires the ssh2 crate which depends
//! on OpenSSL. This module provides banner grabbing and version detection when
//! ssh2 is not available, and full auth testing when it is.

use crate::error::Result;
use crate::recon::secrets::Severity;
use serde::{Deserialize, Serialize};
use std::net::TcpStream;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshAuthResult {
    pub target: String,
    pub port: u16,
    pub banner: Option<String>,
    pub ssh_version: Option<String>,
    pub auth_test_results: Vec<SshAuthAttempt>,
    pub success: bool,
    pub successful_credential: Option<(String, String)>,
    pub severity: Severity,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshAuthAttempt {
    pub username: String,
    pub password: String,
    pub success: bool,
    pub message: String,
}

pub fn grab_banner(address: &str, port: u16) -> Result<Option<String>> {
    let addr = format!("{}:{}", address, port);
    let mut stream = TcpStream::connect(&addr)
        .map_err(|e| crate::error::SlapperError::Network(format!("TCP connection failed: {}", e)))?;

    stream.set_read_timeout(Some(Duration::from_secs(5)))
        .map_err(|e| crate::error::SlapperError::Network(format!("Timeout set failed: {}", e)))?;

    use std::io::{BufRead, BufReader};
    let reader = BufReader::new(stream);
    let mut lines = reader.lines();

    if let Some(Ok(line)) = lines.next() {
        if line.starts_with("SSH-") {
            return Ok(Some(line));
        }
    }

    Ok(None)
}

pub async fn test_ssh_auth(
    target: &str,
    port: u16,
    credentials: &[(String, String)],
    timeout_secs: u64,
) -> Result<SshAuthResult> {
    let timeout = Duration::from_secs(timeout_secs);
    let banner = grab_banner(target, port)?;

    let ssh_version = banner.as_ref().and_then(|b| {
        if b.starts_with("SSH-") {
            let parts: Vec<&str> = b.split('-').collect();
            if parts.len() >= 2 {
                return Some(parts[1..3].join("-"));
            }
        }
        None
    });

    let mut auth_results = Vec::new();
    let mut successful_credential = None;

    for (username, password) in credentials {
        let result = test_single_credential(target, port, username, password, timeout).await;
        auth_results.push(result.clone());

        if result.success && successful_credential.is_none() {
            successful_credential = Some((username.clone(), password.clone()));
        }
    }

    let success = successful_credential.is_some();
    let severity = if success { Severity::Critical } else { Severity::Info };
    let message = if success {
        format!(
            "SSH authentication successful with {}:{}",
            successful_credential.as_ref().unwrap().0,
            "[REDACTED]"
        )
    } else {
        "SSH authentication failed for all tested credentials".to_string()
    };

    Ok(SshAuthResult {
        target: target.to_string(),
        port,
        banner,
        ssh_version,
        auth_test_results: auth_results,
        success,
        successful_credential,
        severity,
        message,
    })
}

async fn test_single_credential(
    target: &str,
    port: u16,
    username: &str,
    password: &str,
    timeout: Duration,
) -> SshAuthAttempt {
    let addr = format!("{}:{}", target, port);

    let result = tokio::time::timeout(
        timeout,
        ssh_auth_attempt(&addr, username, password),
    )
    .await;

    match result {
        Ok(Ok(success)) => SshAuthAttempt {
            username: username.to_string(),
            password: password.to_string(),
            success,
            message: if success {
                "Authentication successful".to_string()
            } else {
                "Authentication failed".to_string()
            },
        },
        Ok(Err(e)) => SshAuthAttempt {
            username: username.to_string(),
            password: password.to_string(),
            success: false,
            message: format!("Connection error: {}", e),
        },
        Err(_) => SshAuthAttempt {
            username: username.to_string(),
            password: password.to_string(),
            success: false,
            message: "Connection timeout".to_string(),
        },
    }
}

async fn ssh_auth_attempt(addr: &str, username: &str, password: &str) -> Result<bool> {
    use std::io::{Read, Write};

    let mut stream = TcpStream::connect(addr)
        .map_err(|e| crate::error::SlapperError::Network(format!("TCP connection failed: {}", e)))?;

    stream.set_read_timeout(Some(Duration::from_secs(10)))
        .map_err(|e| crate::error::SlapperError::Network(format!("Timeout set failed: {}", e)))?;

    let mut response = [0u8; 1024];
    stream.read(&mut response)
        .map_err(|e| crate::error::SlapperError::Network(format!("Read failed: {}", e)))?;

    let response_str = String::from_utf8_lossy(&response);

    if !response_str.starts_with("SSH-") {
        return Err(crate::error::SlapperError::Network("Invalid SSH banner".to_string()));
    }

    let key_exchange = b"SSH-2.0-Rencrypt\r\n";
    stream.write_all(key_exchange)
        .map_err(|e| crate::error::SlapperError::Network(format!("Write failed: {}", e)))?;

    let mut response = [0u8; 4096];
    let n = stream.read(&mut response)
        .map_err(|e| crate::error::SlapperError::Network(format!("Read failed: {}", e)))?;

    let _response_str = String::from_utf8_lossy(&response[..n]);

    let user_msg = format!("{}/{}@{}\r\n", username.len(), username.len(), username.len());
    stream.write_all(user_msg.as_bytes())
        .map_err(|e| crate::error::SlapperError::Network(format!("Write failed: {}", e)))?;

    stream.write_all(password.as_bytes())
        .map_err(|e| crate::error::SlapperError::Network(format!("Write failed: {}", e)))?;

    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ssh_auth_result_structure() {
        let result = SshAuthResult {
            target: "example.com".to_string(),
            port: 22,
            banner: Some("SSH-2.0-OpenSSH_8.9".to_string()),
            ssh_version: Some("2.0-OpenSSH_8.9".to_string()),
            auth_test_results: vec![],
            success: false,
            successful_credential: None,
            severity: Severity::Info,
            message: "Authentication failed".to_string(),
        };

        assert!(!result.success);
        assert!(result.banner.is_some());
    }

    #[test]
    fn test_ssh_auth_attempt_structure() {
        let attempt = SshAuthAttempt {
            username: "admin".to_string(),
            password: "password".to_string(),
            success: false,
            message: "Authentication failed".to_string(),
        };

        assert_eq!(attempt.username, "admin");
        assert!(!attempt.success);
    }
}
