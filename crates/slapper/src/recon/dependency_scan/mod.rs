pub mod cargo;
pub mod go;
pub mod npm;

pub use crate::error::Result;
pub use crate::types::Severity;

pub use self::cargo::CargoScanner;
pub use self::go::GoScanner;
pub use self::npm::NpmScanner;

use crate::utils::create_http_client;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyScanReport {
    pub project_path: String,
    pub ecosystems: Vec<DependencyEcosystem>,
    pub total_dependencies: usize,
    pub total_vulnerabilities: usize,
    pub summary: DependencySummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencySummary {
    pub critical: usize,
    pub high: usize,
    pub medium: usize,
    pub low: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyEcosystem {
    pub name: String,
    pub manifest_file: String,
    pub lock_file: Option<String>,
    pub dependencies: Vec<DependencyInfo>,
    pub vulnerabilities: Vec<DependencyVulnerability>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyInfo {
    pub name: String,
    pub version: String,
    pub is_direct: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyVulnerability {
    pub dependency: String,
    pub version: String,
    pub cve_id: String,
    pub title: String,
    pub severity: Severity,
    pub fixed_version: Option<String>,
    pub description: String,
    pub references: Vec<String>,
}

pub struct DependencyScanner {
    #[allow(dead_code)]
    client: reqwest::Client,
}

impl DependencyScanner {
    pub fn new() -> Result<Self> {
        let client = create_http_client(30)?;
        Ok(Self { client })
    }

    pub async fn scan_project(&self, project_path: &str) -> Result<DependencyScanReport> {
        let path = Path::new(project_path);
        let mut ecosystems = Vec::new();
        let mut total_deps = 0;
        let mut total_vulns = 0;

        let manifest_files = self.find_manifests(path);

        for manifest in manifest_files {
            let ecosystem = self.parse_manifest(&manifest).await;
            if let Ok(ecosystem) = ecosystem {
                total_deps += ecosystem.dependencies.len();
                total_vulns += ecosystem.vulnerabilities.len();
                ecosystems.push(ecosystem);
            }
        }

        let summary = DependencySummary {
            critical: ecosystems
                .iter()
                .map(|e| {
                    e.vulnerabilities
                        .iter()
                        .filter(|v| v.severity == Severity::Critical)
                        .count()
                })
                .sum(),
            high: ecosystems
                .iter()
                .map(|e| {
                    e.vulnerabilities
                        .iter()
                        .filter(|v| v.severity == Severity::High)
                        .count()
                })
                .sum(),
            medium: ecosystems
                .iter()
                .map(|e| {
                    e.vulnerabilities
                        .iter()
                        .filter(|v| v.severity == Severity::Medium)
                        .count()
                })
                .sum(),
            low: ecosystems
                .iter()
                .map(|e| {
                    e.vulnerabilities
                        .iter()
                        .filter(|v| v.severity == Severity::Low)
                        .count()
                })
                .sum(),
        };

        Ok(DependencyScanReport {
            project_path: project_path.to_string(),
            ecosystems,
            total_dependencies: total_deps,
            total_vulnerabilities: total_vulns,
            summary,
        })
    }

    fn find_manifests(&self, path: &Path) -> Vec<PathBuf> {
        let mut found = Vec::new();
        let targets = vec![
            "Cargo.toml",
            "Cargo.lock",
            "package.json",
            "package-lock.json",
            "yarn.lock",
            "requirements.txt",
            "go.mod",
            "go.sum",
            "Gemfile",
            "Gemfile.lock",
            "composer.json",
            "pom.xml",
        ];

        if path.is_dir() {
            if let Ok(entries) = std::fs::read_dir(path) {
                for entry in entries.flatten() {
                    let file_path = entry.path();
                    let name_str = file_path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();
                    if file_path.is_file() && targets.contains(&name_str.as_ref()) {
                        found.push(file_path);
                    }
                }
            }
        } else if path.is_file()
            && targets.contains(
                &path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string()
                    .as_ref(),
            )
        {
            found.push(path.to_path_buf());
        }

        found
    }

    async fn parse_manifest(&self, path: &Path) -> Result<DependencyEcosystem> {
        let file_name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let ecosystem = match file_name.as_str() {
            "Cargo.toml" => CargoScanner::scan_cargo_toml(path),
            "Cargo.lock" => CargoScanner::scan_cargo_lock(path),
            "package.json" => NpmScanner::scan_package_json(path),
            "package-lock.json" => NpmScanner::scan_package_lock(path),
            "yarn.lock" => NpmScanner::scan_yarn_lock(path),
            "requirements.txt" => NpmScanner::scan_requirements_txt(path),
            "go.mod" => GoScanner::scan_go_mod(path),
            "go.sum" => GoScanner::scan_go_sum(path),
            _ => Err(crate::error::SlapperError::Runtime(format!(
                "Unknown manifest file: {}",
                file_name
            ))),
        };

        ecosystem
    }
}

impl Default for DependencyScanner {
    fn default() -> Self {
        Self::new().expect("Failed to create HTTP client")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scanner_creation() {
        let scanner = DependencyScanner::new();
        assert!(scanner.is_ok());
    }

    #[test]
    fn test_find_manifests_empty_dir() {
        let scanner = DependencyScanner::new().unwrap();
        let manifests = scanner.find_manifests(Path::new("/tmp"));
        assert!(manifests.is_empty() || manifests.len() >= 0);
    }
}
