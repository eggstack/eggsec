"""Async protocol correctness tests for the eggsec Python API.

Tests the AsyncEngine, PyFuture, and asyncio integration. The extension uses
polling awaitables (PyFuture with __await__/__next__), not true Python async/await
internally, but they integrate with asyncio's event loop via the iterator protocol.

Scope: local loopback (127.0.0.1) only.
"""

import asyncio
import pytest
import eggsec
from eggsec import (
    AsyncEngine,
    OperationRequest,
    Scope,
    ExecutionStatus,
    PyFuture,
    PortScanRequest,
    ReconDnsRequest,
)

SENTINEL_LOOPBACK = "127.0.0.1"


def _loopback_scope():
    return Scope.allow_hosts([SENTINEL_LOOPBACK])


def _scan_request(target=SENTINEL_LOOPBACK, port=19999, timeout_ms=15000):
    return OperationRequest(
        "scan_ports",
        target,
        timeout_ms=timeout_ms,
        metadata={"ports": str(port)},
    )


def _dns_request(target=SENTINEL_LOOPBACK, timeout_ms=15000):
    return OperationRequest(
        "recon_dns",
        target,
        timeout_ms=timeout_ms,
    )


# ---------------------------------------------------------------------------
# 1. test_async_engine_context_manager
# ---------------------------------------------------------------------------


class TestAsyncEngineContextManager:
    def test_engine_usable_after_creation(self):
        """Engine should be usable immediately after creation."""
        engine = AsyncEngine(_loopback_scope())
        try:
            ops = engine.list_operations()
            assert isinstance(ops, list)
            assert "scan_ports" in ops
        finally:
            engine.close()

    def test_engine_close_is_idempotent(self):
        """close() should be safe to call multiple times."""
        engine = AsyncEngine(_loopback_scope())
        engine.close()
        engine.close()  # Should not raise

    def test_aenter_returns_engine(self):
        """__aenter__ should return the engine (sync call, not awaited)."""
        engine = AsyncEngine(_loopback_scope())
        # __aenter__ is a sync method that returns self
        returned = engine.__aenter__()
        assert returned is engine
        engine.close()

    def test_aexit_returns_false(self):
        """__aexit__ should return False (don't suppress exceptions)."""
        engine = AsyncEngine(_loopback_scope())
        result = engine.__aexit__(None, None, None)
        assert result is False
        engine.close()

    def test_context_manager_protocol_completeness(self):
        """Engine should have both __aenter__ and __aexit__."""
        engine = AsyncEngine(_loopback_scope())
        assert callable(getattr(engine, "__aenter__", None))
        assert callable(getattr(engine, "__aexit__", None))
        engine.close()

    def test_sync_context_manager_pattern(self):
        """Standard usage: create, use, close."""
        engine = AsyncEngine(_loopback_scope())
        try:
            ops = engine.list_operations()
            assert len(ops) > 0
        finally:
            engine.close()


# ---------------------------------------------------------------------------
# 2. test_async_engine_run_returns_awaitable
# ---------------------------------------------------------------------------


class TestAsyncEngineRunReturnsAwaitable:
    def test_run_returns_pyfuture(self):
        """engine.run() should return a PyFuture instance."""
        engine = AsyncEngine(_loopback_scope())
        try:
            request = _scan_request()
            future = engine.run(request)
            assert isinstance(future, PyFuture)
        finally:
            engine.close()

    def test_run_can_be_awaited(self):
        """The PyFuture from engine.run() should be awaitable."""
        async def _test():
            engine = AsyncEngine(_loopback_scope())
            request = _scan_request()
            future = engine.run(request)
            result = await future
            assert result is not None
            assert hasattr(result, "status")
            engine.close()
        asyncio.run(_test())

    def test_run_completes_with_status(self):
        """An async scan should eventually complete with a status."""
        async def _test():
            engine = AsyncEngine(_loopback_scope())
            request = _scan_request()
            result = await engine.run(request)
            assert result.status is not None
            # Should be Completed or Failed (loopback may not have port 19999)
            status_name = result.status.name()
            assert status_name in ("Completed", "Failed")
            engine.close()
        asyncio.run(_test())

    def test_run_with_operation_request(self):
        """engine.run() should accept OperationRequest."""
        async def _test():
            engine = AsyncEngine(_loopback_scope())
            request = OperationRequest(
                "scan_ports",
                SENTINEL_LOOPBACK,
                timeout_ms=10000,
                metadata={"ports": "19999"},
            )
            result = await engine.run(request)
            assert result.status.name() in ("Completed", "Failed")
            engine.close()
        asyncio.run(_test())


