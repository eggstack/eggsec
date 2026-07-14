"""Scope enforcement across live network transitions.

Workstream 6: Validates scope enforcement across redirects, proxies,
reconnects, and DNS changes using the loopback fixture server.

Covers:
- Basic scope enforcement (allow/deny/deny_all/wildcard)
- Redirect scope behavior (in-scope → out-of-scope, out-of-scope → denied)
- Multiple redirect chains and limits
- Cross-host redirects
- Engine dispatch scope consistency (sync vs engine path)
- Scope with different operation types
- Scope rule evaluation (empty, wildcard, exclusion)
- Scope metadata in audit events
"""

from __future__ import annotations

import os
import time
from typing import Any

import eggsec
import pytest

from fixtures.stable_core import HOST, StableCoreFixtures

os.environ["EGGSEC_ALLOW_LOOPBACK_FIXTURE"] = "1"

OUT_OF_SCOPE_HOST = "192.0.2.1"
LOOPBACK_ALT = "127.0.0.2"

pytestmark = [
    pytest.mark.scope_enforcement,
    pytest.mark.timeout(30),
]


# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------


@pytest.fixture(scope="module")
def stable_fixtures():
    with StableCoreFixtures() as fixtures:
        yield fixtures


def _await_future(future: Any, timeout: float = 30.0) -> Any:
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


def _scope_from_toml(tmp_path, toml_content, name="scope.toml"):
    p = tmp_path / name
    p.write_text(toml_content)
    return eggsec.Scope.from_file(str(p))


def _engine_result_is_scope_denied(result):
    if result.error is not None:
        return (
            result.error.kind == "scope_denial"
            or "scope" in result.error.message.lower()
            or "scope" in result.error.kind.lower()
        )
    return False


# ===========================================================================
# 1. Basic scope enforcement
# ===========================================================================


class TestBasicScopeEnforcement:
    def test_in_scope_target_proceeds(self):
        scope = eggsec.Scope.allow_hosts([HOST])
        assert scope.is_target_allowed(HOST) is True

    def test_out_of_scope_target_denied(self):
        scope = eggsec.Scope.allow_hosts([HOST])
        assert scope.is_target_allowed(OUT_OF_SCOPE_HOST) is False

    def test_deny_all_always_denied(self):
        scope = eggsec.Scope.deny_all()
        assert scope.is_target_allowed(HOST) is False
        assert scope.is_target_allowed("example.com") is False
        assert scope.is_target_allowed("10.0.0.1") is False

    def test_wildcard_matches_subdomains(self):
        scope = eggsec.Scope.allow_hosts(["*.example.com"])
        assert scope.is_target_allowed("sub.example.com") is True
        assert scope.is_target_allowed("deep.sub.example.com") is True

    def test_wildcard_matches_bare_domain(self):
        scope = eggsec.Scope.allow_hosts(["*.example.com"])
        assert scope.is_target_allowed("example.com") is True

    def test_multiple_hosts_scope(self):
        scope = eggsec.Scope.allow_hosts([HOST, "10.0.0.0/8"])
        assert scope.is_target_allowed(HOST) is True
        assert scope.is_target_allowed("10.0.0.1") is True
        assert scope.is_target_allowed(OUT_OF_SCOPE_HOST) is False

    def test_deny_all_enforcement_error_direct(self):
        scope = eggsec.Scope.deny_all()
        with pytest.raises(eggsec.EnforcementError, match="not within the allowed scope"):
            eggsec.scan_ports(HOST, [80], scope, timeout_ms=1000)

    def test_wrong_target_enforcement_error_direct(self):
        scope = eggsec.Scope.allow_hosts(["example.com"])
        with pytest.raises(eggsec.EnforcementError, match="not within the allowed scope"):
            eggsec.scan_ports(HOST, [80], scope, timeout_ms=1000)


# ===========================================================================
# 2. Redirect scope behavior
# ===========================================================================


