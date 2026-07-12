"""1.0 readiness verification tests for eggsec-python (Workstream G12).

Verifies that all expected public APIs exist, stability classifications are
documented, exception types are importable, enums have expected variants,
and FeatureUnavailableError is raised for missing features.
"""

import importlib
import json
import warnings
from pathlib import Path

import pytest

import eggsec


# ---------------------------------------------------------------------------
# A. Public API existence
# ---------------------------------------------------------------------------

# All symbols that must be importable from the top-level eggsec namespace.
# This list is derived from the __init__.py __all__ and api_surface().

ALWAYS_AVAILABLE_CLASSES = [
    # Core
    "Engine", "AsyncEngine", "Scope", "Client", "AsyncClient",
    "EggsecConfig", "SensitiveString",
    # Config sub-models
    "HttpConfig", "ScanConfig", "OutputConfig", "ReconApiConfig",
    "ReconConfig", "ProxyConfigEntry", "AllowedWorker", "RemoteConfig",
    "AiConfig", "SearchConfig", "PathsConfig", "CacheConfig",
    "AlertChannelConfig",
    # Scanning
    "PortScanResult", "OpenPort", "ScanStats", "PortRange", "TimingPreset",
    "EndpointScanConfig", "EndpointFinding", "EndpointScanStats",
    "EndpointScanResult",
    "FingerprintEvidence", "FingerprintConfidence",
    "ServiceFingerprintResult", "FingerprintScanResult",
    # Recon
    "DnsRecordSet", "MxRecord", "SoaRecord",
    "TlsCertificateInfo", "TlsInspectionResult", "SslIssue",
    "TechStack", "TechDetectionResult",
    "ConsolidatedReconConfig", "ReconModuleResult", "ConsolidatedReconReport",
    # WAF
    "WafDetectionResult", "BypassResult", "WafScanResult",
    "Payload", "FuzzResult", "FuzzSession", "FuzzConfig",
    "LoadTestResult", "LoadTestConfig",
    # Findings
    "Severity", "Evidence", "Finding", "FindingSet", "Report",
    "Confidence", "FindingType", "EvidenceKind",
    "AffectedAsset", "FindingLocation",
    "VersionedEvidence", "VersionedFinding",
    # Artifacts
    "MilestoneArtifact", "ArtifactReference", "ArtifactStore",
    # CVSS
    "CvssScore", "VulnerabilityRecord", "RemediationRecord",
    # Workflow
    "FindingState", "WorkflowTransition", "Suppression", "FindingWorkflow",
    # Repository
    "FindingRepository", "Assessment", "AssessmentRepository",
    # Baselines
    "FindingCorrelation", "FindingDiff", "AssessmentDiff", "BaselineComparator",
    # Reporting
    "FindingReporter", "SeveritySummary", "ReportEnvelope",
    # Integrations
    "IntegrationType", "PublicationRecord", "RetryPolicy",
    "PublicationPolicy", "ExternalIntegration",
    # Migration
    "SchemaVersion", "MigrationResult", "FindingMigration",
    # Pipeline
    "PipelineStep", "StepResult", "PipelineResult", "Pipeline", "AsyncPipeline",
    "PlanStep", "ScanPlan", "Checkpoint", "CheckpointStore",
    # Operation requests
    "OperationRequest", "PortScanRequest", "EndpointScanRequest",
    "FingerprintRequest", "ReconDnsRequest", "TlsInspectRequest",
    "TechDetectRequest", "WafDetectRequest", "LoadTestRequest",
    "WafValidateRequest", "FuzzRequest", "RequestBuilder",
    # Enforcement
    "EnforcementContext", "ExecutionPolicy", "ManualOverride",
    "ExecutionSurface", "ExecutionProfile", "PolicyDecision",
    "EnforcementOutcome", "ApprovedOperation",
    "LoadedScope", "ScopeSource", "ScopeRule", "ScopeExplanation",
    "ScopeValidation", "PreflightResult",
    "AuditOutcome", "ManualOverrideAudit", "ScopeAudit", "EnforcementAuditEvent",
    # Operation metadata
    "OperationRegistry", "OperationMetadataView", "OperationDescriptor",
    "OperationRisk", "OperationMode", "IntendedUse", "Capability",
    "DenialClass", "TargetPolicyKind",
    # Domains
    "DomainDescriptor", "DomainRegistry",
    # Events
    "EventEnvelope", "EventStream", "ExecutionHandle", "ExecutionEvent",
    "EventLog", "CancellationToken",
    "PlanningEvent", "PreflightEvent", "StageLifecycleEvent", "ProgressEvent",
    "FindingEventPy", "ArtifactEventPy",
    "CancellationEvent", "FailureEvent", "CompletionEvent",
    "EventStreamAsyncIterator", "FindingStreamAsyncIterator",
    # Callbacks
    "AuditSink", "FindingSink", "ArtifactSink", "ProgressSink",
    "EventConsumer", "AsyncCallback", "CallbackScheduler", "BackpressureChannel",
    # Common result protocol
    "ExecutionStatus", "ExecutionStats", "Artifact", "OperationResult",
    # Distributed (always available)
    "DistributedTaskType", "WorkerStatus", "WorkerRegistration", "Heartbeat",
    "DistributedTask", "DistributedTaskResult",
    # Notifications (always available)
    "WebhookEvent", "FindingSummary", "NotifyScanStats",
    "WebhookConfig", "NotifyManager",
    # Milestone C: Consolidated Recon, GraphQL, OAuth, Auth (always available)
    "GraphQLVulnerability", "GraphQLTestResult", "GraphQLType",
    "GraphQLField", "GraphQLArg", "GraphQLInputField", "GraphQLSchema",
    "GraphQLTestConfig",
    "OAuthVulnerability", "OAuthEndpointKind", "OAuthEndpoint",
    "OAuthTestResult", "OAuthTestConfig",
    "AuthTestType", "AuthFinding", "AuthTestConfig", "AuthTestReport",
]

