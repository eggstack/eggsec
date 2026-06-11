# Output & Reporting Module

The Output module handles the formatting, deduplication, and export of security findings and scan data into various standardized formats.

## Supported Formats (`crates/eggsec-output/src/`)

Most output code now lives in the `eggsec-output` crate (`crates/eggsec-output/src/`). Modules with deep engine coupling (`pdf`, `template`, `run_manifest`, `attack_graph`, `report`, `report_summary`) remain in `crates/eggsec/src/output/`. Eggsec supports a wide range of output formats to integrate with different tools and workflows:

| Format | File | Purpose |
|--------|------|---------|
| Pretty | `report.rs` | Formatted console output (default) |
| JSON | `convert.rs` | Pretty-printed JSON |
| Compact | `convert.rs` | Compact single-line JSON |
| HTML | `html.rs` | Human-readable, interactive reports with charts |
| Markdown | `markdown.rs` | Easy copy-pasting into documentation |
| CSV | `csv.rs` | Spreadsheet-based analysis |
| SARIF | `sarif.rs` | Static Analysis Results Interchange Format |
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

`AttackGraphBuilder::from_chains()` converts `AttackChain` values into graph structures. The `AttackChain` type import is feature-gated behind `advanced-hunting`, so `from_chains()` is only available when that feature is enabled. `AttackGraphBuilder::to_html()` is **not** feature-gated — it accepts any `&AttackGraph` and renders an HTML page with D3.js visualization scaffolding, so it can be used with manually constructed graphs without enabling the `advanced-hunting` feature.

### Trend Analysis (`trend.rs`)

Compares current results with historical data using LRU cache storage:

```rust
pub struct TrendAnalyzer {
    results: LruCache<String, ScanResult>,  // capacity: 1000 (NonZeroUsize)
}
pub enum TrendDirection { Improving, Stable, Worsening }
```

`TrendAnalyzer` stores up to 1000 `ScanResult` entries in an `lru::LruCache` keyed by result ID. When the cache is full, the least-recently-used entry is evicted. `get_trend()` sorts results by timestamp and computes sliding-window deltas for critical, high, and medium finding counts across consecutive scans. The overall `TrendDirection` is determined by the critical trend only: any increase yields `Worsening`, any decrease yields `Improving`, otherwise `Stable`. `ResultComparator` provides lower-level comparison with composite deduplication keys `(title, category, cve)`.

### Baseline Comparison (`baseline.rs`)

Detects regressions by comparing current findings against a baseline.

### Session Persistence (`session.rs`)

Persists scan state across TUI sessions:

**Note on standalone commands**: Not all CLI surfaces route through `ScanReportData` or the `eggsec-output` converters. `auth-test` (CredentialTesting risk) builds and emits `AuthTestReport`/`AuthFinding` (local types defined in `auth/mod.rs`) directly from its handler as pretty text or `--json` (see `commands/handlers/auth_test.rs:274-285`). These results are not loadable via `load_scan_report` or convertible to SARIF/JUnit/CSV/etc. via the output crate. `ScanProfile::Auth` is a separate pipeline profile (JWT/OAuth/IDOR fuzzing) that does not invoke the `auth/` testers. See `architecture/auth.md` and `architecture/cli_commands.md` (Special Cases section) for details. (This is the adopted model; no `AuthFinding` → canonical conversion was implemented.)

### Session Persistence (`session.rs`)

Persists scan state across TUI sessions:

```rust
pub struct ScanSession {
    pub version: String,
    pub created_at: String,
    pub last_modified: String,
    pub tab_states: FxHashMap<String, TabSessionState>,
    pub results: FxHashMap<String, serde_json::Value>,
}
```

`ScanSession` saves/loads tab input state and results to JSON files. `TabSessionState` tracks per-tab input fields and options via `FxHashMap`.

### AI Output Schema (`ai_schema.rs`)

Typed output for AI consumption:

```rust
pub struct AiOutput {
    pub findings: Vec<AiFinding>,
    pub summary: AiSummary,
}
```

`AiOutput::from_findings()` computes a risk score (0-10) weighted by severity and confidence, and generates an executive summary string.

### PDF Report (`pdf.rs`)

Feature-gated (`pdf` feature) PDF generation using `printpdf`. `PdfGenerator::generate_report()` renders findings with severity-colored markers and metadata headers.

### Escape Utilities (`escape.rs`)

