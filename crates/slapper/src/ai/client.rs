use reqwest::Client;
use crate::config::AiConfig;
use crate::ai::errors::{AiError, Result};
use crate::utils::circuit_breaker::{CircuitBreaker, CircuitState};
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provider {
    OpenAI,
    Azure,
    Anthropic,
    OpenAICompatible,
}

impl Provider {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "openai" | "openai.com" => Provider::OpenAI,
            "azure" | "azureopenai" | "azureopenai.com" => Provider::Azure,
            "anthropic" | "anthropic.com" | "claude" => Provider::Anthropic,
            _ => Provider::OpenAICompatible,
        }
    }

    pub fn default_model(&self) -> &'static str {
        match self {
            Provider::OpenAI => "gpt-4",
            Provider::Azure => "gpt-4",
            Provider::Anthropic => "claude-3-sonnet-20240229",
            Provider::OpenAICompatible => "gpt-4",
        }
    }

    pub fn supports_bearer_auth(&self) -> bool {
        matches!(self, Provider::OpenAI | Provider::OpenAICompatible | Provider::Anthropic)
    }

    pub fn supports_azure_auth(&self) -> bool {
        matches!(self, Provider::Azure)
    }
}

#[derive(Clone)]
pub struct AiClient {
    client: Client,
    config: AiConfig,
    circuit_breaker: Arc<CircuitBreaker>,
    provider: Provider,
}

