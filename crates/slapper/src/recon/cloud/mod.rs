pub mod iam;
pub mod metadata;
pub mod services;
pub mod storage_test;

use crate::error::Result;
use rustc_hash::FxHashSet;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Semaphore;

use crate::utils::{create_insecure_http_client, extract_target_from_url};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CloudDiscovery {
    pub domain: String,
    pub s3_buckets: Vec<CloudAsset>,
    pub azure_blobs: Vec<CloudAsset>,
    pub gcp_storage: Vec<CloudAsset>,
    pub firebase: Vec<CloudAsset>,
    pub heroku: Vec<CloudAsset>,
    pub github_repos: Vec<CloudAsset>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudAsset {
    pub name: String,
    pub url: String,
    pub exists: bool,
    pub is_public: bool,
    pub bucket_type: String,
}

pub struct CloudScanner {
    client: reqwest::Client,
    concurrency: usize,
}

fn response_indicates_resource_exists(status: u16) -> bool {
    matches!(status, 200 | 201 | 204 | 301 | 302 | 307 | 308 | 403 | 405)
}

impl CloudScanner {
    pub fn new(concurrency: usize) -> Result<Self> {
        let client = create_insecure_http_client(10)?;

        Ok(Self {
            client,
            concurrency,
        })
    }

    pub async fn scan(&self, domain: &str) -> Result<CloudDiscovery> {
        let domain_name = match extract_target_from_url(domain) {
            Some(extracted) => extracted,
            None => {
                tracing::warn!(
                    domain = %domain,
                    "Failed to extract domain from URL, using input as-is"
                );
                domain.to_string()
            }
        };

        let (s3_buckets, azure_blobs, gcp_storage, firebase, heroku, github_repos) = tokio::join!(
            self.enumerate_s3_buckets(&domain_name),
            self.enumerate_azure_blobs(&domain_name),
            self.enumerate_gcp_storage(&domain_name),
            self.enumerate_firebase(&domain_name),
            self.enumerate_heroku(&domain_name),
            self.enumerate_github(&domain_name),
        );

        Ok(CloudDiscovery {
            domain: domain.to_string(),
            s3_buckets,
            azure_blobs,
            gcp_storage,
            firebase,
            heroku,
            github_repos,
        })
    }

    async fn enumerate_s3_buckets(&self, domain: &str) -> Vec<CloudAsset> {
        let bucket_names = self.generate_cloud_names(domain);

        let semaphore = Arc::new(Semaphore::new(self.concurrency));
        let mut handles = Vec::new();

        for name in bucket_names {
            let bucket_url = format!("https://{}.s3.amazonaws.com", name);
            let client = self.client.clone();
            let semaphore = Arc::clone(&semaphore);

            let handle = tokio::spawn(async move {
                let _permit = match semaphore.acquire().await {
                    Ok(p) => p,
                    Err(_) => {
                        tracing::warn!("Semaphore closed while waiting for S3 bucket scan");
                        return None;
                    }
                };

                match tokio::time::timeout(
                    std::time::Duration::from_secs(30),
                    client.get(&bucket_url).send(),
                )
                .await
                {
                    Ok(Ok(response)) => {
                        let status = response.status().as_u16();
                        let is_public = status == 200;

                        Some(CloudAsset {
                            name: name.clone(),
                            url: bucket_url,
                            exists: response_indicates_resource_exists(status),
                            is_public,
                            bucket_type: "S3".to_string(),
                        })
                    }
                    _ => None,
                }
            });

            handles.push(handle);
        }

        let mut results = Vec::new();
        for handle in handles {
            if let Ok(Some(asset)) = handle.await {
                results.push(asset);
            }
        }

        results
    }

    async fn enumerate_azure_blobs(&self, domain: &str) -> Vec<CloudAsset> {
        let storage_names = self.generate_cloud_names(domain);

        let semaphore = Arc::new(Semaphore::new(self.concurrency));
        let mut handles = Vec::new();

        for name in storage_names {
            let blob_url = format!("https://{}.blob.core.windows.net", name);
            let client = self.client.clone();
            let semaphore = Arc::clone(&semaphore);

            let handle = tokio::spawn(async move {
                let _permit = match semaphore.acquire().await {
                    Ok(p) => p,
                    Err(_) => {
                        tracing::warn!("Semaphore closed while waiting for Azure blob scan");
                        return None;
                    }
                };

                match tokio::time::timeout(
                    std::time::Duration::from_secs(30),
                    client.get(&blob_url).send(),
                )
                .await
                {
                    Ok(Ok(response)) => {
                        let status = response.status().as_u16();
                        let is_public = status == 200;

                        Some(CloudAsset {
                            name: name.clone(),
                            url: blob_url,
                            exists: response_indicates_resource_exists(status),
                            is_public,
                            bucket_type: "Azure Blob".to_string(),
                        })
                    }
                    _ => None,
                }
            });

            handles.push(handle);
        }

        let mut results = Vec::new();
        for handle in handles {
            if let Ok(Some(asset)) = handle.await {
                results.push(asset);
            }
        }

        results
    }

    async fn enumerate_gcp_storage(&self, domain: &str) -> Vec<CloudAsset> {
        let bucket_names = self.generate_cloud_names(domain);

        let semaphore = Arc::new(Semaphore::new(self.concurrency));
        let mut handles = Vec::new();

        for name in bucket_names {
            let bucket_url = format!("https://storage.googleapis.com/{}", name);
            let client = self.client.clone();
            let semaphore = Arc::clone(&semaphore);

            let handle = tokio::spawn(async move {
                let _permit = match semaphore.acquire().await {
                    Ok(p) => p,
                    Err(_) => {
                        tracing::warn!("Semaphore closed while waiting for GCP storage scan");
                        return None;
                    }
                };

                match tokio::time::timeout(
                    std::time::Duration::from_secs(30),
                    client.get(&bucket_url).send(),
                )
                .await
                {
                    Ok(Ok(response)) => {
                        let status = response.status().as_u16();
                        let is_public = status == 200;

                        Some(CloudAsset {
                            name: name.clone(),
                            url: bucket_url,
                            exists: response_indicates_resource_exists(status),
                            is_public,
                            bucket_type: "GCP Storage".to_string(),
                        })
                    }
                    _ => None,
                }
            });

            handles.push(handle);
        }

        let mut results = Vec::new();
        for handle in handles {
            if let Ok(Some(asset)) = handle.await {
                results.push(asset);
            }
        }

        results
    }

    async fn enumerate_firebase(&self, domain: &str) -> Vec<CloudAsset> {
        let project_names = self.generate_cloud_names(domain);

        let semaphore = Arc::new(Semaphore::new(self.concurrency));
        let mut handles = Vec::new();

        for name in project_names {
            let firebase_url = format!("https://{}.firebaseio.com", name);
            let client = self.client.clone();
            let semaphore = Arc::clone(&semaphore);

            let handle = tokio::spawn(async move {
                let _permit = match semaphore.acquire().await {
                    Ok(p) => p,
                    Err(_) => {
                        tracing::warn!("Semaphore closed while waiting for Firebase scan");
                        return None;
                    }
                };

                let url = format!("{}/.json", firebase_url);
                match tokio::time::timeout(
                    std::time::Duration::from_secs(30),
                    client.get(&url).send(),
                )
                .await
                {
                    Ok(Ok(response)) => {
                        let status = response.status().as_u16();
                        let is_public = status == 200;

                        Some(CloudAsset {
                            name: name.clone(),
                            url: firebase_url,
                            exists: response_indicates_resource_exists(status),
                            is_public,
                            bucket_type: "Firebase".to_string(),
                        })
                    }
                    _ => None,
                }
            });

            handles.push(handle);
        }

        let mut results = Vec::new();
        for handle in handles {
            if let Ok(Some(asset)) = handle.await {
                results.push(asset);
            }
        }

        results
    }

    async fn enumerate_heroku(&self, domain: &str) -> Vec<CloudAsset> {
        let app_names = self.generate_cloud_names(domain);

        let semaphore = Arc::new(Semaphore::new(self.concurrency));
        let mut handles = Vec::new();

        for name in app_names {
            let heroku_url = format!("https://{}.herokuapp.com", name);
            let client = self.client.clone();
            let semaphore = Arc::clone(&semaphore);

            let handle = tokio::spawn(async move {
                let _permit = match semaphore.acquire().await {
                    Ok(p) => p,
                    Err(_) => {
                        tracing::warn!("Semaphore closed while waiting for Heroku scan");
                        return None;
                    }
                };

                match tokio::time::timeout(
                    std::time::Duration::from_secs(30),
                    client.get(&heroku_url).send(),
                )
                .await
                {
                    Ok(Ok(response)) => {
                        let status = response.status().as_u16();
                        let is_public = status != 404;

                        Some(CloudAsset {
                            name: name.clone(),
                            url: heroku_url,
                            exists: is_public,
                            is_public,
                            bucket_type: "Heroku".to_string(),
                        })
                    }
                    _ => None,
                }
            });

            handles.push(handle);
        }

        let mut results = Vec::new();
        for handle in handles {
            if let Ok(Some(asset)) = handle.await {
                results.push(asset);
            }
        }

        results
    }

    async fn enumerate_github(&self, domain: &str) -> Vec<CloudAsset> {
        let repo_names = self.generate_cloud_names(domain);

        let semaphore = Arc::new(Semaphore::new(self.concurrency));
        let mut handles = Vec::new();

        for name in repo_names {
            let github_url = format!("https://github.com/{}", name);
            let client = self.client.clone();
            let semaphore = Arc::clone(&semaphore);

            let handle = tokio::spawn(async move {
                let _permit = match semaphore.acquire().await {
                    Ok(p) => p,
                    Err(_) => {
                        tracing::warn!("Semaphore closed while waiting for GitHub scan");
                        return None;
                    }
                };

                match tokio::time::timeout(
                    std::time::Duration::from_secs(30),
                    client.get(&github_url).send(),
                )
                .await
                {
                    Ok(Ok(response)) => {
                        let status = response.status().as_u16();
                        let exists = status == 200;

                        Some(CloudAsset {
                            name: name.clone(),
                            url: github_url,
                            exists,
                            is_public: exists,
                            bucket_type: "GitHub".to_string(),
                        })
                    }
                    _ => None,
                }
            });

            handles.push(handle);
        }

        let mut results = Vec::new();
        for handle in handles {
            if let Ok(Some(asset)) = handle.await {
                results.push(asset);
            }
        }

        results
    }

    fn generate_cloud_names(&self, domain: &str) -> Vec<String> {
        let base_name = domain.split('.').next().unwrap_or(domain);
        let mut names = FxHashSet::default();

        names.insert(domain.to_string());
        names.insert(base_name.to_string());
        names.insert(format!("{}-www", base_name));
        names.insert(format!("www-{}", base_name));
        names.insert(format!("{}-prod", base_name));
        names.insert(format!("{}-production", base_name));
        names.insert(format!("{}-staging", base_name));
        names.insert(format!("{}-dev", base_name));
        names.insert(format!("{}-development", base_name));
        names.insert(format!("{}-test", base_name));
        names.insert(format!("{}-backup", base_name));
        names.insert(format!("{}-static", base_name));
        names.insert(format!("{}-assets", base_name));
        names.insert(format!("{}-media", base_name));
        names.insert(format!("{}-files", base_name));
        names.insert(format!("{}-uploads", base_name));
        names.insert(format!("{}-public", base_name));
        names.insert(format!("{}-private", base_name));
        names.insert(format!("{}-app", base_name));
        names.insert(format!("{}-web", base_name));
        names.insert(format!("{}-api", base_name));
        names.insert(format!("{}-cdn", base_name));
        names.insert(format!("{}-storage", base_name));
        names.insert(format!("{}-data", base_name));
        names.insert(format!("{}-logs", base_name));

        names.into_iter().collect()
    }
}

pub async fn scan_cloud(domain: &str, concurrency: usize) -> Result<CloudDiscovery> {
    let scanner = CloudScanner::new(concurrency)?;
    scanner.scan(domain).await
}
