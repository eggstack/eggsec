//! Python bindings for daemon protocol parity — Workstreams 12–18.
//!
//! Pure Python-side types extending the daemon client API for idempotency,
//! reconnect, replay, cancellation, and artifact parity.

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyType};
use serde::{Deserialize, Serialize};

// ═══════════════════════════════════════════════════════════════════════════════
// DaemonEventPy (inline, extended with sequence)
// ═══════════════════════════════════════════════════════════════════════════════

/// Event received from a daemon session subscription.
///
/// Extended variant with a monotonic sequence number for replay ordering.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonEventPy {
    #[pyo3(get)]
    pub session_id: String,
    #[pyo3(get)]
    pub event_type: String,
    #[pyo3(get)]
    pub sequence: u64,
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
        dict.set_item("sequence", self.sequence)?;
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
            "DaemonEvent(session={}, type={}, seq={})",
            self.session_id, self.event_type, self.sequence
        )
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// DaemonProtocolVersion
// ═══════════════════════════════════════════════════════════════════════════════

/// Protocol version descriptor for the daemon host.
///
/// Announces the current wire protocol version, API schema version,
/// operation registry identifier, and feature profile.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonProtocolVersion {
    #[pyo3(get)]
    pub protocol_version: u32,
    #[pyo3(get)]
    pub api_schema_version: u32,
    #[pyo3(get)]
    pub operation_registry_id: String,
    #[pyo3(get)]
    pub feature_profile: String,
}

