use std::sync::Arc;

use pyo3::prelude::*;

use crate::config_model::PyEggsecConfig;
use crate::dto::PortScanResult;
use crate::endpoint::EndpointScanResult;
use crate::engine_state::EngineState;
use crate::error::EggsecResultExt;
use crate::fingerprint::FingerprintScanResult;
use crate::recon::{DnsRecordSet, TechDetectionResult, TlsInspectionResult};
use crate::requests::*;
use crate::runtime_async;
use crate::scope::Scope;
use crate::status::{
    ExecutionStats, ExecutionStatus, OperationError, OperationPayload, OperationResult,
};
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
    payload: Option<super::status::OperationPayload>,
) -> OperationResult {
    let payload_type = payload.as_ref().map(|p| p.type_name().to_string());
    let mut metadata = metadata.unwrap_or_default();
    metadata.insert("policy_decision".to_string(), "allow".to_string());
    metadata.insert("policy_schema_version".to_string(), "1.0".to_string());
    OperationResult {
        status: ExecutionStatus::Completed(),
        stats: Some(stats),
        artifacts: Vec::new(),
        error: None,
        metadata,
        payload,
        payload_type,
    }
}

/// Build an OperationResult from an error.
fn operation_err(error: String) -> OperationResult {
    operation_err_for(None, error)
}

fn operation_err_for(operation: Option<&str>, error: String) -> OperationResult {
    let structured = OperationError::from_message(operation, &error);
    OperationResult {
        status: ExecutionStatus::Failed {
            error: error.clone(),
        },
        stats: None,
        artifacts: Vec::new(),
        error: Some(structured),
        metadata: std::collections::HashMap::new(),
        payload: None,
        payload_type: None,
    }
}

/// Async engine for running scoped security operations.
///
/// Provides the same operations as Engine but returns Python awaitables.
/// Each async operation spawns a background thread with its own Tokio runtime.
///
/// The engine holds a shared `EngineState` (via `Arc`) that is also used by
/// `Engine`, ensuring every operation passes through common validation,
/// scope enforcement, feature gating, and audit logging.
#[pyclass]
pub struct AsyncEngine {
    pub(crate) state: Arc<EngineState>,
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
    /// Routes through the OperationExecutorRegistry, which checks feature gates
    /// and provides "Did you mean?" suggestions for unknown operations.
    /// Returns a PyFuture that resolves to an OperationResult.
    fn run(&self, request: OperationRequest) -> PyResult<runtime_async::PyFuture> {
        self.state
            .registry
            .execute_async(&request.operation, &request, self)
    }

    /// List all registered operation IDs.
    fn list_operations(&self) -> Vec<String> {
        self.state.registry.list()
    }

    /// Check if an operation ID is registered.
    fn has_operation(&self, operation_id: &str) -> bool {
        self.state.registry.contains(operation_id)
    }

    /// Return structured policy decisions emitted by this engine instance.
    fn audit_events(&self) -> Vec<crate::engine_state::DispatchAuditEvent> {
        self.state.audit_events()
    }

    /// Run a port scan (async).
    #[pyo3(signature = (request,))]
    fn run_port_scan(&self, request: PortScanRequest) -> PyResult<runtime_async::PyFuture> {
        self.state
            .pre_dispatch_validate("scan_ports", &request.target)?;
        let effective_timeout = request.timeout_ms.unwrap_or(self.state.timeout_ms);
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
        self.state
            .pre_dispatch_validate("scan_endpoints", &request.target)?;
        let endpoints = request.paths.clone().unwrap_or_default();
        let effective_timeout = request.timeout_ms.unwrap_or(self.state.timeout_ms);
        self.run_endpoint_scan_async(request.target.clone(), endpoints, effective_timeout)
    }

    /// Run service fingerprinting (async).
    #[pyo3(signature = (request,))]
    fn run_fingerprint(&self, request: FingerprintRequest) -> PyResult<runtime_async::PyFuture> {
        self.state
            .pre_dispatch_validate("fingerprint_services", &request.target)?;
        let ports = request.ports.clone().unwrap_or_else(|| vec![80, 443]);
        let effective_timeout = request.timeout_ms.unwrap_or(self.state.timeout_ms);
        self.run_fingerprint_async(request.target.clone(), ports, effective_timeout)
    }

