# Desktop GUI: prerequisites, build recipe, and qualification procedure

This document is the operator-facing half of the desktop GUI's platform work.
It says what a host must have before the GUI will build, what the build
recipe is, what the CI platform matrix does, and how to run the qualification
procedure that produces the evidence for
[`docs/GUI_PLATFORM_QUALIFICATION.md`](GUI_PLATFORM_QUALIFICATION.md).

- The qualification matrix and its evidence rules:
  [`docs/GUI_PLATFORM_QUALIFICATION.md`](GUI_PLATFORM_QUALIFICATION.md).
- The artefact handed to the distribution workstream:
  [`docs/GUI_DISTRIBUTION_HANDOFF.md`](GUI_DISTRIBUTION_HANDOFF.md).
- The shell-technology decision and the Linux platform floor:
  [`ADR-0009`](../plans/adrs/ADR-0009-gui-shell-technology-selection.md).
- The supported-platform decision:
  [`ADR-0010`](../plans/adrs/ADR-0010-gui-supported-platform-scope.md).

---

## 1. What this covers, and what it does not

The desktop GUI is a **daemon-only** frontend (`ADR-0007`). It links no
engine crate and reaches the engine only through the daemon's
Unix-domain-socket transport. That has one consequence that shapes this
whole document: **the GUI's build prerequisites and the daemon's build
prerequisites are two different lists, owned by two different milestones.**

This document covers the **Tauri shell's** prerequisites and the GUI
binary's build recipe. The **daemon's** engine-feature prerequisites are
milestone 008's profile and milestone 009's obtainability recipe; they are
linked, not restated, in §2.8, because two lists that both claim to describe
"the prerequisites" drift.

---

## 2. Prerequisites

### 2.1 The Rust toolchain

The GUI crate is inside the workspace and is covered by the workspace MSRV
contract (`rust-version = "1.89"`). The `tauri` dependency is pinned to the
2.11.x line precisely because the 2.12 line declares MSRV 1.90, above the
floor (`ADR-0009`, "Version pin"). `make check-msrv` covers the GUI crate.

### 2.2 Linux

The Linux build needs the platform webview SDK. The package names below are
the ones the Tauri v2 documentation states.

**Debian / Ubuntu**

```bash
sudo apt update
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file \
  libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
```

**Fedora**

```bash
sudo dnf check-update
sudo dnf install webkit2gtk4.1-devel openssl-devel curl wget file \
  libappindicator-gtk3-devel librsvg2-devel libxdo-devel
sudo dnf group install "Development Tools"   # the c-development group
```

**Arch**

```bash
sudo pacman -Syu
sudo pacman -S --needed webkit2gtk-4.1 base-devel curl wget file openssl \
  appmenu-gtk-module libappindicator-gtk3 librsvg xdotool
```

`libxdo-dev` (Debian/Ubuntu), `libxdo-devel` (Fedora) and `xdotool` (Arch)
are the same package under three names: they provide the XTest library the
Tauri webdriver tooling uses. They are needed to *drive* the shell, not to
compile it.

### 2.3 macOS

The webview ships with the OS as WKWebView, so no system packages are
needed. Install the compiler:

```bash
xcode-select --install     # desktop-only development
```

or install Xcode itself.

### 2.4 The platform floor

`webkit2gtk-4.1` implies **Ubuntu 22.04 / Debian 12 or newer**
(`ADR-0009`, "Linux system-dependency declaration"). That is the documented
platform floor.

Distributions **below the floor** are those whose webview development
packages predate the 4.1 series:

- Ubuntu 20.04 and earlier;
- Debian 11 (bullseye) and earlier;
- enterprise rebuilds frozen before the 22.04 / 12 generation, and any
  distribution carrying a `webkit2gtk-4.0` webview only.

On a host below the floor, an operator has two honest options:

1. **Move to a supported distribution** (Ubuntu 22.04+, Debian 12+, current
   Fedora, current Arch). This is the recommended option: it is the platform
   the CI matrix exercises.
2. **Fall back to the egui shell.** `ADR-0009` pre-sanctioned the egui /
   eframe fallback for exactly this situation, so that a distribution below
   the floor does not force a re-plan. The fallback is a change confined to
   `crates/eggsec-gui`.

