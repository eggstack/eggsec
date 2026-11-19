# Desktop GUI Frontend Milestone 007 — Packaging and Platform Qualification

Status: ready for handoff

Repository baseline: branch `feat/auto-20261022-76692679`, commit `e1916069` ("docs(plans): register the desktop GUI milestones 006 and 007"), on top of `34df13d0` ("feat(gui): add the desktop GUI frontend foundation (milestones 001-005)"). Milestones 001–005 are implemented with closure records under `plans/closure/desktop-gui-frontend/`. Milestone 006 is **planned but not implemented**; its plan lives at `plans/implementation/desktop-gui-frontend/006-full-operation-coverage-and-report-export.md`. Milestones 008 and 009 are **planned but not implemented**; they gate this milestone's daemon prerequisites.

Source roadmap:

- `plans/subsystems/desktop-gui-frontend-roadmap.md#milestone-7--packaging-and-platform-qualification`

Long-term requirements:

- `plans/000-long-term-specification.md#9-verification-contract`
- `plans/000-long-term-specification.md#10-frontend-rules`
- `plans/000-long-term-specification.md#8-runtime-and-dependency-baselines`
- `plans/001-terminology-and-domain-model.md#6-frontend-and-session-terms`

Applicable ADRs:

- `plans/adrs/ADR-0007-desktop-gui-frontend-boundary.md`
- `plans/adrs/ADR-0008-operation-parameter-schema-publication.md`
- `plans/adrs/ADR-0009-gui-shell-technology-selection.md`
- `plans/adrs/ADR-0010-gui-supported-platform-scope.md`

Primary class: operational

> **Recorded deviation from the template's enum.** `plans/implementation/README.md:39`
> lists `Primary class: invariant | capability | infrastructure | polish`, and
> `operational` is not among them. The value is kept because the subsystem
> roadmap declares this milestone `Class: operational`
> (`plans/subsystems/desktop-gui-frontend-roadmap.md:590`) and because the
> milestone's deliverable is qualification evidence and a handoff artifact
> rather than a production capability. The repository already carries
> free-form values in this field (`security + infrastructure`,
> `polish (corrective)`), so the deviation is visible rather than silent — and
> the template's enum is the only thing that ought to change, not this plan.

---

## 1. Objective

Qualify the desktop GUI on the platform matrix the project actually supports, and hand a reproducible, evidence-backed binary build to the separately-managed distribution workstream — without owning signing, notarization, installers, or update channels.

One bounded outcome, in three observable parts:

1. A **platform qualification matrix** exists as a checked-in artifact with one row per platform — **two qualified rows (Linux, macOS)** and **one explicitly out-of-scope row (Windows)** — and one column per required behaviour (build, launch, connect, submit, cancel, render, export). Every cell carries either a reproducible evidence reference or a **named, justified gap**.
2. The **Linux system prerequisites** are documented concretely, with the per-distribution package names and the supported Linux floor.
3. A **handoff artifact** describes the qualified binary (platform, feature set, toolchain, commit, runtime prerequisites, known limitations) and states exactly what the distribution workstream inherits and what it still must decide.

The milestone's deliverable boundary is fixed by the roadmap: "a qualified build per supported platform, documented system prerequisites, and a handoff artifact for the distribution workstream. **This milestone does not own signing, notarization, or installers.**" (`plans/subsystems/desktop-gui-frontend-roadmap.md:598-600`).

---

## 2. Why this milestone is ready

### 2.1 Closed hard dependencies

| Dependency | State at `34df13d0` | Evidence |
|---|---|---|
| M1 — operation parameter schema publication | **closed 2026-10-23** | `plans/closure/desktop-gui-frontend/001-operation-parameter-schema-publication-closure.md`; roadmap status table `plans/subsystems/desktop-gui-frontend-roadmap.md:897` |
| M2 — frontend-client contract extraction | **closed 2026-10-23** | `plans/closure/desktop-gui-frontend/002-frontend-client-contract-extraction-closure.md`; roadmap `:698` |
| M3 — desktop GUI surface parity | **closed 2026-10-23** | `plans/closure/desktop-gui-frontend/003-desktop-gui-surface-parity-closure.md`; roadmap `:699` |
| M4 — GUI shell skeleton | **implemented 2026-10-23** | `plans/closure/desktop-gui-frontend/004-gui-shell-skeleton-closure.md`; roadmap `:700` |
| M5 — schema-driven forms and task dashboard | **implemented 2026-10-23** | `plans/closure/desktop-gui-frontend/005-schema-driven-forms-and-task-dashboard-closure.md`; roadmap `:701` |

### 2.2 Stable interface dependencies (verified at the baseline)

- The GUI crate compiles in both build profiles: `--no-default-features` (no webview SDK needed) and `--features shell` (the default; needs the platform webview SDK). The feature split is declared at `crates/eggsec-gui/Cargo.toml:24-35`; the build script is gated on `CARGO_FEATURE_SHELL` at `crates/eggsec-gui/build.rs:8-10`.
- The GUI's **IPC surface is exactly eight commands**, each declaring `rename_all = "snake_case"` — enforced by an in-crate test at `crates/eggsec-gui/src/commands.rs:641-655`. The surface is registered at `crates/eggsec-gui/src/shell.rs:22-31`.
- The GUI's 34 tests are **unit-level and headless-safe**: no `UnixStream`, `UnixListener`, `TcpStream`, `tauri::test`, `WebviewWindow`, or `std::process::Command` appears anywhere under `crates/eggsec-gui/src/` (verified by search over the crate's source tree). The `connection.rs` tests (`crates/eggsec-gui/src/connection.rs:179-215`) construct error values and strings; they open no socket.
- The frontend is **buildless with zero npm dependencies**: `crates/eggsec-gui/frontend/` holds `index.html` (72 lines), `styles.css` (871), and `app.js` (1307); there is no `package.json`, no lockfile, and no bundler. `tauri.conf.json:10` sets `withGlobalTauri: true` precisely so the webview can call `window.__TAURI__.core.invoke` without a module build step (`architecture/desktop_gui.md:136-141`).
- The daemon transport contract is stable and additive-only (`plans/subsystems/desktop-gui-frontend-roadmap.md:747-765`).

### 2.3 The remaining hard dependencies — stated plainly

**M6 is the roadmap's hard dependency for M7** (`plans/subsystems/desktop-gui-frontend-roadmap.md:595-597`; the status table lists M7's blockers as "M6; M8, M9; distribution policy" at `:818`). M6 is **not implemented at this baseline**. **M8 and M9 are also hard dependencies**: a qualification run needs a daemon that can be **built** with a real feature profile (M8) and **obtained** and started without lying about its capabilities (M9).

Consequences for this plan:

- The **report-export** column of the qualification matrix cannot be qualified until M6 lands, because M6 is what introduces report export (`plans/subsystems/desktop-gui-frontend-roadmap.md:533`, milestone title "Full operation coverage and report export"). M007 therefore treats the export cell as a **dependency-gated cell**: the matrix defines it, the procedure defines it, and the cell stays marked *pending M6* until M6 closes.
- The **connect** and **submit** columns depend on M8 and M9 for the same reason: a qualification run needs a daemon that can actually run the operation it is submitting. Until M8 and M9 close, those cells stay dependency-gated rather than blank.
- This plan is authored to be **handed off and executed once M6, M8, and M9 close**. It is not a claim that M7's work has started or that its dependencies are satisfied.

**The supported-platform matrix is no longer an open question.** `ADR-0010` decides it: the desktop GUI is supported on **Linux (x86_64 glibc) and macOS (x86_64 and arm64)**, and **Windows is out of scope**, because the daemon's transport is a Unix domain socket and the GUI does not compile for Windows. That decision removes the "distribution policy" dependency's *matrix* component: the distribution workstream still owns signing, notarization, installers, and update channels, and §13 still makes its sign-off on the **distribution** an acceptance precondition — but it no longer decides **which platforms the GUI is offered on**. M007's job is now to produce the **evidence** for the two supported platforms and the **named out-of-scope row**, not to recommend a matrix.

---

## 3. Current implementation evidence

### 3.1 The GUI crate as it stands

| Item | Measured fact | Citation |
|---|---|---|
| Package identity | `eggsec-gui`, `version 0.1.0`, `edition 2021`, `license MIT`, `rust-version 1.89` (workspace-inherited) | `crates/eggsec-gui/Cargo.toml:1-14` |
| Binary target | `[[bin]] name = "eggsec-gui"`, `path = "src/main.rs"` | `crates/eggsec-gui/Cargo.toml:20-22` |
| Library target | `[lib] name = "eggsec_gui"`, `path = "src/lib.rs"` | `crates/eggsec-gui/Cargo.toml:16-18` |
| Default features | `default = ["shell"]`; `shell` pulls `tauri`, `tauri-macros`, `tauri-runtime`, `tauri-runtime-wry`, `tauri-utils` | `crates/eggsec-gui/Cargo.toml:24-35` |
| Direct dependencies | `eggsec-frontend-client`, `eggsec-runtime`, `eggsec-daemon-protocol`, `eggsec-ui-model`, `serde`, `serde_json`, `tokio`, `tracing` | `crates/eggsec-gui/Cargo.toml:37-49` |
| Tokio features | `["rt", "sync", "time", "macros", "net"]` — `net` for `UnixStream`; no `rt-multi-thread`, no `test-util` | `crates/eggsec-gui/Cargo.toml:46-48` |
| Tauri features | `wry`, `common-controls-v6`, `x11`, `dbus`, `dynamic-acl`; `rustls-tls` and `native-tls` deliberately **not** enabled | `crates/eggsec-gui/Cargo.toml:51-62` |
| Workspace Tauri pins | `tauri = "=2.11.6"`, `tauri-build = "=2.6.3"`, `tauri-runtime = "=2.11.3"`, `tauri-runtime-wry = "=2.11.4"`, `tauri-macros = "=2.6.3"`, `tauri-utils = "=2.9.3"` — exact (`=`), not carets | `Cargo.toml:93-98` |
| Window | one window, label `main`, title `Eggsec`, 1200×800, min 800×600, resizable | `crates/eggsec-gui/tauri.conf.json:11-21` |
| CSP | `default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self' data:; connect-src 'self' ipc: http://ipc.localhost; object-src 'none'; frame-src 'none'; base-uri 'none'` | `crates/eggsec-gui/tauri.conf.json:22-24` |
| Capabilities | `core:default` only — no `http`, `shell`, `fs`, `process`, or `updater` plugin | `crates/eggsec-gui/capabilities/default.json` |
| **Bundler** | **`bundle.active: false`** — no bundling is configured today | `crates/eggsec-gui/tauri.conf.json:26-28` |
| Icon | `crates/eggsec-gui/icons/icon.png` — 512×512 RGBA PNG, 4 379 bytes | measured on the file |
| Source size | `commands.rs` 672, `config.rs` 145, `connection.rs` 216, `form.rs` 824, `lib.rs` 70, `main.rs` 13, `shell.rs` 34, `tasks.rs` 865 lines | `wc -l crates/eggsec-gui/src/*.rs` |
| Tests | 34: `commands.rs` 4, `config.rs` 6, `connection.rs` 4, `form.rs` 11, `tasks.rs` 9 | counted `#[test]` / `#[tokio::test]` attributes |
| Default socket | `DEFAULT_SOCKET_PATH = "/tmp/eggsec-daemon.sock"`; flags `--socket`, `--socket=`, `--help`, `-h` | `crates/eggsec-gui/src/config.rs:17`, `:21`, `:47`, `:55` |

