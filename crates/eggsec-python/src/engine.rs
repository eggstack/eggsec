use pyo3::prelude::*;

use crate::dto::PortScanResult;
use crate::endpoint::EndpointScanResult;
use crate::error::EggsecResultExt;
use crate::fingerprint::FingerprintScanResult;
use crate::planning::ScanPlan;
use crate::recon::{DnsRecordSet, TechDetectionResult, TlsInspectionResult};
use crate::requests::*;
use crate::runtime_sync;
use crate::scope::Scope;
use crate::status::{ExecutionStats, ExecutionStatus, OperationResult};
use crate::waf::WafDetectionResultPy;

/// Sync engine for running scoped security operations.
///
/// Wraps the Rust engine directly. Each `run_*` method returns an `OperationResult`
/// instead of raising exceptions — errors are captured in the result status.
#[pyclass]
pub struct Engine {
    scope: Scope,
    mode: String,
    concurrency: usize,
    timeout_ms: u64,
}

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
/// Supports plain ports ("80,443") and ranges ("1-1024").
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

#[pymethods]
impl Engine {
    /// Create a new engine.
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
    /// Returns an OperationResult with status and artifacts.
    /// On unknown operation, returns a Failed OperationResult.
    fn run(&self, py: Python<'_>, request: OperationRequest) -> OperationResult {
        self.dispatch(py, request)
    }

    /// Run a port scan.
    #[pyo3(signature = (request,))]
    fn run_port_scan(&self, py: Python<'_>, request: PortScanRequest) -> OperationResult {
        let effective_timeout = request.timeout_ms.unwrap_or(self.timeout_ms);
        let ports_str = request
            .ports
            .clone()
            .unwrap_or_else(|| "1-1024".to_string());
        let ports = match parse_ports_string(&ports_str) {
            Ok(p) => p,
            Err(e) => return operation_err(e.to_string()),
        };
        let effective_concurrency = self.concurrency;
        self.run_port_scan_inner(
            py,
            &request,
            ports,
            effective_concurrency,
            effective_timeout,
        )
    }

    /// Run an endpoint scan.
    #[pyo3(signature = (request,))]
    fn run_endpoint_scan(&self, py: Python<'_>, request: EndpointScanRequest) -> OperationResult {
        self.run_endpoint_scan_inner(py, &request)
    }

    /// Run service fingerprinting.
    #[pyo3(signature = (request,))]
    fn run_fingerprint(&self, py: Python<'_>, request: FingerprintRequest) -> OperationResult {
        let ports = request.ports.clone().unwrap_or_else(|| vec![80, 443]);
        self.run_fingerprint_inner(py, &request, ports)
    }

    /// Run DNS reconnaissance.
    #[pyo3(signature = (request,))]
    fn run_recon_dns(&self, py: Python<'_>, request: ReconDnsRequest) -> OperationResult {
        self.run_recon_dns_inner(py, &request)
    }

    /// Run TLS inspection.
    #[pyo3(signature = (request,))]
    fn run_tls_inspect(&self, py: Python<'_>, request: TlsInspectRequest) -> OperationResult {
        self.run_tls_inspect_inner(py, &request)
    }

    /// Run technology detection.
    #[pyo3(signature = (request,))]
    fn run_tech_detect(&self, py: Python<'_>, request: TechDetectRequest) -> OperationResult {
        self.run_tech_detect_inner(py, &request)
    }

    /// Run WAF detection.
    #[pyo3(signature = (request,))]
    fn run_waf_detect(&self, py: Python<'_>, request: WafDetectRequest) -> OperationResult {
        self.run_waf_detect_inner(py, &request)
    }

    /// Run an HTTP load test.
    #[pyo3(signature = (request,))]
    fn run_load_test(&self, py: Python<'_>, request: LoadTestRequest) -> OperationResult {
        self.run_load_test_inner(py, &request)
    }

