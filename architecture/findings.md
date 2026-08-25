# Findings Module

## Purpose

Canonical `Finding` schema with confidence levels, evidence kinds, and lifecycle management. Defines the target data model for unifying finding representations across all Eggsec modules. The module is the authoritative definition of the finding structure — existing module-specific types (`output::agent::AgentFinding`, `tool::finding::Finding`, `workflow::finding::Finding`) are NOT yet migrated to this schema.

## Role & Responsibilities

- Define the canonical `Finding` record (19 fields) with rich metadata
- Provide `Confidence` (5 variants) and `EvidenceKind` (13 variants) enums
- Compute stable SHA-256 fingerprints for cross-scan deduplication
- Manage finding lifecycle states (`FindingStatus`, 6 variants) with valid transition enforcement
- Persist findings as JSONL via `FindingStore` with dedup-by-fingerprint on write
- Record scan runs with finding counts for historical analysis

## Location & Feature Gating

**Crate**: `crates/eggsec/src/findings/` (3 files, engine crate)

Always compiled. No feature gating. Dependencies: `chrono`, `serde`, `serde_json`, `sha2`, `hex`, `parking_lot`, `anyhow`.

| File | Lines | Tests | Purpose |
|------|-------|-------|---------|
| `mod.rs` | 485 | 13 | All canonical types: `Finding`, `Confidence`, `EvidenceKind`, `Evidence`, `AffectedAsset`, `FindingLocation`, `Reproduction`, `FindingType`, `FindingSource`; `compute_fingerprint()`, `Confidence::from_ratio()` |
| `store.rs` | 297 | 4 | `FindingStore` — JSONL-based persistent file storage with deduplication via fingerprint |
| `lifecycle.rs` | 237 | 7 | `FindingStatus` (6 variants), `StoredFinding`, `StatusChange`, `ScanRun`; valid transition enforcement |

**Test-bearing files**: 3 of 3 (24 total tests).

## Architecture

### Key Types

| Type | Location | Description |
|------|----------|-------------|
| `Finding` | `findings/mod.rs:252` | Canonical finding record (19 fields) |
| `Confidence` | `findings/mod.rs:37` | Confidence level: 5 variants |
| `EvidenceKind` | `findings/mod.rs:88` | Category of evidence data: 13 variants |
| `Evidence` | `findings/mod.rs:126` | A piece of supporting evidence with kind, summary, data, redaction flag |
| `AffectedAsset` | `findings/mod.rs:161` | Asset affected by a finding (type, identifier, host, port, protocol) |
| `FindingLocation` | `findings/mod.rs:176` | Where the finding was observed (URL, path, parameter, header, method, line, file) |
| `Reproduction` | `findings/mod.rs:194` | Steps to reproduce (steps, expected, actual) |
| `FindingType` | `findings/mod.rs:207` | High-level classification: 9 variants |
| `FindingSource` | `findings/mod.rs:237` | Tool/module provenance (tool, module, run_id) |
| `FindingStore` | `findings/store.rs:9` | JSONL-based persistent file storage |
| `StoredFinding` | `findings/lifecycle.rs:50` | Finding with lifecycle metadata and status history |
| `FindingStatus` | `findings/lifecycle.rs:6` | Lifecycle status: 6 variants |
| `StatusChange` | `findings/lifecycle.rs:59` | Status transition record with timestamp and note |
| `ScanRun` | `findings/lifecycle.rs:107` | Scan run record with finding counts |

### Finding Struct Fields (`mod.rs:252-291`) — 19 fields

