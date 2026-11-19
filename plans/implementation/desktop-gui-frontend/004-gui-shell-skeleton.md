# Desktop GUI Frontend Milestone 004 — GUI Shell Skeleton

Status: executed — closed 2026-10-23

Repository baseline: `feat/auto-20261022-76692679` working tree (branch baseline commit `203ab5dd958415340d39cd47fb1add432c687edd`), with milestones 001–003 implemented and uncommitted.

Source roadmap:

- `plans/subsystems/desktop-gui-frontend-roadmap.md#milestone-4--gui-shell-skeleton`

Long-term requirements:

- `plans/000-long-term-specification.md#5-crate-ownership`
- `plans/000-long-term-specification.md#10-frontend-rules`
- `plans/001-terminology-and-domain-model.md`

Applicable ADRs:

- `plans/adrs/ADR-0007-desktop-gui-frontend-boundary.md`
- `plans/adrs/ADR-0009-gui-shell-technology-selection.md`

Primary class: capability

## 1. Objective

Stand up `crates/eggsec-gui` as a leaf application crate that launches a Tauri v2
window, connects to a running daemon through the shared frontend client, creates
a session stamped `RuntimeSurface::DesktopGuiManual`, and renders the operation
catalog the daemon reports.

This is the smallest end-to-end vertical slice. Nothing is submitted to the
engine yet: the deliverable boundary is a window that shows the engine's
operation surface.

## 2. Why this milestone is ready

Closed hard dependencies:

- Milestone 001 (closed 2026-10-23) — `ClientCommand::DescribeOperations`
  (`crates/eggsec-daemon-protocol/src/protocol.rs:183`) and
  `ServerMessage::Operations` exist on the wire, and the daemon answers the
  command (`crates/eggsec-daemon/src/host.rs:348`).
- Milestone 002 (closed 2026-10-23) — `eggsec-frontend-client` owns the
  `RuntimeClient` contract (`crates/eggsec-frontend-client/src/lib.rs:36`) and
  its daemon-backed implementation.
- Milestone 003 (closed 2026-10-23) — `RuntimeSurface::DesktopGuiManual` and
  `ExecutionSurface::DesktopGuiManual` exist, convert both ways through
  `runtime_bridge`, and are classified manual by `eggsec-policy`.
- `ADR-0009` (accepted 2026-10-23) — the shell technology is decided: Tauri v2,
  pinned to the 2.11.x line.

Stable interface dependencies, with one gap to close first:

- `DaemonRuntimeClient::connect` exists
  (`crates/eggsec-frontend-client/src/daemon.rs:56`) but hardcodes the TUI's
  client identity — `ClientKind::Tui` at `:169-170` and the `"tui-"` request-id
  prefix at `:195-197`. The GUI cannot identify itself correctly until that
  seam is widened. That widening is Work package A of this milestone.

## 3. Current implementation evidence

### The frontend client seam

`crates/eggsec-frontend-client/src/lib.rs:36` defines `RuntimeClient` with
exactly eight methods: `capabilities`, `create_session`, `list_sessions`,
`snapshot`, `submit`, `cancel`, `cancel_active`, `subscribe`. It is object-safe
and `Send + Sync + 'static`. The crate depends on `eggsec-runtime` and
`eggsec-daemon-protocol` only — no engine, no framework.

`crates/eggsec-frontend-client/src/daemon.rs:56`:

```rust
pub async fn connect(socket_path: &str) -> Result<Self, DaemonClientError> {
```

The constructor connects the Unix socket, performs the handshake, and then
declares the client — `crates/eggsec-frontend-client/src/daemon.rs:169-170`:

```rust
            .declare_client(
                eggsec_daemon_protocol::client_registry::ClientKind::Tui,
```

Request ids are prefixed at `crates/eggsec-frontend-client/src/daemon.rs:195-197`:

```rust
    fn next_request_id(&self) -> String {
        ...
        format!("tui-{}", id)
```

Both are TUI-specific and live in a crate that milestone 002 deliberately made
frontend-neutral. The GUI needs its own kind and its own request-id namespace.

`DaemonRuntimeClient` already routes `ServerMessage::Operations` to the
pending-response registry, so the wire plumbing for the operation catalog is in
place — only a public method to send the command is missing.

### The embedded client is not an option for the GUI

`crates/eggsec-frontend-client/src/embedded.rs` implements `RuntimeClient`
against an in-process engine `Runtime` supplied by the caller. The GUI cannot
use it: `ADR-0007` confines engine access to the daemon protocol, and an
embedded GUI would link the engine crate. The GUI is therefore
**daemon-only by construction**, not by configuration.

### The client kind enum has no GUI variant

`crates/eggsec-daemon-protocol/src/client_registry.rs:11-20`:

```rust
pub enum ClientKind {
    Cli,
    Tui,
    DaemonInternal,
    Mcp,
    Rest,
    Agent,
    #[default]
    Unknown,
}
```

Every use of `ClientKind` in the workspace is a construction or a `matches!`
predicate — there is no exhaustive `match` on it — so adding a variant is
additive and low blast radius.

### The approval matrix does not know about the GUI

`crates/eggsec-daemon-protocol/src/client_registry.rs:185-201` special-cases
the manual surfaces for policy approval:

```rust
        CommandPermission::Approver => match session_surface {
            RuntimeSurface::TuiManual | RuntimeSurface::CliManual => match client_role {
```

`RuntimeSurface::DesktopGuiManual` is absent. Milestone 003 made the GUI manual
in `eggsec-policy` (`honors_manual_override()` returns true for it), but this
matrix hardcodes the manual surfaces separately and was not updated. A GUI
session therefore falls into the strict branch — owner-only approval — which
contradicts the surface classification milestone 003 established. `ApprovePolicy`
is currently unwired (the daemon answers it with `ErrorCode::Unsupported`), so
no approval is granted either way today; the matrix is still inconsistent.

### How a client finds the daemon

There is **no** socket-path environment variable and no XDG lookup. The default
is hardcoded in three independent places:

- `crates/eggsec/src/cli/mod.rs:245-252` — the global `--socket` flag, default
  `/tmp/eggsec-daemon.sock`.
