#!/usr/bin/env python3
"""
Database Probe using SensitiveString.

Demonstrates database security probing with proper credential handling
via SensitiveString. Credentials are automatically redacted in repr,
events, reports, and checkpoints.

Requirements:
    - eggsec installed with `db-pentest` feature
    - A database server to probe

Usage:
    python database_probe.py <host> [port] [database] [username] [password]
"""

import sys

import eggsec
from eggsec import Engine, Scope, DbProbeRequest


def main():
    if len(sys.argv) < 2:
        print("Usage: python database_probe.py <host> [port] [database] [username] [password]")
        sys.exit(1)

    host = sys.argv[1]
    port = int(sys.argv[2]) if len(sys.argv) > 2 else 5432
    database = sys.argv[3] if len(sys.argv) > 3 else "postgres"
    username = sys.argv[4] if len(sys.argv) > 4 else "postgres"
    password = sys.argv[5] if len(sys.argv) > 5 else ""

    # Verify feature availability
    features = eggsec.features()
    if not features.get("db-pentest", False):
        print("Error: 'db-pentest' feature not compiled.")
        print("Build with: maturin develop --features db-pentest")
        sys.exit(1)

    # Create engine
    scope = Scope.allow_hosts([host])
    engine = Engine(scope)

    # Build request with credentials
    # Password is wrapped in SensitiveString internally — never exposed in repr/logs
    request = DbProbeRequest(
        target=host,
        port=port,
        database=database,
        username=username,
        password=password,
    )

    # Verify redaction works
    print(f"Request repr (password redacted): {repr(request)}")

    print(f"\nProbing database at {host}:{port}/{database}...")

    result = engine.run_db_probe(request)

    if result.status.name() == "Completed":
        report = result.payload
        print(f"\nDatabase Probe Complete")
        print(f"Database type: {report.db_type}")
        print(f"Version: {report.version}")

        # Findings
        if report.findings:
            print(f"\nFindings: {len(report.findings)}")
            for finding in report.findings:
                print(f"\n  [{finding.severity}] {finding.category}: {finding.title}")
                print(f"    {finding.description}")
                if finding.recommendation:
                    print(f"    Recommendation: {finding.recommendation}")
                if finding.evidence:
                    print(f"    Evidence: {finding.evidence[:100]}...")
        else:
            print("\nNo security issues found")

        # Capabilities detected
        if report.capabilities:
            print(f"\nCapabilities:")
            for cap in report.capabilities:
                print(f"  {cap.name}: {cap.description}")

    elif result.status.name() == "Failed":
        error = result.error
        print(f"Probe failed ({error.kind}): {error.message}")
        sys.exit(1)
    else:
        print(f"Unexpected status: {result.status.name()}")
        sys.exit(1)


if __name__ == "__main__":
    main()