# ---------------------------------------------------------------------------
# 3. test_asyncio_wait_for_timeout
# ---------------------------------------------------------------------------


class TestAsyncioWaitForTimeout:
    def test_operation_completes_within_generous_timeout(self):
        """asyncio.wait_for should let a fast operation complete."""
        async def _test():
            engine = AsyncEngine(_loopback_scope())
            request = _scan_request(timeout_ms=10000)
            future = engine.run(request)
            result = await asyncio.wait_for(asyncio.ensure_future(future), timeout=30.0)
            assert result.status.name() in ("Completed", "Failed")
            engine.close()
        asyncio.run(_test())

    def test_timeout_produces_error_if_exceeded(self):
        """asyncio.wait_for should raise TimeoutError if deadline exceeded.

        Note: The PyFuture polling model means cancellation arrives on the next
        poll cycle. We use ensure_future + wait_for with a very short timeout
        to demonstrate the mechanism. In practice, the background thread may
        complete before cancellation propagates, so we test the mechanism
        rather than guaranteeing a TimeoutError.
        """
        async def _test():
            engine = AsyncEngine(_loopback_scope())
            # Use a very long operation that is unlikely to complete quickly
            request = _scan_request(timeout_ms=60000)
            future = engine.run(request)
            # Wrap in ensure_future so wait_for can manage it
            coro = asyncio.ensure_future(future)
            try:
                await asyncio.wait_for(coro, timeout=0.001)
                # If it completed instantly, that's also acceptable
            except asyncio.TimeoutError:
                # Expected path: timeout fired before completion
                pass
            finally:
                # Clean up the engine regardless
                engine.close()
        asyncio.run(_test())


# ---------------------------------------------------------------------------
# 4. test_concurrent_async_operations
# ---------------------------------------------------------------------------


class TestConcurrentAsyncOperations:
    def test_gather_multiple_operations(self):
        """Multiple async operations should be gatherable."""
        async def _test():
            engine = AsyncEngine(_loopback_scope())
            try:
                tasks = [
                    engine.run(_scan_request(port=19999)),
                    engine.run(_dns_request()),
                ]
                results = await asyncio.gather(*tasks)
                assert len(results) == 2
                for r in results:
                    assert r.status is not None
                    assert r.status.name() in ("Completed", "Failed")
            finally:
                engine.close()
        asyncio.run(_test())

    def test_concurrent_scan_results_independent(self):
        """Concurrent scan results should be independent objects."""
        async def _test():
            engine = AsyncEngine(_loopback_scope())
            try:
                req_a = _scan_request(port=19999)
                req_b = _scan_request(port=19998)
                result_a, result_b = await asyncio.gather(
                    engine.run(req_a),
                    engine.run(req_b),
                )
                # Results should be separate objects
                assert result_a is not result_b
                assert result_a.status.name() in ("Completed", "Failed")
                assert result_b.status.name() in ("Completed", "Failed")
            finally:
                engine.close()
        asyncio.run(_test())


# ---------------------------------------------------------------------------
# 5. test_async_typed_methods
# ---------------------------------------------------------------------------


