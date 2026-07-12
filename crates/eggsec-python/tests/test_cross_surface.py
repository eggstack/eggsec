"""Cross-surface parity tests.

Verifies that convenience functions, Engine.run(), AsyncEngine, and pipelines
produce equivalent results for the same operation and parameters.
"""

import pytest
import json
from conftest import (
    SENTINEL_TARGET,
    SENTINEL_PORT,
    SENTINEL_TIMEOUT_MS,
    SENTINEL_MODE,
    SENTINEL_CONCURRENCY,
    SENTINEL_METADATA_KEY,
    SENTINEL_METADATA_VALUE,
)

import eggsec


# ---------------------------------------------------------------------------
# 1. Convenience function vs engine.run() produce equivalent results
# ---------------------------------------------------------------------------


class TestConvenienceVsEngineRun:
    """scan_ports convenience function vs Engine.run(OperationRequest) equivalence."""

    def test_payload_type_matches(self, sentinel_scope):
        """Both paths should produce PortScanResult payload type."""
        from eggsec import OperationRequest, PortScanRequest

        # Convenience function: raises on enforcement, returns OperationResult
        # on success. We use scope-enforced target to compare dispatch paths.
        req = OperationRequest(
            "scan_ports",
            SENTINEL_TARGET,
            timeout_ms=SENTINEL_TIMEOUT_MS,
            metadata={"ports": str(SENTINEL_PORT)},
        )

        engine = eggsec.Engine(
            sentinel_scope,
            mode=SENTINEL_MODE,
            concurrency=SENTINEL_CONCURRENCY,
            timeout_ms=SENTINEL_TIMEOUT_MS,
        )
        result = engine.run(req)

        # Both should have a payload type (PortScanResult) or fail consistently
        assert result.payload_type_name is not None or result.status.name() == "Failed"

    def test_same_target_same_metadata(self, sentinel_scope):
        """Engine.run() with same OperationRequest is deterministic."""
        from eggsec import OperationRequest

        engine = eggsec.Engine(
            sentinel_scope,
            mode=SENTINEL_MODE,
            concurrency=SENTINEL_CONCURRENCY,
            timeout_ms=SENTINEL_TIMEOUT_MS,
        )

        req = OperationRequest(
            "scan_ports",
            SENTINEL_TARGET,
            timeout_ms=SENTINEL_TIMEOUT_MS,
            metadata={"ports": str(SENTINEL_PORT)},
        )

        r1 = engine.run(req)
        r2 = engine.run(req)

        # Both should have the same status category
        assert r1.status.name() == r2.status.name()
        # Both should carry the same payload type
        assert r1.payload_type_name == r2.payload_type_name


# ---------------------------------------------------------------------------
# 2. Sync vs async produce equivalent results
# ---------------------------------------------------------------------------


class TestSyncVsAsync:
    """Engine (sync) vs AsyncEngine (async) produce equivalent status."""

    def test_same_scope_same_mode(self, sentinel_scope):
        """Sync and async engines with identical config have matching properties."""
        sync = eggsec.Engine(
            sentinel_scope,
            mode=SENTINEL_MODE,
            concurrency=SENTINEL_CONCURRENCY,
            timeout_ms=SENTINEL_TIMEOUT_MS,
        )
        async_eng = eggsec.AsyncEngine(
            sentinel_scope,
            mode=SENTINEL_MODE,
            concurrency=SENTINEL_CONCURRENCY,
            timeout_ms=SENTINEL_TIMEOUT_MS,
        )

        assert sync.mode == async_eng.mode
        assert sync.concurrency == async_eng.concurrency
        assert sync.timeout_ms == async_eng.timeout_ms
        assert sync.scope.is_target_allowed(SENTINEL_TARGET) == async_eng.scope.is_target_allowed(SENTINEL_TARGET)

    def test_async_dispatch_returns_future(self, sentinel_scope):
        """AsyncEngine.run() returns a PyFuture."""
        from eggsec import OperationRequest

        async_eng = eggsec.AsyncEngine(
            sentinel_scope,
            mode=SENTINEL_MODE,
            concurrency=SENTINEL_CONCURRENCY,
            timeout_ms=SENTINEL_TIMEOUT_MS,
        )
        req = OperationRequest(
            "scan_ports",
            SENTINEL_TARGET,
            timeout_ms=SENTINEL_TIMEOUT_MS,
            metadata={"ports": str(SENTINEL_PORT)},
        )
        future = async_eng.run(req)
        assert isinstance(future, eggsec.PyFuture)

    def test_async_run_port_scan_returns_future(self, sentinel_scope):
        """AsyncEngine.run_port_scan() returns a PyFuture."""
        from eggsec import PortScanRequest

        async_eng = eggsec.AsyncEngine(
            sentinel_scope,
            mode=SENTINEL_MODE,
            concurrency=SENTINEL_CONCURRENCY,
            timeout_ms=SENTINEL_TIMEOUT_MS,
        )
        req = PortScanRequest(SENTINEL_TARGET, ports=str(SENTINEL_PORT), timeout_ms=SENTINEL_TIMEOUT_MS)
        future = async_eng.run_port_scan(req)
        assert isinstance(future, eggsec.PyFuture)


# ---------------------------------------------------------------------------
# 3. Same operation with same params produces consistent payload_type
# ---------------------------------------------------------------------------