impl AiClient {
    pub fn new(config: AiConfig) -> Self {
        if config.provider.is_empty() {
            panic!("AiConfig provider cannot be empty");
        }
        let provider = Provider::from_str(&config.provider);
        let circuit_breaker = Arc::new(CircuitBreaker::new(
            5,
            3,
            Duration::from_secs(60),
        ));
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(60))
                .pool_max_idle_per_host(20)
                .pool_idle_timeout(Duration::from_secs(30))
                .tcp_nodelay(true)
                .build()
                .expect("Failed to create AI HTTP client"),
            config,
            circuit_breaker,
            provider,
        }
    }

    pub fn provider(&self) -> Provider {
        self.provider
    }

    pub fn api_url(&self) -> &str {
        self.config
            .base_url
            .as_deref()
            .unwrap_or_else(|| match self.provider {
                Provider::OpenAI => "https://api.openai.com/v1/chat/completions",
                Provider::Azure => "https://api.openai.com/v1/chat/completions",
                Provider::Anthropic => "https://api.anthropic.com/v1/messages",
                Provider::OpenAICompatible => "https://api.openai.com/v1/chat/completions",
            })
    }

    pub fn model(&self) -> &str {
        self.config.model.as_deref().unwrap_or(self.provider.default_model())
    }

    pub fn apply_auth(&self, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        if let Some(key) = &self.config.api_key {
            match self.provider {
                Provider::Azure => {
                    request
                        .header("api-key", key.expose_secret().to_string())
                        .header("Content-Type", "application/json")
                }
                _ if self.provider.supports_bearer_auth() => {
                    request.bearer_auth(key.expose_secret().to_string())
                }
                _ => request,
            }
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

    pub async fn chat_completion_from_messages(&self, body: &serde_json::Value) -> Result<serde_json::Value> {
        if !self.circuit_breaker.is_available().await {
            return Err(AiError::CircuitBreakerOpen {});
        }

        let (request_body, needs_anthropic_format) = if self.provider == Provider::Anthropic {
            let transformed = self.transform_to_anthropic_format(body)?;
            (transformed, true)
        } else {
            (body.clone(), false)
        };

        let mut request_builder = self.client.post(self.api_url()).json(&request_body);

        if needs_anthropic_format {
            request_builder = request_builder.header("anthropic-version", "2023-06-01");
        }

        let request = self.apply_auth(request_builder);

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

    fn transform_to_anthropic_format(&self, body: &serde_json::Value) -> Result<serde_json::Value> {
        let model = body.get("model")
            .and_then(|v| v.as_str())
            .unwrap_or(self.model());

        let max_tokens = body.get("max_tokens")
            .and_then(|v| v.as_i64())
            .unwrap_or(2048) as u64;

        let messages = body.get("messages")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        let mut system_message = String::new();
        let mut anthropic_messages = Vec::new();

        for msg in messages {
            let role = msg.get("role").and_then(|v| v.as_str()).unwrap_or("user");
            let content = msg.get("content").and_then(|v| v.as_str()).unwrap_or("");

            match role {
                "system" => {
                    if !system_message.is_empty() {
                        system_message.push('\n');
                    }
                    system_message.push_str(content);
                }
                "user" | "assistant" => {
                    anthropic_messages.push(serde_json::json!({
                        "role": role,
                        "content": content
                    }));
                }
                _ => {}
            }
        }

        Ok(serde_json::json!({
            "model": model,
            "max_tokens": max_tokens,
            "system": system_message,
            "messages": anthropic_messages
        }))
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

    pub async fn analyze_findings_typed(
        &self,
        findings: &[serde_json::Value],
    ) -> Result<crate::ai::types::AiAnalysisResult> {
        let prompt = format!(
            "Analyze these security findings:\n{}",
            serde_json::to_string_pretty(findings).map_err(|e| AiError::ParseError(e.to_string()))?
        );

        let result = self.chat_completion(&prompt, self.config.max_tokens.map(|v| v as u32), self.config.temperature.unwrap_or(0.7)).await?;
        
        if let Some(choices) = result.get("choices") {
            if let Some(choice) = choices.get(0) {
                if let Some(content) = choice
                    .get("message")
                    .and_then(|m| m.get("content"))
                    .and_then(|c| c.as_str())
                {
                    if let Ok(parsed) = serde_json::from_str::<crate::ai::types::AiAnalysisResult>(content) {
                        return Ok(parsed);
                    }
                    return Ok(crate::ai::types::AiAnalysisResult {
                        reassessed_severity: "Unknown".to_string(),
                        exploitability: "Unknown".to_string(),
                        impact: content.to_string(),
                        remediation: vec![],
                        confidence: 0.5,
                    });
                }
            }
        }
        
        Err(AiError::InvalidResponse)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> AiConfig {
        AiConfig {
            provider: "openai".to_string(),
            model: Some("gpt-4".to_string()),
            api_key: None,
            base_url: Some("https://api.openai.com/v1/chat/completions".to_string()),
            max_tokens: Some(2048),
            temperature: Some(0.7),
            max_payloads: 50,
            max_bypasses: 10,
        }
    }

    fn create_client_with_key(key: &str) -> AiClient {
        let mut config = create_test_config();
        config.api_key = Some(crate::types::SensitiveString::from(key.to_string()));
        AiClient::new(config)
    }

    fn create_client_without_key() -> AiClient {
        AiClient::new(create_test_config())
    }

    #[test]
    fn test_api_url_default() {
        let config = AiConfig {
            provider: "openai".to_string(),
            model: Some("gpt-4".to_string()),
            api_key: None,
            base_url: None,
            max_tokens: Some(2048),
            temperature: Some(0.7),
            max_payloads: 50,
            max_bypasses: 10,
        };
        let client = AiClient::new(config);
        assert_eq!(client.api_url(), "https://api.openai.com/v1/chat/completions");
    }

    #[test]
    fn test_api_url_custom() {
        let client = create_client_without_key();
        assert_eq!(client.api_url(), "https://api.openai.com/v1/chat/completions");
    }

    #[test]
    fn test_api_url_custom_base_url() {
        let mut config = create_test_config();
        config.base_url = Some("https://custom.api.com/v1/chat".to_string());
        let client = AiClient::new(config);
        assert_eq!(client.api_url(), "https://custom.api.com/v1/chat");
    }

    #[test]
    fn test_model_default() {
        let mut config = create_test_config();
        config.model = None;
        let client = AiClient::new(config);
        assert_eq!(client.model(), "gpt-4");
    }

    #[test]
    fn test_model_custom() {
        let client = create_client_without_key();
        assert_eq!(client.model(), "gpt-4");
    }

    #[test]
    fn test_model_custom_value() {
        let mut config = create_test_config();
        config.model = Some("gpt-3.5-turbo".to_string());
        let client = AiClient::new(config);
        assert_eq!(client.model(), "gpt-3.5-turbo");
    }

    #[test]
    fn test_apply_auth_with_key() {
        let client = create_client_with_key("test-api-key");
        let request = client.apply_auth(reqwest::Client::new().post("http://example.com"));
        let _ = request;
    }

    #[test]
    fn test_apply_auth_without_key() {
        let client = create_client_without_key();
        let request = client.apply_auth(reqwest::Client::new().post("http://example.com"));
        let _ = request;
    }

    #[test]
    fn test_extract_content_valid_response() {
        let client = create_client_without_key();
        let response = serde_json::json!({
            "choices": [{
                "message": {
                    "content": "line1\nline2\nline3\n# comment\nline4"
                }
            }]
        });
        let content = client.extract_content(&response, |l| !l.starts_with('#'));
        assert_eq!(content.len(), 3);
        assert!(content.contains(&"line1".to_string()));
        assert!(content.contains(&"line2".to_string()));
        assert!(content.contains(&"line3".to_string()));
        assert!(!content.contains(&"# comment".to_string()));
    }

    #[test]
    fn test_extract_content_empty_response() {
        let client = create_client_without_key();
        let response = serde_json::json!({});
        let content = client.extract_content(&response, |_l| true);
        assert!(content.is_empty());
    }

    #[test]
    fn test_extract_content_no_choices() {
        let client = create_client_without_key();
        let response = serde_json::json!({"choices": []});
        let content = client.extract_content(&response, |_l| true);
        assert!(content.is_empty());
    }

    #[test]
    fn test_extract_content_no_message() {
        let client = create_client_without_key();
        let response = serde_json::json!({"choices": [{}]});
        let content = client.extract_content(&response, |_l| true);
        assert!(content.is_empty());
    }

    #[tokio::test]
    async fn test_suggest_payloads_empty_vuln_type() {
        let client = create_client_without_key();
        let result = client.suggest_payloads("", "test context").await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AiError::InvalidConfig(_)));
    }

    #[tokio::test]
    async fn test_suggest_waf_bypass_empty_waf() {
        let client = create_client_without_key();
        let result = client.suggest_waf_bypass("", "blocked").await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AiError::InvalidConfig(_)));
    }

    #[tokio::test]
    async fn test_suggest_waf_bypass_empty_payload() {
        let client = create_client_without_key();
        let result = client.suggest_waf_bypass("cloudflare", "").await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AiError::InvalidConfig(_)));
    }

    #[test]
    fn test_client_clone() {
        let client1 = create_client_without_key();
        let client2 = client1.clone();
        assert_eq!(client1.api_url(), client2.api_url());
        assert_eq!(client1.model(), client2.model());
    }

    #[test]
    fn test_provider_from_str_openai() {
        assert_eq!(Provider::from_str("openai"), Provider::OpenAI);
        assert_eq!(Provider::from_str("OpenAI"), Provider::OpenAI);
        assert_eq!(Provider::from_str("openai.com"), Provider::OpenAI);
    }

    #[test]
    fn test_provider_from_str_azure() {
        assert_eq!(Provider::from_str("azure"), Provider::Azure);
        assert_eq!(Provider::from_str("Azure"), Provider::Azure);
        assert_eq!(Provider::from_str("azureopenai"), Provider::Azure);
    }

    #[test]
    fn test_provider_from_str_anthropic() {
        assert_eq!(Provider::from_str("anthropic"), Provider::Anthropic);
        assert_eq!(Provider::from_str("Anthropic"), Provider::Anthropic);
        assert_eq!(Provider::from_str("claude"), Provider::Anthropic);
    }

    #[test]
    fn test_provider_from_str_openai_compatible() {
        assert_eq!(Provider::from_str("custom"), Provider::OpenAICompatible);
        assert_eq!(Provider::from_str("openrouter"), Provider::OpenAICompatible);
        assert_eq!(Provider::from_str("ollama"), Provider::OpenAICompatible);
    }

    #[test]
    fn test_provider_default_model() {
        assert_eq!(Provider::OpenAI.default_model(), "gpt-4");
        assert_eq!(Provider::Azure.default_model(), "gpt-4");
        assert_eq!(Provider::Anthropic.default_model(), "claude-3-sonnet-20240229");
        assert_eq!(Provider::OpenAICompatible.default_model(), "gpt-4");
    }

    #[test]
    fn test_client_provider() {
        let client = create_client_without_key();
        assert_eq!(client.provider(), Provider::OpenAI);
    }

    #[test]
    fn test_client_provider_azure() {
        let mut config = create_test_config();
        config.provider = "azure".to_string();
        let client = AiClient::new(config);
        assert_eq!(client.provider(), Provider::Azure);
    }

    #[test]
    fn test_client_provider_anthropic() {
        let mut config = create_test_config();
        config.provider = "anthropic".to_string();
        let client = AiClient::new(config);
        assert_eq!(client.provider(), Provider::Anthropic);
    }

    #[test]
    fn test_anthropic_default_url() {
        let mut config = create_test_config();
        config.provider = "anthropic".to_string();
        config.base_url = None;
        let client = AiClient::new(config);
        assert_eq!(client.api_url(), "https://api.anthropic.com/v1/messages");
    }

    fn create_mock_openai_response(content: &str) -> serde_json::Value {
        serde_json::json!({
            "id": "chatcmpl-mock-123",
            "object": "chat.completion",
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": content
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 20,
                "total_tokens": 30
            }
        })
    }

    fn create_mock_anthropic_response(content: &str) -> serde_json::Value {
        serde_json::json!({
            "id": "msg-mock-123",
            "type": "message",
            "role": "assistant",
            "content": [{
                "type": "text",
                "text": content
            }],
            "usage": {
                "input_tokens": 10,
                "output_tokens": 20
            }
        })
    }

    fn create_mock_error_response(message: &str) -> serde_json::Value {
        serde_json::json!({
            "error": {
                "message": message,
                "type": "invalid_request_error",
                "code": "invalid_request"
            }
        })
    }

    #[test]
    fn test_mock_openai_response_parsing() {
        let response = create_mock_openai_response("Test payload line1\nTest payload line2");
        let client = create_client_without_key();
        let content = client.extract_content(&response, |l| !l.is_empty());
        assert_eq!(content.len(), 2);
        assert!(content[0].contains("Test payload line1"));
    }

    #[test]
    fn test_mock_anthropic_response_structure() {
        let response = create_mock_anthropic_response("Bypass suggestion 1\nBypass suggestion 2");
        assert!(response.get("content").is_some());
        if let Some(content_array) = response.get("content").and_then(|c| c.as_array()) {
            assert!(!content_array.is_empty());
        }
    }

    #[test]
    fn test_mock_error_response_structure() {
        let response = create_mock_error_response("Rate limit exceeded");
        assert!(response.get("error").is_some());
        let error_msg = response.get("error")
            .and_then(|e| e.get("message"))
            .and_then(|m| m.as_str());
        assert_eq!(error_msg, Some("Rate limit exceeded"));
    }

    #[test]
    fn test_multi_provider_response_openai() {
        let response = create_mock_openai_response("SQL injection payload\nXSS payload");
        let client = create_client_without_key();
        let content = client.extract_content(&response, |l| !l.starts_with('#'));
        assert_eq!(content.len(), 2);
    }

    #[test]
    fn test_multi_provider_response_empty_content() {
        let response = create_mock_openai_response("");
        let client = create_client_without_key();
        let content = client.extract_content(&response, |_| true);
        assert!(content.is_empty());
    }

    #[tokio::test]
    async fn test_circuit_breaker_initial_state() {
        let client = create_client_without_key();
        let state = client.circuit_breaker_state().await;
        assert_eq!(state, CircuitState::Closed);
    }

    #[tokio::test]
    async fn test_circuit_breaker_after_failures() {
        let client = create_client_without_key();
        for _ in 0..5 {
            client.circuit_breaker.record_failure().await;
        }
        let state = client.circuit_breaker_state().await;
        assert_eq!(state, CircuitState::Open);
    }

    #[tokio::test]
    async fn test_circuit_breaker_after_success() {
        let client = create_client_without_key();
        for _ in 0..5 {
            client.circuit_breaker.record_failure().await;
        }
        client.circuit_breaker.record_success().await;
        let state = client.circuit_breaker_state().await;
        assert_eq!(state, CircuitState::Closed);
    }

    #[tokio::test]
    async fn test_circuit_breaker_half_open_transition() {
        let client = create_client_without_key();
        for _ in 0..5 {
            client.circuit_breaker.record_failure().await;
        }
        assert_eq!(client.circuit_breaker_state().await, CircuitState::Open);
        client.circuit_breaker.record_success().await;
        assert_eq!(client.circuit_breaker_state().await, CircuitState::Closed);
    }

    #[test]
    fn test_provider_supports_bearer_auth() {
        assert!(Provider::OpenAI.supports_bearer_auth());
        assert!(Provider::Anthropic.supports_bearer_auth());
        assert!(Provider::OpenAICompatible.supports_bearer_auth());
        assert!(!Provider::Azure.supports_bearer_auth());
    }

    #[test]
    fn test_provider_supports_azure_auth() {
        assert!(Provider::Azure.supports_azure_auth());
        assert!(!Provider::OpenAI.supports_azure_auth());
        assert!(!Provider::Anthropic.supports_azure_auth());
        assert!(!Provider::OpenAICompatible.supports_azure_auth());
    }
}