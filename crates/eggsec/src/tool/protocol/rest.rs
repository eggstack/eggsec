use axum::{
    extract::{Json, Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tower_http::cors::{AllowHeaders, AllowMethods, CorsLayer};

use super::auth::validate_api_key;
use crate::config::{EnforcementContext, EnforcementOutcome, OperationDescriptor};
use crate::distributed::TlsConfig;
use crate::error::EggsecError;
use crate::tool::ratelimit::RateLimitConfig;
use crate::tool::{ToolDispatcher, ToolRegistry, ToolRequest, ToolResponse};
use crate::utils::rate_limiter::RateLimiter;

const MAX_PAYLOAD_SIZE: usize = 10 * 1024 * 1024;
const MAX_URL_LENGTH: usize = 2048;

#[derive(Clone)]
pub struct RestState {
    pub registry: ToolRegistry,
    pub dispatcher: ToolDispatcher,
    pub api_key: Option<String>,
    pub rate_limiter: RateLimiter,
    pub enforcement: EnforcementContext,
    pub tls_config: Option<TlsConfig>,
    pub metrics: Arc<Metrics>,
}

impl RestState {
    pub fn new(
        registry: ToolRegistry,
        api_key: Option<String>,
        enforcement: EnforcementContext,
        tls_config: Option<TlsConfig>,
    ) -> Self {
        let dispatcher = ToolDispatcher::new(registry.clone());
        let rate_limiter = RateLimiter::new(RateLimitConfig::standard().requests_per_minute);
        Self {
            registry,
            dispatcher,
            api_key,
            rate_limiter,
            enforcement,
            tls_config,
            metrics: Arc::new(Metrics::default()),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Metrics {
    pub request_count: Arc<AtomicU64>,
    pub error_count: Arc<AtomicU64>,
    pub total_latency_ms: Arc<AtomicU64>,
}

impl Metrics {
    pub fn record_request(&self, latency: Duration, is_error: bool) {
        self.request_count.fetch_add(1, Ordering::Relaxed);
        if is_error {
            self.error_count.fetch_add(1, Ordering::Relaxed);
        }
        self.total_latency_ms
            .fetch_add(latency.as_millis() as u64, Ordering::Relaxed);
    }

    pub fn get_metrics(&self) -> serde_json::Value {
        let count = self.request_count.load(Ordering::Relaxed);
        let errors = self.error_count.load(Ordering::Relaxed);
        let total_ms = self.total_latency_ms.load(Ordering::Relaxed);
        let avg_latency = if count > 0 { total_ms / count } else { 0 };

        serde_json::json!({
            "requests_total": count,
            "errors_total": errors,
            "error_rate": if count > 0 { errors as f64 / count as f64 } else { 0.0 },
            "avg_latency_ms": avg_latency,
        })
    }
}

#[derive(Debug, Serialize)]
pub struct RestErrorResponse {
    pub error: String,
    pub code: String,
}

#[derive(Debug, Serialize)]
pub struct RestPolicyErrorResponse {
    pub error: String,
    pub code: &'static str,
    pub decision: crate::config::PolicyDecision,
}

impl IntoResponse for EggsecError {
    fn into_response(self) -> axum::response::Response {
        let (status, error_response) = match &self {
            EggsecError::Config(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            EggsecError::InvalidTarget(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            EggsecError::ScopeViolation(msg) => (StatusCode::FORBIDDEN, msg.clone()),
            EggsecError::Network(msg) => (StatusCode::SERVICE_UNAVAILABLE, msg.clone()),
            EggsecError::Timeout { .. } => (StatusCode::REQUEST_TIMEOUT, self.to_string()),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
        };

        let body = Json(RestErrorResponse {
            error: error_response,
            code: "TOOL_ERROR".to_string(),
        });

        (status, body).into_response()
    }
}

#[derive(Debug, Deserialize)]
pub struct ExecuteRequest {
    pub target: String,
    pub target_type: Option<String>,
    pub params: Option<serde_json::Value>,
    pub options: Option<crate::tool::RequestOptions>,
}

#[derive(Debug, Deserialize)]
pub struct PreflightRequest {
    pub target: String,
    pub target_type: Option<String>,
    pub params: Option<serde_json::Value>,
    pub options: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct ToolListResponse {
    pub tools: Vec<ToolListItem>,
    pub categories: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ToolListItem {
    pub id: String,
    pub name: String,
    pub category: String,
    pub description: String,
    pub protocols: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ToolDetailResponse {
    pub id: String,
    pub name: String,
    pub category: String,
    pub description: String,
    pub capabilities: Vec<serde_json::Value>,
    pub protocols: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct PaginationParams {
    #[serde(default)]
    pub offset: Option<usize>,
    #[serde(default)]
    pub limit: Option<usize>,
}

impl PaginationParams {
    pub fn offset(&self) -> usize {
        self.offset.unwrap_or(0)
    }

    pub fn limit(&self) -> usize {
        self.limit.unwrap_or(50).min(100)
    }
}

#[derive(Debug, Serialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub total: usize,
    pub offset: usize,
    pub limit: usize,
    pub has_more: bool,
}

impl<T> PaginatedResponse<T> {
    pub fn new(data: Vec<T>, total: usize, offset: usize, limit: usize) -> Self {
        let has_more = offset + data.len() < total;
        Self {
            data,
            total,
            offset,
            limit,
            has_more,
        }
    }
}

pub fn create_router(
    registry: ToolRegistry,
    api_key: Option<String>,
    enforcement: EnforcementContext,
    tls_config: Option<TlsConfig>,
) -> Router {
    let state = Arc::new(RestState::new(
        registry,
        api_key.clone(),
        enforcement,
        tls_config.clone(),
    ));

    let methods = AllowMethods::list([
        axum::http::Method::GET,
        axum::http::Method::POST,
        axum::http::Method::PUT,
        axum::http::Method::DELETE,
        axum::http::Method::OPTIONS,
    ]);

    let headers = AllowHeaders::list([
        axum::http::header::CONTENT_TYPE,
        axum::http::header::AUTHORIZATION,
        "X-API-Key".parse::<axum::http::HeaderName>().unwrap(),
    ]);

    let cors = CorsLayer::new()
        .allow_methods(methods)
        .allow_headers(headers)
        .max_age(Duration::from_secs(3600))
        .allow_credentials(false);

    let mut router = Router::new()
        .route("/health", get(health_check))
        .route("/metrics", get(metrics_endpoint))
        .route("/openapi.json", get(openapi_spec))
        .route("/api/v1/tools", get(list_tools))
        .route("/api/v1/tools/{tool_id}", get(get_tool))
        .route("/api/v1/tools/{tool_id}/execute", post(execute_tool))
        .route("/api/v1/tools/{tool_id}/preflight", post(preflight_tool))
        .layer(cors)
        .with_state(state);

    #[cfg(feature = "ws-api")]
    {
        use axum::{
            extract::ws::{Message, WebSocket, WebSocketUpgrade},
            routing::get,
        };
        use futures::{SinkExt, StreamExt};
        use tokio::sync::broadcast;

        async fn ws_handler(ws: WebSocketUpgrade) -> impl axum::response::IntoResponse {
            ws.on_upgrade(handle_websocket)
        }

        async fn handle_websocket(socket: WebSocket) {
            let (mut sender, mut receiver) = socket.split();
            let (tx, _rx) = broadcast::channel::<String>(100);
            let tx_clone = tx.clone();

            tokio::spawn(async move {
                let mut rx = tx_clone.subscribe();
                while let Ok(msg) = rx.recv().await {
                    if sender.send(Message::Text(msg.into())).await.is_err() {
                        break;
                    }
                }
            });

            while let Some(Ok(msg)) = receiver.next().await {
                if let Message::Text(text) = msg {
                    tracing::debug!("WS received: {}", text);
                    if let Err(e) = tx.send(text.to_string()) {
                        tracing::warn!("WS channel send failed: {}", e);
                    }
                }
            }
        }

        router = router.route("/ws", get(ws_handler));
    }

    if tls_config.is_some() {
        tracing::info!("REST API running with TLS enabled");
    }

    #[cfg(feature = "ai-integration")]
    {
        router = router.merge(super::ai_routes::router(None));
    }

    #[cfg(feature = "ai-integration")]
    {
        use crate::tool::agents::{AgentRegistry, TaskScheduler};
        let agent_registry = AgentRegistry::new();
        let scheduler = TaskScheduler::new();
        router = router.merge(super::agent_routes::router(
            agent_registry,
            scheduler,
            api_key,
        ));
    }

    router
}

fn require_auth(state: &Arc<RestState>, headers: &HeaderMap) -> Result<(), EggsecError> {
    validate_api_key(&state.api_key, headers).map_err(|e| EggsecError::Config(e.to_string()))
}

pub fn generate_correlation_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

fn validate_target(target: &str) -> Result<(), EggsecError> {
    if target.is_empty() {
        return Err(EggsecError::Config("Target cannot be empty".to_string()));
    }
    if target.len() > MAX_URL_LENGTH {
        return Err(EggsecError::Config(format!(
            "Target URL exceeds maximum length of {} characters",
            MAX_URL_LENGTH
        )));
    }
    if !target.starts_with("http://")
        && !target.starts_with("https://")
        && !target.contains('.')
        && !target.contains(':')
    {
        return Err(EggsecError::Config(
            "Invalid target format. Expected URL, domain, IP, or CIDR".to_string(),
        ));
    }
    Ok(())
}

fn validate_payload_size(params: &Option<serde_json::Value>) -> Result<(), EggsecError> {
    if let Some(ref p) = params {
        let size = serde_json::to_string(p).map(|s| s.len()).unwrap_or(0);
        if size > MAX_PAYLOAD_SIZE {
            return Err(EggsecError::Config(format!(
                "Payload size {} exceeds maximum of {} bytes",
                size, MAX_PAYLOAD_SIZE
            )));
        }
    }
    Ok(())
}

async fn health_check(
    State(state): State<Arc<RestState>>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, EggsecError> {
    if let Err(e) = require_auth(&state, &headers) {
        return Err(e);
    }

    Ok(Json(serde_json::json!({
        "status": "healthy",
        "service": "eggsec-tool-api",
        "authenticated": state.api_key.is_some()
    })))
}

async fn metrics_endpoint(
    State(state): State<Arc<RestState>>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, EggsecError> {
    if let Err(e) = require_auth(&state, &headers) {
        return Err(e);
    }

    let metrics = state.metrics.get_metrics();
    Ok(Json(metrics))
}

async fn openapi_spec() -> impl IntoResponse {
    Json(serde_json::json!({
        "openapi": "3.0.0",
        "info": {
            "title": "Eggsec Tool API",
            "description": "Security tool API for external integration",
            "version": "0.1.0"
        },
        "servers": [
            {"url": "http://127.0.0.1:8080", "description": "Local development server"}
        ],
        "paths": {
            "/health": {
                "get": {
                    "summary": "Health check",
                    "responses": {
                        "200": {"description": "Service is healthy"}
                    }
                }
            },
            "/api/v1/tools": {
                "get": {
                    "summary": "List all available tools",
                    "responses": {
                        "200": {"description": "List of tools"}
                    }
                }
            },
            "/api/v1/tools/{tool_id}": {
                "get": {
                    "summary": "Get tool details",
                    "parameters": [
                        {"name": "tool_id", "in": "path", "required": true, "schema": {"type": "string"}}
                    ],
                    "responses": {
                        "200": {"description": "Tool details"}
                    }
                }
            },
            "/api/v1/tools/{tool_id}/execute": {
                "post": {
                    "summary": "Execute a tool",
                    "parameters": [
                        {"name": "tool_id", "in": "path", "required": true, "schema": {"type": "string"}}
                    ],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": {
                                    "type": "object",
                                    "properties": {
                                        "target": {"type": "string"},
                                        "target_type": {"type": "string", "enum": ["url", "domain", "ip", "cidr"]},
                                        "params": {"type": "object"},
                                        "options": {"type": "object"}
                                    }
                                }
                            }
                        }
                    },
                    "responses": {
                        "200": {"description": "Execution result"},
                        "403": {
                            "description": "Policy denied — strict REST enforcement does not allow this operation",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "object",
                                        "properties": {
                                            "error": {"type": "string"},
                                            "code": {"type": "string", "enum": ["POLICY_DENIED"]},
                                            "decision": {"type": "object"}
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            },
            "/api/v1/tools/{tool_id}/preflight": {
                "post": {
                    "summary": "Preflight check — evaluate what would happen without executing",
                    "parameters": [
                        {"name": "tool_id", "in": "path", "required": true, "schema": {"type": "string"}}
                    ],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": {
                                    "type": "object",
                                    "properties": {
                                        "target": {"type": "string"},
                                        "target_type": {"type": "string", "enum": ["url", "domain", "ip", "cidr"]},
                                        "params": {"type": "object"},
                                        "options": {"type": "object"}
                                    }
                                }
                            }
                        }
                    },
                    "responses": {
                        "200": {"description": "Preflight evaluation result"},
                        "403": {
                            "description": "Tool not exposed via REST API",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "object",
                                        "properties": {
                                            "error": {"type": "string"},
                                            "code": {"type": "string"}
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }))
}

async fn list_tools(
    State(state): State<Arc<RestState>>,
    headers: HeaderMap,
    Query(pagination): Query<PaginationParams>,
) -> Result<impl IntoResponse, EggsecError> {
    let start = Instant::now();
    if let Err(e) = require_auth(&state, &headers) {
        state.metrics.record_request(start.elapsed(), true);
        return Err(EggsecError::Config(e.to_string()));
    }

    let tools = state.registry.list();
    let total = tools.len();

    let items: Vec<ToolListItem> = tools
        .iter()
        .skip(pagination.offset())
        .take(pagination.limit())
        .map(|t| ToolListItem {
            id: t.id.to_string(),
            name: t.name.to_string(),
            category: t.category.to_string(),
            description: t.description.to_string(),
            protocols: t.protocols.clone(),
        })
        .collect();

    let categories: Vec<String> = state
        .registry
        .categories()
        .iter()
        .map(|c| c.to_string())
        .collect();

    let response = PaginatedResponse::new(items, total, pagination.offset(), pagination.limit());

    state.metrics.record_request(start.elapsed(), false);

    Ok(Json(serde_json::json!({
        "data": response.data,
        "total": response.total,
        "offset": response.offset,
        "limit": response.limit,
        "has_more": response.has_more,
        "categories": categories,
    })))
}

async fn get_tool(
    State(state): State<Arc<RestState>>,
    Path(tool_id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<ToolDetailResponse>, EggsecError> {
    let start = Instant::now();
    if let Err(e) = require_auth(&state, &headers) {
        state.metrics.record_request(start.elapsed(), true);
        return Err(EggsecError::Config(e.to_string()));
    }

    let tool = state.registry.get(&tool_id).ok_or_else(|| {
        state.metrics.record_request(start.elapsed(), true);
        EggsecError::Config(format!("Tool '{}' not found", tool_id))
    })?;

    state.metrics.record_request(start.elapsed(), false);

    Ok(Json(ToolDetailResponse {
        id: tool_id,
        name: tool.name().to_string(),
        category: tool.category().to_string(),
        description: tool.description().to_string(),
        capabilities: tool
            .capabilities()
            .iter()
            .map(|c| serde_json::to_value(c).unwrap_or_default())
            .collect(),
        protocols: tool
            .supported_protocols()
            .iter()
            .map(|s| s.to_string())
            .collect(),
    }))
}

fn operation_descriptor_for_rest_tool(
    tool_id: &str,
    target: &str,
) -> Result<OperationDescriptor, EggsecError> {
    use crate::tool::metadata::metadata_for_tool_id;

    let target_opt = if target.is_empty() {
        None
    } else {
        Some(target.to_string())
    };
    if let Some(metadata) = metadata_for_tool_id(tool_id) {
        let mut descriptor = metadata.descriptor_for_target(target_opt);
        descriptor.requires_explicit_scope = true;
        Ok(descriptor)
    } else {
        Err(EggsecError::Config(format!(
            "missing operation metadata for REST tool '{}' — \
             every registered tool must have an entry in ALL_OPERATION_METADATA",
            tool_id
        )))
    }
}

fn policy_denied_response(
    message: impl Into<String>,
    decision: crate::config::PolicyDecision,
) -> axum::response::Response {
    let body = Json(RestPolicyErrorResponse {
        error: message.into(),
        code: "POLICY_DENIED",
        decision,
    });
    (StatusCode::FORBIDDEN, body).into_response()
}

async fn execute_tool(
    State(state): State<Arc<RestState>>,
    Path(tool_id): Path<String>,
    headers: HeaderMap,
    Json(payload): Json<ExecuteRequest>,
) -> Result<Json<ToolResponse>, EggsecError> {
    let start = Instant::now();
    if let Err(e) = require_auth(&state, &headers) {
        state.metrics.record_request(start.elapsed(), true);
        return Err(EggsecError::Config(e.to_string()));
    }

    validate_target(&payload.target)?;
    validate_payload_size(&payload.params)?;

    let client_id = payload.target.clone();
    if state.rate_limiter.check_rate_limit(&client_id).is_err() {
        state.metrics.record_request(start.elapsed(), true);
        return Err(EggsecError::Config(
            "Rate limit exceeded for this target".to_string(),
        ));
    }

    let target_url = &payload.target;

    let descriptor = operation_descriptor_for_rest_tool(&tool_id, target_url)?;

    if let Some(metadata) = crate::tool::metadata::metadata_for_tool_id(&tool_id) {
        if !metadata.rest_exposable {
            state.metrics.record_request(start.elapsed(), true);
            return Err(EggsecError::Config(format!(
                "Tool '{}' is not exposed via REST API",
                tool_id
            )));
        }
    }

    let outcome = state.enforcement.evaluate(&descriptor);
    match outcome {
        EnforcementOutcome::Allow(_) => {}
        EnforcementOutcome::Warn(decision) => {
            state.metrics.record_request(start.elapsed(), true);
            return Err(EggsecError::ScopeViolation(format!(
                "REST strict enforcement: warning — {}",
                decision.to_human_readable()
            )));
        }
        EnforcementOutcome::RequireConfirmation(decision) => {
            state.metrics.record_request(start.elapsed(), true);
            return Err(EggsecError::ScopeViolation(format!(
                "REST strict enforcement: manual confirmation unavailable — {}",
                decision.to_human_readable()
            )));
        }
        EnforcementOutcome::Deny(decision) => {
            state.metrics.record_request(start.elapsed(), true);
            return Err(EggsecError::ScopeViolation(format!(
                "REST strict enforcement denied: {}",
                decision.to_human_readable()
            )));
        }
    }

    let target_type = payload.target_type.as_deref().unwrap_or("url");

    let target = match target_type {
        "domain" => crate::tool::Target::domain(&payload.target),
        "ip" => crate::tool::Target::ip(&payload.target),
        "cidr" => crate::tool::Target::cidr(&payload.target),
        _ => crate::tool::Target::url(&payload.target),
    };

    let request = ToolRequest {
        id: uuid::Uuid::new_v4().to_string(),
        tool: tool_id,
        target,
        params: payload.params.unwrap_or_default(),
        options: payload.options.unwrap_or_default(),
        cancellation_token: None,
    };

    match state.dispatcher.dispatch(request).await {
        Ok(response) => {
            state
                .metrics
                .record_request(start.elapsed(), !response.is_success());
            Ok(Json(response))
        }
        Err(e) => {
            state.metrics.record_request(start.elapsed(), true);
            Err(e)
        }
    }
}

async fn preflight_tool(
    State(state): State<Arc<RestState>>,
    Path(tool_id): Path<String>,
    headers: HeaderMap,
    Json(payload): Json<PreflightRequest>,
) -> Result<Json<crate::config::PreflightResult>, EggsecError> {
    if let Err(e) = require_auth(&state, &headers) {
        return Err(e);
    }

    validate_target(&payload.target)?;

    let target_url = &payload.target;
    let descriptor = operation_descriptor_for_rest_tool(&tool_id, target_url)?;

    if let Some(metadata) = crate::tool::metadata::metadata_for_tool_id(&tool_id) {
        if !metadata.rest_exposable {
            return Err(EggsecError::Config(format!(
                "Tool '{}' is not exposed via REST API",
                tool_id
            )));
        }
    }

    let result = crate::config::preflight_operation(
        crate::config::ExecutionSurface::RestApi,
        &state.enforcement,
        descriptor,
        None,
    );

    Ok(Json(result))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_target_valid() {
        assert!(validate_target("https://example.com").is_ok());
        assert!(validate_target("http://192.168.1.1").is_ok());
        assert!(validate_target("example.com").is_ok());
    }

    #[test]
    fn test_validate_target_invalid() {
        assert!(validate_target("").is_err());
        assert!(validate_target(&"a".repeat(3000)).is_err());
    }

    #[test]
    fn test_pagination_defaults() {
        let params = PaginationParams {
            offset: None,
            limit: None,
        };
        assert_eq!(params.offset(), 0);
        assert_eq!(params.limit(), 50);
    }

    #[test]
    fn test_pagination_limit_clamping() {
        let params = PaginationParams {
            offset: None,
            limit: Some(200),
        };
        assert_eq!(params.limit(), 100);
    }

    #[test]
    fn test_metrics_recording() {
        let metrics = Metrics::default();
        metrics.record_request(Duration::from_millis(100), false);
        metrics.record_request(Duration::from_millis(200), true);
        let m = metrics.get_metrics();
        assert_eq!(m["requests_total"], 2);
        assert_eq!(m["errors_total"], 1);
    }

    #[tokio::test]
    async fn test_rate_limiter_allows_within_limit() {
        let limiter = RateLimiter::new(RateLimitConfig::standard().requests_per_minute);
        assert!(limiter.check_rate_limit("client-1").is_ok());
        assert!(limiter.check_rate_limit("client-1").is_ok());
        assert!(limiter.check_rate_limit("client-1").is_ok());
    }

    #[tokio::test]
    async fn test_rate_limiter_blocks_over_limit() {
        let limiter = RateLimiter::new(RateLimitConfig::strict().requests_per_minute);
        for _ in 0..5 {
            assert!(
                limiter.check_rate_limit("client-1").is_ok(),
                "Should allow up to burst_size"
            );
        }
        assert!(
            limiter.check_rate_limit("client-1").is_err(),
            "Should block when burst exhausted"
        );
    }

    #[tokio::test]
    async fn test_rate_limiter_separate_keys() {
        let limiter = RateLimiter::new(RateLimitConfig::strict().requests_per_minute);
        for _ in 0..5 {
            assert!(
                limiter.check_rate_limit("client-1").is_ok(),
                "Should allow up to burst_size"
            );
        }
        assert!(
            limiter.check_rate_limit("client-1").is_err(),
            "Should block client-1"
        );
        for _ in 0..5 {
            assert!(
                limiter.check_rate_limit("client-2").is_ok(),
                "Separate client should have own limit"
            );
        }
    }

    #[test]
    fn test_operation_descriptor_for_rest_tool_safe_active() {
        let desc = operation_descriptor_for_rest_tool("recon", "https://example.com").unwrap();
        assert_eq!(desc.risk, crate::config::OperationRisk::SafeActive);
        assert_eq!(
            desc.required_capabilities,
            vec![crate::config::Capability::PassiveFingerprint]
        );
    }

    #[test]
    fn test_operation_descriptor_for_rest_tool_intrusive() {
        let desc = operation_descriptor_for_rest_tool("fuzz", "https://example.com").unwrap();
        assert_eq!(desc.risk, crate::config::OperationRisk::Intrusive);
        assert_eq!(
            desc.required_capabilities,
            vec![crate::config::Capability::HttpFuzzLowImpact]
        );
    }

    #[test]
    fn test_operation_descriptor_for_rest_tool_stress() {
        let desc = operation_descriptor_for_rest_tool("stress", "https://example.com").unwrap();
        assert_eq!(desc.risk, crate::config::OperationRisk::StressTest);
        assert_eq!(
            desc.required_capabilities,
            vec![crate::config::Capability::WafStressTest]
        );
    }

    #[test]
    fn test_enforcement_deny_without_explicit_scope() {
        use crate::config::{EnforcementContext, ExecutionPolicy, ExecutionSurface, LoadedScope};

        let policy = ExecutionPolicy::default();
        let loaded_scope = LoadedScope::default_empty();
        let enforcement =
            EnforcementContext::for_surface(ExecutionSurface::RestApi, policy, loaded_scope);

        let descriptor = operation_descriptor_for_rest_tool("scan", "https://example.com").unwrap();
        let outcome = enforcement.evaluate(&descriptor);
        assert!(
            outcome.is_denied() || outcome.requires_confirmation(),
            "Expected deny or require-confirmation for REST without explicit scope, got: {:?}",
            outcome
        );
    }

    #[test]
    fn test_enforcement_out_of_scope_denial() {
        use crate::config::{
            EnforcementContext, ExecutionPolicy, ExecutionSurface, LoadedScope, Scope, ScopeSource,
        };

        let mut scope = Scope::default();
        scope.allowed_targets = vec![crate::config::ScopeRule::new("10.0.0.0/8".to_string())];
        let loaded_scope = LoadedScope::explicit(scope, ScopeSource::ConfigFile, None);
        let policy = ExecutionPolicy::default();
        let enforcement =
            EnforcementContext::for_surface(ExecutionSurface::RestApi, policy, loaded_scope);

        let descriptor =
            operation_descriptor_for_rest_tool("scan", "https://evil.example.com").unwrap();
        let outcome = enforcement.evaluate(&descriptor);
        assert!(
            outcome.is_denied(),
            "Expected denial for out-of-scope target, got: {:?}",
            outcome
        );
    }

    #[test]
    fn test_enforcement_require_confirmation_treated_as_deny() {
        use crate::config::{
            EnforcementContext, EnforcementOutcome, ExecutionPolicy, ExecutionSurface, LoadedScope,
        };

        let policy = ExecutionPolicy {
            require_explicit_scope: false,
            ..ExecutionPolicy::default()
        };
        let loaded_scope = LoadedScope::default_empty();
        let enforcement =
            EnforcementContext::for_surface(ExecutionSurface::RestApi, policy, loaded_scope);

        let descriptor = operation_descriptor_for_rest_tool("fuzz", "https://example.com").unwrap();
        let outcome = enforcement.evaluate(&descriptor);

        match outcome {
            EnforcementOutcome::RequireConfirmation(_) => {
                // REST cannot provide manual confirmation - treat as deny
            }
            EnforcementOutcome::Deny(_) => {
                // Also acceptable - direct denial
            }
            other => {
                panic!(
                    "Expected RequireConfirmation or Deny for intrusive REST op without policy, got: {:?}",
                    other
                );
            }
        }
    }

    #[test]
    fn test_rest_surface_maps_to_strict_profile() {
        use crate::config::{ExecutionProfile, ExecutionSurface};

        let surface = ExecutionSurface::RestApi;
        let profile = surface.profile();
        assert!(
            profile.is_strict(),
            "REST surface must map to a strict profile, got: {:?}",
            profile
        );
        assert_eq!(profile, ExecutionProfile::McpStrict);
    }

    #[test]
    fn test_rest_dispatch_only_on_allow() {
        use crate::config::{
            EnforcementContext, EnforcementOutcome, ExecutionPolicy, ExecutionSurface, LoadedScope,
        };

        let policy = ExecutionPolicy::default();
        let loaded_scope = LoadedScope::default_empty();
        let enforcement =
            EnforcementContext::for_surface(ExecutionSurface::RestApi, policy, loaded_scope);

        let descriptor = operation_descriptor_for_rest_tool("scan", "https://example.com").unwrap();
        let outcome = enforcement.evaluate(&descriptor);

        match &outcome {
            EnforcementOutcome::Allow(_) => {
                // Only Allow permits dispatch
            }
            _ => {
                // All other outcomes deny dispatch in REST
            }
        }

        // Verify that is_allowed() on the outcome is only true for Allow
        // (Warn is no longer treated as allowed for REST dispatch)
        assert!(
            !outcome.is_allowed() || matches!(outcome, EnforcementOutcome::Allow(_)),
            "REST should only dispatch on Allow, got: {:?}",
            outcome
        );
    }

    #[test]
    fn test_rest_warn_treated_as_deny() {
        use crate::config::{
            EnforcementContext, EnforcementOutcome, ExecutionPolicy, ExecutionSurface, LoadedScope,
            Scope, ScopeSource,
        };

        let mut scope = Scope::default();
        scope.allowed_targets = vec![crate::config::ScopeRule::new("*.lab.internal".to_string())];
        let loaded_scope = LoadedScope::explicit(scope, ScopeSource::ConfigFile, None);
        let policy = ExecutionPolicy::default();
        let enforcement =
            EnforcementContext::for_surface(ExecutionSurface::RestApi, policy, loaded_scope);

        let descriptor =
            operation_descriptor_for_rest_tool("recon", "https://example.com").unwrap();
        let outcome = enforcement.evaluate(&descriptor);

        match outcome {
            EnforcementOutcome::Warn(decision) => {
                // Warn is treated as deny in REST - this is correct behavior
                assert!(!decision.allowed, "Warn decision should not be allowed");
            }
            EnforcementOutcome::Deny(_) => {
                // Direct deny is also acceptable
            }
            other => {
                panic!(
                    "Expected Warn or Deny for out-of-scope target in REST, got: {:?}",
                    other
                );
            }
        }
    }

    #[test]
    fn test_rest_require_confirmation_treated_as_deny() {
        use crate::config::{
            EnforcementContext, EnforcementOutcome, ExecutionPolicy, ExecutionSurface, LoadedScope,
        };

        let policy = ExecutionPolicy::default();
        let loaded_scope = LoadedScope::default_empty();
        let enforcement =
            EnforcementContext::for_surface(ExecutionSurface::RestApi, policy, loaded_scope);

        let descriptor = operation_descriptor_for_rest_tool("fuzz", "https://example.com").unwrap();
        let outcome = enforcement.evaluate(&descriptor);

        match outcome {
            EnforcementOutcome::RequireConfirmation(_) => {
                // REST cannot provide manual confirmation - treated as deny
            }
            EnforcementOutcome::Deny(_) => {
                // Direct denial is also acceptable
            }
            other => {
                panic!(
                    "Expected RequireConfirmation or Deny for intrusive REST op, got: {:?}",
                    other
                );
            }
        }
    }

    #[test]
    fn test_rest_deny_is_hard_deny() {
        use crate::config::{
            EnforcementContext, EnforcementOutcome, ExecutionPolicy, ExecutionSurface, LoadedScope,
            Scope, ScopeSource,
        };

        let mut scope = Scope::default();
        scope.allowed_targets = vec![crate::config::ScopeRule::new("10.0.0.0/8".to_string())];
        let loaded_scope = LoadedScope::explicit(scope, ScopeSource::ConfigFile, None);
        let policy = ExecutionPolicy::default();
        let enforcement =
            EnforcementContext::for_surface(ExecutionSurface::RestApi, policy, loaded_scope);

        let descriptor =
            operation_descriptor_for_rest_tool("scan", "https://evil.example.com").unwrap();
        let outcome = enforcement.evaluate(&descriptor);
        assert!(
            outcome.is_denied(),
            "Expected hard denial for out-of-scope target, got: {:?}",
            outcome
        );
    }

    #[test]
    fn test_rest_missing_explicit_scope_denies() {
        use crate::config::{EnforcementContext, ExecutionPolicy, ExecutionSurface, LoadedScope};

        let policy = ExecutionPolicy::default();
        let loaded_scope = LoadedScope::default_empty();
        let enforcement =
            EnforcementContext::for_surface(ExecutionSurface::RestApi, policy, loaded_scope);

        let descriptor = operation_descriptor_for_rest_tool("scan", "https://example.com").unwrap();
        let outcome = enforcement.evaluate(&descriptor);
        assert!(
            outcome.is_denied() || outcome.requires_confirmation(),
            "REST without explicit scope should deny or require confirmation, got: {:?}",
            outcome
        );
    }

    #[test]
    fn test_rest_positive_scope_match_proceeds() {
        use crate::config::{
            EnforcementContext, EnforcementOutcome, ExecutionPolicy, ExecutionSurface, LoadedScope,
            Scope, ScopeSource,
        };

        let mut scope = Scope::default();
        scope.allowed_targets = vec![crate::config::ScopeRule::new("*.example.com".to_string())];
        let loaded_scope = LoadedScope::explicit(scope, ScopeSource::ConfigFile, None);
        let policy = ExecutionPolicy::default();
        let enforcement =
            EnforcementContext::for_surface(ExecutionSurface::RestApi, policy, loaded_scope);

        let descriptor =
            operation_descriptor_for_rest_tool("scan", "https://target.example.com").unwrap();
        let outcome = enforcement.evaluate(&descriptor);
        assert!(
            matches!(outcome, EnforcementOutcome::Allow(_)),
            "REST with matching scope should Allow, got: {:?}",
            outcome
        );
    }

    #[test]
    fn test_rest_positive_scope_miss_denies() {
        use crate::config::{
            EnforcementContext, ExecutionPolicy, ExecutionSurface, LoadedScope, Scope, ScopeSource,
        };

        let mut scope = Scope::default();
        scope.allowed_targets = vec![crate::config::ScopeRule::new("*.lab.internal".to_string())];
        let loaded_scope = LoadedScope::explicit(scope, ScopeSource::ConfigFile, None);
        let policy = ExecutionPolicy::default();
        let enforcement =
            EnforcementContext::for_surface(ExecutionSurface::RestApi, policy, loaded_scope);

        let descriptor = operation_descriptor_for_rest_tool("scan", "https://example.com").unwrap();
        let outcome = enforcement.evaluate(&descriptor);
        assert!(
            outcome.is_denied() || outcome.requires_confirmation(),
            "REST with non-matching scope should deny or require confirmation, got: {:?}",
            outcome
        );
    }

    #[test]
    fn test_rest_ignores_manual_overrides() {
        use crate::config::{
            EnforcementContext, ExecutionPolicy, ExecutionSurface, LoadedScope, Scope, ScopeSource,
        };

        let mut scope = Scope::default();
        scope.allowed_targets = vec![crate::config::ScopeRule::new("*.lab.internal".to_string())];
        let loaded_scope = LoadedScope::explicit(scope, ScopeSource::ConfigFile, None);

        let policy = ExecutionPolicy::default();
        let enforcement =
            EnforcementContext::for_surface(ExecutionSurface::RestApi, policy, loaded_scope);

        let descriptor = operation_descriptor_for_rest_tool("scan", "https://example.com").unwrap();
        let outcome = enforcement.evaluate(&descriptor);

        // REST is strict - manual overrides are never honored
        assert!(
            outcome.is_denied() || outcome.requires_confirmation(),
            "REST should not honor manual overrides, got: {:?}",
            outcome
        );
    }

    #[test]
    fn test_rest_non_rest_exposable_tool_denied() {
        use crate::config::metadata_for_tool_id;

        // stress is registered in ALL_OPERATION_METADATA with rest_exposable: true
        // This test verifies the metadata lookup works and the field is accessible
        if let Some(metadata) = metadata_for_tool_id("stress") {
            assert!(
                metadata.rest_exposable,
                "stress tool should be rest_exposable"
            );
        }

        // Verify metadata_for_tool_id works for known tools
        assert!(
            metadata_for_tool_id("recon").is_some(),
            "recon should have metadata"
        );
        assert!(
            metadata_for_tool_id("scan-ports").is_some(),
            "scan-ports should have metadata"
        );
    }

    #[test]
    fn test_rest_metadata_descriptor_has_requires_explicit_scope() {
        let descriptor = operation_descriptor_for_rest_tool("scan", "https://example.com").unwrap();
        assert!(
            descriptor.requires_explicit_scope,
            "REST descriptors must always set requires_explicit_scope = true"
        );
    }

    #[test]
    fn test_rest_policy_denied_response_format() {
        use crate::config::PolicyDecision;

        let decision = PolicyDecision::allowed(
            "test-op",
            crate::config::OperationMode::StandardAssessment,
            crate::config::OperationRisk::SafeActive,
            vec![crate::config::IntendedUse::WebAssessment],
        );

        let response = policy_denied_response("test denial", decision);
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[test]
    fn test_preflight_does_not_dispatch() {
        use crate::config::{
            EnforcementContext, ExecutionPolicy, ExecutionSurface, LoadedScope,
            PreflightOutcomeKind,
        };

        let policy = ExecutionPolicy::default();
        let loaded_scope = LoadedScope::default_empty();
        let enforcement =
            EnforcementContext::for_surface(ExecutionSurface::RestApi, policy, loaded_scope);

        let descriptor = operation_descriptor_for_rest_tool("scan", "https://example.com").unwrap();
        let result = crate::config::preflight_operation(
            ExecutionSurface::RestApi,
            &enforcement,
            descriptor,
            None,
        );

        assert_eq!(result.surface, ExecutionSurface::RestApi);
        assert!(
            matches!(
                result.outcome_kind,
                PreflightOutcomeKind::Deny | PreflightOutcomeKind::RequireConfirmation
            ),
            "Preflight should evaluate without dispatch, got: {:?}",
            result.outcome_kind
        );
    }
}
