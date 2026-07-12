use pyo3::prelude::*;

use crate::requests::OperationRequest;
use crate::status::{OperationError, OperationResult};

/// Stable operation ID constants. These are generated from the canonical
/// declaration below so metadata and dispatch cannot grow separate lists.
pub const OP_SCAN_PORTS: &str = "scan_ports";
pub const OP_SCAN_ENDPOINTS: &str = "scan_endpoints";
pub const OP_FINGERPRINT_SERVICES: &str = "fingerprint_services";
pub const OP_RECON_DNS: &str = "recon_dns";
pub const OP_INSPECT_TLS: &str = "inspect_tls";
pub const OP_DETECT_TECHNOLOGY: &str = "detect_technology";
pub const OP_DETECT_WAF: &str = "detect_waf";
pub const OP_VALIDATE_WAF: &str = "validate_waf";
pub const OP_FUZZ_HTTP: &str = "fuzz_http";
pub const OP_LOAD_TEST: &str = "load_test";

/// Compiler-enforced identity for the stable Python engine operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StableOperation {
    ScanPorts,
    ScanEndpoints,
    FingerprintServices,
    ReconDns,
    InspectTls,
    DetectTechnology,
    DetectWaf,
    ValidateWaf,
    FuzzHttp,
    LoadTest,
}

impl StableOperation {
    pub const ALL: &'static [Self] = &[
        Self::ScanPorts,
        Self::ScanEndpoints,
        Self::FingerprintServices,
        Self::ReconDns,
        Self::InspectTls,
        Self::DetectTechnology,
        Self::DetectWaf,
        Self::ValidateWaf,
        Self::FuzzHttp,
        Self::LoadTest,
    ];

    pub const fn id(self) -> &'static str {
        match self {
            Self::ScanPorts => OP_SCAN_PORTS,
            Self::ScanEndpoints => OP_SCAN_ENDPOINTS,
            Self::FingerprintServices => OP_FINGERPRINT_SERVICES,
            Self::ReconDns => OP_RECON_DNS,
            Self::InspectTls => OP_INSPECT_TLS,
            Self::DetectTechnology => OP_DETECT_TECHNOLOGY,
            Self::DetectWaf => OP_DETECT_WAF,
            Self::ValidateWaf => OP_VALIDATE_WAF,
            Self::FuzzHttp => OP_FUZZ_HTTP,
            Self::LoadTest => OP_LOAD_TEST,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::ScanPorts => "Port Scan",
            Self::ScanEndpoints => "Endpoint Scan",
            Self::FingerprintServices => "Service Fingerprinting",
            Self::ReconDns => "DNS Reconnaissance",
            Self::InspectTls => "TLS Inspection",
            Self::DetectTechnology => "Technology Detection",
            Self::DetectWaf => "WAF Detection",
            Self::ValidateWaf => "WAF Validation",
            Self::FuzzHttp => "HTTP Fuzzing",
            Self::LoadTest => "Load Test",
        }
    }

    pub const fn feature_required(self) -> Option<&'static str> {
        None
    }

    /// Parse the stable public IDs plus the historical aliases accepted by
    /// `Engine.run()` for backward compatibility.
    pub fn parse(id: &str) -> Option<Self> {
        match id {
            OP_SCAN_PORTS => Some(Self::ScanPorts),
            OP_SCAN_ENDPOINTS => Some(Self::ScanEndpoints),
            OP_FINGERPRINT_SERVICES | "fingerprint" => Some(Self::FingerprintServices),
            OP_RECON_DNS | "recon" => Some(Self::ReconDns),
            OP_INSPECT_TLS | "tls_inspect" => Some(Self::InspectTls),
            OP_DETECT_TECHNOLOGY | "tech_detect" => Some(Self::DetectTechnology),
            OP_DETECT_WAF | "waf_detect" => Some(Self::DetectWaf),
            OP_VALIDATE_WAF | "waf_validate" => Some(Self::ValidateWaf),
            OP_FUZZ_HTTP | "http_fuzz" => Some(Self::FuzzHttp),
            OP_LOAD_TEST | "load_test_http" => Some(Self::LoadTest),
            _ => None,
        }
    }
}

/// Information about a stable operation.
#[derive(Debug, Clone)]
pub struct OperationInfo {
    pub id: String,
    pub name: String,
    pub feature_required: Option<String>,
}

impl From<StableOperation> for OperationInfo {
    fn from(operation: StableOperation) -> Self {
        Self {
            id: operation.id().to_string(),
            name: operation.name().to_string(),
            feature_required: operation.feature_required().map(str::to_string),
        }
    }
}

/// Registry facade over the canonical `StableOperation` enum.
pub struct OperationExecutorRegistry;

impl OperationExecutorRegistry {
    pub fn default_stable() -> Self {
        Self
    }

