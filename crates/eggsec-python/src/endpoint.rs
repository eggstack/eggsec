use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use serde::{Deserialize, Serialize};

/// Configuration for an endpoint scan.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct EndpointScanConfig {
    #[pyo3(get)]
    pub base_url: String,
    endpoints: Vec<String>,
    #[pyo3(get)]
    pub concurrency: usize,
    #[pyo3(get)]
    pub timeout_ms: u64,
    #[pyo3(get)]
    pub include_404: bool,
    #[pyo3(get)]
    pub verify_tls: bool,
}

#[pymethods]
impl EndpointScanConfig {
    /// Create a new endpoint scan configuration.
    ///
    /// Args:
    ///     base_url: Base URL to scan (e.g. "https://example.com").
    ///     endpoints: List of paths to probe (e.g. ["admin", "login"]).
    ///     concurrency: Max concurrent requests (default: 20).
    ///     timeout_ms: Request timeout in milliseconds (default: 30000).
    ///     include_404: Include 404 responses in results (default: False).
    ///     verify_tls: Verify TLS certificates (default: True).
    #[new]
    #[pyo3(signature = (base_url, endpoints, *, concurrency=20, timeout_ms=30000, include_404=false, verify_tls=true))]
    fn new(
        base_url: String,
        endpoints: Vec<String>,
        concurrency: usize,
        timeout_ms: u64,
        include_404: bool,
        verify_tls: bool,
    ) -> PyResult<Self> {
        if base_url.is_empty() {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "base_url must not be empty",
            ));
        }
        if endpoints.is_empty() {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "endpoints list must not be empty",
            ));
        }
        Ok(Self {
            base_url,
            endpoints,
            concurrency,
            timeout_ms,
            include_404,
            verify_tls,
        })
    }

    /// Returns the list of endpoints.
    #[getter]
    fn endpoints(&self) -> Vec<String> {
        self.endpoints.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "EndpointScanConfig(base_url={}, endpoints={})",
            self.base_url,
            self.endpoints.len()
        )
    }
}

/// A single endpoint result from a scan.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointFinding {
    #[pyo3(get)]
    pub url: String,
    #[pyo3(get)]
    pub path: String,
    #[pyo3(get)]
    pub status_code: u16,
    #[pyo3(get)]
    pub content_length: Option<u64>,
    #[pyo3(get)]
    pub content_type: Option<String>,
    #[pyo3(get)]
    pub redirect_location: Option<String>,
    #[pyo3(get)]
    pub interesting: bool,
    #[pyo3(get)]
    pub response_time_ms: u64,
}

#[pymethods]
impl EndpointFinding {
    /// Convert to a Python dictionary.
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("url", &self.url)?;
        dict.set_item("path", &self.path)?;
        dict.set_item("status_code", self.status_code)?;
        dict.set_item("content_length", &self.content_length)?;
        dict.set_item("content_type", &self.content_type)?;
        dict.set_item("redirect_location", &self.redirect_location)?;
        dict.set_item("interesting", self.interesting)?;
        dict.set_item("response_time_ms", self.response_time_ms)?;
        Ok(dict.into())
    }

    /// Convert to a JSON string.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "EndpointFinding(path={}, status={})",
            self.path, self.status_code
        )
    }

    fn __str__(&self) -> String {
        format!("{} {} - {}", self.status_code, self.path, self.url)
    }
}

/// Statistics for an endpoint scan.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointScanStats {
    #[pyo3(get)]
    pub endpoints_scanned: usize,
    #[pyo3(get)]
    pub endpoints_found: usize,
    #[pyo3(get)]
    pub interesting_findings: usize,
    #[pyo3(get)]
    pub elapsed_ms: u64,
}

#[pymethods]
impl EndpointScanStats {
    fn __repr__(&self) -> String {
        format!(
            "EndpointScanStats(scanned={}, found={}, interesting={}, elapsed_ms={})",
            self.endpoints_scanned,
            self.endpoints_found,
            self.interesting_findings,
            self.elapsed_ms
        )
    }
}

