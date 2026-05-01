use crate::error::Result;
use crate::utils::create_insecure_http_client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageTestReport {
    pub bucket_name: String,
    pub provider: String,
    pub tests: Vec<StorageTest>,
    pub vulnerabilities: Vec<StorageVulnerability>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageTest {
    pub test_name: String,
    pub passed: bool,
    pub details: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageVulnerability {
    pub severity: Severity,
    pub title: String,
    pub description: String,
    pub recommendation: String,
}

pub use crate::types::Severity;

pub struct StorageTester {
    client: reqwest::Client,
}

impl StorageTester {
    pub fn new(timeout_secs: u64) -> Result<Self> {
        let client = create_insecure_http_client(timeout_secs)?;
        Ok(Self { client })
    }

    pub async fn test_s3_bucket(&self, bucket_name: &str) -> Result<StorageTestReport> {
        let mut tests = Vec::new();
        let mut vulnerabilities = Vec::new();

        let base_url = format!("https://{}.s3.amazonaws.com", bucket_name);

        let public_read = self.check_s3_public_read(&base_url).await;
        tests.push(StorageTest {
            test_name: "Public Read Access".to_string(),
            passed: !public_read,
            details: if public_read {
                "Bucket allows public read access".to_string()
            } else {
                "Public read access denied".to_string()
            },
        });
        if public_read {
            vulnerabilities.push(StorageVulnerability {
                severity: Severity::High,
                title: "S3 bucket publicly readable".to_string(),
                description: format!("Bucket {} allows unauthenticated read access", bucket_name),
                recommendation: "Remove public read access from bucket policy".to_string(),
            });
        }

        let public_write = self.check_s3_public_write(&base_url).await;
        tests.push(StorageTest {
            test_name: "Public Write Access".to_string(),
            passed: !public_write,
            details: if public_write {
                "Bucket allows public write access".to_string()
            } else {
                "Public write access denied".to_string()
            },
        });
        if public_write {
            vulnerabilities.push(StorageVulnerability {
                severity: Severity::Critical,
                title: "S3 bucket publicly writable".to_string(),
                description: format!("Bucket {} allows unauthenticated write access", bucket_name),
                recommendation: "Remove public write access immediately".to_string(),
            });
        }

        let object_listing = self.check_s3_object_listing(&base_url).await;
        tests.push(StorageTest {
            test_name: "Object Listing".to_string(),
            passed: !object_listing,
            details: if object_listing {
                "Bucket allows object listing".to_string()
            } else {
                "Object listing denied".to_string()
            },
        });
        if object_listing {
            vulnerabilities.push(StorageVulnerability {
                severity: Severity::Medium,
                title: "S3 bucket allows object listing".to_string(),
                description: format!("Bucket {} exposes object listing", bucket_name),
                recommendation: "Disable ListBucket permission for anonymous users".to_string(),
            });
        }

        let cors_config = self.check_s3_cors(&base_url).await;
        tests.push(StorageTest {
            test_name: "CORS Configuration".to_string(),
            passed: cors_config.is_none(),
            details: if let Some(cors) = cors_config {
                format!("CORS allows origins: {}", cors.join(", "))
            } else {
                "No permissive CORS found".to_string()
            },
        });

        Ok(StorageTestReport {
            bucket_name: bucket_name.to_string(),
            provider: "AWS S3".to_string(),
            tests,
            vulnerabilities,
        })
    }

    async fn check_s3_public_read(&self, url: &str) -> bool {
        match self.client.get(url).send().await {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }

    async fn check_s3_public_write(&self, url: &str) -> bool {
        match self.client.put(url).body("test").send().await {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }

    async fn check_s3_object_listing(&self, url: &str) -> bool {
        let list_url = format!("{}/?list-type=2", url);
        match self.client.get(&list_url).send().await {
            Ok(resp) => {
                let status = resp.status().as_u16();
                let body = resp.text().await.unwrap_or_default();
                status == 200 && body.contains("<ListBucketResult>")
            }
            Err(_) => false,
        }
    }

    async fn check_s3_cors(&self, url: &str) -> Option<Vec<String>> {
        let cors_url = format!("{}/?cors", url);
        match self.client.get(&cors_url).send().await {
            Ok(resp) if resp.status().is_success() => {
                let body = resp.text().await.unwrap_or_default();
                let origins: Vec<String> = body
                    .split("<AllowedOrigin>")
                    .skip(1)
                    .filter_map(|s| s.split("</AllowedOrigin>").next())
                    .map(|s| s.trim().to_string())
                    .collect();
                if origins.iter().any(|o| o == "*") {
                    Some(origins)
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_tester_creation() {
        let tester = StorageTester::new(10);
        assert!(tester.is_ok());
    }

    #[test]
    fn test_storage_vulnerability_creation() {
        let vuln = StorageVulnerability {
            severity: Severity::Critical,
            title: "Public write".to_string(),
            description: "Bucket is publicly writable".to_string(),
            recommendation: "Remove public write".to_string(),
        };
        assert_eq!(vuln.severity, Severity::Critical);
    }

    #[test]
    fn test_storage_test_creation() {
        let test = StorageTest {
            test_name: "Public Read".to_string(),
            passed: true,
            details: "Access denied".to_string(),
        };
        assert!(test.passed);
    }
}
