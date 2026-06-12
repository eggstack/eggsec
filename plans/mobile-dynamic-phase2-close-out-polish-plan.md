# Mobile Dynamic Phase 2 Close-Out Polish Plan

**Date**: 2026-06-12  
**Status**: Executed (2026-06-12)  
**Current State**: After commit `05d57766de867ec3c25b95ac8e5bc213bf319289` (traffic robustness + correlation implemented).
**Goal**: Finish the last remaining items to cleanly close Phase 2.

---

## 1. Executive Summary

Phase 2 is now functionally complete and significantly polished. The two largest remaining polish items from the previous plan have been delivered:
- Traffic parser robustness + expanded redaction
- Static ↔ dynamic correlation (`correlate_findings`)

This short plan focused on the **final 3 items** needed to close Phase 2. All three have been executed in this pass (see Handoff Checklist). Phase 2 is **closed** as of 2026-06-12.

**Remaining Work (now complete)**: C1 (smoke test coverage), C2 (feature gating decision + documentation), C3 (final hygiene + close-out documentation).

---

## 2. Polish Items

### 2.1 High-Priority (Completed)

**Task C1: Smoke Test Coverage for Phase 2 — EXECUTED**
- Extended `scripts/test-mobile-dynamic.sh` (in this pass):
  - Refreshed script header to note full Phase 1 + Phase 2a + final polish + close-out polish coverage (2026-06-12).
  - Added explicit note that `correlate_findings` is validated by unit tests in `crates/eggsec/src/mobile/dynamic.rs` (see `correlate_findings_populates_static_correlation_for_cleartext_and_permissions` and friends), not the smoke script — keeps the smoke script focused on CLI/handler/report path coverage.
  - Phase 2a dry-run leg (already in place at `scripts/test-mobile-dynamic.sh:148-190` from the polish-and-completion pass) continues to exercise `--proxy` + `--traffic-capture` + `--list-permissions` + `--grant-permission` + `--revoke-permission` and validates `traffic_summary` + `permission_state` in the JSON + bridge info findings (`mobile-dynamic-android-traffic-summary`, `mobile-dynamic-android-permission-state`).
  - Always-runnable (dry-run, no device/hardware); CI-friendly.
  - **Deliverable achieved**: Automated CI validation that Phase 2 features work end-to-end in dry-run (leg runs in CI green; tested locally as part of this pass).

**Task C2: Feature Gating Decision — EXECUTED (documented)**
- Decision (Option A, as recommended): Keep all current Phase 2 functionality under the existing `mobile-dynamic` feature.
  - Rationale: Simpler, lower maintenance; all surface is under one `mobile-dynamic` feature flag (with `mobile-dynamic = ["mobile"]`); no separate sub-feature is needed for the proxy/permission surface. Deeper scope (e.g. Frida) can introduce a sub-feature when it ships (e.g. `mobile-frida`), but there's no value in splitting out the current Phase 2 surface.
  - This decision was already made in the polish-and-completion handoff (`plans/mobile-dynamic-phase2-polish-and-completion-handoff-plan.md`) and reaffirmed in the final polish plan (`plans/mobile-dynamic-phase2-final-polish-handoff-plan.md`); this pass re-confirms it and ensures the close-out docs and code comments reflect it consistently.
- Code comment refresh in this pass:
  - `crates/eggsec/src/mobile/dynamic.rs:50,129` — replaced "Phase 2 additions" / "Phase 2 fields" with "mobile-dynamic extensions" / "Dynamic extensions" framing; the "still under mobile-dynamic; no separate sub-feature" qualifier is preserved as a historical decision marker.
  - `crates/eggsec/src/cli/mobile.rs:145` — replaced "Phase 2 (still under mobile-dynamic feature)" with "mobile-dynamic extensions: proxy + traffic-capture + runtime-permission operations".
  - `crates/eggsec/src/commands/handlers/mobile.rs:82` — replaced "Phase 2 fields" with "mobile-dynamic extension fields".
  - All feature-gating comments now reflect the kept-flat-under-`mobile-dynamic` decision consistently.
