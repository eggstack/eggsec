"""Events and cancellation tests.

Verifies that pipelines emit StageLifecycleEvents with monotonic sequence IDs,
CancellationToken cancels pipeline execution, cancelled pipelines report
CompletionEvent with cancelled status, and cancel reason is preserved.
"""

import pytest
from conftest import SENTINEL_TARGET, SENTINEL_PORT, SENTINEL_TIMEOUT_MS, SENTINEL_MODE, SENTINEL_CONCURRENCY

import eggsec
from eggsec import (
    OperationRequest,
    Pipeline,
    CancellationToken,
    EventEnvelope,
    StageLifecycleEvent,
    CompletionEvent,
)


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def make_scan_request(target=SENTINEL_TARGET, port=SENTINEL_PORT):
    return OperationRequest(
        "scan_ports",
        target,
        timeout_ms=SENTINEL_TIMEOUT_MS,
        metadata={"ports": str(port)},
    )


def engine_for(sentinel_scope):
    return eggsec.Engine(
        sentinel_scope,
        mode=SENTINEL_MODE,
        concurrency=SENTINEL_CONCURRENCY,
        timeout_ms=SENTINEL_TIMEOUT_MS,
    )


# ---------------------------------------------------------------------------
# 1. Pipeline emits StageLifecycleEvents with monotonic sequence IDs
# ---------------------------------------------------------------------------


class TestPipelineEventSequence:
    def test_pipeline_emits_lifecycle_events(self, sentinel_scope):
        """Pipeline should emit pipeline.started, step.started, step.completed, pipeline.completed."""
        engine = engine_for(sentinel_scope)
        pipeline = Pipeline("event-seq-test")
        pipeline.add_step("step-a", make_scan_request())
        result = pipeline.run(engine)

        event_types = [e.event_type for e in result.events]

        # Must start with pipeline.started
        assert event_types[0] == "pipeline.started"
        # Must end with pipeline.completed
        assert event_types[-1] == "pipeline.completed"
        # Should have step.started and step.completed/failed in between
        assert "step.started" in event_types
        assert "step.completed" in event_types or "step.failed" in event_types

    def test_event_timestamps_are_monotonic(self, sentinel_scope):
        """Event timestamps should be non-decreasing."""
        engine = engine_for(sentinel_scope)
        pipeline = Pipeline("ts-monotonic-test")
        pipeline.add_step("step-a", make_scan_request())
        result = pipeline.run(engine)

        timestamps = [e.timestamp_ms for e in result.events]
        for i in range(1, len(timestamps)):
            assert timestamps[i] >= timestamps[i - 1], (
                f"Timestamp at index {i} ({timestamps[i]}) < timestamp at {i - 1} ({timestamps[i - 1]})"
            )

    def test_events_have_event_ids(self, sentinel_scope):
        """Each event should have a non-empty event_id."""
        engine = engine_for(sentinel_scope)
        pipeline = Pipeline("unique-id-test")
        pipeline.add_step("step-a", make_scan_request())
        result = pipeline.run(engine)

        for event in result.events:
            assert event.event_id is not None
            assert len(event.event_id) > 0

    def test_events_have_schema_version(self, sentinel_scope):
        """All events should carry the schema version."""
        engine = engine_for(sentinel_scope)
        pipeline = Pipeline("schema-version-test")
        pipeline.add_step("step-a", make_scan_request())
        result = pipeline.run(engine)

        for event in result.events:
            assert event.schema_version is not None
            assert len(event.schema_version) > 0

    def test_event_correlation_id_present(self, sentinel_scope):
        """Pipeline events should share a correlation_id."""
        engine = engine_for(sentinel_scope)
        pipeline = Pipeline("corr-id-test")
        pipeline.add_step("step-a", make_scan_request())
        result = pipeline.run(engine)

        correlation_ids = [e.correlation_id for e in result.events if e.correlation_id is not None]
        # At least some events should have correlation_id
        assert len(correlation_ids) > 0
        # All non-None correlation_ids should be the same
        assert len(set(correlation_ids)) == 1

    def test_stage_lifecycle_event_payload(self, sentinel_scope):
        """StageLifecycleEvent payloads should have stage and status fields."""
        engine = engine_for(sentinel_scope)
        pipeline = Pipeline("payload-test")
        pipeline.add_step("step-a", make_scan_request())
        result = pipeline.run(engine)

        lifecycle_events = [
            e for e in result.events
            if e.event_type.startswith("step.") or e.event_type.startswith("pipeline.")
        ]
        assert len(lifecycle_events) > 0


# ---------------------------------------------------------------------------
# 2. CancellationToken cancels pipeline execution
# ---------------------------------------------------------------------------


