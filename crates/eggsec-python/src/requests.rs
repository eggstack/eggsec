use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::collections::HashMap;

/// Base operation request type.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct OperationRequest {
    #[pyo3(get)]
    pub operation: String,
    #[pyo3(get)]
    pub target: String,
    #[pyo3(get)]
    pub timeout_ms: Option<u64>,
    pub(crate) metadata: HashMap<String, String>,
}

#[pymethods]
impl OperationRequest {
    #[new]
    #[pyo3(signature = (operation, target, *, timeout_ms=None, metadata=None))]
    pub(crate) fn new(
        operation: String,
        target: String,
        timeout_ms: Option<u64>,
        metadata: Option<HashMap<String, String>>,
    ) -> Self {
        Self {
            operation,
            target,
            timeout_ms,
            metadata: metadata.unwrap_or_default(),
        }
    }

    #[getter]
    fn metadata(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        for (k, v) in &self.metadata {
            dict.set_item(k, v)?;
        }
        Ok(dict.into())
    }

    pub(crate) fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("operation", &self.operation)?;
        dict.set_item("target", &self.target)?;
        dict.set_item("timeout_ms", &self.timeout_ms)?;
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
            "OperationRequest(operation={}, target={})",
            self.operation, self.target
        )
    }

    fn __str__(&self) -> String {
        format!("{} -> {}", self.operation, self.target)
    }
}

impl serde::Serialize for OperationRequest {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(4))?;
        map.serialize_entry("operation", &self.operation)?;
        map.serialize_entry("target", &self.target)?;
        map.serialize_entry("timeout_ms", &self.timeout_ms)?;
        map.serialize_entry("metadata", &self.metadata)?;
        map.end()
    }
}

/// Port scan request.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct PortScanRequest {
    #[pyo3(get)]
    pub target: String,
    #[pyo3(get)]
    pub ports: Option<String>,
    #[pyo3(get)]
    pub mode: Option<String>,
    #[pyo3(get)]
    pub timing: Option<String>,
    #[pyo3(get)]
    pub timeout_ms: Option<u64>,
}

#[pymethods]
impl PortScanRequest {
    #[new]
    #[pyo3(signature = (target, *, ports=None, mode=None, timing=None, timeout_ms=None))]
    pub(crate) fn new(
        target: String,
        ports: Option<String>,
        mode: Option<String>,
        timing: Option<String>,
        timeout_ms: Option<u64>,
    ) -> Self {
        Self {
            target,
            ports,
            mode,
            timing,
            timeout_ms,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("target", &self.target)?;
        dict.set_item("ports", &self.ports)?;
        dict.set_item("mode", &self.mode)?;
        dict.set_item("timing", &self.timing)?;
        dict.set_item("timeout_ms", &self.timeout_ms)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!("PortScanRequest(target={})", self.target)
    }
}

impl serde::Serialize for PortScanRequest {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(5))?;
        map.serialize_entry("target", &self.target)?;
        map.serialize_entry("ports", &self.ports)?;
        map.serialize_entry("mode", &self.mode)?;
        map.serialize_entry("timing", &self.timing)?;
        map.serialize_entry("timeout_ms", &self.timeout_ms)?;
        map.end()
    }
}

/// Endpoint scan request.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct EndpointScanRequest {
    #[pyo3(get)]
    pub target: String,
    pub(crate) paths: Option<Vec<String>>,
    methods: Option<Vec<String>>,
    #[pyo3(get)]
    pub timeout_ms: Option<u64>,
}

#[pymethods]
impl EndpointScanRequest {
    #[new]
    #[pyo3(signature = (target, *, paths=None, methods=None, timeout_ms=None))]
    pub(crate) fn new(
        target: String,
        paths: Option<Vec<String>>,
        methods: Option<Vec<String>>,
        timeout_ms: Option<u64>,
    ) -> Self {
        Self {
            target,
            paths,
            methods,
            timeout_ms,
        }
    }

