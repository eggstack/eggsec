# Desktop GUI Frontend Milestone 005 — Schema-Driven Forms and Task Dashboard

Status: executed — closed 2026-10-23

Repository baseline: `feat/auto-20261022-76692679` working tree (branch baseline commit `203ab5dd958415340d39cd47fb1add432c687edd`), with milestones 001–003 implemented and milestone 004 in progress.

Source roadmap:

- `plans/subsystems/desktop-gui-frontend-roadmap.md#milestone-5--schema-driven-forms-and-task-dashboard`

Long-term requirements:

- `plans/000-long-term-specification.md#4-enforcement-invariants`
- `plans/000-long-term-specification.md#5-crate-ownership`
- `plans/000-long-term-specification.md#6-dispatch-runtime-and-wire-identity`
- `plans/000-long-term-specification.md#10-frontend-rules`
- `plans/001-terminology-and-domain-model.md`

Applicable ADRs:

- `plans/adrs/ADR-0007-desktop-gui-frontend-boundary.md`
- `plans/adrs/ADR-0008-operation-parameter-schema-publication.md`
- `plans/adrs/ADR-0009-gui-shell-technology-selection.md`

Primary class: capability

## 1. Objective

Build one generic form engine in the GUI that renders an input form for any of
the 30 published task kinds from the schema the daemon publishes — with no
per-operation frontend code — then submits it, shows live progress from the
daemon's event stream, renders the result through the existing renderer
registry, and supports cancellation.

## 2. Why this milestone is ready

Closed hard dependencies:

- Milestone 001 (closed 2026-10-23) — the schema artifact exists and is
  published: `eggsec_runtime::schema::task_kind_schemas()`
  (`crates/eggsec-runtime/src/schema.rs:227`) and
  `describe_operations()` (`crates/eggsec-daemon-protocol/src/operations.rs:100`).
- Milestone 004 — the GUI shell exists, connects to the daemon, and can fetch
  the operation catalog.

Stable interface dependencies:

- `RuntimeClient::submit(session_id, request: RunRequest) -> TaskId`
  (`crates/eggsec-frontend-client/src/lib.rs:51`).
- `RuntimeClient::cancel` and `cancel_active` (`:53`, `:55`).
- `RuntimeClient::subscribe(session_id) -> RuntimeEventReceiverHandle` (`:57`).
- `RunRequest` (`crates/eggsec-runtime/src/request.rs:107`):

  ```rust
  pub struct RunRequest {
      pub task_kind: TaskKind,
      pub requested_by: Option<ClientId>,
      pub surface: RuntimeSurface,
      pub labels: Vec<String>,
  }
  ```

- `TaskKind` is adjacently tagged (`{"kind": ..., "params": {...}}`), so it
  deserializes from a `serde_json::Value` directly.
- `RuntimeEvent` has 13 variants (`crates/eggsec-runtime/src/event.rs:114`),
  tagged `#[serde(tag = "type")]`.
- `eggsec-ui-model` provides the frontend-neutral view DTOs and the renderer
  registry. It depends on `eggsec-runtime`, `serde`, and `serde_json` only —
  no engine, no framework.

## 3. Current implementation evidence

### The published artifact

`DescribeOperationsResponse` (`crates/eggsec-daemon-protocol/src/operations.rs:85`):

```rust
pub struct DescribeOperationsResponse {
    pub operations: Vec<OperationDescription>,
    pub task_kinds: Vec<TaskKindDescription>,
    pub task_kind_schema: serde_json::Value,
}
```

`TaskKindDescription` (`:72`):

```rust
pub struct TaskKindDescription {
    pub variant: String,
    pub operation_id: String,
    pub capability_name: String,
    pub request_schema: serde_json::Value,
}
```

`OperationDescription` (`:30`) carries 15 fields: `id`, `display_name`, `mode`,
`risk`, `intended_uses`, `required_features`, `required_policy_flags`,
`required_capabilities`, `target_policy`, and the six exposure flags
`manual_exposable`, `tui_exposable`, `mcp_exposable`, `rest_exposable`,
`agent_exposable`, `grpc_exposable`.

There is **no** `gui_exposable` field, by milestone 003's decision: the GUI
reuses `manual_exposable`. A GUI form MUST check `manual_exposable`.

### The measured schema shape

The artifact's exact shape was measured against the baseline rather than
assumed. `task_kind_schemas()` returns 30 entries; the whole-enum artifact is
a JSON Schema **draft 2020-12** document with top-level keys `$defs`,
`$schema`, `description`, `oneOf`, `title` — 30 `oneOf` branches, each with
`properties.kind` (a `const`) and `properties.params` (a `$ref` to
`#/$defs/<ParamsStruct>`), and 30 `$defs` entries.

Measured over all 30 **per-variant** schemas:

| Property | Finding |
| --- | --- |
| Total properties | 149 |
| Schemas containing `$ref` | **0** — every per-variant schema is self-contained |
| Schemas containing `$defs` | **0** |
| Schemas containing an `enum` constraint | **0** |
| Nested object properties | **0** |
| Array item types | always `{"type": "string"}` |
| Numeric formats present | `uint`, `uint16`, `uint32`, `uint64` |
| Properties with no `type` | exactly 1 — `IntegrationsParams::config`, whose schema is the boolean `true` |

