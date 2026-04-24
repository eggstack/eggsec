use crate::error::Result;
use crate::recon::dependency_scan::{DependencyEcosystem, DependencyInfo};
use std::path::Path;

pub struct NpmScanner;

impl NpmScanner {
    pub fn new() -> Self {
        Self
    }

    pub fn scan_package_json(path: &Path) -> Result<DependencyEcosystem> {
        let content = std::fs::read_to_string(path)?;
        let json: serde_json::Value = serde_json::from_str(&content)?;
        let mut dependencies = Vec::new();

        if let Some(deps) = json.get("dependencies").and_then(|v| v.as_object()) {
            for (name, version) in deps {
                let version_str = version.as_str().unwrap_or("*").trim_start_matches('^').trim_start_matches('~').to_string();
                dependencies.push(DependencyInfo {
                    name: name.clone(),
                    version: version_str,
                    is_direct: true,
                });
            }
        }

        if let Some(deps) = json.get("devDependencies").and_then(|v| v.as_object()) {
            for (name, version) in deps {
                let version_str = version.as_str().unwrap_or("*").trim_start_matches('^').trim_start_matches('~').to_string();
                dependencies.push(DependencyInfo {
                    name: name.clone(),
                    version: version_str,
                    is_direct: false,
                });
            }
        }

        if let Some(deps) = json.get("peerDependencies").and_then(|v| v.as_object()) {
            for (name, version) in deps {
                let version_str = version.as_str().unwrap_or("*").trim_start_matches('^').trim_start_matches('~').to_string();
                dependencies.push(DependencyInfo {
                    name: name.clone(),
                    version: version_str,
                    is_direct: false,
                });
            }
        }

        Ok(DependencyEcosystem {
            name: "npm".to_string(),
            manifest_file: path.to_string_lossy().to_string(),
            lock_file: None,
            dependencies,
            vulnerabilities: Vec::new(),
        })
    }

    pub fn scan_package_lock(path: &Path) -> Result<DependencyEcosystem> {
        let content = std::fs::read_to_string(path)?;
        let json: serde_json::Value = serde_json::from_str(&content)?;
        let mut dependencies = Vec::new();

        if let Some(packages) = json.get("packages").and_then(|v| v.as_object()) {
            for (key, value) in packages {
                if key.is_empty() || key == "" {
                    continue;
                }
                let version = value.get("version").and_then(|v| v.as_str()).unwrap_or("*");
                let name = key.trim_start_matches("node_modules/");
                dependencies.push(DependencyInfo {
                    name: name.to_string(),
                    version: version.to_string(),
                    is_direct: true,
                });
            }
        }

        Ok(DependencyEcosystem {
            name: "npm".to_string(),
            manifest_file: path.to_string_lossy().to_string(),
            lock_file: None,
            dependencies,
            vulnerabilities: Vec::new(),
        })
    }

    pub fn scan_yarn_lock(path: &Path) -> Result<DependencyEcosystem> {
        let content = std::fs::read_to_string(path)?;
        let mut dependencies = Vec::new();

        for line in content.lines() {
            let trimmed = line.trim();
            if !trimmed.starts_with('#') && !trimmed.is_empty() {
                if let Some((name, version)) = Self::parse_yarn_entry(trimmed) {
                    dependencies.push(DependencyInfo {
                        name,
                        version,
                        is_direct: true,
                    });
                }
            }
        }

        Ok(DependencyEcosystem {
            name: "npm (yarn)".to_string(),
            manifest_file: path.to_string_lossy().to_string(),
            lock_file: None,
            dependencies,
            vulnerabilities: Vec::new(),
        })
    }

    fn parse_yarn_entry(line: &str) -> Option<(String, String)> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            let version = parts[0].trim_end_matches(':');
            let name = parts[1];
            return Some((name.to_string(), version.to_string()));
        }
        None
    }

    pub fn scan_requirements_txt(path: &Path) -> Result<DependencyEcosystem> {
        let content = std::fs::read_to_string(path)?;
        let mut dependencies = Vec::new();

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("-") {
                continue;
            }

            let name = trimmed.split("==").next().unwrap_or(trimmed);
            let version = if trimmed.contains("==") {
                trimmed.split("==").nth(1).unwrap_or("*")
            } else if trimmed.contains(">=") {
                trimmed.split(">=").nth(1).unwrap_or("*")
            } else {
                "*"
            };

            dependencies.push(DependencyInfo {
                name: name.to_string(),
                version: version.to_string(),
                is_direct: true,
            });
        }

        Ok(DependencyEcosystem {
            name: "pip".to_string(),
            manifest_file: path.to_string_lossy().to_string(),
            lock_file: None,
            dependencies,
            vulnerabilities: Vec::new(),
        })
    }
}