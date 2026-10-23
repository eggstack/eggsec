# ADR-0010: Desktop GUI supported-platform scope

Status: accepted

Date: 2026-10-23

Decision owners: project maintainers

Related specification sections:

- `plans/000-long-term-specification.md#10-frontend-rules`
- `plans/000-long-term-specification.md#11-acceptance-criteria`
- `plans/001-terminology-and-domain-model.md#3-domain-model`

Affected subsystem roadmaps:

- `plans/subsystems/desktop-gui-frontend-roadmap.md`

## Context

The desktop GUI reaches the engine **only** through the daemon, over the
daemon's local transport. That is a deliberate property of `ADR-0007`, and it
means the GUI's platform reach is exactly the daemon transport's platform reach.

The daemon transport is a Unix domain socket, and it is Unix-only **by
construction, not by policy**:

- `crates/eggsec-daemon/src/server.rs:1` imports `std::os::unix::fs::PermissionsExt`
  unconditionally, and `crates/eggsec-daemon/src/server.rs:5` imports
  `tokio::net::UnixListener`;
- `crates/eggsec-daemon/src/client.rs:9` and
  `crates/eggsec-frontend-client/src/daemon.rs:9` import
  `tokio::net::UnixStream` unconditionally.

There is no `cfg(unix)` gate on any of those uses, so the daemon and the
frontend client do not compile for Windows. The GUI therefore inherits the
limitation: `cargo check -p eggsec-gui --no-default-features --target
x86_64-pc-windows-msvc` fails with `error[E0432]: unresolved import
tokio::net::UnixStream`.

This became a governance problem rather than a build problem. The subsystem
roadmap says "the supported platform matrix" in five places
(`plans/subsystems/desktop-gui-frontend-roadmap.md:399`, `:514`, `:519`,
`:526`, `:646`) and **never defines it**; the definition is deferred to
"distribution policy (operational)" — a decision owned outside this repository.
Milestone 007's exit condition is therefore written as "each supported platform
builds and launches", which is **not verifiable** while the set is undefined,
and would be **unsatisfiable** if Windows were included.

The rest of the product is unaffected. The engine, CLI, TUI, and Python
surfaces build and run on Windows today, and the portability CI matrix
(`.github/workflows/deep-checks.yml:61-66`) covers `windows-latest` for those.
Only the desktop GUI is in question.

The maintainer directed this decision on 2026-10-23: keep Windows out of scope
for the desktop GUI.

## Decision drivers

- **The GUI's transport is the daemon's transport.** Adding a Windows transport
  to make the GUI work on Windows would change the daemon's transport *kind*
  for **every** frontend (TUI, CLI, Python), not for the GUI alone. That is a
  far larger blast radius than the GUI's platform reach, and it would not be a
  strictly additive protocol change.
- **Milestone 007 needs a verifiable exit condition.** "Each supported
  platform builds and launches" is only a checkable claim once the set is
  named, and a claim the project cannot currently satisfy on Windows.
- **Honest prerequisites beat aspirational matrices.** The Linux
  `webkit2gtk-4.1` floor is already a documented prerequisite
  (`docs/PLATFORM.md`); the platform story should name what the project
  supports and what it does not, rather than claim a platform to satisfy a
  matrix.
- **Deferring is cheap; forcing a transport now is not.** The GUI's Unix-socket
  transport inherits filesystem-scoped access control (the socket is `chmod`
  `0600` at `crates/eggsec-daemon/src/server.rs:100-101`), which is a stronger
  local-access guarantee than a TCP transport would give by default.
- **`ADR-0009`'s egui fallback does not apply.** The fallback exists to remove
  the *system webview* dependency. The Windows obstacle is the *transport*, not
  the shell, so no shell choice fixes it.

## Considered options

### Option A — Qualify Linux and macOS; hold Windows out of scope explicitly

Description: the desktop GUI's supported-platform set is Linux (x86_64 glibc)
and macOS (x86_64 and arm64), qualified through milestone 007. Windows is
named as out of scope, with its root cause recorded, and re-entry is gated on
a daemon-transport milestone.

