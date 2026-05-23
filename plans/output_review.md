# Output Module Architecture Review

Review of `architecture/output.md` against `crates/slapper/src/output/`

## Verified Claims

### 1. Format Support
All 7 formats documented exist and match implementation:
- JSON → `convert.rs`
- SARIF → `sarif.rs`
- HTML → `html.rs`
- Markdown → `markdown.rs`
- PDF → `pdf.rs` (feature-gated)
- CSV → `csv.rs`
- JUnit XML → `junit.rs`

### 2. DedupStrategy Enum (dedup.rs:6-10)
```rust
pub enum DedupStrategy {
    Strict,      // severity:title:target
    Fuzzy,       // severity:title only
    Disabled,
}
```
Exact match to documentation.

### 3. Templates (template.rs:226-545)
Built-in templates confirmed: `executive`, `technical`, `developer`, `compliance`.
Compliance templates confirmed for: PCI-DSS, SOC2, HIPAA, GDPR, OWASP, NIST.

### 4. FxHashMap Usage
All locations documented use FxHashMap correctly:
- `dedup.rs:27` - `DedupEngine::seen`
- `agent.rs:276-279` - `FindingSummary` fields
- `trend.rs:68,73,211,222` - `ResultComparator` and `TrendAnalyzer` methods
- `diff.rs:39-49` - `DiffEngine::compare`
- `baseline.rs:13-14` - `BaselineComparison::compare`
- `session.rs:11-12,18` - `ScanSession` and `TabSessionState`
- `template.rs:18,73` - `ReportTemplateEngine` and `TemplateRenderContext`
- `attack_graph.rs:20,64,80` - `GraphNode::properties`
- `sarif.rs:74` - `SarifResult::properties`
- `junit.rs:83` - `JUnitBuilder::test_suites`

### 5. XXE Safety
- SARIF (sarif.rs:1-8): Confirmed uses `serde_json` only, no XML parsing
- JUnit (junit.rs:1-9): Confirmed uses `quick_xml::Writer` in write-only mode

### 6. CSV Formula Injection Protection (escape.rs:16-35)
`escape_csv()` uses NFKC normalization and quoting - exactly as documented.

### 7. Error Handling
Correct `Result` types used in:
- `csv.rs:9,33,57` - `Result<String, std::fmt::Error>`
- `junit.rs:316` - `Result<String, quick_xml::Error>`
- `attack_graph.rs:135` - `Result<String, serde_json::Error>`

## Discrepancies

### 1. Documentation Lists Wrong FxHashMap Location for `ResultComparator` (Low)
**Doc says**: `trend.rs` - `ResultComparator`, `TrendAnalyzer`

**Reality**: `ResultComparator` (trend.rs:52) is a unit struct with no fields. The `FxHashMap` usage is in its `compare()` method (lines 68, 73), not in the struct definition. The struct itself doesn't use FxHashMap.

**Fix**: Update doc to clarify FxHashMap usage is in the `compare()` method, not the struct.

### 2. TrendDirection Not Listed as Key Type (Low)
**Doc lists** `TrendAnalysis` (trend.rs) but not `TrendDirection` (trend.rs:136-140). The enum exists and is a key part of trend analysis.

**Fix**: Add `TrendDirection` to the Key Types table.

## Bugs Found

### 1. DiffEngine::has_regressions Only Checks Critical Escalations (Medium)
**Location**: `diff.rs:136-140`
```rust
pub fn has_regressions(diff: &DiffResult) -> bool {
    diff.escalated_findings
        .iter()
        .any(|f| f.severity == Severity::Critical)
}
```

**Issue**: The doc states it "checks Critical escalations" which is accurate, but this is potentially misleading. A finding escalating from Low to Critical is a major regression, but `Severity::as_int()` (used in `DiffEngine::compare` at line 83) means any positive severity change triggers escalation. However, `has_regressions()` only flags Critical.

**Risk**: High severity escalations (e.g., Low→High) are not considered regressions by `has_regressions()`. This could cause missed detection of significant security regressions.

**Recommendation**: Either:
1. Expand to check `Severity::High` as well: `f.severity >= Severity::High`
2. Or document this as intentional behavior

### 2. Template render methods return different error types (Low)
**Doc states**: `MarkdownReport::generate()` returns `Result<String, std::fmt::Error>`

**Reality**: `convert_to_markdown()` (convert.rs:132) returns `Result<String, std::fmt::Error>`. The actual `markdown.rs` may have different error handling. This is a documentation accuracy issue.

**Recommendation**: Verify actual error types in `markdown.rs` and update doc.

## Improvement Opportunities

### 1. Use LazyLock for Compliance Templates (Medium)
**Location**: `template.rs:453-545`

All compliance template functions (`pcidss_template`, `soc2_template`, etc.) recreate the same `ComplianceTemplate` struct on every call. These should be `LazyLock` statics for performance, similar to the pattern used in `waf/bypass/profiles.rs`.

### 2. Missing Division by Zero Guard (Medium)
**Location**: `agent.rs:318`
```rust
(weighted / (self.total.max(1) as f32) * 10.0).min(10.0)
```

Uses `.max(1)` - this is fine but the doc doesn't mention the risk score formula. Consider documenting the risk score algorithm and ensuring consistent handling with `SeverityCounts::risk_score()` (report.rs:70-75) which lacks the guard:
```rust
pub fn risk_score(&self) -> f64 {
    (self.critical as f64 * 10.0)
        + (self.high as f64 * 7.0)
        + (self.medium as f64 * 4.0)
        + (self.low as f64 * 1.0)
}
```

### 3. ResultComparator Uses Clone on Large Findings (Low)
**Location**: `trend.rs:68-77`

The `compare()` method clones entire `Finding` objects into `FxHashMap`:
```rust
let old_findings: FxHashMap<_, _> = old
    .details
    .iter()
    .map(|f| (Self::finding_key(f), f.clone()))
    .collect();
```

For large scan results with many findings, this could be memory intensive. Consider using references or a more memory-efficient approach.

### 4. DiffEngine Could Reuse Collections (Low)
**Location**: `diff.rs:38-134`

Each call to `DiffEngine::compare` allocates multiple new collections (`new_findings`, `resolved_findings`, `escalated_findings`, etc.). For high-frequency diff operations, consider object pooling or reusing allocations.

### 5. ScanSession::save could use atomic write (Low)
**Location**: `session.rs:38-46`

`save()` writes directly to the target file. If the process crashes mid-write, the session file could be corrupted. Consider write-to-temp-then-rename pattern for durability.

## Priority Summary

| Priority | Finding | Location |
|----------|---------|----------|
| Medium | `has_regressions` only checks Critical | diff.rs:136 |
| Medium | LazyLock for compliance templates | template.rs:453-545 |
| Low | TrendDirection missing from Key Types | trend.rs:136 |
| Low | Documentation accuracy on error types | convert.rs:132 |
| Low | `ResultComparator` FxHashMap clarification | trend.rs:52 |
| Low | Clone overhead in trend comparison | trend.rs:68-77 |
| Low | Division by zero guard consistency | report.rs:70-75 |
| Low | Atomic write for session persistence | session.rs:38-46 |

## Overall Assessment

The architecture document is **highly accurate** - most claims were verified against implementation. The main issues are:

1. **Documentation gaps**: Some FxHashMap locations slightly imprecisely described
2. **Design concerns**: `has_regressions()` may be too narrow in scope
3. **Performance**: Compliance templates could benefit from caching

The module is well-structured with proper error handling, security considerations (XXE, CSV injection), and performance optimizations (FxHashMap throughout).