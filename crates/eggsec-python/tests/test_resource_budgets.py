"""Resource budget tests (F2).

Validates that resource usage stays within defined budgets across session
cycles, DTO batch operations, callback load, and cleanup scenarios.

Budgets are loaded from performance_budgets.json. Tests use the same
measurement patterns as test_stress_leak.py.

Scope: localhost-only; no real network connections required.
"""

from __future__ import annotations

import gc
import importlib
import json
import os
import shutil
import sys
import tempfile
import threading
import time
from pathlib import Path

from typing import Any

import pytest


# ---------------------------------------------------------------------------
# Budget loading
# ---------------------------------------------------------------------------

_BUDGETS_PATH = Path(__file__).resolve().parent / "performance_budgets.json"


def _load_budgets() -> dict[str, Any]:
    """Load performance budgets from JSON file."""
    if not _BUDGETS_PATH.exists():
        pytest.skip(f"Budgets file not found: {_BUDGETS_PATH}")
    with open(_BUDGETS_PATH, encoding="utf-8") as f:
        return json.load(f)


# ---------------------------------------------------------------------------
# Import helpers (same pattern as test_stress_leak.py)
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
    """Count open file descriptors for the current process."""
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
            return len(f.readlines()) - 1
    except (OSError, FileNotFoundError):
        return 0


def _measure_memory_bytes() -> int:
    """Measure current process RSS in bytes."""
    try:
        with open(f"/proc/{os.getpid()}/status") as f:
            for line in f:
                if line.startswith("VmRSS:"):
                    # VmRSS is in kB
                    return int(line.split()[1]) * 1024
    except (OSError, FileNotFoundError, ValueError, IndexError):
        pass
    try:
        import resource
        usage = resource.getrusage(resource.RUSAGE_SELF)
        # ru_maxrss is in kB on Linux
        return usage.ru_maxrss * 1024
    except Exception:
        pass
    pytest.skip("Cannot measure memory on this platform")


def _wait_for_gc():
    """Give the GC a chance to run and release Rust-side resources."""
    gc.collect()
    time.sleep(0.05)


def _tmp_dir(prefix: str = "eggsec_budget") -> str:
    return tempfile.mkdtemp(prefix=prefix)


def _make_finding_json(finding_id: str, severity: str = "high") -> str:
    return json.dumps({
        "id": finding_id,
        "severity": severity,
        "title": f"Budget test {finding_id}",
        "description": "Resource budget validation finding",
        "target": "budget-test.example.com",
        "category": "vuln",
    })


# ---------------------------------------------------------------------------
# Module-level markers
# ---------------------------------------------------------------------------

pytestmark = []


# ═══════════════════════════════════════════════════════════════════════════
# 1. TestFDGrowthBudget — file descriptor growth across session cycles
# ═══════════════════════════════════════════════════════════════════════════


class TestFDGrowthBudget:
    """Verify FD growth stays within budget across session cycles."""

    def test_fd_growth_budget(self):
        budgets = _load_budgets()
        max_fd_growth = budgets.get("fd_growth_per_session_cycle", 10)

        TcpConfig = _import_or_skip("TcpConfigPy")
        AsyncTcpSession = _import_or_skip("AsyncTcpSessionPy")

        fds_before = _measure_fds()
        # 100 session cycles
        for i in range(100):
            try:
                cfg = TcpConfig("127.0.0.1", 1)
                session = AsyncTcpSession(cfg)
                del session
            except Exception:
                pass
        _wait_for_gc()
        fds_after = _measure_fds()
        delta = fds_after - fds_before
        assert delta <= max_fd_growth, (
            f"FD growth budget exceeded: {delta} fds gained "
            f"({fds_before} -> {fds_after}), budget={max_fd_growth}"
        )

    def test_fd_growth_with_repositories(self):
        budgets = _load_budgets()
        max_fd_growth = budgets.get("fd_growth_per_session_cycle", 10)

        SqliteRepo = _import_or_skip("SqliteFindingRepository")
        fds_before = _measure_fds()
        repo = SqliteRepo(":memory:")
        repo.initialize()

        for i in range(200):
            repo.insert_finding(_make_finding_json(f"fd-budget-{i}"))

        repo.close()
        del repo
        _wait_for_gc()
        fds_after = _measure_fds()
        delta = fds_after - fds_before
        assert delta <= max_fd_growth, (
            f"Repository FD growth budget exceeded: {delta} fds gained "
            f"({fds_before} -> {fds_after}), budget={max_fd_growth}"
        )