### 3.2 The `bundle.active: false` fact is the pivot of this milestone

`crates/eggsec-gui/tauri.conf.json:26-28` sets `bundle.active: false`, and `architecture/desktop_gui.md:74-78` states the consequence plainly: "the GUI is run from the build tree, not installed as a bundle."

Tauri's bundler, when enabled, produces `.deb` / `.rpm` / `.AppImage` on Linux, `.dmg` / `.app` on macOS, and `.msi` / `.exe` installers on Windows. Every one of those is an **installer**, which the roadmap places outside this milestone (`roadmap:600`, `:567-568`) and outside this workstream (`plans/adrs/ADR-0007-desktop-gui-frontend-boundary.md:88-91`, `:249-251`).

### 3.3 The daemon transport — a Unix domain socket, with no Windows fallback

This is the first-class platform fact for M7, and it is **not** a matter of opinion.

- `crates/eggsec-frontend-client/src/daemon.rs:9` — `use tokio::net::UnixStream;` — unconditional, no `#[cfg(unix)]`.
- `crates/eggsec-frontend-client/src/daemon.rs:56` — `writer: Mutex<tokio::net::unix::OwnedWriteHalf>` — unconditional.
- `crates/eggsec-frontend-client/src/daemon.rs:84-85` — `UnixStream::connect(socket_path).await?` then `stream.into_split()`.
- `crates/eggsec-daemon/src/server.rs:1` — `use std::os::unix::fs::PermissionsExt;` — unconditional.
- `crates/eggsec-daemon/src/server.rs:5` — `use tokio::net::UnixListener;` — unconditional.
- `crates/eggsec-daemon/src/server.rs:25` — `tokio::net::unix::OwnedReadHalf` — unconditional.
- `crates/eggsec-daemon/src/server.rs:95` — `UnixListener::bind(socket_path)?`.
- The **only** Windows-aware code in the daemon is the shutdown-signal handler: `#[cfg(unix)]` at `crates/eggsec-daemon/src/main.rs:206` and `#[cfg(not(unix))]` at `:229`. The socket itself is not gated.

`tokio::net::UnixListener` and `tokio::net::UnixStream` are compiled only on Unix. The resolved tokio is 1.53.1 (`Cargo.lock`), and in that release `src/net/mod.rs:54-59` wraps the Unix types in `cfg_net_unix!`, which expands to `#[cfg(all(unix, feature = "net"))]` (`src/macros/cfg.rs:374-378`).

**Empirical confirmation** (cross-compilation check against the installed `x86_64-pc-windows-msvc` target, run in an isolated `CARGO_TARGET_DIR` so the shared workspace target directory was not touched):

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

The same three errors reproduce through the GUI crate, because `eggsec-frontend-client` is an **unconditional** dependency of `eggsec-gui` (`crates/eggsec-gui/Cargo.toml:38`):

```
$ cargo check -p eggsec-gui --no-default-features --target x86_64-pc-windows-msvc
error[E0432]: unresolved import `tokio::net::UnixStream`
   --> crates/eggsec-frontend-client/src/daemon.rs:9:5
... (identical errors)
```

**Therefore: at the baseline, the desktop GUI does not compile on Windows — with or without the `shell` feature.** The blocking dependency is the shared frontend-client contract crate, which is owned by the frontend-client subsystem, not by the GUI.

**This is now a decision, not an open gap.** `ADR-0010` fixes the desktop GUI's supported platforms at **Linux and macOS** and records **Windows as out of scope**, precisely because of the finding above. The exclusion is a deliberate scope decision with a named root cause and a named re-entry path (a daemon-subsystem transport milestone with its own ADR) — not a defect awaiting a workaround, and not something M007 may resolve by adding a transport. M007's job is to **record** the out-of-scope row with its root cause, and to qualify the two supported platforms.

A separate `cargo check -p eggsec-daemon --target x86_64-pc-windows-msvc` run stopped earlier, in the `libsqlite3-sys v0.30.1` build script (a cross-C-toolchain limitation of this machine, not a property of the code). The daemon's own Windows incompatibility is nevertheless established by the unconditional `std::os::unix::fs::PermissionsExt` and `tokio::net::UnixListener` imports cited above.

### 3.4 What CI actually builds today

| Job | Workflow | Runner | What it runs | GUI shell built? |
|---|---|---|---|---|
| `rust` | `ci.yml:32-53` | `ubuntu-latest` | installs `pkg-config libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev patchelf` (`ci.yml:50-51`), then `make check` | **yes**, via the gated line `Makefile:160` — but gated, so a missing prerequisite silently skips it |
| `dependency-policy` | `ci.yml:55-69` | `ubuntu-latest` | `make check-deps` | n/a |
| `msrv` | `deep-checks.yml:40-58` | `ubuntu-latest` | installs the same webview packages (`deep-checks.yml:55-56`), then `make check-msrv` | **yes**, via the gated line `Makefile:137` |
| `broad` / `check-full` | `deep-checks.yml:29-38` | `ubuntu-latest` | `make check-full`, `make check-features-individual` | gated |
| `portability` | `deep-checks.yml:60-76` | matrix `os: [macos-latest, windows-latest]` (`deep-checks.yml:66`) | **`cargo check -p eggsec` only** (`deep-checks.yml:76`) | **no** — the GUI is never compiled on macOS or Windows in CI today |
| `platform-integration` | `deep-checks.yml:82-104` | `ubuntu-latest` | `eggsec doctor`, `bash scripts/check_platform.sh`, live probes with `\|\| true` (`:101-102`) | no |

`deep-checks.yml`'s `portability` job is the existing precedent for a platform matrix, and it is the natural home for M7's new job. `platform-integration` is scheduled/manual only and its live legs use `|| true`, so a missing prerequisite can never turn that job red (`docs/PLATFORM.md:85-89`).

### 3.5 The Makefile's GUI shell gate — exact behaviour

`Makefile:6-30`:

```make
# The gate follows the discipline of scripts/check-features-individual.sh:
# an ABSENT prerequisite is a SKIP with the reason named, while a PRESENT
# prerequisite whose build breaks is a hard FAIL -- the build error is never
# swallowed. `make check-gui` is the ungated escape hatch: it always runs and
# reports cargo's own error.
GUI_SHELL_PREREQ_HINT = Linux: libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev patchelf pkg-config

define GUI_SHELL_GATE
@if [ "$$(uname -s)" != "Linux" ]; then \
	$(1); \
elif command -v pkg-config >/dev/null 2>&1 \
	&& { pkg-config --exists webkit2gtk-4.1 2>/dev/null || pkg-config --exists webkit2gtk-4.0 2>/dev/null; }; then \
	$(1); \
else \
	echo "SKIP: desktop GUI shell profile (prerequisite absent: $(GUI_SHELL_PREREQ_HINT))."; \
fi
endef
```

Read literally, the gate has three branches:

1. **Not Linux** (macOS, Windows) → the command **always runs**, unconditionally. There is no probe and no skip path on macOS or Windows.
2. **Linux with `pkg-config` and `webkit2gtk-4.1` or `webkit2gtk-4.0`** → the command runs.
3. **Linux without the webview** → the gate prints `SKIP: desktop GUI shell profile (prerequisite absent: …)` and **exits 0**. The command does not run.

Consequence for M7: on Linux the shell build can be **silently skipped** in any `make check` run, and a green `make check` on a Linux host does **not** prove the GUI shell compiles. `make check-gui` (`Makefile:187-190`) is the ungated escape hatch:

```make
check-gui:
	@echo "Desktop GUI shell profile (prerequisite: $(GUI_SHELL_PREREQ_HINT))."
	cargo check -p eggsec-gui --features shell
	cargo clippy -p eggsec-gui --features shell -- -D warnings
```

`docs/VERIFICATION.md:112-132` states the same discipline in prose: "A SKIP is not a pass."

### 3.6 The GUI's own test suite is not in `make check`

`make check` (`Makefile:154-182`) runs the engine, `eggsec-output`, `eggsec-report-model`, `eggsec-policy`, the four corpus crates, `eggsec-transport-eggfetch`, and `eggsec-tui` test suites, plus the architecture guards. It **does not** run `cargo test -p eggsec-gui`. The 34 GUI tests are therefore only run by hand or by a milestone closure record.

### 3.7 The platform-sensitive domain matrix (context, not the GUI's matrix)

`docs/PLATFORM.md:6-14` names the source of truth: `crates/eggsec/src/platform/` — `PlatformReport::collect()` (`prereqs.rs`); `eggsec doctor` prints the same matrix; live tests call `skip_reason_for(domain)`. `report_for()` knows exactly **six** domains: `mobile-dynamic`, `packet-inspection`, `wireless`, `wireless-advanced`, `stress-testing`, `nse` (`docs/PLATFORM.md:11-12`, table at `:16-23`). **The desktop GUI is not one of those six domains** — the GUI's platform requirements are a *build* requirement, not a runtime probing domain, and M007 must not add a seventh domain to `crates/eggsec/src/platform/` without a matching matrix row.

`scripts/check_platform.sh` is the **hermetic** fixture layer (no root, no hardware) and is entirely engine-side. `scripts/release-check.sh` validates artifacts **without publishing** and currently packages only the Python wheel — it does not build or package the GUI.

### 3.8 Known gaps carried into this milestone

| Gap | Evidence | Handling |
|---|---|---|
| The GUI's resident footprint was never measured | `plans/subsystems/desktop-gui-frontend-roadmap.md:803-805` requires it "at the shell milestone"; no footprint measurement appears in `plans/closure/desktop-gui-frontend/004-gui-shell-skeleton-closure.md` | **Carry-forward.** M007 is the measurement milestone; the measurement is discharged here and recorded in the handoff artifact. |
| The GUI does not manage the daemon's lifecycle | `plans/closure/desktop-gui-frontend/004-gui-shell-skeleton-closure.md:561` — "Milestone 006 owns daemon lifecycle management" | M007 qualifies and documents whatever M6 delivers; it does not implement lifecycle management. |
| `bundle.active: false` | `crates/eggsec-gui/tauri.conf.json:26-28` | §7 Work package C decides and records the position. |
| No GUI build on macOS or Windows in CI | `deep-checks.yml:60-76` | §7 Work package C adds the matrix job. |
| `ApprovePolicy` remains unwired | `plans/closure/desktop-gui-frontend/005-schema-driven-forms-and-task-dashboard-closure.md:469` | Out of scope; recorded as a known limitation in the handoff artifact because it affects what an operator can do from the shipped binary. |
| The socket-path default is duplicated in six places | `plans/closure/desktop-gui-frontend/004-gui-shell-skeleton-closure.md:559` | Out of scope; recorded as a known limitation. |

### 3.9 Pre-existing failures inherited by this milestone — **not** regressions of M7

These are recorded so that the closure record does not attribute them to M7, and so that §14 stop conditions are precise.