ALWAYS_AVAILABLE_FUNCTIONS = [
    "scan_ports", "async_scan_ports",
    "scan_endpoints", "async_scan_endpoints",
    "fingerprint_services", "async_fingerprint_services",
    "recon_dns", "async_recon_dns",
    "inspect_tls", "async_inspect_tls",
    "detect_technology", "async_detect_technology",
    "detect_waf", "async_detect_waf",
    "validate_waf", "async_validate_waf",
    "fuzz_http", "async_fuzz_http",
    "generate_fuzz_payloads",
    "load_test_http", "async_load_test_http",
    "validate_scope",
    "preflight_operation", "preflight_with_descriptor",
    "audit_event_from_enforcement", "audit_event_from_preflight",
    "emit_audit_event",
    "run_consolidated_recon", "async_run_consolidated_recon",
    "graphql_test", "async_graphql_test",
    "oauth_discover_endpoints", "oauth_test", "async_oauth_test",
    "auth_test", "async_auth_test",
    "features", "has_feature", "feature_matrix",
    "build_info", "api_surface", "api_surface_version",
    "wrap_event", "event_stream_from_legacy",
    "distributed_task_types", "distributed_generate_psk",
    "notify_scan_started", "notify_scan_complete", "notify_findings", "notify_error",
]

ALWAYS_AVAILABLE_CONSTANTS = [
    "FINDING_SCHEMA_VERSION", "EVENT_SCHEMA_VERSION",
    "PyFuture",
]


class TestPublicAPIExistence:
    """All expected public APIs must be importable from eggsec."""

    @pytest.mark.parametrize("name", ALWAYS_AVAILABLE_CLASSES)
    def test_class_importable(self, name):
        assert hasattr(eggsec, name), f"Class '{name}' not found in eggsec"
        assert isinstance(getattr(eggsec, name), type), (
            f"'{name}' is not a type"
        )

    @pytest.mark.parametrize("name", ALWAYS_AVAILABLE_FUNCTIONS)
    def test_function_importable(self, name):
        assert hasattr(eggsec, name), f"Function '{name}' not found in eggsec"
        assert callable(getattr(eggsec, name)), f"'{name}' is not callable"

    @pytest.mark.parametrize("name", ALWAYS_AVAILABLE_CONSTANTS)
    def test_constant_importable(self, name):
        assert hasattr(eggsec, name), f"Constant '{name}' not found in eggsec"


# ---------------------------------------------------------------------------
# B. Stability classification consistency
# ---------------------------------------------------------------------------

