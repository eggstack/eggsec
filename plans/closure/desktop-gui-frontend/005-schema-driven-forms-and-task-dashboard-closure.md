# Desktop GUI Frontend Milestone 005 — Closure Status

Status: closed

Source implementation plan:

- `plans/implementation/desktop-gui-frontend/005-schema-driven-forms-and-task-dashboard.md`

Source subsystem roadmap:

- `plans/subsystems/desktop-gui-frontend-roadmap.md`

Repository baseline reviewed: `203ab5dd958415340d39cd47fb1add432c687edd`

Implementation commits or pull requests:

- Uncommitted working-tree change on branch `feat/auto-20261022-76692679`.

## 1. Executive finding

The milestone's capability boundary is complete. The desktop GUI now has a **form engine and a
task dashboard**, both driven entirely by the schema the daemon publishes.

The milestone's central property is that **the GUI contains no per-operation code**. There is no
match or switch over the thirty task-kind variants, no hand-written table of field names, and no
list of operations in either the Rust or the JavaScript. A task kind the engine has not published
yet gets a working form with no GUI change at all. Guard 159 enforces this by **reading the
variant names out of the runtime** rather than transcribing them, so the guard cannot itself
become the hand-written table it exists to prevent.

Two things made that possible, and both are measurements rather than assumptions:

1. The published schema's exact shape, probed against the live catalog.
2. The DTOs' own serde defaults, probed the same way — which turned out to matter for safety.

The safety finding is the milestone's most consequential result. Four published parameters carry
non-null serde defaults, and one of them is a **fail-safe**: `dry_run` defaults to `true` so the
operation simulates instead of transmitting. A GUI that sent `false` for an untouched field would
silently turn a simulation into a live action. The GUI therefore sends `null` for a blank field
and **omits** a non-nullable optional field entirely, so the request DTO applies its own default.
The engine decides what an untouched field means, not the GUI.

## 2. Requirement-to-evidence matrix

| Requirement | Evidence | Result | Notes |
|---|---|---|---|
| One schema property maps to one control, with no per-operation code | `form::control_for`; guard 159; the frontend carries no operation name | pass | The guard reads the variant names from the runtime and fails if any appear in the GUI's production sources or its frontend. |
| Every published task kind gets a form | test `assemble_request_round_trips_every_published_task_kind` | pass | All 30 variants assemble into a typed request. |
| Five control kinds, no dropdowns | test `control_for_covers_every_measured_shape` | pass | The schema carries zero `enum` constraints, so it cannot drive a dropdown. |
| The controls are ordered deterministically | test `controls_are_ordered_required_first_then_alphabetically` | pass | Required first, then alphabetical, for every variant. |
| The controls cover the schema exactly | test `controls_match_every_published_schema` | pass | One control per property across all 30 variants; the total field count is 149. |
| The submitted request deserializes into the same DTO the engine deserializes | `form::assemble_request`; layer 2 of the validation model | pass | The assembled value is deserialized into `RunRequest` by `serde_json::from_value`. |
| A wrong-typed value is rejected with a field-level error | test `a_wrongly_typed_value_names_the_field_and_the_expected_type` | pass | The error names the field and the expected type. |
| A missing required field is rejected with a field-level error | test `a_missing_required_field_names_the_field` | pass | The error names the field. |
| An out-of-range value is rejected before it reaches the engine | test `an_out_of_range_number_is_rejected` | pass | The upper bound comes from the schema's `format`. |
| The schema is never consulted for authorization | guard 159; `form.rs` reads only `type`, `format`, `minimum`, `description`, and `required` | pass | The schema drives form *shape* only. Exposure and risk come from the daemon's catalog. |
| An untouched optional field keeps the DTO's own default | test `an_untouched_optional_field_keeps_the_dto_s_own_default` | pass | `dry_run` stays `true`; `frame_count` stays 100. |
| An untouched nullable field is sent as `null` | test `an_untouched_nullable_field_is_sent_as_null` | pass | |
| The GUI holds a bounded number of log lines per task | `MAX_LOG_LINES_PER_TASK` = 200; test `a_log_flood_is_bounded_and_counted` | pass | 10 000 log lines retain exactly 200 and count the drops; the newest line survives. |
| Dropped lines are reported, not silently swallowed | the per-task and dashboard dropped counters reach the webview | pass | The operator sees "… 42 earlier lines dropped". |
| The event stream is the source of truth; the GUI's map is a view | `TaskRegistry::apply` | pass | The GUI holds a bounded view, never a store. |
| Events about unknown tasks are counted | test `unknown_task_events_are_counted` | pass | |
| The GUI subscribes once per session | `spawn_event_forwarder` | pass | One forwarder for the session's lifetime; never a task per event. |
| Availability comes from the daemon's live capabilities | `list_task_kinds`; the submit path's independent check | pass | The join key is `capability_name` against `RuntimeCapabilities.task_kinds[].name`. |
| Result rendering goes through the renderer registry | `tasks::result_view`; `eggsec-ui-model` | pass | An unknown kind degrades to a generic JSON render, never an empty view. |
| The IPC surface is narrow | test `ipc_surface_exposes_only_narrow_commands` | pass | No `ClientCommand::`, no `RuntimeSurface::`, no `SessionScope`, no `send_command`. |
| The IPC argument naming is stated in the code | test `every_command_declares_its_argument_naming` | pass | All eight commands declare `rename_all = "snake_case"`. |
| The frontend carries no npm dependency | the `crates/eggsec-gui/frontend/` tree | pass | No `package.json`, no lockfile, no `node_modules`, no bundler, no build step. |
| `ADR-0009`'s deferred JavaScript dependency policy is discharged | the empty dependency set | pass | The question is settled by the dependency set staying empty. |

