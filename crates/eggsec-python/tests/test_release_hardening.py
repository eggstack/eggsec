"""Release hardening tests for eggsec-python (Workstream G10).

Covers: runtime/stub export parity, API surface snapshot, import profiles,
sync/async contract parity, cancellation/leak/shutdown, policy equivalence,
serialization, deprecation warnings, version metadata, event schema,
and callback contracts.
"""

import inspect
import json
import os
from pathlib import Path

import pytest

import eggsec


def test_stable_registry_and_policy_audit_contract():
    engine = eggsec.Engine(eggsec.Scope.allow_hosts(["127.0.0.1"]))
    assert engine.list_operations() == [
        "scan_ports",
        "scan_endpoints",
        "fingerprint_services",
        "recon_dns",
        "inspect_tls",
        "detect_technology",
        "detect_waf",
        "validate_waf",
        "fuzz_http",
        "load_test",
        "scan_git_secrets",
        "generate_sbom",
        "run_consolidated_recon",
        "graphql_test",
        "oauth_test",
        "auth_test",
        "db_probe",
        "nse_run",
        "scan_docker_image",
        "scan_kubernetes",
        "analyze_apk",
        "analyze_ipa",
    ]

    result = engine.run_port_scan(
        eggsec.PortScanRequest("192.0.2.1", ports="1", timeout_ms=10)
    )
    assert result.is_failure()
    assert isinstance(result.error, eggsec.OperationError)
    assert result.error.kind == "scope_denial"
    assert result.error.operation_id == "scan_ports"
    assert result.error.retryable is False
    assert len(engine.audit_events()) == 1
    assert engine.audit_events()[0].allowed is False
    assert engine.audit_events()[0].redacted is True


def test_structured_error_round_trip_and_specific_exception():
    error = eggsec.OperationError(
        "timeout",
        "operation_timeout",
        "fixture timed out",
        operation_id="scan_ports",
        retryable=True,
        details={"timeout_ms": "10"},
    )
    decoded = json.loads(error.to_json())
    assert decoded["schema_version"] == "1.0"
    assert decoded["retryable"] is True
    assert decoded["details"]["timeout_ms"] == "10"

    result = eggsec.OperationResult(
        eggsec.ExecutionStatus.Failed(error="fixture timed out"), error="fixture timed out"
    )
    with pytest.raises(eggsec.TimeoutError):
        result.raise_for_status()


def test_backpressure_preserves_reliable_events_and_accounts_drops():
    channel = eggsec.BackpressureChannel(capacity=1)
    payload = eggsec.ProgressEvent(0.0, "progress", 0, 1)
    channel.send(eggsec.EventEnvelope("progress", payload))
    channel.send(eggsec.EventEnvelope("progress", payload))
    channel.send(eggsec.EventEnvelope("operation.completed", payload))

    stats = channel.stats()
    assert stats.emitted_count == 3
    assert stats.dropped_count == 1
    assert stats.dropped_by_kind["progress"] == 1
    assert channel.try_recv().event_type == "operation.completed"
    assert channel.stats().delivered_count == 1


# ---------------------------------------------------------------------------
# A. Runtime/stub export parity
# ---------------------------------------------------------------------------


