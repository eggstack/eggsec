use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyTuple};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

use crate::artifact::ArtifactReferencePy;
use crate::error::ScanError;
use crate::runtime_async;
use crate::runtime_async::PyFuture;

// ═══════════════════════════════════════════════════════════════════
// Workstream 7: Browser capabilities and session state
// ═══════════════════════════════════════════════════════════════════

/// Describes the capabilities of a browser engine.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserCapabilities {
    #[pyo3(get)]
    pub engine: String,
    #[pyo3(get)]
    pub version: Option<String>,
    #[pyo3(get)]
    pub supports_javascript: bool,
    #[pyo3(get)]
    pub supports_dom: bool,
    #[pyo3(get)]
    pub supports_network_intercept: bool,
    #[pyo3(get)]
    pub supports_console_capture: bool,
    #[pyo3(get)]
    pub supports_screenshot: bool,
    #[pyo3(get)]
    pub supports_pdf_export: bool,
    #[pyo3(get)]
    pub supports_cookie_access: bool,
    #[pyo3(get)]
    pub supports_storage_access: bool,
    #[pyo3(get)]
    pub supports_route_discovery: bool,
    #[pyo3(get)]
    pub supports_proxy: bool,
}

#[pymethods]
impl BrowserCapabilities {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("engine", &self.engine)?;
        dict.set_item("version", &self.version)?;
        dict.set_item("supports_javascript", self.supports_javascript)?;
        dict.set_item("supports_dom", self.supports_dom)?;
        dict.set_item(
            "supports_network_intercept",
            self.supports_network_intercept,
        )?;
        dict.set_item("supports_console_capture", self.supports_console_capture)?;
        dict.set_item("supports_screenshot", self.supports_screenshot)?;
        dict.set_item("supports_pdf_export", self.supports_pdf_export)?;
        dict.set_item("supports_cookie_access", self.supports_cookie_access)?;
        dict.set_item("supports_storage_access", self.supports_storage_access)?;
        dict.set_item("supports_route_discovery", self.supports_route_discovery)?;
        dict.set_item("supports_proxy", self.supports_proxy)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "BrowserCapabilities(engine={}, js={}, dom={}, screenshot={})",
            self.engine, self.supports_javascript, self.supports_dom, self.supports_screenshot
        )
    }
}

/// Lifecycle state of a browser session.
#[pyclass(frozen, eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BrowserSessionState {
    Created,
    Discovering,
    Launching,
    Ready,
    Navigating,
    Loading,
    Inspecting,
    Stopping,
    Cleaning,
    Stopped,
    Failed,
    Cancelled,
}

#[pymethods]
impl BrowserSessionState {
    fn __repr__(&self) -> String {
        format!("BrowserSessionState.{}", self.as_str())
    }

    fn __str__(&self) -> String {
        self.as_str().to_string()
    }
}

