# Baselines and Comparison

This guide covers using the StreamingDiffReporter for baseline comparison.

## Creating a Baseline

A baseline is a JSON document containing a `findings` array:

```json
{
  "findings": [
    {"id": "f1", "severity": "high", "title": "SQL Injection"},
    {"id": "f2", "severity": "medium", "title": "Open Redirect"}
  ]
}
```

## Comparing Against Baseline

```python
from eggsec import StreamingDiffReporter, StreamingReportConfig

config = StreamingReportConfig(format="jsonl", output_path="/tmp/diff.jsonl")

baseline = '{"findings":[{"id":"f1","severity":"high","title":"SQL Injection"}]}'

reporter = StreamingDiffReporter(config, baseline_json=baseline)
reporter.start()

# Finding f1 with changed severity
r1 = reporter.write_finding('{"id":"f1","severity":"critical","title":"SQL Injection"}')
print(r1.diff_status)  # "changed"
print(r1.changes)       # ["severity: high -> critical"]

# New finding not in baseline
r2 = reporter.write_finding('{"id":"f3","severity":"low","title":"Info Leak"}')
print(r2.diff_status)  # "new"

summary = reporter.finish()
print(f"New: {summary.new_findings}, Changed: {summary.changed_findings}")
```

## Diff Status Values

- `"new"`: Finding exists in current but not in baseline
- `"changed"`: Finding exists in both but fields differ
- `"unchanged"`: Finding exists in both with identical fields
- `"resolved"`: Finding exists in baseline but not in current

## Deterministic Output

The diff reporter produces deterministic output for the same inputs, making it suitable for CI/CD comparison.