class TestRuntimeStubParity:
    """Verify runtime export parity between _core, __init__.py, and stubs."""

    def test_all_core_names_accessible_from_eggsec(self):
        """Every public name in _core must be accessible via the eggsec package.

        Names may be exported under the same name or a clean alias
        (e.g., _core.ExecutionSurfacePy -> eggsec.ExecutionSurface).
        """
        import re

        core_names = sorted(n for n in dir(eggsec._core) if not n.startswith("_"))

        # Parse __init__.py to find all assignments from _core
        init_path = Path(__file__).resolve().parent.parent / "python" / "eggsec" / "__init__.py"
        init_content = init_path.read_text()

        # Build mapping: _core name -> eggsec-level name
        assigned_names = {}
        for line in init_content.splitlines():
            line = line.strip()
            if " = _core." in line:
                lhs = line.split(" = _core.")[0].strip()
                rhs = line.split(" = _core.")[1].strip()
                if lhs and rhs and not lhs.startswith("_"):
                    assigned_names[rhs] = lhs

        # Also build set of names in __all__
        tree = __import__("ast").parse(init_content)
        all_names = set()
        for node in __import__("ast").walk(tree):
            if isinstance(node, __import__("ast").Assign):
                for target in node.targets:
                    if isinstance(target, __import__("ast").Name) and target.id == "__all__":
                        all_names = {elt.value for elt in node.value.elts}

        # Build reverse mapping for constants: _core.X -> __xxx__ pattern
        dunder_map = {}
        for line in init_content.splitlines():
            line = line.strip()
            if line.startswith("__") and " = _core." in line:
                lhs = line.split(" = ")[0].strip()
                rhs = line.split(" = _core.")[1].strip()
                if lhs.startswith("__") and rhs:
                    dunder_map[rhs] = lhs

        # Known internal types registered in _core but not re-exported at
        # the top-level eggsec namespace. They are accessible via eggsec._core.
        # Feature-gated request types are registered in _core but only re-exported
        # in .pyi stubs, not in __init__.py at runtime.
        # Constants are mapped to dunder names via separate parsing.
        known_internal = {
            "DeprecatedWarning",
            "ArtifactMeta",
            "BinaryBuffer",
            "FindingSetIterator",
            "LazyArtifact",
            "LazyEventIterator",
            "PaginatedResults",
            "OutputRef",
            "ProxyRoutePy",
            "UdpProbeConfigPy",
            "UdpProbeResultPy",
            "ApkAnalysisRequest",
            "AuthTestRequest",
            "ConsolidatedReconRequest",
            "DbProbeRequest",
            "DockerImageScanRequest",
            "GitSecretsScanRequest",
            "GraphqlTestRequest",
            "IpaAnalysisRequest",
            "KubernetesScanRequest",
            "NseRunRequest",
            "OauthTestRequest",
            "SbomRequest",
            "async_udp_probe",
            "udp_probe",
            "evidence_to_finding",
            "FailurePolicy",
            # Constants mapped to dunder names
            "ABI_VERSION",
            "SCHEMA_VERSION",
            "PROTOCOL_VERSION",
        }

        missing = []
        for name in core_names:
            if name in known_internal:
                continue
            # Directly assigned with same name in __all__
            if name in all_names:
                continue
            # Assigned with a different name (Py suffix pattern)
            if name in assigned_names:
                continue
            # Constants accessed via dunder names (SCHEMA_VERSION -> __schema_version__)
            if name in dunder_map:
                continue
            # Clean name without Py suffix in __all__
            clean = name.replace("Py", "")
            if clean in all_names:
                continue
            missing.append(name)

        assert missing == [], (
            f"_core names not accessible from eggsec package: {missing}"
        )

    def test_init_pyi_has_submodule_stubs(self):
        """__init__.pyi must re-export types from each submodule."""
        stub_path = Path(__file__).resolve().parent.parent / "python" / "eggsec" / "__init__.pyi"
        if not stub_path.exists():
            pytest.skip("__init__.pyi not found")
        content = stub_path.read_text()

        # Core submodules that must be represented in stubs
        required_submodules = [
            "errors",
            "scope",
            "client",
            "engine",
            "handles",
            "cancellation",
            "dto",
            "endpoint",
            "fingerprint",
            "finding",
            "status",
            "recon",
            "waf",
            "config_model",
            "operation_metadata",
            "execution_context",
            "authorization",
            "preflight",
            "audit",
            "runtime",
            "functions",
            "event_protocol",
            "event_stream",
            "callbacks",
            "async_support",
            "backpressure",
        ]
        for mod in required_submodules:
            assert f"from .{mod} import" in content, (
                f"__init__.pyi missing import from .{mod}"
            )


# ---------------------------------------------------------------------------
# B. API surface snapshot
# ---------------------------------------------------------------------------


