use pyo3::prelude::*;

use crate::dto::PortScanResult;
use crate::endpoint::EndpointScanResult;
use crate::error::EggsecResultExt;
use crate::fingerprint::FingerprintScanResult;
use crate::recon::{DnsRecordSet, TechDetectionResult, TlsInspectionResult};
use crate::requests::*;
use crate::runtime_async;
use crate::scope::Scope;
use crate::status::{ExecutionStats, ExecutionStatus, OperationResult};
use crate::waf::WafDetectionResultPy;

/// Extract hostname from a URL for scope enforcement.
fn extract_host_from_url(url: &str) -> PyResult<String> {
    let parsed = url::Url::parse(url)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("Invalid URL: {}", e)))?;
    parsed
        .host_str()
        .map(|h| h.to_string())
        .ok_or_else(|| pyo3::exceptions::PyValueError::new_err("URL does not contain a valid host"))
}

/// Parse a comma-separated ports string into a Vec<u16>.
fn parse_ports_string(ports: &str) -> PyResult<Vec<u16>> {
    let mut result = Vec::new();
    for part in ports.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if let Some((start_str, end_str)) = part.split_once('-') {
            let start: u16 = start_str.trim().parse().map_err(|_| {
                pyo3::exceptions::PyValueError::new_err(format!(
                    "Invalid port range start: {}",
                    start_str
                ))
            })?;
            let end: u16 = end_str.trim().parse().map_err(|_| {
                pyo3::exceptions::PyValueError::new_err(format!(
                    "Invalid port range end: {}",
                    end_str
                ))
            })?;
            if start > end {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "Invalid port range: {}-{}",
                    start, end
                )));
            }
            for port in start..=end {
                result.push(port);
            }
        } else {
            let port: u16 = part.parse().map_err(|_| {
                pyo3::exceptions::PyValueError::new_err(format!("Invalid port: {}", part))
            })?;
            result.push(port);
        }
    }
    Ok(result)
}

/// Build an OperationResult from a successful engine call.
fn operation_ok(
    stats: ExecutionStats,
    metadata: Option<std::collections::HashMap<String, String>>,
) -> OperationResult {
    OperationResult::new(
        ExecutionStatus::Completed(),
        Some(stats),
        None,
        None,
        metadata,
    )
}

/// Build an OperationResult from an error.
fn operation_err(error: String) -> OperationResult {
    OperationResult::new(
        ExecutionStatus::Failed {
            error: error.clone(),
        },
        None,
        None,
        Some(error),
        None,
    )
}

/// Async engine for running scoped security operations.
///
/// Provides the same operations as Engine but returns Python awaitables.
/// Each async operation spawns a background thread with its own Tokio runtime.
#[pyclass]
pub struct AsyncEngine {
    scope: Scope,
    mode: String,
    concurrency: usize,
    timeout_ms: u64,
}

#[pymethods]
impl AsyncEngine {
    /// Create a new async engine.
    ///
    /// Args:
    ///     scope: Scope defining authorized targets and ports.
    ///     mode: Execution mode ("manual" or "automation").
    ///     concurrency: Max concurrent connections (default: 100).
    ///     timeout_ms: Connection timeout in milliseconds (default: 5000).
    ///
    /// Raises:
    ///     ValueError: If mode is not "manual" or "automation".
    #[new]
    #[pyo3(signature = (scope, *, mode="manual", concurrency=100, timeout_ms=5000))]
    fn new(scope: Scope, mode: &str, concurrency: usize, timeout_ms: u64) -> PyResult<Self> {
        Self::new_inner(scope, mode, concurrency, timeout_ms)
    }

    /// Dispatch a generic operation request to the appropriate engine function.
    ///
    /// Returns a PyFuture that resolves to an OperationResult.
    fn run(&self, request: OperationRequest) -> PyResult<runtime_async::PyFuture> {
        self.dispatch_async(request)
    }

    /// Run a port scan (async).
    #[pyo3(signature = (request,))]
    fn run_port_scan(&self, request: PortScanRequest) -> PyResult<runtime_async::PyFuture> {
        let effective_timeout = request.timeout_ms.unwrap_or(self.timeout_ms);
        let ports_str = request
            .ports
            .clone()
            .unwrap_or_else(|| "1-1024".to_string());
        let ports = parse_ports_string(&ports_str)?;
        self.run_port_scan_async(request.target.clone(), ports, effective_timeout)
    }

