# Desktop GUI Frontend Milestone 006 — Closure Status

Status: implemented

Source implementation plan:

- `plans/implementation/desktop-gui-frontend/006-full-operation-coverage-and-report-export.md`

Source subsystem roadmap:

- `plans/subsystems/desktop-gui-frontend-roadmap.md`

Repository baseline reviewed: `9a9b000f`

Implementation commits or pull requests:

- `818e32d1` — the milestone's implementation, the plan's amendment record, the
  documentation sweep, and the two new architecture guards.
- The closure record and the registry and roadmap status updates are the
  working-tree change committed alongside it on branch
  `feat/auto-20261022-76692679`.

## 1. Executive finding

The milestone's capability boundary is complete. The desktop GUI now tells the
operator **which** of the daemon's published operations it can reach and
**why**, produces the CLI's report artifacts for six formats, and can bring up
and supervise a daemon — and the engine no longer denies every task the GUI
submits.

What landed:

- **A derived coverage model.** One row per task kind the daemon publishes,
  with a verdict and a reason. Nothing is transcribed: the variants, the
  operation ids, the capability names, and the required-feature lists all
  arrive as arguments and the verdicts are computed from them, so an
  operation the engine publishes tomorrow is covered without a line being
  added. Six verdicts: `available`, `not_exposed`, `not_advertised`,
  `feature_disabled`, `unverified`, `blocked`.
- **An availability resolver with a denial ledger.** The daemon publishes its
  enabled-feature set additively, so the GUI can distinguish "the daemon
  withholds this capability" from "the daemon was built without the feature".
  A live `FeatureMissing` denial becomes a learned verdict that **wins over
  every static one** — the engine's own word about the running daemon, which
  no published list can overrule.
- **A report read path.** A new read-only `GetTaskReport` command and a
  `TaskReport` response, with the document produced by a **pure** engine-side
  bridge and held in a **bounded** store. The result envelope's pinned empty
  payload is untouched.
- **Report export** through `eggsec-output` for JSON, CSV, HTML, SARIF, JUnit,
  and Markdown, written with the TUI's export discipline.
- **An opt-in daemon lifecycle**: an operator-supplied `--daemon-bin`, an
  argv-vector spawn, a connection-based pre-flight probe, a bounded
  handshake-based readiness wait, and SIGTERM-then-SIGKILL teardown.

Three defects were found and fixed along the way, and each is recorded below
in full: the `DesktopGuiManual` surface was missing from the engine's
permissive set, so **every GUI task was denied**; the daemon answered an
unknown command with an **empty request id**, so a client waited out its
30-second timeout instead of learning the command was refused; and the GUI's
Content Security Policy forbids embedding the engine's HTML document, so the
report view renders structured content and the HTML format is export-only.

Two of the plan's own premises were falsified before implementation and the
plan was **amended rather than implemented as written**. The plan's §3
measurement that `full-executor` enables only the engine's `cli` feature was
false after milestone 008, which replaced it with the `service` profile. And
the plan's "embedded HTML display that satisfies the CSP" was unsatisfiable
under the GUI's own CSP — a contradiction the plan did not contain when it was
written. Both corrections are in the plan's §0 amendment record.

## 2. Requirement-to-evidence matrix

