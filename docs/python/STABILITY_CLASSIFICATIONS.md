# Stability Classifications

Every public API in the `eggsec` Python package is classified into a stability
level. This document is the canonical mapping from symbol to classification.

> Release boundary (2026-07-12): this package is pre-1.0. The only stable
> execution boundary is the twenty-two-operation stable core described in
> [domain-maturity.md](domain-maturity.md). Earlier milestone tables in this
> document describe API shape, not a blanket compatibility promise; use the
> machine-readable `api_surface()` and `domain_maturity()` results for the
> current classification.

## Stability levels

| Level | Guarantee |
|-------|-----------|
| **stable** | Will not change without a major version bump. Deprecation window before removal. |
| **stable (limited)** | Correct behavior guaranteed; output format or container may expand with new optional fields. |
| **experimental** | May change or be removed without notice. Lives in `eggsec.experimental` namespace. |
| **deprecated** | Will be removed. Replacement documented. |

---

## Stable — Core

- `Engine`
- `AsyncEngine`
- `Scope`
- `Client`
- `AsyncClient`
- `EggsecConfig`
- `SensitiveString` (frozen, `__repr__` redacted)
- `HttpConfig` (frozen)
- `ScanConfig` (frozen)
- `OutputConfig` (frozen)
- `ReconApiConfig` (frozen)
- `ReconConfig` (frozen)
- `ProxyConfigEntry` (frozen)
- `AllowedWorker` (frozen)
- `RemoteConfig` (frozen)
- `AiConfig` (frozen)
- `SearchConfig` (frozen)
- `PathsConfig` (frozen)
- `CacheConfig` (frozen)
- `AlertChannelConfig` (frozen)

## Stable — Scanning

- `scan_ports` / `async_scan_ports`
- `scan_endpoints` / `async_scan_endpoints`
- `fingerprint_services` / `async_fingerprint_services`
- `PortScanResult`
- `OpenPort`
- `ScanStats`
- `PortRange`
- `TimingPreset`
- `EndpointScanConfig`
- `EndpointFinding`
- `EndpointScanStats`
- `EndpointScanResult`
- `FingerprintEvidence`
- `FingerprintConfidence`
- `ServiceFingerprintResult`
- `FingerprintScanResult`

## Stable — Recon

- `recon_dns` / `async_recon_dns`
- `inspect_tls` / `async_inspect_tls`
- `detect_technology` / `async_detect_technology`
- `run_consolidated_recon` / `async_run_consolidated_recon`
- `DnsRecordSet`
- `MxRecord`
- `SoaRecord`
- `TlsCertificateInfo`
- `TlsInspectionResult`
- `SslIssue`
- `TechStack`
- `TechDetectionResult`
- `ConsolidatedReconConfig`
- `ReconModuleResult`
- `ConsolidatedReconReport`

## Stable — WAF

- `detect_waf` / `async_detect_waf`
- `validate_waf` / `async_validate_waf`
- `fuzz_http` / `async_fuzz_http`
- `generate_fuzz_payloads`
- `load_test_http` / `async_load_test_http`
- `WafDetectionResult`
- `BypassResult`
- `WafScanResult`
- `Payload`
- `FuzzResult`
- `FuzzSession`
- `FuzzConfig`
- `LoadTestResult`
- `LoadTestConfig`

## Stable — Findings

- `Severity` (frozen enum, `__hash__` and `__eq__`)
- `Evidence` (frozen)
- `Finding`
- `FindingSet`
- `Report`
- `Confidence` (enum: Confirmed, High, Medium, Low, Informational)
- `FindingType` (enum: Vulnerability, Misconfiguration, InformationLeak, PolicyViolation, ScanResult, ServiceDetection, AssetDiscovery, FuzzResult, WafDetection)
- `EvidenceKind` (enum: Screenshot, HttpRequest, HttpResponse, Header, BodySnippet, Certificate, DnsRecord, FilePath, LogLine, PortState, Timing, Banner, Diff)
- `AffectedAsset`
- `FindingLocation`
- `VersionedEvidence`
- `VersionedFinding`
- `FINDING_SCHEMA_VERSION` (constant)

## Stable — Pipeline

