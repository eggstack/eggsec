use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use std::collections::HashMap;

/// Execution status enum for operation results.
#[pyclass(frozen)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ExecutionStatus {
    Pending(),
    Running(),
    Completed(),
    Failed { error: String },
    Cancelled { reason: Option<String> },
    Timeout { elapsed_ms: u64 },
}

#[pymethods]
impl ExecutionStatus {
    pub(crate) fn name(&self) -> &'static str {
        match self {
            ExecutionStatus::Pending() => "Pending",
            ExecutionStatus::Running() => "Running",
            ExecutionStatus::Completed() => "Completed",
            ExecutionStatus::Failed { .. } => "Failed",
            ExecutionStatus::Cancelled { .. } => "Cancelled",
            ExecutionStatus::Timeout { .. } => "Timeout",
        }
    }

    fn __repr__(&self) -> String {
        match self {
            ExecutionStatus::Pending() => "ExecutionStatus.Pending".to_string(),
            ExecutionStatus::Running() => "ExecutionStatus.Running".to_string(),
            ExecutionStatus::Completed() => "ExecutionStatus.Completed".to_string(),
            ExecutionStatus::Failed { error } => {
                format!("ExecutionStatus.Failed(error={})", error)
            }
            ExecutionStatus::Cancelled { reason } => match reason {
                Some(r) => format!("ExecutionStatus.Cancelled(reason={})", r),
                None => "ExecutionStatus.Cancelled".to_string(),
            },
            ExecutionStatus::Timeout { elapsed_ms } => {
                format!("ExecutionStatus.Timeout(elapsed_ms={})", elapsed_ms)
            }
        }
    }
}

/// Execution statistics for a completed operation.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct ExecutionStats {
    #[pyo3(get)]
    pub duration_ms: u64,
    #[pyo3(get)]
    pub items_processed: u64,
    #[pyo3(get)]
    pub items_failed: u64,
    #[pyo3(get)]
    pub bytes_transferred: u64,
}

#[pymethods]
impl ExecutionStats {
    #[new]
    #[pyo3(signature = (duration_ms=0, items_processed=0, items_failed=0, bytes_transferred=0))]
    pub(crate) fn new(
        duration_ms: u64,
        items_processed: u64,
        items_failed: u64,
        bytes_transferred: u64,
    ) -> Self {
        Self {
            duration_ms,
            items_processed,
            items_failed,
            bytes_transferred,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("duration_ms", self.duration_ms)?;
        dict.set_item("items_processed", self.items_processed)?;
        dict.set_item("items_failed", self.items_failed)?;
        dict.set_item("bytes_transferred", self.bytes_transferred)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "ExecutionStats(duration_ms={}, processed={}, failed={}, bytes={})",
            self.duration_ms, self.items_processed, self.items_failed, self.bytes_transferred
        )
    }

    fn __str__(&self) -> String {
        format!(
            "{}ms, {}/{} items, {} bytes",
            self.duration_ms, self.items_processed, self.items_failed, self.bytes_transferred
        )
    }
}

/// An artifact produced by an operation.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct Artifact {
    #[pyo3(get)]
    pub name: String,
    #[pyo3(get)]
    pub kind: String,
    #[pyo3(get)]
    pub mime_type: Option<String>,
    #[pyo3(get)]
    pub data: Option<String>,
    #[pyo3(get)]
    pub path: Option<String>,
}

#[pymethods]
impl Artifact {
    #[new]
    #[pyo3(signature = (name, kind, *, mime_type=None, data=None, path=None))]
    fn new(
        name: String,
        kind: String,
        mime_type: Option<String>,
        data: Option<String>,
        path: Option<String>,
    ) -> Self {
        Self {
            name,
            kind,
            mime_type,
            data,
            path,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("name", &self.name)?;
        dict.set_item("kind", &self.kind)?;
        dict.set_item("mime_type", &self.mime_type)?;
        dict.set_item("data", &self.data)?;
        dict.set_item("path", &self.path)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!("Artifact(name={}, kind={})", self.name, self.kind)
    }

    fn __str__(&self) -> String {
        match &self.path {
            Some(p) => format!("{} ({})", self.name, p),
            None => self.name.clone(),
        }
    }
}

/// Result of an operation.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct OperationResult {
    pub(crate) status: ExecutionStatus,
    stats: Option<ExecutionStats>,
    artifacts: Vec<Artifact>,
    error: Option<String>,
    metadata: HashMap<String, String>,
}

#[pymethods]
impl OperationResult {
    #[new]
    #[pyo3(signature = (status, *, stats=None, artifacts=None, error=None, metadata=None))]
    pub(crate) fn new(
        status: ExecutionStatus,
        stats: Option<ExecutionStats>,
        artifacts: Option<Vec<Artifact>>,
        error: Option<String>,
        metadata: Option<HashMap<String, String>>,
    ) -> Self {
        Self {
            status,
            stats,
            artifacts: artifacts.unwrap_or_default(),
            error,
            metadata: metadata.unwrap_or_default(),
        }
    }

    #[getter]
    fn status(&self) -> ExecutionStatus {
        self.status.clone()
    }

    #[getter]
    fn stats(&self) -> Option<ExecutionStats> {
        self.stats.clone()
    }

    #[getter]
    fn artifacts(&self) -> Vec<Artifact> {
        self.artifacts.clone()
    }

    #[getter]
    fn error(&self) -> Option<String> {
        self.error.clone()
    }

