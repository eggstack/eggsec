use std::collections::HashMap;
use std::sync::Arc;

use pyo3::prelude::*;
use pyo3::types::PyDict;

use crate::cancellation::CancellationToken;
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

/// Internal state for daemon-backed async engine execution.
#[cfg(feature = "daemon-client")]
#[derive(Clone)]
struct DaemonBackend {
    client: crate::daemon::DaemonClientPy,
    session_id: Option<String>,
    #[allow(dead_code)]
    socket_path: String,
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
        schema_version: "1.0".to_string(),
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
        schema_version: "1.0".to_string(),
    }
}

/// Convert a daemon `DaemonResponsePy` to an `OperationResult`.
#[cfg(feature = "daemon-client")]
fn daemon_response_to_operation_result(
    response: &crate::daemon::DaemonResponsePy,
    operation: &str,
) -> OperationResult {
    if response.ok {
        let mut metadata = std::collections::HashMap::new();
        metadata.insert("daemon_message".to_string(), response.message.clone());
        metadata.insert("daemon_request_id".to_string(), response.request_id.clone());
        metadata.insert("policy_decision".to_string(), "allow".to_string());
        OperationResult {
            status: ExecutionStatus::Completed(),
            stats: Some(ExecutionStats::new(0, 0, 0, 0)),
            artifacts: Vec::new(),
            error: None,
            metadata,
            payload: None,
            payload_type: None,
            schema_version: "1.0".to_string(),
        }
    } else {
        let error_msg = response
            .error_code
            .as_deref()
            .map(|code| format!("{}: {}", code, response.message))
            .unwrap_or_else(|| response.message.clone());
        operation_err_for(Some(operation), error_msg)
    }
}