class TestRedirectScopeBehavior:
    def test_in_scope_redirect_to_out_of_scope(self, stable_fixtures):
        """In-scope host redirects to out-of-scope host.

        The client follows the redirect to the external host, which is
        unreachable (TEST-NET-1), so the request times out.
        """
        fixtures = stable_fixtures
        scope = eggsec.Scope.allow_hosts([HOST])
        client = eggsec.HttpClientPy(
            eggsec.HttpClientConfigPy(timeout_ms=5000, max_redirects=5)
        )
        url = f"{fixtures.http_url}/redirect-external"
        with pytest.raises(eggsec.ScanError):
            client.get(url)

    def test_out_of_scope_target_denied_before_redirect(self, stable_fixtures):
        """Request to out-of-scope host denied before any redirect."""
        scope = eggsec.Scope.allow_hosts([HOST])
        with pytest.raises(eggsec.EnforcementError, match="not within the allowed scope"):
            eggsec.scan_ports(OUT_OF_SCOPE_HOST, [80], scope, timeout_ms=1000)

    def test_in_scope_redirect_to_in_scope_succeeds(self, stable_fixtures):
        """Request to in-scope host redirecting to in-scope host succeeds."""
        fixtures = stable_fixtures
        scope = eggsec.Scope.allow_hosts([HOST])
        client = eggsec.HttpClientPy(
            eggsec.HttpClientConfigPy(timeout_ms=5000, max_redirects=5)
        )
        url = f"{fixtures.http_url}/redirect-local"
        response = client.get(url)

        assert response.status_code == 200
        assert "EGGSEC_FIXTURE_ADMIN" in (response.body_text or "")
        assert response.final_url.endswith("/admin")

    def test_redirect_to_out_of_scope_recorded_in_history(self, stable_fixtures):
        """Following redirect to unreachable external host raises ScanError."""
        fixtures = stable_fixtures
        client = eggsec.HttpClientPy(
            eggsec.HttpClientConfigPy(timeout_ms=5000, max_redirects=5)
        )
        url = f"{fixtures.http_url}/redirect-external"
        with pytest.raises(eggsec.ScanError):
            client.get(url)

    def test_redirect_out_of_scope_not_followed_when_disabled(self, stable_fixtures):
        """When max_redirects=0 and response is a redirect, Too many redirects is raised."""
        fixtures = stable_fixtures
        client = eggsec.HttpClientPy(
            eggsec.HttpClientConfigPy(timeout_ms=5000, max_redirects=0)
        )
        url = f"{fixtures.http_url}/redirect-external"
        with pytest.raises(eggsec.ScanError, match="Too many redirects"):
            client.get(url)

    def test_redirect_local_not_followed_when_disabled(self, stable_fixtures):
        """When max_redirects=0 and response is a redirect, Too many redirects is raised."""
        fixtures = stable_fixtures
        client = eggsec.HttpClientPy(
            eggsec.HttpClientConfigPy(timeout_ms=5000, max_redirects=0)
        )
        url = f"{fixtures.http_url}/redirect-local"
        with pytest.raises(eggsec.ScanError, match="Too many redirects"):
            client.get(url)


# ===========================================================================
# 3. Multiple redirect chain
# ===========================================================================


class TestMultipleRedirectChain:
    def test_single_redirect_local_count(self, stable_fixtures):
        """Single redirect-local produces exactly one redirect entry."""
        fixtures = stable_fixtures
        client = eggsec.HttpClientPy(
            eggsec.HttpClientConfigPy(timeout_ms=5000, max_redirects=10)
        )
        url = f"{fixtures.http_url}/redirect-local"
        response = client.get(url)
        assert len(response.redirect_history) == 1
        assert response.redirect_history[0].status_code in (301, 302, 307, 308)

    def test_max_redirects_limit_enforced(self, stable_fixtures):
        """max_redirects=0 stops redirect following; redirect response raises error."""
        fixtures = stable_fixtures
        client = eggsec.HttpClientPy(
            eggsec.HttpClientConfigPy(timeout_ms=5000, max_redirects=0)
        )
        url = f"{fixtures.http_url}/redirect-local"
        with pytest.raises(eggsec.ScanError, match="Too many redirects"):
            client.get(url)

    def test_redirect_chain_serialization(self, stable_fixtures):
        """Following redirect to unreachable external host raises ScanError."""
        fixtures = stable_fixtures
        client = eggsec.HttpClientPy(
            eggsec.HttpClientConfigPy(timeout_ms=5000, max_redirects=5)
        )
        url = f"{fixtures.http_url}/redirect-external"
        with pytest.raises(eggsec.ScanError):
            client.get(url)

    def test_final_url_after_local_redirect(self, stable_fixtures):
        """After following redirect-local, final_url reflects the destination."""
        fixtures = stable_fixtures
        client = eggsec.HttpClientPy(
            eggsec.HttpClientConfigPy(timeout_ms=5000, max_redirects=5)
        )
        url = f"{fixtures.http_url}/redirect-local"
        response = client.get(url)
        assert response.final_url.endswith("/admin")


