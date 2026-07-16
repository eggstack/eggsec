//! Python bindings for `eggsec-tool-core` types.
//!
//! Provides PyO3 wrappers around the core tool abstraction layer types
//! including request/response DTOs, enums, findings, errors, rate-limiting,
//! and execution history. These wrappers expose a Pythonic API with
//! `to_dict()`, `to_json()`, `__repr__`, `__str__`, and static constructors.

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use std::collections::HashMap;

use chrono::Utc;
use serde_json;

// ---------------------------------------------------------------------------
// Helper: Python-facing map conversion
// ---------------------------------------------------------------------------

fn hashmap_to_pydict(py: Python, map: &HashMap<String, String>) -> PyResult<PyObject> {
    let dict = PyDict::new_bound(py);
    for (k, v) in map {
        dict.set_item(k, v)?;
    }
    Ok(dict.into())
}

fn fxsmap_to_pydict(py: Python, map: &rustc_hash::FxHashMap<String, String>) -> PyResult<PyObject> {
    let dict = PyDict::new_bound(py);
    for (k, v) in map {
        dict.set_item(k, v)?;
    }
    Ok(dict.into())
}

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

/// Target type for a tool request.
#[pyclass(frozen, eq)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TargetTypePy {
    inner: eggsec_tool_core::TargetType,
}

#[pymethods]
impl TargetTypePy {
    #[staticmethod]
    fn url() -> Self {
        Self {
            inner: eggsec_tool_core::TargetType::Url,
        }
    }

    #[staticmethod]
    fn domain() -> Self {
        Self {
            inner: eggsec_tool_core::TargetType::Domain,
        }
    }

    #[staticmethod]
    fn ip() -> Self {
        Self {
            inner: eggsec_tool_core::TargetType::Ip,
        }
    }

    #[staticmethod]
    fn cidr() -> Self {
        Self {
            inner: eggsec_tool_core::TargetType::Cidr,
        }
    }

    #[staticmethod]
    fn file() -> Self {
        Self {
            inner: eggsec_tool_core::TargetType::File,
        }
    }

    #[staticmethod]
    fn from_str(s: &str) -> PyResult<Self> {
        match s.to_lowercase().as_str() {
            "url" => Ok(Self::url()),
            "domain" => Ok(Self::domain()),
            "ip" => Ok(Self::ip()),
            "cidr" => Ok(Self::cidr()),
            "file" => Ok(Self::file()),
            _ => Err(pyo3::exceptions::PyValueError::new_err(format!(
                "Invalid target type: '{}'. Must be one of: url, domain, ip, cidr, file",
                s
            ))),
        }
    }

    #[getter]
    fn value(&self) -> String {
        self.inner.to_string()
    }

    fn to_dict(&self) -> PyResult<String> {
        Ok(self.inner.to_string())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner.to_string())
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!("TargetTypePy.{}", self.inner)
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }

    fn __hash__(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        self.inner.to_string().hash(&mut hasher);
        hasher.finish()
    }
}

impl TargetTypePy {
    pub fn inner(&self) -> eggsec_tool_core::TargetType {
        self.inner
    }

    pub fn from_inner(inner: eggsec_tool_core::TargetType) -> Self {
        Self { inner }
    }
}

impl From<eggsec_tool_core::TargetType> for TargetTypePy {
    fn from(inner: eggsec_tool_core::TargetType) -> Self {
        Self { inner }
    }
}

impl From<TargetTypePy> for eggsec_tool_core::TargetType {
    fn from(py: TargetTypePy) -> Self {
        py.inner
    }
}

/// Authentication type for tool requests.
#[pyclass(frozen, eq)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuthTypePy {
    inner: eggsec_tool_core::AuthType,
}

#[pymethods]
impl AuthTypePy {
    #[staticmethod]
    fn none() -> Self {
        Self {
            inner: eggsec_tool_core::AuthType::None,
        }
    }

    #[staticmethod]
    fn basic() -> Self {
        Self {
            inner: eggsec_tool_core::AuthType::Basic,
        }
    }

    #[staticmethod]
    fn bearer() -> Self {
        Self {
            inner: eggsec_tool_core::AuthType::Bearer,
        }
    }

    #[staticmethod]
    fn api_key() -> Self {
        Self {
            inner: eggsec_tool_core::AuthType::ApiKey,
        }
    }

    #[staticmethod]
    fn oauth2() -> Self {
        Self {
            inner: eggsec_tool_core::AuthType::OAuth2,
        }
    }

    #[staticmethod]
    fn from_str(s: &str) -> PyResult<Self> {
        match s.to_lowercase().as_str() {
            "none" => Ok(Self::none()),
            "basic" => Ok(Self::basic()),
            "bearer" => Ok(Self::bearer()),
            "api_key" | "apikey" | "api-key" => Ok(Self::api_key()),
            "oauth2" | "oauth" => Ok(Self::oauth2()),
            _ => Err(pyo3::exceptions::PyValueError::new_err(format!(
                "Invalid auth type: '{}'. Must be one of: none, basic, bearer, api_key, oauth2",
                s
            ))),
        }
    }

    #[getter]
    fn value(&self) -> String {
        self.inner.to_string()
    }

    fn to_dict(&self) -> PyResult<String> {
        Ok(self.inner.to_string())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner.to_string())
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!("AuthTypePy.{}", self.inner)
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }

    fn __hash__(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        self.inner.to_string().hash(&mut hasher);
        hasher.finish()
    }
}

impl AuthTypePy {
    pub fn inner(&self) -> eggsec_tool_core::AuthType {
        self.inner
    }
}

impl From<eggsec_tool_core::AuthType> for AuthTypePy {
    fn from(inner: eggsec_tool_core::AuthType) -> Self {
        Self { inner }
    }
}

impl From<AuthTypePy> for eggsec_tool_core::AuthType {
    fn from(py: AuthTypePy) -> Self {
        py.inner
    }
}

/// Response status from a tool execution.
#[pyclass(frozen, eq)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResponseTypePy {
    inner: eggsec_tool_core::ResponseStatus,
}

#[pymethods]
impl ResponseTypePy {
    #[staticmethod]
    fn success() -> Self {
        Self {
            inner: eggsec_tool_core::ResponseStatus::Success,
        }
    }

    #[staticmethod]
    fn partial_success() -> Self {
        Self {
            inner: eggsec_tool_core::ResponseStatus::PartialSuccess,
        }
    }

    #[staticmethod]
    fn failed() -> Self {
        Self {
            inner: eggsec_tool_core::ResponseStatus::Failed,
        }
    }

    #[staticmethod]
    fn timeout() -> Self {
        Self {
            inner: eggsec_tool_core::ResponseStatus::Timeout,
        }
    }

    #[staticmethod]
    fn scope_violation() -> Self {
        Self {
            inner: eggsec_tool_core::ResponseStatus::ScopeViolation,
        }
    }

    #[staticmethod]
    fn cancelled() -> Self {
        Self {
            inner: eggsec_tool_core::ResponseStatus::Cancelled,
        }
    }

    #[staticmethod]
    fn from_str(s: &str) -> PyResult<Self> {
        match s.to_lowercase().as_str() {
            "success" => Ok(Self::success()),
            "partial_success" => Ok(Self::partial_success()),
            "failed" => Ok(Self::failed()),
            "timeout" => Ok(Self::timeout()),
            "scope_violation" => Ok(Self::scope_violation()),
            "cancelled" | "canceled" => Ok(Self::cancelled()),
            _ => Err(pyo3::exceptions::PyValueError::new_err(format!(
                "Invalid response status: '{}'. Must be one of: success, partial_success, failed, timeout, scope_violation, cancelled",
                s
            ))),
        }
    }

    #[getter]
    fn value(&self) -> String {
        self.inner.to_string()
    }

    fn to_dict(&self) -> PyResult<String> {
        Ok(self.inner.to_string())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner.to_string())
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!("ResponseTypePy.{}", self.inner)
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }

    fn __hash__(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        self.inner.to_string().hash(&mut hasher);
        hasher.finish()
    }
}

impl ResponseTypePy {
    pub fn inner(&self) -> eggsec_tool_core::ResponseStatus {
        self.inner
    }
}

impl From<eggsec_tool_core::ResponseStatus> for ResponseTypePy {
    fn from(inner: eggsec_tool_core::ResponseStatus) -> Self {
        Self { inner }
    }
}

impl From<ResponseTypePy> for eggsec_tool_core::ResponseStatus {
    fn from(py: ResponseTypePy) -> Self {
        py.inner
    }
}

/// Classification of a finding.
#[pyclass(frozen, eq, name = "ToolFindingType")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FindingTypePy {
    inner: eggsec_tool_core::FindingType,
}

