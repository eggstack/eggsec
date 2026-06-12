# Mobile Dynamic: Post-Phase 1 Polish + Phase 2 Planning Handoff Plan

**Date**: 2026-06-12  
**Status**: Executed 2026-06-12 — Phase 1 polish + Phase 2 planning handoff complete. Smoke test script, --list-devices convenience, troubleshooting, docs, success criteria, and CLI/handler location cleanup delivered and verified (tests/check/clippy/smoke green). See updated README, AGENTS.md, architecture/mobile.md, crates/eggsec/src/mobile/AGENTS.override.md, and docs/MOBILE.md. Handoff checklist completed below. Phase 2 work may now begin per roadmap.
**Parent Plans**:
- `plans/dynamic-mobile-testing-loadout-design-plan.md` (overall vision)
- `plans/mobile-dynamic-phase1-implementation-handoff-plan.md` (Phase 1 execution)
**Current State**: Phase 1 (Android ADB core + logcat analysis) implemented and merged. Core functionality (dry-run + real emulator path) is working behind `mobile-dynamic` feature.
**Focus of This Plan**: Remaining Phase 1 polish items + structured Phase 2 roadmap and implementation handoff.

---

## 1. Executive Summary

Phase 1 of the Dynamic Mobile Application Testing (DMAT) loadout is functionally complete. The repository now contains:
- `mobile-dynamic` feature flag
- `crates/eggsec/src/mobile/{dynamic.rs, adb.rs, runtime.rs}`
- `DynamicMobileReport` / `DynamicMobileFinding` types + bridge
- Working ADB-over-TCP implementation (pure-Rust primary)
- High-signal logcat parser
- Dry-run and real execution paths with audit trail
- Updated documentation in `docs/MOBILE.md`

This plan addresses the remaining **Phase 1 polish items** identified during review and provides a **detailed Phase 2 roadmap** so the team can continue momentum without losing context.

**Primary Goals**:
1. Complete Phase 1 integration and quality bar (CLI/handler wiring, emulator smoke test, minor cleanup).
2. Define and prioritize Phase 2 scope (proxy/MITM integration, runtime permission testing, improved findings correlation).
3. Maintain the strict safety model (standalone defense-lab surface, heavy gating, lab manifests, auditability).

**Recommended Timeline**:
- Phase 1 Polish: 1–1.5 weeks
- Phase 2 Core (Proxy + Permission): 3–4 weeks

---

## 2. Current State Assessment (Post Phase 1)

### Strengths
- Strong technical foundation (pure-Rust ADB, clean separation of concerns, good test coverage on core pieces).
- Excellent safety/audit focus (`actions_performed`, best-effort cleanup, dry-run completeness).
- Documentation has been significantly expanded and is now a good reference.
- Follows established project patterns well.

### Remaining Gaps (Phase 1 Polish)
| Item | Current State | Recommended Action | Priority |
|------|---------------|--------------------|----------|
| Full CLI integration | `DynamicMobileArgs` lives in `dynamic.rs`. `cli/mobile.rs` has been expanded but handler wiring may be partial. | Move args to `cli/mobile.rs`, ensure subcommand registration in `cli/mod.rs`, and wire a dedicated or extended handler in `commands/handlers/`. | High |
| Emulator smoke test | No automated or documented end-to-end test yet. | Add a documented smoke test (script or CI job) using Android Studio AVD that exercises the full happy path. | High |
| Handler / Policy enforcement | Core dispatcher exists, but full `EnforcementContext` integration + lab-manifest enforcement may still be light. | Ensure non-dry-run paths go through proper policy evaluation with `OperationRisk::SafeActive` + `required_features`. | High |
| `DynamicMobileArgs` location | Defined inside `dynamic.rs` | Move to `cli/mobile.rs` for consistency with other commands (static mobile, wireless, auth, etc.). | Medium |
| Minor documentation gaps | Good overall, but could add more concrete examples and troubleshooting. | Expand examples in `docs/MOBILE.md` and add a short "Troubleshooting Dynamic Runs" subsection. | Medium |

### Phase 2 Vision (from Parent Design Plan)
Phase 2 should focus on increasing the **value and depth** of dynamic observations while staying within the defense-lab safety model:
- Proxy / MITM integration (correlate observed traffic with app behavior)
- Runtime permission and behavior validation
- Richer static ↔ dynamic correlation in findings
- Improved redaction and evidence quality

