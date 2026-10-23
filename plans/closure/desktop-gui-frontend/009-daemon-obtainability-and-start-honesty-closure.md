# Desktop GUI Frontend Milestone 009 — Closure Status

Status: closed

Source implementation plan:

- `plans/implementation/desktop-gui-frontend/009-daemon-obtainability-and-start-honesty.md`

Source subsystem roadmap:

- `plans/subsystems/desktop-gui-frontend-roadmap.md`

Repository baseline reviewed: `0c38c403`

Implementation commits or pull requests:

- Uncommitted working-tree change on branch `feat/auto-20261022-76692679`.

## 1. Executive finding

The milestone's capability boundary is complete. The daemon no longer lies about what it can execute, and the CLI no longer reports a daemon as started before it is.

Three defects were closed:

1. **The capability lie is now unrepresentable.** The daemon host used to publish a hand-written capability constant while the executor behind it could reject every task. The published list is now **derived at construction** from the executor's own declaration, intersected with the daemon's conservative policy. A rejecting executor declares the empty set, so it publishes nothing; there is no longer a code path in which the two disagree.
2. **The start path reports only what it observed.** `eggsec daemon start` used to spawn a server, sleep a fixed 100 ms, and print "Daemon started" whether or not the daemon had bound the socket. It now performs a bounded readiness wait — connect, `Health`, then `Capabilities` — and reports success only from a typed *ready* outcome, with the socket path, the protocol version, and the advertised task-kind count.
3. **The daemon cannot be silently displaced.** Both start paths (the CLI's and the daemon binary's) now probe the socket path first. A live daemon that answers is **refused**, not displaced, and the refusal names the path.

The obtainability gap is closed by documentation, not by code: `docs/DAEMON.md` now carries the exact `cargo install` recipe for the daemon binary and the CLI, states that the `--full-executor` **runtime** flag is a different mechanism from the `service` **cargo** feature, and names the "does not cover" table that the distribution workstream inherits.

## 2. Requirement-to-evidence matrix

| Requirement | Evidence | Result | Notes |
|---|---|---|---|
| The published capability list is derived from the executor's declaration | `crates/eggsec-runtime/src/runtime.rs` (`supported_task_kinds()`), `crates/eggsec-daemon/src/host.rs` (`published_task_kinds()`), guard 165 | pass | The `Capabilities` arm no longer names any capability constant |
| A rejecting executor publishes the empty set | `NoopExecutorStub` declares the empty set; `DaemonHost::new_noop` derives an empty list; `cargo test -p eggsec-daemon --features service` | pass | 20 profile tests + 101 daemon lib tests green |
| A real executor still publishes the conservative set | `DaemonHost::new` derives `declared ∩ conservative`; daemon integration test `the_daemon_hosts_the_engine_executor_under_the_service_profile` | pass | The derivation narrows, never widens |
| The CLI carries no rejecting executor | `NoopExecutor` deleted from `crates/eggsec-cli/src/daemon_cli.rs`; guard 164 | pass | The CLI now hosts `EggsecRuntimeExecutor` |
| The readiness wait is bounded and named | `DAEMON_READINESS_TIMEOUT` (60 s), `READINESS_POLL_INTERVAL` (1 s), `SERVER_STOP_TIMEOUT` (10 s); guard 166 | pass | The timeout is a parameter of the helper, so a test exercises the bound without `test-util` |
| Readiness is a protocol exchange, never stdout scraping | No `"Daemon listening on"` literal in the CLI; guard 166 | pass | The server's log line is a log format, not a contract |
| The server task's own result is observed | `DaemonServer::stop` awaits a `oneshot` carrying the server's `Result` | pass | A bind failure reaches the operator |
| Success is reported only on a *ready* outcome | `DaemonStartOutcome::{Ready, Refused, Failed}`; test `the_start_path_does_not_report_success_when_the_socket_fails_to_bind` | pass | `--json` emits the same facts, and only on *ready* |
| A live daemon is refused, not displaced | `probe_daemon_liveness` in both start paths; `refuse_live_daemon()` in `crates/eggsec-daemon/src/main.rs`; test `a_started_daemon_answers_capabilities_with_a_non_empty_list` | pass | The socket is not unlinked on refusal |
| A stale socket does not block a start | The probe is a **connection**, not a `stat`; test `a_stale_socket_does_not_block_a_start` | pass | The server's own unlink cleans up a stale path |
| The pre-flight probe is bounded | `PREFLIGHT_PROBE_TIMEOUT`; test `the_preflight_probe_is_bounded` | pass | A hung daemon cannot hang the start path |
| The protocol version is reported, and a mismatch fails closed | `ReadyFailure::ProtocolMismatch`; test `the_readiness_check_reports_a_protocol_version_mismatch` | pass | Failing closed is a deliberate reading of "reported, not silently accepted" |
| The obtainability recipe names the cargo feature | `docs/DAEMON.md` "Obtaining the daemon"; guard 167 | pass | Both `service` and the deprecated `full-executor` alias are shown |
| The persistence claim is corrected | `docs/DAEMON.md` no longer claims `eggsec daemon start` persists by default | pass | The CLI-hosted daemon uses a no-op store |
| No protocol change | Protocol version stays 2; `DaemonCapabilities` unchanged | pass | No wire-shape change at all |
| No new dependency | `deny.toml` and `docs/DEPENDENCY_EXCEPTIONS.md` unchanged | pass | — |

