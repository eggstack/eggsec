use std::sync::{Arc, Mutex};

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

use crate::event_protocol::EventEnvelope;
use crate::status::{ExecutionStatus, OperationResult};

/// Lifecycle state of a tracked execution.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub enum ExecutionState {
    Pending(),
    Running(),
    Completed(OperationResult),
    Cancelled(String),
    Failed(String),
}

#[pymethods]
impl ExecutionState {
    fn name(&self) -> &'static str {
        match self {
            ExecutionState::Pending() => "Pending",
            ExecutionState::Running() => "Running",
            ExecutionState::Completed(_) => "Completed",
            ExecutionState::Cancelled(_) => "Cancelled",
            ExecutionState::Failed(_) => "Failed",
        }
    }

    fn __repr__(&self) -> String {
        match self {
            ExecutionState::Pending() => "ExecutionState.Pending".to_string(),
            ExecutionState::Running() => "ExecutionState.Running".to_string(),
            ExecutionState::Completed(_) => "ExecutionState.Completed".to_string(),
            ExecutionState::Cancelled(r) => {
                format!("ExecutionState.Cancelled(reason={})", r)
            }
            ExecutionState::Failed(e) => format!("ExecutionState.Failed(error={})", e),
        }
    }
}

impl serde::Serialize for ExecutionState {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        match self {
            ExecutionState::Pending() => {
                let mut s = serializer.serialize_struct("ExecutionState", 1)?;
                s.serialize_field("type", "Pending")?;
                s.end()
            }
            ExecutionState::Running() => {
                let mut s = serializer.serialize_struct("ExecutionState", 1)?;
                s.serialize_field("type", "Running")?;
                s.end()
            }
            ExecutionState::Completed(r) => {
                let mut s = serializer.serialize_struct("ExecutionState", 2)?;
                s.serialize_field("type", "Completed")?;
                s.serialize_field("result", r)?;
                s.end()
            }
            ExecutionState::Cancelled(reason) => {
                let mut s = serializer.serialize_struct("ExecutionState", 2)?;
                s.serialize_field("type", "Cancelled")?;
                s.serialize_field("reason", reason)?;
                s.end()
            }
            ExecutionState::Failed(error) => {
                let mut s = serializer.serialize_struct("ExecutionState", 2)?;
                s.serialize_field("type", "Failed")?;
                s.serialize_field("error", error)?;
                s.end()
            }
        }
    }
}

/// Shared inner state for an execution handle, accessible from any thread.
#[derive(serde::Serialize)]
struct ExecutionHandleInner {
    operation_id: String,
    state: ExecutionState,
    events: Vec<EventEnvelope>,
    started_at_ms: u64,
    sequence: u64,
}

/// A handle to a running or completed execution with full lifecycle tracking.
///
/// Unlike the legacy `ExecutionHandle`, this handle is backed by shared
/// mutable state that can be updated as the operation progresses, providing
/// live status, progress, events, and cancellation support.
#[pyclass]
pub struct TrackedExecutionHandle {
    inner: Arc<Mutex<ExecutionHandleInner>>,
}

#[pymethods]
impl TrackedExecutionHandle {
    /// Create a new tracked execution handle for an operation.
    #[new]
    #[pyo3(signature = (operation_id,))]
    fn new(operation_id: String) -> Self {
        let now_ms = chrono::Utc::now().timestamp_millis() as u64;
        Self {
            inner: Arc::new(Mutex::new(ExecutionHandleInner {
                operation_id,
                state: ExecutionState::Pending(),
                events: Vec::new(),
                started_at_ms: now_ms,
                sequence: 0,
            })),
        }
    }

    /// Get the operation identifier.
    #[getter]
    fn operation_id(&self) -> String {
        self.inner
            .lock()
            .map(|i| i.operation_id.clone())
            .unwrap_or_default()
    }

    /// Get the current execution state.
    #[getter]
    fn state(&self) -> ExecutionState {
        self.inner
            .lock()
            .map(|i| i.state.clone())
            .unwrap_or(ExecutionState::Failed("lock poisoned".to_string()))
    }