The complete set of property type signatures is:

```
string | string|null | integer | integer|null | boolean | boolean|null | array|null
```

Optionality is expressed as a **nullable type union**, not as a separate
`required` list membership: an `Option<T>` field has type `["<T>", "null"]`,
and the field's name is absent from `required`. Only genuinely required fields
appear in `required` — for example `PortScanParams` requires exactly `target`.

**No `enum` constraint exists anywhere.** Enum-like fields such as
`PortScanParams::scan_type` are plain `string` on the wire. The schema
therefore **cannot** drive a dropdown for them, and the GUI must not pretend
it can: the authoritative list of accepted values lives in the engine, and the
GUI renders a free-text input whose value the engine validates.

### The form engine's input space is exactly five control kinds

Because the measured shape is this narrow, the generic form engine is a single
mapping from one JSON Schema property to one control descriptor:

| Schema shape | Control | Notes |
| --- | --- | --- |
| `"type": "string"` | required text input | |
| `"type": ["string", "null"]` | optional text input | absent ⇒ `null` |
| `"type": "integer"` / `["integer","null"]` | number input | `minimum` and `format` bound it |
| `"type": "boolean"` / `["boolean","null"]` | checkbox | `default: null` ⇒ tri-state |
| `"type": ["array","null"]`, `items.type == "string"` | repeatable string list | |
| schema is `true` (no `type`) | JSON text area | one field today: `IntegrationsParams::config` |

### The typed-deserialization insight

`RunRequest` is `Serialize + Deserialize` and `TaskKind` is adjacently tagged,
so a `serde_json::Value` of the form `{"kind": "PortScan", "params": {...}}`
deserializes into a typed `RunRequest` with `serde_json::from_value`. The
params structs are ordinary Rust structs with ordinary types — `u64`, `u32`,
`String`, `bool`, `Option<T>`, `Vec<String>` — so the deserializer rejects a
string where an integer belongs, a negative number where a `u32` belongs, or a
value too large for a `u16`.

This gives the form engine real per-field type checking **without any
hand-written validation table**: the same DTOs the engine will deserialize are
the ones the GUI validates against. The GUI's client-side check is a
convenience; the daemon's and engine's validation remain authoritative.

### The event stream

`RuntimeEvent` (`crates/eggsec-runtime/src/event.rs:114`), tagged
`#[serde(tag = "type")]`, with 13 variants:

`SessionCreated`, `Snapshot`, `TaskQueued`, `TaskStarted`, `TaskProgress`,
`TaskLog`, `PolicyDecisionRequired`, `TaskCompleted`, `TaskFailed`,
`TaskCancelled`, `SessionClosed`, `Audit`.

`TaskProgress` (`crates/eggsec-runtime/src/event.rs:28`):

```rust
pub struct TaskProgress {
    pub completed: u64,
    pub total: Option<u64>,
    pub message: Option<String>,
}
```

`TaskLog` carries `level: LogLevel` and `message: String`, with no task id
guarantee (`task_id: Option<TaskId>`). **This is the high-cardinality event
type**, and it is the one that must not be retained without bound.

### The result path

`TaskOutcome` (`crates/eggsec-runtime/src/event.rs:69`) has `Json(Value)`,
`Text(String)`, `Artifact { artifact_id, summary }`, and a structured-envelope
variant carrying `TaskResultEnvelope` (`:61`):

```rust
pub struct TaskResultEnvelope {
    pub kind: String,
    pub summary: Option<String>,
    pub payload: serde_json::Value,
    pub artifacts: Vec<ArtifactRef>,
}
```

`eggsec-ui-model`'s `ResultEnvelopeView` (`crates/eggsec-ui-model/src/result_view.rs:9`)
already converts from `&TaskResultEnvelope` and joins the registry:

```rust
impl From<&TaskResultEnvelope> for ResultEnvelopeView {
    fn from(env: &TaskResultEnvelope) -> Self {
        ...
        let renderer = super::renderer_registry::renderer_for_kind(&env.kind);
        Self {
            kind: env.kind.clone(),
            kind_label: renderer.map_or("Unknown", |r| r.title).into(),
            ...
            supports_rich_tui: renderer.is_some_and(|r| r.supports_rich_tui),
            supports_json_detail: renderer.is_none_or(|r| r.supports_json_detail),
        }
    }
}
```

`RENDERER_REGISTRY` (`crates/eggsec-ui-model/src/renderer_registry.rs:27`) holds
**23** entries of type `ResultRendererDescriptor`:

```rust
pub struct ResultRendererDescriptor {
    pub kind: &'static str,
    pub title: &'static str,
    pub summary_fields: &'static [&'static str],
    pub artifact_kinds: &'static [&'static str],
    pub supports_rich_tui: bool,
    pub supports_json_detail: bool,
}
```

The 23 registry kinds are result kinds, not request kinds, and do not map
one-to-one onto the 30 task kinds. `renderer_for_kind` returns `None` for an
unknown kind, and the documented degradation is a generic JSON render — the GUI
MUST implement that degradation rather than showing nothing.