    /// Run DNS reconnaissance (async).
    #[pyo3(signature = (request,))]
    fn run_recon_dns(&self, request: ReconDnsRequest) -> PyResult<runtime_async::PyFuture> {
        self.state
            .pre_dispatch_validate("recon_dns", &request.target)?;
        self.run_recon_dns_async(request.target.clone())
    }

    /// Run TLS inspection (async).
    #[pyo3(signature = (request,))]
    fn run_tls_inspect(&self, request: TlsInspectRequest) -> PyResult<runtime_async::PyFuture> {
        self.state
            .pre_dispatch_validate("inspect_tls", &request.target)?;
        self.run_tls_inspect_async(request.target.clone())
    }

    /// Run technology detection (async).
    #[pyo3(signature = (request,))]
    fn run_tech_detect(&self, request: TechDetectRequest) -> PyResult<runtime_async::PyFuture> {
        self.state
            .pre_dispatch_validate("detect_technology", &request.target)?;
        self.run_tech_detect_async(request.target.clone())
    }

    /// Run WAF detection (async).
    #[pyo3(signature = (request,))]
    fn run_waf_detect(&self, request: WafDetectRequest) -> PyResult<runtime_async::PyFuture> {
        self.state
            .pre_dispatch_validate("detect_waf", &request.target)?;
        self.run_waf_detect_async(request.target.clone())
    }

    /// Run an HTTP load test (async).
    #[pyo3(signature = (request,))]
    fn run_load_test(&self, request: LoadTestRequest) -> PyResult<runtime_async::PyFuture> {
        self.state
            .pre_dispatch_validate("load_test", &request.target)?;
        let total_requests = request.requests.unwrap_or(100) as u64;
        let concurrency = request.concurrency.unwrap_or(self.state.concurrency as u32) as usize;
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
        self.state
            .pre_dispatch_validate("validate_waf", &request.target)?;
        self.run_waf_validate_async(request.target.clone())
    }

    /// Run HTTP fuzzing (async).
    #[pyo3(signature = (request,))]
    fn run_fuzz(&self, request: FuzzRequest) -> PyResult<runtime_async::PyFuture> {
        self.state
            .pre_dispatch_validate("fuzz_http", &request.target)?;
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
        self.state.scope.clone()
    }

    /// Get the engine's mode.
    #[getter]
    fn mode(&self) -> String {
        self.state.mode.clone()
    }

    /// Get the engine's concurrency.
    #[getter]
    fn concurrency(&self) -> usize {
        self.state.concurrency
    }

    /// Get the engine's timeout in milliseconds.
    #[getter]
    fn timeout_ms(&self) -> u64 {
        self.state.timeout_ms
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
            self.state.mode, self.state.concurrency
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
        let state = EngineState::from_params(scope, mode, concurrency, timeout_ms)?;
        Ok(Self { state })
    }

    /// Internal constructor from a full EggsecConfig.
    pub(crate) fn new_with_config(
        scope: Scope,
        config: PyEggsecConfig,
        mode: &str,
        concurrency: Option<usize>,
        timeout_ms: Option<u64>,
    ) -> PyResult<Self> {
        let state = EngineState::from_config(scope, config, mode, concurrency, timeout_ms)?;
        Ok(Self { state })
    }

    /// Enforce that a target is within scope, raising EnforcementError if denied.
    pub(crate) fn enforce_target(&self, target: &str) -> PyResult<()> {
        self.state.enforce_target(target)
    }

    /// Enforce that a port is within scope, raising EnforcementError if denied.
    pub(crate) fn enforce_port(&self, port: u16) -> PyResult<()> {
        self.state.enforce_port(port)
    }

    /// Borrow the scope (immutable reference).
    pub(crate) fn scope_ref(&self) -> &Scope {
        self.state.scope_ref()
    }

