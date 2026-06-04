//! Supply chain security module
//!
//! Provides SBOM generation, typosquatting detection, and repository
//! manifest/configuration analysis for software dependencies.

pub mod sbom;
pub mod scanner;
pub mod typosquat;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupplyChainReport {
    pub project_path: String,
    pub sbom: Option<sbom::SbomReport>,
    pub typosquatting: Option<typosquat::TyposquatReport>,
    pub total_packages: usize,
    pub total_risks: usize,
    pub findings: Vec<SupplyChainFinding>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupplyChainFinding {
    pub category: String,
    pub severity: Severity,
    pub title: String,
    pub description: String,
    pub recommendation: String,
    pub file_path: Option<String>,
    pub line: Option<u32>,
}

pub use crate::types::Severity;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_finding_creation() {
        let finding = SupplyChainFinding {
            category: "SBOM".to_string(),
            severity: Severity::High,
            title: "Test".to_string(),
            description: "Test".to_string(),
            recommendation: "Test".to_string(),
            file_path: None,
            line: None,
        };
        assert_eq!(finding.category, "SBOM");
    }
}
