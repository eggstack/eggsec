# Wireless Active Attacks: Surgical TUI Execution Closure Plan

**Date**: 2026-06-12
**Status**: ✅ Resolved — All items complete (closed 2026-06-12)
**Goal**: Complete the remaining execution wiring so active deauth works from the TUI.

> **Resolution note (2026-06-12)**: This plan was drafted against a stale view
> of the codebase. All five "remaining gaps" called out below were already
> implemented in `main` at the time of resolution (committed 2026-06-12 in
> `8b45cdd2` / `3e2b4291` / `87b23b9c` / `39943bea`). The TUI active attack
> execution path is fully wired end-to-end:
>
> | Gap (this plan) | Resolution (where it lives) |
> |-----------------|-----------------------------|
> | `start_active_attack()` only sets UI state | Intentional — wireless tab is `direct_launch: true` (`crates/eggsec-tui/src/tabs/spec.rs:438`); the App's retro-gate at `crates/eggsec-tui/src/app/mod.rs:371-398` handles task submission, and `start_active_attack()` only flips the local `AppState`. The same pattern is used by Auth / Stress / Packet / Hunt / Browser tabs. |
> | No worker for `TaskConfig::WirelessActive` | `crates/eggsec-tui/src/workers/security.rs:865-927` (`run_wireless_active_task`) — dispatches to `run_deauth` / `run_disassoc` with 60s timeout and hard budgets (`max_frames ≤ 1000`, `frames_per_second ≤ 100`). |
> | `run_deauth()` never called from the TUI | Dispatched inside the worker above; `attack_type` selects `run_deauth` (default) vs. `run_disassoc`. |
> | No policy confirmation for non-dry-run attacks | `crates/eggsec-tui/src/app/mod.rs:436-471` special-cases the wireless descriptor to `OperationRisk::Intrusive` under `OperationMode::DefenseLab` for live attacks; the central `EnforcementContext::evaluate()` + `PendingPolicyConfirmation` flow applies. Dry-run is `OperationRisk::SafeActive` and proceeds without the confirmation overlay. |
> | No result feedback from worker back to `WirelessTab` | `crates/eggsec-tui/src/app/state_update.rs:418-422` routes `TaskResult::WirelessActive(result)` → `WirelessTab::set_active_results(result)`, which transitions the tab to `AppState::Completed` and renders findings / evidence / recommendations. |
>
> See `architecture/wireless.md`, `docs/WIRELESS.md`, `architecture/tui.md`
> ("Wireless tab Active Mode"), `.opencode/skills/eggsec-agent/wireless_security_testing.md`,
> and `crates/eggsec/src/wireless/AGENTS.override.md` for the shipped design.
> This plan is retained as a historical artifact documenting the design
> intent; the actual implementation followed a tighter parallel path
> (see `wireless-active-tui-execution-completion-plan.md` and
> `wireless-active-tui-execution-final-polish-plan.md`, both already closed).
>
> Test status at resolution:
> - `eggsec` lib (`wireless-advanced`): 1579 passed, 0 failed
> - `eggsec-tui` lib: 305 passed, 0 failed
> - `eggsec-tui` lib (`wireless-advanced`): 322 passed, 0 failed
> - All `cargo check` targets (`eggsec`, `eggsec-tui`, `eggsec-cli`, `eggsec-nse`,
>   `eggsec-output`, with and without `wireless-advanced`): green
> - Pre-existing clippy warnings: unchanged (8 in `eggsec` lib, 55 in
>   `eggsec-tui` lib with `wireless-advanced`; no new warnings introduced)

---

## Executive Summary

The TUI has excellent scaffolding:
- `active_mode`, input fields, focus handling, `handle_enter()`, `build_task_config()`, and `set_active_results()` are all implemented and tested.
- `TaskConfig::WirelessActive` variant exists.

**What is still missing** (these were not completed in recent commits):
1. `start_active_attack()` only sets UI state — it does **not** submit a task.
2. No worker/handler exists for `TaskConfig::WirelessActive`.
3. `run_deauth()` is never called from the TUI path.
4. No policy confirmation for non-dry-run attacks.
5. No result feedback from worker back to `WirelessTab`.

This plan is intentionally **tight and surgical** — it targets only the missing pieces with precise file locations and steps.

---

## Precise Remaining Gaps

