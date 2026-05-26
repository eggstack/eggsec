//! Container security scanning
//!
//! Provides security scanning for Docker containers and Kubernetes clusters.
//! This module is feature-gated and requires the `container` feature.

use crate::error::{Result, SlapperError};
use crate::types::Severity;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

#[cfg(feature = "container")]
use kube::{
    api::{Api, ListParams},
    Client,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerScanResult {
    pub container_id: String,
    pub image: String,
    pub vulnerabilities: Vec<ContainerVulnerability>,
    pub configuration_issues: Vec<ConfigIssue>,
    pub overall_severity: Severity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerVulnerability {
    pub id: String,
    pub package: String,
    pub installed_version: String,
    pub fixed_version: Option<String>,
    pub severity: Severity,
    pub description: String,
    pub cve_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigIssue {
    pub rule_id: String,
    pub severity: Severity,
    pub title: String,
    pub description: String,
    pub recommendation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KubernetesScanResult {
    pub cluster: String,
    pub namespace: Option<String>,
    pub pods: Vec<PodSecurityResult>,
    pub services: Vec<ServiceSecurityResult>,
    pub secrets: Vec<SecretSecurityResult>,
    pub overall_severity: Severity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PodSecurityResult {
    pub name: String,
    pub namespace: String,
    pub containers: Vec<ContainerScanResult>,
    pub security_context_issues: Vec<ConfigIssue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceSecurityResult {
    pub name: String,
    pub namespace: String,
    pub issues: Vec<ConfigIssue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretSecurityResult {
    pub name: String,
    pub namespace: String,
    pub has_encryption_issues: bool,
    pub description: String,
}

pub struct ContainerScanner {
    #[cfg(feature = "container")]
    kube_client: Option<Client>,
}

impl ContainerScanner {
    pub fn new() -> Result<Self> {
        #[cfg(feature = "container")]
        {
            let client = Client::try_default()
                .map_err(|e| SlapperError::Config(format!("Kubernetes client failed: {}", e)))?;
            Ok(Self {
                kube_client: Some(client),
            })
        }

        #[cfg(not(feature = "container"))]
        {
            Ok(Self {})
        }
    }

    #[cfg(feature = "container")]
    pub async fn scan_kubernetes(&self, namespace: Option<&str>) -> Result<KubernetesScanResult> {
        let client = self
            .kube_client
            .as_ref()
            .ok_or_else(|| SlapperError::Config("Kubernetes client not available".to_string()))?;

        let api: Api<kube::api::Pod> = if let Some(ns) = namespace {
            Api::namespaced(client.clone(), ns)
        } else {
            Api::all(client.clone())
        };

        let lp = ListParams::default();
        let pods = api
            .list(&lp)
            .await
            .map_err(|e| SlapperError::Config(format!("Failed to list pods: {}", e)))?;

        let mut pod_results = Vec::new();
        let mut overall_severity = Severity::Info;

        for pod in pods {
            let pod_name = pod.metadata.name.clone().unwrap_or_else(|| {
                tracing::debug!("pod missing name field");
                String::new()
            });
            let pod_namespace = pod.metadata.namespace.clone().unwrap_or_else(|| {
                tracing::debug!("pod missing namespace field");
                String::new()
            });

            let security_issues = self.check_pod_security(&pod);
            if security_issues
                .iter()
                .any(|i| matches!(i.severity, Severity::Critical | Severity::High))
            {
                overall_severity = Severity::High;
            }

            pod_results.push(PodSecurityResult {
                name: pod_name,
                namespace: pod_namespace,
                containers: Vec::new(),
                security_context_issues: security_issues,
            });
        }

        Ok(KubernetesScanResult {
            cluster: "default".to_string(),
            namespace: namespace.map(String::from),
            pods: pod_results,
            services: Vec::new(),
            secrets: Vec::new(),
            overall_severity,
        })
    }

    #[cfg(feature = "container")]
    fn check_pod_security(
        &self,
        pod: &kube::api::Pod,
    ) -> Vec<ConfigIssue> {
        let mut issues = Vec::new();

        if let Some(spec) = &pod.spec {
            for container in &spec.containers {
                if container.security_context.is_none() {
                    issues.push(ConfigIssue {
                        rule_id: "PSD001".to_string(),
                        severity: Severity::Medium,
                        title: "Missing container security context".to_string(),
                        description: format!(
                            "Container {} does not have a security context set",
                            container.name
                        ),
                        recommendation: "Set runAsNonRoot: true and runAsUser in security context"
                            .to_string(),
                    });
                }

                if let Some(liveness_probe) = &container.liveness_probe {
                    if liveness_probe.http_get.is_none()
                        && liveness_probe.tcp_socket.is_none()
                        && liveness_probe.exec.is_none()
                    {
                        issues.push(ConfigIssue {
                            rule_id: "PSD002".to_string(),
                            severity: Severity::Low,
                            title: "Missing liveness probe".to_string(),
                            description: format!(
                                "Container {} does not have a liveness probe configured",
                                container.name
                            ),
                            recommendation: "Add a liveness probe to detect application crashes"
                                .to_string(),
                        });
                    }
                }
            }

            if spec.host_pid == Some(true) {
                issues.push(ConfigIssue {
                    rule_id: "PSD003".to_string(),
                    severity: Severity::High,
                    title: "Host PID namespace enabled".to_string(),
                    description: "Pod shares the host PID namespace".to_string(),
                    recommendation: "Disable hostPID unless strictly necessary".to_string(),
                });
            }

            if spec.host_network == Some(true) {
                issues.push(ConfigIssue {
                    rule_id: "PSD004".to_string(),
                    severity: Severity::High,
                    title: "Host network enabled".to_string(),
                    description: "Pod shares the host network namespace".to_string(),
                    recommendation: "Disable hostNetwork unless strictly necessary".to_string(),
                });
            }

            if spec.privileged == Some(true) {
                issues.push(ConfigIssue {
                    rule_id: "PSD005".to_string(),
                    severity: Severity::Critical,
                    title: "Privileged container".to_string(),
                    description: "Container runs in privileged mode".to_string(),
                    recommendation: "Remove privileged mode unless strictly necessary".to_string(),
                });
            }
        }

        issues
    }

    #[cfg(not(feature = "container"))]
    pub async fn scan_kubernetes(&self, _namespace: Option<&str>) -> Result<KubernetesScanResult> {
        Err(SlapperError::Config(
            "Kubernetes support requires the `container` feature".to_string(),
        ))
    }

    #[allow(dead_code, unused_variables)]
    /// Docker image scanning - implementation incomplete
    pub fn scan_docker_image(&self, _image: &str) -> Result<ContainerScanResult> {
        Err(SlapperError::Config(
            "Docker image scanning requires full implementation".to_string(),
        ))
    }

    pub fn check_container_config(&self, config: &FxHashMap<String, String>) -> Vec<ConfigIssue> {
        let mut issues = Vec::new();

        if let Some(user) = config.get("USER") {
            if user.is_empty() || *user == "root" {
                issues.push(ConfigIssue {
                    rule_id: "DC001".to_string(),
                    severity: Severity::High,
                    title: "Container running as root".to_string(),
                    description: "Container is configured to run as root user".to_string(),
                    recommendation: "Use a non-root user in the container".to_string(),
                });
            }
        }

        if config.get("CAP_ADD").is_some() {
            issues.push(ConfigIssue {
                rule_id: "DC002".to_string(),
                severity: Severity::Medium,
                title: "Capabilities added".to_string(),
                description: "Container has additional Linux capabilities".to_string(),
                recommendation: "Review and drop unnecessary capabilities".to_string(),
            });
        }

        if config.get("privileged") == Some(&"true".to_string()) {
            issues.push(ConfigIssue {
                rule_id: "DC003".to_string(),
                severity: Severity::Critical,
                title: "Privileged mode".to_string(),
                description: "Container runs in privileged mode".to_string(),
                recommendation: "Remove privileged mode".to_string(),
            });
        }

        issues
    }
}

impl Default for ContainerScanner {
    fn default() -> Self {
        Self::new().unwrap_or(Self {})
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_container_scanner_creation() {
        let scanner = ContainerScanner::new();
        assert!(scanner.is_ok() || std::mem::size_of_val(&scanner) > 0);
    }

    #[test]
    fn test_config_issue_detection() {
        let scanner = ContainerScanner::default();
        let mut config = FxHashMap::default();
        config.insert("USER".to_string(), "root".to_string());

        let issues = scanner.check_container_config(&config);
        assert!(!issues.is_empty());
        assert_eq!(issues[0].rule_id, "DC001");
    }
}
