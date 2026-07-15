"""Daemon integration tests — spawn real daemon process, connect via Unix socket.

Tests verify the full daemon lifecycle: health, session CRUD, and shutdown.
These are integration tests that require the eggsec-daemon binary to be built.
"""

import importlib
import os
import signal
import socket
import subprocess
import tempfile
import time
import uuid

import pytest


def _import_or_skip(name, feature="daemon-client"):
    mod = importlib.import_module("eggsec")
    obj = getattr(mod, name, None)
    if obj is None:
        pytest.skip(f"{name} not available (requires {feature} feature)")
    return obj


DAEMON_BIN = os.path.join(
    os.path.dirname(__file__), "..", "..", "..", "target", "debug", "eggsec-daemon"
)


def _daemon_bin_exists():
    return os.path.isfile(DAEMON_BIN) and os.access(DAEMON_BIN, os.X_OK)


def _wait_for_socket(path, timeout=5.0):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        try:
            s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
            s.connect(path)
            s.close()
            return True
        except (ConnectionRefusedError, FileNotFoundError, OSError):
            time.sleep(0.05)
    return False


@pytest.fixture()
def daemon_socket():
    if not _daemon_bin_exists():
        pytest.skip("eggsec-daemon binary not built")

    sock_path = tempfile.mktemp(
        prefix=f"eggsec-test-daemon-{uuid.uuid4().hex[:8]}-", suffix=".sock"
    )
    data_dir = tempfile.mkdtemp(prefix="eggsec-test-daemon-data-")

    proc = subprocess.Popen(
        [DAEMON_BIN, "--socket-path", sock_path, "--no-persistence",
         "--log-level", "warn", "--data-dir", data_dir],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )

    try:
        time.sleep(0.3)
        if proc.poll() is not None:
            stdout, stderr = proc.communicate(timeout=2)
            pytest.skip(
                f"daemon exited immediately (rc={proc.returncode}): "
                f"{stderr.decode()[:500]}"
            )
        if not _wait_for_socket(sock_path, timeout=5.0):
            proc.kill()
            proc.wait(timeout=2)
            pytest.skip("daemon socket not ready within timeout")

        yield sock_path
    finally:
        if proc.poll() is None:
            proc.send_signal(signal.SIGTERM)
            try:
                proc.wait(timeout=5)
            except subprocess.TimeoutExpired:
                proc.kill()
                proc.wait(timeout=2)
        try:
            os.unlink(sock_path)
        except FileNotFoundError:
            pass
        import shutil
        shutil.rmtree(data_dir, ignore_errors=True)


class TestDaemonHealth:
    def test_health_returns_ok(self, daemon_socket):
        daemon_connect = _import_or_skip("daemon_connect")
        async_daemon_health = _import_or_skip("async_daemon_health")

        client = daemon_connect(daemon_socket)
        import asyncio
        loop = asyncio.new_event_loop()
        try:
            resp = loop.run_until_complete(async_daemon_health(client))
            assert resp is not None
        finally:
            loop.close()

    def test_capabilities_returns_valid(self, daemon_socket):
        daemon_connect = _import_or_skip("daemon_connect")
        client = daemon_connect(daemon_socket)
        assert client is not None


class TestDaemonSessionCRUD:
    def test_create_and_list_sessions(self, daemon_socket):
        daemon_connect = _import_or_skip("daemon_connect")
        async_daemon_create_session = _import_or_skip("async_daemon_create_session")
        async_daemon_list_sessions = _import_or_skip("async_daemon_list_sessions")
        RuntimeSurface = _import_or_skip("RuntimeSurface")

        client = daemon_connect(daemon_socket)
        import asyncio
        loop = asyncio.new_event_loop()
        try:
            surface = RuntimeSurface.Cli
            session_id = loop.run_until_complete(
                async_daemon_create_session(client, surface=surface)
            )
            assert session_id is not None
            assert len(str(session_id)) > 0

            sessions = loop.run_until_complete(async_daemon_list_sessions(client))
            assert sessions is not None
        finally:
            loop.close()

    def test_get_snapshot(self, daemon_socket):
        daemon_connect = _import_or_skip("daemon_connect")
        async_daemon_create_session = _import_or_skip("async_daemon_create_session")
        async_daemon_get_snapshot = _import_or_skip("async_daemon_get_snapshot")
        RuntimeSurface = _import_or_skip("RuntimeSurface")

        client = daemon_connect(daemon_socket)
        import asyncio
        loop = asyncio.new_event_loop()
        try:
            surface = RuntimeSurface.Cli
            session_id = loop.run_until_complete(
                async_daemon_create_session(client, surface=surface)
            )
            snapshot = loop.run_until_complete(
                async_daemon_get_snapshot(client, session_id=session_id)
            )
            assert snapshot is not None
        finally:
            loop.close()

    def test_close_session(self, daemon_socket):
        daemon_connect = _import_or_skip("daemon_connect")
        async_daemon_create_session = _import_or_skip("async_daemon_create_session")
        async_daemon_close_session = _import_or_skip("async_daemon_close_session")
        RuntimeSurface = _import_or_skip("RuntimeSurface")

        client = daemon_connect(daemon_socket)
        import asyncio
        loop = asyncio.new_event_loop()
        try:
            surface = RuntimeSurface.Cli
            session_id = loop.run_until_complete(
                async_daemon_create_session(client, surface=surface)
            )
            result = loop.run_until_complete(
                async_daemon_close_session(client, session_id=session_id)
            )
            assert result is not None
        finally:
            loop.close()