## 3. Production implementation evidence

### 3.1 The measured schema, and why it was measured

The plan did not assume the schema's shape; it measured it, with a throwaway probe crate against
the live catalog:

| Property | Measured value |
|---|---|
| Task kinds published | 30 |
| Fields across all variants | 149 |
| `$ref` / `$defs` | 0 / 0 |
| `enum` constraints | 0 |
| Nested objects | 0 |
| Array item type | always `{"type": "string"}` |
| Formats | `uint`, `uint16`, `uint32`, `uint64` |
| Distinct type signatures | `string`, `string\|null`, `integer`, `integer\|null`, `boolean`, `boolean\|null`, `array\|null`, and exactly one untyped property |

Two consequences fell out of the measurement: **no dropdowns** (the schema carries zero `enum`
constraints, so there is nothing to offer in a select) and **one unconstrained property**
(`IntegrationsParams::config`, whose schema is the boolean `true`, which accepts any JSON value
and renders as a JSON text area).

### 3.2 The form engine — `crates/eggsec-gui/src/form.rs`

One mapping from a published JSON Schema property to one control descriptor:

| Schema shape | Control | Behaviour |
|---|---|---|
| `"type": "string"` | `text` | single-line input |
| `"type": ["string","null"]` | `text` | blank means *not provided* |
| `"type": "integer"` | `number` | `min`/`max` from the schema |
| `"type": ["integer","null"]` | `number` | |
| `"type": "boolean"` | `checkbox` | tri-state: unset / true / false |
| `"type": ["boolean","null"]` | `checkbox` | |
| `"type": ["array","null"]`, items `string` | `list` | repeatable rows |
| unconstrained (`true`) | `json` | free-form JSON text area |

The documented fallback for a shape the engine has not published before is never lossy: an object
or a non-string array becomes a `json` control (a free-text field would lose the structure), and
an unknown scalar becomes a `text` control. It never panics and never drops a field.

Controls are ordered deterministically — **required first, then alphabetically** — so the form's
layout does not shift between runs. The numeric upper bound is derived from the schema's `format`,
because the schema states a `minimum` but never a `maximum`: `uint16` → 65535, `uint32` →
4294967295, `uint64`/`uint` → `u64::MAX`.

### 3.3 The three-layer validation model