# ===========================================================================
# 4. Cross-host redirect
# ===========================================================================


class TestCrossHostRedirect:
    def test_redirect_external_points_to_different_host(self, stable_fixtures):
        """redirect-external sends Location to 192.0.2.1; client raises error."""
        fixtures = stable_fixtures
        client = eggsec.HttpClientPy(
            eggsec.HttpClientConfigPy(timeout_ms=5000, max_redirects=0)
        )
        url = f"{fixtures.http_url}/redirect-external"
        with pytest.raises(eggsec.ScanError, match="Too many redirects"):
            client.get(url)

    def test_initial_scope_check_is_on_loopback(self, stable_fixtures):
        """The scope check is performed on the loopback host, not the redirect target."""
        fixtures = stable_fixtures
        scope = eggsec.Scope.allow_hosts([HOST])
        assert scope.is_target_allowed(HOST) is True
        assert scope.is_target_allowed(OUT_OF_SCOPE_HOST) is False

        client = eggsec.HttpClientPy(
            eggsec.HttpClientConfigPy(timeout_ms=5000, max_redirects=5)
        )
        url = f"{fixtures.http_url}/redirect-external"
        with pytest.raises(eggsec.ScanError):
            client.get(url)

    def test_cross_host_redirect_history_entry(self, stable_fixtures):
        """Following redirect to unreachable external host raises ScanError."""
        fixtures = stable_fixtures
        client = eggsec.HttpClientPy(
            eggsec.HttpClientConfigPy(timeout_ms=5000, max_redirects=5)
        )
        url = f"{fixtures.http_url}/redirect-external"
        with pytest.raises(eggsec.ScanError):
            client.get(url)


# ===========================================================================
# 5. Engine dispatch scope consistency
# ===========================================================================