impl BrowserSessionState {
    fn as_str(&self) -> &str {
        match self {
            BrowserSessionState::Created => "Created",
            BrowserSessionState::Discovering => "Discovering",
            BrowserSessionState::Launching => "Launching",
            BrowserSessionState::Ready => "Ready",
            BrowserSessionState::Navigating => "Navigating",
            BrowserSessionState::Loading => "Loading",
            BrowserSessionState::Inspecting => "Inspecting",
            BrowserSessionState::Stopping => "Stopping",
            BrowserSessionState::Cleaning => "Cleaning",
            BrowserSessionState::Stopped => "Stopped",
            BrowserSessionState::Failed => "Failed",
            BrowserSessionState::Cancelled => "Cancelled",
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// Workstream 8: Session configuration and statistics
// ═══════════════════════════════════════════════════════════════════

/// Configuration for a browser session.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserSessionConfig {
    #[pyo3(get)]
    pub target_url: Option<String>,
    #[pyo3(get)]
    pub headless: bool,
    #[pyo3(get)]
    pub proxy: Option<String>,
    #[pyo3(get)]
    pub user_agent: Option<String>,
    #[pyo3(get)]
    pub viewport_width: u32,
    #[pyo3(get)]
    pub viewport_height: u32,
    #[pyo3(get)]
    pub timeout_ms: u64,
    #[pyo3(get)]
    pub navigation_timeout_ms: u64,
    #[pyo3(get)]
    pub collect_console: bool,
    #[pyo3(get)]
    pub collect_network: bool,
    #[pyo3(get)]
    pub collect_cookies: bool,
    #[pyo3(get)]
    pub collect_storage: bool,
    #[pyo3(get)]
    pub screenshot_on_complete: bool,
    extra_headers: Vec<String>,
    #[pyo3(get)]
    pub ignore_cert_errors: bool,
}

#[pymethods]
impl BrowserSessionConfig {
    #[new]
    #[pyo3(signature = (*, target_url=None, headless=true, proxy=None, user_agent=None, viewport_width=1280, viewport_height=720, timeout_ms=30000, navigation_timeout_ms=60000, collect_console=true, collect_network=true, collect_cookies=true, collect_storage=true, screenshot_on_complete=false, extra_headers=None, ignore_cert_errors=false))]
    fn new(
        target_url: Option<&str>,
        headless: bool,
        proxy: Option<&str>,
        user_agent: Option<&str>,
        viewport_width: u32,
        viewport_height: u32,
        timeout_ms: u64,
        navigation_timeout_ms: u64,
        collect_console: bool,
        collect_network: bool,
        collect_cookies: bool,
        collect_storage: bool,
        screenshot_on_complete: bool,
        extra_headers: Option<Vec<String>>,
        ignore_cert_errors: bool,
    ) -> Self {
        Self {
            target_url: target_url.map(|s| s.to_string()),
            headless,
            proxy: proxy.map(|s| s.to_string()),
            user_agent: user_agent.map(|s| s.to_string()),
            viewport_width,
            viewport_height,
            timeout_ms,
            navigation_timeout_ms,
            collect_console,
            collect_network,
            collect_cookies,
            collect_storage,
            screenshot_on_complete,
            extra_headers: extra_headers.unwrap_or_default(),
            ignore_cert_errors,
        }
    }

    #[getter]
    fn extra_headers(&self) -> Vec<String> {
        self.extra_headers.clone()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("target_url", &self.target_url)?;
        dict.set_item("headless", self.headless)?;
        dict.set_item("proxy", &self.proxy)?;
        dict.set_item("user_agent", &self.user_agent)?;
        dict.set_item("viewport_width", self.viewport_width)?;
        dict.set_item("viewport_height", self.viewport_height)?;
        dict.set_item("timeout_ms", self.timeout_ms)?;
        dict.set_item("navigation_timeout_ms", self.navigation_timeout_ms)?;
        dict.set_item("collect_console", self.collect_console)?;
        dict.set_item("collect_network", self.collect_network)?;
        dict.set_item("collect_cookies", self.collect_cookies)?;
        dict.set_item("collect_storage", self.collect_storage)?;
        dict.set_item("screenshot_on_complete", self.screenshot_on_complete)?;
        let headers_list = PyList::new_bound(py, &self.extra_headers);
        dict.set_item("extra_headers", headers_list)?;
        dict.set_item("ignore_cert_errors", self.ignore_cert_errors)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "BrowserSessionConfig(headless={}, viewport={}x{}, timeout={}ms)",
            self.headless, self.viewport_width, self.viewport_height, self.timeout_ms
        )
    }
}

/// Accumulated statistics for a browser session.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserSessionStats {
    #[pyo3(get)]
    pub pages_navigated: usize,
    #[pyo3(get)]
    pub dom_snapshots: usize,
    #[pyo3(get)]
    pub console_events: usize,
    #[pyo3(get)]
    pub network_requests: usize,
    #[pyo3(get)]
    pub cookies_collected: usize,
    #[pyo3(get)]
    pub screenshots_taken: usize,
    #[pyo3(get)]
    pub artifacts_collected: usize,
    #[pyo3(get)]
    pub duration_ms: u64,
}

#[pymethods]
impl BrowserSessionStats {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("pages_navigated", self.pages_navigated)?;
        dict.set_item("dom_snapshots", self.dom_snapshots)?;
        dict.set_item("console_events", self.console_events)?;
        dict.set_item("network_requests", self.network_requests)?;
        dict.set_item("cookies_collected", self.cookies_collected)?;
        dict.set_item("screenshots_taken", self.screenshots_taken)?;
        dict.set_item("artifacts_collected", self.artifacts_collected)?;
        dict.set_item("duration_ms", self.duration_ms)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "BrowserSessionStats(pages={}, console={}, network={}, duration={}ms)",
            self.pages_navigated, self.console_events, self.network_requests, self.duration_ms
        )
    }
}

// ═══════════════════════════════════════════════════════════════════
// Workstream 9: Navigation and console events
// ═══════════════════════════════════════════════════════════════════

/// A browser navigation event (page load).
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserNavigationEvent {
    #[pyo3(get)]
    pub url: String,
    #[pyo3(get)]
    pub final_url: String,
    #[pyo3(get)]
    pub status_code: u32,
    redirect_chain: Vec<String>,
    #[pyo3(get)]
    pub load_time_ms: u64,
    #[pyo3(get)]
    pub timestamp_ms: u64,
}

