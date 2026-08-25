# Output & Reporting Module

## Purpose

The Output module handles formatting, deduplication, trend analysis, baseline comparison, session persistence, and export of security findings into standardized formats. It is split across two crate boundaries: `eggsec-output` (19 source files, dependency-light, no engine/runtime deps) and the engine crate `eggsec`'s `output/` (8 source files, depends on engine-internal types). Guard-enforced invariant: `eggsec-output` depends only on `eggsec-core` — no engine or runtime dependencies.

## Role & Responsibilities

- **Format conversion**: JSON canonical, CSV, HTML, SARIF (SarifBuilder), JUnit (JUnitBuilder), Markdown, PDF (engine-only, `pdf` feature)
- **Envelope wrapping**: `ReportEnvelope` provides protocol-neutral report container with evidence manifests and redaction policy
- **Deduplication**: `DedupEngine` with strict/fuzzy/disabled strategies
- **Trend analysis**: LRU-cached historical comparison with sliding-window delta computation
- **Baseline comparison**: Finding-level new/resolved/unchanged classification
- **Diff summary**: Numeric envelope (`DiffSummary`) for pipeline run manifests
- **Session persistence**: TUI session save/load via JSON
- **Scheduling**: Cron-based scan scheduling with priority queue
- **AI output schema**: Typed AI-consumable finding output with risk score
- **Policy/audit summaries**: Aggregated enforcement decision statistics

## Location & Feature Gating

### `crates/eggsec-output/src/` (19 files)

Always compiled. Dependencies: `eggsec-core`, `serde`, `serde_json`, `chrono`, `rustc-hash`, `quick-xml`, `unicode-normalization`, `lru`, `uuid`, `hostname`, `tokio`.

| File | Lines | Tests | Purpose |
|------|-------|-------|---------|
| `lib.rs` | 71 | 0 | Module root and re-exports |
| `agent.rs` | 481 | 2 | `AgentFinding`, `Confidence` (4 variants), `Evidence`, `Remediation`, `FindingSummary`, `AttackSurface` (10 variants), `FindingStatus` (5 variants: New/Confirmed/FalsePositive/Ignored/Remediated) |
| `ai_schema.rs` | 237 | 9 | `AiOutput`, `AiFinding`, `AiEvidence`, `AiRemediation`, `AiSummary` — typed AI consumption output |
| `audit_summary.rs` | 82 | 2 | `AuditSummary` — aggregated enforcement decision counts from JSON audit events |
| `baseline.rs` | 192 | 10 | `BaselineComparison` — finding-level new/resolved/unchanged classification by `id` matching |
| `convert.rs` | 367 | 3 | `ScanReportData`, `FindingData`, `PortData`, `ServiceData`, `WirelessNetworkReportData`; conversion functions `load_scan_report`, `convert_to_*` |
| `csv.rs` | 163 | 0 | `CsvExporter` — finding/port/endpoint CSV export; streaming async variant |
| `dedup.rs` | 163 | 6 | `DedupEngine`, `DedupStrategy` (Strict/Fuzzy/Disabled) |
| `diff.rs` | 27 | 1 | `DiffSummary` — numeric diff envelope for `RunManifest` |
| `envelope.rs` | 770 | 11 | `ReportEnvelope`, `FindingRecord`, `EvidenceItem`, `EvidenceManifest`, `EvidenceKind` (20 variants), `BaselineSummary`, `RedactionState`, `RedactionPolicy` |
| `escape.rs` | 81 | 4 | `escape_html()`, `escape_csv()` (NFKC + formula injection protection), `escape_xml()` |
| `html.rs` | 325 | 0 | `HtmlReport` — styled HTML with dark/light themes, Chart.js doughnut |
| `junit.rs` | 407 | 2 | `JUnitBuilder`, `JUnitReport` — JUnit XML via `quick_xml::Writer` (write-only, XXE-safe) |
| `markdown.rs` | 141 | 0 | `MarkdownReport` — markdown-formatted report generation |
| `policy_summary.rs` | 54 | 2 | `PolicySummary` — policy decision metadata for report envelopes |
| `sarif.rs` | 276 | 1 | `SarifBuilder`, `SarifReport` — SARIF 2.1.0 JSON via `serde_json` (no XML parsing, XXE-safe) |
| `schedule.rs` | 516 | 8 | `CronScheduler`, `CronExpression`, `ScanQueue`, `RateLimiter`, `ScheduledScan` |
| `session.rs` | 102 | 0 | `ScanSession`, `TabSessionState`, `InputFieldState`, `SessionInfo` — TUI session persistence |
| `trend.rs` | 539 | 15 | `TrendAnalyzer`, `ResultComparator`, `TrendAnalysis`, `TrendDirection`, `ComparisonResult`, `ScanResult` |