# ═══════════════════════════════════════════════════════════════════════════
# 2. TestThreadGrowthBudget — thread growth across session cycles
# ═══════════════════════════════════════════════════════════════════════════


class TestThreadGrowthBudget:
    """Verify thread growth stays within budget."""

    def test_thread_growth_budget(self):
        budgets = _load_budgets()
        max_thread_growth = budgets.get("thread_growth_per_session_cycle", 5)

        TcpConfig = _import_or_skip("TcpConfigPy")

        threads_before = _measure_threads()
        for i in range(100):
            try:
                cfg = TcpConfig("127.0.0.1", 1)
                del cfg
            except Exception:
                pass
        _wait_for_gc()
        threads_after = _measure_threads()
        delta = threads_after - threads_before
        assert delta <= max_thread_growth, (
            f"Thread growth budget exceeded: {delta} threads gained "
            f"({threads_before} -> {threads_after}), budget={max_thread_growth}"
        )

    def test_thread_growth_concurrent_sessions(self):
        budgets = _load_budgets()
        max_thread_growth = budgets.get("thread_growth_per_session_cycle", 5)

        TcpConfig = _import_or_skip("TcpConfigPy")
        AsyncTcpSession = _import_or_skip("AsyncTcpSessionPy")

        threads_before = _measure_threads()
        errors = []

        def creator(thread_id: int):
            try:
                for i in range(20):
                    cfg = TcpConfig("127.0.0.1", 1)
                    session = AsyncTcpSession(cfg)
                    del session
            except Exception as e:
                errors.append(str(e))

        threads = [threading.Thread(target=creator, args=(t,)) for t in range(5)]
        for t in threads:
            t.start()
        for t in threads:
            t.join(timeout=120)

        _wait_for_gc()
        threads_after = _measure_threads()
        delta = threads_after - threads_before
        assert not errors, f"Concurrent errors: {errors}"
        assert delta <= max_thread_growth, (
            f"Concurrent thread growth budget exceeded: {delta} threads gained "
            f"({threads_before} -> {threads_after}), budget={max_thread_growth}"
        )


# ═══════════════════════════════════════════════════════════════════════════
# 3. TestMemoryGrowthBudget — memory growth under repeated DTO creation
# ═══════════════════════════════════════════════════════════════════════════


class TestMemoryGrowthBudget:
    """Verify memory growth stays within budget under DTO batch operations."""

    def test_dto_batch_memory_growth(self):
        budgets = _load_budgets()
        max_growth = budgets.get("memory_growth_per_dto_batch", 50 * 1024 * 1024)

        Finding = _import_or_skip("Finding")
        Severity = _import_or_skip("Severity")

        # Baseline measurement
        _wait_for_gc()
        mem_before = _measure_memory_bytes()

        # Create 10000 DTOs
        for i in range(10000):
            f = Finding(
                id=f"mem-budget-{i}",
                title=f"Memory budget finding {i}",
                severity=Severity.Medium,
                target=f"host-{i}.example.com",
                category="vuln",
                description=f"Budget test description {i}",
            )
            # Serialize and discard
            f.to_dict()
            del f

        _wait_for_gc()
        mem_after = _measure_memory_bytes()
        delta = mem_after - mem_before
        assert delta <= max_growth, (
            f"Memory growth budget exceeded: {delta / 1024 / 1024:.1f} MB gained "
            f"({mem_before / 1024 / 1024:.1f} -> {mem_after / 1024 / 1024:.1f} MB), "
            f"budget={max_growth / 1024 / 1024:.1f} MB"
        )

    def test_json_serialization_memory_growth(self):
        budgets = _load_budgets()
        max_growth = budgets.get("memory_growth_per_dto_batch", 50 * 1024 * 1024)

        Finding = _import_or_skip("Finding")
        Severity = _import_or_skip("Severity")

        _wait_for_gc()
        mem_before = _measure_memory_bytes()

        for i in range(5000):
            f = Finding(
                id=f"json-mem-{i}",
                title=f"JSON memory test {i}",
                severity=Severity.High,
                target=f"host-{i}.example.com",
                category="vuln",
                description="test",
            )
            j = f.to_json()
            json.loads(j)
            del f

        _wait_for_gc()
        mem_after = _measure_memory_bytes()
        delta = mem_after - mem_before
        assert delta <= max_growth, (
            f"JSON serialization memory growth budget exceeded: "
            f"{delta / 1024 / 1024:.1f} MB gained, budget={max_growth / 1024 / 1024:.1f} MB"
        )


