use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use serde::{Deserialize, Serialize};

// ═══════════════════════════════════════════════════════════════════
// R4-W1: Common managed-session contract
// ═══════════════════════════════════════════════════════════════════

/// Lifecycle state of a managed session.
#[pyclass(frozen, eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SessionState {
    Created,
    Starting,
    Running,
    Pausing,
    Paused,
    Stopping,
    Stopped,
    Failed,
    Cancelled,
}

#[pymethods]
impl SessionState {
    fn __repr__(&self) -> String {
        format!("SessionState.{}", self.as_str())
    }

    fn __str__(&self) -> String {
        self.as_str().to_string()
    }
}

impl SessionState {
    fn as_str(&self) -> &str {
        match self {
            SessionState::Created => "Created",
            SessionState::Starting => "Starting",
            SessionState::Running => "Running",
            SessionState::Pausing => "Pausing",
            SessionState::Paused => "Paused",
            SessionState::Stopping => "Stopping",
            SessionState::Stopped => "Stopped",
            SessionState::Failed => "Failed",
            SessionState::Cancelled => "Cancelled",
        }
    }

    fn from_str(s: &str) -> Option<Self> {
        match s {
            "Created" => Some(SessionState::Created),
            "Starting" => Some(SessionState::Starting),
            "Running" => Some(SessionState::Running),
            "Pausing" => Some(SessionState::Pausing),
            "Paused" => Some(SessionState::Paused),
            "Stopping" => Some(SessionState::Stopping),
            "Stopped" => Some(SessionState::Stopped),
            "Failed" => Some(SessionState::Failed),
            "Cancelled" => Some(SessionState::Cancelled),
            _ => None,
        }
    }
}

/// How a managed session should be closed.
#[pyclass(frozen, eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SessionCloseMode {
    Graceful,
    Forced,
    Immediate,
}

#[pymethods]
impl SessionCloseMode {
    fn __repr__(&self) -> String {
        format!("SessionCloseMode.{}", self.as_str())
    }

    fn __str__(&self) -> String {
        self.as_str().to_string()
    }
}

impl SessionCloseMode {
    fn as_str(&self) -> &str {
        match self {
            SessionCloseMode::Graceful => "Graceful",
            SessionCloseMode::Forced => "Forced",
            SessionCloseMode::Immediate => "Immediate",
        }
    }

    fn from_str(s: &str) -> Option<Self> {
        match s {
            "Graceful" => Some(SessionCloseMode::Graceful),
            "Forced" => Some(SessionCloseMode::Forced),
            "Immediate" => Some(SessionCloseMode::Immediate),
            _ => None,
        }
    }
}

/// Unique identity of a managed session.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionIdentity {
    #[pyo3(get)]
    pub session_id: String,
    #[pyo3(get)]
    pub session_type: String,
    #[pyo3(get)]
    pub created_at_ms: u64,
    #[pyo3(get)]
    pub owner_id: Option<String>,
}

#[pymethods]
impl SessionIdentity {
    #[new]
    #[pyo3(signature = (session_id, session_type, created_at_ms, owner_id=None))]
    fn new(
        session_id: String,
        session_type: String,
        created_at_ms: u64,
        owner_id: Option<&str>,
    ) -> Self {
        Self {
            session_id,
            session_type,
            created_at_ms,
            owner_id: owner_id.map(|s| s.to_string()),
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("session_id", &self.session_id)?;
        dict.set_item("session_type", &self.session_type)?;
        dict.set_item("created_at_ms", self.created_at_ms)?;
        dict.set_item("owner_id", &self.owner_id)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "SessionIdentity(session_id={}, session_type={}, created_at_ms={})",
            self.session_id, self.session_type, self.created_at_ms
        )
    }

    fn __str__(&self) -> String {
        format!(
            "Session {} ({})",
            self.session_id, self.session_type
        )
    }
}

/// Cumulative statistics for a managed session.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStats {
    #[pyo3(get)]
    pub total_operations: u64,
    #[pyo3(get)]
    pub completed_operations: u64,
    #[pyo3(get)]
    pub failed_operations: u64,
    #[pyo3(get)]
    pub cancelled_operations: u64,
    #[pyo3(get)]
    pub elapsed_ms: u64,
    #[pyo3(get)]
    pub last_activity_ms: Option<u64>,
}

#[pymethods]
impl SessionStats {
    #[new]
    #[pyo3(signature = (total_operations=0, completed_operations=0, failed_operations=0, cancelled_operations=0, elapsed_ms=0, last_activity_ms=None))]
    fn new(
        total_operations: u64,
        completed_operations: u64,
        failed_operations: u64,
        cancelled_operations: u64,
        elapsed_ms: u64,
        last_activity_ms: Option<u64>,
    ) -> Self {
        Self {
            total_operations,
            completed_operations,
            failed_operations,
            cancelled_operations,
            elapsed_ms,
            last_activity_ms,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("total_operations", self.total_operations)?;
        dict.set_item("completed_operations", self.completed_operations)?;
        dict.set_item("failed_operations", self.failed_operations)?;
        dict.set_item("cancelled_operations", self.cancelled_operations)?;
        dict.set_item("elapsed_ms", self.elapsed_ms)?;
        dict.set_item("last_activity_ms", self.last_activity_ms)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "SessionStats(total={}, completed={}, failed={}, elapsed_ms={})",
            self.total_operations,
            self.completed_operations,
            self.failed_operations,
            self.elapsed_ms
        )
    }

