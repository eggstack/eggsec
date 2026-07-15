"""Cancellation, timeout, and resource cleanup tests.

Covers CancellationToken lifecycle, Engine/AsyncEngine timeout configuration,
scope enforcement, close idempotency, use-after-close behavior,
OperationRequest construction and serialization, and OperationResult
structure including raise_for_status().
"""

import json
import os
import pytest

import eggsec

SENTINEL_LOOPBACK = "127.0.0.1"
LOOPBACK_ALLOWED = os.environ.get("EGGSEC_ALLOW_LOOPBACK_FIXTURE", "0") == "1"

# Enable loopback fixture access for tests that need it
os.environ["EGGSEC_ALLOW_LOOPBACK_FIXTURE"] = "1"


# ============================================================================
# 1-5: CancellationToken lifecycle
# ============================================================================


class TestCancellationToken:
    def test_cancellation_token_creation(self):
        token = eggsec.CancellationToken()
        assert token.is_cancelled() is False

    def test_cancellation_token_cancel(self):
        token = eggsec.CancellationToken()
        token.cancel("test reason")
        assert token.is_cancelled() is True
        assert token.reason() == "test reason"

    def test_cancellation_token_to_dict(self):
        token = eggsec.CancellationToken()
        d = token.to_dict()
        assert isinstance(d, dict)
        assert d["cancelled"] is False

        token.cancel("dict-test")
        d = token.to_dict()
        assert d["cancelled"] is True
        assert d["reason"] == "dict-test"

    def test_cancellation_token_to_json(self):
        token = eggsec.CancellationToken()
        j = token.to_json()
        parsed = json.loads(j)
        assert parsed["cancelled"] is False

        token.cancel("json-test")
        j = token.to_json()
        parsed = json.loads(j)
        assert parsed["cancelled"] is True
        assert parsed["reason"] == "json-test"

    def test_cancellation_token_double_cancel(self):
        token = eggsec.CancellationToken()
        token.cancel("first")
        assert token.is_cancelled() is True
        assert token.reason() == "first"
        token.cancel("second")
        assert token.is_cancelled() is True


# ============================================================================
# 6: Pre-cancelled operation
# ============================================================================


class TestPreCancelledOperation:
    def test_pre_cancelled_operation(self):
        """Dispatch with pre-cancelled token should report Cancelled status via pipeline."""
        from eggsec import Pipeline

        scope = eggsec.Scope.allow_hosts([SENTINEL_LOOPBACK])
        engine = eggsec.Engine(scope, timeout_ms=2000)
        token = eggsec.CancellationToken()
        token.cancel("pre-cancel")

        pipeline = Pipeline("pre-cancel-test")
        pipeline.add_step(
            "step-a",
            eggsec.OperationRequest("scan_ports", SENTINEL_LOOPBACK, timeout_ms=2000),
        )
        pipeline.set_cancel_token(token)
        result = pipeline.run(engine)
        assert result.status.name() == "Cancelled"
        engine.close()


# ============================================================================
# 7: Engine timeout config
# ============================================================================


class TestEngineTimeoutConfig:
    def test_engine_timeout_config(self):
        scope = eggsec.Scope.allow_hosts([SENTINEL_LOOPBACK])
        engine = eggsec.Engine(scope, timeout_ms=12345)
        assert engine.timeout_ms == 12345
        engine.close()

    def test_engine_default_timeout(self):
        scope = eggsec.Scope.allow_hosts([SENTINEL_LOOPBACK])
        engine = eggsec.Engine(scope)
        assert engine.timeout_ms == 5000
        engine.close()


# ============================================================================
# 8: Engine scope enforcement
# ============================================================================


class TestEngineScopeEnforcement:
    def test_engine_scope_enforcement(self):
        """Engine with deny_all returns failed result with EnforcementError."""
        scope = eggsec.Scope.deny_all()
        engine = eggsec.Engine(scope, timeout_ms=2000)
        req = eggsec.OperationRequest("scan_ports", SENTINEL_LOOPBACK, timeout_ms=2000)
        result = engine.run(req)
        assert result.is_failure() is True
        assert result.error is not None
        assert "scope" in str(result.error).lower() or "enforcement" in str(result.error).lower()
        engine.close()


