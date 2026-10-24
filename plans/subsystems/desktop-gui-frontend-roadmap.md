# Desktop GUI Frontend Roadmap

Status: active (M1–M3 closed 2026-10-23; ADR-0009 ratified 2026-10-23; M4 and M5 implemented 2026-10-23; M7 conditionally closed 2026-10-23; M8 and M9 closed 2026-10-23; M6 implemented 2026-10-24; M10 and M11 implemented 2026-10-24; M12–M15 planned 2026-10-24)

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
pieces into a working desktop frontend; what remained was coverage, export, and
daemon lifecycle — not the foundation. **Milestones 007, 008, and 009 were
implemented on 2026-10-23** (007 conditionally closed, 008 and 009 closed), and
**milestone 006 was implemented on 2026-10-24** — the coverage model, the
report path, and the daemon lifecycle all landed. **Every milestone in this
roadmap now has a closure record.** What remains is the evidence work in
milestone 007's qualification matrix and the gaps listed below.

**The supported-platform set is decided.** `ADR-0010` fixes the desktop GUI's
platforms at **Linux and macOS** and records **Windows as out of scope**, with
the root cause named: the daemon's transport is a Unix domain socket, so the
GUI does not compile for Windows. The engine, CLI, TUI, and Python surfaces are
unaffected.

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
2. **The GUI's operation coverage and report export were not finished.**
   Milestone 005 rendered a form for any published task kind and reported an
   operation as unavailable when the connected daemon could not run it, but
   hiding unavailable operations rather than reporting them, and export
   through `eggsec-output`, were milestone 006's work.
   **CLOSED by milestone 006 (2026-10-24).** The GUI now publishes a **derived
   coverage model** — one row per task kind the daemon publishes, with the
   verdict and the reason — instead of hiding an operation and saying nothing.
   The daemon publishes its enabled-feature set additively, so the GUI can
   distinguish "the daemon withholds this capability" from "the daemon was
   built without the feature", and a live `FeatureMissing` denial becomes a
   learned verdict that wins over every static one. Report export produces the
   CLI's artifacts for six formats, written with the TUI's export discipline.
   The coverage model is derived from what the daemon publishes, so it cannot
   drift into a second, hand-maintained catalog.
3. **The GUI did not manage the daemon's lifecycle.** It required an
   already-running daemon and named the exact start command when the socket
   was unreachable. Spawning and supervising a daemon was milestone 006's work.
   **Milestones 008 and 009 closed the two defects that made the instruction
   unusable**: the daemon has a documented, reproducible build recipe for a
   profile that actually contains the engine's features, and the documented
   start command names the cargo feature the binary must be **built** with —
   the runtime `--full-executor` flag alone cannot add an engine that was not
   compiled in.
   **CLOSED by milestone 006 (2026-10-24).** The GUI can now spawn and
   supervise a daemon, but only **opt-in** and only for a binary the operator
   names with `--daemon-bin`: there is no `PATH` lookup and no sidecar
   convention, because nothing in the workspace installs `eggsec-daemon`. The
   spawn is an argv vector, never a shell string. The pre-flight check is a
   **connection attempt**, so a stale socket file does not block the spawn and
   a live daemon is never displaced. The readiness wait is bounded and
   completes the **protocol handshake** rather than scraping the child's
   stdout. Teardown sends SIGTERM first and escalates to SIGKILL after a
   bounded grace period, and a readiness timeout terminates the child rather
   than leaving an orphan. The default remains attaching to an
   operator-started daemon.
4. **Interactive approval in the daemon.**
   **CLOSED by milestone 011 (2026-10-24).** The daemon holds an operation at
   its confirmation point, exposes the decision, and takes the operator's
   answer. The answer names *yes* or *no*; what it permits is computed by the
   policy kernel, bounded by the confirmation classes the kernel demanded. A
   strict surface never holds a decision, so a strict client is still
   refused. The GUI shows the policy's own prompt with Approve and Reject.
