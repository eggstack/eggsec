//! Per-operation executor functions for the Python binding layer.
//!
//! Each function in this module handles the normalization of a generic
//! [`OperationRequest`] into typed parameters for a specific operation.
//! These functions are intended to replace the match arms in
//! [`Engine::dispatch`] and [`AsyncEngine::dispatch_async`] incrementally.
//!
//! For now the functions are defined but not yet wired into the dispatch
//! path. This module compiles independently of the engine dispatch.

use std::collections::HashMap;

use crate::operation_registry::StableOperation;
use crate::requests::OperationRequest;
use crate::status::{OperationError, OperationResult};

/// Result of normalizing a generic [`OperationRequest`] into typed parameters.
///
/// Contains the common fields extracted from the request plus operation-specific
/// parameters in the `params` map.
pub struct NormalizedRequest {
    /// The target string (URL, domain, IP, CIDR, or file path).
    pub target: String,
    /// Per-operation timeout override (milliseconds), if any.
    pub timeout_ms: Option<u64>,
    /// Operation-specific parameters extracted from the request metadata.
    pub params: HashMap<String, String>,
}

/// Normalize a generic [`OperationRequest`] into a [`NormalizedRequest`].
///
/// Extracts common fields and forwards the metadata map as-is. Operation-specific
/// executors can then pull their parameters from `params` with typed parsing.
pub fn normalize_request(
    _op: StableOperation,
    request: &OperationRequest,
    engine_timeout_ms: u64,
) -> NormalizedRequest {
    NormalizedRequest {
        target: request.target.clone(),
        timeout_ms: request.timeout_ms.or(Some(engine_timeout_ms)),
        params: request.metadata.clone(),
    }
}

/// Check whether the feature gate for the given operation is satisfied.
///
/// Returns `Ok(())` if the feature is available (or no feature is required),
/// or `Err(OperationResult)` with a `feature_unavailable` error if the build
/// was compiled without the required feature.
pub fn check_feature_gate(op: StableOperation) -> Result<(), OperationResult> {
    if let Some(feature) = op.feature_required() {
        if !crate::features::has_feature(feature) {
            return Err(OperationResult {
                status: crate::status::ExecutionStatus::Failed {
                    error: format!(
                        "Operation '{}' requires feature '{}' which is not compiled in this build",
                        op.id(),
                        feature
                    ),
                },
                stats: None,
                artifacts: Vec::new(),
                error: Some(OperationError::with_code(
                    Some(op.id()),
                    "feature_unavailable",
                    "feature_unavailable",
                    format!(
                        "Operation '{}' requires feature '{}' which is not compiled in this build",
                        op.id(),
                        feature
                    ),
                    false,
                )),
                metadata: HashMap::new(),
                payload: None,
                payload_type: None,
                schema_version: "1.0".to_string(),
            });
        }
    }
    Ok(())
}

/// Extract a string parameter from the metadata map, falling back to the default.
pub fn param_str(params: &HashMap<String, String>, key: &str, default: &str) -> String {
    params
        .get(key)
        .cloned()
        .unwrap_or_else(|| default.to_string())
}

/// Extract an optional string parameter from the metadata map.
pub fn param_opt(params: &HashMap<String, String>, key: &str) -> Option<String> {
    params.get(key).cloned()
}

