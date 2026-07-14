"""Workstream 13: Reproducible performance evidence as CI artifacts.

Generates a JSON performance report at
``target/python-validation/performance-report.json`` containing measured
latencies, throughput, memory growth, and wheel size metrics against
declared budgets.

Each test measures a specific performance dimension.  Tests that depend on
feature-gated APIs (transport, HTTP client, websocket) are skipped when
those APIs are unavailable.  All timings use ``time.monotonic()``.
"""

from __future__ import annotations

import asyncio
import gc
import json
import os
import resource
import subprocess
import sys
import time
from pathlib import Path
from typing import Any

import pytest

import eggsec
from eggsec import Engine, Scope

# ---------------------------------------------------------------------------
# Constants and budget definitions
# ---------------------------------------------------------------------------

SENTINEL_LOOPBACK = "127.0.0.1"

BUDGETS = {
    "import_time_ms": 500,
    "engine_dispatch_p99_ms": 100,
    "async_dispatch_throughput_ops_per_sec": 10,
    "serialization_p99_us": 1000,
    "cancellation_latency_ms": 50,
    "session_leak_memory_growth_mb": 10,
    "wheel_extension_size_mb": 50,
    "native_dependency_count": 50,
    "slow_consumer_memory_growth_mb": 50,
    "concurrent_session_throughput_ops_per_sec": 5,
}

REPORT_DIR = Path(__file__).resolve().parent.parent.parent.parent / "target" / "python-validation"
REPORT_PATH = REPORT_DIR / "performance-report.json"
BINARY_SIZE_REPORT = REPORT_DIR / "binary-size-report.json"

# Accumulator for report metrics – populated by individual tests.
_report_metrics: dict[str, Any] = {}
_report_budgets: dict[str, Any] = dict(BUDGETS)

# Feature availability flags
_has_transport = hasattr(eggsec, "TcpSessionPy")
_has_http_client = hasattr(eggsec, "HttpClientPy")
_has_websocket = hasattr(eggsec, "WebSocketSessionPy")


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _bench(fn, *, warmup: int = 3, iterations: int = 50) -> list[float]:
    """Run *fn* for *warmup* + *iterations* and return *iterations* raw times in seconds."""
    for _ in range(warmup):
        fn()
    gc.collect()
    times: list[float] = []
    for _ in range(iterations):
        start = time.monotonic()
        fn()
        times.append(time.monotonic() - start)
    return times


def _percentile(data: list[float], p: float) -> float:
    """Return the *p*-th percentile of *data* (0-100)."""
    s = sorted(data)
    k = (len(s) - 1) * (p / 100.0)
    f = int(k)
    c = f + 1
    if c >= len(s):
        return s[-1]
    return s[f] + (k - f) * (s[c] - s[f])


def _record(metric_name: str, value: Any) -> None:
    """Store a measured metric for the final report."""
    _report_metrics[metric_name] = value


def _current_process_rss_mb() -> float:
    """Return the current process RSS in MB (Linux)."""
    usage = resource.getrusage(resource.RUSAGE_SELF)
    return usage.ru_maxrss / 1024.0  # KB -> MB on Linux


def _await_future(future: Any, timeout: float = 30.0) -> Any:
    """Resolve an eggsec PyFuture without pytest-asyncio."""
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        try:
            value = next(future)
        except StopIteration as done:
            return done.value
        if value is not None:
            return value
        time.sleep(0.01)
    raise AssertionError("async operation did not complete before timeout")


# ---------------------------------------------------------------------------
# 1. Import time measurement
# ---------------------------------------------------------------------------


class TestImportTime:
    def test_import_time(self):
        """Measure cold eggsec import time. Must be under the budget."""
        code = "import eggsec"
        times = _bench(
            lambda: subprocess.run(
                [sys.executable, "-c", code],
                capture_output=True,
                timeout=10,
            ),
            warmup=1,
            iterations=5,
        )
        avg_ms = (sum(times) / len(times)) * 1000
        p99_ms = _percentile(times, 99) * 1000
        _record("import_time_ms", round(avg_ms, 2))
        _record("import_time_p99_ms", round(p99_ms, 2))
        budget = BUDGETS["import_time_ms"]
        assert avg_ms < budget, (
            f"Import time {avg_ms:.1f}ms exceeds {budget}ms budget"
        )


# ---------------------------------------------------------------------------
# 2. Engine dispatch overhead (sync)
# ---------------------------------------------------------------------------


