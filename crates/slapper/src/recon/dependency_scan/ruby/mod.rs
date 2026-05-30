use crate::error::Result;
use crate::recon::dependency_scan::{DependencyEcosystem, DependencyInfo};
use std::path::Path;

pub struct RubyScanner;

impl Default for RubyScanner {
    fn default() -> Self {
        Self::new()
    }
}

impl RubyScanner {
    pub fn new() -> Self {
        Self
    }

    pub fn scan_gemfile(path: &Path) -> Result<DependencyEcosystem> {
        let content = std::fs::read_to_string(path)?;
        let mut dependencies = Vec::new();

        for line in content.lines() {
            let trimmed = line.trim();

            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            if let Some((name, version)) = Self::parse_gemfile_dep(trimmed) {
                dependencies.push(DependencyInfo {
                    name,
                    version,
                    is_direct: true,
                });
            }
        }

        Ok(DependencyEcosystem {
            name: "Ruby (Bundler)".to_string(),
            manifest_file: path.to_string_lossy().to_string(),
            lock_file: None,
            dependencies,
            vulnerabilities: Vec::new(),
        })
    }

    pub fn scan_gemfile_lock(path: &Path) -> Result<DependencyEcosystem> {
        let content = std::fs::read_to_string(path)?;
        let mut dependencies = Vec::new();
        let mut current_name: Option<String> = None;
        let mut current_version: Option<String> = None;

        for line in content.lines() {
            let trimmed = line.trim();

            if trimmed == "[[main]]" || trimmed.starts_with("  specs = [") {
                continue;
            }

            if trimmed.starts_with("PATH") || trimmed.starts_with("GEM") || trimmed.starts_with("DEP") {
                if let (Some(name), Some(version)) = (current_name.take(), current_version.take()) {
                    dependencies.push(DependencyInfo {
                        name,
                        version,
                        is_direct: true,
                    });
                }
                continue;
            }

            if trimmed.starts_with("  name = ") {
                current_name = Some(
                    trimmed
                        .trim_start_matches("  name = ")
                        .trim_matches('"')
                        .to_string(),
                );
            } else if trimmed.starts_with("  version = ") {
                current_version = Some(
                    trimmed
                        .trim_start_matches("  version = ")
                        .trim_matches('"')
                        .to_string(),
                );
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
            name: "Ruby (Bundler)".to_string(),
            manifest_file: path.to_string_lossy().to_string(),
            lock_file: None,
            dependencies,
            vulnerabilities: Vec::new(),
        })
    }

    fn parse_gemfile_dep(line: &str) -> Option<(String, String)> {
        let source = if line.starts_with("source ") {
            return None;
        } else if line.starts_with("gem ") || line.starts_with("gem '") {
            line.trim_start_matches("gem ")
        } else if line.starts_with("path ") || line.starts_with("git ") {
            return None;
        } else {
            line
        };

        let name = if source.starts_with('\'') {
            source.split('\'').nth(1)?.to_string()
        } else if source.starts_with('"') {
            source.split('"').nth(1)?.to_string()
        } else {
            source.split(',').next()?.trim().to_string()
        };

        let version = if line.contains(',') {
            let version_part = line.split(',').nth(1).unwrap_or("*");
            version_part
                .trim()
                .trim_start_matches("version => \"")
                .trim_start_matches("version => '")
                .trim_end_matches("\"")
                .trim_end_matches("'")
                .to_string()
        } else {
            "*".to_string()
        };

        if !name.is_empty() {
            Some((name, version))
        } else {
            None
        }
    }
}