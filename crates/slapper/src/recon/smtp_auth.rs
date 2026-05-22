//! SMTP authentication testing
//!
//! Provides SMTP server banner grabbing and authentication testing capabilities.
//! Tests SMTP authentication using LOGIN and PLAIN mechanisms.

use crate::error::Result;
use crate::recon::secrets::Severity;
use serde::{Deserialize, Serialize};
use std::net::TcpStream;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmtpAuthResult {
    pub target: String,
    pub port: u16,
    pub banner: Option<String>,
    pub auth_mechanisms: Vec<String>,
    pub auth_test_results: Vec<SmtpAuthAttempt>,
    pub success: bool,
    pub successful_credential: Option<(String, String)>,
    pub severity: Severity,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmtpAuthAttempt {
    pub username: String,
    pub password: String,
    pub mechanism: String,
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
        if line.starts_with("220") {
            return Ok(Some(line));
        }
    }

    Ok(None)
}

pub async fn test_smtp_auth(
    target: &str,
    port: u16,
    credentials: &[(String, String)],
    timeout_secs: u64,
) -> Result<SmtpAuthResult> {
    let timeout = Duration::from_secs(timeout_secs);
    let banner = grab_banner(target, port)?;

    let auth_mechanisms = detect_auth_mechanisms(target, port, timeout_secs).await?;

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
            "SMTP authentication successful with {}:{}",
            successful_credential.as_ref().unwrap().0,
            "[REDACTED]"
        )
    } else {
        "SMTP authentication failed for all tested credentials".to_string()
    };

    Ok(SmtpAuthResult {
        target: target.to_string(),
        port,
        banner,
        auth_mechanisms,
        auth_test_results: auth_results,
        success,
        successful_credential,
        severity,
        message,
    })
}

async fn detect_auth_mechanisms(target: &str, port: u16, timeout_secs: u64) -> Result<Vec<String>> {
    let addr = format!("{}:{}", target, port);
    let timeout = Duration::from_secs(timeout_secs);

    let result = tokio::time::timeout(timeout, async {
        use std::io::{Read, Write};

    let stream = TcpStream::connect(&addr)
            .map_err(|e| crate::error::SlapperError::Network(format!("TCP connection failed: {}", e)))?;

        stream.set_read_timeout(Some(Duration::from_secs(10)))
            .map_err(|e| crate::error::SlapperError::Network(format!("Timeout set failed: {}", e)))?;

        let mut response = [0u8; 1024];
        stream.read(&mut response)
            .map_err(|e| crate::error::SlapperError::Network(format!("Read failed: {}", e)))?;

        let ehlo_cmd = "EHLO localhost\r\n";
        stream.write_all(ehlo_cmd.as_bytes())
            .map_err(|e| crate::error::SlapperError::Network(format!("Write failed: {}", e)))?;

        let mut response = [0u8; 4096];
        let n = stream.read(&mut response)
            .map_err(|e| crate::error::SlapperError::Network(format!("Read failed: {}", e)))?;

        let response_str = String::from_utf8_lossy(&response[..n]);
        let mut mechanisms = Vec::new();

        for line in response_str.lines() {
            if line.starts_with("250-AUTH") || line.starts_with("250 AUTH") {
                let auth_part = line.splitn(2, ' ').nth(1).unwrap_or("");
                mechanisms.extend(
                    auth_part.split_whitespace().map(|s| s.to_string())
                );
            }
        }

        Ok(mechanisms) as Result<Vec<String>>
    }).await;

    match result {
        Ok(Ok(mechanisms)) => Ok(mechanisms),
        _ => Ok(vec![]),
    }
}

async fn test_single_credential(
    target: &str,
    port: u16,
    username: &str,
    password: &str,
    timeout: Duration,
) -> SmtpAuthAttempt {
    let addr = format!("{}:{}", target, port);

    let result = tokio::time::timeout(
        timeout,
        smtp_auth_attempt(&addr, username, password),
    )
    .await;

    match result {
        Ok(Ok((success, mechanism))) => SmtpAuthAttempt {
            username: username.to_string(),
            password: password.to_string(),
            mechanism,
            success,
            message: if success {
                "Authentication successful".to_string()
            } else {
                "Authentication failed".to_string()
            },
        },
        Ok(Err(e)) => SmtpAuthAttempt {
            username: username.to_string(),
            password: password.to_string(),
            mechanism: "unknown".to_string(),
            success: false,
            message: format!("Connection error: {}", e),
        },
        Err(_) => SmtpAuthAttempt {
            username: username.to_string(),
            password: password.to_string(),
            mechanism: "unknown".to_string(),
            success: false,
            message: "Connection timeout".to_string(),
        },
    }
}

