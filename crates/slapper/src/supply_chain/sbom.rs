use crate::error::Result;
use crate::supply_chain::Severity;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SbomReport {
    pub format: SbomFormat,
    pub project_name: String,
    pub version: String,
    pub generated_at: String,
    pub components: Vec<SbomComponent>,
    pub vulnerabilities: Vec<SbomVulnerability>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SbomFormat {
    CycloneDx,
    Spdx,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SbomComponent {
    pub name: String,
    pub version: String,
    pub ecosystem: String,
    pub purl: String,
    pub licenses: Vec<String>,
    pub is_direct: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SbomVulnerability {
    pub component: String,
    pub cve_id: String,
    pub severity: Severity,
    pub description: String,
}

pub struct SbomGenerator;

impl Default for SbomGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl SbomGenerator {
    pub fn new() -> Self {
        Self
    }

    pub fn generate_from_cargo(&self, project_path: &str) -> Result<SbomReport> {
        let cargo_toml = Path::new(project_path).join("Cargo.toml");
        let cargo_lock = Path::new(project_path).join("Cargo.lock");

        let mut components = Vec::new();

        if cargo_toml.exists() {
            let content = std::fs::read_to_string(&cargo_toml)?;
            let project_name = self
                .extract_package_name(&content)
                .unwrap_or_else(|| "unknown".to_string());
            let version = self
                .extract_version(&content)
                .unwrap_or_else(|| "0.0.0".to_string());

            if cargo_lock.exists() {
                let lock_content = std::fs::read_to_string(&cargo_lock)?;
                components = self.parse_cargo_lock(&lock_content);
            }

            let direct_deps = self.parse_cargo_toml_deps(&content);
            for comp in &mut components {
                comp.is_direct = direct_deps.contains(&comp.name);
            }

            return Ok(SbomReport {
                format: SbomFormat::CycloneDx,
                project_name,
                version,
                generated_at: chrono::Utc::now().to_rfc3339(),
                components,
                vulnerabilities: Vec::new(),
            });
        }

        Err(crate::error::SlapperError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Cargo.toml not found",
        )))
    }

    pub fn generate_from_npm(&self, project_path: &str) -> Result<SbomReport> {
        let package_json = Path::new(project_path).join("package.json");
        let package_lock = Path::new(project_path).join("package-lock.json");

        if package_json.exists() {
            let content = std::fs::read_to_string(&package_json)?;
            let json: serde_json::Value = serde_json::from_str(&content)?;
            let project_name = json
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();
            let version = json
                .get("version")
                .and_then(|v| v.as_str())
                .unwrap_or("0.0.0")
                .to_string();

            let mut components = Vec::new();

            if let Some(deps) = json.get("dependencies").and_then(|v| v.as_object()) {
                for (name, ver) in deps {
                    components.push(SbomComponent {
                        name: name.clone(),
                        version: ver
                            .as_str()
                            .unwrap_or("*")
                            .trim_start_matches('^')
                            .trim_start_matches('~')
                            .to_string(),
                        ecosystem: "npm".to_string(),
                        purl: format!("pkg:npm/{}@{}", name, ver.as_str().unwrap_or("*")),
                        licenses: Vec::new(),
                        is_direct: true,
                    });
                }
            }

            if package_lock.exists() {
                let lock_content = std::fs::read_to_string(&package_lock)?;
                if let Ok(lock_json) = serde_json::from_str::<serde_json::Value>(&lock_content) {
                    if let Some(packages) = lock_json.get("packages").and_then(|v| v.as_object()) {
                        for (key, info) in packages {
                            if key.is_empty() {
                                continue;
                            }
                            let name = key.trim_start_matches("node_modules/").to_string();
                            let ver = info
                                .get("version")
                                .and_then(|v| v.as_str())
                                .unwrap_or("*")
                                .to_string();
                            if !components.iter().any(|c| c.name == name) {
                                components.push(SbomComponent {
                                    name,
                                    version: ver.clone(),
                                    ecosystem: "npm".to_string(),
                                    purl: format!(
                                        "pkg:npm/{}@{}",
                                        key.trim_start_matches("node_modules/"),
                                        ver
                                    ),
                                    licenses: Vec::new(),
                                    is_direct: false,
                                });
                            }
                        }
                    }
                }
            }

            return Ok(SbomReport {
                format: SbomFormat::CycloneDx,
                project_name,
                version,
                generated_at: chrono::Utc::now().to_rfc3339(),
                components,
                vulnerabilities: Vec::new(),
            });
        }

        Err(crate::error::SlapperError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "package.json not found",
        )))
    }

    pub fn generate_from_requirements(&self, project_path: &str) -> Result<SbomReport> {
        let req_file = Path::new(project_path).join("requirements.txt");
        if req_file.exists() {
            let content = std::fs::read_to_string(&req_file)?;
            let mut components = Vec::new();

            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('-') {
                    continue;
                }
                let parts: Vec<&str> = if trimmed.contains("==") {
                    trimmed.splitn(2, "==").collect()
                } else if trimmed.contains(">=") {
                    trimmed.splitn(2, ">=").collect()
                } else {
                    vec![trimmed, "*"]
                };

                if !parts.is_empty() {
                    let name = parts[0].trim().to_string();
                    let version = parts
                        .get(1)
                        .map(|s| s.trim().to_string())
                        .unwrap_or_else(|| "*".to_string());
                    components.push(SbomComponent {
                        name,
                        version,
                        ecosystem: "pypi".to_string(),
                        purl: format!("pkg:pypi/{}", parts[0].trim()),
                        licenses: Vec::new(),
                        is_direct: true,
                    });
                }
            }

            return Ok(SbomReport {
                format: SbomFormat::CycloneDx,
                project_name: project_path
                    .split('/')
                    .next_back()
                    .unwrap_or("unknown")
                    .to_string(),
                version: "0.0.0".to_string(),
                generated_at: chrono::Utc::now().to_rfc3339(),
                components,
                vulnerabilities: Vec::new(),
            });
        }

        Err(crate::error::SlapperError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "requirements.txt not found",
        )))
    }

    pub fn export_cyclonedx(&self, report: &SbomReport) -> Result<String> {
        let mut json = serde_json::json!({
            "bomFormat": "CycloneDX",
            "specVersion": "1.4",
            "metadata": {
                "component": {
                    "name": report.project_name,
                    "version": report.version,
                },
                "timestamp": report.generated_at,
            },
            "components": report.components.iter().map(|c| {
                serde_json::json!({
                    "type": "library",
                    "name": c.name,
                    "version": c.version,
                    "purl": c.purl,
                    "ecosystem": c.ecosystem,
                })
            }).collect::<Vec<_>>(),
        });

        if !report.vulnerabilities.is_empty() {
            json["vulnerabilities"] = serde_json::json!(report
                .vulnerabilities
                .iter()
                .map(|v| {
                    serde_json::json!({
                        "id": v.cve_id,
                        "source": {"name": "NVD"},
                        "ratings": [{"severity": v.severity.as_str()}],
                        "description": v.description,
                        "affects": [{"ref": v.component}],
                    })
                })
                .collect::<Vec<_>>());
        }

        Ok(serde_json::to_string_pretty(&json)?)
    }

    pub fn export_spdx(&self, report: &SbomReport) -> Result<String> {
        let mut output = String::new();
        output.push_str("SPDXVersion: SPDX-2.3\n");
        output.push_str("DataLicense: CC0-1.0\n");
        output.push_str("SPDXID: SPDXRef-DOCUMENT\n");
        output.push_str(&format!("DocumentName: {}\n", report.project_name));
        output.push_str(&format!(
            "DocumentNamespace: https://spdx.org/spdxdocs/{}-{}\n",
            report.project_name, report.version
        ));
        output.push_str(&format!(
            "Creator: Tool: slapper-{}\n",
            env!("CARGO_PKG_VERSION")
        ));
        output.push_str(&format!("Created: {}\n", report.generated_at));
        output.push('\n');

        output.push_str(&format!("## Package: {}\n", report.project_name));
        output.push_str(&format!(
            "SPDXID: SPDXRef-Package-{}\n",
            report.project_name.replace('/', "-")
        ));
        output.push_str(&format!("PackageVersion: {}\n", report.version));
        output.push('\n');

        for component in &report.components {
            output.push_str(&format!("## Package: {}\n", component.name));
            output.push_str(&format!(
                "SPDXID: SPDXRef-Package-{}\n",
                component.name.replace('/', "-")
            ));
            output.push_str(&format!("PackageVersion: {}\n", component.version));
            output.push_str(&format!(
                "ExternalRef: PACKAGE-MANAGER purl {}\n",
                component.purl
            ));
            output.push('\n');
        }

        Ok(output)
    }

    fn extract_package_name(&self, content: &str) -> Option<String> {
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("name") {
                return trimmed
                    .split_once('=')
                    .map(|x| x.1)
                    .map(|s| s.trim().trim_matches('"').trim_matches('\'').to_string());
            }
        }
        None
    }

    fn extract_version(&self, content: &str) -> Option<String> {
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("version") {
                return trimmed
                    .split_once('=')
                    .map(|x| x.1)
                    .map(|s| s.trim().trim_matches('"').trim_matches('\'').to_string());
            }
        }
        None
    }

    fn parse_cargo_lock(&self, content: &str) -> Vec<SbomComponent> {
        let mut components = Vec::new();
        let mut current_name: Option<String> = None;
        let mut current_version: Option<String> = None;

        for line in content.lines() {
            let trimmed = line.trim();

            if trimmed == "[[package]]" {
                if let (Some(name), Some(version)) = (current_name.take(), current_version.take()) {
                    components.push(SbomComponent {
                        name: name.clone(),
                        version: version.clone(),
                        ecosystem: "cargo".to_string(),
                        purl: format!("pkg:cargo/{}@{}", name, version),
                        licenses: Vec::new(),
                        is_direct: false,
                    });
                }
                continue;
            }

            if trimmed.starts_with("name = ") {
                current_name = Some(
                    trimmed
                        .trim_start_matches("name = ")
                        .trim_matches('"')
                        .to_string(),
                );
            } else if trimmed.starts_with("version = ") {
                current_version = Some(
                    trimmed
                        .trim_start_matches("version = ")
                        .trim_matches('"')
                        .to_string(),
                );
            } else if trimmed.starts_with("source = ") {
            }
        }

        if let (Some(name), Some(version)) = (current_name, current_version) {
            components.push(SbomComponent {
                name: name.clone(),
                version: version.clone(),
                ecosystem: "cargo".to_string(),
                purl: format!("pkg:cargo/{}@{}", name, version),
                licenses: Vec::new(),
                is_direct: false,
            });
        }

        components
    }

    fn parse_cargo_toml_deps(&self, content: &str) -> Vec<String> {
        let mut deps = Vec::new();
        let mut in_deps = false;

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed == "[dependencies]" {
                in_deps = true;
                continue;
            }
            if trimmed.starts_with('[') {
                in_deps = false;
                continue;
            }
            if in_deps {
                if let Some((name, _)) = trimmed.split_once('=') {
                    deps.push(name.trim().to_string());
                }
            }
        }

        deps
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sbom_generator_creation() {
        let gen = SbomGenerator::new();
        let _ = gen;
    }

    #[test]
    fn test_extract_package_name() {
        let gen = SbomGenerator::new();
        let content = "name = \"my-crate\"\nversion = \"1.0.0\"";
        let name = gen.extract_package_name(content);
        assert_eq!(name, Some("my-crate".to_string()));
    }

    #[test]
    fn test_extract_version() {
        let gen = SbomGenerator::new();
        let content = "name = \"my-crate\"\nversion = \"1.0.0\"";
        let version = gen.extract_version(content);
        assert_eq!(version, Some("1.0.0".to_string()));
    }

    #[test]
    fn test_parse_cargo_lock() {
        let gen = SbomGenerator::new();
        let content = r#"
[[package]]
name = "serde"
version = "1.0.197"
source = "registry+https://github.com/rust-lang/crates.io-index"

[[package]]
name = "tokio"
version = "1.36.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
"#;
        let components = gen.parse_cargo_lock(content);
        assert_eq!(components.len(), 2);
        assert_eq!(components[0].name, "serde");
        assert_eq!(components[1].name, "tokio");
    }

    #[test]
    fn test_parse_cargo_toml_deps() {
        let gen = SbomGenerator::new();
        let content = r#"
[dependencies]
serde = "1.0"
tokio = { version = "1.0", features = ["full"] }

[dev-dependencies]
criterion = "0.5"
"#;
        let deps = gen.parse_cargo_toml_deps(content);
        assert_eq!(deps.len(), 2);
        assert!(deps.contains(&"serde".to_string()));
        assert!(deps.contains(&"tokio".to_string()));
    }

    #[test]
    fn test_generate_from_cargo_missing() {
        let gen = SbomGenerator::new();
        let result = gen.generate_from_cargo("/nonexistent/path");
        assert!(result.is_err());
    }

    #[test]
    fn test_export_spdx_format() {
        let report = SbomReport {
            format: SbomFormat::Spdx,
            project_name: "test-project".to_string(),
            version: "1.0.0".to_string(),
            generated_at: "2024-01-01T00:00:00Z".to_string(),
            components: vec![SbomComponent {
                name: "serde".to_string(),
                version: "1.0".to_string(),
                ecosystem: "cargo".to_string(),
                purl: "pkg:cargo/serde@1.0".to_string(),
                licenses: Vec::new(),
                is_direct: true,
            }],
            vulnerabilities: Vec::new(),
        };
        let gen = SbomGenerator::new();
        let spdx = gen.export_spdx(&report).unwrap();
        assert!(spdx.contains("SPDXVersion: SPDX-2.3"));
        assert!(spdx.contains("serde"));
    }

    #[test]
    fn test_export_cyclonedx_format() {
        let report = SbomReport {
            format: SbomFormat::CycloneDx,
            project_name: "test-project".to_string(),
            version: "1.0.0".to_string(),
            generated_at: "2024-01-01T00:00:00Z".to_string(),
            components: vec![SbomComponent {
                name: "serde".to_string(),
                version: "1.0".to_string(),
                ecosystem: "cargo".to_string(),
                purl: "pkg:cargo/serde@1.0".to_string(),
                licenses: Vec::new(),
                is_direct: true,
            }],
            vulnerabilities: Vec::new(),
        };
        let gen = SbomGenerator::new();
        let json = gen.export_cyclonedx(&report).unwrap();
        assert!(json.contains("CycloneDX"));
        assert!(json.contains("serde"));
    }

    #[test]
    fn test_sbom_component_creation() {
        let comp = SbomComponent {
            name: "test-pkg".to_string(),
            version: "1.0.0".to_string(),
            ecosystem: "npm".to_string(),
            purl: "pkg:npm/test-pkg@1.0.0".to_string(),
            licenses: vec!["MIT".to_string()],
            is_direct: true,
        };
        assert_eq!(comp.name, "test-pkg");
        assert!(comp.is_direct);
    }

    #[test]
    fn test_sbom_vulnerability_creation() {
        let vuln = SbomVulnerability {
            component: "test-pkg".to_string(),
            cve_id: "CVE-2024-1234".to_string(),
            severity: Severity::High,
            description: "Test vulnerability".to_string(),
        };
        assert_eq!(vuln.cve_id, "CVE-2024-1234");
    }
}
