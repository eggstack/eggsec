---
name: plan_improvement
description: "Execute codebase improvement plans with wave-based parallelization"
triggers:
  - plan improvement
  - wave-based execution
  - subagent parallelization
  - plan verification
metadata:
  category: agent-ops
  tools: []
  scope: codebase
---

## Overview

This skill guides AI agents through complex multi-wave improvement plans with parallel sub-agent execution.

## Current Status

**Plan is COMPLETED and pruned as of 2026-05-01.**
All waves verified complete. The `plans/plan.md` file now contains verification notes.

## Verification Process

When reviewing plan items or implementing changes:
1. Read `plans/plan.md` for current status
2. Run verification commands to establish baseline: `cargo test --lib -p slapper`
3. Use subagents to verify items in parallel (explore type for research)
4. Always verify claims against actual code, not assuming plan accuracy
5. Commit after each fix for traceability
6. Update plan.md with verification status

## Key Patterns

- Use subagents for parallel work (explore, general types)
- Always verify before claiming DONE
- Commit after each fix
- Update plan.md with completion status
- Test count: 1155 base, 1472 with full features (verified 2026-05-01)

## Verification Results (2026-05-01)

All plan items verified complete:
- **CookieStore (3.3.1)**: reqwest cookies feature enabled in Cargo.toml; manual cookie management in tool/session.rs is intentional for security testing scenarios
- **Regex LRU Cache (4.2)**: chain.rs uses LruCache correctly; filters.rs updated to store compiled Regex directly in PayloadFilter::Regex variant
- **AgentLogger (5.1.1)**: FIXED - was local variable in run(), now stored as Agent struct field `logger: Option<AgentLogger>` for entire lifetime
- **ConfigWatcher (5.1.2)**: Properly wired in agent/mod.rs new() method, stored as field to keep watcher alive

## Common Issues Found During Verification

During the 2026-04-30 review, these items were found incomplete despite plan claims:

| Item | Issue | Fix |
|------|-------|-----|
| CookieStore (3.3.1) | Manual parsing still in session.rs | Enable reqwest cookies feature |
| Regex LRU Cache (4.2) | Unbounded FxHashMap | Use lru crate with 100 entry limit |
| AgentLogger (5.1.1) | Code existed but was local variable in run() | Store as Agent struct field, initialize in run() |
| ConfigWatcher (5.1.2) | Code existed but never called | Wire up in agent new() |

Note: 2026-05-01 verification found these items had subtle bugs that needed fixing (not just missing calls).

## Triggers

Keywords that activate this skill:
- "work through plan"
- "verify plan items"
- "wave-based parallelization"
- "plan execution"
- "subagent assignment"