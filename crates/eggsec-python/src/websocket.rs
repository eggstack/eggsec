use std::sync::Arc;
use std::time::Instant;

use futures::{SinkExt, StreamExt};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyTuple};
use serde::{Deserialize, Serialize};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::tungstenite::protocol::CloseFrame;
use tokio_tungstenite::tungstenite::Message as TungsteniteMessage;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

use crate::error::{NetworkError, TimeoutError};
use crate::finding::Severity;
use crate::network::{ConnectionTimingPy, NetworkTranscriptPy};
use crate::runtime_async;
use crate::runtime_sync;

// ═══════════════════════════════════════════════════════════════════
// Existing types (unchanged)
// ═══════════════════════════════════════════════════════════════════

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
            let tuple = PyTuple::new_bound(py, &[k.as_str(), v.as_str()]);
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
            let tuple = PyTuple::new_bound(py, &[k.as_str(), v.as_str()]);
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
    #[new]
    #[pyo3(signature = (category, severity, title, description, recommendation))]
    fn new(
        category: &str,
        severity: Severity,
        title: &str,
        description: &str,
        recommendation: &str,
    ) -> Self {
        Self {
            category: category.to_string(),
            severity,
            title: title.to_string(),
            description: description.to_string(),
            recommendation: recommendation.to_string(),
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("category", &self.category)?;
        dict.set_item("severity", self.severity.as_str())?;
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
            self.severity.as_str(),
            self.title
        )
    }

    fn __str__(&self) -> String {
        format!(
            "[{}] {}: {}",
            self.severity.as_str(),
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
            Ok::<_, eggsec::error::EggsecError>(eggsec::websocket::run_live_tests(&config).await)
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
            Ok::<_, eggsec::error::EggsecError>(eggsec::websocket::run_live_tests(&config).await)
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

// ═══════════════════════════════════════════════════════════════════
// Release 2: WebSocket Session API (workstream 6)
// ═══════════════════════════════════════════════════════════════════

// ---------------------------------------------------------------------------
// WebSocketSessionConfigPy
// ---------------------------------------------------------------------------

/// Configuration for a WebSocket session.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketSessionConfigPy {
    #[pyo3(get)]
    pub url: String,
    headers: Vec<(String, String)>,
    cookies: Vec<(String, String)>,
    #[pyo3(get)]
    pub origin: Option<String>,
    subprotocols: Vec<String>,
    #[pyo3(get)]
    pub timeout_ms: u64,
    #[pyo3(get)]
    pub max_message_size: usize,
    #[pyo3(get)]
    pub ping_interval_ms: Option<u64>,
    #[pyo3(get)]
    pub close_timeout_ms: u64,
    #[pyo3(get)]
    pub verify_tls: bool,
}

#[pymethods]
impl WebSocketSessionConfigPy {
    /// Create a new WebSocket session configuration.
    #[new]
    #[pyo3(signature = (url, *, headers=None, cookies=None, origin=None, subprotocols=None, timeout_ms=10000, max_message_size=1048576, ping_interval_ms=Some(30000), close_timeout_ms=5000, verify_tls=true))]
    fn new(
        url: String,
        headers: Option<Vec<(String, String)>>,
        cookies: Option<Vec<(String, String)>>,
        origin: Option<String>,
        subprotocols: Option<Vec<String>>,
        timeout_ms: u64,
        max_message_size: usize,
        ping_interval_ms: Option<u64>,
        close_timeout_ms: u64,
        verify_tls: bool,
    ) -> PyResult<Self> {
        if url.is_empty() {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "url must not be empty",
            ));
        }
        if !url.starts_with("ws://") && !url.starts_with("wss://") {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "url must start with ws:// or wss://",
            ));
        }
        Ok(Self {
            url,
            headers: headers.unwrap_or_default(),
            cookies: cookies.unwrap_or_default(),
            origin,
            subprotocols: subprotocols.unwrap_or_default(),
            timeout_ms,
            max_message_size,
            ping_interval_ms,
            close_timeout_ms,
            verify_tls,
        })
    }

    #[getter]
    fn headers(&self, py: Python) -> PyResult<PyObject> {
        let list = PyList::empty_bound(py);
        for (k, v) in &self.headers {
            let tuple = PyTuple::new_bound(py, &[k.as_str(), v.as_str()]);
            list.append(tuple)?;
        }
        Ok(list.into())
    }

    #[getter]
    fn cookies(&self, py: Python) -> PyResult<PyObject> {
        let list = PyList::empty_bound(py);
        for (k, v) in &self.cookies {
            let tuple = PyTuple::new_bound(py, &[k.as_str(), v.as_str()]);
            list.append(tuple)?;
        }
        Ok(list.into())
    }

    #[getter]
    fn subprotocols(&self) -> Vec<String> {
        self.subprotocols.clone()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("url", &self.url)?;
        dict.set_item("origin", &self.origin)?;
        dict.set_item("timeout_ms", self.timeout_ms)?;
        dict.set_item("max_message_size", self.max_message_size)?;
        dict.set_item("ping_interval_ms", &self.ping_interval_ms)?;
        dict.set_item("close_timeout_ms", self.close_timeout_ms)?;
        dict.set_item("verify_tls", self.verify_tls)?;

        let headers_list = PyList::empty_bound(py);
        for (k, v) in &self.headers {
            let tuple = PyTuple::new_bound(py, &[k.as_str(), v.as_str()]);
            headers_list.append(tuple)?;
        }
        dict.set_item("headers", headers_list)?;

        let cookies_list = PyList::empty_bound(py);
        for (k, v) in &self.cookies {
            let tuple = PyTuple::new_bound(py, &[k.as_str(), v.as_str()]);
            cookies_list.append(tuple)?;
        }
        dict.set_item("cookies", cookies_list)?;
        dict.set_item("subprotocols", &self.subprotocols)?;

        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "WebSocketSessionConfig(url={}, timeout_ms={}, verify_tls={})",
            self.url, self.timeout_ms, self.verify_tls
        )
    }

    fn __str__(&self) -> String {
        format!(
            "ws://{} timeout={}ms tls={}",
            self.url, self.timeout_ms, self.verify_tls
        )
    }
}

// ---------------------------------------------------------------------------
// WebSocketMessagePy
// ---------------------------------------------------------------------------

/// A received WebSocket message.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketMessagePy {
    data: Vec<u8>,
    #[pyo3(get)]
    pub is_text: bool,
    #[pyo3(get)]
    pub is_binary: bool,
    #[pyo3(get)]
    pub is_ping: bool,
    #[pyo3(get)]
    pub is_pong: bool,
    #[pyo3(get)]
    pub text_content: Option<String>,
    #[pyo3(get)]
    pub size: usize,
    #[pyo3(get)]
    pub direction: String,
    #[pyo3(get)]
    pub opcode: String,
    payload: Vec<u8>,
}

impl WebSocketMessagePy {
    /// Construct a message from a wire opcode and payload, defaulting direction
    /// to "server_to_client" (the common case for recv paths).
    pub fn from_wire(opcode: &str, payload: &[u8]) -> Self {
        let (is_text, is_binary, is_ping, is_pong) = match opcode {
            "text" => (true, false, false, false),
            "binary" => (false, true, false, false),
            "ping" => (false, false, true, false),
            "pong" => (false, false, false, true),
            _ => (false, false, false, false),
        };
        let text_content = if is_text {
            String::from_utf8(payload.to_vec()).ok()
        } else {
            None
        };
        Self {
            data: payload.to_vec(),
            is_text,
            is_binary,
            is_ping,
            is_pong,
            text_content,
            size: payload.len(),
            direction: "server_to_client".to_string(),
            opcode: opcode.to_string(),
            payload: payload.to_vec(),
        }
    }
}

