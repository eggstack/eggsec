use std::collections::HashMap;

use pyo3::prelude::*;

use crate::requests::OperationRequest;
use crate::status::OperationResult;

/// Stable operation ID constants.
///
/// These string identifiers form the canonical dispatch surface for
/// `Engine.run()` and `AsyncEngine.run()`. All operations are registered
/// in the `OperationExecutorRegistry` at engine construction time.
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

/// All stable operation IDs in registration order.
pub const STABLE_OPERATION_IDS: &[&str] = &[
    OP_SCAN_PORTS,
    OP_SCAN_ENDPOINTS,
    OP_FINGERPRINT_SERVICES,
    OP_RECON_DNS,
    OP_INSPECT_TLS,
    OP_DETECT_TECHNOLOGY,
    OP_DETECT_WAF,
    OP_VALIDATE_WAF,
    OP_FUZZ_HTTP,
    OP_LOAD_TEST,
];

/// Human-readable name for each operation, indexed to match `STABLE_OPERATION_IDS`.
const OPERATION_NAMES: &[&str] = &[
    "Port Scan",
    "Endpoint Scan",
    "Service Fingerprinting",
    "DNS Reconnaissance",
    "TLS Inspection",
    "Technology Detection",
    "WAF Detection",
    "WAF Validation",
    "HTTP Fuzzing",
    "Load Test",
];

/// Feature flag required by each operation, or `None` if always available.
/// Indexed to match `STABLE_OPERATION_IDS`.
const OPERATION_FEATURES: &[Option<&str>] =
    &[None, None, None, None, None, None, None, None, None, None];

/// Minimal Levenshtein distance for "Did you mean?" suggestions.
fn levenshtein(a: &str, b: &str) -> usize {
    let a_len = a.len();
    let b_len = b.len();
    let mut matrix = vec![vec![0usize; b_len + 1]; a_len + 1];

    for i in 0..=a_len {
        matrix[i][0] = i;
    }
    for j in 0..=b_len {
        matrix[0][j] = j;
    }

    let a_bytes = a.as_bytes();
    let b_bytes = b.as_bytes();

    for i in 1..=a_len {
        for j in 1..=b_len {
            let cost = if a_bytes[i - 1] == b_bytes[j - 1] {
                0
            } else {
                1
            };
            matrix[i][j] = (matrix[i - 1][j] + 1)
                .min(matrix[i][j - 1] + 1)
                .min(matrix[i - 1][j - 1] + cost);
        }
    }

    matrix[a_len][b_len]
}

/// Find close matches for an unknown operation ID.
fn suggest_operations(unknown: &str, known: &[&str]) -> Vec<String> {
    let mut suggestions: Vec<(usize, &str)> = known
        .iter()
        .map(|&id| (levenshtein(unknown, id), id))
        .filter(|&(dist, _)| dist <= 3)
        .collect();
    suggestions.sort_by_key(|&(dist, _)| dist);
    suggestions
        .into_iter()
        .map(|(_, id)| id.to_string())
        .collect()
}

/// Information about a registered operation.
#[derive(Debug, Clone)]
pub struct OperationInfo {
    pub id: String,
    pub name: String,
    pub feature_required: Option<String>,
}

/// Registry mapping operation IDs to executor functions.
///
/// Created once per `Engine` / `AsyncEngine` instance at construction time.
/// Contains all stable operations with their feature requirements.
///
/// The registry does NOT hold async executors directly — async engines
/// wrap the sync executor in a spawn call at dispatch time.
pub struct OperationExecutorRegistry {
    executors: HashMap<String, OperationInfo>,
}

impl OperationExecutorRegistry {
    /// Create a default registry with all stable operations registered.
    pub fn default_stable() -> Self {
        let mut executors = HashMap::new();
        for (i, &id) in STABLE_OPERATION_IDS.iter().enumerate() {
            executors.insert(
                id.to_string(),
                OperationInfo {
                    id: id.to_string(),
                    name: OPERATION_NAMES[i].to_string(),
                    feature_required: OPERATION_FEATURES[i].map(String::from),
                },
            );
        }
        Self { executors }
    }

    /// Register a new operation.
    pub fn register(&mut self, id: &str, name: &str, feature_required: Option<&str>) {
        self.executors.insert(
            id.to_string(),
            OperationInfo {
                id: id.to_string(),
                name: name.to_string(),
                feature_required: feature_required.map(String::from),
            },
        );
    }

    /// Execute an operation by ID, dispatching to the engine's internal methods.
    ///
    /// Returns `OperationResult` with `Failed` status for unknown operations,
    /// missing features, or execution errors.
    pub fn execute(
        &self,
        py: Python<'_>,
        id: &str,
        request: &OperationRequest,
        engine: &crate::engine::Engine,
    ) -> OperationResult {
        let info = match self.executors.get(id) {
            Some(info) => info.clone(),
            None => {
                let known: Vec<&str> = self.executors.keys().map(|s| s.as_str()).collect();
                let suggestions = suggest_operations(id, &known);
                let msg = if suggestions.is_empty() {
                    format!("Unknown operation: {}", id)
                } else {
                    format!(
                        "Unknown operation: {}. Did you mean: {}?",
                        id,
                        suggestions.join(", ")
                    )
                };
                return OperationResult {
                    status: crate::status::ExecutionStatus::Failed { error: msg.clone() },
                    stats: None,
                    artifacts: Vec::new(),
                    error: Some(msg),
                    metadata: HashMap::new(),
                    payload: None,
                    payload_type: None,
                };
            }
        };

        // Feature gate check
        if let Some(ref feature) = info.feature_required {
            if !crate::features::has_feature(feature) {
                let msg = format!(
                    "Operation '{}' requires feature '{}' which is not compiled in this build",
                    id, feature
                );
                return OperationResult {
                    status: crate::status::ExecutionStatus::Failed { error: msg.clone() },
                    stats: None,
                    artifacts: Vec::new(),
                    error: Some(msg),
                    metadata: HashMap::new(),
                    payload: None,
                    payload_type: None,
                };
            }
        }

        engine.dispatch(py, request.clone())
    }

