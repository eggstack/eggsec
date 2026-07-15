use pyo3::prelude::*;
use pyo3::types::PyDict;
use serde::{Deserialize, Serialize};

// ═══════════════════════════════════════════════════════════════════
// WS10: Browser event and network correlation — missing types
// ═══════════════════════════════════════════════════════════════════

/// A DOM mutation event captured during browser interaction.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserDomEvent {
    #[pyo3(get)]
    pub event_id: String,
    #[pyo3(get)]
    pub session_id: String,
    #[pyo3(get)]
    pub event_type: String,
    #[pyo3(get)]
    pub sequence: u64,
    #[pyo3(get)]
    pub timestamp_ms: u64,
    #[pyo3(get)]
    pub url: String,
    #[pyo3(get)]
    pub selector: Option<String>,
    #[pyo3(get)]
    pub attribute_name: Option<String>,
    #[pyo3(get)]
    pub old_value: Option<String>,
    #[pyo3(get)]
    pub new_value: Option<String>,
    #[pyo3(get)]
    pub element_tag: Option<String>,
}

#[pymethods]
impl BrowserDomEvent {
    #[new]
    #[pyo3(signature = (event_id, session_id, event_type, sequence, timestamp_ms, url, *, selector=None, attribute_name=None, old_value=None, new_value=None, element_tag=None))]
    fn new(
        event_id: String,
        session_id: String,
        event_type: String,
        sequence: u64,
        timestamp_ms: u64,
        url: String,
        selector: Option<String>,
        attribute_name: Option<String>,
        old_value: Option<String>,
        new_value: Option<String>,
        element_tag: Option<String>,
    ) -> Self {
        Self {
            event_id,
            session_id,
            event_type,
            sequence,
            timestamp_ms,
            url,
            selector,
            attribute_name,
            old_value,
            new_value,
            element_tag,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("event_id", &self.event_id)?;
        dict.set_item("session_id", &self.session_id)?;
        dict.set_item("event_type", &self.event_type)?;
        dict.set_item("sequence", self.sequence)?;
        dict.set_item("timestamp_ms", self.timestamp_ms)?;
        dict.set_item("url", &self.url)?;
        dict.set_item("selector", &self.selector)?;
        dict.set_item("attribute_name", &self.attribute_name)?;
        dict.set_item("old_value", &self.old_value)?;
        dict.set_item("new_value", &self.new_value)?;
        dict.set_item("element_tag", &self.element_tag)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "BrowserDomEvent(id={}, type={}, seq={})",
            self.event_id, self.event_type, self.sequence
        )
    }
}

/// A file download event captured during browser interaction.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserDownloadEvent {
    #[pyo3(get)]
    pub event_id: String,
    #[pyo3(get)]
    pub session_id: String,
    #[pyo3(get)]
    pub sequence: u64,
    #[pyo3(get)]
    pub timestamp_ms: u64,
    #[pyo3(get)]
    pub url: String,
    #[pyo3(get)]
    pub suggested_filename: Option<String>,
    #[pyo3(get)]
    pub content_type: Option<String>,
    #[pyo3(get)]
    pub size_bytes: Option<u64>,
    #[pyo3(get)]
    pub download_path: Option<String>,
    #[pyo3(get)]
    pub status: String,
}

#[pymethods]
impl BrowserDownloadEvent {
    #[new]
    #[pyo3(signature = (event_id, session_id, sequence, timestamp_ms, url, *, suggested_filename=None, content_type=None, size_bytes=None, download_path=None, status=None))]
    fn new(
        event_id: String,
        session_id: String,
        sequence: u64,
        timestamp_ms: u64,
        url: String,
        suggested_filename: Option<String>,
        content_type: Option<String>,
        size_bytes: Option<u64>,
        download_path: Option<String>,
        status: Option<String>,
    ) -> Self {
        Self {
            event_id,
            session_id,
            sequence,
            timestamp_ms,
            url,
            suggested_filename,
            content_type,
            size_bytes,
            download_path,
            status: status.unwrap_or_else(|| "completed".to_string()),
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("event_id", &self.event_id)?;
        dict.set_item("session_id", &self.session_id)?;
        dict.set_item("sequence", self.sequence)?;
        dict.set_item("timestamp_ms", self.timestamp_ms)?;
        dict.set_item("url", &self.url)?;
        dict.set_item("suggested_filename", &self.suggested_filename)?;
        dict.set_item("content_type", &self.content_type)?;
        dict.set_item("size_bytes", self.size_bytes)?;
        dict.set_item("download_path", &self.download_path)?;
        dict.set_item("status", &self.status)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "BrowserDownloadEvent(id={}, url={}, status={})",
            self.event_id, self.url, self.status
        )
    }
}

/// A security observation captured during browser interaction.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserSecurityObservation {
    #[pyo3(get)]
    pub observation_id: String,
    #[pyo3(get)]
    pub session_id: String,
    #[pyo3(get)]
    pub sequence: u64,
    #[pyo3(get)]
    pub timestamp_ms: u64,
    #[pyo3(get)]
    pub observation_type: String,
    #[pyo3(get)]
    pub url: String,
    #[pyo3(get)]
    pub header_name: Option<String>,
    #[pyo3(get)]
    pub header_value: Option<String>,
    #[pyo3(get)]
    pub severity: String,
    #[pyo3(get)]
    pub description: String,
    #[pyo3(get)]
    pub tls_version: Option<String>,
    #[pyo3(get)]
    pub certificate_info: Option<String>,
}

#[pymethods]
impl BrowserSecurityObservation {
    #[new]
    #[pyo3(signature = (observation_id, session_id, sequence, timestamp_ms, observation_type, url, severity, description, *, header_name=None, header_value=None, tls_version=None, certificate_info=None))]
    fn new(
        observation_id: String,
        session_id: String,
        sequence: u64,
        timestamp_ms: u64,
        observation_type: String,
        url: String,
        severity: String,
        description: String,
        header_name: Option<String>,
        header_value: Option<String>,
        tls_version: Option<String>,
        certificate_info: Option<String>,
    ) -> Self {
        Self {
            observation_id,
            session_id,
            sequence,
            timestamp_ms,
            observation_type,
            url,
            header_name,
            header_value,
            severity,
            description,
            tls_version,
            certificate_info,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("observation_id", &self.observation_id)?;
        dict.set_item("session_id", &self.session_id)?;
        dict.set_item("sequence", self.sequence)?;
        dict.set_item("timestamp_ms", self.timestamp_ms)?;
        dict.set_item("observation_type", &self.observation_type)?;
        dict.set_item("url", &self.url)?;
        dict.set_item("header_name", &self.header_name)?;
        dict.set_item("header_value", &self.header_value)?;
        dict.set_item("severity", &self.severity)?;
        dict.set_item("description", &self.description)?;
        dict.set_item("tls_version", &self.tls_version)?;
        dict.set_item("certificate_info", &self.certificate_info)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "BrowserSecurityObservation(id={}, type={}, severity={})",
            self.observation_id, self.observation_type, self.severity
        )
    }
}
