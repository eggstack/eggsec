# Detailed Roadmap: Enhancing Advanced Evasion, Stealth, and Red Team Operations in Eggsec

**Status**: Draft - Phase 1 Planning

## Executive Summary

This roadmap extends Eggsec’s core strengths in scoped, repeatable security assessment into sophisticated adversary emulation for red teaming and defense validation. It maintains strict adherence to project conventions:
- Explicit scope enforcement for all operations.
- Feature-gated modules (`evasion`, `postex`, `c2`).
- Differentiated workflows: Policy-checked MCP/agentic vs. human-confirmed CLI/TUI.
- Dry-run, audit, baseline, and regression support.
- MITRE ATT&CK mapping.

## Core Principles
- Safety first: All red-team features require scopes, timeboxes, and explicit authorization.
- Modularity: Cargo features and crates.
- Repeatability: Profiles, baselines, SARIF outputs.

## Phase 1: Foundations & Evasion Primitives (1-2 Months)

1. **Evasion Core** (`crates/evasion/` or feature flag)
   - Direct syscalls, ETW/AMSI bypass, unhooking.
   - Obfuscation primitives.
   - Process injection (memory-only).

2. **Traffic Stealth Basics**.

3. **Lab Testing Setup** (EDR Docker environments).

**Deliverables**: Skeleton module, docs, tests.

## Phase 2: Post-Exploitation & LOTL (2-4 Months)

... (full detailed content follows the previous plan we discussed)

## Implementation Notes
Follow existing patterns from database-pentesting and web-proxy plans.

See full details in conversation history or expand sections as needed.