    /// Run an endpoint scan (async).
    #[pyo3(signature = (request,))]
    fn run_endpoint_scan(&self, request: EndpointScanRequest) -> PyResult<runtime_async::PyFuture> {
        let endpoints = request.paths.clone().unwrap_or_default();
        let effective_timeout = request.timeout_ms.unwrap_or(self.timeout_ms);
        self.run_endpoint_scan_async(request.target.clone(), endpoints, effective_timeout)
    }

    /// Run service fingerprinting (async).
    #[pyo3(signature = (request,))]
    fn run_fingerprint(&self, request: FingerprintRequest) -> PyResult<runtime_async::PyFuture> {
        let ports = request.ports.clone().unwrap_or_else(|| vec![80, 443]);
        let effective_timeout = request.timeout_ms.unwrap_or(self.timeout_ms);
        self.run_fingerprint_async(request.target.clone(), ports, effective_timeout)
    }

    /// Run DNS reconnaissance (async).
    #[pyo3(signature = (request,))]
    fn run_recon_dns(&self, request: ReconDnsRequest) -> PyResult<runtime_async::PyFuture> {
        self.run_recon_dns_async(request.target.clone())
    }

    /// Run TLS inspection (async).
    #[pyo3(signature = (request,))]
    fn run_tls_inspect(&self, request: TlsInspectRequest) -> PyResult<runtime_async::PyFuture> {
        self.run_tls_inspect_async(request.target.clone())
    }

    /// Run technology detection (async).
    #[pyo3(signature = (request,))]
    fn run_tech_detect(&self, request: TechDetectRequest) -> PyResult<runtime_async::PyFuture> {
        self.run_tech_detect_async(request.target.clone())
    }

    /// Run WAF detection (async).
    #[pyo3(signature = (request,))]
    fn run_waf_detect(&self, request: WafDetectRequest) -> PyResult<runtime_async::PyFuture> {
        self.run_waf_detect_async(request.target.clone())
    }

    /// Run an HTTP load test (async).
    #[pyo3(signature = (request,))]
    fn run_load_test(&self, request: LoadTestRequest) -> PyResult<runtime_async::PyFuture> {
        let total_requests = request.requests.unwrap_or(100) as u64;
        let concurrency = request.concurrency.unwrap_or(self.concurrency as u32) as usize;
        let method = request.method.clone().unwrap_or_else(|| "GET".to_string());
        let timeout_secs = request.timeout_ms.map(|ms| ms / 1000).unwrap_or(30);
        self.run_load_test_async(
            request.target.clone(),
            total_requests,
            concurrency,
            timeout_secs,
            method,
        )
    }

    /// Run WAF validation (async).
    #[pyo3(signature = (request,))]
    fn run_waf_validate(&self, request: WafValidateRequest) -> PyResult<runtime_async::PyFuture> {
        self.run_waf_validate_async(request.target.clone())
    }

    /// Run HTTP fuzzing (async).
    #[pyo3(signature = (request,))]
    fn run_fuzz(&self, request: FuzzRequest) -> PyResult<runtime_async::PyFuture> {
        let payload_type = request
            .payload_type
            .clone()
            .unwrap_or_else(|| "all".to_string());
        let threads = request.threads.unwrap_or(10) as usize;
        let timeout = request.timeout_ms.map(|ms| ms / 1000).unwrap_or(30);
        self.run_fuzz_async(request.target.clone(), payload_type, threads, timeout)
    }

    /// Create a scan plan suggesting what operations to run against a target (async).
    fn plan(&self, target: &str) -> PyResult<runtime_async::PyFuture> {
        let plan = self.plan_inner(target)?;
        runtime_async::spawn_async(async move { Ok(plan) })
    }

    /// Get the engine's scope.
    #[getter]
    fn scope(&self) -> Scope {
        self.scope.clone()
    }

    /// Get the engine's mode.
    #[getter]
    fn mode(&self) -> String {
        self.mode.clone()
    }

