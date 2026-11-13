# ADR-0007: Desktop GUI frontend boundary

Status: accepted

Date: 2026-10-22

Decision owners: project maintainers

Related specification sections:

- `plans/000-long-term-specification.md#1-product-definition`
- `plans/000-long-term-specification.md#5-crate-ownership`
- `plans/000-long-term-specification.md#6-dispatch-runtime-and-wire-identity`
- `plans/001-terminology-and-domain-model.md`
- `plans/002-long-term-roadmap.md#phase-7--standing-maintenance-and-future-capability-open`

Affected subsystem roadmaps:

- `plans/subsystems/desktop-gui-frontend-roadmap.md`

Related implementation plans:

- `plans/implementation/desktop-gui-frontend/001-operation-parameter-schema-publication.md`
- `plans/implementation/desktop-gui-frontend/002-frontend-client-crate-extraction.md`
- `plans/implementation/desktop-gui-frontend/003-desktop-gui-surface-parity.md`

## Context

Eggsec's engine is already frontend-neutral. The canonical specification
requires "the same enforcement and dispatch architecture MUST support all
surfaces without creating separate products", and the workspace already ships
CLI, TUI, REST, MCP, gRPC, and Python frontends over one engine.

The TUI (`crates/eggsec-tui`) is the most complete interactive surface: 33 tabs
and roughly 65,000 lines of `src/`, all bound to the engine through a small
runtime-client abstraction (`crates/eggsec-tui/src/runtime_client/mod.rs:13-35`).
That abstraction is the natural place to add a second interactive frontend, but
it currently lives inside the TUI crate and is named after the TUI.

The maintainer has directed a desktop GUI frontend with these properties:

- it must reach every operation the engine exposes;
- the frontend must be swappable without changing the core;
- the underlying core stays the same;
- binary distribution is handled separately from this workstream.

The forcing constraint is the **frontend boundary**. If a GUI is added as a
consumer of the engine crate directly, it becomes a second engine client in
the same sense the CLI and TUI are, and the core acquires knowledge of a
frontend it must keep compiling. If instead the GUI is a consumer of the
**daemon protocol** — the same contract `DaemonRuntimeClient` already speaks —
then the core is genuinely untouched and the GUI is one more peer of the
existing daemon.

Two repository facts make the daemon-protocol boundary the right one:

1. The daemon protocol is already a complete, versioned, serde-based wire
   contract over a Unix socket (`crates/eggsec-daemon-protocol/src/protocol.rs`),
   with RBAC (`ApprovePolicy`), client declaration, session lifecycle, and a
   capabilities handshake.
2. The engine's daemon executor already resolves the acting surface from
   **session** state rather than from request payload
   (`crates/eggsec/src/runtime_bridge/executor.rs:138`), and fails closed for
   strict surfaces when scope cannot be resolved. A GUI that creates its
   session with a GUI surface therefore inherits the existing enforcement
   semantics without any engine-side special case.

The outstanding gap is that no `RuntimeSurface` / `ExecutionSurface` value
identifies a desktop GUI. `RuntimeSurface` has ten variants
(`crates/eggsec-runtime/src/request.rs:21-33`) and `ExecutionSurface` has nine
(`crates/eggsec-policy/src/policy.rs:362-381`); neither includes a GUI. A GUI
that borrowed `TuiManual` would work, but would write `surface: "tui-manual"`
into audit records, misattributing desktop actions to the terminal UI.

## Decision drivers

- The engine MUST remain unchanged by the presence of a GUI. "Unchanged" is
  read strictly: adding a new surface identity is permitted and necessary, but
  no existing operation's authorization, dispatch, or wire shape may shift.
- The GUI MUST NOT become a second authorization path. It MUST reach the
  engine only through the same enforcement gate every other surface uses.
- The GUI MUST be replaceable. Swapping the GUI framework MUST NOT require
  touching the engine, the daemon protocol, or the runtime DTOs.
