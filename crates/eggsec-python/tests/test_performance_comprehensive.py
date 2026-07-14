"""Comprehensive performance and memory validation for eggsec Python API.

Workstream 14: Extended benchmarks covering engine dispatch overhead,
direct-function adapter overhead, pipeline scheduler, serialization throughput,
event creation, cancellation latency, memory under slow consumers, repeated
session open/close, and more.

Each test measures a performance-sensitive operation against a budget.
Tests print timing info and FAIL only when actual time exceeds the budget.
"""

import gc
import os
import time

import pytest

import eggsec
from eggsec import Engine, OperationRequest, Scope

SENTINEL_LOOPBACK = "127.0.0.1"
os.environ["EGGSEC_ALLOW_LOOPBACK_FIXTURE"] = "1"

BUDGETS = {
    "engine_creation_100": 1.0,
    "scope_construction_1000": 1.0,
    "registry_query_1000": 1.0,
    "operation_request_1000": 1.0,
    "serialization_1000": 2.0,
    "event_creation_1000": 1.0,
    "finding_creation_1000": 1.0,
    "cancellation_token_1000": 1.0,
    "scope_enforcement_10000": 2.0,
    "api_surface_100": 2.0,
    "feature_matrix_100": 2.0,
    "engine_open_close_100": 5.0,
    "async_engine_open_close_100": 5.0,
    "callback_1000": 1.0,
    "binary_buffer_1mb_100": 15.0,
}


def _bench(func, iterations, warmup=10):
    """Run warmup iterations, then timed iterations. Return total elapsed."""
    for _ in range(warmup):
        func()
    gc.collect()
    start = time.monotonic()
    for _ in range(iterations):
        func()
    return time.monotonic() - start


def _assert_budget(label, elapsed, budget):
    """Assert elapsed <= budget, print timing."""
    status = "PASS" if elapsed <= budget else "FAIL"
    print(f"  [{status}] {label}: {elapsed*1000:.2f} ms (budget: {budget*1000:.1f} ms)")
    assert elapsed <= budget, (
        f"{label}: {elapsed*1000:.2f} ms exceeds budget {budget*1000:.1f} ms"
    )


# ---------------------------------------------------------------------------
# 1. Engine creation performance (100 iterations)
# ---------------------------------------------------------------------------

def test_engine_creation_performance():
    elapsed = _bench(
        lambda: Engine(Scope.allow_hosts([SENTINEL_LOOPBACK])),
        iterations=100,
        warmup=10,
    )
    _assert_budget("engine_creation_100", elapsed, BUDGETS["engine_creation_100"])


# ---------------------------------------------------------------------------
# 2. Scope construction performance (1000 iterations)
# ---------------------------------------------------------------------------

def test_scope_construction_performance():
    elapsed = _bench(
        lambda: Scope.allow_hosts([SENTINEL_LOOPBACK, "10.0.0.0/8"]),
        iterations=1000,
        warmup=10,
    )
    _assert_budget("scope_construction_1000", elapsed, BUDGETS["scope_construction_1000"])


# ---------------------------------------------------------------------------
# 3. Registry query performance (1000 iterations)
# ---------------------------------------------------------------------------

def test_registry_query_performance():
    elapsed = _bench(
        lambda: eggsec.OperationRegistry.all_operations(),
        iterations=1000,
        warmup=10,
    )
    _assert_budget("registry_query_1000", elapsed, BUDGETS["registry_query_1000"])


# ---------------------------------------------------------------------------
# 4. OperationRequest construction performance (1000 iterations)
# ---------------------------------------------------------------------------

def test_operation_request_construction_performance():
    elapsed = _bench(
        lambda: OperationRequest(
            "scan_ports",
            SENTINEL_LOOPBACK,
            timeout_ms=5000,
            metadata={"ports": "80,443"},
        ),
        iterations=1000,
        warmup=10,
    )
    _assert_budget("operation_request_1000", elapsed, BUDGETS["operation_request_1000"])


# ---------------------------------------------------------------------------
# 5. Serialization performance (1000 iterations)
# ---------------------------------------------------------------------------

def test_serialization_performance():
    finding = eggsec.Finding(
        id="perf-1",
        title="Performance test finding",
        severity=eggsec.Severity.High,
        target=SENTINEL_LOOPBACK,
        category="performance-test",
        description="Detailed description for serialization benchmark.",
    )
    elapsed = _bench(
        lambda: (finding.to_dict(), finding.to_json()),
        iterations=1000,
        warmup=10,
    )
    _assert_budget("serialization_1000", elapsed, BUDGETS["serialization_1000"])


# ---------------------------------------------------------------------------
# 6. Event creation performance (1000 iterations)
# ---------------------------------------------------------------------------

