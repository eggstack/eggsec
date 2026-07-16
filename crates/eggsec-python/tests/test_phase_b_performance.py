"""Phase B performance benchmarks for registry and dispatch operations.

Measures the overhead introduced by the registry-convergent dispatch
architecture (Release 5 Phase B).  Each test targets a specific
Phase B operation and asserts it stays within a generous budget.

Budgets are deliberately loose (3x headroom) to avoid flaky CI.
Tighten after establishing a stable baseline.
"""

from __future__ import annotations

import time

import pytest

import eggsec
from eggsec import Engine, OperationRequest, Scope


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _bench(fn, warmup=5, iterations=200, label=""):
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


# Budgets in seconds — generous to avoid flaky CI
BUDGETS = {
    # B6: Generated inventories / registry iteration
    "registry_construction": 0.005,   # 5 ms — all_descriptors() or equivalent
    "descriptor_lookup": 0.001,       # 1 ms per lookup
    "operation_listing": 0.005,       # 5 ms — list_operations()
    # B5: Dispatch lifecycle overhead
    "request_normalization": 0.002,   # 2 ms — OperationRequest construction + metadata
    "no_op_denied_dispatch": 0.010,   # 10 ms — full dispatch for out-of-scope target
    "dispatch_overhead_sync": 0.015,  # 15 ms — Engine.run() scope-denied path
    "dispatch_overhead_async": 0.015, # 15 ms — AsyncEngine async dispatch overhead
}


# ---------------------------------------------------------------------------
# A. Registry construction (B6)
# ---------------------------------------------------------------------------

class TestRegistryConstruction:
    """Measure OperationExecutorDescriptor registry construction time."""

    def test_registry_construction(self):
        """Time how long it takes to build the full descriptor table."""
        budget = BUDGETS["registry_construction"]
        avg, _ = _bench(
            lambda: eggsec.OperationRegistry.all_operations(),
            warmup=10,
            iterations=500,
            label="registry_construction",
        )
        _assert_within_budget(avg, budget, "registry_construction")


# ---------------------------------------------------------------------------
# B. Descriptor lookup (B6)
# ---------------------------------------------------------------------------

class TestDescriptorLookup:
    """Measure per-operation descriptor lookup time."""

    @pytest.mark.parametrize(
        "op_id",
        [
            "scan_ports",
            "scan_endpoints",
            "fingerprint_services",
            "recon_dns",
            "inspect_tls",
            "detect_technology",
            "detect_waf",
            "validate_waf",
            "fuzz_http",
            "load_test",
            "scan_git_secrets",
            "generate_sbom",
            "run_consolidated_recon",
            "graphql_test",
            "oauth_test",
            "auth_test",
            "db_probe",
            "nse_run",
            "scan_docker_image",
            "scan_kubernetes",
            "analyze_apk",
            "analyze_ipa",
        ],
    )
    def test_descriptor_lookup(self, op_id):
        """Time OperationRegistry.find() for each stable operation."""
        budget = BUDGETS["descriptor_lookup"]
        avg, _ = _bench(
            lambda: eggsec.OperationRegistry.find(op_id),
            warmup=10,
            iterations=500,
            label=f"descriptor_lookup:{op_id}",
        )
        _assert_within_budget(avg, budget, f"descriptor_lookup:{op_id}")


# ---------------------------------------------------------------------------
# C. Operation listing (B6)
# ---------------------------------------------------------------------------

class TestOperationListing:
    """Measure list_operations() / iteration time."""

    def test_operation_listing(self):
        """Time all_operations() enumeration."""
        budget = BUDGETS["operation_listing"]
        avg, _ = _bench(
            lambda: list(eggsec.OperationRegistry.all_operations()),
            warmup=10,
            iterations=500,
            label="operation_listing",
        )
        _assert_within_budget(avg, budget, "operation_listing")


# ---------------------------------------------------------------------------
# D. Request normalization (B5)
# ---------------------------------------------------------------------------

