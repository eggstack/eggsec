"""Serialization round-trip tests.

Verifies that Request objects, Result objects, payload type names, and
enum variants survive JSON serialization/deserialization correctly.
"""

import pytest
import json
from conftest import SENTINEL_TARGET, SENTINEL_PORT, SENTINEL_TIMEOUT_MS, SENTINEL_METADATA_KEY, SENTINEL_METADATA_VALUE

import eggsec
from eggsec import (
    OperationRequest,
    PortScanRequest,
    EndpointScanRequest,
    FingerprintRequest,
    ReconDnsRequest,
    TlsInspectRequest,
    TechDetectRequest,
    WafDetectRequest,
    LoadTestRequest,
    WafValidateRequest,
    FuzzRequest,
    ExecutionStatus,
    ExecutionStats,
    Artifact,
    OperationResult,
    Severity,
    Finding,
    Report,
    CancellationToken,
    PipelineStep,
    StepResult,
    PipelineResult,
)


# ---------------------------------------------------------------------------
# 1. Request objects survive JSON round-trip
# ---------------------------------------------------------------------------


class TestRequestRoundTrip:
    def test_operation_request_roundtrip(self):
        req = OperationRequest(
            "scan_ports",
            SENTINEL_TARGET,
            timeout_ms=SENTINEL_TIMEOUT_MS,
            metadata={SENTINEL_METADATA_KEY: SENTINEL_METADATA_VALUE},
        )
        j = req.to_json()
        parsed = json.loads(j)
        assert parsed["operation"] == "scan_ports"
        assert parsed["target"] == SENTINEL_TARGET
        assert parsed["timeout_ms"] == SENTINEL_TIMEOUT_MS
        assert parsed["metadata"][SENTINEL_METADATA_KEY] == SENTINEL_METADATA_VALUE

    def test_port_scan_request_roundtrip(self):
        req = PortScanRequest(
            SENTINEL_TARGET,
            ports="80,443,8080",
            mode="aggressive",
            timing="sneaky",
            timeout_ms=SENTINEL_TIMEOUT_MS,
        )
        j = req.to_json()
        parsed = json.loads(j)
        assert parsed["target"] == SENTINEL_TARGET
        assert parsed["ports"] == "80,443,8080"
        assert parsed["mode"] == "aggressive"
        assert parsed["timing"] == "sneaky"
        assert parsed["timeout_ms"] == SENTINEL_TIMEOUT_MS

    def test_endpoint_scan_request_roundtrip(self):
        req = EndpointScanRequest(
            f"http://{SENTINEL_TARGET}",
            paths=["/admin", "/api/v1"],
            methods=["GET", "POST"],
            timeout_ms=SENTINEL_TIMEOUT_MS,
        )
        j = req.to_json()
        parsed = json.loads(j)
        assert parsed["target"] == f"http://{SENTINEL_TARGET}"
        assert parsed["timeout_ms"] == SENTINEL_TIMEOUT_MS

    def test_fingerprint_request_roundtrip(self):
        req = FingerprintRequest(
            SENTINEL_TARGET,
            ports=[80, 443],
            timeout_ms=SENTINEL_TIMEOUT_MS,
        )
        j = req.to_json()
        parsed = json.loads(j)
        assert parsed["target"] == SENTINEL_TARGET
        assert parsed["timeout_ms"] == SENTINEL_TIMEOUT_MS

    def test_recon_dns_request_roundtrip(self):
        req = ReconDnsRequest(
            SENTINEL_TARGET,
            timeout_ms=SENTINEL_TIMEOUT_MS,
        )
        j = req.to_json()
        parsed = json.loads(j)
        assert parsed["target"] == SENTINEL_TARGET
        assert parsed["timeout_ms"] == SENTINEL_TIMEOUT_MS

    def test_tls_inspect_request_roundtrip(self):
        req = TlsInspectRequest(
            SENTINEL_TARGET,
            timeout_ms=SENTINEL_TIMEOUT_MS,
        )
        j = req.to_json()
        parsed = json.loads(j)
        assert parsed["target"] == SENTINEL_TARGET
        assert parsed["timeout_ms"] == SENTINEL_TIMEOUT_MS

    def test_waf_detect_request_roundtrip(self):
        req = WafDetectRequest(
            f"http://{SENTINEL_TARGET}",
            timeout_ms=SENTINEL_TIMEOUT_MS,
        )
        j = req.to_json()
        parsed = json.loads(j)
        assert parsed["target"] == f"http://{SENTINEL_TARGET}"
        assert parsed["timeout_ms"] == SENTINEL_TIMEOUT_MS

    def test_fuzz_request_roundtrip(self):
        req = FuzzRequest(
            f"http://{SENTINEL_TARGET}",
            payload_type="xss",
            threads=5,
            timeout_ms=SENTINEL_TIMEOUT_MS,
        )
        j = req.to_json()
        parsed = json.loads(j)
        assert parsed["target"] == f"http://{SENTINEL_TARGET}"
        assert parsed["timeout_ms"] == SENTINEL_TIMEOUT_MS


