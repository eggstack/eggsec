#!/usr/bin/env python3
"""
Checkpoint and Resume.

Demonstrates saving pipeline state after each step and resuming from
the last checkpoint when the pipeline is re-run. This enables
long-running assessments to survive interruptions.

Requirements:
    - eggsec installed
    - Network access to the target
    - Write access to /tmp for the checkpoint file

Usage:
    python checkpoint_resume.py [target]
"""

import os
import sys
import tempfile

import eggsec
from eggsec import (
    Pipeline, Engine, Scope, OperationRequest,
    PipelineRetryPolicy, FailurePolicy, create_checkpoint_store,
)

TARGET = sys.argv[1] if len(sys.argv) > 1 else "example.com"
CHECKPOINT_PATH = os.path.join(tempfile.gettempdir(), "eggsec-demo-checkpoint.json")


def main():
    scope = Scope.allow_hosts([TARGET])
    engine = Engine(scope)

    # Create file-backed checkpoint store
    store = create_checkpoint_store(CHECKPOINT_PATH)

    # Build pipeline
    pipeline = Pipeline(
        f"checkpoint-demo-{TARGET}",
        retry_policy=PipelineRetryPolicy(max_attempts=2, backoff_ms=1000),
        failure_policy=FailurePolicy.SkipDependents,
        max_concurrency=2,
    )
    pipeline.set_checkpoint_store(store)

    # Define steps
    pipeline.add_step("dns", OperationRequest("recon", TARGET))
    pipeline.add_step("tls", OperationRequest("tls-inspect", TARGET),
                      parallel_group="passive")
    pipeline.add_step("tech", OperationRequest("tech-detect", TARGET),
                      parallel_group="passive")
    pipeline.add_step("fingerprint", OperationRequest("fingerprint-services", TARGET),
                      dependencies=["dns", "tls", "tech"])

    print(f"Pipeline: {pipeline.name}")
    print(f"Checkpoint file: {CHECKPOINT_PATH}")
    print(f"Steps: {pipeline.steps_count()}")

    # Check for existing checkpoint
    existing = store.load(pipeline.pipeline_id())
    if existing:
        cp = existing.checkpoint
        print(f"\nFound existing checkpoint (version {cp.version})")
        print(f"Completed steps: {cp.completed_steps}")
        print(f"Resuming from checkpoint...")
    else:
        print(f"\nNo existing checkpoint — starting fresh")

    # Run the pipeline
    result = pipeline.run(engine)

    print(f"\nPipeline result: {result.status.name()}")
    print(f"Duration: {result.total_duration_ms}ms")

    for step in result.step_results:
        status = step.status.name()
        attempt = step.attempt
        retry_info = f" (attempt {attempt})" if attempt > 1 else ""
        print(f"  {step.step_name}: {status} ({step.duration_ms}ms){retry_info}")

    # Show checkpoint state
    store_result = store.load(pipeline.pipeline_id())
    if store_result:
        cp = store_result.checkpoint
        print(f"\nCheckpoint saved:")
        print(f"  Pipeline ID: {cp.pipeline_id}")
        print(f"  Completed: {cp.completed_steps}")
        print(f"  Schema version: {cp.version}")
        print(f"  Created: {cp.created_at_ms}")
    else:
        print(f"\nNo checkpoint (pipeline completed successfully)")

    # Demonstrate resume scenario
    if not result.is_success():
        print(f"\n--- Simulating resume ---")
        print(f"Re-running pipeline (checkpoint will resume from last step)...")

        # Create a fresh pipeline with the same definition
        resume_pipeline = Pipeline(
            f"checkpoint-demo-{TARGET}",
            retry_policy=PipelineRetryPolicy(max_attempts=2, backoff_ms=1000),
            failure_policy=FailurePolicy.SkipDependents,
            max_concurrency=2,
        )
        resume_pipeline.set_checkpoint_store(store)
        resume_pipeline.add_step("dns", OperationRequest("recon", TARGET))
        resume_pipeline.add_step("tls", OperationRequest("tls-inspect", TARGET),
                                 parallel_group="passive")
        resume_pipeline.add_step("tech", OperationRequest("tech-detect", TARGET),
                                 parallel_group="passive")
        resume_pipeline.add_step("fingerprint", OperationRequest("fingerprint-services", TARGET),
                                 dependencies=["dns", "tls", "tech"])

        resume_result = resume_pipeline.run(engine)
        print(f"Resume result: {resume_result.status.name()}")
        print(f"Duration: {resume_result.total_duration_ms}ms")

    # Cleanup
    print(f"\nCheckpoint file: {CHECKPOINT_PATH}")
    print("(Checkpoint persists for potential future resume)")


if __name__ == "__main__":
    main()