| Requirement | Evidence | Result | Notes |
|---|---|---|---|
| The coverage model is derived from published data, never transcribed | `crates/eggsec-gui/src/coverage.rs`; guard 161; `the_gui_production_sources_name_no_operation_variant_or_capability` | pass | one row per published task kind; the coverage function takes plain slices, so a test can construct its inputs |
| Every published task kind appears in the coverage table | `crates/eggsec-gui/src/coverage.rs`; `crates/eggsec-daemon-protocol/tests/coverage_multiplicity.rs` | pass | 30 task kinds, 34 catalog operations, 26 distinct operation ids, 30 distinct capability names; the three packet kinds share `packet`, `Pipeline`/`Resume` share `pipeline`, `Wireless`/`WirelessActive` share `wireless` |
| The daemon publishes its enabled-feature set, additively | `crates/eggsec-daemon-protocol/src/protocol.rs` (`DaemonCapabilities::enabled_features`); `crates/eggsec-daemon/src/host.rs` | pass | `None` = unknown, `Some(vec![])` = an engine linked with no features enabled; `#[serde(default)]`, so the protocol version stays at 2 |
| An unavailable operation is **reported**, not silently hidden | `crates/eggsec-gui/src/coverage.rs` (six verdicts, each with a reason) | pass | a learned denial overrides the static verdict |
| A live denial becomes an explicit unavailability report | `crates/eggsec-gui/src/availability.rs` (`denial_from_message`, `DenialLedger`) | pass | the ledger is bounded to one entry per variant |
| Report data crosses the wire without touching the pinned envelope | `crates/eggsec-daemon-protocol/src/protocol.rs`; `crates/eggsec-daemon/src/host.rs`; `crates/eggsec/src/runtime_bridge/report.rs` | pass | `TaskResultEnvelope.payload` stays `json!({})` and `.artifacts` stays `vec![]`; `crates/eggsec/tests/runtime_contract_closure.rs` is green |
| Report retention is bounded | `crates/eggsec-runtime/src/report.rs` (`RuntimeReportStore`, `MAX_RETAINED_REPORTS = 64`, oldest-first eviction) | pass | a long-lived daemon cannot accumulate one document per task |
| Export produces the CLI's artifacts for six formats | `crates/eggsec-gui/src/export.rs`; `crates/eggsec-output/src/convert.rs` | pass | the exported HTML is `convert_to_html`, the CLI's document byte-for-byte |
| The export follows the TUI's write discipline | `crates/eggsec-gui/src/export.rs` (`write_export`) | pass | an existing regular file is backed up to `<path>.bak`; a non-regular-file destination is refused; every failure is an `Err` |
| The report display references no remote origin | `crates/eggsec-gui/src/export.rs`; `crates/eggsec-gui/frontend/app.js`; guard 162 | pass | the report reaches the webview as structured data; there is no `innerHTML` in the frontend |
| The GUI's CSP is unchanged | `crates/eggsec-gui/tauri.conf.json`; guard 162 | pass | no `'unsafe-inline'`, no `frame-src` change — the report content is attacker-influenced |
| The daemon lifecycle is explicit and opt-in | `crates/eggsec-gui/src/config.rs` (`--daemon-bin`), `crates/eggsec-gui/src/daemon.rs` | pass | no `PATH` lookup, no sidecar convention; the default still attaches to an operator-started daemon |
| The spawn is an argv vector, never a shell string | `crates/eggsec-gui/src/daemon.rs` (`spawn_argv`) | pass | no shell is involved, so there is no injection surface |
| The pre-flight check is a connection attempt, not a `stat` | `crates/eggsec-gui/src/daemon.rs` | pass | a stale socket file does not block the spawn; a live daemon is never displaced |
| The readiness wait is bounded and handshake-based | `crates/eggsec-gui/src/daemon.rs` (`READINESS_TIMEOUT` 60 s, `READINESS_POLL_INTERVAL` 1 s) | pass | the child's stdout is never scraped |
| Teardown is SIGTERM then SIGKILL, and leaves no orphan | `crates/eggsec-gui/src/daemon.rs` (`TERMINATION_GRACE` 5 s); `crates/eggsec-gui/src/shell.rs` | pass | a readiness timeout terminates the child; the exit path and a signal watcher stop a supervised daemon |
| The GUI's IPC surface stays narrow | `crates/eggsec-gui/src/commands.rs` | pass | 12 commands; none takes a surface or a scope from the webview; `export_report` takes a task id, a format, and an operator-chosen path |
| The `DesktopGuiManual` surface is permissive | `crates/eggsec/src/runtime_bridge/executor.rs` | pass | the regression test was demonstrated to fail before the fix; no other surface was widened |
| An unknown daemon command is refused cleanly | `crates/eggsec-daemon/src/server.rs` (`refused_command_response`) | pass | `UnsupportedCommand` with the request id echoed, versus `InvalidRequest` for a malformed command |
| The CLI's HTML artifact did not change | `crates/eggsec-output/src/html.rs` | pass | the default output was compared byte-for-byte against a pristine worktree at the baseline commit: identical SHA-256 |
| Two new guards exist, pass, and were negative-tested | `scripts/check-architecture-guards.sh` (checks 161, 162) | pass | each was demonstrated to fail under a deliberate violation and to pass again after the restore |
| `make check` is green, or every inherited failure is recorded | §4 and §10 | partial | `make check` still stops at the inherited `check-deps` advisory backlog; every step before it passes and the steps after it were run directly and are green |

