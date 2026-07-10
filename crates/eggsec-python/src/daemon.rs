//! Python bindings for the eggsec daemon client.
//!
//! Feature-gated behind `daemon-client`. When the daemon crate is not available,
//! functions return `FeatureUnavailableError`.

use pyo3::prelude::*;
use pyo3::types::PyDict;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::runtime_async;
use crate::runtime_sync;

// ---------------------------------------------------------------------------
// Internal helper: extract error code string from daemon protocol error codes
// ---------------------------------------------------------------------------

#[cfg(feature = "daemon-client")]
fn error_code_to_string(code: &eggsec_daemon::protocol::ErrorCode) -> String {
    use eggsec_daemon::protocol::ErrorCode;
    match code {
        ErrorCode::InvalidRequest => "InvalidRequest".into(),
        ErrorCode::SessionNotFound => "SessionNotFound".into(),
        ErrorCode::TaskNotFound => "TaskNotFound".into(),
        ErrorCode::TaskAlreadyCompleted => "TaskAlreadyCompleted".into(),
        ErrorCode::UnsupportedCommand => "UnsupportedCommand".into(),
        ErrorCode::Internal => "Internal".into(),
        ErrorCode::PermissionDenied => "PermissionDenied".into(),
        ErrorCode::InvalidSurface => "InvalidSurface".into(),
        ErrorCode::ClientNotDeclared => "ClientNotDeclared".into(),
        ErrorCode::Unsupported => "Unsupported".into(),
        ErrorCode::InvalidState => "InvalidState".into(),
    }
}

// ---------------------------------------------------------------------------
// Simplified response DTO
// ---------------------------------------------------------------------------

/// A simplified representation of a daemon server response.
///
/// The daemon protocol returns rich enum variants, but for Python interop we
/// flatten them into a uniform (ok, request_id, message, error_code) tuple.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonResponsePy {
    #[pyo3(get)]
    pub ok: bool,
    #[pyo3(get)]
    pub request_id: String,
    #[pyo3(get)]
    pub message: String,
    #[pyo3(get)]
    pub error_code: Option<String>,
}

#[pymethods]
impl DaemonResponsePy {
    /// Convert to a Python dictionary.
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("ok", self.ok)?;
        dict.set_item("request_id", &self.request_id)?;
        dict.set_item("message", &self.message)?;
        dict.set_item("error_code", &self.error_code)?;
        Ok(dict.into())
    }

    /// Convert to a JSON string.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "DaemonResponse(ok={}, request_id={}, error_code={:?})",
            self.ok, self.request_id, self.error_code
        )
    }

    fn __str__(&self) -> String {
        if self.ok {
            format!("OK [{}]", self.request_id)
        } else {
            format!(
                "Error [{}]: {} ({})",
                self.request_id,
                self.message,
                self.error_code.as_deref().unwrap_or("Unknown")
            )
        }
    }
}

// ---------------------------------------------------------------------------
// Internal helper: ServerMessage → DaemonResponsePy
// ---------------------------------------------------------------------------

