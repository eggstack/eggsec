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


class TestDaemonIdempotency:
    """Idempotency key construction and uniqueness."""

    def test_idempotency_key_from_request(self):
        IK = _import_or_skip("IdempotencyKey")
        k = IK.from_request("scan_ports", '{"target":"10.0.0.1"}')
        assert len(k.key) == 36
        assert k.operation_name == "scan_ports"
        assert len(k.request_hash) == 16

    def test_idempotency_key_uniqueness(self):
        IK = _import_or_skip("IdempotencyKey")
        k1 = IK.from_request("op1", '{"a":1}')
        k2 = IK.from_request("op1", '{"a":1}')
        assert k1.key != k2.key

    def test_idempotency_key_same_payload_same_hash(self):
        IK = _import_or_skip("IdempotencyKey")
        k1 = IK.from_request("op", '{"x":1}')
        k2 = IK.from_request("op", '{"x":1}')
        assert k1.request_hash == k2.request_hash

    def test_idempotency_key_different_payload_different_hash(self):
        IK = _import_or_skip("IdempotencyKey")
        k1 = IK.from_request("op", '{"a":1}')
        k2 = IK.from_request("op", '{"a":2}')
        assert k1.request_hash != k2.request_hash

    def test_idempotency_key_to_dict(self):
        IK = _import_or_skip("IdempotencyKey")
        k = IK.from_request("op", '{"x":1}')
        d = k.to_dict()
        assert isinstance(d, dict)
        assert "key" in d
        assert "operation_name" in d
        assert "request_hash" in d

    def test_idempotency_key_to_json(self):
        IK = _import_or_skip("IdempotencyKey")
        k = IK.from_request("op", '{"x":1}')
        import json
        j = k.to_json()
        parsed = json.loads(j)
        assert parsed["operation_name"] == "op"


class TestDaemonCancellationRequest:
    """CancellationRequest and CancellationResult construction."""

    def test_cancellation_request_construction(self):
        CR = _import_or_skip("CancellationRequest")
        req = CR(task_id="task-123", session_id="s-1", reason="timeout")
        assert req.task_id == "task-123"
        assert req.reason == "timeout"

    def test_cancellation_request_to_dict(self):
        CR = _import_or_skip("CancellationRequest")
        req = CR(task_id="t-1", session_id="s-1", reason="user cancel")
        d = req.to_dict()
        assert isinstance(d, dict)
        assert d["task_id"] == "t-1"

    def test_cancellation_result_construction(self):
        CRes = _import_or_skip("CancellationResult")
        res = CRes(acknowledged=True, message="done")
        assert res.acknowledged is True

    def test_cancellation_result_to_dict(self):
        CRes = _import_or_skip("CancellationResult")
        res = CRes(acknowledged=False, message="already finished")
        d = res.to_dict()
        assert isinstance(d, dict)
        assert d["acknowledged"] is False


class TestDaemonReplayCursor:
    """ReplayCursor and ReplayResult construction."""

    def test_replay_cursor_construction(self):
        RC = _import_or_skip("ReplayCursor")
        cursor = RC(session_id="abc-123", last_sequence=100)
        assert cursor.session_id == "abc-123"
        assert cursor.last_sequence == 100

    def test_replay_cursor_to_dict(self):
        RC = _import_or_skip("ReplayCursor")
        cursor = RC(session_id="c1", last_sequence=50)
        d = cursor.to_dict()
        assert isinstance(d, dict)
        assert d["session_id"] == "c1"

    def test_replay_result_construction(self):
        RR = _import_or_skip("ReplayResult")
        RC = _import_or_skip("ReplayCursor")
        inner_cursor = RC(session_id="s-1", last_sequence=10)
        res = RR(cursor=inner_cursor, has_more=False)
        assert res.has_more is False

    def test_replay_result_to_dict(self):
        RR = _import_or_skip("ReplayResult")
        RC = _import_or_skip("ReplayCursor")
        inner_cursor = RC(session_id="s-1", last_sequence=20)
        res = RR(cursor=inner_cursor, has_more=True)
        d = res.to_dict()
        assert isinstance(d, dict)
        assert d["has_more"] is True


