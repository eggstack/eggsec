# Repositories and SQLite Storage

This guide covers the persistent finding and assessment repositories.

## Finding Repository

```python
from eggsec import SqliteFindingRepository

repo = SqliteFindingRepository(":memory:")
repo.initialize()

# Insert findings as JSON
finding_id = repo.insert_finding('{
    "id": "f1",
    "title": "SQL Injection in login",
    "severity": "critical",
    "state": "open",
    "finding_type": "vuln"
}')

# Query findings
results = repo.query_findings(severity="critical", limit=10)
count = repo.count_findings(severity="critical")

# Deduplication
existing = repo.deduplicate("dedup-key-123")
if existing:
    print(f"Duplicate found: {existing}")

# Update
repo.update_finding("f1", '{"id":"f1","title":"Fixed SQL Injection","severity":"low","state":"resolved"}')

# Delete
repo.delete_finding("f1")
```

## Assessment Repository

```python
from eggsec import SqliteAssessmentRepository

repo = SqliteAssessmentRepository(":memory:")
repo.initialize()

# Create assessment
assess_id = repo.create_assessment("Q3 Pentest", "example.com", "full-scan")

# Attach findings
repo.attach_finding(assess_id, "f1")
repo.attach_finding(assess_id, "f2")

# Attach artifacts
repo.attach_artifact(assess_id, '{"type":"pcap","path":"/tmp/capture.pcap"}')

# Update state
repo.update_assessment_state(assess_id, "in_progress")

# List with pagination
assessments = repo.list_assessments(limit=10, offset=0)
```

## Migration

```python
from eggsec import SqliteMigration

migration = SqliteMigration(
    version=1,
    description="Initial schema",
    applied_at_ms=1234567890,
)
```

## Context Manager Support

```python
with SqliteFindingRepository("findings.db") as repo:
    repo.initialize()
    repo.insert_finding('{"id":"f1","severity":"high"}')
# Repository is closed on exit
```