#[pymethods]
impl FindingTypePy {
    #[staticmethod]
    fn vulnerability() -> Self {
        Self {
            inner: eggsec_tool_core::FindingType::Vulnerability,
        }
    }

    #[staticmethod]
    fn information() -> Self {
        Self {
            inner: eggsec_tool_core::FindingType::Information,
        }
    }

    #[staticmethod]
    fn weakness() -> Self {
        Self {
            inner: eggsec_tool_core::FindingType::Weakness,
        }
    }

    #[staticmethod]
    fn configuration() -> Self {
        Self {
            inner: eggsec_tool_core::FindingType::Configuration,
        }
    }

    #[staticmethod]
    fn misconfiguration() -> Self {
        Self {
            inner: eggsec_tool_core::FindingType::Misconfiguration,
        }
    }

    #[staticmethod]
    fn sensitive_data() -> Self {
        Self {
            inner: eggsec_tool_core::FindingType::SensitiveData,
        }
    }

    #[staticmethod]
    fn banner() -> Self {
        Self {
            inner: eggsec_tool_core::FindingType::Banner,
        }
    }

    #[staticmethod]
    fn technology() -> Self {
        Self {
            inner: eggsec_tool_core::FindingType::Technology,
        }
    }

    #[staticmethod]
    fn service() -> Self {
        Self {
            inner: eggsec_tool_core::FindingType::Service,
        }
    }

    #[staticmethod]
    fn endpoint() -> Self {
        Self {
            inner: eggsec_tool_core::FindingType::Endpoint,
        }
    }

    #[staticmethod]
    fn subdomain() -> Self {
        Self {
            inner: eggsec_tool_core::FindingType::Subdomain,
        }
    }

    #[staticmethod]
    fn open_port() -> Self {
        Self {
            inner: eggsec_tool_core::FindingType::OpenPort,
        }
    }

    #[staticmethod]
    fn from_str(s: &str) -> PyResult<Self> {
        match s.to_lowercase().as_str() {
            "vulnerability" | "vuln" => Ok(Self::vulnerability()),
            "information" | "info" => Ok(Self::information()),
            "weakness" => Ok(Self::weakness()),
            "configuration" | "config" => Ok(Self::configuration()),
            "misconfiguration" => Ok(Self::misconfiguration()),
            "sensitive_data" => Ok(Self::sensitive_data()),
            "banner" => Ok(Self::banner()),
            "technology" | "tech" => Ok(Self::technology()),
            "service" => Ok(Self::service()),
            "endpoint" => Ok(Self::endpoint()),
            "subdomain" | "sub" => Ok(Self::subdomain()),
            "open_port" | "openport" => Ok(Self::open_port()),
            _ => Err(pyo3::exceptions::PyValueError::new_err(format!(
                "Invalid finding type: '{}'. Must be one of: vulnerability, information, weakness, configuration, misconfiguration, sensitive_data, banner, technology, service, endpoint, subdomain, open_port",
                s
            ))),
        }
    }

    #[getter]
    fn value(&self) -> String {
        self.inner.to_string()
    }

    fn to_dict(&self) -> PyResult<String> {
        Ok(self.inner.to_string())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner.to_string())
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!("FindingTypePy.{}", self.inner)
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }

    fn __hash__(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        self.inner.to_string().hash(&mut hasher);
        hasher.finish()
    }
}

impl FindingTypePy {
    pub fn inner(&self) -> eggsec_tool_core::FindingType {
        self.inner
    }
}

impl From<eggsec_tool_core::FindingType> for FindingTypePy {
    fn from(inner: eggsec_tool_core::FindingType) -> Self {
        Self { inner }
    }
}

impl From<FindingTypePy> for eggsec_tool_core::FindingType {
    fn from(py: FindingTypePy) -> Self {
        py.inner
    }
}

/// Severity level for findings and responses.
#[pyclass(frozen, eq, name = "ToolSeverity")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SeverityPy {
    inner: eggsec_tool_core::ResponseSeverity,
}

#[pymethods]
impl SeverityPy {
    #[staticmethod]
    fn critical() -> Self {
        Self {
            inner: eggsec_tool_core::ResponseSeverity::Critical,
        }
    }

    #[staticmethod]
    fn high() -> Self {
        Self {
            inner: eggsec_tool_core::ResponseSeverity::High,
        }
    }

    #[staticmethod]
    fn medium() -> Self {
        Self {
            inner: eggsec_tool_core::ResponseSeverity::Medium,
        }
    }

    #[staticmethod]
    fn low() -> Self {
        Self {
            inner: eggsec_tool_core::ResponseSeverity::Low,
        }
    }

    #[staticmethod]
    fn info() -> Self {
        Self {
            inner: eggsec_tool_core::ResponseSeverity::Info,
        }
    }

    #[staticmethod]
    fn none() -> Self {
        Self {
            inner: eggsec_tool_core::ResponseSeverity::None,
        }
    }

    #[staticmethod]
    fn from_str(s: &str) -> PyResult<Self> {
        match s.to_lowercase().as_str() {
            "critical" => Ok(Self::critical()),
            "high" => Ok(Self::high()),
            "medium" | "moderate" => Ok(Self::medium()),
            "low" => Ok(Self::low()),
            "info" | "informational" => Ok(Self::info()),
            "none" => Ok(Self::none()),
            _ => Err(pyo3::exceptions::PyValueError::new_err(format!(
                "Invalid severity: '{}'. Must be one of: critical, high, medium, low, info, none",
                s
            ))),
        }
    }

    #[getter]
    fn value(&self) -> String {
        self.inner.as_str().to_string()
    }

    fn to_dict(&self) -> PyResult<String> {
        Ok(self.inner.as_str().to_string())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self.inner.as_str())
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!("SeverityPy.{}", self.inner)
    }

    fn __str__(&self) -> String {
        self.inner.as_str().to_string()
    }

    fn __hash__(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        self.inner.as_str().hash(&mut hasher);
        hasher.finish()
    }
}

impl SeverityPy {
    pub fn inner(&self) -> eggsec_tool_core::ResponseSeverity {
        self.inner
    }
}

impl From<eggsec_tool_core::ResponseSeverity> for SeverityPy {
    fn from(inner: eggsec_tool_core::ResponseSeverity) -> Self {
        Self { inner }
    }
}

impl From<SeverityPy> for eggsec_tool_core::ResponseSeverity {
    fn from(py: SeverityPy) -> Self {
        py.inner
    }
}

/// Classification of a tool error.
#[pyclass(frozen, eq)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToolErrorTypePy {
    inner: eggsec_tool_core::ToolErrorType,
}

#[pymethods]
impl ToolErrorTypePy {
    #[staticmethod]
    fn validation() -> Self {
        Self {
            inner: eggsec_tool_core::ToolErrorType::Validation,
        }
    }

    #[staticmethod]
    fn authentication() -> Self {
        Self {
            inner: eggsec_tool_core::ToolErrorType::Authentication,
        }
    }

    #[staticmethod]
    fn authorization() -> Self {
        Self {
            inner: eggsec_tool_core::ToolErrorType::Authorization,
        }
    }

    #[staticmethod]
    fn rate_limit() -> Self {
        Self {
            inner: eggsec_tool_core::ToolErrorType::RateLimit,
        }
    }

    #[staticmethod]
    fn network() -> Self {
        Self {
            inner: eggsec_tool_core::ToolErrorType::Network,
        }
    }

    #[staticmethod]
    fn timeout() -> Self {
        Self {
            inner: eggsec_tool_core::ToolErrorType::Timeout,
        }
    }

    #[staticmethod]
    fn scope_violation() -> Self {
        Self {
            inner: eggsec_tool_core::ToolErrorType::ScopeViolation,
        }
    }

    #[staticmethod]
    fn not_found() -> Self {
        Self {
            inner: eggsec_tool_core::ToolErrorType::NotFound,
        }
    }

    #[staticmethod]
    fn configuration() -> Self {
        Self {
            inner: eggsec_tool_core::ToolErrorType::Configuration,
        }
    }

    #[staticmethod]
    fn internal() -> Self {
        Self {
            inner: eggsec_tool_core::ToolErrorType::Internal,
        }
    }

    #[staticmethod]
    fn tool_not_found() -> Self {
        Self {
            inner: eggsec_tool_core::ToolErrorType::ToolNotFound,
        }
    }