    /// Get the effective concurrency.
    pub(crate) fn get_concurrency(&self) -> usize {
        self.state.get_concurrency()
    }

    /// Get the effective timeout in milliseconds.
    pub(crate) fn get_timeout_ms(&self) -> u64 {
        self.state.get_timeout_ms()
    }

    /// Get the mode string.
    pub(crate) fn get_mode(&self) -> &str {
        self.state.get_mode()
    }

    /// Dispatch a generic operation request (used by async pipeline and other internal callers).
    ///
    /// This method is called by `OperationExecutorRegistry::execute_async()` after feature
    /// gate validation. Operation IDs must match `StableOperation::ALL`.
    pub(crate) fn dispatch_async(
        &self,
        request: OperationRequest,
    ) -> PyResult<runtime_async::PyFuture> {
        use crate::operation_registry::StableOperation;
        let op = request.operation.clone();

        // Pre-dispatch validation: scope, feature gates, audit logging.
        self.state.pre_dispatch_validate(&op, &request.target)?;

        let operation = StableOperation::parse(&op).ok_or_else(|| {
            pyo3::exceptions::PyValueError::new_err(format!("Unknown operation: {}", op))
        })?;

        match operation {
            StableOperation::ScanPorts => {
                let target = request.target.clone();
                let ports_str = request
                    .metadata
                    .get("ports")
                    .cloned()
                    .unwrap_or_else(|| "1-1024".to_string());
                let effective_timeout = request.timeout_ms.unwrap_or(self.state.timeout_ms);
                let ports = parse_ports_string(&ports_str)?;
                self.run_port_scan_async(target, ports, effective_timeout)
            }
            StableOperation::ScanEndpoints => {
                let target = request.target.clone();
                let endpoints: Vec<String> = request
                    .metadata
                    .get("endpoints")
                    .map(|s| s.split(',').map(|s| s.trim().to_string()).collect())
                    .unwrap_or_default();
                let effective_timeout = request.timeout_ms.unwrap_or(self.state.timeout_ms);
                self.run_endpoint_scan_async(target, endpoints, effective_timeout)
            }
            StableOperation::FingerprintServices => {
                let target = request.target.clone();
                let ports_str = request.metadata.get("ports").cloned().unwrap_or_default();
                let ports = if ports_str.is_empty() {
                    vec![80, 443]
                } else {
                    parse_ports_string(&ports_str)?
                };
                let effective_timeout = request.timeout_ms.unwrap_or(self.state.timeout_ms);
                self.run_fingerprint_async(target, ports, effective_timeout)
            }
            StableOperation::ReconDns => self.run_recon_dns_async(request.target.clone()),
            StableOperation::InspectTls => self.run_tls_inspect_async(request.target.clone()),
            StableOperation::DetectTechnology => self.run_tech_detect_async(request.target.clone()),
            StableOperation::DetectWaf => self.run_waf_detect_async(request.target.clone()),
            StableOperation::LoadTest => {
                let total_requests: u64 = request
                    .metadata
                    .get("requests")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(100);
                let concurrency: usize = request
                    .metadata
                    .get("concurrency")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(self.state.concurrency);
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
            StableOperation::ValidateWaf => self.run_waf_validate_async(request.target.clone()),
            StableOperation::FuzzHttp => {
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
        }
    }

    fn run_port_scan_async(
        &self,
        target: String,
        ports: Vec<u16>,
        effective_timeout_ms: u64,
    ) -> PyResult<runtime_async::PyFuture> {
        self.state.scope.enforce_target(&target)?;
        for &port in &ports {
            self.state.scope.enforce_port(port)?;
        }

        let effective_concurrency = self.state.concurrency;
        let target_owned = target.clone();
        let event_tx = self.state.event_tx.clone();

        // Emit: operation started (before spawn, on calling thread)
        if let Some(ref tx) = event_tx {
            let _ = Python::with_gil(|py| {
                tx.try_send(crate::event_protocol::EventEnvelope::create(
                    "operation.started".to_string(),
                    crate::event_protocol::ProgressEvent::new(
                        0.0,
                        format!("Starting port scan on {}", target_owned),
                        0,
                        ports.len(),
                    )
                    .into_py(py),
                    None,
                    None,
                    None,
                    None,
                ))
            });
        }

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
                    let py_result = PortScanResult::from_engine(r);
                    let items = py_result.scanned_ports as u64;
                    let open = py_result.open_ports.len() as u64;
                    let mut metadata = std::collections::HashMap::new();
                    metadata.insert("target".to_string(), target_owned);
                    let stats = ExecutionStats::new(py_result.elapsed_ms, items, items - open, 0);
                    let result = operation_ok(
                        stats,
                        Some(metadata),
                        Some(OperationPayload::PortScan(py_result)),
                    );
                    Ok(result)
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
        self.state.scope.enforce_target(&host)?;

        let target_owned = target.clone();
        let event_tx = self.state.event_tx.clone();

        // Emit: operation started
        if let Some(ref tx) = event_tx {
            let _ = Python::with_gil(|py| {
                tx.try_send(crate::event_protocol::EventEnvelope::create(
                    "operation.started".to_string(),
                    crate::event_protocol::ProgressEvent::new(
                        0.0,
                        format!("Starting endpoint scan on {}", target_owned),
                        0,
                        0,
                    )
                    .into_py(py),
                    None,
                    None,
                    None,
                    None,
                ))
            });
        }

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
                    let py_result = EndpointScanResult::from_engine(r);
                    let items = py_result.endpoints_found as u64;
                    let mut metadata = std::collections::HashMap::new();
                    metadata.insert("target".to_string(), target_owned);
                    let stats = ExecutionStats::new(py_result.elapsed_ms, items, 0, 0);
                    Ok(operation_ok(
                        stats,
                        Some(metadata),
                        Some(OperationPayload::EndpointScan(py_result)),
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
        self.state.scope.enforce_target(&target)?;
        for &port in &ports {
            self.state.scope.enforce_port(port)?;
        }

        let target_owned = target.clone();
        let ports_owned = ports;
        let event_tx = self.state.event_tx.clone();

        // Emit: operation started
        if let Some(ref tx) = event_tx {
            let _ = Python::with_gil(|py| {
                tx.try_send(crate::event_protocol::EventEnvelope::create(
                    "operation.started".to_string(),
                    crate::event_protocol::ProgressEvent::new(
                        0.0,
                        format!("Starting fingerprint scan on {}", target_owned),
                        0,
                        ports_owned.len(),
                    )
                    .into_py(py),
                    None,
                    None,
                    None,
                    None,
                ))
            });
        }

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
                    let py_result = FingerprintScanResult::from_engine(r);
                    let items = py_result.services_identified as u64;
                    let mut metadata = std::collections::HashMap::new();
                    metadata.insert("target".to_string(), target_owned);
                    let stats = ExecutionStats::new(py_result.elapsed_ms, items, 0, 0);
                    Ok(operation_ok(
                        stats,
                        Some(metadata),
                        Some(OperationPayload::Fingerprint(py_result)),
                    ))
                }
                Err(e) => Ok(operation_err(e.to_string())),
            }
        })
    }

