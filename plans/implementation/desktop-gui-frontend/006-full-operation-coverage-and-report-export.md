# Desktop GUI Frontend Milestone 006 — Full Operation Coverage and Report Export

Status: implemented

Repository baseline: branch `feat/auto-20261022-76692679` at commit `9a9b000f`
("fix(make): drop backticks from the new help lines"), with milestones
001–005 closed, milestones 007–009 implemented (008 and 009 closed, 007
conditionally closed), and milestone 006 in progress.

Source roadmap:

- `plans/subsystems/desktop-gui-frontend-roadmap.md#milestone-6--full-operation-coverage-and-report-export`

Long-term requirements:

- `plans/000-long-term-specification.md#4-enforcement-invariants`
- `plans/000-long-term-specification.md#5-crate-ownership`
- `plans/000-long-term-specification.md#6-dispatch-runtime-and-wire-identity`
- `plans/000-long-term-specification.md#10-frontend-rules`
- `plans/001-terminology-and-domain-model.md`

Applicable ADRs:

- `plans/adrs/ADR-0007-desktop-gui-frontend-boundary.md`
- `plans/adrs/ADR-0008-operation-parameter-schema-publication.md`
- `plans/adrs/ADR-0009-gui-shell-technology-selection.md`

Primary class: capability

## 0. Amendment record

**Amended 2026-10-24.** This plan was authored at `34df13d0`, before
milestones 007–009 landed. Milestone 008 (the daemon feature profile) and
milestone 009 (derived daemon capabilities and start honesty) falsified
several of its §3 measured facts, and one defect it did not anticipate was
found to block its own work. The section numbering and the 16-section
structure are unchanged; the corrections are recorded here and applied in
place in §3, §6, §7, and §13.

| # | Plan said | Now true | Superseded by |
|---|---|---|---|
| A1 | `--full-executor` "enables the engine's `cli` feature and **nothing else**" (§3.7) | `full-executor` is the deprecated alias of the `service` profile, which enables **20** engine features (`crates/eggsec-daemon/Cargo.toml:25,52`) | 008 |
| A2 | "**8 of the 20** capabilities it advertises map to operations whose features are off … A stock `eggsec-daemon --full-executor` will deny all 8 with `FeatureMissing`" (§3.7) | A `service`-profile daemon enables the features of **all 20** conservative capabilities and denies none of them. The published capability set is unchanged; the reason an operation is unavailable is now the daemon's advertised set, not a missing feature | 008, 009 |
| A3 | "A default-built binary accepts `--full-executor` and still runs a no-op executor" (§3.7) | The CLI's rejecting `NoopExecutor` was **deleted**; the CLI now hosts `EggsecRuntimeExecutor` (`crates/eggsec-cli/src/daemon_cli.rs`). A daemon **binary** built without the `service` feature still falls back to `DaemonHost::new_noop` (`crates/eggsec-daemon/src/main.rs:119-130`) and publishes an **empty** capability list | 009 |
| A4 | "A daemon publishes a capability list that says nothing about which operations it can execute" (§3.4) | The published list is now the executor's declaration intersected with the daemon's conservative policy (`crates/eggsec-daemon/src/host.rs:99-108`), so a rejecting executor can no longer advertise anything | 009 |

**The §3.6 trap is closed.** The plan's central hazard — a daemon
advertising 20 capabilities while executing none of them — was real at
`34df13d0` and is now **unrepresentable**: the published list is derived
from the executor's own declaration, and a rejecting executor declares
nothing. The GUI's existing empty-list check remains as a belt-and-braces
guard, but milestone 006 no longer has to work around the trap; it has to
work *with* the derived set, which means the coverage model has to explain
the ten canonical kinds the conservative policy withholds rather than
merely hide them.

**Scope addition (user-directed, 2026-10-24).** The user directed that the
`DesktopGuiManual` permissive-set defect be folded into this milestone.
The defect is engine-side (`crates/eggsec/src/runtime_bridge/executor.rs`),
outside the GUI crate, and owned by no other milestone. It is a live
correctness defect, not a hypothetical: a GUI session whose scope is not
explicit takes the strict branch and fails closed, so **every GUI task
dispatched through the daemon-hosted executor is denied**. ADR-0007 and
§4 of the long-term specification both describe the desktop GUI as a
**permissive manual surface**, so the code contradicts the canonical
contract. Because the defect was found while executing this milestone and
blocks its own end-to-end verification, it is folded in here as work
package G rather than left unowned. The fold-in was explicitly directed by
the user; it is not an expansion the implementer chose.

**Two ambiguities are resolved here rather than left to the implementer.**

1. **The enabled-feature handshake.** The plan left the wire shape open
   between `Vec<String>` and `Option<Vec<String>>`. The resolved shape is
   `Option<Vec<String>>` on the wire and on the executor trait:
   `None` means **unknown** (the daemon links no engine, or predates the
   field), `Some(vec![])` means "an engine is linked with no features
   enabled", and `Some(names)` means "an engine is linked and these are its
   enabled features". An empty `Vec` is therefore never ambiguous, and a
   client that sees `None` degrades to "availability unverified" rather
   than to "nothing is enabled".

2. **The HTML export and the CSP.** §6 requires the exported HTML file to
   be "the CLI's HTML, byte-for-byte", while §7 acceptance evidence and
   §13 criterion 9 require "the exported HTML artifact contains no remote
   origin reference". Those two cannot both hold: the CLI's HTML carries
   exactly one remote reference, the `<script src="https://cdn.jsdelivr.net/npm/chart.js">`
   tag in `crates/eggsec-output/src/html.rs`. The resolved reading, which
   preserves both the stop condition (the CLI's HTML artifact does not
   change) and guard 158, is:
   - the **exported file** is the CLI's HTML byte-for-byte, including its
     CDN reference — the operator opens it in a browser, where the CLI's
     own artifact behaves identically;
   - the **embedded display** uses a new self-contained rendering with the
     remote `<script>` tag omitted, so nothing the GUI renders for display
     references a remote origin;
   - guard 162 constrains the *display* artifact, not the operator's
     exported file.

## 1. Objective

Make the desktop GUI reach **every one of the 30 published `TaskKind`
variants**, including the ones whose engine feature is off in the connected
daemon — hiding or clearly reporting each one rather than letting it fail
opaquely — and add **report export** that produces the same artifacts the CLI
produces, through the existing `eggsec-output` renderers.

Three concrete outcomes in one milestone:

1. A **coverage model** in the GUI that reaches all 30 variants, proven by a
   coverage test that pins the exact multiplicities (30 variants, 26 operation
   ids, 30 capability names).
2. An **availability resolution** path that separates "this daemon cannot run
   this at all" from "this daemon can run it but the engine feature is off",
   using an additive read path plus a mandatory denial-driven fallback.
3. **Report export** through `eggsec-output`, with the daemon-lifecycle
   decision made explicit rather than left as an accident of the shell's first
   cut.

## 2. Why this milestone is ready

Closed hard dependencies:

- Milestone 001 (closed 2026-10-23) — the operation catalog and the request
  schema are published over the read-only `DescribeOperations` command:
  `describe_operations()` (`crates/eggsec-daemon-protocol/src/operations.rs:100`).
  All 30 task kinds are published with their `operation_id`, their
  `capability_name`, and their JSON Schema.
- Milestone 004 (implemented 2026-10-23) — the Tauri shell, the daemon
  connection, the `DesktopGuiManual` session, and the catalog
  (`crates/eggsec-gui/src/shell.rs:24`).
- Milestone 005 (implemented 2026-10-23) — the generic form engine
  (`crates/eggsec-gui/src/form.rs`), the bounded task state
  (`crates/eggsec-gui/src/tasks.rs`), submission, cancellation, and result
  rendering through `eggsec-ui-model`'s registry.

Stable interface dependencies:

- `RuntimeClient` (`crates/eggsec-frontend-client/src/lib.rs:66-88`) — submit,
  cancel, cancel-active, subscribe. The GUI's eight IPC commands are registered
  at `crates/eggsec-gui/src/shell.rs:33-42`.
- `TaskKind` — 30 adjacently tagged variants
  (`crates/eggsec-runtime/src/request.rs:72-103`).
- `TaskKind::capability_name()` (`crates/eggsec-runtime/src/request.rs:562-598`)
  and `TaskKind::operation_id()` (`:620-658`) — two exhaustive hand-written
  identity tables.
- `OperationDescription.required_features`
  (`crates/eggsec-daemon-protocol/src/operations.rs:41-42`), populated from
  `eggsec_policy::ALL_OPERATION_METADATA` (`:127-131`).
- `eggsec-output`'s six renderers (`crates/eggsec-output/src/convert.rs:19,53,82,118,160,179`)
  over `ScanReportData` (`crates/eggsec-report-model/src/report.rs:10-24`).

The milestone is ready **because M005 built the form engine and the task
dashboard**: M006 adds reachability, availability, lifecycle, and export on top
of a working submission path — not a second form engine.

## 3. Current implementation evidence

### 3.1 The GUI crate at the baseline

`crates/eggsec-gui` is a leaf application crate. Its dependencies
(`crates/eggsec-gui/Cargo.toml:37-49`) are `eggsec-frontend-client`,
`eggsec-runtime`, `eggsec-daemon-protocol`, `eggsec-ui-model`, `serde`,
`serde_json`, `tokio`, `tracing`, plus the optional Tauri stack behind the
`shell` feature (`:29-35`; `default = ["shell"]` at `:28`). Two facts from
that manifest matter here: the GUI's Tokio feature list is
`["rt", "sync", "time", "macros", "net"]` (`:48`), so **`tokio::process` is not
available today** and spawning a child requires a manifest change (Work
package C); and `eggsec-output` is **not** a dependency, so adding it widens
the GUI's closure (§6, "Protocol and DTOs").

| Item | Location | Note |
|---|---|---|
| `GuiOptions { socket_path }` | `crates/eggsec-gui/src/config.rs:23-25` | one field only |
| `DEFAULT_SOCKET_PATH` | `crates/eggsec-gui/src/config.rs:19` | `/tmp/eggsec-daemon.sock` |
| `--socket` parsing / `usage()` | `crates/eggsec-gui/src/config.rs:48-83,78-89` | `usage()` ends with `eggsec-daemon --full-executor --socket-path /tmp/eggsec-daemon.sock` |
| `daemon_start_command()` | `crates/eggsec-gui/src/connection.rs:57-59` | the precise start command |
| `unreachable_diagnostic()` | `crates/eggsec-gui/src/connection.rs:66-74` | names the socket and the start command |
| `daemon_advertises_no_task_kinds()` | `crates/eggsec-gui/src/connection.rs:177-179` | the **empty-list** no-op check |
| `list_task_kinds()` / `unavailable_reason()` | `crates/eggsec-gui/src/commands.rs:430-455,319-331` | two reasons only: not-exposed, not-supported |
| `submit_task()` | `crates/eggsec-gui/src/commands.rs:513-567` | checks exposure + capability, then submits |
| log-ring caps | `crates/eggsec-gui/src/tasks.rs:28,35` | 200 per task, 50 per flush |
| IPC command set | `crates/eggsec-gui/src/shell.rs:33-42` | 8 commands |

The GUI has **34 tests** at the baseline: 6 in `config.rs`, 4 in
`connection.rs`, 11 in `form.rs`, 9 in `tasks.rs`, 4 in `commands.rs`.

The frontend is deliberately **buildless**: `crates/eggsec-gui/frontend/` holds
`index.html`, `styles.css`, and `app.js` with **zero** npm dependencies; its
own retention caps are at `crates/eggsec-gui/frontend/app.js:28-29`. The
webview's constraints are in `crates/eggsec-gui/tauri.conf.json`:
`withGlobalTauri: true` (`:10`), a restrictive CSP (`:23`), and
`bundle.active: false` (`:26-28`). The capability set grants `core:default`
only — no http, shell, fs, process, or updater plugin
(`crates/eggsec-gui/capabilities/default.json:6`).

### 3.2 The socket-path default is duplicated in six places, not three

`crates/eggsec-gui/src/config.rs:5-6` states the default is hardcoded in "three
independent places" and lists three; `architecture/desktop_gui.md:406` repeats
the claim as "four". The **measured** count is **six** — those four plus two
in the GUI's own frontend, `crates/eggsec-gui/frontend/app.js:23` and
`crates/eggsec-gui/frontend/index.html:25`. All six carry the identical literal
`/tmp/eggsec-daemon.sock`.

This is a documentation defect, not a functional one, and **collapsing the
duplication is explicitly out of scope** for M006 (§5). M006 must add its own
defaults without making the duplication worse: a new GUI option's default
MUST live in exactly one place in `crates/eggsec-gui/src/config.rs` and be
shipped to the webview, not re-declared in the frontend.

### 3.3 The 30 task kinds and their two identity tables

`TaskKind` has **30** variants (`crates/eggsec-runtime/src/request.rs:72-103`).
`operation_id()` (`:620-658`) and `capability_name()` (`:562-598`) are two
exhaustive, hand-written matches — adding a variant without updating either is
a compile error.

