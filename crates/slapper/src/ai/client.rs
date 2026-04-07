use reqwest::Client;
use crate::config::AiConfig;
use crate::ai::errors::{AiError, Result};
use crate::utils::circuit_breaker::{CircuitBreaker, CircuitState};
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone)]
pub struct AiClient {
    client: Client,
    config: AiConfig,
    circuit_breaker: Arc<CircuitBreaker>,
}

impl AiClient {
    pub fn new(config: AiConfig) -> Self {
        let circuit_breaker = Arc::new(CircuitBreaker::new(
            5,
            3,
            Duration::from_secs(60),
        ));
        Self {
            client: Client::new(),
            config,
            circuit_breaker,
        }
    }

    pub fn api_url(&self) -> &str {
        self.config
            .base_url
            .as_deref()
            .unwrap_or("https://api.openai.com/v1/chat/completions")
    }

    pub fn model(&self) -> &str {
        self.config.model.as_deref().unwrap_or("gpt-4")
    }

    pub fn apply_auth(&self, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        if let Some(key) = &self.config.api_key {
            request.bearer_auth(key.expose_secret().to_string())
        } else {
            request
        }
    }

    async fn chat_completion(&self, prompt: &str, max_tokens: Option<u32>, temperature: f64) -> Result<serde_json::Value> {
        if !self.circuit_breaker.is_available().await {
            return Err(AiError::CircuitBreakerOpen {});
        }

        let body = serde_json::json!({
            "model": self.model(),
            "messages": [{"role": "user", "content": prompt}],
            "max_tokens": max_tokens.unwrap_or(2048),
            "temperature": temperature,
        });

        let request = self.apply_auth(self.client.post(self.api_url()).json(&body));

        match request.send().await {
            Ok(response) => {
                if response.status().as_u16() == 429 {
                    self.circuit_breaker.record_failure().await;
                    return Err(AiError::RateLimited);
                }
                if response.status().is_server_error() {
                    self.circuit_breaker.record_failure().await;
                    return Err(AiError::ApiError(format!("Server error: {}", response.status())));
                }
                self.circuit_breaker.record_success().await;
                let result: serde_json::Value = response.json().await?;
                if let Some(error) = result.get("error") {
                    let message = error.get("message").and_then(|m| m.as_str()).unwrap_or("Unknown error");
                    return Err(AiError::ApiError(message.to_string()));
                }
                Ok(result)
            }
            Err(e) => {
                self.circuit_breaker.record_failure().await;
                Err(AiError::RequestFailed(e.to_string()))
            }
        }
    }

    fn extract_content(&self, result: &serde_json::Value, filter_fn: fn(&str) -> bool) -> Vec<String> {
        if let Some(choices) = result.get("choices") {
            if let Some(choice) = choices.get(0) {
                if let Some(content) = choice
                    .get("message")
                    .and_then(|m| m.get("content"))
                    .and_then(|c| c.as_str())
                {
                    return content
                        .lines()
                        .filter(|l| !l.is_empty() && filter_fn(l))
                        .map(String::from)
                        .collect();
                }
            }
        }
        vec![]
    }

    pub async fn analyze_findings(
        &self,
        findings: &[serde_json::Value],
    ) -> Result<serde_json::Value> {
        let prompt = format!(
            "Analyze these security findings:\n{}",
            serde_json::to_string_pretty(findings).map_err(|e| AiError::ParseError(e.to_string()))?
        );

        self.chat_completion(&prompt, self.config.max_tokens.map(|v| v as u32), self.config.temperature.unwrap_or(0.7)).await
    }

    pub async fn suggest_payloads(
        &self,
        vuln_type: &str,
        context: &str,
    ) -> Result<Vec<String>> {
        if vuln_type.is_empty() {
            return Err(AiError::invalid_config("vuln_type cannot be empty"));
        }

        let prompt = format!(
            "Generate security testing payloads for {} vulnerability. Context: {}",
            vuln_type, context
        );

        let result = self.chat_completion(&prompt, Some(2048), 0.8).await?;
        let payloads = self.extract_content(&result, |l| !l.starts_with('#'));
        Ok(payloads.into_iter().take(50).collect())
    }

    pub async fn suggest_waf_bypass(
        &self,
        waf: &str,
        blocked_payload: &str,
    ) -> Result<Vec<String>> {
        if waf.is_empty() {
            return Err(AiError::invalid_config("waf name cannot be empty"));
        }
        if blocked_payload.is_empty() {
            return Err(AiError::invalid_config("blocked_payload cannot be empty"));
        }

        let prompt = format!(
            "Suggest WAF bypass techniques for {} WAF. This payload was blocked: {}",
            waf, blocked_payload
        );

        let result = self.chat_completion(&prompt, Some(2048), 0.9).await?;
        let bypasses = self.extract_content(&result, |l| !l.starts_with('#'));
        Ok(bypasses.into_iter().take(10).collect())
    }

    pub async fn circuit_breaker_state(&self) -> CircuitState {
        self.circuit_breaker.get_state().await
    }
}
