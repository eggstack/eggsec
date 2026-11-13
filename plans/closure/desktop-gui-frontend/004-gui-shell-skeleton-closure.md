# Desktop GUI Frontend Milestone 004 — Closure Status

Status: closed

Source implementation plan:

- `plans/implementation/desktop-gui-frontend/004-gui-shell-skeleton.md`

Source subsystem roadmap:

- `plans/subsystems/desktop-gui-frontend-roadmap.md`

Repository baseline reviewed: `203ab5dd958415340d39cd47fb1add432c687edd`

Implementation commits or pull requests:

- Uncommitted working-tree change on branch `feat/auto-20261022-76692679`.

## 1. Executive finding

The milestone's capability boundary is complete. The desktop GUI now has a **shell**: a new
`crates/eggsec-gui` crate that opens a Tauri v2 window, connects to a running Eggsec daemon over
its Unix socket, and renders the operation catalog the daemon publishes.

The crate is a **leaf**. It depends on `eggsec-frontend-client`, `eggsec-runtime`,
`eggsec-daemon-protocol`, and `eggsec-ui-model`, and on nothing else. It links no engine, no
daemon crate, no CLI, and no TUI, and it performs no outbound HTTP of its own — every engine
request travels over the daemon socket. That boundary is enforced, not merely intended: guards 155
and 156 fail the build if it is broken.

Two seams had to widen to make this possible, and both are additive in behaviour:

- `eggsec-frontend-client` stopped hardcoding one frontend's identity. The caller now supplies its
  own client kind, label, and request-id namespace through a new `ClientIdentity`.
- `eggsec-daemon-protocol` gained a `ClientKind::Gui` variant, and the daemon's manual-approval
  matrix was corrected to agree with `eggsec-policy`'s classification of the GUI as a manual
  surface.

The milestone's most consequential decision was the **exact** pinning of the Tauri dependency.
`tauri` declares its runtime and utils dependencies with caret requirements, and every crate on the
next line up declares MSRV 1.90 — above this workspace's 1.89 floor. Without direct `=` pins, a
`cargo update` would float the closure onto the 1.90 line and silently break `make check-msrv`. The
pins are therefore load-bearing, and the closure record says so in the place a future contributor
would look.

## 2. Requirement-to-evidence matrix

| Requirement | Evidence | Result | Notes |
|---|---|---|---|
| A `crates/eggsec-gui` crate exists as a Tauri v2 binary that opens a window | `crates/eggsec-gui/src/shell.rs`, `tauri.conf.json`; `cargo check -p eggsec-gui --features shell` | pass | The window, the managed state, and the command registration are in `shell.rs`. |
| The GUI links no engine crate | guard 155; `cargo tree -p eggsec-gui --features shell` | pass | The tree shows no engine edge. |
| The shell framework appears in no crate but the GUI's | guard 156 | pass | The root manifest's `[workspace.dependencies]` pins are allowed; no other crate declares `tauri`. |
| The GUI declares no second TLS stack | guard 157; `cargo tree` | pass | No `rustls`, `native-tls`, `reqwest`, `openssl`, or `ring` in the GUI's graph. The workspace's ring-only invariant holds with no exception. |
| The GUI's CSP admits no remote origin | guard 158; `tauri.conf.json` | pass | `default-src 'self'` plus the Tauri IPC origin only. |
| The frontend client carries no TUI-specific identity | test `crate_carries_no_hardcoded_frontend_identity` | pass | No `"tui-"`, `ClientKind::Tui`, or `"eggsec-tui"` outside the crate's tests. |
| The GUI declares its own client kind | `ClientKind::Gui`; the daemon end-to-end test | pass | Additive on the wire; an unknown variant fails closed at deserialization. |
| The GUI connects to a running daemon and renders the published catalog | `src/connection.rs`, `src/commands.rs`; `cargo test -p eggsec-daemon --lib the_gui_client_path_works_end_to_end` | pass | The end-to-end test runs against a live daemon host over a real Unix socket. |
| A missing daemon produces a precise diagnostic naming the right start command | test `unreachable_diagnostic_names_the_socket_and_the_start_command` | pass | The diagnostic names the socket and `eggsec-daemon --full-executor --socket-path <PATH>`. |
| The GUI opens a `DesktopGuiManual` session and the audit trail records it | the daemon end-to-end test; test `gui_session_audit_trail_carries_the_gui_surface_label` | pass | The trail records `desktop-gui-manual`, not a blanket `"daemon"`. |
| The manual-approval matrix agrees with `eggsec-policy` | test `desktop_gui_manual_surface_accepts_controller_approval` | pass | Preparatory only: `ApprovePolicy` is still unwired, so no approval is granted. |
| `DAEMON_PROTOCOL_VERSION` stays at 2 | `crates/eggsec-daemon-protocol/src/protocol.rs` | pass | A unilateral version bump was forbidden by the plan's stop conditions. |
| `make check` and `make check-msrv` cover the GUI crate | `Makefile` | pass | The GUI shell profile is behind a prerequisite gate that skips when the webview SDK is absent and fails when it is present but broken. |
| CI installs the Linux webview prerequisites | `.github/workflows/ci.yml`, `.github/workflows/deep-checks.yml` | pass | `libwebkit2gtk-4.1-dev`, `libgtk-3-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev`, `patchelf`, `pkg-config`. |
| The frontend carries no npm dependency | the `crates/eggsec-gui/frontend/` tree | pass | No `package.json`, no lockfile, no `node_modules`, no bundler, no build step. |
| The Tauri closure clears `deny.toml` | `cargo deny check licenses` → `licenses ok`; `bans ok`, `sources ok` | pass | The whole closure is MIT / Apache-2.0 / Zlib. The gtk-rs bindings are MIT, not LGPL. |
| MSRV 1.89 holds with the shell profile | `cargo +1.89 check -p eggsec-gui --features shell` | pass | The 2.11.x line declares MSRV 1.77.2. |