    #[getter]
    fn paths(&self) -> Option<Vec<String>> {
        self.paths.clone()
    }

    #[getter]
    fn methods(&self) -> Option<Vec<String>> {
        self.methods.clone()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("target", &self.target)?;
        dict.set_item("paths", &self.paths)?;
        dict.set_item("methods", &self.methods)?;
        dict.set_item("timeout_ms", &self.timeout_ms)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!("EndpointScanRequest(target={})", self.target)
    }
}

impl serde::Serialize for EndpointScanRequest {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(4))?;
        map.serialize_entry("target", &self.target)?;
        map.serialize_entry("paths", &self.paths)?;
        map.serialize_entry("methods", &self.methods)?;
        map.serialize_entry("timeout_ms", &self.timeout_ms)?;
        map.end()
    }
}

/// Service fingerprint request.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct FingerprintRequest {
    #[pyo3(get)]
    pub target: String,
    pub(crate) ports: Option<Vec<u16>>,
    #[pyo3(get)]
    pub timeout_ms: Option<u64>,
}

#[pymethods]
impl FingerprintRequest {
    #[new]
    #[pyo3(signature = (target, *, ports=None, timeout_ms=None))]
    pub(crate) fn new(target: String, ports: Option<Vec<u16>>, timeout_ms: Option<u64>) -> Self {
        Self {
            target,
            ports,
            timeout_ms,
        }
    }

    #[getter]
    fn ports(&self) -> Option<Vec<u16>> {
        self.ports.clone()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("target", &self.target)?;
        dict.set_item("ports", &self.ports)?;
        dict.set_item("timeout_ms", &self.timeout_ms)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!("FingerprintRequest(target={})", self.target)
    }
}

impl serde::Serialize for FingerprintRequest {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(3))?;
        map.serialize_entry("target", &self.target)?;
        map.serialize_entry("ports", &self.ports)?;
        map.serialize_entry("timeout_ms", &self.timeout_ms)?;
        map.end()
    }
}

/// DNS recon request.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct ReconDnsRequest {
    #[pyo3(get)]
    pub target: String,
    record_types: Option<Vec<String>>,
    #[pyo3(get)]
    pub timeout_ms: Option<u64>,
}

#[pymethods]
impl ReconDnsRequest {
    #[new]
    #[pyo3(signature = (target, *, record_types=None, timeout_ms=None))]
    pub(crate) fn new(
        target: String,
        record_types: Option<Vec<String>>,
        timeout_ms: Option<u64>,
    ) -> Self {
        Self {
            target,
            record_types,
            timeout_ms,
        }
    }

    #[getter]
    fn record_types(&self) -> Option<Vec<String>> {
        self.record_types.clone()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("target", &self.target)?;
        dict.set_item("record_types", &self.record_types)?;
        dict.set_item("timeout_ms", &self.timeout_ms)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!("ReconDnsRequest(target={})", self.target)
    }
}

impl serde::Serialize for ReconDnsRequest {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(3))?;
        map.serialize_entry("target", &self.target)?;
        map.serialize_entry("record_types", &self.record_types)?;
        map.serialize_entry("timeout_ms", &self.timeout_ms)?;
        map.end()
    }
}

/// TLS inspection request.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct TlsInspectRequest {
    #[pyo3(get)]
    pub target: String,
    #[pyo3(get)]
    pub timeout_ms: Option<u64>,
}

#[pymethods]
impl TlsInspectRequest {
    #[new]
    #[pyo3(signature = (target, *, timeout_ms=None))]
    pub(crate) fn new(target: String, timeout_ms: Option<u64>) -> Self {
        Self { target, timeout_ms }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("target", &self.target)?;
        dict.set_item("timeout_ms", &self.timeout_ms)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!("TlsInspectRequest(target={})", self.target)
    }
}

impl serde::Serialize for TlsInspectRequest {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(2))?;
        map.serialize_entry("target", &self.target)?;
        map.serialize_entry("timeout_ms", &self.timeout_ms)?;
        map.end()
    }
}