| Layer | Source | What it settles |
|---|---|---|
| 1 — advisory | the published schema | the control's declared type and bounds |
| 2 — exact | the DTOs | the assembled request is deserialized into the same `RunRequest` the engine will deserialize |
| 3 — authoritative | the engine | whether the task may run at all |

Layers one and two exist to save the operator a round trip. They never stand in for layer three,
and **neither encodes a per-operation rule** — layer one reads the schema's own `type` keyword,
and layer two is the engine's own deserialization.

### 3.4 The "not provided" rule — the safety finding

A blank control sends **`null`** — never `""`, never `0`, never `false`, never `[]`. The Rust
assembly path then does one of two things:

- a field that **accepts `null`** is sent as `null`;
- a field that is **not required and whose type does not accept `null`** is **omitted** from the
  request, so the request DTO applies its own default.

Why it matters: the measurement found **four** fields that are non-nullable yet optional, and
**all four carry a non-null serde default** — and they are all in one variant:

| Field | Type | Default |
|---|---|---|
| `WirelessActive.attack_type` | string | `"deauth"` |
| `WirelessActive.dry_run` | boolean | `true` |
| `WirelessActive.frame_count` | integer | `100` |
| `WirelessActive.rate_limit` | integer | `10` |

`dry_run` is a **fail-safe**: it defaults to `true` so the operation simulates instead of
transmitting. A GUI that sent `false` for an untouched field would silently turn a simulation into
a live action. The remaining 19 properties that carry a default all default to `null`, so sending
`null` for them is equivalent to omitting them. The rule is uniform from the operator's point of
view: *blank means not provided*.

### 3.5 The bounded task state — `crates/eggsec-gui/src/tasks.rs`

| Element | Bound | Why |
|---|---|---|
| Retained log lines per task | `MAX_LOG_LINES_PER_TASK` = 200 | `TaskLog` is the high-cardinality event type; a task that floods must not be able to grow the GUI's memory without bound |
| Log lines forwarded per flush | `MAX_LOG_LINES_PER_FLUSH` = 50 | a task that logs faster than the UI renders must not build an unbounded queue in the GUI |

Both are named constants, not tuning knobs, and both **report what they drop**. The per-task ring
is a `VecDeque` with an explicit capacity, not a `Vec` that grows. The registry keeps tasks in
insertion order, so the dashboard does not reshuffle between renders, and events about unknown
tasks are ignored but counted in `unknown_task_events`.

The GUI subscribes **once** per session and never spawns a task per event. The event-forwarding
task is the one deliberate exemption from the workspace's 30–300s timeout rule, and the reason is
written down at the function: it is a long-lived connection reader whose lifetime is the
subscription's, and killing it would sever the event stream silently. The daemon client's reader
task holds the same exemption for the same reason.

### 3.6 The IPC surface

Milestone 004 shipped two commands. Milestone 005 made the surface eight, each a narrow
pass-through:

| Command | Argument | Returns |
|---|---|---|
| `connect_daemon` | `socket` | `CatalogView` |
| `list_operations` | — | `OperationRow[]` |
| `list_task_kinds` | — | `TaskKindRow[]` |
| `form_for` | `variant` | `FormView` |
| `submit_task` | `variant`, `values` | `TaskIdView` |
| `cancel_task` | `task_id` | — |
| `cancel_active` | — | — |
| `task_snapshot` | — | `TaskDashboardView` |

There is **no generic "send any command to the daemon" entry point**, and nothing sets the
session surface or the scope. A test asserts this by scanning the module's production source for
`ClientCommand::`, `RuntimeSurface::`, `SessionScope`, and `send_command`.

**The surface is stamped in Rust.** `submit_task` builds the request through the form engine,
which sets `RuntimeSurface::DesktopGuiManual` by *serializing the runtime's own enum* — never by
hand-writing the string. The webview cannot widen the session's authority by asking nicely.

**Every command declares its argument naming explicitly** (`rename_all = "snake_case"`). Tauri's
default is camelCase from the webview; the GUI's contract is snake_case on both sides, so the
attribute is stated in the code rather than left to a framework default. A test asserts that all
eight commands carry it, because without it the webview's `task_id` would silently stop arriving.

