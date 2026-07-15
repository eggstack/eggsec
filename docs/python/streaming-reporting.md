# Streaming Reporting

This guide covers streaming report generation and diff reporting.

## Basic Streaming Reporter

```python
from eggsec import StreamingReporter, StreamingReportConfig

config = StreamingReportConfig(
    format="jsonl",
    output_path="/tmp/report.jsonl",
    buffer_size=100,
    redact_secrets=True,
)

reporter = StreamingReporter(config)
reporter.start()

# Write findings as JSON strings
reporter.write_finding('{"id":"f1","severity":"critical","title":"SQL Injection"}')
reporter.write_finding('{"id":"f2","severity":"high","title":"XSS"}')

# Batch write
reporter.write_findings_batch('[{"id":"f3","severity":"medium"}]')

# Check buffer
print(f"Buffered: {reporter.get_buffered_count()}")

# Finalize
summary = reporter.finish()
print(f"Total findings: {summary.total_findings}")
print(f"Output: {summary.output_path}")
print(f"Size: {summary.output_size_bytes} bytes")
print(f"Hash: {summary.content_hash}")
```

## Diff Reporter

Compare current findings against a baseline:

```python
from eggsec import StreamingDiffReporter, StreamingReportConfig

config = StreamingReportConfig(format="jsonl", output_path="/tmp/diff.jsonl")

baseline_json = '{"findings":[{"id":"f1","severity":"high","title":"XSS"},{"id":"f2","severity":"medium"}]}'

reporter = StreamingDiffReporter(config, baseline_json=baseline_json)
reporter.start()

# Write current findings - each is compared against baseline
diff1 = reporter.write_finding('{"id":"f1","severity":"critical","title":"XSS Updated"}')
print(f"Status: {diff1.diff_status}")  # "changed" (severity changed)
print(f"Changes: {diff1.changes}")

diff2 = reporter.write_finding('{"id":"f3","severity":"low","title":"New Issue"}')
print(f"Status: {diff2.diff_status}")  # "new"

summary = reporter.finish()
print(f"New: {summary.new_findings}")
print(f"Changed: {summary.changed_findings}")
print(f"Unchanged: {summary.unchanged_findings}")
```

## Report Manifest

```python
from eggsec import ReportManifest

manifest = ReportManifest(
    report_id="rpt-1",
    format="jsonl",
    created_at_ms=1234567890,
    finding_count=42,
    content_hash="abc123...",
    schema_version="1.0.0",
    tool_version="0.1.0",
    artifact_ids=["art-1", "art-2"],
)
```

## Context Manager

```python
with StreamingReporter(config) as reporter:
    reporter.start()
    reporter.write_finding('{"id":"f1","severity":"high"}')
    summary = reporter.finish()
```