- The GUI MUST be a leaf. It MUST NOT become a dependency of the engine, the
  daemon, or the runtime crates.
- Audit records MUST distinguish GUI-originated actions from TUI-originated
  actions.
- The GUI is an **application**, not a library. It is not published to
  crates.io and is not consumed as a dependency by other projects.
- Binary distribution, signing, and platform packaging are explicitly out of
  scope for this workstream and are handled separately.

## Considered options

### Option A — GUI as an engine client (like the TUI)

The GUI crate depends on the `eggsec` engine crate and calls
`EnforcementContext` / `EnforcedDispatcher` in-process.

Benefits: the shortest path to a working GUI; the TUI is a working precedent
with all 33 tabs already implemented.

Costs: the GUI becomes a peer of the engine and the core's compile surface
grows with it. The "swappable frontend" property degrades to "swappable
renderer" — the GUI's Rust code, not just its rendering, is entangled with the
engine. Every engine feature change requires a GUI rebuild and re-review of
whether the GUI's own dispatch path still matches policy. This is precisely
the ownership overlap that the frontend/runtime convergence workstream
(`plans/frontend-runtime-convergence-roadmap-2026-09-10.md`) was chartered to
remove.

### Option B — GUI as a daemon-protocol client (selected)

The GUI is a new leaf workspace crate that depends only on frontend-neutral
crates — `eggsec-runtime`, `eggsec-daemon-protocol`, `eggsec-ui-model` — and
talks to the engine through the daemon protocol. It may either attach to an
already-running daemon or spawn one.

Benefits: the engine is untouched by construction, because the GUI never
appears in the engine's dependency graph; the GUI's authorization identity is
carried in the session it creates, which the executor already honors; the
daemon protocol is already versioned, RBAC-bearing, and exercised by the TUI's
`DaemonRuntimeClient`; and the GUI framework becomes genuinely swappable
because the GUI's Rust side is a protocol client rather than an engine
integration.

Costs: the daemon protocol must carry everything the GUI needs to drive an
operation. Today it does not fully: there is no operation-parameter schema, so
a GUI would have to hand-build request payloads per operation exactly as the
TUI hand-builds per-tab forms. That gap is real and is addressed by
`plans/adrs/ADR-0008-operation-parameter-schema-publication.md`, not by
weakening this boundary.

### Option C — GUI as a second engine binary embedding the runtime

The GUI binary links the engine and hosts an in-process runtime, exposing its
own local IPC surface.

Benefits: no daemon process; lower latency; works with no system daemon
install.

Costs: it re-creates the engine-client coupling of Option A in a new place —
the GUI's Rust code would own dispatch, and enforcement would have to be
re-implemented or re-exposed correctly for a second time. It also duplicates
the daemon's session persistence, audit trail, and RBAC. The in-process
runtime already exists as `EmbeddedRuntimeClient` inside the TUI; the correct
home for that capability is a shared frontend-client crate, not a second
engine-linked GUI binary.

### Option D — GUI as a web application served by the daemon

The daemon's optional HTTP API (`http-api` feature) serves a static web
application that the operator opens in a browser or a webview shell.

Benefits: zero new Rust frontend code; maximum reuse of existing
`eggsec-output` HTML rendering.

Costs: this is not a desktop application. It has no native window management,
no system tray, no OS file dialogs, and no local process lifetime; it inherits
the browser's trust model for a security tool; and it is not deliverable as a
signed desktop binary. It remains valuable as a *complementary* surface and is
explicitly **not** rejected by this ADR — but it is not the desktop GUI this
decision authorizes.

## Decision

A desktop GUI is adopted as a **new frontend surface** with the following
binding properties.

1. **Ownership.** The GUI lives in a new leaf workspace crate,
   `crates/eggsec-gui`. It owns presentation, interaction, and client-side
   protocol adaptation. It owns **no** authorization, dispatch, scope
   resolution, policy evaluation, or operation semantics.