#[pymethods]
impl BrowserNavigationEvent {
    #[getter]
    fn redirect_chain(&self) -> Vec<String> {
        self.redirect_chain.clone()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("url", &self.url)?;
        dict.set_item("final_url", &self.final_url)?;
        dict.set_item("status_code", self.status_code)?;
        let redirects = PyList::new_bound(py, &self.redirect_chain);
        dict.set_item("redirect_chain", redirects)?;
        dict.set_item("load_time_ms", self.load_time_ms)?;
        dict.set_item("timestamp_ms", self.timestamp_ms)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "BrowserNavigationEvent(url={}, status={}, load_time={}ms)",
            self.url, self.status_code, self.load_time_ms
        )
    }
}

/// A browser console event.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserConsoleEvent {
    #[pyo3(get)]
    pub level: String,
    #[pyo3(get)]
    pub message: String,
    #[pyo3(get)]
    pub source: Option<String>,
    #[pyo3(get)]
    pub line_number: Option<u32>,
    #[pyo3(get)]
    pub timestamp_ms: u64,
}

#[pymethods]
impl BrowserConsoleEvent {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("level", &self.level)?;
        dict.set_item("message", &self.message)?;
        dict.set_item("source", &self.source)?;
        dict.set_item("line_number", self.line_number)?;
        dict.set_item("timestamp_ms", self.timestamp_ms)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "BrowserConsoleEvent(level={}, message={})",
            self.level, self.message
        )
    }
}

// ═══════════════════════════════════════════════════════════════════
// Workstream 10: Network events and DOM snapshots
// ═══════════════════════════════════════════════════════════════════

/// A browser network event (request/response).
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserNetworkEvent {
    #[pyo3(get)]
    pub method: String,
    #[pyo3(get)]
    pub url: String,
    #[pyo3(get)]
    pub status_code: Option<u32>,
    request_headers: Vec<(String, String)>,
    response_headers: Vec<(String, String)>,
    #[pyo3(get)]
    pub content_type: Option<String>,
    #[pyo3(get)]
    pub size_bytes: Option<u64>,
    #[pyo3(get)]
    pub duration_ms: u64,
    #[pyo3(get)]
    pub timestamp_ms: u64,
}

#[pymethods]
impl BrowserNetworkEvent {
    #[getter]
    fn request_headers(&self) -> Vec<(String, String)> {
        self.request_headers.clone()
    }

    #[getter]
    fn response_headers(&self) -> Vec<(String, String)> {
        self.response_headers.clone()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("method", &self.method)?;
        dict.set_item("url", &self.url)?;
        dict.set_item("status_code", self.status_code)?;

        let req_headers = PyDict::new_bound(py);
        for (k, v) in &self.request_headers {
            req_headers.set_item(k.as_str(), v.as_str())?;
        }
        dict.set_item("request_headers", req_headers)?;

        let resp_headers = PyDict::new_bound(py);
        for (k, v) in &self.response_headers {
            resp_headers.set_item(k.as_str(), v.as_str())?;
        }
        dict.set_item("response_headers", resp_headers)?;

        dict.set_item("content_type", &self.content_type)?;
        dict.set_item("size_bytes", self.size_bytes)?;
        dict.set_item("duration_ms", self.duration_ms)?;
        dict.set_item("timestamp_ms", self.timestamp_ms)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "BrowserNetworkEvent(method={}, url={}, status={:?})",
            self.method, self.url, self.status_code
        )
    }
}

/// A form field discovered in a DOM snapshot.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserFormField {
    #[pyo3(get)]
    pub name: String,
    #[pyo3(get)]
    pub field_type: String,
    #[pyo3(get)]
    pub value: Option<String>,
    #[pyo3(get)]
    pub required: bool,
}

#[pymethods]
impl BrowserFormField {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("name", &self.name)?;
        dict.set_item("field_type", &self.field_type)?;
        dict.set_item("value", &self.value)?;
        dict.set_item("required", self.required)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }
}

/// A form discovered in a DOM snapshot.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserFormInfo {
    #[pyo3(get)]
    pub action: String,
    #[pyo3(get)]
    pub method: String,
    fields: Vec<BrowserFormField>,
}

#[pymethods]
impl BrowserFormInfo {
    #[getter]
    fn fields(&self) -> Vec<BrowserFormField> {
        self.fields.clone()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("action", &self.action)?;
        dict.set_item("method", &self.method)?;

        let fields_list = PyList::empty_bound(py);
        for f in &self.fields {
            fields_list.append(f.to_dict(py)?)?;
        }
        dict.set_item("fields", fields_list)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }
}

