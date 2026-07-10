use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use serde::{Deserialize, Serialize};

use crate::finding::Severity;
use crate::runtime_sync;

/// DOM XSS finding source type.
#[pyclass(frozen, eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum XssSourcePy {
    LocationHash,
    LocationSearch,
    DocumentCookie,
    DocumentReferrer,
    LocalStorage,
    SessionStorage,
    WebSocket,
    PostMessage,
}

#[pymethods]
impl XssSourcePy {
    fn __repr__(&self) -> String {
        format!("XssSource.{}", self.as_str())
    }

    fn __str__(&self) -> String {
        self.as_str().to_string()
    }
}

impl XssSourcePy {
    fn as_str(&self) -> &str {
        match self {
            XssSourcePy::LocationHash => "LocationHash",
            XssSourcePy::LocationSearch => "LocationSearch",
            XssSourcePy::DocumentCookie => "DocumentCookie",
            XssSourcePy::DocumentReferrer => "DocumentReferrer",
            XssSourcePy::LocalStorage => "LocalStorage",
            XssSourcePy::SessionStorage => "SessionStorage",
            XssSourcePy::WebSocket => "WebSocket",
            XssSourcePy::PostMessage => "PostMessage",
        }
    }
}

/// DOM XSS finding sink type.
#[pyclass(frozen, eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum XssSinkPy {
    InnerHTML,
    OuterHTML,
    JQueryHtml,
    DocumentWrite,
    Eval,
    SetTimeout,
    SetInterval,
    FunctionConstructor,
    ScriptSrc,
    OnEventHandler,
}

#[pymethods]
impl XssSinkPy {
    fn __repr__(&self) -> String {
        format!("XssSink.{}", self.as_str())
    }

    fn __str__(&self) -> String {
        self.as_str().to_string()
    }
}

impl XssSinkPy {
    fn as_str(&self) -> &str {
        match self {
            XssSinkPy::InnerHTML => "InnerHTML",
            XssSinkPy::OuterHTML => "OuterHTML",
            XssSinkPy::JQueryHtml => "JQueryHtml",
            XssSinkPy::DocumentWrite => "DocumentWrite",
            XssSinkPy::Eval => "Eval",
            XssSinkPy::SetTimeout => "SetTimeout",
            XssSinkPy::SetInterval => "SetInterval",
            XssSinkPy::FunctionConstructor => "FunctionConstructor",
            XssSinkPy::ScriptSrc => "ScriptSrc",
            XssSinkPy::OnEventHandler => "OnEventHandler",
        }
    }
}

/// A DOM XSS finding from headless browser scanning.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomXssFindingPy {
    #[pyo3(get)]
    pub id: String,
    #[pyo3(get)]
    pub source: String,
    #[pyo3(get)]
    pub sink: String,
    #[pyo3(get)]
    pub location: String,
    #[pyo3(get)]
    pub severity: Severity,
    #[pyo3(get)]
    pub description: String,
    #[pyo3(get)]
    pub evidence: String,
    #[pyo3(get)]
    pub remediation: String,
    #[pyo3(get)]
    pub cvss_score: Option<f32>,
}

impl DomXssFindingPy {
    fn from_engine(engine: eggsec::browser::xss_dom::DomXssFinding) -> Self {
        Self {
            id: engine.id,
            source: engine.source,
            sink: engine.sink,
            location: engine.location,
            severity: Severity::from_engine(engine.severity),
            description: engine.description,
            evidence: engine.evidence,
            remediation: engine.remediation,
            cvss_score: engine.cvss_score,
        }
    }
}

#[pymethods]
impl DomXssFindingPy {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("id", &self.id)?;
        dict.set_item("source", &self.source)?;
        dict.set_item("sink", &self.sink)?;
        dict.set_item("location", &self.location)?;
        dict.set_item("severity", self.severity.as_str())?;
        dict.set_item("description", &self.description)?;
        dict.set_item("evidence", &self.evidence)?;
        dict.set_item("remediation", &self.remediation)?;
        dict.set_item("cvss_score", self.cvss_score)?;
        Ok(dict.into())
    }

    fn __repr__(&self) -> String {
        format!(
            "DomXssFinding(id={}, source={}, sink={})",
            self.id, self.source, self.sink
        )
    }
}

