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

// ═══════════════════════════════════════════════════════════════════
// WS11: Network-specific event types
// ═══════════════════════════════════════════════════════════════════

/// Typed event: DNS resolution started or completed.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct ResolutionEvent {
    #[pyo3(get)]
    pub target: String,
    #[pyo3(get)]
    pub resolved_address: Option<String>,
    #[pyo3(get)]
    pub status: String,
    #[pyo3(get)]
    pub resolution_time_ms: Option<f64>,
}

#[pymethods]
impl ResolutionEvent {
    #[new]
    #[pyo3(signature = (target, status, resolved_address=None, resolution_time_ms=None))]
    pub(crate) fn new(
        target: String,
        status: String,
        resolved_address: Option<String>,
        resolution_time_ms: Option<f64>,
    ) -> Self {
        Self {
            target,
            resolved_address,
            status,
            resolution_time_ms,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("target", &self.target)?;
        dict.set_item("resolved_address", &self.resolved_address)?;
        dict.set_item("status", &self.status)?;
        dict.set_item("resolution_time_ms", &self.resolution_time_ms)?;
        Ok(dict.into())
    }

    fn to_json(&self, py: Python) -> PyResult<String> {
        let val = serde_json::json!({
            "target": self.target,
            "resolved_address": self.resolved_address,
            "status": self.status,
            "resolution_time_ms": self.resolution_time_ms,
        });
        serde_json::to_string(&val)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "ResolutionEvent(target={}, status={})",
            self.target, self.status
        )
    }
}

/// Typed event: TCP connection started or completed.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct ConnectionEvent {
    #[pyo3(get)]
    pub target: String,
    #[pyo3(get)]
    pub port: u16,
    #[pyo3(get)]
    pub status: String,
    #[pyo3(get)]
    pub rtt_ms: Option<f64>,
}

#[pymethods]
impl ConnectionEvent {
    #[new]
    pub(crate) fn new(target: String, port: u16, status: String, rtt_ms: Option<f64>) -> Self {
        Self {
            target,
            port,
            status,
            rtt_ms,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("target", &self.target)?;
        dict.set_item("port", self.port)?;
        dict.set_item("status", &self.status)?;
        dict.set_item("rtt_ms", &self.rtt_ms)?;
        Ok(dict.into())
    }

    fn to_json(&self, _py: Python) -> PyResult<String> {
        let val = serde_json::json!({
            "target": self.target,
            "port": self.port,
            "status": self.status,
            "rtt_ms": self.rtt_ms,
        });
        serde_json::to_string(&val)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "ConnectionEvent({}:{}, status={})",
            self.target, self.port, self.status
        )
    }
}

/// Typed event: protocol probe response received.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct ProbeEvent {
    #[pyo3(get)]
    pub probe_type: String,
    #[pyo3(get)]
    pub target: String,
    #[pyo3(get)]
    pub success: bool,
    #[pyo3(get)]
    pub rtt_ms: Option<f64>,
}

#[pymethods]
impl ProbeEvent {
    #[new]
    pub(crate) fn new(
        probe_type: String,
        target: String,
        success: bool,
        rtt_ms: Option<f64>,
    ) -> Self {
        Self {
            probe_type,
            target,
            success,
            rtt_ms,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("probe_type", &self.probe_type)?;
        dict.set_item("target", &self.target)?;
        dict.set_item("success", self.success)?;
        dict.set_item("rtt_ms", &self.rtt_ms)?;
        Ok(dict.into())
    }

    fn to_json(&self, _py: Python) -> PyResult<String> {
        let val = serde_json::json!({
            "probe_type": self.probe_type,
            "target": self.target,
            "success": self.success,
            "rtt_ms": self.rtt_ms,
        });
        serde_json::to_string(&val)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "ProbeEvent(type={}, target={}, success={})",
            self.probe_type, self.target, self.success
        )
    }
}

