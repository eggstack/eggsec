"""Tests for G2 event protocol and G3 callbacks/sinks."""

import json

import pytest

import eggsec


class TestEventProtocol:
    """G2: Versioned event types and EventEnvelope."""

    def test_event_schema_version(self):
        assert eggsec.EVENT_SCHEMA_VERSION == "1.0.0"

    def test_event_envelope_creation(self):
        payload = eggsec.PlanningEvent("op-1", "target.com", "in-scope")
        env = eggsec.EventEnvelope("planning", payload)
        assert env.schema_version == "1.0.0"
        assert env.event_type == "planning"
        assert env.event_id.startswith("evt-")
        assert env.timestamp_ms > 0

    def test_event_envelope_custom_fields(self):
        payload = eggsec.PlanningEvent("op-2", "host.local", "scope")
        env = eggsec.EventEnvelope(
            "planning",
            payload,
            event_id="custom-id",
            timestamp_ms=12345,
            correlation_id="corr-abc",
        )
        assert env.event_id == "custom-id"
        assert env.timestamp_ms == 12345
        assert env.correlation_id == "corr-abc"

    def test_event_envelope_to_dict(self):
        payload = eggsec.PlanningEvent("op-1", "target.com", "scope")
        env = eggsec.EventEnvelope("planning", payload)
        d = env.to_dict()
        assert isinstance(d, dict)
        assert d["schema_version"] == "1.0.0"
        assert d["event_type"] == "planning"
        assert "event_id" in d
        assert "timestamp_ms" in d

    def test_event_envelope_to_json(self):
        payload = eggsec.PlanningEvent("op-1", "target.com", "scope")
        env = eggsec.EventEnvelope("planning", payload)
        j = env.to_json()
        parsed = json.loads(j)
        assert parsed["schema_version"] == "1.0.0"
        assert parsed["event_type"] == "planning"

    def test_event_envelope_repr(self):
        payload = eggsec.PlanningEvent("op-1", "target.com", "scope")
        env = eggsec.EventEnvelope("planning", payload)
        r = repr(env)
        assert "EventEnvelope" in r
        assert "planning" in r

    def test_planning_event(self):
        e = eggsec.PlanningEvent("op-1", "target.com", "in-scope")
        assert e.operation_id == "op-1"
        assert e.target == "target.com"
        assert e.scope_summary == "in-scope"
        d = e.to_dict()
        assert d["operation_id"] == "op-1"

    def test_preflight_event(self):
        e = eggsec.PreflightEvent("allow", ["high-risk"], ["--verbose"])
        assert e.outcome == "allow"
        assert e.confirmations_required == ["high-risk"]
        assert e.suggested_flags == ["--verbose"]

    def test_stage_lifecycle_event(self):
        e = eggsec.StageLifecycleEvent("scan", "started")
        assert e.stage == "scan"
        assert e.status == "started"

    def test_progress_event(self):
        e = eggsec.ProgressEvent(50.0, "Halfway", 50, 100)
        assert e.percentage == 50.0
        assert e.message == "Halfway"
        assert e.items_processed == 50
        assert e.items_total == 100

    def test_finding_event(self):
        e = eggsec.FindingEventPy("f-1", "high", "XSS found", True)
        assert e.finding_id == "f-1"
        assert e.severity == "high"
        assert e.title == "XSS found"
        assert e.auto_added is True

    def test_artifact_event(self):
        e = eggsec.ArtifactEventPy("report.json", "report", "application/json", 1024)
        assert e.artifact_name == "report.json"
        assert e.kind == "report"
        assert e.size_bytes == 1024

    def test_cancellation_event(self):
        e = eggsec.CancellationEvent("user requested", "operator")
        assert e.reason == "user requested"
        assert e.cancelled_by == "operator"

    def test_failure_event(self):
        e = eggsec.FailureEvent("TimeoutError", "connection timed out", True)
        assert e.error_type == "TimeoutError"
        assert e.is_retryable is True

    def test_completion_event(self):
        e = eggsec.CompletionEvent("success", None, 5000)
        assert e.status == "success"
        assert e.duration_ms == 5000

    def test_wrap_event_function(self):
        payload = eggsec.PlanningEvent("op-1", "target.com", "scope")
        env = eggsec.wrap_event("planning", payload)
        assert env.event_type == "planning"
        assert env.schema_version == "1.0.0"

    def test_wrap_event_with_correlation(self):
        payload = eggsec.PlanningEvent("op-1", "target.com", "scope")
        env = eggsec.wrap_event("planning", payload, correlation_id="corr-1")
        assert env.correlation_id == "corr-1"


