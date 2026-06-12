# Mobile Dynamic: Phase 4a Final Polish Handoff Plan

**Date**: 2026-06-12  
**Status**: Draft — Ready for Review  
**Context**: Significant Phase 4a work delivered in recent commits (`30f3968...`, `43311fe...`, `e904165...`). Core `CorrelationEngine`, enriched `CorrelatedFinding`, timeline, baseline/regression, and evidence bundle improvements are now in place. Frida (Phase 3) is mature. This plan focuses on **targeted final polish** to cleanly close out the current state.

---

## 1. Executive Summary

The heavy lifting for Phase 4a (Correlation Engine + Evidence Foundation) is functionally complete. The remaining work is **polish-focused** — documentation, test coverage, minor robustness, and consistent messaging.

**Goal**: Bring the current state to a high-quality, well-documented, and maintainable point with minimal remaining effort.

**Estimated Effort**: 2–4 days focused work.

---

## 2. Current State Summary

**Completed**:
- `CorrelationEngine` + `correlate_reports()` with scoring, `CorrelationType`, enrichment, and timeline
- Enriched `CorrelatedFinding` (backward compatible)
- `MobileBaseline` + `capture_baseline()` + `compare_to_baseline()` with category delta
- `export_evidence_bundle()` with `bundle_manifest`
- Full integration into `run_dynamic_cli` (`--baseline`, `--evidence_bundle`, multi-script Frida)
- Extensive new tests for Phase 4a features
- Human report output now surfaces regression/correlation hints

**Remaining Polish Items**:

| # | Item | Priority | Effort | Notes |
|---|------|----------|--------|-------|
| 1 | Documentation updates (plans + MOBILE.md) | High | Low | Mark Phase 4a progress clearly |
| 2 | Smoke test extension for new features | High | Medium | Cover `--baseline` and `--evidence_bundle` |
| 3 | Minor robustness / error handling | Medium | Low | Edge cases in timeline, bundle export, baseline loading |
| 4 | Report formatting polish | Medium | Low | Human output consistency for new fields |
| 5 | Code hygiene & TODO cleanup | Low | Low | Comments in `dynamic.rs` and `frida.rs` |
| 6 | Backward compatibility verification | Medium | Low | Ensure pre-Phase 4 users still work cleanly |

---

## 3. Detailed Polish Tasks

### 3.1 High-Priority Polish

**Task P1: Documentation Pass**
- Update `docs/MOBILE.md` with Phase 4a examples (correlation, baseline, evidence bundle)
- Add clear "Current Status" section referencing the Phase 4 plan
- Update relevant plan files with "Phase 4a core delivered" markers
- Ensure `AGENTS.override.md` reflects latest architecture decisions

**Task P2: Smoke Test Extension**
- Extend `scripts/test-mobile-dynamic.sh` to cover:
  - `--baseline` path (dry-run)
  - `--evidence_bundle` output validation
  - Multi-script Frida + regression note presence
- Keep it CI-friendly (dry-run focused)

### 3.2 Medium-Priority Polish

**Task P3: Robustness Improvements**
- Add defensive checks in `build_timeline()` and `export_evidence_bundle()` for edge cases (missing timestamps, very large reports)
- Improve error messages when baseline JSON has unexpected schema
- Add size guard or early truncation for very large evidence bundles if needed

**Task P4: Report Output Polish**
- Ensure `format_dynamic_report()` consistently handles new fields (`regression_notes`, `correlation_notes`, `structured_results` count)
- Minor formatting improvements for the new "Correlation / Regression:" section

**Task P5: Backward Compatibility Check**
- Verify that existing `CorrelatedFinding` deserialization (without new optional fields) still works
- Confirm low-level `correlate_findings()` behavior is unchanged for callers who only use `static_correlation`

### 3.3 Low-Priority / Hygiene

**Task P6: Code Hygiene**
- Review and clean up any remaining TODO/FIXME comments in `dynamic.rs` and `frida.rs`
- Ensure consistent naming and documentation for new types (`CorrelationEngine`, `CorrelationResult`, etc.)

---

## 4. Recommended Execution Order

**Day 1**:
- Documentation updates (P1)
- Smoke test extension (P2)

**Day 2**:
- Robustness improvements (P3)
- Report formatting polish (P4)

**Day 3 (if needed)**:
- Backward compatibility verification (P5)
- Final hygiene pass (P6)
- Full test run + review

---

## 5. Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| Documentation becomes outdated quickly | Keep it high-level; focus on examples and status rather than deep internals. |
| Smoke test becomes brittle | Focus primarily on dry-run + synthetic data. Validate structure, not exact content. |
| Over-polishing low-value items | Prioritize P1 and P2. Defer P6 if time is limited. |

---

## 6. Handoff Checklist

- [ ] Review and prioritize the 6 polish tasks
- [ ] Assign owners for P1 (docs) and P2 (smoke test)
- [ ] Run full test suite after changes
- [ ] Update all relevant plan files with current status
- [ ] Decide whether to begin light Phase 4b (TUI) exploration next or stabilize here

**Immediate Next Action**: Start with documentation updates (P1) and smoke test extension (P2) — these provide the highest visibility and closure.

---

## 7. References

- Phase 4 plan: `plans/mobile-dynamic-phase4-actionable-intelligence-plan.md`
- Recent implementation commits: `30f39686...`, `43311fed...`, `e9041656...`
- Key files: `crates/eggsec/src/mobile/{dynamic.rs, frida.rs}`, `scripts/test-mobile-dynamic.sh`, `docs/MOBILE.md`

---

**End of Phase 4a Final Polish Handoff Plan**