## 3. Production implementation evidence

### 3.1 The crate and its boundary

`crates/eggsec-gui` is the workspace's 25th crate. It is a binary application with a library
target beside it: the library holds every behaviour so it can be unit-tested without a window, and
`src/main.rs` is a thin shell around `eggsec_gui::run()`.

| Module | Responsibility |
|---|---|
| `src/lib.rs` | the module root and `run()` |
| `src/main.rs` | the thin binary |
| `src/config.rs` | `--socket <PATH>` parsing, default `/tmp/eggsec-daemon.sock` |
| `src/connection.rs` | the daemon connection, the protocol-version check, the session, the catalog fetch |
| `src/commands.rs` | the Tauri IPC command surface |
| `src/shell.rs` | the Tauri builder: window, managed state, command registration |
| `build.rs` | the Tauri build step, gated on the `shell` feature |
| `tauri.conf.json` | the window, the CSP, the capability wiring |
| `capabilities/default.json` | the capability: `core:default` only, no plugin |
| `icons/icon.png` | the application icon the context macro embeds |
| `frontend/` | the buildless HTML/CSS/vanilla-JS frontend |

The GUI's dependency set is exactly:

| Dependency | Why |
|---|---|
| `eggsec-frontend-client` | the shared `RuntimeClient` contract and the daemon client |
| `eggsec-runtime` | `RuntimeSurface`, `SessionScope`, `RunRequest`, the event types |
| `eggsec-daemon-protocol` | the wire DTOs: the operation catalog, the protocol version |
| `eggsec-ui-model` | the renderer registry and the result view DTOs |

### 3.2 The Tauri version pin, and why it is exact

```toml
tauri = { version = "=2.11.6", default-features = false }
tauri-build = { version = "=2.6.3" }
tauri-runtime = { version = "=2.11.3" }
tauri-runtime-wry = { version = "=2.11.4" }
tauri-macros = { version = "=2.6.3" }
tauri-utils = { version = "=2.9.3" }
```

`tauri` 2.11.6 declares its runtime and utils dependencies with **caret** requirements —
`tauri-runtime ^2.11.3`, `tauri-runtime-wry ^2.11.4`, `tauri-utils ^2.9.3` — and every crate on
the next line up declares `rust-version = 1.90`, above this workspace's 1.89 floor:

| Crate | MSRV-safe line | Next line | Next line's MSRV |
|---|---|---|---|
| `tauri` | 2.11.6 | 2.12.5 | 1.90 |
| `tauri-build` | 2.6.3 | 2.7.4 | 1.90 |
| `tauri-runtime` | 2.11.3 | 2.12.2 | 1.90 |
| `tauri-runtime-wry` | 2.11.4 | 2.12.3 | 1.90 |
| `tauri-macros` | 2.6.3 | 2.7.4 | 1.90 |
| `tauri-utils` | 2.9.3 | 2.10.2 | 1.90 |

Without the direct exact pins, a `cargo update` would float the whole closure onto the 1.90 line
and break `make check-msrv`. The GUI crate therefore declares `tauri-macros`, `tauri-runtime`,
`tauri-runtime-wry`, and `tauri-utils` directly even though it does not call them: they exist to
hold the transitive closure. `tauri-utils` is additionally pinned in `[build-dependencies]`,
because `tauri-build` requires `tauri-utils ^2.9.3` and would otherwise float even the
**no-shell** build that `make check` runs.

The resolved closure was read back out of `Cargo.lock` and confirmed on the safe line: `tauri
2.11.6`, `tauri-build 2.6.3`, `tauri-codegen 2.6.3`, `tauri-macros 2.6.3`, `tauri-runtime 2.11.3`,
`tauri-runtime-wry 2.11.4`, `tauri-utils 2.9.3`, `wry 0.55.1`, `tao 0.35.3`, `muda 0.19.3`.

