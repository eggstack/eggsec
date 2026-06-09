//! FTP authentication testing
//!
//! Provides FTP server banner grabbing and authentication testing capabilities.
//! Tests password-based FTP authentication against target servers.

use crate::error::Result;
use crate::recon::secrets::Severity;
use serde::{Deserialize, Serialize};
use std::net::TcpStream;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FtpAuthResult {
    pub target: String,
    pub port: u16,
    pub banner: Option<String>,
    pub auth_test_results: Vec<FtpAuthAttempt>,
    pub success: bool,
    pub successful_credential: Option<(String, String)>,
    pub severity: Severity,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FtpAuthAttempt {
    pub username: String,
    pub password: String,
    pub success: bool,
    pub message: String,
}

pub fn grab_banner(address: &str, port: u16) -> Result<Option<String>> {
    let addr = format!("{}:{}", address, port);
    let mut stream = TcpStream::connect(&addr)
        .map_err(|e| crate::error::EggsecError::Network(format!("TCP connection failed: {}", e)))?;

    stream.set_read_timeout(Some(Duration::from_secs(5)))
        .map_err(|e| crate::error::EggsecError::Network(format!("Timeout set failed: {}", e)))?;

    use std::io::{BufRead, BufReader};
    let reader = BufReader::new(stream);
    let mut lines = reader.lines();

    if let Some(Ok(line)) = lines.next() {
        if line.starts_with("220") {
            return Ok(Some(line));
        }
    }

    Ok(None)
}

pub async fn test_ftp_auth(
    target: &str,
    port: u16,
    credentials: &[(String, String)],
    timeout_secs: u64,
) -> Result<FtpAuthResult> {
    let timeout = Duration::from_secs(timeout_secs);
    let banner = grab_banner(target, port)?;

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
            "FTP authentication successful with {}:{}",
            successful_credential.as_ref().unwrap().0,
            "[REDACTED]"
        )
    } else {
        "FTP authentication failed for all tested credentials".to_string()
    };

    Ok(FtpAuthResult {
        target: target.to_string(),
        port,
        banner,
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
) -> FtpAuthAttempt {
    let addr = format!("{}:{}", target, port);

    let result = tokio::time::timeout(
        timeout,
        ftp_auth_attempt(&addr, username, password),
    )
    .await;

    match result {
        Ok(Ok(success)) => FtpAuthAttempt {
            username: username.to_string(),
            password: password.to_string(),
            success,
            message: if success {
                "Authentication successful".to_string()
            } else {
                "Authentication failed".to_string()
            },
        },
        Ok(Err(e)) => FtpAuthAttempt {
            username: username.to_string(),
            password: password.to_string(),
            success: false,
            message: format!("Connection error: {}", e),
        },
        Err(_) => FtpAuthAttempt {
            username: username.to_string(),
            password: password.to_string(),
            success: false,
            message: "Connection timeout".to_string(),
        },
    }
}

async fn ftp_auth_attempt(addr: &str, username: &str, password: &str) -> Result<bool> {
    use std::io::{Read, Write};

    let mut stream = TcpStream::connect(addr)
        .map_err(|e| crate::error::EggsecError::Network(format!("TCP connection failed: {}", e)))?;

    stream.set_read_timeout(Some(Duration::from_secs(10)))
        .map_err(|e| crate::error::EggsecError::Network(format!("Timeout set failed: {}", e)))?;

    let mut response = [0u8; 1024];
    stream.read(&mut response)
        .map_err(|e| crate::error::EggsecError::Network(format!("Read failed: {}", e)))?;

    let response_str = String::from_utf8_lossy(&response);

    if !response_str.contains("220") {
        return Err(crate::error::EggsecError::Network("Invalid FTP banner".to_string()));
    }

    let user_cmd = format!("USER {}\r\n", username);
    stream.write_all(user_cmd.as_bytes())
        .map_err(|e| crate::error::EggsecError::Network(format!("Write failed: {}", e)))?;

    let mut response = [0u8; 1024];
    stream.read(&mut response)
        .map_err(|e| crate::error::EggsecError::Network(format!("Read failed: {}", e)))?;

    let pass_cmd = format!("PASS {}\r\n", password);
    stream.write_all(pass_cmd.as_bytes())
        .map_err(|e| crate::error::EggsecError::Network(format!("Write failed: {}", e)))?;

    let mut response = [0u8; 1024];
    let n = stream.read(&mut response)
        .map_err(|e| crate::error::EggsecError::Network(format!("Read failed: {}", e)))?;

    let response_str = String::from_utf8_lossy(&response[..n]);
    Ok(response_str.contains("230"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ftp_auth_result_structure() {
        let result = FtpAuthResult {
            target: "example.com".to_string(),
            port: 21,
            banner: Some("220 FTP Server ready".to_string()),
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
    fn test_ftp_auth_attempt_structure() {
        let attempt = FtpAuthAttempt {
            username: "anonymous".to_string(),
            password: "guest@example.com".to_string(),
            success: false,
            message: "Authentication failed".to_string(),
        };

        assert_eq!(attempt.username, "anonymous");
        assert!(!attempt.success);
    }
}