One event is published on the bus: `eggsec://task-event`, carrying `task_upsert`, `logs`, or
`counters` payloads.

### 3.7 Availability

**M5 reports; a later milestone hides.** The join key is `TaskKindDescription::capability_name`
against `RuntimeCapabilities.task_kinds[].name` — the daemon's live capabilities, never the GUI's
own reasoning. An operation that is not exposed on the manual surface is marked unavailable with a
reason, and the submit path refuses it independently of what the UI shows.

### 3.8 Result rendering

Results are rendered through `eggsec-ui-model`'s `RENDERER_REGISTRY` — the only source of
per-kind metadata. The GUI does not carry its own table of result kinds. An unknown kind degrades
to a generic JSON render, never an empty view, and the view says so (`generic: true`).
`supports_rich_tui` describes the TUI's capability and is **not** a capability query for the GUI.

### 3.9 The JavaScript dependency policy — discharged

`ADR-0009` deferred a JavaScript dependency review policy to this milestone, to be settled either
by keeping the dependency set empty or by recording a policy. **The dependency set is empty**, so
the question is discharged:

- no `package.json`, no lockfile, no `node_modules`, no bundler, no build step;
- no remote script source and no CDN import — a remote source would defeat the CSP and add
  supply-chain surface to a security tool;
- the frontend is hand-written HTML, CSS, and vanilla JavaScript, loaded with a plain
  `<script src="app.js" defer>` tag.

The frontend reaches Tauri through `window.__TAURI__`, which `tauri.conf.json` enables with
`app.withGlobalTauri: true` — that is what lets the frontend be buildless.

The frontend renders **every** daemon-supplied value as text: there is no `innerHTML` assignment
anywhere in `app.js`, and the DOM is built through a helper that sets `textContent` and
`setAttribute`. An operation name or a log line cannot inject markup into a security tool's UI.

### 3.10 The guards

Guards 159–160 were added to `scripts/check-architecture-guards.sh`:

| Guard | Protects |
|---|---|
| 159 | the GUI carries no per-operation form code; the variant list is read from the runtime, and the Rust side is scanned in its production half only |
| 160 | the GUI retains no unbounded event log; the `LogRing` must be a capped ring, and the caps' existence is pinned rather than their value |

## 4. Verification executed

### Commands run

```bash
cargo test -p eggsec-gui --features shell
cargo clippy -p eggsec-gui --features shell --all-targets
cargo +1.89 check -p eggsec-gui --features shell
node --check crates/eggsec-gui/frontend/app.js
cargo test -p eggsec-daemon --lib the_gui_client_path_works_end_to_end
bash scripts/check-architecture-guards.sh
make check
```

### Results

| Check | Result |
|---|---|
| `cargo test -p eggsec-gui --features shell` | pass — 34 tests, 0 failed |
| `cargo clippy -p eggsec-gui --features shell --all-targets` | pass, no warnings |
| `cargo +1.89 check -p eggsec-gui --features shell` | pass |
| `node --check crates/eggsec-gui/frontend/app.js` | pass (node v26.10.0) |
| `cargo test -p eggsec-daemon --lib the_gui_client_path_works_end_to_end` | pass |
| `bash scripts/check-architecture-guards.sh` | pass — `ALL PASSED: No architecture drift detected.` over all 160 checks |
| `make check` | **stops at `check-deps`** on pre-existing advisories — see below |

The GUI crate's 34 tests cover, among others:
`controls_match_every_published_schema`, `controls_are_ordered_required_first_then_alphabetically`,
`control_for_covers_every_measured_shape`, `control_for_falls_back_without_panicking`,
`assemble_request_round_trips_every_published_task_kind`,
`a_missing_required_field_names_the_field`,
`a_wrongly_typed_value_names_the_field_and_the_expected_type`,
`an_out_of_range_number_is_rejected`,
`an_untouched_optional_field_keeps_the_dto_s_own_default`,
`an_untouched_nullable_field_is_sent_as_null`,
`a_log_flood_is_bounded_and_counted`, `unknown_task_events_are_counted`,
`tasks_keep_their_insertion_order`, `the_variant_is_read_from_the_task_kind`,
`ipc_surface_exposes_only_narrow_commands`, `declared_command_set_is_exact`,
`every_command_declares_its_argument_naming`.