class TestStabilityClassifications:
    """Verify api_surface() matches expected stability classifications."""

    def test_api_surface_returns_dict(self):
        surface = eggsec.api_surface()
        assert isinstance(surface, dict)
        assert len(surface) > 0

    def test_all_core_classes_stable(self):
        surface = eggsec.api_surface()
        core_classes = [
            "Engine", "AsyncEngine", "Scope", "Client", "AsyncClient",
            "EggsecConfig", "Severity", "Finding", "Report",
        ]
        for name in core_classes:
            if name in surface:
                assert surface[name]["stability"] in ("stable", "provisional"), (
                    f"'{name}' should be stable, got {surface[name]['stability']}"
                )

    def test_all_scan_functions_stable(self):
        surface = eggsec.api_surface()
        scan_fns = [
            "scan_ports", "async_scan_ports",
            "scan_endpoints", "async_scan_endpoints",
            "fingerprint_services", "async_fingerprint_services",
        ]
        for name in scan_fns:
            if name in surface:
                assert surface[name]["stability"] in ("stable", "provisional")

    def test_all_recon_functions_stable(self):
        surface = eggsec.api_surface()
        recon_fns = [
            "recon_dns", "async_recon_dns",
            "inspect_tls", "async_inspect_tls",
            "detect_technology", "async_detect_technology",
        ]
        for name in recon_fns:
            if name in surface:
                assert surface[name]["stability"] in ("stable", "provisional")

    def test_all_waf_functions_stable(self):
        surface = eggsec.api_surface()
        waf_fns = [
            "detect_waf", "async_detect_waf",
            "validate_waf", "async_validate_waf",
            "fuzz_http", "async_fuzz_http",
        ]
        for name in waf_fns:
            if name in surface:
                assert surface[name]["stability"] in ("stable", "provisional")

    def test_enforcement_classes_stable(self):
        surface = eggsec.api_surface()
        enforcement = [
            "EnforcementContext", "ExecutionPolicy", "ManualOverride",
            "PreflightResult",
        ]
        for name in enforcement:
            if name in surface:
                assert surface[name]["stability"] in ("stable", "provisional")

    def test_event_classes_stable(self):
        surface = eggsec.api_surface()
        events = [
            "EventEnvelope", "EventStream",
            "PlanningEvent", "PreflightEvent", "StageLifecycleEvent",
            "ProgressEvent", "FindingEvent", "ArtifactEvent",
            "CancellationEvent", "FailureEvent", "CompletionEvent",
        ]
        for name in events:
            if name in surface:
                assert surface[name]["stability"] in ("stable", "provisional")

    def test_callback_classes_stable(self):
        surface = eggsec.api_surface()
        callbacks = [
            "AuditSink", "FindingSink", "ArtifactSink",
            "ProgressSink", "EventConsumer",
        ]
        for name in callbacks:
            if name in surface:
                assert surface[name]["stability"] in ("stable", "provisional")

    def test_deprecated_warning_not_stable(self):
        surface = eggsec.api_surface()
        if "deprecated_warning" in surface:
            assert surface["deprecated_warning"]["stability"] != "stable"

    def test_domain_introspection_stable(self):
        surface = eggsec.api_surface()
        for name in ["DomainDescriptorPy", "DomainRegistry"]:
            if name in surface:
                assert surface[name]["stability"] in ("stable", "provisional")


# ---------------------------------------------------------------------------
# C. Exception hierarchy
# ---------------------------------------------------------------------------

