# Desktop GUI Frontend Milestone 009 — Daemon Obtainability and Start Honesty

Status: ready for handoff

Repository baseline: branch `feat/auto-20261022-76692679` at commit `e1916069`
("docs(plans): register the desktop GUI milestones 006 and 007"), with
milestones 001–003 closed, 004–005 implemented, and 006–007 planned and ready
for handoff.

Source roadmap:

- `plans/subsystems/desktop-gui-frontend-roadmap.md#milestone-9--daemon-obtainability-and-start-honesty`
- `plans/subsystems/desktop-gui-frontend-roadmap.md#4-current-state` (known gap 7)

Long-term requirements:

- `plans/000-long-term-specification.md#4-enforcement-invariants`
- `plans/000-long-term-specification.md#5-crate-ownership`
- `plans/000-long-term-specification.md#6-dispatch-runtime-and-wire-identity`
- `plans/000-long-term-specification.md#9-verification-contract`
- `plans/000-long-term-specification.md#10-frontend-rules`
- `plans/000-long-term-specification.md#11-acceptance-criteria`
- `plans/001-terminology-and-domain-model.md#3-execution-terms`

Applicable ADRs:

- `plans/adrs/ADR-0007-desktop-gui-frontend-boundary.md`
- `plans/adrs/ADR-0010-gui-supported-platform-scope.md`

Primary class: capability

---

## 1. Objective

Make the daemon **obtainable** — a documented, reproducible build-and-install
path for the `eggsec-daemon` binary — and make `eggsec daemon start`
**honest**: it must never advertise a capability it cannot execute, must never
print a success message for a daemon that did not come up, and must never
displace a daemon that is already serving the socket.

Concretely, one bounded outcome with three parts:

1. **Obtainability.** A reproducible `cargo install` invocation for
   `eggsec-daemon` that compiles the engine executor in, plus a written
   statement of what that recipe does **not** cover (packaging, signing,
   notarization, installers, update channels, Windows).
2. **Start honesty.** The CLI's `daemon start` hosts a **real** executor
   instead of a rejecting stub, and the daemon host's published capability
   list is **derived** from its executor rather than hand-declared, so
   "advertises 20, executes 0" becomes unrepresentable rather than merely
   undocumented.
3. **Truthful lifecycle reporting.** The fixed 100 ms sleep is replaced by a
   bounded readiness wait that is a **connect + protocol check**, the success
   message is emitted only after that check passes, and a pre-flight liveness
   check refuses to displace a live daemon.

This milestone's code surface is **outside the GUI crate** — the CLI, the
daemon, the runtime, and the engine. That is deliberate and is recorded in the
roadmap (`plans/subsystems/desktop-gui-frontend-roadmap.md:660-662`): the GUI
is blocked on a daemon it cannot obtain and cannot trust, and no daemon
subsystem roadmap exists to carry the work.

---

## 2. Why this milestone is ready

**Closed hard dependencies: none.** The roadmap records milestone 8 as a
**soft** dependency — "the obtainable daemon is more useful with a real
feature profile, but the honesty fix does not depend on it"
(`plans/subsystems/desktop-gui-frontend-roadmap.md:638-639`). M009 therefore
does not wait for M008, and it does not pre-empt M008: the feature profile is
M008's deliverable, and M009's recipe will name M008's profile as the
richer build once it exists.

**Stable interface dependencies** (all present and exercised at the baseline):

| Dependency | Evidence | Why M009 needs it |
|---|---|---|
| `RuntimeTaskExecutor` — the runtime's executor port | `crates/eggsec-runtime/src/runtime.rs:40-57` | The place where an executor declares what it can execute (M009's honesty seam) |
| `DaemonHost::new` / `DaemonHost::new_noop` | `crates/eggsec-daemon/src/host.rs:86-118` | The daemon host whose published capability list must become derived |
| `DaemonClient::connect` + `health()` | `crates/eggsec-daemon/src/client.rs:23-29`, `:55-60` | The connect-and-ask probe used for both the pre-flight liveness check and the readiness wait |
| `ServerMessage::Health` carries `protocol_version` | `crates/eggsec-daemon-protocol/src/protocol.rs:300-305` | The protocol half of the readiness check |
| `DAEMON_PROTOCOL_VERSION = 2` | `crates/eggsec-daemon-protocol/src/protocol.rs:15` | The value the readiness check asserts against; unchanged by M009 |
| `eggsec::runtime_bridge::EggsecRuntimeExecutor` | `crates/eggsec/src/runtime_bridge/executor.rs:36`, `:128` | The real executor the CLI can host **today**, at zero dependency cost |
| `eggsec::runtime_bridge` is `#[cfg(feature = "cli")]` | `crates/eggsec/src/lib.rs:136-137` | The CLI already enables that feature, so the executor is reachable without a manifest change |
| The engine's `cli` feature is `["dep:clap", "dep:clap_complete"]` | `crates/eggsec/Cargo.toml:301` | The exact reason the obtainable daemon is feature-limited (M008's problem, §3.8) |

**Why now.** Milestone 006's daemon-lifecycle work requires an
**operator-supplied** daemon executable path because no daemon binary is
discoverable by any documented flow
(`plans/implementation/desktop-gui-frontend/006-full-operation-coverage-and-report-export.md:528-531`,
§7 Work package C items 2 and 9), and milestone 007's qualification run needs
a daemon it can actually start
(`plans/subsystems/desktop-gui-frontend-roadmap.md:552-553`). Both are
blocked on the two defects this milestone closes.

---

## 3. Current implementation evidence

Everything in this section was **measured** against the baseline commit, not
inherited from the briefing. Line numbers are baseline line numbers.

### 3.1 What `eggsec daemon start` actually does

`crates/eggsec-cli/src/daemon_cli.rs:42-76` is the `Start` arm of
`handle_daemon`. Its steps, in order:

| Step | Code | Behaviour |
|---|---|---|
| Resolve the socket path | `:43` | `socket.as_deref().unwrap_or(socket_path)` — the subcommand's `--socket` wins over the CLI's global `--socket` (`crates/eggsec/src/cli/mod.rs:245-252`, default `/tmp/eggsec-daemon.sock`) |
| Build a `DaemonConfig` | `:45-48` | Only `socket_path` is overridden; `max_clients`, `data_dir`, and `enable_persistence` take `DaemonConfig::default()` (`crates/eggsec-daemon/src/config.rs:20-30`) |
| Build the host | `:49-53` | `DaemonHost::new(config, NoopExecutor, eggsec_daemon::store::noop_store())` — a **rejecting** executor **and** a **no-op** store |
| Spawn the server | `:57-59` | `tokio::spawn(async move { run_server(host, shutdown).await })` — **in the same process**, and the `Result` is **discarded** |
| Sleep | `:61` | `tokio::time::sleep(Duration::from_millis(100))` — a **fixed** 100 ms, not a readiness check |
| Report success | `:63`, `:65` | `{"status":"started","socket":"…"}` or `Daemon started on {path}` — printed **whether or not the socket bound** |
| Wait | `:68` | `tokio::signal::ctrl_c().await.ok()` — **ctrl_c only**; `SIGTERM` is not handled, unlike the daemon binary |
| Report stop | `:72`, `:74` | `{"status":"stopped"}` / `Daemon stopped.` |

Two consequences are structural, not stylistic:

- The spawned server's `Result<(), DaemonError>` is dropped at `:58`, so a
  **bind failure is silent**. `run_server` returns `Err` when
  `UnixListener::bind` fails (`crates/eggsec-daemon/src/server.rs:95`), and the
  CLI never learns about it.
- The "started" report is emitted on a timer, so the CLI's success message
  and the daemon's actual state are **uncorrelated**.

### 3.2 The no-op executor, and the 20-advertised / 0-executable trap

`NoopExecutor` is defined at `crates/eggsec-cli/src/daemon_cli.rs:579-607`.
Its `execute` returns `Err(eggsec_runtime::RuntimeError::UnsupportedTaskKind)`
(`:605`).

The advertisement survives because the host is built with the constructor that
**does not** mark the executor as a no-op:

- `DaemonHost::new` sets `executor_is_noop: false`
  (`crates/eggsec-daemon/src/host.rs:98`) and builds its runtime with
  `RuntimeConfig::default()` (`:91`).
- The `Capabilities` arm of the host answers with
  `RuntimeCapabilities::default()` for a non-noop host
  (`crates/eggsec-daemon/src/host.rs:379-381`), and
  `RuntimeCapabilities::default()` is `daemon_conservative()`
  (`crates/eggsec-runtime/src/capabilities.rs:33-37`), which carries exactly
  **20** entries (`crates/eggsec-runtime/src/capabilities.rs:47-68`).
- Only `DaemonHost::new_noop` sets `executor_is_noop: true`
  (`crates/eggsec-daemon/src/host.rs:116`), forces
  `RuntimeCapabilities::noop()` into the runtime config (`:105-108`), and
  answers the `Capabilities` command with an **empty** task-kind list
  (`crates/eggsec-daemon/src/host.rs:371-378`).

So the CLI's `daemon start` publishes **20** task kinds and executes **0**.

### 3.3 Three distinct capability sets, and which one the daemon publishes

The word "capability" means three different things in this codebase, and
conflating them is how the trap survives:

| Set | Value | Where it is defined | What it means |
|---|---|---|---|
| **Canonical capability names** | **30** | `TaskKind::capability_name()`, `crates/eggsec-runtime/src/request.rs:562-598` | One name per `TaskKind` variant, including `resume` |
| **Published (conservative)** | **20** | `RuntimeCapabilities::daemon_conservative()`, `crates/eggsec-runtime/src/capabilities.rs:45-74` | The daemon's advertised policy set; deliberately omits `resume` (see the comment at `crates/eggsec-runtime/src/request.rs:573-575`) |
| **Published (full lab)** | **29** | `RuntimeCapabilities::full_lab()`, `crates/eggsec-runtime/src/capabilities.rs:77-115` | Also omits `resume` |
| **Runtime-enforced** | **29** for a `DaemonHost::new` host | `RuntimeConfig::default().capabilities = RuntimeCapabilities::full_lab()`, `crates/eggsec-runtime/src/runtime_config.rs:32` | The set the runtime *enforces* at submit time (`crates/eggsec-runtime/src/runtime.rs:230-235`) |

Two measured facts follow, and both matter for M009's design:

1. **The runtime enforces `full_lab()` (29), not `daemon_conservative()` (20).**
   `DaemonHost::new` builds the runtime with `RuntimeConfig::default()`
   (`crates/eggsec-daemon/src/host.rs:91`), and that default carries
   `full_lab()` (`crates/eggsec-runtime/src/runtime_config.rs:32`). The
   published list (20) is therefore a **subset** of the enforced list (29) —
   safe in the "does not over-advertise relative to enforcement" direction,
   but it means the published list and the enforced list are two different
   hand-written tables that happen to nest.
2. **`resume` is in the canonical 30 but in neither hand-written list.** A
   `TaskKind::Resume` submission is rejected as `UnsupportedTaskKind` by the
   default runtime config. That is the conservative intent, expressed by
   omission rather than by policy, and M009 **does not** change it (§5).

### 3.4 The real daemon binary, for contrast

`crates/eggsec-daemon/src/main.rs`:

