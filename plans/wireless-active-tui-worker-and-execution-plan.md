# Wireless Active Attacks: TUI Worker Execution & Remaining Gaps Closure Plan

**Date**: 2026-06-12  
**Status**: Draft — Ready for Handoff  
**Focus**: Completing the worker/task execution layer and closing the TUI loop

---

## Executive Summary

Good architectural progress has been made:

- TUI has full interaction support for active attacks (mode toggle, inputs, `handle_enter`, `start_active_attack`, `build_task_config`).
- Task-based design using `TaskConfig::WirelessActive` is in place.
- Reporting bridge and CLI path are complete.

**Remaining gaps** are now concentrated in the **execution layer**:
- No worker currently handles `WirelessActive` tasks.
- `run_deauth()` is never called from the TUI path.
- No result callback back to the `WirelessTab`.
- Policy confirmation for real attacks is missing.

This plan provides a focused path to close these gaps and make active deauth fully functional from the TUI.

---

## Current Remaining Gaps

| Gap | Impact | Priority |
|-----|--------|----------|
| No worker for `TaskConfig::WirelessActive` | TUI cannot execute attacks | Critical |
| `run_deauth()` never called from TUI | Feature unusable from TUI | Critical |
| No result feedback to `WirelessTab` | User sees no outcome | High |
| Missing policy confirmation overlay | Safety / consistency risk | High |
| Limited end-to-end tests | Reliability | Medium |

---

## Recommended Approach

Leverage the existing task/worker architecture:

1. Implement a handler for `TaskConfig::WirelessActive` in the worker system.
2. Inside the handler, call `crate::wireless::active::attacks::deauth::run_deauth()` (respecting `dry_run`).
3. On completion, send the `ActiveWirelessAttackResult` back to the originating tab.
4. Add policy confirmation **before** creating the task when `dry_run == false`.

This keeps execution logic out of the UI layer and follows the pattern used by other long-running operations.

---

## Detailed Tasks

### 1. Implement `WirelessActive` Task Handler (Critical)

**Location**: Likely in `crates/eggsec-tui/src/workers/` or task execution module.

- Create or extend the worker that matches on `TaskConfig::WirelessActive`.
- Extract parameters (interface, bssid, client, frame_count, rate_limit, dry_run).
- Call:
  ```rust
  crate::wireless::active::attacks::deauth::run_deauth(&config, broadcast).await
  ```
- On success: Report result back to `WirelessTab::set_active_results()`.
- On error: Report error via the tab's error handling path.

### 2. Wire Task Creation from TUI

Ensure `start_active_attack()` (or `handle_enter` in Active mode) properly creates and submits a `WirelessActive` task via the existing `TaskBuilder` / task management system.

### 3. Add Policy Confirmation (High Priority)

Before submitting a non-dry-run `WirelessActive` task:
- Trigger the existing TUI policy confirmation overlay.
- Show clear warning about frame transmission.
- Only proceed on explicit user confirmation.
- This mirrors the `--allow-active-wireless` requirement in the CLI.

### 4. Result Feedback Mechanism

Define how the worker communicates results back to the specific `WirelessTab` instance (e.g., via app state, channel, or callback).

### 5. Polish & Testing

- Add end-to-end style tests for the full TUI → task → worker → result flow (mocked where necessary).
- Improve error messages when execution fails.
- Verify dry-run path works without privileges.

### 6. Documentation (Low Effort)

- Once execution works, add a short TUI example in `docs/WIRELESS.md`.

---

## Handoff Checklist

- [ ] Implement handler for `TaskConfig::WirelessActive`
- [ ] Wire `start_active_attack()` to submit the task
- [ ] Add policy confirmation before creating non-dry-run tasks
- [ ] Implement result callback to `WirelessTab::set_active_results()`
- [ ] Add error handling path from worker to tab
- [ ] Write at least one integration test for the full flow
- [ ] Verify dry-run works cleanly from TUI
- [ ] Update `docs/WIRELESS.md` with TUI usage note

---

## Suggested Implementation Order

1. **Implement the `WirelessActive` worker handler** (enables execution)
2. **Wire task submission from TUI** (connects UI to execution)
3. **Add policy confirmation** (safety)
4. **Implement result feedback loop**
5. **Tests + polish + docs**

This order delivers a working TUI experience as quickly as possible.

---

## Open Questions

1. How should the worker report results back to the specific tab? (App-level event bus, direct callback, or shared state?)
2. Should policy confirmation happen in the tab before task creation, or inside the task system?
3. Do we want progress updates during frame injection (e.g. frames sent so far), or just start + final result?

---

**End of Plan**