## 3. Production implementation evidence

### Ownership and seams

The milestone spans the runtime crate, the protocol crate, the daemon, the
engine, the output crate, and the GUI — but **no seam was crossed that the
architecture forbids**. The GUI still links no engine crate, no daemon crate,
no CLI crate, and no TUI crate. Its only new dependency edge is
`eggsec-output`, a leaf data crate (`eggsec-core` + `eggsec-report-model`, no
engine edge), plus `libc` (unix-only) because tokio's `signal` module
*receives* signals but cannot send them, and the specified SIGTERM-then-SIGKILL
teardown needs the syscall.

The runtime crate gained a **report store** and **two defaulted trait
methods**. A defaulted method is additive at the Rust level: every existing
executor keeps compiling unchanged, and the daemon's no-op executor publishes
`None` for the feature set, which is exactly right — it links no engine, so
its feature set is *unknown*.

### The wire contract

`DaemonCapabilities` gained one `#[serde(default)]` field. `ClientCommand` and
`ServerMessage` each gained one variant at the end of the enum, as the
protocol's wire-additivity convention requires. The protocol version stays at
`DAEMON_PROTOCOL_VERSION = 2`: the GUI hard-refuses to connect on a mismatch,
so an additive change must not move it, and the new field and the new command
are both backward-compatible.

The report document is carried as `serde_json::Value` rather than as a typed
report-model struct, so the protocol crate keeps **no report-model
dependency**. The consumer — the GUI — deserializes it into the canonical
`ScanReportData`. The precedent is the existing envelope, whose payload is
already `serde_json::Value`.

### The engine

The report bridge (`crates/eggsec/src/runtime_bridge/report.rs`) is **pure**:
no I/O, no state, no clock. It derives a document from a completed
`TaskResult` for the families whose result carries the facts the canonical
model needs, and returns `None` for every other family rather than a lossy
document. The reused domain bridges (wireless, wireless-active, db-pentest,
C2) are called **verbatim** rather than reimplemented.

Two findings about the reused bridges are recorded rather than hidden: three
bridges named in the plan — `evasion`, `postex`, `mobile` — are **unreachable
from `TaskResult`** at all, because those operations are CLI-only and never
produce a runtime task result; their bridges are untouched and still serve the
CLI. And the reused wireless bridges are **not** pure — they call
`chrono::Utc::now()` internally — which the module documents rather than
claims around.

### The daemon

The daemon serves what the engine produced and knows nothing about report
semantics. Its published capability list was already derived from the
executor's declaration (milestone 009); this milestone adds the enabled-
feature set beside it.

The unknown-command fix is the substantive change. The daemon used to
deserialize a line straight into `ClientCommand`, so a command it did not know
was a *deserialization failure*: answered with `ErrorCode::InvalidRequest` and
an **empty** `request_id`. Because the frontend client routes responses by
request id, the pending request was never resolved and the client **waited out
its 30-second timeout** instead of learning the command was refused. M006
adds a command older daemons will not know, so this had to be fixed: the
daemon now recovers the request id and the command type from the raw line and
answers `ErrorCode::UnsupportedCommand` with the request id echoed, reserving
`ErrorCode::InvalidRequest` for a malformed command.

### The GUI