## 3. Production implementation evidence

**The executor declaration seam** (`crates/eggsec-runtime/src/runtime.rs`). `RuntimeTaskExecutor` gained one **defaulted** method, `supported_task_kinds()`. The default preserves the pre-declaration behaviour — the conservative set — so every existing implementor keeps publishing what it published, and the milestone's compatibility invariant ("`RuntimeTaskExecutor` does not change: no method is added, removed, or re-signed") was relaxed by exactly one **additive, defaulted** method, which is the minimum that makes the declaration possible. `Runtime` captures the declaration at construction and exposes it synchronously, so a host can derive its published list without taking an async lock in a constructor.

**The derivation** (`crates/eggsec-daemon/src/host.rs`). `DaemonHost` gained a `published_task_kinds: Vec<TaskCapability>` field, computed once in `new`, `new_noop`, and `with_runtime` by `published_task_kinds(declared)` — the executor's declaration filtered through `RuntimeCapabilities::daemon_conservative()`. The hand-set `executor_is_noop: bool` field and the `with_runtime` parameter that carried it were **removed**, because the property is now computed rather than asserted.

**The start path** (`crates/eggsec-cli/src/daemon_cli.rs`). The file was substantially rewritten. The outcome is a typed `DaemonStartOutcome` with three variants — *ready* (socket path, protocol version, advertised count), *refused* (a live daemon already serves the path), *failed* (bind error or readiness timeout, with the cause). The readiness wait is `wait_for_daemon_ready`, which probes `Health` and `Capabilities` on a bounded loop and returns a `ReadyFailure` that distinguishes a timeout, a cancellation, a server failure, and a protocol mismatch.

**The pre-flight check** (`crates/eggsec-daemon/src/host.rs`, `crates/eggsec-daemon/src/main.rs`). `probe_daemon_liveness` attempts a connection and a `Health` exchange. An unreachable socket — a stale path left by a crashed daemon — is **not** live, and the start proceeds; the server's own unlink then cleans it up. A socket that answers is live, and the start is refused.

**The obtainability path** (`docs/DAEMON.md`, `CONTRIBUTING.md`). The recipe is `cargo install --path crates/eggsec-daemon --features service --locked` (the deprecated `full-executor` alias selects the same profile) plus `cargo install --path crates/eggsec-cli --features daemon-client --locked`. The document states the socket-path, data-directory, max-clients, and log-level defaults; the "does not cover" table; and the honest limitation that the **CLI-hosted** daemon links the engine with the `cli` feature only, so eight of the twenty advertised capabilities are feature-gated off and are denied with `FeatureMissing`.