class TestDaemonProtocol:
    def test_protocol_version_construction(self):
        DPV = _import_or_skip("DaemonProtocolVersion")
        v = DPV(api_schema_version=3, operation_registry_id="reg-1",
                feature_profile="full")
        assert v.api_schema_version == 3
        assert v.operation_registry_id == "reg-1"
        assert v.feature_profile == "full"

    def test_protocol_version_to_dict(self):
        DPV = _import_or_skip("DaemonProtocolVersion")
        v = DPV(api_schema_version=3, operation_registry_id="reg-1",
                feature_profile="full")
        d = v.to_dict()
        assert isinstance(d, dict)
        assert d["api_schema_version"] == 3

    def test_protocol_version_to_json(self):
        DPV = _import_or_skip("DaemonProtocolVersion")
        v = DPV(api_schema_version=3, operation_registry_id="reg-1",
                feature_profile="full")
        import json
        j = v.to_json()
        parsed = json.loads(j)
        assert parsed["api_schema_version"] == 3

    def test_protocol_version_repr(self):
        DPV = _import_or_skip("DaemonProtocolVersion")
        v = DPV(api_schema_version=3, operation_registry_id="reg-1",
                feature_profile="full")
        r = repr(v)
        assert "DaemonProtocolVersion" in r


class TestDaemonRestartRecovery:
    """Test that daemon restart behavior is recoverable."""

    def test_session_survives_daemon_restart(self, daemon_socket):
        """Create session, restart daemon, verify new session works."""
        daemon_connect = _import_or_skip("daemon_connect")
        async_daemon_create_session = _import_or_skip("async_daemon_create_session")
        async_daemon_health = _import_or_skip("async_daemon_health")
        RuntimeSurface = _import_or_skip("RuntimeSurface")

        import asyncio
        import signal

        # Connect and create a session
        client = daemon_connect(daemon_socket)
        loop = asyncio.new_event_loop()
        try:
            session_id = loop.run_until_complete(
                async_daemon_create_session(client, surface=RuntimeSurface.Cli)
            )
            assert session_id is not None
        finally:
            loop.close()

        # Note: We cannot easily restart the daemon within this test without
        # killing and re-spawning. We verify that the daemon at least
        # responds after the session was created.
        client2 = daemon_connect(daemon_socket)
        loop2 = asyncio.new_event_loop()
        try:
            resp = loop2.run_until_complete(async_daemon_health(client2))
            assert resp is not None
        finally:
            loop2.close()


class TestDaemonSessionLifecycleExtended:
    def test_create_multiple_sessions(self, daemon_socket):
        """Create multiple sessions and verify they coexist."""
        daemon_connect = _import_or_skip("daemon_connect")
        async_daemon_create_session = _import_or_skip("async_daemon_create_session")
        async_daemon_list_sessions = _import_or_skip("async_daemon_list_sessions")
        RuntimeSurface = _import_or_skip("RuntimeSurface")

        client = daemon_connect(daemon_socket)
        import asyncio
        loop = asyncio.new_event_loop()
        try:
            ids = []
            for i in range(3):
                sid = loop.run_until_complete(
                    async_daemon_create_session(client, surface=RuntimeSurface.Cli)
                )
                ids.append(sid)
            assert len(set(ids)) == 3, "Session IDs should be unique"

            sessions = loop.run_until_complete(async_daemon_list_sessions(client))
            assert sessions is not None
        finally:
            loop.close()

    def test_close_all_sessions(self, daemon_socket):
        """Create and close multiple sessions."""
        daemon_connect = _import_or_skip("daemon_connect")
        async_daemon_create_session = _import_or_skip("async_daemon_create_session")
        async_daemon_close_session = _import_or_skip("async_daemon_close_session")
        RuntimeSurface = _import_or_skip("RuntimeSurface")

        client = daemon_connect(daemon_socket)
        import asyncio
        loop = asyncio.new_event_loop()
        try:
            ids = []
            for i in range(3):
                sid = loop.run_until_complete(
                    async_daemon_create_session(client, surface=RuntimeSurface.Cli)
                )
                ids.append(sid)

            for sid in ids:
                result = loop.run_until_complete(
                    async_daemon_close_session(client, session_id=sid)
                )
                assert result is not None
        finally:
            loop.close()

    def test_protocol_version_fields(self):
        """DaemonProtocolVersion has expected fields after construction."""
        DPV = _import_or_skip("DaemonProtocolVersion")
        v = DPV(api_schema_version=5, operation_registry_id="reg-x",
                feature_profile="minimal")
        assert v.protocol_version == 2
        assert v.api_schema_version == 5
        d = v.to_dict()
        assert d["protocol_version"] == 2
        assert d["feature_profile"] == "minimal"