Note that the workspace's own gate is **looser** than the build: `Makefile:21-30`
accepts either `webkit2gtk-4.1` **or** `webkit2gtk-4.0`, while the Tauri v2
build needs 4.1. A host with only 4.0 therefore passes the gate and then
fails to compile — which is one more reason the ungated `make check-gui`
exists.

### 2.5 What the workspace's CI list omits, and why that is safe there

The workspace CI installs a **subset** of Tauri's list:

```
pkg-config libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev \
  librsvg2-dev patchelf
```

(`.github/workflows/ci.yml:50-51` and `.github/workflows/deep-checks.yml:30`,
`:55-56`; `Makefile:17` carries the same names in `GUI_SHELL_PREREQ_HINT`.)

Tauri's names that this list **omits** are:

| Omitted | Why it is safe on the CI images | Why it is not safe on a bare distribution |
|---|---|---|
| `build-essential` | the GitHub-hosted Ubuntu image pre-installs the build toolchain | a bare Debian/Ubuntu install has no compiler |
| `curl`, `wget`, `file` | the images ship them | a minimal image may not |
| `libxdo-dev` | only needed to *drive* the shell, not to compile it | needed for webdriver-based UI testing |
| `libssl-dev` | the images ship it | needed by any closure that links the system OpenSSL |

So the omission is an **image** property, not a property of the dependency
set. On a bare distribution, install the full Tauri list.

`patchelf` is in the workspace's list but **not** in Tauri's list: it is
used by the Tauri bundler when it rewrites binaries, and the bundler is
inactive here (`bundle.active: false`,
`crates/eggsec-gui/tauri.conf.json:26-28`). It is therefore informational
for a plain `cargo build`.

### 2.6 Gate semantics: a green `make check` on Linux does not prove the shell compiles

`Makefile:154-182` includes the GUI shell profile through a gate
(`Makefile:21-30`) that behaves like this:

- **Non-Linux** (macOS, Windows): the command **always runs**. The webview
  ships with the OS, so there is nothing to gate on.
- **Linux**, webview SDK present: the command **runs**.
- **Linux**, webview SDK absent: the gate **skips**, prints
  `SKIP: desktop GUI shell profile (prerequisite absent: …)`, and
  **exits 0**.

The skip is the right shape for a fast CI loop — an absent prerequisite
should not turn every unrelated developer's run red. But it has a direct
consequence for qualification: **a green `make check` on a Linux host
without the webview SDK proves nothing about the shell.** The GUI shell
profile was skipped, not compiled.

The ungated check is:

```bash
make check-gui
```

(`Makefile:187-190`) — it always runs `cargo check -p eggsec-gui --features
shell` and `cargo clippy` on it, and reports cargo's own error message
rather than a silent skip.

Two more gaps in `make check` that matter for the matrix:

- `make check` does **not** run `cargo test -p eggsec-gui`. The GUI's 34
  tests are only run by hand or by the `gui-platform` CI job added in this
  milestone.
- The GUI's non-shell profile is only covered by
  `cargo check --workspace --no-default-features`, which is a compile check,
  not a test run.

### 2.7 The prerequisite probe

```bash
bash scripts/check_gui_prereqs.sh
```

The probe reports **BUILDABLE** or **NOT BUILDABLE**. It never says
"skipped": a skip says "we did not look", and a qualification cell needs a
cell that says "we looked, and the answer is no". When the answer is no, the
probe names each missing prerequisite.

The probe installs nothing and runs no cargo build. Installing to find out
is not a probe, and a full release build takes minutes; the question "can
this host build the shell?" is answered by the presence of the SDKs the shell
links against.

Exit status: `0` buildable, `1` not buildable, `2` out of scope (the
platform is excluded by `ADR-0010`).

### 2.8 The daemon's prerequisites are a different list

The list above is the **Tauri shell's** list. The daemon needs an engine
built with the engine features the operator wants, and its prerequisites are
milestone 008's profile and milestone 009's obtainability recipe:

- [`docs/DAEMON_FEATURE_PROFILE.md`](DAEMON_FEATURE_PROFILE.md) — the
  daemon's engine-feature profile.
- [`docs/DAEMON.md`](DAEMON.md) — the daemon's obtainability recipe.

Two measured facts belong in that link, because the repository's prose was
stale on both and milestone 008 corrects the source of truth:

- **`packet-inspection` needs no libpcap.** The feature is pure Rust:
  `packet-inspection = ["pnet", "pnet_packet", "libc"]`
  (`crates/eggsec/Cargo.toml:326`). There is no `libpcap-dev` in its
  closure.