| Fact | Evidence |
|---|---|
| Flags: `-s/--socket-path` (default `/tmp/eggsec-daemon.sock`), `-m/--max-clients` (default 10), `-d/--data-dir`, `--no-persistence`, `-l/--log-level` (default `info`), `--full-executor` | `:20-43` |
| The engine executor is selected only under `--full-executor` **and** only when the `full-executor` **cargo feature** is compiled in | `:103-117` |
| Without the cargo feature, the binary **warns and falls back to a no-op** | `:110-114` — `tracing::warn!("--full-executor flag set but full-executor feature not enabled, using no-op executor")` |
| Without `--full-executor`, the binary hosts `DaemonHost::new_noop` | `:115-117` |
| Logging goes to **stdout only** — `tracing_subscriber::fmt()` with no file sink | `:50-56` |
| The data directory defaults to `$HOME/.local/share/eggsec/daemon` | `:74-81` |
| Shutdown handles **SIGTERM and ctrl_c** | `:205-227` |

**The `--full-executor` flag is a runtime switch over a build-time feature.**
The cargo feature is `full-executor = ["dep:eggsec", "eggsec/cli"]`
(`crates/eggsec-daemon/Cargo.toml:18`), so a daemon built without it accepts
the flag and still runs a no-op executor, with only a warning. That single
fact is the core of M009's obtainability story (§3.7).

### 3.5 Nothing installs the daemon binary

| Fact | Evidence |
|---|---|
| `cargo install --path crates/eggsec-cli` installs only the `eggsec` binary | `crates/eggsec-cli/Cargo.toml:10-12` — one `[[bin]]`, named `eggsec` |
| `eggsec-daemon` is a **separate package** with its own `[[bin]]` | `crates/eggsec-daemon/Cargo.toml:10-12` — `name = "eggsec-daemon"`, `path = "src/main.rs"` |
| **No `Command::new("eggsec-daemon")` exists anywhere in the workspace** | `rg -n 'Command::new\("eggsec-daemon"\)' crates/` → no match |
| **No `current_exe` sibling lookup and no sidecar convention exists** | `rg -n 'current_exe\|sidecar' crates/ --glob '*.rs'` → no match |
| **No PID file and no lock file** | The daemon's only persistent artefacts are the SQLite store (`crates/eggsec-daemon/src/store/sqlite.rs`) and the socket file |
| The daemon's **only** readiness marker is a log line | `crates/eggsec-daemon/src/server.rs:111-115` — `tracing::info!("Daemon listening on {} (max_clients={})", …)`, rendered to stdout by `tracing_subscriber::fmt()` (`crates/eggsec-daemon/src/main.rs:50-56`). It is a **log format, not a contract** |

The only `Command::new(…)` call sites in the workspace are for external tools
(`adb`, `frida`, `iwlist`, `iptables`, `docker`, `git`) — see
`crates/eggsec-python/src/mobile_session.rs:777`,
`crates/eggsec-mobile-lab/src/frida.rs:70`,
`crates/eggsec/src/wireless/mod.rs:99`,
`crates/eggsec-web-proxy/src/intercept/transparent.rs:132`,
`crates/eggsec/src/container/docker.rs:221`,
`crates/eggsec/src/recon/git_secrets.rs:82`. **There is no precedent for
resolving a workspace binary from `PATH` or from a sibling directory, and
M009 does not create one.**

### 3.6 The socket-steal hazard

`run_server` **unlinks the socket path before binding**
(`crates/eggsec-daemon/src/server.rs:89-94`, bind at `:95`):

```rust
if let Err(error) = std::fs::remove_file(socket_path) {
    if error.kind() != std::io::ErrorKind::NotFound { … return Err(error.into()); }
}
let listener = UnixListener::bind(socket_path)?;
```

The unlink is correct as **stale-socket cleanup** — a crashed daemon leaves a
socket file that would otherwise make `bind` fail with `EADDRINUSE`. But it
is **indistinguishable from displacing a live daemon**, because a live
daemon's socket is also just a file at that path. The socket is then
`chmod`ed to `0600` (`:100-107`) and removed on shutdown (`:144-148`).

Consequences for any start path:

- Readiness **cannot** be inferred from "the socket file exists" — a stale
  file and a live socket look identical before the bind.
- Readiness **can** be inferred from the socket becoming **connectable and
  answering the protocol**, which is strictly stronger.
- A **pre-flight liveness check** is mandatory: attempt a **connection**, not
  a `stat`. A stale socket yields `ECONNREFUSED` on connect and is therefore
  correctly treated as "not live"; a live daemon answers `Health`.

The same rule is already recorded for the GUI's spawn route
(`plans/implementation/desktop-gui-frontend/006-full-operation-coverage-and-report-export.md:957-963`,
and `:964-973` for the readiness criterion). M009 applies it to the CLI's and
the daemon binary's own start paths, which have no such check today.

### 3.7 The obtainability gap, and the CLI feature that hides it

Two independent gaps block an operator:

1. **The daemon binary is not installed by anything** (§3.5).
2. **The CLI's daemon commands are not in the CLI's default build.** The
   `daemon`, `session`, and `task` subcommands are gated on the engine's
   `daemon-client` feature (`crates/eggsec/src/cli/mod.rs:462-470`, and the
   command-id match at `:557-561`), the CLI's `daemon_cli` module is gated on
   the same feature (`crates/eggsec-cli/src/main.rs:11-12`, `:166-172`), and
   the CLI's **default** feature set is `["tui"]`
   (`crates/eggsec-cli/Cargo.toml`) — `daemon-client` is **not** in it.

Therefore `cargo install --path crates/eggsec-cli` — the invocation
`AGENTS.md:73` documents — produces a binary that **has no `daemon`
subcommand at all**.

**And the dishonest code is outside the mandatory verification contract.**
`make check` runs `cargo check -p eggsec-cli` (default features) and
`cargo check -p eggsec-cli --no-default-features`
(`Makefile:166-167`, mirrored in `docs/VERIFICATION.md:19-20`). Neither
compiles the `daemon-client` feature, so `crates/eggsec-cli/src/daemon_cli.rs`
— the whole module — **is never compiled by `make check`**, and no CI job
enables it (`rg -n 'daemon-client' .github/workflows/` → no match).

| Consequence | Why it matters to M009 |
|---|---|
| The defect is invisible to the merge gate | An honesty fix that is never compiled is not a fix. M009 must bring the `daemon-client` profile into the contract (§6, "Runtime and concurrency") |
| The documented install does not produce a usable CLI | The obtainability recipe must name `--features daemon-client` explicitly |
| The daemon binary's real executor needs a **build** feature | The recipe must say `cargo install --path crates/eggsec-daemon --features full-executor`, not "run it with `--full-executor`" |

### 3.8 What milestones 006 and 008 own — the boundary M009 must respect

| Concern | Owner | Why M009 must not touch it |
|---|---|---|
| The daemon's **feature profile** (which engine features a GUI-serving daemon compiles in) | **M008** — `plans/subsystems/desktop-gui-frontend-roadmap.md:596-624` | M009's recipe names a profile; it does not define one. Guard 104 pins the current profile text (`scripts/check-architecture-guards.sh:2677-2680`) and changing it is M008's business |
| The daemon's **runtime publication** of enabled features (the additive `enabled_features` handshake field) | **M006** — `plans/subsystems/desktop-gui-frontend-roadmap.md:619-621`; `plans/implementation/desktop-gui-frontend/006-full-operation-coverage-and-report-export.md:641-681` | M009 MUST NOT change the daemon protocol or `DaemonCapabilities` |
| The **capability → availability** resolution on the client side | **M006** — `plans/implementation/desktop-gui-frontend/006-full-operation-coverage-and-report-export.md:875-930` | Presence is not availability (§4) |

**The doctrine M009's honesty invariant is written against.** M006's plan
states it explicitly: "the daemon advertises all 8 capabilities" whose engine
features are off, and the *client* resolves them as feature-off
(`plans/implementation/desktop-gui-frontend/006-full-operation-coverage-and-report-export.md:915-919`).
The 8 are `nse`, `hunt`, `browser`, `compliance`, `vuln`, `storage`,
`integration`, `workflow`
(`plans/implementation/desktop-gui-frontend/006-full-operation-coverage-and-report-export.md:357-362`).

So "advertises a capability it cannot execute" has **two** readings, and they
have **different owners**:

- **Executor-level** (M009's): the daemon publishes a task kind while its
  executor rejects **every** task as `UnsupportedTaskKind`. This is the lie
  M009 removes.
- **Feature-level** (M006 + M008): the daemon publishes a task kind whose
  *engine feature* is off in this build, so the engine denies it with
  `FeatureMissing` at dispatch
  (`crates/eggsec/src/dispatch/canonical_execution.rs:704-713`). This is a
  **build** property that M008 fixes and M006 publishes. M009 does not
  narrow the published list for feature reasons.

### 3.9 Documentation and GUI text that this milestone invalidates

| Location | Current claim | Status after M009 |
|---|---|---|
| `crates/eggsec-cli/src/daemon_cli.rs:579-584` | "The daemon started via `eggsec daemon start` uses a real executor through `eggsec-daemon`'s main binary. The CLI start command is a convenience wrapper that delegates to the daemon binary logic." | **False.** The code hosts the host inline with a rejecting executor; it never spawns the daemon binary. M009 deletes the struct and the comment with it (§7 Work package B) |
| `crates/eggsec-gui/src/connection.rs:48-50`, `:64-65` | "`eggsec daemon start` and a default-built `eggsec-daemon` both host a no-op executor that rejects every task" | Becomes **false for `eggsec daemon start`** after M009. The GUI's diagnostic MUST be updated to match (§6, "Frontend or operator surface") |
| `crates/eggsec-gui/src/config.rs:87` | Usage text names `eggsec-daemon --full-executor --socket-path …` | Incomplete: the flag is a runtime switch over a **build** feature. The text MUST name the build requirement (§6) |
| `docs/DAEMON.md:63-65` | "Start daemon with persistence (default)" under `eggsec daemon start` | **False.** The CLI's start path uses `eggsec_daemon::store::noop_store()` (`crates/eggsec-cli/src/daemon_cli.rs:52`) — it does **not** persist. M009 corrects the doc (§6, "Documentation and static guards") |
| `docs/DAEMON.md:33-37` ("Capabilities") | "The daemon's default capabilities are conservative — when `full-executor` is enabled but lab mode is not configured, only a safe subset of task kinds is advertised" | Inaccurate in the same way: the *published* set is a constant 20 regardless of the executor, and the daemon binary without the cargo feature publishes **zero**. M009 restates it |
| `.opencode/skills/eggsec-daemon/SKILL.md:30`, `.opencode/skills/eggsec-gui/SKILL.md:226` | Repeat the "both host a no-op executor" claim | Must be updated alongside the code (§12) |
| `architecture/desktop_gui.md:200` | Same claim | Must be updated alongside the code (§12) |

### 3.10 Known pre-existing failures (inherited, not attributable to M009)

These are **inherited** at the baseline. M009 MUST NOT silently widen its
scope to fix them, MUST NOT claim them as M009 regressions, and MUST record
them as stop conditions for the exit conditions they block (§14).

| Failure | Evidence | Blocks |
|---|---|---|
| `make check-deps` fails on advisories. The engine-side entries are `zip 2.4.2` (RUSTSEC-2026-0367, -0368, -0369 — three, not two) and the `rustls 0.21.12` family via `tokio-rustls 0.24.1` → `tiberius 0.12.3` → `eggsec`. **Seven advisory entries are reachable only through the GUI's Tauri closure** and are attributable to `crates/eggsec-gui`: `glib 0.18.5` (RUSTSEC-2024-0429), `glib-macros 0.18.5` → `proc-macro-error 1.0.4` (RUSTSEC-2024-0370), and the five `unic-*` crates (RUSTSEC-2025-0081, -0075, -0080, -0100, -0098). `bans ok, licenses ok, sources ok`. | `Cargo.lock` pins `zip 2.4.2` and `rustls 0.21.12`; `deny.toml:25-37` records ignores for the *webpki* advisories on that path but not for the `zip` triple; the full per-advisory table is in `plans/implementation/desktop-gui-frontend/007-packaging-and-platform-qualification.md` §3.9 | the `make check` contract, and therefore M9's closure |
| `cargo check --workspace --all-features` fails with `E0616: field 'url' of struct 'WebSocketFuzzer' is private` | `WebSocketFuzzer.url` is `pub(crate)` in `crates/eggsec-payloads/src/websocket.rs:41-45`; the engine reads it at `crates/eggsec/src/fuzzer/advanced.rs:446` | `make check-features-individual`, which the roadmap requires before closure (`plans/subsystems/desktop-gui-frontend-roadmap.md:768-771`) |
| `make check-python` was not run (needs `.venv-ci/`) | — | nothing in M009; recorded for completeness |

Recording a `deny.toml` exception is a **maintainer decision** (owner +
review-by date), not an implementation task M009 may take on itself.

### 3.11 Known gaps at the baseline

- **No obtainability path.** No document states how to obtain
  `eggsec-daemon`; `docs/DAEMON.md` documents the daemon's *behaviour*, never
  its *installation*.
- **The CLI's daemon-start path is dishonest** in three independent ways:
  a false executor advertisement (§3.2), a false success message (§3.1), and
  a false doc comment (§3.9).
- **Neither start path has a pre-flight liveness check** (§3.6).
- **Neither start path has a readiness check** — the CLI sleeps 100 ms
  (§3.1); the daemon binary prints its log line and hopes (§3.4).
- **The dishonest path is outside the verification contract** (§3.7).

---

## 4. Invariants that must not regress

- **M009-H1 — the executor-level honesty invariant.** A daemon MUST NOT
  publish a task kind while its executor rejects **every** task as
  `UnsupportedTaskKind`. Equivalently: **the published task-kind list is
  empty whenever the executor cannot execute any task**, and the published
  list is otherwise the executor's declared support intersected with the
  daemon's conservative policy. The invariant is **structural**: the
  published list is *derived* from the executor, not *declared* beside it.
- **Presence is not availability.** A published task kind whose *engine
  feature* is off is denied at dispatch with `FeatureMissing`
  (`crates/eggsec/src/dispatch/canonical_execution.rs:704-713`), not
  `UnsupportedTaskKind`. Narrowing the published list for feature reasons is
  **M006's and M008's** work (§3.8), not M009's.
- **The daemon protocol changes additively only.** M009 MUST NOT change
  `ClientCommand`, `ServerMessage`, `DaemonCapabilities`, or
  `DAEMON_PROTOCOL_VERSION` (which stays **2**,
  `crates/eggsec-daemon-protocol/src/protocol.rs:15`). M009 adds **no**
  protocol field, **no** command, and **no** message variant.
- **The GUI still links no engine, no daemon crate, and no other frontend
  crate** (`ADR-0007`; guard 155,
  `scripts/check-architecture-guards.sh:4528-4560`). Spawning is not linking.
- **The engine stays behind the daemon protocol.** The GUI MUST NOT depend on
  `eggsec`, `eggsec-daemon`, `eggsec-cli`, or `eggsec-tui`.
- **`EnforcementContext::evaluate()` remains the mandatory pre-dispatch
  gate.** M009 MUST NOT import `EnforcementContext`, `EnforcedDispatcher`,
  `Scope::is_target_allowed`, or `tool.execute()`.
- **The daemon's transport kind does not change.** The Unix domain socket and
  its `0600` permissions (`crates/eggsec-daemon/src/server.rs:100-101`) are
  unchanged.
- **Windows stays out of scope for the desktop GUI** (`ADR-0010`), and M009
  MUST NOT claim a Windows obtainability path.
- **Ring-only TLS everywhere.** M009 adds no TLS stack and no outbound HTTP.
- **The workspace Tokio baseline is `default-features = false` with explicit
  per-crate feature lists** (guard 105,
  `scripts/check-architecture-guards.sh:2687-2727`). `test-util` appears
  nowhere.
- **Never `let _ =` or `filter_map(|e| e.ok())`** — log via `tracing`. The
  current `tokio::signal::ctrl_c().await.ok()` at
  `crates/eggsec-cli/src/daemon_cli.rs:68` is exactly this pattern and is
  replaced, not inherited.
- **Spawned tokio tasks carry a 30–300 s timeout wrapper**, except a task
  that is long-lived by nature — in which case the exemption and its reason
  MUST be documented. The daemon's accept loop is long-lived by nature
  (`crates/eggsec-daemon/src/server.rs:117-141`) and is exempt; the
  readiness wait is finite and carries the bound.
- **No `deny.toml` exception is added by M009.** `deny.toml` is a closed
  license allow-list; a new dependency would have to clear it and would
  require a maintainer decision.
- **The `make check` contract is extended, never weakened.** M009 adds the
  `daemon-client` profile to the contract (§6); it removes nothing.
- **Guard 104's pinned dependency shapes stay green**: the CLI's engine
  dependency (`eggsec = { path = "../eggsec", version = "0.1.0", features = ["cli"] }`)
  and the daemon's `full-executor = ["dep:eggsec", "eggsec/cli"]`
  (`scripts/check-architecture-guards.sh:2670-2680`).

---

## 5. Scope

### In scope

- **An obtainability path for the daemon binary**: a reproducible
  `cargo install` invocation with the right cargo features, the documented
  default paths it produces (socket path, data directory), and an explicit
  statement of what it does **not** cover.
- **A decision on `eggsec daemon start`'s honesty**, with the four options
  presented and one chosen with its reasoning (§6, "Capability honesty").
- **The honesty fix itself**: the CLI's daemon-start path hosts a real
  executor; the daemon host's published capability list becomes derived from
  its executor; the rejecting stub executor is deleted from the CLI.
- **The false success message removed** and replaced by a readiness-confirmed
  success message; the fixed 100 ms sleep replaced by a **bounded** readiness
  wait that is a connect + protocol check.
- **A pre-flight liveness check** in both start paths (the CLI's and the
  daemon binary's), so neither displaces a live daemon.
- **The false doc comment corrected** (deleted with the struct it describes).
- **The daemon binary's fallback warning made actionable** — it must name
  the build recipe that would have produced a real executor.
- **The minimum GUI-crate touch**: the GUI's unreachable-daemon diagnostic
  and usage text, updated so they describe the post-M009 reality. Nothing
  else in the GUI crate changes.
- **The `daemon-client` profile brought into the verification contract**, so
  the honesty fix is compiled and tested by the merge gate.
- **New static guards** for the honesty invariant and the obtainability
  claims.
- **The documentation and skill updates** in §12.

### Explicitly out of scope

- **The daemon's feature profile.** Which engine features a GUI-serving
  daemon compiles in is **M008's** deliverable
  (`plans/subsystems/desktop-gui-frontend-roadmap.md:596-624`). M009's recipe
  names the profile that exists today and points at M008's for the richer
  one.
- **Any change to the daemon protocol or to `DaemonCapabilities`.** The
  runtime publication of enabled features is **M006's** deliverable
  (`plans/subsystems/desktop-gui-frontend-roadmap.md:619-621`). If the
  honesty fix turns out to require a protocol change, M009 **stops and
  escalates** (§14).
- **Narrowing the published capability list by engine feature.** That is the
  feature-level half of the doctrine (§3.8) and belongs to M006 and M008.
- **Packaging, signing, notarization, installers, and update channels.**
  Those belong to the separately-managed distribution workstream, which
  milestone 007 hands the binary and a reproducible build recipe
  (`plans/subsystems/desktop-gui-frontend-roadmap.md:557`, `plans/adrs/ADR-0010-gui-supported-platform-scope.md:190-193`).
- **The GUI's own packaging.** `bundle.active` stays `false`
  (`plans/subsystems/desktop-gui-frontend-roadmap.md:592-594`).
- **A Windows obtainability path.** `ADR-0010` fixes the desktop GUI's
  supported platforms at Linux and macOS and records Windows as out of scope
  because the daemon's transport is a Unix domain socket
  (`plans/adrs/ADR-0010-gui-supported-platform-scope.md:161-174`).
- **The GUI's `--daemon-bin` spawn-and-supervise route.** That is **M006's**
  Work package C
  (`plans/implementation/desktop-gui-frontend/006-full-operation-coverage-and-report-export.md:932-1033`).
  M009 supplies the *obtainable* binary that route needs; it does not
  implement the route.
- **Changing `run_server`'s unlink-before-bind behaviour.** The deeper fix
  — a server that refuses to bind over a live socket — is a daemon-subsystem
  change with its own blast radius (every daemon start path, including
  third-party ones). M009 puts the guard in the **start commands**, which is
  where the roadmap places it
  (`plans/subsystems/desktop-gui-frontend-roadmap.md:631-652`), and records
  the server-side hardening as deferred work (§9).
- **Adding session persistence to the CLI-hosted daemon.** The CLI's daemon
  uses `noop_store()` today (`crates/eggsec-cli/src/daemon_cli.rs:52`).
  M009 records the limitation and corrects the documentation; it does not
  create a second persistence path. The daemon binary is the persistent
  daemon.
- **Collapsing the socket-path duplication** (six literals; roadmap
  `plans/subsystems/desktop-gui-frontend-roadmap.md:204-210`). A separate
  corrective milestone.
- **Fixing the inherited `make check-deps` advisory failures or the
  inherited `--all-features` compile error** (§3.10).
- **Changing the `resume` capability's absence from the hand-written
  capability lists** (§3.3).
- **Rewriting the TUI's runtime-mode selection**
  (`crates/eggsec-tui/src/app/mod.rs:376-384`). The TUI is a separate
  frontend with its own in-process runtime and is unaffected by M009.

---

## 6. Required production changes

The template's seven named subsections come first, in the template's order.
Two milestone-specific subsections follow them, because M009's two headline
deliverables (an obtainability path and a capability-honesty rule) do not fit
the template's generic buckets.

### Core/domain

1. **The published capability list becomes a derived value.**
   `DaemonHost` gains a stored published list, computed at construction from
   the executor, and the `Capabilities` arm publishes it instead of
   recomputing a constant (`crates/eggsec-daemon/src/host.rs:379-381` today).
   The hand-set `executor_is_noop: bool` field
   (`crates/eggsec-daemon/src/host.rs:82`, `:98`, `:116`) is replaced by the
   derived state — or, if it is kept for readability, it MUST be computed
   from the same derivation and MUST NOT be settable to a value that
   contradicts the published list.
2. **The executor's declaration seam.** `RuntimeTaskExecutor`
   (`crates/eggsec-runtime/src/runtime.rs:40-57`) gains **one defaulted
   method** that lets an executor state which task kinds it can execute.
   A **defaulted** method is additive at the Rust level: every existing
   implementation keeps compiling unchanged. The default MUST preserve
   today's behaviour (no declaration ⇒ the conservative set is published);
   the workspace's rejecting stubs MUST declare the empty set.
   The method MUST be named so it cannot collide with milestone 006's
   planned `enabled_features()` on the same trait
   (`plans/implementation/desktop-gui-frontend/006-full-operation-coverage-and-report-export.md:656-665`)
   — the two answer different questions (task kinds vs feature names) and
   both are defaulted, so either landing order is safe.
3. **The CLI's rejecting stub is deleted.** `NoopExecutor`
   (`crates/eggsec-cli/src/daemon_cli.rs:579-607`) is removed and the
   daemon-start path is given the engine's real executor. The false doc
   comment goes with it.
4. **No new domain concepts.** M009 introduces no new operation, no new
   task kind, no new DTO, and no new persistence format.

### Storage and migrations

- **No storage change and no migration.** The CLI-hosted daemon keeps
  `noop_store()`; the daemon binary keeps its SQLite store
  (`crates/eggsec-daemon/src/store/sqlite.rs`). No persisted representation
  changes, so no milestone in this subsystem requires a data migration
  (`plans/subsystems/desktop-gui-frontend-roadmap.md:666-672`).
- **No new persisted artefact.** M009 deliberately does **not** introduce a
  PID file or a lock file (§3.5). A lock file would be the natural way to
  make "one daemon per socket" enforceable across processes, but it is a new
  on-disk format with its own stale-lock semantics — a design question that
  belongs to a daemon-subsystem milestone, not to a start-honesty fix. The
  pre-flight **connect** check achieves the same protection for the start
  paths M009 owns, with no new file format.

### Protocol and DTOs

- **No protocol change.** `ClientCommand`, `ServerMessage`,
  `DaemonCapabilities`, `ErrorCode`, and `DAEMON_PROTOCOL_VERSION` (2) are
  **unchanged**. The `Capabilities` response continues to carry the same
  `RuntimeCapabilities` shape; only the *value* of `task_kinds` becomes
  derived, which is a value change inside an unchanged shape.
- **No new wire message.** The readiness and liveness checks use the
  **existing** `Health` command and its `protocol_version` field
  (`crates/eggsec-daemon-protocol/src/protocol.rs:300-305`).
- **The protocol version does not move.** Adding a defaulted trait method to
  `RuntimeTaskExecutor` is a Rust-level addition, not a wire change.

### Runtime and concurrency

1. **The readiness wait is bounded and connect-based.** The CLI's start path
   MUST replace the fixed `sleep(Duration::from_millis(100))`
   (`crates/eggsec-cli/src/daemon_cli.rs:61`) with a wait that repeatedly
   attempts `DaemonClient::connect(path)` followed by `health()`, and that
   succeeds only when the daemon answers. The wait MUST carry a **named
   timeout constant** in the workspace's 30–300 s band, and the constant
   MUST be a **parameter of the wait helper** so a test can exercise the
   bound without `tokio`'s `test-util` (which the workspace forbids —
   `scripts/check-architecture-guards.sh:2700-2706`).
2. **The readiness criterion is a protocol check, not a log scrape.** The
   daemon's "Daemon listening on …" line
   (`crates/eggsec-daemon/src/server.rs:111-115`) is a log format, not a
   contract. M009 MUST NOT use it — and in the CLI's in-process design it is
   not even available, because there is no child process whose stdout could
   be read. The same rule already governs the GUI's spawn route
   (`plans/implementation/desktop-gui-frontend/006-full-operation-coverage-and-report-export.md:964-973`).
3. **The server task's outcome is no longer discarded.** The spawned
   `run_server` future's `Result` MUST be observed
   (`crates/eggsec-cli/src/daemon_cli.rs:57-59` today discards it), so a bind
   failure ends the start path immediately with the daemon's own error
   instead of being swallowed.
4. **The success report is emitted only on the ready outcome.** The start
   path's logic MUST return a **typed outcome** (ready / refused / failed /
   timed out) and the printing layer MUST map only the *ready* outcome to a
   success message. This is what makes "no success message when the socket
   failed to bind" a **testable** property rather than a promise.
