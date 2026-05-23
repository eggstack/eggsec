## Overview

This skill guides AI agents through complex multi-wave improvement plans with parallel sub-agent execution.

## Plan Status

**Plan is COMPLETED and PRUNED as of 2026-05-29.**
The `plans/plan.md` file has been pruned to only contain:
- Completion summary (20/20 items completed)
- Future considerations (lower priority items for later planning)
- Historical reference for completed items

A previous plan was completed on 2026-05-23 (47 items across 3 waves).

## Verification Process

When reviewing plan items or implementing changes:
1. Read `plans/plan.md` for current status and future considerations
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
- Test count: 1324 base, 1469+ with full features (verified 2026-05-29)

## Verification Results (2026-05-29)

All 20 implementation items verified complete:
- **Wave 1 (6 items)**: Production safety fixes - all completed
- **Wave 2 (8 items)**: Error handling improvements - all completed
- **Wave 3 (6 items)**: Implementation items - all completed (4 deferred items pruned)

Deferred items removed from plan:
- TUI unwrap_or_default: Low value, high refactoring risk, async-safe
- NSE DNS rebinding: Architecture already validates IPs at connection time
- NSE OSV/CISA KEV: Already fully implemented

## Plan Pruning Pattern

When completing a plan implementation:
1. Archive detailed line-by-line items to a summary
2. Keep only future/deferred items requiring further work
3. Update AGENTS.md to reference current plan status
4. Update skills to reference current completed plan

## Triggers

Keywords that activate this skill:
- "work through plan"
- "verify plan items"
- "wave-based parallelization"
- "plan execution"
- "subagent assignment"
- "prune completed items"