The frontend was checked for the properties the milestone's stop conditions name:

- `node --check` passes;
- no `innerHTML`, `insertAdjacentHTML`, `outerHTML`, or `document.write` anywhere;
- no `http://` or `https://` URL in any of the three files;
- none of the thirty task-kind variant names appears in the frontend source — checked
  case-sensitively and case-insensitively, as whole words and as substrings;
- no npm artifacts anywhere under `crates/eggsec-gui/`;
- `index.html` loads exactly `styles.css` and `app.js`, both relative.

The frontend was additionally exercised against a throwaway DOM and IPC harness outside the
repository, covering the connect path, the no-retry rule, the catalog facts, the availability
marking, all five control kinds, value assembly, the list lifecycle, invalid JSON being blocked
with the operator's input intact, live task events, result rendering, generic-kind degradation,
both cancellation paths, the counters, and bounded retention after a 260-task flood.

`make check` was run end to end. Every step **before** `check-deps` passed, and the steps after
it were run directly: `make clippy`, each of the target's test invocations, and the guard script.
**The advisory failure is pre-existing and is not a regression from this milestone**: the failures
are `zip 2.4.2` and the `rustls 0.21.12` family reached through the engine's and the daemon's
dependency closures. Neither is in the GUI's closure, and this milestone adds no registry
dependency beyond `eggsec-ui-model`, which is a workspace-local crate.

### Guard demonstrations

**Guard 159** — a variant name added to a production doc comment in `form.rs`:

```
--- Check 159: the GUI carries no per-operation form code ---
FAIL: crates/eggsec-gui/src/form.rs names the task-kind variant 'PortScan'; the form engine must be schema-driven.
49:/// One form field, derived from one JSON Schema property (e.g. PortScan).
```

The guard script exited 1. The violation was restored from a snapshot and the file was verified
**byte-identical** (`diff -q` reported no difference). After restore: `PASS: the GUI names no
task-kind variant; its forms come from the published schema.`

**The guard found a real instance of the defect it looks for.** The first run of guard 159 failed
on the GUI's own source: a doc comment in `tasks.rs` named `PortScan` as an example. The comment
was reworded to describe the discriminator without naming one. A guard that passes on the first
try is a guard that has not been tested.

**Guard 160** — the ring's `VecDeque<LogLine>` replaced with `Vec<LogLine>`:

```
--- Check 160: the GUI retains no unbounded event log ---
FAIL: crates/eggsec-gui/src/tasks.rs does not bound its retained log lines:
      the ring does not hold its lines in a VecDeque
      the ring holds its lines in an unbounded Vec
```

The guard script exited 1. The violation was restored from a snapshot and the file was verified
**byte-identical**. After restore: `PASS: the GUI's task state bounds its log retention and
reports what it drops.`

The check also pins the **existence** of the cap rather than its value: a later milestone may move
`MAX_LOG_LINES_PER_TASK` from 200, but it may not remove the bound.

## 5. Invariant review