Benefits: milestone 007 gets a checkable exit condition; no daemon transport
change is forced; the platform claim is truthful; the GUI's stronger local
access model is preserved unchanged.

Costs: Windows users keep the CLI, TUI, and Python but not the desktop GUI, so
the "one product, many surfaces" story has a platform seam. The GUI's release
footprint is narrower than the rest of the product's.

Failure modes: the GUI is quietly expected on Windows by users or downstream
packagers — mitigated by stating the exclusion in the long-term
specification, the subsystem roadmap, the registry, and milestone 007's
known-limitations record.

### Option B — Add a Windows named-pipe transport now

Description: extend the daemon with a named-pipe transport and a transport
selection handshake, so the GUI and every other frontend work on Windows.

Benefits: one product surface everywhere; no platform seam.

Costs: it changes the daemon's transport kind for every frontend; it is not a
strictly additive protocol change; it multiplies the daemon's test matrix; and
the local-access story changes from filesystem permissions to whatever the pipe
or loopback mechanism provides.

Failure modes: a second transport kind silently diverges between platforms —
the kind of divergence this subsystem's additive-only protocol discipline
exists to prevent.

### Option C — Leave the set undefined and let the distribution workstream decide

Description: keep the current roadmap wording and treat the platform set as an
external operational decision.

Benefits: no new decision; the distribution workstream retains full discretion.

Costs: milestone 007 cannot close, because its exit condition is undecidable
while the set is undefined. It also leaves the GUI's platform reach ambiguous
in every document that mentions it.

Failure modes: the milestone stalls indefinitely on a dependency nobody in this
repository owns.

### Option D — Use the egui fallback to reach Windows

Description: replace Tauri with egui/eframe, the alternative pre-sanctioned in
`ADR-0009`.

Benefits: none with respect to Windows.

Costs: it discards the shipped Tauri shell for no Windows benefit, because the
obstacle is `tokio::net::UnixStream`, not the webview.

Failure modes: rework of milestones 004 and 005 for no gain. Rejected.

### Option E — Reach Windows over a loopback TCP transport

Description: give the daemon a loopback TCP transport for Windows.

Benefits: no named-pipe implementation.

Costs: it weakens the local-access model that the Unix socket's `0600`
permissions provide, and it adds an authentication surface for a local socket.
`plans/000-long-term-specification.md#10-frontend-rules` already requires the
GUI's local transport to be loopback-bound, which a TCP transport would make a
live security concern rather than a filesystem guarantee.

Failure modes: a world-connectable or unauthenticated local port.

## Decision

**The desktop GUI's supported-platform set is Linux (x86_64, glibc) and macOS
(x86_64 and arm64), qualified through milestone 007. Windows is out of scope
for the desktop GUI.**

Precisely:

1. **Linux and macOS are the supported platforms** for the desktop GUI.
   Milestone 007 qualifies one row per platform, with evidence for every
   required behaviour (build, launch, connect, submit, cancel, render, export)
   or a named, justified gap.
2. **Windows is out of scope** for the desktop GUI. The GUI does not compile for
   Windows because its only transport — the daemon's Unix domain socket — is
   unavailable there. The root cause is recorded as the daemon's transport
   choice, not as a GUI defect.
3. **The engine, CLI, TUI, and Python surfaces are unaffected.** Their Windows
   support is unchanged, and the portability CI matrix keeps covering
   `windows-latest` for them.
4. **The re-entry path is a daemon-subsystem milestone, not a GUI milestone.**
   A Windows transport for the daemon — named pipes or an equivalent local IPC
   — changes the daemon's transport kind for every frontend and therefore
   requires its own ADR under the roadmap's decision-point rule ("any change to
   the daemon protocol that is not strictly additive"). When such a milestone
   lands, the GUI's Windows row may be qualified; the GUI needs no new
   architecture to use it.