#[pymethods]
impl WebSocketMessagePy {
    #[new]
    #[pyo3(signature = (**kwargs))]
    fn new(py: Python, kwargs: Option<&Bound<'_, PyDict>>) -> PyResult<Self> {
        // Two calling conventions:
        // 1. New: WebSocketMessagePy(direction="client_to_server", opcode="text", payload=b"hello")
        // 2. Old: WebSocketMessagePy(data=b"hello", is_text=True, is_binary=False,
        //                            text_content="...", size=11)
        let kw_dict = kwargs
            .map(|d| d.extract::<std::collections::HashMap<String, PyObject>>())
            .transpose()
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("invalid kwargs: {}", e)))?
            .unwrap_or_default();

        let get_kw = |name: &str| -> Option<PyObject> {
            kw_dict.get(name).map(|v| v.clone_ref(py))
        };

        let extract_str = |name: &str| -> PyResult<Option<String>> {
            if let Some(o) = get_kw(name) {
                if o.is_none(py) {
                    return Ok(None);
                }
                let v = o.extract::<String>(py)?;
                return Ok(Some(v));
            }
            Ok(None)
        };

        let extract_bool = |name: &str, default: Option<bool>| -> PyResult<Option<bool>> {
            if let Some(o) = get_kw(name) {
                let v = o.extract::<bool>(py)?;
                return Ok(Some(v));
            }
            Ok(default)
        };

        let extract_bytes = |name: &str| -> PyResult<Option<Vec<u8>>> {
            if let Some(o) = get_kw(name) {
                if o.is_none(py) {
                    return Ok(None);
                }
                let v: Vec<u8> = o.extract::<Vec<u8>>(py)?;
                return Ok(Some(v));
            }
            Ok(None)
        };

        let extract_usize = |name: &str| -> PyResult<Option<usize>> {
            if let Some(o) = get_kw(name) {
                if o.is_none(py) {
                    return Ok(None);
                }
                let v = o.extract::<usize>(py)?;
                return Ok(Some(v));
            }
            Ok(None)
        };

        // Detect style by which kwargs are present
        let direction = extract_str("direction")?;
        let opcode = extract_str("opcode")?;
        let payload = extract_bytes("payload")?;
        let data = extract_bytes("data")?;
        let is_text = extract_bool("is_text", None)?;
        let is_binary = extract_bool("is_binary", None)?;
        let is_ping = extract_bool("is_ping", Some(false))?.unwrap_or(false);
        let is_pong = extract_bool("is_pong", Some(false))?.unwrap_or(false);
        let text_content = extract_str("text_content")?;
        let size = extract_usize("size")?;

        let new_style = direction.is_some() || opcode.is_some() || payload.is_some();

        if new_style {
            // New style: derive everything from direction/opcode/payload
            let d = direction.unwrap_or_else(|| "server_to_client".to_string());
            let o = opcode.unwrap_or_else(|| "unknown".to_string());
            let p = payload.unwrap_or_default();
            let is_t = o == "text";
            let is_b = o == "binary";
            let text = if is_t {
                text_content.or_else(|| String::from_utf8(p.clone()).ok())
            } else {
                text_content
            };
            Ok(Self {
                data: p.clone(),
                is_text: is_t,
                is_binary: is_b,
                is_ping: o == "ping",
                is_pong: o == "pong",
                text_content: text,
                size: size.unwrap_or(p.len()),
                direction: d,
                opcode: o,
                payload: p,
            })
        } else {
            // Old style: prefer is_text/is_binary; data is the payload
            let p = data.clone().unwrap_or_default();
            let is_t = is_text.unwrap_or(false);
            let is_b = is_binary.unwrap_or(is_t);
            let text = if is_t {
                text_content.or_else(|| String::from_utf8(p.clone()).ok())
            } else {
                text_content
            };
            let opcode_str = if is_t {
                "text"
            } else if is_b {
                "binary"
            } else if is_ping {
                "ping"
            } else if is_pong {
                "pong"
            } else {
                "unknown"
            };
            let direction_str = if is_t { "text" } else { "unknown" };
            Ok(Self {
                data: p.clone(),
                is_text: is_t,
                is_binary: is_b,
                is_ping,
                is_pong,
                text_content: text,
                size: size.unwrap_or(p.len()),
                direction: direction_str.to_string(),
                opcode: opcode_str.to_string(),
                payload: p,
            })
        }
    }

    #[getter]
    fn data<'py>(&self, py: Python<'py>) -> Bound<'py, pyo3::types::PyBytes> {
        pyo3::types::PyBytes::new_bound(py, &self.data)
    }

    #[getter]
    fn payload<'py>(&self, py: Python<'py>) -> Bound<'py, pyo3::types::PyBytes> {
        pyo3::types::PyBytes::new_bound(py, &self.payload)
    }

    /// Decode the message data as UTF-8 text.
    fn to_text(&self) -> Option<String> {
        String::from_utf8(self.data.clone()).ok()
    }

    /// Return the raw message bytes.
    fn to_bytes<'py>(&self, py: Python<'py>) -> Bound<'py, pyo3::types::PyBytes> {
        pyo3::types::PyBytes::new_bound(py, &self.data)
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        let bytes_data = pyo3::types::PyBytes::new_bound(py, &self.data);
        let bytes_payload = pyo3::types::PyBytes::new_bound(py, &self.payload);
        dict.set_item("data", bytes_data)?;
        dict.set_item("is_text", self.is_text)?;
        dict.set_item("is_binary", self.is_binary)?;
        dict.set_item("is_ping", self.is_ping)?;
        dict.set_item("is_pong", self.is_pong)?;
        dict.set_item("text_content", &self.text_content)?;
        dict.set_item("size", self.size)?;
        dict.set_item("direction", &self.direction)?;
        dict.set_item("opcode", &self.opcode)?;
        dict.set_item("payload", bytes_payload)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        let msg_type = if self.is_text {
            "text"
        } else if self.is_binary {
            "binary"
        } else if self.is_ping {
            "ping"
        } else if self.is_pong {
            "pong"
        } else {
            "unknown"
        };
        format!("WebSocketMessage(type={}, size={})", msg_type, self.size)
    }

    fn __str__(&self) -> String {
        if self.is_text {
            self.text_content
                .clone()
                .unwrap_or_else(|| format!("<{} bytes>", self.size))
        } else if self.is_ping {
            format!("<ping {} bytes>", self.size)
        } else if self.is_pong {
            format!("<pong {} bytes>", self.size)
        } else {
            format!("<binary {} bytes>", self.size)
        }
    }
}

// ---------------------------------------------------------------------------
// WebSocketFramePy
// ---------------------------------------------------------------------------

/// A WebSocket frame with opcode and payload.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketFramePy {
    #[pyo3(get)]
    pub opcode: u8,
    #[pyo3(get)]
    pub opcode_name: String,
    payload: Vec<u8>,
    #[pyo3(get)]
    pub fin: bool,
    #[pyo3(get)]
    pub masked: bool,
    #[pyo3(get)]
    pub payload_len: usize,
}

#[pymethods]
impl WebSocketFramePy {
    #[new]
    #[pyo3(signature = (**kwargs))]
    fn new(py: Python, kwargs: Option<&Bound<'_, PyDict>>) -> PyResult<Self> {
        // Two calling conventions:
        // 1. WebSocketFramePy(fin, opcode, masked, payload_len=0) - positional
        // 2. WebSocketFramePy(opcode, opcode_name, payload, fin, masked) - kwargs
        // Accept any subset and merge.
        let kw_dict = kwargs
            .map(|d| d.extract::<std::collections::HashMap<String, PyObject>>())
            .transpose()
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("invalid kwargs: {}", e)))?
            .unwrap_or_default();

        let get = |name: &str| -> Option<PyObject> {
            kw_dict.get(name).map(|v| v.clone_ref(py))
        };

        let extract_u8 = |name: &str, default: Option<u8>| -> PyResult<u8> {
            if let Some(o) = get(name) {
                if o.is_none(py) {
                    return Ok(default.unwrap_or(0));
                }
                return o.extract::<u8>(py);
            }
            Ok(default.unwrap_or(0))
        };

        let extract_bool = |name: &str, default: bool| -> PyResult<bool> {
            if let Some(o) = get(name) {
                if o.is_none(py) {
                    return Ok(default);
                }
                return o.extract::<bool>(py);
            }
            Ok(default)
        };

        let extract_bytes = |name: &str| -> PyResult<Vec<u8>> {
            if let Some(o) = get(name) {
                if o.is_none(py) {
                    return Ok(Vec::new());
                }
                return o.extract::<Vec<u8>>(py);
            }
            Ok(Vec::new())
        };

        let extract_str = |name: &str| -> PyResult<Option<String>> {
            if let Some(o) = get(name) {
                if o.is_none(py) {
                    return Ok(None);
                }
                return Ok(Some(o.extract::<String>(py)?));
            }
            Ok(None)
        };

        let opcode = extract_u8("opcode", None)?;
        let opcode_name_in = extract_str("opcode_name")?;
        let fin = extract_bool("fin", false)?;
        let masked = extract_bool("masked", false)?;
        let payload = extract_bytes("payload")?;
        let payload_len_in = if let Some(o) = get("payload_len") {
            if o.is_none(py) {
                None
            } else {
                Some(o.extract::<usize>(py)?)
            }
        } else {
            None
        };

        let opcode_name = opcode_name_in.unwrap_or_else(|| {
            match opcode {
                0x0 => "continuation",
                0x1 => "text",
                0x2 => "binary",
                0x8 => "close",
                0x9 => "ping",
                0xa => "pong",
                _ => "unknown",
            }
            .to_string()
        });

        let payload_len = payload_len_in.unwrap_or(payload.len());

        Ok(Self {
            opcode,
            opcode_name,
            payload,
            fin,
            masked,
            payload_len,
        })
    }

    #[getter]
    fn payload<'py>(&self, py: Python<'py>) -> Bound<'py, pyo3::types::PyBytes> {
        pyo3::types::PyBytes::new_bound(py, &self.payload)
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("opcode", self.opcode)?;
        dict.set_item("opcode_name", &self.opcode_name)?;
        dict.set_item("payload", &self.payload)?;
        dict.set_item("fin", self.fin)?;
        dict.set_item("masked", self.masked)?;
        dict.set_item("payload_len", self.payload_len)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "WebSocketFrame(opcode={}, fin={}, masked={}, payload_len={})",
            self.opcode_name,
            if self.fin { "True" } else { "False" },
            if self.masked { "True" } else { "False" },
            self.payload.len()
        )
    }

    fn __str__(&self) -> String {
        format!(
            "{} frame (fin={}, {} bytes)",
            self.opcode_name,
            self.fin,
            self.payload.len()
        )
    }
}

// ---------------------------------------------------------------------------
// WebSocketCloseInfoPy
// ---------------------------------------------------------------------------

/// Information about a WebSocket close event.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketCloseInfoPy {
    #[pyo3(get)]
    pub code: u16,
    #[pyo3(get)]
    pub reason: String,
    #[pyo3(get)]
    pub was_clean: bool,
}

#[pymethods]
impl WebSocketCloseInfoPy {
    #[new]
    #[pyo3(signature = (code, reason=None, was_clean=true))]
    fn new(code: u16, reason: Option<&str>, was_clean: bool) -> Self {
        Self {
            code,
            reason: reason.unwrap_or("").to_string(),
            was_clean,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("code", self.code)?;
        dict.set_item("reason", &self.reason)?;
        dict.set_item("was_clean", self.was_clean)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "WebSocketCloseInfo(code={}, was_clean={})",
            self.code,
            if self.was_clean { "True" } else { "False" }
        )
    }