**Test-bearing files**: 14 of 19 (agent, ai_schema, audit_summary, baseline, convert, dedup, diff, envelope, escape, junit, policy_summary, sarif, schedule, trend).

### `crates/eggsec/src/output/` (8 files, engine crate)

Depend on engine-internal types (`PipelineReport`, `PolicyDecision`, `ExecutionBudget`). Feature-gated where noted.

| File | Lines | Tests | Feature Gate | Purpose |
|------|-------|-------|-------------|---------|
| `mod.rs` | 178 | 0 | — | Re-exports `eggsec_output::*` + local modules; `SarifBuilderExt`/`JUnitBuilderExt` extension traits |
| `attack_graph.rs` | 211 | 3 | `advanced-hunting` | `AttackGraph`, `AttackGraphBuilder`, `GraphNode`, `GraphEdge`, `GraphCluster`; `from_chains()` requires `AttackChain` from `hunt::chain`; `to_html()` is NOT feature-gated |
| `lab_report.rs` | 176 | 2 | — | `LabDefenseReportSection`, `ScopeSummary`, `BudgetSummary`, `TargetResolutionSummary`, `SkippedOperation` |
| `pdf.rs` | 239 | 3 | `pdf` | `PdfGenerator`, `PdfConfig` — single-page PDF via `printpdf`; truncates to 30 findings; `#[cfg(not(feature = "pdf"))]` stub returns error |
| `report.rs` | 77 | 0 | — | `Report` trait, `ReportTemplate` (4 variants: Executive/Technical/Developer/Compliance), `ReportMetadata`, `SeverityCounts` |
| `report_summary.rs` | 286 | 10 | — | `ReportSummary`, `AssetCount` — aggregated statistics from canonical `Finding` with risk narrative generation |
| `run_manifest.rs` | 269 | 4 | — | `RunManifest` — run-level metadata envelope for regression workflows, carries `DiffSummary` |

**Test-bearing files**: 4 of 8 (attack_graph, lab_report, pdf, report_summary, run_manifest = 5).

**Combined test-bearing files**: 19 of 27 total source files.

## Architecture

### Format Writer Summary

| Format | Writer Type | Entry Function | Output | Notable Options |
|--------|------------|----------------|--------|-----------------|
| JSON | `serde_json` | `convert_to_json()` (`convert.rs:236`) | Pretty-printed JSON string | `ScanReportData` → JSON |
| CSV | `CsvExporter` | `export_findings()` (`csv.rs:25`) | String | Streaming async variant; NFKC-normalized escaping |
| HTML | `HtmlReport` | `generate()` (`html.rs:44`) | HTML string | Dark/light themes; Chart.js doughnut; `escape_html()` on all user content |
| Markdown | `MarkdownReport` | `generate()` (`markdown.rs:61`) | `Result<String>` | Pipe-character escaping in wireless tables via closure |
| SARIF | `SarifBuilder` | `build()` → `to_json()` (`sarif.rs:214`) | SARIF 2.1.0 JSON | `serde_json` (no XML); invocations with timestamps |
| JUnit | `JUnitBuilder` | `build()` → `to_xml()` (`junit.rs:185`) | JUnit XML | `quick_xml::Writer` write-only (XXE-safe) |
| PDF | `PdfGenerator` | `generate_report()` (`pdf.rs:26`) | `Result<Vec<u8>>` | Feature-gated `pdf`; `printpdf`; max 30 findings per page |
| AI | `AiOutput` | `from_findings()` (`ai_schema.rs:11`) | `AiOutput` | Risk score 0–10; executive summary string |

### Conversion Pipeline

`convert.rs` provides the bridge from `ScanReportData` to all formats:

```
ScanReportData → convert_to_json()   → JSON string
              → convert_to_csv()    → CSV string
              → convert_to_html()   → HTML string (via markdown::ScanSummary)
              → convert_to_markdown() → Markdown string
              → convert_to_junit()  → JUnit XML string
              → convert_to_sarif()  → SARIF JSON string
```

`load_scan_report()` (`convert.rs:70`) loads a JSON file into `ScanReportData`.

### Envelope Wrapping Pipeline

`ReportEnvelope` (`envelope.rs:436`) is the top-level normalized container:

1. Domain crates produce `ReportEnvelope` from their domain-specific types
2. `FindingRecord` (11 fields) holds normalized finding data
3. `EvidenceItem` (7 fields) with `EvidenceKind` (20 variants), `EvidenceSource`, `RedactionState`
4. `EvidenceManifest` aggregates all evidence items with `RedactionPolicy` (5 variants)
5. Optional `PolicySummary`, `BaselineSummary`, `ToolMetadata`
6. `refresh_evidence_manifest()` rebuilds the manifest from findings
7. `to_json()` / `from_json()` for serialization

### Dedup Engine

`DedupEngine` (`dedup.rs:24`) uses `FxHashSet<String>` for seen-key tracking:

| Strategy | Key | Behavior |
|----------|-----|----------|
| `Strict` | `"{severity}:{title}:{target}"` | Deduplicates on all three fields |
| `Fuzzy` | `"{severity}:{title}"` | Ignores target — same title+severity across hosts collapses |
| `Disabled` | — | Returns all findings unchanged |

### Baseline Comparison

`BaselineComparison::compare()` (`baseline.rs:12`) classifies findings by `AgentFinding.id`:

- **New**: IDs in current but not in baseline
- **Resolved**: IDs in baseline but not in current
- **Unchanged**: IDs present in both

No fingerprint-based matching or severity escalation/de-escalation tracking.

### Trend Computation

`TrendAnalyzer` (`trend.rs:147`) stores up to 1000 `ScanResult` entries in an `lru::LruCache` keyed by result ID. `get_trend()` (`trend.rs:165`):

1. Sorts results by timestamp
2. Computes sliding-window deltas for critical/high/medium finding counts across consecutive scans via `.windows(2)`
3. Direction determined by critical trend only: any increase → `Worsening`, any decrease → `Improving`, otherwise `Stable`
4. `average_scan_time_ms` computed from all cached results

`ResultComparator::compare()` (`trend.rs:68`) uses composite key `(title, category, cve)` for finding-level comparison between two `ScanResult`s.

### Diff Summary

`DiffSummary` (`diff.rs:4`) — 5 fields: `total_new`, `total_resolved`, `total_escalated`, `total_deescalated`, `net_change`. Used in `RunManifest` (`run_manifest.rs:56`) via `with_baseline()` (`run_manifest.rs:93`). This is a numeric metadata envelope, not a comparison engine. The actual comparison logic lives in `BaselineComparison` above.

## Data Model

### `ScanReportData` (`convert.rs:9`)

| Field | Type |
|-------|------|
| `target` | `String` |
| `scan_type` | `String` |
| `timestamp` | `String` |
| `findings` | `Vec<FindingData>` |
| `open_ports` | `Vec<PortData>` |
| `services` | `Vec<ServiceData>` |
| `duration_ms` | `u64` |
| `wireless_networks` | `Vec<WirelessNetworkReportData>` |
| `policy_summary` | `Option<PolicySummary>` |

### `ReportEnvelope` (`envelope.rs:436`)

| Field | Type |
|-------|------|
| `report_id` | `String` |
| `operation_id` | `String` |
| `domain_id` | `Option<String>` |
| `target` | `Option<String>` |
| `generated_at` | `DateTime<Utc>` |
| `findings` | `Vec<FindingRecord>` |
| `evidence_manifest` | `EvidenceManifest` |
| `policy_summary` | `Option<PolicySummary>` |
| `baseline` | `Option<BaselineSummary>` |
| `tool_metadata` | `Option<ToolMetadata>` |

### `EvidenceKind` (Envelope) — 20 variants (`envelope.rs:32`)

`HttpRequest`, `HttpResponse`, `Header`, `BodySnippet`, `Timing`, `Diff`, `Banner`, `DnsRecord`, `Certificate`, `PortState`, `Screenshot`, `FileMetadata`, `LogLine`, `DatabaseFinding`, `MobileManifest`, `TrafficCapture`, `StaticAnalysis`, `RuntimeInstrumentation`, `Correlation`, `Generic`

**Note**: This is a *separate* enum from `EvidenceKind` in `findings/mod.rs` (13 variants). They serve different abstraction levels: the findings module defines the canonical evidence schema, while the envelope defines a broader protocol-neutral evidence taxonomy for cross-domain reports.

### `FindingStatus` Divergence

There are **two separate `FindingStatus` enums** in the output crate:

| Module | Variants | Location |
|--------|----------|----------|
| `agent.rs` | `New`, `Confirmed`, `FalsePositive`, `Ignored`, `Remediated` (5) | `agent.rs:93-101` |
| `findings/lifecycle.rs` | `New`, `Confirmed`, `AcceptedRisk`, `FalsePositive`, `Remediated`, `Reopened` (6) | `lifecycle.rs:6-13` |

The `agent.rs` version lacks `AcceptedRisk` and `Reopened` but adds `Ignored`. This is a known divergence — the findings module defines the target canonical schema.

### `Confidence` Divergence

There are **three separate `Confidence` enums** in the codebase:

| Module | Variants | Score Mapping |
|--------|----------|---------------|
| `findings/mod.rs:37` | `Confirmed`, `High`, `Medium`, `Low`, `Informational` (5) | 1.0, 0.75, 0.5, 0.25, 0.0 |
| `output/agent.rs:8` | `Confirmed`, `Likely`, `Possible`, `Unlikely` (4) | 1.0, 0.75, 0.5, 0.25 |
| `recon/secrets.rs` | `High`, `Medium`, `Low` (3) | Similar |

The `findings` module includes an `Informational` variant (score 0.0) that the other modules lack. Naming diverges (`High`/`Medium`/`Low` vs `Likely`/`Possible`/`Unlikely`).

### `SeverityCounts` (`report.rs:56`)

| Field | Type |
|-------|------|
| `critical` | `usize` |
| `high` | `usize` |
| `medium` | `usize` |
| `low` | `usize` |
| `info` | `usize` |

Method `risk_score()` returns weighted sum capped at 100.0.

## Integration Points

### Producers of ScanReportData

- **Dispatch workers**: Route `TaskResult`s through the output pipeline
- **CLI handlers**: `handle_report` (`commands/handlers/report.rs`) loads/saves `ScanReportData`
- **Wireless/mobile/db-pentest bridges**: Optional `to_scan_report_data()` methods produce `ScanReportData` from domain-specific types
- **TUI report tab**: Consumes `ScanReportData` for display

### Producers of AgentFinding

- **convert.rs**: `From<&FindingData> for AgentFinding` (`convert.rs:282`)
- **AgentFinding::from_scan_result()** (`agent.rs:185`): Parses JSON scan results into `Vec<AgentFinding>`
- **Various module handlers**: Construct `AgentFinding` via builder pattern

### ReportEnvelope Consumers

- **Domain crates** (db-pentest, mobile, wireless): Convert domain-specific report types to `ReportEnvelope`
- **Pipeline report**: `PipelineReport` carries an optional `RunManifest` that can include a `DiffSummary`

## Testing

### Test Counts by File (output crate)

| File | Tests | Key Test Coverage |
|------|-------|-------------------|
| `agent.rs` | 2 | Finding creation, summary aggregation |
| `ai_schema.rs` | 9 | Empty/critical/high/mixed findings, risk score cap, serialization roundtrip |
| `audit_summary.rs` | 2 | Empty summary, multi-event aggregation |
| `baseline.rs` | 10 | No changes, new, resolved, mixed, empty both sides, count helpers |
| `convert.rs` | 3 | Mixed-case severity parsing for JUnit/SARIF, summary counts |
| `dedup.rs` | 6 | FromStr parsing, default strategy, disabled/strict/fuzzy dedup, empty input |
| `diff.rs` | 1 | Struct construction |
| `envelope.rs` | 11 | EvidenceItem/FindingRecord/EvidenceManifest creation, redaction, serialization roundtrip, refresh manifest |
| `escape.rs` | 4 | Fullwidth bypass detection, tab/CR quoting |
| `junit.rs` | 2 | Builder construction, XML output validation |
| `policy_summary.rs` | 2 | Default values, serialization |
| `sarif.rs` | 1 | Builder construction, version/schema validation |
| `schedule.rs` | 8 | Cron parsing, seconds, wildcard, step, next_run, queue complete/fail |
| `trend.rs` | 15 | Comparator added/removed/no-change/same-title-different-category, risk trend, analyzer single/worsening/improving, average time, category counts, most common, default |

**Total output crate tests**: 66 (verified by grep).

### Test Counts by File (engine output + findings)

| File | Tests |
|------|-------|
| `attack_graph.rs` | 3 |
| `lab_report.rs` | 2 |
| `pdf.rs` | 3 |
| `report_summary.rs` | 10 |
| `run_manifest.rs` | 4 |
| `findings/mod.rs` | 13 |
| `findings/store.rs` | 4 |
| `findings/lifecycle.rs` | 7 |

**Total engine output + findings tests**: 46.

