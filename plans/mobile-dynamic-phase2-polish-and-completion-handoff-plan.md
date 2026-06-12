# Mobile Dynamic Phase 2 Polish & Completion Handoff Plan

**Date**: 2026-06-12  
**Status**: Draft — Ready for Team Review  
**Parent Plans**:
- `plans/dynamic-mobile-testing-loadout-design-plan.md`
- `plans/mobile-dynamic-phase1-implementation-handoff-plan.md`
- `plans/mobile-dynamic-post-phase1-polish-and-phase2-planning.md`
- `plans/mobile-dynamic-phase2-implementation-handoff-plan.md`
**Current Commit Context**: Phase 2 foundation landed in `78436afa7ab849d924d93ce84d45e87306191a8a` (new `traffic.rs`, extended `dynamic.rs` + `adb.rs` with proxy/permission support).
**Focus**: Complete and polish the Phase 2 work that was started. Turn the foundation into a production-ready, well-documented capability.

---

## 1. Executive Summary

The core plumbing for Phase 2 (device proxy configuration, traffic summary parsing, and runtime permission testing) has been implemented. This plan focuses on the **remaining integration, documentation, validation, and polish** needed to make Phase 2 usable and maintainable.

**Primary Goals**:
1. Wire the new Phase 2 CLI flags and handler support.
2. Update documentation with practical proxy + permission workflows.
3. Add smoke tests and validation for the new features.
4. Improve finding quality, correlation, and evidence.
5. Ensure consistency with the overall safety and audit model.

**Recommended Timeline**: 1–2 weeks of focused polish work.

---

## 2. Current State Assessment

### What Exists (Post `78436afa...`)
- `traffic.rs`: Solid lenient parser for mitmproxy-style logs and minimal HAR.
- `DynamicMobileArgs`: New fields for `proxy`, `reset_proxy`, `grant_permissions`, `revoke_permissions`, `list_permissions`, `traffic_capture`.
- `DynamicMobileReport`: Now carries `traffic_summary` and `permission_state`.
- `AdbConnection`: Methods for `set_global_proxy`, `clear_global_proxy`, `grant_permission`, `revoke_permission`, `list_permissions`.
- Dry-run and real execution paths updated in `run_dynamic_cli`.
- Report formatting and bridge partially updated.

### What's Still Missing / Needs Polish
| Area | Current State | Needed Work | Priority |
|------|---------------|-------------|----------|
| CLI surface | Internal args updated, but clap definitions and top-level flags not yet exposed | Add clap derives + help text in `cli/mobile.rs` | High |
| Handler integration | Handler knows about dynamic, but new Phase 2 fields not mapped | Extend `commands/handlers/mobile.rs` mapping | High |
| Documentation | Phase 1 docs are excellent; Phase 2 barely mentioned | Add proxy workflow, permission testing examples, and troubleshooting | High |
| Smoke / E2E tests | Only unit tests in `traffic.rs` and `dynamic.rs` | Add documented smoke test script extension or new test for proxy/permission paths | High |
| Finding quality & correlation | Basic traffic findings exist; static ↔ dynamic correlation is weak | Improve correlation helpers and evidence richness | Medium |
| Feature gating decision | Everything under `mobile-dynamic` | Decide whether to keep flat or introduce `mobile-dynamic-advanced` | Medium |
| Polish & edge cases | Good core, but error handling, redaction, and long-running capture can be improved | Targeted polish pass | Medium |

---

## 3. Detailed Next Steps (Prioritized)

### 3.1 High-Priority Polish (Do First)

**Task P2.1: Complete CLI Integration**
- Add clap fields to `DynamicMobileArgs` (or a Phase 2 extension struct) in `cli/mobile.rs`.
- Expose:
  - `--proxy <host:port>`
  - `--reset-proxy`
  - `--grant-permission <perm>` (repeatable)
  - `--revoke-permission <perm>` (repeatable)
  - `--list-permissions`
  - `--traffic-capture <file>`
- Update help text and examples.
- Ensure `--help` for `mobile dynamic` shows the new options cleanly.

**Deliverable**: `eggsec mobile dynamic --help` shows all Phase 2 flags with good descriptions.

**Task P2.2: Handler Mapping & Policy**
- Extend the mapping logic in `commands/handlers/mobile.rs` to pass the new Phase 2 fields from clap args into the internal `DynamicMobileArgs`.
- Ensure policy enforcement still applies correctly (non-dry-run still requires `--allow-dynamic-mobile`).
- Add any new `OperationDescriptor` fields if needed for proxy/permission operations.

