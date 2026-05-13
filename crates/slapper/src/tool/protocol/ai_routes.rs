use axum::{extract::State, http::HeaderMap, routing::get, routing::post, Json, Router};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use subtle::ConstantTimeEq;

#[cfg(feature = "ai-integration")]
use crate::ai::AiClient;

#[derive(Clone)]
pub struct AiState {
    pub api_key: Option<String>,
    #[cfg(feature = "ai-integration")]
    pub ai_client: Option<AiClient>,
}

impl AiState {
    pub fn new(api_key: Option<String>) -> Self {
        Self {
            api_key,
            #[cfg(feature = "ai-integration")]
            ai_client: None,
        }
    }

    #[cfg(feature = "ai-integration")]
    pub fn with_ai_client(mut self, client: AiClient) -> Self {
        self.ai_client = Some(client);
        self
    }
}

fn require_auth(state: &Arc<AiState>, headers: &HeaderMap) -> Result<(), &'static str> {
    if let Some(ref key) = state.api_key {
        let auth = headers
            .get("authorization")
            .or_else(|| headers.get("x-api-key"))
            .and_then(|v| v.to_str().ok());

        match auth {
            Some(v) if bool::from(key.as_bytes().ct_eq(v.as_bytes())) => Ok(()),
            _ => Err("Invalid or missing API key"),
        }
    } else {
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
pub struct AnalyzeRequest {
    pub findings: Vec<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct AnalyzeResponse {
    pub status: String,
    pub analysis: String,
    pub findings_count: usize,
}

#[cfg(feature = "ai-integration")]
async fn analyze_findings(
    State(state): State<Arc<AiState>>,
    headers: HeaderMap,
    Json(req): Json<AnalyzeRequest>,
) -> Result<Json<AnalyzeResponse>, &'static str> {
    require_auth(&state, &headers)?;

    let analysis = if let Some(ref client) = state.ai_client {
        match client.analyze_findings(&req.findings).await {
            Ok(result) => {
                if let Some(choices) = result.get("choices") {
                    if let Some(choice) = choices.get(0) {
                        if let Some(content) = choice
                            .get("message")
                            .and_then(|m| m.get("content"))
                            .and_then(|c| c.as_str())
                        {
                            return Ok(Json(AnalyzeResponse {
                                status: "success".to_string(),
                                analysis: content.to_string(),
                                findings_count: req.findings.len(),
                            }));
                        }
                    }
                }
                "Analysis completed but response format unexpected".to_string()
            }
            Err(e) => format!("AI analysis failed: {}", e),
        }
    } else {
        "AI analysis requires a configured API key. Configure ai.base_url and ai.api_key in your slapper config to enable real analysis.".to_string()
    };

    Ok(Json(AnalyzeResponse {
        status: if state.ai_client.is_some() {
            "partial".to_string()
        } else {
            "placeholder".to_string()
        },
        analysis,
        findings_count: req.findings.len(),
    }))
}

#[cfg(not(feature = "ai-integration"))]
async fn analyze_findings(
    State(state): State<Arc<AiState>>,
    headers: HeaderMap,
    Json(req): Json<AnalyzeRequest>,
) -> Result<Json<AnalyzeResponse>, &'static str> {
    require_auth(&state, &headers)?;
    Ok(Json(AnalyzeResponse {
        status: "placeholder".to_string(),
        analysis: "AI analysis requires ai-integration feature enabled.".to_string(),
        findings_count: req.findings.len(),
    }))
}

#[derive(Debug, Deserialize)]
pub struct SuggestPayloadsRequest {
    pub target: String,
    pub vuln_type: String,
    pub context: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SuggestPayloadsResponse {
    pub status: String,
    pub payloads: Vec<String>,
    pub vuln_type: String,
}

#[cfg(feature = "ai-integration")]
async fn suggest_payloads(
    State(state): State<Arc<AiState>>,
    headers: HeaderMap,
    Json(req): Json<SuggestPayloadsRequest>,
) -> Result<Json<SuggestPayloadsResponse>, &'static str> {
    require_auth(&state, &headers)?;

