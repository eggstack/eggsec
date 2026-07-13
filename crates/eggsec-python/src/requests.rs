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

/// Git secrets scan request.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct GitSecretsScanRequest {
    #[pyo3(get)]
    pub repo_path: String,
    #[pyo3(get)]
    pub max_commits: Option<usize>,
    #[pyo3(get)]
    pub timeout_ms: Option<u64>,
}

#[pymethods]
impl GitSecretsScanRequest {
    #[new]
    #[pyo3(signature = (repo_path, *, max_commits=None, timeout_ms=None))]
    pub(crate) fn new(
        repo_path: String,
        max_commits: Option<usize>,
        timeout_ms: Option<u64>,
    ) -> Self {
        Self {
            repo_path,
            max_commits,
            timeout_ms,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("repo_path", &self.repo_path)?;
        dict.set_item("max_commits", &self.max_commits)?;
        dict.set_item("timeout_ms", &self.timeout_ms)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!("GitSecretsScanRequest(repo_path={})", self.repo_path)
    }
}

impl serde::Serialize for GitSecretsScanRequest {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(3))?;
        map.serialize_entry("repo_path", &self.repo_path)?;
        map.serialize_entry("max_commits", &self.max_commits)?;
        map.serialize_entry("timeout_ms", &self.timeout_ms)?;
        map.end()
    }
}

/// SBOM generation request.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct SbomRequest {
    #[pyo3(get)]
    pub project_path: String,
    #[pyo3(get)]
    pub ecosystem: Option<String>,
    #[pyo3(get)]
    pub format: Option<String>,
    #[pyo3(get)]
    pub timeout_ms: Option<u64>,
}

#[pymethods]
impl SbomRequest {
    #[new]
    #[pyo3(signature = (project_path, *, ecosystem=None, format=None, timeout_ms=None))]
    pub(crate) fn new(
        project_path: String,
        ecosystem: Option<String>,
        format: Option<String>,
        timeout_ms: Option<u64>,
    ) -> Self {
        Self {
            project_path,
            ecosystem,
            format,
            timeout_ms,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("project_path", &self.project_path)?;
        dict.set_item("ecosystem", &self.ecosystem)?;
        dict.set_item("format", &self.format)?;
        dict.set_item("timeout_ms", &self.timeout_ms)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!("SbomRequest(project_path={})", self.project_path)
    }
}

impl serde::Serialize for SbomRequest {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(4))?;
        map.serialize_entry("project_path", &self.project_path)?;
        map.serialize_entry("ecosystem", &self.ecosystem)?;
        map.serialize_entry("format", &self.format)?;
        map.serialize_entry("timeout_ms", &self.timeout_ms)?;
        map.end()
    }
}

/// Consolidated recon request.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct ConsolidatedReconRequest {
    #[pyo3(get)]
    pub target: String,
    #[pyo3(get)]
    pub run_dns: Option<bool>,
    #[pyo3(get)]
    pub run_ssl: Option<bool>,
    #[pyo3(get)]
    pub run_tech_detect: Option<bool>,
    #[pyo3(get)]
    pub run_subdomain: Option<bool>,
    #[pyo3(get)]
    pub run_whois: Option<bool>,
    #[pyo3(get)]
    pub run_cors: Option<bool>,
    #[pyo3(get)]
    pub run_wayback: Option<bool>,
    #[pyo3(get)]
    pub run_js_analysis: Option<bool>,
    #[pyo3(get)]
    pub run_content: Option<bool>,
    #[pyo3(get)]
    pub run_email: Option<bool>,
    #[pyo3(get)]
    pub timeout_ms: Option<u64>,
}