- `crates/eggsec-daemon/src/config.rs:19-29` — `DaemonConfig::default()`.
- `crates/eggsec-daemon/src/main.rs:19-21` — the standalone binary's
  `--socket-path` flag.

The TUI drives the choice with global flags `--runtime <embedded|daemon>`,
`--socket <PATH>`, `--session <ID>`, `--new-session`, and `--attach-latest`
(`crates/eggsec/src/cli/mod.rs:236-266`), mapping to
`RuntimeMode::Daemon { socket_path, session_id, new_session, attach_latest }`
(`crates/eggsec-tui/src/lib.rs:21-34`). The GUI mirrors the daemon half of
this pattern.

### How a session gets its surface

`crates/eggsec-tui/src/app/runner.rs:435` creates a daemon session with:

```rust
                eggsec_runtime::RuntimeSurface::TuiManual,
```

The GUI mirrors this with `RuntimeSurface::DesktopGuiManual`.

The TUI derives its `SessionScope` from the engine's loaded scope via
`eggsec::config::session_scope_from_loaded` — an **engine-crate** call the GUI
cannot make. `SessionScope` itself lives in `eggsec-runtime`
(`crates/eggsec-runtime/src/session.rs:58`) and is `Serialize + Deserialize +
Default` with public fields, so the GUI can construct one directly:

```rust
pub struct SessionScope {
    pub is_explicit: bool,
    pub source: String,
    pub path: Option<String>,
}
```

### The daemon the GUI must talk to

`crates/eggsec-daemon/Cargo.toml:9-18` — the crate is a library plus an
`eggsec-daemon` binary. Its default features are empty; the engine is pulled
in only behind the optional `full-executor` feature:

```toml
full-executor = ["dep:eggsec", "eggsec/cli"]
```

The binary selects its executor at `crates/eggsec-daemon/src/main.rs:102-117`.
Without `--full-executor` it hosts `DaemonHost::new_noop`, whose executor
rejects every task.

`crates/eggsec-cli/src/daemon_cli.rs:42-76` — `eggsec daemon start` also uses
`NoopExecutor`, so the CLI's daemon subcommand is **not** a usable daemon for a
GUI either.

The consequences are firm:

- The GUI **must not** depend on `eggsec-daemon`. Even without the
  `full-executor` feature the dependency would be a daemon-side coupling that
  `ADR-0007` forbids in spirit and that would grow into an engine edge the
  moment the feature is enabled.
- The GUI connects to an **already-running** `eggsec-daemon --full-executor`
  process over the Unix socket.

### The Tauri dependency facts, verified at the baseline

Verified against the crates.io sparse index and the crates.io API:

| Crate | Version to pin | MSRV | License |
| --- | --- | --- | --- |
| `tauri` | `=2.11.6` | 1.77.2 | Apache-2.0 OR MIT |
| `tauri-build` | `=2.6.3` | 1.77.2 | Apache-2.0 OR MIT |
| `tauri-runtime-wry` | `=2.11.4` | 1.77.2 | Apache-2.0 OR MIT |
| `tauri-runtime` | `=2.11.3` | 1.77.2 | Apache-2.0 OR MIT |
| `tauri-macros` | `=2.6.3` | 1.77.2 | Apache-2.0 OR MIT |
| `tauri-utils` | `=2.9.3` | 1.77.2 | Apache-2.0 OR MIT |

`tauri` 2.11.6 was published 2026-09-19 and is the **latest** published patch
on the 2.11.x line. The 2.12 line declares MSRV 1.90 — above the workspace's
1.89 — and is therefore out of bounds. `tauri` 2.11.6 requires
`tauri-runtime ^2.11.3` and `tauri-runtime-wry ^2.11.4`, so pinning `tauri` at
`=2.11.6` transitively holds the runtime crates on the 2.11.x line.

The transitive license closure was checked crate by crate against the
`deny.toml` allow-list and is **fully covered**: `wry`, `tao`, `winit`, `muda`,
`glutin`, `raw-window-handle`, and the Linux GTK stack (`gtk4`, `gdk4`,
`glib`, `gio`, `glib-sys`, `webkit2gtk`, `webkit2gtk-sys`,
`javascriptcore-rs`, `soup3`) are all MIT, Apache-2.0, or Zlib — the
gtk-rs bindings are MIT, not LGPL. **No new `deny.toml` exception is expected.**
The implementer must still re-verify on the real resolved graph.

The `tauri` 2.11.6 feature map was inspected: `rustls-tls` expands to
`['reqwest/rustls-no-provider', 'dep:rustls']` and `native-tls` to
`['reqwest/native-tls']`. Neither is in `tauri`'s default feature set. The
`default` set is `["wry", "compression", "common-controls-v6", "dynamic-acl",
"x11", "dbus"]`.

### The emitted schema shape

Milestone 001's artifact is the input milestone 005 consumes, so its exact
shape was measured rather than assumed. `task_kind_schemas()`
(`crates/eggsec-runtime/src/schema.rs:227`) returns 30 entries; the whole-enum
artifact is a JSON Schema **draft 2020-12** document whose top-level keys are
`$defs`, `$schema`, `description`, `oneOf`, `title`, with 30 `oneOf` branches
and 30 `$defs` entries.

Measured over all 30 per-variant schemas:

- 149 properties in total.
- **No** per-variant schema contains `$ref` or `$defs` — each is self-contained.
- **No** per-variant schema contains an `enum` constraint. Enum-like fields
  (for example `PortScanParams::scan_type`) are plain `string`, so the schema
  cannot drive a dropdown for them.
- Property type signatures: `string`, `string|null`, `integer`, `integer|null`,
  `boolean`, `boolean|null`, `array|null`.
- Array items are always `{"type": "string"}`.
- Numeric formats present: `uint`, `uint16`, `uint32`, `uint64`.
- Exactly one property has no `type`: `IntegrationsParams::config`, whose
  schema is the boolean `true` (an unconstrained JSON value).

### Known gaps at the baseline

