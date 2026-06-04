use crate::container::Severity;
use crate::error::{SlapperError, Result};
use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerScanResult {
    pub image_name: String,
    pub base_image: Option<String>,
    pub layers: Vec<ImageLayer>,
    pub vulnerabilities: Vec<DockerVulnerability>,
    pub misconfigurations: Vec<DockerMisconfiguration>,
    pub exposed_ports: Vec<u16>,
    pub running_as_root: bool,
    pub has_healthcheck: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageLayer {
    pub layer_id: String,
    pub instruction: String,
    pub size_bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerVulnerability {
    pub package: String,
    pub installed_version: String,
    pub fixed_version: Option<String>,
    pub cve_id: String,
    pub severity: Severity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerMisconfiguration {
    pub check: String,
    pub severity: Severity,
    pub description: String,
    pub recommendation: String,
}

pub struct DockerScanner;

impl Default for DockerScanner {
    fn default() -> Self {
        Self::new()
    }
}

impl DockerScanner {
    pub fn new() -> Self {
        Self
    }

    pub async fn scan_image(&self, image_name: &str) -> Result<DockerScanResult> {
        let mut result = DockerScanResult {
            image_name: image_name.to_string(),
            base_image: None,
            layers: Vec::new(),
            vulnerabilities: Vec::new(),
            misconfigurations: Vec::new(),
            exposed_ports: Vec::new(),
            running_as_root: false,
            has_healthcheck: false,
        };

        if let Ok(metadata) = self.inspect_image(image_name).await {
            result.base_image = metadata
                .get("base_image")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            result.running_as_root = metadata
                .get("user")
                .and_then(|v| v.as_str())
                .map(|u| u.is_empty() || u == "root")
                .unwrap_or(true);
            result.has_healthcheck = metadata
                .get("healthcheck")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            if let Some(ports) = metadata.get("exposed_ports").and_then(|v| v.as_array()) {
                for p in ports {
                    if let Some(port_str) = p.as_str() {
                        if let Ok(port) = port_str.split('/').next().unwrap_or("").parse::<u16>() {
                            result.exposed_ports.push(port);
                        }
                    }
                }
            }
        }

        result.misconfigurations = self.check_misconfigurations(&result).await;

        Ok(result)
    }

    pub async fn scan_dockerfile(
        &self,
        dockerfile_path: &str,
    ) -> Result<Vec<DockerMisconfiguration>> {
        let content = std::fs::read_to_string(dockerfile_path)?;
        Ok(self.analyze_dockerfile(&content))
    }

    fn analyze_dockerfile(&self, content: &str) -> Vec<DockerMisconfiguration> {
        let mut issues = Vec::new();

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            if trimmed.starts_with("FROM ")
                && (trimmed.contains("latest") || trimmed.split(':').count() == 1)
            {
                issues.push(DockerMisconfiguration {
                    check: "No specific image tag".to_string(),
                    severity: Severity::Medium,
                    description: "Base image uses 'latest' or no tag".to_string(),
                    recommendation: "Pin base image to a specific version tag".to_string(),
                });
            }

            if trimmed.starts_with("USER root") || trimmed.starts_with("USER 0") {
                issues.push(DockerMisconfiguration {
                    check: "Running as root".to_string(),
                    severity: Severity::High,
                    description: "Container explicitly runs as root user".to_string(),
                    recommendation: "Use a non-root user with minimal privileges".to_string(),
                });
            }

            if trimmed.starts_with("EXPOSE ") {
                let ports: Vec<&str> = trimmed
                    .trim_start_matches("EXPOSE ")
                    .split_whitespace()
                    .collect();
                for port_str in ports {
                    if let Ok(port) = port_str.parse::<u16>() {
                        if port == 22 || port == 23 || port == 3389 {
                            issues.push(DockerMisconfiguration {
                                check: format!("Dangerous port {} exposed", port),
                                severity: Severity::High,
                                description: format!(
                                    "Port {} (SSH/Telnet/RDP) exposed in container",
                                    port
                                ),
                                recommendation: "Avoid exposing management ports in containers"
                                    .to_string(),
                            });
                        }
                    }
                }
            }

            if trimmed.starts_with("ENV ") {
                let env_part = trimmed.trim_start_matches("ENV ");
                if env_part.contains("PASSWORD")
                    || env_part.contains("SECRET")
                    || env_part.contains("API_KEY")
                    || env_part.contains("TOKEN")
                {
                    issues.push(DockerMisconfiguration {
                        check: "Secret in environment variable".to_string(),
                        severity: Severity::Critical,
                        description: "Potential secret stored in environment variable".to_string(),
                        recommendation: "Use Docker secrets or external secret management"
                            .to_string(),
                    });
                }
            }

            if trimmed.starts_with("ADD ") && !trimmed.contains("http") {
                issues.push(DockerMisconfiguration {
                    check: "ADD instead of COPY".to_string(),
                    severity: Severity::Low,
                    description: "ADD used for local files instead of COPY".to_string(),
                    recommendation: "Use COPY for local files, ADD only for URLs/tar extraction"
                        .to_string(),
                });
            }
        }

        if !content.contains("USER ") {
            issues.push(DockerMisconfiguration {
                check: "No USER instruction".to_string(),
                severity: Severity::Medium,
                description: "No USER instruction found - container runs as root by default"
                    .to_string(),
                recommendation: "Add USER instruction to run as non-root".to_string(),
            });
        }

        if !content.contains("HEALTHCHECK ") {
            issues.push(DockerMisconfiguration {
                check: "No HEALTHCHECK".to_string(),
                severity: Severity::Low,
                description: "No HEALTHCHECK instruction found".to_string(),
                recommendation: "Add HEALTHCHECK for container health monitoring".to_string(),
            });
        }

        issues
    }

    fn is_valid_image_name(image_name: &str) -> bool {
        image_name
            .chars()
            .all(|c| c.is_alphanumeric() || c == ':' || c == '@' || c == '-' || c == '_' || c == '.' || c == '/')
    }

    async fn inspect_image(&self, image_name: &str) -> Result<serde_json::Value> {
        if !Self::is_valid_image_name(image_name) {
            return Err(SlapperError::Validation(format!(
                "Invalid image name: contains forbidden characters"
            )));
        }

        let output = Command::new("docker")
            .args(["inspect", image_name])
            .output();

        match output {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let inspections: Vec<serde_json::Value> = serde_json::from_str(&stdout)?;
                if let Some(info) = inspections.into_iter().next() {
                    let mut metadata = serde_json::Map::new();

                    if let Some(config) = info.get("Config") {
                        if let Some(user) = config.get("User") {
                            metadata.insert("user".to_string(), user.clone());
                        }
                        if let Some(_healthcheck) = config.get("Healthcheck") {
                            metadata
                                .insert("healthcheck".to_string(), serde_json::Value::Bool(true));
                        }
                        if let Some(image) = config.get("Image") {
                            metadata.insert("base_image".to_string(), image.clone());
                        }
                    }

                    if let Some(config) = info.get("Config") {
                        if let Some(exposed) = config.get("ExposedPorts") {
                            metadata.insert("exposed_ports".to_string(), exposed.clone());
                        }
                    }

                    Ok(serde_json::Value::Object(metadata))
                } else {
                    Ok(serde_json::Value::Object(serde_json::Map::new()))
                }
            }
            Ok(_) => {
                tracing::debug!("docker inspect returned non-success for {}", image_name);
                Ok(serde_json::Value::Object(serde_json::Map::new()))
            }
            Err(e) => {
                tracing::warn!("Failed to inspect docker image {}: {}", image_name, e);
                Ok(serde_json::Value::Object(serde_json::Map::new()))
            }
        }
    }

    async fn check_misconfigurations(
        &self,
        result: &DockerScanResult,
    ) -> Vec<DockerMisconfiguration> {
        let mut issues = Vec::new();

        if result.running_as_root {
            issues.push(DockerMisconfiguration {
                check: "Running as root".to_string(),
                severity: Severity::High,
                description: "Container runs as root user".to_string(),
                recommendation: "Configure container to run as non-root user".to_string(),
            });
        }

        if !result.has_healthcheck {
            issues.push(DockerMisconfiguration {
                check: "No healthcheck".to_string(),
                severity: Severity::Low,
                description: "Container has no health check configured".to_string(),
                recommendation: "Add HEALTHCHECK instruction to Dockerfile".to_string(),
            });
        }

        for port in &result.exposed_ports {
            if *port == 22 || *port == 23 || *port == 3389 {
                issues.push(DockerMisconfiguration {
                    check: format!("Dangerous port {} exposed", port),
                    severity: Severity::High,
                    description: format!("Management port {} exposed", port),
                    recommendation: "Avoid exposing management ports".to_string(),
                });
            }
        }

        issues
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_docker_scanner_creation() {
        let scanner = DockerScanner::new();
        let _ = scanner;
    }

    #[test]
    fn test_analyze_dockerfile_root_user() {
        let scanner = DockerScanner::new();
        let dockerfile = "FROM ubuntu:latest\nUSER root\nEXPOSE 80\n";
        let issues = scanner.analyze_dockerfile(dockerfile);
        assert!(issues.iter().any(|i| i.check == "Running as root"));
    }

    #[test]
    fn test_analyze_dockerfile_secret_in_env() {
        let scanner = DockerScanner::new();
        let dockerfile = "FROM ubuntu:20.04\nENV API_KEY=secret123\nUSER app\nHEALTHCHECK CMD curl -f http://localhost/ || exit 1\n";
        let issues = scanner.analyze_dockerfile(dockerfile);
        assert!(issues.iter().any(|i| i.severity == Severity::Critical));
    }

    #[test]
    fn test_analyze_dockerfile_no_user() {
        let scanner = DockerScanner::new();
        let dockerfile = "FROM ubuntu:20.04\nEXPOSE 80\n";
        let issues = scanner.analyze_dockerfile(dockerfile);
        assert!(issues.iter().any(|i| i.check == "No USER instruction"));
    }

    #[test]
    fn test_analyze_dockerfile_dangerous_port() {
        let scanner = DockerScanner::new();
        let dockerfile = "FROM ubuntu:20.04\nEXPOSE 22\nUSER app\nHEALTHCHECK CMD true\n";
        let issues = scanner.analyze_dockerfile(dockerfile);
        assert!(issues.iter().any(|i| i.check.contains("22")));
    }
}
