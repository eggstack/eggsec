# Desktop GUI: distribution handoff

This is the hand-off note from milestone 007 (packaging and platform
qualification) to the distribution workstream. It says exactly what the
milestone produced, what is in it, what is **not** in it, and which decisions
are still open.

It is written for the person who will ship the binary, so it leads with the
artefact's identity and ends with the decisions they own.

- The qualification matrix (what was actually exercised, and on which
  platform): [`docs/GUI_PLATFORM_QUALIFICATION.md`](GUI_PLATFORM_QUALIFICATION.md).
- The build recipe, the platform prerequisites, and the operator runbook:
  [`docs/GUI_BUILD.md`](GUI_BUILD.md).

---

## 1. Artifact identity

Milestone 007 produces a **binary plus a build-metadata sidecar**. The
sidecar is not documentation; it is the machine-readable half of the
artefact's identity, and it is what makes two binaries comparable.

| Field | Value |
|---|---|
| Binary name | `eggsec-gui` |
| Platform / architecture (measured) | macOS 25.6.0, `aarch64-apple-darwin`, `Mach-O 64-bit executable arm64` |
| Platform / architecture (CI leg) | `x86_64-unknown-linux-gnu` on `ubuntu-latest` |
| Commit SHA | `7332baeafe5391e228a0b91fec48cf68999729f1` |
| Tree state | **dirty** — the recorded build was produced with `--allow-dirty` because milestone 007's own documents and scripts were uncommitted and two other milestones were editing the tree concurrently. The sidecar records `tree_state: "dirty"`, `dirty: true`, and `allow_dirty: true`. **A release-grade binary must come from a clean tree**; the `gui-platform` CI job is that path. |
| Toolchain | `rustc 1.99.1 (58cf8f9c3 2026-10-13)`, LLVM 23.1.1; `cargo 1.99.1 (5f94df478 2026-08-27)` |
| Enabled cargo features | requested `shell`; resolved `default`, `shell` |
| Build invocation | `cargo build --release -p eggsec-gui --features shell --locked` |
| `Cargo.lock` SHA-256 | `c34ff1b8d3136139b5e3f3b45c23fedc0a232fdc612f38612b2ce85a412c16c3` |
| Binary SHA-256 | `22b44fc0f087994ec870b0b0fea45a548b5e4780b602156c2fecf83ae4253481` |
| Binary size | 5 747 424 bytes (5.48 MiB) |
| Build metadata sidecar | `dist/eggsec-gui.build.json` (schema `eggsec-gui-build-metadata/v1`) |
| Checksum file | `dist/SHA256SUMS` |

The build recipe that produces all of the above is
`scripts/build_gui_release.sh`; it is documented in
[`docs/GUI_BUILD.md`](GUI_BUILD.md) §3. The full recorded run is
[`docs/evidence/gui-qualification/macos-2026-10-23.md`](evidence/gui-qualification/macos-2026-10-23.md).

**What the export procedure produces.** The GUI's export path
(`crates/eggsec-gui/src/export.rs`) writes the **same artefacts the CLI
writes**, through the same `eggsec-output` renderers, in the six formats
`json`, `csv`, `html`, `md`, `sarif`, and `xml` (JUnit). A distribution
packaging the GUI should expect those six files beside a report and no
seventh: the PDF renderer lives in the engine, and the GUI links no engine
crate. The procedure that exercises the path is `docs/GUI_BUILD.md` §5.7,
and the recorded run — with each exported file's digest — is in
[`docs/evidence/gui-qualification/macos-2026-10-24.md`](evidence/gui-qualification/macos-2026-10-24.md).

> **The SHA-256 identifies an artefact, not a commit.** A second run of the
> same command later the same day, against the then-current tree, produced
> SHA-256 `513caadf4613b3dbb1e65d998d9e03f6c9aa4cfca7058a3470ea0dbb7c3c0fbb`
> — the same size, the same commit, and the same `Cargo.lock` hash, but a
> different binary, because another milestone had edited a crate in the GUI's
> dependency closure while the tree was dirty. A receipt that carries only a
> commit SHA would have called those two artefacts identical. Verify the
> **whole sidecar**, not the hash alone.