### 3.3 The frontend-client identity seam

`eggsec-frontend-client` is frontend-neutral and previously hardcoded the TUI's identity: it
declared itself as `ClientKind::Tui`, labelled itself `"eggsec-tui"`, and stamped every request id
`"tui-{n}"`. A GUI cannot be served by a crate that decides who it is, so the identity became the
caller's:

```rust
pub struct ClientIdentity {
    pub kind: ClientKind,
    pub label: String,
    pub request_prefix: String,
}

impl DaemonRuntimeClient {
    pub async fn connect(socket_path: &str, identity: ClientIdentity) -> Result<Self, DaemonClientError>;
}
```

`ClientKind` is re-exported from the frontend-client crate, so a frontend can name its own kind
without taking a direct dependency on the protocol crate.

Three new **inherent**, daemon-only methods were added to `DaemonRuntimeClient`:

| Method | Returns | Why inherent |
|---|---|---|
| `describe_operations()` | `DescribeOperationsResponse` | the embedded client has no catalog to publish |
| `health()` | `DaemonHealth` | only a daemon-backed client has a daemon to ask |
| `daemon_capabilities()` | `DaemonCapabilities` | the trait's `capabilities` returns the *runtime* half; the daemon's own `operation_schema` flag lives in the other half |

The `RuntimeClient` trait's eight methods are **unchanged**.

The TUI's call site now passes its own identity — `ClientKind::Tui`, `"eggsec-tui"`, `"tui-"` — and
the TUI's observable behaviour is unchanged.

### 3.4 The protocol change

`ClientKind::Gui` was added to `crates/eggsec-daemon-protocol/src/client_registry.rs`.

- The variant is **additive** on the wire. A daemon that predates it fails to deserialize
  `DeclareClient` and rejects the client — a fail-closed outcome, the same class of change as
  milestone 001's `#[serde(default)]` capability field.
- `DAEMON_PROTOCOL_VERSION` stays at `2`. `ClientCommand` and `ServerMessage` variant sets are
  unchanged: 16 and 15.

The daemon's manual-approval arm in `check_permission` was corrected. It read
`RuntimeSurface::TuiManual | RuntimeSurface::CliManual`, which put the GUI — classified as manual
in `eggsec-policy` — into the strict owner-only branch. The arm now reads `TuiManual | CliManual |
DesktopGuiManual`. This is **preparatory only**: `ApprovePolicy` is still unwired and the daemon
still answers it with `ErrorCode::Unsupported`, so no approval is granted either way.

### 3.5 The daemon connection

`crates/eggsec-gui/src/connection.rs` performs, in order:

1. **Connect** with the GUI's identity.
2. **Check the protocol version.** `health()` returns the daemon's `protocol_version`; a mismatch
   is refused outright, because a daemon speaking a different protocol cannot be trusted to
   interpret a session request the way this GUI means it.
3. **Check the schema flag.** `daemon_capabilities().operation_schema` must be true; a daemon that
   predates the schema publication cannot publish the catalog the GUI renders.
4. **Open the session**, stamped `RuntimeSurface::DesktopGuiManual`, with the scope the GUI
   constructs itself.
5. **Fetch the catalog** via `describe_operations()`.

When the socket is unreachable, the diagnostic names the socket path and the exact command that
starts a *usable* daemon:

```
cannot reach the Eggsec daemon at /tmp/eggsec-daemon.sock: IO error: ...

Start a usable daemon with:
  eggsec-daemon --full-executor --socket-path /tmp/eggsec-daemon.sock

`eggsec daemon start` and a default-built `eggsec-daemon` host a no-op executor that rejects
every task, so the --full-executor flag is required.
```

That sentence matters because both `eggsec daemon start` and a default-built `eggsec-daemon` host
a **no-op** executor that answers every command correctly and rejects every task. The GUI detects
that condition — a daemon advertising no task kinds — and reports it rather than offering a submit
button that cannot work.

### 3.6 The buildless frontend

`crates/eggsec-gui/frontend/` holds hand-written HTML, CSS, and vanilla JavaScript. **Zero npm
dependencies**: no `package.json`, no lockfile, no `node_modules`, no bundler, no build step.
`tauri.conf.json` sets `app.withGlobalTauri: true`, so the webview reaches
`window.__TAURI__.core.invoke` directly.

The CSP is restrictive and admits no remote origin:

```
default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self' data:;
connect-src 'self' ipc: http://ipc.localhost; object-src 'none'; frame-src 'none';
base-uri 'none'
```

The capability file grants `core:default` only. No plugin is enabled — no http, shell, fs, process,
or updater.