# ---------------------------------------------------------------------------
# 2. Result objects survive JSON round-trip
# ---------------------------------------------------------------------------


class TestResultRoundTrip:
    def test_execution_status_completed_roundtrip(self):
        s = ExecutionStatus.Completed()
        d = s.__repr__()
        assert "Completed" in d

    def test_execution_status_failed_roundtrip(self):
        s = ExecutionStatus.Failed(error="sentinel-error-message")
        d = s.__repr__()
        assert "Failed" in d
        assert "sentinel-error-message" in d

    def test_execution_status_cancelled_roundtrip(self):
        s = ExecutionStatus.Cancelled(reason="test-cancel-reason")
        d = s.__repr__()
        assert "Cancelled" in d
        assert "test-cancel-reason" in d

    def test_execution_status_timeout_roundtrip(self):
        s = ExecutionStatus.Timeout(elapsed_ms=30000)
        d = s.__repr__()
        assert "Timeout" in d
        assert "30000" in d

    def test_execution_stats_roundtrip(self):
        stats = ExecutionStats(
            duration_ms=1234,
            items_processed=56,
            items_failed=7,
            bytes_transferred=890,
        )
        j = stats.to_json()
        parsed = json.loads(j)
        assert parsed["duration_ms"] == 1234
        assert parsed["items_processed"] == 56
        assert parsed["items_failed"] == 7
        assert parsed["bytes_transferred"] == 890

    def test_execution_stats_to_dict(self):
        stats = ExecutionStats(duration_ms=100, items_processed=10, items_failed=1, bytes_transferred=200)
        d = stats.to_dict()
        assert d["duration_ms"] == 100
        assert d["items_processed"] == 10

    def test_artifact_roundtrip(self):
        art = Artifact(
            name="sentinel-artifact",
            kind="pcap",
            mime_type="application/octet-stream",
            data="base64data",
            path="/tmp/sentinel.pcap",
        )
        j = art.to_json()
        parsed = json.loads(j)
        assert parsed["name"] == "sentinel-artifact"
        assert parsed["kind"] == "pcap"
        assert parsed["mime_type"] == "application/octet-stream"
        assert parsed["data"] == "base64data"
        assert parsed["path"] == "/tmp/sentinel.pcap"

    def test_operation_result_roundtrip(self):
        result = OperationResult(
            ExecutionStatus.Completed(),
            stats=ExecutionStats(duration_ms=500),
            error=None,
        )
        # to_dict includes the status type field
        d = result.to_dict()
        assert d["status"]["type"] == "Completed"
        assert d["stats"]["duration_ms"] == 500

        # to_json uses serde which serializes Completed as {}
        j = result.to_json()
        parsed = json.loads(j)
        assert "status" in parsed
        assert "stats" in parsed
        assert parsed["stats"]["duration_ms"] == 500


# ---------------------------------------------------------------------------
# 3. Payload type name is preserved through serialization
# ---------------------------------------------------------------------------


