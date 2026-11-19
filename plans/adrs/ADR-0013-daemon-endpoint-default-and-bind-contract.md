# ADR-0013: Daemon Endpoint Default and Bind Contract

Status: accepted

Date: 2026-10-24

Ratified: 2026-10-24

Decision owners: project maintainers

Related specification sections:

- `plans/000-long-term-specification.md#4-enforcement-invariants`
- `plans/000-long-term-specification.md#6-dispatch-runtime-and-wire-identity`
- `plans/000-long-term-specification.md#10-frontend-rules`

Affected subsystem roadmaps:

- `plans/subsystems/desktop-gui-frontend-roadmap.md`

## Context

The daemon's socket path had no single owner. At the baseline the literal
`/tmp/eggsec-daemon.sock` was transcribed in six places in the workspace's
executable surface — the daemon binary's `--socket-path` default
(`crates/eggsec-daemon/src/main.rs`), the daemon's `DaemonConfig::default()`
(`crates/eggsec-daemon/src/config.rs`), the engine's global `--socket`
(`crates/eggsec/src/cli/mod.rs`), the GUI's Rust options
(`crates/eggsec-gui/src/config.rs`), the GUI's webview JavaScript constant and
its HTML `value` attribute (`crates/eggsec-gui/frontend/`), and the daemon
profile build script (`scripts/build-daemon-profile.sh`). A change to one copy
would have silently failed to change the others, and the GUI's webview copy was
already known to be able to drift away from the Rust copy.

The repository also *named* an environment variable for this without
implementing it: `scripts/build-daemon-profile.sh` read `EGGSEC_DAEMON_SOCKET`
and fell back to the literal, so the script's line was a promise the binaries
did not keep. The alternative spelling `EGGSEC_SOCKET` is named and rejected in
`docs/GUI_DISTRIBUTION_HANDOFF.md`.

The second question is separate but adjacent. `run_server`
(`crates/eggsec-daemon/src/server.rs`) removed the socket path **before** it
bound, unconditionally. A daemon that was already serving that path therefore
lost its socket file to a second daemon's start, and the live daemon kept
running but became unreachable. The pre-flight probe narrows that window; it
does not close it, because the evidence (the socket file) is destroyed before
the bind.

The decision required here is **where the default lives**, **what the
environment override is called and how it is resolved**, and **what the bind
contract is**.

## Decision drivers

- One owner for the default, reachable by the engine, the daemon and the GUI
  without a new dependency edge and without the GUI linking a daemon crate
  (ADR-0007, guard 155).
- One precedence rule, applied identically by every surface, so that a flag, an
  environment value and a built-in default cannot disagree.
- The webview must not hold a copy of a value the Rust side owns; there is no
  build step and no inline script the CSP would permit
  (`crates/eggsec-gui/tauri.conf.json`).
- One liveness notion: "live" means "a daemon answered `Health` on this path",
  decided by `probe_daemon_liveness`
  (`crates/eggsec-daemon/src/host.rs`). No `stat`-based check, no lock file, no
  PID file.
- A refusal must be actionable and fail-closed: the server must not unlink a
  path it has not proven stale.
- A daemon that crashed and left its socket file behind must still be able to
  start on that path.

## Considered options

### Option A — The default lives in `eggsec-daemon`

Rejected. The GUI links no engine, daemon, CLI or TUI crate (ADR-0007, guard
155), so the GUI could not reach the constant without a forbidden dependency
edge, and the GUI's Rust side would have to keep a copy — the duplication the
decision is meant to remove.

### Option B — The default lives in `eggsec-core`

Rejected. `eggsec-core` is blocked for the daemon by guard 16, and it does not
depend on `eggsec-runtime`; adopting it would add an edge rather than remove
one.

### Option C — The default lives in `eggsec-runtime`

Accepted. `eggsec-runtime` is the unique intersection of the crates that
transcribe the default: the engine, the daemon and the GUI already depend on
it, and the CLI depends on it transitively. A `&'static str` constant and two
pure functions need no new edge, no new crate and no `deny.toml` decision.
Guard 22 forbids the runtime from depending on the engine or the domain crates;
this decision does neither. The cost is that the runtime's charter — task
lifecycle and session host — now also owns a *deployment endpoint* fact. That
is a durable cross-crate ownership statement, which is why it is recorded here.

### Option D — Each surface keeps its own copy