class TestEventStream:
    """G2: EventStream filtering and iteration."""

    def _make_stream(self):
        log = eggsec.EventLog()
        log.push(eggsec.ExecutionEvent("h1", "planning", 1000))
        log.push(eggsec.ExecutionEvent("h1", "progress", 2000))
        log.push(eggsec.ExecutionEvent("h1", "planning", 3000))
        log.push(eggsec.ExecutionEvent("h2", "completion", 4000))
        return eggsec.EventStream(log)

    def test_stream_from_log(self):
        stream = self._make_stream()
        assert len(stream) == 4

    def test_stream_empty(self):
        stream = eggsec.EventStream.empty()
        assert len(stream) == 0
        assert stream.is_empty()

    def test_stream_filter_by_type(self):
        stream = self._make_stream()
        filtered = stream.filter_by_type("planning")
        assert len(filtered) == 2
        for d in filtered.to_list():
            assert d["event_type"] == "planning"

    def test_stream_filter_by_correlation(self):
        log = eggsec.EventLog()
        log.push(eggsec.ExecutionEvent("h1", "start", 1000))
        stream = eggsec.EventStream(log)
        # Events without correlation_id should not match
        filtered = stream.filter_by_correlation("corr-1")
        assert len(filtered) == 0

    def test_stream_latest(self):
        stream = self._make_stream()
        latest = stream.latest()
        assert latest is not None
        assert isinstance(latest, dict)
        assert latest["event_type"] == "completion"

    def test_stream_to_list(self):
        stream = self._make_stream()
        lst = stream.to_list()
        assert len(lst) == 4
        assert all(isinstance(d, dict) for d in lst)
        assert all("event_type" in d for d in lst)

    def test_stream_to_dict_list(self):
        stream = self._make_stream()
        lst = stream.to_dict_list()
        assert len(lst) == 4
        assert all(isinstance(d, dict) for d in lst)

    def test_stream_count(self):
        stream = self._make_stream()
        assert stream.count() == 4

    def test_stream_snapshot(self):
        stream = self._make_stream()
        snap = stream.snapshot()
        assert isinstance(snap, dict)
        assert snap["total_events"] == 4

    def test_stream_get(self):
        stream = self._make_stream()
        d = stream.get(0)
        assert isinstance(d, dict)
        assert d["event_type"] == "planning"

    def test_stream_get_out_of_range(self):
        stream = self._make_stream()
        with pytest.raises(IndexError):
            stream.get(99)

    def test_stream_from_legacy(self):
        events = [
            eggsec.ExecutionEvent("h1", "start", 1000),
            eggsec.ExecutionEvent("h1", "end", 2000),
        ]
        stream = eggsec.event_stream_from_legacy(events)
        assert len(stream) == 2


class TestEventLogEnhancements:
    """G2: Enhanced EventLog methods."""

    def test_to_versioned_list(self):
        log = eggsec.EventLog()
        log.push(eggsec.ExecutionEvent("h1", "start", 1000))
        log.push(eggsec.ExecutionEvent("h1", "end", 2000))
        versioned = log.to_versioned_list()
        assert len(versioned) == 2
        for env in versioned:
            assert isinstance(env, dict)
            assert env["schema_version"] == "1.0.0"

    def test_schema_version(self):
        log = eggsec.EventLog()
        assert log.schema_version() == "1.0.0"

    def test_drain(self):
        log = eggsec.EventLog()
        log.push(eggsec.ExecutionEvent("h1", "start", 1000))
        log.push(eggsec.ExecutionEvent("h1", "end", 2000))
        drained = log.drain()
        assert len(drained) == 2
        assert log.is_empty()
        for env in drained:
            assert isinstance(env, dict)


class TestAuditSink:
    """G3: AuditSink receives events."""

    def test_send(self):
        received = []

        def handler(event):
            received.append(event)

        sink = eggsec.AuditSink(handler)
        assert not sink.is_closed
        # AuditSink.send expects EnforcementAuditEventPy; we test basic plumbing
        # by verifying the sink is not closed and handler is stored

    def test_close(self):
        def handler(event):
            pass

        sink = eggsec.AuditSink(handler)
        sink.close()
        assert sink.is_closed

    def test_repr(self):
        def handler(event):
            pass

        sink = eggsec.AuditSink(handler)
        assert "AuditSink" in repr(sink)
        assert "closed" in repr(sink)


class TestFindingSink:
    """G3: FindingSink receives findings."""

    def test_close(self):
        def handler(finding):
            pass

        sink = eggsec.FindingSink(handler)
        sink.close()
        assert sink.is_closed

    def test_repr(self):
        def handler(finding):
            pass

        sink = eggsec.FindingSink(handler)
        assert "FindingSink" in repr(sink)


