"""Eggsec: Python bindings for the Rust security assessment engine.

This is a host-language binding, not an internal plugin runtime.
The core engine is implemented in Rust; this package provides a Python-native API.
"""

import warnings

from . import _core

__version__ = _core.__version__
__version_info__ = _core.__version_info__

# G7: Machine-readable version constants
__schema_version__ = _core.SCHEMA_VERSION
__protocol_version__ = _core.PROTOCOL_VERSION
__abi_version__ = _core.ABI_VERSION

FINDING_SCHEMA_VERSION = _core.FINDING_SCHEMA_VERSION


def _deprecated(name: str, replacement: str | None = None) -> None:
    """Emit a DeprecationWarning for a deprecated API.

    Args:
        name: The deprecated symbol name.
        replacement: Optional name of the recommended replacement.
    """
    msg = f"{name} is deprecated"
    if replacement:
        msg += f"; use {replacement} instead"
    warnings.warn(msg, DeprecationWarning, stacklevel=3)


def api_surface() -> dict:
    """Return a machine-readable dict of all exported names, their stability
    level, and any deprecation info.

    Returns:
        Dict mapping symbol names to info dicts with keys:
        - "stability": one of "stable", "beta", "experimental"
        - "deprecated": bool
        - "deprecated_with": optional replacement name
    """
    return _core.api_surface()

# Re-export functions
features = _core.features
has_feature = _core.has_feature
feature_matrix = _core.feature_matrix
build_info = _core.build_info
api_surface_version = _core.api_surface_version
deprecated_warning = _core.deprecated_warning
scan_ports = _core.scan_ports
async_scan_ports = _core.async_scan_ports
scan_endpoints = _core.scan_endpoints
async_scan_endpoints = _core.async_scan_endpoints
fingerprint_services = _core.fingerprint_services
async_fingerprint_services = _core.async_fingerprint_services

# Phase D: Recon functions
recon_dns = _core.recon_dns
async_recon_dns = _core.async_recon_dns
inspect_tls = _core.inspect_tls
async_inspect_tls = _core.async_inspect_tls
detect_technology = _core.detect_technology
async_detect_technology = _core.async_detect_technology

# Phase D: WAF functions
detect_waf = _core.detect_waf
async_detect_waf = _core.async_detect_waf

# Phase F Track 1: WAF validation and HTTP fuzzing
validate_waf = _core.validate_waf
async_validate_waf = _core.async_validate_waf
fuzz_http = _core.fuzz_http
async_fuzz_http = _core.async_fuzz_http
generate_fuzz_payloads = _core.generate_fuzz_payloads

# Phase F Track 2: Load testing
load_test_http = _core.load_test_http
async_load_test_http = _core.async_load_test_http

# Phase F Track 3: WebSocket testing
try:
    websocket_probe = _core.websocket_probe
    async_websocket_probe = _core.async_websocket_probe
    websocket_fuzz = _core.websocket_fuzz
    async_websocket_fuzz = _core.async_websocket_fuzz
except (AttributeError, ImportError):
    pass

# Release 2: Network Programmability

# Network types
try:
    from ._core import TargetPy, ResolvedTargetPy, ConnectionConfigPy, TimeoutConfigPy
    from ._core import RetryPolicyPy, SocketEndpointPy, ConnectionTimingPy
    from ._core import ConnectionMetadataPy, NetworkEvidencePy
    from ._core import TranscriptEntryPy, NetworkTranscriptPy
    from ._core import resolve_target_sync, async_resolve_target
    # WS10: Evidence-to-finding conversion
    from ._core import evidence_to_finding
except (AttributeError, ImportError):
    pass

# Transport (TCP/UDP sessions)
try:
    from ._core import TcpConfigPy, TcpSessionPy, TcpConnectResultPy
    from ._core import TcpReadResultPy, TcpWriteResultPy
    from ._core import UdpConfigPy, UdpSocketPy, UdpSendResultPy
    from ._core import UdpRecvResultPy, UdpRecvFromResultPy
    from ._core import BannerProbeResultPy
    # Release 2: Async transport sessions
    from ._core import AsyncTcpSessionPy, AsyncUdpSocketPy
    from ._core import tcp_connect_probe, async_tcp_connect_probe
    from ._core import banner_probe, async_banner_probe
except (AttributeError, ImportError):
    pass

# Protocol probes
try:
    from ._core import DnsQueryConfigPy, DnsRecordPy, DnsQueryResultPy
    from ._core import TlsProbeConfigPy, CertificateInfoPy, CertificateChainEntryPy
    from ._core import TlsProbeResultPy, TlsIssuePy
    from ._core import HttpProbeConfigPy, HttpProbeResultPy
    from ._core import UdpProbeConfigPy, UdpProbeResultPy
    from ._core import dns_query, async_dns_query
    from ._core import tls_probe, async_tls_probe
    from ._core import http_probe, async_http_probe
    from ._core import udp_probe, async_udp_probe
except (AttributeError, ImportError):
    pass

# HTTP client
try:
    from ._core import HttpRequestPy, HttpHeadersPy, HttpResponsePy
    from ._core import HttpCookiePy, RedirectEntryPy, TlsMetadataPy, HttpTimingPy
    from ._core import HttpClientConfigPy, HttpClientPy, AsyncHttpClientPy
    from ._core import RedactConfigPy
    from ._core import create_http_client, async_create_http_client
except (AttributeError, ImportError):
    pass

# WebSocket sessions (feature-gated)
try:
    from ._core import WebSocketSessionConfigPy, WebSocketMessagePy
    from ._core import WebSocketFramePy, WebSocketCloseInfoPy, WebSocketHandshakePy
    from ._core import WebSocketSessionPy, AsyncWebSocketSessionPy
    from ._core import WebSocketAssessmentConfigPy, WebSocketAssessmentResultPy
    from ._core import websocket_assess, async_websocket_assess
except (AttributeError, ImportError):
    pass

# Phase F Track 4: Git secrets
try:
    scan_git_secrets = _core.scan_git_secrets
    async_scan_git_secrets = _core.async_scan_git_secrets
except (AttributeError, ImportError):
    pass

# Phase F Track 5: SBOM
try:
    generate_sbom = _core.generate_sbom
    async_generate_sbom = _core.async_generate_sbom
except (AttributeError, ImportError):
    pass

# Phase F Track 6: Database pentesting
try:
    db_probe = _core.db_probe
    async_db_probe = _core.async_db_probe
    db_probe_with_config = _core.db_probe_with_config
    db_probe_postgres = _core.db_probe_postgres
    db_probe_mysql = _core.db_probe_mysql
    db_probe_mssql = _core.db_probe_mssql
    db_probe_mongodb = _core.db_probe_mongodb
    db_probe_redis = _core.db_probe_redis
    # D7: Database extensibility functions
    db_list_drivers = _core.db_list_drivers
    db_get_capabilities = _core.db_get_capabilities
    db_run_with_config = _core.db_run_with_config
except (AttributeError, ImportError):
    pass

# Phase F Track 7: Proxy
try:
    create_proxy_manager = _core.create_proxy_manager
    async_add_proxy = _core.async_add_proxy
    async_proxy_health_check = _core.async_proxy_health_check
except (AttributeError, ImportError):
    pass

# Phase F Track 8: Mobile
try:
    analyze_apk = _core.analyze_apk
    async_analyze_apk = _core.async_analyze_apk
    analyze_ipa = _core.analyze_ipa
    async_analyze_ipa = _core.async_analyze_ipa
    # D5: Mobile dynamic functions
    list_mobile_devices = _core.list_mobile_devices
    dynamic_mobile_analysis = _core.dynamic_mobile_analysis
except (AttributeError, ImportError):
    pass

# Phase F Track 9: Container
try:
    scan_docker_image = _core.scan_docker_image
    async_scan_docker_image = _core.async_scan_docker_image
    scan_kubernetes = _core.scan_kubernetes
    async_scan_kubernetes = _core.async_scan_kubernetes
    detect_escape_risks = _core.detect_escape_risks
    check_cis_docker_benchmark = _core.check_cis_docker_benchmark
except (AttributeError, ImportError):
    pass

# Phase F Track 10: Packet inspection
try:
    list_network_interfaces = _core.list_network_interfaces
    parse_pcap = _core.parse_pcap
    # D3: Network probing functions
    run_traceroute = _core.run_traceroute
    async_run_traceroute = _core.async_run_traceroute
    traceroute = _core.traceroute
    # WS9: Active probe functions
    icmp_probe = _core.icmp_probe
    async_icmp_probe = _core.async_icmp_probe
    tcp_syn_probe = _core.tcp_syn_probe
    async_tcp_syn_probe = _core.async_tcp_syn_probe
except (AttributeError, ImportError):
    pass

