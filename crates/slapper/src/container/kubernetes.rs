use crate::container::Severity;
use crate::error::Result;
use crate::utils::create_insecure_http_client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KubernetesScanResult {
    pub cluster_info: Option<ClusterInfo>,
    pub rbac_issues: Vec<K8sFinding>,
    pub network_policy_issues: Vec<K8sFinding>,
    pub pod_security_issues: Vec<K8sFinding>,
    pub secret_exposure: Vec<K8sFinding>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterInfo {
    pub server_version: Option<String>,
    pub node_count: Option<usize>,
    pub namespace_count: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct K8sFinding {
    pub resource_type: String,
    pub resource_name: String,
    pub severity: Severity,
    pub description: String,
    pub recommendation: String,
}

pub struct KubernetesScanner {
    client: reqwest::Client,
    api_server: String,
    token: Option<String>,
}

impl KubernetesScanner {
    pub fn new(api_server: &str, token: Option<String>, timeout_secs: u64) -> Result<Self> {
        let client = create_insecure_http_client(timeout_secs)?;
        Ok(Self {
            client,
            api_server: api_server.trim_end_matches('/').to_string(),
            token,
        })
    }

    pub fn from_in_cluster_config(timeout_secs: u64) -> Result<Self> {
        let token =
            std::fs::read_to_string("/var/run/secrets/kubernetes.io/serviceaccount/token").ok();
        let api_server = std::env::var("KUBERNETES_SERVICE_HOST")
            .map(|h| format!("https://{}", h))
            .unwrap_or_else(|_| "https://kubernetes.default.svc".to_string());
        Self::new(&api_server, token, timeout_secs)
    }

    pub async fn scan(&self) -> Result<KubernetesScanResult> {
        let mut result = KubernetesScanResult {
            cluster_info: None,
            rbac_issues: Vec::new(),
            network_policy_issues: Vec::new(),
            pod_security_issues: Vec::new(),
            secret_exposure: Vec::new(),
        };

        result.cluster_info = self.get_cluster_info().await.ok();
        result.rbac_issues = self.check_rbac().await;
        result.network_policy_issues = self.check_network_policies().await;
        result.pod_security_issues = self.check_pod_security().await;
        result.secret_exposure = self.check_secret_exposure().await;

        Ok(result)
    }

    async fn get_cluster_info(&self) -> Result<ClusterInfo> {
        let version_url = format!("{}/version", self.api_server);
        let mut req = self.client.get(&version_url);
        if let Some(ref token) = self.token {
            req = req.bearer_auth(token);
        }
        let resp = req.send().await?;
        let version_json: serde_json::Value = resp.json().await?;

        Ok(ClusterInfo {
            server_version: version_json
                .get("gitVersion")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            node_count: None,
            namespace_count: None,
        })
    }

    async fn check_rbac(&self) -> Vec<K8sFinding> {
        let mut findings = Vec::new();

        let cluster_role_url = format!(
            "{}/apis/rbac.authorization.k8s.io/v1/clusterroles",
            self.api_server
        );
        let mut req = self.client.get(&cluster_role_url);
        if let Some(ref token) = self.token {
            req = req.bearer_auth(token);
        }
        if let Ok(resp) = req.send().await {
            if resp.status().is_success() {
                if let Ok(json) = resp.json::<serde_json::Value>().await {
                    if let Some(items) = json
                        .get("items")
                        .and_then(|v: &serde_json::Value| v.as_array())
                    {
                        for item in items {
                            if let Some(name) = item
                                .get("metadata")
                                .and_then(|m: &serde_json::Value| m.get("name"))
                                .and_then(|n: &serde_json::Value| n.as_str())
                            {
                                if name == "cluster-admin" {
                                    if let Some(rules) = item
                                        .get("rules")
                                        .and_then(|r: &serde_json::Value| r.as_array())
                                    {
                                        for rule in rules {
                                            if rule
                                                .get("resources")
                                                .and_then(|r: &serde_json::Value| r.as_array())
                                                .map(|a: &Vec<serde_json::Value>| {
                                                    a.iter().any(|v| v.as_str() == Some("*"))
                                                })
                                                .unwrap_or(false)
                                            {
                                                findings.push(K8sFinding {
                                                    resource_type: "ClusterRole".to_string(),
                                                    resource_name: name.to_string(),
                                                    severity: Severity::Critical,
                                                    description: "Cluster role with wildcard resource access".to_string(),
                                                    recommendation: "Restrict resource access to specific resources".to_string(),
                                                });
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        findings
    }

    async fn check_network_policies(&self) -> Vec<K8sFinding> {
        let mut findings = Vec::new();

        let np_url = format!(
            "{}/apis/networking.k8s.io/v1/networkpolicies",
            self.api_server
        );
        let mut req = self.client.get(&np_url);
        if let Some(ref token) = self.token {
            req = req.bearer_auth(token);
        }
        if let Ok(resp) = req.send().await {
            if resp.status().is_success() {
                if let Ok(json) = resp.json::<serde_json::Value>().await {
                    let count = json
                        .get("items")
                        .and_then(|v: &serde_json::Value| v.as_array())
                        .map(|a: &Vec<serde_json::Value>| a.len())
                        .unwrap_or(0);
                    if count == 0 {
                        findings.push(K8sFinding {
                            resource_type: "NetworkPolicy".to_string(),
                            resource_name: "default".to_string(),
                            severity: Severity::High,
                            description: "No network policies defined".to_string(),
                            recommendation: "Define default deny network policies".to_string(),
                        });
                    }
                }
            }
        }

        findings
    }

    async fn check_pod_security(&self) -> Vec<K8sFinding> {
        let mut findings = Vec::new();

        let pods_url = format!("{}/api/v1/pods", self.api_server);
        let mut req = self.client.get(&pods_url);
        if let Some(ref token) = self.token {
            req = req.bearer_auth(token);
        }
        if let Ok(resp) = req.send().await {
            if resp.status().is_success() {
                if let Ok(json) = resp.json::<serde_json::Value>().await {
                    if let Some(items) = json
                        .get("items")
                        .and_then(|v: &serde_json::Value| v.as_array())
                    {
                        for item in items {
                            if let Some(name) = item
                                .get("metadata")
                                .and_then(|m: &serde_json::Value| m.get("name"))
                                .and_then(|n: &serde_json::Value| n.as_str())
                            {
                                if let Some(spec) = item.get("spec") {
                                    if let Some(containers) = spec
                                        .get("containers")
                                        .and_then(|c: &serde_json::Value| c.as_array())
                                    {
                                        for container in containers {
                                            if let Some(sec) = container.get("securityContext") {
                                                if sec
                                                    .get("privileged")
                                                    .and_then(|v: &serde_json::Value| v.as_bool())
                                                    .unwrap_or(false)
                                                {
                                                    findings.push(K8sFinding {
                                                        resource_type: "Pod".to_string(),
                                                        resource_name: name.to_string(),
                                                        severity: Severity::Critical,
                                                        description:
                                                            "Container running in privileged mode"
                                                                .to_string(),
                                                        recommendation:
                                                            "Disable privileged mode for containers"
                                                                .to_string(),
                                                    });
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        findings
    }

    async fn check_secret_exposure(&self) -> Vec<K8sFinding> {
        let mut findings = Vec::new();

        let secrets_url = format!("{}/api/v1/secrets", self.api_server);
        let mut req = self.client.get(&secrets_url);
        if let Some(ref token) = self.token {
            req = req.bearer_auth(token);
        }
        if let Ok(resp) = req.send().await {
            if resp.status().is_success() {
                if let Ok(json) = resp.json::<serde_json::Value>().await {
                    if let Some(items) = json
                        .get("items")
                        .and_then(|v: &serde_json::Value| v.as_array())
                    {
                        for item in items {
                            if let Some(name) = item
                                .get("metadata")
                                .and_then(|m: &serde_json::Value| m.get("name"))
                                .and_then(|n: &serde_json::Value| n.as_str())
                            {
                                if let Some(secret_type) = item
                                    .get("type")
                                    .and_then(|t: &serde_json::Value| t.as_str())
                                {
                                    if secret_type == "Opaque" {
                                        findings.push(K8sFinding {
                                            resource_type: "Secret".to_string(),
                                            resource_name: name.to_string(),
                                            severity: Severity::Medium,
                                            description:
                                                "Opaque secret found - verify encryption at rest"
                                                    .to_string(),
                                            recommendation: "Enable encryption at rest for secrets"
                                                .to_string(),
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        findings
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_k8s_scanner_creation() {
        let scanner = KubernetesScanner::new("https://k8s.example.com", None, 10);
        assert!(scanner.is_ok());
    }

    #[test]
    fn test_k8s_finding_creation() {
        let finding = K8sFinding {
            resource_type: "Pod".to_string(),
            resource_name: "test-pod".to_string(),
            severity: Severity::Critical,
            description: "Test".to_string(),
            recommendation: "Fix it".to_string(),
        };
        assert_eq!(finding.severity, Severity::Critical);
    }

    #[test]
    fn test_cluster_info_creation() {
        let info = ClusterInfo {
            server_version: Some("v1.28.0".to_string()),
            node_count: Some(3),
            namespace_count: Some(10),
        };
        assert_eq!(info.server_version, Some("v1.28.0".to_string()));
    }
}