5. **The start path's spawned task carries the bound.** The readiness waiter
   is finite and bounded; the daemon's accept loop is long-lived by nature
   and is exempt (its reason MUST be documented).
6. **The `daemon-client` profile joins the verification contract.**
   `make check` MUST compile and test the CLI with `--features
   daemon-client`, because the module under change is otherwise never built
   (§3.7). This is an **additive** expansion of the contract:
   `Makefile:166-167` and `docs/VERIFICATION.md:19-20` gain the profile, and
   the `make check` expansion in `docs/VERIFICATION.md:13-36` is updated to
   match. No existing line is removed.
7. **No new Tokio feature is required.** The readiness and liveness probes
   use `DaemonClient::connect` (`crates/eggsec-daemon/src/client.rs:23-29`),
   which is the daemon crate's API; the CLI therefore does not call
   `tokio::net` itself. **If** an implementer chooses to open a socket
   directly instead, the CLI's Tokio feature list MUST gain `"net"` (and
   `"io-util"` if the implementation reads from it), the change MUST be
   visible in the manifest diff, and the reason MUST be recorded — the
   workspace's per-crate feature discipline (guard 105) requires it. The
   CLI's current Tokio list is `["rt", "rt-multi-thread", "macros", "signal",
   "time"]` (`crates/eggsec-cli/Cargo.toml`).
8. **Cancellation.** The start path's wait MUST be cancellable through the
   `CancellationToken` it already owns
   (`crates/eggsec-cli/src/daemon_cli.rs:54`) so that a shutdown signal
   during the wait does not leave a half-started daemon behind.

### Frontend or operator surface

1. **The CLI's start path messages.** The success message MUST state that
   the daemon is **listening** (not merely "started"), the socket path, the
   protocol version, and the number of advertised task kinds. The JSON form
   MUST carry the same facts and MUST be emitted **only** on the ready
   outcome (`{"status":"started", …}` today, `crates/eggsec-cli/src/daemon_cli.rs:63`).
2. **The failure messages.** A refusal to displace a live daemon, a bind
   failure, and a readiness timeout each get a distinct message naming the
   socket path and the cause. None of them may contain the word "started".
3. **The capability caveat.** The start message MUST NOT claim that every
   advertised capability is executable. It states the advertised count and
   points at the capability/feature documentation, because the feature-level
   gap is real and belongs to M006/M008 (§3.8).
4. **The persistence caveat.** The start message MUST state that the
   CLI-hosted daemon does **not** persist sessions, and point at the daemon
   binary for a persistent daemon. `docs/DAEMON.md:63-65` currently claims
   the opposite.
5. **The daemon binary's fallback warning becomes actionable.** The
   `tracing::warn!` at `crates/eggsec-daemon/src/main.rs:112` MUST name the
   build recipe that would have produced a real executor
   (`cargo install --path crates/eggsec-daemon --features full-executor`),
   because the flag alone is not enough (§3.4).
6. **The GUI's diagnostic and usage text — the only GUI-crate change.**
   `crates/eggsec-gui/src/connection.rs:48-50` and `:64-65` MUST stop
   claiming that `eggsec daemon start` hosts a no-op executor, and
   `crates/eggsec-gui/src/config.rs:87` MUST name the **build** requirement
   (`--full-executor` is a runtime flag over a cargo feature). The
   diagnostic MUST keep naming the exact start command
   (`daemon_start_command`, `crates/eggsec-gui/src/connection.rs:51-53`) —
   that is milestone 006's Work package C item 1, which MUST be preserved.
   **No other GUI file changes**, and the GUI still links no engine or
   daemon crate (guard 155).
7. **No new CLI flag.** M009 adds no flag to `DaemonSubcommand::Start`
   (`crates/eggsec/src/cli/mod.rs:924-930`). The behaviour changes; the
   surface does not.

### Security and authorization

- **No enforcement change.** `EnforcementContext::evaluate()` remains the
  mandatory pre-dispatch gate; M009 touches no enforcement, scope, or
  approval path.
- **The socket's access model is unchanged**: `0600`
  (`crates/eggsec-daemon/src/server.rs:100-107`). M009 MUST NOT weaken it,
  and MUST NOT create the socket with looser permissions before the chmod.
- **No new outbound network.** The readiness and liveness probes connect to
  a **local Unix socket** the caller named. No DNS, no HTTP, no TLS.
- **The pre-flight probe is a local liveness check, not an oracle.** It
  sends `Health` — a read-only command — and does not create sessions, tasks,
  or audit records. `Health` requires no client declaration
  (`crates/eggsec-daemon/src/host.rs:362-368`).
- **Refusing to displace a live daemon is a safety property**, not a
  convenience: without it, a second start silently steals the first
  daemon's socket and the first daemon's clients fail mid-task
  (§3.6).

### Documentation and static guards

1. **`docs/DAEMON.md` gains an "Obtaining the daemon" section** with the
   reproducible recipe, the default paths it produces, and an explicit
   "what this does not cover" list (§7 Work package A).
2. **`docs/DAEMON.md`'s "Capabilities" section is restated** to separate the
   published set from the executor's actual support, and its "CLI Commands"
   block stops claiming persistence for `eggsec daemon start` (`:63-65`).
3. **`CONTRIBUTING.md`'s desktop-GUI prerequisites** (`:652`) name the
   obtainability recipe, because the GUI's documented prerequisite is
   "an already-running `eggsec-daemon --full-executor`" and the *build* step
   was never stated.
4. **`architecture/daemon.md` and `architecture/desktop_gui.md:200`** are
   updated with the corrected claim.
5. **New static guards** in `scripts/check-architecture-guards.sh`, and mirrored in
   `docs/CI_ARCHITECTURE_GUARDS.md`. **Guard numbering:** the milestone order is
   M006 (guards **161** and **162**), then M008 (guard **163**), then M009, then
   M007 — because M007 is gated on M6, M8, and M9. M009's four new guards are
   therefore **G164, G165, G166, and G167**, and M007's single new guard is
   **168**. The implementer MUST confirm the next free number against
   `scripts/check-architecture-guards.sh` at execution time and MUST NOT
   renumber another milestone's guards:
   - **G164 — the CLI carries no rejecting daemon executor.** No
     `struct NoopExecutor` and no `RuntimeError::UnsupportedTaskKind` in
     `crates/eggsec-cli/src/`. Rationale: the trap was possible because the
     CLI carried its own rejecting executor; the guard keeps it from
     reappearing.
   - **G165 — the daemon host does not publish a hardcoded capability
     constant.** `crates/eggsec-daemon/src/host.rs`'s `Capabilities` arm
     MUST NOT contain a literal `RuntimeCapabilities::default()` /
     `daemon_conservative()` / `full_lab()`; it publishes the host's stored,
     derived list.
   - **G166 — the CLI's daemon-start path has no fixed readiness sleep and
     no log-scraping readiness.** `crates/eggsec-cli/src/daemon_cli.rs` MUST
     NOT contain a sub-second readiness `sleep` and MUST NOT contain the
     literal `"Daemon listening on"`. Rationale: the 100 ms sleep is the
     defect; the log line is a log format, not a contract.
   - **G167 — the daemon's obtainability claim is documented.** A guard
     asserting that the documented `cargo install` recipe for the daemon
     names the `full-executor` feature — so a future doc edit cannot drop
     the one part of the recipe that matters.
6. **The skills and the architecture docs** are updated in the same change
   (§12), because the "both host a no-op executor" claim is repeated in
   `.opencode/skills/eggsec-daemon/SKILL.md:30`,
   `.opencode/skills/eggsec-gui/SKILL.md:226`, and
   `architecture/desktop_gui.md:200`.

### Capability honesty

**The decision.** M009 adopts **Option A** — *delegate to the real engine
executor when the engine is linked into the CLI, and advertise honestly
(empty or reduced task kinds) when it is not* — and it applies the
structural form of that option: the published capability list is **derived
from the executor**, so the lying state is unrepresentable rather than
merely avoided.

**The options, as measured.**

| Option | What it means | Cost | Why not chosen / chosen |
|---|---|---|---|
| **A** — delegate to the real engine executor when the engine is linked; advertise honestly (empty or reduced) when it is not | The CLI's `daemon start` hosts `EggsecRuntimeExecutor`; the published list is derived from the executor's declaration | Touches the runtime trait (one defaulted method), the daemon host (one derived field), and the CLI (the stub's deletion) | **CHOSEN.** The engine is *already* a dependency of the CLI with the exact feature that gates the executor (`features = ["cli"]`, pinned by guard 104; `runtime_bridge` is `#[cfg(feature = "cli")]`), so the "real executor" costs **zero new dependencies and zero manifest changes** — the cheapest possible route to a daemon that can actually execute |
| **B** — the CLI's `daemon start` spawns the `eggsec-daemon` binary | The CLI becomes a launcher for a separately installed binary | Requires an installed `eggsec-daemon`; adds a process boundary, an argv contract, a child-lifetime obligation, and a second readiness mechanism | Rejected: **nothing installs `eggsec-daemon`** (§3.5), so the command would fail for every operator who followed the documented install. It also re-creates the dependency that made M006's route operator-supplied in the first place |
| **C** — keep hosting inline, but make the advertised list truthful about what the linked executor can run | Same hosting, honest advertisement | Same trait/host work as A, without the executor swap | Rejected as the *primary* choice: it leaves `eggsec daemon start` as an **honest but empty** daemon. The CLI *can* host a real executor at no cost, so choosing C would deliver a worse product for the same change. **C's mechanism is adopted inside A** — it is exactly the "advertise honestly" half of A |
| **D** — deprecate the CLI's `daemon start` in favour of the daemon binary and print the exact command to run | The CLI stops hosting a daemon; the command prints `eggsec-daemon --full-executor --socket-path <path>` | Removes a working convenience; leaves the operator with a binary they must install separately | Rejected as the primary choice: it does not by itself fix the false success message, the socket-steal hazard, or the dishonest advertisement for anyone who ignores the deprecation. **D's messaging is adopted as a secondary element** — the start message points at the daemon binary for a persistent, feature-profile daemon (§6, "Frontend or operator surface") |

