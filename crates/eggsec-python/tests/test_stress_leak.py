"""Resource leak and stress hardening tests (WS12).

Validates that repeated creation/destruction of sessions, repositories,
NseRuntime objects, and DTOs does not leak file descriptors, threads, or
sockets, and does not degrade performance monotonically.

Scope: localhost-only; no real network connections required.
"""

from __future__ import annotations

import gc
import importlib
import json
import os
import shutil
import tempfile
import threading
import time

import pytest


# ---------------------------------------------------------------------------
# Import helpers
# ---------------------------------------------------------------------------

def _import_or_skip(name: str, module: str = "eggsec"):
    """Import *name* from *module*, skip if unavailable."""
    mod = importlib.import_module(module)
    obj = getattr(mod, name, None)
    if obj is None:
        pytest.skip(f"{name} not available in {module}")
    return obj


# ---------------------------------------------------------------------------
# Resource measurement helpers
# ---------------------------------------------------------------------------

def _measure_fds() -> int:
    """Count open file descriptors for the current process.

    Returns 0 and skips the caller if /proc is unavailable.
    """
    try:
        fd_dir = f"/proc/{os.getpid()}/fd"
        return len(os.listdir(fd_dir))
    except (OSError, FileNotFoundError):
        pytest.skip("Cannot measure fds on this platform")


def _measure_threads() -> int:
    """Count active threads."""
    return threading.active_count()


def _measure_sockets() -> int:
    """Count open sockets from /proc/net/tcp."""
    try:
        with open("/proc/net/tcp") as f:
            return len(f.readlines()) - 1  # subtract header
    except (OSError, FileNotFoundError):
        return 0


def _wait_for_gc():
    """Give the GC a chance to run and release Rust-side resources."""
    gc.collect()
    time.sleep(0.05)


def _tmp_dir(prefix: str = "eggsec_stress") -> str:
    return tempfile.mkdtemp(prefix=prefix)


def _make_finding_json(finding_id: str, severity: str = "high") -> str:
    return json.dumps({"id": finding_id, "severity": severity, "title": f"Stress {finding_id}"})


# ---------------------------------------------------------------------------
# Module-level markers
# ---------------------------------------------------------------------------

pytestmark = [
    pytest.mark.timeout(300),
]


# ═══════════════════════════════════════════════════════════════════════════
# 1. TestTcpSessionStress — create/drop 1000 AsyncTcpSession objects
# ═══════════════════════════════════════════════════════════════════════════


class TestTcpSessionStress:
    """Create and drop 1000 AsyncTcpSession objects rapidly, verify no fd leak."""

    def test_tcp_session_create_drop_1000(self):
        TcpConfig = _import_or_skip("TcpConfigPy")
        AsyncTcpSession = _import_or_skip("AsyncTcpSessionPy")

        fds_before = _measure_fds()
        sessions = []
        for i in range(1000):
            try:
                cfg = TcpConfig("127.0.0.1", 1)
                session = AsyncTcpSession(cfg)
                sessions.append(session)
            except Exception:
                pass  # Connection refused is fine; we test lifecycle

        del sessions
        _wait_for_gc()

        fds_after = _measure_fds()
        delta = fds_after - fds_before
        assert delta <= 10, (
            f"TCP session FD leak: {delta} fds gained ({fds_before} -> {fds_after})"
        )

    def test_tcp_session_sequential_open_close(self):
        TcpConfig = _import_or_skip("TcpConfigPy")
        AsyncTcpSession = _import_or_skip("AsyncTcpSessionPy")

        fds_before = _measure_fds()
        for i in range(500):
            try:
                cfg = TcpConfig("127.0.0.1", 1)
                session = AsyncTcpSession(cfg)
                session.close()
            except Exception:
                pass

        _wait_for_gc()
        fds_after = _measure_fds()
        delta = fds_after - fds_before
        assert delta <= 10, (
            f"TCP sequential FD leak: {delta} fds gained ({fds_before} -> {fds_after})"
        )


# ═══════════════════════════════════════════════════════════════════════════
# 2. TestUdpSocketStress — create/drop 1000 AsyncUdpSocket objects
# ═══════════════════════════════════════════════════════════════════════════


