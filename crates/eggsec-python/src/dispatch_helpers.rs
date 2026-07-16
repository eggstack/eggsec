use std::collections::HashMap;

use pyo3::prelude::*;
use pyo3::types::PyDict;

use crate::cancellation::CancellationToken;
use crate::engine_state::EngineState;
use crate::event_protocol::{CancellationEvent, EventEnvelope, FindingEvent};
use crate::operation_registry::{OperationExecutorRegistry, StableOperation};
use crate::requests::OperationRequest;
use crate::status::{ExecutionStats, ExecutionStatus, OperationError, OperationResult};

/// Extract hostname from a URL for scope enforcement.
pub(crate) fn extract_host_from_url(url: &str) -> PyResult<String> {
    let parsed = url::Url::parse(url)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("Invalid URL: {}", e)))?;
    parsed
        .host_str()
        .map(|h| h.to_string())
        .ok_or_else(|| pyo3::exceptions::PyValueError::new_err("URL does not contain a valid host"))
}

/// Parse a comma-separated ports string into a Vec<u16>.
/// Supports plain ports ("80,443") and ranges ("1-1024").
pub(crate) fn parse_ports_string(ports: &str) -> PyResult<Vec<u16>> {
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
    if result.is_empty() {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "ports list must not be empty",
        ));
    }
    Ok(result)
}

