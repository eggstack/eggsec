use pyo3::prelude::*;
use pyo3::types::PyDict;
use serde::{Deserialize, Serialize};

use crate::error::EggsecResultExt;
use crate::runtime_async;
use crate::runtime_sync;
use crate::scope::Scope;

fn extract_host_from_url(url: &str) -> PyResult<String> {
    let parsed = url::Url::parse(url)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("Invalid URL: {}", e)))?;
    parsed
        .host_str()
        .map(|h| h.to_string())
        .ok_or_else(|| pyo3::exceptions::PyValueError::new_err("URL does not contain a valid host"))
}

// ═══════════════════════════════════════════════════════════════════
// WAF Validation Types
// ═══════════════════════════════════════════════════════════════════

/// Result of a single WAF bypass attempt.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BypassResultPy {
    #[pyo3(get)]
    pub technique: String,
    #[pyo3(get)]
    pub success: bool,
    #[pyo3(get)]
    pub description: String,
    #[pyo3(get)]
    pub payload: Option<String>,
    #[pyo3(get)]
    pub status_code: u16,
    #[pyo3(get)]
    pub error: Option<String>,
}

#[pymethods]
impl BypassResultPy {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("technique", &self.technique)?;
        dict.set_item("success", self.success)?;
        dict.set_item("description", &self.description)?;
        dict.set_item("payload", &self.payload)?;
        dict.set_item("status_code", self.status_code)?;
        dict.set_item("error", &self.error)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "BypassResult(technique={}, success={}, status={})",
            self.technique, self.success, self.status_code
        )
    }

    fn __str__(&self) -> String {
        if self.success {
            format!("Bypass successful: {}", self.technique)
        } else {
            format!("Bypass blocked: {}", self.technique)
        }
    }
}

/// Summary of a WAF validation scan.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WafScanResultPy {
    #[pyo3(get)]
    pub target: String,
    #[pyo3(get)]
    pub waf_detected: bool,
    #[pyo3(get)]
    pub waf_name: Option<String>,
    #[pyo3(get)]
    pub confidence: u8,
    #[pyo3(get)]
    pub duration_ms: u64,
    #[pyo3(get)]
    pub bypasses_tested: usize,
    #[pyo3(get)]
    pub bypasses_successful: usize,
    pub(crate) bypass_results: Vec<BypassResultPy>,
}

#[pymethods]
impl WafScanResultPy {
    #[getter]
    fn bypass_results(&self) -> Vec<BypassResultPy> {
        self.bypass_results.clone()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("target", &self.target)?;
        dict.set_item("waf_detected", self.waf_detected)?;
        dict.set_item("waf_name", &self.waf_name)?;
        dict.set_item("confidence", self.confidence)?;
        dict.set_item("duration_ms", self.duration_ms)?;
        dict.set_item("bypasses_tested", self.bypasses_tested)?;
        dict.set_item("bypasses_successful", self.bypasses_successful)?;
        let bypass_list: Vec<PyObject> = self
            .bypass_results
            .iter()
            .map(|r| r.to_dict(py))
            .collect::<PyResult<Vec<_>>>()?;
        dict.set_item("bypass_results", bypass_list)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "WafScanResult(target={}, waf={}, bypasses={}/{})",
            self.target, self.waf_detected, self.bypasses_successful, self.bypasses_tested
        )
    }

    fn __str__(&self) -> String {
        let waf_info = match &self.waf_name {
            Some(name) => format!("WAF: {} ({}%)", name, self.confidence),
            None => "No WAF detected".to_string(),
        };
        format!(
            "{} — {}/{} bypasses successful",
            waf_info, self.bypasses_successful, self.bypasses_tested
        )
    }
}

// ═══════════════════════════════════════════════════════════════════
// HTTP Fuzzing Types
// ═══════════════════════════════════════════════════════════════════

/// A single fuzzing payload.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayloadPy {
    #[pyo3(get)]
    pub payload_type: String,
    #[pyo3(get)]
    pub payload: String,
    #[pyo3(get)]
    pub description: String,
    #[pyo3(get)]
    pub severity: String,
    #[pyo3(get)]
    pub tags: Vec<String>,
}

