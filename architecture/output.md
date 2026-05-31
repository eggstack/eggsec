# Output & Reporting Module

The Output module handles the formatting, deduplication, and export of security findings and scan data into various standardized formats.

## Supported Formats (`src/output/`)

Slapper supports a wide range of output formats to integrate with different tools and workflows:

| Format | File | Purpose |
|--------|------|---------|
| JSON | `convert.rs` | Pretty-printed and compact JSON |
| SARIF | `sarif.rs` | Static Analysis Results Interchange Format |
| HTML | `html.rs` | Human-readable, interactive reports with charts |
| Markdown | `markdown.rs` | Easy copy-pasting into documentation |
| PDF | `pdf.rs` | Formal reporting (feature-gated) |
| CSV | `csv.rs` | Spreadsheet-based analysis |
| JUnit XML | `junit.rs` | CI/CD pipeline integration |

## Core Features

### Deduplication (`dedup.rs`)

Automatically identifies and groups duplicate findings:

```rust
pub enum DedupStrategy {
    Strict,      // severity:title:target
    Fuzzy,       // severity:title only
    Disabled,
}
```

### Templates (`template.rs`)

Handlebars-based templating with built-in templates:
- `executive` - High-level summary for management
- `technical` - Detailed technical findings
- `developer` - Actionable items for developers
- `compliance` - PCI-DSS, SOC2, HIPAA, GDPR, OWASP, NIST

### Attack Graphs (`attack_graph.rs`)

Visualizes relationships between findings to show potential attack paths.

### Trend Analysis (`trend.rs`)

Compares current results with historical data:

```rust
pub struct TrendAnalyzer { ... }
pub enum TrendDirection { Improving, Stable, Worsening }
```

### Baseline Comparison (`baseline.rs`)

Detects regressions by comparing current findings against a baseline.

### Scheduling (`schedule.rs`)

Provides cron-based scan scheduling with queue management:
- `CronScheduler` - Parses cron expressions and manages scheduled scans
- `ScanQueue` - Priority queue for scan scheduling with status tracking

### Diff Engine (`diff.rs`)

Detailed comparison with escalation tracking:

```rust
pub struct DiffEngine;
pub fn has_regressions(diff: &DiffResult) -> bool;  // checks >= Severity::High (High AND Critical)
```

## Run Manifest (`run_manifest.rs`)

The `RunManifest` provides a structured summary of a single assessment run. It is designed for regression-oriented workflows where runs must be comparable, reproducible, and diffable against a baseline.

```rust
pub struct RunManifest {
    pub schema_version: String,
    pub run_id: String,
    pub started_at: DateTime<Utc>,
    pub ended_at: DateTime<Utc>,
    pub slapper_version: String,
    pub target_scope: String,
    pub profile: String,
    pub probe_intents: Vec<String>,
    pub risk_budget: String,
    pub feature_flags: Vec<String>,
    pub observations: Vec<serde_json::Value>,
    pub findings: Vec<serde_json::Value>,
    pub artifacts: Vec<String>,
    pub baseline_id: Option<String>,
    pub diff_summary: Option<DiffSummary>,
}
```

The manifest is a metadata envelope — it does not replace existing finding or diff types. Instead it wraps them with provenance (run identity, scope, profile, feature flags) so that two manifests can be meaningfully compared.

For defense-lab regression workflows, a baseline run produces a manifest with `baseline_id: None`. Subsequent runs reference the baseline via `baseline_id` and populate `diff_summary` with the delta. See `architecture/defense_lab.md` for the full workflow.

The manifest is integrated into the pipeline output path: `PipelineReport` carries an optional `RunManifest` that is auto-generated after each run. The manifest is serialized alongside the report when output is written to disk.

## Key Types

| Type | Location | Purpose |
|------|----------|---------|
| `RunManifest` | `run_manifest.rs` | Run-level metadata envelope for regression workflows |
| `AgentFinding` | `agent.rs` | Core finding with evidence, remediation, confidence |
| `FindingSummary` | `agent.rs` | Aggregated statistics by severity/confidence/type |
| `ScanReportData` | `convert.rs` | Intermediate format for conversions |
| `SeverityCounts` | `report.rs` | Severity breakdown with risk scoring |
| `DiffResult` | `diff.rs` | Finding set comparison result |
| `TrendAnalysis` | `trend.rs` | Historical trend data |

### Error Handling

**Important**: Methods that perform I/O or serialization return `Result` types:
- `CsvExporter::export_findings()`, `export_ports()`, `export_endpoints()` return `Result<String, std::fmt::Error>`
- `MarkdownReport::generate()` returns `Result<String, std::fmt::Error>`
- `JUnitReport::to_xml()` returns `Result<String, quick_xml::Error>`
- `AttackGraphBuilder::to_html()` returns `Result<String, serde_json::Error>`

Avoid using `unwrap_or_default()` on serialization - use explicit error handling instead.

## Performance Notes

**Hash Collections**: Use `rustc_hash::FxHashMap` instead of `std::collections::HashMap` for performance in:
- `trend.rs` - `ResultComparator`, `TrendAnalyzer`
- `agent.rs` - `FindingSummary`
- `dedup.rs` - `DedupEngine::seen`
- `diff.rs` - DiffEngine compare function
- `baseline.rs` - BaselineComparison compare function
- `session.rs` - `ScanSession::tab_states`, `ScanSession::results`, `TabSessionState::options`
- `template.rs` - `ReportTemplateEngine::custom_templates`, `TemplateRenderContext::custom_data`
- `attack_graph.rs` - `GraphNode::properties`
- `sarif.rs` - `SarifResult::properties`
- `junit.rs` - `JUnitBuilder::test_suites`

## Security Notes

### XXE Safety

Both SARIF and JUnit modules are immune to XXE attacks:
- **SARIF**: Uses `serde_json` (JSON format), no XML parsing
- **JUnit**: Uses `quick_xml::Writer` in write-only mode without entity expansion

### CSV Formula Injection Protection

`escape_csv()` in `escape.rs` uses NFKC normalization and quoting to prevent formula injection attacks.

## Integration

The Output module is typically the final stage in any Slapper operation. It can also be used independently to convert or merge existing Slapper result files.

```rust
use slapper::output::{convert_to_csv, load_scan_report};

let report = load_scan_report("scan.json")?;
let csv = convert_to_csv(&report);
```