# Mobile Dynamic Phase 3: Frida Expansion Plan (Merged under mobile-dynamic)

**Date**: 2026-06-12  
**Status**: Executed (Phase 3a Foundation + First Capability delivered 2026-06-12)  
**Key Decision**: All Frida capabilities will be developed **under the existing `mobile-dynamic` feature** (no separate `mobile-frida` sub-feature). Safety will be enforced via runtime flags and policy.
**Context**: Phase 2 is functionally complete. The initial Frida scaffolding (`frida.rs`) has been added as a clean starting point. This plan defines a structured expansion across three phases.

**Executed note (Phase 3a)**: Per explicit Key Decision and detailed work items 1-8, feature model reconciled (Cargo.toml: removed mobile-frida, full cleaned, comments rewritten to state Frida under mobile-dynamic governed by --allow-frida + policy). All `#[cfg(feature = "mobile-frida")]` replaced/removed. CLI surface (--frida-script, --allow-frida) + MOBILE_*_ABOUT updated. Handler policy: Intrusive for real Frida, runtime --allow-frida gate (dry safe), mapped to DynamicMobileArgs. frida.rs: real connect (CLI check + probe or sim), execute_script (temp+frida CLI or sim), basic_method_trace (generate + execute; safe JS with JSON markers for Cipher/keystore/auth/detection), is_frida_cli_available, generate_basic_method_trace_script, expanded unit tests (dry paths, error on missing CLI, script gen, parsing). DynamicMobileReport/Args + run_dynamic_cli + format_dynamic_report + to_scan_report_data_dynamic + build_recs extended (dry: sim actions/findings/carrier; real: connect/execute/builtin or user script, map to frida-* findings + instrumentation; extra info + bridge categories mobile-dynamic-android-frida-*). Smoke script updated (Phase 3a Frida dry leg; self-doc; hardware-free). Docs updated last (AGENTS.override.md, architecture/mobile.md, docs/MOBILE.md, README.md, root AGENTS.md, plan annotated). All under single mobile-dynamic. cargo check/test/clippy --features mobile-dynamic green; ./scripts/test-mobile-dynamic.sh (dry + Frida leg) passes. Standalone defense-lab, no MCP/agent. Dry-run always valid. Phase 3a complete; 3b/3c future. (2026-06-12)

---

## 1. Executive Summary

This plan outlines the expansion of dynamic mobile capabilities into **Phase 3: Frida-based runtime instrumentation and hooking**.

All work will remain under the single `mobile-dynamic` feature flag for simplicity. Safety and auditability will be maintained through explicit allow flags (e.g. `--allow-frida`) and `EnforcementContext` policy gates.

**Phased Approach**:
- **Phase 3a**: Foundation + First Real Capability (Core Frida integration + one high-value built-in)
- **Phase 3b**: Polish + High-Value Expansion (More built-ins, correlation, evidence quality)
- **Phase 3c**: Advanced Capabilities (Script library, deeper correlation, behavioral regression)

**Timeline**:
- Phase 3a: 2–3 weeks
- Phase 3b: 2–3 weeks
- Phase 3c: Future / iterative

---

## 2. Architectural Decisions

### 2.1 Feature Gating
- **Decision**: Keep Frida under `mobile-dynamic` (no new feature flag).
- Rationale: Simpler user experience and lower maintenance. All dynamic capabilities (ADB + proxy + permissions + Frida) live together.
- Safety will be handled at runtime via:
  - Explicit `--allow-frida` flag (or similar)
  - Policy evaluation in the handler
  - Prominent lab warnings
  - Audit trail requirements

### 2.2 Safety Model
- Frida is more intrusive than Phase 1/2 capabilities.
- Will likely use a higher policy tier (e.g. `OperationRisk::Intrusive`).
- Requires rooted device or Frida-injected emulator.
- All real runs must go through policy confirmation.
- Best-effort cleanup for injected state.

### 2.3 Integration Points
- Extend `DynamicMobileReport` with `frida_instrumentation: Option<FridaInstrumentation>`
- New finding categories: `frida-method-trace`, `frida-secret-extract`, `frida-bypass`, etc.
- Update `to_scan_report_data_dynamic` bridge
- Extend `run_dynamic_cli` and handler mapping

---

## 3. Phase 3a: Foundation + First Capability

**Goal**: Deliver working Frida integration with one high-value built-in capability.

