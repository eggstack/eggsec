# Desktop GUI platform qualification

This document is the authoritative statement of **which platforms the Eggsec
desktop GUI is qualified on, and for which behaviours**. It exists because
"it compiles on my machine" is not a qualification, and because the two
failure modes that matter for a shipped desktop binary are invisible in a
green CI log: a platform that was never exercised, and a platform that was
exercised but whose result was never recorded.

The desktop GUI is a **daemon-only** frontend (`ADR-0007`): it reaches the
engine only through the daemon's Unix-domain-socket transport, and it links
no engine crate. A GUI that cannot connect cannot do anything, so a
qualification that stopped at "the binary builds" would say almost nothing.

- The build recipe, the platform prerequisites, and the operator procedure
  live in [`docs/GUI_BUILD.md`](GUI_BUILD.md).
- The artefact handed to the distribution workstream is described in
  [`docs/GUI_DISTRIBUTION_HANDOFF.md`](GUI_DISTRIBUTION_HANDOFF.md).
- The supported-platform decision is
  [`ADR-0010`](../plans/adrs/ADR-0010-gui-supported-platform-scope.md).

---

## 1. How to read a cell

Every cell in the matrix below states exactly one status, followed by either
the evidence that supports it or the root cause that prevents it.

| Status | Meaning |
|---|---|
| `qualified` | The behaviour was observed on this platform and a reproducible evidence artefact exists. The cell names the artefact. |
| `partially qualified` | Part of the behaviour was observed. The cell names **both** the qualified subset and the missing part. |
| `not qualified` | The behaviour was not observed, or it failed. The cell names the root cause. |
| `pending M6` | The behaviour cannot exist yet because milestone 006 has not closed. The cell names the blocking milestone. |
| `out of scope` | The platform is excluded by a recorded decision. The cell names the decision and its root cause. |

Two rules are absolute:

- **No cell is blank**, and no cell carries an unqualified support claim. A
  cell that says "works" without a status token, an evidence reference, or a
  root cause is a defect in this document, and the static guard
  (`scripts/check-architecture-guards.sh`, check 168) rejects it.
- **No cell says `qualified` without an evidence reference.** An evidence
  reference is a path plus the command that reproduces it plus the date of
  the run, for example
  `docs/evidence/gui-qualification/macos-2026-10-23.md` (run
  2026-10-23 with `bash scripts/build_gui_release.sh --out /tmp/eggsec-gui-release-test --allow-dirty`),
  or a CI run URL.

---

## 2. The qualification matrix

**Last regenerated:** 2026-10-23 — commit
`7332baeafe5391e228a0b91fec48cf68999729f1`.

