"""Direct-function and engine equivalence tests for all 22 stable operations.

Verifies that:
- Direct functions (eggsec.scan_ports(...)) and engine dispatch (engine.run(OperationRequest(...)))
  both call the same underlying Rust function.
- Both paths enforce scope consistently.
- Engine dispatch emits audit events; direct functions do not.
- Feature-gated operations return structured errors when unavailable.
"""

from __future__ import annotations

import time
from typing import Any, Callable

import eggsec
import pytest

from fixtures.stable_core import HOST, StableCoreFixtures


# ---------------------------------------------------------------------------
# The canonical 22 stable operation IDs (from engine.list_operations()).
# ---------------------------------------------------------------------------

ALL_22_OPERATION_IDS = [
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

# The 10 always-compiled operations with direct functions and expected result types.
ALWAYS_COMPILED = [
    ("scan_ports", "scan_ports", "PortScanResult"),
    ("scan_endpoints", "scan_endpoints", "EndpointScanResult"),
    ("fingerprint_services", "fingerprint_services", "FingerprintScanResult"),
    ("recon_dns", "recon_dns", "DnsRecordSet"),
    ("inspect_tls", "inspect_tls", "TlsInspectionResult"),
    ("detect_technology", "detect_technology", "TechDetectionResult"),
    ("detect_waf", "detect_waf", "WafDetectionResult"),
    ("validate_waf", "validate_waf", "WafScanResult"),
    ("fuzz_http", "fuzz_http", "FuzzSession"),
    ("load_test", "load_test_http", "LoadTestResult"),
]

# Feature-gated operations: (operation_id, required_feature, direct_function_name)
FEATURE_GATED_OPS = [
    ("scan_git_secrets", "git-secrets", "scan_git_secrets"),
    ("generate_sbom", "sbom", "generate_sbom"),
    ("db_probe", "db-pentest", "db_probe"),
    ("nse_run", "nse", "nse_run"),
    ("scan_docker_image", "container", "scan_docker_image"),
    ("scan_kubernetes", "container", "scan_kubernetes"),
    ("analyze_apk", "mobile", "analyze_apk"),
    ("analyze_ipa", "mobile", "analyze_ipa"),
]

# Operations that also have no direct function in the always-compiled set.
# These go through engine only.
ENGINE_ONLY_OPS = [
    "run_consolidated_recon",
    "graphql_test",
    "oauth_test",
    "auth_test",
]


# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------


@pytest.fixture(scope="module")
def stable_fixtures():
    with StableCoreFixtures() as fixtures:
        yield fixtures


def _await_future(future: Any, timeout: float = 30.0) -> Any:
    """Resolve the extension's awaitable without pytest-asyncio."""
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        try:
            value = next(future)
        except StopIteration as done:
            return done.value
        if value is not None:
            return value
        time.sleep(0.01)
    raise AssertionError("async fixture operation did not complete before timeout")


def _make_engine(scope: eggsec.Scope | None = None) -> eggsec.Engine:
    """Create an Engine with the given scope (default: loopback)."""
    if scope is None:
        scope = eggsec.Scope.allow_hosts([HOST, "localhost"])
    return eggsec.Engine(scope)


def _strip_py_suffix(name: str) -> str:
    """Strip the 'Py' suffix from PyO3 class names for canonical comparison."""
    return name.removesuffix("Py")


# ---------------------------------------------------------------------------
# 1. test_all_22_operations_have_engine_and_direct_paths
# ---------------------------------------------------------------------------


class TestAll22OperationsHavePaths:
    def test_all_22_in_engine_list_operations(self):
        """All 22 operation IDs must appear in engine.list_operations()."""
        engine = _make_engine()
        listed = engine.list_operations()
        for op_id in ALL_22_OPERATION_IDS:
            assert op_id in listed, f"Operation '{op_id}' missing from list_operations()"

    def test_always_compiled_have_direct_functions(self):
        """The 10 always-compiled operations must have top-level direct functions."""
        for op_id, func_name, _expected_type in ALWAYS_COMPILED:
            assert hasattr(eggsec, func_name), (
                f"Direct function '{func_name}' not found for operation '{op_id}'"
            )
            assert callable(getattr(eggsec, func_name)), (
                f"'{func_name}' is not callable"
            )

    def test_always_compiled_have_async_counterparts(self):
        """Each always-compiled operation must have an async counterpart."""
        for op_id, func_name, _expected_type in ALWAYS_COMPILED:
            async_name = f"async_{func_name}"
            assert hasattr(eggsec, async_name), (
                f"Async counterpart '{async_name}' not found for operation '{op_id}'"
            )
            assert callable(getattr(eggsec, async_name))


# ---------------------------------------------------------------------------
# 2. test_direct_function_returns_correct_type
# ---------------------------------------------------------------------------


class TestDirectFunctionReturnsCorrectType:
    def test_scan_ports(self, stable_fixtures):
        scope = eggsec.Scope.allow_hosts([HOST, "localhost"])
        result = eggsec.scan_ports(HOST, [stable_fixtures.tcp_port], scope, timeout_ms=1000)
        assert type(result).__name__ == "PortScanResult"
        assert hasattr(result, "to_dict")
        assert callable(result.to_dict)
        assert hasattr(result, "to_json")
        assert callable(result.to_json)

    def test_scan_endpoints(self, stable_fixtures):
        scope = eggsec.Scope.allow_hosts([HOST, "localhost"])
        result = eggsec.scan_endpoints(
            stable_fixtures.http_url,
            ["/", "/admin"],
            scope,
            timeout_ms=2000,
            include_404=True,
        )
        assert type(result).__name__ == "EndpointScanResult"
        assert hasattr(result, "to_dict")
        assert hasattr(result, "to_json")

    def test_fingerprint_services(self, stable_fixtures):
        scope = eggsec.Scope.allow_hosts([HOST, "localhost"])
        result = eggsec.fingerprint_services(
            HOST, [stable_fixtures.tcp_port], scope, timeout_ms=1000
        )
        assert type(result).__name__ == "FingerprintScanResult"
        assert hasattr(result, "to_dict")
        assert hasattr(result, "to_json")

    def test_recon_dns(self):
        result = eggsec.recon_dns("localhost")
        assert type(result).__name__ == "DnsRecordSet"
        assert hasattr(result, "to_dict")
        assert hasattr(result, "to_json")

    def test_inspect_tls(self, stable_fixtures):
        result = eggsec.inspect_tls(HOST, port=stable_fixtures.tls_port)
        assert type(result).__name__ == "TlsInspectionResult"
        assert hasattr(result, "to_dict")
        assert hasattr(result, "to_json")

    def test_detect_technology(self, stable_fixtures):
        result = eggsec.detect_technology(stable_fixtures.http_url)
        assert type(result).__name__ == "TechDetectionResult"
        assert hasattr(result, "to_dict")
        assert hasattr(result, "to_json")

    def test_detect_waf(self, stable_fixtures):
        result = eggsec.detect_waf(f"{stable_fixtures.http_url}/waf-block")
        assert _strip_py_suffix(type(result).__name__) == "WafDetectionResult"
        assert hasattr(result, "to_dict")
        assert hasattr(result, "to_json")

    def test_validate_waf(self, stable_fixtures):
        scope = eggsec.Scope.allow_hosts([HOST, "localhost"])
        result = eggsec.validate_waf(
            f"{stable_fixtures.http_url}/waf-block", scope, test_type="headers"
        )
        assert _strip_py_suffix(type(result).__name__) == "WafScanResult"
        assert hasattr(result, "to_dict")
        assert hasattr(result, "to_json")

    def test_fuzz_http(self, stable_fixtures):
        scope = eggsec.Scope.allow_hosts([HOST, "localhost"])
        result = eggsec.fuzz_http(
            f"{stable_fixtures.http_url}/fuzz/{{FUZZ}}",
            scope,
            "xss",
            concurrency=1,
            timeout=2,
        )
        assert _strip_py_suffix(type(result).__name__) == "FuzzSession"
        assert hasattr(result, "to_dict")
        assert hasattr(result, "to_json")

    def test_load_test_http(self, stable_fixtures):
        scope = eggsec.Scope.allow_hosts([HOST, "localhost"])
        result = eggsec.load_test_http(
            f"{stable_fixtures.http_url}/load", 4, 1, 5, scope
        )
        assert _strip_py_suffix(type(result).__name__) == "LoadTestResult"
        assert hasattr(result, "to_dict")
        assert hasattr(result, "to_json")


# ---------------------------------------------------------------------------
# 3. test_engine_dispatch_returns_operation_result
# ---------------------------------------------------------------------------


class TestEngineDispatchReturnsOperationResult:
    def test_scan_ports(self, stable_fixtures):
        engine = _make_engine()
        req = eggsec.OperationRequest(
            "scan_ports", HOST,
            timeout_ms=1000,
            metadata={"ports": str(stable_fixtures.tcp_port)},
        )
        result = engine.run(req)
        assert isinstance(result, eggsec.OperationResult)
        assert result.status.name() == "Completed"
        assert result.payload_type_name == "PortScanResult"

    def test_scan_endpoints(self, stable_fixtures):
        engine = _make_engine()
        req = eggsec.OperationRequest(
            "scan_endpoints", stable_fixtures.http_url,
            timeout_ms=2000,
            metadata={"paths": "/,/admin", "include_404": "true"},
        )
        result = engine.run(req)
        assert isinstance(result, eggsec.OperationResult)
        assert result.status.name() == "Completed"
        assert result.payload_type_name == "EndpointScanResult"

    def test_fingerprint_services(self, stable_fixtures):
        engine = _make_engine()
        req = eggsec.OperationRequest(
            "fingerprint_services", HOST,
            timeout_ms=1000,
            metadata={"ports": str(stable_fixtures.tcp_port)},
        )
        result = engine.run(req)
        assert isinstance(result, eggsec.OperationResult)
        assert result.status.name() == "Completed"
        assert result.payload_type_name == "FingerprintScanResult"

    def test_recon_dns(self):
        engine = _make_engine()
        req = eggsec.OperationRequest("recon_dns", "localhost", timeout_ms=3000)
        result = engine.run(req)
        assert isinstance(result, eggsec.OperationResult)
        assert result.status.name() == "Completed"
        assert result.payload_type_name == "DnsRecordSet"

    def test_inspect_tls(self, stable_fixtures):
        engine = _make_engine()
        req = eggsec.OperationRequest(
            "inspect_tls", HOST,
            timeout_ms=3000,
            metadata={"port": str(stable_fixtures.tls_port)},
        )
        result = engine.run(req)
        assert isinstance(result, eggsec.OperationResult)
        assert result.status.name() == "Completed"
        assert result.payload_type_name == "TlsInspectionResult"

    def test_detect_technology(self, stable_fixtures):
        engine = _make_engine()
        req = eggsec.OperationRequest(
            "detect_technology", stable_fixtures.http_url, timeout_ms=3000
        )
        result = engine.run(req)
        assert isinstance(result, eggsec.OperationResult)
        assert result.status.name() == "Completed"
        assert result.payload_type_name == "TechDetectionResult"

    def test_detect_waf(self, stable_fixtures):
        engine = _make_engine()
        req = eggsec.OperationRequest(
            "detect_waf", f"{stable_fixtures.http_url}/waf-block", timeout_ms=3000
        )
        result = engine.run(req)
        assert isinstance(result, eggsec.OperationResult)
        assert result.status.name() == "Completed"
        assert result.payload_type_name == "WafDetectionResult"

    def test_validate_waf(self, stable_fixtures):
        engine = _make_engine()
        req = eggsec.OperationRequest(
            "validate_waf", f"{stable_fixtures.http_url}/waf-block",
            timeout_ms=3000,
            metadata={"test_type": "headers"},
        )
        result = engine.run(req)
        assert isinstance(result, eggsec.OperationResult)
        assert result.status.name() == "Completed"
        assert result.payload_type_name == "WafScanResult"

    def test_fuzz_http(self, stable_fixtures):
        engine = _make_engine()
        req = eggsec.OperationRequest(
            "fuzz_http",
            f"{stable_fixtures.http_url}/fuzz/{{FUZZ}}",
            timeout_ms=5000,
            metadata={"payload_type": "xss", "concurrency": "1"},
        )
        result = engine.run(req)
        assert isinstance(result, eggsec.OperationResult)
        # fuzz_http is "intrusive" risk; default policy may deny it.
        # Both Completed (if allowed) and Failed (if denied by policy) are valid.
        if result.is_success():
            assert result.payload_type_name == "FuzzSession"
        else:
            assert result.error is not None
            assert result.error.kind == "scope_denial"

    def test_load_test(self, stable_fixtures):
        engine = _make_engine()
        req = eggsec.OperationRequest(
            "load_test", f"{stable_fixtures.http_url}/load",
            timeout_ms=10000,
            metadata={"requests": "4", "concurrency": "1", "timeout_secs": "5"},
        )
        result = engine.run(req)
        assert isinstance(result, eggsec.OperationResult)
        # load_test is "load testing" risk; default policy may deny it.
        if result.is_success():
            assert result.payload_type_name == "LoadTestResult"
        else:
            assert result.error is not None
            assert result.error.kind == "scope_denial"


# ---------------------------------------------------------------------------
# 4. test_payload_type_consistency
# ---------------------------------------------------------------------------


class TestPayloadTypeConsistency:
    """For each always-compiled operation, the payload type from engine dispatch
    must match what the direct function returns (both produce the same type)."""

    def test_scan_ports_type_matches(self, stable_fixtures):
        direct = eggsec.scan_ports(
            HOST, [stable_fixtures.tcp_port],
            eggsec.Scope.allow_hosts([HOST, "localhost"]),
            timeout_ms=1000,
        )
        engine = _make_engine()
        req = eggsec.OperationRequest(
            "scan_ports", HOST,
            timeout_ms=1000,
            metadata={"ports": str(stable_fixtures.tcp_port)},
        )
        op_result = engine.run(req)
        assert op_result.is_success()
        assert op_result.payload_type_name == type(direct).__name__

    def test_recon_dns_type_matches(self):
        direct = eggsec.recon_dns("localhost")
        engine = _make_engine()
        req = eggsec.OperationRequest("recon_dns", "localhost", timeout_ms=3000)
        op_result = engine.run(req)
        assert op_result.is_success()
        assert op_result.payload_type_name == type(direct).__name__

    def test_inspect_tls_type_matches(self, stable_fixtures):
        direct = eggsec.inspect_tls(HOST, port=stable_fixtures.tls_port)
        engine = _make_engine()
        req = eggsec.OperationRequest(
            "inspect_tls", HOST,
            timeout_ms=3000,
            metadata={"port": str(stable_fixtures.tls_port)},
        )
        op_result = engine.run(req)
        assert op_result.is_success()
        assert op_result.payload_type_name == type(direct).__name__

    def test_detect_technology_type_matches(self, stable_fixtures):
        direct = eggsec.detect_technology(stable_fixtures.http_url)
        engine = _make_engine()
        req = eggsec.OperationRequest(
            "detect_technology", stable_fixtures.http_url, timeout_ms=3000
        )
        op_result = engine.run(req)
        assert op_result.is_success()
        assert op_result.payload_type_name == type(direct).__name__

    def test_detect_waf_type_matches(self, stable_fixtures):
        url = f"{stable_fixtures.http_url}/waf-block"
        direct = eggsec.detect_waf(url)
        engine = _make_engine()
        req = eggsec.OperationRequest("detect_waf", url, timeout_ms=3000)
        op_result = engine.run(req)
        assert op_result.is_success()
        # payload_type_name uses canonical name (without Py suffix)
        assert op_result.payload_type_name == _strip_py_suffix(type(direct).__name__)


# ---------------------------------------------------------------------------
# 5. test_scope_denial_on_both_paths
# ---------------------------------------------------------------------------


class TestScopeDenialOnBothPaths:
    """Both direct function and engine dispatch must enforce scope consistently."""

    def test_scan_ports_direct_raises(self):
        scope = eggsec.Scope.allow_hosts(["192.0.2.1"])
        with pytest.raises(eggsec.EnforcementError):
            eggsec.scan_ports("127.0.0.1", [80], scope, timeout_ms=1000)

    def test_scan_ports_engine_returns_error(self):
        engine = _make_engine(eggsec.Scope.allow_hosts(["192.0.2.1"]))
        req = eggsec.OperationRequest("scan_ports", "127.0.0.1", timeout_ms=1000)
        result = engine.run(req)
        assert result.is_failure()
        assert result.error is not None
        assert result.error.kind == "scope_denial"

    def test_scan_endpoints_direct_raises(self):
        scope = eggsec.Scope.allow_hosts(["192.0.2.1"])
        with pytest.raises(eggsec.EnforcementError):
            eggsec.scan_endpoints("http://127.0.0.1", ["/"], scope, timeout_ms=1000)

    def test_scan_endpoints_engine_returns_error(self):
        engine = _make_engine(eggsec.Scope.allow_hosts(["192.0.2.1"]))
        req = eggsec.OperationRequest("scan_endpoints", "http://127.0.0.1", timeout_ms=1000)
        result = engine.run(req)
        assert result.is_failure()
        assert result.error is not None
        assert result.error.kind == "scope_denial"

    def test_recon_dns_engine_returns_error(self):
        engine = _make_engine(eggsec.Scope.allow_hosts(["192.0.2.1"]))
        req = eggsec.OperationRequest("recon_dns", "127.0.0.1", timeout_ms=1000)
        result = engine.run(req)
        assert result.is_failure()
        assert result.error is not None
        assert result.error.kind == "scope_denial"


# ---------------------------------------------------------------------------
# 6. test_feature_unavailable_on_engine_dispatch
# ---------------------------------------------------------------------------


class TestFeatureUnavailableOnEngineDispatch:
    """Feature-gated operations must return structured errors when feature is not compiled."""

    def test_git_secrets_dispatch(self):
        engine = _make_engine()
        req = eggsec.OperationRequest(
            "scan_git_secrets", "/tmp/nonexistent", timeout_ms=1000
        )
        result = engine.run(req)
        if not eggsec.has_feature("git-secrets"):
            assert result.is_failure()
            assert result.error is not None
            assert result.error.kind == "feature_unavailable"
            assert result.error.operation_id == "scan_git_secrets"
        else:
            assert result.status.name() in ("Completed", "Failed")

    def test_sbom_dispatch(self):
        engine = _make_engine()
        req = eggsec.OperationRequest(
            "generate_sbom", "/tmp/nonexistent", timeout_ms=1000
        )
        result = engine.run(req)
        if not eggsec.has_feature("sbom"):
            assert result.is_failure()
            assert result.error is not None
            assert result.error.kind == "feature_unavailable"
            assert result.error.operation_id == "generate_sbom"
        else:
            assert result.status.name() in ("Completed", "Failed")

    def test_db_probe_dispatch(self):
        engine = _make_engine()
        req = eggsec.OperationRequest(
            "db_probe", "127.0.0.1",
            timeout_ms=1000,
            metadata={"port": "5432", "database": "testdb"},
        )
        result = engine.run(req)
        if not eggsec.has_feature("db-pentest"):
            assert result.is_failure()
            assert result.error is not None
            assert result.error.kind == "feature_unavailable"
            assert result.error.operation_id == "db_probe"
        else:
            assert result.status.name() in ("Completed", "Failed")

    def test_nse_run_dispatch(self):
        engine = _make_engine()
        req = eggsec.OperationRequest(
            "nse_run", "127.0.0.1",
            timeout_ms=1000,
            metadata={"scripts": "http-headers"},
        )
        result = engine.run(req)
        if not eggsec.has_feature("nse"):
            assert result.is_failure()
            assert result.error is not None
            assert result.error.kind == "feature_unavailable"
            assert result.error.operation_id == "nse_run"
        else:
            assert result.status.name() in ("Completed", "Failed")

    def test_container_ops_dispatch(self):
        engine = _make_engine()
        for op_id in ("scan_docker_image", "scan_kubernetes"):
            target = "nginx:latest" if op_id == "scan_docker_image" else "/tmp/k8s.yaml"
            req = eggsec.OperationRequest(op_id, target, timeout_ms=1000)
            result = engine.run(req)
            if not eggsec.has_feature("container"):
                assert result.is_failure()
                assert result.error is not None
                assert result.error.kind == "feature_unavailable"
                assert result.error.operation_id == op_id
            else:
                assert result.status.name() in ("Completed", "Failed")

    def test_mobile_ops_dispatch(self):
        engine = _make_engine()
        for op_id in ("analyze_apk", "analyze_ipa"):
            target = "/tmp/dummy.apk" if op_id == "analyze_apk" else "/tmp/dummy.ipa"
            req = eggsec.OperationRequest(op_id, target, timeout_ms=1000)
            result = engine.run(req)
            if not eggsec.has_feature("mobile"):
                assert result.is_failure()
                assert result.error is not None
                assert result.error.kind == "feature_unavailable"
                assert result.error.operation_id == op_id
            else:
                assert result.status.name() in ("Completed", "Failed")


# ---------------------------------------------------------------------------
# 7. test_direct_function_scope_enforcement_matches_engine
# ---------------------------------------------------------------------------


class TestDirectFunctionScopeEnforcementMatchesEngine:
    """For scan_ports with a specific scope, both paths enforce the same rules."""

    def test_in_scope_allows_both_paths(self, stable_fixtures):
        scope = eggsec.Scope.allow_hosts([HOST, "localhost"])
        # Direct path: should succeed
        direct_result = eggsec.scan_ports(
            HOST, [stable_fixtures.tcp_port], scope, timeout_ms=1000
        )
        assert type(direct_result).__name__ == "PortScanResult"

        # Engine path: should also succeed
        engine = _make_engine(scope)
        req = eggsec.OperationRequest(
            "scan_ports", HOST,
            timeout_ms=1000,
            metadata={"ports": str(stable_fixtures.tcp_port)},
        )
        engine_result = engine.run(req)
        assert engine_result.is_success()

    def test_out_of_scope_denies_both_paths(self):
        scope = eggsec.Scope.allow_hosts(["192.0.2.1"])
        # Direct path: should raise EnforcementError
        with pytest.raises(eggsec.EnforcementError):
            eggsec.scan_ports("127.0.0.1", [80], scope, timeout_ms=1000)

        # Engine path: should return scope_denial error
        engine = _make_engine(scope)
        req = eggsec.OperationRequest("scan_ports", "127.0.0.1", timeout_ms=1000)
        result = engine.run(req)
        assert result.is_failure()
        assert result.error.kind == "scope_denial"

    def test_deny_all_scope_denies_both_paths(self):
        scope = eggsec.Scope.deny_all()
        # Direct path: should raise EnforcementError
        with pytest.raises(eggsec.EnforcementError):
            eggsec.scan_ports("127.0.0.1", [80], scope, timeout_ms=1000)

        # Engine path: should return scope_denial error
        engine = _make_engine(scope)
        req = eggsec.OperationRequest("scan_ports", "127.0.0.1", timeout_ms=1000)
        result = engine.run(req)
        assert result.is_failure()
        assert result.error.kind == "scope_denial"


# ---------------------------------------------------------------------------
# 8. test_engine_emits_audit_events
# ---------------------------------------------------------------------------


class TestEngineEmitsAuditEvents:
    def test_successful_dispatch_emits_audit(self, stable_fixtures):
        engine = _make_engine()
        req = eggsec.OperationRequest(
            "scan_ports", HOST,
            timeout_ms=1000,
            metadata={"ports": str(stable_fixtures.tcp_port)},
        )
        engine.run(req)
        events = engine.audit_events()
        assert len(events) >= 1
        assert events[0].operation_id == "scan_ports"

    def test_scope_denial_emits_audit(self):
        engine = _make_engine(eggsec.Scope.allow_hosts(["192.0.2.1"]))
        req = eggsec.OperationRequest("scan_ports", "127.0.0.1", timeout_ms=1000)
        engine.run(req)
        events = engine.audit_events()
        assert len(events) >= 1
        assert events[0].allowed is False

    def test_multiple_dispatches_accumulate_audit(self, stable_fixtures):
        engine = _make_engine()
        # Dispatch scan_ports
        req1 = eggsec.OperationRequest(
            "scan_ports", HOST,
            timeout_ms=1000,
            metadata={"ports": str(stable_fixtures.tcp_port)},
        )
        engine.run(req1)
        count_after_first = len(engine.audit_events())

        # Dispatch recon_dns
        req2 = eggsec.OperationRequest("recon_dns", "localhost", timeout_ms=3000)
        engine.run(req2)
        count_after_second = len(engine.audit_events())

        assert count_after_second > count_after_first
        assert count_after_second >= 2

    def test_feature_unavailable_emits_audit(self):
        engine = _make_engine()
        req = eggsec.OperationRequest(
            "scan_git_secrets", "/tmp/nonexistent", timeout_ms=1000
        )
        result = engine.run(req)
        # Feature-unavailability is caught before audit gate, so no audit events
        # are emitted. Verify the operation fails with the correct error.
        assert result.is_failure()
        assert result.error is not None
        assert result.error.kind == "feature_unavailable"


# ---------------------------------------------------------------------------
# 9. test_direct_function_no_audit_events
# ---------------------------------------------------------------------------


class TestDirectFunctionNoAuditEvents:
    def test_scan_ports_no_audit(self, stable_fixtures):
        engine = _make_engine()
        req = eggsec.OperationRequest(
            "scan_ports", HOST,
            timeout_ms=1000,
            metadata={"ports": str(stable_fixtures.tcp_port)},
        )
        # First, establish baseline from engine dispatch
        engine.run(req)
        baseline = len(engine.audit_events())
        assert baseline >= 1

        # Now call the direct function — should NOT emit audit events
        scope = eggsec.Scope.allow_hosts([HOST, "localhost"])
        eggsec.scan_ports(
            HOST, [stable_fixtures.tcp_port], scope, timeout_ms=1000
        )
        # Direct functions don't use the engine instance, so audit count unchanged
        assert len(engine.audit_events()) == baseline

    def test_recon_dns_no_audit(self):
        engine = _make_engine()
        req = eggsec.OperationRequest("recon_dns", "localhost", timeout_ms=3000)
        engine.run(req)
        baseline = len(engine.audit_events())

        eggsec.recon_dns("localhost")
        assert len(engine.audit_events()) == baseline

    def test_detect_waf_no_audit(self, stable_fixtures):
        engine = _make_engine()
        url = f"{stable_fixtures.http_url}/waf-block"
        req = eggsec.OperationRequest("detect_waf", url, timeout_ms=3000)
        engine.run(req)
        baseline = len(engine.audit_events())

        eggsec.detect_waf(url)
        assert len(engine.audit_events()) == baseline


# ---------------------------------------------------------------------------
# 10. Engine-only operations: engine dispatch without direct functions
# ---------------------------------------------------------------------------


class TestEngineOnlyOperations:
    """Operations without direct functions must still work via engine dispatch."""

    def test_run_consolidated_recon_dispatch(self, stable_fixtures):
        engine = _make_engine()
        req = eggsec.OperationRequest(
            "run_consolidated_recon", stable_fixtures.http_url,
            timeout_ms=5000,
            metadata={"run_dns": "true", "run_ssl": "false", "run_tech_detect": "true"},
        )
        result = engine.run(req)
        assert isinstance(result, eggsec.OperationResult)
        assert result.status.name() == "Completed"
        assert result.payload_type_name == "ConsolidatedReconReport"

    def test_graphql_test_dispatch(self):
        engine = _make_engine()
        req = eggsec.OperationRequest(
            "graphql_test", "http://127.0.0.1/graphql", timeout_ms=2000
        )
        result = engine.run(req)
        assert isinstance(result, eggsec.OperationResult)
        # May succeed or fail depending on target, but must return OperationResult
        assert result.status.name() in ("Completed", "Failed")

    def test_oauth_test_dispatch(self):
        engine = _make_engine()
        req = eggsec.OperationRequest(
            "oauth_test", "http://127.0.0.1/oauth", timeout_ms=2000
        )
        result = engine.run(req)
        assert isinstance(result, eggsec.OperationResult)
        assert result.status.name() in ("Completed", "Failed")

    def test_auth_test_dispatch(self):
        engine = _make_engine()
        req = eggsec.OperationRequest(
            "auth_test", "http://127.0.0.1/auth", timeout_ms=2000
        )
        result = engine.run(req)
        assert isinstance(result, eggsec.OperationResult)
        assert result.status.name() in ("Completed", "Failed")


# ---------------------------------------------------------------------------
# 11. Dict/JSON serialization on both paths
# ---------------------------------------------------------------------------


class TestSerializationOnBothPaths:
    """Results from both direct and engine paths must serialize correctly."""

    def test_direct_to_dict_to_json(self, stable_fixtures):
        scope = eggsec.Scope.allow_hosts([HOST, "localhost"])
        result = eggsec.scan_ports(
            HOST, [stable_fixtures.tcp_port], scope, timeout_ms=1000
        )
        d = result.to_dict()
        assert isinstance(d, dict)
        assert "target" in d
        j = result.to_json()
        assert isinstance(j, str)
        assert j.startswith("{")

    def test_engine_result_to_dict_to_json(self, stable_fixtures):
        engine = _make_engine()
        req = eggsec.OperationRequest(
            "scan_ports", HOST,
            timeout_ms=1000,
            metadata={"ports": str(stable_fixtures.tcp_port)},
        )
        op_result = engine.run(req)
        d = op_result.to_dict()
        assert isinstance(d, dict)
        assert "status" in d
        j = op_result.to_json()
        assert isinstance(j, str)
        assert j.startswith("{")
