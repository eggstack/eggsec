"""Eggsec: Python bindings for the Rust security assessment engine.

This is a host-language binding, not an internal plugin runtime.
The core engine is implemented in Rust; this package provides a Python-native API.
"""

from . import _core

__version__ = _core.__version__
__version_info__ = _core.__version_info__

# Re-export functions
features = _core.features
has_feature = _core.has_feature
build_info = _core.build_info
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
except AttributeError:
    pass

# Phase F Track 4: Git secrets
try:
    scan_git_secrets = _core.scan_git_secrets
    async_scan_git_secrets = _core.async_scan_git_secrets
except AttributeError:
    pass

# Phase F Track 5: SBOM
try:
    generate_sbom = _core.generate_sbom
    async_generate_sbom = _core.async_generate_sbom
except AttributeError:
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
except AttributeError:
    pass

# Phase F Track 7: Proxy
try:
    create_proxy_manager = _core.create_proxy_manager
    async_add_proxy = _core.async_add_proxy
    async_proxy_health_check = _core.async_proxy_health_check
except AttributeError:
    pass

# Phase F Track 8: Mobile
try:
    analyze_apk = _core.analyze_apk
    async_analyze_apk = _core.async_analyze_apk
    analyze_ipa = _core.analyze_ipa
    async_analyze_ipa = _core.async_analyze_ipa
except AttributeError:
    pass

# Phase F Track 9: Container
try:
    scan_docker_image = _core.scan_docker_image
    async_scan_docker_image = _core.async_scan_docker_image
    scan_kubernetes = _core.scan_kubernetes
    async_scan_kubernetes = _core.async_scan_kubernetes
    detect_escape_risks = _core.detect_escape_risks
    check_cis_docker_benchmark = _core.check_cis_docker_benchmark
except AttributeError:
    pass

# Phase F Track 10: Packet inspection
try:
    list_network_interfaces = _core.list_network_interfaces
    parse_pcap = _core.parse_pcap
except AttributeError:
    pass

# Phase F Track 11: Stress testing
try:
    stress_test = _core.stress_test
    async_stress_test = _core.async_stress_test
except AttributeError:
    pass

# Phase F Track 12: NSE
try:
    nse_run = _core.nse_run
    async_nse_run = _core.async_nse_run
    nse_list_libraries = _core.nse_list_libraries
except AttributeError:
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
except AttributeError:
    pass

# Re-export classes
Scope = _core.Scope
Client = _core.Client
AsyncClient = _core.AsyncClient
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
except AttributeError:
    pass

# Phase F Track 4: Git secrets (feature-gated)
try:
    GitSecretsReport = _core.GitSecretsReportPy
    GitSecretsSummary = _core.GitSecretsSummaryPy
    GitSecretFinding = _core.GitSecretFindingPy
    SecretFinding = _core.SecretFindingPy
except AttributeError:
    pass

# Phase F Track 5: SBOM (feature-gated)
try:
    SbomReport = _core.SbomReportPy
    SbomComponent = _core.SbomComponentPy
    SbomVulnerability = _core.SbomVulnerabilityPy
    SbomFormat = _core.SbomFormatPy
except AttributeError:
    pass

# Phase F Track 6: Database pentesting (feature-gated)
try:
    DbPentestReport = _core.DbPentestReportPy
    DbFinding = _core.DbFindingPy
    DbPentestConfig = _core.DbPentestConfig
except AttributeError:
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
except AttributeError:
    pass

# Phase F Track 8: Mobile (feature-gated)
try:
    MobilePlatform = _core.MobilePlatformPy
    MobileFinding = _core.MobileFindingPy
    MobileScanReport = _core.MobileScanReportPy
except AttributeError:
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
except AttributeError:
    pass

# Phase F Track 10: Packet inspection (feature-gated)
try:
    CaptureConfig = _core.CaptureConfigPy
    CaptureStats = _core.CaptureStatsPy
    PacketInfo = _core.PacketInfoPy
    NetworkInterfaceInfo = _core.NetworkInterfaceInfoPy
    PcapWriter = _core.PcapWriterPy
except AttributeError:
    pass

# Phase F Track 11: Stress testing (feature-gated)
try:
    StressType = _core.StressTypePy
    StressConfig = _core.StressConfigPy
    StressStats = _core.StressStatsPy
    StressResult = _core.StressResultPy
except AttributeError:
    pass

# Phase F Track 12: NSE (feature-gated)
try:
    NseConfig = _core.NseConfigPy
    NseReport = _core.NseReportPy
    NseLibraryUse = _core.NseLibraryUsePy
    NseRuleEvaluation = _core.NseRuleEvaluationPy
except AttributeError:
    pass

# Phase F Track 13: Daemon client (feature-gated)
try:
    DaemonClient = _core.DaemonClientPy
    DaemonResponse = _core.DaemonResponsePy
except AttributeError:
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

__all__ = [
    "__version__",
    "__version_info__",
    # Functions
    "features",
    "has_feature",
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
]