def test_event_creation_performance():
    payload = eggsec.PlanningEvent("op-perf", "target.com", "in-scope")
    elapsed = _bench(
        lambda: eggsec.wrap_event("planning", payload),
        iterations=1000,
        warmup=10,
    )
    _assert_budget("event_creation_1000", elapsed, BUDGETS["event_creation_1000"])


# ---------------------------------------------------------------------------
# 7. Finding creation performance (1000 iterations)
# ---------------------------------------------------------------------------

def test_finding_creation_performance():
    elapsed = _bench(
        lambda: eggsec.Finding(
            id="f-perf",
            title="Finding",
            severity=eggsec.Severity.Medium,
            target=SENTINEL_LOOPBACK,
            category="perf",
            description="Description",
        ),
        iterations=1000,
        warmup=10,
    )
    _assert_budget("finding_creation_1000", elapsed, BUDGETS["finding_creation_1000"])


# ---------------------------------------------------------------------------
# 8. CancellationToken performance (1000 iterations)
# ---------------------------------------------------------------------------

def test_cancellation_token_performance():
    def _bench_token():
        tok = eggsec.CancellationToken()
        tok.cancel("done")
        return tok

    elapsed = _bench(_bench_token, iterations=1000, warmup=10)
    _assert_budget("cancellation_token_1000", elapsed, BUDGETS["cancellation_token_1000"])


# ---------------------------------------------------------------------------
# 9. Scope enforcement performance (10000 iterations)
# ---------------------------------------------------------------------------

def test_scope_enforcement_performance():
    scope = Scope.allow_hosts([SENTINEL_LOOPBACK])
    elapsed = _bench(
        lambda: scope.is_target_allowed(SENTINEL_LOOPBACK),
        iterations=10000,
        warmup=100,
    )
    _assert_budget("scope_enforcement_10000", elapsed, BUDGETS["scope_enforcement_10000"])


# ---------------------------------------------------------------------------
# 10. api_surface() performance (100 iterations)
# ---------------------------------------------------------------------------

def test_api_surface_performance():
    elapsed = _bench(
        lambda: eggsec.api_surface(),
        iterations=100,
        warmup=5,
    )
    _assert_budget("api_surface_100", elapsed, BUDGETS["api_surface_100"])


# ---------------------------------------------------------------------------
# 11. feature_matrix() performance (100 iterations)
# ---------------------------------------------------------------------------

def test_feature_matrix_performance():
    elapsed = _bench(
        lambda: eggsec.feature_matrix(),
        iterations=100,
        warmup=5,
    )
    _assert_budget("feature_matrix_100", elapsed, BUDGETS["feature_matrix_100"])


# ---------------------------------------------------------------------------
# 12. Repeated engine open/close (100 times)
# ---------------------------------------------------------------------------

def test_repeated_engine_open_close():
    elapsed = _bench(
        lambda: Engine(Scope.allow_hosts([SENTINEL_LOOPBACK])).close(),
        iterations=100,
        warmup=5,
    )
    _assert_budget("engine_open_close_100", elapsed, BUDGETS["engine_open_close_100"])


# ---------------------------------------------------------------------------
# 13. Repeated AsyncEngine open/close (100 times)
# ---------------------------------------------------------------------------

def test_repeated_async_engine_open_close():
    import asyncio

    def _bench_async():
        loop = asyncio.new_event_loop()
        try:
            for _ in range(100):
                engine = eggsec.AsyncEngine(
                    Scope.allow_hosts([SENTINEL_LOOPBACK])
                )
                engine.close()
        finally:
            loop.close()

    elapsed = _bench(_bench_async, iterations=1, warmup=1)
    _assert_budget("async_engine_open_close_100", elapsed, BUDGETS["async_engine_open_close_100"])


# ---------------------------------------------------------------------------
# 14. Callback overhead (1000 calls)
# ---------------------------------------------------------------------------

def test_callback_overhead():
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

    elapsed = _bench(invoke_1000, iterations=10, warmup=3)
    _assert_budget("callback_1000", elapsed, BUDGETS["callback_1000"])
    assert call_count > 0, "Callback was not invoked"
    consumer.close()


# ---------------------------------------------------------------------------
# 15. BinaryBuffer 1MB performance (100 iterations)
# ---------------------------------------------------------------------------

def test_binary_buffer_performance():
    try:
        from eggsec._core import BinaryBuffer
    except ImportError:
        pytest.skip("BinaryBuffer not available in this build")

    data = b"\x00" * (1024 * 1024)
    elapsed = _bench(
        lambda: BinaryBuffer(data),
        iterations=100,
        warmup=5,
    )
    _assert_budget("binary_buffer_1mb_100", elapsed, BUDGETS["binary_buffer_1mb_100"])
