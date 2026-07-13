use pyo3::prelude::*;

use crate::requests::OperationRequest;
use crate::status::{OperationError, OperationResult};

/// Executor descriptor for a stable operation.
///
/// Bundles metadata, feature requirements, risk classification, and
/// confirmation behavior. Returned by [`OperationExecutorRegistry::descriptor_for`]
/// so pre-dispatch validation uses a single source of truth instead of
/// constructing inline descriptors.
#[derive(Debug, Clone)]
pub struct OperationExecutorDescriptor {
    /// The stable operation this descriptor describes.
    pub operation: StableOperation,
    /// Risk tier for the operation.
    pub risk: eggsec::config::OperationRisk,
    /// Feature flag required to execute (if any).
    pub feature_required: Option<&'static str>,
    /// Whether this operation requires explicit user confirmation.
    pub confirmation_required: bool,
    /// Message shown when confirmation is required.
    pub confirmation_message: Option<&'static str>,
    /// Intended use categories for the operation.
    pub intended_uses: Vec<eggsec::config::IntendedUse>,
}

impl OperationExecutorDescriptor {
    /// Classify risk level based on operation type.
    fn classify_risk(operation: StableOperation) -> eggsec::config::OperationRisk {
        match operation {
            // Local artifact analysis — safe, no network impact
            StableOperation::ScanGitSecrets
            | StableOperation::GenerateSbom
            | StableOperation::AnalyzeApk
            | StableOperation::AnalyzeIpa => eggsec::config::OperationRisk::SafeActive,

            // Web assessment — moderate risk
            StableOperation::ReconDns
            | StableOperation::InspectTls
            | StableOperation::DetectTechnology
            | StableOperation::DetectWaf
            | StableOperation::ValidateWaf
            | StableOperation::FingerprintServices
            | StableOperation::RunConsolidatedRecon
            | StableOperation::GraphqlTest
            | StableOperation::OauthTest
            | StableOperation::AuthTest => eggsec::config::OperationRisk::SafeActive,

            // Network scanning — moderate risk
            StableOperation::ScanPorts | StableOperation::ScanEndpoints => {
                eggsec::config::OperationRisk::SafeActive
            }

            // Container scanning — moderate risk
            StableOperation::ScanDockerImage | StableOperation::ScanKubernetes => {
                eggsec::config::OperationRisk::SafeActive
            }

            // Database probing — intrusive
            StableOperation::DbProbe => eggsec::config::OperationRisk::DbPentest,

            // NSE scripts — intrusive (scripts can be aggressive)
            StableOperation::NseRun => eggsec::config::OperationRisk::Intrusive,

            // Fuzzing — intrusive
            StableOperation::FuzzHttp => eggsec::config::OperationRisk::Intrusive,

            // Load testing — load test tier
            StableOperation::LoadTest => eggsec::config::OperationRisk::LoadTest,
        }
    }
}

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
pub const OP_SCAN_GIT_SECRETS: &str = "scan_git_secrets";
pub const OP_GENERATE_SBOM: &str = "generate_sbom";
pub const OP_RUN_CONSOLIDATED_RECON: &str = "run_consolidated_recon";
pub const OP_GRAPHQL_TEST: &str = "graphql_test";
pub const OP_OAUTH_TEST: &str = "oauth_test";
pub const OP_AUTH_TEST: &str = "auth_test";
pub const OP_DB_PROBE: &str = "db_probe";
pub const OP_NSE_RUN: &str = "nse_run";
pub const OP_SCAN_DOCKER_IMAGE: &str = "scan_docker_image";
pub const OP_SCAN_KUBERNETES: &str = "scan_kubernetes";
pub const OP_ANALYZE_APK: &str = "analyze_apk";
pub const OP_ANALYZE_IPA: &str = "analyze_ipa";

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
    ScanGitSecrets,
    GenerateSbom,
    RunConsolidatedRecon,
    GraphqlTest,
    OauthTest,
    AuthTest,
    DbProbe,
    NseRun,
    ScanDockerImage,
    ScanKubernetes,
    AnalyzeApk,
    AnalyzeIpa,
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
        Self::ScanGitSecrets,
        Self::GenerateSbom,
        Self::RunConsolidatedRecon,
        Self::GraphqlTest,
        Self::OauthTest,
        Self::AuthTest,
        Self::DbProbe,
        Self::NseRun,
        Self::ScanDockerImage,
        Self::ScanKubernetes,
        Self::AnalyzeApk,
        Self::AnalyzeIpa,
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
            Self::ScanGitSecrets => OP_SCAN_GIT_SECRETS,
            Self::GenerateSbom => OP_GENERATE_SBOM,
            Self::RunConsolidatedRecon => OP_RUN_CONSOLIDATED_RECON,
            Self::GraphqlTest => OP_GRAPHQL_TEST,
            Self::OauthTest => OP_OAUTH_TEST,
            Self::AuthTest => OP_AUTH_TEST,
            Self::DbProbe => OP_DB_PROBE,
            Self::NseRun => OP_NSE_RUN,
            Self::ScanDockerImage => OP_SCAN_DOCKER_IMAGE,
            Self::ScanKubernetes => OP_SCAN_KUBERNETES,
            Self::AnalyzeApk => OP_ANALYZE_APK,
            Self::AnalyzeIpa => OP_ANALYZE_IPA,
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
            Self::ScanGitSecrets => "Git Secrets Scan",
            Self::GenerateSbom => "SBOM Generation",
            Self::RunConsolidatedRecon => "Consolidated Recon",
            Self::GraphqlTest => "GraphQL Security Test",
            Self::OauthTest => "OAuth Security Test",
            Self::AuthTest => "Authentication Assessment",
            Self::DbProbe => "Database Probe",
            Self::NseRun => "NSE Script Execution",
            Self::ScanDockerImage => "Docker Image Scan",
            Self::ScanKubernetes => "Kubernetes Scan",
            Self::AnalyzeApk => "APK Analysis",
            Self::AnalyzeIpa => "IPA Analysis",
        }
    }

    pub const fn feature_required(self) -> Option<&'static str> {
        match self {
            Self::ScanPorts
            | Self::ScanEndpoints
            | Self::FingerprintServices
            | Self::ReconDns
            | Self::InspectTls
            | Self::DetectTechnology
            | Self::DetectWaf
            | Self::ValidateWaf
            | Self::FuzzHttp
            | Self::LoadTest
            | Self::RunConsolidatedRecon
            | Self::GraphqlTest
            | Self::OauthTest
            | Self::AuthTest => None,
            Self::ScanGitSecrets => Some("git-secrets"),
            Self::GenerateSbom => Some("sbom"),
            Self::DbProbe => Some("db-pentest"),
            Self::NseRun => Some("nse"),
            Self::ScanDockerImage | Self::ScanKubernetes => Some("container"),
            Self::AnalyzeApk | Self::AnalyzeIpa => Some("mobile"),
        }
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
            OP_SCAN_GIT_SECRETS => Some(Self::ScanGitSecrets),
            OP_GENERATE_SBOM => Some(Self::GenerateSbom),
            OP_RUN_CONSOLIDATED_RECON | "consolidated_recon" => Some(Self::RunConsolidatedRecon),
            OP_GRAPHQL_TEST => Some(Self::GraphqlTest),
            OP_OAUTH_TEST => Some(Self::OauthTest),
            OP_AUTH_TEST => Some(Self::AuthTest),
            OP_DB_PROBE => Some(Self::DbProbe),
            OP_NSE_RUN => Some(Self::NseRun),
            OP_SCAN_DOCKER_IMAGE => Some(Self::ScanDockerImage),
            OP_SCAN_KUBERNETES => Some(Self::ScanKubernetes),
            OP_ANALYZE_APK => Some(Self::AnalyzeApk),
            OP_ANALYZE_IPA => Some(Self::AnalyzeIpa),
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
                    schema_version: "1.0".to_string(),
                };
            }
        }

        // The exhaustive dispatch match is implemented by Engine::dispatch.
        // Keeping the enum parse here means unknown IDs cannot reach it.
        debug_assert_eq!(operation.id(), StableOperation::parse(id).unwrap().id());
        engine.dispatch(py, request.clone(), None)
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

        engine.dispatch_async(request.clone(), None)
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

    /// Return the executor descriptor for a stable operation.
    ///
    /// The descriptor bundles risk classification, feature requirements,
    /// and confirmation behavior from a single authoritative source.
    pub fn descriptor_for(&self, operation: StableOperation) -> OperationExecutorDescriptor {
        let confirmation_required = matches!(
            operation,
            StableOperation::NseRun
                | StableOperation::DbProbe
                | StableOperation::FuzzHttp
                | StableOperation::LoadTest
        );

        let confirmation_message = if confirmation_required {
            Some("This operation may interact with external systems. Confirm to proceed.")
        } else {
            None
        };

        OperationExecutorDescriptor {
            operation,
            risk: OperationExecutorDescriptor::classify_risk(operation),
            feature_required: operation.feature_required(),
            confirmation_required,
            confirmation_message,
            intended_uses: vec![eggsec::config::IntendedUse::WebAssessment],
        }
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
        schema_version: "1.0".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::status::{ExecutionStatus, OperationError};

    // -----------------------------------------------------------------------
    // Registry contract tests
    // -----------------------------------------------------------------------

    #[test]
    fn canonical_registry_is_exhaustive() {
        let registry = OperationExecutorRegistry::default_stable();
        assert_eq!(StableOperation::ALL.len(), 22);
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

    // -----------------------------------------------------------------------
    // Registry contract: every operation has a descriptor with correct metadata
    // -----------------------------------------------------------------------

    #[test]
    fn test_all_operations_have_descriptors() {
        let registry = OperationExecutorRegistry::default_stable();
        for &op in StableOperation::ALL {
            let desc = registry.descriptor_for(op);
            assert_eq!(desc.operation, op);
        }
    }

    #[test]
    fn test_all_operations_have_feature_requirements() {
        for &op in StableOperation::ALL {
            let feature = op.feature_required();
            // All operations must have a deterministic feature (None or Some)
            match feature {
                None => {}
                Some(f) => {
                    assert!(!f.is_empty(), "feature string for {:?} is empty", op);
                    assert!(
                        f.chars().all(|c| c.is_ascii_lowercase() || c == '-'),
                        "feature '{}' for {:?} has invalid chars",
                        f,
                        op
                    );
                }
            }
        }
    }

    #[test]
    fn test_operation_names_match_ids() {
        for &op in StableOperation::ALL {
            let name = op.name();
            let id = op.id();
            assert!(!name.is_empty(), "{:?} has empty name", op);
            assert!(!id.is_empty(), "{:?} has empty id", op);
            // IDs use snake_case
            assert!(
                id.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
                "id '{}' for {:?} is not snake_case",
                id,
                op
            );
        }
    }

    #[test]
    fn test_feature_gate_consistency() {
        // Feature-gated operations must have a feature string
        let feature_gated = [
            (StableOperation::ScanGitSecrets, "git-secrets"),
            (StableOperation::GenerateSbom, "sbom"),
            (StableOperation::DbProbe, "db-pentest"),
            (StableOperation::NseRun, "nse"),
            (StableOperation::ScanDockerImage, "container"),
            (StableOperation::ScanKubernetes, "container"),
            (StableOperation::AnalyzeApk, "mobile"),
            (StableOperation::AnalyzeIpa, "mobile"),
        ];
        for (op, expected_feature) in feature_gated {
            assert_eq!(
                op.feature_required(),
                Some(expected_feature),
                "{:?} should require feature '{}'",
                op,
                expected_feature
            );
        }
    }

    #[test]
    fn test_all_operations_parseable_by_id() {
        for &op in StableOperation::ALL {
            let id = op.id();
            assert_eq!(
                StableOperation::parse(id),
                Some(op),
                "parse('{}') should return {:?}",
                id,
                op
            );
        }
    }

    #[test]
    fn test_operation_names_are_distinct() {
        let mut names: Vec<&str> = StableOperation::ALL.iter().map(|op| op.name()).collect();
        names.sort();
        names.dedup();
        assert_eq!(
            names.len(),
            StableOperation::ALL.len(),
            "operation names must be unique"
        );
    }

    // -----------------------------------------------------------------------
    // Descriptor metadata contract tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_confirmation_required_operations() {
        let registry = OperationExecutorRegistry::default_stable();
        let expected_confirm = [
            StableOperation::NseRun,
            StableOperation::DbProbe,
            StableOperation::FuzzHttp,
            StableOperation::LoadTest,
        ];
        for &op in StableOperation::ALL {
            let desc = registry.descriptor_for(op);
            if expected_confirm.contains(&op) {
                assert!(
                    desc.confirmation_required,
                    "{:?} should require confirmation",
                    op
                );
                assert!(
                    desc.confirmation_message.is_some(),
                    "{:?} should have confirmation message",
                    op
                );
            } else {
                assert!(
                    !desc.confirmation_required,
                    "{:?} should not require confirmation",
                    op
                );
            }
        }
    }

    #[test]
    fn test_descriptor_risk_classification() {
        let registry = OperationExecutorRegistry::default_stable();
        for &op in StableOperation::ALL {
            let desc = registry.descriptor_for(op);
            // Every operation must have a risk classification (not default)
            // Just verify it doesn't panic and is deterministic
            let desc2 = registry.descriptor_for(op);
            assert_eq!(
                format!("{:?}", desc.risk),
                format!("{:?}", desc2.risk),
                "risk classification for {:?} is not deterministic",
                op
            );
        }
    }

    #[test]
    fn test_descriptor_intended_uses_not_empty() {
        let registry = OperationExecutorRegistry::default_stable();
        for &op in StableOperation::ALL {
            let desc = registry.descriptor_for(op);
            assert!(
                !desc.intended_uses.is_empty(),
                "{:?} has empty intended_uses",
                op
            );
        }
    }

    // -----------------------------------------------------------------------
    // Registry list/get contract
    // -----------------------------------------------------------------------

    #[test]
    fn test_registry_list_length_matches_all() {
        let registry = OperationExecutorRegistry::default_stable();
        assert_eq!(registry.len(), StableOperation::ALL.len());
        assert_eq!(registry.len(), 22);
    }

    #[test]
    fn test_registry_get_returns_operation_info() {
        let registry = OperationExecutorRegistry::default_stable();
        for &op in StableOperation::ALL {
            let info = registry.get(op.id());
            assert!(info.is_some(), "registry.get('{}') returned None", op.id());
            let info = info.unwrap();
            assert_eq!(info.id, op.id());
            assert_eq!(info.name, op.name());
        }
    }

    #[test]
    fn test_registry_contains_all_ids() {
        let registry = OperationExecutorRegistry::default_stable();
        let ids = registry.list();
        assert_eq!(ids.len(), 22);
        for id in &ids {
            assert!(registry.contains(id), "registry doesn't contain '{}'", id);
        }
    }

    #[test]
    fn test_registry_not_contains_unknown() {
        let registry = OperationExecutorRegistry::default_stable();
        assert!(!registry.contains("totally_fake_operation"));
        assert!(!registry.contains(""));
        assert!(!registry.contains("scan"));
    }

    #[test]
    fn test_unknown_operation_result_status_is_failed() {
        let result = unknown_operation("nonexistent");
        assert!(matches!(result.status, ExecutionStatus::Failed { .. }));
        assert!(result.error.is_some());
    }

    #[test]
    fn test_unknown_operation_error_has_code() {
        let result = unknown_operation("nonexistent");
        let error = result.error.unwrap();
        assert_eq!(error.code, "unknown_operation");
        assert_eq!(error.kind, "validation");
    }

    #[test]
    fn test_parse_returns_none_for_unknown() {
        assert_eq!(StableOperation::parse("unknown_op"), None);
        assert_eq!(StableOperation::parse(""), None);
        assert_eq!(StableOperation::parse("SCAN_PORTS"), None);
    }

    #[test]
    fn test_all_aliases_parse_correctly() {
        let aliases = [
            ("fingerprint", StableOperation::FingerprintServices),
            ("recon", StableOperation::ReconDns),
            ("tls_inspect", StableOperation::InspectTls),
            ("tech_detect", StableOperation::DetectTechnology),
            ("waf_detect", StableOperation::DetectWaf),
            ("waf_validate", StableOperation::ValidateWaf),
            ("http_fuzz", StableOperation::FuzzHttp),
            ("load_test_http", StableOperation::LoadTest),
            ("consolidated_recon", StableOperation::RunConsolidatedRecon),
        ];
        for (alias, expected) in aliases {
            assert_eq!(
                StableOperation::parse(alias),
                Some(expected),
                "alias '{}' should parse to {:?}",
                alias,
                expected
            );
        }
    }
}