- `ClientKind` has no `Gui` variant.
- `DaemonRuntimeClient::connect` hardcodes the TUI identity.
- The approval matrix does not list `DesktopGuiManual` as manual.
- `DaemonRuntimeClient` exposes no public way to send `DescribeOperations`.
- `crates/eggsec-gui` does not exist.
- The workspace is 24 crates; `architecture/overview.md`,
  `plans/000-long-term-specification.md`, and the registry all carry that
  count.
- CI installs only `ripgrep` on Linux (`.github/workflows/ci.yml:48`,
  `:102`). The Linux webview system packages are not installed anywhere.

## 4. Invariants that must not regress

- **The engine stays behind the daemon protocol.** `crates/eggsec-gui` MUST NOT
  depend on `eggsec`, `eggsec-daemon`, `eggsec-cli`, or `eggsec-tui`. The GUI
  reaches the engine only through the daemon's Unix socket.
- **`EnforcementContext::evaluate()` remains the mandatory pre-dispatch gate.**
  The GUI adds no dispatch path. Every operation it eventually submits goes
  through the daemon and is enforced there.
- **Strict surfaces fail closed without an explicit scope.** The GUI is a
  *manual* surface, so it does not fail closed on scope by policy — but its
  session is still scope-bounded by the engine, and the GUI MUST NOT invent a
  wider scope than the operator supplied.
- **`OperationMetadata` remains the single source of truth for operation
  policy.** The GUI renders what the daemon publishes; it never decides policy
  locally.
- **Ring-only TLS.** The GUI MUST NOT enable `tauri`'s `rustls-tls` or
  `native-tls` features. The GUI performs no outbound HTTP of its own — all
  engine traffic goes over the daemon's Unix socket — so neither feature is
  needed, and excluding them keeps `reqwest`, `rustls`, and `native-tls` out
  of the GUI's dependency closure entirely.
- **MSRV 1.89.** The pinned `tauri` line declares 1.77.2. `make check-msrv`
  MUST cover the GUI crate with its shell feature enabled.
- **`deny.toml` is a closed allow-list.** The GUI's closure MUST clear it. No
  new exception is expected; if one proves necessary it goes in `deny.toml` and
  `docs/DEPENDENCY_EXCEPTIONS.md` with an owner and a review-by date.
- **Tokio baseline.** `default-features = false` with per-crate feature lists.
  The GUI crate needs `rt`, `sync`, `time`, and `macros` only.
- **Bounded concurrency.** The GUI's connection to the daemon is one socket
  with one reader task. No spawn-per-item fan-out.
- **Never `let _ =` or `filter_map(|e| e.ok())`** — log via `tracing`.
- **Spawned tokio tasks carry a 30–300s timeout wrapper.** The daemon-client
  reader task is the one documented exception: it is a long-lived connection
  reader whose lifetime is the socket's, and killing it would drop the
  connection. Any *new* task the GUI spawns MUST carry a timeout wrapper.
- **The `make check` contract.** The GUI crate must compile in the
  `--no-default-features` profile that `make check` already runs.
- **`ADR-0009`'s three conditions** — version pin, dependency admission, Linux
  system-dependency declaration — MUST all hold when this milestone closes.
- **The shell framework appears only inside `crates/eggsec-gui`.** No `tauri`
  type, feature, or dependency edge may appear in the engine, the daemon
  protocol, the runtime DTOs, the schema publication mechanism, or the shared
  frontend contract.

## 5. Scope

### In scope

- A new workspace member `crates/eggsec-gui` — a binary crate, not a library.
- The Tauri v2 shell: window, static frontend assets, Tauri IPC commands, CSP,
  and the capability file.
- Widening `DaemonRuntimeClient` so a caller supplies its own client identity
  and request-id prefix.
- Adding `ClientKind::Gui` to the daemon protocol.
- Adding `DesktopGuiManual` to the daemon's manual-approval arm.
- A public `describe_operations` on `DaemonRuntimeClient`.
- The daemon connection, session creation with `RuntimeSurface::DesktopGuiManual`,
  and the operation list rendered from the daemon's response.
- Four new architecture guards.
- `Makefile`, CI, and documentation updates.

### Explicitly out of scope

- **Operation submission.** No form, no `submit`, no `RunRequest`. Milestone
  005.
- **Results rendering.** No `RENDERER_REGISTRY` consumer. Milestone 005.
- **Report export.** Milestone 006.
- **Daemon lifecycle management.** The GUI does **not** spawn, supervise, or
  stop a daemon. It connects to one that is already running, and reports
  precisely how to start one when the socket is unreachable. Spawning the
  `eggsec-daemon` binary is a separate decision with its own ownership — see
  §16.
- **A JavaScript build step.** The M4 frontend is hand-written HTML, CSS, and
  vanilla JavaScript checked into the repository, with **zero** npm
  dependencies. Any bundler or framework is deferred to milestone 005 and must
  be justified there, not assumed here.
- **Binary distribution, signing, notarization, and installers.** Handled
  separately, per maintainer direction.
- **Any change to the engine, the daemon protocol's wire semantics, the schema
  publication mechanism, or the `RuntimeClient` trait's method set.** The
  trait's eight methods are unchanged; the GUI uses the concrete
  `DaemonRuntimeClient` because it is daemon-only.
- **`ApprovePolicy` wiring.** This milestone aligns the RBAC matrix with the
  surface classification. It does not implement daemon-mediated interactive
  approval.

## 6. Required production changes

### Core/domain

None. The GUI holds no domain model of its own; it renders what the daemon
publishes and sends what the operator types.

### Storage and migrations

None. The GUI owns no authoritative storage. Session and task state lives in
the daemon; the GUI holds only in-memory view state.

### Protocol and DTOs

- `crates/eggsec-daemon-protocol/src/client_registry.rs` — add
  `ClientKind::Gui`. Additive. The wire representation is the variant's serde
  name; a daemon that does not know the variant fails to deserialize the
  `DeclareClient` command, which is a **fail-closed** outcome, not a silent
  misclassification.
- `crates/eggsec-daemon-protocol/src/client_registry.rs:186` — add
  `RuntimeSurface::DesktopGuiManual` to the manual arm of
  `CommandPermission::Approver`, so the RBAC matrix agrees with
  `eggsec-policy`'s classification. No approval is granted today either way,
  because `ApprovePolicy` answers `ErrorCode::Unsupported`.