1. **`make check-deps` fails on advisories.** `cargo deny --workspace --all-features check advisories` currently reports `advisories FAILED`, while `bans ok, licenses ok, sources ok`. `deny.toml:19-21` sets `unmaintained = "transitive"` and `unsound = "transitive"`, so transitive advisories are in scope. The ignore list (`deny.toml:24-38`) was last reviewed 2026-09-13.

   The current failure set is **larger than the set recorded in the M4/M5 closure records**, and it includes advisories that are **reachable only through the GUI's Tauri dependency**:

   | Advisory | Crate | Reachable through |
   |---|---|---|
   | RUSTSEC-2024-0429 (unsound) | `glib 0.18.5` | `eggsec-gui` → `tauri 2.11.6` → `muda 0.19.3` → `gtk 0.18.2` → `atk 0.18.2` → `glib 0.18.5` |
   | RUSTSEC-2024-0370 (unmaintained `proc-macro-error`) | `glib-macros 0.18.5` | same chain |
   | RUSTSEC-2025-0081 / 0075 / 0080 / 0100 / 0098 (unmaintained `unic-*`) | `unic-char-property`, `unic-char-range`, `unic-common`, `unic-ucd-ident`, `unic-ucd-version` | `eggsec-gui` → `tauri-utils 2.9.3` → `urlpattern 0.3.0` → `unic-*` |
   | RUSTSEC-2026-0367 / 0368 / 0369 (vulnerability) | `zip 2.4.2` | engine / daemon closure — **not** GUI-reachable |
   | RUSTSEC-2024-0449 (unmaintained `unsafe-libyaml`) | `serde_yaml_neo 0.11.0` | `eggsec` — **not** GUI-reachable |
   | RUSTSEC-2026-0346 / 0347 / 0348 / 0349 (`rustls 0.21.12`, `0.23.45`), 0382 (`rustls-webpki 0.103.15`), 0383 (`webpki-root-certs 1.0.9`), 0386 (`tokio 1.53.1`) | see the note below | **shared** with the engine, but `cargo deny` names `eggsec-gui` in their chain |
   | RUSTSEC-2026-0385 (vulnerability) | `ctutils 0.4.2` | `eggsec` / `eggsec-db-lab` — **not** GUI-reachable |

   The GUI-exclusive rows are confirmed by dependency-graph inspection, not by inference from the advisory text:

   ```bash
   # present in the GUI's closure, absent from the engine's and the daemon's
   cargo tree -p eggsec-gui    --all-features --target all -e normal --prefix none | grep -E '^(glib|glib-macros|gtk|atk|urlpattern|unic-)'
   cargo tree -p eggsec        --all-features --target all -e normal --prefix none | grep -cE '^(glib|gtk|urlpattern|unic-)'
   cargo tree -p eggsec-daemon --all-features --target all -e normal --prefix none | grep -cE '^(glib|gtk|urlpattern|unic-)'
   ```

   The first command lists `glib v0.18.5`, `glib-macros v0.18.5`, `gtk v0.18.2`, `atk v0.18.2`, `urlpattern v0.3.0`, `unic-char-property v0.9.0`, and `unic-ucd-ident v0.9.0` (the same `grep` also matches the `atk-sys`, `glib-sys`, and `gtk-sys` packages and, on Windows targets, `webview2-com`); the second and third print `0`.

   **Two graphs disagree here, and the difference decides who owns an advisory.** `rustls` appears **zero** times in the GUI's **feature-resolved** tree, across all targets — which is exactly what `crates/eggsec-gui/Cargo.toml:51-55` claims when it declines the `rustls-tls` and `native-tls` Tauri features. So the GUI's *built binary* links no second TLS stack, which is what satisfies guard 157's ring-only invariant without an exception. But `cargo deny` resolves advisories over the **lockfile** graph, in which `tauri` carries its optional `reqwest` edge; there, `rustls 0.23.45` reaches `eggsec-gui` through `eggsec-gui` → `tauri 2.11.6` → `reqwest 0.13.5` → `hyper-rustls 0.27.9`. The same is true of `rustls-webpki 0.103.15` and `webpki-root-certs 1.0.9`. Those advisories are therefore **shared** — the engine reaches them independently — but `make check-deps` will name `eggsec-gui` in their dependency chain, and the closure record MUST say so rather than claim the GUI is uninvolved. The seven **GUI-exclusive** entries are the ones the GUI alone contributes.

   The `glib` and `unic-*` rows mean the GUI's closure **now contributes** to the failing dependency gate. This does not change the roadmap's requirement — "`make check-deps` MUST pass with the GUI's dependency closure admitted to `deny.toml`; any exception MUST also be recorded in `docs/DEPENDENCY_EXCEPTIONS.md` with an owner and a review-by date" (`roadmap:826-828`) — but it does mean the GUI's closure is **not** currently admitted, and admitting it is a **maintainer decision** (owner + review-by date), not something M007 may decide for the maintainer. The roadmap's own risk table anticipates this: "Re-verify on the resolved graph; if it still fails, admit with an owner and a review-by date, or fall back to egui" (`roadmap:855`).

2. **`cargo check --workspace --all-features` fails** with `E0616: field 'url' of struct 'WebSocketFuzzer' is private` at `crates/eggsec/src/fuzzer/advanced.rs:446` (`url: self.url.clone()`), because the corpus type declares `pub(crate) url` at `crates/eggsec-payloads/src/websocket.rs:43` — not visible across the crate boundary. Verified pre-existing in `plans/closure/desktop-gui-frontend/003-desktop-gui-surface-parity-closure.md:273`. It is **not** in `make check`; it affects `make check-full` and `make check-features-individual`.

3. **`make check-python` was not run** in the M5 closure (`plans/closure/desktop-gui-frontend/005-schema-driven-forms-and-task-dashboard-closure.md:473`) — it requires a `.venv-ci/` environment. M007 changes no Python file; the same position carries forward.

4. **The `check_doc_references.py` baseline is "Total missing: 29"**, measured at this commit. A new documentation file must not increase that count.

5. The architecture guards are all green (`scripts/check-architecture-guards.sh`; the last numbered check is 160 at `scripts/check-architecture-guards.sh:4792`; the script contains **166** check blocks in total — 159 purely numbered, plus 7 letter-suffixed blocks such as `7b` — and the run at this baseline ends with `ALL PASSED: No architecture drift detected.`). The script prints **no** check count, so no closure record may assert one.

---

## 4. Invariants that must not regress

1. **The GUI stays a leaf application.** It MUST NOT become a dependency of the engine, daemon, or runtime crates, and MUST NOT be published to crates.io (`plans/adrs/ADR-0007-desktop-gui-frontend-boundary.md:84-91`, `:210-213`). Enforced by guard 155 ("GUI crate links no engine").
2. **The GUI imports no enforcement internals.** It MUST NOT import `EnforcementContext`, `EnforcedDispatcher`, `Scope::is_target_allowed`, or `tool.execute()` (`roadmap:769-771`). Enforced by guard 154 ("desktop GUI stays a manual surface").
3. **The GUI is a manual surface** (`RuntimeSurface::DesktopGuiManual` / `ExecutionSurface::DesktopGuiManual`) — permissive profile, operator overrides allowed, evaluated by the engine, never by the GUI (`roadmap:772-774`).
4. **The GUI performs no outbound HTTP of its own**; all engine traffic travels over the daemon's local transport (`roadmap:782-783`). The `rustls-tls` / `native-tls` Tauri features stay disabled (`crates/eggsec-gui/Cargo.toml:51-55`), and guard 157 ("GUI declares no second TLS stack") keeps the workspace's ring-only TLS rule true.
5. **The CSP admits no remote origin** beyond `http://ipc.localhost` (`crates/eggsec-gui/tauri.conf.json:22-24`); guard 158 enforces it. M007 must not weaken the CSP to make a platform build succeed.
6. **The frontend keeps zero npm dependencies.** Any JavaScript dependency admitted later requires a recorded policy with an owner and a review-by date (`roadmap:866-869`).
7. **Tokio baseline**: `default-features = false` with explicit per-crate feature lists; no `test-util` anywhere; DTO crates carry no Tokio.
8. **Spawned Tokio tasks in the GUI need 30–300 s timeout wrappers**; failures are reported through `tracing`, never `let _ =` and never `filter_map(|e| e.ok())` (`roadmap:790-792`).
9. **`make check` stays green** for everything M007 touches: fmt, the `--no-default-features` workspace check, the gated GUI shell check, clippy, the test suites, and the architecture guards.
10. **The architecture guards stay green**, and any new guard M007 adds is itself negative-tested (violated on purpose, observed failing, then restored).
11. **`deny.toml` remains the canonical dependency policy.** New exceptions require an owner and a review-by date in `deny.toml` **and** `docs/DEPENDENCY_EXCEPTIONS.md`. `.cargo/audit.toml` is not reintroduced.
12. **The Tauri version pin holds** on the 2.11.x line (`ADR-0009:218-224`; `Cargo.toml:93-98`), keeping `make check-msrv` green without a workspace MSRV bump.
13. **The GUI owns no authoritative storage** and requires **no data migration** (`roadmap:739-745`).
14. **The GUI never persists credentials** in its own state or logs (`roadmap:781`).

---

## 5. Scope

### In scope

- The **platform qualification matrix** artifact, its schema (one row per platform, one column per required behaviour), and the evidence-recording mechanism that keeps it honest.
- The **Linux system prerequisites** documentation: per-distribution package names for Debian/Ubuntu, Fedora, and Arch, plus the supported Linux floor.
- The **reproducible build recipe**: a script and a Makefile target that produce a per-platform release binary plus its build metadata (commit, toolchain, target triple, feature set, checksums), without signing or bundling.
- A **CI platform-matrix job** that builds the GUI and runs its test suite on each supported platform, so the qualification is reproducible rather than a one-off manual claim.
- The **qualification procedure** for the behavioural columns: launch, connect, submit, cancel, render, export — written so an operator or a CI job can execute each cell and record the result.
- The **handoff artifact** for the separately-managed distribution workstream: the qualified binary's identity, prerequisites, known limitations, and the explicit list of decisions the distribution workstream still owns.
- The **known-limitations record**.
- The **documentation, guard, and closure-evidence** updates that make the above durable.

### Explicitly out of scope

- **Signing, notarization, installers, and update channels.** The roadmap states it directly: "This milestone does not own signing, notarization, or installers" (`roadmap:600`), and "Deferred work: signing, notarization, installers, update channels — all owned by the separate distribution workstream" (`roadmap:610-611`). `ADR-0007:88-91` and `:249-251` place binary distribution and platform packaging outside this workstream entirely.
- **Enabling the Tauri bundler.** See §7 Work package C for the decision and its justification against the stated boundary.
- **Changing the daemon transport.** Adding a Windows named-pipe transport would be a daemon-subsystem change and, because it alters the protocol's transport kind, potentially a non-additive protocol change requiring its own ADR (`roadmap:863-865`). M007 records the gap and hands it off.
- **Implementing daemon lifecycle management** (spawn, supervise, stop) — owned by M6.
- **Report export** — owned by M6. M007 qualifies the export *column* and leaves the cell dependency-gated.
- **Adding a seventh platform domain to `crates/eggsec/src/platform/`.** The GUI is a build-time platform requirement, not a runtime probing domain; `docs/PLATFORM.md:11-14` requires a matrix row for any new domain, and M007 adds none.
- **Recording `deny.toml` advisory exceptions.** That is a maintainer decision requiring an owner and a review-by date (`roadmap:826-828`; `docs/DEPENDENCY_EXCEPTIONS.md`). M007 reports the failure and its GUI-reachable share; it does not write the exception.
- **Fixing the `--all-features` `E0616` compile error** — a knowledge-corpus crate's field visibility, outside this milestone (as recorded at `plans/closure/desktop-gui-frontend/003-desktop-gui-surface-parity-closure.md:273`).
- **Any npm or JavaScript dependency.**
- **Any engine, daemon-protocol, runtime, or policy change.**

---

## 6. Required production changes

This is an operational milestone with a small production-code surface. The production change is intentionally narrow: **a build and qualification toolchain**, plus **one documentation set** and **one static guard**. M007 does not change the GUI's runtime behaviour.

### Core/domain