#[cfg(feature = "daemon-client")]
fn server_message_to_response(msg: eggsec_daemon::protocol::ServerMessage) -> DaemonResponsePy {
    use eggsec_daemon::protocol::ServerMessage;
    match msg {
        ServerMessage::Ok { request_id } => DaemonResponsePy {
            ok: true,
            request_id,
            message: String::new(),
            error_code: None,
        },
        ServerMessage::Error {
            request_id,
            code,
            message,
        } => DaemonResponsePy {
            ok: false,
            request_id,
            message,
            error_code: Some(error_code_to_string(&code)),
        },
        ServerMessage::Health {
            request_id,
            status,
            version,
            protocol_version,
        } => DaemonResponsePy {
            ok: true,
            request_id,
            message: format!(
                "status={}, version={}, protocol={}",
                status, version, protocol_version
            ),
            error_code: None,
        },
        ServerMessage::ClientDeclared {
            request_id,
            client_id,
        } => DaemonResponsePy {
            ok: true,
            request_id,
            message: format!("client_id={}", client_id),
            error_code: None,
        },
        ServerMessage::SessionCreated {
            request_id,
            session_id,
        } => DaemonResponsePy {
            ok: true,
            request_id,
            message: format!("session_id={}", session_id),
            error_code: None,
        },
        ServerMessage::Sessions {
            request_id,
            sessions,
        } => DaemonResponsePy {
            ok: true,
            request_id,
            message: format!("{} session(s)", sessions.len()),
            error_code: None,
        },
        ServerMessage::Snapshot {
            request_id,
            snapshot,
        } => DaemonResponsePy {
            ok: true,
            request_id,
            message: format!(
                "session={}, active={}, completed={}",
                snapshot.session_id,
                snapshot.active_tasks.len(),
                snapshot.completed_tasks.len()
            ),
            error_code: None,
        },
        ServerMessage::TaskSubmitted {
            request_id,
            task_id,
        } => DaemonResponsePy {
            ok: true,
            request_id,
            message: format!("task_id={}", task_id),
            error_code: None,
        },
        ServerMessage::SessionClosed { request_id } => DaemonResponsePy {
            ok: true,
            request_id,
            message: "session_closed".into(),
            error_code: None,
        },
        ServerMessage::Capabilities {
            request_id,
            capabilities,
        } => DaemonResponsePy {
            ok: true,
            request_id,
            message: format!("transports={}", capabilities.transports.len()),
            error_code: None,
        },
        ServerMessage::RuntimeEvent { .. } => DaemonResponsePy {
            ok: true,
            request_id: String::new(),
            message: "RuntimeEvent (streamed)".into(),
            error_code: None,
        },
        ServerMessage::PersistedSessions {
            request_id,
            sessions,
        } => DaemonResponsePy {
            ok: true,
            request_id,
            message: format!("{} persisted session(s)", sessions.len()),
            error_code: None,
        },
        ServerMessage::PersistedSnapshot {
            request_id,
            snapshot,
        } => DaemonResponsePy {
            ok: true,
            request_id,
            message: match &snapshot {
                Some(s) => format!("session={}", s.session_id),
                None => "not_found".into(),
            },
            error_code: None,
        },
    }
}

// ---------------------------------------------------------------------------
// Daemon client wrapper
// ---------------------------------------------------------------------------

/// Python wrapper around the eggsec daemon client.
///
/// Not frozen — the inner client is mutable for async I/O.
#[pyclass]
#[derive(Clone)]
pub struct DaemonClientPy {
    socket_path: String,
    #[cfg(feature = "daemon-client")]
    client: Arc<Mutex<Option<eggsec_daemon::client::DaemonClient>>>,
}

#[pymethods]
impl DaemonClientPy {
    /// The Unix socket path this client connects to.
    #[getter]
    fn socket_path(&self) -> String {
        self.socket_path.clone()
    }

    /// Check if the client connection has been closed.
    #[getter]
    fn is_closed(&self) -> bool {
        #[cfg(feature = "daemon-client")]
        {
            match self.client.try_lock() {
                Ok(guard) => guard.is_none(),
                Err(_) => false,
            }
        }
        #[cfg(not(feature = "daemon-client"))]
        {
            true
        }
    }

    /// Close the client connection. Idempotent — safe to call multiple times.
    fn close(&self) {
        #[cfg(feature = "daemon-client")]
        {
            if let Ok(mut guard) = self.client.try_lock() {
                *guard = None;
            }
        }
    }

    /// Context manager __enter__.
    fn __enter__(slf: Py<Self>) -> Py<Self> {
        slf
    }

    /// Context manager __exit__ — closes the client connection.
    #[pyo3(signature = (_exc_type=None, _exc_value=None, _traceback=None))]
    fn __exit__(
        &self,
        _exc_type: Option<&Bound<'_, PyAny>>,
        _exc_value: Option<&Bound<'_, PyAny>>,
        _traceback: Option<&Bound<'_, PyAny>>,
    ) -> bool {
        self.close();
        false
    }

    fn __repr__(&self) -> String {
        format!("DaemonClient(socket_path={})", self.socket_path)
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }

    /// Convert to a Python dictionary.
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("socket_path", &self.socket_path)?;
        dict.set_item("is_closed", self.is_closed())?;
        Ok(dict.into())
    }
}

// ---------------------------------------------------------------------------
// Helper: parse a surface string into RuntimeSurface
// ---------------------------------------------------------------------------

