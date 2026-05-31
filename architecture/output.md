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

```rust
pub struct AttackGraph {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub clusters: Vec<GraphCluster>,
}
```

`AttackGraphBuilder::from_chains()` converts `AttackChain` values into graph structures and is feature-gated behind `advanced-hunting`. `AttackGraphBuilder::to_html()` is **not** feature-gated — it accepts any `&AttackGraph` and renders an HTML page with D3.js visualization scaffolding, so it can be used with manually constructed graphs without enabling the `advanced-hunting` feature.

### Trend Analysis (`trend.rs`)

Compares current results with historical data using LRU cache storage:

```rust
pub struct TrendAnalyzer {
    results: LruCache<String, ScanResult>,  // capacity: 1000 (NonZeroUsize)
}
pub enum TrendDirection { Improving, Stable, Worsening }
```

`TrendAnalyzer` stores up to 1000 `ScanResult` entries in an `lru::LruCache` keyed by result ID. When the cache is full, the least-recently-used entry is evicted. `get_trend()` sorts results by timestamp and computes sliding-window deltas for critical, high, and medium finding counts across consecutive scans. `ResultComparator` provides lower-level comparison with composite deduplication keys `(title, category, cve)`.

### Baseline Comparison (`baseline.rs`)

Detects regressions by comparing current findings against a baseline.

### Scheduling (`schedule.rs`)

Provides cron-based scan scheduling with queue management:
- `CronScheduler` - Parses cron expressions and manages scheduled scans
- `ScanQueue` - Priority queue for scan scheduling with status tracking

### Diff Engine (`diff.rs`)

Detailed comparison with escalation tracking:

```rust
pub struct DiffFinding {
    pub id: String,
    pub title: String,
    pub severity: Severity,
    pub description: String,
    pub first_seen: String,
    pub last_seen: String,
}

pub struct DiffResult {
    pub new_findings: Vec<DiffFinding>,
    pub resolved_findings: Vec<DiffFinding>,
    pub escalated_findings: Vec<DiffFinding>,
    pub deescalated_findings: Vec<DiffFinding>,
    pub unchanged_findings: Vec<DiffFinding>,
    pub summary: DiffSummary,
}

pub struct DiffEngine;
pub fn has_regressions(diff: &DiffResult) -> bool;  // checks >= Severity::High (High AND Critical)
```

`DiffFinding` tracks individual findings across scans with `first_seen` and `last_seen` timestamps. Severity escalation/de-escalation is determined by comparing `Severity::as_int()` values between old and new findings.

### Report Summary (`report_summary.rs`)

Aggregated statistics and risk narrative generation from findings:

```rust
pub struct ReportSummary {
    pub total_findings: usize,
    pub by_severity: HashMap<String, usize>,
    pub by_confidence: HashMap<String, usize>,
    pub by_type: HashMap<String, usize>,
    pub top_affected_assets: Vec<AssetCount>,
    pub risk_narrative: String,
    pub remediation_summary: Vec<String>,
}

pub struct AssetCount {
    pub asset: String,
    pub count: usize,
}
```

`ReportSummary::from_findings()` builds a summary from a slice of `Finding` values. It aggregates counts by severity, confidence, and type; identifies the top 10 most-affected assets; deduplicates remediation suggestions; and generates a `risk_narrative` string.

The risk narrative (`generate_risk_narrative()`) produces a severity-annotated text summary: critical findings trigger a "CRITICAL" prefix, high findings a "HIGH" prefix, and so on. If no findings exist, it returns `"No findings detected."`.

### Scheduling (`schedule.rs`)

Cron-based scan scheduling with priority queuing and rate limiting:

```rust
pub struct CronScheduler {
    expressions: Vec<CronExpression>,
}

pub struct CronExpression {
    pub second: u8,
    pub minute: u8,
    pub hour: u8,
    pub day_of_month: u8,
    pub month: u8,
    pub day_of_week: u8,
    // internal matchers for each field
}

pub struct ScanQueue {
    queue: VecDeque<ScheduledScan>,
    max_size: usize,
    running: Option<ScheduledScan>,
}

pub struct RateLimiter {
    requests_per_second: u32,
    burst_size: u32,
    tokens: u32,
    last_refill: Instant,
}
```

`CronScheduler` parses 5- or 6-field cron expressions (with optional seconds field) and evaluates them against `DateTime<Utc>`. It supports wildcards (`*`), exact values, and step expressions (`*/15`). `next_run()` performs a linear scan up to 7 days ahead.

`ScanQueue` is a priority-based queue (max size 100 by default) that inserts scans in priority order (`Low < Normal < High < Critical`). Only one scan runs at a time via `start_next()`.

`RateLimiter` implements a token bucket algorithm with configurable requests-per-second and burst size (2x rate).

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
| `DiffFinding` | `diff.rs` | Individual finding with first/last seen timestamps |
| `ReportSummary` | `report_summary.rs` | Aggregated statistics and risk narrative |
| `TrendAnalysis` | `trend.rs` | Historical trend data |
| `CronScheduler` | `schedule.rs` | Cron-based scan scheduling |
| `ScanQueue` | `schedule.rs` | Priority-based scan queue |

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