---

## 2. Runtime prerequisites

### 2.1 The GUI binary's own prerequisites

The GUI is a Tauri v2 shell, so its build prerequisites are the Tauri
v2 system-dependency lists. In full, with the per-distribution package
names and the reasoning behind the platform floor:
[`docs/GUI_BUILD.md`](GUI_BUILD.md) §2.

Condensed:

| Platform | Packages |
|---|---|
| Debian / Ubuntu | `libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev` |
| Fedora | `webkit2gtk4.1-devel openssl-devel curl wget file libappindicator-gtk3-devel librsvg2-devel libxdo-devel` plus the `Development Tools` (c-development) group |
| Arch | `webkit2gtk-4.1 base-devel curl wget file openssl appmenu-gtk-module libappindicator-gtk3 librsvg xdotool` |
| macOS | Xcode, or `xcode-select --install` for desktop-only development. The webview ships with the OS as WKWebView. |

### 2.2 The daemon the binary expects

The GUI is **daemon-only** (`ADR-0007`): it reaches the engine only through
the daemon's Unix-domain-socket transport, it links no engine crate, and it
has no embedded-runtime path. A GUI with no daemon is inert.

The binary expects:

- a daemon listening on a Unix domain socket, at
  `/tmp/eggsec-daemon.sock` by default, or wherever the operator points it
  with the GUI's `--socket <path>` flag;
- a daemon built **with** the `service` cargo feature (or its deprecated
  `full-executor` alias), otherwise `--full-executor` is accepted but the
  daemon falls back to a no-op executor and warns
  (`crates/eggsec-daemon/src/main.rs:40-46`).

The daemon's own engine-feature profile and obtainability recipe are
milestone 008's and milestone 009's
([`docs/DAEMON_FEATURE_PROFILE.md`](DAEMON_FEATURE_PROFILE.md),
[`docs/DAEMON.md`](DAEMON.md)). They are not restated here.

---

## 3. What is NOT in the artefact

The following are **absent by decision**, not by oversight. A recipient who
expects them will find them missing, and this section is why.

- **No installer.** The Tauri bundler is inactive
  (`bundle.active: false`, `crates/eggsec-gui/tauri.conf.json:26-28`), so
  the build emits a raw binary and nothing else. The build recipe asserts
  this: after every build it scans the output directory for `.deb`, `.rpm`,
  `.AppImage`, `.dmg`, `.app`, `.msi`, and `.exe` artefacts and fails if it
  finds one.
- **No signature.** Nothing in the build recipe signs the binary. The
  sidecar records `signed: false`.
- **No notarization.** Same: `notarized: false`.
- **No update channel.** The GUI does not self-update and performs no
  unsolicited network fetch. `ADR-0009` requires that update checks are not
  enabled by default.
- **No bundled daemon.** **The GUI binary does not ship the daemon.**
  `cargo install --path crates/eggsec-cli` installs exactly one binary, named
  `eggsec` (`crates/eggsec-cli/Cargo.toml:11-13`); the `eggsec-daemon` crate
  is consumed by the workspace as a *library* (an optional dependency of
  `eggsec-cli` and `eggsec-python`), and nothing in the workspace installs
  `eggsec-daemon` as a binary.

  **The distribution workstream must therefore deliver the daemon as a
  separate binary** — or, explicitly and by name, record that the GUI is
  expected to sit alongside a separately-managed daemon. Which of the two it
  is, the distribution workstream decides; what it cannot do is ship the GUI
  alone and leave the daemon implicit.

---

## 4. Known limitations

