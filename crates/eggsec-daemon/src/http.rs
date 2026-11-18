//! Dormant HTTP/SSE adapter: not wired or a supported surface. An ADR is
//! required before exposing or connecting this adapter to a listener.

use std::fmt;
use std::net::{AddrParseError, SocketAddr};
use std::sync::Arc;

use axum::extract::{FromRequestParts, Path, Request, State};
use axum::http::request::Parts;
use axum::http::uri::Authority;
use axum::http::{header, HeaderMap, StatusCode, Uri};
use axum::middleware::{from_fn_with_state, Next};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use futures::stream::Stream;
use subtle::ConstantTimeEq;
use thiserror::Error;
use tokio_util::sync::CancellationToken;

use crate::host::DaemonHost;
use crate::protocol::{
    ClientCommand, DaemonRequestContext, ErrorCode, ServerMessage, TransportKind,
};

/// HTTP header carrying the caller-declared client id.
pub const CLIENT_ID_HEADER: &str = "x-eggsec-client-id";

const AUTHORIZATION_HEADER: &str = "authorization";

/// Errors returned when constructing HTTP authentication or bind config.
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("invalid HTTP bind address")]
    InvalidBindAddress(#[source] AddrParseError),
    #[error("HTTP bearer token must not be empty or contain whitespace")]
    InvalidToken,
    #[error("LoopbackTrust authentication is only allowed for loopback binds")]
    LoopbackTrustRequiresLoopback,
    #[error("non-loopback HTTP bind requires allow_public_bind and token authentication")]
    PublicBindRequiresOptIn,
}

/// A bearer credential. Its value is never included in `Debug` or `Display`.
pub struct AuthToken(String);

impl AuthToken {
    /// Construct a bearer token accepted by the HTTP adapter.
    pub fn new(token: impl Into<String>) -> Result<Self, ConfigError> {
        let token = token.into();
        if token.is_empty() || token.bytes().any(|byte| byte.is_ascii_whitespace()) {
            return Err(ConfigError::InvalidToken);
        }
        Ok(Self(token))
    }

    fn matches(&self, presented: &[u8]) -> bool {
        bool::from(self.0.as_bytes().ct_eq(presented))
    }
}

impl fmt::Debug for AuthToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("AuthToken([REDACTED])")
    }
}

/// Authentication policy for the dormant HTTP adapter.
#[derive(Debug)]
pub enum AuthMode {
    /// A bearer token is required on every route except `/health` and `/capabilities`.
    Token(AuthToken),
    /// Trust the loopback boundary; this mode cannot bind a non-loopback address.
    LoopbackTrust,
}

/// Validated configuration for the dormant HTTP adapter.
#[derive(Debug)]
pub struct HttpConfig {
    bind_addr: SocketAddr,
    auth: AuthMode,
    allow_public_bind: bool,
}

impl HttpConfig {
    /// Validate bind intent and authentication policy before the adapter starts.
    pub fn new(
        bind_addr: impl Into<String>,
        auth: AuthMode,
        allow_public_bind: bool,
    ) -> Result<Self, ConfigError> {
        let bind_addr = bind_addr
            .into()
            .parse::<SocketAddr>()
            .map_err(ConfigError::InvalidBindAddress)?;
        if bind_addr.ip().is_loopback() {
            // Both explicit token auth and loopback trust are valid here.
        } else {
            match &auth {
                AuthMode::LoopbackTrust => {
                    return Err(ConfigError::LoopbackTrustRequiresLoopback);
                }
                AuthMode::Token(_) if !allow_public_bind => {
                    return Err(ConfigError::PublicBindRequiresOptIn);
                }
                AuthMode::Token(_) => {}
            }
        }
        Ok(Self {
            bind_addr,
            auth,
            allow_public_bind,
        })
    }
}

struct HttpState {
    host: Arc<DaemonHost>,
    bind_addr: SocketAddr,
    auth: AuthMode,
}

/// Per-request client identity derived from the `X-Eggsec-Client-Id` header.
#[derive(Clone)]
struct AuthenticatedClientId(Option<eggsec_runtime::ClientId>);