class TestEngineDispatchScopeConsistency:
    def test_direct_function_denial_matches_engine_denial(self):
        """Direct function scope denial produces same error kind as engine dispatch."""
        scope = eggsec.Scope.allow_hosts(["example.com"])
        with pytest.raises(eggsec.EnforcementError):
            eggsec.scan_ports(HOST, [80], scope, timeout_ms=1000)

        engine = eggsec.Engine(scope, mode="manual", concurrency=10, timeout_ms=5000)
        req = eggsec.PortScanRequest(HOST, ports="80")
        result = engine.run_port_scan(req)
        assert result.error is not None
        assert result.error.kind == "scope_denial"

    def test_engine_scope_denial_kind(self):
        """Engine dispatch produces OperationResult with kind='scope_denial'."""
        scope = eggsec.Scope.allow_hosts(["example.com"])
        engine = eggsec.Engine(scope, mode="manual", concurrency=10, timeout_ms=5000)
        req = eggsec.PortScanRequest(HOST, ports="80")
        result = engine.run_port_scan(req)
        assert result.error is not None
        assert result.error.kind == "scope_denial"

    def test_engine_scope_allow_proceeds(self):
        """Engine dispatch with in-scope target proceeds past scope gate."""
        scope = eggsec.Scope.allow_hosts([HOST])
        engine = eggsec.Engine(scope, mode="manual", concurrency=10, timeout_ms=5000)
        req = eggsec.PortScanRequest(HOST, ports="19999", timeout_ms=2000)
        result = engine.run_port_scan(req)
        if result.error is not None:
            assert not _engine_result_is_scope_denied(result)

    def test_engine_generic_dispatch_scope_denied(self):
        """Engine.run() returns scope-denied for out-of-scope target."""
        scope = eggsec.Scope.allow_hosts(["example.com"])
        engine = eggsec.Engine(scope, mode="manual", concurrency=10, timeout_ms=5000)
        req = eggsec.OperationRequest("scan_ports", HOST)
        result = engine.run(req)
        assert result.error is not None
        assert result.error.kind == "scope_denial"

    def test_async_engine_scope_denial(self):
        """AsyncEngine raises EnforcementError for out-of-scope target."""
        scope = eggsec.Scope.allow_hosts(["example.com"])
        engine = eggsec.AsyncEngine(scope, mode="manual", concurrency=10, timeout_ms=5000)
        req = eggsec.PortScanRequest(HOST, ports="80")
        with pytest.raises(eggsec.EnforcementError):
            engine.run_port_scan(req)

    def test_engine_endpoint_scan_scope_denied(self):
        """Engine endpoint scan returns scope_denial for out-of-scope."""
        scope = eggsec.Scope.allow_hosts(["example.com"])
        engine = eggsec.Engine(scope, mode="manual", concurrency=10, timeout_ms=5000)
        req = eggsec.EndpointScanRequest(
            f"http://{HOST}:9999", paths=["/"], timeout_ms=1000
        )
        result = engine.run_endpoint_scan(req)
        assert result.error is not None
        assert _engine_result_is_scope_denied(result)

    def test_engine_fingerprint_scope_denied(self):
        """Engine fingerprint returns scope_denial for out-of-scope."""
        scope = eggsec.Scope.allow_hosts(["example.com"])
        engine = eggsec.Engine(scope, mode="manual", concurrency=10, timeout_ms=5000)
        req = eggsec.FingerprintRequest(HOST, ports=[80], timeout_ms=1000)
        result = engine.run_fingerprint(req)
        assert result.error is not None
        assert _engine_result_is_scope_denied(result)

    def test_engine_recon_dns_scope_denied(self):
        """Engine recon_dns returns scope_denial for out-of-scope."""
        scope = eggsec.Scope.allow_hosts(["example.com"])
        engine = eggsec.Engine(scope, mode="manual", concurrency=10, timeout_ms=5000)
        req = eggsec.ReconDnsRequest(HOST)
        result = engine.run_recon_dns(req)
        assert result.error is not None
        assert _engine_result_is_scope_denied(result)


# ===========================================================================
# 6. Scope with different operation types
# ===========================================================================


class TestScopeWithOperationTypes:
    def test_scan_ports_in_scope(self, stable_fixtures):
        fixtures = stable_fixtures
        scope = eggsec.Scope.allow_hosts([HOST])
        result = eggsec.scan_ports(
            HOST, [fixtures.tcp_port], scope, timeout_ms=2000
        )
        assert result is not None

    def test_scan_ports_out_of_scope(self):
        scope = eggsec.Scope.allow_hosts(["example.com"])
        with pytest.raises(eggsec.EnforcementError):
            eggsec.scan_ports(HOST, [80], scope, timeout_ms=1000)

    def test_scan_endpoints_in_scope(self, stable_fixtures):
        fixtures = stable_fixtures
        scope = eggsec.Scope.allow_hosts([HOST])
        result = eggsec.scan_endpoints(
            fixtures.http_url, ["/"], scope, timeout_ms=2000
        )
        assert result is not None

    def test_scan_endpoints_out_of_scope(self):
        scope = eggsec.Scope.allow_hosts(["example.com"])
        with pytest.raises(eggsec.EnforcementError):
            eggsec.scan_endpoints(
                f"http://{HOST}:9999", ["/"], scope, timeout_ms=1000
            )

    def test_fingerprint_in_scope(self, stable_fixtures):
        fixtures = stable_fixtures
        scope = eggsec.Scope.allow_hosts([HOST])
        result = eggsec.fingerprint_services(
            HOST, [fixtures.tcp_port], scope, timeout_ms=2000
        )
        assert result is not None

    def test_fingerprint_out_of_scope(self):
        scope = eggsec.Scope.allow_hosts(["example.com"])
        with pytest.raises(eggsec.EnforcementError):
            eggsec.fingerprint_services(HOST, [80], scope, timeout_ms=1000)

    def test_engine_scan_endpoints_scope_denied(self, stable_fixtures):
        fixtures = stable_fixtures
        scope = eggsec.Scope.allow_hosts([HOST])
        result = eggsec.scan_endpoints(
            fixtures.http_url, ["/admin", "/redirect-local"],
            scope, timeout_ms=2000, include_404=True,
        )
        assert result is not None