/// SPA route discovery method.
#[pyclass(frozen, eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DiscoveryMethodPy {
    Crawl,
    XhrInterception,
    FetchInterception,
    RouteParsing,
}

#[pymethods]
impl DiscoveryMethodPy {
    fn __repr__(&self) -> String {
        format!("DiscoveryMethod.{}", self.as_str())
    }

    fn __str__(&self) -> String {
        self.as_str().to_string()
    }
}

impl DiscoveryMethodPy {
    fn as_str(&self) -> &str {
        match self {
            DiscoveryMethodPy::Crawl => "Crawl",
            DiscoveryMethodPy::XhrInterception => "XhrInterception",
            DiscoveryMethodPy::FetchInterception => "FetchInterception",
            DiscoveryMethodPy::RouteParsing => "RouteParsing",
        }
    }
}

/// A discovered SPA route.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpaRoutePy {
    #[pyo3(get)]
    pub path: String,
    #[pyo3(get)]
    pub method: String,
    parameters: Vec<String>,
    #[pyo3(get)]
    pub discovered_via: DiscoveryMethodPy,
}

impl SpaRoutePy {
    fn from_engine(engine: eggsec::browser::spa_discovery::SpaRoute) -> Self {
        Self {
            path: engine.path,
            method: engine.method,
            parameters: engine.parameters,
            discovered_via: match engine.discovered_via {
                eggsec::browser::spa_discovery::DiscoveryMethod::Crawl => DiscoveryMethodPy::Crawl,
                eggsec::browser::spa_discovery::DiscoveryMethod::XhrInterception => {
                    DiscoveryMethodPy::XhrInterception
                }
                eggsec::browser::spa_discovery::DiscoveryMethod::FetchInterception => {
                    DiscoveryMethodPy::FetchInterception
                }
                eggsec::browser::spa_discovery::DiscoveryMethod::RouteParsing => {
                    DiscoveryMethodPy::RouteParsing
                }
            },
        }
    }
}

#[pymethods]
impl SpaRoutePy {
    #[getter]
    fn parameters(&self) -> Vec<String> {
        self.parameters.clone()
    }

    fn __repr__(&self) -> String {
        format!("SpaRoute(path={}, method={})", self.path, self.method)
    }
}

/// Client-side issue type.
#[pyclass(frozen, eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ClientIssueTypePy {
    LocalStorageSensitive,
    CorsMisconfiguration,
    CSPSourceMap,
    DebugMode,
    SourceMapsExposed,
    CORSWildcard,
}

#[pymethods]
impl ClientIssueTypePy {
    fn __repr__(&self) -> String {
        format!("ClientIssueType.{}", self.as_str())
    }

    fn __str__(&self) -> String {
        self.as_str().to_string()
    }
}

impl ClientIssueTypePy {
    fn as_str(&self) -> &str {
        match self {
            ClientIssueTypePy::LocalStorageSensitive => "LocalStorageSensitive",
            ClientIssueTypePy::CorsMisconfiguration => "CorsMisconfiguration",
            ClientIssueTypePy::CSPSourceMap => "CSPSourceMap",
            ClientIssueTypePy::DebugMode => "DebugMode",
            ClientIssueTypePy::SourceMapsExposed => "SourceMapsExposed",
            ClientIssueTypePy::CORSWildcard => "CORSWildcard",
        }
    }
}

/// A client-side security issue.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientIssuePy {
    #[pyo3(get)]
    pub id: String,
    #[pyo3(get)]
    pub issue_type: ClientIssueTypePy,
    #[pyo3(get)]
    pub severity: Severity,
    #[pyo3(get)]
    pub location: String,
    #[pyo3(get)]
    pub description: String,
    #[pyo3(get)]
    pub evidence: String,
    #[pyo3(get)]
    pub remediation: String,
    #[pyo3(get)]
    pub cvss_score: Option<f32>,
}

