# Output Module Override

Specialized guidance for the report generation module.

## Key Types

| Type | Location | Purpose |
|------|----------|---------|
| `AgentFinding` | `agent.rs` | Core finding type with evidence, remediation, confidence |
| `Severity` | `types.rs` | Re-exported via `output::agent::Severity` and `output::trend::Severity` |
| `ScanReportData` | `convert.rs` | Intermediate format for format conversions |
| `FindingSummary` | `agent.rs` | Aggregated finding statistics by severity/confidence/type |
| `DiffResult` | `diff.rs` | Result of comparing two finding sets |
| `DiffFinding` | `diff.rs:17` | Individual finding in diff comparison (fields: `id`, `title`, `severity`, `description`, `first_seen`, `last_seen`) |
| `TrendAnalysis` | `trend.rs` | Historical trend analysis with direction |
| `TrendAnalyzer` | `trend.rs:147` | Uses `LruCache<String, ScanResult>` with `NonZeroUsize::new(1000)` |
| `ReportSummary` | `report_summary.rs` | Summary with `risk_narrative: String` field |
| `CronScheduler` | `schedule.rs:201` | Cron-based scan scheduling with 5/6 field expressions |
| `ScanQueue` | `schedule.rs:66` | Priority-based scan queue with enqueue/dequeue/cancel |
| `AttackGraphBuilder` | `attack_graph.rs` | Graph visualization; entire module feature-gated behind `advanced-hunting` at `output/mod.rs:51` |

## Performance: Use FxHashMap

For performance-critical code, use `rustc_hash::FxHashMap` instead of `std::collections::HashMap`:

```rust
use rustc_hash::FxHashMap;

let mut by_severity: FxHashMap<Severity, usize> = FxHashMap::default();
```

All files in the output module use FxHashMap for hash collections. Key files:
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

## Report Format Conversions

```rust
use crate::output::convert::{
    load_scan_report, convert_to_csv, convert_to_html,
    convert_to_junit, convert_to_markdown, convert_to_sarif,
};
```

## Builder Patterns

### SARIF
```rust
use crate::output::sarif::SarifBuilder;

let report = SarifBuilder::new()
    .add_rule("SQLI001", "SQL Injection", "error", "SQL injection detected")
    .add_result("SQLI001", "error", "Payload in 'id' param", "/api/users?id=1")
    .build();
```

### JUnit
```rust
use crate::output::junit::{JUnitBuilder, JUnitTestResult};

let report = JUnitBuilder::new("Security Tests")
    .add_test_case("SQL Injection", "test_sqli", "SQLI", 0.5, JUnitTestResult::Passed)
    .build();
```

## Template Engine

`template.rs` provides Handlebars-based templating with built-in templates:
- `executive` - High-level summary for management
- `technical` - Detailed technical findings
- `developer` - Actionable items for developers
- `compliance` - PCI-DSS, SOC2, HIPAA, GDPR, OWASP, NIST

```rust
use crate::output::template::{ReportTemplateEngine, ComplianceStandard};

let engine = ReportTemplateEngine::new();
let pcidss = engine.get_compliance_template(ComplianceStandard::PCIDSS);
```

## Additional Notes

- **`has_regressions()` threshold**: Checks `severity >= Severity::High` (both High AND Critical), not just Critical. Code at `diff.rs:137-141`.
- **PDF generation**: Feature-gated behind `pdf` feature flag.

## Severity Counts

```rust
use crate::output::report::{SeverityCounts, ReportTemplate};

let counts = SeverityCounts {
    critical: 2,
    high: 5,
    medium: 10,
    low: 20,
    info: 30,
};
let risk_score = counts.risk_score(); // Weighted: critical*10 + high*7 + medium*4 + low*1
```