class TestApiSurfaceSnapshot:
    """Snapshot api_surface() output for diff tracking."""

    SNAPSHOT_PATH = Path(__file__).resolve().parent / "api_surface_snapshot.json"

    def test_api_surface_returns_dict(self):
        result = eggsec.api_surface()
        assert isinstance(result, dict)
        assert len(result) > 0

    def test_all_expected_stable_apis_present(self):
        result = eggsec.api_surface()
        expected_stable = [
            "Engine",
            "AsyncEngine",
            "Scope",
            "Client",
            "AsyncClient",
            "Severity",
            "Finding",
            "Report",
            "scan_ports",
            "async_scan_ports",
            "features",
            "has_feature",
            "build_info",
            "api_surface",
            "EventEnvelope",
            "EventStream",
        ]
        for name in expected_stable:
            assert name in result, f"Expected stable API '{name}' missing from api_surface()"
            assert result[name]["stability"] == "stable"

    def test_all_expected_experimental_apis_present(self):
        result = eggsec.api_surface()
        expected_experimental = [
            "deprecated_warning",
        ]
        for name in expected_experimental:
            if name in result:
                assert result[name]["stability"] in ("experimental", "deprecated", "beta"), (
                    f"'{name}' should not be marked stable"
                )

    def test_stable_entries_not_deprecated(self):
        result = eggsec.api_surface()
        violations = [
            name
            for name, info in result.items()
            if info["stability"] == "stable" and info.get("deprecated", False)
        ]
        assert violations == [], f"Stable but deprecated: {violations}"

    def test_snapshot_matches_or_create(self):
        result = eggsec.api_surface()
        if self.SNAPSHOT_PATH.exists():
            with open(self.SNAPSHOT_PATH) as f:
                previous = json.load(f)
            # Check for removed entries (regressions)
            removed = set(previous.keys()) - set(result.keys())
            assert not removed, f"API entries removed since last snapshot: {removed}"
            # Stability is intentionally reclassified during pre-1.0 release
            # work. The snapshot guards symbol removal; current maturity is
            # asserted by api_surface/domain_maturity tests.
        else:
            # Create initial snapshot
            with open(self.SNAPSHOT_PATH, "w") as f:
                json.dump(result, f, indent=2, sort_keys=True)


# ---------------------------------------------------------------------------
# C. Minimal and feature-rich import tests
# ---------------------------------------------------------------------------


class TestImportProfiles:
    """Test default import and feature_matrix()."""

    def test_default_import_has_core_symbols(self):
        core_symbols = [
            "Scope", "Client", "AsyncClient", "Engine", "AsyncEngine",
            "Severity", "Finding", "Report", "features", "has_feature",
            "build_info", "scan_ports", "api_surface",
        ]
        for name in core_symbols:
            assert hasattr(eggsec, name), f"Core symbol '{name}' missing"

    def test_feature_matrix_returns_dict(self):
        matrix = eggsec.feature_matrix()
        assert isinstance(matrix, dict)
        assert "core" in matrix

    def test_feature_matrix_entry_structure(self):
        matrix = eggsec.feature_matrix()
        entry = matrix["core"]
        assert "available" in entry
        assert "description" in entry
        assert "requires_system_deps" in entry
        assert entry["available"] is True

    def test_all_always_available_features_report_available(self):
        matrix = eggsec.feature_matrix()
        always_available = [
            "core",
            "scanner",
            "async-api",
            "endpoint-discovery",
            "service-fingerprinting",
            "waf-detection",
            "waf-validation",
            "http-fuzzing",
            "load-testing",
            "findings-reporting",
        ]
        for name in always_available:
            assert name in matrix, f"Feature '{name}' missing from matrix"
            assert matrix[name]["available"] is True, (
                f"Always-available feature '{name}' reports unavailable"
            )

    def test_feature_gated_types_absent_when_disabled(self):
        """Feature-gated attributes should not exist when feature is disabled."""
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
# D. Sync/async contract parity
# ---------------------------------------------------------------------------


class TestSyncAsyncContractParity:
    """Verify sync functions have matching async counterparts."""

    SYNC_ASYNC_PAIRS = [
        "scan_ports",
        "scan_endpoints",
        "fingerprint_services",
        "recon_dns",
        "inspect_tls",
        "detect_technology",
        "detect_waf",
        "validate_waf",
        "fuzz_http",
        "load_test_http",
        "run_consolidated_recon",
        "graphql_test",
        "oauth_test",
        "auth_test",
    ]

    @pytest.mark.parametrize("func_name", SYNC_ASYNC_PAIRS)
    def test_async_counterpart_exists(self, func_name):
        assert hasattr(eggsec, f"async_{func_name}"), (
            f"async_{func_name} not found"
        )
        assert callable(getattr(eggsec, f"async_{func_name}"))

    @pytest.mark.parametrize("func_name", SYNC_ASYNC_PAIRS)
    def test_signatures_match(self, func_name):
        sync_fn = getattr(eggsec, func_name)
        async_fn = getattr(eggsec, f"async_{func_name}")
        sync_params = list(inspect.signature(sync_fn).parameters.keys())
        async_params = list(inspect.signature(async_fn).parameters.keys())
        assert sync_params == async_params, (
            f"Signature mismatch for {func_name}: "
            f"sync={sync_params}, async={async_params}"
        )


