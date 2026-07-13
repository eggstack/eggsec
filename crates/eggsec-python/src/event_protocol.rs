use std::sync::atomic::{AtomicU64, Ordering};

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

use crate::handles::ExecutionEvent;

/// Event schema version for compatibility.
pub const EVENT_SCHEMA_VERSION: &str = "1.0.0";

/// Global monotonic event sequence counter.
/// Starts at 1 so the first event has sequence=1.
static EVENT_SEQUENCE: AtomicU64 = AtomicU64::new(1);

/// Allocate a new monotonic sequence number.
fn next_sequence() -> u64 {
    EVENT_SEQUENCE.fetch_add(1, Ordering::Relaxed)
}

/// Base event envelope with version metadata.
///
/// Wraps any typed event payload with schema version, event ID,
/// timestamp, and correlation tracking for backward compatibility.
#[pyclass]
pub struct EventEnvelope {
    #[pyo3(get)]
    pub schema_version: String,
    #[pyo3(get)]
    pub event_id: String,
    /// Monotonic sequence within the producing execution stream.
    #[pyo3(get)]
    pub sequence: u64,
    #[pyo3(get)]
    pub timestamp_ms: u64,
    #[pyo3(get)]
    pub correlation_id: Option<String>,
    #[pyo3(get)]
    pub event_type: String,
    #[pyo3(get)]
    pub payload: PyObject,
}

impl std::fmt::Debug for EventEnvelope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventEnvelope")
            .field("event_id", &self.event_id)
            .field("sequence", &self.sequence)
            .field("event_type", &self.event_type)
            .field("timestamp_ms", &self.timestamp_ms)
            .field("correlation_id", &self.correlation_id)
            .field("schema_version", &self.schema_version)
            .finish()
    }
}

impl serde::Serialize for EventEnvelope {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("EventEnvelope", 7)?;
        s.serialize_field("schema_version", &self.schema_version)?;
        s.serialize_field("event_id", &self.event_id)?;
        s.serialize_field("sequence", &self.sequence)?;
        s.serialize_field("timestamp_ms", &self.timestamp_ms)?;
        s.serialize_field("correlation_id", &self.correlation_id)?;
        s.serialize_field("event_type", &self.event_type)?;
        // payload is a PyObject — serialize as null for JSON transport
        s.serialize_field("payload", &())?;
        s.end()
    }
}

impl Clone for EventEnvelope {
    fn clone(&self) -> Self {
        Python::with_gil(|py| Self {
            schema_version: self.schema_version.clone(),
            event_id: self.event_id.clone(),
            sequence: self.sequence,
            timestamp_ms: self.timestamp_ms,
            correlation_id: self.correlation_id.clone(),
            event_type: self.event_type.clone(),
            payload: self.payload.clone_ref(py),
        })
    }
}

#[pymethods]
impl EventEnvelope {
    #[new]
    #[pyo3(signature = (event_type, payload, *, event_id=None, timestamp_ms=None, correlation_id=None, schema_version=None))]
    pub(crate) fn new(
        _py: Python<'_>,
        event_type: String,
        payload: PyObject,
        event_id: Option<String>,
        timestamp_ms: Option<u64>,
        correlation_id: Option<String>,
        schema_version: Option<String>,
    ) -> Self {
        let now_ms = chrono::Utc::now().timestamp_millis() as u64;
        Self {
            schema_version: schema_version.unwrap_or_else(|| EVENT_SCHEMA_VERSION.to_string()),
            event_id: event_id.unwrap_or_else(|| format!("evt-{}", now_ms)),
            sequence: next_sequence(),
            timestamp_ms: timestamp_ms.unwrap_or(now_ms),
            correlation_id,
            event_type,
            payload,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        self.to_dict_impl(py)
    }

    fn to_json(&self, py: Python) -> PyResult<String> {
        self.to_json_impl(py)
    }

    fn __repr__(&self) -> String {
        format!(
            "EventEnvelope(type={}, id={}, ts={})",
            self.event_type, self.event_id, self.timestamp_ms
        )
    }

    fn __hash__(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        self.event_id.hash(&mut hasher);
        self.event_type.hash(&mut hasher);
        self.timestamp_ms.hash(&mut hasher);
        hasher.finish()
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.event_id == other.event_id
    }

    #[staticmethod]
    pub(crate) fn from_legacy(py: Python<'_>, event: &ExecutionEvent) -> PyResult<EventEnvelope> {
        let payload = event.to_dict_impl(py)?;
        Ok(EventEnvelope {
            schema_version: EVENT_SCHEMA_VERSION.to_string(),
            event_id: format!("evt-{}", event.timestamp_ms),
            sequence: next_sequence(),
            timestamp_ms: event.timestamp_ms,
            correlation_id: None,
            event_type: event.event_type.clone(),
            payload,
        })
    }
}

impl EventEnvelope {
    /// Create a new EventEnvelope (crate-internal, bypasses #[new] visibility).
    pub(crate) fn create(
        event_type: String,
        payload: PyObject,
        event_id: Option<String>,
        timestamp_ms: Option<u64>,
        correlation_id: Option<String>,
        schema_version: Option<String>,
    ) -> Self {
        let now_ms = chrono::Utc::now().timestamp_millis() as u64;
        Self {
            schema_version: schema_version.unwrap_or_else(|| EVENT_SCHEMA_VERSION.to_string()),
            event_id: event_id.unwrap_or_else(|| format!("evt-{}", now_ms)),
            sequence: next_sequence(),
            timestamp_ms: timestamp_ms.unwrap_or(now_ms),
            correlation_id,
            event_type,
            payload,
        }
    }