Four new modules, each with one job: `coverage.rs` (the model),
`availability.rs` (the resolver and the ledger), `export.rs` (the formats and
the write discipline), and `daemon.rs` (the lifecycle). The IPC surface grew
from eight commands to twelve — the coverage table, the export, the report
content, and the daemon status — and the command set's **three pinning
sites** (the argument-naming test, the declared-set test, and the shell's
`generate_handler!` list) moved together.

The report reaches the webview as **structured data**. The frontend builds its
DOM with `createElement` and `textContent`; there is no `innerHTML` anywhere
in `frontend/app.js`. This is the security property that matters: the report
content is attacker-influenced — banners, service names, and finding text come
from the target — so the GUI renders it as text and never as a document.

## 4. Verification executed

### Commands run

```bash
cargo fmt --all -- --check
cargo check --workspace --no-default-features

cargo test -p eggsec-runtime --lib
cargo test -p eggsec-daemon-protocol
cargo test -p eggsec-frontend-client
cargo test -p eggsec-output
cargo test -p eggsec-daemon --features service
cargo test -p eggsec --features cli --lib runtime_bridge
cargo test -p eggsec-gui --no-default-features --lib
cargo test -p eggsec-gui --features shell --lib

cargo clippy -p eggsec-runtime -- -D warnings
cargo clippy -p eggsec-daemon-protocol --all-targets -- -D warnings
cargo clippy -p eggsec-frontend-client --all-targets -- -D warnings
cargo clippy -p eggsec-output --all-targets -- -D warnings
cargo clippy -p eggsec-gui --all-targets -- -D warnings
cargo clippy -p eggsec-gui --no-default-features --all-targets -- -D warnings
cargo clippy -p eggsec --features cli -- -D warnings
cargo clippy -p eggsec-cli --features daemon-client -- -D warnings

bash scripts/check-architecture-guards.sh
python3 scripts/check_doc_references.py
```

### Results

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | clean |
| `cargo check --workspace --no-default-features` | pass — the CI baseline stays green |
| `cargo test -p eggsec-runtime --lib` | **95 passed**, 0 failed |
| `cargo test -p eggsec-daemon-protocol` | **89 + 7 passed**, 0 failed |
| `cargo test -p eggsec-frontend-client` | **13 passed**, 0 failed |
| `cargo test -p eggsec-output` | **73 + 7 + 12 passed**, 0 failed |
| `cargo test -p eggsec-daemon --features service` | **108 + 1 + 10 + 20 passed**, 0 failed |
| `cargo test -p eggsec --features cli --lib runtime_bridge` | **80 passed**, 0 failed |
| `cargo test -p eggsec-gui --no-default-features --lib` | **76 passed**, 0 failed |
| `cargo test -p eggsec-gui --features shell --lib` | **80 passed**, 0 failed |
| clippy, every crate this milestone touched | clean |
| `bash scripts/check-architecture-guards.sh` | **ALL PASSED** — 133 checks, including the two new ones |
| `python3 scripts/check_doc_references.py` | **Total missing: 29** — the baseline is unchanged |

### The negative tests for the two new guards

A guard that has never failed is not a guard. Each was demonstrated to fail
under a deliberate violation and to pass again after the restore.

- **Guard 161.** A hardcoded operation id was injected into the GUI's
  production half (`const HARDCODED_OPERATION: &str = "scan-ports";`). The
  guard failed with the file and line, and the guard script exited 1. The
  injection was removed and the guard returned to green.
- **Guard 162.** Two violations were injected in turn. First, an
  `innerHTML` markup sink was added to `frontend/app.js`; the guard failed
  naming the file and line. Second, `'unsafe-inline'` was added to the CSP's
  `script-src`; the guard failed naming the relaxation. Both were reverted
  and the guard returned to green.

### The regression test for the permissive-set defect

The engine's regression test was run against the **pre-fix** code by
temporarily removing the `DesktopGuiManual` arm. The test failed with
`the desktop GUI is a manual surface and must not be denied a scope`, while
the negative test for the strict surfaces stayed green. The fix was restored
and the same suite passed: 80 passed, 0 failed.