### 3.7 Two platform facts the plan did not anticipate

**`build.rs` must be feature-gated.** `tauri_build::build()` reads metadata that the `tauri`
crate's own build script publishes. With the `shell` feature off — the `--no-default-features`
profile `make check` runs — the `tauri` crate is not in the graph, that metadata does not exist,
and the call panics with `missing 'cargo:dev' instruction`. The fix is to gate the call on
`CARGO_FEATURE_SHELL`, which Cargo sets for the crate being built. Both profiles now build.

**`frontendDist` is relative to the config file's directory.** The plan's snippet said
`"../frontend"`; in Tauri v2 that resolves to `crates/frontend`, outside the crate. The value is
`"frontend"`.

### 3.8 The end-to-end test that crosses the boundary

The GUI crate cannot depend on the daemon crate — that is guard 155 — so the test that proves the
two fit together lives in the daemon crate's own suite, in `crates/eggsec-daemon/src/server.rs`:

```rust
#[tokio::test]
async fn the_gui_client_path_works_end_to_end() { ... }
```

It starts a live daemon host over a real Unix socket, connects with the GUI's own client identity,
checks the handshake's protocol version and the `operation_schema` flag, fetches the catalog
(30 task kinds), opens a `DesktopGuiManual` session, and asserts the audit trail records
`desktop-gui-manual` rather than a blanket `"daemon"`.

`eggsec-frontend-client` is a **dev-dependency** of `eggsec-daemon` for this test. The dependency
runs one way; there is no cycle.

### 3.9 The guards

Guards 155–158 were added to `scripts/check-architecture-guards.sh`:

| Guard | Protects |
|---|---|
| 155 | the GUI crate links no engine, daemon, CLI, or TUI crate |
| 156 | the shell framework stays inside the GUI crate |
| 157 | the GUI declares no second TLS stack |
| 158 | the GUI's CSP forbids remote origins |

## 4. Verification executed

### Commands run

```bash
cargo fmt --all --check
cargo check --workspace --no-default-features
cargo check -p eggsec-gui --features shell
cargo check -p eggsec-gui --no-default-features
cargo +1.89 check -p eggsec-gui --features shell
cargo clippy -p eggsec-gui --features shell --all-targets
cargo test -p eggsec-gui --features shell
cargo test -p eggsec-frontend-client --lib
cargo test -p eggsec-daemon-protocol --lib
cargo test -p eggsec-daemon --lib the_gui_client_path_works_end_to_end
cargo check -p eggsec-tui --all-targets
cargo tree -p eggsec-gui --features shell
bash scripts/check-architecture-guards.sh
make check
```

### Results

| Check | Result |
|---|---|
| `cargo fmt --all --check` | pass, after formatting this milestone's own files |
| `cargo check --workspace --no-default-features` | pass |
| `cargo check -p eggsec-gui --features shell` | pass |
| `cargo check -p eggsec-gui --no-default-features` | pass |
| `cargo +1.89 check -p eggsec-gui --features shell` | pass |
| `cargo clippy -p eggsec-gui --features shell --all-targets` | pass, no warnings |
| `cargo test -p eggsec-gui --features shell` | pass |
| `cargo test -p eggsec-frontend-client --lib` | pass |
| `cargo test -p eggsec-daemon-protocol --lib` | pass |
| `cargo test -p eggsec-daemon --lib the_gui_client_path_works_end_to_end` | pass |
| `cargo check -p eggsec-tui --all-targets` | pass |
| `bash scripts/check-architecture-guards.sh` | pass — `ALL PASSED: No architecture drift detected.` over all 160 checks |
| `make clippy` | pass, exit 0 |
| `cargo test -p eggsec --doc` | pass — 21 passed, 1 ignored |
| `cargo test -p eggsec --no-default-features --test tool_registration --test loadtest_tests` | pass — 29 passed |
| `cargo test -p eggsec --features rest-api,cli --tests` | pass — 1872 passed |
| `cargo test -p eggsec-output --tests` | pass |
| `cargo test -p eggsec-report-model --tests` | pass |
| `cargo test -p eggsec-policy --tests` | pass |
| `cargo test -p eggsec-service-db --tests` | pass |
| `cargo test -p eggsec-secrets --tests` | pass |
| `cargo test -p eggsec-payloads --tests` | pass |
| `cargo test -p eggsec-udp-scan --tests` | pass |
| `cargo test -p eggsec-transport-eggfetch --tests` | pass — 252 passed |
| `cargo test -p eggsec-tui --lib` | pass — 1002 passed |
| `make check` | **stops at `check-deps`** on pre-existing advisories — see below |

