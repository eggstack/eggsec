use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use crate::ai::client::AiClient;
use crate::ai::errors::{AiError, Result};

pub struct AiPayloadGenerator {
    client: AiClient,
    cache: Arc<RwLock<HashMap<String, Vec<String>>>>,
}

impl AiPayloadGenerator {
    pub fn new(client: AiClient) -> Self {
        Self {
            client,
            cache: Arc::new(RwLock::new(HashMap::new())),
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

        {
            let cache = self.cache.read();
            if let Some(cached) = cache.get(&cache_key) {
                return Ok(cached.clone());
            }
        }

        let payloads = self.client.suggest_payloads(vuln_type, context).await?;
        let payloads: Vec<String> = payloads.into_iter().take(50).collect();

        {
            let mut cache = self.cache.write();
            cache.insert(cache_key, payloads.clone());
        }

        Ok(payloads)
    }

    pub fn clear_cache(&self) {
        let mut cache = self.cache.write();
        cache.clear();
    }

    pub fn cache_size(&self) -> usize {
        let cache = self.cache.read();
        cache.len()
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
