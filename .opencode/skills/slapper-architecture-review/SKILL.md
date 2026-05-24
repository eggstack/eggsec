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

## Summary

## Verified Correct

## Bugs Found
| Priority | Issue | Location |
|----------|-------|----------|

## Recommended Fixes

## Discrepancies
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