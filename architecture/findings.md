# Findings Module

## Purpose

Canonical `Finding` schema with confidence levels, evidence kinds, and lifecycle management. Defines the target data model for unifying finding representations across all Slapper modules.

## Key Types

| Type | Location | Description |
|------|----------|-------------|
| `Finding` | `findings/mod.rs` | Canonical finding record with fingerprint, severity, confidence, evidence |
| `Confidence` | `findings/mod.rs` | Confidence level: Confirmed, High, Medium, Low, Informational |
| `EvidenceKind` | `findings/mod.rs` | Category of evidence data (HTTP, Screenshot, Log, etc.) |
| `Evidence` | `findings/mod.rs` | A piece of supporting evidence with kind and content |
| `AffectedAsset` | `findings/mod.rs` | Asset affected by a finding (URL, IP, Host, etc.) |
| `FindingLocation` | `findings/mod.rs` | Where the finding was observed |
| `Reproduction` | `findings/mod.rs` | Steps to reproduce the finding |
| `FindingType` | `findings/mod.rs` | High-level classification |
| `FindingSource` | `findings/mod.rs` | Which tool/module produced the finding |
| `FindingStore` | `findings/store.rs` | JSONL-based persistent file storage with deduplication |
| `FindingStatus` | `findings/lifecycle.rs` | Finding status transitions (6 states) |

## Files

| File | Description |
|------|-------------|
| `mod.rs` | Module root: all canonical types, `Confidence::score()`, `Confidence::from_ratio()` |
| `store.rs` | `FindingStore` for JSONL-based persistent file storage (`findings.jsonl`) and deduplication |
| `lifecycle.rs` | Finding lifecycle state machine (status transitions) |

## Implementation Status

Fully implemented. Canonical `Finding` schema is defined with rich metadata. Module notes that existing module-specific types are not yet migrated to this canonical schema.
