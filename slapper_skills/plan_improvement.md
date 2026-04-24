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

## Usage

When the user asks to work through a plan file:
1. Read the plan.md file to understand wave structure
2. Run verification commands to establish baseline
3. Verify each item status as complete or needs work
4. Execute fixes in order (Waves 1-2 first as foundational)
5. Commit after each wave for traceability

## Wave Execution Order

```
Wave 1-2 (Security) ──► Foundation (start first)
     │
     ├── Wave 5, 6, 8 (Independent - can parallelize)
     │
     ├── Wave 3-4 (Refactoring, Performance)
     │
     └── Wave 7 (Dependency Updates - highest risk, LAST)
```

## Key Patterns

- Use subagents for parallel work (explore, general types)
- Always verify before claiming DONE
- Commit after each wave
- Update plan.md with completion status

## Triggers

Keywords that activate this skill:
- "work through all remaining waves"
- "wave-based parallelization"
- "plan execution"
- "subagent assignment"