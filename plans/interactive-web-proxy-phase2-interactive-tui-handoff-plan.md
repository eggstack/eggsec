# Interactive Web Proxy / Traffic Interception Loadout - Phase 2 Interactive TUI & Core Manipulation Handoff Plan

**Date**: 2026-06-12  
**Status**: Ready for Execution After Phase 1  
**Phase**: 2 — Interactive TUI + Core Manipulation & Editing  
**Parent Documents**:
- `plans/interactive-web-proxy-loadout-design-plan.md` (authoritative detailed design)
- `plans/interactive-web-proxy-implementation-roadmap.md` (high-level sequencing)
- `plans/interactive-web-proxy-phase1-foundation-handoff-plan.md` (Phase 1 foundation)
**Precedent**: `plans/database-pentesting-phase2-mssql-and-polish-handoff-plan.md`, mobile-dynamic phase handoff plans  
**Target Branch**: `feature/interactive-web-proxy-loadout`  
**Authoring Note**: This document provides the detailed, actionable task breakdown for Phase 2. It assumes Phase 1 (core MITM server + logging + CLI + dry-run + policy + bridge) is complete. Follow the exact execution pattern of previous loadouts.

---

## 1. Phase 2 Executive Summary & Scope

**Goal**: Deliver the rich **interactive manual experience** — a first-class TUI for live traffic flow inspection, breakpoint/intercept rules, header & body editing, forward/drop/replay actions, and full manipulation audit trails. This turns the Phase 1 passive capture proxy into a powerful manual adversarial simulation and defense regression tool.

**In Scope for Phase 2**:
- TUI TabSpec implementation ("Proxy" or "Intercept" tab)
- Live flow list + detail views (table + pretty/hex/raw panes)
- Edit modal for headers and body (with validation + before/after diff preview)
- Core actions: forward, drop, replay, pause/resume individual flows or global
- Basic intercept/breakpoint rule engine (host, path, method, header, body patterns)
- Full manipulation recording (`ManipulationRecord` with before/after + reason)
- Session management (save/load, export to HAR/JSON)
- Enhanced findings generation from interactive sessions
- Integration with existing policy/EnforcementContext (already wired in Phase 1)
- Polish: preflight indicators, enforcement posture, graceful shutdown, small-terminal layouts
- Updated tests, docs, and examples for interactive workflows

**Out of Scope for Phase 2** (deferred to Phase 3+)
- WebSocket / HTTP/2 / gRPC protocol support
- Advanced persistent rule engine or scripting
- Deep correlation / evidence bundles with other loadouts
- Pipeline profile integration beyond basic awareness
- Transparent proxy mode
- MCP/agent exposure for the interactive surface

**Success Vision**: After Phase 2, a user can launch `eggsec proxy intercept --interactive`, configure a lab target, intercept live traffic in a rich TUI, manually edit headers/body, forward or drop flows, and generate a complete `WebProxySessionReport` with full manipulation audit trail — all while staying inside Eggsec’s safety model.

---

## 2. Key Decisions Confirmed for Phase 2

- **TUI Tab Name**: "Proxy" (primary) or "Intercept" — final decision to be made early in Phase 2 with TUI team.
- **Edit Capabilities**: Header map editor + body textarea (text/JSON/XML pretty-print + hex fallback). Binary/large payloads get hex view + download in Phase 2; advanced patching deferred.
- **Rule Engine Scope**: Simple in-memory rule matching (host/path/method/header/body contains/regex). Persistent rules and complex actions deferred to Phase 3.
- **Manipulation Audit**: Immutable `ManipulationRecord` list stored in `ProxyFlow`. Always included in report and TUI history.
- **Session Persistence**: JSON-based save/load for sessions. HAR export supported.
- **Policy Integration**: Reuse Phase 1 `EnforcementContext` wiring. Interactive mode still respects `--allow-web-proxy` and provenance.
- **Standalone Defense-Lab Surface**: Interactive TUI remains part of the standalone loadout (no MCP exposure in Phase 2).

---

## 3. Detailed Deliverables & Task Breakdown