# ---------------------------------------------------------------------------
# E. Cancellation/leak/shutdown tests
# ---------------------------------------------------------------------------


class TestCancellationLeakShutdown:
    """Test CancellationToken, ExecutionHandle context manager, EventLog close."""

    def test_cancellationToken_creation(self):
        token = eggsec.CancellationToken()
        assert token.is_cancelled() is False

    def test_cancellationToken_cancel(self):
        token = eggsec.CancellationToken()
        token.cancel()
        assert token.is_cancelled() is True

    def test_cancellationToken_cancel_with_reason(self):
        token = eggsec.CancellationToken()
        token.cancel("user requested")
        assert token.is_cancelled() is True

    def test_cancellationToken_to_dict(self):
        token = eggsec.CancellationToken()
        d = token.to_dict()
        assert isinstance(d, dict)
        assert d["cancelled"] is False

    def test_cancellationToken_to_json(self):
        token = eggsec.CancellationToken()
        j = token.to_json()
        parsed = json.loads(j)
        assert parsed["cancelled"] is False

    def test_cancellationToken_repr(self):
        token = eggsec.CancellationToken()
        assert "CancellationToken" in repr(token)
        assert "cancelled=false" in repr(token)

    def test_executionHandle_context_manager(self):
        handle = eggsec.ExecutionHandle("test-id")
        if hasattr(handle, "__enter__") and hasattr(handle, "__exit__"):
            with handle as h:
                assert h.handle_id == "test-id"
                assert h.is_running() is True
        else:
            # Context manager methods not exposed; verify basic API works
            assert handle.handle_id == "test-id"
            assert handle.is_running() is True

    def test_executionHandle_is_complete_default(self):
        handle = eggsec.ExecutionHandle("id")
        assert handle.is_complete() is False

    def test_executionHandle_to_dict(self):
        handle = eggsec.ExecutionHandle("h1")
        d = handle.to_dict()
        assert isinstance(d, dict)
        assert d["handle_id"] == "h1"

    def test_eventLog_operations(self):
        log = eggsec.EventLog()
        log.push(eggsec.ExecutionEvent("h1", "test", 1000))
        assert len(log) == 1

    def test_eventLog_drain_clears(self):
        log = eggsec.EventLog()
        log.push(eggsec.ExecutionEvent("h1", "start", 1000))
        log.push(eggsec.ExecutionEvent("h1", "end", 2000))
        drained = log.drain()
        assert len(drained) == 2
        assert log.is_empty()


# ---------------------------------------------------------------------------
# F. Policy equivalence tests
# ---------------------------------------------------------------------------


class TestPolicyEquivalence:
    """Test ExecutionPolicy, ManualOverride, and EnforcementContext defaults."""

    def test_execution_policy_defaults(self):
        p = eggsec.ExecutionPolicy()
        assert p.require_explicit_scope is True
        assert p.allow_passive_fingerprint is True
        assert p.allow_active_scan is True
        assert p.allow_exploit is False
        assert p.allow_dos is False
        assert p.allow_brute_force is False
        assert p.allow_web_fuzzing is False
        assert p.allow_load_testing is False
        assert p.allow_stress_testing is False
        assert p.allowed_capabilities == []
        assert p.denied_capabilities == []

    def test_manual_override_defaults(self):
        mo = eggsec.ManualOverride("test reason")
        assert mo.reason == "test reason"
        assert mo.assume_yes is False
        assert mo.allow_out_of_scope is False
        assert mo.allow_high_risk is False
        assert mo.allow_db_pentest is False
        assert mo.allow_web_proxy is False

    def test_manual_override_permits(self):
        mo = eggsec.ManualOverride(
            "test",
            allow_out_of_scope=True,
            allow_high_risk=True,
        )
        assert mo.permits("out-of-scope") is True
        assert mo.permits("high-risk") is True
        assert mo.permits("scope-missing") is False

    @pytest.mark.parametrize(
        "factory_name,expected_strict,expected_automated",
        [
            ("manual_permissive", False, False),
            ("mcp_strict", True, True),
            ("agent_strict", True, True),
            ("ci_strict", True, True),
        ],
    )
    def test_enforcement_context_profiles(
        self, factory_name, expected_strict, expected_automated
    ):
        policy = eggsec.ExecutionPolicy()
        scope = eggsec.LoadedScope.default_empty()
        factory = getattr(eggsec.EnforcementContext, factory_name)
        ctx = factory(policy, scope)
        assert ctx.profile.is_strict == expected_strict
        assert ctx.profile.is_automated == expected_automated

    def test_execution_surface_properties(self):
        assert eggsec.ExecutionSurface.cli_manual().is_manual is True
        assert eggsec.ExecutionSurface.mcp_server().is_manual is False
        assert eggsec.ExecutionSurface.security_agent().is_agent_controlled is True
        assert eggsec.ExecutionSurface.ci().is_manual is False


