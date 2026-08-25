# Diff Modules

## Purpose

There is no single "diff module." Finding comparison, response diffing, and diff summaries are spread across three separate module trees: `output`, `fuzzer`, and `waf`. The output crate provides finding-level comparison and a numeric summary envelope. The fuzzer provides HTTP response-level diffing with anomaly scoring. The WAF provides a separate response comparison type for detection.

**Note on overview.md**: The overview lists Diff with source `(in eggsec-output + engine)`. This is accurate — the finding-level comparison and summary struct live in `eggsec-output`, while the HTTP response diff engine lives in the engine crate (`fuzzer/diff.rs`). The WAF's `ResponseDiff` is unrelated.

---

## 1. Output Baseline Comparison

**File:** `crates/eggsec-output/src/baseline.rs` (192 lines)

Compares two `AgentFinding` slices by `id` field and classifies results into new, resolved, and unchanged categories. This is the actual finding-level comparison engine.

### Key Types

| Type | Location | Description |
|------|----------|-------------|
| `BaselineComparison` | `baseline.rs:5` | Result of comparing current findings against a baseline |

### Fields (`baseline.rs:5-9`)

| Field | Type |
|-------|------|
| `new_findings` | `Vec<AgentFinding>` |
| `resolved_findings` | `Vec<AgentFinding>` |
| `unchanged_findings` | `Vec<AgentFinding>` |

### Functions

| Method | Line | Signature |
|--------|------|-----------|
| `BaselineComparison::compare()` | `baseline.rs:12` | `fn compare(current: &[AgentFinding], baseline: &[AgentFinding]) -> Self` |
| `has_new_findings()` | `baseline.rs:41` | `fn has_new_findings(&self) -> bool` |
| `new_finding_count()` | `baseline.rs:45` | `fn new_finding_count(&self) -> usize` |

### Matching Logic (`baseline.rs:12-38`)

Uses `FxHashSet` of finding `id` fields:

1. Build `baseline_ids: FxHashSet<String>` from `baseline.iter().map(|f| f.id.clone())`
2. Build `current_ids: FxHashSet<String>` from `current.iter().map(|f| f.id.clone())`
3. **New**: IDs in `current` but not in `baseline` — `current.iter().filter(|f| !baseline_ids.contains(&f.id))`
4. **Resolved**: IDs in `baseline` but not in `current` — `baseline.iter().filter(|f| !current_ids.contains(&f.id))`
5. **Unchanged**: IDs present in both — `current.iter().filter(|f| baseline_ids.contains(&f.id))`

### Semantics

- Matching is purely by `AgentFinding.id` (UUID string)
- No fingerprint-based comparison
- No severity escalation/de-escalation detection
- No temporal ordering — `compare()` is a point-in-time snapshot
- Both `new_findings` and `unchanged_findings` come from `current`; `resolved_findings` from `baseline`
- All vectors contain cloned `AgentFinding` values (not references)

### Limitations

- Uses `AgentFinding` (from `output::agent`), not the canonical `Finding` (from `findings::mod`)
- No escalation/de-escalation tracking
- No fingerprint-based matching (fingerprints exist in `findings/mod.rs:299-321` but are only used for `FindingStore` deduplication)
- No diff summary production — callers must compute counts from the result vectors

### Tests

10 tests in `baseline.rs:80-192`: no changes, new findings, resolved findings, mixed, empty baseline, empty current, both empty, `has_new_findings()`, `has_no_new_findings()`, `new_finding_count()`.

---

## 2. Output Diff Summary

**File:** `crates/eggsec-output/src/diff.rs` (27 lines)

A minimal numeric summary struct for attaching diff results to pipeline run manifests. This is a data envelope, not a comparison engine.

### Key Types

| Type | Location | Description |
|------|----------|-------------|
| `DiffSummary` | `diff.rs:4` | Counts of new, resolved, escalated, de-escalated findings plus net change |

### Fields (`diff.rs:4-10`)

| Field | Type |
|-------|------|
| `total_new` | `usize` |
| `total_resolved` | `usize` |
| `total_escalated` | `usize` |
| `total_deescalated` | `usize` |
| `net_change` | `i32` |

### Usage

- Re-exported at `output/mod.rs:58`
- Used in `RunManifest` (`output/run_manifest.rs:56`) as `diff_summary: Option<DiffSummary>`
- Populated via `with_baseline()` at `run_manifest.rs:93`
- `net_change` = `total_new - total_resolved` (caller must compute)

### Relationship to BaselineComparison

`DiffSummary` is the numeric companion to `BaselineComparison`. A typical workflow:

```
let comparison = BaselineComparison::compare(&current, &baseline);
let diff_summary = DiffSummary {
    total_new: comparison.new_finding_count(),
    total_resolved: comparison.resolved_findings.len(),
    total_escalated: 0,    // not tracked by BaselineComparison
    total_deescalated: 0,  // not tracked by BaselineComparison
    net_change: comparison.new_finding_count() as i32 - comparison.resolved_findings.len() as i32,
};
```

**Note**: `total_escalated` and `total_deescalated` are always 0 when produced from `BaselineComparison` because severity change detection is not implemented.

### Tests

1 test in `diff.rs:17` verifying struct construction.

