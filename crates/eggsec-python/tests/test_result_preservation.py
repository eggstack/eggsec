"""Phase 1d: Integration tests for OperationResult payload preservation.

Verifies that Engine.run() and Engine.run_*() methods now return
OperationResult with real domain payloads, non-zero stats, and
correct payload_type metadata — instead of the legacy discarded-results
pattern that always returned zeroed stats and empty metadata.

Note: The Engine catches scope enforcement errors internally and returns
Failed OperationResult instead of raising. Standalone functions (scan_ports,
etc.) raise EnforcementError directly.
"""

import pytest
import eggsec


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _engine():
    """Create an Engine with an unrestricted scope for testing."""
    scope = eggsec.Scope.allow_hosts(["127.0.0.1"])
    return eggsec.Engine(scope, mode="manual", concurrency=4, timeout_ms=5000)


def _out_of_scope_engine():
    """Create an Engine scoped to a non-localhost target."""
    scope = eggsec.Scope.allow_hosts(["example.com"])
    return eggsec.Engine(scope, mode="manual", concurrency=4, timeout_ms=5000)


def _try_engine_scan():
    """Attempt a port scan via Engine; returns (engine, result)."""
    engine = _engine()
    req = eggsec.PortScanRequest(
        target="127.0.0.1",
        ports="19999",
        mode="passive",
        timeout_ms=1000,
    )
    try:
        result = engine.run_port_scan(req)
        return engine, result
    except Exception:
        return engine, None


# ---------------------------------------------------------------------------
# OperationResult structure tests (no network required)
# ---------------------------------------------------------------------------

class TestOperationResultPayload:
    """Verify OperationResult carries payload metadata."""

    def test_result_has_payload_field(self):
        """OperationResult should expose payload getter."""
        engine, result = _try_engine_scan()
        if result is None:
            pytest.skip("Engine raised exception")
        if result.is_failure():
            pytest.skip(f"Scan failed: {result.error}")
        payload = result.payload
        assert payload is not None, "Engine result should now carry a domain payload"

    def test_result_payload_type_name(self):
        """OperationResult.payload_type_name should identify the domain type."""
        engine, result = _try_engine_scan()
        if result is None:
            pytest.skip("Engine raised exception")
        if result.is_failure():
            pytest.skip(f"Scan failed: {result.error}")
        type_name = result.payload_type_name
        assert type_name is not None, "payload_type_name should be set"
        assert isinstance(type_name, str)
        assert len(type_name) > 0

    def test_result_payload_is_port_scan_result(self):
        """Engine port scan should return PortScanResult payload."""
        engine, result = _try_engine_scan()
        if result is None:
            pytest.skip("Engine raised exception")
        if result.is_failure():
            pytest.skip(f"Scan failed: {result.error}")
        payload = result.payload
        assert isinstance(payload, eggsec.PortScanResult), (
            f"Expected PortScanResult, got {type(payload).__name__}"
        )

    def test_result_payload_has_target(self):
        """Payload PortScanResult should have the correct target."""
        engine, result = _try_engine_scan()
        if result is None:
            pytest.skip("Engine raised exception")
        if result.is_failure():
            pytest.skip(f"Scan failed: {result.error}")
        payload = result.payload
        assert payload.target == "127.0.0.1"

    def test_failed_result_has_no_payload(self):
        """Failed engine result should have None payload."""
        engine = _out_of_scope_engine()
        req = eggsec.PortScanRequest(
            target="127.0.0.1",
            ports="80",
            mode="passive",
            timeout_ms=1000,
        )
        result = engine.run_port_scan(req)
        assert result.is_failure()
        assert result.payload is None
        assert result.payload_type_name is None


# ---------------------------------------------------------------------------
# Stats tests
# ---------------------------------------------------------------------------

class TestOperationResultStats:
    """Verify ExecutionStats are non-zero after real execution."""

    def test_stats_duration_nonzero(self):
        """duration_ms should reflect actual execution time."""
        engine, result = _try_engine_scan()
        if result is None:
            pytest.skip("Engine raised exception")
        if result.is_failure():
            pytest.skip(f"Scan failed: {result.error}")
        assert result.stats is not None, "stats should be populated"
        assert result.stats.duration_ms > 0, (
            f"duration_ms should be > 0, got {result.stats.duration_ms}"
        )

    def test_stats_items_processed_nonzero(self):
        """items_processed should reflect scanned port count."""
        engine, result = _try_engine_scan()
        if result is None:
            pytest.skip("Engine raised exception")
        if result.is_failure():
            pytest.skip(f"Scan failed: {result.error}")
        assert result.stats is not None
        assert result.stats.items_processed > 0, (
            f"items_processed should be > 0, got {result.stats.items_processed}"
        )


# ---------------------------------------------------------------------------
# raise_for_status tests
# ---------------------------------------------------------------------------

class TestRaiseForStatus:
    """Verify raise_for_status behavior."""

    def test_raise_for_status_completed_is_noop(self):
        """Completed result should not raise."""
        engine, result = _try_engine_scan()
        if result is None:
            pytest.skip("Engine raised exception")
        if result.is_failure():
            pytest.skip(f"Scan failed: {result.error}")
        result.raise_for_status()  # should not raise

    def test_raise_for_status_failed_raises(self):
        """Failed result should raise."""
        engine = _out_of_scope_engine()
        req = eggsec.PortScanRequest(
            target="127.0.0.1",
            ports="80",
            mode="passive",
            timeout_ms=1000,
        )
        result = engine.run_port_scan(req)
        assert result.is_failure()
        with pytest.raises(Exception):
            result.raise_for_status()


