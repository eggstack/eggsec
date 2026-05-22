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
| `TrendAnalysis` | `trend.rs` | Historical trend analysis with direction |

## Performance: Use FxHashMap

For performance-critical code, use `rustc_hash::FxHashMap` instead of `std::collections::HashMap`:

```rust
use rustc_hash::FxHashMap;

let mut by_severity: FxHashMap<Severity, usize> = FxHashMap::default();
```

Files needing updates:
- `trend.rs` - `ResultComparator`, `TrendAnalyzer`
- `agent.rs` - `FindingSummary`

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

## PDF Generation

PDF generation is feature-gated:
```rust
#[cfg(feature = "pdf")]
let pdf_bytes = PdfGenerator::generate_report(findings, &config)?;
```

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