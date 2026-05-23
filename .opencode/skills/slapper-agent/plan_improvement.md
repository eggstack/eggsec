## Overview

This skill guides AI agents through complex multi-wave improvement plans with parallel sub-agent execution.

## Plan Status

**Plan is COMPLETED and PRUNED as of 2026-05-23.**
The `plans/plan.md` file has been archived and pruned to only contain:
- Completion summary
- Future/deferred items still pending
- Historical reference

## Verification Process

When reviewing plan items or implementing changes:
1. Read `plans/plan.md` for historical reference
2. Run verification commands to establish baseline: `cargo test --lib -p slapper`
3. Use subagents to verify items in parallel (explore type for research)
4. Always verify claims against actual code, not assuming plan accuracy
5. Commit after each fix for traceability
6. Update plan.md with verification status (only for new items)

## Key Patterns

- Use subagents for parallel work (explore, general types)
- Always verify before claiming DONE
- Commit after each fix
- Update plan.md with completion status (only if not fully pruned)
- Test count: 1324 base, 1469+ with full features (verified 2026-05-23)

## Verification Results (2026-05-23)

All 47 original plan items verified complete and pruned from plan file:
- **Wave 1 (6 items)**: Production safety fixes
- **Wave 2 (13 items)**: Error handling improvements
- **Wave 3 (28 items)**: Cleanup and documentation

Remaining deferred items (require design work):
- `loadtest/runner.rs` - Per-worker metrics aggregation
- `pipeline/session.rs` - Async file I/O conversion (acceptable as-is)
- `networking/parse_impl.rs` - `from_utf8_lossy` optimization (may not apply)
- `ai/cache.rs` - CacheKeyBuilder separator collision (informational only)

## Plan Pruning Pattern

When completing a plan implementation:
1. Archive detailed line-by-line items to a summary
2. Keep only future/deferred items requiring further work
3. Update AGENTS.md to remove plan reference section
4. Update skills to reference archived plan (not active)

## Triggers

Keywords that activate this skill:
- "work through plan"
- "verify plan items"
- "wave-based parallelization"
- "plan execution"
- "subagent assignment"
- "prune completed items"