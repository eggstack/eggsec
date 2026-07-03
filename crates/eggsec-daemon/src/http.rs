use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use futures::stream::Stream;
use tokio_util::sync::CancellationToken;

use crate::host::DaemonHost;
use crate::protocol::{
    ClientCommand, DaemonRequestContext, ErrorCode, ServerMessage, TransportKind,
};

pub struct HttpConfig {
    pub bind_addr: String,
    pub require_auth: bool,
    pub allow_public_bind: bool,
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            bind_addr: "127.0.0.1:9876".into(),
            require_auth: false,
            allow_public_bind: false,
        }
    }
}

struct HttpState {
    host: Arc<DaemonHost>,
    client_id: std::sync::Mutex<Option<eggsec_runtime::ClientId>>,
}

fn make_ctx(state: &HttpState) -> DaemonRequestContext {
    DaemonRequestContext {
        client_id: *state.client_id.lock().unwrap(),
        peer: None,
        transport: TransportKind::LoopbackHttp,
    }
}

fn error_response(status: StatusCode, code: ErrorCode, message: String) -> Response {
    (
        status,
        Json(serde_json::json!({"code": code, "message": message})),
    )
        .into_response()
}

async fn health(State(state): State<Arc<HttpState>>) -> Response {
    let cmd = ClientCommand::Health {
        request_id: uuid::Uuid::new_v4().to_string(),
    };
    let resp = state.host.handle_command(cmd, make_ctx(&state)).await;
    Json(serde_json::to_value(&resp).unwrap()).into_response()
}

async fn capabilities(State(state): State<Arc<HttpState>>) -> Response {
    let cmd = ClientCommand::Capabilities {
        request_id: uuid::Uuid::new_v4().to_string(),
    };
    let resp = state.host.handle_command(cmd, make_ctx(&state)).await;
    Json(serde_json::to_value(&resp).unwrap()).into_response()
}

async fn declare_client(
    State(state): State<Arc<HttpState>>,
    Json(body): Json<serde_json::Value>,
) -> Response {
    let kind = match body
        .get("kind")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
    {
        Some(k) => k,
        None => {
            return error_response(
                StatusCode::BAD_REQUEST,
                ErrorCode::InvalidRequest,
                "missing or invalid 'kind' field".into(),
            );
        }
    };
    let label = body.get("label").and_then(|v| v.as_str()).map(String::from);
    let cmd = ClientCommand::DeclareClient {
        request_id: uuid::Uuid::new_v4().to_string(),
        kind,
        label,
    };
    let resp = state.host.handle_command(cmd, make_ctx(&state)).await;
    if let ServerMessage::ClientDeclared { client_id, .. } = &resp {
        *state.client_id.lock().unwrap() = Some(*client_id);
    }
    Json(serde_json::to_value(&resp).unwrap()).into_response()
}

async fn list_sessions(State(state): State<Arc<HttpState>>) -> Response {
    let cmd = ClientCommand::ListSessions {
        request_id: uuid::Uuid::new_v4().to_string(),
    };
    let resp = state.host.handle_command(cmd, make_ctx(&state)).await;
    Json(serde_json::to_value(&resp).unwrap()).into_response()
}

async fn create_session(
    State(state): State<Arc<HttpState>>,
    Json(body): Json<serde_json::Value>,
) -> Response {
    let surface = body
        .get("surface")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or(eggsec_runtime::RuntimeSurface::Unknown);
    let scope = body
        .get("scope")
        .and_then(|v| serde_json::from_value(v.clone()).ok());
    let labels = body
        .get("labels")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();
    let cmd = ClientCommand::CreateSession {
        request_id: uuid::Uuid::new_v4().to_string(),
        surface,
        scope,
        labels,
    };
    let resp = state.host.handle_command(cmd, make_ctx(&state)).await;
    Json(serde_json::to_value(&resp).unwrap()).into_response()
}