/// Typed event: WebSocket message received.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct WebSocketMessageEvent {
    #[pyo3(get)]
    pub url: String,
    #[pyo3(get)]
    pub direction: String,
    #[pyo3(get)]
    pub message_type: String,
    #[pyo3(get)]
    pub size: usize,
}

#[pymethods]
impl WebSocketMessageEvent {
    #[new]
    pub(crate) fn new(url: String, direction: String, message_type: String, size: usize) -> Self {
        Self {
            url,
            direction,
            message_type,
            size,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("url", &self.url)?;
        dict.set_item("direction", &self.direction)?;
        dict.set_item("message_type", &self.message_type)?;
        dict.set_item("size", self.size)?;
        Ok(dict.into())
    }

    fn to_json(&self, _py: Python) -> PyResult<String> {
        let val = serde_json::json!({
            "url": self.url,
            "direction": self.direction,
            "message_type": self.message_type,
            "size": self.size,
        });
        serde_json::to_string(&val)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "WebSocketMessageEvent(url={}, dir={}, type={}, size={})",
            self.url, self.direction, self.message_type, self.size
        )
    }
}

/// Typed event: capture statistics update.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct CaptureStatsEvent {
    #[pyo3(get)]
    pub interface: String,
    #[pyo3(get)]
    pub packets_captured: u64,
    #[pyo3(get)]
    pub packets_dropped: u64,
    #[pyo3(get)]
    pub bytes_captured: u64,
    #[pyo3(get)]
    pub runtime_ms: u64,
}

