# ADR-0012: Daemon-mediated Policy Approval

Status: proposed

Date: 2026-10-24

Decision owners: project maintainers

Related specification sections:

- `plans/000-long-term-specification.md#4-enforcement-invariants`
- `plans/000-long-term-specification.md#6-dispatch-runtime-and-wire-identity`
- `plans/000-long-term-specification.md#10-frontend-rules`

Affected subsystem roadmaps:

- `plans/subsystems/desktop-gui-frontend-roadmap.md`

## Context

ADR-0011 made the daemon publish and admit every canonical capability name for
manual execution. Its Decision section also stated the consequence that
milestone 010 could not itself deliver:

> The daemon must be able to hold an operation at its confirmation point,
> expose that decision to the client, accept the client's answer, and dispatch
> the held operation.

Milestone 010 implemented the advertisement and the admission, and deliberately
deferred the confirmation path to milestone 011, because wiring the confirmation
path changes the daemon's **execution** semantics rather than its advertisement.

The remaining gap is concrete:

- `EnforcementContext::evaluate()` returns `EnforcementOutcome::RequireConfirmation`
  when an operation needs an operator's confirmation — an out-of-scope target, a
  high-risk tier, an explicitly excluded target, and so on. For a permissive
  manual surface, `approve_manual()` turns that into
  `EnforcementError::ConfirmationRequired { decision, required_classes }`
  unless the caller supplies a `ManualOverride` that permits every required
  class.
- The only way to supply that override today is the CLI's `--allow-*` flags
  (`crates/eggsec-cli/src/main.rs`), which are decided **before** the operation
  runs, by whoever launched the CLI.
- The daemon's `ClientCommand::ApprovePolicy` handler answers
  `ErrorCode::Unsupported` — "not wired yet"
  (`crates/eggsec-daemon/src/host.rs`).
- `RuntimeEvent::PolicyDecisionRequired` and `PolicyPrompt` already exist in the
  runtime's vocabulary (`crates/eggsec-runtime/src/event.rs`), and the desktop
  GUI already models the state (`GuiTaskStatus::AwaitingPolicy`,
  `crates/eggsec-gui/src/tasks.rs`). **Nothing produces the event.** The prompt
  is a shape with no author.

So the desktop GUI cannot complete an operation whose policy demands
confirmation. An operator can start the task; they cannot answer it. Milestone
010's advertisement is honest, but the capability it advertises is not
completable through the surface it advertises it on.

The decision required here is **where the hold lives** and **what an answer
means**.

## Decision drivers

- The answer must be **bounded by the policy**. A client answering "yes" must
  not be able to grant more than the specific confirmation classes the kernel
  demanded for that operation, and must never be able to grant an authorisation
  the kernel would not give a manual operator holding the full CLI flag set.
- The answer must be **recorded** — who answered, what they answered, for which
  task, and why.
- A surface with **no operator** must not be able to acquire one. A strict
  surface (CI, MCP, agent, REST) has no human at a prompt, so it must never be
  approvable.
- The hold must be **cancellable and bounded**. An unanswered approval must not
  pin a session slot forever.
- Ownership must not blur: the runtime owns task lifecycle, the daemon owns the
  process, and `eggsec-policy` owns authorisation semantics. The engine's
  runtime bridge must not import daemon types.
- The runtime must stay **policy-neutral**. It is a DTO crate; it must not learn
  the policy crate's vocabulary.
- No **parallel lookup tables**: the mapping from a confirmation class to the
  override that permits it must exist in exactly one place.

## Considered options

### Option A — The daemon host owns the hold

The daemon holds the pending approval in its own registry; the engine's
executor blocks on a channel the daemon resolves.

Benefits: the hold lives next to the audit trail and the RBAC matrix.

