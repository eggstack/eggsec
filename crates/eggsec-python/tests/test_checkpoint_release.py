"""Release-grade checkpoint, resume, and secret-redaction coverage."""

from __future__ import annotations

import json
from pathlib import Path

import eggsec

from fixtures.stable_core import HOST, StableCoreFixtures


SENTINEL = "EGGSEC_SECRET_SENTINEL_7F4B9D2A"


def test_checkpoint_redacts_secret_fields_and_rejects_corruption(tmp_path: Path):
    path = tmp_path / "checkpoints.json"
    store = eggsec.create_checkpoint_store(str(path))
    checkpoint = eggsec.PipelineCheckpoint(
        "pipeline-1",
        "fixture-pipeline",
        completed_steps=["step-a"],
        step_results={
            "step-a": {
                "status": "completed",
                "authorization": SENTINEL,
                "nested": {"client_secret": SENTINEL},
            }
        },
    )
    assert SENTINEL not in checkpoint.to_json()
    store.save(checkpoint)
    assert path.exists()
    assert not list(tmp_path.glob("*.tmp-*"))
    assert SENTINEL not in path.read_text()

    reloaded = eggsec.create_checkpoint_store(str(path)).load("pipeline-1")
    assert reloaded is not None
    assert reloaded.checkpoint.version == 3
    assert reloaded.checkpoint.step_results["step-a"]["authorization"] == "[REDACTED]"

    path.write_text("{not-json")
    try:
        eggsec.create_checkpoint_store(str(path))
    except ValueError as error:
        assert "Failed to parse checkpoint file" in str(error)
    else:
        raise AssertionError("corrupted checkpoint must be rejected")


def test_pipeline_resume_restores_typed_completed_results(tmp_path: Path):
    with StableCoreFixtures() as fixtures:
        scope = eggsec.Scope.allow_hosts([HOST])
        engine = eggsec.Engine(scope)
        path = tmp_path / "pipeline.json"
        store = eggsec.create_checkpoint_store(str(path))
        request = eggsec.OperationRequest(
            "scan_ports",
            HOST,
            timeout_ms=1000,
            metadata={"ports": str(fixtures.tcp_port)},
        )
        denied = eggsec.OperationRequest("scan_ports", "192.0.2.1", timeout_ms=20)

        pipeline = eggsec.Pipeline("resume-fixture", stop_on_failure=False, failure_policy=eggsec.FailurePolicy.Continue)
        pipeline.add_step("open", request)
        pipeline.add_step("denied", denied)
        pipeline.set_checkpoint_store(store)
        first = pipeline.run(engine)
        assert len(first.step_results) == 2
        open_result = next(r for r in first.step_results if r.step_name == "open")
        assert open_result.result.payload_type_name == "PortScanResult"
        assert path.exists()

        resumed_store = eggsec.create_checkpoint_store(str(path))
        resumed_pipeline = eggsec.Pipeline("resume-fixture", stop_on_failure=False, failure_policy=eggsec.FailurePolicy.Continue)
        resumed_pipeline.add_step("open", request)
        resumed_pipeline.add_step("denied", denied)
        resumed_pipeline.set_checkpoint_store(resumed_store)
        resumed = resumed_pipeline.run(engine)

        assert len(resumed.step_results) == 2
        resumed_open = next(r for r in resumed.step_results if r.step_name == "open")
        assert resumed_open.result.payload_type_name == "PortScanResult"
        resumed_denied = next(r for r in resumed.step_results if r.step_name == "denied")
        assert resumed_denied is not None
        assert any(event.event_type == "pipeline.resumed_from_checkpoint" for event in resumed.events)
        assert isinstance(json.loads(resumed.to_json()), dict)