| Platform | Build | Launch | Connect | Submit | Cancel | Render | Export |
|---|---|---|---|---|---|---|---|
| `linux` | **not qualified** — no Linux host was executed in this environment; the `gui-platform` CI job on `ubuntu-latest` is the evidence path | **not qualified** — no Linux host was executed in this environment; the `gui-platform` CI job on `ubuntu-latest` is the evidence path | **not qualified** — no Linux host was executed in this environment; the `gui-platform` CI job on `ubuntu-latest` is the evidence path | **not qualified** — no Linux host was executed in this environment; the `gui-platform` CI job on `ubuntu-latest` is the evidence path | **not qualified** — no Linux host was executed in this environment; the `gui-platform` CI job on `ubuntu-latest` is the evidence path | **not qualified** — no Linux host was executed in this environment; the `gui-platform` CI job on `ubuntu-latest` is the evidence path | **pending M6** — milestone 006 owns the export path; the GUI frontend has no export affordance today |
| `macos` | **qualified** — evidence: `docs/evidence/gui-qualification/macos-2026-10-23.md` (run 2026-10-23, `bash scripts/build_gui_release.sh --out /tmp/eggsec-gui-release-test --allow-dirty`); SHA-256 `22b44fc0f087994ec870b0b0fea45a548b5e4780b602156c2fecf83ae4253481` | **partially qualified** — qualified subset: the binary starts, registers with the macOS WindowServer, instantiates the WKWebView (three WebKit XPC helpers), emits no panic, and terminates on `SIGTERM`; missing part: the painted window was never observed (a full-screen capture showed no `eggsec-gui` window and the accessibility query returned zero windows), so the operator-visible window and its contents were not demonstrated; evidence: `docs/evidence/gui-qualification/macos-2026-10-23.md` | **not qualified** — the GUI connects only on an operator action (`connect_daemon` is invoked from the connect form's submit handler, `crates/eggsec-gui/frontend/app.js:1284`), and no windowed operator session or window-automation driver was available in this environment; the repository ships no such driver | **not qualified** — submitting requires a connected session (see the Connect cell), and the daemon's engine-feature profile and obtainability recipe (milestones 008 and 009) had not closed, so no operation could be dispatched | **not qualified** — a cancellation can only be observed against a live task, which requires a connected session and a dispatched task (see the Connect and Submit cells); the event stream's authority (`crates/eggsec-gui/src/commands.rs:432`) is implemented and unit-tested but was not exercised against a live daemon | **not qualified** — rendering a real result requires a completed task carrying a result payload, which requires the Connect and Submit cells; the empty dashboard was not observable either | **pending M6** — milestone 006 owns the export path; the GUI frontend has no export affordance today |
| `windows` | **out of scope** — `ADR-0010` excludes Windows from the desktop GUI's supported platforms; the GUI's unconditional dependency `eggsec-frontend-client` imports `tokio::net::UnixStream` without a `cfg(unix)` gate, so the GUI does not compile for Windows | **out of scope** — `ADR-0010` excludes Windows from the desktop GUI's supported platforms; the GUI's unconditional dependency `eggsec-frontend-client` imports `tokio::net::UnixStream` without a `cfg(unix)` gate, so the GUI does not compile for Windows | **out of scope** — `ADR-0010` excludes Windows from the desktop GUI's supported platforms; the GUI's unconditional dependency `eggsec-frontend-client` imports `tokio::net::UnixStream` without a `cfg(unix)` gate, so the GUI does not compile for Windows | **out of scope** — `ADR-0010` excludes Windows from the desktop GUI's supported platforms; the GUI's unconditional dependency `eggsec-frontend-client` imports `tokio::net::UnixStream` without a `cfg(unix)` gate, so the GUI does not compile for Windows | **out of scope** — `ADR-0010` excludes Windows from the desktop GUI's supported platforms; the GUI's unconditional dependency `eggsec-frontend-client` imports `tokio::net::UnixStream` without a `cfg(unix)` gate, so the GUI does not compile for Windows | **out of scope** — `ADR-0010` excludes Windows from the desktop GUI's supported platforms; the GUI's unconditional dependency `eggsec-frontend-client` imports `tokio::net::UnixStream` without a `cfg(unix)` gate, so the GUI does not compile for Windows | **out of scope** — `ADR-0010` excludes Windows from the desktop GUI's supported platforms; the GUI's unconditional dependency `eggsec-frontend-client` imports `tokio::net::UnixStream` without a `cfg(unix)` gate, so the GUI does not compile for Windows |

### 2.1 Reading the matrix honestly

The matrix is mostly `not qualified`, and that is the accurate result of a
milestone whose job is to *measure* rather than to assert. Three things are
worth saying plainly:

1. **The Linux row is empty because this environment has no Linux host.** It
   is not empty because the GUI failed to build on Linux. The `gui-platform`
   CI job added in this milestone is the evidence path that will promote the
   Linux row; until that job has a green run and its artefact is recorded,
   the row stays `not qualified`.
2. **The macOS row is qualified only where there is an artefact.** `Build`
   has one. `Launch` has a partial one — the process starts and the webview
   comes up, but nobody saw the window, and the runbook's own criterion is a
   *visible* window, so the cell does not claim more than was observed.
3. **The behavioural columns are blocked on the daemon, not on the GUI
   shell.** The GUI cannot connect to anything but a daemon, and the
   daemon's engine-feature prerequisites (milestone 008) and obtainability
   recipe (milestone 009) are separate, still-open milestones. Until they
   close, the `Connect`, `Submit`, `Cancel`, and `Render` columns cannot be
   exercised at all — on any platform.

### 2.2 The bundler decision

`bundle.active` stays `false` in `crates/eggsec-gui/tauri.conf.json`. The
reasoning is in [`docs/GUI_BUILD.md`](GUI_BUILD.md) §3.3: the roadmap and
`ADR-0007` place installers, signing, and notarization in the separate
distribution workstream, and enabling the bundler would make a plain
`make check` emit installers as a side effect.

If a later decision re-opens it, this document must name the superseding
ADR on the same line as the `bundle.active` mention; the static guard
(check 168) rejects a `bundle.active: true` configuration that does not.

---

## 3. Provenance

This matrix is **regenerated**, not hand-maintained. The regeneration
procedure is:

1. Run the qualification procedure in
   [`docs/GUI_BUILD.md`](GUI_BUILD.md) §5 on the platform being qualified.
2. Write the run's evidence file under
   `docs/evidence/gui-qualification/`, following
   `docs/evidence/gui-qualification/TEMPLATE.md`.
3. For each column, set the cell to the status the evidence supports and
   point it at the evidence file (or name the root cause / blocking
   milestone).
4. Update the **Last regenerated** line above with the new date and the
   commit the matrix was regenerated at.
5. Re-run `python3 scripts/check_doc_references.py` and
   `bash scripts/check-architecture-guards.sh` (check 168 enforces the cell
   grammar).

The matrix is a claim about a specific commit. A matrix row that was
qualified against an older commit is stale, not merely historical — which is
why the regeneration header carries the commit SHA.

---

## 4. The evidence mechanism

### 4.1 Where evidence lives

```
docs/evidence/gui-qualification/
  TEMPLATE.md                        # the fields every run must fill
  macos-2026-10-23.md                # one file per qualification run
```

One file per run, named `<platform>-<YYYY-MM-DD>.md`. The filename is part of
the evidence reference, so a cell that cites an evidence file cites it by
name.

### 4.2 What an evidence file must contain

`docs/evidence/gui-qualification/TEMPLATE.md` fixes the fields. In short:
platform; the columns the run supports; the date; the operator or CI job;
the commit SHA; the tree state; the host OS version and architecture; the
target triple; the toolchain and cargo version; the requested and resolved
feature set; the prerequisite-probe result; the daemon version and build
features; the socket path and scope in force; the exact commands run with
their exit statuses; the observed output; the pass criterion and its
outcome; and any deviation from the runbook.

Two of those fields exist because they are the ones that get forgotten:

- **The tree state.** A build from a dirty tree is a diagnostic build, not a
  release build, and the difference has to be visible in the record rather
  than inferred later.
- **The deviations.** A deviation is recorded as a deviation. A runbook that
  was followed approximately is a different runbook, and the matrix cell has
  to say so.

### 4.3 Reproducibility

An evidence file is only useful if a second operator can re-run its commands
and expect the same observable outcome. That is why the template records the
*commands* rather than a description of them, and why the runbook in
`docs/GUI_BUILD.md` §5 gives each step an explicit pass criterion and names
the artefact the step produces.

### 4.4 CI as an evidence source

The `gui-platform` job in `.github/workflows/deep-checks.yml` runs on
`ubuntu-latest` and `macos-latest` and uploads the build metadata sidecar and
the binary's checksum as job artefacts. A matrix cell may therefore cite a CI
run URL instead of a checked-in evidence file. The rule is the same in both
cases: the reference must be reproducible by a documented command.

---

## 5. Why Windows is out of scope

The supported-platform decision is
[`ADR-0010`](../plans/adrs/ADR-0010-gui-supported-platform-scope.md):
**the desktop GUI is supported on Linux and macOS; Windows is out of scope.**
This milestone implements that decision; it does not make it.

The technical root cause is measured, not inferred. Cross-compiling the
GUI's unconditional dependency for Windows fails at the Unix-domain-socket
transport:

```
$ cargo check -p eggsec-frontend-client --target x86_64-pc-windows-msvc
error[E0432]: unresolved import `tokio::net::UnixStream`
   --> crates/eggsec-frontend-client/src/daemon.rs:9:5
error[E0433]: cannot find `unix` in `net`
   --> crates/eggsec-frontend-client/src/daemon.rs:56:31
error[E0599]: no method named `into_split` found for type `!`
   --> crates/eggsec-frontend-client/src/daemon.rs:85:46
error: could not compile `eggsec-frontend-client` (lib) due to 3 previous errors
```

The same three errors reproduce through the GUI crate, because
`eggsec-frontend-client` is an **unconditional** dependency of
`crates/eggsec-gui` (`crates/eggsec-gui/Cargo.toml:38`). The two
unconditional Unix-only import sites are:

- `crates/eggsec-frontend-client/src/daemon.rs:9` —
  `use tokio::net::UnixStream;`
- `crates/eggsec-frontend-client/src/daemon.rs:56` —
  `writer: Mutex<tokio::net::unix::OwnedWriteHalf>`

and the daemon side is equally unconditional:

- `crates/eggsec-daemon/src/server.rs:5` —
  `use tokio::net::UnixListener;`
- `crates/eggsec-daemon/src/server.rs:95` — `UnixListener::bind(socket_path)?`

### 5.1 The re-entry path

Adding a Windows leg to the CI matrix is gated on the daemon-transport
milestone `ADR-0010` names. A Windows daemon transport — named pipes or an
equivalent local IPC — is a **daemon-subsystem** change, not a GUI change: it
changes the *kind* of transport rather than adding a read path, so it
requires an ADR of its own. Milestone 007 names that path; it does not
implement it, and it deliberately does **not** add a Windows transport in
order to make a matrix cell green.

Until that ADR lands and the transport is implemented, the Windows row stays
`out of scope` in every column, and the CI matrix stays
`[ubuntu-latest, macos-latest]`.

---

## 6. What promotes a cell

| From | To | Requires |
|---|---|---|
| `not qualified` (Linux row) | `qualified` | A green `gui-platform` CI run on `ubuntu-latest` whose artefacts are recorded, plus an operator run of the behavioural steps if the cell is a behavioural one. |
| `not qualified` (behavioural columns) | `qualified` | Milestones 006, 008, and 009 closed, then the runbook steps in `docs/GUI_BUILD.md` §5 executed and recorded. |
| `partially qualified` | `qualified` | The missing part of the cell is observed and recorded. |
| `pending M6` | `qualified` | Milestone 006 closed and the export path exercised. |

The `Export` column is `pending M6` on both in-scope platforms and stays that
way until milestone 006 closes. The GUI frontend has no export affordance
today — there is no `export` reference anywhere in
`crates/eggsec-gui/frontend/app.js` — so the column is blocked on the
milestone that owns the feature, not on a platform.