#[pymethods]
impl PayloadPy {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("payload_type", &self.payload_type)?;
        dict.set_item("payload", &self.payload)?;
        dict.set_item("description", &self.description)?;
        dict.set_item("severity", &self.severity)?;
        dict.set_item("tags", &self.tags)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "Payload(type={}, severity={})",
            self.payload_type, self.severity
        )
    }

    fn __str__(&self) -> String {
        format!("[{}] {}", self.severity, self.description)
    }
}

/// Result of a single fuzz request.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuzzResultPy {
    #[pyo3(get)]
    pub payload: String,
    #[pyo3(get)]
    pub payload_type: String,
    #[pyo3(get)]
    pub status_code: u16,
    #[pyo3(get)]
    pub response_time_ms: u64,
    #[pyo3(get)]
    pub is_vulnerable: bool,
    #[pyo3(get)]
    pub is_waf_blocked: bool,
    #[pyo3(get)]
    pub leaks_found: Vec<String>,
    #[pyo3(get)]
    pub error: Option<String>,
}

#[pymethods]
impl FuzzResultPy {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("payload", &self.payload)?;
        dict.set_item("payload_type", &self.payload_type)?;
        dict.set_item("status_code", self.status_code)?;
        dict.set_item("response_time_ms", self.response_time_ms)?;
        dict.set_item("is_vulnerable", self.is_vulnerable)?;
        dict.set_item("is_waf_blocked", self.is_waf_blocked)?;
        dict.set_item("leaks_found", &self.leaks_found)?;
        dict.set_item("error", &self.error)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "FuzzResult(status={}, vulnerable={})",
            self.status_code, self.is_vulnerable
        )
    }

    fn __str__(&self) -> String {
        if self.is_vulnerable {
            format!("VULNERABLE at status {}", self.status_code)
        } else {
            format!("Status {}", self.status_code)
        }
    }
}

/// Complete fuzzing session results.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuzzSessionPy {
    #[pyo3(get)]
    pub target_url: String,
    #[pyo3(get)]
    pub payload_type: String,
    #[pyo3(get)]
    pub total_payloads: usize,
    #[pyo3(get)]
    pub successful_requests: usize,
    #[pyo3(get)]
    pub failed_requests: usize,
    #[pyo3(get)]
    pub waf_bypasses: usize,
    #[pyo3(get)]
    pub potential_leaks: usize,
    #[pyo3(get)]
    pub time_anomalies: usize,
    #[pyo3(get)]
    pub redos_suspected: usize,
    #[pyo3(get)]
    pub duration_ms: u64,
    #[pyo3(get)]
    pub total_requests: usize,
    pub(crate) results: Vec<FuzzResultPy>,
}

#[pymethods]
impl FuzzSessionPy {
    #[getter]
    fn results(&self) -> Vec<FuzzResultPy> {
        self.results.clone()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("target_url", &self.target_url)?;
        dict.set_item("payload_type", &self.payload_type)?;
        dict.set_item("total_payloads", self.total_payloads)?;
        dict.set_item("successful_requests", self.successful_requests)?;
        dict.set_item("failed_requests", self.failed_requests)?;
        dict.set_item("waf_bypasses", self.waf_bypasses)?;
        dict.set_item("potential_leaks", self.potential_leaks)?;
        dict.set_item("time_anomalies", self.time_anomalies)?;
        dict.set_item("redos_suspected", self.redos_suspected)?;
        dict.set_item("duration_ms", self.duration_ms)?;
        dict.set_item("total_requests", self.total_requests)?;
        let results_list: Vec<PyObject> = self
            .results
            .iter()
            .map(|r| r.to_dict(py))
            .collect::<PyResult<Vec<_>>>()?;
        dict.set_item("results", results_list)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "FuzzSession(url={}, payloads={}, leaks={})",
            self.target_url, self.total_payloads, self.potential_leaks
        )
    }

    fn __str__(&self) -> String {
        format!(
            "Fuzz session: {} payloads → {} successful, {} potential leaks",
            self.total_payloads, self.successful_requests, self.potential_leaks
        )
    }
}