`eggsec-ui-model` also provides `TaskProgressView` (with a computed
`percentage`), `TaskView`, `SessionView`, `SessionSummaryView`,
`EventView`, `ArtifactView`, `DashboardSummaryView`, `PolicyPromptView`, and
`PermissionView`.

### Which operations the connected daemon can actually run

`RuntimeClient::capabilities()` returns `RuntimeCapabilities`
(`crates/eggsec-runtime/src/capabilities.rs:23`), whose `task_kinds:
Vec<TaskCapability>` carries `{ name, description }`. The `name` is the
**capability name** — the same string as `TaskKindDescription::capability_name`.
That is the exact join key between a published task kind and the daemon's
support for it.

`RuntimeCapabilities::daemon_conservative()` lists a safe subset
(`port-scan`, `endpoint-scan`, `fingerprint`, `waf`, `pipeline`, `recon`,
`load-test`, `fuzz`, `waf-stress`, `nse`, …). A daemon built with a narrower
executor supports fewer kinds, so the GUI can and must determine support from
the live capabilities rather than assuming the full 30.

### Known gaps at the baseline

- No form engine, no task list, no result view, no cancellation in the GUI.
- The GUI's frontend has no notion of a task, a running task, or a result.
- `RuntimeClient` has no catalog method — but milestone 004 adds
  `describe_operations` as an inherent method on `DaemonRuntimeClient`, which
  is sufficient, because the GUI is daemon-only.
- `RuntimeEvent::PolicyDecisionRequired` carries a `PolicyPrompt`. The daemon's
  `ApprovePolicy` is unwired and answers `ErrorCode::Unsupported`, so this
  event cannot occur today. The GUI MUST still handle it rather than ignoring
  the variant.

## 4. Invariants that must not regress

- **The engine stays behind the daemon protocol.** The GUI MUST NOT depend on
  `eggsec`, `eggsec-daemon`, `eggsec-cli`, or `eggsec-tui`. The form engine
  consumes a published schema and submits a `RunRequest` — it does not know
  what any operation does.
- **`EnforcementContext::evaluate()` remains the mandatory pre-dispatch gate.**
  Submitting from the GUI is a daemon request; the daemon enforces. The GUI
  MUST NOT attempt to pre-authorize, pre-scope, or pre-approve anything.
- **The GUI never widens scope.** The session's scope is whatever the operator
  supplied. A form field is not a scope rule.
- **The published schema is descriptive, never authoritative.** The schema
  drives *form shape* only. It MUST NOT be consulted to decide whether an
  operation may run, what its risk is, or whether its required features are
  present — those come from `OperationDescription` and the daemon's live
  capabilities.
- **`OperationMetadata` remains the single source of truth for operation
  policy.** The GUI renders what the daemon publishes.
- **Strict surfaces fail closed without an explicit scope.** The GUI is manual,
  but a manual GUI still does not get a wider scope than the operator supplied.
- **Ring-only TLS.** The GUI still performs no outbound HTTP of its own. A
  build step that introduces a JavaScript HTTP client does not change that, but
  it MUST NOT introduce one that bypasses the daemon.
- **Bounded concurrency and bounded retention.** The event stream is
  high-cardinality. The GUI MUST NOT retain an unbounded event log, and MUST
  NOT spawn a task per event.
- **Never `let _ =` or `filter_map(|e| e.ok())`** — log via `tracing`.
- **Spawned tokio tasks carry a 30–300s timeout wrapper.** The event-forwarding
  task is long-lived by nature (it lives as long as the subscription); the
  timeout rule applies to finite work, and the implementer MUST document any
  task that is exempt and why.
- **The `make check` contract** — the GUI crate still compiles in the
  `--no-default-features` profile.
- **The shell framework stays inside `crates/eggsec-gui`.** Milestone 005
  adds no `tauri` edge anywhere else.
- **`ADR-0009`'s conditions still hold**, including the JS dependency policy
  deferred to this milestone.

## 5. Scope

### In scope

- The generic form engine: one mapping from a JSON Schema property to one
  control descriptor, and one assembly path from control values to a typed
  `RunRequest`.
- A form for every task kind the connected daemon publishes and supports.
- Submission against the live daemon.
- A live task list driven by the daemon's event stream.
- Progress, log, and status rendering.
- A result view driven by `RENDERER_REGISTRY`, with the generic JSON
  degradation for unknown kinds.
- Cancellation, per task and for the active task.
- Reporting an operation as unavailable when the connected daemon does not
  support its capability.
- The JavaScript dependency-audit policy that `ADR-0009` deferred here.

### Explicitly out of scope

- **Per-operation bespoke forms or bespoke result visuals.** The whole point
  of this milestone is that there are none. Bespoke visuals are deferred by
  the roadmap.
- **Hiding unavailable operations entirely.** Milestone 005 *reports* an
  operation as unavailable; milestone 006 hides it and reaches full coverage
  of all 30 variants including the feature-gated domains.
- **Report export.** Milestone 006.
- **Interactive policy approval.** `ApprovePolicy` remains unwired; the
  `PolicyDecisionRequired` event is handled defensively but no approval flow
  is built.
- **Engine-side validation changes.** The GUI validates for the operator's
  benefit; the engine's validation is unchanged and remains authoritative.
