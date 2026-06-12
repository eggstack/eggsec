# Wireless Active Attacks: Final Dispatcher Wiring, Feedback Loop & Polish Plan

**Date**: 2026-06-12  
**Status**: Draft — Ready for Handoff  
**Focus**: Closing the last gaps to make TUI active attacks fully functional

---

## Executive Summary

Good progress has been made:

- TUI now submits `TaskConfig::WirelessActive` tasks via `start_active_attack()` + `TaskBuilder`.
- `handle_wireless_active_task()` handler exists and correctly calls `run_deauth()`.
- Registration helper and dispatcher wiring example are available.

**Remaining items** (these still need to be completed):

1. Insert the `WirelessActive` match arm into the main task dispatcher.
2. Wire result feedback from the handler back to `WirelessTab::set_active_results()`.
3. Add policy confirmation for non-dry-run attacks.
4. Add at least one end-to-end style test.

This plan provides a focused, surgical path to finish the feature.

---

## Remaining Gaps

| Gap | Impact | Priority | Suggested Location |
|-----|--------|----------|--------------------|
| Dispatcher match arm for `WirelessActive` | Tasks are submitted but never executed | Critical | Main task dispatcher (likely `task_management.rs` or `workers/mod.rs`) |
| Result feedback to `WirelessTab` | User never sees attack results | High | Handler + tab communication layer (channels / app state / callback) |
| Policy confirmation overlay | No safety guard for real attacks | High | `WirelessTab::start_active_attack()` or `handle_enter()` |
| End-to-end test | No verification of full flow | Medium | `wireless.rs` tests or integration test module |

---

## Surgical Tasks

### 1. Insert Dispatcher Match Arm (Critical)

**File**: Main task dispatcher (search for existing `match task_config` or `TaskConfig::` arms)

Add this arm:

```rust
TaskConfig::WirelessActive {
    interface,
    attack_type,
    bssid,
    client,
    frame_count,
    rate_limit,
    dry_run,
} => {
    let result = crate::workers::wireless_active_handler::handle_wireless_active_task(
        interface,
        attack_type,
        bssid,
        client,
        frame_count,
        rate_limit,
        dry_run,
    ).await;

    match result {
        Ok(res) => {
            // TODO: deliver to the correct WirelessTab
            // e.g. app_state.send_to_tab(tab_id, TabMessage::ActiveAttackResult(res));
        }
        Err(e) => {
            // e.g. app_state.send_to_tab(tab_id, TabMessage::Error(e.to_string()));
        }
    }
}
```

Call `register_wireless_active_handler()` during worker/app initialization.

### 2. Implement Result Feedback Loop (High Priority)

Options (choose one consistent with existing architecture):

- **Preferred**: Use an app-level event bus or typed channel (`TabMessage` enum).
- Alternative: Pass a callback or `Weak<WirelessTab>` when submitting the task.
- Fallback: Store result in shared `AppState` and have the tab poll on next frame.

Once delivered, call:
```rust
twireless_tab.set_active_results(result);
```

### 3. Add Policy Confirmation (High Priority)

Before submitting a non-dry-run task in `start_active_attack()`:

```rust
if !dry_run {
    if !self.confirm_active_attack() {  // show overlay / prompt
        self.stop();
        return;
    }
}

// then proceed with task submission
```

Reuse or extend any existing confirmation overlay system in the TUI.

### 4. Add One End-to-End Test (Medium)

Add a test that covers:
- `handle_enter()` in ActiveConfig mode
- Task submission
- (Mocked) handler execution
- `set_active_results()` being called

Can be done with a test harness or by mocking the worker layer.

### 5. Minor Polish (Low)

- Improve error messages when `run_deauth` fails.
- Add a short note in `docs/WIRELESS.md` about TUI active attack usage.
- Consider adding progress updates during frame injection (optional).

---

## Recommended Implementation Order

1. Insert the dispatcher match arm (unblocks execution)
2. Wire result feedback to the tab (makes feature usable)
3. Add policy confirmation (safety)
4. Add one end-to-end test
5. Polish + docs

---

## Handoff Checklist

- [ ] Add `TaskConfig::WirelessActive` match arm in main dispatcher
- [ ] Call `register_wireless_active_handler()` at startup
- [ ] Implement result delivery mechanism to `WirelessTab`
- [ ] Add policy confirmation before non-dry-run task submission
- [ ] Write one integration-style test for the full flow
- [ ] Verify dry-run works end-to-end from TUI
- [ ] Update `docs/WIRELESS.md` with TUI usage note (optional but recommended)

---

## Open Questions

1. What is the preferred mechanism for sending results/errors from workers back to specific tabs? (Event bus, channels, shared state?)
2. Should policy confirmation be handled in the tab or centralized in the task submission layer?

---

**End of Plan**