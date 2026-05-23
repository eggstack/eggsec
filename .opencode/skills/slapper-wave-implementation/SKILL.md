# Wave Implementation Skill

Guidelines for executing multi-wave implementation plans in this codebase.

## Overview

This codebase uses a "Wave" pattern for implementing large sets of related fixes across multiple modules. Waves are organized by priority:
- **Wave 1**: Production safety and critical performance fixes
- **Wave 2**: Error handling improvements within specific modules
- **Wave 3**: Cleanup, documentation, and optional enhancements

## Wave Execution Pattern

### Pre-Execution Checklist
1. Read the full plan (`plans/plan.md`)
2. Identify items that can be executed in parallel
3. Create separate git branches for each parallel work item (branch naming: `fix/<module>-<issue>`)
4. Ensure each subagent works on a separate branch to avoid merge conflicts

### Branch Naming Convention
```bash
fix/<module>-<specific-issue>
# Examples:
fix/ai-planner-clock-skew
fix/tool-planner-fxhashset
fix/fuzzer-api-fxhashmap
fix/nse-smbauth-duplicates
fix/recon-regex-expect
fix/networking-capture-error-propagation
```

### Execution Steps
1. Launch subagents for parallel Wave 1 items
2. Verify each branch compiles (`cargo check --lib -p <package>`)
3. Push branches to origin
4. Fetch and merge branches to main sequentially (avoid race conditions)
5. Verify compilation after each merge
6. Repeat for Wave 2 and Wave 3

### Merge Strategy
When merging multiple branches:
1. Always fetch origin/main before merging
2. Use `--no-edit` for automatic merge commits
3. If conflicts occur, resolve by keeping the cleaner version (usually the incoming branch)
4. Push after each successful merge to avoid stale state

### Post-Wave Checklist
1. Update `plans/plan.md` with completion status and commit hashes
2. Update `AGENTS.md` with bug fix summary
3. Update relevant `AGENTS.override.md` files if module-specific guidance changed
4. Update skills in `.opencode/skills/` if new patterns need documenting

## Plan Completion (2026-05-29)

As of 2026-05-29, all items across 3 waves have been verified and implemented (20/24 implemented, 4 deferred). The plan file has been pruned to a summary.

### Implementation Summary (2026-05-29)
- **Wave 1**: All 6 items completed (PluginManager FxHashMap, Ruby timeout, CMS error handling, CacheKeyBuilder, AI HashMap, NSE CVE Vec storage)
- **Wave 2**: All 8 items completed (WAF HTTP/2 docs, Scope.validate(), Scanner progress, Distributed TaskResult, heartbeat cached, Pipeline errors, fingerprint ports, WAF config)
- **Wave 3**: 6/10 completed (TUI dispatcher cache, CLI output flag, docs module counts, fuzzer progress, CSV streaming, TUI theme restore, recon secrets), 4 deferred (TUI unwrap_or_default, NSE DNS rebinding, NSE OSV/CISA KEV)

If starting a new wave of fixes:

1. Create a new plan document in `plans/` directory (e.g., `plans/plan-YYYY-MM-DD.md`)
2. Use the wave-based parallelization approach demonstrated in this skill
3. Mark items as COMPLETED only after verifying in code with subagents
4. Use `cargo test --lib -p slapper` to verify changes don't break existing functionality

## Key Implementation Patterns

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

## Resources

- `plans/plan.md` - Current implementation plan (all items completed as of 2026-05-29)
- `AGENTS.md` - General guidelines for all agents
- `AGENTS.override.md` - Module-specific guidance (in each module directory)

## Architecture Review Wave Pattern

When executing architecture reviews from `architecture/review_plan.md`:

1. **Review Phase**: Run subagents to review each module's architecture document and produce `plans/{module}_review.md`

2. **Consolidation Phase**: Aggregate all findings into waves by priority:
   - Wave 1: Production-critical bugs (high severity, affects correctness)
   - Wave 2: High-priority issues (medium severity, known bugs)
   - Wave 3: Medium-priority improvements (performance, code quality)
   - Wave 4: Documentation and low-priority fixes

3. **Implementation Phase**: Execute waves sequentially:
   - Use subagents for independent fixes
   - Each subagent on its own branch
   - Verify with `cargo check --lib -p slapper` after each fix
   - Commit wave together after verification

4. **Documentation Phase**: Update files:
   - Update `AGENTS.md` with new bug fixes in Recent Bug Fixes section
   - Update relevant module `AGENTS.override.md` if needed
   - Update `architecture/review_plan.md` to mark completed waves

### Key Verification Commands
```bash
cargo check --lib -p slapper
cargo check -p slapper-nse
cargo test --lib -p slapper
cargo clippy --lib -p slapper
```