/// Python-facing fuzzing configuration.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct FuzzConfig {
    #[pyo3(get)]
    pub url: String,
    #[pyo3(get)]
    pub payload_type: String,
    #[pyo3(get)]
    pub method: String,
    #[pyo3(get)]
    pub param: Option<String>,
    #[pyo3(get)]
    pub concurrency: usize,
    #[pyo3(get)]
    pub timeout: u64,
}

#[pymethods]
impl FuzzConfig {
    #[new]
    #[pyo3(signature = (url, payload_type="all", method="GET", param=None, concurrency=10, timeout=30))]
    fn new(
        url: &str,
        payload_type: &str,
        method: &str,
        param: Option<&str>,
        concurrency: usize,
        timeout: u64,
    ) -> Self {
        Self {
            url: url.to_string(),
            payload_type: payload_type.to_string(),
            method: method.to_string(),
            param: param.map(|s| s.to_string()),
            concurrency,
            timeout,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("url", &self.url)?;
        dict.set_item("payload_type", &self.payload_type)?;
        dict.set_item("method", &self.method)?;
        dict.set_item("param", &self.param)?;
        dict.set_item("concurrency", self.concurrency)?;
        dict.set_item("timeout", self.timeout)?;
        Ok(dict.into())
    }

    fn __repr__(&self) -> String {
        format!(
            "FuzzConfig(url={}, type={}, method={})",
            self.url, self.payload_type, self.method
        )
    }

    fn __str__(&self) -> String {
        format!(
            "Fuzz {} {} {} (concurrency={})",
            self.method, self.url, self.payload_type, self.concurrency
        )
    }
}

// ═══════════════════════════════════════════════════════════════════
// Payload generation (sync, no I/O)
// ═══════════════════════════════════════════════════════════════════

fn parse_payload_type(s: &str) -> PyResult<eggsec::fuzzer::PayloadType> {
    match s.to_lowercase().as_str() {
        "sqli" | "sql" => Ok(eggsec::fuzzer::PayloadType::Sqli),
        "xss" => Ok(eggsec::fuzzer::PayloadType::Xss),
        "traversal" | "lfi" => Ok(eggsec::fuzzer::PayloadType::Traversal),
        "ssrf" => Ok(eggsec::fuzzer::PayloadType::Ssrf),
        "redirect" => Ok(eggsec::fuzzer::PayloadType::Redirect),
        "redos" => Ok(eggsec::fuzzer::PayloadType::Redos),
        "headers" => Ok(eggsec::fuzzer::PayloadType::Headers),
        "compression" => Ok(eggsec::fuzzer::PayloadType::Compression),
        "graphql" => Ok(eggsec::fuzzer::PayloadType::GraphQL),
        "oauth" => Ok(eggsec::fuzzer::PayloadType::OAuth),
        "jwt" => Ok(eggsec::fuzzer::PayloadType::Jwt),
        "idor" => Ok(eggsec::fuzzer::PayloadType::Idor),
        "ssti" => Ok(eggsec::fuzzer::PayloadType::Ssti),
        "grpc" => Ok(eggsec::fuzzer::PayloadType::Grpc),
        "xxe" => Ok(eggsec::fuzzer::PayloadType::Xxe),
        "ldap" => Ok(eggsec::fuzzer::PayloadType::Ldap),
        "cmd" | "rce" | "command" => Ok(eggsec::fuzzer::PayloadType::Cmd),
        "deser" | "deserialization" => Ok(eggsec::fuzzer::PayloadType::Deser),
        "host" => Ok(eggsec::fuzzer::PayloadType::Host),
        "cache" => Ok(eggsec::fuzzer::PayloadType::Cache),
        "csv" => Ok(eggsec::fuzzer::PayloadType::Csv),
        "soap" => Ok(eggsec::fuzzer::PayloadType::Soap),
        "websocket" | "ws" => Ok(eggsec::fuzzer::PayloadType::Websocket),
        "nosql" => Ok(eggsec::fuzzer::PayloadType::Nosql),
        "xpath" => Ok(eggsec::fuzzer::PayloadType::Xpath),
        "expression" => Ok(eggsec::fuzzer::PayloadType::Expression),
        "prototype" => Ok(eggsec::fuzzer::PayloadType::Prototype),
        "race" => Ok(eggsec::fuzzer::PayloadType::Race),
        "massassign" | "mass_assign" => Ok(eggsec::fuzzer::PayloadType::MassAssign),
        "oast" => Ok(eggsec::fuzzer::PayloadType::Oast),
        "saml" => Ok(eggsec::fuzzer::PayloadType::Saml),
        "html" | "html_inject" => Ok(eggsec::fuzzer::PayloadType::HtmlInject),
        "css" | "css_inject" => Ok(eggsec::fuzzer::PayloadType::CssInject),
        "ssi" => Ok(eggsec::fuzzer::PayloadType::Ssi),
        "dom_clobber" | "domclobber" => Ok(eggsec::fuzzer::PayloadType::DomClobber),
        "xslt" => Ok(eggsec::fuzzer::PayloadType::Xslt),
        "viewstate" => Ok(eggsec::fuzzer::PayloadType::Viewstate),
        "dep_confusion" | "dependency" => Ok(eggsec::fuzzer::PayloadType::DepConfusion),
        "xs_leak" | "xsleak" => Ok(eggsec::fuzzer::PayloadType::XsLeak),
        "latex" => Ok(eggsec::fuzzer::PayloadType::Latex),
        _ => Err(pyo3::exceptions::PyValueError::new_err(format!(
            "Unknown payload type: '{}'. Valid: sqli, xss, traversal, ssrf, redirect, redos, \
             headers, compression, graphql, oauth, jwt, idor, ssti, grpc, xxe, ldap, cmd, deser, \
             host, cache, csv, soap, websocket, nosql, xpath, expression, prototype, race, \
             massassign, oast, saml, html, css, ssi, dom_clobber, xslt, viewstate, dep_confusion, \
             xs_leak, latex",
            s
        ))),
    }
}

fn parse_test_type(s: &str) -> eggsec::waf::TestType {
    match s.to_lowercase().as_str() {
        "sqli" | "sql" => eggsec::waf::TestType::Sql,
        "xss" => eggsec::waf::TestType::Xss,
        "ssrf" => eggsec::waf::TestType::Ssrf,
        "cmd" | "command" => eggsec::waf::TestType::Cmd,
        "traversal" | "lfi" => eggsec::waf::TestType::Traversal,
        _ => eggsec::waf::TestType::All,
    }
}

/// Generate fuzzing payloads for a given type (no I/O).
///
/// Args:
///     payload_type: Type of payloads ("sqli", "xss", "traversal", etc.)
///
/// Returns:
///     List of PayloadPy objects.
#[pyfunction]
pub fn generate_fuzz_payloads(payload_type: &str) -> PyResult<Vec<PayloadPy>> {
    let pt = parse_payload_type(payload_type)?;
    let payloads = eggsec::fuzzer::get_payloads(pt);
    Ok(payloads
        .into_iter()
        .map(|p| PayloadPy {
            payload_type: format!("{}", p.payload_type),
            payload: p.payload,
            description: p.description,
            severity: format!("{:?}", p.severity),
            tags: p.tags,
        })
        .collect())
}

// ═══════════════════════════════════════════════════════════════════
// WAF validation functions
// ═══════════════════════════════════════════════════════════════════

/// Validate WAF protection and optionally test bypass techniques.
///
/// Detects the WAF on the target URL and, if bypass=True, runs bypass
/// attempts against the detected WAF.
///
/// Args:
///     url: Target URL (e.g. "https://example.com").
///     bypass: Run bypass testing after detection.
///     test_type: Payload type filter ("sqli", "xss", "ssrf", "cmd", "traversal", "all").
///
/// Returns:
///     WafScanResultPy: Detection result with optional bypass results.
#[pyfunction]
#[pyo3(signature = (url, scope, *, bypass=false, test_type=None))]
pub fn validate_waf(
    url: &str,
    scope: Scope,
    bypass: bool,
    test_type: Option<&str>,
) -> PyResult<WafScanResultPy> {
    let host = extract_host_from_url(url)?;
    scope.enforce_target(&host)?;

    Python::with_gil(|py| {
        let url_owned = url.to_string();
        let test_type_owned = test_type.map(|s| s.to_string());

        let result = runtime_sync::block_on(py, async move {
            let start = std::time::Instant::now();

            let detector = eggsec::waf::WafDetector::new().map_pyerr()?;
            let detection = detector.detect(&url_owned).await.map_pyerr()?;

            let mut bypass_results = Vec::new();
            if bypass {
                let tt = match test_type_owned.as_deref() {
                    Some(t) => parse_test_type(t),
                    None => eggsec::waf::TestType::All,
                };
                let profile = eggsec::waf::get_auto_profile();
                let waf_args = eggsec::cli::WafArgs {
                    url: url_owned.clone(),
                    detect_only: false,
                    bypass: true,
                    header_bypass: false,
                    smuggling: false,
                    evasion: false,
                    profile: "auto".to_string(),
                    test_type: test_type_owned.clone(),
                    concurrency: 10,
                    timeout: 30,
                    json: false,
                    verbose: false,
                    quiet: true,
                    output: None,
                    common: eggsec::cli::CommonHttpArgs::default(),
                };
                let bypass_engine =
                    eggsec::waf::BypassEngine::new(&waf_args, Some(profile), tt).map_pyerr()?;
                let results = bypass_engine.run_bypasses(&detection).await.map_pyerr()?;
                bypass_results = results
                    .into_iter()
                    .map(|r| BypassResultPy {
                        technique: format!("{:?}", r.technique),
                        success: r.success,
                        description: r.description,
                        payload: r.payload,
                        status_code: r.status_code,
                        error: r.error,
                    })
                    .collect();
            }

            let duration_ms = u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX);
            let successful = bypass_results.iter().filter(|r| r.success).count();

            Ok::<WafScanResultPy, PyErr>(WafScanResultPy {
                target: url_owned,
                waf_detected: detection.waf_name.is_some(),
                waf_name: detection.waf_name,
                confidence: detection.confidence,
                duration_ms,
                bypasses_tested: bypass_results.len(),
                bypasses_successful: successful,
                bypass_results,
            })
        })?;

        Ok(result)
    })
}