- `crates/eggsec-daemon-protocol/src/protocol.rs` — **no** change. The
  protocol version stays at `2`. `ClientKind::Gui` is an additive variant
  behind a fail-closed deserialization boundary, the same class of change as
  milestone 001's `DaemonCapabilities::operation_schema`, which added a field
  with `#[serde(default)]` without a version bump.

### Runtime and concurrency

- `crates/eggsec-frontend-client/src/daemon.rs` — replace the hardcoded
  identity with a caller-supplied one:

  ```rust
  /// How a frontend identifies itself to the daemon.
  #[derive(Debug, Clone)]
  pub struct ClientIdentity {
      /// The kind recorded in the daemon's client registry.
      pub kind: ClientKind,
      /// Human-readable label shown in daemon-side listings.
      pub label: String,
      /// Prefix for this client's request ids.
      pub request_prefix: String,
  }
  ```

  `DaemonRuntimeClient::connect` takes it as a second argument and uses it for
  both the `DeclareClient` command and `next_request_id`. This is a breaking
  change to an internal 0.1.0 crate with exactly one in-tree caller.
- `crates/eggsec-frontend-client/src/daemon.rs` — add a public
  `describe_operations` returning `DescribeOperationsResponse`, reusing the
  existing pending-response routing and the existing 30s request timeout.
- `crates/eggsec-tui/src/app/mod.rs:390` — the single in-tree caller, updated
  to pass the TUI's identity (`ClientKind::Tui`, label `"eggsec-tui"`, prefix
  `"tui-"`). The TUI's behavior is unchanged.

### Frontend or operator surface

- `crates/eggsec-gui/` — new crate:

  ```
  crates/eggsec-gui/
  ├── Cargo.toml
  ├── build.rs
  ├── tauri.conf.json
  ├── capabilities/default.json
  ├── frontend/
  │   ├── index.html
  │   ├── styles.css
  │   └── app.js
  └── src/
      ├── main.rs
      ├── config.rs
      ├── connection.rs
      └── commands.rs
  ```

- `crates/eggsec-gui/src/config.rs` — parses `--socket <PATH>` (default
  `/tmp/eggsec-daemon.sock`, matching the three existing defaults) and, in a
  later milestone, `--session <ID>`. No environment variable is introduced in
  this milestone: the workspace has no socket-path env var today, and adding
  one here would create a second, differently-named source of truth. If the
  maintainers want one, it belongs in a corrective plan that also collapses
  the three duplicated defaults.
- `crates/eggsec-gui/src/connection.rs` — connects to the daemon, verifies the
  handshake's protocol version, creates the GUI session, and fetches the
  operation catalog.
- `crates/eggsec-gui/src/commands.rs` — the Tauri IPC command surface. This
  is the **only** path from the webview to the engine, and it must be a thin
  pass-through to `DaemonRuntimeClient`.
- `crates/eggsec-gui/tauri.conf.json` — window definition, `frontendDist`
  pointing at `frontend/`, and the CSP (see §7).
- `crates/eggsec-gui/frontend/` — the static frontend. No build step, no npm
  dependency, no framework.
- `crates/eggsec-gui/src/main.rs` — the binary entry point.

### Security and authorization

- The GUI's Tauri IPC commands MUST NOT expose a generic "send any command to
  the daemon" escape hatch. Each command maps to one narrow operation.
- The webview MUST NOT be able to reach the network. The GUI's CSP restricts
  `connect-src` to the Tauri IPC origins, and no `http` plugin is enabled.
- The GUI MUST NOT enable `shell`, `fs`, `process`, `updater`, or `http`
  plugins.
- The GUI MUST NOT enable `tauri`'s `rustls-tls` or `native-tls` features.
- The Tauri capability file MUST grant only the app's own commands.
- The GUI MUST verify the daemon's `protocol_version` against
  `DAEMON_PROTOCOL_VERSION` and refuse to operate on a mismatch, reporting it
  as a version incompatibility rather than a generic I/O error.
- The GUI's session MUST be created with `RuntimeSurface::DesktopGuiManual` so
  the daemon's audit trail records the GUI's identity rather than `"daemon"`.

### Documentation and static guards

- Four new guards in `scripts/check-architecture-guards.sh`, following the
  conventions of guards 151–154:
  - **155 — the GUI crate links no engine.** `crates/eggsec-gui/Cargo.toml`
    MUST NOT name `eggsec`, `eggsec-daemon`, `eggsec-cli`, or `eggsec-tui` as
    a dependency, and no `eggsec-gui` source file may import them.
  - **156 — the shell framework stays inside the GUI crate.** No crate other
    than `crates/eggsec-gui` may declare a `tauri` dependency, and the
    workspace's other crates' manifests MUST NOT mention `tauri`.
  - **157 — the GUI declares no second TLS stack.** The GUI's manifest MUST
    NOT enable `rustls-tls` or `native-tls`, and MUST NOT depend on `reqwest`,
    `rustls`, or `native-tls` directly.
  - **158 — the GUI's CSP forbids remote origins.**
    `crates/eggsec-gui/tauri.conf.json`'s `app.security.csp` MUST NOT contain
    a wildcard source and MUST NOT include `*` in `connect-src`,
    `script-src`, or `default-src`.
- `architecture/overview.md` — crate count 24 → 25, the new crate row, the
  dependency map, and the guardrail list.
- `architecture/desktop_gui.md` — **new**: the GUI's architecture — the Tauri
  shell, the daemon connection, the IPC command surface, the CSP and
  capability model, the client identity, and the platform prerequisites.
- `architecture/frontend_client.md` — the `ClientIdentity` seam and the
  `ClientKind::Gui` addition.
- `architecture/daemon.md` — the GUI client kind and the manual-approval arm.
- `architecture/tui.md` — the TUI's call-site update.
- `docs/ARCHITECTURE.md`, `docs/ARCHITECTURE_INVARIANTS.md`,
  `docs/CI_ARCHITECTURE_GUARDS.md`, `docs/VERIFICATION.md`,
  `docs/ENFORCEMENT_MODES.md` — the GUI crate, the new guards, and the
  platform prerequisites.
