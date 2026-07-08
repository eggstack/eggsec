# Milestone B: Daemon Host Hardening Plan

## Objective

Harden `eggsec-daemon` as a real local runtime host before real Eggsec execution is wired through it. This milestone should close resource-control, configuration, persistence, session lifecycle, and access-control gaps in the daemon layer.

The daemon should remain usable in a protocol-only/noop-executor mode during this milestone. Do not make the daemon depend on the full `eggsec` engine yet; that belongs in Milestone D.

## Current Findings

`DaemonConfig.max_clients` exists but the Unix socket server accepts and spawns clients without enforcing the limit.

The daemon binary currently reads only a positional socket path from `std::env::args()` and otherwise uses defaults. There is no proper CLI parser for data directory, persistence, max clients, default surface, logging, or future transport selection.

Persistence currently saves snapshots on create, submit, cancel, and cancel-active command paths. Runtime task completion occurs inside `eggsec-runtime`, so successful/failed/timed-out terminal states may not be persisted unless another command later triggers snapshot persistence.

`CloseSession` currently records an audit event and returns `SessionClosed`, but it does not appear to remove or terminalize the live runtime session. This makes session close more of a notification than lifecycle mutation.

Persisted-session commands are broadly available to declared clients. That may be acceptable for single-user local use, but it is too loose for a multi-client daemon that may serve TUI, CLI, desktop, and automated clients concurrently.

## Files and Areas to Inspect

- `crates/eggsec-daemon/src/main.rs`
- `crates/eggsec-daemon/src/config.rs`
- `crates/eggsec-daemon/src/server.rs`
- `crates/eggsec-daemon/src/host.rs`
- `crates/eggsec-daemon/src/client_registry.rs`
- `crates/eggsec-daemon/src/protocol.rs`
- `crates/eggsec-daemon/src/store/`
- `crates/eggsec-runtime/src/runtime.rs`
- `crates/eggsec-runtime/src/session.rs`

## Work Item B1: Enforce `max_clients`

### Desired behavior

The daemon must not accept unlimited active clients. `DaemonConfig.max_clients` should cap concurrent client connections.

### Implementation guidance

Use `tokio::sync::Semaphore` in `run_server()`.

When `listener.accept()` succeeds:

1. Try to acquire an owned permit.
2. If a permit is available, move it into the spawned `handle_client` task so it is released on disconnect.
3. If no permit is available, close the connection. If practical, write a JSON error response before closing, but do not block or complicate accept-loop behavior.

Keep the default `max_clients = 10` unless there is a strong reason to change it.

### Tests

Add a server test with `max_clients = 1`:

- connect first client and keep it open;
- attempt a second connection;
- assert the second client cannot complete a normal health roundtrip or receives a deterministic over-limit error if implemented.

Avoid timing-flaky tests by using explicit connection hold and short timeouts.

## Work Item B2: Add Real Daemon CLI Configuration

### Desired behavior

The daemon binary should be configurable without editing code or relying on positional arguments.

### Recommended CLI flags

Add `clap` to `eggsec-daemon` or reuse workspace dependency patterns if already available indirectly.

Minimum flags:

```text
eggsec-daemon \
  --socket /tmp/eggsec-daemon.sock \
  --data-dir ~/.local/share/eggsec/daemon \
  --max-clients 10 \
  --default-surface unknown \
  --log-level info
```

Additional useful flags:

```text
--no-persistence
--json-logs
--protocol-only
```

Do not wire remote/non-loopback HTTP transport in this milestone unless it is already implemented. If a future `--transport` flag is added now, only accept implemented values.

### Implementation guidance

Create a small `DaemonArgs` struct in `main.rs` or a new `cli.rs` module. Convert it into `DaemonConfig`.

`default_surface` parsing should only accept known safe labels from `RuntimeSurface::label()` or a dedicated `FromStr` implementation. Invalid values should produce CLI errors, not silently default.

### Tests

Add parser unit tests if the CLI parsing module is testable without running the daemon:

- default args produce current defaults;
- `--no-persistence` disables persistence;
- custom socket/data-dir/max-clients are honored;
- invalid surface errors.

## Work Item B3: Persist Terminal Runtime Events

### Desired behavior

Persisted session snapshots should reflect terminal task states even when no further client command occurs after task completion.

### Implementation guidance

Add a daemon-owned background subscriber to runtime events. This can live in `DaemonHost` or be spawned from `main.rs` after host creation.

The subscriber should listen for terminal events:

- `TaskCompleted`
- `TaskFailed`
- `TaskCancelled`
- timeout if represented as `TaskCancelled` with timeout reason or another status/event

For each terminal event:

1. Fetch the session snapshot from runtime.
2. Save it with `store.save_session_snapshot(&snapshot)`.
3. Record an audit event if appropriate, or rely on existing command audit plus snapshot persistence.