/// Async version of validate_waf.
///
/// Returns a PyFuture that can be awaited in Python.
#[pyfunction]
#[pyo3(signature = (url, scope, *, bypass=false, test_type=None))]
pub fn async_validate_waf(
    url: &str,
    scope: Scope,
    bypass: bool,
    test_type: Option<&str>,
) -> PyResult<crate::runtime_async::PyFuture> {
    let host = extract_host_from_url(url)?;
    scope.enforce_target(&host)?;

    let url_owned = url.to_string();
    let test_type_owned = test_type.map(|s| s.to_string());

    runtime_async::spawn_async(async move {
        let start = std::time::Instant::now();

        let detector = eggsec::waf::WafDetector::new().map_pyerr()?;
        let detection = detector.detect(&url_owned).await.map_pyerr()?;

        let mut bypass_results = Vec::new();
        if bypass {
            let tt = match test_type_owned.as_deref() {
                Some(t) => parse_test_type(t),
                None => eggsec::waf::TestType::All,
            };
            let profile = eggsec::waf::get_auto_profile();
            let waf_args = eggsec::cli::WafArgs {
                url: url_owned.clone(),
                detect_only: false,
                bypass: true,
                header_bypass: false,
                smuggling: false,
                evasion: false,
                profile: "auto".to_string(),
                test_type: test_type_owned.clone(),
                concurrency: 10,
                timeout: 30,
                json: false,
                verbose: false,
                quiet: true,
                output: None,
                common: eggsec::cli::CommonHttpArgs::default(),
            };
            let bypass_engine =
                eggsec::waf::BypassEngine::new(&waf_args, Some(profile), tt).map_pyerr()?;
            let results = bypass_engine.run_bypasses(&detection).await.map_pyerr()?;
            bypass_results = results
                .into_iter()
                .map(|r| BypassResultPy {
                    technique: format!("{:?}", r.technique),
                    success: r.success,
                    description: r.description,
                    payload: r.payload,
                    status_code: r.status_code,
                    error: r.error,
                })
                .collect();
        }

        let duration_ms = u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX);
        let successful = bypass_results.iter().filter(|r| r.success).count();

        Ok(WafScanResultPy {
            target: url_owned,
            waf_detected: detection.waf_name.is_some(),
            waf_name: detection.waf_name,
            confidence: detection.confidence,
            duration_ms,
            bypasses_tested: bypass_results.len(),
            bypasses_successful: successful,
            bypass_results,
        })
    })
}

