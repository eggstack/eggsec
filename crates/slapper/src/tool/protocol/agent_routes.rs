use axum::{routing::get, routing::post, routing::delete, extract::{State, Path}, response::{IntoResponse, Response}, Json, Router, http::{HeaderMap, StatusCode}};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use subtle::ConstantTimeEq;
use std::net::IpAddr;

use crate::tool::agents::{AgentRegistry, AgentInfo, AgentStatus, TaskScheduler, ScheduledTask, TaskPriority, TaskStatus};

#[derive(Debug, Clone, PartialEq)]
pub enum CallbackUrlValidationError {
    InvalidUrl(String),
    UnsupportedScheme(String),
    ContainsCredentials,
    ResolvesToForbiddenIp(String),
    MissingHost,
}

pub fn validate_callback_url(url_str: &str) -> Result<(), CallbackUrlValidationError> {
    let resolver = |host: &str, port: u16| -> Result<Vec<IpAddr>, CallbackUrlValidationError> {
        let addr_str = if port == 0 {
            host.to_string()
        } else {
            format!("{}:{}", host, port)
        };
        std::net::ToSocketAddrs::to_socket_addrs(addr_str.as_str())
            .map(|iter| iter.map(|a| a.ip()).collect())
            .map_err(|e| CallbackUrlValidationError::ResolvesToForbiddenIp(format!("DNS failed: {}", e)))
    };
    validate_callback_url_with_resolver(url_str, resolver)
}

pub fn validate_callback_url_with_resolver<F>(url_str: &str, resolver: F) -> Result<(), CallbackUrlValidationError>
where
    F: Fn(&str, u16) -> Result<Vec<IpAddr>, CallbackUrlValidationError>,
{
    let parsed = url::Url::parse(url_str)
        .map_err(|e| CallbackUrlValidationError::InvalidUrl(e.to_string()))?;

    let scheme = parsed.scheme();
    if scheme != "http" && scheme != "https" {
        return Err(CallbackUrlValidationError::UnsupportedScheme(scheme.to_string()));
    }

    let has_cred = !parsed.username().is_empty() || parsed.password().is_some();
    if has_cred {
        return Err(CallbackUrlValidationError::ContainsCredentials);
    }

    let host = parsed.host_str()
        .ok_or(CallbackUrlValidationError::MissingHost)?;

    let host_lower = host.to_lowercase();
    if host_lower == "localhost" || host_lower == "localhost." {
        return Err(CallbackUrlValidationError::ResolvesToForbiddenIp(host.to_string()));
    }

    let port = parsed.port().unwrap_or_else(|| {
        match scheme {
            "https" => 443,
            _ => 80,
        }
    });

    if let Ok(ip) = host.parse::<IpAddr>() {
        if is_forbidden_ip(&ip) {
            return Err(CallbackUrlValidationError::ResolvesToForbiddenIp(host.to_string()));
        }
    } else {
        let addrs = resolver(host, port)?;
        if addrs.iter().any(|ip| is_forbidden_ip(ip)) {
            return Err(CallbackUrlValidationError::ResolvesToForbiddenIp(host.to_string()));
        }
    }

    Ok(())
}

fn is_forbidden_ip(ip: &IpAddr) -> bool {
    is_loopback_ip(ip)
        || is_private_ip(ip)
        || ip.is_unspecified()
        || ip.is_multicast()
        || is_link_local_ip(ip)
        || is_benchmark_ip(ip)
        || is_documentation_ip(ip)
}

fn is_loopback_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(ipv4) => ipv4.octets()[0] == 127,
        IpAddr::V6(ipv6) => ipv6.is_loopback(),
    }
}

fn is_private_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(ipv4) => {
            let octets = ipv4.octets();
            octets[0] == 10
                || (octets[0] == 172 && (15..=31).contains(&octets[1]))
                || (octets[0] == 192 && octets[1] == 168)
        }
        IpAddr::V6(ipv6) => {
            let segs = ipv6.segments();
            (segs[0] & 0xfe00) == 0xfc00
        }
    }
}

