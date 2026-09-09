use crate::PyObject;
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
//
// Phase E: capabilities are derived from the active backend compiled into
// this build, not hard-coded aspirational values. `BrowserCapabilities`
// mirrors `eggsec::browser::backend::BrowserBackendCapabilities`; use
// `BrowserCapabilities::current()` instead of constructing values by hand.
// ═══════════════════════════════════════════════════════════════════

/// Validate a navigation URL against managed-session policy (Phase E WS3).
///
/// Managed sessions are network execution: only `http`/`https` URLs with a
/// host are allowed, and userinfo (`user:pass@host`) is rejected so
/// credentials never enter the navigation path. Redirect targets must be
/// re-validated with this helper by the caller; full scope authorization
/// stays with the dispatching surface that owns the loaded scope.
pub fn validate_browser_url_py(url: &str) -> Result<(), String> {
    let url = url.trim();
    if url.is_empty() {
        return Err("browser navigation URL must not be empty".to_string());
    }
    let (scheme, rest) = url.split_once("://").ok_or_else(|| {
        format!("browser navigation URL must use http:// or https:// (got '{url}')")
    })?;
    if scheme != "http" && scheme != "https" {
        return Err(format!(
            "browser navigation URL scheme must be http or https (got '{scheme}')"
        ));
    }
    let authority = rest.split('/').next().unwrap_or_default();
    let authority = authority.split('?').next().unwrap_or_default();
    if authority.is_empty() {
        return Err(format!(
            "browser navigation URL must include a host (got '{url}')"
        ));
    }
    if authority.contains('@') {
        return Err("browser navigation URL must not embed userinfo credentials".to_string());
    }
    Ok(())
}

