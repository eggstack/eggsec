# Desktop GUI Frontend Milestone 002 — Frontend-Client Contract Extraction

Status: ready for handoff

Repository baseline: `203ab5dd958415340d39cd47fb1add432c687edd` (`feat/auto-20261022-76692679`)

Source roadmap:

- `plans/subsystems/desktop-gui-frontend-roadmap.md#milestone-2--frontend-client-contract-extraction`

Long-term requirements:

- `plans/000-long-term-specification.md#5-crate-ownership`
- `plans/000-long-term-specification.md#6-dispatch-runtime-and-wire-identity`
- `plans/000-long-term-specification.md#8-runtime-and-dependency-baselines`
- `plans/002-long-term-roadmap.md#phase-7--standing-maintenance-and-future-capability-open`

Applicable ADRs:

- `plans/adrs/ADR-0007-desktop-gui-frontend-boundary.md`

Primary class: infrastructure

## 1. Objective

Relocate the runtime-client contract out of the TUI crate into a shared,
frontend-neutral home, so that more than one interactive frontend can
implement it — and so that a second frontend does not have to either
duplicate the trait or import it across a frontend boundary.

Concretely: move `TuiRuntimeClient` and its two implementations
(`EmbeddedRuntimeClient`, `DaemonRuntimeClient`) out of
`crates/eggsec-tui/src/runtime_client/` into a shared crate, migrate the TUI
onto the shared contract with **no behavioral change**, and rename the trait
so it no longer names a single frontend.

## 2. Why this milestone is ready

- **No hard dependencies.** This milestone is independent of M001 (schema
  publication) and M003 (GUI surface parity), and may proceed in parallel
  with them.
- **Stable interface dependency.** `ADR-0007` is accepted and fixes the
  boundary: the GUI reaches the engine only through the daemon protocol, and
  the frontend-client contract is the shared place where that boundary is
  expressed.
- **Existing code to relocate, not design.** The trait and both
  implementations already exist and are already exercised by the TUI. This
  is a move plus a rename, with a test suite that already covers the
  behavior.

## 3. Current implementation evidence

### The contract as it exists today

`crates/eggsec-tui/src/runtime_client/mod.rs` defines:

```rust
pub type RuntimeClientFuture<T> =
    Pin<Box<dyn Future<Output = Result<T, String>> + Send + 'static>>;

pub trait TuiRuntimeClient: Send + Sync + 'static {
    fn capabilities(&self) -> RuntimeClientFuture<RuntimeCapabilities>;
    fn create_session(
        &self,
        surface: eggsec_runtime::request::RuntimeSurface,
        scope: Option<eggsec_runtime::session::SessionScope>,
        labels: Vec<String>,
    ) -> RuntimeClientFuture<SessionId>;
    fn list_sessions(&self) -> RuntimeClientFuture<Vec<SessionSummary>>;
    fn snapshot(&self, session_id: SessionId) -> RuntimeClientFuture<SessionSnapshot>;
    fn submit(&self, session_id: SessionId, request: RunRequest) -> RuntimeClientFuture<TaskId>;
    fn cancel(&self, session_id: SessionId, task_id: TaskId) -> RuntimeClientFuture<()>;
    fn cancel_active(&self, session_id: SessionId) -> RuntimeClientFuture<()>;
    fn subscribe(&self, session_id: SessionId) -> RuntimeClientFuture<RuntimeEventReceiverHandle>;
}
```

It also defines `RuntimeEventReceiverHandle`, a wrapper over
`tokio::sync::mpsc::UnboundedReceiver<RuntimeEvent>` exposing `recv()` and
`try_recv()`.

The module exports `DaemonRuntimeClient` and `EmbeddedRuntimeClient` from
submodules `daemon.rs` and `embedded.rs`.

### The two implementations

- `EmbeddedRuntimeClient` (`crates/eggsec-tui/src/runtime_client/embedded.rs`)
  wraps an in-process `Arc<eggsec_runtime::Runtime>` and delegates each
  method to the corresponding runtime call. `capabilities()` currently
  returns `RuntimeCapabilities::default()`.
- `DaemonRuntimeClient` (`crates/eggsec-tui/src/runtime_client/daemon.rs`)
  speaks the daemon's Unix-socket JSON-lines protocol using
  `eggsec_daemon_protocol::protocol::{ClientCommand, ServerMessage}`. It
  maintains a request counter, a pending-response map of
  `oneshot::Sender<ServerMessage>`, and a fan-out registry of
  `mpsc::UnboundedSender<RuntimeEvent>` subscribers, with a spawned reader
  task routing responses and events. Its error type is `DaemonClientError`
  (`Io`, `Serialization`, `Protocol`, `ConnectionClosed`).

