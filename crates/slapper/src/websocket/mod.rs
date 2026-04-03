//! WebSocket security testing module
//!
//! Provides real WebSocket connection testing including injection,
//! authentication bypass, origin validation, and frame fuzzing.

pub mod connection;
pub mod fuzz;
pub mod injection;
pub mod origin;

use serde::{Deserialize, Serialize};

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

pub use crate::types::Severity;

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
