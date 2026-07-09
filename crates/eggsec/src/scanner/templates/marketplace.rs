//! Template marketplace integration
//!
//! Provides integration with template marketplaces for downloading
//! and managing community-contributed vulnerability templates.

use crate::error::{EggsecError, Result};
use crate::scanner::templates::verify::{SignedTemplate, TemplateVerifier};
use crate::scanner::templates::{TemplateLoader, VulnerabilityTemplate};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceTemplate {
    pub id: String,
    pub name: String,
    pub author: String,
    pub description: String,
    pub severity: String,
    pub tags: Vec<String>,
    pub downloads: u64,
    pub rating: f32,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceListing {
    pub templates: Vec<MarketplaceTemplate>,
    pub total_count: usize,
    pub page: usize,
    pub per_page: usize,
}

pub struct TemplateMarketplace {
    base_url: String,
    http_client: reqwest::Client,
    local_cache: PathBuf,
    verifier: Option<TemplateVerifier>,
    verify_downloaded: bool,
}

pub fn validate_template_id(template_id: &str) -> Result<()> {
    if template_id.is_empty() {
        return Err(EggsecError::Validation(
            "Template ID cannot be empty".to_string(),
        ));
    }
    if template_id.contains('/')
        || template_id.contains('\\')
        || template_id.contains("..")
        || template_id.contains('\0')
    {
        return Err(EggsecError::Validation(format!(
            "Invalid template ID: {}",
            template_id
        )));
    }
    Ok(())
}

impl TemplateMarketplace {
    pub fn new(base_url: &str) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| EggsecError::Config(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            base_url: base_url.to_string(),
            http_client: client,
            local_cache: directories::ProjectDirs::from("com", "eggsec", "eggsec")
                .map(|d| d.cache_dir().join("template_marketplace"))
                .unwrap_or_else(|| PathBuf::from(".template_cache")),
            verifier: None,
            verify_downloaded: true,
        })
    }

    pub fn with_verifier(mut self, verifier: TemplateVerifier) -> Self {
        self.verifier = Some(verifier);
        self
    }

    pub fn with_verify_downloaded(mut self, verify: bool) -> Self {
        self.verify_downloaded = verify;
        self
    }

    pub fn with_cache_dir(mut self, cache_dir: PathBuf) -> Self {
        self.local_cache = cache_dir;
        self
    }

    pub async fn list_templates(
        &self,
        page: usize,
        per_page: usize,
        tag_filter: Option<&str>,
    ) -> Result<MarketplaceListing> {
        let mut url = format!(
            "{}/api/v1/templates?page={}&per_page={}",
            self.base_url, page, per_page
        );

        if let Some(tag) = tag_filter {
            url.push_str(&format!("&tag={}", urlencoding::encode(tag)));
        }

        let response = self
            .http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| EggsecError::Network(format!("Failed to fetch templates: {}", e)))?;

        if !response.status().is_success() {
            return Err(EggsecError::Network(format!(
                "Marketplace returned status: {}",
                response.status()
            )));
        }

        let listing: MarketplaceListing = response.json().await.map_err(|e| {
            EggsecError::Network(format!("Failed to parse marketplace response: {}", e))
        })?;

        Ok(listing)
    }

    pub async fn download_template(&self, template_id: &str) -> Result<VulnerabilityTemplate> {
        let url = format!(
            "{}/api/v1/templates/{}/download",
            self.base_url, template_id
        );

        let response = self
            .http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| EggsecError::Network(format!("Failed to download template: {}", e)))?;

        if !response.status().is_success() {
            return Err(EggsecError::Network(format!(
                "Download failed with status: {}",
                response.status()
            )));
        }

        let content = response
            .text()
            .await
            .map_err(|e| EggsecError::Network(format!("Failed to read template content: {}", e)))?;

        let loader = TemplateLoader::default();
        let mut final_template = loader.parse_template(&content)?;

        if self.verify_downloaded {
            if let Some(verifier) = &self.verifier {
                let signed: SignedTemplate = serde_yaml_neo::from_str(&content)
                    .or_else(|_| serde_json::from_str(&content))
                    .map_err(|_| {
                        EggsecError::Validation(format!(
                            "Template {} is not a signed template envelope",
                            template_id
                        ))
                    })?;

                let verification = verifier.verify(&signed)?;
                if !verification.valid {
                    return Err(EggsecError::Validation(format!(
                        "Template {} has invalid signature: {}",
                        template_id,
                        verification
                            .error
                            .unwrap_or_else(|| "unknown verification error".to_string())
                    )));
                }
                final_template = signed.template;
            } else {
                tracing::warn!(
                    "Template {} downloaded but no verifier configured - signature not verified",
                    template_id
                );
            }
        }

        self.save_to_cache(template_id, &content)?;

        Ok(final_template)
    }

    fn save_to_cache(&self, template_id: &str, content: &str) -> Result<()> {
        validate_template_id(template_id)?;

        std::fs::create_dir_all(&self.local_cache)
            .map_err(|e| EggsecError::Config(format!("Failed to create cache directory: {}", e)))?;

        let cache_path = self.local_cache.join(format!("{}.yaml", template_id));

        std::fs::write(&cache_path, content)
            .map_err(|e| EggsecError::Config(format!("Failed to write cache file: {}", e)))?;

        Ok(())
    }

    pub fn get_cached_template(&self, template_id: &str) -> Result<Option<VulnerabilityTemplate>> {
        validate_template_id(template_id)?;

        let cache_path = self.local_cache.join(format!("{}.yaml", template_id));

        if !cache_path.exists() {
            return Ok(None);
        }

        let content = std::fs::read_to_string(&cache_path)
            .map_err(|e| EggsecError::Config(format!("Failed to read cache file: {}", e)))?;

        let loader = TemplateLoader::default();
        let template = loader.parse_template(&content)?;

        Ok(Some(template))
    }

    pub fn list_cached_templates(&self) -> Result<Vec<PathBuf>> {
        if !self.local_cache.exists() {
            return Ok(Vec::new());
        }

        let entries = std::fs::read_dir(&self.local_cache)
            .map_err(|e| EggsecError::Config(format!("Failed to read cache directory: {}", e)))?;

        let templates: Vec<PathBuf> = entries
            .filter_map(|e| match e {
                Ok(entry) => Some(entry),
                Err(e) => {
                    tracing::debug!("Skipping unreadable cache entry: {:?}", e);
                    None
                }
            })
            .filter(|e| {
                e.path()
                    .extension()
                    .map(|ext| ext == "yaml" || ext == "yml")
                    .unwrap_or(false)
            })
            .map(|e| e.path())
            .collect();

        Ok(templates)
    }

    pub fn clear_cache(&self) -> Result<()> {
        if self.local_cache.exists() {
            std::fs::remove_dir_all(&self.local_cache)
                .map_err(|e| EggsecError::Config(format!("Failed to clear cache: {}", e)))?;
        }
        Ok(())
    }

    pub async fn sync_templates(&self, template_dir: &Path) -> Result<Vec<String>> {
        let listing = self.list_templates(1, 100, None).await?;
        let mut synced = Vec::new();

        for marketplace_template in listing.templates {
            if let Ok(Some(_)) = self.get_cached_template(&marketplace_template.id) {
                continue;
            }

            match self.download_template(&marketplace_template.id).await {
                Ok(template) => {
                    if let Err(e) = validate_template_id(&template.id) {
                        tracing::warn!(
                            "Refusing to write template with invalid ID '{}': {}",
                            template.id,
                            e
                        );
                        continue;
                    }
                    let path = template_dir.join(format!("{}.yaml", template.id));
                    let yaml = serde_yaml_neo::to_string(&template).map_err(|e| {
                        EggsecError::Config(format!("Failed to serialize template: {}", e))
                    })?;
                    tokio::fs::write(&path, yaml).await.map_err(|e| {
                        EggsecError::Config(format!("Failed to write template: {}", e))
                    })?;
                    synced.push(marketplace_template.id.clone());
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to download template {}: {}",
                        marketplace_template.id,
                        e
                    );
                }
            }
        }

        Ok(synced)
    }
}

