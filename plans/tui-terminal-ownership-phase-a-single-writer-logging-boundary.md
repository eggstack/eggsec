# TUI terminal ownership Phase A — single-writer logging boundary

Status: Ready for implementation

Date: 2026-09-19

Baseline: `05e04b6643f739025d54fe188f5fe381d32439c6`

Parent roadmap:
[`tui-terminal-ownership-corrective-roadmap-2026-09-19.md`](tui-terminal-ownership-corrective-roadmap-2026-09-19.md)

## Purpose

Close the active rendering-corruption mechanism by ensuring the rich TUI has no
independent console writer competing with Ratatui.

This phase is about output ownership, not terminal teardown. Phase B owns
failure-safe terminal lifecycle and subprocess inheritance.

## Confirmed source state

### CLI initializes console tracing before TUI selection

`crates/eggsec-cli/src/main.rs` currently:

1. parses CLI;
2. calls `init_logging(...)`;
3. only then tests `cli.command.is_none() && stdout.is_terminal()`;
4. launches `eggsec_tui::run_with_mode(...)`.

The normal TUI path therefore inherits the default console formatting layer.

### The formatter writes to the terminal

Both:

- `crates/eggsec-cli/src/logging.rs`
- `crates/eggsec/src/logging/init.rs`

construct `fmt::layer()` without `with_writer(...)` for the console layer.
The current tracing-subscriber contract uses stdout by default.

The default `EnvFilter` is `info`, so ordinary TUI lifecycle messages are
eligible for output even when the user has not enabled verbose logging.

### Normal TUI operation emits tracing records

Representative current sites include:

- configuration-load warnings in `app/runner.rs`;
- daemon attach/create/subscribe informational events;
- daemon parse/read/close/rejection warnings in
  `runtime_client/daemon.rs`;
- task submission debug/error events;
- session persistence warnings;
- terminal event-stream warnings.

These call sites are not individually wrong. They should remain diagnostic
events.

### Direct write exists inside the alternate screen

`app/runner.rs` enters raw mode + alternate screen + mouse capture and then
uses `eprintln!` when the terminal is smaller than 80x24.

The UI already has a Ratatui-rendered small-terminal/degraded-layout path, so
this direct write is unnecessary and actively breaks the frame model.

## Design decision

Introduce an explicit *console emission policy* in the logging initialization
boundary instead of deleting tracing calls or changing them to stderr.

Preferred shape:

```rust
pub enum ConsoleLogging {
    Enabled,
    Disabled,
}

pub fn init_logging_with_console(
    format: LogFormat,
    log_dir: Option<PathBuf>,
    console: ConsoleLogging,
) -> Option<WorkerGuard>
```

Keep the existing `init_logging(format, log_dir)` API as a compatibility
wrapper that delegates with `ConsoleLogging::Enabled`.

Apply the same additive API to both existing logging implementations so
`architecture/logging.md` remains truthful and the engine copy does not drift
from the process-host copy.

When console logging is disabled:

- if `log_dir` is `Some`, keep the existing JSON file layer and omit only the
  console formatter;
- if `log_dir` is `None`, install the filtered registry without a formatting
  writer (or equivalent no-console subscriber) so tracing call sites remain
  valid but emit no terminal bytes;
- do **not** silently create a new persistent TUI log directory. That would
  introduce a new data-retention/privacy behavior outside this defect fix.

The exact enum/function naming may differ, but the semantics above are required.

## Workstream 1 — Resolve launch surface before subscriber construction

Refactor `crates/eggsec-cli/src/main.rs` so the process knows whether it will
enter rich TUI mode before selecting console logging behavior.

Requirements:

1. Preserve early `--generate-config` and shell-completion behavior.
2. Compute one boolean/enum representing rich-TUI launch eligibility:
   - TUI feature compiled;
   - no command;
   - stdout is a terminal.
3. Initialize logging with console output disabled for that surface.
4. Preserve the existing Pretty/Json choice for non-TUI paths.
5. Preserve `agent_log_dir()` behavior.
6. Do not make stderr the TUI writer; stderr is the same terminal and would
   reproduce the corruption.
7. Unknown `--runtime` values may still be reported before terminal
   acquisition, but prefer returning a structured argument/config error rather
   than relying on ad-hoc output if the current CLI contract allows it.

Add focused tests around the logging-surface decision. Avoid tests that mutate
the global tracing subscriber in parallel; test the policy-selection/building
logic independently where possible.

## Workstream 2 — Make console/no-console logging a first-class policy

Update:

- `crates/eggsec-cli/src/logging.rs`
- `crates/eggsec/src/logging/init.rs`
- `crates/eggsec/src/logging/mod.rs`

Requirements:

- existing `init_logging` behavior remains source-compatible and
  behavior-compatible;
- new no-console mode does not construct a stdout/stderr fmt layer;
- file logging, when explicitly requested by an existing caller, remains JSON,
  non-blocking, ANSI-free, and guarded by the existing `WorkerGuard`;
- `RUST_LOG` / EnvFilter semantics remain unchanged;
- initialization failures that occur before rich terminal acquisition may still
  report to stderr;
