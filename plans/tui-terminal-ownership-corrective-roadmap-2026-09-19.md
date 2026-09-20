# TUI terminal ownership corrective roadmap

Status: Ready for implementation

Date: 2026-09-19

Baseline: `05e04b6643f739025d54fe188f5fe381d32439c6`

## Purpose

Eliminate the recurring class of Eggsec TUI corruption where warnings, errors,
status information, or child-process output are written directly to the terminal
while Ratatui owns the alternate screen.

The observed symptom is text appearing below, through, or outside the main TUI
box, followed by unstable differential redraws. The root contract is broader
than any individual `println!` or warning site: while the rich TUI is active,
Ratatui/Crossterm must be the only writer to the controlling terminal.

This roadmap is intentionally narrow. It does not redesign TUI layout, runtime
dispatch, task execution, or the frontend/runtime convergence work that is
already complete.

## Confirmed baseline defects

At the baseline SHA:

1. `crates/eggsec-cli/src/main.rs` calls `init_logging(...)` before deciding
   to launch the TUI.
2. `crates/eggsec-cli/src/logging.rs` installs a
   `tracing_subscriber::fmt::layer()` console layer by default. The current
   tracing-subscriber formatter defaults to stdout unless a writer is
   overridden.
3. The TUI and its runtime clients emit ordinary `tracing::info!`,
   `tracing::warn!`, `tracing::error!`, and `tracing::debug!` records
   during normal operation. Those records therefore race the Ratatui backend
   for the same terminal.
4. `crates/eggsec-tui/src/app/runner.rs` explicitly calls `eprintln!` for
   the sub-80x24 warning *after* raw mode, alternate-screen entry, and mouse
   capture have been enabled.
5. `run_with_mode()` performs terminal cleanup manually at the bottom of the
   function. Fallible work occurs after terminal acquisition, including daemon
   Tokio-runtime construction, so an early `?` or panic can bypass the normal
   teardown sequence.
6. Current TUI conventions correctly require errors to be logged rather than
   silently discarded, but the logging architecture does not distinguish
   headless/CLI console output from alternate-screen TUI execution. This makes
   future observability improvements capable of reintroducing corruption.
7. No repository guard currently expresses the single-terminal-writer
   invariant.
8. TUI-reachable external-process execution has not been closed as part of the
   same invariant. Any child process inheriting stdout/stderr can corrupt the
   alternate screen even after tracing is fixed.

## Upstream behavior verified

Research was checked against the versions Eggsec currently uses
(`ratatui 0.30`, `crossterm 0.29`, `tracing-subscriber 0.3`):

- Ratatui 0.30 documents `run` as the recommended full-screen path and
  restores raw mode / alternate screen after the closure completes.
- `ratatui::init` / `try_init` install a panic hook that restores the
  terminal before delegating to the prior panic hook.
- Manual `Terminal::new` construction leaves terminal-mode setup/teardown to
  the application.
- Ratatui restoration covers raw mode and alternate-screen state; Eggsec must
  still pair `EnableMouseCapture` with `DisableMouseCapture`.
- `tracing_subscriber::fmt::Layer` uses stdout as its default writer. Moving
  console logs from stdout to stderr would therefore *not* solve the TUI
  problem; both descriptors target the same controlling terminal.

References:

- https://docs.rs/ratatui/0.30.2/ratatui/fn.run.html
- https://docs.rs/ratatui/0.30.2/ratatui/fn.try_init.html
- https://docs.rs/ratatui/0.30.2/ratatui/fn.try_restore.html
- https://docs.rs/tracing-subscriber/latest/tracing_subscriber/fmt/struct.Layer.html
- https://docs.rs/crossterm/0.29.0/crossterm/event/struct.DisableMouseCapture.html

## Corrective sequence

### Phase A — Single-writer logging and user-message boundary

Plan:
[`tui-terminal-ownership-phase-a-single-writer-logging-boundary.md`](tui-terminal-ownership-phase-a-single-writer-logging-boundary.md)

Make TUI launch mode explicit before subscriber construction, suppress console
formatting for the rich TUI while preserving the existing CLI/CI/daemon console
behavior, remove direct terminal writes from the TUI ownership window, route
recoverable user-facing information into existing TUI notification/error state,
and add static/test guards for the invariant.

Exit condition: ordinary tracing at every level and the existing small-terminal
warning path cannot write directly to the controlling terminal while the rich
TUI is active.

### Phase B — Terminal lifecycle and inherited-process closure