impl Default for TemplateMarketplace {
    fn default() -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            base_url: "https://templates.eggsec.io".to_string(),
            http_client: client,
            local_cache: directories::ProjectDirs::from("com", "eggsec", "eggsec")
                .map(|d| d.cache_dir().join("template_marketplace"))
                .unwrap_or_else(|| PathBuf::from(".template_cache")),
            verifier: None,
            verify_downloaded: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_marketplace_creation() {
        let marketplace = TemplateMarketplace::new("https://templates.example.com");
        assert!(marketplace.is_ok());
    }

    #[test]
    fn test_default_marketplace() {
        let marketplace = TemplateMarketplace::default();
        assert_eq!(marketplace.base_url, "https://templates.eggsec.io");
    }

    #[test]
    fn test_validate_template_id_rejects_traversal() {
        assert!(validate_template_id("..").is_err());
        assert!(validate_template_id("../etc/passwd").is_err());
        assert!(validate_template_id("foo/bar").is_err());
        assert!(validate_template_id("foo\\bar").is_err());
        assert!(validate_template_id("foo\0bar").is_err());
        assert!(validate_template_id("").is_err());
    }

    #[test]
    fn test_validate_template_id_accepts_valid() {
        assert!(validate_template_id("cve-2024-1234").is_ok());
        assert!(validate_template_id("simple_id").is_ok());
    }

    #[test]
    fn test_get_cached_template_rejects_traversal_id() {
        let dir = std::env::temp_dir().join("eggsec_test_marketplace_get");
        let _ = std::fs::create_dir_all(&dir);
        let mp = TemplateMarketplace::new("https://templates.example.com")
            .unwrap()
            .with_cache_dir(dir);
        assert!(mp.get_cached_template("../escape").is_err());
        assert!(mp.get_cached_template("ok/../escape").is_err());
    }

    #[test]
    fn test_validate_template_id_rejects_save_traversal() {
        let dir = std::env::temp_dir().join("eggsec_test_marketplace_save");
        let _ = std::fs::create_dir_all(&dir);
        let _mp = TemplateMarketplace::new("https://templates.example.com")
            .unwrap()
            .with_cache_dir(dir);
        assert!(validate_template_id("../escape").is_err());
    }
}