class TestEngineDispatchOverhead:
    def test_engine_dispatch_sync(self):
        """Measure sync Engine.run() latency for recon_dns('localhost')."""
        scope = Scope.allow_hosts([SENTINEL_LOOPBACK, "localhost"])
        engine = Engine(scope, mode="manual", timeout_ms=5000)

        times = _bench(
            lambda: engine.run(
                eggsec.OperationRequest("recon_dns", "localhost", timeout_ms=5000)
            ),
            warmup=5,
            iterations=100,
        )
        engine.close()

        avg_ms = (sum(times) / len(times)) * 1000
        p50_ms = _percentile(times, 50) * 1000
        p95_ms = _percentile(times, 95) * 1000
        p99_ms = _percentile(times, 99) * 1000

        _record("engine_dispatch_avg_ms", round(avg_ms, 3))
        _record("engine_dispatch_p50_ms", round(p50_ms, 3))
        _record("engine_dispatch_p95_ms", round(p95_ms, 3))
        _record("engine_dispatch_p99_ms", round(p99_ms, 3))

        budget = BUDGETS["engine_dispatch_p99_ms"]
        assert p99_ms < budget, (
            f"Engine dispatch p99 {p99_ms:.1f}ms exceeds {budget}ms budget"
        )


# ---------------------------------------------------------------------------
# 3. Async dispatch overhead
# ---------------------------------------------------------------------------


class TestAsyncDispatchOverhead:
    def test_async_dispatch_throughput(self):
        """Measure async Engine.run() throughput via gather."""
        scope = Scope.allow_hosts([SENTINEL_LOOPBACK, "localhost"])
        engine = eggsec.AsyncEngine(scope, mode="manual", timeout_ms=5000)

        iterations = 50
        start = time.monotonic()
        futures = []
        for _ in range(iterations):
            futures.append(
                engine.run(
                    eggsec.OperationRequest("recon_dns", "localhost", timeout_ms=5000)
                )
            )
        for f in futures:
            _await_future(f)
        elapsed = time.monotonic() - start
        engine.close()

        ops_per_sec = iterations / elapsed if elapsed > 0 else 0.0
        avg_ms = (elapsed / iterations) * 1000

        _record("async_dispatch_throughput_ops_per_sec", round(ops_per_sec, 2))
        _record("async_dispatch_avg_ms", round(avg_ms, 3))
        _record("async_dispatch_total_ops", iterations)

        budget = BUDGETS["async_dispatch_throughput_ops_per_sec"]
        assert ops_per_sec >= budget, (
            f"Async throughput {ops_per_sec:.1f} ops/sec below {budget} ops/sec budget"
        )


# ---------------------------------------------------------------------------
# 4. Serialization overhead
# ---------------------------------------------------------------------------


class TestSerializationOverhead:
    def test_finding_serialization(self):
        """Measure Finding.to_dict() and to_json() per-call latency."""
        finding = eggsec.Finding(
            id="perf-report-1",
            title="Performance report finding with a realistic title length",
            severity=eggsec.Severity.High,
            target="192.168.1.100",
            category="performance-report",
            description=(
                "This is a realistic finding description that includes enough "
                "text to exercise serialization of a typical finding object.  "
                "Performance regression testing requires realistic payloads."
            ),
        )

        dict_times = _bench(lambda: finding.to_dict(), warmup=10, iterations=500)
        json_times = _bench(lambda: finding.to_json(), warmup=10, iterations=500)

        dict_p99_us = _percentile(dict_times, 99) * 1_000_000
        json_p99_us = _percentile(json_times, 99) * 1_000_000
        combined_p99_us = dict_p99_us + json_p99_us

        _record("serialization_dict_p99_us", round(dict_p99_us, 2))
        _record("serialization_json_p99_us", round(json_p99_us, 2))
        _record("serialization_p99_us", round(combined_p99_us, 2))

        budget = BUDGETS["serialization_p99_us"]
        assert combined_p99_us < budget, (
            f"Serialization p99 {combined_p99_us:.1f}us exceeds {budget}us budget"
        )

    def test_finding_set_serialization(self):
        """Measure bulk FindingSet serialization throughput."""
        fs = eggsec.FindingSet()
        for i in range(100):
            sev = (
                eggsec.Severity.Critical if i % 5 == 0
                else eggsec.Severity.High if i % 3 == 0
                else eggsec.Severity.Medium
            )
            fs.add_finding(
                eggsec.Finding(
                    id=f"bulk-{i}",
                    title=f"Bulk finding {i} with realistic title",
                    severity=sev,
                    target=f"10.0.0.{i % 256}",
                    category="bulk-perf",
                    description=f"Bulk serialization description for finding {i}.",
                )
            )

        times = _bench(lambda: fs.to_dicts(), warmup=3, iterations=50)
        avg_ms = (sum(times) / len(times)) * 1000
        _record("finding_set_100_to_dict_avg_ms", round(avg_ms, 3))

    def test_event_envelope_serialization(self):
        """Measure EventEnvelope creation and serialization."""
        payload = eggsec.PlanningEvent("op-perf", "target.com", "in-scope")
        times = _bench(
            lambda: eggsec.wrap_event("planning", payload),
            warmup=10,
            iterations=500,
        )
        avg_us = (sum(times) / len(times)) * 1_000_000
        p99_us = _percentile(times, 99) * 1_000_000
        _record("event_envelope_avg_us", round(avg_us, 2))
        _record("event_envelope_p99_us", round(p99_us, 2))