1. **Windows is out of scope for the desktop GUI, by decision.**
   [`ADR-0010`](../plans/adrs/ADR-0010-gui-supported-platform-scope.md)
   fixes the desktop GUI's supported platforms at Linux and macOS. The root
   cause is measured, not inferred: the GUI's **unconditional** dependency
   `eggsec-frontend-client` imports the Unix-domain-socket transport without
   a `cfg(unix)` gate —
   `crates/eggsec-frontend-client/src/daemon.rs:9`
   (`use tokio::net::UnixStream;`) and `:56`
   (`writer: Mutex<tokio::net::unix::OwnedWriteHalf>`) — and the daemon side
   is equally unconditional
   (`crates/eggsec-daemon/src/server.rs:5`, `:95`). Cross-compiling fails:

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
   `eggsec-frontend-client` is unconditional
   (`crates/eggsec-gui/Cargo.toml:38`).

   **The re-entry path**, as `ADR-0010` records it: a Windows daemon
   transport (named pipes or an equivalent local IPC) is a **daemon-subsystem**
   change. It changes the *kind* of transport rather than adding a read
   path, so it requires an ADR of its own. Milestone 007 names that path; it
   does not implement it, and it deliberately does not add a Windows
   transport to make a matrix cell green.

2. **The transport is a Unix-domain socket at a default that one crate owns,
   with one environment-variable override and no XDG lookup.** The built-in
   default is `/tmp/eggsec-daemon.sock`, and it is stated **once**, in
   `crates/eggsec-runtime/src/daemon_endpoint.rs`. Milestone 012
   (`ADR-0013`) collapsed the six production copies into that one owner and
   added the override `EGGSEC_DAEMON_SOCKET`, with the precedence **explicit
   flag → environment → built-in default** applied by the same resolver in
   the daemon binary, the CLI, the TUI and the GUI. The GUI's webview holds
   no copy of the default at all: it learns the path from Rust through the
   `daemon_status` command. There is still no `EGGSEC_SOCKET` environment
   variable and no `XDG_RUNTIME_DIR` lookup anywhere in the workspace, so a
   per-user socket location is expressible only through the environment
   variable. For a distribution, a default path in `/tmp` is a multi-user-host
   concern that the distribution workstream should decide about explicitly.

3. **The GUI does not manage the daemon's lifecycle.** It does not start,
   supervise, or stop the daemon; it connects to one that is already
   running. That is an architectural boundary, not an open milestone: the
   GUI is a daemon-only frontend (`ADR-0007`), and there is no milestone
   left to close here. The operator starts the daemon by hand, or points the
   GUI at one the host's service manager already runs.

4. **`ApprovePolicy` is now wired** (milestone 011, `ADR-0012`). The daemon
   resolves the runtime's pending decision instead of answering
   `ErrorCode::Unsupported`. The GUI marks a task `awaiting_policy`, shows the
   policy's own prompt, and offers **Approve** and **Reject** — and it still
   **must not auto-approve**, because an auto-approval would be the GUI
   deciding an authorisation question that only the daemon's enforcement layer
   may decide. The answer bounds itself: it permits exactly the confirmation
   classes the policy kernel demanded, and a strict surface never holds a
   decision at all.

5. **The GUI's resident footprint was not previously measured; it is now
   measured on one platform only.** The measurement (macOS 25.6.0,
   `aarch64-apple-darwin`, at rest with the webview loaded) is a sum of RSS
   across four processes — the application plus the three WKWebView XPC
   helpers:

   | Process | RSS (KiB) |
   |---|---|
   | `eggsec-gui` (application) | 79 760 |
   | `com.apple.WebKit.GPU` | 18 512 |
   | `com.apple.WebKit.Networking` | 7 616 |
   | `com.apple.WebKit.WebContent` | 19 872 |
   | **Sum** | **125 760 (122.8 MiB)** |

   The application process peaked at 105 344 KiB shortly after start-up. The
   sum double-counts the WebKit framework pages shared between the helpers,
   so it is an upper bound rather than a private-footprint figure. **There is
   no Linux measurement**, because no Linux host was executed during this
   milestone; and the measurement is of an idle window, not of a running
   assessment. `ADR-0009` had already flagged webview-backed shells as
   costing roughly 208–211 MiB peak RSS in prior art, so the operator should
   treat this figure as a floor, not a ceiling.