Rejected. That is the status quo, and it is the defect: six copies of one
default, and a webview copy that can drift from the Rust copy.

## Decision

**Accepted: Option C — the default lives in `eggsec-runtime`.**

The maintainer ratified this proposal on 2026-10-24. `eggsec-runtime` owns the
daemon's endpoint facts.

1. **One owner.** `crates/eggsec-runtime/src/daemon_endpoint.rs` is the only
   place in the workspace's executable surface that states the daemon's default
   socket path, the environment variable's name, and the precedence rule. The
   module exposes exactly four items:

   ```rust
   pub const DAEMON_SOCKET_ENV_VAR: &str = "EGGSEC_DAEMON_SOCKET";
   pub const DEFAULT_DAEMON_SOCKET_PATH: &str = "/tmp/eggsec-daemon.sock";
   pub fn resolve_daemon_socket_path(env_value: Option<&str>) -> &str;
   pub fn daemon_socket_path_from_env() -> String;
   ```

2. **The environment variable is `EGGSEC_DAEMON_SOCKET`.** That is the name the
   repository already used in `scripts/build-daemon-profile.sh`, so adopting it
   turns a promise into a fact. It is not `EGGSEC_SOCKET`, which is named and
   rejected in `docs/GUI_DISTRIBUTION_HANDOFF.md`.

3. **One precedence, everywhere: explicit flag → `EGGSEC_DAEMON_SOCKET` →
   built-in default.** The rule is applied by the resolver, not by the call
   sites. An absent, empty or whitespace-only environment value is **absent**,
   not an empty path: a daemon must never be asked to bind the empty path. A
   value with surrounding whitespace is trimmed. The trim lives in the
   resolver.

4. **The resolver is pure; the environment is read in one place.**
   `resolve_daemon_socket_path` takes the raw value and returns the resolved
   one, and never touches the process environment — which is what makes the
   precedence testable without mutating a process-global.
   `daemon_socket_path_from_env` is the only function in the workspace that
   reads `EGGSEC_DAEMON_SOCKET`; the surfaces call it and never read the
   environment themselves.

5. **The surfaces resolve, they do not transcribe.** The daemon binary's
   `--socket-path`, the engine's global `--socket` (and therefore the CLI
   binary and the TUI), the daemon's `DaemonConfig::default()` and the GUI's
   `GuiOptions::default()` all name the canonical items. `DaemonConfig::default()`
   stays **pure** — it returns the built-in default and does not read the
   environment; the environment override is applied by the surfaces that parse
   the operator's flags. This is deliberate: `Default` is a library-level fact,
   and a third-party embedder that constructs `DaemonConfig::default()`
   directly gets the built-in default.

6. **The webview holds no copy.** The GUI's JavaScript constant and the HTML
   `value` attribute are gone. The webview learns the path from Rust through
   the **existing** `daemon_status` command, which gains a
   `default_socket_path` field — a field, not a new command, so the IPC
   command count stays at 13. An empty socket field is resolved by the **Rust**
   side of `connect_daemon` to the GUI's own default, so the connection still
   targets the right path even when the IPC call that would have filled the
   field failed.

7. **The bind contract: bind first, refuse over a live daemon, reclaim only a
   proven-stale path.** `run_server` binds the socket path **without unlinking
   it first**. If the bind fails with `AddrInUse`, the server asks the shared
   probe whether the occupant is live:

   - **Live** → the server **refuses**. It returns a dedicated error that names
     the path, says a live daemon holds it, gives that daemon's protocol
     version, and names both remedies — stop the running daemon, or use a
     different path. The socket file is **not** removed: the live daemon's
     clients depend on it.
   - **Not live** → the path holds no daemon. The server removes the file
     **once** and binds again. If that second bind fails with `AddrInUse`,
     another daemon took the path in between; the server refuses with a
     distinct claim ("another daemon took the path") and does not unlink
     again.

   The probe is `probe_daemon_liveness` with the pre-flight's own bound
   (`PREFLIGHT_PROBE_TIMEOUT`, 5 s). The liveness notion is not changed, and
   `DaemonLiveness::NotLive` keeps its recorded meaning — "not proven live" —
   which includes a probe timeout.

8. **A refused bind is fail-closed.** The server never unlinks a path it has
   not proven stale. The residual window — between the probe and the reclaim, a
   third actor could remove the stale path and a live daemon could bind it — is
   one `remove_file` call wide and is stated as a residual, not as a closed
   race.

