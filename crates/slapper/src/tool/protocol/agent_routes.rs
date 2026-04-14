use axum::{routing::get, routing::post, routing::delete, extract::State, response::IntoResponse, Json, Router, http::HeaderMap};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use subtle::ConstantTimeEq;

use crate::tool::agents::{AgentRegistry, AgentInfo, AgentStatus, TaskScheduler, ScheduledTask, TaskPriority};

#[derive(Debug, Deserialize)]
pub struct RegisterAgentRequest {
    pub name: String,
    pub capabilities: Vec<String>,
    pub callback_url: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RegisterAgentResponse {
    pub id: Uuid,
    pub name: String,
    pub status: String,
    pub capabilities: Vec<String>,
}

async fn register_agent(
    State(state): State<AgentState>,
    headers: HeaderMap,
    Json(req): Json<RegisterAgentRequest>,
) -> Result<Json<RegisterAgentResponse>, &'static str> {
    if let Err(e) = require_auth(&state.api_key, &headers) {
        return Err(e);
    }
    let id = Uuid::new_v4();
    let agent = AgentInfo {
        id,
        name: req.name.clone(),
        capabilities: req.capabilities.clone(),
        status: AgentStatus::Active,
        last_heartbeat: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        callback_url: req.callback_url.clone(),
    };
    state.registry.register(agent).await;
    Ok(Json(RegisterAgentResponse {
        id,
        name: req.name,
        status: "active".to_string(),
        capabilities: req.capabilities,
    }))
}

#[derive(Debug, Serialize)]
pub struct ListAgentsResponse {
    pub agents: Vec<AgentInfo>,
    pub total: usize,
}

async fn list_agents(
    State(state): State<AgentState>,
    headers: HeaderMap,
) -> Result<Json<ListAgentsResponse>, &'static str> {
    if let Err(e) = require_auth(&state.api_key, &headers) {
        return Err(e);
    }
    let agents = state.registry.list().await;
    let total = agents.len();
    Ok(Json(ListAgentsResponse { agents, total }))
}

async fn get_agent(
    State(state): State<AgentState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, impl IntoResponse> {
    if let Err(e) = require_auth(&state.api_key, &headers) {
        return Err((StatusCode::UNAUTHORIZED, e));
    }
    match state.registry.get(id).await {
        Some(agent) => Ok(axum::Json(agent)),
        None => Err(StatusCode::NOT_FOUND),
    }
}

async fn unregister_agent(
    State(state): State<AgentState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, impl IntoResponse> {
    if let Err(e) = require_auth(&state.api_key, &headers) {
        return Err((StatusCode::UNAUTHORIZED, e));
    }
    state.registry.unregister(id).await;
    Ok(Response::builder()
        .status(StatusCode::NO_CONTENT)
        .body("".to_string())
        .unwrap_or_else(|_| Response::builder()
            .status(StatusCode::NO_CONTENT)
            .body("".to_string())
            .unwrap()))
}

#[derive(Debug, Serialize)]
pub struct HeartbeatResponse {
    pub id: Uuid,
    pub last_heartbeat: u64,
    pub status: String,
}

async fn heartbeat(
    State(state): State<AgentState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<HeartbeatResponse>, &'static str> {
    if let Err(e) = require_auth(&state.api_key, &headers) {
        return Err(e);
    }
    state.registry.heartbeat(id).await;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    Ok(Json(HeartbeatResponse {
        id,
        last_heartbeat: now,
        status: "ok".to_string(),
    }))
}

#[derive(Debug, Deserialize)]
pub struct CreateTaskRequest {
    pub task_type: String,
    pub payload: serde_json::Value,
    pub priority: Option<String>,
    pub agent_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
pub struct CreateTaskResponse {
    pub id: Uuid,
    pub task_type: String,
    pub priority: String,
    pub status: String,
}

async fn create_task(
    State(state): State<AgentState>,
    headers: HeaderMap,
    Json(req): Json<CreateTaskRequest>,
) -> Result<Json<CreateTaskResponse>, &'static str> {
    if let Err(e) = require_auth(&state.api_key, &headers) {
        return Err(e);
    }
    let priority = match req.priority.as_deref() {
        Some("critical") => TaskPriority::Critical,
        Some("high") => TaskPriority::High,
        Some("low") => TaskPriority::Low,
        _ => TaskPriority::Normal,
    };

    let task = ScheduledTask {
        id: Uuid::new_v4(),
        task_type: req.task_type.clone(),
        payload: req.payload.clone(),
        priority,
        retry_count: 0,
        max_retries: 3,
        created_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64,
        scheduled_for: None,
    };

    let task_id = task.id;
    state.scheduler.schedule(task).await;

    let priority_str = match priority {
        TaskPriority::Critical => "critical",
        TaskPriority::High => "high",
        TaskPriority::Normal => "normal",
        TaskPriority::Low => "low",
    };

    Ok(Json(CreateTaskResponse {
        id: task_id,
        task_type: req.task_type,
        priority: priority_str.to_string(),
        status: "scheduled".to_string(),
    }))
}

#[derive(Debug, Serialize)]
pub struct ListTasksResponse {
    pub tasks: Vec<TaskInfo>,
    pub total: usize,
}

#[derive(Debug, Serialize)]
pub struct TaskInfo {
    pub id: Uuid,
    pub task_type: String,
    pub priority: String,
    pub retry_count: usize,
    pub created_at: u64,
}

async fn list_tasks(
    State(state): State<AgentState>,
    headers: HeaderMap,
) -> Result<Json<ListTasksResponse>, &'static str> {
    if let Err(e) = require_auth(&state.api_key, &headers) {
        return Err(e);
    }
    let count = state.scheduler.pending_count().await;
    Ok(Json(ListTasksResponse {
        tasks: vec![],
        total: count,
    }))
}

async fn get_task(
    State(state): State<AgentState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, impl IntoResponse> {
    if let Err(e) = require_auth(&state.api_key, &headers) {
        return Err((StatusCode::UNAUTHORIZED, e));
    }
    let body = serde_json::to_string(&serde_json::json!({
        "id": id,
        "status": "not_found",
        "message": "Task details are only available while tasks are in memory"
    })).unwrap_or_else(|_| r#"{"id":"error","status":"not_found"}"#.to_string());
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "application/json")
        .body(body)
        .unwrap_or_else(|_| {
            Response::builder()
                .status(StatusCode::OK)
                .body("{\"id\":\"error\",\"status\":\"not_found\"}".to_string())
                .unwrap()
        }))
}

#[derive(Debug, Serialize)]
pub struct CancelTaskResponse {
    pub id: Uuid,
    pub cancelled: bool,
}

async fn cancel_task(
    State(state): State<AgentState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<CancelTaskResponse>, &'static str> {
    if let Err(e) = require_auth(&state.api_key, &headers) {
        return Err(e);
    }
    let cancelled = state.scheduler.cancel(id).await;
    Ok(Json(CancelTaskResponse { id, cancelled }))
}

#[derive(Clone)]
pub struct AgentState {
    pub registry: AgentRegistry,
    pub scheduler: TaskScheduler,
    pub api_key: Option<String>,
}

fn require_auth(api_key: &Option<String>, headers: &HeaderMap) -> Result<(), &'static str> {
    if let Some(ref key) = api_key {
        let auth = headers
            .get("authorization")
            .or_else(|| headers.get("x-api-key"))
            .and_then(|v| v.to_str().ok());

        match auth {
            Some(v) if key.as_bytes().ct_eq(v.as_bytes()).unwrap_u8() == 1 => Ok(()),
            _ => Err("Invalid or missing API key"),
        }
    } else {
        Ok(())
    }
}