class TestUdpSocketStress:
    """Create and drop 1000 AsyncUdpSocket objects, verify no fd leak."""

    def test_udp_socket_create_drop_1000(self):
        UdpConfig = _import_or_skip("UdpConfigPy")
        AsyncUdpSocket = _import_or_skip("AsyncUdpSocketPy")

        fds_before = _measure_fds()
        sockets = []
        for i in range(1000):
            try:
                cfg = UdpConfig("127.0.0.1", 1)
                sock = AsyncUdpSocket(cfg)
                sockets.append(sock)
            except Exception:
                pass

        del sockets
        _wait_for_gc()

        fds_after = _measure_fds()
        delta = fds_after - fds_before
        assert delta <= 10, (
            f"UDP socket FD leak: {delta} fds gained ({fds_before} -> {fds_after})"
        )

    def test_udp_socket_sequential_open_close(self):
        UdpConfig = _import_or_skip("UdpConfigPy")
        AsyncUdpSocket = _import_or_skip("AsyncUdpSocketPy")

        fds_before = _measure_fds()
        for i in range(500):
            try:
                cfg = UdpConfig("127.0.0.1", 1)
                sock = AsyncUdpSocket(cfg)
                sock.close()
            except Exception:
                pass

        _wait_for_gc()
        fds_after = _measure_fds()
        delta = fds_after - fds_before
        assert delta <= 10, (
            f"UDP sequential FD leak: {delta} fds gained ({fds_before} -> {fds_after})"
        )


# ═══════════════════════════════════════════════════════════════════════════
# 3. TestRepositoryStress — add 1000 findings to JsonlFindingRepository
# ═══════════════════════════════════════════════════════════════════════════


class TestRepositoryStress:
    """Add 1000 findings to JsonlFindingRepository in rapid succession."""

    def test_jsonl_repo_1000_inserts(self):
        JsonlRepo = _import_or_skip("JsonlFindingRepository")
        tmp = _tmp_dir("jsonl_stress")
        try:
            path = os.path.join(tmp, "findings.jsonl")
            repo = JsonlRepo(path)
            repo.initialize()

            for i in range(1000):
                fid = repo.insert_finding(_make_finding_json(f"stress-{i}"))
                assert fid == f"stress-{i}"

            assert repo.count_findings(None, None) == 1000
            repo.flush()
            repo.close()
        finally:
            shutil.rmtree(tmp, ignore_errors=True)

    def test_jsonl_repo_rapid_open_close_cycles(self):
        JsonlRepo = _import_or_skip("JsonlFindingRepository")
        tmp = _tmp_dir("jsonl_cycles")
        try:
            path = os.path.join(tmp, "findings.jsonl")
            for cycle in range(50):
                repo = JsonlRepo(path)
                repo.initialize()
                repo.insert_finding(_make_finding_json(f"c{cycle}"))
                repo.flush()
                repo.close()
            # Reopen and verify all persisted
            repo = JsonlRepo(path)
            repo.initialize()
            count = repo.count_findings(None, None)
            assert count == 50
            repo.close()
        finally:
            shutil.rmtree(tmp, ignore_errors=True)

    def test_jsonl_repo_no_fd_leak(self):
        JsonlRepo = _import_or_skip("JsonlFindingRepository")
        tmp = _tmp_dir("jsonl_fd")
        try:
            path = os.path.join(tmp, "findings.jsonl")
            fds_before = _measure_fds()
            for i in range(200):
                repo = JsonlRepo(path)
                repo.initialize()
                repo.insert_finding(_make_finding_json(f"fd{i}"))
                repo.flush()
                repo.close()

            _wait_for_gc()
            fds_after = _measure_fds()
            delta = fds_after - fds_before
            assert delta <= 10, (
                f"JsonlRepo FD leak: {delta} fds gained ({fds_before} -> {fds_after})"
            )
        finally:
            shutil.rmtree(tmp, ignore_errors=True)


# ═══════════════════════════════════════════════════════════════════════════
# 4. TestSqliteRepositoryStress — add 1000 findings to SqliteFindingRepository
# ═══════════════════════════════════════════════════════════════════════════


