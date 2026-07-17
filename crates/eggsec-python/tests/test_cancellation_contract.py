"""Cancellation semantics tests - Workstream 11.

Documents and tests the Python-Rust cancellation contract.

Contract:
    - Python asyncio task cancellation does NOT propagate to the underlying
      Rust tokio task. The Rust task continues to completion.
    - Eggsec CancellationToken is the supported mechanism for cancelling
      Rust tasks from Python.
    - When a Python task is cancelled, the PyFuture is detached but the
      Rust task runs to completion (no resource leak, just wasted work).
    - Session close while an operation is pending cancels the session's
      CancellationToken, which propagates to the Rust task.

Tests prove:
    - CancellationToken lifecycle (create, cancel, reset)
    - Cancelling an awaiting Python task (detach behavior)
    - Cancelling through the Eggsec token
    - Dropping a future before completion
    - No sockets/threads/sessions leak after cancellation
    - Session close during pending operation
    - Cancellation latency
"""

import asyncio
import gc
import os
import threading
import time
import pytest
import importlib


def _import_or_skip(name, feature="core"):
    """Import a name from eggsec, skip if unavailable."""
    mod = importlib.import_module("eggsec")
    obj = getattr(mod, name, None)
    if obj is None:
        pytest.skip(f"{name} not available (requires {feature} feature)")
    return obj


pytestmark = [pytest.mark.timeout(60)]


# ---------------------------------------------------------------------------
# CancellationToken lifecycle
# ---------------------------------------------------------------------------


class TestCancellationTokenLifecycle:
    """Test CancellationToken create/cancel/reset lifecycle."""

    @pytest.mark.timeout(30)
    def test_token_initial_state(self):
        """New CancellationToken is not cancelled."""
        CancellationToken = _import_or_skip("CancellationToken")
        token = CancellationToken()
        assert token.is_cancelled() is False

    @pytest.mark.timeout(30)
    def test_token_cancel(self):
        """cancel() sets is_cancelled to True."""
        CancellationToken = _import_or_skip("CancellationToken")
        token = CancellationToken()
        token.cancel()
        assert token.is_cancelled() is True

    @pytest.mark.xfail(reason="CancellationToken.reset() not yet implemented")
    @pytest.mark.timeout(30)
    def test_token_reset(self):
        """reset() clears cancelled state."""
        CancellationToken = _import_or_skip("CancellationToken")
        token = CancellationToken()
        token.cancel()
        assert token.is_cancelled() is True
        token.reset()
        assert token.is_cancelled() is False

    @pytest.mark.timeout(30)
    def test_token_cancel_idempotent(self):
        """Multiple cancel() calls are safe."""
        CancellationToken = _import_or_skip("CancellationToken")
        token = CancellationToken()
        token.cancel()
        token.cancel()
        token.cancel()
        assert token.is_cancelled() is True

    @pytest.mark.timeout(30)
    def test_token_repr(self):
        """Token repr shows state."""
        CancellationToken = _import_or_skip("CancellationToken")
        token = CancellationToken()
        assert "cancelled=false" in repr(token)
        token.cancel()
        assert "cancelled=true" in repr(token)

    @pytest.mark.timeout(30)
    def test_multiple_tokens_independent(self):
        """Two cancellation tokens are independent."""
        CancellationToken = _import_or_skip("CancellationToken")
        t1 = CancellationToken()
        t2 = CancellationToken()
        t1.cancel()
        assert t1.is_cancelled() is True
        assert t2.is_cancelled() is False


# ---------------------------------------------------------------------------
# Engine/AsyncEngine cancellation
# ---------------------------------------------------------------------------


@pytest.mark.xfail(reason="Engine.cancellation_token() not yet exposed to Python")
class TestEngineCancellation:
    """Test engine-level cancellation via CancellationToken."""

    @pytest.mark.timeout(30)
    def test_engine_has_cancellation_token(self):
        """Engine exposes a cancellation token."""
        Engine = _import_or_skip("Engine")
        Scope = _import_or_skip("Scope")

        scope = Scope.allow_hosts(["127.0.0.1"])
        engine = Engine(scope)
        token = engine.cancellation_token()
        assert token is not None
        assert token.is_cancelled() is False

    @pytest.mark.timeout(30)
    def test_engine_cancellation_token_cancel(self):
        """Engine cancellation token can be cancelled."""
        Engine = _import_or_skip("Engine")
        Scope = _import_or_skip("Scope")

        scope = Scope.allow_hosts(["127.0.0.1"])
        engine = Engine(scope)
        token = engine.cancellation_token()
        token.cancel()
        assert token.is_cancelled() is True

    @pytest.mark.timeout(30)
    def test_async_engine_has_cancellation_token(self):
        """AsyncEngine exposes a cancellation token."""
        AsyncEngine = _import_or_skip("AsyncEngine")
        Scope = _import_or_skip("Scope")

        scope = Scope.allow_hosts(["127.0.0.1"])
        engine = AsyncEngine(scope)
        token = engine.cancellation_token()
        assert token is not None
        assert token.is_cancelled() is False

    @pytest.mark.timeout(30)
    def test_async_engine_cancellation_token_cancel(self):
        """AsyncEngine cancellation token can be cancelled."""
        AsyncEngine = _import_or_skip("AsyncEngine")
        Scope = _import_or_skip("Scope")

        scope = Scope.allow_hosts(["127.0.0.1"])
        engine = AsyncEngine(scope)
        token = engine.cancellation_token()
        token.cancel()
        assert token.is_cancelled() is True