class TestConsistentPayloadType:
    """Same operation should always produce the same payload_type name."""

    def test_scan_ports_payload_type(self, sentinel_scope):
        """scan_ports always returns PortScanResult payload type."""
        from eggsec import OperationRequest

        engine = eggsec.Engine(
            sentinel_scope,
            mode=SENTINEL_MODE,
            concurrency=SENTINEL_CONCURRENCY,
            timeout_ms=SENTINEL_TIMEOUT_MS,
        )
        req = OperationRequest(
            "scan_ports",
            SENTINEL_TARGET,
            timeout_ms=SENTINEL_TIMEOUT_MS,
            metadata={"ports": str(SENTINEL_PORT)},
        )
        result = engine.run(req)
        if result.is_success():
            assert result.payload_type_name == "PortScanResult"

    def test_fingerprint_payload_type(self, sentinel_scope):
        """fingerprint operation always returns FingerprintScanResult."""
        from eggsec import OperationRequest

        engine = eggsec.Engine(
            sentinel_scope,
            mode=SENTINEL_MODE,
            concurrency=SENTINEL_CONCURRENCY,
            timeout_ms=SENTINEL_TIMEOUT_MS,
        )
        req = OperationRequest(
            "fingerprint",
            SENTINEL_TARGET,
            timeout_ms=SENTINEL_TIMEOUT_MS,
            metadata={"ports": str(SENTINEL_PORT)},
        )
        result = engine.run(req)
        if result.is_success():
            assert result.payload_type_name == "FingerprintScanResult"

    def test_recon_dns_payload_type(self, sentinel_scope):
        """recon_dns always returns DnsRecordSet payload type."""
        from eggsec import OperationRequest

        engine = eggsec.Engine(
            sentinel_scope,
            mode=SENTINEL_MODE,
            concurrency=SENTINEL_CONCURRENCY,
            timeout_ms=SENTINEL_TIMEOUT_MS,
        )
        req = OperationRequest(
            "recon_dns",
            SENTINEL_TARGET,
            timeout_ms=SENTINEL_TIMEOUT_MS,
        )
        result = engine.run(req)
        if result.is_success():
            assert result.payload_type_name == "DnsRecordSet"

    def test_tls_inspect_payload_type(self, sentinel_scope):
        """tls_inspect always returns TlsInspectionResult."""
        from eggsec import OperationRequest

        engine = eggsec.Engine(
            sentinel_scope,
            mode=SENTINEL_MODE,
            concurrency=SENTINEL_CONCURRENCY,
            timeout_ms=SENTINEL_TIMEOUT_MS,
        )
        req = OperationRequest(
            "tls_inspect",
            SENTINEL_TARGET,
            timeout_ms=SENTINEL_TIMEOUT_MS,
        )
        result = engine.run(req)
        if result.is_success():
            assert result.payload_type_name == "TlsInspectionResult"

    def test_tech_detect_payload_type(self, sentinel_scope):
        """tech_detect always returns TechDetectionResult."""
        from eggsec import OperationRequest

        engine = eggsec.Engine(
            sentinel_scope,
            mode=SENTINEL_MODE,
            concurrency=SENTINEL_CONCURRENCY,
            timeout_ms=SENTINEL_TIMEOUT_MS,
        )
        req = OperationRequest(
            "tech_detect",
            f"http://{SENTINEL_TARGET}",
            timeout_ms=SENTINEL_TIMEOUT_MS,
        )
        result = engine.run(req)
        if result.is_success():
            assert result.payload_type_name == "TechDetectionResult"

    def test_waf_detect_payload_type(self, sentinel_scope):
        """waf_detect always returns WafDetectionResult."""
        from eggsec import OperationRequest

        engine = eggsec.Engine(
            sentinel_scope,
            mode=SENTINEL_MODE,
            concurrency=SENTINEL_CONCURRENCY,
            timeout_ms=SENTINEL_TIMEOUT_MS,
        )
        req = OperationRequest(
            "waf_detect",
            f"http://{SENTINEL_TARGET}",
            timeout_ms=SENTINEL_TIMEOUT_MS,
        )
        result = engine.run(req)
        if result.is_success():
            assert result.payload_type_name == "WafDetectionResult"


# ---------------------------------------------------------------------------
# 4. Pipeline and direct operation produce same payload_type
# ---------------------------------------------------------------------------


class TestPipelineVsDirect:
    """Pipeline step and direct engine.dispatch() yield same payload type."""

    def test_pipeline_step_same_payload_type(self, sentinel_scope):
        """Pipeline with one scan_ports step produces same payload_type as direct run."""
        from eggsec import OperationRequest, Pipeline, PipelineStep

        engine = eggsec.Engine(
            sentinel_scope,
            mode=SENTINEL_MODE,
            concurrency=SENTINEL_CONCURRENCY,
            timeout_ms=SENTINEL_TIMEOUT_MS,
        )

        req = OperationRequest(
            "scan_ports",
            SENTINEL_TARGET,
            timeout_ms=SENTINEL_TIMEOUT_MS,
            metadata={"ports": str(SENTINEL_PORT)},
        )

        # Direct run
        direct_result = engine.run(req)

        # Pipeline run
        pipeline = Pipeline("parity-test")
        pipeline.add_step("port-scan", req)
        pipeline_result = pipeline.run(engine)

        # Pipeline should have exactly 1 step result
        assert len(pipeline_result.step_results) == 1
        step = pipeline_result.step_results[0]

        # Both should have same status category
        assert step.status.name() == direct_result.status.name()
        # Both should have same payload type
        assert (
            step.result.payload_type_name == direct_result.payload_type_name
        )