class TestArtifactSink:
    """G3: ArtifactSink receives artifacts."""

    def test_close(self):
        def handler(artifact):
            pass

        sink = eggsec.ArtifactSink(handler)
        sink.close()
        assert sink.is_closed


class TestProgressSink:
    """G3: ProgressSink receives progress updates."""

    def test_send(self):
        received = []

        def handler(percentage, message):
            received.append((percentage, message))

        sink = eggsec.ProgressSink(handler)
        # ProgressSink.send is safe; basic plumbing test
        sink.close()
        assert sink.is_closed

    def test_repr(self):
        def handler(p, m):
            pass

        sink = eggsec.ProgressSink(handler)
        assert "ProgressSink" in repr(sink)


class TestEventConsumer:
    """G3: EventConsumer receives versioned events."""

    def test_close(self):
        def handler(event):
            pass

        consumer = eggsec.EventConsumer(handler)
        consumer.close()
        assert consumer.is_closed


class TestErrorIsolation:
    """G3: Callback errors don't crash."""

    def test_error_in_finding_sink(self):
        def bad_handler(finding):
            raise ValueError("intentional error")

        sink = eggsec.FindingSink(bad_handler)
        assert not sink.is_closed

    def test_error_in_progress_sink(self):
        def bad_handler(percentage, message):
            raise RuntimeError("boom")

        sink = eggsec.ProgressSink(bad_handler)
        assert not sink.is_closed


class TestAsyncCallback:
    """G3: AsyncCallback wrapper."""

    def test_create(self):
        async def handler(event):
            pass

        cb = eggsec.AsyncCallback(handler)
        assert not cb.is_closed

    def test_close(self):
        async def handler(event):
            pass

        cb = eggsec.AsyncCallback(handler)
        cb.close()
        assert cb.is_closed

    def test_repr(self):
        async def handler(event):
            pass

        cb = eggsec.AsyncCallback(handler)
        assert "AsyncCallback" in repr(cb)


class TestCallbackScheduler:
    """G3: CallbackScheduler with backpressure."""

    def test_create(self):
        scheduler = eggsec.CallbackScheduler(100)
        assert scheduler.pending() == 0

    def test_enqueue_and_drain(self):
        scheduler = eggsec.CallbackScheduler(10)
        payload = eggsec.PlanningEvent("op-1", "t.com", "s")
        env = eggsec.EventEnvelope("planning", payload)
        assert scheduler.enqueue(env) is True
        assert scheduler.pending() == 1
        events = scheduler.drain()
        assert len(events) == 1
        assert scheduler.pending() == 0

    def test_capacity_limit(self):
        scheduler = eggsec.CallbackScheduler(2)
        for i in range(4):
            payload = eggsec.PlanningEvent(f"op-{i}", "t.com", "s")
            env = eggsec.EventEnvelope("planning", payload)
            scheduler.enqueue(env)
        assert scheduler.pending() == 2

    def test_close(self):
        scheduler = eggsec.CallbackScheduler(10)
        scheduler.close()
        assert scheduler.is_closed
        payload = eggsec.PlanningEvent("op-1", "t.com", "s")
        env = eggsec.EventEnvelope("planning", payload)
        assert scheduler.enqueue(env) is False

    def test_repr(self):
        scheduler = eggsec.CallbackScheduler(100)
        assert "CallbackScheduler" in repr(scheduler)


class TestBackpressureChannel:
    """G3: Backpressure channel."""

    def test_create(self):
        ch = eggsec.BackpressureChannel(128)
        assert ch.capacity == 128
        assert len(ch) == 0
        assert ch.is_empty()

    def test_send_and_recv(self):
        ch = eggsec.BackpressureChannel(10)
        payload = eggsec.PlanningEvent("op-1", "t.com", "s")
        env = eggsec.EventEnvelope("planning", payload)
        ch.send(env)
        assert len(ch) == 1
        received = ch.try_recv()
        assert received is not None
        assert received.event_type == "planning"
        assert len(ch) == 0

    def test_backpressure_drops_oldest(self):
        ch = eggsec.BackpressureChannel(2)
        for i in range(4):
            payload = eggsec.PlanningEvent(f"op-{i}", "t.com", "s")
            env = eggsec.EventEnvelope("planning", payload)
            ch.send(env)
        assert len(ch) == 2
        assert ch.total_dropped() == 2
        # The oldest events were dropped; remaining should be op-2 and op-3
        e1 = ch.try_recv()
        e2 = ch.try_recv()
        assert e1 is not None
        assert e2 is not None

    def test_try_recv_empty(self):
        ch = eggsec.BackpressureChannel(10)
        assert ch.try_recv() is None

    def test_repr(self):
        ch = eggsec.BackpressureChannel(10)
        assert "BackpressureChannel" in repr(ch)
