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
use crate::planning::ScanPlan;
use crate::recon::{DnsRecordSet, TechDetectionResult, TlsInspectionResult};
use crate::requests::*;
use crate::runtime_sync;
use crate::scope::Scope;
use crate::status::{ExecutionStats, OperationPayload, OperationResult};
use crate::waf::WafDetectionResultPy;

/// Internal state for daemon-backed engine execution.
///
/// When present, `Engine::run()` routes requests through the daemon
/// instead of the local in-process engine.
#[cfg(feature = "daemon-client")]
#[derive(Clone)]
struct DaemonBackend {
    client: crate::daemon::DaemonClientPy,
    session_id: Option<String>,
    #[allow(dead_code)]
    socket_path: String,
}

/// Sync engine for running scoped security operations.
///
/// Wraps the Rust engine directly. Each `run_*` method returns an `OperationResult`
/// instead of raising exceptions — errors are captured in the result status.
///
/// The engine holds a shared `EngineState` (via `Arc`) that is also used by
/// `AsyncEngine`, ensuring every operation passes through common validation,
/// scope enforcement, feature gating, and audit logging.
#[pyclass]
#[derive(Clone)]
pub struct Engine {
    pub(crate) state: Arc<EngineState>,
    #[cfg(feature = "daemon-client")]
    daemon_backend: Option<DaemonBackend>,
}

#[cfg(feature = "daemon-client")]
use crate::dispatch_helpers::daemon_response_to_operation_result;
use crate::dispatch_helpers::{
    emit_finding_event, extract_host_from_url, operation_err, operation_ok, parse_ports_string,
    pydict_to_string_metadata,
};

