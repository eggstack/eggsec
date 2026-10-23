# ADR-0009: GUI shell technology selection

Status: accepted

Date: 2026-10-22

Ratified: 2026-10-23

Decision owners: project maintainers

Related specification sections:

- `plans/000-long-term-specification.md#5-crate-ownership`
- `plans/000-long-term-specification.md#8-runtime-and-dependency-baselines`
- `plans/000-long-term-specification.md#10-frontend-rules`
- `plans/002-long-term-roadmap.md#phase-7--standing-maintenance-and-future-capability-open`

Affected subsystem roadmaps:

- `plans/subsystems/desktop-gui-frontend-roadmap.md`

Related ADRs:

- `plans/adrs/ADR-0007-desktop-gui-frontend-boundary.md`
- `plans/adrs/ADR-0008-operation-parameter-schema-publication.md`

Related implementation plans:

- `plans/implementation/desktop-gui-frontend/004-gui-shell-skeleton.md`
- `plans/implementation/desktop-gui-frontend/005-schema-driven-forms-and-task-dashboard.md`

## Context

`ADR-0007` fixes the desktop GUI's **boundary** — a leaf application crate that
reaches the engine only through the daemon protocol — without fixing the
**shell technology** that renders it. The shell is the largest single
dependency-family decision in this workstream, and it is the one most likely
to be regretted if chosen for the wrong reason.

Eggsec's GUI surface is unusually demanding for a UI toolkit. It must present
roughly 30 operation kinds across 33 logical panels, each with its own input
form, its own result rendering, and its own live progress and log stream. It
must handle dense, sortable, filterable result tables. It must stay
responsive while the engine performs network I/O. It must run on macOS, Linux,
and Windows. It must be operable by a security practitioner, who expects a
polished, information-dense tool rather than a developer debug UI.

The workspace constrains the choice more than most:

- `rust-version = "1.89"` is a declared, CI-enforced contract
  (`Makefile:93-98` runs `cargo +1.89 check --workspace`).
- `deny.toml` is a closed license allow-list — MIT, Apache-2.0, BSD-2/3,
  ISC, Unicode-3.0, MPL-2.0, CC0-1.0, Unlicense, Zlib, BSL-1.0, 0BSD,
  CDLA-Permissive-2.0 — and `make check-deps` fails closed when
  `cargo-deny` is absent.
- TLS is ring-only everywhere; `reqwest` uses `rustls-no-provider`, never
  `rustls`. A GUI shell that pulls its own HTTP stack with a different TLS
  posture would violate this.
- The repository has **no JavaScript toolchain today**. The root
  `package.json` is an empty object (`{}`).
- The maintainer has stated that **binary distribution is handled
  separately**, which removes packaging, signing, and installer concerns from
  this decision — but does not remove build-time or runtime concerns.

A prior survey of the candidate space produced three findings that materially
shape this decision:

1. **A prior art exists for exactly this shape.** A team building a
   data-heavy table application benchmarked egui, iced, Slint, and GTK, hit
   walls in every native toolkit, and shipped Electron for the UI with Rust
   for the core. Eggsec's GUI is the same shape — dense tables, live streams,
   many panels — so this result is directly relevant rather than adjacent.
2. **Webview-backed shells cost memory.** Measured figures put webview-backed
   desktop apps at roughly 208–211 MiB peak RSS versus 79–109 MiB for native
   Rust toolkits. Eggsec runs concurrent network assessments; resident memory
   is a real operational cost.
3. **Toolkit accessibility is the weakest common denominator.** Self-drawn
   Rust toolkits ship weaker accessibility than platform-native widget sets,
   and a webview inherits the platform webview's accessibility tree.

Verified dependency facts as of 2026-10-22:

| Crate | Latest stable | MSRV | Edition | License |
|---|---|---|---|---|
| `tauri` | 2.12.5 (2026-10-21) | 1.90 | 2024 | Apache-2.0 OR MIT |
| `tauri` | 2.11.4 (2026-06-30) | **1.77.2** | 2021 | Apache-2.0 OR MIT |
| `tauri-runtime-wry` | 2.12.3 (2026-10-21) | 1.90 | 2024 | Apache-2.0 OR MIT |
| `tauri-runtime-wry` | 2.11.4 (2026-06-30) | **1.77.2** | 2021 | Apache-2.0 OR MIT |
| `tauri` | 3.0.0-alpha.5 (2026-10-14) | 1.95 | 2024 | Apache-2.0 OR MIT |
| `egui` | 0.33.0 (2025-10-09) | **1.88** | 2024 | MIT OR Apache-2.0 |

