# Output & Reporting Architecture Review

**Document:** architecture/output.md
**Reviewed:** 2026-05-31
**Accuracy:** High
**Lines Reviewed:** 262

## Verified Claims

- **Supported Formats table (lines 9-17)**: All 7 format files exist: `convert.rs`, `sarif.rs`, `html.rs`, `markdown.rs`, `pdf.rs`, `csv.rs`, `junit.rs` — Verified at `crates/slapper/src/output/` directory listing
- **DedupStrategy enum (lines 26-30)**: `Strict`, `Fuzzy`, `Disabled` variants match exactly — Verified at `crates/slapper/src/output/dedup.rs:5-11`
- **Strict dedup key format `severity:title:target` (line 27)**: Confirmed in `dedup_strict()` at `dedup.rs:50` — `format!("{}:{}:{}", f.severity, f.title, f.target)`
- **Fuzzy dedup key format `severity:title` (line 28)**: Confirmed in `dedup_fuzzy()` at `dedup.rs:61` — `format!("{}:{}", f.severity, f.title)`
- **Template names: executive, technical, developer, compliance (lines 37-39)**: All 4 built-in templates registered at `template.rs:142-152`
- **Compliance standards: PCI-DSS, SOC2, HIPAA, GDPR, OWASP, NIST (line 39)**: All 6 standards in `ComplianceStandard` enum at `template.rs:33-39`
- **AttackGraph struct fields (lines 46-51)**: `nodes`, `edges`, `clusters` match `attack_graph.rs:7-12`
- **AttackGraphBuilder::from_chains() is feature-gated (line 53)**: Correct — `attack_graph` module is `#[cfg(feature = "advanced-hunting")]` at `mod.rs:51-52`
- **AttackGraphBuilder::to_html() is NOT feature-gated (line 53)**: Partially correct — `to_html()` method itself has no cfg annotation (`attack_graph.rs:135`), but the entire module is behind `advanced-hunting` feature gate (`mod.rs:51`), so it IS inaccessible without the feature
- **TrendAnalyzer LRU cache capacity 1000 (line 61)**: Confirmed — `DEFAULT_MAX_HISTORY: usize = 1000` at `trend.rs:5`, `LruCache::new(NonZeroUsize::new(DEFAULT_MAX_HISTORY).unwrap())` at `trend.rs:154`
- **TrendDirection enum (line 63)**: `Improving`, `Stable`, `Worsening` match `trend.rs:141-145`
- **ResultComparator composite key `(title, category, cve)` (line 66)**: Confirmed at `trend.rs:60-66`
- **DiffFinding struct fields (lines 83-90)**: All 6 fields match `diff.rs:17-24`
- **DiffResult struct fields (lines 92-99)**: All 6 fields match `diff.rs:7-14`
- **DiffEngine struct and has_regressions signature (lines 101-102)**: `DiffEngine` struct at `diff.rs:35`, `has_regressions` at `diff.rs:137` — signature matches
- **has_regressions checks >= Severity::High (line 102)**: Confirmed — `f.severity >= Severity::High` at `diff.rs:140`
- **ReportSummary struct fields (lines 112-120)**: All 7 fields match `report_summary.rs:8-16`
- **AssetCount struct (lines 122-125)**: `asset` and `count` fields match `report_summary.rs:20-23`
- **from_findings() aggregates top 10 assets (line 128)**: Confirmed — `top_affected_assets.truncate(10)` at `report_summary.rs:60`
- **Risk narrative produces CRITICAL/HIGH/MEDIUM/LOW prefixes (line 130)**: Confirmed at `report_summary.rs:96-119`
- **"No findings detected." for empty findings (line 130)**: Confirmed at `report_summary.rs:121-122`
- **CronScheduler struct (lines 137-139)**: `expressions: Vec<CronExpression>` matches `schedule.rs:201-203`
- **CronExpression struct fields (lines 141-149)**: All public fields match `schedule.rs:206-219`
- **ScanQueue struct (lines 151-155)**: `queue`, `max_size`, `running` match `schedule.rs:66-70`
- **RateLimiter struct (lines 157-162)**: All 4 fields match `schedule.rs:149-154`
- **CronScheduler parses 5- or 6-field expressions (line 165)**: Confirmed — `schedule.rs:224-225`: `if parts.len() != 5 && parts.len() != 6`
- **ScanQueue max size 100 by default (line 167)**: Confirmed — `Default for ScanQueue` at `schedule.rs:196-198`: `Self::new(100)`
- **RateLimiter burst_size = 2x rate (line 169)**: Confirmed — `burst_size: requests_per_second * 2` at `schedule.rs:160`
- **RunManifest struct fields (lines 176-192)**: All 14 fields match `run_manifest.rs:25-56`
- **CsvExporter::export_findings() returns Result<String, std::fmt::Error> (line 220)**: Confirmed at `csv.rs:10`
- **CsvExporter::export_ports() returns Result<String, std::fmt::Error> (line 220)**: Confirmed at `csv.rs:74`
- **CsvExporter::export_endpoints() returns Result<String, std::fmt::Error> (line 220)**: Confirmed at `csv.rs:98`
- **MarkdownReport::generate() returns Result<String, std::fmt::Error> (line 221)**: Confirmed at `markdown.rs:60`
- **JUnitReport::to_xml() returns Result<String, quick_xml::Error> (line 222)**: Confirmed at `junit.rs:316`
- **AttackGraphBuilder::to_html() returns Result<String, serde_json::Error> (line 223)**: Confirmed at `attack_graph.rs:135`
- **XXE safety claims (lines 245-247)**: SARIF uses `serde_json` (confirmed at `sarif.rs:7-8` doc comment and usage), JUnit uses `quick_xml::Writer` in write-only mode (confirmed at `junit.rs:3-9` doc comment)
- **CSV formula injection protection via escape_csv() (line 251)**: Confirmed — `escape_csv()` at `escape.rs:16-35` uses NFKC normalization (`s.nfkc()`) and quotes fields containing formula chars
- **FxHashMap usage in trend.rs (line 230)**: Confirmed at `trend.rs:2`
- **FxHashMap usage in agent.rs (line 231)**: Confirmed at `agent.rs:276-287,322`
- **FxHashMap usage in dedup.rs (line 232)**: Confirmed at `dedup.rs:2`
- **FxHashMap usage in diff.rs (line 233)**: Confirmed at `diff.rs:3`
- **FxHashMap usage in session.rs (line 235)**: Confirmed at `session.rs:1`
- **FxHashMap usage in template.rs (line 236)**: Confirmed at `template.rs:7`
- **FxHashMap usage in attack_graph.rs (line 237)**: Confirmed at `attack_graph.rs:4`
- **FxHashMap usage in sarif.rs (line 238)**: Confirmed at `sarif.rs:11`
- **FxHashMap usage in junit.rs (line 239)**: Confirmed at `junit.rs:14`
- **Key Types table (lines 203-215)**: All types verified in their respective files