- No engine, daemon, runtime, or policy code changes.
- The GUI crate's runtime modules (`config.rs`, `connection.rs`, `form.rs`, `tasks.rs`, `commands.rs`, `shell.rs`) are **not** modified by M007, except where a qualification procedure reveals a defect that must be fixed for the platform to qualify — and any such fix is reported as an explicit, separately-scoped change, not smuggled into the packaging work.

### Storage and migrations

- None. The GUI owns no authoritative storage (`roadmap:739-745`). The qualification record and the handoff artifact are **checked-in documents**, not runtime state.
- The build recipe writes its outputs to a caller-supplied output directory that is **outside** the source tree by default (`dist/` at the repository root is the proposed default and MUST be git-ignored by this milestone), so no build artifact is ever committed.

### Protocol and DTOs

- None. The daemon protocol is unchanged (`roadmap:747-765`).

### Runtime and concurrency

- None. The GUI's runtime semantics are M1–M5's subject.
- One runtime-adjacent obligation is *measured*, not changed: the GUI's **resident footprint** (`roadmap:803-805`), which was never recorded in the M4 closure. M007 records it per platform in the handoff artifact. The measurement must not alter the GUI's memory behaviour; it is an observation.

### Frontend or operator surface

- No change to the webview, the command set, or the CSP.
- The **operator-facing** change is the new runbook: how to install the Linux prerequisites, how to launch the qualified binary, how to point it at a daemon, and how to reproduce the build. This belongs in the documentation set below, not in the GUI's UI.

### Security and authorization

- The built binary **must not embed secrets**. The handoff artifact records that the binary was produced by a clean tree at a pinned commit with no credential-bearing environment inputs, and M7's negative tests (§10) check that no credential-shaped string appears in the shipped artifact's metadata or in the qualification logs.
- The binary **must not carry a remote origin in its CSP**: the CSP at `crates/eggsec-gui/tauri.conf.json:22-24` is checked into the source tree and therefore into the binary; M007's negative test asserts the shipped binary's embedded CSP still admits no remote origin beyond `http://ipc.localhost`.
- The **capability set stays `core:default`** — the shipped binary must not enable the `http`, `shell`, `fs`, `process`, or `updater` plugins (`crates/eggsec-gui/capabilities/default.json`).
- The GUI's **daemon transport remains local-only**; the binary must not be qualified on any platform where the transport would require a network listener.

### Documentation and static guards

- **New**: `docs/GUI_PLATFORM_QUALIFICATION.md` — the qualification matrix, the per-behaviour evidence references, and the named gaps.
- **New**: `docs/GUI_DISTRIBUTION_HANDOFF.md` — the handoff artifact for the distribution workstream.
- **New**: `docs/GUI_BUILD.md` (or an equivalent section inside `docs/VERIFICATION.md`) — the build recipe, the Linux prerequisites, and the per-distribution package names.
- **Updated**: `architecture/desktop_gui.md` — the platform section (`:377-387`) gains the measured platform floor, the qualified matrix reference, and the distribution-handoff pointer.
- **Updated**: `docs/PLATFORM.md` — a short cross-reference stating that the GUI's build requirements are documented separately and are **not** one of the six runtime probing domains.
- **Updated**: `docs/VERIFICATION.md` — the "Desktop GUI platform prerequisites" section (`:102-110`) and "Prerequisite-skip semantics" (`:112-132`) gain the new matrix job and the explicit statement that `make check` does not run the GUI's test suite.
- **Updated**: `.opencode/skills/eggsec-gui/SKILL.md` — the platform and packaging section, so the skill does not drift from `architecture/desktop_gui.md`.
- **New static guard (check 168)**: "the GUI's platform qualification record names every platform it claims and carries no unqualified support claim, and the Tauri bundler stays inactive." It must:

  > **Guard numbering.** M007 is the **last** milestone in this subsystem's order, because it is gated on M6, M8, and M9. The guards land in execution order: M006 (**161**, **162**), M008 (**163**), M009 (**164**–**167**), and M007 (**168**). The implementer MUST confirm the next free number against `scripts/check-architecture-guards.sh` at execution time and MUST NOT renumber another milestone's checks.
  - require `docs/GUI_PLATFORM_QUALIFICATION.md` to exist;
  - require a row for each of Linux and macOS, and an explicitly **out of scope** row for Windows naming its root cause (`ADR-0010`);
  - require every status cell to be one of an enumerated set (`qualified`, `partially qualified`, `not qualified`, `pending M6`, `out of scope`) — no blank, no free-text claim;
  - require `crates/eggsec-gui/tauri.conf.json` to keep `bundle.active: false`, or to name an ADR that supersedes the decision.

  The guard is written in the style of the existing ones (an `echo "--- Check 168: …"` block, a `SECTION_FAIL` counter, and a `python3` heredoc — cf. `scripts/check-architecture-guards.sh:4665-4718`), and it is negative-tested.

---

## 7. Ordered work packages

### Work package A — Define the platform qualification matrix and its evidence mechanism

**Intent:**
Make "qualified" a checkable, reviewable claim instead of an assertion. The roadmap requires the GUI to carry "its own platform qualification evidence (launch, connect, submit, cancel, render) on each supported platform, since the `make check` contract does not exercise a windowed application" (`roadmap:844-846`), and the M7 exit conditions require "each builds and launches, with the Linux system prerequisites documented and known limitations recorded" (`roadmap:605-608`).

**Required changes:**
1. Create `docs/GUI_PLATFORM_QUALIFICATION.md` with a fixed schema:
   - **Rows**: one per platform — `linux`, `macos`, `windows`.
   - **Columns**: one per required behaviour, in this order:
     | Column | What "pass" means |
     |---|---|
     | Build | The release binary is produced by the §7 C recipe at the pinned commit. |
     | Launch | The binary starts, creates its window, and exits cleanly on a signal. |
     | Connect | The GUI attaches to a running daemon over the local transport and lists the daemon's advertised capabilities. |
     | Submit | A task kind is submitted from a schema-derived form and the daemon accepts it. |
     | Cancel | The submitted task is cancelled and the GUI's dashboard reflects the terminal state. |
     | Render | The task's result renders in the dashboard (controls, result rows, counters). |
     | Export | The result exports to a report artefact (M6-dependent). |
   - **Cell grammar**: every cell is one of `qualified` (with an evidence reference), `partially qualified` (with the qualified subset named and the missing part named as a gap), `not qualified` (with the root cause), or `pending M6` (with the blocking milestone named), or `out of scope` (with the decision that excludes the platform and its root cause). No cell may be blank, and no cell may carry an unqualified claim.
   - **Evidence reference grammar**: a path plus a command plus a date, e.g. `docs/evidence/gui-qualification/linux-2026-xx-xx.md` or a CI run URL. Evidence must be reproducible by the documented command.
2. Add the **evidence-recording mechanism**: a checked-in `docs/evidence/gui-qualification/` directory (one Markdown file per qualification run) and a template that fixes the fields the matrix cell must point at — platform, host/OS version and architecture, commit SHA, toolchain, feature set, the exact commands run, the observed output, and the operator or CI job that produced it.
3. Record the matrix's own **provenance**: it is regenerated by a documented procedure, and the header states the commit and the date it was last regenerated.
4. Record the Windows finding from §3.3 as the **out-of-scope row** that `ADR-0010` requires — not a blank, not a hopeful cell, and not a gap awaiting a workaround — with the exact cross-compilation error text, the two unconditional unix-only imports that cause it, and a pointer to the ADR that decides it.

**Acceptance evidence:**
- `docs/GUI_PLATFORM_QUALIFICATION.md` exists, contains the two qualified rows (Linux, macOS) and the one out-of-scope row (Windows) and the seven behaviour columns, and every cell matches the cell grammar.
- A reviewer can pick any non-gap cell and re-run its cited command to reproduce the cited result.
- The Windows row cites the cross-compilation failure with its root cause, names `ADR-0010` as the decision that excludes the platform, and the row's status is `out of scope` — never `qualified` and never `pending`.

### Work package B — Document the Linux system prerequisites and the supported Linux floor

**Intent:**
Satisfy the roadmap's exit condition "the Linux system prerequisites documented" (`roadmap:605-608`) and discharge `ADR-0009`'s third condition, which the shell milestone was required to satisfy and which was not closed: "**Linux system-dependency declaration.** The GUI crate MUST declare its Linux prerequisites explicitly, and the supported Linux baseline MUST be recorded as a documented platform floor (webkit2gtk-4.1 implies Ubuntu 22.04 / Debian 12 or newer)" (`ADR-0009:235-240`).

**Required changes:**
1. Write the Linux prerequisite section with the package names **as the Tauri v2 documentation states them** (Tauri v2 "Prerequisites → Linux"):
   - **Debian / Ubuntu** — `libwebkit2gtk-4.1-dev`, `build-essential`, `curl`, `wget`, `file`, `libxdo-dev`, `libssl-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev`.
   - **Fedora** — `webkit2gtk4.1-devel`, `openssl-devel`, `curl`, `wget`, `file`, `libappindicator-gtk3-devel`, `librsvg2-devel`, `libxdo-devel`, plus the `c-development` group.
   - **Arch** — `webkit2gtk-4.1`, `base-devel`, `curl`, `wget`, `file`, `openssl`, `appmenu-gtk-module`, `libappindicator-gtk3`, `librsvg`, `xdotool`.
   - **macOS** — Xcode, or `xcode-select --install` for desktop-only development.
   - **Windows** — the Microsoft C++ Build Tools ("Desktop development with C++") and the Microsoft Edge WebView2 runtime (preinstalled on Windows 10 build 1803 and later). The **VBSCRIPT** optional feature is required only for building MSI installers and is therefore **out of M007's scope** — it belongs to the distribution workstream.
2. State the **platform floor** explicitly: `webkit2gtk-4.1` implies Ubuntu 22.04 / Debian 12 or newer (`ADR-0009:102-103`, `:237-238`). State which distributions are therefore **below the floor** and what the operator should do (the egui fallback in `ADR-0009:242-246`, or a supported distribution).
3. Reconcile with the repository's existing lists. The workspace's own CI installs a **subset** — `pkg-config libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev patchelf` (`ci.yml:50-51`, `deep-checks.yml:30`, `:56`) — and `Makefile:17` carries a `GUI_SHELL_PREREQ_HINT` with the same names. The document MUST state which of the Tauri's names the workspace's own CI list omits (`build-essential`, `curl`, `wget`, `file`, `libxdo-dev`, `libssl-dev`) and whether that omission is safe on the CI images (it is, because the GitHub-hosted Ubuntu image pre-installs the build toolchain and `libssl-dev`), so an operator on a bare distribution is not misled into a partial install.
4. Record the gate semantics alongside the prerequisites: on Linux the `make check` GUI line **skips** when `pkg-config` cannot find `webkit2gtk-4.1` or `webkit2gtk-4.0` (`Makefile:22-29`), so a green `make check` on Linux does not prove the shell compiles — `make check-gui` is the ungated check (`Makefile:187-190`). This is the same statement `docs/VERIFICATION.md:112-132` already makes; M007 links it rather than restating it.
5. Add a **prerequisite probe** script or Makefile target that a host can run to report whether the GUI's shell profile can be built, so an operator does not have to install a full toolchain to find out.
6. **Separate the shell's prerequisites from the daemon's.** The lists above are the **Tauri shell's** prerequisites, which the GUI needs. The **daemon's** engine-feature prerequisites are a different set, owned by **milestone 008**'s profile and its build recipe — and M007 links that profile's list rather than restating it, so the two cannot drift. Two measured facts belong in that link, because the repository's own prose is stale on both: `packet-inspection` needs **no** libpcap (it is pure Rust — `pnet`, `pnet_packet`, `libc`), and `nse` **vendors** OpenSSL (so it needs a C toolchain, not a system `libssl-dev`). M008's plan carries the corrections to `docs/PLATFORM.md`, `docs/FEATURE_MATRIX.md`, and `crates/eggsec/src/platform/prereqs.rs`; M007 MUST NOT reintroduce the stale claims when it writes the qualification record.

