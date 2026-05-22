# Slapper Output Skill

Report generation module workflows and patterns for exporting scan results.

## Key Types and Patterns

### Report Formats
`output/` supports multiple output formats:
- JSON (via `convert_to_json()`)
- HTML (via `convert_to_html()`)
- SARIF (via `convert_to_sarif()`)
- JUnit XML (via `convert_to_junit()`)
- Markdown (via `convert_to_markdown()`)
- CSV (via `convert_to_csv()`)
- PDF (feature-gated, via `PdfGenerator`)

### Severity Re-export
`output/agent::Severity` and `output::trend::Severity` re-export from `crate::types::Severity`.

### Hash Collections
**Important**: Use `FxHashMap`/`FxHashSet` instead of `std::collections::HashMap` for performance:
- `trend.rs` - `ResultComparator::compare()`, `TrendAnalyzer::get_findings_by_category()`, `TrendAnalyzer::get_most_common_findings()`
- `agent.rs` - `FindingSummary::from_findings()`

```rust
use rustc_hash::FxHashMap;

let mut map: FxHashMap<String, usize> = FxHashMap::default();
```

### Dedup Strategies
`dedup.rs` provides three strategies:
- `Strict` - deduplicates by `severity:title:target`
- `Fuzzy` - deduplicates by `severity:title` only
- `Disabled` - no deduplication

### CSV Escaping
`escape_csv()` in `escape.rs` implements formula injection protection using NFKC normalization:
- Prefixes cells starting with `=`, `+`, `-`, `@`, tab, or CR are quoted
- Handles commas, quotes, and newlines by quoting

### XXE Safety
Both SARIF and JUnit modules are immune to XXE attacks:
- **SARIF** (`sarif.rs`): Uses `serde_json` (not XML), operates on in-memory structures
- **JUnit** (`junit.rs`): Uses `quick_xml::Writer` in write-only mode without entity expansion

## Testing

### Running Output Tests
```bash
cargo test --lib -p slapper output::
```

### Writing Tests
Follow existing test patterns in `output/` modules, testing report generation for all supported formats.

## Common Tasks

### Adding a New Report Format
1. Implement format generation in `output/`
2. Re-export `Severity` from `crate::types::Severity` if needed
3. Use `FxHashMap`/`FxHashSet` for hash collections
4. Add tests for new report format

### Using Trend Analysis
```rust
use crate::output::trend::{TrendAnalyzer, TrendDirection};

// Analyze findings over time
let mut analyzer = TrendAnalyzer::new();
analyzer.add_result(scan_result);
let trend = analyzer.get_trend();
match trend.direction {
    TrendDirection::Worsening => println!("Security posture degrading"),
    TrendDirection::Improving => println!("Security posture improving"),
    TrendDirection::Stable => println!("No significant change"),
}
```

### Using Baseline Comparison
```rust
use crate::output::baseline::BaselineComparison;

let comparison = BaselineComparison::compare(&current_findings, &baseline_findings);
if comparison.has_new_findings() {
    println!("{} new findings since baseline", comparison.new_finding_count());
}
```

### Using DiffEngine for Finding Changes
```rust
use crate::output::diff::{DiffEngine, DiffResult};

let diff = DiffEngine::compare(&old_findings, &new_findings);
if DiffEngine::has_regressions(&diff) {
    // Critical findings have escalated
}
```

## Resources
- `crates/slapper/src/output/AGENTS.override.md` - Detailed output patterns
- `AGENTS.md` - General project guidelines
- `architecture/output.md` - Module architecture documentation