### Deliverables
| # | Deliverable | Description |
|---|-------------|-------------|
| 1 | Real Frida connection | Implement `connect(device)` using frida crate or CLI | 
| 2 | Script execution | Working `execute_script()` that runs user or built-in JS | 
| 3 | First built-in: `basic_method_trace` | Hook common sensitive methods and produce structured findings |
| 4 | CLI surface | Add `--frida-script`, `--allow-frida`, and related flags |
| 5 | Handler + Policy | Wire Frida paths through `EnforcementContext` with explicit allow flag |
| 6 | Basic reporting | Populate `FridaInstrumentation` and emit findings via the bridge |

**Recommended First Built-in Targets**:
- `javax.crypto.Cipher.doFinal`
- `android.security.keystore.*`
- Common login / token handling methods
- Root / Frida detection hooks

**Success Criteria**:
- `cargo build --features mobile-dynamic` succeeds
- Dry-run with Frida flags produces valid report structure
- Real connection + script execution works on a Frida-injected emulator
- Findings appear in both native JSON and bridged `ScanReportData`

---

## 4. Phase 3b: Polish & High-Value Expansion

**Goal**: Make Frida capabilities robust, useful, and well-integrated.

### Deliverables
- Additional built-in capabilities:
  - Crypto / keystore observation
  - Bypass and detection validation
  - API call tracing with parameter inspection
- Frida + traffic/proxy correlation (e.g. correlate Frida-observed calls with proxy traffic)
- Improved evidence quality and redaction for Frida output
- Richer `FridaInstrumentation` struct on `DynamicMobileReport`
- Better structured output from Frida scripts (JSON preferred over raw text)
- Expanded test coverage and robustness

**Success Criteria**:
- Multiple high-signal built-ins available
- Correlation between Frida findings and existing dynamic data works
- Evidence is clean and actionable
- Performance and stability are acceptable for lab use

---

## 5. Phase 3c: Advanced Capabilities

**Goal**: Mature Frida support into a powerful, flexible capability.

### Deliverables
- User script library / reusable Frida components
- Advanced correlation engine (static ↔ dynamic ↔ Frida)
- Behavioral baselining and regression for dynamic + Frida runs
- Support for complex multi-script sessions
- Constrained iOS dynamic exploration (jailbroken devices or equivalent)
- TUI integration considerations (post-stabilization)
- Optional evidence bundle export (Frida output + traffic + logs)

**Success Criteria**:
- Users can easily write and reuse custom Frida scripts
- Strong correlation across all dynamic data sources
- Behavioral regression becomes a practical lab workflow

---

## 6. Implementation Recommendations

**Technical Approach**:
- Prefer the `frida` Rust crate when stable and feature-complete. Fall back to shelling out to the `frida` CLI if needed.
- Keep the module clean and well-documented (current scaffolding is a good model).
- Make built-in scripts configurable where possible.
- Prioritize structured/JSON output from Frida scripts for easier parsing.

**Safety & Policy**:
- Introduce `--allow-frida` flag (mirrors `--allow-dynamic-mobile`).
- Consider elevating the policy risk tier for Frida operations.
- Always record Frida actions in the audit trail.

**Testing Strategy**:
- Heavy use of dry-run for most development
- Unit tests for scaffolding and correlation logic
- Integration tests against Frida-injected emulators for real paths
- Smoke test updates in `scripts/test-mobile-dynamic.sh`

---

## 7. Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Frida stability / device variability | High | Start with well-known emulator setups. Make error messages clear. |
| Scope creep in built-ins | Medium | Prioritize 2–3 high-value methods first. Expand iteratively. |
| Safety / misuse concerns | High | Strong gating + explicit allow flag + prominent warnings. Standalone defense-lab only. |
| Performance overhead | Medium | Make Frida optional. Allow users to run without it. |

---

## 8. Handoff Checklist

- [ ] Review and approve this phased plan
- [ ] Confirm decision to keep Frida under `mobile-dynamic` (no new feature flag)
- [ ] Start Phase 3a work (Frida connection + first built-in)
- [ ] Update `docs/MOBILE.md` with Phase 3 vision and examples as work progresses
- [ ] Extend smoke test script for Frida paths
- [ ] Schedule architecture review for correlation engine (Phase 3b)

**Immediate Next Action**: Begin implementation of real `connect()` and `execute_script()` in `frida.rs`, along with CLI flag definitions.

---

## 9. References

- Current scaffolding: `crates/eggsec/src/mobile/frida.rs` (commit `d0bc009b...`)
- Previous plans:
  - `plans/mobile-dynamic-phase2-closeout-and-phase3-kickoff-plan.md`
  - `plans/mobile-dynamic-phase2-close-out-polish-plan.md`
- Parent design: `plans/dynamic-mobile-testing-loadout-design-plan.md`
- Documentation: `docs/MOBILE.md`

---

**End of Phase 3 Frida Expansion Plan**