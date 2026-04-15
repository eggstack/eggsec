use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use serde::{Serialize, Deserialize};
use crate::ai::cache::AiCache;
use crate::ai::client::AiClient;
use crate::ai::errors::{AiError, Result};
use crate::ai::cache::CacheKeyBuilder;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WafBypassEntry {
    pub waf_name: String,
    pub original_payload: String,
    pub bypass_payload: String,
    pub technique: String,
    pub success: bool,
}

pub struct SmartWafBypass {
    client: AiClient,
    cache: Arc<AiCache>,
    knowledge_base: Vec<WafBypassEntry>,
    persist_path: PathBuf,
}

impl SmartWafBypass {
    pub fn new(client: AiClient) -> Self {
        let persist_path = directories::ProjectDirs::from("com", "slapper", "slapper")
            .map(|d| d.config_dir().join("waf_bypasses.json"))
            .unwrap_or_else(|| PathBuf::from("waf_bypasses.json"));

        let knowledge_base = if persist_path.exists() {
            std::fs::read_to_string(&persist_path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        Self {
            client,
            cache: Arc::new(AiCache::new(50, Duration::from_secs(1800))),
            knowledge_base,
            persist_path,
        }
    }

    pub async fn find_bypass(
        &mut self,
        waf: &str,
        blocked_payload: &str,
    ) -> Result<Option<String>> {
        if waf.is_empty() {
            return Err(AiError::invalid_config("waf name cannot be empty"));
        }
        if blocked_payload.is_empty() {
            return Err(AiError::invalid_config("blocked_payload cannot be empty"));
        }

        for entry in &self.knowledge_base {
            if entry.waf_name == waf && entry.original_payload == blocked_payload && entry.success {
                return Ok(Some(entry.bypass_payload.clone()));
            }
        }

        let cache_key = CacheKeyBuilder::for_waf_bypass(waf, blocked_payload);
        if let Some(cached) = self.cache.get(&cache_key).await {
            return Ok(Some(cached));
        }

        let suggestions = self.client.suggest_waf_bypass(waf, blocked_payload).await?;
        if let Some(bypass) = suggestions.first().cloned() {
            self.cache.set(&cache_key, &bypass, Some(Duration::from_secs(1800))).await;
            return Ok(Some(bypass));
        }

        Ok(None)
    }

    pub async fn iterative_bypass(
        &mut self,
        waf: &str,
        mut payload: String,
        max_iterations: usize,
    ) -> Result<Option<String>> {
        if waf.is_empty() {
            return Err(AiError::invalid_config("waf name cannot be empty"));
        }

        for _ in 0..max_iterations.min(10) {
            let suggestions = self.client.suggest_waf_bypass(waf, &payload).await?;
            if let Some(new_payload) = suggestions.first() {
                payload = new_payload.clone();
            } else {
                break;
            }
        }
        Ok(Some(payload))
    }

    pub fn record_success(&mut self, waf: &str, original: &str, bypass: &str, technique: &str) {
        self.knowledge_base.push(WafBypassEntry {
            waf_name: waf.to_string(),
            original_payload: original.to_string(),
            bypass_payload: bypass.to_string(),
            technique: technique.to_string(),
            success: true,
        });
        self.persist();
    }

    fn persist(&self) {
        if let Some(parent) = self.persist_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string(&self.knowledge_base) {
            let _ = std::fs::write(&self.persist_path, json);
        }
    }
}

impl Clone for SmartWafBypass {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            cache: Arc::clone(&self.cache),
            knowledge_base: self.knowledge_base.clone(),
            persist_path: self.persist_path.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::client::AiClient;
    use crate::config::AiConfig;

    fn create_mock_client() -> AiClient {
        AiClient::new(AiConfig {
            provider: "openai".to_string(),
            model: Some("gpt-4".to_string()),
            api_key: None,
            base_url: None,
            max_tokens: Some(2048),
            temperature: Some(0.7),
        })
    }

    fn create_test_bypass() -> SmartWafBypass {
        SmartWafBypass::new(create_mock_client())
    }

    #[tokio::test]
    async fn test_find_bypass_empty_waf() {
        let mut bypass = create_test_bypass();
        let result = bypass.find_bypass("", "payload").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_find_bypass_empty_payload() {
        let mut bypass = create_test_bypass();
        let result = bypass.find_bypass("cloudflare", "").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_iterative_bypass_empty_waf() {
        let mut bypass = create_test_bypass();
        let result = bypass.iterative_bypass("", "payload".to_string(), 5).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_record_success_adds_to_knowledge_base() {
        let mut bypass = create_test_bypass();
        let initial_len = bypass.knowledge_base.len();
        bypass.record_success("cloudflare", "payload1", "bypassed1", "technique1");
        assert_eq!(bypass.knowledge_base.len(), initial_len + 1);
        let entry = &bypass.knowledge_base[initial_len];
        assert_eq!(entry.waf_name, "cloudflare");
        assert_eq!(entry.original_payload, "payload1");
        assert_eq!(entry.bypass_payload, "bypassed1");
        assert_eq!(entry.technique, "technique1");
        assert!(entry.success);
    }

    #[test]
    fn test_waf_bypass_entry_serialization() {
        let entry = WafBypassEntry {
            waf_name: "cloudflare".to_string(),
            original_payload: "payload".to_string(),
            bypass_payload: "bypassed".to_string(),
            technique: "encoding".to_string(),
            success: true,
        };
        let json = serde_json::to_string(&entry).unwrap();
        let deserialized: WafBypassEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.waf_name, entry.waf_name);
        assert_eq!(deserialized.original_payload, entry.original_payload);
        assert_eq!(deserialized.bypass_payload, entry.bypass_payload);
        assert_eq!(deserialized.technique, entry.technique);
        assert_eq!(deserialized.success, entry.success);
    }

    #[test]
    fn test_clone_preserves_knowledge_base() {
        let mut bypass = create_test_bypass();
        bypass.record_success("cloudflare", "p1", "b1", "t1");
        let bypass_clone = bypass.clone();
        assert_eq!(bypass_clone.knowledge_base.len(), bypass.knowledge_base.len());
        assert_eq!(bypass_clone.knowledge_base[0].waf_name, "cloudflare");
    }
}