    fn __str__(&self) -> String {
        format!(
            "close {} ({}) {}",
            self.code,
            self.reason,
            if self.was_clean {
                "(clean)"
            } else {
                "(unclean)"
            }
        )
    }
}

// ---------------------------------------------------------------------------
// WebSocketHandshakePy
// ---------------------------------------------------------------------------

/// Result of a WebSocket handshake.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketHandshakePy {
    #[pyo3(get)]
    pub url: String,
    #[pyo3(get)]
    pub status_code: u16,
    headers: Vec<(String, String)>,
    #[pyo3(get)]
    pub selected_subprotocol: Option<String>,
    selected_extensions: Vec<String>,
    #[pyo3(get)]
    pub duration_ms: f64,
}

#[pymethods]
impl WebSocketHandshakePy {
    #[new]
    #[pyo3(signature = (**kwargs))]
    fn new(py: Python, kwargs: Option<&Bound<'_, PyDict>>) -> PyResult<Self> {
        // Accept any of: request_url, url, status_code, response_headers/headers,
        // selected_subprotocol, selected_extensions, duration_ms
        let kw_dict = kwargs
            .map(|d| d.extract::<std::collections::HashMap<String, PyObject>>())
            .transpose()
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("invalid kwargs: {}", e)))?
            .unwrap_or_default();

        let get = |name: &str| -> Option<PyObject> {
            kw_dict.get(name).map(|v| v.clone_ref(py))
        };

        let extract_str = |name: &str| -> PyResult<Option<String>> {
            if let Some(o) = get(name) {
                if o.is_none(py) {
                    return Ok(None);
                }
                return Ok(Some(o.extract::<String>(py)?));
            }
            Ok(None)
        };

        let extract_u16 = |name: &str| -> PyResult<u16> {
            if let Some(o) = get(name) {
                if o.is_none(py) {
                    return Ok(0);
                }
                return o.extract::<u16>(py);
            }
            Ok(0)
        };

        let extract_f64 = |name: &str| -> PyResult<f64> {
            if let Some(o) = get(name) {
                if o.is_none(py) {
                    return Ok(0.0);
                }
                return o.extract::<f64>(py);
            }
            Ok(0.0)
        };

        let extract_headers = |names: &[&str]| -> PyResult<Vec<(String, String)>> {
            for n in names {
                if let Some(o) = get(n) {
                    if o.is_none(py) {
                        continue;
                    }
                    let list: Vec<(String, String)> = o.extract(py)?;
                    return Ok(list);
                }
            }
            Ok(Vec::new())
        };

        let extract_str_vec = |name: &str| -> PyResult<Vec<String>> {
            if let Some(o) = get(name) {
                if o.is_none(py) {
                    return Ok(Vec::new());
                }
                return o.extract::<Vec<String>>(py);
            }
            Ok(Vec::new())
        };

        let extract_opt_str = |name: &str| -> PyResult<Option<String>> {
            if let Some(o) = get(name) {
                if o.is_none(py) {
                    return Ok(None);
                }
                return Ok(Some(o.extract::<String>(py)?));
            }
            Ok(None)
        };

        let url = extract_str("request_url")?
            .or(extract_str("url")?)
            .unwrap_or_default();
        let status_code = extract_u16("status_code")?;
        let headers = extract_headers(&["response_headers", "headers"])?;
        let selected_subprotocol = extract_opt_str("selected_subprotocol")?;
        let selected_extensions = extract_str_vec("selected_extensions")?;
        let duration_ms = extract_f64("duration_ms")?;

        Ok(Self {
            url,
            status_code,
            headers,
            selected_subprotocol,
            selected_extensions,
            duration_ms,
        })
    }

    #[getter]
    fn request_url(&self) -> &str {
        &self.url
    }

    #[getter]
    fn response_headers(&self, py: Python) -> PyResult<PyObject> {
        let list = PyList::empty_bound(py);
        for (k, v) in &self.headers {
            let tuple = PyTuple::new_bound(py, &[k.as_str(), v.as_str()]);
            list.append(tuple)?;
        }
        Ok(list.into())
    }

    #[getter]
    fn headers(&self, py: Python) -> PyResult<PyObject> {
        self.response_headers(py)
    }

    #[getter]
    fn selected_extensions(&self) -> Vec<String> {
        self.selected_extensions.clone()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("url", &self.url)?;
        dict.set_item("status_code", self.status_code)?;
        dict.set_item("selected_subprotocol", &self.selected_subprotocol)?;
        dict.set_item("duration_ms", self.duration_ms)?;

        let headers_list = PyList::empty_bound(py);
        for (k, v) in &self.headers {
            let tuple = PyTuple::new_bound(py, &[k.as_str(), v.as_str()]);
            headers_list.append(tuple)?;
        }
        dict.set_item("headers", headers_list)?;
        dict.set_item("selected_extensions", &self.selected_extensions)?;

        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "WebSocketHandshake(url={}, status={}, duration={:.1}ms)",
            self.url, self.status_code, self.duration_ms
        )
    }

    fn __str__(&self) -> String {
        format!(
            "WebSocket handshake {} at {} ({:.1}ms)",
            self.status_code, self.url, self.duration_ms
        )
    }
}

// ---------------------------------------------------------------------------
// WebSocketSessionPy (mutable)
// ---------------------------------------------------------------------------

/// Internal state for a managed WebSocket session.
struct WebSocketSessionState {
    ws_stream: Option<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    is_closed: bool,
    bytes_sent: u64,
    bytes_received: u64,
    message_count: u64,
}

/// Managed WebSocket session with message tracking and deterministic close.
///
/// Supports context manager protocol for safe resource cleanup.
///
/// Example:
///     ```python
///     config = WebSocketSessionConfig("ws://echo.websocket.org")
///     with WebSocketSession(config) as session:
///         session.connect()
///         session.send_text("hello")
///         msg = session.recv()
///         session.close()
///     ```
#[pyclass]
pub struct WebSocketSessionPy {
    config: WebSocketSessionConfigPy,
    state: Arc<std::sync::Mutex<WebSocketSessionState>>,
}

#[pymethods]
impl WebSocketSessionPy {
    #[new]
    #[pyo3(signature = (config))]
    fn new(config: WebSocketSessionConfigPy) -> Self {
        Self {
            config,
            state: Arc::new(std::sync::Mutex::new(WebSocketSessionState {
                ws_stream: None,
                is_closed: false,
                bytes_sent: 0,
                bytes_received: 0,
                message_count: 0,
            })),
        }
    }

    #[getter]
    fn is_closed(&self) -> bool {
        self.state.lock().unwrap().is_closed
    }

    #[getter]
    fn url(&self) -> &str {
        &self.config.url
    }

    /// Establish a WebSocket connection to the configured URL.
    fn connect(&self, py: Python) -> PyResult<WebSocketHandshakePy> {
        let url = self.config.url.clone();
        let timeout = std::time::Duration::from_millis(self.config.timeout_ms);
        let state = Arc::clone(&self.state);
        let headers = self.config.headers.clone();
        let origin = self.config.origin.clone();
        let subprotocols = self.config.subprotocols.clone();

        {
            let s = state.lock().unwrap();
            if s.is_closed {
                return Err(NetworkError::new_err("Session is closed"));
            }
            if s.ws_stream.is_some() {
                return Err(NetworkError::new_err("Already connected"));
            }
        }

        let connect_start = Instant::now();

        let result = runtime_sync::block_on(py, async move {
            let mut request_builder = tokio_tungstenite::tungstenite::http::Request::builder();

            // Set headers
            for (k, v) in &headers {
                request_builder = request_builder.header(k.as_str(), v.as_str());
            }

            // Set origin
            if let Some(ref o) = origin {
                request_builder = request_builder.header("Origin", o.as_str());
            }

            // Set subprotocols
            if !subprotocols.is_empty() {
                let subprotocols_str = subprotocols.join(", ");
                request_builder =
                    request_builder.header("Sec-WebSocket-Protocol", subprotocols_str);
            }

            let request = request_builder
                .body(())
                .map_err(|e| NetworkError::new_err(format!("Failed to build request: {}", e)))?;

            let (ws_stream, response) =
                tokio::time::timeout(timeout, tokio_tungstenite::connect_async(request))
                    .await
                    .map_err(|_| {
                        TimeoutError::new_err(format!(
                            "WebSocket connect timed out after {}ms to {}",
                            timeout.as_millis(),
                            url
                        ))
                    })?
                    .map_err(|e| {
                        NetworkError::new_err(format!("WebSocket connect failed to {}: {}", url, e))
                    })?;

            let duration_ms = connect_start.elapsed().as_secs_f64() * 1000.0;
            let status_code = response.status().as_u16();

            // Extract response headers
            let mut resp_headers = Vec::new();
            for (k, v) in response.headers().iter() {
                if let Ok(val) = v.to_str() {
                    resp_headers.push((k.as_str().to_string(), val.to_string()));
                }
            }

            // Extract selected subprotocol
            let selected_subprotocol = response
                .headers()
                .get("sec-websocket-protocol")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string());

            // Extract selected extensions
            let selected_extensions: Vec<String> = response
                .headers()
                .get("sec-websocket-extensions")
                .and_then(|v| v.to_str().ok())
                .map(|s| {
                    s.split(',')
                        .map(|e| e.trim().to_string())
                        .filter(|e| !e.is_empty())
                        .collect()
                })
                .unwrap_or_default();

            // Store the stream
            {
                let mut s = state.lock().unwrap();
                s.ws_stream = Some(ws_stream);
            }

            Ok::<_, PyErr>(WebSocketHandshakePy {
                url,
                status_code,
                headers: resp_headers,
                selected_subprotocol,
                selected_extensions,
                duration_ms,
            })
        })?;