#[pymethods]
impl ConsolidatedReconRequest {
    #[new]
    #[pyo3(signature = (target, *, run_dns=None, run_ssl=None, run_tech_detect=None, run_subdomain=None, run_whois=None, run_cors=None, run_wayback=None, run_js_analysis=None, run_content=None, run_email=None, timeout_ms=None))]
    pub(crate) fn new(
        target: String,
        run_dns: Option<bool>,
        run_ssl: Option<bool>,
        run_tech_detect: Option<bool>,
        run_subdomain: Option<bool>,
        run_whois: Option<bool>,
        run_cors: Option<bool>,
        run_wayback: Option<bool>,
        run_js_analysis: Option<bool>,
        run_content: Option<bool>,
        run_email: Option<bool>,
        timeout_ms: Option<u64>,
    ) -> Self {
        Self {
            target,
            run_dns,
            run_ssl,
            run_tech_detect,
            run_subdomain,
            run_whois,
            run_cors,
            run_wayback,
            run_js_analysis,
            run_content,
            run_email,
            timeout_ms,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("target", &self.target)?;
        dict.set_item("run_dns", &self.run_dns)?;
        dict.set_item("run_ssl", &self.run_ssl)?;
        dict.set_item("run_tech_detect", &self.run_tech_detect)?;
        dict.set_item("run_subdomain", &self.run_subdomain)?;
        dict.set_item("run_whois", &self.run_whois)?;
        dict.set_item("run_cors", &self.run_cors)?;
        dict.set_item("run_wayback", &self.run_wayback)?;
        dict.set_item("run_js_analysis", &self.run_js_analysis)?;
        dict.set_item("run_content", &self.run_content)?;
        dict.set_item("run_email", &self.run_email)?;
        dict.set_item("timeout_ms", &self.timeout_ms)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!("ConsolidatedReconRequest(target={})", self.target)
    }
}

impl serde::Serialize for ConsolidatedReconRequest {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(12))?;
        map.serialize_entry("target", &self.target)?;
        map.serialize_entry("run_dns", &self.run_dns)?;
        map.serialize_entry("run_ssl", &self.run_ssl)?;
        map.serialize_entry("run_tech_detect", &self.run_tech_detect)?;
        map.serialize_entry("run_subdomain", &self.run_subdomain)?;
        map.serialize_entry("run_whois", &self.run_whois)?;
        map.serialize_entry("run_cors", &self.run_cors)?;
        map.serialize_entry("run_wayback", &self.run_wayback)?;
        map.serialize_entry("run_js_analysis", &self.run_js_analysis)?;
        map.serialize_entry("run_content", &self.run_content)?;
        map.serialize_entry("run_email", &self.run_email)?;
        map.serialize_entry("timeout_ms", &self.timeout_ms)?;
        map.end()
    }
}

/// GraphQL test request.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct GraphqlTestRequest {
    #[pyo3(get)]
    pub target: String,
    #[pyo3(get)]
    pub timeout_ms: Option<u64>,
}

#[pymethods]
impl GraphqlTestRequest {
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
        format!("GraphqlTestRequest(target={})", self.target)
    }
}

impl serde::Serialize for GraphqlTestRequest {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(2))?;
        map.serialize_entry("target", &self.target)?;
        map.serialize_entry("timeout_ms", &self.timeout_ms)?;
        map.end()
    }
}

/// OAuth test request.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct OauthTestRequest {
    #[pyo3(get)]
    pub target: String,
    #[pyo3(get)]
    pub timeout_ms: Option<u64>,
}

#[pymethods]
impl OauthTestRequest {
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
        format!("OauthTestRequest(target={})", self.target)
    }
}

impl serde::Serialize for OauthTestRequest {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(2))?;
        map.serialize_entry("target", &self.target)?;
        map.serialize_entry("timeout_ms", &self.timeout_ms)?;
        map.end()
    }
}

/// Auth test request.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct AuthTestRequest {
    #[pyo3(get)]
    pub target: String,
    #[pyo3(get)]
    pub timeout_ms: Option<u64>,
}

#[pymethods]
impl AuthTestRequest {
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
        format!("AuthTestRequest(target={})", self.target)
    }
}

impl serde::Serialize for AuthTestRequest {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(2))?;
        map.serialize_entry("target", &self.target)?;
        map.serialize_entry("timeout_ms", &self.timeout_ms)?;
        map.end()
    }
}

/// Database probe request.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct DbProbeRequest {
    #[pyo3(get)]
    pub target: String,
    #[pyo3(get)]
    pub port: Option<u16>,
    #[pyo3(get)]
    pub database: Option<String>,
    #[pyo3(get)]
    pub username: Option<String>,
    #[pyo3(get)]
    pub password: Option<String>,
    #[pyo3(get)]
    pub timeout_ms: Option<u64>,
}

#[pymethods]
impl DbProbeRequest {
    #[new]
    #[pyo3(signature = (target, *, port=None, database=None, username=None, password=None, timeout_ms=None))]
    pub(crate) fn new(
        target: String,
        port: Option<u16>,
        database: Option<String>,
        username: Option<String>,
        password: Option<String>,
        timeout_ms: Option<u64>,
    ) -> Self {
        Self {
            target,
            port,
            database,
            username,
            password,
            timeout_ms,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("target", &self.target)?;
        dict.set_item("port", &self.port)?;
        dict.set_item("database", &self.database)?;
        dict.set_item("username", &self.username)?;
        dict.set_item("password", &self.password)?;
        dict.set_item("timeout_ms", &self.timeout_ms)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!("DbProbeRequest(target={})", self.target)
    }
}

impl serde::Serialize for DbProbeRequest {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(6))?;
        map.serialize_entry("target", &self.target)?;
        map.serialize_entry("port", &self.port)?;
        map.serialize_entry("database", &self.database)?;
        map.serialize_entry("username", &self.username)?;
        map.serialize_entry("password", &self.password)?;
        map.serialize_entry("timeout_ms", &self.timeout_ms)?;
        map.end()
    }
}