    #[staticmethod]
    fn from_str(s: &str) -> PyResult<Self> {
        match s.to_lowercase().as_str() {
            "validation" => Ok(Self::validation()),
            "authentication" | "auth" => Ok(Self::authentication()),
            "authorization" => Ok(Self::authorization()),
            "rate_limit" | "ratelimit" => Ok(Self::rate_limit()),
            "network" => Ok(Self::network()),
            "timeout" => Ok(Self::timeout()),
            "scope_violation" => Ok(Self::scope_violation()),
            "not_found" => Ok(Self::not_found()),
            "configuration" | "config" => Ok(Self::configuration()),
            "internal" => Ok(Self::internal()),
            "tool_not_found" => Ok(Self::tool_not_found()),
            _ => Err(pyo3::exceptions::PyValueError::new_err(format!(
                "Invalid error type: '{}'. Must be one of: validation, authentication, authorization, rate_limit, network, timeout, scope_violation, not_found, configuration, internal, tool_not_found",
                s
            ))),
        }
    }

    #[getter]
    fn value(&self) -> String {
        self.inner.as_str().to_string()
    }

    fn to_dict(&self) -> PyResult<String> {
        Ok(self.inner.as_str().to_string())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self.inner.as_str())
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!("ToolErrorTypePy.{}", self.inner)
    }

    fn __str__(&self) -> String {
        self.inner.as_str().to_string()
    }

    fn __hash__(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        self.inner.as_str().hash(&mut hasher);
        hasher.finish()
    }
}

impl ToolErrorTypePy {
    pub fn inner(&self) -> eggsec_tool_core::ToolErrorType {
        self.inner
    }
}

impl From<eggsec_tool_core::ToolErrorType> for ToolErrorTypePy {
    fn from(inner: eggsec_tool_core::ToolErrorType) -> Self {
        Self { inner }
    }
}

impl From<ToolErrorTypePy> for eggsec_tool_core::ToolErrorType {
    fn from(py: ToolErrorTypePy) -> Self {
        py.inner
    }
}

/// Port scan result state.
#[pyclass(frozen, eq)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PortStatePy {
    inner: eggsec_tool_core::PortState,
}

#[pymethods]
impl PortStatePy {
    #[staticmethod]
    fn open() -> Self {
        Self {
            inner: eggsec_tool_core::PortState::Open,
        }
    }

    #[staticmethod]
    fn closed() -> Self {
        Self {
            inner: eggsec_tool_core::PortState::Closed,
        }
    }

    #[staticmethod]
    fn filtered() -> Self {
        Self {
            inner: eggsec_tool_core::PortState::Filtered,
        }
    }

    #[staticmethod]
    fn from_str(s: &str) -> PyResult<Self> {
        match s.to_lowercase().as_str() {
            "open" => Ok(Self::open()),
            "closed" => Ok(Self::closed()),
            "filtered" => Ok(Self::filtered()),
            _ => Err(pyo3::exceptions::PyValueError::new_err(format!(
                "Invalid port state: '{}'. Must be one of: open, closed, filtered",
                s
            ))),
        }
    }

    #[getter]
    fn value(&self) -> String {
        format!("{:?}", self.inner).to_lowercase()
    }

    fn to_dict(&self) -> PyResult<String> {
        Ok(format!("{:?}", self.inner).to_lowercase())
    }

    fn to_json(&self) -> PyResult<String> {
        let s = format!("{:?}", self.inner).to_lowercase();
        serde_json::to_string(&s)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!("PortStatePy.{:?}", self.inner)
    }

    fn __str__(&self) -> String {
        format!("{:?}", self.inner).to_lowercase()
    }

    fn __hash__(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        format!("{:?}", self.inner).hash(&mut hasher);
        hasher.finish()
    }
}

impl PortStatePy {
    pub fn inner(&self) -> eggsec_tool_core::PortState {
        self.inner
    }
}

impl From<eggsec_tool_core::PortState> for PortStatePy {
    fn from(inner: eggsec_tool_core::PortState) -> Self {
        Self { inner }
    }
}

impl From<PortStatePy> for eggsec_tool_core::PortState {
    fn from(py: PortStatePy) -> Self {
        py.inner
    }
}

/// Type of stream event.
#[pyclass(frozen, eq)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StreamEventTypePy {
    inner: eggsec_tool_core::StreamEventType,
}

#[pymethods]
impl StreamEventTypePy {
    #[staticmethod]
    fn progress() -> Self {
        Self {
            inner: eggsec_tool_core::StreamEventType::Progress,
        }
    }

    #[staticmethod]
    fn finding() -> Self {
        Self {
            inner: eggsec_tool_core::StreamEventType::Finding,
        }
    }

    #[staticmethod]
    fn result() -> Self {
        Self {
            inner: eggsec_tool_core::StreamEventType::Result,
        }
    }

    #[staticmethod]
    fn error() -> Self {
        Self {
            inner: eggsec_tool_core::StreamEventType::Error,
        }
    }

    #[staticmethod]
    fn from_str(s: &str) -> PyResult<Self> {
        match s.to_lowercase().as_str() {
            "progress" => Ok(Self::progress()),
            "finding" => Ok(Self::finding()),
            "result" => Ok(Self::result()),
            "error" => Ok(Self::error()),
            _ => Err(pyo3::exceptions::PyValueError::new_err(format!(
                "Invalid stream event type: '{}'. Must be one of: progress, finding, result, error",
                s
            ))),
        }
    }

    #[getter]
    fn value(&self) -> String {
        self.inner.to_string()
    }

    fn to_dict(&self) -> PyResult<String> {
        Ok(self.inner.to_string())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner.to_string())
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!("StreamEventTypePy.{}", self.inner)
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }

    fn __hash__(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        self.inner.to_string().hash(&mut hasher);
        hasher.finish()
    }
}

impl StreamEventTypePy {
    pub fn inner(&self) -> eggsec_tool_core::StreamEventType {
        self.inner
    }
}

impl From<eggsec_tool_core::StreamEventType> for StreamEventTypePy {
    fn from(inner: eggsec_tool_core::StreamEventType) -> Self {
        Self { inner }
    }
}

// ---------------------------------------------------------------------------
// Structs
// ---------------------------------------------------------------------------

/// Tool execution scope (allowed/excluded patterns and IPs).
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct ScopeToolPy {
    inner: eggsec_tool_core::Scope,
}

#[pymethods]
impl ScopeToolPy {
    #[staticmethod]
    #[pyo3(signature = (allowed_patterns=None, excluded_patterns=None, allowed_ips=None, allow_subdomains=true))]
    fn new(
        allowed_patterns: Option<Vec<String>>,
        excluded_patterns: Option<Vec<String>>,
        allowed_ips: Option<Vec<String>>,
        allow_subdomains: bool,
    ) -> Self {
        Self {
            inner: eggsec_tool_core::Scope {
                allowed_patterns: allowed_patterns.unwrap_or_else(|| vec!["*".to_string()]),
                excluded_patterns: excluded_patterns.unwrap_or_default(),
                allowed_ips: allowed_ips.unwrap_or_default(),
                allow_subdomains,
            },
        }
    }

    #[staticmethod]
    fn allow_all() -> Self {
        Self {
            inner: eggsec_tool_core::Scope::default(),
        }
    }

    #[staticmethod]
    fn deny_all() -> Self {
        Self {
            inner: eggsec_tool_core::Scope {
                allowed_patterns: vec![],
                excluded_patterns: vec!["*".to_string()],
                allowed_ips: vec![],
                allow_subdomains: false,
            },
        }
    }

    #[getter]
    fn allowed_patterns(&self) -> Vec<String> {
        self.inner.allowed_patterns.clone()
    }

    #[getter]
    fn excluded_patterns(&self) -> Vec<String> {
        self.inner.excluded_patterns.clone()
    }

    #[getter]
    fn allowed_ips(&self) -> Vec<String> {
        self.inner.allowed_ips.clone()
    }

    #[getter]
    fn allow_subdomains(&self) -> bool {
        self.inner.allow_subdomains
    }

    fn is_allowed(&self, target: &str) -> bool {
        self.inner.is_allowed(target)
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("allowed_patterns", &self.inner.allowed_patterns)?;
        dict.set_item("excluded_patterns", &self.inner.excluded_patterns)?;
        dict.set_item("allowed_ips", &self.inner.allowed_ips)?;
        dict.set_item("allow_subdomains", self.inner.allow_subdomains)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "ScopeToolPy(allowed_patterns={:?}, allow_subdomains={})",
            self.inner.allowed_patterns, self.inner.allow_subdomains
        )
    }

    fn __str__(&self) -> String {
        format!(
            "Scope({} patterns, {} excluded, {} ips, subdomains={})",
            self.inner.allowed_patterns.len(),
            self.inner.excluded_patterns.len(),
            self.inner.allowed_ips.len(),
            self.inner.allow_subdomains
        )
    }
}

impl ScopeToolPy {
    pub fn inner(&self) -> &eggsec_tool_core::Scope {
        &self.inner
    }

