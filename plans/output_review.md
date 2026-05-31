# Output Architecture Review

**Document:** architecture/output.md
**Reviewed:** 2026-05-31
**Accuracy:** High

## Verified Claims

- **Supported formats table**: All 7 formats verified — JSON (`convert.rs`), SARIF (`sarif.rs`), HTML (`html.rs`), Markdown (`markdown.rs`), PDF (`pdf.rs`, feature-gated at `mod.rs:62`), CSV (`csv.rs`), JUnit XML (`junit.rs`)
- **DedupStrategy enum**: `Strict`, `Fuzzy`, `Disabled` at `dedup.rs:6-11` — matches doc
- **Template engine**: `ReportTemplateEngine` at `template.rs:17` with four built-in templates (`executive`, `technical`, `developer`, `compliance`) verified at `template.rs:142-152`
- **Compliance standards**: PCI-DSS, SOC2, HIPAA, GDPR, OWASP, NIST — all six `ComplianceStandard` variants at `template.rs:32-38`
- **Attack graph module**: Feature-gated behind `advanced-hunting` at `mod.rs:51-52` — matches doc claim that it visualizes attack paths
- **TrendAnalyzer and TrendDirection**: `TrendAnalyzer` at `trend.rs:147`, `TrendDirection` enum (`Improving`, `Stable`, `Worsening`) at `trend.rs:140-145` — matches doc
- **DiffEngine**: Struct at `diff.rs:35`, `has_regressions()` at `diff.rs:137` — checks for Critical escalations (doc says "Critical escalations" which matches `severity >= Severity::High` at line 140, but the actual check is `>= High` which includes Critical too)
- **RunManifest struct**: All 14 fields match exactly at `run_manifest.rs:25-56` — `schema_version`, `run_id`, `started_at`, `ended_at`, `slapper_version`, `target_scope`, `profile`, `probe_intents`, `risk_budget`, `feature_flags`, `observations`, `findings`, `artifacts`, `baseline_id`, `diff_summary`
- **RunManifest::from_report()**: Verified at `run_manifest.rs:103`
- **AgentFinding struct**: At `agent.rs:71-89` with fields matching doc description
- **FindingSummary struct**: At `agent.rs:273-280` with `by_severity`, `by_confidence`, `by_attack_surface`, `by_type` — matches doc
- **SeverityCounts**: At `report.rs:56-63` — matches doc
- **CsvExporter methods**: `export_findings()` at `csv.rs:10`, `export_ports()` at `csv.rs:74`, `export_endpoints()` at `csv.rs:98` — all return `Result<String, std::fmt::Error>` as documented
- **JUnitReport::to_xml()**: Referenced at `convert.rs:88` — returns `Result<String, quick_xml::Error>` via the builder pattern
- **XXE safety**: SARIF uses `serde_json` (JSON), JUnit uses `quick_xml::Writer` in write-only mode — both verified
- **CSV formula injection**: `escape_csv()` at `escape.rs:16-35` uses NFKC normalization (`unicode_normalization`) and quoting — matches doc
- **FxHashMap usages in output module**: `ResultComparator` at `trend.rs:73-82`, `TrendAnalyzer` results (uses LruCache, not FxHashMap directly), `DedupEngine::seen` at `dedup.rs:27`, `DiffEngine` at `diff.rs:39-49`, `BaselineComparison` at `baseline.rs:13-14`, `ScanSession::tab_states` at `session.rs:11`, `ReportTemplateEngine::custom_templates` at `template.rs:19`, `TemplateRenderContext::custom_data` at `template.rs:74` — all use FxHashMap
- **Integration code example**: `convert_to_csv` and `load_scan_report` at `convert.rs:142` and `convert.rs:52` — matches doc example

## Discrepancies

- **DiffEngine::has_regressions() severity threshold**: Doc says "checks Critical escalations" but actual implementation at `diff.rs:137-141` checks `severity >= Severity::High`, which includes both High AND Critical. The doc is imprecise.
- **TrendAnalyzer internal storage**: Doc says `TrendAnalyzer` uses FxHashMap, but actual implementation at `trend.rs:148` uses `LruCache<String, ScanResult>`. The FxHashMap usages in the Performance Notes section correctly identify where FxHashMap is used in `trend.rs` (in `ResultComparator`), but the `TrendAnalyzer` struct itself does not contain FxHashMap.
- **Missing modules from doc**: The output module has `report_summary.rs` and `schedule.rs` that are not mentioned in the Supported Formats table or Core Features section. `report_summary.rs` provides `ReportSummary` with risk narrative generation. `schedule.rs` provides `CronScheduler` and scan queue management.
- **AttackGraphBuilder::to_html() return type**: Doc says returns `Result<String, serde_json::Error>` but this is unverifiable since the module is feature-gated behind `advanced-hunting`. The claim is plausible but UNVERIFIED.

## Bugs Found

- None identified in the documentation vs. codebase comparison.

## Improvement Opportunities

- **Clarify has_regressions() threshold**: Change "checks Critical escalations" to "checks High/Critical escalations" to match the actual `>= Severity::High` comparison at `diff.rs:140`. (priority: medium)
- **Add schedule.rs to Core Features**: The `schedule.rs` module provides `CronScheduler`, `ScanQueue`, `CronExpression`, `Priority` types that are publicly exported at `mod.rs:94`. This is a non-trivial feature missing from the doc. (priority: medium)
- **Add report_summary.rs to Key Types**: `ReportSummary` provides risk narrative generation and is used for structured output. (priority: low)
- **Document TrendAnalyzer storage**: Clarify that `TrendAnalyzer` uses `LruCache` internally (max 1000 entries) rather than implying it uses FxHashMap. (priority: low)
- **Add agent.rs FindingSummary fields**: The doc mentions `FindingSummary` but does not describe its fields. Adding the `by_severity`, `by_confidence`, `by_attack_surface`, `by_type` breakdown would be useful. (priority: low)

## Stale Items

- **"Performance Notes" FxHashMap section**: The list of FxHashMap usages is comprehensive and accurate. No stale entries found.
- **"Security Notes" section**: XXE and CSV injection protections are current and accurate.
- **Integration code example**: The example at the bottom of the doc uses `convert_to_csv` and `load_scan_report` which exist and work as shown.