pub fn router(registry: AgentRegistry, scheduler: TaskScheduler, api_key: Option<String>) -> Router {
    let state = AgentState { registry, scheduler, api_key };
    Router::new()
        .route("/api/v1/agents", post(register_agent))
        .route("/api/v1/agents", get(list_agents))
        .route("/api/v1/agents/{id}", get(get_agent))
        .route("/api/v1/agents/{id}", delete(unregister_agent))
        .route("/api/v1/agents/{id}/heartbeat", post(heartbeat))
        .route("/api/v1/tasks", post(create_task))
        .route("/api/v1/tasks", get(list_tasks))
        .route("/api/v1/tasks/{id}", get(get_task))
        .route("/api/v1/tasks/{id}/cancel", post(cancel_task))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_agent(name: &str, caps: Vec<String>) -> AgentInfo {
        AgentInfo {
            id: Uuid::new_v4(),
            name: name.to_string(),
            capabilities: caps,
            status: AgentStatus::Active,
            last_heartbeat: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            callback_url: None,
        }
    }

    #[tokio::test]
    async fn test_register_agent_in_registry() {
        let registry = AgentRegistry::new();
        let agent = make_agent("test-agent", vec!["scan".to_string()]);
        registry.register(agent.clone()).await;
        let found = registry.get(agent.id).await;
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "test-agent");
    }

    #[tokio::test]
    async fn test_list_agents_empty() {
        let registry = AgentRegistry::new();
        let agents = registry.list().await;
        assert!(agents.is_empty());
    }

    #[tokio::test]
    async fn test_list_agents_after_register() {
        let registry = AgentRegistry::new();
        registry.register(make_agent("agent1", vec!["scan".to_string()])).await;
        registry.register(make_agent("agent2", vec!["fuzz".to_string()])).await;
        let agents = registry.list().await;
        assert_eq!(agents.len(), 2);
    }

    #[tokio::test]
    async fn test_unregister_agent() {
        let registry = AgentRegistry::new();
        let agent = make_agent("test-agent", vec!["scan".to_string()]);
        let id = agent.id;
        registry.register(agent).await;
        registry.unregister(id).await;
        assert!(registry.get(id).await.is_none());
    }

    #[tokio::test]
    async fn test_heartbeat_updates_agent() {
        let registry = AgentRegistry::new();
        let agent = make_agent("test-agent", vec!["scan".to_string()]);
        let id = agent.id;
        registry.register(agent).await;
        let before = registry.get(id).await.unwrap().last_heartbeat;
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        registry.heartbeat(id).await;
        let after = registry.get(id).await.unwrap().last_heartbeat;
        assert!(after > before);
    }

    #[tokio::test]
    async fn test_scheduler_create_task() {
        let scheduler = TaskScheduler::new();
        let task = scheduler.create_task(
            "scan",
            serde_json::json!({"target": "http://example.com"}),
        );
        scheduler.schedule(task).await;
        assert!(scheduler.pending_count().await > 0);
    }

    #[test]
    fn test_router_creation() {
        let registry = AgentRegistry::new();
        let scheduler = TaskScheduler::new();
        let _router = router(registry, scheduler);
    }
}