### 3.1 TUI Architecture & Tab Implementation
1. Create new `TabSpec` for Proxy/Intercept in `eggsec-tui` (following recent 10-phase architecture patterns).
2. Implement flow list table view (columns: ID, Time, Method, Host/Path, Status, Size, Modified, Actions).
3. Implement detail pane (split or tabbed): Headers (pretty map), Body (pretty-print / hex / raw toggle), Manipulation history.
4. Wire global task strip, enforcement posture indicators, and preflight checks (reuse existing TUI components).
5. Implement small-terminal degraded layout and "too small" fallback.

### 3.2 Edit Modal & Manipulation Engine
6. Build edit modal for headers (add/modify/delete with validation).
7. Build body editor (textarea with basic syntax awareness for JSON/XML + hex view for binary).
8. Implement before/after diff preview before applying changes.
9. Create `ManipulationRecord` struct and append logic (field, before, after, reason, timestamp).
10. Wire edit actions to update `ProxyFlow` in real time.

### 3.3 Intercept / Breakpoint Rules
11. Implement simple rule matching engine (host contains, path regex, method, header value, body contains).
12. Add UI for creating/editing/deleting rules in the TUI.
13. Integrate rules with flow pause/resume logic.
14. Record rule-triggered intercepts in the manipulation/audit trail.

### 3.4 Core Actions (Forward / Drop / Replay / Pause)
15. Implement per-flow actions: Forward (modified or original), Drop, Replay original.
16. Implement global actions: Pause all, Resume all, Forward all pending.
17. Add confirmation overlays for destructive actions (drop/replay) per policy model.
18. Ensure all actions update the `ProxyFlow` state and generate proper `ManipulationRecord` entries.

### 3.5 Session Management & Export
19. Implement save/load session (JSON format with full flows + manipulations).
20. Add HAR export capability for flows.
21. Support baseline vs current diff for regression workflows (simple comparison view).
22. Add session metadata (start time, target, CA fingerprint, budgets used).

### 3.6 Enhanced Findings & Reporting
23. Generate interactive-specific findings (e.g., "Manual JWT modification performed", "Sensitive header edited", "Request replayed").
24. Ensure all manipulations are visible in the final `WebProxySessionReport` and bridged output.
25. Update `to_scan_report_data_proxy()` if needed to include manipulation details.

### 3.7 Polish & UX
26. Add prominent enforcement posture and policy decision indicators in TUI.
27. Implement graceful shutdown (stop server, flush sessions).
28. Improve error handling and user feedback for failed forwards/edits.
29. Add keyboard shortcuts and action palette (consistent with recent TUI pass).
30. Update human output and CLI banners for interactive mode.

### 3.8 Testing
31. Unit tests for rule matching, manipulation recording, edit validation, diff generation.
32. Integration tests for TUI flows (dry-run heavy + mocked server).
33. Lab smoke tests with real interactive sessions against docker targets (httpbin/dvwa).
34. Regression tests ensuring Phase 1 non-interactive mode still works perfectly.

### 3.9 Documentation & Examples
35. Update `docs/WEB_PROXY.md` with Phase 2 interactive workflows, TUI screenshots/keybindings, example manual edit scenarios (JWT tampering, header injection, replay for regression).
36. Add interactive examples and CA + browser setup guide refresh.
37. Update `README.md` quick reference and lab defense commands table.
38. Update architecture docs and AGENTS files.
39. Create example interactive session JSON/HAR artifacts.

---

## 4. Recommended Implementation Order (Lowest Risk First)

1. **TUI Tab skeleton + flow list + detail pane** (visual foundation, reuse existing components)
2. **ManipulationRecord + edit modal + diff preview** (core interactive value)
3. **Forward / Drop / Replay actions** (make the TUI actually useful)
4. **Basic rule / breakpoint engine** (intercept control)
5. **Session save/load + HAR export**
6. **Enhanced findings from manipulations**
7. **Policy / enforcement indicators + polish**
8. **Tests (unit → integration → lab interactive smoke)**
9. **Documentation & examples**

This order delivers a working interactive TUI as early as possible while building on the solid Phase 1 server foundation.

---

## 5. Success Criteria (Measurable)