5. **The socket-path default is duplicated in six places** — the CLI's
   `--socket`, the daemon's `DaemonConfig::default`, the daemon binary's
   `--socket-path`, the GUI crate's own default, and the same literal in the
   GUI's `frontend/app.js` and `frontend/index.html` — and there is no
   socket-path environment variable. (The count was previously recorded as
   four; it is six, and the GUI's own source comment understates it further.)
   Collapsing the duplication belongs to a corrective plan, not to this roadmap.
6. **A daemon built the way the daemon builds itself exposes almost nothing.**
   `eggsec-daemon`'s `full-executor` feature enables `eggsec/cli`, and the
   engine's `cli` feature enables only `clap` — so a `--full-executor` daemon
   links the engine with **none** of the engine's 28 `full`-aggregate members.
   Milestone 006's coverage value ("the GUI reaches the engine's full feature
   set") is therefore unreachable without a daemon feature profile, which is
   **milestone 008's** work.
   **CLOSED by milestone 008 (2026-10-23).** The daemon now has a `service`
   cargo feature carrying 20 engine features, derived from the policy catalog
   and machine-checked against the compiled binary; `full-executor` is its
   deprecated alias. The profile's document is
   `docs/DAEMON_FEATURE_PROFILE.md`.
7. **Nothing installs the daemon binary, and `eggsec daemon start` advertises
   capabilities it cannot execute.** `cargo install --path crates/eggsec-cli`
   installs only `eggsec`; there is no `PATH` or sidecar convention for
   `eggsec-daemon`. Meanwhile `eggsec daemon start` hosts a no-op executor
   that advertises **20** task kinds and rejects every task, and prints
   "Daemon started" after a fixed 100 ms whether or not the socket bound. The
   CLI's own doc comment claims the command "delegates to the daemon binary
   logic", which the code does not do. Milestone 009 owns the obtainability
   path and the start honesty fix; the CLI-side defect is visible to CLI users
   today, independently of this roadmap.
   **CLOSED by milestone 009 (2026-10-23).** The published capability list is
   now **derived from the executor's declaration**, so a rejecting executor
   publishes nothing; the CLI's rejecting executor is deleted; the start path
   waits for a real readiness signal and reports success only from a *ready*
   outcome; and both start paths refuse to displace a live daemon. The
   obtainability recipe is in `docs/DAEMON.md`.

8. **The GUI's manual surface was not in the engine's permissive set.**
   `resolve_loaded_scope` treated only `RuntimeSurface::CliManual` and
   `RuntimeSurface::TuiManual` as permissive
   (`crates/eggsec/src/runtime_bridge/executor.rs`). A
   `DesktopGuiManual` session with a non-explicit scope therefore took the
   strict branch and **failed closed** instead of receiving the default empty
   scope the CLI and TUI get. The GUI is documented — here and in `ADR-0007` —
   as a permissive manual surface, so this was a **behavioural divergence**, not
   a design choice. It was an engine-side defect in the runtime bridge, outside
   the GUI crate, and it was not owned by any milestone in this roadmap. It was
   recorded here so it was not lost.
   **CLOSED by milestone 006 (2026-10-24).** The defect was found while
   executing milestone 006 and blocked its own end-to-end verification, so the
   user directed that it be folded in. `RuntimeSurface::DesktopGuiManual` is
   now matched with the CLI and TUI surfaces on the permissive branch, and no
   other surface was widened — the strict surfaces still fail closed. The
   policy crate had always classified the desktop GUI as a peer of the CLI and
   the TUI manual surfaces; the engine's scope resolution was the outlier.
   The regression test was demonstrated to fail before the fix.
9. **The desktop GUI's platform qualification is honest but incomplete.**
   Milestone 007 produced the matrix, the build recipe, the prerequisite
   probe, the CI matrix job, the runbook, and the distribution handoff
   artifact — but only **one** behavioural cell is qualified (`macos` /
   Build), `macos` / Launch is partially qualified, the **Linux row is not
   qualified** (no Linux host was executed), and the behavioural cells are not
   qualified (no window-automation driver). The milestone is therefore
   **conditionally closed**. The evidence path for each cell is recorded in the
   matrix's promotion table and in `docs/GUI_PLATFORM_QUALIFICATION.md`; the
   `gui-platform` CI job on `ubuntu-latest` and `macos-latest` is what
   promotes a cell. **Milestone 006 has since removed one blocker**: the
   daemon lifecycle the runbook otherwise had to perform by hand is now the
   GUI's own opt-in `--daemon-bin` route, so the `Export` cell's path is
   shorter — but the cell still needs its own evidence, and the behavioural
   cells still need a window-automation driver.
10. **The daemon's server still unlinks the socket path before binding.**
    Milestone 009's pre-flight liveness check refuses to displace a live
    daemon from both start paths, but the check narrows the window to a race
    between two simultaneous starts; it does not close it, because
    `run_server` still unlinks the path before binding
    (`crates/eggsec-daemon/src/server.rs:140-147`). Server-side hardening —
    refusing to bind over a live socket — changes the server's bind contract
    and is explicitly deferred to a separate milestone.

11. **The report bridge covers only the result families with a clean
    mapping.** Milestone 006 derives a report document from a completed task
    result through a **pure** engine-side bridge, and it covers the families
    whose result carries the facts the canonical report model needs — the
    scan, pipeline, fingerprint, fuzz, and recon families, plus the wireless,
    database-pentest, and C2 domains that already had a bridge. Every other
    `TaskResult` variant yields **no report form** rather than a lossy
    document, so an operator can export a report for some completed tasks and
    not for others. A workspace-wide bridge for all thirty variants is a
    separate corrective milestone: it is a mapping exercise, not a contract
    change, and doing it half-way would be worse than not doing it.
12. **The GUI's IPC surface is thirteen commands.** Milestone 006 added four
    over milestone 005's eight — the coverage table, the export, the report
    content, and the daemon status — and milestone 011 added the thirteenth,
    `approve_task`. The command set's three pinning sites (the
    argument-naming test, the declared-set test, and the shell's handler list)
    move together. The surface stays narrow: no command takes a surface or a
    scope from the webview, and the export command takes a task id, a format,
    and an operator-chosen path — never a pre-built report.

**Gap owners.** Each open gap now has an owning milestone, so none of them is
unowned:

| Gap | Owner milestone | Note |
|---|---|---|
| 1 — only the GUI consumes the published schema | *none — explicitly out of scope* | Migrating the TUI's hand-built forms is an ADR-gated decision point and a stated non-goal of this roadmap. It is recorded, not scheduled. |
| 4 — interactive approval unwired | **closed by milestone 011** | Also a prerequisite of milestone 010's operator-visible outcome: without it, a manual daemon client cannot complete a confirmation-required operation. |
| 5 — socket-path default duplicated in six places | **milestone 012** | Paired with gap 10 because both are the daemon's socket contract. |
| 9 — platform qualification evidence | **milestone 014** | An evidence milestone; the driver is the deliverable. |
| 10 — the server unlinks the socket path before binding | **milestone 012** | Paired with gap 5. |
| 11 — the report bridge's coverage | **milestone 013** | |
| 12 — the GUI's IPC surface is thirteen commands | *none — informational* | Recorded so the count is not mistaken for a defect. Milestone 011 added the thirteenth (`approve_task`). |
**Gaps closed by the delivered milestones** (recorded so the history is not
rewritten): the parameter-schema gap and the TUI-namespaced client trait were
closed by milestones 001 and 002 respectively; the missing GUI surface identity
was closed by milestone 003, which added `RuntimeSurface::DesktopGuiManual` and
`ExecutionSurface::DesktopGuiManual` and classified both as manual; milestone
004 delivered the GUI crate and its shell, and milestone 005 delivered the
schema-driven form engine and the task dashboard. Milestone 011 closed gap 4,
the daemon's unwired interactive approval. Milestone 005 also
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
(**implemented 2026-10-24**, `Primary class: capability`; closure
`plans/closure/desktop-gui-frontend/006-full-operation-coverage-and-report-export-closure.md`).

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

**What the implementation established (2026-10-24).** Both findings above were
confirmed and closed. The daemon publishes its enabled-feature set additively,
so the GUI can now distinguish "the daemon withholds this capability" from "the
daemon was built without the feature", and the denial-driven fallback turns a
live `FeatureMissing` refusal into a reported verdict that wins over every
static one. Report data reaches the client over a new read-only command, with
the envelope's pinned empty payload untouched. The coverage model is derived
from what the daemon publishes — one row per published task kind — so an
operation the engine publishes tomorrow is covered without a line being added.

**Three findings the implementation produced, each recorded in the closure
record:**

- **The `DesktopGuiManual` surface was missing from the engine's permissive
  set.** The policy crate has always classified the desktop GUI as a peer of
  the CLI and the TUI manual surfaces; the engine's scope resolution did not,
  so every task the GUI dispatched through the daemon-hosted executor was
  denied. The fix is one arm in one `matches!`, and it is a user-directed
  fold-in of a defect no milestone owned.
- **An unknown daemon command was answered with an empty request id.** The
  daemon deserialized the line straight into `ClientCommand`, so a command it
  did not know was a deserialization failure answered with
  `ErrorCode::InvalidRequest` and no request id — and because the client
  routes responses by request id, the client waited out its 30-second
  timeout instead of learning the command was refused. M006 adds a command
  older daemons will not know, so this had to be fixed: the daemon now
  answers `ErrorCode::UnsupportedCommand` with the request id echoed.
- **The GUI's Content Security Policy forbids embedding the engine's HTML
  document.** `frame-src 'none'` rules out a frame, and `script-src 'self'`
  and `style-src 'self'` mean the document's inline script and style do not
  apply. The report content is attacker-influenced — banners and service
  names come from the target — so the CSP was **not** relaxed to make an
  embedded document work. The GUI renders the report's structured content as
  text in its own DOM, and the HTML format is **export-only**.

**What the implementation did not deliver.** The report bridge covers the
result families with a clean, non-lossy mapping to the canonical report
model; every other `TaskResult` variant yields "no report form" rather than a
lossy document, so an operator can export a report for some completed tasks
and not for others. A workspace-wide bridge is a separate corrective
milestone. The GUI's IPC surface grew from eight commands to twelve: the
coverage table, the export, the report content, and the daemon status are four
new commands, and the command set's three pinning sites moved together.

### Milestone 7 — Packaging and platform qualification

Class: operational

Objective: qualify the GUI on the supported platform matrix and hand the
binary to the separately-managed distribution process.

Dependencies: M6 (hard); M8 and M9 (hard — a qualification run needs a daemon
that can be built and obtained); distribution policy (operational).

Deliverable boundary: a qualified build per supported platform, documented
system prerequisites, and a handoff artifact for the distribution workstream.
**This milestone does not own signing, notarization, or installers.**

User or operator value: an installable desktop build exists for the
platforms the project supports.

Exit conditions: **Linux and macOS** — each builds and launches, with the
Linux system prerequisites documented and known limitations recorded.
**Windows is out of scope** for the desktop GUI and is recorded as such
(`ADR-0010`); it is not a qualified platform and must not appear as one.

Deferred work: signing, notarization, installers, update channels — all
owned by the separate distribution workstream.

Implementation plan: `plans/implementation/desktop-gui-frontend/007-packaging-and-platform-qualification.md`
(**conditionally closed 2026-10-23**, `Primary class: operational`; closure
`plans/closure/desktop-gui-frontend/007-packaging-and-platform-qualification-closure.md`).

**The milestone's infrastructure is implemented; its operational evidence is
outstanding.** The qualification matrix, the reproducible build recipe, the
prerequisite probe, the `gui-platform` CI job, the runbook, and the
distribution handoff artifact all landed. The matrix is deliberately honest
about what was and was not exercised: only `macos` / Build is **qualified**,
`macos` / Launch is **partially qualified**, the Linux row is **not
qualified** (no Linux host was executed here — the CI job is the named
evidence path), the behavioural cells are **not qualified** (no
window-automation driver, and M6 owns the daemon lifecycle the procedure
otherwise has to do by hand), and `Export` is **`pending M006`**. M6 remains
open, so the milestone is **conditionally closed** rather than closed.

**The supported-platform set is now decided.** `ADR-0010` fixes it at **Linux
(x86_64 glibc) and macOS (x86_64 and arm64)**, and records **Windows as out of
scope** for the desktop GUI. The root cause is the daemon's transport: the
frontend client imports `tokio::net::UnixStream` unconditionally
(`crates/eggsec-frontend-client/src/daemon.rs:9`) and the daemon imports
`std::os::unix::fs::PermissionsExt` and `tokio::net::UnixListener`
unconditionally (`crates/eggsec-daemon/src/server.rs:1,5`), so the GUI does
not compile for Windows. That is a daemon-transport property, not a GUI defect,
and `ADR-0009`'s egui fallback does not address it — the obstacle is the
transport, not the shell. Re-entry is gated on a **daemon-subsystem** transport
milestone with its own ADR, not on GUI work.

The platform matrix therefore has one row per platform — two qualified rows
and one explicitly out-of-scope row with its root cause named — and one column
per required behaviour: build, launch, connect, submit, cancel, render, export.
Each cell carries evidence or a justified gap, because `make check` never
exercises a windowed application. `bundle.active` stays `false`, so this
milestone hands over the binary and a reproducible build recipe rather than
making a packaging decision that belongs to the distribution workstream.

### Milestone 8 — Daemon feature profile and build recipe

Class: capability

Objective: make it possible to **build** a daemon that actually exposes the
engine's feature set, and to record how to build it.

Dependencies: none (independent of M5–M7, but it gates M6's coverage value).

Deliverable boundary: an explicit daemon feature profile — the engine features
a GUI-serving daemon enables — published as a cargo feature on
`crates/eggsec-daemon`; the system prerequisites each profile implies; and a
reproducible build recipe per profile.

User or operator value: milestone 006's stated value — "the GUI reaches the
engine's full feature set" — is unreachable without it, because a daemon built
the way the daemon builds itself today links the engine with **none** of the
engine's 28 `full`-aggregate members.

Exit conditions: a daemon built with the profile reports the profile's feature
set; the recipe is documented with the per-profile system prerequisites; and
the profile is a cargo feature on the daemon crate, not a hand-maintained list.

Deferred work: the profile's **runtime publication** belongs to milestone 006
(the additive enabled-features handshake field); the daemon's **install path**
belongs to milestone 009.

Implementation plan: `plans/implementation/desktop-gui-frontend/008-daemon-feature-profile-and-build-recipe.md`
(**closed 2026-10-23**, `Primary class: capability`; closure
`plans/closure/desktop-gui-frontend/008-daemon-feature-profile-and-build-recipe-closure.md`).

**Implemented.** The profile contract lives in the engine at
`crates/eggsec/src/config/daemon_profile.rs` — 20 included features, 30
excluded, an absolute-no forbidden subset, a **derived** required set that
reads the policy catalog, and a pure, fail-closed validator. The daemon's
`service` cargo feature enables the profile and `full-executor` is its
deprecated alias. The build recipe is `scripts/build-daemon-profile.sh` plus
`make build-daemon-profile`, and the conformance test runs on both sides —
statically in every `make check`, and at runtime against the **compiled**
feature set.

This is the one milestone in this roadmap whose **code surface is outside the
GUI crate**. It is tracked here because milestone 006 depends on it and there is
no daemon subsystem roadmap to carry it; that dependency is recorded rather than
hidden.

### Milestone 9 — Daemon obtainability and start honesty

Class: capability

Objective: make it possible to **obtain** a working daemon, and stop the
daemon that starts from advertising capabilities it cannot execute.

Dependencies: M8 (soft — the obtainable daemon is more useful with a real
feature profile, but the honesty fix does not depend on it).

Deliverable boundary: a documented install path for the daemon binary, and a
`eggsec daemon start` that either runs a real executor or advertises
honestly.

User or operator value: `eggsec daemon start` currently advertises **20** task
kinds and executes **none**, and nothing in the workspace installs the daemon
binary at all — so an operator following the GUI's own diagnostic ends up with
a daemon that looks capable and is not.

Exit conditions: the daemon binary has a documented, reproducible install path;
`eggsec daemon start` never advertises a capability it cannot execute; and the
CLI's false "Daemon started" message is corrected.

Deferred work: the GUI's own packaging, and the distribution workstream's
signing and installers.

Implementation plan: `plans/implementation/desktop-gui-frontend/009-daemon-obtainability-and-start-honesty.md`
(**closed 2026-10-23**, `Primary class: capability`; closure
`plans/closure/desktop-gui-frontend/009-daemon-obtainability-and-start-honesty-closure.md`).

**Implemented.** The capability lie is now unrepresentable: the executor
declares its own support, the host derives its published list from that
declaration, and a rejecting executor publishes nothing. The CLI's
rejecting executor is gone; the start path waits for a **real readiness
signal** and reports success only from a typed *ready* outcome; and both
start paths refuse to displace a live daemon. The obtainability recipe is
documented in `docs/DAEMON.md`.

Like M8, this milestone's code surface is outside the GUI crate. It is tracked
here because milestone 006's daemon-lifecycle work needs a daemon that can be
obtained, and milestone 007's qualification run needs one that works.

### Milestone 10 — The daemon's full manual capability policy

Class: capability

Objective: make the daemon **publish and admit every canonical task kind the
engine can execute**, so that no capability is blocked for manual execution at
the runtime layer.

Dependencies: `ADR-0011` (hard — **met**: accepted 2026-10-24);
M9 (hard — the derived publication rests on the executor's declaration).

Deliverable boundary: one capability list, **derived** from the canonical
task-kind set rather than transcribed, covering every canonical kind; the
daemon's published set and its admitted set made equal to that list
intersected with the executor's declaration; the interface-bound kinds given
an explicit target story instead of a button that fails; and the wireless
active kind's operation identity corrected so a deauth is not authorised
under the passive operation's risk level.

User or operator value: the daemon's published capability list is a
twenty-kind ceiling against a canonical thirty, and the daemon already
admits more than it advertises. The GUI's coverage table reports the
difference as unavailability, so the operator sees a smaller product than
the engine provides.

Exit conditions: the published set and the admitted set are the same object
and cover every canonical capability name; a kind that was previously
unadvertised is reachable; a rejecting daemon still publishes and admits
nothing; the interface-bound kinds are either runnable or reported with an
honest reason; and the wireless active kind is authorised under the
deauth operation.

Deferred work: the confirmation path (milestone 011) and therefore the
hazardous families' authorisation, which is a separate milestone because
wiring it changes the daemon's execution semantics rather than its
advertisement.

Implementation plan: `plans/implementation/desktop-gui-frontend/010-daemon-full-manual-capability-policy.md`
(**implemented 2026-10-24**; closure
`plans/closure/desktop-gui-frontend/010-daemon-full-manual-capability-policy-closure.md`,
`Primary class: capability`).

### Milestone 11 — Daemon-mediated policy approval

Class: capability

Objective: let a **manual daemon client answer a confirmation**, so that an
operation requiring confirmation is completable over the daemon protocol.

Dependencies: M10 (hard — the approval path is only exercisable for
published capabilities); M6 (soft — the GUI's task state already models a
policy prompt); `ADR-0012` (hard — **met**: the decision record this milestone
implements).

Deliverable boundary: the runtime holds an operation at its confirmation
point and exposes the decision; the daemon answers the approval command
instead of refusing it; the frontend client exposes the answer; and the GUI
offers the affordance.

User or operator value: today an operation the operator's policy permits but
which requires confirmation moves from "refused" to "needs confirmation" and
then stops there — the daemon has no way to receive a "yes". The CLI can
complete the same work, so the daemon is the weaker surface for exactly the
operations a manual operator most wants to confirm.

Exit conditions: a manual client submits an operation that requires
confirmation, observes the decision, answers it, and the operation runs; a
strict client of the same daemon is still refused; and a denied or unanswered
decision leaves the operation refused rather than pending forever.

Deferred work: the TUI's own approval flow, which is local, works today, and
is a different mechanism.

Implementation plan: `plans/implementation/desktop-gui-frontend/011-daemon-mediated-policy-approval.md`
(**implemented 2026-10-24**; closure
`plans/closure/desktop-gui-frontend/011-daemon-mediated-policy-approval-closure.md`,
`Primary class: capability`).

**Implemented.** The daemon can now hold an operation at its confirmation
point and take the operator's answer. The runtime owns the hold — a pending
decision registered before the prompt is announced, so a client that answers
immediately never races the registration — and it stays policy-neutral: the
registry holds a prompt and returns an answer, and only `eggsec-policy` decides
what an answer permits. The engine's executor re-runs the *same*
authorisation path with an override bounded by exactly the confirmation classes
the kernel demanded, so the answer is bounded by the policy rather than by the
client. A strict surface never holds a decision, so there is nothing for a
strict client to answer; the GUI shows the policy's own prompt with Approve
and Reject, and still never auto-approves. The daemon audits every answer
under the action `approve-policy`.

Like M8 and M10, this milestone's code surface is mostly outside the GUI
crate. It is tracked here because milestone 010's operator-visible outcome
depends on it: without the approval path, a manual daemon client can start a
confirmation-required operation but cannot finish it.

### Milestone 12 — The daemon's socket path and bind contract

Class: corrective

Objective: one canonical socket-path default, and a daemon that cannot
displace a live one.

Dependencies: none hard.

Deliverable boundary: a single default socket path with an environment
override, and a daemon that refuses to bind over a socket a live daemon is
holding.

User or operator value: the default path is transcribed in six places, and
the server unlinks the socket path before binding, so a second start can
take the socket from a running daemon.

Exit conditions: the default exists in exactly one place; the daemon refuses
to displace a live daemon and says so; and the CLI's start path reports the
refusal rather than silently taking the socket.

Deferred work: none.

### Milestone 13 — The report bridge's full coverage

Class: corrective

Objective: every completed task whose result has a report form produces one.

Dependencies: M6 (hard — the bridge and the bounded store exist).

Deliverable boundary: the engine-side bridge covers every task-kind variant
whose result carries the facts the canonical report model needs, and names
the variants for which no clean mapping exists.

User or operator value: an operator can export a report for some completed
tasks and not for others, with no stated reason.

Exit conditions: every variant with a clean mapping produces a document; the
variants without one are named and justified; and the export path reports
"no report available" rather than an empty artifact.

Deferred work: none.

### Milestone 14 — The GUI's qualification driver and runbook

Class: capability

Objective: make the behavioural cells of the platform qualification matrix
**promotable**, and write the missing export procedure.

Dependencies: M7 (hard — the matrix, the recipe, and the CI job exist); M6
(soft — the export path the export procedure documents exists).

Deliverable boundary: a window-automation driver, or a maintained operator
procedure, for the GUI's behavioural steps; and the complete runbook,
including the export procedure the matrix leaves undefined.

User or operator value: eight behavioural cells across two platforms are
blocked on the absence of a driver, and the export cells are blocked on a
procedure that was never written.

Exit conditions: the export procedure is written; the driver or the operator
procedure is in the repository; and the CI job's artefacts are recorded
against the matrix.

Deferred work: the Windows row, which is out of scope under `ADR-0010`.

### Milestone 15 — The dependency advisory backlog

Class: corrective

Objective: make the mandatory Rust contract **complete**, by upgrading the
advisories that can be upgraded and recording the ones that cannot.

Dependencies: none hard.

Deliverable boundary: every advisory with a safe upgrade is upgraded; every
retained exception carries an owner, a review-by date, and a record in the
dependency-exception document.

User or operator value: `make check` stops at the dependency-policy step, so
the repository's mandatory contract cannot complete today. Most of the
backlog is a stale lockfile rather than a genuine exception — the
patch-level upgrades alone clear a majority of the entries.

Exit conditions: `make check-deps` is green; every retained exception is
recorded with an owner and a review-by date; and `make check` completes.

Deferred work: none.

## 8. Cross-cutting requirements

### Storage and migration

The GUI owns **no** authoritative storage. Session and task state lives in
the daemon; the GUI re-attaches after a restart rather than restoring its own
copy. The schema artifact is derived at build or serve time and is not
persisted. **No data migration is required** by any milestone, because no
milestone changes an existing persisted representation.

### Protocol and compatibility

The daemon protocol changes **additively only**: one new read-only command
(`DescribeOperations`) and one new surface label, added by milestone 001.
Milestone 006 adds two more **additive** changes, and nothing else: a
defaulted enabled-feature field on the capabilities handshake, and a second
read-only command carrying report data. No existing `ClientCommand` or
`ServerMessage` shape changes, so existing clients
continue to work and older daemons answer the new command with the protocol's
existing `UnsupportedCommand` error, which clients MUST treat as a
graceful degradation to a static form set rather than a launch failure.

The **result envelope's empty payload stays empty**. `TaskResultEnvelope.payload`
is `json!({})` with `artifacts: vec![]`, and that is pinned by a test
(`crates/eggsec/tests/runtime_contract_closure.rs:618-619`). Milestone 006
routes report data over a new read-only command rather than filling the
envelope, because filling it is **not** additive and would be an ADR decision
point. The same reasoning forbids adding a `TaskOutcome` variant: the enum is
externally tagged, so an older client fails to deserialize an unknown variant.

### Security and authorization

- The GUI MUST NOT link the engine crate and MUST NOT import
  `EnforcementContext`, `EnforcedDispatcher`, `Scope::is_target_allowed`, or
  `tool.execute()`.
- The GUI surface is manual: permissive profile, operator overrides allowed,
  exactly like CLI and TUI manual sessions. Overrides are evaluated by the
  engine, never by the GUI.
- The daemon's `ApprovePolicy` is wired (milestone 011, `ADR-0012`). A GUI
  operating against a remote daemon MAY answer a pending interactive approval
  on the operator's behalf, and MUST NOT auto-approve without an operator's
  answer. The answer names *yes* or *no*; it never names the authorisation,
  which the policy kernel computes on the executor's side.
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
| The shell's dependency closure fails `cargo deny` | M4/M6/M7 blocked | **Corrected 2026-10-23.** The milestone-004 pre-verification established that the `tauri` 2.11.x closure is entirely MIT / Apache-2.0 / Zlib and clears the existing `deny.toml` allow-list — the gtk-rs bindings are MIT, not LGPL — so `cargo deny check licenses` is `licenses ok` and needs no exception. It did **not** check *advisories*, and seven advisory entries are reachable only through the GUI's Tauri closure: `glib 0.18.5` (RUSTSEC-2024-0429), `glib-macros 0.18.5` → `proc-macro-error 1.0.4` (RUSTSEC-2024-0370), and the five `unic-*` crates (RUSTSEC-2025-0081, -0075, -0080, -0100, -0098). Four more (`rustls 0.23.45`, `rustls-webpki 0.103.15`, `webpki-root-certs 1.0.9`, `tokio 1.53.1`) are shared with the engine but are attributed by `cargo deny` to a chain through `eggsec-gui` → `tauri` → `reqwest`, because `cargo deny` resolves over the lockfile graph while the GUI's feature-resolved build declines Tauri's `rustls-tls` feature. Re-verify on the resolved graph; if it still fails, admit with an owner and a review-by date, or fall back to egui |
| Schema derivation over 30 structs inflates compile time for downstream consumers | Build-time regression | Feature-gate or split into a leaf crate; measure before choosing |
| Linux `webkit2gtk-4.1` floor excludes target distributions | Packaging gap | Document the platform floor; the egui fallback avoids the system dependency entirely |
| The schema artifact becomes a second policy surface by accident | Enforcement regression | Guard: the schema MUST NOT be imported by any authorization or policy path; explicit test required |
| Feature-gated operations are reachable in the GUI but fail opaquely | Operator confusion | **Corrected 2026-10-23.** The earlier mitigation — hide or report unavailability "based on the daemon's advertised capabilities" — is **not implementable**: the published capability list is a compile-time constant that is never feature-derived, and it therefore cannot distinguish a feature that is off from a capability that was simply never advertised. Milestone 006 resolves availability from an **additive enabled-features read path** on the capabilities handshake, with a **denial-driven fallback** that turns a `FeatureMissing` denial into an explicit unavailability report. The fallback is mandatory: an older daemon answers with `UnsupportedCommand`, and the GUI MUST degrade to "cannot verify" rather than hide nothing |

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
  through the daemon protocol alone (**milestones 010 and 011** together: 010
  publishes and admits every canonical capability kind, 011 lets a manual
  client complete the confirmation the operator's policy demands);
- the engine, the daemon protocol's existing shapes, and the runtime DTOs are
  demonstrably unchanged in substance by the GUI's existence, beyond the
  **additive** items recorded here — the read-only schema command and the GUI
  surface variant (milestone 001 and 003), and the defaulted enabled-features
  handshake field plus the read-only report command (milestone 006);
- every operation is reachable from the GUI without per-operation frontend
  code, driven by the published schema;
- the shell framework is confined to `crates/eggsec-gui`, proven by guard;
- all workspace contracts — `make check`, `make check-deps`,
  `make check-msrv`, and the architecture guards — are green including the
  GUI crate (**milestone 015** closes the dependency-policy step, which is
  the only one still red);
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
| 6 — Full operation coverage and report export | **implemented 2026-10-24** | `plans/implementation/desktop-gui-frontend/006-full-operation-coverage-and-report-export.md` | `plans/closure/desktop-gui-frontend/006-full-operation-coverage-and-report-export-closure.md` | M5 (met — M5 implemented 2026-10-23) |
| 7 — Packaging and platform qualification | **conditionally closed 2026-10-23** | `plans/implementation/desktop-gui-frontend/007-packaging-and-platform-qualification.md` | `plans/closure/desktop-gui-frontend/007-packaging-and-platform-qualification-closure.md` | M6 (**met** — implemented 2026-10-24, so the export path exists; the `Export` cell still needs its own evidence); outstanding operational evidence named per cell in the qualification matrix |
| 8 — Daemon feature profile and build recipe | **closed 2026-10-23** | `plans/implementation/desktop-gui-frontend/008-daemon-feature-profile-and-build-recipe.md` | `plans/closure/desktop-gui-frontend/008-daemon-feature-profile-and-build-recipe-closure.md` | — |
| 9 — Daemon obtainability and start honesty | **closed 2026-10-23** | `plans/implementation/desktop-gui-frontend/009-daemon-obtainability-and-start-honesty.md` | `plans/closure/desktop-gui-frontend/009-daemon-obtainability-and-start-honesty-closure.md` | — (M8 met: closed 2026-10-23) |
| 10 — The daemon's full manual capability policy | **implemented 2026-10-24** | `plans/implementation/desktop-gui-frontend/010-daemon-full-manual-capability-policy.md` | `plans/closure/desktop-gui-frontend/010-daemon-full-manual-capability-policy-closure.md` | `ADR-0011` (**met** — accepted 2026-10-24); M9 (met — closed 2026-10-23) |
| 11 — Daemon-mediated policy approval | **implemented 2026-10-24** | `plans/implementation/desktop-gui-frontend/011-daemon-mediated-policy-approval.md` | `plans/closure/desktop-gui-frontend/011-daemon-mediated-policy-approval-closure.md` | M10 (met — implemented 2026-10-24); `ADR-0012` (implemented as written; acceptance is the maintainer's) |
| 12 — The daemon's socket path and bind contract | **planned 2026-10-24** | — | — | — |
| 13 — The report bridge's full coverage | **planned 2026-10-24** | — | — | M6 (met — implemented 2026-10-24) |
| 14 — The GUI's qualification driver and runbook | **planned 2026-10-24** | — | — | M7 (met — conditionally closed 2026-10-23); M6 (met) |
| 15 — The dependency advisory backlog | **planned 2026-10-24** | — | — | — |