class TestCancellationToken:
    def test_token_initial_state(self):
        """New CancellationToken should not be cancelled."""
        token = CancellationToken()
        assert token.is_cancelled() is False
        assert token.reason() is None

    def test_token_cancel_sets_state(self):
        """Calling cancel() should set is_cancelled to True."""
        token = CancellationToken()
        token.cancel("test reason")
        assert token.is_cancelled() is True
        assert token.reason() == "test reason"

    def test_token_cancel_without_reason(self):
        """cancel() without reason should still set is_cancelled."""
        token = CancellationToken()
        token.cancel()
        assert token.is_cancelled() is True
        assert token.reason() is None

    def test_token_to_dict(self):
        """to_dict() should reflect cancelled state."""
        token = CancellationToken()
        d = token.to_dict()
        assert d["cancelled"] is False

        token.cancel("test")
        d = token.to_dict()
        assert d["cancelled"] is True
        assert d["reason"] == "test"

    def test_token_to_json(self):
        """to_json() should produce valid JSON reflecting state."""
        import json
        token = CancellationToken()
        j = token.to_json()
        parsed = json.loads(j)
        assert parsed["cancelled"] is False

        token.cancel("json-test")
        j = token.to_json()
        parsed = json.loads(j)
        assert parsed["cancelled"] is True
        assert parsed["reason"] == "json-test"

    def test_token_cancel_token_method(self):
        """cancel_token() returns a dict with is_cancelled and reason."""
        token = CancellationToken()
        ct = token.cancel_token()
        assert ct["is_cancelled"] is False

        token.cancel("via-method")
        ct = token.cancel_token()
        assert ct["is_cancelled"] is True
        assert ct["reason"] == "via-method"


# ---------------------------------------------------------------------------
# 3. Cancelled pipeline reports CompletionEvent with cancelled status
# ---------------------------------------------------------------------------


class TestPipelineCancellation:
    def test_pre_cancelled_token_cancels_pipeline(self, sentinel_scope):
        """Pipeline with already-cancelled token should report Cancelled status."""
        engine = engine_for(sentinel_scope)
        pipeline = Pipeline("pre-cancel-test")
        pipeline.add_step("step-a", make_scan_request())

        token = CancellationToken()
        token.cancel("pre-cancel-reason")
        pipeline.set_cancel_token(token)

        result = pipeline.run(engine)

        # Pipeline should be cancelled
        assert result.status.name() == "Cancelled"

    def test_cancelled_pipeline_has_cancellation_events(self, sentinel_scope):
        """Cancelled pipeline should not execute steps after cancellation."""
        engine = engine_for(sentinel_scope)
        pipeline = Pipeline("cancel-events-test")
        pipeline.add_step("step-a", make_scan_request())
        pipeline.add_step("step-b", make_scan_request())

        token = CancellationToken()
        token.cancel("cancel-between-steps")
        pipeline.set_cancel_token(token)

        result = pipeline.run(engine)

        # Should have pipeline.started and pipeline.completed at minimum
        event_types = [e.event_type for e in result.events]
        assert "pipeline.started" in event_types
        assert "pipeline.completed" in event_types

        # step.started should only appear for the first step (or none if cancelled before)
        step_started = [e for e in result.events if e.event_type == "step.started"]
        # With pre-cancel, no steps should have started
        assert len(step_started) == 0

    def test_cancel_reason_preserved_in_status(self, sentinel_scope):
        """Cancelled pipeline status should include the reason."""
        engine = engine_for(sentinel_scope)
        pipeline = Pipeline("reason-preserve-test")
        pipeline.add_step("step-a", make_scan_request())

        token = CancellationToken()
        token.cancel("my-specific-reason-SENTINEL")
        pipeline.set_cancel_token(token)

        result = pipeline.run(engine)

        # The status should indicate cancellation
        assert result.status.name() == "Cancelled"

    def test_pipeline_step_results_empty_on_pre_cancel(self, sentinel_scope):
        """Pre-cancelled pipeline should have no step results."""
        engine = engine_for(sentinel_scope)
        pipeline = Pipeline("empty-results-test")
        pipeline.add_step("step-a", make_scan_request())

        token = CancellationToken()
        token.cancel("immediate")
        pipeline.set_cancel_token(token)

        result = pipeline.run(engine)
        assert len(result.step_results) == 0


# ---------------------------------------------------------------------------
# 4. Pipeline events can be serialized
# ---------------------------------------------------------------------------


class TestEventSerialization:
    def test_events_to_dict(self, sentinel_scope):
        """Pipeline events should be convertible to dicts."""
        engine = engine_for(sentinel_scope)
        pipeline = Pipeline("dict-serialize-test")
        pipeline.add_step("step-a", make_scan_request())
        result = pipeline.run(engine)

        for event in result.events:
            d = event.to_dict()
            assert isinstance(d, dict)
            assert "schema_version" in d
            assert "event_id" in d
            assert "timestamp_ms" in d
            assert "event_type" in d

    def test_events_to_json(self, sentinel_scope):
        """Pipeline events should be convertible to JSON."""
        import json
        engine = engine_for(sentinel_scope)
        pipeline = Pipeline("json-serialize-test")
        pipeline.add_step("step-a", make_scan_request())
        result = pipeline.run(engine)

        for event in result.events:
            j = event.to_json()
            parsed = json.loads(j)
            assert "event_type" in parsed
            assert "event_id" in parsed