// ═══════════════════════════════════════════════════════════════════
// HTTP fuzzing functions
// ═══════════════════════════════════════════════════════════════════

/// Run an HTTP fuzzing session.
///
/// Args:
///     url: Target URL.
///     payload_type: Fuzzing payload type ("sqli", "xss", "all", etc.)
///     method: HTTP method (default: "GET").
///     param: Parameter name to inject into (optional).
///     concurrency: Max concurrent requests (default: 10).
///     timeout: Request timeout in seconds (default: 30).
///
/// Returns:
///     FuzzSessionPy: Complete fuzzing session results.
#[pyfunction]
#[pyo3(signature = (url, scope, payload_type="all", *, method="GET", param=None, concurrency=10, timeout=30))]
pub fn fuzz_http(
    url: &str,
    scope: Scope,
    payload_type: &str,
    method: &str,
    param: Option<&str>,
    concurrency: usize,
    timeout: u64,
) -> PyResult<FuzzSessionPy> {
    if concurrency == 0 {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "concurrency must be > 0",
        ));
    }
    if timeout == 0 {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "timeout must be > 0",
        ));
    }

    let host = extract_host_from_url(url)?;
    scope.enforce_target(&host)?;

    Python::with_gil(|py| {
        let url_owned = url.to_string();
        let pt_owned = payload_type.to_string();
        let method_owned = method.to_string();
        let param_owned = param.map(|s| s.to_string());

        let session = runtime_sync::block_on(py, async move {
            let fuzz_args = eggsec::cli::FuzzArgs {
                url: url_owned,
                payload_type: pt_owned,
                mode: eggsec::cli::FuzzMode::Sequential,
                mutate: false,
                mutation_count: 3,
                grammar_fuzz: false,
                grammar_type: None,
                adaptive_rate: false,
                session: false,
                diffing: false,
                capture_baseline: false,
                enhanced_redos: false,
                waf_fingerprint: false,
                chaining: false,
                chain_file: None,
                method: method_owned,
                param: param_owned,
                concurrency,
                timeout,
                json: false,
                output: None,
                verbose: false,
                quiet: true,
                format: None,
                target: None,
                jwt_token: None,
                oauth_issuer: None,
                oauth_client_id: None,
                oauth_client_secret: None,
                idor_base_id: None,
                idor_user_ids: None,
                ssti_param: None,
                graphql_introspection: true,
                graphql_depth_bypass: true,
                graphql_alias_overload: true,
                oauth_redirect: true,
                oauth_scope: true,
                oauth_state: true,
                oauth_grant: true,
                schema: None,
                discover_only: false,
                auto_discover_schema: false,
                calibrate: false,
                fc: None,
                fs: None,
                fw: None,
                fl: None,
                ft: None,
                fr: None,
                common: eggsec::cli::CommonHttpArgs::default(),
            };

            let mut engine = eggsec::fuzzer::FuzzEngine::new(fuzz_args).map_pyerr()?;
            engine.run_return_session().await.map_pyerr()
        })?;

        Ok(session_to_py(session))
    })
}