| # | Field | Type | Description |
|---|-------|------|-------------|
| 1 | `id` | `String` | Unique identifier for this finding instance |
| 2 | `fingerprint` | `String` | Stable fingerprint for deduplication across scan runs |
| 3 | `title` | `String` | Short human-readable title |
| 4 | `description` | `String` | Detailed description of the finding |
| 5 | `severity` | `crate::types::Severity` | Severity rating (canonical `Severity` from `types.rs`) |
| 6 | `confidence` | `Confidence` | How confident we are this is a true positive |
| 7 | `finding_type` | `FindingType` | High-level classification |
| 8 | `cwe` | `Option<String>` | CWE identifier (e.g. "CWE-79") |
| 9 | `owasp` | `Option<String>` | OWASP category (e.g. "A03:2021-Injection") |
| 10 | `cve` | `Option<String>` | CVE identifier (e.g. "CVE-2024-1234") |
| 11 | `affected_asset` | `AffectedAsset` | The affected asset |
| 12 | `location` | `FindingLocation` | Where within the asset the finding was observed |
| 13 | `evidence` | `Vec<Evidence>` | Supporting evidence |
| 14 | `reproduction` | `Option<Reproduction>` | Steps to reproduce |
| 15 | `remediation` | `Option<String>` | Recommended remediation |
| 16 | `discovered_at` | `DateTime<Utc>` | When this finding was discovered |
| 17 | `source` | `FindingSource` | Which tool/module produced this finding |
| 18 | `tags` | `Vec<String>` | Freeform tags for filtering and grouping |
| 19 | `metadata` | `serde_json::Value` | Additional metadata as key-value pairs |

### Confidence Variants (5, `mod.rs:37-43`)

| Variant | Score | `from_ratio()` threshold |
|---------|-------|--------------------------|
| `Confirmed` | 1.0 | ≥ 0.9 |
| `High` | 0.75 | ≥ 0.6 |
| `Medium` | 0.5 | ≥ 0.3 |
| `Low` | 0.25 | > 0.0 |
| `Informational` | 0.0 | 0.0 or tested=0 |

### EvidenceKind Variants (13, `mod.rs:88-102`)

`HttpRequest`, `HttpResponse`, `Header`, `BodySnippet`, `Timing`, `Diff`, `Banner`, `DnsRecord`, `Certificate`, `PortState`, `Screenshot`, `FilePath`, `LogLine`

**Note**: This is distinct from the envelope's `EvidenceKind` (20 variants) in `eggsec-output/src/envelope.rs:32`. The findings module defines the canonical evidence schema; the envelope defines a broader protocol-neutral taxonomy for cross-domain reports.

### FindingType Variants (9, `mod.rs:207-217`)

`Vulnerability`, `Misconfiguration`, `InformationLeak`, `PolicyViolation`, `AssetDiscovery`, `ServiceDetection`, `WafDetection`, `FuzzResult`, `ScanResult`

### FindingStatus Variants (6, `lifecycle.rs:6-13`)

| Variant | Description |
|---------|-------------|
| `New` | Initial state when first discovered |
| `Confirmed` | Verified as a true positive |
| `AcceptedRisk` | Acknowledged but accepted |
| `FalsePositive` | Determined to be a false alarm |
| `Remediated` | Fix has been applied |
| `Reopened` | Previously remediated but found again |

### Valid Transitions (`lifecycle.rs:17-27`)

| From | Valid Targets |
|------|---------------|
| `New` | `Confirmed`, `FalsePositive`, `AcceptedRisk` |
| `Confirmed` | `Remediated`, `AcceptedRisk`, `FalsePositive` |
| `AcceptedRisk` | `Reopened`, `FalsePositive` |
| `FalsePositive` | `Reopened` |
| `Remediated` | `Reopened` |
| `Reopened` | `Confirmed`, `FalsePositive`, `AcceptedRisk` |

Invalid transitions return `Err` via `StoredFinding::change_status()` (`lifecycle.rs:80-102`).

## Behavior/Flow

### Fingerprint Computation (`mod.rs:299-321`)

`Finding::compute_fingerprint()` generates a stable SHA-256 hex string from:

1. `affected_asset.asset_type` (bytes)
2. `affected_asset.identifier` (lowercased bytes)
3. `finding_type` (Debug format bytes)
4. `location.path` (lowercased bytes, if present)
5. `location.parameter` (lowercased bytes, if present)
6. `cwe` (bytes, if present)
7. `title` (lowercased + trimmed bytes)

Deterministic across scan runs when the same issue is rediscovered on the same asset. Case-insensitive for title, identifier, path, and parameter.

`refresh_fingerprint()` (`mod.rs:324`) recomputes and stores in-place.

### FindingStore Persistence (`store.rs`)

JSONL-based file storage with two files:
- `findings.jsonl` — one `StoredFinding` per line
- `scan_runs.jsonl` — one `ScanRun` per line

