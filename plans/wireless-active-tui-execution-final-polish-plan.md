# Wireless Active Attacks: Final TUI Execution + Polish Plan

**Date**: 2026-06-11  
**Status**: Final Draft — Ready for Handoff  
**Goal**: Close the remaining gaps to make active deauth fully usable from both CLI and TUI.

---

## Executive Summary

Good progress has been made across the active wireless loadout:

- Core deauth logic, CLI, handler, and reporting bridge are solid.
- TUI now has active mode UI, input collection, `start_active_attack()`, and improved messaging.

**Remaining work is focused and limited**:
1. Wire actual execution from the TUI (`start_active_attack()` → real `run_deauth` call).
2. Add policy confirmation for non-dry-run attacks.
3. Minor polish and test improvements.

This plan keeps scope tight so the feature can reach a complete, usable state for Phase 1.

---

## Current State Summary

| Component                    | Status          | Notes |
|-----------------------------|-----------------|-------|
| Core deauth + frame injection | Complete       | Functional |
| CLI (`deauth` subcommand)   | Complete       | Good |
| Handler + Policy gating     | Complete       | `--allow-active-wireless` works |
| Reporting Bridge            | Complete       | Wired into `report convert` |
| TUI UI scaffolding          | Good           | Inputs, rendering, `start_active_attack()` exist |
| **TUI Execution**           | **Partial**    | UI goes to Running state but does not call `run_deauth` |
| Policy confirmation (TUI)   | Missing        | Needed for non-dry-run |
| Integration tests           | Partial        | Some TUI unit tests added |

---

## Prioritized Remaining Tasks

### 1. Wire Real Execution from TUI (Highest Priority)

**Goal**: When user presses Enter in Active mode, actually run the deauth attack.

**Tasks**:
- Modify `start_active_attack()` (or a new internal method) to:
  - Extract config using `active_attack_config()`.
  - If `dry_run == true`: Call `run_deauth()` directly or via lightweight async.
  - If `dry_run == false`: First trigger policy confirmation, then execute.
- On completion: Call `set_active_results(result)` with the real `ActiveWirelessAttackResult`.
- On error: Use `set_error()`.

**File**: `crates/eggsec-tui/src/tabs/wireless.rs`

### 2. Add Policy Confirmation for Non-Dry-Run Attacks

Integrate with the existing TUI confirmation/overlay system:
- Show a clear warning before transmitting frames.
- Require explicit user confirmation.
- Respect the same risk level (`Intrusive`) used in the CLI handler.

This is important for safety and consistency with the CLI path.

### 3. Background Execution (Recommended)

For a good UX, run the attack in a background worker or tokio task instead of blocking the UI thread:
- Use or extend an existing worker pattern in `eggsec-tui`.
- Update progress if possible (e.g., frames sent).
- Ensure `stop()` / cancellation works reasonably.

### 4. Polish Items

- Update the passive results view tip if needed once TUI execution is fully working.
- Ensure dry-run works cleanly and quickly from TUI (no privileges required).
- Add a couple more integration-style tests (e.g., full happy path with mocked execution).
- Minor robustness: Better error messages in TUI when interface is invalid or feature not enabled.

### 5. Documentation Touch-up (Low Effort)

- Add a short TUI usage example in `docs/WIRELESS.md` under the Active Attacks section (once execution is complete).
- Confirm architecture notes are still accurate.

---

## Handoff Checklist

- [ ] Wire `run_deauth` call inside `start_active_attack()` (or via worker)
- [ ] Implement policy confirmation overlay for real attacks
- [ ] Add background task execution
- [ ] Handle success / error results properly back into the tab
- [ ] Add 1–2 integration tests for TUI active attack flow
- [ ] Update `docs/WIRELESS.md` with brief TUI example
- [ ] Verify dry-run works from TUI without root
- [ ] Clean up any remaining "TODO" comments in active wireless code

---

## Suggested Implementation Order

1. Wire basic execution (dry-run first) — biggest usability win
2. Add policy confirmation for real attacks
3. Add background worker support
4. Polish + tests
5. Documentation update

This order lets you deliver a working TUI experience quickly.

---

## Open Questions

1. Should we support live frame count updates during execution, or keep it simple (progress bar + final result)?
2. Do we want a dedicated "Launch Attack" button/key, or is Enter sufficient?
3. How should monitor-mode interface selection work in the TUI (manual entry vs auto-detect)?

---

**End of Final Polish Plan**