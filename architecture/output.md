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

### Diff Engine (`diff.rs`)

Detailed comparison with escalation tracking:

```rust
pub struct DiffEngine;
pub fn has_regressions(diff: &DiffResult) -> bool;  // checks Critical escalations
```

## Key Types

| Type | Location | Purpose |
|------|----------|---------|
| `AgentFinding` | `agent.rs` | Core finding with evidence, remediation, confidence |
| `FindingSummary` | `agent.rs` | Aggregated statistics by severity/confidence/type |
| `ScanReportData` | `convert.rs` | Intermediate format for conversions |
| `SeverityCounts` | `report.rs` | Severity breakdown with risk scoring |
| `DiffResult` | `diff.rs` | Finding set comparison result |
| `TrendAnalysis` | `trend.rs` | Historical trend data |

## Performance Notes

**Hash Collections**: Use `rustc_hash::FxHashMap` instead of `std::collections::HashMap` for performance in:
- `trend.rs` - `ResultComparator`, `TrendAnalyzer`
- `agent.rs` - `FindingSummary`

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