| Invariant | Evidence | Status |
|---|---|---|
| `EnforcementContext::evaluate()` is the mandatory pre-dispatch gate for all surfaces | The GUI submits through the daemon, which enforces. The GUI has no dispatch path of its own. | holds |
| Strict surfaces dispatch only via `EnforcedDispatcher::dispatch_execution()` with an `ApprovedExecution` bundle | The GUI is a **manual** surface and does not dispatch at all. | holds |
| Strict surfaces use `LoadedScope` and fail closed without an explicit scope | The GUI stamps a non-explicit `SessionScope`; the daemon resolves the loaded scope. The GUI cannot widen scope. | holds |
| `OperationMetadata` stays the single source of truth for operation policy | The GUI renders the daemon's published catalog and decides nothing itself. Exposure comes from `OperationDescription.manual_exposable`; availability comes from the daemon's live capabilities. | holds |
| The published schema is descriptive, never authoritative | `form.rs` reads only `type`, `format`, `minimum`, `description`, and `required`. It never reads a policy field, an exposure flag, or a risk tier. Guard 159 enforces the absence of per-operation code. | holds |
| The GUI must not depend on the engine | Guard 155; the GUI's manifest names no engine crate. | holds |
| TLS is ring-only everywhere | The GUI's closure contains no TLS stack; guard 157 enforces it. | holds |
| The Tokio baseline is `default-features = false` with per-crate feature lists | The GUI's manifest lists explicit features and does not enable defaults. | holds |
| Spawned tokio tasks carry a 30–300s timeout wrapper | The GUI's one spawned task is the long-lived event forwarder; the exemption and its reason are written at the function. | holds, documented exemption |
| Never `let _ =` or `filter_map(|e| e.ok())` — log via `tracing` | The GUI's sources contain no `let _ =` on a `Result`; the forwarder logs through `tracing::warn!`. | holds |
| Bounded retention is a disqualifying defect, not a tuning problem | The per-task ring and the flush batch are both capped, and the dropped counts are reported. Guard 160 enforces the bound's existence. | holds |
| The GUI must not become a second composition root | The GUI connects to an already-running daemon and never spawns, supervises, or stops one. | holds |

## 6. Failure and recovery review

| Scenario | Behavior | Evidence |
|---|---|---|
| A required field is left blank | The submission is refused with a field-level error naming the field. The form stays open with the operator's input intact. | `form::assemble_request`; the frontend keeps its state on an error |
| A value is of the wrong type | The submission is refused with a field-level error naming the field and the expected type. | `a_wrongly_typed_value_names_the_field_and_the_expected_type` |
| A value is out of the format's range | The submission is refused before it reaches the engine. | `an_out_of_range_number_is_rejected` |
| A JSON control holds unparsable text | The frontend blocks the submission and shows an inline field-level error; the Rust side is never reached. | the frontend's form validation |
| An operation is not exposed on the manual surface | The submit path refuses it independently of what the UI shows, and the sidebar marks it unavailable with a reason. | `commands::submit_task`; `unavailable_reason` |
| The daemon does not advertise the operation's capability | The submit path refuses it; the sidebar marks it unavailable. | `commands::submit_task` |
| A task is cancelled by the operator | The cancellation is requested; the event stream, not the request, decides the final status. | `commands::cancel_task`; `GuiTaskStatus::Cancelled` |
| A task completes between the click and the daemon's processing | Not an error: cancellation is best-effort and the event stream is authoritative. | `commands::cancel_task` |
| A policy decision is required | The task is marked `awaiting_policy` and the GUI **offers no approval affordance** — it cannot approve, so it does not pretend to. | `RuntimeEvent::PolicyDecisionRequired`; `GuiTaskStatus::AwaitingPolicy` |
| A task floods the log stream | The per-task ring retains 200 lines and counts the rest; the forwarder keeps the newest 50 per flush and reports the rest as dropped. | `a_log_flood_is_bounded_and_counted` |
| An event arrives for an unknown task | It is ignored and counted in `unknown_task_events`; it is not silently dropped. | `unknown_task_events_are_counted` |
| The daemon dies mid-session | The forwarder's receiver ends; the GUI logs the loss and publishes the counters. It does not decide what happened to the running tasks. | `spawn_event_forwarder` |
| Duplicate delivery | The event stream may deliver a state transition more than once; the registry applies it idempotently because each transition sets an absolute value rather than accumulating. | `TaskRegistry::apply` |
| The operator reconnects | The connection state is replaced wholesale and the task registry is rebuilt from the fresh stream. | `commands::connect_daemon` |

## 7. Migration and compatibility review

