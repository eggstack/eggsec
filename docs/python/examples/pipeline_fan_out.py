#!/usr/bin/env python3
"""
Pipeline Fan-Out and Fan-In.

Demonstrates parallel execution of independent steps (fan-out) followed
by a step that depends on all of them (fan-in) using parallel_group
and dependencies.

Requirements:
    - eggsec installed
    - Network access to the target

Usage:
    python pipeline_fan_out.py [target]
"""

import sys

import eggsec
from eggsec import (
    Pipeline, Engine, Scope, OperationRequest,
    RetryPolicy, FailurePolicy,
)

TARGET = sys.argv[1] if len(sys.argv) > 1 else "example.com"


def main():
    scope = Scope.allow_hosts([TARGET])
    engine = Engine(scope)

    # Create pipeline with parallel execution
    pipeline = Pipeline(
        f"fan-out-{TARGET}",
        retry_policy=RetryPolicy(
            max_attempts=2,
            retryable_errors=["network", "timeout"],
            backoff_ms=1000,
        ),
        failure_policy=FailurePolicy.SkipDependents,
        max_concurrency=4,
    )

    # Fan-out: three independent recon steps in the same parallel group
    pipeline.add_step(
        "dns-recon",
        OperationRequest("recon", TARGET),
        parallel_group="passive-recon",
    )
    pipeline.add_step(
        "tls-inspect",
        OperationRequest("tls-inspect", TARGET),
        parallel_group="passive-recon",
    )
    pipeline.add_step(
        "tech-detect",
        OperationRequest("tech-detect", TARGET),
        parallel_group="passive-recon",
    )

    # Fan-in: fingerprint depends on all three recon steps completing
    pipeline.add_step(
        "fingerprint",
        OperationRequest("fingerprint-services", TARGET),
        dependencies=["dns-recon", "tls-inspect", "tech-detect"],
    )

    # Final step: WAF detection (depends on fingerprint)
    pipeline.add_step(
        "waf-detect",
        OperationRequest("waf-detect", f"https://{TARGET}"),
        dependencies=["fingerprint"],
    )

    print(f"Pipeline: {pipeline.name}")
    print(f"Steps: {pipeline.steps_count()}")
    print(f"Max concurrency: {pipeline.max_concurrency}")
    print(f"\nExecuting pipeline...")

    result = pipeline.run(engine)

    print(f"\nPipeline result: {result.status.name()}")
    print(f"Duration: {result.total_duration_ms}ms")
    print(f"Retried steps: {result.retried_steps}")

    for step in result.step_results:
        status = step.status.name()
        duration = step.duration_ms
        attempt = step.attempt
        retry_info = f" (attempt {attempt})" if attempt > 1 else ""
        print(f"  {step.step_name}: {status} ({duration}ms){retry_info}")

    # Print events summary
    event_types = {}
    for event in result.events:
        event_types[event.event_type] = event_types.get(event.event_type, 0) + 1

    print(f"\nEvents ({len(result.events)} total):")
    for event_type, count in sorted(event_types.items()):
        print(f"  {event_type}: {count}")


if __name__ == "__main__":
    main()