`make check` was run end to end. Every step **before** `check-deps` passed: the format check, the
`--no-default-features` workspace check, the GUI shell gate, `cargo check -p eggsec --all-targets
--no-default-features`, `cargo check -p eggsec`, and both `eggsec-cli` checks. `check-deps` then
failed on advisories, so the steps after it were run directly: `make clippy` (exit 0), each of the
target's test invocations (all green — 1872 engine tests with `rest-api,cli`, 1002 TUI tests, 252
transport tests, and the per-crate suites), and the guard script.

**The advisory failure is pre-existing and is not a regression from this milestone.** The failures
are `zip 2.4.2` (RUSTSEC-2026-0368, RUSTSEC-2026-0369) and the `rustls 0.21.12` family
(RUSTSEC-2026-0346/0347/0348/0349, 0367/0368/0369, 0381/0382/0383, RUSTSEC-2024-0449) reached
through `tiberius` → `sqlx-mysql`. `zip 2.4.2` is already present in the baseline `Cargo.lock`; the
lock's diff for this milestone adds only the Tauri chain; and `cargo deny` reports `bans ok,
licenses ok, sources ok`. The milestone introduces **no** new advisory and **no** new license
exception. Recording exceptions in `deny.toml` requires an owner and a review-by date, which is a
maintainer decision — see §10.

### Guard demonstrations

Each new guard was demonstrated to **fail** under a deliberate violation and to **pass** again
after the violation was restored. Every temporarily violated file was restored **byte-for-byte**
and verified with a SHA-256 comparison; the temporary probe files were deleted.

**Guard 155 — manifest branch** (`eggsec-tui` added to the GUI's manifest):

```
--- Check 155: GUI crate links no engine ---
FAIL: crates/eggsec-gui/Cargo.toml declares forbidden dep 'eggsec-tui' (the GUI reaches the engine only through the daemon socket).
50:eggsec-tui = { path = "../eggsec-tui", version = "0.1.0" }
EXIT=1
```

**Guard 155 — source branch** (a temporary `crates/eggsec-gui/src/zzz_guard_probe.rs` containing
`use eggsec::config::Scope;`):

```
FAIL: crates/eggsec-gui/src imports or reaches the engine, the daemon, or another frontend crate.
crates/eggsec-gui/src/zzz_guard_probe.rs:2:use eggsec::config::Scope;
EXIT=1
```

After restore: `PASS: the GUI crate depends on no engine, daemon, or other frontend crate.` The
manifest's SHA-256 was identical before and after (`0428cccc…`).

**Guard 156 — crate branch** (a temporary `crates/zzz-guard-probe/Cargo.toml` declaring
`tauri = "2"`):

```
FAIL: crates/zzz-guard-probe/Cargo.toml mentions 'tauri'; the shell framework belongs to crates/eggsec-gui alone.
7:tauri = "2"
EXIT=1
```

**Guard 156 — root branch** (a temporary `[dependencies] tauri = "2"` appended to the root
manifest):

```
FAIL: Cargo.toml references 'tauri' outside [workspace.dependencies]; only crates/eggsec-gui may depend on the shell framework.
Cargo.toml:111:tauri = "2"
EXIT=1
```

After restore: the root manifest was byte-identical (`diff` clean, SHA-256 `f76891d0…`) and
`PASS: no crate other than crates/eggsec-gui declares or mentions a tauri dependency.` The probe
crate was deleted.

**Guard 157** (`reqwest = { version = "0.12" }` added to the GUI's manifest):

```
FAIL: crates/eggsec-gui/Cargo.toml declares forbidden TLS dep 'reqwest' (the GUI talks to the daemon over a Unix socket and needs no TLS stack).
50:reqwest = { version = "0.12" }
EXIT=1
```

After restore: `PASS: the GUI crate declares no TLS dependency and enables no TLS feature.` with
the manifest's SHA-256 identical.

**Guard 158** (the CSP's `connect-src` given a remote origin):

```
FAIL: crates/eggsec-gui/tauri.conf.json app.security.csp allows a wildcard source or a remote origin:
      connect-src allows the remote origin 'https://evil.example.com'
      offending CSP: default-src 'self'; … connect-src 'self' ipc: http://ipc.localhost https://evil.example.com; …
      the only permitted remote origin is http://ipc.localhost