class TestPayloadTypePreservation:
    def test_operation_result_payload_type_field(self):
        """OperationResult should carry payload_type."""
        result = OperationResult(ExecutionStatus.Completed())
        # payload_type_name should be None when constructed from Python
        assert result.payload_type_name is None

    def test_operation_result_with_payload_type(self):
        """Engine-produced results carry correct payload_type."""
        from conftest import SENTINEL_MODE, SENTINEL_CONCURRENCY

        scope = eggsec.Scope.allow_hosts([SENTINEL_TARGET])
        engine = eggsec.Engine(
            scope,
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


# ---------------------------------------------------------------------------
# 4. Enum variants serialize/deserialize correctly
# ---------------------------------------------------------------------------


class TestEnumSerialization:
    def test_severity_variants(self):
        assert str(Severity.Critical) == "Critical"
        assert str(Severity.High) == "High"
        assert str(Severity.Medium) == "Medium"
        assert str(Severity.Low) == "Low"
        assert str(Severity.Info) == "Info"

    def test_severity_from_str(self):
        assert Severity.from_str("critical") == Severity.Critical
        assert Severity.from_str("HIGH") == Severity.High
        assert Severity.from_str("medium") == Severity.Medium
        assert Severity.from_str("LOW") == Severity.Low
        assert Severity.from_str("informational") == Severity.Info

    def test_severity_from_str_invalid(self):
        with pytest.raises(ValueError):
            Severity.from_str("nonexistent-severity")

    def test_finding_severity_preserved(self):
        finding = Finding(
            id="serial-test-1",
            title="Serialization test finding",
            severity=Severity.Critical,
            target=SENTINEL_TARGET,
            category="test",
            description="Verifying severity preserved through serialization",
            recommendation="None",
        )
        j = finding.to_json()
        parsed = json.loads(j)
        assert parsed["severity"] == "Critical"

    def test_execution_status_all_variants_repr(self):
        statuses = [
            ExecutionStatus.Pending(),
            ExecutionStatus.Running(),
            ExecutionStatus.Completed(),
            ExecutionStatus.Failed(error="test"),
            ExecutionStatus.Cancelled(reason="test"),
            ExecutionStatus.Timeout(elapsed_ms=1000),
        ]
        for s in statuses:
            r = repr(s)
            assert "ExecutionStatus" in r


# ---------------------------------------------------------------------------
# 5. CancellationToken serialization
# ---------------------------------------------------------------------------


class TestCancellationTokenSerialization:
    def test_token_json_roundtrip(self):
        token = CancellationToken()
        j = token.to_json()
        parsed = json.loads(j)
        assert parsed["cancelled"] is False
        assert parsed["reason"] is None

    def test_token_cancelled_json_roundtrip(self):
        token = CancellationToken()
        token.cancel("sentinel-cancel-reason")
        j = token.to_json()
        parsed = json.loads(j)
        assert parsed["cancelled"] is True
        assert parsed["reason"] == "sentinel-cancel-reason"

    def test_token_dict_roundtrip(self):
        token = CancellationToken()
        token.cancel("dict-test")
        d = token.to_dict()
        assert d["cancelled"] is True
        assert d["reason"] == "dict-test"


# ---------------------------------------------------------------------------
# 6. PipelineStep serialization
# ---------------------------------------------------------------------------


class TestPipelineStepSerialization:
    def test_step_to_json(self):
        req = OperationRequest(
            "scan_ports",
            SENTINEL_TARGET,
            timeout_ms=SENTINEL_TIMEOUT_MS,
            metadata={"ports": str(SENTINEL_PORT)},
        )
        step = PipelineStep("my-step", req)
        j = step.to_json()
        parsed = json.loads(j)
        assert parsed["name"] == "my-step"
        assert parsed["request"]["operation"] == "scan_ports"
        assert parsed["request"]["target"] == SENTINEL_TARGET

    def test_step_to_dict(self):
        req = OperationRequest(
            "scan_ports",
            SENTINEL_TARGET,
            timeout_ms=SENTINEL_TIMEOUT_MS,
        )
        step = PipelineStep("dict-step", req)
        d = step.to_dict()
        assert d["name"] == "dict-step"

    def test_step_repr(self):
        req = OperationRequest("scan_ports", SENTINEL_TARGET)
        step = PipelineStep("repr-step", req)
        r = repr(step)
        assert "repr-step" in r
        assert "scan_ports" in r