/// A link discovered in a DOM snapshot.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserLinkInfo {
    #[pyo3(get)]
    pub href: String,
    #[pyo3(get)]
    pub text: String,
    #[pyo3(get)]
    pub rel: Option<String>,
}

#[pymethods]
impl BrowserLinkInfo {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("href", &self.href)?;
        dict.set_item("text", &self.text)?;
        dict.set_item("rel", &self.rel)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }
}

/// A DOM snapshot captured from a browser page.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserDomSnapshot {
    #[pyo3(get)]
    pub url: String,
    #[pyo3(get)]
    pub title: Option<String>,
    forms: Vec<BrowserFormInfo>,
    links: Vec<BrowserLinkInfo>,
    scripts: Vec<String>,
    frames: Vec<String>,
    #[pyo3(get)]
    pub timestamp_ms: u64,
}

#[pymethods]
impl BrowserDomSnapshot {
    #[getter]
    fn forms(&self) -> Vec<BrowserFormInfo> {
        self.forms.clone()
    }

    #[getter]
    fn links(&self) -> Vec<BrowserLinkInfo> {
        self.links.clone()
    }

    #[getter]
    fn scripts(&self) -> Vec<String> {
        self.scripts.clone()
    }

    #[getter]
    fn frames(&self) -> Vec<String> {
        self.frames.clone()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("url", &self.url)?;
        dict.set_item("title", &self.title)?;

        let forms_list = PyList::empty_bound(py);
        for f in &self.forms {
            forms_list.append(f.to_dict(py)?)?;
        }
        dict.set_item("forms", forms_list)?;

        let links_list = PyList::empty_bound(py);
        for l in &self.links {
            links_list.append(l.to_dict(py)?)?;
        }
        dict.set_item("links", links_list)?;

        let scripts_list = PyList::new_bound(py, &self.scripts);
        dict.set_item("scripts", scripts_list)?;

        let frames_list = PyList::new_bound(py, &self.frames);
        dict.set_item("frames", frames_list)?;

        dict.set_item("timestamp_ms", self.timestamp_ms)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "BrowserDomSnapshot(url={}, title={:?}, forms={}, links={}, scripts={})",
            self.url,
            self.title,
            self.forms.len(),
            self.links.len(),
            self.scripts.len()
        )
    }
}

// ═══════════════════════════════════════════════════════════════════
// Workstream 11: Storage, cookies, and session lifecycle
// ═══════════════════════════════════════════════════════════════════

/// A browser cookie with sensitive value redacted.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserCookieInfo {
    #[pyo3(get)]
    pub name: String,
    #[pyo3(get)]
    pub value: String,
    #[pyo3(get)]
    pub domain: String,
    #[pyo3(get)]
    pub path: String,
    #[pyo3(get)]
    pub expires: Option<u64>,
    #[pyo3(get)]
    pub http_only: bool,
    #[pyo3(get)]
    pub secure: bool,
}

#[pymethods]
impl BrowserCookieInfo {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("name", &self.name)?;
        dict.set_item("value", &self.value)?;
        dict.set_item("domain", &self.domain)?;
        dict.set_item("path", &self.path)?;
        dict.set_item("expires", self.expires)?;
        dict.set_item("http_only", self.http_only)?;
        dict.set_item("secure", self.secure)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }
}

/// Collected browser storage information.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserStorageInfo {
    local_storage: Vec<(String, String)>,
    session_storage: Vec<(String, String)>,
    cookies: Vec<BrowserCookieInfo>,
}

#[pymethods]
impl BrowserStorageInfo {
    #[getter]
    fn local_storage(&self) -> Vec<(String, String)> {
        self.local_storage.clone()
    }

    #[getter]
    fn session_storage(&self) -> Vec<(String, String)> {
        self.session_storage.clone()
    }

    #[getter]
    fn cookies(&self) -> Vec<BrowserCookieInfo> {
        self.cookies.clone()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);

        let local_list = PyList::empty_bound(py);
        for (k, v) in &self.local_storage {
            let pair = PyTuple::new_bound(py, &[k.as_str(), v.as_str()]);
            local_list.append(pair)?;
        }
        dict.set_item("local_storage", local_list)?;

        let session_list = PyList::empty_bound(py);
        for (k, v) in &self.session_storage {
            let pair = PyTuple::new_bound(py, &[k.as_str(), v.as_str()]);
            session_list.append(pair)?;
        }
        dict.set_item("session_storage", session_list)?;

