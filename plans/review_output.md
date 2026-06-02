# Output Module Architecture Review

**Document:** architecture/output.md
**Reviewed:** 2026-06-02
**Accuracy:** Medium
**Lines Reviewed:** 281

## Verified Claims

- DedupStrategy enum (Strict, Fuzzy, Disabled): Verified at `output/dedup.rs:6-11`
- Templates (executive, technical, developer, compliance): Verified at `output/template.rs:142-152`
- Compliance standards (PCI-DSS, SOC2, HIPAA, GDPR, OWASP, NIST): Verified at `output/template.rs:32-39`
- AttackGraph struct (nodes, edges, clusters): Verified at `output/attack_graph.rs:7-12`
- AttackGraphBuilder::from_chains() is feature-gated behind `advanced-hunting`: Verified at `output/attack_graph.rs:1-2`
- TrendAnalyzer LruCache capacity 1000: Verified at `output/trend.rs:5` and `output/trend.rs:154`
- DiffEngine::has_regressions() checks >= Severity::High: Verified at `output/diff.rs:137-141`
- DiffEngine uses Severity::as_int() for escalation comparison: Verified at `output/diff.rs:83`
- RunManifest::from_report() at lines 103-175: Verified at `output/run_manifest.rs:103-175`
- populate_findings_from_report() at lines 179-194: Verified at `output/run_manifest.rs:179-194`
- CronScheduler parses 5 or 6 field cron expressions: Verified at `output/schedule.rs:224-226`
- next_run() linear scan 7 days ahead: Verified at `output/schedule.rs:369`
- RateLimiter token bucket with 2x burst: Verified at `output/schedule.rs:160`
- ScanQueue priority ordering (Low < Normal < High < Critical): Verified at `output/schedule.rs:86-94`
- CSV formula injection protection uses NFKC normalization: Verified at `output/escape.rs:16-35`
- SARIF uses serde_json (no XXE risk): Verified at `output/sarif.rs:1-9`
- JUnit uses quick_xml::Writer write-only mode: Verified at `output/junit.rs:1-9`
- FindingSummary uses FxHashMap: Verified at `output/agent.rs:274-280`
- session.rs uses FxHashMap for tab_states, results: Verified (module exists with correct types)
- template.rs uses FxHashMap for custom_templates: Verified at `output/template.rs:19`
- sarif.rs uses FxHashMap for SarifResult::properties: Verified at `output/sarif.rs:74`
- junit.rs uses FxHashMap for JUnitBuilder::test_suites: Verified at `output/junit.rs:83`

## Discrepancies

- **Format count mismatch**: Document says "8 output formats" at line 9 header, but the table at lines 11-18 lists only 7 formats (JSON, SARIF, HTML, Markdown, PDF, CSV, JUnit XML). No 8th format exists in the module. The header claim is incorrect.
- **Error return type for AttackGraphBuilder::to_html()**: Documented at line 240 as returning `Result<String, serde_json::Error>`, which is correct per `output/attack_graph.rs:135`. However, this method is not publicly exported - the attack_graph module is feature-gated behind `advanced-hunting` and only `from_chains()` is exported via `pub use attack_graph::AttackGraphBuilder` in mod.rs.
- **MarkdownReport::generate()**: Documented at line 238 as returning `Result<String, std::fmt::Error>`. This is a method on `MarkdownReport` struct (verified at `output/markdown.rs:60`), not a standalone function.

## Bugs Found

- **No bugs found in core logic**: All verified methods behave as documented.

## Improvement Opportunities

- **HashMap migration incomplete**: Document at line 258 correctly identifies `report_summary.rs` as using `std::collections::HashMap` for `by_severity`, `by_confidence`, `by_type`, and `asset_counts`. This is verified at `output/report_summary.rs:4,28-31`. Migration to FxHashMap recommended for consistency (priority: low).
- **Feature-gated exports**: The `attack_graph` module is only publicly exposed when `advanced-hunting` feature is enabled (verified at `output/mod.rs:51-52,79-82`). Documentation should clarify that `AttackGraphBuilder::to_html()` is only available with the feature flag.
- **Defensive copy in dedup_strict()**: At `output/dedup.rs:50-51`, the key format is `severity:title:target`. The `AgentFinding::target` field is used correctly, but the format string creates a new String allocation for each finding on every call. For high-volume deduplication, this could be optimized.

## Stale Items

- **CSV formula injection test coverage**: The test at `output/escape.rs:42-49` verifies fullwidth character bypasses, but there's no test for the primary formula injection vector (values starting with =, +, -, @). Consider adding test coverage for these.

## Code Interrogation Findings

- **Escape functions exist but may not be used everywhere**: `escape_csv()` in escape.rs properly handles formula injection, but `pipeline/report.rs:10-22` has its own local `escape_csv()` function that does NOT use NFKC normalization. This could be a security concern if CSV export is used from the pipeline module directly.
- **RunManifest population gap**: `populate_findings_from_report()` at `output/run_manifest.rs:179-194` only creates findings for endpoints marked as `interesting`. Non-interesting endpoints are excluded, which may not reflect complete scan results.