### The proof that the CLI's HTML artifact did not change

Not by inspection — by **empirical byte comparison**. A throwaway worktree at
the baseline commit rendered the same fixture, and the two outputs were
compared:

```
baseline (9a9b000f)  sha256 45c67ac9b5e24e54f431e0f9172c43adb6f11b5a7ceda673f65e85918cdf4551  6744 bytes
modified             sha256 45c67ac9b5e24e54f431e0f9172c43adb6f11b5a7ceda673f65e85918cdf4551  6744 bytes
diff -> IDENTICAL
```

The fixture exercised the wireless-networks path too, so the extracted shared
helper was covered by the comparison.

## 5. Invariant review

| Invariant | Status | Evidence |
|---|---|---|
| `EnforcementContext::evaluate()` remains the pre-dispatch gate | unchanged | the milestone adds a read path and a presentation layer; no dispatch path was added |
| Strict surfaces dispatch only via `EnforcedDispatcher::dispatch_execution()` | unchanged | no dispatch code was touched |
| `OperationMetadata` remains the single source of operation policy | strengthened | the coverage model reads the published catalog; it computes no policy |
| The GUI authorizes nothing | held | a hidden operation is a **presentation** decision derived from published state; the engine remains the only gate |
| `TaskResultEnvelope.payload` stays `json!({})` and `.artifacts` stays `vec![]` | held | `crates/eggsec/tests/runtime_contract_closure.rs` is green; `task_result_envelope` was not touched |
| No `TaskOutcome` variant was added | held | the report travels on its own command |
| The wire contract is additive | held | `#[serde(default)]` on the new field; new variants appended at the end of the enums; the protocol version stays at 2 |
| The GUI links no engine, daemon, CLI, or TUI crate | held | guard 155; the new edge is `eggsec-output`, a leaf data crate |
| The GUI performs no outbound HTTP | held | guard 157; the report arrives over the daemon's Unix socket |
| The CSP forbids remote origins | held | guard 158; guard 162 additionally pins the absence of `'unsafe-inline'` and of a markup sink |
| Ring-only TLS | held | the GUI gains no TLS dependency |
| Tokio features are declared per crate | held | the GUI adds `process` and `signal` explicitly |
| Spawned tokio tasks carry a 30–300 s timeout wrapper | held | the readiness wait is a 60 s bounded loop; the pre-flight probe is 2 s |
| Never `let _ =`, never `filter_map(|e| e.ok())` | held | diagnostics go through `tracing` |
| Bounded event and result retention | extended | the report store is bounded at 64 with oldest-first eviction |
| The GUI's forms and coverage are schema-driven, not per-operation | held | guards 159, 160, 161 |

## 6. Failure and recovery review

- **The daemon is unreachable and no `--daemon-bin` was given.** The GUI
  reports the existing diagnostic and opens in a "not connected" state. It
  does not retry in a tight loop.
- **The daemon is unreachable and `--daemon-bin` was given.** The pre-flight
  probe runs, the child is spawned, and the readiness wait begins with a
  bounded timeout. On timeout the child is terminated, the timeout is
  reported, and no orphan is left behind.
- **The spawned child exits during the readiness wait.** The wait sees the
  exit and reports the child's exit status as a failure — never as "the
  daemon is starting".
- **Two GUI processes, one daemon.** Independent clients, independent
  sessions. The connection-based pre-flight check is what prevents the second
  one from stealing the socket.
- **A second daemon is started by the operator while the GUI supervises
  one.** The daemon still unlinks the socket path before binding, so the
  second daemon takes the socket. The GUI detects the disconnect and reports
  it plainly. **This is a pre-existing defect that milestone 006 does not
  close** — see §10.
- **A capability is advertised but the feature is off.** The submission is
  denied by the engine; the GUI records the denial in its ledger and the
  coverage row becomes `blocked`.