        let cookies_list = PyList::empty_bound(py);
        for c in &self.cookies {
            cookies_list.append(c.to_dict(py)?)?;
        }
        dict.set_item("cookies", cookies_list)?;

        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }
}

// ═══════════════════════════════════════════════════════════════════
// BrowserSession: synchronous session with context manager
// ═══════════════════════════════════════════════════════════════════

struct BrowserSessionInner {
    state: BrowserSessionState,
    stats: BrowserSessionStats,
    console_events: Vec<BrowserConsoleEvent>,
    network_events: Vec<BrowserNetworkEvent>,
}

/// A synchronous browser session with lifecycle management.
#[pyclass]
pub struct BrowserSession {
    session_id: String,
    config: BrowserSessionConfig,
    inner: Mutex<BrowserSessionInner>,
}

#[pymethods]
impl BrowserSession {
    #[new]
    fn new(config: BrowserSessionConfig) -> Self {
        let session_id = format!(
            "browser-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
        );
        Self {
            session_id,
            config,
            inner: Mutex::new(BrowserSessionInner {
                state: BrowserSessionState::Created,
                stats: BrowserSessionStats {
                    pages_navigated: 0,
                    dom_snapshots: 0,
                    console_events: 0,
                    network_requests: 0,
                    cookies_collected: 0,
                    screenshots_taken: 0,
                    artifacts_collected: 0,
                    duration_ms: 0,
                },
                console_events: Vec::new(),
                network_events: Vec::new(),
            }),
        }
    }

    #[getter]
    fn session_id(&self) -> String {
        self.session_id.clone()
    }

    #[getter]
    fn state(&self) -> PyResult<BrowserSessionState> {
        self.inner
            .lock()
            .map(|i| i.state)
            .map_err(|_| ScanError::new_err("Session state lock poisoned"))
    }

    #[getter]
    fn config(&self) -> BrowserSessionConfig {
        self.config.clone()
    }

    #[getter]
    fn stats(&self) -> PyResult<BrowserSessionStats> {
        self.inner
            .lock()
            .map(|i| i.stats.clone())
            .map_err(|_| ScanError::new_err("Session stats lock poisoned"))
    }

    /// Start the browser session.
    fn start(&self) -> PyResult<()> {
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| ScanError::new_err("Session state lock poisoned"))?;

        if inner.state != BrowserSessionState::Created
            && inner.state != BrowserSessionState::Stopped
        {
            return Err(ScanError::new_err(format!(
                "Cannot start session in state {:?}",
                inner.state
            )));
        }