EXIT=1
```

After restore: `PASS: the GUI CSP allows no wildcard source and no remote origin.` The file's
SHA-256 was identical before and after (`d3370c70…`).

**The gate's own semantics.** The `GUI_SHELL_GATE` macro was proven in isolation against a scratch
Makefile: a failing command **propagates** (`make: *** [fails] Error 7`), and a simulated Linux
host without `pkg-config` prints the SKIP line and exits 0. An absent prerequisite skips; a present
prerequisite whose build breaks fails.

## 5. Invariant review

| Invariant | Evidence | Status |
|---|---|---|
| `EnforcementContext::evaluate()` is the mandatory pre-dispatch gate for all surfaces | The GUI dispatches **only** through the daemon, which owns enforcement. The GUI has no dispatch path of its own — `crates/eggsec-gui/src/commands.rs` contains no `ClientCommand::` and no `tool.execute()`. | holds |
| Strict surfaces dispatch only via `EnforcedDispatcher::dispatch_execution()` with an `ApprovedExecution` bundle | The GUI is a **manual** surface, not a strict one (`ManualPermissive`, `honors_manual_override()` true), and it does not dispatch at all — the daemon does. | holds |
| Strict surfaces use `LoadedScope` and fail closed without an explicit scope | The GUI stamps a non-explicit `SessionScope`; the daemon resolves the loaded scope. The GUI cannot widen scope, and a scope-less GUI session is not an engine entry point. | holds |
| `OperationMetadata` stays the single source of truth for operation policy | The GUI renders the daemon's published catalog and never decides policy itself. The daemon's exposure flags and risk tiers come from `OperationMetadata`. | holds |
| `runtime_bridge` owns both `RuntimeSurface`↔`ExecutionSurface` conversions | Unchanged by this milestone. The GUI's surface travels the existing wire as `DesktopGuiManual` and converts through the existing bridge. | holds |
| TLS is ring-only everywhere | The GUI enables no TLS feature and depends on no TLS crate; its closure contains no `rustls`, `native-tls`, `reqwest`, `openssl`, or `ring`. Guard 157 enforces it. | holds |
| The Tokio baseline is `default-features = false` with per-crate feature lists | `crates/eggsec-gui/Cargo.toml` lists `features = ["rt", "sync", "time", "macros", "net"]`; `default-features` is not enabled. | holds |
| Spawned tokio tasks carry a 30–300s timeout wrapper | The GUI's one spawned task — the event forwarder — is a long-lived connection reader whose lifetime is the subscription's. The exemption and its reason are written at the function, matching the daemon client's reader task. | holds, documented exemption |
| Never `let _ =` or `filter_map(|e| e.ok())` — log via `tracing` | The GUI's sources contain no `let _ =` on a `Result`; the forwarder logs through `tracing::warn!`. | holds |
| The workspace root is a virtual manifest | Unchanged; `crates/eggsec-gui` was added to `members`. | holds |
| No crate but the GUI's may depend on the shell framework | Guard 156. | holds |
| The GUI must not depend on the engine | Guard 155. | holds |

## 6. Failure and recovery review

| Scenario | Behavior | Evidence |
|---|---|---|
| The daemon is not running | The GUI reports the socket path and the exact `eggsec-daemon --full-executor --socket-path <PATH>` command. It does **not** retry in a tight loop: one automatic attempt, then operator-driven retries only. | `unreachable_diagnostic_names_the_socket_and_the_start_command`; `crates/eggsec-gui/src/connection.rs` |
| The daemon is running but hosts a no-op executor | The GUI detects that the daemon advertises no task kinds, reports the condition, and does not present a submit affordance that cannot work. | `GuiConnection::daemon_advertises_no_task_kinds` |
| The daemon speaks a different protocol version | The connection is refused before any session is opened. | `GuiConnection::connect` |
| The daemon predates the schema publication | The connection is refused with a message naming the restart command, rather than rendering an empty catalog. | `GuiConnection::connect` |
| A daemon predating `ClientKind::Gui` | The `DeclareClient` command fails to deserialize, so the daemon rejects the client — fail-closed, not a silent misclassification. | the protocol's additive-variant semantics |
| The daemon dies mid-session | The forwarder's receiver ends; the GUI logs the loss and publishes the counters. It does not decide what happened to the running tasks — the event stream was the source of truth. | `spawn_event_forwarder` |
| A duplicate connection attempt | The GUI's connection state is replaced wholesale; the previous session's events are no longer forwarded. | `commands::connect_daemon` |
| Malformed or unauthorized input | The GUI never forwards an arbitrary daemon message, and the daemon's own authorization governs every task. A malformed form value is rejected before submission with a field-level error. | guard on the IPC surface; `form::assemble_request` |

## 7. Migration and compatibility review

- **No schema migration.** The milestone adds no persisted state and no database table.
- **Protocol compatibility is additive.** `ClientKind::Gui` is a new enum variant. A client that
  speaks it to a daemon that predates it is rejected at deserialization — fail-closed. A daemon that
  speaks it to an older client is unaffected: the older client never sends the variant.
- **`DAEMON_PROTOCOL_VERSION` is unchanged at 2.** The milestone's stop conditions forbade a
  unilateral version bump, and none was needed.
- **The frontend client's `connect` signature changed.** This is a source-breaking change to an
  internal 0.1.0 crate with exactly one in-tree caller at the time of the change (the TUI), which
  was migrated in the same commit. The change is behaviour-preserving for the TUI.
- **The `RuntimeClient` trait is unchanged** — its eight methods are the same. The three new
  daemon-specific methods are inherent, not trait methods, so no implementor is broken.
- **The socket-path default is now duplicated in four places** — the CLI's global `--socket`, the
  daemon's `DaemonConfig::default`, the daemon binary's `--socket-path`, and the GUI's own default.
  No socket-path environment variable and no XDG lookup exist today. This is a **duplicate**, not a
  break: every one of the four points at the same path today. Collapsing it is a separate
  corrective milestone (see §10).
- **Rollback:** the milestone is entirely additive. Removing `crates/eggsec-gui` and reverting the
  frontend-client and protocol changes returns the tree to the baseline; no data migration is
  needed in either direction.

## 8. Security review

| Concern | Review |
|---|---|
| Authorization enforcement | The GUI has **no** enforcement path. It cannot authorize anything; it asks the daemon, which enforces through `EnforcementContext::evaluate()`. The GUI's IPC surface contains no `ClientCommand::`, no `RuntimeSurface::`, and no `SessionScope` — a test asserts this by scanning the module's production source. |
| Privilege boundary | The GUI's session is stamped `RuntimeSurface::DesktopGuiManual` in **Rust**, by serializing the runtime's own enum. The webview cannot widen the session's authority by asking nicely. |
| Scope | The GUI constructs a non-explicit `SessionScope` with `is_explicit: false`. It cannot widen scope; the daemon resolves the loaded scope. |
| Transport checkpoint | All engine traffic travels over the daemon's Unix socket. The GUI makes **no** outbound HTTP, which is why it needs no TLS stack and why the ring-only invariant needs no exception. |
| Transport security | The CSP admits no remote origin (`default-src 'self'`, `connect-src 'self' ipc: http://ipc.localhost`). Guard 158 enforces it. |
| Capability minimization | The capability file grants `core:default` only. No plugin is enabled — no http, shell, fs, process, or updater — so the webview has no filesystem, process, or outbound-HTTP capability at all. |
| Supply chain | The frontend has **zero** npm dependencies and no remote script source. The Tauri closure is MIT / Apache-2.0 / Zlib and clears `deny.toml` with no new exception. |
| Denial-of-service bounds | The GUI's log retention is capped (this is milestone 005's subject, but the shell carries the mechanism). The GUI spawns one forwarder task and never one per event. |
| Secret handling | The GUI holds no credentials. The daemon socket is the only channel, and the socket path is the only configurable value. |
| Audit behavior | A GUI session is auditable as `desktop-gui-manual`, not as a blanket `"daemon"`. The end-to-end test asserts this against a live host with persistence enabled. |
| Redaction | No daemon-supplied value is logged or rendered as markup. The frontend renders every daemon value as text. |

