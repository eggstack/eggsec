#!/usr/bin/env python3
"""Policy preflight and enforcement context.

Demonstrates previewing operation dispatch through the policy gate
without executing. Uses preflight_operation for a clean dry-run.
All logic is in-memory -- no network calls.

Requirements:
    - eggsec (default features)

Usage:
    python3 docs/python/examples/policy_preflight.py
"""

from eggsec import (
    ExecutionPolicy,
    LoadedScope,
    OperationRegistry,
    preflight_operation,
)


def main():
    # Build a scope (empty by default for demo)
    scope = LoadedScope.default_empty()

    # Default execution policy (constructed with no args)
    policy = ExecutionPolicy()
    print(f"Policy allows passive: {policy.allow_passive_fingerprint}")
    print(f"Policy allows load testing: {policy.allow_load_testing}")
    print(f"Policy allows stress testing: {policy.allow_stress_testing}")

    # Query the operation registry
    total = OperationRegistry.operation_count()
    print(f"\nRegistered operations: {total}")

    # Inspect a specific operation
    scan_ports_op = OperationRegistry.find("scan-ports")
    if scan_ports_op:
        print(f"\nscan-ports descriptor:")
        print(f"  operation_id: {scan_ports_op.operation_id}")
        print(f"  operation_name: {scan_ports_op.operation_name}")
        print(f"  default_risk: {scan_ports_op.default_risk}")
        print(f"  default_timeout_ms: {scan_ports_op.default_timeout_ms}")
        print(f"  surfaces: {scan_ports_op.supported_surfaces}")

        # Dry-run preview through the policy gate (no side effects)
        outcome = preflight_operation("scan-ports", scope, policy, target="example.com")
        print(f"\n  Preflight outcome: {outcome.outcome}")
        print(f"  Decision: {outcome.decision}")
        print(f"  Risk level: {outcome.risk_level}")
        print(f"  Requires confirmation: {outcome.requires_confirmation}")

        if outcome.outcome == "allow":
            print("  -> Operation would be approved for dispatch")
        else:
            print("  -> Operation blocked by policy")

        # Human-readable summary
        print(f"\n  Summary:\n  {outcome.to_human_readable()}")

    # List some operations by feature
    db_ops = OperationRegistry.operations_for_feature("db-pentest")
    print(f"\nDB pentest operations: {[op.operation_id for op in db_ops]}")

    cli_ops = OperationRegistry.operations_for_surface("cli")
    print(f"CLI-surface operations: {len(cli_ops)}")


if __name__ == "__main__":
    main()