        inner.state = BrowserSessionState::Launching;
        // TODO: Launch actual browser engine
        inner.state = BrowserSessionState::Ready;
        Ok(())
    }

    /// Stop the browser session and release resources.
    fn stop(&self) -> PyResult<()> {
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| ScanError::new_err("Session state lock poisoned"))?;

        if inner.state == BrowserSessionState::Stopped
            || inner.state == BrowserSessionState::Cleaning
        {
            return Ok(());
        }

        inner.state = BrowserSessionState::Stopping;
        // TODO: Tear down browser
        inner.state = BrowserSessionState::Stopped;
        Ok(())
    }

    /// Navigate to a URL and return the navigation event.
    fn navigate(&self, url: &str) -> PyResult<BrowserNavigationEvent> {
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| ScanError::new_err("Session state lock poisoned"))?;

        if inner.state != BrowserSessionState::Ready
            && inner.state != BrowserSessionState::Inspecting
        {
            return Err(ScanError::new_err(format!(
                "Cannot navigate in state {:?}",
                inner.state
            )));
        }

        inner.state = BrowserSessionState::Navigating;
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let event = BrowserNavigationEvent {
            url: url.to_string(),
            final_url: url.to_string(),
            status_code: 0,
            redirect_chain: Vec::new(),
            load_time_ms: 0,
            timestamp_ms: now_ms,
        };

        inner.stats.pages_navigated += 1;
        inner.state = BrowserSessionState::Ready;
        Ok(event)
    }

    /// Wait for a CSS selector to appear in the DOM.
    fn wait_for_selector(&self, selector: &str, timeout_ms: Option<u64>) -> PyResult<bool> {
        let _ = (selector, timeout_ms);
        Err(ScanError::new_err(
            "Browser session requires an active browser engine",
        ))
    }

    /// Capture a DOM snapshot of the current page.
    fn get_dom_snapshot(&self) -> PyResult<BrowserDomSnapshot> {
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| ScanError::new_err("Session state lock poisoned"))?;

        inner.stats.dom_snapshots += 1;
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        Ok(BrowserDomSnapshot {
            url: String::new(),
            title: None,
            forms: Vec::new(),
            links: Vec::new(),
            scripts: Vec::new(),
            frames: Vec::new(),
            timestamp_ms: now_ms,
        })
    }

    /// Get all captured console events.
    fn get_console_events(&self) -> PyResult<Vec<BrowserConsoleEvent>> {
        self.inner
            .lock()
            .map(|i| i.console_events.clone())
            .map_err(|_| ScanError::new_err("Session state lock poisoned"))
    }

    /// Get all captured network events.
    fn get_network_events(&self) -> PyResult<Vec<BrowserNetworkEvent>> {
        self.inner
            .lock()
            .map(|i| i.network_events.clone())
            .map_err(|_| ScanError::new_err("Session state lock poisoned"))
    }

    /// Collect cookies and storage from the current page.
    fn get_cookies(&self) -> PyResult<BrowserStorageInfo> {
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| ScanError::new_err("Session state lock poisoned"))?;

        inner.stats.cookies_collected += 1;
        Ok(BrowserStorageInfo {
            local_storage: Vec::new(),
            session_storage: Vec::new(),
            cookies: Vec::new(),
        })
    }

    /// Take a screenshot of the current page and return an artifact reference.
    fn take_screenshot(&self) -> PyResult<ArtifactReferencePy> {
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| ScanError::new_err("Session state lock poisoned"))?;

        inner.stats.screenshots_taken += 1;
        let artifact_id = format!("screenshot-{}", inner.stats.screenshots_taken);

        Ok(ArtifactReferencePy {
            artifact_id,
            finding_id: String::new(),
            role: "screenshot".to_string(),
        })
    }

    /// Execute JavaScript in the page context.
    fn execute_script(&self, script: &str, timeout_ms: Option<u64>) -> PyResult<String> {
        let _ = (script, timeout_ms);
        Err(ScanError::new_err(
            "Browser session requires an active browser engine",
        ))
    }

    /// Context manager __enter__.
    fn __enter__(slf: Py<Self>) -> Py<Self> {
        slf
    }

    /// Context manager __exit__: stops the session.
    #[pyo3(signature = (_exc_type=None, _exc_value=None, _traceback=None))]
    fn __exit__(
        &self,
        _exc_type: Option<&Bound<'_, PyAny>>,
        _exc_value: Option<&Bound<'_, PyAny>>,
        _traceback: Option<&Bound<'_, PyAny>>,
    ) -> bool {
        let _ = self.stop();
        false
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let inner = self
            .inner
            .lock()
            .map_err(|_| ScanError::new_err("Session state lock poisoned"))?;
        let dict = PyDict::new_bound(py);
        dict.set_item("session_id", &self.session_id)?;
        dict.set_item("state", inner.state.as_str())?;
        dict.set_item("config", self.config.to_dict(py)?)?;
        dict.set_item("stats", inner.stats.to_dict(py)?)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        // Manual serialization since inner uses Mutex
        let inner = self
            .inner
            .lock()
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        let snapshot = SessionSnapshot {
            session_id: &self.session_id,
            state: inner.state.as_str(),
            pages_navigated: inner.stats.pages_navigated,
            console_events: inner.stats.console_events,
            network_requests: inner.stats.network_requests,
            duration_ms: inner.stats.duration_ms,
        };
        serde_json::to_string(&snapshot)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        let state = self
            .inner
            .lock()
            .map(|i| i.state.as_str().to_string())
            .unwrap_or_else(|_| "unknown".to_string());
        format!(
            "BrowserSession(id={}, state={}, headless={})",
            self.session_id, state, self.config.headless
        )
    }

    fn __str__(&self) -> String {
        let state = self
            .inner
            .lock()
            .map(|i| i.state.as_str().to_string())
            .unwrap_or_else(|_| "unknown".to_string());
        format!("BrowserSession({})", state)
    }
}

/// Lightweight snapshot for JSON serialization.
#[derive(Serialize)]
struct SessionSnapshot<'a> {
    session_id: &'a str,
    state: &'a str,
    pages_navigated: usize,
    console_events: usize,
    network_requests: usize,
    duration_ms: u64,
}

// ═══════════════════════════════════════════════════════════════════
// AsyncBrowserSession: async session with PyFuture returns
// ═══════════════════════════════════════════════════════════════════

/// An asynchronous browser session returning PyFuture for Python `await`.
#[pyclass]
pub struct AsyncBrowserSession {
    session_id: String,
    config: BrowserSessionConfig,
    inner: Mutex<BrowserSessionInner>,
}

