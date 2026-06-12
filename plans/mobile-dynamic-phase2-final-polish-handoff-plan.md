# Mobile Dynamic Phase 2 Final Polish Handoff Plan

**Date**: 2026-06-12  
**Status**: Draft — Ready for Review  
**Context**: Phase 2a foundation + CLI/handler integration + documentation complete as of commit `32c08585a5bf3cb4001b9a97aedbe320a15fb2c5`. Phase 2 is functionally done. This plan focuses on **targeted polish** to close it out cleanly.
**Goal**: Bring Phase 2 to a high-quality, maintainable state with minimal remaining work.

---

## 1. Executive Summary

Phase 2 (proxy Level-1 + runtime permission testing) is now end-to-end functional. The remaining work is **polish-focused** rather than new functionality.

**Primary Polish Objectives**:
- Add smoke / validation coverage for Phase 2 paths
- Improve finding quality and static ↔ dynamic correlation
- Strengthen robustness and redaction in the traffic parser
- Make a clear feature-gating decision
- Final documentation and code hygiene pass

**Estimated Effort**: 3–5 days of focused work.

---

## 2. Current State

**Completed**:
- Core library support (`traffic.rs`, proxy/permission methods in `adb.rs`, `DynamicMobileReport` extensions)
- Full CLI surface with clap flags
- Handler mapping and policy enforcement
- Comprehensive documentation updates (Phase 2a examples, success criteria, troubleshooting)

**Remaining Polish Items**:

| # | Item | Priority | Effort | Owner Suggestion |
|---|------|----------|--------|------------------|
| 1 | Smoke test / validation for Phase 2 features | High | Medium | QA / Dev |
| 2 | Static ↔ dynamic finding correlation helpers | Medium | Medium | Core Dev |
| 3 | Traffic parser robustness + redaction improvements | Medium | Low-Medium | Core Dev |
| 4 | Feature gating decision (`mobile-dynamic` vs sub-feature) | Medium | Low | Architecture |
| 5 | Final code hygiene + minor report polish | Low | Low | Core Dev |
| 6 | Update "Future" section and mark Phase 2 complete in key docs | Low | Low | Docs |

---

## 3. Detailed Polish Tasks

### 3.1 High-Priority Polish

**Task F1: Smoke Test Coverage for Phase 2**
- Extend `scripts/test-mobile-dynamic.sh` (or create `test-mobile-dynamic-phase2.sh`)
- Cover:
  - Dry-run with `--proxy`, `--traffic-capture`, `--grant-permission`, `--list-permissions`
  - Verification that `traffic_summary` and `permission_state` appear in JSON output
  - Basic bridge roundtrip for new finding categories
- Make the test runnable in CI (dry-run leg only)

**Deliverable**: Passing automated validation that Phase 2 features work as expected in dry-run mode.

**Task F2: Static ↔ Dynamic Correlation**
- Add a small helper (in `dynamic.rs` or new lightweight `correlation.rs`):
  ```rust
  pub fn correlate_findings(
      dynamic_findings: &[DynamicMobileFinding],
      static_findings: &[MobileFinding],
  ) -> Vec<CorrelatedFinding>;
  ```
- Implement initial high-value correlations:
  - `traffic-cleartext` ↔ static `usesCleartextTraffic` / `network_security_config`
  - Runtime permission grants ↔ static declared permissions
- Surface correlated findings in the report (or as notes in evidence)

**Deliverable**: Basic but useful correlation between static baseline and dynamic observations.

### 3.2 Medium-Priority Polish

**Task F3: Traffic Parser Robustness**
- Improve error handling in `parse_traffic_capture` and `try_parse_minimal_har`
- Add size limit / early truncation for very large capture files
- Strengthen `sanitize_for_listing` redaction (cover more secret patterns)
- Add a few more edge-case tests (malformed HAR, very long lines, mixed schemes)

**Deliverable**: More resilient parser with better redaction coverage.

**Task F4: Feature Gating Decision**
- Team decision point:
  - **Option A (Recommended)**: Keep all current Phase 2 functionality under the existing `mobile-dynamic` feature.
  - **Option B**: Introduce `mobile-dynamic-advanced` for proxy/permission work (cleaner separation but more complexity).
- Once decided, update:
  - `Cargo.toml` feature definitions (if splitting)
  - Documentation in `docs/MOBILE.md` and `AGENTS.override.md`
  - Any conditional compilation

**Deliverable**: Clear, documented decision with minimal code impact.

### 3.3 Low-Priority / Hygiene Polish

**Task F5: Report & Output Polish**
- Improve formatting of the "Phase 2 extensions present" section in `format_dynamic_report`
- Ensure bridged info findings (`mobile-dynamic-android-traffic-summary`, etc.) are concise but informative
- Minor cleanup of comments/TODOs in `dynamic.rs` and `traffic.rs`

**Task F6: Documentation Final Pass**
- Update the "Future" section in `docs/MOBILE.md` to clearly mark Phase 2a as complete
- Add a short note about recommended workflow (static baseline → dynamic with proxy/permissions)
- Ensure all plan files have consistent "Phase 2 complete" markers

---

## 4. Recommended Order & Timeline

**Day 1–2**:
- Smoke test extension (F1)
- Traffic parser robustness (F3)

**Day 3**:
- Static ↔ dynamic correlation (F2)
- Feature gating decision (F4)

**Day 4–5**:
- Report polish + hygiene (F5)
- Final documentation pass (F6)
- Full test run + review

**Parallel**: F1 and F3 can be done in parallel. F2 and F4 are good follow-ups.

---

## 5. Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| Correlation scope creep | Keep initial implementation simple (2–3 high-value rules). Defer advanced matching to later. |
| Feature split decision takes too long | Default to Option A (keep under `mobile-dynamic`) unless strong reason to split. Revisit in Phase 2b. |
| Smoke test becomes brittle | Focus primarily on dry-run + synthetic data. Real device/proxy leg can be manual/optional. |

---

## 6. Handoff Checklist

- [ ] Review and prioritize the 6 polish tasks
- [ ] Assign owners (especially F1 and F2)
- [ ] Make feature gating decision early (affects documentation)
- [ ] Run full test suite + new smoke test before merge
- [ ] Update all relevant plan files with "Phase 2 complete" status
- [ ] Announce Phase 2 closure to the team

**Immediate Next Action**: Decide on feature gating strategy and assign owner for smoke test extension (F1).

---

## 7. References

- Phase 2 foundation: commit `78436afa7ab849d924d93ce84d45e87306191a8a`
- Phase 2 integration: commit `32c08585a5bf3cb4001b9a97aedbe320a15fb2c5`
- Previous plans: `plans/mobile-dynamic-phase2-implementation-handoff-plan.md` and `plans/mobile-dynamic-phase2-polish-and-completion-handoff-plan.md`
- Key files: `crates/eggsec/src/mobile/{dynamic.rs, traffic.rs, adb.rs}`, `cli/mobile.rs`, `commands/handlers/mobile.rs`, `docs/MOBILE.md`

---

**End of Phase 2 Final Polish Handoff Plan**