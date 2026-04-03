//! Container security scanning module
//!
//! Provides Docker image analysis, Kubernetes security checks,
//! container escape detection, and CIS benchmark validation.

pub mod cis;
pub mod docker;
pub mod escape;
pub mod kubernetes;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerScanReport {
    pub target: String,
    pub scan_type: ContainerScanType,
    pub docker: Option<docker::DockerScanResult>,
    pub kubernetes: Option<kubernetes::KubernetesScanResult>,
    pub escape_risks: Option<escape::EscapeDetectionResult>,
    pub cis_benchmarks: Option<cis::CisBenchmarkResult>,
    pub findings: Vec<ContainerFinding>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ContainerScanType {
    Docker,
    Kubernetes,
    EscapeDetection,
    CisBenchmark,
    Full,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerFinding {
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
    fn test_scan_type_variants() {
        assert_eq!(ContainerScanType::Docker, ContainerScanType::Docker);
        assert_ne!(ContainerScanType::Docker, ContainerScanType::Kubernetes);
    }

    #[test]
    fn test_container_finding_creation() {
        let finding = ContainerFinding {
            category: "Docker".to_string(),
            severity: Severity::High,
            title: "Test finding".to_string(),
            description: "Test description".to_string(),
            recommendation: "Test recommendation".to_string(),
        };
        assert_eq!(finding.category, "Docker");
        assert_eq!(finding.severity, Severity::High);
    }
}