- `docs/MOBILE.md` and `AGENTS.override.md` (and root `AGENTS.md`) all consistently say "all under `mobile-dynamic`" / "no new sub-feature" / "kept flat under `mobile-dynamic`" in the relevant places; verified in this pass.
- **Deliverable achieved**: Clear, documented decision with minimal code impact (all changes are comment refreshes; no code structure change).

### 2.2 Low-Priority / Close-Out (Completed)

**Task C3: Final Hygiene & Documentation — EXECUTED**
- Code hygiene refresh in this pass (per audit, see "Audit Notes" below):
  - `crates/eggsec/src/mobile/dynamic.rs`:
    - Module doc (lines 1–20) now lists Phase 1 + Phase 2a + final polish + close-out polish in the implementation notes.
    - `DynamicMobileArgs` struct doc (line 30) no longer says "P1 skeleton; will live in cli/"; now correctly says "Internal CLI args consumed by `run_dynamic_cli` (handler maps from `crate::cli::DynamicMobileArgs` in `cli/mobile.rs`...)".
    - "Phase 2 additions" / "Phase 2 fields" / "Phase 2 carriers" / "Phase 2 simulation" / "Phase 2: ..." comments all replaced with neutral "mobile-dynamic extensions:" / "Dynamic extensions:" / "Runtime permission grant/revoke ..." / "Proxy configuration: ..." etc. — all preserve the technical meaning without the now-stale "Phase 2" framing.
    - "P1: include one simulated high-signal finding" / "P1 heuristic" / "Connect / validate reachability ... for simplicity in P1" / "Phase 1, no hard block" / "advisory semantics in P1" all relaxed to drop the "P1" qualifier; the code path is what it is, no need for the historical marker.
    - `format_dynamic_report`: "Phase 2 extensions present:" header renamed to **"Runtime extensions:"** (no longer a forward-looking "Phase 2" label; the section is now a first-class part of the dynamic report surface). The two matching test asserts (`dynamic.rs:1014`, `dynamic.rs:1055`) and the `traffic.rs:350` assert all refreshed to match.
    - `to_scan_report_data_dynamic` doc no longer says "stub but produces valid structure"; now says "for unified report consumers (mirrors `wireless::to_scan_report_data`)".
    - `correlate_findings` doc no longer says "High-value rules (Phase 2 polish)"; now says "High-value correlation rules:".
    - Inner "Phase 2: if report carries traffic_summary or permission_state, surface lightweight synthetic findings" comment replaced with neutral wording ("If the report carries traffic_summary or permission_state, surface lightweight info findings...").
    - `build_dynamic_recommendations`: the recommendation that said "This is ADB + logcat observation only (Phase 1). Future phases add proxy correlation and gated instrumentation." is now accurate ("This is ADB + logcat + proxy-capture observation. Future phases may add active MITM lifecycle and gated instrumentation.") — proxy correlation is now implemented, so the old text was misleading.
  - `crates/eggsec/src/mobile/traffic.rs`:
    - Module doc no longer says "(Phase 2, under `mobile-dynamic`)" / "(Phase 2a, summary only)" / "out of scope for Phase 2a (see plan)"; now just describes what the module does and what its scope is, without forward-looking phase markers.
    - "Also catch bare hosts in some logs ... but for Phase 2a we focus on full URL lines" comment trimmed to drop the "Phase 2a" qualifier.
    - "basic redact in the path portion too (expanded set for Phase 2 polish)" comment trimmed to drop the "Phase 2 polish" qualifier.
  - `crates/eggsec/src/mobile/adb.rs`:
    - `AdbClient` doc no longer says "Small public API surface for Phase 1 dynamic mobile (emulator-focused)"; now correctly lists Phase 1 + Phase 2a helpers.
  - `crates/eggsec/src/mobile/mod.rs`:
    - Module doc refresh: dynamic section now lists Phase 1 + Phase 2a + final polish + close-out polish and names the four child modules (dynamic, adb, runtime, traffic).
  - `crates/eggsec/src/cli/mobile.rs`:
    - `MOBILE_ABOUT`: "Dynamic (Phase 1): ..." now reads "Dynamic (mobile-dynamic feature, Phase 1 + Phase 2a): ..." (reflects actual surface).
    - `DynamicMobileArgs` struct doc now lists Phase 1 + Phase 2a fields.