Shared escaping functions used across output formats:
- `escape_html()` - HTML entity encoding
- `escape_csv()` - NFKC normalization + quoting for formula injection protection
- `escape_xml()` - XML entity encoding

### Scheduling (`schedule.rs`)

Provides cron-based scan scheduling with queue management:
- `CronScheduler` - Parses cron expressions and manages scheduled scans
- `ScanQueue` - Priority queue for scan scheduling with status tracking

### Diff Summary (`diff.rs`)

Provides a summary struct for diff results used by `RunManifest`:

```rust
pub struct DiffSummary {
    pub total_new: usize,
    pub total_resolved: usize,
    pub total_escalated: usize,
    pub total_deescalated: usize,
    pub net_change: i32,
}
```

`DiffSummary` is a lightweight metadata envelope used by `RunManifest` to record the delta between two assessment runs.

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
    pub eggsec_version: String,
    pub target_scope: String,
    pub profile: String,
    pub probe_intents: Vec<ProbeIntent>,
    pub risk_budget: ProbeRisk,
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

### `RunManifest::from_report()` (`run_manifest.rs:103-179`)

Constructs a `RunManifest` from a completed `PipelineReport`. The conversion logic:

- **`started_at`**: Computed as `now - report.total_duration_ms` (derived from the report's measured duration).
- **`ended_at`**: Set to `Utc::now()`.
- **`run_id`**: Generated via `uuid::Uuid::new_v4()`.
- **`observations`**: Built by chaining three iterators:
  1. Open ports → `{"type": "port", "port", "status", "service"}`
  2. Services → `{"type": "service", "port", "service", "product", "version"}`
  3. Interesting endpoints → `{"type": "endpoint", "path", "status_code", "content_length"}`
- **`probe_intents`**: `ProbeIntent` values derived from successful `stage_results` via `stage.to_probe_intent()`.
- **`feature_flags`**: All stage results formatted as `"stage:{name}"` (success) or `"stage:{name}:failed"` (failure).
- **`findings`**: Left empty; populated separately via `populate_findings_from_report()`.

`populate_findings_from_report()` (`run_manifest.rs:179-194`) generates a finding for each interesting endpoint with severity `Info`, category `"endpoint_discovery"`, and a description including the status code.

## Key Types

| Type | Location | Purpose |
|------|----------|---------|
| `RunManifest` | `run_manifest.rs` | Run-level metadata envelope for regression workflows |
| `AgentFinding` | `agent.rs` | Core finding with evidence, remediation, confidence |
| `FindingSummary` | `agent.rs` | Aggregated statistics by severity/confidence/type |
| `ScanReportData` | `convert.rs` | Intermediate format for conversions |
| `SeverityCounts` | `report.rs` | Severity breakdown with risk scoring |
| `DiffSummary` | `diff.rs` | Lightweight diff envelope for run manifests |
| `ReportSummary` | `report_summary.rs` | Aggregated statistics and risk narrative |
| `TrendAnalysis` | `trend.rs` | Historical trend data |
| `CronScheduler` | `schedule.rs` | Cron-based scan scheduling |
| `ScanQueue` | `schedule.rs` | Priority-based scan queue |
| `ScanSession` | `session.rs` | TUI session persistence |
| `AiOutput` | `ai_schema.rs` | AI-consumable finding output with risk score |
| `PdfGenerator` | `pdf.rs` | PDF report generation (feature-gated) |
| `BaselineComparison` | `baseline.rs` | Regression detection against baseline |

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
- `report_summary.rs` - `ReportSummary::from_findings` (by_severity, by_confidence, by_type, asset_counts)

All `HashMap` usage in the output module has been migrated to `FxHashMap`, including `report_summary.rs` (`by_severity`, `by_confidence`, `by_type`, `asset_counts`).

## Security Notes

### XXE Safety

Both SARIF and JUnit modules are immune to XXE attacks:
- **SARIF**: Uses `serde_json` (JSON format), no XML parsing
- **JUnit**: Uses `quick_xml::Writer` in write-only mode without entity expansion

### CSV Formula Injection Protection

`escape_csv()` in `escape.rs` uses NFKC normalization and quoting to prevent formula injection attacks.

## Integration

The Output module is typically the final stage in any Eggsec operation. It can also be used independently to convert or merge existing Eggsec result files.

```rust
use eggsec::output::{convert_to_csv, load_scan_report};

let report = load_scan_report("scan.json")?;
let csv = convert_to_csv(&report);
```