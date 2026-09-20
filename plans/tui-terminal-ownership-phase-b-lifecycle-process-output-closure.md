# TUI terminal ownership Phase B — lifecycle and process-output closure

Status: Ready for implementation

Date: 2026-09-19

Baseline: `05e04b6643f739025d54fe188f5fe381d32439c6`

Depends on:
[`tui-terminal-ownership-phase-a-single-writer-logging-boundary.md`](tui-terminal-ownership-phase-a-single-writer-logging-boundary.md)

Parent roadmap:
[`tui-terminal-ownership-corrective-roadmap-2026-09-19.md`](tui-terminal-ownership-corrective-roadmap-2026-09-19.md)

## Purpose

Make terminal ownership failure-safe and close the remaining output bypass:
TUI-reachable child processes that could inherit stdout/stderr.

Phase A must already ensure tracing/direct application output does not compete
with Ratatui.

## Confirmed lifecycle weakness

`crates/eggsec-tui/src/app/runner.rs` currently enters:

1. raw mode;
2. alternate screen;
3. mouse capture;
4. `Terminal::new`.

Cleanup is performed manually near the end:

1. disable raw mode;
2. leave alternate screen;
3. disable mouse capture;
4. show cursor.

Fallible work occurs between acquisition and cleanup. A representative
pre-loop example is `tokio::runtime::Runtime::new()?` in daemon mode. Any
early return before the cleanup block can leave terminal modes active.

The current `run_app` error is also logged after teardown and then discarded;
`run_with_mode` returns `Ok(())`. The terminal cleanup refactor should
preserve errors rather than turning fatal loop failures into successful exits.

## Upstream lifecycle guidance

Ratatui 0.30 provides higher-level lifecycle helpers:

- `ratatui::run` initializes and restores a default Crossterm terminal around
  a closure;
- `ratatui::try_init` installs a panic hook and returns initialization errors;
- `ratatui::try_restore` disables raw mode and leaves the alternate screen;
- manual `Terminal::new` requires the application to manage these states.

Ratatui does not own Eggsec's explicit mouse-capture enable/disable pair, so
Eggsec still needs a cleanup owner for mouse capture.

Preferred implementation is a small terminal-session abstraction local to the
TUI runner, not a new crate.

## Workstream 1 — Introduce one cleanup-safe terminal owner

Refactor `app/runner.rs` around an explicit session/guard abstraction.

Required behavior:

- acquire raw mode + alternate screen using Ratatui 0.30 lifecycle support
  where practical;
- enable mouse capture only after terminal initialization succeeds;
- if mouse-capture enable fails, restore already-acquired terminal state before
  returning;
- own whether mouse capture is active;
- restore mouse capture, raw/alternate-screen state, and cursor visibility on
  every normal exit;
- provide a best-effort `Drop`/unwind fallback;
- cleanup steps are independent: one failure must not prevent later cleanup
  attempts;
- cleanup must be effectively idempotent so panic-hook restoration plus guard
  drop cannot make the terminal worse.

A suitable shape is an internal `TerminalSession` that owns the
`DefaultTerminal` and mouse state, with:

- fallible `new()`;
- access to `&mut DefaultTerminal`;
- explicit fallible `restore()` for the normal path;
- best-effort `Drop` fallback for early return/unwind.

Using `ratatui::try_init()` is preferred over recreating raw/alternate-screen
logic unless testing shows a concrete incompatibility. Preserve Crossterm
0.29/Ratatui 0.30 alignment.

## Workstream 2 — Structure the TUI body so cleanup dominates all errors

Split terminal acquisition from app/runtime setup:

```text
run_with_mode
  -> acquire TerminalSession
  -> run_tui_body(&mut terminal, ...)
  -> save/collect final state
  -> restore terminal
  -> return original body error (with restoration context if needed)
```

Requirements:

1. No `?` after terminal acquisition may bypass the session guard.
2. Daemon runtime construction and attach occur inside the guarded body.
3. `run_app` errors are returned, not logged-and-converted to success.
4. Fatal error presentation occurs only after the alternate screen is gone.
5. If both the TUI body and restoration fail, preserve the body error as primary
   and attach restoration failure as context rather than losing either.
6. Quick-save failure must have an explicit precedence rule:
   - do not overwrite a more important runtime/render failure;
   - do not silently disappear;
   - report after restoration or through the established UI/diagnostic path.
7. Keep the public `run` / `run_with_mode` signatures unless a concrete
   caller requires a change.

## Workstream 3 — Panic/unwind restoration

Verify the actual panic-hook ordering after adopting Ratatui lifecycle support.

Ratatui 0.30 installs a hook that restores raw/alternate-screen state before
delegating to the previous hook. Eggsec must ensure mouse capture is also
disabled by unwind cleanup.

Do not install competing hooks that recursively wrap each other on repeated
TUI invocations in tests.

Add focused tests around the session abstraction using injectable cleanup
operations where practical. Do not rely only on source inspection.

## Workstream 4 — Exhaustive process-output audit

Before changing process execution, record the actual inventory with:

```bash
rg -n 'std::process::Command|tokio::process::Command|Command::new|Stdio::inherit|\.stdout\(|\.stderr\(|\.status\(|\.spawn\(' crates/
```

For every production site:

1. identify the owning crate/module;
2. determine whether it can be reached from canonical operations exposed by the
   TUI;
3. record stdout disposition;
4. record stderr disposition;
5. record stdin disposition;
6. record whether output is returned, parsed, converted to a runtime event, or
   logged.

For every TUI-reachable site, enforce one of these dispositions:

- `Command::output()` / piped capture with bounded collection;
- explicit `Stdio::piped()` consumed by the operation and surfaced through
  results/events/tracing;