| Gap | Current Behavior | Required Behavior | File(s) to Change |
|-----|------------------|-------------------|-------------------|
| Task submission | `start_active_attack()` only sets `AppState::Running` | Must create and submit a `WirelessActive` task | `crates/eggsec-tui/src/tabs/wireless.rs` |
| Worker handler | No handler for `WirelessActive` | Must call `run_deauth()` and return result | Worker module (likely `crates/eggsec-tui/src/workers/`) |
| Policy confirmation | None | Show overlay before non-dry-run tasks | TUI tab + app confirmation system |
| Result callback | `set_active_results()` exists but is never called from execution | Worker must invoke it with real result | Worker + tab communication layer |
| Error path | No error propagation from worker | Worker must call `set_error()` on failure | Worker + tab |

---

## Surgical Implementation Steps

### Step 1: Modify `start_active_attack()` to Submit a Task

**File**: `crates/eggsec-tui/src/tabs/wireless.rs`

In `start_active_attack()`:

```rust
#[cfg(feature = "wireless-advanced")]
pub fn start_active_attack(&mut self) {
    if let Some((interface, attack_type, bssid, client, frame_count, rate_limit, dry_run)) = self.active_attack_config() {
        self.state = AppState::Running;
        self.progress.current = 0;
        self.active_results = None;
        self.results_view.clear();
        self.error = None;

        // NEW: Build and submit task
        if let Some(task_config) = self.build_task_config() {
            // Submit via existing TaskBuilder / task management system
            // Example pattern (adjust to actual API):
            // let task = TaskBuilder::new(task_config).build();
            // submit_task(task);
        }
    }
}
```

Also ensure `build_task_config()` is public or accessible.

### Step 2: Implement `WirelessActive` Task Handler

**Location**: Worker module (search for existing task handlers or `match task_config`)

Create a handler that matches:

```rust
TaskConfig::WirelessActive { interface, attack_type, bssid, client, frame_count, rate_limit, dry_run } => {
    // Build ActiveAttackConfig from parameters
    let config = ActiveAttackConfig { ... };

    let result = if dry_run {
        // Return simulated or early result
    } else {
        crate::wireless::active::attacks::deauth::run_deauth(&config, broadcast).await?
    };

    // Send result back to tab
    // (see Step 4)
}
```

### Step 3: Add Policy Confirmation

Before submitting a non-dry-run task in `start_active_attack()` or `handle_enter()`:

- Check `!dry_run`
- Trigger the existing TUI confirmation overlay system
- Only proceed if user confirms
- Pass confirmation reason if the system supports it

### Step 4: Implement Result Feedback

Define how the worker sends `ActiveWirelessAttackResult` back to the specific `WirelessTab`:

Options (choose one consistent with existing patterns):
- App-level event / callback channel
- Direct method call via tab reference
- Shared state update that the tab polls

Once received, call:
```rust
ttab.set_active_results(result);
```

### Step 5: Add Error Handling Path

In the worker handler:
- On any error, call the tab’s error path:
  ```rust
  tab.set_error(TabError::new(...));
  ```

### Step 6: Add Minimal End-to-End Test

Add one test that exercises:
- `handle_enter()` in ActiveConfig → task submission → (mocked) worker result → `set_active_results()` called.

---

## Files to Modify (Surgical List)

| File | Changes | Priority |
|------|---------|----------|
| `crates/eggsec-tui/src/tabs/wireless.rs` | Update `start_active_attack()` to submit task + add policy check | Critical |
| Worker handler file (TBD) | Implement `WirelessActive` match arm + call `run_deauth` | Critical |
| Task communication layer | Add result/error callback mechanism | High |
| Test file for wireless tab | Add one integration-style test | Medium |

---

## Handoff Checklist (Specific)

- [ ] Update `start_active_attack()` to actually submit a `WirelessActive` task
- [ ] Implement handler for `TaskConfig::WirelessActive` that calls `run_deauth()`
- [ ] Add policy confirmation before non-dry-run task submission
- [ ] Wire result callback so worker can call `set_active_results()`
- [ ] Wire error path so worker can call `set_error()`
- [ ] Add one end-to-end test for the active attack flow
- [ ] Verify dry-run works from TUI without requiring root

---

## Suggested Order (Surgical)

1. Modify `start_active_attack()` to submit the task (quick win)
2. Implement the `WirelessActive` worker handler + `run_deauth` call
3. Add policy confirmation guard
4. Implement result + error feedback
5. Add one test

This order lets you get a working (dry-run first) flow quickly.

---

## Open Questions

1. What is the exact API for submitting tasks from the tab? (Is it `TaskBuilder`, a global submit function, or something else?)
2. How should the worker communicate results back to a specific tab instance?
3. Should policy confirmation live in the tab or be handled by the task system before execution?

---

**End of Surgical Follow-up Plan**