    pub fn into_inner(self) -> eggsec_tool_core::Scope {
        self.inner
    }
}

impl From<eggsec_tool_core::Scope> for ScopeToolPy {
    fn from(inner: eggsec_tool_core::Scope) -> Self {
        Self { inner }
    }
}

/// Scanning target (type + value + optional scope).
#[pyclass(frozen, name = "ToolTarget")]
#[derive(Debug, Clone)]
pub struct TargetPy {
    inner: eggsec_tool_core::Target,
}

#[pymethods]
impl TargetPy {
    #[staticmethod]
    fn url(value: &str) -> Self {
        Self {
            inner: eggsec_tool_core::Target::url(value),
        }
    }

    #[staticmethod]
    fn domain(value: &str) -> Self {
        Self {
            inner: eggsec_tool_core::Target::domain(value),
        }
    }

    #[staticmethod]
    fn ip(value: &str) -> Self {
        Self {
            inner: eggsec_tool_core::Target::ip(value),
        }
    }

    #[staticmethod]
    fn cidr(value: &str) -> Self {
        Self {
            inner: eggsec_tool_core::Target::cidr(value),
        }
    }

    #[staticmethod]
    fn file(value: &str) -> Self {
        Self {
            inner: eggsec_tool_core::Target {
                target_type: eggsec_tool_core::TargetType::File,
                value: value.to_string(),
                scope: None,
            },
        }
    }

    #[staticmethod]
    fn with_scope(target: &TargetPy, scope: ScopeToolPy) -> Self {
        Self {
            inner: target.inner.clone().with_scope(scope.into_inner()),
        }
    }

    #[getter]
    fn target_type(&self) -> TargetTypePy {
        TargetTypePy::from_inner(self.inner.target_type)
    }

    #[getter]
    fn value(&self) -> String {
        self.inner.value.clone()
    }

    #[getter]
    fn scope(&self) -> Option<ScopeToolPy> {
        self.inner.scope.clone().map(ScopeToolPy::from)
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("target_type", format!("{}", self.inner.target_type))?;
        dict.set_item("value", &self.inner.value)?;
        match &self.inner.scope {
            Some(scope) => {
                dict.set_item("scope", ScopeToolPy::from(scope.clone()).to_dict(py)?)?;
            }
            None => {
                dict.set_item("scope", py.None())?;
            }
        }
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "TargetPy(type={}, value={})",
            self.inner.target_type, self.inner.value
        )
    }

    fn __str__(&self) -> String {
        format!("{}:{}", self.inner.target_type, self.inner.value)
    }
}

impl TargetPy {
    pub fn inner(&self) -> &eggsec_tool_core::Target {
        &self.inner
    }

    pub fn into_inner(self) -> eggsec_tool_core::Target {
        self.inner
    }
}

impl From<eggsec_tool_core::Target> for TargetPy {
    fn from(inner: eggsec_tool_core::Target) -> Self {
        Self { inner }
    }
}

/// Request options (timeout, concurrency, proxy, stealth, etc.).
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct RequestOptionsPy {
    inner: eggsec_tool_core::RequestOptions,
}

#[pymethods]
impl RequestOptionsPy {
    #[staticmethod]
    #[pyo3(signature = (timeout_ms=None, concurrency=None, rate_limit=None, proxy=None, stealth=false, follow_redirects=true, verify_ssl=true))]
    fn new(
        timeout_ms: Option<u64>,
        concurrency: Option<usize>,
        rate_limit: Option<f64>,
        proxy: Option<String>,
        stealth: bool,
        follow_redirects: bool,
        verify_ssl: bool,
    ) -> Self {
        Self {
            inner: eggsec_tool_core::RequestOptions {
                timeout_ms,
                concurrency,
                rate_limit,
                proxy,
                headers: None,
                auth: None,
                stealth,
                follow_redirects,
                verify_ssl,
            },
        }
    }

    #[getter]
    fn timeout_ms(&self) -> Option<u64> {
        self.inner.timeout_ms
    }

    #[getter]
    fn concurrency(&self) -> Option<usize> {
        self.inner.concurrency
    }

    #[getter]
    fn rate_limit(&self) -> Option<f64> {
        self.inner.rate_limit
    }

    #[getter]
    fn proxy(&self) -> Option<String> {
        self.inner.proxy.clone()
    }

    #[getter]
    fn stealth(&self) -> bool {
        self.inner.stealth
    }

    #[getter]
    fn follow_redirects(&self) -> bool {
        self.inner.follow_redirects
    }

    #[getter]
    fn verify_ssl(&self) -> bool {
        self.inner.verify_ssl
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("timeout_ms", self.inner.timeout_ms)?;
        dict.set_item("concurrency", self.inner.concurrency)?;
        dict.set_item("rate_limit", self.inner.rate_limit)?;
        dict.set_item("proxy", &self.inner.proxy)?;
        dict.set_item("stealth", self.inner.stealth)?;
        dict.set_item("follow_redirects", self.inner.follow_redirects)?;
        dict.set_item("verify_ssl", self.inner.verify_ssl)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "RequestOptionsPy(timeout_ms={:?}, concurrency={:?}, stealth={}, verify_ssl={})",
            self.inner.timeout_ms,
            self.inner.concurrency,
            self.inner.stealth,
            self.inner.verify_ssl
        )
    }

    fn __str__(&self) -> String {
        format!(
            "timeout={:?}ms, concurrency={:?}, stealth={}",
            self.inner.timeout_ms, self.inner.concurrency, self.inner.stealth
        )
    }
}

impl RequestOptionsPy {
    pub fn inner(&self) -> &eggsec_tool_core::RequestOptions {
        &self.inner
    }

    pub fn into_inner(self) -> eggsec_tool_core::RequestOptions {
        self.inner
    }
}

impl From<eggsec_tool_core::RequestOptions> for RequestOptionsPy {
    fn from(inner: eggsec_tool_core::RequestOptions) -> Self {
        Self { inner }
    }
}

/// Authentication configuration (type + credentials).
///
/// NOTE: `__repr__` and `__str__` redact credential values for safety.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct AuthConfigPy {
    inner: eggsec_tool_core::AuthConfig,
}

#[pymethods]
impl AuthConfigPy {
    #[staticmethod]
    fn basic(username: &str, password: &str) -> Self {
        Self {
            inner: eggsec_tool_core::AuthConfig::basic(username, password),
        }
    }

    #[staticmethod]
    fn bearer(token: &str) -> Self {
        Self {
            inner: eggsec_tool_core::AuthConfig::bearer(token),
        }
    }

    #[staticmethod]
    fn api_key(key: &str, header: &str) -> Self {
        Self {
            inner: eggsec_tool_core::AuthConfig::api_key(key, header),
        }
    }

    #[getter]
    fn auth_type(&self) -> AuthTypePy {
        AuthTypePy::from(self.inner.auth_type)
    }

    #[getter]
    fn credentials(&self, py: Python) -> PyResult<PyObject> {
        fxsmap_to_pydict(py, &self.inner.credentials)
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("auth_type", format!("{}", self.inner.auth_type))?;
        let creds_dict = PyDict::new_bound(py);
        for (k, _v) in &self.inner.credentials {
            creds_dict.set_item(k, "[REDACTED]")?;
        }
        dict.set_item("credentials", creds_dict)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        let redacted = serde_json::json!({
            "auth_type": self.inner.auth_type.to_string(),
            "credentials": self.inner.credentials.keys().collect::<Vec<_>>(),
        });
        serde_json::to_string(&redacted)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "AuthConfigPy(auth_type={}, credentials=[REDACTED])",
            self.inner.auth_type
        )
    }

    fn __str__(&self) -> String {
        format!(
            "AuthConfig(type={}, credentials=[REDACTED])",
            self.inner.auth_type
        )
    }
}

impl AuthConfigPy {
    pub fn inner(&self) -> &eggsec_tool_core::AuthConfig {
        &self.inner
    }

    pub fn into_inner(self) -> eggsec_tool_core::AuthConfig {
        self.inner
    }
}

impl From<eggsec_tool_core::AuthConfig> for AuthConfigPy {
    fn from(inner: eggsec_tool_core::AuthConfig) -> Self {
        Self { inner }
    }
}

/// A tool execution request.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct ToolRequestPy {
    inner: eggsec_tool_core::ToolRequest,
}