async fn get_snapshot(
    State(state): State<Arc<HttpState>>,
    Path(session_id): Path<String>,
) -> Response {
    let session_id = match session_id.parse() {
        Ok(id) => id,
        Err(_) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                ErrorCode::InvalidRequest,
                "invalid session_id".into(),
            );
        }
    };
    let cmd = ClientCommand::GetSnapshot {
        request_id: uuid::Uuid::new_v4().to_string(),
        session_id,
    };
    let resp = state.host.handle_command(cmd, make_ctx(&state)).await;
    Json(serde_json::to_value(&resp).unwrap()).into_response()
}

async fn submit_task(
    State(state): State<Arc<HttpState>>,
    Path(session_id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Response {
    let session_id = match session_id.parse() {
        Ok(id) => id,
        Err(_) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                ErrorCode::InvalidRequest,
                "invalid session_id".into(),
            );
        }
    };
    let request = match body
        .get("request")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
    {
        Some(r) => r,
        None => {
            return error_response(
                StatusCode::BAD_REQUEST,
                ErrorCode::InvalidRequest,
                "missing or invalid 'request' field".into(),
            );
        }
    };
    let cmd = ClientCommand::SubmitTask {
        request_id: uuid::Uuid::new_v4().to_string(),
        session_id,
        request,
    };
    let resp = state.host.handle_command(cmd, make_ctx(&state)).await;
    Json(serde_json::to_value(&resp).unwrap()).into_response()
}

async fn cancel_task(
    State(state): State<Arc<HttpState>>,
    Path((session_id, task_id)): Path<(String, String)>,
) -> Response {
    let session_id = match session_id.parse() {
        Ok(id) => id,
        Err(_) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                ErrorCode::InvalidRequest,
                "invalid session_id".into(),
            );
        }
    };
    let task_id = match task_id.parse() {
        Ok(id) => id,
        Err(_) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                ErrorCode::InvalidRequest,
                "invalid task_id".into(),
            );
        }
    };
    let cmd = ClientCommand::CancelTask {
        request_id: uuid::Uuid::new_v4().to_string(),
        session_id,
        task_id,
    };
    let resp = state.host.handle_command(cmd, make_ctx(&state)).await;
    Json(serde_json::to_value(&resp).unwrap()).into_response()
}

async fn cancel_active(
    State(state): State<Arc<HttpState>>,
    Path(session_id): Path<String>,
) -> Response {
    let session_id = match session_id.parse() {
        Ok(id) => id,
        Err(_) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                ErrorCode::InvalidRequest,
                "invalid session_id".into(),
            );
        }
    };
    let cmd = ClientCommand::CancelActive {
        request_id: uuid::Uuid::new_v4().to_string(),
        session_id,
    };
    let resp = state.host.handle_command(cmd, make_ctx(&state)).await;
    Json(serde_json::to_value(&resp).unwrap()).into_response()
}

async fn subscribe_events(
    State(state): State<Arc<HttpState>>,
    Path(session_id): Path<String>,
) -> Result<Sse<impl Stream<Item = Result<Event, std::convert::Infallible>>>, Response> {
    let session_id: eggsec_runtime::SessionId = session_id.parse().map_err(|_| {
        error_response(
            StatusCode::BAD_REQUEST,
            ErrorCode::InvalidRequest,
            "invalid session_id".into(),
        )
    })?;

    if state.host.runtime().snapshot(session_id).await.is_err() {
        return Err(error_response(
            StatusCode::OK,
            ErrorCode::SessionNotFound,
            "session not found".into(),
        ));
    }

    let mut receiver = state.host.runtime().subscribe().await;
    let sid = session_id;

    let event_stream = async_stream::stream! {
        while let Some(event) = receiver.recv().await {
            let event_session_id = crate::server::event_session_id(&event);
            if event_session_id == Some(&sid) {
                let msg = ServerMessage::RuntimeEvent {
                    session_id: sid,
                    event,
                };
                let data = serde_json::to_string(&msg).unwrap();
                yield Ok(Event::default().data(data));
            }
        }
    };

    Ok(Sse::new(event_stream).keep_alive(KeepAlive::default()))
}