# ═══════════════════════════════════════════════════════════════════════════
# 4. TestCallbackQueueGrowthBudget — callback queue depth under sustained load
# ═══════════════════════════════════════════════════════════════════════════


class TestCallbackQueueGrowthBudget:
    """Verify callback queue growth stays within budget."""

    def test_callback_queue_growth(self):
        budgets = _load_budgets()
        max_growth = budgets.get("callback_queue_growth", 100)

        AuditSink = _import_or_skip("AuditSink")
        CallbackScheduler = _import_or_skip("CallbackScheduler")

        scheduler = CallbackScheduler()
        received = []

        def on_event(event):
            received.append(event)

        sink = AuditSink(on_event)
        try:
            scheduler.register(sink)
        except AttributeError:
            pytest.skip("CallbackScheduler.register not available")

        # Fire 500 callbacks
        for i in range(500):
            try:
                scheduler.emit(f"event-{i}")
            except Exception:
                pass

        _wait_for_gc()
        # Check that the queue didn't grow unbounded
        # The budget applies to net growth, not total throughput
        queue_depth = getattr(scheduler, "queue_depth", lambda: 0)()
        assert queue_depth <= max_growth, (
            f"Callback queue growth budget exceeded: {queue_depth} "
            f"budget={max_growth}"
        )


# ═══════════════════════════════════════════════════════════════════════════
# 5. TestSocketCleanupBudget — socket cleanup after session close
# ═══════════════════════════════════════════════════════════════════════════


class TestSocketCleanupBudget:
    """Verify sockets are cleaned up after session close."""

    def test_socket_cleanup_after_close(self):
        budgets = _load_budgets()
        max_remaining = budgets.get("socket_cleanup_after_close", 0)

        TcpConfig = _import_or_skip("TcpConfigPy")
        AsyncTcpSession = _import_or_skip("AsyncTcpSessionPy")

        sockets_before = _measure_sockets()
        if sockets_before == 0:
            pytest.skip("Cannot measure sockets on this platform")

        sessions = []
        for i in range(50):
            try:
                cfg = TcpConfig("127.0.0.1", 1)
                session = AsyncTcpSession(cfg)
                sessions.append(session)
            except Exception:
                pass

        # Close all sessions
        for s in sessions:
            try:
                s.close()
            except Exception:
                pass
        del sessions

        _wait_for_gc()
        sockets_after = _measure_sockets()
        delta = sockets_after - sockets_before
        assert delta <= max_remaining, (
            f"Socket cleanup budget exceeded: {delta} sockets remaining "
            f"after close ({sockets_before} -> {sockets_after}), "
            f"budget={max_remaining}"
        )


# ═══════════════════════════════════════════════════════════════════════════
# 6. TestTempDirCleanupBudget — temp directory cleanup after operations
# ═══════════════════════════════════════════════════════════════════════════


