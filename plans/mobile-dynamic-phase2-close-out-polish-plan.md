# Mobile Dynamic Phase 2 Close-Out Polish Plan

**Date**: 2026-06-12  
**Status**: Draft — Final Close-Out Plan  
**Current State**: After commit `05d57766de867ec3c25b95ac8e5bc213bf319289` (traffic robustness + correlation implemented).
**Goal**: Finish the last remaining items to cleanly close Phase 2.

---

## 1. Executive Summary

Phase 2 is now functionally complete and significantly polished. The two largest remaining polish items from the previous plan have been delivered:
- Traffic parser robustness + expanded redaction
- Static ↔ dynamic correlation (`correlate_findings`)

This short plan focuses on the **final 3 items** needed to close Phase 2.

**Remaining Work** (estimated 2–4 days):
1. Smoke test / validation coverage for Phase 2 features
2. Feature gating decision + documentation
3. Final hygiene + close-out documentation

---

## 2. Remaining Polish Items

### 2.1 High-Priority (Recommended First)

**Task C1: Smoke Test Coverage for Phase 2**
- Extend `scripts/test-mobile-dynamic.sh` (or create a dedicated Phase 2 test script)
- Cover key Phase 2 flows in dry-run mode:
  - `--proxy` + `--traffic-capture`
  - `--grant-permission` / `--revoke-permission` / `--list-permissions`
  - Verification that `traffic_summary`, `permission_state`, and `static_correlation` appear correctly in JSON output
- Ensure the test is CI-friendly (dry-run only, no external dependencies)

**Deliverable**: Automated validation that Phase 2 features work end-to-end in dry-run.

**Task C2: Feature Gating Decision**
- Make and document the decision:
  - **Recommended**: Keep all current Phase 2 functionality under the existing `mobile-dynamic` feature (simpler, lower maintenance).
  - Alternative: Introduce `mobile-dynamic-advanced` (cleaner separation but adds complexity).
- Once decided:
  - Update `Cargo.toml` (if splitting)
  - Update `docs/MOBILE.md` and `AGENTS.override.md`
  - Add clear comments in code

**Deliverable**: Clear, documented decision with minimal code changes.

### 2.2 Low-Priority / Close-Out

**Task C3: Final Hygiene & Documentation**
- Minor report formatting polish in `format_dynamic_report` (Phase 2 section)
- Clean up any remaining TODOs or outdated comments in `dynamic.rs` and `traffic.rs`
- Final pass on `docs/MOBILE.md`:
  - Ensure "Phase 2 complete" messaging is consistent
  - Add a short recommended workflow note (static baseline → dynamic with proxy/permissions)
- Update all plan files with final "Phase 2 closed" markers

**Deliverable**: Clean codebase and consistent documentation ready for Phase 2 closure announcement.

---

## 3. Recommended Execution Order

**Day 1**:
- Smoke test extension (C1)

**Day 2**:
- Feature gating decision + minimal documentation updates (C2)

**Day 3 (if needed)**:
- Final hygiene and documentation pass (C3)
- Full test run + review

---

## 4. Handoff Checklist

- [ ] Assign owner for smoke test (C1)
- [ ] Make feature gating decision early (affects docs)
- [ ] Run full test suite before final merge
- [ ] Update plan files and `docs/MOBILE.md` with Phase 2 closure status
- [ ] Announce Phase 2 completion to the team

**Immediate Next Action**: Decide on feature gating strategy and start smoke test work.

---

## 5. References

- Latest polish commit: `05d57766de867ec3c25b95ac8e5bc213bf319289`
- Previous final polish plan: `plans/mobile-dynamic-phase2-final-polish-handoff-plan.md`
- Key files: `crates/eggsec/src/mobile/{dynamic.rs, traffic.rs}`, `docs/MOBILE.md`

---

**End of Phase 2 Close-Out Polish Plan**