async fn approve_policy(
    State(state): State<Arc<HttpState>>,
    Path((session_id, task_id)): Path<(String, String)>,
    Json(body): Json<serde_json::Value>,
) -> Response {
    let session_id = match session_id.parse() {
        Ok(id) => id,
        Err(_) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                ErrorCode::InvalidRequest,
                "invalid session_id".into(),
            );
        }
    };
    let task_id = match task_id.parse() {
        Ok(id) => id,
        Err(_) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                ErrorCode::InvalidRequest,
                "invalid task_id".into(),
            );
        }
    };
    let approved = body
        .get("approved")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let reason = body
        .get("reason")
        .and_then(|v| v.as_str())
        .map(String::from);
    let cmd = ClientCommand::ApprovePolicy {
        request_id: uuid::Uuid::new_v4().to_string(),
        session_id,
        task_id,
        approved,
        reason,
    };
    let resp = state.host.handle_command(cmd, make_ctx(&state)).await;
    Json(serde_json::to_value(&resp).unwrap()).into_response()
}

async fn close_session(
    State(state): State<Arc<HttpState>>,
    Path(session_id): Path<String>,
) -> Response {
    let session_id = match session_id.parse() {
        Ok(id) => id,
        Err(_) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                ErrorCode::InvalidRequest,
                "invalid session_id".into(),
            );
        }
    };
    let cmd = ClientCommand::CloseSession {
        request_id: uuid::Uuid::new_v4().to_string(),
        session_id,
    };
    let resp = state.host.handle_command(cmd, make_ctx(&state)).await;
    Json(serde_json::to_value(&resp).unwrap()).into_response()
}

async fn list_persisted_sessions(State(state): State<Arc<HttpState>>) -> Response {
    let cmd = ClientCommand::ListPersistedSessions {
        request_id: uuid::Uuid::new_v4().to_string(),
    };
    let resp = state.host.handle_command(cmd, make_ctx(&state)).await;
    Json(serde_json::to_value(&resp).unwrap()).into_response()
}

async fn get_persisted_snapshot(
    State(state): State<Arc<HttpState>>,
    Path(session_id): Path<String>,
) -> Response {
    let session_id = match session_id.parse() {
        Ok(id) => id,
        Err(_) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                ErrorCode::InvalidRequest,
                "invalid session_id".into(),
            );
        }
    };
    let cmd = ClientCommand::GetPersistedSnapshot {
        request_id: uuid::Uuid::new_v4().to_string(),
        session_id,
    };
    let resp = state.host.handle_command(cmd, make_ctx(&state)).await;
    Json(serde_json::to_value(&resp).unwrap()).into_response()
}

fn validate_bind_addr(addr: &SocketAddr, allow_public: bool) -> Result<(), String> {
    if !addr.ip().is_loopback() && !allow_public {
        return Err(format!(
            "refusing to bind to non-loopback address {}; \
             set allow_public_bind = true to override",
            addr
        ));
    }
    Ok(())
}