async fn smtp_auth_attempt(addr: &str, username: &str, password: &str) -> Result<(bool, String)> {
    use std::io::{Read, Write};

    let mut stream = TcpStream::connect(addr)
        .map_err(|e| crate::error::SlapperError::Network(format!("TCP connection failed: {}", e)))?;

    stream.set_read_timeout(Some(Duration::from_secs(10)))
        .map_err(|e| crate::error::SlapperError::Network(format!("Timeout set failed: {}", e)))?;

    let mut response = [0u8; 1024];
    stream.read(&mut response)
        .map_err(|e| crate::error::SlapperError::Network(format!("Read failed: {}", e)))?;

    let response_str = String::from_utf8_lossy(&response);

    if !response_str.contains("220") {
        return Err(crate::error::SlapperError::Network("Invalid SMTP banner".to_string()));
    }

    let ehlo_cmd = "EHLO localhost\r\n";
    stream.write_all(ehlo_cmd.as_bytes())
        .map_err(|e| crate::error::SlapperError::Network(format!("Write failed: {}", e)))?;

    let mut response = [0u8; 4096];
    let n = stream.read(&mut response)
        .map_err(|e| crate::error::SlapperError::Network(format!("Read failed: {}", e)))?;

    let response_str = String::from_utf8_lossy(&response[..n]);

    if response_str.contains("250-AUTH") || response_str.contains("250 AUTH") {
        if response_str.contains("LOGIN") {
            return test_login_auth(&mut stream, username, password).await;
        } else if response_str.contains("PLAIN") {
            return test_plain_auth(&mut stream, username, password).await;
        }
    }

    Ok((false, "no-auth".to_string()))
}

async fn test_login_auth(
    stream: &mut TcpStream,
    username: &str,
    password: &str,
) -> Result<(bool, String)> {
    use std::io::{Read, Write};

    let auth_login_cmd = "AUTH LOGIN\r\n";
    stream.write_all(auth_login_cmd.as_bytes())
        .map_err(|e| crate::error::SlapperError::Network(format!("Write failed: {}", e)))?;

    let mut response = [0u8; 1024];
    stream.read(&mut response)
        .map_err(|e| crate::error::SlapperError::Network(format!("Read failed: {}", e)))?;

    let username_b64 = base64::engine::general_purpose::STANDARD.encode(username);
    stream.write_all(format!("{}\r\n", username_b64).as_bytes())
        .map_err(|e| crate::error::SlapperError::Network(format!("Write failed: {}", e)))?;

    let mut response = [0u8; 1024];
    stream.read(&mut response)
        .map_err(|e| crate::error::SlapperError::Network(format!("Read failed: {}", e)))?;

    let password_b64 = base64::engine::general_purpose::STANDARD.encode(password);
    stream.write_all(format!("{}\r\n", password_b64).as_bytes())
        .map_err(|e| crate::error::SlapperError::Network(format!("Write failed: {}", e)))?;

    let mut response = [0u8; 1024];
    let n = stream.read(&mut response)
        .map_err(|e| crate::error::SlapperError::Network(format!("Read failed: {}", e)))?;

    let response_str = String::from_utf8_lossy(&response[..n]);
    let success = response_str.contains("235") || response_str.contains("Authentication successful");
    Ok((success, "LOGIN".to_string()))
}

async fn test_plain_auth(
    stream: &mut TcpStream,
    username: &str,
    password: &str,
) -> Result<(bool, String)> {
    use std::io::{Read, Write};

    let auth_plain_cmd = "AUTH PLAIN\r\n";
    stream.write_all(auth_plain_cmd.as_bytes())
        .map_err(|e| crate::error::SlapperError::Network(format!("Write failed: {}", e)))?;

    let mut response = [0u8; 1024];
    stream.read(&mut response)
        .map_err(|e| crate::error::SlapperError::Network(format!("Read failed: {}", e)))?;

    let auth_string = format!("\0{}\0{}", username, password);
    let auth_b64 = base64::engine::general_purpose::STANDARD.encode(&auth_string);
    stream.write_all(format!("{}\r\n", auth_b64).as_bytes())
        .map_err(|e| crate::error::SlapperError::Network(format!("Write failed: {}", e)))?;

    let mut response = [0u8; 1024];
    let n = stream.read(&mut response)
        .map_err(|e| crate::error::SlapperError::Network(format!("Read failed: {}", e)))?;

    let response_str = String::from_utf8_lossy(&response[..n]);
    let success = response_str.contains("235") || response_str.contains("Authentication successful");
    Ok((success, "PLAIN".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smtp_auth_result_structure() {
        let result = SmtpAuthResult {
            target: "example.com".to_string(),
            port: 25,
            banner: Some("220 mail.example.com ESMTP".to_string()),
            auth_mechanisms: vec!["LOGIN".to_string(), "PLAIN".to_string()],
            auth_test_results: vec![],
            success: false,
            successful_credential: None,
            severity: Severity::Info,
            message: "Authentication failed".to_string(),
        };

        assert!(!result.success);
        assert!(result.banner.is_some());
        assert_eq!(result.auth_mechanisms.len(), 2);
    }

    #[test]
    fn test_smtp_auth_attempt_structure() {
        let attempt = SmtpAuthAttempt {
            username: "user".to_string(),
            password: "password".to_string(),
            mechanism: "LOGIN".to_string(),
            success: false,
            message: "Authentication failed".to_string(),
        };

        assert_eq!(attempt.username, "user");
        assert_eq!(attempt.mechanism, "LOGIN");
        assert!(!attempt.success);
    }
}