**Acceptance evidence:**
- The documented package names match the Tauri v2 documentation exactly (a reviewer checks them against the upstream prerequisites page).
- The platform floor sentence cites `ADR-0009` and the `webkit2gtk-4.1` → Ubuntu 22.04 / Debian 12 implication.
- The probe script reports the GUI shell profile as buildable or not on the executing host, and reports "not buildable" rather than "skipped" when the prerequisite is absent.
- The document explicitly states that the Linux `make check` GUI line may skip, and names `make check-gui` as the ungated check.

### Work package C — Produce the reproducible build recipe and the CI platform-matrix job

**Intent:**
Make the qualification reproducible and CI-enforced, and decide — explicitly and with justification — what M007 hands over: a raw binary plus a recipe, or a bundled installer.

**Required changes:**

*Decision 1 — `bundle.active` stays `false`.*

Rationale, stated against the milestone's own boundary:

- The roadmap says M7 "does not own signing, notarization, or installers" (`roadmap:600`) and that installers are "owned by the separate distribution workstream" (`roadmap:610-611`). `ADR-0007:88-91` and `:249-251` say the same.
- Enabling the Tauri bundler makes `cargo build`/`tauri build` emit `.deb`, `.rpm`, `.AppImage`, `.dmg`, `.app`, `.msi`, and `.exe` installers — precisely the artefacts M007 does not own.
- The bundler also introduces platform-specific *build* requirements that are themselves packaging concerns: the Windows MSI path requires the VBSCRIPT optional feature, and the macOS path requires a bundle identifier and (for distribution) signing identity.
- What the roadmap's user value actually requires — "an installable desktop build exists for the platforms the project supports" (`roadmap:602-603`) — is satisfied by a single self-contained binary plus a documented system-prerequisite list. The Linux prerequisite list is already a required deliverable (`roadmap:605-608`), which means the "install" step is documented prerequisites + a binary, not a package-manager artefact.
- Enabling the bundler would also make `make check`'s GUI line produce installers as a side effect, polluting the mandatory contract with artefacts that are outside the milestone.

**Therefore:** `bundle.active` remains `false`. M007 produces the **binary** and the **reproducible build recipe**; the distribution workstream inherits the binary and decides whether to enable the Tauri bundler, adopt another packaging route, or ship a raw artefact. Guard 168 enforces the decision until an ADR supersedes it.

*Decision 2 — a build recipe that is reproducible and honest about what it cannot do.*

Create `scripts/build_gui_release.sh` and a `make gui-release` target that:
- refuse to run on a dirty working tree unless `--allow-dirty` is passed, and record the tree state in the build metadata either way;
- build with an explicit, pinned invocation: `cargo build --release -p eggsec-gui --features shell --locked`, with the target triple, the toolchain (`rustc -Vv`), and the resolved feature set recorded;
- emit the binary plus a **build metadata sidecar** (JSON or TOML) containing: commit SHA, dirty flag, rustc version, cargo version, target triple, enabled cargo features, the resolved `Cargo.lock` hash, the binary's SHA-256, and the binary's size;
- write into a caller-supplied output directory that is **git-ignored** by default (`dist/`), so no artefact is ever committed;
- **not** sign, notarize, bundle, or publish.

*Decision 3 — a CI platform-matrix job.*

Add a `gui-platform` job to `.github/workflows/deep-checks.yml`, next to the existing `portability` job (`deep-checks.yml:60-76`), with:

- `strategy.fail-fast: false` and `matrix.os: [ubuntu-latest, macos-latest]`.
  - **Windows is excluded from the matrix by decision** (`ADR-0010`), and the exclusion is stated in the job's own comment and in the qualification matrix. The reason is the measured failure in §3.3: the GUI's unconditional dependency `eggsec-frontend-client` does not compile on Windows, so a Windows leg would be **permanently red**. A red job is worse than an explicit, documented exclusion; adding the Windows leg is gated on the daemon-transport milestone `ADR-0010` names as the re-entry path.
- On Linux: install the documented prerequisites, then run `make check-gui` (the ungated escape hatch — not the gated `make check` line), `cargo test -p eggsec-gui --features shell --lib`, and `make gui-release`.
- On macOS: run the same three commands (no system packages needed — the webview ships with the OS).
- The job uploads the build metadata sidecar and the binary's checksum as artifacts so the matrix cell's evidence reference is a CI URL rather than a claim.

**Decision 4 — close the test-suite gap.**

`make check` does not run the GUI's tests (§3.6). M007 adds `cargo test -p eggsec-gui --lib` to the `gui-platform` job (both legs) so the 34 GUI tests are exercised in CI on every supported platform. The GUI's tests are headless-safe — no socket, no window, no subprocess (§3.2) — so they run unattended on a CI runner with no display. The `--no-default-features` variant of the same suite MUST also run, because the shell feature gates `commands.rs` and `shell.rs` (`crates/eggsec-gui/src/lib.rs:16-19`) and the non-shell path is the one the `--no-default-features` workspace check compiles.

**Acceptance evidence:**
- `make gui-release` on Linux and macOS produces a binary plus a metadata sidecar containing every field listed above, from a clean tree at a pinned commit.
- The `gui-platform` job runs green on `ubuntu-latest` and `macos-latest`, and the matrix's Build column cites the job.
- `cargo test -p eggsec-gui --lib` and `cargo test -p eggsec-gui --no-default-features --lib` both run inside the job, and the 34 tests pass on both platforms.
- The job's comment and the qualification matrix both state, in the same terms, that Windows is excluded and why.

### Work package D — Execute the launch/connect/submit/cancel/render qualification procedure

**Intent:**
Produce the behavioural evidence the roadmap names — "launch, connect, submit, cancel, render" (`roadmap:844-846`) — for every platform the matrix claims, using a procedure that an operator or a CI job can execute and record.

**Required changes:**

1. Write the procedure as a runbook in `docs/GUI_BUILD.md` (or the qualification document), one numbered step per behaviour, each with:
   - the **precondition** (daemon running, socket path, scope);
   - the **exact command or action**;
   - the **observable pass criterion** (what the operator must see);
   - the **evidence artefact** the step produces (a log excerpt, a screenshot-equivalent, a recorded line).
2. The procedure must cover the daemon dependency honestly. The GUI is **daemon-only** (`ADR-0007`): it cannot connect to anything else, and there is no embedded-runtime path in the GUI crate. The procedure therefore starts the daemon separately (`eggsec-daemon --full-executor --socket-path <path>`, the command the GUI itself names in its unreachable diagnostic — `crates/eggsec-gui/src/connection.rs:206`, `:211-214`) and points the GUI at it with `--socket <path>` (`crates/eggsec-gui/src/config.rs:47`). If M6 lands before M007 executes, the runbook uses **M6's** daemon-lifecycle path instead and says so; if M6 has not landed, the runbook uses the manual start above and records that the lifecycle is M6's.
3. The procedure must specify the **cancel race** honestly: cancellation is best-effort, and a task that completes between the operator's click and the daemon's processing is not an error — the event stream is authoritative (`crates/eggsec-gui/src/commands.rs:432-434`). The runbook records the terminal state observed, not the click.
4. The procedure must specify the **render** check as *rendering a real result*, not merely painting an empty dashboard — including the generic-kind degradation path that the GUI shows for a task kind with no schema.
5. The **export** column is **dependency-gated**: the procedure is written and reviewed, but the cell stays `pending M6` until M6 closes, at which point the procedure is executed once and the cell is updated with its evidence reference.
6. Record the procedure's own **repeatability contract**: the same steps on the same platform with the same daemon produce the same observable outcomes; any deviation is recorded as a cell gap, not smoothed over.

**Acceptance evidence:**
- Each behaviour column in the matrix, for each claimed platform, cites an evidence artefact produced by the documented procedure.
- The procedure names the daemon precondition explicitly; no step assumes a daemon the GUI itself started.
- The cancel step records the terminal state, and the runbook states that a lost cancel race is not a failure.
- The export cell is `pending M6` with the blocking milestone named.

### Work package E — Produce the distribution handoff artifact and the known-limitations record

**Intent:**
Hand the binary to the separately-managed distribution workstream with everything it needs and nothing it must re-derive, while stating plainly what it still owns.

**Required changes:**

1. Create `docs/GUI_DISTRIBUTION_HANDOFF.md`. Required sections, in this order:
   - **Artifact identity** — the binary name, the platform and architecture, the commit SHA, the tree state, the toolchain, the enabled cargo features, the SHA-256, the size, and the build metadata sidecar's path.
   - **Runtime prerequisites** — the per-platform prerequisite list (Linux package names and floor; macOS Xcode/webview; Windows build tools and WebView2) and the daemon the binary expects.
   - **What is NOT in the artefact** — no installer, no signature, no notarization, no update channel, no bundled daemon. The GUI binary does not ship the daemon: `cargo install --path crates/eggsec-cli` installs only the `eggsec` binary, and nothing in the workspace installs `eggsec-daemon`. The handoff must state that the daemon is a **separate binary the distribution workstream must also deliver**, or explicitly record that the GUI is expected to be installed alongside a separately-managed daemon.
   - **Known limitations** — the explicit list (§3.8 plus the platform gaps found in A).
   - **Supply-chain posture** — the `deny.toml` outcome for the shipped closure, including the GUI-reachable advisories from §3.9, and an explicit statement that the exceptions are a maintainer decision.
   - **Decisions the distribution workstream owns** — the list below.
   - **Decisions M007 made** — the `bundle.active` decision, the recipe, the supported-platform recommendation.
2. Record the **known limitations** as an explicit, numbered list. At the baseline this list must include:
   - Windows: **out of scope for the desktop GUI** by decision (`ADR-0010`), because `eggsec-frontend-client` (`crates/eggsec-frontend-client/src/daemon.rs:9`, `:56`) and `eggsec-daemon` (`crates/eggsec-daemon/src/server.rs:1`, `:5`) use Unix-domain-socket APIs unconditionally. The engine, CLI, TUI, and Python surfaces are unaffected.
   - The daemon's transport is a Unix domain socket at `/tmp/eggsec-daemon.sock` by default (`crates/eggsec-gui/src/config.rs:17`); there is no socket-path environment variable and no XDG lookup, and the default is duplicated in six places.
   - The GUI does not manage the daemon's lifecycle (M6's).
   - `ApprovePolicy` is unwired: the daemon answers it with `ErrorCode::Unsupported`, so the GUI can mark a task `awaiting_policy` but can offer no approval affordance and must not auto-approve (`roadmap:775-777`).
   - The GUI's resident footprint was not measured before this milestone.
   - The `make check-deps` advisory failure, including the GUI-reachable share.
   - The `--all-features` `E0616` compile error.
   - The Linux platform floor and the distributions below it.
