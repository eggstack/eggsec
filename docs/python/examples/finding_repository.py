#!/usr/bin/env python3
"""Finding repository storage and baseline comparison.

Demonstrates storing findings in an in-memory SQLite repository,
querying by severity, and running a baseline comparison. No network
required.

Requirements:
    - eggsec (default features)

Usage:
    python3 docs/python/examples/finding_repository.py
"""

import json

from eggsec import SqliteFindingRepository


FINDINGS = [
    {"id": "f1", "title": "SQL Injection in login", "severity": "high",
     "finding_type": "vulnerability", "state": "open"},
    {"id": "f2", "title": "Missing HSTS header", "severity": "medium",
     "finding_type": "misconfiguration", "state": "open"},
    {"id": "f3", "title": "Open redirect", "severity": "low",
     "finding_type": "vulnerability", "state": "open"},
]


def main():
    with SqliteFindingRepository(":memory:") as repo:
        repo.initialize()

        # Insert findings
        for f in FINDINGS:
            repo.insert_finding(json.dumps(f))
        print(f"Inserted {len(FINDINGS)} findings")

        # Query by severity
        high = repo.query_findings(severity="high")
        medium = repo.query_findings(severity="medium")
        print(f"High: {len(high)}, Medium: {len(medium)}, Total: {repo.count_findings()}")

        # Deduplication check
        dup = repo.deduplicate("sql-injection-login")
        print(f"Dedup check: {dup}")

        # Update a finding
        updated = json.dumps({"id": "f1", "title": "SQL Injection (fixed)",
                              "severity": "info", "state": "resolved"})
        repo.update_finding("f1", updated)
        print(f"After update: {repo.get_finding('f1')[:60]}...")

        # Baseline comparison
        baseline_count = repo.count_findings()
        new_finding = {"id": "f4", "title": "XSS in search", "severity": "high",
                       "finding_type": "vulnerability", "state": "open"}
        repo.insert_finding(json.dumps(new_finding))
        new_count = repo.count_findings()
        print(f"\nBaseline: {baseline_count}, New: {new_count}, Delta: {new_count - baseline_count}")


if __name__ == "__main__":
    main()