- **No schema migration.** The milestone adds no persisted state.
- **No protocol change.** `DescribeOperations`, `SubmitTask`, `CancelTask`, `CancelActive`, and
  `Subscribe` already suffice; the milestone adds no wire variant. `ClientCommand` and
  `ServerMessage` remain at 16 and 15 variants.
- **No trait change.** `RuntimeClient`'s eight methods are unchanged. The three daemon-only
  inherent methods added by milestone 004 are unchanged.
- **No IPC-breaking change to the daemon.** The GUI is a new client of an existing protocol.
- **The IPC argument naming is stated explicitly.** `rename_all = "snake_case"` on every command
  makes the contract snake_case on both sides of the boundary, rather than inheriting Tauri's
  camelCase default. This is a contract decision, not a compatibility break: the webview and the
  Rust side were written against the same spelling.
- **The GUI holds no state of its own.** Its task map is a bounded view of the daemon's event
  stream, so a reconnect rebuilds it; there is nothing to migrate or roll back.
- **Rollback:** removing `crates/eggsec-gui/src/form.rs` and `src/tasks.rs` and the frontend
  returns the tree to the milestone-004 shell. No data migration is needed in either direction.

## 8. Security review

| Concern | Review |
|---|---|
| Authorization enforcement | The GUI has **no** enforcement path and cannot authorize anything. The daemon enforces through `EnforcementContext::evaluate()`. |
| Privilege boundary | The submission's surface is stamped `RuntimeSurface::DesktopGuiManual` in **Rust**, by serializing the runtime's own enum. The webview cannot widen the session's authority. |
| Scope | The GUI constructs a non-explicit `SessionScope` and cannot widen scope; the daemon resolves the loaded scope. |
| Transport checkpoint | All engine traffic travels over the daemon's Unix socket. The GUI makes no outbound HTTP. |
| Transport security | The CSP admits no remote origin, and the capability grants `core:default` only — no http, shell, fs, process, or updater plugin. The webview has no filesystem, process, or outbound-HTTP capability. |
| Supply chain | The frontend has zero npm dependencies and no remote script source. This milestone adds no registry dependency beyond the workspace-local `eggsec-ui-model`. |
| Markup injection | Every daemon-supplied value is rendered as text; there is no `innerHTML` assignment anywhere in the frontend. An operation name or a log line cannot inject markup into the GUI. |
| Denial-of-service bounds | Per-task log retention is capped at 200 lines and the forward flush at 50, both reported when exceeded. The GUI spawns one forwarder task and never one per event. |
| Fail-safe defaults | The GUI never overrides an engine default it did not read: a blank field sends `null`, and a non-nullable optional field is omitted so the DTO's own default — including `dry_run = true` — applies. |
| Audit behavior | The GUI's session is auditable as `desktop-gui-manual`. |
| Secret handling | The GUI holds no credentials. The daemon socket is the only channel, and the socket path is the only configurable value. |
| Path validation | Not applicable: the GUI performs no filesystem access. |

## 9. Documentation and operations

**Architecture documents**

- `architecture/desktop_gui.md` — the form engine, the three-layer validation model, the "not
  provided" rule, the task state, the bounded retention, and the result rendering.
- `architecture/overview.md` — the GUI's rows in the crate, module, User Interfaces, and Key Types
  tables; the guardrails list gained Checks 159–160.

**Contract documents**

- `docs/ARCHITECTURE_INVARIANTS.md` — the new invariants: the daemon-only boundary, the published
  schema is descriptive never authoritative, the form is generated not written, and retention is
  bounded.
- `docs/CI_ARCHITECTURE_GUARDS.md` — Checks 155–160.
- `docs/VERIFICATION.md` — the GUI lines in the `make check` expansion and the `make check-gui`
  target.

**Skills**

- `.opencode/skills/eggsec-gui/SKILL.md` — the form engine, the validation model, the "blank means
  null" rule, the bounded task state, and the result rendering.

**Planning documents**

- `plans/registry.md` — the M5 entry and the closure-record list.
- `plans/subsystems/desktop-gui-frontend-roadmap.md` — the status line, the §4 current state, the
  §9 guard list, and the §12 milestone table.