- **`nse` vendors OpenSSL.** The feature pulls the published `eggsec-nse`
  crate (`crates/eggsec/Cargo.toml:329`), whose closure is
  `eggsec-nse` → `openssl` → `openssl-sys` → `openssl-src`
  (`Cargo.lock`, `openssl-src` 300.6.1+3.6.3). OpenSSL is therefore
  **compiled from source**, so the `nse` feature needs a C toolchain, not a
  system `libssl-dev`.

Neither fact is restated as a daemon prerequisite here, because the daemon's
list is not this document's to maintain.

---

## 3. The build recipe

### 3.1 The pinned invocation

```bash
cargo build --release -p eggsec-gui --features shell --locked
```

`--locked` is load-bearing: it refuses a lockfile that would change, so the
lockfile hash recorded in the build metadata describes the graph that was
actually built. `--features shell` selects the Tauri shell profile; without
it the crate builds without the webview and produces a binary with no
window.

### 3.2 The build metadata sidecar

`scripts/build_gui_release.sh` runs the pinned invocation and emits, beside
the binary:

| Field | Why it is recorded |
|---|---|
| `commit_sha` | the source the binary was built from |
| `tree_state` / `dirty` / `allow_dirty` | whether the binary came from a pinned commit or a dirty tree |
| `rustc_version` / `rustc_version_verbose` | the compiler, including its LLVM version |
| `cargo_version` | the Cargo that resolved the graph |
| `target_triple` | the platform the binary is for |
| `cargo_features_requested` / `cargo_features_resolved` | the feature set, read from Cargo's own resolution rather than transcribed |
| `cargo_lock_sha256` | the lockfile the build used |
| `binary_sha256` | the artefact's identity |
| `binary_size_bytes` | the artefact's size |
| `credential_env_vars_present` | the **names** (never the values) of credential-shaped environment variables that were set during the build |

The script refuses to run on a dirty tree unless `--allow-dirty` is passed,
and records the tree state either way. It then asserts four things about
the artefact it just produced:

1. No installer artefact (`.deb`, `.rpm`, `.AppImage`, `.dmg`, `.app`,
   `.msi`, `.exe`) was emitted.
2. The metadata sidecar contains no credential-shaped material.
3. The built binary contains no credential-shaped material.
4. The CSP compiled into the binary is the checked-in one verbatim and
   admits no remote origin beyond `http://ipc.localhost`.

### 3.3 What the recipe deliberately does not do

`bundle.active` stays **`false`** (`crates/eggsec-gui/tauri.conf.json:26-28`).
The reasoning is the milestone's own boundary:

- The roadmap and `ADR-0007` place installers, signing, and notarization in
  a **separate distribution workstream**. Enabling the bundler would make a
  plain `make check` emit installers as a side effect, which is exactly the
  kind of surprise a build gate must not produce.
- Milestone 007 produces the **binary** and the **reproducible build
  recipe**. The distribution workstream inherits the binary and decides
  whether to enable the bundler or to ship a raw artefact.

So the recipe does not sign, does not notarize, does not bundle, does not
publish, and has no npm step.

### 3.4 Running the recipe

```bash
# default output directory: dist/ (git-ignored)
bash scripts/build_gui_release.sh

# explicit output directory
bash scripts/build_gui_release.sh --out dist/

# cross-target build
bash scripts/build_gui_release.sh --target x86_64-unknown-linux-gnu

# diagnostic build from a dirty tree (recorded as dirty in the sidecar)
bash scripts/build_gui_release.sh --allow-dirty
```

The Makefile target `make gui-release` wraps this for CI.

---

## 4. The CI platform-matrix job

The `gui-platform` job in `.github/workflows/deep-checks.yml` runs on
`ubuntu-latest` and `macos-latest` with `fail-fast: false`, so one leg's
failure does not hide the other's result.

**Windows is excluded from the matrix by decision** (`ADR-0010`), not by
oversight. The GUI's unconditional dependency `eggsec-frontend-client`
imports `tokio::net::UnixStream` without a `cfg(unix)` gate
(`crates/eggsec-frontend-client/src/daemon.rs:9`, `:56`), so the GUI does not
compile for Windows at all; a Windows leg would be permanently red rather
than informative. Adding it is gated on the daemon-transport milestone
`ADR-0010` names — a Windows daemon transport (named pipes or equivalent
local IPC) is a daemon-subsystem change that requires its own ADR.