    /// Get the engine's concurrency.
    #[getter]
    fn concurrency(&self) -> usize {
        self.concurrency
    }

    /// Get the engine's timeout in milliseconds.
    #[getter]
    fn timeout_ms(&self) -> u64 {
        self.timeout_ms
    }

    /// Close the engine (no-op).
    fn close(&self) {}

    /// Async context manager __aenter__.
    fn __aenter__(slf: Py<Self>) -> Py<Self> {
        slf
    }

    /// Async context manager __aexit__.
    #[pyo3(signature = (_exc_type=None, _exc_value=None, _traceback=None))]
    fn __aexit__(
        &self,
        _exc_type: Option<&Bound<'_, PyAny>>,
        _exc_value: Option<&Bound<'_, PyAny>>,
        _traceback: Option<&Bound<'_, PyAny>>,
    ) -> bool {
        false
    }

    fn __repr__(&self) -> String {
        format!(
            "AsyncEngine(mode={}, concurrency={})",
            self.mode, self.concurrency
        )
    }
}

// Internal constructor and helpers (not exposed to Python)
impl AsyncEngine {
    /// Internal constructor shared by AsyncEngine and AsyncClient.
    pub(crate) fn new_inner(
        scope: Scope,
        mode: &str,
        concurrency: usize,
        timeout_ms: u64,
    ) -> PyResult<Self> {
        if mode != "manual" && mode != "automation" {
            return Err(pyo3::exceptions::PyValueError::new_err(format!(
                "Invalid mode '{}'. Must be 'manual' or 'automation'.",
                mode
            )));
        }
        Ok(Self {
            scope,
            mode: mode.to_string(),
            concurrency,
            timeout_ms,
        })
    }

    /// Enforce that a target is within scope, raising EnforcementError if denied.
    pub(crate) fn enforce_target(&self, target: &str) -> PyResult<()> {
        self.scope.enforce_target(target)
    }

    /// Enforce that a port is within scope, raising EnforcementError if denied.
    pub(crate) fn enforce_port(&self, port: u16) -> PyResult<()> {
        self.scope.enforce_port(port)
    }

    /// Borrow the scope (immutable reference).
    pub(crate) fn scope_ref(&self) -> &Scope {
        &self.scope
    }

    /// Get the effective concurrency.
    pub(crate) fn get_concurrency(&self) -> usize {
        self.concurrency
    }

    /// Get the effective timeout in milliseconds.
    pub(crate) fn get_timeout_ms(&self) -> u64 {
        self.timeout_ms
    }

    /// Get the mode string.
    pub(crate) fn get_mode(&self) -> &str {
        &self.mode
    }

    /// Dispatch a generic operation request (used by async pipeline and other internal callers).
    pub(crate) fn dispatch_async(
        &self,
        request: OperationRequest,
    ) -> PyResult<runtime_async::PyFuture> {
        let op = request.operation.clone();
        match op.as_str() {
            "scan_ports" => {
                let target = request.target.clone();
                let ports_str = request
                    .metadata
                    .get("ports")
                    .cloned()
                    .unwrap_or_else(|| "1-1024".to_string());
                let effective_timeout = request.timeout_ms.unwrap_or(self.timeout_ms);
                let ports = parse_ports_string(&ports_str)?;
                self.run_port_scan_async(target, ports, effective_timeout)
            }
            "scan_endpoints" => {
                let target = request.target.clone();
                let endpoints: Vec<String> = request
                    .metadata
                    .get("endpoints")
                    .map(|s| s.split(',').map(|s| s.trim().to_string()).collect())
                    .unwrap_or_default();
                let effective_timeout = request.timeout_ms.unwrap_or(self.timeout_ms);
                self.run_endpoint_scan_async(target, endpoints, effective_timeout)
            }
            "fingerprint" => {
                let target = request.target.clone();
                let ports_str = request.metadata.get("ports").cloned().unwrap_or_default();
                let ports = if ports_str.is_empty() {
                    vec![80, 443]
                } else {
                    parse_ports_string(&ports_str)?
                };
                let effective_timeout = request.timeout_ms.unwrap_or(self.timeout_ms);
                self.run_fingerprint_async(target, ports, effective_timeout)
            }
            "recon_dns" => self.run_recon_dns_async(request.target.clone()),
            "tls_inspect" => self.run_tls_inspect_async(request.target.clone()),
            "tech_detect" => self.run_tech_detect_async(request.target.clone()),
            "waf_detect" => self.run_waf_detect_async(request.target.clone()),
            "load_test" => {
                let total_requests: u64 = request
                    .metadata
                    .get("requests")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(100);
                let concurrency: usize = request
                    .metadata
                    .get("concurrency")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(self.concurrency);
                let method = request
                    .metadata
                    .get("method")
                    .cloned()
                    .unwrap_or_else(|| "GET".to_string());
                let timeout_secs = request.timeout_ms.map(|ms| ms / 1000).unwrap_or(30);
                self.run_load_test_async(
                    request.target.clone(),
                    total_requests,
                    concurrency,
                    timeout_secs,
                    method,
                )
            }
            "waf_validate" => self.run_waf_validate_async(request.target.clone()),
            "fuzz" => {
                let payload_type = request.metadata.get("payload_type").cloned();
                let threads: usize = request
                    .metadata
                    .get("threads")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(10);
                let timeout = request.timeout_ms.map(|ms| ms / 1000).unwrap_or(30);
                self.run_fuzz_async(
                    request.target.clone(),
                    payload_type.unwrap_or_else(|| "all".to_string()),
                    threads,
                    timeout,
                )
            }
            other => Err(pyo3::exceptions::PyValueError::new_err(format!(
                "Unknown operation: {}",
                other
            ))),
        }
    }