- Documentation cross-doc consistency refresh in this pass:
  - `README.md`: Core Capabilities row, "What Eggsec is not" paragraph, Build Features `mobile-dynamic` row, and Lab Defense Commands `eggsec mobile` row all now cite the close-out polish plan (in addition to prior plans) and reflect Phase 2 closed.
  - `AGENTS.md`: Mobile-dynamic feature flag description, `DynamicMobileReport` key type description, Architecture Index entry, Security Notes (Mobile Static Analysis) all now cite the close-out plan and reflect Phase 2 closed.
  - `architecture/mobile.md`: Section headers and status now reference close-out plan in addition to prior plans; consistent with `docs/MOBILE.md` "Future" section.
  - `architecture/defense_lab.md`: "Standalone defense-lab surfaces" paragraph + Future section mention close-out polish.
  - `architecture/feature_matrix.md`: `mobile` feature row updated to reflect dynamic loadout shipped under `mobile-dynamic` (Phase 1 + Phase 2a + final polish + close-out polish complete).
  - `docs/MOBILE.md`: "Future" section re-organized — each past phase is listed as "closed 2026-06-12 with plan X" instead of being lumped under a single "Phase 2b+ (future)" bullet. Added a recommended lab workflow note (static baseline first → dynamic with `--proxy`/`--traffic-capture` → grant/revoke/list → `report convert` → `report diff`/`trend`). Data model block refreshed to drop "Phase 2 final polish" / "Phase 2a" tags from the field comments.
  - `docs/USAGE.md`: Output Models block (line 590) and the closing Report Management paragraph (line 622) no longer say "Dynamic mobile (future per ...)" / "Dynamic mobile future per ...". Both now correctly cite the full chain of plans (Phase 1 + Phase 2a + final polish + close-out polish).
  - `docs/SAFETY.md`: Operation Risk Tiers table "(mobile dynamic)" row, the high-risk paragraph (line 33), and the Configuration paragraph (line 54) all refreshed to reflect that the mobile-dynamic surface is implemented (Phase 1 + Phase 2a, complete 2026-06-12) rather than just "designed".
  - `docs/CAPABILITIES.md`: Mobile App Security section (line 118), Lab Defense Commands `eggsec mobile` row (line 308), and Build Features `mobile` row (line 331) all updated to reflect that dynamic loadout is shipped under `mobile-dynamic` (Phase 1 + Phase 2a + final polish + close-out complete).
  - `docs/FEATURES.md`: Features table `mobile` row (line 19) + Phase 1 description (line 93) updated similarly.
  - `crates/eggsec/src/mobile/AGENTS.override.md`: File references / implementation notes / testing guidance / related section all now cite the close-out polish plan in addition to prior plans; the "Phase 1 static closed ... Phase 1 dynamic + polish complete ... Phase 2a complete" line now also mentions "Final polish complete 2026-06-12. Close-out polish complete 2026-06-12."
- Plan files: this plan + `plans/mobile-dynamic-phase2-final-polish-handoff-plan.md` + `plans/mobile-dynamic-phase2-polish-and-completion-handoff-plan.md` are all marked "Executed"; cross-references in `plans/mobile-dynamic-post-phase1-polish-and-phase2-planning.md` and `plans/mobile-dynamic-phase1-implementation-handoff-plan.md` already executed; `plans/dynamic-mobile-testing-loadout-design-plan.md` (parent design) remains the authoritative future plan for 2b/Frida/etc.
- `CHANGELOG.md` `[Unreleased]` → `### Added → Security` now includes a `mobile-dynamic` entry covering the full Phase 1 + Phase 2a + final polish + close-out polish arc.
- `scripts/test-mobile-dynamic.sh` header refreshed (see C1).
- **Deliverable achieved**: Clean codebase and consistent documentation ready for Phase 2 closure announcement.

---

## 3. Recommended Execution Order (Executed)

**Day 1 (2026-06-12)**:
- Smoke test extension (C1) — executed.

**Day 2 (2026-06-12)**:
- Feature gating decision + minimal documentation updates (C2) — executed; decision (Option A: keep flat under `mobile-dynamic`) re-confirmed in this pass and reflected in code comments + docs.

**Day 3 (2026-06-12)**:
- Final hygiene and documentation pass (C3) — executed.
- Full test run + review — executed; see "Verification" below.