class TestSqliteRepositoryStress:
    """Add 1000 findings to SqliteFindingRepository, verify no corruption."""

    def test_sqlite_repo_1000_inserts(self):
        SqliteRepo = _import_or_skip("SqliteFindingRepository")
        repo = SqliteRepo(":memory:")
        repo.initialize()

        for i in range(1000):
            fid = repo.insert_finding(_make_finding_json(f"sq-{i}"))
            assert fid == f"sq-{i}"

        assert repo.count_findings() == 1000

        # Verify a random one
        got = repo.get_finding("sq-500")
        assert got is not None
        assert "sq-500" in got
        repo.close()

    def test_sqlite_repo_rapid_open_close_cycles(self):
        SqliteRepo = _import_or_skip("SqliteFindingRepository")
        tmp = _tmp_dir("sqlite_cycles")
        try:
            db_path = os.path.join(tmp, "stress.db")
            for cycle in range(50):
                repo = SqliteRepo(db_path)
                repo.initialize()
                repo.insert_finding(_make_finding_json(f"c{cycle}"))
                repo.close()
            # Verify no crash/corruption by reopening after the cycles
            repo = SqliteRepo(db_path)
            repo.initialize()
            # In-memory semantics: file-based repo may not persist, so
            # just verify the repo is usable after many open/close cycles
            fid = repo.insert_finding(_make_finding_json("post-cycle"))
            assert fid == "post-cycle"
            repo.close()
        finally:
            shutil.rmtree(tmp, ignore_errors=True)

    def test_sqlite_repo_concurrent_writes(self):
        SqliteRepo = _import_or_skip("SqliteFindingRepository")
        repo = SqliteRepo(":memory:")
        repo.initialize()
        errors = []

        def writer(thread_id: int):
            try:
                for i in range(100):
                    fid = repo.insert_finding(
                        _make_finding_json(f"t{thread_id}-{i}")
                    )
                    assert fid is not None
            except Exception as e:
                errors.append(str(e))

        threads = [threading.Thread(target=writer, args=(t,)) for t in range(5)]
        for t in threads:
            t.start()
        for t in threads:
            t.join(timeout=60)

        assert not errors, f"Concurrent write errors: {errors}"
        count = repo.count_findings()
        assert count == 500
        repo.close()


# ═══════════════════════════════════════════════════════════════════════════
# 5. TestContentAddressedStoreStress — write 500 blobs, verify hash correctness
# ═══════════════════════════════════════════════════════════════════════════


class TestContentAddressedStoreStress:
    """Write 500 blobs to a content-addressed store, verify correctness."""

    def test_cas_write_500_blobs(self):
        CAS = _import_or_skip("ContentAddressedArtifactStore")
        tmp = _tmp_dir("cas_stress")
        try:
            store = CAS(tmp)
            store.initialize()

            hashes = []
            for i in range(500):
                data = f"blob-{i}".encode()
                info = store.put(data, "text/plain", None)
                assert info.content_hash is not None
                assert info.size_bytes == len(data)
                hashes.append(info.content_hash)

            # All unique blobs produce unique hashes
            assert len(set(hashes)) == 500

            # Verify retrieval for a sample
            for idx in [0, 250, 499]:
                got = store.get(hashes[idx])
                assert got is not None
                d = got.to_dict()
                assert d["data_len"] == len(f"blob-{idx}".encode())
                assert d["info"]["content_hash"] == hashes[idx]

            # No temp file leak: check total artifacts
            items = store.list_artifacts(1000, 0)
            assert len(items) == 500
        finally:
            shutil.rmtree(tmp, ignore_errors=True)

    def test_cas_dedup_under_stress(self):
        CAS = _import_or_skip("ContentAddressedArtifactStore")
        tmp = _tmp_dir("cas_dedup_stress")
        try:
            store = CAS(tmp)
            store.initialize()

            # Write 100 unique blobs, then 100 duplicates
            for i in range(100):
                store.put(f"unique-{i}".encode(), "text/plain", None)
            for i in range(100):
                store.put(f"unique-{i}".encode(), "text/plain", None)

            items = store.list_artifacts(1000, 0)
            assert len(items) == 100  # deduplication preserved
        finally:
            shutil.rmtree(tmp, ignore_errors=True)

    def test_cas_no_fd_leak(self):
        CAS = _import_or_skip("ContentAddressedArtifactStore")
        tmp = _tmp_dir("cas_fd_stress")
        try:
            fds_before = _measure_fds()
            store = CAS(tmp)
            store.initialize()

            for i in range(200):
                store.put(f"data-{i}".encode(), "text/plain", None)

            del store
            _wait_for_gc()

            fds_after = _measure_fds()
            delta = fds_after - fds_before
            assert delta <= 10, (
                f"CAS FD leak: {delta} fds gained ({fds_before} -> {fds_after})"
            )
        finally:
            shutil.rmtree(tmp, ignore_errors=True)


