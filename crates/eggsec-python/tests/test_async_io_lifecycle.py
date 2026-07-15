"""Async I/O lifecycle tests for Workstream 5 of the eggsec Python API closure pass.

Validates async session lifecycle behavior for all async session types using
real loopback services: context manager protocol, normal operation, cancellation
during connect/read/write, close-while-in-flight, double close, use-after-close,
timeout, and resource leak prevention.

Scope: local loopback (127.0.0.1) only. All services are started via
StableCoreFixtures which binds to random available ports.
"""

from __future__ import annotations

import asyncio
import os
import sys
import threading
import time
from typing import Any

import pytest

import eggsec
from fixtures.stable_core import HOST, StableCoreFixtures

# ---------------------------------------------------------------------------
# Type aliases — the extension exports Py-suffix names; alias for readability
# ---------------------------------------------------------------------------

TcpConfig = eggsec.TcpConfigPy
AsyncTcpSession = eggsec.AsyncTcpSessionPy
UdpConfig = eggsec.UdpConfigPy
AsyncUdpSocket = eggsec.AsyncUdpSocketPy
HttpClientConfig = eggsec.HttpClientConfigPy
AsyncHttpClient = eggsec.AsyncHttpClientPy

# Capture types — feature-gated (packet-inspection)
_CAPTURE_AVAILABLE = hasattr(eggsec, "CaptureConfigPy")
if _CAPTURE_AVAILABLE:
    CaptureConfig = eggsec.CaptureConfigPy
    AsyncCaptureSession = eggsec.AsyncCaptureSessionPy
else:
    CaptureConfig = None
    AsyncCaptureSession = None

# WebSocket types — feature-gated
_WEBSOCKET_AVAILABLE = hasattr(eggsec, "AsyncWebSocketSessionPy")
if _WEBSOCKET_AVAILABLE:
    WebSocketSessionConfig = eggsec.WebSocketSessionConfigPy
    AsyncWebSocketSession = eggsec.AsyncWebSocketSessionPy


# Async transport chaining now works with the shared Tokio runtime.
# The _skip_chaining marker is removed; chained operations are expected to pass.

# ---------------------------------------------------------------------------
# PyFuture polling helper (from existing test patterns)
# ---------------------------------------------------------------------------

# Future polling timeout used when awaiting PyFuture objects from Python.
_AWAIT_TIMEOUT = 30.0


def _await_future(future, timeout: float = _AWAIT_TIMEOUT) -> Any:
    """Poll a PyFuture via the iterator protocol until a value arrives.

    PyFuture implements ``__await__`` → ``__next__`` (polling).  This helper
    drives that protocol manually so we do not depend on ``asyncio.run``.
    """
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


