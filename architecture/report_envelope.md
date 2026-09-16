# Report and Evidence Envelope

## Overview

The normalized report/evidence envelope provides a protocol-neutral contract for report data across all Eggsec domains. Since Phase B (2026-09-16) the canonical owner is `eggsec-report-model` (`crates/eggsec-report-model/src/envelope.rs`); `eggsec_output::envelope` re-exports it for backward compatibility, and domain crates (`eggsec-db-lab`, `eggsec-mobile-lab`, `eggsec-web-proxy`, `eggsec-nse`) depend on the model directly (guards Checks 118–120).

## Key Types

- `ReportEnvelope` - Top-level container with report_id, operation_id, domain_id, target, generated_at, findings, evidence_manifest, policy_summary, baseline, tool_metadata
- `FindingRecord` - Normalized finding with id, domain, operation_id, severity, title, description, evidence items, remediation, references, category, location
- `EvidenceItem` - Single evidence entry with id, kind, source, summary, data_ref, redaction state
- `EvidenceManifest` - Manifest tracking all evidence items with total/redacted counts and redaction policy
- `BaselineSummary` - Standardized baseline comparison with added/resolved/unchanged counts and severity deltas
- `ToolMetadata` - Tool name and version information

## Conversion Pattern

Domain crates maintain their domain-specific types and provide `to_report_envelope()` functions:

1. mobile-static: `MobileScanReport` → `ReportEnvelope` (lib.rs:327)
2. db-pentest: `DbPentestReport` → `ReportEnvelope` (bridge.rs:76)

Existing `to_scan_report_data()` bridges are preserved for backward compatibility.

## Evidence Redaction

`RedactionState` classifies individual evidence item sensitivity:
- `None` - No sensitive data, full content safe
- `FullyRedacted` - Only placeholder included
- `PartiallyRedacted` - Sensitive fields masked
- `Summarized` - Original content replaced with summary

`RedactionPolicy` declares the manifest-level redaction strategy:
- `None` - No redaction applied
- `RedactAll` - Redact all items regardless of individual state
- `RedactSensitive` - Redact only items marked as sensitive
- `SummarizeAll` - Replace raw content with summaries
- `DomainSpecific` - Domain-specific logic; individual item states take precedence

## Files

- `crates/eggsec-report-model/src/envelope.rs` - Core types (canonical owner)
- `crates/eggsec-report-model/src/report.rs` - `ScanReportData` family (canonical owner)
- `crates/eggsec-report-model/src/summary.rs` - `PolicySummary`, `DiffSummary` (canonical owner)
- `crates/eggsec-report-model/tests/roundtrip.rs` - Contract round-trip tests (independent of output)
- `crates/eggsec-output/src/envelope.rs` - Compatibility facade (re-export + `From<&AgentFinding>`)
- `crates/eggsec-output/tests/report_envelope.rs` - Integration tests (via re-export)
- `docs/REPORT_EVIDENCE_MODEL.md` - Full inventory and design doc

*Last verified against source: 2026-09-16*