- TUI tab renders cleanly and is consistent with other tabs (Wireless, DbPentest, etc.).
- User can intercept a live flow, edit headers + body, forward the modified request, and see the change recorded in the report.
- Drop and replay actions work correctly with proper audit trail.
- Basic rule matching pauses flows as expected.
- Sessions can be saved/loaded and exported to HAR.
- All safety gates from Phase 1 continue to function in interactive mode.
- `cargo test --features web-proxy` green (including new TUI-related tests).
- Lab interactive smoke test succeeds (real traffic + manual edits + complete report with manipulations).
- Documentation is accurate and usable for interactive workflows.
- Phase 3 handoff plan draft is ready.

---

## 6. Risks & Mitigations Specific to Phase 2

| Risk                                      | Likelihood | Impact     | Mitigation Strategy                                                                 |
|-------------------------------------------|------------|------------|-------------------------------------------------------------------------------------|
| TUI complexity / architecture misalignment | Medium     | High       | Heavy reuse of recent 10-phase TUI patterns; early coordination with TUI team      |
| Edit validation introducing invalid HTTP   | Medium     | Medium     | Strong client-side validation + diff preview + revert option                       |
| Performance under concurrent live flows    | Medium     | Medium     | Flow buffering + limits from Phase 1; background capture + on-demand detail        |
| Manipulation audit trail incomplete        | Low        | High       | Immutable append-only records; test coverage for every action path                 |
| Scope / policy bypass in interactive mode  | Low        | High       | Reuse Phase 1 `EnforcementContext` wiring; per-flow host checks                    |
| Documentation lag on interactive workflows | Medium     | Medium     | Parallel docs workstream from the start                                            |

---

## 7. Dependencies & Coordination Points

- **TUI team** — critical for TabSpec, components, and architecture alignment
- **Policy / EnforcementContext team** — reuse and minor extensions if needed
- **Output / reporting team** — ensure manipulation data flows correctly through bridge
- **CLI team** — minor updates for `--interactive` flag behavior
- **Testing / DevEx** — interactive lab smoke test infrastructure

Coordinate early with the TUI team on component reuse and TabSpec patterns.

---

## 8. Phase 2 Handoff Checklist (Before Merging to Main)

- [ ] All numbered tasks in Section 3 completed or explicitly deferred
- [ ] TUI tab is functional and polished
- [ ] Edit + forward/drop/replay workflow works end-to-end (dry-run + lab)
- [ ] Manipulation audit trail is complete and visible in reports
- [ ] Rule/breakpoint engine works as designed
- [ ] Session management and export functional
- [ ] All safety gates validated in interactive context
- [ ] Tests green (unit + integration + lab smoke)
- [ ] Documentation updated and reviewed
- [ ] Phase 3 handoff plan draft created
- [ ] Short Phase 2 closeout note added

---

## 9. Next Steps After Phase 2

1. Merge Phase 2 to main (after checklist complete).
2. Create `plans/interactive-web-proxy-phase3-advanced-protocols-handoff-plan.md`.
3. Begin Phase 3 (WebSocket/HTTP/2 + advanced rule engine + correlation).
4. Gather user feedback from Phase 1 + Phase 2 interactive usage.
5. Plan deeper integration with other loadouts (web fuzzing, auth testing, etc.).

---

## 10. References

- Parent Design: `plans/interactive-web-proxy-loadout-design-plan.md`
- Roadmap: `plans/interactive-web-proxy-implementation-roadmap.md`
- Phase 1 Handoff: `plans/interactive-web-proxy-phase1-foundation-handoff-plan.md`
- Precedent: database-pentesting and mobile-dynamic phase handoff plans
- TUI Architecture: Recent 10-phase updates in `crates/eggsec-tui/`
- Core Types: `crates/eggsec/src/proxy/types.rs` (from Phase 1)

---

**End of Phase 2 Interactive TUI & Core Manipulation Handoff Plan**

This document is the execution blueprint for Phase 2. Implement in the recommended order, coordinate closely with the TUI team, and maintain the high safety and quality bar established in Phase 1.

**Phase 1 Closeout Note** (to be filled after Phase 1 completion):

Phase 1 foundation (MITM server, CA, CLI, dry-run, policy, bridge) complete per plan. Ready for Phase 2 interactive layer on top of the solid server foundation.