Each leg runs:

```bash
make check-gui                                              # the UNGATED check
cargo test -p eggsec-gui --features shell --lib
cargo test -p eggsec-gui --no-default-features --lib
make gui-release
```

- `make check-gui`, not the gated `make check` line: on Linux the gated line
  skips when the webview SDK is absent, and this job's whole point is to
  prove that the SDK is present *and* the shell compiles.
- **Both** test profiles: the `shell` feature gates `commands.rs` and
  `shell.rs`, so the non-shell path has to be exercised too. The GUI's
  tests are headless-safe — no socket, no window, no subprocess — so they
  run unattended with no display.
- `make gui-release` produces the binary and its metadata sidecar, which the
  job uploads as artefacts. That is what lets a matrix cell cite a CI run
  URL instead of an assertion.

---

## 5. The qualification procedure

This is the runbook. One step per behaviour column of the matrix, in the
order the columns appear.

### 5.0 Preconditions

**The daemon.** The GUI is daemon-only: it cannot connect to anything else,
and there is no embedded-runtime path in the GUI crate. The daemon is
therefore started **separately**, and the GUI is pointed at it.

```bash
eggsec-daemon --full-executor --socket-path /tmp/eggsec-daemon.sock
```

`--full-executor` only has an effect if the daemon binary was **built with**
the `service` cargo feature (or its deprecated `full-executor` alias); a
binary built without it accepts the flag and falls back to a no-op executor
with a warning (`crates/eggsec-daemon/src/main.rs:40-46`). The daemon's
obtainability recipe is milestone 009's
([`docs/DAEMON.md`](DAEMON.md)); its engine-feature profile is milestone
008's ([`docs/DAEMON_FEATURE_PROFILE.md`](DAEMON_FEATURE_PROFILE.md)).

**The daemon's lifecycle is milestone 006's.** The GUI does not start,
supervise, or stop the daemon. Until M6 closes, the operator starts the
daemon by hand as above and points the GUI at it.

**The socket path.** The GUI takes `--socket <path>`
(`crates/eggsec-gui/src/config.rs:47`). Its default is
`/tmp/eggsec-daemon.sock` (`crates/eggsec-gui/src/config.rs:17`), which is
the same default the daemon binary, the daemon's config, the CLI, and the
webview each spell out again — **six production sites** in total. Use an
explicit path in qualification runs so that the record says which socket
was used.

**The scope.** The GUI carries **no scope of its own** and has no scope
field. The daemon opens the GUI's session with the scope it constructs
(`crates/eggsec-daemon/src/server.rs:476-483` — `RuntimeSurface::DesktopGuiManual`
with an implicit `gui-default` scope). The scope in force for a run is
therefore the daemon's, and it is recorded in the evidence file as the
daemon's configuration, not as a GUI setting.

### 5.1 Step 1 — Build

| | |
|---|---|
| **Precondition** | The prerequisites of §2 are present; `bash scripts/check_gui_prereqs.sh` reports BUILDABLE. |
| **Action** | `bash scripts/build_gui_release.sh --out dist/` (or `make gui-release`). |
| **Pass criterion** | The command exits 0; `dist/eggsec-gui` and `dist/eggsec-gui.build.json` exist; the sidecar's `binary_sha256` equals the SHA-256 of the emitted binary; the output directory contains no installer artefact. |
| **Evidence artefact** | `dist/eggsec-gui.build.json` and `dist/SHA256SUMS`, copied into `docs/evidence/gui-qualification/<platform>-<date>.md`. |

### 5.2 Step 2 — Launch

| | |
|---|---|
| **Precondition** | The built binary exists; the host has an attached desktop session (a macOS window-server session; a Wayland or X11 session on Linux). |
| **Action** | `dist/eggsec-gui --socket <path>` |
| **Pass criterion** | A window titled **Eggsec** appears (the title comes from `crates/eggsec-gui/tauri.conf.json`); the process emits no panic; closing the window ends the process cleanly. |
| **Evidence artefact** | A screenshot of the window, the process's exit status, and the captured stdout/stderr. |

A process that starts and registers with the window server but never paints
a visible window does **not** pass this step. The distinction is recorded in
the matrix as a partial qualification, not smoothed into a pass.

### 5.3 Step 3 — Connect