- `architecture/overview.md` and the `eggsec-frontend-client` skill — the
  renamed constructor seam.
- A new skill `.opencode/skills/eggsec-gui/SKILL.md`. A new subsystem means a
  new skill; the missing `eggsec-knowledge-corpus` skill is the precedent for
  that gap.

## 7. Ordered work packages

### Work package A — Widen the frontend-client identity seam

The GUI cannot identify itself until the frontend client stops hardcoding the
TUI's identity. This is the first package because every later package depends
on it.

Required changes:

1. Add `ClientIdentity { kind, label, request_prefix }` to
   `crates/eggsec-frontend-client/src/lib.rs` and re-export it.
2. Change `DaemonRuntimeClient::connect(socket_path: &str)` to
   `connect(socket_path: &str, identity: ClientIdentity)`. Store the prefix on
   the client and use it in `next_request_id` instead of the literal
   `"tui-"`.
3. Use `identity.kind` and `identity.label` in the `DeclareClient` command.
4. Update the one in-tree caller, `crates/eggsec-tui/src/app/mod.rs:390`, to
   pass the TUI's identity. The TUI's observable behavior is unchanged: same
   kind, same label, same request-id prefix.
5. Add a `describe_operations` method returning
   `Result<DescribeOperationsResponse, DaemonClientError>`, reusing the
   existing send path and its 30s timeout.

Acceptance: the frontend client contains no occurrence of the string `"tui-"`,
`ClientKind::Tui`, or `"eggsec-tui"` outside its tests.

### Work package B — Add the GUI client kind and align the approval matrix

Required changes:

1. Add `ClientKind::Gui` to
   `crates/eggsec-daemon-protocol/src/client_registry.rs`. The variant is
   additive; no existing match is exhaustive over `ClientKind`.
2. Add `RuntimeSurface::DesktopGuiManual` to the manual arm at
   `crates/eggsec-daemon-protocol/src/client_registry.rs:186`.
3. Extend the daemon's `client_kind_stored_in_registry` test
   (`crates/eggsec-daemon/src/host.rs:2421`) to cover `ClientKind::Gui`.
4. Add a test that a `DesktopGuiManual` session accepts an `Approver`-role
   approval from a controller, matching `CliManual` and `TuiManual`.

Acceptance: the RBAC matrix and `eggsec-policy`'s surface classification agree
about which surfaces are manual.

### Work package C — Create the GUI crate and its Tauri shell

Required changes:

1. Add `crates/eggsec-gui` to the workspace members and to the root
   `Cargo.toml`'s `[workspace.dependencies]`.
2. The manifest MUST declare:

   ```toml
   [features]
   default = ["shell"]
   shell = ["dep:tauri", "dep:tauri-runtime-wry"]

   [dependencies]
   eggsec-frontend-client = { path = "../eggsec-frontend-client", version = "0.1.0" }
   eggsec-runtime = { path = "../eggsec-runtime", version = "0.1.0" }
   eggsec-daemon-protocol = { path = "../eggsec-daemon-protocol", version = "0.1.0" }
   tauri = { workspace = true, optional = true }
   tauri-runtime-wry = { workspace = true, optional = true }
   serde = { workspace = true }
   serde_json = { workspace = true }
   tokio = { workspace = true, features = ["rt", "sync", "time", "macros"] }
   tracing = { workspace = true }

   [build-dependencies]
   tauri-build = { workspace = true }
   ```

   The workspace entries pin the versions verified in §3:
   `tauri = "=2.11.6"`, `tauri-build = "=2.6.3"`,
   `tauri-runtime-wry = "=2.11.4"`. The **exact** `=` pin is required — a caret
   requirement would float onto the 2.12 line, whose MSRV is 1.90.
3. `tauri`'s features MUST be `default-features = false` plus the minimum
   needed for a window. `rustls-tls` and `native-tls` MUST NOT be enabled.
   Whether `wry` is taken directly or via `tauri`'s own feature is an
   implementation detail; what matters is that the closure contains no second
   TLS stack.
4. `crates/eggsec-gui/build.rs` calls `tauri_build::build()`.
5. `crates/eggsec-gui/tauri.conf.json` defines the window, `frontendDist:
   "../frontend"`, and the CSP:

   ```
   default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self' data:; connect-src 'self' ipc: http://ipc.localhost; object-src 'none'; frame-src 'none'; base-uri 'none'
   ```

   No wildcard. No remote origin.
6. `crates/eggsec-gui/capabilities/default.json` grants only the app's own
   commands. No plugin permissions.
7. `crates/eggsec-gui/frontend/` holds `index.html`, `styles.css`, and
   `app.js` — hand-written, no framework, no npm dependency, no build step.
8. `crates/eggsec-gui/src/main.rs` — the entry point. Under
   `--no-default-features` (the `shell` feature off) it MUST still compile:
   the `shell`-gated code is behind `#[cfg(feature = "shell")]`, and the
   ungated binary prints a diagnostic that the GUI shell was not built in this
   profile and exits non-zero. This is what keeps `make check`'s
   `--no-default-features` workspace line green.

Acceptance: `cargo check -p eggsec-gui --no-default-features` and
`cargo check -p eggsec-gui --features shell` both succeed; the GUI crate's
resolved closure contains no `rustls`, no `native-tls`, and no direct
`reqwest`.

### Work package D — Wire the daemon connection and the operation list

Required changes:

1. `crates/eggsec-gui/src/config.rs` — parse `--socket <PATH>`, defaulting to
   `/tmp/eggsec-daemon.sock`.
2. `crates/eggsec-gui/src/connection.rs`:
   - Connect with `DaemonRuntimeClient::connect(&socket, ClientIdentity {
     kind: ClientKind::Gui, label: "eggsec-gui".to_owned(),
     request_prefix: "gui-".to_owned() })`.
   - Verify the handshake's `protocol_version` against
     `eggsec_daemon_protocol::protocol::DAEMON_PROTOCOL_VERSION`. On mismatch,
     report a version incompatibility and refuse to continue.
   - Create a session with
     `RuntimeSurface::DesktopGuiManual` and a `SessionScope` the GUI
     constructs itself — `is_explicit: false`, `source: "gui-default"`,
     `path: None`. The GUI MUST NOT widen scope beyond what the operator
     supplied.
   - Fetch the operation catalog with `describe_operations`.