- **A new daemon command.** The existing `DescribeOperations`, `SubmitTask`,
  `CancelTask`, `CancelActive`, and `Subscribe` commands are sufficient.
- **Binary distribution, signing, notarization, and installers** — handled
  separately.

## 6. Required production changes

### Core/domain

None. The GUI gains no domain model. It gains a *view* of the published
schema — a control descriptor list — which is presentation, not domain.

### Storage and migrations

None. The GUI owns no authoritative storage. The task list is in-memory view
state derived from the event stream, bounded per §7.

### Protocol and DTOs

- **No protocol change is required.** `DescribeOperations`, `SubmitTask`,
  `CancelTask`, `CancelActive`, and `Subscribe` already exist.
- The GUI MUST NOT construct a `TaskKind` variant by hand. It builds
  `serde_json::Value` in the adjacently-tagged wire shape and lets
  `serde_json::from_value::<RunRequest>` do the typing. This is what keeps the
  form engine free of a per-operation switch.
- `RuntimeClient`'s method set is unchanged.

### Runtime and concurrency

- The GUI subscribes **once** per session with
  `client.subscribe(session_id)` and holds the
  `RuntimeEventReceiverHandle`. It does not resubscribe per task.
- The event-forwarding task drains the receiver and emits to the webview via
  Tauri's event mechanism. It MUST NOT spawn a task per event, and MUST NOT
  retain an unbounded event history.
- Per-task state is a map keyed by `TaskId` holding the **current** status:
  queued, running, completed, failed, or cancelled. `TaskLog` lines are
  retained in a **capped ring per task** — the cap is a named constant in the
  GUI, and the GUI MUST state the cap in its documentation. The event stream
  is the source of truth; the GUI's copy is a bounded view.

### Frontend or operator surface

- `crates/eggsec-gui/src/form.rs` — the form engine:
  - `control_for(property_schema: &serde_json::Value) -> Control` — the single
    mapping from a JSON Schema property to a control descriptor.
  - `controls_for(task_kind: &TaskKindDescription) -> Vec<Control>` — the
    field list for one task kind, derived from `request_schema.properties`
    and `request_schema.required`.
  - `assemble_request(variant: &str, values: &serde_json::Value) -> Result<RunRequest, FormError>`
    — builds `{"kind": variant, "params": {...}}`, adds the surface and
    labels, and deserializes into a typed `RunRequest`.
- `crates/eggsec-gui/src/tasks.rs` — the task-list state: the per-task map,
  the capped log rings, the status transitions, and the rendering payload.
- `crates/eggsec-gui/src/commands.rs` — additional IPC commands:
  `form_for(variant)`, `submit_task(variant, values)`, `cancel_task(task_id)`,
  `cancel_active()`, and `subscribe_tasks()`.
- `crates/eggsec-gui/frontend/app.js` — the form renderer, the task list, and
  the result view.
- The webview MUST receive the form descriptors as data. It MUST NOT contain a
  per-operation table, a per-operation branch, or a hardcoded field list.

### Security and authorization

- The IPC surface stays narrow. `submit_task` takes a variant name and a
  values object — it MUST NOT take a pre-built `RunRequest`, and MUST NOT
  accept a surface parameter from the webview. The surface is always
  `DesktopGuiManual`, set in Rust.
- The webview MUST NOT be able to widen scope. There is no IPC command that
  sets scope.
- The GUI MUST NOT filter operations by its own policy logic. Availability
  comes from the daemon's live `RuntimeCapabilities`, and exposure comes from
  `OperationDescription.manual_exposable`.
- An operation whose `manual_exposable` is false MUST NOT be submittable from
  the GUI, even though the GUI is a manual surface — the catalog's exposure
  flags are the authority on what a manual surface may run.
- The CSP from milestone 004 still applies. A JavaScript build step MUST NOT
  introduce a remote script source, a CDN import, or an outbound `connect-src`
  allowance.

### Documentation and static guards

- Two new guards:
  - **159 — the GUI has no per-operation form code.** The GUI's Rust sources
    and its frontend MUST NOT contain a match or switch over the 30 task-kind
    variant names, and MUST NOT contain a hand-written table of operation
    names. The form MUST be generated from the published schema.
  - **160 — the GUI retains no unbounded event log.** The GUI's task state
    MUST hold a capped number of log lines per task. The guard checks for the
    cap constant and for the absence of an unbounded push-to-a-`Vec` pattern
    in the task-state module.
- `architecture/overview.md` — the new modules and guards.
- `architecture/frontend_client.md` — the submit/cancel/subscribe path.
- `architecture/desktop_gui.md` — **new**, describing the GUI's architecture: the form
  engine, the task state, the result rendering, and the frontend's data flow.
- `docs/ARCHITECTURE.md`, `docs/ARCHITECTURE_INVARIANTS.md`,
  `docs/CI_ARCHITECTURE_GUARDS.md`, `docs/VERIFICATION.md` — as applicable.
- `docs/DEPENDENCY_EXCEPTIONS.md` — only if a JavaScript dependency exception
  is needed, following the same owner-plus-review-by rule the Rust side uses.
- `.opencode/skills/eggsec-gui/SKILL.md` — the form engine and task state.