- `plans/implementation/desktop-gui-frontend/005-schema-driven-forms-and-task-dashboard.md` —
  `Status:` moved from `ready for handoff` to `executed — closed 2026-10-23`.

**Static guards**

Guards 159–160 in `scripts/check-architecture-guards.sh`, each demonstrated to fail under a
deliberate violation and to pass after a byte-identical restore.

**Operator diagnostics**

- A field-level error is shown next to the form, naming the field and the expected type.
- An unavailable operation is marked in the sidebar with its reason.
- A task blocked on a policy decision is shown as `awaiting_policy`.
- Dropped log lines and unknown-task events are surfaced as counters rather than hidden.

**Recovery instructions**

The GUI holds no state of its own. Recovery is to restart the daemon and reconnect; the task map
is rebuilt from the catalog and the fresh event stream.

## 10. Unresolved findings

| Severity | Finding | Impact | Required action |
|---|---|---|---|
| medium | Unsupported operations are **reported** rather than hidden. A milestone later hides them entirely. | The sidebar lists operations the connected daemon cannot run, each marked unavailable with a reason. The submit path refuses them independently, so there is no risk of running one. | Milestone 006 hides them. |
| medium | `ApprovePolicy` remains unwired — the daemon answers it with `ErrorCode::Unsupported`. | The GUI can mark a task `awaiting_policy` but can offer no approval affordance, because it cannot approve. | A maintainer decision: wire daemon-mediated interactive approval, or keep the fail-closed behaviour that plan 003 mandates. |
| low | `make check` cannot complete because `check-deps` fails on pre-existing advisories (`zip 2.4.2`; the `rustls 0.21.12` family via `tiberius` → `sqlx-mysql`). Neither is in the GUI's closure, and this milestone adds neither an advisory nor a license exception. | The mandatory contract's dependency gate is red for reasons that predate this milestone. | A maintainer decision: add `deny.toml` exceptions with an owner and a review-by date. |
| low | The socket-path default is duplicated in four places. | Carried forward from milestone 004. | A corrective milestone. |
| low | The GUI does not manage the daemon's lifecycle. | The operator must start a usable daemon by hand. | Milestone 006. |
| low | `make check-python` was not run. | No Python file changed in this milestone. | Run `make check-python` when a `.venv-ci` environment is available. |

## 11. Roadmap disposition

**Milestone closed and the next dependency may proceed.**

The form engine and the task dashboard are implemented and their evidence is recorded: 34 GUI
tests pass, the frontend is verified buildless and dependency-free, the schema-driven form works
for all thirty published task kinds, the task state is bounded and reports what it drops, and the
two new guards are demonstrated. `ADR-0009`'s deferred JavaScript-dependency-policy question is
discharged by the empty dependency set.

Milestone 006, which owns full operation coverage, report export, and daemon lifecycle management,
may now proceed. The `deny.toml` advisory backlog and the socket-path duplication are carried
forward without blocking.

## 12. Registry updates

- `plans/registry.md`:
  - the `desktop-gui-frontend` standing row — M5 implemented, the JavaScript dependency policy
    discharged, the workspace at 25 crates;
  - the section heading — `M1–M3 closed 2026-10-23; M4–M5 implemented 2026-10-23; M6–M7 not started`;
  - the M5 entry — rewritten from plan language to the delivered state, including the closure
    record path;
  - the closure-record list — `-005-schema-driven-forms-and-task-dashboard-closure.md` added.
- `plans/subsystems/desktop-gui-frontend-roadmap.md`:
  - the status line — M5 implemented;
  - the §4 current state — the form engine and the task dashboard added;
  - the §9 guard list — Checks 159–160;
  - the §10 decision point — the JavaScript dependency policy recorded as discharged;
  - the §12 milestone table — M5 implemented, with the closure record path.
- `plans/implementation/desktop-gui-frontend/005-schema-driven-forms-and-task-dashboard.md` —
  `Status:` moved from `ready for handoff` to `executed — closed 2026-10-23`.