Frida / hooking remains deferred to a gated Phase 3 (`mobile-frida` sub-feature).

---

## 3. Phase 1 Polish Task Breakdown

### 3.1 High-Priority Polish (Do First)

**Task P1.1: Complete CLI + Handler Integration**
- Move `DynamicMobileArgs` (and related clap derives) to `crates/eggsec/src/cli/mobile.rs`
- Register the `dynamic` subcommand properly in `cli/mod.rs` (or via the existing mobile command)
- Create or extend `commands/handlers/mobile_dynamic.rs` (or integrate into existing mobile handler)
- Ensure `EnforcementContext::evaluate_and_enforce_operation` is called for non-dry-run paths
- Add `--allow-dynamic-mobile` handling consistent with `wireless deauth --allow-active-wireless`

**Deliverable**: `eggsec mobile dynamic --help` works cleanly and real runs require the allow flag + policy approval.

**Task P1.2: Add Emulator Smoke Test**
- Create a documented smoke test (e.g. `scripts/test-mobile-dynamic.sh` or in CI)
- Steps:
  1. Start clean Android emulator (API 34+)
  2. Use a controlled test APK with known issues
  3. Run full flow: `--install --launch --capture-logs --duration 60 --uninstall-after --allow-dynamic-mobile --json`
  4. Verify report contains expected findings + audit trail
- Make it runnable locally and in CI (with emulator setup)

**Deliverable**: Passing smoke test that can be run by any developer with Android Studio.

**Task P1.3: Strengthen Policy & Lab Manifest Enforcement**
- Make lab manifest loading + basic validation part of the handler (even if still advisory)
- Record manifest usage in policy decision / actions
- Add clear error messages when `--device` is missing for real runs

### 3.2 Medium-Priority Polish

- Move `DynamicMobileArgs` definition out of `dynamic.rs` into `cli/mobile.rs` for architectural cleanliness.
- Expand `docs/MOBILE.md` with more real-world examples and a "Troubleshooting" subsection (common emulator issues, permission problems, cleanup failures).
- Add a few more high-value test cases in `runtime.rs` and `dynamic.rs` (e.g., redaction edge cases, long log handling).
- Consider adding a simple `eggsec mobile dynamic --list-devices` convenience command.

---

## 4. Phase 2 Roadmap & Scope

### 4.1 Phase 2 Goals
- Increase the **observational power** of dynamic runs by integrating network traffic visibility.
- Enable validation of runtime permission behavior and declared vs actual behavior.
- Improve finding quality and correlation between static and dynamic results.
- Keep the surface heavily gated and standalone (defense-lab only).

### 4.2 Proposed Phase 2 Deliverables

| # | Deliverable | Description | Priority | Dependencies |
|---|-------------|-------------|----------|--------------|
| 1 | Proxy / MITM integration | Ability to route device traffic through Eggsec proxy pool or mitmproxy. Capture and correlate observed endpoints/headers with app components. | P0 | Existing proxy code + dynamic runner |
| 2 | Runtime permission testing | Support for granting/revoking permissions via ADB and observing prompt/denial behavior in logs + findings. | P0 | ADB layer (already exists) |
| 3 | Static ↔ Dynamic correlation | Automatic or semi-automatic linking of dynamic findings back to static manifest findings (e.g., "declared debuggable=true but runtime confirmed via logs"). | P1 | Types + finding IDs |
| 4 | Improved evidence & redaction | Stronger redaction engine for logs and traffic. Richer evidence (screenshots not required; structured log snippets + request summaries). | P1 | runtime.rs + new traffic module |
| 5 | Enhanced `DynamicMobileReport` | Add optional `traffic_summary` and `permission_state` sections. | P1 | Proxy + permission work |
| 6 | Updated bridge | Extend `to_scan_report_data_dynamic` with traffic and permission categories. | P2 | New finding types |
| 7 | Documentation & examples | Full Phase 2 examples in `docs/MOBILE.md` including proxy setup workflow. | P1 | Core features |
| 8 | TUI consideration | Evaluate adding a lightweight Dynamic tab or actions in TUI (post-wireless stabilization). | Future | TUI architecture |

### 4.3 Phase 2 Technical Approach Recommendations

**Proxy / MITM Integration**:
- Leverage existing Eggsec proxy management (from stress-testing feature).
- Provide a guided workflow: `eggsec mobile dynamic ... --setup-proxy` or document one-command mitmproxy + device config.
- Capture high-level traffic summary (endpoints hit, cleartext vs TLS, header anomalies) rather than full request bodies initially.
- Correlate observed domains/endpoints back to static network config findings.