# ============================================================================
# 9-10: Engine close idempotency and use-after-close
# ============================================================================


class TestEngineResourceCleanup:
    def test_engine_close_idempotent(self):
        scope = eggsec.Scope.allow_hosts([SENTINEL_LOOPBACK])
        engine = eggsec.Engine(scope)
        engine.close()
        engine.close()

    def test_engine_use_after_close(self):
        """Engine may still accept requests after close (no hard resource cleanup)."""
        scope = eggsec.Scope.allow_hosts([SENTINEL_LOOPBACK])
        engine = eggsec.Engine(scope, timeout_ms=2000)
        engine.close()
        req = eggsec.OperationRequest("scan_ports", SENTINEL_LOOPBACK, timeout_ms=2000)
        result = engine.run(req)
        assert result.status.name() == "Completed"


# ============================================================================
# 11-12: AsyncEngine close idempotency and use-after-close
# ============================================================================


class TestAsyncEngineResourceCleanup:
    def test_async_engine_close_idempotent(self):
        scope = eggsec.Scope.allow_hosts([SENTINEL_LOOPBACK])
        engine = eggsec.AsyncEngine(scope)
        engine.close()
        engine.close()

    def test_async_engine_use_after_close(self):
        """AsyncEngine may still accept requests after close (no hard resource cleanup)."""
        scope = eggsec.Scope.allow_hosts([SENTINEL_LOOPBACK])
        engine = eggsec.AsyncEngine(scope, timeout_ms=2000)
        engine.close()
        req = eggsec.OperationRequest("scan_ports", SENTINEL_LOOPBACK, timeout_ms=2000)
        result = engine.run(req)
        assert result is not None


# ============================================================================
# 13-16: OperationRequest construction and serialization
# ============================================================================


class TestOperationRequest:
    def test_operation_request_construction(self):
        req = eggsec.OperationRequest(
            "scan_ports",
            "192.168.1.1",
            timeout_ms=10000,
            metadata={"ports": "80,443", "trace_id": "abc-123"},
        )
        assert req.operation == "scan_ports"
        assert req.target == "192.168.1.1"
        assert req.timeout_ms == 10000
        assert req.metadata["ports"] == "80,443"
        assert req.metadata["trace_id"] == "abc-123"

    def test_operation_request_defaults(self):
        req = eggsec.OperationRequest("recon_dns", "example.com")
        assert req.operation == "recon_dns"
        assert req.target == "example.com"
        assert req.timeout_ms is None
        assert req.metadata == {}

    def test_operation_request_to_dict(self):
        req = eggsec.OperationRequest(
            "scan_endpoints",
            "10.0.0.1",
            timeout_ms=5000,
            metadata={"path": "/admin"},
        )
        d = req.to_dict()
        assert isinstance(d, dict)
        assert d["operation"] == "scan_endpoints"
        assert d["target"] == "10.0.0.1"
        assert d["timeout_ms"] == 5000
        assert d["metadata"]["path"] == "/admin"

    def test_operation_request_to_json(self):
        req = eggsec.OperationRequest(
            "fingerprint_services",
            SENTINEL_LOOPBACK,
            timeout_ms=3000,
        )
        j = req.to_json()
        parsed = json.loads(j)
        assert parsed["operation"] == "fingerprint_services"
        assert parsed["target"] == SENTINEL_LOOPBACK
        assert parsed["timeout_ms"] == 3000


# ============================================================================
# 17-21: OperationResult structure
# ============================================================================