# Phase F Track 11: Stress testing
try:
    stress_test = _core.stress_test
    async_stress_test = _core.async_stress_test
except (AttributeError, ImportError):
    pass

# Phase F Track 12: NSE
try:
    nse_run = _core.nse_run
    async_nse_run = _core.async_nse_run
    nse_list_libraries = _core.nse_list_libraries
    # D1: NSE runtime completion functions
    nse_list_scripts = _core.nse_list_scripts
    nse_get_script_metadata = _core.nse_get_script_metadata
    # Release 3: NSE library registry and execution improvements
    nse_list_libraries_detailed = _core.nse_list_libraries_detailed
    nse_get_library_descriptor = _core.nse_get_library_descriptor
    nse_run_with_config = _core.nse_run_with_config
    nse_validate_script = _core.nse_validate_script
except (AttributeError, ImportError):
    pass

# Phase F Track 13: Daemon client
try:
    daemon_connect = _core.daemon_connect
    async_daemon_health = _core.async_daemon_health
    async_daemon_declare_client = _core.async_daemon_declare_client
    async_daemon_create_session = _core.async_daemon_create_session
    async_daemon_list_sessions = _core.async_daemon_list_sessions
    async_daemon_get_snapshot = _core.async_daemon_get_snapshot
    async_daemon_close_session = _core.async_daemon_close_session
    async_daemon_submit_task = _core.async_daemon_submit_task
    async_daemon_cancel_task = _core.async_daemon_cancel_task
    async_daemon_cancel_active = _core.async_daemon_cancel_active
    async_daemon_approve_policy = _core.async_daemon_approve_policy
    async_daemon_list_persisted_sessions = _core.async_daemon_list_persisted_sessions
    async_daemon_get_persisted_snapshot = _core.async_daemon_get_persisted_snapshot
    async_daemon_subscribe = _core.async_daemon_subscribe
except (AttributeError, ImportError):
    pass

# Milestone C: Consolidated Reconnaissance
run_consolidated_recon = _core.run_consolidated_recon
async_run_consolidated_recon = _core.async_run_consolidated_recon
ConsolidatedReconConfig = _core.ConsolidatedReconConfigPy
ReconModuleResult = _core.ReconModuleResultPy
ConsolidatedReconReport = _core.ConsolidatedReconReportPy

# Milestone C: GraphQL Security Assessment
graphql_test = _core.graphql_test
async_graphql_test = _core.async_graphql_test
GraphQLVulnerability = _core.GraphQLVulnerabilityPy
GraphQLTestResult = _core.GraphQLTestResultPy
GraphQLType = _core.GraphQLTypePy
GraphQLField = _core.GraphQLFieldPy
GraphQLArg = _core.GraphQLArgPy
GraphQLInputField = _core.GraphQLInputFieldPy
GraphQLSchema = _core.GraphQLSchemaPy
GraphQLTestConfig = _core.GraphQLTestConfigPy

# Milestone C: OAuth/OIDC Assessment
oauth_discover_endpoints = _core.oauth_discover_endpoints
oauth_test = _core.oauth_test
async_oauth_test = _core.async_oauth_test
OAuthVulnerability = _core.OAuthVulnerabilityPy
OAuthEndpointKind = _core.OAuthEndpointKindPy
OAuthEndpoint = _core.OAuthEndpointPy
OAuthTestResult = _core.OAuthTestResultPy
OAuthTestConfig = _core.OAuthTestConfigPy

# Milestone C: Authentication Assessment
auth_test = _core.auth_test
async_auth_test = _core.async_auth_test
AuthTestType = _core.AuthTestTypePy
AuthFinding = _core.AuthFindingPy
AuthTestConfig = _core.AuthTestConfigPy
AuthTestReport = _core.AuthTestReportPy

# Milestone C: Headless Browser Assessment (feature-gated: headless-browser)
try:
    browser_test = _core.browser_test
    async_browser_test = _core.async_browser_test
    XssSource = _core.XssSourcePy
    XssSink = _core.XssSinkPy
    DomXssFinding = _core.DomXssFindingPy
    DiscoveryMethod = _core.DiscoveryMethodPy
    SpaRoute = _core.SpaRoutePy
    ClientIssueType = _core.ClientIssueTypePy
    ClientIssue = _core.ClientIssuePy
    BrowserTestConfig = _core.BrowserTestConfigPy
    BrowserTestReport = _core.BrowserTestReportPy
except (AttributeError, ImportError):
    pass

# Milestone C: Advanced Hunting (feature-gated: advanced-hunting)
try:
    hunt_test = _core.hunt_test
    async_hunt_test = _core.async_hunt_test
    ChainType = _core.ChainTypePy
    ChainStep = _core.ChainStepPy
    AttackChain = _core.AttackChainPy
    FlawType = _core.FlawTypePy
    BusinessLogicFlaw = _core.BusinessLogicFlawPy
    RaceType = _core.RaceTypePy
    RaceCondition = _core.RaceConditionPy
    BypassType = _core.BypassTypePy
    AuthzBypass = _core.AuthzBypassPy
    SessionIssueType = _core.SessionIssueTypePy
    SessionIssue = _core.SessionIssuePy
    HuntTestConfig = _core.HuntTestConfigPy
    HuntReport = _core.HuntReportPy
except (AttributeError, ImportError):
    pass

# Milestone B: Configuration, Policy, and Execution Context
SensitiveString = _core.SensitiveString
HttpConfig = _core.HttpConfig
ScanConfig = _core.ScanConfig
OutputConfig = _core.OutputConfig
ReconApiConfig = _core.ReconApiConfig
ReconConfig = _core.ReconConfig
ProxyConfigEntry = _core.ProxyConfigEntry
AllowedWorker = _core.AllowedWorker
RemoteConfig = _core.RemoteConfig
AiConfig = _core.AiConfig
SearchConfig = _core.SearchConfig
PathsConfig = _core.PathsConfig
CacheConfig = _core.CacheConfig
AlertChannelConfig = _core.AlertChannelConfig
EggsecConfig = _core.EggsecConfig
ScopeSource = _core.ScopeSource
LoadedScope = _core.LoadedScope
ScopeRule = _core.ScopeRule
ScopeExplanation = _core.ScopeExplanation
ScopeValidation = _core.ScopeValidation
OperationRisk = _core.OperationRisk
OperationMode = _core.OperationMode
IntendedUse = _core.IntendedUse
Capability = _core.Capability
DenialClass = _core.DenialClass
TargetPolicyKind = _core.TargetPolicyKind
OperationDescriptor = _core.OperationDescriptor
OperationDescriptorPy = _core.OperationDescriptorPy
OperationMetadataView = _core.OperationMetadataView
OperationRegistry = _core.OperationRegistry
# G1: Domain descriptors
DomainDescriptor = _core.DomainDescriptorPy
DomainRegistry = _core.DomainRegistry
domain_maturity = _core.domain_maturity
ExecutionSurface = _core.ExecutionSurfacePy
ExecutionProfile = _core.ExecutionProfilePy
PolicyDecision = _core.PolicyDecisionPy
EnforcementOutcome = _core.EnforcementOutcomePy
ApprovedOperation = _core.ApprovedOperationPy
EnforcementContext = _core.EnforcementContext
ExecutionPolicy = _core.ExecutionPolicyPy
ManualOverride = _core.ManualOverridePy
PreflightResult = _core.PreflightResultPy
AuditOutcome = _core.AuditOutcomePy
ManualOverrideAudit = _core.ManualOverrideAuditPy
ScopeAudit = _core.ScopeAuditPy
EnforcementAuditEvent = _core.EnforcementAuditEventPy
validate_scope = _core.validate_scope
preflight_operation = _core.preflight_operation
preflight_with_descriptor = _core.preflight_with_descriptor
audit_event_from_enforcement = _core.audit_event_from_enforcement
audit_event_from_preflight = _core.audit_event_from_preflight
emit_audit_event = _core.emit_audit_event

# Re-export classes
Scope = _core.Scope
Client = _core.Client
AsyncClient = _core.AsyncClient
Engine = _core.Engine
AsyncEngine = _core.AsyncEngine
ExecutionHandle = _core.ExecutionHandle
ExecutionEvent = _core.ExecutionEvent
EventLog = _core.EventLog
ExecutionState = _core.ExecutionState
TrackedExecutionHandle = _core.TrackedExecutionHandle
CancellationToken = _core.CancellationToken
PyFuture = _core.PyFuture
PortScanResult = _core.PortScanResult
OpenPort = _core.OpenPort
ScanStats = _core.ScanStats
PortRange = _core.PortRange
TimingPreset = _core.TimingPreset
EndpointScanConfig = _core.EndpointScanConfig
EndpointFinding = _core.EndpointFinding
EndpointScanStats = _core.EndpointScanStats
EndpointScanResult = _core.EndpointScanResult
FingerprintEvidence = _core.FingerprintEvidence
FingerprintConfidence = _core.FingerprintConfidence
ServiceFingerprintResult = _core.ServiceFingerprintResult
FingerprintScanResult = _core.FingerprintScanResult