The MSRV column is the load-bearing one. The Tauri 2.12 line raised its MSRV
to 1.90, above the workspace's 1.89 — but the **2.11.4 line still declares
MSRV 1.77.2**, comfortably below the workspace floor. `egui` 0.33.0 declares
MSRV 1.88, also below 1.89. **Both leading candidates can be used without
changing the workspace MSRV or excluding the GUI crate from
`make check-msrv`**, provided the dependency is pinned to a specific
version rather than floating on `^`.

Tauri's `wry` feature resolves to `webview2-com` on Windows and
`webkit2gtk` on Linux; there is no bundled webview on Linux, so the GUI's
Linux build has a system dependency on `webkit2gtk-4.1`, which requires
Ubuntu 22.04 / Debian 12 or newer.

## Decision drivers

- The UI must support dense tables, live streaming output, and many
  parameterized panels without bespoke widget work per panel.
- The shell MUST NOT weaken any workspace invariant: MSRV, license allow-list,
  ring-only TLS, dependency direction, or the `make check` contract.
- The shell MUST be replaceable without engine changes — this is the whole
  point of `ADR-0007`.
- A JavaScript build step is acceptable, but it is a **new** toolchain in a
  Rust-only repository and must be justified, not assumed.
- The shell's runtime trust boundary must be defensible for a security tool.
- The choice must not foreclose the fallback: if the shell proves unsuitable,
  swapping it must be a contained change to `crates/eggsec-gui` only.

## Considered options

### Option A — Tauri v2 (Rust shell + system webview + web frontend)

Benefits: the highest UI ceiling per unit of effort for dense, data-heavy
interfaces — the web ecosystem's table, chart, and form libraries are mature
and directly consumable; the published JSON Schema from `ADR-0008` is
web-native, so the schema-driven form renderer needs no adapter; the binary
uses the operating system's webview rather than bundling Chromium, keeping
footprint small; Rust owns the protocol client, so the daemon-protocol
boundary from `ADR-0007` is preserved; both `tauri` and `tauri-runtime-wry`
are Apache-2.0 OR MIT, already inside the `deny.toml` allow-list.

Costs: introduces a Node-based frontend build into a Rust-only repository;
Linux builds gain a `webkit2gtk-4.1` system dependency that excludes older
distributions; the webview process raises resident memory relative to a
native toolkit; the `wry`/`webkit2gtk`/`webview2-com` dependency closure must
clear `cargo deny` under `--all-features`, which is a real admission task;
the 2.12 line's MSRV of 1.90 forces an explicit pin to the 2.11.4 line unless
the workspace MSRV is bumped; and a content-security-policy and
loopback-binding discipline becomes a first-class security requirement.

### Option B — egui / eframe (pure Rust immediate-mode)

Benefits: no new toolchain — the repository stays Rust-only and the
`make check` contract is untouched; immediate mode is a natural fit for a
**schema-driven generic form renderer**, because a form is redrawn from
current state each frame rather than requiring a per-operation widget tree;
`egui_extras::Table` covers dense result tables; `egui_kittest` enables
headless screenshot testing, which fits the repository's evidence-heavy
verification culture; MSRV 1.88 is below the workspace floor; license is
MIT OR Apache-2.0.

Costs: the visual language is custom-rendered, so the result reads as a
developer tool unless substantial styling work is done; widget breadth for
complex data presentation is narrower than the web ecosystem's; accessibility
is provided via AccessKit but is generally weaker than a platform webview's
tree; there is no built-in system-tray or native menu integration; and the
prior-art benchmark above found walls in exactly this kind of toolkit for a
data-heavy table application.

### Option C — Iced (pure Rust retained)

Benefits: Elm-style architecture; pure Rust; consistent cross-platform
rendering.

Costs: weaker accessibility story than egui, no built-in menu or tray
integration, a smaller widget ecosystem for dense data presentation, and the
same prior-art warning. Strictly worse than Option B for this use case.

### Option D — Slint (declarative, product-grade)

Benefits: a polished product-UI aesthetic closer to a commercial tool; a
declarative model that maps well onto a schema-driven form; integrated
accessibility.

Costs: the `.slint` DSL is a second language and a second toolchain; the
community and Rust ecosystem are smaller than egui's or the web's; the
licensing model (GPL-3.0, a royalty-free tier carrying an attribution
requirement, or a commercial license) is a materially different posture from
Eggsec's MIT license and would need legal review; and the current MSRV is
above the workspace floor.