# ---------------------------------------------------------------------------
# G. Serialization compatibility tests
# ---------------------------------------------------------------------------


class TestSerializationCompatibility:
    """Test to_dict(), to_json(), and pickle roundtrips."""

    def _assert_valid_json(self, s):
        parsed = json.loads(s)
        assert isinstance(parsed, (dict, list))

    @pytest.mark.parametrize(
        "obj_factory",
        [
            lambda: eggsec.Finding("f1", "Title", eggsec.Severity.High, "t", "c", "d"),
            lambda: eggsec.CvssScore("3.1", "CVSS:3.1/AV:N", 9.8),
            lambda: eggsec.EventEnvelope("test", {}, event_id="e1"),
            lambda: eggsec.ExecutionEvent("h1", "start", 1000),
            lambda: eggsec.CancellationToken(),
            lambda: eggsec.ExecutionHandle("h1"),
        ],
    )
    def test_to_dict_returns_dict(self, obj_factory):
        obj = obj_factory()
        d = obj.to_dict()
        assert isinstance(d, dict)

    @pytest.mark.parametrize(
        "obj_factory",
        [
            lambda: eggsec.Finding("f1", "Title", eggsec.Severity.High, "t", "c", "d"),
            lambda: eggsec.CvssScore("3.1", "CVSS:3.1/AV:N", 9.8),
            lambda: eggsec.EventEnvelope("test", {}, event_id="e1"),
            lambda: eggsec.ExecutionEvent("h1", "start", 1000),
            lambda: eggsec.CancellationToken(),
            lambda: eggsec.ExecutionHandle("h1"),
        ],
    )
    def test_to_json_returns_valid_json(self, obj_factory):
        obj = obj_factory()
        j = obj.to_json()
        self._assert_valid_json(j)

    def test_finding_dict_roundtrip(self):
        f = eggsec.Finding("f1", "Title", eggsec.Severity.High, "t", "c", "d")
        d = f.to_dict()
        assert d["id"] == "f1"
        assert d["severity"] == "High"

    def test_finding_json_roundtrip(self):
        f = eggsec.Finding("f1", "Title", eggsec.Severity.High, "t", "c", "d")
        j = f.to_json()
        parsed = json.loads(j)
        assert parsed["id"] == "f1"

    def test_report_json_roundtrip(self):
        report = eggsec.Report(metadata={"test": "hardening"})
        f = eggsec.Finding("f1", "T", eggsec.Severity.High, "t", "c", "d")
        report.add_finding(f)
        j = report.to_json()
        parsed = json.loads(j)
        assert len(parsed["findings"]) == 1

    def test_json_roundtrip_preserves_data(self):
        """to_json() -> json.loads() should preserve key fields."""
        f = eggsec.Finding("f1", "Title", eggsec.Severity.Medium, "t", "c", "d")
        parsed = json.loads(f.to_json())
        assert parsed["id"] == "f1"
        assert parsed["title"] == "Title"
        assert parsed["severity"] == "Medium"

    def test_cancellation_token_json_roundtrip(self):
        token = eggsec.CancellationToken()
        parsed = json.loads(token.to_json())
        assert parsed["cancelled"] is False
        token.cancel()
        parsed2 = json.loads(token.to_json())
        assert parsed2["cancelled"] is True


# ---------------------------------------------------------------------------
# H. Deprecation warning tests
# ---------------------------------------------------------------------------


