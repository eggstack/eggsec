# Architecture Review Skill

Guide for reviewing architecture documents against actual implementation.

## When to Use

Use this skill when:
- Reviewing an architecture document (`architecture/*.md`)
- Verifying implementation matches documented design
- Identifying bugs, performance issues, and discrepancies

## Review Methodology

For each module, follow this checklist:

### 1. Read Architecture Document
- Understand the intended design and key claims
- Note specific functionality, patterns, and behaviors described

### 2. Verify Against Code
- Locate the implementation in `crates/slapper/src/<module>/`
- For NSE: `slapper-nse/src/`
- Check if implementation matches documented claims

### 3. Check for Bugs
- Look for `unwrap()`/`expect()` calls that could panic
- Check `HashMap`/`HashSet` instead of `FxHashMap`/`FxHashSet`
- Look for `unwrap_or_default()` silently suppressing errors
- Check for race conditions or concurrency issues

### 4. Check for Performance
- Verify `rustc_hash::FxHashMap` and `FxHashSet` usage
- Check for lock contention on shared metrics
- Look for unnecessary allocations

### 5. Check Patterns
- Verify traits and abstractions are properly implemented
- Check error handling patterns (Result vs panic)
- Verify feature gating is correctly applied

### 6. Document Findings
Write to `plans/<module>_review.md`:
- Summary of what's implemented correctly
- List of bugs/issues with file:line references
- Recommended fixes
- Any discrepancies between arch and impl

## Key Patterns to Verify

### Division by Zero Guard
```rust
// Always check before division
if self.stages.is_empty() {
    return 0.0;
}
```

### Scroll Offset Bounds
```rust
// Check empty before calculating offset
if self.lines.is_empty() {
    return 0;
}
```

### Arc::try_unwrap Error Handling
```rust
// Use map_err instead of expect()
Arc::try_unwrap(arc).map_err(|_| MyError::TooManyOwners)?
```

### LazyLock Regex Initialization
```rust
// Use unwrap_or_else for descriptive panic
static REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(pattern).unwrap_or_else(|e| panic!("Invalid regex: {}", e))
});
```

### Error Handling Pattern
```rust
// Instead of unwrap_or_default()
let body = match response.text().await {
    Ok(text) => text,
    Err(e) => {
        tracing::debug!("Failed to read response body: {}", e);
        String::new()
    }
};
```

## Review Output Format

```markdown
# <Module> Architecture Review

**Document:** architecture/<module>.md
**Review Date:** YYYY-MM-DD
**Implementation Path:** crates/slapper/src/<module>/

## Summary Statistics

| Metric | Count |
|--------|-------|
| Verified Claims | N |
| Discrepancies | N |
| Bugs Found | N |
| Improvement Opportunities | N |

## Verified Claims
- [claim] — Verified in file:line

## Discrepancies
- [issue] — Documented as X, implementation is Y

## Bugs Found
1. **[HIGH/MEDIUM/LOW]** [title]
   - File: [path:line]
   - Description: [what's wrong]
   - Fix: [suggested approach]

## Improvement Opportunities
1. **[HIGH/MEDIUM/LOW]** [title]
   - Current: [description]
   - Suggested: [description]
   - Impact: [performance/correctness/maintainability]
```

## Branch Naming
Create branches like `architecture/<module>-review` for each review.

## Commit Message Format
```
docs: review <module>.md architecture
```

## Known Issues from Past Reviews

### HashMap/HashSet (All Fixed as of 2026-05-24)
All `std::collections::HashMap`/`HashSet` instances in the NSE and slapper crates have been replaced with `FxHashMap`/`FxHashSet` for performance:
- `slapper-nse/public_api/api.rs` - 8 HashMap instances replaced ✅
- `slapper-nse/libraries/http.rs:143` - HashMap replaced ✅
- `slapper-nse/libraries/datafiles.rs:31-33` - HashMap replaced ✅
- `slapper-nse/libraries/creds.rs:102,123` - HashSet replaced ✅

