use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use serde::{Deserialize, Serialize};

use crate::error::EggsecResultExt;
use crate::finding::Severity;
use crate::runtime_sync;

/// WebSocket connection test result.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionTestResultPy {
    #[pyo3(get)]
    pub url: String,
    #[pyo3(get)]
    pub connected: bool,
    response_headers: Vec<(String, String)>,
    #[pyo3(get)]
    pub subprotocols: Vec<String>,
    #[pyo3(get)]
    pub extensions: Vec<String>,
    #[pyo3(get)]
    pub latency_ms: Option<f64>,
    #[pyo3(get)]
    pub error: Option<String>,
}

#[pymethods]
impl ConnectionTestResultPy {
    #[getter]
    fn response_headers(&self, py: Python) -> PyResult<PyObject> {
        let list = PyList::empty_bound(py);
        for (k, v) in &self.response_headers {
            let tuple = pyo3::types::PyTuple::new_bound(py, &[k.as_str(), v.as_str()]);
            list.append(tuple)?;
        }
        Ok(list.into())
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("url", &self.url)?;
        dict.set_item("connected", self.connected)?;
        dict.set_item("subprotocols", &self.subprotocols)?;
        dict.set_item("extensions", &self.extensions)?;
        dict.set_item("latency_ms", &self.latency_ms)?;
        dict.set_item("error", &self.error)?;

        let headers_list = PyList::empty_bound(py);
        for (k, v) in &self.response_headers {
            let tuple = pyo3::types::PyTuple::new_bound(py, &[k.as_str(), v.as_str()]);
            headers_list.append(tuple)?;
        }
        dict.set_item("response_headers", headers_list)?;

        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "ConnectionTestResult(url={}, connected={})",
            self.url, self.connected
        )
    }

    fn __str__(&self) -> String {
        if self.connected {
            format!("WebSocket connected to {}", self.url)
        } else {
            format!(
                "WebSocket connection failed to {} - {}",
                self.url,
                self.error.as_deref().unwrap_or("unknown error")
            )
        }
    }
}

/// WebSocket injection test result.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InjectionTestResultPy {
    #[pyo3(get)]
    pub payload: String,
    #[pyo3(get)]
    pub sent: bool,
    #[pyo3(get)]
    pub received_response: bool,
    #[pyo3(get)]
    pub response_content: Option<String>,
    #[pyo3(get)]
    pub vulnerability_detected: bool,
    #[pyo3(get)]
    pub details: String,
}

#[pymethods]
impl InjectionTestResultPy {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("payload", &self.payload)?;
        dict.set_item("sent", self.sent)?;
        dict.set_item("received_response", self.received_response)?;
        dict.set_item("response_content", &self.response_content)?;
        dict.set_item("vulnerability_detected", self.vulnerability_detected)?;
        dict.set_item("details", &self.details)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "InjectionTestResult(payload={}, vuln={})",
            &self.payload[..self.payload.len().min(32)],
            self.vulnerability_detected
        )
    }

    fn __str__(&self) -> String {
        if self.vulnerability_detected {
            format!("Injection vulnerability detected with: {}", self.payload)
        } else {
            format!("Payload '{}' - no vulnerability", self.payload)
        }
    }
}

/// WebSocket origin validation test result.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OriginTestResultPy {
    #[pyo3(get)]
    pub origin: String,
    #[pyo3(get)]
    pub accepted: bool,
    #[pyo3(get)]
    pub status_code: Option<u16>,
    #[pyo3(get)]
    pub details: String,
}

#[pymethods]
impl OriginTestResultPy {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("origin", &self.origin)?;
        dict.set_item("accepted", self.accepted)?;
        dict.set_item("status_code", &self.status_code)?;
        dict.set_item("details", &self.details)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "OriginTestResult(origin={}, accepted={})",
            self.origin, self.accepted
        )
    }

    fn __str__(&self) -> String {
        if self.accepted {
            format!("CSWSH risk: origin '{}' was accepted", self.origin)
        } else {
            format!("Origin '{}' was rejected", self.origin)
        }
    }
}