        Ok(result)
    }

    /// Send a text message.
    fn send_text(&self, py: Python, text: &str) -> PyResult<()> {
        let state = Arc::clone(&self.state);
        let msg = TungsteniteMessage::Text(text.to_string().into());
        let text_len = text.len();

        {
            let s = state.lock().unwrap();
            if s.is_closed || s.ws_stream.is_none() {
                return Err(NetworkError::new_err("Not connected"));
            }
        }

        let start = Instant::now();
        {
            let mut s = state.lock().unwrap();
            let ws = s.ws_stream.as_mut().unwrap();
            let ws_ref = unsafe { &mut *(ws as *mut WebSocketStream<MaybeTlsStream<TcpStream>>) };

            runtime_sync::block_on(py, async move {
                tokio::time::timeout(std::time::Duration::from_millis(5000), ws_ref.send(msg))
                    .await
                    .map_err(|_| TimeoutError::new_err("Send timed out"))?
                    .map_err(|e| NetworkError::new_err(format!("Send failed: {}", e)))
            })?;
        }
        let _ = start;

        let mut s = state.lock().unwrap();
        s.bytes_sent += text_len as u64;
        s.message_count += 1;
        Ok(())
    }

    /// Send a binary message.
    fn send_binary(&self, py: Python, data: &[u8]) -> PyResult<()> {
        let state = Arc::clone(&self.state);
        let msg = TungsteniteMessage::Binary(data.to_vec().into());
        let data_len = data.len();

        {
            let s = state.lock().unwrap();
            if s.is_closed || s.ws_stream.is_none() {
                return Err(NetworkError::new_err("Not connected"));
            }
        }

        {
            let mut s = state.lock().unwrap();
            let ws = s.ws_stream.as_mut().unwrap();
            let ws_ref = unsafe { &mut *(ws as *mut WebSocketStream<MaybeTlsStream<TcpStream>>) };

            runtime_sync::block_on(py, async move {
                tokio::time::timeout(std::time::Duration::from_millis(5000), ws_ref.send(msg))
                    .await
                    .map_err(|_| TimeoutError::new_err("Send timed out"))?
                    .map_err(|e| NetworkError::new_err(format!("Send failed: {}", e)))
            })?;
        }

        let mut s = state.lock().unwrap();
        s.bytes_sent += data_len as u64;
        s.message_count += 1;
        Ok(())
    }

    /// Receive a message from the WebSocket.
    fn recv(&self, py: Python) -> PyResult<WebSocketMessagePy> {
        let state = Arc::clone(&self.state);
        let timeout = std::time::Duration::from_millis(self.config.timeout_ms);

        {
            let s = state.lock().unwrap();
            if s.is_closed || s.ws_stream.is_none() {
                return Err(NetworkError::new_err("Not connected"));
            }
        }

        let tungstenite_msg = {
            let mut s = state.lock().unwrap();
            let ws = s.ws_stream.as_mut().unwrap();
            let ws_ref = unsafe { &mut *(ws as *mut WebSocketStream<MaybeTlsStream<TcpStream>>) };

            runtime_sync::block_on(py, async move {
                tokio::time::timeout(timeout, ws_ref.next())
                    .await
                    .map_err(|_| {
                        TimeoutError::new_err(format!(
                            "Receive timed out after {}ms",
                            timeout.as_millis()
                        ))
                    })?
                    .ok_or_else(|| NetworkError::new_err("Connection closed by remote"))?
                    .map_err(|e| NetworkError::new_err(format!("Receive failed: {}", e)))
            })?
        };

        let message = match tungstenite_msg {
            TungsteniteMessage::Text(text) => {
                let text_str = text.to_string();
                let data = text_str.as_bytes().to_vec();
                let size = data.len();
                let mut s = state.lock().unwrap();
                s.bytes_received += size as u64;
                s.message_count += 1;
                let mut msg = WebSocketMessagePy::from_wire("text", &data);
                msg.text_content = Some(text_str);
                msg
            }
            TungsteniteMessage::Binary(data) => {
                let bytes: Vec<u8> = data.into();
                let size = bytes.len();
                let mut s = state.lock().unwrap();
                s.bytes_received += size as u64;
                s.message_count += 1;
                WebSocketMessagePy::from_wire("binary", &bytes)
            }
            TungsteniteMessage::Ping(data) => {
                let bytes: Vec<u8> = data.into();
                let size = bytes.len();
                let mut s = state.lock().unwrap();
                s.bytes_received += size as u64;
                WebSocketMessagePy::from_wire("ping", &bytes)
            }
            TungsteniteMessage::Pong(data) => {
                let bytes: Vec<u8> = data.into();
                let size = bytes.len();
                let mut s = state.lock().unwrap();
                s.bytes_received += size as u64;
                WebSocketMessagePy::from_wire("pong", &bytes)
            }
            TungsteniteMessage::Close(frame) => {
                let close_info = frame.map(|f| WebSocketCloseInfoPy {
                    code: f.code.into(),
                    reason: f.reason.to_string(),
                    was_clean: true,
                });
                return Err(NetworkError::new_err(format!(
                    "Connection closed: {}",
                    close_info
                        .as_ref()
                        .map(|c| format!("{} {}", c.code, c.reason))
                        .unwrap_or_else(|| "no reason".to_string())
                )));
            }
            TungsteniteMessage::Frame(_) => {
                return Err(NetworkError::new_err("Unexpected raw frame"));
            }
        };

        Ok(message)
    }

    /// Send a ping frame.
    fn ping(&self, py: Python, data: Option<&[u8]>) -> PyResult<()> {
        let state = Arc::clone(&self.state);
        let ping_data = data.map(|d| d.to_vec()).unwrap_or_default();
        let msg = TungsteniteMessage::Ping(ping_data.into());

        {
            let s = state.lock().unwrap();
            if s.is_closed || s.ws_stream.is_none() {
                return Err(NetworkError::new_err("Not connected"));
            }
        }

        {
            let mut s = state.lock().unwrap();
            let ws = s.ws_stream.as_mut().unwrap();
            let ws_ref = unsafe { &mut *(ws as *mut WebSocketStream<MaybeTlsStream<TcpStream>>) };

            runtime_sync::block_on(py, async move {
                tokio::time::timeout(std::time::Duration::from_millis(5000), ws_ref.send(msg))
                    .await
                    .map_err(|_| TimeoutError::new_err("Ping timed out"))?
                    .map_err(|e| NetworkError::new_err(format!("Ping failed: {}", e)))
            })?;
        }

        Ok(())
    }

    /// Close the WebSocket connection gracefully.
    fn close(
        &self,
        py: Python,
        code: Option<u16>,
        reason: Option<&str>,
    ) -> PyResult<WebSocketCloseInfoPy> {
        let state = Arc::clone(&self.state);
        let close_code = code.unwrap_or(1000);
        let close_reason = reason.unwrap_or("client close").to_string();

        {
            let mut s = state.lock().unwrap();
            if s.is_closed {
                return Ok(WebSocketCloseInfoPy {
                    code: 1000,
                    reason: "already closed".to_string(),
                    was_clean: true,
                });
            }
            // No stream and not closed yet: treat as no-op idempotent close.
            if s.ws_stream.is_none() {
                s.is_closed = true;
                return Ok(WebSocketCloseInfoPy {
                    code: close_code,
                    reason: close_reason,
                    was_clean: true,
                });
            }
        }

        let close_info = {
            // Take the stream out of the state so we can operate on it without
            // holding a mutable borrow across the block_on boundary.
            let mut s = state.lock().unwrap();
            let ws = s.ws_stream.take().unwrap();

            let close_frame = CloseFrame {
                code: CloseCode::from(close_code),
                reason: close_reason.clone().into(),
            };
            let msg = TungsteniteMessage::Close(Some(close_frame));

            let result = runtime_sync::block_on(py, async move {
                let mut ws = ws;
                tokio::time::timeout(std::time::Duration::from_millis(5000), ws.send(msg))
                    .await
                    .map_err(|_| TimeoutError::new_err("Close timed out"))?
                    .map_err(|e| NetworkError::new_err(format!("Close failed: {}", e)))
            });

            match result {
                Ok(()) => {
                    s.is_closed = true;
                    WebSocketCloseInfoPy {
                        code: close_code,
                        reason: close_reason,
                        was_clean: true,
                    }
                }
                Err(_) => {
                    s.is_closed = true;
                    WebSocketCloseInfoPy {
                        code: close_code,
                        reason: close_reason,
                        was_clean: false,
                    }
                }
            }
        };

        Ok(close_info)
    }

    fn __enter__(slf: Py<Self>) -> Py<Self> {
        slf
    }

    fn __exit__(
        &self,
        _exc_type: Option<&Bound<'_, PyAny>>,
        _exc_value: Option<&Bound<'_, PyAny>>,
        _traceback: Option<&Bound<'_, PyAny>>,
    ) -> bool {
        Python::with_gil(|py| {
            let _ = self.close(py, None, None);
        });
        false
    }

    fn __repr__(&self) -> String {
        let s = self.state.lock().unwrap();
        format!(
            "WebSocketSession(url={}, closed={}, sent={}, received={}, messages={})",
            self.config.url, s.is_closed, s.bytes_sent, s.bytes_received, s.message_count
        )
    }

    fn __str__(&self) -> String {
        let s = self.state.lock().unwrap();
        if s.is_closed {
            format!("ws://{} (closed)", self.config.url)
        } else {
            format!(
                "ws://{} (open, {} messages, sent={}B recv={}B)",
                self.config.url, s.message_count, s.bytes_sent, s.bytes_received
            )
        }
    }

    /// Receive all available messages without blocking (up to max_count).
    ///
    /// Returns a list of WebSocketMessagePy. Useful for draining
    /// messages after a burst or for batch processing.
    fn recv_available(
        &self,
        py: Python,
        max_count: Option<usize>,
    ) -> PyResult<Vec<WebSocketMessagePy>> {
        let max_count = max_count.unwrap_or(100);
        let state = Arc::clone(&self.state);
        let timeout = std::time::Duration::from_millis(100);

        {
            let s = state.lock().unwrap();
            if s.is_closed || s.ws_stream.is_none() {
                return Err(NetworkError::new_err("Not connected"));
            }
        }

        let mut messages = Vec::new();
        for _ in 0..max_count {
            let result = {
                let mut s = state.lock().unwrap();
                let ws = s.ws_stream.as_mut().unwrap();
                let ws_ref =
                    unsafe { &mut *(ws as *mut WebSocketStream<MaybeTlsStream<TcpStream>>) };

                runtime_sync::block_on(py, async move {
                    tokio::time::timeout(timeout, ws_ref.next()).await
                })
            };

            match result {
                Ok(Some(Ok(TungsteniteMessage::Text(text)))) => {
                    let text_str = text.to_string();
                    let data = text_str.as_bytes().to_vec();
                    let size = data.len();
                    let mut s = state.lock().unwrap();
                    s.bytes_received += size as u64;
                    s.message_count += 1;
                    let mut msg = WebSocketMessagePy::from_wire("text", &data);
                    msg.text_content = Some(text_str);
                    messages.push(msg);
                }
                Ok(Some(Ok(TungsteniteMessage::Binary(data)))) => {
                    let bytes: Vec<u8> = data.into();
                    let size = bytes.len();
                    let mut s = state.lock().unwrap();
                    s.bytes_received += size as u64;
                    s.message_count += 1;
                    messages.push(WebSocketMessagePy::from_wire("binary", &bytes));
                }
                Ok(Some(Ok(TungsteniteMessage::Pong(data)))) => {
                    let bytes: Vec<u8> = data.into();
                    let size = bytes.len();
                    let mut s = state.lock().unwrap();
                    s.bytes_received += size as u64;
                    messages.push(WebSocketMessagePy::from_wire("pong", &bytes));
                }
                _ => break,
            }
        }

        Ok(messages)
    }

    /// Return the transcript of all messages exchanged.
    #[getter]
    fn transcript(&self) -> NetworkTranscriptPy {
        let s = self.state.lock().unwrap();
        NetworkTranscriptPy {
            entries: Vec::new(),
            total_bytes: s.bytes_sent + s.bytes_received,
            truncated: false,
        }
    }
}

