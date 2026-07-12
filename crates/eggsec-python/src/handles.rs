use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

use crate::status::{ExecutionStatus, OperationResult};

/// A handle to a running or completed execution.
///
/// Tracks the lifecycle of an operation and provides access to results.
#[pyclass]
pub struct ExecutionHandle {
    handle_id: String,
    status: ExecutionStatus,
    result: Option<OperationResult>,
}

#[pymethods]
impl ExecutionHandle {
    /// Create a new execution handle.
    #[new]
    #[pyo3(signature = (handle_id, status=None, result=None))]
    fn new(
        handle_id: String,
        status: Option<ExecutionStatus>,
        result: Option<OperationResult>,
    ) -> Self {
        Self {
            handle_id,
            status: status.unwrap_or(ExecutionStatus::Pending()),
            result,
        }
    }

    /// Get the unique handle identifier.
    #[getter]
    fn handle_id(&self) -> String {
        self.handle_id.clone()
    }

    /// Get the current execution status.
    #[getter]
    fn status(&self) -> ExecutionStatus {
        self.status.clone()
    }

    /// Check if the execution is complete (success or failure).
    fn is_complete(&self) -> bool {
        matches!(
            self.status,
            ExecutionStatus::Completed()
                | ExecutionStatus::Failed { .. }
                | ExecutionStatus::Cancelled { .. }
                | ExecutionStatus::Timeout { .. }
        )
    }

    /// Check if the execution is currently running or pending.
    fn is_running(&self) -> bool {
        matches!(
            self.status,
            ExecutionStatus::Pending() | ExecutionStatus::Running()
        )
    }

    /// Get the result, if available.
    #[getter]
    fn result(&self) -> Option<OperationResult> {
        self.result.clone()
    }

    /// Block until the result is available and return it.
    ///
    /// For local handles this returns the result immediately if complete,
    /// or blocks briefly polling. For remote handles this would block
    /// on a channel.
    fn await_result(&self, _py: Python<'_>) -> PyResult<OperationResult> {
        match &self.result {
            Some(r) => Ok(r.clone()),
            None => {
                if self.is_complete() {
                    // Should not happen — result should be set on completion
                    Err(pyo3::exceptions::PyRuntimeError::new_err(
                        "Execution completed but no result available",
                    ))
                } else {
                    Err(pyo3::exceptions::PyRuntimeError::new_err(
                        "Execution not yet complete",
                    ))
                }
            }
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("handle_id", &self.handle_id)?;
        dict.set_item("status", self.status.name())?;
        match &self.result {
            Some(r) => dict.set_item("result", r.to_dict(py)?)?,
            None => dict.set_item("result", py.None())?,
        }
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "ExecutionHandle(id={}, status={})",
            self.handle_id,
            self.status.name()
        )
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

impl serde::Serialize for ExecutionHandle {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("ExecutionHandle", 3)?;
        s.serialize_field("handle_id", &self.handle_id)?;
        s.serialize_field("status", &self.status)?;
        s.serialize_field("result", &self.result)?;
        s.end()
    }
}

/// A frozen event produced during execution.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct ExecutionEvent {
    #[pyo3(get)]
    pub handle_id: String,
    #[pyo3(get)]
    pub event_type: String,
    #[pyo3(get)]
    pub timestamp_ms: u64,
    #[pyo3(get)]
    pub detail: Option<String>,
}

#[pymethods]
impl ExecutionEvent {
    /// Create a new execution event.
    #[new]
    #[pyo3(signature = (handle_id, event_type, timestamp_ms, *, detail=None))]
    fn new(
        handle_id: String,
        event_type: String,
        timestamp_ms: u64,
        detail: Option<String>,
    ) -> Self {
        Self {
            handle_id,
            event_type,
            timestamp_ms,
            detail,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("handle_id", &self.handle_id)?;
        dict.set_item("event_type", &self.event_type)?;
        dict.set_item("timestamp_ms", self.timestamp_ms)?;
        dict.set_item("detail", &self.detail)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "ExecutionEvent(handle={}, type={}, ts={})",
            self.handle_id, self.event_type, self.timestamp_ms
        )
    }

    fn __hash__(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        self.handle_id.hash(&mut hasher);
        self.event_type.hash(&mut hasher);
        self.timestamp_ms.hash(&mut hasher);
        hasher.finish()
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.handle_id == other.handle_id
            && self.event_type == other.event_type
            && self.timestamp_ms == other.timestamp_ms
    }
}

impl serde::Serialize for ExecutionEvent {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("ExecutionEvent", 4)?;
        s.serialize_field("handle_id", &self.handle_id)?;
        s.serialize_field("event_type", &self.event_type)?;
        s.serialize_field("timestamp_ms", &self.timestamp_ms)?;
        s.serialize_field("detail", &self.detail)?;
        s.end()
    }
}

impl ExecutionEvent {
    /// Serialize to a Python dict (crate-internal).
    pub(crate) fn to_dict_impl(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("handle_id", &self.handle_id)?;
        dict.set_item("event_type", &self.event_type)?;
        dict.set_item("timestamp_ms", self.timestamp_ms)?;
        dict.set_item("detail", &self.detail)?;
        Ok(dict.into())
    }
}