    /// Execute an async operation by ID.
    ///
    /// Returns a `PyFuture` that resolves to `OperationResult`.
    pub fn execute_async(
        &self,
        id: &str,
        request: &OperationRequest,
        engine: &crate::async_engine::AsyncEngine,
    ) -> PyResult<crate::runtime_async::PyFuture> {
        let info = match self.executors.get(id) {
            Some(info) => info.clone(),
            None => {
                let known: Vec<&str> = self.executors.keys().map(|s| s.as_str()).collect();
                let suggestions = suggest_operations(id, &known);
                let msg = if suggestions.is_empty() {
                    format!("Unknown operation: {}", id)
                } else {
                    format!(
                        "Unknown operation: {}. Did you mean: {}?",
                        id,
                        suggestions.join(", ")
                    )
                };
                return Err(pyo3::exceptions::PyValueError::new_err(msg));
            }
        };

        // Feature gate check
        if let Some(ref feature) = info.feature_required {
            if !crate::features::has_feature(feature) {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "Operation '{}' requires feature '{}' which is not compiled in this build",
                    id, feature
                )));
            }
        }

        engine.dispatch_async(request.clone())
    }

    /// Return all registered operation IDs.
    pub fn list(&self) -> Vec<String> {
        let mut ids: Vec<String> = self.executors.keys().cloned().collect();
        ids.sort();
        ids
    }

    /// Return operation info for a given ID, or `None` if not found.
    pub fn get(&self, id: &str) -> Option<OperationInfo> {
        self.executors.get(id).cloned()
    }

    /// Return the number of registered operations.
    pub fn len(&self) -> usize {
        self.executors.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.executors.is_empty()
    }

    /// Check if a given operation ID is registered.
    pub fn contains(&self, id: &str) -> bool {
        self.executors.contains_key(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_operation_ids_count() {
        assert_eq!(STABLE_OPERATION_IDS.len(), 10);
        assert_eq!(OPERATION_NAMES.len(), 10);
        assert_eq!(OPERATION_FEATURES.len(), 10);
    }

    #[test]
    fn operation_ids_are_unique() {
        let mut seen = std::collections::HashSet::new();
        for id in STABLE_OPERATION_IDS {
            assert!(seen.insert(id), "Duplicate operation ID: {}", id);
        }
    }

    #[test]
    fn default_registry_contains_all_stable() {
        let reg = OperationExecutorRegistry::default_stable();
        assert_eq!(reg.len(), 10);
        for id in STABLE_OPERATION_IDS {
            assert!(reg.contains(id), "Missing operation: {}", id);
        }
    }

    #[test]
    fn list_returns_sorted_ids() {
        let reg = OperationExecutorRegistry::default_stable();
        let ids = reg.list();
        let mut sorted = ids.clone();
        sorted.sort();
        assert_eq!(ids, sorted);
    }

    #[test]
    fn get_returns_operation_info() {
        let reg = OperationExecutorRegistry::default_stable();
        let info = reg.get("scan_ports").unwrap();
        assert_eq!(info.id, "scan_ports");
        assert_eq!(info.name, "Port Scan");
        assert!(info.feature_required.is_none());
    }

    #[test]
    fn register_new_operation() {
        let mut reg = OperationExecutorRegistry::default_stable();
        reg.register("custom_op", "Custom Operation", Some("custom-feature"));
        assert!(reg.contains("custom_op"));
        let info = reg.get("custom_op").unwrap();
        assert_eq!(info.name, "Custom Operation");
        assert_eq!(info.feature_required.as_deref(), Some("custom-feature"));
    }

    #[test]
    fn levenshtein_basic() {
        assert_eq!(levenshtein("", ""), 0);
        assert_eq!(levenshtein("abc", "abc"), 0);
        assert_eq!(levenshtein("abc", "ab"), 1);
        assert_eq!(levenshtein("abc", "ac"), 1);
        assert_eq!(levenshtein("kitten", "sitting"), 3);
    }

    #[test]
    fn suggest_operations_finds_close_matches() {
        let known = vec!["scan_ports", "scan_endpoints", "fuzz_http"];
        let suggestions = suggest_operations("scan_port", &known);
        assert!(suggestions.contains(&"scan_ports".to_string()));
    }

    #[test]
    fn suggest_operations_empty_for_distant_matches() {
        let known = vec!["scan_ports", "scan_endpoints"];
        let suggestions = suggest_operations("xyzzy", &known);
        assert!(suggestions.is_empty());
    }
}