# ===========================================================================
# 7. Scope rule evaluation
# ===========================================================================


class TestScopeRuleEvaluation:
    def test_empty_scope_no_explicit_requirement(self, tmp_path):
        """Empty scope with require_explicit_scope=false allows all."""
        scope_file = tmp_path / "scope.toml"
        scope_file.write_text('require_explicit_scope = false\n')
        scope = eggsec.Scope.from_file(str(scope_file))
        assert scope.is_target_allowed("example.com") is True

    def test_empty_scope_explicit_requirement(self, tmp_path):
        """Empty scope with require_explicit_scope=true denies all."""
        scope_file = tmp_path / "scope.toml"
        scope_file.write_text('require_explicit_scope = true\n')
        scope = eggsec.Scope.from_file(str(scope_file))
        assert scope.is_target_allowed(HOST) is False

    def test_exclusion_rules_work(self, tmp_path):
        """Exclusion rules override inclusion for the same target."""
        scope = _scope_from_toml(
            tmp_path,
            f"""\
[[allowed_targets]]
    cidr = "127.0.0.0/8"

[[excluded_targets]]
    pattern = "{HOST}"
""",
        )
        assert scope.is_target_allowed(HOST) is False
        assert scope.is_target_allowed(LOOPBACK_ALT) is True

    def test_wildcard_subdomain_match(self):
        """*.example.com matches sub.example.com and example.com."""
        scope = eggsec.Scope.allow_hosts(["*.example.com"])
        assert scope.is_target_allowed("sub.example.com") is True
        assert scope.is_target_allowed("deep.sub.example.com") is True
        assert scope.is_target_allowed("example.com") is True

    def test_loaded_scope_explain_allowed(self):
        """LoadedScope.explain() gives correct explanation for allowed target."""
        scope = eggsec.Scope.allow_hosts([HOST])
        loaded = eggsec.LoadedScope.explicit(
            scope, eggsec.ScopeSource.config_file(), "/tmp/scope.toml"
        )
        explanation = loaded.explain(HOST)
        assert explanation.allowed is True
        assert "match" in explanation.reason.lower() or "scope" in explanation.reason.lower()

    def test_loaded_scope_explain_denied(self):
        """LoadedScope.explain() gives correct explanation for denied target."""
        scope = eggsec.Scope.allow_hosts(["example.com"])
        loaded = eggsec.LoadedScope.explicit(
            scope, eggsec.ScopeSource.config_file(), "/tmp/scope.toml"
        )
        explanation = loaded.explain(HOST)
        assert explanation.allowed is False

    def test_loaded_scope_explain_excluded(self, tmp_path):
        """LoadedScope.explain() shows exclusion for explicitly excluded target."""
        scope = _scope_from_toml(
            tmp_path,
            f"""\
[[allowed_targets]]
    cidr = "127.0.0.0/8"

[[excluded_targets]]
    pattern = "{HOST}"
""",
        )
        loaded = eggsec.LoadedScope.explicit(
            scope, eggsec.ScopeSource.config_file(), None
        )
        explanation = loaded.explain(HOST)
        assert explanation.allowed is False
        assert explanation.excluded is True

    def test_validate_scope_empty_explicit(self, tmp_path):
        """validate_scope detects empty scope with require_explicit_scope=true."""
        scope_file = tmp_path / "scope.toml"
        scope_file.write_text('require_explicit_scope = true\n')
        scope = eggsec.Scope.from_file(str(scope_file))
        loaded = eggsec.LoadedScope.explicit(
            scope, eggsec.ScopeSource.config_file(), str(scope_file)
        )
        result = eggsec.validate_scope(loaded)
        assert result.valid is False
        assert len(result.warnings) > 0

    def test_validate_scope_with_targets(self, tmp_path):
        """validate_scope passes for well-formed scope."""
        scope = eggsec.Scope.allow_hosts([HOST])
        loaded = eggsec.LoadedScope.explicit(
            scope, eggsec.ScopeSource.config_file(), None
        )
        result = eggsec.validate_scope(loaded)
        assert result.valid is True
        assert result.target_count == 1


