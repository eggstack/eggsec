# Wireless Active Attacks: TUI Execution Completion Plan

**Date**: 2026-06-11  
**Status**: ✅ Complete (2026-06-12)  
**Focus**: Completing the execution path for active attacks from the TUI

---

## Executive Summary

Significant UI scaffolding for active attacks now exists in the Wireless TUI tab:

- `active_mode` toggle (`a` key)
- Input fields for BSSID, Client MAC, Frame Count, and Rate Limit
- `dry_run` toggle (`d` key)
- `ActiveWirelessAttackResult` display support
- Focus handling and rendering for Active Configuration section

However, **pressing Enter while in Active Configuration mode does not yet execute the deauth attack**. The TUI can collect parameters and display results, but the actual execution flow is not wired.

This plan provides a focused, actionable roadmap to complete the **TUI execution path** for Phase 1 (deauth).

**Goal**: Allow users to configure and trigger deauth attacks directly from the TUI (with proper policy confirmation and dry-run support), while keeping the implementation clean and consistent with existing TUI architecture.

---

## Current TUI State

### What Exists
- `WirelessTab` has conditional `#[cfg(feature = "wireless-advanced")]` fields:
  - `active_mode: bool`
  - `active_inputs: InputGroup`
  - `dry_run: bool`
  - `active_results: Option<ActiveWirelessAttackResult>`
- `active_attack_config()` method that extracts parameters
- `set_active_results()` and `update_active_results_view()`
- Rendering logic for the Active Attack Configuration block
- Focus navigation between Inputs → ActiveConfig → Results
- Keyboard handling for toggling modes

### What Is Missing for Execution
- `handle_enter()` in `ActiveConfig` mode only blurs the inputs — it does not trigger execution.
- No worker/task is launched to call `crate::wireless::active::attacks::deauth::run_deauth`.
- No integration with the TUI's policy preflight / confirmation system for high-risk actions.
- The passive results view still shows: *"Active attacks are available via CLI only."*
- No state management for "Running active attack" vs "Completed".

---

## Recommended Approach

Follow the existing TUI patterns used for other long-running or high-risk operations:

1. Use the existing worker system (or a lightweight async task) to run the attack in the background.
2. Go through the TUI's policy confirmation flow before execution (critical for `Intrusive` risk operations).
3. Support `dry_run` natively (no confirmation needed for dry runs).
4. Update UI state properly (`AppState::Running` → `Completed` or `Error`).
5. Reuse `set_active_results()` for final output.

---

## Implementation Tasks

### 1. Modify `handle_enter()` in `WirelessTab`

**File**: `crates/eggsec-tui/src/tabs/wireless.rs`

When `focus_area == WirelessFocusArea::ActiveConfig` and `active_mode == true`:
- If `dry_run == true`: Execute immediately (no confirmation).
- If `dry_run == false`: Trigger policy confirmation overlay first.
- On confirmation (or dry run): Call a new method `start_active_attack()`.

### 2. Add `start_active_attack()` Method

Create a method that:
- Extracts config via `active_attack_config()`.
- Sets `state = AppState::Running`.
- Launches the actual deauth execution (via worker or async task).
- Handles the result by calling `set_active_results()` or setting an error.

### 3. Integrate with TUI Worker / Task System

Options (choose one consistent with project patterns):
- Extend an existing worker (e.g., security worker).
- Create a lightweight dedicated task using tokio.
- Use the app-level command execution path if available.

The worker should call:
```rust
crate::wireless::active::attacks::deauth::run_deauth(&config, broadcast).await
```

### 4. Add Policy Confirmation Flow

Before executing a non-dry-run attack:
- Use the existing `PendingPolicyConfirmation` / overlay system.
- Show a clear warning: "This will transmit deauthentication frames."
- Require explicit confirmation.
- Record the action for audit (if the system supports it).

### 5. Update UI State & Feedback

- Show progress indicator while attack is running.
- On completion: Switch to Results view and display `active_results`.
- On error: Set error state with clear message.
- Update the passive results note once TUI execution is fully working (remove or soften the "CLI only" message).

### 6. Keyboard / UX Polish

- Consider adding a dedicated key (e.g., `Ctrl+Enter` or `x`) to trigger active attack directly from ActiveConfig.
- Ensure `dry_run` toggle is clearly visible in the UI title.

---

## Key Files to Modify

| File | Changes |
|------|---------|
| `crates/eggsec-tui/src/tabs/wireless.rs` | Main logic: `handle_enter`, new `start_active_attack()`, state handling |
| `crates/eggsec-tui/src/app/` (likely) | Policy confirmation overlay integration |
| `crates/eggsec-tui/src/workers/` | Optional: New or extended worker for active attack execution |
| `docs/WIRELESS.md` | Minor update to TUI section once complete |

---

## Handoff Checklist

- [x] Implement `handle_enter()` logic for ActiveConfig mode
- [x] Create `start_active_attack()` method
- [x] Wire background execution (worker or async task)
- [x] Integrate policy confirmation for non-dry-run attacks
- [x] Handle success / error states properly
- [x] Update passive results view note (remove "CLI only" message)
- [x] Test dry-run and real execution paths from TUI
- [x] Add basic TUI test coverage if time permits

## Completion Notes (2026-06-12)

All plan items shipped. See commit log and AGENTS.md "TUI Wireless Active Execution Completion (2026-06-12)" for the implementation summary.

---

## Suggested Minimal Scope for First Pass

For a focused handoff:
1. Get dry-run execution working from TUI (lower risk).
2. Add confirmation overlay for real execution.
3. Wire basic success/error feedback.

Full robustness and advanced UX can come in a follow-up iteration.

---

## Open Questions

1. Should active attack execution reuse the existing command execution infrastructure, or run directly via the library?
2. How should broadcast vs targeted deauth be selected in the TUI? (Currently only via input fields — is a checkbox better?)
3. Do we want to support monitor interface selection in the TUI, or assume the user provides the correct interface name?
4. Should we show a live frame counter during execution, or just a simple progress indicator?

---

**End of Focused TUI Execution Plan**