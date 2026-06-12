# Mobile Dynamic Phase 4: Actionable Intelligence Plan

**Date**: 2026-06-12  
**Status**: Phase 4a (Core Correlation Engine + Evidence Foundation) — Implemented 2026-06-12; Phase 4b (TUI + Reporting Polish) — TUI deferred per standalone defense-lab policy; reporting polish delivered 2026-06-12 (human output enhancements only)  
**Theme**: From Powerful Instrumentation to Actionable Intelligence  

**Implementation Note (2026-06-12)**: Phase 4a delivered under single `mobile-dynamic` feature (no sub-feature split, consistent with prior M1/Key Decision). Core deliverable: `CorrelationEngine` + `correlate_reports` + enriched `CorrelatedFinding` (optional score 0-100, `CorrelationType` (Direct/Indirect/Behavioral/CrossLayer), enrichment) + `CorrelationResult` (correlations + timeline + summary) + `build_timeline`. Non-breaking extension of existing `correlate_findings`/`static_correlation`/`CorrelatedFinding`. Timeline derived from report timestamps + ordered actions + Frida start_time + regression notes. Conservative scoring + min_score filter. All dry-run safe, hermetic (no hw), no new deps, serde roundtrips preserved, standalone defense-lab (MCP/agent/TUI/pipeline absent). Baseline/regression/evidence bundles from Phase 3c remain unchanged and integrate cleanly. 6 new unit tests + all prior ~85 mobile-dynamic tests green. See dynamic.rs:216 (CorrelatedFinding), ~229 (new types), ~340 (engine), ~1276 (updated correlate_findings + scoring), tests at end. Docs updated in same pass (MOBILE.md, architecture/mobile.md, AGENTS.md, plan itself). Phase 4b (TUI) and 4c (advanced) deferred per standalone defense-lab policy (no TUI for mobile; see architecture/mobile.md + defense_lab.md). Handoff checklist items for 4a marked complete. Smoke script already covers baseline/bundle paths (correlation is post-run API, exercised in unit tests). No architecture review for TUI needed at this time.

**Phase 4b worked through 2026-06-12**: TUI Foundation / Live Correlation View / Session Management explicitly deferred (zero mobile code in eggsec-tui crate; Tab enum 30 variants, no Mobile; no TaskConfig/Result, no TabSpec, no task wiring, no cfg(feature="mobile*") in TUI source — only feature decl in Cargo.toml; consistent with wireless/auth precedent being fully wired while mobile remains CLI-only per standalone policy). Reporting polish delivered as minimal non-TUI human-output enhancements (under mobile-dynamic, no new deps, additive, preserves all JSON/serde/bridges/tests/contract): `format_dynamic_report` now surfaces `regression_notes=` count in frida line + new "Correlation / Regression:" section (counts of regression notes + static_correlation findings + callout to `correlate_reports`/`CorrelationEngine`/`build_timeline`); `build_dynamic_recommendations` appends regression note bullets when present. 1 new unit test for polish visibility. No `eggsec report diff` command surface exists (ReportCommand only Convert/Trend/Schedule; trend is severity-only; DiffSummary is pipeline-only/coarse); bridged mobile-dynamic-* categories flow to convert; native JSON richer (full carriers + CorrelationResult via lib). Polish makes reports more self-documenting for users who then externally diff or call the engine. Phase 4b implementation note + handoff checklist updated below. All changes non-breaking, dry-run safe, standalone defense-lab (MCP/agent/TUI/pipeline absent). Verification: check/test/clippy/smoke/doc-tests green.  
**Context**: Phase 3 (Frida expansion under `mobile-dynamic`) is well underway with real execution, multiple built-ins, library support, and structured output. Phase 2 is complete. The next logical step is helping users **make sense of** the rich data produced by static + dynamic + Frida capabilities.

---

## 1. Executive Summary

Phase 4 shifts focus from **adding more data sources** to **making the existing data actionable**.

While Phase 3 delivers powerful instrumentation (Frida), Phase 4 delivers the intelligence layer on top — correlation, regression, evidence quality, and usability.

**Primary Goals**:
- Build a unified Correlation & Regression Engine across Static → Dynamic → Frida
- Significantly improve evidence quality and workflow artifacts
- Introduce a TUI / interactive layer for power users
- Enable practical lab workflows (baseline comparison, regression detection, evidence bundles)