Plan:
[`tui-terminal-ownership-phase-b-lifecycle-process-output-closure.md`](tui-terminal-ownership-phase-b-lifecycle-process-output-closure.md)

Replace the fallible manual terminal lifecycle with a cleanup-safe owner,
preserve post-restoration fatal error reporting, audit all TUI-reachable process
spawns for inherited stdout/stderr, add a PTY smoke test where supported, and
reconcile architecture/agent guidance.

Exit condition: terminal state is restored on normal return, runtime/setup
errors, and panic/unwind paths; no known TUI-reachable child process can bypass
the single-writer rule.

## Ordering

Phase A lands first because it fixes the active corruption mechanism with a
small, reviewable boundary change.

Phase B depends on the Phase A output contract. It must not use terminal
restoration as a substitute for eliminating concurrent writers.

The phases may be implemented in separate commits, but the line is not closed
until both phases pass their acceptance criteria.

## Global invariants

After this roadmap:

1. While the rich TUI owns the alternate screen, only the Ratatui/Crossterm
   renderer may write to its stdout/stderr terminal.
2. `tracing` remains the internal diagnostic facade; call sites must not be
   downgraded to silent suppression to protect rendering.
3. CLI/CI/headless/daemon-console behavior remains unchanged unless explicitly
   covered by a separate change.
4. Recoverable user-facing TUI errors belong in app state (notification,
   per-tab error, popup, or status), not `print!`/`eprintln!`.
5. Fatal errors may be printed only after terminal restoration.
6. TUI-reachable child processes must capture/pipe output or intentionally
   suspend/restore the TUI before taking terminal ownership. No such interactive
   handoff is currently required by this roadmap.
7. Cleanup is best-effort and idempotent on unwind; a cleanup failure must never
   prevent later cleanup steps from being attempted.
8. The change must not weaken scope, policy, dispatch, cancellation, daemon
   protocol, or task-result semantics.

## Roadmap acceptance criteria

The corrective line is complete only when all of the following are true:

1. The CLI determines rich-TUI launch intent before selecting the logging
   destination.
2. Rich TUI mode installs no stdout/stderr tracing formatter.
3. Existing non-TUI logging output remains behaviorally compatible.
4. The sub-80x24 warning no longer uses direct terminal output after terminal
   acquisition.
5. TUI configuration/runtime warnings can be emitted without corrupting the
   screen.
6. No production `println!`, `eprintln!`, `print!`, `eprint!`, or
   `dbg!` remains in `crates/eggsec-tui/src` on a live TUI path.
7. Terminal raw mode, alternate screen, mouse capture, and cursor state are
   restored on all ordinary error paths.
8. Panic/unwind behavior leaves the terminal usable.
9. `run_with_mode()` does not swallow its terminal-loop fatal error merely to
   log it.
10. TUI-reachable subprocess sites are inventoried and none inherit
    stdout/stderr while the rich TUI is active.
11. A regression guard pins the direct-writer rule.
12. A PTY/integration smoke test proves a representative warning/error does not
    leak into the rendered terminal stream, where the host supports PTYs.
13. `cargo fmt --all --check`, `cargo check -p eggsec-cli`,
    `cargo check -p eggsec-tui`, `cargo test -p eggsec-tui`, the relevant
    CLI tests, architecture guards, and `make check` pass.
14. `architecture/tui.md`, `architecture/logging.md`, and relevant agent
    guidance describe the new ownership contract from source, not from plan
    assumptions.

## Explicit exclusions

This roadmap does not authorize:

- a TUI visual redesign;
- changing tab layout or theme architecture;
- replacing Ratatui or Crossterm;
- removing tracing diagnostics from engine/runtime code;
- introducing default persistent TUI log files without a separate privacy and
  retention decision;
- changing command semantics, operation IDs, scope policy, or runtime DTOs;
- adding a new process-execution framework unless the subprocess audit proves
  repeated TUI-specific ownership logic warrants one;
- broad logging-crate extraction unrelated to the terminal defect;
- release or publication work.

## Handoff requirement

Implementation must record:

- actual starting and final SHAs;
- exact direct-write and subprocess audit commands used;
- every TUI-reachable process site and its final stdout/stderr disposition;
- test/guard commands and outcomes;
- any platform-specific PTY test skip with reason;
- whether any compatibility behavior was intentionally retained.

Do not close the roadmap by clearing/redrawing the terminal after leaked output.
That masks the defect. Closure requires exclusive writer ownership.