**Three guards** — 164 (no rejecting executor in the CLI), 165 (the host derives its published capabilities), 166 (the start path waits for a real readiness signal), and 167 (the obtainability recipe names the cargo feature) — plus the documentation entries in `docs/CI_ARCHITECTURE_GUARDS.md`.

**The `make check` expansion** is additive: `cargo check -p eggsec-cli --features daemon-client` and `cargo test -p eggsec-cli --features daemon-client` were added, and the CLI's clippy line now covers the same profile. No existing line was removed.

## 4. Verification executed

### Commands run

```bash
cargo fmt --all --check
cargo check --workspace --no-default-features
cargo check -p eggsec-cli --features daemon-client
cargo test  -p eggsec-cli --features daemon-client
cargo check -p eggsec-daemon --features service
cargo test  -p eggsec-daemon --features service
cargo check -p eggsec-runtime
cargo test  -p eggsec-runtime --lib
cargo clippy -p eggsec-cli --features daemon-client -- -D warnings
cargo clippy -p eggsec-daemon -- -D warnings
cargo clippy -p eggsec-runtime -- -D warnings
bash scripts/check-architecture-guards.sh
make check-deps
```

### Results

| Command | Result | Detail |
|---|---|---|
| `cargo fmt --all --check` | pass | clean |
| `cargo check --workspace --no-default-features` | pass | — |
| `cargo check -p eggsec-cli --features daemon-client` | pass | — |
| `cargo test -p eggsec-cli --features daemon-client` | pass | **31 passed, 0 failed** (11 of them new) |
| `cargo check -p eggsec-daemon --features service` | pass | — |
| `cargo test -p eggsec-daemon --features service` | pass | **20 profile tests passed, 0 failed**; the crate's lib, bin, and integration suites are green |
| `cargo check -p eggsec-runtime` | pass | — |
| `cargo test -p eggsec-runtime --lib` | pass | **89 passed, 0 failed** |
| `cargo clippy -p eggsec-cli --features daemon-client -- -D warnings` | pass | — |
| `cargo clippy -p eggsec-daemon -- -D warnings` | pass | — |
| `cargo clippy -p eggsec-runtime -- -D warnings` | pass | — |
| `bash scripts/check-architecture-guards.sh` | pass | **ALL PASSED** (172 check blocks; 166–168 added alongside this milestone) |
| `make check-deps` | **fail (inherited)** | Pre-existing advisories. See §10. |

**Thirty-one new tests** were added: 11 in the CLI, 6 in the daemon host, 10 in the daemon crate's new `daemon_start_honesty.rs` integration suite, 1 in the daemon binary, and 3 in the runtime crate.

The four new guards were **negative-tested**: each defect was reintroduced into the tree and each guard was confirmed to fail with its expected message, after which the tree was restored and confirmed green.

**Not run:** `make check-python` (requires `.venv-ci/`). `make check` cannot complete because it stops at `make check-deps`; every step before it passes, and the steps after it were run directly and are green.

## 5. Invariant review

| Invariant | Evidence |
|---|---|
| `EnforcementContext::evaluate()` remains the mandatory pre-dispatch gate | The milestone changes no dispatch path. The daemon host's dispatch is untouched; only the `Capabilities` reply changed. |
| Strict surfaces dispatch only via `EnforcedDispatcher::dispatch_execution()` with an `ApprovedExecution` bundle | Untouched. The CLI-hosted daemon uses the same `EggsecRuntimeExecutor` the daemon binary uses, so the enforcement path is the engine's, not a new one. |
| `OperationMetadata` remains the single source of truth for operation policy | Untouched. No catalog entry was added, removed, or re-signed. |
| `RuntimeTaskExecutor`'s existing surface is unchanged | One **additive defaulted** method was added. No existing method was removed or re-signed. The plan's stricter wording ("no method is added") was relaxed by the milestone's own design, and the relaxation is recorded here. |
| The daemon protocol and protocol version are unchanged | Version stays 2; `DaemonCapabilities` is byte-identical. No wire shape changed. |
| The daemon's default build still links no engine | `crates/eggsec-daemon/Cargo.toml`'s `default = []` is unchanged. |
| Ring-only TLS | No TLS crate was added, removed, or reconfigured. |
| Workspace Tokio baseline (`default-features = false`, per-crate feature lists) | No crate's Tokio feature list changed. |
| No `let _ =` and no `filter_map(\|e\| e.ok())` | The new code logs through `tracing`; the outcomes are typed enums, not discarded errors. |
| Spawned tokio tasks carry a 30–300 s timeout wrapper | The readiness wait is bounded by `DAEMON_READINESS_TIMEOUT` (60 s); the server-stop wait by `SERVER_STOP_TIMEOUT` (10 s, a shutdown wait, not long-lived work); the pre-flight by `PREFLIGHT_PROBE_TIMEOUT`. |

