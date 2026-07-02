# Phase 9 Plan: CLI Headless and Daemon Client Integration

## Goal

Make the CLI a clean direct-command entrypoint and a daemon client without forcing TUI dependencies into headless usage. After this phase, users should be able to run Eggsec in three distinct modes:

1. Direct CLI command execution.
2. Embedded TUI manual mode.
3. Daemon client mode for session/task lifecycle operations.

The CLI should not be structurally tied to `eggsec-tui` except behind an explicit TUI feature or launch path.

## Current Baseline

The current CLI historically launches the TUI when no command is provided and stdout is a terminal. That behavior should remain available, but headless builds and daemon-client commands should not require terminal UI dependencies unless explicitly enabled.

After Phase 7 and Phase 8, there should be:

- A local daemon host.
- A daemon protocol.
- TUI daemon attach support.
- Runtime session/task/event APIs.

This phase makes CLI packaging and daemon client operations coherent.

## Desired User-Facing Commands

Add or refine commands such as:

```text
eggsec daemon start --socket <path>
eggsec daemon status --socket <path>
eggsec daemon stop --socket <path>
eggsec session list --socket <path>
eggsec session create --surface cli-manual --socket <path>
eggsec session snapshot <session-id> --socket <path>
eggsec task submit <session-id> --kind <kind> [task options] --socket <path>
eggsec task cancel <session-id> <task-id> --socket <path>
eggsec task watch <session-id> --socket <path>
eggsec tui --runtime daemon --socket <path>
```

Exact naming should match existing CLI conventions. Avoid making command names perfect at the cost of delaying the implementation.

## Feature and Crate Boundary Goals

The CLI package should support:

- Headless direct CLI build without TUI dependencies.
- Optional TUI launch feature.
- Optional daemon client feature if transport dependencies are non-trivial.

Suggested feature split:

```toml
[features]
default = ["tui"]
tui = ["dep:eggsec-tui"]
daemon-client = ["dep:eggsec-daemon"]
headless = []
```

The exact feature names can differ if the repo already has conventions. The important invariant is that headless CLI can compile without Ratatui/crossterm.

## CLI Dependency Rules

- `eggsec-cli` may depend on `eggsec-tui` only behind a feature.
- `eggsec-cli` may depend on daemon client code behind a feature if necessary.
- `eggsec-runtime` must not gain CLI/TUI/transport dependencies.
- `eggsec-daemon` should expose a reusable client library if the CLI needs one.

## Direct CLI vs Daemon Client Semantics

Direct CLI commands should continue to execute as they do now through `eggsec` command handlers and enforcement context.

Daemon client commands should send requests to daemon and consume responses/events. They should not silently fall back to direct execution unless explicitly requested, because that can confuse session/accounting semantics.

Add clear flags:

```text
--daemon --socket <path>
--direct
```

If a command supports both direct and daemon mode, make the default mode explicit in docs.

## Execution Surface Mapping

CLI daemon sessions must use correct surface semantics.

Recommended mapping:

- Direct CLI normal manual: `CliManual`.
- Direct CLI strict: `CliManualStrict` or existing strict surface.
- Daemon local manual session created by CLI: `CliManual`, unless a dedicated `DaemonLocalManual` surface exists.
- Programmatic CLI/CI daemon session: strict/CI surface.

Do not let CLI daemon commands create `TuiManual` sessions.

## Headless Build Work

Audit `crates/eggsec-cli/Cargo.toml` and `crates/eggsec-cli/src/main.rs`.

Current concern:

- The CLI may depend on `eggsec-tui` directly for no-command terminal launch.

Target:

- TUI launch is feature-gated.
- If no command and TUI feature is disabled, print help or a clear message.
- Headless CI/server installs can build the CLI without terminal UI dependencies.

## Daemon Client Library

If daemon client logic is more than trivial, add:

```text
crates/eggsec-daemon/src/client.rs
```

Expose functions:

```rust
DaemonClient::connect(socket_path)
DaemonClient::health()
DaemonClient::capabilities()
DaemonClient::create_session(...)
DaemonClient::list_sessions()
DaemonClient::snapshot(session_id)
DaemonClient::submit(session_id, request)
DaemonClient::cancel(session_id, task_id)
DaemonClient::subscribe(session_id)
```

The CLI should use this rather than duplicating protocol framing.

## Output Formatting

Support both human and machine-readable output.

Minimum:

```text
--json
```

For daemon session/task commands, JSON output should include:

- session IDs
- task IDs
- surface
- scope metadata
- task status
- envelope kind/summary/artifacts

For `task watch`, stream newline-delimited JSON if `--json` is set.

## Files Likely to Change

- `crates/eggsec-cli/Cargo.toml`
- `crates/eggsec-cli/src/main.rs`
- `crates/eggsec-cli/src/daemon.rs` or equivalent
- `crates/eggsec-daemon/src/client.rs`
- `crates/eggsec-daemon/src/protocol.rs`
- `crates/eggsec-runtime/src/request.rs`
- `crates/eggsec-runtime/src/session.rs`
- `architecture/overview.md`
- `README.md`
- `AGENTS.md`
- `scripts/check-architecture-guards.sh`

## Implementation Steps

1. Audit current CLI dependency on `eggsec-tui`.
2. Add CLI feature split for TUI dependency.
3. Ensure headless CLI compile path works.
4. Add daemon client library to `eggsec-daemon` if missing.
5. Add daemon-related CLI subcommands.
6. Add JSON output for session/task/capability responses.
7. Add event streaming for `task watch`.
8. Ensure daemon CLI sessions use CLI/manual or strict surfaces, not TUI manual.
9. Add tests for CLI argument parsing and daemon command serialization.
10. Add architecture guard that headless CLI does not require TUI feature.
11. Update docs and help text.

## Tests

Unit tests:

- CLI parses daemon commands.
- CLI maps daemon session surfaces correctly.
- CLI rejects invalid surface strings.
- CLI serializes daemon commands correctly.
- JSON output includes required fields.

Compile tests/checks:

```bash
cargo check -p eggsec-cli --no-default-features
cargo check -p eggsec-cli --no-default-features --features daemon-client
cargo check -p eggsec-cli --features tui
```

Integration tests if practical:

- Start daemon.
- Use CLI client to health-check daemon.
- Use CLI client to create/list session.
- Use CLI client to submit test task.
- Use CLI client to watch events.
- Use CLI client to cancel task.

## Non-Goals

Do not remove default TUI launch behavior for normal builds.

Do not make daemon mode default.

Do not expose daemon over public network.

Do not implement persistence.

Do not redesign all direct CLI command handling.

## Validation

Run:

```bash
cargo fmt --all --check
cargo check -p eggsec-cli --no-default-features
cargo check -p eggsec-cli --no-default-features --features daemon-client
cargo check -p eggsec-cli --features tui
cargo test -p eggsec-cli
cargo check -p eggsec-daemon
cargo test -p eggsec-daemon
cargo check -p eggsec-tui
cargo test -p eggsec-tui
./scripts/check-architecture-guards.sh
```

## Acceptance Criteria

- CLI can build without TUI dependencies.
- Existing normal CLI/TUI launch behavior remains available under default or TUI feature.
- CLI can start/status local daemon or otherwise invoke daemon host as designed.
- CLI can create/list/snapshot sessions through daemon.
- CLI can submit/cancel/watch daemon tasks.
- CLI daemon commands emit useful JSON.
- CLI daemon sessions do not use `TuiManual` surface.
- Architecture docs and guards reflect the new build modes.
