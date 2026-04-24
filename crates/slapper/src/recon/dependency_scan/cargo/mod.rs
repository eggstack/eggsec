use crate::error::Result;
use crate::recon::dependency_scan::{DependencyEcosystem, DependencyInfo};
use std::path::Path;

pub struct CargoScanner;

impl CargoScanner {
    pub fn new() -> Self {
        Self
    }

    pub fn scan_cargo_toml(path: &Path) -> Result<DependencyEcosystem> {
        let content = std::fs::read_to_string(path)?;
        let mut dependencies = Vec::new();

        let mut in_deps = false;
        let mut in_dev_deps = false;
        let mut in_build_deps = false;

        for line in content.lines() {
            let trimmed = line.trim();

            if trimmed == "[dependencies]" {
                in_deps = true;
                in_dev_deps = false;
                in_build_deps = false;
                continue;
            }
            if trimmed == "[dev-dependencies]" {
                in_deps = false;
                in_dev_deps = true;
                in_build_deps = false;
                continue;
            }
            if trimmed == "[build-dependencies]" {
                in_deps = false;
                in_dev_deps = false;
                in_build_deps = true;
                continue;
            }
            if trimmed.starts_with('[') {
                in_deps = false;
                in_dev_deps = false;
                in_build_deps = false;
                continue;
            }

            if in_deps || in_dev_deps || in_build_deps {
                if let Some((name, version)) = Self::parse_toml_dep(trimmed) {
                    dependencies.push(DependencyInfo {
                        name,
                        version,
                        is_direct: true,
                    });
                }
            }
        }

        Ok(DependencyEcosystem {
            name: "Rust (Cargo)".to_string(),
            manifest_file: path.to_string_lossy().to_string(),
            lock_file: None,
            dependencies,
            vulnerabilities: Vec::new(),
        })
    }

    pub fn scan_cargo_lock(path: &Path) -> Result<DependencyEcosystem> {
        let content = std::fs::read_to_string(path)?;
        let mut dependencies = Vec::new();

        let mut current_name: Option<String> = None;
        let mut current_version: Option<String> = None;

        for line in content.lines() {
            let trimmed = line.trim();

            if trimmed == "[[package]]" {
                if let (Some(name), Some(version)) = (current_name.take(), current_version.take()) {
                    dependencies.push(DependencyInfo {
                        name,
                        version,
                        is_direct: true,
                    });
                }
                continue;
            }

            if trimmed.starts_with("name = ") {
                current_name = Some(trimmed.trim_start_matches("name = ").trim_matches('"').to_string());
            } else if trimmed.starts_with("version = ") {
                current_version = Some(trimmed.trim_start_matches("version = ").trim_matches('"').to_string());
            }
        }

        if let (Some(name), Some(version)) = (current_name, current_version) {
            dependencies.push(DependencyInfo {
                name,
                version,
                is_direct: true,
            });
        }

        Ok(DependencyEcosystem {
            name: "Rust (Cargo)".to_string(),
            manifest_file: path.to_string_lossy().to_string(),
            lock_file: None,
            dependencies,
            vulnerabilities: Vec::new(),
        })
    }

    fn parse_toml_dep(line: &str) -> Option<(String, String)> {
        if let Some(eq_pos) = line.find('=') {
            let name = line[..eq_pos].trim().to_string();
            let rest = line[eq_pos + 1..].trim();
            let version = if rest.starts_with('{') {
                rest.trim_start_matches('{')
                    .trim_end_matches('}')
                    .trim_start_matches("version = ")
                    .trim_matches('"')
                    .to_string()
            } else {
                rest.trim_matches('"').to_string()
            };
            if !name.is_empty() {
                return Some((name, version));
            }
        }
        None
    }
}