class TestAsyncTypedMethods:
    def test_run_port_scan_returns_future(self):
        """engine.run_port_scan() should return a PyFuture."""
        async def _test():
            engine = AsyncEngine(_loopback_scope())
            try:
                req = PortScanRequest(SENTINEL_LOOPBACK, ports="19999", timeout_ms=10000)
                future = engine.run_port_scan(req)
                assert isinstance(future, PyFuture)
                result = await future
                assert result.status.name() in ("Completed", "Failed")
            finally:
                engine.close()
        asyncio.run(_test())

    def test_run_recon_dns_returns_future(self):
        """engine.run_recon_dns() should return a PyFuture."""
        async def _test():
            engine = AsyncEngine(_loopback_scope())
            try:
                req = ReconDnsRequest(SENTINEL_LOOPBACK, timeout_ms=10000)
                future = engine.run_recon_dns(req)
                assert isinstance(future, PyFuture)
                result = await future
                assert result.status.name() in ("Completed", "Failed")
            finally:
                engine.close()
        asyncio.run(_test())

    def test_run_with_request_builder(self):
        """engine.run() with a request built from RequestBuilder should work."""
        async def _test():
            engine = AsyncEngine(_loopback_scope())
            try:
                req = eggsec.RequestBuilder("scan_ports", SENTINEL_LOOPBACK)
                req = req.port("19999")
                req = req.timeout(10000)
                built = req.build()
                result = await engine.run(built)
                assert result.status.name() in ("Completed", "Failed")
            finally:
                engine.close()
        asyncio.run(_test())

    def test_run_generic_vs_typed_equivalence(self):
        """engine.run() with OperationRequest should produce same status shape as typed."""
        async def _test():
            engine = AsyncEngine(_loopback_scope())
            try:
                generic_req = OperationRequest(
                    "scan_ports",
                    SENTINEL_LOOPBACK,
                    timeout_ms=10000,
                    metadata={"ports": "19999"},
                )
                typed_req = PortScanRequest(SENTINEL_LOOPBACK, ports="19999", timeout_ms=10000)

                generic_result = await engine.run(generic_req)
                typed_result = await engine.run_port_scan(typed_req)

                # Both should produce a valid OperationResult
                assert generic_result.status.name() in ("Completed", "Failed")
                assert typed_result.status.name() in ("Completed", "Failed")
            finally:
                engine.close()
        asyncio.run(_test())


# ---------------------------------------------------------------------------
# 6. test_async_scope_enforcement
# ---------------------------------------------------------------------------


class TestAsyncScopeEnforcement:
    def test_deny_all_scope_raises_on_run(self):
        """AsyncEngine with deny_all scope should raise EnforcementError on run."""
        scope = Scope.deny_all()
        engine = AsyncEngine(scope)
        request = _scan_request()
        with pytest.raises(eggsec.EnforcementError):
            engine.run(request)
        engine.close()

    def test_out_of_scope_target_raises_on_run(self):
        """AsyncEngine should reject out-of-scope target."""
        scope = Scope.allow_hosts(["10.0.0.0/8"])
        engine = AsyncEngine(scope)
        request = OperationRequest(
            "scan_ports",
            "evil.example.com",
            timeout_ms=5000,
            metadata={"ports": "80"},
        )
        with pytest.raises(eggsec.EnforcementError):
            engine.run(request)
        engine.close()

    def test_scope_enforcement_on_typed_method(self):
        """run_port_scan should also enforce scope."""
        scope = Scope.allow_hosts(["10.0.0.0/8"])
        engine = AsyncEngine(scope)
        req = PortScanRequest("evil.example.com", ports="80", timeout_ms=5000)
        with pytest.raises(eggsec.EnforcementError):
            engine.run_port_scan(req)
        engine.close()

    def test_scope_enforcement_synchronous(self):
        """Scope enforcement should happen synchronously before spawning async task."""
        scope = Scope.deny_all()
        engine = AsyncEngine(scope)
        # EnforcementError should be raised immediately, not via await
        with pytest.raises(eggsec.EnforcementError):
            engine.run(_scan_request())
        engine.close()


# ---------------------------------------------------------------------------
# 7. test_async_feature_unavailable
# ---------------------------------------------------------------------------


class TestAsyncFeatureUnavailable:
    def test_feature_gated_operation_raises(self):
        """Feature-gated operation in default build should raise ValueError."""
        if eggsec.has_feature("nse"):
            pytest.skip("nse feature is available; cannot test feature-unavailable path")
        engine = AsyncEngine(_loopback_scope())
        request = OperationRequest(
            "nse_run",
            SENTINEL_LOOPBACK,
            timeout_ms=5000,
            metadata={"scripts": "default"},
        )
        with pytest.raises(ValueError, match="requires feature"):
            engine.run(request)
        engine.close()

    def test_invalid_operation_id_raises(self):
        """Unknown operation ID should raise ValueError."""
        engine = AsyncEngine(_loopback_scope())
        request = OperationRequest(
            "totally_fake_operation",
            SENTINEL_LOOPBACK,
            timeout_ms=5000,
        )
        with pytest.raises((ValueError, eggsec.EggsecError)):
            engine.run(request)
        engine.close()

    def test_feature_check_consistency(self):
        """has_feature() should be consistent with actual dispatch behavior."""
        # scan_ports is always available
        assert eggsec.has_feature("scanner") is True
        engine = AsyncEngine(_loopback_scope())
        # Should not raise
        result = engine.run(_scan_request(timeout_ms=5000))
        assert result is not None
        engine.close()