# Phase D: Findings and reporting
Severity = _core.Severity
Evidence = _core.Evidence
Finding = _core.Finding
FindingSet = _core.FindingSet
Report = _core.Report

# Phase A3: Common result protocol types
ExecutionStatus = _core.ExecutionStatus
ExecutionStats = _core.ExecutionStats
Artifact = _core.Artifact
OperationResult = _core.OperationResult
OperationError = _core.OperationError
DispatchAuditEvent = _core.DispatchAuditEvent

# Phase D: Recon types
DnsRecordSet = _core.DnsRecordSet
MxRecord = _core.MxRecord
SoaRecord = _core.SoaRecord
TlsCertificateInfo = _core.TlsCertificateInfo
TlsInspectionResult = _core.TlsInspectionResult
SslIssue = _core.SslIssue
TechStack = _core.TechStack
TechDetectionResult = _core.TechDetectionResult

# Phase D: WAF detection
WafDetectionResult = _core.WafDetectionResultPy

# Operation request types
OperationRequest = _core.OperationRequest
PortScanRequest = _core.PortScanRequest
EndpointScanRequest = _core.EndpointScanRequest
FingerprintRequest = _core.FingerprintRequest
ReconDnsRequest = _core.ReconDnsRequest
TlsInspectRequest = _core.TlsInspectRequest
TechDetectRequest = _core.TechDetectRequest
WafDetectRequest = _core.WafDetectRequest
LoadTestRequest = _core.LoadTestRequest
WafValidateRequest = _core.WafValidateRequest
FuzzRequest = _core.FuzzRequest
RequestBuilder = _core.RequestBuilder

# Pipeline and assessment types
PipelineStep = _core.PipelineStep
StepResult = _core.StepResult
PipelineResult = _core.PipelineResult
Pipeline = _core.Pipeline
AsyncPipeline = _core.AsyncPipeline
FailurePolicy = _core.FailurePolicy

# Planning types
PlanStep = _core.PlanStep
ScanPlan = _core.ScanPlan

# Checkpoint types
Checkpoint = _core.Checkpoint
CheckpointStore = _core.CheckpointStore
PipelineCheckpoint = _core.PipelineCheckpoint
CheckpointLoadResult = _core.CheckpointLoadResult
create_checkpoint_store = _core.create_checkpoint_store

# Phase F Track 1: WAF validation and HTTP fuzzing
BypassResult = _core.BypassResultPy
WafScanResult = _core.WafScanResultPy
Payload = _core.PayloadPy
FuzzResult = _core.FuzzResultPy
FuzzSession = _core.FuzzSessionPy
FuzzConfig = _core.FuzzConfig

# Phase F Track 2: Load testing
LoadTestResult = _core.LoadTestResultPy
LoadTestConfig = _core.LoadTestConfig

# Phase F Track 3: WebSocket (feature-gated)
try:
    WebSocketReport = _core.WebSocketReportPy
    WebSocketFinding = _core.WebSocketFindingPy
    ConnectionTestResult = _core.ConnectionTestResultPy
    InjectionTestResult = _core.InjectionTestResultPy
    OriginTestResult = _core.OriginTestResultPy
    FuzzTestResult = _core.FuzzTestResultPy
    WebSocketTestConfig = _core.WebSocketTestConfigPy
except (AttributeError, ImportError):
    pass

# Phase F Track 4: Git secrets (feature-gated)
try:
    GitSecretsReport = _core.GitSecretsReportPy
    GitSecretsSummary = _core.GitSecretsSummaryPy
    GitSecretFinding = _core.GitSecretFindingPy
    SecretFinding = _core.SecretFindingPy
except (AttributeError, ImportError):
    pass

# Phase F Track 5: SBOM (feature-gated)
try:
    SbomReport = _core.SbomReportPy
    SbomComponent = _core.SbomComponentPy
    SbomVulnerability = _core.SbomVulnerabilityPy
    SbomFormat = _core.SbomFormatPy
except (AttributeError, ImportError):
    pass

# Phase F Track 6: Database pentesting (feature-gated)
try:
    DbPentestReport = _core.DbPentestReportPy
    DbFinding = _core.DbFindingPy
    DbPentestConfig = _core.DbPentestConfig
    # D7: Database extensibility
    DbDriverInfo = _core.DbDriverInfoPy
    DbCapability = _core.DbCapabilityPy
    DbCredentialProvider = _core.DbCredentialProviderPy
    DbSessionConfig = _core.DbSessionConfigPy
    # WS16: Driver registry and target
    DbDriverRegistry = _core.DbDriverRegistryPy
    DbTarget = _core.DbTargetPy
    # WS17-18: Session types
    DatabaseSessionState = _core.DatabaseSessionStatePy
    DatabaseConnectionMetadata = _core.DatabaseConnectionMetadataPy
    DatabaseSessionStats = _core.DatabaseSessionStatsPy
    DatabaseCredentialRequest = _core.DatabaseCredentialRequestPy
    DatabaseCredentialResult = _core.DatabaseCredentialResultPy
    # WS19: Query types
    DatabaseQuery = _core.DatabaseQueryPy
    DatabaseQueryResult = _core.DatabaseQueryResultPy
    DatabaseColumn = _core.DatabaseColumnPy
    # WS20: Schema/privilege inspection
    DatabaseTableInfo = _core.DatabaseTableInfoPy
    DatabaseSchemaInfo = _core.DatabaseSchemaInfoPy
    DatabasePrivilegeInfo = _core.DatabasePrivilegeInfoPy
    # Release 3 WS18: Credential provider variants
    StaticCredentialProvider = _core.StaticCredentialProviderPy
    EnvironmentCredentialProvider = _core.EnvironmentCredentialProviderPy
    CallbackCredentialProvider = _core.CallbackCredentialProviderPy
    # Release 3 WS19: Row stream and query plan
    DatabaseRowStream = _core.DatabaseRowStreamPy
    DatabaseQueryPlan = _core.DatabaseQueryPlanPy
    # Release 3 WS20: Index and extension metadata
    DatabaseIndexInfo = _core.DatabaseIndexInfoPy
    DatabaseExtensionInfo = _core.DatabaseExtensionInfoPy
except (AttributeError, ImportError):
    pass

# Phase F Track 7: Proxy (feature-gated)
try:
    ProxyType = _core.ProxyTypePy
    RotationStrategy = _core.RotationStrategyPy
    ProxyConfig = _core.ProxyConfigPy
    ProxyEntry = _core.ProxyEntryPy
    ProxyManager = _core.ProxyManagerPy
    HealthCheckResult = _core.HealthCheckResultPy
    ProxyHealth = _core.ProxyHealthPy
    # D4: Interception proxy
    InterceptConfig = _core.InterceptConfigPy
    CapturedExchange = _core.CapturedExchangePy
    InterceptSessionResult = _core.InterceptSessionResultPy
    # Release 3: Interception proxy lifecycle and DTOs
    InterceptSessionState = _core.InterceptSessionStatePy
    InterceptStats = _core.InterceptStatsPy
    InterceptFilter = _core.InterceptFilterPy
    InterceptRule = _core.InterceptRulePy
    CertificateAuthorityConfig = _core.CertificateAuthorityConfigPy
    IssuedCertificate = _core.IssuedCertificatePy
    HarEntry = _core.HarEntryPy
    HarDocument = _core.HarDocumentPy
    # Release 3 WS12: Mutation decision/error
    MutationDecision = _core.MutationDecisionPy
    MutationError = _core.MutationErrorPy
    # Release 3 WS13: CA and cert store
    CertificateAuthority = _core.CertificateAuthorityPy
    CertificateStore = _core.CertificateStorePy
    # Release 3 WS14: Replay and comparison
    ReplayRequest = _core.ReplayRequestPy
    ReplayResult = _core.ReplayResultPy
    ResponseComparison = _core.ResponseComparisonPy
    ComparisonRule = _core.ComparisonRulePy
    # Release 3: Interception session functions
    run_intercept_session = _core.run_intercept_session
    async_run_intercept_session = _core.async_run_intercept_session
except (AttributeError, ImportError):
    pass

# Phase F Track 8: Mobile (feature-gated)
try:
    MobilePlatform = _core.MobilePlatformPy
    MobileFinding = _core.MobileFindingPy
    MobileScanReport = _core.MobileScanReportPy
    # D5: Mobile dynamic
    MobileDevice = _core.MobileDevicePy
    DynamicMobileConfig = _core.DynamicMobileConfigPy
    DynamicMobileReport = _core.DynamicMobileReportPy