    fn __str__(&self) -> String {
        format!(
            "Session stats: {}/{} completed, {}/{}ms",
            self.completed_operations,
            self.total_operations,
            self.elapsed_ms,
            self.total_operations + self.failed_operations + self.cancelled_operations
        )
    }
}

/// A single event in a session event stream.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionEvent {
    #[pyo3(get)]
    pub sequence: u64,
    #[pyo3(get)]
    pub timestamp_ms: u64,
    #[pyo3(get)]
    pub event_type: String,
    #[pyo3(get)]
    pub message: Option<String>,
}

#[pymethods]
impl SessionEvent {
    #[new]
    #[pyo3(signature = (sequence, timestamp_ms, event_type, message=None))]
    fn new(
        sequence: u64,
        timestamp_ms: u64,
        event_type: String,
        message: Option<&str>,
    ) -> Self {
        Self {
            sequence,
            timestamp_ms,
            event_type,
            message: message.map(|s| s.to_string()),
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("sequence", self.sequence)?;
        dict.set_item("timestamp_ms", self.timestamp_ms)?;
        dict.set_item("event_type", &self.event_type)?;
        dict.set_item("message", &self.message)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "SessionEvent(sequence={}, event_type={})",
            self.sequence, self.event_type
        )
    }

    fn __str__(&self) -> String {
        let msg = self.message.as_deref().unwrap_or("");
        format!(
            "[{}] {} {}",
            self.sequence, self.event_type, msg
        )
    }
}

/// Ordered stream of events for a managed session.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionEventStream {
    #[pyo3(get)]
    pub session_id: String,
    events: Vec<SessionEvent>,
    #[pyo3(get)]
    pub sequence: u64,
}

impl SessionEventStream {
    fn as_str(&self) -> &str {
        "SessionEventStream"
    }
}

#[pymethods]
impl SessionEventStream {
    #[new]
    #[pyo3(signature = (session_id, events=None, sequence=0))]
    fn new(
        session_id: String,
        events: Option<Vec<SessionEvent>>,
        sequence: u64,
    ) -> Self {
        Self {
            session_id,
            events: events.unwrap_or_default(),
            sequence,
        }
    }

    #[getter]
    fn events(&self) -> Vec<SessionEvent> {
        self.events.clone()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("session_id", &self.session_id)?;

        let events_list = PyList::empty_bound(py);
        for e in &self.events {
            events_list.append(e.to_dict(py)?)?;
        }
        dict.set_item("events", events_list)?;

        dict.set_item("sequence", self.sequence)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "SessionEventStream(session_id={}, events={}, sequence={})",
            self.session_id,
            self.events.len(),
            self.sequence
        )
    }

    fn __str__(&self) -> String {
        format!(
            "Event stream for {} ({} events, seq={})",
            self.session_id,
            self.events.len(),
            self.sequence
        )
    }
}

/// Describes the capabilities supported by a managed session type.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionCapabilities {
    #[pyo3(get)]
    pub supports_cancellation: bool,
    #[pyo3(get)]
    pub supports_timeout: bool,
    #[pyo3(get)]
    pub supports_artifacts: bool,
    #[pyo3(get)]
    pub supports_streaming: bool,
    #[pyo3(get)]
    pub max_concurrent_operations: usize,
}

#[pymethods]
impl SessionCapabilities {
    #[new]
    #[pyo3(signature = (supports_cancellation=false, supports_timeout=false, supports_artifacts=false, supports_streaming=false, max_concurrent_operations=1))]
    fn new(
        supports_cancellation: bool,
        supports_timeout: bool,
        supports_artifacts: bool,
        supports_streaming: bool,
        max_concurrent_operations: usize,
    ) -> Self {
        Self {
            supports_cancellation,
            supports_timeout,
            supports_artifacts,
            supports_streaming,
            max_concurrent_operations,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("supports_cancellation", self.supports_cancellation)?;
        dict.set_item("supports_timeout", self.supports_timeout)?;
        dict.set_item("supports_artifacts", self.supports_artifacts)?;
        dict.set_item("supports_streaming", self.supports_streaming)?;
        dict.set_item(
            "max_concurrent_operations",
            self.max_concurrent_operations,
        )?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "SessionCapabilities(cancel={}, timeout={}, artifacts={}, streaming={}, max_ops={})",
            self.supports_cancellation,
            self.supports_timeout,
            self.supports_artifacts,
            self.supports_streaming,
            self.max_concurrent_operations
        )
    }

    fn __str__(&self) -> String {
        format!(
            "Session capabilities: cancel={}, timeout={}, artifacts={}, streaming={}, max_ops={}",
            self.supports_cancellation,
            self.supports_timeout,
            self.supports_artifacts,
            self.supports_streaming,
            self.max_concurrent_operations
        )
    }
}

/// Create a SessionEvent with an auto-incremented sequence number.
///
/// Args:
///     sequence: Current sequence counter (will be incremented by 1 for the new event).
///     timestamp_ms: Event timestamp in milliseconds since epoch.
///     event_type: Type of event (e.g. "state_changed", "operation_started").
///     message: Optional human-readable message.
///
/// Returns:
///     SessionEvent: The new event with sequence = sequence + 1.
#[pyfunction]
pub fn create_session_event(
    sequence: u64,
    timestamp_ms: u64,
    event_type: &str,
    message: Option<&str>,
) -> SessionEvent {
    SessionEvent {
        sequence: sequence + 1,
        timestamp_ms,
        event_type: event_type.to_string(),
        message: message.map(|s| s.to_string()),
    }
}