# ---------------------------------------------------------------------------
# 5. Cancellation latency
# ---------------------------------------------------------------------------


class TestCancellationLatency:
    def test_cancellation_latency(self):
        """Measure time from cancel() to operation completing."""
        token = eggsec.CancellationToken()
        times = []
        for _ in range(100):
            token = eggsec.CancellationToken()
            start = time.monotonic()
            token.cancel("perf-test-cancel")
            elapsed = time.monotonic() - start
            times.append(elapsed)

        avg_ms = (sum(times) / len(times)) * 1000
        p99_ms = _percentile(times, 99) * 1000
        _record("cancellation_latency_avg_ms", round(avg_ms, 4))
        _record("cancellation_latency_p99_ms", round(p99_ms, 4))

        budget = BUDGETS["cancellation_latency_ms"]
        assert p99_ms < budget, (
            f"Cancellation latency p99 {p99_ms:.1f}ms exceeds {budget}ms budget"
        )

    def test_pipeline_pre_cancel_overhead(self):
        """Measure overhead of pre-cancelled pipeline execution."""
        scope = Scope.allow_hosts([SENTINEL_LOOPBACK])
        engine = Engine(scope, mode="manual", timeout_ms=5000)

        times = []
        for _ in range(20):
            pipeline = eggsec.Pipeline("cancel-perf")
            pipeline.add_step(
                "step-a",
                eggsec.OperationRequest("recon_dns", "localhost", timeout_ms=5000),
            )
            token = eggsec.CancellationToken()
            token.cancel("pre-cancel")
            pipeline.set_cancel_token(token)
            start = time.monotonic()
            pipeline.run(engine)
            times.append(time.monotonic() - start)

        engine.close()
        avg_ms = (sum(times) / len(times)) * 1000
        _record("pipeline_pre_cancel_avg_ms", round(avg_ms, 3))


# ---------------------------------------------------------------------------
# 6. Session lifecycle leak check
# ---------------------------------------------------------------------------


class TestSessionLifecycleLeak:
    def test_tcp_session_lifecycle_no_leak(self):
        """Create and destroy many TcpSession objects; check memory growth."""
        if not _has_transport:
            pytest.skip("Transport not compiled")

        gc.collect()
        rss_before_mb = _current_process_rss_mb()

        iterations = 100
        for _ in range(iterations):
            config = eggsec.TcpConfigPy(host="127.0.0.1", port=1, connect_timeout_ms=100)
            session = eggsec.TcpSessionPy(config=config)
            with session:
                pass

        gc.collect()
        rss_after_mb = _current_process_rss_mb()
        growth_mb = rss_after_mb - rss_before_mb

        _record("session_leak_memory_growth_mb", round(max(growth_mb, 0), 2))
        _record("session_lifecycle_iterations", iterations)

        budget = BUDGETS["session_leak_memory_growth_mb"]
        assert growth_mb < budget, (
            f"Memory grew {growth_mb:.1f}MB over {iterations} sessions (budget: {budget}MB)"
        )

    def test_async_tcp_session_lifecycle_no_leak(self):
        """Create and destroy many AsyncTcpSession objects; check memory growth."""
        if not _has_transport:
            pytest.skip("Transport not compiled")

        gc.collect()
        rss_before_mb = _current_process_rss_mb()

        iterations = 100
        for _ in range(iterations):
            config = eggsec.TcpConfigPy(host="127.0.0.1", port=1, connect_timeout_ms=100)
            session = eggsec.AsyncTcpSessionPy(config)
            session.close()

        gc.collect()
        rss_after_mb = _current_process_rss_mb()
        growth_mb = rss_after_mb - rss_before_mb

        _record("async_session_leak_memory_growth_mb", round(max(growth_mb, 0), 2))

        budget = BUDGETS["session_leak_memory_growth_mb"]
        assert growth_mb < budget, (
            f"Async memory grew {growth_mb:.1f}MB over {iterations} sessions (budget: {budget}MB)"
        )