except (AttributeError, ImportError):
    pass

# Phase F Track 9: Container (feature-gated)
try:
    ContainerScanType = _core.ContainerScanTypePy
    EscapeRiskLevel = _core.EscapeRiskLevelPy
    CisCheckStatus = _core.CisCheckStatusPy
    DockerScanResult = _core.DockerScanResultPy
    KubernetesScanResult = _core.KubernetesScanResultPy
    EscapeDetectionResult = _core.EscapeDetectionResultPy
    CisBenchmarkResult = _core.CisBenchmarkResultPy
    ContainerFinding = _core.ContainerFindingPy
    ContainerReport = _core.ContainerReportPy
except (AttributeError, ImportError):
    pass

# Phase F Track 10: Packet inspection (feature-gated)
try:
    CaptureConfig = _core.CaptureConfigPy
    CaptureStats = _core.CaptureStatsPy
    PacketInfo = _core.PacketInfoPy
    NetworkInterfaceInfo = _core.NetworkInterfaceInfoPy
    PcapWriter = _core.PcapWriterPy
    # D2: Live packet inspection
    PacketFilter = _core.PacketFilterPy
    FlowRecord = _core.FlowRecordPy
    LiveCaptureResult = _core.LiveCaptureResultPy
    # D3: Network probing
    TracerouteConfig = _core.TracerouteConfigPy
    TracerouteHop = _core.TracerouteHopPy
    TracerouteResult = _core.TracerouteResultPy
    # WS7: Managed capture lifecycle
    BackpressurePolicy = _core.BackpressurePolicyPy
    CaptureDropStats = _core.CaptureDropStatsPy
    CapturedPacket = _core.CapturedPacketPy
    AsyncCaptureSession = _core.AsyncCaptureSessionPy
    # WS8: Packet layer DTOs
    EthernetFrame = _core.EthernetFramePy
    Ipv4Packet = _core.Ipv4PacketPy
    Ipv6Packet = _core.Ipv6PacketPy
    TcpSegment = _core.TcpSegmentPy
    UdpDatagram = _core.UdpDatagramPy
    IcmpPacket = _core.IcmpPacketPy
    FlowKey = _core.FlowKeyPy
    FlowAggregator = _core.FlowAggregatorPy
    # WS9: Active probe types
    IcmpProbeConfig = _core.IcmpProbeConfigPy
    IcmpProbeReply = _core.IcmpProbeReplyPy
    IcmpProbeResult = _core.IcmpProbeResultPy
    TcpProbeConfig = _core.TcpProbeConfigPy
    TcpProbeResult = _core.TcpProbeResultPy
    # WS7: Additional capture types
    PacketTimestampPy = _core.PacketTimestampPy
    PacketStreamPy = _core.PacketStreamPy
    PacketArtifactPy = _core.PacketArtifactPy
    SyncCaptureSessionPy = _core.SyncCaptureSessionPy
    # WS8: Additional layer DTOs
    DnsPacketPy = _core.DnsPacketPy
    TlsRecordInfoPy = _core.TlsRecordInfoPy
    # WS9: UDP reachability
    UdpReachabilityConfigPy = _core.UdpReachabilityConfigPy
    UdpReachabilityResultPy = _core.UdpReachabilityResultPy
    udp_reachability = _core.udp_reachability
except (AttributeError, ImportError):
    pass

# Phase F Track 11: Stress testing (feature-gated)
try:
    StressType = _core.StressTypePy
    StressConfig = _core.StressConfigPy
    StressStats = _core.StressStatsPy
    StressResult = _core.StressResultPy
except (AttributeError, ImportError):
    pass

# Phase F Track 12: NSE (feature-gated)
try:
    NseConfig = _core.NseConfigPy
    NseReport = _core.NseReportPy
    NseLibraryUse = _core.NseLibraryUsePy
    NseRuleEvaluation = _core.NseRuleEvaluationPy
    # D1: NSE runtime completion
    NseScriptMetadata = _core.NseScriptMetadataPy
    NseSandboxPolicy = _core.NseSandboxPolicyPy
    NseTargetContext = _core.NseTargetContextPy
    # Release 3: NSE library registry and descriptors
    NseLibraryDescriptor = _core.NseLibraryDescriptorPy
    NseArgument = _core.NseArgumentPy
    NseLibraryRegistry = _core.NseLibraryRegistryPy
    NseEvidenceItem = _core.NseEvidenceItemPy
    # Release 3 WS2: Runtime lifecycle
    NseExecutionLimits = _core.NseExecutionLimitsPy
    NseCancellationToken = _core.NseCancellationTokenPy
    NseRuntimeStats = _core.NseRuntimeStatsPy
    NseRuntimeConfig = _core.NseRuntimeConfigPy
    NseRuntime = _core.NseRuntimePy
    # Release 3 WS3: Script inspection
    NseScriptSource = _core.NseScriptSourcePy
    NseDiagnostic = _core.NseDiagnosticPy
    # Release 3 WS2: Capability context
    NseCapabilityContext = _core.NseCapabilityContextPy
    # Release 3 WS4: Rule evaluation
    NseHostContext = _core.NseHostContextPy
    NsePortContext = _core.NsePortContextPy
    NseRuleResult = _core.NseRuleResultPy
    # Release 3 WS6: Library version and conflicts
    NseLibraryVersion = _core.NseLibraryVersionPy
    NseLibraryConflict = _core.NseLibraryConflictPy
    # Release 3 WS7: Execution request/result types
    NseExecutionRequest = _core.NseExecutionRequestPy
    NseExecutionResult = _core.NseExecutionResultPy
    NseScriptResult = _core.NseScriptResultPy
    NseOutputValue = _core.NseOutputValuePy
except (AttributeError, ImportError):
    pass

# Phase F Track 13: Daemon client (feature-gated)
try:
    DaemonClient = _core.DaemonClientPy
    DaemonResponse = _core.DaemonResponsePy
    # D6: Daemon task API
    DaemonCapabilities = _core.DaemonCapabilitiesPy
    TaskHandle = _core.TaskHandlePy
    TaskStatus = _core.TaskStatusPy
    DaemonEvent = _core.DaemonEventPy
    SessionSummary = _core.SessionSummaryPy
    TransportMetadata = _core.TransportMetadataPy
except (AttributeError, ImportError):
    pass

# Milestone E: Versioned finding schema
FINDING_SCHEMA_VERSION = _core.FINDING_SCHEMA_VERSION
Confidence = _core.Confidence
FindingType = _core.FindingType
EvidenceKind = _core.EvidenceKind
AffectedAsset = _core.AffectedAsset
FindingLocation = _core.FindingLocation
VersionedEvidence = _core.VersionedEvidence
VersionedFinding = _core.VersionedFinding

# Milestone E: Evidence and artifact model
MilestoneArtifact = _core.MilestoneArtifact
ArtifactReference = _core.ArtifactReference
ArtifactStore = _core.ArtifactStore

# Milestone E: CVSS and vulnerability records
CvssScore = _core.CvssScore
VulnerabilityRecord = _core.VulnerabilityRecord
RemediationRecord = _core.RemediationRecord

# Milestone E: Finding workflow
FindingState = _core.FindingState
WorkflowTransition = _core.WorkflowTransition
Suppression = _core.Suppression
FindingWorkflow = _core.FindingWorkflow

# Milestone E: Repository abstraction
FindingRepository = _core.FindingRepository
Assessment = _core.Assessment
AssessmentRepository = _core.AssessmentRepository

# Milestone E: Baselines and comparisons
FindingCorrelation = _core.FindingCorrelation
FindingDiff = _core.FindingDiff
AssessmentDiff = _core.AssessmentDiff
BaselineComparator = _core.BaselineComparator

# Milestone E: Reporting
FindingReporter = _core.FindingReporter
SeveritySummary = _core.SeveritySummary
ReportEnvelope = _core.ReportEnvelope

# Milestone E: External integrations
IntegrationType = _core.IntegrationType
PublicationRecord = _core.PublicationRecord
RetryPolicy = _core.RetryPolicy
PublicationPolicy = _core.PublicationPolicy
ExternalIntegration = _core.ExternalIntegration

# Milestone E: Migration and compatibility
SchemaVersion = _core.SchemaVersion
MigrationResult = _core.MigrationResult
FindingMigration = _core.FindingMigration

# Milestone E: Compliance mapping (feature-gated)
try:
    ComplianceFramework = _core.ComplianceFramework
    ComplianceControl = _core.ComplianceControl
    ComplianceMapping = _core.ComplianceMapping
    ComplianceResult = _core.ComplianceResult
    ControlAssessment = _core.ControlAssessment
    ComplianceReport = _core.ComplianceReport
    ComplianceMapper = _core.ComplianceMapper