**Grand total**: 112 tests across output and findings modules.

## Invariants & Gotchas

1. **`eggsec-output` must not depend on `eggsec` or `eggsec-runtime`** — enforced by `scripts/check-architecture-guards.sh`
2. **PDF lives in the engine crate**, not in `eggsec-output` — behind `pdf` feature gate
3. **Two separate `EvidenceKind` enums** — `findings/mod.rs` (13 variants) vs `envelope.rs` (20 variants); different abstraction levels
4. **Two separate `FindingStatus` enums** — `agent.rs` (5 variants) vs `findings/lifecycle.rs` (6 variants); not yet unified
5. **Three separate `Confidence` enums** — known divergence documented in `findings.md`
6. **Baseline comparison uses `AgentFinding.id`**, not fingerprint — fingerprint-based diff is not implemented
7. **`DedupEngine` uses unbounded `FxHashSet`** — no capacity limit; could grow without bound across long sessions
8. **`ScanSession` is NOT atomic** — writes the full JSON file; partial write on crash leaves corrupted state
9. **`TrendAnalyzer` LRU cache** — max 1000 entries; eviction is LRU, not time-based
10. **SARIF is JSON-based** (RFC 8259), not XML — XXE does not apply
11. **JUnit uses `quick_xml::Writer`** in write-only mode — no entity expansion, XXE-safe
12. **HTML escaping** applied consistently via `escape_html()` on all user content in `html.rs`
13. **CSV escaping** uses NFKC normalization + formula injection protection (leading `=`, `+`, `-`, `@`, `\t`, `\r`)
14. **Markdown** does NOT escape pipe characters in finding fields — tables could break with `|` in content
15. **PDF truncates to 30 findings** per page with a warning; no multi-page support
16. **`ScanQueue` max size** defaults to 100; priority-based insertion
17. **`CronScheduler::next_run()`** does linear scan up to 7 days ahead — O(7*86400) worst case

## Security Notes

### XXE Safety

- **SARIF**: Uses `serde_json` (JSON format), no XML parsing — immune to XXE
- **JUnit**: Uses `quick_xml::Writer` in write-only mode without entity expansion — immune to XXE

### CSV Formula Injection Protection

`escape_csv()` (`escape.rs:16`) uses NFKC normalization and quoting to prevent formula injection. Detects leading formula characters (`=`, `+`, `-`, `@`, `\t`, `\r`) and wraps in double-quoted field with internal double-quote escaping.

### HTML Injection Prevention

`escape_html()` (`escape.rs:1`) encodes `&`, `<`, `>`, `"`, `'` — all user content passed through before HTML template insertion.

## Performance Notes

**Hash Collections**: Use `rustc_hash::FxHashMap`/`FxHashSet` instead of `std::collections::HashMap` in:
- `trend.rs` — `ResultComparator`, `TrendAnalyzer`
- `agent.rs` — `FindingSummary`
- `dedup.rs` — `DedupEngine::seen`
- `baseline.rs` — `BaselineComparison::compare()`
- `session.rs` — `ScanSession::tab_states`, `ScanSession::results`, `TabSessionState::options`
- `sarif.rs` — `SarifResult::properties`
- `junit.rs` — `JUnitBuilder::test_suites`
- `report_summary.rs` — `ReportSummary::from_findings()` (all maps)
- `envelope.rs` — Notable: `BaselineSummary::severity_deltas` uses `std::collections::HashMap`, not FxHashMap (minor inconsistency)

## Bug Sweep (Report Only)

| Location | Issue | Severity |
|----------|-------|----------|
| `dedup.rs:26` | `DedupEngine::seen: FxHashSet<String>` has **no capacity bound**. Long-running sessions with many findings could grow this set without limit. | Low |
| `markdown.rs:196` | `escape_pipe` closure only escapes `\|` but markdown tables can also break on newlines in content. Not a security issue but could produce malformed output. | Informational |
| `session.rs:38-44` | `ScanSession::save()` writes the full JSON file non-atomically. A crash during write leaves corrupted state. No temp-file-and-rename pattern. | Low |
| `pdf.rs:142-205` | `generate_html()` is `#[cfg(test)]`-only test helper — its HTML output does NOT escape finding content. Safe because it's test-only, but if accidentally used in production would be an injection vector. | Informational |
| `envelope.rs:377` | `BaselineSummary::severity_deltas` uses `std::collections::HashMap` while the rest of the output crate uses `FxHashMap`. Minor performance inconsistency. | Informational |

---

*Last verified against source: 2026-08-25*