/// WebSocket fuzz test result.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuzzTestResultPy {
    #[pyo3(get)]
    pub test_name: String,
    #[pyo3(get)]
    pub payload_size: usize,
    #[pyo3(get)]
    pub sent: bool,
    #[pyo3(get)]
    pub connection_dropped: bool,
    #[pyo3(get)]
    pub server_response: Option<String>,
    #[pyo3(get)]
    pub vulnerability_detected: bool,
    #[pyo3(get)]
    pub details: String,
}

#[pymethods]
impl FuzzTestResultPy {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("test_name", &self.test_name)?;
        dict.set_item("payload_size", self.payload_size)?;
        dict.set_item("sent", self.sent)?;
        dict.set_item("connection_dropped", self.connection_dropped)?;
        dict.set_item("server_response", &self.server_response)?;
        dict.set_item("vulnerability_detected", self.vulnerability_detected)?;
        dict.set_item("details", &self.details)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "FuzzTestResult(test={}, size={}, vuln={})",
            self.test_name, self.payload_size, self.vulnerability_detected
        )
    }

    fn __str__(&self) -> String {
        if self.vulnerability_detected {
            format!("Fuzz vulnerability: {}", self.test_name)
        } else {
            format!("Fuzz test '{}' - OK", self.test_name)
        }
    }
}

/// A single WebSocket security finding.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketFindingPy {
    #[pyo3(get)]
    pub category: String,
    #[pyo3(get)]
    pub severity: Severity,
    #[pyo3(get)]
    pub title: String,
    #[pyo3(get)]
    pub description: String,
    #[pyo3(get)]
    pub recommendation: String,
}

#[pymethods]
impl WebSocketFindingPy {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("category", &self.category)?;
        dict.set_item("severity", self.severity.__str__())?;
        dict.set_item("title", &self.title)?;
        dict.set_item("description", &self.description)?;
        dict.set_item("recommendation", &self.recommendation)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "WebSocketFinding(category={}, severity={}, title={})",
            self.category,
            self.severity.__str__(),
            self.title
        )
    }

    fn __str__(&self) -> String {
        format!(
            "[{}] {}: {}",
            self.severity.__str__(),
            self.category,
            self.title
        )
    }
}

/// Complete WebSocket test report.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketReportPy {
    #[pyo3(get)]
    pub target: String,
    connection_test: Option<ConnectionTestResultPy>,
    injection_tests: Vec<InjectionTestResultPy>,
    origin_tests: Vec<OriginTestResultPy>,
    fuzz_tests: Vec<FuzzTestResultPy>,
    findings: Vec<WebSocketFindingPy>,
}

#[pymethods]
impl WebSocketReportPy {
    #[getter]
    fn connection_test(&self) -> Option<ConnectionTestResultPy> {
        self.connection_test.clone()
    }

    #[getter]
    fn injection_tests(&self) -> Vec<InjectionTestResultPy> {
        self.injection_tests.clone()
    }

    #[getter]
    fn origin_tests(&self) -> Vec<OriginTestResultPy> {
        self.origin_tests.clone()
    }

    #[getter]
    fn fuzz_tests(&self) -> Vec<FuzzTestResultPy> {
        self.fuzz_tests.clone()
    }

    #[getter]
    fn findings(&self) -> Vec<WebSocketFindingPy> {
        self.findings.clone()
    }

    #[getter]
    fn finding_count(&self) -> usize {
        self.findings.len()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("target", &self.target)?;
        dict.set_item("finding_count", self.findings.len())?;

        if let Some(ref ct) = self.connection_test {
            dict.set_item("connection_test", ct.to_dict(py)?)?;
        } else {
            dict.set_item("connection_test", py.None())?;
        }

        let inj_list = PyList::empty_bound(py);
        for t in &self.injection_tests {
            inj_list.append(t.to_dict(py)?)?;
        }
        dict.set_item("injection_tests", inj_list)?;

        let orig_list = PyList::empty_bound(py);
        for t in &self.origin_tests {
            orig_list.append(t.to_dict(py)?)?;
        }
        dict.set_item("origin_tests", orig_list)?;

        let fuzz_list = PyList::empty_bound(py);
        for t in &self.fuzz_tests {
            fuzz_list.append(t.to_dict(py)?)?;
        }
        dict.set_item("fuzz_tests", fuzz_list)?;

        let findings_list = PyList::empty_bound(py);
        for f in &self.findings {
            findings_list.append(f.to_dict(py)?)?;
        }
        dict.set_item("findings", findings_list)?;

        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "WebSocketReport(target={}, findings={})",
            self.target,
            self.findings.len()
        )
    }

    fn __str__(&self) -> String {
        format!(
            "WebSocket report for {}: {} findings",
            self.target,
            self.findings.len()
        )
    }
}