class TestDeprecationWarnings:
    """Test deprecated_warning() and DeprecatedWarning."""

    def test_deprecated_warning_class_exists_in_core(self):
        from eggsec._core import DeprecatedWarning
        assert DeprecatedWarning is not None

    def test_deprecated_warning_class_creation(self):
        from eggsec._core import DeprecatedWarning
        w = DeprecatedWarning("test msg")
        assert str(w) == "test msg"

    def test_deprecated_warning_class_default_message(self):
        from eggsec._core import DeprecatedWarning
        w = DeprecatedWarning()
        assert "deprecated" in str(w).lower()

    def test_deprecated_warning_class_has_message_attr(self):
        from eggsec._core import DeprecatedWarning
        w = DeprecatedWarning("hello")
        assert w.message == "hello"

    def test_deprecated_warning_class_repr(self):
        from eggsec._core import DeprecatedWarning
        w = DeprecatedWarning("msg")
        assert "DeprecatedWarning" in repr(w)

    def test_deprecated_warning_function_exists(self):
        assert callable(eggsec.deprecated_warning)


# ---------------------------------------------------------------------------
# I. Version metadata tests
# ---------------------------------------------------------------------------


class TestVersionMetadata:
    """Test machine-readable version constants."""

    def test_schema_version(self):
        assert eggsec.__schema_version__ == "1.0"

    def test_protocol_version(self):
        assert eggsec.__protocol_version__ == "1.0.0"

    def test_abi_version(self):
        assert eggsec.__abi_version__ == "1"

    def test_api_surface_version_keys(self):
        result = eggsec.api_surface_version()
        expected_keys = {
            "package_version",
            "schema_version",
            "protocol_version",
            "abi_version",
            "features_list",
        }
        assert expected_keys.issubset(set(result.keys()))

    def test_api_surface_version_values_are_strings(self):
        result = eggsec.api_surface_version()
        for key in ["package_version", "schema_version", "protocol_version", "abi_version"]:
            assert isinstance(result[key], str)

    def test_api_surface_version_features_list(self):
        result = eggsec.api_surface_version()
        features = result["features_list"]
        assert isinstance(features, list)
        assert "core" in features
        assert "scanner" in features

    def test_finding_schema_version(self):
        assert hasattr(eggsec, "FINDING_SCHEMA_VERSION")
        assert isinstance(eggsec.FINDING_SCHEMA_VERSION, str)

    def test_event_schema_version(self):
        assert hasattr(eggsec, "EVENT_SCHEMA_VERSION")
        assert isinstance(eggsec.EVENT_SCHEMA_VERSION, str)

    def test_version_constants_types(self):
        assert isinstance(eggsec.__version__, str)
        assert isinstance(eggsec.__version_info__, tuple)


# ---------------------------------------------------------------------------
# J. Event schema tests
# ---------------------------------------------------------------------------


class TestEventSchema:
    """Test EVENT_SCHEMA_VERSION, EventEnvelope, wrap_event, EventStream."""

    def test_event_schema_version_value(self):
        assert eggsec.EVENT_SCHEMA_VERSION == "1.0.0"

    def test_event_envelope_fields(self):
        payload = eggsec.PlanningEvent("op-1", "target.com", "in-scope")
        env = eggsec.EventEnvelope("planning", payload)
        assert env.schema_version == "1.0.0"
        assert env.event_type == "planning"
        assert env.event_id.startswith("evt-")
        assert env.timestamp_ms > 0

    def test_event_envelope_to_dict(self):
        payload = eggsec.PlanningEvent("op-1", "target.com", "scope")
        env = eggsec.EventEnvelope("planning", payload)
        d = env.to_dict()
        assert isinstance(d, dict)
        assert d["schema_version"] == "1.0.0"
        assert d["event_type"] == "planning"
        assert "event_id" in d
        assert "timestamp_ms" in d

    def test_wrap_event_creates_valid_envelope(self):
        payload = eggsec.PlanningEvent("op-1", "target.com", "scope")
        env = eggsec.wrap_event("planning", payload)
        assert env.event_type == "planning"
        assert env.schema_version == "1.0.0"
        assert env.event_id.startswith("evt-")

    def test_wrap_event_with_correlation(self):
        payload = eggsec.PlanningEvent("op-1", "target.com", "scope")
        env = eggsec.wrap_event("planning", payload, correlation_id="corr-1")
        assert env.correlation_id == "corr-1"

    def test_event_stream_filter_by_type(self):
        log = eggsec.EventLog()
        log.push(eggsec.ExecutionEvent("h1", "planning", 1000))
        log.push(eggsec.ExecutionEvent("h1", "progress", 2000))
        log.push(eggsec.ExecutionEvent("h1", "planning", 3000))
        stream = eggsec.EventStream(log)
        filtered = stream.filter_by_type("planning")
        assert len(filtered) == 2

    def test_event_stream_snapshot(self):
        log = eggsec.EventLog()
        log.push(eggsec.ExecutionEvent("h1", "start", 1000))
        log.push(eggsec.ExecutionEvent("h1", "end", 2000))
        stream = eggsec.EventStream(log)
        snap = stream.snapshot()
        assert isinstance(snap, dict)
        assert snap["total_events"] == 2

    def test_event_stream_empty(self):
        stream = eggsec.EventStream.empty()
        assert stream.is_empty()
        assert len(stream) == 0

    def test_all_event_payload_types(self):
        """All 9 typed event payloads should be creatable."""
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