#[pymethods]
impl ToolRequestPy {
    #[staticmethod]
    #[pyo3(signature = (tool, target, params=None, options=None))]
    fn new(
        tool: &str,
        target: &TargetPy,
        params: Option<&Bound<'_, PyDict>>,
        options: Option<&RequestOptionsPy>,
    ) -> PyResult<Self> {
        let mut req = eggsec_tool_core::ToolRequest::new(tool, target.inner.clone());
        if let Some(p) = params {
            let json_mod = p.py().import_bound("json")?;
            let json_str_obj = json_mod.call_method1("dumps", (p,))?;
            let json_str: String = json_str_obj.extract()?;
            let json_val: serde_json::Value = serde_json::from_str(&json_str)
                .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
            req = req.with_params(json_val);
        }
        if let Some(opts) = options {
            req = req.with_options(opts.inner.clone());
        }
        Ok(Self { inner: req })
    }

    #[getter]
    fn id(&self) -> String {
        self.inner.id.clone()
    }

    #[getter]
    fn tool(&self) -> String {
        self.inner.tool.clone()
    }

    #[getter]
    fn target(&self) -> TargetPy {
        TargetPy::from(self.inner.target.clone())
    }

    #[getter]
    fn params(&self, py: Python) -> PyResult<PyObject> {
        let json_str = serde_json::to_string(&self.inner.params)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        let json_mod = py.import_bound("json")?;
        let obj = json_mod.call_method1("loads", (json_str,))?;
        Ok(obj.into())
    }

    #[getter]
    fn options(&self) -> RequestOptionsPy {
        RequestOptionsPy::from(self.inner.options.clone())
    }

    #[getter]
    fn has_cancellation(&self) -> bool {
        self.inner.cancellation_token.is_some()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("id", &self.inner.id)?;
        dict.set_item("tool", &self.inner.tool)?;
        dict.set_item("target_type", format!("{}", self.inner.target.target_type))?;
        dict.set_item("target_value", &self.inner.target.value)?;
        dict.set_item("params", self.params(py)?)?;
        dict.set_item(
            "options",
            RequestOptionsPy::from(self.inner.options.clone()).to_dict(py)?,
        )?;
        dict.set_item("has_cancellation", self.has_cancellation())?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "ToolRequestPy(id={}, tool={}, target_type={}, target_value={})",
            self.inner.id, self.inner.tool, self.inner.target.target_type, self.inner.target.value
        )
    }

    fn __str__(&self) -> String {
        format!(
            "ToolRequest({}: {} -> {})",
            self.inner.tool, self.inner.target.target_type, self.inner.target.value
        )
    }
}

impl ToolRequestPy {
    pub fn inner(&self) -> &eggsec_tool_core::ToolRequest {
        &self.inner
    }

    pub fn into_inner(self) -> eggsec_tool_core::ToolRequest {
        self.inner
    }
}

/// Response metadata (timing, counts).
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct ResponseMetadataPy {
    inner: eggsec_tool_core::ResponseMetadata,
}

#[pymethods]
impl ResponseMetadataPy {
    #[new]
    #[pyo3(signature = (started_at, completed_at, duration_ms, targets_scanned, findings_count))]
    fn new(
        started_at: &str,
        completed_at: &str,
        duration_ms: u64,
        targets_scanned: usize,
        findings_count: usize,
    ) -> PyResult<Self> {
        let parse_dt = |s: &str| -> PyResult<chrono::DateTime<chrono::Utc>> {
            chrono::DateTime::parse_from_rfc3339(s)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .or_else(|_| {
                    chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S")
                        .map(|naive| naive.and_utc())
                })
                .map_err(|_| {
                    pyo3::exceptions::PyValueError::new_err(format!(
                        "Invalid datetime: '{}'. Expected ISO 8601 format.",
                        s
                    ))
                })
        };
        Ok(Self {
            inner: eggsec_tool_core::ResponseMetadata {
                started_at: parse_dt(started_at)?,
                completed_at: parse_dt(completed_at)?,
                duration_ms,
                targets_scanned,
                findings_count,
            },
        })
    }

    #[staticmethod]
    fn now() -> Self {
        let now = Utc::now();
        Self {
            inner: eggsec_tool_core::ResponseMetadata {
                started_at: now,
                completed_at: now,
                duration_ms: 0,
                targets_scanned: 0,
                findings_count: 0,
            },
        }
    }

    #[getter]
    fn started_at(&self) -> String {
        self.inner.started_at.to_rfc3339()
    }

    #[getter]
    fn completed_at(&self) -> String {
        self.inner.completed_at.to_rfc3339()
    }

    #[getter]
    fn duration_ms(&self) -> u64 {
        self.inner.duration_ms
    }

    #[getter]
    fn targets_scanned(&self) -> usize {
        self.inner.targets_scanned
    }

    #[getter]
    fn findings_count(&self) -> usize {
        self.inner.findings_count
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("started_at", self.inner.started_at.to_rfc3339())?;
        dict.set_item("completed_at", self.inner.completed_at.to_rfc3339())?;
        dict.set_item("duration_ms", self.inner.duration_ms)?;
        dict.set_item("targets_scanned", self.inner.targets_scanned)?;
        dict.set_item("findings_count", self.inner.findings_count)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "ResponseMetadataPy(started={}, completed={}, duration_ms={}, findings={})",
            self.inner.started_at.to_rfc3339(),
            self.inner.completed_at.to_rfc3339(),
            self.inner.duration_ms,
            self.inner.findings_count
        )
    }

    fn __str__(&self) -> String {
        format!(
            "metadata({}ms, {} targets, {} findings)",
            self.inner.duration_ms, self.inner.targets_scanned, self.inner.findings_count
        )
    }
}

impl ResponseMetadataPy {
    pub fn inner(&self) -> &eggsec_tool_core::ResponseMetadata {
        &self.inner
    }

    pub fn into_inner(self) -> eggsec_tool_core::ResponseMetadata {
        self.inner
    }
}

impl From<eggsec_tool_core::ResponseMetadata> for ResponseMetadataPy {
    fn from(inner: eggsec_tool_core::ResponseMetadata) -> Self {
        Self { inner }
    }
}

/// A finding discovered during tool execution.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct ToolFindingPy {
    inner: eggsec_tool_core::Finding,
}

#[pymethods]
impl ToolFindingPy {
    #[new]
    #[pyo3(signature = (id, finding_type, severity, title, description, location, evidence=None, cve_ids=None, remediation=None))]
    fn new(
        id: &str,
        finding_type: &FindingTypePy,
        severity: &SeverityPy,
        title: &str,
        description: &str,
        location: &str,
        evidence: Option<String>,
        cve_ids: Option<Vec<String>>,
        remediation: Option<String>,
    ) -> Self {
        Self {
            inner: eggsec_tool_core::Finding {
                id: id.to_string(),
                finding_type: finding_type.inner(),
                severity: severity.inner(),
                title: title.to_string(),
                description: description.to_string(),
                location: location.to_string(),
                evidence,
                cve_ids: cve_ids.unwrap_or_default(),
                remediation,
                references: vec![],
                metadata: rustc_hash::FxHashMap::default(),
            },
        }
    }

    #[getter]
    fn id(&self) -> String {
        self.inner.id.clone()
    }

    #[getter]
    fn finding_type(&self) -> FindingTypePy {
        FindingTypePy::from(self.inner.finding_type)
    }

    #[getter]
    fn severity(&self) -> SeverityPy {
        SeverityPy::from(self.inner.severity)
    }

    #[getter]
    fn title(&self) -> String {
        self.inner.title.clone()
    }

    #[getter]
    fn description(&self) -> String {
        self.inner.description.clone()
    }

    #[getter]
    fn location(&self) -> String {
        self.inner.location.clone()
    }

    #[getter]
    fn evidence(&self) -> Option<String> {
        self.inner.evidence.clone()
    }

    #[getter]
    fn cve_ids(&self) -> Vec<String> {
        self.inner.cve_ids.clone()
    }

    #[getter]
    fn remediation(&self) -> Option<String> {
        self.inner.remediation.clone()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("id", &self.inner.id)?;
        dict.set_item("finding_type", self.inner.finding_type.to_string())?;
        dict.set_item("severity", self.inner.severity.as_str())?;
        dict.set_item("title", &self.inner.title)?;
        dict.set_item("description", &self.inner.description)?;
        dict.set_item("location", &self.inner.location)?;
        dict.set_item("evidence", &self.inner.evidence)?;
        dict.set_item("cve_ids", &self.inner.cve_ids)?;
        dict.set_item("remediation", &self.inner.remediation)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "ToolFindingPy(id={}, finding_type={}, severity={}, title={})",
            self.inner.id, self.inner.finding_type, self.inner.severity, self.inner.title
        )
    }

    fn __str__(&self) -> String {
        format!(
            "[{}] {} at {}",
            self.inner.severity, self.inner.title, self.inner.location
        )
    }
}

