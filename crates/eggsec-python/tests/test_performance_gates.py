"""Performance gate tests for eggsec Python bindings (Workstream G11).

Each test measures a performance-sensitive operation against a regression budget.
Tests print timing info and FAIL only when actual time exceeds 3x the budget.
"""

import json
import subprocess
import sys
import time
from pathlib import Path

import pytest


BUDGETS_PATH = Path(__file__).resolve().parent / "performance_budgets.json"


def _load_budgets():
    with open(BUDGETS_PATH) as f:
        return json.load(f)


BUDGETS = _load_budgets()


def _bench(fn, warmup=3, iterations=20, label=""):
    """Run fn for warmup + iterations, return (avg_seconds, all_times)."""
    for _ in range(warmup):
        fn()
    times = []
    for _ in range(iterations):
        start = time.perf_counter()
        fn()
        elapsed = time.perf_counter() - start
        times.append(elapsed)
    avg = sum(times) / len(times)
    return avg, times


def _assert_within_budget(actual, budget, label, multiplier=3.0):
    """Assert actual <= budget * multiplier, print timing info."""
    threshold = budget * multiplier
    status = "PASS" if actual <= threshold else "FAIL"
    over = ""
    if actual > threshold:
        over = f" (OVER by {actual / budget:.1f}x)"
    print(
        f"  [{status}] {label}: {actual*1000:.2f} ms "
        f"(budget: {budget*1000:.1f} ms, threshold: {threshold*1000:.1f} ms){over}"
    )
    assert actual <= threshold, (
        f"{label}: {actual*1000:.2f} ms exceeds {multiplier}x budget "
        f"({threshold*1000:.1f} ms)"
    )


# ---------------------------------------------------------------------------
# A. Import overhead
# ---------------------------------------------------------------------------

class TestImportOverhead:
    """Measure cold import and attribute access times."""

    def test_cold_import(self):
        budget = BUDGETS["import_cold"]
        code = "import eggsec"
        avg, _ = _bench(
            lambda: subprocess.run(
                [sys.executable, "-c", code],
                capture_output=True,
                timeout=10,
            ),
            warmup=1,
            iterations=3,
            label="cold_import",
        )
        _assert_within_budget(avg, budget, "cold_import")

    def test_attribute_access(self):
        budget = BUDGETS["import_attribute"]
        avg, _ = _bench(
            lambda: subprocess.run(
                [
                    sys.executable,
                    "-c",
                    "from eggsec import Engine, Scope, scan_ports",
                ],
                capture_output=True,
                timeout=10,
            ),
            warmup=1,
            iterations=5,
            label="attribute_access",
        )
        _assert_within_budget(avg, budget, "attribute_access")


# ---------------------------------------------------------------------------
# B. Engine creation overhead
# ---------------------------------------------------------------------------

class TestEngineCreationOverhead:
    """Measure Engine() construction time."""

    def test_engine_creation(self):
        import eggsec

        budget = BUDGETS["engine_creation"]
        scope = eggsec.Scope.allow_hosts(["example.com"])
        avg, _ = _bench(
            lambda: eggsec.Engine(scope),
            warmup=5,
            iterations=50,
            label="engine_creation",
        )
        _assert_within_budget(avg, budget, "engine_creation")


# ---------------------------------------------------------------------------
# C. Scope construction overhead
# ---------------------------------------------------------------------------

class TestScopeConstructionOverhead:
    """Measure Scope.allow_hosts() construction time."""

    def test_scope_construction(self):
        import eggsec

        budget = BUDGETS["scope_construction"]
        avg, _ = _bench(
            lambda: eggsec.Scope.allow_hosts(["example.com", "10.0.0.0/8"]),
            warmup=10,
            iterations=100,
            label="scope_construction",
        )
        _assert_within_budget(avg, budget, "scope_construction")


# ---------------------------------------------------------------------------
# D. OperationRegistry query overhead
# ---------------------------------------------------------------------------

class TestRegistryQueryOverhead:
    """Measure OperationRegistry query times."""

    def test_all_operations(self):
        import eggsec

        budget = BUDGETS["registry_all"]
        avg, _ = _bench(
            lambda: eggsec.OperationRegistry.all_operations(),
            warmup=5,
            iterations=50,
            label="registry_all",
        )
        _assert_within_budget(avg, budget, "registry_all")

    def test_find(self):
        import eggsec

        budget = BUDGETS["registry_find"]
        avg, _ = _bench(
            lambda: eggsec.OperationRegistry.find("scan-ports"),
            warmup=5,
            iterations=50,
            label="registry_find",
        )
        _assert_within_budget(avg, budget, "registry_find")


# ---------------------------------------------------------------------------
# E. Serialization overhead
# ---------------------------------------------------------------------------