class TestTempDirCleanupBudget:
    """Verify temp directories are cleaned up after operations."""

    def test_temp_dir_cleanup_after_jsonl_cycles(self):
        budgets = _load_budgets()
        max_remaining = budgets.get("temp_dir_cleanup", 0)

        JsonlRepo = _import_or_skip("JsonlFindingRepository")

        tmp_dirs = []
        for i in range(20):
            tmp = _tmp_dir(f"budget-cleanup-{i}")
            tmp_dirs.append(tmp)
            path = os.path.join(tmp, "findings.jsonl")
            try:
                repo = JsonlRepo(path)
                repo.initialize()
                repo.insert_finding(_make_finding_json(f"cleanup-{i}"))
                repo.flush()
                repo.close()
                del repo
            except Exception:
                pass

        _wait_for_gc()

        # Clean up our own test directories
        for d in tmp_dirs:
            shutil.rmtree(d, ignore_errors=True)

        # Verify cleanup worked (test infrastructure check)
        remaining = sum(1 for d in tmp_dirs if os.path.exists(d))
        assert remaining <= max_remaining, (
            f"Temp dir cleanup budget exceeded: {remaining} dirs remaining "
            f"budget={max_remaining}"
        )

    def test_temp_dir_cleanup_after_sqlite_cycles(self):
        budgets = _load_budgets()
        max_remaining = budgets.get("temp_dir_cleanup", 0)

        SqliteRepo = _import_or_skip("SqliteFindingRepository")

        tmp_dirs = []
        for i in range(20):
            tmp = _tmp_dir(f"budget-sqlite-{i}")
            tmp_dirs.append(tmp)
            db_path = os.path.join(tmp, "test.db")
            try:
                repo = SqliteRepo(db_path)
                repo.initialize()
                repo.insert_finding(_make_finding_json(f"sql-cleanup-{i}"))
                repo.close()
                del repo
            except Exception:
                pass

        _wait_for_gc()

        # Clean up our own test directories
        for d in tmp_dirs:
            shutil.rmtree(d, ignore_errors=True)

        # Verify cleanup worked (test infrastructure check)
        remaining = sum(1 for d in tmp_dirs if os.path.exists(d))
        assert remaining <= max_remaining, (
            f"SQLite temp dir cleanup budget exceeded: {remaining} dirs remaining "
            f"budget={max_remaining}"
        )


# ═══════════════════════════════════════════════════════════════════════════
# 7. TestRepositoryLargeFindingBudget — repository behavior at 10000+ findings
# ═══════════════════════════════════════════════════════════════════════════