2. **Dependency direction.** `eggsec-gui` depends only on frontend-neutral
   crates (`eggsec-runtime`, `eggsec-daemon-protocol`, `eggsec-ui-model`) and
   on the chosen GUI shell framework. The engine MUST NOT depend on
   `eggsec-gui`. The daemon MUST NOT depend on `eggsec-gui`. This extends the
   existing guards — check 16 (daemon free of TUI and engine dependencies) and
   check 21 (engine free of TUI and daemon dependencies) in
   `scripts/check-architecture-guards.sh` — to a new, equivalent guard for the
   GUI crate.

3. **Engine access.** The GUI reaches the engine **only** through the daemon
   protocol. It MUST NOT link the engine crate, MUST NOT call
   `EnforcementContext`, `EnforcedDispatcher`, `Scope::is_target_allowed`, or
   `tool.execute()` directly, and MUST NOT construct an `ApprovedExecution`
   bundle.

4. **Surface identity.** A GUI is identified by a new surface variant,
   `RuntimeSurface::DesktopGuiManual`, with a matching
   `ExecutionSurface::DesktopGuiManual`. It is a **manual** surface:
   permissive profile, operator overrides allowed, exactly like CLI and TUI
   manual sessions. `ExecutionSurface::honors_manual_override()` MUST include
   it (`crates/eggsec-policy/src/policy.rs:417-419`). A strict GUI variant is
   NOT introduced at this time; if a headless/automated GUI mode is later
   required it must be added as a separate, separately-authorized surface
   under its own ADR, never by reusing the manual variant.

5. **Audit identity.** Sessions, tasks, and audit events originating from the
   GUI record the GUI surface label, so desktop actions are never
   misattributed to the TUI.

6. **Swappability.** The GUI shell framework is an implementation detail of
   `crates/eggsec-gui`. Changing it MUST NOT require changes to the engine,
   the daemon protocol, the runtime DTOs, or the schema publication
   mechanism. The framework selection is recorded separately in
   `plans/adrs/ADR-0009-gui-shell-technology-selection.md`.

7. **Not a library.** `eggsec-gui` is an application crate. It is not
   published to crates.io and is not a dependency of any other workspace
   crate. Its exclusion from the workspace MSRV gate is a consequence of this
   property, not a separate concession.

## Consequences

### Positive

- The engine's dependency graph, authorization path, and operation semantics
  are provably unaffected by the GUI's existence — verifiable by inspection and
  by a static guard, not by convention.
- The GUI inherits the daemon's existing guarantees for free: session
  persistence, task history, audit trail, RBAC, cancellation, and the
  fail-closed scope resolution that strict surfaces already get.
- The GUI framework becomes replaceable without engine churn, which is the
  maintainer-stated goal of a swappable frontend.
- The TUI gains a cleaner boundary as a side effect: the runtime-client
  abstraction moves out of the TUI crate into a shared frontend-client crate,
  so both interactive frontends consume one contract instead of two copies.
- Schema publication (`ADR-0008`) is reusable by REST, MCP, and Python, so
  this workstream also de-brittles existing programmatic surfaces.

### Negative

- The GUI cannot reach engine functionality that the daemon protocol does not
  expose. This is the real cost of the boundary and it is deliberate: the
  protocol becomes the complete, versioned contract for interactive surfaces.
- The GUI adds one network hop (Unix socket) compared with in-process
  dispatch. This is immaterial for scan-class operations whose cost is
  dominated by the assessment itself, but it must not be claimed as zero for
  very short operations.
- One additional surface variant must be added to two enums with exhaustive
  matches, and every surface-parity test that enumerates surfaces must be
  updated. That is deliberate churn: it is what makes the surface visible to
  audit and policy rather than implicit.

### Neutral or deferred

- Binary distribution, code signing, notarization, installer formats, and
  per-platform release channels are **deferred** — the maintainer has stated
  these are handled separately.