/// Result of an endpoint scan operation.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointScanResult {
    #[pyo3(get)]
    pub base_url: String,
    findings: Vec<EndpointFinding>,
    #[pyo3(get)]
    pub endpoints_found: usize,
    #[pyo3(get)]
    pub elapsed_ms: u64,
    #[pyo3(get)]
    pub stats: EndpointScanStats,
}

#[pymethods]
impl EndpointScanResult {
    /// Returns the list of endpoint findings.
    #[getter]
    fn findings(&self) -> Vec<EndpointFinding> {
        self.findings.clone()
    }

    /// Convert result to a Python dictionary.
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("base_url", &self.base_url)?;
        dict.set_item("endpoints_found", self.endpoints_found)?;
        dict.set_item("elapsed_ms", self.elapsed_ms)?;

        let findings_list = PyList::empty_bound(py);
        for f in &self.findings {
            let f_dict = PyDict::new_bound(py);
            f_dict.set_item("url", &f.url)?;
            f_dict.set_item("path", &f.path)?;
            f_dict.set_item("status_code", f.status_code)?;
            f_dict.set_item("content_length", &f.content_length)?;
            f_dict.set_item("content_type", &f.content_type)?;
            f_dict.set_item("redirect_location", &f.redirect_location)?;
            f_dict.set_item("interesting", f.interesting)?;
            f_dict.set_item("response_time_ms", f.response_time_ms)?;
            findings_list.append(f_dict)?;
        }
        dict.set_item("findings", findings_list)?;

        let stats_dict = PyDict::new_bound(py);
        stats_dict.set_item("endpoints_scanned", self.stats.endpoints_scanned)?;
        stats_dict.set_item("endpoints_found", self.stats.endpoints_found)?;
        stats_dict.set_item("interesting_findings", self.stats.interesting_findings)?;
        stats_dict.set_item("elapsed_ms", self.stats.elapsed_ms)?;
        dict.set_item("stats", stats_dict)?;

        Ok(dict.into())
    }

    /// Convert result to a JSON string.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "EndpointScanResult(base_url={}, found={})",
            self.base_url, self.endpoints_found
        )
    }

    fn __str__(&self) -> String {
        format!(
            "Endpoint scan of {}: {} endpoints found in {}ms",
            self.base_url, self.endpoints_found, self.elapsed_ms
        )
    }

    /// Convert endpoint findings to a list of row dicts suitable for tabular output.
    fn to_rows(&self, py: Python) -> PyResult<PyObject> {
        let list = PyList::empty_bound(py);
        for f in &self.findings {
            let dict = PyDict::new_bound(py);
            dict.set_item("base_url", &self.base_url)?;
            dict.set_item("url", &f.url)?;
            dict.set_item("path", &f.path)?;
            dict.set_item("status_code", f.status_code)?;
            dict.set_item("content_length", &f.content_length)?;
            dict.set_item("content_type", &f.content_type)?;
            dict.set_item("redirect_location", &f.redirect_location)?;
            dict.set_item("interesting", f.interesting)?;
            dict.set_item("response_time_ms", f.response_time_ms)?;
            list.append(dict)?;
        }
        Ok(list.into())
    }
}

impl EndpointScanResult {
    /// Convert from engine EndpointScanResults.
    pub fn from_engine(engine: eggsec::scanner::EndpointScanResults) -> Self {
        let findings: Vec<EndpointFinding> = engine
            .results
            .into_iter()
            .map(|r| EndpointFinding {
                url: format!("{}{}", engine.base_url, r.path),
                path: r.path,
                status_code: r.status_code,
                content_length: r.content_length,
                content_type: None,
                redirect_location: r.redirect,
                interesting: r.interesting,
                response_time_ms: r.response_time_ms,
            })
            .collect();

        let stats = EndpointScanStats {
            endpoints_scanned: engine.endpoints_scanned,
            endpoints_found: engine.endpoints_found,
            interesting_findings: engine.interesting_findings,
            elapsed_ms: engine.duration_ms,
        };

        Self {
            base_url: engine.base_url,
            endpoints_found: engine.endpoints_found,
            elapsed_ms: engine.duration_ms,
            findings,
            stats,
        }
    }
}
