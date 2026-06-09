//! Container security scanning
//!
//! Provides security scanning for Docker containers and Kubernetes clusters.
//! This module is feature-gated and requires the `container` feature.

use crate::container::ContainerFinding;
use crate::container::Severity;
use crate::error::{Result, EggsecError};
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

#[cfg(feature = "container")]
use kube::{
    api::{Api, ListParams},
    Client,
};

#[cfg(feature = "container")]
use k8s_openapi::api::core::v1::Pod;

pub use crate::container::ContainerScanReport as ContainerScanResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KubernetesScanResult {
    pub cluster: String,
    pub namespace: Option<String>,
    pub pods: Vec<PodSecurityResult>,
    pub overall_severity: Severity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PodSecurityResult {
    pub name: String,
    pub namespace: String,
    pub security_context_issues: Vec<ContainerFinding>,
}

pub struct ContainerScanner {
    #[cfg(feature = "container")]
    kube_client: Option<Client>,
}

impl ContainerScanner {
    pub fn new() -> Result<Self> {
        #[cfg(not(feature = "container"))]
        {
            Ok(Self {})
        }

        #[cfg(feature = "container")]
        {
            Ok(Self { kube_client: None })
        }
    }

    #[cfg(feature = "container")]
    pub async fn scan_kubernetes(&self, namespace: Option<&str>) -> Result<KubernetesScanResult> {
        let client = self
            .kube_client
            .as_ref()
            .ok_or_else(|| EggsecError::Config("Kubernetes client not available".to_string()))?;

        let api: Api<Pod> = if let Some(ns) = namespace {
            Api::namespaced(client.clone(), ns)
        } else {
            Api::all(client.clone())
        };

        let lp = ListParams::default();
        let pods = api
            .list(&lp)
            .await
            .map_err(|e| EggsecError::Config(format!("Failed to list pods: {}", e)))?;

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
                security_context_issues: security_issues,
            });
        }

        Ok(KubernetesScanResult {
            cluster: "default".to_string(),
            namespace: namespace.map(String::from),
            pods: pod_results,
            overall_severity,
        })
    }

    #[cfg(feature = "container")]
    fn check_pod_security(&self, pod: &Pod) -> Vec<ContainerFinding> {
        let mut issues = Vec::new();

        if let Some(spec) = &pod.spec {
            for container in &spec.containers {
                if container.security_context.is_none() {
                    issues.push(ContainerFinding {
                        category: "PSD001".to_string(),
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

                if container.liveness_probe.is_none() {
                    issues.push(ContainerFinding {
                        category: "PSD002".to_string(),
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

            if spec.host_pid == Some(true) {
                issues.push(ContainerFinding {
                    category: "PSD003".to_string(),
                    severity: Severity::High,
                    title: "Host PID namespace enabled".to_string(),
                    description: "Pod shares the host PID namespace".to_string(),
                    recommendation: "Disable hostPID unless strictly necessary".to_string(),
                });
            }

            if spec.host_network == Some(true) {
                issues.push(ContainerFinding {
                    category: "PSD004".to_string(),
                    severity: Severity::High,
                    title: "Host network enabled".to_string(),
                    description: "Pod shares the host network namespace".to_string(),
                    recommendation: "Disable hostNetwork unless strictly necessary".to_string(),
                });
            }

            for container in &spec.containers {
                if let Some(sc) = &container.security_context {
                    if sc.privileged == Some(true) {
                        issues.push(ContainerFinding {
                            category: "PSD005".to_string(),
                            severity: Severity::Critical,
                            title: "Privileged container".to_string(),
                            description: format!(
                                "Container {} runs in privileged mode",
                                container.name
                            ),
                            recommendation: "Remove privileged mode unless strictly necessary"
                                .to_string(),
                        });
                    }
                }
            }
        }

        issues
    }

    #[cfg(not(feature = "container"))]
    pub async fn scan_kubernetes(&self, _namespace: Option<&str>) -> Result<KubernetesScanResult> {
        Err(EggsecError::Config(
            "Kubernetes support requires the `container` feature".to_string(),
        ))
    }

    #[cfg(feature = "container")]
    pub async fn scan_docker_image(&self, image: &str) -> Result<ContainerScanResult> {
        let scanner = crate::container::docker::DockerScanner::new();
        let docker_result = scanner.scan_image(image).await?;

        let mut findings = Vec::new();
        for misconfig in &docker_result.misconfigurations {
            findings.push(crate::container::ContainerFinding {
                category: "Docker Misconfiguration".to_string(),
                severity: misconfig.severity.clone(),
                title: misconfig.check.clone(),
                description: misconfig.description.clone(),
                recommendation: misconfig.recommendation.clone(),
            });
        }

        Ok(ContainerScanResult {
            target: image.to_string(),
            scan_type: crate::container::ContainerScanType::Docker,
            docker: Some(docker_result),
            kubernetes: None,
            escape_risks: None,
            cis_benchmarks: None,
            findings,
        })
    }

    #[cfg(not(feature = "container"))]
    pub fn scan_docker_image(&self, _image: &str) -> Result<ContainerScanResult> {
        Err(EggsecError::Config(
            "Docker image scanning requires the `container` feature".to_string(),
        ))
    }

    pub fn check_container_config(
        &self,
        config: &FxHashMap<String, String>,
    ) -> Vec<ContainerFinding> {
        let mut issues = Vec::new();

        if let Some(user) = config.get("USER") {
            if user.is_empty() || *user == "root" {
                issues.push(ContainerFinding {
                    category: "DC001".to_string(),
                    severity: Severity::High,
                    title: "Container running as root".to_string(),
                    description: "Container is configured to run as root user".to_string(),
                    recommendation: "Use a non-root user in the container".to_string(),
                });
            }
        }

        if config.get("CAP_ADD").is_some() {
            issues.push(ContainerFinding {
                category: "DC002".to_string(),
                severity: Severity::Medium,
                title: "Capabilities added".to_string(),
                description: "Container has additional Linux capabilities".to_string(),
                recommendation: "Review and drop unnecessary capabilities".to_string(),
            });
        }

        if config.get("privileged") == Some(&"true".to_string()) {
            issues.push(ContainerFinding {
                category: "DC003".to_string(),
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
        Self::new().unwrap_or(Self {
            #[cfg(feature = "container")]
            kube_client: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_container_scanner_creation() {
        let scanner = ContainerScanner::new();
        assert!(scanner.is_ok());
    }

    #[test]
    fn test_config_issue_detection() {
        let scanner = ContainerScanner::default();
        let mut config = FxHashMap::default();
        config.insert("USER".to_string(), "root".to_string());

        let issues = scanner.check_container_config(&config);
        assert!(!issues.is_empty());
        assert_eq!(issues[0].category, "DC001");
    }
}
