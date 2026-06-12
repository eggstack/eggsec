# Mobile Dynamic: Phase 2 Close-Out + Phase 3 Kickoff Handoff Plan

**Date**: 2026-06-12  
**Status**: Draft — Ready for Team Review  
**Context**: Phase 2 foundation, integration, correlation, robustness, and smoke test coverage are complete (as of commit `7e343f6c9789f0b69980b1539686af5ae0b4e83d`). Only minor close-out items remain for Phase 2.
**Goal**: Formally close Phase 2 with the last minor items, then kick off the first pass of **Phase 3 (Frida / runtime instrumentation)**.

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

- [ ] Review and approve this combined close-out + kickoff plan.
- [ ] Complete Phase 2 minor items (M1 + M2) this week.
- [ ] Formally mark Phase 2 as closed in docs and plan files.
- [ ] Start Phase 3a architecture discussion (Frida crate vs CLI, script model, safety gates).
- [ ] Create feature branch for Phase 3 work.
- [ ] Assign initial owner(s) for Phase 3a scaffolding.

**Immediate Next Action**: Decide on feature gating for Phase 2 (M1) and begin Phase 3 design discussion.

---

## 7. References

- Phase 2 close-out plan: `plans/mobile-dynamic-phase2-close-out-polish-plan.md`
- Parent design: `plans/dynamic-mobile-testing-loadout-design-plan.md`
- Key files: `crates/eggsec/src/mobile/{dynamic.rs, traffic.rs}`, `docs/MOBILE.md`
- Smoke test: `scripts/test-mobile-dynamic.sh`

---

**End of Phase 2 Close-Out + Phase 3 Kickoff Handoff Plan**