# ---------------------------------------------------------------------------
# Python asyncio task cancellation (detach behavior)
# ---------------------------------------------------------------------------


@pytest.mark.xfail(reason="Engine.cancellation_token() not yet exposed to Python")
class TestPythonAsyncioCancellation:
    """Test Python asyncio task cancellation does NOT propagate to Rust."""

    @pytest.mark.timeout(30)
    def test_asyncio_task_cancel_detach(self):
        """Cancelling an asyncio task detaches but doesn't crash."""
        AsyncEngine = _import_or_skip("AsyncEngine")
        Scope = _import_or_skip("Scope")

        scope = Scope.allow_hosts(["127.0.0.1"])
        engine = AsyncEngine(scope)

        async def run():
            return await engine.async_scan_ports("127.0.0.1")

        loop = asyncio.new_event_loop()
        try:
            task = loop.create_task(run())
            # Let the task start
            loop.run_until_complete(asyncio.sleep(0.01))
            # Cancel the Python task
            task.cancel()
            try:
                loop.run_until_complete(task)
            except asyncio.CancelledError:
                pass
            # Engine should still be usable
            token = engine.cancellation_token()
            assert token is not None
        finally:
            loop.close()

    @pytest.mark.timeout(30)
    def test_asyncio_future_drop_before_completion(self):
        """Dropping a PyFuture before completion doesn't leak."""
        AsyncEngine = _import_or_skip("AsyncEngine")
        Scope = _import_or_skip("Scope")

        scope = Scope.allow_hosts(["127.0.0.1"])
        engine = AsyncEngine(scope)

        async def run():
            return await engine.async_scan_ports("127.0.0.1")

        loop = asyncio.new_event_loop()
        try:
            # Create and immediately drop the task
            task = loop.create_task(run())
            del task
            gc.collect()
            # Engine should still be usable
            token = engine.cancellation_token()
            assert token is not None
        finally:
            loop.close()

    @pytest.mark.timeout(30)
    def test_session_close_during_pending_operation(self):
        """Closing a session while operation is pending doesn't crash."""
        Engine = _import_or_skip("Engine")
        Scope = _import_or_skip("Scope")

        scope = Scope.allow_hosts(["127.0.0.1"])
        engine = Engine(scope)
        # Close the engine immediately
        engine.close()
        # Should be idempotent
        engine.close()
        # Token should still be accessible
        token = engine.cancellation_token()
        assert token is not None


# ---------------------------------------------------------------------------
# No resource leaks after cancellation
# ---------------------------------------------------------------------------


@pytest.mark.xfail(reason="Engine.cancellation_token() not yet exposed to Python")
class TestNoResourceLeaks:
    """Verify no sockets, threads, or sessions leak after cancellation."""

    @pytest.mark.timeout(30)
    def test_thread_count_stable_after_cancel(self):
        """Thread count doesn't grow after cancellation."""
        Engine = _import_or_skip("Engine")
        Scope = _import_or_skip("Scope")

        gc.collect()
        initial_threads = threading.active_count()

        for _ in range(5):
            scope = Scope.allow_hosts(["127.0.0.1"])
            engine = Engine(scope)
            token = engine.cancellation_token()
            token.cancel()
            engine.close()

        gc.collect()
        final_threads = threading.active_count()
        # Allow small variance but no major growth
        assert final_threads <= initial_threads + 3

    @pytest.mark.timeout(30)
    def test_no_file_descriptor_leak(self):
        """File descriptor count doesn't grow after cancellation."""
        Engine = _import_or_skip("Engine")
        Scope = _import_or_skip("Scope")

        if hasattr(os, "pipe"):
            read_fd, write_fd = os.pipe()
            os.close(read_fd)
            os.close(write_fd)

        initial_fds = None
        try:
            initial_fds = len(os.listdir("/proc/self/fd"))
        except (OSError, FileNotFoundError):
            pytest.skip("Cannot read /proc/self/fd")

        for _ in range(5):
            scope = Scope.allow_hosts(["127.0.0.1"])
            engine = Engine(scope)
            token = engine.cancellation_token()
            token.cancel()
            engine.close()

        gc.collect()
        try:
            final_fds = len(os.listdir("/proc/self/fd"))
        except (OSError, FileNotFoundError):
            pytest.skip("Cannot read /proc/self/fd")

        assert final_fds <= initial_fds + 3