    /// Run WAF validation.
    #[pyo3(signature = (request,))]
    fn run_waf_validate(&self, py: Python<'_>, request: WafValidateRequest) -> OperationResult {
        self.run_waf_validate_inner(py, &request)
    }

    /// Run HTTP fuzzing.
    #[pyo3(signature = (request,))]
    fn run_fuzz(&self, py: Python<'_>, request: FuzzRequest) -> OperationResult {
        self.run_fuzz_inner(py, &request)
    }

    /// Create a scan plan suggesting what operations to run against a target.
    fn plan(&self, target: &str) -> PyResult<ScanPlan> {
        self.plan_inner(target)
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

    /// Context manager __enter__.
    fn __enter__(slf: Py<Self>) -> Py<Self> {
        slf
    }

    /// Context manager __exit__.
    #[pyo3(signature = (_exc_type=None, _exc_value=None, _traceback=None))]
    fn __exit__(
        &self,
        _exc_type: Option<&Bound<'_, PyAny>>,
        _exc_value: Option<&Bound<'_, PyAny>>,
        _traceback: Option<&Bound<'_, PyAny>>,
    ) -> bool {
        false
    }

    fn __repr__(&self) -> String {
        format!(
            "Engine(mode={}, concurrency={})",
            self.mode, self.concurrency
        )
    }
}

// Internal constructor and helpers (not exposed to Python)
impl Engine {
    /// Internal constructor shared by Engine and Client.
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

