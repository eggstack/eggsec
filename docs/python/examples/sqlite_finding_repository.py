#!/usr/bin/env python3
"""SQLite finding and assessment repository.

Demonstrates persisting findings and assessments using SQLite-backed
repositories with deduplication, query, and pagination.

Requirements:
    - eggsec (default features)

Usage:
    python3 docs/python/examples/sqlite_finding_repository.py
"""

import tempfile
import os
import json

from eggsec import (
    SqliteFindingRepository,
    SqliteAssessmentRepository,
)


def main():
    db_path = os.path.join(tempfile.gettempdir(), "demo-findings.db")

    with SqliteFindingRepository(db_path) as repo:
        repo.initialize()

        fid1 = repo.insert_finding(json.dumps({
            "id": "f1",
            "title": "SQL Injection in login",
            "severity": "high",
            "finding_type": "vulnerability",
            "state": "open",
        }))
        print(f"Inserted finding: {fid1}")

        fid2 = repo.insert_finding(json.dumps({
            "id": "f2",
            "title": "Missing HSTS header",
            "severity": "medium",
            "finding_type": "misconfiguration",
            "state": "open",
        }))
        print(f"Inserted finding: {fid2}")

        finding = repo.get_finding("f1")
        print(f"Retrieved: {finding[:80]}...")

        high = repo.query_findings(severity="high")
        print(f"High severity findings: {len(high)}")

        count = repo.count_findings(severity="medium")
        print(f"Medium count: {count}")

        dup = repo.deduplicate("dk1")
        print(f"Dedup lookup: {dup}")

    with SqliteAssessmentRepository(db_path) as arepo:
        arepo.initialize()

        aid = arepo.create_assessment("Login Audit", "app.example.com", "pentest")
        print(f"Created assessment: {aid}")

        arepo.attach_finding(aid, "f1")
        arepo.attach_finding(aid, "f2")

        arepo.update_assessment_state(aid, "in_progress")

        assessment = arepo.get_assessment(aid)
        print(f"Assessment: {assessment[:100]}...")

        all_assessments = arepo.list_assessments()
        print(f"Total assessments: {len(all_assessments)}")

    os.unlink(db_path)
    print("Database cleaned up")


if __name__ == "__main__":
    main()