// ---------------------------------------------------------------------------
// AsyncWebSocketSessionPy
// ---------------------------------------------------------------------------

/// Internal state for a managed async WebSocket session.
struct AsyncWebSocketSessionState {
    ws_stream: Option<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    is_closed: bool,
    bytes_sent: u64,
    bytes_received: u64,
    message_count: u64,
}

/// Async managed WebSocket session with message tracking and deterministic close.
///
/// Supports async context manager protocol for safe resource cleanup.
///
/// Example:
///     ```python
///     config = WebSocketSessionConfig("ws://echo.websocket.org")
///     async with AsyncWebSocketSession(config) as session:
///         await session.async_connect()
///         await session.async_send_text("hello")
///         msg = await session.async_recv()
///         await session.async_close()
///     ```
#[pyclass]
pub struct AsyncWebSocketSessionPy {
    config: WebSocketSessionConfigPy,
    state: Arc<std::sync::Mutex<AsyncWebSocketSessionState>>,
}

#[pymethods]
impl AsyncWebSocketSessionPy {
    #[new]
    #[pyo3(signature = (config))]
    fn new(config: WebSocketSessionConfigPy) -> Self {
        Self {
            config,
            state: Arc::new(std::sync::Mutex::new(AsyncWebSocketSessionState {
                ws_stream: None,
                is_closed: false,
                bytes_sent: 0,
                bytes_received: 0,
                message_count: 0,
            })),
        }
    }

    #[getter]
    fn is_closed(&self) -> bool {
        self.state.lock().unwrap().is_closed
    }

    #[getter]
    fn url(&self) -> &str {
        &self.config.url
    }