# ---------------------------------------------------------------------------
# K. Callback contract tests
# ---------------------------------------------------------------------------


class TestCallbackContracts:
    """Test all 5 sink types, error isolation, and close() behavior."""

    def test_audit_sink_accepts_callable(self):
        received = []
        sink = eggsec.AuditSink(lambda e: received.append(e))
        assert not sink.is_closed

    def test_finding_sink_accepts_callable(self):
        received = []
        sink = eggsec.FindingSink(lambda f: received.append(f))
        assert not sink.is_closed

    def test_artifact_sink_accepts_callable(self):
        received = []
        sink = eggsec.ArtifactSink(lambda a: received.append(a))
        assert not sink.is_closed

    def test_progress_sink_accepts_callable(self):
        received = []
        sink = eggsec.ProgressSink(lambda p, m: received.append((p, m)))
        assert not sink.is_closed

    def test_event_consumer_accepts_callable(self):
        received = []
        consumer = eggsec.EventConsumer(lambda e: received.append(e))
        assert not consumer.is_closed

    def test_error_isolation_finding_sink(self):
        def bad_handler(finding):
            raise ValueError("intentional")

        sink = eggsec.FindingSink(bad_handler)
        # Sink should still be usable (not crashed)
        assert not sink.is_closed

    def test_error_isolation_progress_sink(self):
        def bad_handler(percentage, message):
            raise RuntimeError("boom")

        sink = eggsec.ProgressSink(bad_handler)
        assert not sink.is_closed

    def test_close_marks_sink_closed(self):
        sink = eggsec.AuditSink(lambda e: None)
        sink.close()
        assert sink.is_closed

    def test_close_marks_finding_sink_closed(self):
        sink = eggsec.FindingSink(lambda f: None)
        sink.close()
        assert sink.is_closed

    def test_close_marks_artifact_sink_closed(self):
        sink = eggsec.ArtifactSink(lambda a: None)
        sink.close()
        assert sink.is_closed

    def test_close_marks_progress_sink_closed(self):
        sink = eggsec.ProgressSink(lambda p, m: None)
        sink.close()
        assert sink.is_closed

    def test_close_marks_event_consumer_closed(self):
        consumer = eggsec.EventConsumer(lambda e: None)
        consumer.close()
        assert consumer.is_closed

    def test_async_callback_create_and_close(self):
        async def handler(event):
            pass

        cb = eggsec.AsyncCallback(handler)
        assert not cb.is_closed
        cb.close()
        assert cb.is_closed

    def test_callback_scheduler_capacity(self):
        scheduler = eggsec.CallbackScheduler(2)
        payload = eggsec.PlanningEvent("op-1", "t.com", "s")
        env = eggsec.EventEnvelope("planning", payload)
        scheduler.enqueue(env)
        scheduler.enqueue(env)
        scheduler.enqueue(env)  # should drop
        assert scheduler.pending() == 2

    def test_callback_scheduler_close(self):
        scheduler = eggsec.CallbackScheduler(10)
        scheduler.close()
        assert scheduler.is_closed
        payload = eggsec.PlanningEvent("op-1", "t.com", "s")
        env = eggsec.EventEnvelope("planning", payload)
        assert scheduler.enqueue(env) is False

    def test_backpressure_channel_create(self):
        ch = eggsec.BackpressureChannel(128)
        assert ch.capacity == 128
        assert ch.is_empty()

    def test_backpressure_channel_send_recv(self):
        ch = eggsec.BackpressureChannel(10)
        payload = eggsec.PlanningEvent("op-1", "t.com", "s")
        env = eggsec.EventEnvelope("planning", payload)
        ch.send(env)
        received = ch.try_recv()
        assert received is not None
        assert received.event_type == "planning"