impl<S> FromRequestParts<S> for AuthenticatedClientId
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let value = parts
            .headers
            .get(CLIENT_ID_HEADER)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.trim().to_string());
        let parsed = value.and_then(|raw| raw.parse::<eggsec_runtime::ClientId>().ok());
        Ok(Self(parsed))
    }
}

fn make_ctx(client_id: Option<eggsec_runtime::ClientId>) -> DaemonRequestContext {
    DaemonRequestContext {
        client_id,
        peer: None,
        transport: TransportKind::LoopbackHttp,
    }
}

fn auth_required_response() -> Response {
    error_response(
        StatusCode::UNAUTHORIZED,
        ErrorCode::InvalidRequest,
        "authentication required".to_string(),
    )
}

fn enforce_client_identity(auth: &AuthenticatedClientId) -> Option<Response> {
    auth.0.is_none().then(auth_required_response)
}

fn reject_request(status: StatusCode, route: &str, rule: &'static str) -> Response {
    tracing::warn!(route, rule, "rejected HTTP request");
    error_response(
        status,
        ErrorCode::InvalidRequest,
        "request rejected by HTTP adapter security policy".to_string(),
    )
}

fn is_session_route(path: &str) -> bool {
    path == "/sessions" || path.starts_with("/sessions/")
}

fn has_valid_client_id(headers: &HeaderMap) -> bool {
    headers
        .get(CLIENT_ID_HEADER)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.trim().parse::<eggsec_runtime::ClientId>().is_ok())
}

fn bearer_token(headers: &HeaderMap) -> Option<&[u8]> {
    let value = headers.get(AUTHORIZATION_HEADER)?.to_str().ok()?;
    let (scheme, token) = value.split_once(' ')?;
    if !scheme.eq_ignore_ascii_case("Bearer")
        || token.is_empty()
        || token.bytes().any(|byte| byte.is_ascii_whitespace())
    {
        return None;
    }
    Some(token.as_bytes())
}

fn authority_matches(authority: &Authority, bind_addr: SocketAddr, default_port: u16) -> bool {
    if authority.as_str().contains('@') {
        return false;
    }
    let port = match authority.port() {
        Some(_) => match authority.port_u16() {
            Some(port) => port,
            None => return false,
        },
        None => default_port,
    };
    if port != bind_addr.port() {
        return false;
    }

    let host = authority.host();
    let host = host
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
        .unwrap_or(host);
    let bound_host = bind_addr.ip().to_string();
    if host.eq_ignore_ascii_case(&bound_host) {
        return true;
    }
    if !bind_addr.ip().is_loopback() || !host.eq_ignore_ascii_case("localhost") {
        return false;
    }
    matches!(
        bind_addr.ip(),
        std::net::IpAddr::V4(ip) if ip == std::net::Ipv4Addr::LOCALHOST
    ) || matches!(
        bind_addr.ip(),
        std::net::IpAddr::V6(ip) if ip == std::net::Ipv6Addr::LOCALHOST
    )
}

fn host_matches(headers: &HeaderMap, bind_addr: SocketAddr) -> bool {
    let mut hosts = headers.get_all(header::HOST).iter();
    let Some(host) = hosts.next() else {
        return false;
    };
    if hosts.next().is_some() {
        return false;
    }
    host.to_str()
        .ok()
        .and_then(|value| value.parse::<Authority>().ok())
        .is_some_and(|authority| authority_matches(&authority, bind_addr, 80))
}

fn origin_matches(headers: &HeaderMap, bind_addr: SocketAddr) -> bool {
    let mut origins = headers.get_all(header::ORIGIN).iter();
    let Some(origin) = origins.next() else {
        return true;
    };
    if origins.next().is_some() {
        return false;
    }
    let Some(origin) = origin
        .to_str()
        .ok()
        .and_then(|value| value.parse::<Uri>().ok())
    else {
        return false;
    };
    let Some(scheme) = origin.scheme_str() else {
        return false;
    };
    // This adapter serves plain HTTP only; matching authority is insufficient
    // if a browser Origin claims the HTTPS scheme for the same host and port.
    if !scheme.eq_ignore_ascii_case("http") {
        return false;
    }
    let default_port = 80;
    if (!origin.path().is_empty() && origin.path() != "/") || origin.query().is_some() {
        return false;
    }
    origin
        .authority()
        .is_some_and(|authority| authority_matches(authority, bind_addr, default_port))
}

