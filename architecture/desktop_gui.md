# Desktop GUI (Tauri Shell)

The Eggsec desktop GUI is a **daemon-only** frontend: it reaches the engine
exclusively through the daemon's Unix socket, using the shared
`eggsec-frontend-client` contract. It links no engine code and performs no
outbound HTTP. It starts **no process at all** unless the operator names a
daemon executable with `--daemon-bin`; when they do, the GUI owns that
process's lifetime and stops it on exit.

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
| The IPC surface | `crates/eggsec-gui/src/commands.rs` | Twelve narrow Tauri commands and one event channel. |
| The daemon connection | `crates/eggsec-gui/src/connection.rs` | Client identity, protocol check, session, catalog fetch. |
| The form engine | `crates/eggsec-gui/src/form.rs` | Schema property → control; control values → typed `RunRequest`. |
| Task state and result views | `crates/eggsec-gui/src/tasks.rs` | Bounded per-task retention, status, result rendering. |
| The coverage model | `crates/eggsec-gui/src/coverage.rs` | One derived reachability row per published task kind. |
| The availability resolver | `crates/eggsec-gui/src/availability.rs` | The denial ledger: what a live refusal taught the GUI. |
| The report path | `crates/eggsec-gui/src/export.rs` | The format map, the export rendering, the write discipline, the report view. |
| The daemon lifecycle | `crates/eggsec-gui/src/daemon.rs` | The opt-in probe / spawn / supervise / stop route. |
| Command-line parsing | `crates/eggsec-gui/src/config.rs` | `--socket <PATH>`, `--daemon-bin <PATH>`. |
| Engine execution | *nobody here* | The daemon's executor owns it; the GUI only submits requests. |

The crate ships a `src/lib.rs` library (for testability) and a thin
`src/main.rs` binary that calls `eggsec_gui::run()`
(`crates/eggsec-gui/src/lib.rs:64-68`).

## Location & Feature Gating

| Crate | Path | Feature | Notes |
|-------|------|---------|-------|
| `eggsec-gui` | `crates/eggsec-gui/` | `shell` (default) | The desktop GUI. Without `shell` the binary reports `GuiError::ShellNotBuilt` and exits non-zero (`crates/eggsec-gui/src/lib.rs:71-74`) rather than pretending to work. |

The crate is a workspace member. Its dependencies are exactly
`eggsec-frontend-client`, `eggsec-runtime`, `eggsec-daemon-protocol`,
`eggsec-ui-model` and `eggsec-output` (`crates/eggsec-gui/Cargo.toml:38-47`).

`eggsec-output` is a **leaf data crate** — `eggsec-core` + `eggsec-report-model`
only, no engine edge, no runtime, no framework
(`crates/eggsec-gui/Cargo.toml:44-47`). The GUI renders the same report
artifacts the CLI renders, through the same functions.

### Boundary rule

The GUI **must not** depend on `eggsec` (the engine), `eggsec-daemon`,
`eggsec-cli` or `eggsec-tui` (`ADR-0007`). `eggsec-frontend-client`,
`eggsec-runtime`, `eggsec-daemon-protocol`, `eggsec-ui-model` and
`eggsec-output` are the only workspace dependencies it may name, and the Tauri
dependency may not appear in any other crate. Both rules are enforced by the
architecture guards (Checks 155 and 156 — see `docs/CI_ARCHITECTURE_GUARDS.md`).

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

The GUI's one non-workspace dependency is `libc`, and it is **unix-only**
(`crates/eggsec-gui/Cargo.toml:57-63`). Tokio's `signal` module only *receives*
signals; it has no send API, so the SIGTERM the GUI delivers to the daemon it
started is made through `libc::kill` directly. The Tokio `process` and `signal`
features are the other two additions to the GUI's dependency set
(`crates/eggsec-gui/Cargo.toml:54`).

## The Tauri Shell