## Consequences

### Positive

- The daemon's default socket path is stated once in the executable surface,
  in a crate the engine, the daemon, the GUI and the CLI all depend on.
- The environment override is real: `EGGSEC_DAEMON_SOCKET` works for the daemon
  binary, the CLI, the TUI and the GUI, with one precedence.
- The webview cannot drift: it has no copy of the default to drift with.
- A second daemon can no longer displace a live daemon's socket file. The
  loser of a bind race always observes the winner's socket in place, and the
  probe then finds the winner live and refuses.

### Negative

- The runtime crate's charter widens by one module: it now owns a deployment
  endpoint fact (the constant, the variable's name, the precedence rule).
- The daemon's start behaviour changes for launchers outside this repository:
  a start that used to displace a live daemon now refuses. This is the point of
  the decision, and it is a behaviour change for any embedder that relied on
  the displacement.
- `DaemonConfig::default()` is pure while `GuiOptions::default()` is not. The
  asymmetry is deliberate and documented: the GUI's options describe a process
  that is about to connect to a daemon, so they honour the operator's
  environment, while `DaemonConfig` is a library-level default that the
  launchers set explicitly.

### Neutral or deferred

- The wire protocol is unchanged. No `ClientCommand`, no `ServerMessage`, no
  field on either.
- The GUI's IPC command count is unchanged at 13; `daemon_status` gains a
  field.
- `DaemonLiveness`'s shape is unchanged. "Proven dead" is a different notion
  and would be a new variant and a new ADR.
- The reclaim window between the probe and the `remove_file` is not closed. It
  needs a third actor to open and is one call wide.

## Compatibility and migration

- **No dependency change.** No crate is added, no `deny.toml` decision is
  needed, and no existing dependency edge changes. `clap`'s `env` feature is
  **not** enabled: the environment variable's name would then be transcribed a
  second time as a clap attribute literal, and the rule would live in two
  mechanisms that can drift.
- **Public API.** `eggsec-runtime` gains a module and four items. The engine's
  `Cli::socket` keeps its type (`String`); the daemon binary's `socket_path`
  keeps its type (`String`). The GUI keeps its `DEFAULT_SOCKET_PATH` name as a
  re-export.
- **Behavioural change.** A start against a socket path that a live daemon is
  holding now **refuses** instead of displacing the live daemon. The refusal
  exits non-zero and prints an actionable message.
- **No storage migration.** Nothing is persisted.

## Security and reliability implications

- **Authorisation**: unchanged. The environment override chooses *where* the
  daemon listens; it grants no access the operator could not already reach, and
  the socket's owner-only (0600) permissions still gate who may connect.
- **Fail-closed**: the server unlinks only a path a probe has shown to hold no
  daemon.
- **No leak**: the refusal carries the path the operator supplied and the live
  daemon's protocol version — facts the operator already has.
- **No second liveness notion**: no lock file, no PID file, no `stat`-based
  check. The bind guard uses the pre-flight's own probe.

## Verification

- `crates/eggsec-runtime`: unit tests for the resolver's five rows and for the
  constant's value.
- `crates/eggsec-daemon`: the config default resolves to the canonical
  constant; two servers against one path — exactly one binds, the other is
  refused with the actionable message, the survivor keeps answering `Health`,
  and the live daemon's socket file is still there and still 0600.
- `crates/eggsec-gui`: the options default resolves to the canonical constant;
  `daemon_status` reports `default_socket_path`; `connect_daemon` with an empty
  socket resolves to the GUI's default; the IPC command count is still 13.
- `crates/eggsec`: the global `--socket` default resolves to the canonical
  constant, and an explicit `--socket` wins.
- Subprocess tests: with `EGGSEC_DAEMON_SOCKET` set, the operator's path is
  used; with it set to the empty string, the built-in default is used; with it
  set *and* an explicit flag, the flag wins.
- Static guards 177–179 pin that the default exists in one place, that the
  server binds before it unlinks, and that the environment variable has one
  name and one resolver.

## Supersession

This ADR supersedes nothing. It records the ownership decision that milestone
012 of `plans/subsystems/desktop-gui-frontend-roadmap.md` implements; the
implementation plan is
`plans/implementation/desktop-gui-frontend/012-daemon-socket-path-and-bind-contract.md`.