    let context = req.context.unwrap_or_default();
    let payloads = if let Some(ref client) = state.ai_client {
        match client.suggest_payloads(&req.vuln_type, &context).await {
            Ok(payloads) => payloads,
            Err(_) => vec![
                format!("' OR 1=1 -- (example {} payload)", req.vuln_type),
                format!(
                    "\"; DROP TABLE users; -- (example {} payload)",
                    req.vuln_type
                ),
                format!(
                    "<script>alert('xss')</script> (example {} payload)",
                    req.vuln_type
                ),
            ],
        }
    } else {
        vec![
            format!("' OR 1=1 -- (example {} payload)", req.vuln_type),
            format!(
                "\"; DROP TABLE users; -- (example {} payload)",
                req.vuln_type
            ),
            format!(
                "<script>alert('xss')</script> (example {} payload)",
                req.vuln_type
            ),
        ]
    };

    Ok(Json(SuggestPayloadsResponse {
        status: if state.ai_client.is_some() {
            "success".to_string()
        } else {
            "placeholder".to_string()
        },
        payloads,
        vuln_type: req.vuln_type,
    }))
}

#[cfg(not(feature = "ai-integration"))]
async fn suggest_payloads(
    State(state): State<Arc<AiState>>,
    headers: HeaderMap,
    Json(req): Json<SuggestPayloadsRequest>,
) -> Result<Json<SuggestPayloadsResponse>, &'static str> {
    require_auth(&state, &headers)?;
    Ok(Json(SuggestPayloadsResponse {
        status: "placeholder".to_string(),
        payloads: vec![
            format!("' OR 1=1 -- (example {} payload)", req.vuln_type),
            format!(
                "\"; DROP TABLE users; -- (example {} payload)",
                req.vuln_type
            ),
            format!(
                "<script>alert('xss')</script> (example {} payload)",
                req.vuln_type
            ),
        ],
        vuln_type: req.vuln_type,
    }))
}

#[derive(Debug, Deserialize)]
pub struct WafBypassRequest {
    pub waf_name: String,
    pub blocked_payload: String,
}

#[derive(Debug, Serialize)]
pub struct WafBypassResponse {
    pub status: String,
    pub bypass_suggestions: Vec<String>,
    pub waf_name: String,
}

#[cfg(feature = "ai-integration")]
async fn waf_bypass(
    State(state): State<Arc<AiState>>,
    headers: HeaderMap,
    Json(req): Json<WafBypassRequest>,
) -> Result<Json<WafBypassResponse>, &'static str> {
    require_auth(&state, &headers)?;

    let bypasses = if let Some(ref client) = state.ai_client {
        match client
            .suggest_waf_bypass(&req.waf_name, &req.blocked_payload)
            .await
        {
            Ok(bypasses) => bypasses,
            Err(_) => vec![
                format!("Try encoding payload for {} bypass", req.waf_name),
                format!("Try header manipulation technique for {}", req.waf_name),
                format!("Try HTTP parameter pollution for {}", req.waf_name),
            ],
        }
    } else {
        vec![
            format!("Try encoding payload for {} bypass", req.waf_name),
            format!("Try header manipulation technique for {}", req.waf_name),
            format!("Try HTTP parameter pollution for {}", req.waf_name),
        ]
    };

    Ok(Json(WafBypassResponse {
        status: if state.ai_client.is_some() {
            "success".to_string()
        } else {
            "placeholder".to_string()
        },
        bypass_suggestions: bypasses,
        waf_name: req.waf_name,
    }))
}

#[cfg(not(feature = "ai-integration"))]
async fn waf_bypass(
    State(state): State<Arc<AiState>>,
    headers: HeaderMap,
    Json(req): Json<WafBypassRequest>,
) -> Result<Json<WafBypassResponse>, &'static str> {
    require_auth(&state, &headers)?;
    Ok(Json(WafBypassResponse {
        status: "placeholder".to_string(),
        bypass_suggestions: vec![
            format!("Try encoding payload for {} bypass", req.waf_name),
            format!("Try header manipulation technique for {}", req.waf_name),
            format!("Try HTTP parameter pollution for {}", req.waf_name),
        ],
        waf_name: req.waf_name,
    }))
}

