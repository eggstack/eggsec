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
        let mut in_group = false;
        let mut group_name: Option<String> = None;

        for line in content.lines() {
            let trimmed = line.trim();

            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            if trimmed.starts_with("source ") {
                continue;
            }

            if trimmed.starts_with("group ") {
                if trimmed.ends_with("do") {
                    in_group = true;
                    let g = trimmed.trim_start_matches("group ").trim_end_matches(" do");
                    group_name = Some(g.to_string());
                }
                continue;
            }

            if trimmed == "end" {
                in_group = false;
                group_name = None;
                continue;
            }

            if let Some((name, version)) = Self::parse_gem_spec(trimmed) {
                let is_dev = group_name.as_ref().map(|g| {
                    g == "development" || g == "test" || g == "dev"
                }).unwrap_or(false);

                dependencies.push(DependencyInfo {
                    name,
                    version,
                    is_direct: !is_dev,
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
        let mut in_specs = false;

        for line in content.lines() {
            let trimmed = line.trim();

            if trimmed == "[[main]]" {
                in_specs = false;
                if let (Some(name), Some(version)) = (current_name.take(), current_version.take()) {
                    dependencies.push(DependencyInfo {
                        name,
                        version,
                        is_direct: true,
                    });
                }
                continue;
            }

            if trimmed.starts_with("specs = [") {
                in_specs = true;
                continue;
            }

            if trimmed == "]" && in_specs {
                in_specs = false;
                continue;
            }

            if !in_specs && (trimmed.starts_with("PATH") || trimmed.starts_with("GEM") || trimmed.starts_with("DEP") || trimmed.starts_with("BUNDLED")) {
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
                if let (Some(name), Some(version)) = (current_name.take(), current_version.take()) {
                    dependencies.push(DependencyInfo {
                        name,
                        version,
                        is_direct: true,
                    });
                }
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

    fn parse_gem_spec(line: &str) -> Option<(String, String)> {
        if !line.starts_with("gem ") {
            return None;
        }

        let rest = line[4..].trim();

        let name = if rest.starts_with('\'') {
            rest.split('\'').nth(1)?.to_string()
        } else if rest.starts_with('"') {
            rest.split('"').nth(1)?.to_string()
        } else {
            let parts: Vec<&str> = rest.split_whitespace().collect();
            if parts.is_empty() {
                return None;
            }
            parts[0].to_string()
        };

        let version = if let Some(colon_pos) = line.find(':') {
            let after_colon = &line[colon_pos + 1..].trim();

            if after_colon.starts_with("git:") || after_colon.starts_with("path:") || after_colon.starts_with("github:") {
                "*".to_string()
            } else if let Some(ver) = after_colon.split_whitespace().nth(0) {
                ver.trim_matches(',').to_string()
            } else {
                "*".to_string()
            }
        } else {
            let version_indicators = ["~>", ">=", "<=", "=", "!"];
            for indicator in version_indicators {
                if let Some(pos) = line.find(indicator) {
                    let potential_version = line[pos..].trim().trim_start_matches(indicator).trim().trim_matches(',');
                    if !potential_version.is_empty() && potential_version != line[..pos].trim() {
                        let cleaned = potential_version.trim_matches('"').trim_matches('\'');
                        if !cleaned.is_empty() && cleaned != "v" && !cleaned.starts_with("git") && !cleaned.starts_with("path") {
                            return Some((name, cleaned.to_string()));
                        }
                    }
                }
            }
            "*".to_string()
        };

        if !name.is_empty() {
            Some((name, version))
        } else {
            None
        }
    }
}