use std::sync::Arc;
use std::time::Duration;
use crate::ai::cache::AiCache;
use crate::ai::client::AiClient;
use crate::ai::errors::{AiError, Result};

pub struct AiPayloadGenerator {
    client: AiClient,
    cache: Arc<AiCache>,
}

impl AiPayloadGenerator {
    pub fn new(client: AiClient) -> Self {
        Self {
            client,
            cache: Arc::new(AiCache::new(100, Duration::from_secs(3600))),
        }
    }

    pub async fn generate_payloads(
        &self,
        vuln_type: &str,
        context: &str,
    ) -> Result<Vec<String>> {
        if vuln_type.is_empty() {
            return Err(AiError::invalid_config("vuln_type cannot be empty"));
        }

        let cache_key = format!("{}:{}", vuln_type, context);

        if let Some(cached) = self.cache.get(&cache_key).await {
            return Ok(cached.split('\n').map(String::from).collect());
        }

        let payloads = self.client.suggest_payloads(vuln_type, context).await?;
        let payload_str = payloads.join("\n");
        self.cache.set(&cache_key, &payload_str, Some(Duration::from_secs(3600))).await;

        Ok(payloads)
    }

    pub async fn clear_cache(&self) {
        self.cache.clear().await;
    }

    pub async fn cache_size(&self) -> usize {
        self.cache.len().await
    }
}

impl Clone for AiPayloadGenerator {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            cache: Arc::clone(&self.cache),
        }
    }
}

impl AiClient {
    pub fn into_payload_generator(self) -> AiPayloadGenerator {
        AiPayloadGenerator::new(self)
    }
}