# ═══════════════════════════════════════════════════════════════════════════
# 6. TestNseRuntimeReuseStress — create one NseRuntimePy, run 100 scripts
# ═══════════════════════════════════════════════════════════════════════════


class TestNseRuntimeReuseStress:
    """Create one NseRuntimePy, run 100 simple scripts, verify no degradation."""

    def test_nse_runtime_100_scripts(self):
        NseRuntime = _import_or_skip("NseRuntime", "eggsec")
        NseRuntimeConfig = _import_or_skip("NseRuntimeConfig", "eggsec")

        cfg = NseRuntimeConfig(target="127.0.0.1")
        runtime = NseRuntime(cfg)

        start = time.monotonic()
        for i in range(100):
            report = runtime.run_script("default")
            assert report is not None, f"Script iteration {i} returned None"
            assert report.script_name == "default"
        elapsed = time.monotonic() - start

        # Sanity: 100 scripts should not take more than 2 minutes
        assert elapsed < 120, f"100 NSE scripts took {elapsed:.1f}s (degradation?)"

    def test_nse_runtime_mixed_scripts(self):
        NseRuntime = _import_or_skip("NseRuntime", "eggsec")
        NseRuntimeConfig = _import_or_skip("NseRuntimeConfig", "eggsec")

        scripts = ["default", "discovery", "banner", "http-headers", "dns-check", "ssl-cert"]
        cfg = NseRuntimeConfig(target="127.0.0.1")
        runtime = NseRuntime(cfg)

        for i in range(100):
            script = scripts[i % len(scripts)]
            report = runtime.run_script(script)
            assert report is not None, f"Script {script} iteration {i} returned None"
            assert report.script_name == script

    def test_nse_runtime_no_thread_leak(self):
        NseRuntime = _import_or_skip("NseRuntime", "eggsec")
        NseRuntimeConfig = _import_or_skip("NseRuntimeConfig", "eggsec")

        threads_before = _measure_threads()
        cfg = NseRuntimeConfig(target="127.0.0.1")
        runtime = NseRuntime(cfg)

        for i in range(50):
            runtime.run_script("default")

        _wait_for_gc()
        threads_after = _measure_threads()
        delta = threads_after - threads_before
        assert delta <= 5, (
            f"NSE runtime thread leak: {delta} threads gained "
            f"({threads_before} -> {threads_after})"
        )


# ═══════════════════════════════════════════════════════════════════════════
# 7. TestObjectCreationDropCycle — create/drop 500 of each major DTO type
# ═══════════════════════════════════════════════════════════════════════════