**Deliverable**: Full end-to-end CLI → handler → `run_dynamic_cli` flow works for Phase 2 flags.

**Task P2.3: Documentation Update**
- Expand `docs/MOBILE.md` "Dynamic Testing Phases" section with:
  - Practical proxy workflow (how to run mitmproxy + point capture back)
  - Permission testing examples
  - Updated "Phase 2 Success Criteria"
- Add a short "Phase 2 Lab Workflow" subsection.
- Update troubleshooting section with proxy and permission issues.

**Deliverable**: Clear, usable documentation for the new capabilities.

**Task P2.4: Smoke Test Extension**
- Extend or create `scripts/test-mobile-dynamic.sh` (or a new `test-mobile-dynamic-phase2.sh`) that exercises:
  - `--proxy` + `--traffic-capture` path (dry-run + simulated real)
  - Permission grant/revoke/list
  - Report contains `traffic_summary` and `permission_state`
- Make it runnable locally with an emulator.

**Deliverable**: Automated validation that Phase 2 features work end-to-end in dry-run (and optionally real).

### 3.2 Medium-Priority Polish

- **Finding Quality & Correlation**:
  - Improve `static_correlation` usage in traffic and permission findings.
  - Add simple heuristic correlation in `dynamic.rs` or a small `correlation.rs` helper.
  - Enrich evidence in `TrafficSummary` findings (e.g., include domain + path).

- **Redaction & Evidence**:
  - Review and strengthen redaction in `traffic.rs` `sanitize_for_listing`.
  - Consider moving common redaction logic to a shared utility.

- **Error Handling & Robustness**:
  - Make proxy parsing more robust.
  - Handle long traffic captures gracefully (streaming or size limits).
  - Better error messages when `--traffic-capture` file is missing or unreadable.

- **Feature Gating Decision**:
  - Team decision: Keep all Phase 2 under `mobile-dynamic`, or introduce a `mobile-dynamic-advanced` sub-feature for proxy/permission work?
  - Update `Cargo.toml` and feature docs if a split is chosen.

- **Report Polish**:
  - Make `format_dynamic_report` output for traffic/permission sections cleaner and more informative.
  - Ensure bridged `ScanReportData` findings for Phase 2 are useful to downstream consumers.

---

## 4. Recommended Implementation Order

**Week 1 (Integration & Documentation)**
1. CLI clap definitions + help text (P2.1)
2. Handler mapping (P2.2)
3. Documentation updates (P2.3)

**Week 2 (Validation & Polish)**
1. Smoke test extension (P2.4)
2. Finding quality + correlation improvements
3. Redaction / error handling polish
4. Feature gating decision + any Cargo.toml changes
5. Final review + merge

**Parallel Track**: One person can own CLI + handler while another owns docs + smoke test.

---

## 5. Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| CLI surface becomes cluttered | Medium | Group Phase 2 flags under a logical section in help; consider `--proxy-*` namespace if needed. |
| Documentation lags behind code | High | Treat docs update as a first-class task (not an afterthought). |
| Smoke test complexity with real proxy | Medium | Focus smoke test on dry-run + synthetic capture first. Real mitmproxy leg can be optional/manual. |
| Feature split decision paralysis | Low | Default to keeping everything under `mobile-dynamic` for now unless scope grows significantly. |

---

## 6. Handoff Checklist

- [ ] Review this plan with the team and assign owners.
- [ ] Create feature branch `feature/mobile-dynamic-phase2-polish` (or continue on current branch).
- [ ] Prioritize P2.1–P2.4 (CLI, handler, docs, smoke test).
- [ ] Decide on feature gating strategy early in the week.
- [ ] After completion: Run full test suite + new smoke test.
- [ ] Update `docs/MOBILE.md` "Phase 2 Success Criteria" section.
- [ ] Merge and announce Phase 2 availability to the team.

**Immediate Next Action**: Assign owners for CLI integration (P2.1) and documentation (P2.3). These two tasks unblock the rest of the polish work.

---

## 7. References

- Phase 2 foundation commit: `78436afa7ab849d924d93ce84d45e87306191a8a`
- Previous Phase 2 plan: `plans/mobile-dynamic-phase2-implementation-handoff-plan.md`
- Core files: `crates/eggsec/src/mobile/{dynamic.rs, adb.rs, traffic.rs, runtime.rs}`
- CLI: `crates/eggsec/src/cli/mobile.rs`
- Handler: `crates/eggsec/src/commands/handlers/mobile.rs`
- Documentation: `docs/MOBILE.md`

---

**End of Phase 2 Polish & Completion Handoff Plan**