    /// Dispatch a generic operation request (used by pipeline and other internal callers).
    pub(crate) fn dispatch(&self, py: Python<'_>, request: OperationRequest) -> OperationResult {
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
                let effective_concurrency = self.concurrency;
                match parse_ports_string(&ports_str) {
                    Ok(ports) => {
                        let req = PortScanRequest::new(
                            target,
                            Some(ports_str),
                            None,
                            None,
                            Some(effective_timeout),
                        );
                        self.run_port_scan_inner(
                            py,
                            &req,
                            ports,
                            effective_concurrency,
                            effective_timeout,
                        )
                    }
                    Err(e) => operation_err(e.to_string()),
                }
            }
            "scan_endpoints" => {
                let target = request.target.clone();
                let effective_timeout = request.timeout_ms.unwrap_or(self.timeout_ms);
                let endpoints: Vec<String> = request
                    .metadata
                    .get("endpoints")
                    .map(|s| s.split(',').map(|s| s.trim().to_string()).collect())
                    .unwrap_or_default();
                let req = EndpointScanRequest::new(
                    target,
                    Some(endpoints),
                    None,
                    Some(effective_timeout),
                );
                self.run_endpoint_scan_inner(py, &req)
            }
            "fingerprint" => {
                let target = request.target.clone();
                let effective_timeout = request.timeout_ms.unwrap_or(self.timeout_ms);
                let ports_str = request.metadata.get("ports").cloned().unwrap_or_default();
                let ports = if ports_str.is_empty() {
                    vec![80, 443]
                } else {
                    match parse_ports_string(&ports_str) {
                        Ok(p) => p,
                        Err(e) => return operation_err(e.to_string()),
                    }
                };
                let req =
                    FingerprintRequest::new(target, Some(ports.clone()), Some(effective_timeout));
                self.run_fingerprint_inner(py, &req, ports)
            }
            "recon_dns" => {
                let req = ReconDnsRequest::new(request.target.clone(), None, request.timeout_ms);
                self.run_recon_dns_inner(py, &req)
            }
            "tls_inspect" => {
                let req = TlsInspectRequest::new(request.target.clone(), request.timeout_ms);
                self.run_tls_inspect_inner(py, &req)
            }
            "tech_detect" => {
                let req = TechDetectRequest::new(request.target.clone(), request.timeout_ms);
                self.run_tech_detect_inner(py, &req)
            }
            "waf_detect" => {
                let req = WafDetectRequest::new(request.target.clone(), request.timeout_ms);
                self.run_waf_detect_inner(py, &req)
            }
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
                let req = LoadTestRequest::new(
                    request.target.clone(),
                    Some(total_requests as u32),
                    Some(concurrency as u32),
                    Some(method),
                    request.timeout_ms,
                );
                self.run_load_test_inner(py, &req)
            }
            "waf_validate" => {
                let req = WafValidateRequest::new(request.target.clone(), None, request.timeout_ms);
                self.run_waf_validate_inner(py, &req)
            }
            "fuzz" => {
                let payload_type = request.metadata.get("payload_type").cloned();
                let threads: Option<u32> =
                    request.metadata.get("threads").and_then(|s| s.parse().ok());
                let req = FuzzRequest::new(
                    request.target.clone(),
                    payload_type,
                    threads,
                    request.timeout_ms,
                );
                self.run_fuzz_inner(py, &req)
            }
            other => operation_err(format!("Unknown operation: {}", other)),
        }
    }

    fn run_port_scan_inner(
        &self,
        py: Python<'_>,
        request: &PortScanRequest,
        ports: Vec<u16>,
        effective_concurrency: usize,
        effective_timeout_ms: u64,
    ) -> OperationResult {
        if let Err(e) = self.scope.enforce_target(&request.target) {
            return operation_err(e.to_string());
        }
        for &port in &ports {
            if let Err(e) = self.scope.enforce_port(port) {
                return operation_err(e.to_string());
            }
        }

        let target_owned = request.target.clone();
        let config = eggsec::scanner::PortScanConfig {
            ports,
            concurrency: effective_concurrency,
            timeout_duration: std::time::Duration::from_millis(effective_timeout_ms),
            tui_mode: false,
            spoof_config: eggsec::scanner::SpoofConfig::default(),
            progress_tx: None,
            max_results: None,
        };

        let result = runtime_sync::block_on(py, async move {
            eggsec::scanner::scan_ports(&target_owned, config)
                .await
                .map_pyerr()
        });

        match result {
            Ok(r) => {
                let _py_result = PortScanResult::from_engine(r);
                let mut metadata = std::collections::HashMap::new();
                metadata.insert("target".to_string(), request.target.clone());
                operation_ok(ExecutionStats::new(0, 0, 0, 0), Some(metadata))
            }
            Err(e) => operation_err(e.to_string()),
        }
    }

    fn run_endpoint_scan_inner(
        &self,
        py: Python<'_>,
        request: &EndpointScanRequest,
    ) -> OperationResult {
        let host = match extract_host_from_url(&request.target) {
            Ok(h) => h,
            Err(e) => return operation_err(e.to_string()),
        };
        if let Err(e) = self.scope.enforce_target(&host) {
            return operation_err(e.to_string());
        }

        let effective_timeout = request.timeout_ms.unwrap_or(self.timeout_ms);
        let endpoints = request.paths.clone().unwrap_or_default();

        let config = eggsec::scanner::EndpointScanConfig {
            base_url: request.target.clone(),
            endpoints,
            concurrency: self.concurrency,
            timeout_duration: std::time::Duration::from_millis(effective_timeout),
            include_404: false,
            tui_mode: false,
            spoof_config: std::sync::Arc::new(eggsec::scanner::SpoofConfig::default()),
            verify_tls: true,
            progress_tx: None,
            max_results: None,
        };

        let result = runtime_sync::block_on(py, async move {
            eggsec::scanner::scan_endpoints(config).await.map_pyerr()
        });

        match result {
            Ok(r) => {
                let _py_result = EndpointScanResult::from_engine(r);
                let mut metadata = std::collections::HashMap::new();
                metadata.insert("target".to_string(), request.target.clone());
                operation_ok(ExecutionStats::new(0, 0, 0, 0), Some(metadata))
            }
            Err(e) => operation_err(e.to_string()),
        }
    }

    fn run_fingerprint_inner(
        &self,
        py: Python<'_>,
        request: &FingerprintRequest,
        ports: Vec<u16>,
    ) -> OperationResult {
        if let Err(e) = self.scope.enforce_target(&request.target) {
            return operation_err(e.to_string());
        }
        for &port in &ports {
            if let Err(e) = self.scope.enforce_port(port) {
                return operation_err(e.to_string());
            }
        }

        let effective_timeout = request.timeout_ms.unwrap_or(self.timeout_ms);
        let target_owned = request.target.clone();
        let ports_owned = ports;

        let result = runtime_sync::block_on(py, async move {
            eggsec::scanner::fingerprint_services(
                &target_owned,
                ports_owned,
                std::time::Duration::from_millis(effective_timeout),
                false,
                100,
                None,
                None,
            )
            .await
            .map_pyerr()
        });

        match result {
            Ok(r) => {
                let _py_result = FingerprintScanResult::from_engine(r);
                let mut metadata = std::collections::HashMap::new();
                metadata.insert("target".to_string(), request.target.clone());
                operation_ok(ExecutionStats::new(0, 0, 0, 0), Some(metadata))
            }
            Err(e) => operation_err(e.to_string()),
        }
    }

    fn run_recon_dns_inner(&self, py: Python<'_>, request: &ReconDnsRequest) -> OperationResult {
        if let Err(e) = self.scope.enforce_target(&request.target) {
            return operation_err(e.to_string());
        }

        let domain_owned = request.target.clone();
        let result = runtime_sync::block_on(py, async move {
            eggsec::recon::dns_records::enumerate_dns_records(&domain_owned)
                .await
                .map_pyerr()
        });

        match result {
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
                metadata.insert("target".to_string(), request.target.clone());
                operation_ok(ExecutionStats::new(0, 0, 0, 0), Some(metadata))
            }
            Err(e) => operation_err(e.to_string()),
        }
    }

    fn run_tls_inspect_inner(
        &self,
        py: Python<'_>,
        request: &TlsInspectRequest,
    ) -> OperationResult {
        if let Err(e) = self.scope.enforce_target(&request.target) {
            return operation_err(e.to_string());
        }

        let host_owned = request.target.clone();
        let result = runtime_sync::block_on(py, async move {
            eggsec::recon::ssl::analyze_ssl(&host_owned, 443)
                .await
                .map_pyerr()
        });

        match result {
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
                metadata.insert("target".to_string(), request.target.clone());
                operation_ok(ExecutionStats::new(0, 0, 0, 0), Some(metadata))
            }
            Err(e) => operation_err(e.to_string()),
        }
    }

    fn run_tech_detect_inner(
        &self,
        py: Python<'_>,
        request: &TechDetectRequest,
    ) -> OperationResult {
        let host = match extract_host_from_url(&request.target) {
            Ok(h) => h,
            Err(e) => return operation_err(e.to_string()),
        };
        if let Err(e) = self.scope.enforce_target(&host) {
            return operation_err(e.to_string());
        }

        let url_owned = request.target.clone();
        let result = runtime_sync::block_on(py, async move {
            eggsec::recon::techdetect::detect_tech_stack(&url_owned)
                .await
                .map_pyerr()
        });

        match result {
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
                metadata.insert("target".to_string(), request.target.clone());
                operation_ok(ExecutionStats::new(0, 0, 0, 0), Some(metadata))
            }
            Err(e) => operation_err(e.to_string()),
        }
    }

    fn run_waf_detect_inner(&self, py: Python<'_>, request: &WafDetectRequest) -> OperationResult {
        let host = match extract_host_from_url(&request.target) {
            Ok(h) => h,
            Err(e) => return operation_err(e.to_string()),
        };
        if let Err(e) = self.scope.enforce_target(&host) {
            return operation_err(e.to_string());
        }

        let url_owned = request.target.clone();
        let url_clone = url_owned.clone();
        let result = runtime_sync::block_on(py, async move {
            let detector = eggsec::waf::WafDetector::new().map_pyerr()?;
            detector.detect(&url_clone).await.map_pyerr()
        });

        match result {
            Ok(r) => {
                let _py_result = WafDetectionResultPy {
                    url: url_owned,
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
                metadata.insert("target".to_string(), request.target.clone());
                operation_ok(ExecutionStats::new(0, 0, 0, 0), Some(metadata))
            }
            Err(e) => operation_err(e.to_string()),
        }
    }

    fn run_load_test_inner(&self, py: Python<'_>, request: &LoadTestRequest) -> OperationResult {
        let host = match extract_host_from_url(&request.target) {
            Ok(h) => h,
            Err(e) => return operation_err(e.to_string()),
        };
        if let Err(e) = self.scope.enforce_target(&host) {
            return operation_err(e.to_string());
        }

        let total_requests = request.requests.unwrap_or(100) as u64;
        let concurrency = request.concurrency.unwrap_or(self.concurrency as u32) as usize;
        let method = request.method.clone().unwrap_or_else(|| "GET".to_string());
        let timeout_secs = request.timeout_ms.map(|ms| ms / 1000).unwrap_or(30);

        if total_requests == 0 {
            return operation_err("total_requests must be > 0".to_string());
        }
        if concurrency == 0 {
            return operation_err("concurrency must be > 0".to_string());
        }
        if timeout_secs == 0 {
            return operation_err("timeout_secs must be > 0".to_string());
        }

        let scope_clone = self.scope.clone();

        let result = crate::loadtest::load_test_http(
            py,
            &request.target,
            total_requests,
            concurrency,
            timeout_secs,
            scope_clone,
            &method,
        );

        match result {
            Ok(_r) => {
                let mut metadata = std::collections::HashMap::new();
                metadata.insert("target".to_string(), request.target.clone());
                operation_ok(ExecutionStats::new(0, 0, 0, 0), Some(metadata))
            }
            Err(e) => operation_err(e.to_string()),
        }
    }

    fn run_waf_validate_inner(
        &self,
        _py: Python<'_>,
        request: &WafValidateRequest,
    ) -> OperationResult {
        let host = match extract_host_from_url(&request.target) {
            Ok(h) => h,
            Err(e) => return operation_err(e.to_string()),
        };
        if let Err(e) = self.scope.enforce_target(&host) {
            return operation_err(e.to_string());
        }

        let scope_clone = self.scope.clone();
        let result = crate::waf_validation::validate_waf(&request.target, scope_clone, false, None);

        match result {
            Ok(_r) => {
                let mut metadata = std::collections::HashMap::new();
                metadata.insert("target".to_string(), request.target.clone());
                operation_ok(ExecutionStats::new(0, 0, 0, 0), Some(metadata))
            }
            Err(e) => operation_err(e.to_string()),
        }
    }

    fn run_fuzz_inner(&self, _py: Python<'_>, request: &FuzzRequest) -> OperationResult {
        let host = match extract_host_from_url(&request.target) {
            Ok(h) => h,
            Err(e) => return operation_err(e.to_string()),
        };
        if let Err(e) = self.scope.enforce_target(&host) {
            return operation_err(e.to_string());
        }

        let payload_type = request
            .payload_type
            .clone()
            .unwrap_or_else(|| "all".to_string());
        let concurrency = request.threads.unwrap_or(10) as usize;
        let timeout = request.timeout_ms.map(|ms| ms / 1000).unwrap_or(30);

        if concurrency == 0 {
            return operation_err("concurrency must be > 0".to_string());
        }
        if timeout == 0 {
            return operation_err("timeout must be > 0".to_string());
        }

        let scope_clone = self.scope.clone();
        let target = request.target.clone();

        let result = crate::waf_validation::fuzz_http(
            &target,
            scope_clone,
            &payload_type,
            "GET",
            None,
            concurrency,
            timeout,
        );

        match result {
            Ok(_r) => {
                let mut metadata = std::collections::HashMap::new();
                metadata.insert("target".to_string(), request.target.clone());
                operation_ok(ExecutionStats::new(0, 0, 0, 0), Some(metadata))
            }
            Err(e) => operation_err(e.to_string()),
        }
    }
}
