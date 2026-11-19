# Desktop GUI Frontend Milestone 012 — The Daemon's Socket Path and Bind Contract

Status: ready for handoff

Repository baseline: branch `feat/auto-20261022-76692679` at commit `821c8189`
("docs(plans): correct the roadmap's remaining stale approval claims"), with
milestones 001–009 closed, 010–011 implemented, and milestone 012 planned.

Source roadmap:

- `plans/subsystems/desktop-gui-frontend-roadmap.md#milestone-12--the-daemons-socket-path-and-bind-contract`
- `plans/subsystems/desktop-gui-frontend-roadmap.md#4-current-state` (known gaps 5 and 10)

Long-term requirements:

- `plans/000-long-term-specification.md#5-crate-ownership`
- `plans/000-long-term-specification.md#9-verification-contract`
- `plans/000-long-term-specification.md#10-frontend-rules`
- `plans/001-terminology-and-domain-model.md#3-execution-terms`

Applicable ADRs:

- `plans/adrs/ADR-0007-desktop-gui-frontend-boundary.md` (accepted)
- `plans/adrs/ADR-0010-gui-supported-platform-scope.md` (accepted)
- `plans/adrs/ADR-0013-daemon-endpoint-default-and-bind-contract.md` — **to
  author**. This plan states a decision the repository has not yet recorded:
  which crate owns the daemon's socket-path default, what the environment
  override is called, what the precedence is, and what the server's bind
  contract is. §14 stop condition 1 makes this plan **not executable** until
  that ADR is `Status: accepted`.

Primary class: corrective

---

## 0. Baseline and supersession record

This is a **new** plan, not an amendment. The 16 numbered sections below are
the whole of it. This block records what this milestone **inherits**, what it
**closes**, and — equally important — what **already exists and must not be
rebuilt**.

### 0.1 The originating work

| Role | Artefact |
|---|---|
| Originating plan | `plans/implementation/desktop-gui-frontend/009-daemon-obtainability-and-start-honesty.md` |
| Originating closure | `plans/closure/desktop-gui-frontend/009-daemon-obtainability-and-start-honesty-closure.md` |
| Prior deferral (plan) | `plans/implementation/desktop-gui-frontend/004-gui-shell-skeleton.md` |
| Prior deferral (closure) | `plans/closure/desktop-gui-frontend/004-gui-shell-skeleton-closure.md` |

The closure record of 009 is **not rewritten** by this milestone. The
amendment record inside plan 009 (§9, "Deferred, and recorded as deferred") and
the completion record inside the 009 closure stay as they are; this plan
implements the deferral they announce.

### 0.2 The deferred decisions this milestone discharges

**From plan 004 (the GUI shell skeleton), three separate deferrals, quoted
verbatim:**

1. Plan 004 §3 ("How a client finds the daemon"),
   `plans/implementation/desktop-gui-frontend/004-gui-shell-skeleton.md:148-149`:
   *"There is **no** socket-path environment variable and no XDG lookup. The
   default is hardcoded in three independent places"*.
2. Plan 004 §7 (the crate map),
   `plans/implementation/desktop-gui-frontend/004-gui-shell-skeleton.md:450-454`:
   *"No environment variable is introduced in this milestone: the workspace has
   no socket-path env var today, and adding one here would create a second,
   differently-named source of truth. If the maintainers want one, it belongs
   in a corrective plan that also collapses the three duplicated defaults."*