except (AttributeError, ImportError):
    pass

# Milestone F: Wireless assessment (feature-gated)
try:
    SecurityType = _core.SecurityTypePy
    WirelessNetwork = _core.WirelessNetworkPy
    WirelessVulnerability = _core.WirelessVulnerabilityPy
    WirelessScanResult = _core.WirelessScanResultPy
    WirelessScanConfig = _core.WirelessScanConfigPy
    wireless_scan = _core.wireless_scan
    async_wireless_scan = _core.async_wireless_scan
    wireless_analyze_networks = _core.wireless_analyze_networks
except (AttributeError, ImportError):
    pass

# Milestone F: Evasion validation (feature-gated)
try:
    EvasionTargetType = _core.EvasionTargetTypePy
    EvasionCategory = _core.EvasionCategoryPy
    EvasionRisk = _core.EvasionRiskPy
    EvasionTechnique = _core.EvasionTechniquePy
    EvasionDetection = _core.EvasionDetectionPy
    EvasionSummary = _core.EvasionSummaryPy
    EvasionReport = _core.EvasionReportPy
    EvasionScanConfig = _core.EvasionScanConfigPy
    evasion_scan = _core.evasion_scan
    async_evasion_scan = _core.async_evasion_scan
    evasion_list_techniques = _core.evasion_list_techniques
except (AttributeError, ImportError):
    pass

# Milestone F: Post-exploitation simulation (feature-gated)
try:
    PostexCategory = _core.PostexCategoryPy
    PostexRisk = _core.PostexRiskPy
    PostexProfile = _core.PostexProfilePy
    PostexTechnique = _core.PostexTechniquePy
    PostexDetection = _core.PostexDetectionPy
    PostexSummary = _core.PostexSummaryPy
    PostexReport = _core.PostexReportPy
    PostexScanConfig = _core.PostexScanConfigPy
    postex_scan = _core.postex_scan
    async_postex_scan = _core.async_postex_scan
    postex_list_techniques = _core.postex_list_techniques
except (AttributeError, ImportError):
    pass

# Milestone F: C2 simulation (feature-gated)
try:
    BeaconProtocol = _core.BeaconProtocolPy
    C2TaskType = _core.TaskTypePy
    C2TaskStatus = _core.TaskStatusPy
    OpsecCategory = _core.OpsecCategoryPy
    OpsecSeverity = _core.OpsecSeverityPy
    CampaignPhase = _core.CampaignPhasePy
    C2Campaign = _core.C2CampaignPy
    BeaconResult = _core.BeaconResultPy
    C2TaskResult = _core.C2TaskResultPy
    OpsecFinding = _core.OpsecFindingPy
    OpsecAssessment = _core.OpsecAssessmentPy
    C2Summary = _core.C2SummaryPy
    C2Report = _core.C2ReportPy
    C2ScanConfig = _core.C2ScanConfigPy
    c2_scan = _core.c2_scan
    async_c2_scan = _core.async_c2_scan
    c2_get_campaign = _core.c2_get_campaign
except (AttributeError, ImportError):
    pass

# Milestone F: Distributed scanning (always-available)
DistributedTaskType = _core.DistributedTaskTypePy
WorkerStatus = _core.WorkerStatusPy
WorkerRegistration = _core.WorkerRegistrationPy
Heartbeat = _core.HeartbeatPy
DistributedTask = _core.DistributedTaskPy
DistributedTaskResult = _core.DistributedTaskResultPy
distributed_task_types = _core.distributed_task_types
distributed_generate_psk = _core.distributed_generate_psk

# Milestone F: Notifications (always-available)
WebhookEvent = _core.WebhookEventPy
FindingSummary = _core.FindingSummaryPy
NotifyScanStats = _core.NotifyScanStatsPy
WebhookConfig = _core.WebhookConfigPy
NotifyManager = _core.NotifyManagerPy
notify_scan_started = _core.notify_scan_started
notify_scan_complete = _core.notify_scan_complete
notify_findings = _core.notify_findings
notify_error = _core.notify_error

# G2: Event protocol stabilization (always-available)
EVENT_SCHEMA_VERSION = _core.EVENT_SCHEMA_VERSION
EventEnvelope = _core.EventEnvelope
PlanningEvent = _core.PlanningEvent
PreflightEvent = _core.PreflightEvent
StageLifecycleEvent = _core.StageLifecycleEvent
ProgressEvent = _core.ProgressEvent
FindingEventPy = _core.FindingEvent
ArtifactEventPy = _core.ArtifactEvent
CancellationEvent = _core.CancellationEvent
FailureEvent = _core.FailureEvent
CompletionEvent = _core.CompletionEvent
# WS11: Network-specific events
ResolutionEvent = _core.ResolutionEvent
ConnectionEvent = _core.ConnectionEvent
ProbeEvent = _core.ProbeEvent
WebSocketMessageEvent = _core.WebSocketMessageEvent
CaptureStatsEvent = _core.CaptureStatsEvent
# WS11: Additional network events
HandshakeCompletedEvent = _core.HandshakeCompletedEvent
RequestSentEvent = _core.RequestSentEvent
ResponseHeadersReceivedEvent = _core.ResponseHeadersReceivedEvent
BodyProgressEvent = _core.BodyProgressEvent
CaptureStartedEvent = _core.CaptureStartedEvent
PacketSampledEvent = _core.PacketSampledEvent
FlowObservedEvent = _core.FlowObservedEvent
ArtifactCreatedEvent = _core.ArtifactCreatedEvent
wrap_event = _core.wrap_event
EventStream = _core.EventStream
event_stream_from_legacy = _core.event_stream_from_legacy

# G4: Async iterators
EventStreamAsyncIterator = _core.EventStreamAsyncIterator
FindingStreamAsyncIterator = _core.FindingStreamAsyncIterator

# G3: Callbacks and sinks (always-available)
AuditSink = _core.AuditSink
FindingSink = _core.FindingSink
ArtifactSink = _core.ArtifactSink
ProgressSink = _core.ProgressSink
EventConsumer = _core.EventConsumer
AsyncCallback = _core.AsyncCallback
CallbackScheduler = _core.CallbackScheduler
BackpressureChannel = _core.PyBackpressureChannel
EventDeliveryStats = _core.EventDeliveryStats

# Milestone F: AI post-processing (feature-gated)
try:
    AiProvider = _core.AiProviderPy
    PluginLanguage = _core.PluginLanguagePy
    ScriptTarget = _core.ScriptTargetPy
    AiAnalysisResult = _core.AiAnalysisResultPy
    AiPayloadSuggestion = _core.AiPayloadSuggestionPy
    AiWafBypassSuggestion = _core.AiWafBypassSuggestionPy
    AiCacheStats = _core.AiCacheStatsPy
    ScriptMetadata = _core.ScriptMetadataPy
    GeneratedScript = _core.GeneratedScriptPy
    AiCache = _core.AiCachePy
    ai_analyze_finding = _core.ai_analyze_finding
    async_ai_analyze_finding = _core.async_ai_analyze_finding
    ai_generate_payloads = _core.ai_generate_payloads
    ai_suggest_waf_bypass = _core.ai_suggest_waf_bypass
    ai_generate_script = _core.ai_generate_script
except (AttributeError, ImportError):
    pass

# Release 4: Common managed-session contract (WS1) (always available)
SessionState = _core.SessionState
SessionIdentity = _core.SessionIdentity
SessionStats = _core.SessionStats
SessionCloseMode = _core.SessionCloseMode
SessionEvent = _core.SessionEvent
SessionEventStream = _core.SessionEventStream
SessionCapabilities = _core.SessionCapabilities
create_session_event = _core.create_session_event

# Release 4: Mobile session lifecycle (WS2-6) (feature-gated: mobile)
try:
    MobileDeviceDescriptor = _core.MobileDeviceDescriptor
    MobileDeviceCapabilities = _core.MobileDeviceCapabilities
    MobileSessionConfig = _core.MobileSessionConfig
    MobileSessionState = _core.MobileSessionState
    MobileSessionStats = _core.MobileSessionStats
    MobileSession = _core.MobileSession
    AsyncMobileSession = _core.AsyncMobileSession
    MobileDeviceRegistry = _core.MobileDeviceRegistry
    # WS4-6: Mobile convergence, instrumentation, evidence
    StaticAnalysisSummary = _core.StaticAnalysisSummary
    AnalysisTarget = _core.AnalysisTarget
    DynamicAnalysisPlan = _core.DynamicAnalysisPlan
    InstrumentationConfig = _core.InstrumentationConfig
    InstrumentationScript = _core.InstrumentationScript
    InstrumentationEvent = _core.InstrumentationEvent
    InstrumentationResult = _core.InstrumentationResult
    MobileEvidenceKind = _core.MobileEvidenceKind
    MobileEvidence = _core.MobileEvidence
    MobileEvidenceCollection = _core.MobileEvidenceCollection