    fn run_recon_dns_async(&self, target: String) -> PyResult<runtime_async::PyFuture> {
        self.state.scope.enforce_target(&target)?;

        let domain_owned = target.clone();
        let event_tx = self.state.event_tx.clone();

        // Emit: operation started
        if let Some(ref tx) = event_tx {
            let _ = Python::with_gil(|py| {
                tx.try_send(crate::event_protocol::EventEnvelope::create(
                    "operation.started".to_string(),
                    crate::event_protocol::ProgressEvent::new(
                        0.0,
                        format!("Starting DNS recon on {}", domain_owned),
                        0,
                        0,
                    )
                    .into_py(py),
                    None,
                    None,
                    None,
                    None,
                ))
            });
        }

        runtime_async::spawn_async(async move {
            match eggsec::recon::dns_records::enumerate_dns_records(&domain_owned)
                .await
                .map_pyerr()
            {
                Ok(r) => {
                    let py_result = DnsRecordSet {
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
                    let record_count = (py_result.a_records.len()
                        + py_result.aaaa_records.len()
                        + py_result.cname_records.len()
                        + py_result.mx_records.len()
                        + py_result.txt_records.len()
                        + py_result.ns_records.len()
                        + py_result.caa_records.len())
                        as u64;
                    let mut metadata = std::collections::HashMap::new();
                    metadata.insert("target".to_string(), domain_owned);
                    let stats = ExecutionStats::new(0, record_count, 0, 0);
                    Ok(operation_ok(
                        stats,
                        Some(metadata),
                        Some(OperationPayload::DnsRecon(py_result)),
                    ))
                }
                Err(e) => Ok(operation_err(e.to_string())),
            }
        })
    }

    fn run_tls_inspect_async(&self, target: String) -> PyResult<runtime_async::PyFuture> {
        self.state.scope.enforce_target(&target)?;

        let host_owned = target.clone();
        let event_tx = self.state.event_tx.clone();

        // Emit: operation started
        if let Some(ref tx) = event_tx {
            let _ = Python::with_gil(|py| {
                tx.try_send(crate::event_protocol::EventEnvelope::create(
                    "operation.started".to_string(),
                    crate::event_protocol::ProgressEvent::new(
                        0.0,
                        format!("Starting TLS inspection on {}", host_owned),
                        0,
                        0,
                    )
                    .into_py(py),
                    None,
                    None,
                    None,
                    None,
                ))
            });
        }

        runtime_async::spawn_async(async move {
            match eggsec::recon::ssl::analyze_ssl(&host_owned, 443)
                .await
                .map_pyerr()
            {
                Ok(r) => {
                    let py_result = TlsInspectionResult {
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
                    let issue_count = py_result.issues.len() as u64;
                    let mut metadata = std::collections::HashMap::new();
                    metadata.insert("target".to_string(), host_owned);
                    let stats = ExecutionStats::new(0, 1, issue_count, 0);
                    Ok(operation_ok(
                        stats,
                        Some(metadata),
                        Some(OperationPayload::TlsInspection(py_result)),
                    ))
                }
                Err(e) => Ok(operation_err(e.to_string())),
            }
        })
    }

    fn run_tech_detect_async(&self, target: String) -> PyResult<runtime_async::PyFuture> {
        let host = extract_host_from_url(&target)?;
        self.state.scope.enforce_target(&host)?;

        let url_owned = target.clone();
        let event_tx = self.state.event_tx.clone();

        // Emit: operation started
        if let Some(ref tx) = event_tx {
            let _ = Python::with_gil(|py| {
                tx.try_send(crate::event_protocol::EventEnvelope::create(
                    "operation.started".to_string(),
                    crate::event_protocol::ProgressEvent::new(
                        0.0,
                        format!("Starting technology detection on {}", url_owned),
                        0,
                        0,
                    )
                    .into_py(py),
                    None,
                    None,
                    None,
                    None,
                ))
            });
        }

        runtime_async::spawn_async(async move {
            match eggsec::recon::techdetect::detect_tech_stack(&url_owned)
                .await
                .map_pyerr()
            {
                Ok(r) => {
                    let py_result = TechDetectionResult {
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
                    let tech_count = (py_result.tech_stack.servers.len()
                        + py_result.tech_stack.frameworks.len()
                        + py_result.tech_stack.languages.len()
                        + py_result.tech_stack.databases.len()
                        + py_result.tech_stack.cdns.len()
                        + py_result.tech_stack.cms.len()
                        + py_result.tech_stack.javascript.len()
                        + py_result.tech_stack.other.len())
                        as u64;
                    let mut metadata = std::collections::HashMap::new();
                    metadata.insert("target".to_string(), url_owned);
                    let stats = ExecutionStats::new(0, tech_count, 0, 0);
                    Ok(operation_ok(
                        stats,
                        Some(metadata),
                        Some(OperationPayload::TechnologyDetection(py_result)),
                    ))
                }
                Err(e) => Ok(operation_err(e.to_string())),
            }
        })
    }

    fn run_waf_detect_async(&self, target: String) -> PyResult<runtime_async::PyFuture> {
        let host = extract_host_from_url(&target)?;
        self.state.scope.enforce_target(&host)?;

        let url_owned = target.clone();
        let event_tx = self.state.event_tx.clone();

        // Emit: operation started
        if let Some(ref tx) = event_tx {
            let _ = Python::with_gil(|py| {
                tx.try_send(crate::event_protocol::EventEnvelope::create(
                    "operation.started".to_string(),
                    crate::event_protocol::ProgressEvent::new(
                        0.0,
                        format!("Starting WAF detection on {}", url_owned),
                        0,
                        0,
                    )
                    .into_py(py),
                    None,
                    None,
                    None,
                    None,
                ))
            });
        }

        runtime_async::spawn_async(async move {
            match async {
                let detector = eggsec::waf::WafDetector::new().map_pyerr()?;
                detector.detect(&url_owned).await.map_pyerr()
            }
            .await
            {
                Ok(r) => {
                    let py_result = WafDetectionResultPy {
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
                    let items = if py_result.detected { 1 } else { 0 };
                    let mut metadata = std::collections::HashMap::new();
                    metadata.insert("target".to_string(), url_owned);
                    let stats = ExecutionStats::new(0, items, 0, 0);
                    Ok(operation_ok(
                        stats,
                        Some(metadata),
                        Some(OperationPayload::WafDetection(py_result)),
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
        self.state.scope.enforce_target(&host)?;

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

        // Emit: operation started
        let event_tx = self.state.event_tx.clone();
        if let Some(ref tx) = event_tx {
            let _ = Python::with_gil(|py| {
                tx.try_send(crate::event_protocol::EventEnvelope::create(
                    "operation.started".to_string(),
                    crate::event_protocol::ProgressEvent::new(
                        0.0,
                        format!("Starting load test on {}", target),
                        0,
                        total_requests as usize,
                    )
                    .into_py(py),
                    None,
                    None,
                    None,
                    None,
                ))
            });
        }

        crate::loadtest::async_load_test_http(
            &target,
            total_requests,
            concurrency,
            timeout_secs,
            self.state.scope.clone(),
            &method,
        )
    }

    fn run_waf_validate_async(&self, target: String) -> PyResult<runtime_async::PyFuture> {
        let host = extract_host_from_url(&target)?;
        self.state.scope.enforce_target(&host)?;

        // Emit: operation started
        let event_tx = self.state.event_tx.clone();
        if let Some(ref tx) = event_tx {
            let _ = Python::with_gil(|py| {
                tx.try_send(crate::event_protocol::EventEnvelope::create(
                    "operation.started".to_string(),
                    crate::event_protocol::ProgressEvent::new(
                        0.0,
                        format!("Starting WAF validation on {}", target),
                        0,
                        0,
                    )
                    .into_py(py),
                    None,
                    None,
                    None,
                    None,
                ))
            });
        }

        crate::waf_validation::async_validate_waf(&target, self.state.scope.clone(), false, None)
    }

    fn run_fuzz_async(
        &self,
        target: String,
        payload_type: String,
        concurrency: usize,
        timeout: u64,
    ) -> PyResult<runtime_async::PyFuture> {
        let host = extract_host_from_url(&target)?;
        self.state.scope.enforce_target(&host)?;

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

        // Emit: operation started
        let event_tx = self.state.event_tx.clone();
        if let Some(ref tx) = event_tx {
            let _ = Python::with_gil(|py| {
                tx.try_send(crate::event_protocol::EventEnvelope::create(
                    "operation.started".to_string(),
                    crate::event_protocol::ProgressEvent::new(
                        0.0,
                        format!("Starting HTTP fuzz on {}", target),
                        0,
                        0,
                    )
                    .into_py(py),
                    None,
                    None,
                    None,
                    None,
                ))
            });
        }

        crate::waf_validation::async_fuzz_http(
            &target,
            self.state.scope.clone(),
            &payload_type,
            "GET",
            None,
            concurrency,
            timeout,
        )
    }
}