- `PipelineStep`
- `StepResult`
- `PipelineResult`
- `Pipeline`
- `AsyncPipeline`
- `PlanStep`
- `ScanPlan`
- `Checkpoint`
- `CheckpointStore`
- `OperationRequest`
- `PortScanRequest`
- `EndpointScanRequest`
- `FingerprintRequest`
- `ReconDnsRequest`
- `TlsInspectRequest`
- `TechDetectRequest`
- `WafDetectRequest`
- `LoadTestRequest`
- `WafValidateRequest`
- `FuzzRequest`
- `RequestBuilder`

## Stable — Enforcement

- `EnforcementContext`
- `ExecutionPolicy`
- `ManualOverride`
- `ExecutionSurface`
- `ExecutionProfile`
- `PolicyDecision`
- `EnforcementOutcome`
- `ApprovedOperation`
- `LoadedScope`
- `ScopeSource`
- `ScopeRule`
- `ScopeExplanation`
- `ScopeValidation`
- `PreflightResult`
- `AuditOutcome`
- `ManualOverrideAudit`
- `ScopeAudit`
- `EnforcementAuditEvent`

## Stable — Operation Metadata

- `OperationRegistry`
- `OperationMetadataView`
- `OperationDescriptor`
- `OperationRisk` (enum: passive, safe_active, intrusive, exploit_adjacent, and others)
- `OperationMode` (enum)
- `IntendedUse` (enum)
- `Capability` (enum)
- `DenialClass` (enum)
- `TargetPolicyKind` (enum)

## Stable — Domain Introspection

- `DomainDescriptor` (frozen)
- `DomainRegistry`

## Stable — Events

- `EventEnvelope`
- `EventStream`
- `ExecutionHandle`
- `ExecutionEvent`
- `EventLog`
- `CancellationToken`
- `PlanningEvent`
- `PreflightEvent`
- `StageLifecycleEvent`
- `ProgressEvent`
- `FindingEvent`
- `ArtifactEvent`
- `CancellationEvent`
- `FailureEvent`
- `CompletionEvent`
- `wrap_event` (function)
- `event_stream_from_legacy` (function)
- `EVENT_SCHEMA_VERSION` (constant)
- `EventStreamAsyncIterator`
- `FindingStreamAsyncIterator`

## Stable — Callbacks

- `AuditSink`
- `FindingSink`
- `ArtifactSink`
- `ProgressSink`
- `EventConsumer`
- `AsyncCallback`
- `CallbackScheduler`
- `BackpressureChannel`

## Stable — Buffers

The following types are registered in `_core` but are not re-exported in the
top-level `eggsec` namespace. They are accessible via `eggsec._core`:

- `BinaryBufferPy` (internal)
- `ArtifactMetaPy` (internal)
- `LazyArtifactPy` (internal)
- `PaginatedResultsPy` (internal)

These are implementation details for binary buffer protocol support and
lazy artifact loading. They may be promoted to the public API in a future
version.

## Stable — Introspection

- `api_surface` (function)
- `api_surface_version` (function)
- `features` (function)
- `has_feature` (function)
- `feature_matrix` (function)
- `build_info` (function)

## Stable — Audit / Preflight

- `validate_scope` (function)
- `preflight_operation` (function)
- `preflight_with_descriptor` (function)
- `audit_event_from_enforcement` (function)
- `audit_event_from_preflight` (function)
- `emit_audit_event` (function)

## Stable — Common Result Protocol

- `ExecutionStatus` (enum)
- `ExecutionStats`
- `Artifact`
- `OperationResult`

## Stable — Version Constants

- `__version__`
- `__version_info__`
- `__schema_version__`
- `__protocol_version__`
- `__abi_version__`
- `SCHEMA_VERSION`
- `PROTOCOL_VERSION`
- `ABI_VERSION`

## Stable — Milestone E: Artifacts

- `MilestoneArtifact`
- `ArtifactReference`
- `ArtifactStore`

## Stable — Milestone E: CVSS / Vulnerability Records

- `CvssScore`
- `VulnerabilityRecord`
- `RemediationRecord`

## Stable — Milestone E: Workflow

- `FindingState` (enum: New, Triaged, InProgress, Remediated, FalsePositive, AcceptedRisk, Confirmed, Reopened)
- `WorkflowTransition`
- `Suppression`
- `FindingWorkflow`

## Stable — Milestone E: Repository

- `FindingRepository`
- `Assessment`
- `AssessmentRepository`

## Stable — Milestone E: Baselines

- `FindingCorrelation`
- `FindingDiff`
- `AssessmentDiff`
- `BaselineComparator`

## Stable — Milestone E: Reporting

- `FindingReporter`
- `SeveritySummary`
- `ReportEnvelope`

