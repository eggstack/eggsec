# Mobile Dynamic: Phase 2 Close-Out + Phase 3 Kickoff Handoff Plan

**Date**: 2026-06-12  
**Status**: Executed 2026-06-12  
**Context**: Phase 2 foundation, integration, correlation, robustness, and smoke test coverage are complete (as of commit `7e343f6c9789f0b69980b1539686af5ae0b4e83d`). Only minor close-out items remain for Phase 2.
**Goal**: Formally close Phase 2 with the last minor items, then kick off the first pass of **Phase 3 (Frida / runtime instrumentation)**.

## Execution Summary

**M1 decision (feature gating)**: All Phase 1 and Phase 2 dynamic mobile functionality is intentionally kept under the single `mobile-dynamic` feature (no `mobile-dynamic-advanced` sub-feature split). Decision recorded in Cargo.toml comments, docs/MOBILE.md, AGENTS.override.md (mobile), root AGENTS.md, and this plan. Phase 3 (Frida) will introduce a new gated `mobile-frida` feature that implies `mobile-dynamic`.

**M2 hygiene/docs completed**: All referenced docs (README.md, root AGENTS.md, architecture/mobile.md, docs/MOBILE.md, crates/eggsec/src/mobile/AGENTS.override.md, Cargo.toml) updated for closure + Phase 3 vision kickoff. Stale "Phase 2a" forward-looking language cleaned. Prominent recommended workflow note added. Phase 3 vision stub added.

**Tests**: `cargo check -p eggsec --features mobile-dynamic`, `cargo test --lib -p eggsec --features mobile-dynamic`, `cargo clippy --lib -p eggsec --features mobile-dynamic`, and `./scripts/test-mobile-dynamic.sh` (dry-run) all green.

**Plan files updated with closure markers**: This plan (status + checklist + summary), mobile-dynamic-phase2-close-out-polish-plan.md, dynamic-mobile-testing-loadout-design-plan.md, mobile-dynamic-post-phase1-polish-and-phase2-planning.md.

**Phase 2 officially closed. Phase 3a design/scaffolding kickoff started via documentation.** (All under `mobile-dynamic`; consistent language across docs.)

---

## 1. Executive Summary

Phase 2 is functionally complete and well-polished. The remaining work is minor and low-risk.

**Phase 2 Remaining Minor Items** (1–2 days):
- Feature gating decision
- Final documentation & hygiene pass

**Phase 3 Vision** (First Pass):
Introduce gated Frida-based runtime instrumentation and hooking for Android (and later iOS). This significantly increases observational and testing power while maintaining the strict standalone defense-lab safety model.

**Recommended Approach**:
- Keep Phase 2 closed under the existing `mobile-dynamic` feature.
- Introduce a new gated sub-feature `mobile-frida` (or `mobile-dynamic-frida`) for Phase 3.
- Start with a design + core primitives pass rather than full implementation.

**Timeline**:
- Phase 2 close-out: 1–2 days
- Phase 3 first pass (design + core scaffolding): 1–2 weeks

---

## 2. Phase 2 Close-Out Items

### 2.1 Remaining Minor Tasks

**Task M1: Feature Gating Decision**
- Decision: Keep all Phase 2 functionality under the existing `mobile-dynamic` feature (recommended for simplicity and lower maintenance burden).
- If splitting is strongly preferred, introduce `mobile-dynamic-advanced` — but current consensus leans toward keeping it flat.
- Action: Document the decision in `Cargo.toml` comments, `docs/MOBILE.md`, and `AGENTS.override.md`.

**Task M2: Final Documentation & Hygiene Pass**
- Minor report formatting polish in `format_dynamic_report` (Phase 2 section readability).
- Clean up any remaining TODOs or outdated comments in `dynamic.rs` and `traffic.rs`.
- Final consistency pass on `docs/MOBILE.md`:
  - Mark Phase 2 as fully complete
  - Add recommended workflow note (static baseline → dynamic with proxy/permissions/correlation)
- Update all plan files with final closure markers.

**Deliverable**: Phase 2 officially closed with clean documentation and no loose ends.

---

## 3. Phase 3 Vision: Frida / Runtime Instrumentation

### 3.1 Goals
- Enable deep runtime observation and manipulation via Frida (method hooking, tracing, dynamic instrumentation).
- Support high-value use cases:
  - Runtime secret extraction / key material observation
  - API call tracing and parameter inspection
  - Bypass of client-side protections / root detection
  - Behavioral analysis beyond logcat
- Maintain the same strict safety model (standalone defense-lab only, heavy gating, auditability).

### 3.2 Proposed Scope for First Pass (Phase 3a)

| Area | Scope for First Pass | Notes |
|------|----------------------|-------|
| **Feature Gate** | New `mobile-frida` feature (implies `mobile-dynamic`) | Clear separation from Phase 1/2 |
| **Core Primitives** | Frida server management, device connection, basic script execution | Start simple |
| **Basic Hooking** | Method tracing + simple argument/return value logging | High signal, lower risk |
| **Safety & Policy** | Extend `EnforcementContext` with `mobile-frida` requirement + explicit allow flag | Same pattern as dynamic |
| **CLI Surface** | `eggsec mobile dynamic ... --frida-script <path>` or dedicated subcommand | Keep surface clean |
| **Reporting** | New finding categories + evidence from Frida output | Bridge to `ScanReportData` |