#[pymethods]
impl CaptureStatsEvent {
    #[new]
    pub(crate) fn new(
        interface: String,
        packets_captured: u64,
        packets_dropped: u64,
        bytes_captured: u64,
        runtime_ms: u64,
    ) -> Self {
        Self {
            interface,
            packets_captured,
            packets_dropped,
            bytes_captured,
            runtime_ms,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("interface", &self.interface)?;
        dict.set_item("packets_captured", self.packets_captured)?;
        dict.set_item("packets_dropped", self.packets_dropped)?;
        dict.set_item("bytes_captured", self.bytes_captured)?;
        dict.set_item("runtime_ms", self.runtime_ms)?;
        Ok(dict.into())
    }

    fn to_json(&self, _py: Python) -> PyResult<String> {
        let val = serde_json::json!({
            "interface": self.interface,
            "packets_captured": self.packets_captured,
            "packets_dropped": self.packets_dropped,
            "bytes_captured": self.bytes_captured,
            "runtime_ms": self.runtime_ms,
        });
        serde_json::to_string(&val)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "CaptureStatsEvent(iface={}, captured={}, dropped={})",
            self.interface, self.packets_captured, self.packets_dropped
        )
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

/// Typed event: handshake completed (TCP, TLS, WebSocket).
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct HandshakeCompletedEvent {
    #[pyo3(get)]
    pub protocol: String,
    #[pyo3(get)]
    pub host: String,
    #[pyo3(get)]
    pub port: u16,
    #[pyo3(get)]
    pub duration_ms: f64,
    #[pyo3(get)]
    pub negotiated_version: Option<String>,
    #[pyo3(get)]
    pub cipher_suite: Option<String>,
}

#[pymethods]
impl HandshakeCompletedEvent {
    #[new]
    #[pyo3(signature = (protocol, host, port, duration_ms, negotiated_version=None, cipher_suite=None))]
    pub(crate) fn new(
        protocol: String,
        host: String,
        port: u16,
        duration_ms: f64,
        negotiated_version: Option<String>,
        cipher_suite: Option<String>,
    ) -> Self {
        Self {
            protocol,
            host,
            port,
            duration_ms,
            negotiated_version,
            cipher_suite,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("protocol", &self.protocol)?;
        dict.set_item("host", &self.host)?;
        dict.set_item("port", self.port)?;
        dict.set_item("duration_ms", self.duration_ms)?;
        dict.set_item("negotiated_version", &self.negotiated_version)?;
        dict.set_item("cipher_suite", &self.cipher_suite)?;
        Ok(dict.into())
    }

    fn to_json(&self, _py: Python) -> PyResult<String> {
        let val = serde_json::json!({
            "protocol": self.protocol,
            "host": self.host,
            "port": self.port,
            "duration_ms": self.duration_ms,
            "negotiated_version": self.negotiated_version,
            "cipher_suite": self.cipher_suite,
        });
        serde_json::to_string(&val)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "HandshakeCompletedEvent(protocol={}, host={}, port={})",
            self.protocol, self.host, self.port
        )
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

/// Typed event: HTTP request sent.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct RequestSentEvent {
    #[pyo3(get)]
    pub method: String,
    #[pyo3(get)]
    pub url: String,
    #[pyo3(get)]
    pub headers_count: usize,
    #[pyo3(get)]
    pub body_size: Option<usize>,
    #[pyo3(get)]
    pub request_index: u64,
}

#[pymethods]
impl RequestSentEvent {
    #[new]
    #[pyo3(signature = (method, url, headers_count, body_size=None, request_index=0))]
    pub(crate) fn new(
        method: String,
        url: String,
        headers_count: usize,
        body_size: Option<usize>,
        request_index: u64,
    ) -> Self {
        Self {
            method,
            url,
            headers_count,
            body_size,
            request_index,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("method", &self.method)?;
        dict.set_item("url", &self.url)?;
        dict.set_item("headers_count", self.headers_count)?;
        dict.set_item("body_size", &self.body_size)?;
        dict.set_item("request_index", self.request_index)?;
        Ok(dict.into())
    }

    fn to_json(&self, _py: Python) -> PyResult<String> {
        let val = serde_json::json!({
            "method": self.method,
            "url": self.url,
            "headers_count": self.headers_count,
            "body_size": self.body_size,
            "request_index": self.request_index,
        });
        serde_json::to_string(&val)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!("RequestSentEvent(method={}, url={})", self.method, self.url)
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

/// Typed event: response headers received.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct ResponseHeadersReceivedEvent {
    #[pyo3(get)]
    pub status_code: u16,
    #[pyo3(get)]
    pub reason: String,
    #[pyo3(get)]
    pub headers_count: usize,
    #[pyo3(get)]
    pub content_length: Option<usize>,
    #[pyo3(get)]
    pub content_type: Option<String>,
    #[pyo3(get)]
    pub redirect_url: Option<String>,
    #[pyo3(get)]
    pub request_index: u64,
}

#[pymethods]
impl ResponseHeadersReceivedEvent {
    #[new]
    #[pyo3(signature = (status_code, reason, headers_count, content_length=None, content_type=None, redirect_url=None, request_index=0))]
    pub(crate) fn new(
        status_code: u16,
        reason: String,
        headers_count: usize,
        content_length: Option<usize>,
        content_type: Option<String>,
        redirect_url: Option<String>,
        request_index: u64,
    ) -> Self {
        Self {
            status_code,
            reason,
            headers_count,
            content_length,
            content_type,
            redirect_url,
            request_index,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("status_code", self.status_code)?;
        dict.set_item("reason", &self.reason)?;
        dict.set_item("headers_count", self.headers_count)?;
        dict.set_item("content_length", &self.content_length)?;
        dict.set_item("content_type", &self.content_type)?;
        dict.set_item("redirect_url", &self.redirect_url)?;
        dict.set_item("request_index", self.request_index)?;
        Ok(dict.into())
    }

    fn to_json(&self, _py: Python) -> PyResult<String> {
        let val = serde_json::json!({
            "status_code": self.status_code,
            "reason": self.reason,
            "headers_count": self.headers_count,
            "content_length": self.content_length,
            "content_type": self.content_type,
            "redirect_url": self.redirect_url,
            "request_index": self.request_index,
        });
        serde_json::to_string(&val)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "ResponseHeadersReceivedEvent(status={}, reason={})",
            self.status_code, self.reason
        )
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

/// Typed event: body download progress.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct BodyProgressEvent {
    #[pyo3(get)]
    pub bytes_received: u64,
    #[pyo3(get)]
    pub bytes_expected: Option<u64>,
    #[pyo3(get)]
    pub percentage: Option<f64>,
    #[pyo3(get)]
    pub is_complete: bool,
}

#[pymethods]
impl BodyProgressEvent {
    #[new]
    #[pyo3(signature = (bytes_received, is_complete, bytes_expected=None, percentage=None))]
    pub(crate) fn new(
        bytes_received: u64,
        is_complete: bool,
        bytes_expected: Option<u64>,
        percentage: Option<f64>,
    ) -> Self {
        Self {
            bytes_received,
            bytes_expected,
            percentage,
            is_complete,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("bytes_received", self.bytes_received)?;
        dict.set_item("bytes_expected", &self.bytes_expected)?;
        dict.set_item("percentage", &self.percentage)?;
        dict.set_item("is_complete", self.is_complete)?;
        Ok(dict.into())
    }

    fn to_json(&self, _py: Python) -> PyResult<String> {
        let val = serde_json::json!({
            "bytes_received": self.bytes_received,
            "bytes_expected": self.bytes_expected,
            "percentage": self.percentage,
            "is_complete": self.is_complete,
        });
        serde_json::to_string(&val)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "BodyProgressEvent(received={}, complete={})",
            self.bytes_received, self.is_complete
        )
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

/// Typed event: packet capture started.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct CaptureStartedEvent {
    #[pyo3(get)]
    pub interface: String,
    #[pyo3(get)]
    pub filter: Option<String>,
    #[pyo3(get)]
    pub promiscuous: bool,
    #[pyo3(get)]
    pub snapshot_len: usize,
}

#[pymethods]
impl CaptureStartedEvent {
    #[new]
    #[pyo3(signature = (interface, promiscuous, snapshot_len, filter=None))]
    pub(crate) fn new(
        interface: String,
        promiscuous: bool,
        snapshot_len: usize,
        filter: Option<String>,
    ) -> Self {
        Self {
            interface,
            filter,
            promiscuous,
            snapshot_len,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("interface", &self.interface)?;
        dict.set_item("filter", &self.filter)?;
        dict.set_item("promiscuous", self.promiscuous)?;
        dict.set_item("snapshot_len", self.snapshot_len)?;
        Ok(dict.into())
    }

    fn to_json(&self, _py: Python) -> PyResult<String> {
        let val = serde_json::json!({
            "interface": self.interface,
            "filter": self.filter,
            "promiscuous": self.promiscuous,
            "snapshot_len": self.snapshot_len,
        });
        serde_json::to_string(&val)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "CaptureStartedEvent(iface={}, promiscuous={})",
            self.interface, self.promiscuous
        )
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

/// Typed event: individual packet sampled during capture.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct PacketSampledEvent {
    #[pyo3(get)]
    pub interface: String,
    #[pyo3(get)]
    pub packet_index: u64,
    #[pyo3(get)]
    pub captured_len: u32,
    #[pyo3(get)]
    pub original_len: u32,
    #[pyo3(get)]
    pub protocol_hint: Option<String>,
}

#[pymethods]
impl PacketSampledEvent {
    #[new]
    #[pyo3(signature = (interface, packet_index, captured_len, original_len, protocol_hint=None))]
    pub(crate) fn new(
        interface: String,
        packet_index: u64,
        captured_len: u32,
        original_len: u32,
        protocol_hint: Option<String>,
    ) -> Self {
        Self {
            interface,
            packet_index,
            captured_len,
            original_len,
            protocol_hint,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("interface", &self.interface)?;
        dict.set_item("packet_index", self.packet_index)?;
        dict.set_item("captured_len", self.captured_len)?;
        dict.set_item("original_len", self.original_len)?;
        dict.set_item("protocol_hint", &self.protocol_hint)?;
        Ok(dict.into())
    }

    fn to_json(&self, _py: Python) -> PyResult<String> {
        let val = serde_json::json!({
            "interface": self.interface,
            "packet_index": self.packet_index,
            "captured_len": self.captured_len,
            "original_len": self.original_len,
            "protocol_hint": self.protocol_hint,
        });
        serde_json::to_string(&val)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "PacketSampledEvent(iface={}, index={}, captured={})",
            self.interface, self.packet_index, self.captured_len
        )
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

/// Typed event: network flow observed.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct FlowObservedEvent {
    #[pyo3(get)]
    pub src_address: String,
    #[pyo3(get)]
    pub src_port: u16,
    #[pyo3(get)]
    pub dst_address: String,
    #[pyo3(get)]
    pub dst_port: u16,
    #[pyo3(get)]
    pub protocol: String,
    #[pyo3(get)]
    pub packet_count: u64,
    #[pyo3(get)]
    pub byte_count: u64,
}

#[pymethods]
impl FlowObservedEvent {
    #[new]
    #[pyo3(signature = (src_address, src_port, dst_address, dst_port, protocol, packet_count, byte_count))]
    pub(crate) fn new(
        src_address: String,
        src_port: u16,
        dst_address: String,
        dst_port: u16,
        protocol: String,
        packet_count: u64,
        byte_count: u64,
    ) -> Self {
        Self {
            src_address,
            src_port,
            dst_address,
            dst_port,
            protocol,
            packet_count,
            byte_count,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("src_address", &self.src_address)?;
        dict.set_item("src_port", self.src_port)?;
        dict.set_item("dst_address", &self.dst_address)?;
        dict.set_item("dst_port", self.dst_port)?;
        dict.set_item("protocol", &self.protocol)?;
        dict.set_item("packet_count", self.packet_count)?;
        dict.set_item("byte_count", self.byte_count)?;
        Ok(dict.into())
    }

    fn to_json(&self, _py: Python) -> PyResult<String> {
        let val = serde_json::json!({
            "src_address": self.src_address,
            "src_port": self.src_port,
            "dst_address": self.dst_address,
            "dst_port": self.dst_port,
            "protocol": self.protocol,
            "packet_count": self.packet_count,
            "byte_count": self.byte_count,
        });
        serde_json::to_string(&val)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "FlowObservedEvent(src={}:{}, dst={}:{}, proto={})",
            self.src_address, self.src_port, self.dst_address, self.dst_port, self.protocol
        )
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

/// Typed event: artifact created during execution.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct ArtifactCreatedEvent {
    #[pyo3(get)]
    pub artifact_type: String,
    #[pyo3(get)]
    pub path: Option<String>,
    #[pyo3(get)]
    pub size: u64,
    #[pyo3(get)]
    pub mime_type: Option<String>,
    #[pyo3(get)]
    pub description: String,
}

#[pymethods]
impl ArtifactCreatedEvent {
    #[new]
    #[pyo3(signature = (artifact_type, description, size, path=None, mime_type=None))]
    pub(crate) fn new(
        artifact_type: String,
        description: String,
        size: u64,
        path: Option<String>,
        mime_type: Option<String>,
    ) -> Self {
        Self {
            artifact_type,
            path,
            size,
            mime_type,
            description,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("artifact_type", &self.artifact_type)?;
        dict.set_item("path", &self.path)?;
        dict.set_item("size", self.size)?;
        dict.set_item("mime_type", &self.mime_type)?;
        dict.set_item("description", &self.description)?;
        Ok(dict.into())
    }

    fn to_json(&self, _py: Python) -> PyResult<String> {
        let val = serde_json::json!({
            "artifact_type": self.artifact_type,
            "path": self.path,
            "size": self.size,
            "mime_type": self.mime_type,
            "description": self.description,
        });
        serde_json::to_string(&val)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "ArtifactCreatedEvent(type={}, size={})",
            self.artifact_type, self.size
        )
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}