    fn run_port_scan_async(
        &self,
        target: String,
        ports: Vec<u16>,
        effective_timeout_ms: u64,
    ) -> PyResult<runtime_async::PyFuture> {
        self.scope.enforce_target(&target)?;
        for &port in &ports {
            self.scope.enforce_port(port)?;
        }

        let effective_concurrency = self.concurrency;
        let target_owned = target.clone();

        runtime_async::spawn_async(async move {
            let config = eggsec::scanner::PortScanConfig {
                ports,
                concurrency: effective_concurrency,
                timeout_duration: std::time::Duration::from_millis(effective_timeout_ms),
                tui_mode: false,
                spoof_config: eggsec::scanner::SpoofConfig::default(),
                progress_tx: None,
                max_results: None,
            };

            match eggsec::scanner::scan_ports(&target_owned, config)
                .await
                .map_pyerr()
            {
                Ok(r) => {
                    let _py_result = PortScanResult::from_engine(r);
                    let mut metadata = std::collections::HashMap::new();
                    metadata.insert("target".to_string(), target_owned);
                    Ok(operation_ok(
                        ExecutionStats::new(0, 0, 0, 0),
                        Some(metadata),
                    ))
                }
                Err(e) => Ok(operation_err(e.to_string())),
            }
        })
    }

    fn run_endpoint_scan_async(
        &self,
        target: String,
        endpoints: Vec<String>,
        effective_timeout_ms: u64,
    ) -> PyResult<runtime_async::PyFuture> {
        let host = extract_host_from_url(&target)?;
        self.scope.enforce_target(&host)?;

        let target_owned = target.clone();

        runtime_async::spawn_async(async move {
            let config = eggsec::scanner::EndpointScanConfig {
                base_url: target_owned.clone(),
                endpoints,
                concurrency: 100,
                timeout_duration: std::time::Duration::from_millis(effective_timeout_ms),
                include_404: false,
                tui_mode: false,
                spoof_config: std::sync::Arc::new(eggsec::scanner::SpoofConfig::default()),
                verify_tls: true,
                progress_tx: None,
                max_results: None,
            };

            match eggsec::scanner::scan_endpoints(config).await.map_pyerr() {
                Ok(r) => {
                    let _py_result = EndpointScanResult::from_engine(r);
                    let mut metadata = std::collections::HashMap::new();
                    metadata.insert("target".to_string(), target_owned);
                    Ok(operation_ok(
                        ExecutionStats::new(0, 0, 0, 0),
                        Some(metadata),
                    ))
                }
                Err(e) => Ok(operation_err(e.to_string())),
            }
        })
    }