impl WebSocketReportPy {
    fn from_engine(engine: eggsec::websocket::WebSocketTestReport) -> Self {
        let connection_test = engine.connection_test.map(|ct| ConnectionTestResultPy {
            url: ct.url,
            connected: ct.connected,
            response_headers: ct.response_headers,
            subprotocols: ct.subprotocols,
            extensions: ct.extensions,
            latency_ms: ct.latency_ms,
            error: ct.error,
        });

        let injection_tests = engine
            .injection_tests
            .into_iter()
            .map(|it| InjectionTestResultPy {
                payload: it.payload,
                sent: it.sent,
                received_response: it.received_response,
                response_content: it.response_content,
                vulnerability_detected: it.vulnerability_detected,
                details: it.details,
            })
            .collect();

        let origin_tests = engine
            .origin_tests
            .into_iter()
            .map(|ot| OriginTestResultPy {
                origin: ot.origin,
                accepted: ot.accepted,
                status_code: ot.status_code,
                details: ot.details,
            })
            .collect();

        let fuzz_tests = engine
            .fuzz_tests
            .into_iter()
            .map(|ft| FuzzTestResultPy {
                test_name: ft.test_name,
                payload_size: ft.payload_size,
                sent: ft.sent,
                connection_dropped: ft.connection_dropped,
                server_response: ft.server_response,
                vulnerability_detected: ft.vulnerability_detected,
                details: ft.details,
            })
            .collect();

        let findings = engine
            .findings
            .into_iter()
            .map(|f| WebSocketFindingPy {
                category: f.category,
                severity: Severity::from_engine(f.severity),
                title: f.title,
                description: f.description,
                recommendation: f.recommendation,
            })
            .collect();

        Self {
            target: engine.target,
            connection_test,
            injection_tests,
            origin_tests,
            fuzz_tests,
            findings,
        }
    }
}

/// Configuration for WebSocket testing.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct WebSocketTestConfigPy {
    #[pyo3(get)]
    pub url: String,
    #[pyo3(get)]
    pub timeout_secs: u64,
    injection_payloads: Vec<String>,
    #[pyo3(get)]
    pub test_connection: bool,
    #[pyo3(get)]
    pub test_origins: bool,
    #[pyo3(get)]
    pub test_injection: bool,
    #[pyo3(get)]
    pub test_dos: bool,
    #[pyo3(get)]
    pub test_message_fuzz: bool,
}

#[pymethods]
impl WebSocketTestConfigPy {
    /// Create a new WebSocket test configuration.
    ///
    /// Args:
    ///     url: WebSocket URL (e.g. "ws://example.com/ws").
    ///     timeout_secs: Timeout per test in seconds.
    ///     injection_payloads: List of payloads for injection testing.
    ///     test_connection: Run connection test (default: True).
    ///     test_origins: Run origin validation tests (default: True).
    ///     test_injection: Run injection tests (default: True).
    ///     test_dos: Run DoS tests (default: False).
    ///     test_message_fuzz: Run message fuzzing tests (default: False).
    #[new]
    #[pyo3(signature = (url, timeout_secs=10, injection_payloads=None, *, test_connection=true, test_origins=true, test_injection=true, test_dos=false, test_message_fuzz=false))]
    fn new(
        url: String,
        timeout_secs: u64,
        injection_payloads: Option<Vec<String>>,
        test_connection: bool,
        test_origins: bool,
        test_injection: bool,
        test_dos: bool,
        test_message_fuzz: bool,
    ) -> PyResult<Self> {
        if url.is_empty() {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "url must not be empty",
            ));
        }
        Ok(Self {
            url,
            timeout_secs,
            injection_payloads: injection_payloads.unwrap_or_default(),
            test_connection,
            test_origins,
            test_injection,
            test_dos,
            test_message_fuzz,
        })
    }

    #[getter]
    fn injection_payloads(&self) -> Vec<String> {
        self.injection_payloads.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "WebSocketTestConfig(url={}, timeout_secs={})",
            self.url, self.timeout_secs
        )
    }
}

