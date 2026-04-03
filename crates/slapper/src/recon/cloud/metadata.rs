use crate::error::Result;
use crate::utils::create_insecure_http_client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataTestResult {
    pub target: String,
    pub imdsv1_accessible: bool,
    pub imdsv2_required: bool,
    pub credentials_exposed: bool,
    pub metadata_accessible: bool,
    pub findings: Vec<MetadataFinding>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataFinding {
    pub severity: Severity,
    pub title: String,
    pub description: String,
    pub recommendation: String,
}

pub use crate::types::Severity;

const METADATA_ENDPOINTS: &[&str] = &[
    "http://169.254.169.254/latest/meta-data/",
    "http://169.254.169.254/latest/meta-data/iam/security-credentials/",
    "http://169.254.169.254/latest/meta-data/iam/info",
    "http://169.254.169.254/latest/user-data",
    "http://169.254.169.254/latest/dynamic/instance-identity/document",
];

pub struct MetadataTester {
    client: reqwest::Client,
}

impl MetadataTester {
    pub fn new(timeout_secs: u64) -> Result<Self> {
        let client = create_insecure_http_client(timeout_secs)?;
        Ok(Self { client })
    }

    pub async fn test_metadata_access(&self, target: &str) -> Result<MetadataTestResult> {
        let mut result = MetadataTestResult {
            target: target.to_string(),
            imdsv1_accessible: false,
            imdsv2_required: false,
            credentials_exposed: false,
            metadata_accessible: false,
            findings: Vec::new(),
        };

        for endpoint in METADATA_ENDPOINTS {
            match self.client.get(*endpoint).timeout(std::time::Duration::from_secs(3)).send().await {
                Ok(resp) if resp.status().is_success() => {
                    result.metadata_accessible = true;
                    result.imdsv1_accessible = true;

                    if endpoint.contains("security-credentials") {
                        result.credentials_exposed = true;
                        result.findings.push(MetadataFinding {
                            severity: Severity::Critical,
                            title: "IAM credentials exposed via metadata".to_string(),
                            description: format!("IAM security credentials accessible at {}", endpoint),
                            recommendation: "Use IMDSv2 and restrict metadata access".to_string(),
                        });
                    }
                    if endpoint.contains("iam/info") {
                        result.findings.push(MetadataFinding {
                            severity: Severity::High,
                            title: "IAM role info exposed".to_string(),
                            description: "IAM role information accessible via metadata".to_string(),
                            recommendation: "Restrict metadata endpoint access".to_string(),
                        });
                    }
                }
                _ => {}
            }
        }

        let token_response = self.client
            .put("http://169.254.169.254/latest/api/token")
            .header("X-aws-ec2-metadata-token-ttl-seconds", "21600")
            .timeout(std::time::Duration::from_secs(3))
            .send()
            .await;

        result.imdsv2_required = token_response.is_ok();

        if result.imdsv1_accessible && !result.imdsv2_required {
            result.findings.push(MetadataFinding {
                severity: Severity::High,
                title: "IMDSv1 accessible without IMDSv2 enforcement".to_string(),
                description: "Instance Metadata Service v1 is accessible and not requiring v2 tokens".to_string(),
                recommendation: "Enable IMDSv2 enforcement (hop limit = 1)".to_string(),
            });
        }

        if !result.metadata_accessible {
            result.findings.push(MetadataFinding {
                severity: Severity::Info,
                title: "Metadata endpoint not accessible".to_string(),
                description: "Could not reach cloud metadata endpoints from current network".to_string(),
                recommendation: "Test from within the cloud environment".to_string(),
            });
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata_tester_creation() {
        let tester = MetadataTester::new(10);
        assert!(tester.is_ok());
    }

    #[test]
    fn test_metadata_endpoints_not_empty() {
        assert!(!METADATA_ENDPOINTS.is_empty());
        assert!(METADATA_ENDPOINTS.iter().any(|e| e.contains("security-credentials")));
    }

    #[test]
    fn test_metadata_result_default() {
        let result = MetadataTestResult {
            target: "http://example.com".to_string(),
            imdsv1_accessible: false,
            imdsv2_required: false,
            credentials_exposed: false,
            metadata_accessible: false,
            findings: Vec::new(),
        };
        assert!(!result.imdsv1_accessible);
        assert!(!result.credentials_exposed);
    }
}