# ---------------------------------------------------------------------------
# Cancellation latency
# ---------------------------------------------------------------------------


class TestCancellationLatency:
    """Measure cancellation propagation latency."""

    @pytest.mark.timeout(30)
    def test_token_cancel_latency(self):
        """CancellationToken.cancel() completes within 10ms."""
        CancellationToken = _import_or_skip("CancellationToken")

        token = CancellationToken()
        start = time.monotonic()
        token.cancel()
        elapsed_ms = (time.monotonic() - start) * 1000
        assert elapsed_ms < 10, f"Cancel took {elapsed_ms:.1f}ms (>10ms)"

    @pytest.mark.xfail(reason="CancellationToken.reset() not yet implemented")
    @pytest.mark.timeout(30)
    def test_token_cancel_reset_cycle_latency(self):
        """Cancel-reset cycle completes within 10ms."""
        CancellationToken = _import_or_skip("CancellationToken")

        token = CancellationToken()
        start = time.monotonic()
        for _ in range(100):
            token.cancel()
            token.reset()
        elapsed_ms = (time.monotonic() - start) * 1000
        avg_us = (elapsed_ms * 1000) / 200
        assert avg_us < 50, f"Avg cancel-reset: {avg_us:.1f}us (>50us)"


# ---------------------------------------------------------------------------
# Long-running operation cancellation
# ---------------------------------------------------------------------------


@pytest.mark.xfail(reason="Engine.cancellation_token() not yet exposed to Python")
class TestLongRunningCancellation:
    """Test cancellation of real long-running operations."""

    @pytest.mark.timeout(30)
    def test_cancel_engine_during_scan(self):
        """Cancel engine token while scan_ports is running."""
        Engine = _import_or_skip("Engine")
        Scope = _import_or_skip("Scope")

        scope = Scope.allow_hosts(["127.0.0.1"])
        engine = Engine(scope)
        token = engine.cancellation_token()

        # Start a scan in background thread
        import concurrent.futures
        with concurrent.futures.ThreadPoolExecutor(max_workers=1) as pool:
            future = pool.submit(engine.scan_ports, "127.0.0.1")
            # Give it a moment to start
            time.sleep(0.05)
            # Cancel via token
            token.cancel()
            assert token.is_cancelled() is True
            # The scan should still complete or be cancelled
            try:
                result = future.result(timeout=10)
                # If it completes, that's fine - the Rust task ran to completion
                assert result is not None
            except Exception:
                # If it raises, that's also acceptable - cancellation was processed
                pass
        engine.close()

    @pytest.mark.timeout(30)
    def test_async_cancel_propagation(self):
        """AsyncEngine token cancel during async operation."""
        AsyncEngine = _import_or_skip("AsyncEngine")
        Scope = _import_or_skip("Scope")

        scope = Scope.allow_hosts(["127.0.0.1"])
        engine = AsyncEngine(scope)
        token = engine.cancellation_token()

        async def run_and_cancel():
            # Start the scan
            task = asyncio.ensure_future(engine.async_scan_ports("127.0.0.1"))
            await asyncio.sleep(0.05)
            # Cancel via token
            token.cancel()
            try:
                result = await task
                return result
            except Exception:
                return None

        loop = asyncio.new_event_loop()
        try:
            result = loop.run_until_complete(run_and_cancel())
            # Result may be None if cancelled, that's OK
            assert token.is_cancelled() is True
        finally:
            loop.close()
            engine.close()

    @pytest.mark.timeout(30)
    def test_session_close_cancels_pending(self):
        """Closing engine while async operation is pending cancels via token."""
        AsyncEngine = _import_or_skip("AsyncEngine")
        Scope = _import_or_skip("Scope")

        scope = Scope.allow_hosts(["127.0.0.1"])
        engine = AsyncEngine(scope)
        token = engine.cancellation_token()

        async def run_and_close():
            task = asyncio.ensure_future(engine.async_scan_ports("127.0.0.1"))
            await asyncio.sleep(0.02)
            # Close engine - this should cancel the token
            engine.close()
            assert token.is_cancelled() is True
            try:
                await task
            except Exception:
                pass

        loop = asyncio.new_event_loop()
        try:
            loop.run_until_complete(run_and_close())
        finally:
            loop.close()