/// Async version of fuzz_http.
///
/// Returns a PyFuture that can be awaited in Python.
#[pyfunction]
#[pyo3(signature = (url, scope, payload_type="all", *, method="GET", param=None, concurrency=10, timeout=30))]
pub fn async_fuzz_http(
    url: &str,
    scope: Scope,
    payload_type: &str,
    method: &str,
    param: Option<&str>,
    concurrency: usize,
    timeout: u64,
) -> PyResult<crate::runtime_async::PyFuture> {
    if concurrency == 0 {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "concurrency must be > 0",
        ));
    }
    if timeout == 0 {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "timeout must be > 0",
        ));
    }

    let host = extract_host_from_url(url)?;
    scope.enforce_target(&host)?;

    let url_owned = url.to_string();
    let pt_owned = payload_type.to_string();
    let method_owned = method.to_string();
    let param_owned = param.map(|s| s.to_string());

    runtime_async::spawn_async(async move {
        let fuzz_args = eggsec::cli::FuzzArgs {
            url: url_owned,
            payload_type: pt_owned,
            mode: eggsec::cli::FuzzMode::Sequential,
            mutate: false,
            mutation_count: 3,
            grammar_fuzz: false,
            grammar_type: None,
            adaptive_rate: false,
            session: false,
            diffing: false,
            capture_baseline: false,
            enhanced_redos: false,
            waf_fingerprint: false,
            chaining: false,
            chain_file: None,
            method: method_owned,
            param: param_owned,
            concurrency,
            timeout,
            json: false,
            output: None,
            verbose: false,
            quiet: true,
            format: None,
            target: None,
            jwt_token: None,
            oauth_issuer: None,
            oauth_client_id: None,
            oauth_client_secret: None,
            idor_base_id: None,
            idor_user_ids: None,
            ssti_param: None,
            graphql_introspection: true,
            graphql_depth_bypass: true,
            graphql_alias_overload: true,
            oauth_redirect: true,
            oauth_scope: true,
            oauth_state: true,
            oauth_grant: true,
            schema: None,
            discover_only: false,
            auto_discover_schema: false,
            calibrate: false,
            fc: None,
            fs: None,
            fw: None,
            fl: None,
            ft: None,
            fr: None,
            common: eggsec::cli::CommonHttpArgs::default(),
        };

        let mut engine = eggsec::fuzzer::FuzzEngine::new(fuzz_args).map_pyerr()?;
        let session = engine.run_return_session().await.map_pyerr()?;

        Ok(session_to_py(session))
    })
}