/// Technology detection request.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct TechDetectRequest {
    #[pyo3(get)]
    pub target: String,
    #[pyo3(get)]
    pub timeout_ms: Option<u64>,
}

#[pymethods]
impl TechDetectRequest {
    #[new]
    #[pyo3(signature = (target, *, timeout_ms=None))]
    pub(crate) fn new(target: String, timeout_ms: Option<u64>) -> Self {
        Self { target, timeout_ms }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("target", &self.target)?;
        dict.set_item("timeout_ms", &self.timeout_ms)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!("TechDetectRequest(target={})", self.target)
    }
}

impl serde::Serialize for TechDetectRequest {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(2))?;
        map.serialize_entry("target", &self.target)?;
        map.serialize_entry("timeout_ms", &self.timeout_ms)?;
        map.end()
    }
}

/// WAF detection request.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct WafDetectRequest {
    #[pyo3(get)]
    pub target: String,
    #[pyo3(get)]
    pub timeout_ms: Option<u64>,
}

#[pymethods]
impl WafDetectRequest {
    #[new]
    #[pyo3(signature = (target, *, timeout_ms=None))]
    pub(crate) fn new(target: String, timeout_ms: Option<u64>) -> Self {
        Self { target, timeout_ms }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("target", &self.target)?;
        dict.set_item("timeout_ms", &self.timeout_ms)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!("WafDetectRequest(target={})", self.target)
    }
}

impl serde::Serialize for WafDetectRequest {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(2))?;
        map.serialize_entry("target", &self.target)?;
        map.serialize_entry("timeout_ms", &self.timeout_ms)?;
        map.end()
    }
}

/// Load test request.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct LoadTestRequest {
    #[pyo3(get)]
    pub target: String,
    #[pyo3(get)]
    pub requests: Option<u32>,
    #[pyo3(get)]
    pub concurrency: Option<u32>,
    #[pyo3(get)]
    pub method: Option<String>,
    #[pyo3(get)]
    pub timeout_ms: Option<u64>,
}

#[pymethods]
impl LoadTestRequest {
    #[new]
    #[pyo3(signature = (target, *, requests=None, concurrency=None, method=None, timeout_ms=None))]
    pub(crate) fn new(
        target: String,
        requests: Option<u32>,
        concurrency: Option<u32>,
        method: Option<String>,
        timeout_ms: Option<u64>,
    ) -> Self {
        Self {
            target,
            requests,
            concurrency,
            method,
            timeout_ms,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("target", &self.target)?;
        dict.set_item("requests", &self.requests)?;
        dict.set_item("concurrency", &self.concurrency)?;
        dict.set_item("method", &self.method)?;
        dict.set_item("timeout_ms", &self.timeout_ms)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!("LoadTestRequest(target={})", self.target)
    }
}

impl serde::Serialize for LoadTestRequest {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(5))?;
        map.serialize_entry("target", &self.target)?;
        map.serialize_entry("requests", &self.requests)?;
        map.serialize_entry("concurrency", &self.concurrency)?;
        map.serialize_entry("method", &self.method)?;
        map.serialize_entry("timeout_ms", &self.timeout_ms)?;
        map.end()
    }
}

/// WAF validation request.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct WafValidateRequest {
    #[pyo3(get)]
    pub target: String,
    payloads: Option<Vec<String>>,
    #[pyo3(get)]
    pub timeout_ms: Option<u64>,
}

#[pymethods]
impl WafValidateRequest {
    #[new]
    #[pyo3(signature = (target, *, payloads=None, timeout_ms=None))]
    pub(crate) fn new(
        target: String,
        payloads: Option<Vec<String>>,
        timeout_ms: Option<u64>,
    ) -> Self {
        Self {
            target,
            payloads,
            timeout_ms,
        }
    }

    #[getter]
    fn payloads(&self) -> Option<Vec<String>> {
        self.payloads.clone()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("target", &self.target)?;
        dict.set_item("payloads", &self.payloads)?;
        dict.set_item("timeout_ms", &self.timeout_ms)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!("WafValidateRequest(target={})", self.target)
    }
}