3. `crates/eggsec-gui/src/commands.rs` — the IPC commands:
   - `connect_daemon(socket: String) -> Result<CatalogView, String>`
   - `list_operations() -> Result<Vec<OperationRow>, String>`

   Each is a narrow pass-through. There is no command that forwards an
   arbitrary daemon message.
4. `crates/eggsec-gui/frontend/app.js` — call the commands with Tauri's
   `invoke`, and render the operation rows: id, display name, mode, risk, and
   whether the operation is manual-exposed.
5. Error handling: a failed connection MUST render a precise diagnostic naming
   the socket path and the exact command that starts a usable daemon
   (`eggsec-daemon --full-executor --socket-path <PATH>`), because
   `eggsec daemon start` and a default-built `eggsec-daemon` both host a
   no-op executor that rejects every task.

Acceptance: with a daemon running at the configured socket, the window lists
the operations the daemon reports; the daemon's audit trail records the GUI's
session with the `desktop-gui-manual` surface label.

### Work package E — Guards, Makefile, CI, and documentation

Required changes:

1. Add guards 155–158 to `scripts/check-architecture-guards.sh`, in the style
   of guards 151–154. **Each new guard MUST be demonstrated to fail under a
   deliberate violation and to pass again after the violation is restored** —
   the same standard guards 151–154 met.
2. `Makefile`:
   - `make check` — the existing `cargo check --workspace --no-default-features`
     line already covers the GUI crate's non-shell profile. ADD a
     `cargo check -p eggsec-gui --features shell` line, guarded by a
     prerequisite probe that SKIPs with a clear message when the Linux webview
     development packages are absent, and FAILS when they are present but the
     build breaks. This mirrors `scripts/check_platform.sh`'s skip semantics
     for system prerequisites, and CI — which installs the packages — always
     runs it for real.
   - `make check-msrv` — ADD
     `cargo +1.89 check -p eggsec-gui --features shell`, so the GUI crate's
     real dependency closure is MSRV-verified rather than skipped by the
     `--no-default-features` workspace line. The roadmap's exit condition —
     "`make check-msrv` is green including the GUI crate" — is satisfied only
     by this line.
   - `make clippy` — ADD `cargo clippy -p eggsec-gui --features shell -- -D
     warnings`, under the same prerequisite gate.
   - Add a `make check-gui` convenience target that runs the shell-profile
     check and clippy without gating.
3. CI — `.github/workflows/ci.yml` and `.github/workflows/deep-checks.yml`:
   install the Linux prerequisites on the `ubuntu-latest` jobs:
   `libwebkit2gtk-4.1-dev`, `libgtk-3-dev`, `libayatana-appindicator3-dev`,
   `librsvg2-dev`, and `patchelf`. `pkg-config` is already present via
   `deep-checks.yml`; add it to `ci.yml` if it is not.
4. Documentation — the list in §6.
5. The `eggsec-frontend-client` and `eggsec-daemon` skills, plus the new
   `eggsec-gui` skill.

Acceptance: `make check` is green; the guards pass; the documentation and
skills describe the GUI crate as it actually is.

## 8. Failure, cancellation, restart, and contention semantics

- **Daemon unreachable.** The connection attempt fails with an I/O error. The
  GUI MUST NOT retry in a tight loop. It surfaces the socket path and the
  command that starts a usable daemon, and the operator retries deliberately.
- **Daemon dies mid-session.** The reader task ends; in-flight requests time out
  after the existing 30s request timeout. The GUI shows the connection as lost
  and offers an explicit reconnect. Reconnect creates a **new** session; it
  MUST NOT silently reuse a session id whose daemon is gone.
- **Protocol version mismatch.** The GUI refuses to operate and says so
  explicitly. It does not attempt a partial handshake.
- **Daemon declares no operation schema.** `DaemonCapabilities::operation_schema`
  is false — a daemon too old to publish the catalog. The GUI reports that the
  connected daemon predates the schema publication and stops at the
  connection step rather than rendering an empty catalog.
- **No-op executor daemon.** The daemon connects and answers `DescribeOperations`
  perfectly, but rejects every submitted task. The GUI's M4 scope submits
  nothing, so this is not reachable yet; the M5 plan MUST surface the
  no-op-executor condition rather than presenting a dead-end submit button.
- **Cancellation.** Not applicable in M4: there is nothing to cancel. The
  connection task and any future long-running command MUST carry a
  30–300s timeout wrapper.
- **Contention.** One GUI process owns one daemon connection and one session.
  Two GUI processes are two independent clients and two sessions; the daemon
  arbitrates through the existing client registry. No GUI-side locking exists
  or is needed.
- **Shutdown.** Closing the window ends the process. The daemon session is
  closed explicitly on a clean shutdown where the connection is still live, so
  the daemon does not retain an orphan session.

## 9. Compatibility and migration

- `DaemonRuntimeClient::connect` gains a parameter. This is a breaking change
  to `eggsec-frontend-client`, an internal 0.1.0 crate with exactly one
  in-tree caller, updated in the same work package. The crate is not
  published, so there is no external consumer.
- `ClientKind::Gui` is additive on the wire. A daemon that predates the
  variant fails to deserialize `DeclareClient` and rejects the GUI — a
  fail-closed outcome. The GUI's protocol-version check catches this first and
  reports it as a version incompatibility.
- `ClientCommand` and `ServerMessage` variant sets are unchanged.
  `DAEMON_PROTOCOL_VERSION` stays at `2`.
- `RuntimeClient`'s eight methods are unchanged. The TUI's behavior is
  unchanged.
- The workspace grows from 24 to 25 crates. `architecture/overview.md`,
  `plans/000-long-term-specification.md`, and `plans/registry.md` carry that
  count and MUST be updated.
- The Python bindings are untouched: `crates/eggsec-python` does not depend on
  the GUI crate, and `ClientKind` is not part of the Python API surface.

## 10. Required tests

