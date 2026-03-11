use axum::{
    routing::{get, post},
    Router, extract::{Path, State, Json},
    response::IntoResponse,
    http::{StatusCode, HeaderMap},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use std::collections::HashMap;
use tokio::sync::RwLock;

use crate::error::SlapperError;
use crate::tool::{ToolRegistry, ToolDispatcher, ToolRequest, ToolResponse};

#[derive(Clone)]
pub struct RestState {
    pub registry: ToolRegistry,
    pub dispatcher: ToolDispatcher,
    pub api_key: Option<String>,
    pub rate_limiter: RateLimiter,
}

pub struct RateLimiter {
    requests: RwLock<HashMap<String, Vec<std::time::Instant>>>,
    max_requests: u64,
    window_secs: u64,
}

impl Clone for RateLimiter {
    fn clone(&self) -> Self {
        Self {
            requests: RwLock::new(HashMap::new()),
            max_requests: self.max_requests,
            window_secs: self.window_secs,
        }
    }
}

impl RateLimiter {
    pub fn new(max_requests: u64, window_secs: u64) -> Self {
        Self {
            requests: RwLock::new(HashMap::new()),
            max_requests,
            window_secs,
        }
    }

    pub async fn check(&self, key: &str) -> bool {
        let now = std::time::Instant::now();
        let mut requests = self.requests.write().await;
        
        let timestamps = requests.entry(key.to_string()).or_insert_with(Vec::new);
        timestamps.retain(|t| now.duration_since(*t) < Duration::from_secs(self.window_secs));
        
        if timestamps.len() >= self.max_requests as usize {
            return false;
        }
        
        timestamps.push(now);
        true
    }

    pub fn blocking_check(&self, key: &str) -> bool {
        let now = std::time::Instant::now();
        
        if let Ok(mut requests) = self.requests.try_write() {
            let timestamps = requests.entry(key.to_string()).or_insert_with(Vec::new);
            timestamps.retain(|t| now.duration_since(*t) < Duration::from_secs(self.window_secs));
            
            if timestamps.len() >= self.max_requests as usize {
                return false;
            }
            timestamps.push(now);
            true
        } else {
            true
        }
    }
}

impl RestState {
    pub fn new(registry: ToolRegistry, api_key: Option<String>) -> Self {
        let dispatcher = ToolDispatcher::new(registry.clone());
        let rate_limiter = RateLimiter::new(100, 60);
        Self { registry, dispatcher, api_key, rate_limiter }
    }
}

#[derive(Debug, Serialize)]
pub struct RestErrorResponse {
    pub error: String,
    pub code: String,
}

impl IntoResponse for SlapperError {
    fn into_response(self) -> axum::response::Response {
        let (status, error_response) = match &self {
            SlapperError::Config(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            SlapperError::InvalidTarget(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            SlapperError::Network(msg) => (StatusCode::SERVICE_UNAVAILABLE, msg.clone()),
            SlapperError::Timeout { .. } => (StatusCode::REQUEST_TIMEOUT, self.to_string()),
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

pub fn create_router(registry: ToolRegistry, api_key: Option<String>) -> Router {
    let state = Arc::new(RestState::new(registry, api_key));

    Router::new()
        .route("/health", get(health_check))
        .route("/openapi.json", get(openapi_spec))
        .route("/api/v1/tools", get(list_tools))
        .route("/api/v1/tools/:tool_id", get(get_tool))
        .route("/api/v1/tools/:tool_id/execute", post(execute_tool))
        .with_state(state)
}

fn check_rate_limit(state: &Arc<RestState>, client_id: &str) -> Result<(), SlapperError> {
    if !state.rate_limiter.blocking_check(client_id) {
        return Err(SlapperError::Config("Rate limit exceeded".to_string()));
    }
    Ok(())
}

fn require_auth(state: &Arc<RestState>, headers: &HeaderMap) -> Result<(), SlapperError> {
    if let Some(ref key) = state.api_key {
        let auth = headers.get("authorization")
            .or_else(|| headers.get("x-api-key"))
            .and_then(|v| v.to_str().ok());
        
        match auth {
            Some(v) if v == key => Ok(()),
            _ => Err(SlapperError::Config("Invalid or missing API key".to_string())),
        }
    } else {
        Ok(())
    }
}

pub fn generate_correlation_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "slapper-tool-api"
    }))
}

async fn openapi_spec() -> impl IntoResponse {
    Json(serde_json::json!({
        "openapi": "3.0.0",
        "info": {
            "title": "Slapper Tool API",
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
) -> Result<Json<ToolListResponse>, SlapperError> {
    if let Err(e) = require_auth(&state, &headers) {
        return Err(SlapperError::Config(e.to_string()));
    }
    
    let tools = state.registry.list();
    
    let items: Vec<ToolListItem> = tools.iter().map(|t| ToolListItem {
        id: t.id.to_string(),
        name: t.name.to_string(),
        category: t.category.to_string(),
        description: t.description.to_string(),
        protocols: t.protocols.clone(),
    }).collect();

    let categories: Vec<String> = state.registry.categories()
        .iter()
        .map(|c| c.to_string())
        .collect();

    Ok(Json(ToolListResponse { tools: items, categories }))
}

async fn get_tool(
    State(state): State<Arc<RestState>>,
    Path(tool_id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<ToolDetailResponse>, SlapperError> {
    if let Err(e) = require_auth(&state, &headers) {
        return Err(SlapperError::Config(e.to_string()));
    }
    
    let tools = state.registry.list();
    let info = tools.into_iter()
        .find(|t| t.id == tool_id)
        .ok_or_else(|| SlapperError::Config(format!("Tool '{}' not found", tool_id)))?;

    Ok(Json(ToolDetailResponse {
        id: info.id,
        name: info.name,
        category: info.category.to_string(),
        description: info.description,
        capabilities: info.capabilities.iter().map(|c| serde_json::to_value(c).unwrap_or_default()).collect(),
        protocols: info.protocols,
    }))
}

async fn execute_tool(
    State(state): State<Arc<RestState>>,
    Path(tool_id): Path<String>,
    headers: HeaderMap,
    Json(payload): Json<ExecuteRequest>,
) -> Result<Json<ToolResponse>, SlapperError> {
    if let Err(e) = require_auth(&state, &headers) {
        return Err(SlapperError::Config(e.to_string()));
    }
    
    let client_id = payload.target.clone();
    if !state.rate_limiter.blocking_check(&client_id) {
        return Err(SlapperError::Config("Rate limit exceeded for this target".to_string()));
    }

    let target_type = payload.target_type.as_deref()
        .or(Some("url"))
        .unwrap();

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
    };

    let response = state.dispatcher.dispatch(request).await?;

    Ok(Json(response))
}