pub async fn run_http_server(
    host: Arc<DaemonHost>,
    config: HttpConfig,
    shutdown: CancellationToken,
) -> Result<(), crate::error::DaemonError> {
    let addr: SocketAddr = config
        .bind_addr
        .parse()
        .map_err(|e| crate::error::DaemonError::Protocol(format!("invalid bind address: {}", e)))?;

    validate_bind_addr(&addr, config.allow_public_bind).map_err(|e| {
        tracing::warn!("{}", e);
        crate::error::DaemonError::Protocol(e)
    })?;

    if addr.ip().is_loopback() {
        tracing::info!("HTTP API listening on {}", addr);
    } else {
        tracing::warn!(
            "HTTP API binding to non-loopback address {} — ensure network access is intentional",
            addr
        );
    }

    let state = Arc::new(HttpState {
        host,
        client_id: std::sync::Mutex::new(None),
    });

    let app = Router::new()
        .route("/health", get(health))
        .route("/capabilities", get(capabilities))
        .route("/clients/declare", post(declare_client))
        .route("/sessions", get(list_sessions).post(create_session))
        .route("/sessions/{session_id}/snapshot", get(get_snapshot))
        .route("/sessions/{session_id}/tasks", post(submit_task))
        .route(
            "/sessions/{session_id}/tasks/{task_id}/cancel",
            post(cancel_task),
        )
        .route("/sessions/{session_id}/cancel-active", post(cancel_active))
        .route("/sessions/{session_id}/events", get(subscribe_events))
        .route(
            "/sessions/{session_id}/policy/approve",
            post(approve_policy),
        )
        .route("/sessions/{session_id}", delete(close_session))
        .route("/sessions/persisted", get(list_persisted_sessions))
        .route(
            "/sessions/persisted/{session_id}",
            get(get_persisted_snapshot),
        )
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| crate::error::DaemonError::Protocol(format!("failed to bind: {}", e)))?;

    let shutdown_static: &'static CancellationToken = Box::leak(Box::new(shutdown));

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_static.cancelled())
        .await
        .map_err(|e| crate::error::DaemonError::Protocol(format!("server error: {}", e)))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DaemonConfig;
    use crate::host::DaemonHost;
    use crate::store::NoopStore;
    use eggsec_runtime::{
        CancellationToken, RuntimeEventSink, RuntimeTaskExecutor, TaskId, TaskOutcome,
    };
    use std::future::Future;
    use std::pin::Pin;

    struct TestExecutor;

    impl RuntimeTaskExecutor for TestExecutor {
        fn execute(
            &self,
            _task_id: TaskId,
            _request: eggsec_runtime::RunRequest,
            _sink: RuntimeEventSink,
            _cancel: CancellationToken,
        ) -> Pin<
            Box<
                dyn Future<Output = Result<TaskOutcome, eggsec_runtime::RuntimeError>>
                    + Send
                    + 'static,
            >,
        > {
            Box::pin(async { Ok(TaskOutcome::Text("test-result".into())) })
        }
    }

    async fn start_server() -> (String, CancellationToken) {
        let config = DaemonConfig::default();
        let host = Arc::new(DaemonHost::new(
            config,
            TestExecutor,
            crate::store::noop_store(),
        ));
        let shutdown = CancellationToken::new();

        let host_clone = host.clone();
        let shutdown_static: &'static CancellationToken = Box::leak(Box::new(shutdown.clone()));

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let bind_addr = addr.to_string();

        tokio::spawn(async move {
            let state = Arc::new(HttpState {
                host: host_clone,
                client_id: std::sync::Mutex::new(None),
            });
            let app = Router::new()
                .route("/health", get(health))
                .route("/capabilities", get(capabilities))
                .route("/clients/declare", post(declare_client))
                .route("/sessions", get(list_sessions).post(create_session))
                .route("/sessions/{session_id}/snapshot", get(get_snapshot))
                .route("/sessions/{session_id}/tasks", post(submit_task))
                .route(
                    "/sessions/{session_id}/tasks/{task_id}/cancel",
                    post(cancel_task),
                )
                .route("/sessions/{session_id}/cancel-active", post(cancel_active))
                .route("/sessions/{session_id}/events", get(subscribe_events))
                .route(
                    "/sessions/{session_id}/policy/approve",
                    post(approve_policy),
                )
                .route("/sessions/{session_id}", delete(close_session))
                .route("/sessions/persisted", get(list_persisted_sessions))
                .route(
                    "/sessions/persisted/{session_id}",
                    get(get_persisted_snapshot),
                )
                .with_state(state);

            axum::serve(listener, app)
                .with_graceful_shutdown(shutdown_static.cancelled())
                .await
                .unwrap();
        });

        (bind_addr, shutdown)
    }

    #[tokio::test]
    async fn http_health() {
        let (addr, shutdown) = start_server().await;
        let resp = reqwest::get(format!("http://{}/health", addr))
            .await
            .unwrap();
        assert!(resp.status().is_success());
        let body: serde_json::Value = resp.json().await.unwrap();
        assert_eq!(body["type"], "Health");
        assert_eq!(body["status"], "ok");
        shutdown.cancel();
    }

    #[tokio::test]
    async fn http_capabilities() {
        let (addr, shutdown) = start_server().await;
        let resp = reqwest::get(format!("http://{}/capabilities", addr))
            .await
            .unwrap();
        assert!(resp.status().is_success());
        let body: serde_json::Value = resp.json().await.unwrap();
        assert_eq!(body["type"], "Capabilities");
        assert!(body["capabilities"]["runtime"]["task_kinds"].is_array());
        shutdown.cancel();
    }

    #[tokio::test]
    async fn http_declare_client() {
        let (addr, shutdown) = start_server().await;
        let client = reqwest::Client::new();
        let resp = client
            .post(format!("http://{}/clients/declare", addr))
            .json(&serde_json::json!({"kind": {"type": "Tui"}, "label": "test"}))
            .send()
            .await
            .unwrap();
        assert!(resp.status().is_success());
        let body: serde_json::Value = resp.json().await.unwrap();
        assert_eq!(body["type"], "ClientDeclared");
        assert!(body["client_id"].is_string());
        shutdown.cancel();
    }

    #[tokio::test]
    async fn http_create_and_list_sessions() {
        let (addr, shutdown) = start_server().await;
        let client = reqwest::Client::new();

        let resp = client
            .post(format!("http://{}/sessions", addr))
            .json(&serde_json::json!({}))
            .send()
            .await
            .unwrap();
        assert!(resp.status().is_success());
        let body: serde_json::Value = resp.json().await.unwrap();
        assert_eq!(body["type"], "SessionCreated");
        let session_id = body["session_id"].as_str().unwrap();

        let resp = client
            .get(format!("http://{}/sessions", addr))
            .send()
            .await
            .unwrap();
        assert!(resp.status().is_success());
        let body: serde_json::Value = resp.json().await.unwrap();
        assert_eq!(body["type"], "Sessions");
        let sessions = body["sessions"].as_array().unwrap();
        assert!(sessions.iter().any(|s| s["session_id"] == session_id));
        shutdown.cancel();
    }

    #[tokio::test]
    async fn http_get_snapshot() {
        let (addr, shutdown) = start_server().await;
        let client = reqwest::Client::new();

        let _ = client
            .post(format!("http://{}/clients/declare", addr))
            .json(&serde_json::json!({"kind": {"type": "Tui"}, "label": "test"}))
            .send()
            .await
            .unwrap();

        let resp = client
            .post(format!("http://{}/sessions", addr))
            .json(&serde_json::json!({}))
            .send()
            .await
            .unwrap();
        let body: serde_json::Value = resp.json().await.unwrap();
        let session_id = body["session_id"].as_str().unwrap();

        let resp = client
            .get(format!("http://{}/sessions/{}/snapshot", addr, session_id))
            .send()
            .await
            .unwrap();
        assert!(resp.status().is_success());
        let body: serde_json::Value = resp.json().await.unwrap();
        assert_eq!(body["type"], "Snapshot");
        assert_eq!(body["snapshot"]["session_id"], session_id);
        shutdown.cancel();
    }

    #[tokio::test]
    async fn http_close_session() {
        let (addr, shutdown) = start_server().await;
        let client = reqwest::Client::new();

        let _ = client
            .post(format!("http://{}/clients/declare", addr))
            .json(&serde_json::json!({"kind": {"type": "Tui"}, "label": "test"}))
            .send()
            .await
            .unwrap();

        let resp = client
            .post(format!("http://{}/sessions", addr))
            .json(&serde_json::json!({}))
            .send()
            .await
            .unwrap();
        let body: serde_json::Value = resp.json().await.unwrap();
        let session_id = body["session_id"].as_str().unwrap();

        let resp = client
            .delete(format!("http://{}/sessions/{}", addr, session_id))
            .send()
            .await
            .unwrap();
        assert!(resp.status().is_success());
        let body: serde_json::Value = resp.json().await.unwrap();
        assert_eq!(body["type"], "SessionClosed");
        shutdown.cancel();
    }

    #[tokio::test]
    async fn http_session_not_found() {
        let (addr, shutdown) = start_server().await;
        let client = reqwest::Client::new();

        let _ = client
            .post(format!("http://{}/clients/declare", addr))
            .json(&serde_json::json!({"kind": {"type": "Tui"}, "label": "test"}))
            .send()
            .await
            .unwrap();

        let fake_id = eggsec_runtime::SessionId::new();
        let resp = client
            .get(format!("http://{}/sessions/{}/snapshot", addr, fake_id))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body: serde_json::Value = resp.json().await.unwrap();
        assert_eq!(body["type"], "Error");
        assert_eq!(body["code"]["type"], "SessionNotFound");
        shutdown.cancel();
    }

    #[tokio::test]
    async fn http_invalid_session_id() {
        let (addr, shutdown) = start_server().await;
        let resp = reqwest::get(format!("http://{}/sessions/not-a-uuid/snapshot", addr))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        shutdown.cancel();
    }

    #[tokio::test]
    async fn validate_bind_addr_loopback() {
        let addr: SocketAddr = "127.0.0.1:9876".parse().unwrap();
        assert!(validate_bind_addr(&addr, false).is_ok());
    }

    #[tokio::test]
    async fn validate_bind_addr_public_denied() {
        let addr: SocketAddr = "0.0.0.0:9876".parse().unwrap();
        assert!(validate_bind_addr(&addr, false).is_err());
    }

    #[tokio::test]
    async fn validate_bind_addr_public_allowed() {
        let addr: SocketAddr = "0.0.0.0:9876".parse().unwrap();
        assert!(validate_bind_addr(&addr, true).is_ok());
    }

    #[tokio::test]
    async fn http_default_config() {
        let config = HttpConfig::default();
        assert_eq!(config.bind_addr, "127.0.0.1:9876");
        assert!(!config.require_auth);
        assert!(!config.allow_public_bind);
    }

    #[tokio::test]
    async fn http_submit_task() {
        let (addr, shutdown) = start_server().await;
        let client = reqwest::Client::new();

        let _ = client
            .post(format!("http://{}/clients/declare", addr))
            .json(&serde_json::json!({"kind": {"type": "Tui"}, "label": "test"}))
            .send()
            .await
            .unwrap();

        let resp = client
            .post(format!("http://{}/sessions", addr))
            .json(&serde_json::json!({}))
            .send()
            .await
            .unwrap();
        let body: serde_json::Value = resp.json().await.unwrap();
        let session_id = body["session_id"].as_str().unwrap();

        let resp = client
            .post(format!("http://{}/sessions/{}/tasks", addr, session_id))
            .json(&serde_json::json!({
                "request": {
                    "task_kind": {"kind": "PortScan", "params": {"target": "10.0.0.1"}},
                    "surface": {"CliManual": null},
                    "labels": []
                }
            }))
            .send()
            .await
            .unwrap();
        assert!(resp.status().is_success());
        let body: serde_json::Value = resp.json().await.unwrap();
        assert_eq!(body["type"], "TaskSubmitted");
        shutdown.cancel();
    }

    #[tokio::test]
    async fn http_cancel_active() {
        let (addr, shutdown) = start_server().await;
        let client = reqwest::Client::new();

        let _ = client
            .post(format!("http://{}/clients/declare", addr))
            .json(&serde_json::json!({"kind": {"type": "Tui"}, "label": "test"}))
            .send()
            .await
            .unwrap();

        let resp = client
            .post(format!("http://{}/sessions", addr))
            .json(&serde_json::json!({}))
            .send()
            .await
            .unwrap();
        let body: serde_json::Value = resp.json().await.unwrap();
        let session_id = body["session_id"].as_str().unwrap();

        let resp = client
            .post(format!(
                "http://{}/sessions/{}/cancel-active",
                addr, session_id
            ))
            .send()
            .await
            .unwrap();
        assert!(resp.status().is_success());
        let body: serde_json::Value = resp.json().await.unwrap();
        assert_eq!(body["type"], "Ok");
        shutdown.cancel();
    }

    #[tokio::test]
    async fn http_submit_without_declaration_denied() {
        let (addr, shutdown) = start_server().await;
        let client = reqwest::Client::new();

        let fake_session = eggsec_runtime::SessionId::new();
        let resp = client
            .post(format!("http://{}/sessions/{}/tasks", addr, fake_session))
            .json(&serde_json::json!({
                "request": {
                    "task_kind": {"kind": "PortScan", "params": {"target": "10.0.0.1"}},
                    "surface": {"CliManual": null},
                    "labels": []
                }
            }))
            .send()
            .await
            .unwrap();
        let body: serde_json::Value = resp.json().await.unwrap();
        assert_eq!(body["type"], "Error");
        assert_eq!(body["code"]["type"], "ClientNotDeclared");
        shutdown.cancel();
    }

    #[tokio::test]
    async fn http_sse_event_delivery() {
        let (addr, shutdown) = start_server().await;
        let client = reqwest::Client::new();

        let _ = client
            .post(format!("http://{}/clients/declare", addr))
            .json(&serde_json::json!({"kind": {"type": "Tui"}, "label": "sse-test"}))
            .send()
            .await
            .unwrap();

        let resp = client
            .post(format!("http://{}/sessions", addr))
            .json(&serde_json::json!({}))
            .send()
            .await
            .unwrap();
        let body: serde_json::Value = resp.json().await.unwrap();
        let session_id = body["session_id"].as_str().unwrap();

        let resp = client
            .get(format!("http://{}/sessions/{}/events", addr, session_id))
            .header("Accept", "text/event-stream")
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers().get("content-type").unwrap(),
            "text/event-stream"
        );
        shutdown.cancel();
    }

    #[tokio::test]
    async fn http_list_persisted_sessions() {
        let (addr, shutdown) = start_server().await;
        let client = reqwest::Client::new();

        let _ = client
            .post(format!("http://{}/clients/declare", addr))
            .json(&serde_json::json!({"kind": {"type": "Tui"}, "label": "test"}))
            .send()
            .await
            .unwrap();

        let resp = client
            .get(format!("http://{}/sessions/persisted", addr))
            .send()
            .await
            .unwrap();
        assert!(resp.status().is_success());
        let body: serde_json::Value = resp.json().await.unwrap();
        assert_eq!(body["type"], "PersistedSessions");
        assert!(body["sessions"].is_array());
        shutdown.cancel();
    }

    #[tokio::test]
    async fn http_get_persisted_snapshot() {
        let (addr, shutdown) = start_server().await;
        let client = reqwest::Client::new();

        let _ = client
            .post(format!("http://{}/clients/declare", addr))
            .json(&serde_json::json!({"kind": {"type": "Tui"}, "label": "test"}))
            .send()
            .await
            .unwrap();

        let fake_id = eggsec_runtime::SessionId::new();
        let resp = client
            .get(format!("http://{}/sessions/persisted/{}", addr, fake_id))
            .send()
            .await
            .unwrap();
        assert!(resp.status().is_success());
        let body: serde_json::Value = resp.json().await.unwrap();
        assert_eq!(body["type"], "PersistedSnapshot");
        assert!(body["snapshot"].is_null());
        shutdown.cancel();
    }

    #[tokio::test]
    async fn http_get_persisted_snapshot_invalid_id() {
        let (addr, shutdown) = start_server().await;
        let resp = reqwest::get(format!("http://{}/sessions/persisted/not-a-uuid", addr))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        shutdown.cancel();
    }
}