- **The daemon is too old to know `GetTaskReport`.** The daemon answers
  `UnsupportedCommand`, the client maps it to `Ok(None)`, and the GUI reports
  that no report is available. It is a graceful degradation, never a launch
  failure.
- **The daemon publishes no enabled-feature set.** The coverage row is
  `unverified`, not `feature_disabled`. `None` means *unknown*, never
  "nothing is enabled".

## 7. Migration and compatibility review

There is no data migration. The GUI owns no authoritative storage.

Compatibility is preserved in both directions on the wire. A new daemon's
capabilities decoded by an old client work, because serde ignores unknown
fields. An old daemon's capabilities decoded by a new client work, because
`#[serde(default)]` supplies `None` for the absent field. The protocol version
stays at 2, so an old client and a new daemon still agree on the version.

A **new** client against an **old** daemon degrades cleanly rather than
failing: the absent enabled-feature set means the coverage verdict is
`unverified`, and the absent `GetTaskReport` command means the report is
simply unavailable. Before this milestone the second case was not clean — the
daemon's refusal carried no request id, so the client hung for 30 seconds.
That defect is fixed.

## 8. Security review

- **The report content is attacker-influenced.** Banners, service names, and
  finding text come from the target. The GUI therefore renders it as **text**
  in its own DOM, never as a document, and the CSP is left unchanged. This
  is the reason the plan's "embedded HTML display" was not implemented as
  written: making it work would have required either relaxing the CSP to
  `'unsafe-inline'` — which is exactly the control that stops injected content
  from executing — or relaxing `frame-src`.
- **The spawn route executes only a binary the operator named.** There is no
  `PATH` lookup, no sidecar convention, and no download. The child's argv is
  a vector, never a shell string, so no shell is involved and no injection
  surface exists.
- **The GUI does not widen scope.** A spawned daemon inherits the operator's
  environment; the GUI injects no scope into it.
- **The GUI authorizes nothing.** The coverage verdicts are presentation
  decisions derived from published state.
- **The export path is narrow.** `export_report` takes a task id, a format,
  and an operator-chosen path. It accepts no pre-built report, no surface, and
  no scope. The write refuses a non-regular-file destination and backs an
  existing regular file up rather than silently overwriting it.
- **The GUI persists no credentials** and does not log the report body.

## 9. Documentation and operations

Updated to describe what landed:

- `architecture/desktop_gui.md` — the coverage model, the availability
  resolver, the daemon lifecycle, the export path, the report-display
  decision, and the corrected socket-duplication count.
- `architecture/daemon.md` — the published enabled-feature set, the report read
  path, the unknown-command refusal, and the corrected `ClientKind` count.
- `architecture/frontend_client.md` — the new inherent accessor and the
  `UnsupportedCommand` degradation.
- `architecture/runtime.md` — the two defaulted trait methods and the report
  store's bounded lifetime.
- `architecture/overview.md` — the new GUI modules and guards 161–162.
- `docs/ARCHITECTURE.md` — the GUI's position in the dependency picture.
- `docs/ARCHITECTURE_INVARIANTS.md` — the descriptive-not-authoritative rule
  extended to `required_features`.
- `docs/CI_ARCHITECTURE_GUARDS.md` — guards 161 and 162.
- `docs/ENFORCEMENT_MODES.md` — the desktop GUI as a permissive manual surface.
- `docs/REPORT_EVIDENCE_MODEL.md` — the report read path and the
  self-contained HTML mode.
- `docs/VERIFICATION.md`, `docs/FEATURE_MATRIX.md`, `docs/COMMAND_REGISTRY.md` —
  checked; the last two needed no change.
- `.opencode/skills/eggsec-gui/SKILL.md` and
  `.opencode/skills/eggsec-output/SKILL.md`.
- `plans/implementation/desktop-gui-frontend/006-full-operation-coverage-and-report-export.md`
  — the amendment record, the corrected §3 evidence, the resolved ambiguities,
  and the new work package G.

## 10. Unresolved findings