## 7. Ordered work packages

### Work package A — The form engine

This is the core of the milestone. Everything else depends on it.

Required changes:

1. `Control` — a frontend-neutral descriptor the webview can render
   directly. It MUST carry, per field: the property name, the control kind
   (text, number, checkbox, list, json), whether the field is required, the
   numeric minimum and maximum derived from `minimum` and `format`, and the
   schema's `description` when present.
2. `control_for` — the single mapping. It reads one property schema and
   returns one control. It MUST handle exactly the shapes measured in §3 and
   MUST have a documented fallback for an unrecognized shape: a free-text
   input, never a panic and never a silent drop. The fallback matters because
   a future params struct may introduce a shape the engine has not seen, and
   the GUI must degrade rather than break.
3. `controls_for` — reads `request_schema.properties` and
   `request_schema.required`, and emits one control per property, ordered
   deterministically (required fields first, then alphabetical) so the form's
   layout is stable across runs.
4. `assemble_request` — the assembly path:
   - Build `params` from the control values: an absent optional field becomes
     `null`; a checkbox that was never toggled becomes `null` when the
     schema says the field is nullable and `default` is `null`; a list
     becomes an array of strings.
   - Build `{"kind": variant, "params": params}`.
   - Add `"surface": "desktop-gui-manual"`, `"requested_by": null`, and
     `"labels": []`.
   - `serde_json::from_value::<RunRequest>(value)` — the typed check.
   - A deserialization failure becomes a `FormError` naming the offending
     field and the expected type, derived from the serde error's position.

   The `surface` value MUST be produced by serializing
   `RuntimeSurface::DesktopGuiManual` rather than by hand-writing the string,
   so it cannot drift from the runtime's own label.

Acceptance: for every one of the 30 published task kinds, `controls_for`
produces one control per schema property — 149 controls in total — and
`assemble_request` round-trips a form filled with valid values into a typed
`RunRequest` whose `task_kind` is the right variant.

### Work package B — The task state

Required changes:

1. A per-task map keyed by `TaskId`, holding the current status, the
   progress, and a capped ring of log lines.
2. Status transitions driven by the event stream: `TaskQueued` → queued,
   `TaskStarted` → running, `TaskProgress` → progress, `TaskCompleted` →
   completed, `TaskFailed` → failed, `TaskCancelled` → cancelled.
3. A named cap constant for retained log lines per task. The cap MUST be
   small enough that a long-running task cannot grow the GUI's memory without
   bound, and large enough to be useful for diagnosis. The value and the
   reasoning MUST be recorded in `architecture/desktop_gui.md`.
4. Events for tasks the GUI does not know about are ignored by the task list
   but still counted in a diagnostic counter, so a mismatch between the event
   stream and the GUI's own submissions is visible rather than silent.
5. `PolicyDecisionRequired` is handled defensively: the task is marked as
   awaiting a policy decision and the GUI reports that the connected daemon
   cannot accept a manual approval. It MUST NOT offer an approval affordance,
   because `ApprovePolicy` is unwired.

Acceptance: a task's status is always derivable from the event stream, and
the GUI's retained state per task is bounded by the cap.

### Work package C — Submission and cancellation

Required changes:

1. `submit_task(variant, values)` — the IPC command:
   - Look the variant up in the catalog the GUI already holds. An unknown
     variant is an error, not an empty form.
   - Check `manual_exposable` on the operation. A non-manual-exposed operation
     is refused with an explanation.
   - Check the daemon's live capabilities: the task kind's `capability_name`
     must appear in `RuntimeCapabilities.task_kinds`. If not, refuse with a
     "not supported by the connected daemon" message. This is the
     "reported as unavailable rather than failing opaquely" half of the
     roadmap's milestone 006 exit condition.
   - Assemble and deserialize the `RunRequest` (Work package A).
   - Call `client.submit(session_id, request)`.
   - Return the `TaskId`.
2. `cancel_task(task_id)` — calls `client.cancel(session_id, task_id)`.
3. `cancel_active()` — calls `client.cancel_active(session_id)`.
4. The GUI MUST NOT surface a submit affordance for an operation it knows is
   unsupported or not manual-exposed. The control is disabled, with the
   reason shown.

Acceptance: a form submitted against a live daemon produces a running task
visible in the task list, and cancellation reaches the daemon.

### Work package D — Live progress and results

Required changes:

1. Subscribe once per session; forward events to the webview.
2. The webview renders the task list from the forwarded state, with a progress
   bar driven by `TaskProgressView::percentage` and the capped log lines
   beneath it.
3. On `TaskCompleted`, convert the outcome with `eggsec-ui-model`'s existing
   `From` implementations:
   - `TaskOutcome::Json(value)` → the generic JSON result path.
   - The structured-envelope variant → `ResultEnvelopeView::from(&envelope)`.
   - `TaskOutcome::Text(text)` → the text path.
   - `TaskOutcome::Artifact { .. }` → the artifact path.
4. The result view uses `ResultEnvelopeView::kind_label` for the heading and
   `RENDERER_REGISTRY`'s `summary_fields` for the compact summary line — the
   registry's own metadata, not a GUI-side table.
