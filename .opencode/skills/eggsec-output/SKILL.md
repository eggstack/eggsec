---
name: eggsec-output
description: "Report generation and export formats - use when working with JSON/HTML/SARIF/JUnit/Markdown/CSV/PDF output, trend analysis, baseline comparison, deduplication, or diff engine."
---

# Eggsec Output Skill

Report generation module workflows and patterns for exporting scan results.

## Crate Location

Most output types and renderers live in `crates/eggsec-output/`. The `eggsec` crate
re-exports them via `pub use eggsec_output::*` in `crates/eggsec/src/output/mod.rs`.
Engine-coupled modules (`pdf`, `report`, `report_summary`, `run_manifest`, `attack_graph`)
remain in `crates/eggsec/src/output/`.

## Key Types and Patterns

### Normalized Report Envelope
The `envelope` module (`eggsec_output::envelope`) provides protocol-neutral report types for cross-domain report unification. Domain crates convert their domain-specific types into `ReportEnvelope` via `to_report_envelope()` functions. This module is always available (no feature gate).

Key types: `ReportEnvelope`, `FindingRecord`, `EvidenceItem`, `EvidenceManifest`, `BaselineSummary`, `ToolMetadata`, `EvidenceKind`, `EvidenceSource`, `RedactionState`, `RedactionPolicy`.

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
- `session.rs` - `ScanSession::tab_states`, `ScanSession::results`
- `attack_graph.rs` - `GraphNode::properties`
- `sarif.rs` - `SarifResult::properties`
- `junit.rs` - `JUnitBuilder::test_suites`
- `dedup.rs` - `DedupEngine::seen`
- `diff.rs` - `DiffEngine::compare()`

```rust
use rustc_hash::FxHashMap;

let mut map: FxHashMap<String, usize> = FxHashMap::default();
```

### Error Handling
**Important**: Methods that perform I/O or serialization should return `Result` types:
- `CsvExporter::export_findings()`, `export_ports()`, `export_endpoints()` return `Result<String, std::fmt::Error>`
- `MarkdownReport::generate()` returns `Result<String, std::fmt::Error>`
- `JUnitReport::to_xml()` returns `Result<String, quick_xml::Error>`
- `AttackGraphBuilder::to_html()` returns `Result<String, serde_json::Error>`
- `TemplateRenderContext::render_with_styling()` uses explicit `map_err` instead of `unwrap_or_default()`

When using `CsvExporter` methods, handle errors appropriately:
```rust
let csv = CsvExporter::export_ports(&ports).unwrap_or_default();
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
cargo test -p eggsec-output
cargo test --lib -p eggsec output::
```

### Writing Tests
Follow existing test patterns in `output/` modules, testing report generation for all supported formats.

## Common Tasks

### Adding a New Report Format
1. Implement format generation in `crates/eggsec-output/src/`
2. Re-export `Severity` from `eggsec_core::types::Severity` if needed
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
- `crates/eggsec-output/src/` - Output crate source code
- `crates/eggsec/src/output/AGENTS.override.md` - Detailed output patterns
- `AGENTS.md` - General project guidelines
- `architecture/output.md` - Module architecture documentation
- `docs/USAGE.md` (Report Management) - short shared "Output Models" block explaining pipeline `ScanReportData` vs. wireless/mobile optional bridge vs. `auth-test` local-only (no bridge). See also per-module Integration sections and architecture/{output,cli_commands,defense_lab,wireless,mobile,auth}.md for the adopted standalone defense-lab pattern (integration-priorities-1-2-plan.md). Final cleanup recorded in `plans/final-cleanup-new-modules-plan.md` (resolution note at top). Active wireless (Phase 1, complete 2026-06-12, under `wireless-advanced` per `plans/wireless-active-attacks-loadout-design-plan.md`) extends the bridge with `wireless-active-*` categories (same standalone + MCP-absent model). Dynamic mobile Phase 4a (Correlation Engine + Evidence Foundation + polish handoff executed 2026-06-12) enriches the surface with `CorrelationEngine` / `correlate_reports` / enriched `CorrelatedFinding` (score/correlation_type/enrichment) / `CorrelationResult` (with timeline/summary) + scoring inside correlate_findings + build_timeline; builds on 3c baseline/regression/bundles (MobileBaseline, --baseline, --evidence-bundle); human output surfaces regression/correlation hints; auto-bridge categories include behavioral-regression + frida-*; all under single mobile-dynamic; see docs/MOBILE.md "Current Status" + Phase 4a example + plans.