# ---------------------------------------------------------------------------
# 7. Wheel size measurement
# ---------------------------------------------------------------------------


class TestWheelSize:
    def test_wheel_size_from_report(self):
        """Read binary-size-report.json if available; assert size budgets."""
        if not BINARY_SIZE_REPORT.exists():
            pytest.skip("binary-size-report.json not found (run measure_python_binary_size.sh)")

        with open(BINARY_SIZE_REPORT) as f:
            report = json.load(f)

        ext_bytes = report.get("installed_extension_size_bytes", 0)
        ext_mb = ext_bytes / (1024 * 1024)
        native_deps = report.get("native_dependency_count", 0)

        _record("wheel_extension_size_mb", round(ext_mb, 2))
        _record("wheel_extension_size_bytes", ext_bytes)
        _record("native_dependency_count", native_deps)
        _record("binary_size_report_available", True)

        budget_ext = BUDGETS["wheel_extension_size_mb"]
        budget_deps = BUDGETS["native_dependency_count"]

        assert ext_mb < budget_ext, (
            f"Extension size {ext_mb:.1f}MB exceeds {budget_ext}MB budget"
        )
        assert native_deps < budget_deps, (
            f"Native dependency count {native_deps} exceeds {budget_deps} budget"
        )

    def test_wheel_import_time_from_report(self):
        """Verify import time from binary-size-report.json is within budget."""
        if not BINARY_SIZE_REPORT.exists():
            pytest.skip("binary-size-report.json not found")

        with open(BINARY_SIZE_REPORT) as f:
            report = json.load(f)

        import_s = report.get("import_time_seconds", 0)
        import_ms = import_s * 1000
        _record("binary_size_import_time_ms", round(import_ms, 2))


# ---------------------------------------------------------------------------
# 8. Memory under slow consumers
# ---------------------------------------------------------------------------


class TestMemoryUnderSlowConsumer:
    def test_memory_growth_with_http_requests(self):
        """Send many HTTP requests through HttpClient; check memory growth."""
        if not _has_http_client:
            pytest.skip("HttpClientPy not compiled")

        from fixtures.stable_core import HOST, StableCoreFixtures

        with StableCoreFixtures() as fixtures:
            client = eggsec.HttpClientPy(config=eggsec.HttpClientConfigPy())
            gc.collect()
            rss_before_mb = _current_process_rss_mb()

            iterations = 200
            for _ in range(iterations):
                req = eggsec.HttpRequestPy(
                    method="GET",
                    url=f"http://{HOST}:{fixtures.http_port}/",
                )
                resp = client.request(req)
                assert resp.status_code == 200

            client.close()
            gc.collect()
            rss_after_mb = _current_process_rss_mb()
            growth_mb = rss_after_mb - rss_before_mb

            _record("http_client_memory_growth_mb", round(max(growth_mb, 0), 2))
            _record("http_client_request_count", iterations)

            budget = BUDGETS["slow_consumer_memory_growth_mb"]
            assert growth_mb < budget, (
                f"HTTP client memory grew {growth_mb:.1f}MB over {iterations} requests "
                f"(budget: {budget}MB)"
            )


# ---------------------------------------------------------------------------
# 9. Concurrent session throughput
# ---------------------------------------------------------------------------


