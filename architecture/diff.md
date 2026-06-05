# Diff Modules

## Purpose

There is no single "diff module." Finding comparison, response diffing, and diff summaries are spread across three separate module trees: `output`, `fuzzer`, and `waf`.

---

## 1. Output Baseline Comparison

**File:** `crates/slapper/src/output/baseline.rs` (192 lines)

Compares two `AgentFinding` slices by `id` field and classifies results into new, resolved, and unchanged categories. This is the actual finding-level comparison engine.

### Key Types

| Type | Location | Description |
|------|----------|-------------|
| `BaselineComparison` | `baseline.rs:4` | Result of comparing current findings against a baseline |

### Functions

| Method | Line | Signature |
|--------|------|-----------|
| `BaselineComparison::compare()` | `baseline.rs:12` | `fn compare(current: &[AgentFinding], baseline: &[AgentFinding]) -> Self` |
| `has_new_findings()` | `baseline.rs:41` | `fn has_new_findings(&self) -> bool` |
| `new_finding_count()` | `baseline.rs:45` | `fn new_finding_count(&self) -> usize` |

### Matching Logic

Matching uses `FxHashSet` of finding `id` fields:
- **New:** IDs in `current` but not in `baseline`
- **Resolved:** IDs in `baseline` but not in `current`
- **Unchanged:** IDs present in both

### Limitations

- Uses `AgentFinding` (from `output::agent`), not the canonical `Finding` (from `findings::mod`)
- No escalation/de-escalation tracking
- No fingerprint-based matching (fingerprints exist in `findings/mod.rs:293-326` but are only used for `FindingStore` deduplication)

### Tests

8 tests in `baseline.rs:50-192` covering no changes, new findings, resolved findings, mixed, empty baseline, empty current, both empty, and count helpers.

---

## 2. Output Diff Summary

**File:** `crates/slapper/src/output/diff.rs` (27 lines)

A minimal numeric summary struct for attaching diff results to pipeline run manifests.

### Key Types

| Type | Location | Description |
|------|----------|-------------|
| `DiffSummary` | `diff.rs:4` | Counts of new, resolved, escalated, de-escalated findings plus net change |

### Fields

| Field | Type |
|-------|------|
| `total_new` | `usize` |
| `total_resolved` | `usize` |
| `total_escalated` | `usize` |
| `total_deescalated` | `usize` |
| `net_change` | `i32` |

### Usage

Re-exported at `output/mod.rs:88`. Used in `RunManifest` (`output/run_manifest.rs:56`) as `diff_summary: Option<DiffSummary>`, populated via `with_baseline()` at `run_manifest.rs:93`.

### Tests

1 test in `diff.rs:17` verifying struct construction.

---

## 3. Fuzzer Response Diff Engine

**File:** `crates/slapper/src/fuzzer/diff.rs` (336 lines)

HTTP response diff engine for comparing responses during fuzzing sessions. Detects anomalies by scoring differences between a baseline and current response.

### Key Types

| Type | Location | Description |
|------|----------|-------------|
| `ResponseDiffer` | `diff.rs:69` | Main engine with baseline, ignore lists, and threshold |
| `ResponseDiff` | `diff.rs:7` | Wrapper: baseline snapshot + diff result |
| `ResponseSnapshot` | `diff.rs:13` | Captures status, headers, body hash, length, content type, timing |
| `HeaderSnapshot` | `diff.rs:23` | Header details including etag, set-cookie, cache-control, server |
| `DiffResult` | `diff.rs:34` | Comparison output with anomaly score |
| `HeaderChange` | `diff.rs:47` | Individual header value change record |

### ResponseDiffer Methods

| Method | Line | Description |
|--------|------|-------------|
| `new()` | `diff.rs:83` | Creates differ with default ignore list (`date`, `content-length`, `connection`, `keep-alive`) |
| `with_ignore_headers()` | `diff.rs:98` | Builder: adds headers to ignore |
| `with_body_patterns()` | `diff.rs:105` | Builder: sets body patterns to ignore |
| `set_baseline()` | `diff.rs:110` | Sets baseline snapshot |
| `capture_baseline()` | `diff.rs:118` | Captures and sets baseline from raw response |
| `diff()` | `diff.rs:130` | Compares current response against baseline |
| `is_anomaly()` | `diff.rs:302` | Returns true if anomaly score >= threshold (default 0.3) |

### Anomaly Scoring

| Change | Score |
|--------|-------|
| Status code change | +0.3 |
| Content-type change | +0.2 |
| Body length diff > 1000 bytes | +0.2 |
| New header | +0.1 |
| Removed header | +0.1 |
| Header value change | +0.05 |
| New cookie | +0.15 |
| Timing increase > 1000ms | +0.2 |

### Integration

Used in `FuzzEngine` (`fuzzer/engine/core.rs:106`) as `differ: Option<ResponseDiffer>`. Re-exported at `fuzzer/mod.rs:119`.

### Tests

1 test in `diff.rs:312` verifying baseline setup.

---

## 4. WAF Response Diff (Separate)

**File:** `crates/slapper/src/waf/detector/types.rs:25-34`

A completely separate `ResponseDiff` type for WAF detection, comparing normal vs. malicious request responses. Uses different logic (`is_waf_blocked()`) checking status codes, length diffs, and header keywords. Not related to the output or fuzzer diff modules.

---

## Cross-Module Relationships

```
output/baseline.rs    -> AgentFinding (finding-level comparison)
output/diff.rs        -> DiffSummary (numeric summary for RunManifest)
output/run_manifest.rs -> uses DiffSummary
fuzzer/diff.rs        -> ResponseDiffer (HTTP response-level comparison)
fuzzer/engine/core.rs -> uses ResponseDiffer
waf/detector/types.rs -> ResponseDiff (WAF detection, unrelated)
findings/mod.rs       -> compute_fingerprint() (deduplication only, not used by any diff)
```

## Missing Functionality

The following capabilities are **not implemented** despite being plausible:
- Fingerprint-based finding comparison (fingerprints exist but no diff uses them)
- Finding-level escalation/de-escalation tracking
- A unified diff API that works with `Finding` instead of `AgentFinding`
- CLI commands that expose diff functionality directly

## Implementation Status

**Partially implemented.** Finding comparison exists but is split across `baseline.rs` (comparison logic) and `diff.rs` (summary struct). Response diffing is fully implemented for the fuzzer. There is no unified diff module as the original architecture document may have implied.
