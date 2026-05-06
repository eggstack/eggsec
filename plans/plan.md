# TUI Navigation and Input Corrective Plan

## Status

Completed.

This corrective pass finished the selector/navigation cleanup that was still pending after the earlier TUI refactor attempt.

## Completed Items

### Selector Semantics

- `Selector::focus()` now only focuses the control.
- `Selector::focus_open()` exists for explicit focus-and-open behavior where needed.
- `Selector::handle_enter()` now opens a closed selector and confirms an open selector.
- Closed selectors no longer mutate selection through left/right navigation.
- Explicit open/close/confirm/cancel methods are available and covered by tests.

### Control Scaffolding Cleanup

- The unused `ControlEvent` / `ControlOutcome` scaffolding was removed.
- `crates/slapper/src/tui/components/events.rs` was deleted.
- `components/mod.rs` no longer exports the removed module.
- Stale docs guidance in `AGENTS.override.md` was updated to match the current code.

### Tab Migration

The high-impact tabs were updated to use the explicit selector contract:

- `Cluster`
- `Load`
- `Packet`
- `Scan`
- `Fuzz`
- `Report`
- `Settings`

### Overlay Rendering

- Packet dropdown rendering now follows the overlay pattern instead of being rendered inline.

### Cluster Reachability

- Command palette routing now switches to `Tab::Cluster` correctly.
- The Cluster command is reachable through tab navigation, quick switch, and command palette selection.

### Verification

Validated locally:

```bash
cargo test --lib -p slapper tui::
cargo check --lib -p slapper
```

Results:

- TUI tests passed.
- `cargo check` passed.
- Dead-code warnings from the removed control scaffolding are gone.

## Historical Context

The earlier plan tracked a broader TUI cleanup effort. That work is now mostly complete, and this file is retained as a record of the refactor intent and the completed corrective pass.

## Notes

- The current work branch includes the implementation changes plus this plan update.
- No further action is required for the selector/navigation corrective pass unless a regression is found.