### unwrap_or_default() Issues (All Fixed as of 2026-05-24)
- `ai/waf_bypass.rs:44` - Now uses explicit match with tracing.warn ✅
- `recon/` - 20 instances replaced with explicit match ✅

### Bounds Check Issues (Fixed)
- `packet/parse_impl.rs:531,551` - DNS parsing now has bounds check ✅

### Documentation Discrepancies (All Fixed)
- `recon/recon.md` - secrets module documented, FxHashMap count updated to 55 ✅
- `architecture/tui.md` - payload count updated to 31 ✅

### Pre-existing Compilation Issues (Fixed 2026-05-24)
- `tool/planner.rs` - FxFxHashSet → FxHashSet, use default() not new() ✅
- `tool/implementations/pipeline.rs` - Arc import added, Display fix for Arc<Mutex> ✅
- `recon/mod.rs` - Removed unused FxHashMap import, use std HashMap for Finding.metadata ✅

### Previously Fixed (Verify if Regressions)
- `waf/mod.rs` - Correctly lists 34 WAF products (fixed 2026-05-24) ✅
- `scanner/` - All bug fixes verified applied ✅

### Review Cycle 2026-05-31 Findings

#### Phase 1: Full Architecture Review (34 documents)
- All 34 architecture documents reviewed against implementation
- 7 subagents deployed in parallel, each handling 3-11 documents
- Findings in `plans/review_*.md` (34 files), consolidated in `plans/review_consolidated.md`

#### Key Patterns Discovered
- **Type location drift**: Many documented type locations are wrong (e.g., ScanResults at scanner/mod.rs → actual waf/types.rs)
- **Feature gate gaps**: Multiple docs claim features are "fully implemented" without noting feature gates (websocket, advanced-hunting)
- **Aspirational claims**: Some documented features don't exist (wireless handshake capture, diff_findings_from_files)
- **Stub implementations**: storage/postgres.rs is entirely stub — all CRUD methods return empty values
- **Dead code**: auth/multi_protocol.rs and submodules are unreachable (never declared in mod.rs)

#### Output File Convention
Use `plans/review_<module>.md` (not `plans/<module>_review.md`) for consistency with the full review set.

### Review Cycle 2026-05-31 (17 documents)

#### Phase 1: Document Reviews (17 modules)
- All 17 architecture documents reviewed against implementation
- Findings in `plans/*_review.md`

#### Accuracy Summary
| Document | Accuracy | Key Issues |
|----------|----------|------------|
| ai_agents.md | Medium | McpProfilePolicy underspecified (7/18 fields), TargetPolicy::None doesn't exist |
| cli_commands.md | Medium | cluster.rs:349 fix not applied, wrong line numbers |
| config.md | High | Minor line drift, missing sub-configs |
| output.md | High | has_regressions() check broader than documented |
| pipeline.md | High | Defense-lab profiles stale (implemented, doc says planned) |
| fuzzer.md | High | Minor naming mismatches in magic numbers |
| scanner.md | High | Endpoint count 261 vs documented 224 |
| waf.md | High | Payload count drift (XSS 17 vs 18, SSRF 16 vs 15) |
| recon.md | Medium | Parallel tasks 13 vs 14, ASN lookup detached |
| networking.md | High | BPF-style filters claim misleading |
| loadtest.md | Medium | run_cli() signature wrong (async, config param) |
| distributed.md | High | Line number ranges drifted 10-150 lines |
| nse_integration.md | High | Bug fix history is documentation debt |
| tui.md | High | ~700 lines session fix history should be changelog |
| overview.md | Medium-High | Source files 522 not 526, features 28 not 30 |
| defense_lab.md | High | probe_intents field type wrong |
| feature_matrix.md | Medium-High | api-schema missing from feature table |

#### Critical Issues Found
1. **overview.md**: "Feature flags: 30" wrong — should be 28
2. **overview.md**: "526 source files" wrong — actual 522
3. **feature_matrix.md**: `api-schema` feature undeclared in table
4. **tui.md**: ~700 lines session fix history should be extracted
5. **pipeline.md:90**: Defense-lab profiles "planned" but fully implemented

#### Stale Items
See `plans/stale_items.md` for full list