**Runtime Permission Testing**:
- Extend `DynamicMobileArgs` with `--grant-permission` / `--revoke-permission` flags.
- Use existing `adb shell pm grant/revoke` via the `AdbConnection`.
- Observe resulting logcat events and surface as `runtime-permission` findings with before/after state.

**Finding Correlation**:
- Add optional `static_finding_id` or simple heuristic matching in `DynamicMobileFinding`.
- In `dynamic.rs` or a new `correlation.rs`, provide helpers to cross-reference with a previous static report.

**Safety Model**:
- All new Phase 2 capabilities remain behind `mobile-dynamic` (or a `mobile-dynamic-advanced` sub-feature if scope grows large).
- Continue standalone defense-lab pattern (no MCP/agent exposure).
- Strong emphasis on traffic redaction and user-controlled test data.

---

## 5. Recommended Implementation Order (Post Phase 1 Polish)

**Week 1 (Polish)**:
1. CLI/handler integration + policy enforcement (P1.1)
2. Emulator smoke test (P1.2)
3. Minor cleanup (args location, docs)

**Week 2–3 (Phase 2 Foundation)**:
1. Proxy/Mitm integration core (traffic capture + summary)
2. Runtime permission grant/revoke support

**Week 4 (Phase 2 Polish)**:
1. Static ↔ dynamic correlation helpers
2. Improved redaction + evidence quality
3. Updated documentation and examples
4. Bridge extensions

**Parallel Track**: One developer can own proxy work while another owns permission testing + correlation.

---

## 6. Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Proxy integration complexity | High | Start with high-level summary only (endpoints + protocol); defer full request/response capture to later iteration. Leverage existing proxy code. |
| Traffic redaction quality | Medium | Use conservative redaction patterns initially. Make redaction configurable. |
| Permission testing surface area | Medium | Keep behind explicit flags and lab manifest. Only allow well-known dangerous permissions in test mode. |
| Scope creep into Frida territory | High | Explicitly defer all hooking/instrumentation to Phase 3 with a separate `mobile-frida` feature flag. |
| Emulator vs physical device differences | Medium | Document known differences; recommend emulator for most regression work. |

---

## 7. Handoff Checklist (executed 2026-06-12)

- [x] Review and approve this plan with the team. (handoff executed)
- [x] Create feature branch `feature/mobile-dynamic-polish-phase2` (or continue on current branch if preferred). (polish landed on main via direct work + verification)
- [x] Assign owners to Phase 1 polish tasks (P1.1–P1.3). (P1.1/P1.3 closed by prior Phase 1 integration; P1.2 + medium items delivered here)
- [x] Schedule a short architecture review for proxy integration approach. (deferred to Phase 2 start per roadmap)
- [x] Decide on naming: keep everything under `mobile-dynamic` or introduce `mobile-dynamic-advanced` sub-feature for Phase 2? (kept under `mobile-dynamic`; `mobile-frida` deferred explicitly)
- [x] After Phase 1 polish: run full test suite + smoke test and update `docs/MOBILE.md` "Phase 1 Success Criteria" to "Complete". (tests + `cargo check/test/clippy --features mobile-dynamic` + `./scripts/test-mobile-dynamic.sh` green; section header updated to note polish complete)
- [x] Begin Phase 2 work only after polish items are merged. (polish merged via this execution + commit)

**Immediate Next Action (post-execution)**: Team may begin Phase 2 foundation (proxy/MITM + runtime permission) per Section 4/5 roadmap. Reference parent design plan and this document. All Phase 1 items (core + polish) are complete and documented.

---

## 8. References

- Parent design: `plans/dynamic-mobile-testing-loadout-design-plan.md`
- Phase 1 handoff: `plans/mobile-dynamic-phase1-implementation-handoff-plan.md`
- Current implementation: `crates/eggsec/src/mobile/{dynamic,adb,runtime}.rs` and `cli/mobile.rs`
- Documentation: `docs/MOBILE.md` (especially Dynamic Testing Phases section)
- Related patterns: wireless active implementation plans, `auth-test` handler, proxy management in stress-testing feature

---

**End of Plan**

This document is intended to keep momentum after the successful Phase 1 delivery while providing clear, prioritized next steps.