    pub(crate) fn to_dict_impl(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("schema_version", &self.schema_version)?;
        dict.set_item("event_id", &self.event_id)?;
        dict.set_item("sequence", self.sequence)?;
        dict.set_item("timestamp_ms", self.timestamp_ms)?;
        dict.set_item("correlation_id", &self.correlation_id)?;
        dict.set_item("event_type", &self.event_type)?;
        dict.set_item("payload", &self.payload)?;
        Ok(dict.into())
    }

    pub(crate) fn to_json_impl(&self, py: Python) -> PyResult<String> {
        let val = self.to_json_value(py)?;
        serde_json::to_string(&val)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn to_json_value(&self, py: Python) -> PyResult<serde_json::Value> {
        let mut map = serde_json::Map::new();
        map.insert(
            "schema_version".into(),
            serde_json::Value::String(self.schema_version.clone()),
        );
        map.insert(
            "event_id".into(),
            serde_json::Value::String(self.event_id.clone()),
        );
        map.insert(
            "sequence".into(),
            serde_json::Value::Number(self.sequence.into()),
        );
        map.insert(
            "timestamp_ms".into(),
            serde_json::Value::Number(self.timestamp_ms.into()),
        );
        map.insert(
            "correlation_id".into(),
            match &self.correlation_id {
                Some(s) => serde_json::Value::String(s.clone()),
                None => serde_json::Value::Null,
            },
        );
        map.insert(
            "event_type".into(),
            serde_json::Value::String(self.event_type.clone()),
        );
        map.insert("payload".into(), pyobject_to_json(py, &self.payload)?);
        Ok(serde_json::Value::Object(map))
    }
}

fn pyobject_to_json(py: Python<'_>, obj: &PyObject) -> PyResult<serde_json::Value> {
    let any = obj.bind(py);
    if any.is_none() {
        return Ok(serde_json::Value::Null);
    }
    if let Ok(v) = any.extract::<bool>() {
        return Ok(serde_json::Value::Bool(v));
    }
    if let Ok(v) = any.extract::<i64>() {
        return Ok(serde_json::Value::Number(v.into()));
    }
    if let Ok(v) = any.extract::<u64>() {
        return Ok(serde_json::Value::Number(v.into()));
    }
    if let Ok(v) = any.extract::<f64>() {
        if let Some(n) = serde_json::Number::from_f64(v) {
            return Ok(serde_json::Value::Number(n));
        }
    }
    if let Ok(v) = any.extract::<String>() {
        return Ok(serde_json::Value::String(v));
    }
    if let Ok(d) = any.downcast::<PyDict>() {
        let mut map = serde_json::Map::new();
        for (key, value) in d.iter() {
            if let Ok(key_str) = key.extract::<String>() {
                map.insert(key_str, pyobject_to_json(py, &value.into())?);
            }
        }
        return Ok(serde_json::Value::Object(map));
    }
    if let Ok(list) = any.downcast::<PyList>() {
        let mut arr = Vec::new();
        for item in list.iter() {
            arr.push(pyobject_to_json(py, &item.into())?);
        }
        return Ok(serde_json::Value::Array(arr));
    }
    Ok(serde_json::Value::String(format!("{:?}", any)))
}

/// Typed event: operation planning initiated.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct PlanningEvent {
    #[pyo3(get)]
    pub operation_id: String,
    #[pyo3(get)]
    pub target: String,
    #[pyo3(get)]
    pub scope_summary: String,
}

#[pymethods]
impl PlanningEvent {
    #[new]
    pub(crate) fn new(operation_id: String, target: String, scope_summary: String) -> Self {
        Self {
            operation_id,
            target,
            scope_summary,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("operation_id", &self.operation_id)?;
        dict.set_item("target", &self.target)?;
        dict.set_item("scope_summary", &self.scope_summary)?;
        Ok(dict.into())
    }

    fn to_json(&self, py: Python) -> PyResult<String> {
        serde_json::to_string(&serde_json::json!({
            "operation_id": self.operation_id,
            "target": self.target,
            "scope_summary": self.scope_summary,
        }))
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "PlanningEvent(op={}, target={})",
            self.operation_id, self.target
        )
    }
}

/// Typed event: preflight evaluation completed.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct PreflightEvent {
    #[pyo3(get)]
    pub outcome: String,
    #[pyo3(get)]
    pub confirmations_required: Vec<String>,
    #[pyo3(get)]
    pub suggested_flags: Vec<String>,
}