## 6. Failure and recovery review

- **Duplicate delivery.** Two simultaneous starts race. The pre-flight narrows the window to the gap between the probe and the bind, but it does not close it — the server still unlinks the socket path before binding. This is the plan's explicitly deferred server-side hardening; see §10.
- **Cancellation races.** The readiness wait is cancellable through the existing `CancellationToken`. A cancellation during the wait produces `ReadyFailure::Cancelled`, not a hang.
- **Daemon restart.** A daemon that is killed leaves its socket path behind. The next start's probe treats an unreachable socket as **not** live, so the start proceeds and the server's own unlink removes the stale path.
- **Partial persistence failure.** Not applicable to the start path — the CLI-hosted daemon uses a no-op store, which is now stated in the documentation rather than implied.
- **Stale generation or lease.** Not applicable; the milestone changes no session or lease machinery.
- **Contention and resource release.** The spawned server task's result is observed on shutdown, so a bind failure is not lost in a detached task.
- **Malformed or unauthorized input.** A daemon that answers with an unexpected protocol version produces `ProtocolMismatch` and the start **fails closed** — it is never accepted silently.
- **Bounded event and artifact behavior.** The readiness loop is bounded in both the number of probes and the wall-clock time.

## 7. Migration and compatibility review

- **Schema migration.** None. No persisted shape changed.
- **Backward compatibility.** A daemon built with the deprecated `full-executor` cargo feature behaves identically to one built with `service`, because `full-executor = ["service"]`. The `--full-executor` **runtime** flag is unchanged, so every existing invocation keeps working.
- **Protocol negotiation.** The version is reported on every successful start, and a mismatch is a failed outcome rather than a warning.
- **Configuration behaviour.** The daemon's socket path, data directory, max-clients, and log-level defaults are unchanged. The CLI's `daemon start` gains no new flag.
- **Rollback limitations.** Reverting this milestone restores the old behaviour — including the lie. The guards would then fail, which is the intended signal.
- **Legacy-path status.** `eggsec daemon start` remains a supported path, but it is now honest about what it hosts and about the fact that it does not persist sessions.

## 8. Security review

- **Authorization enforcement.** Unchanged. The milestone touches the daemon's *advertisement*, not its authorization. The engine's `EnforcementContext` gate remains the only path to execution.
- **Scope checks.** Unchanged.
- **Secret handling.** No secret is read, logged, or printed by the new code. The readiness probe reads only the protocol version and the advertised task-kind count.
- **Path validation.** The socket path is operator-supplied and is used as-is, as before.
- **Privilege boundaries.** Unchanged.
- **Transport checkpoints.** The start path connects over the daemon's own Unix socket, the same transport the GUI uses.
- **Denial-of-service bounds.** The pre-flight probe and the readiness wait are both bounded, so a hung or hostile daemon cannot hang the start path indefinitely.
- **Redaction.** The failure message names the socket path and the cause; neither carries a secret.
- **Audit behaviour.** Unchanged.

One security-relevant consequence is worth recording: the pre-flight check means a start attempt against a live socket now **fails** where it previously displaced the running daemon. That is the intended behaviour, and it is a small availability change for an operator who was relying on the displacement.

## 9. Documentation and operations