/// Run a WebSocket connectivity probe (connection test only).
///
/// Args:
///     url: WebSocket URL to test (e.g. "ws://example.com/ws").
///     timeout_secs: Connection timeout in seconds (default: 10).
///
/// Returns:
///     WebSocketReportPy: Report with connection test result.
///
/// Raises:
///     NetworkError: If the connection fails.
///     ConfigError: If the URL is invalid.
#[pyfunction]
pub fn websocket_probe(url: &str, timeout_secs: u64) -> PyResult<WebSocketReportPy> {
    Python::with_gil(|py| {
        let url_owned = url.to_string();
        let result = runtime_sync::block_on(py, async move {
            let config = eggsec::websocket::WebSocketTestConfig {
                url: url_owned,
                timeout_secs,
                injection_payloads: Vec::new(),
                test_connection: true,
                test_origins: false,
                test_injection: false,
                test_dos: false,
                test_message_fuzz: false,
            };
            let report = eggsec::websocket::run_live_tests(&config).await;
            Ok(report)
        })?;

        Ok(WebSocketReportPy::from_engine(result))
    })
}

/// Run a WebSocket connectivity probe (async).
#[pyfunction]
pub fn async_websocket_probe(
    url: &str,
    timeout_secs: u64,
) -> PyResult<crate::runtime_async::PyFuture> {
    let url_owned = url.to_string();

    crate::runtime_async::spawn_async(async move {
        let config = eggsec::websocket::WebSocketTestConfig {
            url: url_owned,
            timeout_secs,
            injection_payloads: Vec::new(),
            test_connection: true,
            test_origins: false,
            test_injection: false,
            test_dos: false,
            test_message_fuzz: false,
        };
        let report = eggsec::websocket::run_live_tests(&config).await;
        Ok(WebSocketReportPy::from_engine(report))
    })
}

/// Run a full WebSocket fuzz suite (connection, origin, injection, DoS, message fuzz).
///
/// Args:
///     url: WebSocket URL to test (e.g. "ws://example.com/ws").
///     timeout_secs: Timeout per test category in seconds (default: 10).
///
/// Returns:
///     WebSocketReportPy: Full test report with all results and findings.
///
/// Raises:
///     NetworkError: If the connection fails.
///     ConfigError: If the URL is invalid.
#[pyfunction]
pub fn websocket_fuzz(url: &str, timeout_secs: u64) -> PyResult<WebSocketReportPy> {
    Python::with_gil(|py| {
        let url_owned = url.to_string();
        let result = runtime_sync::block_on(py, async move {
            let config = eggsec::websocket::WebSocketTestConfig {
                url: url_owned,
                timeout_secs,
                injection_payloads: vec![
                    "<script>alert(1)</script>".to_string(),
                    "{{7*7}}".to_string(),
                    "${7*7}".to_string(),
                    "../../etc/passwd".to_string(),
                    "\0".to_string(),
                    "A".repeat(10000),
                ],
                test_connection: true,
                test_origins: true,
                test_injection: true,
                test_dos: true,
                test_message_fuzz: true,
            };
            let report = eggsec::websocket::run_live_tests(&config).await;
            Ok(report)
        })?;

        Ok(WebSocketReportPy::from_engine(result))
    })
}

/// Run a full WebSocket fuzz suite (async).
#[pyfunction]
pub fn async_websocket_fuzz(
    url: &str,
    timeout_secs: u64,
) -> PyResult<crate::runtime_async::PyFuture> {
    let url_owned = url.to_string();

    crate::runtime_async::spawn_async(async move {
        let config = eggsec::websocket::WebSocketTestConfig {
            url: url_owned,
            timeout_secs,
            injection_payloads: vec![
                "<script>alert(1)</script>".to_string(),
                "{{7*7}}".to_string(),
                "${7*7}".to_string(),
                "../../etc/passwd".to_string(),
                "\0".to_string(),
                "A".repeat(10000),
            ],
            test_connection: true,
            test_origins: true,
            test_injection: true,
            test_dos: true,
            test_message_fuzz: true,
        };
        let report = eggsec::websocket::run_live_tests(&config).await;
        Ok(WebSocketReportPy::from_engine(report))
    })
}