### Dependency facts

- `eggsec-tui` already depends on `eggsec-runtime` and
  `eggsec-daemon-protocol`; `DaemonRuntimeClient`'s import of
  `eggsec_daemon_protocol::protocol` is therefore not a new edge.
- The trait's signatures use only `eggsec_runtime` types
  (`RunRequest`, `RuntimeCapabilities`, `RuntimeEvent`, `SessionId`,
  `SessionSnapshot`, `SessionSummary`, `TaskId`,
  `RuntimeSurface`, `SessionScope`) — **no** engine types and **no** TUI
  types. The contract is already frontend-neutral in substance; only its
  name and location are TUI-shaped.
- The trait's `RuntimeClientFuture` boxes a `Send + 'static` future.
- `eggsec-runtime` is free of engine and domain dependencies (guard 22) and
  free of TUI dependencies (guard 11). A new crate depending on
  `eggsec-runtime` and `eggsec-daemon-protocol` would inherit that posture.

### Consumers

The TUI is the current consumer. The concrete call sites are in the TUI's
app layer — the session/task management code under
`crates/eggsec-tui/src/app/` (including `task_runtime.rs` and
`task_management.rs`) and the TUI's runtime wiring. The implementing agent
MUST enumerate the actual call sites at the baseline rather than assuming
them.

## 4. Invariants that must not regress

- **No behavioral change to the TUI.** This is a relocation. The TUI's
  observable behavior, its terminal-ownership rules, its single-writer rule,
  its `!self.is_running()` input guards, and its `reset()` semantics are
  untouched.
- Production `eggsec-tui` writes no terminal bytes (guard check 138).
- `EnforcementContext::evaluate()` remains the mandatory pre-dispatch gate.
  The relocated contract MUST NOT become an enforcement path: it is a client
  of an already-authorized daemon, and it MUST NOT import enforcement
  internals.
- The engine MUST NOT depend on the new crate, and the new crate MUST NOT
  depend on the engine. The new crate depends only on frontend-neutral
  crates (`eggsec-runtime`, `eggsec-daemon-protocol`).
- `eggsec-tui` MUST NOT depend on the engine crate (guard 19 covers the
  `eggsec-cli` → `eggsec-tui` edge; guard 21 covers engine → TUI).
- The daemon's default dependency set stays `eggsec-runtime` +
  `eggsec-daemon-protocol` only (guard 16).
- Workspace Tokio baseline stays `default-features = false` with per-crate
  feature lists. The new crate needs `tokio` with `rt`, `macros`, `sync`,
  and `time` — the same set `eggsec-runtime` uses — plus `io-util` for the
  daemon client's `AsyncBufReadExt`/`AsyncWriteExt` usage, if the client
  moves rather than the trait alone.
- Never `let _ =` or `filter_map(|e| e.ok())`; log via `tracing`.
- All spawned Tokio tasks need 30–300s timeout wrappers — note that the
  daemon client's reader task is a long-lived loop and must be exempt by
  design, not by accident; record the reasoning in the closure record.
- MSRV 1.89, license allow-list, and the `make check` contract remain
  intact.

## 5. Scope

### In scope

- Creating the shared frontend-client home for the runtime-client contract.
- Moving `RuntimeClientFuture`, the trait, `RuntimeEventReceiverHandle`,
  `EmbeddedRuntimeClient`, `DaemonRuntimeClient`, and `DaemonClientError`
  into it.
- Renaming the trait so it no longer names the TUI, and updating every
  call site and doc reference.
- Migrating `eggsec-tui` onto the shared contract.
- A guard proving the new crate depends on no engine crate and no frontend
  crate.
- Documentation and skill updates listed in §12.

### Explicitly out of scope

- Any change to the trait's method set or signatures beyond the rename and
  any required re-export. If a signature change is genuinely needed, it
  MUST be justified in the closure record — and it is a stop condition if
  it would change observable behavior.
- Any change to the daemon protocol.
- Any change to the runtime DTOs.
- The GUI surface variant (M003).
- Schema publication (M001).
- Creating the GUI crate. **This milestone must not create
  `crates/eggsec-gui`.** The point of the extraction is to prepare a shared
  home; the GUI lands in M004.
- Migrating the TUI's hand-built forms.
- Any change to the TUI's rendering, tabs, or dispatch wiring.

## 6. Required production changes

### Core/domain

- Create the shared contract. Two placements are acceptable and the
  implementing agent selects one on measured dependency payoff:
  - **A new leaf crate** (for example `crates/eggsec-frontend-client`)
    depending on `eggsec-runtime` and `eggsec-daemon-protocol`. This is
    preferred, because it gives the GUI a crate to depend on without
    depending on the TUI, and it makes the ownership boundary explicit in
    the dependency graph.
  - **A new module in `eggsec-runtime`** (for example
    `eggsec_runtime::client`). This is acceptable only if adding
    `eggsec-daemon-protocol` as a dependency of `eggsec-runtime` does not
    create a cycle and does not violate the crate's dependency-light
    posture. **Note:** `eggsec-daemon-protocol` depends on `eggsec-runtime`,
    so a module inside `eggsec-runtime` **cannot** hold the daemon client —
    only the trait and the embedded client. If the daemon client must live
    with the trait, a separate crate is required.
- Resolve the cycle question explicitly and record it. The trait and the
  embedded client have no `eggsec-daemon-protocol` dependency; the daemon
  client does. Splitting the trait from the daemon client across two crates
  is a valid outcome and must not be treated as a failure.
- Rename the trait to a frontend-neutral name (for example
  `RuntimeClient`) and update the module and type names accordingly.
- Keep `RuntimeClientFuture` as the boxed-future alias. Its error type is
  `String`, which is coarse but is the current contract; changing it is out
  of scope and would ripple into every call site.

### Storage and migrations

None. No persisted representation changes.

### Protocol and DTOs

None. The daemon protocol and the runtime DTOs are unchanged. The daemon
client continues to speak the existing `ClientCommand` / `ServerMessage`
vocabulary.

### Runtime and concurrency

- The daemon client's reader task, the pending-response map, and the
  event fan-out registry move unchanged. Their concurrency behavior —
  one reader task, `oneshot` correlation by request id, `UnboundedSender`
  fan-out to subscribers — MUST be preserved exactly.
- The moved code MUST keep using `tracing` for diagnostics; the daemon
  client's existing `tracing::warn!` on a message-parse failure is the
  pattern to preserve.

### Frontend or operator surface

None. The TUI's observable behavior is unchanged.

### Security and authorization

- The shared contract MUST NOT import enforcement internals. It is a
  protocol client, not an authorization path. Add a guard asserting the
  new crate references neither `EnforcementContext`, `EnforcedDispatcher`,
  `Scope`, nor the engine crate.
- The daemon client MUST continue to use the daemon's own access controls.
  It does not and must not bypass them.

### Documentation and static guards

- Add a guard proving the new crate depends on no engine crate and no
  frontend crate (no `eggsec-tui`, no `eggsec-gui`).
- Add a guard proving the engine and the daemon crates do not depend on the
  new crate.
- Update the architecture documentation to describe the new ownership
  boundary.

## 7. Ordered work packages

### Work package A — Enumerate the actual surface

Intent: establish the real call-site set at the baseline before moving
anything.

Required changes:

- Enumerate every reference to `TuiRuntimeClient`,
  `RuntimeClientFuture`, `RuntimeEventReceiverHandle`,
  `EmbeddedRuntimeClient`, `DaemonRuntimeClient`, and `DaemonClientError`
  across the workspace, including doc comments, tests, and the TUI's
  `AGENTS.override.md` and `architecture/tui.md`.
- Record the count in the closure record. This is the regression baseline
  for WP D.

Acceptance evidence:

- A complete, reproducible reference list exists (for example
  `rg -n 'TuiRuntimeClient|RuntimeEventReceiverHandle|EmbeddedRuntimeClient|DaemonRuntimeClient'`),
  and no reference is missed.

### Work package B — Create the shared contract and move

Intent: relocate the contract into its new home without changing what it
does.

Required changes:

- Create the chosen crate or module with a manifest that depends only on
  `eggsec-runtime` and `eggsec-daemon-protocol` (plus Tokio features as
  needed and `tracing`).
- Move the trait, the future alias, the event-receiver handle, and the two
  implementations.
- Rename to the frontend-neutral name chosen in §6.
- Preserve the public method signatures exactly.
- Preserve the daemon client's error type and its variants.

Acceptance evidence:

- The moved code compiles with no logic changes; the diff shows moves and a
  rename, not rewrites.

### Work package C — Migrate the TUI

Intent: point the TUI at the shared contract with no behavioral change.

Required changes:

- Update every call site, import, and test in `eggsec-tui` to use the
  shared contract.
- Update the TUI's own `AGENTS.override.md` if it references the old paths.
- Keep the TUI's daemon-attach and embedded-runtime modes behaving exactly
  as before, including the `runner::block_on_ambient` rule for daemon sync
  paths (never a nested `Runtime::new`) and the single-terminal-writer rule.

Acceptance evidence:

- The TUI's full test suite is green.
- The TUI's daemon-attach and embedded modes are both exercised by the
  existing tests; if a mode is not currently covered, note the gap in the
  closure record rather than adding a new test silently.

### Work package D — Guards, docs, and verification

Intent: freeze the ownership boundary.

Required changes:

- Add the guards from §6.
- Update `architecture/tui.md`, `architecture/overview.md` (crate table and
  dependency map), and `architecture/daemon.md` (the daemon client's new
  home) to describe the new boundary.
- Update the matching skill under `.opencode/skills/` if it names the old
  paths or counts crates. **The crate count changes if a crate is added** —
  update `architecture/overview.md`'s headline count and the skill
  accordingly, per the repository's single-source rule.

Acceptance evidence:

- The guards fail when deliberately violated.
- `make check` is green.
- The crate count in `architecture/overview.md` and any skill that restates
  it are consistent.

## 8. Failure, cancellation, restart, and contention semantics

- **Partial failure.** The daemon client's error mapping
  (`DaemonClientError::{Io, Serialization, Protocol, ConnectionClosed}`) is
  preserved exactly, including how it maps into the trait's `String` error.
- **Duplicate delivery.** The request-id → `oneshot` correlation is
  unchanged. The event fan-out continues to deliver to all active
  subscribers.
- **Process restart.** The client holds no persistent state; a reconnect
  re-establishes the connection and re-subscribes. This behavior must not
  change.
- **Cancellation races.** `cancel` and `cancel_active` are request/response
  operations. Their semantics are unchanged by the move.
- **Stale generations.** The client does not version its own state; the
  daemon owns session and task truth. Unchanged.
- **Concurrent callers.** The pending-response map and the subscriber
  registry are already shared across callers via `Arc` and `RwLock`. The
  move must preserve that sharing. A subtle failure mode to watch for:
  cloning the connection per call would silently break fan-out, so the
  `Arc<DaemonConnection>` shape must be preserved.

## 9. Compatibility and migration

- **Wire protocol.** Unchanged. The daemon client speaks the same
  `ClientCommand` / `ServerMessage` vocabulary before and after.
- **Rust API.** `eggsec-tui`'s *public* API may change in the sense that
  `TuiRuntimeClient` and friends are no longer exported from that crate —
  this is the intended effect and the reason for the milestone. Consumers
  of `eggsec-tui` are unaffected in practice because the TUI is an
  application crate, not a published library; confirm that `eggsec-tui` is
  not published to crates.io before treating this as a breaking change.
- **Enforcement.** No change.
- **Existing frontends.** The TUI's behavior is unchanged. The CLI, REST,
  MCP, gRPC, and Python surfaces are untouched.

## 10. Required tests

### Focused unit tests

- The moved trait and both implementations compile and are exercised by
  existing tests, relocated with them.
- The daemon client's request/response correlation resolves the correct
  response for a given request id, and the event fan-out delivers to all
  subscribers.

### Integration tests

- A client against a live daemon can create a session, submit a task,
  observe events, and cancel — using the shared contract.
- The embedded client can create a session, submit a task, observe events,
  and cancel — using the shared contract.

### Restart and recovery tests

- A client that loses its connection reports `ConnectionClosed` and can
  reconnect and re-subscribe. This is existing behavior that the move must
  not regress.

### Contention and cancellation tests

- Multiple concurrent subscribers each receive events.
- `cancel` and `cancel_active` behave as before.

### Security and negative tests

- A guard proves the shared crate imports no enforcement internals and
  depends on no engine crate.
- A guard proves the engine and daemon crates do not depend on the shared
  crate.

### Migration and compatibility tests

- The TUI's full existing test suite is green with no test deleted or
  weakened. If a test is removed rather than relocated, that is a stop
  condition.
- The reference list from WP A resolves to zero remaining references to the
  old names.

## 11. Required verification commands

```bash
make check                  # mandatory Rust contract
make check-msrv             # the new crate must not raise the floor
bash scripts/check-architecture-guards.sh
```

If the new crate's feature surface changes the runtime or daemon crates'
default builds:

```bash
cargo check --workspace --no-default-features
```

Do not claim commands that were not actually run in the closure record.

## 12. Documentation updates

- `architecture/tui.md` — the TUI's runtime-client paths, and the fact that
  the contract is now shared.
- `architecture/overview.md` — the crate table, the dependency map, and the
  headline crate count if a crate was added.
- `architecture/daemon.md` — the daemon client's new home.
- `architecture/runtime.md` — the shared contract's relationship to the
  runtime DTOs.
- `crates/eggsec-tui/src/AGENTS.override.md` — if it references the old
  paths.
- `docs/DEPENDENCY_EXCEPTIONS.md` — only if a new dependency is introduced
  (the intended dependency set introduces none).
- `plans/registry.md` — this milestone's status.
- The matching skill under `.opencode/skills/` — if it names the old paths
  or counts crates.

## 13. Acceptance criteria

1. The runtime-client contract lives in a frontend-neutral location whose
   name does not name a single frontend.
2. The shared location depends only on frontend-neutral crates — no engine
   crate, no TUI crate, and (once M004 exists) no GUI crate.
3. `eggsec-tui` is migrated onto the shared contract with **no observable
   behavior change**, and its full test suite is green with no test removed
   or weakened.
4. The daemon client's request/response correlation, event fan-out, error
   mapping, and connection-lifecycle behavior are unchanged.
5. No existing protocol message shape and no runtime DTO changed.
6. A guard proves the shared contract is not an enforcement path and that
   the engine and daemon crates do not depend on it.
7. `make check` and `make check-msrv` are green, and the crate count and
   dependency map in `architecture/overview.md` are consistent with reality.

## 14. Stop conditions

The agent must stop and report rather than improvise when:

- relocating the contract would require a dependency cycle that cannot be
  resolved by splitting the trait from the daemon client — report the cycle
  and the proposed split rather than forcing a cycle;
- the shared contract cannot be expressed without importing an engine type
  — report the specific type and why, rather than widening the contract's
  dependency surface;
- the move would change any observable TUI behavior — this milestone is a
  relocation, and a behavior change means the scope is wrong;
- a test would have to be deleted or weakened to make the move pass — a
  failing test is evidence, not an obstacle;
- the daemon client's behavior cannot be preserved exactly — report rather
  than reimplementing it;
- the engine crate would need to depend on the new crate — that reverses the
  dependency direction this milestone exists to establish.

## 15. Closure evidence required

- The implementation commits or pull requests.
- A requirement-to-evidence matrix mapping each §13 acceptance criterion to
  the specific test or guard that proves it.
- The reference list from WP A and its final resolution, proving no old
  name survives.
- A statement of whether the trait, the embedded client, and the daemon
  client ended up in one location or two, and why.
- The `make check`, `make check-msrv`, and guard run outcomes, including any
  command not run and why.
- The crate count and dependency-map diff for `architecture/overview.md`.
- The reasoning for the reader task's exemption from the 30–300s timeout
  wrapper convention.
- Known limitations.
- Unresolved findings classified by severity.
- Recommendation: closed, conditionally closed, corrective pass required, or
  blocked.

## 16. Handoff notes

- **Baseline.** `203ab5dd958415340d39cd47fb1add432c687edd` on
  `feat/auto-20261022-76692679`.
- **This is a move, not a redesign.** If you find yourself changing
  behavior, adding methods, or improving the daemon client's design, stop.
  Those are separate changes with their own justification.
- **Watch the `Arc` shape.** The daemon client must keep sharing one
  `Arc<DaemonConnection>` across callers. Cloning the connection per call
  would silently break event fan-out, and the symptom (one subscriber
  receiving events) looks like a flaky test rather than a clone bug.
- **The trait's `String` error type is the current contract.** It is coarse.
  Improving it is tempting and out of scope; it would ripple into every TUI
  call site.
- **Do not create `crates/eggsec-gui` here.** The extraction prepares a
  shared home; the GUI arrives in M004.
- **Verification.** `make check` is the minimum. Preserve unrelated user
  changes in the worktree.
- **Guard numbering.** Existing checks are numbered; add new checks with the
  next free number and update the guard script's own header count if it
  tracks one.
