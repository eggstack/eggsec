# Desktop GUI Frontend Roadmap

Status: active (M1–M3 closed 2026-10-23; ADR-0009 ratified 2026-10-23; M4 and M5 implemented 2026-10-23; M6 and M7 not started)

Long-term references:

- `plans/000-long-term-specification.md#1-product-definition`
- `plans/000-long-term-specification.md#4-enforcement-invariants`
- `plans/000-long-term-specification.md#5-crate-ownership`
- `plans/000-long-term-specification.md#6-dispatch-runtime-and-wire-identity`
- `plans/000-long-term-specification.md#8-runtime-and-dependency-baselines`
- `plans/000-long-term-specification.md#10-frontend-rules`
- `plans/001-terminology-and-domain-model.md`
- `plans/002-long-term-roadmap.md#phase-7--standing-maintenance-and-future-capability-open`

Related ADRs:

- `plans/adrs/ADR-0007-desktop-gui-frontend-boundary.md`
- `plans/adrs/ADR-0008-operation-parameter-schema-publication.md`
- `plans/adrs/ADR-0009-gui-shell-technology-selection.md`

## 1. Purpose and ownership boundary

This subsystem delivers a **desktop graphical frontend** for Eggsec that
exposes the engine's full operation surface to an operator through a native
window, while leaving the engine, the daemon protocol, and the runtime DTOs
unchanged.

The subsystem **owns**:

- the GUI application crate (`crates/eggsec-gui`) and its presentation,
  interaction, and local client-side adaptation;
- the frontend-client contract shared with the TUI (the runtime-client trait
  and its embedded / daemon implementations);
- the discovery artifact that lets any frontend learn operation parameter
  shapes;
- the GUI's own surface identity in the wire and policy surface enums.

The subsystem **does not own**:

- authorization, dispatch, scope resolution, policy evaluation, or operation
  semantics — these remain engine-owned and are governed by
  `plans/000-long-term-specification.md#4-enforcement-invariants`;
- the daemon protocol's existing message shapes beyond an **additive**
  read-only command;
- the `OperationMetadata` policy catalog's authority — the GUI consumes it,
  it never extends it;
- binary distribution, code signing, notarization, or installer packaging —
  the maintainer has stated these are handled separately;
- the TUI's existing hand-built forms, which remain valid and are not
  migrated by this roadmap.

## 2. Work classification

### Invariants

- `EnforcementContext::evaluate()` remains the mandatory pre-dispatch gate.
  The GUI MUST NOT bypass it, and the GUI MUST NOT hold an
  `ApprovedExecution` bundle.
- The GUI MUST reach the engine only through the daemon protocol. The engine,
  the daemon, and the runtime crates MUST NOT depend on the GUI crate, and
  the GUI crate MUST NOT link the engine crate.
- `OperationMetadata` remains the single source of truth for operation
  policy. The schema artifact published by this subsystem is **descriptive**
  and MUST NOT participate in any authorization or policy decision.
- Strict surfaces continue to fail closed when scope cannot be resolved. The
  GUI surface is a **manual** surface and MUST NOT relax that path.
- TLS remains ring-only; the GUI MUST NOT introduce a second TLS stack.
- The workspace MSRV, license allow-list, and `make check` contract remain
  intact for every crate, including the GUI crate.

### Capabilities

- A desktop application that lists the engine's operations, renders an input
  form for each, submits it, shows live progress, and renders the result.
- A session and task view that reflects daemon-held state rather than
  client-held state.
- Operation-result rendering driven by the existing
  `eggsec-ui-model::RENDERER_REGISTRY` descriptors (23 renderer kinds).
- Report export through the existing `eggsec-output` renderers, including
  the HTML renderer for embedded report display.

### Infrastructure

- A shared frontend-client contract (the runtime-client trait currently held
  inside `eggsec-tui`) usable by more than one interactive frontend.
- A mechanically derived, machine-readable operation-parameter schema served
  over the daemon protocol.
- A `DesktopGuiManual` surface identity that is visible to policy and audit.

### Polish

- Visual design language, theming, and layout refinement.
- Accessibility conformance beyond the shell framework's defaults.
- Operator-facing ergonomics: keyboard shortcuts, command palette, saved
  layouts, remembered form state.

## 3. Non-goals

