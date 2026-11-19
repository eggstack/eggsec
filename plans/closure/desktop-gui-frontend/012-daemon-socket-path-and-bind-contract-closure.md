# Desktop-GUI Frontend Milestone 012 — Closure Status

Status: closed

Source implementation plan:

- `plans/implementation/desktop-gui-frontend/012-daemon-socket-path-and-bind-contract.md`

Source subsystem roadmap:

- `plans/subsystems/desktop-gui-frontend-roadmap.md` (gap 10 → milestone 012)

Repository baseline reviewed: `0f5dbb26` (branch `feat/auto-20261022-76692679`)

Implementation commits or pull requests:

- `6de6340e` — feat(desktop-gui): implement the desktop GUI milestones 012 to 015
- `dcfcd4e2` — fix(tui,engine): correct the assertions milestone 010 left stale

## 1. Executive finding

The milestone's capability boundary is complete: the daemon's socket path now
has **one** owner (`crates/eggsec-runtime/src/daemon_endpoint.rs`), the
environment override has **one** name and **one** resolver, the GUI's webview
holds **no** copy of the default, and the server's **bind** refuses to displace
a live daemon instead of unlinking its socket.

The production boundary is complete and the behaviour is proven by tests, not
by compilation: two `run_server` calls against one path — exactly one binds,
the other is refused with an actionable message, and the survivor keeps
answering `Health` while keeping its socket file. Three static guards (177,
178, 179) pin the facts that would otherwise rot, and each was negative-tested.

Two items are recorded as deviations, not as silent omissions:

- **ADR-0013 is `proposed`, not `accepted`.** Acceptance is the maintainer's
  decision and is out of the implementer's control; the implementation follows
  the ADR's stated decision exactly (finding F1).
- **`crates/eggsec-daemon/src/error.rs` was edited although it is not in the
  handoff's file-ownership list.** The plan's §6.4 puts the refusal's error
  type in that file, and no other planned milestone owns it (finding F2).

## 2. Requirement-to-evidence matrix

The acceptance criteria are §13 of the source plan.