1. **The report bridge covers only the families with a clean mapping.** Nine
   `TaskResult` families produce a document; every other variant yields "no
   report form" rather than a lossy document. An operator can therefore
   export a report for some completed tasks and not for others. A
   workspace-wide bridge for all thirty variants is a separate corrective
   milestone: it is a mapping exercise, not a contract change, and doing it
   half-way would be worse than not doing it. **Recorded as roadmap known gap
   11.**
2. **The daemon's server still unlinks the socket path before binding.** A
   second daemon started by an operator while the GUI supervises one takes
   the socket. M006's pre-flight probe narrows the window; it does not close
   it. Server-side hardening changes the bind contract and belongs to its own
   milestone. **Pre-existing; recorded as roadmap known gap 10.**
3. **`convert_to_html_self_contained` has no GUI consumer.** It was added to
   `eggsec-output` as an opt-in mode that drops the remote chart-library
   reference, with the default `convert_to_html` left byte-for-byte unchanged
   and verified as such. The GUI does not use it, because the CSP makes
   document embedding unavailable. It is a tested capability in its own
   right, not a GUI feature.
4. **The `service` profile is not clippy-clean.** `cargo clippy -p
   eggsec-daemon --features service` reports **80 pre-existing** findings
   across roughly twenty engine files that the profile newly compiles. None
   is in code this milestone touched. `make clippy` is unaffected — it lints
   the engine with `--features cli` only — and `make check-daemon-profile`
   gates on `cargo check`, with a Makefile comment recording why.
   **Pre-existing; inherited from milestone 008.**
5. **`cargo clippy -p eggsec-runtime --all-targets` reports three
   `collapsible_if` findings** in `crates/eggsec-runtime/tests/in_process_client.rs`.
   That file is **not** touched by this milestone; the findings surface only
   under `--all-targets`, which lints integration tests. `make clippy` and
   `cargo clippy -p eggsec-runtime` are clean.
6. **`make check` still stops at `check-deps`.** The advisory backlog is
   unchanged and is a maintainer decision — recording an exception needs an
   owner and a review-by date in `deny.toml` and
   `docs/DEPENDENCY_EXCEPTIONS.md`. Every step before it passes, and the
   steps after it were run directly and are green.
7. **`make check-python` was not run** — it requires a `.venv-ci/`, which
   cannot be provisioned here. The Python bindings' `ServerMessage` match
   gained the one arm the new variant requires, and
   `cargo check -p eggsec-python --features daemon-client` passes.
8. **The report bridge's `timestamp` field is empty for four families.** A
   pure bridge cannot stamp a clock, and those result types carry no scan
   timestamp, so the field is empty rather than invented. The pipeline family
   carries one and populates it.

## 11. Roadmap disposition

The roadmap's milestone 6 entry, its §4 current state, and its §12 status
table are updated. Known gaps 2, 3, and 8 are marked closed with what closed
them. Two new known gaps are recorded: 11 (the report bridge's coverage) and
12 (the GUI's IPC surface is twelve commands).

Milestone 7's dependency on milestone 6 is now **met**: the export path
exists and the daemon lifecycle the qualification runbook otherwise performs
by hand is the GUI's own `--daemon-bin` route. Milestone 7's `Export` cell
still needs its own evidence, and the behavioural cells still need a
window-automation driver, so milestone 7 remains **conditionally closed**.

## 12. Registry updates

- `plans/registry.md` — the `desktop-gui-frontend` section heading, the index
  row, the plan list, and the closure record reference are updated.
- `plans/subsystems/desktop-gui-frontend-roadmap.md` — the status line, §4's
  current state and known gaps, the milestone 6 entry, and the §12 status
  table are updated.
- `plans/implementation/desktop-gui-frontend/006-full-operation-coverage-and-report-export.md`
  — the status line is `implemented`, and the plan carries a §0 amendment
  record, the corrected §3 evidence, the two resolved ambiguities, and work
  package G.
- `plans/closure/desktop-gui-frontend/006-full-operation-coverage-and-report-export-closure.md`
  — this record.