- `Stdio::null()` only where output is intentionally irrelevant and the
  decision is documented.

`Stdio::inherit()` for stdout/stderr is forbidden while the rich TUI remains
active.

If an operation genuinely requires an interactive child terminal in the
future, implement an explicit suspend/restore/resume handoff; do not weaken the
single-writer invariant. No new interactive handoff is authorized in this pass
unless a current supported operation proves it is required.

Avoid introducing a general process wrapper if the audit finds only isolated
sites. If three or more TUI-reachable modules independently repeat the same
capture/limit/error mapping, extract a small engine-owned helper and document
its ownership.

## Workstream 5 — Bounded child-output handling

Capturing child output creates a memory/DoS consideration.

For any changed child process that can produce unbounded output:

- stream with a bounded buffer or apply an explicit byte cap;
- preserve exit status separately from captured text;
- truncate diagnostics with a clear marker;
- sanitize terminal control characters before rendering user-controlled child
  text in the TUI;
- do not treat truncation as successful parsing if complete output is required
  for semantics.

Reuse existing Eggsec sanitization helpers where they fit rather than creating a
parallel escape implementation.

## Workstream 6 — PTY regression smoke

Add a Unix/Linux PTY smoke test, preferably using Python's standard-library
`pty` support so no Rust runtime dependency is added.

The test should:

1. build/run the TUI binary in a pseudo-terminal;
2. set a deterministic terminal size;
3. trigger a representative tracing warning that previously leaked (for
   example, a malformed TUI config or daemon connection/read failure);
4. allow at least one redraw;
5. send the normal quit input;
6. capture the PTY byte stream;
7. assert the warning text is not present as raw out-of-band terminal output;
8. assert expected alternate-screen entry/exit or equivalent restoration
   evidence is present;
9. assert the child exits successfully for the recoverable-warning case.

Add a second bounded failure case if practical that forces a guarded setup/body
error and proves restoration before the final error text.

Keep this test out of unsupported Windows PTY paths. A named platform skip is
acceptable; a Rust compile/test failure is not.

Wire the smoke test into an appropriate existing Deep Checks or specialist
target without making normal unit tests depend on terminal timing.

## Workstream 7 — Architecture guards

After the Phase A guard, add the next available guard(s) to pin:

- no direct production terminal writers in `eggsec-tui`;
- no `Stdio::inherit()` in `eggsec-tui`;
- the runner uses the cleanup-safe terminal-session path rather than returning
  to open-coded raw/alternate lifecycle.

Do **not** globally ban `Stdio::inherit()` across every CLI-only maintenance
command unless the process audit proves that is the intended workspace
contract.

Document guard ownership in `docs/CI_ARCHITECTURE_GUARDS.md`.

## Workstream 8 — Documentation reconciliation

Update:

- `architecture/tui.md`
- `architecture/logging.md`
- any architecture deep-dive owning changed external-process sites;
- `crates/eggsec-tui/src/AGENTS.override.md`;
- `docs/VERIFICATION.md` if a new PTY/deep-check command is introduced.

Required wording:

- Ratatui owns terminal bytes during rich mode;
- tracing sink selection is a frontend/process-host responsibility;
- terminal lifecycle is cleanup-safe;
- user-facing fatal output occurs only after restoration;
- child-process output reachable from TUI is captured, not inherited;
- PTY coverage is platform-scoped and does not replace unit/architecture
  guards.

## Validation

Minimum implementation validation:

```bash
cargo fmt --all --check
cargo check -p eggsec-tui
cargo test -p eggsec-tui
cargo check -p eggsec-cli
cargo test -p eggsec-cli
bash scripts/check-architecture-guards.sh
make check
```

Also run:

- the new PTY smoke on a supported Unix/Linux host;
- `cargo check -p eggsec-tui --features full` if the host has the prerequisites
  expected by the existing TUI feature contract;
- focused tests for every changed process-execution module.

Record skips/blocked checks explicitly.

## Acceptance criteria

Phase B is complete only when:

1. Terminal setup has one cleanup-safe owner.
2. No ordinary `?` path after acquisition can strand raw/alternate-screen
   state.
3. Mouse capture is paired on normal return and unwind.
4. Cursor visibility is restored.
5. Panic-hook/guard interaction is documented and tested enough to avoid
   recursive/double-install behavior.
6. `run_app` fatal errors propagate after restoration.
7. Combined body/restore failures preserve useful diagnostic context.
8. The whole-workspace process inventory is recorded.
9. Every TUI-reachable process has explicit stdout/stderr disposition.
10. No TUI-reachable process inherits stdout/stderr while rich mode is active.
11. Changed child-output capture is bounded where output can be unbounded.
12. PTY smoke demonstrates that a representative warning no longer leaks
    outside the frame.
13. Architecture guards prevent obvious regression.
14. Docs/agent guidance match the implementation.
15. Required validation passes or has a concrete documented platform blocker.

## Explicit exclusions

- No new TUI visual features.
- No broad process abstraction without measured duplication.
- No change to operation authorization or target scope.
- No removal of engine diagnostics.
- No default persistent TUI log retention.
- No pseudo-terminal dependency added to the shipped Rust runtime solely for
  testing.
- No Windows-specific PTY framework as part of this bounded pass.
- No terminal-clearing workaround for leaked text.

## Completion record

When executed, append:

- actual starting/final SHAs;
- terminal-session implementation chosen;
- panic-hook behavior verified;
- complete process-output inventory table;
- files changed to capture child output;
- PTY command/output classification;
- architecture guard numbers;
- all validation results using PASS/FAIL/SKIPPED/BLOCKED terminology.