6. **`make check-deps` currently fails on advisories, and part of that
   failure is GUI-reachable.** The dependency policy gate
   (`cargo deny --workspace --all-features check`) fails closed. As of
   2026-10-23 the advisories check reports these findings:

   | Advisory | Crate | GUI-reachable? |
   |---|---|---|
   | RUSTSEC-2026-0386 (unsound) | `tokio` 1.53.1 | **yes**, on every target — the GUI's transport client uses Tokio |
   | RUSTSEC-2024-0429 (unsound) | `glib` 0.18.5 | **yes on Linux only** — the webview's GTK binding |
   | RUSTSEC-2024-0370 (unmaintained) | `proc-macro-error` 1.0.4 | **yes on Linux only**, and build-time only |
   | RUSTSEC-2025-0075 / -0080 / -0081 / -0098 / -0100 (unmaintained) | `unic-char-range`, `unic-common`, `unic-char-property`, `unic-ucd-version`, `unic-ucd-ident` | **yes**, on every target — reached through `idna` |
   | RUSTSEC-2026-0385 (vulnerability) | `ctutils` 0.4.2 | no |
   | RUSTSEC-2026-0346 / -0347 / -0348 / -0349 (vulnerability) | `rustls` 0.21.12 and 0.23.45 | no |
   | RUSTSEC-2026-0382 (vulnerability) | `rustls-webpki` 0.101.7 and 0.103.15 | no |
   | RUSTSEC-2026-0381 / -0383 (vulnerability) | `webpki-roots`, `webpki-root-certs` | no |
   | RUSTSEC-2024-0449 (unmaintained) | `unsafe-libyaml` | no |
   | RUSTSEC-2026-0368 / -0369 (vulnerability) | `zip` 2.4.2 | no — reached through the engine, not the GUI |

   The absence of `rustls` from the GUI's closure is deliberate and is worth
   noting in the positive: the GUI introduces **no second TLS stack**, which
   is a condition of `ADR-0009`. The GUI's entire network surface is the
   local Unix socket.

   **Recording the exceptions is a maintainer decision.** Each exception
   needs an owner and a review-by date, recorded in `deny.toml` **and**
   `docs/DEPENDENCY_EXCEPTIONS.md`. Milestone 007 does not write those
   entries — it is not its decision to make, and a distribution that ships a
   binary with unrecorded advisories is worse than one that ships a binary
   with recorded ones.

7. **The Linux platform floor excludes older distributions.**
   `webkit2gtk-4.1` implies Ubuntu 22.04 / Debian 12 or newer
   (`ADR-0009`). Ubuntu 20.04 and earlier, Debian 11 and earlier, and
   enterprise rebuilds frozen before that generation are therefore **below
   the floor**, as is any distribution carrying only a `webkit2gtk-4.0`
   webview. The workspace's own gate is looser than the build
   (`Makefile:21-30` accepts 4.1 **or** 4.0), so a 4.0-only host can pass
   the gate and then fail to compile. An operator below the floor has two
   options: move to a supported distribution, or take the `ADR-0009` egui
   fallback, which is pre-sanctioned for exactly this case.

### 4.1 Resolved during this milestone