Measured over all 30 variants:

- **30** distinct `capability_name` values.
- **26** distinct `operation_id` values.
- **`operation_id != capability_name` for 10 of the 30.**
- Four collapses produce the 26: `Resume` → `pipeline`; `PacketCapture`,
  `PacketTraceroute`, `PacketSend` → `packet`; `WirelessActive` → `wireless`.

The full table, with the catalog's `required_features` and the daemon's
conservative capability set:

| # | Variant | `operation_id` | `capability_name` | Catalog `required_features` | In `daemon_conservative()`? |
|---|---|---|---|---|---|
| 1 | `LoadTest` | `load-test` | `load-test` | — | yes |
| 2 | `StressTest` | `stress-test` | `stress-test` | `stress-testing` | **no** |
| 3 | `PortScan` | `scan-ports` | `port-scan` | — | yes |
| 4 | `EndpointScan` | `scan-endpoints` | `endpoint-scan` | — | yes |
| 5 | `Fingerprint` | `fingerprint` | `fingerprint` | — | yes |
| 6 | `Fuzz` | `fuzz` | `fuzz` | — | yes |
| 7 | `Waf` | `waf-detect` | `waf` | — | yes |
| 8 | `WafStress` | `waf-stress` | `waf-stress` | — | yes |
| 9 | `Pipeline` | `pipeline` | `pipeline` | — | yes |
| 10 | `Resume` | `pipeline` | `resume` | — | **no** |
| 11 | `Recon` | `recon` | `recon` | — | yes |
| 12 | `PacketCapture` | `packet` | `packet-capture` | `packet-inspection` | **no** |
| 13 | `PacketTraceroute` | `packet` | `traceroute` | `packet-inspection` | **no** |
| 14 | `PacketSend` | `packet` | `packet-send` | `packet-inspection` | **no** |
| 15 | `GraphQl` | `graphql` | `graphql` | — | yes |
| 16 | `OAuth` | `oauth` | `oauth` | — | yes |
| 17 | `AuthTest` | `auth-test` | `auth-test` | — | yes |
| 18 | `Nse` | `nse` | `nse` | `nse` | yes |
| 19 | `Hunt` | `hunt` | `hunt` | `advanced-hunting` | yes |
| 20 | `Browser` | `browser` | `browser` | `headless-browser` | yes |
| 21 | `Compliance` | `compliance` | `compliance` | `compliance` | yes |
| 22 | `Storage` | `storage` | `storage` | `database` | yes |
| 23 | `Integrations` | `integrations` | `integration` | `external-integrations` | yes |
| 24 | `Workflow` | `workflow` | `workflow` | `finding-workflow` | yes |
| 25 | `Vuln` | `vuln` | `vuln` | `vuln-management` | yes |
| 26 | `Wireless` | `wireless` | `wireless` | `wireless` | **no** |
| 27 | `WirelessActive` | `wireless` | `wireless-active` | `wireless` (see note) | **no** |
| 28 | `DbPentest` | `db-pentest` | `db-pentest` | `db-pentest` | **no** |
| 29 | `Intercept` | `proxy-intercept` | `intercept` | `web-proxy` | **no** |
| 30 | `C2` | `c2` | `c2` | `c2` | **no** |

The `required_features` column is read from
`eggsec_policy::ALL_OPERATION_METADATA` (`crates/eggsec-policy/src/catalog.rs:291`,
34 entries) and reaches the client through
`DescribeOperationsResponse.operations[].required_features`
(`crates/eggsec-daemon-protocol/src/operations.rs:41-42,127-131`).

**Note on row 27.** The catalog publishes `wireless` as `WirelessActive`'s
required feature, because its `operation_id` is `wireless`. The **actual
compile gate** for the active attack is `wireless-advanced`
(`crates/eggsec/src/dispatch/canonical_execution.rs:1842-1855`, which degrades
to `wireless` only when `wireless` is on and `wireless-advanced` is off); the
catalog's separate `wireless-deauth` entry carries `wireless-advanced`. The
published catalog therefore **under-reports** this one gate, so the GUI MUST
NOT treat the catalog as the sole authority for feature gating (§3.4).

**Note on rows 19–25.** These 8 variants are advertised by the daemon's
conservative capability set, yet their engine features are **off** in a stock
`--full-executor` daemon (§3.7). "Advertised" does not mean "runnable".

### 3.4 A client CANNOT determine feature-gated unavailability from the capability list

This is the decisive fact behind the roadmap's "hidden or reported as
unavailable" exit condition, and it is **measured**, not assumed:

1. **The capability list is a compile-time constant, never filtered by enabled
   features.** The only construction sites are the three in
   `crates/eggsec-runtime/src/capabilities.rs` — `daemon_conservative()`
   (`:45-74`, 20 entries), `full_lab()` (`:77-115`, 29 entries), `noop()`
   (`:124-132`, 0 entries) — plus the literal at
   `crates/eggsec-daemon/src/host.rs:373`.
2. **The engine never populates `task_kinds`.** `RuntimeTaskExecutor` has
   exactly one method, `execute`
   (`crates/eggsec-runtime/src/runtime.rs:40-57`); the engine's
   `EggsecRuntimeExecutor` implements no capabilities method
   (`crates/eggsec/src/runtime_bridge/executor.rs:141-238`).
3. **`TaskCapability` carries no feature field** — it is
   `{ name, description }` (`crates/eggsec-runtime/src/capabilities.rs:7-10`).
4. **Presence ≠ available.** `daemon_conservative()` advertises `nse`, `hunt`,
   `browser`, `compliance`, `vuln`, `storage`, `integration`, and `workflow` —
   **8 of its 20** — whose operations require engine features a stock daemon
   build does not enable (§3.7).
5. **Absence ≠ feature-off.** The conservative set omits stress, packet,
   wireless, db-pentest, intercept, and c2 for **hazard** reasons
   (`crates/eggsec-runtime/src/capabilities.rs:42-44`), and omits `resume`
   deliberately (`crates/eggsec-runtime/src/request.rs:573-575`).
6. **The protocol has no "enabled features" field.** `DaemonCapabilities` is
   `{ runtime, transports, operation_schema }`
   (`crates/eggsec-daemon-protocol/src/protocol.rs:46-51`), and the daemon fills
   it with exactly those three (`crates/eggsec-daemon/src/host.rs:399-409`).

**Only two reliable ways exist to learn that a feature is off:** (a) attempt
the task and read the denial — `EnforcementContext` snapshots
`current_enabled_features()` (`crates/eggsec/src/policy_bridge/features.rs:11-17`),
the descriptor's `required_features` propagate, and the check is fail-closed:
a missing feature denies with `DenialClass::FeatureMissing` and the message
`required feature '<X>' is not enabled`
(`crates/eggsec-policy/src/decision.rs:914-930`); or (b) know the daemon
binary's build out of band. There is a third, execution-side signal: the
strict executor path raises
`ExecutionError::FeatureUnavailable { operation_id, feature }`
(`crates/eggsec/src/dispatch/canonical_execution.rs:1795-1798,1817-1820,1842-1855`).

**The three CLI mechanisms a feature-off operation can hit** — the GUI must
understand all three, because the denial text differs:

| # | Mechanism | Where | Observable effect |
|---|---|---|---|
| 1 | **Clap-level hiding** — the subcommand variant does not exist in the binary | 25 `#[cfg(feature = ...)]` subcommand arms in `crates/eggsec/src/cli/mod.rs` (`:285,336,345,348,359,369,372,375,378,391,398,401,410,418,423,428,433,438,443,448,453,458,463,466,469`), mirrored in the routing at `crates/eggsec/src/commands/route.rs:149-266` (19 `cfg`-guarded arms) | the CLI reports "unrecognized subcommand" — the operation is *absent*, not denied |
| 2 | **Handler-level "not enabled" with exit code 0** | e.g. `crates/eggsec/src/commands/handlers/storage.rs:66-71` prints "Database storage requires the 'database' feature to be enabled." and returns `Ok(())` | **exit 0** — a caller cannot treat non-zero as "unavailable" |
| 3 | **Enforcement-time denial** | `crates/eggsec/src/policy_bridge/features.rs:11-17` → `crates/eggsec-policy/src/decision.rs:914-930` → `Denied` / `DenialClass::FeatureMissing`; second gate `ExecutionError::FeatureUnavailable` | a structured denial naming the missing feature |

The CLI's own registry records the gate as data:
`feature: Option<&'static str>` (`crates/eggsec/src/commands/registry.rs:76`).
The GUI reads the same fact over the wire through
`OperationDescription.required_features`.

### 3.5 The daemon's readiness and lifecycle signals

The roadmap makes the daemon-lifecycle decision **M6's** to make
(`plans/subsystems/desktop-gui-frontend-roadmap.md:543-549`). The measured
facts:

| Fact | Evidence |
|---|---|
| `--full-executor` is a **cargo feature**, not a runtime capability | `crates/eggsec-daemon/Cargo.toml:18`: `full-executor = ["dep:eggsec", "eggsec/cli"]` — it enables the engine's `cli` feature and **nothing else** |
| A default-built binary accepts `--full-executor` and still runs a no-op executor | `crates/eggsec-daemon/src/main.rs:40-43` (the flag), `:103-117` (the selection); the `#[cfg(not(feature = "full-executor"))]` arm at `:110-114` logs a warning and builds `DaemonHost::new_noop` |
| The daemon logs to **stdout only** | `crates/eggsec-daemon/src/main.rs:50-56` — no file sink, no PID file, no lock file anywhere in the crate |
| The data directory defaults to `$HOME/.local/share/eggsec/daemon` | `crates/eggsec-daemon/src/main.rs:74-81` |
| Shutdown is **SIGTERM / ctrl_c** — not SIGHUP | `crates/eggsec-daemon/src/main.rs:205-227` |
| `run_server` **unlinks the socket path before binding** | `crates/eggsec-daemon/src/server.rs:141-146`, then binds at `:95` — a second daemon silently steals a live socket |
| The socket is chmod 0600 | `crates/eggsec-daemon/src/server.rs:152-159` |
| The **only** readiness marker is a stdout log line | `crates/eggsec-daemon/src/server.rs:163-167`: `"Daemon listening on {socket} (max_clients={n})"`. No handshake, no PID file, no ready byte |
| Shutdown removes the socket | `crates/eggsec-daemon/src/server.rs:196-200` |

Consequences for spawn-and-supervise:

- Readiness cannot be inferred from "the socket file exists" — the path is
  unlinked and re-created, and a *stale* socket file from a crashed daemon
  looks identical to a live one before the bind.
- Readiness **can** be inferred from the socket becoming connectable, which is
  strictly stronger than the log line: the GUI's own connect path already
  proves the daemon speaks the protocol
  (`crates/eggsec-gui/src/connection.rs:111-170`).
- A pre-flight liveness check is mandatory: before spawning, the GUI must
  determine whether a daemon is already serving the socket path, or it will
  start a second daemon that steals the first one's socket.

**No daemon executable is discoverable.** `cargo install --path crates/eggsec-cli`
installs only the `eggsec` binary — nothing in the workspace installs
`eggsec-daemon`. There is no `current_exe` sibling lookup, no sidecar
convention, and no `which` precedent anywhere in the workspace, and no
`Command::new("eggsec-daemon")` exists in the repository. **A spawn-and-supervise
route therefore cannot assume `eggsec-daemon` is on `PATH`.**

**The GUI must not depend on `eggsec-daemon`.** The daemon's `full-executor`
feature pulls the engine (`crates/eggsec-daemon/Cargo.toml:18`), so a GUI that
linked it would link the engine and violate `ADR-0007` and guard 155
(`scripts/check-architecture-guards.sh:4768-4834`). Spawning it as a **child
process** is a different matter from linking it — but the spawn route is still
opt-in and operator-supplied (Work package C).

### 3.6 The 20-advertised / 0-executable trap — closed by milestone 009

> **Superseded at the current baseline.** The trap described below was real
> at `34df13d0` and milestone 009 closed it. The historical evidence is
> kept because it explains why the coverage model has to explain the
> *withheld* kinds rather than trust the advertised list.