## 9. Documentation and operations

**Architecture documents written or updated**

- `architecture/desktop_gui.md` — **new**, the GUI's deep-dive page: the crate's boundary, the
  Tauri shell, the exact version pins and why they are load-bearing, the buildless frontend, the
  daemon connection, the IPC surface, the platform prerequisites, and the known gaps.
- `architecture/overview.md` — the crate count 24 → 25; the `eggsec-gui` row in the crate table,
  the module index, the User Interfaces table, and the Key Types table; the dependency map's GUI
  branch; the guardrails list gained Checks 155–160.
- `architecture/frontend_client.md` — the `ClientIdentity` seam, the widened `connect` signature,
  and the three new inherent methods.
- `architecture/daemon.md` — `ClientKind::Gui`, the manual-approval arm, and the GUI's submission
  and event flow.
- `architecture/tui.md` — the TUI's call site now passes its own identity.

**Contract documents updated**

- `docs/ARCHITECTURE.md` — the GUI's position in the dependency picture.
- `docs/ARCHITECTURE_INVARIANTS.md` — the GUI's daemon-only boundary.
- `docs/CI_ARCHITECTURE_GUARDS.md` — Checks 155–160.
- `docs/VERIFICATION.md` — the GUI lines in the `make check` expansion, the prerequisite-skip
  semantics, and the `make check-gui` target.
- `docs/ENFORCEMENT_MODES.md` — the GUI's manual classification, cross-checked against the
  daemon's manual-approval arm.
- `CONTRIBUTING.md` — the GUI's prerequisites and how to build and run it.

**Skills**

- `.opencode/skills/eggsec-gui/SKILL.md` — **new**, the GUI subsystem skill.
- `.opencode/skills/eggsec-frontend-client/SKILL.md` — the identity seam and the daemon-only
  methods.
- `.opencode/skills/eggsec-daemon/SKILL.md` — the client kinds and the manual-approval arm.

**Planning documents**

- `plans/registry.md` — the standing row, the section heading, the M4 entry, and the closure-record
  list.