`crates/eggsec-gui/src/shell.rs` is the only place the shell framework is
wired up, and the module doc says so in as many words: the framework "appears
only in this module and in `commands.rs`" (`crates/eggsec-gui/src/shell.rs:3-4`).

The builder registers the managed state and the thirteen IPC commands, then runs
the generated context (`crates/eggsec-gui/src/shell.rs:27-46`).

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

The policy is **not** relaxed to accommodate a report, and it is not relaxed
for the report document specifically: `style-src 'self'` and `script-src 'self'`
forbid the inline `<style>` and `<script>` the engine's HTML carries, and
`frame-src 'none'` forbids embedding the document as a frame. Check 162 pins
all three, so the reasoning below cannot be quietly undone by a later edit to
the policy. See [Report Export and Display](#report-export-and-display).

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
request-id namespace `gui-` (`crates/eggsec-gui/src/connection.rs:30-35`). The
`ClientIdentity` seam is documented in
[frontend_client.md](frontend_client.md); the point here is that the GUI
supplies its own identity rather than inheriting the TUI's.

### Connection sequence

`GuiConnection::connect` performs, in order:

1. **Connect** to the Unix socket at the configured path.
2. **Protocol-version check** — the daemon's advertised `protocol_version` must
   match `DAEMON_PROTOCOL_VERSION` (`crates/eggsec-gui/src/connection.rs:124`).
   A mismatch is a hard failure, not a warning.
3. **Capability check** — the daemon must advertise the `operation_schema`
   capability (`crates/eggsec-gui/src/connection.rs:138`,
   `crates/eggsec-daemon-protocol/src/protocol.rs:61`). Without it the daemon
   cannot publish the request schemas the form engine is built on, so the GUI
   refuses to connect rather than presenting empty forms.
4. **Session creation** with the `DesktopGuiManual` surface
   (`crates/eggsec-gui/src/connection.rs:147-152`).
5. **Catalog fetch** — `describe_operations()` yields the published task kinds
   and their schemas (`crates/eggsec-gui/src/connection.rs:160`).

`ApprovePolicy` is wired (milestone 011, `ADR-0012`). A manual operation
whose policy demands a confirmation is held at that point: the daemon exposes
the decision as `RuntimeEvent::PolicyDecisionRequired`, and the GUI shows the
policy's own prompt with **Approve** and **Reject** buttons.

The GUI's part in this is small and strictly bounded:

- The answer goes to the daemon as *yes* or *no*. **It never names the
  authorisation.**
- What the answer permits is computed by the policy kernel on the daemon's
  executor side, which re-runs its own authorisation path with an override
  bounded by exactly the confirmation classes the kernel demanded. The GUI
  cannot widen it.
- **The GUI never auto-approves.** An auto-approval would be the GUI settling
  an authorisation question that only the daemon's enforcement layer may
  settle.
- A task with nothing pending is an error the operator sees, not a silent
  no-op.

A strict surface never holds a decision, so there is nothing for a GUI to
answer.

### The no-op executor condition

The GUI requires an `eggsec-daemon` with the engine executor **compiled in**.
By default that daemon is one the operator started: the GUI connects to a
running daemon and never starts a process.

### The opt-in spawn route

The one exception is **opt-in**: when the operator names a daemon executable
with `--daemon-bin <PATH>`, the GUI starts that executable itself and owns
its lifetime (`crates/eggsec-gui/src/daemon.rs`). There is no `PATH` lookup
and no sidecar convention — an unnamed executable is never found, because a
GUI that silently starts *some* daemon can start the wrong one. The option
lives on `GuiOptions.daemon_bin` and defaults to `None`
(`crates/eggsec-gui/src/config.rs:26-31`).

The spawn route follows four rules:

- **The pre-flight check is a connection, never a `stat`.** The daemon
  unlinks its socket path before binding it, so a second daemon started at
  the same path steals a live socket. Only a connection tells a live daemon
  from a socket file left behind by a crashed one
  (`crates/eggsec-gui/src/daemon.rs:96-109`). A stale socket file does not
  block the spawn, and a live daemon is detected so the GUI does not steal it
  (`crates/eggsec-gui/src/commands.rs:340-346`).
- **The child is started from an argv vector, never a shell string.**
  `spawn_argv` builds `[daemon_bin, "--full-executor", "--socket-path",
  socket_path]` (`crates/eggsec-gui/src/daemon.rs:80-87`), so a binary path or
  a socket path with a space in it cannot become an injection surface.
- **Readiness is a handshake, not a log line.** The GUI waits for a
  successful connection that completes the protocol handshake — never for
  the daemon's "Daemon listening on ..." log line, which is a log format and
  not a contract. The wait is bounded by `READINESS_TIMEOUT` (60 s) and
  retried every `READINESS_POLL_INTERVAL` (1 s); each probe is itself
  bounded by `PROBE_TIMEOUT` (2 s)
  (`crates/eggsec-gui/src/daemon.rs:49-62`). A child that exits during the
  wait is reported as a failure with its exit status, not left to run out the
  clock (`crates/eggsec-gui/src/daemon.rs:199-213`).
- **The child does not outlive the GUI.** Teardown sends **SIGTERM first** —
  the daemon's own shutdown path — and escalates to an unconditional kill
  only after the `TERMINATION_GRACE` (5 s) expires
  (`crates/eggsec-gui/src/daemon.rs:298-332`). SIGHUP is never sent: the
  daemon does not handle it. A readiness timeout terminates the child before
  the error is returned, so a failed start leaves no orphan behind
  (`crates/eggsec-gui/src/daemon.rs:153-166`).

The GUI reports a daemon that dies; it never silently respawns one. A shutdown
watcher stops the supervised child when the window closes, so a killed GUI
does not leave the daemon it started running
(`crates/eggsec-gui/src/shell.rs:61-77`).

The daemon binary's `--full-executor` is a **runtime** flag over a **cargo**
feature: the binary must be *built* with the daemon's `service` profile (its
deprecated `full-executor` alias selects the same profile) for the flag to
have any effect. A binary built without it accepts the flag, warns, and falls
back to a no-op executor that publishes an **empty** task-kind list and rejects
every task (`crates/eggsec-daemon/src/main.rs`). See
[docs/DAEMON.md](../docs/DAEMON.md#obtaining-the-daemon) for the obtainability
recipe:

```bash
cargo install --path crates/eggsec-daemon --features service --locked
```

`eggsec daemon start` is no longer in that state: the CLI's start path hosts
the **engine** executor
(`crates/eggsec-cli/src/daemon_cli.rs`), so a CLI-hosted daemon advertises
and can execute the conservative task kinds. What it does **not** do is
persist sessions — the daemon binary is the persistent one.

The GUI therefore does two things instead of offering a submit button that
cannot work:

- Its connection diagnostic names the socket path and the exact command that
  starts a usable daemon (`crates/eggsec-gui/src/connection.rs:58`).
- When the connected daemon advertises **no task kinds**, the GUI reports the
  no-op-executor condition explicitly
  (`crates/eggsec-gui/src/connection.rs:180`).

### Never retry in a tight loop

The GUI never retries the connection in a tight loop. A failed connect is
reported to the operator with the socket path and the remedy, and the operator
retries deliberately. A retry loop against a missing daemon is a spin at 100%
CPU for a condition that only an operator can fix.

## The IPC Command Surface

Twelve Tauri commands, each a narrow pass-through
(`crates/eggsec-gui/src/commands.rs`, registered at
`crates/eggsec-gui/src/shell.rs`):

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
| `coverage()` | The reachability table for every published task kind. |
| `export_report(task_id, format, path)` | Write a completed task's report to a file. |
| `report_for(task_id, format)` | A completed task's report, as data. |
| `daemon_status()` | The daemon attachment, socket, feature set and capability count. |

`report_for` and `export_report` are separate on purpose: the first delivers
the report's structured content to the webview, the second writes a file to a
path the operator chose (`crates/eggsec-gui/src/commands.rs:640-676`,
`crates/eggsec-gui/src/commands.rs:688-729`).

**The command set is pinned in three places, and they move together.**
`declared_command_set_is_exact` asserts the declared list is the whole list
(`crates/eggsec-gui/src/commands.rs:993`),
`every_command_declares_its_argument_naming` asserts each command fixes its
argument naming (`crates/eggsec-gui/src/commands.rs:1024`), and the Tauri
handler registration is the list that is actually served
(`crates/eggsec-gui/src/shell.rs:33-46`). A command added to one and not the
others fails a test or silently never reaches the webview.

**There is no generic "send any command" entry point.** The webview cannot
name a `ClientCommand` variant, cannot set the runtime surface, and cannot set
the scope. The surface is fixed by the GUI (`RuntimeSurface::DesktopGuiManual`,
`crates/eggsec-gui/src/form.rs:300`), and the scope is resolved by the daemon.

## The Coverage Model

`crates/eggsec-gui/src/coverage.rs` answers one question per published task
kind: **can this GUI run it?** The answer is *derived*, never transcribed:
the variants, operation ids, capability names and required-feature lists all
arrive as arguments — the daemon's own published catalog, the daemon's
advertised capability list, and the daemon's published enabled-feature set
(`crates/eggsec-gui/src/coverage.rs:85-98`). One row per published task kind
(`crates/eggsec-gui/src/coverage.rs:58-71`).

Check 161 pins this: the GUI's production Rust halves and its frontend may not
name an operation id or a capability name at all, and the guard reads those
names from their owners (`crates/eggsec-policy/src/catalog.rs` and
`crates/eggsec-runtime/src/capabilities.rs`) rather than transcribing them.

The verdict is one of six (`crates/eggsec-gui/src/coverage.rs:37-53`):

| Verdict | What produces it |
|---|---|
| `available` | The catalog exposes the operation on the manual surface, the daemon advertises the capability, and every required feature is enabled. |
| `not_exposed` | The catalog does not expose the operation on the manual surface. |
| `not_advertised` | The daemon does not advertise the capability this task kind needs. |
| `feature_disabled` | The daemon advertises the capability but was built without one of the operation's required features. |
| `unverified` | The daemon's enabled-feature set is unknown, so availability cannot be verified. |
| `blocked` | A live denial proved the operation unavailable in this session. |

Two distinctions the model refuses to blur:

- **An unknown feature set is not an empty one.** A daemon too old to publish
  the field yields `None`, which means *unknown*. A task kind whose required
  features cannot be confirmed is `unverified` — never `feature_disabled`,
  and never silently dropped.
- **"The daemon withholds the capability" is not "the daemon lacks the
  feature."** A daemon that has every required feature and still does not
  advertise the capability is making a choice, not a build fact, and the
  reason says which one it is.

### The availability resolver and the denial ledger

`crates/eggsec-gui/src/availability.rs` turns a live failure into a fact the
coverage table can use. `denial_from_message` reads the engine's own
`FeatureMissing` denial out of a daemon error message
(`crates/eggsec-gui/src/availability.rs:68-77`): the policy layer phrases it
as `required feature '<X>' is not enabled`
(`crates/eggsec-policy/src/decision.rs:926`) and the dispatch layer as
`requires feature '<X>' which is not compiled in`
(`crates/eggsec/src/dispatch/canonical_execution.rs:626`). The GUI recognises
both markers (`crates/eggsec-gui/src/availability.rs:59-60`); every other
message yields `Denial::Other`, carried verbatim. A denial is never guessed —
the GUI records what the engine said.

The `DenialLedger` holds **one entry per task-kind variant**
(`crates/eggsec-gui/src/availability.rs:102-113`), so it is bounded by the
catalog's size rather than by the number of denials, and the most recent
denial for a variant replaces the earlier one.

A **learned denial wins** over every static verdict
(`crates/eggsec-gui/src/coverage.rs:106-133`): when the engine refuses an
operation at run time, that refusal is the engine's own word about the
running daemon, and no published list can overrule it. The GUI records the
denial against the failed task's variant
(`crates/eggsec-gui/src/commands.rs:927-960`), and the coverage table then
reports that variant as `blocked` with the engine's own reason.

### The event channel

The webview receives exactly one event on the bus: `eggsec://task-event`
(`crates/eggsec-gui/src/commands.rs:35`). Its payload is one of three shapes —
`task_upsert`, `logs`, or `counters` (`crates/eggsec-gui/src/commands.rs:291-313`). The GUI subscribes **once per session**
and never spawns a task per event: one forwarder task lives for the lifetime
of the subscription, coalescing log lines and flushing on a 100 ms interval
(`crates/eggsec-gui/src/commands.rs:844`).

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

## Report Export and Display

The GUI writes the **same artifacts the CLI writes** (`crates/eggsec-gui/src/export.rs`):
JSON, CSV, HTML, SARIF, JUnit and Markdown, through the same
`eggsec-output` renderers. There is no GUI-specific report format and no
second renderer. The exported HTML is the CLI's document byte for byte —
`convert_to_html`, remote chart-library reference and all — because the
exported file is the operator's artifact, opened in the operator's browser
(`crates/eggsec-gui/src/export.rs:312-323`).

### The format map

`ReportFormat` names six formats, each with the extension the exported file
carries (`crates/eggsec-gui/src/export.rs:63-100`):

| Format | Extension | Export rendering | Display rendering |
|---|---|---|---|
| `Json` | `.json` | `convert_to_json` | inert text |
| `Csv` | `.csv` | `convert_to_csv` | inert text |
| `Html` | `.html` | `convert_to_html` | **none — export-only** |
| `Sarif` | `.sarif` | `convert_to_sarif` | inert text |
| `Junit` | `.xml` | `convert_to_junit` | inert text |
| `Markdown` | `.md` | `convert_to_markdown` | inert text |

The JUnit extension is `.xml`, not `.junit`, and the Markdown extension is `.md`,
not `.markdown`: the map is the extension the operator's file actually gets
(`crates/eggsec-gui/src/export.rs:92-100`), while the wire name stays
`junit` and `markdown` (`crates/eggsec-gui/src/export.rs:123-129`).

### The write discipline

The write discipline is the TUI's, unchanged: a destination that is not a
regular file is **refused**, an existing regular file is **backed up** to
`<path>.bak` before it is replaced, and every failure is an `Err` rather than
a silent overwrite (`crates/eggsec-gui/src/export.rs:349-363`,
`crates/eggsec-gui/src/export.rs:391-411`).

### The report-display decision

**The GUI does not render the engine's HTML document.** That document carries
an inline `<style>` block and an inline `<script>` block, and the GUI's CSP
sets `style-src 'self'`, `script-src 'self'` and `frame-src 'none'`: an
inline style does not apply, an inline script does not execute, and the
document cannot be framed. Rendering it faithfully would mean relaxing the
CSP, and relaxing the CSP is the wrong trade — the report's content is
attacker-influenced (banners, service names and finding text all come from
the target), and the CSP is the control that stops that content from doing
anything.

So the report reaches the webview as **data** (`export::ReportView`,
`crates/eggsec-gui/src/export.rs:188-205`), and the frontend renders it with
the GUI's own stylesheet and its own escaping. The frontend uses `textContent`
and `createElement`; there is no `innerHTML` anywhere in
`crates/eggsec-gui/frontend/app.js`.

`render_for_display(ReportFormat::Html, ..)` returns
`Err(ExportError::NoDisplayRendering)`
(`crates/eggsec-gui/src/export.rs:337-347`): **the HTML format is
export-only**. The five text formats keep their inert-text display rendering,
and the frontend shows them as text.

The CSP is **not** relaxed to accommodate this, and Check 162 pins it: the
policy may not contain `'unsafe-inline'`, `frame-src` must stay `'none'`, no
GUI production source or frontend file may reference a remote origin, the
frontend may not use a markup-injection sink, and the GUI may not consume an
engine-produced HTML document. `eggsec-output` gained
`convert_to_html_self_contained` — a tested capability in `eggsec-output`, not
a GUI feature. The GUI does not consume it, because the CSP makes document
embedding unavailable.

## Platform Prerequisites and Build Profiles

The default (`shell`) profile needs the platform's webview development
packages:

- **Linux**: `libwebkit2gtk-4.1-dev`, `libgtk-3-dev`, `librsvg2-dev`,
  `patchelf`, and the `libayatana-appindicator3-dev` / `libxdo-dev` packages
  for the tray.
- **macOS**: the system webview is present; no extra packages.
- **Windows**: out of scope — see below.

The `--no-default-features` profile builds the GUI crate without the shell —
the `tauri` crate is not in the graph, `build.rs` skips `tauri_build::build()`
(the `tauri` crate's metadata does not exist in that profile, so the call would
panic — `crates/eggsec-gui/build.rs:8-10`), and `run()` returns
`GuiError::ShellNotBuilt`. That profile is what `make check` and
`make check-msrv` build, so the no-shell build stays green without the webview
SDK installed.

### The supported platforms and the measured floor

The desktop GUI is supported on **Linux and macOS**. **Windows is out of
scope** — `plans/adrs/ADR-0010-gui-supported-platform-scope.md` decides it,
and the measured reason is that the GUI's *unconditional* dependency
`eggsec-frontend-client` imports the Unix-domain-socket transport without a
`cfg(unix)` gate (`crates/eggsec-frontend-client/src/daemon.rs:9`, `:56`), so
`cargo check -p eggsec-frontend-client --target x86_64-pc-windows-msvc` fails
with E0432/E0433/E0599 and the GUI does not compile for Windows at all.
Re-entry is gated on a Windows daemon transport (named pipes or equivalent
local IPC), which is a daemon-subsystem change and needs its own ADR.

The **Linux platform floor** is Ubuntu 22.04 / Debian 12 or newer:
`webkit2gtk-4.1` implies it (`ADR-0009`). Older distributions are below the
floor, and the `ADR-0009` egui fallback is the sanctioned escape hatch. The
workspace's own gate is looser than the build — `Makefile:21-30` accepts
`webkit2gtk-4.1` **or** `webkit2gtk-4.0` — so a 4.0-only host can pass the
gate and then fail to compile. `make check-gui` is the ungated check.

### The qualification matrix

What was actually built, launched, and exercised — per platform, per
behaviour — is recorded in
[`docs/GUI_PLATFORM_QUALIFICATION.md`](../docs/GUI_PLATFORM_QUALIFICATION.md),
with one evidence file per run under `docs/evidence/gui-qualification/`. The
matrix is the authoritative statement of platform support; the CI job that
promotes a cell is `gui-platform` in `.github/workflows/deep-checks.yml`
(`ubuntu-latest` and `macos-latest`; Windows is excluded by `ADR-0010`).

### The bundler and the distribution hand-off

`bundle.active` stays `false` (`crates/eggsec-gui/tauri.conf.json:26-28`).
The roadmap and `ADR-0007` place installers, signing, and notarization in the
separate distribution workstream, and enabling the bundler would make a plain
`make check` emit installers as a side effect. Milestone 007 produces the
binary and the reproducible build recipe (`scripts/build_gui_release.sh`,
which emits the binary plus a build-metadata sidecar); the distribution
workstream inherits the artefact and decides whether to enable the bundler or
ship a raw binary. See
[`docs/GUI_DISTRIBUTION_HANDOFF.md`](../docs/GUI_DISTRIBUTION_HANDOFF.md).

### Resident footprint

Measured on macOS 25.6.0 (`aarch64-apple-darwin`), at rest with the webview
loaded: the application process plus the three WKWebView XPC helpers sum to
125 760 KiB (122.8 MiB) of RSS — 79 760 KiB for the application, 18 512 KiB
for `com.apple.WebKit.GPU`, 7 616 KiB for `com.apple.WebKit.Networking`, and
19 872 KiB for `com.apple.WebKit.WebContent`. The sum double-counts the
WebKit framework pages the helpers share, so it is an upper bound rather than
a private-footprint figure. There is no Linux measurement, and this is an
idle window rather than a running assessment.

## Known Gaps

These are deliberate and documented, not oversights.

- **The GUI manages the daemon's lifecycle only when asked.** By default it
  requires an already-running `eggsec-daemon --full-executor` and never
  spawns, supervises, or stops one. With `--daemon-bin <PATH>` it starts
  that executable and stops it on exit — the opt-in route described above.
- **The socket-path default is duplicated in six places.** The default
  `/tmp/eggsec-daemon.sock` is hardcoded independently in the CLI
  (`crates/eggsec/src/cli/mod.rs:249`), the daemon's config
  (`crates/eggsec-daemon/src/config.rs:22`), the daemon's main
  (`crates/eggsec-daemon/src/main.rs:20`), the GUI's Rust default
  (`crates/eggsec-gui/src/config.rs:19`), and twice in the GUI's frontend
  (`crates/eggsec-gui/frontend/app.js:23`,
  `crates/eggsec-gui/frontend/index.html:25`). The GUI's `--socket` flag
  overrides it.

## Key Invariants

1. The GUI links no engine, daemon, CLI or TUI code. It depends only on
   `eggsec-frontend-client`, `eggsec-runtime`, `eggsec-daemon-protocol`,
   `eggsec-ui-model` and `eggsec-output` (Check 155).
2. The Tauri dependency lives only in `crates/eggsec-gui` (Check 156).
3. The GUI declares no TLS stack of its own: no `rustls-tls`, no `native-tls`,
   no direct `reqwest`/`rustls`/`native-tls` dependency (Check 157).
4. The CSP names no wildcard source and no remote origin (Check 158).
5. The form is generated from the published schema. There is no per-operation
   code and no hand-written operation table (Check 159).
6. Log retention is capped per task and the forwarder caps each flush.
   Unbounded retention is a disqualifying defect (Check 160).
7. The coverage model is derived, never transcribed. The GUI's production
   halves and its frontend name no operation id and no capability name
   (Check 161).
8. The report display carries no remote origin and no executable content. The
   HTML format is export-only (Check 162).
9. The version pins are exact. Loosening them breaks `make check-msrv`.
10. The frontend has no npm dependency and no build step.
11. The published schema is descriptive, never authoritative. The engine's own
    validation decides whether a task may run.
12. A blank field is "not provided", never a zero value. The fail-safe
    defaults must survive an untouched form.
13. The GUI's approval affordance answers a decision; it never decides one. An
    auto-approval would be the GUI settling an authorisation question that
    only the daemon's enforcement layer may settle.
14. A learned denial wins over every static verdict.

## See Also

- [frontend_client.md](frontend_client.md) — the `ClientIdentity` seam and the
  daemon client the GUI speaks through
- [daemon.md](daemon.md) — the daemon the GUI connects to, its RBAC matrix and
  its audit trail
- [tui.md](tui.md) — the terminal UI, the other frontend on the same contract
- [runtime_bridge.md](runtime_bridge.md) — where `RuntimeSurface::DesktopGuiManual`
  becomes an `ExecutionSurface`
- [overview.md](overview.md) — System-wide architecture and crate counts

*Last verified against source: 2026-10-24 (milestone 006: full operation
coverage, report export, and the opt-in daemon spawn-and-supervise lifecycle;
earlier passes 2026-10-23 for milestones 004–005: Tauri shell skeleton,
schema-driven forms and task dashboard)*