**Recommended Sub-Phasing**:
- **Phase 4a**: Core Correlation Engine + Evidence Foundation
- **Phase 4b**: TUI + Reporting Polish
- **Phase 4c**: Advanced Workflows (Regression, Supply Chain, iOS exploration)

**Timeline**:
- Phase 4a: 3–4 weeks
- Phase 4b: 2–3 weeks
- Phase 4c: Future / iterative

---

## 2. Phase 4 Theme & Strategic Rationale

### Why Phase 4 Now?

After Phase 3, users will have:
- Static analysis results
- Dynamic (ADB + proxy + permissions) observations
- Rich Frida instrumentation data (multiple built-ins + custom scripts)

Without strong correlation and usability layers, this creates **data overload** rather than insight. Phase 4 turns raw capability into a **cohesive lab workflow**.

### Alignment with Repo Direction
- Maintains strong defensive-lab focus
- Builds on existing architecture (`DynamicMobileReport`, `correlate_findings`, `FridaInstrumentation`, bridge pattern)
- Keeps everything under `mobile-dynamic`
- Prioritizes high-signal, actionable output over breadth

---

## 3. Phase 4a: Core Correlation Engine + Evidence Foundation

**Goal**: Deliver a practical, unified correlation system and significantly better evidence artifacts.

### Key Deliverables

| # | Deliverable | Description | Priority |
|---|-------------|-------------|----------|
| 1 | Unified Correlation Engine | Extend `correlate_findings` into a full `CorrelationEngine` that handles Static + Dynamic + Frida findings with scoring | High |
| 2 | Behavioral Baselining | Ability to capture and compare against a "golden" dynamic + Frida run | High |
| 3 | Regression Detection | Surface new/changed behavior between baseline and current run | High |
| 4 | Evidence Bundle Export | Export a self-contained bundle (findings + Frida output + traffic + metadata) | High |
| 5 | Structured Finding Enrichment | Automatically enrich findings with correlation context and confidence scores | Medium |
| 6 | Timeline / Sequence View | Basic support for ordering findings across static + dynamic + Frida layers | Medium |

**Success Criteria**:
- Correlation works across all three layers with reasonable accuracy
- Users can save and compare against baselines
- Evidence bundles are usable artifacts for reporting/handover
- New findings clearly show correlation context

---

## 4. Phase 4b: TUI + Reporting Polish

**Goal**: Improve usability for power users and make reporting more professional.

### Key Deliverables

| # | Deliverable | Description | Priority |
|---|-------------|-------------|----------|
| 1 | TUI Foundation | Basic interactive TUI (ratatui or similar) for managing dynamic + Frida runs | High |
| 2 | Live Correlation View | TUI screen showing correlated findings in real time | High |
| 3 | Session Management | Interactive Frida session control (start/stop scripts, view output) | Medium |
| 4 | Report Quality Improvements | Better human-readable formatting, evidence presentation, and summary sections | Medium |
| 5 | `eggsec report diff` Enhancement | Strong support for comparing two dynamic/Frida runs | Medium |

**Success Criteria**:
- Basic TUI allows interactive Frida + dynamic workflows
- Correlation is visible and useful in the TUI
- Report output is noticeably more professional and actionable

---

## 5. Phase 4c: Advanced Workflows

**Goal**: Mature the intelligence layer with advanced use cases.

### Key Deliverables

| # | Deliverable | Description | Priority |
|---|-------------|-------------|----------|
| 1 | Runtime Supply Chain Observation | Use Frida to observe native library loading, dynamic code loading, and dependency behavior | Medium |
| 2 | Advanced Regression Engine | Statistical / ML-light approaches to behavioral diffing (optional, stretch) | Low |
| 3 | Constrained iOS Dynamic | Light Frida support on jailbroken devices or with existing tooling | Low |
| 4 | Evidence Bundle + Report Integration | Deep integration of bundles into SARIF / custom report formats | Medium |
| 5 | Workflow Automation Helpers | Scriptable ways to run baseline → test → compare flows | Medium |

**Success Criteria**:
- Supply chain observation provides new high-signal findings
- iOS support is usable (even if limited)
- Advanced workflows become practical in lab settings

---

## 6. Technical Approach

### Correlation Engine
- Build on the existing `correlate_findings` function
- Create a new `CorrelationEngine` struct that can ingest:
  - `MobileScanReport` (static)
  - `DynamicMobileReport` (dynamic + Frida)
- Use a scoring system (e.g., 0–100 confidence)
- Support different correlation types:
  - Direct (same permission, same method, same endpoint)
  - Indirect (related categories, timing proximity)
  - Behavioral (new behavior vs baseline)