except (AttributeError, ImportError):
    pass

# Release 4: Browser session lifecycle (WS7-11) (feature-gated: headless-browser)
try:
    BrowserCapabilities = _core.BrowserCapabilities
    BrowserSessionState = _core.BrowserSessionState
    BrowserSessionConfig = _core.BrowserSessionConfig
    BrowserSessionStats = _core.BrowserSessionStats
    BrowserNavigationEvent = _core.BrowserNavigationEvent
    BrowserConsoleEvent = _core.BrowserConsoleEvent
    BrowserNetworkEvent = _core.BrowserNetworkEvent
    BrowserDomSnapshot = _core.BrowserDomSnapshot
    BrowserFormInfo = _core.BrowserFormInfo
    BrowserFormField = _core.BrowserFormField
    BrowserLinkInfo = _core.BrowserLinkInfo
    BrowserStorageInfo = _core.BrowserStorageInfo
    BrowserCookieInfo = _core.BrowserCookieInfo
    BrowserSession = _core.BrowserSession
    AsyncBrowserSession = _core.AsyncBrowserSession
    # WS10: Browser event types
    BrowserDomEvent = _core.BrowserDomEvent
    BrowserDownloadEvent = _core.BrowserDownloadEvent
    BrowserSecurityObservation = _core.BrowserSecurityObservation
except (AttributeError, ImportError):
    pass

# Release 4: Daemon parity types (WS12-18) (always available)
DaemonProtocolVersion = _core.DaemonProtocolVersion
IdempotencyKey = _core.IdempotencyKey
DaemonSubmissionResult = _core.DaemonSubmissionResult
ReconnectOptions = _core.ReconnectOptions
ReplayCursor = _core.ReplayCursor
ReplayResult = _core.ReplayResult
DaemonEventPy = _core.DaemonEventPy
CancellationRequest = _core.CancellationRequest
CancellationResult = _core.CancellationResult
TaskArtifactDescriptor = _core.TaskArtifactDescriptor
EventReplayInfo = _core.EventReplayInfo
DaemonHealthDetail = _core.DaemonHealthDetail

# Release 4: SQLite repository (WS20-22) (always available)
SqliteFindingRepository = _core.SqliteFindingRepository
SqliteAssessmentRepository = _core.SqliteAssessmentRepository
SqliteMigration = _core.SqliteMigration
SqliteMigrationResult = _core.SqliteMigrationResult

# Release 4: JSONL repository (WS22) (always available)
JsonlFindingRepository = _core.JsonlFindingRepository
JsonlAssessmentRepository = _core.JsonlAssessmentRepository

# Release 4: Content-addressed artifact store (WS23) (always available)
ContentAddressedArtifactStore = _core.ContentAddressedArtifactStore
DirectoryArtifactStore = _core.DirectoryArtifactStore
ArtifactInfo = _core.ArtifactInfo
ArtifactData = _core.ArtifactData
IntegrityResult = _core.IntegrityResult
ArtifactQuery = _core.ArtifactQuery

# Release 4: Streaming reporting (WS26-27) (always available)
StreamingReportConfig = _core.StreamingReportConfig
StreamingReporter = _core.StreamingReporter
ReportSummary = _core.ReportSummary
StreamingDiffReporter = _core.StreamingDiffReporter
FindingDiffResult = _core.FindingDiffResult
DiffReportSummary = _core.DiffReportSummary
ReportManifest = _core.ReportManifest

# Release 5: Tool-core types (eggsec-tool-core bindings)
try:
    from ._core import (
        # Enums (aliased to avoid conflicts with existing types)
        TargetTypePy as ToolTargetType,
        AuthTypePy as ToolAuthType,
        ResponseTypePy as ToolResponseType,
        ToolFindingType as ToolFindingType,
        ToolSeverity as ToolSeverity,
        ToolErrorTypePy as ToolErrorType,
        PortStatePy as ToolPortState,
        StreamEventTypePy as ToolStreamEventType,
        # Structs
        ScopeToolPy as ToolScope,
        ToolTarget as ToolTarget,
        RequestOptionsPy as ToolRequestOptions,
        AuthConfigPy as ToolAuthConfig,
        ToolRequestPy as ToolRequest,
        ResponseMetadataPy as ToolResponseMetadata,
        ToolFindingPy as ToolFinding,
        ToolErrorPy as ToolError,
        ToolResponsePy as ToolResponse,
        ProgressUpdatePy as ToolProgressUpdate,
        StreamEventPy as ToolStreamEvent,
        PortDataPy as ToolPortData,
        EndpointDataPy as ToolEndpointData,
        TechnologyDataPy as ToolTechnologyData,
        RateLimitConfigPy as ToolRateLimitConfig,
        RateLimitStatusPy as ToolRateLimitStatus,
        ExecutionEntryPy as ToolExecutionEntry,
        # Descriptors and registry
        ToolDescriptor as ToolDescriptor,
        ToolRegistry as ToolRegistry,
        OperationToolView as OperationToolView,
        ValidationReport as ValidationReport,
        SchemaGenerator as SchemaGenerator,
        operation_as_tool as operation_as_tool,
    )
except (AttributeError, ImportError):
    pass

# Re-export exceptions
EggsecError = _core.EggsecError
ConfigError = _core.ConfigError
ScopeError = _core.ScopeError
EnforcementError = _core.EnforcementError
NetworkError = _core.NetworkError
ScanError = _core.ScanError
TimeoutError = _core.TimeoutError
FeatureUnavailableError = _core.FeatureUnavailableError
SerializationError = _core.SerializationError
InternalError = _core.InternalError
CancellationError = _core.CancellationError

# G6: Experimental namespace (subpackage for unstable APIs)
from . import experimental  # noqa: E402