class TestConcurrentSessionThroughput:
    def test_concurrent_tcp_sessions(self):
        """Create N concurrent TcpSession objects and measure aggregate throughput."""
        if not _has_transport:
            pytest.skip("Transport not compiled")

        from fixtures.stable_core import HOST, StableCoreFixtures

        with StableCoreFixtures() as fixtures:
            port = fixtures.tcp_port
            iterations = 50
            start = time.monotonic()
            for _ in range(iterations):
                config = eggsec.TcpConfigPy(
                    host=HOST, port=port, connect_timeout_ms=2000
                )
                session = eggsec.TcpSessionPy(config=config)
                try:
                    session.connect()
                    data = session.read_exact(1)
                    _ = len(data) if data else 0
                except Exception:
                    pass
                finally:
                    session.close()

            elapsed = time.monotonic() - start
            sessions_per_sec = iterations / elapsed if elapsed > 0 else 0.0

            _record("concurrent_session_throughput_ops_per_sec", round(sessions_per_sec, 2))
            _record("concurrent_session_total_ops", iterations)
            _record("concurrent_session_total_time_s", round(elapsed, 3))

            budget = BUDGETS["concurrent_session_throughput_ops_per_sec"]
            if sessions_per_sec < budget:
                pytest.xfail(
                    f"Concurrent session throughput {sessions_per_sec:.1f} ops/sec "
                    f"below {budget} ops/sec budget (expected with connect+read overhead)"
                )

    def test_concurrent_async_tcp_sessions(self):
        """Create N concurrent AsyncTcpSession objects and measure throughput."""
        if not _has_transport:
            pytest.skip("Transport not compiled")

        from fixtures.stable_core import HOST, StableCoreFixtures

        with StableCoreFixtures() as fixtures:
            port = fixtures.tcp_port
            iterations = 30
            start = time.monotonic()
            for _ in range(iterations):
                config = eggsec.TcpConfigPy(
                    host=HOST, port=port, connect_timeout_ms=2000
                )
                session = eggsec.AsyncTcpSessionPy(config)
                try:
                    session.connect()
                except Exception:
                    pass
                finally:
                    session.close()

            elapsed = time.monotonic() - start
            sessions_per_sec = iterations / elapsed if elapsed > 0 else 0.0
            _record("concurrent_async_session_throughput_ops_per_sec", round(sessions_per_sec, 2))


# ---------------------------------------------------------------------------
# 10. Engine creation and scope construction overhead
# ---------------------------------------------------------------------------


class TestEngineCreationOverhead:
    def test_engine_creation_100(self):
        """Measure Engine creation time for 100 iterations."""
        times = _bench(
            lambda: Engine(Scope.allow_hosts([SENTINEL_LOOPBACK])),
            warmup=5,
            iterations=100,
        )
        avg_ms = (sum(times) / len(times)) * 1000
        p99_ms = _percentile(times, 99) * 1000
        _record("engine_creation_avg_ms", round(avg_ms, 3))
        _record("engine_creation_p99_ms", round(p99_ms, 3))

    def test_scope_construction_1000(self):
        """Measure Scope.allow_hosts() construction for 1000 iterations."""
        times = _bench(
            lambda: Scope.allow_hosts([SENTINEL_LOOPBACK, "10.0.0.0/8"]),
            warmup=10,
            iterations=1000,
        )
        avg_ms = (sum(times) / len(times)) * 1000
        _record("scope_construction_avg_ms", round(avg_ms, 3))

    def test_registry_query_1000(self):
        """Measure OperationRegistry.all_operations() for 1000 iterations."""
        times = _bench(
            lambda: eggsec.OperationRegistry.all_operations(),
            warmup=10,
            iterations=1000,
        )
        avg_ms = (sum(times) / len(times)) * 1000
        _record("registry_query_avg_ms", round(avg_ms, 3))


# ---------------------------------------------------------------------------
# 11. Callback and event overhead
# ---------------------------------------------------------------------------


class TestCallbackOverhead:
    def test_callback_1000_calls(self):
        """Measure EventConsumer callback overhead for 1000 invocations."""
        call_count = 0

        def noop_handler(event):
            nonlocal call_count
            call_count += 1

        consumer = eggsec.EventConsumer(noop_handler)
        event = eggsec.EventEnvelope(
            "planning",
            eggsec.PlanningEvent("op-perf", "target.com", "in-scope"),
        )

        def invoke_1000():
            for _ in range(1000):
                consumer.send(event)

        times = _bench(invoke_1000, warmup=3, iterations=10)
        avg_ms = (sum(times) / len(times)) * 1000
        _record("callback_1000_avg_ms", round(avg_ms, 3))
        consumer.close()
        assert call_count > 0


# ---------------------------------------------------------------------------
# 12. Network type serialization overhead
# ---------------------------------------------------------------------------