    /// Get the current execution status (derived from state).
    #[getter]
    fn status(&self) -> ExecutionStatus {
        let guard = match self.inner.lock() {
            Ok(g) => g,
            Err(_) => {
                return ExecutionStatus::Failed {
                    error: "lock poisoned".to_string(),
                }
            }
        };
        match &guard.state {
            ExecutionState::Pending() => ExecutionStatus::Pending(),
            ExecutionState::Running() => ExecutionStatus::Running(),
            ExecutionState::Completed(r) => r.status.clone(),
            ExecutionState::Cancelled(reason) => ExecutionStatus::Cancelled {
                reason: Some(reason.clone()),
            },
            ExecutionState::Failed(error) => ExecutionStatus::Failed {
                error: error.clone(),
            },
        }
    }

    /// Get the timestamp when execution started (millis since epoch).
    #[getter]
    fn started_at_ms(&self) -> u64 {
        self.inner.lock().map(|i| i.started_at_ms).unwrap_or(0)
    }

    /// Get the number of events emitted for this execution.
    #[getter]
    fn event_count(&self) -> usize {
        self.inner.lock().map(|i| i.events.len()).unwrap_or(0)
    }

    /// Check if the execution is complete (terminal state).
    fn is_complete(&self) -> bool {
        self.inner
            .lock()
            .map(|i| {
                matches!(
                    i.state,
                    ExecutionState::Completed(_)
                        | ExecutionState::Cancelled(_)
                        | ExecutionState::Failed(_)
                )
            })
            .unwrap_or(true)
    }

    /// Check if the execution is currently running or pending.
    fn is_running(&self) -> bool {
        self.inner
            .lock()
            .map(|i| {
                matches!(
                    i.state,
                    ExecutionState::Pending() | ExecutionState::Running()
                )
            })
            .unwrap_or(false)
    }

    /// Get the result, if available (only set in Completed state).
    #[getter]
    fn result(&self) -> PyResult<Option<OperationResult>> {
        self.inner
            .lock()
            .map(|i| match &i.state {
                ExecutionState::Completed(r) => Some(r.clone()),
                _ => None,
            })
            .map_err(|_| pyo3::exceptions::PyRuntimeError::new_err("lock poisoned"))
    }

    /// Get all events emitted for this execution.
    #[getter]
    fn events(&self) -> Vec<EventEnvelope> {
        self.inner
            .lock()
            .map(|i| i.events.clone())
            .unwrap_or_default()
    }

    /// Get a specific event by index.
    fn get_event(&self, index: usize) -> PyResult<EventEnvelope> {
        self.inner
            .lock()
            .map_err(|_| pyo3::exceptions::PyRuntimeError::new_err("lock poisoned"))?
            .events
            .get(index)
            .cloned()
            .ok_or_else(|| {
                pyo3::exceptions::PyIndexError::new_err(format!(
                    "Event index {} out of range",
                    index
                ))
            })
    }

    /// Block until the result is available and return it.
    fn await_result(&self) -> PyResult<OperationResult> {
        let guard = self
            .inner
            .lock()
            .map_err(|_| pyo3::exceptions::PyRuntimeError::new_err("lock poisoned"))?;
        match &guard.state {
            ExecutionState::Completed(r) => Ok(r.clone()),
            ExecutionState::Failed(e) => Err(pyo3::exceptions::PyException::new_err(e.clone())),
            ExecutionState::Cancelled(r) => Err(pyo3::exceptions::PyException::new_err(format!(
                "Execution cancelled: {}",
                r
            ))),
            _ => Err(pyo3::exceptions::PyRuntimeError::new_err(
                "Execution not yet complete",
            )),
        }
    }

