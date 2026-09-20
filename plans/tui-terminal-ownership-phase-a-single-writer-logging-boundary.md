# TUI terminal ownership Phase A — single-writer logging boundary

Status: Executed (2026-09-20)

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

Executed 2026-09-20.

- Start SHA: `5b653e3f` (plans campaign registration head). Final
  implementation SHA: Phase A implementation commit on `main` (this commit;
  record-only SHA follow-up, if any, touches plan text only).
- Source audit results:
  - `crates/eggsec-cli/src/main.rs` called `init_logging()` before testing
    TUI launch eligibility; both logging copies built an unconditional console
    `fmt::layer()` (stdout default); normal TUI operation emits ordinary
    `tracing` records eligible at default `info` filter.
  - Direct-write audit over `crates/eggsec-tui/src/**/*.rs` found one
    production direct write: `app/runner.rs` post-`EnterAlternateScreen`
    `eprintln!` small-terminal warning. The only other `io::stdout` use is the
    legitimate `CrosstermBackend::new(stdout)` constructor. No `println!` /
    `print!` / `eprint!` / `dbg!` on live paths; test-only mentions live in
    `AGENTS.override.md` prose (not Rust code).
  - TUI-reachable subprocess audit (Phase A scope): no new subprocess sites
    introduced or closed here beyond recording; Phase B owns inherited
    stdout/stderr closure.
- Selected logging API names (identical in both copies):
  `ConsoleLogging::{Enabled, Disabled}`, `init_logging_with_console(format,
  log_dir, console)`, `init_logging(format, log_dir)` compat wrapper
  (`Enabled`), `resolve_console_logging(is_rich_tui_launch)`,
  `console_layer_enabled(console)`. Process host adds
  `rich_tui_launch_requested(has_command, stdout_is_terminal, tui_feature)` +
  `console_policy_for_launch(is_rich_tui_launch)` in `eggsec-cli/src/main.rs`;
  TUI adds `small_terminal_warning_message(width, height)` in
  `app/runner.rs`.
- Test commands/outcomes (local, before push):
  - `cargo fmt --all --check` PASS (after `cargo fmt --all`).
  - `cargo check -p eggsec-cli` PASS (one pre-existing-style dead_code warning
    on the compat wrapper silenced via `#[allow(dead_code)]` with rationale).
  - `cargo check -p eggsec-tui` PASS.
  - `cargo test -p eggsec-tui --lib` PASS (875 passed).
  - `cargo test -p eggsec-tui` PASS (875 passed, 12 ignored).
  - `cargo test -p eggsec-cli` PASS (9 passed: 3 launch-policy + 6
    injectable-writer logging policy tests).
  - `cargo test -p eggsec --features logging-subscriber --lib logging` PASS
    (engine copy policy + 4-combo emission tests via local dispatcher, no
    global subscriber mutation; file-only claim verified by observed event in
    file writer while console capture stays empty).
  - `bash scripts/check-architecture-guards.sh` ALL PASSED (Check 138 new).
  - `make check` PASS (EXIT 0, full mandatory Rust contract incl. check-deps,
    clippy, doc/integration/output/report-model/policy/eggfetch/TUI/guards).
- Architecture guard number: Check 138 (TUI single-writer logging boundary;
  138a production macro ban outside `#[cfg(test)]` with comment-aware match,
  138b CLI launch uses `init_logging_with_console` + intent resolution, 138c
  both logging copies expose/gate the policy, 138d runner in-frame warning
  path). `docs/CI_ARCHITECTURE_GUARDS.md` documents the rationale (tracing
  deliberately not banned).
- Intentionally retained pre-terminal stderr output: unknown `--runtime` notice
  (`Unknown runtime mode '...', falling back to embedded`) and headless
  no-command guidance remain `eprintln!` before terminal acquisition (cannot
  corrupt the alternate screen; preserves the fallback-to-embedded CLI
  contract). Initialization failures before acquisition may still report to
  stderr. No new default persistent TUI log directory introduced.
- Docs updated: `architecture/logging.md` (console emission policy, pruned
  stale no-`log_dir`-implies-console claim), `architecture/tui.md`
  (single-writer section, corrected entry point, new convention 10),
  `architecture/cli_commands.md` (startup + logging setup),
  `crates/eggsec-tui/src/AGENTS.override.md` (new Phase A section),
  `AGENTS.md` (workspace TUI single-writer bullet + guard), skills
  `eggsec-tui` (ownership + corrected `overlay.notification`),
  `eggsec-cli` (logging surface), `eggsec-agent/agent_observability.md`
  (file-only vs composed wording + new API). README required no change (no
  logging/console claims to prune).
