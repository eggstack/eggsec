# Wireless Active Attacks: Final Dispatcher Wiring, Feedback Loop & Polish Plan

**Date**: 2026-06-12  
**Status**: ✅ Resolved — All items complete (closed 2026-06-12)  
**Focus**: Closing the last gaps to make TUI active attacks fully functional

> **Resolution note (2026-06-12)**: This plan was drafted against a stale view
> of the codebase (proposing a separate `wireless_active_handler` + `register_*`
> + direct `TaskBuilder::new().build()` submission path). At the time of
> resolution, the TUI active attack execution path was already fully wired
> end-to-end via the main runner + security worker + central state update
> (consistent with other `direct_launch` tabs like Auth/Stress/Packet). The
> four "remaining gaps" were already satisfied by shipped code:
>
> | Gap (this plan) | Resolution (where it lives) |
> |-----------------|-----------------------------|
> | Dispatcher match arm for `WirelessActive` | `crates/eggsec-tui/src/workers/runner.rs:652-658` (cfg-gated arm calls `super::security::run_wireless_active_task`). |
> | Result feedback to `WirelessTab` | `crates/eggsec-tui/src/app/state_update.rs:418-422` routes `TaskResult::WirelessActive(r) => self.tabs.wireless.set_active_results(r)`. |
> | Policy confirmation overlay | `crates/eggsec-tui/src/app/mod.rs:436-471` (wireless-active special case in `build_current_operation_descriptor` for `SafeActive` dry-run vs. `Intrusive` live under `DefenseLab`); retro gate + `EnforcementContext::evaluate` + `PendingPolicyConfirmation` for direct_launch tabs; dry-run proceeds without overlay. |
> | End-to-end test | Unit coverage already present (active_attack_config variants, `build_task_config_returns_wireless_active_variant`, `set_active_results_renders_and_completes`, `start_active_attack`/`handle_enter` transitions, descriptor risk tests). Additional E2E-style test `test_e2e_active_flow_handle_enter_build_task_set_results` added during this pass (wireless.rs:1089+) chaining handle_enter → build_task_config (WirelessActive) → simulated result → set_active_results + assertions on state/content. |
>
> Cleanup performed in this pass (per "vestigial" analysis):
> - Removed unused planning artifacts: `crates/eggsec-tui/src/workers/wireless_active_handler.rs` and `crates/eggsec-tui/src/workers/dispatcher_wiring_example.rs` (never mod-declared in workers/mod.rs; real path is runner/security/state_update).
> - Fixed dead code in `start_active_attack()` (wireless.rs): replaced `let _task = crate::app::task_management::TaskBuilder::new(task_config).build();` (invalid; TaskBuilder is a trait) with a clarifying comment explaining the `direct_launch` + App-driven spawn + policy gate path.
>
> The actual implementation followed the tighter parallel path also documented in the sibling closed plans (`wireless-active-tui-execution-completion-plan.md`, `wireless-active-tui-execution-closure-plan.md`, `wireless-active-tui-execution-final-polish-plan.md`).
>
> Test status at resolution of this plan:
> - `eggsec-tui` lib (`wireless-advanced`): 323 passed, 0 failed (includes the new E2E-style test).
> - All `cargo check` targets for tui + core crates (with/without wireless-advanced): green.
> - Pre-existing clippy warnings: unchanged.
>
> See `architecture/wireless.md`, `docs/WIRELESS.md`, `architecture/tui.md` ("Wireless tab Active Mode"), `.opencode/skills/eggsec-agent/wireless_security_testing.md`, `crates/eggsec/src/wireless/AGENTS.override.md`, and AGENTS.md (TUI Wireless Active Execution Completion + standalone defense-lab surfaces sections) for the shipped design. Wireless remains a standalone defense-lab surface (no MCP/agent tool exposure).
>
> This plan is retained as a historical artifact. All checklist items are satisfied by the combination of prior commits + the targeted cleanup + new test in this pass.

---

## Executive Summary (Historical Draft Context)

Good progress had been made at the time of drafting:

- TUI now submits `TaskConfig::WirelessActive` tasks via `start_active_attack()` + `TaskBuilder`.
- `handle_wireless_active_task()` handler exists and correctly calls `run_deauth()`.
- Registration helper and dispatcher wiring example are available.

**Items listed as remaining at draft time** (all subsequently verified as already complete via the runner/security/state_update path, or addressed via cleanup + test in the resolution pass):

1. Insert the `WirelessActive` match arm into the main task dispatcher.
2. Wire result feedback from the handler back to `WirelessTab::set_active_results()`.
3. Add policy confirmation for non-dry-run attacks.
4. Add at least one end-to-end style test.

This plan provided a focused, surgical path; the shipped solution used a tighter architecture-aligned implementation.

---

## Remaining Gaps (Historical — All Resolved at Draft Time + Cleanup/Test Pass)