async fn enforce_request_security(
    State(state): State<Arc<HttpState>>,
    request: Request,
    next: Next,
) -> Response {
    let route = request.uri().path();
    if !host_matches(request.headers(), state.bind_addr) {
        return reject_request(StatusCode::FORBIDDEN, route, "host");
    }
    if !origin_matches(request.headers(), state.bind_addr) {
        return reject_request(StatusCode::FORBIDDEN, route, "origin");
    }

    if !matches!(route, "/health" | "/capabilities") {
        if let AuthMode::Token(token) = &state.auth {
            if !bearer_token(request.headers()).is_some_and(|presented| token.matches(presented)) {
                return reject_request(StatusCode::UNAUTHORIZED, route, "bearer_token");
            }
        }
    }
    if is_session_route(route) && !has_valid_client_id(request.headers()) {
        return reject_request(StatusCode::UNAUTHORIZED, route, "client_id");
    }
    next.run(request).await
}

fn error_response(status: StatusCode, code: ErrorCode, message: String) -> Response {
    (
        status,
        Json(serde_json::json!({"code": code, "message": message})),
    )
        .into_response()
}

/// Serialize a `ServerMessage` for an HTTP response.
///
/// Falls back to a 500 response instead of panicking when serialization
/// fails, so one malformed message cannot cascade into handler-wide panics.
fn json_response(msg: &ServerMessage) -> Response {
    match serde_json::to_value(msg) {
        Ok(value) => Json(value).into_response(),
        Err(error) => {
            tracing::warn!(?error, "failed to serialize daemon response");
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                ErrorCode::Internal,
                "response serialization failed".to_string(),
            )
        }
    }
}

async fn health(State(state): State<Arc<HttpState>>) -> Response {
    let cmd = ClientCommand::Health {
        request_id: uuid::Uuid::new_v4().to_string(),
    };
    let resp = state.host.handle_command(cmd, make_ctx(None)).await;
    json_response(&resp)
}

async fn capabilities(State(state): State<Arc<HttpState>>) -> Response {
    let cmd = ClientCommand::Capabilities {
        request_id: uuid::Uuid::new_v4().to_string(),
    };
    let resp = state.host.handle_command(cmd, make_ctx(None)).await;
    json_response(&resp)
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
    let resp = state.host.handle_command(cmd, make_ctx(None)).await;
    json_response(&resp)
}

async fn list_sessions(
    State(state): State<Arc<HttpState>>,
    auth: AuthenticatedClientId,
) -> Response {
    if let Some(resp) = enforce_client_identity(&auth) {
        return resp;
    }
    let cmd = ClientCommand::ListSessions {
        request_id: uuid::Uuid::new_v4().to_string(),
    };
    let resp = state.host.handle_command(cmd, make_ctx(auth.0)).await;
    json_response(&resp)
}

async fn create_session(
    State(state): State<Arc<HttpState>>,
    auth: AuthenticatedClientId,
    Json(body): Json<serde_json::Value>,
) -> Response {
    if let Some(resp) = enforce_client_identity(&auth) {
        return resp;
    }
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
    let resp = state.host.handle_command(cmd, make_ctx(auth.0)).await;
    json_response(&resp)
}

async fn get_snapshot(
    State(state): State<Arc<HttpState>>,
    auth: AuthenticatedClientId,
    Path(session_id): Path<String>,
) -> Response {
    if let Some(resp) = enforce_client_identity(&auth) {
        return resp;
    }
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
    let resp = state.host.handle_command(cmd, make_ctx(auth.0)).await;
    json_response(&resp)
}

