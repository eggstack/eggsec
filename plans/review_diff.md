# Diff Module Architecture Review

**Document:** architecture/diff.md
**Reviewed:** 2026-06-02
**Accuracy:** High
**Lines Reviewed:** 23

## Verified Claims
- [DiffResult]: Verified at `crates/slapper/src/diff/mod.rs:9`
- [FindingChange]: Verified at `crates/slapper/src/diff/mod.rs:18`
- [DiffSummary]: Verified at `crates/slapper/src/diff/mod.rs:29`
- [diff_findings() function]: Verified at `crates/slapper/src/diff/mod.rs:39`
- [load_findings_from_file() function]: Verified at `crates/slapper/src/diff/mod.rs:103`

## Discrepancies
- [Document says "JSONL file" but code uses JSON]: The document states `load_findings_from_file()` loads from a "single JSONL file", but `crates/slapper/src/diff/mod.rs:105` uses `serde_json::from_str(&content)` which parses standard JSON, not JSONL (newline-delimited JSON) (priority: low)

## Bugs Found
- None found.

## Improvement Opportunities
- [Fingerprint collision possible]: The diff logic at `crates/slapper/src/diff/mod.rs:40-48` uses `fingerprint` as the key. If two different findings have the same fingerprint (hash collision), one will be lost. Consider using a HashMap with Vec<Vec<Finding>> to handle collisions (priority: low)
- [No evidence change detection for severity changes]: When a finding changes severity, only the severity is compared. If evidence also changed, that's tracked separately via `evidence_changed` flag (line 68), but the old/new evidence content is not stored in `FindingChange` (priority: low)

## Stale Items
- None.

## Code Interrogation Findings
- [format_diff_text() not documented]: The module has a `format_diff_text()` function at line 110 that formats diff results as human-readable text, but it's not mentioned in the architecture document. This is a useful utility that should be documented.
- [Uses std HashMap instead of FxHashMap]: Line 4 imports `std::collections::HashMap` rather than `rustc_hash::FxHashMap`. For large finding sets, this could be a performance concern.