//! Multi-protocol authentication testing
//!
//! Provides authentication testing capabilities for non-HTTP protocols
//! including SSH, FTP, SMTP, and other common network services.

pub mod ftp;
pub mod smtp;
pub mod ssh;

use crate::error::Result;
use crate::types::Severity;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthTestResult {
    pub protocol: String,
    pub target: String,
    pub port: u16,
    pub success: bool,
    pub credentials_used: Option<(String, String)>,
    pub severity: Severity,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct ProtocolAuthTester {
    timeout: Duration,
    max_attempts: usize,
}

impl ProtocolAuthTester {
    pub fn new(timeout_secs: u64) -> Result<Self> {
        Ok(Self {
            timeout: Duration::from_secs(timeout_secs),
            max_attempts: 100,
        })
    }

    pub fn with_max_attempts(mut self, max: usize) -> Self {
        self.max_attempts = max;
        self
    }

    pub async fn test_ssh(
        &self,
        target: &str,
        port: u16,
        credentials: &[(String, String)],
    ) -> Result<Vec<AuthTestResult>> {
        ssh::test_ssh_auth(target, port, credentials, self.timeout).await
    }

    pub async fn test_ftp(
        &self,
        target: &str,
        port: u16,
        credentials: &[(String, String)],
    ) -> Result<Vec<AuthTestResult>> {
        ftp::test_ftp_auth(target, port, credentials, self.timeout).await
    }

    pub async fn test_smtp(
        &self,
        target: &str,
        port: u16,
        credentials: &[(String, String)],
    ) -> Result<Vec<AuthTestResult>> {
        smtp::test_smtp_auth(target, port, credentials, self.timeout).await
    }
}

pub const MULTI_PROTOCOL_AUTH_BANNER: &str = r#"
╔══════════════════════════════════════════════════════════╗
║  ⚠️  AUTHORIZED USE ONLY  ⚠️                            ║
║                                                          ║
║  This tool performs authentication security testing      ║
║  on multiple protocols. Only use against systems you    ║
║  have explicit permission to test.                      ║
╚══════════════════════════════════════════════════════════╝
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_protocol_auth_tester_creation() {
        let tester = ProtocolAuthTester::new(30);
        assert_eq!(tester.max_attempts, 100);
    }
}