async fn submit_task(
    State(state): State<Arc<HttpState>>,
    auth: AuthenticatedClientId,
    Path(session_id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Response {
    if let Some(resp) = enforce_client_identity(&auth) {
        return resp;
    }
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
    let resp = state.host.handle_command(cmd, make_ctx(auth.0)).await;
    json_response(&resp)
}

async fn get_task_result(
    State(state): State<Arc<HttpState>>,
    auth: AuthenticatedClientId,
    Path((session_id, task_id)): Path<(String, String)>,
) -> Response {
    if let Some(resp) = enforce_client_identity(&auth) {
        return resp;
    }
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
    let cmd = ClientCommand::GetTaskResult {
        request_id: uuid::Uuid::new_v4().to_string(),
        session_id,
        task_id,
    };
    let resp = state.host.handle_command(cmd, make_ctx(auth.0)).await;
    json_response(&resp)
}

async fn cancel_task(
    State(state): State<Arc<HttpState>>,
    auth: AuthenticatedClientId,
    Path((session_id, task_id)): Path<(String, String)>,
) -> Response {
    if let Some(resp) = enforce_client_identity(&auth) {
        return resp;
    }
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
    let resp = state.host.handle_command(cmd, make_ctx(auth.0)).await;
    json_response(&resp)
}

async fn cancel_active(
    State(state): State<Arc<HttpState>>,
    auth: AuthenticatedClientId,
    Path(session_id): Path<String>,
) -> Response {
    if let Some(resp) = enforce_client_identity(&auth) {
        return resp;
    }
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
    let resp = state.host.handle_command(cmd, make_ctx(auth.0)).await;
    json_response(&resp)
}

async fn subscribe_events(
    State(state): State<Arc<HttpState>>,
    auth: AuthenticatedClientId,
    Path(session_id): Path<String>,
) -> Result<Sse<impl Stream<Item = Result<Event, std::convert::Infallible>>>, Response> {
    if let Some(resp) = enforce_client_identity(&auth) {
        return Err(resp);
    }
    let session_id: eggsec_runtime::SessionId = session_id.parse().map_err(|_| {
        error_response(
            StatusCode::BAD_REQUEST,
            ErrorCode::InvalidRequest,
            "invalid session_id".into(),
        )
    })?;

    // Mirror the Unix-socket path: require a declared client and enforce the
    // host-level session access/role check before streaming any events.
    if let Err((code, message)) = state
        .host
        .check_event_stream_access(session_id, &make_ctx(auth.0))
        .await
    {
        return Err(error_response(StatusCode::OK, code, message));
    }

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
                let data = match serde_json::to_string(&msg) {
                    Ok(data) => data,
                    Err(error) => {
                        tracing::warn!(?error, "failed to serialize SSE runtime event");
                        continue;
                    }
                };
                yield Ok(Event::default().data(data));
            }
        }
    };

    Ok(Sse::new(event_stream).keep_alive(KeepAlive::default()))
}

async fn approve_policy(
    State(state): State<Arc<HttpState>>,
    auth: AuthenticatedClientId,
    Path((session_id, task_id)): Path<(String, String)>,
    Json(body): Json<serde_json::Value>,
) -> Response {
    if let Some(resp) = enforce_client_identity(&auth) {
        return resp;
    }
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
    let resp = state.host.handle_command(cmd, make_ctx(auth.0)).await;
    json_response(&resp)
}

async fn close_session(
    State(state): State<Arc<HttpState>>,
    auth: AuthenticatedClientId,
    Path(session_id): Path<String>,
) -> Response {
    if let Some(resp) = enforce_client_identity(&auth) {
        return resp;
    }
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
    let resp = state.host.handle_command(cmd, make_ctx(auth.0)).await;
    json_response(&resp)
}

async fn list_persisted_sessions(
    State(state): State<Arc<HttpState>>,
    auth: AuthenticatedClientId,
) -> Response {
    if let Some(resp) = enforce_client_identity(&auth) {
        return resp;
    }
    let cmd = ClientCommand::ListPersistedSessions {
        request_id: uuid::Uuid::new_v4().to_string(),
    };
    let resp = state.host.handle_command(cmd, make_ctx(auth.0)).await;
    json_response(&resp)
}

async fn get_persisted_snapshot(
    State(state): State<Arc<HttpState>>,
    auth: AuthenticatedClientId,
    Path(session_id): Path<String>,
) -> Response {
    if let Some(resp) = enforce_client_identity(&auth) {
        return resp;
    }
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
    let resp = state.host.handle_command(cmd, make_ctx(auth.0)).await;
    json_response(&resp)
}