#[cfg(feature = "daemon-client")]
fn parse_surface(surface: &str) -> PyResult<eggsec_runtime::RuntimeSurface> {
    use eggsec_runtime::RuntimeSurface;
    match surface {
        "cli_manual" | "CliManual" => Ok(RuntimeSurface::CliManual),
        "tui_manual" | "TuiManual" => Ok(RuntimeSurface::TuiManual),
        "mcp_server" | "McpServer" => Ok(RuntimeSurface::McpServer),
        "rest_api" | "RestApi" => Ok(RuntimeSurface::RestApi),
        "grpc_api" | "GrpcApi" => Ok(RuntimeSurface::GrpcApi),
        "security_agent" | "SecurityAgent" => Ok(RuntimeSurface::SecurityAgent),
        "ci" | "Ci" => Ok(RuntimeSurface::Ci),
        _ => Err(pyo3::exceptions::PyValueError::new_err(format!(
            "Unknown surface: {}. Valid values: cli_manual, tui_manual, mcp_server, rest_api, grpc_api, security_agent, ci",
            surface
        ))),
    }
}

// ---------------------------------------------------------------------------
// Synchronous connect
// ---------------------------------------------------------------------------

/// Connect to a running eggsec daemon via Unix socket (synchronous).
///
/// The actual connection is performed asynchronously under the hood, with the
/// GIL released during I/O.
///
/// Args:
///     socket_path: Path to the daemon Unix socket (e.g. "/tmp/eggsec.sock").
///
/// Returns:
///     DaemonClientPy: A client handle that can be used with the async_* functions.
///
/// Raises:
///     FeatureUnavailableError: If the daemon-client feature is not enabled.
///     NetworkError: If the connection fails.
#[pyfunction]
pub fn daemon_connect(py: Python<'_>, socket_path: &str) -> PyResult<DaemonClientPy> {
    #[cfg(feature = "daemon-client")]
    {
        let path = socket_path.to_string();
        let client = runtime_sync::block_on(py, async move {
            eggsec_daemon::client::DaemonClient::connect(&path)
                .await
                .map_err(|e| {
                    crate::error::NetworkError::new_err(format!(
                        "Failed to connect to daemon at {}: {}",
                        path, e
                    ))
                })
        })?;

        Ok(DaemonClientPy {
            socket_path: socket_path.to_string(),
            client: Arc::new(Mutex::new(Some(client))),
        })
    }

    #[cfg(not(feature = "daemon-client"))]
    {
        Err(crate::error::FeatureUnavailableError::new_err(
            "Daemon client is not available. Rebuild with the `daemon-client` feature enabled.",
        ))
    }
}

// ---------------------------------------------------------------------------
// Async operations
// ---------------------------------------------------------------------------

/// Check daemon health (async).
///
/// Returns a PyFuture that resolves to a DaemonResponsePy.
#[pyfunction]
pub fn async_daemon_health(client: DaemonClientPy) -> PyResult<runtime_async::PyFuture> {
    #[cfg(feature = "daemon-client")]
    {
        let client_arc = client.client.clone();
        runtime_async::spawn_async(async move {
            let mut guard = client_arc.lock().await;
            let inner = guard
                .as_mut()
                .ok_or_else(|| crate::error::NetworkError::new_err("client is closed"))?;

            let msg = inner.health().await.map_err(|e| {
                crate::error::NetworkError::new_err(format!("daemon health failed: {}", e))
            })?;

            Ok(server_message_to_response(msg))
        })
    }

    #[cfg(not(feature = "daemon-client"))]
    {
        Err(crate::error::FeatureUnavailableError::new_err(
            "Daemon client is not available. Rebuild with the `daemon-client` feature enabled.",
        ))
    }
}