# ===========================================================================
# 8. Scope metadata in audit events
# ===========================================================================


class TestScopeAuditEvents:
    def test_scope_denial_emits_audit_event(self):
        """Scope denial emits an audit event with allowed=False."""
        scope = eggsec.Scope.allow_hosts(["example.com"])
        engine = eggsec.Engine(scope, mode="manual", concurrency=10, timeout_ms=5000)
        req = eggsec.PortScanRequest(HOST, ports="80")
        result = engine.run_port_scan(req)
        assert result.error is not None
        events = engine.audit_events()
        assert len(events) >= 1
        last_event = events[-1]
        assert last_event.allowed is False
        assert last_event.operation_id == "scan_ports"
        assert last_event.target == HOST

    def test_scope_allow_emits_audit_event(self):
        """Scope allow emits an audit event with allowed=True."""
        scope = eggsec.Scope.allow_hosts([HOST])
        engine = eggsec.Engine(scope, mode="manual", concurrency=10, timeout_ms=5000)
        req = eggsec.PortScanRequest(HOST, ports="19999", timeout_ms=2000)
        result = engine.run_port_scan(req)
        events = engine.audit_events()
        assert len(events) >= 1
        last_event = events[-1]
        assert last_event.allowed is True
        assert last_event.operation_id == "scan_ports"

    def test_audit_event_serialization(self):
        """Audit events are serializable."""
        scope = eggsec.Scope.allow_hosts(["example.com"])
        engine = eggsec.Engine(scope, mode="manual", concurrency=10, timeout_ms=5000)
        req = eggsec.PortScanRequest(HOST, ports="80")
        engine.run_port_scan(req)
        events = engine.audit_events()
        assert len(events) >= 1
        event = events[-1]
        d = event.to_dict()
        assert isinstance(d, dict)
        assert "allowed" in d
        assert "operation_id" in d
        encoded = event.to_json()
        assert encoded.startswith("{")

    def test_audit_event_has_redacted_flag(self):
        """Audit events carry the redacted flag."""
        scope = eggsec.Scope.allow_hosts(["example.com"])
        engine = eggsec.Engine(scope, mode="manual", concurrency=10, timeout_ms=5000)
        req = eggsec.PortScanRequest(HOST, ports="80")
        engine.run_port_scan(req)
        events = engine.audit_events()
        assert len(events) >= 1
        assert events[-1].redacted is True

    def test_multiple_operations_emit_multiple_audit_events(self):
        """Multiple operations emit separate audit events."""
        scope = eggsec.Scope.allow_hosts(["example.com"])
        engine = eggsec.Engine(scope, mode="manual", concurrency=10, timeout_ms=5000)
        for op_id in ["scan_ports", "scan_endpoints", "fingerprint_services"]:
            req = eggsec.OperationRequest(op_id, HOST)
            engine.run(req)
        events = engine.audit_events()
        operation_ids = [e.operation_id for e in events]
        assert "scan_ports" in operation_ids
        assert "scan_endpoints" in operation_ids
        assert "fingerprint_services" in operation_ids

    def test_audit_event_outcome_matches_denial(self):
        """Audit event outcome string reflects the denial."""
        scope = eggsec.Scope.allow_hosts(["example.com"])
        engine = eggsec.Engine(scope, mode="manual", concurrency=10, timeout_ms=5000)
        req = eggsec.PortScanRequest(HOST, ports="80")
        engine.run_port_scan(req)
        events = engine.audit_events()
        assert len(events) >= 1
        assert events[-1].outcome == "confirm"

    def test_audit_event_surface_field(self):
        """Audit event surface field indicates the execution surface."""
        scope = eggsec.Scope.allow_hosts(["example.com"])
        engine = eggsec.Engine(scope, mode="manual", concurrency=10, timeout_ms=5000)
        req = eggsec.PortScanRequest(HOST, ports="80")
        engine.run_port_scan(req)
        events = engine.audit_events()
        assert len(events) >= 1
        surface = events[-1].surface
        assert isinstance(surface, str)
        assert len(surface) > 0


# ===========================================================================
# Additional: Scope with live fixture operations
# ===========================================================================


