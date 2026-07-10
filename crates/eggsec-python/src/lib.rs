mod artifact;
mod async_client;
mod async_engine;
mod audit;
mod auth_assess;
mod authorization;
mod baseline;
mod cancellation;
mod checkpoint;
mod client;
mod config_model;
#[cfg(feature = "container")]
mod container;
mod cvss;
#[cfg(feature = "daemon-client")]
mod daemon;
#[cfg(feature = "db-pentest")]
mod db_pentest;
mod dto;
mod endpoint;
mod engine;
mod error;
mod execution_context;
mod features;
mod finding;
mod finding_schema;
mod finding_workflow;
mod fingerprint;
#[cfg(feature = "git-secrets")]
mod git_secrets;
mod graphql;
mod handles;
mod loadtest;
#[cfg(feature = "mobile")]
mod mobile;
#[cfg(feature = "nse")]
mod nse;
mod oauth;
mod operation_metadata;
#[cfg(feature = "packet-inspection")]
mod packet_inspection;
mod pipeline;
mod planning;
mod preflight;
#[cfg(feature = "web-proxy")]
mod proxy;
mod recon;
mod reporters;
mod repository;
mod requests;
mod runtime_async;
mod runtime_sync;
#[cfg(feature = "sbom")]
mod sbom;
mod scanner;
mod scope;
mod scope_eval;
mod status;
#[cfg(feature = "stress-testing")]
mod stress;
mod version;
mod waf;
mod waf_validation;

#[cfg(feature = "headless-browser")]
mod browser_assess;
#[cfg(feature = "compliance")]
mod compliance;
mod consolidated_recon;
#[cfg(feature = "advanced-hunting")]
mod hunt;
mod integrations;
mod migration;

pub use error::*;
use pyo3::prelude::*;