## Discrepancies

- **Output format count**: The architecture doc lists 7 formats in the table (lines 9-17). AGENTS.md claims 8 formats ("Pretty, Json, Compact, Html, Csv, Sarif, Junit, Markdown") by splitting JSON into Pretty/Json/Compact. The architecture doc is more accurate — JSON is one format with two rendering modes. (Minor: AGENTS.md inconsistency)
- **AttackGraphBuilder::to_html() accessibility (line 53)**: The doc claims `to_html()` "can be used with manually constructed graphs without enabling the `advanced-hunting` feature." This is incorrect — the entire `attack_graph` module is behind `#[cfg(feature = "advanced-hunting")]` at `output/mod.rs:51`, making `to_html()` inaccessible without the feature. (`output/mod.rs:51-52`)
- **FxHashMap usage in baseline.rs (line 234)**: The doc lists `baseline.rs - BaselineComparison compare function` as using `FxHashMap`. Actual code uses `FxHashSet` (not `FxHashMap`) at `baseline.rs:2`. (`crates/slapper/src/output/baseline.rs:2`)
- **ReportSummary::from_findings() uses std HashMap (line 112-120)**: The code block shows `HashMap<String, usize>` for `by_severity`, `by_confidence`, `by_type` fields. Actual code at `report_summary.rs:4,10-12` uses `std::collections::HashMap`, NOT `FxHashMap`. The Performance Notes section (lines 229-239) does NOT list `report_summary.rs` as a FxHashMap user, which is correct — but the doc should note this inconsistency for future improvement. (`crates/slapper/src/output/report_summary.rs:4,10-12`)

## Bugs Found

- **None identified**: No bugs found in the codebase claims. All error handling patterns documented are correctly implemented.

## Improvement Opportunities

- **Add `report_summary.rs` to FxHashMap migration list (priority: medium)**: `report_summary.rs` uses `std::collections::HashMap` for `by_severity`, `by_confidence`, `by_type`, and `asset_counts` (lines 10-12, 28-31). These should use `FxHashMap` for consistency with the rest of the output module. (`crates/slapper/src/output/report_summary.rs:4,10-12,28-31`)
- **Add `pipeline/report.rs` escape_csv to documentation (priority: low)**: `pipeline/report.rs:10-22` has its own `escape_csv()` function that lacks NFKC normalization (unlike `output/escape.rs:16-35`). This is a potential formula injection vector if pipeline CSV output is opened in spreadsheet software. (`crates/slapper/src/pipeline/report.rs:10-22`)
- **Document Streaming CSV export (priority: low)**: `CsvExporter::export_findings_streaming()` at `csv.rs:34-72` provides async streaming CSV export but is not mentioned in the architecture doc. (`crates/slapper/src/output/csv.rs:34-72`)

## Stale Items

- **`attack_graph` module availability claim (line 53)**: The claim that `to_html()` can be used without `advanced-hunting` is stale/incorrect. The entire module is feature-gated at `output/mod.rs:51-52`. Recommendation: Update the doc to clarify that while `to_html()` itself has no conditional compilation, the entire module requires the feature flag.
- **Key Types table missing `RunManifest::from_report()` (line 205)**: The table lists `RunManifest` at `run_manifest.rs` but doesn't mention the `from_report()` factory method or `populate_findings_from_report()` method, which are key integration points. (`crates/slapper/src/output/run_manifest.rs:103,179`)
