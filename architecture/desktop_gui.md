# Desktop GUI (Tauri Shell)

The Eggsec desktop GUI is a **daemon-only** frontend: it reaches the engine
exclusively through the daemon's Unix socket, using the shared
`eggsec-frontend-client` contract. It links no engine code, spawns no
subprocess, and performs no outbound HTTP.

Its shell is [Tauri v2](https://tauri.app), pinned to the 2.11.x line
(`ADR-0009`). The frontend assets are hand-written HTML, CSS and vanilla
JavaScript with **no build step and no npm dependency**.

Decisions: `plans/adrs/ADR-0007-desktop-gui-frontend-boundary.md` (the
daemon-only boundary), `plans/adrs/ADR-0009-gui-shell-technology-selection.md`
(the shell technology).

## Role & Responsibilities

| Responsibility | Owned by | Notes |
|----------------|----------|-------|
| The desktop window | `crates/eggsec-gui/src/shell.rs` | Tauri builder, managed state, command registration. |
| The IPC surface | `crates/eggsec-gui/src/commands.rs` | Eight narrow Tauri commands and one event channel. |
| The daemon connection | `crates/eggsec-gui/src/connection.rs` | Client identity, protocol check, session, catalog fetch. |
| The form engine | `crates/eggsec-gui/src/form.rs` | Schema property → control; control values → typed `RunRequest`. |
| Task state and result views | `crates/eggsec-gui/src/tasks.rs` | Bounded per-task retention, status, result rendering. |
| Command-line parsing | `crates/eggsec-gui/src/config.rs` | `--socket <PATH>`. |
| Engine execution | *nobody here* | The daemon's executor owns it; the GUI only submits requests. |

The crate ships a `src/lib.rs` library (for testability) and a thin
`src/main.rs` binary that calls `eggsec_gui::run()`
(`crates/eggsec-gui/src/lib.rs:60-64`).

## Location & Feature Gating

| Crate | Path | Feature | Notes |
|-------|------|---------|-------|
| `eggsec-gui` | `crates/eggsec-gui/` | `shell` (default) | The desktop GUI. Without `shell` the binary reports `GuiError::ShellNotBuilt` and exits non-zero (`crates/eggsec-gui/src/lib.rs:66-70`) rather than pretending to work. |

The crate is a workspace member. Its dependencies are exactly
`eggsec-frontend-client`, `eggsec-runtime`, `eggsec-daemon-protocol` and
`eggsec-ui-model` (`crates/eggsec-gui/Cargo.toml`).

### Boundary rule

The GUI **must not** depend on `eggsec` (the engine), `eggsec-daemon`,
`eggsec-cli` or `eggsec-tui` (`ADR-0007`). `eggsec-frontend-client`,
`eggsec-runtime`, `eggsec-daemon-protocol` and `eggsec-ui-model` are the only
workspace dependencies it may name, and the Tauri dependency may not appear in
any other crate. Both rules are enforced by the architecture guards (Checks
155 and 156 — see `docs/CI_ARCHITECTURE_GUARDS.md`).

The consequence is that the GUI **cannot** enforce policy, resolve scope, or
dispatch an operation. It assembles a request and hands it to the daemon, which
does all three. The GUI has no `EnforcementContext`, no `Scope`, and no
`ApprovedExecution`.

The GUI performs **no outbound HTTP**. All engine traffic travels over the
daemon's Unix socket. The crate therefore enables neither `tauri`'s
`rustls-tls` nor `native-tls` feature, and depends on neither `reqwest` nor
`rustls` nor `native-tls`: those crates stay out of the GUI's dependency
closure entirely, so the workspace's ring-only TLS invariant holds with no
exception (Check 157).

## The Tauri Shell

`crates/eggsec-gui/src/shell.rs` is the only place the shell framework is
wired up, and the module doc says so in as many words: the framework "appears
only in this module and in `commands.rs`" (`crates/eggsec-gui/src/shell.rs:3-4`).

The builder registers the managed state and the eight IPC commands, then runs
the generated context (`crates/eggsec-gui/src/shell.rs:20-33`).

### Window and configuration

`crates/eggsec-gui/tauri.conf.json` declares one window: label `main`, title
`Eggsec`, 1200×800 with a 800×600 minimum, resizable
(`crates/eggsec-gui/tauri.conf.json:11-21`). `app.bundle.active` is `false`
(`crates/eggsec-gui/tauri.conf.json:26-28`) — the GUI is run from the build
tree, not installed as a bundle.

### Capability model

`crates/eggsec-gui/capabilities/default.json` grants `core:default` and
nothing else: `"permissions": ["core:default"]`, scoped to the `main` window.
**No plugin is enabled** — no http, no shell, no fs, no process, no updater.
Every engine request travels over the daemon's Unix socket, so the GUI needs no
plugin to reach the engine, and granting one would be pure attack surface.

### Content Security Policy

```
default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self' data:;
connect-src 'self' ipc: http://ipc.localhost; object-src 'none';
frame-src 'none'; base-uri 'none'
```

(`crates/eggsec-gui/tauri.conf.json:23`). There is no wildcard source and no
remote origin. `object-src 'none'` and `frame-src 'none'` forbid plugin and
frame embedding outright; `base-uri 'none'` forbids a base-tag rewrite. The
only non-`'self'` source is the IPC scheme the webview needs to reach its own
host process. Check 158 fails the build if a wildcard or a `*` appears in
`connect-src`, `script-src` or `default-src`.

## Version Pins — and why they are exact

The Tauri dependency set is pinned with `=` requirements, not carets:

| Crate | Pin |
|-------|-----|
| `tauri` | `=2.11.6` |
| `tauri-build` | `=2.6.3` |
| `tauri-runtime` | `=2.11.3` |
| `tauri-runtime-wry` | `=2.11.4` |
| `tauri-macros` | `=2.6.3` |
| `tauri-utils` | `=2.9.3` |

(`Cargo.toml:93-98`.)

**These pins are load-bearing. Do not loosen them.**

The next lines up — Tauri 2.12, `tauri-utils` 2.10, `tauri-macros` 2.7 — all
declare **MSRV 1.90**, above this workspace's 1.89 floor. `tauri` declares its
runtime and utils dependencies with *caret* requirements, so without these
direct exact pins a plain `cargo update` floats the transitive closure onto the
1.90 line and breaks `make check-msrv`. The GUI crate depends on each of these
directly for exactly that reason: they hold the closure on the MSRV-safe line
(`Cargo.toml:86-92`).

`tauri-build` and `tauri-utils` are unconditional build-dependencies, and
`tauri-build` requires `tauri-utils ^2.9.3`. That is why `tauri-utils` is
pinned exactly even in the `--no-default-features` profile — that profile is
what `make check` and `make check-msrv` build, so an unpinned `tauri-utils`
would break the MSRV gate even though the shell is off
(`crates/eggsec-gui/Cargo.toml`).

## The Buildless Frontend

`crates/eggsec-gui/frontend/` holds hand-written HTML, CSS and vanilla
JavaScript. There is **no `package.json`, no lockfile, no bundler, and no
build step**. `tauri.conf.json` sets `app.withGlobalTauri: true`
(`crates/eggsec-gui/tauri.conf.json:10`) so the webview can reach
`window.__TAURI__.core.invoke` without a module bundler.

### JavaScript dependency policy

**Zero npm dependencies.** A remote script source, a CDN import, or any
`package.json` at all is not permitted, for two reasons:

1. The CSP has no remote origin, so a CDN import would be blocked by the
   browser anyway — the policy and the dependency rule agree.
2. A remote script in a security tool is supply-chain surface that the
   operator cannot audit from the source tree. The GUI's entire value is that
   the bytes it runs are the bytes in this repository.

`ADR-0009` allowed a JavaScript build step if it was justified. The
implementation went further and took none: the schema-driven form engine
means the frontend needs no code generation, so the toolchain buys nothing.

## The Daemon Connection

`crates/eggsec-gui/src/connection.rs` owns everything between the window and
the daemon.

### Client identity

The GUI presents itself as `ClientKind::Gui`, labelled `eggsec-gui`, with the
request-id namespace `gui-` (`crates/eggsec-gui/src/connection.rs:26`). The
`ClientIdentity` seam is documented in
[frontend_client.md](frontend_client.md); the point here is that the GUI
supplies its own identity rather than inheriting the TUI's.

### Connection sequence

`GuiConnection::connect` performs, in order:

1. **Connect** to the Unix socket at the configured path.
2. **Protocol-version check** — the daemon's advertised `protocol_version` must
   match `DAEMON_PROTOCOL_VERSION` (`crates/eggsec-gui/src/connection.rs:105`).
   A mismatch is a hard failure, not a warning.
3. **Capability check** — the daemon must advertise the `operation_schema`
   capability (`crates/eggsec-gui/src/connection.rs:118`,
   `crates/eggsec-daemon-protocol/src/protocol.rs:61`). Without it the daemon
   cannot publish the request schemas the form engine is built on, so the GUI
   refuses to connect rather than presenting empty forms.
4. **Session creation** with the `DesktopGuiManual` surface
   (`crates/eggsec-gui/src/connection.rs:129`).
5. **Catalog fetch** — `describe_operations()` yields the published task kinds
   and their schemas (`crates/eggsec-gui/src/connection.rs:140`).

`ApprovePolicy` remains unwired: the daemon answers it with
`ErrorCode::Unsupported` (`crates/eggsec-daemon/src/host.rs:829-858`). No
approval is granted either way. The GUI handles `PolicyDecisionRequired`
defensively and offers **no approval affordance** — there is no button that
could do anything.

### The no-op executor condition

The GUI requires an already-running `eggsec-daemon --full-executor`. It does
**not** spawn, supervise, or stop a daemon (`crates/eggsec-gui/src/shell.rs:12-16`).

`eggsec daemon start` and a default-built `eggsec-daemon` both host a no-op
executor that rejects every task — the CLI's start path uses a `NoopExecutor`
that returns `UnsupportedTaskKind`
(`crates/eggsec-cli/src/daemon_cli.rs:584-605`), and the daemon crate's
`full-executor` feature is opt-in (`crates/eggsec-daemon/Cargo.toml`). The GUI
therefore does two things instead of offering a submit button that cannot work:

- Its connection diagnostic names the socket path and the exact command that
  starts a usable daemon (`crates/eggsec-gui/src/connection.rs:51`).
- When the connected daemon advertises **no task kinds**, the GUI reports the
  no-op-executor condition explicitly
  (`crates/eggsec-gui/src/connection.rs:160`).

### Never retry in a tight loop

The GUI never retries the connection in a tight loop. A failed connect is
reported to the operator with the socket path and the remedy, and the operator
retries deliberately. A retry loop against a missing daemon is a spin at 100%
CPU for a condition that only an operator can fix.

## The IPC Command Surface

Exactly eight Tauri commands, each a narrow pass-through
(`crates/eggsec-gui/src/commands.rs`, registered at
`crates/eggsec-gui/src/shell.rs:22-31`):

| Command | Purpose |
|---------|---------|
| `connect_daemon(socket)` | Connect to the daemon and load the catalog. |
| `list_operations()` | The published operation catalog. |
| `list_task_kinds()` | The published task kinds. |
| `form_for(variant)` | The control set for one task kind. |
| `submit_task(variant, values)` | Assemble a `RunRequest` and submit it. |
| `cancel_task(task_id)` | Cancel one task. |
| `cancel_active()` | Cancel the task the GUI considers active. |
| `task_snapshot()` | The dashboard's view of tracked tasks. |

**There is no generic "send any command" entry point.** The webview cannot
name a `ClientCommand` variant, cannot set the runtime surface, and cannot set
the scope. The surface is fixed by the GUI (`RuntimeSurface::DesktopGuiManual`,
`crates/eggsec-gui/src/form.rs:300`), and the scope is resolved by the daemon.

### The event channel

The webview receives exactly one event on the bus: `eggsec://task-event`
(`crates/eggsec-gui/src/commands.rs:28`). Its payload is one of three shapes —
`task_upsert`, `logs`, or `counters`. The GUI subscribes **once per session**
and never spawns a task per event: one forwarder task lives for the lifetime
of the subscription, coalescing log lines and flushing on a 100 ms interval
(`crates/eggsec-gui/src/commands.rs:488-504`).

## The Form Engine

`crates/eggsec-gui/src/form.rs` is the whole point of the GUI. There is **no
per-operation code anywhere**: no `match` or `switch` over the published
task-kind variant names, no hand-written field table, no per-operation
validation. A task kind the engine has not published yet gets a form with no
GUI change.

### The control-kind mapping

One published JSON Schema property maps to exactly one control
(`crates/eggsec-gui/src/form.rs:172-209`):

| Schema shape | Control | Note |
|--------------|---------|------|
| `type: "string"` | `text` | |
| `type: "integer"` | `number` | |
| `type: "boolean"` | `checkbox` | Tri-state: unset, true, false. |
| `type: "array"` with string items | `list` | Repeatable text control. |
| `type: "object"`, array of non-strings, or an unconstrained schema | `json` | A JSON text area. |
| Any other scalar | `text` | Documented fallback; never panics, never drops the field. |

The five control kinds are `text`, `number`, `checkbox`, `list` and `json`
(`crates/eggsec-gui/src/form.rs:36-47`).

The numeric upper bound is derived from the schema's `format`, not from a
`maximum` keyword — the published schema states a `minimum` but never a
`maximum` (`crates/eggsec-gui/src/form.rs:221-234`).

### Deterministic ordering

Controls are sorted **required first, then alphabetically**
(`crates/eggsec-gui/src/form.rs:130-135`), so the form's layout does not shift
between runs.

### The three-layer validation model

This is the design to preserve.

1. **Advisory, from the schema** — the control's declared type and bounds.
   `check_value` (`crates/eggsec-gui/src/form.rs:296`) checks the supplied
   value against the schema property.
2. **Exact, from the DTOs** — the assembled request is deserialized into the
   *same* `RunRequest` the engine will deserialize
   (`crates/eggsec-gui/src/form.rs:321`). A wrong-typed value is therefore
   rejected by the engine's own rules, and the error names the field.
3. **Authoritative, in the engine** — the engine's own validation still decides
   whether the task may run.

Layers one and two save the operator a round trip. They never stand in for
layer three, and neither encodes a per-operation rule.

### The "not provided" rule

**A blank form field sends `null` — never `""`, `0`, `false` or `[]`.**

This is a safety rule, not a style preference. Four published parameters carry
non-null serde defaults, and one of them is a fail-safe flag: `dry_run`
defaults to `true` (`crates/eggsec-runtime/src/request.rs:502`). A GUI that
sent `false` for an untouched field would silently turn a simulation into a
live action.

In the Rust assembly path (`crates/eggsec-gui/src/form.rs:281-294`):

- A field that is **required** and absent is an error.
- A **nullable optional** field absent is sent as `null`.
- A field that is **not required** and whose type does not accept `null` is
  **omitted** from the request, so the request DTO applies its own default.

The published schema's measured shape is **30 task-kind variants and 149 fields
in total** (`crates/eggsec-gui/src/form.rs:484`,
`crates/eggsec-daemon-protocol/src/operations.rs:211`). The shape is flat:
every property is a scalar, a list of strings, or an unconstrained value — no
nested objects, no `$ref`/`$defs` indirection, and no `enum` constraint. The
control mapping is written against exactly those shapes, with a documented
fallback for anything else (`crates/eggsec-gui/src/form.rs:514-589`).

## The Task State

`crates/eggsec-gui/src/tasks.rs` owns the GUI's view of running work.

### Status

The GUI's own status enum is `Queued`, `Running`, `Completed`, `Failed`,
`Cancelled`, `AwaitingPolicy` and `Unknown`
(`crates/eggsec-gui/src/tasks.rs:44-59`). `AwaitingPolicy` is the GUI's
addition: the runtime reports it as a separate event rather than a status.

### Bounded retention

**Unbounded retention is a disqualifying defect, not a tuning problem.**

- Each task keeps at most **`MAX_LOG_LINES_PER_TASK` = 200** log lines in a
  capped ring (`crates/eggsec-gui/src/tasks.rs:28`, `crates/eggsec-gui/src/tasks.rs:72-76`).
  The ring counts what it drops.
- The event forwarder coalesces log lines and forwards at most
  **`MAX_LOG_LINES_PER_FLUSH` = 50** per flush
  (`crates/eggsec-gui/src/tasks.rs:35`), reporting the rest as dropped rather
  than silently discarding them.
- Dropped lines are counted per task (`dropped_log_lines`,
  `crates/eggsec-gui/src/tasks.rs:200`) and surfaced in the dashboard view
  (`crates/eggsec-gui/src/tasks.rs:553-597`).

### The event stream is the source of truth

The GUI does not poll for task state. The daemon's event stream is the source
of truth; the GUI applies events to its registry and renders. This is why the
subscription happens once per session and why the forwarder coalesces rather
than spawning: a task per event would multiply the retention problem by the
event rate.

## Result Rendering

The renderer registry in `eggsec-ui-model` is the **only** source of per-kind
metadata for a result view — the GUI imports `renderer_for_kind` from
`eggsec_ui_model::renderer_registry` (`crates/eggsec-gui/src/tasks.rs:20`) and
consults nothing else (`crates/eggsec-gui/src/tasks.rs:444-477`). A kind the
registry does not know degrades to a generic JSON rendering rather than being
hidden or dropped.

`supports_rich_tui` describes the **TUI** — it is a field on the registry's
`RendererDescriptor` (`crates/eggsec-ui-model/src/renderer_registry.rs:18`),
and the GUI never reads it. It is not a capability query for the GUI and must
not be read as one: the GUI renders every published kind, rich or generic, and
the registry's own metadata decides which.

## Platform Prerequisites and Build Profiles

The default (`shell`) profile needs the platform's webview development
packages:

- **Linux**: `libwebkit2gtk-4.1-dev`, `libgtk-3-dev`, `librsvg2-dev`,
  `patchelf`, and the `libayatana-appindicator3-dev` / `libxdo-dev` packages
  for the tray.
- **macOS / Windows**: the system webview is present; no extra packages.

The `--no-default-features` profile builds the GUI crate without the shell —
the `tauri` crate is not in the graph, `build.rs` skips `tauri_build::build()`
(the `tauri` crate's metadata does not exist in that profile, so the call would
panic — `crates/eggsec-gui/build.rs:8-10`), and `run()` returns
`GuiError::ShellNotBuilt`. That profile is what `make check` and
`make check-msrv` build, so the no-shell build stays green without the webview
SDK installed.

## Known Gaps

These are deliberate and documented, not oversights.

- **`ApprovePolicy` is unwired.** The daemon answers it with
  `ErrorCode::Unsupported` (`crates/eggsec-daemon/src/host.rs:829-858`), so no
  approval is granted either way. The GUI handles `PolicyDecisionRequired`
  defensively and offers no approval affordance.
- **The GUI does not manage the daemon's lifecycle.** It requires an
  already-running `eggsec-daemon --full-executor` and never spawns,
  supervises, or stops one.
- **The socket-path default is duplicated in four places.** The default
  `/tmp/eggsec-daemon.sock` is hardcoded independently in the CLI
  (`crates/eggsec/src/cli/mod.rs:249`), the daemon's config
  (`crates/eggsec-daemon/src/config.rs:22`), the daemon's main
  (`crates/eggsec-daemon/src/main.rs:20`), and now the GUI
  (`crates/eggsec-gui/src/config.rs:17`). The GUI's `--socket` flag overrides
  it.

## Key Invariants

1. The GUI links no engine, daemon, CLI or TUI code. It depends only on
   `eggsec-frontend-client`, `eggsec-runtime`, `eggsec-daemon-protocol` and
   `eggsec-ui-model` (Check 155).
2. The Tauri dependency lives only in `crates/eggsec-gui` (Check 156).
3. The GUI declares no TLS stack of its own: no `rustls-tls`, no `native-tls`,
   no direct `reqwest`/`rustls`/`native-tls` dependency (Check 157).
4. The CSP names no wildcard source and no remote origin (Check 158).
5. The form is generated from the published schema. There is no per-operation
   code and no hand-written operation table (Check 159).
6. Log retention is capped per task and the forwarder caps each flush.
   Unbounded retention is a disqualifying defect (Check 160).
7. The version pins are exact. Loosening them breaks `make check-msrv`.
8. The frontend has no npm dependency and no build step.
9. The published schema is descriptive, never authoritative. The engine's own
   validation decides whether a task may run.
10. A blank field is "not provided", never a zero value. The fail-safe
    defaults must survive an untouched form.
11. The GUI offers no approval affordance while `ApprovePolicy` is unwired.

## See Also

- [frontend_client.md](frontend_client.md) — the `ClientIdentity` seam and the
  daemon client the GUI speaks through
- [daemon.md](daemon.md) — the daemon the GUI connects to, its RBAC matrix and
  its audit trail
- [tui.md](tui.md) — the terminal UI, the other frontend on the same contract
- [runtime_bridge.md](runtime_bridge.md) — where `RuntimeSurface::DesktopGuiManual`
  becomes an `ExecutionSurface`
- [overview.md](overview.md) — System-wide architecture and crate counts

*Last verified against source: 2026-10-23 (milestones 004–005: Tauri shell
skeleton, schema-driven forms and task dashboard)*