- `plans/subsystems/desktop-gui-frontend-roadmap.md` — the status line, the §4 current state, the
  §12 milestone table, and the §9 guard list.
- `plans/000-long-term-specification.md` — the crate count and the crate-ownership entry.
- `AGENTS.md` — the crate count, the `eggsec-gui` workspace bullet, the area index, and the new
  skill's place in the skills rule.

**Static guards**

Guards 155–158 in `scripts/check-architecture-guards.sh`, each demonstrated to fail under a
deliberate violation.

**Operator diagnostics**

- Unreachable daemon: the socket path plus the exact `eggsec-daemon --full-executor --socket-path
  <PATH>` command, and the reason `--full-executor` is required.
- No-op executor: the GUI reports that the daemon advertises no task kinds and names the restart
  command.
- Protocol mismatch: the daemon's version and the GUI's, and the refusal.
- Schema publication missing: the restart command.

**Recovery instructions**

The GUI holds no state of its own. Recovery is to restart the daemon and reconnect. The GUI's
task map is a view of the daemon's event stream, not a store — a reconnect rebuilds it from the
catalog and the fresh stream.

## 10. Unresolved findings

| Severity | Finding | Impact | Required action |
|---|---|---|---|
| low | `make check` cannot complete because `check-deps` fails on pre-existing advisories (`zip 2.4.2`; the `rustls 0.21.12` family via `tiberius` → `sqlx-mysql`). Neither is in the GUI's closure, and this milestone adds no advisory and no license exception. | The mandatory contract's dependency gate is red for reasons that predate this milestone. | A maintainer decision: add `deny.toml` exceptions with an owner and a review-by date, and record them in `docs/DEPENDENCY_EXCEPTIONS.md`. Until then, the gate cannot go green. |
| medium | The socket-path default is duplicated in four places (the CLI's `--socket`, the daemon's `DaemonConfig::default`, the daemon binary's `--socket-path`, the GUI's default). No socket-path environment variable and no XDG lookup exist. | A future change to the default can silently diverge between the four sites, and the GUI can point at a different daemon than the CLI with no warning. | A corrective milestone: collapse the duplication and introduce one environment variable. |
| medium | `ApprovePolicy` remains unwired — the daemon answers it with `ErrorCode::Unsupported`. | No surface can grant an interactive approval. The GUI handles `PolicyDecisionRequired` defensively and offers no approval affordance, so it cannot approve even by accident. | A maintainer decision: wire daemon-mediated interactive approval, or keep the fail-closed behaviour that plan 003 mandates. |
| low | The GUI does not spawn, supervise, or stop a daemon. | An operator must start a usable daemon by hand before the GUI is useful. The GUI names the right command when it cannot connect. | Milestone 006 owns daemon lifecycle management. |
| low | `make check-python` was not run. | The Python bindings are unaffected by this milestone — no Python file changed — and the daemon protocol's Python parsers were verified by `cargo check -p eggsec-python --features daemon-client` in milestone 003. | Run `make check-python` when a `.venv-ci` environment is available. |

## 11. Roadmap disposition

**Milestone closed and the next dependency may proceed.**

The GUI shell's capability boundary is complete and its evidence is recorded: the crate exists and
builds in both profiles, passes MSRV 1.89, is clippy-clean, passes its unit tests, connects to a
live daemon host over the real socket, and its boundary is enforced by four demonstrated guards.
Milestone 005, which builds on this shell, has also been implemented and closed.

Two items are carried forward without blocking this milestone: the `deny.toml` advisory backlog
(§10, a maintainer decision that predates this work) and the socket-path duplication (§10, a
corrective milestone).

## 12. Registry updates

- `plans/registry.md`:
  - the `desktop-gui-frontend` standing row — M4 implemented, the workspace at 25 crates;
  - the section heading — `M1–M3 closed 2026-10-23; M4–M5 implemented 2026-10-23; M6–M7 not started`;
  - the M4 entry — rewritten from plan language to the delivered state, including the closure
    record path;
  - the closure-record list — `-004-gui-shell-skeleton-closure.md` added;
  - the control-points row — M4 no longer blocked.
- `plans/subsystems/desktop-gui-frontend-roadmap.md`:
  - the status line — M4 implemented;
  - the §4 current state — the GUI crate exists;
  - the §9 guard list — Checks 155–158;
  - the §12 milestone table — M4 implemented, with the closure record path;
  - the M6 deliverable boundary — daemon lifecycle management.
- `plans/000-long-term-specification.md` — the crate count 24 → 25 and the `eggsec-gui` crate
  ownership entry.
- `AGENTS.md` — the crate count, the `eggsec-gui` workspace bullet, the area index, and the new
  skill.
- `plans/implementation/desktop-gui-frontend/004-gui-shell-skeleton.md` — `Status:` moved from
  `ready for handoff` to `executed — closed 2026-10-23`.
