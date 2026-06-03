# Findings Module

## Purpose

Canonical `Finding` schema with confidence levels, evidence kinds, and lifecycle management. Defines the target data model for unifying finding representations across all Slapper modules.

## Key Types

| Type | Location | Description |
|------|----------|-------------|
| `Finding` | `findings/mod.rs` | Canonical finding record with fingerprint, severity, confidence, evidence |
| `Confidence` | `findings/mod.rs` | Confidence level: Confirmed, High, Medium, Low, Informational (5 variants) |
| `EvidenceKind` | `findings/mod.rs` | Category of evidence data (HTTP, Screenshot, Log, etc.) - 13 variants |
| `Evidence` | `findings/mod.rs` | A piece of supporting evidence with kind and content |
| `AffectedAsset` | `findings/mod.rs` | Asset affected by a finding (URL, IP, Host, etc.) |
| `FindingLocation` | `findings/mod.rs` | Where the finding was observed |
| `Reproduction` | `findings/mod.rs` | Steps to reproduce the finding |
| `FindingType` | `findings/mod.rs` | High-level classification - 9 variants |
| `FindingSource` | `findings/mod.rs` | Which tool/module produced the finding |
| `FindingStore` | `findings/store.rs` | JSONL-based persistent file storage with deduplication |
| `FindingStatus` | `findings/lifecycle.rs` | Finding lifecycle status - 6 variants |
| `StoredFinding` | `findings/lifecycle.rs` | Finding with lifecycle metadata and status history |
| `StatusChange` | `findings/lifecycle.rs` | Status transition record with timestamp and note |
| `ScanRun` | `findings/lifecycle.rs` | Scan run record with finding counts |

## Files

| File | Description |
|------|-------------|
| `mod.rs` | Module root: all canonical types, `Confidence::score()`, `Confidence::from_ratio()` |
| `store.rs` | `FindingStore` for JSONL-based persistent file storage (`findings.jsonl`) and deduplication |
| `lifecycle.rs` | Finding lifecycle state machine (status transitions) |

## Finding Struct Fields (`mod.rs:252-291`)

All 19 fields of the canonical `Finding`:

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

## FindingStatus Variants (`lifecycle.rs:6-13`)

```rust
pub enum FindingStatus {
    New,            // Initial state when first discovered
    Confirmed,      // Verified as a true positive
    AcceptedRisk,   // Acknowledged but accepted
    FalsePositive,  // Determined to be a false alarm
    Remediated,     // Fix has been applied
    Reopened,       // Previously remediated but found again
}
```

Each transition is recorded in `status_history: Vec<StatusChange>` with timestamps and optional notes.

## Confidence Divergence

**IMPORTANT**: There are THREE separate `Confidence` enums in the codebase with different variants:

| Module | Variants | Score Mapping |
|--------|----------|---------------|
| `findings/mod.rs` | `Confirmed`, `High`, `Medium`, `Low`, `Informational` (5) | 1.0, 0.75, 0.5, 0.25, 0.0 |
| `output/agent.rs` | `Confirmed`, `Likely`, `Possible`, `Unlikely` (4) | 1.0, 0.75, 0.5, 0.25 |
| `recon/secrets.rs` | `High`, `Medium`, `Low` (3) | Similar |

The `findings` module includes an `Informational` variant (score 0.0) that the other modules lack. The naming also diverges (`High`/`Medium`/`Low` vs `Likely`/`Possible`/`Unlikely`). This is a known divergence — the findings module defines the target canonical schema, and the other modules have not yet been migrated.

## Implementation Status

Fully implemented. Canonical `Finding` schema is defined with rich metadata. Module notes that existing module-specific types are not yet migrated to this canonical schema.
