use std::sync::Arc;
use std::time::Duration;

use async_stream::stream;
use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
    response::{sse::Event as SseEvent, IntoResponse, Sse},
    routing::{get, post},
    Router,
};
use futures::Stream;
use serde::Deserialize;
use tokio::io::AsyncWriteExt;

use crate::tool::{ChainPlanner, ExecutionPlan, OpenApiGenerator, PlanRequest, ToolRegistry};

use super::handlers::McpServer;
use super::profile::McpProfile;
use super::streaming::StreamEvent;
use super::types::{McpError, McpRequest, McpResponse};

#[derive(Deserialize)]
#[serde(untagged)]
enum McpIncoming {
    Single(McpRequest),
    Batch(Vec<McpRequest>),
}

impl McpIncoming {
    fn into_vec(self) -> Vec<McpRequest> {
        match self {
            McpIncoming::Single(req) => vec![req],
            McpIncoming::Batch(reqs) => reqs,
        }
    }
}

fn validate_batch_size(requests: &[McpRequest], max: usize) -> Result<(), McpResponse> {
    if requests.len() > max {
        return Err(McpResponse {
            jsonrpc: "2.0".to_string(),
            id: None,
            result: None,
            error: Some(McpError::invalid_request(&format!(
                "Batch size exceeds limit of {}",
                max
            ))),
        });
    }
    Ok(())
}

async fn process_batch(
    server: &McpServer,
    requests: Vec<McpRequest>,
    max_batch_size: usize,
) -> Vec<McpResponse> {
    if let Err(error_response) = validate_batch_size(&requests, max_batch_size) {
        return vec![error_response];
    }

    let mut responses = Vec::with_capacity(requests.len());
    for req in requests {
        if let Err(e) = server.validate_auth_params(&req.params) {
            responses.push(req.error_response(e));
            continue;
        }
        let response = match tokio::time::timeout(
            Duration::from_secs(30),
            server.handle_request(req),
        )
        .await
        {
            Ok(response) => response,
            Err(e) => {
                tracing::warn!(error = %e, "MCP request handler timed out after 30s");
                McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id: None,
                    result: None,
                    error: Some(McpError::internal(&format!(
                        "MCP request handler timed out: {}",
                        e
                    ))),
                }
            }
        };
        responses.push(response);
    }
    responses
}

struct AppState {
    mcp_server: Arc<McpServer>,
    planner: ChainPlanner,
    openapi_generator: OpenApiGenerator,
}

async fn handle_openapi_json(State(state): State<Arc<AppState>>) -> axum::Json<serde_json::Value> {
    let spec = state.openapi_generator.generate(&state.mcp_server.registry);
    axum::Json(
        serde_json::from_str(&spec.to_json())
            .inspect_err(|e| {
                tracing::warn!(error = %e, "Failed to parse OpenAPI JSON spec");
            })
            .unwrap_or_default(),
    )
}

async fn handle_openapi_yaml(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let spec = state.openapi_generator.generate(&state.mcp_server.registry);
    ([("Content-Type", "application/x-yaml")], spec.to_yaml())
}

async fn handle_create_plan(
    State(state): State<Arc<AppState>>,
    Json(request): Json<PlanRequest>,
) -> axum::Json<ExecutionPlan> {
    let plan = state.planner.plan(&request);
    let validation = state.planner.validate_plan(&plan);

    if !validation.valid {
        tracing::warn!("Plan validation failed: {:?}", validation.errors);
    }

    if !validation.warnings.is_empty() {
        tracing::info!("Plan warnings: {:?}", validation.warnings);
    }

    axum::Json(plan)
}

pub async fn create_mcp_router(
    registry: ToolRegistry,
    api_key: Option<String>,
    profile: McpProfile,
    enforcement: crate::config::EnforcementContext,
) -> Router {
    // Use the production constructor that directly accepts EnforcementContext (no build-then-patch).
    let server = Arc::new(McpServer::with_enforcement(
        registry.clone(),
        api_key,
        profile,
        enforcement,
    ));
    let planner = ChainPlanner::new(registry.clone());
    let openapi_generator = OpenApiGenerator::new("http://localhost:8080", "0.1.0");

    let app_state = Arc::new(AppState {
        mcp_server: server,
        planner,
        openapi_generator,
    });

    Router::new()
        .route("/mcp", post(handle_mcp))
        .route("/json-rpc", post(handle_mcp))
        .route("/mcp/stream/{request_id}", get(handle_sse_stream))
        .route("/health", get(handle_health))
        .route("/openapi.json", get(handle_openapi_json))
        .route("/openapi.yaml", get(handle_openapi_yaml))
        .route("/plan", post(handle_create_plan))
        .with_state(app_state)
}

async fn handle_health() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "status": "healthy",
        "service": "eggsec-mcp",
        "version": "0.1.0"
    }))
}

struct SseStreamState {
    receiver: tokio::sync::broadcast::Receiver<StreamEvent>,
    request_id: String,
}