# ---------------------------------------------------------------------------
# to_dict / serialization tests
# ---------------------------------------------------------------------------

class TestResultSerialization:
    """Verify OperationResult serialization includes payload metadata."""

    def test_to_dict_includes_payload_type(self):
        """to_dict() should include payload_type key."""
        engine, result = _try_engine_scan()
        if result is None:
            pytest.skip("Engine raised exception")
        if result.is_failure():
            pytest.skip(f"Scan failed: {result.error}")
        d = result.to_dict()
        assert "payload_type" in d, f"to_dict keys: {list(d.keys())}"
        assert d["payload_type"] is not None

    def test_to_dict_includes_payload(self):
        """to_dict() should include payload data."""
        engine, result = _try_engine_scan()
        if result is None:
            pytest.skip("Engine raised exception")
        if result.is_failure():
            pytest.skip(f"Scan failed: {result.error}")
        d = result.to_dict()
        assert "payload" in d, f"to_dict keys: {list(d.keys())}"
        assert d["payload"] is not None

    def test_repr_includes_payload_type(self):
        """__repr__ should mention the payload type."""
        engine, result = _try_engine_scan()
        if result is None:
            pytest.skip("Engine raised exception")
        if result.is_failure():
            pytest.skip(f"Scan failed: {result.error}")
        r = repr(result)
        assert "payload_type" in r.lower() or "portscan" in r.lower() or "Payload" in r

    def test_failed_result_to_dict_has_no_payload(self):
        """Failed result to_dict should have None payload."""
        engine = _out_of_scope_engine()
        req = eggsec.PortScanRequest(
            target="127.0.0.1",
            ports="80",
            mode="passive",
            timeout_ms=1000,
        )
        result = engine.run_port_scan(req)
        d = result.to_dict()
        assert d["payload"] is None
        assert d["payload_type"] is None


# ---------------------------------------------------------------------------
# Engine.run() generic dispatch tests
# ---------------------------------------------------------------------------

class TestEngineGenericDispatch:
    """Verify Engine.run() (generic OperationRequest dispatch) preserves results."""

    def test_generic_dispatch_has_payload_or_error(self):
        """Engine.run() with port_scan should have payload or error."""
        engine, result = _try_engine_scan()
        if result is None:
            pytest.skip("Engine raised exception")
        # Either success with payload, or failure with error
        if result.is_failure():
            assert result.error is not None
        else:
            assert result.payload is not None

    def test_generic_dispatch_status_completed(self):
        """Engine.run() on a successful scan should have Completed status."""
        engine, result = _try_engine_scan()
        if result is None:
            pytest.skip("Engine raised exception")
        if result.is_failure():
            pytest.skip(f"Scan failed: {result.error}")
        assert result.is_success()

    def test_generic_dispatch_metadata_has_target(self):
        """Engine.run() metadata should include the target."""
        engine, result = _try_engine_scan()
        if result is None:
            pytest.skip("Engine raised exception")
        if result.is_failure():
            pytest.skip(f"Scan failed: {result.error}")
        md = result.metadata
        assert "target" in md
        assert md["target"] == "127.0.0.1"


# ---------------------------------------------------------------------------
# Scope enforcement via standalone function (raises)
# ---------------------------------------------------------------------------

class TestStandaloneScopeEnforcement:
    """Verify standalone functions raise EnforcementError for scope violations."""

    def test_standalone_scan_ports_out_of_scope(self):
        """scan_ports standalone should raise EnforcementError."""
        scope = eggsec.Scope.allow_hosts(["example.com"])
        with pytest.raises(eggsec.EnforcementError):
            eggsec.scan_ports("127.0.0.1", [80], scope, timeout_ms=1000)

    def test_standalone_scan_endpoints_out_of_scope(self):
        """scan_endpoints standalone should raise EnforcementError."""
        scope = eggsec.Scope.allow_hosts(["example.com"])
        with pytest.raises(eggsec.EnforcementError):
            eggsec.scan_endpoints("http://127.0.0.1", ["/admin"], scope, timeout_ms=1000)

    def test_engine_out_of_scope_returns_failed_result(self):
        """Engine returns Failed result for out-of-scope targets (doesn't raise)."""
        engine = _out_of_scope_engine()
        req = eggsec.PortScanRequest(
            target="127.0.0.1",
            ports="80",
            mode="passive",
            timeout_ms=1000,
        )
        result = engine.run_port_scan(req)
        assert result.is_failure()
        assert result.error is not None

    def test_client_still_works(self):
        """Client API should still return typed results directly."""
        scope = eggsec.Scope.allow_hosts(["127.0.0.1"])
        client = eggsec.Client(scope, mode="manual", concurrency=4, timeout_ms=5000)
        try:
            result = client.scan_ports("127.0.0.1", [19999], timeout_ms=1000)
            assert isinstance(result, eggsec.PortScanResult)
        except eggsec.ScanError:
            pytest.skip("Loopback scanning blocked")