- **Updated:** `docs/DAEMON.md` (the "Obtaining the daemon" section, the capabilities restatement, the persistence-claim correction), `CONTRIBUTING.md` (the GUI prerequisite names the recipe), `architecture/daemon.md` (the derived-capability, readiness, and pre-flight invariants, plus the milestone 008 feature column), `architecture/desktop_gui.md` (the corrected no-op-executor claim), `architecture/runtime.md`, `docs/ARCHITECTURE.md`, `docs/ARCHITECTURE_INVARIANTS.md`, `docs/VERIFICATION.md`, `docs/CI_ARCHITECTURE_GUARDS.md`, and the `eggsec-daemon`, `eggsec-gui`, and `eggsec-cli` skills.
- **Operator diagnostics.** `eggsec daemon start` now reports the socket path, the protocol version, and the advertised task-kind count on success; the refusal names the path and the live daemon's protocol version; the failure names the path and the cause.
- **Static guards.** 164, 165, 166, and 167 — all negative-tested.
- **Recovery instructions.** To clear a wedged start, stop the process holding the socket; the start path refuses rather than displaces, so no socket file needs to be removed by hand for a live daemon. A stale socket file from a crashed daemon is cleaned up by the next start.

## 10. Unresolved findings

| Severity | Finding | Impact | Required action |
|---|---|---|---|
| medium | `run_server` still unlinks the socket path before binding (`crates/eggsec-daemon/src/server.rs:89-95`), so a server that reaches the bind step can still displace a live daemon. The pre-flight narrows the window to a race between two simultaneous starts; it does not close it. | Two simultaneous starts can still displace one another. | Server-side hardening — refuse to bind over a live socket. Explicitly deferred by the plan's §7-D-7; it needs its own milestone because it changes the server's bind contract. |
| medium | The `DesktopGuiManual` surface is missing from the engine's permissive set (`crates/eggsec/src/runtime_bridge/executor.rs:70-73` treats only `CliManual` and `TuiManual` as permissive), so a GUI session with a non-explicit scope takes the strict branch and fails closed instead of receiving the default empty scope. | Engine-side defect, outside the GUI crate. A GUI session that does not set an explicit scope fails closed. | **Unowned.** Recorded as known gap 8 in the subsystem roadmap. It needs either a new corrective milestone or a scope addition to an existing one. It is not attributable to this milestone. |
| low | The CLI-hosted daemon links the engine with the `cli` feature only, so eight of the twenty advertised capabilities are feature-gated off and are denied with `FeatureMissing`. | A CLI-hosted daemon is less capable than a `service`-profile daemon binary. | Documented in `docs/DAEMON.md`. Milestone 008's profile is the answer for the daemon binary; milestone 006 owns the client-side resolution. |
| low | `make check-deps` fails on pre-existing advisories (`zip 2.4.2` and the `rustls 0.21.12` family, among others). | The dependency-policy gate is red at the baseline. | **Inherited, not attributable to this milestone.** Recording the exceptions is a maintainer decision (owner + review-by date). No `deny.toml` entry was added here. |
| low | Pre-existing clippy findings in `cargo clippy -p eggsec-daemon --tests -- -D warnings` (unused `NoopStore`/`RuntimeEvent` imports, `items_after_test_module`, and one in `crates/eggsec-daemon/src/store/sqlite.rs`). | Outside `make clippy`'s scope. | Not introduced by this milestone; the new code is clippy-clean. |

## 11. Roadmap disposition

**Milestone closed and next dependency may proceed.** Milestone 006 may proceed on the strength of this milestone: the daemon's published capability list is now derived from what the daemon can actually execute, so the additive enabled-feature handshake that 006 specifies no longer has to work around a host that advertises capabilities it cannot run.

## 12. Registry updates

- `plans/registry.md`: the milestone 009 entry changes from **planned 2026-10-23** to **closed 2026-10-23**, and names this closure record.
- `plans/subsystems/desktop-gui-frontend-roadmap.md`: the §4 current-state section, the milestone 9 entry, and the §12 status table row are updated to the closed state.
- The known-gap list gains the two **medium** findings above (the deferred server-side bind hardening and the unowned `DesktopGuiManual` permissive-set defect), so neither is silently dropped.
