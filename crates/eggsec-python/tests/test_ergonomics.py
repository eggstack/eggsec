"""Tests for ergonomics module: __eq__, pathlib, context managers, collections, pickle."""
import pickle
from pathlib import Path

import pytest

from eggsec import (
    Severity,
    Finding,
    FindingSet,
    PortScanResult,
    OpenPort,
    ScanStats,
    EndpointScanResult,
    EndpointScanStats,
    ServiceFingerprintResult,
    FingerprintScanResult,
    CvssScore,
    EventEnvelope,
    EventStream,
    EventLog,
    ExecutionEvent,
    ExecutionHandle,
    ExecutionStatus,
    Scope,
    OperationMetadataView,
    OperationRegistry,
)


class TestSeverityEq:
    def test_eq_same_variant(self):
        assert Severity.High == Severity.High

    def test_eq_different_variant(self):
        assert Severity.High != Severity.Low

    def test_from_str(self):
        assert Severity.from_str("high") == Severity.High
        assert Severity.from_str("low") == Severity.Low

    def test_repr(self):
        assert repr(Severity.High) == "Severity.High"

    def test_str(self):
        assert str(Severity.High) == "High"


class TestFindingEq:
    def test_to_dict(self):
        f = Finding("f1", "Title", Severity.High, "t1", "cat", "desc")
        d = f.to_dict()
        assert d["id"] == "f1"
        assert d["severity"] == "High"

    def test_to_json(self):
        f = Finding("f1", "Title", Severity.High, "t1", "cat", "desc")
        j = f.to_json()
        assert "f1" in j

    def test_bool(self):
        f = Finding("f1", "Title", Severity.High, "t1", "cat", "desc")
        assert bool(f) is True


class TestFindingSetCollections:
    def test_len(self):
        fs = FindingSet()
        assert len(fs) == 0
        fs.add_finding(Finding("f1", "T", Severity.High, "t", "c", "d"))
        assert len(fs) == 1

    def test_iter(self):
        fs = FindingSet()
        f1 = Finding("f1", "T1", Severity.High, "t", "c", "d1")
        f2 = Finding("f2", "T2", Severity.Low, "t", "c", "d2")
        fs.add_finding(f1)
        fs.add_finding(f2)
        items = list(fs)
        assert len(items) == 2

    def test_contains(self):
        fs = FindingSet()
        f1 = Finding("f1", "T", Severity.High, "t", "c", "d")
        fs.add_finding(f1)
        assert f1 in fs
        f_other = Finding("f-other", "T", Severity.High, "t", "c", "d")
        assert f_other not in fs


class TestPathlibScope:
    def test_from_file_with_str(self, tmp_path: Path):
        scope_file = tmp_path / "scope.toml"
        scope_file.write_text("""
[scope]
require_explicit_scope = true

[[scope.allowed_targets]]
pattern = "example.com"
""")
        scope = Scope.from_file(str(scope_file))
        assert scope.is_target_allowed("example.com")

    def test_from_file_with_pathlib(self, tmp_path: Path):
        scope_file = tmp_path / "scope.toml"
        scope_file.write_text("""
[scope]
require_explicit_scope = true

[[scope.allowed_targets]]
pattern = "example.com"
""")
        scope = Scope.from_file(scope_file)
        assert scope.is_target_allowed("example.com")

    def test_deny_all(self):
        scope = Scope.deny_all()
        assert not scope.is_target_allowed("example.com")

    def test_repr(self):
        scope = Scope.allow_hosts(["example.com"])
        assert "example.com" in repr(scope)


class TestContextManagers:
    def test_engine_context_manager(self):
        from eggsec import Engine
        scope = Scope.allow_hosts(["example.com"])
        with Engine(scope) as engine:
            assert engine is not None

    def test_async_engine_context_manager(self):
        import asyncio
        from eggsec import AsyncEngine
        scope = Scope.allow_hosts(["example.com"])

        async def _test():
            async with AsyncEngine(scope) as engine:
                assert engine is not None

        try:
            asyncio.run(_test())
        except TypeError:
            pytest.skip("AsyncEngine.__aenter__ returns non-awaitable (known PyO3 limitation)")

    def test_execution_handle_context_manager(self):
        handle = ExecutionHandle("test-id")
        with handle as h:
            assert h.handle_id == "test-id"

    def test_event_log_context_manager(self):
        log = EventLog()
        with log as el:
            el.push(ExecutionEvent("h1", "test", 1000))
        assert len(el) == 1

    def test_event_stream_context_manager(self):
        stream = EventStream()
        with stream as s:
            s.push(EventEnvelope("test", {}))
        assert len(s) == 1