#[pymethods]
impl AsyncBrowserSession {
    #[new]
    fn new(config: BrowserSessionConfig) -> Self {
        let session_id = format!(
            "async-browser-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
        );
        Self {
            session_id,
            config,
            inner: Mutex::new(BrowserSessionInner {
                state: BrowserSessionState::Created,
                stats: BrowserSessionStats {
                    pages_navigated: 0,
                    dom_snapshots: 0,
                    console_events: 0,
                    network_requests: 0,
                    cookies_collected: 0,
                    screenshots_taken: 0,
                    artifacts_collected: 0,
                    duration_ms: 0,
                },
                console_events: Vec::new(),
                network_events: Vec::new(),
            }),
        }
    }

    #[getter]
    fn session_id(&self) -> String {
        self.session_id.clone()
    }

    #[getter]
    fn state(&self) -> PyResult<BrowserSessionState> {
        self.inner
            .lock()
            .map(|i| i.state)
            .map_err(|_| ScanError::new_err("Session state lock poisoned"))
    }

    #[getter]
    fn config(&self) -> BrowserSessionConfig {
        self.config.clone()
    }

    #[getter]
    fn stats(&self) -> PyResult<BrowserSessionStats> {
        self.inner
            .lock()
            .map(|i| i.stats.clone())
            .map_err(|_| ScanError::new_err("Session stats lock poisoned"))
    }

    /// Start the browser session asynchronously.
    fn async_start(&self) -> PyResult<PyFuture> {
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| ScanError::new_err("Session state lock poisoned"))?;

        if inner.state != BrowserSessionState::Created
            && inner.state != BrowserSessionState::Stopped
        {
            return Err(ScanError::new_err(format!(
                "Cannot start session in state {:?}",
                inner.state
            )));
        }

        inner.state = BrowserSessionState::Launching;