5. **`ADR-0009`'s egui fallback remains pre-sanctioned** for its original
   purpose (removing the system webview dependency, and as the escape hatch
   for a failing Tauri closure). It is **not** the Windows remedy, and this
   decision does not invoke it.

Ownership boundary: this decision fixes **which platforms the GUI is offered
on**. It does not own signing, notarization, installers, or update channels —
those remain with the separately-managed distribution workstream, and milestone
007 hands that workstream the binary and a reproducible build recipe.

## Consequences

### Positive

- Milestone 007's exit condition becomes decidable and checkable.
- No daemon transport change is forced, and the daemon's Unix-socket access
  model (`0600` socket permissions at
  `crates/eggsec-daemon/src/server.rs:100-101`) is preserved.
- The platform claim matches what the project can actually build and qualify.
- The GUI's own cross-platform surface (Linux's `webkit2gtk-4.1` floor,
  macOS's webview) stays a prerequisite-documentation problem rather than a
  transport problem.

### Negative

- Windows users have the CLI, TUI, and Python but not the desktop GUI.
- The GUI's release footprint is narrower than the rest of the product's, so
  "one product, many surfaces" now carries a platform qualifier.
- The GUI's code remains Unix-only, so any future Windows work must revisit
  `crates/eggsec-frontend-client/src/daemon.rs` even if only to compile.

### Neutral or deferred

- **Windows named pipes** — deferred to a future daemon-subsystem milestone.
- **A loopback TCP transport** — not selected; it would weaken the local
  access model.
- **Signing, notarization, installers, update channels** — unchanged; owned by
  the distribution workstream.
- **The GUI's own build prerequisites on Linux and macOS** — unchanged;
  milestone 007 documents them.
- **The egui fallback** — unchanged and still available for its original
  purpose.

## Compatibility and migration

No protocol, storage, DTO, authorization, or CLI change. No data migration: the
GUI owns no authoritative state, and the supported-platform set is a build and
packaging fact rather than a persisted one.

There is one operator-visible effect: a Windows operator who expected a
desktop GUI now has the CLI, TUI, and Python surfaces, and the GUI is
documented as unsupported rather than merely unbuilt.

## Security and reliability implications

- The GUI's local transport remains a Unix domain socket with `0600`
  permissions (`crates/eggsec-daemon/src/server.rs:100-101`). Deferring a
  Windows transport avoids introducing a local TCP port whose authentication
  would have to be designed separately.
- `plans/000-long-term-specification.md#10-frontend-rules` requires the GUI's
  local transport to be loopback-bound. A Unix socket satisfies that by
  filesystem locality; a future Windows transport MUST re-establish the same
  guarantee, and its ADR MUST address it explicitly.
- No enforcement, scope, or approval semantics change. The GUI remains a
  manual surface, and the engine remains the only authorizer.
- Reliability: the supported set now matches the set for which the project can
  produce launch, connect, submit, cancel, render, and export evidence, so a
  qualified GUI release is one whose behaviour is known.

## Verification

Implementation conformance is proved by:

1. Milestone 007's platform matrix carries exactly two qualified rows (Linux,
   macOS) and one row for Windows that is explicitly **out of scope** with its
   root cause named — not blank, not "planned", not hopeful.
2. A CI job builds the GUI and runs its test suite on `ubuntu-latest` and
   `macos-latest`; **no** Windows GUI job is claimed or required.
3. The Windows exclusion is recorded in `plans/000-long-term-specification.md`
   §10, the subsystem roadmap, `plans/registry.md`, and milestone 007's
   known-limitations record, so no document implies Windows GUI support.
4. A guard asserts that the GUI's supported-platform claim and its build
   matrix agree — that is, that no document claims a Windows GUI build that
   does not exist.
5. The cross-compilation evidence is retained in milestone 007's closure
   record: `cargo check -p eggsec-gui --no-default-features --target
   x86_64-pc-windows-msvc` fails with the unresolved `tokio::net::UnixStream`
   import, recorded as the **expected** documented state rather than a
   regression.

## Supersession

None.