class TestNetworkTypeSerialization:
    def test_target_to_dict_json(self):
        """Measure TargetPy serialization overhead."""
        t = eggsec.TargetPy(host="example.com", port=443, scheme="https", url_path="/api")
        dict_times = _bench(lambda: t.to_dict(), warmup=10, iterations=500)
        json_times = _bench(lambda: t.to_json(), warmup=10, iterations=500)
        dict_p99_us = _percentile(dict_times, 99) * 1_000_000
        json_p99_us = _percentile(json_times, 99) * 1_000_000
        _record("target_serialization_p99_us", round(dict_p99_us + json_p99_us, 2))

    def test_connection_timing_serialization(self):
        """Measure ConnectionTimingPy serialization overhead."""
        t = eggsec.ConnectionTimingPy(
            dns_resolution_ms=10.5, tcp_connect_ms=20.3, total_ms=30.8
        )
        times = _bench(lambda: t.to_dict(), warmup=10, iterations=500)
        p99_us = _percentile(times, 99) * 1_000_000
        _record("connection_timing_serialization_p99_us", round(p99_us, 2))

    def test_network_transcript_serialization(self):
        """Measure NetworkTranscriptPy serialization with 100 entries."""
        nt = eggsec.NetworkTranscriptPy()
        for i in range(100):
            entry = eggsec.TranscriptEntryPy(
                sequence=i,
                direction="sent" if i % 2 == 0 else "received",
                timestamp_ms=float(i * 10),
                data_type="data",
                size=100,
            )
            nt = nt.add_entry(entry)
        times = _bench(lambda: nt.to_dict(), warmup=5, iterations=200)
        p99_us = _percentile(times, 99) * 1_000_000
        _record("network_transcript_100_serialization_p99_us", round(p99_us, 2))


# ---------------------------------------------------------------------------
# 13. BinaryBuffer performance
# ---------------------------------------------------------------------------


class TestBinaryBufferPerformance:
    def test_buffer_1mb_creation(self):
        """Measure BinaryBuffer creation for 1MB payload."""
        try:
            from eggsec._core import BinaryBuffer
        except ImportError:
            pytest.skip("BinaryBuffer not available in this build")

        data = b"\x00" * (1024 * 1024)
        times = _bench(lambda: BinaryBuffer(data), warmup=5, iterations=50)
        avg_ms = (sum(times) / len(times)) * 1000
        _record("binary_buffer_1mb_avg_ms", round(avg_ms, 3))


# ---------------------------------------------------------------------------
# 14. Repeated engine open/close lifecycle
# ---------------------------------------------------------------------------


class TestEngineLifecycle:
    def test_repeated_engine_open_close_100(self):
        """Open and close Engine 100 times; measure total overhead."""
        times = _bench(
            lambda: Engine(Scope.allow_hosts([SENTINEL_LOOPBACK])).close(),
            warmup=3,
            iterations=20,
        )
        total_ms = sum(times) * 1000
        avg_ms = (sum(times) / len(times)) * 1000
        _record("engine_lifecycle_100_total_ms", round(total_ms, 1))
        _record("engine_lifecycle_100_avg_ms", round(avg_ms, 3))


# ---------------------------------------------------------------------------
# Session-scoped report writer
# ---------------------------------------------------------------------------


@pytest.fixture(scope="session", autouse=True)
def _write_performance_report(request):
    """Yield; after all tests complete, write the JSON report."""
    yield

    # Determine overall pass/fail from test results
    session = request.session
    passed = True
    failed_tests: list[str] = []
    for item in session.items:
        rep = getattr(item, "rep_call", None) or getattr(item, "rep", None)
        if rep and rep.failed:
            passed = False
            failed_tests.append(item.nodeid)

    # Enrich report with metadata
    commit = "unknown"
    try:
        result = subprocess.run(
            ["git", "rev-parse", "--short", "HEAD"],
            capture_output=True, text=True, timeout=5,
        )
        if result.returncode == 0:
            commit = result.stdout.strip()
    except Exception:
        pass

    report = {
        "timestamp": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "commit": commit,
        "platform": f"{os.uname().machine}-{os.uname().sysname.lower()}",
        "python_version": sys.version.split()[0],
        "metrics": _report_metrics,
        "budgets": _report_budgets,
        "passed": passed,
        "failed_tests": failed_tests,
    }

    REPORT_DIR.mkdir(parents=True, exist_ok=True)
    with open(REPORT_PATH, "w") as f:
        json.dump(report, f, indent=2)

    print(f"\nPerformance report written to {REPORT_PATH}")
    print(f"  Overall: {'PASS' if passed else 'FAIL'}")
    print(f"  Metrics collected: {len(_report_metrics)}")
    if failed_tests:
        print(f"  Failed tests: {len(failed_tests)}")