#[pymethods]
impl PreflightEvent {
    #[new]
    pub(crate) fn new(
        outcome: String,
        confirmations_required: Vec<String>,
        suggested_flags: Vec<String>,
    ) -> Self {
        Self {
            outcome,
            confirmations_required,
            suggested_flags,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("outcome", &self.outcome)?;
        dict.set_item("confirmations_required", &self.confirmations_required)?;
        dict.set_item("suggested_flags", &self.suggested_flags)?;
        Ok(dict.into())
    }

    fn to_json(&self, py: Python) -> PyResult<String> {
        serde_json::to_string(&serde_json::json!({
            "outcome": self.outcome,
            "confirmations_required": self.confirmations_required,
            "suggested_flags": self.suggested_flags,
        }))
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!("PreflightEvent(outcome={})", self.outcome)
    }
}

/// Typed event: pipeline stage lifecycle change.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct StageLifecycleEvent {
    #[pyo3(get)]
    pub stage: String,
    #[pyo3(get)]
    pub status: String,
}

#[pymethods]
impl StageLifecycleEvent {
    #[new]
    pub(crate) fn new(stage: String, status: String) -> Self {
        Self { stage, status }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("stage", &self.stage)?;
        dict.set_item("status", &self.status)?;
        Ok(dict.into())
    }

    fn to_json(&self, py: Python) -> PyResult<String> {
        serde_json::to_string(&serde_json::json!({
            "stage": self.stage,
            "status": self.status,
        }))
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "StageLifecycleEvent(stage={}, status={})",
            self.stage, self.status
        )
    }
}

/// Typed event: operation progress update.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct ProgressEvent {
    #[pyo3(get)]
    pub percentage: f64,
    #[pyo3(get)]
    pub message: String,
    #[pyo3(get)]
    pub items_processed: usize,
    #[pyo3(get)]
    pub items_total: usize,
}

#[pymethods]
impl ProgressEvent {
    #[new]
    pub(crate) fn new(
        percentage: f64,
        message: String,
        items_processed: usize,
        items_total: usize,
    ) -> Self {
        Self {
            percentage,
            message,
            items_processed,
            items_total,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("percentage", self.percentage)?;
        dict.set_item("message", &self.message)?;
        dict.set_item("items_processed", self.items_processed)?;
        dict.set_item("items_total", self.items_total)?;
        Ok(dict.into())
    }

    fn to_json(&self, py: Python) -> PyResult<String> {
        serde_json::to_string(&serde_json::json!({
            "percentage": self.percentage,
            "message": self.message,
            "items_processed": self.items_processed,
            "items_total": self.items_total,
        }))
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "ProgressEvent({}%, {}/{})",
            self.percentage, self.items_processed, self.items_total
        )
    }
}

/// Typed event: finding discovered.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct FindingEvent {
    #[pyo3(get)]
    pub finding_id: String,
    #[pyo3(get)]
    pub severity: String,
    #[pyo3(get)]
    pub title: String,
    #[pyo3(get)]
    pub auto_added: bool,
}

#[pymethods]
impl FindingEvent {
    #[new]
    pub(crate) fn new(
        finding_id: String,
        severity: String,
        title: String,
        auto_added: bool,
    ) -> Self {
        Self {
            finding_id,
            severity,
            title,
            auto_added,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("finding_id", &self.finding_id)?;
        dict.set_item("severity", &self.severity)?;
        dict.set_item("title", &self.title)?;
        dict.set_item("auto_added", self.auto_added)?;
        Ok(dict.into())
    }

    fn to_json(&self, py: Python) -> PyResult<String> {
        serde_json::to_string(&serde_json::json!({
            "finding_id": self.finding_id,
            "severity": self.severity,
            "title": self.title,
            "auto_added": self.auto_added,
        }))
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "FindingEvent(id={}, severity={}, title={})",
            self.finding_id, self.severity, self.title
        )
    }
}

/// Typed event: artifact produced.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct ArtifactEvent {
    #[pyo3(get)]
    pub artifact_name: String,
    #[pyo3(get)]
    pub kind: String,
    #[pyo3(get)]
    pub mime_type: String,
    #[pyo3(get)]
    pub size_bytes: u64,
}

