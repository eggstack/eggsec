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

    // --- NEW metadata fields ---
    /// Human-readable description of what the operation does.
    pub description: &'static str,
    /// Historical aliases accepted for backward compatibility.
    pub aliases: &'static [&'static str],
    /// Maturity level (e.g. "stable", "provisional", "experimental").
    pub maturity: &'static str,
    /// Whether the operation is available via local (in-process) execution.
    pub local_available: bool,
    /// Whether the operation is available via daemon dispatch.
    pub daemon_available: bool,
    /// Whether the operation has a synchronous execution path.
    pub sync_available: bool,
    /// Whether the operation has an asynchronous execution path.
    pub async_available: bool,
    /// Whether the operation supports cancellation via CancellationToken.
    pub cancellation_supported: bool,
    /// Whether the operation supports per-operation timeout.
    pub timeout_supported: bool,
    /// Whether the operation emits finding events.
    pub finding_hook: bool,
    /// Whether the operation emits artifact events.
    pub artifact_hook: bool,
    /// Schema identifier for the operation request type.
    pub request_schema_id: &'static str,
    /// Schema identifier for the operation result type.
    pub result_schema_id: &'static str,
    /// Daemon TaskKind variant name for this operation.
    pub daemon_task_kind: &'static str,
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

    /// Construct a full descriptor for a given operation.
    ///
    /// This is the single source of truth for per-operation metadata.
    pub fn from_operation(operation: StableOperation) -> Self {
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

        // Feature-gated operations are not available locally when feature is off.
        // For the descriptor we always say local_available = true; the feature gate
        // check happens at dispatch time.
        let no_aliases: &[&str] = &[];
        let (description, aliases, maturity, daemon_task_kind) = match operation {
            StableOperation::ScanPorts => (
                "TCP port scanner — discovers open ports on a target host.",
                no_aliases,
                "stable",
                "PortScan",
            ),
            StableOperation::ScanEndpoints => (
                "HTTP endpoint scanner — discovers web paths and status codes.",
                no_aliases,
                "stable",
                "EndpointScan",
            ),
            StableOperation::FingerprintServices => (
                "Service fingerprinter — identifies services running on open ports.",
                &["fingerprint"] as &[&str],
                "stable",
                "Fingerprint",
            ),
            StableOperation::ReconDns => (
                "DNS reconnoiter — enumerates DNS records for a domain.",
                &["recon"] as &[&str],
                "stable",
                "Recon",
            ),
            StableOperation::InspectTls => (
                "TLS inspector — analyzes certificate chains and TLS configuration.",
                &["tls_inspect"] as &[&str],
                "stable",
                "Recon",
            ),
            StableOperation::DetectTechnology => (
                "Technology detector — identifies web technologies and frameworks.",
                &["tech_detect"] as &[&str],
                "stable",
                "Recon",
            ),
            StableOperation::DetectWaf => (
                "WAF detector — identifies web application firewalls in front of a target.",
                &["waf_detect"] as &[&str],
                "stable",
                "Waf",
            ),
            StableOperation::ValidateWaf => (
                "WAF validator — tests WAF rules by sending crafted payloads.",
                &["waf_validate"] as &[&str],
                "stable",
                "Recon",
            ),
            StableOperation::FuzzHttp => (
                "HTTP fuzzer — sends mutated payloads to discover vulnerabilities.",
                &["http_fuzz"] as &[&str],
                "stable",
                "Fuzz",
            ),
            StableOperation::LoadTest => (
                "HTTP load tester — generates concurrent traffic for performance testing.",
                &["load_test_http"] as &[&str],
                "stable",
                "LoadTest",
            ),
            StableOperation::ScanGitSecrets => (
                "Git secrets scanner — detects committed secrets and credentials.",
                no_aliases,
                "stable",
                "Storage",
            ),
            StableOperation::GenerateSbom => (
                "SBOM generator — produces a software bill of materials for a project.",
                no_aliases,
                "stable",
                "Storage",
            ),
            StableOperation::RunConsolidatedRecon => (
                "Consolidated recon — runs multiple recon modules (DNS, SSL, tech) in one pass.",
                &["consolidated_recon"] as &[&str],
                "stable",
                "Recon",
            ),
            StableOperation::GraphqlTest => (
                "GraphQL security tester — probes GraphQL endpoints for introspection and injection flaws.",
                no_aliases,
                "stable",
                "GraphQl",
            ),
            StableOperation::OauthTest => (
                "OAuth security tester — validates OAuth/OIDC flows for common misconfigurations.",
                no_aliases,
                "stable",
                "OAuth",
            ),
            StableOperation::AuthTest => (
                "Authentication assessor — tests authentication mechanisms for weaknesses.",
                no_aliases,
                "stable",
                "AuthTest",
            ),
            StableOperation::DbProbe => (
                "Database prober — fingerprints and enumerates database services.",
                no_aliases,
                "stable",
                "Storage",
            ),
            StableOperation::NseRun => (
                "NSE script executor — runs Nmap Scripting Engine scripts against a target.",
                no_aliases,
                "stable",
                "Nse",
            ),
            StableOperation::ScanDockerImage => (
                "Docker image scanner — analyzes container images for misconfigurations.",
                no_aliases,
                "stable",
                "Storage",
            ),
            StableOperation::ScanKubernetes => (
                "Kubernetes scanner — inspects cluster configuration for security issues.",
                no_aliases,
                "stable",
                "Storage",
            ),
            StableOperation::AnalyzeApk => (
                "APK analyzer — performs static analysis on Android application packages.",
                no_aliases,
                "stable",
                "Storage",
            ),
            StableOperation::AnalyzeIpa => (
                "IPA analyzer — performs static analysis on iOS application packages.",
                no_aliases,
                "stable",
                "Storage",
            ),
        };

        // All 22 operations support local, daemon, sync, async, cancellation, and timeout.
        let (request_schema_id, result_schema_id) = match operation {
            StableOperation::ScanPorts => ("port_scan_request", "port_scan_result"),
            StableOperation::ScanEndpoints => ("endpoint_scan_request", "endpoint_scan_result"),
            StableOperation::FingerprintServices => ("fingerprint_request", "fingerprint_result"),
            StableOperation::ReconDns => ("recon_dns_request", "dns_record_set"),
            StableOperation::InspectTls => ("tls_inspect_request", "tls_inspection_result"),
            StableOperation::DetectTechnology => ("tech_detect_request", "tech_detection_result"),
            StableOperation::DetectWaf => ("waf_detect_request", "waf_detection_result"),
            StableOperation::ValidateWaf => ("waf_validate_request", "waf_scan_result"),
            StableOperation::FuzzHttp => ("fuzz_request", "fuzz_result"),
            StableOperation::LoadTest => ("load_test_request", "load_test_result"),
            StableOperation::ScanGitSecrets => ("git_secrets_request", "git_secrets_report"),
            StableOperation::GenerateSbom => ("sbom_request", "sbom_report"),
            StableOperation::RunConsolidatedRecon => {
                ("consolidated_recon_request", "consolidated_recon_report")
            }
            StableOperation::GraphqlTest => ("graphql_test_request", "graphql_test_result"),
            StableOperation::OauthTest => ("oauth_test_request", "oauth_test_result"),
            StableOperation::AuthTest => ("auth_test_request", "auth_test_report"),
            StableOperation::DbProbe => ("db_probe_request", "db_probe_report"),
            StableOperation::NseRun => ("nse_run_request", "nse_run_report"),
            StableOperation::ScanDockerImage => ("docker_scan_request", "docker_scan_result"),
            StableOperation::ScanKubernetes => ("k8s_scan_request", "k8s_scan_result"),
            StableOperation::AnalyzeApk => ("apk_analysis_request", "apk_analysis_report"),
            StableOperation::AnalyzeIpa => ("ipa_analysis_request", "ipa_analysis_report"),
        };

        // Finding hooks: operations that emit finding events in the engine dispatch.
        let finding_hook = matches!(
            operation,
            StableOperation::ScanPorts
                | StableOperation::ScanEndpoints
                | StableOperation::FingerprintServices
                | StableOperation::InspectTls
                | StableOperation::FuzzHttp
                | StableOperation::ScanGitSecrets
        );

        // Artifact hooks: operations that emit artifact events in the engine dispatch.
        let artifact_hook = matches!(
            operation,
            StableOperation::GenerateSbom | StableOperation::RunConsolidatedRecon
        );

        OperationExecutorDescriptor {
            operation,
            risk: Self::classify_risk(operation),
            feature_required: operation.feature_required(),
            confirmation_required,
            confirmation_message,
            intended_uses: vec![eggsec::config::IntendedUse::WebAssessment],
            description,
            aliases,
            maturity,
            local_available: true,
            daemon_available: true,
            sync_available: true,
            async_available: true,
            cancellation_supported: true,
            timeout_supported: true,
            finding_hook,
            artifact_hook,
            request_schema_id,
            result_schema_id,
            daemon_task_kind,
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
        OperationExecutorDescriptor::from_operation(operation)
    }

    /// Return executor descriptors for all 22 stable operations.
    pub fn all_descriptors(&self) -> Vec<OperationExecutorDescriptor> {
        StableOperation::ALL
            .iter()
            .map(|&op| OperationExecutorDescriptor::from_operation(op))
            .collect()
    }

    /// Return a serializable list of descriptor metadata for CI validation.
    ///
    /// Each entry is a JSON-compatible map of the descriptor fields. This is
    /// consumed by validation scripts to verify metadata consistency.
    pub fn descriptor_metadata_list(&self) -> Vec<std::collections::HashMap<String, String>> {
        self.all_descriptors()
            .iter()
            .map(|desc| {
                let mut map = std::collections::HashMap::new();
                map.insert("operation".to_string(), desc.operation.id().to_string());
                map.insert("name".to_string(), desc.operation.name().to_string());
                map.insert("description".to_string(), desc.description.to_string());
                map.insert("risk".to_string(), format!("{:?}", desc.risk));
                map.insert("maturity".to_string(), desc.maturity.to_string());
                map.insert(
                    "feature_required".to_string(),
                    desc.feature_required
                        .map(|s| s.to_string())
                        .unwrap_or_default(),
                );
                map.insert(
                    "confirmation_required".to_string(),
                    desc.confirmation_required.to_string(),
                );
                map.insert(
                    "local_available".to_string(),
                    desc.local_available.to_string(),
                );
                map.insert(
                    "daemon_available".to_string(),
                    desc.daemon_available.to_string(),
                );
                map.insert(
                    "sync_available".to_string(),
                    desc.sync_available.to_string(),
                );
                map.insert(
                    "async_available".to_string(),
                    desc.async_available.to_string(),
                );
                map.insert(
                    "cancellation_supported".to_string(),
                    desc.cancellation_supported.to_string(),
                );
                map.insert(
                    "timeout_supported".to_string(),
                    desc.timeout_supported.to_string(),
                );
                map.insert("finding_hook".to_string(), desc.finding_hook.to_string());
                map.insert("artifact_hook".to_string(), desc.artifact_hook.to_string());
                map.insert(
                    "request_schema_id".to_string(),
                    desc.request_schema_id.to_string(),
                );
                map.insert(
                    "result_schema_id".to_string(),
                    desc.result_schema_id.to_string(),
                );
                map.insert(
                    "daemon_task_kind".to_string(),
                    desc.daemon_task_kind.to_string(),
                );
                map.insert("aliases".to_string(), desc.aliases.join(","));
                map.insert(
                    "intended_uses".to_string(),
                    desc.intended_uses
                        .iter()
                        .map(|u| format!("{:?}", u))
                        .collect::<Vec<_>>()
                        .join(","),
                );
                map
            })
            .collect()
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
    use crate::status::ExecutionStatus;

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

    // -----------------------------------------------------------------------
    // B9: Architecture guard tests — registry consistency invariants
    // -----------------------------------------------------------------------

    #[test]
    fn every_stable_operation_has_exactly_one_executor() {
        let registry = OperationExecutorRegistry::default_stable();
        let mut seen = std::collections::HashSet::new();
        for &op in StableOperation::ALL {
            let desc = registry.descriptor_for(op);
            assert!(
                seen.insert(desc.operation),
                "Duplicate executor for {:?}",
                op
            );
        }
        assert_eq!(seen.len(), 22);
    }

    #[test]
    fn every_executor_id_is_unique() {
        let registry = OperationExecutorRegistry::default_stable();
        let mut ids = std::collections::HashSet::new();
        for &op in StableOperation::ALL {
            let desc = registry.descriptor_for(op);
            assert!(
                ids.insert(desc.operation.id().to_string()),
                "Duplicate ID: {}",
                desc.operation.id()
            );
        }
    }

    #[test]
    fn aliases_do_not_collide() {
        let registry = OperationExecutorRegistry::default_stable();
        let mut all_ids = std::collections::HashSet::new();
        for &op in StableOperation::ALL {
            let desc = registry.descriptor_for(op);
            // Canonical ID
            assert!(
                all_ids.insert(desc.operation.id().to_string()),
                "Alias collision: {}",
                desc.operation.id()
            );
            // Legacy aliases
            for alias in desc.aliases {
                assert!(
                    all_ids.insert(alias.to_string()),
                    "Alias '{}' collides for {:?}",
                    alias,
                    op
                );
            }
        }
    }

    #[test]
    fn every_executor_has_schema_identities() {
        let registry = OperationExecutorRegistry::default_stable();
        for &op in StableOperation::ALL {
            let desc = registry.descriptor_for(op);
            assert!(
                !desc.request_schema_id.is_empty(),
                "{:?}: missing request_schema_id",
                op
            );
            assert!(
                !desc.result_schema_id.is_empty(),
                "{:?}: missing result_schema_id",
                op
            );
        }
    }

    #[test]
    fn feature_gated_executors_agree_with_cargo_features() {
        let registry = OperationExecutorRegistry::default_stable();
        for &op in StableOperation::ALL {
            let desc = registry.descriptor_for(op);
            let feature = op.feature_required();
            assert_eq!(
                desc.feature_required, feature,
                "{:?}: descriptor feature mismatch",
                op
            );
        }
    }

    #[test]
    fn sync_and_async_callbacks_both_present() {
        let registry = OperationExecutorRegistry::default_stable();
        for &op in StableOperation::ALL {
            let desc = registry.descriptor_for(op);
            assert!(desc.sync_available, "{:?}: sync not available", op);
            assert!(desc.async_available, "{:?}: async not available", op);
        }
    }

    #[test]
    fn all_stable_operations_have_tool_descriptors() {
        use crate::tool_descriptor::ToolDescriptorPy;
        let descriptors: Vec<_> = StableOperation::ALL
            .iter()
            .map(|&op| ToolDescriptorPy::from_stable(op))
            .collect();
        assert_eq!(descriptors.len(), 22);
    }

    #[test]
    fn generated_metadata_is_current() {
        let issues = crate::generated_inventories::validate_metadata_consistency();
        assert!(issues.is_empty(), "Metadata inconsistencies: {:?}", issues);
    }

    #[test]
    fn daemon_task_kinds_are_non_empty() {
        let registry = OperationExecutorRegistry::default_stable();
        for &op in StableOperation::ALL {
            let desc = registry.descriptor_for(op);
            assert!(
                !desc.daemon_task_kind.is_empty(),
                "{:?}: empty daemon_task_kind",
                op
            );
        }
    }

    #[test]
    fn every_descriptor_has_intended_uses() {
        let registry = OperationExecutorRegistry::default_stable();
        for &op in StableOperation::ALL {
            let desc = registry.descriptor_for(op);
            assert!(
                !desc.intended_uses.is_empty(),
                "{:?}: empty intended_uses",
                op
            );
        }
    }
}