#[derive(Debug, Deserialize)]
pub struct ScanStrategyRequest {
    pub target: String,
    pub current_findings: Option<Vec<serde_json::Value>>,
    pub scan_depth: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ScanStrategyResponse {
    pub status: String,
    pub recommended_strategy: String,
    pub reasoning: String,
}

#[cfg(feature = "ai-integration")]
async fn scan_strategy(
    State(state): State<Arc<AiState>>,
    headers: HeaderMap,
    Json(req): Json<ScanStrategyRequest>,
) -> Result<Json<ScanStrategyResponse>, &'static str> {
    require_auth(&state, &headers)?;
    let depth = req.scan_depth.unwrap_or_else(|| "standard".to_string());
    let findings_count = req.current_findings.as_ref().map_or(0, |f| f.len());

    let strategy = if findings_count > 5 {
        "deep"
    } else if findings_count > 0 {
        "thorough"
    } else {
        &depth
    };

    let reasoning = if state.ai_client.is_some() {
        format!(
            "Based on {} findings for target '{}', a {} scan is recommended.",
            findings_count, req.target, strategy
        )
    } else {
        format!(
            "Based on {} findings for target '{}', a {} scan is recommended. Configure AI integration for adaptive strategy.",
            findings_count, req.target, strategy
        )
    };

    Ok(Json(ScanStrategyResponse {
        status: if state.ai_client.is_some() {
            "success".to_string()
        } else {
            "placeholder".to_string()
        },
        recommended_strategy: strategy.to_string(),
        reasoning,
    }))
}

#[cfg(not(feature = "ai-integration"))]
async fn scan_strategy(
    State(state): State<Arc<AiState>>,
    headers: HeaderMap,
    Json(req): Json<ScanStrategyRequest>,
) -> Result<Json<ScanStrategyResponse>, &'static str> {
    require_auth(&state, &headers)?;
    let depth = req.scan_depth.unwrap_or_else(|| "standard".to_string());
    let findings_count = req.current_findings.as_ref().map_or(0, |f| f.len());

    let strategy = if findings_count > 5 {
        "deep"
    } else if findings_count > 0 {
        "thorough"
    } else {
        &depth
    };

    Ok(Json(ScanStrategyResponse {
        status: "placeholder".to_string(),
        recommended_strategy: strategy.to_string(),
        reasoning: format!(
            "Based on {} findings for target '{}', a {} scan is recommended. Configure AI integration for adaptive strategy.",
            findings_count, req.target, strategy
        ),
    }))
}

#[derive(Debug, Serialize)]
pub struct CircuitBreakerResponse {
    pub status: String,
    pub state: String,
    pub description: String,
}

#[cfg(feature = "ai-integration")]
async fn circuit_breaker_status(
    State(state): State<Arc<AiState>>,
    headers: HeaderMap,
) -> Result<Json<CircuitBreakerResponse>, &'static str> {
    require_auth(&state, &headers)?;

    if let Some(ref client) = state.ai_client {
        let cb_state = client.circuit_breaker_state();
        let (state_str, description) = match cb_state {
            crate::utils::circuit_breaker::CircuitState::Closed => {
                ("closed", "Circuit breaker is closed. Requests are allowed.")
            }
            crate::utils::circuit_breaker::CircuitState::Open => (
                "open",
                "Circuit breaker is open. Requests are being rejected due to failures.",
            ),
            crate::utils::circuit_breaker::CircuitState::HalfOpen => (
                "half_open",
                "Circuit breaker is half-open. Testing with limited requests.",
            ),
        };
        return Ok(Json(CircuitBreakerResponse {
            status: "ok".to_string(),
            state: state_str.to_string(),
            description: description.to_string(),
        }));
    }

    Ok(Json(CircuitBreakerResponse {
        status: "ok".to_string(),
        state: "unknown".to_string(),
        description:
            "Circuit breaker state requires an active AI client. Configure ai.base_url to enable."
                .to_string(),
    }))
}