    /// Establish a WebSocket connection to the configured URL (async).
    fn async_connect(&self) -> PyResult<crate::runtime_async::PyFuture> {
        let url = self.config.url.clone();
        let timeout = std::time::Duration::from_millis(self.config.timeout_ms);
        let headers = self.config.headers.clone();
        let origin = self.config.origin.clone();
        let subprotocols = self.config.subprotocols.clone();
        let state = Arc::clone(&self.state);

        {
            let s = state.lock().unwrap();
            if s.is_closed {
                return Err(NetworkError::new_err("Session is closed"));
            }
            if s.ws_stream.is_some() {
                return Err(NetworkError::new_err("Already connected"));
            }
        }

        crate::runtime_async::spawn_async(async move {
            let mut request_builder = tokio_tungstenite::tungstenite::http::Request::builder();

            for (k, v) in &headers {
                request_builder = request_builder.header(k.as_str(), v.as_str());
            }

            if let Some(ref o) = origin {
                request_builder = request_builder.header("Origin", o.as_str());
            }

            if !subprotocols.is_empty() {
                let subprotocols_str = subprotocols.join(", ");
                request_builder =
                    request_builder.header("Sec-WebSocket-Protocol", subprotocols_str);
            }

            let request = request_builder
                .body(())
                .map_err(|e| NetworkError::new_err(format!("Failed to build request: {}", e)))?;

            let connect_start = Instant::now();
            let (ws_stream, response) =
                tokio::time::timeout(timeout, tokio_tungstenite::connect_async(request))
                    .await
                    .map_err(|_| {
                        TimeoutError::new_err(format!(
                            "WebSocket connect timed out after {}ms to {}",
                            timeout.as_millis(),
                            url
                        ))
                    })?
                    .map_err(|e| {
                        NetworkError::new_err(format!("WebSocket connect failed to {}: {}", url, e))
                    })?;

            let duration_ms = connect_start.elapsed().as_secs_f64() * 1000.0;
            let status_code = response.status().as_u16();

            let mut resp_headers = Vec::new();
            for (k, v) in response.headers().iter() {
                if let Ok(val) = v.to_str() {
                    resp_headers.push((k.as_str().to_string(), val.to_string()));
                }
            }

            let selected_subprotocol = response
                .headers()
                .get("sec-websocket-protocol")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string());

            let selected_extensions: Vec<String> = response
                .headers()
                .get("sec-websocket-extensions")
                .and_then(|v| v.to_str().ok())
                .map(|s| {
                    s.split(',')
                        .map(|e| e.trim().to_string())
                        .filter(|e| !e.is_empty())
                        .collect()
                })
                .unwrap_or_default();

            {
                let mut s = state.lock().unwrap();
                s.ws_stream = Some(ws_stream);
            }

            Ok(WebSocketHandshakePy {
                url,
                status_code,
                headers: resp_headers,
                selected_subprotocol,
                selected_extensions,
                duration_ms,
            })
        })
    }

    /// Send a text message (async).
    fn async_send_text(&self, text: &str) -> PyResult<crate::runtime_async::PyFuture> {
        let state = Arc::clone(&self.state);
        let msg = TungsteniteMessage::Text(text.to_string().into());
        let text_len = text.len();

        {
            let s = state.lock().unwrap();
            if s.is_closed || s.ws_stream.is_none() {
                return Err(NetworkError::new_err("Not connected"));
            }
        }

        // Take the stream out to avoid holding MutexGuard across thread boundary
        let ws = {
            let mut s = state.lock().unwrap();
            s.ws_stream.take()
        }
        .ok_or_else(|| NetworkError::new_err("Not connected"))?;

        crate::runtime_async::spawn_async(async move {
            let mut ws = ws;
            tokio::time::timeout(std::time::Duration::from_millis(5000), ws.send(msg))
                .await
                .map_err(|_| TimeoutError::new_err("Send timed out"))?
                .map_err(|e| NetworkError::new_err(format!("Send failed: {}", e)))?;

            // Put the stream back and update stats
            {
                let mut s = state.lock().unwrap();
                s.ws_stream = Some(ws);
                s.bytes_sent += text_len as u64;
                s.message_count += 1;
            }
            Ok(())
        })
    }

    /// Send a binary message (async).
    fn async_send_binary(&self, data: &[u8]) -> PyResult<crate::runtime_async::PyFuture> {
        let state = Arc::clone(&self.state);
        let msg = TungsteniteMessage::Binary(data.to_vec().into());
        let data_len = data.len();

        {
            let s = state.lock().unwrap();
            if s.is_closed || s.ws_stream.is_none() {
                return Err(NetworkError::new_err("Not connected"));
            }
        }

        let ws = {
            let mut s = state.lock().unwrap();
            s.ws_stream.take()
        }
        .ok_or_else(|| NetworkError::new_err("Not connected"))?;

        crate::runtime_async::spawn_async(async move {
            let mut ws = ws;
            tokio::time::timeout(std::time::Duration::from_millis(5000), ws.send(msg))
                .await
                .map_err(|_| TimeoutError::new_err("Send timed out"))?
                .map_err(|e| NetworkError::new_err(format!("Send failed: {}", e)))?;

            {
                let mut s = state.lock().unwrap();
                s.ws_stream = Some(ws);
                s.bytes_sent += data_len as u64;
                s.message_count += 1;
            }
            Ok(())
        })
    }

    /// Receive a message from the WebSocket (async).
    fn async_recv(&self) -> PyResult<crate::runtime_async::PyFuture> {
        let state = Arc::clone(&self.state);
        let timeout = std::time::Duration::from_millis(self.config.timeout_ms);

        {
            let s = state.lock().unwrap();
            if s.is_closed || s.ws_stream.is_none() {
                return Err(NetworkError::new_err("Not connected"));
            }
        }

        let ws = {
            let mut s = state.lock().unwrap();
            s.ws_stream.take()
        }
        .ok_or_else(|| NetworkError::new_err("Not connected"))?;

        crate::runtime_async::spawn_async(async move {
            let mut ws = ws;
            let tungstenite_msg = tokio::time::timeout(timeout, ws.next())
                .await
                .map_err(|_| {
                    TimeoutError::new_err(format!(
                        "Receive timed out after {}ms",
                        timeout.as_millis()
                    ))
                })?
                .ok_or_else(|| NetworkError::new_err("Connection closed by remote"))?
                .map_err(|e| NetworkError::new_err(format!("Receive failed: {}", e)))?;

            let message = match tungstenite_msg {
                TungsteniteMessage::Text(text) => {
                    let text_str = text.to_string();
                    let data = text_str.as_bytes().to_vec();
                    let size = data.len();
                    {
                        let mut s = state.lock().unwrap();
                        s.ws_stream = Some(ws);
                        s.bytes_received += size as u64;
                        s.message_count += 1;
                    }
                    let mut msg = WebSocketMessagePy::from_wire("text", &data);
                    msg.text_content = Some(text_str);
                    msg
                }
                TungsteniteMessage::Binary(data) => {
                    let bytes: Vec<u8> = data.into();
                    let size = bytes.len();
                    {
                        let mut s = state.lock().unwrap();
                        s.ws_stream = Some(ws);
                        s.bytes_received += size as u64;
                        s.message_count += 1;
                    }
                    WebSocketMessagePy::from_wire("binary", &bytes)
                }
                TungsteniteMessage::Ping(data) => {
                    let bytes: Vec<u8> = data.into();
                    let size = bytes.len();
                    {
                        let mut s = state.lock().unwrap();
                        s.ws_stream = Some(ws);
                        s.bytes_received += size as u64;
                    }
                    WebSocketMessagePy::from_wire("ping", &bytes)
                }
                TungsteniteMessage::Pong(data) => {
                    let bytes: Vec<u8> = data.into();
                    let size = bytes.len();
                    {
                        let mut s = state.lock().unwrap();
                        s.ws_stream = Some(ws);
                        s.bytes_received += size as u64;
                    }
                    WebSocketMessagePy::from_wire("pong", &bytes)
                }
                TungsteniteMessage::Close(frame) => {
                    let mut s = state.lock().unwrap();
                    s.is_closed = true;
                    // Don't put the stream back - connection is closed
                    let close_info = frame.map(|f| WebSocketCloseInfoPy {
                        code: f.code.into(),
                        reason: f.reason.to_string(),
                        was_clean: true,
                    });
                    return Err(NetworkError::new_err(format!(
                        "Connection closed: {}",
                        close_info
                            .as_ref()
                            .map(|c| format!("{} {}", c.code, c.reason))
                            .unwrap_or_else(|| "no reason".to_string())
                    )));
                }
                TungsteniteMessage::Frame(_) => {
                    // Put the stream back
                    let mut s = state.lock().unwrap();
                    s.ws_stream = Some(ws);
                    return Err(NetworkError::new_err("Unexpected raw frame"));
                }
            };

            Ok(message)
        })
    }

    /// Send a ping frame (async).
    fn async_ping(&self, data: Option<&[u8]>) -> PyResult<crate::runtime_async::PyFuture> {
        let state = Arc::clone(&self.state);
        let ping_data = data.map(|d| d.to_vec()).unwrap_or_default();
        let msg = TungsteniteMessage::Ping(ping_data.into());

        {
            let s = state.lock().unwrap();
            if s.is_closed || s.ws_stream.is_none() {
                return Err(NetworkError::new_err("Not connected"));
            }
        }

        let ws = {
            let mut s = state.lock().unwrap();
            s.ws_stream.take()
        }
        .ok_or_else(|| NetworkError::new_err("Not connected"))?;

        crate::runtime_async::spawn_async(async move {
            let mut ws = ws;
            tokio::time::timeout(std::time::Duration::from_millis(5000), ws.send(msg))
                .await
                .map_err(|_| TimeoutError::new_err("Ping timed out"))?
                .map_err(|e| NetworkError::new_err(format!("Ping failed: {}", e)))?;

            // Put the stream back
            {
                let mut s = state.lock().unwrap();
                s.ws_stream = Some(ws);
            }
            Ok(())
        })
    }

    /// Close the WebSocket connection gracefully (async).
    fn async_close(
        &self,
        code: Option<u16>,
        reason: Option<&str>,
    ) -> PyResult<crate::runtime_async::PyFuture> {
        let state = Arc::clone(&self.state);
        let close_code = code.unwrap_or(1000);
        let close_reason = reason.unwrap_or("client close").to_string();

        let ws = {
            let mut s = state.lock().unwrap();
            if s.is_closed {
                // Idempotent: already closed returns a successful close info.
                return crate::runtime_async::spawn_async(async move {
                    Ok::<_, pyo3::PyErr>(WebSocketCloseInfoPy {
                        code: 1000,
                        reason: "already closed".to_string(),
                        was_clean: true,
                    })
                });
            }
            if s.ws_stream.is_none() {
                // No connection: mark closed and return idempotent close info.
                s.is_closed = true;
                return crate::runtime_async::spawn_async(async move {
                    Ok::<_, pyo3::PyErr>(WebSocketCloseInfoPy {
                        code: close_code,
                        reason: close_reason,
                        was_clean: true,
                    })
                });
            }
            s.ws_stream.take()
        }
        .ok_or_else(|| NetworkError::new_err("Not connected"))?;

        crate::runtime_async::spawn_async(async move {
            let mut ws = ws;
            let close_frame = CloseFrame {
                code: CloseCode::from(close_code),
                reason: close_reason.clone().into(),
            };
            let msg = TungsteniteMessage::Close(Some(close_frame));

            let result =
                tokio::time::timeout(std::time::Duration::from_millis(5000), ws.send(msg)).await;

            // Mark as closed - don't put the stream back
            {
                let mut s = state.lock().unwrap();
                s.is_closed = true;
            }

            match result {
                Ok(Ok(())) => Ok(WebSocketCloseInfoPy {
                    code: close_code,
                    reason: close_reason,
                    was_clean: true,
                }),
                _ => Ok(WebSocketCloseInfoPy {
                    code: close_code,
                    reason: close_reason,
                    was_clean: false,
                }),
            }
        })
    }

    fn __aenter__(slf: Py<Self>) -> Py<Self> {
        slf
    }

    fn __aexit__(
        &self,
        _exc_type: Option<&Bound<'_, PyAny>>,
        _exc_value: Option<&Bound<'_, PyAny>>,
        _traceback: Option<&Bound<'_, PyAny>>,
    ) -> bool {
        let state = Arc::clone(&self.state);
        {
            let mut s = state.lock().unwrap();
            if !s.is_closed {
                s.is_closed = true;
                s.ws_stream.take();
            }
        }
        false
    }

    fn __repr__(&self) -> String {
        let s = self.state.lock().unwrap();
        format!(
            "AsyncWebSocketSession(url={}, closed={}, sent={}, received={}, messages={})",
            self.config.url, s.is_closed, s.bytes_sent, s.bytes_received, s.message_count
        )
    }

    fn __str__(&self) -> String {
        let s = self.state.lock().unwrap();
        if s.is_closed {
            format!("ws://{} (closed)", self.config.url)
        } else {
            format!(
                "ws://{} (open, {} messages, sent={}B recv={}B)",
                self.config.url, s.message_count, s.bytes_sent, s.bytes_received
            )
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// Assessment Operation
// ═══════════════════════════════════════════════════════════════════

// ---------------------------------------------------------------------------
// WebSocketAssessmentConfigPy
// ---------------------------------------------------------------------------

/// Configuration for a comprehensive WebSocket security assessment.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketAssessmentConfigPy {
    #[pyo3(get)]
    pub url: String,
    #[pyo3(get)]
    pub timeout_ms: u64,
    #[pyo3(get)]
    pub test_connection: bool,
    #[pyo3(get)]
    pub test_origin_validation: bool,
    #[pyo3(get)]
    pub test_authentication: bool,
    #[pyo3(get)]
    pub test_subprotocol: bool,
    #[pyo3(get)]
    pub test_message_access: bool,
    #[pyo3(get)]
    pub test_close_behavior: bool,
}

#[pymethods]
impl WebSocketAssessmentConfigPy {
    #[new]
    #[pyo3(signature = (url, *, timeout_ms=30000, test_connection=true, test_origin_validation=true, test_authentication=true, test_subprotocol=true, test_message_access=true, test_close_behavior=true))]
    fn new(
        url: String,
        timeout_ms: u64,
        test_connection: bool,
        test_origin_validation: bool,
        test_authentication: bool,
        test_subprotocol: bool,
        test_message_access: bool,
        test_close_behavior: bool,
    ) -> PyResult<Self> {
        if url.is_empty() {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "url must not be empty",
            ));
        }
        Ok(Self {
            url,
            timeout_ms,
            test_connection,
            test_origin_validation,
            test_authentication,
            test_subprotocol,
            test_message_access,
            test_close_behavior,
        })
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("url", &self.url)?;
        dict.set_item("timeout_ms", self.timeout_ms)?;
        dict.set_item("test_connection", self.test_connection)?;
        dict.set_item("test_origin_validation", self.test_origin_validation)?;
        dict.set_item("test_authentication", self.test_authentication)?;
        dict.set_item("test_subprotocol", self.test_subprotocol)?;
        dict.set_item("test_message_access", self.test_message_access)?;
        dict.set_item("test_close_behavior", self.test_close_behavior)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "WebSocketAssessmentConfig(url={}, timeout_ms={})",
            self.url, self.timeout_ms
        )
    }
}