/// Declare this client to the daemon (async).
///
/// Must be called before session operations. Returns a PyFuture that resolves
/// to a DaemonResponsePy.
///
/// Args:
///     client: A DaemonClientPy from daemon_connect().
///     kind: Client kind ("cli", "tui", "mcp", "rest", "agent", "unknown").
///     label: Optional human-readable label for this client.
#[pyfunction]
#[pyo3(signature = (client, kind="cli", label=None))]
pub fn async_daemon_declare_client(
    client: DaemonClientPy,
    kind: &str,
    label: Option<String>,
) -> PyResult<runtime_async::PyFuture> {
    #[cfg(feature = "daemon-client")]
    {
        let client_arc = client.client.clone();
        let kind_owned = kind.to_string();
        let label_owned = label;
        runtime_async::spawn_async(async move {
            use eggsec_daemon::client_registry::ClientKind;

            let client_kind = match kind_owned.as_str() {
                "cli" => ClientKind::Cli,
                "tui" => ClientKind::Tui,
                "mcp" => ClientKind::Mcp,
                "rest" => ClientKind::Rest,
                "agent" => ClientKind::Agent,
                "daemon_internal" => ClientKind::DaemonInternal,
                _ => ClientKind::Unknown,
            };

            let mut guard = client_arc.lock().await;
            let inner = guard
                .as_mut()
                .ok_or_else(|| crate::error::NetworkError::new_err("client is closed"))?;

            let msg = inner
                .declare_client(client_kind, label_owned)
                .await
                .map_err(|e| {
                    crate::error::NetworkError::new_err(format!(
                        "daemon declare_client failed: {}",
                        e
                    ))
                })?;

            Ok(server_message_to_response(msg))
        })
    }

    #[cfg(not(feature = "daemon-client"))]
    {
        Err(crate::error::FeatureUnavailableError::new_err(
            "Daemon client is not available. Rebuild with the `daemon-client` feature enabled.",
        ))
    }
}

/// Create a new session on the daemon (async).
///
/// Returns a PyFuture that resolves to a DaemonResponsePy with the session_id
/// in the message field.
///
/// Args:
///     client: A DaemonClientPy from daemon_connect().
///     surface: Runtime surface ("cli_manual", "tui_manual", "mcp_server", etc.).
///     labels: Optional list of labels to attach to the session.
#[pyfunction]
#[pyo3(signature = (client, surface="cli_manual", labels=None))]
pub fn async_daemon_create_session(
    client: DaemonClientPy,
    surface: &str,
    labels: Option<Vec<String>>,
) -> PyResult<runtime_async::PyFuture> {
    #[cfg(feature = "daemon-client")]
    {
        let client_arc = client.client.clone();
        let surface_owned = surface.to_string();
        let labels_owned = labels.unwrap_or_default();
        runtime_async::spawn_async(async move {
            let runtime_surface = parse_surface(&surface_owned)?;

            let mut guard = client_arc.lock().await;
            let inner = guard
                .as_mut()
                .ok_or_else(|| crate::error::NetworkError::new_err("client is closed"))?;

            let msg = inner
                .create_session(runtime_surface, None, labels_owned)
                .await
                .map_err(|e| {
                    crate::error::NetworkError::new_err(format!(
                        "daemon create_session failed: {}",
                        e
                    ))
                })?;

            Ok(server_message_to_response(msg))
        })
    }

    #[cfg(not(feature = "daemon-client"))]
    {
        Err(crate::error::FeatureUnavailableError::new_err(
            "Daemon client is not available. Rebuild with the `daemon-client` feature enabled.",
        ))
    }
}

/// List all active sessions on the daemon (async).
///
/// Returns a PyFuture that resolves to a DaemonResponsePy.
#[pyfunction]
pub fn async_daemon_list_sessions(client: DaemonClientPy) -> PyResult<runtime_async::PyFuture> {
    #[cfg(feature = "daemon-client")]
    {
        let client_arc = client.client.clone();
        runtime_async::spawn_async(async move {
            let mut guard = client_arc.lock().await;
            let inner = guard
                .as_mut()
                .ok_or_else(|| crate::error::NetworkError::new_err("client is closed"))?;

            let msg = inner.list_sessions().await.map_err(|e| {
                crate::error::NetworkError::new_err(format!("daemon list_sessions failed: {}", e))
            })?;

            Ok(server_message_to_response(msg))
        })
    }

    #[cfg(not(feature = "daemon-client"))]
    {
        Err(crate::error::FeatureUnavailableError::new_err(
            "Daemon client is not available. Rebuild with the `daemon-client` feature enabled.",
        ))
    }
}