#[cfg(not(feature = "ai-integration"))]
async fn circuit_breaker_status(
    State(state): State<Arc<AiState>>,
    headers: HeaderMap,
) -> Result<Json<CircuitBreakerResponse>, &'static str> {
    require_auth(&state, &headers)?;
    Ok(Json(CircuitBreakerResponse {
        status: "ok".to_string(),
        state: "unknown".to_string(),
        description: "Circuit breaker state requires ai-integration feature enabled.".to_string(),
    }))
}

#[derive(Debug, Deserialize)]
pub struct ValidateConfigRequest {
    pub base_url: Option<String>,
    pub model: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ValidateConfigResponse {
    pub valid: bool,
    pub issues: Vec<String>,
    pub recommendations: Vec<String>,
}

async fn validate_config(
    State(state): State<Arc<AiState>>,
    headers: HeaderMap,
    Json(req): Json<ValidateConfigRequest>,
) -> Result<Json<ValidateConfigResponse>, &'static str> {
    require_auth(&state, &headers)?;
    let mut issues = Vec::new();
    let mut recommendations = Vec::new();

    if req.base_url.is_none() {
        issues.push("base_url is not configured".to_string());
        recommendations.push("Set ai.base_url to your OpenAI-compatible API endpoint".to_string());
    }

    if req.model.is_none() {
        recommendations.push("Consider setting ai.model (default varies by provider)".to_string());
    }

    if let Some(ref url) = req.base_url {
        if !url.starts_with("http://") && !url.starts_with("https://") {
            issues.push("base_url should start with http:// or https://".to_string());
        }
    }

    let valid = issues.is_empty();

    Ok(Json(ValidateConfigResponse {
        valid,
        issues,
        recommendations,
    }))
}

#[cfg(feature = "ai-integration")]
pub fn router(ai_config: Option<crate::config::AiConfig>) -> Router {
    let api_key = ai_config
        .as_ref()
        .and_then(|c| c.api_key.clone())
        .map(|k| k.expose_secret().to_string());
    let mut state = AiState::new(api_key);

    if let Some(config) = ai_config {
        match AiClient::new(config) {
            Ok(client) => {
                state = state.with_ai_client(client);
            }
            Err(e) => {
                tracing::warn!("Failed to initialize AI routes client: {}", e);
            }
        }
    }

    Router::new()
        .route("/api/v1/ai/analyze", post(analyze_findings))
        .route("/api/v1/ai/suggest-payloads", post(suggest_payloads))
        .route("/api/v1/ai/waf-bypass", post(waf_bypass))
        .route("/api/v1/ai/scan-strategy", post(scan_strategy))
        .route("/api/v1/ai/circuit-breaker", get(circuit_breaker_status))
        .route("/api/v1/ai/validate-config", post(validate_config))
        .with_state(Arc::new(state))
}