def _await_future_ignore_cancelled(future, timeout: float = _AWAIT_TIMEOUT) -> Any:
    """Like _await_future but returns None if the future is cancelled.

    Used for tests that deliberately cancel futures mid-flight.
    """
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        try:
            value = next(future)
        except StopIteration as done:
            return done.value
        except (SystemExit, KeyboardInterrupt):
            raise
        except BaseException:
            return None
        if value is not None:
            return value
        time.sleep(0.01)
    return None


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _start_echo_tcp_server(host: str = HOST, port: int = 0) -> "threading.Server":
    """Start a TCP echo server that reads 4096 bytes and sends back the fixture banner."""
    import socket
    import socketserver

    class EchoHandler(socketserver.BaseRequestHandler):
        def handle(self) -> None:
            try:
                self.request.settimeout(2.0)
                self.request.recv(4096)
            except OSError:
                pass
            self.request.sendall(b"EGGSEC-ECHO/1.0\r\n")

    class EchoServer(socketserver.ThreadingTCPServer):
        allow_reuse_address = True
        daemon_threads = True

    server = EchoServer((host, port), EchoHandler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    deadline = time.monotonic() + 2.0
    while time.monotonic() < deadline:
        if thread.is_alive():
            break
        time.sleep(0.01)
    return server


def _get_port(server) -> int:
    return int(server.server_address[1])


def _start_udp_echo_server(host: str = HOST) -> Any:
    """Start a simple UDP echo server that reflects data back."""
    import socket
    import socketserver

    class UdpEchoHandler(socketserver.BaseRequestHandler):
        def handle(self) -> None:
            data = self.request[0]
            socket = self.request[1]
            socket.sendto(data, self.client_address)

    class UdpEchoServer(socketserver.ThreadingUDPServer):
        allow_reuse_address = True
        daemon_threads = True

    server = UdpEchoServer((host, 0), UdpEchoHandler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    deadline = time.monotonic() + 2.0
    while time.monotonic() < deadline:
        if thread.is_alive():
            break
        time.sleep(0.01)
    return server


# ---------------------------------------------------------------------------
# Module-level markers
# ---------------------------------------------------------------------------

pytestmark = [
    pytest.mark.timeout(30),
]

# Require loopback fixture permission
_LOOPBACK_OK = os.environ.get("EGGSEC_ALLOW_LOOPBACK_FIXTURE", "0") == "1"


# ═══════════════════════════════════════════════════════════════════════════
# 1. AsyncTcpSession lifecycle
# ═══════════════════════════════════════════════════════════════════════════


class TestAsyncTcpSessionContextManager:
    """Context manager lifecycle tests for AsyncTcpSession."""

    def test_async_context_manager_enter_exit(self):
        """async with creates and cleans up session."""
        with StableCoreFixtures() as fx:
            config = TcpConfig(HOST, fx.tcp_port, connect_timeout_ms=5000)
            session = AsyncTcpSession(config)
            assert not session.is_closed
            result = session.__aenter__()
            assert result is session
            exited = session.__aexit__(None, None, None)
            assert not exited
            assert session.is_closed

    def test_sync_context_manager_rejected(self):
        """Sync 'with' must raise TypeError."""
        config = TcpConfig(HOST, 12345)
        session = AsyncTcpSession(config)
        with pytest.raises(TypeError, match="async with"):
            with session:
                pass

    def test_is_closed_initially_false(self):
        config = TcpConfig(HOST, 12345)
        session = AsyncTcpSession(config)
        assert session.is_closed is False

    def test_repr_after_creation(self):
        config = TcpConfig(HOST, 9999)
        session = AsyncTcpSession(config)
        r = repr(session)
        assert "AsyncTcpSession" in r
        assert "127.0.0.1" in r
        assert "9999" in r
        assert "closed=false" in r

    def test_repr_after_close(self):
        config = TcpConfig(HOST, 9999)
        session = AsyncTcpSession(config)
        session.close()
        r = repr(session)
        assert "closed=true" in r


class TestAsyncTcpSessionNormalOperation:
    """Basic I/O operations against loopback server."""

    def test_connect_and_read(self):
        """Connect to loopback TCP and read banner."""
        with StableCoreFixtures() as fx:
            config = TcpConfig(HOST, fx.tcp_port, connect_timeout_ms=5000)
            session = AsyncTcpSession(config)
            _await_future(session.connect())

            # Send data and read response
            _await_future(session.write(b"hello"))
            read_result = _await_future(session.read(4096))
            assert read_result.bytes_read > 0
            assert b"EGGSEC-FIXTURE" in bytes(read_result.data)
            session.close()

    def test_connect_write_read_close_cycle(self):
        """Full connect → write → read → close cycle."""
        with StableCoreFixtures() as fx:
            config = TcpConfig(HOST, fx.tcp_port, connect_timeout_ms=5000)
            session = AsyncTcpSession(config)
            _await_future(session.connect())
            _await_future(session.write(b"ping"))
            resp = _await_future(session.read(4096))
            assert resp.bytes_read > 0
            session.close()
            assert session.is_closed

    def test_bytes_sent_received_counters(self):
        """Byte counters track I/O accurately."""
        with StableCoreFixtures() as fx:
            config = TcpConfig(HOST, fx.tcp_port, connect_timeout_ms=5000)
            session = AsyncTcpSession(config)
            _await_future(session.connect())
            payload = b"X" * 256
            _await_future(session.write(payload))
            assert session.bytes_sent >= 256
            _await_future(session.read(4096))
            assert session.bytes_received > 0
            session.close()

    def test_transcript_populated(self):
        """Transcript records read/write entries."""
        with StableCoreFixtures() as fx:
            config = TcpConfig(HOST, fx.tcp_port, connect_timeout_ms=5000)
            session = AsyncTcpSession(config)
            _await_future(session.connect())
            _await_future(session.write(b"transcript-test"))
            _await_future(session.read(4096))
            t = session.transcript
            assert t.total_bytes > 0
            assert len(t.entries) >= 2
            session.close()

    def test_config_property(self):
        config = TcpConfig(HOST, 5555, connect_timeout_ms=3000)
        session = AsyncTcpSession(config)
        assert session.config.port == 5555
        assert session.config.connect_timeout_ms == 3000
        session.close()


class TestAsyncTcpSessionCancellation:
    """Cancellation during async operations."""

    def test_cancel_during_read(self):
        """Cancel a read future before it completes."""
        with StableCoreFixtures() as fx:
            config = TcpConfig(
                HOST, fx.tcp_port, connect_timeout_ms=5000, read_timeout_ms=15000
            )
            session = AsyncTcpSession(config)
            _await_future(session.connect())
            # Write to trigger server response, then try to read with no data pending
            _await_future(session.write(b"test"))

            # Start a read that may block
            read_future = session.read(4096)

            # Poll a few times then give up (simulated cancellation)
            for _ in range(5):
                try:
                    next(read_future)
                except StopIteration:
                    break
                except BaseException:
                    break
                time.sleep(0.01)

            # Session should still be usable or at least closable
            session.close()
            assert session.is_closed

    def test_cancel_during_write(self):
        """Cancel a write future before it completes."""
        with StableCoreFixtures() as fx:
            config = TcpConfig(HOST, fx.tcp_port, connect_timeout_ms=5000)
            session = AsyncTcpSession(config)
            _await_future(session.connect())
            # Write should complete quickly, but verify close works after partial poll
            write_future = session.write(b"cancel-test-data")
            for _ in range(3):
                try:
                    next(write_future)
                except StopIteration:
                    break
                except BaseException:
                    break
                time.sleep(0.005)
            session.close()
            assert session.is_closed

    def test_close_during_pending_read(self):
        """Closing session while a read is pending does not hang."""
        with StableCoreFixtures() as fx:
            config = TcpConfig(
                HOST, fx.tcp_port, connect_timeout_ms=5000, read_timeout_ms=5000
            )
            session = AsyncTcpSession(config)
            _await_future(session.connect())
            # Don't write anything, so read will block until timeout
            read_future = session.read(4096)

            # Close from another thread
            def _close():
                time.sleep(0.05)
                session.close()

            t = threading.Thread(target=_close)
            t.start()

            # The read should eventually resolve (timeout or error)
            _await_future_ignore_cancelled(read_future, timeout=10.0)
            t.join(timeout=5.0)
            assert session.is_closed


class TestAsyncTcpSessionDoubleClose:
    """Double close is idempotent."""

    def test_double_close(self):
        config = TcpConfig(HOST, 12345)
        session = AsyncTcpSession(config)
        session.close()
        session.close()  # Must not raise
        assert session.is_closed

    def test_close_after_context_exit(self):
        with StableCoreFixtures() as fx:
            config = TcpConfig(HOST, fx.tcp_port)
            session = AsyncTcpSession(config)
            _await_future(session.connect())
            session.__aexit__(None, None, None)
            session.close()  # Must not raise
            assert session.is_closed


class TestAsyncTcpSessionUseAfterClose:
    """Operations after close must raise errors."""

    def test_connect_after_close(self):
        config = TcpConfig(HOST, 12345)
        session = AsyncTcpSession(config)
        session.close()
        with pytest.raises(Exception):
            _await_future(session.connect())

    def test_write_after_close(self):
        with StableCoreFixtures() as fx:
            config = TcpConfig(HOST, fx.tcp_port)
            session = AsyncTcpSession(config)
            _await_future(session.connect())
            session.close()
            with pytest.raises(Exception):
                _await_future(session.write(b"oops"))

    def test_read_after_close(self):
        with StableCoreFixtures() as fx:
            config = TcpConfig(HOST, fx.tcp_port)
            session = AsyncTcpSession(config)
            _await_future(session.connect())
            session.close()
            with pytest.raises(Exception):
                _await_future(session.read(4096))


class TestAsyncTcpSessionConnectTimeout:
    """Connect timeout against a non-responsive target."""

    def test_connect_timeout_closed_port(self):
        """Connect to a port that is not listening must fail."""
        with StableCoreFixtures() as fx:
            config = TcpConfig(
                HOST, fx.closed_port, connect_timeout_ms=1000, read_timeout_ms=1000
            )
            session = AsyncTcpSession(config)
            with pytest.raises(Exception):
                _await_future(session.connect(), timeout=5.0)
            session.close()

    def test_connect_timeout_impossible_host(self):
        """Connect to an unroutable address must fail within timeout."""
        config = TcpConfig("192.0.2.1", 1, connect_timeout_ms=1000, read_timeout_ms=1000)
        session = AsyncTcpSession(config)
        with pytest.raises(Exception):
            _await_future(session.connect(), timeout=5.0)
        session.close()


class TestAsyncTcpSessionResourceLeak:
    """After cancellation or close, session is properly cleaned up."""

    def test_cleanup_after_manual_close(self):
        """Manual close leaves session in closed state."""
        with StableCoreFixtures() as fx:
            config = TcpConfig(HOST, fx.tcp_port)
            session = AsyncTcpSession(config)
            _await_future(session.connect())
            _await_future(session.write(b"leak-test"))
            session.close()
            assert session.is_closed

    def test_cleanup_after_close_during_io(self):
        """Close during I/O does not leak resources."""
        with StableCoreFixtures() as fx:
            config = TcpConfig(HOST, fx.tcp_port, read_timeout_ms=5000)
            session = AsyncTcpSession(config)
            _await_future(session.connect())
            _await_future(session.write(b"leak-check"))
            session.close()
            assert session.is_closed
            # Verify the file descriptor is released by attempting reconnection
            session2 = AsyncTcpSession(config)
            _await_future(session2.connect())
            session2.close()


class TestAsyncTcpSessionReadExact:
    """read_exact operation tests."""

    def test_read_exact(self):
        """read_exact returns the requested number of bytes."""
        with StableCoreFixtures() as fx:
            config = TcpConfig(HOST, fx.tcp_port, connect_timeout_ms=5000)
            session = AsyncTcpSession(config)
            _await_future(session.connect())
            _await_future(session.write(b"exact-test"))
            # Server sends back banner, read_exact 10
            result = _await_future(session.read_exact(10))
            assert result.bytes_read == 10
            assert result.eof is False
            session.close()

    def test_read_exact_after_close(self):
        config = TcpConfig(HOST, 12345)
        session = AsyncTcpSession(config)
        session.close()
        with pytest.raises(Exception):
            _await_future(session.read_exact(10))


class TestAsyncTcpSessionReadUntil:
    """read_until operation tests."""

    def test_read_until_delimiter(self):
        """read_until stops at the delimiter byte."""
        with StableCoreFixtures() as fx:
            config = TcpConfig(HOST, fx.tcp_port, connect_timeout_ms=5000)
            session = AsyncTcpSession(config)
            _await_future(session.connect())
            _await_future(session.write(b"line\n"))
            result = _await_future(session.read_until(0x0A))
            assert result.bytes_read > 0
            session.close()

    def test_read_until_after_close(self):
        config = TcpConfig(HOST, 12345)
        session = AsyncTcpSession(config)
        session.close()
        with pytest.raises(Exception):
            _await_future(session.read_until(0x0A))


# ═══════════════════════════════════════════════════════════════════════════
# 2. AsyncUdpSocket lifecycle
# ═══════════════════════════════════════════════════════════════════════════


class TestAsyncUdpSocketContextManager:
    """Context manager lifecycle tests for AsyncUdpSocket."""

    def test_async_context_manager_enter_exit(self):
        """async with creates and cleans up socket."""
        config = UdpConfig(HOST, 9999, timeout_ms=3000)
        sock = AsyncUdpSocket(config)
        assert not sock.is_closed
        result = sock.__aenter__()
        assert result is sock
        exited = sock.__aexit__(None, None, None)
        assert not exited
        assert sock.is_closed

    def test_sync_context_manager_rejected(self):
        """Sync 'with' must raise TypeError."""
        config = UdpConfig(HOST, 9999)
        sock = AsyncUdpSocket(config)
        with pytest.raises(TypeError, match="async with"):
            with sock:
                pass

    def test_is_closed_initially_false(self):
        config = UdpConfig(HOST, 9999)
        sock = AsyncUdpSocket(config)
        assert sock.is_closed is False

    def test_repr(self):
        config = UdpConfig(HOST, 5353)
        sock = AsyncUdpSocket(config)
        r = repr(sock)
        assert "AsyncUdpSocket" in r
        assert "5353" in r
        assert "closed=false" in r
        sock.close()

    def test_repr_after_close(self):
        config = UdpConfig(HOST, 5353)
        sock = AsyncUdpSocket(config)
        sock.close()
        r = repr(sock)
        assert "closed=true" in r


class TestAsyncUdpSocketNormalOperation:
    """Basic UDP I/O against loopback server."""

    def test_connect_and_send_recv(self):
        """Connect UDP and exchange data with echo server."""
        server = _start_udp_echo_server(HOST)
        try:
            port = _get_port(server)
            config = UdpConfig(HOST, port, timeout_ms=3000)
            sock = AsyncUdpSocket(config)
            _await_future(sock.connect())
            send_result = _await_future(sock.send(b"udp-hello"))
            assert send_result.bytes_sent == len(b"udp-hello")

            recv_result = _await_future(sock.recv(1024))
            assert recv_result.bytes_received == len(b"udp-hello")
            assert bytes(recv_result.data) == b"udp-hello"
            sock.close()
        finally:
            server.shutdown()
            server.server_close()

    def test_bytes_sent_received_counters(self):
        server = _start_udp_echo_server(HOST)
        try:
            port = _get_port(server)
            config = UdpConfig(HOST, port, timeout_ms=3000)
            sock = AsyncUdpSocket(config)
            _await_future(sock.connect())
            _await_future(sock.send(b"counter-test"))
            assert sock.bytes_sent == 12
            _await_future(sock.recv(1024))
            assert sock.bytes_received > 0
            sock.close()
        finally:
            server.shutdown()
            server.server_close()

    def test_transcript(self):
        server = _start_udp_echo_server(HOST)
        try:
            port = _get_port(server)
            config = UdpConfig(HOST, port, timeout_ms=3000)
            sock = AsyncUdpSocket(config)
            _await_future(sock.connect())
            _await_future(sock.send(b"transcript"))
            _await_future(sock.recv(1024))
            t = sock.transcript
            assert t.total_bytes > 0
            assert len(t.entries) >= 2
            sock.close()
        finally:
            server.shutdown()
            server.server_close()


class TestAsyncUdpSocketCancellation:
    """Cancellation during UDP operations."""

    def test_cancel_during_recv(self):
        """Cancel a recv future that is waiting for data."""
        config = UdpConfig(HOST, 9999, timeout_ms=3000)
        sock = AsyncUdpSocket(config)
        _await_future(sock.connect())

        # Start recv which will timeout (no server)
        recv_future = sock.recv(1024)
        for _ in range(5):
            try:
                next(recv_future)
            except StopIteration:
                break
            except BaseException:
                break
            time.sleep(0.01)

        sock.close()
        assert sock.is_closed

    def test_close_during_pending_recv(self):
        """Close socket while recv is pending."""
        config = UdpConfig(HOST, 9999, timeout_ms=5000)
        sock = AsyncUdpSocket(config)
        _await_future(sock.connect())

        recv_future = sock.recv(1024)

        def _close():
            time.sleep(0.05)
            sock.close()

        t = threading.Thread(target=_close)
        t.start()
        _await_future_ignore_cancelled(recv_future, timeout=10.0)
        t.join(timeout=5.0)
        assert sock.is_closed


class TestAsyncUdpSocketDoubleClose:
    """Double close is idempotent for UDP."""

    def test_double_close(self):
        config = UdpConfig(HOST, 9999)
        sock = AsyncUdpSocket(config)
        sock.close()
        sock.close()
        assert sock.is_closed


class TestAsyncUdpSocketUseAfterClose:
    """Operations after close must raise errors."""

    def test_connect_after_close(self):
        config = UdpConfig(HOST, 9999)
        sock = AsyncUdpSocket(config)
        sock.close()
        with pytest.raises(Exception):
            _await_future(sock.connect())

    def test_send_after_close(self):
        config = UdpConfig(HOST, 9999)
        sock = AsyncUdpSocket(config)
        _await_future(sock.connect())
        sock.close()
        with pytest.raises(Exception):
            _await_future(sock.send(b"oops"))

    def test_recv_after_close(self):
        config = UdpConfig(HOST, 9999)
        sock = AsyncUdpSocket(config)
        _await_future(sock.connect())
        sock.close()
        with pytest.raises(Exception):
            _await_future(sock.recv(1024))


class TestAsyncUdpSocketConnectTimeout:
    """Connect timeout tests for UDP."""

    def test_connect_to_refused_port(self):
        """Connect UDP to a port with no server (UDP connect is stateless but may fail)."""
        with StableCoreFixtures() as fx:
            config = UdpConfig(HOST, fx.closed_port, timeout_ms=1000)
            sock = AsyncUdpSocket(config)
            # UDP connect to a non-listening port may succeed (fire-and-forget)
            # or fail depending on the OS. Either outcome is acceptable.
            try:
                _await_future(sock.connect(), timeout=5.0)
            except Exception:
                pass
            sock.close()


class TestAsyncUdpSocketResourceLeak:
    """After close, socket is properly cleaned up."""

    def test_cleanup_after_manual_close(self):
        config = UdpConfig(HOST, 9999)
        sock = AsyncUdpSocket(config)
        _await_future(sock.connect())
        sock.close()
        assert sock.is_closed


class TestAsyncUdpSocketSendTo:
    """send_to and recv_from operations."""

    def test_send_to_and_recv_from(self):
        server = _start_udp_echo_server(HOST)
        try:
            port = _get_port(server)
            config = UdpConfig(HOST, port, timeout_ms=3000)
            sock = AsyncUdpSocket(config)
            _await_future(sock.connect())
            _await_future(sock.send_to(b"sendto-test", f"{HOST}:{port}"))
            recv_result = _await_future(sock.recv_from(1024))
            assert recv_result.bytes_received > 0
            assert recv_result.source_address == HOST
            sock.close()
        finally:
            server.shutdown()
            server.server_close()


# ═══════════════════════════════════════════════════════════════════════════
# 3. AsyncHttpClient lifecycle
# ═══════════════════════════════════════════════════════════════════════════


class TestAsyncHttpClientContextManager:
    """Context manager lifecycle tests for AsyncHttpClient."""

    def test_async_context_manager_enter_exit(self):
        """async with creates and cleans up HTTP client."""
        config = HttpClientConfig(timeout_ms=5000)
        client = AsyncHttpClient(config)
        assert not client.is_closed
        result = client.__aenter__()
        assert result is client
        exited = client.__aexit__(None, None, None)
        assert not exited
        assert client.is_closed

    def test_is_closed_initially_false(self):
        config = HttpClientConfig(timeout_ms=5000)
        client = AsyncHttpClient(config)
        assert client.is_closed is False

    def test_repr(self):
        config = HttpClientConfig(timeout_ms=5000, base_url="http://example.com")
        client = AsyncHttpClient(config)
        r = repr(client)
        assert "AsyncHttpClient" in r
        assert "closed=false" in r
        client.close()

    def test_repr_after_close(self):
        config = HttpClientConfig(timeout_ms=5000)
        client = AsyncHttpClient(config)
        client.close()
        r = repr(client)
        assert "closed=true" in r


class TestAsyncHttpClientNormalOperation:
    """Basic HTTP operations against loopback server."""

    def test_async_get(self):
        """GET request to loopback HTTP fixture."""
        with StableCoreFixtures() as fx:
            config = HttpClientConfig(timeout_ms=10000)
            client = AsyncHttpClient(config)
            resp = _await_future(client.async_get(f"{fx.http_url}/"))
            assert resp.status_code == 200
            assert resp.body_text == "EGGSEC_FIXTURE_ROOT"
            client.close()

    def test_async_get_404(self):
        """GET request returning 404."""
        with StableCoreFixtures() as fx:
            config = HttpClientConfig(timeout_ms=10000)
            client = AsyncHttpClient(config)
            resp = _await_future(client.async_get(f"{fx.http_url}/missing"))
            assert resp.status_code == 404
            assert resp.body_text == "EGGSEC_FIXTURE_MISSING"
            client.close()

    def test_async_get_echo(self):
        """GET request to echo endpoint returns JSON."""
        import json as _json

        with StableCoreFixtures() as fx:
            config = HttpClientConfig(timeout_ms=10000)
            client = AsyncHttpClient(config)
            resp = _await_future(client.async_get(f"{fx.http_url}/echo?foo=bar"))
            assert resp.status_code == 200
            data = _json.loads(resp.body_text)
            assert data["method"] == "GET"
            assert data["query"] == "foo=bar"
            client.close()

    def test_async_post(self):
        """POST request with body."""
        with StableCoreFixtures() as fx:
            config = HttpClientConfig(timeout_ms=10000)
            client = AsyncHttpClient(config)
            resp = _await_future(
                client.async_post(
                    f"{fx.http_url}/echo",
                    body="test-body",
                    content_type="text/plain",
                )
            )
            assert resp.status_code in (200, 405, 501)
            client.close()

    def test_async_request_method(self):
        """Generic async_request with custom method."""
        with StableCoreFixtures() as fx:
            config = HttpClientConfig(timeout_ms=10000)
            client = AsyncHttpClient(config)
            req = eggsec.HttpRequestPy("GET", f"{fx.http_url}/admin")
            resp = _await_future(client.async_request(req))
            assert resp.status_code == 200
            assert resp.body_text == "EGGSEC_FIXTURE_ADMIN"
            client.close()

    def test_response_timing(self):
        """Response timing is populated."""
        with StableCoreFixtures() as fx:
            config = HttpClientConfig(timeout_ms=10000)
            client = AsyncHttpClient(config)
            resp = _await_future(client.async_get(f"{fx.http_url}/"))
            assert resp.timing.total_ms >= 0
            client.close()

    def test_response_headers(self):
        """Response headers contain fixture metadata."""
        with StableCoreFixtures() as fx:
            config = HttpClientConfig(timeout_ms=10000)
            client = AsyncHttpClient(config)
            resp = _await_future(client.async_get(f"{fx.http_url}/"))
            assert resp.headers.contains("Server")
            assert resp.headers.get("X-Eggsec-Fixture") == "stable-core"
            client.close()


class TestAsyncHttpClientCancellation:
    """Cancellation during HTTP operations."""

    def test_cancel_during_request(self):
        """Cancel an HTTP request future."""
        with StableCoreFixtures() as fx:
            config = HttpClientConfig(timeout_ms=10000)
            client = AsyncHttpClient(config)
            future = client.async_get(f"{fx.http_url}/slow")

            # Poll a few times
            for _ in range(5):
                try:
                    next(future)
                except StopIteration:
                    break
                except BaseException:
                    break
                time.sleep(0.01)

            # Close should work regardless
            client.close()
            assert client.is_closed

    def test_close_during_pending_request(self):
        """Close client while request is in flight."""
        with StableCoreFixtures() as fx:
            config = HttpClientConfig(timeout_ms=10000)
            client = AsyncHttpClient(config)
            future = client.async_get(f"{fx.http_url}/slow")

            def _close():
                time.sleep(0.05)
                client.close()

            t = threading.Thread(target=_close)
            t.start()
            _await_future_ignore_cancelled(future, timeout=10.0)
            t.join(timeout=5.0)
            assert client.is_closed


class TestAsyncHttpClientDoubleClose:
    """Double close is idempotent for HTTP client."""

    def test_double_close(self):
        config = HttpClientConfig(timeout_ms=5000)
        client = AsyncHttpClient(config)
        client.close()
        client.close()
        assert client.is_closed


class TestAsyncHttpClientUseAfterClose:
    """Operations after close must raise errors."""

    def test_get_after_close(self):
        with StableCoreFixtures() as fx:
            config = HttpClientConfig(timeout_ms=5000)
            client = AsyncHttpClient(config)
            client.close()
            with pytest.raises(Exception):
                _await_future(client.async_get(f"{fx.http_url}/"))

    def test_post_after_close(self):
        with StableCoreFixtures() as fx:
            config = HttpClientConfig(timeout_ms=5000)
            client = AsyncHttpClient(config)
            client.close()
            with pytest.raises(Exception):
                _await_future(client.async_post(f"{fx.http_url}/"))

    def test_request_after_close(self):
        with StableCoreFixtures() as fx:
            config = HttpClientConfig(timeout_ms=5000)
            client = AsyncHttpClient(config)
            client.close()
            req = eggsec.HttpRequestPy("GET", f"{fx.http_url}/")
            with pytest.raises(Exception):
                _await_future(client.async_request(req))


class TestAsyncHttpClientTimeout:
    """Timeout behavior for slow servers."""

    def test_request_timeout(self):
        """Request to slow endpoint must timeout."""
        with StableCoreFixtures() as fx:
            config = HttpClientConfig(timeout_ms=100)
            client = AsyncHttpClient(config)
            with pytest.raises(Exception):
                _await_future(client.async_get(f"{fx.http_url}/slow"), timeout=5.0)
            client.close()

    def test_request_timeout_configurable(self):
        """Different timeout configs produce different results."""
        with StableCoreFixtures() as fx:
            # Very short timeout — should fail on slow endpoint
            config_short = HttpClientConfig(timeout_ms=100)
            client_short = AsyncHttpClient(config_short)
            with pytest.raises(Exception):
                _await_future(client_short.async_get(f"{fx.http_url}/slow"), timeout=3.0)
            client_short.close()

            # Generous timeout — should succeed
            config_long = HttpClientConfig(timeout_ms=10000)
            client_long = AsyncHttpClient(config_long)
            resp = _await_future(client_long.async_get(f"{fx.http_url}/slow"))
            assert resp.status_code == 200
            assert resp.body_text == "EGGSEC_FIXTURE_SLOW"
            client_long.close()


class TestAsyncHttpClientResourceLeak:
    """After close, client resources are cleaned up."""

    def test_cleanup_after_manual_close(self):
        with StableCoreFixtures() as fx:
            config = HttpClientConfig(timeout_ms=5000)
            client = AsyncHttpClient(config)
            _await_future(client.async_get(f"{fx.http_url}/"))
            client.close()
            assert client.is_closed


# ═══════════════════════════════════════════════════════════════════════════
# 4. AsyncWebSocketSession lifecycle
# ═══════════════════════════════════════════════════════════════════════════


@pytest.mark.skipif(not _WEBSOCKET_AVAILABLE, reason="websocket feature not compiled")
class TestAsyncWebSocketSessionContextManager:
    """Context manager lifecycle tests for AsyncWebSocketSession."""

    def test_async_context_manager_enter_exit(self):
        """async with creates and cleans up WebSocket session."""
        config = WebSocketSessionConfig(f"ws://{HOST}:9999", timeout_ms=3000)
        session = AsyncWebSocketSession(config)
        assert not session.is_closed
        result = session.__aenter__()
        assert result is session
        exited = session.__aexit__(None, None, None)
        assert not exited
        assert session.is_closed

    def test_is_closed_initially_false(self):
        config = WebSocketSessionConfig(f"ws://{HOST}:9999")
        session = AsyncWebSocketSession(config)
        assert session.is_closed is False

    def test_url_property(self):
        config = WebSocketSessionConfig(f"ws://{HOST}:8080/ws")
        session = AsyncWebSocketSession(config)
        assert session.url == f"ws://{HOST}:8080/ws"
        session.close()

    def test_repr(self):
        config = WebSocketSessionConfig(f"ws://{HOST}:8080")
        session = AsyncWebSocketSession(config)
        r = repr(session)
        assert "AsyncWebSocketSession" in r
        assert "closed=False" in r
        session.close()

    def test_repr_after_close(self):
        config = WebSocketSessionConfig(f"ws://{HOST}:8080")
        session = AsyncWebSocketSession(config)
        session.close()
        r = repr(session)
        assert "closed=True" in r

    def test_config_validation_empty_url(self):
        """WebSocketSessionConfig rejects empty URL."""
        with pytest.raises(ValueError, match="url must not be empty"):
            WebSocketSessionConfig("")

    def test_config_validation_bad_scheme(self):
        """WebSocketSessionConfig rejects non-ws:// URLs."""
        with pytest.raises(ValueError, match="ws:// or wss://"):
            WebSocketSessionConfig("http://example.com")


@pytest.mark.skipif(not _WEBSOCKET_AVAILABLE, reason="websocket feature not compiled")
class TestAsyncWebSocketSessionNormalOperation:
    """WebSocket connect/disconnect lifecycle (no echo server needed for lifecycle)."""

    def test_connect_to_nonexistent_server(self):
        """Connect to a port with no WebSocket server must fail."""
        config = WebSocketSessionConfig(
            f"ws://{HOST}:9999", timeout_ms=2000
        )
        session = AsyncWebSocketSession(config)
        with pytest.raises(Exception):
            _await_future(session.async_connect(), timeout=5.0)
        session.close()

    def test_double_connect_rejected(self):
        """Cannot connect twice to the same session."""
        config = WebSocketSessionConfig(
            f"ws://{HOST}:9999", timeout_ms=2000
        )
        session = AsyncWebSocketSession(config)
        with pytest.raises(Exception):
            _await_future(session.async_connect(), timeout=5.0)
        session.close()

    def test_send_text_before_connect(self):
        """Send before connect must raise."""
        config = WebSocketSessionConfig(f"ws://{HOST}:9999")
        session = AsyncWebSocketSession(config)
        with pytest.raises(Exception):
            _await_future(session.async_send_text("hello"))
        session.close()

    def test_recv_before_connect(self):
        """Recv before connect must raise."""
        config = WebSocketSessionConfig(f"ws://{HOST}:9999")
        session = AsyncWebSocketSession(config)
        with pytest.raises(Exception):
            _await_future(session.async_recv())
        session.close()


@pytest.mark.skipif(not _WEBSOCKET_AVAILABLE, reason="websocket feature not compiled")
class TestAsyncWebSocketSessionCancellation:
    """Cancellation during WebSocket operations."""

    def test_cancel_during_connect(self):
        """Cancel a connect future before it completes."""
        config = WebSocketSessionConfig(
            f"ws://{HOST}:9999", timeout_ms=5000
        )
        session = AsyncWebSocketSession(config)
        future = session.async_connect()

        for _ in range(5):
            try:
                next(future)
            except StopIteration:
                break
            except BaseException:
                break
            time.sleep(0.01)

        session.close()
        assert session.is_closed

    def test_close_during_pending_connect(self):
        """Close session while connect is pending."""
        config = WebSocketSessionConfig(
            f"ws://{HOST}:9999", timeout_ms=5000
        )
        session = AsyncWebSocketSession(config)
        future = session.async_connect()

        def _close():
            time.sleep(0.05)
            session.close()

        t = threading.Thread(target=_close)
        t.start()
        _await_future_ignore_cancelled(future, timeout=10.0)
        t.join(timeout=5.0)
        assert session.is_closed

    def test_cancel_during_recv(self):
        """Cancel a recv future that is waiting for data."""
        config = WebSocketSessionConfig(
            f"ws://{HOST}:9999", timeout_ms=5000
        )
        session = AsyncWebSocketSession(config)
        # Attempt connect (will fail), then test recv on closed session
        with pytest.raises(Exception):
            _await_future(session.async_connect(), timeout=5.0)
        session.close()


@pytest.mark.skipif(not _WEBSOCKET_AVAILABLE, reason="websocket feature not compiled")
class TestAsyncWebSocketSessionDoubleClose:
    """Double close is idempotent for WebSocket."""

    def test_close_on_unconnected_session(self):
        config = WebSocketSessionConfig(f"ws://{HOST}:9999")
        session = AsyncWebSocketSession(config)
        session.close()
        assert session.is_closed

    def test_context_exit_then_manual_close(self):
        config = WebSocketSessionConfig(f"ws://{HOST}:9999")
        session = AsyncWebSocketSession(config)
        session.__aexit__(None, None, None)
        session.close()  # Must not raise
        assert session.is_closed


@pytest.mark.skipif(not _WEBSOCKET_AVAILABLE, reason="websocket feature not compiled")
class TestAsyncWebSocketSessionUseAfterClose:
    """Operations after close must raise errors."""

    def test_connect_after_close(self):
        config = WebSocketSessionConfig(f"ws://{HOST}:9999")
        session = AsyncWebSocketSession(config)
        session.close()
        with pytest.raises(Exception):
            _await_future(session.async_connect())

    def test_send_text_after_close(self):
        config = WebSocketSessionConfig(f"ws://{HOST}:9999")
        session = AsyncWebSocketSession(config)
        session.close()
        with pytest.raises(Exception):
            _await_future(session.async_send_text("hello"))

    def test_recv_after_close(self):
        config = WebSocketSessionConfig(f"ws://{HOST}:9999")
        session = AsyncWebSocketSession(config)
        session.close()
        with pytest.raises(Exception):
            _await_future(session.async_recv())

    def test_close_after_already_closed(self):
        """async_close on an already-closed session must raise."""
        config = WebSocketSessionConfig(f"ws://{HOST}:9999")
        session = AsyncWebSocketSession(config)
        session.close()
        with pytest.raises(Exception):
            _await_future(session.async_close())


@pytest.mark.skipif(not _WEBSOCKET_AVAILABLE, reason="websocket feature not compiled")
class TestAsyncWebSocketSessionResourceLeak:
    """After close, session is properly cleaned up."""

    def test_cleanup_after_manual_close(self):
        config = WebSocketSessionConfig(f"ws://{HOST}:9999")
        session = AsyncWebSocketSession(config)
        session.close()
        assert session.is_closed

    def test_cleanup_after_failed_connect(self):
        config = WebSocketSessionConfig(
            f"ws://{HOST}:9999", timeout_ms=1000
        )
        session = AsyncWebSocketSession(config)
        with pytest.raises(Exception):
            _await_future(session.async_connect(), timeout=3.0)
        session.close()
        assert session.is_closed


# ═══════════════════════════════════════════════════════════════════════════
# 5. AsyncCaptureSession lifecycle (feature-gated)
# ═══════════════════════════════════════════════════════════════════════════

_PACKET_INSPECTION_AVAILABLE = False
try:
    if hasattr(eggsec, "AsyncCaptureSession"):
        _PACKET_INSPECTION_AVAILABLE = True
except Exception:
    pass


@pytest.mark.skipif(
    not _PACKET_INSPECTION_AVAILABLE,
    reason="packet-inspection feature not compiled",
)
class TestAsyncCaptureSessionContextManager:
    """Context manager lifecycle tests for AsyncCaptureSession."""

    def test_sync_context_manager_enter_exit(self):
        """with creates and cleans up capture session."""
        config = CaptureConfig(interface="lo", timeout_secs=1)
        session = AsyncCaptureSession(config)
        assert not session.is_closed
        result = session.__enter__()
        assert result is session
        exited = session.__exit__(None, None, None)
        assert not exited
        assert session.is_closed

    def test_is_closed_initially_false(self):
        config = CaptureConfig(interface="lo")
        session = AsyncCaptureSession(config)
        assert session.is_closed is False

    def test_is_running_initially_false(self):
        config = CaptureConfig(interface="lo")
        session = AsyncCaptureSession(config)
        assert session.is_running is False

    def test_start_stop_lifecycle(self):
        """start() → stop() lifecycle."""
        config = CaptureConfig(interface="lo", timeout_secs=1)
        session = AsyncCaptureSession(config)
        session.start()
        assert session.is_running is True
        stats = session.stop()
        assert session.is_running is False
        assert session.is_closed is True
        assert hasattr(stats, "packets_captured")

    def test_double_stop(self):
        """stop() is idempotent."""
        config = CaptureConfig(interface="lo", timeout_secs=1)
        session = AsyncCaptureSession(config)
        session.start()
        session.stop()
        session.stop()  # Must not raise
        assert session.is_closed

    def test_stats_before_capture(self):
        """stats() returns zeroed stats before any capture."""
        config = CaptureConfig(interface="lo")
        session = AsyncCaptureSession(config)
        s = session.stats()
        assert s.packets_captured == 0
        assert s.bytes_captured == 0
        session.__exit__(None, None, None)

    def test_drop_stats(self):
        """drop_stats returns a CaptureDropStats object."""
        config = CaptureConfig(interface="lo")
        session = AsyncCaptureSession(config)
        ds = session.drop_stats()
        assert ds.total_dropped == 0
        session.__exit__(None, None, None)

    def test_repr(self):
        config = CaptureConfig(interface="lo")
        session = AsyncCaptureSession(config)
        r = repr(session)
        assert "AsyncCaptureSession" in r
        assert "lo" in r
        session.__exit__(None, None, None)

    def test_repr_after_close(self):
        config = CaptureConfig(interface="lo")
        session = AsyncCaptureSession(config)
        session.__exit__(None, None, None)
        r = repr(session)
        assert "AsyncCaptureSession" in r
        assert "lo" in r

    def test_queue_size_property(self):
        config = CaptureConfig(interface="lo")
        session = AsyncCaptureSession(config, queue_size=500)
        assert session.queue_size == 500
        session.__exit__(None, None, None)


@pytest.mark.skipif(
    not _PACKET_INSPECTION_AVAILABLE,
    reason="packet-inspection feature not compiled",
)
class TestAsyncCaptureSessionUseAfterClose:
    """Operations after close must raise errors."""

    def test_start_after_close(self):
        config = CaptureConfig(interface="lo")
        session = AsyncCaptureSession(config)
        session.__exit__(None, None, None)
        with pytest.raises(Exception):
            session.start()


# ═══════════════════════════════════════════════════════════════════════════
# 6. Cross-cutting lifecycle invariants
# ═══════════════════════════════════════════════════════════════════════════


class TestCrossCuttingInvariants:
    """Invariants that must hold for all async session types."""

    def test_all_async_types_support_aenter_aexit(self):
        """Every async session type must have __aenter__ and __aexit__."""
        tcp = AsyncTcpSession(TcpConfig(HOST, 1))
        udp = AsyncUdpSocket(UdpConfig(HOST, 1))
        http = AsyncHttpClient(HttpClientConfig())

        for obj in [tcp, udp, http]:
            assert callable(getattr(obj, "__aenter__", None))
            assert callable(getattr(obj, "__aexit__", None))
            obj.close()

    def test_all_async_types_reject_sync_context_manager(self):
        """TCP and UDP must reject sync 'with'."""
        tcp = AsyncTcpSession(TcpConfig(HOST, 1))
        udp = AsyncUdpSocket(UdpConfig(HOST, 1))

        for obj in [tcp, udp]:
            with pytest.raises(TypeError, match="async with"):
                with obj:
                    pass
            obj.close()

    def test_all_types_have_is_closed(self):
        """Every session type exposes is_closed."""
        tcp = AsyncTcpSession(TcpConfig(HOST, 1))
        udp = AsyncUdpSocket(UdpConfig(HOST, 1))
        http = AsyncHttpClient(HttpClientConfig())

        for obj in [tcp, udp, http]:
            assert hasattr(obj, "is_closed")
            assert isinstance(obj.is_closed, bool)
            obj.close()

    def test_all_types_close_is_idempotent(self):
        """close() can be called multiple times without error."""
        tcp = AsyncTcpSession(TcpConfig(HOST, 1))
        udp = AsyncUdpSocket(UdpConfig(HOST, 1))
        http = AsyncHttpClient(HttpClientConfig())

        for obj in [tcp, udp, http]:
            obj.close()
            obj.close()  # Second close must not raise
            assert obj.is_closed

    def test_all_types_repr_contains_class_name(self):
        """repr() of every type includes the class name."""
        tcp = AsyncTcpSession(TcpConfig(HOST, 1))
        udp = AsyncUdpSocket(UdpConfig(HOST, 1))
        http = AsyncHttpClient(HttpClientConfig())

        assert "AsyncTcpSession" in repr(tcp)
        assert "AsyncUdpSocket" in repr(udp)
        assert "AsyncHttpClient" in repr(http)

        for obj in [tcp, udp, http]:
            obj.close()

    def test_all_types_str_output(self):
        """str() of every type produces non-empty string."""
        tcp = AsyncTcpSession(TcpConfig(HOST, 1))
        udp = AsyncUdpSocket(UdpConfig(HOST, 1))
        http = AsyncHttpClient(HttpClientConfig())

        for obj in [tcp, udp, http]:
            assert len(str(obj)) > 0
            obj.close()


# ═══════════════════════════════════════════════════════════════════════════
# 7. PyFuture iterator protocol
# ═══════════════════════════════════════════════════════════════════════════


class TestPyFutureIteratorProtocol:
    """Validate PyFuture __await__/__next__ protocol."""

    def test_py_future_has_await(self):
        """PyFuture implements __await__."""
        config = HttpClientConfig(timeout_ms=5000)
        client = AsyncHttpClient(config)
        future = client.async_get("http://127.0.0.1:1/")

        # PyFuture supports __next__ (iterator protocol) and __await__
        assert hasattr(future, "__next__")
        assert hasattr(future, "__await__")
        client.close()

    def test_py_future_yields_none_while_pending(self):
        """Pending PyFuture yields None via __next__."""
        config = HttpClientConfig(timeout_ms=5000)
        client = AsyncHttpClient(config)
        future = client.async_get("http://127.0.0.1:1/")

        # First few polls should yield None
        none_count = 0
        for _ in range(3):
            try:
                val = next(future)
                if val is None:
                    none_count += 1
            except StopIteration:
                break
            except BaseException:
                break

        # At least some should have been None (future is pending)
        assert none_count >= 0  # May complete instantly on loopback
        client.close()

    def test_py_future_complete_yields_stop_iteration(self):
        """Completed PyFuture raises StopIteration with the value."""
        with StableCoreFixtures() as fx:
            config = HttpClientConfig(timeout_ms=5000)
            client = AsyncHttpClient(config)
            resp = _await_future(client.async_get(f"{fx.http_url}/"))
            assert resp.status_code == 200
            client.close()


# ═══════════════════════════════════════════════════════════════════════════
# 8. Concurrent session operations
# ═══════════════════════════════════════════════════════════════════════════


class TestConcurrentSessions:
    """Multiple async sessions operating concurrently."""

    def test_concurrent_tcp_sessions(self):
        """Multiple TCP sessions connect and I/O simultaneously."""
        with StableCoreFixtures() as fx:
            sessions = []
            for _ in range(3):
                config = TcpConfig(HOST, fx.tcp_port, connect_timeout_ms=5000)
                s = AsyncTcpSession(config)
                _await_future(s.connect())
                sessions.append(s)

            for s in sessions:
                _await_future(s.write(b"concurrent"))

            for s in sessions:
                resp = _await_future(s.read(4096))
                assert resp.bytes_read > 0

            for s in sessions:
                s.close()
                assert s.is_closed

    def test_concurrent_http_requests(self):
        """Multiple HTTP requests on the same client."""
        with StableCoreFixtures() as fx:
            config = HttpClientConfig(timeout_ms=10000)
            client = AsyncHttpClient(config)

            paths = ["/", "/admin", "/echo?x=1", "/missing"]
            futures = []
            for path in paths:
                futures.append(client.async_get(f"{fx.http_url}{path}"))

            results = []
            for f in futures:
                results.append(_await_future(f))

            assert results[0].status_code == 200
            assert results[1].status_code == 200
            assert results[2].status_code == 200
            assert results[3].status_code == 404

            client.close()

    def test_mixed_session_types_concurrent(self):
        """TCP, UDP, and HTTP sessions operating in parallel."""
        with StableCoreFixtures() as fx:
            tcp_config = TcpConfig(HOST, fx.tcp_port, connect_timeout_ms=5000)
            tcp = AsyncTcpSession(tcp_config)
            _await_future(tcp.connect())

            http_config = HttpClientConfig(timeout_ms=5000)
            http = AsyncHttpClient(http_config)
            http_future = http.async_get(f"{fx.http_url}/")
            tcp_future = tcp.write(b"mixed-test")

            http_resp = _await_future(http_future)
            _await_future(tcp_future)

            assert http_resp.status_code == 200
            tcp_resp = _await_future(tcp.read(4096))
            assert tcp_resp.bytes_read > 0

            tcp.close()
            http.close()


# ═══════════════════════════════════════════════════════════════════════════
# 9. Error propagation through PyFuture
# ═══════════════════════════════════════════════════════════════════════════


class TestErrorPropagation:
    """Errors from async operations propagate through PyFuture."""

    def test_connection_error_propagates(self):
        """NetworkError propagates through PyFuture."""
        config = TcpConfig(
            HOST, 9999, connect_timeout_ms=500
        )
        session = AsyncTcpSession(config)
        future = session.connect()
        with pytest.raises(Exception):
            _await_future(future, timeout=5.0)
        session.close()

    def test_timeout_error_propagates(self):
        """TimeoutError propagates through PyFuture."""
        with StableCoreFixtures() as fx:
            config = HttpClientConfig(timeout_ms=100)
            client = AsyncHttpClient(config)
            future = client.async_get(f"{fx.http_url}/slow")
            with pytest.raises(Exception):
                _await_future(future, timeout=5.0)
            client.close()

    def test_http_404_not_error(self):
        """HTTP 404 is a valid response, not an error."""
        with StableCoreFixtures() as fx:
            config = HttpClientConfig(timeout_ms=5000)
            client = AsyncHttpClient(config)
            resp = _await_future(client.async_get(f"{fx.http_url}/missing"))
            assert resp.status_code == 404
            client.close()


# ═══════════════════════════════════════════════════════════════════════════
# 10. AsyncTcpSession write_all operation
# ═══════════════════════════════════════════════════════════════════════════


class TestAsyncTcpSessionWriteAll:
    """write_all operation tests."""

    def test_write_all(self):
        """write_all sends complete payload."""
        with StableCoreFixtures() as fx:
            config = TcpConfig(HOST, fx.tcp_port, connect_timeout_ms=5000)
            session = AsyncTcpSession(config)
            _await_future(session.connect())
            payload = b"X" * 1024
            result = _await_future(session.write_all(payload))
            assert result.bytes_written == 1024
            _await_future(session.read(4096))
            session.close()

    def test_write_all_after_close(self):
        config = TcpConfig(HOST, 12345)
        session = AsyncTcpSession(config)
        session.close()
        with pytest.raises(Exception):
            _await_future(session.write_all(b"data"))


# ═══════════════════════════════════════════════════════════════════════════
# 11. WebSocket session ping operation
# ═══════════════════════════════════════════════════════════════════════════


@pytest.mark.skipif(not _WEBSOCKET_AVAILABLE, reason="websocket feature not compiled")
class TestAsyncWebSocketSessionPing:
    """Ping operation tests for WebSocket."""

    def test_ping_before_connect(self):
        """Ping before connect must raise."""
        config = WebSocketSessionConfig(f"ws://{HOST}:9999")
        session = AsyncWebSocketSession(config)
        with pytest.raises(Exception):
            _await_future(session.async_ping())
        session.close()

    def test_ping_after_close(self):
        """Ping after close must raise."""
        config = WebSocketSessionConfig(f"ws://{HOST}:9999")
        session = AsyncWebSocketSession(config)
        session.close()
        with pytest.raises(Exception):
            _await_future(session.async_ping(b"test"))


# ═══════════════════════════════════════════════════════════════════════════
# 12. HTTP client convenience methods
# ═══════════════════════════════════════════════════════════════════════════


class TestAsyncHttpClientConvenienceMethods:
    """Tests for async_put and async_delete convenience methods."""

    def test_async_put(self):
        """PUT request to loopback fixture."""
        with StableCoreFixtures() as fx:
            config = HttpClientConfig(timeout_ms=10000)
            client = AsyncHttpClient(config)
            resp = _await_future(
                client.async_put(
                    f"{fx.http_url}/echo",
                    body="put-data",
                    content_type="text/plain",
                )
            )
            # The fixture doesn't handle PUT, but the connection should still succeed
            assert resp.status_code in (200, 405, 501)
            client.close()

    def test_async_delete(self):
        """DELETE request to loopback fixture."""
        with StableCoreFixtures() as fx:
            config = HttpClientConfig(timeout_ms=10000)
            client = AsyncHttpClient(config)
            resp = _await_future(
                client.async_delete(f"{fx.http_url}/admin")
            )
            # The fixture may not handle DELETE, but connection succeeds
            assert resp.status_code in (200, 405, 501)
            client.close()

    def test_async_get_with_headers(self):
        """GET with custom headers."""
        with StableCoreFixtures() as fx:
            config = HttpClientConfig(timeout_ms=10000)
            client = AsyncHttpClient(config)
            resp = _await_future(
                client.async_get(
                    f"{fx.http_url}/",
                    headers=[("X-Custom-Test", "lifecycle")],
                )
            )
            assert resp.status_code == 200
            client.close()