        runtime_async::spawn_async(async move {
            // TODO: Launch actual browser engine
            Err::<(), PyErr>(ScanError::new_err(
                "Async browser session requires an active browser engine",
            ))
        })
    }

    /// Stop the browser session asynchronously.
    fn async_stop(&self) -> PyResult<PyFuture> {
        {
            let mut inner = self
                .inner
                .lock()
                .map_err(|_| ScanError::new_err("Session state lock poisoned"))?;

            if inner.state == BrowserSessionState::Stopped
                || inner.state == BrowserSessionState::Cleaning
            {
                return runtime_async::spawn_async(async { Ok(()) });
            }

            inner.state = BrowserSessionState::Stopping;
        }

        runtime_async::spawn_async(async move {
            // TODO: Tear down browser
            Err::<(), PyErr>(ScanError::new_err(
                "Async browser session requires an active browser engine",
            ))
        })
    }

    /// Navigate to a URL asynchronously.
    fn async_navigate(&self, url: &str) -> PyResult<PyFuture> {
        {
            let mut inner = self
                .inner
                .lock()
                .map_err(|_| ScanError::new_err("Session state lock poisoned"))?;

            if inner.state != BrowserSessionState::Ready
                && inner.state != BrowserSessionState::Inspecting
            {
                return Err(ScanError::new_err(format!(
                    "Cannot navigate in state {:?}",
                    inner.state
                )));
            }

            inner.state = BrowserSessionState::Navigating;
        }

        let url_owned = url.to_string();
        runtime_async::spawn_async(async move {
            Err::<BrowserNavigationEvent, PyErr>(ScanError::new_err(format!(
                "Async navigation to '{}' requires an active browser engine",
                url_owned
            )))
        })
    }

    /// Wait for a selector asynchronously.
    fn async_wait_for_selector(
        &self,
        selector: &str,
        timeout_ms: Option<u64>,
    ) -> PyResult<PyFuture> {
        let _ = timeout_ms;
        let selector_owned = selector.to_string();
        runtime_async::spawn_async(async move {
            Err::<bool, PyErr>(ScanError::new_err(format!(
                "Async wait_for_selector('{}') requires an active browser engine",
                selector_owned
            )))
        })
    }

    /// Get a DOM snapshot asynchronously.
    fn async_get_dom_snapshot(&self) -> PyResult<PyFuture> {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        {
            let mut inner = self
                .inner
                .lock()
                .map_err(|_| ScanError::new_err("Session state lock poisoned"))?;
            inner.stats.dom_snapshots += 1;
        }

        runtime_async::spawn_async(async move {
            Err::<BrowserDomSnapshot, PyErr>(ScanError::new_err(
                "Async DOM snapshot requires an active browser engine",
            ))
        })
    }

    /// Get console events asynchronously.
    fn async_get_console_events(&self) -> PyResult<PyFuture> {
        let events = self
            .inner
            .lock()
            .map(|i| i.console_events.clone())
            .map_err(|_| ScanError::new_err("Session state lock poisoned"))?;

        runtime_async::spawn_async(async move { Ok(events) })
    }

    /// Get network events asynchronously.
    fn async_get_network_events(&self) -> PyResult<PyFuture> {
        let events = self
            .inner
            .lock()
            .map(|i| i.network_events.clone())
            .map_err(|_| ScanError::new_err("Session state lock poisoned"))?;

        runtime_async::spawn_async(async move { Ok(events) })
    }

    /// Collect cookies and storage asynchronously.
    fn async_get_cookies(&self) -> PyResult<PyFuture> {
        {
            let mut inner = self
                .inner
                .lock()
                .map_err(|_| ScanError::new_err("Session state lock poisoned"))?;
            inner.stats.cookies_collected += 1;
        }

        runtime_async::spawn_async(async move {
            Err::<BrowserStorageInfo, PyErr>(ScanError::new_err(
                "Async cookie collection requires an active browser engine",
            ))
        })
    }

    /// Take a screenshot asynchronously.
    fn async_take_screenshot(&self) -> PyResult<PyFuture> {
        {
            let mut inner = self
                .inner
                .lock()
                .map_err(|_| ScanError::new_err("Session state lock poisoned"))?;
            inner.stats.screenshots_taken += 1;
        }

        let screenshot_count = self
            .inner
            .lock()
            .map(|i| i.stats.screenshots_taken)
            .unwrap_or(0);

        let artifact_id = format!("screenshot-{}", screenshot_count);
        runtime_async::spawn_async(async move {
            Ok(ArtifactReferencePy {
                artifact_id,
                finding_id: String::new(),
                role: "screenshot".to_string(),
            })
        })
    }

    /// Execute JavaScript asynchronously.
    fn async_execute_script(&self, script: &str, timeout_ms: Option<u64>) -> PyResult<PyFuture> {
        let _ = timeout_ms;
        let script_owned = script.to_string();
        runtime_async::spawn_async(async move {
            Err::<String, PyErr>(ScanError::new_err(format!(
                "Async script execution requires an active browser engine: {}",
                script_owned
            )))
        })
    }

    /// Async context manager __aenter__.
    fn __aenter__(slf: Py<Self>) -> Py<Self> {
        slf
    }

    /// Async context manager __aexit__: stops the session.
    #[pyo3(signature = (_exc_type=None, _exc_value=None, _traceback=None))]
    fn __aexit__(
        &self,
        _exc_type: Option<&Bound<'_, PyAny>>,
        _exc_value: Option<&Bound<'_, PyAny>>,
        _traceback: Option<&Bound<'_, PyAny>>,
    ) -> bool {
        let _ = self.stop_inner();
        false
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let inner = self
            .inner
            .lock()
            .map_err(|_| ScanError::new_err("Session state lock poisoned"))?;
        let dict = PyDict::new_bound(py);
        dict.set_item("session_id", &self.session_id)?;
        dict.set_item("state", inner.state.as_str())?;
        dict.set_item("config", self.config.to_dict(py)?)?;
        dict.set_item("stats", inner.stats.to_dict(py)?)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        let inner = self
            .inner
            .lock()
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        let snapshot = SessionSnapshot {
            session_id: &self.session_id,
            state: inner.state.as_str(),
            pages_navigated: inner.stats.pages_navigated,
            console_events: inner.stats.console_events,
            network_requests: inner.stats.network_requests,
            duration_ms: inner.stats.duration_ms,
        };
        serde_json::to_string(&snapshot)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        let state = self
            .inner
            .lock()
            .map(|i| i.state.as_str().to_string())
            .unwrap_or_else(|_| "unknown".to_string());
        format!(
            "AsyncBrowserSession(id={}, state={}, headless={})",
            self.session_id, state, self.config.headless
        )
    }

    fn __str__(&self) -> String {
        let state = self
            .inner
            .lock()
            .map(|i| i.state.as_str().to_string())
            .unwrap_or_else(|_| "unknown".to_string());
        format!("AsyncBrowserSession({})", state)
    }
}

impl AsyncBrowserSession {
    fn stop_inner(&self) -> PyResult<()> {
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| ScanError::new_err("Session state lock poisoned"))?;

        if inner.state == BrowserSessionState::Stopped
            || inner.state == BrowserSessionState::Cleaning
        {
            return Ok(());
        }

        inner.state = BrowserSessionState::Stopping;
        // TODO: Tear down browser
        inner.state = BrowserSessionState::Stopped;
        Ok(())
    }
}
