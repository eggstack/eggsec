"""Eggsec: Python bindings for the Rust security assessment engine.

This is a host-language binding, not an internal plugin runtime.
The core engine is implemented in Rust; this package provides a Python-native API.

Package structure (Phase C):
    eggsec              - Stable core: engine, operations, scope, config, events
    eggsec.net          - Network programmability (provisional)
    eggsec.sessions     - Managed session types (provisional)
    eggsec.storage      - Repositories and artifact stores (provisional)
    eggsec.reporting    - Reporters and output formats (provisional)
    eggsec.daemon       - Daemon client (provisional)
    eggsec.experimental - Unstable capabilities (experimental)
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
EVENT_SCHEMA_VERSION = _core.EVENT_SCHEMA_VERSION


# ---------------------------------------------------------------------------
# Deprecation helpers
# ---------------------------------------------------------------------------

def _deprecated(name: str, replacement: str | None = None) -> None:
    """Emit a DeprecationWarning for a deprecated API."""
    msg = f"{name} is deprecated"
    if replacement:
        msg += f"; use {replacement} instead"
    warnings.warn(msg, DeprecationWarning, stacklevel=3)


# ---------------------------------------------------------------------------
# Feature guard: structured errors for unavailable capabilities
# ---------------------------------------------------------------------------

_UNAVAILABLE_FEATURES: dict[str, dict[str, str]] = {}


def _register_unavailable(name: str, feature: str, maturity: str = "experimental",
                          install_hint: str = "", platform_prereqs: str = "") -> None:
    """Register that a feature-gated symbol is unavailable."""
    _UNAVAILABLE_FEATURES[name] = {
        "feature": feature,
        "maturity": maturity,
        "install_hint": install_hint,
        "platform_prereqs": platform_prereqs,
    }


def _unavailable_error(name: str) -> None:
    """Raise AttributeError with structured message for unavailable feature.

    Raises AttributeError (not ImportError) so that hasattr() returns False
    and the structured error message is preserved.
    """
    if name in _UNAVAILABLE_FEATURES:
        info = _UNAVAILABLE_FEATURES[name]
        parts = [f"'{name}' requires the '{info['feature']}' feature"]
        parts.append("Current profile: default")
        if info.get("maturity"):
            parts.append(f"Maturity: {info['maturity']}")
        if info.get("install_hint"):
            parts.append(f"Install: {info['install_hint']}")
        if info.get("platform_prereqs"):
            parts.append(f"Platform: {info['platform_prereqs']}")
        raise AttributeError(". ".join(parts))
    raise AttributeError(f"'{name}' is not available in the current build configuration.")


def list_unavailable_features() -> list[dict[str, str]]:
    """List features that are not available in the current build.

    Returns:
        List of dicts with feature info for unavailable capabilities.
    """
    return [
        {"symbol": name, **info}
        for name, info in _UNAVAILABLE_FEATURES.items()
    ]


# Canonical naming aliases: Py-suffixed names -> clean names.
# Accessing the old name emits a deprecation warning.
_DEPRECATED_ALIASES: dict[str, tuple[str, str]] = {
    # Network types
    "TargetPy": ("Target", "eggsec.net.Target"),
    "ResolvedTargetPy": ("ResolvedTarget", "eggsec.net.ResolvedTarget"),
    "ConnectionConfigPy": ("ConnectionConfig", "eggsec.net.ConnectionConfig"),
    "TimeoutConfigPy": ("TimeoutConfig", "eggsec.net.TimeoutConfig"),
    "RetryPolicyPy": ("RetryPolicy", "eggsec.net.RetryPolicy"),
    "SocketEndpointPy": ("SocketEndpoint", "eggsec.net.SocketEndpoint"),
    "ConnectionTimingPy": ("ConnectionTiming", "eggsec.net.ConnectionTiming"),
    "ConnectionMetadataPy": ("ConnectionMetadata", "eggsec.net.ConnectionMetadata"),
    "NetworkEvidencePy": ("NetworkEvidence", "eggsec.net.NetworkEvidence"),
    "TranscriptEntryPy": ("TranscriptEntry", "eggsec.net.TranscriptEntry"),
    "NetworkTranscriptPy": ("NetworkTranscript", "eggsec.net.NetworkTranscript"),
    # Transport
    "TcpConfigPy": ("TcpConfig", "eggsec.net.TcpConfig"),
    "TcpSessionPy": ("TcpSession", "eggsec.net.TcpSession"),
    "TcpConnectResultPy": ("TcpConnectResult", "eggsec.net.TcpConnectResult"),
    "TcpReadResultPy": ("TcpReadResult", "eggsec.net.TcpReadResult"),
    "TcpWriteResultPy": ("TcpWriteResult", "eggsec.net.TcpWriteResult"),
    "UdpConfigPy": ("UdpConfig", "eggsec.net.UdpConfig"),
    "UdpSocketPy": ("UdpSocket", "eggsec.net.UdpSocket"),
    "UdpSendResultPy": ("UdpSendResult", "eggsec.net.UdpSendResult"),
    "UdpRecvResultPy": ("UdpRecvResult", "eggsec.net.UdpRecvResult"),
    "UdpRecvFromResultPy": ("UdpRecvFromResult", "eggsec.net.UdpRecvFromResult"),
    "BannerProbeResultPy": ("BannerProbeResult", "eggsec.net.BannerProbeResult"),
    "AsyncTcpSessionPy": ("AsyncTcpSession", "eggsec.net.AsyncTcpSession"),
    "AsyncUdpSocketPy": ("AsyncUdpSocket", "eggsec.net.AsyncUdpSocket"),
    # Probes
    "DnsQueryConfigPy": ("DnsQueryConfig", "eggsec.net.DnsQueryConfig"),
    "DnsRecordPy": ("DnsRecord", "eggsec.net.DnsRecord"),
    "DnsQueryResultPy": ("DnsQueryResult", "eggsec.net.DnsQueryResult"),
    "TlsProbeConfigPy": ("TlsProbeConfig", "eggsec.net.TlsProbeConfig"),
    "CertificateInfoPy": ("CertificateInfo", "eggsec.net.CertificateInfo"),
    "CertificateChainEntryPy": ("CertificateChainEntry", "eggsec.net.CertificateChainEntry"),
    "TlsProbeResultPy": ("TlsProbeResult", "eggsec.net.TlsProbeResult"),
    "TlsIssuePy": ("TlsIssue", "eggsec.net.TlsIssue"),
    "HttpProbeConfigPy": ("HttpProbeConfig", "eggsec.net.HttpProbeConfig"),
    "HttpProbeResultPy": ("HttpProbeResult", "eggsec.net.HttpProbeResult"),
    "UdpProbeConfigPy": ("UdpProbeConfig", "eggsec.net.UdpProbeConfig"),
    "UdpProbeResultPy": ("UdpProbeResult", "eggsec.net.UdpProbeResult"),
    # HTTP client
    "HttpRequestPy": ("HttpRequest", "eggsec.net.HttpRequest"),
    "HttpHeadersPy": ("HttpHeaders", "eggsec.net.HttpHeaders"),
    "HttpResponsePy": ("HttpResponse", "eggsec.net.HttpResponse"),
    "HttpCookiePy": ("HttpCookie", "eggsec.net.HttpCookie"),
    "RedirectEntryPy": ("RedirectEntry", "eggsec.net.RedirectEntry"),
    "TlsMetadataPy": ("TlsMetadata", "eggsec.net.TlsMetadata"),
    "HttpTimingPy": ("HttpTiming", "eggsec.net.HttpTiming"),
    "HttpClientConfigPy": ("HttpClientConfig", "eggsec.net.HttpClientConfig"),
    "HttpClientPy": ("HttpClient", "eggsec.net.HttpClient"),
    "AsyncHttpClientPy": ("AsyncHttpClient", "eggsec.net.AsyncHttpClient"),
    "RedactConfigPy": ("RedactConfig", "eggsec.net.RedactConfig"),
    # WebSocket
    "WebSocketSessionConfigPy": ("WebSocketSessionConfig", "eggsec.net.WebSocketSessionConfig"),
    "WebSocketMessagePy": ("WebSocketMessage", "eggsec.net.WebSocketMessage"),
    "WebSocketFramePy": ("WebSocketFrame", "eggsec.net.WebSocketFrame"),
    "WebSocketCloseInfoPy": ("WebSocketCloseInfo", "eggsec.net.WebSocketCloseInfo"),
    "WebSocketHandshakePy": ("WebSocketHandshake", "eggsec.net.WebSocketHandshake"),
    "WebSocketSessionPy": ("WebSocketSession", "eggsec.net.WebSocketSession"),
    "AsyncWebSocketSessionPy": ("AsyncWebSocketSession", "eggsec.net.AsyncWebSocketSession"),
    "WebSocketAssessmentConfigPy": ("WebSocketAssessmentConfig", "eggsec.net.WebSocketAssessmentConfig"),
    "WebSocketAssessmentResultPy": ("WebSocketAssessmentResult", "eggsec.net.WebSocketAssessmentResult"),
    # WAF
    "WafDetectionResultPy": ("WafDetectionResult", "eggsec.WafDetectionResult"),
    "BypassResultPy": ("BypassResult", "eggsec.BypassResult"),
    "WafScanResultPy": ("WafScanResult", "eggsec.WafScanResult"),
    "PayloadPy": ("Payload", "eggsec.Payload"),
    "FuzzResultPy": ("FuzzResult", "eggsec.FuzzResult"),
    "FuzzSessionPy": ("FuzzSession", "eggsec.FuzzSession"),
    "LoadTestResultPy": ("LoadTestResult", "eggsec.LoadTestResult"),
    # WebSocket test
    "WebSocketReportPy": ("WebSocketReport", "eggsec.WebSocketReport"),
    "WebSocketFindingPy": ("WebSocketFinding", "eggsec.WebSocketFinding"),
    "ConnectionTestResultPy": ("ConnectionTestResult", "eggsec.ConnectionTestResult"),
    "InjectionTestResultPy": ("InjectionTestResult", "eggsec.InjectionTestResult"),
    "OriginTestResultPy": ("OriginTestResult", "eggsec.OriginTestResult"),
    "FuzzTestResultPy": ("FuzzTestResult", "eggsec.FuzzTestResult"),
    "WebSocketTestConfigPy": ("WebSocketTestConfig", "eggsec.WebSocketTestConfig"),
    # Git secrets
    "GitSecretsReportPy": ("GitSecretsReport", "eggsec.GitSecretsReport"),
    "GitSecretsSummaryPy": ("GitSecretsSummary", "eggsec.GitSecretsSummary"),
    "GitSecretFindingPy": ("GitSecretFinding", "eggsec.GitSecretFinding"),
    "SecretFindingPy": ("SecretFinding", "eggsec.SecretFinding"),
    # SBOM
    "SbomReportPy": ("SbomReport", "eggsec.SbomReport"),
    "SbomComponentPy": ("SbomComponent", "eggsec.SbomComponent"),
    "SbomVulnerabilityPy": ("SbomVulnerability", "eggsec.SbomVulnerability"),
    "SbomFormatPy": ("SbomFormat", "eggsec.SbomFormat"),
    # Database
    "DbPentestReportPy": ("DbPentestReport", "eggsec.DbPentestReport"),
    "DbFindingPy": ("DbFinding", "eggsec.DbFinding"),
    "DbDriverInfoPy": ("DbDriverInfo", "eggsec.sessions.DbDriverInfo"),
    "DbCapabilityPy": ("DbCapability", "eggsec.sessions.DbCapability"),
    "DbCredentialProviderPy": ("DbCredentialProvider", "eggsec.sessions.DbCredentialProvider"),
    "DbSessionConfigPy": ("DbSessionConfig", "eggsec.sessions.DbSessionConfig"),
    "DbDriverRegistryPy": ("DbDriverRegistry", "eggsec.sessions.DbDriverRegistry"),
    "DbTargetPy": ("DbTarget", "eggsec.sessions.DbTarget"),
    "DatabaseSessionStatePy": ("DatabaseSessionState", "eggsec.sessions.DatabaseSessionState"),
    "DatabaseConnectionMetadataPy": ("DatabaseConnectionMetadata", "eggsec.sessions.DatabaseConnectionMetadata"),
    "DatabaseSessionStatsPy": ("DatabaseSessionStats", "eggsec.sessions.DatabaseSessionStats"),
    "DatabaseCredentialRequestPy": ("DatabaseCredentialRequest", "eggsec.sessions.DatabaseCredentialRequest"),
    "DatabaseCredentialResultPy": ("DatabaseCredentialResult", "eggsec.sessions.DatabaseCredentialResult"),
    "DatabaseQueryPy": ("DatabaseQuery", "eggsec.sessions.DatabaseQuery"),
    "DatabaseQueryResultPy": ("DatabaseQueryResult", "eggsec.sessions.DatabaseQueryResult"),
    "DatabaseColumnPy": ("DatabaseColumn", "eggsec.sessions.DatabaseColumn"),
    "DatabaseTableInfoPy": ("DatabaseTableInfo", "eggsec.sessions.DatabaseTableInfo"),
    "DatabaseSchemaInfoPy": ("DatabaseSchemaInfo", "eggsec.sessions.DatabaseSchemaInfo"),
    "DatabasePrivilegeInfoPy": ("DatabasePrivilegeInfo", "eggsec.sessions.DatabasePrivilegeInfo"),
    "StaticCredentialProviderPy": ("StaticCredentialProvider", "eggsec.sessions.StaticCredentialProvider"),
    "EnvironmentCredentialProviderPy": ("EnvironmentCredentialProvider", "eggsec.sessions.EnvironmentCredentialProvider"),
    "CallbackCredentialProviderPy": ("CallbackCredentialProvider", "eggsec.sessions.CallbackCredentialProvider"),
    "DatabaseRowStreamPy": ("DatabaseRowStream", "eggsec.sessions.DatabaseRowStream"),
    "DatabaseQueryPlanPy": ("DatabaseQueryPlan", "eggsec.sessions.DatabaseQueryPlan"),
    "DatabaseIndexInfoPy": ("DatabaseIndexInfo", "eggsec.sessions.DatabaseIndexInfo"),
    "DatabaseExtensionInfoPy": ("DatabaseExtensionInfo", "eggsec.sessions.DatabaseExtensionInfo"),
    # Proxy
    "ProxyTypePy": ("ProxyType", "eggsec.sessions.ProxyType"),
    "RotationStrategyPy": ("RotationStrategy", "eggsec.sessions.RotationStrategy"),
    "ProxyConfigPy": ("ProxyConfig", "eggsec.sessions.ProxyConfig"),
    "ProxyEntryPy": ("ProxyEntry", "eggsec.sessions.ProxyEntry"),
    "ProxyManagerPy": ("ProxyManager", "eggsec.sessions.ProxyManager"),
    "HealthCheckResultPy": ("HealthCheckResult", "eggsec.sessions.HealthCheckResult"),
    "ProxyHealthPy": ("ProxyHealth", "eggsec.sessions.ProxyHealth"),
    "InterceptConfigPy": ("InterceptConfig", "eggsec.sessions.InterceptConfig"),
    "CapturedExchangePy": ("CapturedExchange", "eggsec.sessions.CapturedExchange"),
    "InterceptSessionResultPy": ("InterceptSessionResult", "eggsec.sessions.InterceptSessionResult"),
    "InterceptSessionStatePy": ("InterceptSessionState", "eggsec.sessions.InterceptSessionState"),
    "InterceptStatsPy": ("InterceptStats", "eggsec.sessions.InterceptStats"),
    "InterceptFilterPy": ("InterceptFilter", "eggsec.sessions.InterceptFilter"),
    "InterceptRulePy": ("InterceptRule", "eggsec.sessions.InterceptRule"),
    "CertificateAuthorityConfigPy": ("CertificateAuthorityConfig", "eggsec.sessions.CertificateAuthorityConfig"),
    "IssuedCertificatePy": ("IssuedCertificate", "eggsec.sessions.IssuedCertificate"),
    "HarEntryPy": ("HarEntry", "eggsec.sessions.HarEntry"),
    "HarDocumentPy": ("HarDocument", "eggsec.sessions.HarDocument"),
    "MutationDecisionPy": ("MutationDecision", "eggsec.sessions.MutationDecision"),
    "MutationErrorPy": ("MutationError", "eggsec.sessions.MutationError"),
    "CertificateAuthorityPy": ("CertificateAuthority", "eggsec.sessions.CertificateAuthority"),
    "CertificateStorePy": ("CertificateStore", "eggsec.sessions.CertificateStore"),
    "ReplayRequestPy": ("ReplayRequest", "eggsec.sessions.ReplayRequest"),
    "ReplayResultPy": ("ReplayResult", "eggsec.sessions.ReplayResult"),
    "ResponseComparisonPy": ("ResponseComparison", "eggsec.sessions.ResponseComparison"),
    "ComparisonRulePy": ("ComparisonRule", "eggsec.sessions.ComparisonRule"),
    # Mobile
    "MobilePlatformPy": ("MobilePlatform", "eggsec.MobilePlatform"),
    "MobileFindingPy": ("MobileFinding", "eggsec.MobileFinding"),
    "MobileScanReportPy": ("MobileScanReport", "eggsec.MobileScanReport"),
    "MobileDevicePy": ("MobileDevice", "eggsec.experimental.MobileDevice"),
    "DynamicMobileConfigPy": ("DynamicMobileConfig", "eggsec.experimental.DynamicMobileConfig"),
    "DynamicMobileReportPy": ("DynamicMobileReport", "eggsec.experimental.DynamicMobileReport"),
    # Container
    "ContainerScanTypePy": ("ContainerScanType", "eggsec.ContainerScanType"),
    "EscapeRiskLevelPy": ("EscapeRiskLevel", "eggsec.EscapeRiskLevel"),
    "CisCheckStatusPy": ("CisCheckStatus", "eggsec.CisCheckStatus"),
    "DockerScanResultPy": ("DockerScanResult", "eggsec.DockerScanResult"),
    "KubernetesScanResultPy": ("KubernetesScanResult", "eggsec.KubernetesScanResult"),
    "EscapeDetectionResultPy": ("EscapeDetectionResult", "eggsec.EscapeDetectionResult"),
    "CisBenchmarkResultPy": ("CisBenchmarkResult", "eggsec.CisBenchmarkResult"),
    "ContainerFindingPy": ("ContainerFinding", "eggsec.ContainerFinding"),
    "ContainerReportPy": ("ContainerReport", "eggsec.ContainerReport"),
    # Packet
    "CaptureConfigPy": ("CaptureConfig", "eggsec.CaptureConfig"),
    "CaptureStatsPy": ("CaptureStats", "eggsec.CaptureStats"),
    "PacketInfoPy": ("PacketInfo", "eggsec.PacketInfo"),
    "NetworkInterfaceInfoPy": ("NetworkInterfaceInfo", "eggsec.NetworkInterfaceInfo"),
    "PcapWriterPy": ("PcapWriter", "eggsec.PcapWriter"),
    "PacketFilterPy": ("PacketFilter", "eggsec.PacketFilter"),
    "FlowRecordPy": ("FlowRecord", "eggsec.FlowRecord"),
    "LiveCaptureResultPy": ("LiveCaptureResult", "eggsec.LiveCaptureResult"),
    "TracerouteConfigPy": ("TracerouteConfig", "eggsec.TracerouteConfig"),
    "TracerouteHopPy": ("TracerouteHop", "eggsec.TracerouteHop"),
    "TracerouteResultPy": ("TracerouteResult", "eggsec.TracerouteResult"),
    "BackpressurePolicyPy": ("BackpressurePolicy", "eggsec.BackpressurePolicy"),
    "CaptureDropStatsPy": ("CaptureDropStats", "eggsec.CaptureDropStats"),
    "CapturedPacketPy": ("CapturedPacket", "eggsec.CapturedPacket"),
    "AsyncCaptureSessionPy": ("AsyncCaptureSession", "eggsec.AsyncCaptureSession"),
    "EthernetFramePy": ("EthernetFrame", "eggsec.EthernetFrame"),
    "Ipv4PacketPy": ("Ipv4Packet", "eggsec.Ipv4Packet"),
    "Ipv6PacketPy": ("Ipv6Packet", "eggsec.Ipv6Packet"),
    "TcpSegmentPy": ("TcpSegment", "eggsec.TcpSegment"),
    "UdpDatagramPy": ("UdpDatagram", "eggsec.UdpDatagram"),
    "IcmpPacketPy": ("IcmpPacket", "eggsec.IcmpPacket"),
    "FlowKeyPy": ("FlowKey", "eggsec.FlowKey"),
    "FlowAggregatorPy": ("FlowAggregator", "eggsec.FlowAggregator"),
    "IcmpProbeConfigPy": ("IcmpProbeConfig", "eggsec.IcmpProbeConfig"),
    "IcmpProbeReplyPy": ("IcmpProbeReply", "eggsec.IcmpProbeReply"),
    "IcmpProbeResultPy": ("IcmpProbeResult", "eggsec.IcmpProbeResult"),
    "TcpProbeConfigPy": ("TcpProbeConfig", "eggsec.TcpProbeConfig"),
    "TcpProbeResultPy": ("TcpProbeResult", "eggsec.TcpProbeResult"),
    "PacketTimestampPy": ("PacketTimestamp", "eggsec.PacketTimestamp"),
    "PacketStreamPy": ("PacketStream", "eggsec.PacketStream"),
    "PacketArtifactPy": ("PacketArtifact", "eggsec.PacketArtifact"),
    "SyncCaptureSessionPy": ("SyncCaptureSession", "eggsec.SyncCaptureSession"),
    "DnsPacketPy": ("DnsPacket", "eggsec.DnsPacket"),
    "TlsRecordInfoPy": ("TlsRecordInfo", "eggsec.TlsRecordInfo"),
    "UdpReachabilityConfigPy": ("UdpReachabilityConfig", "eggsec.UdpReachabilityConfig"),
    "UdpReachabilityResultPy": ("UdpReachabilityResult", "eggsec.UdpReachabilityResult"),
    # Stress
    "StressTypePy": ("StressType", "eggsec.experimental.StressType"),
    "StressConfigPy": ("StressConfig", "eggsec.experimental.StressConfig"),
    "StressStatsPy": ("StressStats", "eggsec.experimental.StressStats"),
    "StressResultPy": ("StressResult", "eggsec.experimental.StressResult"),
    # NSE
    "NseConfigPy": ("NseConfig", "eggsec.NseConfig"),
    "NseReportPy": ("NseReport", "eggsec.NseReport"),
    "NseLibraryUsePy": ("NseLibraryUse", "eggsec.NseLibraryUse"),
    "NseRuleEvaluationPy": ("NseRuleEvaluation", "eggsec.NseRuleEvaluation"),
    "NseScriptMetadataPy": ("NseScriptMetadata", "eggsec.NseScriptMetadata"),
    "NseSandboxPolicyPy": ("NseSandboxPolicy", "eggsec.NseSandboxPolicy"),
    "NseTargetContextPy": ("NseTargetContext", "eggsec.NseTargetContext"),
    "NseLibraryDescriptorPy": ("NseLibraryDescriptor", "eggsec.NseLibraryDescriptor"),
    "NseArgumentPy": ("NseArgument", "eggsec.NseArgument"),
    "NseLibraryRegistryPy": ("NseLibraryRegistry", "eggsec.NseLibraryRegistry"),
    "NseEvidenceItemPy": ("NseEvidenceItem", "eggsec.NseEvidenceItem"),
    "NseExecutionLimitsPy": ("NseExecutionLimits", "eggsec.NseExecutionLimits"),
    "NseCancellationTokenPy": ("NseCancellationToken", "eggsec.NseCancellationToken"),
    "NseRuntimeStatsPy": ("NseRuntimeStats", "eggsec.NseRuntimeStats"),
    "NseRuntimeConfigPy": ("NseRuntimeConfig", "eggsec.NseRuntimeConfig"),
    "NseRuntimePy": ("NseRuntime", "eggsec.NseRuntime"),
    "NseScriptSourcePy": ("NseScriptSource", "eggsec.NseScriptSource"),
    "NseDiagnosticPy": ("NseDiagnostic", "eggsec.NseDiagnostic"),
    "NseCapabilityContextPy": ("NseCapabilityContext", "eggsec.NseCapabilityContext"),
    "NseHostContextPy": ("NseHostContext", "eggsec.NseHostContext"),
    "NsePortContextPy": ("NsePortContext", "eggsec.NsePortContext"),
    "NseRuleResultPy": ("NseRuleResult", "eggsec.NseRuleResult"),
    "NseLibraryVersionPy": ("NseLibraryVersion", "eggsec.NseLibraryVersion"),
    "NseLibraryConflictPy": ("NseLibraryConflict", "eggsec.NseLibraryConflict"),
    "NseExecutionRequestPy": ("NseExecutionRequest", "eggsec.NseExecutionRequest"),
    "NseExecutionResultPy": ("NseExecutionResult", "eggsec.NseExecutionResult"),
    "NseScriptResultPy": ("NseScriptResult", "eggsec.NseScriptResult"),
    "NseOutputValuePy": ("NseOutputValue", "eggsec.NseOutputValue"),
    # Daemon
    "DaemonClientPy": ("DaemonClient", "eggsec.daemon.DaemonClient"),
    "DaemonResponsePy": ("DaemonResponse", "eggsec.daemon.DaemonResponse"),
    "DaemonCapabilitiesPy": ("DaemonCapabilities", "eggsec.daemon.DaemonCapabilities"),
    "TaskHandlePy": ("TaskHandle", "eggsec.daemon.TaskHandle"),
    "TaskStatusPy": ("TaskStatus", "eggsec.daemon.TaskStatus"),
    "SessionSummaryPy": ("SessionSummary", "eggsec.daemon.SessionSummary"),
    "TransportMetadataPy": ("TransportMetadata", "eggsec.daemon.TransportMetadata"),
    # Distributed
    "DistributedTaskTypePy": ("DistributedTaskType", "eggsec.DistributedTaskType"),
    "WorkerStatusPy": ("WorkerStatus", "eggsec.WorkerStatus"),
    "WorkerRegistrationPy": ("WorkerRegistration", "eggsec.WorkerRegistration"),
    "HeartbeatPy": ("Heartbeat", "eggsec.Heartbeat"),
    "DistributedTaskPy": ("DistributedTask", "eggsec.DistributedTask"),
    "DistributedTaskResultPy": ("DistributedTaskResult", "eggsec.DistributedTaskResult"),
    # Notifications
    "WebhookEventPy": ("WebhookEvent", "eggsec.WebhookEvent"),
    "FindingSummaryPy": ("FindingSummary", "eggsec.FindingSummary"),
    "NotifyScanStatsPy": ("NotifyScanStats", "eggsec.NotifyScanStats"),
    "WebhookConfigPy": ("WebhookConfig", "eggsec.WebhookConfig"),
    "NotifyManagerPy": ("NotifyManager", "eggsec.NotifyManager"),
    # AI
    "AiProviderPy": ("AiProvider", "eggsec.experimental.AiProvider"),
    "PluginLanguagePy": ("PluginLanguage", "eggsec.experimental.PluginLanguage"),
    "ScriptTargetPy": ("ScriptTarget", "eggsec.experimental.ScriptTarget"),
    "AiAnalysisResultPy": ("AiAnalysisResult", "eggsec.experimental.AiAnalysisResult"),
    "AiPayloadSuggestionPy": ("AiPayloadSuggestion", "eggsec.experimental.AiPayloadSuggestion"),
    "AiWafBypassSuggestionPy": ("AiWafBypassSuggestion", "eggsec.experimental.AiWafBypassSuggestion"),
    "AiCacheStatsPy": ("AiCacheStats", "eggsec.experimental.AiCacheStats"),
    "ScriptMetadataPy": ("ScriptMetadata", "eggsec.experimental.ScriptMetadata"),
    "GeneratedScriptPy": ("GeneratedScript", "eggsec.experimental.GeneratedScript"),
    "AiCachePy": ("AiCache", "eggsec.experimental.AiCache"),
    # Operation metadata
    "OperationDescriptorPy": ("OperationDescriptor", "eggsec.OperationDescriptor"),
    "ExecutionSurfacePy": ("ExecutionSurface", "eggsec.ExecutionSurface"),
    "ExecutionProfilePy": ("ExecutionProfile", "eggsec.ExecutionProfile"),
    "PolicyDecisionPy": ("PolicyDecision", "eggsec.PolicyDecision"),
    "EnforcementOutcomePy": ("EnforcementOutcome", "eggsec.EnforcementOutcome"),
    "ApprovedOperationPy": ("ApprovedOperation", "eggsec.ApprovedOperation"),
    "ExecutionPolicyPy": ("ExecutionPolicy", "eggsec.ExecutionPolicy"),
    "ManualOverridePy": ("ManualOverride", "eggsec.ManualOverride"),
    "PreflightResultPy": ("PreflightResult", "eggsec.PreflightResult"),
    "AuditOutcomePy": ("AuditOutcome", "eggsec.AuditOutcome"),
    "ManualOverrideAuditPy": ("ManualOverrideAudit", "eggsec.ManualOverrideAudit"),
    "ScopeAuditPy": ("ScopeAudit", "eggsec.ScopeAudit"),
    "EnforcementAuditEventPy": ("EnforcementAuditEvent", "eggsec.EnforcementAuditEvent"),
    # Event aliases
    "FindingEventPy": ("FindingEvent", "eggsec.FindingEvent"),
    "ArtifactEventPy": ("ArtifactEvent", "eggsec.ArtifactEvent"),
    # DaemonEvent (already has clean name via daemon submodule)
    "DaemonEventPy": ("DaemonEvent", "eggsec.daemon.DaemonEvent"),
}


class _DeprecatedAlias:
    """Proxy that emits DeprecationWarning on first access."""

    def __init__(self, module, canonical_name: str, new_path: str):
        self._module = module
        self._canonical_name = canonical_name
        self._new_path = new_path
        self._warned = False

    def __getattr__(self, name: str):
        if not self._warned:
            self._warned = True
            _deprecated(self._canonical_name, self._new_path)
        return getattr(self._module, name)


# ---------------------------------------------------------------------------
# Stable core: version, introspection, engine, operations, config, events
# ---------------------------------------------------------------------------

# Introspection functions
api_surface = _core.api_surface
api_surface_version = _core.api_surface_version
features = _core.features
has_feature = _core.has_feature
feature_matrix = _core.feature_matrix
build_info = _core.build_info
wheel_profile = _core.wheel_profile
domain_maturity = _core.domain_maturity
deprecated_warning = _core.deprecated_warning

# Core engine classes
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

# Stable operation functions (22 core operations)
scan_ports = _core.scan_ports
async_scan_ports = _core.async_scan_ports
scan_endpoints = _core.scan_endpoints
async_scan_endpoints = _core.async_scan_endpoints
fingerprint_services = _core.fingerprint_services
async_fingerprint_services = _core.async_fingerprint_services
recon_dns = _core.recon_dns
async_recon_dns = _core.async_recon_dns
inspect_tls = _core.inspect_tls
async_inspect_tls = _core.async_inspect_tls
detect_technology = _core.detect_technology
async_detect_technology = _core.async_detect_technology
detect_waf = _core.detect_waf
async_detect_waf = _core.async_detect_waf
validate_waf = _core.validate_waf
async_validate_waf = _core.async_validate_waf
fuzz_http = _core.fuzz_http
async_fuzz_http = _core.async_fuzz_http
generate_fuzz_payloads = _core.generate_fuzz_payloads
load_test_http = _core.load_test_http
async_load_test_http = _core.async_load_test_http
run_consolidated_recon = _core.run_consolidated_recon
async_run_consolidated_recon = _core.async_run_consolidated_recon
graphql_test = _core.graphql_test
async_graphql_test = _core.async_graphql_test
oauth_discover_endpoints = _core.oauth_discover_endpoints
oauth_test = _core.oauth_test
async_oauth_test = _core.async_oauth_test
auth_test = _core.auth_test
async_auth_test = _core.async_auth_test

# Feature-gated operation functions
try:
    scan_git_secrets = _core.scan_git_secrets
    async_scan_git_secrets = _core.async_scan_git_secrets
except (AttributeError, ImportError):
    _register_unavailable("scan_git_secrets", "git-secrets", "stable",
                          "pip install eggsec[git-secrets]")

try:
    generate_sbom = _core.generate_sbom
    async_generate_sbom = _core.async_generate_sbom
except (AttributeError, ImportError):
    _register_unavailable("generate_sbom", "sbom", "stable",
                          "pip install eggsec[sbom]")

try:
    db_probe = _core.db_probe
    async_db_probe = _core.async_db_probe
    db_probe_with_config = _core.db_probe_with_config
    db_probe_postgres = _core.db_probe_postgres
    db_probe_mysql = _core.db_probe_mysql
    db_probe_mssql = _core.db_probe_mssql
    db_probe_mongodb = _core.db_probe_mongodb
    db_probe_redis = _core.db_probe_redis
    db_list_drivers = _core.db_list_drivers
    db_get_capabilities = _core.db_get_capabilities
    db_run_with_config = _core.db_run_with_config
except (AttributeError, ImportError):
    _register_unavailable("db_probe", "db-pentest", "stable",
                          "pip install eggsec[db-pentest]")

try:
    create_proxy_manager = _core.create_proxy_manager
    async_add_proxy = _core.async_add_proxy
    async_proxy_health_check = _core.async_proxy_health_check
except (AttributeError, ImportError):
    _register_unavailable("create_proxy_manager", "web-proxy", "experimental",
                          "pip install eggsec[web-proxy]")

try:
    analyze_apk = _core.analyze_apk
    async_analyze_apk = _core.async_analyze_apk
    analyze_ipa = _core.analyze_ipa
    async_analyze_ipa = _core.async_analyze_ipa
except (AttributeError, ImportError):
    _register_unavailable("analyze_apk", "mobile", "stable",
                          "pip install eggsec[mobile]")

try:
    MobileDeviceDescriptor = _core.MobileDeviceDescriptor
    MobileDeviceCapabilities = _core.MobileDeviceCapabilities
    MobileSessionConfig = _core.MobileSessionConfig
    MobileSessionState = _core.MobileSessionState
    MobileSessionStats = _core.MobileSessionStats
    MobileSession = _core.MobileSession
    AsyncMobileSession = _core.AsyncMobileSession
    MobileDeviceRegistry = _core.MobileDeviceRegistry
    MobileDevicePy = _core.MobileDevicePy
    DynamicMobileConfigPy = _core.DynamicMobileConfigPy
    DynamicMobileReportPy = _core.DynamicMobileReportPy
except (AttributeError, ImportError):
    pass

try:
    BrowserCapabilities = _core.BrowserCapabilities
    BrowserSessionState = _core.BrowserSessionState
    BrowserSessionConfig = _core.BrowserSessionConfig
    BrowserSessionStats = _core.BrowserSessionStats
    BrowserSession = _core.BrowserSession
    AsyncBrowserSession = _core.AsyncBrowserSession
    BrowserNavigationEvent = _core.BrowserNavigationEvent
    BrowserConsoleEvent = _core.BrowserConsoleEvent
    BrowserNetworkEvent = _core.BrowserNetworkEvent
    BrowserDomSnapshot = _core.BrowserDomSnapshot
    BrowserFormInfo = _core.BrowserFormInfo
    BrowserFormField = _core.BrowserFormField
    BrowserLinkInfo = _core.BrowserLinkInfo
    BrowserStorageInfo = _core.BrowserStorageInfo
    BrowserCookieInfo = _core.BrowserCookieInfo
    BrowserTestConfigPy = _core.BrowserTestConfigPy
    BrowserTestReportPy = _core.BrowserTestReportPy
    DomXssFindingPy = _core.DomXssFindingPy
    SpaRoutePy = _core.SpaRoutePy
except (AttributeError, ImportError):
    pass

try:
    scan_docker_image = _core.scan_docker_image
    async_scan_docker_image = _core.async_scan_docker_image
    scan_kubernetes = _core.scan_kubernetes
    async_scan_kubernetes = _core.async_scan_kubernetes
    detect_escape_risks = _core.detect_escape_risks
    check_cis_docker_benchmark = _core.check_cis_docker_benchmark
except (AttributeError, ImportError):
    _register_unavailable("scan_docker_image", "container", "stable",
                          "pip install eggsec[container]")

try:
    list_network_interfaces = _core.list_network_interfaces
    parse_pcap = _core.parse_pcap
    run_traceroute = _core.run_traceroute
    async_run_traceroute = _core.async_run_traceroute
    traceroute = _core.traceroute
    icmp_probe = _core.icmp_probe
    async_icmp_probe = _core.async_icmp_probe
    tcp_syn_probe = _core.tcp_syn_probe
    async_tcp_syn_probe = _core.async_tcp_syn_probe
except (AttributeError, ImportError):
    _register_unavailable("parse_pcap", "packet-inspection", "experimental",
                          "pip install eggsec[packet-inspection]",
                          "libpcap-dev")

try:
    nse_run = _core.nse_run
    async_nse_run = _core.async_nse_run
    nse_list_libraries = _core.nse_list_libraries
    nse_list_scripts = _core.nse_list_scripts
    nse_get_script_metadata = _core.nse_get_script_metadata
    nse_list_libraries_detailed = _core.nse_list_libraries_detailed
    nse_get_library_descriptor = _core.nse_get_library_descriptor
    nse_run_with_config = _core.nse_run_with_config
    nse_validate_script = _core.nse_validate_script
except (AttributeError, ImportError):
    _register_unavailable("nse_run", "nse", "stable",
                          "pip install eggsec[nse]", "libssl-dev")

# Provisional: network probes (always available in default build)
try:
    dns_query = _core.dns_query
    async_dns_query = _core.async_dns_query
    tls_probe = _core.tls_probe
    async_tls_probe = _core.async_tls_probe
    http_probe = _core.http_probe
    async_http_probe = _core.async_http_probe
    udp_probe = _core.udp_probe
    async_udp_probe = _core.async_udp_probe
except (AttributeError, ImportError):
    pass

# Provisional: target resolution and transport probes
try:
    resolve_target_sync = _core.resolve_target_sync
    async_resolve_target = _core.async_resolve_target
    tcp_connect_probe = _core.tcp_connect_probe
    async_tcp_connect_probe = _core.async_tcp_connect_probe
    banner_probe = _core.banner_probe
    async_banner_probe = _core.async_banner_probe
except (AttributeError, ImportError):
    pass

try:
    create_http_client = _core.create_http_client
    async_create_http_client = _core.async_create_http_client
except (AttributeError, ImportError):
    pass

# Provisional: WebSocket (feature-gated)
try:
    websocket_probe = _core.websocket_probe
    async_websocket_probe = _core.async_websocket_probe
    websocket_fuzz = _core.websocket_fuzz
    async_websocket_fuzz = _core.async_websocket_fuzz
    websocket_assess = _core.websocket_assess
    async_websocket_assess = _core.async_websocket_assess
except (AttributeError, ImportError):
    _register_unavailable("websocket_probe", "websocket", "provisional",
                          "pip install eggsec[websocket]")

# Provisional: NSE library details (feature-gated)
try:
    nse_list_libraries_detailed = _core.nse_list_libraries_detailed
    nse_get_library_descriptor = _core.nse_get_library_descriptor
    nse_run_with_config = _core.nse_run_with_config
    nse_validate_script = _core.nse_validate_script
except (AttributeError, ImportError):
    pass

# Provisional: Interception proxy (feature-gated)
try:
    run_intercept_session = _core.run_intercept_session
    async_run_intercept_session = _core.async_run_intercept_session
except (AttributeError, ImportError):
    pass

# Provisional: Daemon client functions (feature-gated)
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

# Experimental: wireless, evasion, postex, C2 (feature-gated)
try:
    wireless_scan = _core.wireless_scan
    async_wireless_scan = _core.async_wireless_scan
    wireless_analyze_networks = _core.wireless_analyze_networks
except (AttributeError, ImportError):
    _register_unavailable("wireless_scan", "wireless", "experimental",
                          "pip install eggsec[wireless]", "Linux; root required")

try:
    evasion_scan = _core.evasion_scan
    async_evasion_scan = _core.async_evasion_scan
    evasion_list_techniques = _core.evasion_list_techniques
except (AttributeError, ImportError):
    _register_unavailable("evasion_scan", "evasion", "experimental",
                          "pip install eggsec[evasion]")

try:
    postex_scan = _core.postex_scan
    async_postex_scan = _core.async_postex_scan
    postex_list_techniques = _core.postex_list_techniques
except (AttributeError, ImportError):
    _register_unavailable("postex_scan", "postex", "experimental",
                          "pip install eggsec[postex]")

try:
    c2_scan = _core.c2_scan
    async_c2_scan = _core.async_c2_scan
    c2_get_campaign = _core.c2_get_campaign
except (AttributeError, ImportError):
    _register_unavailable("c2_scan", "c2", "experimental",
                          "pip install eggsec[c2]")

# Experimental: hunt (feature-gated)
try:
    hunt_test = _core.hunt_test
    async_hunt_test = _core.async_hunt_test
except (AttributeError, ImportError):
    _register_unavailable("hunt_test", "advanced-hunting", "experimental",
                          "pip install eggsec[advanced-hunting]")

# Experimental: AI (feature-gated)
try:
    ai_analyze_finding = _core.ai_analyze_finding
    async_ai_analyze_finding = _core.async_ai_analyze_finding
    ai_generate_payloads = _core.ai_generate_payloads
    ai_suggest_waf_bypass = _core.ai_suggest_waf_bypass
    ai_generate_script = _core.ai_generate_script
except (AttributeError, ImportError):
    _register_unavailable("ai_analyze_finding", "ai-integration", "experimental",
                          "pip install eggsec[ai-integration]")

# Experimental: stress (feature-gated)
try:
    stress_test = _core.stress_test
    async_stress_test = _core.async_stress_test
except (AttributeError, ImportError):
    _register_unavailable("stress_test", "stress-testing", "experimental",
                          "pip install eggsec[stress-testing]")

# Experimental: mobile dynamic (feature-gated)
try:
    list_mobile_devices = _core.list_mobile_devices
    dynamic_mobile_analysis = _core.dynamic_mobile_analysis
except (AttributeError, ImportError):
    _register_unavailable("list_mobile_devices", "mobile-dynamic", "experimental",
                          "pip install eggsec[mobile-dynamic]", "ADB and Android device/emulator")

# Experimental: browser test (feature-gated)
try:
    browser_test = _core.browser_test
    async_browser_test = _core.async_browser_test
except (AttributeError, ImportError):
    _register_unavailable("browser_test", "headless-browser", "provisional",
                          "pip install eggsec[headless-browser]")

# Core DTOs - Findings and reporting
Severity = _core.Severity
Evidence = _core.Evidence
Finding = _core.Finding
FindingSet = _core.FindingSet
Report = _core.Report

# Core DTOs - Result protocol
ExecutionStatus = _core.ExecutionStatus
ExecutionStats = _core.ExecutionStats
Artifact = _core.Artifact
OperationResult = _core.OperationResult
OperationError = _core.OperationError
DispatchAuditEvent = _core.DispatchAuditEvent

# Core DTOs - Scan results
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

# Core DTOs - Recon types
DnsRecordSet = _core.DnsRecordSet
MxRecord = _core.MxRecord
SoaRecord = _core.SoaRecord
TlsCertificateInfo = _core.TlsCertificateInfo
TlsInspectionResult = _core.TlsInspectionResult
SslIssue = _core.SslIssue
TechStack = _core.TechStack
TechDetectionResult = _core.TechDetectionResult

# Core DTOs - WAF
WafDetectionResult = _core.WafDetectionResultPy
BypassResult = _core.BypassResultPy
WafScanResult = _core.WafScanResultPy
Payload = _core.PayloadPy
FuzzResult = _core.FuzzResultPy
FuzzSession = _core.FuzzSessionPy
FuzzConfig = _core.FuzzConfig
LoadTestResult = _core.LoadTestResultPy
LoadTestConfig = _core.LoadTestConfig

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

# Configuration, Policy, and Execution Context
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
DomainDescriptor = _core.DomainDescriptorPy
DomainRegistry = _core.DomainRegistry
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

# Consolidated Recon types
ConsolidatedReconConfig = _core.ConsolidatedReconConfigPy
ReconModuleResult = _core.ReconModuleResultPy
ConsolidatedReconReport = _core.ConsolidatedReconReportPy

# GraphQL types
GraphQLVulnerability = _core.GraphQLVulnerabilityPy
GraphQLTestResult = _core.GraphQLTestResultPy
GraphQLType = _core.GraphQLTypePy
GraphQLField = _core.GraphQLFieldPy
GraphQLArg = _core.GraphQLArgPy
GraphQLInputField = _core.GraphQLInputFieldPy
GraphQLSchema = _core.GraphQLSchemaPy
GraphQLTestConfig = _core.GraphQLTestConfigPy

# OAuth types
OAuthVulnerability = _core.OAuthVulnerabilityPy
OAuthEndpointKind = _core.OAuthEndpointKindPy
OAuthEndpoint = _core.OAuthEndpointPy
OAuthTestResult = _core.OAuthTestResultPy
OAuthTestConfig = _core.OAuthTestConfigPy

# Auth types
AuthTestType = _core.AuthTestTypePy
AuthFinding = _core.AuthFindingPy
AuthTestConfig = _core.AuthTestConfigPy
AuthTestReport = _core.AuthTestReportPy

# Versioned finding schema
Confidence = _core.Confidence
FindingType = _core.FindingType
EvidenceKind = _core.EvidenceKind
AffectedAsset = _core.AffectedAsset
FindingLocation = _core.FindingLocation
VersionedEvidence = _core.VersionedEvidence
VersionedFinding = _core.VersionedFinding

# Evidence and artifact model
MilestoneArtifact = _core.MilestoneArtifact
ArtifactReference = _core.ArtifactReference
ArtifactStore = _core.ArtifactStore

# CVSS and vulnerability records
CvssScore = _core.CvssScore
VulnerabilityRecord = _core.VulnerabilityRecord
RemediationRecord = _core.RemediationRecord

# Finding workflow
FindingState = _core.FindingState
WorkflowTransition = _core.WorkflowTransition
Suppression = _core.Suppression
FindingWorkflow = _core.FindingWorkflow

# Repository abstraction
FindingRepository = _core.FindingRepository
Assessment = _core.Assessment
AssessmentRepository = _core.AssessmentRepository

# Baselines and comparisons
FindingCorrelation = _core.FindingCorrelation
FindingDiff = _core.FindingDiff
AssessmentDiff = _core.AssessmentDiff
BaselineComparator = _core.BaselineComparator

# Reporting
FindingReporter = _core.FindingReporter
SeveritySummary = _core.SeveritySummary
ReportEnvelope = _core.ReportEnvelope

# External integrations
IntegrationType = _core.IntegrationType
PublicationRecord = _core.PublicationRecord
RetryPolicy = _core.RetryPolicy
PublicationPolicy = _core.PublicationPolicy
ExternalIntegration = _core.ExternalIntegration

# Migration and compatibility
SchemaVersion = _core.SchemaVersion
MigrationResult = _core.MigrationResult
FindingMigration = _core.FindingMigration

# Compliance mapping (feature-gated)
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

# Distributed scanning
DistributedTaskType = _core.DistributedTaskTypePy
WorkerStatus = _core.WorkerStatusPy
WorkerRegistration = _core.WorkerRegistrationPy
Heartbeat = _core.HeartbeatPy
DistributedTask = _core.DistributedTaskPy
DistributedTaskResult = _core.DistributedTaskResultPy
distributed_task_types = _core.distributed_task_types
distributed_generate_psk = _core.distributed_generate_psk

# Notifications
WebhookEvent = _core.WebhookEventPy
FindingSummary = _core.FindingSummaryPy
NotifyScanStats = _core.NotifyScanStatsPy
WebhookConfig = _core.WebhookConfigPy
NotifyManager = _core.NotifyManagerPy
notify_scan_started = _core.notify_scan_started
notify_scan_complete = _core.notify_scan_complete
notify_findings = _core.notify_findings
notify_error = _core.notify_error

# Event protocol
EventEnvelope = _core.EventEnvelope
PlanningEvent = _core.PlanningEvent
PreflightEvent = _core.PreflightEvent
StageLifecycleEvent = _core.StageLifecycleEvent
ProgressEvent = _core.ProgressEvent
FindingEvent = _core.FindingEvent
ArtifactEvent = _core.ArtifactEvent
CancellationEvent = _core.CancellationEvent
FailureEvent = _core.FailureEvent
CompletionEvent = _core.CompletionEvent
ResolutionEvent = _core.ResolutionEvent
ConnectionEvent = _core.ConnectionEvent
ProbeEvent = _core.ProbeEvent
WebSocketMessageEvent = _core.WebSocketMessageEvent
CaptureStatsEvent = _core.CaptureStatsEvent
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

# Async iterators
EventStreamAsyncIterator = _core.EventStreamAsyncIterator
FindingStreamAsyncIterator = _core.FindingStreamAsyncIterator

# Callbacks and sinks
AuditSink = _core.AuditSink
FindingSink = _core.FindingSink
ArtifactSink = _core.ArtifactSink
ProgressSink = _core.ProgressSink
EventConsumer = _core.EventConsumer
AsyncCallback = _core.AsyncCallback
CallbackScheduler = _core.CallbackScheduler
BackpressureChannel = _core.PyBackpressureChannel
EventDeliveryStats = _core.EventDeliveryStats

# Session contract
SessionState = _core.SessionState
SessionIdentity = _core.SessionIdentity
SessionStats = _core.SessionStats
SessionCloseMode = _core.SessionCloseMode
SessionEvent = _core.SessionEvent
SessionEventStream = _core.SessionEventStream
SessionCapabilities = _core.SessionCapabilities
create_session_event = _core.create_session_event

# Daemon parity types
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

# SQLite repository
SqliteFindingRepository = _core.SqliteFindingRepository
SqliteAssessmentRepository = _core.SqliteAssessmentRepository
SqliteMigration = _core.SqliteMigration
SqliteMigrationResult = _core.SqliteMigrationResult

# JSONL repository
JsonlFindingRepository = _core.JsonlFindingRepository
JsonlAssessmentRepository = _core.JsonlAssessmentRepository

# Content-addressed artifact store
ContentAddressedArtifactStore = _core.ContentAddressedArtifactStore
DirectoryArtifactStore = _core.DirectoryArtifactStore
ArtifactInfo = _core.ArtifactInfo
ArtifactData = _core.ArtifactData
IntegrityResult = _core.IntegrityResult
ArtifactQuery = _core.ArtifactQuery

# Streaming reporting
StreamingReportConfig = _core.StreamingReportConfig
StreamingReporter = _core.StreamingReporter
ReportSummary = _core.ReportSummary
StreamingDiffReporter = _core.StreamingDiffReporter
FindingDiffResult = _core.FindingDiffResult
DiffReportSummary = _core.DiffReportSummary
ReportManifest = _core.ReportManifest

# Re-export submodules
from . import net      # noqa: E402
from . import sessions  # noqa: E402
from . import storage   # noqa: E402
from . import reporting # noqa: E402
from . import daemon    # noqa: E402
from . import experimental  # noqa: E402

# ---------------------------------------------------------------------------
# Backward-compatible Py-suffixed re-exports
# These names were previously exported at the top level. They are kept for
# backward compatibility; new code should use the canonical names from
# submodules (eggsec.net, eggsec.sessions, etc.) or the clean top-level names.
# ---------------------------------------------------------------------------

# Network types (canonical: eggsec.net.*)
try:
    TargetPy = _core.TargetPy
    ResolvedTargetPy = _core.ResolvedTargetPy
    ConnectionConfigPy = _core.ConnectionConfigPy
    TimeoutConfigPy = _core.TimeoutConfigPy
    RetryPolicyPy = _core.RetryPolicyPy
    SocketEndpointPy = _core.SocketEndpointPy
    ConnectionTimingPy = _core.ConnectionTimingPy
    ConnectionMetadataPy = _core.ConnectionMetadataPy
    NetworkEvidencePy = _core.NetworkEvidencePy
    TranscriptEntryPy = _core.TranscriptEntryPy
    NetworkTranscriptPy = _core.NetworkTranscriptPy
except (AttributeError, ImportError):
    pass

# Transport (canonical: eggsec.net.*)
try:
    TcpConfigPy = _core.TcpConfigPy
    TcpSessionPy = _core.TcpSessionPy
    TcpConnectResultPy = _core.TcpConnectResultPy
    TcpReadResultPy = _core.TcpReadResultPy
    TcpWriteResultPy = _core.TcpWriteResultPy
    UdpConfigPy = _core.UdpConfigPy
    UdpSocketPy = _core.UdpSocketPy
    UdpSendResultPy = _core.UdpSendResultPy
    UdpRecvResultPy = _core.UdpRecvResultPy
    UdpRecvFromResultPy = _core.UdpRecvFromResultPy
    BannerProbeResultPy = _core.BannerProbeResultPy
    AsyncTcpSessionPy = _core.AsyncTcpSessionPy
    AsyncUdpSocketPy = _core.AsyncUdpSocketPy
except (AttributeError, ImportError):
    pass

# Probes (canonical: eggsec.net.*)
try:
    DnsQueryConfigPy = _core.DnsQueryConfigPy
    DnsRecordPy = _core.DnsRecordPy
    DnsQueryResultPy = _core.DnsQueryResultPy
    TlsProbeConfigPy = _core.TlsProbeConfigPy
    CertificateInfoPy = _core.CertificateInfoPy
    CertificateChainEntryPy = _core.CertificateChainEntryPy
    TlsProbeResultPy = _core.TlsProbeResultPy
    TlsIssuePy = _core.TlsIssuePy
    HttpProbeConfigPy = _core.HttpProbeConfigPy
    HttpProbeResultPy = _core.HttpProbeResultPy
    UdpProbeConfigPy = _core.UdpProbeConfigPy
    UdpProbeResultPy = _core.UdpProbeResultPy
except (AttributeError, ImportError):
    pass

# HTTP client (canonical: eggsec.net.*)
try:
    HttpRequestPy = _core.HttpRequestPy
    HttpHeadersPy = _core.HttpHeadersPy
    HttpResponsePy = _core.HttpResponsePy
    HttpCookiePy = _core.HttpCookiePy
    RedirectEntryPy = _core.RedirectEntryPy
    TlsMetadataPy = _core.TlsMetadataPy
    HttpTimingPy = _core.HttpTimingPy
    HttpClientConfigPy = _core.HttpClientConfigPy
    HttpClientPy = _core.HttpClientPy
    AsyncHttpClientPy = _core.AsyncHttpClientPy
    RedactConfigPy = _core.RedactConfigPy
except (AttributeError, ImportError):
    pass

# WebSocket (canonical: eggsec.net.*)
try:
    WebSocketSessionConfigPy = _core.WebSocketSessionConfigPy
    WebSocketMessagePy = _core.WebSocketMessagePy
    WebSocketFramePy = _core.WebSocketFramePy
    WebSocketCloseInfoPy = _core.WebSocketCloseInfoPy
    WebSocketHandshakePy = _core.WebSocketHandshakePy
    WebSocketSessionPy = _core.WebSocketSessionPy
    AsyncWebSocketSessionPy = _core.AsyncWebSocketSessionPy
    WebSocketAssessmentConfigPy = _core.WebSocketAssessmentConfigPy
    WebSocketAssessmentResultPy = _core.WebSocketAssessmentResultPy
except (AttributeError, ImportError):
    pass

# Packet capture (feature-gated)
try:
    CaptureConfigPy = _core.CaptureConfigPy
    PacketTimestampPy = _core.PacketTimestampPy
    PacketStreamPy = _core.PacketStreamPy
    PacketArtifactPy = _core.PacketArtifactPy
    SyncCaptureSessionPy = _core.SyncCaptureSessionPy
    DnsPacketPy = _core.DnsPacketPy
    TlsRecordInfoPy = _core.TlsRecordInfoPy
    UdpReachabilityConfigPy = _core.UdpReachabilityConfigPy
    UdpReachabilityResultPy = _core.UdpReachabilityResultPy
except (AttributeError, ImportError):
    pass

# Feature-gated classes (consolidated)
try:
    # WebSocket test
    WebSocketReport = _core.WebSocketReportPy
    WebSocketFinding = _core.WebSocketFindingPy
    ConnectionTestResult = _core.ConnectionTestResultPy
    InjectionTestResult = _core.InjectionTestResultPy
    OriginTestResult = _core.OriginTestResultPy
    FuzzTestResult = _core.FuzzTestResultPy
    WebSocketTestConfig = _core.WebSocketTestConfigPy
except (AttributeError, ImportError):
    pass

try:
    GitSecretsReport = _core.GitSecretsReportPy
    GitSecretsSummary = _core.GitSecretsSummaryPy
    GitSecretFinding = _core.GitSecretFindingPy
    SecretFinding = _core.SecretFindingPy
except (AttributeError, ImportError):
    pass

try:
    SbomReport = _core.SbomReportPy
    SbomComponent = _core.SbomComponentPy
    SbomVulnerability = _core.SbomVulnerabilityPy
    SbomFormat = _core.SbomFormatPy
except (AttributeError, ImportError):
    pass

try:
    DbPentestReport = _core.DbPentestReportPy
    DbFinding = _core.DbFindingPy
    DbPentestConfig = _core.DbPentestConfig
    DbCapabilityPy = _core.DbCapabilityPy
    DbDriverInfoPy = _core.DbDriverInfoPy
    DbCredentialProviderPy = _core.DbCredentialProviderPy
    DbSessionConfigPy = _core.DbSessionConfigPy
    DbDriverRegistryPy = _core.DbDriverRegistryPy
    DbTargetPy = _core.DbTargetPy
    DatabaseSessionStatePy = _core.DatabaseSessionStatePy
    DatabaseConnectionMetadataPy = _core.DatabaseConnectionMetadataPy
    DatabaseSessionStatsPy = _core.DatabaseSessionStatsPy
    DatabaseCredentialRequestPy = _core.DatabaseCredentialRequestPy
    DatabaseCredentialResultPy = _core.DatabaseCredentialResultPy
    DatabaseQueryPy = _core.DatabaseQueryPy
    DatabaseQueryResultPy = _core.DatabaseQueryResultPy
    DatabaseColumnPy = _core.DatabaseColumnPy
    DatabaseTableInfoPy = _core.DatabaseTableInfoPy
    DatabaseSchemaInfoPy = _core.DatabaseSchemaInfoPy
    DatabasePrivilegeInfoPy = _core.DatabasePrivilegeInfoPy
    StaticCredentialProviderPy = _core.StaticCredentialProviderPy
    EnvironmentCredentialProviderPy = _core.EnvironmentCredentialProviderPy
    CallbackCredentialProviderPy = _core.CallbackCredentialProviderPy
    DatabaseRowStreamPy = _core.DatabaseRowStreamPy
    DatabaseQueryPlanPy = _core.DatabaseQueryPlanPy
    DatabaseIndexInfoPy = _core.DatabaseIndexInfoPy
    DatabaseExtensionInfoPy = _core.DatabaseExtensionInfoPy
    SeverityChangePy = _core.SeverityChangePy
    ComplianceHitPy = _core.ComplianceHitPy
    ComplianceSummaryPy = _core.ComplianceSummaryPy
    DbCorrelatedFindingPy = _core.DbCorrelatedFindingPy
    DbCorrelationResultPy = _core.DbCorrelationResultPy
    DbCorrelationSummaryPy = _core.DbCorrelationSummaryPy
    DbCorrelationTypePy = _core.DbCorrelationTypePy
    DbRegressionResultPy = _core.DbRegressionResultPy
except (AttributeError, ImportError):
    pass

try:
    ProxyTypePy = _core.ProxyTypePy
    RotationStrategyPy = _core.RotationStrategyPy
    ProxyConfigPy = _core.ProxyConfigPy
    ProxyEntryPy = _core.ProxyEntryPy
    ProxyManagerPy = _core.ProxyManagerPy
    HealthCheckResultPy = _core.HealthCheckResultPy
    ProxyHealthPy = _core.ProxyHealthPy
    InterceptConfigPy = _core.InterceptConfigPy
    CapturedExchangePy = _core.CapturedExchangePy
    InterceptSessionResultPy = _core.InterceptSessionResultPy
    InterceptSessionStatePy = _core.InterceptSessionStatePy
    InterceptStatsPy = _core.InterceptStatsPy
    InterceptFilterPy = _core.InterceptFilterPy
    InterceptRulePy = _core.InterceptRulePy
    CertificateAuthorityConfigPy = _core.CertificateAuthorityConfigPy
    IssuedCertificatePy = _core.IssuedCertificatePy
    HarEntryPy = _core.HarEntryPy
    HarDocumentPy = _core.HarDocumentPy
    MutationDecisionPy = _core.MutationDecisionPy
    MutationErrorPy = _core.MutationErrorPy
    CertificateAuthorityPy = _core.CertificateAuthorityPy
    CertificateStorePy = _core.CertificateStorePy
    ReplayRequestPy = _core.ReplayRequestPy
    ResponseComparisonPy = _core.ResponseComparisonPy
    ComparisonRulePy = _core.ComparisonRulePy
    RequestModificationPy = _core.RequestModificationPy
    ResponseModificationPy = _core.ResponseModificationPy
except (AttributeError, ImportError):
    pass

try:
    MobilePlatform = _core.MobilePlatformPy
    MobileFinding = _core.MobileFindingPy
    MobileScanReport = _core.MobileScanReportPy
except (AttributeError, ImportError):
    pass

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

try:
    CaptureConfig = _core.CaptureConfigPy
    CaptureStats = _core.CaptureStatsPy
    PacketInfo = _core.PacketInfoPy
    NetworkInterfaceInfo = _core.NetworkInterfaceInfoPy
    PcapWriter = _core.PcapWriterPy
    PacketFilter = _core.PacketFilterPy
    FlowRecord = _core.FlowRecordPy
    LiveCaptureResult = _core.LiveCaptureResultPy
    TracerouteConfig = _core.TracerouteConfigPy
    TracerouteHop = _core.TracerouteHopPy
    TracerouteResult = _core.TracerouteResultPy
    BackpressurePolicy = _core.BackpressurePolicyPy
    CaptureDropStats = _core.CaptureDropStatsPy
    CapturedPacket = _core.CapturedPacketPy
    AsyncCaptureSession = _core.AsyncCaptureSessionPy
    EthernetFrame = _core.EthernetFramePy
    Ipv4Packet = _core.Ipv4PacketPy
    Ipv6Packet = _core.Ipv6PacketPy
    TcpSegment = _core.TcpSegmentPy
    UdpDatagram = _core.UdpDatagramPy
    IcmpPacket = _core.IcmpPacketPy
    FlowKey = _core.FlowKeyPy
    FlowAggregator = _core.FlowAggregatorPy
    IcmpProbeConfig = _core.IcmpProbeConfigPy
    IcmpProbeReply = _core.IcmpProbeReplyPy
    IcmpProbeResult = _core.IcmpProbeResultPy
    TcpProbeConfig = _core.TcpProbeConfigPy
    TcpProbeResult = _core.TcpProbeResultPy
    PacketTimestampPy = _core.PacketTimestampPy
    PacketStreamPy = _core.PacketStreamPy
    PacketArtifactPy = _core.PacketArtifactPy
    SyncCaptureSessionPy = _core.SyncCaptureSessionPy
    DnsPacketPy = _core.DnsPacketPy
    TlsRecordInfoPy = _core.TlsRecordInfoPy
    UdpReachabilityConfigPy = _core.UdpReachabilityConfigPy
    UdpReachabilityResultPy = _core.UdpReachabilityResultPy
    udp_reachability = _core.udp_reachability
except (AttributeError, ImportError):
    pass

try:
    StressType = _core.StressTypePy
    StressConfig = _core.StressConfigPy
    StressStats = _core.StressStatsPy
    StressResult = _core.StressResultPy
    StressConfigSummaryPy = _core.StressConfigSummaryPy
except (AttributeError, ImportError):
    pass

try:
    NseConfig = _core.NseConfigPy
    NseReport = _core.NseReportPy
    NseLibraryUse = _core.NseLibraryUsePy
    NseRuleEvaluation = _core.NseRuleEvaluationPy
    NseScriptMetadata = _core.NseScriptMetadataPy
    NseSandboxPolicy = _core.NseSandboxPolicyPy
    NseTargetContext = _core.NseTargetContextPy
    NseLibraryDescriptor = _core.NseLibraryDescriptorPy
    NseArgument = _core.NseArgumentPy
    NseLibraryRegistry = _core.NseLibraryRegistryPy
    NseEvidenceItem = _core.NseEvidenceItemPy
    NseExecutionLimits = _core.NseExecutionLimitsPy
    NseCancellationToken = _core.NseCancellationTokenPy
    NseRuntimeStats = _core.NseRuntimeStatsPy
    NseRuntimeConfig = _core.NseRuntimeConfigPy
    NseRuntime = _core.NseRuntimePy
    NseScriptSource = _core.NseScriptSourcePy
    NseDiagnostic = _core.NseDiagnosticPy
    NseCapabilityContext = _core.NseCapabilityContextPy
    NseHostContext = _core.NseHostContextPy
    NsePortContext = _core.NsePortContextPy
    NseRuleResult = _core.NseRuleResultPy
    NseLibraryVersion = _core.NseLibraryVersionPy
    NseLibraryConflict = _core.NseLibraryConflictPy
    NseExecutionRequest = _core.NseExecutionRequestPy
    NseExecutionResult = _core.NseExecutionResultPy
    NseScriptResult = _core.NseScriptResultPy
    NseOutputValue = _core.NseOutputValuePy
except (AttributeError, ImportError):
    pass

try:
    DaemonClient = _core.DaemonClientPy
    DaemonResponse = _core.DaemonResponsePy
    DaemonCapabilities = _core.DaemonCapabilitiesPy
    TaskHandle = _core.TaskHandlePy
    TaskStatus = _core.TaskStatusPy
    SessionSummary = _core.SessionSummaryPy
    TransportMetadata = _core.TransportMetadataPy
except (AttributeError, ImportError):
    pass

# Tool-core types (Release 5)
try:
    from ._core import (
        TargetTypePy as ToolTargetType,
        AuthTypePy as ToolAuthType,
        ResponseTypePy as ToolResponseType,
        ToolFindingType as ToolFindingType,
        ToolSeverity as ToolSeverity,
        ToolErrorTypePy as ToolErrorType,
        PortStatePy as ToolPortState,
        StreamEventTypePy as ToolStreamEventType,
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
        ToolDescriptor as ToolDescriptor,
        ToolRegistry as ToolRegistry,
        OperationToolView as OperationToolView,
        ValidationReport as ValidationReport,
        SchemaGenerator as SchemaGenerator,
        OpenApiAdapter as OpenApiAdapter,
        operation_as_tool as operation_as_tool,
    )
except (AttributeError, ImportError):
    pass

# Exceptions
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

# Backward-compatible aliases (deprecated Py-suffixed names)
# These are kept in __all__ for backward compatibility but emit deprecation warnings.
# (The actual class names from _core are already registered; we don't need to
# re-alias them since the clean names are already the primary exports.)

# Backward-compatible event aliases
FindingEventPy = FindingEvent
ArtifactEventPy = ArtifactEvent
DaemonEventPy = DaemonEventPy  # Already named with Py suffix in _core


# ---------------------------------------------------------------------------
# __all__ - comprehensive export list for backward compatibility
# ---------------------------------------------------------------------------

__all__ = [
    # Version and introspection
    "__version__",
    "__version_info__",
    "__schema_version__",
    "__protocol_version__",
    "__abi_version__",
    "FINDING_SCHEMA_VERSION",
    "EVENT_SCHEMA_VERSION",
    "api_surface",
    "api_surface_version",
    "_deprecated",
    "deprecated_warning",
    "features",
    "has_feature",
    "feature_matrix",
    "build_info",
    "domain_maturity",
    # Submodules
    "net",
    "sessions",
    "storage",
    "reporting",
    "daemon",
    "experimental",
    # Core engine classes
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
    # Stable operations
    "scan_ports",
    "async_scan_ports",
    "scan_endpoints",
    "async_scan_endpoints",
    "fingerprint_services",
    "async_fingerprint_services",
    "recon_dns",
    "async_recon_dns",
    "inspect_tls",
    "async_inspect_tls",
    "detect_technology",
    "async_detect_technology",
    "detect_waf",
    "async_detect_waf",
    "validate_waf",
    "async_validate_waf",
    "fuzz_http",
    "async_fuzz_http",
    "generate_fuzz_payloads",
    "load_test_http",
    "async_load_test_http",
    "run_consolidated_recon",
    "async_run_consolidated_recon",
    "graphql_test",
    "async_graphql_test",
    "oauth_discover_endpoints",
    "oauth_test",
    "async_oauth_test",
    "auth_test",
    "async_auth_test",
    # Feature-gated operations
    "scan_git_secrets",
    "async_scan_git_secrets",
    "generate_sbom",
    "async_generate_sbom",
    "db_probe",
    "async_db_probe",
    "db_probe_with_config",
    "db_probe_postgres",
    "db_probe_mysql",
    "db_probe_mssql",
    "db_probe_mongodb",
    "db_probe_redis",
    "db_list_drivers",
    "db_get_capabilities",
    "db_run_with_config",
    "create_proxy_manager",
    "async_add_proxy",
    "async_proxy_health_check",
    "analyze_apk",
    "async_analyze_apk",
    "analyze_ipa",
    "async_analyze_ipa",
    "scan_docker_image",
    "async_scan_docker_image",
    "scan_kubernetes",
    "async_scan_kubernetes",
    "detect_escape_risks",
    "check_cis_docker_benchmark",
    "list_network_interfaces",
    "parse_pcap",
    "run_traceroute",
    "async_run_traceroute",
    "traceroute",
    "icmp_probe",
    "async_icmp_probe",
    "tcp_syn_probe",
    "async_tcp_syn_probe",
    "nse_run",
    "async_nse_run",
    "nse_list_libraries",
    "nse_list_scripts",
    "nse_get_script_metadata",
    "nse_list_libraries_detailed",
    "nse_get_library_descriptor",
    "nse_run_with_config",
    "nse_validate_script",
    # Provisional: network probes
    "dns_query",
    "async_dns_query",
    "tls_probe",
    "async_tls_probe",
    "http_probe",
    "async_http_probe",
    "udp_probe",
    "async_udp_probe",
    "resolve_target_sync",
    "async_resolve_target",
    "tcp_connect_probe",
    "async_tcp_connect_probe",
    "banner_probe",
    "async_banner_probe",
    "create_http_client",
    "async_create_http_client",
    # Provisional: WebSocket
    "websocket_probe",
    "async_websocket_probe",
    "websocket_fuzz",
    "async_websocket_fuzz",
    "websocket_assess",
    "async_websocket_assess",
    # Provisional: proxy
    "run_intercept_session",
    "async_run_intercept_session",
    # Provisional: daemon
    "daemon_connect",
    "async_daemon_health",
    "async_daemon_declare_client",
    "async_daemon_create_session",
    "async_daemon_list_sessions",
    "async_daemon_get_snapshot",
    "async_daemon_close_session",
    "async_daemon_submit_task",
    "async_daemon_cancel_task",
    "async_daemon_cancel_active",
    "async_daemon_approve_policy",
    "async_daemon_list_persisted_sessions",
    "async_daemon_get_persisted_snapshot",
    "async_daemon_subscribe",
    # Experimental
    "wireless_scan",
    "async_wireless_scan",
    "wireless_analyze_networks",
    "evasion_scan",
    "async_evasion_scan",
    "evasion_list_techniques",
    "postex_scan",
    "async_postex_scan",
    "postex_list_techniques",
    "c2_scan",
    "async_c2_scan",
    "c2_get_campaign",
    "hunt_test",
    "async_hunt_test",
    "ai_analyze_finding",
    "async_ai_analyze_finding",
    "ai_generate_payloads",
    "ai_suggest_waf_bypass",
    "ai_generate_script",
    "stress_test",
    "async_stress_test",
    "list_mobile_devices",
    "dynamic_mobile_analysis",
    "browser_test",
    "async_browser_test",
    # Core DTOs
    "Severity",
    "Evidence",
    "Finding",
    "FindingSet",
    "Report",
    "ExecutionStatus",
    "ExecutionStats",
    "Artifact",
    "OperationResult",
    "OperationError",
    "DispatchAuditEvent",
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
    "DnsRecordSet",
    "MxRecord",
    "SoaRecord",
    "TlsCertificateInfo",
    "TlsInspectionResult",
    "SslIssue",
    "TechStack",
    "TechDetectionResult",
    "WafDetectionResult",
    "BypassResult",
    "WafScanResult",
    "Payload",
    "FuzzResult",
    "FuzzSession",
    "FuzzConfig",
    "LoadTestResult",
    "LoadTestConfig",
    # Request types
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
    # Pipeline
    "PipelineStep",
    "StepResult",
    "PipelineResult",
    "Pipeline",
    "AsyncPipeline",
    "FailurePolicy",
    # Planning
    "PlanStep",
    "ScanPlan",
    # Checkpoint
    "Checkpoint",
    "CheckpointStore",
    "PipelineCheckpoint",
    "CheckpointLoadResult",
    "create_checkpoint_store",
    # Configuration
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
    # Consolidated Recon
    "ConsolidatedReconConfig",
    "ReconModuleResult",
    "ConsolidatedReconReport",
    # GraphQL
    "GraphQLVulnerability",
    "GraphQLTestResult",
    "GraphQLType",
    "GraphQLField",
    "GraphQLArg",
    "GraphQLInputField",
    "GraphQLSchema",
    "GraphQLTestConfig",
    # OAuth
    "OAuthVulnerability",
    "OAuthEndpointKind",
    "OAuthEndpoint",
    "OAuthTestResult",
    "OAuthTestConfig",
    # Auth
    "AuthTestType",
    "AuthFinding",
    "AuthTestConfig",
    "AuthTestReport",
    # Browser test
    "XssSource",
    "XssSink",
    "DomXssFinding",
    "DiscoveryMethod",
    "SpaRoute",
    "ClientIssueType",
    "ClientIssue",
    "BrowserTestConfig",
    "BrowserTestReport",
    # Hunt
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
    # Versioned findings
    "Confidence",
    "FindingType",
    "EvidenceKind",
    "AffectedAsset",
    "FindingLocation",
    "VersionedEvidence",
    "VersionedFinding",
    "MilestoneArtifact",
    "ArtifactReference",
    "ArtifactStore",
    "CvssScore",
    "VulnerabilityRecord",
    "RemediationRecord",
    "FindingState",
    "WorkflowTransition",
    "Suppression",
    "FindingWorkflow",
    "FindingRepository",
    "Assessment",
    "AssessmentRepository",
    "FindingCorrelation",
    "FindingDiff",
    "AssessmentDiff",
    "BaselineComparator",
    "FindingReporter",
    "SeveritySummary",
    "ReportEnvelope",
    "IntegrationType",
    "PublicationRecord",
    "RetryPolicy",
    "PublicationPolicy",
    "ExternalIntegration",
    "SchemaVersion",
    "MigrationResult",
    "FindingMigration",
    # Compliance
    "ComplianceFramework",
    "ComplianceControl",
    "ComplianceMapping",
    "ComplianceResult",
    "ControlAssessment",
    "ComplianceReport",
    "ComplianceMapper",
    # Distributed
    "DistributedTaskType",
    "WorkerStatus",
    "WorkerRegistration",
    "Heartbeat",
    "DistributedTask",
    "DistributedTaskResult",
    "distributed_task_types",
    "distributed_generate_psk",
    # Notifications
    "WebhookEvent",
    "FindingSummary",
    "NotifyScanStats",
    "WebhookConfig",
    "NotifyManager",
    "notify_scan_started",
    "notify_scan_complete",
    "notify_findings",
    "notify_error",
    # Event protocol
    "EventEnvelope",
    "PlanningEvent",
    "PreflightEvent",
    "StageLifecycleEvent",
    "ProgressEvent",
    "FindingEvent",
    "ArtifactEvent",
    "CancellationEvent",
    "FailureEvent",
    "CompletionEvent",
    "ResolutionEvent",
    "ConnectionEvent",
    "ProbeEvent",
    "WebSocketMessageEvent",
    "CaptureStatsEvent",
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
    "EventStreamAsyncIterator",
    "FindingStreamAsyncIterator",
    # Callbacks
    "AuditSink",
    "FindingSink",
    "ArtifactSink",
    "ProgressSink",
    "EventConsumer",
    "AsyncCallback",
    "CallbackScheduler",
    "BackpressureChannel",
    "EventDeliveryStats",
    # Session contract
    "SessionState",
    "SessionIdentity",
    "SessionStats",
    "SessionCloseMode",
    "SessionEvent",
    "SessionEventStream",
    "SessionCapabilities",
    "create_session_event",
    # Mobile session
    "MobileDeviceDescriptor",
    "MobileDeviceCapabilities",
    "MobileSessionConfig",
    "MobileSessionState",
    "MobileSessionStats",
    "MobileSession",
    "AsyncMobileSession",
    "MobileDeviceRegistry",
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
    # Browser session
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
    "BrowserDomEvent",
    "BrowserDownloadEvent",
    "BrowserSecurityObservation",
    # Daemon parity
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
    # Repositories
    "SqliteFindingRepository",
    "SqliteAssessmentRepository",
    "SqliteMigration",
    "SqliteMigrationResult",
    "JsonlFindingRepository",
    "JsonlAssessmentRepository",
    "ContentAddressedArtifactStore",
    "DirectoryArtifactStore",
    "ArtifactInfo",
    "ArtifactData",
    "IntegrityResult",
    "ArtifactQuery",
    # Streaming
    "StreamingReportConfig",
    "StreamingReporter",
    "ReportSummary",
    "StreamingDiffReporter",
    "FindingDiffResult",
    "DiffReportSummary",
    "ReportManifest",
    # Tool-core
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
    "OpenApiAdapter",
    "operation_as_tool",
    # Feature-gated classes
    "WebSocketReport",
    "WebSocketFinding",
    "ConnectionTestResult",
    "InjectionTestResult",
    "OriginTestResult",
    "FuzzTestResult",
    "WebSocketTestConfig",
    "GitSecretsReport",
    "GitSecretsSummary",
    "GitSecretFinding",
    "SecretFinding",
    "SbomReport",
    "SbomComponent",
    "SbomVulnerability",
    "SbomFormat",
    "DbPentestReport",
    "DbFinding",
    "DbPentestConfig",
    "MobilePlatform",
    "MobileFinding",
    "MobileScanReport",
    "ContainerScanType",
    "EscapeRiskLevel",
    "CisCheckStatus",
    "DockerScanResult",
    "KubernetesScanResult",
    "EscapeDetectionResult",
    "CisBenchmarkResult",
    "ContainerFinding",
    "ContainerReport",
    "CaptureConfig",
    "CaptureStats",
    "PacketInfo",
    "NetworkInterfaceInfo",
    "PcapWriter",
    "PacketFilter",
    "FlowRecord",
    "LiveCaptureResult",
    "TracerouteConfig",
    "TracerouteHop",
    "TracerouteResult",
    "BackpressurePolicy",
    "CaptureDropStats",
    "CapturedPacket",
    "AsyncCaptureSession",
    "EthernetFrame",
    "Ipv4Packet",
    "Ipv6Packet",
    "TcpSegment",
    "UdpDatagram",
    "IcmpPacket",
    "FlowKey",
    "FlowAggregator",
    "IcmpProbeConfig",
    "IcmpProbeReply",
    "IcmpProbeResult",
    "TcpProbeConfig",
    "TcpProbeResult",
    "PacketTimestampPy",
    "PacketStreamPy",
    "PacketArtifactPy",
    "SyncCaptureSessionPy",
    "DnsPacketPy",
    "TlsRecordInfoPy",
    "UdpReachabilityConfigPy",
    "UdpReachabilityResultPy",
    "udp_reachability",
    "StressType",
    "StressConfig",
    "StressStats",
    "StressResult",
    "NseConfig",
    "NseReport",
    "NseLibraryUse",
    "NseRuleEvaluation",
    "NseScriptMetadata",
    "NseSandboxPolicy",
    "NseTargetContext",
    "NseLibraryDescriptor",
    "NseArgument",
    "NseLibraryRegistry",
    "NseEvidenceItem",
    "NseExecutionLimits",
    "NseCancellationToken",
    "NseRuntimeStats",
    "NseRuntimeConfig",
    "NseRuntime",
    "NseScriptSource",
    "NseDiagnostic",
    "NseCapabilityContext",
    "NseHostContext",
    "NsePortContext",
    "NseRuleResult",
    "NseLibraryVersion",
    "NseLibraryConflict",
    "NseExecutionRequest",
    "NseExecutionResult",
    "NseScriptResult",
    "NseOutputValue",
    "DaemonClient",
    "DaemonResponse",
    "DaemonCapabilities",
    "TaskHandle",
    "TaskStatus",
    "SessionSummary",
    "TransportMetadata",
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
    # Backward-compatible Py-suffixed re-exports (use canonical names in new code)
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
    "AsyncTcpSessionPy",
    "AsyncUdpSocketPy",
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
    "WebSocketSessionConfigPy",
    "WebSocketMessagePy",
    "WebSocketFramePy",
    "WebSocketCloseInfoPy",
    "WebSocketHandshakePy",
    "WebSocketSessionPy",
    "AsyncWebSocketSessionPy",
    "WebSocketAssessmentConfigPy",
    "WebSocketAssessmentResultPy",
    "CaptureConfigPy",
    "PacketTimestampPy",
    "PacketStreamPy",
    "PacketArtifactPy",
    "SyncCaptureSessionPy",
    "DnsPacketPy",
    "TlsRecordInfoPy",
    "UdpReachabilityConfigPy",
    "UdpReachabilityResultPy",
]

# Keep the runtime export contract truthful for feature-gated builds.
for _name in tuple(__all__):
    if _name not in globals():
        __all__.remove(_name)


def __dir__() -> list[str]:
    """Include submodule names in dir() output."""
    return __all__ + ["net", "sessions", "storage", "reporting", "daemon", "experimental"]


def __getattr__(name: str):
    """Provide structured errors for unavailable feature-gated symbols.

    Raises AttributeError with structured guidance for known unavailable features,
    and AttributeError for truly unknown names. hasattr() returns False for both.
    """
    if name in _UNAVAILABLE_FEATURES:
        _unavailable_error(name)
    raise AttributeError(f"module 'eggsec' has no attribute {name!r}")