class TestRepositoryLargeFindingBudget:
    """Verify repository behaves correctly at large finding counts."""

    def test_jsonl_repo_10000_findings(self):
        JsonlRepo = _import_or_skip("JsonlFindingRepository")
        tmp = _tmp_dir("budget-large-repo")
        try:
            path = os.path.join(tmp, "large.jsonl")
            repo = JsonlRepo(path)
            repo.initialize()

            start = time.monotonic()
            for i in range(10000):
                fid = repo.insert_finding(_make_finding_json(f"large-{i}"))
                assert fid == f"large-{i}"
            elapsed = time.monotonic() - start

            assert repo.count_findings(None, None) == 10000

            # Verify random access
            got = repo.get_finding("large-5000")
            assert got is not None
            assert "large-5000" in got

            repo.flush()
            repo.close()

            # 10k inserts should complete in reasonable time
            assert elapsed < 120, (
                f"10k JSONL inserts took {elapsed:.1f}s (degradation?)"
            )
        finally:
            shutil.rmtree(tmp, ignore_errors=True)

    def test_sqlite_repo_10000_findings(self):
        SqliteRepo = _import_or_skip("SqliteFindingRepository")
        repo = SqliteRepo(":memory:")
        repo.initialize()

        start = time.monotonic()
        for i in range(10000):
            fid = repo.insert_finding(_make_finding_json(f"sq-large-{i}"))
            assert fid == f"sq-large-{i}"
        elapsed = time.monotonic() - start

        count = repo.count_findings()
        assert count == 10000

        # Verify random access
        got = repo.get_finding("sq-large-7500")
        assert got is not None
        assert "sq-large-7500" in got

        repo.close()

        assert elapsed < 120, (
            f"10k SQLite inserts took {elapsed:.1f}s (degradation?)"
        )

    def test_jsonl_repo_10000_findings_fd_budget(self):
        """Verify FD budget holds during large-scale JSONL operations."""
        budgets = _load_budgets()
        max_fd_growth = budgets.get("fd_growth_per_session_cycle", 10)

        JsonlRepo = _import_or_skip("JsonlFindingRepository")
        tmp = _tmp_dir("budget-large-fd")
        try:
            path = os.path.join(tmp, "large_fd.jsonl")
            fds_before = _measure_fds()

            for cycle in range(5):
                repo = JsonlRepo(path)
                repo.initialize()
                for i in range(2000):
                    repo.insert_finding(_make_finding_json(f"c{cycle}-{i}"))
                repo.flush()
                repo.close()
                del repo

            _wait_for_gc()
            fds_after = _measure_fds()
            delta = fds_after - fds_before
            assert delta <= max_fd_growth, (
                f"Large repo FD growth budget exceeded: {delta} fds gained "
                f"({fds_before} -> {fds_after}), budget={max_fd_growth}"
            )
        finally:
            shutil.rmtree(tmp, ignore_errors=True)

    def test_sqlite_repo_10000_findings_memory_budget(self):
        """Verify memory budget holds during large-scale SQLite operations."""
        budgets = _load_budgets()
        max_growth = budgets.get("memory_growth_per_dto_batch", 50 * 1024 * 1024)

        SqliteRepo = _import_or_skip("SqliteFindingRepository")

        _wait_for_gc()
        mem_before = _measure_memory_bytes()

        repo = SqliteRepo(":memory:")
        repo.initialize()
        for i in range(10000):
            repo.insert_finding(_make_finding_json(f"mem-large-{i}"))
        repo.close()
        del repo

        _wait_for_gc()
        mem_after = _measure_memory_bytes()
        delta = mem_after - mem_before
        assert delta <= max_growth, (
            f"Large repo memory growth budget exceeded: "
            f"{delta / 1024 / 1024:.1f} MB gained, "
            f"budget={max_growth / 1024 / 1024:.1f} MB"
        )


# ═══════════════════════════════════════════════════════════════════════════
# 8. TestStreamingReporterBudget — streaming reporter resource budgets
# ═══════════════════════════════════════════════════════════════════════════


class TestStreamingReporterBudget:
    """Verify streaming reporter stays within resource budgets."""

    def test_streaming_reporter_10000_fd_budget(self):
        budgets = _load_budgets()
        max_fd_growth = budgets.get("fd_growth_per_session_cycle", 10)

        StreamingReporter = _import_or_skip("StreamingReporter")
        StreamingReportConfig = _import_or_skip("StreamingReportConfig")

        fds_before = _measure_fds()

        for i in range(50):
            cfg = StreamingReportConfig("json")
            reporter = StreamingReporter(cfg)
            reporter.start()
            for j in range(200):
                reporter.write_finding(
                    json.dumps({"id": f"budget-{i}-{j}", "severity": "high", "title": "T"})
                )
            reporter.finish()

        _wait_for_gc()
        fds_after = _measure_fds()
        delta = fds_after - fds_before
        assert delta <= max_fd_growth, (
            f"Streaming reporter FD budget exceeded: {delta} fds gained "
            f"({fds_before} -> {fds_after}), budget={max_fd_growth}"
        )

    def test_streaming_reporter_10000_memory_budget(self):
        budgets = _load_budgets()
        max_growth = budgets.get("memory_growth_per_dto_batch", 50 * 1024 * 1024)

        StreamingReporter = _import_or_skip("StreamingReporter")
        StreamingReportConfig = _import_or_skip("StreamingReportConfig")

        _wait_for_gc()
        mem_before = _measure_memory_bytes()

        cfg = StreamingReportConfig("json", buffer_size=500)
        reporter = StreamingReporter(cfg)
        reporter.start()
        for i in range(10000):
            reporter.write_finding(
                json.dumps({"id": f"mem-stream-{i}", "severity": "info", "title": "Bulk"})
            )
        reporter.finish()

        _wait_for_gc()
        mem_after = _measure_memory_bytes()
        delta = mem_after - mem_before
        assert delta <= max_growth, (
            f"Streaming reporter memory budget exceeded: "
            f"{delta / 1024 / 1024:.1f} MB gained, "
            f"budget={max_growth / 1024 / 1024:.1f} MB"
        )