| Gap | Impact | Priority | Resolution (actual shipped locations) |
|-----|--------|----------|---------------------------------------|
| Dispatcher match arm for `WirelessActive` | Tasks are submitted but never executed | Critical | Already present: `workers/runner.rs:652-658` (calls `security::run_wireless_active_task`). The separate handler + register approach in the draft was never adopted. |
| Result feedback to `WirelessTab` | User never sees attack results | High | Already present: `app/state_update.rs:418-422` (`TaskResult::WirelessActive(r) => self.tabs.wireless.set_active_results(r)`). Uses the shared result channel (same as passive wireless / auth / stress). |
| Policy confirmation overlay | No safety guard for real attacks | High | Already present: descriptor special-casing in `app/mod.rs:436-471` (SafeActive for dry-run, Intrusive for live under DefenseLab); direct_launch TabSpec + retro gate + central `EnforcementContext::evaluate()` + `PendingPolicyConfirmation` overlay. Dry-run (default) proceeds without prompt. |
| End-to-end test | No verification of full flow | Medium | Unit coverage pre-existed (config, task build, set_results, handle_enter/start transitions, descriptor risks). New E2E-style test `test_e2e_active_flow_handle_enter_build_task_set_results` added (wireless.rs:1089+) exercising handle_enter in ActiveConfig → build_task_config (WirelessActive) → simulated result → set_active_results + state/content assertions. |

---

## Surgical Tasks (Historical Draft — Implementation Path Diverged)

The draft proposed a separate `wireless_active_handler` + register hook + direct TaskBuilder submission. The shipped implementation used the existing runner match arm + security worker (consistent with other direct_launch tabs) + central state_update routing + App-level policy gates. The vestigial handler/example files were removed in the resolution pass; the dead `TaskBuilder::new(...).build()` line was replaced by a clarifying comment. One new E2E-style test was added. No dispatcher changes or new handler registration were required.

### 1. Insert Dispatcher Match Arm (Critical) — Already Complete
See `workers/runner.rs:652-658` (and `security.rs:865-927` for the worker). No separate handler registration is used.

### 2. Implement Result Feedback Loop (High Priority) — Already Complete
See `app/state_update.rs:418-422` + `WirelessTab::set_active_results` + `update_active_results_view`.

### 3. Add Policy Confirmation (High Priority) — Already Complete
Descriptor risk override + App gates + overlay machinery (no per-tab local confirm in WirelessTab).

### 4. Add One End-to-End Test (Medium) — Completed in Resolution Pass
New test `test_e2e_active_flow_handle_enter_build_task_set_results` (plus pre-existing coverage).

### 5. Minor Polish (Low) — Addressed
- Error messages in the worker already include "Active wireless attack failed: ..." and timeout cases.
- `docs/WIRELESS.md` and architecture docs already document TUI active usage (a/d/Enter, dry-run default, overlay for live, TaskConfig path, set_active_results).
- Progress updates (0/2 → 2/2) are sent by the worker.

---

## Recommended Implementation Order (Historical)

1. Insert the dispatcher match arm (unblocks execution) — already present.
2. Wire result feedback to the tab (makes feature usable) — already present.
3. Add policy confirmation (safety) — already present.
4. Add one end-to-end test — added in this pass.
5. Polish + docs — docs were already accurate; minor cleanup performed.

---

## Handoff Checklist (All Items Satisfied)

- [x] Add `TaskConfig::WirelessActive` match arm in main dispatcher (present in runner.rs)
- [x] Call `register_wireless_active_handler()` at startup (N/A — vestigial planning artifact removed; real wiring is static match)
- [x] Implement result delivery mechanism to `WirelessTab` (state_update.rs routing)
- [x] Add policy confirmation before non-dry-run task submission (descriptor + App gates + overlay)
- [x] Write one integration-style test for the full flow (new E2E test added; prior unit coverage existed)
- [x] Verify dry-run works end-to-end from TUI (tests + default behavior)
- [x] Update `docs/WIRELESS.md` with TUI usage note (already present and accurate)

---

## Open Questions (Historical)

1. What is the preferred mechanism for sending results/errors from workers back to specific tabs? (Event bus, channels, shared state?)  
   **Answer (as implemented)**: Single shared `mpsc` result channel on App + central `process_task_result` in state_update.rs (same pattern for wireless passive, auth, stress, packet, etc.). No per-tab TabMessage bus exists or was needed.

2. Should policy confirmation be handled in the tab or centralized in the task submission layer?  
   **Answer (as implemented)**: Centralized in the App (`handle_enter` + `build_current_operation_descriptor` + `request_policy_confirmation`) for consistency across direct_launch tabs; tab only manages local UI state for the launch intent.

---

**End of Plan** (resolved 2026-06-12; see resolution note at top)