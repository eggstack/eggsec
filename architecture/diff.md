# Diff Module

## Purpose

Finding comparison engine that compares two scan result sets and identifies new, resolved, persisting, and changed findings using fingerprint-based matching.

## Key Types

| Type | Location | Description |
|------|----------|-------------|
| `DiffResult` | `diff/mod.rs` | Result of comparing two finding sets |
| `FindingChange` | `diff/mod.rs` | Description of a finding that changed between scans |
| `DiffSummary` | `diff/mod.rs` | Numeric summary of diff results |

## Files

| File | Description |
|------|-------------|
| `mod.rs` | Module root: `diff_findings()`, `load_findings_from_file()`, `DiffResult`, `DiffSummary`, fingerprint comparison logic |

## Implementation Status

Fully implemented. `diff_findings()` compares `&[Finding]` slices by fingerprint and produces a structured `DiffResult`. `load_findings_from_file()` loads findings from a single JSONL file; load both files and pass the slices to `diff_findings()`.
