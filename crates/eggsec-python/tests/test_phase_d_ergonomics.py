"""Phase D ergonomics tests.

Covers context manager protocol for callback/sink classes, ExecutionStatus
methods, enum from_str behavior, serialization round-trips, and closed
resource behavior.
"""

import json

import pytest

import eggsec


# ---------------------------------------------------------------------------
# 1. Context manager protocol for callback/sink classes
# ---------------------------------------------------------------------------

class TestContextManagers:
    def test_audit_sink_context_manager(self):
        events = []
        with eggsec.AuditSink(lambda e: events.append(e)) as sink:
            assert not sink.is_closed
        assert sink.is_closed

    def test_finding_sink_context_manager(self):
        findings = []
        with eggsec.FindingSink(lambda f: findings.append(f)) as sink:
            assert not sink.is_closed
        assert sink.is_closed

    def test_artifact_sink_context_manager(self):
        artifacts = []
        with eggsec.ArtifactSink(lambda a: artifacts.append(a)) as sink:
            assert not sink.is_closed
        assert sink.is_closed

    def test_progress_sink_context_manager(self):
        updates = []
        with eggsec.ProgressSink(lambda p, m: updates.append((p, m))) as sink:
            assert not sink.is_closed
        assert sink.is_closed

    def test_event_consumer_context_manager(self):
        events = []
        with eggsec.EventConsumer(lambda e: events.append(e)) as consumer:
            assert not consumer.is_closed
        assert consumer.is_closed


# ---------------------------------------------------------------------------
# 2. ExecutionStatus methods
# ---------------------------------------------------------------------------

class TestExecutionStatus:
    def test_from_str_valid(self):
        from eggsec import ExecutionStatus
        assert ExecutionStatus.from_str("Pending") is not None
        assert ExecutionStatus.from_str("Completed") is not None
        assert ExecutionStatus.from_str("Failed") is not None

    def test_from_str_invalid(self):
        from eggsec import ExecutionStatus
        with pytest.raises(ValueError):
            ExecutionStatus.from_str("InvalidStatus")

    def test_str_repr(self):
        from eggsec import ExecutionStatus
        status = ExecutionStatus.Completed()
        assert str(status) == "Completed"
        assert "Completed" in repr(status)

    def test_equality(self):
        from eggsec import ExecutionStatus
        assert ExecutionStatus.Pending() == ExecutionStatus.Pending()
        assert ExecutionStatus.Completed() != ExecutionStatus.Failed(error="boom")

    def test_hash(self):
        from eggsec import ExecutionStatus
        s1 = ExecutionStatus.Completed()
        s2 = ExecutionStatus.Completed()
        assert hash(s1) == hash(s2)
        assert len({s1, s2}) == 1


# ---------------------------------------------------------------------------
# 3. Enum from_str behavior
# ---------------------------------------------------------------------------

class TestEnumFromStr:
    def test_confidence_from_str_valid(self):
        from eggsec import Confidence
        assert Confidence.from_str("high") is not None
        assert Confidence.from_str("medium") is not None

    def test_confidence_from_str_invalid(self):
        from eggsec import Confidence
        with pytest.raises(ValueError):
            Confidence.from_str("invalid")

    def test_finding_type_from_str_invalid(self):
        from eggsec import FindingType
        with pytest.raises(ValueError):
            FindingType.from_str("invalid")

    def test_evidence_kind_from_str_invalid(self):
        from eggsec import EvidenceKind
        with pytest.raises(ValueError):
            EvidenceKind.from_str("invalid")


# ---------------------------------------------------------------------------
# 4. Serialization round-trip (from_dict / from_json)
# ---------------------------------------------------------------------------

class TestSerializationRoundTrip:
    def test_operation_error_roundtrip(self):
        from eggsec import OperationError
        err = OperationError(kind="network", code="timeout", message="Connection timed out")
        d = err.to_dict()
        restored = OperationError.from_dict(d)
        assert restored.kind == "network"
        assert restored.code == "timeout"
        assert restored.message == "Connection timed out"

    def test_operation_error_json_roundtrip(self):
        from eggsec import OperationError
        err = OperationError(kind="scope", code="denied", message="Out of scope")
        j = err.to_json()
        restored = OperationError.from_json(j)
        assert restored.kind == "scope"
        assert restored.message == "Out of scope"

    def test_execution_stats_roundtrip(self):
        from eggsec import ExecutionStats
        stats = ExecutionStats(duration_ms=100, items_processed=50, items_failed=2, bytes_transferred=4096)
        d = stats.to_dict()
        restored = ExecutionStats.from_dict(d)
        assert restored.duration_ms == 100
        assert restored.items_processed == 50

    def test_artifact_roundtrip(self):
        from eggsec import Artifact
        art = Artifact(name="report.json", kind="report", mime_type="application/json")
        d = art.to_dict()
        restored = Artifact.from_dict(d)
        assert restored.name == "report.json"
        assert restored.kind == "report"

    def test_from_json_invalid(self):
        from eggsec import OperationError
        with pytest.raises(ValueError):
            OperationError.from_json("not valid json")


# ---------------------------------------------------------------------------
# 5. Closed resource behavior
# ---------------------------------------------------------------------------

class TestClosedResourceBehavior:
    def test_send_after_close_sink(self):
        sink = eggsec.AuditSink(lambda e: None)
        sink.close()
        assert sink.is_closed

    def test_double_close_idempotent(self):
        sink = eggsec.AuditSink(lambda e: None)
        sink.close()
        sink.close()
        assert sink.is_closed