Costs: the daemon is not the task-lifecycle owner. The executor would need a
daemon-supplied handle, and the engine's runtime bridge cannot take one — the
engine must not depend on `eggsec-daemon` (guard 155's direction). It also
splits task state across two owners: the runtime's task table and the daemon's
approval table.

### Option B — The runtime owns the hold

The runtime holds the pending approval, keyed by session and task, and exposes
both the request and the answer. The engine's executor registers the hold
through a gate carried in `RuntimeExecutionContext`, awaits the answer, and —
on approval — retries the *same* authorisation path with a `ManualOverride`
bounded by the required classes. The daemon's `ApprovePolicy` handler resolves
the hold and records the answer.

Benefits: one owner of task state; the executor needs no daemon types; the
answer is bounded by the kernel's own vocabulary; cancellation and the task
timeout already belong to the runtime.

Costs: the runtime gains a small amount of machinery that only the manual path
uses.

### Option C — No hold; the client re-submits with an override

The client resubmits the task carrying the override flags it wants.

Benefits: no new machinery at all.

Costs: it inverts authority. The client would name the authorisation instead
of the policy, which is the approve-one-dispatch-another shape the
`ApprovedExecution` bundle exists to prevent. It also loses the binding between
the prompt and the answer, and the operator never learns what the policy
actually demanded. Rejected.

### Option D — The policy crate owns the hold

Rejected for the same authority inversion, and because `eggsec-policy` is a pure
kernel with no Tokio and no task lifecycle.

## Decision

**Option B.** The runtime holds the operation; the answer is a bounded override.

1. **The runtime owns the hold.** `eggsec-runtime` gains an approval registry.
   A cheap clonable handle (`ApprovalGate`) is carried in
   `RuntimeExecutionContext`, so an executor can register a pending decision and
   await its answer. `Runtime` exposes the answering side (`resolve_approval`)
   and the observing side (`pending_approval`). The registry is in-memory and
   per-process; a pending entry is removed when it is answered, when the holding
   task is cancelled, or when the task's timeout fires.

2. **The hold is registered before the client is told.** The executor registers
   the pending decision and *then* emits
   `RuntimeEvent::PolicyDecisionRequired`. A client that answers immediately
   therefore never races the registration.

3. **The answer is bounded by the required classes.** The kernel's
   `EnforcementError::ConfirmationRequired` carries the `ConfirmationClass`
   values it demanded. Those classes are mapped, in **one** place
   (`eggsec-policy`), to the `ManualOverride` fields that permit them. The
   executor then retries the *same* `approve_run_request_bundle` call with that
   override. It does not construct an `ApprovedExecution` itself and it does
   not bypass the kernel. If the retry still fails, the task fails closed.

4. **A strict surface never holds.** The hold exists only where the execution
   surface honours manual overrides. A strict surface's `approve_manual` path
   refuses to be confirmed by anyone, so the executor registers no hold at all
   and the operation is refused exactly as it is today.

5. **The daemon's `ApprovePolicy` becomes the answer path.** It resolves the
   runtime's pending approval. The existing RBAC is unchanged and still
   applies: `CommandPermission::Approver`, with `Owner`, `Controller`, and
   `Approver` permitted on a manual surface and `Owner` only on a strict
   surface. The daemon records the answer in the audit trail with the
   answering client's identity, the session, the session's surface, and the
   outcome. The audit schema is unchanged: a session holds at most one active
   task, so the session identifies the decision, and the operator's free-text
   reason is not persisted.

6. **The runtime stays policy-neutral.** The registry stores a `PolicyPrompt` —
   a message, a confirmation-class label, and a flag — not a `ConfirmationClass`.
   The gate returns an answer (`approved` or `denied`, with a reason); it does
   not interpret authorisation. Only `eggsec-policy` decides what an approval
   permits.

7. **The client contract gains one method.** `RuntimeClient` gains an approval
   method, so the GUI reaches the daemon through the same frontend-neutral
   contract it uses for everything else.

## Consequences

### Positive

- The desktop GUI can complete an operation whose policy demands confirmation:
  the operator sees the prompt, answers, and the held operation proceeds.
- The answer is bounded by the kernel's own demand; a client cannot widen its
  own authorisation.
- Strict surfaces are unaffected: they never hold, so there is nothing to
  approve.
- One owner of task state, and one owner of authorisation semantics.

### Negative

- The runtime gains machinery that only the manual path uses.
- The answer is a "yes" to a *specific* set of classes. The operator cannot
  widen it at answer time — widening is the CLI's `--allow-*` flags, decided
  before the run. This is deliberate.
- A pending approval holds the session's single active-task slot until it is
  answered, cancelled, or the task times out (300 s default).

### Neutral or deferred

- The approval registry is **not** persisted. A daemon restart drops pending
  approvals, and the held task dies with the process.
- The TUI's local approval path is unchanged and does not use the runtime's
  gate. Its confirmation-class→override mapping is refactored onto the shared
  policy helper so the table exists once, but the TUI's prompt flow stays local.
- The REST surface keeps its existing approval route; its path-arity defect is
  corrected so the route is callable.

## Compatibility and migration

- **Wire protocol**: unchanged. `ClientCommand::ApprovePolicy`, `ErrorCode`,
  and `RuntimeEvent::PolicyDecisionRequired` all already exist. A daemon that
  gains the wired handler remains compatible with an older client, which would
  only ever have received an event it never had.
- **`RuntimeClient` trait**: gains one method. This is source-breaking for
  external implementors of the trait; the two in-tree implementations
  (`DaemonRuntimeClient`, `EmbeddedRuntimeClient`) are updated in the same
  change.
- **Protocol version**: unchanged. No message shape changes; only the daemon's
  behaviour in response to a command that previously always failed.
- **No storage migration**: the approval registry is in-memory.

## Security and reliability implications

- **Authorisation**: the answer grants exactly the confirmation classes the
  kernel demanded, computed by the kernel. The client names *yes* or *no*; it
  never names the authorisation.
- **Scope**: unchanged. An approval does not relax the scope check — the retry
  re-runs the whole authorisation path, scope included.
- **Audit**: the answer is recorded with the answering client's identity, the
  session, the session's surface, and the outcome. The operator's reason is
  carried to the executor, which logs the approval; it is stored as text and
  never evaluated, and it is not persisted in the audit trail.
- **Cancellation**: a cancelled task drops its pending approval. The registry
  entry cannot outlive the task.
- **Denial of service**: an unanswered approval holds one session's single
  active-task slot for at most the task timeout.
- **Replay**: an answer resolves one pending entry, which is removed on use. A
  second answer for the same task finds nothing pending and is refused as
  invalid state.
- **Surface confusion**: a strict session has no pending approval to answer;
  the answer path refuses rather than guessing.

## Verification

- The runtime's registry: register → answer → the holding task resumes; cancel
  → the entry is gone; answer twice → the second is refused.
- The engine: an operation that demands confirmation holds; after approval it
  proceeds; after denial it fails with the operator's reason; a strict surface
  never holds.
- The daemon: `ApprovePolicy` on a held task succeeds and is audited; on a
  task with nothing pending it is refused as invalid state; a client without
  the approver role is refused by the RBAC.
- The client contract: the GUI answers through `RuntimeClient`, and the answer
  reaches the daemon.
- The mapping: the confirmation-class→override mapping exists in exactly one
  place.

## Supersession

None.