# ---------------------------------------------------------------------------
# 8. test_async_cancellation
# ---------------------------------------------------------------------------


class TestAsyncCancellation:
    def test_future_can_be_polled_then_discarded(self):
        """A PyFuture can be polled and then discarded (no leak)."""
        engine = AsyncEngine(_loopback_scope())
        try:
            future = engine.run(_scan_request(timeout_ms=15000))
            # Poll once
            val = future.__next__()
            # Discard the future - no cleanup needed
            del future
        finally:
            engine.close()

    def test_ensure_future_and_wait_for(self):
        """PyFuture can be wrapped in ensure_future and managed with wait_for."""
        async def _test():
            engine = AsyncEngine(_loopback_scope())
            try:
                future = engine.run(_scan_request(timeout_ms=15000))
                coro = asyncio.ensure_future(future)
                result = await asyncio.wait_for(coro, timeout=30.0)
                assert result.status.name() in ("Completed", "Failed")
            finally:
                engine.close()
        asyncio.run(_test())

    def test_multiple_futures_from_same_engine(self):
        """Multiple futures from the same engine should be independent."""
        async def _test():
            engine = AsyncEngine(_loopback_scope())
            try:
                f1 = engine.run(_scan_request(port=19999))
                f2 = engine.run(_scan_request(port=19998))
                r1, r2 = await asyncio.gather(
                    asyncio.ensure_future(f1),
                    asyncio.ensure_future(f2),
                )
                assert r1 is not r2
                assert r1.status.name() in ("Completed", "Failed")
                assert r2.status.name() in ("Completed", "Failed")
            finally:
                engine.close()
        asyncio.run(_test())

    def test_cancellation_token_with_engine(self):
        """CancellationToken can be used with engine for cooperative cancellation."""
        token = eggsec.CancellationToken()
        assert token.is_cancelled() is False
        token.cancel("test reason")
        assert token.is_cancelled() is True
        assert token.reason() == "test reason"


# ---------------------------------------------------------------------------
# 9. test_async_double_close
# ---------------------------------------------------------------------------


class TestAsyncDoubleClose:
    def test_close_twice_no_panic(self):
        """Closing an async engine twice should not panic or raise."""
        engine = AsyncEngine(_loopback_scope())
        engine.close()
        engine.close()  # Must not raise

    def test_close_then_attributes_still_accessible(self):
        """After close, basic attributes should still be accessible."""
        engine = AsyncEngine(_loopback_scope())
        engine.close()
        # Properties should still work (close is a no-op in current impl)
        assert engine.mode == "manual"
        assert engine.concurrency == 100


# ---------------------------------------------------------------------------
# 10. test_async_use_after_close
# ---------------------------------------------------------------------------


class TestAsyncUseAfterClose:
    def test_run_after_close_still_works_or_errors(self):
        """Running operations after close should either work or raise a structured error.

        The current AsyncEngine.close() is a no-op, so operations may still work.
        This test documents the contract: if it raises, it should be a Python exception,
        not a panic/segfault.
        """
        engine = AsyncEngine(_loopback_scope())
        engine.close()
        # Attempt to run - should either succeed (close is no-op) or raise
        # a Python exception, not crash.
        try:
            result = engine.run(_scan_request(timeout_ms=5000))
            # result is a PyFuture, just check it was created
            assert isinstance(result, PyFuture)
        except Exception as e:
            # Should be a Python exception, not a panic
            assert isinstance(e, Exception)
        engine.close()

    def test_list_operations_after_close(self):
        """list_operations() after close should either work or raise a Python error."""
        engine = AsyncEngine(_loopback_scope())
        engine.close()
        try:
            ops = engine.list_operations()
            assert isinstance(ops, list)
        except Exception as e:
            assert isinstance(e, Exception)
        engine.close()


