mod artifact;
mod async_client;
mod async_engine;
mod async_iter;
mod async_support;
mod audit;
mod auth_assess;
mod authorization;
mod backpressure;
mod baseline;
mod buffer_support;
mod callbacks;
mod cancellation;
mod checkpoint;
mod checkpoint_store;
mod client;
mod config_model;
#[cfg(feature = "container")]
mod container;
mod cvss;
#[cfg(feature = "daemon-client")]
mod daemon;
#[cfg(feature = "db-pentest")]
mod db_pentest;
mod deprecated;
mod domains;
mod dto;
mod endpoint;
mod engine;
mod engine_state;
mod ergonomics;
mod error;
mod event_protocol;
mod event_stream;
mod execution_context;
mod execution_handle;
mod experimental;
mod features;
mod finding;
mod finding_schema;
mod finding_workflow;
mod fingerprint;
#[cfg(feature = "git-secrets")]
mod git_secrets;
mod graphql;
mod handles;
mod http_client;
mod iter_support;
mod lazy_load;
mod loadtest;
#[cfg(feature = "mobile")]
mod mobile;
mod network;
#[cfg(feature = "nse")]
mod nse;
mod oauth;
mod operation_metadata;
pub(crate) mod operation_registry;
#[cfg(feature = "packet-inspection")]
mod packet_inspection;
mod pipeline;
mod planning;
mod preflight;
mod probes;
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

