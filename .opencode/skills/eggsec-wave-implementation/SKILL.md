# Wave Implementation Skill

**Status**: HISTORICAL REFERENCE ONLY - All waves completed 2026-06-02

This skill documents the multi-wave implementation pattern used for executing large sets of related fixes across multiple modules. All items are now complete. This file serves as a historical reference for future agents working on similar large-scale tasks.

## Overview

The "Wave" pattern organized implementation by priority:
- **Wave 0**: Critical Bug Fixes (routing.rs panic, spoof_ip rename, unwrap_or clarity)
- **Wave 1A-1E**: Architecture Documentation (counts, structure, AI/MCP, recon, stub modules)
- **Wave 2**: Agent/MCP Profile Productionization (12 phases, mostly pre-existing)
  - Phase 7: CodingAgentFindingReport typed struct (new)
- **Wave 3**: Output Module Documentation

All items have been verified and completed.

## Key Implementation Patterns (Historical)

The following patterns were established during the wave implementation:

### Clock Skew Panic Prevention
```rust
// BAD - can panic on NTP correction
.last_used = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .unwrap()
    .as_secs()

// GOOD - graceful fallback
.last_used = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .unwrap_or_else(|_| Duration::from_secs(0))
    .as_secs()
```

### LazyLock Regex Initialization
```rust
// BAD - unwrap can panic at startup
static EMAIL_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"pattern").unwrap());

// GOOD - expect provides descriptive message
static EMAIL_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"pattern").expect("VALID_REGEX: email pattern"));
```

### Error Propagation Instead of Silent Suppression
```rust
// BAD - silently loses data
fn write_packet(&mut self, data: &[u8]) -> io::Result<()> {
    let ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_else(|e| {
        tracing::warn!("Failed to get system time: {}", e);
        return Ok(());  // Silent suppression
    });
    // ...
}

// GOOD - propagates error to caller
fn write_packet(&mut self, data: &[u8]) -> io::Result<()> {
    let ts = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e| {
        tracing::warn!("Failed to get system time: {}", e);
        e
    })?;
    // ...
}
```

### Arc::try_unwrap with Graceful Fallback
```rust
// BAD - expect can panic
let findings = Arc::try_unwrap(findings)
    .expect("Arc should have single owner")
    .into_inner();

// GOOD - map_err provides context
let findings = match Arc::try_unwrap(findings) {
    Ok(inner) => inner.into_inner(),
    Err(e) => {
        tracing::warn!("Callback still referenced, using empty result: {}", e);
        Vec::new()
    }
};
```

### FxHashMap/FxHashSet for Performance
```rust
// Use rustc_hash for performance-critical collections
use rustc_hash::FxHashMap;
use rustc_hash::FxHashSet;

// Instead of std::collections::HashMap/HashSet
```

## Common Pitfalls

1. **Branch collision**: Multiple subagents working on the same branch. Solution: Each subagent must have its own branch.

2. **Merge order issues**: Pushing when local is behind. Solution: Always `git fetch origin main && git reset --hard origin/main` before merging multiple branches.

3. **Compilation not verified**: Pushing branches without verifying compilation. Solution: Always run `cargo check --lib -p <package>` before pushing.

4. **Plan not updated**: Forgetting to mark items as completed. Solution: Update plan.md before marking wave complete.

## Historical Pitfalls (Lessons Learned)

These pitfalls were encountered during the wave implementation and are preserved as guidance for future agents:

1. **Branch collision**: Multiple subagents working on the same branch. Solution: Each subagent must have its own branch.

2. **Merge order issues**: Pushing when local is behind. Solution: Always `git fetch origin main && git reset --hard origin/main` before merging multiple branches.

3. **Compilation not verified**: Pushing branches without verifying compilation. Solution: Always run `cargo check --lib -p <package>` before pushing.

4. **Plan not updated**: Forgetting to mark items as completed. Solution: Update plan.md before marking wave complete.

5. **Stale assertions**: Plans may contain claims that no longer match reality. Solution: Always verify against actual source code before implementing.

6. **Count verification**: Always verify statistical claims (file counts, enum variants, match arms) against actual source.

## Resources

- `AGENTS.md` - General guidelines for all agents
- `AGENTS.override.md` - Module-specific guidance (in each module directory)