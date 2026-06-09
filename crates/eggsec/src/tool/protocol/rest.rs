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
use subtle::ConstantTimeEq;
use tower_http::cors::{AllowHeaders, AllowMethods, CorsLayer};

use crate::config::Scope;
use crate::distributed::TlsConfig;
use crate::error::EggsecError;
use crate::tool::ratelimit::{RateLimitConfig, RateLimiter};
use crate::tool::{ToolDispatcher, ToolRegistry, ToolRequest, ToolResponse};

const MAX_PAYLOAD_SIZE: usize = 10 * 1024 * 1024;
const MAX_URL_LENGTH: usize = 2048;

#[derive(Clone)]
pub struct RestState {
    pub registry: ToolRegistry,
    pub dispatcher: ToolDispatcher,
    pub api_key: Option<String>,
    pub rate_limiter: RateLimiter,
    pub scope: Option<Scope>,
    pub tls_config: Option<TlsConfig>,
    pub metrics: Arc<Metrics>,
}

impl RestState {
    pub fn new(
        registry: ToolRegistry,
        api_key: Option<String>,
        scope: Option<Scope>,
        tls_config: Option<TlsConfig>,
    ) -> Self {
        let dispatcher = ToolDispatcher::new(registry.clone());
        let rate_limiter = RateLimiter::new(RateLimitConfig::standard());
        Self {
            registry,
            dispatcher,
            api_key,
            rate_limiter,
            scope,
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
    scope: Option<Scope>,
    tls_config: Option<TlsConfig>,
) -> Router {
    let state = Arc::new(RestState::new(
        registry,
        api_key.clone(),
        scope,
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
                    if let Err(e) = tx.send(text.to_string()).await {
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
    if let Some(ref key) = state.api_key {
        let auth = headers
            .get("authorization")
            .or_else(|| headers.get("x-api-key"))
            .and_then(|v| v.to_str().ok());

        match auth {
            Some(v) if bool::from(key.as_bytes().ct_eq(v.as_bytes())) => Ok(()),
            _ => Err(EggsecError::Config(
                "Invalid or missing API key".to_string(),
            )),
        }
    } else {
        Ok(())
    }
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
        && !target.starts_with("http%3A")
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
    if state.api_key.is_some() {
        if let Err(e) = require_auth(&state, &headers) {
            return Err(EggsecError::Config(e.to_string()));
        }
    }

    let start = Instant::now();
    state.metrics.record_request(start.elapsed(), false);

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
    if state.api_key.is_some() {
        if let Err(e) = require_auth(&state, &headers) {
            return Err(EggsecError::Config(e.to_string()));
        }
    }

    let start = Instant::now();
    let metrics = state.metrics.get_metrics();
    state.metrics.record_request(start.elapsed(), false);

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
                        "200": {"description": "Execution result"}
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

    let tools = state.registry.list();
    let info = tools.into_iter().find(|t| t.id == tool_id).ok_or_else(|| {
        state.metrics.record_request(start.elapsed(), true);
        EggsecError::Config(format!("Tool '{}' not found", tool_id))
    })?;

    state.metrics.record_request(start.elapsed(), false);

    Ok(Json(ToolDetailResponse {
        id: info.id,
        name: info.name,
        category: info.category.to_string(),
        description: info.description,
        capabilities: info
            .capabilities
            .iter()
            .map(|c| serde_json::to_value(c).unwrap_or_default())
            .collect(),
        protocols: info.protocols,
    }))
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
    if let Some(ref scope) = state.scope {
        match scope.is_target_allowed(target_url) {
            Ok(false) | Err(_) => {
                state.metrics.record_request(start.elapsed(), true);
                return Err(EggsecError::ScopeViolation(target_url.clone()));
            }
            Ok(true) => {}
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
        let limiter = RateLimiter::new(RateLimitConfig::standard());
        assert!(limiter.check_rate_limit("client-1").is_ok());
        assert!(limiter.check_rate_limit("client-1").is_ok());
        assert!(limiter.check_rate_limit("client-1").is_ok());
    }

    #[tokio::test]
    async fn test_rate_limiter_blocks_over_limit() {
        let limiter = RateLimiter::new(RateLimitConfig::strict());
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
        let limiter = RateLimiter::new(RateLimitConfig::strict());
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
}