/// Name of the browser backend compiled into this build (Phase E WS1).
///
/// Returns `"headless_chrome"` when the `headless-browser` Cargo feature is
/// enabled, `"unsupported"` otherwise. Managed sessions derive their
/// advertised capabilities from this, never from hard-coded values.
pub fn browser_backend_name_py() -> &'static str {
    if cfg!(feature = "headless-browser") {
        "headless_chrome"
    } else {
        "unsupported"
    }
}

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
    /// Truthful capabilities for the backend compiled into this build.
    ///
    /// Derives every field from the active backend instead of hard-coding
    /// aspirational values. When the `headless-browser` feature is enabled
    /// the `headless_chrome` backend backs DOM/XSS, SPA discovery, client
    /// checks, interception, console capture, screenshots, cookies and
    /// storage; PDF export and proxying have no engine path and stay false.
    /// Without the feature, nothing is supported.
    #[staticmethod]
    fn current() -> Self {
        if cfg!(feature = "headless-browser") {
            Self {
                engine: "headless_chrome".to_string(),
                version: None,
                supports_javascript: true,
                supports_dom: true,
                supports_network_intercept: true,
                supports_console_capture: true,
                supports_screenshot: true,
                supports_pdf_export: false,
                supports_cookie_access: true,
                supports_storage_access: true,
                supports_route_discovery: true,
                supports_proxy: false,
            }
        } else {
            Self {
                engine: "unsupported".to_string(),
                version: None,
                supports_javascript: false,
                supports_dom: false,
                supports_network_intercept: false,
                supports_console_capture: false,
                supports_screenshot: false,
                supports_pdf_export: false,
                supports_cookie_access: false,
                supports_storage_access: false,
                supports_route_discovery: false,
                supports_proxy: false,
            }
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new(py);
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
        let dict = PyDict::new(py);
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
        let headers_list = PyList::new(py, &self.extra_headers)?;
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
        let dict = PyDict::new(py);
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
        let dict = PyDict::new(py);
        dict.set_item("url", &self.url)?;
        dict.set_item("final_url", &self.final_url)?;
        dict.set_item("status_code", self.status_code)?;
        let redirects = PyList::new(py, &self.redirect_chain)?;
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
        let dict = PyDict::new(py);
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
        let dict = PyDict::new(py);
        dict.set_item("method", &self.method)?;
        dict.set_item("url", &self.url)?;
        dict.set_item("status_code", self.status_code)?;

        let req_headers = PyDict::new(py);
        for (k, v) in &self.request_headers {
            req_headers.set_item(k.as_str(), v.as_str())?;
        }
        dict.set_item("request_headers", req_headers)?;

        let resp_headers = PyDict::new(py);
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
        let dict = PyDict::new(py);
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
        let dict = PyDict::new(py);
        dict.set_item("action", &self.action)?;
        dict.set_item("method", &self.method)?;

        let fields_list = PyList::empty(py);
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
        let dict = PyDict::new(py);
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
        let dict = PyDict::new(py);
        dict.set_item("url", &self.url)?;
        dict.set_item("title", &self.title)?;

        let forms_list = PyList::empty(py);
        for f in &self.forms {
            forms_list.append(f.to_dict(py)?)?;
        }
        dict.set_item("forms", forms_list)?;

        let links_list = PyList::empty(py);
        for l in &self.links {
            links_list.append(l.to_dict(py)?)?;
        }
        dict.set_item("links", links_list)?;

        let scripts_list = PyList::new(py, &self.scripts)?;
        dict.set_item("scripts", scripts_list)?;

        let frames_list = PyList::new(py, &self.frames)?;
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

/// A browser cookie.
///
/// The `value` field carries secret material (session tokens). It is
/// available to the session holder via the getter and `to_dict()`/`to_json()`
/// for explicit opt-in collection, but `__repr__`/`__str__` always mask it
/// so cookie values never leak into logs, events, or reports by accident.
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
        let dict = PyDict::new(py);
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

    /// Masked summary: the value is never rendered into logs or reports.
    fn __repr__(&self) -> String {
        format!(
            "BrowserCookieInfo(name={}, domain={}, path={}, value=[REDACTED])",
            self.name, self.domain, self.path
        )
    }

    fn __str__(&self) -> String {
        format!("BrowserCookieInfo({}=[REDACTED])", self.name)
    }

    /// Copy of this cookie with the value replaced by `[REDACTED]`.
    ///
    /// Use before persisting cookies to reports, checkpoints, or daemon
    /// snapshots unless full-value retention was explicitly opted in.
    fn redacted(&self) -> Self {
        Self {
            name: self.name.clone(),
            value: "[REDACTED]".to_string(),
            domain: self.domain.clone(),
            path: self.path.clone(),
            expires: self.expires,
            http_only: self.http_only,
            secure: self.secure,
        }
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
        let dict = PyDict::new(py);

        let local_list = PyList::empty(py);
        for (k, v) in &self.local_storage {
            let pair = PyTuple::new(py, &[k.as_str(), v.as_str()])?;
            local_list.append(pair)?;
        }
        dict.set_item("local_storage", local_list)?;

        let session_list = PyList::empty(py);
        for (k, v) in &self.session_storage {
            let pair = PyTuple::new(py, &[k.as_str(), v.as_str()])?;
            session_list.append(pair)?;
        }
        dict.set_item("session_storage", session_list)?;

        let cookies_list = PyList::empty(py);
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
    ///
    /// Managed live sessions are provisional: `start()` launches a backend
    /// and reaches `Ready`, or fails with a structured error. The live
    /// tab driver is not yet bound into this class, so `start()` fails
    /// explicitly instead of pretending to launch. Use `browser_test()`
    /// (backed by the real `headless_chrome` engine) for assessment until
    /// managed live sessions are wired to [`browser_backend_name`].
    fn start(&self) -> PyResult<()> {
        let inner = self
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

        Err(ScanError::new_err(
            "Browser session requires an active browser engine: managed live sessions are not yet bound to a backend (see browser_backend_name); use browser_test() for headless assessment",
        ))
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
        // No browser engine to tear down: start() returns an error until the engine
        // is wired up. State transition is a no-op until then.
        inner.state = BrowserSessionState::Stopped;
        Ok(())
    }

    /// Navigate to a URL and return the navigation event.
    ///
    /// The URL is validated against managed-session policy first (http/https
    /// only, host required, no embedded credentials); redirect targets must
    /// be re-validated by the caller under the same policy. Navigation
    /// itself requires a live backend and fails explicitly until managed
    /// live sessions are bound — no synthetic status/timing is returned.
    fn navigate(&self, url: &str) -> PyResult<BrowserNavigationEvent> {
        let inner = self
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

        if let Err(reason) = validate_browser_url_py(url) {
            return Err(ScanError::new_err(format!(
                "Invalid navigation URL: {reason}"
            )));
        }

        Err(ScanError::new_err(format!(
            "Navigation to '{url}' requires an active browser engine: managed live sessions are not yet bound to a backend; use browser_test() for headless assessment"
        )))
    }

    /// Wait for a CSS selector to appear in the DOM.
    ///
    /// Uses actual DOM state with a bounded timeout once a live backend is
    /// bound; until then it fails explicitly. An empty selector is rejected
    /// up front.
    #[pyo3(signature = (selector, timeout_ms=None))]
    fn wait_for_selector(&self, selector: &str, timeout_ms: Option<u64>) -> PyResult<bool> {
        if selector.trim().is_empty() {
            return Err(ScanError::new_err("selector must not be empty"));
        }
        let _ = timeout_ms;
        Err(ScanError::new_err(format!(
            "wait_for_selector('{selector}') requires an active browser engine: managed live sessions are not yet bound to a backend; use browser_test() for headless assessment"
        )))
    }

    /// Capture a DOM snapshot of the current page.
    ///
    /// Requires a live session (`Ready`/`Inspecting`) and a bound backend.
    /// Returns an explicit unsupported error until the backend is wired —
    /// never a synthetic empty snapshot — so callers cannot mistake an
    /// empty page for a successful capture.
    fn get_dom_snapshot(&self) -> PyResult<BrowserDomSnapshot> {
        let inner = self
            .inner
            .lock()
            .map_err(|_| ScanError::new_err("Session state lock poisoned"))?;

        if inner.state != BrowserSessionState::Ready
            && inner.state != BrowserSessionState::Inspecting
        {
            return Err(ScanError::new_err(format!(
                "Cannot capture DOM snapshot in state {:?}",
                inner.state
            )));
        }

        Err(ScanError::new_err(
            "DOM snapshot requires an active browser engine: managed live sessions are not yet bound to a backend; use browser_test() for headless assessment",
        ))
    }

    /// Get all captured console events.
    ///
    /// Returns only events captured from a live backend while collection was
    /// enabled (`collect_console`). Without a live session there is nothing
    /// truthful to return, so this fails explicitly instead of returning a
    /// synthetic empty list.
    fn get_console_events(&self) -> PyResult<Vec<BrowserConsoleEvent>> {
        let inner = self
            .inner
            .lock()
            .map_err(|_| ScanError::new_err("Session state lock poisoned"))?;

        if inner.state != BrowserSessionState::Ready
            && inner.state != BrowserSessionState::Inspecting
        {
            return Err(ScanError::new_err(format!(
                "Cannot read console events in state {:?}: no live backend has captured events",
                inner.state
            )));
        }

        Ok(inner.console_events.clone())
    }

    /// Get all captured network events.
    ///
    /// Returns only events captured from a live backend while collection was
    /// enabled (`collect_network`). Without a live session this fails
    /// explicitly instead of returning a synthetic empty list.
    fn get_network_events(&self) -> PyResult<Vec<BrowserNetworkEvent>> {
        let inner = self
            .inner
            .lock()
            .map_err(|_| ScanError::new_err("Session state lock poisoned"))?;

        if inner.state != BrowserSessionState::Ready
            && inner.state != BrowserSessionState::Inspecting
        {
            return Err(ScanError::new_err(format!(
                "Cannot read network events in state {:?}: no live backend has captured events",
                inner.state
            )));
        }

        Ok(inner.network_events.clone())
    }

    /// Collect cookies and storage from the current page.
    ///
    /// Requires a live session and honors the configured collection settings
    /// (`collect_cookies`/`collect_storage`): with collection disabled this
    /// fails explicitly rather than returning synthetic empty storage.
    /// Cookie values are sensitive; callers must treat the returned storage
    /// as secret material (see `BrowserCookieInfo` redaction notes).
    fn get_cookies(&self) -> PyResult<BrowserStorageInfo> {
        let inner = self
            .inner
            .lock()
            .map_err(|_| ScanError::new_err("Session state lock poisoned"))?;

        if inner.state != BrowserSessionState::Ready
            && inner.state != BrowserSessionState::Inspecting
        {
            return Err(ScanError::new_err(format!(
                "Cannot collect cookies in state {:?}",
                inner.state
            )));
        }

        if !self.config.collect_cookies && !self.config.collect_storage {
            return Err(ScanError::new_err(
                "Cookie/storage collection is disabled for this session (collect_cookies=false, collect_storage=false)",
            ));
        }

        Err(ScanError::new_err(
            "Cookie collection requires an active browser engine: managed live sessions are not yet bound to a backend; use browser_test() for headless assessment",
        ))
    }

    /// Take a screenshot of the current page and return an artifact reference.
    ///
    /// Screenshots write through the artifact store and return a resolvable
    /// artifact reference once a backend is bound. Until then this fails
    /// explicitly — no synthetic `screenshot-N` references are issued, so
    /// every returned artifact ID is guaranteed resolvable.
    fn take_screenshot(&self) -> PyResult<ArtifactReferencePy> {
        let inner = self
            .inner
            .lock()
            .map_err(|_| ScanError::new_err("Session state lock poisoned"))?;

        if inner.state != BrowserSessionState::Ready
            && inner.state != BrowserSessionState::Inspecting
        {
            return Err(ScanError::new_err(format!(
                "Cannot take screenshot in state {:?}",
                inner.state
            )));
        }

        Err(ScanError::new_err(
            "Screenshot capture requires an active browser engine: managed live sessions are not yet bound to a backend; use browser_test() for headless assessment",
        ))
    }

    /// Execute JavaScript in the page context.
    ///
    /// Script execution is risk-classified: it runs only when the backend
    /// capability and security policy allow it. Managed sessions have no
    /// bound backend, so execution is refused explicitly. An empty script
    /// is rejected up front.
    #[pyo3(signature = (script, timeout_ms=None))]
    fn execute_script(&self, script: &str, timeout_ms: Option<u64>) -> PyResult<String> {
        if script.trim().is_empty() {
            return Err(ScanError::new_err("script must not be empty"));
        }
        let _ = timeout_ms;
        Err(ScanError::new_err(
            "Script execution requires an active browser engine and an explicit security-policy allowance: managed live sessions are not yet bound to a backend",
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
        if let Err(error) = self.stop() {
            tracing::warn!(%error, "Failed to stop browser session during context cleanup");
        }
        false
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let inner = self
            .inner
            .lock()
            .map_err(|_| ScanError::new_err("Session state lock poisoned"))?;
        let dict = PyDict::new(py);
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
        let inner = self
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

        runtime_async::spawn_async(async move {
            Err::<(), PyErr>(ScanError::new_err(
                "Async browser session requires an active browser engine",
            ))
        })
    }

    /// Stop the browser session asynchronously.
    ///
    /// Idempotent resource cleanup, mirroring the synchronous `stop()`:
    /// transitions to `Stopped` with no backend to tear down until managed
    /// live sessions are bound.
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
            inner.state = BrowserSessionState::Stopped;
        }

        runtime_async::spawn_async(async { Ok(()) })
    }

    /// Navigate to a URL asynchronously.
    ///
    /// Validates the URL against managed-session policy before spawning;
    /// navigation itself requires a bound backend and fails explicitly.
    fn async_navigate(&self, url: &str) -> PyResult<PyFuture> {
        {
            let inner = self
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
        }

        if let Err(reason) = validate_browser_url_py(url) {
            return Err(ScanError::new_err(format!(
                "Invalid navigation URL: {reason}"
            )));
        }

        let url_owned = url.to_string();
        runtime_async::spawn_async(async move {
            Err::<BrowserNavigationEvent, PyErr>(ScanError::new_err(format!(
                "Async navigation to '{url_owned}' requires an active browser engine: managed live sessions are not yet bound to a backend; use browser_test() for headless assessment"
            )))
        })
    }

    /// Wait for a selector asynchronously.
    #[pyo3(signature = (selector, timeout_ms=None))]
    fn async_wait_for_selector(
        &self,
        selector: &str,
        timeout_ms: Option<u64>,
    ) -> PyResult<PyFuture> {
        if selector.trim().is_empty() {
            return Err(ScanError::new_err("selector must not be empty"));
        }
        let _ = timeout_ms;
        let selector_owned = selector.to_string();
        runtime_async::spawn_async(async move {
            Err::<bool, PyErr>(ScanError::new_err(format!(
                "Async wait_for_selector('{selector_owned}') requires an active browser engine: managed live sessions are not yet bound to a backend"
            )))
        })
    }

    /// Get a DOM snapshot asynchronously.
    ///
    /// Requires a live session; fails explicitly without a bound backend.
    /// Statistics are only updated on successful capture, never on failure.
    fn async_get_dom_snapshot(&self) -> PyResult<PyFuture> {
        {
            let inner = self
                .inner
                .lock()
                .map_err(|_| ScanError::new_err("Session state lock poisoned"))?;

            if inner.state != BrowserSessionState::Ready
                && inner.state != BrowserSessionState::Inspecting
            {
                return Err(ScanError::new_err(format!(
                    "Cannot capture DOM snapshot in state {:?}",
                    inner.state
                )));
            }
        }

        runtime_async::spawn_async(async move {
            Err::<BrowserDomSnapshot, PyErr>(ScanError::new_err(
                "Async DOM snapshot requires an active browser engine: managed live sessions are not yet bound to a backend",
            ))
        })
    }

    /// Get console events asynchronously.
    ///
    /// Without a live session there is nothing truthful to return, so this
    /// fails explicitly instead of resolving to a synthetic empty list.
    fn async_get_console_events(&self) -> PyResult<PyFuture> {
        {
            let inner = self
                .inner
                .lock()
                .map_err(|_| ScanError::new_err("Session state lock poisoned"))?;

            if inner.state != BrowserSessionState::Ready
                && inner.state != BrowserSessionState::Inspecting
            {
                return Err(ScanError::new_err(format!(
                    "Cannot read console events in state {:?}: no live backend has captured events",
                    inner.state
                )));
            }
        }

        let events = self
            .inner
            .lock()
            .map(|i| i.console_events.clone())
            .map_err(|_| ScanError::new_err("Session state lock poisoned"))?;

        runtime_async::spawn_async(async move { Ok(events) })
    }

    /// Get network events asynchronously.
    ///
    /// Without a live session there is nothing truthful to return, so this
    /// fails explicitly instead of resolving to a synthetic empty list.
    fn async_get_network_events(&self) -> PyResult<PyFuture> {
        {
            let inner = self
                .inner
                .lock()
                .map_err(|_| ScanError::new_err("Session state lock poisoned"))?;

            if inner.state != BrowserSessionState::Ready
                && inner.state != BrowserSessionState::Inspecting
            {
                return Err(ScanError::new_err(format!(
                    "Cannot read network events in state {:?}: no live backend has captured events",
                    inner.state
                )));
            }
        }

        let events = self
            .inner
            .lock()
            .map(|i| i.network_events.clone())
            .map_err(|_| ScanError::new_err("Session state lock poisoned"))?;

        runtime_async::spawn_async(async move { Ok(events) })
    }

    /// Collect cookies and storage asynchronously.
    ///
    /// Honors the configured collection settings and fails explicitly
    /// without a bound backend. Statistics are only updated on success.
    fn async_get_cookies(&self) -> PyResult<PyFuture> {
        {
            let inner = self
                .inner
                .lock()
                .map_err(|_| ScanError::new_err("Session state lock poisoned"))?;

            if inner.state != BrowserSessionState::Ready
                && inner.state != BrowserSessionState::Inspecting
            {
                return Err(ScanError::new_err(format!(
                    "Cannot collect cookies in state {:?}",
                    inner.state
                )));
            }
        }

        if !self.config.collect_cookies && !self.config.collect_storage {
            return Err(ScanError::new_err(
                "Cookie/storage collection is disabled for this session (collect_cookies=false, collect_storage=false)",
            ));
        }

        runtime_async::spawn_async(async move {
            Err::<BrowserStorageInfo, PyErr>(ScanError::new_err(
                "Async cookie collection requires an active browser engine: managed live sessions are not yet bound to a backend",
            ))
        })
    }

    /// Take a screenshot asynchronously.
    ///
    /// No synthetic `screenshot-N` references are issued: without a bound
    /// backend and artifact-store write this fails explicitly, so every
    /// returned artifact ID stays resolvable.
    fn async_take_screenshot(&self) -> PyResult<PyFuture> {
        {
            let inner = self
                .inner
                .lock()
                .map_err(|_| ScanError::new_err("Session state lock poisoned"))?;

            if inner.state != BrowserSessionState::Ready
                && inner.state != BrowserSessionState::Inspecting
            {
                return Err(ScanError::new_err(format!(
                    "Cannot take screenshot in state {:?}",
                    inner.state
                )));
            }
        }

        runtime_async::spawn_async(async move {
            Err::<ArtifactReferencePy, PyErr>(ScanError::new_err(
                "Async screenshot capture requires an active browser engine: managed live sessions are not yet bound to a backend",
            ))
        })
    }

    /// Execute JavaScript asynchronously.
    ///
    /// Refused without a bound backend and an explicit security-policy
    /// allowance; an empty script is rejected up front. The script text is
    /// echoed in the error only as an identifier, never executed.
    #[pyo3(signature = (script, timeout_ms=None))]
    fn async_execute_script(&self, script: &str, timeout_ms: Option<u64>) -> PyResult<PyFuture> {
        if script.trim().is_empty() {
            return Err(ScanError::new_err("script must not be empty"));
        }
        let _ = timeout_ms;
        let script_owned = script.to_string();
        runtime_async::spawn_async(async move {
            Err::<String, PyErr>(ScanError::new_err(format!(
                "Async script execution requires an active browser engine and an explicit security-policy allowance: managed live sessions are not yet bound to a backend ({script_owned})"
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
        if let Err(error) = self.stop_inner() {
            tracing::warn!(%error, "Failed to stop async browser session during context cleanup");
        }
        false
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let inner = self
            .inner
            .lock()
            .map_err(|_| ScanError::new_err("Session state lock poisoned"))?;
        let dict = PyDict::new(py);
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
        // No browser engine to tear down: start() returns an error until the engine
        // is wired up. State transition is a no-op until then.
        inner.state = BrowserSessionState::Stopped;
        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════════════
// Phase E WS1/WS3: backend-derived module functions
// ═══════════════════════════════════════════════════════════════════

/// Name of the browser backend compiled into this build.
///
/// Returns `"headless_chrome"` when the `headless-browser` Cargo feature is
/// enabled, `"unsupported"` otherwise. Use with
/// `BrowserCapabilities::current()` to advertise truthful capabilities.
#[pyfunction]
pub fn browser_backend_name() -> &'static str {
    browser_backend_name_py()
}

/// Whether a real browser backend is compiled into this build.
#[pyfunction]
pub fn browser_backend_available() -> bool {
    cfg!(feature = "headless-browser")
}

/// Validate a navigation URL against managed-session policy.
///
/// Only `http`/`https` URLs with a host are accepted; embedded userinfo
/// credentials are rejected. Redirect targets must be re-validated with
/// this function. Raises `ScanError` on violation.
#[pyfunction]
pub fn validate_browser_url(url: &str) -> PyResult<()> {
    validate_browser_url_py(url).map_err(ScanError::new_err)
}