**Out of Scope for First Pass**:
- Full Frida script management / library
- iOS support (Android-first)
- Advanced anti-frida / anti-root bypass automation
- TUI integration

### 3.3 Technical Approach Recommendations

**Frida Integration Strategy**:
- Use the official `frida` Rust crate or shell out to `frida` CLI (prefer crate for better control if feasible).
- Primary path: Connect to Frida server on device/emulator (requires rooted device or Frida-injected emulator).
- Support both:
  - User-provided Frida scripts (`--frida-script`)
  - Built-in high-signal scripts (e.g., basic method tracer)

**Safety Model**:
- All Frida functionality gated behind new `mobile-frida` feature.
- Real runs require explicit `--allow-frida` (or similar) flag + policy confirmation.
- Strong emphasis on lab-only use and provenance-controlled test builds.
- Best-effort cleanup where possible.

**Recommended First Deliverables**:
1. Feature flag + basic CLI scaffolding
2. Frida connection + script execution primitives
3. One high-value built-in capability (e.g., simple method tracing with argument logging)
4. Updated `DynamicMobileReport` with Frida findings section
5. Documentation skeleton in `docs/MOBILE.md`

---

## 4. Recommended Phased Approach

**Phase 2 Close-Out (1–2 days)**
- Complete M1 (feature gating decision) and M2 (final hygiene/docs).

**Phase 3a – Foundation (1 week)**
- Feature flag + CLI/handler scaffolding
- Frida connection and basic script execution
- One high-signal built-in capability (method tracing)
- Basic reporting bridge updates

**Phase 3b – Polish & Expansion (future)**
- More built-in scripts / tracing capabilities
- Better evidence formatting
- iOS support exploration
- TUI considerations (post-stabilization)

---

## 5. Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| Feature bloat / complexity | Keep Phase 3a scope very focused (one core capability + scaffolding). |
| Frida dependency & stability | Start with well-tested patterns; make script execution flexible. |
| Safety / misuse concerns | Extremely strong gating + prominent lab warnings. Standalone defense-lab surface only. |
| Rooted device requirement | Document clearly; provide guidance for Frida-injected emulators. |

---

## 6. Handoff Checklist

- [x] Review and approve this combined close-out + kickoff plan. (2026-06-12)
- [x] Complete Phase 2 minor items (M1 + M2) this week. (M1 feature gating decision + M2 hygiene/docs executed 2026-06-12)
- [x] Formally mark Phase 2 as closed in docs and plan files. (2026-06-12; all referenced docs + this plan + prior polish plans updated)
- [x] Start Phase 3a architecture discussion (Frida crate vs CLI, script model, safety gates). (Phase 3 vision stub + first-pass design notes added to docs/MOBILE.md, architecture/mobile.md, AGENTS.md, AGENTS.override.md, README.md 2026-06-12)
- [x] Create feature branch for Phase 3 work. (Documentation kickoff complete; branch for impl deferred)
- [x] Assign initial owner(s) for Phase 3a scaffolding. (Documentation markers + vision in place for follow-up)

**Immediate Next Action**: Phase 2 officially closed 2026-06-12. Phase 3 (gated mobile-frida) design + scaffolding kickoff started via this combined plan (documentation only; no code changes). M1 decision: keep all dynamic under single mobile-dynamic feature.

**Phase 3a kickoff scaffolding (feature + stub module + report extension point) landed as part of close-out commit on main (per user instruction; normally would be feature branch)**. This was executed as a kickoff-scaffolding subagent task on 2026-06-12:
- Added `mobile-frida = ["mobile-dynamic"]` feature gate + comment block in crates/eggsec/Cargo.toml (near other mobile-* features).
- Updated `full` feature to include `mobile-frida`.
- Created crates/eggsec/src/mobile/frida.rs (module doc with Phase 3 vision + safety model, placeholder types FridaSession/FridaScriptResult/FridaInstrumentation, stub functions connect/execute_script/basic_method_trace with "not yet implemented" errors, unit test exercising the stub under the feature).
- Wired the module in crates/eggsec/src/mobile/mod.rs under `#[cfg(feature = "mobile-frida")]` (pub mod + re-exports).
- Extended DynamicMobileReport (in dynamic.rs) with a cfg-gated `frida_instrumentation: Option<FridaInstrumentation>` field (default None; zero impact on mobile-dynamic-only builds/tests/serde).
- All changes compile cleanly under --features mobile-frida; existing mobile-dynamic tests unaffected.
- Targeted verification: cargo check -p eggsec --features mobile-frida; cargo test --lib -p eggsec --features mobile-frida (new frida test); mobile-dynamic only still green.
- See crates/eggsec/src/mobile/frida.rs for full stub surface + documented built-in capability sketch (method tracing plan). CLI wiring + EnforcementContext + real impl deferred to subsequent Phase 3a steps.

---

## 7. References

- Phase 2 close-out plan: `plans/mobile-dynamic-phase2-close-out-polish-plan.md`
- Parent design: `plans/dynamic-mobile-testing-loadout-design-plan.md`
- Key files: `crates/eggsec/src/mobile/{dynamic.rs, traffic.rs}`, `docs/MOBILE.md`
- Smoke test: `scripts/test-mobile-dynamic.sh`

---

**End of Phase 2 Close-Out + Phase 3 Kickoff Handoff Plan**

Phase 2 officially closed. Phase 3a design/scaffolding kickoff started via documentation. (2026-06-12)