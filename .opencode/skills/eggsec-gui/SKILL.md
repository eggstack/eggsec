---
name: eggsec-gui
description: "Desktop GUI frontend for Eggsec (Tauri v2, daemon-only) - use when working on crates/eggsec-gui, the shell and its Tauri pins, the schema-driven form engine, the Tauri IPC command surface, the buildless webview frontend, the bounded task dashboard, or the GUI's daemon connection."
---

# Eggsec GUI Skill

The desktop graphical frontend: a Tauri v2 application that reaches the engine
through the daemon's Unix socket and nothing else.

## Crate facts

| Fact | Value |
|------|-------|
| Crate path | `crates/eggsec-gui/` |
| Shape | `src/lib.rs` library + `src/main.rs` binary (the binary is a thin shell around `eggsec_gui::run`, `crates/eggsec-gui/src/main.rs:8`) |
| Depends on | `eggsec-frontend-client`, `eggsec-runtime`, `eggsec-daemon-protocol`, `eggsec-ui-model`, `eggsec-output` only (`crates/eggsec-gui/Cargo.toml:38-47`) |
| Must never depend on | `eggsec` (the engine), `eggsec-daemon`, `eggsec-cli`, `eggsec-tui` |
| Shell framework | Tauri v2, confined to this crate (`crates/eggsec-gui/src/shell.rs:3`) |
| Frontend | Buildless: hand-written HTML, CSS, and vanilla JavaScript, **zero npm dependencies** |
| Features | `default = ["shell"]`; the crate compiles with `--no-default-features` (`crates/eggsec-gui/Cargo.toml:24`) |
| Enforced by | architecture guards 155–172 (no engine edge, framework confinement, no second TLS stack, CSP forbids remote origins, no per-operation form code, bounded event retention, derived coverage model, remote-origin-free report display, and the daemon's derived capability list) |

Headline crate and surface counts live in `architecture/overview.md` — point at
it rather than restating the numbers.

## Module structure

```
crates/eggsec-gui/
├── src/
│   ├── lib.rs      # Crate docs, GuiError, run()
│   ├── main.rs     # The binary: a thin shell around eggsec_gui::run
│   ├── config.rs   # CLI options (--socket, --daemon-bin) and the default socket path
│   ├── connection.rs# The daemon connection, the GUI's identity, the diagnostics
│   ├── coverage.rs # The derived reachability table for every published task kind
│   ├── availability.rs # The denial ledger: what a live refusal taught the GUI
│   ├── export.rs   # The six report formats, the write discipline, the report view
│   ├── daemon.rs   # The opt-in daemon lifecycle: probe, spawn, supervise, stop
│   ├── form.rs     # The generic form engine (schema property → control → RunRequest)
│   ├── tasks.rs    # The bounded task state and the result view
│   ├── commands.rs # The Tauri IPC surface and the event forwarder
│   └── shell.rs    # The Tauri builder
├── frontend/       # The buildless webview frontend (HTML, CSS, vanilla JS)
├── tauri.conf.json # Window, CSP, withGlobalTauri
├── capabilities/default.json
└── build.rs        # Gates tauri_build::build() on the `shell` feature
```

## The boundary (hard rules)

- **The GUI links no engine.** It never imports `EnforcementContext`,
  `EnforcedDispatcher`, `Scope`, or `tool.execute()`, and never constructs an
  `ApprovedExecution` bundle. Every dispatch happens in the daemon, under the
  daemon's own enforcement.
- **The GUI does not depend on `eggsec-daemon`.** The daemon's `service` feature
  (its deprecated `full-executor` alias) pulls the engine, so a GUI that embedded
  a daemon would link the engine and break the boundary. The GUI therefore
  connects to an already-running daemon over its Unix socket, or — opt-in, with
  `--daemon-bin` — starts a daemon binary the operator names, without linking
  the daemon crate.
- **No outbound HTTP.** The GUI enables neither `tauri`'s `rustls-tls` nor
  `native-tls` (`crates/eggsec-gui/Cargo.toml:70`), so the whole
  `reqwest`/`rustls`/`native-tls` closure stays out of the GUI's graph and the
  workspace's ring-only TLS invariant holds without an exception.
- **The GUI holds no authoritative state.** The daemon owns task truth; the
  GUI's map is a bounded *view* of the event stream
  (`crates/eggsec-gui/src/tasks.rs:3`).
- **The surface is stamped in Rust.** The webview cannot set the surface or
  the scope; `assemble_request` serializes `RuntimeSurface::DesktopGuiManual`
  itself (`crates/eggsec-gui/src/form.rs:300`), and the session scope is
  `is_explicit: false` with source `gui-default`
  (`crates/eggsec-gui/src/connection.rs:38`).
- **The CSP forbids remote origins**
  (`crates/eggsec-gui/tauri.conf.json:23`), and the window capability grants
  `core:default` only — no http, shell, fs, process, or updater plugin
  (`crates/eggsec-gui/capabilities/default.json:6`).
- **The GUI names no operation id and no capability name.** Its coverage table
  is derived from what the daemon publishes; guard 161 enforces it.

## The Tauri version pins are exact, and load-bearing

The workspace pins Tauri to the 2.11.x line with `=` requirements
(`Cargo.toml:93`): `tauri = "=2.11.6"`, `tauri-build = "=2.6.3"`,
`tauri-runtime = "=2.11.3"`, `tauri-runtime-wry = "=2.11.4"`,
`tauri-macros = "=2.6.3"`, `tauri-utils = "=2.9.3"`.

The exactness is the point, not a style preference:

- the next lines up (tauri 2.12, tauri-utils 2.10, tauri-macros 2.7) declare
  **MSRV 1.90**, above the workspace's 1.89 floor;
- `tauri` declares its runtime and utils dependencies with **caret**
  requirements, so without the GUI crate's direct exact pins a plain
  `cargo update` would float the closure onto the 1.90 line and break
  `make check-msrv`.

The GUI crate depends on `tauri-macros`, `tauri-runtime`,
`tauri-runtime-wry`, and `tauri-utils` **directly** for that reason, even
though it calls none of them directly (`crates/eggsec-gui/Cargo.toml:83-86`).
`tauri-utils` is also a build-dependency so the no-shell build stays off the
1.90 line (`crates/eggsec-gui/Cargo.toml:92`).

Do not relax a `=` to `^` here without a recorded ADR: raising the workspace
MSRV to 1.90 is a separate, explicit trade.

## The buildless frontend

- No `package.json`, no lockfile, no bundler, no build step: the frontend is
  a static directory served as-is (`frontendDist`, `crates/eggsec-gui/tauri.conf.json:7`).
- `withGlobalTauri: true` (`crates/eggsec-gui/tauri.conf.json:10`) is what
  lets the vanilla JavaScript reach the IPC commands without a bundler.
- **Zero npm dependencies is a rule, not an accident.** A remote script
  source or a CDN import is not permitted: it would defeat the CSP and add
  supply-chain surface to a security tool. This discharges `ADR-0009`'s
  deferred JavaScript-dependency-policy question — the dependency set is
  empty. Any JavaScript dependency admitted later needs a recorded policy with
  an owner and a review-by date.
- The repository's root `package.json` is an empty object (`{}`); it is not a
  frontend manifest.

## The IPC surface

Exactly thirteen narrow commands, registered in
`crates/eggsec-gui/src/shell.rs:33-47`:

| Command | Source |
|---|---|
| `connect_daemon(socket)` | `crates/eggsec-gui/src/commands.rs` |
| `list_operations()` | `crates/eggsec-gui/src/commands.rs` |
| `list_task_kinds()` | `crates/eggsec-gui/src/commands.rs` |
| `form_for(variant)` | `crates/eggsec-gui/src/commands.rs` |
| `submit_task(variant, values)` | `crates/eggsec-gui/src/commands.rs` |
| `cancel_task(task_id)` | `crates/eggsec-gui/src/commands.rs` |
| `cancel_active()` | `crates/eggsec-gui/src/commands.rs` |
| `approve_task(task_id, approved, reason)` | `crates/eggsec-gui/src/commands.rs` |
| `task_snapshot()` | `crates/eggsec-gui/src/commands.rs` |
| `coverage()` | `crates/eggsec-gui/src/commands.rs` |
| `export_report(task_id, format, path)` | `crates/eggsec-gui/src/commands.rs` |
| `report_for(task_id, format)` | `crates/eggsec-gui/src/commands.rs` |
| `daemon_status()` | `crates/eggsec-gui/src/commands.rs` |

`report_for` and `export_report` are separate on purpose: the first delivers
the report's structured content to the webview, the second writes a file to a
path the operator chose.

The command set is pinned in **three** places, and they must move together:
`declared_command_set_is_exact` (`crates/eggsec-gui/src/commands.rs:993`),
`every_command_declares_its_argument_naming`
(`crates/eggsec-gui/src/commands.rs:1024`), and the Tauri handler
registration (`crates/eggsec-gui/src/shell.rs:33-46`).

**There is no generic command-forwarding entry point.** Nothing accepts an
arbitrary daemon message, sets the session surface, or sets the scope
(`crates/eggsec-gui/src/commands.rs:3`). Adding one would hand the webview a
wider capability than the operator's UI needs.

One event on the bus: `eggsec://task-event`
(`crates/eggsec-gui/src/commands.rs:35`), carrying `task_upsert`, `logs`, or
`counters` (`crates/eggsec-gui/src/commands.rs:291-313`).

## The coverage model: derived, never transcribed

`crates/eggsec-gui/src/coverage.rs` answers, per published task kind, whether
this GUI can run it. Every input arrives as an argument — the daemon's
catalog, its advertised capability list, its published enabled-feature set —
so a task kind published tomorrow is covered with no GUI change
(`crates/eggsec-gui/src/coverage.rs:85-98`). One row per published task kind.

The GUI names **no operation id and no capability name** in its production
sources or its frontend. Guard 161 reads those names from their owners
(`crates/eggsec-policy/src/catalog.rs` and
`crates/eggsec-runtime/src/capabilities.rs`) and fails if the GUI names any of
them.

| Verdict | What produces it |
|---|---|
| `available` | Exposed on the manual surface, capability advertised, required features enabled. |
| `not_exposed` | The catalog does not expose the operation on the manual surface. |
| `not_advertised` | The daemon does not advertise the capability. |
| `feature_disabled` | Advertised, but a required feature is not enabled. |
| `unverified` | The enabled-feature set is **unknown** (the daemon published none). |
| `blocked` | A live denial proved it unavailable in this session. |

- **An unknown feature set is not an empty one.** `None` means unknown, so a
  task kind whose requirements cannot be confirmed is `unverified` — never
  `feature_disabled`, never dropped.
- **"Withholds the capability" is not "lacks the feature."** The reason text
  distinguishes them.
- A **`required_features` list is a requirement, not a state report.** It says
  what the operation needs; the daemon's published enabled-feature set says what
  the daemon has. Never read one as the other.
- A **learned denial wins** over every static verdict
  (`crates/eggsec-gui/src/coverage.rs:106-133`). The ledger holds one entry per
  published task kind, so it is bounded by the catalog's size.

### The availability resolver

`crates/eggsec-gui/src/availability.rs` turns a live failure into a fact the
coverage table can use. `denial_from_message` reads the engine's own
`FeatureMissing` denial out of a daemon error message
(`crates/eggsec-gui/src/availability.rs:68-77`): the policy layer phrases it
as `required feature '<X>' is not enabled`
(`crates/eggsec-policy/src/decision.rs:926`) and the dispatch layer as
`requires feature '<X>' which is not compiled in`
(`crates/eggsec/src/dispatch/canonical_execution.rs:626`). Both markers are
recognised (`crates/eggsec-gui/src/availability.rs:59-60`); every other message
yields `Denial::Other`, carried verbatim. A denial is never guessed.

The `DenialLedger` holds **one entry per task-kind variant**
(`crates/eggsec-gui/src/availability.rs:102-113`), so it is bounded by the
catalog's size, and the most recent denial for a variant replaces the earlier
one. The GUI records the denial against the failed task's variant
(`crates/eggsec-gui/src/commands.rs:927-960`).

## The form engine: no per-operation code anywhere

One mapping from a published JSON Schema property to one control descriptor,
and one assembly path from control values to a typed `RunRequest`
(`crates/eggsec-gui/src/form.rs:3`). A task kind the engine has not published
yet gets a form with no GUI change at all.

- **No match or switch over task-kind variant names**, no hand-written field
  table, no per-operation branch. Guard 159 pins this.
- Five control kinds only (`crates/eggsec-gui/src/form.rs:36`): `text`,
  `number`, `checkbox`, `list`, `json`.
- The published schema's measured shape — how many variants and properties it
  carries, and that it contains no `$ref`, no `$defs`, no `enum` constraint,
  and no nested objects — is recorded in the subsystem roadmap's milestone-005
  entry and the registry. Read it there rather than re-deriving it.
- The absence of `enum` constraints means the schema **cannot** drive
  dropdowns for enum-like fields: the GUI renders free text and the engine
  validates.
- The schema is **descriptive, never authoritative**
  (`crates/eggsec-gui/src/form.rs:9`): it decides form *shape* only. Whether
  an operation may run, its risk, and its required features come from the
  daemon's operation catalog and its live capabilities.

## Three-layer validation, and "blank means null"

Validation is layered (`crates/eggsec-gui/src/form.rs:16`):

1. **Advisory**, from the schema — the control's declared type and bounds.
2. **Exact**, from the DTOs — the assembled request is deserialized into the
   same `RunRequest` the engine will deserialize
   (`crates/eggsec-gui/src/form.rs:321`).
3. **Authoritative**, in the engine — the engine's own validation still
   decides whether the task may run.

Layers 1–2 exist to save the operator a round trip. They never stand in for
layer 3, and neither encodes a per-operation rule.

**A blank field sends `null`** — never `""`, `0`, `false`, or `[]`
(`crates/eggsec-gui/src/form.rs:257`). Several published parameters carry
non-null serde defaults and one is a fail-safe flag (`dry_run`, default
`true`); a GUI that sent `false` for an untouched field would turn a
simulation into a live action. In the Rust assembly path, a field that is not
required and whose type does not accept `null` is **omitted** from the
request so the request DTO applies its own default
(`crates/eggsec-gui/src/form.rs:281`).

## Bounded task state

- The event stream is the source of truth; the GUI's map is a bounded view of
  it (`crates/eggsec-gui/src/tasks.rs:3`).
- At most **200** log lines are retained per task (`MAX_LOG_LINES_PER_TASK`,
  `crates/eggsec-gui/src/tasks.rs:28`), in a capped ring that drops the
  *oldest* lines and counts them.
- The event forwarder coalesces and forwards at most **50** log lines per
  flush (`MAX_LOG_LINES_PER_FLUSH`, `crates/eggsec-gui/src/tasks.rs:35`),
  reporting the rest as dropped.
- Dropped counts are **reported, never silently swallowed**: the operator who
  sees "42 earlier lines dropped" knows the view is partial.
- The GUI subscribes **once per session** and never spawns a task per event
  (`crates/eggsec-gui/src/commands.rs:827`).

## Policy decisions: the GUI answers, it never decides

A task waiting on a policy decision shows **the policy's own prompt** — the
wording comes from the daemon, not from the GUI — with **Approve** and
**Reject** buttons (`crates/eggsec-gui/frontend/app.js`).

- The answer goes to the daemon as *yes* or *no*. **It never names the
  authorisation.**
- What the answer permits is computed by the policy kernel on the daemon's
  executor side, bounded by exactly the confirmation classes the kernel
  demanded. The GUI cannot widen it.
- **The GUI never auto-approves.** An auto-approval would be the GUI settling
  an authorisation question that only the daemon's enforcement layer may
  settle.
- A task with nothing pending is an error the operator sees, not a silent
  no-op: the daemon refuses the answer because there is no decision to
  resolve.

See `plans/adrs/ADR-0012-daemon-mediated-policy-approval.md`.

## Result rendering

Driven by `eggsec-ui-model`'s `RENDERER_REGISTRY` through
`renderer_for_kind` (`crates/eggsec-gui/src/tasks.rs:20`) — the only source of
per-kind metadata. An unknown kind degrades to a generic JSON render, never
an empty view (`crates/eggsec-gui/src/tasks.rs:418-421`).

`supports_rich_tui` describes the **TUI's** capability
(`crates/eggsec-ui-model/src/renderer_registry.rs:18`). It is not a capability
query for the GUI; do not gate GUI rendering on it.

## Availability and exposure

- Availability comes from the daemon's live capabilities, never from the GUI's
  own reasoning. The join key is the task kind's `capability_name` against
  the capability list the daemon advertises
  (`crates/eggsec-gui/src/commands.rs:146-153`).
- Exposure comes from `OperationDescription::manual_exposable`
  (`crates/eggsec-gui/src/commands.rs:442-444`). The GUI is a **manual** surface.
- `unavailable_reason` states why a task kind cannot be submitted
  (`crates/eggsec-gui/src/commands.rs:458`).
- The full per-task-kind reachability verdict — exposure, advertisement, the
  enabled-feature set and any learned denial — is the **coverage** table, not
  this list. See [The coverage model](#the-coverage-model-derived-never-transcribed).

## The daemon expectation

The GUI requires an `eggsec-daemon` with the engine executor **compiled in**.
By default the daemon is one the **operator** started: the GUI connects to a
running daemon and starts no process at all.

The **opt-in** exception is `--daemon-bin <PATH>`: the GUI starts that
executable itself and owns its lifetime
(`crates/eggsec-gui/src/daemon.rs`). There is no `PATH` lookup and no sidecar
convention — an unnamed executable is never found. The option lives on
`GuiOptions.daemon_bin` and defaults to `None`
(`crates/eggsec-gui/src/config.rs:26-31`).

- The pre-flight liveness check is a **connection attempt**, never a `stat`:
  the daemon unlinks its socket path before binding it, so only a connection
  tells a live daemon from a stale socket file
  (`crates/eggsec-gui/src/daemon.rs:96-109`). A stale socket file does not
  block the spawn; a live daemon is detected so the GUI does not steal it.
- The child is started from an **argv vector**, never a shell string:
  `[daemon_bin, "--full-executor", "--socket-path", socket_path]`
  (`crates/eggsec-gui/src/daemon.rs:80-87`).
- Readiness is a **protocol handshake**, never the daemon's "Daemon
  listening on ..." log line. The wait is bounded by `READINESS_TIMEOUT`
  (60 s) and retried every `READINESS_POLL_INTERVAL` (1 s); each probe is
  bounded by `PROBE_TIMEOUT` (2 s)
  (`crates/eggsec-gui/src/daemon.rs:49-62`). A child that exits during the
  wait is reported as a failure with its exit status
  (`crates/eggsec-gui/src/daemon.rs:199-213`).
- Teardown is **SIGTERM first**, escalating to an unconditional kill after the
  `TERMINATION_GRACE` (5 s) expires (`crates/eggsec-gui/src/daemon.rs:298-332`).
  SIGHUP is never sent.
- A readiness timeout **terminates the child** before the error is returned:
  a failed start leaves no orphan (`crates/eggsec-gui/src/daemon.rs:153-166`).
- A shutdown watcher stops the supervised child when the window closes, so a
  killed GUI does not leave the daemon it started running
  (`crates/eggsec-gui/src/shell.rs:61-77`).
- The GUI reports a dead daemon; it never silently respawns one.

`--full-executor` is a *runtime* flag over the daemon's `service` *cargo*
feature (its deprecated `full-executor` alias selects the same profile), so
the binary must be **built** with that feature:

```bash
cargo install --path crates/eggsec-daemon --features service --locked
```

A binary built without the cargo feature accepts the flag and falls back to a
**no-op executor** that publishes an **empty** task-kind list, so the GUI's
diagnostic names the exact start command
(`crates/eggsec-gui/src/connection.rs:58`) and the GUI reports the
no-op-executor condition — the daemon advertises no task kinds
(`crates/eggsec-gui/src/connection.rs:180`) — rather than offering a submit
button that cannot work.

`eggsec daemon start` is no longer in that state: the CLI's start path hosts
the **engine** executor, so a CLI-hosted daemon advertises and can execute
the conservative task kinds. It does **not** persist sessions — the daemon
binary is the persistent one. See `docs/DAEMON.md` for the obtainability
recipe and its limits.

## Platform prerequisites and build profiles

- Supported platforms: **Linux and macOS**. **Windows is out of scope** —
  `plans/adrs/ADR-0010-gui-supported-platform-scope.md` decides it, and the
  measured reason is that the GUI's unconditional `eggsec-frontend-client`
  dependency imports the Unix-domain-socket transport without a `cfg(unix)`
  gate, so the GUI does not compile for Windows. Re-entry needs a Windows
  daemon transport, which is a daemon-subsystem change with its own ADR.
- macOS uses the system webview; Linux needs `webkit2gtk-4.1`,
  which sets the supported floor at Ubuntu 22.04 / Debian 12 or newer.
- The crate compiles in two profiles:
  - `--features shell` (the default) — the full desktop application. This is
    the profile that needs the platform webview SDK.
  - `--no-default-features` — the library without the shell, which is what
    keeps `cargo check --workspace --no-default-features` (the `make check`
    baseline) green on hosts with no webview SDK. `run()` then reports
    `ShellNotBuilt` and exits non-zero rather than pretending to work
    (`crates/eggsec-gui/src/lib.rs:71-74`).
- `build.rs` gates `tauri_build::build()` on `CARGO_FEATURE_SHELL` because
  the `tauri` crate's build metadata does not exist when the shell feature is
  off (`crates/eggsec-gui/build.rs:8`).
- **What was actually built and exercised** per platform, per behaviour, is
  recorded in `docs/GUI_PLATFORM_QUALIFICATION.md`, with one evidence file per
  run under `docs/evidence/gui-qualification/`. That matrix — not a claim —
  is the authoritative statement of platform support; the `gui-platform` CI
  job is what promotes a cell.
- `bundle.active` stays `false`. Installers, signing, and notarization belong
  to the separate distribution workstream; the handoff artefact and its
  known limitations are in `docs/GUI_DISTRIBUTION_HANDOFF.md`.
- The daemon the GUI connects to has its own engine-feature profile and build
  recipe, documented in `docs/DAEMON_FEATURE_PROFILE.md`. The GUI's
  prerequisites and the daemon's are deliberately kept apart.

## Build, test, run

```bash
cargo check -p eggsec-gui --features shell          # the desktop profile
cargo check -p eggsec-gui --no-default-features     # the make check baseline
cargo test -p eggsec-gui --features shell            # unit tests
cargo +1.89 check -p eggsec-gui --features shell     # the MSRV gate
```

Run it against a usable daemon:

```bash
eggsec-daemon --full-executor --socket-path /tmp/eggsec-daemon.sock
cargo run -p eggsec-gui --features shell -- --socket /tmp/eggsec-daemon.sock
```

The GUI's own flags are `--socket <PATH>`/`--socket=<PATH>` and
`--daemon-bin <PATH>`/`--daemon-bin=<PATH>`; the default socket path is
`/tmp/eggsec-daemon.sock` (`crates/eggsec-gui/src/config.rs:19`).

## Report export and display

`crates/eggsec-gui/src/export.rs` writes the **same artifacts the CLI writes**,
through the same `eggsec-output` renderers. There is no GUI-specific report
format and no second renderer.

| Format | Extension | Export rendering | Display rendering |
|---|---|---|---|
| `Json` | `.json` | `convert_to_json` | inert text |
| `Csv` | `.csv` | `convert_to_csv` | inert text |
| `Html` | `.html` | `convert_to_html` | **none — export-only** |
| `Sarif` | `.sarif` | `convert_to_sarif` | inert text |
| `Junit` | `.xml` | `convert_to_junit` | inert text |
| `Markdown` | `.md` | `convert_to_markdown` | inert text |

The extension map is `crates/eggsec-gui/src/export.rs:92-100`; JUnit's
extension is `.xml` and Markdown's is `.md`, while the wire name stays
`junit` and `markdown` (`crates/eggsec-gui/src/export.rs:123-129`).

- **The write discipline is the TUI's, unchanged.** A destination that is not a
  regular file is **refused**, an existing regular file is **backed up** to
  `<path>.bak` before it is replaced, and every failure is an `Err` rather than
  a silent overwrite (`crates/eggsec-gui/src/export.rs:349-363`).
- **The GUI does not render the engine's HTML document.** That document carries
  an inline `<style>` block and an inline `<script>` block, and the GUI's CSP
  sets `style-src 'self'`, `script-src 'self'` and `frame-src 'none'`: an
  inline style does not apply, an inline script does not execute, and the
  document cannot be framed. Rendering it faithfully would mean relaxing the
  CSP, and relaxing the CSP is the wrong trade — the report's content is
  attacker-influenced (banners, service names and finding text all come from
  the target), and the CSP is the control that stops that content from doing
  anything.
- So the report reaches the webview as **data** (`export::ReportView`), and the
  frontend renders it with the GUI's own stylesheet and its own escaping. The
  frontend uses `textContent` and `createElement`; there is no `innerHTML`
  anywhere in `crates/eggsec-gui/frontend/app.js`.
- `render_for_display(ReportFormat::Html, ..)` returns
  `Err(ExportError::NoDisplayRendering)`
  (`crates/eggsec-gui/src/export.rs:337-347`): **the HTML format is
  export-only**. The five text formats keep their inert-text display rendering.
- The exported HTML is still the CLI's document byte for byte
  (`convert_to_html`), remote chart-library reference and all, because the
  exported file is the operator's artifact, opened in the operator's browser.

## Known gaps

- **The GUI manages the daemon's lifecycle only when asked.** By default it
  never spawns or supervises a daemon. With `--daemon-bin <PATH>` it starts
  that executable and stops it on exit.
- **The report is shown as data, never as the engine's HTML document.** See
  [Report export and display](#report-export-and-display). The CSP is not
  relaxed: the report's content is attacker-influenced, and the CSP is the
  control that stops it acting. The exported HTML file is still the CLI's
  document byte for byte (`convert_to_html`); only the *display* path differs.
- **The socket-path default is duplicated in six places** — the CLI's
  `--socket` (`crates/eggsec/src/cli/mod.rs:249`), the daemon's
  `DaemonConfig::default` (`crates/eggsec-daemon/src/config.rs:22`), the daemon
  binary's `--socket-path` (`crates/eggsec-daemon/src/main.rs:20`), the GUI's
  own Rust default (`crates/eggsec-gui/src/config.rs:19`), and twice in the
  GUI's frontend (`crates/eggsec-gui/frontend/app.js:23`,
  `crates/eggsec-gui/frontend/index.html:25`). There is no socket-path
  environment variable. Collapsing the duplication, and introducing one,
  belongs to a corrective plan.
- **Binary distribution, signing, notarization, and installers are out of
  scope** — handled separately.

## Resources

- `plans/adrs/ADR-0007-desktop-gui-frontend-boundary.md` — the crate boundary.
- `plans/adrs/ADR-0009-gui-shell-technology-selection.md` — the shell choice,
  the three conditions, and the egui fallback.
- `plans/subsystems/desktop-gui-frontend-roadmap.md` — the milestone status.
- `architecture/frontend_client.md` — the shared runtime-client contract.
- `architecture/daemon.md` — the daemon side of the protocol.
- `architecture/ui_model.md` — the view DTOs and the renderer registry.
- `.opencode/skills/eggsec-frontend-client/SKILL.md`,
  `.opencode/skills/eggsec-daemon/SKILL.md` — the two seams the GUI sits on.
