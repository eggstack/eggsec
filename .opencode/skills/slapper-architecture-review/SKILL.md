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

### HashMap/HashSet (P1 Priority)
- `cli/report.rs:44-57`
- `fuzzer/targets/api.rs` (multiple lines)
- `slapper-nse/vulns.rs`, `rpc.rs`, `smbauth.rs`, `public_api/api.rs`, `creds.rs`

### unwrap()/expect() in Production
- `planner.rs:208,469,482` - SystemTime unwrap()
- `chain.rs:381` - LazyLock regex unwrap()

### Documentation Discrepancies
- `config.md` - ScanConfig.profiles reference location
- `waf/mod.rs` - Lists 25 WAF products instead of 34
- `tui.md` - Key binding for toggle_bookmark is `Ctrl+b`, not `b`