/// Get a snapshot of a session (async).
///
/// Returns a PyFuture that resolves to a DaemonResponsePy.
///
/// Args:
///     client: A DaemonClientPy from daemon_connect().
///     session_id: The session UUID string.
#[pyfunction]
pub fn async_daemon_get_snapshot(
    client: DaemonClientPy,
    session_id: &str,
) -> PyResult<runtime_async::PyFuture> {
    #[cfg(feature = "daemon-client")]
    {
        let client_arc = client.client.clone();
        let session_id_owned = session_id.to_string();
        runtime_async::spawn_async(async move {
            let sid: eggsec_runtime::SessionId = session_id_owned.parse().map_err(|_| {
                crate::error::ConfigError::new_err(format!(
                    "Invalid session ID: {}",
                    session_id_owned
                ))
            })?;

            let mut guard = client_arc.lock().await;
            let inner = guard
                .as_mut()
                .ok_or_else(|| crate::error::NetworkError::new_err("client is closed"))?;

            let msg = inner.get_snapshot(sid).await.map_err(|e| {
                crate::error::NetworkError::new_err(format!("daemon get_snapshot failed: {}", e))
            })?;

            Ok(server_message_to_response(msg))
        })
    }

    #[cfg(not(feature = "daemon-client"))]
    {
        Err(crate::error::FeatureUnavailableError::new_err(
            "Daemon client is not available. Rebuild with the `daemon-client` feature enabled.",
        ))
    }
}

/// Close a session on the daemon (async).
///
/// Returns a PyFuture that resolves to a DaemonResponsePy.
///
/// Args:
///     client: A DaemonClientPy from daemon_connect().
///     session_id: The session UUID string.
#[pyfunction]
pub fn async_daemon_close_session(
    client: DaemonClientPy,
    session_id: &str,
) -> PyResult<runtime_async::PyFuture> {
    #[cfg(feature = "daemon-client")]
    {
        let client_arc = client.client.clone();
        let session_id_owned = session_id.to_string();
        runtime_async::spawn_async(async move {
            let sid: eggsec_runtime::SessionId = session_id_owned.parse().map_err(|_| {
                crate::error::ConfigError::new_err(format!(
                    "Invalid session ID: {}",
                    session_id_owned
                ))
            })?;

            let mut guard = client_arc.lock().await;
            let inner = guard
                .as_mut()
                .ok_or_else(|| crate::error::NetworkError::new_err("client is closed"))?;

            let msg = inner.close_session(sid).await.map_err(|e| {
                crate::error::NetworkError::new_err(format!("daemon close_session failed: {}", e))
            })?;

            Ok(server_message_to_response(msg))
        })
    }

    #[cfg(not(feature = "daemon-client"))]
    {
        Err(crate::error::FeatureUnavailableError::new_err(
            "Daemon client is not available. Rebuild with the `daemon-client` feature enabled.",
        ))
    }
}

// ═══════════════════════════════════════════════════════════════════
// D6: Daemon task API types
// ═══════════════════════════════════════════════════════════════════

/// Capabilities reported by the daemon host.
///
/// Describes supported transports, runtime features, and protocol version.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonCapabilitiesPy {
    #[pyo3(get)]
    pub protocol_version: u32,
    #[pyo3(get)]
    pub transports: Vec<String>,
    #[pyo3(get)]
    pub has_persistence: bool,
    #[pyo3(get)]
    pub max_clients: usize,
    #[pyo3(get)]
    pub runtime_version: String,
}

#[pymethods]
impl DaemonCapabilitiesPy {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("protocol_version", self.protocol_version)?;
        dict.set_item("transports", &self.transports)?;
        dict.set_item("has_persistence", self.has_persistence)?;
        dict.set_item("max_clients", self.max_clients)?;
        dict.set_item("runtime_version", &self.runtime_version)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "DaemonCapabilities(version={}, transports={})",
            self.protocol_version,
            self.transports.len()
        )
    }
}

/// Handle to a submitted daemon task.
///
/// Used to track and manage a task after submission.
#[pyclass]
#[derive(Debug, Clone)]
pub struct TaskHandlePy {
    pub(crate) task_id: String,
    pub(crate) session_id: String,
    pub(crate) client_id: Option<String>,
}

