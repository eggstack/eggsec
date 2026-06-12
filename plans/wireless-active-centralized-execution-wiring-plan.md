# Wireless Active Attacks: Critical Missing Pieces – Centralized Execution & Feedback Wiring Plan

**Date**: 2026-06-12  
**Status**: ✅ Resolved — All items complete (closed 2026-06-12)  
**Focus**: Completing the centralized task execution model and closing the feedback loop

> **Resolution note (2026-06-12)**: This plan was a historical draft describing gaps
> immediately after the centralized `App::handle_enter()` + `EnforcementContext` refactor
> (post `35a2170a`). At the time of resolution (and in `main`), the "critical missing pieces"
> were already satisfied by the shipped implementation (same paths used by Auth/Stress/Packet
> direct_launch tabs):
>
> | Gap (this plan) | Resolution (where it lives) |
> |-----------------|-----------------------------|
> | `TaskBuilder` / `build_current_task()` for `WirelessActive` | `crates/eggsec-tui/src/app/task_management.rs:371` (under `#[cfg(feature = "wireless")]`): `impl TaskBuilder for WirelessTab` returns `TaskConfig::WirelessActive` when `active_mode` + valid `active_attack_config()`. |
> | `TaskRunner` match arm calling the worker | `crates/eggsec-tui/src/workers/runner.rs:652` (under `wireless-advanced`): `TaskConfig::WirelessActive { ... } => super::security::run_wireless_active_task(...)`. |
> | `EnforcementContext` policy check for active wireless | `crates/eggsec-tui/src/app/mod.rs:371` (retro gate for `is_direct_launch_tab`) + `build_current_operation_descriptor:436` (wireless-active override: `OperationMode::DefenseLab`, risk `SafeActive` for dry-run or `Intrusive` for live, `required_features: ["wireless-advanced"]`) + central `self.enforcement.evaluate(&desc)`. Dry-run proceeds; live triggers `RequireConfirmation` overlay. |
> | Result delivery from worker to `WirelessTab` | `crates/eggsec-tui/src/app/state_update.rs:418` (under `wireless && wireless-advanced`): `TaskResult::WirelessActive(r) => self.tabs.wireless.set_active_results(r)`. `set_active_results` renders + transitions to `Completed`. |
> | Wiring `start_active_attack()` into `App::handle_enter()` | `WirelessTab::handle_enter()` (active config focus) calls `start_active_attack()` (UI-only: sets `Running`, clears prior results). `App::handle_enter()` then sees `is_direct_launch_tab + running` and drives the descriptor/enforcement/spawn path. `start_active_attack` comment in wireless.rs:372 documents the design. |
>
> Additional artifacts:
> - Worker implementation: `crates/eggsec-tui/src/workers/security.rs:865` (`run_wireless_active_task`; 60s timeout, hard budgets on frames/rate, dispatches deauth/disassoc).
> - TabSpec: `crates/eggsec-tui/src/tabs/spec.rs:438` declares `direct_launch: true` for Wireless (risk overridden at descriptor time).
> - Unit/E2E-style tests: 12+ tests in `tabs/wireless.rs` (under `wireless-advanced`) + 2 descriptor tests in `app/mod.rs` + the `test_e2e_active_flow_handle_enter_build_task_set_results` flow test.
> - Compile/test status at closure: `cargo check -p eggsec-tui --features wireless,wireless-advanced` clean (pre-existing unrelated warnings only); `cargo test --lib -p eggsec-tui --features wireless,wireless-advanced` → 323 passed.
>
> Sibling plans (`wireless-active-tui-execution-completion-plan.md`, `wireless-active-tui-execution-closure-plan.md`, `wireless-active-tui-final-wiring-and-polish-plan.md`) document the same wiring from slightly different angles and were closed with similar notes on 2026-06-12. Vestigial handler files referenced in early drafts were never mod-declared and are absent.
>
> See `architecture/wireless.md`, `architecture/tui.md` ("Wireless tab Active Mode"), `docs/WIRELESS.md`, AGENTS.md (TUI Wireless Active Execution Completion + standalone defense-lab), `.opencode/skills/eggsec-agent/wireless_security_testing.md`, and `crates/eggsec/src/wireless/AGENTS.override.md` + `crates/eggsec-tui/src/AGENTS.override.md` for the final design. Wireless remains a standalone defense-lab surface (no MCP/agent exposure).
>
> This plan is retained as a historical artifact. All checklist items are satisfied by the implementation present in `main`. No code changes were required for resolution; verification + doc alignment only.

---

---

## Executive Summary

After commit `35a2170a6d0d990e5e40f8934e51873fbe4b7c82`, the architecture has shifted to a cleaner centralized model:

- `App::handle_enter()` is now responsible for task building, policy enforcement, and spawning `TaskRunner`.
- `EnforcementContext` handles safety/policy checks.
- Results are intended to flow through `state_update.rs` → `WirelessTab::set_active_results()`.

However, the **critical wiring for `WirelessActive`** is still missing:

- No `TaskBuilder` implementation or `build_current_task()` support for `WirelessActive`.
- No `TaskRunner` match arm that calls `handle_wireless_active_task()`.
- No actual `EnforcementContext` policy check for non-dry-run attacks.
- Result delivery path from worker to tab is not connected.

This plan provides detailed, surgical guidance to finish the centralized execution path.

---

## Current Architecture Context (Post-35a2170a)

The intended flow is now:

```
User presses Enter in ActiveConfig
    → WirelessTab::handle_enter() → start_active_attack() (UI state only)
    → App::handle_enter() detects Running + Active mode
    → build_current_task() (uses TaskBuilder trait)
    → EnforcementContext::check(...)  (policy / confirmation)
    → TaskRunner::spawn(task) if allowed
    → Worker executes handle_wireless_active_task()
    → Result delivered via state_update.rs
    → WirelessTab::set_active_results(result)
```

**What exists**:
- `handle_wireless_active_task()` in `wireless_active_handler.rs`
- `set_active_results()` and result rendering in the tab
- New E2E-style test skeleton

**What is missing**:
- The centralized pieces above for `WirelessActive`

---

## Critical Missing Pieces

| # | Missing Piece | Impact | Files to Modify | Difficulty |
|---|---------------|--------|------------------|------------|
| 1 | `TaskBuilder` impl / `build_current_task()` for `WirelessActive` | Tasks never get created in the new model | `app/task_management.rs` or equivalent | Medium |
| 2 | `TaskRunner` match arm for `WirelessActive` | `handle_wireless_active_task()` is never called | `workers/task_runner.rs` or main worker loop | Medium |
| 3 | `EnforcementContext` policy check for active wireless | No safety guard for real attacks | `app/enforcement.rs` or policy module | Medium-High |
| 4 | Result delivery from worker to `WirelessTab` | User never sees results | `state_update.rs` + tab communication | High |
| 5 | Wiring `start_active_attack()` into `App::handle_enter()` | Flow is broken at the App layer | `app/mod.rs` or main app event handling | Medium |

---

## Detailed Surgical Tasks

### Task 1: Implement `TaskBuilder` Support for `WirelessActive`

**Goal**: Make `build_current_task()` able to produce a `WirelessActive` task.

In the file that implements `TaskBuilder` for tabs (likely `app/task_management.rs` or a trait impl):

```rust
impl TaskBuilder for WirelessTab {
    fn build_current_task(&self) -> Option<TaskConfig> {
        if self.active_mode {
            return self.build_task_config().map(TaskConfig::WirelessActive);
        }
        // fall back to passive scan task if not in active mode
        ...
    }
}
```

Ensure `build_task_config()` remains public and returns `Option<TaskConfig::WirelessActive>`.

### Task 2: Add `TaskRunner` Match Arm

**File**: Worker / `TaskRunner` implementation (search for existing `match task` or `TaskRunner`).

```rust
match task {
    TaskConfig::WirelessActive { interface, attack_type, bssid, client, frame_count, rate_limit, dry_run } => {
        let result = handle_wireless_active_task(
            interface, attack_type, bssid, client, frame_count, rate_limit, dry_run
        ).await;

        // Send result back via state update channel
        state_update_tx.send(StateUpdate::ActiveWirelessResult(result)).await?;
    }
    ...
}
```

### Task 3: Wire Policy Enforcement (`EnforcementContext`)

**Location**: `App::handle_enter()` or the task spawning logic.

Before spawning the `TaskRunner`:

```rust
if let Some(task) = tab.build_current_task() {
    let context = EnforcementContext {
        operation: Operation::WirelessActive { dry_run: task.dry_run },
        risk: if task.dry_run { OperationRisk::Low } else { OperationRisk::Intrusive },
        ...
    };

    if !enforcement_context.allow(context) {
        // show confirmation overlay or deny
        return;
    }

    task_runner.spawn(task).await;
}
```

Reuse or extend the existing `EnforcementContext` system used by other intrusive operations.

### Task 4: Implement Result Delivery Path

**File**: `state_update.rs` (or equivalent state update module).

Add a variant:

```rust
pub enum StateUpdate {
    ...
    ActiveWirelessResult(ActiveWirelessAttackResult),
}
```

In the main app update loop, match on it and call:

```rust
if let Some(wireless_tab) = app.get_wireless_tab_mut() {
    wireless_tab.set_active_results(result);
}
```

### Task 5: Connect `start_active_attack()` to `App::handle_enter()`

Ensure that when `WirelessTab` is in `ActiveConfig` mode and Enter is pressed:

- `start_active_attack()` only sets UI state to `Running`
- `App::handle_enter()` then picks up the `Running` state + `active_mode` flag
- Proceeds with `build_current_task()` + policy + execution

This is mostly wiring in the main app event handler.

---

## Recommended Implementation Order

1. **Task 1 + Task 5** — Make `build_current_task()` work and connect `App::handle_enter()` (unblocks the flow)
2. **Task 2** — Add the `TaskRunner` match arm (enables actual execution)
3. **Task 3** — Add `EnforcementContext` policy check (safety)
4. **Task 4** — Wire result delivery via `state_update.rs` (closes the loop)
5. **Polish & Test** — Expand the existing E2E test to cover the full centralized path

---

## Handoff Checklist

- [ ] Implement `TaskBuilder` / `build_current_task()` support for `WirelessActive`
- [ ] Wire `App::handle_enter()` to detect Active mode + Running state
- [ ] Add `TaskRunner` match arm that calls `handle_wireless_active_task()`
- [ ] Integrate `EnforcementContext` policy check before spawning TaskRunner
- [ ] Add `StateUpdate::ActiveWirelessResult` variant and delivery logic
- [ ] Update the existing E2E test to exercise the full `App → TaskRunner → result` path
- [ ] Verify dry-run works end-to-end from TUI
- [ ] Verify non-dry-run triggers policy confirmation (if implemented)

---

## Open Questions

1. Where exactly is `build_current_task()` and the `TaskBuilder` trait implemented?
2. Where is the main `TaskRunner` / worker match statement located?
3. How does `state_update.rs` currently deliver messages to specific tabs?
4. Is `EnforcementContext` already used for other wireless or intrusive operations we can model after?

---

**End of Plan**