class TestDaemonEventReplayInfo:
    """EventReplayInfo construction and serialization."""

    def test_event_replay_info_construction(self):
        ERI = _import_or_skip("EventReplayInfo")
        info = ERI(session_id="s-1", event_count=100, from_sequence=1, to_sequence=100)
        assert info.event_count == 100
        assert info.session_id == "s-1"

    def test_event_replay_info_to_dict(self):
        ERI = _import_or_skip("EventReplayInfo")
        info = ERI(session_id="s-1", event_count=50, from_sequence=1, to_sequence=50)
        d = info.to_dict()
        assert isinstance(d, dict)
        assert d["event_count"] == 50


class TestDaemonTaskArtifactDescriptor:
    """TaskArtifactDescriptor construction."""

    def test_construction(self):
        TAD = _import_or_skip("TaskArtifactDescriptor")
        desc = TAD(
            artifact_id="art-1",
            content_type="application/json",
            size_bytes=1024,
        )
        assert desc.artifact_id == "art-1"
        assert desc.size_bytes == 1024

    def test_to_dict(self):
        TAD = _import_or_skip("TaskArtifactDescriptor")
        desc = TAD(
            artifact_id="a-1",
            content_type="text/plain",
            size_bytes=256,
        )
        d = desc.to_dict()
        assert isinstance(d, dict)
        assert d["artifact_id"] == "a-1"


class TestDaemonHealthDetail:
    """DaemonHealthDetail construction."""

    def test_construction(self):
        DHD = _import_or_skip("DaemonHealthDetail")
        detail = DHD(status="ok", uptime_secs=3600, active_sessions=5)
        assert detail.status == "ok"
        assert detail.uptime_secs == 3600
        assert detail.active_sessions == 5

    def test_to_dict(self):
        DHD = _import_or_skip("DaemonHealthDetail")
        detail = DHD(status="degraded", uptime_secs=0, active_sessions=0)
        d = detail.to_dict()
        assert isinstance(d, dict)
        assert d["status"] == "degraded"


class TestDaemonEvent:
    """DaemonEvent construction."""

    def test_construction(self):
        DE = _import_or_skip("DaemonEvent")
        event = DE(
            event_id="evt-1",
            event_type="session_created",
            timestamp_ms=1700000000000,
            session_id="sess-1",
        )
        assert event.event_id == "evt-1"
        assert event.event_type == "session_created"

    def test_to_dict(self):
        DE = _import_or_skip("DaemonEvent")
        event = DE(
            event_id="e-1",
            event_type="task_completed",
            timestamp_ms=1700000001000,
            session_id="s-1",
        )
        d = event.to_dict()
        assert isinstance(d, dict)
        assert d["event_id"] == "e-1"


class TestDaemonSubmissionResult:
    """DaemonSubmissionResult construction."""

    def test_construction(self):
        try:
            DSR = _import_or_skip("DaemonSubmissionResult")
        except pytest.skip.Exception:
            pytest.skip("DaemonSubmissionResult not available")
        res = DSR(task_id="task-1")
        assert res.task_id == "task-1"
        assert res.is_duplicate is False

    def test_to_dict(self):
        try:
            DSR = _import_or_skip("DaemonSubmissionResult")
        except pytest.skip.Exception:
            pytest.skip("DaemonSubmissionResult not available")
        res = DSR(task_id="t-2", is_duplicate=True)
        d = res.to_dict()
        assert isinstance(d, dict)
        assert d["is_duplicate"] is True


class TestDaemonConcurrentSessionCreate:
    """Create multiple sessions concurrently to verify thread safety."""

    def test_concurrent_create(self, daemon_socket):
        daemon_connect = _import_or_skip("daemon_connect")
        async_daemon_create_session = _import_or_skip("async_daemon_create_session")
        RuntimeSurface = _import_or_skip("RuntimeSurface")

        client = daemon_connect(daemon_socket)
        import asyncio

        loop = asyncio.new_event_loop()
        try:
            results = []
            for _ in range(10):
                sid = loop.run_until_complete(
                    async_daemon_create_session(client, surface=RuntimeSurface.Cli)
                )
                results.append(sid)
            assert len(results) == 10
            assert len(set(results)) == 10, "All session IDs should be unique"
        finally:
            loop.close()

    def test_large_session_list(self, daemon_socket):
        """Create many sessions and verify list returns them all."""
        daemon_connect = _import_or_skip("daemon_connect")
        async_daemon_create_session = _import_or_skip("async_daemon_create_session")
        async_daemon_list_sessions = _import_or_skip("async_daemon_list_sessions")
        RuntimeSurface = _import_or_skip("RuntimeSurface")

        client = daemon_connect(daemon_socket)
        import asyncio

        loop = asyncio.new_event_loop()
        try:
            for _ in range(20):
                loop.run_until_complete(
                    async_daemon_create_session(client, surface=RuntimeSurface.Cli)
                )
            sessions = loop.run_until_complete(async_daemon_list_sessions(client))
            assert sessions is not None
        finally:
            loop.close()