impl ToolFindingPy {
    pub fn inner(&self) -> &eggsec_tool_core::Finding {
        &self.inner
    }

    pub fn into_inner(self) -> eggsec_tool_core::Finding {
        self.inner
    }
}

impl From<eggsec_tool_core::Finding> for ToolFindingPy {
    fn from(inner: eggsec_tool_core::Finding) -> Self {
        Self { inner }
    }
}

/// Tool execution error.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct ToolErrorPy {
    inner: eggsec_tool_core::ToolError,
}

#[pymethods]
impl ToolErrorPy {
    #[new]
    #[pyo3(signature = (code, message, error_type=None, recoverable=false, retry_after_ms=None))]
    fn new(
        code: &str,
        message: &str,
        error_type: Option<&ToolErrorTypePy>,
        recoverable: bool,
        retry_after_ms: Option<u64>,
    ) -> Self {
        Self {
            inner: eggsec_tool_core::ToolError {
                code: code.to_string(),
                message: message.to_string(),
                details: None,
                target: None,
                recoverable,
                error_type: error_type
                    .map(|e| e.inner())
                    .unwrap_or(eggsec_tool_core::ToolErrorType::Internal),
                retry_after_ms,
            },
        }
    }

    #[getter]
    fn code(&self) -> String {
        self.inner.code.clone()
    }

    #[getter]
    fn message(&self) -> String {
        self.inner.message.clone()
    }

    #[getter]
    fn error_type(&self) -> ToolErrorTypePy {
        ToolErrorTypePy::from(self.inner.error_type)
    }

    #[getter]
    fn recoverable(&self) -> bool {
        self.inner.recoverable
    }

    #[getter]
    fn retry_after_ms(&self) -> Option<u64> {
        self.inner.retry_after_ms
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("code", &self.inner.code)?;
        dict.set_item("message", &self.inner.message)?;
        dict.set_item("error_type", self.inner.error_type.as_str())?;
        dict.set_item("recoverable", self.inner.recoverable)?;
        dict.set_item("retry_after_ms", self.inner.retry_after_ms)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "ToolErrorPy(code={}, message={}, error_type={}, recoverable={})",
            self.inner.code, self.inner.message, self.inner.error_type, self.inner.recoverable
        )
    }

    fn __str__(&self) -> String {
        format!("[{}] {}", self.inner.code, self.inner.message)
    }
}

impl ToolErrorPy {
    pub fn inner(&self) -> &eggsec_tool_core::ToolError {
        &self.inner
    }

    pub fn into_inner(self) -> eggsec_tool_core::ToolError {
        self.inner
    }
}

impl From<eggsec_tool_core::ToolError> for ToolErrorPy {
    fn from(inner: eggsec_tool_core::ToolError) -> Self {
        Self { inner }
    }
}

/// A tool execution response (status + results + metadata + errors + findings).
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct ToolResponsePy {
    inner: eggsec_tool_core::ToolResponse,
}

#[pymethods]
impl ToolResponsePy {
    #[getter]
    fn request_id(&self) -> String {
        self.inner.request_id.clone()
    }

    #[getter]
    fn tool_id(&self) -> String {
        self.inner.tool_id.clone()
    }

    #[getter]
    fn status(&self) -> ResponseTypePy {
        ResponseTypePy::from(self.inner.status)
    }

    #[getter]
    fn results(&self, py: Python) -> PyResult<PyObject> {
        let json_str = serde_json::to_string(&self.inner.results)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        let json_mod = py.import_bound("json")?;
        let obj = json_mod.call_method1("loads", (json_str,))?;
        Ok(obj.into())
    }

    #[getter]
    fn metadata(&self) -> ResponseMetadataPy {
        ResponseMetadataPy::from(self.inner.metadata.clone())
    }

    #[getter]
    fn errors(&self) -> Vec<ToolErrorPy> {
        self.inner
            .errors
            .iter()
            .map(|e| ToolErrorPy::from(e.clone()))
            .collect()
    }

    #[getter]
    fn findings(&self) -> Vec<ToolFindingPy> {
        self.inner
            .findings
            .iter()
            .map(|f| ToolFindingPy::from(f.clone()))
            .collect()
    }

    fn is_success(&self) -> bool {
        self.inner.is_success()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("request_id", &self.inner.request_id)?;
        dict.set_item("tool_id", &self.inner.tool_id)?;
        dict.set_item("status", self.inner.status.to_string())?;
        dict.set_item("results", self.results(py)?)?;
        dict.set_item(
            "metadata",
            ResponseMetadataPy::from(self.inner.metadata.clone()).to_dict(py)?,
        )?;
        let errors_list = PyList::empty_bound(py);
        for e in &self.inner.errors {
            errors_list.append(ToolErrorPy::from(e.clone()).to_dict(py)?)?;
        }
        dict.set_item("errors", errors_list)?;
        let findings_list = PyList::empty_bound(py);
        for f in &self.inner.findings {
            findings_list.append(ToolFindingPy::from(f.clone()).to_dict(py)?)?;
        }
        dict.set_item("findings", findings_list)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "ToolResponsePy(request_id={}, tool_id={}, status={}, findings={}, errors={})",
            self.inner.request_id,
            self.inner.tool_id,
            self.inner.status,
            self.inner.findings.len(),
            self.inner.errors.len()
        )
    }

    fn __str__(&self) -> String {
        format!(
            "ToolResponse({}: {} with {} findings, {} errors)",
            self.inner.tool_id,
            self.inner.status,
            self.inner.findings.len(),
            self.inner.errors.len()
        )
    }
}

impl ToolResponsePy {
    pub fn inner(&self) -> &eggsec_tool_core::ToolResponse {
        &self.inner
    }

    pub fn into_inner(self) -> eggsec_tool_core::ToolResponse {
        self.inner
    }
}

impl From<eggsec_tool_core::ToolResponse> for ToolResponsePy {
    fn from(inner: eggsec_tool_core::ToolResponse) -> Self {
        Self { inner }
    }
}

/// Progress update for a streaming tool execution.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct ProgressUpdatePy {
    inner: eggsec_tool_core::ProgressUpdate,
}

#[pymethods]
impl ProgressUpdatePy {
    #[new]
    fn new(
        request_id: &str,
        stage: &str,
        progress: f32,
        message: &str,
        items_found: usize,
    ) -> Self {
        Self {
            inner: eggsec_tool_core::ProgressUpdate {
                request_id: request_id.to_string(),
                stage: stage.to_string(),
                progress,
                message: message.to_string(),
                items_found,
            },
        }
    }

    #[getter]
    fn request_id(&self) -> String {
        self.inner.request_id.clone()
    }

    #[getter]
    fn stage(&self) -> String {
        self.inner.stage.clone()
    }

    #[getter]
    fn progress(&self) -> f32 {
        self.inner.progress
    }

    #[getter]
    fn message(&self) -> String {
        self.inner.message.clone()
    }

    #[getter]
    fn items_found(&self) -> usize {
        self.inner.items_found
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("request_id", &self.inner.request_id)?;
        dict.set_item("stage", &self.inner.stage)?;
        dict.set_item("progress", self.inner.progress)?;
        dict.set_item("message", &self.inner.message)?;
        dict.set_item("items_found", self.inner.items_found)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "ProgressUpdatePy(request_id={}, stage={}, progress={}, items_found={})",
            self.inner.request_id, self.inner.stage, self.inner.progress, self.inner.items_found
        )
    }

    fn __str__(&self) -> String {
        format!(
            "Progress({}, {:.0}%, {} items)",
            self.inner.stage,
            self.inner.progress * 100.0,
            self.inner.items_found
        )
    }
}

impl From<eggsec_tool_core::ProgressUpdate> for ProgressUpdatePy {
    fn from(inner: eggsec_tool_core::ProgressUpdate) -> Self {
        Self { inner }
    }
}

/// A stream event (progress, finding, result, or error).
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct StreamEventPy {
    inner: eggsec_tool_core::StreamEvent,
}

#[pymethods]
impl StreamEventPy {
    #[staticmethod]
    fn progress_event(
        request_id: &str,
        stage: &str,
        progress: f32,
        message: &str,
        items_found: usize,
    ) -> Self {
        Self {
            inner: eggsec_tool_core::StreamEvent::progress(
                request_id,
                stage,
                progress,
                message,
                items_found,
            ),
        }
    }

    #[staticmethod]
    fn finding_event(finding: &ToolFindingPy) -> Self {
        Self {
            inner: eggsec_tool_core::StreamEvent::finding(finding.inner.clone()),
        }
    }

    #[staticmethod]
    fn result_event(response: &ToolResponsePy) -> Self {
        Self {
            inner: eggsec_tool_core::StreamEvent::result(response.inner.clone()),
        }
    }

