---
name: plan-improvement
description: "Multi-wave improvement plans with parallel sub-agent execution - use when working with plan execution, wave-based parallelization, or subagent assignment."
---

## Overview

This skill guides AI agents through complex multi-wave improvement plans with parallel sub-agent execution.

## Plan Status

**Plan is FULLY COMPLETE as of 2026-06-02.**
The `plans/plan.md` file contains:
- Completion summary of all 8 waves (0-7) with 100+ items total
- Key module locations reference
- Defense-lab profiles reference
- Probe classification reference
- Non-goals list

All architecture documentation has been verified against the codebase.

## Verification Process

When reviewing plan items or implementing changes:
1. Read `plans/plan.md` for current status and future considerations
2. Run verification commands to establish baseline: `cargo test --lib -p eggsec`
3. Use subagents to verify items in parallel (explore type for research)
4. Always verify claims against actual code, not assuming plan accuracy
5. Commit after each fix for traceability
6. Update plan.md with verification status (only for new items)

## Key Patterns

- Use subagents for parallel work (explore, general types)
- Always verify before claiming DONE
- Commit after each fix
- Update plan.md with completion status (only if not fully pruned)
- Test count: 1324 base, 1469+ with full features

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