### Evidence Bundles
- Define a clear bundle format (directory or archive)
- Include:
  - Findings (JSON)
  - Frida raw/structured output
  - Traffic capture summary
  - Metadata + timestamps
  - Optional screenshots or logs
- Provide both CLI and library access

### TUI
- Use `ratatui` (or similar) for consistency with potential future wireless TUI work
- Start minimal: one main screen with tabs (Findings, Correlation, Frida Sessions, Actions)
- Keep it optional — CLI remains fully functional

### Safety Model
- Phase 4 is mostly analysis and presentation — lower risk than Frida execution
- Correlation and regression should be safe in dry-run
- TUI should respect the same `--allow-*` and policy gates as the underlying operations

---

## 7. Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Correlation accuracy / false positives | Medium | Start conservative. Provide confidence scores. Allow users to override. |
| TUI scope creep | Medium | Keep initial TUI minimal and focused. Defer advanced features to later iterations. |
| Performance of correlation on large runs | Low | Use efficient data structures. Make correlation optional or configurable. |
| Complexity of regression engine | Medium | Start with simple diffing. Add statistical methods only if clearly valuable. |

---

## 8. Success Metrics

- Users can complete a full workflow: Static baseline → Dynamic run → Frida instrumentation → Correlated report + Evidence bundle
- Correlation reduces manual analysis time noticeably
- TUI becomes the preferred way to interact with complex dynamic + Frida sessions for many users
- New high-signal findings emerge from correlation and regression that were previously hidden

---

## 9. Handoff Checklist

- [x] Review and approve this Phase 4 plan
- [x] Decide on primary focus for Phase 4a (Correlation Engine vs Evidence Bundles)
- [x] Assign initial owner(s) for Correlation Engine work
- [x] Update `docs/MOBILE.md` with Phase 4 vision as work begins (and architecture/AGENTS/README cross-docs)
- [x] Extend smoke tests to cover correlation and baseline workflows (smoke already covers baseline/bundle; correlation exercised in unit tests; 6 new unit tests added)
- [x] Schedule architecture review for TUI approach (deferred; Phase 4b TUI deferred per standalone defense-lab policy)
- [x] Phase 4b TUI work reviewed 2026-06-12: confirmed absent from TUI crate (subagent exploration: zero mobile mentions in any .rs; Tab enum has 30 variants ending Auth=29, no Mobile; no TaskConfig/Result variants, no TabSpec, no task/state_update/key/overlay wiring; feature decl only in Cargo.toml; wireless/auth are fully wired precedents). TUI remains deferred per standalone defense-lab policy (MCP/agent/pipeline also absent).
- [x] Phase 4b reporting polish delivered 2026-06-12 (non-TUI, human output only): `format_dynamic_report` + `build_dynamic_recommendations` enhanced to surface regression_notes count + bullets + "Correlation / Regression:" section (timeline hint + callout to `correlate_reports`/`CorrelationEngine`); 1 new unit test; additive only (no serde/JSON/bridge/contract changes). See dynamic.rs ~1083 (recs), ~1122 (frida line + section), ~2179 (new test). No `eggsec report diff` command exists (only Convert/Trend/Schedule); bridged categories flow; native richer for external diff + lib correlation surface.

**Phase 4a implemented** (Core Correlation Engine + Evidence Foundation delivered 2026-06-12 executed per plan; non-breaking extension; docs finalized in same pass; see Implementation Note above). Phase 4b (TUI) deferred per standalone policy (TUI exploration + deferral note added 2026-06-12); reporting polish delivered 2026-06-12 (human output enhancements only, no TUI). No architecture review for TUI needed at this time. Phase 4c remains future/iterative.

**Immediate Next Action**: Phase 4a + Phase 4b (TUI review + reporting polish) complete 2026-06-12. Future work (4c advanced) aspirational under standalone defense-lab model. Update docs cross-refs + verification in this pass.

---

## 10. References

- Phase 3 plan: `plans/mobile-dynamic-phase3-frida-expansion-plan.md`
- Current Frida implementation: `crates/eggsec/src/mobile/frida.rs`
- Existing correlation: `dynamic.rs` (`correlate_findings`, `CorrelatedFinding`)
- Parent design: `plans/dynamic-mobile-testing-loadout-design-plan.md`
- Documentation: `docs/MOBILE.md`

---

**End of Phase 4 Actionable Intelligence Plan**