- **Not** a second engine. The GUI does not evaluate, authorize, dispatch, or
  implement any operation.
- **Not** a replacement for the TUI. The TUI remains a supported surface.
- **Not** a headless or automated GUI mode. If required later, it is a strict
  surface and needs its own ADR.
- **Not** a web-served application. The daemon's HTTP surface remains a
  complementary surface, not the desktop GUI.
- **Not** a REST/MCP/Python client redesign. This roadmap publishes the
  schema artifact that those surfaces may later adopt, but their migration is
  out of scope.
- **Not** binary distribution, signing, notarization, or packaging. Handled
  separately.
- **Not** a migration of the TUI's hand-built forms to schema-driven forms.
  That is an optional later benefit, not a dependency.
- **Not** a performance rewrite of the engine or the daemon.

## 4. Current state

The engine is already frontend-neutral, and the workspace already contains
most of the pieces a GUI needs. Milestones 001–005 have since turned those
pieces into a working desktop frontend; what remains is coverage, export, and
daemon lifecycle — not the foundation. Milestones 006 and 007 are **planned and
registered** (2026-10-23) but not yet implemented.

**What exists today:**

- A complete operation catalog: 34 canonical operations and 42 aliases in
  `crates/eggsec-policy/src/catalog.rs`, re-exported through the
  `config/policy_catalog` facade. Every entry carries identity, risk,
  intended uses, required features, required capabilities, target policy, and
  per-surface exposure flags.
- A versioned daemon IPC contract over a Unix socket in
  `crates/eggsec-daemon-protocol/src/protocol.rs`: 16 `ClientCommand`
  variants, a `ServerMessage` set, RBAC with client declaration, and a
  `DaemonCapabilities` handshake. The schema command added by milestone 001
  is one of the sixteen.
- A session host in `crates/eggsec-daemon` with session persistence, task
  history, audit events, and cancellation.
- A frontend-neutral runtime layer in `crates/eggsec-runtime`: `TaskKind`
  with 30 variants, each paired with a typed, serde-capable params struct
  (`PortScanParams`, `FuzzParams`, `WafParams`, `DbPentestParams`, and 26
  more) in `crates/eggsec-runtime/src/request.rs`.
- Frontend-neutral view DTOs in `crates/eggsec-ui-model`: 10 view modules and
  a `RENDERER_REGISTRY` of 23 result-renderer descriptors.
- A frontend-neutral runtime-client contract in
  `crates/eggsec-frontend-client`: `RuntimeClient` with eight methods,
  implemented by `EmbeddedRuntimeClient` (wrapping an in-process `Runtime`)
  and `DaemonRuntimeClient` (Unix-socket JSON-lines). Milestone 002 moved
  this out of the TUI, which previously owned it at
  `crates/eggsec-tui/src/runtime_client/mod.rs` under the name
  `TuiRuntimeClient`.
- A published operation-parameter schema, added by milestone 001: a
  mechanically derived JSON Schema over `TaskKind` and its 30 params
  structs, served by the read-only `DescribeOperations` daemon command.
- A desktop-GUI surface identity, added by milestone 003:
  `RuntimeSurface::DesktopGuiManual` and `ExecutionSurface::DesktopGuiManual`,
  both classified manual.
- A desktop GUI application crate, added by milestone 004:
  `crates/eggsec-gui` — a Tauri v2 window (`src/shell.rs`), a daemon
  connection over the shared frontend client (`src/connection.rs`), the
  Tauri IPC surface and the event forwarder (`src/commands.rs`), and a
  buildless HTML/CSS/vanilla-JavaScript frontend. It depends on
  `eggsec-frontend-client`, `eggsec-runtime`, `eggsec-daemon-protocol`, and
  `eggsec-ui-model` only, links no engine, and performs no outbound HTTP.
- A schema-driven form engine and task dashboard, added by milestone 005: one
  mapping from a published schema property to one control descriptor and one
  assembly path to a typed `RunRequest` (`src/form.rs`), a bounded view of the
  daemon's event stream (`src/tasks.rs`), and result rendering through
  `eggsec-ui-model`'s renderer registry. There is no per-operation frontend
  code anywhere in the crate.
- A working interactive surface: the TUI, with 33 tabs across 43 tab modules
  and roughly 65,000 lines of `src/`.
- Report rendering in `crates/eggsec-output` across JSON, CSV, HTML, SARIF,
  JUnit, and Markdown.

