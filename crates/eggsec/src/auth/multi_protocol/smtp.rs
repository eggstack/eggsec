//! SMTP authentication testing
//!
//! Tests SMTP authentication mechanisms including PLAIN and LOGIN.

use super::AuthTestResult;
use crate::error::{Result, EggsecError};
use crate::types::Severity;
use std::net::TcpStream;
use std::time::Duration;

pub async fn test_smtp_auth(
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

    let result = tokio::time::timeout(timeout, smtp_auth_attempt(&addr, username, password)).await;

    match result {
        Ok(Ok(success)) => AuthTestResult {
            protocol: "SMTP".to_string(),
            target: target.to_string(),
            port,
            success,
            credentials_used: if success {
                Some((username.to_string(), password.to_string()))
            } else {
                None
            },
            severity: if success {
                Severity::Critical
            } else {
                Severity::Info
            },
            message: if success {
                "Authentication successful".to_string()
            } else {
                "Authentication failed".to_string()
            },
        },
        Ok(Err(e)) => AuthTestResult {
            protocol: "SMTP".to_string(),
            target: target.to_string(),
            port,
            success: false,
            credentials_used: None,
            severity: Severity::Info,
            message: format!("Connection error: {}", e),
        },
        Err(_) => AuthTestResult {
            protocol: "SMTP".to_string(),
            target: target.to_string(),
            port,
            success: false,
            credentials_used: None,
            severity: Severity::Info,
            message: "Connection timeout".to_string(),
        },
    }
}

async fn smtp_auth_attempt(addr: &str, username: &str, password: &str) -> Result<bool> {
    use std::io::{Read, Write};

    let mut stream = TcpStream::connect(addr)
        .map_err(|e| EggsecError::Network(format!("TCP connection failed: {}", e)))?;

    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .map_err(|e| EggsecError::Network(format!("Timeout set failed: {}", e)))?;

    let mut response = [0u8; 1024];
    stream
        .read(&mut response)
        .map_err(|e| EggsecError::Network(format!("Read failed: {}", e)))?;

    let response_str = String::from_utf8_lossy(&response);

    if !response_str.contains("220") {
        return Err(EggsecError::Network("Invalid SMTP banner".to_string()));
    }

    let ehlo_cmd = "EHLO localhost\r\n";
    stream
        .write_all(ehlo_cmd.as_bytes())
        .map_err(|e| EggsecError::Network(format!("Write failed: {}", e)))?;

    let mut response = [0u8; 4096];
    let n = stream
        .read(&mut response)
        .map_err(|e| EggsecError::Network(format!("Read failed: {}", e)))?;

    let response_str = String::from_utf8_lossy(&response[..n]);

    if response_str.contains("250-AUTH") {
        if response_str.contains("LOGIN") {
            return test_login_auth(&mut stream, username, password).await;
        } else if response_str.contains("PLAIN") {
            return test_plain_auth(&mut stream, username, password).await;
        }
    }

    Ok(false)
}

async fn test_login_auth(stream: &mut TcpStream, username: &str, password: &str) -> Result<bool> {
    use std::io::{Read, Write};

    let auth_login_cmd = "AUTH LOGIN\r\n";
    stream
        .write_all(auth_login_cmd.as_bytes())
        .map_err(|e| EggsecError::Network(format!("Write failed: {}", e)))?;

    let mut response = [0u8; 1024];
    stream
        .read(&mut response)
        .map_err(|e| EggsecError::Network(format!("Read failed: {}", e)))?;

    let username_b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, username);
    stream
        .write_all(format!("{}\r\n", username_b64).as_bytes())
        .map_err(|e| EggsecError::Network(format!("Write failed: {}", e)))?;

    let mut response = [0u8; 1024];
    stream
        .read(&mut response)
        .map_err(|e| EggsecError::Network(format!("Read failed: {}", e)))?;

    let password_b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, password);
    stream
        .write_all(format!("{}\r\n", password_b64).as_bytes())
        .map_err(|e| EggsecError::Network(format!("Write failed: {}", e)))?;

    let mut response = [0u8; 1024];
    let n = stream
        .read(&mut response)
        .map_err(|e| EggsecError::Network(format!("Read failed: {}", e)))?;

    let response_str = String::from_utf8_lossy(&response[..n]);
    Ok(response_str.contains("235") || response_str.contains("Authentication successful"))
}

async fn test_plain_auth(stream: &mut TcpStream, username: &str, password: &str) -> Result<bool> {
    use std::io::{Read, Write};

    let auth_plain_cmd = "AUTH PLAIN\r\n";
    stream
        .write_all(auth_plain_cmd.as_bytes())
        .map_err(|e| EggsecError::Network(format!("Write failed: {}", e)))?;

    let mut response = [0u8; 1024];
    stream
        .read(&mut response)
        .map_err(|e| EggsecError::Network(format!("Read failed: {}", e)))?;

    let auth_string = format!("\0{}\0{}", username, password);
    let auth_b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &auth_string);
    stream
        .write_all(format!("{}\r\n", auth_b64).as_bytes())
        .map_err(|e| EggsecError::Network(format!("Write failed: {}", e)))?;

    let mut response = [0u8; 1024];
    let n = stream
        .read(&mut response)
        .map_err(|e| EggsecError::Network(format!("Read failed: {}", e)))?;

    let response_str = String::from_utf8_lossy(&response[..n]);
    Ok(response_str.contains("235") || response_str.contains("Authentication successful"))
}

pub fn check_smtp_banner(address: &str, port: u16) -> Result<Option<String>> {
    use std::io::{BufRead, BufReader};
    use std::net::TcpStream;

    let addr = format!("{}:{}", address, port);
    let mut stream = TcpStream::connect(&addr)
        .map_err(|e| EggsecError::Network(format!("TCP connection failed: {}", e)))?;

    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .map_err(|e| EggsecError::Network(format!("Timeout set failed: {}", e)))?;

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
    async fn test_smtp_auth_result_structure() {
        let result = AuthTestResult {
            protocol: "SMTP".to_string(),
            target: "example.com".to_string(),
            port: 25,
            success: false,
            credentials_used: None,
            severity: Severity::Info,
            message: "Connection refused".to_string(),
        };

        assert!(!result.success);
        assert_eq!(result.protocol, "SMTP");
    }
}