/// A log of execution events.
#[pyclass]
pub struct EventLog {
    events: Vec<ExecutionEvent>,
}

#[pymethods]
impl EventLog {
    /// Create a new empty event log.
    #[new]
    fn new() -> Self {
        Self { events: Vec::new() }
    }

    /// Push an event onto the log.
    fn push(&mut self, event: ExecutionEvent) {
        self.events.push(event);
    }

    /// Get the number of events in the log.
    fn len(&self) -> usize {
        self.events.len()
    }

    /// Check if the log is empty.
    fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Get an event by index.
    fn get(&self, i: usize) -> PyResult<ExecutionEvent> {
        self.events.get(i).cloned().ok_or_else(|| {
            pyo3::exceptions::PyIndexError::new_err(format!("Index {} out of range", i))
        })
    }

    /// Convert the log to a Python list of dicts.
    fn to_list(&self, py: Python) -> PyResult<PyObject> {
        let list = PyList::empty_bound(py);
        for event in &self.events {
            list.append(event.to_dict(py)?)?;
        }
        Ok(list.into())
    }

    /// Filter events by event type.
    fn filter_by_type(&self, event_type: &str) -> Vec<ExecutionEvent> {
        self.events
            .iter()
            .filter(|e| e.event_type == event_type)
            .cloned()
            .collect()
    }

    /// Return events wrapped in versioned EventEnvelope dicts.
    fn to_versioned_list(&self, py: Python) -> PyResult<PyObject> {
        let list = PyList::empty_bound(py);
        for event in &self.events {
            let env = crate::event_protocol::EventEnvelope::from_legacy(py, event)?;
            list.append(env.to_dict_impl(py)?)?;
        }
        Ok(list.into())
    }

    /// Return the current event schema version.
    fn schema_version(&self) -> String {
        crate::event_protocol::EVENT_SCHEMA_VERSION.to_string()
    }

    /// Take all events from the log and clear it.
    /// Returns the events wrapped in EventEnvelope dicts.
    fn drain(&mut self, py: Python) -> PyResult<PyObject> {
        let events: Vec<ExecutionEvent> = self.events.drain(..).collect();
        let list = PyList::empty_bound(py);
        for event in &events {
            let env = crate::event_protocol::EventEnvelope::from_legacy(py, event)?;
            list.append(env.to_dict_impl(py)?)?;
        }
        Ok(list.into())
    }

    fn __repr__(&self) -> String {
        format!("EventLog(events={})", self.events.len())
    }

    fn __len__(&self) -> usize {
        self.events.len()
    }

    /// Iterate over events (yields ExecutionEvent objects).
    fn __iter__<'py>(slf: PyRef<'py, Self>, py: Python<'py>) -> PyResult<PyObject> {
        let list = PyList::empty_bound(py);
        for event in slf.events.iter() {
            let obj = event.clone().into_py(py);
            list.append(obj)?;
        }
        list.call_method0("__iter__").map(|o| o.into())
    }

    /// Create a lazy iterator that yields events one at a time.
    fn iter_lazy(&self) -> LazyEventIterator {
        LazyEventIterator::new(self.events.clone())
    }

    /// Check if an event with the given handle_id exists in the log.
    fn __contains__(&self, event: &ExecutionEvent) -> bool {
        self.events.iter().any(|e| {
            e.handle_id == event.handle_id
                && e.event_type == event.event_type
                && e.timestamp_ms == event.timestamp_ms
        })
    }

    /// Create an async iterator for this EventLog.
    fn __aiter__(slf: Py<Self>) -> Py<Self> {
        slf
    }

    fn __anext__<'py>(slf: PyRef<'py, Self>, _py: Python<'py>) -> PyResult<Option<PyObject>> {
        // For a non-async context, return None immediately (empty async iterator)
        Ok(None)
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

impl EventLog {
    /// Access the raw events (crate-internal).
    pub(crate) fn events(&self) -> &[ExecutionEvent] {
        &self.events
    }

    /// Number of events (crate-internal).
    pub(crate) fn event_count(&self) -> usize {
        self.events.len()
    }
}

/// Lazy iterator for EventLog — yields events one at a time without materializing a list.
#[pyclass(name = "LazyEventIterator")]
pub struct LazyEventIterator {
    events: Vec<ExecutionEvent>,
    index: usize,
}

#[pymethods]
impl LazyEventIterator {
    #[new]
    fn new(events: Vec<ExecutionEvent>) -> Self {
        Self { events, index: 0 }
    }

    fn __iter__(slf: Py<Self>) -> Py<Self> {
        slf
    }

    fn __next__<'py>(mut slf: PyRefMut<'py, Self>, py: Python<'py>) -> PyResult<Option<PyObject>> {
        if slf.index >= slf.events.len() {
            return Ok(None);
        }
        let event = slf.events[slf.index].clone();
        slf.index += 1;
        Ok(Some(event.into_py(py)))
    }

    fn __len__(&self) -> usize {
        self.events.len() - self.index
    }
}