#[pymethods]
impl ArtifactEvent {
    #[new]
    pub(crate) fn new(
        artifact_name: String,
        kind: String,
        mime_type: String,
        size_bytes: u64,
    ) -> Self {
        Self {
            artifact_name,
            kind,
            mime_type,
            size_bytes,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("artifact_name", &self.artifact_name)?;
        dict.set_item("kind", &self.kind)?;
        dict.set_item("mime_type", &self.mime_type)?;
        dict.set_item("size_bytes", self.size_bytes)?;
        Ok(dict.into())
    }

    fn to_json(&self, py: Python) -> PyResult<String> {
        serde_json::to_string(&serde_json::json!({
            "artifact_name": self.artifact_name,
            "kind": self.kind,
            "mime_type": self.mime_type,
            "size_bytes": self.size_bytes,
        }))
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "ArtifactEvent(name={}, kind={}, size={})",
            self.artifact_name, self.kind, self.size_bytes
        )
    }
}

/// Typed event: operation cancelled.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct CancellationEvent {
    #[pyo3(get)]
    pub reason: String,
    #[pyo3(get)]
    pub cancelled_by: String,
}

#[pymethods]
impl CancellationEvent {
    #[new]
    pub(crate) fn new(reason: String, cancelled_by: String) -> Self {
        Self {
            reason,
            cancelled_by,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("reason", &self.reason)?;
        dict.set_item("cancelled_by", &self.cancelled_by)?;
        Ok(dict.into())
    }

    fn to_json(&self, py: Python) -> PyResult<String> {
        serde_json::to_string(&serde_json::json!({
            "reason": self.reason,
            "cancelled_by": self.cancelled_by,
        }))
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "CancellationEvent(reason={}, by={})",
            self.reason, self.cancelled_by
        )
    }
}

/// Typed event: operation failed.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct FailureEvent {
    #[pyo3(get)]
    pub error_type: String,
    #[pyo3(get)]
    pub error_message: String,
    #[pyo3(get)]
    pub is_retryable: bool,
}

#[pymethods]
impl FailureEvent {
    #[new]
    pub(crate) fn new(error_type: String, error_message: String, is_retryable: bool) -> Self {
        Self {
            error_type,
            error_message,
            is_retryable,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("error_type", &self.error_type)?;
        dict.set_item("error_message", &self.error_message)?;
        dict.set_item("is_retryable", self.is_retryable)?;
        Ok(dict.into())
    }

    fn to_json(&self, py: Python) -> PyResult<String> {
        serde_json::to_string(&serde_json::json!({
            "error_type": self.error_type,
            "error_message": self.error_message,
            "is_retryable": self.is_retryable,
        }))
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "FailureEvent(type={}, retryable={})",
            self.error_type, self.is_retryable
        )
    }
}

/// Typed event: operation completed.
#[pyclass(frozen)]
pub struct CompletionEvent {
    #[pyo3(get)]
    pub status: String,
    #[pyo3(get)]
    pub stats: Option<PyObject>,
    #[pyo3(get)]
    pub duration_ms: u64,
}

#[pymethods]
impl CompletionEvent {
    #[new]
    #[pyo3(signature = (status, stats, duration_ms))]
    pub(crate) fn new(
        _py: Python<'_>,
        status: String,
        stats: Option<PyObject>,
        duration_ms: u64,
    ) -> Self {
        Self {
            status,
            stats,
            duration_ms,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("status", &self.status)?;
        dict.set_item("stats", &self.stats)?;
        dict.set_item("duration_ms", self.duration_ms)?;
        Ok(dict.into())
    }

    fn to_json(&self, py: Python) -> PyResult<String> {
        let val = serde_json::json!({
            "status": self.status,
            "duration_ms": self.duration_ms,
        });
        serde_json::to_string(&val)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "CompletionEvent(status={}, duration_ms={})",
            self.status, self.duration_ms
        )
    }
}

/// Wrap a typed event payload into an EventEnvelope.
#[pyfunction]
#[pyo3(signature = (event_type, payload, *, correlation_id=None, event_id=None))]
pub fn wrap_event(
    _py: Python<'_>,
    event_type: String,
    payload: PyObject,
    correlation_id: Option<String>,
    event_id: Option<String>,
) -> PyResult<EventEnvelope> {
    let now_ms = chrono::Utc::now().timestamp_millis() as u64;
    Ok(EventEnvelope {
        schema_version: EVENT_SCHEMA_VERSION.to_string(),
        event_id: event_id.unwrap_or_else(|| format!("evt-{}", now_ms)),
        sequence: next_sequence(),
        timestamp_ms: now_ms,
        correlation_id,
        event_type,
        payload,
    })
}
