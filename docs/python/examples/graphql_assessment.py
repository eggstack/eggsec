#!/usr/bin/env python3
"""
GraphQL Assessment.

Demonstrates running a GraphQL security assessment against a target endpoint.

Requirements:
    - eggsec installed (graphql is in the default wheel)
    - A GraphQL endpoint to test

Usage:
    python graphql_assessment.py [target]
"""

import sys

from eggsec import (
    Scope,
    GraphQLTestConfig,
    graphql_test,
)


def main():
    target = sys.argv[1] if len(sys.argv) > 1 else "https://api.example.com/graphql"

    # Scope: declare which hosts are in scope
    scope = Scope.allow_hosts([target])
    print(f"Scope: {target}")

    # Build the GraphQL assessment config
    config = GraphQLTestConfig(endpoint=target)

    print(f"\nRunning GraphQL assessment against {target}...")

    try:
        results = graphql_test(config)
    except Exception as e:
        print(f"Assessment failed: {e}")
        sys.exit(1)

    print(f"\nGraphQL Assessment Complete")
    print(f"  Results: {len(results)}")

    vulnerable = sum(1 for r in results if r.vulnerability)
    successful = sum(1 for r in results if r.success)
    print(f"  Vulnerable: {vulnerable}")
    print(f"  Successful queries: {successful}")


if __name__ == "__main__":
    main()