Do not persist every progress/log event. Avoid high-frequency writes.

### Race considerations

Terminal events are emitted before or around final state updates depending on runtime implementation. If the event is emitted before the state update, the subscriber may snapshot too early. Coordinate with Milestone A. If needed, either emit terminal events after state update or have the subscriber retry once after a short delay when the snapshot does not yet show terminal state.

Preferred fix: update runtime state first, then emit terminal event, or provide a stronger event ordering contract.

### Tests

Add a daemon host test with a test executor that completes immediately:

- create session;
- submit task;
- wait for completion;
- query persisted snapshot;
- assert the persisted snapshot contains a terminal completed task.

Use a temporary SQLite store if practical. If existing store tests are no-op only, add a focused SQLite integration test behind existing test infrastructure.

## Work Item B4: Implement Real Close-session Lifecycle

### Desired behavior

`CloseSession` should close or remove the live session so it can no longer accept task submissions.

### Implementation options

Option 1: remove closed sessions from runtime state.

- Add `Runtime::close_session(session_id)`.
- Cancel active tasks.
- Remove the session from the live map.
- Persist the final snapshot before removal if persistence is enabled.

Option 2: add a closed flag to `RuntimeSession`.

- Add `SessionStatus` or `closed: bool`.
- `submit()` rejects closed sessions with a new `RuntimeError::SessionClosed`.
- Snapshots can still be retrieved from live runtime after close.

Option 2 is preferable for frontend UX because the user can close a session while still inspecting its final state during the same daemon lifetime. Persistence can then keep it available after restart.

### Tests

- Close an existing session returns `SessionClosed`.
- Submit after close returns an error.
- Close cancels active tasks or refuses close when active depending on chosen semantics. Prefer close-cancels-active with an explicit cancellation reason.
- Snapshot after close works if using closed flag; if using removal, snapshot should return not found and persisted snapshot should remain available.

## Work Item B5: Tighten Persisted-session Access Control

### Desired behavior

Persisted session inventory and snapshots should not be globally readable by arbitrary declared clients in multi-client mode.

### Implementation guidance

Extend the access model to distinguish local single-user development from stricter multi-client behavior.

Minimum safe improvement:

- `GetPersistedSnapshot { session_id }` should require observer access to that session if it is known in `session_access`.
- If the session is not known live, persisted access should require owner/admin metadata from the persisted snapshot or a daemon admin role.

Since persisted snapshots may not currently store owner client ID, add persistence of enough session access metadata to make this enforceable. If that is too large for this milestone, add a config field such as `allow_global_persisted_session_reads: bool` defaulting to true only in local/single-user mode, with docs marking it temporary.

Preferred long-term model:

- session access metadata is persisted with session snapshots;
- owner and allowed clients can read their own persisted sessions;
- daemon-internal/admin clients can list all;
- arbitrary declared clients cannot list/read everything.

### Tests

Add host-level tests:

- owner can read its persisted session;
- unrelated declared client cannot read persisted snapshot when strict persisted access is enabled;
- daemon internal/admin client can list/read if such a role exists.

## Work Item B6: Improve Capability Honesty

### Desired behavior

Daemon capabilities should tell clients what is actually available.

In protocol-only/noop mode, task execution should be reported as unavailable or unsupported. In future full mode, supported task kinds and transports should be advertised accurately.

### Implementation guidance

Extend `DaemonCapabilities` or `RuntimeCapabilities` if needed to include:

- executor mode: `noop`, `limited`, `full`;
- supported task kinds;
- enabled transports;
- persistence enabled/disabled;
- policy approval support enabled/disabled.

Do not overbuild. The immediate problem is avoiding a daemon that claims generic runtime capability while every task returns `UnsupportedTaskKind`.

### Tests

- Noop daemon advertises protocol/session support but not real task execution.
- Capabilities reflect persistence disabled when `--no-persistence` is used.

## Validation Commands

Run at minimum:

```bash
cargo test -p eggsec-daemon
cargo test -p eggsec-runtime
cargo check -p eggsec-daemon --all-targets
```

If adding `clap` or changing protocol DTOs affects workspace compilation, also run:

```bash
cargo check --workspace --all-targets
```

## Acceptance Checklist

- [ ] `max_clients` is enforced.
- [ ] Daemon has a real CLI parser and no longer relies only on positional socket arg.
- [ ] Terminal task states are persisted from runtime events.
- [ ] `CloseSession` performs real lifecycle mutation.
- [ ] Persisted-session access is tightened or clearly config-gated as temporary.
- [ ] Capabilities distinguish noop/protocol-only behavior from real execution.
- [ ] Daemon tests cover client limit, CLI config, persistence, close semantics, and access control.

## Handoff Notes

Do not wire real scan execution in this milestone. Keep the daemon host robust but still executor-agnostic. Real execution belongs in Milestone D after the runtime/enforcement bridge from Milestone C is complete.