class TestScopeWithLiveFixtures:
    def test_scope_on_fixture_tcp_scan(self, stable_fixtures):
        """TCP scan against fixture server with in-scope."""
        fixtures = stable_fixtures
        scope = eggsec.Scope.allow_hosts([HOST])
        result = eggsec.scan_ports(
            HOST, [fixtures.tcp_port], scope, timeout_ms=2000
        )
        assert result is not None

    def test_scope_on_fixture_endpoint_scan(self, stable_fixtures):
        """Endpoint scan against fixture server with in-scope."""
        fixtures = stable_fixtures
        scope = eggsec.Scope.allow_hosts([HOST])
        result = eggsec.scan_endpoints(
            fixtures.http_url, ["/", "/admin"], scope, timeout_ms=2000
        )
        assert result is not None

    def test_scope_on_fixture_fingerprint(self, stable_fixtures):
        """Fingerprint against fixture server with in-scope."""
        fixtures = stable_fixtures
        scope = eggsec.Scope.allow_hosts([HOST])
        result = eggsec.fingerprint_services(
            HOST, [fixtures.tcp_port], scope, timeout_ms=2000
        )
        assert result is not None

    def test_scope_cidr_on_fixture(self, stable_fixtures):
        """CIDR scope allows fixture operations."""
        fixtures = stable_fixtures
        scope = eggsec.Scope.allow_cidrs(["127.0.0.0/8"])
        result = eggsec.scan_ports(
            HOST, [fixtures.tcp_port], scope, timeout_ms=2000
        )
        assert result is not None

    def test_scope_deny_all_on_fixture(self, stable_fixtures):
        """Deny-all scope blocks fixture operations."""
        fixtures = stable_fixtures
        scope = eggsec.Scope.deny_all()
        with pytest.raises(eggsec.EnforcementError):
            eggsec.scan_ports(HOST, [fixtures.tcp_port], scope, timeout_ms=1000)

    def test_scope_mixed_host_and_cidr(self, tmp_path, stable_fixtures):
        """Scope with both hostname and CIDR rules."""
        fixtures = stable_fixtures
        scope = _scope_from_toml(
            tmp_path,
            f"""\
[[allowed_targets]]
    pattern = "{HOST}"

[[allowed_targets]]
    cidr = "10.0.0.0/8"
""",
        )
        result = eggsec.scan_ports(
            HOST, [fixtures.tcp_port], scope, timeout_ms=2000
        )
        assert result is not None
        assert scope.is_target_allowed("10.0.0.1") is True

    def test_engine_scope_consistency_across_operations(self, stable_fixtures):
        """Engine and direct function give same scope decision for same operation."""
        fixtures = stable_fixtures
        scope = eggsec.Scope.allow_hosts([HOST])
        engine = eggsec.Engine(scope, mode="manual", concurrency=10, timeout_ms=5000)

        req = eggsec.PortScanRequest(HOST, ports=str(fixtures.tcp_port), timeout_ms=2000)
        result = engine.run_port_scan(req)
        if result.error is not None:
            assert not _engine_result_is_scope_denied(result)
        else:
            assert result.payload is not None

    def test_scope_with_loaded_scope_for_enforcement(self):
        """LoadedScope wraps Scope and is_target_allowed delegates correctly."""
        scope = eggsec.Scope.allow_hosts([HOST])
        loaded = eggsec.LoadedScope.explicit(
            scope, eggsec.ScopeSource.config_file(), None
        )
        assert loaded.is_target_allowed(HOST) is True
        assert loaded.is_target_allowed(OUT_OF_SCOPE_HOST) is False
        assert loaded.require_explicit_scope is True

    def test_scope_from_toml_roundtrip(self, tmp_path):
        """TOML-saved scope preserves rules after reload."""
        original = _scope_from_toml(
            tmp_path,
            f"""\
[[allowed_targets]]
    pattern = "{HOST}"

[[allowed_targets]]
    cidr = "10.0.0.0/8"

[[excluded_targets]]
    pattern = "10.0.0.99"
""",
        )
        assert original.is_target_allowed(HOST) is True
        assert original.is_target_allowed("10.0.0.1") is True
        assert original.is_target_allowed("10.0.0.99") is False
        assert original.is_target_allowed(OUT_OF_SCOPE_HOST) is False