    fn run_fingerprint_async(
        &self,
        target: String,
        ports: Vec<u16>,
        effective_timeout_ms: u64,
    ) -> PyResult<runtime_async::PyFuture> {
        self.scope.enforce_target(&target)?;
        for &port in &ports {
            self.scope.enforce_port(port)?;
        }

        let target_owned = target.clone();
        let ports_owned = ports;

        runtime_async::spawn_async(async move {
            match eggsec::scanner::fingerprint_services(
                &target_owned,
                ports_owned,
                std::time::Duration::from_millis(effective_timeout_ms),
                false,
                100,
                None,
                None,
            )
            .await
            .map_pyerr()
            {
                Ok(r) => {
                    let _py_result = FingerprintScanResult::from_engine(r);
                    let mut metadata = std::collections::HashMap::new();
                    metadata.insert("target".to_string(), target_owned);
                    Ok(operation_ok(
                        ExecutionStats::new(0, 0, 0, 0),
                        Some(metadata),
                    ))
                }
                Err(e) => Ok(operation_err(e.to_string())),
            }
        })
    }

    fn run_recon_dns_async(&self, target: String) -> PyResult<runtime_async::PyFuture> {
        self.scope.enforce_target(&target)?;

        let domain_owned = target.clone();

        runtime_async::spawn_async(async move {
            match eggsec::recon::dns_records::enumerate_dns_records(&domain_owned)
                .await
                .map_pyerr()
            {
                Ok(r) => {
                    let _py_result = DnsRecordSet {
                        domain: r.domain,
                        a_records: r.a,
                        aaaa_records: r.aaaa,
                        cname_records: r.cname,
                        mx_records: r
                            .mx
                            .into_iter()
                            .map(|m| crate::recon::MxRecord {
                                preference: m.preference,
                                exchange: m.exchange,
                            })
                            .collect(),
                        txt_records: r.txt,
                        ns_records: r.ns,
                        soa_record: r.soa.map(|s| crate::recon::SoaRecord {
                            mname: s.mname,
                            rname: s.rname,
                            serial: s.serial,
                            refresh: s.refresh,
                            retry: s.retry,
                            expire: s.expire,
                            minimum: s.minimum,
                        }),
                        caa_records: r.caa,
                    };
                    let mut metadata = std::collections::HashMap::new();
                    metadata.insert("target".to_string(), domain_owned);
                    Ok(operation_ok(
                        ExecutionStats::new(0, 0, 0, 0),
                        Some(metadata),
                    ))
                }
                Err(e) => Ok(operation_err(e.to_string())),
            }
        })
    }

    fn run_tls_inspect_async(&self, target: String) -> PyResult<runtime_async::PyFuture> {
        self.scope.enforce_target(&target)?;

        let host_owned = target.clone();

        runtime_async::spawn_async(async move {
            match eggsec::recon::ssl::analyze_ssl(&host_owned, 443)
                .await
                .map_pyerr()
            {
                Ok(r) => {
                    let _py_result = TlsInspectionResult {
                        target: r.target,
                        has_ssl: r.has_ssl,
                        certificate: r.certificate.map(|c| crate::recon::TlsCertificateInfo {
                            subject: c.subject,
                            issuer: c.issuer,
                            valid_from: c.valid_from,
                            valid_until: c.valid_until,
                            serial_number: c.serial_number,
                            signature_algorithm: c.signature_algorithm,
                            public_key_algorithm: c.public_key_algorithm,
                            key_size: c.key_size,
                            is_expired: c.is_expired,
                            days_until_expiry: c.days_until_expiry,
                            sans: c.subject_alternative_names,
                        }),
                        supported_versions: r.supported_versions,
                        supported_cipher_suites: r.supported_cipher_suites,
                        issues: r
                            .issues
                            .into_iter()
                            .map(|i| crate::recon::SslIssue {
                                severity: i.severity,
                                code: i.code,
                                description: i.description,
                            })
                            .collect(),
                    };
                    let mut metadata = std::collections::HashMap::new();
                    metadata.insert("target".to_string(), host_owned);
                    Ok(operation_ok(
                        ExecutionStats::new(0, 0, 0, 0),
                        Some(metadata),
                    ))
                }
                Err(e) => Ok(operation_err(e.to_string())),
            }
        })
    }