**Known gaps:**

1. **Only the GUI consumes the published schema.** `OperationMetadata` still
   carries no parameter information, nor does the tool registry's `ToolInfo`
   (`crates/eggsec/src/tool/registry.rs`), nor the REST surface's
   `ToolListItem` / `ToolDetailResponse`
   (`crates/eggsec/src/tool/protocol/rest.rs`). The Python bindings'
   `request_schema` is a human-readable hint, not a schema. The GUI renders
   every published task kind from the schema, but **the TUI's ~65,000 lines
   are still mostly hand-built forms.**
2. **The GUI's operation coverage and report export are not finished.**
   Milestone 005 renders a form for any published task kind and reports an
   operation as unavailable when the connected daemon cannot run it, but
   hiding unavailable operations rather than reporting them, and export
   through `eggsec-output`, are milestone 006's work.
3. **The GUI does not manage the daemon's lifecycle.** It requires an
   already-running `eggsec-daemon --full-executor` and names the exact start
   command when the socket is unreachable. Spawning and supervising a daemon
   is milestone 006's work.
4. **Interactive approval is unwired in the daemon.** `ApprovePolicy` returns
   `ErrorCode::Unsupported` (`crates/eggsec-daemon/src/host.rs`), so a
   daemon-mediated confirmation flow cannot complete today. The GUI handles
   `PolicyDecisionRequired` defensively and offers no approval affordance.