// ---------------------------------------------------------------------------
// WebSocketAssessmentResultPy
// ---------------------------------------------------------------------------

/// Result of a comprehensive WebSocket security assessment.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketAssessmentResultPy {
    #[pyo3(get)]
    pub target: String,
    handshake: Option<WebSocketHandshakePy>,
    origin_test: Option<OriginTestResultPy>,
    auth_test: Option<ConnectionTestResultPy>,
    subprotocol_test: Option<ConnectionTestResultPy>,
    message_test: Option<ConnectionTestResultPy>,
    close_test: Option<WebSocketCloseInfoPy>,
    findings: Vec<WebSocketFindingPy>,
    timing: ConnectionTimingPy,
}

#[pymethods]
impl WebSocketAssessmentResultPy {
    #[new]
    #[pyo3(signature = (
        target,
        *,
        handshake=None,
        origin_test=None,
        auth_test=None,
        subprotocol_test=None,
        message_test=None,
        close_test=None,
        findings=None,
        timing=None,
    ))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        target: String,
        handshake: Option<WebSocketHandshakePy>,
        origin_test: Option<OriginTestResultPy>,
        auth_test: Option<ConnectionTestResultPy>,
        subprotocol_test: Option<ConnectionTestResultPy>,
        message_test: Option<ConnectionTestResultPy>,
        close_test: Option<WebSocketCloseInfoPy>,
        findings: Option<Vec<WebSocketFindingPy>>,
        timing: Option<crate::network::ConnectionTimingPy>,
    ) -> Self {
        Self {
            target,
            handshake,
            origin_test,
            auth_test,
            subprotocol_test,
            message_test,
            close_test,
            findings: findings.unwrap_or_default(),
            timing: timing.unwrap_or(crate::network::ConnectionTimingPy {
                dns_resolution_ms: None,
                tcp_connect_ms: None,
                tls_handshake_ms: None,
                first_byte_ms: None,
                total_ms: 0.0,
                connection_reused: false,
            }),
        }
    }

    #[getter]
    fn handshake(&self) -> Option<WebSocketHandshakePy> {
        self.handshake.clone()
    }

    #[getter]
    fn origin_test(&self) -> Option<OriginTestResultPy> {
        self.origin_test.clone()
    }

    #[getter]
    fn auth_test(&self) -> Option<ConnectionTestResultPy> {
        self.auth_test.clone()
    }

    #[getter]
    fn subprotocol_test(&self) -> Option<ConnectionTestResultPy> {
        self.subprotocol_test.clone()
    }

    #[getter]
    fn message_test(&self) -> Option<ConnectionTestResultPy> {
        self.message_test.clone()
    }

    #[getter]
    fn close_test(&self) -> Option<WebSocketCloseInfoPy> {
        self.close_test.clone()
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

        if let Some(ref h) = self.handshake {
            dict.set_item("handshake", h.to_dict(py)?)?;
        } else {
            dict.set_item("handshake", py.None())?;
        }

        if let Some(ref o) = self.origin_test {
            dict.set_item("origin_test", o.to_dict(py)?)?;
        } else {
            dict.set_item("origin_test", py.None())?;
        }

        if let Some(ref a) = self.auth_test {
            dict.set_item("auth_test", a.to_dict(py)?)?;
        } else {
            dict.set_item("auth_test", py.None())?;
        }

        if let Some(ref s) = self.subprotocol_test {
            dict.set_item("subprotocol_test", s.to_dict(py)?)?;
        } else {
            dict.set_item("subprotocol_test", py.None())?;
        }

        if let Some(ref m) = self.message_test {
            dict.set_item("message_test", m.to_dict(py)?)?;
        } else {
            dict.set_item("message_test", py.None())?;
        }

        if let Some(ref c) = self.close_test {
            dict.set_item("close_test", c.to_dict(py)?)?;
        } else {
            dict.set_item("close_test", py.None())?;
        }

        let findings_list = PyList::empty_bound(py);
        for f in &self.findings {
            findings_list.append(f.to_dict(py)?)?;
        }
        dict.set_item("findings", findings_list)?;

        // Construct timing dict manually (ConnectionTimingPy::to_dict is private)
        let timing_dict = PyDict::new_bound(py);
        timing_dict.set_item("dns_resolution_ms", &self.timing.dns_resolution_ms)?;
        timing_dict.set_item("tcp_connect_ms", &self.timing.tcp_connect_ms)?;
        timing_dict.set_item("tls_handshake_ms", &self.timing.tls_handshake_ms)?;
        timing_dict.set_item("first_byte_ms", &self.timing.first_byte_ms)?;
        timing_dict.set_item("total_ms", self.timing.total_ms)?;
        timing_dict.set_item("connection_reused", self.timing.connection_reused)?;
        dict.set_item("timing", timing_dict)?;

        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "WebSocketAssessmentResult(target={}, findings={})",
            self.target,
            self.findings.len()
        )
    }

    fn __str__(&self) -> String {
        format!(
            "WebSocket assessment for {}: {} findings",
            self.target,
            self.findings.len()
        )
    }
}

// ---------------------------------------------------------------------------
// Internal assessment helpers
// ---------------------------------------------------------------------------

async fn assess_connection(
    url: &str,
    timeout_ms: u64,
) -> PyResult<(Option<WebSocketHandshakePy>, ConnectionTimingPy)> {
    let timeout = std::time::Duration::from_millis(timeout_ms);
    let connect_start = Instant::now();

    let mut request_builder = tokio_tungstenite::tungstenite::http::Request::builder();
    request_builder = request_builder.header("User-Agent", "eggsec/0.1");

    let request = request_builder
        .body(())
        .map_err(|e| NetworkError::new_err(format!("Failed to build request: {}", e)))?;

    let (ws_stream, response) =
        tokio::time::timeout(timeout, tokio_tungstenite::connect_async(request))
            .await
            .map_err(|_| {
                TimeoutError::new_err(format!(
                    "WebSocket connect timed out after {}ms",
                    timeout.as_millis()
                ))
            })?
            .map_err(|e| NetworkError::new_err(format!("WebSocket connect failed: {}", e)))?;

    let duration_ms = connect_start.elapsed().as_secs_f64() * 1000.0;
    let status_code = response.status().as_u16();

    let mut resp_headers = Vec::new();
    for (k, v) in response.headers().iter() {
        if let Ok(val) = v.to_str() {
            resp_headers.push((k.as_str().to_string(), val.to_string()));
        }
    }

    let selected_subprotocol = response
        .headers()
        .get("sec-websocket-protocol")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let selected_extensions: Vec<String> = response
        .headers()
        .get("sec-websocket-extensions")
        .and_then(|v| v.to_str().ok())
        .map(|s| {
            s.split(',')
                .map(|e| e.trim().to_string())
                .filter(|e| !e.is_empty())
                .collect()
        })
        .unwrap_or_default();

    let timing = ConnectionTimingPy {
        dns_resolution_ms: None,
        tcp_connect_ms: None,
        tls_handshake_ms: None,
        first_byte_ms: Some(duration_ms),
        total_ms: duration_ms,
        connection_reused: false,
    };

    // Close the connection immediately
    {
        let mut ws = ws_stream;
        let close_frame = CloseFrame {
            code: CloseCode::Normal,
            reason: "assessment complete".into(),
        };
        let _ = tokio::time::timeout(
            std::time::Duration::from_millis(2000),
            ws.send(TungsteniteMessage::Close(Some(close_frame))),
        )
        .await;
    }

    let handshake = WebSocketHandshakePy {
        url: url.to_string(),
        status_code,
        headers: resp_headers,
        selected_subprotocol,
        selected_extensions,
        duration_ms,
    };

    Ok((Some(handshake), timing))
}