/// Convert a Python dict to `HashMap<String, String>` for OperationRequest metadata.
///
/// Each value is converted to its string representation. Complex types (lists,
/// nested dicts) are serialized via Python's `json.dumps`.
fn pydict_to_string_metadata(dict: &Bound<'_, PyDict>) -> PyResult<HashMap<String, String>> {
    let mut map = HashMap::new();
    let json_mod = dict.py().import_bound("json")?;
    for (key, value) in dict.iter() {
        if let Ok(key_str) = key.extract::<String>() {
            let val_str: String = if let Ok(s) = value.extract::<String>() {
                s
            } else if let Ok(b) = value.extract::<bool>() {
                b.to_string()
            } else if let Ok(i) = value.extract::<i64>() {
                i.to_string()
            } else if let Ok(f) = value.extract::<f64>() {
                f.to_string()
            } else {
                // Fallback: use json.dumps for complex types (lists, dicts, None)
                let json_str_obj = json_mod.call_method1("dumps", (&value,))?;
                json_str_obj.extract()?
            };
            map.insert(key_str, val_str);
        }
    }
    Ok(map)
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
    #[cfg(feature = "daemon-client")]
    daemon_backend: Option<DaemonBackend>,
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

    /// Construct an async engine backed by the in-process Rust engine.
    ///
    /// This is equivalent to ``AsyncEngine(scope, mode=mode, ...)`` but uses the
    /// explicit constructor name for clarity in mixed local/daemon codebases.
    #[staticmethod]
    #[pyo3(signature = (scope, *, mode="manual", concurrency=100, timeout_ms=5000))]
    fn local(scope: Scope, mode: &str, concurrency: usize, timeout_ms: u64) -> PyResult<Self> {
        Self::new_inner(scope, mode, concurrency, timeout_ms)
    }

    /// Construct an async engine backed by a daemon over a Unix socket.
    ///
    /// The engine auto-creates a daemon session on first dispatch. All
    /// subsequent operations are submitted to the daemon using the same
    /// request DTOs as the local engine.
    ///
    /// Args:
    ///     socket_path: Path to the daemon Unix socket.
    ///     session_id: Optional pre-existing session ID. If not provided,
    ///                 a session is created automatically on first dispatch.
    ///     mode: Execution mode ("manual" or "automation").
    ///     concurrency: Max concurrent connections (default: 100).
    ///     timeout_ms: Connection timeout in milliseconds (default: 5000).
    ///
    /// Raises:
    ///     FeatureUnavailableError: If the daemon-client feature is not enabled.
    ///     NetworkError: If the connection fails.
    #[staticmethod]
    #[pyo3(signature = (socket_path, *, session_id=None, mode="manual", concurrency=100, timeout_ms=5000))]
    fn daemon(
        py: Python<'_>,
        socket_path: &str,
        session_id: Option<&str>,
        mode: &str,
        concurrency: usize,
        timeout_ms: u64,
    ) -> PyResult<Self> {
        #[cfg(feature = "daemon-client")]
        {
            let client = crate::daemon::daemon_connect(py, socket_path)?;
            let scope = Scope {
                inner: eggsec::config::Scope {
                    allowed_targets: vec![eggsec::config::ScopeRule {
                        pattern: "*".to_string(),
                        cidr: None,
                        description: Some("daemon wildcard scope".to_string()),
                    }],
                    require_explicit_scope: true,
                    ..Default::default()
                },
            };
            let state = EngineState::from_params(scope, mode, concurrency, timeout_ms)?;
            Ok(Self {
                state,
                daemon_backend: Some(DaemonBackend {
                    client,
                    session_id: session_id.map(|s| s.to_string()),
                    socket_path: socket_path.to_string(),
                }),
            })
        }
        #[cfg(not(feature = "daemon-client"))]
        {
            let _ = (py, socket_path, session_id, mode, concurrency, timeout_ms);
            Err(crate::error::FeatureUnavailableError::new_err(
                "Daemon client is not available. Rebuild with the `daemon-client` feature enabled.",
            ))
        }
    }

    /// Dispatch a generic operation request to the appropriate engine function.
    ///
    /// Routes through the OperationExecutorRegistry, which checks feature gates
    /// and provides "Did you mean?" suggestions for unknown operations.
    /// Returns a PyFuture that resolves to an OperationResult.
    ///
    /// When the engine was constructed via `AsyncEngine.daemon(...)`, the request
    /// is submitted to the daemon session instead of executing locally.
    fn run(&self, py: Python<'_>, request: OperationRequest) -> PyResult<runtime_async::PyFuture> {
        #[cfg(feature = "daemon-client")]
        {
            if let Some(daemon) = &self.daemon_backend {
                return self.run_via_daemon_async(py, &request, daemon);
            }
        }
        self.state
            .registry
            .execute_async(&request.operation, &request, self)
    }

    /// Invoke a tool by tool ID with a validated payload dictionary (async).
    ///
    /// This delegates through the engine's enforcement pipeline, preserving
    /// scope, policy, audit, timeout, cancellation, and rate-limit behavior.
    ///
    /// Args:
    ///     tool_id: The tool identifier (e.g. "scan_ports", "fuzz_http").
    ///     target: Target string (URL, domain, IP, or CIDR).
    ///     payload: Optional dict of tool-specific parameters.
    ///     timeout_ms: Optional timeout override in milliseconds.
    ///
    /// Returns:
    ///     PyFuture: A future that resolves to OperationResult.
    #[pyo3(signature = (tool_id, target, payload=None, timeout_ms=None))]
    fn async_invoke_tool(
        &self,
        py: Python<'_>,
        tool_id: &str,
        target: &str,
        payload: Option<&Bound<'_, PyDict>>,
        timeout_ms: Option<u64>,
    ) -> PyResult<runtime_async::PyFuture> {
        let metadata = if let Some(p) = payload {
            pydict_to_string_metadata(p)?
        } else {
            HashMap::new()
        };
        let request = OperationRequest::new(
            tool_id.to_string(),
            target.to_string(),
            timeout_ms,
            Some(metadata),
        );
        self.run(py, request)
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
        self.run_port_scan_async(request.target.clone(), ports, effective_timeout, None, None)
    }

    /// Run an endpoint scan (async).
    #[pyo3(signature = (request,))]
    fn run_endpoint_scan(&self, request: EndpointScanRequest) -> PyResult<runtime_async::PyFuture> {
        self.state
            .pre_dispatch_validate("scan_endpoints", &request.target)?;
        let endpoints = request.paths.clone().unwrap_or_default();
        let effective_timeout = request.timeout_ms.unwrap_or(self.state.timeout_ms);
        self.run_endpoint_scan_async(
            request.target.clone(),
            endpoints,
            effective_timeout,
            None,
            None,
        )
    }

    /// Run service fingerprinting (async).
    #[pyo3(signature = (request,))]
    fn run_fingerprint(&self, request: FingerprintRequest) -> PyResult<runtime_async::PyFuture> {
        self.state
            .pre_dispatch_validate("fingerprint_services", &request.target)?;
        let ports = request.ports.clone().unwrap_or_else(|| vec![80, 443]);
        let effective_timeout = request.timeout_ms.unwrap_or(self.state.timeout_ms);
        self.run_fingerprint_async(request.target.clone(), ports, effective_timeout, None, None)
    }

    /// Run DNS reconnaissance (async).
    #[pyo3(signature = (request,))]
    fn run_recon_dns(&self, request: ReconDnsRequest) -> PyResult<runtime_async::PyFuture> {
        self.state
            .pre_dispatch_validate("recon_dns", &request.target)?;
        self.run_recon_dns_async(request.target.clone(), None, None)
    }

    /// Run TLS inspection (async).
    #[pyo3(signature = (request,))]
    fn run_tls_inspect(&self, request: TlsInspectRequest) -> PyResult<runtime_async::PyFuture> {
        self.state
            .pre_dispatch_validate("inspect_tls", &request.target)?;
        self.run_tls_inspect_async(request.target.clone(), None, None)
    }

    /// Run technology detection (async).
    #[pyo3(signature = (request,))]
    fn run_tech_detect(&self, request: TechDetectRequest) -> PyResult<runtime_async::PyFuture> {
        self.state
            .pre_dispatch_validate("detect_technology", &request.target)?;
        self.run_tech_detect_async(request.target.clone(), None, None)
    }

    /// Run WAF detection (async).
    #[pyo3(signature = (request,))]
    fn run_waf_detect(&self, request: WafDetectRequest) -> PyResult<runtime_async::PyFuture> {
        self.state
            .pre_dispatch_validate("detect_waf", &request.target)?;
        self.run_waf_detect_async(request.target.clone(), None, None)
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
            None,
            None,
        )
    }

    /// Run WAF validation (async).
    #[pyo3(signature = (request,))]
    fn run_waf_validate(&self, request: WafValidateRequest) -> PyResult<runtime_async::PyFuture> {
        self.state
            .pre_dispatch_validate("validate_waf", &request.target)?;
        self.run_waf_validate_async(request.target.clone(), None, None)
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
        self.run_fuzz_async(
            request.target.clone(),
            payload_type,
            threads,
            timeout,
            None,
            None,
        )
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
        Ok(Self {
            state,
            #[cfg(feature = "daemon-client")]
            daemon_backend: None,
        })
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
        Ok(Self {
            state,
            #[cfg(feature = "daemon-client")]
            daemon_backend: None,
        })
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
        cancel_token: Option<CancellationToken>,
    ) -> PyResult<runtime_async::PyFuture> {
        use crate::event_protocol::{
            CancellationEvent, EventEnvelope, PlanningEvent, PreflightEvent,
        };
        use crate::operation_registry::StableOperation;
        let op = request.operation.clone();
        let target = request.target.clone();

        // Emit: operation.planning (need GIL for Python objects)
        let planning_event = EventEnvelope::create(
            "operation.planning".to_string(),
            Python::with_gil(|py| {
                PlanningEvent::new(op.clone(), target.clone(), String::new()).into_py(py)
            }),
            None,
            None,
            Some(target.clone()),
            None,
        );
        self.state.emit_event(planning_event);

        // Pre-dispatch validation: scope, feature gates, audit logging.
        self.state.pre_dispatch_validate(&op, &target)?;

        // Emit: operation.preflight
        let preflight_event = EventEnvelope::create(
            "operation.preflight".to_string(),
            Python::with_gil(|py| {
                PreflightEvent::new("approved".to_string(), Vec::new(), Vec::new()).into_py(py)
            }),
            None,
            None,
            Some(target.clone()),
            None,
        );
        self.state.emit_event(preflight_event);

        // Compute deadline from request or engine timeout
        let deadline = request
            .timeout_ms
            .or(Some(self.state.timeout_ms))
            .map(|ms| std::time::Instant::now() + std::time::Duration::from_millis(ms));

        // Clone cancel_token for move into async closures
        let cancel_token_clone = cancel_token.clone();

        let operation = StableOperation::parse(&op).ok_or_else(|| {
            pyo3::exceptions::PyValueError::new_err(format!("Unknown operation: {}", op))
        })?;

        // Cancellation check helper macro
        macro_rules! check_cancel {
            ($op_id:expr) => {
                if let Some(ref token) = cancel_token {
                    if token.is_cancelled() {
                        let reason = token.reason().unwrap_or_else(|| "cancelled".to_string());
                        Python::with_gil(|py| {
                            self.state.emit_event(EventEnvelope::create(
                                "operation.cancelled".to_string(),
                                CancellationEvent::new(reason, "operator".to_string()).into_py(py),
                                None,
                                None,
                                Some($op_id.to_string()),
                                None,
                            ));
                        });
                        return Err(pyo3::exceptions::PyRuntimeError::new_err(
                            "Operation cancelled",
                        ));
                    }
                }
            };
        }

        match operation {
            StableOperation::ScanPorts => {
                check_cancel!("scan_ports");
                let target = request.target.clone();
                let ports_str = request
                    .metadata
                    .get("ports")
                    .cloned()
                    .unwrap_or_else(|| "1-1024".to_string());
                let effective_timeout = request.timeout_ms.unwrap_or(self.state.timeout_ms);
                let ports = parse_ports_string(&ports_str)?;
                self.run_port_scan_async(
                    target,
                    ports,
                    effective_timeout,
                    cancel_token_clone,
                    deadline,
                )
            }
            StableOperation::ScanEndpoints => {
                check_cancel!("scan_endpoints");
                let target = request.target.clone();
                let endpoints: Vec<String> = request
                    .metadata
                    .get("endpoints")
                    .map(|s| s.split(',').map(|s| s.trim().to_string()).collect())
                    .unwrap_or_default();
                let effective_timeout = request.timeout_ms.unwrap_or(self.state.timeout_ms);
                self.run_endpoint_scan_async(
                    target,
                    endpoints,
                    effective_timeout,
                    cancel_token_clone,
                    deadline,
                )
            }
            StableOperation::FingerprintServices => {
                check_cancel!("fingerprint_services");
                let target = request.target.clone();
                let ports_str = request.metadata.get("ports").cloned().unwrap_or_default();
                let ports = if ports_str.is_empty() {
                    vec![80, 443]
                } else {
                    parse_ports_string(&ports_str)?
                };
                let effective_timeout = request.timeout_ms.unwrap_or(self.state.timeout_ms);
                self.run_fingerprint_async(
                    target,
                    ports,
                    effective_timeout,
                    cancel_token_clone,
                    deadline,
                )
            }
            StableOperation::ReconDns => {
                check_cancel!("recon_dns");
                self.run_recon_dns_async(request.target.clone(), cancel_token_clone, deadline)
            }
            StableOperation::InspectTls => {
                check_cancel!("inspect_tls");
                self.run_tls_inspect_async(request.target.clone(), cancel_token_clone, deadline)
            }
            StableOperation::DetectTechnology => {
                check_cancel!("detect_technology");
                self.run_tech_detect_async(request.target.clone(), cancel_token_clone, deadline)
            }
            StableOperation::DetectWaf => {
                check_cancel!("detect_waf");
                self.run_waf_detect_async(request.target.clone(), cancel_token_clone, deadline)
            }
            StableOperation::LoadTest => {
                check_cancel!("load_test");
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
                    cancel_token_clone,
                    deadline,
                )
            }
            StableOperation::ValidateWaf => {
                check_cancel!("validate_waf");
                self.run_waf_validate_async(request.target.clone(), cancel_token_clone, deadline)
            }
            StableOperation::FuzzHttp => {
                check_cancel!("fuzz_http");
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
                    cancel_token_clone,
                    deadline,
                )
            }
            #[cfg(feature = "git-secrets")]
            StableOperation::ScanGitSecrets => {
                check_cancel!("scan_git_secrets");
                let repo_path = request
                    .metadata
                    .get("repo_path")
                    .cloned()
                    .unwrap_or_else(|| request.target.clone());
                let max_commits: usize = request
                    .metadata
                    .get("max_commits")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(1000);
                self.run_git_secrets_async(repo_path, max_commits)
            }
            #[cfg(feature = "sbom")]
            StableOperation::GenerateSbom => {
                check_cancel!("generate_sbom");
                let project_path = request
                    .metadata
                    .get("project_path")
                    .cloned()
                    .unwrap_or_else(|| request.target.clone());
                let ecosystem = request
                    .metadata
                    .get("ecosystem")
                    .cloned()
                    .unwrap_or_else(|| "cargo".to_string());
                let format = request
                    .metadata
                    .get("format")
                    .cloned()
                    .unwrap_or_else(|| "cyclonedx".to_string());
                self.run_sbom_async(project_path, ecosystem, format)
            }
            StableOperation::RunConsolidatedRecon => {
                check_cancel!("run_consolidated_recon");
                let run_dns = request
                    .metadata
                    .get("run_dns")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(true);
                let run_ssl = request
                    .metadata
                    .get("run_ssl")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(true);
                let run_tech_detect = request
                    .metadata
                    .get("run_tech_detect")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(true);
                let run_subdomain = request
                    .metadata
                    .get("run_subdomain")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(true);
                let run_whois = request
                    .metadata
                    .get("run_whois")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(true);
                let run_cors = request
                    .metadata
                    .get("run_cors")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(true);
                let run_wayback = request
                    .metadata
                    .get("run_wayback")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(true);
                let run_js_analysis = request
                    .metadata
                    .get("run_js_analysis")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(true);
                let run_content = request
                    .metadata
                    .get("run_content")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(true);
                let run_email = request
                    .metadata
                    .get("run_email")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(true);
                let timeout_secs = request
                    .metadata
                    .get("timeout_secs")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(30);
                let concurrency = request
                    .metadata
                    .get("concurrency")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(10);
                let config = crate::consolidated_recon::ConsolidatedReconConfigPy {
                    run_dns,
                    run_ssl,
                    run_tech_detect,
                    run_subdomain,
                    run_whois,
                    run_cors,
                    run_wayback,
                    run_js_analysis,
                    run_content,
                    run_email,
                    timeout_secs,
                    concurrency,
                };
                self.run_consolidated_recon_async(request.target, config)
            }
            StableOperation::GraphqlTest => {
                check_cancel!("graphql_test");
                let enable_introspection = request
                    .metadata
                    .get("enable_introspection")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(true);
                let enable_depth_bypass = request
                    .metadata
                    .get("enable_depth_bypass")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(true);
                let enable_alias_overload = request
                    .metadata
                    .get("enable_alias_overload")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(true);
                let timeout_secs = request
                    .metadata
                    .get("timeout_secs")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(10);
                let config = crate::graphql::GraphQLTestConfigPy {
                    endpoint: request.target.clone(),
                    enable_introspection,
                    enable_depth_bypass,
                    enable_alias_overload,
                    timeout_secs,
                };
                self.run_graphql_async(config)
            }
            StableOperation::OauthTest => {
                let client_id = request
                    .metadata
                    .get("client_id")
                    .cloned()
                    .unwrap_or_default();
                let redirect_uri = request
                    .metadata
                    .get("redirect_uri")
                    .cloned()
                    .unwrap_or_default();
                let client_secret = request.metadata.get("client_secret").cloned();
                let issuer_url = request.metadata.get("issuer_url").cloned();
                let enable_redirect_test = request
                    .metadata
                    .get("enable_redirect_test")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(true);
                let enable_scope_test = request
                    .metadata
                    .get("enable_scope_test")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(true);
                let enable_state_test = request
                    .metadata
                    .get("enable_state_test")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(true);
                let enable_grant_test = request
                    .metadata
                    .get("enable_grant_test")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(true);
                let timeout_secs = request
                    .metadata
                    .get("timeout_secs")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(10);
                let config = crate::oauth::OAuthTestConfigPy {
                    client_id,
                    redirect_uri,
                    client_secret,
                    issuer_url,
                    enable_redirect_test,
                    enable_scope_test,
                    enable_state_test,
                    enable_grant_test,
                    timeout_secs,
                };
                let auth_endpoint = request
                    .metadata
                    .get("auth_endpoint")
                    .cloned()
                    .unwrap_or_else(|| request.target.clone());
                self.run_oauth_async(config, auth_endpoint)
            }
            StableOperation::AuthTest => {
                check_cancel!("auth_test");
                self.run_auth_test_async(request.target.clone())
            }
            #[cfg(feature = "db-pentest")]
            StableOperation::DbProbe => {
                check_cancel!("db_probe");
                let db_type = request
                    .metadata
                    .get("db_type")
                    .cloned()
                    .unwrap_or_else(|| "all".to_string());
                let user = request.metadata.get("username").cloned();
                let password = request.metadata.get("password").cloned();
                let database = request.metadata.get("database").cloned();
                let port: Option<u16> = request.metadata.get("port").and_then(|s| s.parse().ok());
                self.run_db_probe_async(request.target, db_type, user, password, database, port)
            }
            #[cfg(feature = "nse")]
            StableOperation::NseRun => {
                check_cancel!("nse_run");
                let scripts: Vec<String> = request
                    .metadata
                    .get("scripts")
                    .map(|s| {
                        s.split(',')
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect()
                    })
                    .unwrap_or_default();
                let script_name = scripts
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "default".to_string());
                let script_args = request.metadata.get("script_args").cloned();
                self.run_nse_async(request.target, script_name, script_args)
            }
            #[cfg(feature = "container")]
            StableOperation::ScanDockerImage => {
                check_cancel!("scan_docker_image");
                let image = request
                    .metadata
                    .get("image")
                    .cloned()
                    .unwrap_or_else(|| request.target.clone());
                self.run_docker_image_async(image)
            }
            #[cfg(feature = "container")]
            StableOperation::ScanKubernetes => {
                check_cancel!("scan_kubernetes");
                let api_server = request
                    .metadata
                    .get("api_server")
                    .cloned()
                    .unwrap_or_else(|| request.target.clone());
                let token = request.metadata.get("token").cloned();
                let timeout_secs: u64 = request
                    .metadata
                    .get("timeout_secs")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(30);
                self.run_kubernetes_async(api_server, token, timeout_secs)
            }
            #[cfg(feature = "mobile")]
            StableOperation::AnalyzeApk => {
                check_cancel!("analyze_apk");
                let apk_path = request
                    .metadata
                    .get("apk_path")
                    .cloned()
                    .unwrap_or_else(|| request.target.clone());
                self.run_apk_async(apk_path)
            }
            #[cfg(feature = "mobile")]
            StableOperation::AnalyzeIpa => {
                check_cancel!("analyze_ipa");
                let ipa_path = request
                    .metadata
                    .get("ipa_path")
                    .cloned()
                    .unwrap_or_else(|| request.target.clone());
                self.run_ipa_async(ipa_path)
            }
            _ => {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "Operation '{}' is not available in this build configuration",
                    op
                )));
            }
        }
    }

    /// Dispatch an OperationRequest through the daemon backend (async).
    ///
    /// Creates a session on first use if none was provided at construction time.
    /// Converts the request to a `TaskKind` JSON payload and submits it to the
    /// daemon, returning a PyFuture that resolves to an `OperationResult`.
    #[cfg(feature = "daemon-client")]
    fn run_via_daemon_async(
        &self,
        py: Python<'_>,
        request: &OperationRequest,
        daemon: &DaemonBackend,
    ) -> PyResult<runtime_async::PyFuture> {
        // Ensure we have a session ID (sync step — quick I/O)
        let session_id = match &daemon.session_id {
            Some(sid) => sid.clone(),
            None => {
                let client = daemon.client.clone();
                let response: crate::daemon::DaemonResponsePy =
                    crate::runtime_sync::block_on(py, async move {
                        let client_arc = client.client.clone();
                        let mut guard = client_arc.lock().await;
                        let inner = guard.as_mut().ok_or_else(|| {
                            crate::error::NetworkError::new_err("daemon client is closed")
                        })?;
                        let msg = inner
                            .create_session(eggsec_runtime::RuntimeSurface::CliManual, None, vec![])
                            .await
                            .map_err(|e| {
                                crate::error::NetworkError::new_err(format!(
                                    "daemon create_session failed: {}",
                                    e
                                ))
                            })?;
                        Ok::<_, PyErr>(crate::daemon::server_message_to_response(msg))
                    })?;
                response
                    .message
                    .strip_prefix("session_id=")
                    .map(|s| s.to_string())
                    .ok_or_else(|| {
                        crate::error::NetworkError::new_err(format!(
                            "Unexpected create_session response: {}",
                            response.message
                        ))
                    })?
            }
        };

        // Convert request to TaskKind JSON
        let task_kind_json = crate::engine::operation_request_to_task_kind_json(request)?;
        let client = daemon.client.clone();
        let sid = session_id.clone();
        let op = request.operation.clone();

        runtime_async::spawn_async(async move {
            let client_arc = client.client.clone();
            let task_kind: eggsec_runtime::TaskKind = serde_json::from_str(&task_kind_json)
                .map_err(|e| {
                    crate::error::ConfigError::new_err(format!("Invalid task_kind JSON: {}", e))
                })?;
            let run_request = eggsec_runtime::RunRequest {
                task_kind,
                requested_by: None,
                surface: eggsec_runtime::RuntimeSurface::CliManual,
                labels: vec![],
            };
            let session_id: eggsec_runtime::SessionId = sid.parse().map_err(|_| {
                crate::error::ConfigError::new_err(format!("Invalid session ID: {}", sid))
            })?;
            let mut guard = client_arc.lock().await;
            let inner = guard
                .as_mut()
                .ok_or_else(|| crate::error::NetworkError::new_err("daemon client is closed"))?;
            let msg = inner
                .submit_task(session_id, run_request)
                .await
                .map_err(|e| {
                    crate::error::NetworkError::new_err(format!("daemon submit_task failed: {}", e))
                })?;
            let response = crate::daemon::server_message_to_response(msg);
            Ok(daemon_response_to_operation_result(&response, &op))
        })
    }

    fn run_port_scan_async(
        &self,
        target: String,
        ports: Vec<u16>,
        effective_timeout_ms: u64,
        cancel_token: Option<CancellationToken>,
        deadline: Option<std::time::Instant>,
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
            // Deadline check
            if let Some(dl) = deadline {
                if std::time::Instant::now() >= dl {
                    return Ok(operation_err("Operation timed out".to_string()));
                }
            }
            // Cancellation check
            if let Some(ref token) = cancel_token {
                if token.is_cancelled() {
                    return Ok(operation_err("Operation cancelled".to_string()));
                }
            }

            let config = eggsec::scanner::PortScanConfig {
                ports,
                concurrency: effective_concurrency,
                timeout_duration: std::time::Duration::from_millis(effective_timeout_ms),
                tui_mode: false,
                spoof_config: eggsec::scanner::SpoofConfig::default(),
                progress_tx: None,
                max_results: None,
            };

            // Apply per-operation timeout if deadline is set
            let scan_future = eggsec::scanner::scan_ports(&target_owned, config);
            let result = if let Some(dl) = deadline {
                let remaining = dl.saturating_duration_since(std::time::Instant::now());
                if remaining.is_zero() {
                    return Ok(operation_err("Operation timed out".to_string()));
                }
                match tokio::time::timeout(remaining, scan_future).await {
                    Ok(r) => r.map_pyerr(),
                    Err(_) => return Ok(operation_err("Operation timed out".to_string())),
                }
            } else {
                scan_future.await.map_pyerr()
            };

            match result {
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
        cancel_token: Option<CancellationToken>,
        deadline: Option<std::time::Instant>,
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
            // Deadline check
            if let Some(dl) = deadline {
                if std::time::Instant::now() >= dl {
                    return Ok(operation_err("Operation timed out".to_string()));
                }
            }
            // Cancellation check
            if let Some(ref token) = cancel_token {
                if token.is_cancelled() {
                    return Ok(operation_err("Operation cancelled".to_string()));
                }
            }

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
        cancel_token: Option<CancellationToken>,
        deadline: Option<std::time::Instant>,
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
            // Deadline check
            if let Some(dl) = deadline {
                if std::time::Instant::now() >= dl {
                    return Ok(operation_err("Operation timed out".to_string()));
                }
            }
            // Cancellation check
            if let Some(ref token) = cancel_token {
                if token.is_cancelled() {
                    return Ok(operation_err("Operation cancelled".to_string()));
                }
            }

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

    fn run_recon_dns_async(
        &self,
        target: String,
        cancel_token: Option<CancellationToken>,
        deadline: Option<std::time::Instant>,
    ) -> PyResult<runtime_async::PyFuture> {
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
            // Deadline check
            if let Some(dl) = deadline {
                if std::time::Instant::now() >= dl {
                    return Ok(operation_err("Operation timed out".to_string()));
                }
            }
            // Cancellation check
            if let Some(ref token) = cancel_token {
                if token.is_cancelled() {
                    return Ok(operation_err("Operation cancelled".to_string()));
                }
            }

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

    fn run_tls_inspect_async(
        &self,
        target: String,
        cancel_token: Option<CancellationToken>,
        deadline: Option<std::time::Instant>,
    ) -> PyResult<runtime_async::PyFuture> {
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
            // Deadline check
            if let Some(dl) = deadline {
                if std::time::Instant::now() >= dl {
                    return Ok(operation_err("Operation timed out".to_string()));
                }
            }
            // Cancellation check
            if let Some(ref token) = cancel_token {
                if token.is_cancelled() {
                    return Ok(operation_err("Operation cancelled".to_string()));
                }
            }

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

    fn run_tech_detect_async(
        &self,
        target: String,
        cancel_token: Option<CancellationToken>,
        deadline: Option<std::time::Instant>,
    ) -> PyResult<runtime_async::PyFuture> {
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
            // Deadline check
            if let Some(dl) = deadline {
                if std::time::Instant::now() >= dl {
                    return Ok(operation_err("Operation timed out".to_string()));
                }
            }
            // Cancellation check
            if let Some(ref token) = cancel_token {
                if token.is_cancelled() {
                    return Ok(operation_err("Operation cancelled".to_string()));
                }
            }

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

    fn run_waf_detect_async(
        &self,
        target: String,
        cancel_token: Option<CancellationToken>,
        deadline: Option<std::time::Instant>,
    ) -> PyResult<runtime_async::PyFuture> {
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
            // Deadline check
            if let Some(dl) = deadline {
                if std::time::Instant::now() >= dl {
                    return Ok(operation_err("Operation timed out".to_string()));
                }
            }
            // Cancellation check
            if let Some(ref token) = cancel_token {
                if token.is_cancelled() {
                    return Ok(operation_err("Operation cancelled".to_string()));
                }
            }

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
        _cancel_token: Option<CancellationToken>,
        _deadline: Option<std::time::Instant>,
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

    fn run_waf_validate_async(
        &self,
        target: String,
        _cancel_token: Option<CancellationToken>,
        _deadline: Option<std::time::Instant>,
    ) -> PyResult<runtime_async::PyFuture> {
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
        _cancel_token: Option<CancellationToken>,
        _deadline: Option<std::time::Instant>,
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

    #[cfg(feature = "git-secrets")]
    fn run_git_secrets_async(
        &self,
        repo_path: String,
        max_commits: usize,
    ) -> PyResult<runtime_async::PyFuture> {
        runtime_async::spawn_async(async move {
            let result = eggsec::recon::git_secrets::scan_git_secrets(&repo_path, max_commits)
                .map_err(|e| anyhow::anyhow!("Git secrets scan failed: {}", e))?;
            Ok(crate::git_secrets::GitSecretsReportPy {
                repo_path: result.repo_path,
                commits_scanned: result.commits_scanned,
                files_scanned: result.files_scanned,
                findings: result
                    .findings
                    .into_iter()
                    .map(crate::git_secrets::GitSecretFindingPy::from_engine)
                    .collect(),
                summary: crate::git_secrets::GitSecretsSummaryPy::from_engine(result.summary),
            })
        })
    }

    #[cfg(feature = "sbom")]
    fn run_sbom_async(
        &self,
        project_path: String,
        ecosystem: String,
        format: String,
    ) -> PyResult<runtime_async::PyFuture> {
        let sbom_format = crate::sbom::SbomFormatPy::from_str_py(&format)
            .unwrap_or(crate::sbom::SbomFormatPy::Cyclonedx);
        let engine_format = sbom_format.to_engine();
        runtime_async::spawn_async(async move {
            let gen = eggsec::supply_chain::sbom::SbomGenerator::new();
            let r = match ecosystem.as_str() {
                "cargo" => gen.generate_from_cargo(&project_path, engine_format),
                "npm" => gen.generate_from_npm(&project_path, engine_format),
                "pip" => gen.generate_from_requirements(&project_path, engine_format),
                other => return Err(anyhow::anyhow!("Unsupported ecosystem: '{}'", other)),
            };
            let result = r.map_err(|e| anyhow::anyhow!("SBOM generation failed: {}", e))?;
            Ok(crate::sbom::SbomReportPy {
                format: crate::sbom::SbomFormatPy::from_engine(result.format),
                project_name: result.project_name,
                version: result.version,
                generated_at: result.generated_at,
                components: result
                    .components
                    .into_iter()
                    .map(crate::sbom::SbomComponentPy::from_engine)
                    .collect(),
                vulnerabilities: result
                    .vulnerabilities
                    .into_iter()
                    .map(crate::sbom::SbomVulnerabilityPy::from_engine)
                    .collect(),
            })
        })
    }

    fn run_consolidated_recon_async(
        &self,
        target: String,
        config: crate::consolidated_recon::ConsolidatedReconConfigPy,
    ) -> PyResult<runtime_async::PyFuture> {
        runtime_async::spawn_async(async move {
            let mut modules = Vec::new();
            if config.run_dns {
                let module_result =
                    match eggsec::recon::dns_records::enumerate_dns_records(&target).await {
                        Ok(_) => crate::consolidated_recon::ReconModuleResultPy {
                            module: "dns_records".to_string(),
                            success: true,
                            data: Some("DNS records enumerated successfully".to_string()),
                            error: None,
                        },
                        Err(e) => crate::consolidated_recon::ReconModuleResultPy {
                            module: "dns_records".to_string(),
                            success: false,
                            data: None,
                            error: Some(e.to_string()),
                        },
                    };
                modules.push(module_result);
            }
            if config.run_ssl {
                let module_result = match eggsec::recon::ssl::analyze_ssl(&target, 443).await {
                    Ok(_) => crate::consolidated_recon::ReconModuleResultPy {
                        module: "ssl".to_string(),
                        success: true,
                        data: Some("SSL/TLS analysis completed".to_string()),
                        error: None,
                    },
                    Err(e) => crate::consolidated_recon::ReconModuleResultPy {
                        module: "ssl".to_string(),
                        success: false,
                        data: None,
                        error: Some(e.to_string()),
                    },
                };
                modules.push(module_result);
            }
            if config.run_tech_detect {
                let module_result =
                    match eggsec::recon::techdetect::detect_tech_stack(&target).await {
                        Ok(_) => crate::consolidated_recon::ReconModuleResultPy {
                            module: "tech_detect".to_string(),
                            success: true,
                            data: Some("Technology detection completed".to_string()),
                            error: None,
                        },
                        Err(e) => crate::consolidated_recon::ReconModuleResultPy {
                            module: "tech_detect".to_string(),
                            success: false,
                            data: None,
                            error: Some(e.to_string()),
                        },
                    };
                modules.push(module_result);
            }
            let modules_succeeded = modules.iter().filter(|m| m.success).count();
            let modules_failed = modules.len() - modules_succeeded;
            Ok(crate::consolidated_recon::ConsolidatedReconReportPy {
                target,
                modules_run: modules.len(),
                modules_succeeded,
                modules_failed,
                modules,
            })
        })
    }

    fn run_graphql_async(
        &self,
        config: crate::graphql::GraphQLTestConfigPy,
    ) -> PyResult<runtime_async::PyFuture> {
        runtime_async::spawn_async(async move {
            let mut fuzzer =
                eggsec::fuzzer::payloads::graphql::GraphQLFuzzer::new(config.endpoint.clone())
                    .with_introspection(config.enable_introspection)
                    .with_depth_bypass(config.enable_depth_bypass)
                    .with_alias_overload(config.enable_alias_overload);
            let mut results = Vec::new();
            results.extend(fuzzer.test_introspection_enabled());
            results.extend(fuzzer.generate_injection_queries(
                config.enable_depth_bypass,
                config.enable_alias_overload,
            ));
            results.extend(fuzzer.generate_batch_queries(config.enable_alias_overload));
            Ok(results
                .into_iter()
                .map(crate::graphql::GraphQLTestResultPy::from_engine)
                .collect::<Vec<_>>())
        })
    }

    fn run_oauth_async(
        &self,
        config: crate::oauth::OAuthTestConfigPy,
        auth_endpoint: String,
    ) -> PyResult<runtime_async::PyFuture> {
        runtime_async::spawn_async(async move {
            let fuzzer = crate::oauth::build_oauth_fuzzer(&config)?;
            let mut results = Vec::new();
            if config.enable_redirect_test {
                results.extend(fuzzer.test_redirect_uri(&auth_endpoint));
            }
            if config.enable_state_test {
                results.extend(fuzzer.test_state_parameter(&auth_endpoint));
            }
            if config.enable_scope_test {
                results.extend(fuzzer.test_scope_escalation(&auth_endpoint));
            }
            Ok::<_, PyErr>(
                results
                    .into_iter()
                    .map(crate::oauth::OAuthTestResultPy::from_engine)
                    .collect::<Vec<_>>(),
            )
        })
    }

    fn run_auth_test_async(&self, target: String) -> PyResult<runtime_async::PyFuture> {
        runtime_async::spawn_async(async move {
            let mut engine = eggsec::auth::AuthEngine::new(100, 10, 30, true).map_pyerr()?;
            let report = engine.run_full_test(&target).await.map_pyerr()?;
            Ok(crate::auth_assess::AuthTestReportPy::from_engine(report))
        })
    }

    #[cfg(feature = "db-pentest")]
    fn run_db_probe_async(
        &self,
        target: String,
        db_type: String,
        user: Option<String>,
        password: Option<String>,
        database: Option<String>,
        port: Option<u16>,
    ) -> PyResult<runtime_async::PyFuture> {
        let args = crate::db_pentest::build_args(
            Some(&target),
            Some(&db_type),
            "all",
            200,
            120,
            false,
            None,
            port,
            user.as_deref(),
            password.as_deref(),
            database.as_deref(),
        );
        crate::db_pentest::run_async(args)
    }

    #[cfg(feature = "nse")]
    fn run_nse_async(
        &self,
        target: String,
        script: String,
        script_args: Option<String>,
    ) -> PyResult<runtime_async::PyFuture> {
        let config = crate::nse::build_nse_config(&target, &script, script_args.as_deref(), false);
        crate::nse::run_nse_async(config, None)
    }

    #[cfg(feature = "container")]
    fn run_docker_image_async(&self, image_name: String) -> PyResult<runtime_async::PyFuture> {
        runtime_async::spawn_async(async move {
            let scanner = eggsec::container::docker::DockerScanner::new();
            let result = scanner
                .scan_image(&image_name)
                .await
                .map_err(|e| anyhow::anyhow!("Docker image scan failed: {}", e))?;
            Ok(crate::container::DockerScanResultPy::from_engine(result))
        })
    }

    #[cfg(feature = "container")]
    fn run_kubernetes_async(
        &self,
        api_server: String,
        token: Option<String>,
        timeout_secs: u64,
    ) -> PyResult<runtime_async::PyFuture> {
        runtime_async::spawn_async(async move {
            let scanner = eggsec::container::kubernetes::KubernetesScanner::new(
                &api_server,
                token,
                timeout_secs,
            )
            .map_err(|e| anyhow::anyhow!("Kubernetes scanner init failed: {}", e))?;
            let result = scanner
                .scan()
                .await
                .map_err(|e| anyhow::anyhow!("Kubernetes scan failed: {}", e))?;
            Ok(crate::container::KubernetesScanResultPy::from_engine(
                result,
            ))
        })
    }

    #[cfg(feature = "mobile")]
    fn run_apk_async(&self, apk_path: String) -> PyResult<runtime_async::PyFuture> {
        runtime_async::spawn_async(async move {
            let path_ref = std::path::Path::new(&apk_path);
            let result = eggsec::mobile::analyze_apk(path_ref).await.map_err(|e| {
                pyo3::exceptions::PyValueError::new_err(format!("APK analysis failed: {}", e))
            })?;
            Ok(crate::mobile::MobileScanReportPy::from_engine(result))
        })
    }

    #[cfg(feature = "mobile")]
    fn run_ipa_async(&self, ipa_path: String) -> PyResult<runtime_async::PyFuture> {
        runtime_async::spawn_async(async move {
            let path_ref = std::path::Path::new(&ipa_path);
            let result = eggsec::mobile::analyze_ipa(path_ref).await.map_err(|e| {
                pyo3::exceptions::PyValueError::new_err(format!("IPA analysis failed: {}", e))
            })?;
            Ok(crate::mobile::MobileScanReportPy::from_engine(result))
        })
    }
}
