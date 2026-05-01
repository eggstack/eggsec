use axum::{routing::get, routing::post, routing::delete, extract::{State, Path}, response::{IntoResponse, Response}, Json, Router, http::{HeaderMap, StatusCode}};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use subtle::ConstantTimeEq;
use std::net::{IpAddr, ToSocketAddrs};

use crate::tool::agents::{AgentRegistry, AgentInfo, AgentStatus, TaskScheduler, ScheduledTask, TaskPriority};

#[derive(Debug, Clone, PartialEq)]
pub enum CallbackUrlValidationError {
    InvalidUrl(String),
    UnsupportedScheme(String),
    ContainsCredentials,
    ResolvesToForbiddenIp(String),
    MissingHost,
}

pub fn validate_callback_url(url_str: &str) -> Result<(), CallbackUrlValidationError> {
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

    if let Ok(ip) = host.parse::<IpAddr>() {
        if is_forbidden_ip(&ip) {
            return Err(CallbackUrlValidationError::ResolvesToForbiddenIp(host.to_string()));
        }
    } else {
        let addrs: Vec<_> = (host, 0).to_socket_addrs()
            .map_err(|e| CallbackUrlValidationError::ResolvesToForbiddenIp(format!("DNS failed: {}", e)))?
            .collect();
        
        if let Some(first_ip) = addrs.first().map(|a| a.ip()) {
            if is_forbidden_ip(&first_ip) {
                return Err(CallbackUrlValidationError::ResolvesToForbiddenIp(host.to_string()));
            }
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
            segs[0] == 0xfc00 >> 8 || segs[0] == 0xfd00 >> 8
        }
    }
}

fn is_link_local_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(ipv4) => ipv4.octets()[0] == 169 && ipv4.octets()[1] == 254,
        IpAddr::V6(ipv6) => ipv6.segments()[0] == 0xfe80 >> 8,
    }
}

fn is_benchmark_ip(ip: &IpAddr) -> bool {
    if let IpAddr::V4(ipv4) = ip {
        ipv4.octets()[0] == 198 && ipv4.octets()[1] == 18
    } else {
        false
    }
}

fn is_documentation_ip(ip: &IpAddr) -> bool {
    if let IpAddr::V4(ipv4) = ip {
        ipv4.octets() == [192, 0, 2, 0]
            || ipv4.octets() == [198, 51, 100, 0]
            || ipv4.octets() == [203, 0, 113, 0]
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
) -> Result<Json<RegisterAgentResponse>, &'static str> {
    if let Err(e) = require_auth(&state.api_key, &headers) {
        return Err(e);
    }
    if let Some(ref callback_url) = req.callback_url {
        if let Err(e) = validate_callback_url(callback_url) {
            return Err("Invalid callback URL");
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
}