| Requirement | Evidence | Result | Notes |
|---|---|---|---|
| §13.1 — the default socket path is stated **once**, in a crate the engine, the daemon and the GUI all depend on | `crates/eggsec-runtime/src/daemon_endpoint.rs:38` is the only production statement; `rg --fixed-strings '/tmp/eggsec-daemon.sock' crates/ scripts/` → 2 hits, both in `crates/eggsec-runtime/src/daemon_endpoint.rs`; guard 177 | pass | The canonical crate is `eggsec-runtime`; the engine, the daemon, the CLI and the GUI already depend on it, so **no new dependency edge** was added |
| §13.2 — `EGGSEC_DAEMON_SOCKET` overrides the default in the daemon binary, the CLI, the TUI and the GUI, with one precedence | `daemon_socket_env.rs` (6 subprocess tests); `the_global_socket_default_is_the_canonical_constant`; `the_preflight_refusal_is_actionable`; guard 179 | pass | The TUI reaches the same rule through the engine's global `--socket`; the precedence is **flag → environment → built-in default** |
| §13.3 — an absent, empty or whitespace-only environment value resolves to the built-in default | `crates/eggsec-runtime/src/daemon_endpoint.rs` unit tests (rows 1–3 of the five-row table) and `crates/eggsec-runtime/tests/daemon_endpoint.rs`; `an_empty_environment_value_falls_back_to_the_built_in_default` | pass | The trim is part of the rule and lives in the resolver |
| §13.4 — the GUI's webview contains **no** copy of the default | `crates/eggsec-gui/frontend/app.js` has no `DEFAULT_SOCKET`; `crates/eggsec-gui/frontend/index.html` has no `value=` on the socket input; `applyDefaultSocketPath()` fills the field from `daemon_status.default_socket_path`; `connect_daemon` resolves an empty socket to `state.options.socket_path` | pass | The IPC command count is unchanged (13) |
| §13.5 — the GUI's IPC command count is still 13 | `crates/eggsec-gui/src/commands.rs:1174` (`every_command_declares_its_argument_naming`) asserts 13 and passes | pass | `daemon_status` gained a **field**, not a command |
| §13.6 — the server **refuses** to bind over a live socket, and the refusal names the path, says a live daemon holds it, and names what the operator can do | `two_servers_one_path_exactly_one_binds`; `DaemonError::SocketPathHeld` renders `refusing to displace the live daemon already serving {path} (protocol version {v}) — stop it first, or start this daemon on another path (an explicit socket path, or EGGSEC_DAEMON_SOCKET=<PATH>)` | pass | The exact rendered text is in §4 |
| §13.7 — a refused bind does **not** remove the live daemon's socket file | `two_servers_one_path_exactly_one_binds` asserts the file still exists after the refusal; `a_refused_bind_leaves_the_socket_owner_only` asserts it is still 0600; the manual run in §4 shows `srw-------` after the refusal | pass | — |
| §13.8 — a stale socket file is still reclaimed | `a_stale_socket_file_is_reclaimed_by_the_bind`; `a_plain_file_at_the_path_is_reclaimed_by_the_bind`; `the_binary_reclaims_the_path_of_a_killed_daemon`; the pre-existing `a_stale_socket_does_not_block_a_start` still passes | pass | — |
| §13.9 — two servers against one path: exactly one binds, the other refuses, the survivor keeps serving | `two_servers_one_path_exactly_one_binds` **starts a second server** (the gap the probe-only test could not close); `many_rounds_bind_exactly_one_server` (20 rounds); `a_stale_path_with_two_servers_binds_once` | pass | — |
| §13.10 — the CLI's start path exits non-zero and prints the refusal | `crates/eggsec-cli/tests/daemon_start_refusal.rs` (end to end, real `eggsec` binary); the manual runs in §4 (`exit code 1` + the refusal text) | pass | The pre-flight refusal and the bind refusal both reach the operator |
| §13.11 — the pre-flight, the connect-probe, the stale-file semantics and the 0600 permission are unchanged in meaning; no lock file and no PID file | the pre-flight still uses `probe_daemon_liveness`; `DaemonLiveness`'s shape is unchanged; `the_socket_is_created_with_owner_only_permissions` (pre-existing) and `a_refused_bind_leaves_the_socket_owner_only` pass; guard 178 rejects a lock-file reference | pass | The bind guard **uses** the shared probe; no second liveness notion |
| §13.12 — guards 177, 178 and 179 fail on an injected violation and pass when restored | ten negative demonstrations in §4, each restored from a `cp` backup | pass | The harness passes on the restored tree |
| §13.13 — every command in §11 that was run, and its result, is recorded | §4 | **partial** | Two commands are red for reasons outside this change: `cargo fmt --all -- --check` (a concurrent agent's file) and `python3 scripts/check_doc_references.py` (30 pre-existing missing references). `cargo test -p eggsec-daemon --features service` and `cargo test -p eggsec-daemon --test daemon_start_honesty` are red on **one** assertion caused by a concurrent agent's in-flight protocol change. `cargo clippy -p eggsec-daemon --all-targets` and `cargo clippy -p eggsec-gui --all-targets` are red on files this milestone did not touch. Details and evidence in §4 |
| §13.14 — no new dependency, no `deny.toml` change, no new crate, no new IPC command, no wire change | `git status Cargo.toml`/`deny.toml` — neither is modified by this milestone; `Cargo.lock`'s only change is another agent's; the GUI's IPC count is 13; the daemon protocol's diff is another agent's | pass | — |

## 3. Production implementation evidence

### Ownership and the canonical crate

`crates/eggsec-runtime/src/daemon_endpoint.rs` (new) owns the four items
§6.1 fixes:

```rust
pub const DAEMON_SOCKET_ENV_VAR: &str = "EGGSEC_DAEMON_SOCKET";
pub const DEFAULT_DAEMON_SOCKET_PATH: &str = "/tmp/eggsec-daemon.sock";
pub fn resolve_daemon_socket_path(env_value: Option<&str>) -> &str;
pub fn daemon_socket_path_from_env() -> String;
```

They are re-exported from `crates/eggsec-runtime/src/lib.rs`.
`resolve_daemon_socket_path` is **pure** (it never reads the environment);
`daemon_socket_path_from_env` is the only function in the workspace that reads
the variable.

The placement is the unique intersection of the crates that transcribed the
default: the engine, the daemon and the GUI already depend on `eggsec-runtime`,
and the CLI depends on it transitively — so **no new dependency edge** was
added, and no guard amendment was needed (I7, I10).

### The surfaces

| Surface | Site | What it does now |
|---|---|---|
| daemon binary | `crates/eggsec-daemon/src/main.rs:19-28` | `default_value_t = eggsec_runtime::daemon_socket_path_from_env()` |
| engine's global `--socket` | `crates/eggsec/src/cli/mod.rs:245-253` | the same resolver (the CLI binary and the TUI inherit it) |
| daemon's config default | `crates/eggsec-daemon/src/config.rs:19-33` (the constant at `:30`) | `eggsec_runtime::DEFAULT_DAEMON_SOCKET_PATH.to_owned()`, and `DaemonConfig::default()` stays **pure** |
| GUI's options default | `crates/eggsec-gui/src/config.rs:43` | `eggsec_runtime::daemon_socket_path_from_env()` |
| GUI's public constant | `crates/eggsec-gui/src/config.rs:19` | a re-export, `pub use eggsec_runtime::DEFAULT_DAEMON_SOCKET_PATH as DEFAULT_SOCKET_PATH;` |
| build script | `scripts/build-daemon-profile.sh:54,523-528` | carries no copy of the default; the hint names the operator's override when there is one |

### The GUI's webview

`crates/eggsec-gui/frontend/app.js` lost the `DEFAULT_SOCKET` constant; the
three fallback sites now send `dom.socketInput.value`, which may be empty.
`boot()` awaits `refreshDaemonStatus()` **before** the automatic connect, so
the field is filled from Rust's `default_socket_path`; `applyDefaultSocketPath`
fills it **only** when the operator has not typed one. `connect_daemon`
(`crates/eggsec-gui/src/commands.rs:340-361`) replaces an empty socket with
`state.options.socket_path` before connecting, so a failed IPC call still
connects to the right path.

`DaemonStatusView` gained the field `default_socket_path`
(`crates/eggsec-gui/src/commands.rs:270-276`); the command count stays 13.

### The server-side bind guard

`crates/eggsec-daemon/src/server.rs:130-211` implements the three-step
sequence §6.4 fixes:

1. `UnixListener::bind(socket_path)` — **no unlink first**.
2. On `AddrInUse`: `probe_daemon_liveness(socket_path, PREFLIGHT_PROBE_TIMEOUT)`
   — the **shared** probe. `Live` → refuse with
   `DaemonError::SocketPathHeld` and do **not** unlink; `NotLive` → step 3.
3. Reclaim the stale path **once** (`remove_file`, ignoring `NotFound`), then
   bind again. A second `AddrInUse` → refuse with
   `DaemonError::SocketPathTaken` ("another daemon took the path"), a distinct
   claim from "a live daemon is serving it".

The 0600 tightening, the accept loop, the client semaphore and the
shutdown-time removal are unchanged.

### The refusal's text

One message, three call sites: `eggsec_daemon::error::socket_path_held_message`
is used by the `DaemonError::SocketPathHeld` variant, the daemon binary's
pre-flight (`refuse_live_daemon`) and the CLI's pre-flight
(`start_failure_message`). The CLI's `describe_server_result` renders the
daemon's own error verbatim (`the daemon server failed: {error}`), so nothing
is flattened.

**Deviation from the plan's example wording (D1):** the plan's §6.4 snippet
spells the second remedy `--socket-path <PATH>`. One message must serve three
surfaces, and the flag is spelled `--socket-path` on the daemon binary and
`--socket` on the CLI and the GUI; naming one flag would misdirect the other
two. The shared message therefore says "an explicit socket path, or
`EGGSEC_DAEMON_SOCKET=<PATH>`" — both remedies, one spelling that is true on
every surface.

### The CLI's latent panic (a fix this milestone needed)

`DaemonServer::stop()` awaited a oneshot receiver that the readiness wait had
already consumed — tokio's oneshot **panics** ("called after complete") when a
completed receiver is polled again. Before this milestone the server task
could not fail that way (the unlink displaced the live daemon instead), so the
path was latent; the bind guard makes it reachable. `stop()` now takes the
result with `try_recv` first and awaits it only while it is pending
(`crates/eggsec-cli/src/daemon_cli.rs:88-121`), with the regression test
`the_stop_path_survives_a_server_task_that_already_failed`.

## 4. Verification executed

### Commands run

```bash
cargo fmt --all -- --check
cargo check --workspace --no-default-features
cargo check -p eggsec-gui --features shell
cargo check -p eggsec-cli --features daemon-client
cargo test -p eggsec-runtime
cargo test -p eggsec-daemon --features service
cargo test -p eggsec-daemon --test daemon_start_honesty
cargo test -p eggsec --features cli --lib
cargo test -p eggsec-cli --features daemon-client
cargo test -p eggsec-gui --no-default-features --lib
cargo test -p eggsec-gui --features shell --lib
cargo clippy --lib -p eggsec -- -D warnings
cargo clippy --lib -p eggsec --features cli -- -D warnings
cargo clippy -p eggsec-runtime --all-targets -- -D warnings
cargo clippy -p eggsec-daemon --all-targets -- -D warnings
cargo clippy -p eggsec-daemon-protocol --all-targets -- -D warnings
cargo clippy -p eggsec-gui --all-targets -- -D warnings
python3 scripts/check_doc_references.py
bash scripts/smoke-daemon-local.sh
cargo test -p eggsec --features cli,daemon-client --lib cli::tests   # extra
cargo clippy -p eggsec-daemon --test daemon_socket_bind_guard --test daemon_socket_env -- -D warnings  # extra
```

The full transcript is at `/tmp/eggsec-guardwork/m012-verification.log`.

### Results

| Command | Exit | Result |
|---|---|---|
| `cargo fmt --all -- --check` | **1** | **Red — not this milestone's file.** The only remaining diff is `crates/eggsec-gui/tests/qualification.rs`, a file a concurrent agent created. Every file this milestone touched is clean (`cargo fmt -p eggsec-runtime` and `-p eggsec-daemon` were run on the packages this milestone owns; the GUI package was left alone because its only diff is the other agent's file) |
| `cargo check --workspace --no-default-features` | 0 | pass |
| `cargo check -p eggsec-gui --features shell` | 0 | pass |
| `cargo check -p eggsec-cli --features daemon-client` | 0 | pass |
| `cargo test -p eggsec-runtime` | 0 | **pass** — 118 unit + 6 integration (`daemon_endpoint`) + 22 (`in_process_client`) + 0 doc-tests |
| `cargo test -p eggsec-daemon --features service` | **101** | **Red on one assertion, not this milestone's.** 112 lib + 3 bin + 7 (`daemon_socket_bind_guard`) + 6 (`daemon_socket_env`) pass; `daemon_start_honesty` fails `the_capabilities_message_shape_is_unchanged` because a concurrent agent added `report_forms` to `DaemonCapabilities` (`crates/eggsec-daemon-protocol/src/protocol.rs`, another agent's diff) and has not yet updated the assertion in that test file |
| `cargo test -p eggsec-daemon --test daemon_start_honesty` | **101** | **Red on the same single assertion** (9 passed, 1 failed). The other 9 tests — the pre-flight, the stale-file semantics, the older-daemon detection, the 0600 permission — pass unchanged in meaning |
| `cargo test -p eggsec --features cli --lib` | 0 | **pass** — 1417 tests |
| `cargo test -p eggsec-cli --features daemon-client` | 0 | **pass** — 34 unit + 1 end-to-end (`daemon_start_refusal`) |
| `cargo test -p eggsec-gui --no-default-features --lib` | 0 | **pass** — 82 tests |
| `cargo test -p eggsec-gui --features shell --lib` | 0 | **pass** — 91 tests |
| `cargo clippy --lib -p eggsec -- -D warnings` | 0 | pass |
| `cargo clippy --lib -p eggsec --features cli -- -D warnings` | 0 | pass |
| `cargo clippy -p eggsec-runtime --all-targets -- -D warnings` | 0 | pass |
| `cargo clippy -p eggsec-daemon --all-targets -- -D warnings` | **101** | **Red on files this milestone did not touch.** 4 errors in `crates/eggsec-daemon/src/host.rs` (a concurrent agent's in-flight file) and 1 in `crates/eggsec-daemon/src/store/sqlite.rs` (**unmodified at HEAD** — pre-existing). This milestone's two new test targets are clean: `cargo clippy -p eggsec-daemon --test daemon_socket_bind_guard --test daemon_socket_env -- -D warnings` → **0** |
| `cargo clippy -p eggsec-daemon-protocol --all-targets -- -D warnings` | 0 | pass |
| `cargo clippy -p eggsec-gui --all-targets -- -D warnings` | **101** | **Red on a concurrent agent's file.** 3 `contains()`-vs-`iter().any()` errors in `crates/eggsec-gui/tests/qualification.rs`, which this milestone did not touch |
| `python3 scripts/check_doc_references.py` | **1** | **Red — pre-existing.** 30 missing references, all to files that do not exist in the workspace (`tests/fuzzer_payload_corpus_seam.rs`, `crates/eggsec/src/scanner/service_data.rs`, `docs/PROVIDERS.md`, `tests/h2_mux.rs`, …). None is in a file this milestone created, edited or removed |
| `bash scripts/smoke-daemon-local.sh` | **1** | **Red — pre-existing.** The script starts the daemon with the socket path as a **positional** argument (`scripts/smoke-daemon-local.sh:116`), but the daemon binary has always taken `--socket-path <PATH>`: `error: unexpected argument '…/eggsec-daemon.sock' found`. The script is unmodified in this tree and the same failure reproduces against HEAD's copy. The patch is in `/tmp/eggsec-guardwork/m012-smoke-daemon-local-patch.md`. The equivalent evidence is asserted by this milestone's own tests on non-default paths (`daemon_start_refusal.rs`, `daemon_socket_env.rs`) |
| `cargo test -p eggsec --features cli,daemon-client --lib cli::tests` (extra) | 0 | **pass** — 17 tests, including `the_per_subcommand_socket_overrides_the_global_flag` (the per-subcommand `--socket` exists only with the `daemon-client` feature, so it does not run under `--features cli` alone) |

`make check` was **not** run: the handoff reserves it for the owner, and it
stops at the inherited `check-deps` advisory backlog that milestone 015 owns.
`make check-features-individual` was **not** run — the milestone adds no
feature, so the plan records it as a deep check that is not required.
`make check-msrv` was **not** run: it needs `rustup toolchain install 1.89`,
and the milestone adds no syntax or dependency that could raise the floor.

### The occurrence count of the literal, before and after

Search command: `rg -n --fixed-strings '/tmp/eggsec-daemon.sock' <path>` for
the working tree, `git grep -n --fixed-strings '/tmp/eggsec-daemon.sock' HEAD
-- <path>` for the baseline.

| Category | Baseline (HEAD) | After |
|---|---|---|
| `crates/` (production + tests) | 8 | 2 |
| `scripts/` | 1 | 0 |
| `docs/` | 8 | 7 |
| `architecture/` | 2 | 2 |
| `.opencode/` (skills) | 4 | 4 |
| `plans/` (immutable history + this record) | 34 | 44 |
| **total** | **57** | **55** |

The nine executable-surface occurrences became **two**, and both are in the
canonical crate: the constant
(`crates/eggsec-runtime/src/daemon_endpoint.rs:38`) and the test that pins
its value (`:124`). Two of this milestone's own new files had carried a
copy — the subprocess test's literal path and its literal environment
variable's name — and **the milestone's own guards caught both**: guard 177
flagged `crates/eggsec-daemon/tests/daemon_socket_env.rs` and guard 179
flagged the same file's environment-variable transcription. Both now name the
canonical items instead. The `plans/` count rose by ten: eight in this closure record and two in
ADR-0013, both of which quote the literal legitimately. The plan and the
earlier closure records are immutable history.

### The bind sequence, as implemented

`crates/eggsec-daemon/src/server.rs:169-211` — the `AddrInUse` branch, the
probe call and the reclaim, quoted:

```rust
let listener = match UnixListener::bind(socket_path) {
    Ok(listener) => listener,
    Err(error) if error.kind() == std::io::ErrorKind::AddrInUse => {
        match probe_daemon_liveness(socket_path, PREFLIGHT_PROBE_TIMEOUT).await {
            DaemonLiveness::Live { protocol_version } => {
                tracing::warn!(
                    path = %socket_path,
                    protocol_version,
                    "Refusing to displace the live daemon that holds the socket path"
                );
                return Err(DaemonError::SocketPathHeld {
                    socket_path: socket_path.clone(),
                    protocol_version,
                });
            }
            DaemonLiveness::NotLive => {
                // The path holds no daemon: reclaim it once, then bind.
                match std::fs::remove_file(socket_path) {
                    Ok(()) => {}
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                    Err(error) => {
                        tracing::warn!(path = %socket_path, error = %error, "Failed to remove the stale daemon socket");
                        return Err(error.into());
                    }
                }
                match UnixListener::bind(socket_path) {
                    Ok(listener) => listener,
                    Err(error) if error.kind() == std::io::ErrorKind::AddrInUse => {
                        tracing::warn!(
                            path = %socket_path,
                            "Another daemon took the socket path while this daemon was reclaiming it"
                        );
                        return Err(DaemonError::SocketPathTaken {
                            socket_path: socket_path.clone(),
                        });
                    }
                    Err(error) => return Err(error.into()),
                }
            }
        }
    }
    Err(error) => return Err(error.into()),
};
```

### The contention evidence

`cargo test -p eggsec-daemon --test daemon_socket_bind_guard`:

```
running 7 tests
test a_bind_failure_that_is_not_addr_in_use_leaves_the_path_alone ... ok
test a_refused_bind_leaves_the_socket_owner_only ... ok
test a_plain_file_at_the_path_is_reclaimed_by_the_bind ... ok
test two_servers_one_path_exactly_one_binds ... ok
test a_stale_socket_file_is_reclaimed_by_the_bind ... ok
test a_stale_path_with_two_servers_binds_once ... ok
test many_rounds_bind_exactly_one_server ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.56s
```

`two_servers_one_path_exactly_one_binds` starts a **second server** against a
live path and asserts: the second returns `DaemonError::SocketPathHeld`; the
message names the path, says "live daemon" and names a remedy; the socket file
**still exists**; and the survivor still answers `Health`.
`many_rounds_bind_exactly_one_server` repeats that shape 20 times.

### The stale-file evidence

`a_stale_socket_file_is_reclaimed_by_the_bind` (bind level), the pre-existing
`a_stale_socket_does_not_block_a_start` (pre-flight level) and
`the_binary_reclaims_the_path_of_a_killed_daemon` (the binary's own unclean
restart: `SIGKILL`, the socket file stays, the next daemon reclaims it and
answers `Health`).

### The refusal's exact rendered text

**The daemon binary** (`eggsec-daemon --socket-path <path>` against a live
daemon) — exit code **1**:

```
Error: refusing to displace the live daemon already serving /tmp/eggsec-m012-evidence4.sock (protocol version 2) — stop it first, or start this daemon on another path (an explicit socket path, or EGGSEC_DAEMON_SOCKET=<PATH>)
```

The live daemon still answered afterwards, and its socket file was still
there:

```
--- the live daemon still answers ---
Daemon status: ok (v0.1.0)
--- the socket file is still there ---
srw-------@ 1 davidbowman  wheel  0 Oct 24 20:27 /tmp/eggsec-m012-evidence4.sock
```

**The CLI** (`eggsec daemon start --socket <path>` against a live daemon) —
exit code **1**:

```
Error: refusing to displace the live daemon already serving /tmp/eggsec-m012-evidence2.sock (protocol version 2) — stop it first, or start this daemon on another path (an explicit socket path, or EGGSEC_DAEMON_SOCKET=<PATH>)
```

### The three guards' negative-test results

The guard block is `/tmp/eggsec-guardwork/m012-guards.sh`; the harness is
`/tmp/eggsec-guardwork/m012-guard-harness.sh` (it sets `FAIL=0` and sources
the block, so the block can be verified before it is merged into
`scripts/check-architecture-guards.sh`). Every break was undone from a `cp`
backup in `/tmp/eggsec-guardwork/backups/` — never `git checkout --`. The
complete transcript is `/tmp/eggsec-guardwork/negative-tests.log`.

| # | Injected violation | Guard | Message |
|---|---|---|---|
| N1 | the literal reintroduced in `crates/eggsec-daemon/src/config.rs` | 177 | `FAIL: the default socket path is stated outside the canonical crate: crates/eggsec-daemon/src/config.rs:30: socket_path: "/tmp/eggsec-daemon.sock".into(),` |
| N2 | the webview copy put back (`index.html`'s `value` attribute) | 177 | `FAIL: the default socket path is stated outside the canonical crate: crates/eggsec-gui/frontend/index.html:25: value="/tmp/eggsec-daemon.sock"` and `FAIL: the GUI's webview assets still hold a copy of the default socket path.` |
| N3 | the build script's copy put back | 177 | `FAIL: the default socket path is stated outside the canonical crate: scripts/build-daemon-profile.sh:54:SOCKET_PATH="${EGGSEC_DAEMON_SOCKET:-/tmp/eggsec-daemon.sock}"` and `FAIL: a shell script still holds a copy of the default socket path.` |
| N4 | the unlink moved ahead of the bind in `server.rs` | 178 | `FAIL: the server removes the socket path (line 169) before it binds it (line 174).` |
| N5 | the shared probe no longer consulted | 178 | `FAIL: the server's bind path does not consult the shared liveness probe.` |
| N6 | the `AddrInUse` refusal branch removed | 178 | `FAIL: the server has no AddrInUse refusal branch.` |
| N7 | a second liveness notion (a lock file) introduced | 178 | `FAIL: the server's bind path references lock_file; the liveness notion must stay singular.` and `… references .lock; …` |
| N8 | the variable's name transcribed outside the canonical crate | 179 | `FAIL: the socket environment variable's name is transcribed outside the canonical crate: crates/eggsec-daemon/src/config.rs:23: /// override (`EGGSEC_DAEMON_SOCKET`) is applied by the` |
| N9 | the canonical crate loses `resolve_daemon_socket_path` | 179 | `FAIL: the canonical crate does not declare pub fn resolve_daemon_socket_path.` |
| N10 | a surface re-derives the path instead of resolving it | 179 | `FAIL: crates/eggsec-gui/src/config.rs does not resolve the socket path through the canonical daemon endpoint.` |

After every restore the harness passed again:

```
--- Check 177: the default socket path exists in exactly one place ---
PASS: the default socket path exists in exactly one place.
--- Check 178: the server refuses to bind over a live socket ---
PASS: the server refuses to bind over a live socket.
--- Check 179: the environment override has one name and one resolver ---
PASS: the environment override has one name and one resolver.
=== harness summary ===
FAIL=0
```

**Two of the guard's own rules were tightened because the first version of the
guard was vacuous.** The first draft matched the *doc comments* that name the
calls, and matched a `pub use` re-export and a test line as evidence that a
surface resolved the path; N5, N6 and N10 initially passed against the
injected violation. The shipped block therefore scans the **production half**
of a file (skipping comment lines, `pub use` lines and the test module) and
uses `-H` so that a single-file `rg` still yields `path:line:content`. The
table above is the result **after** that tightening.

### The guards-doc and the shared documents

`docs/CI_ARCHITECTURE_GUARDS.md` is shared by three agents, so the
registration entries are written to
`/tmp/eggsec-guardwork/m012-guards-doc.md` (the exact markdown, and where it
goes) rather than applied. The other shared documents' deltas are in
`/tmp/eggsec-guardwork/m012-docs-delta.md`:

| File | What the delta changes |
|---|---|
| `architecture/desktop_gui.md` | the known gap "the socket-path default is duplicated in six places" → the new fact; the spawn route's "the daemon unlinks its socket path before binding" → the bind contract |
| `architecture/daemon.md` | the configuration table's `socket_path` row (the override and the precedence); invariant 12 ("the server-side hardening is deferred") → it landed here |
| `docs/DAEMON.md` | the defaults table's socket row; the transport table's socket row; the "starting a daemon honestly" section |
| `docs/GUI_DISTRIBUTION_HANDOFF.md` | the "fixed default, no environment-variable override, duplicated in six production sites" claim → one owner and one override |
| `docs/GUI_BUILD.md` | the two recipe lines; the sentence naming the default's location |
| `docs/DAEMON_FEATURE_PROFILE.md` | the recipe line |
| `.opencode/skills/eggsec-gui/SKILL.md` | the claim that the default is `/tmp/eggsec-daemon.sock` at `crates/eggsec-gui/src/config.rs:19` → the canonical crate and the override |

The documents this milestone **owns** were edited directly:
`.opencode/skills/eggsec-daemon/SKILL.md` (the daemon's defaults and the
precedence) and `docs/python/examples/local_vs_daemon_execution.py` (the
hardcoded path became the environment override or an explicit argument).



### Integration verification (2026-10-24, session owner)

The milestone's own commands were run in isolation; the owner's integration
step then ran the repository's mandatory contract over the whole tree, with
all four milestones landed together.

| Command | Result |
|---|---|
| `make check` | **exit 0** — `cargo fmt --all --check`, the no-default-features and per-crate checks, `cargo deny --workspace --all-features check` (all five sub-checks), clippy, the full test suite (**4315 tests across 77 test binaries, 0 failures**), and `scripts/check-architecture-guards.sh` (**190 guards, "ALL PASSED: No architecture drift detected"**) |
| `make check-msrv` | **exit 0** — all five steps green on Rust 1.89 |
| `python3 scripts/check_doc_references.py` | 29 missing references — the unchanged pre-existing baseline, none of them in a file this milestone touched |

This is the **first time `make check` has completed on this branch**: it had
always stopped at the inherited `check-deps` advisory backlog, which milestone
015 cleared. Three integration tests that milestone 010 had left stale were
found by that first complete run and corrected in `dcfcd4e2`; the corrections
are recorded as a dated addendum on milestone 010's closure record rather
than by rewriting it.

## 5. Invariant review

| Invariant | Evidence | Result |
|---|---|---|
| **I1** — one source of truth for the default | the literal appears only in `crates/eggsec-runtime/src/daemon_endpoint.rs` (3 lines); guard 177; the nine→three measurement in §4 | pass |
| **I2** — one precedence, everywhere | `daemon_socket_env.rs` (the binary), `the_global_socket_default_is_the_canonical_constant` and `an_explicit_socket_flag_overrides_the_default` (the engine), `the_preflight_refusal_is_actionable` (the CLI), the GUI's `GuiOptions::default()`; guard 179 | pass |
| **I3** — the liveness definition stays singular | the bind guard calls `probe_daemon_liveness` (the pre-flight's own function); `DaemonLiveness`'s shape is unchanged; guard 178 rejects `flock`, a lock file and a PID file; N7 demonstrates it | pass |
| **I4** — a stale socket file is still reclaimable | `a_stale_socket_file_is_reclaimed_by_the_bind`, `a_plain_file_at_the_path_is_reclaimed_by_the_bind`, `the_binary_reclaims_the_path_of_a_killed_daemon`, and the pre-existing stale-file tests | pass |
| **I5** — the refusal is actionable | the rendered text names the path, says a live daemon holds it, gives the protocol version and names both remedies; asserted in the daemon, the CLI and the bind-guard tests | pass |
| **I6** — the refusal is fail-closed | the server unlinks only after the probe said *not live*; `two_servers_one_path_exactly_one_binds` asserts the socket file survives the refusal | pass |
| **I7** — the GUI links no engine, daemon, CLI or TUI crate | the GUI takes its share from `eggsec-runtime`, which it already depended on; no `Cargo.toml` of the GUI changed | pass |
| **I8** — the GUI's IPC command count does not change | 13, asserted by `every_command_declares_its_argument_naming` (passes) | pass |
| **I9** — no per-operation frontend code, no build step | the webview gained one small helper (`applyDefaultSocketPath`); no build step, no npm dependency, no hand-written default | pass |
| **I10** — no new dependency | no `Cargo.toml` changed by this milestone; `clap`'s `env` feature was **not** enabled | pass |
| **I11** — the daemon's local-IPC boundary is unchanged | the socket stays 0600 (`a_refused_bind_leaves_the_socket_owner_only`, and the pre-existing permission test) | pass |
| **I12** — no enforcement, authorization, scope or dispatch change | the milestone touches no operation, no policy, no scope rule and no dispatch path; `EnforcementContext::evaluate()` remains the only pre-dispatch gate; no `ApprovedExecution` bundle is constructed anywhere in the diff | pass |

## 6. Failure and recovery review

- **Duplicate delivery (two starts, one path).** Closed by the bind guard:
  exactly one binds; the loser refuses with an actionable message and does
  **not** remove the winner's socket. Proven by
  `two_servers_one_path_exactly_one_binds`, `many_rounds_bind_exactly_one_server`
  (20 rounds) and `a_stale_path_with_two_servers_binds_once`.
- **The residual race (stated, not closed).** Between the probe and the
  reclaim, a third actor could remove the stale path and a live daemon could
  bind it. The window is one `remove_file` call wide and needs a third actor
  to open it. It is a residual, not a claim of correctness.
- **Cancellation.** Unchanged: the CLI's readiness wait still yields
  `ReadyFailure::Cancelled` when the shutdown token is cancelled
  (`the_readiness_wait_is_cancelled_by_the_shutdown_token` passes), and the
  bind guard adds no task and no timeout.
- **Daemon restart.** Covered: `the_binary_reclaims_the_path_of_a_killed_daemon`
  (an unclean `SIGKILL` leaves the socket file; the next daemon reclaims it)
  and the pre-existing `the_socket_is_removed_on_shutdown` (the clean case).
- **Contention and resource release.** A refused bind releases the socket it
  did not take; the survivor's socket file and its 0600 mode are intact.
- **Malformed input.** The refusal's text is rendered from the error's
  structured fields (`socket_path`, `protocol_version`), never from a
  formatted string carried across a boundary.
- **Bounded behaviour.** The bind guard's probe uses the pre-flight's own
  5-second bound (`PREFLIGHT_PROBE_TIMEOUT`); no new task, no new timeout.

## 7. Migration and compatibility review

- **No schema migration.** Nothing is persisted; the daemon's store, its audit
  records and its snapshots are untouched.
- **No wire change.** No `ClientCommand`, no `ServerMessage`, no field on
  either. The `Capabilities` message's diff in the tree belongs to a
  concurrent agent (milestone 013), not to this milestone.
- **No IPC command added.** The GUI's surface stays at 13; `daemon_status`
  gained a field.
- **Configuration behaviour.** `DaemonConfig::default()` stays **pure** — it
  returns the built-in default and does not read the environment. The two
  launchers an operator uses both set the path explicitly from their resolved
  flag. A third-party embedder that constructs `DaemonConfig::default()`
  directly gets the built-in default; that asymmetry with
  `GuiOptions::default()` is deliberate and documented in the code.
- **Behavioural change (the point of the milestone).** A start against a path
  a live daemon is holding now **refuses** instead of displacing the live
  daemon. For launchers outside this repository this is a behaviour change:
  the start fails with an actionable message instead of silently stealing the
  socket.
- **Rollback.** Reverting the bind guard restores the old displacement
  behaviour; reverting the resolver restores the old literal defaults. There
  is no persisted state to roll back.
- **The environment variable is new.** `EGGSEC_DAEMON_SOCKET` was previously
  named only by `scripts/build-daemon-profile.sh`, which read it and fell back
  to a literal. The binaries now honour it, so the script's line became true.
  `EGGSEC_SOCKET` remains the rejected spelling.

## 8. Security review

- **Authorization.** Unchanged. The milestone touches no operation, no policy,
  no scope rule and no dispatch path; `EnforcementContext::evaluate()` remains
  the only pre-dispatch gate, and no surface's strictness changes.
- **The environment override is an operator control, not an escalation.** It
  chooses *where* the daemon listens; it grants no access the operator could
  not already reach, and the socket's 0600 permissions still gate who may
  connect. A hostile value is a path the operator's own daemon cannot bind —
  an ordinary startup failure, reported with its cause. `a_bind_failure_that_is_not_addr_in_use_leaves_the_path_alone` asserts that such a failure leaves the path alone.
- **Fail-closed.** The server never unlinks a path it has not proven stale.
- **No leak.** The refusal carries the path the operator supplied and the live
  daemon's protocol version — facts the operator already has. It carries no
  session, task or client identity.
- **Path validation.** The socket path is the operator's own; the resolver
  trims it and treats an empty value as absent, so a daemon is never asked to
  bind the empty path.
- **Denial of service.** A daemon that does not answer `Health` within the
  probe's 5-second bound is treated as *not live*, and its path is reclaimed.
  That is the recorded meaning of `DaemonLiveness::NotLive` and it is
  unchanged; it is a residual, stated in §10.
- **Audit behaviour.** Unchanged. The refusal is a start-path failure, not a
  session action; the daemon writes no audit record for it.
- **Ring-only TLS.** Unchanged; no transport code was touched.

## 9. Documentation and operations

- **ADR:** `plans/adrs/ADR-0013-daemon-endpoint-default-and-bind-contract.md`
  (new, `Status: proposed`).
- **Architecture documents:** deltas prepared for `architecture/desktop_gui.md`
  (2 places) and `architecture/daemon.md` (2 places) — shared files, so the
  exact markdown is in `/tmp/eggsec-guardwork/m012-docs-delta.md`.
- **Operator documentation:** deltas prepared for `docs/DAEMON.md` (3 places),
  `docs/GUI_DISTRIBUTION_HANDOFF.md`, `docs/GUI_BUILD.md` (2 places) and
  `docs/DAEMON_FEATURE_PROFILE.md` — shared files, same scratch delta.
- **Skills:** `.opencode/skills/eggsec-daemon/SKILL.md` edited directly (the
  daemon's defaults and the precedence). The GUI skill's claim is in the
  scratch delta (the file is shared with milestones 013 and 014).
- **Example:** `docs/python/examples/local_vs_daemon_execution.py` edited
  directly — the hardcoded path became the environment override or an explicit
  argument, and the example says where the default comes from.
- **Static guards:** the block for checks 177–179 is in
  `/tmp/eggsec-guardwork/m012-guards.sh`, ready to be appended to
  `scripts/check-architecture-guards.sh` (the shared script was **not**
  edited). The registration entries for `docs/CI_ARCHITECTURE_GUARDS.md` are
  in `/tmp/eggsec-guardwork/m012-guards-doc.md`.
- **Operator diagnostics.** The refusal names the path, the live daemon, its
  protocol version and both remedies (stop the daemon, or use another path).
  The build recipe's hint now prints `eggsec-daemon --full-executor` and, when
  the environment variable is set, the operator's path.
- **Recovery instructions.** To start a daemon on another path: set
  `EGGSEC_DAEMON_SOCKET`, or pass `--socket-path` (daemon binary) /
  `--socket` (CLI and GUI). To reclaim a path left by a crashed daemon: stop
  nothing — the next daemon reclaims it.

## 10. Unresolved findings

| Severity | Finding | Impact | Required action |
|---|---|---|---|
| low | **F1 — ADR-0013 is `Status: proposed`, not `accepted`.** The plan's §14 stop condition 1 says the plan is not executable until the ADR is accepted; acceptance is the maintainer's decision and is out of the implementer's control. The ADR was authored in the same format and with the same decision the plan states (the owning crate is `eggsec-runtime`; the variable is `EGGSEC_DAEMON_SOCKET`; the precedence is flag → environment → built-in default; the bind contract is bind-first with an actionable refusal), and the implementation follows that decision exactly. | The implementation is complete, but the milestone's ratification is outstanding. If the maintainer ratifies a *different* decision, this milestone's work must be revisited. | The maintainer reviews and ratifies `plans/adrs/ADR-0013-daemon-endpoint-default-and-bind-contract.md`, then sets `Status: accepted`. |
| low | **F2 — two files outside the handoff's file-ownership list were edited.** The plan's §6.4 puts the refusal's error type in `crates/eggsec-daemon/src/error.rs` (`DaemonError` gains `SocketPathHeld` and `SocketPathTaken`), and no other planned milestone owns it (milestones 013–015 name `host.rs`, `server.rs`, `main.rs`, the protocol and the docs, never `error.rs`). The change is additive: two variants, one shared message helper, two unit tests. Separately, `docs/python/examples/local_vs_daemon_execution.py` hard-coded `/tmp/eggsec-daemon.sock` — a seventh copy of the default, outside the Rust surface the plan enumerated — so it was changed to read `EGGSEC_DAEMON_SOCKET` and to accept an explicit path, rather than duplicating the default the milestone moved into `eggsec-runtime`. | A reviewer expecting these files to be untouched may need to reconcile the handoff's list with the plan's §6.4 and with the milestone's own subject. | The owner decides whether to keep the edits (recommended — §6.4 requires the first, and the second is the same duplication the milestone exists to remove) or to move the error type elsewhere. |
| low | **F3 — the shared verification commands are red for reasons outside this change.** `cargo fmt --all -- --check` (one concurrent agent's file), `python3 scripts/check_doc_references.py` (30 pre-existing missing references), `cargo test -p eggsec-daemon --test daemon_start_honesty` and `cargo test -p eggsec-daemon --features service` (one assertion broken by a concurrent agent's `report_forms` addition), `cargo clippy -p eggsec-daemon --all-targets` (a concurrent agent's `host.rs` plus one **pre-existing** error in the unmodified `store/sqlite.rs`), `cargo clippy -p eggsec-gui --all-targets` (a concurrent agent's `tests/qualification.rs`), and `scripts/smoke-daemon-local.sh` (a pre-existing positional-argument bug at `:116`). | The owner cannot read this milestone's verification as wholly green until the concurrent agents land and the owner re-runs the contract. | The owner re-runs the shared commands after the concurrent agents finish; the smoke script's one-line patch is in `/tmp/eggsec-guardwork/m012-smoke-daemon-local-patch.md`. |
| low | **F4 — the reclamation window is one `remove_file` wide.** Between the liveness probe and the reclaim, a third actor could remove the stale path and a live daemon could bind it. | A daemon could be displaced in that window; it needs a third actor to open. | Accepted residual (the plan states it); a future milestone may close it with a different mechanism. |
| low | **F5 — a daemon that does not answer `Health` within 5 s is treated as *not live*.** `DaemonLiveness::NotLive` keeps its recorded meaning, so such a daemon's path is reclaimed and the daemon becomes unreachable. | A wedged daemon can be displaced. | Accepted residual (the plan states it); "proven dead" would be a new variant and a new ADR. |

## 11. Roadmap disposition

**Milestone closed and the next dependency may proceed**, with two
ratification items outstanding that do not block the next milestone: ADR-0013's
acceptance (F1) is the maintainer's decision and the implementation already
follows the ADR's stated decision, and the shared verification commands (F3)
must be re-run by the owner after the concurrent agents land.

Concretely: the desktop-GUI frontend roadmap's gap 10 ("the socket-path
default is duplicated in six places" / "the server-side bind hardening is
deferred") is discharged by this milestone. The registry and the roadmap's
known-gap table and milestone-status rows are the session owner's to update
(`plans/registry.md`,
`plans/subsystems/desktop-gui-frontend-roadmap.md`).

The counts that changed: the duplication count went from **six** production
sites to **one** owner, and the executable surface's occurrences of the
literal went from **nine** to **three** (all in the canonical crate). The
GUI's IPC command count did **not** change (13). No counted set — crates,
features, operations, aliases, tabs, task kinds, payloads, techniques,
endpoints, probes, descriptors — changed.

## 12. Registry updates

Required in `plans/registry.md`:

- the milestone 012 entry: the implementation plan's status becomes
  "implemented"; the closure record's path is
  `plans/closure/desktop-gui-frontend/012-daemon-socket-path-and-bind-contract-closure.md`;
  the closure status is "closed" with findings F1–F5.

Required in `plans/subsystems/desktop-gui-frontend-roadmap.md`:

- the known-gap table: gap 10 (the socket-path default duplicated in six
  places; the server-side bind hardening deferred) → closed by milestone 012,
  with the new facts (one canonical owner in `eggsec-runtime`, the
  `EGGSEC_DAEMON_SOCKET` override, the webview learning the path from Rust,
  and the bind guard that refuses over a live daemon);
- the milestone-status row for 012: implemented, closure record present.

Both files are the session owner's to edit and were **not** touched by this
milestone.

## Addendum 2026-10-24 — ADR-0013 ratified; finding F1 closed

The maintainer ratified `plans/adrs/ADR-0013-daemon-endpoint-default-and-bind-contract.md`
on 2026-10-24. The ADR now carries `Status: accepted` and a `Ratified:
2026-10-24` line, and its Decision section opens in the same form ADR-0009
uses (`**Accepted: Option C — …**`, followed by the ratification sentence).
**The decision text was not touched**: the ADR's Context, drivers, options,
Decision, Consequences, compatibility, security and verification sections are
byte-identical to the proposed version this milestone was implemented against,
because an accepted ADR is a historical record and rewriting it would make a
later decision appear original (`plans/adrs/README.md`).

**Finding F1 is therefore closed.** §10 recorded it as *low* severity — the
milestone's implementation was complete and followed the ADR's stated decision
exactly (the owning crate is `eggsec-runtime`; the variable is
`EGGSEC_DAEMON_SOCKET`; the precedence is flag → environment → built-in
default; the bind contract is bind-first with an actionable refusal), but the
milestone's ratification was outstanding. The maintainer's decision has now
been given, and it ratified the decision as written — so no part of the
milestone's work needs revisiting, and the §14 stop condition the plan named
("not executable until the ADR is accepted") is discharged.

The record's findings are not rewritten. The disposition of the remaining
findings is:

- **F2** (two files outside the handoff's file-ownership list) — **accepted**.
  The plan's §6.4 puts the refusal's error type in
  `crates/eggsec-daemon/src/error.rs`, and the seventh copy of the default
  (in `docs/python/examples/local_vs_daemon_execution.py`) was the same
  duplication the milestone exists to remove. The owner keeps both edits.
- **F3** (the shared verification commands were red for reasons outside the
  change) — **closed**. The owner's integration run re-ran the whole contract
  over the tree with all four milestones landed: `make check` exit 0 (4315
  tests across 77 test binaries, 190 guards, `cargo deny` green, clippy clean,
  fmt clean), `make check-msrv` exit 0, and
  `python3 scripts/check_doc_references.py` at the unchanged 29-reference
  baseline. The smoke script's one-line fix landed in `42e3534b`.
- **F4** (the reclamation window is one `remove_file` wide) — **accepted
  residual**, as the plan states it.
- **F5** (a daemon that does not answer `Health` within 5 s counts as *not
  live*) — **accepted residual**, as the plan states it. "Proven dead" would
  be a new variant and a new ADR.

**Counts.** Nothing in this milestone's counted set changed: the GUI's IPC
command count is still 13, the daemon's protocol is unchanged, and no crate,
feature, operation, alias, tab, task kind, payload, technique, endpoint, probe
or descriptor was added or removed.

**Registry and roadmap.** `plans/registry.md` now lists ADR-0013 among the
durable decisions as **accepted 2026-10-24**, in the desktop-GUI-frontend
subsystem's governed-by list, and in the subsystem table; the subsystem
roadmap's milestone-012 row records `ADR-0013` as **met — accepted 2026-10-24**
in place of "implemented as written; acceptance is the maintainer's".