impl ClientIssuePy {
    fn from_engine(engine: eggsec::browser::client_checks::ClientIssue) -> Self {
        Self {
            id: engine.id,
            issue_type: match engine.issue_type {
                eggsec::browser::client_checks::ClientIssueType::LocalStorageSensitive => {
                    ClientIssueTypePy::LocalStorageSensitive
                }
                eggsec::browser::client_checks::ClientIssueType::CorsMisconfiguration => {
                    ClientIssueTypePy::CorsMisconfiguration
                }
                eggsec::browser::client_checks::ClientIssueType::CSPSourceMap => {
                    ClientIssueTypePy::CSPSourceMap
                }
                eggsec::browser::client_checks::ClientIssueType::DebugMode => {
                    ClientIssueTypePy::DebugMode
                }
                eggsec::browser::client_checks::ClientIssueType::SourceMapsExposed => {
                    ClientIssueTypePy::SourceMapsExposed
                }
                eggsec::browser::client_checks::ClientIssueType::CORSWildcard => {
                    ClientIssueTypePy::CORSWildcard
                }
            },
            severity: Severity::from_engine(engine.severity),
            location: engine.location,
            description: engine.description,
            evidence: engine.evidence,
            remediation: engine.remediation,
            cvss_score: engine.cvss_score,
        }
    }
}

#[pymethods]
impl ClientIssuePy {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("id", &self.id)?;
        dict.set_item("issue_type", self.issue_type.as_str())?;
        dict.set_item("severity", self.severity.as_str())?;
        dict.set_item("location", &self.location)?;
        dict.set_item("description", &self.description)?;
        dict.set_item("evidence", &self.evidence)?;
        dict.set_item("remediation", &self.remediation)?;
        dict.set_item("cvss_score", self.cvss_score)?;
        Ok(dict.into())
    }

    fn __repr__(&self) -> String {
        format!(
            "ClientIssue(id={}, type={}, severity={})",
            self.id,
            self.issue_type.as_str(),
            self.severity.as_str()
        )
    }
}

/// Configuration for headless browser security testing.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct BrowserTestConfigPy {
    #[pyo3(get)]
    pub check_dom_xss: bool,
    #[pyo3(get)]
    pub discover_spa_routes: bool,
    #[pyo3(get)]
    pub check_client_security: bool,
    #[pyo3(get)]
    pub timeout_ms: u64,
    #[pyo3(get)]
    pub xss_payload: String,
}

impl Default for BrowserTestConfigPy {
    fn default() -> Self {
        Self {
            check_dom_xss: true,
            discover_spa_routes: true,
            check_client_security: true,
            timeout_ms: 30000,
            xss_payload: "<img src=x onerror=alert(1)>".to_string(),
        }
    }
}

#[pymethods]
impl BrowserTestConfigPy {
    /// Create a new browser test configuration.
    ///
    /// Args:
    ///     check_dom_xss: Run DOM XSS detection (default: true).
    ///     discover_spa_routes: Discover SPA routes via interception (default: true).
    ///     check_client_security: Run client-side security checks (default: true).
    ///     timeout_ms: Browser timeout in milliseconds (default: 30000).
    ///     xss_payload: XSS payload for testing (default: "<img src=x onerror=alert(1)>").
    #[new]
    #[pyo3(signature = (*, check_dom_xss=true, discover_spa_routes=true, check_client_security=true, timeout_ms=30000, xss_payload="<img src=x onerror=alert(1)>"))]
    fn new(
        check_dom_xss: bool,
        discover_spa_routes: bool,
        check_client_security: bool,
        timeout_ms: u64,
        xss_payload: &str,
    ) -> PyResult<Self> {
        Ok(Self {
            check_dom_xss,
            discover_spa_routes,
            check_client_security,
            timeout_ms,
            xss_payload: xss_payload.to_string(),
        })
    }

    fn __repr__(&self) -> String {
        format!(
            "BrowserTestConfig(dom_xss={}, spa_routes={}, client_security={})",
            self.check_dom_xss, self.discover_spa_routes, self.check_client_security
        )
    }
}

/// Complete browser security test report.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserTestReportPy {
    #[pyo3(get)]
    pub target: String,
    dom_xss: Vec<DomXssFindingPy>,
    spa_routes: Vec<SpaRoutePy>,
    client_issues: Vec<ClientIssuePy>,
    #[pyo3(get)]
    pub total_findings: usize,
}

