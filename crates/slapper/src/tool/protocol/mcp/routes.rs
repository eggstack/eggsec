use std::sync::Arc;
use std::time::Duration;

use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
    response::{sse::Event as SseEvent, IntoResponse, Sse},
    routing::{get, post},
    Router,
};
use async_stream::stream;
use futures::Stream;

use crate::tool::{ChainPlanner, OpenApiGenerator, PlanRequest, ExecutionPlan, ToolRegistry};

use super::handlers::McpServer;
use super::streaming::StreamEvent;
use super::types::{McpError, McpRequest, McpResponse};

struct AppState {
    mcp_server: Arc<McpServer>,
    planner: ChainPlanner,
    openapi_generator: OpenApiGenerator,
}

async fn handle_openapi_json(
    State(state): State<Arc<AppState>>,
) -> axum::Json<serde_json::Value> {
    let spec = state.openapi_generator.generate(&state.mcp_server.registry);
    axum::Json(serde_json::from_str(&spec.to_json()).unwrap_or_default())
}

async fn handle_openapi_yaml(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let spec = state.openapi_generator.generate(&state.mcp_server.registry);
    (
        [("Content-Type", "application/x-yaml")],
        spec.to_yaml(),
    )
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

pub async fn create_mcp_router(registry: ToolRegistry, api_key: Option<String>) -> Router {
    let server = Arc::new(McpServer::new(registry.clone(), api_key));
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
        .route("/mcp/stream/:request_id", get(handle_sse_stream))
        .route("/health", get(handle_health))
        .route("/openapi.json", get(handle_openapi_json))
        .route("/openapi.yaml", get(handle_openapi_yaml))
        .route("/plan", post(handle_create_plan))
        .with_state(app_state)
}

async fn handle_health() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "status": "healthy",
        "service": "slapper-mcp",
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
    
    let state = Arc::new(tokio::sync::Mutex::new(SseStreamState {
        receiver,
        request_id: request_id.clone(),
    }));
    
    let stream = stream! {
        let mut tick_interval = tokio::time::interval(Duration::from_secs(30));
        
        loop {
            let event = {
                let mut s = state.lock().await;
                s.receiver.recv().await
            };
            
            match event {
                Ok(event) => {
                    let current_request_id = {
                        let s = state.lock().await;
                        s.request_id.clone()
                    };
                    if event.request_id == current_request_id || event.request_id == "*" {
                        yield Ok::<_, axum::Error>(SseEvent::default()
                            .event(&event.event_type)
                            .data(serde_json::to_string(&event.data).unwrap_or_default()));
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

            tokio::select! {
                _ = tick_interval.tick() => {
                    yield Ok::<_, axum::Error>(SseEvent::default()
                        .event("heartbeat")
                        .data("{\"timestamp\": \"alive\"}"));
                }
                _ = tokio::time::sleep(Duration::from_millis(100)) => {
                    // Small delay to prevent busy loop
                }
            }
        }
    };

    Sse::new(stream)
        .keep_alive(axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(15)))
}

async fn handle_mcp(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(requests): Json<Vec<McpRequest>>,
) -> impl IntoResponse {
    const MAX_BATCH_SIZE: usize = 100;

    if requests.len() > MAX_BATCH_SIZE {
        return (
            StatusCode::BAD_REQUEST,
            Json(vec![McpResponse {
                jsonrpc: "2.0".to_string(),
                id: None,
                result: None,
                error: Some(McpError::invalid_request(&format!(
                    "Batch size exceeds limit of {}",
                    MAX_BATCH_SIZE
                ))),
            }]),
        );
    }

    let mut responses = Vec::new();

    for req in requests {
        if let Err(e) = state.mcp_server.validate_auth(&headers) {
            responses.push(req.error_response(e));
            continue;
        }

        let response = state.mcp_server.handle_request(req).await;
        responses.push(response);
    }

    (StatusCode::OK, Json(responses))
}

pub async fn run_stdio(registry: ToolRegistry, api_key: Option<String>) {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};

    let server = Arc::new(McpServer::new(registry, api_key));

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let mut reader: tokio::io::Lines<BufReader<tokio::io::Stdin>> = BufReader::new(stdin).lines();
    let mut writer = BufWriter::new(stdout);

    tracing::info!("MCP stdio server started, waiting for requests...");

    while let Ok(Some(line)) = reader.next_line().await {
        if line.trim().is_empty() {
            continue;
        }

        let requests: Result<Vec<McpRequest>, _> = serde_json::from_str(&line);

        match requests {
            Ok(reqs) => {
                if reqs.len() > 100 {
                    let error = McpError::invalid_request("Batch size exceeds limit of 100");
                    let response = McpResponse {
                        jsonrpc: "2.0".to_string(),
                        id: None,
                        result: None,
                        error: Some(error),
                    };
                    if let Ok(response_json) = serde_json::to_string(&response) {
                        let _ = writer.write_all(response_json.as_bytes()).await;
                        let _ = writer.write_all(b"\n").await;
                        let _ = writer.flush().await;
                    }
                    continue;
                }

                let mut responses = Vec::new();

                for req in reqs {
                    if req.method != "initialize" {
                        if let Err(e) = server.validate_auth_params(&req.params) {
                            responses.push(req.error_response(e));
                            continue;
                        }
                    }

                    let response = server.handle_request(req).await;
                    responses.push(response);
                }

                if let Ok(response_json) = serde_json::to_string(&responses) {
                    let _ = writer.write_all(response_json.as_bytes()).await;
                    let _ = writer.write_all(b"\n").await;
                    let _ = writer.flush().await;
                }
            }
            Err(e) => {
                let error = McpError::parse_error(&format!("Invalid JSON: {}", e));
                let response = McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id: None,
                    result: None,
                    error: Some(error),
                };
                if let Ok(response_json) = serde_json::to_string(&response) {
                    let _ = writer.write_all(response_json.as_bytes()).await;
                    let _ = writer.write_all(b"\n").await;
                    let _ = writer.flush().await;
                }
            }
        }
    }
}