async fn test_origin(
    url: &str,
    malicious_origin: &str,
    timeout_ms: u64,
) -> PyResult<OriginTestResultPy> {
    let timeout = std::time::Duration::from_millis(timeout_ms);

    let mut request_builder = tokio_tungstenite::tungstenite::http::Request::builder();
    request_builder = request_builder.header("Origin", malicious_origin);

    let request = request_builder
        .body(())
        .map_err(|e| NetworkError::new_err(format!("Failed to build request: {}", e)))?;

    let result = tokio::time::timeout(timeout, tokio_tungstenite::connect_async(request)).await;

    match result {
        Ok(Ok((mut ws, response))) => {
            let status = response.status().as_u16();
            let accepted = status == 101;
            let _ = tokio::time::timeout(
                std::time::Duration::from_millis(1000),
                ws.send(TungsteniteMessage::Close(Some(CloseFrame {
                    code: CloseCode::Normal,
                    reason: "test complete".into(),
                }))),
            )
            .await;

            let details = if accepted {
                "Server accepted WebSocket connection with malicious origin — potential CSWSH vulnerability"
                    .to_string()
            } else {
                format!("Server rejected connection with status {}", status)
            };

            Ok(OriginTestResultPy {
                origin: malicious_origin.to_string(),
                accepted,
                status_code: Some(status),
                details,
            })
        }
        Ok(Err(_)) => Ok(OriginTestResultPy {
            origin: malicious_origin.to_string(),
            accepted: false,
            status_code: None,
            details: "Connection rejected".to_string(),
        }),
        Err(_) => Ok(OriginTestResultPy {
            origin: malicious_origin.to_string(),
            accepted: false,
            status_code: None,
            details: "Connection timed out".to_string(),
        }),
    }
}

async fn test_message_access(url: &str, timeout_ms: u64) -> PyResult<ConnectionTestResultPy> {
    let timeout = std::time::Duration::from_millis(timeout_ms);
    let connect_start = Instant::now();

    let mut request_builder = tokio_tungstenite::tungstenite::http::Request::builder();
    request_builder = request_builder.header("User-Agent", "eggsec/0.1");

    let request = request_builder
        .body(())
        .map_err(|e| NetworkError::new_err(format!("Failed to build request: {}", e)))?;

    let connect_result =
        tokio::time::timeout(timeout, tokio_tungstenite::connect_async(request)).await;

    match connect_result {
        Ok(Ok((mut ws, _response))) => {
            let latency_ms = connect_start.elapsed().as_secs_f64() * 1000.0;

            // Try sending a test message
            let send_result = tokio::time::timeout(
                std::time::Duration::from_millis(2000),
                ws.send(TungsteniteMessage::Text("eggsec-test".to_string().into())),
            )
            .await;

            let connected = send_result.is_ok();

            let _ = tokio::time::timeout(
                std::time::Duration::from_millis(1000),
                ws.send(TungsteniteMessage::Close(Some(CloseFrame {
                    code: CloseCode::Normal,
                    reason: "test complete".into(),
                }))),
            )
            .await;

            Ok(ConnectionTestResultPy {
                url: url.to_string(),
                connected,
                response_headers: Vec::new(),
                subprotocols: Vec::new(),
                extensions: Vec::new(),
                latency_ms: Some(latency_ms),
                error: if connected {
                    None
                } else {
                    Some("Failed to send test message".to_string())
                },
            })
        }
        Ok(Err(e)) => Ok(ConnectionTestResultPy {
            url: url.to_string(),
            connected: false,
            response_headers: Vec::new(),
            subprotocols: Vec::new(),
            extensions: Vec::new(),
            latency_ms: None,
            error: Some(format!("Connect failed: {}", e)),
        }),
        Err(_) => Ok(ConnectionTestResultPy {
            url: url.to_string(),
            connected: false,
            response_headers: Vec::new(),
            subprotocols: Vec::new(),
            extensions: Vec::new(),
            latency_ms: None,
            error: Some("Connection timed out".to_string()),
        }),
    }
}

async fn test_close_behavior(url: &str, timeout_ms: u64) -> PyResult<WebSocketCloseInfoPy> {
    let timeout = std::time::Duration::from_millis(timeout_ms);

    let mut request_builder = tokio_tungstenite::tungstenite::http::Request::builder();
    request_builder = request_builder.header("User-Agent", "eggsec/0.1");

    let request = request_builder
        .body(())
        .map_err(|e| NetworkError::new_err(format!("Failed to build request: {}", e)))?;

    let connect_result =
        tokio::time::timeout(timeout, tokio_tungstenite::connect_async(request)).await;

    match connect_result {
        Ok(Ok((mut ws, _response))) => {
            let close_frame = CloseFrame {
                code: CloseCode::Normal,
                reason: "assessment close test".into(),
            };

            let send_result = tokio::time::timeout(
                std::time::Duration::from_millis(2000),
                ws.send(TungsteniteMessage::Close(Some(close_frame))),
            )
            .await;

            match send_result {
                Ok(Ok(())) => {
                    // Wait for close response
                    let close_response =
                        tokio::time::timeout(std::time::Duration::from_millis(2000), ws.next())
                            .await;

                    match close_response {
                        Ok(Some(Ok(TungsteniteMessage::Close(frame)))) => {
                            if let Some(f) = frame {
                                Ok(WebSocketCloseInfoPy {
                                    code: f.code.into(),
                                    reason: f.reason.to_string(),
                                    was_clean: true,
                                })
                            } else {
                                Ok(WebSocketCloseInfoPy {
                                    code: 1000,
                                    reason: String::new(),
                                    was_clean: true,
                                })
                            }
                        }
                        _ => Ok(WebSocketCloseInfoPy {
                            code: 0,
                            reason: "No close frame received".to_string(),
                            was_clean: false,
                        }),
                    }
                }
                _ => Ok(WebSocketCloseInfoPy {
                    code: 0,
                    reason: "Failed to send close frame".to_string(),
                    was_clean: false,
                }),
            }
        }
        _ => Ok(WebSocketCloseInfoPy {
            code: 0,
            reason: "Connection failed".to_string(),
            was_clean: false,
        }),
    }
}

// ---------------------------------------------------------------------------
// Assessment functions
// ---------------------------------------------------------------------------

async fn run_assessment(url: &str, timeout_ms: u64) -> PyResult<WebSocketAssessmentResultPy> {
    let mut findings = Vec::new();

    // Test connection and handshake
    let (handshake, timing) = match assess_connection(url, timeout_ms).await {
        Ok(result) => result,
        Err(e) => {
            findings.push(WebSocketFindingPy {
                category: "connection".to_string(),
                severity: Severity::High,
                title: "WebSocket connection failed".to_string(),
                description: format!("Failed to establish WebSocket connection: {}", e),
                recommendation:
                    "Verify the WebSocket endpoint is available and accepting connections"
                        .to_string(),
            });
            return Ok(WebSocketAssessmentResultPy {
                target: url.to_string(),
                handshake: None,
                origin_test: None,
                auth_test: None,
                subprotocol_test: None,
                message_test: None,
                close_test: None,
                findings,
                timing: ConnectionTimingPy {
                    dns_resolution_ms: None,
                    tcp_connect_ms: None,
                    tls_handshake_ms: None,
                    first_byte_ms: None,
                    total_ms: 0.0,
                    connection_reused: false,
                },
            });
        }
    };

    // Test origin validation
    let origin_test = test_origin(url, "https://evil.example.com", timeout_ms)
        .await
        .ok();

    if let Some(ref ot) = origin_test {
        if ot.accepted {
            findings.push(WebSocketFindingPy {
                category: "origin-validation".to_string(),
                severity: Severity::High,
                title: "Cross-Site WebSocket Hijacking (CSWSH)".to_string(),
                description: format!(
                    "Server accepted WebSocket connection with origin '{}'",
                    ot.origin
                ),
                recommendation:
                    "Validate Origin header on the server and reject connections from untrusted origins"
                        .to_string(),
            });
        }
    }

    // Test message access
    let message_test = test_message_access(url, timeout_ms).await.ok();

    // Test close behavior
    let close_test = test_close_behavior(url, timeout_ms).await.ok();

    if let Some(ref ct) = close_test {
        if !ct.was_clean {
            findings.push(WebSocketFindingPy {
                category: "close-behavior".to_string(),
                severity: Severity::Low,
                title: "Unclean WebSocket close".to_string(),
                description:
                    "Server did not send a close frame in response to a clean close request"
                        .to_string(),
                recommendation:
                    "Ensure the server properly handles WebSocket close frames per RFC 6455"
                        .to_string(),
            });
        }
    }

    Ok(WebSocketAssessmentResultPy {
        target: url.to_string(),
        handshake,
        origin_test,
        auth_test: None,
        subprotocol_test: None,
        message_test,
        close_test,
        findings,
        timing,
    })
}

/// Run a comprehensive WebSocket security assessment.
///
/// Args:
///     url: WebSocket URL to assess (e.g. "ws://example.com/ws").
///     timeout_ms: Assessment timeout in milliseconds (default: 30000).
///
/// Returns:
///     WebSocketAssessmentResultPy: Assessment results with findings.
///
/// Raises:
///     NetworkError: If the connection fails.
///     ConfigError: If the URL is invalid.
#[pyfunction]
#[pyo3(signature = (url, timeout_ms=30000))]
pub fn websocket_assess(url: &str, timeout_ms: u64) -> PyResult<WebSocketAssessmentResultPy> {
    Python::with_gil(|py| {
        let url_owned = url.to_string();
        runtime_sync::block_on(
            py,
            async move { run_assessment(&url_owned, timeout_ms).await },
        )
    })
}

/// Run a comprehensive WebSocket security assessment (async).
///
/// Args:
///     url: WebSocket URL to assess (e.g. "ws://example.com/ws").
///     timeout_ms: Assessment timeout in milliseconds (default: 30000).
///
/// Returns:
///     PyFuture that resolves to WebSocketAssessmentResultPy.
#[pyfunction]
#[pyo3(signature = (url, timeout_ms=30000))]
pub fn async_websocket_assess(
    url: &str,
    timeout_ms: u64,
) -> PyResult<crate::runtime_async::PyFuture> {
    let url_owned = url.to_string();

    crate::runtime_async::spawn_async(async move { run_assessment(&url_owned, timeout_ms).await })
}