// ═══════════════════════════════════════════════════════════════════
// Internal conversion helpers
// ═══════════════════════════════════════════════════════════════════

fn session_to_py(session: eggsec::fuzzer::FuzzSession) -> FuzzSessionPy {
    FuzzSessionPy {
        target_url: session.target_url,
        payload_type: session.payload_type,
        total_payloads: session.total_payloads,
        successful_requests: session.successful_requests,
        failed_requests: session.failed_requests,
        waf_bypasses: session.waf_bypasses,
        potential_leaks: session.potential_leaks,
        time_anomalies: session.time_anomalies,
        redos_suspected: session.redos_suspected,
        duration_ms: session.duration_ms,
        total_requests: session.total_requests,
        results: session
            .results
            .into_iter()
            .map(|r| {
                let is_vuln = r.is_vulnerable();
                let pt = format!("{}", r.payload.payload_type);
                let payload_str = r.payload.payload;
                FuzzResultPy {
                    payload: payload_str,
                    payload_type: pt,
                    status_code: r.status_code,
                    response_time_ms: r.response_time_ms,
                    is_vulnerable: is_vuln,
                    is_waf_blocked: r.is_waf_blocked,
                    leaks_found: r.leaks_found,
                    error: r.error,
                }
            })
            .collect(),
    }
}
