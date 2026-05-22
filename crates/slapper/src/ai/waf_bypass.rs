use crate::ai::cache::AiCache;
use crate::ai::cache::CacheKeyBuilder;
use crate::ai::client::AiClient;
use crate::ai::errors::{AiError, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WafBypassEntry {
    pub waf_name: String,
    pub original_payload: String,
    pub bypass_payload: String,
    pub technique: String,
    pub success: bool,
    #[serde(default)]
    pub failed_attempts: usize,
}

pub struct SmartWafBypass {
    client: AiClient,
    cache: Arc<AiCache>,
    knowledge_base: Vec<WafBypassEntry>,
    persist_path: PathBuf,
    max_bypasses: usize,
    max_knowledge_base_size: usize,
}

impl SmartWafBypass {
    pub fn new(client: AiClient) -> Self {
        Self::with_config(client, 10)
    }

    pub fn with_config(client: AiClient, max_bypasses: usize) -> Self {
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
            max_bypasses,
            max_knowledge_base_size: 1000,
        }
    }

    #[allow(dead_code)]
    fn with_knowledge_base(client: AiClient, knowledge_base: Vec<WafBypassEntry>) -> Self {
        Self {
            client,
            cache: Arc::new(AiCache::new(50, Duration::from_secs(1800))),
            knowledge_base,
            persist_path: PathBuf::from("waf_bypasses.json"),
            max_bypasses: 10,
            max_knowledge_base_size: 1000,
        }
    }

    fn evict_knowledge_base_if_needed(&mut self) {
        if self.knowledge_base.len() >= self.max_knowledge_base_size {
            self.knowledge_base.retain(|e| e.success);
            if self.knowledge_base.len() >= self.max_knowledge_base_size {
                self.knowledge_base.sort_by_key(|e| e.failed_attempts);
                self.knowledge_base.truncate(self.max_knowledge_base_size / 2);
            }
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
            if entry.waf_name == waf && entry.original_payload == blocked_payload {
                if entry.success {
                    return Ok(Some(entry.bypass_payload.clone()));
                }
                if entry.failed_attempts >= 3 {
                    tracing::debug!(
                        "Skipping WAF bypass query for {}/{} - previously failed {} attempts",
                        waf,
                        blocked_payload,
                        entry.failed_attempts
                    );
                    return Ok(None);
                }
                continue;
            }
        }

        let cache_key = CacheKeyBuilder::for_waf_bypass(waf, blocked_payload);
        if let Some(cached) = self.cache.get(&cache_key).await {
            return Ok(Some(cached));
        }

        let suggestions = self.client.suggest_waf_bypass(waf, blocked_payload).await?;
        if let Some(bypass) = suggestions.first().cloned() {
            self.cache
                .set(&cache_key, &bypass, Some(Duration::from_secs(1800)))
                .await;
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

        let original_payload = payload.clone();
        let mut changed = false;
        for _ in 0..max_iterations.min(self.max_bypasses) {
            let suggestions = self.client.suggest_waf_bypass(waf, &payload).await?;
            if let Some(new_payload) = suggestions.first() {
                if *new_payload == payload {
                    break;
                }
                payload = new_payload.clone();
                changed = true;
            } else {
                break;
            }
        }
        if changed && payload != original_payload {
            Ok(Some(payload))
        } else {
            Ok(None)
        }
    }

    pub fn record_success(&mut self, waf: &str, original: &str, bypass: &str, technique: &str) {
        if let Some(entry) = self
            .knowledge_base
            .iter_mut()
            .find(|e| e.waf_name == waf && e.original_payload == original)
        {
            entry.bypass_payload = bypass.to_string();
            entry.technique = technique.to_string();
            entry.success = true;
            entry.failed_attempts = 0;
        } else {
            self.evict_knowledge_base_if_needed();
            self.knowledge_base.push(WafBypassEntry {
                waf_name: waf.to_string(),
                original_payload: original.to_string(),
                bypass_payload: bypass.to_string(),
                technique: technique.to_string(),
                success: true,
                failed_attempts: 0,
            });
        }
        self.persist();
    }

    pub fn record_failure(&mut self, waf: &str, original: &str) {
        if let Some(entry) = self
            .knowledge_base
            .iter_mut()
            .find(|e| e.waf_name == waf && e.original_payload == original)
        {
            entry.failed_attempts += 1;
            entry.success = false;
        } else {
            self.evict_knowledge_base_if_needed();
            self.knowledge_base.push(WafBypassEntry {
                waf_name: waf.to_string(),
                original_payload: original.to_string(),
                bypass_payload: String::new(),
                technique: String::new(),
                success: false,
                failed_attempts: 1,
            });
        }
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
            max_bypasses: self.max_bypasses,
            max_knowledge_base_size: self.max_knowledge_base_size,
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
            max_payloads: 50,
            max_bypasses: 10,
        })
        .expect("test AI client should be valid")
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
        let mut bypass = SmartWafBypass::with_knowledge_base(create_mock_client(), Vec::new());
        bypass.record_success("cloudflare", "payload1", "bypassed1", "technique1");
        assert_eq!(bypass.knowledge_base.len(), 1);
        let entry = &bypass.knowledge_base[0];
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
            failed_attempts: 0,
        };
        let json = serde_json::to_string(&entry).unwrap();
        let deserialized: WafBypassEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.waf_name, entry.waf_name);
        assert_eq!(deserialized.original_payload, entry.original_payload);
        assert_eq!(deserialized.bypass_payload, entry.bypass_payload);
        assert_eq!(deserialized.technique, entry.technique);
        assert_eq!(deserialized.success, entry.success);
        assert_eq!(deserialized.failed_attempts, 0);
    }

    #[test]
    fn test_clone_preserves_knowledge_base() {
        let mut bypass = create_test_bypass();
        bypass.record_success("cloudflare", "p1", "b1", "t1");
        let bypass_clone = bypass.clone();
        assert_eq!(
            bypass_clone.knowledge_base.len(),
            bypass.knowledge_base.len()
        );
        assert_eq!(bypass_clone.knowledge_base[0].waf_name, "cloudflare");
    }
}