fn is_link_local_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(ipv4) => ipv4.octets()[0] == 169 && ipv4.octets()[1] == 254,
        IpAddr::V6(ipv6) => (ipv6.segments()[0] & 0xffc0) == 0xfe80,
    }
}

fn is_benchmark_ip(ip: &IpAddr) -> bool {
    if let IpAddr::V4(ipv4) = ip {
        let octets = ipv4.octets();
        octets[0] == 198 && (octets[1] == 18 || octets[1] == 19)
    } else {
        false
    }
}

fn is_documentation_ip(ip: &IpAddr) -> bool {
    if let IpAddr::V4(ipv4) = ip {
        let octets = ipv4.octets();
        (octets[0] == 192 && octets[1] == 0 && octets[2] == 2)
            || (octets[0] == 198 && octets[1] == 51 && octets[2] == 100)
            || (octets[0] == 203 && octets[1] == 0 && octets[2] == 113)
    } else {
        false
    }
}

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
) -> Result<Json<RegisterAgentResponse>, (StatusCode, String)> {
    if let Err(e) = require_auth(&state.api_key, &headers) {
        return Err((StatusCode::UNAUTHORIZED, e.to_string()));
    }
    if let Some(ref callback_url) = req.callback_url {
        if let Err(e) = validate_callback_url(callback_url) {
            return Err((StatusCode::BAD_REQUEST, format!("Invalid callback URL: {:?}", e)));
        }
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
        None => Err((StatusCode::NOT_FOUND, "Agent not found")),
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

    let scheduler = state.scheduler.clone();
    let mut task = scheduler.create_task(
        req.task_type.clone(),
        req.payload.clone(),
    );
    task.priority = priority;
    if let Some(agent_id) = req.agent_id {
        task.assigned_agent_id = Some(agent_id);
    }
    let task_id = task.id;
    let priority_str = match task.priority {
        TaskPriority::Critical => "critical",
        TaskPriority::High => "high",
        TaskPriority::Normal => "normal",
        TaskPriority::Low => "low",
    };
    scheduler.schedule(task).await;

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
    pub status: String,
    pub retry_count: usize,
    pub created_at: u64,
    pub assigned_agent_id: Option<Uuid>,
}

async fn list_tasks(
    State(state): State<AgentState>,
    headers: HeaderMap,
) -> Result<Json<ListTasksResponse>, &'static str> {
    if let Err(e) = require_auth(&state.api_key, &headers) {
        return Err(e);
    }
    let tasks_raw = state.scheduler.list_all_tasks().await;
    let tasks: Vec<TaskInfo> = tasks_raw.iter().map(|t| {
        let priority_str = match t.priority {
            TaskPriority::Critical => "critical",
            TaskPriority::High => "high",
            TaskPriority::Normal => "normal",
            TaskPriority::Low => "low",
        };
        let status_str = match t.status {
            TaskStatus::Pending => "pending",
            TaskStatus::Leased => "leased",
            TaskStatus::Completed => "completed",
            TaskStatus::Failed => "failed",
            TaskStatus::Cancelled => "cancelled",
        };
        TaskInfo {
            id: t.id,
            task_type: t.task_type.clone(),
            priority: priority_str.to_string(),
            status: status_str.to_string(),
            retry_count: t.retry_count,
            created_at: t.created_at,
            assigned_agent_id: t.assigned_agent_id,
        }
    }).collect();
    let total = tasks.len();
    Ok(Json(ListTasksResponse { tasks, total }))
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

#[derive(Debug, Deserialize)]
pub struct LeaseTaskRequest {
    pub agent_id: Uuid,
    pub lease_duration_ms: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct LeaseTaskResponse {
    pub id: Uuid,
    pub leased: bool,
    pub task_type: Option<String>,
    pub payload: Option<serde_json::Value>,
}

async fn lease_task(
    State(state): State<AgentState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(req): Json<LeaseTaskRequest>,
) -> Result<Json<LeaseTaskResponse>, &'static str> {
    if let Err(e) = require_auth(&state.api_key, &headers) {
        return Err(e);
    }
    let lease_duration_ms = req.lease_duration_ms.unwrap_or(300000);
    let task = state.scheduler.get_task(id).await;
    match task {
        Some(t) if t.status == TaskStatus::Pending => {
            if state.scheduler.lease_task(id, req.agent_id, lease_duration_ms).await {
                let task_after = state.scheduler.get_task(id).await;
                Ok(Json(LeaseTaskResponse {
                    id,
                    leased: true,
                    task_type: task_after.as_ref().map(|t| t.task_type.clone()),
                    payload: task_after.as_ref().map(|t| t.payload.clone()),
                }))
            } else {
                Ok(Json(LeaseTaskResponse {
                    id,
                    leased: false,
                    task_type: None,
                    payload: None,
                }))
            }
        }
        _ => Ok(Json(LeaseTaskResponse {
            id,
            leased: false,
            task_type: None,
            payload: None,
        })),
    }
}

#[derive(Debug, Deserialize)]
pub struct SubmitResultRequest {
    pub success: bool,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SubmitResultResponse {
    pub id: Uuid,
    pub accepted: bool,
}

async fn submit_task_result(
    State(state): State<AgentState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(req): Json<SubmitResultRequest>,
) -> Result<Json<SubmitResultResponse>, &'static str> {
    if let Err(e) = require_auth(&state.api_key, &headers) {
        return Err(e);
    }
    let accepted = state.scheduler.submit_result(id, req.success, req.result, req.error).await;
    Ok(Json(SubmitResultResponse { id, accepted }))
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
            Some(v) if bool::from(key.as_bytes().ct_eq(v.as_bytes())) => Ok(()),
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
        .route("/api/v1/tasks/{id}/lease", post(lease_task))
        .route("/api/v1/tasks/{id}/result", post(submit_task_result))
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

#[tokio::test]
    async fn test_task_status_pending() {
        let scheduler = TaskScheduler::new();
        let task = scheduler.create_task("scan", serde_json::json!({}));
        assert_eq!(task.status, TaskStatus::Pending);
        scheduler.schedule(task).await;
        let agent_id = Uuid::new_v4();
        let next = scheduler.lease_next_task(agent_id, 60000).await;
        assert!(next.is_some());
        assert_eq!(next.unwrap().status, TaskStatus::Leased);
    }

    #[tokio::test]
    async fn test_leased_task_cannot_be_leased_again() {
        let scheduler = TaskScheduler::new();
        let task = scheduler.create_task("scan", serde_json::json!({}));
        let task_id = task.id;
        scheduler.schedule(task).await;
        let agent_id1 = Uuid::new_v4();
        let agent_id2 = Uuid::new_v4();
        let first_lease = scheduler.lease_task(task_id, agent_id1, 60000).await;
        assert!(first_lease);
        let second_lease = scheduler.lease_task(task_id, agent_id2, 60000).await;
        assert!(!second_lease);
    }

    #[tokio::test]
    async fn test_task_lease_and_result() {
        let scheduler = TaskScheduler::new();
        let task = scheduler.create_task("scan", serde_json::json!({}));
        let task_id = task.id;
        scheduler.schedule(task).await;
        let agent_id = Uuid::new_v4();
        let leased = scheduler.lease_task(task_id, agent_id, 60000).await;
        assert!(leased);
        let updated = scheduler.get_task(task_id).await;
        assert!(updated.is_some());
        let updated_task = updated.unwrap();
        assert_eq!(updated_task.status, TaskStatus::Leased);
        assert_eq!(updated_task.assigned_agent_id, Some(agent_id));
        let submitted = scheduler.submit_result(task_id, true, Some(serde_json::json!({"result": "ok"})), None).await;
        assert!(submitted);
        let final_task = scheduler.get_task(task_id).await;
        assert!(final_task.is_some());
        assert_eq!(final_task.unwrap().status, TaskStatus::Completed);
    }

    #[tokio::test]
    async fn test_leased_task_invisible_to_next_task() {
        let scheduler = TaskScheduler::new();
        let task = scheduler.create_task("scan", serde_json::json!({}));
        let task_id = task.id;
        scheduler.schedule(task).await;
        let agent_id = Uuid::new_v4();
        scheduler.lease_task(task_id, agent_id, 60000).await;
        let agent_id2 = Uuid::new_v4();
        let next = scheduler.lease_next_task(agent_id2, 60000).await;
        assert!(next.is_none());
    }

    #[tokio::test]
    async fn test_cancel_prevents_lease() {
        let scheduler = TaskScheduler::new();
        let task = scheduler.create_task("scan", serde_json::json!({}));
        let task_id = task.id;
        scheduler.schedule(task).await;
        scheduler.cancel(task_id).await;
        let agent_id = Uuid::new_v4();
        let leased = scheduler.lease_task(task_id, agent_id, 60000).await;
        assert!(!leased);
    }

#[tokio::test]
    async fn test_failed_task_with_retries_becomes_pending() {
        let scheduler = TaskScheduler::new();
        let task = scheduler.create_task("scan", serde_json::json!({}));
        let task_id = task.id;
        scheduler.schedule(task).await;
        let agent_id = Uuid::new_v4();
        scheduler.lease_task(task_id, agent_id, 60000).await;
        scheduler.submit_result(task_id, false, None, Some("error".to_string())).await;
        let failed_task = scheduler.get_task(task_id).await.unwrap();
        assert_eq!(failed_task.status, TaskStatus::Pending);
        assert_eq!(failed_task.retry_count, 1);
        assert_eq!(failed_task.error, Some("error".to_string()));
    }

    #[test]
    fn test_router_creation() {
        let registry = AgentRegistry::new();
        let scheduler = TaskScheduler::new();
        let _router = router(registry, scheduler, None);
    }

    #[test]
    fn test_callback_url_rejects_localhost() {
        assert!(validate_callback_url("http://127.0.0.1:8080").is_err());
        assert!(validate_callback_url("http://localhost:8080").is_err());
    }

    #[test]
    fn test_callback_url_rejects_loopback() {
        assert!(validate_callback_url("http://127.0.0.1").is_err());
        assert!(validate_callback_url("http://127.255.255.255").is_err());
    }

    #[test]
    fn test_callback_url_rejects_private_ips() {
        assert!(validate_callback_url("http://10.0.0.1").is_err());
        assert!(validate_callback_url("http://172.16.0.1").is_err());
        assert!(validate_callback_url("http://192.168.1.1").is_err());
    }

    #[test]
    fn test_callback_url_rejects_link_local() {
        assert!(validate_callback_url("http://169.254.0.1").is_err());
    }

    #[test]
    fn test_callback_url_rejects_aws_metadata() {
        assert!(validate_callback_url("http://169.254.169.254/latest/meta-data").is_err());
    }

    #[test]
    fn test_callback_url_rejects_file_scheme() {
        assert!(validate_callback_url("file:///tmp/x").is_err());
    }

    #[test]
    fn test_callback_url_rejects_credentials() {
        assert!(validate_callback_url("http://user:pass@example.com").is_err());
    }

    #[test]
    fn test_callback_url_rejects_unspecified() {
        assert!(validate_callback_url("http://0.0.0.0").is_err());
    }

    #[test]
    fn test_callback_url_rejects_documentation_ips() {
        assert!(validate_callback_url("http://192.0.2.0").is_err());
        assert!(validate_callback_url("http://198.51.100.0").is_err());
        assert!(validate_callback_url("http://203.0.113.0").is_err());
    }

    #[test]
    fn test_callback_url_rejects_benchmark() {
        assert!(validate_callback_url("http://198.18.0.1").is_err());
    }

    #[test]
    fn test_callback_url_accepts_safe_https() {
        assert!(validate_callback_url("https://example.com").is_ok());
        assert!(validate_callback_url("https://example.com:8443/callback").is_ok());
    }

    #[test]
    fn test_callback_url_rejects_unsupported_scheme() {
        assert!(validate_callback_url("ftp://example.com").is_err());
        assert!(validate_callback_url("ws://example.com").is_err());
    }

    #[test]
    fn test_is_forbidden_ip_direct() {
        use std::net::IpAddr;
        use std::str::FromStr;

        assert!(is_forbidden_ip(&IpAddr::from_str("127.0.0.1").unwrap()));
        assert!(is_forbidden_ip(&IpAddr::from_str("10.0.0.1").unwrap()));
        assert!(is_forbidden_ip(&IpAddr::from_str("172.16.0.1").unwrap()));
        assert!(is_forbidden_ip(&IpAddr::from_str("192.168.1.1").unwrap()));
        assert!(is_forbidden_ip(&IpAddr::from_str("0.0.0.0").unwrap()));
        assert!(is_forbidden_ip(&IpAddr::from_str("169.254.0.1").unwrap()));
        assert!(is_forbidden_ip(&IpAddr::from_str("192.0.2.0").unwrap()));
        assert!(is_forbidden_ip(&IpAddr::from_str("198.18.0.1").unwrap()));

        assert!(!is_forbidden_ip(&IpAddr::from_str("8.8.8.8").unwrap()));
        assert!(!is_forbidden_ip(&IpAddr::from_str("1.1.1.1").unwrap()));
    }

    #[test]
    fn test_is_forbidden_ip_extended() {
        use std::net::IpAddr;
        use std::str::FromStr;

        assert!(is_forbidden_ip(&IpAddr::from_str("127.255.255.255").unwrap()));
        assert!(is_forbidden_ip(&IpAddr::from_str("172.31.255.255").unwrap()));
        assert!(is_forbidden_ip(&IpAddr::from_str("224.0.0.1").unwrap()));
        assert!(is_forbidden_ip(&IpAddr::from_str("192.0.2.55").unwrap()));
        assert!(is_forbidden_ip(&IpAddr::from_str("198.51.100.55").unwrap()));
        assert!(is_forbidden_ip(&IpAddr::from_str("203.0.113.55").unwrap()));
        assert!(is_forbidden_ip(&IpAddr::from_str("198.19.255.255").unwrap()));
        assert!(is_forbidden_ip(&IpAddr::from_str("::1").unwrap()));
        assert!(is_forbidden_ip(&IpAddr::from_str("fc00::1").unwrap()));
        assert!(is_forbidden_ip(&IpAddr::from_str("fd00::1").unwrap()));
        assert!(is_forbidden_ip(&IpAddr::from_str("fe80::1").unwrap()));
        assert!(is_forbidden_ip(&IpAddr::from_str("169.254.169.254").unwrap()));
    }

    #[test]
    fn test_callback_url_rejects_non_first_private_ip() {
        let resolver = |_host: &str, _port: u16| -> Result<Vec<IpAddr>, CallbackUrlValidationError> {
            Ok(vec![
                "8.8.8.8".parse().unwrap(),
                "10.0.0.1".parse().unwrap(),
            ])
        };
        assert!(validate_callback_url_with_resolver("http://example.com", resolver).is_err());
    }

    #[test]
    fn test_callback_url_accepts_safe_with_fake_resolver() {
        let resolver = |_host: &str, _port: u16| -> Result<Vec<IpAddr>, CallbackUrlValidationError> {
            Ok(vec!["8.8.8.8".parse().unwrap()])
        };
        assert!(validate_callback_url_with_resolver("http://example.com", resolver).is_ok());
    }

    #[test]
    fn test_callback_url_rejects_localhost_case_insensitive() {
        assert!(validate_callback_url("http://LOCALHOST:8080").is_err());
        assert!(validate_callback_url("http://Localhost:8080").is_err());
        assert!(validate_callback_url("http://localhost.:8080").is_err());
    }
}