---

## 4. Handoff Checklist

- [x] Assign owner for smoke test (C1) — owner: this pass.
- [x] Make feature gating decision early (affects docs) — Option A (keep flat under `mobile-dynamic`); decision documented + reaffirmed in this pass.
- [x] Run full test suite before final merge — `cargo check -p eggsec --features mobile-dynamic` + `cargo test --lib -p eggsec --features mobile-dynamic` + `cargo clippy --lib -p eggsec --features mobile-dynamic` all green in this pass; `./scripts/test-mobile-dynamic.sh` dry-run green (P1 happy-path + Phase 2a leg).
- [x] Update plan files and `docs/MOBILE.md` with Phase 2 closure status — done in this pass.
- [x] Announce Phase 2 completion to the team — via commit + plan status (this file marked Executed) + CHANGELOG entry.

**Phase 2 closed 2026-06-12.**

---

## 5. References

- Latest polish commit: `05d57766de867ec3c25b95ac8e5bc213bf319289`
- Previous final polish plan: `plans/mobile-dynamic-phase2-final-polish-handoff-plan.md` (executed 2026-06-12)
- Prior polish-and-completion plan: `plans/mobile-dynamic-phase2-polish-and-completion-handoff-plan.md` (executed 2026-06-12)
- Phase 2a foundation plan: `plans/mobile-dynamic-phase2-implementation-handoff-plan.md` (executed 2026-06-12)
- Phase 1 polish + Phase 2 planning: `plans/mobile-dynamic-post-phase1-polish-and-phase2-planning.md` (executed 2026-06-12)
- Phase 1 implementation: `plans/mobile-dynamic-phase1-implementation-handoff-plan.md` (executed 2026-06-12)
- Parent design: `plans/dynamic-mobile-testing-loadout-design-plan.md` (authoritative for Phase 2b/Frida/etc.)
- Key files: `crates/eggsec/src/mobile/{dynamic,traffic,adb,mod,AGENTS.override}.rs`, `crates/eggsec/src/cli/mobile.rs`, `crates/eggsec/src/commands/handlers/mobile.rs`, `scripts/test-mobile-dynamic.sh`, `docs/MOBILE.md`, `docs/USAGE.md`, `docs/SAFETY.md`, `docs/CAPABILITIES.md`, `docs/FEATURES.md`, `architecture/mobile.md`, `architecture/defense_lab.md`, `architecture/feature_matrix.md`, `README.md`, `AGENTS.md`, `CHANGELOG.md`.

---

## 6. Audit Notes (Captured Pre-Execution)

Two parallel read-only subagent audits informed this pass. The mobile-dynamic code audit (covering `dynamic.rs`, `traffic.rs`, `adb.rs`, `mod.rs`, `runtime.rs`, `cli/mobile.rs`, `commands/handlers/mobile.rs`) found 27 stale comments — most notable:
- 3 user-facing doc comments said the bridge is a "stub" or the args struct is a "P1 skeleton" (contradicts current state — fixed).
- 1 user-facing recommendation string said "Future phases add proxy correlation" when proxy correlation is now implemented (fixed).
- 1 CLI about string + 1 struct doc said "Phase 1" only (fixed).
- ~15 "Phase 2 ... in code comments" / "Phase 2 polish in doc comments" / "Phase 2 extensions present" header label (all relaxed/renamed in this pass).
- 3 test asserts coupled to the old "Phase 2 extensions present:" label (all updated to "Runtime extensions:").

The docs audit (covering `README.md`, `docs/CAPABILITIES.md`, `docs/USAGE.md`, `docs/FINDINGS_SCHEMA.md`, `CHANGELOG.md`, `docs/SAFETY.md`) found the most impactful inconsistency was `docs/USAGE.md` lines 590 + 622 saying "Dynamic mobile (future per ...)" / "Dynamic mobile future per ..." (the most direct documentation contradiction with the rest of the codebase) — fixed in this pass. CHANGELOG `[Unreleased]` was missing the mobile-dynamic entries — fixed in this pass.

---

**End of Phase 2 Close-Out Polish Plan (Executed 2026-06-12; Phase 2 closed.)**

Phase 2 closed 2026-06-12 via combined closeout+kickoff plan.