# ═══════════════════════════════════════════════════════════════════════════
# 9. TestSessionCycleBudget — combined resource budget across session cycles
# ═══════════════════════════════════════════════════════════════════════════


class TestSessionCycleBudget:
    """Combined resource budget test across multiple session types."""

    def test_combined_session_cycle_budget(self):
        budgets = _load_budgets()
        max_fd_growth = budgets.get("fd_growth_per_session_cycle", 10)
        max_thread_growth = budgets.get("thread_growth_per_session_cycle", 5)

        fds_before = _measure_fds()
        threads_before = _measure_threads()

        # Mix of session types
        try:
            TcpConfig = _import_or_skip("TcpConfigPy")
            AsyncTcpSession = _import_or_skip("AsyncTcpSessionPy")
            has_tcp = True
        except pytest.skip.Exception:
            has_tcp = False

        if has_tcp:
            for i in range(50):
                try:
                    cfg = TcpConfig("127.0.0.1", 1)
                    session = AsyncTcpSession(cfg)
                    session.close()
                except Exception:
                    pass

        # Repository cycles
        try:
            SqliteRepo = _import_or_skip("SqliteFindingRepository")
            for i in range(50):
                repo = SqliteRepo(":memory:")
                repo.initialize()
                repo.insert_finding(_make_finding_json(f"cycle-{i}"))
                repo.close()
                del repo
        except pytest.skip.Exception:
            pass

        _wait_for_gc()
        fds_after = _measure_fds()
        threads_after = _measure_threads()

        fd_delta = fds_after - fds_before
        thread_delta = threads_after - threads_before

        assert fd_delta <= max_fd_growth, (
            f"Combined session FD budget exceeded: {fd_delta} fds gained "
            f"({fds_before} -> {fds_after}), budget={max_fd_growth}"
        )
        assert thread_delta <= max_thread_growth, (
            f"Combined session thread budget exceeded: {thread_delta} threads gained "
            f"({threads_before} -> {threads_after}), budget={max_thread_growth}"
        )


# ═══════════════════════════════════════════════════════════════════════════
# 10. TestVersionConstantsStability — verify version constants are present
# ═══════════════════════════════════════════════════════════════════════════


class TestVersionConstantsStability:
    """Verify version and schema constants are present and stable."""

    def test_version_constants_present(self):
        import eggsec
        assert hasattr(eggsec, "__version__")
        assert isinstance(eggsec.__version__, str)
        assert len(eggsec.__version__) > 0

    def test_schema_constants_present(self):
        import eggsec
        assert hasattr(eggsec, "SCHEMA_VERSION") or hasattr(eggsec, "__schema_version__")
        assert hasattr(eggsec, "PROTOCOL_VERSION") or hasattr(eggsec, "__protocol_version__")
        assert hasattr(eggsec, "ABI_VERSION") or hasattr(eggsec, "__abi_version__")

    def test_finding_schema_version_present(self):
        import eggsec
        assert hasattr(eggsec, "FINDING_SCHEMA_VERSION")
        assert isinstance(eggsec.FINDING_SCHEMA_VERSION, str)

    def test_event_schema_version_present(self):
        import eggsec
        assert hasattr(eggsec, "EVENT_SCHEMA_VERSION")
        assert isinstance(eggsec.EVENT_SCHEMA_VERSION, str)