5. **The socket-path default is duplicated in six places** — the CLI's
   `--socket`, the daemon's `DaemonConfig::default`, the daemon binary's
   `--socket-path`, the GUI crate's own default, and the same literal in the
   GUI's `frontend/app.js` and `frontend/index.html` — and there is no
   socket-path environment variable. (The count was previously recorded as
   four; it is six, and the GUI's own source comment understates it further.)
   Collapsing the duplication belongs to a corrective plan, not to this roadmap.

**Gaps closed by the delivered milestones** (recorded so the history is not
rewritten): the parameter-schema gap and the TUI-namespaced client trait were
closed by milestones 001 and 002 respectively; the missing GUI surface identity
was closed by milestone 003, which added `RuntimeSurface::DesktopGuiManual` and
`ExecutionSurface::DesktopGuiManual` and classified both as manual; milestone
004 delivered the GUI crate and its shell, and milestone 005 delivered the
schema-driven form engine and the task dashboard. Milestone 005 also
**discharged `ADR-0009`'s deferred JavaScript dependency policy**: the
frontend ships an empty dependency set, so no JavaScript review policy is
needed today — any dependency admitted later requires a recorded policy with
an owner and a review-by date.

## 5. Target architecture

```text
┌──────────────────────────────────────────────────────────────┐
│  crates/eggsec-gui          (leaf application crate)          │
│  ┌──────────────┐  ┌──────────────┐  ┌───────────────────┐   │
│  │ GUI shell    │  │ schema-driven│  │ result rendering  │   │
│  │ (ADR-0009)   │  │ form engine  │  │ via RENDERER_     │   │
│  │              │  │              │  │ REGISTRY (23)     │   │
│  └──────┬───────┘  └──────┬───────┘  └─────────┬─────────┘   │
│         └─────────────────┼────────────────────┘             │
│                           │ implements                        │
│                  ┌────────▼─────────┐                          │
│                  │ RuntimeClient    │  (shared contract)       │
│                  │ trait            │                          │
│                  └────────┬─────────┘                          │
└───────────────────────────┼──────────────────────────────────┘
                            │ eggsec-daemon-protocol (wire DTOs)
┌───────────────────────────▼──────────────────────────────────┐
│  crates/eggsec-daemon          (unchanged + additive command) │
│    session host · RBAC · audit · persistence · cancellation     │
└───────────────────────────┬──────────────────────────────────┘
                            │ canonical execution seam
┌───────────────────────────▼──────────────────────────────────┐
│  crates/eggsec                      (unchanged in substance)  │
│    EnforcementContext · OperationMetadata · dispatch           │
└──────────────────────────────────────────────────────────────┘
```

Two shared contracts underpin the GUI:

- **The runtime-client contract** — the trait now at
  `crates/eggsec-frontend-client/src/lib.rs`, where milestone 002 relocated
  it from the TUI so that the TUI and the GUI both implement it. The TUI's
  `DaemonRuntimeClient` is the reference implementation and the GUI's client
  is a second implementation of the same contract.
- **The schema artifact** — a JSON Schema derived from the canonical request
  DTOs, published over the daemon protocol. This is what makes the GUI's form
  engine generic rather than per-operation.

The engine's dependency graph does not change. The daemon gains one additive
read-only command. The policy and runtime crates gain one surface variant
each.

## 6. Dependency graph

```text
M1  Schema publication ─────────────┐
    (ADR-0008)                     │ interface
                                   ▼
M2  Frontend-client extraction ──► M3  GUI surface parity
    (shared RuntimeClient)            │
                                     │ hard
                                     ▼
                              M4  GUI shell skeleton
                                  (ADR-0009 — blocked on ratification)
                                     │
                                     ▼
                              M5  Schema-driven forms
                                  + task dashboard
                                     │
                                     ▼
                              M6  Full operation coverage
                                  + report export
                                     │
                                     ▼
                              M7  Packaging & platform qualification
                                  (operational — distribution handled separately)
```

| Dependency | Milestone | Class |
|---|---|---|
| none | M1 | — |
| none (independent vertical slice) | M2 | — |
| none — M3 is a pure enum-extension slice that may run parallel to M1/M2 | M3 | — |
| M2, M3 (hard); ADR-0009 ratified (hard) | M4 | hard |
| M1, M4 (hard) | M5 | hard |
| M5 (hard) | M6 | hard |
| M6 (hard); distribution policy (operational) | M7 | operational |

M1, M2, and M3 are mutually independent and may proceed in any order or in
parallel. They are the **foundation** this roadmap's first three
implementation plans cover.

## 7. Milestones

### Milestone 1 — Operation parameter schema publication

Class: infrastructure

Objective: publish a mechanically derived, machine-readable description of
every operation's request shape, so that any frontend can discover parameter
structure instead of hand-coding it.

Dependencies: none.

Deliverable boundary: a derived JSON Schema over the canonical request DTOs,
reachable over the daemon protocol via a new additive read-only command,
without any change to existing message shapes or to authorization behavior.

User or operator value: none directly. It removes the per-frontend
duplication that currently costs the TUI most of its size, and it is
prerequisite to a GUI that is cheap to extend as operations are added.

Exit conditions: schema derivation covers every `TaskKind` variant; the
daemon command returns identity and policy metadata consistent with
`OperationMetadata`; a test proves the schema changes when a params struct
changes; `make check` and `make check-deps` are green with the new
dependency admitted to `deny.toml` and documented in
`docs/DEPENDENCY_EXCEPTIONS.md`.

Deferred work: REST, MCP, and Python adoption of the artifact.

### Milestone 2 — Frontend-client contract extraction

Class: infrastructure

Objective: relocate the runtime-client trait out of the TUI crate into a
shared frontend contract so more than one interactive frontend can implement
it, with the TUI migrated onto the shared contract unchanged in behavior.

Dependencies: none.

Deliverable boundary: a shared contract crate or module owning the runtime
client trait, its event receiver handle, and the embedded and daemon
implementations; `eggsec-tui` consumes it with no behavioral change.

User or operator value: none directly. It is the structural precondition for
a second frontend and it removes a TUI-shaped name from a shared contract.

Exit conditions: the TUI's full test suite is green with no behavioral
change; the trait is no longer TUI-namespaced; a guard proves the new crate
depends only on frontend-neutral crates; no engine dependency is introduced.

Deferred work: migrating the TUI's hand-built forms to the schema.

### Milestone 3 — Desktop GUI surface parity

Class: invariant

Objective: introduce a `DesktopGuiManual` surface identity that is visible to
policy evaluation and audit, and prove that it behaves as a manual surface
without weakening any strict-surface path.

Dependencies: none.

Deliverable boundary: one new variant in `RuntimeSurface` and one in
`ExecutionSurface`, the conversion between them, inclusion in
`honors_manual_override()`, audit labeling, and the surface-parity tests and
guards that enumerate surfaces.

User or operator value: GUI-originated actions are distinguishable in the
audit trail from TUI-originated actions.

Exit conditions: the GUI surface is accepted as a manual surface by the
existing policy evaluation; strict surfaces still fail closed with no
explicit scope; a session created with the GUI surface produces audit
records carrying the GUI label; the frontend surface matrix test and the
architecture guards are updated and green.

Deferred work: any strict or headless GUI mode, which requires its own ADR.

### Milestone 4 — GUI shell skeleton

Class: capability

Objective: stand up the GUI application crate with a working window, a live
connection to the daemon, and a rendered operation list — the smallest
end-to-end vertical slice.

Dependencies: M2 and M3 (hard); `ADR-0009` ratified (hard).

Implementation plan: `plans/implementation/desktop-gui-frontend/004-gui-shell-skeleton.md`

Deliverable boundary: `crates/eggsec-gui` builds and launches, connects to a
daemon over the shared client, lists the operations the daemon reports, and
renders them. No operation submission yet.

User or operator value: an operator can see the engine's operation surface in
a desktop window.

Exit conditions: the crate builds on the supported platform matrix; the
dependency closure clears `deny.toml`; the shell's local transport is
loopback-bound and its CSP forbids remote origins; `make check-msrv` is green
including the GUI crate; the shell framework appears only inside
`crates/eggsec-gui`.

Deferred work: form generation, results rendering, report export.

Delivered (2026-10-23): the crate exists as `crates/eggsec-gui`, with the
Tauri shell in `src/shell.rs`, the daemon connection and the GUI's client
identity in `src/connection.rs`, the IPC surface and the event forwarder in
`src/commands.rs`, and the CLI options in `src/config.rs`. The Tauri
dependencies are pinned with **exact** (`=`) requirements to the 2.11.x line
because the next lines up declare MSRV 1.90, above the workspace's 1.89 floor,
and `tauri` declares its runtime and utils dependencies with caret
requirements. The frontend is buildless, with zero npm dependencies. The
deferred form generation and result rendering above were delivered by
milestone 005.

### Milestone 5 — Schema-driven forms and task dashboard

Class: capability

Objective: render a generic input form for any operation from the published
schema, submit it, and show live progress and results using the existing
renderer registry.

Dependencies: M1 and M4 (hard).

Implementation plan: `plans/implementation/desktop-gui-frontend/005-schema-driven-forms-and-task-dashboard.md`

Deliverable boundary: one generic form engine that renders a field per
schema property; a task list showing live progress from daemon events; a
result view driven by `RENDERER_REGISTRY` descriptors; cancellation.

User or operator value: an operator can run an assessment from the desktop
GUI and watch it live — the actual product.

Exit conditions: a form is generated for every operation the schema
describes, with no per-operation frontend code; submission and cancellation
work against a live daemon; results render for at least the generic
`ResultEnvelopeView` path and the registry's structured kinds.

Deferred work: per-kind bespoke result visuals.

Delivered (2026-10-23): the generic form engine is `crates/eggsec-gui/src/form.rs`
— one mapping from a published schema property to one control descriptor, five
control kinds, no per-operation code — and the bounded task state and result
view are `crates/eggsec-gui/src/tasks.rs`, rendering through
`eggsec-ui-model`'s renderer registry. A blank field sends `null`, never
`""`, `0`, `false`, or `[]`, so an untouched fail-safe field such as `dry_run`
keeps the request DTO's own default. Retention is bounded: a capped log ring
per task and a capped number of log lines forwarded per flush, with the
dropped counts reported. This milestone also **discharged `ADR-0009`'s deferred
JavaScript dependency policy** — the dependency set is empty.

### Milestone 6 — Full operation coverage and report export

Class: capability

Objective: reach every operation the engine exposes, including feature-gated
domains, and provide report export.

Dependencies: M5 (hard).

Deliverable boundary: coverage of all 30 `TaskKind` variants, including the
feature-gated ones, with the GUI hiding operations whose required features
are not enabled in the connected daemon; export through the existing
`eggsec-output` renderers, including embedded HTML report display; **daemon
lifecycle management, which this milestone owns** — the GUI may spawn and
supervise an `eggsec-daemon --full-executor` child process when no daemon is
reachable, or MUST report precisely how to start one. Milestones 004 and 005
deliberately implement only the second half of that choice: the GUI connects
to an already-running daemon and names the exact start command when the socket
is unreachable, so the lifecycle decision is **M6's** to make, not an
accident of the shell's first cut.

User or operator value: the GUI reaches the engine's full feature set —
the maintainer's stated requirement for the GUI.

Exit conditions: a coverage test asserts every `TaskKind` variant is
reachable from the GUI; feature-gated operations are correctly hidden or
reported as unavailable rather than failing opaquely; export produces the
same artifacts as the CLI; the GUI's daemon-lifecycle behavior is explicit —
either it brings up a usable daemon or it says exactly how to, with no
opaque failure.

Deferred work: visual refinement, accessibility conformance beyond defaults.

Implementation plan: `plans/implementation/desktop-gui-frontend/006-full-operation-coverage-and-report-export.md`
(**planned 2026-10-23**, `Primary class: capability`; not yet implemented).

Two findings recorded at plan-authoring time, because they shape what this
milestone can deliver and they were measured rather than assumed:

- **The capability list cannot answer the availability question.** The daemon's
  published task kinds are a compile-time constant that is never feature-derived,
  and `DaemonCapabilities` publishes no enabled-feature set, so a client cannot
  tell whether a reachable daemon can execute a given operation. The plan
  therefore adds an **additive** enabled-features read path and keeps a
  denial-driven fallback for older daemons.
- **Report data cannot cross the wire today.** The result envelope's payload is
  empty and that is pinned by a test, so the plan routes report data over a new
  read-only command and treats filling the pinned payload as an **ADR-gated stop
  condition** rather than an implementation choice.

The daemon-lifecycle choice this milestone owns is resolved in the plan as: the
default remains "connect to a running daemon" with the precise start command,
and spawn-and-supervise is **opt-in behind an operator-supplied executable path**,
because nothing in the workspace installs `eggsec-daemon`.

### Milestone 7 — Packaging and platform qualification

Class: operational

Objective: qualify the GUI on the supported platform matrix and hand the
binary to the separately-managed distribution process.

Dependencies: M6 (hard); distribution policy (operational).

Deliverable boundary: a qualified build per supported platform, documented
system prerequisites, and a handoff artifact for the distribution workstream.
**This milestone does not own signing, notarization, or installers.**

User or operator value: an installable desktop build exists for the
platforms the project supports.

Exit conditions: each supported platform builds and launches; the Linux
system prerequisites are documented; known limitations are recorded.

Deferred work: signing, notarization, installers, update channels — all
owned by the separate distribution workstream.

Implementation plan: `plans/implementation/desktop-gui-frontend/007-packaging-and-platform-qualification.md`
(**planned 2026-10-23**, `Primary class: operational`; not yet implemented).

The plan is authored to be handed off **once M6 closes**; M6 is its hard
dependency and that is stated in the plan rather than papered over. One finding
recorded at plan-authoring time bounds the milestone's Windows row: the GUI
**does not currently compile on Windows**, because the frontend client uses
`tokio::net::UnixStream` unconditionally and the daemon's transport is a Unix
domain socket. Windows is therefore a **named limitation to escalate**, not a
qualification to claim. The platform matrix has one row per platform
(Linux/macOS/Windows) and one column per required behaviour — build, launch,
connect, submit, cancel, render, export — each cell carrying evidence or a
justified gap, because `make check` never exercises a windowed application.
`bundle.active` stays `false`, so this milestone hands over the binary and a
reproducible build recipe rather than making a packaging decision that belongs
to the distribution workstream.

## 8. Cross-cutting requirements

### Storage and migration

The GUI owns **no** authoritative storage. Session and task state lives in
the daemon; the GUI re-attaches after a restart rather than restoring its own
copy. The schema artifact is derived at build or serve time and is not
persisted. **No data migration is required** by any milestone, because no
milestone changes an existing persisted representation.

### Protocol and compatibility

The daemon protocol changes **additively only**: one new read-only command
(`DescribeOperations`) and one new surface label. No existing
`ClientCommand` or `ServerMessage` shape changes, so existing clients
continue to work and older daemons answer the new command with the protocol's
existing `UnsupportedCommand` error, which clients MUST treat as a
graceful degradation to a static form set rather than a launch failure.

### Security and authorization

- The GUI MUST NOT link the engine crate and MUST NOT import
  `EnforcementContext`, `EnforcedDispatcher`, `Scope::is_target_allowed`, or
  `tool.execute()`.
- The GUI surface is manual: permissive profile, operator overrides allowed,
  exactly like CLI and TUI manual sessions. Overrides are evaluated by the
  engine, never by the GUI.
- Until the daemon's `ApprovePolicy` is wired, a GUI operating against a
  remote daemon MUST fail closed on any operation requiring interactive
  approval and MUST NOT auto-approve on the operator's behalf.
- The schema artifact describes operation shapes across the whole operation
  surface. It is a reconnaissance aid and MUST be served only over the
  daemon's local transport under the same access controls as other commands.
- The GUI MUST NOT persist credentials in its own state or logs.
- The GUI MUST NOT perform outbound HTTP to remote hosts; all engine traffic
  goes through the daemon.

### Concurrency, cancellation, and recovery

The GUI holds no authoritative task state. It requests cancellation through
the protocol. Event streams are consumed from the daemon's subscription
channel; a dropped subscription must not affect the running task. A GUI
restart re-attaches to existing sessions and tasks. Spawned Tokio tasks in
the GUI's Rust side require 30–300s timeout wrappers, and failures are
reported through `tracing` — never `let _ =` or `filter_map(|e| e.ok())`.

### Observability and audit

Every GUI-initiated action carries the GUI surface label in sessions, tasks,
and audit records, so desktop activity is distinguishable from terminal
activity. The GUI's own errors are surfaced in its UI state rather than
written to the terminal.

### Performance and resource use

The GUI's resident footprint must be measured and recorded at the shell
milestone, because the shell framework's memory profile is a known variable
between candidate frameworks. The GUI MUST NOT retain unbounded event or
result payloads; result payloads are fetched on demand and large collections
are paginated. High-cardinality fan-out remains bounded by the engine's
configured concurrency — the GUI does not add a parallel fan-out.

### Documentation and operations

Each milestone updates `architecture/` alongside the code: a new
`architecture/desktop_gui.md` deep-dive is created at the shell milestone,
and `architecture/overview.md` gains the crate and surface entries. The
`eggsec-knowledge-corpus`-style skill discipline applies — a new subsystem
means a new skill under `.opencode/skills/`, and counted sets (crates,
surfaces, operations) must be updated in both `architecture/overview.md` and
the matching skill. `plans/registry.md` is updated at each milestone.

## 9. Verification strategy

- `make check` is the mandatory per-milestone contract: fmt, no-default
  checks, `make check-deps`, clippy, tests, and the architecture guards.
- `make check-msrv` MUST remain green **including** the GUI crate, unless an
  explicitly recorded decision excludes it.
- `make check-deps` MUST pass with the GUI's dependency closure admitted to
  `deny.toml`; any exception MUST also be recorded in
  `docs/DEPENDENCY_EXCEPTIONS.md` with an owner and a review-by date.
- `bash scripts/check-architecture-guards.sh` gains new guards: the GUI
  crate depends on no engine crate and imports no enforcement internals; the
  engine, daemon, and runtime crates depend on no GUI crate; and the shell
  framework appears only within `crates/eggsec-gui`. Milestones 004 and 005
  add six more: the no-engine edge, the framework's confinement to the GUI
  crate, the absence of a second TLS stack, the CSP's refusal of remote
  origins, the absence of per-operation form code, and the bounded event
  retention.
- The frontend surface matrix test
  (`crates/eggsec/tests/frontend_surface_matrix.rs`) is extended to include
  the GUI surface.
- Deep checks (`make check-full`, `make check-features-individual`,
  `make clippy-domain`, `make test-tui-pty`) are not per-PR obligations, but
  the feature-gated operation coverage in M6 requires
  `make check-features-individual` before closure.
- The GUI shell requires its own platform qualification evidence (launch,
  connect, submit, cancel, render) on each supported platform, since the
  `make check` contract does not exercise a windowed application.

## 10. Risks and decision points

| Risk | Impact | Mitigation |
|---|---|---|
| The daemon protocol proves insufficient for GUI needs (e.g. no per-task log streaming granularity, no artifact transfer) | M5/M6 stall | Audit the protocol against GUI needs during M2; add only **additive** read paths, never change existing shapes |
| Interactive approval cannot be completed through the daemon | Operations requiring confirmation are unusable from a remote daemon | Fail closed (M3); wire `ApprovePolicy` as a separate corrective milestone if required |
| The shell framework's memory or rendering characteristics disappoint | M4 rework | Measure at M4 before building on it; egui fallback is pre-sanctioned in `ADR-0009` |
| The shell's dependency closure fails `cargo deny` | M4 blocked | Pre-verified at the milestone-004 baseline: the `tauri` 2.11.x closure is entirely MIT / Apache-2.0 / Zlib and clears the existing `deny.toml` allow-list with no new exception — the gtk-rs bindings are MIT, not LGPL. Re-verify on the resolved graph; if it still fails, admit with an owner and review-by date, or fall back to egui |
| Schema derivation over 30 structs inflates compile time for downstream consumers | Build-time regression | Feature-gate or split into a leaf crate; measure before choosing |
| Linux `webkit2gtk-4.1` floor excludes target distributions | Packaging gap | Document the platform floor; the egui fallback avoids the system dependency entirely |
| The schema artifact becomes a second policy surface by accident | Enforcement regression | Guard: the schema MUST NOT be imported by any authorization or policy path; explicit test required |
| Feature-gated operations are reachable in the GUI but fail opaquely | Operator confusion | Hide or clearly report unavailable operations based on the daemon's advertised capabilities |

Decision points requiring an ADR:

- Any strict or headless GUI mode (currently out of scope).
- Any change to the daemon protocol that is not strictly additive.
- Migrating the TUI's forms to the schema, if that work is ever undertaken.
- The JavaScript dependency review policy, **discharged by milestone 005**: the
  frontend ships with an empty dependency set, and any JavaScript dependency
  admitted later requires a recorded policy with an owner and a review-by
  date.

## 11. Completion definition

This roadmap is closed when:

- a desktop GUI exists that reaches the engine's full operation surface
  through the daemon protocol alone;
- the engine, the daemon protocol's existing shapes, and the runtime DTOs are
  demonstrably unchanged in substance by the GUI's existence, beyond the two
  additive items recorded here (the read-only schema command and the GUI
  surface variant);
- every operation is reachable from the GUI without per-operation frontend
  code, driven by the published schema;
- the shell framework is confined to `crates/eggsec-gui`, proven by guard;
- all workspace contracts — `make check`, `make check-deps`,
  `make check-msrv`, and the architecture guards — are green including the
  GUI crate;
- documentation and skills are updated to match the delivered crate and
  surface counts;
- the binary has been handed to the separately-managed distribution
  workstream.

## 12. Milestone status

| Milestone | Status | Implementation plan | Closure record | Blockers |
|---|---|---|---|---|
| 1 — Operation parameter schema publication | **closed 2026-10-23** | `plans/implementation/desktop-gui-frontend/001-operation-parameter-schema-publication.md` | `plans/closure/desktop-gui-frontend/001-operation-parameter-schema-publication-closure.md` | — |
| 2 — Frontend-client contract extraction | **closed 2026-10-23** | `plans/implementation/desktop-gui-frontend/002-frontend-client-contract-extraction.md` | `plans/closure/desktop-gui-frontend/002-frontend-client-contract-extraction-closure.md` | — |
| 3 — Desktop GUI surface parity | **closed 2026-10-23** | `plans/implementation/desktop-gui-frontend/003-desktop-gui-surface-parity.md` | `plans/closure/desktop-gui-frontend/003-desktop-gui-surface-parity-closure.md` | — |
| 4 — GUI shell skeleton | **implemented 2026-10-23** | `plans/implementation/desktop-gui-frontend/004-gui-shell-skeleton.md` | `plans/closure/desktop-gui-frontend/004-gui-shell-skeleton-closure.md` | — |
| 5 — Schema-driven forms and task dashboard | **implemented 2026-10-23** | `plans/implementation/desktop-gui-frontend/005-schema-driven-forms-and-task-dashboard.md` | `plans/closure/desktop-gui-frontend/005-schema-driven-forms-and-task-dashboard-closure.md` | — |
| 6 — Full operation coverage and report export | **planned 2026-10-23** | `plans/implementation/desktop-gui-frontend/006-full-operation-coverage-and-report-export.md` | — (not yet closed) | M5 (met — M5 implemented 2026-10-23) |
| 7 — Packaging and platform qualification | **planned 2026-10-23** | `plans/implementation/desktop-gui-frontend/007-packaging-and-platform-qualification.md` | — (not yet closed) | M6 (**open** — M7 is gated on M6 closing); distribution policy |