### Focused unit tests

- `crates/eggsec-frontend-client` — the request-id prefix is the caller's, not
  the literal `"tui-"`; `connect` uses the supplied kind and label.
- `crates/eggsec-daemon-protocol` — `ClientKind::Gui` round-trips through
  serde; the `Approver` arm treats a `DesktopGuiManual` session as manual.
- `crates/eggsec-gui` — argument parsing (`--socket`, its default);
  `ClientIdentity` construction; the `SessionScope` the GUI builds.

### Integration tests

- The GUI's daemon-connection path, exercised against an in-memory daemon
  host the way `crates/eggsec-daemon`'s own tests do: connect, declare as
  `ClientKind::Gui`, create a `DesktopGuiManual` session, fetch the catalog.
- The daemon's audit trail records `desktop-gui-manual` for a GUI session's
  scoped events — the same assertion milestone 003 established for the
  surface label, now exercised through the GUI's own client kind.

### Restart and recovery tests

- Connecting to a socket path with no daemon produces the diagnostic that
  names the socket and the start command — not a panic and not a generic
  error.
- A daemon that restarts between the handshake and the catalog fetch is
  reported as a connection loss.

### Contention and cancellation tests

- Not applicable to M4 beyond the single-connection case.

### Security and negative tests

- Guard 155: adding an `eggsec` dependency to the GUI's manifest fails the
  guard.
- Guard 156: adding a `tauri` dependency to any other crate fails the guard.
- Guard 157: enabling `rustls-tls` or `native-tls` on `tauri` fails the
  guard.
- Guard 158: a CSP containing `*` in `connect-src`, `script-src`, or
  `default-src` fails the guard.
- The GUI's IPC command surface exposes no generic "send any command" entry
  point — asserted by inspection of `commands.rs`, and by a test that the
  command list is exactly the declared set.

### Migration and compatibility tests

- The TUI's daemon mode still works end to end after the `connect` signature
  change: same kind, same label, same request-id prefix, same session
  surface.
- A `DescribeOperations` request from a client that has not declared is still
  refused with `ErrorCode::ClientNotDeclared`.

## 11. Required verification commands

```bash
# Format and lint
cargo fmt --all --check
cargo clippy -p eggsec-gui --features shell -- -D warnings
cargo clippy -p eggsec-frontend-client -p eggsec-daemon-protocol -p eggsec-daemon -- -D warnings
cargo clippy -p eggsec-tui -- -D warnings

# Build matrix
cargo check -p eggsec-gui --no-default-features
cargo check -p eggsec-gui --features shell --all-targets
cargo check --workspace --no-default-features
cargo check --workspace --all-targets

# Contract
make check-msrv
make check-deps
bash scripts/check-architecture-guards.sh

# Tests for the crates this milestone touches
cargo test -p eggsec-frontend-client --lib
cargo test -p eggsec-daemon-protocol --lib
cargo test -p eggsec-daemon --lib
cargo test -p eggsec-tui --lib
cargo test -p eggsec-gui --lib --features shell

# Documentation references
python3 scripts/check_doc_references.py
```

`make check-deps` MUST pass `bans`, `licenses`, and `sources`. The `advisories`
check is **known to fail at the baseline** on the pre-existing `rustls 0.21.12`
family via `tiberius` → `sqlx-mysql`; that backlog is a separate maintainer
decision and is not a gate for this milestone. The GUI crate's own closure MUST
introduce no new advisory.

## 12. Documentation updates

- `architecture/overview.md` — crate count 24 → 25; the `eggsec-gui` row in the
  crate table, the module table, and the key-type table; the dependency map;
  the guardrail list (guards 155–158); the footer.
- `architecture/frontend_client.md` — the `ClientIdentity` seam, the widened
  `connect`, and `describe_operations`.
- `architecture/daemon.md` — `ClientKind::Gui`, the manual-approval arm, and
  the GUI's audit-trail surface label.
- `architecture/tui.md` — the call-site update; the TUI's observable behavior
  is unchanged.
- `architecture/config.md` — no change is required; the GUI introduces no
  configuration surface in this milestone. If the implementation adds one, this
  document must be updated with it.
- `docs/ARCHITECTURE.md` — the new leaf crate in the dependency picture.
- `docs/ARCHITECTURE_INVARIANTS.md` — the GUI's daemon-only boundary.
- `docs/CI_ARCHITECTURE_GUARDS.md` — guards 155–158.
- `docs/VERIFICATION.md` — the new `make check` and `make check-msrv` lines,
  the prerequisite-skip semantics, and the `make check-gui` target.
- `docs/ENFORCEMENT_MODES.md` — the GUI's manual classification, cross-checked
  against the daemon's manual-approval arm.
- `docs/COMMAND_REGISTRY.md` — the GUI is not a CLI command; no entry is
  required. Verify and leave unchanged.
- `CONTRIBUTING.md` — the GUI crate's prerequisites, if the crate's build
  instructions belong there.
- `.opencode/skills/eggsec-gui/SKILL.md` — new.
- `.opencode/skills/eggsec-frontend-client/SKILL.md` — the `ClientIdentity`
  seam.
- `.opencode/skills/eggsec-daemon/SKILL.md` — `ClientKind::Gui`.
- `plans/000-long-term-specification.md` — the crate count.

## 13. Acceptance criteria

1. `crates/eggsec-gui` exists as a workspace member, is a binary crate, and
   does not depend on the engine, the daemon crate, the CLI, or the TUI.
2. The GUI launches a window and renders the operation catalog published by a
   live daemon over the shared frontend client.
3. The GUI's daemon session is created with `RuntimeSurface::DesktopGuiManual`,
   and the daemon's audit trail records that surface label for its scoped
   events.
4. The `tauri` dependency is pinned to the 2.11.x line with an exact `=`
   requirement, and `make check-msrv` is green **including the GUI crate with
   its shell feature enabled**.
5. The GUI's resolved dependency closure contains no `rustls`, no
   `native-tls`, and no direct `reqwest`; `cargo deny check licenses` passes
   under the existing `deny.toml` allow-list with no new exception.