- The `--all-features` `E0616` compile error named in milestone 007's plan
  is **resolved**: `crates/eggsec-payloads`'s `WebSocketFuzzer` now exposes
  its target URL through a `pub fn url(&self) -> &str` accessor
  (commit `7332baea`, "fix(payloads): expose WebSocketFuzzer's target url to
  the engine"). It is **not** an open limitation and is listed here only so
  that a reader who saw it in the plan does not go looking for it.

---

## 5. Supply-chain posture

**Policy.** `deny.toml` is the canonical dependency policy, and
`make check-deps` fails closed when `cargo-deny` is absent. The GUI
introduced **no new dependency** in this milestone, and no entry was added
to `deny.toml` or to `docs/DEPENDENCY_EXCEPTIONS.md`.

**Outcome for the shipped closure.** As recorded in §4.6, the advisories
check currently fails. The GUI-reachable findings are `tokio`
(RUSTSEC-2026-0386), the five unmaintained `unic-*` crates, and — on Linux
only — `glib` (RUSTSEC-2024-0429) and the build-time-only
`proc-macro-error` (RUSTSEC-2024-0370). `licenses`, `bans`, and `sources`
are green.

**What the distribution workstream must know.** The gate is red. Shipping
the GUI binary does not make it redder, and does not make it greener either
— but a distribution that claims a clean `make check-deps` would be making a
false claim, and should not.

**The exception record is a maintainer decision.** Each advisory that is
accepted needs an entry in `deny.toml` and a matching record in
`docs/DEPENDENCY_EXCEPTIONS.md` carrying the advisory ID, the path, the
feature, the API use, the exploitability assessment, the compensating
control, an **owner**, a created date, a **review-by date**, and the
blocker. Milestone 007 deliberately writes none of them: the decision to
accept an advisory is a risk decision, and it belongs to the maintainers,
not to the milestone that measured the closure.

---

## 6. Decisions the distribution workstream owns

These are open, and they are the distribution workstream's to make:

- **Signing.** What identity signs the binary, and how a consumer verifies
  it.
- **Notarization.** Whether, and for which platforms, the binary is
  notarized.
- **Installer format.** Whether to enable the Tauri bundler
  (`bundle.active`) and, if so, which formats. Milestone 007 deliberately
  leaves the bundler inactive (§7).
- **Update channels.** Whether the GUI gains an update mechanism, and how it
  is triggered. `ADR-0009` requires that it is never enabled by default.
- **Distribution policy.** Where the binary comes from, how it is delivered,
  and how it is kept current.

**Not the distribution workstream's to decide:** the supported-platform
scope. `ADR-0010` has already fixed the desktop GUI's supported platforms at
**Linux and macOS**, with Windows out of scope. That decision is made, and
re-opening it is an ADR question, not a packaging question. The
distribution workstream's job is to package the two supported platforms and
to state the Windows exclusion in its own materials.

---

## 7. Decisions M007 made

1. **`bundle.active` stays `false`.** The Tauri bundler remains inactive
   (`crates/eggsec-gui/tauri.conf.json:26-28`), and the build recipe asserts
   that no installer artefact is produced. The reasoning is the milestone's
   own boundary: the roadmap and `ADR-0007` place installers, signing, and
   notarization in the separate distribution workstream, and enabling the
   bundler would make a plain `make check` emit installers as a side effect.
   Milestone 007 produces the **binary** and the **reproducible build
   recipe**; the distribution workstream inherits the binary and decides
   whether to enable the bundler or to ship a raw artefact.

2. **The build recipe is `scripts/build_gui_release.sh`, with a build
   metadata sidecar.** It refuses a dirty tree unless `--allow-dirty` is
   passed, records the tree state either way, uses the pinned invocation
   `cargo build --release -p eggsec-gui --features shell --locked`, and emits
   the binary beside `eggsec-gui.build.json` and `SHA256SUMS`. It signs
   nothing, notarizes nothing, bundles nothing, publishes nothing, and has
   no npm step.

3. **The supported-platform recommendation is Linux and macOS, with an
   explicit Windows exclusion**, implementing `ADR-0010` rather than
   re-deciding it. The recommendation is accompanied by the measured reason
   (the unconditional Unix-only imports and the cross-compilation failure
   in §4.1), the CI matrix that exercises it
   (`gui-platform` on `ubuntu-latest` and `macos-latest`), and the named
   re-entry path for Windows.

4. **The qualification is a matrix with an evidence mechanism**, not a
   prose claim: [`docs/GUI_PLATFORM_QUALIFICATION.md`](GUI_PLATFORM_QUALIFICATION.md),
   with one evidence file per run under `docs/evidence/gui-qualification/`
   and a static guard (check 168) that enforces the cell grammar.