async fn handle_sse_stream(
    State(state): State<Arc<AppState>>,
    Path(request_id): Path<String>,
) -> Sse<impl Stream<Item = Result<SseEvent, axum::Error>>> {
    let receiver = state.mcp_server.subscribe_to_stream();

    let stream = stream! {
        let mut receiver = receiver;
        let mut tick_interval = tokio::time::interval(Duration::from_secs(30));

        loop {
            tokio::select! {
                event = receiver.recv() => {
                    match event {
                        Ok(event) => {
                            if event.request_id == request_id || event.request_id == "*" {
                                yield Ok::<_, axum::Error>(SseEvent::default()
                                    .event(&event.event_type)
                                    .data(serde_json::to_string(&event.data).inspect_err(|e| {
                                        tracing::warn!(error = %e, "Failed to serialize SSE event data");
                                    }).unwrap_or_default()));
                            }
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                            yield Ok::<_, axum::Error>(SseEvent::default()
                                .event("lagged")
                                .data(format!("{{\"lagged_events\": {}}}", n)));
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                            break;
                        }
                    }
                }
                _ = tick_interval.tick() => {
                    yield Ok::<_, axum::Error>(SseEvent::default()
                        .event("heartbeat")
                        .data("{\"timestamp\": \"alive\"}"));
                }
            }
        }
    };

    Sse::new(stream)
        .keep_alive(axum::response::sse::KeepAlive::new().interval(Duration::from_secs(15)))
}

async fn handle_mcp(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(incoming): Json<McpIncoming>,
) -> (StatusCode, Json<Vec<McpResponse>>) {
    let requests = incoming.into_vec();
    let max_batch_size = state.mcp_server.policy.max_batch_size;

    if let Err(error_response) = validate_batch_size(&requests, max_batch_size) {
        return (StatusCode::BAD_REQUEST, Json(vec![error_response]));
    }

    let mut responses = Vec::with_capacity(requests.len());
    for req in requests {
        if let Err(e) = state.mcp_server.validate_auth(&headers) {
            responses.push(req.error_response(e));
            continue;
        }
        let response = match tokio::time::timeout(
            Duration::from_secs(30),
            state.mcp_server.handle_request(req),
        )
        .await
        {
            Ok(response) => response,
            Err(e) => {
                tracing::warn!(error = %e, "MCP request handler timed out after 30s");
                McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id: None,
                    result: None,
                    error: Some(McpError::internal(&format!(
                        "MCP request handler timed out: {}",
                        e
                    ))),
                }
            }
        };
        responses.push(response);
    }

    (StatusCode::OK, Json(responses))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_incoming_single_object() {
        let single = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "ping"
        });
        let incoming: McpIncoming = serde_json::from_value(single).unwrap();
        let vec = incoming.into_vec();
        assert_eq!(vec.len(), 1);
        assert_eq!(vec[0].method, "ping");
    }

    #[test]
    fn test_mcp_incoming_batch_array() {
        let batch = serde_json::json!([
            {"jsonrpc": "2.0", "id": 1, "method": "ping"},
            {"jsonrpc": "2.0", "id": 2, "method": "initialize"}
        ]);
        let incoming: McpIncoming = serde_json::from_value(batch).unwrap();
        let vec = incoming.into_vec();
        assert_eq!(vec.len(), 2);
        assert_eq!(vec[0].method, "ping");
        assert_eq!(vec[1].method, "initialize");
    }

    #[test]
    fn test_mcp_incoming_batch_empty() {
        let batch = serde_json::json!([]);
        let incoming: McpIncoming = serde_json::from_value(batch).unwrap();
        let vec = incoming.into_vec();
        assert!(vec.is_empty());
    }

    #[test]
    fn test_mcp_incoming_invalid_json() {
        let invalid = serde_json::json!("not a request");
        let result = serde_json::from_value::<McpIncoming>(invalid);
        assert!(result.is_err());
    }
}

async fn write_json_line(
    writer: &mut tokio::io::BufWriter<tokio::io::Stdout>,
    value: &impl serde::Serialize,
) {
    if let Ok(json) = serde_json::to_string(value) {
        if let Err(e) = writer.write_all(json.as_bytes()).await {
            tracing::warn!(error = %e, "Failed to write response");
        }
        if let Err(e) = writer.write_all(b"\n").await {
            tracing::warn!(error = %e, "Failed to write newline");
        }
        if let Err(e) = writer.flush().await {
            tracing::warn!(error = %e, "Failed to flush writer");
        }
    }
}

pub async fn run_stdio(
    registry: ToolRegistry,
    api_key: Option<String>,
    profile: McpProfile,
    enforcement: crate::config::EnforcementContext,
) {
    use tokio::io::{AsyncBufReadExt, BufReader, BufWriter};

    let server = Arc::new(McpServer::with_enforcement(
        registry,
        api_key,
        profile,
        enforcement,
    ));

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let mut reader: tokio::io::Lines<BufReader<tokio::io::Stdin>> = BufReader::new(stdin).lines();
    let mut writer = BufWriter::new(stdout);

    tracing::info!("MCP stdio server started, waiting for requests...");

    while let Ok(Some(line)) = reader.next_line().await {
        if line.trim().is_empty() {
            continue;
        }

        let incoming: Result<McpIncoming, _> = serde_json::from_str(&line);

        match incoming {
            Ok(incoming) => {
                let requests = incoming.into_vec();
                let responses = process_batch(&server, requests, server.policy.max_batch_size).await;
                write_json_line(&mut writer, &responses).await;
            }
            Err(e) => {
                let error = McpError::parse_error(&format!("Invalid JSON: {}", e));
                let response = McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id: None,
                    result: None,
                    error: Some(error),
                };
                write_json_line(&mut writer, &response).await;
            }
        }
    }
}