**Why A, in one line:** the CLI already links the engine with the one
feature that gates the real executor, so A buys a genuinely executable daemon
for **no new dependency**, and its "advertise honestly" half becomes
structural when the published list is derived from the executor rather than
declared beside it.

**The precise invariant A delivers.** M009-H1 (§4): the published task-kind
list is the executor's declared support ∩ the daemon's conservative policy,
and is empty whenever the executor declares no support. Under A the CLI's
executor is the engine executor, which declares the conservative set, so the
CLI publishes **20** and executes **20** at the executor level; the 8
feature-gated kinds are denied with `FeatureMissing`, which is M006's and
M008's domain (§3.8).

**What A does NOT deliver, stated plainly.** A does not make the CLI's
daemon feature-complete: the CLI's engine build enables only the `cli`
feature (`crates/eggsec/Cargo.toml:301`), so the 8 feature-gated kinds are
denied. A does not add persistence. A does not make the daemon binary
obtainable — that is Work package A. And A does not change the protocol,
which is why M009 can implement it without an ADR.

### Build, install, and obtainability

**The boundary, stated first.** The **distribution workstream** owns signing,
notarization, installers, and update channels for the **GUI**; milestone 007
hands that workstream the binary and a reproducible build recipe
(`plans/subsystems/desktop-gui-frontend-roadmap.md:557`, `:567-568`;
`plans/adrs/ADR-0010-gui-supported-platform-scope.md:190-193`). **M009's scope
is the daemon binary's build-and-install path** — which cargo features it
must be compiled with, the exact reproducible invocation, the default paths
the result uses, and an honest statement of what the resulting binary can and
cannot do. M009 produces **no signed artifact, no installer, and no release
channel**, and it does not touch the GUI's packaging.

**The recipe.** The daemon binary is a separate package with its own `[[bin]]`
(`crates/eggsec-daemon/Cargo.toml:10-12`), so it installs on its own:

```bash
# The daemon binary, with the engine executor compiled in.
cargo install --path crates/eggsec-daemon --features full-executor --locked
```

`--features full-executor` is **the part that matters**: it enables
`["dep:eggsec", "eggsec/cli"]` (`crates/eggsec-daemon/Cargo.toml:18`), which
is what makes the binary able to execute tasks at all. Without it, the binary
accepts `--full-executor` at runtime and **warns and falls back to a no-op**
(`crates/eggsec-daemon/src/main.rs:110-114`) — which is the entire defect
this milestone exists to remove.

The CLI's daemon commands need their own invocation, because
`daemon-client` is **not** in the CLI's default feature set (§3.7):

```bash
# The CLI, with the `daemon`, `session`, and `task` subcommands.
cargo install --path crates/eggsec-cli --features daemon-client --locked
```

**What the recipe does not cover** (this list is part of the deliverable):

| Not covered | Why | Owner |
|---|---|---|
| A **packaged** artifact (`.deb`, `.rpm`, `.pkg`, Homebrew formula, installer) | Packaging is the distribution workstream's | M007 → distribution workstream |
| **Signing** or **notarization** | Same | M007 → distribution workstream |
| **Update channels** or a version-pinned release feed | Same | M007 → distribution workstream |
| **Windows** | `ADR-0010`: the daemon's transport is a Unix domain socket, so the daemon and the GUI do not compile for Windows (`plans/adrs/ADR-0010-gui-supported-platform-scope.md:25-39`, `:161-174`) | A daemon-subsystem transport milestone, with its own ADR |
| A **`PATH` guarantee** | `cargo install` writes to `$CARGO_HOME/bin` (default `~/.cargo/bin`), which is on `PATH` for most developers but which the project does **not** promise. **No component may look for the daemon there** — not the GUI, not the CLI | Standing rule (§4, `ADR-0007`) |
| A **feature-complete** daemon | `--features full-executor` links the engine with the `cli` feature only (`crates/eggsec/Cargo.toml:301`), so the daemon publishes 20 capabilities of which **8** are feature-gated off in that build and are denied with `FeatureMissing` | **M008** (profile) and **M006** (publication) |
| **Session persistence** for the CLI-hosted daemon | The CLI's start path uses `noop_store()` | The daemon binary |

---

## 7. Ordered work packages

### Work package A — The obtainability path for the daemon binary

Intent:

Give an operator a reproducible way to **obtain** a daemon that can execute
tasks, and say plainly what that daemon is and is not. This unblocks
milestone 006's operator-supplied `--daemon-bin` route and milestone 007's
qualification run.

Required changes:

1. **Document the recipe** in `docs/DAEMON.md` under a new "Obtaining the
   daemon" heading, with the exact invocations from §6, "Build, install, and
   obtainability", and with `--locked` shown so the build is reproducible
   against the workspace lockfile.
2. **State the feature requirement in the same breath as the flag.** The
   document MUST say that `--full-executor` is a **runtime** flag over a
   **cargo** feature, and that a binary built without the cargo feature
   accepts the flag and falls back to a no-op with a warning
   (`crates/eggsec-daemon/src/main.rs:110-114`).
3. **State the resulting daemon's defaults**: socket path
   `/tmp/eggsec-daemon.sock` (`crates/eggsec-daemon/src/main.rs:20`,
   `crates/eggsec-daemon/src/config.rs:23`), data directory
   `$HOME/.local/share/eggsec/daemon` (`crates/eggsec-daemon/src/main.rs:74-81`),
   max clients 10, log level `info`.
4. **State the honest limitation**: the `full-executor` build links the
   engine with the `cli` feature only, so 8 of the 20 advertised
   capabilities are feature-gated off and are denied with `FeatureMissing`
   (§3.8). Point at M008 for the richer profile and at M006 for the
   client-side resolution.
5. **State the "does not cover" table** from §6, "Build, install, and
   obtainability" verbatim enough that a reader cannot mistake the recipe
   for a distribution channel.
6. **Update `CONTRIBUTING.md:652`** — the GUI's prerequisite line names an
   already-running `eggsec-daemon --full-executor` without saying how to
   obtain one or that the binary must be *built* with the feature.
7. **Correct `docs/DAEMON.md:63-65`**, which claims `eggsec daemon start`
   persists by default; it does not (`crates/eggsec-cli/src/daemon_cli.rs:52`).
8. **Make the daemon binary's fallback warning actionable**
   (`crates/eggsec-daemon/src/main.rs:112`): it MUST name the install
   invocation that produces a real executor.
9. **Do not create a `PATH`, sidecar, or `current_exe` lookup** (§3.5). The
   GUI's `--daemon-bin` remains the operator-supplied path (M006).

Acceptance evidence:

- A reviewer can follow `docs/DAEMON.md`'s obtainability section from a clean
  checkout to a running daemon, using only the commands shown, with the
  engine executor compiled in — and the running daemon answers `Health`
  with `protocol_version: 2` and `Capabilities` with a **non-empty** task-kind
  list.
- The same document states, in the same section, that a daemon built
  **without** `--features full-executor` publishes an **empty** task-kind
  list and rejects every task, and that the `--full-executor` flag alone does
  not change that.
- The document contains the "does not cover" table, including **Windows**,
  with the `ADR-0010` root cause named.
- Guard **G167** (§6, "Documentation and static guards") is green.

### Work package B — The capability-honesty fix

Intent:

Make "advertises a capability it cannot execute" **unrepresentable** in the
daemon host, and remove the CLI's rejecting executor so the CLI's daemon can
actually execute what it advertises.

Required changes:

1. **Add the executor's declaration seam.** One **defaulted** method on
   `RuntimeTaskExecutor` (`crates/eggsec-runtime/src/runtime.rs:40-57`)
   stating which task kinds the executor can execute. The default preserves
   today's behaviour; the workspace's rejecting stubs declare the empty set.
   The method MUST NOT collide with M006's planned `enabled_features()`
   (`plans/implementation/desktop-gui-frontend/006-full-operation-coverage-and-report-export.md:656-665`);
   both are defaulted, so either landing order compiles.
2. **Derive the published list in the daemon host.** `DaemonHost` computes
   its published task-kind list at construction from the executor's
   declaration intersected with the conservative policy, stores it, and
   publishes it in the `Capabilities` arm
   (`crates/eggsec-daemon/src/host.rs:370-396`). The hardcoded
   `RuntimeCapabilities::default()` at `:380` goes away, and the hand-set
   `executor_is_noop` field (`:82`, `:98`, `:116`) becomes derived.
3. **Keep the daemon's existing behaviour for a real executor.** A host with
   a real executor MUST still publish the conservative set (20) — M009 does
   **not** narrow it by engine feature (§3.8), and M006's 8-of-20 resolution
   continues to hold.
4. **Keep the daemon's existing behaviour for a no-op host.**
   `DaemonHost::new_noop` (`:104-118`) MUST still publish an **empty**
   list, now by derivation rather than by a separate branch.