impl BrowserTestReportPy {
    fn from_engine(engine: eggsec::browser::BrowserReport) -> Self {
        Self {
            target: engine.target,
            dom_xss: engine
                .dom_xss
                .into_iter()
                .map(DomXssFindingPy::from_engine)
                .collect(),
            spa_routes: engine
                .spa_routes
                .into_iter()
                .map(SpaRoutePy::from_engine)
                .collect(),
            client_issues: engine
                .client_issues
                .into_iter()
                .map(ClientIssuePy::from_engine)
                .collect(),
            total_findings: engine.total_findings,
        }
    }
}

#[pymethods]
impl BrowserTestReportPy {
    #[getter]
    fn dom_xss(&self) -> Vec<DomXssFindingPy> {
        self.dom_xss.clone()
    }

    #[getter]
    fn spa_routes(&self) -> Vec<SpaRoutePy> {
        self.spa_routes.clone()
    }

    #[getter]
    fn client_issues(&self) -> Vec<ClientIssuePy> {
        self.client_issues.clone()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("target", &self.target)?;
        dict.set_item("total_findings", self.total_findings)?;

        let xss_list = PyList::empty_bound(py);
        for f in &self.dom_xss {
            xss_list.append(f.to_dict(py)?)?;
        }
        dict.set_item("dom_xss", xss_list)?;

        let routes_list = PyList::empty_bound(py);
        for r in &self.spa_routes {
            routes_list.append(r.__repr__())?;
        }
        dict.set_item("spa_routes", routes_list)?;

        let issues_list = PyList::empty_bound(py);
        for i in &self.client_issues {
            issues_list.append(i.to_dict(py)?)?;
        }
        dict.set_item("client_issues", issues_list)?;

        Ok(dict.into())
    }

    fn __repr__(&self) -> String {
        format!(
            "BrowserTestReport(target={}, findings={})",
            self.target, self.total_findings
        )
    }
}

/// Run headless browser security testing against a target.
///
/// Performs DOM XSS detection, SPA route discovery, and client-side security
/// checks using a headless Chrome instance.
///
/// Args:
///     target: Target URL (e.g. "https://example.com").
///     config: Browser test configuration (optional).
///
/// Returns:
///     BrowserTestReportPy: Full browser security report.
///
/// Raises:
///     FeatureUnavailableError: If headless-browser feature is not enabled.
///     NetworkError: If the target is unreachable.
#[pyfunction]
#[pyo3(signature = (target, config=None))]
pub fn browser_test(
    target: &str,
    config: Option<BrowserTestConfigPy>,
) -> PyResult<BrowserTestReportPy> {
    let cfg = config.unwrap_or_default();
    let target_owned = target.to_string();

    Python::with_gil(|py| {
        let result = runtime_sync::block_on(py, async move {
            let browser_config = eggsec::browser::BrowserConfig {
                check_dom_xss: cfg.check_dom_xss,
                discover_spa_routes: cfg.discover_spa_routes,
                check_client_security: cfg.check_client_security,
                timeout_ms: cfg.timeout_ms,
                xss_payload: cfg.xss_payload,
            };
            eggsec::browser::run_browser_scan(&target_owned, browser_config)
                .await
                .map_err(|e| {
                    pyo3::exceptions::PyRuntimeError::new_err(format!("Browser scan failed: {}", e))
                })
        })?;

        Ok(BrowserTestReportPy::from_engine(result))
    })
}

/// Run headless browser security testing (async).
///
/// Returns a PyFuture that resolves to a BrowserTestReportPy.
#[pyfunction]
#[pyo3(signature = (target, config=None))]
pub fn async_browser_test(
    target: &str,
    config: Option<BrowserTestConfigPy>,
) -> PyResult<crate::runtime_async::PyFuture> {
    let cfg = config.unwrap_or_default();
    let target_owned = target.to_string();

    crate::runtime_async::spawn_async(async move {
        let browser_config = eggsec::browser::BrowserConfig {
            check_dom_xss: cfg.check_dom_xss,
            discover_spa_routes: cfg.discover_spa_routes,
            check_client_security: cfg.check_client_security,
            timeout_ms: cfg.timeout_ms,
            xss_payload: cfg.xss_payload,
        };
        let report = eggsec::browser::run_browser_scan(&target_owned, browser_config)
            .await
            .map_err(|e| {
                pyo3::exceptions::PyRuntimeError::new_err(format!("Browser scan failed: {}", e))
            })?;
        Ok(BrowserTestReportPy::from_engine(report))
    })
}