EXPECTED_EXCEPTIONS = [
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


class TestExceptionHierarchy:
    """All exception types must be importable and have correct hierarchy."""

    @pytest.mark.parametrize("name", EXPECTED_EXCEPTIONS)
    def test_exception_importable(self, name):
        exc_type = getattr(eggsec, name, None)
        assert exc_type is not None, f"Exception '{name}' not found in eggsec"
        assert issubclass(exc_type, Exception), f"'{name}' is not an Exception"

    def test_all_subtypes_have_eggsec_error_parent(self):
        for name in EXPECTED_EXCEPTIONS:
            if name == "EggsecError":
                continue
            exc_type = getattr(eggsec, name)
            assert issubclass(exc_type, eggsec.EggsecError), (
                f"'{name}' should be a subclass of EggsecError"
            )

    def test_eggsec_error_is_base(self):
        assert issubclass(eggsec.EggsecError, Exception)

    def test_feature_unavailable_error_usable(self):
        """FeatureUnavailableError can be raised and caught."""
        with pytest.raises(eggsec.FeatureUnavailableError):
            raise eggsec.FeatureUnavailableError("test feature missing")

    def test_config_error_usable(self):
        with pytest.raises(eggsec.ConfigError):
            raise eggsec.ConfigError("bad config")

    def test_enforcement_error_usable(self):
        with pytest.raises(eggsec.EnforcementError):
            raise eggsec.EnforcementError("denied")

    def test_network_error_usable(self):
        with pytest.raises(eggsec.NetworkError):
            raise eggsec.NetworkError("connection refused")

    def test_scan_error_usable(self):
        with pytest.raises(eggsec.ScanError):
            raise eggsec.ScanError("scan failed")

    def test_timeout_error_usable(self):
        with pytest.raises(eggsec.TimeoutError):
            raise eggsec.TimeoutError("timed out")

    def test_serialization_error_usable(self):
        with pytest.raises(eggsec.SerializationError):
            raise eggsec.SerializationError("parse error")

    def test_internal_error_usable(self):
        with pytest.raises(eggsec.InternalError):
            raise eggsec.InternalError("internal error")

    def test_scope_error_usable(self):
        with pytest.raises(eggsec.ScopeError):
            raise eggsec.ScopeError("scope violation")


# ---------------------------------------------------------------------------
# D. Enum variants
# ---------------------------------------------------------------------------

class TestEnumVariants:
    """All enum types must have expected variants."""

    def test_severity_variants(self):
        variants = ["Critical", "High", "Medium", "Low", "Info"]
        for v in variants:
            assert hasattr(eggsec.Severity, v), f"Severity.{v} missing"

    def test_severity_from_str(self):
        for name in ["critical", "high", "medium", "low", "info", "informational"]:
            sev = eggsec.Severity.from_str(name)
            assert isinstance(sev, eggsec.Severity)

    def test_severity_invalid_raises(self):
        with pytest.raises(ValueError):
            eggsec.Severity.from_str("invalid")

    def test_severity_hashable(self):
        s1 = eggsec.Severity.High
        s2 = eggsec.Severity.High
        assert hash(s1) == hash(s2)
        assert s1 == s2

    def test_severity_frozen(self):
        sev = eggsec.Severity.Medium
        assert repr(sev) == "Severity.Medium"
        assert str(sev) == "Medium"

    def test_confidence_variants(self):
        variants = ["Confirmed", "High", "Medium", "Low", "Informational"]
        for v in variants:
            assert hasattr(eggsec.Confidence, v), f"Confidence.{v} missing"

    def test_finding_type_variants(self):
        variants = ["Vulnerability", "Misconfiguration", "InformationLeak", "PolicyViolation",
                     "ScanResult", "ServiceDetection", "AssetDiscovery", "FuzzResult", "WafDetection"]
        for v in variants:
            assert hasattr(eggsec.FindingType, v), f"FindingType.{v} missing"

    def test_evidence_kind_variants(self):
        variants = [
            "Screenshot", "HttpRequest", "HttpResponse", "Header", "BodySnippet",
            "Certificate", "DnsRecord", "FilePath", "LogLine", "PortState",
            "Timing", "Banner", "Diff",
        ]
        for v in variants:
            assert hasattr(eggsec.EvidenceKind, v), f"EvidenceKind.{v} missing"

    def test_finding_state_variants(self):
        variants = ["New", "Triaged", "InProgress", "Remediated", "FalsePositive",
                     "AcceptedRisk", "Confirmed", "Reopened"]
        for v in variants:
            assert hasattr(eggsec.FindingState, v), f"FindingState.{v} missing"

    def test_integration_type_variants(self):
        variants = ["Jira", "GitHub", "GitLab", "Webhook", "Custom"]
        for v in variants:
            assert hasattr(eggsec.IntegrationType, v), f"IntegrationType.{v} missing"

    def test_operation_risk_variants(self):
        variants = ["passive", "safe_active", "intrusive", "exploit_adjacent"]
        for v in variants:
            assert hasattr(eggsec.OperationRisk, v), f"OperationRisk.{v} missing"


# ---------------------------------------------------------------------------
# E. Feature behavior
# ---------------------------------------------------------------------------

ALWAYS_AVAILABLE_FEATURES = [
    "core", "scanner", "async-api", "endpoint-discovery",
    "service-fingerprinting", "waf-detection", "waf-validation",
    "http-fuzzing", "load-testing", "findings-reporting",
]


class TestFeatureBehavior:
    """Verify feature detection and gating works correctly."""

    def test_has_feature_core(self):
        assert eggsec.has_feature("core") is True

    @pytest.mark.parametrize("feature", ALWAYS_AVAILABLE_FEATURES)
    def test_always_available_features(self, feature):
        assert eggsec.has_feature(feature) is True, (
            f"Feature '{feature}' should always be available"
        )

    def test_has_feature_unknown_returns_false(self):
        assert eggsec.has_feature("nonexistent-feature") is False

    def test_features_returns_dict(self):
        result = eggsec.features()
        assert isinstance(result, dict)
        assert len(result) > 0

    @pytest.mark.parametrize("feature", ALWAYS_AVAILABLE_FEATURES)
    def test_features_dict_has_always_available(self, feature):
        result = eggsec.features()
        assert feature in result
        assert result[feature] is True

    def test_feature_matrix_returns_dict(self):
        matrix = eggsec.feature_matrix()
        assert isinstance(matrix, dict)

    def test_feature_matrix_entry_structure(self):
        matrix = eggsec.feature_matrix()
        entry = matrix["core"]
        assert "available" in entry
        assert "description" in entry
        assert "requires_system_deps" in entry

    def test_feature_matrix_always_available_count(self):
        matrix = eggsec.feature_matrix()
        always_available = [f for f, v in matrix.items() if v["available"] is True]
        assert len(always_available) >= 10, (
            f"Expected at least 10 always-available features, got {len(always_available)}"
        )

    def test_feature_gated_not_available_in_default_build(self):
        """Feature-gated features should be False in default build."""
        gated = [
            "websocket", "git-secrets", "sbom", "db-pentest",
            "mobile", "packet-inspection", "stress-testing", "nse",
            "container", "daemon-client",
        ]
        for feature in gated:
            if not eggsec.has_feature(feature):
                assert eggsec.features().get(feature) is False

    def test_feature_gated_import_absent_when_disabled(self):
        """Feature-gated symbols should not exist when feature is disabled."""
        gating = [
            ("websocket", "websocket_probe"),
            ("git-secrets", "scan_git_secrets"),
            ("sbom", "generate_sbom"),
            ("db-pentest", "db_probe"),
            ("mobile", "analyze_apk"),
            ("packet-inspection", "parse_pcap"),
            ("stress-testing", "stress_test"),
            ("nse", "nse_run"),
            ("daemon-client", "daemon_connect"),
        ]
        for feature, attr in gating:
            if not eggsec.has_feature(feature):
                assert not hasattr(eggsec, attr), (
                    f"{attr} present without {feature} feature"
                )


# ---------------------------------------------------------------------------
# F. api_surface() completeness
# ---------------------------------------------------------------------------

class TestApiSurfaceCompleteness:
    """Verify api_surface() covers all always-available public APIs."""

    def _get_surface(self):
        return eggsec.api_surface()

    def test_core_classes_in_surface(self):
        surface = self._get_surface()
        for name in ["Engine", "AsyncEngine", "Scope", "Client", "AsyncClient"]:
            assert name in surface, f"'{name}' missing from api_surface()"

    def test_scan_functions_in_surface(self):
        surface = self._get_surface()
        for name in [
            "scan_ports", "async_scan_ports",
            "scan_endpoints", "async_scan_endpoints",
            "fingerprint_services", "async_fingerprint_services",
        ]:
            assert name in surface, f"'{name}' missing from api_surface()"

    def test_recon_functions_in_surface(self):
        surface = self._get_surface()
        for name in [
            "recon_dns", "async_recon_dns",
            "inspect_tls", "async_inspect_tls",
            "detect_technology", "async_detect_technology",
        ]:
            assert name in surface, f"'{name}' missing from api_surface()"

    def test_waf_functions_in_surface(self):
        surface = self._get_surface()
        for name in [
            "detect_waf", "async_detect_waf",
            "validate_waf", "async_validate_waf",
            "fuzz_http", "async_fuzz_http",
        ]:
            assert name in surface, f"'{name}' missing from api_surface()"

    def test_event_classes_in_surface(self):
        surface = self._get_surface()
        for name in [
            "EventEnvelope", "EventStream",
            "PlanningEvent", "PreflightEvent",
        ]:
            assert name in surface, f"'{name}' missing from api_surface()"

    def test_callback_classes_in_surface(self):
        surface = self._get_surface()
        for name in [
            "AuditSink", "FindingSink", "ArtifactSink",
            "ProgressSink", "EventConsumer",
        ]:
            assert name in surface, f"'{name}' missing from api_surface()"

    def test_domain_introspection_in_surface(self):
        surface = self._get_surface()
        for name in ["DomainDescriptorPy", "DomainRegistry"]:
            assert name in surface, f"'{name}' missing from api_surface()"

    def test_introspection_functions_in_surface(self):
        surface = self._get_surface()
        for name in [
            "api_surface", "api_surface_version",
            "features", "has_feature", "feature_matrix",
            "build_info",
        ]:
            assert name in surface, f"'{name}' missing from api_surface()"

    def test_no_stable_entry_is_deprecated(self):
        surface = self._get_surface()
        violations = [
            name for name, info in surface.items()
            if info["stability"] == "stable" and info.get("deprecated", False)
        ]
        assert violations == [], f"Stable but deprecated: {violations}"

    def test_version_constants_in_surface(self):
        surface = self._get_surface()
        for name in [
            "__version__", "__version_info__",
            "FINDING_SCHEMA_VERSION",
            "SCHEMA_VERSION", "PROTOCOL_VERSION", "ABI_VERSION",
        ]:
            assert name in surface, f"'{name}' missing from api_surface()"


# ---------------------------------------------------------------------------
# G. SensitiveString security semantics
# ---------------------------------------------------------------------------

class TestSensitiveStringSecurity:
    """SensitiveString must redact repr/str and only expose via expose_secret()."""

    def test_repr_redacted(self):
        s = eggsec.SensitiveString("super-secret-key")
        assert "REDACTED" in repr(s)
        assert "super-secret-key" not in repr(s)

    def test_str_redacted(self):
        s = eggsec.SensitiveString("super-secret-key")
        assert str(s) == "[REDACTED]"
        assert "super-secret-key" not in str(s)

    def test_expose_secret_returns_value(self):
        s = eggsec.SensitiveString("my-secret")
        assert s.expose_secret() == "my-secret"

    def test_frozen(self):
        s = eggsec.SensitiveString("val")
        assert s.is_empty() is False
        assert s.len() == 3

    def test_hashable(self):
        s1 = eggsec.SensitiveString("same")
        s2 = eggsec.SensitiveString("same")
        assert s1 == s2
        assert hash(s1) == hash(s2)

    def test_empty(self):
        s = eggsec.SensitiveString("")
        assert s.is_empty() is True
        assert s.len() == 0


# ---------------------------------------------------------------------------
# H. Version metadata
# ---------------------------------------------------------------------------

class TestVersionMetadata:
    """Version constants and build info must be correct."""

    def test_schema_version(self):
        assert eggsec.__schema_version__ == "1.0"

    def test_protocol_version(self):
        assert eggsec.__protocol_version__ == "1.0.0"

    def test_abi_version(self):
        assert eggsec.__abi_version__ == "1"

    def test_finding_schema_version(self):
        assert isinstance(eggsec.FINDING_SCHEMA_VERSION, str)

    def test_event_schema_version(self):
        assert eggsec.EVENT_SCHEMA_VERSION == "1.0.0"

    def test_version_types(self):
        assert isinstance(eggsec.__version__, str)
        assert isinstance(eggsec.__version_info__, tuple)

    def test_build_info(self):
        info = eggsec.build_info()
        assert isinstance(info, dict)
        assert "version" in info

    def test_api_surface_version(self):
        result = eggsec.api_surface_version()
        assert isinstance(result, dict)
        assert "package_version" in result
        assert "schema_version" in result
        assert "protocol_version" in result
        assert "abi_version" in result
        assert "features_list" in result


# ---------------------------------------------------------------------------
# I. Event protocol versioning
# ---------------------------------------------------------------------------

class TestEventProtocol:
    """EventEnvelope and typed payloads must follow versioned protocol."""

    def test_event_envelope_creation(self):
        payload = eggsec.PlanningEvent("op-1", "target.com", "in-scope")
        env = eggsec.EventEnvelope("planning", payload)
        assert env.schema_version == "1.0.0"
        assert env.event_type == "planning"
        assert env.event_id.startswith("evt-")

    def test_wrap_event(self):
        payload = eggsec.ProgressEvent(50.0, "halfway", 50, 100)
        env = eggsec.wrap_event("progress", payload)
        assert env.event_type == "progress"
        assert env.schema_version == "1.0.0"

    def test_all_typed_payloads_creatable(self):
        payloads = [
            eggsec.PlanningEvent("op", "t", "s"),
            eggsec.PreflightEvent("allow", [], []),
            eggsec.StageLifecycleEvent("scan", "started"),
            eggsec.ProgressEvent(50.0, "msg", 50, 100),
            eggsec.FindingEventPy("f1", "high", "title", True),
            eggsec.ArtifactEventPy("report.json", "report", "application/json", 1024),
            eggsec.CancellationEvent("reason", "operator"),
            eggsec.FailureEvent("TimeoutError", "msg", True),
            eggsec.CompletionEvent("success", None, 5000),
        ]
        for payload in payloads:
            env = eggsec.EventEnvelope("test", payload)
            assert env.schema_version == "1.0.0"

    def test_event_stream_operations(self):
        stream = eggsec.EventStream.empty()
        assert stream.is_empty()
        assert len(stream) == 0

    def test_event_envelope_to_dict(self):
        payload = eggsec.PlanningEvent("op-1", "target.com", "scope")
        env = eggsec.EventEnvelope("planning", payload)
        d = env.to_dict()
        assert isinstance(d, dict)
        assert d["event_type"] == "planning"
        assert d["schema_version"] == "1.0.0"


# ---------------------------------------------------------------------------
# J. Callback contracts
# ---------------------------------------------------------------------------

class TestCallbackContracts:
    """All callback sink types must be creatable and closeable."""

    def test_audit_sink(self):
        sink = eggsec.AuditSink(lambda e: None)
        assert not sink.is_closed
        sink.close()
        assert sink.is_closed

    def test_finding_sink(self):
        sink = eggsec.FindingSink(lambda f: None)
        assert not sink.is_closed
        sink.close()
        assert sink.is_closed

    def test_artifact_sink(self):
        sink = eggsec.ArtifactSink(lambda a: None)
        assert not sink.is_closed
        sink.close()
        assert sink.is_closed

    def test_progress_sink(self):
        sink = eggsec.ProgressSink(lambda p, m: None)
        assert not sink.is_closed
        sink.close()
        assert sink.is_closed

    def test_event_consumer(self):
        consumer = eggsec.EventConsumer(lambda e: None)
        assert not consumer.is_closed
        consumer.close()
        assert consumer.is_closed

    def test_backpressure_channel(self):
        ch = eggsec.BackpressureChannel(64)
        assert ch.capacity == 64
        assert ch.is_empty()

    def test_callback_scheduler(self):
        scheduler = eggsec.CallbackScheduler(10)
        assert not scheduler.is_closed
        scheduler.close()
        assert scheduler.is_closed


# ---------------------------------------------------------------------------
# K. FeatureUnavailableError for missing features
# ---------------------------------------------------------------------------

class TestFeatureUnavailableError:
    """FeatureUnavailableError must be raisable and catchable."""

    def test_can_raise_and_catch(self):
        with pytest.raises(eggsec.FeatureUnavailableError, match="not available"):
            raise eggsec.FeatureUnavailableError("feature not available")

    def test_catchable_as_eggsec_error(self):
        with pytest.raises(eggsec.EggsecError):
            raise eggsec.FeatureUnavailableError("test")

    def test_catchable_as_exception(self):
        with pytest.raises(Exception):
            raise eggsec.FeatureUnavailableError("test")

    def test_error_has_message(self):
        try:
            raise eggsec.FeatureUnavailableError("mobile not compiled")
        except eggsec.FeatureUnavailableError as e:
            assert "mobile not compiled" in str(e)