    fn run_tech_detect_async(&self, target: String) -> PyResult<runtime_async::PyFuture> {
        let host = extract_host_from_url(&target)?;
        self.scope.enforce_target(&host)?;

        let url_owned = target.clone();

        runtime_async::spawn_async(async move {
            match eggsec::recon::techdetect::detect_tech_stack(&url_owned)
                .await
                .map_pyerr()
            {
                Ok(r) => {
                    let _py_result = TechDetectionResult {
                        url: r.url,
                        status_code: r.status_code,
                        headers: r.headers.into_iter().collect(),
                        tech_stack: crate::recon::TechStack {
                            servers: r.tech_stack.servers,
                            frameworks: r.tech_stack.frameworks,
                            languages: r.tech_stack.languages,
                            databases: r.tech_stack.databases,
                            cdns: r.tech_stack.cdns,
                            cms: r.tech_stack.cms,
                            javascript: r.tech_stack.javascript,
                            other: r.tech_stack.other,
                        },
                    };
                    let mut metadata = std::collections::HashMap::new();
                    metadata.insert("target".to_string(), url_owned);
                    Ok(operation_ok(
                        ExecutionStats::new(0, 0, 0, 0),
                        Some(metadata),
                    ))
                }
                Err(e) => Ok(operation_err(e.to_string())),
            }
        })
    }

    fn run_waf_detect_async(&self, target: String) -> PyResult<runtime_async::PyFuture> {
        let host = extract_host_from_url(&target)?;
        self.scope.enforce_target(&host)?;

        let url_owned = target.clone();

        runtime_async::spawn_async(async move {
            match async {
                let detector = eggsec::waf::WafDetector::new().map_pyerr()?;
                detector.detect(&url_owned).await.map_pyerr()
            }
            .await
            {
                Ok(r) => {
                    let _py_result = WafDetectionResultPy {
                        url: url_owned.clone(),
                        detected: r.waf_name.is_some(),
                        vendor: r.waf_name.clone(),
                        waf_name: r.waf_name,
                        confidence: r.confidence,
                        matched_headers: r.matched_headers,
                        matched_cookies: r.matched_cookies,
                        matched_patterns: r.matched_patterns,
                        server_header: r.server_header,
                        status_code: r.status_code,
                        request_error: r.request_error,
                    };
                    let mut metadata = std::collections::HashMap::new();
                    metadata.insert("target".to_string(), url_owned);
                    Ok(operation_ok(
                        ExecutionStats::new(0, 0, 0, 0),
                        Some(metadata),
                    ))
                }
                Err(e) => Ok(operation_err(e.to_string())),
            }
        })
    }

    fn run_load_test_async(
        &self,
        target: String,
        total_requests: u64,
        concurrency: usize,
        timeout_secs: u64,
        method: String,
    ) -> PyResult<runtime_async::PyFuture> {
        let host = extract_host_from_url(&target)?;
        self.scope.enforce_target(&host)?;

        if total_requests == 0 {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "total_requests must be > 0",
            ));
        }
        if concurrency == 0 {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "concurrency must be > 0",
            ));
        }
        if timeout_secs == 0 {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "timeout_secs must be > 0",
            ));
        }

        crate::loadtest::async_load_test_http(
            &target,
            total_requests,
            concurrency,
            timeout_secs,
            self.scope.clone(),
            &method,
        )
    }

    fn run_waf_validate_async(&self, target: String) -> PyResult<runtime_async::PyFuture> {
        let host = extract_host_from_url(&target)?;
        self.scope.enforce_target(&host)?;

        crate::waf_validation::async_validate_waf(&target, self.scope.clone(), false, None)
    }

    fn run_fuzz_async(
        &self,
        target: String,
        payload_type: String,
        concurrency: usize,
        timeout: u64,
    ) -> PyResult<runtime_async::PyFuture> {
        let host = extract_host_from_url(&target)?;
        self.scope.enforce_target(&host)?;

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

        crate::waf_validation::async_fuzz_http(
            &target,
            self.scope.clone(),
            &payload_type,
            "GET",
            None,
            concurrency,
            timeout,
        )
    }
}