class TestRequestNormalization:
    """Measure OperationRequest construction and metadata overhead."""

    def test_request_construction_minimal(self):
        """Time OperationRequest() with minimal fields."""
        budget = BUDGETS["request_normalization"]
        avg, _ = _bench(
            lambda: OperationRequest("scan_ports", "127.0.0.1"),
            warmup=10,
            iterations=500,
            label="request_construction_minimal",
        )
        _assert_within_budget(avg, budget, "request_construction_minimal")

    def test_request_construction_with_metadata(self):
        """Time OperationRequest() with metadata dict."""
        budget = BUDGETS["request_normalization"]
        meta = {"ports": "1-1024", "concurrency": "10"}
        avg, _ = _bench(
            lambda: OperationRequest("scan_ports", "127.0.0.1", metadata=meta),
            warmup=10,
            iterations=500,
            label="request_construction_with_metadata",
        )
        _assert_within_budget(avg, budget, "request_construction_with_metadata")

    def test_request_to_dict(self):
        """Time OperationRequest.to_dict() serialization."""
        budget = BUDGETS["request_normalization"]
        req = OperationRequest(
            "scan_ports", "127.0.0.1", metadata={"ports": "1-1024"}
        )
        avg, _ = _bench(
            lambda: req.to_dict(),
            warmup=10,
            iterations=500,
            label="request_to_dict",
        )
        _assert_within_budget(avg, budget, "request_to_dict")


# ---------------------------------------------------------------------------
# E. No-op / denied dispatch (B5)
# ---------------------------------------------------------------------------

class TestNoOpDeniedDispatch:
    """Measure full dispatch overhead for a scope-denied operation.

    The engine emits planning/validation/audit events but returns
    quickly with an error result — no network I/O occurs.
    """

    @pytest.fixture
    def denied_engine(self):
        """Engine whose scope excludes 127.0.0.1."""
        scope = Scope.allow_hosts(["example.com"])
        return Engine(scope)

    def test_no_op_denied_dispatch(self, denied_engine):
        """Time a full dispatch that fails scope validation."""
        budget = BUDGETS["no_op_denied_dispatch"]
        req = OperationRequest("scan_ports", "127.0.0.1")

        def _dispatch():
            result = denied_engine.run(req)
            assert not result.is_success()

        avg, _ = _bench(
            _dispatch,
            warmup=5,
            iterations=200,
            label="no_op_denied_dispatch",
        )
        _assert_within_budget(avg, budget, "no_op_denied_dispatch")


# ---------------------------------------------------------------------------
# F. Sync dispatch overhead (B5)
# ---------------------------------------------------------------------------

class TestSyncDispatchOverhead:
    """Measure Engine.run() overhead for scope-denied operations.

    This captures the full pre_dispatch_lifecycle → execute_operation
    → post_dispatch_hooks path, but the operation itself returns
    immediately due to scope denial.
    """

    @pytest.fixture
    def denied_engine(self):
        scope = Scope.allow_hosts(["example.com"])
        return Engine(scope)

    @pytest.mark.parametrize(
        "op_id",
        ["scan_ports", "recon_dns", "detect_waf", "fuzz_http"],
    )
    def test_dispatch_overhead_sync(self, denied_engine, op_id):
        """Time Engine.run() for a scope-denied operation."""
        budget = BUDGETS["dispatch_overhead_sync"]
        req = OperationRequest(op_id, "127.0.0.1")

        def _dispatch():
            result = denied_engine.run(req)
            assert not result.is_success()

        avg, _ = _bench(
            _dispatch,
            warmup=5,
            iterations=200,
            label=f"dispatch_overhead_sync:{op_id}",
        )
        _assert_within_budget(avg, budget, f"dispatch_overhead_sync:{op_id}")


# ---------------------------------------------------------------------------
# G. Async dispatch overhead (B5)
# ---------------------------------------------------------------------------

class TestAsyncDispatchOverhead:
    """Measure async dispatch overhead for scope-denied operations."""

    @pytest.fixture
    def denied_engine(self):
        scope = Scope.allow_hosts(["example.com"])
        return Engine(scope)

    @pytest.mark.parametrize(
        "op_id",
        ["scan_ports", "recon_dns", "detect_waf"],
    )
    def test_dispatch_overhead_async(self, denied_engine, op_id):
        """Time async dispatch for a scope-denied operation."""
        import asyncio

        budget = BUDGETS["dispatch_overhead_async"]

        async def _async_dispatch():
            from eggsec import AsyncEngine

            async_engine = AsyncEngine(Scope.allow_hosts(["example.com"]))
            req = OperationRequest(op_id, "127.0.0.1")
            try:
                await async_engine.run(req)
            except RuntimeError:
                pass  # EnforcementError on scope denial

        async def _bench_async():
            for _ in range(5):
                await _async_dispatch()
            times = []
            for _ in range(100):
                start = time.perf_counter()
                await _async_dispatch()
                elapsed = time.perf_counter() - start
                times.append(elapsed)
            return sum(times) / len(times)

        avg = asyncio.run(_bench_async())
        _assert_within_budget(avg, budget, f"dispatch_overhead_async:{op_id}")