/// NSE run request.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct NseRunRequest {
    #[pyo3(get)]
    pub target: String,
    #[pyo3(get)]
    pub scripts: Option<Vec<String>>,
    #[pyo3(get)]
    pub timeout_ms: Option<u64>,
}

#[pymethods]
impl NseRunRequest {
    #[new]
    #[pyo3(signature = (target, *, scripts=None, timeout_ms=None))]
    pub(crate) fn new(
        target: String,
        scripts: Option<Vec<String>>,
        timeout_ms: Option<u64>,
    ) -> Self {
        Self {
            target,
            scripts,
            timeout_ms,
        }
    }

    #[getter]
    fn scripts(&self) -> Option<Vec<String>> {
        self.scripts.clone()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("target", &self.target)?;
        dict.set_item("scripts", &self.scripts)?;
        dict.set_item("timeout_ms", &self.timeout_ms)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!("NseRunRequest(target={})", self.target)
    }
}

impl serde::Serialize for NseRunRequest {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(3))?;
        map.serialize_entry("target", &self.target)?;
        map.serialize_entry("scripts", &self.scripts)?;
        map.serialize_entry("timeout_ms", &self.timeout_ms)?;
        map.end()
    }
}

/// Docker image scan request.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct DockerImageScanRequest {
    #[pyo3(get)]
    pub image: String,
    #[pyo3(get)]
    pub timeout_ms: Option<u64>,
}

#[pymethods]
impl DockerImageScanRequest {
    #[new]
    #[pyo3(signature = (image, *, timeout_ms=None))]
    pub(crate) fn new(image: String, timeout_ms: Option<u64>) -> Self {
        Self { image, timeout_ms }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("image", &self.image)?;
        dict.set_item("timeout_ms", &self.timeout_ms)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!("DockerImageScanRequest(image={})", self.image)
    }
}

impl serde::Serialize for DockerImageScanRequest {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(2))?;
        map.serialize_entry("image", &self.image)?;
        map.serialize_entry("timeout_ms", &self.timeout_ms)?;
        map.end()
    }
}

/// Kubernetes scan request.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct KubernetesScanRequest {
    #[pyo3(get)]
    pub target: String,
    #[pyo3(get)]
    pub timeout_ms: Option<u64>,
}

#[pymethods]
impl KubernetesScanRequest {
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
        format!("KubernetesScanRequest(target={})", self.target)
    }
}

impl serde::Serialize for KubernetesScanRequest {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(2))?;
        map.serialize_entry("target", &self.target)?;
        map.serialize_entry("timeout_ms", &self.timeout_ms)?;
        map.end()
    }
}

/// APK analysis request.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct ApkAnalysisRequest {
    #[pyo3(get)]
    pub apk_path: String,
    #[pyo3(get)]
    pub timeout_ms: Option<u64>,
}

#[pymethods]
impl ApkAnalysisRequest {
    #[new]
    #[pyo3(signature = (apk_path, *, timeout_ms=None))]
    pub(crate) fn new(apk_path: String, timeout_ms: Option<u64>) -> Self {
        Self {
            apk_path,
            timeout_ms,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("apk_path", &self.apk_path)?;
        dict.set_item("timeout_ms", &self.timeout_ms)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!("ApkAnalysisRequest(apk_path={})", self.apk_path)
    }
}

impl serde::Serialize for ApkAnalysisRequest {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(2))?;
        map.serialize_entry("apk_path", &self.apk_path)?;
        map.serialize_entry("timeout_ms", &self.timeout_ms)?;
        map.end()
    }
}

/// IPA analysis request.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct IpaAnalysisRequest {
    #[pyo3(get)]
    pub ipa_path: String,
    #[pyo3(get)]
    pub timeout_ms: Option<u64>,
}

#[pymethods]
impl IpaAnalysisRequest {
    #[new]
    #[pyo3(signature = (ipa_path, *, timeout_ms=None))]
    pub(crate) fn new(ipa_path: String, timeout_ms: Option<u64>) -> Self {
        Self {
            ipa_path,
            timeout_ms,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("ipa_path", &self.ipa_path)?;
        dict.set_item("timeout_ms", &self.timeout_ms)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!("IpaAnalysisRequest(ipa_path={})", self.ipa_path)
    }
}

impl serde::Serialize for IpaAnalysisRequest {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(2))?;
        map.serialize_entry("ipa_path", &self.ipa_path)?;
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