3. **Record** the supported-platform matrix, which is **already decided** by `ADR-0010`: **Linux and macOS are qualified; Windows is out of scope** for the desktop GUI. M007 supplies the **evidence** for the two qualified platforms and the **root cause** for the out-of-scope row. The decision itself is no longer the distribution workstream's to make; what remains the distribution workstream's is the **distribution** policy — signing, notarization, installers, update channels (`roadmap:595-597`) — and §13 still makes its sign-off an acceptance precondition for M007's closure.
4. Name the **re-entry path** for Windows precisely, as `ADR-0010` records it: a Windows daemon transport (named pipes, or an equivalent local IPC) is a **daemon-subsystem** change; because it changes the transport kind rather than adding a read path, it requires the ADR the roadmap demands for "any change to the daemon protocol that is not strictly additive" (`roadmap:864`). M007 names the re-entry path; it does not implement it, and it MUST NOT add a Windows transport to make the matrix green.

**Acceptance evidence:**
- `docs/GUI_DISTRIBUTION_HANDOFF.md` contains every required section, and each section names something concrete (a commit, a checksum, a package name, a file path, a decision) rather than a category.
- The known-limitations list is numbered, and the Windows entry cites the two unconditional unix-only imports and the cross-compilation error.
- The handoff states what the distribution workstream inherits and what it must decide, with no overlap between the two lists.
- The supply-chain section names the GUI-reachable advisories and states that recording the exceptions is a maintainer decision.

### Work package F — Documentation, skills, static guard, and closure evidence pack

**Intent:**
Make the milestone's findings durable and guard-enforced, so the qualification cannot silently rot into a stale claim.

**Required changes:**
1. Update `architecture/desktop_gui.md` — extend the platform section (`:377-387`) with the measured platform floor, the qualified-matrix reference, the `bundle.active` decision, the CI job name, and the distribution-handoff pointer. Add the measured resident footprint.
2. Update `docs/VERIFICATION.md`:
   - the "Desktop GUI platform prerequisites" table (`:102-110`) gains the new matrix job and the fact that `make check` does not run the GUI's test suite;
   - "Prerequisite-skip semantics" (`:112-132`) gains a sentence stating that the Linux GUI line can skip and that the qualification matrix is the authoritative statement of what was actually built;
   - "Make targets reference" gains `gui-release` and the prerequisite probe.
3. Update `docs/PLATFORM.md` with a cross-reference stating that the GUI's platform requirements are **build-time** requirements documented in the qualification document, and that the GUI is **not** one of the six runtime probing domains (`docs/PLATFORM.md:11-14`).
4. Update `.opencode/skills/eggsec-gui/SKILL.md` — its platform/packaging section must agree with `architecture/desktop_gui.md`, per the repository's skill-drift discipline. The skill points at `architecture/overview.md` for headline counts rather than restating them.
5. Add **guard 168** as specified in §6 ("Documentation and static guards"), and negative-test it: introduce a deliberate violation (a blank cell, a missing platform row, or `bundle.active: true`), confirm the guard fails with a named reason, restore, and confirm it passes again.
6. Update `plans/registry.md` and the roadmap's milestone status table to record M7's status and the closure record's path — **as part of the closure record**, not as part of this plan's authoring. (This plan does not modify either file.)
7. Run `python3 scripts/check_doc_references.py` and confirm the count has **not** increased beyond the baseline of **29**.
8. Run `cargo fmt --all` before committing.

**Acceptance evidence:**
- `bash scripts/check-architecture-guards.sh` ends with `ALL PASSED: No architecture drift detected.`, including the new check 168. (The script prints **no** check count — the summary line is the only marker; do not assert a count in the closure record.)
- The deliberate-violation run of check 168 fails with a named reason and the restore run passes again.
- `python3 scripts/check_doc_references.py` reports `Total missing: 29` (or lower).
- `cargo fmt --all --check` is clean.
- `architecture/desktop_gui.md`, `docs/VERIFICATION.md`, `docs/PLATFORM.md`, and `.opencode/skills/eggsec-gui/SKILL.md` all name the qualification document and the CI job consistently.

---

## 8. Failure, cancellation, restart, and contention semantics

M007 is an operational milestone with a small production-code surface. Its semantics are therefore split: the **qualification procedure's** repeatability, and the **runtime semantics** the milestone touches only by measurement.

### 8.1 The qualification procedure's semantics

| Situation | Required behaviour |
|---|---|
| **Partial failure** — one behaviour column fails on one platform | The cell records the failure and its root cause. **The row is not marked qualified.** A platform row is qualified only when every non-gap column is qualified. A failing column does not invalidate the other platforms' rows, and does not invalidate the milestone — it is the milestone's *output*. |
| **Duplicate delivery** — the same qualification is recorded twice, or two operators run the procedure concurrently | The matrix's cells reference **evidence artefacts**, not prose. A second run produces a **new** evidence artefact with its own date and host, and updates the cell's reference. Two concurrent runs never edit the same artefact: each writes its own file under `docs/evidence/gui-qualification/`. The matrix records the **most recent** reference and keeps the superseded one in the evidence directory. |
| **Process restart** — the matrix, the handoff artifact, and the evidence directory are checked-in documents; the build recipe is re-runnable | Re-running the recipe from a clean tree at the same commit reproduces the same binary hash **only if** the toolchain, the target, and the dependency versions are unchanged. The recipe records the toolchain and the `Cargo.lock` hash precisely so that a hash mismatch is **diagnosable** rather than mysterious. A hash change after a `cargo update` is a legitimate finding and MUST be recorded, not suppressed. |
| **Cancellation races** | The procedure's cancel step is explicitly race-tolerant: cancellation is best-effort and a task that completes between the operator's click and the daemon's processing is **not** a failure (`crates/eggsec-gui/src/commands.rs:432-434`). The runbook records the **terminal state observed**, never the click. A procedure run that records a click instead of a state is invalid evidence. |
| **Stale generations** — the matrix describes a GUI that has since changed | Every cell's evidence reference carries the commit SHA it was produced at. The matrix's header carries the commit it describes. A matrix whose header commit differs from the current HEAD is **stale** and MUST say so; guard 168 does not enforce freshness (it cannot), so freshness is a **review obligation** stated in the document and in the closure checklist. |
| **Concurrent callers** — two operators, or an operator and a CI job, run the qualification at once | They write distinct evidence artefacts and race only on the matrix file. The matrix is updated by a single write per cell group; a conflicting concurrent edit is resolved by the last writer and is **detected** by the commit review, not by tooling. The CI job does not write the matrix — it uploads artefacts; a human curates the matrix from the artifacts. |

### 8.2 The runtime semantics M007 touches by measurement only

- **The GUI holds no authoritative task state**; it requests cancellation through the protocol and consumes events from the daemon's subscription channel (`roadmap:787-790`). M007 changes none of this.
- **A dropped subscription must not affect the running task** (`roadmap:789-790`). The connect procedure's evidence must show a task completing even if the GUI's event subscription is interrupted — recorded as an observation, not a new invariant.
- **A GUI restart re-attaches to existing sessions and tasks** (`roadmap:789-790`). The procedure includes a restart step: disconnect the GUI, restart it against the same daemon, and record that the dashboard re-attaches to the still-running task. This is an M7-relevant observation of an existing guarantee.
- **Spawned Tokio tasks in the GUI require 30–300 s timeout wrappers**, and failures are reported through `tracing` — never `let _ =`, never `filter_map(|e| e.ok())` (`roadmap:790-792`). M007 adds no spawned task; the rule continues to bind any task a future change adds.
- **The GUI holds no unbounded event or result log** (`roadmap:805-808`); guard 160 enforces the bounded retention. M007's report-export qualification must not introduce a path that retains payloads without bound — the export procedure runs against the existing bounded store.

---

## 9. Compatibility and migration

- **No data migration.** No milestone in the roadmap changes a persisted representation (`roadmap:743-745`), and M007 changes no persisted representation at all.
- **No protocol change.** The daemon protocol is additive-only (`roadmap:747-765`) and M007 makes no change to it.
- **No configuration change.** `GuiOptions` keeps its shape (`crates/eggsec-gui/src/config.rs:21`); `--socket`, `--socket=`, `--help`, `-h` keep their meanings (`:47`, `:55`). The default socket path is unchanged (`crates/eggsec-gui/src/config.rs:17`).
- **No compatibility shim.** M007 introduces no legacy path, so there is nothing to remove later and no removal criteria to state.
- **The one compatibility surface M007 *does* touch** is the *operator-facing* one: the Linux prerequisite list. An operator on a distribution below the `webkit2gtk-4.1` floor is **newly** told that the GUI is unsupported there. This is a **documentation-only** change to the operator's expectations; no previously-working configuration stops working, because the GUI was never qualified on that distribution before.
- **Backward compatibility of the daemon**: an older daemon answers the GUI's new read-only command with the protocol's existing `UnsupportedCommand` error, which the GUI MUST treat as a graceful degradation to a static form set rather than a launch failure (`roadmap:749-765`). M007's connect step must record which daemon version it ran against, so a degradation is distinguishable from a failure.
- **The `bundle.active` decision is a one-way door in practice**: enabling the bundler later changes what `cargo build` emits for the GUI crate, so guard 168 makes that change require an explicit ADR rather than an accidental edit.

---

## 10. Required tests

### Focused unit tests

- **Guard 168's own unit** (in the guard script's style): a blank status cell, a missing platform row, an unrecognised status string, and `bundle.active: true` each fail the guard with a named reason.
- The GUI crate's existing 34 tests MUST continue to pass unchanged on every supported platform (§7 Work package C runs them in CI). M007 adds **no** unit test to the GUI crate unless a qualification procedure reveals a defect that must be fixed (§6 "Core/domain").
- A test for the **build recipe's metadata sidecar**: it contains every field listed in §7 Work package C, and the recorded SHA-256 matches the emitted binary. The recipe is scriptable, so the check is a script assertion, not a Rust test.

### Integration tests

- **The CI platform-matrix job** (`gui-platform` in `.github/workflows/deep-checks.yml`) is itself the integration evidence: on `ubuntu-latest` and `macos-latest` it runs `make check-gui`, `cargo test -p eggsec-gui --features shell --lib`, `cargo test -p eggsec-gui --no-default-features --lib`, and `make gui-release`.
- **The daemon round-trip**: the GUI must connect to a live `eggsec-daemon` over the Unix socket and complete one submit → event → render cycle. This is exercised by the §7 Work package D procedure on each claimed platform, and its evidence is the matrix's connect/submit/render cells.
- **The launch check**: the built binary starts, creates its window, and exits cleanly on a signal. Evidence is the process exit code and the absence of a panic in the captured output.
- **The daemon end-to-end test** `cargo test -p eggsec-daemon --lib the_gui_client_path_works_end_to_end` (recorded as passing in `plans/closure/desktop-gui-frontend/005-schema-driven-forms-and-task-dashboard-closure.md:269`) MUST remain green — it is the closest existing automated evidence that the GUI's client path works against a real daemon.

### Restart and recovery tests

- **GUI restart re-attach**: disconnect and restart the GUI against the same daemon; record that the dashboard re-attaches to the still-running task (`roadmap:789-790`).
- **Daemon restart**: restart the daemon while the GUI is open; record the GUI's behaviour (it must surface the error in its own UI state, not crash, and not write to the terminal — `roadmap:798-799`).
- **Stale-matrix detection**: the qualification document's header commit is compared with HEAD at review time; a mismatch marks the matrix stale.
- **Reproducible rebuild**: rebuilding at the same commit with the same toolchain yields the same binary hash; a mismatch is recorded as a finding with its cause.

### Contention and cancellation tests

