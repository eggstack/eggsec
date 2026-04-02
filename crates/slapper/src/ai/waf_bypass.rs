use std::path::PathBuf;
use serde::{Serialize, Deserialize};
use crate::ai::client::AiClient;

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
            knowledge_base,
            persist_path,
        }
    }

    pub async fn find_bypass(
        &mut self,
        waf: &str,
        blocked_payload: &str,
    ) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>> {
        for entry in &self.knowledge_base {
            if entry.waf_name == waf && entry.original_payload == blocked_payload && entry.success {
                return Ok(Some(entry.bypass_payload.clone()));
            }
        }

        let suggestions = self.client.suggest_waf_bypass(waf, blocked_payload).await?;
        Ok(suggestions.first().cloned())
    }

    pub async fn iterative_bypass(
        &mut self,
        waf: &str,
        mut payload: String,
        max_iterations: usize,
    ) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>> {
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
        if let Ok(json) = serde_json::to_string_pretty(&self.knowledge_base) {
            let _ = std::fs::write(&self.persist_path, json);
        }
    }
}
