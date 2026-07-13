__version__: str
__version_info__: tuple[int, int, int]
__schema_version__: str
__protocol_version__: str
__abi_version__: str
FINDING_SCHEMA_VERSION: str

def api_surface() -> dict[str, dict]: ...
def domain_maturity() -> dict[str, dict[str, str]]: ...
def _deprecated(name: str, replacement: str | None = None) -> None: ...

from .errors import (
    EggsecError as EggsecError,
    ConfigError as ConfigError,
    ScopeError as ScopeError,
    EnforcementError as EnforcementError,
    NetworkError as NetworkError,
    ScanError as ScanError,
    TimeoutError as TimeoutError,
    FeatureUnavailableError as FeatureUnavailableError,
    SerializationError as SerializationError,
    InternalError as InternalError,
    CancellationError as CancellationError,
)
from .scope import Scope as Scope
from .client import Client as Client
from .async_client import AsyncClient as AsyncClient
from .engine import Engine as Engine
from .async_engine import AsyncEngine as AsyncEngine
from .handles import ExecutionHandle as ExecutionHandle, ExecutionEvent as ExecutionEvent, EventLog as EventLog
from .handles import ExecutionState as ExecutionState, TrackedExecutionHandle as TrackedExecutionHandle
from .cancellation import CancellationToken as CancellationToken
from .dto import (
    PortRange as PortRange,
    TimingPreset as TimingPreset,
    OpenPort as OpenPort,
    ScanStats as ScanStats,
    PortScanResult as PortScanResult,
)
from .endpoint import (
    EndpointScanConfig as EndpointScanConfig,
    EndpointFinding as EndpointFinding,
    EndpointScanStats as EndpointScanStats,
    EndpointScanResult as EndpointScanResult,
)
from .fingerprint import (
    FingerprintEvidence as FingerprintEvidence,
    FingerprintConfidence as FingerprintConfidence,
    ServiceFingerprintResult as ServiceFingerprintResult,
    FingerprintScanResult as FingerprintScanResult,
)
from .finding import (
    Severity as Severity,
    Evidence as Evidence,
    Finding as Finding,
    FindingSet as FindingSet,
    Report as Report,
)
from .status import (
    ExecutionStatus as ExecutionStatus,
    ExecutionStats as ExecutionStats,
    Artifact as Artifact,
    OperationResult as OperationResult,
    OperationError as OperationError,
)
from .recon import (
    MxRecord as MxRecord,
    SoaRecord as SoaRecord,
    DnsRecordSet as DnsRecordSet,
    TlsCertificateInfo as TlsCertificateInfo,
    SslIssue as SslIssue,
    TlsInspectionResult as TlsInspectionResult,
    TechStack as TechStack,
    TechDetectionResult as TechDetectionResult,
)
from .waf import WafDetectionResult as WafDetectionResult
from .requests import (
    OperationRequest as OperationRequest,
    PortScanRequest as PortScanRequest,
    EndpointScanRequest as EndpointScanRequest,
    FingerprintRequest as FingerprintRequest,
    ReconDnsRequest as ReconDnsRequest,
    TlsInspectRequest as TlsInspectRequest,
    TechDetectRequest as TechDetectRequest,
    WafDetectRequest as WafDetectRequest,
    LoadTestRequest as LoadTestRequest,
    WafValidateRequest as WafValidateRequest,
    FuzzRequest as FuzzRequest,
    GitSecretsScanRequest as GitSecretsScanRequest,
    SbomRequest as SbomRequest,
    ConsolidatedReconRequest as ConsolidatedReconRequest,
    GraphqlTestRequest as GraphqlTestRequest,
    OauthTestRequest as OauthTestRequest,
    AuthTestRequest as AuthTestRequest,
    DbProbeRequest as DbProbeRequest,
    NseRunRequest as NseRunRequest,
    DockerImageScanRequest as DockerImageScanRequest,
    KubernetesScanRequest as KubernetesScanRequest,
    ApkAnalysisRequest as ApkAnalysisRequest,
    IpaAnalysisRequest as IpaAnalysisRequest,
    RequestBuilder as RequestBuilder,
)
from .pipeline import (
    OutputRef as OutputRef,
    RetryPolicy as RetryPolicy,
    FailurePolicy as FailurePolicy,
    PipelineStep as PipelineStep,
    StepResult as StepResult,
    PipelineResult as PipelineResult,
    Pipeline as Pipeline,
    AsyncPipeline as AsyncPipeline,
)
from .planning import (
    PlanStep as PlanStep,
    ScanPlan as ScanPlan,
)
from .checkpoint import (
    Checkpoint as Checkpoint,
    CheckpointStore as CheckpointStore,
    PipelineCheckpoint as PipelineCheckpoint,
    CheckpointLoadResult as CheckpointLoadResult,
    create_checkpoint_store as create_checkpoint_store,
)
from .engine_state import DispatchAuditEvent as DispatchAuditEvent
from .config_model import (
    SensitiveString as SensitiveString,
    HttpConfig as HttpConfig,
    ScanConfig as ScanConfig,
    OutputConfig as OutputConfig,
    ReconApiConfig as ReconApiConfig,
    ReconConfig as ReconConfig,
    ProxyConfigEntry as ProxyConfigEntry,
    AllowedWorker as AllowedWorker,
    RemoteConfig as RemoteConfig,
    AiConfig as AiConfig,
    SearchConfig as SearchConfig,
    PathsConfig as PathsConfig,
    CacheConfig as CacheConfig,
    AlertChannelConfig as AlertChannelConfig,
    EggsecConfig as EggsecConfig,
)
from .scope_eval import (
    ScopeSourcePy as ScopeSourcePy,
    LoadedScopePy as LoadedScopePy,
    ScopeRulePy as ScopeRulePy,
    ScopeExplanationPy as ScopeExplanationPy,
    ScopeValidationPy as ScopeValidationPy,
    validate_scope as validate_scope,
)
from .operation_metadata import (
    OperationRiskPy as OperationRiskPy,
    OperationModePy as OperationModePy,
    IntendedUsePy as IntendedUsePy,
    CapabilityPy as CapabilityPy,
    DenialClassPy as DenialClassPy,
    TargetPolicyKindPy as TargetPolicyKindPy,
    OperationDescriptorPy as OperationDescriptorPy,
    OperationMetadataViewPy as OperationMetadataViewPy,
    OperationRegistry as OperationRegistry,
)
from .domains import (
    DomainDescriptorPy as DomainDescriptorPy,
    DomainRegistry as DomainRegistry,
)
from .execution_context import (
    ExecutionSurfacePy as ExecutionSurfacePy,
    ExecutionProfilePy as ExecutionProfilePy,
    PolicyDecisionPy as PolicyDecisionPy,
    EnforcementOutcomePy as EnforcementOutcomePy,
    ApprovedOperationPy as ApprovedOperationPy,
    EnforcementContextPy as EnforcementContextPy,
)
from .authorization import (
    ExecutionPolicyPy as ExecutionPolicyPy,
    ManualOverridePy as ManualOverridePy,
)
from .preflight import (
    PreflightResultPy as PreflightResultPy,
    preflight_operation as preflight_operation,
    preflight_with_descriptor as preflight_with_descriptor,
)
from .audit import (
    AuditOutcomePy as AuditOutcomePy,
    ManualOverrideAuditPy as ManualOverrideAuditPy,
    ScopeAuditPy as ScopeAuditPy,
    EnforcementAuditEventPy as EnforcementAuditEventPy,
    audit_event_from_enforcement as audit_event_from_enforcement,
    audit_event_from_preflight as audit_event_from_preflight,
    emit_audit_event as emit_audit_event,
)
from .runtime import PyFuture as PyFuture
from .functions import (
    features as features,
    has_feature as has_feature,
    feature_matrix as feature_matrix,
    build_info as build_info,
    api_surface_version as api_surface_version,
    scan_ports as scan_ports,
    async_scan_ports as async_scan_ports,
    scan_endpoints as scan_endpoints,
    async_scan_endpoints as async_scan_endpoints,
    fingerprint_services as fingerprint_services,
    async_fingerprint_services as async_fingerprint_services,
    recon_dns as recon_dns,
    async_recon_dns as async_recon_dns,
    inspect_tls as inspect_tls,
    async_inspect_tls as async_inspect_tls,
    detect_technology as detect_technology,
    async_detect_technology as async_detect_technology,
    detect_waf as detect_waf,
    async_detect_waf as async_detect_waf,
    # Phase F Track 1: WAF validation and HTTP fuzzing
    validate_waf as validate_waf,
    async_validate_waf as async_validate_waf,
    fuzz_http as fuzz_http,
    async_fuzz_http as async_fuzz_http,
    generate_fuzz_payloads as generate_fuzz_payloads,
    # Phase F Track 2: Load testing
    load_test_http as load_test_http,
    async_load_test_http as async_load_test_http,
)
from .waf_validation import (
    BypassResult as BypassResult,
    WafScanResult as WafScanResult,
    Payload as Payload,
    FuzzResult as FuzzResult,
    FuzzSession as FuzzSession,
    FuzzConfig as FuzzConfig,
)
from .loadtest import (
    LoadTestResult as LoadTestResult,
    LoadTestConfig as LoadTestConfig,
)
from .websocket import (
    ConnectionTestResultPy as ConnectionTestResultPy,
    InjectionTestResultPy as InjectionTestResultPy,
    OriginTestResultPy as OriginTestResultPy,
    FuzzTestResultPy as FuzzTestResultPy,
    WebSocketFindingPy as WebSocketFindingPy,
    WebSocketReportPy as WebSocketReportPy,
    WebSocketTestConfigPy as WebSocketTestConfigPy,
    websocket_probe as websocket_probe,
    async_websocket_probe as async_websocket_probe,
    websocket_fuzz as websocket_fuzz,
    async_websocket_fuzz as async_websocket_fuzz,
)
from .git_secrets import (
    Confidence as Confidence,
    SecretType as SecretType,
    SecretFindingPy as SecretFindingPy,
    GitSecretFindingPy as GitSecretFindingPy,
    GitSecretsSummaryPy as GitSecretsSummaryPy,
    GitSecretsReportPy as GitSecretsReportPy,
    scan_git_secrets as scan_git_secrets,
    async_scan_git_secrets as async_scan_git_secrets,
)
from .sbom import (
    SbomFormatPy as SbomFormatPy,
    SbomComponentPy as SbomComponentPy,
    SbomVulnerabilityPy as SbomVulnerabilityPy,
    SbomReportPy as SbomReportPy,
    generate_sbom as generate_sbom,
    async_generate_sbom as async_generate_sbom,
)
from .db_pentest import (
    DbFindingPy as DbFindingPy,
    DbPentestReportPy as DbPentestReportPy,
    DbPentestConfig as DbPentestConfig,
    db_probe as db_probe,
    async_db_probe as async_db_probe,
    db_probe_with_config as db_probe_with_config,
    db_probe_postgres as db_probe_postgres,
    db_probe_mysql as db_probe_mysql,
    db_probe_mssql as db_probe_mssql,
    db_probe_mongodb as db_probe_mongodb,
    db_probe_redis as db_probe_redis,
    DbDriverInfoPy as DbDriverInfoPy,
    DbCapabilityPy as DbCapabilityPy,
    DbCredentialProviderPy as DbCredentialProviderPy,
    DbSessionConfigPy as DbSessionConfigPy,
    db_list_drivers as db_list_drivers,
    db_get_capabilities as db_get_capabilities,
    db_run_with_config as db_run_with_config,
)
from .proxy import (
    ProxyTypePy as ProxyTypePy,
    RotationStrategyPy as RotationStrategyPy,
    ProxyConfigPy as ProxyConfigPy,
    ProxyEntryPy as ProxyEntryPy,
    HealthCheckResultPy as HealthCheckResultPy,
    ProxyHealthPy as ProxyHealthPy,
    ProxyManagerPy as ProxyManagerPy,
    create_proxy_manager as create_proxy_manager,
    async_add_proxy as async_add_proxy,
    async_proxy_health_check as async_proxy_health_check,
    InterceptConfigPy as InterceptConfigPy,
    CapturedExchangePy as CapturedExchangePy,
    InterceptSessionResultPy as InterceptSessionResultPy,
)
from .mobile import (
    MobilePlatformPy as MobilePlatformPy,
    MobileFindingPy as MobileFindingPy,
    MobileScanReportPy as MobileScanReportPy,
    analyze_apk as analyze_apk,
    async_analyze_apk as async_analyze_apk,
    analyze_ipa as analyze_ipa,
    async_analyze_ipa as async_analyze_ipa,
    MobileDevicePy as MobileDevicePy,
    DynamicMobileConfigPy as DynamicMobileConfigPy,
    DynamicMobileReportPy as DynamicMobileReportPy,
    list_mobile_devices as list_mobile_devices,
    dynamic_mobile_analysis as dynamic_mobile_analysis,
)
from .container import (
    ContainerScanTypePy as ContainerScanTypePy,
    EscapeRiskLevelPy as EscapeRiskLevelPy,
    CisCheckStatusPy as CisCheckStatusPy,
    ImageLayerPy as ImageLayerPy,
    DockerMisconfigPy as DockerMisconfigPy,
    DockerScanResultPy as DockerScanResultPy,
    ClusterInfoPy as ClusterInfoPy,
    K8sFindingPy as K8sFindingPy,
    KubernetesScanResultPy as KubernetesScanResultPy,
    EscapeRiskPy as EscapeRiskPy,
    EscapeDetectionResultPy as EscapeDetectionResultPy,
    CisCheckPy as CisCheckPy,
    CisBenchmarkResultPy as CisBenchmarkResultPy,
    ContainerFindingPy as ContainerFindingPy,
    ContainerReportPy as ContainerReportPy,
    scan_docker_image as scan_docker_image,
    async_scan_docker_image as async_scan_docker_image,
    scan_kubernetes as scan_kubernetes,
    async_scan_kubernetes as async_scan_kubernetes,
    detect_escape_risks as detect_escape_risks,
    check_cis_docker_benchmark as check_cis_docker_benchmark,
)
from .packet_inspection import (
    CaptureConfigPy as CaptureConfigPy,
    CaptureStatsPy as CaptureStatsPy,
    PacketInfoPy as PacketInfoPy,
    NetworkInterfaceInfoPy as NetworkInterfaceInfoPy,
    PcapWriterPy as PcapWriterPy,
    list_network_interfaces as list_network_interfaces,
    parse_pcap as parse_pcap,
    PacketFilterPy as PacketFilterPy,
    FlowRecordPy as FlowRecordPy,
    LiveCaptureResultPy as LiveCaptureResultPy,
    TracerouteConfigPy as TracerouteConfigPy,
    TracerouteHopPy as TracerouteHopPy,
    TracerouteResultPy as TracerouteResultPy,
    run_traceroute as run_traceroute,
    async_run_traceroute as async_run_traceroute,
    traceroute as traceroute,
)
from .stress import (
    StressTypePy as StressTypePy,
    StressConfigPy as StressConfigPy,
    StressStatsPy as StressStatsPy,
    StressConfigSummaryPy as StressConfigSummaryPy,
    StressResultPy as StressResultPy,
    stress_test as stress_test,
    async_stress_test as async_stress_test,
)
from .nse import (
    NseConfigPy as NseConfigPy,
    NseLibraryUsePy as NseLibraryUsePy,
    NseRuleEvaluationPy as NseRuleEvaluationPy,
    NseReportPy as NseReportPy,
    nse_run as nse_run,
    async_nse_run as async_nse_run,
    nse_list_libraries as nse_list_libraries,
    NseScriptMetadataPy as NseScriptMetadataPy,
    NseSandboxPolicyPy as NseSandboxPolicyPy,
    NseTargetContextPy as NseTargetContextPy,
    nse_list_scripts as nse_list_scripts,
    nse_get_script_metadata as nse_get_script_metadata,
)
from .daemon import (
    DaemonResponsePy as DaemonResponsePy,
    DaemonClientPy as DaemonClientPy,
    daemon_connect as daemon_connect,
    async_daemon_health as async_daemon_health,
    async_daemon_declare_client as async_daemon_declare_client,
    async_daemon_create_session as async_daemon_create_session,
    async_daemon_list_sessions as async_daemon_list_sessions,
    async_daemon_get_snapshot as async_daemon_get_snapshot,
    async_daemon_close_session as async_daemon_close_session,
    async_daemon_submit_task as async_daemon_submit_task,
    async_daemon_cancel_task as async_daemon_cancel_task,
    async_daemon_cancel_active as async_daemon_cancel_active,
    async_daemon_approve_policy as async_daemon_approve_policy,
    async_daemon_list_persisted_sessions as async_daemon_list_persisted_sessions,
    async_daemon_get_persisted_snapshot as async_daemon_get_persisted_snapshot,
    async_daemon_subscribe as async_daemon_subscribe,
    DaemonCapabilitiesPy as DaemonCapabilitiesPy,
    TaskHandlePy as TaskHandlePy,
    TaskStatusPy as TaskStatusPy,
    DaemonEventPy as DaemonEventPy,
    SessionSummaryPy as SessionSummaryPy,
    TransportMetadataPy as TransportMetadataPy,
)
from .wireless import (
    SecurityTypePy as SecurityTypePy,
    WirelessNetworkPy as WirelessNetworkPy,
    WirelessVulnerabilityPy as WirelessVulnerabilityPy,
    WirelessScanResultPy as WirelessScanResultPy,
    WirelessScanConfigPy as WirelessScanConfigPy,
    wireless_scan as wireless_scan,
    async_wireless_scan as async_wireless_scan,
    wireless_analyze_networks as wireless_analyze_networks,
)
from .evasion import (
    EvasionTargetTypePy as EvasionTargetTypePy,
    EvasionCategoryPy as EvasionCategoryPy,
    EvasionRiskPy as EvasionRiskPy,
    EvasionTechniquePy as EvasionTechniquePy,
    EvasionDetectionPy as EvasionDetectionPy,
    EvasionSummaryPy as EvasionSummaryPy,
    EvasionReportPy as EvasionReportPy,
    EvasionScanConfigPy as EvasionScanConfigPy,
    evasion_scan as evasion_scan,
    async_evasion_scan as async_evasion_scan,
    evasion_list_techniques as evasion_list_techniques,
)
from .postex import (
    PostexCategoryPy as PostexCategoryPy,
    PostexRiskPy as PostexRiskPy,
    PostexProfilePy as PostexProfilePy,
    PostexTechniquePy as PostexTechniquePy,
    PostexDetectionPy as PostexDetectionPy,
    PostexSummaryPy as PostexSummaryPy,
    PostexReportPy as PostexReportPy,
    PostexScanConfigPy as PostexScanConfigPy,
    postex_scan as postex_scan,
    async_postex_scan as async_postex_scan,
    postex_list_techniques as postex_list_techniques,
)
from .c2 import (
    BeaconProtocolPy as BeaconProtocolPy,
    TaskTypePy as TaskTypePy,
    TaskStatusPy as TaskStatusPy,
    OpsecCategoryPy as OpsecCategoryPy,
    OpsecSeverityPy as OpsecSeverityPy,
    CampaignPhasePy as CampaignPhasePy,
    C2CampaignPy as C2CampaignPy,
    BeaconResultPy as BeaconResultPy,
    C2TaskResultPy as C2TaskResultPy,
    OpsecFindingPy as OpsecFindingPy,
    OpsecAssessmentPy as OpsecAssessmentPy,
    C2SummaryPy as C2SummaryPy,
    C2ReportPy as C2ReportPy,
    C2ScanConfigPy as C2ScanConfigPy,
    c2_scan as c2_scan,
    async_c2_scan as async_c2_scan,
    c2_get_campaign as c2_get_campaign,
)
from .distributed import (
    DistributedTaskTypePy as DistributedTaskTypePy,
    WorkerStatusPy as WorkerStatusPy,
    WorkerRegistrationPy as WorkerRegistrationPy,
    HeartbeatPy as HeartbeatPy,
    DistributedTaskPy as DistributedTaskPy,
    DistributedTaskResultPy as DistributedTaskResultPy,
    distributed_task_types as distributed_task_types,
    distributed_generate_psk as distributed_generate_psk,
)
from .notification import (
    WebhookEventPy as WebhookEventPy,
    FindingSummaryPy as FindingSummaryPy,
    NotifyScanStatsPy as NotifyScanStatsPy,
    WebhookConfigPy as WebhookConfigPy,
    NotifyManagerPy as NotifyManagerPy,
    notify_scan_started as notify_scan_started,
    notify_scan_complete as notify_scan_complete,
    notify_findings as notify_findings,
    notify_error as notify_error,
)
from .ai_postprocess import (
    AiProviderPy as AiProviderPy,
    PluginLanguagePy as PluginLanguagePy,
    ScriptTargetPy as ScriptTargetPy,
    AiAnalysisResultPy as AiAnalysisResultPy,
    AiPayloadSuggestionPy as AiPayloadSuggestionPy,
    AiWafBypassSuggestionPy as AiWafBypassSuggestionPy,
    AiCacheStatsPy as AiCacheStatsPy,
    ScriptMetadataPy as ScriptMetadataPy,
    GeneratedScriptPy as GeneratedScriptPy,
    AiCachePy as AiCachePy,
    ai_analyze_finding as ai_analyze_finding,
    async_ai_analyze_finding as async_ai_analyze_finding,
    ai_generate_payloads as ai_generate_payloads,
    ai_suggest_waf_bypass as ai_suggest_waf_bypass,
    ai_generate_script as ai_generate_script,
)
from .event_protocol import (
    EVENT_SCHEMA_VERSION as EVENT_SCHEMA_VERSION,
    EventEnvelope as EventEnvelope,
    PlanningEvent as PlanningEvent,
    PreflightEvent as PreflightEvent,
    StageLifecycleEvent as StageLifecycleEvent,
    ProgressEvent as ProgressEvent,
    FindingEvent as FindingEvent,
    ArtifactEvent as ArtifactEvent,
    CancellationEvent as CancellationEvent,
    FailureEvent as FailureEvent,
    CompletionEvent as CompletionEvent,
    wrap_event as wrap_event,
)
from .event_stream import (
    EventStream as EventStream,
    event_stream_from_legacy as event_stream_from_legacy,
)
from .callbacks import (
    AuditSink as AuditSink,
    FindingSink as FindingSink,
    ArtifactSink as ArtifactSink,
    ProgressSink as ProgressSink,
    EventConsumer as EventConsumer,
)
from .async_support import (
    AsyncCallback as AsyncCallback,
    CallbackScheduler as CallbackScheduler,
)
from .backpressure import (
    BackpressureChannel as BackpressureChannel,
    EventDeliveryStats as EventDeliveryStats,
)
from .async_iter import (
    EventStreamAsyncIterator as EventStreamAsyncIterator,
    FindingStreamAsyncIterator as FindingStreamAsyncIterator,
)