- **Cancel race**: submit a task, cancel it, and record the **terminal state** — `cancelled` or `completed` — without treating a completed-after-cancel as a failure (`crates/eggsec-gui/src/commands.rs:432-434`).
- **Cancel the active task** path and the **cancel by task id** path are both exercised; the GUI exposes both commands (`crates/eggsec-gui/src/commands.rs:435`, `:451`).
- **Concurrent qualification runs**: two runs against the same platform produce distinct evidence artefacts and do not corrupt the matrix's cell references.

### Security and negative tests

- **No credentials in the logs.** The GUI MUST NOT persist credentials in its own state or logs (`roadmap:781`). The qualification run's captured output is scanned for credential-shaped strings; the GUI's error surfaces must carry the socket path and the start command but **no** secret material (`crates/eggsec-gui/src/connection.rs:199-215` is the existing model).
- **No secrets in the artefact.** The built binary and its metadata sidecar are scanned for credential-shaped strings and for any `.env`-style input. The build recipe refuses to run on a dirty tree unless `--allow-dirty` is passed, and the dirty flag is recorded either way.
- **The shipped binary carries no remote origin in its CSP.** The CSP is compiled into the binary from `crates/eggsec-gui/tauri.conf.json:22-24`; the negative test asserts the shipped binary's embedded CSP still admits no remote origin beyond `http://ipc.localhost`. This is the artefact-level counterpart to guard 158, which checks the source file (`scripts/check-architecture-guards.sh:4665-4718`).
- **The shipped binary's capability set is `core:default` only** — no `http`, `shell`, `fs`, `process`, or `updater` plugin (`crates/eggsec-gui/capabilities/default.json`).
- **The GUI performs no outbound HTTP of its own** (`roadmap:782-783`): the network-observation part of the launch check records no outbound connection attempt from the GUI process during a full connect → submit → cancel → render cycle.
- **The bundle stays inactive**: a negative test confirms that building the GUI crate does not produce `.deb`, `.rpm`, `.AppImage`, `.dmg`, `.app`, `.msi`, or `.exe` installer artefacts. This enforces the milestone's own boundary at the artefact level.

### Migration and compatibility tests

- **No migration** exists; the equivalent check is that the qualification matrix, the handoff artifact, and the evidence directory introduce **no** runtime state and no new persisted representation (`roadmap:739-745`).
- **Older-daemon degradation**: connecting to a daemon that answers the GUI's read-only schema command with `UnsupportedCommand` degrades to a static form set rather than failing the launch (`roadmap:749-765`). The connect step records which daemon version it ran against.
- **The `check_doc_references.py` baseline** stays at 29 missing references or lower.
- **The `deny.toml` outcome for the shipped closure** is re-verified at the milestone's commit and recorded in the handoff artifact, with the GUI-reachable advisory share named explicitly (§3.9).

---

## 11. Required verification commands

```bash
# Mandatory Rust contract (M007 must not regress it)
make check

# The GUI shell profile, ungated — the check that actually proves the shell compiles
make check-gui

# The GUI's own test suite (not part of `make check` at this baseline)
cargo test -p eggsec-gui --features shell --lib
cargo test -p eggsec-gui --no-default-features --lib

# The build recipe and its metadata
make gui-release

# The Linux prerequisite probe
bash scripts/check_gui_prereqs.sh          # proposed name; see Work package B

# Dependency policy (currently RED on inherited advisories — see §3.9)
make check-deps

# MSRV contract, including the GUI's shell profile behind the gate
make check-msrv

# Architecture guards (168 after this milestone)
bash scripts/check-architecture-guards.sh

# Documentation reference baseline (must not exceed 29)
python3 scripts/check_doc_references.py

# Formatting
cargo fmt --all --check
```

Deep checks, **only if** the milestone's changes touch the feature surface (they should not):

```bash
make check-features-individual
make check-full
```

Both are currently **blocked** by the inherited `--all-features` `E0616` failure at `crates/eggsec/src/fuzzer/advanced.rs:446` (§3.9, item 2). If a deep check is attempted, its failure must be attributed to that inherited error and not to M007.

**Not run by M007:** `make check-python` — M007 changes no Python file, and the check requires a `.venv-ci/` environment. The position is recorded in the closure record.

The closure record must report **only commands actually run**, with their observed output. The template's rule applies verbatim: "Do not claim commands that were not actually run in the closure record."

---

## 12. Documentation updates

| File | Change |
|---|---|
| `docs/GUI_PLATFORM_QUALIFICATION.md` | **New.** The qualification matrix, its cell grammar, the evidence-artifact references, the Windows gap with its root cause, the matrix's provenance header. |
| `docs/GUI_DISTRIBUTION_HANDOFF.md` | **New.** The handoff artifact: artifact identity, runtime prerequisites, what is *not* in the artefact, known limitations, supply-chain posture, the decisions the distribution workstream owns, the decisions M007 made. |
| `docs/GUI_BUILD.md` | **New** (or an equivalent section inside `docs/VERIFICATION.md`). The build recipe, the Linux prerequisites with per-distribution package names, the platform floor, the gate semantics, and the qualification runbook from Work package D. |
| `docs/evidence/gui-qualification/` | **New** directory. One evidence artefact per qualification run, plus the template. |
| `architecture/desktop_gui.md` | Extend the platform section (`:377-387`) with the platform floor, the qualified matrix reference, the `bundle.active` decision, the CI job, the distribution pointer, and the measured resident footprint. |
| `docs/VERIFICATION.md` | Extend the "Desktop GUI platform prerequisites" table (`:102-110`) and "Prerequisite-skip semantics" (`:112-132`); add `gui-release` and the prerequisite probe to the make-target reference; state that `make check` does not run the GUI's test suite. |
| `docs/PLATFORM.md` | Add a cross-reference: the GUI's platform requirements are build-time requirements, documented in the qualification document; the GUI is not one of the six runtime probing domains (`:11-14`). |
| `.opencode/skills/eggsec-gui/SKILL.md` | Align the platform/packaging section with `architecture/desktop_gui.md`. Point at `architecture/overview.md` for headline counts rather than restating them. |
| `scripts/check-architecture-guards.sh` | Add guard 168. |
| `plans/registry.md`, `plans/subsystems/desktop-gui-frontend-roadmap.md` | Updated **by the closure record**, recording M7's status and the closure record's path. Not touched by this plan. |

---

## 13. Acceptance criteria

Externally observable and contract-level. Each maps to a roadmap M7 exit condition (`roadmap:605-608`) or a cross-cutting requirement.

**Mapping to the roadmap's exit conditions:**

| # | Acceptance criterion | Roadmap clause |
|---|---|---|
| A1 | `docs/GUI_PLATFORM_QUALIFICATION.md` exists and contains the **two qualified rows (Linux, macOS)** and the **one out-of-scope row (Windows)** and one column per required behaviour (build, launch, connect, submit, cancel, render, export). Every cell's status is one of `qualified`, `partially qualified`, `not qualified`, `pending M6`, `out of scope` — no cell is blank and no cell carries an unqualified claim. | `roadmap:605-608` |
| A2 | Every `qualified` cell cites a reproducible evidence artefact (a path plus the command that regenerates it plus a date), and a reviewer can reproduce the cited result by running the cited command. | `roadmap:605-608`, `roadmap:844-846` |
| A3 | Every non-qualified cell names its **root cause** and the **escalation**, and no cell is left as an unexplained gap. | `roadmap:605-608` |
| A4 | The Linux system prerequisites are documented with the per-distribution package names for Debian/Ubuntu, Fedora, and Arch, and the supported Linux floor is stated as a documented platform floor (`webkit2gtk-4.1` ⇒ Ubuntu 22.04 / Debian 12 or newer). | `roadmap:605-608`; `ADR-0009:235-240` |
| A5 | A prerequisite probe reports the GUI shell profile as buildable or not on the executing host, and reports "not buildable" — never "skipped" — when the webview SDK is absent. | `roadmap:605-608`; `Makefile:22-29` |
| A6 | A reproducible build recipe produces, from a clean tree at a pinned commit, a per-platform release binary plus a metadata sidecar recording the commit, tree state, toolchain, target triple, feature set, `Cargo.lock` hash, size, and SHA-256 — and **no** installer, signature, or notarization. | `roadmap:598-600` |
| A7 | `crates/eggsec-gui/tauri.conf.json` keeps `bundle.active: false`, and guard 168 fails if it is changed without a superseding ADR. | `roadmap:600`, `:567-568`; `ADR-0007:88-91` |
| A8 | A CI job builds the GUI and runs its test suite on every platform the matrix claims to qualify, and the matrix's Build column cites that job. The job is not red, and any platform it omits is stated in the job's own comment and in the matrix. | `roadmap:844-846` |
| A9 | `cargo test -p eggsec-gui --features shell --lib` and `cargo test -p eggsec-gui --no-default-features --lib` both pass on every platform the matrix claims, and the GUI's 34 tests are executed in CI (they are not run by `make check` at this baseline). | `roadmap:822-823` |
| A10 | `docs/GUI_DISTRIBUTION_HANDOFF.md` contains every required section, each naming something concrete; it states the **decided** supported-platform matrix — Linux and macOS qualified, Windows out of scope — and names `ADR-0010` as the decision that settles it, while stating that the **distribution** policy (signing, notarization, installers, update channels) remains the distribution workstream's (the operational dependency named at `roadmap:595-597`). | `roadmap:598-599`, `:552-554` |
| A11 | The known-limitations record is an explicit numbered list and includes, at minimum, the Windows transport gap, the duplicated socket-path default, the unwired `ApprovePolicy`, the absent daemon lifecycle management, the Linux platform floor, the `make check-deps` advisory failure with the GUI-reachable share, and the `--all-features` compile error. | `roadmap:605-608` |
| A12 | The Windows exclusion is stated as an **out-of-scope decision with its root cause and its re-entry path** — the unconditional `tokio::net::UnixStream` / `tokio::net::UnixListener` / `std::os::unix::fs::PermissionsExt` imports at `crates/eggsec-frontend-client/src/daemon.rs:9`, `crates/eggsec-daemon/src/server.rs:1`, `:5` — and **not** papered over by marking the Windows row qualified. | `roadmap:605-608` |
| A13 | The handoff artifact states that nothing in the workspace installs `eggsec-daemon` and that the GUI binary does not ship a daemon, so the distribution workstream's daemon-delivery obligation is explicit. | `roadmap:598-599` |
| A14 | The GUI's resident footprint is measured on each qualified platform and recorded in the handoff artifact. | `roadmap:803-805` |
| A15 | `bash scripts/check-architecture-guards.sh` ends with `ALL PASSED: No architecture drift detected.`, including the new check 168, which was negative-tested (a deliberate violation failed with a named reason, and the restore passed). | `roadmap:829-836` |
| A16 | `make check` shows no new failure relative to the inherited baseline, and every step of it that passes before `check-deps` still passes. | `roadmap:822-823` |
| A17 | `make check-msrv` shows no new failure, including the GUI's shell profile behind the gate. | `roadmap:824-825` |
| A18 | `python3 scripts/check_doc_references.py` reports 29 missing references or fewer. | repo baseline |
| A19 | `cargo fmt --all --check` is clean. | `roadmap:822` |
| A20 | The GUI remains a leaf application with no engine dependency, imports no enforcement internals, carries `core:default` capabilities only, and its CSP still admits no remote origin beyond `http://ipc.localhost`. | `roadmap:769-783`; guards 154–158 |
| A21 | The frontend still has **zero** npm dependencies, and the produced binary's embedded CSP contains no remote origin. | `roadmap:866-869`; `crates/eggsec-gui/tauri.conf.json:22-24` |
| A22 | The closure record names the distribution workstream's sign-off on the supported-platform matrix as a **precondition for M7's closure**, and states that M7 does not close while it is outstanding — unless the distribution workstream has already recorded that matrix. | `roadmap:595-597`, `:562-565` |

