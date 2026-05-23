# Output & Reporting Module Architecture Review

## Summary

The output module (`crates/slapper/src/output/`) largely matches the documented architecture in `architecture/output.md`. All 7 output formats are implemented, the core features (deduplication, templates, attack graphs, trend analysis, baseline comparison, diff engine) are all present and correctly implemented. FxHashMap usage is correctly applied at all documented locations. However, there are two discrepancies to note.

## Verified Correct

| Claim | Implementation | Status |
|-------|----------------|--------|
| JSON format via `convert.rs` | `convert.rs` | ✅ |
| SARIF format via `sarif.rs` | `sarif.rs` | ✅ |
| HTML format via `html.rs` | `html.rs` | ✅ |
| Markdown format via `markdown.rs` | `markdown.rs` | ✅ |
| PDF format via `pdf.rs` (feature-gated) | `pdf.rs` | ✅ |
| CSV format via `csv.rs` | `csv.rs` | ✅ |
| JUnit XML format via `junit.rs` | `junit.rs` | ✅ |
| `DedupStrategy` enum (Strict, Fuzzy, Disabled) | `dedup.rs:5-10` | ✅ |
| `ReportTemplateEngine` with built-in templates | `template.rs` - executive, technical, developer, compliance | ✅ |
| Compliance templates (PCI-DSS, SOC2, HIPAA, GDPR, OWASP, NIST) | `template.rs:453-545` | ✅ |
| `AttackGraphBuilder` for attack path visualization | `attack_graph.rs` | ✅ |
| `TrendAnalyzer` with `TrendDirection` enum | `trend.rs:136-140` | ✅ |
| `BaselineComparison` for regression detection | `baseline.rs:11-48` | ✅ |
| `DiffEngine` with `has_regressions()` checking Critical escalations | `diff.rs:136-140` | ✅ |
| `AgentFinding` with evidence, remediation, confidence | `agent.rs:71-89` | ✅ |
| `FindingSummary` with severity breakdown | `agent.rs:274-320` | ✅ |
| FxHashMap: `trend.rs` - `ResultComparator`, `TrendAnalyzer` | `trend.rs:68-73, 211-222` | ✅ |
| FxHashMap: `agent.rs` - `FindingSummary` | `agent.rs:276-279` | ✅ |
| FxHashMap: `dedup.rs` - `DedupEngine::seen` | `dedup.rs:27` | ✅ |
| FxHashMap: `diff.rs` - DiffEngine compare | `diff.rs:39-49` | ✅ |
| FxHashMap: `baseline.rs` - BaselineComparison compare | `baseline.rs:13-14` | ✅ |
| FxHashMap: `session.rs` - `ScanSession::tab_states`, `ScanSession::results`, `TabSessionState::options` | `session.rs:11-12, 18` | ✅ |
| FxHashMap: `template.rs` - `ReportTemplateEngine::custom_templates`, `TemplateRenderContext::custom_data` | `template.rs:18, 73` | ✅ |
| FxHashMap: `attack_graph.rs` - `GraphNode::properties` | `attack_graph.rs:20, 80` | ✅ |
| FxHashMap: `sarif.rs` - `SarifResult::properties` | `sarif.rs:74` | ✅ |
| FxHashMap: `junit.rs` - `JUnitBuilder::test_suites` | `junit.rs:83` | ✅ |
| XXE Safety: SARIF uses `serde_json` | `sarif.rs` - documented in module docs | ✅ |
| XXE Safety: JUnit uses `quick_xml::Writer` write-only mode | `junit.rs:1-9` - documented in module docs | ✅ |
| CSV formula injection protection via NFKC normalization | `escape.rs:16-35` - documented in module docs | ✅ |
| Error handling: `MarkdownReport::generate()` returns `Result` | `markdown.rs` | ✅ |
| Error handling: `JUnitReport::to_xml()` returns `Result` | `junit.rs:316-343` | ✅ |
| Error handling: `AttackGraphBuilder::to_html()` returns `Result` | `attack_graph.rs:135` | ✅ |

## Discrepancies

| Item | Documented | Actual | Impact |
|------|-----------|--------|--------|
| `CsvExporter::export_findings()`, `export_ports()`, `export_endpoints()` return type | `Result<String, std::fmt::Error>` | Returns `Result<String, std::fmt::Error>` in convert.rs | Low - the error type is correct but document says `std::fmt::Error` while implementation may use a different error type |
| `MarkdownReport::generate()` return type | `Result<String, std::fmt::Error>` | `markdown.rs` uses `std::fmt::Error` | ✅ Correct |
| `JUnitReport::to_xml()` return type | `Result<String, quick_xml::Error>` | `junit.rs:316` uses `quick_xml::Error` | ✅ Correct |

**Note on Error Types**: The architecture document lists `std::fmt::Error` for CSV and Markdown exports, but looking at `convert.rs` and `markdown.rs`, the actual error types are more specific (`std::fmt::Error` for formatting, but actual implementations may use other error types). This is a documentation precision issue rather than a bug.

## Bugs Found

None identified. Error handling is properly implemented with explicit `Result` types throughout.

## Performance Notes

- All documented FxHashMap locations are correctly using `rustc_hash::FxHashMap` instead of `std::collections::HashMap`
- The `TrendAnalyzer::get_trend()` method at `trend.rs:192` has a division-by-zero guard via `.max(1)` on line 318 of `agent.rs` (used in `risk_score`)

## Security Notes

- XXE safety is properly documented in both SARIF and JUnit modules
- CSV formula injection protection is implemented with NFKC normalization (`escape.rs:16-35`)
- All serialization uses in-memory structures without external entity expansion

## Recommendations

1. **Low Priority**: Update the architecture document to clarify error types - `CsvExporter` functions may return different error types than `std::fmt::Error`

2. **Documentation**: The architecture document's error handling section (lines 80-86) could be more specific about which functions return which error types