impl serde::Serialize for WafValidateRequest {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(3))?;
        map.serialize_entry("target", &self.target)?;
        map.serialize_entry("payloads", &self.payloads)?;
        map.serialize_entry("timeout_ms", &self.timeout_ms)?;
        map.end()
    }
}

/// HTTP fuzz request.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct FuzzRequest {
    #[pyo3(get)]
    pub target: String,
    #[pyo3(get)]
    pub payload_type: Option<String>,
    #[pyo3(get)]
    pub threads: Option<u32>,
    #[pyo3(get)]
    pub timeout_ms: Option<u64>,
}

#[pymethods]
impl FuzzRequest {
    #[new]
    #[pyo3(signature = (target, *, payload_type=None, threads=None, timeout_ms=None))]
    pub(crate) fn new(
        target: String,
        payload_type: Option<String>,
        threads: Option<u32>,
        timeout_ms: Option<u64>,
    ) -> Self {
        Self {
            target,
            payload_type,
            threads,
            timeout_ms,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("target", &self.target)?;
        dict.set_item("payload_type", &self.payload_type)?;
        dict.set_item("threads", &self.threads)?;
        dict.set_item("timeout_ms", &self.timeout_ms)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!("FuzzRequest(target={})", self.target)
    }
}

impl serde::Serialize for FuzzRequest {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(4))?;
        map.serialize_entry("target", &self.target)?;
        map.serialize_entry("payload_type", &self.payload_type)?;
        map.serialize_entry("threads", &self.threads)?;
        map.serialize_entry("timeout_ms", &self.timeout_ms)?;
        map.end()
    }
}

/// Fluent builder for constructing operation requests.
#[pyclass]
#[derive(Debug, Clone)]
pub struct RequestBuilder {
    operation: String,
    target: String,
    ports: Option<String>,
    mode: Option<String>,
    timing: Option<String>,
    timeout_ms: Option<u64>,
    metadata: HashMap<String, String>,
}

#[pymethods]
impl RequestBuilder {
    #[new]
    fn new(operation: String, target: String) -> Self {
        Self {
            operation,
            target,
            ports: None,
            mode: None,
            timing: None,
            timeout_ms: None,
            metadata: HashMap::new(),
        }
    }

    fn port(mut pyself: PyRefMut<'_, Self>, ports: String) -> PyRefMut<'_, Self> {
        pyself.ports = Some(ports);
        pyself
    }

    fn timing(mut pyself: PyRefMut<'_, Self>, t: String) -> PyRefMut<'_, Self> {
        pyself.timing = Some(t);
        pyself
    }

    fn timeout(mut pyself: PyRefMut<'_, Self>, ms: u64) -> PyRefMut<'_, Self> {
        pyself.timeout_ms = Some(ms);
        pyself
    }

    fn metadata_key(mut pyself: PyRefMut<'_, Self>, k: String, v: String) -> PyRefMut<'_, Self> {
        pyself.metadata.insert(k, v);
        pyself
    }

    fn mode(mut pyself: PyRefMut<'_, Self>, m: String) -> PyRefMut<'_, Self> {
        pyself.mode = Some(m);
        pyself
    }

    fn build(&self) -> OperationRequest {
        let mut meta = self.metadata.clone();
        // Include typed fields as metadata for the generic OperationRequest
        if let Some(ref p) = self.ports {
            meta.insert("ports".to_string(), p.clone());
        }
        if let Some(ref m) = self.mode {
            meta.insert("mode".to_string(), m.clone());
        }
        if let Some(ref t) = self.timing {
            meta.insert("timing".to_string(), t.clone());
        }
        OperationRequest {
            operation: self.operation.clone(),
            target: self.target.clone(),
            timeout_ms: self.timeout_ms,
            metadata: meta,
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "RequestBuilder(operation={}, target={})",
            self.operation, self.target
        )
    }
}