#[pymethods]
impl DaemonProtocolVersion {
    #[new]
    fn new(
        api_schema_version: u32,
        operation_registry_id: String,
        feature_profile: String,
    ) -> Self {
        Self {
            protocol_version: 2,
            api_schema_version,
            operation_registry_id,
            feature_profile,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("protocol_version", self.protocol_version)?;
        dict.set_item("api_schema_version", self.api_schema_version)?;
        dict.set_item("operation_registry_id", &self.operation_registry_id)?;
        dict.set_item("feature_profile", &self.feature_profile)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "DaemonProtocolVersion(protocol={}, schema={}, registry={}, profile={})",
            self.protocol_version,
            self.api_schema_version,
            self.operation_registry_id,
            self.feature_profile
        )
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// IdempotencyKey
// ═══════════════════════════════════════════════════════════════════════════════

/// Unique key for idempotent task submission.
///
/// Clients attach this to submission requests so that the daemon can detect
/// and deduplicate retries of the same logical operation.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdempotencyKey {
    #[pyo3(get)]
    pub key: String,
    #[pyo3(get)]
    pub created_at_ms: u64,
    #[pyo3(get)]
    pub operation_name: String,
    #[pyo3(get)]
    pub request_hash: String,
}

#[pymethods]
impl IdempotencyKey {
    /// Construct from a request payload.
    ///
    /// Generates a fresh UUID, hashes the request JSON, and stamps the
    /// current wall-clock time.
    #[classmethod]
    fn from_request(_cls: &Bound<'_, PyType>, operation_name: &str, request_json: &str) -> Self {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        request_json.hash(&mut hasher);
        let hash_val = hasher.finish();

        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        Self {
            key: uuid::Uuid::new_v4().to_string(),
            created_at_ms: now_ms,
            operation_name: operation_name.to_string(),
            request_hash: format!("{:016x}", hash_val),
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("key", &self.key)?;
        dict.set_item("created_at_ms", self.created_at_ms)?;
        dict.set_item("operation_name", &self.operation_name)?;
        dict.set_item("request_hash", &self.request_hash)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "IdempotencyKey(key={}, op={})",
            &self.key[..8],
            self.operation_name
        )
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// DaemonSubmissionResult
// ═══════════════════════════════════════════════════════════════════════════════

/// Result of submitting a task to the daemon.
///
/// Includes the assigned task ID, the idempotency key used for deduplication,
/// and whether this submission was a duplicate of a prior one.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonSubmissionResult {
    #[pyo3(get)]
    pub task_id: String,
    #[pyo3(get)]
    pub session_id: String,
    #[pyo3(get)]
    pub idempotency_key: String,
    #[pyo3(get)]
    pub is_duplicate: bool,
    #[pyo3(get)]
    pub submitted_at_ms: u64,
    #[pyo3(get)]
    pub estimated_duration_ms: Option<u64>,
}

#[pymethods]
impl DaemonSubmissionResult {
    #[new]
    #[pyo3(signature = (task_id, session_id="", idempotency_key="", is_duplicate=false, submitted_at_ms=0, estimated_duration_ms=None))]
    fn new(
        task_id: String,
        session_id: &str,
        idempotency_key: &str,
        is_duplicate: bool,
        submitted_at_ms: u64,
        estimated_duration_ms: Option<u64>,
    ) -> Self {
        Self {
            task_id,
            session_id: session_id.to_string(),
            idempotency_key: idempotency_key.to_string(),
            is_duplicate,
            submitted_at_ms,
            estimated_duration_ms,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("task_id", &self.task_id)?;
        dict.set_item("session_id", &self.session_id)?;
        dict.set_item("idempotency_key", &self.idempotency_key)?;
        dict.set_item("is_duplicate", self.is_duplicate)?;
        dict.set_item("submitted_at_ms", self.submitted_at_ms)?;
        dict.set_item("estimated_duration_ms", self.estimated_duration_ms)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "DaemonSubmissionResult(task={}, duplicate={})",
            self.task_id, self.is_duplicate
        )
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// ReconnectOptions
// ═══════════════════════════════════════════════════════════════════════════════

/// Options governing automatic reconnection after transport failure.
///
/// Controls retry count, delay, exponential backoff, and optional replay
/// from a known sequence number.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconnectOptions {
    #[pyo3(get)]
    pub max_retries: u32,
    #[pyo3(get)]
    pub retry_delay_ms: u64,
    #[pyo3(get)]
    pub backoff_multiplier: f64,
    #[pyo3(get)]
    pub max_backoff_ms: u64,
    #[pyo3(get)]
    pub replay_from_sequence: Option<u64>,
}

#[pymethods]
impl ReconnectOptions {
    #[new]
    #[pyo3(signature = (max_retries=5, retry_delay_ms=500, backoff_multiplier=2.0, max_backoff_ms=30_000, replay_from_sequence=None))]
    fn new(
        max_retries: u32,
        retry_delay_ms: u64,
        backoff_multiplier: f64,
        max_backoff_ms: u64,
        replay_from_sequence: Option<u64>,
    ) -> Self {
        Self {
            max_retries,
            retry_delay_ms,
            backoff_multiplier,
            max_backoff_ms,
            replay_from_sequence,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("max_retries", self.max_retries)?;
        dict.set_item("retry_delay_ms", self.retry_delay_ms)?;
        dict.set_item("backoff_multiplier", self.backoff_multiplier)?;
        dict.set_item("max_backoff_ms", self.max_backoff_ms)?;
        dict.set_item("replay_from_sequence", self.replay_from_sequence)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "ReconnectOptions(retries={}, delay_ms={}, backoff={})",
            self.max_retries, self.retry_delay_ms, self.backoff_multiplier
        )
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// ReplayCursor
// ═══════════════════════════════════════════════════════════════════════════════

/// Cursor describing the current position in a replay stream.
///
/// Tracks the last consumed sequence number, total events, and any gaps
/// or duplicates detected during replay.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayCursor {
    #[pyo3(get)]
    pub session_id: String,
    #[pyo3(get)]
    pub last_sequence: u64,
    #[pyo3(get)]
    pub total_events: u64,
    #[pyo3(get)]
    pub gap_count: usize,
    #[pyo3(get)]
    pub duplicate_count: usize,
    #[pyo3(get)]
    pub timestamp_ms: u64,
}

#[pymethods]
impl ReplayCursor {
    #[new]
    #[pyo3(signature = (session_id, last_sequence=0, total_events=0, gap_count=0, duplicate_count=0, timestamp_ms=0))]
    fn new(
        session_id: String,
        last_sequence: u64,
        total_events: u64,
        gap_count: usize,
        duplicate_count: usize,
        timestamp_ms: u64,
    ) -> Self {
        Self {
            session_id,
            last_sequence,
            total_events,
            gap_count,
            duplicate_count,
            timestamp_ms,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("session_id", &self.session_id)?;
        dict.set_item("last_sequence", self.last_sequence)?;
        dict.set_item("total_events", self.total_events)?;
        dict.set_item("gap_count", self.gap_count)?;
        dict.set_item("duplicate_count", self.duplicate_count)?;
        dict.set_item("timestamp_ms", self.timestamp_ms)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "ReplayCursor(session={}, seq={}, gaps={}, dupes={})",
            self.session_id, self.last_sequence, self.gap_count, self.duplicate_count
        )
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// ReplayResult
// ═══════════════════════════════════════════════════════════════════════════════

/// Result of replaying events from a daemon session.
///
/// Contains the events recovered, a cursor for resumption, and a flag
/// indicating whether more events are available beyond this batch.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayResult {
    #[pyo3(get)]
    pub cursor: ReplayCursor,
    #[pyo3(get)]
    pub has_more: bool,
}

#[pymethods]
impl ReplayResult {
    #[new]
    fn new(cursor: ReplayCursor, has_more: bool) -> Self {
        Self { cursor, has_more }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("cursor", self.cursor.to_dict(py)?)?;
        dict.set_item("has_more", self.has_more)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!("ReplayResult(has_more={})", self.has_more)
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// CancellationRequest
// ═══════════════════════════════════════════════════════════════════════════════

/// Request to cancel a running or pending task.
///
/// Supports both cooperative and forced cancellation via the `force` flag.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancellationRequest {
    #[pyo3(get)]
    pub session_id: String,
    #[pyo3(get)]
    pub task_id: String,
    #[pyo3(get)]
    pub reason: Option<String>,
    #[pyo3(get)]
    pub force: bool,
    #[pyo3(get)]
    pub requested_at_ms: u64,
}

#[pymethods]
impl CancellationRequest {
    #[new]
    #[pyo3(signature = (task_id, session_id="", reason=None, force=false, requested_at_ms=0))]
    fn new(
        task_id: String,
        session_id: &str,
        reason: Option<String>,
        force: bool,
        requested_at_ms: u64,
    ) -> Self {
        Self {
            session_id: session_id.to_string(),
            task_id,
            reason,
            force,
            requested_at_ms,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("session_id", &self.session_id)?;
        dict.set_item("task_id", &self.task_id)?;
        dict.set_item("reason", &self.reason)?;
        dict.set_item("force", self.force)?;
        dict.set_item("requested_at_ms", self.requested_at_ms)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "CancellationRequest(session={}, task={}, force={})",
            self.session_id, self.task_id, self.force
        )
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// CancellationResult
// ═══════════════════════════════════════════════════════════════════════════════

/// Result of a task cancellation request.
///
/// Reports whether the cancellation was acknowledged and what happened
/// to the targeted task.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancellationResult {
    #[pyo3(get)]
    pub acknowledged: bool,
    #[pyo3(get)]
    pub task_was_running: bool,
    #[pyo3(get)]
    pub task_was_completed: bool,
    #[pyo3(get)]
    pub cleanup_started: bool,
    #[pyo3(get)]
    pub message: Option<String>,
}

#[pymethods]
impl CancellationResult {
    #[new]
    #[pyo3(signature = (acknowledged, task_was_running=false, task_was_completed=false, cleanup_started=false, message=None))]
    fn new(
        acknowledged: bool,
        task_was_running: bool,
        task_was_completed: bool,
        cleanup_started: bool,
        message: Option<String>,
    ) -> Self {
        Self {
            acknowledged,
            task_was_running,
            task_was_completed,
            cleanup_started,
            message,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("acknowledged", self.acknowledged)?;
        dict.set_item("task_was_running", self.task_was_running)?;
        dict.set_item("task_was_completed", self.task_was_completed)?;
        dict.set_item("cleanup_started", self.cleanup_started)?;
        dict.set_item("message", &self.message)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "CancellationResult(ack={}, running={}, completed={}, cleanup={})",
            self.acknowledged, self.task_was_running, self.task_was_completed, self.cleanup_started
        )
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// TaskArtifactDescriptor
// ═══════════════════════════════════════════════════════════════════════════════

/// Descriptor for a task artifact produced by the daemon.
///
/// Provides metadata about the artifact (kind, content type, hash) without
/// transferring the full payload.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskArtifactDescriptor {
    #[pyo3(get)]
    pub artifact_id: String,
    #[pyo3(get)]
    pub task_id: String,
    #[pyo3(get)]
    pub session_id: String,
    #[pyo3(get)]
    pub kind: String,
    #[pyo3(get)]
    pub content_type: String,
    #[pyo3(get)]
    pub size_bytes: u64,
    #[pyo3(get)]
    pub content_hash: String,
    #[pyo3(get)]
    pub created_at_ms: u64,
    #[pyo3(get)]
    pub redacted: bool,
    #[pyo3(get)]
    pub download_url: Option<String>,
}

#[pymethods]
impl TaskArtifactDescriptor {
    #[new]
    #[pyo3(signature = (artifact_id, task_id="", session_id="", kind="report", content_type="application/octet-stream", size_bytes=0, content_hash="", created_at_ms=0, redacted=false, download_url=None))]
    fn new(
        artifact_id: String,
        task_id: &str,
        session_id: &str,
        kind: &str,
        content_type: &str,
        size_bytes: u64,
        content_hash: &str,
        created_at_ms: u64,
        redacted: bool,
        download_url: Option<String>,
    ) -> Self {
        Self {
            artifact_id,
            task_id: task_id.to_string(),
            session_id: session_id.to_string(),
            kind: kind.to_string(),
            content_type: content_type.to_string(),
            size_bytes,
            content_hash: content_hash.to_string(),
            created_at_ms,
            redacted,
            download_url,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("artifact_id", &self.artifact_id)?;
        dict.set_item("task_id", &self.task_id)?;
        dict.set_item("session_id", &self.session_id)?;
        dict.set_item("kind", &self.kind)?;
        dict.set_item("content_type", &self.content_type)?;
        dict.set_item("size_bytes", self.size_bytes)?;
        dict.set_item("content_hash", &self.content_hash)?;
        dict.set_item("created_at_ms", self.created_at_ms)?;
        dict.set_item("redacted", self.redacted)?;
        dict.set_item("download_url", &self.download_url)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "TaskArtifactDescriptor(id={}, kind={}, bytes={}, redacted={})",
            self.artifact_id, self.kind, self.size_bytes, self.redacted
        )
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// EventReplayInfo
// ═══════════════════════════════════════════════════════════════════════════════

/// Metadata about a replayed event range.
///
/// Describes the sequence span, any detected gaps or duplicates, and
/// whether the events are fully ordered.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventReplayInfo {
    #[pyo3(get)]
    pub session_id: String,
    #[pyo3(get)]
    pub from_sequence: u64,
    #[pyo3(get)]
    pub to_sequence: u64,
    #[pyo3(get)]
    pub event_count: usize,
    #[pyo3(get)]
    pub ordered: bool,
}

#[pymethods]
impl EventReplayInfo {
    #[new]
    #[pyo3(signature = (session_id, from_sequence=0, to_sequence=0, event_count=0, ordered=true))]
    fn new(
        session_id: String,
        from_sequence: u64,
        to_sequence: u64,
        event_count: usize,
        ordered: bool,
    ) -> Self {
        Self {
            session_id,
            from_sequence,
            to_sequence,
            event_count,
            ordered,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("session_id", &self.session_id)?;
        dict.set_item("from_sequence", self.from_sequence)?;
        dict.set_item("to_sequence", self.to_sequence)?;
        dict.set_item("event_count", self.event_count)?;
        dict.set_item("ordered", self.ordered)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "EventReplayInfo(session={}, seq={}..{}, count={}, ordered={})",
            self.session_id, self.from_sequence, self.to_sequence, self.event_count, self.ordered
        )
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// DaemonHealthDetail
// ═══════════════════════════════════════════════════════════════════════════════

/// Detailed health information for the daemon host.
///
/// Extends the basic health response with operational metrics.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonHealthDetail {
    #[pyo3(get)]
    pub status: String,
    #[pyo3(get)]
    pub uptime_secs: u64,
    #[pyo3(get)]
    pub protocol_version: u32,
    #[pyo3(get)]
    pub active_sessions: usize,
    #[pyo3(get)]
    pub active_clients: usize,
    #[pyo3(get)]
    pub total_tasks_completed: u64,
    #[pyo3(get)]
    pub persistence_backend: String,
    #[pyo3(get)]
    pub transport: String,
}

#[pymethods]
impl DaemonHealthDetail {
    #[new]
    #[pyo3(signature = (status, uptime_secs=0, protocol_version=2, active_sessions=0, active_clients=0, total_tasks_completed=0, persistence_backend="none", transport="unix_socket"))]
    fn new(
        status: String,
        uptime_secs: u64,
        protocol_version: u32,
        active_sessions: usize,
        active_clients: usize,
        total_tasks_completed: u64,
        persistence_backend: &str,
        transport: &str,
    ) -> Self {
        Self {
            status,
            uptime_secs,
            protocol_version,
            active_sessions,
            active_clients,
            total_tasks_completed,
            persistence_backend: persistence_backend.to_string(),
            transport: transport.to_string(),
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("status", &self.status)?;
        dict.set_item("uptime_secs", self.uptime_secs)?;
        dict.set_item("protocol_version", self.protocol_version)?;
        dict.set_item("active_sessions", self.active_sessions)?;
        dict.set_item("active_clients", self.active_clients)?;
        dict.set_item("total_tasks_completed", self.total_tasks_completed)?;
        dict.set_item("persistence_backend", &self.persistence_backend)?;
        dict.set_item("transport", &self.transport)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "DaemonHealthDetail(status={}, uptime={}s, protocol={}, sessions={}, clients={}, tasks={})",
            self.status,
            self.uptime_secs,
            self.protocol_version,
            self.active_sessions,
            self.active_clients,
            self.total_tasks_completed
        )
    }
}