/// The eggsec Python module.
///
/// Python bindings for the Eggsec security assessment engine.
/// This is a host-language binding over the Rust engine, not an internal plugin runtime.
#[pymodule]
pub fn _core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add("__version_info__", (0, 1, 0))?;

    // Exceptions
    m.add("EggsecError", m.py().get_type_bound::<EggsecError>())?;
    m.add("ConfigError", m.py().get_type_bound::<ConfigError>())?;
    m.add("ScopeError", m.py().get_type_bound::<ScopeError>())?;
    m.add(
        "EnforcementError",
        m.py().get_type_bound::<EnforcementError>(),
    )?;
    m.add("NetworkError", m.py().get_type_bound::<NetworkError>())?;
    m.add("ScanError", m.py().get_type_bound::<ScanError>())?;
    m.add("TimeoutError", m.py().get_type_bound::<TimeoutError>())?;
    m.add(
        "FeatureUnavailableError",
        m.py().get_type_bound::<FeatureUnavailableError>(),
    )?;
    m.add(
        "SerializationError",
        m.py().get_type_bound::<SerializationError>(),
    )?;
    m.add("InternalError", m.py().get_type_bound::<InternalError>())?;

    // Classes
    m.add_class::<config_model::PySensitiveString>()?;
    m.add_class::<config_model::PyHttpConfig>()?;
    m.add_class::<config_model::PyScanConfig>()?;
    m.add_class::<config_model::PyOutputConfig>()?;
    m.add_class::<config_model::PyReconApiConfig>()?;
    m.add_class::<config_model::PyReconConfig>()?;
    m.add_class::<config_model::PyProxyConfigEntry>()?;
    m.add_class::<config_model::PyAllowedWorker>()?;
    m.add_class::<config_model::PyRemoteConfig>()?;
    m.add_class::<config_model::PyAiConfig>()?;
    m.add_class::<config_model::PySearchConfig>()?;
    m.add_class::<config_model::PyPathsConfig>()?;
    m.add_class::<config_model::PyCacheConfig>()?;
    m.add_class::<config_model::PyAlertChannelConfig>()?;
    m.add_class::<config_model::PyEggsecConfig>()?;
    m.add_class::<scope::Scope>()?;
    m.add_class::<scope_eval::ScopeSourcePy>()?;
    m.add_class::<scope_eval::LoadedScopePy>()?;
    m.add_class::<scope_eval::ScopeRulePy>()?;
    m.add_class::<scope_eval::ScopeExplanationPy>()?;
    m.add_class::<scope_eval::ScopeValidationPy>()?;
    // Operation metadata and capabilities
    m.add_class::<operation_metadata::OperationRiskPy>()?;
    m.add_class::<operation_metadata::OperationModePy>()?;
    m.add_class::<operation_metadata::IntendedUsePy>()?;
    m.add_class::<operation_metadata::CapabilityPy>()?;
    m.add_class::<operation_metadata::DenialClassPy>()?;
    m.add_class::<operation_metadata::TargetPolicyKindPy>()?;
    m.add_class::<operation_metadata::OperationDescriptorPy>()?;
    m.add_class::<operation_metadata::OperationMetadataViewPy>()?;
    m.add_class::<operation_metadata::OperationRegistry>()?;
    m.add_class::<client::Client>()?;
    m.add_class::<async_client::AsyncClient>()?;
    m.add_class::<engine::Engine>()?;
    m.add_class::<async_engine::AsyncEngine>()?;
    m.add_class::<handles::ExecutionHandle>()?;
    m.add_class::<handles::ExecutionEvent>()?;
    m.add_class::<handles::EventLog>()?;
    m.add_class::<cancellation::CancellationToken>()?;
    m.add_class::<runtime_async::PyFuture>()?;
    m.add_class::<dto::PortScanResult>()?;
    m.add_class::<dto::OpenPort>()?;
    m.add_class::<dto::ScanStats>()?;
    m.add_class::<dto::PortRange>()?;
    m.add_class::<dto::TimingPreset>()?;
    m.add_class::<endpoint::EndpointScanConfig>()?;
    m.add_class::<endpoint::EndpointFinding>()?;
    m.add_class::<endpoint::EndpointScanStats>()?;
    m.add_class::<endpoint::EndpointScanResult>()?;
    m.add_class::<fingerprint::FingerprintEvidence>()?;
    m.add_class::<fingerprint::FingerprintConfidence>()?;
    m.add_class::<fingerprint::ServiceFingerprintResult>()?;
    m.add_class::<fingerprint::FingerprintScanResult>()?;
    // Phase D: Findings and reporting
    m.add_class::<finding::Severity>()?;
    m.add_class::<finding::Evidence>()?;
    m.add_class::<finding::Finding>()?;
    m.add_class::<finding::FindingSet>()?;
    m.add_class::<finding::Report>()?;
    // E5: Repository abstraction
    m.add_class::<finding_schema::ConfidencePy>()?;
    m.add_class::<finding_schema::FindingTypePy>()?;
    m.add_class::<finding_schema::EvidenceKindPy>()?;
    m.add_class::<finding_schema::AffectedAssetPy>()?;
    m.add_class::<finding_schema::FindingLocationPy>()?;
    m.add_class::<finding_schema::VersionedEvidencePy>()?;
    m.add_class::<finding_schema::VersionedFindingPy>()?;
    m.add(
        "FINDING_SCHEMA_VERSION",
        finding_schema::FINDING_SCHEMA_VERSION,
    )?;
    // E2: Artifacts
    m.add_class::<artifact::ArtifactPy>()?;
    m.add_class::<artifact::ArtifactReferencePy>()?;
    m.add_class::<artifact::ArtifactStorePy>()?;
    // E3: CVSS and vulnerability records
    m.add_class::<cvss::CvssScorePy>()?;
    m.add_class::<cvss::VulnerabilityRecordPy>()?;
    m.add_class::<cvss::RemediationRecordPy>()?;
    // E4: Finding workflow
    m.add_class::<finding_workflow::FindingStatePy>()?;
    m.add_class::<finding_workflow::WorkflowTransitionPy>()?;
    m.add_class::<finding_workflow::SuppressionPy>()?;
    m.add_class::<finding_workflow::FindingWorkflowPy>()?;
    m.add_class::<repository::FindingRepositoryPy>()?;
    m.add_class::<repository::AssessmentPy>()?;
    m.add_class::<repository::AssessmentRepositoryPy>()?;
    // E6: Baselines and comparisons
    m.add_class::<baseline::FindingCorrelationPy>()?;
    m.add_class::<baseline::FindingDiffPy>()?;
    m.add_class::<baseline::AssessmentDiffPy>()?;
    m.add_class::<baseline::BaselineComparatorPy>()?;
    // E7: Reporting
    m.add_class::<reporters::FindingReporterPy>()?;
    m.add_class::<reporters::SeveritySummaryPy>()?;
    m.add_class::<reporters::ReportEnvelopePy>()?;
    // Phase A3: Common result protocol types
    m.add_class::<status::ExecutionStatus>()?;
    m.add_class::<status::ExecutionStats>()?;
    m.add_class::<status::Artifact>()?;
    m.add_class::<status::OperationResult>()?;
    // Phase D: Recon
    m.add_class::<recon::DnsRecordSet>()?;
    m.add_class::<recon::MxRecord>()?;
    m.add_class::<recon::SoaRecord>()?;
    m.add_class::<recon::TlsCertificateInfo>()?;
    m.add_class::<recon::TlsInspectionResult>()?;
    m.add_class::<recon::SslIssue>()?;
    m.add_class::<recon::TechStack>()?;
    m.add_class::<recon::TechDetectionResult>()?;
    // Phase D: WAF detection
    m.add_class::<waf::WafDetectionResultPy>()?;
    // Operation request types
    m.add_class::<requests::OperationRequest>()?;
    m.add_class::<requests::PortScanRequest>()?;
    m.add_class::<requests::EndpointScanRequest>()?;
    m.add_class::<requests::FingerprintRequest>()?;
    m.add_class::<requests::ReconDnsRequest>()?;
    m.add_class::<requests::TlsInspectRequest>()?;
    m.add_class::<requests::TechDetectRequest>()?;
    m.add_class::<requests::WafDetectRequest>()?;
    m.add_class::<requests::LoadTestRequest>()?;
    m.add_class::<requests::WafValidateRequest>()?;
    m.add_class::<requests::FuzzRequest>()?;
    m.add_class::<requests::RequestBuilder>()?;
    // Pipeline and assessment types
    m.add_class::<pipeline::PipelineStep>()?;
    m.add_class::<pipeline::StepResult>()?;
    m.add_class::<pipeline::PipelineResult>()?;
    m.add_class::<pipeline::Pipeline>()?;
    m.add_class::<pipeline::AsyncPipeline>()?;
    // Planning types
    m.add_class::<planning::PlanStep>()?;
    m.add_class::<planning::ScanPlan>()?;
    // Checkpoint types
    m.add_class::<checkpoint::Checkpoint>()?;
    m.add_class::<checkpoint::CheckpointStore>()?;
    // Phase F Track 1: WAF validation and HTTP fuzzing
    m.add_class::<waf_validation::BypassResultPy>()?;
    m.add_class::<waf_validation::WafScanResultPy>()?;
    m.add_class::<waf_validation::PayloadPy>()?;
    m.add_class::<waf_validation::FuzzResultPy>()?;
    m.add_class::<waf_validation::FuzzSessionPy>()?;
    m.add_class::<waf_validation::FuzzConfig>()?;
    // Phase F Track 4: Git secrets
    #[cfg(feature = "git-secrets")]
    {
        m.add_class::<git_secrets::Confidence>()?;
        m.add_class::<git_secrets::SecretType>()?;
        m.add_class::<git_secrets::SecretFindingPy>()?;
        m.add_class::<git_secrets::GitSecretFindingPy>()?;
        m.add_class::<git_secrets::GitSecretsSummaryPy>()?;
        m.add_class::<git_secrets::GitSecretsReportPy>()?;
    }
    // Phase F Track 5: SBOM
    #[cfg(feature = "sbom")]
    {
        m.add_class::<sbom::SbomFormatPy>()?;
        m.add_class::<sbom::SbomComponentPy>()?;
        m.add_class::<sbom::SbomVulnerabilityPy>()?;
        m.add_class::<sbom::SbomReportPy>()?;
    }
    // Phase F Track 8: Mobile lab
    #[cfg(feature = "mobile")]
    {
        m.add_class::<mobile::MobilePlatformPy>()?;
        m.add_class::<mobile::MobileFindingPy>()?;
        m.add_class::<mobile::MobileScanReportPy>()?;
        // D5: Mobile dynamic
        m.add_class::<mobile::MobileDevicePy>()?;
        m.add_class::<mobile::DynamicMobileConfigPy>()?;
        m.add_class::<mobile::DynamicMobileReportPy>()?;
    }
    // Phase F Track 6: Database pentesting
    #[cfg(feature = "db-pentest")]
    {
        m.add_class::<db_pentest::DbFindingPy>()?;
        m.add_class::<db_pentest::DbPentestReportPy>()?;
        m.add_class::<db_pentest::DbPentestConfig>()?;
        // D7: Database extensibility
        m.add_class::<db_pentest::DbDriverInfoPy>()?;
        m.add_class::<db_pentest::DbCapabilityPy>()?;
        m.add_class::<db_pentest::DbCredentialProviderPy>()?;
        m.add_class::<db_pentest::DbSessionConfigPy>()?;
    }
    // Phase F Track 9: Container security
    #[cfg(feature = "container")]
    {
        m.add_class::<container::ContainerScanTypePy>()?;
        m.add_class::<container::EscapeRiskLevelPy>()?;
        m.add_class::<container::CisCheckStatusPy>()?;
        m.add_class::<container::ImageLayerPy>()?;
        m.add_class::<container::DockerMisconfigPy>()?;
        m.add_class::<container::DockerScanResultPy>()?;
        m.add_class::<container::ClusterInfoPy>()?;
        m.add_class::<container::K8sFindingPy>()?;
        m.add_class::<container::KubernetesScanResultPy>()?;
        m.add_class::<container::EscapeRiskPy>()?;
        m.add_class::<container::EscapeDetectionResultPy>()?;
        m.add_class::<container::CisCheckPy>()?;
        m.add_class::<container::CisBenchmarkResultPy>()?;
        m.add_class::<container::ContainerFindingPy>()?;
        m.add_class::<container::ContainerReportPy>()?;
    }
    // Phase F Track 10: Packet inspection
    #[cfg(feature = "packet-inspection")]
    {
        m.add_class::<packet_inspection::CaptureConfigPy>()?;
        m.add_class::<packet_inspection::CaptureStatsPy>()?;
        m.add_class::<packet_inspection::PacketInfoPy>()?;
        m.add_class::<packet_inspection::NetworkInterfaceInfoPy>()?;
        m.add_class::<packet_inspection::PcapWriterPy>()?;
        // D2: Live packet inspection
        m.add_class::<packet_inspection::PacketFilterPy>()?;
        m.add_class::<packet_inspection::FlowRecordPy>()?;
        m.add_class::<packet_inspection::LiveCaptureResultPy>()?;
        // D3: Network probing
        m.add_class::<packet_inspection::TracerouteConfigPy>()?;
        m.add_class::<packet_inspection::TracerouteHopPy>()?;
        m.add_class::<packet_inspection::TracerouteResultPy>()?;
    }
    // Phase F Track 2: Load testing
    m.add_class::<loadtest::LoadTestResultPy>()?;
    m.add_class::<loadtest::LoadTestConfig>()?;
    // Phase F Track 11: Stress testing
    #[cfg(feature = "stress-testing")]
    {
        m.add_class::<stress::StressTypePy>()?;
        m.add_class::<stress::StressConfigPy>()?;
        m.add_class::<stress::StressStatsPy>()?;
        m.add_class::<stress::StressConfigSummaryPy>()?;
        m.add_class::<stress::StressResultPy>()?;
    }
    // Phase F Track 12: NSE bindings
    #[cfg(feature = "nse")]
    {
        m.add_class::<nse::NseConfigPy>()?;
        m.add_class::<nse::NseLibraryUsePy>()?;
        m.add_class::<nse::NseRuleEvaluationPy>()?;
        m.add_class::<nse::NseReportPy>()?;
        // D1: NSE runtime completion
        m.add_class::<nse::NseScriptMetadataPy>()?;
        m.add_class::<nse::NseSandboxPolicyPy>()?;
        m.add_class::<nse::NseTargetContextPy>()?;
    }
    // Phase F Track 7: Proxy and web proxy
    #[cfg(feature = "web-proxy")]
    {
        m.add_class::<proxy::ProxyTypePy>()?;
        m.add_class::<proxy::RotationStrategyPy>()?;
        m.add_class::<proxy::ProxyConfigPy>()?;
        m.add_class::<proxy::ProxyEntryPy>()?;
        m.add_class::<proxy::ProxyManagerPy>()?;
        m.add_class::<proxy::HealthCheckResultPy>()?;
        m.add_class::<proxy::ProxyHealthPy>()?;
        // D4: Interception proxy
        m.add_class::<proxy::InterceptConfigPy>()?;
        m.add_class::<proxy::CapturedExchangePy>()?;
        m.add_class::<proxy::InterceptSessionResultPy>()?;
    }

    // Functions
    m.add_function(wrap_pyfunction!(scope_eval::validate_scope, m)?)?;
    m.add_function(wrap_pyfunction!(features::features, m)?)?;
    m.add_function(wrap_pyfunction!(features::has_feature, m)?)?;
    m.add_function(wrap_pyfunction!(version::build_info, m)?)?;
    m.add_function(wrap_pyfunction!(scanner::scan_ports, m)?)?;
    m.add_function(wrap_pyfunction!(scanner::async_scan_ports, m)?)?;
    m.add_function(wrap_pyfunction!(scanner::scan_endpoints, m)?)?;
    m.add_function(wrap_pyfunction!(scanner::async_scan_endpoints, m)?)?;
    m.add_function(wrap_pyfunction!(scanner::fingerprint_services, m)?)?;
    m.add_function(wrap_pyfunction!(scanner::async_fingerprint_services, m)?)?;
    // Phase D: Recon functions
    m.add_function(wrap_pyfunction!(recon::recon_dns, m)?)?;
    m.add_function(wrap_pyfunction!(recon::async_recon_dns, m)?)?;
    m.add_function(wrap_pyfunction!(recon::inspect_tls, m)?)?;
    m.add_function(wrap_pyfunction!(recon::async_inspect_tls, m)?)?;
    m.add_function(wrap_pyfunction!(recon::detect_technology, m)?)?;
    m.add_function(wrap_pyfunction!(recon::async_detect_technology, m)?)?;
    // Phase D: WAF functions
    m.add_function(wrap_pyfunction!(waf::detect_waf, m)?)?;
    m.add_function(wrap_pyfunction!(waf::async_detect_waf, m)?)?;
    // Phase F Track 1: WAF validation and HTTP fuzzing functions
    m.add_function(wrap_pyfunction!(waf_validation::validate_waf, m)?)?;
    m.add_function(wrap_pyfunction!(waf_validation::async_validate_waf, m)?)?;
    m.add_function(wrap_pyfunction!(waf_validation::fuzz_http, m)?)?;
    m.add_function(wrap_pyfunction!(waf_validation::async_fuzz_http, m)?)?;
    m.add_function(wrap_pyfunction!(waf_validation::generate_fuzz_payloads, m)?)?;
    // Phase F Track 4: Git secrets functions
    #[cfg(feature = "git-secrets")]
    {
        m.add_function(wrap_pyfunction!(git_secrets::scan_git_secrets, m)?)?;
        m.add_function(wrap_pyfunction!(git_secrets::async_scan_git_secrets, m)?)?;
    }
    // Phase F Track 5: SBOM functions
    #[cfg(feature = "sbom")]
    {
        m.add_function(wrap_pyfunction!(sbom::generate_sbom, m)?)?;
        m.add_function(wrap_pyfunction!(sbom::async_generate_sbom, m)?)?;
    }
    // Phase F Track 8: Mobile functions
    #[cfg(feature = "mobile")]
    {
        m.add_function(wrap_pyfunction!(mobile::analyze_apk, m)?)?;
        m.add_function(wrap_pyfunction!(mobile::async_analyze_apk, m)?)?;
        m.add_function(wrap_pyfunction!(mobile::analyze_ipa, m)?)?;
        m.add_function(wrap_pyfunction!(mobile::async_analyze_ipa, m)?)?;
        // D5: Mobile dynamic functions
        m.add_function(wrap_pyfunction!(mobile::list_mobile_devices, m)?)?;
        m.add_function(wrap_pyfunction!(mobile::dynamic_mobile_analysis, m)?)?;
    }
    // Phase F Track 6: Database pentesting functions
    #[cfg(feature = "db-pentest")]
    {
        m.add_function(wrap_pyfunction!(db_pentest::db_probe, m)?)?;
        m.add_function(wrap_pyfunction!(db_pentest::async_db_probe, m)?)?;
        m.add_function(wrap_pyfunction!(db_pentest::db_probe_with_config, m)?)?;
        m.add_function(wrap_pyfunction!(db_pentest::db_probe_postgres, m)?)?;
        m.add_function(wrap_pyfunction!(db_pentest::db_probe_mysql, m)?)?;
        m.add_function(wrap_pyfunction!(db_pentest::db_probe_mssql, m)?)?;
        m.add_function(wrap_pyfunction!(db_pentest::db_probe_mongodb, m)?)?;
        m.add_function(wrap_pyfunction!(db_pentest::db_probe_redis, m)?)?;
        // D7: Database extensibility functions
        m.add_function(wrap_pyfunction!(db_pentest::db_list_drivers, m)?)?;
        m.add_function(wrap_pyfunction!(db_pentest::db_get_capabilities, m)?)?;
        m.add_function(wrap_pyfunction!(db_pentest::db_run_with_config, m)?)?;
    }
    // Phase F Track 9: Container functions
    #[cfg(feature = "container")]
    {
        m.add_function(wrap_pyfunction!(container::scan_docker_image, m)?)?;
        m.add_function(wrap_pyfunction!(container::async_scan_docker_image, m)?)?;
        m.add_function(wrap_pyfunction!(container::scan_kubernetes, m)?)?;
        m.add_function(wrap_pyfunction!(container::async_scan_kubernetes, m)?)?;
        m.add_function(wrap_pyfunction!(container::detect_escape_risks, m)?)?;
        m.add_function(wrap_pyfunction!(container::check_cis_docker_benchmark, m)?)?;
    }
    // Phase F Track 10: Packet inspection functions
    #[cfg(feature = "packet-inspection")]
    {
        m.add_function(wrap_pyfunction!(
            packet_inspection::list_network_interfaces,
            m
        )?)?;
        m.add_function(wrap_pyfunction!(packet_inspection::parse_pcap, m)?)?;
        // D3: Network probing functions
        m.add_function(wrap_pyfunction!(packet_inspection::run_traceroute, m)?)?;
        m.add_function(wrap_pyfunction!(
            packet_inspection::async_run_traceroute,
            m
        )?)?;
        m.add_function(wrap_pyfunction!(packet_inspection::traceroute, m)?)?;
    }
    // Phase F Track 2: Load testing functions
    m.add_function(wrap_pyfunction!(loadtest::load_test_http, m)?)?;
    m.add_function(wrap_pyfunction!(loadtest::async_load_test_http, m)?)?;
    // Phase F Track 11: Stress testing functions
    #[cfg(feature = "stress-testing")]
    {
        m.add_function(wrap_pyfunction!(stress::stress_test, m)?)?;
        m.add_function(wrap_pyfunction!(stress::async_stress_test, m)?)?;
    }
    // Phase F Track 12: NSE functions
    #[cfg(feature = "nse")]
    {
        m.add_function(wrap_pyfunction!(nse::nse_run, m)?)?;
        m.add_function(wrap_pyfunction!(nse::async_nse_run, m)?)?;
        m.add_function(wrap_pyfunction!(nse::nse_list_libraries, m)?)?;
        // D1: NSE runtime completion functions
        m.add_function(wrap_pyfunction!(nse::nse_list_scripts, m)?)?;
        m.add_function(wrap_pyfunction!(nse::nse_get_script_metadata, m)?)?;
    }
    // Phase F Track 7: Proxy and web proxy functions
    #[cfg(feature = "web-proxy")]
    {
        m.add_function(wrap_pyfunction!(proxy::create_proxy_manager, m)?)?;
        m.add_function(wrap_pyfunction!(proxy::async_add_proxy, m)?)?;
        m.add_function(wrap_pyfunction!(proxy::async_proxy_health_check, m)?)?;
    }
    // Phase F Track 13: Daemon client
    #[cfg(feature = "daemon-client")]
    {
        m.add_class::<daemon::DaemonResponsePy>()?;
        m.add_class::<daemon::DaemonClientPy>()?;
        // D6: Daemon task API
        m.add_class::<daemon::DaemonCapabilitiesPy>()?;
        m.add_class::<daemon::TaskHandlePy>()?;
        m.add_class::<daemon::TaskStatusPy>()?;
        m.add_class::<daemon::DaemonEventPy>()?;
        m.add_class::<daemon::SessionSummaryPy>()?;
        m.add_class::<daemon::TransportMetadataPy>()?;
        m.add_function(wrap_pyfunction!(daemon::daemon_connect, m)?)?;
        m.add_function(wrap_pyfunction!(daemon::async_daemon_health, m)?)?;
        m.add_function(wrap_pyfunction!(daemon::async_daemon_declare_client, m)?)?;
        m.add_function(wrap_pyfunction!(daemon::async_daemon_create_session, m)?)?;
        m.add_function(wrap_pyfunction!(daemon::async_daemon_list_sessions, m)?)?;
        m.add_function(wrap_pyfunction!(daemon::async_daemon_get_snapshot, m)?)?;
        m.add_function(wrap_pyfunction!(daemon::async_daemon_close_session, m)?)?;
    }
    // Milestone C: Core assessment domains
    // C1: Consolidated recon
    m.add_class::<consolidated_recon::ConsolidatedReconConfigPy>()?;
    m.add_class::<consolidated_recon::ReconModuleResultPy>()?;
    m.add_class::<consolidated_recon::ConsolidatedReconReportPy>()?;
    m.add_function(wrap_pyfunction!(
        consolidated_recon::run_consolidated_recon,
        m
    )?)?;
    m.add_function(wrap_pyfunction!(
        consolidated_recon::async_run_consolidated_recon,
        m
    )?)?;
    // C2: GraphQL
    m.add_class::<graphql::GraphQLVulnerabilityPy>()?;
    m.add_class::<graphql::GraphQLTestResultPy>()?;
    m.add_class::<graphql::GraphQLTypePy>()?;
    m.add_class::<graphql::GraphQLFieldPy>()?;
    m.add_class::<graphql::GraphQLArgPy>()?;
    m.add_class::<graphql::GraphQLInputFieldPy>()?;
    m.add_class::<graphql::GraphQLSchemaPy>()?;
    m.add_class::<graphql::GraphQLTestConfigPy>()?;
    m.add_function(wrap_pyfunction!(graphql::graphql_test, m)?)?;
    m.add_function(wrap_pyfunction!(graphql::async_graphql_test, m)?)?;
    // C3: OAuth/OIDC
    m.add_class::<oauth::OAuthVulnerabilityPy>()?;
    m.add_class::<oauth::OAuthEndpointKindPy>()?;
    m.add_class::<oauth::OAuthEndpointPy>()?;
    m.add_class::<oauth::OAuthTestResultPy>()?;
    m.add_class::<oauth::OAuthTestConfigPy>()?;
    m.add_function(wrap_pyfunction!(oauth::oauth_discover_endpoints, m)?)?;
    m.add_function(wrap_pyfunction!(oauth::oauth_test, m)?)?;
    m.add_function(wrap_pyfunction!(oauth::async_oauth_test, m)?)?;
    // C4: Auth assessment
    m.add_class::<auth_assess::AuthTestTypePy>()?;
    m.add_class::<auth_assess::AuthFindingPy>()?;
    m.add_class::<auth_assess::AuthTestConfigPy>()?;
    m.add_class::<auth_assess::AuthTestReportPy>()?;
    m.add_function(wrap_pyfunction!(auth_assess::auth_test, m)?)?;
    m.add_function(wrap_pyfunction!(auth_assess::async_auth_test, m)?)?;
    // C5: Headless browser
    #[cfg(feature = "headless-browser")]
    {
        m.add_class::<browser_assess::XssSourcePy>()?;
        m.add_class::<browser_assess::XssSinkPy>()?;
        m.add_class::<browser_assess::DomXssFindingPy>()?;
        m.add_class::<browser_assess::DiscoveryMethodPy>()?;
        m.add_class::<browser_assess::SpaRoutePy>()?;
        m.add_class::<browser_assess::ClientIssueTypePy>()?;
        m.add_class::<browser_assess::ClientIssuePy>()?;
        m.add_class::<browser_assess::BrowserTestConfigPy>()?;
        m.add_class::<browser_assess::BrowserTestReportPy>()?;
        m.add_function(wrap_pyfunction!(browser_assess::browser_test, m)?)?;
        m.add_function(wrap_pyfunction!(browser_assess::async_browser_test, m)?)?;
    }
    // C6: Advanced hunting
    #[cfg(feature = "advanced-hunting")]
    {
        m.add_class::<hunt::ChainTypePy>()?;
        m.add_class::<hunt::ChainStepPy>()?;
        m.add_class::<hunt::AttackChainPy>()?;
        m.add_class::<hunt::FlawTypePy>()?;
        m.add_class::<hunt::BusinessLogicFlawPy>()?;
        m.add_class::<hunt::RaceTypePy>()?;
        m.add_class::<hunt::RaceConditionPy>()?;
        m.add_class::<hunt::BypassTypePy>()?;
        m.add_class::<hunt::AuthzBypassPy>()?;
        m.add_class::<hunt::SessionIssueTypePy>()?;
        m.add_class::<hunt::SessionIssuePy>()?;
        m.add_class::<hunt::HuntTestConfigPy>()?;
        m.add_class::<hunt::HuntReportPy>()?;
        m.add_function(wrap_pyfunction!(hunt::hunt_test, m)?)?;
        m.add_function(wrap_pyfunction!(hunt::async_hunt_test, m)?)?;
    }
    // B4: Execution Context types
    m.add_class::<execution_context::ExecutionSurfacePy>()?;
    m.add_class::<execution_context::ExecutionProfilePy>()?;
    m.add_class::<execution_context::EnforcementContextPy>()?;
    m.add_class::<execution_context::EnforcementOutcomePy>()?;
    m.add_class::<execution_context::ApprovedOperationPy>()?;
    m.add_class::<execution_context::OperationDescriptorPy>()?;
    m.add_class::<execution_context::PolicyDecisionPy>()?;
    // B5: Authorization Policy types
    m.add_class::<authorization::ExecutionPolicyPy>()?;
    m.add_class::<authorization::ManualOverridePy>()?;
    // B6: Preflight types
    m.add_class::<preflight::PreflightResultPy>()?;
    m.add_function(wrap_pyfunction!(preflight::preflight_operation, m)?)?;
    m.add_function(wrap_pyfunction!(preflight::preflight_with_descriptor, m)?)?;
    // B7: Audit types
    m.add_class::<audit::AuditOutcomePy>()?;
    m.add_class::<audit::ManualOverrideAuditPy>()?;
    m.add_class::<audit::ScopeAuditPy>()?;
    m.add_class::<audit::EnforcementAuditEventPy>()?;
    m.add_function(wrap_pyfunction!(audit::audit_event_from_enforcement, m)?)?;
    m.add_function(wrap_pyfunction!(audit::audit_event_from_preflight, m)?)?;
    m.add_function(wrap_pyfunction!(audit::emit_audit_event, m)?)?;
    // E8: Compliance mapping (feature-gated)
    #[cfg(feature = "compliance")]
    {
        m.add_class::<compliance::ComplianceFrameworkPy>()?;
        m.add_class::<compliance::ComplianceControlPy>()?;
        m.add_class::<compliance::ComplianceMappingPy>()?;
        m.add_class::<compliance::ComplianceResultPy>()?;
        m.add_class::<compliance::ControlAssessmentPy>()?;
        m.add_class::<compliance::ComplianceReportPy>()?;
        m.add_class::<compliance::ComplianceMapperPy>()?;
    }
    // E9: External integrations
    m.add_class::<integrations::IntegrationTypePy>()?;
    m.add_class::<integrations::PublicationRecordPy>()?;
    m.add_class::<integrations::RetryPolicyPy>()?;
    m.add_class::<integrations::PublicationPolicyPy>()?;
    m.add_class::<integrations::ExternalIntegrationPy>()?;
    // E10: Migration and compatibility
    m.add_class::<migration::SchemaVersionPy>()?;
    m.add_class::<migration::MigrationResultPy>()?;
    m.add_class::<migration::FindingMigrationPy>()?;

    Ok(())
}
