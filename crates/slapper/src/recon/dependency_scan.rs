use crate::error::Result;
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

pub use crate::types::Severity;

struct ParsedDependency {
    name: String,
    version: String,
}

pub struct DependencyScanner {
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
                .map(|e| e.vulnerabilities.iter().filter(|v| v.severity == Severity::Critical).count())
                .sum(),
            high: ecosystems
                .iter()
                .map(|e| e.vulnerabilities.iter().filter(|v| v.severity == Severity::High).count())
                .sum(),
            medium: ecosystems
                .iter()
                .map(|e| e.vulnerabilities.iter().filter(|v| v.severity == Severity::Medium).count())
                .sum(),
            low: ecosystems
                .iter()
                .map(|e| e.vulnerabilities.iter().filter(|v| v.severity == Severity::Low).count())
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
        let mut manifests = Vec::new();
        let supported_files = [
            "Cargo.toml",
            "Cargo.lock",
            "package.json",
            "package-lock.json",
            "yarn.lock",
            "requirements.txt",
            "Pipfile.lock",
            "poetry.lock",
            "go.mod",
            "go.sum",
            "Gemfile",
            "Gemfile.lock",
            "pom.xml",
            "build.gradle",
            "composer.json",
            "composer.lock",
        ];

        self.search_manifests(path, &supported_files, &mut manifests, 3);
        manifests
    }

    fn search_manifests(&self, dir: &Path, targets: &[&str], found: &mut Vec<PathBuf>, depth: usize) {
        if depth == 0 {
            return;
        }

        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return,
        };

        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            let name = entry.file_name();
            let name_str = name.to_string_lossy();

            if path.is_dir() {
                if name_str == "node_modules" || name_str == "target" || name_str == ".git"
                    || name_str == "vendor" || name_str == "__pycache__" || name_str == ".venv"
                {
                    continue;
                }
                self.search_manifests(&path, targets, found, depth - 1);
            } else if path.is_file() && targets.contains(&name_str.as_ref()) {
                found.push(path);
            }
        }
    }

    async fn parse_manifest(&self, path: &Path) -> Result<DependencyEcosystem> {
        let file_name = path.file_name().unwrap_or_default().to_string_lossy().to_string();

        match file_name.as_str() {
            "Cargo.toml" => self.parse_cargo_toml(path),
            "Cargo.lock" => self.parse_cargo_lock(path),
            "package.json" => self.parse_package_json(path),
            "package-lock.json" => self.parse_package_lock(path),
            "yarn.lock" => self.parse_yarn_lock(path),
            "requirements.txt" => self.parse_requirements_txt(path),
            "go.mod" => self.parse_go_mod(path),
            "go.sum" => self.parse_go_sum(path),
            "Gemfile" => self.parse_gemfile(path),
            "Gemfile.lock" => self.parse_gemfile_lock(path),
            "composer.json" => self.parse_composer_json(path),
            "pom.xml" => self.parse_pom_xml(path),
            _ => self.parse_unknown(path),
        }
    }

    fn parse_cargo_toml(&self, path: &Path) -> Result<DependencyEcosystem> {
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
                if let Some((name, version)) = self.parse_toml_dep(trimmed) {
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

    fn parse_cargo_lock(&self, path: &Path) -> Result<DependencyEcosystem> {
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

    fn parse_package_json(&self, path: &Path) -> Result<DependencyEcosystem> {
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

        let lock_file = path.parent().and_then(|p| {
            let lock = p.join("package-lock.json");
            if lock.exists() {
                Some(lock.to_string_lossy().to_string())
            } else {
                let yarn = p.join("yarn.lock");
                if yarn.exists() {
                    Some(yarn.to_string_lossy().to_string())
                } else {
                    None
                }
            }
        });

        Ok(DependencyEcosystem {
            name: "JavaScript (npm)".to_string(),
            manifest_file: path.to_string_lossy().to_string(),
            lock_file,
            dependencies,
            vulnerabilities: Vec::new(),
        })
    }

    fn parse_package_lock(&self, path: &Path) -> Result<DependencyEcosystem> {
        let content = std::fs::read_to_string(path)?;
        let json: serde_json::Value = serde_json::from_str(&content)?;
        let mut dependencies = Vec::new();

        if let Some(pkgs) = json.get("packages").and_then(|v| v.as_object()) {
            for (key, pkg) in pkgs {
                if key.is_empty() {
                    continue;
                }
                if let Some(version) = pkg.get("version").and_then(|v| v.as_str()) {
                    let name = key.trim_start_matches("node_modules/").to_string();
                    dependencies.push(DependencyInfo {
                        name,
                        version: version.to_string(),
                        is_direct: !key.contains('/'),
                    });
                }
            }
        } else if let Some(deps) = json.get("dependencies").and_then(|v| v.as_object()) {
            for (name, info) in deps {
                if let Some(version) = info.get("version").and_then(|v| v.as_str()) {
                    dependencies.push(DependencyInfo {
                        name: name.clone(),
                        version: version.to_string(),
                        is_direct: true,
                    });
                }
            }
        }

        Ok(DependencyEcosystem {
            name: "JavaScript (npm)".to_string(),
            manifest_file: path.to_string_lossy().to_string(),
            lock_file: None,
            dependencies,
            vulnerabilities: Vec::new(),
        })
    }

    fn parse_yarn_lock(&self, path: &Path) -> Result<DependencyEcosystem> {
        let content = std::fs::read_to_string(path)?;
        let mut dependencies = Vec::new();

        let mut current_name: Option<String> = None;

        for line in content.lines() {
            if !line.starts_with(' ') && !line.starts_with('\t') && line.contains('@') && line.ends_with(':') {
                let name_part = line.trim_end_matches(':');
                if let Some(at_pos) = name_part.rfind('@') {
                    current_name = Some(name_part[..at_pos].to_string());
                }
            }

            if line.starts_with("  version ") {
                if let Some(name) = current_name.take() {
                    let version = line.trim_start_matches("  version ").trim_matches('"').to_string();
                    dependencies.push(DependencyInfo {
                        name,
                        version,
                        is_direct: true,
                    });
                }
            }
        }

        Ok(DependencyEcosystem {
            name: "JavaScript (yarn)".to_string(),
            manifest_file: path.to_string_lossy().to_string(),
            lock_file: None,
            dependencies,
            vulnerabilities: Vec::new(),
        })
    }

    fn parse_requirements_txt(&self, path: &Path) -> Result<DependencyEcosystem> {
        let content = std::fs::read_to_string(path)?;
        let mut dependencies = Vec::new();

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('-') {
                continue;
            }

            if let Some(dep) = self.parse_python_dep(trimmed) {
                dependencies.push(DependencyInfo {
                    name: dep.name,
                    version: dep.version,
                    is_direct: true,
                });
            }
        }

        Ok(DependencyEcosystem {
            name: "Python (pip)".to_string(),
            manifest_file: path.to_string_lossy().to_string(),
            lock_file: None,
            dependencies,
            vulnerabilities: Vec::new(),
        })
    }

    fn parse_go_mod(&self, path: &Path) -> Result<DependencyEcosystem> {
        let content = std::fs::read_to_string(path)?;
        let mut dependencies = Vec::new();
        let mut in_require = false;

        for line in content.lines() {
            let trimmed = line.trim();

            if trimmed.starts_with("require (") {
                in_require = true;
                continue;
            }
            if trimmed == ")" && in_require {
                in_require = false;
                continue;
            }

            if trimmed.starts_with("require ") && !trimmed.contains('(') {
                let parts: Vec<&str> = trimmed.trim_start_matches("require ").split_whitespace().collect();
                if parts.len() >= 2 {
                    dependencies.push(DependencyInfo {
                        name: parts[0].to_string(),
                        version: parts[1].trim_start_matches('v').to_string(),
                        is_direct: true,
                    });
                }
                continue;
            }

            if in_require && !trimmed.is_empty() {
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() >= 2 {
                    dependencies.push(DependencyInfo {
                        name: parts[0].to_string(),
                        version: parts[1].trim_start_matches('v').to_string(),
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

    fn parse_go_sum(&self, path: &Path) -> Result<DependencyEcosystem> {
        let content = std::fs::read_to_string(path)?;
        let mut dependencies = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for line in content.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let key = format!("{}:{}", parts[0], parts[1]);
                if seen.insert(key) {
                    let version = parts[1].trim_start_matches('v').split('/').next().unwrap_or(parts[1]).to_string();
                    dependencies.push(DependencyInfo {
                        name: parts[0].to_string(),
                        version,
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

    fn parse_gemfile(&self, path: &Path) -> Result<DependencyEcosystem> {
        let content = std::fs::read_to_string(path)?;
        let mut dependencies = Vec::new();

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("gem ") {
                let parts: Vec<&str> = trimmed.split(',').collect();
                if !parts.is_empty() {
                    let name = parts[0]
                        .trim_start_matches("gem ")
                        .trim_matches(|c: char| c == '\'' || c == '"')
                        .trim()
                        .to_string();

                    let version = if parts.len() >= 2 {
                        parts[1].trim().trim_matches(|c: char| c == '\'' || c == '"' || c == '~' || c == '>').trim().to_string()
                    } else {
                        "*".to_string()
                    };

                    dependencies.push(DependencyInfo {
                        name,
                        version,
                        is_direct: true,
                    });
                }
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

    fn parse_gemfile_lock(&self, path: &Path) -> Result<DependencyEcosystem> {
        let content = std::fs::read_to_string(path)?;
        let mut dependencies = Vec::new();
        let mut current_name: Option<String> = None;

        for line in content.lines() {
            let trimmed = line.trim();

            if trimmed.starts_with("specs:") || trimmed.starts_with("  ") && trimmed.contains('(') {
                if let Some(name) = current_name.take() {
                    if let Some(open_paren) = trimmed.find('(') {
                        if let Some(close_paren) = trimmed.find(')') {
                            let version = trimmed[open_paren + 1..close_paren].to_string();
                            dependencies.push(DependencyInfo {
                                name,
                                version,
                                is_direct: true,
                            });
                        }
                    }
                }
            }

            if trimmed.starts_with("    ") && !trimmed.starts_with("      ") {
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if !parts.is_empty() {
                    let name = parts[0].to_string();
                    current_name = Some(name);
                }
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

    fn parse_composer_json(&self, path: &Path) -> Result<DependencyEcosystem> {
        let content = std::fs::read_to_string(path)?;
        let json: serde_json::Value = serde_json::from_str(&content)?;
        let mut dependencies = Vec::new();

        for key in &["require", "require-dev"] {
            if let Some(deps) = json.get(*key).and_then(|v| v.as_object()) {
                for (name, version) in deps {
                    let version_str = version.as_str().unwrap_or("*").trim_start_matches('^').to_string();
                    dependencies.push(DependencyInfo {
                        name: name.clone(),
                        version: version_str,
                        is_direct: *key == "require",
                    });
                }
            }
        }

        Ok(DependencyEcosystem {
            name: "PHP (Composer)".to_string(),
            manifest_file: path.to_string_lossy().to_string(),
            lock_file: None,
            dependencies,
            vulnerabilities: Vec::new(),
        })
    }

    fn parse_pom_xml(&self, path: &Path) -> Result<DependencyEcosystem> {
        let content = std::fs::read_to_string(path)?;
        let mut dependencies = Vec::new();

        let mut in_deps = false;
        let mut current_group: Option<String> = None;
        let mut current_artifact: Option<String> = None;
        let mut current_version: Option<String> = None;

        for line in content.lines() {
            let trimmed = line.trim();

            if trimmed.contains("<dependencies>") {
                in_deps = true;
                continue;
            }
            if trimmed.contains("</dependencies>") {
                in_deps = false;
                if let (Some(a), Some(v)) = (current_artifact.take(), current_version.take()) {
                    let name = format!("{}:{}", current_group.take().unwrap_or_default(), a);
                    dependencies.push(DependencyInfo {
                        name,
                        version: v,
                        is_direct: true,
                    });
                }
                continue;
            }

            if in_deps {
                if trimmed.contains("<groupId>") && trimmed.contains("</groupId>") {
                    current_group = Some(
                        trimmed
                            .split("<groupId>")
                            .nth(1)
                            .unwrap_or("")
                            .split("</groupId>")
                            .next()
                            .unwrap_or("")
                            .to_string(),
                    );
                }
                if trimmed.contains("<artifactId>") && trimmed.contains("</artifactId>") {
                    current_artifact = Some(
                        trimmed
                            .split("<artifactId>")
                            .nth(1)
                            .unwrap_or("")
                            .split("</artifactId>")
                            .next()
                            .unwrap_or("")
                            .to_string(),
                    );
                }
                if trimmed.contains("<version>") && trimmed.contains("</version>") {
                    current_version = Some(
                        trimmed
                            .split("<version>")
                            .nth(1)
                            .unwrap_or("")
                            .split("</version>")
                            .next()
                            .unwrap_or("")
                            .to_string(),
                    );
                    if let (Some(a), Some(v)) = (current_artifact.take(), current_version.take()) {
                        let name = format!("{}:{}", current_group.take().unwrap_or_default(), a);
                        dependencies.push(DependencyInfo {
                            name,
                            version: v,
                            is_direct: true,
                        });
                    }
                }
            }
        }

        Ok(DependencyEcosystem {
            name: "Java (Maven)".to_string(),
            manifest_file: path.to_string_lossy().to_string(),
            lock_file: None,
            dependencies,
            vulnerabilities: Vec::new(),
        })
    }

    fn parse_unknown(&self, path: &Path) -> Result<DependencyEcosystem> {
        Ok(DependencyEcosystem {
            name: "Unknown".to_string(),
            manifest_file: path.to_string_lossy().to_string(),
            lock_file: None,
            dependencies: Vec::new(),
            vulnerabilities: Vec::new(),
        })
    }

    fn parse_toml_dep(&self, line: &str) -> Option<(String, String)> {
        if line.is_empty() || line.starts_with('#') {
            return None;
        }

        let parts: Vec<&str> = line.splitn(2, '=').collect();
        if parts.len() < 2 {
            return None;
        }

        let name = parts[0].trim().to_string();
        let value = parts[1].trim();

        if value.starts_with('{') {
            if let Some(ver_start) = value.find("version") {
                let after = &value[ver_start..];
                if let Some(eq) = after.find('=') {
                    let rest = after[eq + 1..].trim();
                    let ver = rest.trim_matches('"').trim_matches('\'').split(',').next().unwrap_or(rest).trim().trim_matches('"').trim_matches('\'').to_string();
                    return Some((name, ver));
                }
            }
            None
        } else {
            let version = value.trim_matches('"').trim_matches('\'').to_string();
            Some((name, version))
        }
    }

    fn parse_python_dep(&self, line: &str) -> Option<ParsedDependency> {
        if line.contains("==") {
            let parts: Vec<&str> = line.splitn(2, "==").collect();
            if parts.len() == 2 {
                return Some(ParsedDependency {
                    name: parts[0].trim().to_string(),
                    version: parts[1].trim().to_string(),
                });
            }
        }

        if line.contains(">=") {
            let parts: Vec<&str> = line.splitn(2, ">=").collect();
            if parts.len() == 2 {
                let ver = parts[1].trim().split(',').next().unwrap_or("").trim().to_string();
                return Some(ParsedDependency {
                    name: parts[0].trim().to_string(),
                    version: ver,
                });
            }
        }

        if line.contains("~=") {
            let parts: Vec<&str> = line.splitn(2, "~=").collect();
            if parts.len() == 2 {
                return Some(ParsedDependency {
                    name: parts[0].trim().to_string(),
                    version: parts[1].trim().to_string(),
                });
            }
        }

        if !line.contains('=') && !line.contains('>') && !line.contains('<') && !line.contains('!') {
            return Some(ParsedDependency {
                name: line.trim().to_string(),
                version: "*".to_string(),
            });
        }

        None
    }

    pub async fn check_vulnerabilities(&self, report: &mut DependencyScanReport) -> Result<()> {
        for ecosystem in &mut report.ecosystems {
            let vulns = self.check_ecosystem_vulns(ecosystem).await;
            ecosystem.vulnerabilities = vulns;
        }

        report.total_vulnerabilities = report.ecosystems.iter().map(|e| e.vulnerabilities.len()).sum();
        report.summary = DependencySummary {
            critical: report.ecosystems.iter().map(|e| e.vulnerabilities.iter().filter(|v| v.severity == Severity::Critical).count()).sum(),
            high: report.ecosystems.iter().map(|e| e.vulnerabilities.iter().filter(|v| v.severity == Severity::High).count()).sum(),
            medium: report.ecosystems.iter().map(|e| e.vulnerabilities.iter().filter(|v| v.severity == Severity::Medium).count()).sum(),
            low: report.ecosystems.iter().map(|e| e.vulnerabilities.iter().filter(|v| v.severity == Severity::Low).count()).sum(),
        };

        Ok(())
    }

    async fn check_ecosystem_vulns(&self, ecosystem: &DependencyEcosystem) -> Vec<DependencyVulnerability> {
        let mut vulns = Vec::new();

        for dep in &ecosystem.dependencies {
            match ecosystem.name.as_str() {
                "Rust (Cargo)" => {
                    if let Some(v) = self.check_rustsec(&dep.name, &dep.version).await {
                        vulns.extend(v);
                    }
                }
                "JavaScript (npm)" | "JavaScript (yarn)" => {
                    if let Some(v) = self.check_npm_vulns(&dep.name, &dep.version).await {
                        vulns.extend(v);
                    }
                }
                "Python (pip)" => {
                    if let Some(v) = self.check_pypi_vulns(&dep.name, &dep.version).await {
                        vulns.extend(v);
                    }
                }
                _ => {}
            }
        }

        vulns
    }

    async fn check_rustsec(&self, name: &str, version: &str) -> Option<Vec<DependencyVulnerability>> {
        let url = format!("https://api.github.com/search/issues?q={}%20in:title%20repo:rustsec/advisory-db", name);

        let response = match self.client.get(&url).header("Accept", "application/vnd.github.v3+json").send().await {
            Ok(r) => r,
            Err(_) => return None,
        };

        let json: serde_json::Value = match response.json().await {
            Ok(j) => j,
            Err(_) => return None,
        };

        let mut vulns = Vec::new();
        if let Some(items) = json.get("items").and_then(|v| v.as_array()) {
            for item in items.iter().take(5) {
                let title = item.get("title").and_then(|v| v.as_str()).unwrap_or("Unknown").to_string();
                let html_url = item.get("html_url").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let title_lower = title.to_lowercase();
                let severity = if title_lower.contains("critical") || title_lower.contains("rce") {
                    Severity::Critical
                } else if title_lower.contains("high") || title_lower.contains("overflow") {
                    Severity::High
                } else {
                    Severity::Medium
                };

                vulns.push(DependencyVulnerability {
                    dependency: name.to_string(),
                    version: version.to_string(),
                    cve_id: html_url.clone(),
                    title,
                    severity,
                    fixed_version: None,
                    description: format!("Potential vulnerability in {} {}", name, version),
                    references: vec![html_url],
                });
            }
        }

        if vulns.is_empty() {
            None
        } else {
            Some(vulns)
        }
    }

    async fn check_npm_vulns(&self, name: &str, version: &str) -> Option<Vec<DependencyVulnerability>> {
        let url = format!("https://registry.npmjs.org/{}/{}", name, version);

        let response = match self.client.get(&url).send().await {
            Ok(r) => r,
            Err(_) => return None,
        };

        if response.status().is_success() {
            None
        } else {
            Some(vec![DependencyVulnerability {
                dependency: name.to_string(),
                version: version.to_string(),
                cve_id: "N/A".to_string(),
                title: format!("Package {}@{} not found in registry", name, version),
                severity: Severity::Low,
                fixed_version: None,
                description: "Package version not found in npm registry".to_string(),
                references: Vec::new(),
            }])
        }
    }

    async fn check_pypi_vulns(&self, name: &str, _version: &str) -> Option<Vec<DependencyVulnerability>> {
        let url = format!("https://pypi.org/pypi/{}/json", name);

        let response = match self.client.get(&url).send().await {
            Ok(r) => r,
            Err(_) => return None,
        };

        if response.status().is_success() {
            None
        } else {
            Some(vec![DependencyVulnerability {
                dependency: name.to_string(),
                version: _version.to_string(),
                cve_id: "N/A".to_string(),
                title: format!("Package {} not found on PyPI", name),
                severity: Severity::Low,
                fixed_version: None,
                description: "Package not found in PyPI registry".to_string(),
                references: Vec::new(),
            }])
        }
    }
}

pub async fn scan_dependencies(project_path: &str) -> Result<DependencyScanReport> {
    let scanner = DependencyScanner::new()?;
    scanner.scan_project(project_path).await
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
    fn test_parse_toml_dep_simple() {
        let scanner = DependencyScanner::new().unwrap();
        let result = scanner.parse_toml_dep("serde = \"1.0\"");
        assert!(result.is_some());
        let (name, version) = result.unwrap();
        assert_eq!(name, "serde");
        assert_eq!(version, "1.0");
    }

    #[test]
    fn test_parse_toml_dep_complex() {
        let scanner = DependencyScanner::new().unwrap();
        let result = scanner.parse_toml_dep("tokio = { version = \"1.0\", features = [\"full\"] }");
        assert!(result.is_some());
        let (name, version) = result.unwrap();
        assert_eq!(name, "tokio");
        assert_eq!(version, "1.0");
    }

    #[test]
    fn test_parse_toml_dep_comment() {
        let scanner = DependencyScanner::new().unwrap();
        let result = scanner.parse_toml_dep("# this is a comment");
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_python_dep_exact() {
        let scanner = DependencyScanner::new().unwrap();
        let result = scanner.parse_python_dep("requests==2.28.0");
        assert!(result.is_some());
        let dep = result.unwrap();
        assert_eq!(dep.name, "requests");
        assert_eq!(dep.version, "2.28.0");
    }

    #[test]
    fn test_parse_python_dep_gte() {
        let scanner = DependencyScanner::new().unwrap();
        let result = scanner.parse_python_dep("flask>=2.0.0");
        assert!(result.is_some());
        let dep = result.unwrap();
        assert_eq!(dep.name, "flask");
        assert_eq!(dep.version, "2.0.0");
    }

    #[test]
    fn test_parse_python_dep_bare() {
        let scanner = DependencyScanner::new().unwrap();
        let result = scanner.parse_python_dep("numpy");
        assert!(result.is_some());
        let dep = result.unwrap();
        assert_eq!(dep.name, "numpy");
        assert_eq!(dep.version, "*");
    }

    #[test]
    fn test_dependency_info_creation() {
        let dep = DependencyInfo {
            name: "test-pkg".to_string(),
            version: "1.0.0".to_string(),
            is_direct: true,
        };
        assert_eq!(dep.name, "test-pkg");
        assert!(dep.is_direct);
    }

    #[test]
    fn test_dependency_vulnerability_creation() {
        let vuln = DependencyVulnerability {
            dependency: "test-pkg".to_string(),
            version: "1.0.0".to_string(),
            cve_id: "CVE-2024-1234".to_string(),
            title: "Test vulnerability".to_string(),
            severity: Severity::High,
            fixed_version: Some("1.0.1".to_string()),
            description: "Test description".to_string(),
            references: vec!["https://example.com".to_string()],
        };
        assert_eq!(vuln.cve_id, "CVE-2024-1234");
        assert_eq!(vuln.severity, Severity::High);
    }

    #[test]
    fn test_dependency_summary_creation() {
        let summary = DependencySummary {
            critical: 1,
            high: 2,
            medium: 3,
            low: 4,
        };
        assert_eq!(summary.critical, 1);
        assert_eq!(summary.high, 2);
    }

    #[tokio::test]
    async fn test_scan_nonexistent_project() {
        let scanner = DependencyScanner::new().unwrap();
        let result = scanner.scan_project("/nonexistent/path/that/does/not/exist").await;
        assert!(result.is_ok());
        let report = result.unwrap();
        assert_eq!(report.total_dependencies, 0);
    }
}