3. Plan 004 §9, `…/004-gui-shell-skeleton.md:993-996`:
   *"**The socket-path default is duplicated in three places** and this
   milestone adds a fourth (the GUI's own default). It MUST match
   `/tmp/eggsec-daemon.sock`. Collapsing the duplication — and introducing an
   environment variable — belongs in a corrective plan, not here."*

**From the 004 closure**, the `medium` row of its §10 "Unresolved findings"
table,
`plans/closure/desktop-gui-frontend/004-gui-shell-skeleton-closure.md:559`:

> `The socket-path default is duplicated in four places (the CLI's --socket,
> the daemon's DaemonConfig::default, the daemon binary's --socket-path, the
> GUI's default). No socket-path environment variable and no XDG lookup exist
> today. … A corrective milestone: collapse the duplication and introduce one
> environment variable.`

Plan 004 named the four sites; two more have since been added (the GUI's two
frontend assets), which is why the roadmap counts **six**
(`plans/subsystems/desktop-gui-frontend-roadmap.md:344`, known gap 5).

### 0.3 The unclosed findings this milestone closes

| # | Finding | Recorded in | Severity | Closed by |
|---|---|---|---|---|
| F1 | `run_server` unlinks the socket path before binding, so a server that reaches the bind step can still displace a live daemon. The pre-flight narrows the window to a race between two simultaneous starts; it does not close it. | 009 closure §10, row 1 (`medium`); roadmap known gap 10 | medium | §6.4, WP-D |
| F2 | The socket-path default is transcribed independently in six production sites. | 004 closure §10 (`medium`); 009 plan §5 out-of-scope bullet at `:489-491`; roadmap known gap 5 | medium | §6.1, WP-A, WP-B, WP-C |
| F3 | No socket-path environment variable exists, so a per-user socket location is not expressible; the repo already half-has a name (`EGGSEC_DAEMON_SOCKET`) that no Rust code honours. | 004 closure §10; `docs/GUI_DISTRIBUTION_HANDOFF.md:164-179`; `architecture/desktop_gui.md:710-717` | medium | §6.1, §6.5, WP-A, WP-B |

**F1's own wording**
(`plans/closure/desktop-gui-frontend/009-daemon-obtainability-and-start-honesty-closure.md:172`):
*"…the server still unlinks the socket path before binding… Required action:
Server-side hardening — refuse to bind over a live socket. Explicitly deferred
by the plan's §7-D-7; it needs its own milestone because it changes the
server's bind contract."* Plan 009's own deferral statement
(`plans/implementation/desktop-gui-frontend/009-daemon-obtainability-and-start-honesty.md:1205-1211`)
says the same thing and adds why: the hardening *"changes the daemon's
behaviour for **every** launcher, including ones outside this repository, and
it interacts with the stale-socket cleanup that the current unlink provides."*

### 0.4 What milestone 009 already delivered — and this milestone MUST NOT rebuild

This is the most important honesty constraint in this plan. **The
"refuse to displace a live daemon" half of the roadmap's exit condition is
already met at the start-path level.** It is met by milestone 009, and the
implementation below is present at the baseline:

| Behaviour | Where it lives at the baseline | Status |
|---|---|---|
| The CLI's `daemon start` probes the path and refuses when a daemon answers | `crates/eggsec-cli/src/daemon_cli.rs:141-160` (`start_daemon` → `DaemonStartOutcome::Refused`) | **exists** |
| The CLI reports the refusal as the command's error (non-zero exit) | `crates/eggsec-cli/src/daemon_cli.rs:399-413` (`start_failure_message`) and `:483-490` (`bail!`) | **exists** |
| The daemon **binary** refuses before touching the store or the socket | `crates/eggsec-daemon/src/main.rs:71-75` (`refuse_live_daemon(&config.socket_path).await?`), function at `:226-241` | **exists** |
| Liveness is a **connect + `Health` exchange**, never a `stat` | `crates/eggsec-daemon/src/host.rs:1315-1356` (`probe_daemon_liveness`), doc rationale at `:1316-1321` | **exists** |
| The probe is bounded | `crates/eggsec-daemon/src/host.rs:1299` (`PREFLIGHT_PROBE_TIMEOUT` = 5 s) | **exists** |
| A stale socket file is **not** a live daemon | `crates/eggsec-daemon/src/host.rs:1309-1312` (`DaemonLiveness::NotLive`); asserted at `crates/eggsec-daemon/tests/daemon_start_honesty.rs:193-240` | **exists** |
| The daemon's socket is owner-only (0600) | `crates/eggsec-daemon/src/server.rs:149-159` | **exists** |
| The start path never claims success for a daemon that did not come up | `crates/eggsec-cli/src/daemon_cli.rs:181-221` (`wait_for_daemon_ready` → `DaemonStartOutcome::Failed`) | **exists** |

**Consequences for this plan.** No work package may re-implement the
pre-flight, replace the connect-probe with a lock file or a PID file, or
change the recorded meaning of `DaemonLiveness`. The **open** work is
narrower and sharper than the roadmap's one-line framing suggests:

- **The gap is server-side.** The pre-flight is a *check before the bind*; it
  is not the bind. `run_server` still unlinks the path unconditionally
  (`crates/eggsec-daemon/src/server.rs:141-147`). Two starts that both pass
  the probe, or a start that passes the probe while another daemon is binding,
  can still have the second silently take the socket. The guarantee has to
  move from "the start path checks first" to "**the bind itself refuses**".
- **The gap is the duplication.** Six production sites transcribe the default
  and one shell script half-honours an env var that no Rust code reads.

### 0.5 The regression evidence that would have caught each finding

A corrective plan must name the evidence whose absence let the defect through.
This milestone adds it:

| Finding | Evidence that would have caught it | Added in |
|---|---|---|
| F1 — the bind displaces a live daemon | A **bind-level** contention test: two `run_server` calls against one socket path; exactly one binds, the other is refused with an actionable error, and the survivor keeps answering `Health`. The existing test `two_concurrent_starts_against_one_socket_path` (`crates/eggsec-daemon/tests/daemon_start_honesty.rs:273-329`) probes liveness **twice** and then asserts the original daemon still answers — it never starts a **second server**, so it exercises the pre-flight, not the bind. That is the evidence that was missing. | §10 "Contention and cancellation tests"; WP-D |
| F2 — the default is transcribed six times | A **static** check that the literal exists in exactly one place, plus per-surface tests that assert each surface resolves the *canonical* value rather than its own copy. | §6.7 (guards 177, 179); §10 "Focused unit tests" |
| F3 — no env override | A test that the resolver honours an explicit value, and that an absent/empty value falls back to the built-in default. | §10 "Focused unit tests"; §10 "Migration and compatibility tests" |

### 0.6 Citation drift this milestone inherits (recorded, not fixed here)

Three records point at code that has since moved. They are recorded here so
the implementer does not "fix" them by accident, and so the closure record can
state the current locations:

| Record | Its citation | Actual location at the baseline |
|---|---|---|
| 009 closure §10 row 1 | `crates/eggsec-daemon/src/server.rs:89-95` | `crates/eggsec-daemon/src/server.rs:140-147` |
| `docs/GUI_DISTRIBUTION_HANDOFF.md:171` | `crates/eggsec-gui/src/config.rs:17` | `crates/eggsec-gui/src/config.rs:19` |
| `docs/GUI_BUILD.md:381` | `crates/eggsec-gui/src/config.rs:17` | `crates/eggsec-gui/src/config.rs:19` |
| Plan 009 §5 | roadmap `…-roadmap.md:216-222` | `…-roadmap.md:338-350` (the known-gap table) |
| Plan 009 §5 | roadmap `…-roadmap.md:695-716` | `…-roadmap.md:944-965` (the milestone 12 entry) |

The historical plan files under `plans/` are **immutable history** and are not
edited by this milestone.

---

## 1. Objective

Two outcomes, both of them the daemon's **socket contract**:

1. **One canonical default, with one environment override.** The daemon's
   default socket path exists in exactly **one** place in the workspace's
   executable surface, and one environment variable (`EGGSEC_DAEMON_SOCKET`)
   overrides it everywhere, with one stated precedence.

2. **A bind that cannot displace a live daemon.** The daemon's server **refuses
   to bind** over a socket a live daemon is holding, instead of unlinking the
   path and taking it. The refusal is **actionable**: it names the path, says
   a live daemon holds it, and says what the operator can do.

The guarantee moves from "the start path checks first" to "**the bind itself
refuses**". A caller that never runs the pre-flight — a third-party embedder
of the daemon crate, a future launcher, a second daemon racing the first — is
covered by the same rule, because the rule now lives in the bind.

**What this milestone explicitly does not deliver.** It does not add a second
transport, a Windows transport, an XDG lookup, a per-user socket directory, a
lock file, a PID file, or a daemon lifecycle manager. It does not change the
wire protocol. It does not change the GUI's IPC command count.

---

## 2. Why this milestone is ready

**Closed hard dependencies:**

- **Milestone 009** (closed 2026-10-23) — the start-path refusal, the
  connect-probe, the bounded probe timeout, the stale-file semantics and the
  honest start reporting all exist (§0.4). This milestone builds the
  **server-side** half on top of that probe; it does not invent a liveness
  notion.
- **Milestone 004** (closed) — the GUI's socket handling, its `--socket` flag,
  its `DEFAULT_SOCKET_PATH`, and the two frontend transcriptions all exist.
  The 004 closure's `medium` finding is the recorded origin of this milestone.

**Open dependency (blocking):**

- **ADR-0013 does not exist yet.** It must be authored and accepted before
  any file is touched (§14 stop condition 1). The decision it must record is
  the one this plan argues for in §3.5 and §6.1: the owning crate, the env
  var name, the precedence, and the bind contract.

**Stable interface dependencies (all present at the baseline):**

| Interface | Where | Why this milestone needs it |
|---|---|---|
| `probe_daemon_liveness(socket_path, timeout) -> DaemonLiveness` | `crates/eggsec-daemon/src/host.rs:1322-1356` | The bind guard must use the **same** liveness definition the pre-flight uses. One definition, not two. |
| `DaemonLiveness::{Live { protocol_version }, NotLive}` | `crates/eggsec-daemon/src/host.rs:1302-1313` | The refusal message can name the live daemon's protocol version. |
| `PREFLIGHT_PROBE_TIMEOUT` (5 s) | `crates/eggsec-daemon/src/host.rs:1299` | The bind guard's probe must be bounded by the same budget. |
| `run_server(host, shutdown)` | `crates/eggsec-daemon/src/server.rs:136-139` | The bind guard goes here, so every launcher inherits it. |
| `DaemonConfig::socket_path` | `crates/eggsec-daemon/src/config.rs:5-16` (the struct), `:22` (the default) | The config default becomes the canonical constant. |
| `eggsec::cli::Cli::socket` (global `--socket`) | `crates/eggsec/src/cli/mod.rs:245-252` | The CLI's and the TUI's socket path come from here. |
| `GuiOptions::socket_path` | `crates/eggsec-gui/src/config.rs:23-32`, `:34-41` | The GUI's default comes from here. |
| `DaemonStatusView` | `crates/eggsec-gui/src/commands.rs:263-286` | Carries the default to the webview (§6.5). |

**Not a dependency, and worth saying:** the GUI cannot link the daemon crate
(guard 155, `scripts/check-architecture-guards.sh:4571-4600`; ADR-0007), so
the GUI's share of the canonical constant must come from a crate the GUI may
link. §3.5 measures which.

---

## 3. Current implementation evidence

### 3.1 The server-side bind, quoted

`crates/eggsec-daemon/src/server.rs:136-147`:

```rust
pub async fn run_server(
    host: Arc<DaemonHost>,
    shutdown: CancellationToken,
) -> Result<(), DaemonError> {
    let socket_path = &host.config().socket_path;
    if let Err(error) = std::fs::remove_file(socket_path) {
        if error.kind() != std::io::ErrorKind::NotFound {
            tracing::warn!(path = %socket_path, error = %error, "Failed to remove stale daemon socket");
            return Err(error.into());
        }
    }
    let listener = UnixListener::bind(socket_path)?;
```

Three properties of this code, each of which the milestone changes:

1. The unlink is **unconditional** — it removes whatever is at the path, a
   stale file *or* a live daemon's socket, before the bind.
2. The unlink happens **before** the bind, so the bind never sees an occupied
   path and can never fail with `AddrInUse` for that reason.
3. The removal is **not** conditional on the path being stale, and nothing in
   the server consults `probe_daemon_liveness`.

The 0600 permission tightening that follows (`crates/eggsec-daemon/src/server.rs:149-159`)
is unrelated to liveness: its stated reason (`server.rs:149-151`) is that a
permissive umask would leave the socket world-connectable. It is **not** a
liveness signal and this milestone does not repurpose it. The shutdown-time
removal (`crates/eggsec-daemon/src/server.rs:196-199`) is unchanged too.

### 3.2 Every occurrence of the default, measured

Counted by `rg -n '/tmp/eggsec-daemon\.sock'` over the whole worktree at the
baseline. **8 occurrences in `crates/`, 1 in `scripts/`, 14 in
documentation/skills/examples, 17 in immutable plan history — 40 in total.**

**Production sites in `crates/` — 6** (the roadmap's count):

| # | Site | What it is |
|---|---|---|
| 1 | `crates/eggsec-daemon/src/main.rs:20` | `#[arg(short, long, default_value = "/tmp/eggsec-daemon.sock")]` — the daemon binary's `--socket-path` default |
| 2 | `crates/eggsec-daemon/src/config.rs:22` | `socket_path: "/tmp/eggsec-daemon.sock".into(),` — `DaemonConfig::default()` |
| 3 | `crates/eggsec/src/cli/mod.rs:249` | `default_value = "/tmp/eggsec-daemon.sock",` — the engine's global `--socket` default |
| 4 | `crates/eggsec-gui/src/config.rs:19` | `pub const DEFAULT_SOCKET_PATH: &str = "/tmp/eggsec-daemon.sock";` |
| 5 | `crates/eggsec-gui/frontend/app.js:23` | `const DEFAULT_SOCKET = "/tmp/eggsec-daemon.sock";` |
| 6 | `crates/eggsec-gui/frontend/index.html:25` | `value="/tmp/eggsec-daemon.sock"` — the socket form field's value attribute |

**The two test sites in `crates/`** assert the *value*, so they must not
simply be deleted (§10): `crates/eggsec-daemon/src/config.rs:38`
(`assert_eq!(config.socket_path, "/tmp/eggsec-daemon.sock");`) and
`crates/eggsec-gui/src/config.rs:134`
(`assert_eq!(options.socket_path, "/tmp/eggsec-daemon.sock");`).

**The one script site** is `scripts/build-daemon-profile.sh:45`:
`SOCKET_PATH="${EGGSEC_DAEMON_SOCKET:-/tmp/eggsec-daemon.sock}"`.

**Documentation, skills and examples — 14** (these quote the operator-facing
default and are legitimate; §12 updates the ones that make a *false claim*,
not the ones that quote a path):

`architecture/desktop_gui.md:711`; `architecture/daemon.md:455`;
`docs/GUI_BUILD.md:364` and `:381`; `docs/GUI_DISTRIBUTION_HANDOFF.md:84` and
`:168`; `docs/DAEMON.md:118` and `:179`;
`docs/DAEMON_FEATURE_PROFILE.md:178`;
`docs/python/examples/local_vs_daemon_execution.py:38`;
`.opencode/skills/eggsec-gui/SKILL.md:426`, `:427`, `:432`;
`.opencode/skills/eggsec-daemon/SKILL.md:55`.

**Plan history — 12**, in `plans/implementation/desktop-gui-frontend/004`,
`006`, `007`, `009` and `plans/closure/…/004-…-closure.md`. Immutable; not
edited.

**The Python bindings carry no default.** `daemon_connect(py, socket_path:
&str)` (`crates/eggsec-python/src/daemon.rs:400`) takes the path as a required
argument; there is no socket default anywhere in `crates/eggsec-python`.

### 3.3 The environment-override precedent

- `scripts/build-daemon-profile.sh:45` reads `EGGSEC_DAEMON_SOCKET` and falls
  back to the literal. That value is used in exactly **one** place in that
  script: the printed hint at `scripts/build-daemon-profile.sh:514`
  (`echo "  $BINARY_PATH --full-executor --socket-path $SOCKET_PATH"`). It is
  a **build-recipe hint**, not a functional parameter — the script builds the
  binary and prints how to run it.
- **No Rust code honours `EGGSEC_DAEMON_SOCKET`.** A workspace-wide search for
  that exact name at the baseline returns exactly **one** hit:
  `scripts/build-daemon-profile.sh:45`. (`docs/GUI_DISTRIBUTION_HANDOFF.md:176`
  names the *other* spelling, `EGGSEC_SOCKET`, to say it does not exist.)
- The name is therefore **a naming precedent with no implementation**: the repo
  already spells the concept this way, and the script's line is currently a
  promise the binaries do not keep. Adopting the name makes the script true.
- `EGGSEC_SOCKET` is **not** the name. It is named and rejected in
  `docs/GUI_DISTRIBUTION_HANDOFF.md:176`.

A second, unrelated socket path exists in the smoke script:
`scripts/smoke-daemon-local.sh:39` defaults to
`/tmp/eggsec-daemon-smoke.sock` and `:45` overrides it with
`$WORK_DIR/eggsec-daemon.sock`. That is a **deliberately different** path for
an isolated smoke run, not a transcription of the daemon's default; it is
recorded here so the guard's scope is not misread as covering it.

### 3.4 What the start path already does, and where the race still is

The pre-flight in the CLI (`crates/eggsec-cli/src/daemon_cli.rs:141-160`) runs
**before** the server task is spawned (`:162-178`). The daemon binary's
pre-flight (`crates/eggsec-daemon/src/main.rs:75`) runs before the store is
opened and before the socket is touched.

Between the pre-flight and the bind there is a window. Inside that window:

- another daemon can complete its bind, and this daemon's `run_server` then
  unlinks **its** socket and binds its own — the live daemon keeps running
  but is unreachable, and its clients break;
- two daemons can both pass the pre-flight and both reach the bind.

The 009 closure says this precisely
(`plans/closure/desktop-gui-frontend/009-daemon-obtainability-and-start-honesty-closure.md:129`):

> **Duplicate delivery.** Two simultaneous starts race. The pre-flight narrows
> the window to the gap between the probe and the bind, but it does not close
> it — the server still unlinks the socket path before binding. This is the
> plan's explicitly deferred server-side hardening.

The existing contention test
(`crates/eggsec-daemon/tests/daemon_start_honesty.rs:273-329`) runs two
**probes** against one live socket path and asserts that both see a live
daemon and that the daemon still answers `Health`. Its own comment says *"The
live daemon still owns the path: at most one daemon serves it"* — but the test
never starts a second **server**, so it cannot observe the bind behaviour it
describes. That gap is the regression evidence §0.5 requires.

### 3.5 Where a shared constant can live — the dependency graph, measured

The GUI is a leaf application crate. It links **no** engine, daemon, CLI or
TUI crate — guard 155 (`scripts/check-architecture-guards.sh:4571-4600`)
enforces the manifest and the source-level reach, and ADR-0007 records the
boundary. The daemon likewise may not depend on the engine's shared types:
guard 16 (`scripts/check-architecture-guards.sh:271`) rejects `eggsec-core`
in `crates/eggsec-daemon/Cargo.toml`.

The constant must be reachable from **all three** crates that transcribe it:
the engine (which owns the CLI's clap tree), the daemon (which binds), and
the GUI (which connects). Measured from the manifests:

| Candidate | engine | daemon | GUI | reachable by all three? |
|---|---|---|---|---|
| `eggsec-runtime` | yes (`crates/eggsec/Cargo.toml:114`) | yes (`crates/eggsec-daemon/Cargo.toml:55`) | yes (`crates/eggsec-gui/Cargo.toml:39`) | **yes — the only one** |
| `eggsec-daemon-protocol` | **no** | yes | yes | no |
| `eggsec-core` | yes (`crates/eggsec/Cargo.toml:99`) | **no — forbidden by guard 16** | no (only transitively, via `eggsec-output`) | no |
| `eggsec-output` | yes | no | yes | no |
| `eggsec-tool-core` | yes (`crates/eggsec/Cargo.toml:116`) | no | no | no |
| `eggsec-ui-model` | no | no | yes | no |
| `eggsec-frontend-client` | no | no | yes | no |

`eggsec-runtime` is also a **non-optional** dependency of the CLI binary
(`crates/eggsec-cli/Cargo.toml`) and of the TUI
(`crates/eggsec-tui/Cargo.toml:18`), so the TUI's daemon runtime mode — which
reads `cli.socket` (`crates/eggsec-cli/src/main.rs:142`) — inherits the same
value with no extra work.

Two candidates deserve an explicit rejection, because both are defensible
places to put a shared fact and both are blocked by a *recorded* rule rather
than by taste:

- **`eggsec-daemon-protocol`** is the daemon's transport contract — it owns
  `TransportKind::UnixSocket`
  (`crates/eggsec-daemon-protocol/src/protocol.rs:22-27`) and
  `DAEMON_PROTOCOL_VERSION` (`:15`), and is the most *semantically* apt home
  for a socket path. It is blocked because the **engine** cannot depend on it:
  guard 21 (`scripts/check-architecture-guards.sh:361-372`) rejects the
  pattern `eggsec-tui|eggsec-daemon` in `crates/eggsec/Cargo.toml`, and
  `eggsec-daemon-protocol` contains `eggsec-daemon` as a substring. Placing
  the constant there would need either a long-standing guard amended or the
  engine's `--socket` default removed — a larger blast radius than the defect
  warrants.
- **`eggsec-core`** is the workspace's shared-types leaf and has a
  `constants.rs` (`crates/eggsec-core/src/constants.rs`). It is blocked by
  guard 16 for the daemon, as above.

**Recommendation: `eggsec-runtime`.** It is the unique intersection of the
crates that transcribe the default; it needs **no new dependency edge**, no
guard amendment and no new crate; and the constant plus a pure resolver fit
the crate's dependency-light character (guard 22 forbids the runtime from
depending on the engine or the domain crates, which a `&'static str` and two
functions do not touch). The cost is that the runtime crate's charter — task
lifecycle, session host — now also owns a *deployment endpoint* fact. That is
a durable, cross-crate ownership statement, which is why **ADR-0013 is
required** (§14 stop condition 1) rather than assumed.

### 3.6 The GUI's three sites, and what the webview can be told

The GUI is **buildless** — `crates/eggsec-gui/tauri.conf.json` sets
`frontendDist: "frontend"` and `bundle.active: false`, and the crate has no
npm dependency of any kind. It has a restrictive CSP
(`crates/eggsec-gui/tauri.conf.json:23`): `default-src 'self'; script-src
'self'; …; connect-src 'self' ipc: http://ipc.localhost; …`. There is no
build step that could template a value into `index.html`, and no inline
script the CSP would permit. The webview therefore has exactly two honest
ways to learn a value the Rust side owns: the **Tauri IPC** (already there,
already used) or a value baked into the shipped asset (the duplication this
milestone removes).

The IPC surface is **13 commands** (`crates/eggsec-gui/src/commands.rs:1069`
asserts the count; the declared set is at `:1028-1043`). One of them,
`daemon_status` (`crates/eggsec-gui/src/commands.rs:783-837`), already
returns `DaemonStatusView`, which already carries the socket the GUI is
talking to (`socket_path: Option<String>`, `:268-269`). **Adding a field to
that view does not add a command** — the counted set stays 13, and no skill or
roadmap count moves.

The frontend's current use of the default:

| Site | Use |
|---|---|
| `crates/eggsec-gui/frontend/app.js:1672` | `const socket = String(socketPath \|\| "").trim() \|\| DEFAULT_SOCKET;` — `connect()`'s fallback |
| `crates/eggsec-gui/frontend/app.js:1804` | the connect-form submit handler's fallback |
| `crates/eggsec-gui/frontend/app.js:1836` | `boot()`'s automatic connect fallback |
| `crates/eggsec-gui/frontend/app.js:1685-1687` | after a successful connect, the catalog's `socket_path` overwrites the input — the one place the webview already learns a path from Rust |
| `crates/eggsec-gui/frontend/index.html:25` | the input's initial `value` attribute |

`refreshDaemonStatus()` (`crates/eggsec-gui/frontend/app.js:1593-1612`) is
currently called from exactly **one** place: the end of `connect()`
(`:1699`). It is **not** called at boot, so the boot-time automatic connect
(`app.js:1836-1837`) reads the input's HTML `value` attribute. Removing that
attribute therefore requires the boot sequence to learn the default from Rust
**before** it connects.

### 3.7 The CLI's and the daemon's flag surfaces

| Surface | Site | Type |
|---|---|---|
| The engine's global `--socket` | `crates/eggsec/src/cli/mod.rs:245-252` | `pub socket: String` with a `default_value` literal |
| The engine's per-subcommand `--socket` | `crates/eggsec/src/cli/mod.rs:927-941` | `Option<String>` on `DaemonSubcommand::{Start, Status, Stop}` — an explicit override that already wins over the global flag (`crates/eggsec-cli/src/daemon_cli.rs:454`, `:494`, `:524`) |
| The daemon binary's `--socket-path` | `crates/eggsec-daemon/src/main.rs:19-21` | `String` with a `default_value` literal |

The engine's clap dependency already enables the `env` feature
(`crates/eggsec/Cargo.toml:41`: `features = ["derive", "env"]`); the daemon
binary's does not (`crates/eggsec-daemon/Cargo.toml`: `features = ["derive"]`).
For the record, `clap_builder`'s `env` feature is declared as `env = []`
(clap 4.6.7) — it adds **no** crate to the dependency graph, so enabling it
would be a feature-flag change, not a `deny.toml` decision.

This plan does **not** use clap's `env` attribute. The reason is precedence
clarity: the env var's *name* would then be transcribed a second time (as a
clap attribute literal) and the rule would live in two mechanisms — clap's and
the resolver's — which can drift. A single resolver, applied at every
surface, keeps one rule (§6.1).

`clap`'s `default_value_t` accepts an arbitrary expression: `clap_derive`
4.6.1 generates `let val: #ty = #expr;` and converts it with `ToString`
(`clap_derive-4.6.1/src/item.rs:552-598`). The expression's type must be the
**field's** type. Both fields concerned are `String`, and the resolver returns
`String`, so `default_value_t = eggsec_runtime::daemon_socket_path_from_env()`
type-checks. Note that clap caches the computed default in a
`static OnceLock<String>`, so the environment is read **once per process**, at
the first parse — which is what a long-running daemon wants anyway.

---

## 4. Invariants that must not regress

- **I1 — One source of truth for the default.** The daemon's default socket
  path is stated **once** in the workspace's executable surface. No second
  copy, no re-typed literal, no second spelling of the env var's name.
- **I2 — One precedence, everywhere.** Explicit flag → `EGGSEC_DAEMON_SOCKET`
  → built-in default, applied identically by the daemon binary, the CLI, the
  TUI and the GUI.
- **I3 — The liveness definition stays singular.** "Live" means "a daemon
  answered `Health` on this path", decided by
  `probe_daemon_liveness`. No second liveness notion (no `stat`-based check,
  no lock file, no PID file) may be introduced.
- **I4 — A stale socket file is still reclaimable.** A daemon that crashed and
  left its path behind must not block a new daemon. This is an existing,
  tested behaviour (`crates/eggsec-daemon/tests/daemon_start_honesty.rs:193-240`)
  and the bind guard must preserve it.
- **I5 — The refusal is actionable.** A refusal names the path, says a live
  daemon holds it, and tells the operator what to do next. A refusal that
  only says "address already in use" is not a refusal an operator can act on.
- **I6 — The refusal is fail-closed.** When the server cannot determine that
  the path is stale, it does **not** unlink. Unlinking on doubt is the defect
  this milestone removes.
- **I7 — The GUI links no engine, daemon, CLI or TUI crate** (guard 155,
  ADR-0007). The GUI's share of the canonical constant must come from a crate
  it may link; the engine is not one of them.
- **I8 — The GUI's IPC command count does not change.** 13 commands
  (`crates/eggsec-gui/src/commands.rs:1069`). A new field on an existing view
  is not a new command.
- **I9 — The GUI carries no per-operation frontend code** (guard 159,
  `scripts/check-architecture-guards.sh:4783-4830`) and no build step. The
  webview must not grow a hand-written default, a generated bundle, or an npm
  dependency.
- **I10 — No new dependency.** No crate is added to the workspace and no
  `deny.toml` decision is needed. `clap`'s `env` feature is not enabled.
- **I11 — The daemon's local-IPC boundary is unchanged.** The socket stays
  0600, owner-only (`crates/eggsec-daemon/src/server.rs:149-159`), and the
  protocol is untouched.
- **I12 — No enforcement surface is touched.** This milestone changes no
  operation, no policy, no scope rule, and no dispatch path.
  `EnforcementContext::evaluate()` remains the only pre-dispatch gate, and no
  surface's strictness changes.

---

## 5. Scope

### In scope

- The canonical constant and the resolver, in one crate (§6.1).
- The three flag-parsing surfaces (the daemon binary, the engine's CLI, the
  GUI) adopting the resolver (§6.5).
- The daemon's `DaemonConfig::default()` adopting the canonical constant.
- The GUI's three transcriptions removed: the Rust constant becomes a
  re-export, the JS constant is deleted, the HTML `value` attribute is
  deleted (§6.5).
- The GUI's webview learning the default from Rust through the existing
  `daemon_status` command (§6.5).
- The server-side bind guard in `run_server` (§6.4).
- The refusal's error type and its operator-facing message (§6.4, §6.5).
- The CLI's reporting of a server-side refusal (§6.5).
- The build script's hint no longer transcribing the default (§6.5).
- Guards 177–179 (§6.7), the documentation updates (§12), and the skill
  updates (§12).

### Explicitly out of scope

- **Rebuilding the start-path pre-flight.** It exists (§0.4). This milestone
  does not re-implement it, does not move it, and does not change
  `DaemonLiveness`.
- **A lock file, a PID file, or a lease.** I3 forbids a second liveness
  notion; the connect-probe is the recorded decision.
- **An XDG lookup, a per-user socket directory, or `XDG_RUNTIME_DIR`.** The
  roadmap's deliverable boundary is "a single default socket path with an
  environment override". An XDG lookup is a different (larger) decision and
  `docs/GUI_DISTRIBUTION_HANDOFF.md:164-179` explicitly leaves the per-user
  socket location to the distribution workstream.
- **A second transport.** `docs/GUI_DISTRIBUTION_HANDOFF.md:157-163` records
  that a non-Unix transport is an ADR-gated daemon-subsystem change.
- **Changing `DaemonLiveness`'s shape.** Adding an `Indeterminate` variant
  would change the recorded meaning of the pre-flight's `NotLive` and the
  existing test at
  `crates/eggsec-daemon/tests/daemon_start_honesty.rs:206-210`. Not this
  milestone's work.
- **The Python bindings' API.** `daemon_connect` takes an explicit path
  (`crates/eggsec-python/src/daemon.rs:400`); there is no default to change.
- **The `make check-deps` advisory backlog.** Inherited, red at the baseline,
  owned by milestone 015 (§11).
- **The `DesktopGuiManual` permissive-set defect** (009 closure §10 row 2) —
  unowned, recorded as roadmap gap 8, not this milestone's.
- **The daemon's persistence, session store, capability publication, and
  protocol.** Untouched.
- **Plan and closure history.** `plans/implementation/…/004`, `…/009` and the
  closure records are immutable history and are not edited by this milestone.

---

## 6. Required production changes

### 6.1 Core/domain — the canonical constant, the env var name, and the resolver

**New module: `crates/eggsec-runtime/src/daemon_endpoint.rs`**, re-exported
from `crates/eggsec-runtime/src/lib.rs`. The crate owns task lifecycle; this
module is the one place the runtime owns a *deployment endpoint* fact, and
ADR-0013 records that. The module exposes exactly four items (the names are
fixed by this plan so the surfaces, the guards and the closure record agree):

```rust
pub const DAEMON_SOCKET_ENV_VAR: &str = "EGGSEC_DAEMON_SOCKET";
pub const DEFAULT_DAEMON_SOCKET_PATH: &str = "/tmp/eggsec-daemon.sock";
pub fn resolve_daemon_socket_path(env_value: Option<&str>) -> &str;
pub fn daemon_socket_path_from_env() -> String;
```

**The resolver's contract** (this is the whole precedence rule, in one place):

| Input | Output |
|---|---|
| `None` (the variable is unset) | `DEFAULT_DAEMON_SOCKET_PATH` |
| `Some("")` (set but empty) | `DEFAULT_DAEMON_SOCKET_PATH` |
| `Some("   ")` (whitespace only) | `DEFAULT_DAEMON_SOCKET_PATH` |
| `Some("/run/user/1000/eggsec.sock")` | that value, verbatim |
| `Some(" /run/user/1000/eggsec.sock ")` | the **trimmed** value |

An empty or whitespace-only value is **absent**, not an empty path: a daemon
must never be asked to bind the empty path. The trim is part of the rule, so
it lives in the resolver and not at the call sites.

`resolve_daemon_socket_path` is **pure** — it takes the raw value, returns the
resolved one, and never reads the process environment. That is what makes the
precedence testable without mutating the environment (§10). The workspace is
Rust **edition 2021** (`Cargo.toml:33`), where `std::env::set_var` is safe
but process-global and therefore hostile to parallel tests; the pure shape
avoids the problem entirely.

`daemon_socket_path_from_env` is the **only** function in the workspace that
reads `EGGSEC_DAEMON_SOCKET`. It reads the variable once per call and applies
`resolve_daemon_socket_path`. The surfaces call it; they do not read the
environment themselves.

**The env var's name is `EGGSEC_DAEMON_SOCKET`** — the name the repository
already uses in `scripts/build-daemon-profile.sh:45`. It is not
`EGGSEC_SOCKET` (`docs/GUI_DISTRIBUTION_HANDOFF.md:176` names and rejects
that spelling). Adopting the existing name is what turns the script's line
from a promise the binaries do not keep into a fact.

### 6.2 Storage and migrations

None. No persisted representation changes. The daemon's session store, its
audit records and its snapshot files are untouched.

### 6.3 Protocol and DTOs

- **No wire change.** No `ClientCommand`, no `ServerMessage`, no changed
  shape, no protocol version change
  (`DAEMON_PROTOCOL_VERSION = 2`, `crates/eggsec-daemon-protocol/src/protocol.rs:15`).
  The `Health` exchange the bind guard uses already exists.
- **No new IPC command in the GUI.** `daemon_status` gains a field; the
  command count stays 13 (I8).
- **One DTO change**, in the GUI: `DaemonStatusView`
  (`crates/eggsec-gui/src/commands.rs:263-286`) gains a
  `default_socket_path: String` — "the socket path this GUI uses when the
  operator has not chosen one: the `--socket` flag if the operator gave one,
  otherwise the resolved default (environment override, else the built-in
  default). It is the same value the GUI substitutes for an empty socket
  field." It is populated from `state.options.socket_path`
  (`crates/eggsec-gui/src/commands.rs:39-44` — `GuiState::options`).

### 6.4 Runtime and concurrency — the server-side bind guard

**The rule.** `run_server` must bind **without unlinking first**, and must
refuse when the path is held by a live daemon.

The required sequence, in this order:

1. **Bind first, without unlinking.** `UnixListener::bind(socket_path)`.
   - Success → this daemon owns the path. Nothing was removed, so nothing
     could have been displaced.
   - Failure with `std::io::ErrorKind::AddrInUse` → the path is occupied;
     go to step 2.
   - Any other error → return it unchanged. The current code's behaviour for
     a non-`NotFound` removal error (return the error) is preserved in
     substance.
2. **Ask whether the occupant is live**, using the **existing** probe:
   `probe_daemon_liveness(socket_path, PREFLIGHT_PROBE_TIMEOUT)`
   (`crates/eggsec-daemon/src/host.rs:1322`). No new liveness notion (I3).
   - `DaemonLiveness::Live { protocol_version }` → **refuse**. Return a
     dedicated error (below). Do **not** unlink (I6).
   - `DaemonLiveness::NotLive` → the path holds no daemon. Go to step 3.
3. **Reclaim the stale path, once.** Remove the file, ignoring `NotFound`,
   then bind again.
   - Success → this daemon owns the path.
   - Failure with `AddrInUse` → **refuse**. Another daemon took the path
     between the probe and this bind; this daemon does not unlink again and
     does not take the socket. This is the race the milestone closes.
   - Any other error → return it.

**Why this closes the race.** The loser of a bind race always observes the
winner's socket **in place**, and the probe then finds the winner live and
refuses. Today the loser's `remove_file` is what makes the displacement
possible; under the new rule the only unlink happens after a probe has said
*not live*, and a second unlink never happens.

**The residual, stated honestly.** Between the probe in step 2 and the
`remove_file` in step 3, a third actor could remove the stale path and a live
daemon could bind it. The window is one `remove_file` call and it needs a
third actor to open it. It is **not closed** by this milestone, and the closure
record must say so. The pre-bind `AddrInUse` check removes the practical
exposure: a daemon can only bind a path that does not exist, and this
daemon's failed bind proves the path existed at that moment.

**The error type.** `DaemonError` (`crates/eggsec-daemon/src/error.rs:5-17`)
gains a variant that carries the fact, not a formatted string:

```rust
/// The socket path is held by a live daemon.
#[error("refusing to displace the live daemon already serving {socket_path} \
         (protocol version {protocol_version}) — stop it first, or start this \
         daemon on another path (--socket-path <PATH>, or \
         {env_var}=<PATH>)")]
SocketPathHeld {
    socket_path: String,
    protocol_version: u32,
},
```

The display text is the operator-facing refusal and must satisfy I5: it names
the path, says a live daemon holds it, gives the live daemon's protocol
version, and names **both** remedies — stop the running daemon, or use a
different path. The env var's name in the message comes from
`DAEMON_SOCKET_ENV_VAR`, so the message cannot drift from the rule. The
`AddrInUse`-after-reclaim case needs its own message ("another daemon took
the path"); it must not be reported as "a live daemon is serving it" if the
probe did not establish that, so the two claims stay in distinct variants (or
distinct fields).

**The probe's budget.** The bind guard's probe uses
`PREFLIGHT_PROBE_TIMEOUT` (5 s, `crates/eggsec-daemon/src/host.rs:1299`) —
the same bound the pre-flight uses. `run_server` is called inside a spawned
task by both launchers (`crates/eggsec-cli/src/daemon_cli.rs:169-178`; the
daemon binary's own task), so the probe is already inside the task's
lifetime; no new timeout wrapper is required and no spawned task is added.

**What does not change:** the 0600 permission tightening
(`crates/eggsec-daemon/src/server.rs:149-159`), the accept loop, the client
semaphore, and the shutdown-time removal (`:196-199`).

### 6.5 Frontend or operator surface

**6.5.1 The daemon binary** — `crates/eggsec-daemon/src/main.rs:20`:
`#[arg(short, long, default_value_t = eggsec_runtime::daemon_socket_path_from_env())]`
on the existing `socket_path: String` field. The field is `String` and the
resolver returns `String`, which is what `clap_derive` requires
(`clap_derive-4.6.1/src/item.rs:578-590`: the expression is bound as the
field's type). The pre-flight at `crates/eggsec-daemon/src/main.rs:75` is
unchanged.

**6.5.2 The engine's global `--socket`** — `crates/eggsec/src/cli/mod.rs:249`:
the same `default_value_t = eggsec_runtime::daemon_socket_path_from_env()` on
the existing `pub socket: String` field (`:245-252`). The engine already
depends on `eggsec-runtime` (`crates/eggsec/Cargo.toml:114`) — **no new
dependency edge**. The field's type stays `String`, so
`crates/eggsec-cli/src/main.rs:142` and
`crates/eggsec-cli/src/daemon_cli.rs:114` are unaffected. The
per-subcommand overrides at `crates/eggsec/src/cli/mod.rs:927-941` are
`Option<String>` and already win over the global flag; they are unchanged.

**6.5.3 The daemon's config default** — `crates/eggsec-daemon/src/config.rs:22`:

```rust
socket_path: eggsec_runtime::DEFAULT_DAEMON_SOCKET_PATH.to_owned(),
```

`DaemonConfig::default()` stays **pure**: it does not read the environment.
`Default` is a library-level fact, and the two launchers an operator actually
uses both set `socket_path` explicitly from their resolved flag. This is a
deliberate, documented behaviour: *the environment override is applied by the
surfaces that parse the operator's flags; `DaemonConfig::default()` is the
library default.* A third-party embedder that constructs
`DaemonConfig::default()` directly gets the built-in default.

**6.5.4 The GUI's Rust side** — `crates/eggsec-gui/src/config.rs`:

- `:19` becomes a re-export of the canonical constant, so the GUI keeps its
  public name and gains no second value:

  ```rust
  pub use eggsec_runtime::DEFAULT_DAEMON_SOCKET_PATH as DEFAULT_SOCKET_PATH;
  ```

- `:37` (`GuiOptions::default()`) uses
  `eggsec_runtime::daemon_socket_path_from_env()` so the GUI honours the
  operator's environment.
- The usage text (`:96-120`) names the environment variable, so `--help`
  tells the operator that the default can be overridden.

**6.5.5 The GUI's webview** — the three frontend transcriptions go away:

- `crates/eggsec-gui/frontend/index.html:25`: the `value="…"` attribute is
  **removed**. The input starts empty; the webview fills it from Rust.
- `crates/eggsec-gui/frontend/app.js:23`: `const DEFAULT_SOCKET = …` is
  **removed**. The three fallback sites (`:1672`, `:1804`, `:1836`) become
  `dom.socketInput.value` — which may be empty.
- `crates/eggsec-gui/frontend/app.js`: `boot()` learns the default **before**
  the automatic connect: `await refreshDaemonStatus()` → the status view's
  `default_socket_path` populates the input **only if the operator has not
  typed one** → then the automatic connect uses `dom.socketInput.value`.
- `crates/eggsec-gui/frontend/app.js`: `connect(socketPath)` sends
  `dom.socketInput.value` (possibly empty), and the **Rust** side resolves an
  empty socket to the GUI's default. That is the property that makes the
  frontend duplication removable rather than merely relocated: if the IPC call
  fails, the connect still resolves to the right path.

**6.5.6 The GUI's `connect_daemon`** — `crates/eggsec-gui/src/commands.rs:326-330`:

An empty `socket` argument is replaced by `state.options.socket_path` before
`GuiConnection::connect` is called. An explicitly supplied socket is used
verbatim. The existing behaviour for a non-empty socket is unchanged, and the
GUI's own pre-flight (`crates/eggsec-gui/src/commands.rs:344-346`) keeps
using the resolved path.

**6.5.7 The CLI's report of a server-side refusal** — no new code path is
required, but the behaviour must be verified end to end:

- The CLI's pre-flight still produces `DaemonStartOutcome::Refused` for a
  daemon that is live *at probe time* (`crates/eggsec-cli/src/daemon_cli.rs:144-158`).
- A daemon that becomes live *after* the probe now makes the **server task**
  fail with `DaemonError::SocketPathHeld`. That failure travels through
  `wait_for_daemon_ready` → `ReadyFailure::ServerFailed { cause }`
  (`crates/eggsec-cli/src/daemon_cli.rs:197-220`) → `DaemonStartOutcome::Failed`
  → `start_failure_message` (`:399-413`) → `bail!` (`:483-490`), so the
  operator sees the daemon's own actionable refusal text and the command
  exits non-zero.
- `describe_server_result` (used at `:258`) must not flatten the daemon's
  message into something unactionable. The implementer verifies the rendered
  text and adjusts only if it is lossy.

**6.5.8 The build script's hint** — `scripts/build-daemon-profile.sh:45`
and `:514`:

The script's `SOCKET_PATH` exists only to print a hint. With the binaries
honouring `EGGSEC_DAEMON_SOCKET`, the script no longer needs its own copy of
the default. The hint becomes:

- when `EGGSEC_DAEMON_SOCKET` is set: print
  `eggsec-daemon --full-executor --socket-path <that value>`;
- when it is not set: print `eggsec-daemon --full-executor` and let the
  binary apply its own default.

Either way the literal `/tmp/eggsec-daemon.sock` leaves the script. The
script's *behaviour* does not change — it still builds the profile and prints
the same recipe.

### 6.6 Security and authorization

- **No authorization change.** This milestone touches no operation, no
  policy, no scope rule and no dispatch path. `EnforcementContext::evaluate()`
  remains the only pre-dispatch gate, and no surface's strictness changes
  (I12).
- **The socket's local-IPC boundary is unchanged.** The socket stays 0600,
  owner-only (`crates/eggsec-daemon/src/server.rs:149-159`). The bind guard
  does not relax it and does not read or write it.
- **The env override is an operator control, not a privilege escalation.**
  `EGGSEC_DAEMON_SOCKET` chooses *where* the daemon listens; it grants no
  access to anything the operator could not already reach, and the socket's
  permissions still gate who may connect. A hostile value is a path the
  operator's own daemon cannot bind — an ordinary startup failure, reported
  with its cause.
- **A refusal is fail-closed** (I6). The server never unlinks a path it has
  not proven stale.
- **The refusal does not leak.** The message carries the path the operator
  supplied and the live daemon's protocol version — both facts the operator
  already has.

### 6.7 Documentation and static guards

**Three new guards, 177–179**, appended to
`scripts/check-architecture-guards.sh` in the house style of the existing
checks (a numbered comment block stating the invariant, `SECTION_FAIL=0`,
`echo "--- Check NNN: … ---"`, per-item failures, then a PASS/FAIL that
increments `FAIL`). The highest existing guard number is **176**
(`scripts/check-architecture-guards.sh:5874`), so the new ones start at 177.

**The numbers 177–179 are reserved for this milestone**, in the execution order
of the four planned milestones: this milestone takes **177–179**, milestone 013
takes 180, milestone 014 takes 181–189, and milestone 015 takes 190. The
reservation follows the precedent of milestone 007's plan, which took 168
because the guards of milestones 006, 008, and 009 landed first. An
implementer who takes this milestone out of milestone order must re-derive the
numbers from the script's then-current highest, and must not reuse a number
another planned milestone has reserved.

**Guard 177 — the default exists in exactly one place.** It pins that the
literal `/tmp/eggsec-daemon.sock` appears **nowhere** in the workspace's
executable surface except the canonical crate. The check:

- `rg -n --fixed-strings '/tmp/eggsec-daemon.sock'` over `crates/` and
  `scripts/` — **not** the whole repository. Documentation, skills and plan
  history legitimately quote the operator-facing default, and a
  whole-repository scan would false-positive on 31 such occurrences (§3.2).
  `scripts/check_doc_references.py` already skips `plans/` for the same
  reason (`scripts/check_doc_references.py:34`); this guard uses the same
  discipline.
- The hits must be confined to the canonical crate
  (`crates/eggsec-runtime/src/daemon_endpoint.rs`). That crate may hold more
  than one occurrence — the constant and a test that pins its *value* — and
  the guard must not fail on that.
- The GUI's frontend assets (`crates/eggsec-gui/frontend/`) must contain
  **zero** hits. This is the "no third transcription" rule, stated separately
  so the failure message can name it.
- The shell scripts (`scripts/`) must contain **zero** hits, so the build
  script cannot re-introduce a copy.

Implementation note: **do not use the script's `code_search` helper**
(`scripts/check-architecture-guards.sh:4200-4212`) for this guard.
`code_search` strips string literals before matching
(`sed -e 's/"[^"]*"/""/g'`), so it would blank the very literal this guard
looks for and pass vacuously. Use `rg` directly, as guards 159, 160 and
173–176 do.

**Guard 178 — the server refuses to bind over a live socket.** It pins that
`crates/eggsec-daemon/src/server.rs` no longer unlinks the socket path before
binding, and that the bind path consults the liveness probe. The check:

- The file must **not** contain an unconditional `remove_file` of the socket
  path ahead of the first `UnixListener::bind`. The check is positional: the
  offset of the first `remove_file` must be **after** the offset of the first
  `UnixListener::bind`, or the file must contain no `remove_file` of the
  socket path at all.
- The file must reference `AddrInUse` (the refusal branch exists).
- The file must reference the shared probe — `probe_daemon_liveness` — so a
  second liveness notion cannot be introduced (I3).
- The bind path must not reference `flock`, a lock file, or a PID file.

This guard pins the *shape*; the *behaviour* is pinned by the Rust tests in
§10, because a static check cannot observe a race.

**Guard 179 — the environment override has one name and one resolver.** It
pins that the env var's name is stated once and that no surface reads the
environment for it directly. The check:

- `rg -n --fixed-strings 'EGGSEC_DAEMON_SOCKET'` over `crates/` must return
  hits **only** in the canonical crate. Any hit in the daemon, the engine, the
  GUI or the CLI is a second transcription of the name.
- The canonical crate must declare both `DAEMON_SOCKET_ENV_VAR` and
  `DEFAULT_DAEMON_SOCKET_PATH`, and must define `resolve_daemon_socket_path`
  and `daemon_socket_path_from_env`.
- The four surfaces must **reference** the canonical items rather than
  re-deriving them: the daemon binary's `--socket-path`, the engine's global
  `--socket`, the daemon's `DaemonConfig::default()`, and the GUI's
  `GuiOptions::default()` must each name
  `daemon_socket_path_from_env` or `DEFAULT_DAEMON_SOCKET_PATH`.

Each guard is negative-tested with a file backup — **never** `git checkout --`
on a tree with uncommitted work — and the injected violation and the message
it produced are recorded in the closure.

---

## 7. Ordered work packages

Each package is independently verifiable. A package's tests must pass before
the next begins.

### WP-A — The canonical constant and the resolver

**Intent:** one crate owns the daemon's socket-path default, the env var's
name, and the precedence rule.

**Required changes:** add `crates/eggsec-runtime/src/daemon_endpoint.rs` with
the four items of §6.1 and the resolver contract of §6.1; re-export them
from `crates/eggsec-runtime/src/lib.rs`.

**Acceptance evidence:** the resolver's five-row table (§6.1) is asserted by
unit tests; the constant's *value* is pinned by one test in the canonical
crate.

### WP-B — The three flag-parsing surfaces adopt the resolver

**Intent:** the daemon binary, the engine's CLI (and therefore the CLI binary
and the TUI), and the GUI all resolve the same path by the same rule.

**Required changes:** §6.5.1 (daemon binary), §6.5.2 (engine's global
`--socket`), §6.5.3 (`DaemonConfig::default()`), §6.5.4 (the GUI's options
default and usage text).

**Acceptance evidence:** each surface's test asserts it resolves the
**canonical** value, not a copy of it; the two value-pinning assertions at
`crates/eggsec-daemon/src/config.rs:38` and
`crates/eggsec-gui/src/config.rs:134` are rewritten to compare against the
canonical constant, so the literal survives in exactly one place (§10).

### WP-C — The GUI's frontend transcriptions go away

**Intent:** the webview learns the default from Rust, so the JS constant and
the HTML `value` attribute are gone.

**Required changes:** §6.5.5 and §6.5.6 — remove
`crates/eggsec-gui/frontend/app.js:23`, remove the `value` attribute at
`crates/eggsec-gui/frontend/index.html:25`, add `default_socket_path` to
`DaemonStatusView` (§6.3), populate it from `state.options.socket_path`,
resolve an empty socket in `connect_daemon`, and make `boot()` learn the
default before the automatic connect.

**Acceptance evidence:** the IPC-surface count test still asserts 13 (I8); a
Rust test asserts `daemon_status` reports the GUI's default; guard 177's
check over `crates/eggsec-gui/frontend/` is empty.

### WP-D — The server-side bind guard

**Intent:** the bind itself refuses to displace a live daemon.

**Required changes:** §6.4 — the three-step sequence in `run_server`, the new
`DaemonError` variant(s), and the refusal's operator-facing text.

**Acceptance evidence:** the contention test of §10 — two `run_server` calls
against one path, exactly one binds, the other is refused with the actionable
message, and the survivor keeps answering `Health`.

### WP-E — The refusal reaches the operator

**Intent:** the CLI's start path reports a server-side refusal rather than a
generic failure, and the daemon binary's own failure carries the same text.

**Required changes:** §6.5.7 — verify (and, only if lossy, adjust)
`describe_server_result` and `start_failure_message` so the daemon's refusal
text survives to the command's error; no new code path is expected.

**Acceptance evidence:** an end-to-end test that a second start against a live
daemon's path exits non-zero and prints the path, the phrase that names the
live daemon, and at least one remedy.

### WP-F — Guards, documentation, and skills

**Intent:** the two facts this milestone establishes cannot silently rot.

**Required changes:** guards 177–179 (§6.7), each negative-tested; the
documentation and skill updates of §12.

**Acceptance evidence:** each guard's injected violation and its message are
recorded in the closure record.

---

## 8. Failure, cancellation, restart, and contention semantics

| Situation | Required behaviour |
|---|---|
| A live daemon holds the path; a second daemon starts | The second daemon's **bind** fails with `AddrInUse`; the probe finds the daemon live; the second **refuses** with the actionable message and exits non-zero. The live daemon keeps serving and keeps its socket file. |
| A live daemon holds the path; the **pre-flight** sees it first | The existing refusal path (`DaemonStartOutcome::Refused`) applies, unchanged (§0.4). The message must carry the same two remedies as the bind refusal. |
| A stale socket file is left by a crashed daemon | The bind fails with `AddrInUse`; the probe finds no daemon; the file is removed **once**; the bind succeeds. The daemon starts (I4). |
| The path holds a **regular file**, not a socket | The bind fails with `AddrInUse`; the probe cannot connect; the file is removed once; the bind succeeds. Same shape as the stale-socket case. |
| Two daemons start simultaneously against a free path | Exactly one bind succeeds. The loser sees `AddrInUse`, probes, finds the winner live, and refuses. The winner's socket file is never removed. |
| Two daemons start simultaneously against a **stale** path | Both see `AddrInUse`; both probe *not live*; both attempt the reclaim. One removes and binds; the other's second bind fails with `AddrInUse` and it **refuses** without a second unlink. Exactly one daemon serves the path. |
| A third actor removes the stale path between the probe and the reclaim | The residual of §6.4: the reclaiming daemon may take a path a live daemon has just bound. The window is one `remove_file` call and needs a third actor. Recorded as a residual, not as a guarantee. |
| The daemon is killed and restarted on the same path | The restarted daemon finds the stale path, reclaims it, and binds. No operator action is needed (I4). |
| The daemon shuts down cleanly | The socket is removed by the existing shutdown path (`crates/eggsec-daemon/src/server.rs:196-199`). Unchanged. |
| The path's directory does not exist | The bind fails with a non-`AddrInUse` error; the daemon reports it. No unlink is attempted. |
| The path is not writable | The bind fails with a non-`AddrInUse` error; the daemon reports it. No unlink is attempted. |
| The probe times out (5 s) | The probe reports `NotLive` (`crates/eggsec-daemon/src/host.rs:1349-1355`), and the reclaim proceeds — **the same semantics the pre-flight already has** (`NotLive` covers "no socket, a stale socket file, or a daemon that did not answer in time", `crates/eggsec-daemon/src/host.rs:1309-1312`). This is recorded, not changed: a daemon that holds the path but does not answer `Health` within 5 s is treated as not live. I6's fail-closed rule applies to the case where the server **cannot establish** staleness, which is the `AddrInUse`-after-reclaim case. |
| The operator sets `EGGSEC_DAEMON_SOCKET` to an empty value | The resolver treats it as absent; the built-in default applies (§6.1). |
| The operator sets `EGGSEC_DAEMON_SOCKET` to a path the daemon cannot bind | The daemon reports the bind's own error. Nothing is unlinked. |
| The operator sets `EGGSEC_DAEMON_SOCKET` to a path a live daemon holds | The refusal, with the path the operator chose. |
| The CLI's start path is cancelled during the readiness wait | Unchanged: `ReadyFailure::Cancelled` (`crates/eggsec-cli/src/daemon_cli.rs:202-204`), and the half-started daemon is stopped (`:210-212`). |
| The GUI's IPC call fails at boot | The socket field stays empty; `connect_daemon` receives an empty socket and the **Rust** side resolves it to the GUI's default (§6.5.6). The frontend never needs the default to connect correctly. |

---

## 9. Compatibility and migration

- **Wire protocol**: unchanged. No new command, no new event, no changed
  shape, no version change.
- **`DaemonLiveness`**: unchanged. No new variant (§5).
- **The daemon's `DaemonError`**: gains a variant. It is a public enum in the
  daemon crate; the added variant is additive, and the daemon crate's
  in-tree consumers (`crates/eggsec-cli/src/daemon_cli.rs`,
  `crates/eggsec-daemon/src/main.rs`) match on it by construction.
- **`eggsec::cli::Cli::socket`**: the field's type stays `String`; only its
  default changes (from a literal to the resolved value). **No source change
  for any consumer.** The only behavioural difference is that an operator who
  sets `EGGSEC_DAEMON_SOCKET` now gets that path instead of the built-in
  default — which is the point of the milestone.
- **`GuiOptions::socket_path`**: the type stays `String`; the default becomes
  the resolved value.
- **`DaemonStatusView`**: gains a field. Additive; the GUI's IPC command
  count is unchanged (13).
- **The GUI's webview assets**: `app.js` and `index.html` change. The GUI is
  buildless and ships these files directly, so the change ships with the
  binary; there is no asset cache to invalidate beyond the application
  bundle itself.
- **The daemon's config**: `DaemonConfig::default()` returns the same value
  it returns today when no environment variable is set. With
  `EGGSEC_DAEMON_SOCKET` set, the two *launchers* use the operator's path and
  `DaemonConfig::default()` still returns the built-in default — a deliberate,
  documented behaviour (§6.5.3).
- **Storage**: no migration.
- **Configuration fallback**: the built-in default is the fallback for the
  absent, empty and whitespace-only cases (§6.1). There is no deprecation
  window: the env var is new, and nothing is removed.
- **Removal criteria for legacy paths**: the six literal transcriptions and
  the script's copy are **removed** by this milestone. The literal survives
  once, in the canonical crate. Nothing is deprecated in place.

---

## 10. Required tests

### Focused unit tests

**`crates/eggsec-runtime`** (WP-A):

1. `resolve_daemon_socket_path(None)` → the built-in default.
2. `resolve_daemon_socket_path(Some(""))` → the built-in default.
3. `resolve_daemon_socket_path(Some("   \t "))` → the built-in default.
4. `resolve_daemon_socket_path(Some("/run/user/1000/eggsec.sock"))` → that
   value verbatim.
5. `resolve_daemon_socket_path(Some(" /run/user/1000/eggsec.sock "))` → the
   trimmed value.
6. The constant's **value** is pinned: `DEFAULT_DAEMON_SOCKET_PATH ==
   "/tmp/eggsec-daemon.sock"`. This is the one test in the workspace that may
   state the literal, and it lives beside the constant.
7. `DAEMON_SOCKET_ENV_VAR == "EGGSEC_DAEMON_SOCKET"`.

**`crates/eggsec-daemon`** (WP-B):

8. `DaemonConfig::default().socket_path` equals `DEFAULT_DAEMON_SOCKET_PATH` —
   rewritten from the literal at `crates/eggsec-daemon/src/config.rs:38`.
9. The daemon binary's argument parser resolves its `--socket-path` default
   through the resolver. **No test may set the process environment**; the
   environment-dependent path is covered by the subprocess test below.

**`crates/eggsec`** (WP-B):

10. The engine's global `--socket` default equals the canonical constant.
11. An explicit `--socket <PATH>` overrides the default.
12. The per-subcommand `--socket` still overrides the global flag
    (`crates/eggsec/src/cli/mod.rs:927-941`).

**`crates/eggsec-gui`** (WP-B, WP-C):

13. `GuiOptions::from_args([]).socket_path` equals the canonical constant —
    rewritten from the literal at `crates/eggsec-gui/src/config.rs:134`.
14. `GuiOptions::from_args(["--socket", "/tmp/x.sock"])` keeps the operator's
    path.
15. `daemon_status` reports `default_socket_path` equal to the GUI's start-up
    path.
16. `connect_daemon` with an **empty** socket resolves to the GUI's default
    path (§6.5.6).
17. The IPC-surface count test still asserts **13**
    (`crates/eggsec-gui/src/commands.rs:1069`).

### Integration tests

18. **The end-to-end refusal.** A daemon is started on a socket path; a second
    daemon is started against the same path; the second **fails**, the command
    exits non-zero, and the error text contains the path, a phrase naming the
    live daemon, and at least one remedy. The first daemon still answers
    `Health` afterwards.
19. **The GUI's client path still works end to end** against a daemon on a
    **non-default** path: the GUI connects to the path it was given and the
    catalog arrives — proof that the canonical default was not hard-wired
    anywhere.
20. **The CLI's start path with a non-default path** — `eggsec daemon start
    --socket <PATH>` binds that path and the daemon answers `Health`. The
    existing smoke script (`scripts/smoke-daemon-local.sh:39-45`, which uses a
    per-run `$WORK_DIR` path) exercises this; the implementer runs it as
    evidence.

### Restart and recovery tests

21. **The stale-file recovery.** A socket file is left behind with no
    listener; a daemon starts on that path; it succeeds and answers `Health`.
    The existing test at
    `crates/eggsec-daemon/tests/daemon_start_honesty.rs:193-240` asserts this
    for the *probe*; the milestone adds the **bind-level** equivalent, because
    the bind is what now reclaims the path.
22. **The kill-and-restart path.** A daemon is killed (leaving its socket
    file); a new daemon starts on the same path and succeeds. The existing
    test `the_socket_is_removed_on_shutdown`
    (`crates/eggsec-daemon/tests/daemon_start_honesty.rs:332-367`) covers the
    clean-shutdown case; the unclean case is the new one.
23. **The daemon binary's own restart** — the same, through
    `crates/eggsec-daemon/src/main.rs`, so the binary's path is covered and not
    only the library's.

### Contention and cancellation tests

24. **Two servers, one path** (the regression evidence F1 lacked). Two
    `run_server` calls against one socket path: exactly one binds, the other
    returns the refusal error whose text names the path, and the survivor
    answers `Health`. This test **starts a second server** — the existing
    `two_concurrent_starts_against_one_socket_path`
    (`crates/eggsec-daemon/tests/daemon_start_honesty.rs:273-329`) only
    probes twice and never reaches a second bind, which is why the defect
    survived it.
25. **The concurrent-start race, many rounds.** The same shape repeated (for
    example 20 iterations with distinct paths), asserting that exactly one
    server ever binds and that no server ever removes another's socket.
26. **The stale-then-race shape.** Two servers against a path holding a stale
    file: exactly one binds; the other refuses; the survivor serves.
27. **Cancellation.** The CLI's start path cancelled during the readiness
    wait still yields `ReadyFailure::Cancelled` and stops the half-started
    daemon — asserted so the bind guard's new error does not change the
    cancellation path.

### Security and negative tests

28. **The refusal does not unlink.** After a refused bind, the live daemon's
    socket file **still exists** and is still 0600
    (`crates/eggsec-daemon/src/server.rs:152`). A refusal that removed the
    file would be a worse defect than the one being fixed.
29. **A non-`AddrInUse` bind failure does not unlink.** A bind that fails for
    another reason (a missing directory, for instance) leaves the path alone
    and returns the error.
30. **No liveness drift.** The bind guard uses the shared probe: a test
    asserts the guard's refusal path references `probe_daemon_liveness`'s
    `Live` variant (and guard 178 pins the same statically). No second
    liveness notion exists.
31. **The env var cannot widen access.** A daemon started with
    `EGGSEC_DAEMON_SOCKET` pointing at a path it cannot bind fails with that
    bind's error and unlinks nothing.

### Migration and compatibility tests

32. **No environment variable → the built-in default.** The daemon binary, the
    engine's CLI and the GUI all resolve to `DEFAULT_DAEMON_SOCKET_PATH`.
33. **With the environment variable → the operator's path**, observed through
    a **subprocess** (the one place an environment variable is set, because it
    is process-global). The in-process tests never mutate the environment.
34. **An explicit flag beats the environment.** A child process with
    `EGGSEC_DAEMON_SOCKET` set **and** an explicit `--socket` uses the flag's
    value (I2's first rule).
35. **An empty environment value falls back.** A child process with
    `EGGSEC_DAEMON_SOCKET=""` resolves to the built-in default.
36. **The existing suites still pass unchanged in meaning.** The pre-flight's
    tests (`crates/eggsec-daemon/tests/daemon_start_honesty.rs:142-240`,
    `:613-640`; `crates/eggsec-daemon/src/main.rs:300-345`) must pass
    **without** a change of meaning — in particular the stale-file test at
    `:206-210` and the older-daemon detection at `:613-640`.

---

## 11. Required verification commands

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
bash scripts/check-architecture-guards.sh
python3 scripts/check_doc_references.py
make check-msrv
```

`make check` is the mandatory contract. It is expected to stop at the
inherited `check-deps` advisory backlog: `make check-deps` runs
`cargo deny --workspace --all-features check` over the full closure
(`Makefile:152-159`) and is **red at the baseline** on pre-existing advisories.
That backlog belongs to milestone 015 and is **not this milestone's to fix**;
no `deny.toml` entry is added here, because the milestone adds no dependency
(§6.7, I10). The closure record must attribute a `check-deps` failure to the
inherited backlog rather than to this change.

`make check-python` is required only if the Python bindings, stubs, or Python
docs change. This milestone changes **no** Python code. It does touch
`docs/python/examples/local_vs_daemon_execution.py:38` (§12), which is an
example, not a binding — the implementer records whether the maintainers
consider that a Python-doc change and runs the gate if they do.

`make check-msrv` requires `rustup toolchain install 1.89`. The workspace's
`rust-version` is 1.89 (`Cargo.toml`); the milestone adds no syntax or
dependency that could raise it, but the gate is listed because the canonical
constant lands in a crate every workspace member links.

`make check-features-individual` is a deep check, not a per-change gate: the
milestone adds no feature, so it is **not** required, and the implementer
records that decision rather than silently omitting the command.

The implementer runs `bash scripts/smoke-daemon-local.sh` (which uses a
per-run `$WORK_DIR` socket, `scripts/smoke-daemon-local.sh:39-45`) as
end-to-end evidence that a daemon on a **non-default** path still starts and
serves.

**No command in this plan is claimed to pass.** This is a plan; the commands
above are what an implementer must run, and the closure record states their
results.

---

## 12. Documentation updates

- `architecture/desktop_gui.md:710-717` — the known gap "The socket-path
  default is duplicated in six places" is **removed**, and the text is
  replaced by the new fact: one canonical default, one environment override,
  and the webview's path learned from Rust. The gap list is a living record;
  the change is recorded, not erased silently.
- `architecture/daemon.md:455` — the `socket_path` row of the daemon
  configuration table gains the environment override and the precedence.
- `architecture/daemon.md:489` — the line recording that the server-side
  hardening "is deferred to a daemon-subsystem milestone" is corrected: it
  lands here. (The daemon-subsystem roadmap does not exist; the GUI roadmap
  carries it, as gap 10 → milestone 012.)
- `docs/DAEMON.md:118` — the "Socket path" default row gains the environment
  override.
- `docs/DAEMON.md:179` — the transport table's socket row likewise.
- `docs/GUI_DISTRIBUTION_HANDOFF.md:164-179` — the paragraph that states
  "There is no `EGGSEC_SOCKET` environment variable and no `XDG_RUNTIME_DIR`
  lookup anywhere in the workspace" is corrected: there is now one
  environment variable, and the duplication count changes from six to one.
  The XDG statement stays true and is kept.
- `docs/GUI_BUILD.md:364` and `:381` — the recipe and the default's location;
  the cited line `:17` becomes `:19` (§0.6) and the text points at the
  canonical crate.
- `docs/DAEMON_FEATURE_PROFILE.md:178` — the recipe line; the script's hint
  no longer prints `--socket-path` unconditionally (§6.5.8).
- `docs/python/examples/local_vs_daemon_execution.py:38` — the example's
  hardcoded path becomes the environment override or an explicit argument, and
  the example states where the default comes from.
- `docs/CI_ARCHITECTURE_GUARDS.md` — guards 177–179.
- `scripts/build-daemon-profile.sh` — the header comment records that the
  script no longer carries a copy of the default (§6.5.8).
- `.opencode/skills/eggsec-daemon/SKILL.md:55` and
  `.opencode/skills/eggsec-gui/SKILL.md:426`, `:427`, `:432` — the claims
  that the default is a fixed path, and the cited location
  (`crates/eggsec-gui/src/config.rs:19`), are updated to the canonical
  crate and the environment override. Skills are not type-checked; a claim
  this milestone falsifies is corrected in the same change (AGENTS.md).
- `plans/registry.md` — the milestone 012 entry (the plan's own registration
  is the planner's task; the closure updates the status).

**Not edited:** `plans/implementation/desktop-gui-frontend/004-…`,
`…/009-…`, and the closure records of 004 and 009. They are immutable
history.

---

## 13. Acceptance criteria

1. The default socket path is stated **once** in the workspace's executable
   surface, in the crate the engine, the daemon and the GUI all depend on.
2. `EGGSEC_DAEMON_SOCKET` overrides the default in the daemon binary, the
   CLI, the TUI and the GUI, with one precedence: flag → environment →
   built-in default.
3. An absent, empty or whitespace-only environment value resolves to the
   built-in default.
4. The GUI's webview contains **no** copy of the default: the JS constant and
   the HTML `value` attribute are gone, and the webview learns the path from
   Rust through the existing `daemon_status` command.
5. The GUI's IPC command count is still 13.
6. The daemon's server **refuses** to bind over a socket a live daemon is
   holding, and the refusal names the path, says a live daemon holds it, and
   names what the operator can do.
7. A refused bind **does not** remove the live daemon's socket file.
8. A stale socket file is still reclaimed: a daemon starts on a path left
   behind by a crashed daemon.
9. Two servers started against one path: exactly one binds, the other
   refuses, and the survivor keeps serving — asserted by a test that starts a
   **second server**, not one that only probes.
10. The CLI's start path exits non-zero and prints the refusal when a second
    start is refused — by the pre-flight or by the bind.
11. The start-path pre-flight, the connect-probe, the stale-file semantics
    and the 0600 socket permission are **unchanged** in meaning; no lock file
    and no PID file are introduced.
12. Guards 177, 178 and 179 fail on an injected violation and pass when
    restored.
13. Every command in §11 that was run, and its result, is recorded — with any
    pre-existing failure attributed to the inherited backlog rather than to
    this change.
14. No new dependency, no `deny.toml` change, no new crate, no new IPC
    command, no wire change.

---

## 14. Stop conditions

The implementer **stops and reports** rather than improvising if any of these
holds:

1. **ADR-0013 is not `Status: accepted`,** or is accepted with a different
   ownership decision (a different owning crate, a different env var name, a
   different precedence, or a different bind contract). This plan implements
   one specific decision; a changed decision needs a changed plan.
2. **The canonical constant would have to live in `eggsec-daemon`.** The GUI
   cannot link it (guard 155, ADR-0007), so that placement cannot satisfy the
   requirement. If the guards make every placement impossible, stop.
3. **The engine would need a new dependency edge** on
   `eggsec-daemon-protocol` (or anything else) to reach the constant. That
   means the ownership decision changed; §3.5's measurement is the evidence
   to bring to the escalation.
4. **The refusal would need to change the meaning of `DaemonLiveness`** or
   of the existing pre-flight. The recorded semantics of `NotLive` ("no
   socket, a stale socket file left by a crashed daemon, or a daemon that did
   not answer in time", `crates/eggsec-daemon/src/host.rs:1309-1312`) are
   load-bearing for the existing tests.
5. **A second liveness notion would be introduced** — a `stat`-based check, a
   lock file, a PID file, a lease. The connect-probe is the recorded decision
   (I3).
6. **The stale-file reclamation would be broken.** A daemon that crashed and
   left its path behind must still be able to start (I4). If the bind guard
   cannot tell a stale path from a live one, that is a design failure, not a
   reason to unlink on doubt (I6).
7. **The GUI's IPC command count would have to change**, or the webview
   would need a build step, an npm dependency, an inline script, or a second
   copy of the default (I8, I9).
8. **The daemon's socket permission, the wire protocol, or
   `DaemonLiveness`'s shape would change.** None is in scope (§5).
9. **A new dependency would be needed** — including enabling `clap`'s `env`
   feature (I10). The milestone is designed so that no crate is added.
10. **An existing counted set changes** (crates, features, operations,
    aliases, tabs, task kinds, payloads, techniques, endpoints, probes,
    descriptors, IPC commands) in a way this plan did not anticipate. The
    affected skill and `architecture/overview.md` must be updated in the same
    change.
11. **The `make check-deps` advisory backlog would need fixing here.** It is
    inherited and owned by milestone 015 (§11).
12. **The implementation would need to change an enforcement, authorization,
    scope, or dispatch path.** This milestone touches none (I12), and any
    such need means the design diverged from the plan.

---

## 15. Closure evidence required

The closure record
(`plans/closure/desktop-gui-frontend/012-daemon-socket-path-and-bind-contract-closure.md`)
must state, with evidence:

1. The **measured** occurrence count of the literal before and after, by
   category (production code, tests, scripts, documentation, plan history),
   with the search command used.
2. The canonical constant's location, its owning crate, and the dependency
   edges that made that placement necessary (the §3.5 table, re-measured).
3. The env var's name, the resolver's five-row behaviour, and the precedence
   as exercised by the §10 tests.
4. The **bind sequence** as implemented, quoted, with the line numbers of the
   `AddrInUse` branch, the probe call and the reclaim.
5. The contention evidence: the two-server test's output, showing exactly one
   bind and one refusal, and the survivor answering `Health`.
6. The stale-file evidence: the reclaim still works, with the test's output.
7. The refusal's **exact rendered text**, as the operator sees it, from both
   the daemon binary and the CLI.
8. The evidence that a refused bind **did not** remove the live daemon's
   socket file.
9. The three guards' negative-test results, each with the injected violation
   and the message it produced.
10. The exact commands from §11 that were run, and their results.
11. Every command that failed, with the reason and whether it is pre-existing
    (the `check-deps` advisory backlog is expected to be one).
12. The residual risks this milestone accepts, stated as residuals — in
    particular the third-actor reclaim window of §6.4, and the "a daemon that
    does not answer `Health` within 5 s is treated as not live" semantics of
    §8 — rather than as claims of correctness.
13. The counts that changed, and where they are now stated (the GUI's IPC
    command count did **not** change; the duplication count did).
14. The plan's deviations, if any, with the reason for each.
15. The registry and roadmap updates this milestone made
    (`plans/registry.md`, the roadmap's known-gap table, and the milestone's
    status row).

---

## 16. Handoff notes

- **Do not rebuild the start-path pre-flight.** It exists and it is tested
  (§0.4). This milestone's job is the **bind**.
- **Bind first, unlink second.** The order is the whole fix. A guard that
  unlinks first and probes afterwards cannot distinguish "the file was stale"
  from "the file was live when I removed it", because the evidence is gone.
- **Do not use the script's `code_search` helper for guard 177.** It strips
  string literals before matching (`scripts/check-architecture-guards.sh:4204`),
  so it would blank the literal the guard is looking for and pass vacuously.
  Use `rg` directly, and scope the search to `crates/` and `scripts/`: 31
  occurrences of the literal live in documentation, skills and plan history.
- **`clap`'s `default_value_t` binds the expression as the field's type.** The
  field must stay `String` and the resolver must return `String`. The engine's
  `Cli::socket` is public API: its type does not change, so no consumer
  changes. Clap caches the computed default in a `OnceLock`, so the
  environment is read once per process, at the first parse — correct for a
  daemon, harmless for the CLI. Do not add a second, contradictory read.
- **Do not mutate the process environment in an in-process test.** The
  workspace is edition 2021, so `set_var` is safe, but it is process-global
  and the test harness runs threads in parallel. The resolver is pure
  precisely so that this is unnecessary; the one environment-dependent test is
  a subprocess test.
- **The contention test must start a second server.** Probing twice — which
  is what the existing test does — cannot observe the bind defect, which is
  why the defect survived it.
- **A refusal must not remove the socket file.** The live daemon's clients
  depend on that path; removing it is the defect, not a side effect of the
  fix.
- **`DaemonLiveness::NotLive` means "not proven live"**, which includes a
  5-second probe timeout. The milestone does not change that meaning. If a
  future milestone wants "proven dead", that is a new variant and a new ADR.
- **`DaemonConfig::default()` stays pure.** It returns the built-in default
  and does not read the environment; the two launchers set the path
  explicitly. Document this; do not "fix" it by adding an environment read
  to `Default`.
- **The GUI's webview must not learn the default from a copy.** It learns it
  from `daemon_status.default_socket_path`, and an empty socket field is
  resolved by the **Rust** side, so a failed IPC call still connects to the
  right path. `daemon_status` gains a **field**, not a command — the IPC count
  stays 13 (`crates/eggsec-gui/src/commands.rs:1069`). If a new command
  proves genuinely necessary, that is a counted-set change and §14 stop
  condition 10 applies.
- **Negative-test the guards with file backups**, never `git checkout --` on
  a tree with uncommitted work.
- **Do not claim a command passed.** Record what was run and what it
  returned. `make check-deps` is expected to be red on the inherited
  advisories, and that is milestone 015's, not this milestone's.