class TestSerializationOverhead:
    """Measure Finding to_dict() and to_json() times."""

    @pytest.fixture
    def finding(self):
        import eggsec

        return eggsec.Finding(
            id="perf-1",
            title="Performance test finding with a reasonably long title",
            severity=eggsec.Severity.High,
            target="192.168.1.100",
            category="performance-test",
            description=(
                "This is a detailed description of a finding that includes "
                "multiple sentences and enough text to exercise serialization "
                "of a typical finding object with realistic data volumes."
            ),
        )

    def test_finding_to_dict(self, finding):
        budget = BUDGETS["finding_to_dict"]
        avg, _ = _bench(
            lambda: finding.to_dict(),
            warmup=10,
            iterations=100,
            label="finding_to_dict",
        )
        _assert_within_budget(avg, budget, "finding_to_dict")

    def test_finding_to_json(self, finding):
        budget = BUDGETS["finding_to_json"]
        avg, _ = _bench(
            lambda: finding.to_json(),
            warmup=10,
            iterations=100,
            label="finding_to_json",
        )
        _assert_within_budget(avg, budget, "finding_to_json")


# ---------------------------------------------------------------------------
# F. Event creation overhead
# ---------------------------------------------------------------------------

class TestEventCreationOverhead:
    """Measure EventEnvelope creation via wrap_event()."""

    def test_event_envelope(self):
        import eggsec

        budget = BUDGETS["event_envelope"]
        payload = eggsec.PlanningEvent("op-perf", "target.com", "in-scope")
        avg, _ = _bench(
            lambda: eggsec.wrap_event("planning", payload),
            warmup=10,
            iterations=100,
            label="event_envelope",
        )
        _assert_within_budget(avg, budget, "event_envelope")


# ---------------------------------------------------------------------------
# G. Binary buffer overhead
# ---------------------------------------------------------------------------

class TestBinaryBufferOverhead:
    """Measure BinaryBuffer creation and buffer protocol for 1 MB."""

    def test_buffer_1mb(self):
        try:
            from eggsec._core import BinaryBuffer
        except ImportError:
            pytest.skip("BinaryBuffer not available in this build")

        budget = BUDGETS["buffer_1mb"]
        data = b"\x00" * (1024 * 1024)
        avg, _ = _bench(
            lambda: BinaryBuffer(data),
            warmup=5,
            iterations=20,
            label="buffer_1mb",
        )
        _assert_within_budget(avg, budget, "buffer_1mb")


# ---------------------------------------------------------------------------
# H. Collection iteration overhead
# ---------------------------------------------------------------------------

class TestCollectionIterationOverhead:
    """Measure FindingSet iteration with 100 items."""

    def test_finding_set_iteration_100(self):
        import eggsec

        budget = BUDGETS["finding_set_iteration_100"]
        fs = eggsec.FindingSet()
        for i in range(100):
            sev = (
                eggsec.Severity.Critical
                if i % 5 == 0
                else eggsec.Severity.High
                if i % 3 == 0
                else eggsec.Severity.Medium
            )
            fs.add_finding(
                eggsec.Finding(
                    id=f"f-{i}",
                    title=f"Finding {i}",
                    severity=sev,
                    target=f"10.0.0.{i % 256}",
                    category="perf",
                    description=f"Description for finding {i}",
                )
            )

        def iterate_all():
            for f in fs.findings:
                _ = f.to_dict()

        avg, _ = _bench(
            iterate_all,
            warmup=3,
            iterations=20,
            label="finding_set_iteration_100",
        )
        _assert_within_budget(avg, budget, "finding_set_iteration_100")


# ---------------------------------------------------------------------------
# I. API surface query overhead
# ---------------------------------------------------------------------------

class TestApiSurfaceOverhead:
    """Measure api_surface() and feature_matrix() times."""

    def test_api_surface(self):
        import eggsec

        budget = BUDGETS["api_surface"]
        avg, _ = _bench(
            lambda: eggsec.api_surface(),
            warmup=5,
            iterations=50,
            label="api_surface",
        )
        _assert_within_budget(avg, budget, "api_surface")

    def test_feature_matrix(self):
        import eggsec

        budget = BUDGETS["feature_matrix"]
        avg, _ = _bench(
            lambda: eggsec.feature_matrix(),
            warmup=5,
            iterations=50,
            label="feature_matrix",
        )
        _assert_within_budget(avg, budget, "feature_matrix")


# ---------------------------------------------------------------------------
# J. Callback invocation overhead
# ---------------------------------------------------------------------------

class TestCallbackOverhead:
    """Measure EventConsumer callback invocation for 1000 calls.

    AuditSink.send() requires EnforcementAuditEventPy (no Python constructor),
    so we measure EventConsumer.send() with EventEnvelope instead. The callback
    overhead is dominated by the Python->Rust->Python round-trip, which is the
    same path for all sink types.
    """

    def test_callback_1000_calls(self):
        import eggsec

        budget = BUDGETS["callback_1000_calls"]
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

        avg, _ = _bench(
            invoke_1000,
            warmup=3,
            iterations=10,
            label="callback_1000_calls",
        )
        _assert_within_budget(avg, budget, "callback_1000_calls")
        assert call_count > 0, "Callback was not invoked"
        consumer.close()
