# Desktop GUI Frontend Milestone 011 — Daemon-mediated Policy Approval

Status: ready for handoff

Repository baseline: branch `feat/auto-20261022-76692679` at commit `6f75853b`
("docs(plans): record the closure of the desktop GUI milestone 010"), with
milestones 001–010 closed, conditionally closed, or implemented, and ADR-0011
accepted.

Source roadmap:

- `plans/subsystems/desktop-gui-frontend-roadmap.md#milestone-11--daemon-mediated-policy-approval`

Authoritative decision:

- `plans/adrs/ADR-0012-daemon-mediated-policy-approval.md` (status: proposed)
- `plans/adrs/ADR-0011-daemon-manual-capability-policy.md` (status: accepted)

Related long-term requirements:

- `plans/000-long-term-specification.md#4-enforcement-invariants`
- `plans/000-long-term-specification.md#6-dispatch-runtime-and-wire-identity`
- `plans/000-long-term-specification.md#10-frontend-rules`

Primary class: capability

## 0. Baseline and supersession record

This is a **new** plan, not an amendment: the 16 numbered sections below are
the whole of it, and nothing here is executed.

| # | Earlier plan said | Now true | Superseded by |
|---|---|---|---|
| S1 | "the confirmation path (milestone 011) … is deferred because wiring it changes the daemon's execution semantics rather than its advertisement" (`010`, §0) | The confirmation path is wired. The daemon holds an operation at its confirmation point, exposes the decision, accepts the answer, and dispatches the held operation | 011 |
| S2 | "`ApprovePolicy` … is refused with `ErrorCode::Unsupported`" (`plans/adrs/ADR-0011`, §5) | The handler resolves the runtime's pending approval. The refusal survives only for a task with nothing pending, and then it is `ErrorCode::InvalidState` | 011 |
| S3 | "the desktop GUI cannot complete an operation whose policy demands confirmation" (measured, §3 below) | The GUI shows the prompt, the operator answers, and the held operation proceeds | 011 |
| S4 | "`RuntimeEvent::PolicyDecisionRequired` … has no producer" (measured, §3 below) | The engine's runtime bridge produces it, and the GUI consumes it | 011 |

**The dependency this milestone waits on is ADR-0012, and ADR-0012 is
`Status: proposed`.** The roadmap states milestone 11's dependencies as
"`ADR-0011` (**hard** — met, accepted 2026-10-24)" and "`M6` (soft — the GUI's
task state already models a policy prompt)". ADR-0011 is met. ADR-0012 is this
milestone's own decision record, written because the decision — *where the hold
lives and what an answer means* — affects the runtime's public contract, the
daemon's IPC behaviour, the policy kernel's vocabulary, and the frontend
client's trait. §14 requires the implementer to stop and report if ADR-0012 is
amended before acceptance rather than follow this plan's reading of it.

## 1. Objective

Let a **manual daemon client** answer a pending policy confirmation, and let the
**held operation then run**.

Concretely, four things must become true:

1. The runtime can **hold** an operation at its confirmation point and expose
   the decision to clients.
2. The daemon's `ApprovePolicy` command **answers** that hold instead of
   refusing it.
3. The frontend-neutral `RuntimeClient` contract **exposes** the answer.
4. The desktop GUI **offers the affordance** to give it.

The answer is bounded by the policy: a client that answers "yes" grants exactly
the confirmation classes the enforcement kernel demanded for that operation, and
nothing more. The client names *yes* or *no*; it never names the authorisation.

## 2. Why this milestone is ready