**Write path** (`store_finding()`, `store.rs:38-62`):
1. Acquire `parking_lot::Mutex` guard
2. Load all findings from JSONL
3. Check for existing fingerprint match → update in-place if found
4. Otherwise append new `StoredFinding` as JSON line
5. **Non-atomic**: Full file rewrite on update; append-only on new finding

**Dedup on write**: Findings are deduplicated by `fingerprint` field. If a finding with the same fingerprint exists, the existing record is updated (not duplicated).

### ScanRun Recording (`store.rs:127-138`)

Append-only JSONL. Each `ScanRun` captures: `id`, `started_at`, `completed_at`, `target`, `findings_count`, `new_findings_count`, `resolved_findings_count`.

## Integration Points

### Who Produces Findings

- **Engine modules** (scanner, fuzzer, waf, recon, auth, etc.): Produce module-specific finding types that are NOT yet the canonical `Finding`
- **`output::agent::AgentFinding`**: The output crate's finding type used by dedup, baseline, trend — distinct from canonical `Finding`
- **`ReportSummary::from_findings()`** (`output/report_summary.rs:27`): Consumes canonical `Finding` slices
- **`output::envelope::FindingRecord`**: Protocol-neutral finding record in `ReportEnvelope`

### FindingStore Consumers

- **CLI handlers**: Store findings from scan results
- **TUI**: Query findings by status for display
- **Pipeline**: Record scan runs with finding counts

### Conversion Path

```
Module-specific types → (NOT YET MIGRATED) → canonical Finding
AgentFinding ←→ FindingData (bidirectional via From impls in convert.rs)
Finding → ReportEnvelope::FindingRecord (via envelope.rs From impl for AgentFinding)
```

## Testing

### Test Counts by File

| File | Tests | Key Coverage |
|------|-------|--------------|
| `mod.rs` | 13 | Fingerprint stability, case insensitivity, title/CWE/type changes, confidence scores, `from_ratio()`, serialization roundtrip, refresh, evidence constructors |
| `store.rs` | 4 | Store+load, update status, findings_by_status, record+load run |
| `lifecycle.rs` | 7 | Starts as New, history recording, invalid transition rejection, valid transitions from New, display strings, serialization, ScanRun serialization |

**Total**: 24 tests.

## Invariants & Gotchas

1. **Canonical `Finding` is NOT yet the working type** — existing modules produce `AgentFinding` or module-specific types; migration is pending
2. **`FindingStore` uses `parking_lot::Mutex`** (not `std::sync::Mutex`) — no poisoning, but contention possible under high concurrency
3. **`FindingStore` is NOT atomic** — `write_findings_inner()` (`store.rs:164`) truncates and rewrites the file; crash during write corrupts the store
4. **Fingerprint does NOT include severity** — two findings with the same asset/type/location/CWE/title but different severities produce the same fingerprint
5. **`FindingStatus::valid_transitions()` is exhaustive** — invalid transitions return `Err`, not panic
6. **`FindingStore::store_finding()`** updates in-place on fingerprint match, not append — the `updated_at` field is NOT updated on the `StoredFinding` when updating via store (it updates the inner `finding` field only)
7. **`ScanRun` is append-only** — no dedup; duplicate run IDs will coexist
8. **No index on `findings_by_status()`** — linear scan of all findings for each query

## Bug Sweep (Report Only)

| Location | Issue | Severity |
|----------|-------|----------|
| `store.rs:164-170` | `write_findings_inner()` uses `fs::File::create()` which truncates before writing. A crash after truncate but before completing the write loop loses all findings. No temp-file-and-rename. | Medium |
| `store.rs:47-49` | When updating an existing finding by fingerprint, the `StoredFinding.updated_at` is NOT updated (only the inner `finding` field is replaced). The `updated_at` remains from the original creation. | Low |
| `store.rs:52` | `StoredFinding::new(finding, "")` passes empty string as `scan_id` for updated findings, losing the original scan provenance. | Low |
| `mod.rs:299-321` | `compute_fingerprint()` does not include `severity` in the hash. Two findings with identical everything except severity will have the same fingerprint, causing one to overwrite the other in `FindingStore`. | Low |
| `lifecycle.rs:96` | `StoredFinding::change_status()` calls `chrono::Utc::now()` twice (once for `StatusChange.changed_at` and once for `self.updated_at`), which could yield different timestamps if the clock advances between calls. | Informational |

---

*Last verified against source: 2026-08-25*