    pub fn execute(
        &self,
        py: Python<'_>,
        id: &str,
        request: &OperationRequest,
        engine: &crate::engine::Engine,
    ) -> OperationResult {
        let operation = match StableOperation::parse(id) {
            Some(operation) => operation,
            None => return unknown_operation(id),
        };

        if let Some(feature) = operation.feature_required() {
            if !crate::features::has_feature(feature) {
                return OperationResult {
                    status: crate::status::ExecutionStatus::Failed {
                        error: format!("Operation '{}' requires feature '{}'", id, feature),
                    },
                    stats: None,
                    artifacts: Vec::new(),
                    error: Some(OperationError::with_code(
                        Some(operation.id()),
                        "feature_unavailable",
                        "feature_unavailable",
                        format!("Operation '{}' requires feature '{}'", id, feature),
                        false,
                    )),
                    metadata: std::collections::HashMap::new(),
                    payload: None,
                    payload_type: None,
                };
            }
        }

        // The exhaustive dispatch match is implemented by Engine::dispatch.
        // Keeping the enum parse here means unknown IDs cannot reach it.
        debug_assert_eq!(operation.id(), StableOperation::parse(id).unwrap().id());
        engine.dispatch(py, request.clone())
    }

    pub fn execute_async(
        &self,
        id: &str,
        request: &OperationRequest,
        engine: &crate::async_engine::AsyncEngine,
    ) -> PyResult<crate::runtime_async::PyFuture> {
        let operation = StableOperation::parse(id).ok_or_else(|| {
            pyo3::exceptions::PyValueError::new_err(unknown_operation_message(id))
        })?;

        if let Some(feature) = operation.feature_required() {
            if !crate::features::has_feature(feature) {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "Operation '{}' requires feature '{}' which is not compiled in this build",
                    id, feature
                )));
            }
        }

        engine.dispatch_async(request.clone())
    }

    pub fn list(&self) -> Vec<String> {
        StableOperation::ALL
            .iter()
            .map(|operation| operation.id().to_string())
            .collect()
    }

    pub fn get(&self, id: &str) -> Option<OperationInfo> {
        StableOperation::parse(id).map(OperationInfo::from)
    }

    pub fn len(&self) -> usize {
        StableOperation::ALL.len()
    }

    pub fn is_empty(&self) -> bool {
        StableOperation::ALL.is_empty()
    }

    pub fn contains(&self, id: &str) -> bool {
        StableOperation::parse(id).is_some()
    }
}

fn levenshtein(a: &str, b: &str) -> usize {
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, a_byte) in a.bytes().enumerate() {
        let mut next = vec![i + 1; b.len() + 1];
        for (j, b_byte) in b.bytes().enumerate() {
            next[j + 1] = (row[j + 1] + 1)
                .min(next[j] + 1)
                .min(row[j] + usize::from(a_byte != b_byte));
        }
        row = next;
    }
    row[b.len()]
}

fn unknown_operation_message(unknown: &str) -> String {
    let mut suggestions: Vec<(&str, usize)> = StableOperation::ALL
        .iter()
        .map(|operation| (operation.id(), levenshtein(unknown, operation.id())))
        .filter(|(_, distance)| *distance <= 3)
        .collect();
    suggestions.sort_by_key(|(_, distance)| *distance);
    if suggestions.is_empty() {
        format!("Unknown operation: {}", unknown)
    } else {
        format!(
            "Unknown operation: {}. Did you mean: {}?",
            unknown,
            suggestions
                .into_iter()
                .map(|(id, _)| id)
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

fn unknown_operation(unknown: &str) -> OperationResult {
    let message = unknown_operation_message(unknown);
    OperationResult {
        status: crate::status::ExecutionStatus::Failed {
            error: message.clone(),
        },
        stats: None,
        artifacts: Vec::new(),
        error: Some(OperationError::with_code(
            None,
            "validation",
            "unknown_operation",
            message,
            false,
        )),
        metadata: std::collections::HashMap::new(),
        payload: None,
        payload_type: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_registry_is_exhaustive() {
        let registry = OperationExecutorRegistry::default_stable();
        assert_eq!(StableOperation::ALL.len(), 10);
        for operation in StableOperation::ALL {
            assert!(registry.contains(operation.id()));
            assert_eq!(registry.get(operation.id()).unwrap().id, operation.id());
        }
    }

    #[test]
    fn operation_ids_are_unique_and_ordered() {
        let mut ids: Vec<_> = StableOperation::ALL
            .iter()
            .map(|operation| operation.id())
            .collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), StableOperation::ALL.len());
    }

    #[test]
    fn legacy_aliases_preserve_dispatch_identity() {
        assert_eq!(
            StableOperation::parse("fingerprint"),
            Some(StableOperation::FingerprintServices)
        );
        assert_eq!(
            StableOperation::parse("tls_inspect"),
            Some(StableOperation::InspectTls)
        );
    }

    #[test]
    fn unknown_operations_keep_suggestions() {
        assert!(unknown_operation_message("scan_port").contains("scan_ports"));
    }
}