`eggsec daemon start` (the CLI's convenience wrapper) hosts a **no-op
executor** that nonetheless advertises the full conservative capability list:

```rust
let host = std::sync::Arc::new(eggsec_daemon::host::DaemonHost::new(
    config,
    NoopExecutor,
    eggsec_daemon::store::noop_store(),
));
```

(`crates/eggsec-cli/src/daemon_cli.rs:45-53`). `NoopExecutor::execute` returns
`Err(RuntimeError::UnsupportedTaskKind)`
(`crates/eggsec-cli/src/daemon_cli.rs:584-607`, `:605`).

The advertisement survives because `DaemonHost::new` sets
`executor_is_noop: false` (`crates/eggsec-daemon/src/host.rs:104`) and builds
its runtime with `RuntimeConfig::default()` (`:91`), whose capabilities are
`daemon_conservative()` (`crates/eggsec-runtime/src/capabilities.rs:33-37`, `:45`).
Only `DaemonHost::new_noop` sets `executor_is_noop: true`
(`crates/eggsec-daemon/src/host.rs:123`) and answers the `Capabilities`
command with an **empty** task-kind list (`crates/eggsec-daemon/src/host.rs:386-393`).

The GUI's existing no-op detection
(`crates/eggsec-gui/src/connection.rs:177-179`) therefore returns **false** for
this daemon: the GUI would offer a submit button that always fails. The
failure arrives as `ErrorCode::InvalidRequest`
(`crates/eggsec-daemon/src/host.rs:695-706`). **M006 must handle this.** The
GUI cannot rely on the capability list alone to decide "this daemon can work".

**At the current baseline this is no longer possible.** The CLI's rejecting
`NoopExecutor` was deleted — the CLI now hosts `EggsecRuntimeExecutor`
(`crates/eggsec-cli/src/daemon_cli.rs`) — and the daemon's published list is
now *derived* from the executor's own declaration intersected with the
conservative policy (`crates/eggsec-daemon/src/host.rs:99-108`). The
hand-maintained `executor_is_noop` field is gone. A rejecting executor
declares no capabilities, so it advertises none, and the GUI's existing
empty-list check (`crates/eggsec-gui/src/connection.rs:177-179`) now catches
every remaining no-op case.

M006 therefore does not need a workaround for the trap. It needs the
coverage model to explain the kinds the conservative policy **withholds**
from an otherwise healthy daemon — ten of the thirty canonical kinds — so
that "not advertised" is a reported reason rather than a silent omission.

### 3.7 The engine's feature set, and what a stock daemon actually enables

`crates/eggsec/Cargo.toml:291-450` declares the engine's features;
`default = []` (`:292`), so **every** feature is opt-in. The compile-time
registry `ALL_FEATURES` (`crates/eggsec/src/config/feature_registry.rs:110`,
populated by the `feature_registry!` macro at `:179`) holds **50 entries** (49
features plus the `full` aggregate), each with a `cfg!()`-derived `enabled`
flag. The `full` aggregate is a **curated 28-member** list (`FULL_MEMBERS`,
`crates/eggsec/src/config/feature_registry.rs:481-510`), not "everything".

> **Corrected at the current baseline** (was: "A `full-executor` daemon links
> the engine with **only** the `cli` feature … A stock `eggsec-daemon
> --full-executor` will deny all 8 with `FeatureMissing`"). Milestone 008
> replaced that measurement. `full-executor` is now the deprecated alias of
> the `service` profile, which enables **20** engine features
> (`crates/eggsec-daemon/Cargo.toml:25,52`), and the `service` profile
> includes every feature the conservative capability set needs — `nse`,
> `advanced-hunting`, `headless-browser`, `compliance`, `vuln-management`,
> `database`, `external-integrations`, and `finding-workflow` are all in
> `DAEMON_SERVICE_PROFILE`
> (`crates/eggsec/src/config/daemon_profile.rs`). A `service`-profile daemon
> therefore denies **none** of the 20 conservative capabilities for a missing
> feature.

What remains true, and is the reason milestone 006 still has work to do, is
that the daemon's **published capability set is not the canonical task-kind
set**. The daemon publishes the conservative 20, while the catalog describes
30. The ten withheld kinds are withheld by *policy*, not by a missing
feature, and the ten are exactly the ones whose capability is not in the
conservative set — `stress-test`, `resume`, `packet-capture`, `traceroute`,
`packet-send`, `wireless`, `wireless-active`, `db-pentest`, `intercept`,
`c2`.

The feature-gated-unavailability problem therefore survives, but for a
different reason and with different numbers: the client must learn which
kinds the daemon withholds, and — for a daemon whose feature set is
published — whether the withholding is a policy decision or a missing
feature.

### 3.8 Report data does not cross the wire

**The daemon produces no reports.** The protocol has no report command — the
`ClientCommand` variants are `Health`, `Capabilities`, `DeclareClient`,
`CreateSession`, `ListSessions`, `GetSnapshot`, `GetTaskResult`, `SubmitTask`,
`CancelTask`, `CancelActive`, `Subscribe`, `CloseSession`, `ApprovePolicy`,
`ListPersistedSessions`, `GetPersistedSnapshot`, `DescribeOperations`
(`crates/eggsec-daemon-protocol/src/protocol.rs:114-202`).

`TaskResultEnvelope.payload` is **always** `serde_json::json!({})` and
`artifacts` is **always** empty:

```rust
TaskResultEnvelope {
    kind,
    summary,
    payload: serde_json::json!({}),
    artifacts: vec![],
}
```

(`crates/eggsec/src/dispatch/canonical_execution.rs:1111-1116`.) The engine's
executor produces exactly that envelope
(`crates/eggsec/src/runtime_bridge/executor.rs:136-138`), so the report body is
discarded at the engine boundary.

**This is a pinned contract.** `crates/eggsec/tests/runtime_contract_closure.rs:618-619`
asserts it for every result variant:

```rust
assert_eq!(envelope.payload, serde_json::json!({}));
assert!(envelope.artifacts.is_empty());
```

`GetTaskResult` — the daemon's durable result read
(`crates/eggsec-daemon/src/host.rs:581-620`) — returns the stored
`TaskOutcome` (`crates/eggsec-daemon/src/host.rs:34-66`), which is that same
empty envelope.

**There is no general `PipelineReport → ScanReportData` bridge.** Per-domain
bridges exist: evasion (`crates/eggsec/src/evasion/mod.rs:788`), postex
(`crates/eggsec/src/postex/mod.rs:394`), wireless
(`crates/eggsec/src/wireless/mod.rs:608`), wireless-active
(`crates/eggsec/src/wireless/active/mod.rs:116`), plus mobile, db-pentest, and
proxy/intercept (all in
`crates/eggsec/src/commands/handlers/report.rs:215-283`). None covers the
scan/fuzz/pipeline families.

The consequence: **the GUI cannot obtain report data over the wire today**, and
M006 cannot ship report export without solving it. The additive read path
(Work package D) is the only in-scope route; filling the envelope's payload is
not additive and is a stop condition.

### 3.9 The report renderers, and the HTML constraint

`eggsec-output` depends **only** on `eggsec-core` + `eggsec-report-model`
(plus `serde`, `serde_json`, `chrono`, `rustc-hash`, `quick-xml`,
`unicode-normalization`, `lru`, `uuid`, `hostname`, and `tokio` with
`io-util`) — `crates/eggsec-output/Cargo.toml:16-32`. **There is no engine
edge**, so the GUI may depend on it without violating guard 155, but it does
widen the GUI's closure (§6).

| Format | Function | Location |
|---|---|---|
| JUnit XML | `convert_to_junit` | `crates/eggsec-output/src/convert.rs:19` |
| SARIF | `convert_to_sarif` | `crates/eggsec-output/src/convert.rs:53` |
| HTML | `convert_to_html` | `crates/eggsec-output/src/convert.rs:82` |
| Markdown | `convert_to_markdown` | `crates/eggsec-output/src/convert.rs:148` |
| CSV | `convert_to_csv` | `crates/eggsec-output/src/convert.rs:190` |
| JSON | `convert_to_json` | `crates/eggsec-output/src/convert.rs:209` |

The CLI's path is `eggsec report convert`
(`crates/eggsec/src/commands/handlers/report.rs:6-49`): it reads a
`ScanReportData` JSON and renders to `-o` or stdout, with the same six
formats.

**PDF lives only in the engine.** `crates/eggsec/src/output/pdf.rs:22-26` —
`PdfGenerator::generate_report` is `#[cfg(feature = "pdf")]`. The GUI cannot
produce PDF without linking the engine. PDF is out of scope.

**The HTML renderer cannot be embedded as-is.**
`crates/eggsec-output/src/html.rs:9-10` emits:

```html
<script src="https://cdn.jsdelivr.net/npm/chart.js"></script>
```

The GUI's CSP (`crates/eggsec-gui/tauri.conf.json:23`) sets
`default-src 'self'; script-src 'self'; …` — no remote origin is permitted,
and guard 158 (`scripts/check-architecture-guards.sh:4708-4766`) enforces that
mechanically. In addition, the renderer emits no `<canvas>` element (so the
chart never draws anyway) and `frame-src 'none'` forbids an iframe. The
current `eggsec-output` HTML renderer **cannot** be used as-is for the GUI's
embedded HTML display.

### 3.10 A working export implementation to model from

The TUI already exports reports (`crates/eggsec-tui/src/app/export.rs`). Its
documented lessons carry over: the extension map is duplicated and
error-prone (the TUI's `get_export_extension` maps `Junit` → `"xml"` at
`crates/eggsec-tui/src/app/mod.rs:427-438`, while the export module keeps its
own list `["html", "md", "sarif", "junit", "xml", "json"]` at
`crates/eggsec-tui/src/app/export.rs:10` and documents the `.xml`-vs-`junit`
stripping bug that the list exists to fix, `:5-10,39-49`); every failure mode
is an `Err`, so a broken conversion can never be written out and then reported
as a success (`:51-70`); and an existing export is **backed up**, not
overwritten, with a non-regular-file destination refused (`:78-110`).

The GUI MUST NOT introduce a second divergent extension map. One map, in one
place, in the GUI crate.

### 3.11 Known gaps at the baseline

- No coverage model: the GUI has no notion of "all 30 variants", only the
  catalog rows it happens to receive.
- No feature-availability resolution: `unavailable_reason`
  (`crates/eggsec-gui/src/commands.rs:458-470`) has exactly two reasons.
- No daemon lifecycle: the GUI "never spawns, supervises, or stops a daemon"
  (`crates/eggsec-gui/src/connection.rs:3-7`,
  `crates/eggsec-gui/src/shell.rs:13-25`).
- No report export at all: no `eggsec-output` dependency, no export command, no
  report view.
- The socket-path duplication is documented incorrectly in two places (§3.2).

### 3.12 Known pre-existing failures (inherited, not attributable to M006)

These are **inherited** at the baseline. M006 MUST NOT silently widen its
scope to fix them, and MUST NOT claim them as M006 regressions.

| Failure | Evidence | Blocks |
|---|---|---|
| `make check-deps` fails on advisories. **Not all of them are unrelated to the GUI.** The engine-side entries are `zip 2.4.2` (RUSTSEC-2026-0367, -0368, -0369 — three, not two) and the `rustls 0.21.12` family via `tokio-rustls 0.24.1` → `tiberius 0.12.3` → `eggsec` (not `tiberius` → `sqlx-mysql`; `sqlx-mysql` is a sibling under `sqlx`). **Seven advisory entries are reachable only through the GUI's Tauri closure** and are therefore attributable to `crates/eggsec-gui`: `glib 0.18.5` (RUSTSEC-2024-0429), `glib-macros 0.18.5` → `proc-macro-error 1.0.4` (RUSTSEC-2024-0370), and the five `unic-*` crates (RUSTSEC-2025-0081, -0075, -0080, -0100, -0098). A further four (`rustls 0.23.45`, `rustls-webpki 0.103.15`, `webpki-root-certs 1.0.9`, `tokio 1.53.1`) are **shared** with the engine but are attributed by `cargo deny` to a chain that passes through `eggsec-gui` → `tauri` → `reqwest`, because `cargo deny` resolves over the lockfile graph while the GUI's feature-resolved build declines Tauri's `rustls-tls` feature. `bans ok, licenses ok, sources ok`. | `Cargo.lock` pins `zip 2.4.2` (`:11488-11489`) and `rustls 0.21.12` (`:7596-7597`); `deny.toml:25-37` records ignores for the *webpki* advisories on that path but not for the `zip` triple; `plans/implementation/desktop-gui-frontend/007-packaging-and-platform-qualification.md` §3.9 carries the full per-advisory table | the `make check` contract, and therefore M6's closure |
| `cargo check --workspace --all-features` fails with `E0616: field 'url' of struct 'WebSocketFuzzer' is private` | `WebSocketFuzzer.url` is `pub(crate)` in `crates/eggsec-payloads/src/websocket.rs:41-45`, but the engine reads it at `crates/eggsec/src/fuzzer/advanced.rs:446` | `make check-features-individual`, which the roadmap requires before M6 closure (`plans/subsystems/desktop-gui-frontend-roadmap.md:840-843`) |
| `make check-python` was not run (needs `.venv-ci/`) | — | nothing in M6; recorded for completeness |

Recording a `deny.toml` exception is a **maintainer decision** (owner +
review-by date), not an implementation task M006 may take on itself.

## 4. Invariants that must not regress

- **The engine stays behind the daemon protocol.** The GUI MUST NOT depend on
  `eggsec`, `eggsec-daemon`, `eggsec-cli`, or `eggsec-tui` (guard 155,
  `scripts/check-architecture-guards.sh:4768-4834`).
- **`EnforcementContext::evaluate()` remains the mandatory pre-dispatch
  gate.** The GUI MUST NOT import `EnforcementContext`,
  `EnforcedDispatcher`, `Scope::is_target_allowed`, or `tool.execute()`.
- **The GUI never widens scope.** A form field is not a scope rule.
- **`OperationMetadata` remains the single source of truth for operation
  policy.** The GUI renders what the daemon publishes; it MUST NOT compute
  risk, exposure, or feature requirements itself.
- **The published schema and catalog are descriptive, never authoritative.**
  The catalog's `required_features` is a *requirement list*, not a state
  report; the GUI MUST NOT infer "the feature is on" from the catalog alone.
- **Strict surfaces fail closed.** The GUI is manual
  (`RuntimeSurface::DesktopGuiManual`), but a manual GUI still gets no wider
  scope than the operator supplied.
- **The daemon protocol changes additively only.** No existing `ClientCommand`
  or `ServerMessage` shape changes. The pinned empty-payload envelope contract
  (`crates/eggsec/tests/runtime_contract_closure.rs:618-619`) MUST stay green.
- **Ring-only TLS everywhere.** The GUI performs no outbound HTTP of its own.
- **Bounded concurrency and bounded retention.** The GUI MUST NOT retain an
  unbounded event log, an unbounded report store, or an unbounded report
  payload.
- **Never `let _ =` or `filter_map(|e| e.ok())`** — log via `tracing`.
- **Spawned tokio tasks carry a 30–300s timeout wrapper**, except a task that
  is long-lived by nature — in which case the exemption and its reason MUST be
  documented.
- **Cancellation goes through `eggsec-runtime::race_with_cancel`**
  (`crates/eggsec-runtime/src/cancel.rs:27-43`). The GUI MUST NOT hand-roll a
  `tokio::select!` on a cancellation token in adapter code.
- **The shell framework stays inside `crates/eggsec-gui`** (guard 156).
- **The GUI declares no second TLS stack** (guard 157).
- **The CSP names no wildcard source and no remote origin** (guard 158).
- **No per-operation form code** (guard 159) — M006's coverage model MUST NOT
  become a hand-written per-operation table.
- **Bounded event retention** (guard 160).
- **The frontend keeps zero npm dependencies and no build step.** M006's
  export view MUST NOT introduce one.
- **The `make check` contract** — the GUI crate still compiles in the
  `--no-default-features` profile (`crates/eggsec-gui/Cargo.toml:28`).

## 5. Scope

### In scope

- A **coverage model** in the GUI: for every published task kind, the variant,
  its `operation_id`, its `capability_name`, the catalog's
  `required_features`, and the GUI's reachability verdict — plus a coverage
  test that pins the exact multiplicities.
- **Availability resolution**: an additive read path publishing the connected
  daemon's enabled features, a denial-driven fallback that turns a
  `FeatureMissing` denial into a clear unavailability report, and the
  hide-or-report behaviour the roadmap requires.
- **Daemon lifecycle**: an explicit decision, an opt-in operator-supplied
  daemon executable path enabling spawn-and-supervise, a pre-flight liveness
  check, a bounded readiness wait, the no-op-executor trap, and clean
  teardown.
- **Report data acquisition** through an additive read path.
- **Report export** through `eggsec-output`, matching the CLI's artifacts, with
  an embedded HTML display that satisfies the CSP.
- A **coverage test** asserting every `TaskKind` variant is reachable.
- **The `DesktopGuiManual` permissive-set fix** (user-directed; work package
  G). `crates/eggsec/src/runtime_bridge/executor.rs` treats only `CliManual`
  and `TuiManual` as permissive surfaces, so a desktop-GUI session whose
  scope is not explicit takes the strict branch and fails closed. The
  desktop GUI is a permissive manual surface under ADR-0007, and every GUI
  task dispatched through the daemon-hosted executor is affected, so the fix
  is a precondition of this milestone's own end-to-end verification.
- New guards and the documentation updates in §12.

### Explicitly out of scope

- **PDF export.** It lives only in the engine
  (`crates/eggsec/src/output/pdf.rs:22-26`, `#[cfg(feature = "pdf")]`). The GUI
  MUST NOT link the engine to produce it.
- **Changing the pinned empty-payload envelope contract.** That is not
  additive and requires an ADR (§14).
- **A general `PipelineReport → ScanReportData` bridge for every domain.**
  M006 needs the bridge for the report-producing families it exports; a
  workspace-wide bridge for all 30 variants is a separate corrective
  milestone.
- **Collapsing the socket-path duplication** (§3.2). Six literals exist; M006
  records the count correctly and adds no seventh. Collapsing them is a
  separate corrective milestone.
- **Fixing the inherited `--all-features` compile error or the inherited
  `make check-deps` advisory failures** (§3.12). Both are recorded as stop
  conditions for the exit conditions they block.
- **A strict or headless GUI mode** — an ADR decision point
  (`plans/subsystems/desktop-gui-frontend-roadmap.md:861-864`).
- **Interactive policy approval.** `ApprovePolicy` stays unwired; the GUI
  MUST NOT offer an approval affordance.
- **Per-operation bespoke forms or result visuals.** M005's engine stays; M006
  adds reachability, not per-operation code.
- **Engine-side validation changes.**
- **Binary distribution, signing, notarization, installers** — milestone 007
  and the separate distribution workstream.
- **A new daemon process-management facility for other frontends.** The TUI's
  embedded fallback (`crates/eggsec-tui/src/app/mod.rs:378-412`) is unchanged.

## 6. Required production changes

### Core/domain

- `crates/eggsec-gui/src/coverage.rs` — **new**. The coverage model:
  - `TaskCoverage` — one record per published task kind: `variant`,
    `operation_id`, `capability_name`, `required_features`, and the
    reachability verdict.
  - `Reachability` — the verdict enum: `Available`, `HiddenByDaemon` (the
    daemon does not advertise the capability), `HiddenByFeature` (the daemon
    advertises the capability but the feature is off), `NotExposed` (the
    operation is not `manual_exposable`), `Unverified` (the daemon's feature
    set is unknown — the fallback state).
  - `coverage_for(catalog, capabilities, enabled_features) -> Vec<TaskCoverage>`
    — the single derivation, driven entirely by published data.
- `crates/eggsec-gui/src/availability.rs` — **new**. The availability
  resolver: the join of the catalog's `required_features`, the daemon's
  advertised capabilities, and the daemon's published enabled-feature set,
  producing the reachability verdict and the operator-facing reason.
- The model MUST NOT contain a hardcoded list of variant names — the 30
  variants are read from the published catalog
  (`DescribeOperationsResponse.task_kinds`,
  `crates/eggsec-daemon-protocol/src/operations.rs:89`), exactly as guard 159
  requires of the form engine — and the GUI MUST NOT re-derive
  `operation_id` or `capability_name`, which are published per task kind
  (`crates/eggsec-daemon-protocol/src/operations.rs:75,77`) and are the
  engine's own tables.

### Storage and migrations

- **None.** The GUI owns no authoritative storage. The task list is a bounded
  in-memory view; the export destination is an operator-chosen path on the
  operator's filesystem, written by the GUI's Rust side (the Tauri `fs` plugin
  is **not** enabled — `crates/eggsec-gui/capabilities/default.json:6` — and
  MUST NOT be enabled for this).
- No data migration is required, because no milestone changes an existing
  persisted representation (`plans/subsystems/desktop-gui-frontend-roadmap.md:739-745`).
- The daemon-side report store (Work package D) is **engine/daemon-owned
  runtime state**, bounded and cleared with its session — not a persisted
  representation, so it needs no migration.

### Protocol and DTOs

Three additive changes, and one hard prohibition.

**(P1) The daemon's enabled-feature set.** `DaemonCapabilities` gains one
field (`crates/eggsec-daemon-protocol/src/protocol.rs:46-51`):

```rust
#[serde(default)]
pub enabled_features: Option<Vec<String>>,
```

> **Resolved in §0, ambiguity 1.** The shape is `Option<Vec<String>>`, not
> `Vec<String>`. `None` means **unknown**; `Some(vec![])` means an engine is
> linked with no features enabled. The distinction is the whole point of the
> field: an empty vector would be ambiguous, and a client that read it as
> "unknown" would report the wrong reason for an unavailable operation.

This is strictly additive in both directions: a **new** daemon's capabilities
decoded by an **old** client works, because serde ignores unknown fields by
default; an **old** daemon's capabilities decoded by a **new** client works,
because `#[serde(default)]` supplies `None`. The protocol version therefore
stays at `DAEMON_PROTOCOL_VERSION` — the same reason milestone 003 kept it at 2
when `ClientKind::Gui` was added. The GUI MUST NOT abort its connection when
the field is absent: a defaulted field can be absent on an old daemon, and
the GUI degrades to "availability unverified" instead.

The daemon's value comes from the executor, which gains one **defaulted**
method (`crates/eggsec-runtime/src/runtime.rs`):

```rust
/// The engine features compiled into this executor's build.
///
/// `None` means unknown: the executor links no engine, or does not publish
/// the fact. `Some(vec![])` means an engine is linked with no features
/// enabled — a different, knowable fact.
fn enabled_features(&self) -> Option<Vec<String>> { None }
```

A defaulted method is additive at the Rust level — every existing
implementation (`NoopExecutorStub` at `crates/eggsec-daemon/src/host.rs`,
the daemon binary's fallback executor) keeps compiling unchanged, and the
no-op executor's `None` is exactly right: it links no engine, so its
feature set is unknown. The engine's `EggsecRuntimeExecutor` overrides it
with the engine's own authoritative compile-time registry
(`crates/eggsec/src/config/feature_registry.rs`, whose entries each carry a
`cfg!()`-derived `enabled` flag).

**(P2) The report read path.** One new read-only command and one new
response:

```rust
ClientCommand::GetTaskReport { request_id, session_id, task_id }
ServerMessage::TaskReport { request_id, session_id, task_id, report }
```

> **Corrected at the current baseline.** The plan said an old daemon
> answers `ErrorCode::UnsupportedCommand` for a command it does not know.
> It does not: the daemon deserializes the line straight into
> `ClientCommand`, so an unknown discriminant is a *deserialization
> failure*, answered with `ErrorCode::InvalidRequest` **and an empty
> `request_id`** (`crates/eggsec-daemon/src/server.rs`). Because the
> frontend client routes responses by request id, the pending request is
> never resolved and the client **hangs for its 30-second timeout**
> instead of receiving an error.
>
> M006 MUST fix that, because M006 is the milestone that adds a command
> older daemons will not know. The daemon MUST distinguish "this client
> sent a command this daemon does not know" from "this client sent a
> malformed command", answer the former with `ErrorCode::UnsupportedCommand`
> **and the request id echoed back**, and answer the latter with
> `ErrorCode::InvalidRequest`. The protocol therefore also gains a way for
> the daemon to enumerate the commands it knows
> (`ClientCommand::command_names()`).

The report document itself is carried as `serde_json::Value` on the wire
rather than as a typed report-model struct, so the protocol crate keeps no
report-model dependency; the GUI deserializes it into the canonical
`ScanReportData`. The engine produces it through a **pure** bridge from the
completed task result, and it is held in a **bounded** store
(`eggsec_runtime::RuntimeReportStore`) so a long-lived daemon cannot
accumulate one report document per task for the life of the process. The
store lives behind the executor and is read through the runtime, so the
daemon never learns a report's domain semantics — it only serves what the
engine produced.

The read path MUST NOT touch `TaskResultEnvelope` (see P-HARD). A new client
MUST treat `ErrorCode::UnsupportedCommand` and a "no report for this task"
answer as a graceful degradation (no export), never as a launch failure.

**(P3) The frontend-client accessor.** `DaemonRuntimeClient` gains an inherent
method — **not** a `RuntimeClient` trait method, mirroring milestone 004's
decision for `describe_operations`. The trait
(`crates/eggsec-frontend-client/src/lib.rs:66-88`) stays frontend-neutral and
unchanged.

**(P-HARD) The prohibition.** `TaskResultEnvelope.payload` and
`TaskResultEnvelope.artifacts` MUST continue to be `json!({})` and `vec![]`
for every result variant, and `crates/eggsec/tests/runtime_contract_closure.rs:618-619`
MUST stay green. Filling the payload is **not additive** and is a stop
condition (§14).

**The GUI's dependency on `eggsec-output`.** `crates/eggsec-gui/Cargo.toml`
gains `eggsec-output`. This is permitted — `eggsec-output` has no engine edge
(`crates/eggsec-output/Cargo.toml:16-32`) — but it widens the GUI's closure
with `eggsec-core` (→ `subtle`, `zeroize`), `chrono`, `quick-xml`,
`unicode-normalization` (→ `tinyvec`), `lru`, `hostname`, `rustc-hash`, and
`tokio`/`io-util`. `deny.toml` is a **closed** license allow-list
(`deny.toml:39-56`); every one of those crates' licenses must clear it, and
`make check-deps` fails closed if any does not. The implementer MUST run
`make check-deps` and record the outcome rather than assume the closure is
clean. No new **tokio feature** is required by `eggsec-output`; its `io-util`
need comes in through the dependency.

### Runtime and concurrency

- The GUI's availability state is a **bounded, in-memory** projection of the
  daemon's published catalog plus the daemon's enabled-feature set. It is
  recomputed when the daemon publishes new data, never accumulated.
- The export path is a **finite** operation and its spawned task MUST carry a
  30–300s timeout wrapper.
- The spawn-and-supervise path (Work package C) owns one child process at a
  time. The GUI MUST NOT spawn more than one.
- Every spawned task's failure is reported through `tracing`. No `let _ =`, no
  `filter_map(|e| e.ok())`.
- Cancellation of an in-flight export goes through
  `eggsec-runtime::race_with_cancel` (`crates/eggsec-runtime/src/cancel.rs:27-43`).

### Frontend or operator surface

- `crates/eggsec-gui/src/commands.rs` — additional IPC commands:
  `coverage()` (the per-variant reachability table with each variant's verdict
  and reason), `export_report(task_id, format, path)`, and `daemon_status()`
  (the daemon's lifecycle state and, when the operator supplied a daemon
  binary, the ability to start/stop the supervised child).
- `crates/eggsec-gui/src/export.rs` — **new**. The export module:
  - `ReportFormat` — the six formats the CLI supports, minus PDF, plus the
    extension map. **One** map, in this module.
  - `export(report, format, path) -> Result<ExportOutcome, ExportError>` —
    renders through `eggsec-output`'s `convert_to_*` and writes the artifact.
  - The write follows the TUI's discipline: back up an existing regular file
    to `<name>.bak` before overwriting, refuse a non-regular-file destination,
    and make every failure an `Err` so a partial write is never reported as a
    success (`crates/eggsec-tui/src/app/export.rs:51-110`).
  - The path MUST come from the operator. The GUI MUST NOT write outside the
    operator-chosen path and MUST NOT choose a default output directory.
- `crates/eggsec-gui/src/daemon.rs` — **new**. The daemon lifecycle module:
  the pre-flight liveness check, the spawn, the bounded readiness wait, the
  supervision loop, and the teardown (Work package C).
- `crates/eggsec-gui/frontend/app.js` — the coverage table (all 30 rows with
  the verdict and the reason), the export controls, the export result view, and
  the daemon status panel. **Zero npm dependencies.** No per-operation branch.
- The webview MUST receive the coverage table as **data** and render it. It
  MUST NOT contain a per-variant table, a per-variant branch, or a hardcoded
  field list.
- **Embedded HTML display — corrected by a third finding.** The plan's option 1
  assumed that a self-contained HTML rendering could be *embedded* inside the
  GUI webview. It cannot. The GUI's CSP
  (`crates/eggsec-gui/tauri.conf.json:23`) is
  `default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self' data:; connect-src 'self' ipc: http://ipc.localhost; object-src 'none'; frame-src 'none'; base-uri 'none'`, and
  that forbids the document three ways: `frame-src 'none'` rules out an
  iframe, `script-src 'self'` means an inline `<script>` does not execute, and
  `style-src 'self'` means an inline `<style>` does not apply. The engine's
  HTML document carries an inline `<style>` block and an inline `<script>`
  block, so a "self-contained" rendering of it still cannot be rendered
  faithfully inside the webview.
  - The CSP MUST NOT be relaxed to work around this. The report content is
    **attacker-influenced** — banners, service names, and finding text come
    from the target — and the CSP is exactly the control that stops that
    content from doing anything. No `'unsafe-inline'`, no `frame-src` change,
    no `dangerousDisableAssetCspModification`.
  - The resolved shape is **option 2**: the GUI renders the report's
    *structured content* in its own DOM, from the same `ScanReportData`, with
    the GUI's own stylesheet and its own escaping. No engine HTML is injected
    into the webview. The HTML format is therefore **export-only**.
  - `eggsec-output` gains the opt-in `convert_to_html_self_contained` mode
    anyway: it drops the remote chart-library reference, it leaves the
    CLI's default `convert_to_html` byte-for-byte unchanged (verified by
    comparing the rendered output against a pristine worktree at the
    baseline commit), and it is a tested capability in its own right. The
    GUI does not consume it, because the CSP makes document embedding
    unavailable. That is recorded as a finding, not as a failure.
- The **exported file** for HTML MUST be the CLI's HTML, byte-for-byte,
  whatever the embedded display does. The embedded view and the exported file
  are two different things, and §13 says so.

### Security and authorization

- The IPC surface stays narrow. `export_report` takes a task id, a format, and
  an operator-chosen path. It MUST NOT accept a pre-built report, and MUST
  NOT accept a surface or scope parameter from the webview.
- The GUI MUST NOT filter operations by its own policy logic. Exposure comes
  from `OperationDescription.manual_exposable`; the feature verdict comes from
  the daemon's published enabled-feature set plus the denial-driven fallback.
- The GUI MUST NOT authorize anything. A hidden operation is a **presentation**
  decision derived from published state; the engine's `EnforcementContext`
  remains the only pre-dispatch gate.
- The GUI MUST NOT persist credentials in its own state or logs. An exported
  report contains scan findings, not credentials; the GUI MUST NOT log the
  report body.
- The GUI MUST NOT perform outbound HTTP. The report arrives over the daemon's
  Unix socket; the artifact is written locally.
- The spawn route (Work package C) executes a **binary the operator named**.
  The GUI MUST NOT search `PATH` for a daemon, MUST NOT auto-download one,
  and MUST NOT execute anything the operator did not name. The child's argv
  MUST be constructed as a vector (never a shell string), so no shell is
  involved and no injection surface exists.
- The GUI MUST NOT widen scope. A spawned daemon inherits the operator's
  environment; the GUI MUST NOT inject scope into it.

### Documentation and static guards

- Two new guards:
  - **161 — the GUI's coverage model is derived, not transcribed.** The GUI's
    production sources MUST NOT contain a hardcoded list of the 30 variant
    names or of operation ids. The check reads the variant list from the
    runtime, exactly as guard 159 does
    (`scripts/check-architecture-guards.sh:4783-4834`).
  - **162 — the GUI's exported HTML is self-contained.** No artifact the GUI
    writes for display may reference a remote origin. The check asserts that
    the HTML the GUI produces contains no `http://` or `https://` source
    reference, in the same spirit as guard 158.
- Each new guard MUST be **demonstrated to fail** under a deliberate violation
  and to pass again after restore.
- `architecture/desktop_gui.md` — the coverage model, the availability
  resolution, the daemon lifecycle, the export path, and the corrected
  socket-duplication count.
- `architecture/overview.md`, `architecture/daemon.md`,
  `architecture/frontend_client.md`, `architecture/runtime.md` — as applicable.
- `docs/ARCHITECTURE.md`, `docs/ARCHITECTURE_INVARIANTS.md`,
  `docs/CI_ARCHITECTURE_GUARDS.md`, `docs/VERIFICATION.md` — as applicable.
- `docs/DEPENDENCY_EXCEPTIONS.md` — only if the widened closure needs an
  exception, with an owner and a review-by date.
- `.opencode/skills/eggsec-gui/SKILL.md` — the coverage model, the availability
  resolver, the daemon lifecycle, and the export path.
- `plans/registry.md` and the subsystem roadmap — the milestone status.

## 7. Ordered work packages

### Work package A — The coverage model

Intent:

Give the GUI a single, derived, per-variant reachability record for all 30
`TaskKind` variants, and the coverage test that proves it.

Required changes:

1. `crates/eggsec-gui/src/coverage.rs` — new module, as described in §6
   ("Core/domain"). It reads **only** published data:
   `DescribeOperationsResponse.task_kinds` for the 30 variants, their
   `operation_id` and their `capability_name`
   (`crates/eggsec-daemon-protocol/src/operations.rs:71-81`);
   `DescribeOperationsResponse.operations` for `required_features` and
   `manual_exposable` (`:30-62`); the daemon's
   `RuntimeCapabilities.task_kinds` for the advertised capability names
   (`crates/eggsec-runtime/src/capabilities.rs:23-29`); and the daemon's
   published enabled-feature set (Work package B).
2. The coverage test MUST assert the **exact multiplicities**, read from the
   published artifact rather than from a transcribed list: **30** task kinds;
   **30** distinct `capability_name` values; **26** distinct `operation_id`
   values; the four collapses are exactly `Resume → pipeline`,
   `{PacketCapture, PacketTraceroute, PacketSend} → packet`, and
   `WirelessActive → wireless`; and `operation_id != capability_name` for
   exactly **10** of the 30 variants.
3. A second test MUST assert the **join** is sound: for every variant, the
   `capability_name` is the key the daemon's capability list is searched by
   (`crates/eggsec-runtime/src/request.rs:556-561`), and every variant's
   `operation_id` resolves to exactly one catalog operation.
4. The coverage model MUST carry the reason a variant is unreachable, in
   operator-readable words — the same shape as the existing
   `unavailable_reason` (`crates/eggsec-gui/src/commands.rs:458-470`), extended
   rather than replaced.

Acceptance evidence:

- A test that fails if a 31st `TaskKind` variant is added without the GUI
  covering it.
- A test that fails if the daemon's capability list stops being joined on
  `capability_name`.
- The four collapses and the 10/30 divergence are pinned by name, not by
  count alone, so a *different* collapse cannot pass unnoticed.

### Work package B — Availability resolution

Intent:

Let the GUI say "this daemon cannot run this" and "this daemon can run this,
but the engine feature is off" — separately — and hide or report accordingly.

Required changes:

1. **The additive read path (P1)** — §6, "Protocol and DTOs".
2. **The resolver.** `crates/eggsec-gui/src/availability.rs` computes, per
   variant:
   - **not exposed** — the catalog's `manual_exposable` is false. The variant
     is hidden. Unchanged from M005.
   - **capability not advertised** — the daemon's capability list does not
     contain the variant's `capability_name`. The variant is hidden, with the
     reason naming the capability.
   - **feature off** — the daemon advertises the capability but the feature it
     requires is **not** in the daemon's published enabled-feature set. The
     variant is hidden, and the reason names the feature.
   - **available** — the capability is advertised and the required features
     are all enabled.
   - **unverified** — the daemon's enabled-feature set is unknown (the
     fallback case). The variant is **shown** with an explicit "cannot verify"
     marker, because claiming it is unavailable would be a false statement.
3. **The mandatory denial-driven fallback.** Whether or not the read path
   exists on the connected daemon, the GUI MUST turn a `FeatureMissing`
   denial into a clear, non-opaque unavailability report. The denial arrives
   as `RuntimeErrorInfo` on `TaskFailed`, with the message
   `required feature '<X>' is not enabled`
   (`crates/eggsec-policy/src/decision.rs:921-930`) or as
   `ExecutionError::FeatureUnavailable { operation_id, feature }`
   (`crates/eggsec/src/dispatch/canonical_execution.rs:1795-1798`). The GUI
   MUST recognise both, record the variant as feature-blocked for the rest of
   the session, and re-render it as hidden with the feature named.
4. **Degradation.** A daemon too old to publish the enabled-feature set
   answers `UnsupportedCommand` or omits the field. The GUI MUST degrade to
   the `unverified` state — **not** to "everything is available" and **not** to
   "everything is unavailable". This is the M001 precedent
   (`crates/eggsec-daemon-protocol/src/protocol.rs:54-59,181`).
5. **The 8-of-20 case is handled by construction.** With the enabled-feature
   set published, a stock `--full-executor` daemon (engine features = `cli`
   only) makes `nse`, `hunt`, `browser`, `compliance`, `vuln`, `storage`,
   `integration`, and `workflow` resolve to **feature-off**, even though the
   daemon advertises all 8 capabilities (§3.7). A test MUST pin this.

Acceptance evidence:

- A test asserting the 8 advertised-but-feature-off variants resolve to
  feature-off against a daemon whose enabled-feature set is `{ cli }`.
- A test asserting the 10 capability-not-advertised variants resolve to
  capability-not-advertised.
- A test asserting that an old daemon (no enabled-feature set) degrades to
  `unverified`, and that no variant is hidden on the strength of missing data.
- A test asserting a `FeatureMissing` denial flips the variant to
  feature-blocked and the reason names the feature.

### Work package C — The daemon lifecycle

Intent:

Make the GUI's daemon-lifecycle behaviour **explicit**, and give the operator
a way to have the GUI bring up a usable daemon — without the GUI ever linking
the daemon crate or assuming a binary is on `PATH`.

Required changes:

1. **The decision, stated in the code and the docs.** The GUI's default
   remains today's behaviour: **connect to a running daemon**, and when the
   socket is unreachable, report precisely how to start one
   (`crates/eggsec-gui/src/connection.rs:66-74`). That satisfies the roadmap's
   second permitted option and MUST be preserved in the diagnostic.
2. **The opt-in spawn route.** A new GUI option, `--daemon-bin <PATH>`, names
   an operator-supplied daemon executable. When it is present and the socket
   is unreachable, the GUI may spawn and supervise that binary. When it is
   absent, the GUI MUST NOT look for a daemon anywhere — not on `PATH`, not
   next to its own executable, not in a well-known directory.
3. **The spawn arguments.** The child is started as
   `<daemon-bin> --full-executor --socket-path <socket>`, matching
   `daemon_start_command` (`crates/eggsec-gui/src/connection.rs:57-59`) and the
   daemon's own flags (`crates/eggsec-daemon/src/main.rs:18-44`). The argv MUST
   be a vector; **no shell string**, so no injection surface exists.
4. **The pre-flight liveness check.** Before spawning, the GUI MUST determine
   whether a daemon is already serving the socket path — because `run_server`
   unlinks the socket before binding
   (`crates/eggsec-daemon/src/server.rs:141-146`), a second daemon **steals a
   live socket**. The check MUST attempt a connection, not merely stat the
   socket file. If a daemon is already there, the GUI MUST connect to it and
   MUST NOT spawn.
5. **The readiness wait.** After spawn, the GUI MUST wait for the daemon to
   become connectable with a **bounded timeout** (a named constant in the
   30–300s band; the implementer picks and documents it). Readiness is
   determined by a successful connect + protocol-version check — the same
   check `GuiConnection::connect` performs
   (`crates/eggsec-gui/src/connection.rs:111-170`) — **not** by scraping the
   child's stdout for the `"Daemon listening on ..."` line
   (`crates/eggsec-daemon/src/server.rs:163-167`), because that string is a log
   format, not a contract. Scraping it MAY be used as a **diagnostic hint**,
   never as the readiness criterion.
6. **The child must not outlive the GUI by accident.** The GUI owns the child's
   lifetime: on exit, on an explicit stop, or on the readiness timeout, the
   GUI MUST terminate the child, using a bounded, escalating sequence — the
   daemon handles **SIGTERM** and ctrl_c
   (`crates/eggsec-daemon/src/main.rs:205-227`) and removes its socket on
   shutdown (`crates/eggsec-daemon/src/server.rs:196-200`); it does **not**
   handle SIGHUP, so the GUI MUST NOT send SIGHUP as its first signal. If the
   child does not exit within the escalation window, the GUI escalates to an
   unconditional kill and reports that it had to.
7. **The no-op-executor trap MUST be detected after connecting.** The
   pre-spawn check and the post-connect check are different: a daemon can be
   reachable and still be useless. The GUI's existing empty-list check
   (`crates/eggsec-gui/src/connection.rs:177-179`) catches
   `DaemonHost::new_noop` but **not** `eggsec daemon start` (§3.6). M006 MUST
   add a check that catches the 20-advertised/0-executable case. The plan
   requires the implementer to determine a **reliable** signal and to document
   it; the candidate signals are:
   - the daemon's published enabled-feature set being empty or absent **while**
     the capability list is non-empty (a `full-executor` daemon publishes a
     non-empty enabled set; a no-op daemon publishes none);
   - a probe submit of a trivially-scoped, harmless operation — **not**
     acceptable as the primary signal, because a probe submit is a real
     dispatch with side effects.
   The **first** candidate is the required primary signal; the second is only
   a last resort and MUST NOT be the default. If no reliable signal can be
   established, the GUI MUST report the daemon's identity and the exact start
   command, and MUST NOT offer a submit affordance it cannot back.
8. **`tokio::process` or `std::process`.** The GUI's tokio feature list has no
   `process` (`crates/eggsec-gui/Cargo.toml:48`). The workspace precedent for
   `tokio::process` is the engine's list, which carries **both** `process` and
   `signal` (`crates/eggsec/Cargo.toml:34`: `["rt", "rt-multi-thread", "net",
   "sync", "time", "macros", "fs", "process", "io-util", "io-std", "signal"]`).
   The plan's recommendation: add `"process"` and `"signal"` to the GUI's tokio
   feature list, matching the engine's precedent. The alternative —
   `std::process` with a blocking supervision thread — is legitimate but is a
   second, differently-shaped concurrency model in a crate that is otherwise
   tokio-native; if the implementer chooses it, the reason MUST be recorded.
   Adding a tokio feature is permitted by the workspace's Tokio baseline
   (`default-features = false` with an explicit per-crate feature list), but it
   MUST be named in the manifest change and visible in the diff.
9. **The GUI still links no daemon crate.** Spawning is not linking. Guard 155
   (`scripts/check-architecture-guards.sh:4768-4834`) MUST stay green, and the
   spawned path MUST be a process-spawn over a filesystem path — never a Rust
   dependency on `eggsec-daemon`.

Acceptance evidence:

- A test asserting the default path (no `--daemon-bin`) never spawns anything
  and produces the exact existing diagnostic
  (`crates/eggsec-gui/src/connection.rs:66-74`).
- A test asserting that a daemon already serving the socket path is **not**
  displaced by a spawn.
- A test asserting the readiness wait has a bounded timeout and reports a
  clear failure when the child never becomes connectable.
- A test asserting the GUI terminates a child it spawned, and that the first
  signal is not SIGHUP.
- A test asserting the 20-advertised/0-executable daemon is detected and
  reported, not offered as usable.
- A test asserting that no code path resolves a daemon binary from `PATH` or
  from a directory the operator did not name.

### Work package D — Report data acquisition

Intent:

Get a `ScanReportData` from a completed task over the wire, **without**
changing the pinned empty-payload contract.

Required changes:

1. **The constraint, restated as a requirement.** The engine's
   `task_result_envelope` produces `payload: json!({})` and `artifacts: vec![]`
   (`crates/eggsec/src/dispatch/canonical_execution.rs:1111-1116`), and
   `crates/eggsec/tests/runtime_contract_closure.rs:618-619` pins it. **The
   pinned contract MUST remain green.** Filling the payload is **not
   additive** and is an ADR-gated change (§14).
2. **The additive read path (P2)** — §6, "Protocol and DTOs".
3. **The engine side.** The engine must produce the report and hand it to the
   daemon by a route that does **not** touch the envelope. The plan's
   requirement, not its implementation:
   - a **pure** conversion from a completed result to a `ScanReportData` (the
     per-domain bridges exist — evasion
     `crates/eggsec/src/evasion/mod.rs:788`, postex
     `crates/eggsec/src/postex/mod.rs:394`, wireless
     `crates/eggsec/src/wireless/mod.rs:608`, wireless-active
     `crates/eggsec/src/wireless/active/mod.rs:116` — but there is **no
     general bridge**, and M006 adds one only for the families it exports);
   - a **bounded** store the daemon can read, owned by the daemon and written
     by the executor. "Bounded" is a hard requirement: a report store that
     grows with the number of completed tasks is the same unbounded-retention
     defect guard 160 exists to prevent, in a new place.
4. **The degradation.** An old daemon answers `ErrorCode::UnsupportedCommand`.
   The GUI MUST degrade to "no report available for this task" and MUST offer
   the export as unavailable — **not** a launch failure and **not** a silent
   empty report.
5. **The explicit prohibition.** `TaskOutcome` is a plain externally tagged
   serde enum (`crates/eggsec-runtime/src/event.rs:68-84`). **Adding a variant
   to it is NOT wire-additive** — an older client deserializing an unknown
   variant fails. The implementer MUST NOT add a `TaskOutcome` variant. The
   existing `TaskOutcome::Json(Value)` variant is wire-compatible, but the
   GUI's result rendering depends on the envelope's `kind` and `summary`
   (`crates/eggsec-ui-model/src/result_view.rs:9`), so replacing the envelope
   with a bare JSON outcome would be a **behaviour** change on every daemon
   client — out of scope.
6. **The escalation trigger.** If the report cannot reach the daemon without
   changing `TaskResultEnvelope.payload` or `.artifacts`, the implementer
   **stops and escalates** (§14). The escalation is an ADR under `plans/adrs/`,
   because the roadmap names "Any change to the daemon protocol that is not
   strictly additive" as a decision point requiring an ADR
   (`plans/subsystems/desktop-gui-frontend-roadmap.md:861-864`).

Acceptance evidence:

- `crates/eggsec/tests/runtime_contract_closure.rs` remains green, unmodified.
- A test asserting an old daemon's `UnsupportedCommand` degrades to "no
  report", not to an error screen.
- A test asserting the report store is bounded — a long session with many
  completed tasks retains at most the documented cap.
- A test asserting the report the GUI receives deserializes into
  `ScanReportData` (`crates/eggsec-report-model/src/report.rs:10-24`).

### Work package E — Report export

Intent:

Produce the same artifacts the CLI produces, through the existing
`eggsec-output` renderers, and show the HTML report inside the GUI.

Required changes:

1. `crates/eggsec-gui/Cargo.toml` gains `eggsec-output` (§6, "Protocol and
   DTOs").
2. `crates/eggsec-gui/src/export.rs` — the export module, as described in §6
   ("Frontend or operator surface"): one format enum, **one** extension map,
   the backup-then-write discipline, and every failure an `Err`.
3. **The format set** is the CLI's six — JSON, CSV, HTML, Markdown, SARIF,
   JUnit (`crates/eggsec-output/src/convert.rs:19,53,82,118,160,179`) — and
   **not** PDF (§5).
4. **The extension map** MUST be the single map, and it MUST resolve JUnit to
   `xml` — the TUI's own lesson
   (`crates/eggsec-tui/src/app/mod.rs:427-438`, and the bug it caused,
   `crates/eggsec-tui/src/app/export.rs:5-10`). The GUI MUST NOT copy the
   TUI's two-list arrangement.
5. **The artifact-equivalence test.** For a fixed `ScanReportData` fixture, the
   GUI's rendered bytes MUST equal the CLI's rendered bytes for each format.
   The CLI's reference implementation is
   `crates/eggsec/src/commands/handlers/report.rs:29-42`, which calls the same
   `eggsec-output` functions. The test MUST compare against the `eggsec-output`
   functions directly (the CLI is a thin wrapper over them) **and** MUST
   document that the comparison is against the shared renderer, not against a
   captured CLI output file — a captured file would go stale.
6. **The embedded HTML display** — the bounded options in §6 ("Frontend or
   operator surface"). Option 1 (an opt-in self-contained mode in
   `eggsec-output`) is preferred. The exported HTML **file** MUST be the CLI's
   HTML, byte-for-byte, in either option.
7. **The export destination is operator-chosen.** The GUI MUST NOT invent a
   default output directory, MUST NOT write outside the chosen path, and MUST
   refuse a destination that is not a regular file (following the TUI's
   discipline, `crates/eggsec-tui/src/app/export.rs:78-110`).
8. **The export is a finite operation** and its spawned task carries a 30–300s
   timeout wrapper.

Acceptance evidence:

- The artifact-equivalence test passes for all six formats.
- The artifact-equivalence test passes for all six formats: the exported
  file is byte-for-byte what the CLI's shared renderers produce.
- The **embedded** HTML the GUI renders for display contains no remote
  origin reference, and the self-containment guard (162) is green. The
  **exported** HTML file deliberately keeps the CLI's remote chart-library
  reference, because it is the CLI's artifact and is opened in the
  operator's browser.
- A test asserting an existing export is backed up rather than silently
  overwritten, and that a non-regular-file destination is refused.
- A test asserting no export is written when the renderer fails — a failed
  conversion is never reported as a success.

### Work package F — Guards, documentation, and the closure evidence pack

Intent:

Pin the milestone's contracts mechanically, and leave the repository
describing what was built.

Required changes:

1. Add guards 161 and 162 to `scripts/check-architecture-guards.sh`, in the
   style of guards 155–160. **Each MUST be demonstrated to fail under a
   deliberate violation and to pass again after restore.**
2. Update the documents listed in §12.
3. Update `.opencode/skills/eggsec-gui/SKILL.md` with the coverage model, the
   availability resolver, the daemon lifecycle, and the export path.
4. Update `plans/registry.md` and the subsystem roadmap's milestone table.
5. Record the **inherited** failures (§3.12) in the closure record, explicitly
   attributed to the baseline and not to M006.
6. Record which HTML option was taken (option 1 or option 2) and, if option 2,
   the artifact-equivalence caveat.

Acceptance evidence:

- The guards pass; the documents describe the GUI as it is.
- `python3 scripts/check_doc_references.py` reports no failure on any line the
  milestone changed.

### Work package G — The `DesktopGuiManual` permissive set (user-directed fold-in)

Intent:

Make the desktop GUI a permissive manual surface in the engine, as ADR-0007
and the long-term specification already say it is. Without this, every task
the GUI dispatches through the daemon-hosted executor is denied before it
runs, and none of this milestone's end-to-end behaviour can be observed.

Required changes:

1. In `crates/eggsec/src/runtime_bridge/executor.rs`, the scope-resolution
   helper that decides whether a surface gets the permissive default scope
   MUST treat `RuntimeSurface::DesktopGuiManual` exactly as it treats
   `RuntimeSurface::CliManual` and `RuntimeSurface::TuiManual`. The match
   currently names only those two, so a desktop-GUI session whose scope is
   not explicit falls into the strict branch and is denied with an empty
   scope.
2. The fix MUST NOT widen any other surface. In particular it MUST NOT
   make an agent surface permissive, and it MUST NOT make the strict
   surfaces (`Rest`, `Mcp`, `Ci`, `Agent`) permissive. The GUI is a
   **manual** surface; an agent surface is a different class and is not
   part of this fix.
3. Add a focused regression test in the engine that drives the scope
   resolution for a `DesktopGuiManual` session and asserts the permissive
   default, and a negative test that asserts a strict surface still takes
   the strict branch. The regression test MUST fail against the pre-fix
   code and pass after it.
4. The GUI's own coverage and end-to-end evidence in work packages A–E
   MUST be gathered **after** this fix, so that the observed behaviour is
   the fixed behaviour and the closure record does not have to caveat it.

Acceptance evidence:

- A `DesktopGuiManual` session with no explicit scope receives the
  permissive default scope rather than being denied.
- A strict surface session with no explicit scope is still denied.
- The GUI's end-to-end submission path succeeds against a
  `service`-profile daemon, which is only observable once this fix lands.

## 8. Failure, cancellation, restart, and contention semantics

- **The daemon is unreachable and no `--daemon-bin` was given.** The GUI
  reports the existing diagnostic verbatim
  (`crates/eggsec-gui/src/connection.rs:66-74`) and the window opens in a "not
  connected" state. The GUI MUST NOT retry in a tight loop — M005's existing
  rule (`architecture/desktop_gui.md:213`).
- **The daemon is unreachable and `--daemon-bin` was given.** The pre-flight
  check runs, the child is spawned, and the readiness wait begins with a
  bounded timeout. On timeout the GUI MUST terminate the child, report the
  timeout, and show the diagnostic. It MUST NOT leave an orphan.
- **The spawned child exits immediately, or dies after becoming ready.** The
  readiness wait sees the exit and reports the child's exit status and
  captured stderr, if any; the supervision loop detects a later death, reports
  "the daemon exited", and offers an explicit restart. A child that dies is
  reported as a failure, never as "the daemon is starting". The GUI MUST NOT
  silently reconnect to a dead socket and MUST NOT silently respawn in a loop.
- **Two GUI processes, one daemon.** Independent clients, independent sessions;
  the daemon arbitrates through its existing client registry. Neither GUI may
  spawn a daemon the other already owns — the pre-flight liveness check is
  what prevents the second one from stealing the socket.
- **A second daemon is started by the operator while the GUI supervises one.**
  Because `run_server` unlinks before binding
  (`crates/eggsec-daemon/src/server.rs:141-146`), the second daemon takes the
  socket, and the supervised child then owns a socket path it no longer
  serves. The GUI MUST detect the disconnect and report it plainly, naming
  the possibility, rather than retrying against a stolen socket.
- **A capability is advertised but the feature is off.** The submission is
  denied by the engine with `DenialClass::FeatureMissing`
  (`crates/eggsec-policy/src/decision.rs:921-930`). The GUI MUST record the
  variant as feature-blocked, re-render it as hidden with the feature named,
  and MUST NOT retry.
- **The daemon's enabled-feature set is unknown.** The variant is shown with
  an explicit "cannot verify" marker. The GUI MUST NOT hide it on missing
  data.
- **Report data is unavailable for a completed task** (an old daemon, or a
  task whose result the engine could not bridge). The GUI reports "no report
  available for this task" and the export is offered as unavailable. It MUST
  NOT write an empty report and MUST NOT report a successful export.
- **The report conversion fails.** The error is an `Err`; no file is written,
  and the GUI MUST NOT report a success
  (`crates/eggsec-tui/src/app/export.rs:51-70`).
- **The export destination already exists.** The previous file is backed up to
  `<name>.bak` first. A destination that is not a regular file is refused.
- **The export is cancelled or times out.** The export is a finite operation;
  its spawned task carries a 30–300s timeout wrapper, and cancellation goes
  through `eggsec-runtime::race_with_cancel`
  (`crates/eggsec-runtime/src/cancel.rs:27-43`). A cancelled export leaves no
  partial file — the write is atomic from the caller's point of view (write to
  a temporary name in the same directory, then rename), or the partial file is
  removed and reported.
- **The GUI restarts, or the daemon restarts.** The GUI owns no authoritative
  state; it re-attaches. The task list is rebuilt from the daemon's session
  snapshot, and the export history is **not** restored (the GUI keeps no export
  history). On a daemon restart the report store is empty, and the GUI reports
  "no report available" rather than a stale one. A daemon restart mid-task
  ends the subscription; running tasks are shown as "connection lost", not as
  failed — M005's rule, unchanged.
- **Concurrent callers.** One GUI process, one session, one subscription. The
  coverage table is a read-only projection, so concurrent reads are free. The
  export path MUST serialise writes to the same destination path within the
  GUI, so two concurrent exports cannot interleave into one file.
- **Stale generations.** The coverage table and the availability verdict are
  recomputed from the daemon's *current* published data. A verdict computed
  against an older catalog MUST NOT be applied to a newer one. The GUI keys
  its state on the catalog it was derived from, and a catalog change discards
  the derived verdict.
- **The event stream outruns the UI.** Unchanged from M005: the per-task log
  cap (`crates/eggsec-gui/src/tasks.rs:28`) and the flush cap (`:35`) apply.
  M006 adds no new event source.
- **Shutdown.** The GUI closes the session and the subscription cleanly and
  terminates a child it spawned. In-flight tasks are left running in the
  daemon — the GUI does not cancel them implicitly on exit.

## 9. Compatibility and migration

- **Backward compatibility with older daemons.** A daemon that predates the
  enabled-feature publication omits the field or answers
  `ErrorCode::UnsupportedCommand`. The GUI degrades to the `unverified` state
  and to "no report available" — the M001 precedent applied to two new read
  paths. It MUST NOT fail to launch.
- **Backward compatibility with older clients.** `DaemonCapabilities` grows one
  `#[serde(default)]` field, so a new daemon's capabilities decode cleanly
  against an old client (serde ignores unknown fields) and an old daemon's
  capabilities decode cleanly against a new client (the default). The
  protocol version stays at `DAEMON_PROTOCOL_VERSION`.
- **The pinned envelope contract is unchanged.**
  `crates/eggsec/tests/runtime_contract_closure.rs:618-619` MUST remain green
  and unmodified.
- **No `TaskOutcome` variant is added** (Work package D, requirement 5),
  because an externally tagged serde enum is not wire-additive.
- **No `RuntimeClient` trait change.** The new accessors are inherent methods
  on `DaemonRuntimeClient`, mirroring `describe_operations`. The trait stays
  frontend-neutral (`crates/eggsec-frontend-client/src/lib.rs:66-88`).
- **No data migration.** The GUI owns no authoritative storage, and the
  daemon-side report store is runtime state, not a persisted representation
  (`plans/subsystems/desktop-gui-frontend-roadmap.md:739-745`).
- **Configuration fallback.** The GUI's new option (`--daemon-bin`) is
  optional and defaults to "no spawn route", which is today's behaviour. An
  unparseable or empty value is a usage error, consistent with `--socket`
  (`crates/eggsec-gui/src/config.rs:56-80`).
- **Removal criteria.** The `--daemon-bin` spawn route MAY be removed if the
  daemon gains a first-class readiness handshake or a PID file — at which
  point the pre-flight and readiness logic becomes redundant. Until then it
  stays. The `Unverified` availability state is removed when every supported
  daemon publishes its enabled-feature set.

## 10. Required tests

### Focused unit tests

- **Coverage model** — for every published task kind: the verdict, the reason,
  and the record's fields. The aggregate: **30** variants, **30** distinct
  capability names, **26** distinct operation ids, the **four** named
  collapses, and `operation_id != capability_name` for exactly **10** of the
  30 — asserted from the published artifact, not from a transcribed list.
- **Availability resolver** — one test per verdict: available,
  capability-not-advertised, feature-off, not-exposed, unverified.
- **The 8-of-20 case** — against a daemon publishing enabled features
  `{ cli }`, the eight advertised-but-feature-off variants resolve to
  feature-off, and the twelve advertised-and-feature-on variants resolve to
  available.
- **Denial-driven fallback** — a `FeatureMissing` denial and an
  `ExecutionError::FeatureUnavailable` both flip the variant to
  feature-blocked, with the feature named in the reason.
- **Degradation** — a daemon that omits the enabled-feature set yields
  `unverified` for every variant, and hides **none**.
- **Export format map** — one test per format, asserting the extension; JUnit
  MUST resolve to `xml`.
- **Export write discipline** — backup of an existing regular file, refusal of
  a non-regular-file destination, and no file written on a failed conversion.
- **Daemon lifecycle helpers** — the pre-flight liveness check, the readiness
  timeout, the termination sequence's signal choice (not SIGHUP), and the argv
  construction (a vector, never a shell string).

### Integration tests

- **The full coverage path against a live daemon host**, the way
  `crates/eggsec-daemon`'s own tests do: connect, create a `DesktopGuiManual`
  session, fetch the catalog, build the coverage table, resolve every
  variant's verdict, submit an available variant, receive the events, render
  the result, cancel.
- **The report path against a live daemon**: run a task that produces a
  report, fetch it over the new read path, render it through `eggsec-output`,
  and write the artifact.
- **The export path against a live daemon**: export the same report in all six
  formats and compare each against `eggsec-output`'s own output.
- The event stream and the report MUST be driven by the daemon's own emission,
  not by a hand-written fixture list, so the test breaks if the wire shape
  changes.

### Restart and recovery tests

- A daemon restart between submission and completion is reported as a lost
  connection, and the task is **not** reported as failed.
- A daemon restart after completion empties the report store; the GUI reports
  "no report available", not a stale report.
- The GUI restart rebuilds the coverage table from the daemon's current
  catalog and does not resurrect a stale verdict.
- A supervised daemon that dies is reported as an exit, with its status.

### Contention and cancellation tests

- Cancelling a running task reaches the daemon and the status becomes
  cancelled.
- Cancelling a task that has already completed is reconciled from the event
  stream, not reported as an error.
- A log flood of 10,000 lines retains at most the cap, and the dropped-line
  counter is non-zero (M005's rule, unchanged).
- Two concurrent exports to the **same** path do not interleave; the second is
  refused or serialised.
- The readiness wait times out on a child that never becomes connectable, and
  the child is terminated on timeout.
- Two GUI processes against one daemon: neither displaces the other's daemon.

### Security and negative tests

- **Guard 161**: a hardcoded variant list in the GUI's production sources
  fails the guard. A hand-written operation-id table fails the guard.
- **Guard 162**: an exported HTML artifact with a remote source reference
  fails the guard.
- The GUI's IPC surface exposes no command that sets the surface, sets the
  scope, or forwards an arbitrary daemon message.
- The webview cannot set the surface: the request's surface is stamped in
  Rust, as in M005.
- An export path that escapes the operator-chosen location is refused (a path
  with a `..` component that resolves outside the chosen directory).
- A report is not written when the destination is not a regular file.
- No credentials appear in the GUI's logs or state: the export path logs the
  path and the outcome, never the report body.
- The GUI performs no outbound HTTP: the report arrives over the Unix socket.
- The spawn route executes only the operator-named binary; a test asserts no
  `PATH` lookup occurs.

### Migration and compatibility tests

- A daemon that predates the enabled-feature publication degrades to
  `unverified`, and no variant is hidden on missing data.
- A daemon that predates the report read path answers `UnsupportedCommand`,
  and the GUI reports "no report available" rather than failing.
- `crates/eggsec/tests/runtime_contract_closure.rs` remains green and
  unmodified.
- An old client decoding a new daemon's `DaemonCapabilities` still succeeds
  (the new field is ignored), and a new client decoding an old daemon's
  capabilities succeeds (the field defaults).
- The result rendering path is unchanged for every one of the 23 registry
  kinds, and an unknown kind degrades to generic JSON.

## 11. Required verification commands

```bash
# Format and lint
cargo fmt --all --check
cargo clippy -p eggsec-gui --features shell -- -D warnings
cargo clippy -p eggsec-frontend-client -p eggsec-daemon-protocol -p eggsec-daemon -p eggsec-runtime -- -D warnings

# Build matrix
cargo check -p eggsec-gui --no-default-features
cargo check -p eggsec-gui --features shell --all-targets
cargo check --workspace --no-default-features
cargo check --workspace --all-targets

# Contract
make check-msrv
make check-deps
bash scripts/check-architecture-guards.sh

# Tests
cargo test -p eggsec-gui --lib --features shell
cargo test -p eggsec-frontend-client --lib
cargo test -p eggsec-runtime --features schema --lib
cargo test -p eggsec-daemon-protocol --lib
cargo test -p eggsec-daemon --lib
cargo test -p eggsec-ui-model --lib
cargo test -p eggsec-output --tests
cargo test -p eggsec-report-model --tests

# Deep check required before M6 closure (roadmap §9)
make check-features-individual

# Documentation references
python3 scripts/check_doc_references.py
```

`make check-deps` MUST pass `bans`, `licenses`, and `sources`. The `advisories`
check is **known to fail at the baseline** on the inherited `zip 2.4.2` and
`rustls 0.21.12` advisories (§3.12); that backlog is a separate maintainer
decision and is not a gate this milestone may take on itself. The implementer
MUST record the baseline state and the post-change state of every
`cargo deny check` sub-check, so a *new* failure is distinguishable from the
inherited one.

`make check-features-individual` is **known to fail at the baseline** on the
`--all-features` compile error (§3.12). The implementer MUST record the exact
failure and MUST NOT silently widen M006 to fix it (§14).

## 12. Documentation updates

- `architecture/desktop_gui.md` — the coverage model and its multiplicities,
  the availability resolver and its five verdicts, the daemon-lifecycle
  decision and the spawn route, the export path and the format map, the HTML
  embedding constraint and the option taken, and the **corrected**
  socket-duplication count (six, not three or four).
- `architecture/overview.md` — the new modules and guards 161–162.
- `architecture/daemon.md` — the daemon's published enabled-feature set, the
  report read path, and the readiness marker.
- `architecture/frontend_client.md` — the new inherent accessors and the
  `UnsupportedCommand` degradation.
- `architecture/runtime.md` — the `RuntimeTaskExecutor::enabled_features`
  addition and the report store's bounded lifetime.
- `docs/REPORT_EVIDENCE_MODEL.md` — the report read path and the
  self-contained HTML mode.
- `docs/ARCHITECTURE.md` — the GUI's position in the dependency picture,
  including the new `eggsec-output` edge.
- `docs/ARCHITECTURE_INVARIANTS.md` — the descriptive-not-authoritative rule
  extended to `required_features` (a requirement list is not a state report).
- `docs/CI_ARCHITECTURE_GUARDS.md` — guards 161–162.
- `docs/VERIFICATION.md` — any new verification command.
- `docs/DEPENDENCY_EXCEPTIONS.md` — only if the widened closure needs an
  exception, with an owner and a review-by date.
- `docs/FEATURE_MATRIX.md` — if the daemon's published feature set changes the
  documented feature surface.
- `docs/COMMAND_REGISTRY.md` — the GUI is not a CLI command; verify and leave
  unchanged.
- `.opencode/skills/eggsec-gui/SKILL.md` — the coverage model, the
  availability resolver, the daemon lifecycle, and the export path.
- `plans/registry.md` and `plans/subsystems/desktop-gui-frontend-roadmap.md` —
  the milestone status.

## 13. Acceptance criteria

1. A coverage test asserts that **every** one of the 30 `TaskKind` variants is
   reachable from the GUI, with the exact multiplicities pinned: 30 variants,
   30 distinct capability names, 26 distinct operation ids, the four named
   collapses, and 10 of 30 with `operation_id != capability_name`.
2. The GUI derives its coverage entirely from the published catalog. No
   per-variant table, branch, or hardcoded name exists in the GUI's production
   sources (guard 161).
3. A feature-gated operation is **hidden with the missing feature named** when
   the daemon's published enabled-feature set shows it is off, and **reported
   as unavailable** when the denial arrives instead. Neither case presents an
   opaque failure.
4. A daemon whose enabled-feature set is unknown degrades to an explicit
   "cannot verify" state and hides nothing.
5. The GUI's daemon-lifecycle behaviour is explicit: with no `--daemon-bin`,
   the GUI connects and, on failure, names exactly how to start a usable
   daemon; with `--daemon-bin`, the GUI brings up a usable daemon, detects a
   daemon that is already running, waits for readiness with a bounded timeout,
   and terminates the child it started.
6. A daemon that advertises task kinds it cannot execute (the
   20-advertised/0-executable case) is detected and reported, not offered as
   usable.
7. The GUI never spawns a daemon binary the operator did not name, and never
   resolves one from `PATH`.
8. Export produces the **same artifacts as the CLI** for all six formats —
   JSON, CSV, HTML, Markdown, SARIF, JUnit — compared against the shared
   `eggsec-output` renderers. PDF is not offered.
9. The GUI renders the report's **structured content** in its own DOM, with
   its own escaping and its own stylesheet, and **no engine-produced HTML is
   injected into the webview**. Nothing the GUI renders for display executes a
   script, applies an inline style, or references a remote origin, and the
   GUI's CSP is **unchanged** — no `'unsafe-inline'`, no `frame-src` change.
   The **exported** HTML file is the CLI's HTML artifact byte-for-byte,
   including the remote chart-library reference the CLI's artifact carries,
   because the exported file is opened in the operator's browser exactly as
   the CLI's is. (See §0, ambiguity 2, and §6 "Frontend or operator
   surface": the plan previously required an embedded HTML document that
   satisfies the CSP, which the CSP's own `frame-src 'none'` and inline
   `script-src`/`style-src` forbid.)
10. The GUI owns no authoritative storage and performs no data migration.
11. The GUI adds no dependency on the engine, the daemon crate, the CLI, or
    the TUI; no `tauri` edge appears outside `crates/eggsec-gui`; the
    frontend's npm dependency set remains empty.
12. The GUI's IPC surface sets neither the surface nor the scope, forwards no
    arbitrary daemon message, and writes no file outside the operator-chosen
    export path.
13. The pinned empty-payload envelope contract
    (`crates/eggsec/tests/runtime_contract_closure.rs:618-619`) is green and
    unmodified, and no `TaskOutcome` variant was added.
14. Guards 161 and 162 exist, pass, and each was demonstrated to fail under a
    deliberate violation.
15. `make check` is green, or every inherited failure (§3.12) is recorded
    verbatim with its baseline evidence and is demonstrably unchanged by M006.
16. Every document and skill listed in §12 is updated, and
    `python3 scripts/check_doc_references.py` reports no failure on any line
    this milestone changed.

## 14. Stop conditions

Stop and escalate rather than improvise if any of these arise:

- **Report data cannot reach the daemon without changing the pinned
  empty-payload contract.** `TaskResultEnvelope.payload` and `.artifacts` are
  pinned (`crates/eggsec/tests/runtime_contract_closure.rs:618-619`). Changing
  them is **not additive** and is an ADR decision point
  (`plans/subsystems/desktop-gui-frontend-roadmap.md:861-864`). Escalate with
  the ADR; do not fill the payload.
- **The additive report read path turns out to require a new `TaskOutcome`
  variant.** An externally tagged serde enum is not wire-additive; an old
  client fails to deserialize an unknown variant. Escalate.
- **The daemon-lifecycle decision cannot be made within this milestone's
  ownership.** Specifically: if the spawn route cannot be made reliable (no
  dependable readiness signal, no safe termination sequence, or the operator
  cannot supply a binary), the GUI MUST fall back to the precise-diagnostic
  behaviour and record that the spawn route was not delivered. Escalate
  rather than shipping a spawn route that can orphan a process or steal a
  live socket.
- **No daemon executable is discoverable.** Nothing in the workspace installs
  `eggsec-daemon`, and there is no `PATH` or sidecar convention. The spawn
  route therefore MUST be operator-supplied. If the implementer finds
  themselves adding a discovery mechanism, that is a scope expansion into
  packaging (milestone 007) — stop.
- **The reliable "can this daemon actually execute?" signal cannot be
  established.** If neither the published enabled-feature set nor another
  non-invasive signal distinguishes a usable daemon from the
  20-advertised/0-executable case, the GUI MUST report the daemon's identity
  and the exact start command instead of offering a submit affordance it
  cannot back. Escalate rather than shipping a probe-submit heuristic.
- **The inherited `make check-deps` advisory failure blocks the closure.** The
  `zip 2.4.2` and `rustls 0.21.12` advisories (§3.12) are a **maintainer
  decision** (owner + review-by date in `deny.toml` and
  `docs/DEPENDENCY_EXCEPTIONS.md`). Escalate; do not add an exception on the
  implementer's own authority.
- **The inherited `--all-features` compile error blocks the closure.**
  `E0616: field 'url' of struct 'WebSocketFuzzer' is private`
  (`crates/eggsec-payloads/src/websocket.rs:43` vs
  `crates/eggsec/src/fuzzer/advanced.rs:446`) breaks
  `make check-features-individual`, which the roadmap requires before M6
  closure. It is a **separate corrective milestone**. Escalate; do not widen
  M006 to fix it. Note the narrow exception: a **scoped** fix to those two
  lines is arguably in scope if the maintainer approves it in writing before
  the work starts — it is not a licence to refactor the fuzzer.
- **The GUI appears to need engine-side access** for any reason. That is an
  `ADR-0007` boundary violation, not an implementation difficulty. Escalate.
- **The GUI appears to need to decide policy.** The catalog is descriptive. If
  the implementer is tempted to pre-authorize or pre-scope, escalate.
- **The GUI's coverage model needs a hand-written per-operation table.** That
  is guard 159's defect in a new place. Escalate.
- **The HTML self-containment cannot be achieved without changing
  `eggsec-output`'s default output.** The CLI's HTML artifact must not
  change. Escalate rather than shipping a silently different default.
- **Bounded retention cannot be maintained** for the coverage table, the
  report store, or an exported payload. Unbounded retention is a
  disqualifying defect, not a tuning problem.

## 15. Closure evidence required

The closure record (`plans/closure/desktop-gui-frontend/006-full-operation-coverage-and-report-export-closure.md`)
MUST contain:

1. The **measured** coverage numbers: 30 variants, 30 capability names, 26
   operation ids, the four collapses, and the 10-of-30 divergence — with the
   command that produced them.
2. The **availability verdicts** against a stock `--full-executor` daemon: the
   eight feature-off variants and the ten capability-not-advertised variants,
   listed by name.
3. The **daemon-lifecycle decision** as implemented: the default connect
   path, the `--daemon-bin` spawn path, the readiness signal, the termination
   sequence, and the exact wording of the operator-facing diagnostics.
4. The **report path**: the wire shape added, the degradation against an old
   daemon, the store's bound and its documented value, and the evidence that
   `crates/eggsec/tests/runtime_contract_closure.rs` is green and unmodified.
5. The **artifact-equivalence evidence**: the six formats, each compared
   against `eggsec-output`'s own output, and the statement of which HTML
   option was taken (and, if option 2, the equivalence caveat).
6. The **guard evidence**: guards 161 and 162, each with a demonstration of
   failing under a deliberate violation and passing after restore.
7. The **verification commands actually run**, with their outcomes. Commands
   that were not run MUST NOT be claimed.
8. The **inherited failures** (§3.12), stated plainly with their baseline
   evidence, and an explicit statement that M006 neither fixed nor worsened
   them.
9. The **dependency-closure delta**: the crates M006 added to the GUI's
   closure, and the `cargo deny` result for `bans`, `licenses`, and `sources`.
10. The **documentation diff summary** — every file in §12, and the
    `check_doc_references.py` result.

## 16. Handoff notes

- **The 13 pre-existing stashes MUST be preserved untouched.** `git stash list`
  currently holds 13 entries on this branch. Do not pop, drop, or rewrite
  them; they predate this milestone and belong to other work.
- **`cargo fmt --all` is required before committing.** The workspace's `make
  check` starts with `cargo fmt --all --check` (`Makefile:155`), and a
  formatting-only failure is a common way an otherwise complete milestone
  fails the contract.
- **`crates/eggsec-gui/gen/schemas/` is gitignored** (`.gitignore:10` covers
  `crates/*/gen/schemas/`). The directory is regenerated by `tauri-build`; it
  MUST NOT be committed and MUST NOT be hand-edited.
- **`crates/eggsec-gui/icons/icon.png` MUST be kept.** Tauri's
  `generate_context!` (`crates/eggsec-gui/src/shell.rs:47`) panics without the
  icon set; deleting it breaks the shell build in a way that looks like an
  unrelated failure.
- **Tauri's command arguments default to camelCase.** Every IPC command MUST
  declare `#[tauri::command(rename_all = "snake_case")]`, as the existing eight
  do (`crates/eggsec-gui/src/commands.rs:325,265,291,338,374,435,451,463`).
  A new command without it will not be callable from the frontend under the
  names the frontend uses.
- **The frontend has zero npm dependencies and no build step, and that MUST
  stay true.** The workspace has no `package.json` under
  `crates/eggsec-gui/`. M006's coverage table and export view are vanilla
  JavaScript. Introducing a build step would re-open `ADR-0009`'s deferred
  dependency-policy question, which milestone 005 discharged.
- **The `shell` feature gates the Tauri dependency**
  (`crates/eggsec-gui/Cargo.toml:29-35`). The GUI crate MUST continue to build
  under `--no-default-features`, which is the `make check` baseline
  (`Makefile:156`). New modules that need Tauri types MUST be gated the way
  `commands.rs` and `shell.rs` are (`crates/eggsec-gui/src/lib.rs:20-23`).
- **Building the whole workspace is expensive and other crates may be busy.**
  Prefer `-p` scoped commands during development and run the full `make check`
  once, at the end.
- **The socket-path default must not be duplicated a seventh time.** If the
  export path needs a default directory, it MUST come from one place in
  `crates/eggsec-gui/src/config.rs` and be shipped to the webview — not
  re-declared in `frontend/app.js` or `frontend/index.html`.
- **The 34 existing GUI tests must stay green.** They are the M005 contract; a
  failure there is a regression, not a test to update. **`make check-python`
  was not run at the baseline** and is not needed for M006 (no Python bindings
  change) — record it as not run rather than claiming it.
