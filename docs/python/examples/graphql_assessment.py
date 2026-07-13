#!/usr/bin/env python3
"""
GraphQL Assessment with Preflight.

Demonstrates using the preflight policy gate before dispatching a GraphQL
security assessment, and handling the result.

Requirements:
    - eggsec installed (graphql is in the default wheel)
    - A GraphQL endpoint to test

Usage:
    python graphql_assessment.py [target]
"""

import sys

import eggsec
from eggsec import (
    Engine, Scope, GraphQLTestConfig,
    EnforcementContext, ExecutionPolicy, ExecutionSurface,
    LoadedScope, OperationRegistry,
)


def main():
    target = sys.argv[1] if len(sys.argv) > 1 else "https://api.example.com/graphql"

    # Set up scope and policy
    scope = Scope.allow_hosts([target])
    engine = Engine(scope)

    # --- Preflight: preview the policy decision ---
    loaded_scope = LoadedScope.from_scope(scope)
    policy = ExecutionPolicy.default()
    ctx = EnforcementContext.manual_permissive(policy, loaded_scope)

    # Look up the operation
    op = OperationRegistry.find("graphql-test")
    desc = op.descriptor_for_target(target)

    # Evaluate without side effects
    outcome = ctx.evaluate(desc)
    print(f"Preflight decision: {outcome.outcome_type}")

    if outcome.outcome_type != "allow":
        print(f"Operation would be denied: {outcome.reason}")
        sys.exit(1)

    # Approve (generates audit token)
    approved = ctx.approve(ExecutionSurface.CLI_MANUAL, desc)
    print(f"Audit token: {approved.audit_event_id}")

    # --- Dispatch the operation ---
    config = GraphQLTestConfig(target=target)

    print(f"\nRunning GraphQL assessment against {target}...")

    result = engine.run_graphql_test(config)

    if result.status.name() == "Completed":
        report = result.payload
        print(f"\nGraphQL Assessment Complete")

        # Schema info
        if report.schema:
            print(f"Schema introspection: {'enabled' if report.schema.introspection_enabled else 'disabled'}")
            print(f"Types: {len(report.schema.types)}")
            print(f"Queries: {len(report.schema.queries)}")
            print(f"Mutations: {len(report.schema.mutations)}")

        # Vulnerabilities
        if report.vulnerabilities:
            print(f"\nVulnerabilities found: {len(report.vulnerabilities)}")
            for vuln in report.vulnerabilities:
                severity = vuln.severity if hasattr(vuln, "severity") else "unknown"
                print(f"  [{severity}] {vuln.vuln_type}: {vuln.description}")
        else:
            print("\nNo vulnerabilities detected")

        # Audit trail
        if result.audit_event_id:
            print(f"\nAudit event: {result.audit_event_id}")

    elif result.status.name() == "Failed":
        error = result.error
        print(f"Assessment failed ({error.kind}): {error.message}")
        sys.exit(1)
    else:
        print(f"Unexpected status: {result.status.name()}")
        sys.exit(1)


if __name__ == "__main__":
    main()