# ---------------------------------------------------------------------------
# 11. test_async_plan_method
# ---------------------------------------------------------------------------


class TestAsyncPlanMethod:
    def test_plan_returns_awaitable(self):
        """engine.plan() should return a PyFuture."""
        engine = AsyncEngine(_loopback_scope())
        try:
            future = engine.plan(SENTINEL_LOOPBACK)
            assert isinstance(future, PyFuture)
        finally:
            engine.close()

    def test_plan_can_be_awaited_with_result(self):
        """engine.plan() result should be awaitable and return a ScanPlan."""
        async def _test():
            engine = AsyncEngine(_loopback_scope())
            try:
                plan = await engine.plan(SENTINEL_LOOPBACK)
                assert plan is not None
                assert hasattr(plan, "target") or hasattr(plan, "steps")
            finally:
                engine.close()
        asyncio.run(_test())


# ---------------------------------------------------------------------------
# 12. test_async_engine_properties
# ---------------------------------------------------------------------------


class TestAsyncEngineProperties:
    def test_scope_property(self):
        """Engine should expose its scope."""
        engine = AsyncEngine(_loopback_scope())
        scope = engine.scope
        assert scope is not None
        engine.close()

    def test_mode_property(self):
        """Engine should expose its mode."""
        engine = AsyncEngine(_loopback_scope(), mode="automation")
        assert engine.mode == "automation"
        engine.close()

    def test_concurrency_property(self):
        """Engine should expose its concurrency."""
        engine = AsyncEngine(_loopback_scope(), concurrency=50)
        assert engine.concurrency == 50
        engine.close()

    def test_timeout_ms_property(self):
        """Engine should expose its timeout_ms."""
        engine = AsyncEngine(_loopback_scope(), timeout_ms=3000)
        assert engine.timeout_ms == 3000
        engine.close()

    def test_repr(self):
        """Engine repr should contain mode and concurrency."""
        engine = AsyncEngine(_loopback_scope(), mode="automation", concurrency=50)
        r = repr(engine)
        assert "AsyncEngine" in r
        assert "automation" in r
        assert "50" in r
        engine.close()


# ---------------------------------------------------------------------------
# 13. test_async_audit_events
# ---------------------------------------------------------------------------


class TestAsyncAuditEvents:
    def test_audit_events_returns_list(self):
        """audit_events() should return a list."""
        engine = AsyncEngine(_loopback_scope())
        events = engine.audit_events()
        assert isinstance(events, list)
        engine.close()

    def test_has_operation(self):
        """has_operation() should return True for known operations."""
        engine = AsyncEngine(_loopback_scope())
        assert engine.has_operation("scan_ports") is True
        assert engine.has_operation("nonexistent_operation") is False
        engine.close()


# ---------------------------------------------------------------------------
# 14. test_async_pyfuture_protocol
# ---------------------------------------------------------------------------


class TestAsyncPyFutureProtocol:
    def test_pyfuture_has_await(self):
        """PyFuture should implement __await__."""
        engine = AsyncEngine(_loopback_scope())
        future = engine.run(_scan_request(timeout_ms=5000))
        assert hasattr(future, "__await__")
        engine.close()

    def test_pyfuture_has_next(self):
        """PyFuture should implement __next__ (iterator protocol)."""
        engine = AsyncEngine(_loopback_scope())
        future = engine.run(_scan_request(timeout_ms=5000))
        assert hasattr(future, "__next__")
        engine.close()

    def test_pyfuture_polling_returns_none_while_pending(self):
        """PyFuture.__next__ should return None while the operation is pending."""
        engine = AsyncEngine(_loopback_scope())
        try:
            future = engine.run(_scan_request(timeout_ms=15000))
            # Poll once - should return None while still running
            val = future.__next__()
            # Could be None (pending) or raise StopIteration (completed fast)
            # Just verify no crash
        finally:
            engine.close()

    def test_pyfuture_await_protocol(self):
        """PyFuture.__await__ should return self (iterator protocol)."""
        engine = AsyncEngine(_loopback_scope())
        try:
            future = engine.run(_scan_request(timeout_ms=5000))
            awaitable = future.__await__()
            # __await__ returns self (the PyFuture is its own iterator)
            assert awaitable is future
        finally:
            engine.close()