#[cfg(feature = "ai-integration")]
mod ai_postprocess;
#[cfg(feature = "headless-browser")]
mod browser_assess;
#[cfg(feature = "c2")]
mod c2;
#[cfg(feature = "compliance")]
mod compliance;
mod consolidated_recon;
mod distributed;
#[cfg(feature = "evasion")]
mod evasion;
#[cfg(feature = "advanced-hunting")]
mod hunt;
mod integrations;
mod migration;
mod notification;
#[cfg(feature = "postex")]
mod postex;
mod transport;
#[cfg(feature = "websocket")]
mod websocket;
#[cfg(feature = "wireless")]
mod wireless;

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
    m.add(
        "CancellationError",
        m.py().get_type_bound::<CancellationError>(),
    )?;

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
    // G1: Domain descriptors
    m.add_class::<domains::DomainDescriptorPy>()?;
    m.add_class::<domains::DomainRegistry>()?;
    m.add_function(wrap_pyfunction!(domains::domain_maturity, m)?)?;
    m.add_class::<client::Client>()?;
    m.add_class::<async_client::AsyncClient>()?;
    m.add_class::<engine::Engine>()?;
    m.add_class::<async_engine::AsyncEngine>()?;
    m.add_class::<handles::ExecutionHandle>()?;
    m.add_class::<handles::ExecutionEvent>()?;
    m.add_class::<handles::EventLog>()?;
    m.add_class::<handles::LazyEventIterator>()?;
    m.add_class::<execution_handle::ExecutionState>()?;
    m.add_class::<execution_handle::TrackedExecutionHandle>()?;
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
    m.add_class::<finding::FindingSetIteratorPy>()?;
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
    // G5: Buffer support and lazy loading
    m.add_class::<buffer_support::BinaryBufferPy>()?;
    m.add_class::<lazy_load::ArtifactMetaPy>()?;
    m.add_class::<lazy_load::LazyArtifactPy>()?;
    m.add_class::<iter_support::PaginatedResultsPy>()?;
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
    m.add_class::<engine_state::DispatchAuditEvent>()?;
    m.add_class::<status::OperationError>()?;
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
    m.add_class::<requests::GitSecretsScanRequest>()?;
    m.add_class::<requests::SbomRequest>()?;
    m.add_class::<requests::ConsolidatedReconRequest>()?;
    m.add_class::<requests::GraphqlTestRequest>()?;
    m.add_class::<requests::OauthTestRequest>()?;
    m.add_class::<requests::AuthTestRequest>()?;
    m.add_class::<requests::DbProbeRequest>()?;
    m.add_class::<requests::NseRunRequest>()?;
    m.add_class::<requests::DockerImageScanRequest>()?;
    m.add_class::<requests::KubernetesScanRequest>()?;
    m.add_class::<requests::ApkAnalysisRequest>()?;
    m.add_class::<requests::IpaAnalysisRequest>()?;
    m.add_class::<requests::RequestBuilder>()?;
    // Pipeline and assessment types
    m.add_class::<pipeline::OutputRef>()?;
    m.add_class::<pipeline::RetryPolicy>()?;
    m.add_class::<pipeline::FailurePolicy>()?;
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
    // Versioned checkpoint store
    m.add_class::<checkpoint_store::PipelineCheckpoint>()?;
    m.add_class::<checkpoint_store::CheckpointLoadResult>()?;
    m.add_class::<checkpoint_store::CheckpointStore>()?;
    m.add_function(wrap_pyfunction!(
        checkpoint_store::create_checkpoint_store,
        m
    )?)?;
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
        // WS7: Managed capture lifecycle
        m.add_class::<packet_inspection::BackpressurePolicyPy>()?;
        m.add_class::<packet_inspection::CaptureDropStatsPy>()?;
        m.add_class::<packet_inspection::CapturedPacketPy>()?;
        m.add_class::<packet_inspection::AsyncCaptureSessionPy>()?;
        // WS10: Timestamps, streaming, artifacts, sync capture
        m.add_class::<packet_inspection::PacketTimestampPy>()?;
        m.add_class::<packet_inspection::PacketStreamPy>()?;
        m.add_class::<packet_inspection::PacketArtifactPy>()?;
        m.add_class::<packet_inspection::SyncCaptureSessionPy>()?;
        // D3: Network probing
        m.add_class::<packet_inspection::TracerouteConfigPy>()?;
        m.add_class::<packet_inspection::TracerouteHopPy>()?;
        m.add_class::<packet_inspection::TracerouteResultPy>()?;
        // WS8: Packet layer DTOs
        m.add_class::<packet_inspection::EthernetFramePy>()?;
        m.add_class::<packet_inspection::Ipv4PacketPy>()?;
        m.add_class::<packet_inspection::Ipv6PacketPy>()?;
        m.add_class::<packet_inspection::TcpSegmentPy>()?;
        m.add_class::<packet_inspection::UdpDatagramPy>()?;
        m.add_class::<packet_inspection::IcmpPacketPy>()?;
        m.add_class::<packet_inspection::FlowKeyPy>()?;
        m.add_class::<packet_inspection::FlowAggregatorPy>()?;
        // WS10: DNS and TLS decode DTOs
        m.add_class::<packet_inspection::DnsPacketPy>()?;
        m.add_class::<packet_inspection::TlsRecordInfoPy>()?;
        // WS9: Active probe types
        m.add_class::<packet_inspection::IcmpProbeConfigPy>()?;
        m.add_class::<packet_inspection::IcmpProbeReplyPy>()?;
        m.add_class::<packet_inspection::IcmpProbeResultPy>()?;
        m.add_class::<packet_inspection::TcpProbeConfigPy>()?;
        m.add_class::<packet_inspection::TcpProbeResultPy>()?;
        // WS10: UDP reachability probe
        m.add_class::<packet_inspection::UdpReachabilityConfigPy>()?;
        m.add_class::<packet_inspection::UdpReachabilityResultPy>()?;
        m.add_function(wrap_pyfunction!(packet_inspection::icmp_probe, m)?)?;
        m.add_function(wrap_pyfunction!(packet_inspection::async_icmp_probe, m)?)?;
        m.add_function(wrap_pyfunction!(packet_inspection::tcp_syn_probe, m)?)?;
        m.add_function(wrap_pyfunction!(packet_inspection::async_tcp_syn_probe, m)?)?;
        m.add_function(wrap_pyfunction!(packet_inspection::udp_reachability, m)?)?;
    }
    // Phase F Track 2: Load testing
    m.add_class::<loadtest::LoadTestResultPy>()?;
    m.add_class::<loadtest::LoadTestConfig>()?;
    // Release 2: Network programmability DTOs
    m.add_class::<network::TargetPy>()?;
    m.add_class::<network::ResolvedTargetPy>()?;
    m.add_class::<network::ConnectionConfigPy>()?;
    m.add_class::<network::TimeoutConfigPy>()?;
    m.add_class::<network::RetryPolicyPy>()?;
    m.add_class::<network::SocketEndpointPy>()?;
    m.add_class::<network::ConnectionTimingPy>()?;
    m.add_class::<network::ConnectionMetadataPy>()?;
    m.add_class::<network::NetworkEvidencePy>()?;
    m.add_class::<network::TranscriptEntryPy>()?;
    m.add_class::<network::NetworkTranscriptPy>()?;
    m.add_class::<network::ProxyRoutePy>()?;
    // Release 2: TCP/UDP transport session primitives
    m.add_class::<transport::TcpConfigPy>()?;
    m.add_class::<transport::TcpSessionPy>()?;
    m.add_class::<transport::AsyncTcpSessionPy>()?;
    m.add_class::<transport::TcpConnectResultPy>()?;
    m.add_class::<transport::TcpReadResultPy>()?;
    m.add_class::<transport::TcpWriteResultPy>()?;
    m.add_class::<transport::UdpConfigPy>()?;
    m.add_class::<transport::UdpSocketPy>()?;
    m.add_class::<transport::AsyncUdpSocketPy>()?;
    m.add_class::<transport::UdpSendResultPy>()?;
    m.add_class::<transport::UdpRecvResultPy>()?;
    m.add_class::<transport::UdpRecvFromResultPy>()?;
    m.add_class::<transport::BannerProbeResultPy>()?;
    m.add_function(wrap_pyfunction!(transport::tcp_connect_probe, m)?)?;
    m.add_function(wrap_pyfunction!(transport::async_tcp_connect_probe, m)?)?;
    m.add_function(wrap_pyfunction!(transport::banner_probe, m)?)?;
    m.add_function(wrap_pyfunction!(transport::async_banner_probe, m)?)?;
    // Release 2: Protocol probes
    m.add_class::<probes::DnsQueryConfigPy>()?;
    m.add_class::<probes::DnsRecordPy>()?;
    m.add_class::<probes::DnsQueryResultPy>()?;
    m.add_class::<probes::TlsProbeConfigPy>()?;
    m.add_class::<probes::CertificateInfoPy>()?;
    m.add_class::<probes::CertificateChainEntryPy>()?;
    m.add_class::<probes::TlsProbeResultPy>()?;
    m.add_class::<probes::TlsIssuePy>()?;
    m.add_class::<probes::HttpProbeConfigPy>()?;
    m.add_class::<probes::HttpProbeResultPy>()?;
    m.add_class::<probes::UdpProbeConfigPy>()?;
    m.add_class::<probes::UdpProbeResultPy>()?;
    m.add_function(wrap_pyfunction!(probes::dns_query, m)?)?;
    m.add_function(wrap_pyfunction!(probes::async_dns_query, m)?)?;
    m.add_function(wrap_pyfunction!(probes::tls_probe, m)?)?;
    m.add_function(wrap_pyfunction!(probes::async_tls_probe, m)?)?;
    m.add_function(wrap_pyfunction!(probes::http_probe, m)?)?;
    m.add_function(wrap_pyfunction!(probes::async_http_probe, m)?)?;
    m.add_function(wrap_pyfunction!(probes::udp_probe, m)?)?;
    m.add_function(wrap_pyfunction!(probes::async_udp_probe, m)?)?;
    m.add_function(wrap_pyfunction!(network::resolve_target_sync, m)?)?;
    m.add_function(wrap_pyfunction!(network::async_resolve_target, m)?)?;
    // WS10: Evidence-to-finding conversion
    m.add_function(wrap_pyfunction!(network::evidence_to_finding, m)?)?;
    // Release 2: HTTP client
    m.add_class::<http_client::RedactConfigPy>()?;
    m.add_class::<http_client::HttpRequestPy>()?;
    m.add_class::<http_client::HttpHeadersPy>()?;
    m.add_class::<http_client::HttpCookiePy>()?;
    m.add_class::<http_client::RedirectEntryPy>()?;
    m.add_class::<http_client::TlsMetadataPy>()?;
    m.add_class::<http_client::HttpTimingPy>()?;
    m.add_class::<http_client::HttpResponsePy>()?;
    m.add_class::<http_client::HttpClientConfigPy>()?;
    m.add_class::<http_client::HttpClientPy>()?;
    m.add_class::<http_client::AsyncHttpClientPy>()?;
    m.add_function(wrap_pyfunction!(http_client::create_http_client, m)?)?;
    m.add_function(wrap_pyfunction!(http_client::async_create_http_client, m)?)?;
    // Phase F Track 11: Stress testing
    #[cfg(feature = "stress-testing")]
    {
        m.add_class::<stress::StressTypePy>()?;
        m.add_class::<stress::StressConfigPy>()?;
        m.add_class::<stress::StressStatsPy>()?;
        m.add_class::<stress::StressConfigSummaryPy>()?;
        m.add_class::<stress::StressResultPy>()?;
    }
    // Release 2: WebSocket session API
    #[cfg(feature = "websocket")]
    {
        m.add_class::<websocket::WebSocketSessionConfigPy>()?;
        m.add_class::<websocket::WebSocketMessagePy>()?;
        m.add_class::<websocket::WebSocketFramePy>()?;
        m.add_class::<websocket::WebSocketCloseInfoPy>()?;
        m.add_class::<websocket::WebSocketHandshakePy>()?;
        m.add_class::<websocket::WebSocketSessionPy>()?;
        m.add_class::<websocket::AsyncWebSocketSessionPy>()?;
        m.add_class::<websocket::WebSocketAssessmentConfigPy>()?;
        m.add_class::<websocket::WebSocketAssessmentResultPy>()?;
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
    m.add_function(wrap_pyfunction!(features::feature_matrix, m)?)?;
    m.add_function(wrap_pyfunction!(version::build_info, m)?)?;
    m.add_function(wrap_pyfunction!(version::api_surface_version, m)?)?;
    m.add_function(wrap_pyfunction!(deprecated::deprecated_warning, m)?)?;
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
    // Release 2: WebSocket session functions
    #[cfg(feature = "websocket")]
    {
        m.add_function(wrap_pyfunction!(websocket::websocket_assess, m)?)?;
        m.add_function(wrap_pyfunction!(websocket::async_websocket_assess, m)?)?;
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
        m.add_function(wrap_pyfunction!(daemon::async_daemon_submit_task, m)?)?;
        m.add_function(wrap_pyfunction!(daemon::async_daemon_cancel_task, m)?)?;
        m.add_function(wrap_pyfunction!(daemon::async_daemon_cancel_active, m)?)?;
        m.add_function(wrap_pyfunction!(daemon::async_daemon_approve_policy, m)?)?;
        m.add_function(wrap_pyfunction!(
            daemon::async_daemon_list_persisted_sessions,
            m
        )?)?;
        m.add_function(wrap_pyfunction!(
            daemon::async_daemon_get_persisted_snapshot,
            m
        )?)?;
        m.add_function(wrap_pyfunction!(daemon::async_daemon_subscribe, m)?)?;
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
    // Milestone F: Specialized lab domains
    #[cfg(feature = "wireless")]
    {
        m.add_class::<wireless::SecurityTypePy>()?;
        m.add_class::<wireless::WirelessNetworkPy>()?;
        m.add_class::<wireless::WirelessVulnerabilityPy>()?;
        m.add_class::<wireless::WirelessScanResultPy>()?;
        m.add_class::<wireless::WirelessScanConfigPy>()?;
        m.add_function(wrap_pyfunction!(wireless::wireless_scan, m)?)?;
        m.add_function(wrap_pyfunction!(wireless::async_wireless_scan, m)?)?;
        m.add_function(wrap_pyfunction!(wireless::wireless_analyze_networks, m)?)?;
    }
    #[cfg(feature = "evasion")]
    {
        m.add_class::<evasion::EvasionTargetTypePy>()?;
        m.add_class::<evasion::EvasionCategoryPy>()?;
        m.add_class::<evasion::EvasionRiskPy>()?;
        m.add_class::<evasion::EvasionTechniquePy>()?;
        m.add_class::<evasion::EvasionDetectionPy>()?;
        m.add_class::<evasion::EvasionSummaryPy>()?;
        m.add_class::<evasion::EvasionReportPy>()?;
        m.add_class::<evasion::EvasionScanConfigPy>()?;
        m.add_function(wrap_pyfunction!(evasion::evasion_scan, m)?)?;
        m.add_function(wrap_pyfunction!(evasion::async_evasion_scan, m)?)?;
        m.add_function(wrap_pyfunction!(evasion::evasion_list_techniques, m)?)?;
    }
    #[cfg(feature = "postex")]
    {
        m.add_class::<postex::PostexCategoryPy>()?;
        m.add_class::<postex::PostexRiskPy>()?;
        m.add_class::<postex::PostexProfilePy>()?;
        m.add_class::<postex::PostexTechniquePy>()?;
        m.add_class::<postex::PostexDetectionPy>()?;
        m.add_class::<postex::PostexSummaryPy>()?;
        m.add_class::<postex::PostexReportPy>()?;
        m.add_class::<postex::PostexScanConfigPy>()?;
        m.add_function(wrap_pyfunction!(postex::postex_scan, m)?)?;
        m.add_function(wrap_pyfunction!(postex::async_postex_scan, m)?)?;
        m.add_function(wrap_pyfunction!(postex::postex_list_techniques, m)?)?;
    }
    #[cfg(feature = "c2")]
    {
        m.add_class::<c2::BeaconProtocolPy>()?;
        m.add_class::<c2::TaskTypePy>()?;
        m.add_class::<c2::TaskStatusPy>()?;
        m.add_class::<c2::OpsecCategoryPy>()?;
        m.add_class::<c2::OpsecSeverityPy>()?;
        m.add_class::<c2::CampaignPhasePy>()?;
        m.add_class::<c2::C2CampaignPy>()?;
        m.add_class::<c2::BeaconResultPy>()?;
        m.add_class::<c2::C2TaskResultPy>()?;
        m.add_class::<c2::OpsecFindingPy>()?;
        m.add_class::<c2::OpsecAssessmentPy>()?;
        m.add_class::<c2::C2SummaryPy>()?;
        m.add_class::<c2::C2ReportPy>()?;
        m.add_class::<c2::C2ScanConfigPy>()?;
        m.add_function(wrap_pyfunction!(c2::c2_scan, m)?)?;
        m.add_function(wrap_pyfunction!(c2::async_c2_scan, m)?)?;
        m.add_function(wrap_pyfunction!(c2::c2_get_campaign, m)?)?;
    }
    // Distributed (always-available)
    m.add_class::<distributed::DistributedTaskTypePy>()?;
    m.add_class::<distributed::WorkerStatusPy>()?;
    m.add_class::<distributed::WorkerRegistrationPy>()?;
    m.add_class::<distributed::HeartbeatPy>()?;
    m.add_class::<distributed::DistributedTaskPy>()?;
    m.add_class::<distributed::DistributedTaskResultPy>()?;
    m.add_function(wrap_pyfunction!(distributed::distributed_task_types, m)?)?;
    m.add_function(wrap_pyfunction!(distributed::distributed_generate_psk, m)?)?;
    // Notifications (always-available)
    m.add_class::<notification::WebhookEventPy>()?;
    m.add_class::<notification::FindingSummaryPy>()?;
    m.add_class::<notification::NotifyScanStatsPy>()?;
    m.add_class::<notification::WebhookConfigPy>()?;
    m.add_class::<notification::NotifyManagerPy>()?;
    m.add_function(wrap_pyfunction!(notification::notify_scan_started, m)?)?;
    m.add_function(wrap_pyfunction!(notification::notify_scan_complete, m)?)?;
    m.add_function(wrap_pyfunction!(notification::notify_findings, m)?)?;
    m.add_function(wrap_pyfunction!(notification::notify_error, m)?)?;
    // G2: Event protocol stabilization
    m.add_class::<event_protocol::EventEnvelope>()?;
    m.add_class::<event_protocol::PlanningEvent>()?;
    m.add_class::<event_protocol::PreflightEvent>()?;
    m.add_class::<event_protocol::StageLifecycleEvent>()?;
    m.add_class::<event_protocol::ProgressEvent>()?;
    m.add_class::<event_protocol::FindingEvent>()?;
    m.add_class::<event_protocol::ArtifactEvent>()?;
    m.add_class::<event_protocol::CancellationEvent>()?;
    m.add_class::<event_protocol::FailureEvent>()?;
    m.add_class::<event_protocol::CompletionEvent>()?;
    // WS11: Network-specific events
    m.add_class::<event_protocol::ResolutionEvent>()?;
    m.add_class::<event_protocol::ConnectionEvent>()?;
    m.add_class::<event_protocol::ProbeEvent>()?;
    m.add_class::<event_protocol::WebSocketMessageEvent>()?;
    m.add_class::<event_protocol::CaptureStatsEvent>()?;
    m.add_class::<event_protocol::HandshakeCompletedEvent>()?;
    m.add_class::<event_protocol::RequestSentEvent>()?;
    m.add_class::<event_protocol::ResponseHeadersReceivedEvent>()?;
    m.add_class::<event_protocol::BodyProgressEvent>()?;
    m.add_class::<event_protocol::CaptureStartedEvent>()?;
    m.add_class::<event_protocol::PacketSampledEvent>()?;
    m.add_class::<event_protocol::FlowObservedEvent>()?;
    m.add_class::<event_protocol::ArtifactCreatedEvent>()?;
    m.add_function(wrap_pyfunction!(event_protocol::wrap_event, m)?)?;
    m.add("EVENT_SCHEMA_VERSION", event_protocol::EVENT_SCHEMA_VERSION)?;
    m.add_class::<event_stream::EventStream>()?;
    m.add_function(wrap_pyfunction!(event_stream::event_stream_from_legacy, m)?)?;
    // G4: Async iterators
    m.add_class::<async_iter::EventStreamAsyncIterator>()?;
    m.add_class::<async_iter::FindingStreamAsyncIterator>()?;
    // G3: Callbacks and sinks
    m.add_class::<callbacks::AuditSink>()?;
    m.add_class::<callbacks::FindingSink>()?;
    m.add_class::<callbacks::ArtifactSink>()?;
    m.add_class::<callbacks::ProgressSink>()?;
    m.add_class::<callbacks::EventConsumer>()?;
    m.add_class::<async_support::AsyncCallback>()?;
    m.add_class::<async_support::CallbackScheduler>()?;
    m.add_class::<backpressure::PyBackpressureChannel>()?;
    m.add_class::<backpressure::EventDeliveryStats>()?;
    // AI post-processing (feature-gated)
    #[cfg(feature = "ai-integration")]
    {
        m.add_class::<ai_postprocess::AiProviderPy>()?;
        m.add_class::<ai_postprocess::PluginLanguagePy>()?;
        m.add_class::<ai_postprocess::AiAnalysisResultPy>()?;
        m.add_class::<ai_postprocess::AiPayloadSuggestionPy>()?;
        m.add_class::<ai_postprocess::AiWafBypassSuggestionPy>()?;
        m.add_class::<ai_postprocess::AiCacheStatsPy>()?;
        m.add_class::<ai_postprocess::ScriptMetadataPy>()?;
        m.add_class::<ai_postprocess::GeneratedScriptPy>()?;
        m.add_class::<ai_postprocess::AiCachePy>()?;
        m.add_function(wrap_pyfunction!(ai_postprocess::ai_analyze_finding, m)?)?;
        m.add_function(wrap_pyfunction!(
            ai_postprocess::async_ai_analyze_finding,
            m
        )?)?;
        m.add_function(wrap_pyfunction!(ai_postprocess::ai_generate_payloads, m)?)?;
        m.add_function(wrap_pyfunction!(ai_postprocess::ai_suggest_waf_bypass, m)?)?;
    }

    // G6: Deprecation and experimental markers
    m.add_class::<deprecated::DeprecatedWarning>()?;
    m.add("_experimental", true)?;
    // G6: api_surface introspection
    m.add_function(wrap_pyfunction!(api_surface, m)?)?;
    // G7: Version constants
    m.add("SCHEMA_VERSION", version::SCHEMA_VERSION)?;
    m.add("PROTOCOL_VERSION", version::PROTOCOL_VERSION)?;
    m.add("ABI_VERSION", version::ABI_VERSION)?;

    Ok(())
}

/// Returns a machine-readable dict of all exported names, their stability level,
/// and any deprecation info.
#[pyfunction]
fn api_surface() -> PyObject {
    Python::with_gil(|py| {
        let dict = pyo3::types::PyDict::new_bound(py);

        macro_rules! add_entry {
            ($name:expr, $stability:expr) => {
                let entry = pyo3::types::PyDict::new_bound(py);
                entry.set_item("stability", $stability).unwrap();
                entry.set_item::<_, bool>("deprecated", false).unwrap();
                dict.set_item($name, entry).unwrap();
            };
            ($name:expr, $stability:expr, $deprecated:expr) => {
                let entry = pyo3::types::PyDict::new_bound(py);
                entry.set_item("stability", $stability).unwrap();
                entry.set_item::<_, bool>("deprecated", true).unwrap();
                entry.set_item("deprecated_with", $deprecated).unwrap();
                dict.set_item($name, entry).unwrap();
            };
        }

        // Stable: real execution path, result data preserved, policy integrated,
        // serialization versioned, behavior tests pass
        add_entry!("scan_ports", "stable");
        add_entry!("async_scan_ports", "stable");
        add_entry!("scan_endpoints", "stable");
        add_entry!("async_scan_endpoints", "stable");
        add_entry!("fingerprint_services", "stable");
        add_entry!("async_fingerprint_services", "stable");
        add_entry!("recon_dns", "stable");
        add_entry!("async_recon_dns", "stable");
        add_entry!("inspect_tls", "stable");
        add_entry!("async_inspect_tls", "stable");
        add_entry!("detect_technology", "stable");
        add_entry!("async_detect_technology", "stable");
        add_entry!("detect_waf", "stable");
        add_entry!("async_detect_waf", "stable");
        add_entry!("validate_waf", "stable");
        add_entry!("async_validate_waf", "stable");
        add_entry!("fuzz_http", "stable");
        add_entry!("async_fuzz_http", "stable");
        add_entry!("generate_fuzz_payloads", "stable");
        add_entry!("load_test_http", "stable");
        add_entry!("async_load_test_http", "stable");

        // Stable: introspection and utility functions
        add_entry!("features", "stable");
        add_entry!("has_feature", "stable");
        add_entry!("build_info", "stable");
        add_entry!("feature_matrix", "stable");
        add_entry!("domain_maturity", "stable");
        add_entry!("api_surface_version", "stable");
        add_entry!("api_surface", "stable");
        add_entry!("validate_scope", "stable");
        add_entry!("preflight_operation", "stable");
        add_entry!("preflight_with_descriptor", "stable");
        add_entry!("audit_event_from_enforcement", "stable");
        add_entry!("audit_event_from_preflight", "stable");
        add_entry!("emit_audit_event", "stable");
        add_entry!("distributed_task_types", "stable");
        add_entry!("distributed_generate_psk", "stable");
        add_entry!("notify_scan_started", "stable");
        add_entry!("notify_scan_complete", "stable");
        add_entry!("notify_findings", "stable");
        add_entry!("notify_error", "stable");

        // Stable: promoted domains — real execution path, result data preserved,
        // policy integrated, serialization versioned, behavior tests pass
        add_entry!("run_consolidated_recon", "stable");
        add_entry!("async_run_consolidated_recon", "stable");
        add_entry!("graphql_test", "stable");
        add_entry!("async_graphql_test", "stable");
        add_entry!("oauth_discover_endpoints", "stable");
        add_entry!("oauth_test", "stable");
        add_entry!("async_oauth_test", "stable");
        add_entry!("auth_test", "stable");
        add_entry!("async_auth_test", "stable");

        // Stable: feature-gated promoted domains
        add_entry!("scan_git_secrets", "stable");
        add_entry!("async_scan_git_secrets", "stable");
        add_entry!("generate_sbom", "stable");
        add_entry!("async_generate_sbom", "stable");
        add_entry!("db_probe", "stable");
        add_entry!("async_db_probe", "stable");
        add_entry!("db_probe_with_config", "stable");
        add_entry!("db_probe_postgres", "stable");
        add_entry!("db_probe_mysql", "stable");
        add_entry!("db_probe_mssql", "stable");
        add_entry!("db_probe_mongodb", "stable");
        add_entry!("db_probe_redis", "stable");
        add_entry!("db_list_drivers", "stable");
        add_entry!("db_get_capabilities", "stable");
        add_entry!("db_run_with_config", "stable");
        add_entry!("analyze_apk", "stable");
        add_entry!("async_analyze_apk", "stable");
        add_entry!("analyze_ipa", "stable");
        add_entry!("async_analyze_ipa", "stable");
        add_entry!("scan_docker_image", "stable");
        add_entry!("async_scan_docker_image", "stable");
        add_entry!("scan_kubernetes", "stable");
        add_entry!("async_scan_kubernetes", "stable");
        add_entry!("nse_run", "stable");
        add_entry!("async_nse_run", "stable");
        add_entry!("nse_list_libraries", "stable");
        add_entry!("nse_list_scripts", "stable");
        add_entry!("nse_get_script_metadata", "stable");

        // Provisional: public API shape accepted, implementation works but
        // lacks full backend validation or end-to-end tests
        add_entry!("websocket_probe", "provisional");
        add_entry!("async_websocket_probe", "provisional");
        add_entry!("websocket_fuzz", "provisional");
        add_entry!("async_websocket_fuzz", "provisional");

        // Provisional: WebSocket session API (Release 2 workstream 6)
        add_entry!("WebSocketSessionConfig", "provisional");
        add_entry!("WebSocketMessage", "provisional");
        add_entry!("WebSocketFrame", "provisional");
        add_entry!("WebSocketCloseInfo", "provisional");
        add_entry!("WebSocketHandshake", "provisional");
        add_entry!("WebSocketSession", "provisional");
        add_entry!("AsyncWebSocketSession", "provisional");
        add_entry!("WebSocketAssessmentConfig", "provisional");
        add_entry!("WebSocketAssessmentResult", "provisional");
        add_entry!("websocket_assess", "provisional");
        add_entry!("async_websocket_assess", "provisional");

        // Provisional: feature-gated, implementation works but lacks full
        // backend/platform validation
        add_entry!("create_proxy_manager", "provisional");
        add_entry!("async_add_proxy", "provisional");
        add_entry!("async_proxy_health_check", "provisional");
        add_entry!("detect_escape_risks", "provisional");
        add_entry!("check_cis_docker_benchmark", "provisional");
        add_entry!("list_network_interfaces", "provisional");
        add_entry!("parse_pcap", "provisional");
        add_entry!("run_traceroute", "provisional");
        add_entry!("async_run_traceroute", "provisional");
        add_entry!("traceroute", "provisional");
        add_entry!("daemon_connect", "provisional");
        add_entry!("async_daemon_health", "provisional");
        add_entry!("async_daemon_declare_client", "provisional");
        add_entry!("async_daemon_create_session", "provisional");
        add_entry!("async_daemon_list_sessions", "provisional");
        add_entry!("async_daemon_get_snapshot", "provisional");
        add_entry!("async_daemon_close_session", "provisional");
        add_entry!("async_daemon_submit_task", "provisional");
        add_entry!("async_daemon_cancel_task", "provisional");
        add_entry!("async_daemon_cancel_active", "provisional");
        add_entry!("async_daemon_approve_policy", "provisional");
        add_entry!("async_daemon_list_persisted_sessions", "provisional");
        add_entry!("async_daemon_get_persisted_snapshot", "provisional");
        add_entry!("async_daemon_subscribe", "provisional");

        // Experimental: platform-sensitive, hazardous, incomplete, or
        // subject to substantial change
        add_entry!("wireless_scan", "experimental");
        add_entry!("async_wireless_scan", "experimental");
        add_entry!("wireless_analyze_networks", "experimental");
        add_entry!("evasion_scan", "experimental");
        add_entry!("async_evasion_scan", "experimental");
        add_entry!("evasion_list_techniques", "experimental");
        add_entry!("postex_scan", "experimental");
        add_entry!("async_postex_scan", "experimental");
        add_entry!("postex_list_techniques", "experimental");
        add_entry!("c2_scan", "experimental");
        add_entry!("async_c2_scan", "experimental");
        add_entry!("c2_get_campaign", "experimental");
        add_entry!("browser_test", "experimental");
        add_entry!("async_browser_test", "experimental");
        add_entry!("hunt_test", "experimental");
        add_entry!("async_hunt_test", "experimental");
        add_entry!("ai_analyze_finding", "experimental");
        add_entry!("async_ai_analyze_finding", "experimental");
        add_entry!("ai_generate_payloads", "experimental");
        add_entry!("ai_suggest_waf_bypass", "experimental");
        add_entry!("list_mobile_devices", "experimental");
        add_entry!("dynamic_mobile_analysis", "experimental");
        add_entry!("stress_test", "experimental");
        add_entry!("async_stress_test", "experimental");

        // Release 2: Protocol probes (provisional)
        add_entry!("dns_query", "provisional");
        add_entry!("async_dns_query", "provisional");
        add_entry!("tls_probe", "provisional");
        add_entry!("async_tls_probe", "provisional");
        add_entry!("http_probe", "provisional");
        add_entry!("async_http_probe", "provisional");
        // Release 2: HTTP client (provisional)
        add_entry!("create_http_client", "provisional");
        add_entry!("async_create_http_client", "provisional");
        add_entry!("HttpClientPy", "provisional");
        add_entry!("AsyncHttpClientPy", "provisional");
        add_entry!("HttpRequestPy", "provisional");
        add_entry!("HttpResponsePy", "provisional");
        add_entry!("HttpClientConfigPy", "provisional");
        add_entry!("HttpHeadersPy", "provisional");
        add_entry!("HttpCookiePy", "provisional");
        add_entry!("HttpTimingPy", "provisional");
        add_entry!("TlsMetadataPy", "provisional");
        add_entry!("RedirectEntryPy", "provisional");
        add_entry!("RedactConfigPy", "provisional");

        // G2: Event protocol (stable — versioned serialization)
        add_entry!("EventEnvelope", "stable");
        add_entry!("PlanningEvent", "stable");
        add_entry!("PreflightEvent", "stable");
        add_entry!("StageLifecycleEvent", "stable");
        add_entry!("ProgressEvent", "stable");
        add_entry!("FindingEvent", "stable");
        add_entry!("ArtifactEvent", "stable");
        add_entry!("CancellationEvent", "stable");
        add_entry!("FailureEvent", "stable");
        add_entry!("CompletionEvent", "stable");
        add_entry!("EventDeliveryStats", "provisional");
        // WS11: Network-specific events
        add_entry!("HandshakeCompletedEvent", "provisional");
        add_entry!("RequestSentEvent", "provisional");
        add_entry!("ResponseHeadersReceivedEvent", "provisional");
        add_entry!("BodyProgressEvent", "provisional");
        add_entry!("CaptureStartedEvent", "provisional");
        add_entry!("PacketSampledEvent", "provisional");
        add_entry!("FlowObservedEvent", "provisional");
        add_entry!("ArtifactCreatedEvent", "provisional");
        add_entry!("wrap_event", "stable");
        add_entry!("EVENT_SCHEMA_VERSION", "stable");
        add_entry!("EventStream", "stable");
        add_entry!("event_stream_from_legacy", "stable");
        // G3: Callbacks and sinks (stable — contract types)
        add_entry!("AuditSink", "stable");
        add_entry!("FindingSink", "stable");
        add_entry!("ArtifactSink", "stable");
        add_entry!("ProgressSink", "stable");
        add_entry!("EventConsumer", "stable");
        add_entry!("AsyncCallback", "stable");
        add_entry!("CallbackScheduler", "stable");
        add_entry!("PyBackpressureChannel", "stable");
        add_entry!("EventDeliveryStats", "provisional");

        // Stable classes (always available, real execution path)
        add_entry!("Scope", "stable");
        add_entry!("Client", "stable");
        add_entry!("AsyncClient", "stable");
        add_entry!("Engine", "stable");
        add_entry!("AsyncEngine", "stable");
        add_entry!("Severity", "stable");
        add_entry!("Finding", "stable");
        add_entry!("Report", "stable");
        add_entry!("ExecutionHandle", "stable");
        add_entry!("ExecutionEvent", "stable");
        add_entry!("EventLog", "stable");
        add_entry!("CancellationToken", "stable");

        // Provisional: Milestone B/C/D/E types — working but lacking
        // full backend validation
        add_entry!("EggsecConfig", "provisional");
        add_entry!("LoadedScope", "provisional");
        add_entry!("OperationRegistry", "provisional");
        add_entry!("EnforcementContext", "provisional");
        add_entry!("ExecutionPolicy", "provisional");
        add_entry!("ManualOverride", "provisional");
        add_entry!("PreflightResult", "provisional");
        add_entry!("EnforcementAuditEvent", "provisional");
        add_entry!("OperationDescriptor", "provisional");
        add_entry!("OperationMetadataView", "provisional");
        add_entry!("Pipeline", "provisional");
        add_entry!("AsyncPipeline", "provisional");
        add_entry!("PipelineStep", "provisional");
        add_entry!("StepResult", "provisional");
        add_entry!("PipelineResult", "provisional");
        add_entry!("ScanPlan", "provisional");
        add_entry!("PlanStep", "provisional");
        add_entry!("Checkpoint", "provisional");
        add_entry!("CheckpointStore", "provisional");
        add_entry!("ExecutionSurface", "provisional");
        add_entry!("ExecutionProfile", "provisional");
        add_entry!("ApprovedOperation", "provisional");
        add_entry!("PolicyDecision", "provisional");
        add_entry!("DomainDescriptorPy", "provisional");
        add_entry!("DomainRegistry", "provisional");

        // Provisional: Config model types (Milestone B)
        add_entry!("PyEggsecConfig", "provisional");
        add_entry!("PySensitiveString", "provisional");
        add_entry!("PyHttpConfig", "provisional");
        add_entry!("PyScanConfig", "provisional");
        add_entry!("PyOutputConfig", "provisional");
        add_entry!("PyReconConfig", "provisional");
        add_entry!("PyReconApiConfig", "provisional");
        add_entry!("PyAiConfig", "provisional");
        add_entry!("PySearchConfig", "provisional");
        add_entry!("PyPathsConfig", "provisional");
        add_entry!("PyCacheConfig", "provisional");

        // Stable: Scope evaluation types
        add_entry!("ScopeSource", "stable");
        add_entry!("ScopeRule", "stable");
        add_entry!("ScopeExplanation", "stable");
        add_entry!("ScopeValidation", "stable");

        // Provisional: Operation metadata types
        add_entry!("OperationRisk", "provisional");
        add_entry!("OperationMode", "provisional");
        add_entry!("IntendedUse", "provisional");
        add_entry!("Capability", "provisional");
        add_entry!("DenialClass", "provisional");
        add_entry!("TargetPolicyKind", "provisional");

        // Stable: DTO types (result data preserved, serialization versioned)
        add_entry!("PortScanResult", "stable");
        add_entry!("OpenPort", "stable");
        add_entry!("ScanStats", "stable");
        add_entry!("PortRange", "stable");
        add_entry!("TimingPreset", "stable");
        add_entry!("EndpointScanResult", "stable");
        add_entry!("EndpointFinding", "stable");
        add_entry!("EndpointScanStats", "stable");
        add_entry!("FingerprintScanResult", "stable");
        add_entry!("FingerprintEvidence", "stable");
        add_entry!("FingerprintConfidence", "stable");

        // Stable: Recon result types
        add_entry!("DnsRecordSet", "stable");
        add_entry!("MxRecord", "stable");
        add_entry!("SoaRecord", "stable");
        add_entry!("TlsInspectionResult", "stable");
        add_entry!("TlsCertificateInfo", "stable");
        add_entry!("SslIssue", "stable");
        add_entry!("TechDetectionResult", "stable");
        add_entry!("TechStack", "stable");
        add_entry!("WafDetectionResult", "stable");

        // Stable: Request types
        add_entry!("OperationRequest", "stable");
        add_entry!("PortScanRequest", "stable");
        add_entry!("EndpointScanRequest", "stable");
        add_entry!("FingerprintRequest", "stable");
        add_entry!("ReconDnsRequest", "stable");
        add_entry!("TlsInspectRequest", "stable");
        add_entry!("TechDetectRequest", "stable");
        add_entry!("WafDetectRequest", "stable");
        add_entry!("LoadTestRequest", "stable");
        add_entry!("WafValidateRequest", "stable");
        add_entry!("FuzzRequest", "stable");
        add_entry!("GitSecretsScanRequest", "stable");
        add_entry!("SbomRequest", "stable");
        add_entry!("ConsolidatedReconRequest", "stable");
        add_entry!("GraphqlTestRequest", "stable");
        add_entry!("OauthTestRequest", "stable");
        add_entry!("AuthTestRequest", "stable");
        add_entry!("DbProbeRequest", "stable");
        add_entry!("NseRunRequest", "stable");
        add_entry!("DockerImageScanRequest", "stable");
        add_entry!("KubernetesScanRequest", "stable");
        add_entry!("ApkAnalysisRequest", "stable");
        add_entry!("IpaAnalysisRequest", "stable");
        add_entry!("RequestBuilder", "stable");

        // Stable: Common result protocol types
        add_entry!("ExecutionStatus", "stable");
        add_entry!("ExecutionStats", "stable");
        add_entry!("Artifact", "stable");
        add_entry!("OperationResult", "stable");
        add_entry!("OperationError", "stable");
        add_entry!("DispatchAuditEvent", "stable");

        // Provisional: Finding and reporting types (Milestone D/E)
        add_entry!("Evidence", "provisional");
        add_entry!("FindingSet", "provisional");
        add_entry!("Confidence", "provisional");
        add_entry!("FindingType", "provisional");
        add_entry!("EvidenceKind", "provisional");
        add_entry!("AffectedAsset", "provisional");
        add_entry!("FindingLocation", "provisional");
        add_entry!("VersionedEvidence", "provisional");
        add_entry!("VersionedFinding", "provisional");
        add_entry!("ArtifactPy", "provisional");
        add_entry!("ArtifactReferencePy", "provisional");
        add_entry!("ArtifactStorePy", "provisional");
        add_entry!("CvssScore", "provisional");
        add_entry!("VulnerabilityRecord", "provisional");
        add_entry!("RemediationRecord", "provisional");
        add_entry!("FindingState", "provisional");
        add_entry!("WorkflowTransition", "provisional");
        add_entry!("Suppression", "provisional");
        add_entry!("FindingWorkflow", "provisional");
        add_entry!("FindingRepository", "provisional");
        add_entry!("Assessment", "provisional");
        add_entry!("AssessmentRepository", "provisional");
        add_entry!("FindingCorrelation", "provisional");
        add_entry!("FindingDiff", "provisional");
        add_entry!("AssessmentDiff", "provisional");
        add_entry!("BaselineComparator", "provisional");
        add_entry!("FindingReporter", "provisional");
        add_entry!("SeveritySummary", "provisional");
        add_entry!("ReportEnvelope", "provisional");
        add_entry!("BinaryBuffer", "provisional");
        add_entry!("LazyArtifact", "provisional");
        add_entry!("PaginatedResults", "provisional");

        // Provisional: WAF validation and fuzz types
        add_entry!("BypassResult", "provisional");
        add_entry!("WafScanResult", "provisional");
        add_entry!("Payload", "provisional");
        add_entry!("FuzzResult", "provisional");
        add_entry!("FuzzSession", "provisional");
        add_entry!("FuzzConfig", "provisional");

        // Provisional: Load test types
        add_entry!("LoadTestResult", "provisional");
        add_entry!("LoadTestConfig", "provisional");

        // Provisional: Consolidated recon types (Milestone C)
        add_entry!("ConsolidatedReconConfig", "provisional");
        add_entry!("ReconModuleResult", "provisional");
        add_entry!("ConsolidatedReconReport", "provisional");

        // Provisional: GraphQL types (Milestone C)
        add_entry!("GraphQLVulnerability", "provisional");
        add_entry!("GraphQLTestResult", "provisional");
        add_entry!("GraphQLType", "provisional");
        add_entry!("GraphQLField", "provisional");
        add_entry!("GraphQLArg", "provisional");
        add_entry!("GraphQLInputField", "provisional");
        add_entry!("GraphQLSchema", "provisional");
        add_entry!("GraphQLTestConfig", "provisional");

        // Provisional: OAuth types (Milestone C)
        add_entry!("OAuthVulnerability", "provisional");
        add_entry!("OAuthEndpointKind", "provisional");
        add_entry!("OAuthEndpoint", "provisional");
        add_entry!("OAuthTestResult", "provisional");
        add_entry!("OAuthTestConfig", "provisional");

        // Provisional: Auth assessment types (Milestone C)
        add_entry!("AuthTestType", "provisional");
        add_entry!("AuthFinding", "provisional");
        add_entry!("AuthTestConfig", "provisional");
        add_entry!("AuthTestReport", "provisional");

        // Version constants
        add_entry!("__version__", "stable");
        add_entry!("__version_info__", "stable");
        add_entry!("FINDING_SCHEMA_VERSION", "stable");
        add_entry!("SCHEMA_VERSION", "stable");
        add_entry!("PROTOCOL_VERSION", "stable");
        add_entry!("ABI_VERSION", "stable");

        // Deprecated
        add_entry!(
            "deprecated_warning",
            "deprecated",
            "Use DeprecationWarning directly"
        );

        dict.into()
    })
}