5. **Delete the CLI's rejecting executor.** `NoopExecutor`
   (`crates/eggsec-cli/src/daemon_cli.rs:579-607`) is removed, and the
   daemon-start path hosts `eggsec::runtime_bridge::EggsecRuntimeExecutor`
   instead (`:49-53`). The engine is already a dependency with the gating
   feature (`crates/eggsec-cli/Cargo.toml`; guard 104,
   `scripts/check-architecture-guards.sh:2670-2673`), so **no manifest
   change is required** — and the plan requires the diff to prove it.
6. **Correct the false doc comment** by deleting it with the struct, and
   replace it with an accurate comment on the start path stating what it
   hosts and what it does not.
7. **Do not change the protocol, the protocol version, or
   `DaemonCapabilities`.**

Acceptance evidence:

- A test in the daemon crate proves that a host whose executor declares no
  support publishes an **empty** task-kind list, and that a host whose
  executor declares exactly one kind publishes exactly that one kind.
- A test in the daemon crate proves that a host with an executor that
  declares nothing still publishes the conservative set (**20**) — no
  behaviour change for real daemons.
- A test in the CLI crate proves that the CLI's daemon-start path publishes
  a **non-empty** list and that the CLI crate contains **no** rejecting
  executor (source-level assertion).
- Guard **G164** and **G165** are green.
- The daemon's `Capabilities` response for a `new_noop` host is **unchanged**
  (empty list), and for a `new` host is **unchanged** (20 conservative
  names).

### Work package C — The readiness wait and the true success message

Intent:

Replace the fixed 100 ms sleep and the unconditional "Daemon started" with a
**bounded readiness wait** and a success message that is only printed when
the daemon is demonstrably up.

Required changes:

1. **Factor the start path's logic into a typed outcome.** The start path
   MUST return one of: *ready* (with the socket path, protocol version, and
   advertised count), *refused* (a live daemon already serves the path), or
   *failed* (bind error, or readiness timeout, with the cause). The printing
   layer maps **only** the *ready* outcome to a success message. This
   factoring is what makes the "no success when the socket did not bind"
   property testable (§10).
2. **Observe the server task's result.** The spawned `run_server` future's
   `Result` MUST be awaited or otherwise observed, so a bind failure ends
   the start path with the daemon's own error
   (`crates/eggsec-daemon/src/server.rs:95`).
3. **Implement the readiness wait.** Repeatedly attempt
   `DaemonClient::connect(path)` + `health()`; succeed on the first daemon
   that answers; fail when the timeout elapses. The timeout MUST be a
   **named constant** in the 30–300 s band and MUST be a **parameter** of
   the helper so a test can exercise the bound without `test-util`.
4. **Check the protocol version.** On a successful probe, the start path MUST
   read `protocol_version` from the `Health` response
   (`crates/eggsec-daemon-protocol/src/protocol.rs:300-305`) and MUST report
   it. An unexpected version MUST be reported rather than silently accepted.
5. **Never scrape stdout.** No part of the readiness decision may depend on
   the string `"Daemon listening on"`
   (`crates/eggsec-daemon/src/server.rs:111-115`) — and in the CLI's
   in-process design there is no child stdout to scrape at all.
6. **Replace the messages.** The success message names the socket path, the
   protocol version, and the advertised task-kind count; the failure
   messages name the path and the cause. No message claims "started" unless
   the outcome is *ready*.
7. **Do not claim more than is true.** The success message states the
   advertised count and points at the capability documentation; it MUST NOT
   claim that every advertised capability is executable (§3.8), and it MUST
   state that the CLI-hosted daemon does not persist sessions.
8. **Preserve the JSON form.** `--json` MUST emit the same facts in JSON
   and MUST emit them only on the *ready* outcome.
9. **Make the wait cancellable** through the existing `CancellationToken`
   (`crates/eggsec-cli/src/daemon_cli.rs:54`).
10. **Optional, not required by the exit conditions:** handle `SIGTERM` in
    the CLI's start path, matching the daemon binary
    (`crates/eggsec-daemon/src/main.rs:205-227`). If implemented, it MUST be
    `#[cfg(unix)]`-gated like the daemon binary's, and the reason for
    deferring it MUST be recorded if it is not implemented.

Acceptance evidence:

- A test proves that a start path whose socket **cannot** bind returns the
  *failed* outcome and that **no** success text is produced.
- A test proves that the readiness wait returns the *failed* outcome with a
  timeout cause within its bound, using a short test-supplied timeout.
- A test proves that the readiness wait succeeds against a live daemon host
  and reports the protocol version.
- Guard **G166** is green.

### Work package D — The pre-flight liveness check

Intent:

Stop a start path from displacing a daemon that is already serving the
socket — because `run_server` unlinks the socket path before binding
(`crates/eggsec-daemon/src/server.rs:89-94`), a second start silently
steals a live socket.

Required changes:

1. **Probe before binding.** Each start path MUST, before starting a
   server, attempt to **connect** to the socket path and issue `Health`. A
   successful probe means a live daemon serves the path.
2. **Refuse to displace.** On a successful probe, the start path MUST
   **not** start a server, MUST report the refusal with the socket path, and
   MUST exit non-zero. It MUST NOT unlink the socket.
3. **Treat an unreachable socket as "not live".** A stale socket file (from
   a crashed daemon) fails the connect with `ECONNREFUSED`; the start path
   MUST proceed and let the server's own unlink clean it up
   (`crates/eggsec-daemon/src/server.rs:89-94`).
4. **Apply the check in both start paths**: the CLI's
   (`crates/eggsec-cli/src/daemon_cli.rs:42-76`) and the daemon binary's
   (`crates/eggsec-daemon/src/main.rs`, before `run_server` at `:199`). The
   daemon binary's check is what protects the **obtainable** daemon that
   Work package A hands operators.
5. **Never `stat` the socket.** The check MUST be a connection, not a
   file-existence test — the two are indistinguishable for a live daemon and
   misleading for a stale one (§3.6).
6. **Bound the probe.** The connect + `Health` exchange MUST be wrapped in a
   timeout so a hung daemon cannot hang the start path.
7. **Do not change `run_server`'s unlink behaviour** (§5) — the guard belongs
   in the start commands. Record the server-side hardening as deferred work
   (§9).

Acceptance evidence:

- A test starts a real daemon host on a temporary socket, runs the
  pre-flight against it, and asserts the outcome is *refused* **and** that
  the live daemon still answers `Health` afterwards — i.e. its socket was
  not stolen.
- A test leaves a stale socket file with nothing listening on it, runs the
  pre-flight, and asserts the outcome is **not** *refused* — i.e. the start
  proceeds.
- A test asserts the pre-flight's probe is bounded.

### Work package E — The verification contract, the guards, and the docs

Intent:

Make the honesty fix **compiled, tested, and guarded** by the merge gate —
because at the baseline the entire defective module is outside it (§3.7) —
and make every corrected claim in the repository agree with the code.

Required changes:

1. **Bring the `daemon-client` profile into `make check`.** Add
   `cargo check -p eggsec-cli --features daemon-client` and
   `cargo test -p eggsec-cli --features daemon-client` to the `check` target
   (`Makefile:154-182`), and mirror them in `docs/VERIFICATION.md:13-36`
   and in the defect-class table at `docs/VERIFICATION.md:50-64`. This is an
   **additive** expansion; no existing line is removed. The same profile
   MUST be added to the clippy line for the CLI
   (`Makefile:71`) so `-D warnings` covers it.
2. **Add guards G164–G167** (§6, "Documentation and static guards") to
   `scripts/check-architecture-guards.sh`, numbered after 160, and mirror
   them in `docs/CI_ARCHITECTURE_GUARDS.md`.
3. **Update the skills** `.opencode/skills/eggsec-daemon/SKILL.md:30` and
   `.opencode/skills/eggsec-gui/SKILL.md:226`, and
   `architecture/desktop_gui.md:200`, so the "both host a no-op executor"
   claim is corrected everywhere it is repeated. The skills are canonical
   (`.skills/`, `.agents/skills/`, `.claude/skills/` are symlinks to
   `.opencode/skills/`), so the edit is made once.
4. **Update `architecture/daemon.md`** with the derived-capability rule, the
   readiness criterion, and the pre-flight rule.
5. **Do not restate counted sets in the skills.** Where a skill needs a
   number, it points at `architecture/overview.md` instead.
6. **Record the milestone in `plans/registry.md`** and in the roadmap's
   milestone entry, following the pattern of the already-registered entry
   (`plans/registry.md:211`). The registry already carries the *planned*
   entry at the baseline; the implementation handoff updates it to the
   implemented state and adds the closure pointer.

Acceptance evidence:

- `make check` compiles and tests the CLI with `--features daemon-client`,
  and the expansion in `docs/VERIFICATION.md` matches the Makefile exactly.
- The four new guards are green and are listed in
  `docs/CI_ARCHITECTURE_GUARDS.md`.
- `rg -n 'no-op executor' .opencode/skills architecture docs` returns no
  claim that `eggsec daemon start` hosts one.

### Work package F — Tests, evidence, and the closure pack

Intent:

Assemble the evidence the closure record must contain, and make the
inherited failures visible as inherited rather than as M009 regressions.

Required changes:

1. **Write the tests named in §10**, in the crates that own them (the daemon
   crate, the runtime crate, the CLI crate, and the daemon binary's crate).
2. **Record the inherited failures** (§3.10) in the closure record verbatim,
   with the evidence and the blocked exit conditions.
3. **Record the commands actually run** — no command may be claimed that was
   not run (§11).
4. **Record the option decision and its reasoning** (§6, "Capability
   honesty") in the closure record, including the three rejected options and
   why each was rejected, so a later reader can re-litigate it with the
   evidence in hand.
5. **Record the deviations.** If the implementer deviates from this plan —
   a different readiness timeout, a different outcome type, a different
   guard formulation — the deviation and its reason MUST be recorded in the
   closure record, in the style of the recorded deviations in
   `plans/closure/desktop-gui-frontend/001-operation-parameter-schema-publication-closure.md:46-52`.

Acceptance evidence:

- Every test in §10 exists, is named, and passes.
- The closure record contains the §15 evidence list in full.
- The inherited failures appear in the closure record as inherited.

---

## 8. Failure, cancellation, restart, and contention semantics

**Partial failure.** If the socket cannot be bound, the start path MUST
return the *failed* outcome carrying the daemon's own error
(`DaemonError` from `crates/eggsec-daemon/src/server.rs:95`) and MUST NOT
print a success message. The server task's error MUST be observed, not
dropped — the current `tokio::spawn` at
`crates/eggsec-cli/src/daemon_cli.rs:57-59` discards it, which is the
mechanism of the false success.

**Duplicate delivery.** Not applicable: the start path is a one-shot command,
not a request/response protocol.

**Process restart.** A daemon that is killed leaves a stale socket file. The
next start's pre-flight connect fails with `ECONNREFUSED` — correctly
reading "not live" — and the server's unlink removes the stale file before
binding (`crates/eggsec-daemon/src/server.rs:89-94`). The stale-file case is
covered by a test (§10).

**Cancellation races.** The readiness wait MUST observe the
`CancellationToken` the start path already owns
(`crates/eggsec-cli/src/daemon_cli.rs:54`), so a shutdown signal during the
wait tears the attempt down rather than leaving a half-started daemon. The
wait's own timeout and the cancellation token are two independent exits, and
both MUST land on the *failed* outcome.

**Stale generations.** Not applicable: no persisted generation counter
exists, and M009 adds none.

**Concurrent callers.** Two starts against one socket path race. The
guarantee is: **at most one daemon ends up serving the path, and neither
reports a false success.** The pre-flight narrows the window but cannot
eliminate it — both callers can pass the pre-flight before either binds. In
that case the loser's `bind` fails (or its `run_server` returns `Err`), and
the loser MUST report the failure. The loser's bind failure is the
backstop; the pre-flight is the friendly path.

**The socket-steal property.** Because `run_server` unlinks before binding,
the loser's bind can *succeed* by displacing the winner. That is why the
pre-flight exists and why M009 puts it in **every** start path it owns. The
**server-side** hardening — `run_server` refusing to bind over a live socket
— is deferred (§9), and until it lands, a start path that M009 does not own
(a third-party launcher) can still displace a live daemon.

**Timeouts.** The readiness wait's timeout is a named constant in the 30–300 s
band. The pre-flight probe's timeout is shorter and separate. Every spawned
tokio task in the start path is either finite (the waiter) or long-lived by
nature (the accept loop, exempt with its reason documented).

**No `let _ =`.** The current `tokio::signal::ctrl_c().await.ok()`
(`crates/eggsec-cli/src/daemon_cli.rs:68`) swallows the signal-handler error.
M009 replaces it with a match that logs via `tracing`.

---

## 9. Compatibility and migration

**Backward compatibility.** The daemon protocol is **unchanged**: no
`ClientCommand`, no `ServerMessage`, no `DaemonCapabilities` field, no
`ErrorCode`, and `DAEMON_PROTOCOL_VERSION` stays **2**
(`crates/eggsec-daemon-protocol/src/protocol.rs:15`). An older client talking
to a post-M009 daemon sees the same message shapes. An older daemon talking
to a post-M009 client sees the same message shapes.

**The published capability value changes, and that is the point.** For a
**no-op** daemon, the published list was already empty and stays empty. For a
**real** daemon, the published list stays the conservative 20. For the
**CLI's** daemon, the published list stays the conservative 20 — but the
executor behind it changes from "rejects everything" to "dispatches through
the engine", which is the entire behavioural change of this milestone. **No
client can observe a shape change; a client CAN observe the capability value
change for a host built from the CLI's old code path — which no longer
exists.**

**Configuration fallback.** The CLI's `--socket` resolution is unchanged
(`crates/eggsec-cli/src/daemon_cli.rs:43`). The daemon binary's flags are
unchanged (`crates/eggsec-daemon/src/main.rs:20-43`).

**Data migration.** None. No persisted representation changes (§6, "Storage
and migrations").

**Removal criteria for legacy paths.** The CLI's rejecting `NoopExecutor` is
**removed** by Work package B. There is no deprecation window to manage
because the type is private to the CLI crate and has no other consumer.

**Deferred, and recorded as deferred.** The server-side hardening — making
`run_server` refuse to bind over a **live** socket rather than unlinking it
unconditionally — is not M009's, because it changes the daemon's behaviour
for **every** launcher, including ones outside this repository, and it
interacts with the stale-socket cleanup that the current unlink provides.
It belongs to a daemon-subsystem milestone. M009's start-path pre-flight is
the mitigation that is in scope.

---

## 10. Required tests

### Focused unit tests

1. **`an_executor_that_declares_no_support_publishes_no_task_kinds`** (daemon
   crate) — a stub executor whose declaration is the empty set; the host's
   `Capabilities` answer carries an **empty** `task_kinds` list. *This is the
   honesty invariant's positive half: the 20/0 state is unreachable.*
2. **`the_published_list_is_the_declared_support_intersected_with_the_policy`**
   (daemon crate) — a stub executor declaring exactly `["port-scan"]`; the
   published list is exactly `["port-scan"]`.
3. **`an_executor_that_declares_nothing_keeps_the_conservative_set`** (daemon
   crate) — the default declaration; the published list is the conservative
   set (**20** names), unchanged from the baseline.
4. **`a_noop_host_still_publishes_an_empty_list`** (daemon crate) —
   `DaemonHost::new_noop` publishes an empty list, unchanged.
5. **`the_cli_daemon_start_path_links_the_engine_executor`** (CLI crate) —
   the CLI crate contains **no** rejecting executor (a source-level
   assertion over `crates/eggsec-cli/src/`), and the host the CLI's start
   path builds publishes a **non-empty** list. *This is the "the CLI's
   capability list matches the linked executor" test.*
6. **`the_readiness_wait_has_a_bounded_timeout`** (CLI crate) — the named
   timeout constant is within the 30–300 s band, and the wait helper takes
   the timeout as a parameter (so the bound is testable without `test-util`).
7. **`the_preflight_probe_is_bounded`** (CLI crate and daemon crate) — the
   connect + `Health` probe is wrapped in a timeout.

### Integration tests

8. **`the_start_path_does_not_report_success_when_the_socket_fails_to_bind`**
   (CLI crate) — a start path pointed at a socket path that cannot be bound
   returns the *failed* outcome, and the success text is **not** produced.
   *This is the "no false success" test.*
9. **`the_readiness_wait_times_out_when_no_daemon_appears`** (CLI crate) —
   with a short test-supplied timeout, the wait returns the *failed* outcome
   with a timeout cause, within the bound.
10. **`the_readiness_wait_succeeds_against_a_live_daemon`** (CLI crate) — the
    wait returns the *ready* outcome and reports the protocol version.
11. **`the_preflight_refuses_to_displace_a_live_daemon`** (daemon crate) — a
    live host is serving the path; the pre-flight returns the *refused*
    outcome, no server is started, and the live daemon still answers
    `Health` afterwards. *This is the "refuses to displace" test.*
12. **`the_preflight_treats_a_stale_socket_as_not_live`** (daemon crate) — a
    socket file with nothing listening on it; the pre-flight does **not**
    refuse, and the start proceeds and rebinds.
13. **`a_started_daemon_answers_capabilities_with_a_non_empty_list`**
    (CLI crate) — an end-to-end over the real Unix socket: `Health` reports
    `protocol_version: 2` and `Capabilities` reports the conservative set.
14. **`the_daemon_binary_refuses_to_displace_a_live_daemon`** (daemon crate,
    against `crates/eggsec-daemon/src/main.rs`'s start path) — the same
    property on the path Work package A hands operators.

### Restart and recovery tests

15. **`a_stale_socket_does_not_block_a_start`** (CLI crate) — after a
    "crashed" daemon leaves a socket file behind, a fresh start succeeds and
    the new daemon answers `Health`.
16. **`the_socket_is_removed_on_shutdown`** (daemon crate) — the existing
    behaviour (`crates/eggsec-daemon/src/server.rs:144-149`) is pinned for
    the CLI-hosted daemon too.

### Contention and cancellation tests

17. **`two_concurrent_starts_against_one_socket_path`** (daemon crate) — at
    most one daemon ends up serving the path, and **neither** reports a
    false success; the loser reports the refusal or the bind failure.
18. **`the_readiness_wait_is_cancelled_by_the_shutdown_token`** (CLI crate) —
    cancelling during the wait ends the attempt with the *failed* outcome
    and does not leave a half-started daemon behind.

### Security and negative tests

19. **`the_start_path_never_publishes_a_capability_it_cannot_execute`** (daemon
    crate) — the honesty invariant as a property: for every host
    construction path, the published list is empty **iff** the executor
    declares no support. *This is the "advertising more capabilities than it
    can execute is impossible" test.*
20. **`the_socket_is_created_with_owner_only_permissions`** (CLI crate and
    daemon crate) — the socket's mode is `0600`
    (`crates/eggsec-daemon/src/server.rs:100-107`).
21. **`the_preflight_probe_creates_no_session_and_no_audit_record`** (daemon
    crate) — the liveness probe issues only `Health`, which requires no
    client declaration (`crates/eggsec-daemon/src/host.rs:362-368`) and
    writes nothing.
22. **`the_start_path_does_not_scrape_stdout_for_readiness`** (guard G166,
    not a Rust test) — a static check over
    `crates/eggsec-cli/src/daemon_cli.rs`.

### Migration and compatibility tests

23. **`the_protocol_version_is_unchanged`** (daemon crate and protocol
    crate) — the daemon still answers `DAEMON_PROTOCOL_VERSION == 2`, and the
    CLI's readiness check accepts it.
24. **`an_older_daemon_on_the_socket_is_detected_as_live`** (daemon crate) —
    a daemon answering a different protocol version is still *live*, so the
    pre-flight refuses to displace it, and the readiness check reports the
    version mismatch rather than accepting it silently.
25. **`the_capabilities_message_shape_is_unchanged`** (protocol crate) — the
    `ServerMessage::Capabilities` shape is byte-for-byte the baseline shape;
    M009 changes a **value**, never a **shape**.

---

## 11. Required verification commands

```bash
make check                  # mandatory Rust contract (extended with the
                            # daemon-client profile — Work package E)

# The milestone's own surfaces, explicitly (the daemon-client feature is
# NOT in the CLI's default set, so these lines are not redundant):
cargo check -p eggsec-cli --features daemon-client
cargo test  -p eggsec-cli --features daemon-client
cargo clippy -p eggsec-cli --features daemon-client -- -D warnings
cargo test  -p eggsec-daemon --lib
cargo test  -p eggsec-runtime --lib
cargo check -p eggsec-daemon --features full-executor

# Deep check required before closure (the roadmap requires it before the
# subsystem's milestones close):
make check-features-individual

# Documentation and guard references:
bash scripts/check-architecture-guards.sh
```

**Inherited failures that M009 MUST run and record, not fix** (§3.10):

```bash
make check-deps             # expected to fail on the inherited advisories
cargo check --workspace --all-features   # expected to fail with E0616
```

Do not claim commands that were not actually run in the closure record.

---

## 12. Documentation updates

| File | Change |
|---|---|
| `docs/DAEMON.md` | New "Obtaining the daemon" section (Work package A); "Capabilities" restated to separate the published set from the executor's support; "CLI Commands" stops claiming persistence for `eggsec daemon start` (`:63-65`) |
| `CONTRIBUTING.md:652` | The desktop GUI's daemon prerequisite names the obtainability recipe and the **build** requirement, not just the runtime flag |
| `architecture/daemon.md` | The derived-capability rule (M009-H1), the readiness criterion (connect + protocol, never a log scrape), and the pre-flight rule |
| `architecture/desktop_gui.md:200` | The corrected claim about `eggsec daemon start`; a pointer to the obtainability recipe for the GUI's prerequisite |
| `.opencode/skills/eggsec-daemon/SKILL.md:30` | The corrected claim; the obtainability recipe; the readiness and pre-flight rules |
| `.opencode/skills/eggsec-gui/SKILL.md:226` | The corrected claim; the GUI's prerequisite now names the build requirement |
| `.opencode/skills/eggsec-cli/SKILL.md:266` | The `daemon start` row states that the command requires the `daemon-client` feature and hosts the engine executor |
| `docs/VERIFICATION.md:13-36`, `:50-64` | The extended `make check` expansion and the new defect-class rows |
| `docs/CI_ARCHITECTURE_GUARDS.md` | Guards G164–G167 |
| `plans/registry.md` | The milestone's entry moves from *planned* to *implemented*, with the closure pointer |
| `plans/subsystems/desktop-gui-frontend-roadmap.md` | The milestone's entry records the implemented state and the closure pointer |

**Skill discipline.** `.opencode/skills/` is canonical; `.skills/`,
`.agents/skills/`, and `.claude/skills/` are symlinks to it, so the edit is
made once. Headline counts live in `architecture/overview.md`; skills point
at it rather than restating them.

---

## 13. Acceptance criteria

1. **A documented, reproducible obtainability path exists for the daemon
   binary.** `docs/DAEMON.md` states the exact `cargo install` invocation
   (with `--features full-executor`), the paths the result uses, and the
   "does not cover" list — including **Windows**, with the `ADR-0010` root
   cause named.
2. **The daemon binary is obtainable on Linux and macOS** with the engine
   executor compiled in, and a daemon built that way answers `Health` with
   `protocol_version: 2` and `Capabilities` with a **non-empty** task-kind
   list.
3. **`eggsec daemon start` never advertises a capability it cannot
   execute.** The published task-kind list is derived from the executor; a
   host whose executor declares no support publishes **none**; the CLI's
   start path hosts the engine executor and publishes a **non-empty** list.
4. **`eggsec daemon start` never prints a success message for a daemon that
   did not come up.** The success text is produced only by the *ready*
   outcome, which is reached only after a successful connect + `Health`
   probe.
5. **The fixed 100 ms readiness sleep is gone**, replaced by a bounded
   readiness wait whose timeout is a named constant in the 30–300 s band and
   whose criterion is a protocol check — never a stdout scrape.
6. **A start path refuses to displace a live daemon.** A pre-flight connect
   detects a live daemon and the start path refuses, leaving the live
   daemon's socket intact.
7. **The false doc comment is gone.** The CLI's daemon-start path carries an
   accurate description of what it hosts and what it does not.
8. **The protocol is unchanged.** No `ClientCommand`, no `ServerMessage`, no
   `DaemonCapabilities` field, and `DAEMON_PROTOCOL_VERSION` is still 2.
9. **The GUI links no engine and no daemon crate**, and the GUI's only
   changes are its unreachable-daemon diagnostic and its usage text, both
   updated to describe the post-M009 reality.
10. **The honesty fix is inside the merge gate.** `make check` compiles and
    tests the CLI with `--features daemon-client`.
11. **Every claim the milestone invalidates is corrected everywhere it is
    repeated** — `docs/DAEMON.md`, `CONTRIBUTING.md`, `architecture/daemon.md`,
    `architecture/desktop_gui.md`, and the three skills.
12. **The inherited failures are recorded as inherited**, with their
    evidence, and are not claimed as M009 regressions.

---

## 14. Stop conditions

The agent must **stop and report** rather than improvise when:

- **The honesty fix turns out to require a protocol change.** If the
  implementer concludes that the published capability list can only be made
  honest by adding a field to `DaemonCapabilities`, changing the
  `Capabilities` message shape, or moving `DAEMON_PROTOCOL_VERSION`, M009
  **stops**: that is not additive, and the roadmap's decision-point rule
  requires an ADR. The escalation is: state the exact protocol change the
  fix needs and why no value-only derivation suffices. M009 MUST NOT make
  the change.
- **The feature-level narrowing is judged to be in scope.** If the
  implementer concludes that M009's exit condition ("never advertises a
  capability it cannot execute") requires narrowing the published list by
  engine feature, M009 **stops** and escalates: that is M006's publication
  and M008's profile (§3.8), and doing it here would pre-empt both
  milestones.
- **The obtainability path starts drifting into packaging or signing.** If
  the work turns into producing a packaged, signed, notarized, or
  auto-updating artifact, M009 **stops**: signing, notarization,
  installers, and update channels belong to the separately-managed
  distribution workstream, which milestone 007 feeds
  (`plans/subsystems/desktop-gui-frontend-roadmap.md:557`, `:567-568`).
- **The inherited failures are hit.** `make check-deps` failing on the
  inherited advisories, or `cargo check --workspace --all-features` failing
  with `E0616: field 'url' of struct 'WebSocketFuzzer' is private`, are
  **inherited** (§3.10). The agent records them and does **not** attempt to
  fix them, and does **not** add a `deny.toml` exception — that is a
  maintainer decision with an owner and a review-by date.
- **Windows enters the picture.** If the work is asked to make the daemon
  obtainable, or the GUI supported, on Windows, M009 **stops**:
  `ADR-0010` fixes the desktop GUI's supported platforms at Linux and
  macOS and records Windows as out of scope because the daemon's transport is
  a Unix domain socket. Re-entry requires a daemon-subsystem transport
  milestone with its own ADR
  (`plans/adrs/ADR-0010-gui-supported-platform-scope.md:178-184`).
- **The CLI's engine dependency shape would have to change.** Guard 104
  pins `eggsec = { path = "../eggsec", version = "0.1.0", features = ["cli"] }`
  for the CLI and `full-executor = ["dep:eggsec", "eggsec/cli"]` for the
  daemon (`scripts/check-architecture-guards.sh:2670-2680`). If the honesty
  fix appears to need a different shape, M009 **stops** and escalates:
  changing the daemon's feature definition is **M008's** business.
- **A new dependency is required.** `deny.toml` is a closed license
  allow-list. If the fix appears to need a new direct dependency, the agent
  records the requirement and stops; the dependency decision and any
  exception are maintainer decisions.
- **The executor-declaration method would conflict with M006's
  `enabled_features()`.** If the two cannot coexist on
  `RuntimeTaskExecutor` without one of them changing shape, M009 **stops**
  and escalates with the exact conflict, rather than reshaping a trait
  another milestone owns.
- **The server-side socket-steal hardening is judged necessary.** If the
  implementer concludes that the pre-flight check in the start commands is
  insufficient and that `run_server` itself must refuse to bind over a live
  socket, M009 **stops** and escalates: that changes the daemon's behaviour
  for every launcher, including third-party ones, and belongs to a
  daemon-subsystem milestone (§9).
- **Repository evidence contradicts a canonical invariant.** If any
  measurement in §3 turns out to be wrong at the implementation baseline, the
  agent reports the corrected measurement with its citation and re-derives
  the affected requirement, rather than implementing to a stale fact.

---

## 15. Closure evidence required

The closure record MUST contain:

1. **The option decision and its reasoning** — the four options, the
   measurement that decided between them, the chosen option (A), and the
   three rejected options with their rejection reasons (§6, "Capability
   honesty").
2. **The obtainability recipe as executed** — the exact commands run, the
   resulting binary's behaviour (`Health` → `protocol_version: 2`;
   `Capabilities` → the published list), and the default socket path and
   data directory the binary used.
3. **The before/after capability evidence** — the published task-kind list
   of the CLI's daemon before the change (20 names, executor rejecting
   everything) and after (20 names, executor dispatching through the
   engine), with the mechanism that made the difference impossible to
   regress.
4. **The readiness evidence** — the bounded timeout constant and its value,
   the probe used, and the outcome of a deliberately failed bind (no success
   text) and of a deliberately absent daemon (bounded timeout).
5. **The contention evidence** — the two-concurrent-starts result, and the
   proof that the live daemon was not displaced.
6. **The test inventory** — every test named in §10, with its crate, and
   the pass counts for `cargo test -p eggsec-cli --features daemon-client`,
   `cargo test -p eggsec-daemon --lib`, and `cargo test -p eggsec-runtime --lib`.
7. **The guard inventory** — G164–G167 with their names, their locations in
   `scripts/check-architecture-guards.sh`, and the `docs/CI_ARCHITECTURE_GUARDS.md`
   mirror.
8. **The contract expansion** — the diff to `Makefile` and
   `docs/VERIFICATION.md` showing the `daemon-client` profile added, with
   the before/after expansion.
9. **The documentation diff** — every file in §12, with the corrected
   claims quoted.
10. **The inherited failures, recorded as inherited** — the `make check-deps`
    advisory table, the `--all-features` `E0616` failure, and
    `make check-python` not run, each with its evidence and the exit
    conditions it blocks.
11. **The commands actually run**, and their results. No command may be
    claimed that was not run.
12. **Any deviation** from this plan, with its reason, in the style of the
    recorded deviations in
    `plans/closure/desktop-gui-frontend/001-operation-parameter-schema-publication-closure.md:46-52`.

---

## 16. Handoff notes

**Known hazards.**

- **The defective module is outside the merge gate at the baseline**
  (§3.7). If Work package E is skipped, the milestone's central change is
  never compiled by CI — and the fix would look done while being unverified.
  This is the single most important sequencing constraint in the plan.
- **`--features full-executor` is the difference between a usable daemon and
  a no-op one** (`crates/eggsec-daemon/src/main.rs:110-114`). Any doc edit
  that drops it re-creates the defect for operators.
- **`DaemonHost::new` currently publishes a constant**, and the daemon's
  tests assert against today's behaviour. Changing the derivation must not
  change the published list for a real daemon (§7 Work package B item 3) —
  otherwise the change is a behaviour regression, not a refactor.
- **The socket-steal window is narrowed, not closed.** Two concurrent starts
  can still both pass the pre-flight; the loser's bind is the backstop, and
  the loser's bind can still succeed by displacing the winner
  (§8). M009's guarantee is "neither reports a false success", not "the
  winner always survives".
- **The CLI's daemon-start path still does not persist.** M009 documents that
  rather than fixing it (§5); an operator who needs persistence must run the
  daemon binary.
- **The 8 feature-gated capabilities remain advertised** by a `cli`-only
  build. That is M006's and M008's (§3.8). The start message states the
  advertised count and points at the documentation rather than claiming
  full executability.

**Resource constraints.** The readiness wait's timeout is in the 30–300 s
band; the default MUST be short enough that an operator sees a failure
promptly and long enough that a loaded machine can bind. The wait helper
takes the timeout as a parameter so the tests can use a short one without
`tokio`'s `test-util`, which the workspace forbids
(`scripts/check-architecture-guards.sh:2700-2706`).

**Serial-test requirements.** The socket tests bind real Unix sockets under a
temporary directory. Each test MUST use its own socket path; tests that
exercise the socket-steal behaviour MUST NOT run concurrently with another
test that touches the same path. The workspace has no `cargo-nextest`
requirement (`docs/VERIFICATION.md:38`), so serial execution is the default
and must be arranged by path isolation, not by a test-runner flag.

**Environment requirements.** The tests are Unix-only by construction — the
daemon's transport is a Unix domain socket
(`crates/eggsec-daemon/src/server.rs:1`, `:5`) and `ADR-0010` records
Windows as out of scope for the GUI. The CI matrix for the affected crates
is Linux and macOS. No webview SDK is required: none of M009's changes touch
`crates/eggsec-gui`'s shell profile.

**Preserved user changes.** M009 touches `crates/eggsec-cli`,
`crates/eggsec-daemon`, `crates/eggsec-runtime`, `crates/eggsec-gui`
(two files' text only), `Makefile`, `docs/`, `scripts/`, `.opencode/skills/`,
`architecture/`, and `plans/`. Any pre-existing uncommitted change in those
paths MUST be preserved and merged, not overwritten — in particular the
`plans/implementation/desktop-gui-frontend/006-…` and `007-…` plans and
their closures, which this milestone must not contradict.

**Boundary reminder for the next agent.** M009 does **not** own the daemon's
feature profile (M008), the runtime publication of enabled features (M006),
the GUI's spawn-and-supervise route (M006), the GUI's packaging (M007), or
signing and installers (the distribution workstream). If a change appears to
need one of those, it is a stop condition, not a decision.
