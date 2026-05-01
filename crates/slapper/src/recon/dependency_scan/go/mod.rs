use crate::error::Result;
use crate::recon::dependency_scan::{DependencyEcosystem, DependencyInfo};
use std::path::Path;

pub struct GoScanner;

impl Default for GoScanner {
    fn default() -> Self {
        Self::new()
    }
}

impl GoScanner {
    pub fn new() -> Self {
        Self
    }

    pub fn scan_go_mod(path: &Path) -> Result<DependencyEcosystem> {
        let content = std::fs::read_to_string(path)?;
        let mut dependencies = Vec::new();
        let mut in_require = false;

        for line in content.lines() {
            let trimmed = line.trim();

            if trimmed.starts_with("require (") {
                in_require = true;
                continue;
            }
            if in_require && trimmed == ")" {
                in_require = false;
                continue;
            }

            if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with("module") || trimmed.starts_with("go ") {
                continue;
            }

            if in_require || (!trimmed.starts_with("replace") && !trimmed.starts_with("exclude") && !trimmed.starts_with("=>")) {
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() >= 2 {
                    dependencies.push(DependencyInfo {
                        name: parts[0].to_string(),
                        version: parts[1].to_string(),
                        is_direct: true,
                    });
                }
            }
        }

        Ok(DependencyEcosystem {
            name: "Go".to_string(),
            manifest_file: path.to_string_lossy().to_string(),
            lock_file: None,
            dependencies,
            vulnerabilities: Vec::new(),
        })
    }

    pub fn scan_go_sum(path: &Path) -> Result<DependencyEcosystem> {
        let content = std::fs::read_to_string(path)?;
        let mut dependencies = Vec::new();

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with("//") {
                continue;
            }

            if let Some(hash_pos) = trimmed.find(" h1:") {
                let module_line = &trimmed[..hash_pos];
                if let Some(space_pos) = module_line.rfind(' ') {
                    let name = &module_line[..space_pos];
                    let version = &module_line[space_pos + 1..];
                    dependencies.push(DependencyInfo {
                        name: name.to_string(),
                        version: version.to_string(),
                        is_direct: true,
                    });
                }
            }
        }

        Ok(DependencyEcosystem {
            name: "Go".to_string(),
            manifest_file: path.to_string_lossy().to_string(),
            lock_file: None,
            dependencies,
            vulnerabilities: Vec::new(),
        })
    }
}