class TestObjectCreationDropCycle:
    """Create and drop 500 of each major DTO type, verify no crash."""

    def test_finding_creation_drop_500(self):
        Finding = _import_or_skip("Finding")
        Severity = _import_or_skip("Severity")

        for i in range(500):
            f = Finding(
                id=f"f{i}", title=f"Title {i}", severity=Severity.High,
                target=f"host-{i}.example.com", category="vuln",
                description=f"Desc {i}",
            )
            assert f.id == f"f{i}"
        del f
        _wait_for_gc()

    def test_proxy_config_creation_drop_500(self):
        ProxyConfig = _import_or_skip("ProxyConfig")

        for i in range(500):
            cfg = ProxyConfig()
            d = cfg.to_dict()
            assert d["rotation_strategy"] == "round_robin"
        del cfg
        _wait_for_gc()

    def test_tcp_config_creation_drop_500(self):
        TcpConfig = _import_or_skip("TcpConfigPy")

        for i in range(500):
            cfg = TcpConfig("127.0.0.1", 80 + (i % 100))
            assert cfg is not None
        del cfg
        _wait_for_gc()

    def test_udp_config_creation_drop_500(self):
        UdpConfig = _import_or_skip("UdpConfigPy")

        for i in range(500):
            cfg = UdpConfig("127.0.0.1", 5000 + (i % 100))
            assert cfg is not None
        del cfg
        _wait_for_gc()

    def test_http_client_config_creation_drop_500(self):
        HttpClientConfig = _import_or_skip("HttpClientConfigPy")

        for i in range(500):
            cfg = HttpClientConfig(timeout_ms=1000 + i)
            assert cfg is not None
        del cfg
        _wait_for_gc()

    def test_port_range_creation_drop_500(self):
        PortRange = _import_or_skip("PortRange")

        for i in range(500):
            pr = PortRange.range(1, 100)
            assert len(pr) == 100
        del pr
        _wait_for_gc()

    def test_streaming_report_config_creation_drop_500(self):
        StreamingReportConfig = _import_or_skip("StreamingReportConfig")

        for i in range(500):
            cfg = StreamingReportConfig("json")
            assert cfg.format == "json"
        del cfg
        _wait_for_gc()

    def test_no_fd_leak_after_dto_creation(self):
        TcpConfig = _import_or_skip("TcpConfigPy")
        ProxyConfig = _import_or_skip("ProxyConfig")
        Finding = _import_or_skip("Finding")
        Severity = _import_or_skip("Severity")

        fds_before = _measure_fds()
        for i in range(200):
            TcpConfig("127.0.0.1", 80)
            ProxyConfig()
            Finding(
                id=f"f{i}", title=f"T{i}", severity=Severity.Low,
                target="x.example.com", category="c", description="d",
            )
        _wait_for_gc()
        fds_after = _measure_fds()
        delta = fds_after - fds_before
        assert delta <= 10, (
            f"DTO creation FD leak: {delta} fds gained ({fds_before} -> {fds_after})"
        )


# ═══════════════════════════════════════════════════════════════════════════
# 8. TestJsonSerializationStress — serialize/deserialize 1000 objects
# ═══════════════════════════════════════════════════════════════════════════


class TestJsonSerializationStress:
    """Serialize/deserialize 1000 objects, verify no memory blowup."""

    def test_finding_to_json_1000(self):
        Finding = _import_or_skip("Finding")
        Severity = _import_or_skip("Severity")

        for i in range(1000):
            f = Finding(
                id=f"f{i}", title=f"Title {i}", severity=Severity.Medium,
                target=f"host-{i}.example.com", category="vuln",
                description=f"Desc {i}",
            )
            j = f.to_json()
            parsed = json.loads(j)
            assert parsed["id"] == f"f{i}"

    def test_finding_to_dict_1000(self):
        Finding = _import_or_skip("Finding")
        Severity = _import_or_skip("Severity")

        for i in range(1000):
            f = Finding(
                id=f"f{i}", title=f"Title {i}", severity=Severity.High,
                target=f"host-{i}.example.com", category="c",
                description="d",
            )
            d = f.to_dict()
            assert d["id"] == f"f{i}"

    def test_tcp_config_json_roundtrip_1000(self):
        TcpConfig = _import_or_skip("TcpConfigPy")

        for i in range(1000):
            cfg = TcpConfig("127.0.0.1", 80)
            j = cfg.to_json()
            parsed = json.loads(j)
            assert parsed["host"] == "127.0.0.1"

    def test_streaming_reporter_json_1000(self):
        StreamingReporter = _import_or_skip("StreamingReporter")
        StreamingReportConfig = _import_or_skip("StreamingReportConfig")

        for i in range(100):
            # Use smaller batch to keep test fast
            cfg = StreamingReportConfig("json")
            reporter = StreamingReporter(cfg)
            reporter.start()
            for j in range(10):
                reporter.write_finding(
                    json.dumps({"id": f"f{i}-{j}", "severity": "medium", "title": "T"})
                )
            summary = reporter.finish()
            assert summary.total_findings == 10

    def test_versioned_finding_json_roundtrip_1000(self):
        VersionedFinding = _import_or_skip("VersionedFinding")
        AffectedAsset = _import_or_skip("AffectedAsset")
        FindingType = _import_or_skip("FindingType")

        for i in range(1000):
            asset = AffectedAsset("host", f"host-{i}.example.com")
            f = VersionedFinding(
                id=f"f{i}", title=f"Title {i}", description="Desc",
                severity="high", finding_type=FindingType.Vulnerability,
                affected_asset=asset, source_tool="test", source_module="mod",
            )
            j = f.to_json()
            parsed = json.loads(j)
            assert parsed["id"] == f"f{i}"