/// Extract a numeric parameter from the metadata map, falling back to the default.
pub fn param_u64(params: &HashMap<String, String>, key: &str, default: u64) -> u64 {
    params
        .get(key)
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

/// Extract an optional numeric parameter from the metadata map.
pub fn param_u64_opt(params: &HashMap<String, String>, key: &str) -> Option<u64> {
    params.get(key).and_then(|s| s.parse().ok())
}

/// Extract a usize parameter from the metadata map, falling back to the default.
pub fn param_usize(params: &HashMap<String, String>, key: &str, default: usize) -> usize {
    params
        .get(key)
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

/// Extract a boolean parameter from the metadata map, falling back to the default.
pub fn param_bool(params: &HashMap<String, String>, key: &str, default: bool) -> bool {
    params
        .get(key)
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

/// Extract a comma-separated list of strings from the metadata map.
pub fn param_csv(params: &HashMap<String, String>, key: &str) -> Vec<String> {
    params
        .get(key)
        .map(|s| {
            s.split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

// ---------------------------------------------------------------------------
// Representative per-operation executor functions
//
// These show the intended pattern for the eventual migration. Each function
// takes a normalized request and returns either typed parameters or an error.
// ---------------------------------------------------------------------------

/// Parameters extracted for a port scan operation.
pub struct PortScanParams {
    pub target: String,
    pub ports_str: String,
    pub timeout_ms: u64,
    pub concurrency: usize,
}

/// Normalize an `OperationRequest` into port scan parameters.
pub fn normalize_port_scan(
    normalized: &NormalizedRequest,
    default_concurrency: usize,
) -> Result<PortScanParams, OperationResult> {
    let ports_str = param_str(&normalized.params, "ports", "1-1024");
    Ok(PortScanParams {
        target: normalized.target.clone(),
        ports_str,
        timeout_ms: normalized.timeout_ms.unwrap_or(5000),
        concurrency: param_usize(&normalized.params, "concurrency", default_concurrency),
    })
}

/// Parameters extracted for an endpoint scan operation.
pub struct EndpointScanParams {
    pub target: String,
    pub endpoints: Vec<String>,
    pub timeout_ms: u64,
}

/// Normalize an `OperationRequest` into endpoint scan parameters.
pub fn normalize_endpoint_scan(
    normalized: &NormalizedRequest,
) -> Result<EndpointScanParams, OperationResult> {
    let endpoints = param_csv(&normalized.params, "endpoints");
    Ok(EndpointScanParams {
        target: normalized.target.clone(),
        endpoints,
        timeout_ms: normalized.timeout_ms.unwrap_or(5000),
    })
}

/// Parameters extracted for a fingerprint operation.
pub struct FingerprintParams {
    pub target: String,
    pub ports: Vec<u16>,
    pub timeout_ms: u64,
}

/// Normalize an `OperationRequest` into fingerprint parameters.
pub fn normalize_fingerprint(
    normalized: &NormalizedRequest,
) -> Result<FingerprintParams, OperationResult> {
    let ports_str = param_str(&normalized.params, "ports", "");
    let ports = if ports_str.is_empty() {
        vec![80, 443]
    } else {
        crate::dispatch_helpers::parse_ports_string(&ports_str).map_err(|e| OperationResult {
            status: crate::status::ExecutionStatus::Failed {
                error: e.to_string(),
            },
            stats: None,
            artifacts: Vec::new(),
            error: Some(OperationError::with_code(
                Some("fingerprint_services"),
                "validation",
                "invalid_ports",
                e.to_string(),
                false,
            )),
            metadata: HashMap::new(),
            payload: None,
            payload_type: None,
            schema_version: "1.0".to_string(),
        })?
    };
    Ok(FingerprintParams {
        target: normalized.target.clone(),
        ports,
        timeout_ms: normalized.timeout_ms.unwrap_or(5000),
    })
}

/// Parameters extracted for a load test operation.
pub struct LoadTestParams {
    pub target: String,
    pub total_requests: u64,
    pub concurrency: usize,
    pub method: String,
    pub timeout_ms: u64,
}

/// Normalize an `OperationRequest` into load test parameters.
pub fn normalize_load_test(
    normalized: &NormalizedRequest,
    default_concurrency: usize,
) -> Result<LoadTestParams, OperationResult> {
    Ok(LoadTestParams {
        target: normalized.target.clone(),
        total_requests: param_u64(&normalized.params, "requests", 100),
        concurrency: param_usize(&normalized.params, "concurrency", default_concurrency),
        method: param_str(&normalized.params, "method", "GET"),
        timeout_ms: normalized.timeout_ms.unwrap_or(5000),
    })
}

/// Parameters extracted for an NSE run operation.
pub struct NseRunParams {
    pub target: String,
    pub script_name: String,
    pub script_args: Option<String>,
    pub timeout_ms: u64,
}

/// Normalize an `OperationRequest` into NSE run parameters.
pub fn normalize_nse_run(normalized: &NormalizedRequest) -> Result<NseRunParams, OperationResult> {
    let scripts = param_csv(&normalized.params, "scripts");
    let script_name = scripts
        .first()
        .cloned()
        .unwrap_or_else(|| "default".to_string());
    let script_args = param_opt(&normalized.params, "script_args");
    Ok(NseRunParams {
        target: normalized.target.clone(),
        script_name,
        script_args,
        timeout_ms: normalized.timeout_ms.unwrap_or(5000),
    })
}

/// Parameters extracted for a database probe operation.
pub struct DbProbeParams {
    pub target: String,
    pub db_type: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub database: Option<String>,
    pub port: Option<u16>,
    pub timeout_ms: u64,
}

/// Normalize an `OperationRequest` into database probe parameters.
pub fn normalize_db_probe(
    normalized: &NormalizedRequest,
) -> Result<DbProbeParams, OperationResult> {
    Ok(DbProbeParams {
        target: normalized.target.clone(),
        db_type: param_str(&normalized.params, "db_type", "all"),
        username: param_opt(&normalized.params, "username"),
        password: param_opt(&normalized.params, "password"),
        database: param_opt(&normalized.params, "database"),
        port: param_u64_opt(&normalized.params, "port").map(|p| p as u16),
        timeout_ms: normalized.timeout_ms.unwrap_or(5000),
    })
}

/// Parameters extracted for a fuzz HTTP operation.
pub struct FuzzHttpParams {
    pub target: String,
    pub payload_type: Option<String>,
    pub threads: Option<u32>,
    pub timeout_ms: u64,
}

/// Normalize an `OperationRequest` into fuzz HTTP parameters.
pub fn normalize_fuzz_http(
    normalized: &NormalizedRequest,
) -> Result<FuzzHttpParams, OperationResult> {
    Ok(FuzzHttpParams {
        target: normalized.target.clone(),
        payload_type: param_opt(&normalized.params, "payload_type"),
        threads: param_u64_opt(&normalized.params, "threads").map(|t| t as u32),
        timeout_ms: normalized.timeout_ms.unwrap_or(5000),
    })
}