### Option E — Electron + Rust sidecar

Benefits: the widest ecosystem and the most mature desktop packaging.

Costs: bundles Chromium, producing a large binary and high resident memory
for what is a single-operator security tool; a Node toolchain plus an
additional IPC hop; and it duplicates capabilities a system-webview shell
already provides at a fraction of the cost. Rejected on footprint.

### Option F — Web application served by the daemon's HTTP API

Benefits: zero new Rust frontend code; reuses `eggsec-output`'s HTML
renderer; lowest implementation cost.

Costs: this is not a desktop application — no native window, no system tray,
no OS file dialogs, no local process lifetime — and it inherits the
browser's trust model. `ADR-0007` explicitly retains it as a *complementary*
surface rather than the desktop GUI. Rejected as the desktop GUI; not
rejected as a surface.

### Option G — Qt via `qt-rs` / `cxx`

Benefits: the most capable native widget set; native accessibility.

Costs: a C++ build dependency, a very large binding surface, substantial
complexity, and a licensing posture (GPL/LGPL/commercial) that requires
legal review. Disproportionate to this problem.

## Decision

**Accepted: Option A — Tauri v2 — with a pinned dependency line.**

The maintainer ratified this proposal on 2026-10-23. The decision remains
deliberately conditional on three specific, checkable conditions, each of
which the shell milestone MUST satisfy before it is allowed to close:

1. **Version pin.** The `tauri` dependency MUST be pinned to the **2.11.x**
   line (latest published patch 2.11.6, MSRV 1.77.2) rather than floating on
   `^2.12`, because the 2.12 line declares MSRV 1.90, above the workspace's
   1.89. The pin is recorded in the workspace manifest and re-verified by
   `make check-msrv`. Raising the workspace MSRV to 1.90 in order to adopt
   the 2.12 line is **not** part of this decision — that would be a separate
   recorded trade, requiring its own ADR, not an accident.

2. **Dependency admission.** The `wry` / `webkit2gtk` / `webview2-com`
   closure MUST pass `cargo deny --workspace --all-features` under the
   existing `deny.toml` allow-list without new license exceptions beyond
   what is recorded in `docs/DEPENDENCY_EXCEPTIONS.md` with an owner and a
   review-by date. **Tauri's TLS posture MUST NOT introduce a second TLS
   stack**: the `rustls-tls` feature must not be used if it would pull
   `rustls` with default features; the GUI does not perform outbound HTTP
   of its own — all engine traffic goes through the daemon.

3. **Linux system-dependency declaration.** The GUI crate MUST declare its
   Linux prerequisites explicitly, and the supported Linux baseline MUST be
   recorded as a documented platform floor (webkit2gtk-4.1 implies Ubuntu
   22.04 / Debian 12 or newer). Where a distribution below that floor
   matters for the target audience, the egui fallback in the next paragraph
   is the escape hatch.

**Fallback: Option B — egui / eframe.** If any of the three conditions above
cannot be met, the GUI shell is implemented with egui instead, and the
fallback is exercised behind the same `ADR-0007` boundary. The fallback is
**sanctioned in advance** so that a switch is a contained change to
`crates/eggsec-gui` rather than a re-plan.

The framework choice is a **property of `crates/eggsec-gui` only**. It MUST
NOT appear in the engine, the daemon protocol, the runtime DTOs, the schema
publication mechanism, or any shared frontend contract. A test or guard must
be able to demonstrate that swapping the shell requires no change outside
`crates/eggsec-gui`.

### Rationale for selecting Tauri over egui

The deciding factor is the shape of the workload, not a general preference for
web technology. Eggsec's GUI is a **data-heavy table application** with live
streams and many parameterized panels — the precise shape where the prior-art
benchmark found native toolkits insufficient and the team shipped a web UI
with a Rust core. Tauri provides that web UI capability while keeping the
Rust protocol client and the daemon boundary intact, and without Electron's
bundled-Chromium footprint. The `ADR-0008` schema artifact is also natively
consumable by a web frontend with no adapter layer.

The egui fallback is preserved precisely because the prior art also shows
this class of project can succeed with a native toolkit given enough
styling investment, and because the repository's Rust-only, guard-heavy
culture makes a zero-new-toolchain path valuable.

## Consequences

### Positive

- The GUI can achieve a high-quality, information-dense interface in a
  fraction of the effort a bespoke native toolkit would require for 33
  panels.
