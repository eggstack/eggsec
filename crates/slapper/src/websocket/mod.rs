//! WebSocket security testing module
//!
//! Provides real WebSocket connection testing including injection,
//! authentication bypass, origin validation, and frame fuzzing.

pub mod connection;
pub mod fuzz;
pub mod injection;
pub mod origin;

use serde::{Deserialize, Serialize};

use crate::types::Severity;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketTestReport {
    pub target: String,
    pub connection_test: Option<connection::ConnectionTestResult>,
    pub injection_tests: Vec<injection::InjectionTestResult>,
    pub origin_tests: Vec<origin::OriginTestResult>,
    pub fuzz_tests: Vec<fuzz::FuzzTestResult>,
    pub findings: Vec<WebSocketFinding>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketFinding {
    pub category: String,
    pub severity: Severity,
    pub title: String,
    pub description: String,
    pub recommendation: String,
}

#[cfg(feature = "websocket")]
pub struct WebSocketTestConfig {
    pub url: String,
    pub timeout_secs: u64,
    pub injection_payloads: Vec<String>,
    pub test_connection: bool,
    pub test_origins: bool,
    pub test_injection: bool,
    pub test_dos: bool,
    pub test_message_fuzz: bool,
}

#[cfg(feature = "websocket")]
pub async fn run_live_tests(config: &WebSocketTestConfig) -> WebSocketTestReport {
    let mut findings = Vec::new();
    let mut injection_tests = Vec::new();
    let mut origin_tests = Vec::new();
    let mut fuzz_tests = Vec::new();

    let connection_test = if config.test_connection {
        let result = connection::test_connection(&config.url, config.timeout_secs).await;
        if !result.connected {
            findings.push(WebSocketFinding {
                category: "Connection".to_string(),
                severity: Severity::High,
                title: "WebSocket connection failed".to_string(),
                description: result
                    .error
                    .clone()
                    .unwrap_or_else(|| "Unknown error".to_string()),
                recommendation: "Verify the WebSocket endpoint is accessible".to_string(),
            });
        }
        Some(result)
    } else {
        None
    };

    if config.test_origins {
        origin_tests = origin::test_origins(&config.url, config.timeout_secs).await;
        for test in &origin_tests {
            if test.accepted {
                findings.push(WebSocketFinding {
                    category: "CSWSH".to_string(),
                    severity: Severity::High,
                    title: "Cross-Site WebSocket Hijacking".to_string(),
                    description: format!(
                        "Origin '{}' was accepted without validation",
                        test.origin
                    ),
                    recommendation: "Validate the Origin header on WebSocket upgrades".to_string(),
                });
            }
        }
    }

    if config.test_injection && !config.injection_payloads.is_empty() {
        injection_tests = injection::test_injection(
            &config.url,
            &config.injection_payloads,
            config.timeout_secs,
        )
        .await;
        for test in &injection_tests {
            if test.vulnerability_detected {
                findings.push(WebSocketFinding {
                    category: "Injection".to_string(),
                    severity: Severity::Critical,
                    title: "WebSocket injection vulnerability".to_string(),
                    description: format!(
                        "Payload '{}' triggered a vulnerability indicator",
                        test.payload
                    ),
                    recommendation: "Sanitize all input received via WebSocket messages"
                        .to_string(),
                });
            }
        }
    }

    if config.test_dos {
        let dos_tests = fuzz::test_dos(&config.url, config.timeout_secs).await;
        for test in &dos_tests {
            if test.vulnerability_detected {
                findings.push(WebSocketFinding {
                    category: "DoS".to_string(),
                    severity: Severity::Medium,
                    title: "WebSocket DoS vulnerability".to_string(),
                    description: test.details.clone(),
                    recommendation: "Implement message size limits and rate limiting".to_string(),
                });
            }
        }
        fuzz_tests.extend(dos_tests);
    }

    if config.test_message_fuzz {
        let fuzz_results = fuzz::test_message_fuzz(&config.url, config.timeout_secs).await;
        for test in &fuzz_results {
            if test.vulnerability_detected {
                findings.push(WebSocketFinding {
                    category: "Message Fuzzing".to_string(),
                    severity: Severity::Medium,
                    title: "Server error on fuzzed message".to_string(),
                    description: test.details.clone(),
                    recommendation: "Implement proper error handling for malformed messages"
                        .to_string(),
                });
            }
        }
        fuzz_tests.extend(fuzz_results);
    }

    WebSocketTestReport {
        target: config.url.clone(),
        connection_test,
        injection_tests,
        origin_tests,
        fuzz_tests,
        findings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_finding_creation() {
        let finding = WebSocketFinding {
            category: "Injection".to_string(),
            severity: Severity::High,
            title: "Test".to_string(),
            description: "Test".to_string(),
            recommendation: "Test".to_string(),
        };
        assert_eq!(finding.category, "Injection");
    }
}