5. `supports_json_detail` controls whether the raw JSON detail is offered.
6. `supports_rich_tui` is **not** a signal to the GUI. It describes the TUI's
   rendering capability. The GUI MUST NOT read it as "the GUI cannot render
   this" — the registry's structured kinds are rendered from `payload`, and an
   unknown kind falls back to generic JSON.
7. An unknown result kind degrades to a generic JSON render, as the registry
   documents. It MUST NOT show an empty view.

Acceptance: a completed task renders a result for every registry kind and for
an unknown kind, with no GUI-side per-kind branch.

### Work package E — The JavaScript dependency policy

`ADR-0009` deferred the JavaScript-side dependency-audit policy to the shell
milestone, "where the actual frontend dependency tree exists". Milestone 004
deliberately shipped a buildless frontend with zero npm dependencies, so the
question is live here.

Required changes:

1. **Prefer the buildless path.** The form engine, the task list, and the
   result view are a few hundred lines of vanilla JavaScript. The default
   expectation is that this milestone keeps the frontend buildless and adds no
   npm dependency at all.
2. If a build step or a JavaScript dependency is nevertheless required, it
   MUST come with a recorded policy, equivalent in force to the Rust side:
   - the lockfile is committed;
   - no install script (`postinstall` and friends) is permitted in any
     dependency;
   - the dependency's license must be in a recorded allow-list;
   - the dependency tree is audited in CI, and the audit failure is fatal;
   - the policy is documented in `docs/DEPENDENCY_EXCEPTIONS.md` or a
     dedicated document, with an owner and a review-by date.
3. A remote script source or a CDN import is **not** permitted — it would
   defeat the CSP and add a supply-chain dependency to a security tool.

Acceptance: the GUI's JavaScript dependency set is empty, or the recorded
policy is in place and CI enforces it.

### Work package F — Guards, documentation, and skills

Required changes:

1. Add guards 159 and 160 to `scripts/check-architecture-guards.sh`, in the
   style of guards 151–158. **Each MUST be demonstrated to fail under a
   deliberate violation and to pass again after restore.**
2. Update the documents listed in §6.
3. Update `.opencode/skills/eggsec-gui/SKILL.md` with the form engine, the
   task state, and the result rendering.
4. Update `plans/registry.md` and the subsystem roadmap.

Acceptance: the guards pass; the documents describe the GUI as it is.

## 8. Failure, cancellation, restart, and contention semantics

- **Submission rejected by the daemon.** The daemon returns
  `ServerMessage::Error` with an `ErrorCode`. The GUI surfaces the daemon's
  message verbatim in the task list. The GUI MUST NOT retry a rejected
  submission automatically.
- **Submission rejected client-side.** A `serde_json::from_value` failure
  produces a form error naming the field and the expected type. The form stays
  open with the operator's input intact.
- **Operation not supported by the connected daemon.** The submit control is
  disabled with the reason shown, before any request is sent.
- **Operation not manual-exposed.** Same treatment, from `manual_exposable`.
- **Task fails.** `TaskFailed` carries a `RuntimeErrorInfo`. The task list
  shows the failure. The GUI MUST NOT retry.
- **Task cancelled.** `TaskCancelled` carries an optional reason. The status
  becomes cancelled.
- **Cancellation is best-effort.** `cancel_task` reports the daemon's
  response. A task that completes between the operator's click and the
  daemon's processing is not an error — the GUI reconciles from the event
  stream, which is authoritative.
- **The daemon dies mid-run.** The subscription ends. Running tasks are shown
  as "connection lost", not as failed — the distinction matters, because a
  lost connection says nothing about the task's fate in the engine. The GUI
  reconnects on operator action and creates a new session; it does not
  silently reuse a dead session id.
- **The event stream outruns the UI.** The forwarding task drains the
  receiver and updates per-task state. It MUST NOT accumulate an event queue
  proportional to the stream length, and MUST NOT spawn a task per event.
- **Log flooding.** `TaskLog` is the high-cardinality case. The per-task cap
  bounds retention. The GUI MUST also bound the *rate* it forwards log lines
  to the webview if a task emits them faster than the UI renders — a
  coalescing rule that keeps the newest lines and drops the middle, reported
  as a dropped-line count rather than silently.
- **Contention.** One GUI process, one session, one subscription. Two GUI
  processes are independent clients and independent sessions; the daemon
  arbitrates through the existing client registry.
- **Shutdown.** The GUI closes the session and the subscription cleanly.
  In-flight tasks are left running in the daemon — the GUI does not cancel
  them implicitly on exit, because that would silently abort an assessment
  the operator started.

## 9. Compatibility and migration

- **No protocol change.** No new `ClientCommand`, no new `ServerMessage`, no
  version bump.
- **No `RuntimeClient` trait change.** `describe_operations` remains an
  inherent method on `DaemonRuntimeClient`, as milestone 004 established.
- **No change to `eggsec-runtime`, `eggsec-policy`, `eggsec-daemon-protocol`,
  `eggsec-ui-model`, or the engine.** The GUI consumes published artifacts; it
  does not change them.
- The GUI's IPC surface grows from milestone 004's two commands to five. This
  is internal to `crates/eggsec-gui`.