---

## 3. Fuzzer Response Diff Engine

**File:** `crates/eggsec/src/fuzzer/diff.rs` (336 lines)

HTTP response diff engine for comparing responses during fuzzing sessions. Detects anomalies by scoring differences between a baseline and current response. This is a fully independent system from the output diff modules.

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
| `baseline_snapshot()` | `diff.rs:114` | Returns reference to current baseline snapshot |
| `capture_baseline()` | `diff.rs:118` | Captures and sets baseline from raw response |
| `diff()` | `diff.rs:130` | Compares current response against baseline |
| `is_anomaly()` | `diff.rs:302` | Returns true if anomaly score ≥ threshold (default 0.3) |

### Anomaly Scoring (`diff.rs:213-299`)

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

Scores are cumulative. `is_anomaly()` returns `true` when `anomaly_score >= min_anomaly_threshold` (default 0.3).

### Body Comparison

Body comparison is hash-based (`sha2::Sha256`), not content-based. Only body *length* differences are scored (> 1000 bytes threshold). Two bodies with identical length but different content produce no anomaly score. This is by design — the fuzzer uses this to detect response *changes*, not content vulnerabilities.

### Default Ignore Headers

`["date", "content-length", "connection", "keep-alive"]` — always ignored in header comparison.

### Integration

Used in `FuzzEngine` (`fuzzer/engine/core.rs:106`) as `differ: Option<ResponseDiffer>`. Re-exported at `fuzzer/mod.rs:119`.

### Tests

1 test in `diff.rs:312` verifying baseline setup. Limited coverage — anomaly scoring logic is not tested.

---

## 4. WAF Response Diff (Separate)

**File:** `crates/eggsec/src/waf/detector/types.rs:25-34`

A completely separate `ResponseDiff` type for WAF detection, comparing normal vs. malicious request responses. Uses different logic (`is_waf_blocked()`) checking status codes, length diffs, and header keywords. Not related to the output or fuzzer diff modules.

---

## Cross-Module Relationships

```
eggsec-output/src/baseline.rs  → AgentFinding (finding-level comparison by id)
eggsec-output/src/diff.rs      → DiffSummary (numeric summary for RunManifest)
output/run_manifest.rs          → uses DiffSummary
fuzzer/diff.rs                  → ResponseDiffer (HTTP response-level comparison)
fuzzer/engine/core.rs           → uses ResponseDiffer
waf/detector/types.rs           → ResponseDiff (WAF detection, unrelated)
findings/mod.rs                 → compute_fingerprint() (deduplication only, not used by any diff)
```

## Missing Functionality

The following capabilities are **not implemented** despite being plausible:

- **Fingerprint-based finding comparison** — fingerprints exist (`findings/mod.rs:299-321`) but no diff uses them; `BaselineComparison` matches by `id` only
- **Finding-level escalation/de-escalation tracking** — `DiffSummary` has fields for this but no code populates them from `BaselineComparison`
- **A unified diff API** that works with canonical `Finding` instead of `AgentFinding`
- **CLI commands that expose diff functionality directly** — diff is available only through `RunManifest` in pipeline output
- **Severity-weighted diff** — `DiffSummary` tracks counts but not severity impact changes

## Implementation Status

**Partially implemented.** Finding comparison exists but is split across `baseline.rs` (comparison logic) and `diff.rs` (summary struct). Response diffing is fully implemented for the fuzzer. There is no unified diff module.

## Testing

| Module | File | Tests |
|--------|------|-------|
| Baseline comparison | `baseline.rs` | 10 |
| Diff summary | `diff.rs` | 1 |
| Fuzzer response diff | `fuzzer/diff.rs` | 1 |

**Total**: 12 tests across all diff-related code.

## Invariants & Gotchas

1. **`BaselineComparison` uses `AgentFinding.id`**, not fingerprint — two different findings with the same fingerprint but different IDs will be treated as distinct
2. **`DiffSummary.total_escalated` and `total_deescalated`** are dead fields when produced from `BaselineComparison` — no severity-change detection exists
3. **Fuzzer `ResponseDiffer` body comparison is hash-based** — identical-length bodies with different content produce no anomaly score
4. **Fuzzer anomaly scoring is cumulative** — many small changes can trigger anomaly even if no single change exceeds threshold
5. **WAF `ResponseDiff` is completely separate** — shares only the name with fuzzer/output diff types
6. **`BaselineComparison::compare()` clones all findings** — O(n) memory for each input vector; no streaming/iterator-based approach

## Bug Sweep (Report Only)

| Location | Issue | Severity |
|----------|-------|----------|
| `fuzzer/diff.rs:173` | `value.to_str().unwrap_or("").to_string()` — non-UTF-8 header values silently become empty strings. Not a crash (unwrap_or), but lossy. | Informational |
| `baseline.rs` | No tests for `BaselineComparison` with duplicate IDs in input (e.g., two findings with the same `id` in `current`). Behavior: both would appear in `unchanged_findings`. | Informational |
| `diff.rs` | `DiffSummary` has no constructor or builder — callers must construct all 5 fields manually. Easy to miscompute `net_change`. | Informational |

---

*Last verified against source: 2026-08-25*