6. The GUI's CSP contains no wildcard and permits no remote origin; no
   `http`, `shell`, `fs`, `process`, or `updater` plugin is enabled; the
   capability file grants only the app's own commands.
7. The GUI's IPC surface is a set of narrow pass-throughs with no generic
   command-forwarding entry point.
8. The GUI does not spawn, supervise, or stop a daemon.
9. Guards 155–158 exist, pass, and each was demonstrated to fail under a
   deliberate violation.
10. `make check` is green, and `make check-msrv` covers the GUI crate's real
    dependency closure.
11. The frontend client contains no TUI-specific identity or request-id
    prefix; the TUI's daemon mode still works after the signature change.
12. The RBAC matrix's manual-approval arm includes `DesktopGuiManual`, with a
    test.
13. Every document and skill listed in §12 is updated.
14. `python3 scripts/check_doc_references.py` reports no failure on any line
    this milestone changed. The 29 failures that exist at the baseline are
    pre-existing and are not this milestone's to fix.

## 14. Stop conditions

Stop and escalate rather than improvise if any of these arise:

- **The GUI's dependency closure cannot clear `deny.toml`** and the only way
  forward is a new license exception without an owner and a review-by date.
  `ADR-0009` names the egui fallback for exactly this case.
- **A second TLS stack is unavoidable.** `tauri`'s `rustls-tls` pulls `rustls`
  with default features, which the workspace forbids. Do not enable it to work
  around a problem; the GUI does not need outbound HTTP.
- **The Tauri dependency cannot be made to build at MSRV 1.89** within the
  2.11.x line. Re-evaluate the egui fallback instead of raising the MSRV —
  raising it requires its own ADR.
- **`ClientKind::Gui` proves to require a protocol version bump.** That is a
  cross-cutting change affecting every daemon client. Escalate; do not bump
  the version unilaterally.
- **The GUI appears to need engine-side access** for any reason. That is a
  boundary violation of `ADR-0007`, not an implementation difficulty.
- **The Linux webview prerequisites cannot be installed in CI.** Then the
  shell-profile check cannot be enforced per-PR, and the `make check-msrv`
  exit condition cannot be met literally. Escalate rather than silently
  excluding the GUI crate from the MSRV gate.
- **`ADR-0009`'s three conditions cannot all be met.** The fallback is
  sanctioned in advance; take it rather than weakening an invariant.

## 15. Closure evidence required

A closure record at `plans/closure/desktop-gui-frontend/004-gui-shell-skeleton-closure.md`
following the twelve-section template in `plans/closure/README.md`, containing:

1. The requirement-to-evidence matrix, with a file:line citation or a command
   output for every acceptance criterion in §13.
2. The production implementation evidence: the new crate's file inventory, the
   `Cargo.toml` dependency block, the `tauri.conf.json`, the CSP, and the
   capability file.
3. The verification actually run, with real output — not a description of what
   would be run.
4. The invariant review, addressing each invariant in §4 by name.
5. The failure and recovery review, addressing §8.
6. The migration and compatibility review, addressing §9.
7. The security review, addressing the §6 security requirements, including the
   dependency-closure evidence that no second TLS stack was introduced.
8. The documentation and operations section, listing every document and skill
   changed.
9. The unresolved findings table — every finding that remains open, with an
   owner and a next action, including any that were known at the baseline.
10. The roadmap disposition and the registry updates.

Additional milestone-specific evidence:

- The `cargo deny` output showing `bans ok`, `licenses ok`, and `sources ok`.
- The `cargo tree` output for the GUI crate showing no `rustls`,
  `native-tls`, or direct `reqwest`.
- The guard-failure demonstrations for guards 155–158, showing the guard
  failing under a deliberate violation and passing after restore.
- The CI diff showing the Linux webview packages installed.
- A screenshot or a transcript of the window listing the operation catalog
  against a live daemon.

## 16. Handoff notes

- **Milestone 005 consumes this milestone's two published artifacts**: the
  per-variant `request_schema` in `TaskKindDescription`, and the operation
  metadata in `OperationDescription`. The measured shape is in §3 — no `$ref`,
  no `enum`, five control kinds — and milestone 005's form engine is designed
  against it.
- **`describe_operations` is an inherent method on `DaemonRuntimeClient`, not
  a `RuntimeClient` trait method.** The GUI is daemon-only by `ADR-0007`, so
  the trait's embedded/daemon abstraction buys the GUI nothing, and the
  embedded client genuinely has no catalog to publish. If a second
  daemon-only frontend appears, promote it to a narrow
  `OperationCatalogClient` trait then.
- **The GUI holds the concrete `DaemonRuntimeClient`, not
  `Arc<dyn RuntimeClient>`.** The TUI needs the trait because it chooses
  between an embedded runtime and a daemon; the GUI has no such choice. The
  shared contract is still `eggsec-frontend-client` — the roadmap's
  "connects to a daemon over the shared client" is satisfied by using that
  crate's daemon implementation.
- **Daemon lifecycle management is unowned.** The GUI requires a running
  `eggsec-daemon --full-executor`. Spawning it as a child process is
  possible — the GUI would link nothing by doing so — but it raises build-
  feature questions (`full-executor` is not a default feature of the daemon
  crate) and a child-process supervision burden that the skeleton milestone
  does not need. **This milestone recommends adding it to milestone 006's
  deliverable boundary**, and the roadmap MUST be amended to record that
  ownership when this milestone closes.
- **The socket-path default is duplicated in three places** and this milestone
  adds a fourth (the GUI's own default). It MUST match
  `/tmp/eggsec-daemon.sock`. Collapsing the duplication — and introducing an
  environment variable — belongs in a corrective plan, not here.
- **The Tauri frontend is deliberately buildless.** Zero npm dependencies
  means zero JavaScript supply-chain surface in this milestone. Milestone 005
  may introduce a build step; if it does, it owes the repository a recorded
  JavaScript dependency-audit policy, which `ADR-0009` explicitly deferred to
  the shell milestone.
- **`ADR-0007` is load-bearing and unforgiving.** The GUI's engine access is
  the daemon socket and nothing else. Every apparent need to reach into the
  engine is a sign that a command should be added to the daemon protocol
  instead.