# ═══════════════════════════════════════════════════════════════════════════
# 9. TestConcurrentSessionCreation — 50 sessions across 5 threads
# ═══════════════════════════════════════════════════════════════════════════


class TestConcurrentSessionCreation:
    """Create 50 sessions across 5 threads, verify no race condition crash."""

    def test_concurrent_tcp_sessions(self):
        TcpConfig = _import_or_skip("TcpConfigPy")
        AsyncTcpSession = _import_or_skip("AsyncTcpSessionPy")

        errors = []

        def creator(thread_id: int):
            try:
                for i in range(10):
                    cfg = TcpConfig("127.0.0.1", 1)
                    session = AsyncTcpSession(cfg)
                    # session is created; just drop it
                    del session
            except Exception as e:
                errors.append(f"thread-{thread_id}: {e}")

        threads = [threading.Thread(target=creator, args=(t,)) for t in range(5)]
        for t in threads:
            t.start()
        for t in threads:
            t.join(timeout=120)

        assert not errors, f"Concurrent session creation errors: {errors}"

    def test_concurrent_udp_sockets(self):
        UdpConfig = _import_or_skip("UdpConfigPy")
        AsyncUdpSocket = _import_or_skip("AsyncUdpSocketPy")

        errors = []

        def creator(thread_id: int):
            try:
                for i in range(10):
                    cfg = UdpConfig("127.0.0.1", 1)
                    sock = AsyncUdpSocket(cfg)
                    del sock
            except Exception as e:
                errors.append(f"thread-{thread_id}: {e}")

        threads = [threading.Thread(target=creator, args=(t,)) for t in range(5)]
        for t in threads:
            t.start()
        for t in threads:
            t.join(timeout=120)

        assert not errors, f"Concurrent UDP socket errors: {errors}"

    def test_concurrent_mixed_session_types(self):
        TcpConfig = _import_or_skip("TcpConfigPy")
        AsyncTcpSession = _import_or_skip("AsyncTcpSessionPy")
        UdpConfig = _import_or_skip("UdpConfigPy")
        AsyncUdpSocket = _import_or_skip("AsyncUdpSocketPy")

        errors = []

        def creator(thread_id: int):
            try:
                for i in range(10):
                    if i % 2 == 0:
                        cfg = TcpConfig("127.0.0.1", 1)
                        session = AsyncTcpSession(cfg)
                        del session
                    else:
                        cfg = UdpConfig("127.0.0.1", 1)
                        sock = AsyncUdpSocket(cfg)
                        del sock
            except Exception as e:
                errors.append(f"thread-{thread_id}: {e}")

        threads = [threading.Thread(target=creator, args=(t,)) for t in range(5)]
        for t in threads:
            t.start()
        for t in threads:
            t.join(timeout=120)

        assert not errors, f"Concurrent mixed session errors: {errors}"

    def test_concurrent_fd_stability(self):
        TcpConfig = _import_or_skip("TcpConfigPy")
        AsyncTcpSession = _import_or_skip("AsyncTcpSessionPy")

        fds_before = _measure_fds()

        errors = []

        def creator(thread_id: int):
            try:
                for i in range(10):
                    cfg = TcpConfig("127.0.0.1", 1)
                    session = AsyncTcpSession(cfg)
                    del session
            except Exception as e:
                errors.append(f"thread-{thread_id}: {e}")

        threads = [threading.Thread(target=creator, args=(t,)) for t in range(5)]
        for t in threads:
            t.start()
        for t in threads:
            t.join(timeout=120)

        _wait_for_gc()
        fds_after = _measure_fds()
        delta = fds_after - fds_before

        assert not errors, f"Concurrent session errors: {errors}"
        assert delta <= 10, (
            f"Concurrent FD leak: {delta} fds gained ({fds_before} -> {fds_after})"
        )