    #[staticmethod]
    fn error() -> Self {
        Self {
            inner: eggsec_tool_core::StreamEvent {
                event_type: eggsec_tool_core::StreamEventType::Error,
                request_id: None,
                progress: None,
                finding: None,
                result: None,
            },
        }
    }

    #[getter]
    fn event_type(&self) -> StreamEventTypePy {
        StreamEventTypePy::from(self.inner.event_type)
    }

    #[getter]
    fn request_id(&self) -> Option<String> {
        self.inner.request_id.clone()
    }

    #[getter]
    fn progress(&self) -> Option<ProgressUpdatePy> {
        self.inner.progress.clone().map(ProgressUpdatePy::from)
    }

    #[getter]
    fn finding(&self) -> Option<ToolFindingPy> {
        self.inner.finding.clone().map(ToolFindingPy::from)
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("event_type", self.inner.event_type.to_string())?;
        dict.set_item("request_id", &self.inner.request_id)?;
        match &self.inner.progress {
            Some(p) => {
                dict.set_item("progress", ProgressUpdatePy::from(p.clone()).to_dict(py)?)?;
            }
            None => {
                dict.set_item("progress", py.None())?;
            }
        }
        match &self.inner.finding {
            Some(f) => {
                dict.set_item("finding", ToolFindingPy::from(f.clone()).to_dict(py)?)?;
            }
            None => {
                dict.set_item("finding", py.None())?;
            }
        }
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "StreamEventPy(event_type={}, request_id={:?})",
            self.inner.event_type, self.inner.request_id
        )
    }

    fn __str__(&self) -> String {
        match &self.inner.event_type {
            eggsec_tool_core::StreamEventType::Progress => {
                if let Some(ref p) = self.inner.progress {
                    format!("StreamEvent(Progress: {})", p.stage)
                } else {
                    "StreamEvent(Progress)".to_string()
                }
            }
            eggsec_tool_core::StreamEventType::Finding => {
                if let Some(ref f) = self.inner.finding {
                    format!("StreamEvent(Finding: {})", f.title)
                } else {
                    "StreamEvent(Finding)".to_string()
                }
            }
            eggsec_tool_core::StreamEventType::Result => "StreamEvent(Result)".to_string(),
            eggsec_tool_core::StreamEventType::Error => "StreamEvent(Error)".to_string(),
        }
    }
}

impl From<eggsec_tool_core::StreamEvent> for StreamEventPy {
    fn from(inner: eggsec_tool_core::StreamEvent) -> Self {
        Self { inner }
    }
}

/// Port scan result data for a single port.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct PortDataPy {
    inner: eggsec_tool_core::PortData,
}

#[pymethods]
impl PortDataPy {
    #[new]
    #[pyo3(signature = (port, protocol, state, service=None, version=None, banner=None))]
    fn new(
        port: u16,
        protocol: &str,
        state: &PortStatePy,
        service: Option<String>,
        version: Option<String>,
        banner: Option<String>,
    ) -> Self {
        Self {
            inner: eggsec_tool_core::PortData {
                port,
                protocol: protocol.to_string(),
                state: state.inner(),
                service,
                version,
                banner,
            },
        }
    }

    #[getter]
    fn port(&self) -> u16 {
        self.inner.port
    }

    #[getter]
    fn protocol(&self) -> String {
        self.inner.protocol.clone()
    }

    #[getter]
    fn state(&self) -> PortStatePy {
        PortStatePy::from(self.inner.state)
    }

    #[getter]
    fn service(&self) -> Option<String> {
        self.inner.service.clone()
    }

    #[getter]
    fn version(&self) -> Option<String> {
        self.inner.version.clone()
    }

    #[getter]
    fn banner(&self) -> Option<String> {
        self.inner.banner.clone()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("port", self.inner.port)?;
        dict.set_item("protocol", &self.inner.protocol)?;
        dict.set_item("state", format!("{:?}", self.inner.state).to_lowercase())?;
        dict.set_item("service", &self.inner.service)?;
        dict.set_item("version", &self.inner.version)?;
        dict.set_item("banner", &self.inner.banner)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "PortDataPy(port={}, protocol={}, state={:?})",
            self.inner.port, self.inner.protocol, self.inner.state
        )
    }

    fn __str__(&self) -> String {
        format!(
            "{}/{} ({:?})",
            self.inner.port, self.inner.protocol, self.inner.state
        )
    }
}

impl From<eggsec_tool_core::PortData> for PortDataPy {
    fn from(inner: eggsec_tool_core::PortData) -> Self {
        Self { inner }
    }
}

/// Discovered endpoint data.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct EndpointDataPy {
    inner: eggsec_tool_core::EndpointData,
}

#[pymethods]
impl EndpointDataPy {
    #[new]
    #[pyo3(signature = (url, status_code=None, content_length=None, content_type=None))]
    fn new(
        url: &str,
        status_code: Option<u16>,
        content_length: Option<u64>,
        content_type: Option<String>,
    ) -> Self {
        Self {
            inner: eggsec_tool_core::EndpointData {
                url: url.to_string(),
                status_code,
                content_length,
                content_type,
                discovered_at: Utc::now(),
            },
        }
    }

    #[getter]
    fn url(&self) -> String {
        self.inner.url.clone()
    }

    #[getter]
    fn status_code(&self) -> Option<u16> {
        self.inner.status_code
    }

    #[getter]
    fn content_length(&self) -> Option<u64> {
        self.inner.content_length
    }

    #[getter]
    fn content_type(&self) -> Option<String> {
        self.inner.content_type.clone()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("url", &self.inner.url)?;
        dict.set_item("status_code", self.inner.status_code)?;
        dict.set_item("content_length", self.inner.content_length)?;
        dict.set_item("content_type", &self.inner.content_type)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "EndpointDataPy(url={}, status_code={:?})",
            self.inner.url, self.inner.status_code
        )
    }

    fn __str__(&self) -> String {
        format!(
            "Endpoint({}, status={:?})",
            self.inner.url, self.inner.status_code
        )
    }
}

impl From<eggsec_tool_core::EndpointData> for EndpointDataPy {
    fn from(inner: eggsec_tool_core::EndpointData) -> Self {
        Self { inner }
    }
}

/// Detected technology information.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct TechnologyDataPy {
    inner: eggsec_tool_core::TechnologyData,
}

#[pymethods]
impl TechnologyDataPy {
    #[new]
    #[pyo3(signature = (name, category, version=None, confidence=0.0, website=None, cpe=None))]
    fn new(
        name: &str,
        category: &str,
        version: Option<String>,
        confidence: f32,
        website: Option<String>,
        cpe: Option<String>,
    ) -> Self {
        Self {
            inner: eggsec_tool_core::TechnologyData {
                name: name.to_string(),
                version,
                category: category.to_string(),
                confidence,
                website,
                cpe,
            },
        }
    }

    #[getter]
    fn name(&self) -> String {
        self.inner.name.clone()
    }

    #[getter]
    fn version(&self) -> Option<String> {
        self.inner.version.clone()
    }

    #[getter]
    fn category(&self) -> String {
        self.inner.category.clone()
    }

    #[getter]
    fn confidence(&self) -> f32 {
        self.inner.confidence
    }

    #[getter]
    fn website(&self) -> Option<String> {
        self.inner.website.clone()
    }

    #[getter]
    fn cpe(&self) -> Option<String> {
        self.inner.cpe.clone()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("name", &self.inner.name)?;
        dict.set_item("version", &self.inner.version)?;
        dict.set_item("category", &self.inner.category)?;
        dict.set_item("confidence", self.inner.confidence)?;
        dict.set_item("website", &self.inner.website)?;
        dict.set_item("cpe", &self.inner.cpe)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "TechnologyDataPy(name={}, version={:?}, category={})",
            self.inner.name, self.inner.version, self.inner.category
        )
    }

    fn __str__(&self) -> String {
        match &self.inner.version {
            Some(v) => format!("{} {} ({})", self.inner.name, v, self.inner.category),
            None => format!("{} ({})", self.inner.name, self.inner.category),
        }
    }
}

impl From<eggsec_tool_core::TechnologyData> for TechnologyDataPy {
    fn from(inner: eggsec_tool_core::TechnologyData) -> Self {
        Self { inner }
    }
}

/// Rate-limit configuration for tool execution.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct RateLimitConfigPy {
    inner: eggsec_tool_core::RateLimitConfig,
}

#[pymethods]
impl RateLimitConfigPy {
    #[staticmethod]
    fn standard() -> Self {
        Self {
            inner: eggsec_tool_core::RateLimitConfig::standard(),
        }
    }