**Preconditions that are explicitly *not* acceptance criteria** (they are inherited and recorded, not fixed):

- `make check-deps` passing. The advisory failure is inherited; M007 records it and reports the GUI-reachable share. Admitting the exceptions is a maintainer decision (`roadmap:826-828`).
- `cargo check --workspace --all-features` passing. The `E0616` failure is inherited and outside this milestone.
- Windows building. That requires a daemon-transport milestone; M007 records the gap.

---

## 14. Stop conditions

The implementer MUST stop and report rather than improvise when any of these occurs.

1. **A platform where the GUI cannot build, and no pre-sanctioned fallback applies.** The escalation path is pre-sanctioned: `ADR-0009:242-246` — "If any of the three conditions above cannot be met, the GUI shell is implemented with egui instead, and the fallback is exercised behind the same `ADR-0007` boundary. The fallback is **sanctioned in advance** so that a switch is a contained change to `crates/eggsec-gui` rather than a re-plan." Switching to egui is a **major** change and is **not** an in-milestone decision: the implementer stops and reports, and the switch is taken as a separate recorded decision. `egui` 0.33.0 declares MSRV 1.88, below the workspace's 1.89 floor (`ADR-0009:90`, `:94-98`).
2. **The daemon transport does not exist on a platform.** This is **already true for Windows at the baseline** (§3.3), and `ADR-0010` decides that Windows is out of scope for the desktop GUI. The implementer MUST NOT add a Windows transport to make the matrix green: that is a daemon-subsystem change and a candidate for the ADR the roadmap requires for any non-additive protocol change (`roadmap:864`). Stop, record the exclusion with its root cause and its re-entry path, and escalate it in the handoff artifact.
3. **The inherited `make check-deps` failure changes shape** — a *new* advisory appears that is reachable only through the GUI's closure, or the existing GUI-reachable advisories (`glib` RUSTSEC-2024-0429, `glib-macros` RUSTSEC-2024-0370, `unic-*` RUSTSEC-2025-0081/0075/0080/0100/0098) get worse. M007 reports; it does not write the `deny.toml` exception, which needs an owner and a review-by date (`roadmap:826-828`; `docs/DEPENDENCY_EXCEPTIONS.md`). The implementer stops if the fix would require a Tauri pin change, because the 2.11.x pin is what keeps MSRV 1.89 true (`ADR-0009:218-224`).
4. **The `--all-features` compile error blocks `make check-features-individual` or `make check-full`.** The implementer MUST NOT fix `crates/eggsec/src/fuzzer/advanced.rs:446` as part of M007 — it is a knowledge-corpus crate's field visibility, already recorded as a deliberate non-fix (`plans/closure/desktop-gui-frontend/003-desktop-gui-surface-parity-closure.md:273`). Stop and report if a deep check is required for M007's acceptance.
5. **The scope widens into signing, notarization, installers, or update channels.** These belong to the separately-managed distribution workstream (`roadmap:600`, `:567-568`; `ADR-0007:88-91`, `:249-251`). Enabling `bundle.active`, adding a code-signing identity, adding an updater plugin, or producing an installer is a **stop**, not a judgement call — M007's deliverable is the binary, the recipe, the prerequisites, the matrix, and the handoff artifact.
6. **The supported-platform matrix cannot be decided.** The decision belongs to the distribution workstream (the operational dependency at `roadmap:595-597`). If the distribution workstream's position is unavailable, M007 delivers the evidence and the **recommendation**, records the decision as outstanding, and the closure names the sign-off as a precondition (A22) rather than inventing a decision.
7. **M6 has not closed and the export cell is needed for the milestone's closure.** The export cell stays `pending M6`; it does not block the rest of the matrix, and M007 does not implement export.
8. **A qualification procedure reveals a runtime defect in the GUI.** A defect found by the procedure is reported as a **separately-scoped finding** with its reproduction, not fixed inside the packaging milestone. Fixing it in M007 would silently widen a packaging milestone into a runtime milestone.
9. **The prerequisite documentation would require a package name that cannot be supported.** The package names are stated **as the Tauri v2 documentation states them**; the implementer MUST NOT invent or guess a package name for a distribution the upstream documentation does not cover. For an uncovered distribution, the document says so and points at the egui fallback.
10. **The CI matrix job would be permanently red.** A job that cannot be green on any of its legs is not added. Excluding a platform is a legitimate outcome provided the exclusion is stated in the job and in the matrix.

---

## 15. Closure evidence required

The closure record (`plans/closure/desktop-gui-frontend/007-packaging-and-platform-qualification-closure.md`) MUST contain:

1. **The qualification matrix as delivered**, with every cell's status and every evidence reference, plus the matrix's header commit and date.
2. **The evidence artefacts** under `docs/evidence/gui-qualification/`, one per qualification run, each with: platform, host OS and architecture, commit SHA, toolchain, feature set, the exact commands run, the observed output, and the operator or CI job that produced it.
3. **The cross-compilation result for Windows** — the exact `cargo check -p eggsec-gui --no-default-features --target x86_64-pc-windows-msvc` error output and the two unconditional unix-only imports that cause it, so the gap is auditable rather than asserted.
4. **The CI run URL** for the `gui-platform` job on each qualified platform, with the job's matrix and the commands it ran.
5. **The build metadata sidecar** from `make gui-release` for each qualified platform, including the binary's SHA-256 and the `Cargo.lock` hash.
6. **The Linux prerequisite probe's output** on a host with the prerequisites and, if available, on one without — showing the "not buildable" verdict rather than a silent skip.
7. **The resident-footprint measurement** per qualified platform, with the method used.
8. **The handoff artifact** `docs/GUI_DISTRIBUTION_HANDOFF.md` as delivered, including the known-limitations list and the **decided** supported-platform matrix (Linux and macOS qualified, Windows out of scope per `ADR-0010`).
9. **The distribution workstream's position** on the supported-platform matrix — either the recorded decision, or an explicit statement that the sign-off is outstanding and that M7's closure is conditional on it (A22).
10. **The guard's negative test** — the deliberate violation, the guard's failure output naming the reason, and the restore run's pass.
11. **The verification commands actually run, with their observed output** — §11's list, with the exact command and the result. Commands that were **not** run are listed as not run, with the reason. No claim may appear for a command that was not executed.
12. **The inherited failures**, restated with their current output and their attribution: the `make check-deps` advisory failure (with the GUI-reachable share enumerated), the `--all-features` `E0616` error, and `make check-python` not run.
13. **The `check_doc_references.py` count** (29 or lower) and the `cargo fmt --all --check` result.
14. **The roadmap disposition** — which M7 exit conditions are met, which are met with a recorded gap, and which are not met, stated plainly. A milestone that closes with an unmet exit condition (Windows) MUST say so in the roadmap's status table.
15. **The stashes and preserved state** — confirmation that the 13 pre-existing stashes were untouched, that `crates/eggsec-gui/gen/schemas/` stayed git-ignored, and that `crates/eggsec-gui/icons/icon.png` is still present.

---

## 16. Handoff notes

### Hazards

- **The 13 pre-existing stashes MUST be preserved untouched.** `git stash list` reports 13 at this baseline. M007 MUST NOT pop, drop, apply, or reorder them, and MUST NOT run `git stash` itself.
- **`crates/eggsec-gui/icons/icon.png` MUST be kept.** Tauri's `tauri::generate_context!` panics without it ("failed to open icon"). The file is a generated 512×512 RGBA PNG (4 379 bytes). It is required at build time; it is not decoration and must not be deleted, moved, or "cleaned up" during a packaging cleanup.
- **`crates/eggsec-gui/gen/schemas/` is git-ignored** (`.gitignore:10`, pattern `crates/*/gen/schemas/`). `tauri-build` regenerates it on every build, and `capabilities/default.json`'s `$schema` points into it only as an editor hint. Do not force-add it; do not delete it and commit the absence.
- **Tauri's command arguments default to camelCase.** Every GUI command therefore declares `#[tauri::command(rename_all = "snake_case")]` — all eight of them (`crates/eggsec-gui/src/commands.rs:218`, `:265`, `:291`, `:338`, `:374`, `:435`, `:451`, `:463`). The in-crate test at `:641-655` fails if any command omits it. Any command added during M007 (none is planned) MUST declare it.
- **The frontend has zero npm dependencies and that must stay true.** There is no `package.json`, no lockfile, no `node_modules`, and no bundler under `crates/eggsec-gui/frontend/`. Any JavaScript dependency admitted later requires a recorded policy with an owner and a review-by date (`roadmap:866-869`). The build recipe MUST NOT introduce an npm step.
- **`cargo fmt --all` before committing.** The mandatory contract starts with `cargo fmt --all --check` (`Makefile:155`).

### Resource constraints and environment

- **Other crates are busy in this workspace.** M007's implementer should prefer `cargo check -p eggsec-gui` / `cargo test -p eggsec-gui` over workspace-wide builds, and MUST NOT run a workspace-wide `--all-features` build as a routine step — it is currently broken anyway (§3.9, item 2).
- **The Linux shell profile needs the webview SDK.** Without it, `make check`'s GUI line **skips** rather than fails (`Makefile:22-29`). On Linux, always use `make check-gui` for the shell profile; a green `make check` on a bare Linux host proves nothing about the GUI.
- **The GUI's tests need no display, no socket, and no daemon** — verified by search: no `UnixStream`, `UnixListener`, `TcpStream`, `tauri::test`, `WebviewWindow`, or `std::process::Command` appears under `crates/eggsec-gui/src/`. They are safe to run unattended in CI.
- **The `gui-platform` CI job needs a macOS runner** to cover the second qualified platform; the Linux leg alone does not discharge the roadmap's platform-qualification requirement. No Windows leg is required or claimed — `ADR-0010` puts Windows out of scope.
- **Cross-compilation for the Windows evidence** requires the `x86_64-pc-windows-msvc` target (`rustup target add x86_64-pc-windows-msvc`). A `cargo check` against it needs no linker; a *build* of the daemon additionally needs a cross C toolchain for `libsqlite3-sys`, which is why the daemon's own Windows check stopped in the build script during verification.

### Serial-test requirements

- The GUI's test suite is a single crate with 34 tests and no shared mutable state; it runs serially by default and needs no `--test-threads` tuning.
- The daemon round-trip test (`cargo test -p eggsec-daemon --lib the_gui_client_path_works_end_to_end`) binds a Unix socket; if a qualification run holds `/tmp/eggsec-daemon.sock`, that test must use a distinct socket path.
- The qualification procedure binds a socket for the daemon; use a **per-run** socket path (e.g. under `$TMPDIR`) rather than the default `/tmp/eggsec-daemon.sock`, so a run never collides with a developer's live daemon and so two concurrent runs do not collide with each other.

### Preserved user changes

- The 13 stashes are preserved untouched (see Hazards).
- The worktree is a dedicated feature branch (`feat/auto-20261022-76692679`); M007's commits land on it.

### Scope discipline for the implementer

- The deliverable is **one milestone's worth of work**: the matrix, the prerequisites doc, the build recipe, the CI job, the procedure, the handoff artifact, the known-limitations record, the documentation updates, and one new guard.
- It is **not** a distribution milestone. The moment the work touches signing, notarization, an installer format, an updater, or the `bundle.active` flag, stop and report (§14, condition 5).
- It is **not** a daemon milestone. The Windows transport gap is recorded and escalated, not fixed here (§14, condition 2).