/// Convert an OperationRequest to a TaskKind JSON string for daemon submission.
///
/// Delegates to the registry-driven `operation_request_to_daemon_task` which
/// uses `OperationExecutorDescriptor.daemon_task_kind` instead of a hardcoded match.
#[cfg(feature = "daemon-client")]
pub(crate) fn operation_request_to_task_kind_json(request: &OperationRequest) -> PyResult<String> {
    crate::dispatch_helpers::operation_request_to_daemon_task(request)
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

    /// Construct an engine backed by the in-process Rust engine.
    ///
    /// This is equivalent to ``Engine(scope, mode=mode, ...)`` but uses the
    /// explicit constructor name for clarity in mixed local/daemon codebases.
    #[staticmethod]
    #[pyo3(signature = (scope, *, mode="manual", concurrency=100, timeout_ms=5000))]
    fn local(scope: Scope, mode: &str, concurrency: usize, timeout_ms: u64) -> PyResult<Self> {
        Self::new_inner(scope, mode, concurrency, timeout_ms)
    }

    /// Construct an engine backed by a daemon over a Unix socket.
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

    /// List all registered operation IDs.
    ///
    /// Returns:
    ///     list[str]: Stable operation identifiers available for dispatch.
    fn list_operations(&self) -> Vec<String> {
        self.state.registry.list()
    }

    /// Check if an operation ID is registered.
    ///
    /// Args:
    ///     operation_id: The operation identifier to check.
    ///
    /// Returns:
    ///     bool: True if the operation is registered.
    fn has_operation(&self, operation_id: &str) -> bool {
        self.state.registry.contains(operation_id)
    }

    /// Return structured policy decisions emitted by this engine instance.
    fn audit_events(&self) -> Vec<crate::engine_state::DispatchAuditEvent> {
        self.state.audit_events()
    }

    /// Dispatch a generic operation request to the appropriate engine function.
    ///
    /// Routes through the OperationExecutorRegistry, which checks feature gates
    /// and provides "Did you mean?" suggestions for unknown operations.
    /// Returns an OperationResult with status and artifacts.
    ///
    /// When the engine was constructed via `Engine.daemon(...)`, the request
    /// is submitted to the daemon session instead of executing locally.
    fn run(&self, py: Python<'_>, request: OperationRequest) -> PyResult<OperationResult> {
        #[cfg(feature = "daemon-client")]
        {
            if let Some(daemon) = &self.daemon_backend {
                return self.run_via_daemon(py, &request, daemon);
            }
        }
        Ok(self
            .state
            .registry
            .execute(py, &request.operation, &request, self))
    }

    /// Invoke a tool by tool ID with a validated payload dictionary.
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
    ///     OperationResult: The common result envelope.
    #[pyo3(signature = (tool_id, target, payload=None, timeout_ms=None))]
    fn invoke_tool(
        &self,
        py: Python<'_>,
        tool_id: &str,
        target: &str,
        payload: Option<&Bound<'_, PyDict>>,
        timeout_ms: Option<u64>,
    ) -> PyResult<OperationResult> {
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

    /// Invoke a tool using a typed ToolRequest object.
    ///
    /// Accepts a ToolRequest and delegates through the engine's enforcement
    /// pipeline, preserving scope, policy, audit, timeout, cancellation,
    /// and rate-limit behavior.
    ///
    /// Args:
    ///     request: A ToolRequest object with tool ID, target, params, and options.
    ///
    /// Returns:
    ///     OperationResult: The common result envelope.
    #[pyo3(signature = (request,))]
    fn invoke_tool_request(
        &self,
        py: Python<'_>,
        request: crate::tool_core::ToolRequestPy,
    ) -> PyResult<OperationResult> {
        let inner = request.into_inner();
        let metadata: HashMap<String, String> = if let Some(obj) = inner.params.as_object() {
            obj.iter()
                .map(|(k, v)| {
                    let val = match v {
                        serde_json::Value::String(s) => s.clone(),
                        other => other.to_string(),
                    };
                    (k.clone(), val)
                })
                .collect()
        } else {
            HashMap::new()
        };
        let op_request = OperationRequest::new(
            inner.tool,
            inner.target.value,
            inner.options.timeout_ms,
            Some(metadata),
        );
        self.run(py, op_request)
    }

    /// Run a port scan.
    ///
    /// Thin delegate: validates scope (raising EnforcementError for denial),
    /// then routes through the canonical engine dispatch path.
    #[pyo3(signature = (request,))]
    fn run_port_scan(&self, py: Python<'_>, request: PortScanRequest) -> PyResult<OperationResult> {
        let mut metadata = std::collections::HashMap::new();
        if let Some(ref ports) = request.ports {
            metadata.insert("ports".to_string(), ports.clone());
        }
        if let Some(ref mode) = request.mode {
            metadata.insert("mode".to_string(), mode.clone());
        }
        if let Some(ref timing) = request.timing {
            metadata.insert("timing".to_string(), timing.clone());
        }
        let op_request = OperationRequest::new(
            "scan_ports".to_string(),
            request.target,
            request.timeout_ms,
            Some(metadata),
        );
        self.run(py, op_request)
    }

    /// Run an endpoint scan.
    ///
    /// Thin delegate: validates scope, then routes through canonical dispatch.
    #[pyo3(signature = (request,))]
    fn run_endpoint_scan(
        &self,
        py: Python<'_>,
        request: EndpointScanRequest,
    ) -> PyResult<OperationResult> {
        let mut metadata = std::collections::HashMap::new();
        if let Some(ref paths) = request.paths {
            metadata.insert("endpoints".to_string(), paths.join(","));
        }
        let op_request = OperationRequest::new(
            "scan_endpoints".to_string(),
            request.target,
            request.timeout_ms,
            Some(metadata),
        );
        self.run(py, op_request)
    }

    /// Run service fingerprinting.
    ///
    /// Thin delegate: validates scope, then routes through canonical dispatch.
    #[pyo3(signature = (request,))]
    fn run_fingerprint(
        &self,
        py: Python<'_>,
        request: FingerprintRequest,
    ) -> PyResult<OperationResult> {
        let mut metadata = std::collections::HashMap::new();
        if let Some(ref ports) = request.ports {
            let ports_str: Vec<String> = ports.iter().map(|p| p.to_string()).collect();
            metadata.insert("ports".to_string(), ports_str.join(","));
        }
        let op_request = OperationRequest::new(
            "fingerprint_services".to_string(),
            request.target,
            request.timeout_ms,
            Some(metadata),
        );
        self.run(py, op_request)
    }

    /// Run DNS reconnaissance.
    ///
    /// Thin delegate: validates scope, then routes through canonical dispatch.
    #[pyo3(signature = (request,))]
    fn run_recon_dns(&self, py: Python<'_>, request: ReconDnsRequest) -> PyResult<OperationResult> {
        let op_request = OperationRequest::new(
            "recon_dns".to_string(),
            request.target,
            request.timeout_ms,
            None,
        );
        self.run(py, op_request)
    }

    /// Run TLS inspection.
    ///
    /// Thin delegate: validates scope, then routes through canonical dispatch.
    #[pyo3(signature = (request,))]
    fn run_tls_inspect(
        &self,
        py: Python<'_>,
        request: TlsInspectRequest,
    ) -> PyResult<OperationResult> {
        let op_request = OperationRequest::new(
            "inspect_tls".to_string(),
            request.target,
            request.timeout_ms,
            None,
        );
        self.run(py, op_request)
    }

    /// Run technology detection.
    ///
    /// Thin delegate: validates scope, then routes through canonical dispatch.
    #[pyo3(signature = (request,))]
    fn run_tech_detect(
        &self,
        py: Python<'_>,
        request: TechDetectRequest,
    ) -> PyResult<OperationResult> {
        let op_request = OperationRequest::new(
            "detect_technology".to_string(),
            request.target,
            request.timeout_ms,
            None,
        );
        self.run(py, op_request)
    }

    /// Run WAF detection.
    ///
    /// Thin delegate: validates scope, then routes through canonical dispatch.
    #[pyo3(signature = (request,))]
    fn run_waf_detect(
        &self,
        py: Python<'_>,
        request: WafDetectRequest,
    ) -> PyResult<OperationResult> {
        let op_request = OperationRequest::new(
            "detect_waf".to_string(),
            request.target,
            request.timeout_ms,
            None,
        );
        self.run(py, op_request)
    }

    /// Run an HTTP load test.
    ///
    /// Thin delegate: validates scope, then routes through canonical dispatch.
    #[pyo3(signature = (request,))]
    fn run_load_test(&self, py: Python<'_>, request: LoadTestRequest) -> PyResult<OperationResult> {
        let mut metadata = std::collections::HashMap::new();
        if let Some(reqs) = request.requests {
            metadata.insert("requests".to_string(), reqs.to_string());
        }
        if let Some(conc) = request.concurrency {
            metadata.insert("concurrency".to_string(), conc.to_string());
        }
        if let Some(ref method) = request.method {
            metadata.insert("method".to_string(), method.clone());
        }
        let op_request = OperationRequest::new(
            "load_test".to_string(),
            request.target,
            request.timeout_ms,
            Some(metadata),
        );
        self.run(py, op_request)
    }

    /// Run WAF validation.
    ///
    /// Thin delegate: validates scope, then routes through canonical dispatch.
    #[pyo3(signature = (request,))]
    fn run_waf_validate(
        &self,
        py: Python<'_>,
        request: WafValidateRequest,
    ) -> PyResult<OperationResult> {
        let op_request = OperationRequest::new(
            "validate_waf".to_string(),
            request.target,
            request.timeout_ms,
            None,
        );
        self.run(py, op_request)
    }

    /// Run HTTP fuzzing.
    ///
    /// Thin delegate: validates scope, then routes through canonical dispatch.
    #[pyo3(signature = (request,))]
    fn run_fuzz(&self, py: Python<'_>, request: FuzzRequest) -> PyResult<OperationResult> {
        let mut metadata = std::collections::HashMap::new();
        if let Some(ref pt) = request.payload_type {
            metadata.insert("payload_type".to_string(), pt.clone());
        }
        if let Some(threads) = request.threads {
            metadata.insert("threads".to_string(), threads.to_string());
        }
        let op_request = OperationRequest::new(
            "fuzz_http".to_string(),
            request.target,
            request.timeout_ms,
            Some(metadata),
        );
        self.run(py, op_request)
    }

    /// Create a scan plan suggesting what operations to run against a target.
    fn plan(&self, target: &str) -> PyResult<ScanPlan> {
        self.plan_inner(target)
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
            self.state.mode, self.state.concurrency
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

    /// Dispatch a generic operation request (used by pipeline and other internal callers).
    ///
    /// This method is called by `OperationExecutorRegistry::execute()` after feature
    /// gate validation. Operation IDs must match `StableOperation::ALL`.
    pub(crate) fn dispatch(
        &self,
        py: Python<'_>,
        request: OperationRequest,
        cancel_token: Option<CancellationToken>,
    ) -> OperationResult {
        use crate::operation_registry::StableOperation;
        let op = request.operation.clone();
        let target = request.target.clone();

        // Phase 1: Common lifecycle (planning, validation, preflight, cancel, deadline)
        let _deadline = match crate::dispatch_helpers::pre_dispatch_lifecycle(
            py,
            &op,
            &target,
            request.timeout_ms,
            self.state.timeout_ms,
            &self.state,
            &cancel_token,
        ) {
            Ok(dl) => dl,
            Err(result) => return result,
        };

        // Phase 2: Operation-specific dispatch
        let operation = match StableOperation::parse(&op) {
            Some(op) => op,
            None => return operation_err(format!("Unknown operation: {}", op)),
        };

        let result = self.execute_operation(py, operation, &request);

        // Phase 3: Post-dispatch hooks (finding events)
        self.post_dispatch_hooks(py, operation, &target, &result);

        result
    }

    /// Operation-specific dispatch: extract typed params and call the inner method.
    fn execute_operation(
        &self,
        py: Python<'_>,
        operation: crate::operation_registry::StableOperation,
        request: &OperationRequest,
    ) -> OperationResult {
        use crate::operation_registry::StableOperation;
        match operation {
            StableOperation::ScanPorts => {
                let ports_str = request
                    .metadata
                    .get("ports")
                    .cloned()
                    .unwrap_or_else(|| "1-1024".to_string());
                let effective_timeout = request.timeout_ms.unwrap_or(self.state.timeout_ms);
                let effective_concurrency = self.state.concurrency;
                match parse_ports_string(&ports_str) {
                    Ok(ports) => {
                        let req = PortScanRequest::new(
                            request.target.clone(),
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
            StableOperation::ScanEndpoints => {
                let endpoints: Vec<String> = request
                    .metadata
                    .get("endpoints")
                    .map(|s| s.split(',').map(|s| s.trim().to_string()).collect())
                    .unwrap_or_default();
                let effective_timeout = request.timeout_ms.unwrap_or(self.state.timeout_ms);
                let req = EndpointScanRequest::new(
                    request.target.clone(),
                    Some(endpoints),
                    None,
                    Some(effective_timeout),
                );
                self.run_endpoint_scan_inner(py, &req)
            }
            StableOperation::FingerprintServices => {
                let ports_str = request.metadata.get("ports").cloned().unwrap_or_default();
                let ports = if ports_str.is_empty() {
                    vec![80, 443]
                } else {
                    match parse_ports_string(&ports_str) {
                        Ok(p) => p,
                        Err(e) => return operation_err(e.to_string()),
                    }
                };
                let effective_timeout = request.timeout_ms.unwrap_or(self.state.timeout_ms);
                let req = FingerprintRequest::new(
                    request.target.clone(),
                    Some(ports.clone()),
                    Some(effective_timeout),
                );
                self.run_fingerprint_inner(py, &req, ports)
            }
            StableOperation::ReconDns => {
                let req = ReconDnsRequest::new(request.target.clone(), None, request.timeout_ms);
                self.run_recon_dns_inner(py, &req)
            }
            StableOperation::InspectTls => {
                let req = TlsInspectRequest::new(request.target.clone(), request.timeout_ms);
                self.run_tls_inspect_inner(py, &req)
            }
            StableOperation::DetectTechnology => {
                let req = TechDetectRequest::new(request.target.clone(), request.timeout_ms);
                self.run_tech_detect_inner(py, &req)
            }
            StableOperation::DetectWaf => {
                let req = WafDetectRequest::new(request.target.clone(), request.timeout_ms);
                self.run_waf_detect_inner(py, &req)
            }
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
                let req = LoadTestRequest::new(
                    request.target.clone(),
                    Some(total_requests as u32),
                    Some(concurrency as u32),
                    Some(method),
                    request.timeout_ms,
                );
                self.run_load_test_inner(py, &req)
            }
            StableOperation::ValidateWaf => {
                let req = WafValidateRequest::new(request.target.clone(), None, request.timeout_ms);
                self.run_waf_validate_inner(py, &req)
            }
            StableOperation::FuzzHttp => {
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
            #[cfg(feature = "git-secrets")]
            StableOperation::ScanGitSecrets => {
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
                self.run_git_secrets_inner(py, &repo_path, max_commits)
            }
            #[cfg(feature = "sbom")]
            StableOperation::GenerateSbom => {
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
                self.run_sbom_inner(py, &project_path, &ecosystem, &format)
            }
            StableOperation::RunConsolidatedRecon => {
                let config = crate::consolidated_recon::ConsolidatedReconConfigPy {
                    run_dns: request
                        .metadata
                        .get("run_dns")
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(true),
                    run_ssl: request
                        .metadata
                        .get("run_ssl")
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(true),
                    run_tech_detect: request
                        .metadata
                        .get("run_tech_detect")
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(true),
                    run_subdomain: request
                        .metadata
                        .get("run_subdomain")
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(true),
                    run_whois: request
                        .metadata
                        .get("run_whois")
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(true),
                    run_cors: request
                        .metadata
                        .get("run_cors")
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(true),
                    run_wayback: request
                        .metadata
                        .get("run_wayback")
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(true),
                    run_js_analysis: request
                        .metadata
                        .get("run_js_analysis")
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(true),
                    run_content: request
                        .metadata
                        .get("run_content")
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(true),
                    run_email: request
                        .metadata
                        .get("run_email")
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(true),
                    timeout_secs: request
                        .metadata
                        .get("timeout_secs")
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(30),
                    concurrency: request
                        .metadata
                        .get("concurrency")
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(10),
                };
                self.run_consolidated_recon_inner(py, &request.target, config)
            }
            StableOperation::GraphqlTest => {
                let config = crate::graphql::GraphQLTestConfigPy {
                    endpoint: request.target.clone(),
                    enable_introspection: request
                        .metadata
                        .get("enable_introspection")
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(true),
                    enable_depth_bypass: request
                        .metadata
                        .get("enable_depth_bypass")
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(true),
                    enable_alias_overload: request
                        .metadata
                        .get("enable_alias_overload")
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(true),
                    timeout_secs: request
                        .metadata
                        .get("timeout_secs")
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(10),
                };
                self.run_graphql_inner(py, config)
            }
            StableOperation::OauthTest => {
                let config = crate::oauth::OAuthTestConfigPy {
                    client_id: request
                        .metadata
                        .get("client_id")
                        .cloned()
                        .unwrap_or_default(),
                    redirect_uri: request
                        .metadata
                        .get("redirect_uri")
                        .cloned()
                        .unwrap_or_default(),
                    client_secret: request.metadata.get("client_secret").cloned(),
                    issuer_url: request.metadata.get("issuer_url").cloned(),
                    enable_redirect_test: request
                        .metadata
                        .get("enable_redirect_test")
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(true),
                    enable_scope_test: request
                        .metadata
                        .get("enable_scope_test")
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(true),
                    enable_state_test: request
                        .metadata
                        .get("enable_state_test")
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(true),
                    enable_grant_test: request
                        .metadata
                        .get("enable_grant_test")
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(true),
                    timeout_secs: request
                        .metadata
                        .get("timeout_secs")
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(10),
                };
                let auth_endpoint = request
                    .metadata
                    .get("auth_endpoint")
                    .cloned()
                    .unwrap_or_else(|| request.target.clone());
                self.run_oauth_inner(py, config, &auth_endpoint)
            }
            StableOperation::AuthTest => self.run_auth_test_inner(py, &request.target),
            #[cfg(feature = "db-pentest")]
            StableOperation::DbProbe => {
                let db_type = request
                    .metadata
                    .get("db_type")
                    .cloned()
                    .unwrap_or_else(|| "all".to_string());
                let user = request.metadata.get("username").cloned();
                let password = request.metadata.get("password").cloned();
                let database = request.metadata.get("database").cloned();
                let port: Option<u16> = request.metadata.get("port").and_then(|s| s.parse().ok());
                self.run_db_probe_inner(
                    py,
                    &request.target,
                    &db_type,
                    user.as_deref(),
                    password.as_deref(),
                    database.as_deref(),
                    port,
                )
            }
            #[cfg(feature = "nse")]
            StableOperation::NseRun => {
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
                self.run_nse_inner(py, &request.target, &script_name, script_args.as_deref())
            }
            #[cfg(feature = "container")]
            StableOperation::ScanDockerImage => {
                let image = request
                    .metadata
                    .get("image")
                    .cloned()
                    .unwrap_or_else(|| request.target.clone());
                self.run_docker_image_inner(py, &image)
            }
            #[cfg(feature = "container")]
            StableOperation::ScanKubernetes => {
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
                self.run_kubernetes_inner(py, &api_server, token.as_deref(), timeout_secs)
            }
            #[cfg(feature = "mobile")]
            StableOperation::AnalyzeApk => {
                let apk_path = request
                    .metadata
                    .get("apk_path")
                    .cloned()
                    .unwrap_or_else(|| request.target.clone());
                self.run_apk_inner(py, &apk_path)
            }
            #[cfg(feature = "mobile")]
            StableOperation::AnalyzeIpa => {
                let ipa_path = request
                    .metadata
                    .get("ipa_path")
                    .cloned()
                    .unwrap_or_else(|| request.target.clone());
                self.run_ipa_inner(py, &ipa_path)
            }
            _ => operation_err(format!(
                "Operation '{}' is not available in this build configuration",
                request.operation
            )),
        }
    }

    /// Emit finding and artifact events after a successful operation dispatch.
    fn post_dispatch_hooks(
        &self,
        py: Python<'_>,
        operation: crate::operation_registry::StableOperation,
        target: &str,
        result: &OperationResult,
    ) {
        if !result.is_success() {
            return;
        }
        use crate::event_protocol::{ArtifactEvent, EventEnvelope};
        use crate::operation_registry::StableOperation;

        if let Some(ref payload) = result.payload {
            match operation {
                StableOperation::ScanPorts => {
                    if let OperationPayload::PortScan(ref ps) = payload {
                        if !ps.open_ports.is_empty() {
                            emit_finding_event(
                                &self.state,
                                format!("port-scan-{}", target),
                                "info".to_string(),
                                format!("{} open port(s) found on {}", ps.open_ports.len(), target),
                                true,
                                target.to_string(),
                            );
                        }
                    }
                }
                StableOperation::ScanEndpoints => {
                    if let OperationPayload::EndpointScan(ref es) = payload {
                        if es.endpoints_found > 0 {
                            emit_finding_event(
                                &self.state,
                                format!("endpoint-scan-{}", target),
                                "info".to_string(),
                                format!("{} endpoint(s) found on {}", es.endpoints_found, target),
                                true,
                                target.to_string(),
                            );
                        }
                    }
                }
                StableOperation::FingerprintServices => {
                    if let OperationPayload::Fingerprint(ref fp) = payload {
                        if fp.services_identified > 0 {
                            emit_finding_event(
                                &self.state,
                                format!("fingerprint-{}", target),
                                "info".to_string(),
                                format!(
                                    "{} service(s) identified on {}",
                                    fp.services_identified, target
                                ),
                                true,
                                target.to_string(),
                            );
                        }
                    }
                }
                StableOperation::InspectTls => {
                    if let OperationPayload::TlsInspection(ref tls) = payload {
                        if !tls.issues.is_empty() {
                            emit_finding_event(
                                &self.state,
                                format!("tls-inspect-{}", target),
                                "warning".to_string(),
                                format!("{} TLS issue(s) found on {}", tls.issues.len(), target),
                                true,
                                target.to_string(),
                            );
                        }
                    }
                }
                StableOperation::FuzzHttp => {
                    if let OperationPayload::HttpFuzz(ref fuzz) = payload {
                        let issues =
                            fuzz.waf_bypasses + fuzz.potential_leaks + fuzz.redos_suspected;
                        if issues > 0 {
                            emit_finding_event(
                                &self.state,
                                format!("fuzz-{}", target),
                                "high".to_string(),
                                format!("{} fuzzing issue(s) found on {}", issues, target),
                                false,
                                target.to_string(),
                            );
                        }
                    }
                }
                #[cfg(feature = "git-secrets")]
                StableOperation::ScanGitSecrets => {
                    if let OperationPayload::GitSecrets(ref gs) = payload {
                        if !gs.findings.is_empty() {
                            emit_finding_event(
                                &self.state,
                                format!("git-secrets-{}", target),
                                "critical".to_string(),
                                format!("{} secret(s) found in {}", gs.findings.len(), target),
                                false,
                                target.to_string(),
                            );
                        }
                    }
                }
                #[cfg(feature = "sbom")]
                StableOperation::GenerateSbom => {
                    let artifact = ArtifactEvent::new(
                        format!("sbom-{}", target),
                        "sbom".to_string(),
                        "application/json".to_string(),
                        0,
                    );
                    self.state.emit_event(EventEnvelope::create(
                        "operation.artifact".to_string(),
                        artifact.into_py(py),
                        None,
                        None,
                        Some(target.to_string()),
                        None,
                    ));
                }
                StableOperation::RunConsolidatedRecon => {
                    let artifact = ArtifactEvent::new(
                        format!("recon-{}", target),
                        "consolidated_recon".to_string(),
                        "application/json".to_string(),
                        0,
                    );
                    self.state.emit_event(EventEnvelope::create(
                        "operation.artifact".to_string(),
                        artifact.into_py(py),
                        None,
                        None,
                        Some(target.to_string()),
                        None,
                    ));
                }
                _ => {}
            }
        }
    }

    /// Dispatch an OperationRequest through the daemon backend.
    ///
    /// Creates a session on first use if none was provided at construction time.
    /// Converts the request to a `TaskKind` JSON payload and submits it to the
    /// daemon, returning an `OperationResult` with the daemon response.
    #[cfg(feature = "daemon-client")]
    fn run_via_daemon(
        &self,
        py: Python<'_>,
        request: &OperationRequest,
        daemon: &DaemonBackend,
    ) -> PyResult<OperationResult> {
        // Ensure we have a session ID
        let session_id = match &daemon.session_id {
            Some(sid) => sid.clone(),
            None => {
                // Create a session on first use
                let client = daemon.client.clone();
                let response: crate::daemon::DaemonResponsePy =
                    runtime_sync::block_on(py, async move {
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
                // Extract session_id from response message: "session_id=<uuid>"
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
        let task_kind_json = operation_request_to_task_kind_json(request)?;

        // Submit via daemon
        let client = daemon.client.clone();
        let sid = session_id.clone();
        let op = request.operation.clone();
        let response: crate::daemon::DaemonResponsePy = runtime_sync::block_on(py, async move {
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
            Ok::<_, PyErr>(crate::daemon::server_message_to_response(msg))
        })?;

        Ok(daemon_response_to_operation_result(&response, &op))
    }

    fn run_port_scan_inner(
        &self,
        py: Python<'_>,
        request: &PortScanRequest,
        ports: Vec<u16>,
        effective_concurrency: usize,
        effective_timeout_ms: u64,
    ) -> OperationResult {
        if let Err(e) = self.state.scope.enforce_target(&request.target) {
            return operation_err(e.to_string());
        }
        for &port in &ports {
            if let Err(e) = self.state.scope.enforce_port(port) {
                return operation_err(e.to_string());
            }
        }

        // Emit: operation started
        self.state
            .emit_event(crate::event_protocol::EventEnvelope::create(
                "operation.started".to_string(),
                crate::event_protocol::ProgressEvent::new(
                    0.0,
                    format!("Starting port scan on {}", request.target),
                    0,
                    ports.len(),
                )
                .into_py(py),
                None,
                None,
                None,
                None,
            ));

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
                let py_result = PortScanResult::from_engine(r);
                let items = py_result.scanned_ports as u64;
                let open = py_result.open_ports.len() as u64;
                let mut metadata = std::collections::HashMap::new();
                metadata.insert("target".to_string(), request.target.clone());
                let stats = ExecutionStats::new(py_result.elapsed_ms, items, items - open, 0);

                // Emit: operation completed
                self.state
                    .emit_event(crate::event_protocol::EventEnvelope::create(
                        "operation.completed".to_string(),
                        crate::event_protocol::CompletionEvent::new(
                            py,
                            "Completed".to_string(),
                            None,
                            py_result.elapsed_ms,
                        )
                        .into_py(py),
                        None,
                        None,
                        None,
                        None,
                    ));

                operation_ok(
                    stats,
                    Some(metadata),
                    Some(OperationPayload::PortScan(py_result)),
                )
            }
            Err(e) => {
                // Emit: operation failed
                self.state
                    .emit_event(crate::event_protocol::EventEnvelope::create(
                        "operation.failed".to_string(),
                        crate::event_protocol::FailureEvent::new(
                            "scan_error".to_string(),
                            e.to_string(),
                            false,
                        )
                        .into_py(py),
                        None,
                        None,
                        None,
                        None,
                    ));
                operation_err(e.to_string())
            }
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
        if let Err(e) = self.state.scope.enforce_target(&host) {
            return operation_err(e.to_string());
        }

        // Emit: operation started
        self.state
            .emit_event(crate::event_protocol::EventEnvelope::create(
                "operation.started".to_string(),
                crate::event_protocol::ProgressEvent::new(
                    0.0,
                    format!("Starting endpoint scan on {}", request.target),
                    0,
                    0,
                )
                .into_py(py),
                None,
                None,
                None,
                None,
            ));

        let effective_timeout = request.timeout_ms.unwrap_or(self.state.timeout_ms);
        let endpoints = request.paths.clone().unwrap_or_default();

        let config = eggsec::scanner::EndpointScanConfig {
            base_url: request.target.clone(),
            endpoints,
            concurrency: self.state.concurrency,
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
                let py_result = EndpointScanResult::from_engine(r);
                let items = py_result.endpoints_found as u64;
                let mut metadata = std::collections::HashMap::new();
                metadata.insert("target".to_string(), request.target.clone());
                let stats = ExecutionStats::new(py_result.elapsed_ms, items, 0, 0);

                // Emit: operation completed
                self.state
                    .emit_event(crate::event_protocol::EventEnvelope::create(
                        "operation.completed".to_string(),
                        crate::event_protocol::CompletionEvent::new(
                            py,
                            "Completed".to_string(),
                            None,
                            py_result.elapsed_ms,
                        )
                        .into_py(py),
                        None,
                        None,
                        None,
                        None,
                    ));

                operation_ok(
                    stats,
                    Some(metadata),
                    Some(OperationPayload::EndpointScan(py_result)),
                )
            }
            Err(e) => {
                self.state
                    .emit_event(crate::event_protocol::EventEnvelope::create(
                        "operation.failed".to_string(),
                        crate::event_protocol::FailureEvent::new(
                            "scan_error".to_string(),
                            e.to_string(),
                            false,
                        )
                        .into_py(py),
                        None,
                        None,
                        None,
                        None,
                    ));
                operation_err(e.to_string())
            }
        }
    }

    fn run_fingerprint_inner(
        &self,
        py: Python<'_>,
        request: &FingerprintRequest,
        ports: Vec<u16>,
    ) -> OperationResult {
        if let Err(e) = self.state.scope.enforce_target(&request.target) {
            return operation_err(e.to_string());
        }
        for &port in &ports {
            if let Err(e) = self.state.scope.enforce_port(port) {
                return operation_err(e.to_string());
            }
        }

        // Emit: operation started
        self.state
            .emit_event(crate::event_protocol::EventEnvelope::create(
                "operation.started".to_string(),
                crate::event_protocol::ProgressEvent::new(
                    0.0,
                    format!("Starting fingerprint scan on {}", request.target),
                    0,
                    ports.len(),
                )
                .into_py(py),
                None,
                None,
                None,
                None,
            ));

        let effective_timeout = request.timeout_ms.unwrap_or(self.state.timeout_ms);
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
                let py_result = FingerprintScanResult::from_engine(r);
                let items = py_result.services_identified as u64;
                let mut metadata = std::collections::HashMap::new();
                metadata.insert("target".to_string(), request.target.clone());
                let stats = ExecutionStats::new(py_result.elapsed_ms, items, 0, 0);

                self.state
                    .emit_event(crate::event_protocol::EventEnvelope::create(
                        "operation.completed".to_string(),
                        crate::event_protocol::CompletionEvent::new(
                            py,
                            "Completed".to_string(),
                            None,
                            py_result.elapsed_ms,
                        )
                        .into_py(py),
                        None,
                        None,
                        None,
                        None,
                    ));

                operation_ok(
                    stats,
                    Some(metadata),
                    Some(OperationPayload::Fingerprint(py_result)),
                )
            }
            Err(e) => {
                self.state
                    .emit_event(crate::event_protocol::EventEnvelope::create(
                        "operation.failed".to_string(),
                        crate::event_protocol::FailureEvent::new(
                            "fingerprint_error".to_string(),
                            e.to_string(),
                            false,
                        )
                        .into_py(py),
                        None,
                        None,
                        None,
                        None,
                    ));
                operation_err(e.to_string())
            }
        }
    }

    fn run_recon_dns_inner(&self, py: Python<'_>, request: &ReconDnsRequest) -> OperationResult {
        if let Err(e) = self.state.scope.enforce_target(&request.target) {
            return operation_err(e.to_string());
        }

        // Emit: operation started
        self.state
            .emit_event(crate::event_protocol::EventEnvelope::create(
                "operation.started".to_string(),
                crate::event_protocol::ProgressEvent::new(
                    0.0,
                    format!("Starting DNS recon on {}", request.target),
                    0,
                    0,
                )
                .into_py(py),
                None,
                None,
                None,
                None,
            ));

        let domain_owned = request.target.clone();
        let result = runtime_sync::block_on(py, async move {
            eggsec::recon::dns_records::enumerate_dns_records(&domain_owned)
                .await
                .map_pyerr()
        });

        match result {
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
                    + py_result.caa_records.len()) as u64;
                let mut metadata = std::collections::HashMap::new();
                metadata.insert("target".to_string(), request.target.clone());
                let stats = ExecutionStats::new(0, record_count, 0, 0);

                self.state
                    .emit_event(crate::event_protocol::EventEnvelope::create(
                        "operation.completed".to_string(),
                        crate::event_protocol::CompletionEvent::new(
                            py,
                            "Completed".to_string(),
                            None,
                            0,
                        )
                        .into_py(py),
                        None,
                        None,
                        None,
                        None,
                    ));

                operation_ok(
                    stats,
                    Some(metadata),
                    Some(OperationPayload::DnsRecon(py_result)),
                )
            }
            Err(e) => {
                self.state
                    .emit_event(crate::event_protocol::EventEnvelope::create(
                        "operation.failed".to_string(),
                        crate::event_protocol::FailureEvent::new(
                            "dns_error".to_string(),
                            e.to_string(),
                            false,
                        )
                        .into_py(py),
                        None,
                        None,
                        None,
                        None,
                    ));
                operation_err(e.to_string())
            }
        }
    }

    fn run_tls_inspect_inner(
        &self,
        py: Python<'_>,
        request: &TlsInspectRequest,
    ) -> OperationResult {
        if let Err(e) = self.state.scope.enforce_target(&request.target) {
            return operation_err(e.to_string());
        }

        // Emit: operation started
        self.state
            .emit_event(crate::event_protocol::EventEnvelope::create(
                "operation.started".to_string(),
                crate::event_protocol::ProgressEvent::new(
                    0.0,
                    format!("Starting TLS inspection on {}", request.target),
                    0,
                    0,
                )
                .into_py(py),
                None,
                None,
                None,
                None,
            ));

        let host_owned = request.target.clone();
        let result = runtime_sync::block_on(py, async move {
            eggsec::recon::ssl::analyze_ssl(&host_owned, 443)
                .await
                .map_pyerr()
        });

        match result {
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
                metadata.insert("target".to_string(), request.target.clone());
                let stats = ExecutionStats::new(0, 1, issue_count, 0);

                self.state
                    .emit_event(crate::event_protocol::EventEnvelope::create(
                        "operation.completed".to_string(),
                        crate::event_protocol::CompletionEvent::new(
                            py,
                            "Completed".to_string(),
                            None,
                            0,
                        )
                        .into_py(py),
                        None,
                        None,
                        None,
                        None,
                    ));

                operation_ok(
                    stats,
                    Some(metadata),
                    Some(OperationPayload::TlsInspection(py_result)),
                )
            }
            Err(e) => {
                self.state
                    .emit_event(crate::event_protocol::EventEnvelope::create(
                        "operation.failed".to_string(),
                        crate::event_protocol::FailureEvent::new(
                            "tls_error".to_string(),
                            e.to_string(),
                            false,
                        )
                        .into_py(py),
                        None,
                        None,
                        None,
                        None,
                    ));
                operation_err(e.to_string())
            }
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
        if let Err(e) = self.state.scope.enforce_target(&host) {
            return operation_err(e.to_string());
        }

        // Emit: operation started
        self.state
            .emit_event(crate::event_protocol::EventEnvelope::create(
                "operation.started".to_string(),
                crate::event_protocol::ProgressEvent::new(
                    0.0,
                    format!("Starting technology detection on {}", request.target),
                    0,
                    0,
                )
                .into_py(py),
                None,
                None,
                None,
                None,
            ));

        let url_owned = request.target.clone();
        let result = runtime_sync::block_on(py, async move {
            eggsec::recon::techdetect::detect_tech_stack(&url_owned)
                .await
                .map_pyerr()
        });

        match result {
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
                    + py_result.tech_stack.other.len()) as u64;
                let mut metadata = std::collections::HashMap::new();
                metadata.insert("target".to_string(), request.target.clone());
                let stats = ExecutionStats::new(0, tech_count, 0, 0);

                self.state
                    .emit_event(crate::event_protocol::EventEnvelope::create(
                        "operation.completed".to_string(),
                        crate::event_protocol::CompletionEvent::new(
                            py,
                            "Completed".to_string(),
                            None,
                            0,
                        )
                        .into_py(py),
                        None,
                        None,
                        None,
                        None,
                    ));

                operation_ok(
                    stats,
                    Some(metadata),
                    Some(OperationPayload::TechnologyDetection(py_result)),
                )
            }
            Err(e) => {
                self.state
                    .emit_event(crate::event_protocol::EventEnvelope::create(
                        "operation.failed".to_string(),
                        crate::event_protocol::FailureEvent::new(
                            "tech_detect_error".to_string(),
                            e.to_string(),
                            false,
                        )
                        .into_py(py),
                        None,
                        None,
                        None,
                        None,
                    ));
                operation_err(e.to_string())
            }
        }
    }

    fn run_waf_detect_inner(&self, py: Python<'_>, request: &WafDetectRequest) -> OperationResult {
        let host = match extract_host_from_url(&request.target) {
            Ok(h) => h,
            Err(e) => return operation_err(e.to_string()),
        };
        if let Err(e) = self.state.scope.enforce_target(&host) {
            return operation_err(e.to_string());
        }

        // Emit: operation started
        self.state
            .emit_event(crate::event_protocol::EventEnvelope::create(
                "operation.started".to_string(),
                crate::event_protocol::ProgressEvent::new(
                    0.0,
                    format!("Starting WAF detection on {}", request.target),
                    0,
                    0,
                )
                .into_py(py),
                None,
                None,
                None,
                None,
            ));

        let url_owned = request.target.clone();
        let url_clone = url_owned.clone();
        let result = runtime_sync::block_on(py, async move {
            let detector = eggsec::waf::WafDetector::new().map_pyerr()?;
            detector.detect(&url_clone).await.map_pyerr()
        });

        match result {
            Ok(r) => {
                let py_result = WafDetectionResultPy {
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
                let items = if py_result.detected { 1 } else { 0 };
                let mut metadata = std::collections::HashMap::new();
                metadata.insert("target".to_string(), request.target.clone());
                let stats = ExecutionStats::new(0, items, 0, 0);

                self.state
                    .emit_event(crate::event_protocol::EventEnvelope::create(
                        "operation.completed".to_string(),
                        crate::event_protocol::CompletionEvent::new(
                            py,
                            "Completed".to_string(),
                            None,
                            0,
                        )
                        .into_py(py),
                        None,
                        None,
                        None,
                        None,
                    ));

                operation_ok(
                    stats,
                    Some(metadata),
                    Some(OperationPayload::WafDetection(py_result)),
                )
            }
            Err(e) => {
                self.state
                    .emit_event(crate::event_protocol::EventEnvelope::create(
                        "operation.failed".to_string(),
                        crate::event_protocol::FailureEvent::new(
                            "waf_detect_error".to_string(),
                            e.to_string(),
                            false,
                        )
                        .into_py(py),
                        None,
                        None,
                        None,
                        None,
                    ));
                operation_err(e.to_string())
            }
        }
    }

    fn run_load_test_inner(&self, py: Python<'_>, request: &LoadTestRequest) -> OperationResult {
        let host = match extract_host_from_url(&request.target) {
            Ok(h) => h,
            Err(e) => return operation_err(e.to_string()),
        };
        if let Err(e) = self.state.scope.enforce_target(&host) {
            return operation_err(e.to_string());
        }

        let total_requests = request.requests.unwrap_or(100) as u64;
        let concurrency = request.concurrency.unwrap_or(self.state.concurrency as u32) as usize;
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

        // Emit: operation started
        self.state
            .emit_event(crate::event_protocol::EventEnvelope::create(
                "operation.started".to_string(),
                crate::event_protocol::ProgressEvent::new(
                    0.0,
                    format!("Starting load test on {}", request.target),
                    0,
                    total_requests as usize,
                )
                .into_py(py),
                None,
                None,
                None,
                None,
            ));

        let scope_clone = self.state.scope.clone();

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
            Ok(r) => {
                let mut metadata = std::collections::HashMap::new();
                metadata.insert("target".to_string(), request.target.clone());
                let stats = ExecutionStats::new(
                    r.total_duration_ms,
                    r.total_requests,
                    r.failed_requests,
                    0,
                );

                self.state
                    .emit_event(crate::event_protocol::EventEnvelope::create(
                        "operation.completed".to_string(),
                        crate::event_protocol::CompletionEvent::new(
                            py,
                            "Completed".to_string(),
                            None,
                            r.total_duration_ms,
                        )
                        .into_py(py),
                        None,
                        None,
                        None,
                        None,
                    ));

                operation_ok(stats, Some(metadata), Some(OperationPayload::LoadTest(r)))
            }
            Err(e) => {
                self.state
                    .emit_event(crate::event_protocol::EventEnvelope::create(
                        "operation.failed".to_string(),
                        crate::event_protocol::FailureEvent::new(
                            "load_test_error".to_string(),
                            e.to_string(),
                            false,
                        )
                        .into_py(py),
                        None,
                        None,
                        None,
                        None,
                    ));
                operation_err(e.to_string())
            }
        }
    }

    fn run_waf_validate_inner(
        &self,
        py: Python<'_>,
        request: &WafValidateRequest,
    ) -> OperationResult {
        let host = match extract_host_from_url(&request.target) {
            Ok(h) => h,
            Err(e) => return operation_err(e.to_string()),
        };
        if let Err(e) = self.state.scope.enforce_target(&host) {
            return operation_err(e.to_string());
        }

        // Emit: operation started
        self.state
            .emit_event(crate::event_protocol::EventEnvelope::create(
                "operation.started".to_string(),
                crate::event_protocol::ProgressEvent::new(
                    0.0,
                    format!("Starting WAF validation on {}", request.target),
                    0,
                    0,
                )
                .into_py(py),
                None,
                None,
                None,
                None,
            ));

        let scope_clone = self.state.scope.clone();
        let result = crate::waf_validation::validate_waf(&request.target, scope_clone, false, None);

        match result {
            Ok(r) => {
                let mut metadata = std::collections::HashMap::new();
                metadata.insert("target".to_string(), request.target.clone());
                let stats = ExecutionStats::new(
                    r.duration_ms,
                    r.bypasses_tested as u64,
                    r.bypasses_successful as u64,
                    0,
                );

                self.state
                    .emit_event(crate::event_protocol::EventEnvelope::create(
                        "operation.completed".to_string(),
                        crate::event_protocol::CompletionEvent::new(
                            py,
                            "Completed".to_string(),
                            None,
                            r.duration_ms,
                        )
                        .into_py(py),
                        None,
                        None,
                        None,
                        None,
                    ));

                operation_ok(
                    stats,
                    Some(metadata),
                    Some(OperationPayload::WafValidation(r)),
                )
            }
            Err(e) => {
                self.state
                    .emit_event(crate::event_protocol::EventEnvelope::create(
                        "operation.failed".to_string(),
                        crate::event_protocol::FailureEvent::new(
                            "waf_validation_error".to_string(),
                            e.to_string(),
                            false,
                        )
                        .into_py(py),
                        None,
                        None,
                        None,
                        None,
                    ));
                operation_err(e.to_string())
            }
        }
    }

    fn run_fuzz_inner(&self, py: Python<'_>, request: &FuzzRequest) -> OperationResult {
        let host = match extract_host_from_url(&request.target) {
            Ok(h) => h,
            Err(e) => return operation_err(e.to_string()),
        };
        if let Err(e) = self.state.scope.enforce_target(&host) {
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

        // Emit: operation started
        self.state
            .emit_event(crate::event_protocol::EventEnvelope::create(
                "operation.started".to_string(),
                crate::event_protocol::ProgressEvent::new(
                    0.0,
                    format!("Starting HTTP fuzz on {}", request.target),
                    0,
                    0,
                )
                .into_py(py),
                None,
                None,
                None,
                None,
            ));

        let scope_clone = self.state.scope.clone();
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
            Ok(r) => {
                let mut metadata = std::collections::HashMap::new();
                metadata.insert("target".to_string(), request.target.clone());
                let stats = ExecutionStats::new(
                    r.duration_ms,
                    r.total_payloads as u64,
                    r.failed_requests as u64,
                    0,
                );

                self.state
                    .emit_event(crate::event_protocol::EventEnvelope::create(
                        "operation.completed".to_string(),
                        crate::event_protocol::CompletionEvent::new(
                            py,
                            "Completed".to_string(),
                            None,
                            r.duration_ms,
                        )
                        .into_py(py),
                        None,
                        None,
                        None,
                        None,
                    ));

                operation_ok(stats, Some(metadata), Some(OperationPayload::HttpFuzz(r)))
            }
            Err(e) => {
                self.state
                    .emit_event(crate::event_protocol::EventEnvelope::create(
                        "operation.failed".to_string(),
                        crate::event_protocol::FailureEvent::new(
                            "fuzz_error".to_string(),
                            e.to_string(),
                            false,
                        )
                        .into_py(py),
                        None,
                        None,
                        None,
                        None,
                    ));
                operation_err(e.to_string())
            }
        }
    }

    #[cfg(feature = "git-secrets")]
    fn run_git_secrets_inner(
        &self,
        py: Python<'_>,
        repo_path: &str,
        max_commits: usize,
    ) -> OperationResult {
        if let Err(e) = self
            .state
            .pre_dispatch_validate("scan_git_secrets", repo_path)
        {
            return operation_err(e.to_string());
        }

        let repo_path_owned = repo_path.to_string();
        let result = runtime_sync::block_on(py, async move {
            eggsec::recon::git_secrets::scan_git_secrets(&repo_path_owned, max_commits).map_pyerr()
        });

        match result {
            Ok(r) => {
                let py_result = crate::git_secrets::GitSecretsReportPy {
                    repo_path: r.repo_path,
                    commits_scanned: r.commits_scanned,
                    files_scanned: r.files_scanned,
                    findings: r
                        .findings
                        .into_iter()
                        .map(crate::git_secrets::GitSecretFindingPy::from_engine)
                        .collect(),
                    summary: crate::git_secrets::GitSecretsSummaryPy::from_engine(r.summary),
                };
                let mut metadata = std::collections::HashMap::new();
                metadata.insert("repo_path".to_string(), repo_path.to_string());
                let stats = ExecutionStats::new(0, py_result.findings.len() as u64, 0, 0);
                operation_ok(
                    stats,
                    Some(metadata),
                    Some(OperationPayload::GitSecrets(py_result)),
                )
            }
            Err(e) => operation_err(e.to_string()),
        }
    }

    #[cfg(feature = "sbom")]
    fn run_sbom_inner(
        &self,
        py: Python<'_>,
        project_path: &str,
        ecosystem: &str,
        format: &str,
    ) -> OperationResult {
        if let Err(e) = self
            .state
            .pre_dispatch_validate("generate_sbom", project_path)
        {
            return operation_err(e.to_string());
        }

        let sbom_format = crate::sbom::SbomFormatPy::from_str(format)
            .unwrap_or(crate::sbom::SbomFormatPy::CycloneDx);
        let project_path_owned = project_path.to_string();
        let ecosystem_owned = ecosystem.to_string();
        let engine_format = sbom_format.to_engine();

        let result = runtime_sync::block_on(py, async move {
            let gen = eggsec::supply_chain::sbom::SbomGenerator::new();
            let r = match ecosystem_owned.as_str() {
                "cargo" => gen.generate_from_cargo(&project_path_owned, engine_format),
                "npm" => gen.generate_from_npm(&project_path_owned, engine_format),
                "pip" => gen.generate_from_requirements(&project_path_owned, engine_format),
                other => return Err(anyhow::anyhow!("Unsupported ecosystem: '{}'", other)),
            };
            r.map_err(|e| e.into())
        });

        match result {
            Ok(r) => {
                let py_result = crate::sbom::SbomReportPy {
                    format: crate::sbom::SbomFormatPy::from_engine(r.format),
                    project_name: r.project_name,
                    version: r.version,
                    generated_at: r.generated_at,
                    components: r
                        .components
                        .into_iter()
                        .map(crate::sbom::SbomComponentPy::from_engine)
                        .collect(),
                    vulnerabilities: r
                        .vulnerabilities
                        .into_iter()
                        .map(crate::sbom::SbomVulnerabilityPy::from_engine)
                        .collect(),
                };
                let mut metadata = std::collections::HashMap::new();
                metadata.insert("project_path".to_string(), project_path.to_string());
                let stats = ExecutionStats::new(0, py_result.components.len() as u64, 0, 0);
                operation_ok(
                    stats,
                    Some(metadata),
                    Some(OperationPayload::Sbom(py_result)),
                )
            }
            Err(e) => operation_err(e.to_string()),
        }
    }

    fn run_consolidated_recon_inner(
        &self,
        py: Python<'_>,
        target: &str,
        config: crate::consolidated_recon::ConsolidatedReconConfigPy,
    ) -> OperationResult {
        if let Err(e) = self
            .state
            .pre_dispatch_validate("run_consolidated_recon", target)
        {
            return operation_err(e.to_string());
        }

        let target_owned = target.to_string();
        let result = runtime_sync::block_on(py, async move {
            let mut modules = Vec::new();
            if config.run_dns {
                let module_result =
                    match eggsec::recon::dns_records::enumerate_dns_records(&target_owned).await {
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
                let module_result = match eggsec::recon::ssl::analyze_ssl(&target_owned, 443).await
                {
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
                    match eggsec::recon::techdetect::detect_tech_stack(&target_owned).await {
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
            Ok::<_, anyhow::Error>(crate::consolidated_recon::ConsolidatedReconReportPy {
                target: target_owned,
                modules_run: modules.len(),
                modules_succeeded,
                modules_failed,
                modules,
            })
        });

        match result {
            Ok(r) => {
                let mut metadata = std::collections::HashMap::new();
                metadata.insert("target".to_string(), target.to_string());
                let stats =
                    ExecutionStats::new(0, r.modules_run as u64, r.modules_failed as u64, 0);
                operation_ok(
                    stats,
                    Some(metadata),
                    Some(OperationPayload::ConsolidatedRecon(r)),
                )
            }
            Err(e) => operation_err(e.to_string()),
        }
    }

    fn run_graphql_inner(
        &self,
        py: Python<'_>,
        config: crate::graphql::GraphQLTestConfigPy,
    ) -> OperationResult {
        if let Err(e) = self
            .state
            .pre_dispatch_validate("graphql_test", &config.endpoint)
        {
            return operation_err(e.to_string());
        }

        let endpoint = config.endpoint.clone();
        let result = runtime_sync::block_on(py, async move {
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
            Ok::<_, anyhow::Error>(results)
        });

        match result {
            Ok(r) => {
                let py_results: Vec<_> = r
                    .into_iter()
                    .map(crate::graphql::GraphQLTestResultPy::from_engine)
                    .collect();
                let mut metadata = std::collections::HashMap::new();
                metadata.insert("target".to_string(), endpoint);
                let stats = ExecutionStats::new(0, py_results.len() as u64, 0, 0);
                operation_ok(
                    stats,
                    Some(metadata),
                    Some(OperationPayload::Graphql(py_results)),
                )
            }
            Err(e) => operation_err(e.to_string()),
        }
    }

    fn run_oauth_inner(
        &self,
        py: Python<'_>,
        config: crate::oauth::OAuthTestConfigPy,
        auth_endpoint: &str,
    ) -> OperationResult {
        if let Err(e) = self
            .state
            .pre_dispatch_validate("oauth_test", &config.client_id)
        {
            return operation_err(e.to_string());
        }

        let auth_endpoint_owned = auth_endpoint.to_string();
        let result = runtime_sync::block_on(py, async move {
            let fuzzer = crate::oauth::build_oauth_fuzzer(&config)?;
            let mut results = Vec::new();
            if config.enable_redirect_test {
                results.extend(fuzzer.test_redirect_uri(&auth_endpoint_owned));
            }
            if config.enable_state_test {
                results.extend(fuzzer.test_state_parameter(&auth_endpoint_owned));
            }
            if config.enable_scope_test {
                results.extend(fuzzer.test_scope_escalation(&auth_endpoint_owned));
            }
            Ok::<_, PyErr>(results)
        });

        match result {
            Ok(r) => {
                let py_results: Vec<_> = r
                    .into_iter()
                    .map(crate::oauth::OAuthTestResultPy::from_engine)
                    .collect();
                let mut metadata = std::collections::HashMap::new();
                metadata.insert("auth_endpoint".to_string(), auth_endpoint.to_string());
                let stats = ExecutionStats::new(0, py_results.len() as u64, 0, 0);
                operation_ok(
                    stats,
                    Some(metadata),
                    Some(OperationPayload::Oauth(py_results)),
                )
            }
            Err(e) => operation_err(e.to_string()),
        }
    }

    fn run_auth_test_inner(&self, py: Python<'_>, target: &str) -> OperationResult {
        if let Err(e) = self.state.pre_dispatch_validate("auth_test", target) {
            return operation_err(e.to_string());
        }

        let target_owned = target.to_string();
        let result = runtime_sync::block_on(py, async move {
            let mut engine = eggsec::auth::AuthEngine::new(100, 10, 30, true).map_err(|e| {
                pyo3::exceptions::PyValueError::new_err(format!("Auth engine error: {}", e))
            })?;
            engine.run_full_test(&target_owned).await.map_err(|e| {
                pyo3::exceptions::PyRuntimeError::new_err(format!("Auth test failed: {}", e))
            })
        });

        match result {
            Ok(r) => {
                let py_result = crate::auth_assess::AuthTestReportPy::from_engine(r);
                let mut metadata = std::collections::HashMap::new();
                metadata.insert("target".to_string(), target.to_string());
                let stats = ExecutionStats::new(0, py_result.total_attempts as u64, 0, 0);
                operation_ok(
                    stats,
                    Some(metadata),
                    Some(OperationPayload::Auth(py_result)),
                )
            }
            Err(e) => operation_err(e.to_string()),
        }
    }

    #[allow(clippy::too_many_arguments)]
    #[cfg(feature = "db-pentest")]
    fn run_db_probe_inner(
        &self,
        py: Python<'_>,
        target: &str,
        db_type: &str,
        user: Option<&str>,
        password: Option<&str>,
        database: Option<&str>,
        port: Option<u16>,
    ) -> OperationResult {
        if let Err(e) = self.state.pre_dispatch_validate("db_probe", target) {
            return operation_err(e.to_string());
        }

        let target_owned = target.to_string();
        let db_type_owned = db_type.to_string();
        let user_owned = user.map(|s| s.to_string());
        let password_owned = password.map(|s| s.to_string());
        let database_owned = database.map(|s| s.to_string());

        let result = runtime_sync::block_on(py, async move {
            let args = crate::db_pentest::build_args(
                Some(&target_owned),
                Some(&db_type_owned),
                "all",
                200,
                120,
                false,
                None,
                port,
                user_owned.as_deref(),
                password_owned.as_deref(),
                database_owned.as_deref(),
            );
            crate::db_pentest::run_sync(args)
        });

        match result {
            Ok(r) => {
                let mut metadata = std::collections::HashMap::new();
                metadata.insert("target".to_string(), target.to_string());
                let stats = ExecutionStats::new(0, 1, 0, 0);
                operation_ok(stats, Some(metadata), Some(OperationPayload::DbProbe(r)))
            }
            Err(e) => operation_err(e.to_string()),
        }
    }

    #[cfg(feature = "nse")]
    fn run_nse_inner(
        &self,
        py: Python<'_>,
        target: &str,
        script: &str,
        script_args: Option<&str>,
    ) -> OperationResult {
        if let Err(e) = self.state.pre_dispatch_validate("nse_run", target) {
            return operation_err(e.to_string());
        }

        let target_owned = target.to_string();
        let script_owned = script.to_string();
        let script_args_owned = script_args.map(|s| s.to_string());

        let result = runtime_sync::block_on(py, async move {
            let config = crate::nse::build_nse_config(
                &target_owned,
                &script_owned,
                script_args_owned.as_deref(),
                false,
            );
            crate::nse::run_nse_sync(config, None)
        });

        match result {
            Ok(r) => {
                let mut metadata = std::collections::HashMap::new();
                metadata.insert("target".to_string(), target.to_string());
                metadata.insert("script".to_string(), script.to_string());
                let stats = ExecutionStats::new(0, 1, 0, 0);
                operation_ok(stats, Some(metadata), Some(OperationPayload::Nse(r)))
            }
            Err(e) => operation_err(e.to_string()),
        }
    }

    #[cfg(feature = "container")]
    fn run_docker_image_inner(&self, py: Python<'_>, image_name: &str) -> OperationResult {
        if let Err(e) = self
            .state
            .pre_dispatch_validate("scan_docker_image", image_name)
        {
            return operation_err(e.to_string());
        }

        let image_owned = image_name.to_string();
        let result = runtime_sync::block_on(py, async move {
            let scanner = eggsec::container::docker::DockerScanner::new();
            scanner.scan_image(&image_owned).await.map_pyerr()
        });

        match result {
            Ok(r) => {
                let py_result = crate::container::DockerScanResultPy::from_engine(r);
                let mut metadata = std::collections::HashMap::new();
                metadata.insert("image".to_string(), image_name.to_string());
                let stats = ExecutionStats::new(0, 1, 0, 0);
                operation_ok(
                    stats,
                    Some(metadata),
                    Some(OperationPayload::DockerImage(py_result)),
                )
            }
            Err(e) => operation_err(e.to_string()),
        }
    }

    #[cfg(feature = "container")]
    fn run_kubernetes_inner(
        &self,
        py: Python<'_>,
        api_server: &str,
        token: Option<&str>,
        timeout_secs: u64,
    ) -> OperationResult {
        if let Err(e) = self
            .state
            .pre_dispatch_validate("scan_kubernetes", api_server)
        {
            return operation_err(e.to_string());
        }

        let api_owned = api_server.to_string();
        let token_owned = token.map(|s| s.to_string());
        let result = runtime_sync::block_on(py, async move {
            let scanner = eggsec::container::kubernetes::KubernetesScanner::new(
                &api_owned,
                token_owned,
                timeout_secs,
            )
            .map_pyerr()?;
            scanner.scan().await.map_pyerr()
        });

        match result {
            Ok(r) => {
                let py_result = crate::container::KubernetesScanResultPy::from_engine(r);
                let mut metadata = std::collections::HashMap::new();
                metadata.insert("api_server".to_string(), api_server.to_string());
                let stats = ExecutionStats::new(0, 1, 0, 0);
                operation_ok(
                    stats,
                    Some(metadata),
                    Some(OperationPayload::Kubernetes(py_result)),
                )
            }
            Err(e) => operation_err(e.to_string()),
        }
    }

    #[cfg(feature = "mobile")]
    fn run_apk_inner(&self, py: Python<'_>, apk_path: &str) -> OperationResult {
        if let Err(e) = self.state.pre_dispatch_validate("analyze_apk", apk_path) {
            return operation_err(e.to_string());
        }

        let path_owned = apk_path.to_string();
        let result = runtime_sync::block_on(py, async move {
            let path_ref = std::path::Path::new(&path_owned);
            eggsec::mobile::analyze_apk(path_ref).await.map_err(|e| {
                crate::error::ScanError::new_err(format!("Mobile analysis failed: {}", e))
            })
        });

        match result {
            Ok(r) => {
                let py_result = crate::mobile::MobileScanReportPy::from_engine(r);
                let mut metadata = std::collections::HashMap::new();
                metadata.insert("apk_path".to_string(), apk_path.to_string());
                let stats = ExecutionStats::new(0, 1, 0, 0);
                operation_ok(
                    stats,
                    Some(metadata),
                    Some(OperationPayload::Apk(py_result)),
                )
            }
            Err(e) => operation_err(e.to_string()),
        }
    }

    #[cfg(feature = "mobile")]
    fn run_ipa_inner(&self, py: Python<'_>, ipa_path: &str) -> OperationResult {
        if let Err(e) = self.state.pre_dispatch_validate("analyze_ipa", ipa_path) {
            return operation_err(e.to_string());
        }

        let path_owned = ipa_path.to_string();
        let result = runtime_sync::block_on(py, async move {
            let path_ref = std::path::Path::new(&path_owned);
            eggsec::mobile::analyze_ipa(path_ref).await.map_err(|e| {
                crate::error::ScanError::new_err(format!("Mobile analysis failed: {}", e))
            })
        });

        match result {
            Ok(r) => {
                let py_result = crate::mobile::MobileScanReportPy::from_engine(r);
                let mut metadata = std::collections::HashMap::new();
                metadata.insert("ipa_path".to_string(), ipa_path.to_string());
                let stats = ExecutionStats::new(0, 1, 0, 0);
                operation_ok(
                    stats,
                    Some(metadata),
                    Some(OperationPayload::Ipa(py_result)),
                )
            }
            Err(e) => operation_err(e.to_string()),
        }
    }
}