- The choice of shell framework is deferred to `ADR-0009`.
- Operation-parameter schema publication is deferred to `ADR-0008`.
- The GUI's visual design language, panel layout, theming, and accessibility
  targets are deferred to the shell milestone; this ADR constrains only
  ownership and access.
- Automated/headless GUI operation is explicitly **not** in scope. If required
  later, it is a strict surface and needs its own ADR.

## Compatibility and migration

- **Wire protocol.** No existing daemon message shape changes. The GUI uses
  the existing `ClientCommand` / `ServerMessage` vocabulary. The schema
  publication described in `ADR-0008` adds a new command rather than altering
  existing ones.
- **Existing frontends.** No behavioral change. The TUI, CLI, REST, MCP, gRPC,
  and Python surfaces continue to function identically.
- **Enforcement.** No existing operation's policy, risk tier, exposure flags,
  or dispatch path changes. `OperationMetadata` entries are untouched.
- **Audit records.** New records may carry a `desktop-gui-manual` surface
  label. Existing record consumers that filter on known surface labels MUST
  treat the new label as a recognized manual surface rather than an unknown
  value, or they may silently mis-handle GUI sessions.
- **Crate graph.** The workspace gains one member. The canonical specification's
  crate-ownership section is amended to record the new leaf.

## Security and reliability implications

- **Authorization.** The GUI has no authorization capability of its own. Every
  operation it requests is evaluated by the engine's `EnforcementContext` on
  behalf of the session's surface. The GUI cannot widen its own permissions
  because it never holds an `ApprovedExecution` bundle.
- **Scope.** The GUI is a manual surface and may use operator overrides, but
  overrides are evaluated by the engine, not the GUI. The GUI cannot bypass
  `scope_from_spec` fail-closed conversion because it never performs scope
  conversion.
- **Approval flow.** The daemon currently answers `ApprovePolicy` with
  `ErrorCode::Unsupported` ("not wired yet",
  `crates/eggsec-daemon/src/host.rs:791-812`). Until that is wired, a GUI
  operating against a remote daemon MUST fail closed on any operation that
  requires interactive approval, and MUST NOT auto-approve on the operator's
  behalf. The GUI's own confirmation UX may resolve approvals locally for
  embedded use, but it MUST NOT treat a local confirmation as satisfying a
  daemon-side policy decision.
- **Local trust boundary.** The GUI shell process hosts untrusted-by-default
  web content on some platforms. The shell's content security policy, its
  binding to a loopback-only transport, and its exclusion of remote origins
  are shell-milestone concerns governed by `ADR-0009`.
- **Secret handling.** Credentials referenced by an operation are owned by the
  engine and the daemon; the GUI MUST NOT persist them in its own state or
  write them to logs. Any credential the GUI collects for a request is
  transmitted over the local daemon transport and is not echoed back in
  responses.
- **Reliability.** The GUI holds no authoritative state. Task truth lives in
  the daemon and the runtime. A GUI restart re-attaches to existing sessions
  and tasks; it does not lose them. Cancellation is requested through the
  protocol, not performed locally.

## Verification

Implementation is conformant when:

- a static guard proves the engine, the daemon, and the runtime crates have no
  dependency edge to `eggsec-gui`;
- a static guard proves `eggsec-gui` does not link the engine crate and does
  not import `EnforcementContext`, `EnforcedDispatcher`, `Scope`, or
  `tool.execute()`;
- a test proves a GUI-created session is evaluated with the GUI surface and
  that the resulting audit record carries the GUI surface label;
- a test proves a strict surface still fails closed when scope cannot be
  resolved, and that the GUI surface does not relax that path;
- `make check` is green, and the architecture guards script reports no new
  failures;
- the daemon protocol's existing conformance tests pass unchanged apart from
  additions required by `ADR-0008`.

## Supersession

None.