    #[staticmethod]
    fn relaxed() -> Self {
        Self {
            inner: eggsec_tool_core::RateLimitConfig::relaxed(),
        }
    }

    #[staticmethod]
    fn strict() -> Self {
        Self {
            inner: eggsec_tool_core::RateLimitConfig::strict(),
        }
    }

    #[new]
    #[pyo3(signature = (requests_per_minute=60, concurrent_scans=5, burst_size=10, global_rate_limit=None, enable_ip_based_limiting=false))]
    fn new(
        requests_per_minute: u32,
        concurrent_scans: u32,
        burst_size: u32,
        global_rate_limit: Option<u32>,
        enable_ip_based_limiting: bool,
    ) -> Self {
        Self {
            inner: eggsec_tool_core::RateLimitConfig {
                requests_per_minute,
                concurrent_scans,
                burst_size,
                per_endpoint_limits: rustc_hash::FxHashMap::default(),
                global_rate_limit,
                enable_ip_based_limiting,
            },
        }
    }

    #[getter]
    fn requests_per_minute(&self) -> u32 {
        self.inner.requests_per_minute
    }

    #[getter]
    fn concurrent_scans(&self) -> u32 {
        self.inner.concurrent_scans
    }

    #[getter]
    fn burst_size(&self) -> u32 {
        self.inner.burst_size
    }

    #[getter]
    fn global_rate_limit(&self) -> Option<u32> {
        self.inner.global_rate_limit
    }

    #[getter]
    fn enable_ip_based_limiting(&self) -> bool {
        self.inner.enable_ip_based_limiting
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("requests_per_minute", self.inner.requests_per_minute)?;
        dict.set_item("concurrent_scans", self.inner.concurrent_scans)?;
        dict.set_item("burst_size", self.inner.burst_size)?;
        dict.set_item("global_rate_limit", self.inner.global_rate_limit)?;
        dict.set_item(
            "enable_ip_based_limiting",
            self.inner.enable_ip_based_limiting,
        )?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "RateLimitConfigPy(rpm={}, concurrent={}, burst={})",
            self.inner.requests_per_minute, self.inner.concurrent_scans, self.inner.burst_size
        )
    }

    fn __str__(&self) -> String {
        format!(
            "RateLimit({}rpm, {} concurrent, burst {})",
            self.inner.requests_per_minute, self.inner.concurrent_scans, self.inner.burst_size
        )
    }
}

impl From<eggsec_tool_core::RateLimitConfig> for RateLimitConfigPy {
    fn from(inner: eggsec_tool_core::RateLimitConfig) -> Self {
        Self { inner }
    }
}

/// Current rate-limit status for a tool execution.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct RateLimitStatusPy {
    inner: eggsec_tool_core::RateLimitStatus,
}

#[pymethods]
impl RateLimitStatusPy {
    #[new]
    fn new(
        tokens_available: f64,
        requests_this_minute: u32,
        requests_per_minute: u32,
        concurrent_available: usize,
        concurrent_limit: u32,
        concurrent_in_use: usize,
    ) -> Self {
        Self {
            inner: eggsec_tool_core::RateLimitStatus {
                tokens_available,
                requests_this_minute,
                requests_per_minute,
                concurrent_available,
                concurrent_limit,
                concurrent_in_use,
            },
        }
    }

    #[getter]
    fn tokens_available(&self) -> f64 {
        self.inner.tokens_available
    }

    #[getter]
    fn requests_this_minute(&self) -> u32 {
        self.inner.requests_this_minute
    }

    #[getter]
    fn requests_per_minute(&self) -> u32 {
        self.inner.requests_per_minute
    }

    #[getter]
    fn concurrent_available(&self) -> usize {
        self.inner.concurrent_available
    }

    #[getter]
    fn concurrent_limit(&self) -> u32 {
        self.inner.concurrent_limit
    }

    #[getter]
    fn concurrent_in_use(&self) -> usize {
        self.inner.concurrent_in_use
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("tokens_available", self.inner.tokens_available)?;
        dict.set_item("requests_this_minute", self.inner.requests_this_minute)?;
        dict.set_item("requests_per_minute", self.inner.requests_per_minute)?;
        dict.set_item("concurrent_available", self.inner.concurrent_available)?;
        dict.set_item("concurrent_limit", self.inner.concurrent_limit)?;
        dict.set_item("concurrent_in_use", self.inner.concurrent_in_use)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        let val = serde_json::json!({
            "tokens_available": self.inner.tokens_available,
            "requests_this_minute": self.inner.requests_this_minute,
            "requests_per_minute": self.inner.requests_per_minute,
            "concurrent_available": self.inner.concurrent_available,
            "concurrent_limit": self.inner.concurrent_limit,
            "concurrent_in_use": self.inner.concurrent_in_use,
        });
        serde_json::to_string(&val)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "RateLimitStatusPy(tokens={:.1}, rpm={}/{}, concurrent={}/{})",
            self.inner.tokens_available,
            self.inner.requests_this_minute,
            self.inner.requests_per_minute,
            self.inner.concurrent_in_use,
            self.inner.concurrent_limit
        )
    }

    fn __str__(&self) -> String {
        format!(
            "RateLimit({:.0}/{}, concurrent {}/{})",
            self.inner.tokens_available,
            self.inner.requests_per_minute,
            self.inner.concurrent_in_use,
            self.inner.concurrent_limit
        )
    }
}

impl From<eggsec_tool_core::RateLimitStatus> for RateLimitStatusPy {
    fn from(inner: eggsec_tool_core::RateLimitStatus) -> Self {
        Self { inner }
    }
}

/// Execution history entry (record of a completed tool execution).
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct ExecutionEntryPy {
    inner: eggsec_tool_core::ExecutionEntry,
}

#[pymethods]
impl ExecutionEntryPy {
    #[getter]
    fn request_id(&self) -> String {
        self.inner.request_id.clone()
    }

    #[getter]
    fn tool_id(&self) -> String {
        self.inner.tool_id.clone()
    }

    #[getter]
    fn target(&self) -> String {
        self.inner.target.clone()
    }

    #[getter]
    fn target_type(&self) -> String {
        self.inner.target_type.clone()
    }

    #[getter]
    fn status(&self) -> String {
        self.inner.status.clone()
    }

    #[getter]
    fn started_at(&self) -> String {
        self.inner.started_at.to_rfc3339()
    }

    #[getter]
    fn completed_at(&self) -> String {
        self.inner.completed_at.to_rfc3339()
    }

    #[getter]
    fn duration_ms(&self) -> u64 {
        self.inner.duration_ms
    }

    #[getter]
    fn findings_count(&self) -> usize {
        self.inner.findings_count
    }

    #[getter]
    fn errors_count(&self) -> usize {
        self.inner.errors_count
    }

    #[getter]
    fn summary(&self) -> String {
        self.inner.summary.clone()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("request_id", &self.inner.request_id)?;
        dict.set_item("tool_id", &self.inner.tool_id)?;
        dict.set_item("target", &self.inner.target)?;
        dict.set_item("target_type", &self.inner.target_type)?;
        dict.set_item("status", &self.inner.status)?;
        dict.set_item("started_at", self.inner.started_at.to_rfc3339())?;
        dict.set_item("completed_at", self.inner.completed_at.to_rfc3339())?;
        dict.set_item("duration_ms", self.inner.duration_ms)?;
        dict.set_item("findings_count", self.inner.findings_count)?;
        dict.set_item("errors_count", self.inner.errors_count)?;
        dict.set_item("summary", &self.inner.summary)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "ExecutionEntryPy(request_id={}, tool_id={}, target={}, status={})",
            self.inner.request_id, self.inner.tool_id, self.inner.target, self.inner.status
        )
    }

    fn __str__(&self) -> String {
        format!(
            "{}({} -> {}, {}ms, {} findings)",
            self.inner.tool_id,
            self.inner.target,
            self.inner.status,
            self.inner.duration_ms,
            self.inner.findings_count
        )
    }
}

impl From<eggsec_tool_core::ExecutionEntry> for ExecutionEntryPy {
    fn from(inner: eggsec_tool_core::ExecutionEntry) -> Self {
        Self { inner }
    }
}

// ---------------------------------------------------------------------------
// Conversion helpers
// ---------------------------------------------------------------------------

/// Convert a Python ToolRequestPy into a Rust ToolRequest.
pub fn into_tool_request(req: &ToolRequestPy) -> eggsec_tool_core::ToolRequest {
    req.inner.clone()
}

/// Convert a Rust ToolResponse into a Python ToolResponsePy.
pub fn from_tool_response(resp: eggsec_tool_core::ToolResponse) -> ToolResponsePy {
    ToolResponsePy::from(resp)
}