#[cfg(not(feature = "ai-integration"))]
pub fn router(_ai_config: Option<crate::config::AiConfig>) -> Router {
    let state = Arc::new(AiState::new(None));
    Router::new()
        .route("/api/v1/ai/analyze", post(analyze_findings))
        .route("/api/v1/ai/suggest-payloads", post(suggest_payloads))
        .route("/api/v1/ai/waf-bypass", post(waf_bypass))
        .route("/api/v1/ai/scan-strategy", post(scan_strategy))
        .route("/api/v1/ai/circuit-breaker", get(circuit_breaker_status))
        .route("/api/v1/ai/validate-config", post(validate_config))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_analyze_findings_counts() {
        let state = Arc::new(AiState::new(None));
        let req = AnalyzeRequest {
            findings: vec![
                serde_json::json!({"sev": "high"}),
                serde_json::json!({"sev": "low"}),
            ],
        };
        let resp = analyze_findings(State(state), HeaderMap::new(), Json(req)).await;
        assert!(resp.is_ok());
        let resp = resp.unwrap();
        assert_eq!(resp.findings_count, 2);
        assert_eq!(resp.status, "placeholder");
    }

    #[tokio::test]
    async fn test_suggest_payloads_returns_vuln_type() {
        let state = Arc::new(AiState::new(None));
        let req = SuggestPayloadsRequest {
            target: "http://example.com".to_string(),
            vuln_type: "sqli".to_string(),
            context: None,
        };
        let resp = suggest_payloads(State(state), HeaderMap::new(), Json(req)).await;
        assert!(resp.is_ok());
        let resp = resp.unwrap();
        assert_eq!(resp.vuln_type, "sqli");
        assert_eq!(resp.payloads.len(), 3);
    }

    #[tokio::test]
    async fn test_waf_bypass_returns_suggestions() {
        let state = Arc::new(AiState::new(None));
        let req = WafBypassRequest {
            waf_name: "Cloudflare".to_string(),
            blocked_payload: "' OR 1=1".to_string(),
        };
        let resp = waf_bypass(State(state), HeaderMap::new(), Json(req)).await;
        assert!(resp.is_ok());
        let resp = resp.unwrap();
        assert_eq!(resp.waf_name, "Cloudflare");
        assert!(!resp.bypass_suggestions.is_empty());
    }

    #[tokio::test]
    async fn test_scan_strategy_deep_for_many_findings() {
        let state = Arc::new(AiState::new(None));
        let req = ScanStrategyRequest {
            target: "http://example.com".to_string(),
            current_findings: Some(vec![serde_json::json!({}); 6]),
            scan_depth: Some("light".to_string()),
        };
        let resp = scan_strategy(State(state), HeaderMap::new(), Json(req)).await;
        assert!(resp.is_ok());
        let resp = resp.unwrap();
        assert_eq!(resp.recommended_strategy, "deep");
    }

    #[tokio::test]
    async fn test_scan_strategy_standard_for_no_findings() {
        let state = Arc::new(AiState::new(None));
        let req = ScanStrategyRequest {
            target: "http://example.com".to_string(),
            current_findings: Some(vec![]),
            scan_depth: Some("standard".to_string()),
        };
        let resp = scan_strategy(State(state), HeaderMap::new(), Json(req)).await;
        assert!(resp.is_ok());
        let resp = resp.unwrap();
        assert_eq!(resp.recommended_strategy, "standard");
    }

    #[tokio::test]
    async fn test_validate_config_missing_base_url() {
        let state = Arc::new(AiState::new(None));
        let req = ValidateConfigRequest {
            base_url: None,
            model: None,
        };
        let resp = validate_config(State(state), HeaderMap::new(), Json(req)).await;
        assert!(resp.is_ok());
        let resp = resp.unwrap();
        assert!(!resp.valid);
        assert!(resp.issues.iter().any(|i| i.contains("base_url")));
    }

    #[tokio::test]
    async fn test_validate_config_valid() {
        let state = Arc::new(AiState::new(None));
        let req = ValidateConfigRequest {
            base_url: Some("https://api.openai.com".to_string()),
            model: Some("gpt-4".to_string()),
        };
        let resp = validate_config(State(state), HeaderMap::new(), Json(req)).await;
        assert!(resp.is_ok());
        let resp = resp.unwrap();
        assert!(resp.valid);
        assert!(resp.issues.is_empty());
    }

    #[tokio::test]
    async fn test_validate_config_bad_url() {
        let state = Arc::new(AiState::new(None));
        let req = ValidateConfigRequest {
            base_url: Some("not-a-url".to_string()),
            model: None,
        };
        let resp = validate_config(State(state), HeaderMap::new(), Json(req)).await;
        assert!(resp.is_ok());
        let resp = resp.unwrap();
        assert!(!resp.valid);
        assert!(resp.issues.iter().any(|i| i.contains("http")));
    }

    #[test]
    fn test_router_has_routes() {
        let _router = router(None);
    }
}