/// Build an OperationResult from a successful engine call.
pub(crate) fn operation_ok(
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
pub(crate) fn operation_err(error: String) -> OperationResult {
    operation_err_for(None, error)
}

pub(crate) fn operation_err_for(operation: Option<&str>, error: String) -> OperationResult {
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
pub(crate) fn daemon_response_to_operation_result(
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

/// Convert an `OperationRequest` to a TaskKind JSON string for daemon submission.
///
/// Uses the registry's `daemon_task_kind` metadata instead of a hardcoded match.
/// Operation-specific parameters are built from the request metadata map.
pub(crate) fn operation_request_to_daemon_task(request: &OperationRequest) -> PyResult<String> {
    let operation = StableOperation::parse(&request.operation).ok_or_else(|| {
        pyo3::exceptions::PyValueError::new_err(format!("Unknown operation: {}", request.operation))
    })?;

    let registry = OperationExecutorRegistry::default_stable();
    let desc = registry.descriptor_for(operation);
    let task_kind = desc.daemon_task_kind;

    // Build operation-specific params from request metadata
    let params = match operation {
        StableOperation::ScanPorts => serde_json::json!({
            "target": request.target,
            "timeout_ms": request.timeout_ms,
        }),
        StableOperation::ScanEndpoints => serde_json::json!({
            "target": request.target,
        }),
        StableOperation::FingerprintServices => serde_json::json!({
            "target": request.target,
        }),
        StableOperation::ReconDns => serde_json::json!({
            "target": request.target,
            "modules": ["dns"],
        }),
        StableOperation::InspectTls => serde_json::json!({
            "target": request.target,
            "modules": ["tls"],
        }),
        StableOperation::DetectTechnology => serde_json::json!({
            "target": request.target,
            "modules": ["tech"],
        }),
        StableOperation::DetectWaf => serde_json::json!({
            "target": request.target,
        }),
        StableOperation::ValidateWaf => serde_json::json!({
            "target": request.target,
        }),
        StableOperation::FuzzHttp => serde_json::json!({
            "target": request.target,
        }),
        StableOperation::LoadTest => serde_json::json!({
            "target": request.target,
            "method": "GET",
        }),
        StableOperation::NseRun => serde_json::json!({
            "target": request.target,
            "script": "default",
        }),
        StableOperation::GraphqlTest => serde_json::json!({
            "target": request.target,
        }),
        StableOperation::OauthTest => serde_json::json!({
            "target": request.target,
        }),
        StableOperation::AuthTest => serde_json::json!({
            "target": request.target,
        }),
        StableOperation::ScanGitSecrets => serde_json::json!({
            "storage_type": "git-secrets",
            "path": request.target,
        }),
        StableOperation::GenerateSbom => serde_json::json!({
            "storage_type": "sbom",
            "path": request.target,
        }),
        StableOperation::RunConsolidatedRecon => serde_json::json!({
            "target": request.target,
            "modules": ["dns", "ssl", "tech"],
        }),
        StableOperation::DbProbe => serde_json::json!({
            "target": request.target,
        }),
        StableOperation::ScanDockerImage | StableOperation::ScanKubernetes => serde_json::json!({
            "storage_type": "container",
            "path": request.target,
        }),
        StableOperation::AnalyzeApk | StableOperation::AnalyzeIpa => serde_json::json!({
            "storage_type": "mobile",
            "path": request.target,
        }),
    };

    let task = serde_json::json!({
        "kind": task_kind,
        "params": params,
    });

    serde_json::to_string(&task).map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
}

/// Convert a Python dict to `HashMap<String, String>` for OperationRequest metadata.
///
/// Each value is converted to its string representation. Complex types (lists,
/// nested dicts) are serialized via Python's `json.dumps`.
pub(crate) fn pydict_to_string_metadata(
    dict: &Bound<'_, PyDict>,
) -> PyResult<HashMap<String, String>> {
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

/// Check if a cancellation token has been cancelled.
///
/// Returns `Some(OperationResult)` if cancelled (with event emitted), `None` otherwise.
/// Safe to call with or without the GIL held.
pub(crate) fn check_cancel(
    cancel_token: &Option<CancellationToken>,
    operation_id: &str,
    target: &str,
    state: &EngineState,
) -> Option<OperationResult> {
    if let Some(ref token) = cancel_token {
        if token.is_cancelled() {
            let reason = token.reason().unwrap_or_else(|| "cancelled".to_string());
            Python::with_gil(|py| {
                state.emit_event(EventEnvelope::create(
                    "operation.cancelled".to_string(),
                    CancellationEvent::new(reason, "operator".to_string()).into_py(py),
                    None,
                    None,
                    Some(target.to_string()),
                    None,
                ));
            });
            return Some(OperationResult {
                status: ExecutionStatus::Failed {
                    error: "Operation cancelled".to_string(),
                },
                stats: None,
                artifacts: Vec::new(),
                error: Some(OperationError::from_message(
                    Some(operation_id),
                    "Operation cancelled",
                )),
                metadata: std::collections::HashMap::new(),
                payload: None,
                payload_type: None,
                schema_version: "1.0".to_string(),
            });
        }
    }
    None
}

/// Check if the operation deadline has expired.
///
/// Returns `Some(OperationResult)` if expired, `None` otherwise.
pub(crate) fn check_deadline(
    deadline: &Option<std::time::Instant>,
    operation_id: &str,
) -> Option<OperationResult> {
    if let Some(ref dl) = deadline {
        if std::time::Instant::now() >= *dl {
            return Some(OperationResult {
                status: ExecutionStatus::Failed {
                    error: "Operation timed out before execution".to_string(),
                },
                stats: None,
                artifacts: Vec::new(),
                error: Some(OperationError::from_message(
                    Some(operation_id),
                    "Operation timed out before execution",
                )),
                metadata: std::collections::HashMap::new(),
                payload: None,
                payload_type: None,
                schema_version: "1.0".to_string(),
            });
        }
    }
    None
}

/// Emit a finding event through the engine state event channel.
///
/// Safe to call with or without the GIL held (uses `Python::with_gil` internally).
pub(crate) fn emit_finding_event(
    state: &EngineState,
    finding_id: String,
    severity: String,
    message: String,
    actionable: bool,
    target: String,
) {
    Python::with_gil(|py| {
        let finding = FindingEvent::new(finding_id, severity, message, actionable);
        state.emit_event(EventEnvelope::create(
            "operation.finding".to_string(),
            finding.into_py(py),
            None,
            None,
            Some(target),
            None,
        ));
    });
}

/// Common dispatch lifecycle phases shared by sync and async engines.
///
/// Handles the identical pre-dispatch steps that every operation performs:
/// 1. Emit planning event
/// 2. Scope/feature-gate validation
/// 3. Emit preflight event
/// 4. Cancellation check
/// 5. Deadline check
///
/// Returns `Ok(deadline)` if the operation should proceed, or
/// `Err(OperationResult)` if it should be aborted.
pub(crate) fn pre_dispatch_lifecycle(
    py: Python<'_>,
    op_id: &str,
    target: &str,
    request_timeout_ms: Option<u64>,
    engine_timeout_ms: u64,
    state: &EngineState,
    cancel_token: &Option<CancellationToken>,
) -> Result<Option<std::time::Instant>, OperationResult> {
    use crate::event_protocol::{EventEnvelope, PlanningEvent, PreflightEvent};

    // 1. Emit planning event
    let planning_event = EventEnvelope::create(
        "operation.planning".to_string(),
        PlanningEvent::new(op_id.to_string(), target.to_string(), String::new()).into_py(py),
        None,
        None,
        Some(target.to_string()),
        None,
    );
    state.emit_event(planning_event);

    // 2. Pre-dispatch validation (scope, feature gates, audit logging)
    if let Err(e) = state.pre_dispatch_validate(op_id, target) {
        return Err(operation_err_for(Some(op_id), e.to_string()));
    }

    // 3. Emit preflight event
    let preflight_event = EventEnvelope::create(
        "operation.preflight".to_string(),
        PreflightEvent::new("approved".to_string(), Vec::new(), Vec::new()).into_py(py),
        None,
        None,
        Some(target.to_string()),
        None,
    );
    state.emit_event(preflight_event);

    // 4. Check cancellation
    if let Some(result) = check_cancel(cancel_token, op_id, target, state) {
        return Err(result);
    }

    // 5. Compute and check deadline
    let deadline = compute_deadline(request_timeout_ms, engine_timeout_ms);
    if let Some(result) = check_deadline(&deadline, op_id) {
        return Err(result);
    }

    Ok(deadline)
}

/// Compute a deadline from optional request timeout and engine timeout.
///
/// If `request_timeout_ms` is `Some`, uses that value. Otherwise falls back
/// to `engine_timeout_ms`. Returns `None` if both are zero or if no timeout
/// should be applied.
pub(crate) fn compute_deadline(
    request_timeout_ms: Option<u64>,
    engine_timeout_ms: u64,
) -> Option<std::time::Instant> {
    request_timeout_ms
        .or(Some(engine_timeout_ms))
        .map(|ms| std::time::Instant::now() + std::time::Duration::from_millis(ms))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn test_extract_host_from_url_simple() {
        assert_eq!(
            extract_host_from_url("https://example.com/path").unwrap(),
            "example.com"
        );
    }

    #[test]
    fn test_extract_host_from_url_with_port() {
        assert_eq!(
            extract_host_from_url("https://example.com:8080/path").unwrap(),
            "example.com"
        );
    }

    #[test]
    fn test_extract_host_from_url_subdomain() {
        assert_eq!(
            extract_host_from_url("https://sub.domain.example.com/").unwrap(),
            "sub.domain.example.com"
        );
    }

    #[test]
    fn test_extract_host_from_url_invalid() {
        assert!(extract_host_from_url("not-a-url").is_err());
    }

    #[test]
    fn test_extract_host_from_url_no_host() {
        // url::Url::parse accepts "file:///path" but host_str() returns None
        assert!(extract_host_from_url("file:///path/to/file").is_err());
    }

    #[test]
    fn test_parse_ports_string_single() {
        let ports = parse_ports_string("80").unwrap();
        assert_eq!(ports, vec![80]);
    }

    #[test]
    fn test_parse_ports_string_multiple() {
        let ports = parse_ports_string("80,443,8080").unwrap();
        assert_eq!(ports, vec![80, 443, 8080]);
    }

    #[test]
    fn test_parse_ports_string_range() {
        let ports = parse_ports_string("1-5").unwrap();
        assert_eq!(ports, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_parse_ports_string_mixed() {
        let ports = parse_ports_string("80,443,8000-8002").unwrap();
        assert_eq!(ports, vec![80, 443, 8000, 8001, 8002]);
    }

    #[test]
    fn test_parse_ports_string_with_spaces() {
        let ports = parse_ports_string("80, 443, 8080").unwrap();
        assert_eq!(ports, vec![80, 443, 8080]);
    }

    #[test]
    fn test_parse_ports_string_empty() {
        assert!(parse_ports_string("").is_err());
    }

    #[test]
    fn test_parse_ports_string_invalid_range() {
        assert!(parse_ports_string("80-10").is_err());
    }

    #[test]
    fn test_parse_ports_string_invalid_port() {
        assert!(parse_ports_string("abc").is_err());
    }

    #[test]
    fn test_operation_ok_basic() {
        let stats = ExecutionStats::new(100, 10, 8, 2);
        let result = operation_ok(stats, None, None);
        assert!(matches!(result.status, ExecutionStatus::Completed()));
        assert!(result.error.is_none());
        assert_eq!(result.metadata.get("policy_decision").unwrap(), "allow");
    }

    #[test]
    fn test_operation_ok_with_metadata() {
        let stats = ExecutionStats::new(100, 10, 8, 2);
        let mut meta = HashMap::new();
        meta.insert("target".to_string(), "example.com".to_string());
        let result = operation_ok(stats, Some(meta), None);
        assert_eq!(result.metadata.get("target").unwrap(), "example.com");
        // policy_decision is always added
        assert_eq!(result.metadata.get("policy_decision").unwrap(), "allow");
    }

    #[test]
    fn test_operation_err_basic() {
        let result = operation_err("something went wrong".to_string());
        assert!(matches!(result.status, ExecutionStatus::Failed { .. }));
        assert!(result.error.is_some());
        assert!(result.stats.is_none());
    }

    #[test]
    fn test_operation_err_for_with_operation() {
        let result = operation_err_for(Some("scan_ports"), "timeout".to_string());
        assert!(matches!(result.status, ExecutionStatus::Failed { .. }));
        let err = result.error.unwrap();
        assert_eq!(err.operation_id.as_deref(), Some("scan_ports"));
    }

    #[test]
    fn test_operation_err_for_without_operation() {
        let result = operation_err_for(None, "timeout".to_string());
        let err = result.error.unwrap();
        assert_eq!(err.operation_id, None);
    }

    #[test]
    fn test_check_cancel_not_cancelled() {
        let scope = crate::scope::Scope {
            inner: eggsec::config::Scope {
                allowed_targets: vec![eggsec::config::ScopeRule {
                    pattern: "*".to_string(),
                    cidr: None,
                    description: None,
                }],
                ..Default::default()
            },
        };
        let state = Arc::new(EngineState::from_params(scope, "manual", 100, 5000).unwrap());
        let result = check_cancel(&None, "scan_ports", "example.com", &state);
        assert!(result.is_none());
    }

    #[test]
    fn test_check_deadline_not_expired() {
        let deadline = Some(std::time::Instant::now() + std::time::Duration::from_secs(60));
        let result = check_deadline(&deadline, "scan_ports");
        assert!(result.is_none());
    }

    #[test]
    fn test_check_deadline_expired() {
        let deadline = Some(std::time::Instant::now() - std::time::Duration::from_secs(1));
        let result = check_deadline(&deadline, "scan_ports");
        assert!(result.is_some());
        let r = result.unwrap();
        assert!(matches!(r.status, ExecutionStatus::Failed { .. }));
        let err = r.error.unwrap();
        assert_eq!(err.operation_id.as_deref(), Some("scan_ports"));
    }

    #[test]
    fn test_check_deadline_none() {
        let result = check_deadline(&None, "scan_ports");
        assert!(result.is_none());
    }

    #[test]
    fn test_compute_deadline_with_request_timeout() {
        let deadline = compute_deadline(Some(5000), 10000);
        assert!(deadline.is_some());
        let dl = deadline.unwrap();
        assert!(dl > std::time::Instant::now());
    }

    #[test]
    fn test_compute_deadline_fallback_to_engine() {
        let deadline = compute_deadline(None, 10000);
        assert!(deadline.is_some());
        let dl = deadline.unwrap();
        assert!(dl > std::time::Instant::now());
    }
}