class TestOperationResultStructure:
    def test_operation_result_status_completed(self):
        """Successful result should have Completed status."""
        scope = eggsec.Scope.allow_hosts([SENTINEL_LOOPBACK])
        engine = eggsec.Engine(scope, timeout_ms=10000)
        req = eggsec.OperationRequest("scan_ports", SENTINEL_LOOPBACK, timeout_ms=10000)
        result = engine.run(req)
        assert result.is_success() is True
        assert result.is_failure() is False
        assert result.status.name() == "Completed"
        assert result.error is None
        engine.close()

    def test_operation_result_status_failed(self):
        """Failed result should have Failed status and error."""
        scope = eggsec.Scope.deny_all()
        engine = eggsec.Engine(scope, timeout_ms=2000)
        req = eggsec.OperationRequest("scan_ports", SENTINEL_LOOPBACK, timeout_ms=2000)
        result = engine.run(req)
        assert result.is_failure() is True
        assert result.is_success() is False
        assert result.status.name() == "Failed"
        assert result.error is not None
        engine.close()

    def test_operation_result_error_structure(self):
        """OperationError should have expected fields."""
        err = eggsec.OperationError(
            kind="enforcement",
            code="SCOPE_DENIED",
            message="Target out of scope",
            operation_id="scan_ports",
            retryable=False,
            denial_class="scope",
            source="engine",
            details={"target": "evil.com"},
            causes=["denied by policy"],
        )
        assert err.kind == "enforcement"
        assert err.code == "SCOPE_DENIED"
        assert err.message == "Target out of scope"
        assert err.operation_id == "scan_ports"
        assert err.retryable is False
        assert err.denial_class == "scope"
        assert err.source == "engine"
        assert err.details == {"target": "evil.com"}
        assert err.causes == ["denied by policy"]

    def test_operation_result_error_to_dict(self):
        err = eggsec.OperationError(kind="timeout", code="T", message="timed out")
        d = err.to_dict()
        assert isinstance(d, dict)
        assert d["kind"] == "timeout"
        assert d["code"] == "T"
        assert d["message"] == "timed out"

    def test_operation_result_error_to_json(self):
        err = eggsec.OperationError(kind="network", code="N", message="conn refused")
        j = err.to_json()
        parsed = json.loads(j)
        assert parsed["kind"] == "network"

    def test_operation_result_raise_for_status(self):
        """raise_for_status() should not raise on success."""
        scope = eggsec.Scope.allow_hosts([SENTINEL_LOOPBACK])
        engine = eggsec.Engine(scope, timeout_ms=10000)
        req = eggsec.OperationRequest("scan_ports", SENTINEL_LOOPBACK, timeout_ms=10000)
        result = engine.run(req)
        result.raise_for_status()
        engine.close()

    def test_operation_result_raise_for_status_on_error(self):
        """raise_for_status() should raise on failure."""
        scope = eggsec.Scope.deny_all()
        engine = eggsec.Engine(scope, timeout_ms=2000)
        req = eggsec.OperationRequest("scan_ports", SENTINEL_LOOPBACK, timeout_ms=2000)
        result = engine.run(req)
        with pytest.raises(Exception):
            result.raise_for_status()
        engine.close()

    def test_operation_result_metadata(self):
        """Result should have a metadata dict."""
        scope = eggsec.Scope.allow_hosts([SENTINEL_LOOPBACK])
        engine = eggsec.Engine(scope, timeout_ms=10000)
        req = eggsec.OperationRequest("scan_ports", SENTINEL_LOOPBACK, timeout_ms=10000)
        result = engine.run(req)
        assert isinstance(result.metadata, dict)
        engine.close()

    def test_operation_result_to_dict(self):
        scope = eggsec.Scope.allow_hosts([SENTINEL_LOOPBACK])
        engine = eggsec.Engine(scope, timeout_ms=10000)
        req = eggsec.OperationRequest("scan_ports", SENTINEL_LOOPBACK, timeout_ms=10000)
        result = engine.run(req)
        d = result.to_dict()
        assert isinstance(d, dict)
        assert "status" in d
        engine.close()

    def test_operation_result_to_json(self):
        scope = eggsec.Scope.allow_hosts([SENTINEL_LOOPBACK])
        engine = eggsec.Engine(scope, timeout_ms=10000)
        req = eggsec.OperationRequest("scan_ports", SENTINEL_LOOPBACK, timeout_ms=10000)
        result = engine.run(req)
        j = result.to_json()
        parsed = json.loads(j)
        assert "status" in parsed
        engine.close()
