use std::collections::HashMap;
use crate::ai::client::AiClient;

pub struct AiPayloadGenerator {
    client: AiClient,
    cache: HashMap<String, Vec<String>>,
}

impl AiPayloadGenerator {
    pub fn new(client: AiClient) -> Self {
        Self {
            client,
            cache: HashMap::new(),
        }
    }

    pub async fn generate_payloads(
        &mut self,
        vuln_type: &str,
        context: &str,
    ) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
        let cache_key = format!("{}:{}", vuln_type, context);
        if let Some(cached) = self.cache.get(&cache_key) {
            return Ok(cached.clone());
        }

        let payloads = self.client.suggest_payloads(vuln_type, context).await?;
        let payloads: Vec<String> = payloads.into_iter().take(50).collect();
        self.cache.insert(cache_key, payloads.clone());
        Ok(payloads)
    }
}