- The `ADR-0008` JSON Schema is consumed natively by a web frontend.
- The workspace MSRV contract is preserved without a gate exclusion.
- Both candidate licenses sit inside the existing `deny.toml` allow-list.
- The fallback is pre-sanctioned, so the framework decision is reversible at
  bounded cost.

### Negative

- A Node-based frontend build is introduced into a Rust-only repository. This
  adds a toolchain, a lockfile, and a supply-chain surface that the current
  dependency policy does not cover — `deny.toml` governs Rust crates, and the
  JavaScript dependency tree needs its own review.
- Linux gains a `webkit2gtk-4.1` system dependency, raising the platform
  floor and complicating packaging on older enterprise distributions.
- The webview raises resident memory relative to a native toolkit.
- The pinned 2.11.x line forgoes 2.12 features and fixes until the workspace
  MSRV is revisited.
- The shell introduces a content-security-policy, local-transport-binding,
  and remote-origin-exclusion discipline that the TUI does not have to
  satisfy, because a terminal has no origin model.

### Neutral or deferred

- Binary distribution, code signing, notarization, and installer formats are
  out of scope — handled separately per the maintainer's direction.
- JavaScript-side dependency auditing policy is deferred to the shell
  milestone, where the actual frontend dependency tree exists.
- The GUI's visual design language is deferred; this ADR constrains only the
  technology and its boundary.
- A strict or headless GUI mode is out of scope; see `ADR-0007`.

## Compatibility and migration

- **Engine and daemon.** Unaffected. The shell is confined to
  `crates/eggsec-gui`.
- **Rust workspace.** The `tauri` dependency enters only the GUI crate's
  manifest. The workspace `rust-version` stays 1.89 provided the 2.11.x pin
  holds.
- **Dependency policy.** The Rust dependency closure must clear `deny.toml`.
  The JavaScript closure is a new surface and must have a documented review
  policy before the shell milestone closes.
- **MSRV.** `make check-msrv` must remain green, including the GUI crate, if
  the 2.11.x pin holds. Excluding the GUI crate from the MSRV gate is
  permitted only as an explicitly recorded fallback if the pin proves
  infeasible, and is not the default.

## Security and reliability implications

- **Content security policy.** The GUI shell MUST ship with a restrictive CSP
  that forbids remote origins, and the webview MUST be bound such that it
  cannot load operator-supplied remote content. A security tool whose UI can
  fetch arbitrary URLs is a confused-deputy risk.
- **Local transport binding.** Any local transport the shell exposes MUST bind
  to loopback only, on an unpredictable port, with the daemon's own
  authorization still applying. The shell MUST NOT proxy engine traffic
  around the daemon's access controls.
- **TLS.** The GUI MUST NOT introduce a second TLS stack. Engine traffic is
  the daemon's; the GUI's own transport is loopback-only. If the GUI's
  frontend build requires an HTTP client, it MUST NOT weaken the ring-only
  policy for the Rust side.
- **Secret handling.** Credentials collected by a GUI form are transmitted to
  the daemon over the local transport and MUST NOT be persisted in the
  frontend's own storage or echoed in logs.
- **Reliability.** The shell holds no authoritative state; task truth remains
  in the daemon and the runtime. A shell crash MUST NOT lose or corrupt
  running tasks, and re-launch MUST re-attach to existing sessions.
- **Update surface.** A Tauri application performs its own update checks in
  some configurations. The GUI MUST NOT self-update without explicit
  operator action, and update checks MUST NOT be enabled by default, because
  an unsolicited network fetch from a security tool is itself a risk.

## Verification

Implementation is conformant when:

- the pinned `tauri` version and its MSRV are recorded in the workspace
  manifest and `make check-msrv` is green **including** the GUI crate;
- `cargo deny --workspace --all-features check` is green, and any new
  exception is recorded in `deny.toml` **and**
  `docs/DEPENDENCY_EXCEPTIONS.md` with an owner and a review-by date;
- the GUI crate's Rust dependency closure contains no `rustls` with default
  features, and the ring-only TLS invariant is demonstrably intact;
- a guard proves the shell framework is referenced only within
  `crates/eggsec-gui` and is absent from the engine, daemon protocol,
  runtime, and shared frontend contracts;
- a test or inspection demonstrates that the GUI performs no direct outbound
  HTTP to remote hosts and that its local transport is loopback-bound;
- the Linux system prerequisites and the supported platform floor are
  documented.

## Supersession

None. If a later decision selects Option B (egui) instead, this ADR is
superseded by a replacement that records the egui selection and the reasons
the Tauri conditions could not be met — the proposal history MUST be
preserved.