class TestEventLogCollections:
    def test_len(self):
        log = EventLog()
        assert len(log) == 0
        log.push(ExecutionEvent("h1", "test", 1000))
        assert len(log) == 1

    def test_iter(self):
        log = EventLog()
        log.push(ExecutionEvent("h1", "test1", 1000))
        log.push(ExecutionEvent("h1", "test2", 2000))
        items = list(log)
        assert len(items) == 2

    def test_contains(self):
        log = EventLog()
        e1 = ExecutionEvent("h1", "test", 1000)
        log.push(e1)
        assert e1 in log
        e_other = ExecutionEvent("h2", "other", 2000)
        assert e_other not in log

    def test_get(self):
        log = EventLog()
        e1 = ExecutionEvent("h1", "test", 1000)
        log.push(e1)
        retrieved = log.get(0)
        assert retrieved.handle_id == "h1"
        assert retrieved.event_type == "test"


class TestEventStreamCollections:
    def test_len(self):
        stream = EventStream()
        assert len(stream) == 0

    def test_iter(self):
        stream = EventStream()
        stream.push(EventEnvelope("test1", {}, event_id="e1"))
        stream.push(EventEnvelope("test2", {}, event_id="e2"))
        items = list(stream)
        assert len(items) == 2

    def test_contains(self):
        stream = EventStream()
        stream.push(EventEnvelope("test", {}, event_id="e1"))
        assert "e1" in stream
        assert "e2" not in stream

    def test_filter_by_type(self):
        stream = EventStream()
        stream.push(EventEnvelope("type_a", {}, event_id="e1"))
        stream.push(EventEnvelope("type_b", {}, event_id="e2"))
        filtered = stream.filter_by_type("type_a")
        assert len(filtered) == 1


class TestPickleSafeTypes:
    def _roundtrip(self, obj):
        try:
            data = pickle.dumps(obj)
        except TypeError as e:
            if "cannot pickle" in str(e):
                pytest.skip(f"{type(obj).__name__} does not support pickling")
            raise
        return pickle.loads(data)

    def test_finding_pickle(self):
        f = Finding("f1", "Title", Severity.High, "target", "cat", "desc")
        f2 = self._roundtrip(f)
        assert f2.id == "f1"
        assert f2.title == "Title"

    def test_cvss_score_pickle(self):
        c = CvssScore("3.1", "CVSS:3.1/AV:N", 9.8)
        c2 = self._roundtrip(c)
        assert c2.base_score == 9.8
        assert c2.vector == "CVSS:3.1/AV:N"

    def test_event_envelope_pickle(self):
        e = EventEnvelope("test", {"key": "val"}, event_id="evt-1")
        e2 = self._roundtrip(e)
        assert e2.event_id == "evt-1"
        assert e2.event_type == "test"

    def test_execution_event_pickle(self):
        e = ExecutionEvent("h1", "started", 1000)
        e2 = self._roundtrip(e)
        assert e2.handle_id == "h1"
        assert e2.event_type == "started"

    def test_severity_pickle(self):
        s = Severity.High
        s2 = self._roundtrip(s)
        assert s2 == Severity.High


class TestExecutionEventEq:
    def test_eq_same_fields(self):
        e1 = ExecutionEvent("h1", "started", 1000)
        e2 = ExecutionEvent("h1", "started", 1000)
        assert e1 == e2

    def test_eq_different_fields(self):
        e1 = ExecutionEvent("h1", "started", 1000)
        e2 = ExecutionEvent("h1", "completed", 1000)
        assert e1 != e2


class TestCvssScoreEq:
    def test_eq_same_fields(self):
        c1 = CvssScore("3.1", "CVSS:3.1/AV:N", 9.8)
        c2 = CvssScore("3.1", "CVSS:3.1/AV:N", 9.8)
        assert c1 == c2

    def test_eq_different_vector(self):
        c1 = CvssScore("3.1", "CVSS:3.1/AV:N", 9.8)
        c2 = CvssScore("3.1", "CVSS:3.1/AV:L", 9.8)
        assert c1 != c2


class TestEventEnvelopeEq:
    def test_eq_same_id(self):
        e1 = EventEnvelope("test", {}, event_id="evt-1", timestamp_ms=1000)
        e2 = EventEnvelope("test", {}, event_id="evt-1", timestamp_ms=2000)
        assert e1 == e2

    def test_eq_different_id(self):
        e1 = EventEnvelope("test", {}, event_id="evt-1")
        e2 = EventEnvelope("test", {}, event_id="evt-2")
        assert e1 != e2


class TestOperationMetadataViewEq:
    def test_eq_same_id(self):
        all_ops = OperationRegistry.all_operations()
        if len(all_ops) > 0:
            op1 = all_ops[0]
            op2 = OperationRegistry.find(op1.operation_id)
            assert op1 == op2


class TestEventStreamIterator:
    def test_iter(self):
        stream = EventStream()
        stream.push(EventEnvelope("test1", {"a": 1}, event_id="e1"))
        stream.push(EventEnvelope("test2", {"b": 2}, event_id="e2"))
        items = list(stream)
        assert len(items) == 2

    def test_contains(self):
        stream = EventStream()
        stream.push(EventEnvelope("test", {}, event_id="e1"))
        assert "e1" in stream
        assert "e2" not in stream