- The workspace crate count is unchanged from milestone 004.

## 10. Required tests

### Focused unit tests

- `control_for` — one test per measured shape (required string, optional
  string, required integer, optional integer, required boolean, optional
  boolean, array of strings, the unconstrained `true` schema), plus the
  documented fallback for an unrecognized shape.
- `controls_for` — for **every** published task kind: the control count
  equals the schema's property count, and the required set matches the
  schema's `required` list.
- The aggregate: 149 controls across 30 task kinds, asserted from the
  published artifact rather than a transcribed list.
- `assemble_request` — a valid form for each of the 30 variants deserializes
  into a typed `RunRequest` with the right variant; an invalid value produces
  a `FormError` naming the field.
- The task state machine — every transition, the log cap, and the
  dropped-line counter.

### Integration tests

- The full path against a live daemon host, the way `crates/eggsec-daemon`'s
  own tests do: connect, create a `DesktopGuiManual` session, fetch the
  catalog, build a form, submit it, receive the events, render the result,
  cancel.
- The event stream is driven by the daemon's own emission, not by a
  hand-written event list, so the test breaks if the wire shape changes.

### Restart and recovery tests

- A daemon restart between submission and completion is reported as a lost
  connection, and the task is **not** reported as failed.
- Reconnecting creates a new session and does not resurrect a stale task list.

### Contention and cancellation tests

- Cancelling a running task reaches the daemon and the status becomes
  cancelled.
- Cancelling a task that has already completed is reconciled from the event
  stream, not reported as an error.
- A log flood of 10,000 lines retains at most the cap, and the dropped-line
  counter is non-zero.

### Security and negative tests

- Guard 159: a per-operation `match` over the variant names in the GUI's
  sources fails the guard. A hand-written field list fails the guard.
- Guard 160: removing the log cap fails the guard.
- A non-manual-exposed operation cannot be submitted from the GUI.
- An operation whose capability the daemon lacks cannot be submitted.
- The IPC surface exposes no command that sets the surface, sets the scope, or
  forwards an arbitrary daemon message.
- The webview cannot set the surface: `assemble_request` always stamps
  `DesktopGuiManual` in Rust.

### Migration and compatibility tests

- The catalog the GUI receives from an older daemon without
  `operation_schema` degrades to a clear "this daemon predates the schema
  publication" state rather than an empty operation list.
- The result rendering path is unchanged for every one of the 23 registry
  kinds, and an unknown kind degrades to generic JSON.

## 11. Required verification commands

```bash
# Format and lint
cargo fmt --all --check
cargo clippy -p eggsec-gui --features shell -- -D warnings
cargo clippy -p eggsec-frontend-client -p eggsec-daemon-protocol -p eggsec-daemon -- -D warnings

# Build matrix
cargo check -p eggsec-gui --no-default-features
cargo check -p eggsec-gui --features shell --all-targets
cargo check --workspace --no-default-features
cargo check --workspace --all-targets

# Contract
make check-msrv
make check-deps
bash scripts/check-architecture-guards.sh

# Tests
cargo test -p eggsec-gui --lib --features shell
cargo test -p eggsec-frontend-client --lib
cargo test -p eggsec-runtime --features schema --lib
cargo test -p eggsec-daemon-protocol --lib
cargo test -p eggsec-daemon --lib
cargo test -p eggsec-ui-model --lib

# Documentation references
python3 scripts/check_doc_references.py
```

`make check-deps` MUST pass `bans`, `licenses`, and `sources`. The `advisories`
check is known to fail at the baseline on the pre-existing `rustls 0.21.12`
family via `tiberius` → `sqlx-mysql`; that backlog is a separate maintainer
decision and is not a gate for this milestone.

If Work package E introduces a JavaScript dependency tree, the corresponding
audit command MUST be added to this list and MUST pass.

## 12. Documentation updates

- `architecture/desktop_gui.md` — **new**: the form engine, the control-kind mapping,
  the assembly path, the task state and its cap, the result rendering, the
  frontend data flow, and the JavaScript dependency policy.
- `architecture/overview.md` — the new modules and guards 159–160.
- `architecture/frontend_client.md` — the submit, cancel, and subscribe path.
- `architecture/daemon.md` — the GUI's submission and event flow.
- `architecture/runtime.md` — the event stream the GUI consumes.
- `docs/ARCHITECTURE.md` — the GUI's position in the dependency picture.
- `docs/ARCHITECTURE_INVARIANTS.md` — the descriptive-not-authoritative rule
  for the schema.
- `docs/CI_ARCHITECTURE_GUARDS.md` — guards 159–160.
- `docs/VERIFICATION.md` — any new verification command.
- `docs/DEPENDENCY_EXCEPTIONS.md` — only if Work package E introduces a
  JavaScript dependency.
- `docs/COMMAND_REGISTRY.md` — the GUI is not a CLI command; verify and leave
  unchanged.
- `.opencode/skills/eggsec-gui/SKILL.md` — the form engine and task state.

## 13. Acceptance criteria

1. A form is generated for **every** task kind the schema describes — all 30
   variants, 149 fields — from the published artifact, with no per-operation
   frontend code.
2. The form engine's control mapping covers exactly the five measured shapes
   plus a documented fallback.
