# Detailed Roadmap: Enhancing Advanced Evasion, Stealth, and Red Team Operations in Eggsec

**Status**: Phase 3 Complete (2026-06-14)

## Executive Summary

This roadmap extends Eggsec's core strengths in scoped, repeatable security assessment into sophisticated adversary emulation for red teaming and defense validation. It maintains strict adherence to project conventions:
- Explicit scope enforcement for all operations.
- Feature-gated modules (`evasion`, `postex`, `c2`).
- Differentiated workflows: Policy-checked MCP/agentic vs. human-confirmed CLI/TUI.
- Dry-run, audit, baseline, and regression support.
- MITRE ATT&CK mapping.

## Implementation Status

| Phase | Status | Notes |
|-------|--------|-------|
| Phase 1: Evasion Primitives | ✅ Complete | 16 techniques, MITRE mapped, standalone defense-lab |
| Phase 2: Post-Exploitation & LOTL | ✅ Complete | 16 techniques, 4 categories, reversible actions |
| Phase 3: Full C2 & Agentic Red Teaming | ✅ Complete | Core C2 framework with agent lifecycle, postex integration, attack graph, timeline |

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

**Deliverables**: Skeleton module, docs, tests. ✅ Complete

## Phase 2: Post-Exploitation & LOTL (2-4 Months)

... (full detailed content follows the previous plan we discussed)

## Phase 3: Full C2 & Agentic Red Teaming

- **Core framework**: `C2Scanner`, `C2Report`, `C2Campaign`, beacon protocol simulation, task queue simulation, OPSEC scoring
- **Agent lifecycle**: Registration, check-in, task dispatch, self-destruct, state machine
- **Postex integration**: LOTL, lateral movement, credential access, and persistence techniques mapped to C2 tasks
- **Attack graph**: Campaign dependency graph with critical path analysis
- **Timeline**: Sequential event timeline with phase progression
- **Campaign profiles**: APT29 (Cozy Bear), Carbanak/FIN7, generic default
- **Reporting bridge**: `to_scan_report_data()` auto-detected in `report convert`
- **Policy**: `C2Operation` risk tier; `allow_cateral` flag; dry-run default; `--allow-c2` required for real ops

## Implementation Notes
Follow existing patterns from database-pentesting and web-proxy plans.

See full details in conversation history or expand sections as needed.