## Stable — Milestone E: Integrations

- `IntegrationType` (enum: Jira, GitHub, GitLab, Webhook, Custom)
- `PublicationRecord`
- `RetryPolicy`
- `PublicationPolicy`
- `ExternalIntegration`

## Stable — Milestone E: Migration

- `SchemaVersion`
- `MigrationResult`
- `FindingMigration`

## Stable — Milestone C: GraphQL

- `graphql_test` / `async_graphql_test`
- `GraphQLVulnerability` (enum)
- `GraphQLTestResult`
- `GraphQLType`
- `GraphQLField`
- `GraphQLArg`
- `GraphQLInputField`
- `GraphQLSchema`
- `GraphQLTestConfig`

## Stable — Milestone C: OAuth

- `oauth_discover_endpoints` (function)
- `oauth_test` / `async_oauth_test`
- `OAuthVulnerability` (enum)
- `OAuthEndpointKind` (enum)
- `OAuthEndpoint`
- `OAuthTestResult`
- `OAuthTestConfig`

## Stable — Milestone C: Auth Assessment

- `auth_test` / `async_auth_test`
- `AuthTestType` (enum)
- `AuthFinding`
- `AuthTestConfig`
- `AuthTestReport`

## Stable — Milestone F: Distributed

- `DistributedTaskType` (enum)
- `WorkerStatus`
- `WorkerRegistration`
- `Heartbeat`
- `DistributedTask`
- `DistributedTaskResult`
- `distributed_task_types` (function)
- `distributed_generate_psk` (function)

## Stable — Milestone F: Notifications

- `WebhookEvent`
- `FindingSummary`
- `NotifyScanStats`
- `WebhookConfig`
- `NotifyManager`
- `notify_scan_started` (function)
- `notify_scan_complete` (function)
- `notify_findings` (function)
- `notify_error` (function)

## Stable (feature-gated) — when feature compiled

These are stable when available but only present when compiled with the
corresponding feature flag. `FeatureUnavailableError` is raised if
accessed when the feature is not compiled.