- ADR-0011 is **accepted** and its §6 mandate ("the daemon must be able to hold
  an operation at its confirmation point, expose that decision to the client,
  accept the client's answer, and dispatch the held operation") is this
  milestone's obligation.
- Milestone 010 made the daemon publish and admit every canonical capability
  name for manual execution, so the advertisement half of the problem is
  settled and the remaining gap is purely the confirmation path.
- The vocabulary already exists and is unused:
  `RuntimeEvent::PolicyDecisionRequired`, `PolicyPrompt`,
  `GuiTaskStatus::AwaitingPolicy`, `ClientCommand::ApprovePolicy`,
  `ClientRole::Approver`, `CommandPermission::Approver`,
  `PolicyPromptView`, `ClientRoleView::can_approve_policy`. Five of the seven
  surfaces are already modelled; only the *behaviour* is missing.
- The enforcement kernel already returns the required confirmation classes
  (`EnforcementError::ConfirmationRequired { decision, required_classes }`), so
  the bound is computable without inventing policy.
- The TUI already implements the same operator flow locally
  (`crates/eggsec-tui/src/app/enforcement_facade.rs`), so the semantics of
  "the operator's yes becomes an override" are proven, not hypothetical.

## 3. Current implementation evidence

All statements below are measured against commit `6f75853b`.

| # | Fact | Evidence |
|---|---|---|
| E1 | The daemon refuses every approval with "not wired yet" | `crates/eggsec-daemon/src/host.rs` — the `ClientCommand::ApprovePolicy` arm returns `ServerMessage::Error { code: ErrorCode::Unsupported, .. }` |
| E2 | The wire command already carries everything an answer needs | `crates/eggsec-daemon-protocol/src/protocol.rs` — `ApprovePolicy { request_id, session_id, task_id, approved: bool, reason: Option<String> }` |
| E3 | The RBAC already maps the command to a permission | `crates/eggsec-daemon-protocol/src/client_registry.rs` — `CommandPermission::Approver`; `Owner`/`Controller`/`Approver` on a manual surface, `Owner` only on a strict surface |
| E4 | The prompt shape exists with no producer | `crates/eggsec-runtime/src/event.rs` — `RuntimeEvent::PolicyDecisionRequired { session_id, task_id, prompt }` and `PolicyPrompt { message, confirmation_class, requires_explicit_approval }` are defined; nothing in the workspace constructs the event |
| E5 | The GUI already models the state and the prompt | `crates/eggsec-gui/src/tasks.rs` — `GuiTaskStatus::AwaitingPolicy`; the `PolicyDecisionRequired` arm stores the prompt and sets the status |
| E6 | The GUI cannot see the prompt text | `crates/eggsec-gui/src/tasks.rs` — `TaskRow` has no `policy_prompt` field, so `to_row()` drops the prompt the registry already holds |
| E7 | The runtime has no approval API at all | `crates/eggsec-runtime/src/runtime.rs` and `runtime_sink.rs` — no pending-approval registry, no resolve entry point, and `RuntimeEventSink` cannot emit a policy decision |
| E8 | The executor passes no override, so a manual daemon task cannot be confirmed | `crates/eggsec/src/runtime_bridge/executor.rs` — `approve_run_request_bundle(surface, policy, loaded_scope, request, None)` |
| E9 | The kernel already returns the required classes | `crates/eggsec-policy/src/decision.rs` — `EnforcementError::ConfirmationRequired { decision, required_classes: Vec<ConfirmationClass> }`; 8 `ConfirmationClass` variants exist |
| E10 | The only current source of an override is the CLI's pre-run flags | `crates/eggsec-cli/src/main.rs` — `ManualOverride` is built from `--allow-*` before the command runs |
| E11 | The TUI already maps classes to override fields, locally | `crates/eggsec-tui/src/app/enforcement_facade.rs` — `confirm_override` matches on `ConfirmationClass` to set `ManualOverride` fields |
| E12 | The client contract has no approval method | `crates/eggsec-frontend-client/src/lib.rs` — the `RuntimeClient` trait exposes no approval operation; `daemon.rs` references `ClientCommand::ApprovePolicy` only in its request-id matcher |
| E13 | The daemon's HTTP approval route is unreachable | `crates/eggsec-daemon/src/http.rs` — the route is `/sessions/{session_id}/policy/approve` (one path parameter) but the handler extracts `Path((session_id, task_id))` (two) |
| E14 | The runtime's task timeout already bounds a hold | `crates/eggsec-runtime/src/runtime_config.rs` — the default task timeout is 300 s; `max_active_tasks_per_session` is 1 |

## 4. Invariants that must not regress

These are the workspace's standing invariants. This milestone adds a *hold*; it
adds no *bypass*.

- **I1** — `EnforcementContext::evaluate()` remains the mandatory pre-dispatch
  gate for all surfaces. The approval path **re-runs** the existing
  authorisation path; it never constructs an `ApprovedOperation` or
  `ApprovedExecution` itself.
- **I2** — `OperationMetadata` remains the single source of operation policy. No
  new inline policy check is introduced.
- **I3** — Strict surfaces (REST, MCP, agent, CI) fail closed. A strict surface
  never holds an operation, so there is never an approval for a strict client
  to give.
- **I4** — Scope enforcement is unchanged. An approval does not relax the scope
  check; the retry re-runs the whole authorisation path.
- **I5** — The engine's runtime bridge must not import daemon, CLI, TUI, or
  daemon-protocol types (guard 155's direction).
- **I6** — The desktop GUI links no engine, daemon, CLI, or TUI crate
  (guard 155). It reaches the daemon only through `eggsec-frontend-client`.
- **I7** — The runtime stays policy-neutral. `eggsec-runtime` must not depend
  on `eggsec-policy` and must not name the policy crate's vocabulary.
- **I8** — No parallel lookup tables. The confirmation-class→override mapping
  exists in exactly one place.
- **I9** — `let _ =` and `filter_map(|e| e.ok())` remain forbidden; failures are
  logged through `tracing`.
- **I10** — Every spawned Tokio task keeps a 30–300 s timeout wrapper.
- **I11** — Ring-only TLS; the workspace Tokio baseline is unchanged
  (`default-features = false` with per-crate features).
- **I12** — The daemon's published capability list and its admitted set stay
  the same object (guards 169 and 170). This milestone changes neither.

## 5. Scope

### In scope

- The runtime's approval registry, gate, event emission, and resolve API.
- The engine's runtime bridge holding at the confirmation point and resuming on
  approval.
- The policy kernel owning the confirmation-class→override mapping, and the TUI
  refactored onto it.
- The daemon's `ApprovePolicy` handler, its audit record, and the HTTP route's
  arity defect.
- The `RuntimeClient` contract's approval method, and its two in-tree
  implementations.
- The desktop GUI's prompt text, its approval command, and its webview
  affordance.
- Four new architecture guards, negatively tested.
- The documentation and skill updates that state the old behaviour.

### Explicitly out of scope

- **Persisting pending approvals.** The registry is in-memory. A daemon restart
  drops pending approvals and the held task dies with the process. Persistence
  is a separate milestone.
- **The TUI's local approval flow.** The TUI keeps its own prompt. Only its
  class→override *mapping* is refactored onto the shared policy helper, so the
  table exists once.
- **The REST surface's own approval route design.** The route stays as it is;
  only its arity defect is corrected so it is callable.
- **A denial reason vocabulary, a multi-approver quorum, or an approval
  expiry policy.** A single approver answers; the task timeout bounds the wait.
- **Any change to the risk-tier defaults or the enforcement policy itself.**
  ADR-0011 §"Rejected" recorded the decision to leave them alone.
- **Any change to the published capability list or the operation catalog.**

## 6. Required production changes

### Core and domain

1. **`crates/eggsec-runtime/src/approval.rs` (new module).** The approval
   registry and the gate.
   - `pub enum ApprovalAnswer { Approved { reason: Option<String> }, Denied { reason: Option<String> } }`
   - `pub struct ApprovalGate` — a cheap clonable handle over an
     `Arc<ApprovalRegistry>`.
   - `ApprovalGate::request(&self, sink: &RuntimeEventSink, task_id: TaskId,
     prompt: PolicyPrompt, cancel: CancellationToken) -> Result<ApprovalAnswer, RuntimeError>`
     — registers the pending decision, **then** emits the event, **then**
     awaits the answer, racing the cancel token.
   - `ApprovalGate::resolve(&self, session_id: SessionId, task_id: TaskId,
     answer: ApprovalAnswer) -> Result<(), RuntimeError>`
   - `ApprovalGate::pending(&self, session_id: SessionId, task_id: TaskId) -> Option<PolicyPrompt>`
   - The pending entry is removed when it is answered, when the awaiting future
     is dropped (cancellation), or when the task times out. A dropped request
     future must not leave a stale entry behind.
2. **`crates/eggsec-runtime/src/session.rs`.** `RuntimeExecutionContext` gains
   `pub approval: ApprovalGate`. The struct stays `Debug + Clone`.
3. **`crates/eggsec-runtime/src/runtime.rs`.** `Runtime` gains an approval
   registry field, and:
   - `pub fn resolve_approval(&self, session_id: SessionId, task_id: TaskId,
     approved: bool, reason: Option<String>) -> Result<(), RuntimeError>`
   - `pub fn pending_approval(&self, session_id: SessionId, task_id: TaskId) -> Option<PolicyPrompt>`
   - `Runtime::submit` places the gate in the execution context it builds.
4. **`crates/eggsec-runtime/src/runtime_sink.rs`.** `RuntimeEventSink` gains
   `pub fn policy_decision_required(&self, prompt: PolicyPrompt)`. The task is
   the sink's own — a caller cannot announce a prompt for some other task.
5. **`crates/eggsec-policy/src/decision.rs`.** `ManualOverride` gains
   `pub fn permitting_classes(classes: &[ConfirmationClass]) -> ManualOverride`
   — the **one** place a confirmation class is mapped to the override field
   that permits it. The mapping is the inverse of
   `ManualOverride::permits(class)`.
6. **`crates/eggsec-tui/src/app/enforcement_facade.rs`.** `confirm_override`
   delegates to `ManualOverride::permitting_classes`, and its local
   class→field match is deleted.
7. **`crates/eggsec/src/runtime_bridge/executor.rs`.** The executor detects
   `EnforcementError::ConfirmationRequired`, holds, and resumes:
   - It holds **only** when the execution surface honours manual overrides. A
     strict surface's refusal is unchanged.
   - The prompt's `message` names the operation and the decision's reasons
     (`EnforcementError::reasons()`); `confirmation_class` carries the
     `as_str()` labels of the required classes joined by `", "`;
     `requires_explicit_approval` is `true`.
   - On `Approved`, it builds `ManualOverride::permitting_classes(required_classes)`
     with the operator's reason and **retries the same**
     `approve_run_request_bundle` call with that override. A second failure
     fails the task closed.
   - On `Denied`, the task fails with the operator's reason.
   - The retry needs its own copy of the `RunRequest` and the `LoadedScope`,
     because the first call consumes them.

### Storage and migrations

None. The approval registry is in-memory and per-process.

### Protocol and DTOs

- **No wire change.** `ClientCommand::ApprovePolicy` already carries
  `session_id`, `task_id`, `approved`, and `reason`.
- **No protocol version change.** The message shapes do not change; only the
  daemon's behaviour in response to a command that previously always failed.
- `RuntimeClient` gains one method (a source-breaking change for external
  implementors, updated in-tree in the same change):
  `fn approve_task(&self, session_id: SessionId, task_id: TaskId, approved: bool, reason: Option<String>) -> RuntimeClientFuture<()>`.

### Runtime and concurrency

- The approval registry is a `Mutex`-guarded map keyed by
  `(SessionId, TaskId)`. At most one pending approval per task: a second
  request for the same key replaces the first only if the first is already
  gone; otherwise it fails.
- The hold's await races the task's cancel token, so cancelling the task drops
  the pending entry.
- The task's 300 s default timeout already bounds an unanswered hold.
- The hold occupies the session's single active-task slot while it waits, which
  is the honest accounting: the task has not finished.

### Frontend and operator surface

- **`crates/eggsec-frontend-client/src/lib.rs`** — the trait method.
- **`crates/eggsec-frontend-client/src/daemon.rs`** — the daemon
  implementation; sends `ClientCommand::ApprovePolicy` and maps
  `ServerMessage::Ok` to `Ok(())` and `ServerMessage::Error` to the error
  message.
- **`crates/eggsec-frontend-client/src/embedded.rs`** — the embedded
  implementation; calls the runtime's `resolve_approval`.
- **`crates/eggsec-gui/src/tasks.rs`** — `TaskRow` gains
  `policy_prompt: Option<PolicyPromptView>`, and `to_row()` copies it.
- **`crates/eggsec-gui/src/commands.rs`** — a new IPC command `approve_task`
  (the GUI's 13th). It reaches the daemon through `RuntimeClient`, never
  through `ClientCommand` (the existing guard test bans `ClientCommand:` in
  this file).
- **`crates/eggsec-gui/src/shell.rs`** — registers the new command.
- **`crates/eggsec-gui/frontend/app.js`** — the task row shows the prompt's
  message and offers **Approve** and **Reject** when a task's status is
  `awaiting_policy`.

### Security and authorization

- The answer grants exactly the required classes. The client cannot widen it.
- The strict-surface refusal is preserved by construction: a strict surface
  never registers a hold, so the answer path finds nothing pending.
- The daemon's existing RBAC still applies: `CommandPermission::Approver`, with
  `Owner`, `Controller`, and `Approver` on a manual surface, and `Owner` only
  on a strict surface.
- The daemon records the answer as an audit event carrying the answering
  client's identity, the session, the task, the outcome, and the reason.
- The reason is operator-supplied text. It is stored as text and never
  evaluated, interpolated into a command, or echoed into a shell.

### Documentation and static guards

Four new guards, 173–176, in `scripts/check-architecture-guards.sh`:

- **173** — The daemon's `ApprovePolicy` is wired: the handler references the
  runtime's `resolve_approval` and no longer contains the "not wired yet"
  refusal.
- **174** — The runtime stays policy-neutral: `crates/eggsec-runtime` does not
  depend on `eggsec-policy` and names neither `ConfirmationClass` nor
  `EnforcementContext` in its sources.
- **175** — The confirmation-class→override mapping exists once:
  `eggsec-policy` defines `permitting_classes`, and neither the TUI's
  enforcement facade nor the engine's runtime bridge maps a
  `ConfirmationClass` to an override field itself.
- **176** — The daemon's approval answer is audited: the `ApprovePolicy`
  handler records an audit event carrying the answering client's identity.

## 7. Ordered work packages

Each package is independently verifiable. A package's tests must pass before
the next begins.

- **WP-A — One confirmation-class→override mapping, in the policy crate.**
  Add `ManualOverride::permitting_classes` to
  `crates/eggsec-policy/src/decision.rs`; refactor the TUI's
  `confirm_override` onto it; delete the TUI's local match. Add a policy
  test asserting the mapping is the exact inverse of `permits` for all 8
  classes.

- **WP-B — The runtime's approval registry.**
  Add `crates/eggsec-runtime/src/approval.rs`; add the sink method; add the
  `RuntimeExecutionContext` field; add `Runtime::resolve_approval` and
  `Runtime::pending_approval`; wire the gate into `Runtime::submit`. Tests:
  register→resolve→observed; resolve twice→refused; drop the awaiting future→
  the entry is gone; `pending_approval` reports the prompt.

- **WP-C — The engine holds and resumes.**
  In `crates/eggsec/src/runtime_bridge/executor.rs`, detect
  `ConfirmationRequired`, hold, and resume. Tests: a manual surface holds and
  then proceeds after approval; a denial fails the task with the operator's
  reason; a strict surface never holds; the retry's override permits exactly
  the required classes.

- **WP-D — The daemon answers.**
  Replace the `ApprovePolicy` refusal in `crates/eggsec-daemon/src/host.rs`
  with the resolve path plus its audit record. Correct the HTTP route's arity
  defect in `crates/eggsec-daemon/src/http.rs`. Tests: the daemon answers a
  held task and audits it; a task with nothing pending is refused as invalid
  state; the RBAC still refuses a client without the approver role.

- **WP-E — The client contract exposes the answer.**
  Add the `RuntimeClient` method and implement it in `daemon.rs` and
  `embedded.rs`. Tests: the daemon client round-trips an answer; the embedded
  client resolves against its runtime.

- **WP-F — The GUI offers the affordance.**
  Add `TaskRow::policy_prompt`; add the `approve_task` IPC command; register
  it; add the webview affordance. Update the GUI's IPC-surface count test
  (12 → 13) and its `ClientCommand`-ban test.

- **WP-G — Guards, docs, and skills.**
  Add guards 173–176 and negative-test each. Update `docs/DAEMON.md`,
  `architecture/daemon.md`, `architecture/runtime.md`, `architecture/overview.md`,
  `docs/ENFORCEMENT_MODES.md`, `docs/CI_ARCHITECTURE_GUARDS.md`, and the
  `eggsec-daemon`, `eggsec-gui`, `eggsec-frontend-client`, and `eggsec-runtime`
  skills.

## 8. Failure, cancellation, restart, and contention semantics

| Situation | Required behaviour |
|---|---|
| The operator approves | The held operation resumes, re-runs authorisation with the bounded override, and dispatches. If the retry fails, the task fails closed. |
| The operator rejects | The task fails with the operator's reason. The pending entry is removed. |
| The operator never answers | The task's 300 s timeout fires and the task fails. The pending entry is removed. |
| The task is cancelled while held | The cancel token wins the race; the pending entry is removed; the task is reported cancelled. |
| The daemon restarts while a task is held | The registry is lost with the process and the held task dies with it. This is recorded, not hidden. |
| Two clients answer the same prompt | The first wins. The second finds nothing pending and is refused as invalid state. |
| A strict client answers | There is nothing pending, because a strict surface never holds. The answer is refused as invalid state. |
| A client without the approver role answers | The RBAC refuses it before the handler runs. |
| The session's single active-task slot is taken by a held task | A second task in the same session is refused by the existing admission rule. The hold does not bypass it. |

## 9. Compatibility and migration

- **Wire protocol**: unchanged. No new command, no new event, no changed shape.
- **Protocol version**: unchanged.
- **`RuntimeClient` trait**: gains one method — source-breaking for external
  implementors. Both in-tree implementations are updated in the same change.
- **`RuntimeExecutionContext`**: gains one field — source-breaking for external
  constructors. It is a runtime-crate struct; its in-tree constructors are
  updated in the same change.
- **Storage**: no migration.
- **Behavioural compatibility**: a daemon that gains the wired handler is
  compatible with an older client. A client that asks to approve a task with
  nothing pending receives `ErrorCode::InvalidState` instead of the previous
  `ErrorCode::Unsupported` — a narrower, more truthful refusal.

## 10. Required tests

**`crates/eggsec-policy`** (WP-A)

- The class→override mapping is the exact inverse of `permits` for all 8
  classes.
- `permitting_classes(&[])` grants nothing.

**`crates/eggsec-runtime`** (WP-B)

- A registered prompt is observable through `pending_approval`.
- An answer resolves the hold and the awaiting task observes it.
- A second answer for the same task is refused.
- Dropping the awaiting future removes the pending entry.
- Cancelling while held returns a cancellation error and removes the entry.

**`crates/eggsec`** (WP-C)

- A manual surface holds at the confirmation point and resumes after approval.
- A denial fails the task with the operator's reason.
- A strict surface never registers a hold.
- The retry's override permits exactly the required classes and no more.

**`crates/eggsec-daemon`** (WP-D)

- `ApprovePolicy` on a held task succeeds and writes an audit event naming the
  answering client.
- `ApprovePolicy` on a task with nothing pending is refused as invalid state.
- A client without the approver role is refused by the RBAC.

**`crates/eggsec-frontend-client`** (WP-E)

- The daemon client round-trips an answer.
- The embedded client resolves the answer against its runtime.

**`crates/eggsec-gui`** (WP-F)

- A task row carries the prompt's message.
- The IPC surface exposes 13 commands and still names no `ClientCommand`.

**Integration** (WP-D, WP-F)

- An end-to-end proof: a manual daemon client receives
  `PolicyDecisionRequired`, answers through the client contract, and the held
  operation runs.

## 11. Required verification commands

```bash
cargo fmt --all -- --check
cargo check --workspace --no-default-features
cargo test -p eggsec-policy
cargo test -p eggsec-runtime
cargo test -p eggsec --features cli --lib
cargo test -p eggsec-daemon --features service
cargo test -p eggsec-daemon-protocol
cargo test -p eggsec-frontend-client
cargo test -p eggsec-gui --no-default-features --lib
cargo test -p eggsec-gui --features shell --lib
cargo test -p eggsec-cli --features daemon-client
cargo clippy --lib -p eggsec -- -D warnings
cargo clippy --lib -p eggsec --features cli -- -D warnings
cargo clippy -p eggsec-core -p eggsec-tool-core -p eggsec-report-model -p eggsec-output -p eggsec-runtime -p eggsec-ui-model -p eggsec-agent -p eggsec-transport -p eggsec-transport-eggfetch -p eggsec-policy -- -D warnings
cargo clippy -p eggsec-gui --all-targets -- -D warnings
cargo clippy -p eggsec-gui --no-default-features --all-targets -- -D warnings
cargo clippy -p eggsec-daemon-protocol --all-targets -- -D warnings
cargo clippy -p eggsec-frontend-client --all-targets -- -D warnings
cargo clippy -p eggsec-runtime --all-targets -- -D warnings
bash scripts/check-architecture-guards.sh
python3 scripts/check_doc_references.py
make check-msrv
```

`make check` is the mandatory contract and is expected to stop at the inherited
`check-deps` advisory backlog; that is owned by milestone 015 and is not this
milestone's to fix. `make check-python` is required only if the Python
bindings, stubs, or docs change — this milestone does not change them.

## 12. Documentation updates

- `docs/DAEMON.md` — the `ApprovePolicy` command is no longer refused; describe
  the hold, the answer, and the audit record.
- `docs/ENFORCEMENT_MODES.md` — the line recording that the approval path is
  unwired must be corrected.
- `architecture/daemon.md` — the daemon's approval invariants and the audit
  record.
- `architecture/runtime.md` — the runtime's approval registry, the gate, the
  event, and the resolve entry point.
- `architecture/overview.md` — the headline counts, if any of them change.
- `docs/CI_ARCHITECTURE_GUARDS.md` — guards 173–176.
- `.opencode/skills/eggsec-daemon/SKILL.md`,
  `.opencode/skills/eggsec-gui/SKILL.md`,
  `.opencode/skills/eggsec-frontend-client/SKILL.md`, and
  `.opencode/skills/eggsec-runtime/SKILL.md` — where they describe the
  approval path as unwired.

Skills are not type-checked. Any claim in a skill that this milestone falsifies
is corrected in the same change.

## 13. Acceptance criteria

1. A manual daemon client receives `RuntimeEvent::PolicyDecisionRequired` for
   an operation whose policy demands confirmation, with a prompt naming the
   operation and the decision's reasons.
2. The client answers through the `RuntimeClient` contract, and the held
   operation then runs and completes.
3. The answer grants exactly the confirmation classes the kernel demanded: a
   test asserts the retry's override permits the required classes and no
   others.
4. A rejection fails the task with the operator's reason.
5. A strict client of the same daemon is still refused: a strict surface never
   registers a hold, and an answer for it finds nothing pending.
6. The daemon's answer is audited with the answering client's identity, the
   session, the task, the outcome, and the reason.
7. The GUI shows the prompt's message and offers an Approve/Reject affordance
   for a task awaiting a decision.
8. The four new guards fail on an injected violation and pass when restored.
9. Every command in §11 passes, except where §11 records a pre-existing
   failure that this milestone does not own.

## 14. Stop conditions

The implementer **stops and reports** rather than improvising if any of these
holds:

1. **ADR-0012 is amended or rejected after this plan is written.** The plan
   implements ADR-0012's Decision section; a changed decision needs a changed
   plan.
2. **The engine's runtime bridge would need a daemon, CLI, TUI, or
   daemon-protocol import** to implement the hold. That would break I5 and
   means the design is wrong, not that the import is needed.
3. **The runtime would need to depend on `eggsec-policy`** to implement the
   hold. That would break I7.
4. **The approval would have to relax the scope check**, or bypass
   `EnforcementContext`, or construct an `ApprovedOperation` directly. Any of
   those breaks I1 or I4 and must be escalated.
5. **A strict surface would acquire a hold.** That breaks I3.
6. **A second confirmation-class→override mapping would be introduced.** That
   breaks I8; the shared helper must be reused instead.
7. **The approval would need to be persisted** to satisfy a requirement in this
   plan. Persistence is explicitly out of scope (§5).
8. **An existing counted set changes** (crates, features, operations, aliases,
   tabs, task kinds, payloads, techniques, endpoints, probes, descriptors) in a
   way this plan did not anticipate. The affected skill and
   `architecture/overview.md` must be updated in the same change.
9. **The wire protocol would need a new command or event.** This plan requires
   none; a need for one means the design diverged from ADR-0012.

## 15. Closure evidence required

The closure record (`plans/closure/desktop-gui-frontend/011-daemon-mediated-policy-approval-closure.md`)
must state, with evidence:

1. The measured confirmation-class count and the measured list of classes.
2. The measured pending-approval lifecycle: registered, answered, removed —
   and the removal on cancel and on timeout.
3. The measured end-to-end proof: the client received the prompt, answered,
   and the held operation ran.
4. The measured proof that a strict client is still refused.
5. The measured proof that the answer is bounded by the required classes.
6. The measured audit record for one approval, naming the answering client.
7. The four guards' negative-test results, each with the injected violation
   and the message it produced.
8. The exact commands from §11 that were run, and their results.
9. Every command that failed, with the reason and whether it is pre-existing.
10. The residual risks this milestone accepts, stated as residuals rather
    than as claims of correctness.
11. The counts that changed, and where they are now stated.
12. The plan's deviations, if any, with the reason for each.

## 16. Handoff notes

- **Start with WP-A.** The shared mapping is the smallest change and it removes
  a duplicate table, which makes WP-C's override construction obviously
  correct.
- **The registration order matters.** The executor must register the pending
  decision *before* emitting `PolicyDecisionRequired`, or a client that answers
  immediately will find nothing pending.
- **The retry must reuse the existing authorisation path.** The temptation is
  to build the `ApprovedOperation` directly once the operator says yes. That
  breaks I1. The retry re-runs `approve_run_request_bundle` with the bounded
  override and lets the kernel decide.
- **`RunRequest` and `LoadedScope` are consumed by the first call.** Clone
  them before the first attempt; the retry needs its own copies.
- **The `RuntimeClient` trait change is source-breaking.** Update both in-tree
  implementations in the same change, or the workspace will not compile.
- **The GUI must not name `ClientCommand`.** Its existing guard test bans the
  string in `commands.rs`; the approval goes through `RuntimeClient`.
- **The GUI's IPC command count changes from 12 to 13.** Its test asserts the
  count; update it in WP-F, not later.
- **The HTTP route's arity defect is a real defect**, not a cosmetic one: the
  route declares one path parameter and the handler extracts two, so the route
  cannot currently be called. Fix it in WP-D.
- **The TUI's `confirm_override` is proven behaviour.** Its refactor onto
  `permitting_classes` must preserve the exact flags it set, including the
  `OutOfScope`/`TargetExpansion` pair, which share `allow_out_of_scope`.
- **Do not add a second `ManualOverride` table** in the engine's runtime
  bridge. Call `permitting_classes`.
- **Negative-test the guards with file backups, never `git checkout --`** on a
  tree with uncommitted work.