__all__ = [
    "__version__",
    "__version_info__",
    "__schema_version__",
    "__protocol_version__",
    "__abi_version__",
    "FINDING_SCHEMA_VERSION",
    # G6/G7 introspection
    "api_surface",
    "api_surface_version",
    "_deprecated",
    "deprecated_warning",
    # Functions
    "features",
    "has_feature",
    "feature_matrix",
    "build_info",
    "scan_ports",
    "async_scan_ports",
    "scan_endpoints",
    "async_scan_endpoints",
    "fingerprint_services",
    "async_fingerprint_services",
    # Phase D: Recon functions
    "recon_dns",
    "async_recon_dns",
    "inspect_tls",
    "async_inspect_tls",
    "detect_technology",
    "async_detect_technology",
    # Phase D: WAF functions
    "detect_waf",
    "async_detect_waf",
    # Phase F Track 1: WAF validation and HTTP fuzzing
    "validate_waf",
    "async_validate_waf",
    "fuzz_http",
    "async_fuzz_http",
    "generate_fuzz_payloads",
    # Phase F Track 2: Load testing
    "load_test_http",
    "async_load_test_http",
    # Classes
    "Scope",
    "Client",
    "AsyncClient",
    "Engine",
    "AsyncEngine",
    "ExecutionHandle",
    "ExecutionEvent",
    "EventLog",
    "ExecutionState",
    "TrackedExecutionHandle",
    "CancellationToken",
    "PyFuture",
    "PortScanResult",
    "OpenPort",
    "ScanStats",
    "PortRange",
    "TimingPreset",
    "EndpointScanConfig",
    "EndpointFinding",
    "EndpointScanStats",
    "EndpointScanResult",
    "FingerprintEvidence",
    "FingerprintConfidence",
    "ServiceFingerprintResult",
    "FingerprintScanResult",
    # Phase D: Findings and reporting
    "Severity",
    "Evidence",
    "Finding",
    "FindingSet",
    "Report",
    # Phase A3: Common result protocol types
    "ExecutionStatus",
    "ExecutionStats",
    "Artifact",
    "OperationResult",
    "OperationError",
    "DispatchAuditEvent",
    # Phase D: Recon types
    "DnsRecordSet",
    "MxRecord",
    "SoaRecord",
    "TlsCertificateInfo",
    "TlsInspectionResult",
    "SslIssue",
    "TechStack",
    "TechDetectionResult",
    # Phase D: WAF detection
    "WafDetectionResult",
    # Operation request types
    "OperationRequest",
    "PortScanRequest",
    "EndpointScanRequest",
    "FingerprintRequest",
    "ReconDnsRequest",
    "TlsInspectRequest",
    "TechDetectRequest",
    "WafDetectRequest",
    "LoadTestRequest",
    "WafValidateRequest",
    "FuzzRequest",
    "RequestBuilder",
    # Pipeline and assessment types
    "PipelineStep",
    "StepResult",
    "PipelineResult",
    "Pipeline",
    "AsyncPipeline",
    "FailurePolicy",
    # Planning types
    "PlanStep",
    "ScanPlan",
    # Checkpoint types
    "Checkpoint",
    "CheckpointStore",
    "PipelineCheckpoint",
    "CheckpointLoadResult",
    "create_checkpoint_store",
    # Phase F Track 1: WAF validation and HTTP fuzzing classes
    "BypassResult",
    "WafScanResult",
    "Payload",
    "FuzzResult",
    "FuzzSession",
    "FuzzConfig",
    # Phase F Track 2: Load testing classes
    "LoadTestResult",
    "LoadTestConfig",
    # Milestone B: Configuration, Policy, and Execution Context
    "SensitiveString",
    "HttpConfig",
    "ScanConfig",
    "OutputConfig",
    "ReconApiConfig",
    "ReconConfig",
    "ProxyConfigEntry",
    "AllowedWorker",
    "RemoteConfig",
    "AiConfig",
    "SearchConfig",
    "PathsConfig",
    "CacheConfig",
    "AlertChannelConfig",
    "EggsecConfig",
    "ScopeSource",
    "LoadedScope",
    "ScopeRule",
    "ScopeExplanation",
    "ScopeValidation",
    "OperationRisk",
    "OperationMode",
    "IntendedUse",
    "Capability",
    "DenialClass",
    "TargetPolicyKind",
    "OperationDescriptor",
    "OperationDescriptorPy",
    "OperationMetadataView",
    "OperationRegistry",
    "DomainDescriptor",
    "DomainRegistry",
    "domain_maturity",
    "ExecutionSurface",
    "ExecutionProfile",
    "PolicyDecision",
    "EnforcementOutcome",
    "ApprovedOperation",
    "EnforcementContext",
    "ExecutionPolicy",
    "ManualOverride",
    "PreflightResult",
    "AuditOutcome",
    "ManualOverrideAudit",
    "ScopeAudit",
    "EnforcementAuditEvent",
    "validate_scope",
    "preflight_operation",
    "preflight_with_descriptor",
    "audit_event_from_enforcement",
    "audit_event_from_preflight",
    "emit_audit_event",
    # Milestone C: Consolidated Recon
    "run_consolidated_recon",
    "async_run_consolidated_recon",
    "ConsolidatedReconConfig",
    "ReconModuleResult",
    "ConsolidatedReconReport",
    # Milestone C: GraphQL
    "graphql_test",
    "async_graphql_test",
    "GraphQLVulnerability",
    "GraphQLTestResult",
    "GraphQLType",
    "GraphQLField",
    "GraphQLArg",
    "GraphQLInputField",
    "GraphQLSchema",
    "GraphQLTestConfig",
    # Milestone C: OAuth/OIDC
    "oauth_discover_endpoints",
    "oauth_test",
    "async_oauth_test",
    "OAuthVulnerability",
    "OAuthEndpointKind",
    "OAuthEndpoint",
    "OAuthTestResult",
    "OAuthTestConfig",
    # Milestone C: Auth Assessment
    "auth_test",
    "async_auth_test",
    "AuthTestType",
    "AuthFinding",
    "AuthTestConfig",
    "AuthTestReport",
    # Milestone C: Headless Browser (feature-gated)
    "browser_test",
    "async_browser_test",
    "XssSource",
    "XssSink",
    "DomXssFinding",
    "DiscoveryMethod",
    "SpaRoute",
    "ClientIssueType",
    "ClientIssue",
    "BrowserTestConfig",
    "BrowserTestReport",
    # Milestone C: Advanced Hunting (feature-gated)
    "hunt_test",
    "async_hunt_test",
    "ChainType",
    "ChainStep",
    "AttackChain",
    "FlawType",
    "BusinessLogicFlaw",
    "RaceType",
    "RaceCondition",
    "BypassType",
    "AuthzBypass",
    "SessionIssueType",
    "SessionIssue",
    "HuntTestConfig",
    "HuntReport",
    # Milestone D: NSE runtime completion (feature-gated)
    "NseScriptMetadata",
    "NseSandboxPolicy",
    "NseTargetContext",
    "nse_list_scripts",
    "nse_get_script_metadata",
    # Release 3: NSE library registry (feature-gated)
    "NseLibraryDescriptor",
    "NseArgument",
    "NseLibraryRegistry",
    "NseEvidenceItem",
    "NseCapabilityContext",
    "NseLibraryVersion",
    "NseLibraryConflict",
    "NseExecutionRequest",
    "NseExecutionResult",
    "NseScriptResult",
    "NseOutputValue",
    "nse_list_libraries_detailed",
    "nse_get_library_descriptor",
    "nse_run_with_config",
    "nse_validate_script",
    # Milestone D: Live packet inspection (feature-gated)
    "PacketFilter",
    "FlowRecord",
    "LiveCaptureResult",
    # Milestone D: Network probing (feature-gated)
    "TracerouteConfig",
    "TracerouteHop",
    "TracerouteResult",
    "run_traceroute",
    "async_run_traceroute",
    "traceroute",
    # WS7: Additional capture types (feature-gated)
    "PacketTimestampPy",
    "PacketStreamPy",
    "PacketArtifactPy",
    "SyncCaptureSessionPy",
    # WS8: Additional layer DTOs (feature-gated)
    "DnsPacketPy",
    "TlsRecordInfoPy",
    # WS9: UDP reachability (feature-gated)
    "UdpReachabilityConfigPy",
    "UdpReachabilityResultPy",
    "udp_reachability",
    # Milestone D: Interception proxy (feature-gated)
    "InterceptConfig",
    "CapturedExchange",
    "InterceptSessionResult",
    # Release 3: Interception proxy lifecycle and DTOs (feature-gated)
    "InterceptSessionState",
    "InterceptStats",
    "InterceptFilter",
    "InterceptRule",
    "CertificateAuthorityConfig",
    "IssuedCertificate",
    "HarEntry",
    "HarDocument",
    "MutationDecision",
    "MutationError",
    "CertificateAuthority",
    "CertificateStore",
    "ReplayRequest",
    "ReplayResult",
    "ResponseComparison",
    "ComparisonRule",
    "run_intercept_session",
    "async_run_intercept_session",
    # Milestone D: Mobile dynamic (feature-gated)
    "MobileDevice",
    "DynamicMobileConfig",
    "DynamicMobileReport",
    "list_mobile_devices",
    "dynamic_mobile_analysis",
    # Milestone D: Daemon task API (feature-gated)
    "DaemonCapabilities",
    "TaskHandle",
    "TaskStatus",
    "DaemonEvent",
    "SessionSummary",
    "TransportMetadata",
    # Milestone D: Database extensibility (feature-gated)
    "DbDriverInfo",
    "DbCapability",
    "DbCredentialProvider",
    "DbSessionConfig",
    "db_list_drivers",
    "db_get_capabilities",
    "db_run_with_config",
    # WS16: Driver registry and target
    "DbDriverRegistry",
    "DbTarget",
    # WS17-18: Session types
    "DatabaseSessionState",
    "DatabaseConnectionMetadata",
    "DatabaseSessionStats",
    "DatabaseCredentialRequest",
    "DatabaseCredentialResult",
    # WS19: Query types
    "DatabaseQuery",
    "DatabaseQueryResult",
    "DatabaseColumn",
    # WS20: Schema/privilege inspection
    "DatabaseTableInfo",
    "DatabaseSchemaInfo",
    "DatabasePrivilegeInfo",
    # Release 3: DB credential providers (feature-gated)
    "StaticCredentialProvider",
    "EnvironmentCredentialProvider",
    "CallbackCredentialProvider",
    # Release 3: DB row stream and query plan (feature-gated)
    "DatabaseRowStream",
    "DatabaseQueryPlan",
    # Release 3: DB index and extension metadata (feature-gated)
    "DatabaseIndexInfo",
    "DatabaseExtensionInfo",
    # Milestone E: Versioned finding schema
    "FINDING_SCHEMA_VERSION",
    "Confidence",
    "FindingType",
    "EvidenceKind",
    "AffectedAsset",
    "FindingLocation",
    "VersionedEvidence",
    "VersionedFinding",
    # Milestone E: Evidence and artifact model
    "MilestoneArtifact",
    "ArtifactReference",
    "ArtifactStore",
    # Milestone E: CVSS and vulnerability records
    "CvssScore",
    "VulnerabilityRecord",
    "RemediationRecord",
    # Milestone E: Finding workflow
    "FindingState",
    "WorkflowTransition",
    "Suppression",
    "FindingWorkflow",
    # Milestone E: Repository abstraction
    "FindingRepository",
    "Assessment",
    "AssessmentRepository",
    # Milestone E: Baselines and comparisons
    "FindingCorrelation",
    "FindingDiff",
    "AssessmentDiff",
    "BaselineComparator",
    # Milestone E: Reporting
    "FindingReporter",
    "SeveritySummary",
    "ReportEnvelope",
    # Milestone E: External integrations
    "IntegrationType",
    "PublicationRecord",
    "RetryPolicy",
    "PublicationPolicy",
    "ExternalIntegration",
    # Milestone E: Migration and compatibility
    "SchemaVersion",
    "MigrationResult",
    "FindingMigration",
    # Milestone E: Compliance mapping (feature-gated)
    "ComplianceFramework",
    "ComplianceControl",
    "ComplianceMapping",
    "ComplianceResult",
    "ControlAssessment",
    "ComplianceReport",
    "ComplianceMapper",
    # Exceptions
    "EggsecError",
    "ConfigError",
    "ScopeError",
    "EnforcementError",
    "NetworkError",
    "ScanError",
    "TimeoutError",
    "FeatureUnavailableError",
    "SerializationError",
    "InternalError",
    "CancellationError",
    # G2: Event protocol
    "EVENT_SCHEMA_VERSION",
    "EventEnvelope",
    "PlanningEvent",
    "PreflightEvent",
    "StageLifecycleEvent",
    "ProgressEvent",
    "FindingEventPy",
    "ArtifactEventPy",
    "CancellationEvent",
    "FailureEvent",
    "CompletionEvent",
    "HandshakeCompletedEvent",
    "RequestSentEvent",
    "ResponseHeadersReceivedEvent",
    "BodyProgressEvent",
    "CaptureStartedEvent",
    "PacketSampledEvent",
    "FlowObservedEvent",
    "ArtifactCreatedEvent",
    "wrap_event",
    "EventStream",
    "event_stream_from_legacy",
    # G3: Callbacks and sinks
    "AuditSink",
    "FindingSink",
    "ArtifactSink",
    "ProgressSink",
    "EventConsumer",
    "AsyncCallback",
    "CallbackScheduler",
    "BackpressureChannel",
    "EventDeliveryStats",
    # Release 2: Network Programmability - Network types
    "TargetPy",
    "ResolvedTargetPy",
    "ConnectionConfigPy",
    "TimeoutConfigPy",
    "RetryPolicyPy",
    "SocketEndpointPy",
    "ConnectionTimingPy",
    "ConnectionMetadataPy",
    "NetworkEvidencePy",
    "TranscriptEntryPy",
    "NetworkTranscriptPy",
    "resolve_target_sync",
    "async_resolve_target",
    # Release 2: Transport (TCP/UDP sessions)
    "TcpConfigPy",
    "TcpSessionPy",
    "TcpConnectResultPy",
    "TcpReadResultPy",
    "TcpWriteResultPy",
    "UdpConfigPy",
    "UdpSocketPy",
    "UdpSendResultPy",
    "UdpRecvResultPy",
    "UdpRecvFromResultPy",
    "BannerProbeResultPy",
    "tcp_connect_probe",
    "async_tcp_connect_probe",
    "banner_probe",
    "async_banner_probe",
    # Release 2: Protocol probes
    "DnsQueryConfigPy",
    "DnsRecordPy",
    "DnsQueryResultPy",
    "TlsProbeConfigPy",
    "CertificateInfoPy",
    "CertificateChainEntryPy",
    "TlsProbeResultPy",
    "TlsIssuePy",
    "HttpProbeConfigPy",
    "HttpProbeResultPy",
    "UdpProbeConfigPy",
    "UdpProbeResultPy",
    "dns_query",
    "async_dns_query",
    "tls_probe",
    "async_tls_probe",
    "http_probe",
    "async_http_probe",
    "udp_probe",
    "async_udp_probe",
    # Release 2: HTTP client
    "HttpRequestPy",
    "HttpHeadersPy",
    "HttpResponsePy",
    "HttpCookiePy",
    "RedirectEntryPy",
    "TlsMetadataPy",
    "HttpTimingPy",
    "HttpClientConfigPy",
    "HttpClientPy",
    "AsyncHttpClientPy",
    "RedactConfigPy",
    "create_http_client",
    "async_create_http_client",
    # Release 2: WebSocket sessions (feature-gated)
    "WebSocketSessionConfigPy",
    "WebSocketMessagePy",
    "WebSocketFramePy",
    "WebSocketCloseInfoPy",
    "WebSocketHandshakePy",
    "WebSocketSessionPy",
    "AsyncWebSocketSessionPy",
    "AsyncTcpSessionPy",
    "AsyncUdpSocketPy",
    "WebSocketAssessmentConfigPy",
    "WebSocketAssessmentResultPy",
    "websocket_assess",
    "async_websocket_assess",
    # Release 4: Common managed-session contract (WS1)
    "SessionState",
    "SessionIdentity",
    "SessionStats",
    "SessionCloseMode",
    "SessionEvent",
    "SessionEventStream",
    "SessionCapabilities",
    "create_session_event",
    # Release 4: Mobile session lifecycle (WS2-6) (feature-gated)
    "MobileDeviceDescriptor",
    "MobileDeviceCapabilities",
    "MobileSessionConfig",
    "MobileSessionState",
    "MobileSessionStats",
    "MobileSession",
    "AsyncMobileSession",
    "MobileDeviceRegistry",
    # Release 4: Mobile convergence, instrumentation, evidence (WS4-6) (feature-gated)
    "StaticAnalysisSummary",
    "AnalysisTarget",
    "DynamicAnalysisPlan",
    "InstrumentationConfig",
    "InstrumentationScript",
    "InstrumentationEvent",
    "InstrumentationResult",
    "MobileEvidenceKind",
    "MobileEvidence",
    "MobileEvidenceCollection",
    # Release 4: Browser session lifecycle (WS7-11) (feature-gated)
    "BrowserCapabilities",
    "BrowserSessionState",
    "BrowserSessionConfig",
    "BrowserSessionStats",
    "BrowserNavigationEvent",
    "BrowserConsoleEvent",
    "BrowserNetworkEvent",
    "BrowserDomSnapshot",
    "BrowserFormInfo",
    "BrowserFormField",
    "BrowserLinkInfo",
    "BrowserStorageInfo",
    "BrowserCookieInfo",
    "BrowserSession",
    "AsyncBrowserSession",
    # Release 4: Browser event types (WS10) (feature-gated)
    "BrowserDomEvent",
    "BrowserDownloadEvent",
    "BrowserSecurityObservation",
    # Release 4: Daemon parity types (WS12-18)
    "DaemonProtocolVersion",
    "IdempotencyKey",
    "DaemonSubmissionResult",
    "ReconnectOptions",
    "ReplayCursor",
    "ReplayResult",
    "DaemonEventPy",
    "CancellationRequest",
    "CancellationResult",
    "TaskArtifactDescriptor",
    "EventReplayInfo",
    "DaemonHealthDetail",
    # Release 4: SQLite repository (WS20-22)
    "SqliteFindingRepository",
    "SqliteAssessmentRepository",
    "SqliteMigration",
    "SqliteMigrationResult",
    # Release 4: JSONL repository (WS22)
    "JsonlFindingRepository",
    "JsonlAssessmentRepository",
    # Release 4: Content-addressed artifact store (WS23)
    "ContentAddressedArtifactStore",
    "DirectoryArtifactStore",
    "ArtifactInfo",
    "ArtifactData",
    "IntegrityResult",
    "ArtifactQuery",
    # Release 4: Streaming reporting (WS26-27)
    "StreamingReportConfig",
    "StreamingReporter",
    "ReportSummary",
    "StreamingDiffReporter",
    "FindingDiffResult",
    "DiffReportSummary",
    "ReportManifest",
    # Release 5: Tool-core types
    "ToolTargetType",
    "ToolAuthType",
    "ToolResponseType",
    "ToolFindingType",
    "ToolSeverity",
    "ToolErrorType",
    "ToolPortState",
    "ToolStreamEventType",
    "ToolScope",
    "ToolTarget",
    "ToolRequestOptions",
    "ToolAuthConfig",
    "ToolRequest",
    "ToolResponseMetadata",
    "ToolFinding",
    "ToolError",
    "ToolResponse",
    "ToolProgressUpdate",
    "ToolStreamEvent",
    "ToolPortData",
    "ToolEndpointData",
    "ToolTechnologyData",
    "ToolRateLimitConfig",
    "ToolRateLimitStatus",
    "ToolExecutionEntry",
    "ToolDescriptor",
    "ToolRegistry",
    "OperationToolView",
    "ValidationReport",
    "SchemaGenerator",
    "operation_as_tool",
]

# Keep the runtime export contract truthful for feature-gated builds. The
# compatibility list above documents every supported symbol, while this
# filter removes names whose optional extension was not compiled in.
for _name in tuple(__all__):
    if _name not in globals():
        __all__.remove(_name)