| | |
|---|---|
| **Precondition** | The daemon is running on `<path>`; the GUI is launched. |
| **Action** | Type `<path>` into the socket field and press **Connect**. This invokes the `connect_daemon` IPC command (`crates/eggsec-gui/frontend/app.js:1284-1288`). |
| **Pass criterion** | The connection badge (`#connection-badge`) changes from "Not connected" to "Connected"; the operation list in the sidebar populates from the daemon's catalogue; the "no-op executor" badge (`#no-op-badge`) stays hidden, because a daemon built without the engine features reports a no-op executor. |
| **Evidence artefact** | A screenshot of the connected window, plus the daemon's log line showing the client's session. |

### 5.4 Step 4 — Submit

| | |
|---|---|
| **Precondition** | Connected; an operation kind selected in the sidebar; the form populated with values that pass the three-layer validation (required fields, per-field schema, operation-level constraints). |
| **Action** | Press **Run** (`crates/eggsec-gui/frontend/app.js:604-609`, submit handler at `:626`). The frontend invokes `submit_task` with the variant and the values. |
| **Pass criterion** | A task appears in the Tasks panel with a task id and a non-terminal status; the daemon's audit trail records the dispatch. A rejection is a legitimate observation and is recorded as such — the daemon enforces, and the GUI's only job is to surface the refusal. |
| **Evidence artefact** | A screenshot of the task card with its task id, and the daemon's audit record. |

### 5.5 Step 5 — Cancel

| | |
|---|---|
| **Precondition** | A task is in a non-terminal state. |
| **Action** | Press the task's **Cancel** button (`crates/eggsec-gui/frontend/app.js:896-904`, `cancel_task`) or the panel's **Cancel active** button (`crates/eggsec-gui/frontend/app.js:1128`, `cancel_active`). |
| **Pass criterion** | The **terminal state observed in the event stream** is the criterion — not the click, and not the cancel request's own acknowledgement. |
| **Evidence artefact** | A screenshot of the task's terminal status, and the daemon's audit record. |

**Cancellation is best-effort, and a cancel race is not an error.** A task
that completes between the operator's click and the daemon's processing is a
legitimate outcome: the cancellation lost a race it was never guaranteed to
win. The event stream is authoritative for terminal state
(`crates/eggsec-gui/src/commands.rs:432`), so the runbook records the
terminal state that was **observed**, and a `completed` terminal state after
a cancel click is recorded as a completed task, not as a failed
cancellation.

### 5.6 Step 6 — Render

| | |
|---|---|
| **Precondition** | A task reached `completed` and carries a result payload. |
| **Action** | Observe the task card in the Tasks panel. |
| **Pass criterion** | The card renders the **real result** — the payload's fields, with their values — and not an empty dashboard. If the task kind has **no** schema, the generic-kind degradation path renders the payload generically rather than rendering nothing; that degraded render is a pass for the degradation path, and is recorded as such. |
| **Evidence artefact** | A screenshot of the rendered result, including the generic-kind case when it was exercised. |

### 5.7 Step 7 — Export

| | |
|---|---|
| **Precondition** | Milestone 006 closed. |
| **Action** | *Not yet defined.* The GUI frontend has no export affordance today — there is no `export` reference anywhere in `crates/eggsec-gui/frontend/app.js`. |
| **Pass criterion** | *Not yet defined.* |
| **Evidence artefact** | *Not yet defined.* |

The procedure is written and reviewed, but the **matrix cell stays `pending
M6`** until M6 closes. A procedure without a feature behind it is not
evidence of anything.

### 5.8 The repeatability contract

The same steps, on the same platform, with the same daemon, produce the same
observable outcomes.

A deviation is recorded as a **cell gap** in the evidence file — with what
was expected, what was observed, and why — and the matrix cell is updated to
match. It is never smoothed over, and it is never left as a passing cell
with a footnote. The evidence template has a **Deviations and notes**
section precisely so that this is a mechanical step rather than a judgement
call.

---

## 6. Evidence

Each run of §5 produces one file under `docs/evidence/gui-qualification/`,
named `<platform>-<YYYY-MM-DD>.md`, following
`docs/evidence/gui-qualification/TEMPLATE.md`. The matrix cell points at that
file. The rules for what an evidence file must contain, and how a matrix cell
may cite a CI run instead, are in
[`docs/GUI_PLATFORM_QUALIFICATION.md`](GUI_PLATFORM_QUALIFICATION.md) §4.