- do not add a second subscriber, reload handle, global mutable writer switch,
  or TUI-specific logging crate.

Add tests for layer/policy construction using injectable writers or a pure
configuration builder where practical. The test should prove that the TUI
policy has no console writer rather than merely asserting an enum value.

## Workstream 3 — Remove direct terminal writes from the TUI ownership window

In `crates/eggsec-tui/src/app/runner.rs`:

1. Remove the post-`EnterAlternateScreen` `eprintln!` size warning.
2. Preserve the existing Ratatui-rendered too-small fallback.
3. For the "usable but below recommended 80x24" range, either:
   - surface a normal in-frame `NotificationSeverity::Warning` after
     `App` construction; or
   - document that the existing responsive layout is sufficient and omit the
     warning.
4. Do not call `terminal.clear()` or force full redraws to hide leaked
   messages.

Audit `crates/eggsec-tui/src` for:

```text
println!(
eprintln!(
print!(
eprint!(
dbg!(
std::io::stdout
std::io::stderr
io::stdout
io::stderr
```

Classify test-only uses separately. Production rich-TUI code must not emit
direct terminal text.

## Workstream 4 — Route recoverable information through TUI state

Do not replace every tracing event with a notification. Keep tracing for
diagnostics.

Only information that a user needs to act on during the TUI session should be
mapped into existing UI state:

- notification overlay for transient warnings/info;
- per-tab error for task/target failures;
- existing popup/status surfaces for modal/lifecycle conditions.

At minimum, evaluate these current runner-level cases:

- malformed/unreadable TUI configuration;
- daemon attach failure;
- terminal event-stream failure/end;
- quick-save failure on exit.

Do not duplicate the same message into multiple persistent UI surfaces.

## Workstream 5 — Add regression guard for terminal writers

Append the next available architecture guard number (137 is current at the
baseline) to `scripts/check-architecture-guards.sh`.

The guard should scan production `crates/eggsec-tui/src/**/*.rs` and fail on
direct print/debug macros outside `#[cfg(test)]` sections. Prefer a small
purpose-built script/helper if reliably excluding test modules with shell regex
becomes brittle.

The guard should also fail if the CLI TUI launch path uses console-enabled
logging.

Do not write a guard that bans `tracing::warn!` or other tracing macros.
Those are required diagnostics; the sink policy is the architectural control.

Update `docs/CI_ARCHITECTURE_GUARDS.md` with the guard and rationale.

## Workstream 6 — Documentation and contributor guidance

Update:

- `architecture/logging.md`
- `architecture/tui.md`
- `crates/eggsec-tui/src/AGENTS.override.md`
- `AGENTS.md` only if the rule needs workspace-level visibility.

Required documentation:

- process host chooses logging destination by execution surface;
- rich TUI console logging is disabled;
- stdout *and* stderr are forbidden as side channels while the alternate screen
  is owned;
- tracing remains the required diagnostic facade;
- user-visible recoverable errors use TUI state;
- fatal errors are deferred until after restoration (implemented in Phase B);
- no new default persistent TUI logging was introduced.

Remove or amend the stale statement in `architecture/logging.md` that every
no-`log_dir` process necessarily receives a console layer.

## Tests and validation

Minimum local validation:

```bash
cargo fmt --all --check
cargo check -p eggsec-cli
cargo check -p eggsec-tui
cargo test -p eggsec-tui
cargo test -p eggsec-cli
bash scripts/check-architecture-guards.sh
make check
```

Also run focused logging tests with at least:

- console-enabled/no-file;
- console-enabled/file;
- console-disabled/no-file;
- console-disabled/file.

Do not claim file-only behavior unless an event is actually observed in the
test writer/file while console capture remains empty.

## Acceptance criteria

Phase A is complete only when:

1. Rich TUI launch intent is resolved before logging initialization.
2. Rich TUI mode has no console fmt writer.
3. Non-TUI CLI/CI logging remains compatible.
4. Existing `init_logging` callers remain source-compatible.
5. File logging still works when explicitly requested.
6. The sub-80x24 path contains no direct terminal write after TUI acquisition.
7. Production `eggsec-tui` code has no direct print/debug macro on a live path.
8. Normal `tracing::warn!` / `info!` events can occur during TUI execution
   without emitting terminal bytes.
9. Recoverable user-relevant runner errors are represented through TUI state
   where appropriate.
10. Architecture guard(s), docs, and agent guidance encode the single-writer
    rule.
11. Required validation passes.

## Non-goals

- Do not implement terminal teardown refactoring here.
- Do not add default TUI log persistence.
- Do not lower log severity or remove diagnostics simply to reduce output.
- Do not redesign notification rendering.
- Do not touch canonical dispatch/scope/policy semantics.
- Do not solve child-process inheritance here beyond recording any sites found
  during the direct-output audit; Phase B owns that closure.

## Completion record

When executed, append:

- actual start/final SHA;
- source audit results;
- selected logging API names;
- exact test commands/outcomes;
- architecture guard number;
- any intentionally retained pre-terminal stderr output.