#[pymethods]
impl TaskHandlePy {
    #[getter]
    fn task_id(&self) -> &str {
        &self.task_id
    }

    #[getter]
    fn session_id(&self) -> &str {
        &self.session_id
    }

    #[getter]
    fn client_id(&self) -> Option<&str> {
        self.client_id.as_deref()
    }

    fn __repr__(&self) -> String {
        format!(
            "TaskHandle(task_id={}, session={})",
            self.task_id, self.session_id
        )
    }

    fn __str__(&self) -> String {
        format!("Task {} in session {}", self.task_id, self.session_id)
    }
}

/// Current status of a daemon task.
///
/// Tracks the task's lifecycle from submission through completion.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStatusPy {
    #[pyo3(get)]
    pub task_id: String,
    #[pyo3(get)]
    pub session_id: String,
    #[pyo3(get)]
    pub state: String,
    #[pyo3(get)]
    pub submitted_at_ms: Option<u64>,
    #[pyo3(get)]
    pub started_at_ms: Option<u64>,
    #[pyo3(get)]
    pub completed_at_ms: Option<u64>,
    #[pyo3(get)]
    pub error: Option<String>,
    #[pyo3(get)]
    pub result_available: bool,
}

#[pymethods]
impl TaskStatusPy {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("task_id", &self.task_id)?;
        dict.set_item("session_id", &self.session_id)?;
        dict.set_item("state", &self.state)?;
        dict.set_item("submitted_at_ms", self.submitted_at_ms)?;
        dict.set_item("started_at_ms", self.started_at_ms)?;
        dict.set_item("completed_at_ms", self.completed_at_ms)?;
        dict.set_item("error", &self.error)?;
        dict.set_item("result_available", self.result_available)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!("TaskStatus(task={}, state={})", self.task_id, self.state)
    }

    fn __str__(&self) -> String {
        format!("Task {} is {}", self.task_id, self.state)
    }
}

/// Event received from a daemon session subscription.
///
/// Represents a runtime event pushed from the daemon to subscribed clients.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonEventPy {
    #[pyo3(get)]
    pub session_id: String,
    #[pyo3(get)]
    pub event_type: String,
    #[pyo3(get)]
    pub timestamp_ms: u64,
    #[pyo3(get)]
    pub data: Option<String>,
}

#[pymethods]
impl DaemonEventPy {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("session_id", &self.session_id)?;
        dict.set_item("event_type", &self.event_type)?;
        dict.set_item("timestamp_ms", self.timestamp_ms)?;
        dict.set_item("data", &self.data)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "DaemonEvent(session={}, type={})",
            self.session_id, self.event_type
        )
    }
}

/// Summary of a daemon session.
///
/// Contains high-level information about a session for listing purposes.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummaryPy {
    #[pyo3(get)]
    pub session_id: String,
    #[pyo3(get)]
    pub surface: String,
    #[pyo3(get)]
    pub state: String,
    #[pyo3(get)]
    pub labels: Vec<String>,
    #[pyo3(get)]
    pub created_at_ms: Option<u64>,
    #[pyo3(get)]
    pub task_count: usize,
}

#[pymethods]
impl SessionSummaryPy {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("session_id", &self.session_id)?;
        dict.set_item("surface", &self.surface)?;
        dict.set_item("state", &self.state)?;
        dict.set_item("labels", &self.labels)?;
        dict.set_item("created_at_ms", self.created_at_ms)?;
        dict.set_item("task_count", self.task_count)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "SessionSummary(id={}, surface={}, state={})",
            self.session_id, self.surface, self.state
        )
    }
}

/// Transport metadata for a daemon connection.
///
/// Describes the transport type and connection details for a daemon client.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportMetadataPy {
    #[pyo3(get)]
    pub kind: String,
    #[pyo3(get)]
    pub bind_address: String,
    #[pyo3(get)]
    pub enabled: bool,
}

#[pymethods]
impl TransportMetadataPy {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("kind", &self.kind)?;
        dict.set_item("bind_address", &self.bind_address)?;
        dict.set_item("enabled", self.enabled)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "TransportMetadata(kind={}, addr={})",
            self.kind, self.bind_address
        )
    }
}