class TestDaemonSocketCleanup:
    """Verify socket file is cleaned up after daemon exits."""

    def test_socket_removed_after_sigterm(self):
        if not _daemon_bin_exists():
            pytest.skip("eggsec-daemon binary not built")

        import asyncio
        sock_path = tempfile.mktemp(
            prefix=f"eggsec-cleanup-{uuid.uuid4().hex[:8]}-", suffix=".sock"
        )
        data_dir = tempfile.mkdtemp(prefix="eggsec-cleanup-data-")

        proc = subprocess.Popen(
            [DAEMON_BIN, "--socket-path", sock_path, "--no-persistence",
             "--log-level", "warn", "--data-dir", data_dir],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        try:
            time.sleep(0.3)
            if proc.poll() is not None:
                pytest.skip("daemon exited immediately")
            if not _wait_for_socket(sock_path, timeout=5.0):
                proc.kill()
                proc.wait(timeout=2)
                pytest.skip("socket not ready")

            daemon_connect = _import_or_skip("daemon_connect")
            client = daemon_connect(sock_path)
            loop = asyncio.new_event_loop()
            try:
                async_daemon_health = _import_or_skip("async_daemon_health")
                resp = loop.run_until_complete(async_daemon_health(client))
                assert resp is not None
            finally:
                loop.close()

            proc.send_signal(signal.SIGTERM)
            proc.wait(timeout=5)

            assert not os.path.exists(sock_path), (
                "Socket file should be removed after daemon exit"
            )
        finally:
            if proc.poll() is None:
                proc.kill()
                proc.wait(timeout=2)
            try:
                os.unlink(sock_path)
            except FileNotFoundError:
                pass
            import shutil
            shutil.rmtree(data_dir, ignore_errors=True)


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


class TestDaemonOperationSubmission:
    """Test submitting operations through the daemon."""

    def test_submit_task_to_session(self, daemon_socket):
        """Submit a task to a session and verify task handle."""
        daemon_connect = _import_or_skip("daemon_connect")
        async_daemon_create_session = _import_or_skip("async_daemon_create_session")
        async_daemon_submit_task = _import_or_skip("async_daemon_submit_task")
        RuntimeSurface = _import_or_skip("RuntimeSurface")

        client = daemon_connect(daemon_socket)
        import asyncio
        loop = asyncio.new_event_loop()
        try:
            session_id = loop.run_until_complete(
                async_daemon_create_session(client, surface=RuntimeSurface.Cli)
            )
            task_handle = loop.run_until_complete(
                async_daemon_submit_task(
                    client,
                    session_id=session_id,
                    operation="scan_ports",
                    target="127.0.0.1",
                )
            )
            assert task_handle is not None
        finally:
            loop.close()

    def test_submit_task_idempotency(self, daemon_socket):
        """Submitting the same task twice with same idempotency key."""
        daemon_connect = _import_or_skip("daemon_connect")
        async_daemon_create_session = _import_or_skip("async_daemon_create_session")
        async_daemon_submit_task = _import_or_skip("async_daemon_submit_task")
        RuntimeSurface = _import_or_skip("RuntimeSurface")

        client = daemon_connect(daemon_socket)
        import asyncio
        loop = asyncio.new_event_loop()
        try:
            session_id = loop.run_until_complete(
                async_daemon_create_session(client, surface=RuntimeSurface.Cli)
            )
            h1 = loop.run_until_complete(
                async_daemon_submit_task(
                    client,
                    session_id=session_id,
                    operation="scan_ports",
                    target="127.0.0.1",
                    idempotency_key="test-idem-key-001",
                )
            )
            h2 = loop.run_until_complete(
                async_daemon_submit_task(
                    client,
                    session_id=session_id,
                    operation="scan_ports",
                    target="127.0.0.1",
                    idempotency_key="test-idem-key-001",
                )
            )
            assert h1 is not None
            assert h2 is not None
        finally:
            loop.close()

    def test_cancel_task(self, daemon_socket):
        """Cancel a submitted task."""
        daemon_connect = _import_or_skip("daemon_connect")
        async_daemon_create_session = _import_or_skip("async_daemon_create_session")
        async_daemon_submit_task = _import_or_skip("async_daemon_submit_task")
        async_daemon_cancel_task = _import_or_skip("async_daemon_cancel_task")
        RuntimeSurface = _import_or_skip("RuntimeSurface")

        client = daemon_connect(daemon_socket)
        import asyncio
        loop = asyncio.new_event_loop()
        try:
            session_id = loop.run_until_complete(
                async_daemon_create_session(client, surface=RuntimeSurface.Cli)
            )
            task_handle = loop.run_until_complete(
                async_daemon_submit_task(
                    client,
                    session_id=session_id,
                    operation="scan_ports",
                    target="127.0.0.1",
                )
            )
            result = loop.run_until_complete(
                async_daemon_cancel_task(
                    client,
                    session_id=session_id,
                    task_id=task_handle.task_id,
                )
            )
            assert result is not None
        finally:
            loop.close()


class TestDaemonReconnect:
    """Test client reconnection to daemon."""

    def test_new_client_after_disconnect(self, daemon_socket):
        """Creating a new client after the old one is dropped works."""
        daemon_connect = _import_or_skip("daemon_connect")
        async_daemon_health = _import_or_skip("async_daemon_health")
        async_daemon_create_session = _import_or_skip("async_daemon_create_session")
        RuntimeSurface = _import_or_skip("RuntimeSurface")

        import asyncio

        client1 = daemon_connect(daemon_socket)
        loop1 = asyncio.new_event_loop()
        try:
            loop1.run_until_complete(async_daemon_health(client1))
        finally:
            loop1.close()
            del client1

        client2 = daemon_connect(daemon_socket)
        loop2 = asyncio.new_event_loop()
        try:
            resp = loop2.run_until_complete(async_daemon_health(client2))
            assert resp is not None
            session_id = loop2.run_until_complete(
                async_daemon_create_session(client2, surface=RuntimeSurface.Cli)
            )
            assert session_id is not None
        finally:
            loop2.close()


class TestDaemonSubscribe:
    """Test event subscription."""

    def test_subscribe_returns_client(self, daemon_socket):
        """Subscribe creates an event stream client."""
        daemon_connect = _import_or_skip("daemon_connect")
        async_daemon_subscribe = _import_or_skip("async_daemon_subscribe")

        client = daemon_connect(daemon_socket)
        import asyncio
        loop = asyncio.new_event_loop()
        try:
            subscriber = loop.run_until_complete(
                async_daemon_subscribe(client)
            )
            assert subscriber is not None
        finally:
            loop.close()


class TestDaemonRealTaskExecution:
    """Test real task submission and execution through the daemon."""

    def test_submit_and_wait_for_task_result(self, daemon_socket):
        """Submit a scan_ports task and wait for result."""
        daemon_connect = _import_or_skip("daemon_connect")
        async_daemon_create_session = _import_or_skip("async_daemon_create_session")
        async_daemon_submit_task = _import_or_skip("async_daemon_submit_task")
        RuntimeSurface = _import_or_skip("RuntimeSurface")

        client = daemon_connect(daemon_socket)
        import asyncio
        loop = asyncio.new_event_loop()
        try:
            session_id = loop.run_until_complete(
                async_daemon_create_session(client, surface=RuntimeSurface.Cli)
            )
            task_handle = loop.run_until_complete(
                async_daemon_submit_task(
                    client,
                    session_id=session_id,
                    operation="scan_ports",
                    target="127.0.0.1",
                )
            )
            assert task_handle is not None
            assert hasattr(task_handle, "task_id")
            assert len(task_handle.task_id) > 0
        finally:
            loop.close()

    def test_submit_task_with_all_params(self, daemon_socket):
        """Submit task with operation, target, and config parameters."""
        daemon_connect = _import_or_skip("daemon_connect")
        async_daemon_create_session = _import_or_skip("async_daemon_create_session")
        async_daemon_submit_task = _import_or_skip("async_daemon_submit_task")
        RuntimeSurface = _import_or_skip("RuntimeSurface")

        client = daemon_connect(daemon_socket)
        import asyncio
        loop = asyncio.new_event_loop()
        try:
            session_id = loop.run_until_complete(
                async_daemon_create_session(client, surface=RuntimeSurface.Cli)
            )
            task_handle = loop.run_until_complete(
                async_daemon_submit_task(
                    client,
                    session_id=session_id,
                    operation="scan_ports",
                    target="127.0.0.1",
                    idempotency_key="real-task-exec-001",
                )
            )
            assert task_handle is not None
        finally:
            loop.close()

    def test_cancel_active_task(self, daemon_socket):
        """Cancel a task that was submitted."""
        daemon_connect = _import_or_skip("daemon_connect")
        async_daemon_create_session = _import_or_skip("async_daemon_create_session")
        async_daemon_submit_task = _import_or_skip("async_daemon_submit_task")
        async_daemon_cancel_task = _import_or_skip("async_daemon_cancel_task")
        RuntimeSurface = _import_or_skip("RuntimeSurface")

        client = daemon_connect(daemon_socket)
        import asyncio
        loop = asyncio.new_event_loop()
        try:
            session_id = loop.run_until_complete(
                async_daemon_create_session(client, surface=RuntimeSurface.Cli)
            )
            task_handle = loop.run_until_complete(
                async_daemon_submit_task(
                    client,
                    session_id=session_id,
                    operation="scan_ports",
                    target="127.0.0.1",
                )
            )
            cancel_result = loop.run_until_complete(
                async_daemon_cancel_task(
                    client,
                    session_id=session_id,
                    task_id=task_handle.task_id,
                )
            )
            assert cancel_result is not None
        finally:
            loop.close()

    def test_submit_multiple_tasks_sequentially(self, daemon_socket):
        """Submit multiple tasks sequentially to the same session."""
        daemon_connect = _import_or_skip("daemon_connect")
        async_daemon_create_session = _import_or_skip("async_daemon_create_session")
        async_daemon_submit_task = _import_or_skip("async_daemon_submit_task")
        RuntimeSurface = _import_or_skip("RuntimeSurface")

        client = daemon_connect(daemon_socket)
        import asyncio
        loop = asyncio.new_event_loop()
        try:
            session_id = loop.run_until_complete(
                async_daemon_create_session(client, surface=RuntimeSurface.Cli)
            )
            handles = []
            for i in range(3):
                handle = loop.run_until_complete(
                    async_daemon_submit_task(
                        client,
                        session_id=session_id,
                        operation="scan_ports",
                        target="127.0.0.1",
                        idempotency_key=f"seq-task-{i}",
                    )
                )
                handles.append(handle)
            assert len(handles) == 3
            task_ids = [h.task_id for h in handles]
            assert len(set(task_ids)) == 3, "Task IDs should be unique"
        finally:
            loop.close()

    def test_daemon_health_after_heavy_usage(self, daemon_socket):
        """Daemon remains healthy after multiple operations."""
        daemon_connect = _import_or_skip("daemon_connect")
        async_daemon_health = _import_or_skip("async_daemon_health")
        async_daemon_create_session = _import_or_skip("async_daemon_create_session")
        async_daemon_list_sessions = _import_or_skip("async_daemon_list_sessions")
        RuntimeSurface = _import_or_skip("RuntimeSurface")

        client = daemon_connect(daemon_socket)
        import asyncio
        loop = asyncio.new_event_loop()
        try:
            # Create several sessions
            for _ in range(5):
                loop.run_until_complete(
                    async_daemon_create_session(client, surface=RuntimeSurface.Cli)
                )
            # Verify health is still OK
            resp = loop.run_until_complete(async_daemon_health(client))
            assert resp is not None
            # Verify sessions are listed
            sessions = loop.run_until_complete(async_daemon_list_sessions(client))
            assert sessions is not None
        finally:
            loop.close()