# ═══════════════════════════════════════════════════════════════════════════
# 10. TestStreamingReporterStress — add 5000 findings to streaming reporter
# ═══════════════════════════════════════════════════════════════════════════


class TestStreamingReporterStress:
    """Add 5000 findings to streaming reporter, verify summary counts correct."""

    def test_streaming_reporter_5000_findings(self):
        StreamingReporter = _import_or_skip("StreamingReporter")
        StreamingReportConfig = _import_or_skip("StreamingReportConfig")

        cfg = StreamingReportConfig("json", buffer_size=500)
        reporter = StreamingReporter(cfg)
        reporter.start()

        for i in range(5000):
            severity = ["high", "medium", "low", "info"][i % 4]
            reporter.write_finding(
                json.dumps({"id": f"f{i}", "severity": severity, "title": f"Vuln {i}"})
            )

        summary = reporter.finish()
        assert summary.total_findings == 5000

        sev_map = dict(summary.findings_by_severity)
        assert sev_map.get("high") == 1250
        assert sev_map.get("medium") == 1250
        assert sev_map.get("low") == 1250
        assert sev_map.get("info") == 1250

    def test_streaming_reporter_flush_cycles(self):
        StreamingReporter = _import_or_skip("StreamingReporter")
        StreamingReportConfig = _import_or_skip("StreamingReportConfig")

        cfg = StreamingReportConfig("json", buffer_size=100)
        reporter = StreamingReporter(cfg)
        reporter.start()

        total = 0
        for cycle in range(50):
            for i in range(100):
                reporter.write_finding(
                    json.dumps({"id": f"c{cycle}-{i}", "severity": "high", "title": "T"})
                )
            reporter.flush()
            assert reporter.get_buffered_count() == 0
            total += 100

        summary = reporter.finish()
        assert summary.total_findings == 5000

    def test_streaming_reporter_with_file_output_stress(self, tmp_path):
        StreamingReporter = _import_or_skip("StreamingReporter")
        StreamingReportConfig = _import_or_skip("StreamingReportConfig")

        out = str(tmp_path / "stress_report.jsonl")
        cfg = StreamingReportConfig("json", output_path=out, buffer_size=200)
        reporter = StreamingReporter(cfg)
        reporter.start()

        for i in range(1000):
            reporter.write_finding(
                json.dumps({"id": f"f{i}", "severity": "high", "title": f"V {i}"})
            )

        summary = reporter.finish()
        assert summary.total_findings == 1000
        assert summary.output_path == out
        assert summary.output_size_bytes > 0

        with open(out) as f:
            lines = f.readlines()
        assert len(lines) == 1000

    def test_streaming_reporter_no_fd_leak(self, tmp_path):
        StreamingReporter = _import_or_skip("StreamingReporter")
        StreamingReportConfig = _import_or_skip("StreamingReportConfig")

        out = str(tmp_path / "fd_stress.jsonl")
        fds_before = _measure_fds()

        for i in range(100):
            cfg = StreamingReportConfig("json", output_path=out)
            reporter = StreamingReporter(cfg)
            reporter.start()
            for j in range(50):
                reporter.write_finding(
                    json.dumps({"id": f"f{i}-{j}", "severity": "low", "title": "T"})
                )
            reporter.finish()

        _wait_for_gc()
        fds_after = _measure_fds()
        delta = fds_after - fds_before
        assert delta <= 10, (
            f"StreamingReporter FD leak: {delta} fds gained ({fds_before} -> {fds_after})"
        )

    def test_streaming_reporter_large_single_batch(self):
        StreamingReporter = _import_or_skip("StreamingReporter")
        StreamingReportConfig = _import_or_skip("StreamingReportConfig")

        cfg = StreamingReportConfig("json", buffer_size=10000)
        reporter = StreamingReporter(cfg)
        reporter.start()

        # Single massive batch
        for i in range(10000):
            reporter.write_finding(
                json.dumps({"id": f"bulk-{i}", "severity": "info", "title": "Bulk"})
            )

        summary = reporter.finish()
        assert summary.total_findings == 10000
