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

    pub fn apply_auth(&self, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        if let Some(key) = &self.config.api_key {
            request.bearer_auth(key.expose_secret().to_string())
        } else {
            request
        }
    }

    pub async fn analyze_findings(
        &self,
        findings: &[serde_json::Value],
    ) -> Result<serde_json::Value> {
        if !self.circuit_breaker.is_available().await {
            return Err(AiError::CircuitBreakerOpen {});
        }

        let api_url = self
            .config
            .base_url
            .as_deref()
            .unwrap_or("https://api.openai.com/v1/chat/completions");
        let model = self.config.model.as_deref().unwrap_or("gpt-4");

        let prompt = format!(
            "Analyze these security findings:\n{}",
            serde_json::to_string_pretty(findings).map_err(|e| AiError::ParseError(e.to_string()))?
        );

        let body = serde_json::json!({
            "model": model,
            "messages": [{"role": "user", "content": prompt}],
            "max_tokens": self.config.max_tokens.unwrap_or(4096),
            "temperature": self.config.temperature.unwrap_or(0.7),
        });

        let request = self.apply_auth(self.client.post(api_url).json(&body));

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

    pub async fn suggest_payloads(
        &self,
        vuln_type: &str,
        context: &str,
    ) -> Result<Vec<String>> {
        if vuln_type.is_empty() {
            return Err(AiError::invalid_config("vuln_type cannot be empty"));
        }

        if !self.circuit_breaker.is_available().await {
            return Err(AiError::CircuitBreakerOpen {});
        }

        let api_url = self
            .config
            .base_url
            .as_deref()
            .unwrap_or("https://api.openai.com/v1/chat/completions");
        let model = self.config.model.as_deref().unwrap_or("gpt-4");

        let prompt = format!(
            "Generate security testing payloads for {} vulnerability. Context: {}",
            vuln_type, context
        );

        let body = serde_json::json!({
            "model": model,
            "messages": [{"role": "user", "content": prompt}],
            "max_tokens": self.config.max_tokens.unwrap_or(2048),
            "temperature": 0.8,
        });

        let request = self.apply_auth(self.client.post(api_url).json(&body));

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
                if let Some(choices) = result.get("choices") {
                    if let Some(choice) = choices.get(0) {
                        if let Some(content) = choice
                            .get("message")
                            .and_then(|m| m.get("content"))
                            .and_then(|c| c.as_str())
                        {
                            return Ok(content
                                .lines()
                                .filter(|l| !l.is_empty() && !l.starts_with('#'))
                                .map(String::from)
                                .take(50)
                                .collect());
                        }
                    }
                }
                Ok(vec![])
            }
            Err(e) => {
                self.circuit_breaker.record_failure().await;
                Err(AiError::RequestFailed(e.to_string()))
            }
        }
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

        if !self.circuit_breaker.is_available().await {
            return Err(AiError::CircuitBreakerOpen {});
        }

        let api_url = self
            .config
            .base_url
            .as_deref()
            .unwrap_or("https://api.openai.com/v1/chat/completions");
        let model = self.config.model.as_deref().unwrap_or("gpt-4");

        let prompt = format!(
            "Suggest WAF bypass techniques for {} WAF. This payload was blocked: {}",
            waf, blocked_payload
        );

        let body = serde_json::json!({
            "model": model,
            "messages": [{"role": "user", "content": prompt}],
            "max_tokens": self.config.max_tokens.unwrap_or(2048),
            "temperature": 0.9,
        });

        let request = self.apply_auth(self.client.post(api_url).json(&body));

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
                if let Some(choices) = result.get("choices") {
                    if let Some(choice) = choices.get(0) {
                        if let Some(content) = choice
                            .get("message")
                            .and_then(|m| m.get("content"))
                            .and_then(|c| c.as_str())
                        {
                            return Ok(content
                                .lines()
                                .filter(|l| !l.is_empty() && !l.starts_with('#'))
                                .map(String::from)
                                .take(10)
                                .collect());
                        }
                    }
                }
                Ok(vec![])
            }
            Err(e) => {
                self.circuit_breaker.record_failure().await;
                Err(AiError::RequestFailed(e.to_string()))
            }
        }
    }

    pub async fn circuit_breaker_state(&self) -> CircuitState {
        self.circuit_breaker.get_state().await
    }
}