| Feature | Classes | Functions |
|---------|---------|-----------|
| `websocket` | WebSocketReport, WebSocketFinding, ConnectionTestResult, InjectionTestResult, OriginTestResult, FuzzTestResult, WebSocketTestConfig | websocket_probe, async_websocket_probe, websocket_fuzz, async_websocket_fuzz |
| `git-secrets` | GitSecretsReport, GitSecretsSummary, GitSecretFinding, SecretFinding, Confidence (git-secrets), SecretType | scan_git_secrets, async_scan_git_secrets |
| `sbom` | SbomReport, SbomComponent, SbomVulnerability, SbomFormat | generate_sbom, async_generate_sbom |
| `db-pentest` | DbPentestReport, DbFinding, DbPentestConfig, DbDriverInfo, DbCapability, DbCredentialProvider, DbSessionConfig | db_probe, async_db_probe, db_probe_with_config, db_probe_postgres, db_probe_mysql, db_probe_mssql, db_probe_mongodb, db_probe_redis, db_list_drivers, db_get_capabilities, db_run_with_config |
| `web-proxy` | ProxyType, RotationStrategy, ProxyConfig, ProxyEntry, ProxyManager, HealthCheckResult, ProxyHealth, InterceptConfig, CapturedExchange, InterceptSessionResult | create_proxy_manager, async_add_proxy, async_proxy_health_check |
| `mobile` | MobilePlatform, MobileFinding, MobileScanReport, MobileDevice, DynamicMobileConfig, DynamicMobileReport | analyze_apk, async_analyze_apk, analyze_ipa, async_analyze_ipa, list_mobile_devices, dynamic_mobile_analysis |
| `container` | ContainerScanType, EscapeRiskLevel, CisCheckStatus, DockerScanResult, KubernetesScanResult, EscapeDetectionResult, CisBenchmarkResult, ContainerFinding, ContainerReport, ImageLayer, DockerMisconfig, ClusterInfo, K8sFinding, EscapeRisk, CisCheck | scan_docker_image, async_scan_docker_image, scan_kubernetes, async_scan_kubernetes, detect_escape_risks, check_cis_docker_benchmark |
| `packet-inspection` | CaptureConfig, CaptureStats, PacketInfo, NetworkInterfaceInfo, PcapWriter, PacketFilter, FlowRecord, LiveCaptureResult, TracerouteConfig, TracerouteHop, TracerouteResult | list_network_interfaces, parse_pcap, run_traceroute, async_run_traceroute, traceroute |
| `stress-testing` | StressType, StressConfig, StressStats, StressConfigSummary, StressResult | stress_test, async_stress_test |
| `nse` | NseConfig, NseReport, NseLibraryUse, NseRuleEvaluation, NseScriptMetadata, NseSandboxPolicy, NseTargetContext | nse_run, async_nse_run, nse_list_libraries, nse_list_scripts, nse_get_script_metadata |
| `daemon-client` | DaemonClient, DaemonResponse, DaemonCapabilities, TaskHandle, TaskStatus, DaemonEvent, SessionSummary, TransportMetadata | daemon_connect, async_daemon_health, async_daemon_declare_client, async_daemon_create_session, async_daemon_list_sessions, async_daemon_get_snapshot, async_daemon_close_session |
| `headless-browser` | XssSource, XssSink, DomXssFinding, DiscoveryMethod, SpaRoute, ClientIssueType, ClientIssue, BrowserTestConfig, BrowserTestReport | browser_test, async_browser_test |
| `advanced-hunting` | ChainType, ChainStep, AttackChain, FlawType, BusinessLogicFlaw, RaceType, RaceCondition, BypassType, AuthzBypass, SessionIssueType, SessionIssue, HuntTestConfig, HuntReport | hunt_test, async_hunt_test |
| `compliance` | ComplianceFramework, ComplianceControl, ComplianceMapping, ComplianceResult, ControlAssessment, ComplianceReport, ComplianceMapper | -- |
| `wireless` | SecurityType, WirelessNetwork, WirelessVulnerability, WirelessScanResult, WirelessScanConfig | wireless_scan, async_wireless_scan, wireless_analyze_networks |
| `evasion` | EvasionTargetType, EvasionCategory, EvasionRisk, EvasionTechnique, EvasionDetection, EvasionSummary, EvasionReport, EvasionScanConfig | evasion_scan, async_evasion_scan, evasion_list_techniques |
| `postex` | PostexCategory, PostexRisk, PostexProfile, PostexTechnique, PostexDetection, PostexSummary, PostexReport, PostexScanConfig | postex_scan, async_postex_scan, postex_list_techniques |
| `c2` | BeaconProtocol, C2TaskType, C2TaskStatus, OpsecCategory, OpsecSeverity, CampaignPhase, C2Campaign, BeaconResult, C2TaskResult, OpsecFinding, OpsecAssessment, C2Summary, C2Report, C2ScanConfig | c2_scan, async_c2_scan, c2_get_campaign |
| `ai-integration` | AiProvider, PluginLanguage, AiAnalysisResult, AiPayloadSuggestion, AiWafBypassSuggestion, AiCacheStats, ScriptMetadata, GeneratedScript, AiCache | ai_analyze_finding, async_ai_analyze_finding, ai_generate_payloads, ai_suggest_waf_bypass |

## Stable (limited) — output formats

These types have guaranteed correct behavior, but their serialized output
(`to_dict()`, `to_json()`) may gain new optional fields in minor versions.

- All `to_dict()` output dicts
- All `to_json()` output strings
- All typed event payloads (PlanningEvent, PreflightEvent, etc.)
- `BinaryBuffer` output formats
- `LazyArtifact` metadata

## Deprecated

- `deprecated_warning` — replaced by `warnings.warn(msg, DeprecationWarning)` directly
- `DeprecatedWarning` class — replaced by stdlib `DeprecationWarning`

## Experimental

Wireless, evasion, post-exploitation, C2, browser, dynamic mobile, proxy,
packet-inspection, distributed, and AI-related domains are experimental. They
may require system dependencies, privileges, external providers, or hazardous
lab authorization and do not carry stable-core compatibility guarantees.

---

## api_surface() correspondence

The `api_surface()` function (defined in `src/lib.rs:807-1009`) returns a
dict mapping symbol names to `{"stability": "...", "deprecated": bool, ...}`.
Stable entries are limited to validated core execution and introspection
contracts. Provisional and experimental entries are intentionally present in
the surface map so callers can reject them before building compatibility-
sensitive workflows.

Symbols not in `api_surface()` but present in the public API are those
accessed via `try/except AttributeError` blocks in `__init__.py` (feature-gated)
and the `DeprecatedWarning` class. The `api_surface()` snapshot is tracked
in `tests/api_surface_snapshot.json` for regression detection.
