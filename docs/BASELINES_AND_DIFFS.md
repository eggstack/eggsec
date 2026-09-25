# Baselines and Differential Scans

> Status: historical. There is no `eggsec report diff` subcommand in the
> current CLI. Compare scans over time with per-command `--baseline` flags
> (e.g. mobile, fuzz, CI) and the trend/baseline/diff analysis in
> `eggsec-output` (see `architecture/output.md`).

Compare scan results over time to track changes.

## Diff Command

Compare two JSON result files:

```bash
eggsec report diff old.json new.json
```

## Output

The diff shows:
- **New findings**: Issues in new scan not in old
- **Resolved findings**: Issues in old scan not in new
- **Changed findings**: Issues that changed severity/confidence
- **Persisting findings**: Unchanged issues

## Example Output

```text
=== Scan Diff Report ===

Old scan: 5 findings
New scan: 7 findings

--- New Findings (2) ---
  [High] SQL Injection (/api/search)
  [Medium] Information Leak (/api/users)

--- Resolved Findings (1) ---
  [Low] Information Disclosure (/api/status)

--- Persisting Findings (4) ---
```

## JSON Output

```json
{
  "new": [...],
  "resolved": [...],
  "persisting": [...],
  "changed": [...],
  "summary": {
    "new_count": 2,
    "resolved_count": 1,
    "persisting_count": 4,
    "changed_count": 0
  }
}
```