3. A submitted form becomes a typed `RunRequest` and is accepted by a live
   daemon; the resulting task appears in the task list.
4. An invalid value is rejected client-side with a field-level error, using
   the same DTOs the engine deserializes.
5. Live progress, log output, and terminal status are rendered from the
   daemon's event stream.
6. A completed task renders a result for every registry kind and for an
   unknown kind, with the generic JSON degradation.
7. Cancellation works per task and for the active task.
8. Per-task retained state is bounded, and the cap is documented.
9. Operations the connected daemon does not support, and operations that are
   not manual-exposed, are reported as unavailable rather than failing
   opaquely.
10. The GUI adds no dependency on the engine, the daemon crate, the CLI, or
    the TUI, and no `tauri` edge appears outside `crates/eggsec-gui`.
11. The GUI's IPC surface sets neither the surface nor the scope, and
    forwards no arbitrary daemon message.
12. Guards 159 and 160 exist, pass, and each was demonstrated to fail under a
    deliberate violation.
13. The JavaScript dependency set is empty, or the recorded policy from Work
    package E is in place and CI enforces it.
14. Every document and skill listed in §12 is updated.
15. `python3 scripts/check_doc_references.py` reports no failure on any line
    this milestone changed.

## 14. Stop conditions

Stop and escalate rather than improvise if any of these arise:

- **The generic form engine cannot express a published schema shape.** The
  fallback path — a free-text input — exists for exactly this, but if a shape
  would silently lose information, escalate instead of shipping a lossy
  control.
- **A field needs a hand-written validator.** That is a signal the schema is
  missing something the engine has. Escalate; do not transcribe validation
  rules into the GUI, which would create the second source of truth milestone
  001 was written to eliminate.
- **The GUI appears to need engine-side access** for any reason. That is a
  boundary violation of `ADR-0007`, not an implementation difficulty.
- **The GUI appears to need to decide policy.** The schema and the catalog
  are descriptive. If the GUI is tempted to pre-authorize or pre-scope
  anything, escalate.
- **The event stream cannot be bounded.** Unbounded retention is a
  disqualifying defect, not a tuning problem.
- **A JavaScript dependency is unavoidable and its license or audit posture
  cannot be recorded** with an owner and a review-by date. Escalate.
- **Interactive policy approval turns out to be required** for the milestone
  to be usable. `ApprovePolicy` is a separate corrective milestone; escalate
  rather than widening this one.

## 15. Closure evidence required

A closure record at
`plans/closure/desktop-gui-frontend/005-schema-driven-forms-and-task-dashboard-closure.md`
following the twelve-section template in `plans/closure/README.md`, containing:

1. The requirement-to-evidence matrix, with a file:line citation or a command
   output for every acceptance criterion in §13.
2. The production implementation evidence: the form engine's module
   inventory, the control mapping table, the task-state module, and the
   frontend's data flow.
3. The verification actually run, with real output.
4. The invariant review, addressing each invariant in §4 by name.
5. The failure and recovery review, addressing §8.
6. The migration and compatibility review, addressing §9.
7. The security review, addressing §6's security requirements.
8. The documentation and operations section, listing every document and skill
   changed, and the JavaScript dependency policy — or an explicit statement
   that the dependency set is empty.
9. The unresolved findings table.
10. The roadmap disposition and the registry updates.

Additional milestone-specific evidence:

- The aggregate control-count assertion (30 variants, 149 fields), with its
  output.
- The guard-failure demonstrations for guards 159 and 160.
- The bounded-retention evidence: the cap, and the dropped-line counter under
  a flood.
- A screenshot or transcript of a form, a live task, and a rendered result
  against a live daemon.

## 16. Handoff notes

- **Milestone 006 inherits a generic engine.** By the time 005 closes, adding
  an operation to the engine requires no GUI change at all: the schema
  publication is derived, so the form appears automatically. Milestone 006's
  work is coverage and export, not form authoring. If 006 finds itself writing
  a per-operation form, 005 did not land.
- **The three-layer validation model is the design to preserve.** Layer one is
  advisory, from the schema. Layer two is the typed `RunRequest`
  deserialization, which is exact. Layer three is the engine's own validation,
  which is authoritative. The GUI's layers one and two exist to save the
  operator a round trip; they never stand in for layer three.
- **`describe_operations` stays an inherent method.** Promote it to a narrow
  `OperationCatalogClient` trait only when a second daemon-only frontend
  appears.
- **The event stream is the source of truth for task state.** The GUI's task
  map is a bounded view of it. Any future feature that needs the full history
  must get it from the daemon's persistence, not from the GUI's memory.
- **The `supports_rich_tui` flag is about the TUI.** It is not a capability
  query for the GUI. The registry's structured kinds are rendered from
  `payload`; unknown kinds degrade to generic JSON.
- **`ADR-0007` still holds.** The GUI's only route to the engine is the daemon
  socket. If a feature seems to need more, the answer is a new daemon
  command, not a new dependency.
- **The JavaScript question is now answered.** Milestone 005 either ships a
  dependency-free frontend or ships a policy with it. Either way the question
  is closed, and `ADR-0009`'s deferral is discharged.