    #[getter]
    fn metadata(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        for (k, v) in &self.metadata {
            dict.set_item(k, v)?;
        }
        Ok(dict.into())
    }

    fn is_success(&self) -> bool {
        matches!(self.status, ExecutionStatus::Completed())
    }

    fn is_failure(&self) -> bool {
        matches!(
            self.status,
            ExecutionStatus::Failed { .. } | ExecutionStatus::Timeout { .. }
        )
    }

    fn artifact_count(&self) -> usize {
        self.artifacts.len()
    }

    pub(crate) fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);

        // Status: represent as a dict with name + payload
        let status_dict = PyDict::new_bound(py);
        status_dict.set_item("type", self.status.name())?;
        match &self.status {
            ExecutionStatus::Failed { error } => {
                status_dict.set_item("error", error)?;
            }
            ExecutionStatus::Cancelled { reason } => {
                status_dict.set_item("reason", reason)?;
            }
            ExecutionStatus::Timeout { elapsed_ms } => {
                status_dict.set_item("elapsed_ms", elapsed_ms)?;
            }
            _ => {}
        }
        dict.set_item("status", status_dict)?;

        // Stats
        match &self.stats {
            Some(s) => dict.set_item("stats", s.to_dict(py)?)?,
            None => dict.set_item("stats", py.None())?,
        }

        // Artifacts
        let artifacts_list = PyList::empty_bound(py);
        for a in &self.artifacts {
            artifacts_list.append(a.to_dict(py)?)?;
        }
        dict.set_item("artifacts", artifacts_list)?;

        dict.set_item("error", &self.error)?;

        let meta_dict = PyDict::new_bound(py);
        for (k, v) in &self.metadata {
            meta_dict.set_item(k, v)?;
        }
        dict.set_item("metadata", meta_dict)?;

        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "OperationResult(status={}, artifacts={}, error={})",
            self.status.name(),
            self.artifacts.len(),
            self.error.is_some()
        )
    }

    fn __str__(&self) -> String {
        let base = format!(
            "{} ({} artifacts)",
            self.status.name(),
            self.artifacts.len()
        );
        match &self.error {
            Some(e) => format!("{}: {}", base, e),
            None => base,
        }
    }
}

impl serde::Serialize for ExecutionStatus {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let (tag, fields): (&str, usize) = match self {
            ExecutionStatus::Pending() => ("Pending", 0),
            ExecutionStatus::Running() => ("Running", 0),
            ExecutionStatus::Completed() => ("Completed", 0),
            ExecutionStatus::Failed { .. } => ("Failed", 1),
            ExecutionStatus::Cancelled { .. } => ("Cancelled", 1),
            ExecutionStatus::Timeout { .. } => ("Timeout", 1),
        };
        let mut s = serializer.serialize_struct(tag, fields)?;
        match self {
            ExecutionStatus::Pending()
            | ExecutionStatus::Running()
            | ExecutionStatus::Completed() => {}
            ExecutionStatus::Failed { error } => {
                s.serialize_field("error", error)?;
            }
            ExecutionStatus::Cancelled { reason } => {
                s.serialize_field("reason", reason)?;
            }
            ExecutionStatus::Timeout { elapsed_ms } => {
                s.serialize_field("elapsed_ms", elapsed_ms)?;
            }
        }
        s.end()
    }
}

impl<'de> serde::Deserialize<'de> for ExecutionStatus {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(serde::Deserialize)]
        struct Raw {
            #[serde(rename = "type")]
            kind: String,
            #[serde(default)]
            error: Option<String>,
            #[serde(default)]
            reason: Option<String>,
            #[serde(default)]
            elapsed_ms: Option<u64>,
        }
        let raw = Raw::deserialize(deserializer)?;
        match raw.kind.as_str() {
            "Pending" => Ok(ExecutionStatus::Pending()),
            "Running" => Ok(ExecutionStatus::Running()),
            "Completed" => Ok(ExecutionStatus::Completed()),
            "Failed" => Ok(ExecutionStatus::Failed {
                error: raw.error.unwrap_or_default(),
            }),
            "Cancelled" => Ok(ExecutionStatus::Cancelled { reason: raw.reason }),
            "Timeout" => Ok(ExecutionStatus::Timeout {
                elapsed_ms: raw.elapsed_ms.unwrap_or(0),
            }),
            other => Err(serde::de::Error::custom(format!(
                "unknown status type: {}",
                other
            ))),
        }
    }
}

impl serde::Serialize for ExecutionStats {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("ExecutionStats", 4)?;
        s.serialize_field("duration_ms", &self.duration_ms)?;
        s.serialize_field("items_processed", &self.items_processed)?;
        s.serialize_field("items_failed", &self.items_failed)?;
        s.serialize_field("bytes_transferred", &self.bytes_transferred)?;
        s.end()
    }
}

impl serde::Serialize for Artifact {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("Artifact", 5)?;
        s.serialize_field("name", &self.name)?;
        s.serialize_field("kind", &self.kind)?;
        s.serialize_field("mime_type", &self.mime_type)?;
        s.serialize_field("data", &self.data)?;
        s.serialize_field("path", &self.path)?;
        s.end()
    }
}

impl serde::Serialize for OperationResult {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("OperationResult", 5)?;
        s.serialize_field("status", &self.status)?;
        s.serialize_field("stats", &self.stats)?;
        s.serialize_field("artifacts", &self.artifacts)?;
        s.serialize_field("error", &self.error)?;
        s.serialize_field("metadata", &self.metadata)?;
        s.end()
    }
}
