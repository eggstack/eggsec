use crate::error::Result;
use crate::utils::create_insecure_http_client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudServiceDiscovery {
    pub domain: String,
    pub services: Vec<DiscoveredService>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredService {
    pub service_type: String,
    pub identifier: String,
    pub exists: bool,
    pub details: String,
}

const SERVICE_ENDPOINTS: &[(&str, &str, &str)] = &[
    ("AWS Lambda", "https://{}.lambda.amazonaws.com", "lambda"),
    ("AWS API Gateway", "https://{}.execute-api.amazonaws.com", "api-gateway"),
    ("AWS CloudFront", "https://{}.cloudfront.net", "cloudfront"),
    ("Azure Functions", "https://{}.azurewebsites.net", "azure-functions"),
    ("GCP Cloud Functions", "https://{}.cloudfunctions.net", "gcp-functions"),
];

pub struct CloudServiceEnumerator {
    client: reqwest::Client,
}

impl CloudServiceEnumerator {
    pub fn new(timeout_secs: u64) -> Result<Self> {
        let client = create_insecure_http_client(timeout_secs)?;
        Ok(Self { client })
    }

    pub async fn enumerate(&self, domain: &str) -> Result<CloudServiceDiscovery> {
        let mut services = Vec::new();
        let base_name = domain.split('.').next().unwrap_or(domain);

        let name_variants = vec![
            domain.to_string(),
            base_name.to_string(),
            format!("{}-api", base_name),
            format!("{}-app", base_name),
            format!("{}-prod", base_name),
        ];

        for (service_type, endpoint_template, _service_id) in SERVICE_ENDPOINTS {
            for name in &name_variants {
                let url = endpoint_template.replace("{}", name);
                let exists = self.check_endpoint(&url).await;
                if exists {
                    services.push(DiscoveredService {
                        service_type: service_type.to_string(),
                        identifier: name.clone(),
                        exists: true,
                        details: format!("Found at {}", url),
                    });
                }
            }
        }

        Ok(CloudServiceDiscovery {
            domain: domain.to_string(),
            services,
        })
    }

    async fn check_endpoint(&self, url: &str) -> bool {
        match self.client.get(url).send().await {
            Ok(resp) => resp.status().as_u16() != 404,
            Err(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_enumerator_creation() {
        let enumerator = CloudServiceEnumerator::new(10);
        assert!(enumerator.is_ok());
    }

    #[test]
    fn test_service_endpoints_not_empty() {
        assert!(!SERVICE_ENDPOINTS.is_empty());
    }
}
