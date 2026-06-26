# Architecture Review Skill

Guide for reviewing architecture documents against actual implementation.

## When to Use

Use this skill when:
- Reviewing an architecture document (`architecture/*.md`)
- Verifying implementation matches documented design
- Identifying bugs, performance issues, and discrepancies

## Review Methodology

### 1. Read Architecture Document
- Understand the intended design and key claims
- Note specific functionality, patterns, and behaviors described

### 2. Verify Against Code
- Locate the implementation in `crates/eggsec/src/<module>/`
- For NSE: `eggsec-nse/src/`
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
Write to `plans/<module>_review.md` with summary, bugs/issues with file:line references, recommended fixes, and discrepancies.

## Key Patterns to Verify

### Division by Zero Guard
```rust
if self.stages.is_empty() { return 0.0; }
```

### Arc::try_unwrap Error Handling
```rust
Arc::try_unwrap(arc).map_err(|_| MyError::TooManyOwners)?
```

### LazyLock Regex Initialization
```rust
static REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(pattern).unwrap_or_else(|e| panic!("Invalid regex: {}", e))
});
```

### Error Handling Pattern
```rust
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
**Implementation Path:** crates/eggsec/src/<module>/

## Summary Statistics
| Metric | Count |
|--------|-------|
| Verified Claims | N |
| Discrepancies | N |
| Bugs Found | N |

## Verified Claims
- [claim] — Verified in file:line

## Discrepancies
- [issue] — Documented as X, implementation is Y

## Bugs Found
1. **[HIGH/MEDIUM/LOW]** [title]
   - File: [path:line]
   - Description: [what's wrong]
   - Fix: [suggested approach]
```

## Branch Naming
`architecture/<module>-review`

## Commit Message Format
`docs: review <module>.md architecture`

## Resources
- `AGENTS.md` - General guidelines
- `architecture/overview.md` - System-wide architecture and module index