fn http_router(state: Arc<HttpState>) -> Router {
    Router::new()
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
        .route(
            "/sessions/{session_id}/tasks/{task_id}",
            get(get_task_result),
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
        .layer(from_fn_with_state(state.clone(), enforce_request_security))
        .with_state(state)
}

pub async fn run_http_server(
    host: Arc<DaemonHost>,
    config: HttpConfig,
    shutdown: CancellationToken,
) -> Result<(), crate::error::DaemonError> {
    let HttpConfig {
        bind_addr,
        auth,
        allow_public_bind,
    } = config;

    if bind_addr.ip().is_loopback() {
        tracing::info!("HTTP API listening on {}", bind_addr);
    } else {
        debug_assert!(allow_public_bind);
        tracing::warn!(
            "HTTP API binding to non-loopback address {} — ensure network access is intentional",
            bind_addr
        );
    }

    let listener = tokio::net::TcpListener::bind(bind_addr)
        .await
        .map_err(|e| crate::error::DaemonError::Protocol(format!("failed to bind: {}", e)))?;
    let bound_addr = listener.local_addr().map_err(|e| {
        crate::error::DaemonError::Protocol(format!("failed to read bound address: {}", e))
    })?;
    let state = Arc::new(HttpState {
        host,
        bind_addr: bound_addr,
        auth,
    });
    let app = http_router(state);

    // Leaked intentionally: axum's graceful shutdown future requires 'static
    // and the token lives for the remainder of the process. One leak per HTTP
    // server start; run_http_server is called at most once per daemon.
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
    use axum::body::Body;
    use eggsec_runtime::{
        CancellationToken, RuntimeEventSink, RuntimeTaskExecutor, TaskId, TaskOutcome,
    };
    use std::future::Future;
    use std::pin::Pin;
    use tower::ServiceExt;

    struct TestExecutor;

    impl RuntimeTaskExecutor for TestExecutor {
        fn execute(
            &self,
            _task_id: TaskId,
            _request: eggsec_runtime::RunRequest,
            _context: eggsec_runtime::RuntimeExecutionContext,
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
        // reqwest's dev-dependency uses `rustls-no-provider`; install the
        // ring provider once per test process before any client is built.
        static CRYPTO_PROVIDER: std::sync::Once = std::sync::Once::new();
        CRYPTO_PROVIDER.call_once(|| {
            if rustls::crypto::ring::default_provider()
                .install_default()
                .is_err()
            {
                tracing::debug!("rustls crypto provider already installed");
            }
        });

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
                bind_addr: addr,
                auth: AuthMode::LoopbackTrust,
            });
            let app = http_router(state);

            axum::serve(listener, app)
                .with_graceful_shutdown(shutdown_static.cancelled())
                .await
                .unwrap();
        });

        (bind_addr, shutdown)
    }

    async fn request_router(
        auth: AuthMode,
        method: axum::http::Method,
        uri: &str,
        headers: &[(&str, &str)],
        body: Body,
    ) -> Response {
        let host = Arc::new(DaemonHost::new(
            DaemonConfig::default(),
            TestExecutor,
            crate::store::noop_store(),
        ));
        let bind_addr = "127.0.0.1:9876".parse().unwrap();
        let state = Arc::new(HttpState {
            host,
            bind_addr,
            auth,
        });
        let mut builder = Request::builder().method(method).uri(uri);
        for (name, value) in headers {
            builder = builder.header(*name, *value);
        }
        http_router(state)
            .oneshot(builder.body(body).unwrap())
            .await
            .unwrap()
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

    async fn declare_client_id(addr: &str, label: &str) -> String {
        let client = reqwest::Client::new();
        let resp = client
            .post(format!("http://{}/clients/declare", addr))
            .json(&serde_json::json!({"kind": {"type": "Tui"}, "label": label}))
            .send()
            .await
            .unwrap();
        assert!(resp.status().is_success());
        let body: serde_json::Value = resp.json().await.unwrap();
        assert_eq!(body["type"], "ClientDeclared");
        body["client_id"].as_str().unwrap().to_string()
    }

    fn auth_header(client_id: &str) -> (reqwest::header::HeaderName, reqwest::header::HeaderValue) {
        (
            reqwest::header::HeaderName::from_static(CLIENT_ID_HEADER),
            reqwest::header::HeaderValue::from_str(client_id).unwrap(),
        )
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
        let client_id = declare_client_id(&addr, "test").await;
        let (name, value) = auth_header(&client_id);
        let client = reqwest::Client::new();

        let resp = client
            .post(format!("http://{}/sessions", addr))
            .header(name.clone(), value.clone())
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
            .header(name, value)
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
        let client_id = declare_client_id(&addr, "test").await;
        let (name, value) = auth_header(&client_id);
        let client = reqwest::Client::new();

        let resp = client
            .post(format!("http://{}/sessions", addr))
            .header(name.clone(), value.clone())
            .json(&serde_json::json!({}))
            .send()
            .await
            .unwrap();
        let body: serde_json::Value = resp.json().await.unwrap();
        let session_id = body["session_id"].as_str().unwrap();

        let resp = client
            .get(format!("http://{}/sessions/{}/snapshot", addr, session_id))
            .header(name, value)
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
        let client_id = declare_client_id(&addr, "test").await;
        let (name, value) = auth_header(&client_id);
        let client = reqwest::Client::new();

        let resp = client
            .post(format!("http://{}/sessions", addr))
            .header(name.clone(), value.clone())
            .json(&serde_json::json!({}))
            .send()
            .await
            .unwrap();
        let body: serde_json::Value = resp.json().await.unwrap();
        let session_id = body["session_id"].as_str().unwrap();

        let resp = client
            .delete(format!("http://{}/sessions/{}", addr, session_id))
            .header(name, value)
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
        let client_id = declare_client_id(&addr, "test").await;
        let (name, value) = auth_header(&client_id);
        let client = reqwest::Client::new();

        let fake_id = eggsec_runtime::SessionId::new();
        let resp = client
            .get(format!("http://{}/sessions/{}/snapshot", addr, fake_id))
            .header(name, value)
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
        let client_id = declare_client_id(&addr, "invalid-session-id").await;
        let (name, value) = auth_header(&client_id);
        let resp = reqwest::Client::new()
            .get(format!("http://{}/sessions/not-a-uuid/snapshot", addr))
            .header(name, value)
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        shutdown.cancel();
    }

    #[test]
    fn http_config_admission_matrix_and_redaction() {
        assert!(HttpConfig::new("127.0.0.1:9876", AuthMode::LoopbackTrust, false).is_ok());
        assert!(HttpConfig::new(
            "[::1]:9876",
            AuthMode::Token(AuthToken::new("local-token").unwrap()),
            false,
        )
        .is_ok());
        assert!(HttpConfig::new("0.0.0.0:9876", AuthMode::LoopbackTrust, true,).is_err());
        assert!(HttpConfig::new(
            "[::]:9876",
            AuthMode::Token(AuthToken::new("public-token").unwrap()),
            false,
        )
        .is_err());
        assert!(HttpConfig::new(
            "[::]:9876",
            AuthMode::Token(AuthToken::new("public-token").unwrap()),
            true,
        )
        .is_ok());
        assert!(HttpConfig::new(
            "192.168.1.10:9876",
            AuthMode::Token(AuthToken::new("lan-token").unwrap()),
            true,
        )
        .is_ok());
        assert!(AuthToken::new("").is_err());

        let config = HttpConfig::new(
            "127.0.0.1:9876",
            AuthMode::Token(AuthToken::new("secret-token").unwrap()),
            false,
        )
        .unwrap();
        assert!(!format!("{config:?}").contains("secret-token"));
    }

    #[tokio::test]
    async fn host_and_origin_security_middleware_matrix() {
        use axum::http::Method;

        let auth = || AuthMode::LoopbackTrust;
        let valid_host = [("host", "127.0.0.1:9876")];
        assert_eq!(
            request_router(auth(), Method::GET, "/health", &valid_host, Body::empty())
                .await
                .status(),
            StatusCode::OK,
        );
        assert_eq!(
            request_router(auth(), Method::GET, "/health", &[], Body::empty())
                .await
                .status(),
            StatusCode::FORBIDDEN,
        );
        assert_eq!(
            request_router(
                auth(),
                Method::GET,
                "/health",
                &[("host", "attacker.example:9876")],
                Body::empty(),
            )
            .await
            .status(),
            StatusCode::FORBIDDEN,
        );
        assert_eq!(
            request_router(
                auth(),
                Method::GET,
                "/health",
                &[
                    ("host", "127.0.0.1:9876"),
                    ("origin", "http://attacker.example:9876")
                ],
                Body::empty(),
            )
            .await
            .status(),
            StatusCode::FORBIDDEN,
        );
        assert_eq!(
            request_router(
                auth(),
                Method::GET,
                "/health",
                &[("host", "127.0.0.1:9876"), ("origin", "null")],
                Body::empty(),
            )
            .await
            .status(),
            StatusCode::FORBIDDEN,
        );
        assert_eq!(
            request_router(
                auth(),
                Method::GET,
                "/health",
                &[
                    ("host", "localhost:9876"),
                    ("origin", "http://localhost:9876")
                ],
                Body::empty(),
            )
            .await
            .status(),
            StatusCode::OK,
        );
        assert_eq!(
            request_router(
                auth(),
                Method::GET,
                "/health",
                &[
                    ("host", "localhost:9876"),
                    ("origin", "https://localhost:9876")
                ],
                Body::empty(),
            )
            .await
            .status(),
            StatusCode::FORBIDDEN,
        );
    }

    #[tokio::test]
    async fn token_auth_and_session_identity_fail_closed() {
        use axum::http::Method;

        let body = || Body::from(r#"{"kind":{"type":"Tui"},"label":"test"}"#);
        let host = [("host", "127.0.0.1:9876")];
        let token_auth = || AuthMode::Token(AuthToken::new("expected-token").unwrap());
        assert_eq!(
            request_router(
                token_auth(),
                Method::POST,
                "/clients/declare",
                &host,
                body()
            )
            .await
            .status(),
            StatusCode::UNAUTHORIZED,
        );
        assert_eq!(
            request_router(
                token_auth(),
                Method::POST,
                "/clients/declare",
                &[("host", "127.0.0.1:9876"), ("authorization", "Bearer x")],
                body(),
            )
            .await
            .status(),
            StatusCode::UNAUTHORIZED,
        );
        assert_eq!(
            request_router(
                token_auth(),
                Method::POST,
                "/clients/declare",
                &[
                    ("host", "127.0.0.1:9876"),
                    ("authorization", "Bearer expected-token"),
                    ("content-type", "application/json"),
                ],
                body(),
            )
            .await
            .status(),
            StatusCode::OK,
        );
        assert_eq!(
            request_router(token_auth(), Method::GET, "/health", &host, Body::empty())
                .await
                .status(),
            StatusCode::OK,
        );
        assert_eq!(
            request_router(
                token_auth(),
                Method::GET,
                "/capabilities",
                &host,
                Body::empty()
            )
            .await
            .status(),
            StatusCode::OK,
        );

        let anonymous_response = request_router(
            AuthMode::LoopbackTrust,
            Method::GET,
            "/sessions",
            &host,
            Body::empty(),
        )
        .await;
        assert_eq!(anonymous_response.status(), StatusCode::UNAUTHORIZED);
        let error_bytes = axum::body::to_bytes(anonymous_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let error: serde_json::Value = serde_json::from_slice(&error_bytes).unwrap();
        assert_eq!(error["code"]["type"], "InvalidRequest");
        assert_eq!(
            request_router(
                AuthMode::LoopbackTrust,
                Method::GET,
                "/sessions",
                &[
                    ("host", "127.0.0.1:9876"),
                    ("x-eggsec-client-id", "00000000-0000-0000-0000-000000000000"),
                ],
                Body::empty(),
            )
            .await
            .status(),
            StatusCode::OK,
        );
    }

    #[tokio::test]
    async fn http_submit_task() {
        let (addr, shutdown) = start_server().await;
        let client_id = declare_client_id(&addr, "test").await;
        let (name, value) = auth_header(&client_id);
        let client = reqwest::Client::new();

        let resp = client
            .post(format!("http://{}/sessions", addr))
            .header(name.clone(), value.clone())
            .json(&serde_json::json!({}))
            .send()
            .await
            .unwrap();
        let body: serde_json::Value = resp.json().await.unwrap();
        let session_id = body["session_id"].as_str().unwrap();

        let resp = client
            .post(format!("http://{}/sessions/{}/tasks", addr, session_id))
            .header(name.clone(), value.clone())
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
        let client_id = declare_client_id(&addr, "test").await;
        let (name, value) = auth_header(&client_id);
        let client = reqwest::Client::new();

        let resp = client
            .post(format!("http://{}/sessions", addr))
            .header(name.clone(), value.clone())
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
            .header(name, value)
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
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
        shutdown.cancel();
    }

    #[tokio::test]
    async fn http_sse_event_delivery() {
        let (addr, shutdown) = start_server().await;
        let client_id = declare_client_id(&addr, "sse-test").await;
        let (name, value) = auth_header(&client_id);
        let client = reqwest::Client::new();

        let resp = client
            .post(format!("http://{}/sessions", addr))
            .header(name.clone(), value.clone())
            .json(&serde_json::json!({}))
            .send()
            .await
            .unwrap();
        let body: serde_json::Value = resp.json().await.unwrap();
        let session_id = body["session_id"].as_str().unwrap();

        let resp = client
            .get(format!("http://{}/sessions/{}/events", addr, session_id))
            .header(name, value)
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

    /// Regression test for the SSE authorization bypass: an undeclared caller
    /// must not be able to subscribe to a session's event stream.
    #[tokio::test]
    async fn http_sse_undeclared_client_denied() {
        let (addr, shutdown) = start_server().await;
        let owner_id = declare_client_id(&addr, "sse-owner").await;
        let (name, value) = auth_header(&owner_id);
        let client = reqwest::Client::new();

        let resp = client
            .post(format!("http://{}/sessions", addr))
            .header(name, value)
            .json(&serde_json::json!({}))
            .send()
            .await
            .unwrap();
        let body: serde_json::Value = resp.json().await.unwrap();
        let session_id = body["session_id"].as_str().unwrap().to_string();

        // No X-Eggsec-Client-Id header at all.
        let resp = reqwest::Client::new()
            .get(format!("http://{}/sessions/{}/events", addr, session_id))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
        shutdown.cancel();
    }

    #[tokio::test]
    async fn http_list_persisted_sessions() {
        let (addr, shutdown) = start_server().await;
        let client_id = declare_client_id(&addr, "persisted-list").await;
        let (name, value) = auth_header(&client_id);
        let client = reqwest::Client::new();

        let resp = client
            .get(format!("http://{}/sessions/persisted", addr))
            .header(name, value)
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
    async fn http_list_persisted_sessions_undeclared_denied() {
        let (addr, shutdown) = start_server().await;
        let client = reqwest::Client::new();

        let resp = client
            .get(format!("http://{}/sessions/persisted", addr))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
        shutdown.cancel();
    }

    #[tokio::test]
    async fn http_get_persisted_snapshot() {
        let (addr, shutdown) = start_server().await;
        let client_id = declare_client_id(&addr, "persisted-get").await;
        let (name, value) = auth_header(&client_id);
        let client = reqwest::Client::new();

        let fake_id = eggsec_runtime::SessionId::new();
        let resp = client
            .get(format!("http://{}/sessions/persisted/{}", addr, fake_id))
            .header(name, value)
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
    async fn http_get_persisted_snapshot_undeclared_denied() {
        let (addr, shutdown) = start_server().await;
        let client = reqwest::Client::new();

        let fake_id = eggsec_runtime::SessionId::new();
        let resp = client
            .get(format!("http://{}/sessions/persisted/{}", addr, fake_id))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
        shutdown.cancel();
    }

    #[tokio::test]
    async fn http_get_persisted_snapshot_invalid_id() {
        let (addr, shutdown) = start_server().await;
        let client_id = declare_client_id(&addr, "invalid-persisted-id").await;
        let (name, value) = auth_header(&client_id);
        let resp = reqwest::Client::new()
            .get(format!("http://{}/sessions/persisted/not-a-uuid", addr))
            .header(name, value)
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        shutdown.cancel();
    }
}
