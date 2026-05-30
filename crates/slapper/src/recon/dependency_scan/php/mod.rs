use crate::error::Result;
use crate::recon::dependency_scan::{DependencyEcosystem, DependencyInfo};
use std::path::Path;

pub struct PhpScanner;

impl Default for PhpScanner {
    fn default() -> Self {
        Self::new()
    }
}

impl PhpScanner {
    pub fn new() -> Self {
        Self
    }

    pub fn scan_composer_json(path: &Path) -> Result<DependencyEcosystem> {
        let content = std::fs::read_to_string(path)?;
        let json: serde_json::Value = serde_json::from_str(&content)?;
        let mut dependencies = Vec::new();

        if let Some(require) = json.get("require").and_then(|v| v.as_object()) {
            for (name, version) in require {
                if name == "php" {
                    continue;
                }
                let version_str = version
                    .as_str()
                    .unwrap_or("*")
                    .trim_start_matches('^')
                    .trim_start_matches('~')
                    .trim_start_matches(">=")
                    .to_string();
                dependencies.push(DependencyInfo {
                    name: name.clone(),
                    version: version_str,
                    is_direct: true,
                });
            }
        }

        if let Some(require_dev) = json.get("require-dev").and_then(|v| v.as_object()) {
            for (name, version) in require_dev {
                if name == "php" {
                    continue;
                }
                let version_str = version
                    .as_str()
                    .unwrap_or("*")
                    .trim_start_matches('^')
                    .trim_start_matches('~')
                    .trim_start_matches(">=")
                    .to_string();
                dependencies.push(DependencyInfo {
                    name: name.clone(),
                    version: version_str,
                    is_direct: false,
                });
            }
        }

        let lock_file = path
            .parent()
            .map(|p| p.join("composer.lock"))
            .filter(|p| p.exists())
            .map(|p| p.to_string_lossy().to_string());

        Ok(DependencyEcosystem {
            name: "PHP (Composer)".to_string(),
            manifest_file: path.to_string_lossy().to_string(),
            lock_file,
            dependencies,
            vulnerabilities: Vec::new(),
        })
    }
}