    /// Transition the handle to Running state.
    fn mark_running(&self) -> PyResult<()> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| pyo3::exceptions::PyRuntimeError::new_err("lock poisoned"))?;
        if matches!(guard.state, ExecutionState::Pending()) {
            guard.state = ExecutionState::Running();
        }
        Ok(())
    }

    /// Transition the handle to Completed state with a result.
    fn mark_completed(&self, result: OperationResult) -> PyResult<()> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| pyo3::exceptions::PyRuntimeError::new_err("lock poisoned"))?;
        guard.state = ExecutionState::Completed(result);
        Ok(())
    }

    /// Transition the handle to Cancelled state.
    fn mark_cancelled(&self, reason: String) -> PyResult<()> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| pyo3::exceptions::PyRuntimeError::new_err("lock poisoned"))?;
        guard.state = ExecutionState::Cancelled(reason);
        Ok(())
    }

    /// Transition the handle to Failed state.
    fn mark_failed(&self, error: String) -> PyResult<()> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| pyo3::exceptions::PyRuntimeError::new_err("lock poisoned"))?;
        guard.state = ExecutionState::Failed(error);
        Ok(())
    }

    /// Append an event to the handle's event log.
    fn push_event(&self, event: EventEnvelope) -> PyResult<()> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| pyo3::exceptions::PyRuntimeError::new_err("lock poisoned"))?;
        guard.sequence += 1;
        guard.events.push(event);
        Ok(())
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let guard = self
            .inner
            .lock()
            .map_err(|_| pyo3::exceptions::PyRuntimeError::new_err("lock poisoned"))?;
        let dict = PyDict::new_bound(py);
        dict.set_item("operation_id", &guard.operation_id)?;
        dict.set_item("state", guard.state.name())?;
        dict.set_item("started_at_ms", guard.started_at_ms)?;
        dict.set_item("event_count", guard.events.len())?;

        match &guard.state {
            ExecutionState::Completed(r) => {
                dict.set_item("result", r.to_dict(py)?)?;
            }
            ExecutionState::Cancelled(reason) => {
                dict.set_item("cancel_reason", reason)?;
            }
            ExecutionState::Failed(error) => {
                dict.set_item("error", error)?;
            }
            _ => {}
        }

        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        let guard = self
            .inner
            .lock()
            .map_err(|_| pyo3::exceptions::PyRuntimeError::new_err("lock poisoned"))?;
        serde_json::to_string(&*guard)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        let op = self
            .inner
            .lock()
            .map(|i| i.operation_id.clone())
            .unwrap_or_default();
        let state = self
            .inner
            .lock()
            .map(|i| i.state.name().to_string())
            .unwrap_or_else(|_| "unknown".to_string());
        format!("TrackedExecutionHandle(operation={}, state={})", op, state)
    }

    /// Context manager __enter__.
    fn __enter__(slf: Py<Self>) -> Py<Self> {
        slf
    }

    /// Context manager __exit__.
    #[pyo3(signature = (_exc_type=None, _exc_value=None, _traceback=None))]
    fn __exit__(
        &self,
        _exc_type: Option<&Bound<'_, PyAny>>,
        _exc_value: Option<&Bound<'_, PyAny>>,
        _traceback: Option<&Bound<'_, PyAny>>,
    ) -> bool {
        false
    }
}

impl serde::Serialize for TrackedExecutionHandle {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let guard = self.inner.lock().map_err(serde::ser::Error::custom)?;
        let mut s = serializer.serialize_struct("TrackedExecutionHandle", 4)?;
        s.serialize_field("operation_id", &guard.operation_id)?;
        s.serialize_field("state", &guard.state)?;
        s.serialize_field("started_at_ms", &guard.started_at_ms)?;
        s.serialize_field("event_count", &guard.events.len())?;
        s.end()
    }
}

/// Helper to emit a lifecycle event and optionally push it to a handle.
pub(crate) fn emit_lifecycle_event(
    event_tx: &Option<crate::engine_state::EventSender>,
    handle: Option<&TrackedExecutionHandle>,
    event_type: &str,
    payload: PyObject,
    py: Python<'_>,
) -> PyResult<()> {
    let event = crate::event_protocol::wrap_event(py, event_type.to_string(), payload, None, None)?;

    // Push to handle if provided
    if let Some(h) = handle {
        h.push_event(event.clone())?;
    }

    // Send on event channel if configured
    if let Some(tx) = event_tx {
        let _ = tx.try_send(event);
    }

    Ok(())
}
