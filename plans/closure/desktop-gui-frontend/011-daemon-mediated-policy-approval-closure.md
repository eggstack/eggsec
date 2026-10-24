# Desktop GUI Frontend Milestone 011 — Closure Status

Status: closed

Source implementation plan:

- `plans/implementation/desktop-gui-frontend/011-daemon-mediated-policy-approval.md`

Source subsystem roadmap:

- `plans/subsystems/desktop-gui-frontend-roadmap.md#7-milestones`

Repository baseline reviewed: `6f75853b`

Durable decision implemented:

- `plans/adrs/ADR-0012-daemon-mediated-policy-approval.md` (**proposed
  2026-10-24** — see §10, finding F1: the ADR is deliberately left
  `proposed` rather than accepted, because this milestone implemented it and
  the decision is the maintainer's to ratify)

Implementation commits:

- `9d812c68` — feat(daemon): implement the desktop GUI milestone 011
- `this commit` — docs(plans): record the closure of the desktop GUI
  milestone 011

## 1. Executive finding

**The daemon can now hold an operation at its confirmation point, expose the
decision, and take the operator's answer.** A manual client that submits an
operation whose policy demands a confirmation now sees the prompt, answers it,
and the held operation runs.

The design decision that makes this safe — and that this milestone had to
make, because `ADR-0011` mandated the capability but not the mechanism — is
**who owns the hold and what an answer means**:

- **The runtime owns the hold.** It is the task-lifecycle owner; the daemon
  owns the process and `eggsec-policy` owns authorisation semantics. The
  runtime's new `ApprovalRegistry` holds a pending decision keyed by session
  and task, and the decision is registered **before** the prompt is announced,
  so a client that answers the instant it sees the prompt never races the
  registration.
- **The answer is bounded by the policy, not by the client.** The kernel's
  `EnforcementError::ConfirmationRequired` already carried the
  `ConfirmationClass` values it demanded; the bridge now preserves them across
  the crossing into the runtime, and the executor re-runs the **same**
  authorisation path with an override built from exactly those classes. The
  client names *yes* or *no*. It never names the authorisation.
- **A strict surface never holds.** It has no operator to ask, so the
  operation is refused exactly as it always was, and an answer for a strict
  session finds nothing pending. The strict client's refusal is preserved by
  construction, not by a check.

The GUI side is small by design: the task row now carries the policy's own
prompt, and the webview offers **Approve** and **Reject**. The GUI still never
auto-approves — an auto-approval would be the GUI settling an authorisation
question that only the daemon's enforcement layer may settle.

**What this milestone did NOT do**, and why: it did not persist pending
approvals, it did not add a quorum or an expiry policy, it did not touch the
TUI's own local approval flow (beyond removing a duplicated mapping table),
and it did not change the risk-tier defaults or the enforcement policy
itself. Those are §10's residuals, not claims of completeness.

## 2. Requirement-to-evidence matrix

| Plan requirement (§7 work package) | Evidence | Status |
|---|---|---|
| WP-A — one confirmation-class→override mapping in `eggsec-policy` | `crates/eggsec-policy/src/decision.rs` — `ManualOverride::permitting_classes`; `crates/eggsec-tui/src/app/enforcement_facade.rs` — `confirm_override` delegates to it and its local match is deleted | done |
| WP-B — the runtime's approval registry, gate, sink event, resolve API | `crates/eggsec-runtime/src/approval.rs` (new); `runtime_sink.rs` — `policy_decision_required`; `session.rs` — `RuntimeExecutionContext::approval`; `runtime.rs` — `resolve_approval`, `pending_approval` | done |
| WP-C — the engine holds at the confirmation point and resumes | `crates/eggsec/src/runtime_bridge/executor.rs` — the hold, the prompt, the bounded retry; `manual.rs` / `surface.rs` — the bridge preserves the required classes | done |
| WP-D — the daemon answers `ApprovePolicy`, audits it, and the HTTP route is callable | `crates/eggsec-daemon/src/host.rs` — the handler resolves the runtime's pending decision and audits it; `crates/eggsec-daemon/src/http.rs` — the route's path arity corrected | done |
| WP-E — the client contract exposes the answer | `crates/eggsec-frontend-client/src/lib.rs` — `RuntimeClient::approve_task`; `daemon.rs` and `embedded.rs` — the two in-tree implementations | done |
| WP-F — the GUI offers the affordance | `crates/eggsec-gui/src/tasks.rs` — `TaskRow::policy_prompt`; `commands.rs` — `approve_task`; `shell.rs` — registration; `frontend/app.js` — the Approve/Reject buttons | done |
| WP-G — guards, docs, skills | `scripts/check-architecture-guards.sh` — guards 173–176; §9 below | done |

| Plan acceptance criterion (§13) | Evidence | Status |
|---|---|---|
| 1 — a manual client receives the prompt, naming the operation and the reasons | `crates/eggsec/src/runtime_bridge/executor.rs` — `a_manual_surface_holds_the_operation_until_the_operator_answers` asserts the prompt names `fuzz` and demands an explicit approval | met |
| 2 — the client answers through the contract and the held operation runs | `crates/eggsec-daemon/tests/policy_approval.rs` — `the_client_answers_a_pending_decision_over_the_socket` (the whole chain over a live Unix socket) and `a_held_operation_resumes_when_the_client_answers` | met |
| 3 — the answer grants exactly the demanded classes | `crates/eggsec/src/runtime_bridge/executor.rs` — `an_answer_is_bounded_by_the_classes_the_kernel_demanded`; `crates/eggsec-policy/src/decision.rs` — the four mapping tests | met |
| 4 — a rejection fails the task with the operator's reason | `crates/eggsec-daemon/tests/policy_approval.rs` — `a_refused_operation_stays_refused` | met |
| 5 — a strict client is still refused | `crates/eggsec/src/runtime_bridge/executor.rs` — `a_strict_surface_never_holds_the_operation`; `crates/eggsec-daemon/tests/policy_approval.rs` — `a_client_without_the_approver_role_cannot_answer` | met |
| 6 — the answer is audited with the answering client's identity | `crates/eggsec-daemon/tests/policy_approval.rs` — `the_answer_is_audited_with_the_answering_client` | met |
| 7 — the GUI shows the prompt and offers the affordance | `crates/eggsec-gui/src/tasks.rs` — `a_task_row_carries_the_policy_prompt`; `frontend/app.js` — the Approve/Reject buttons | met |
| 8 — the four guards fail on an injected violation | §4's negative-test table | met |
| 9 — every §11 command passes | §4 | met, with the §10 caveats |

## 3. Production implementation evidence

### 3.1 The runtime's pending-approval registry

`crates/eggsec-runtime/src/approval.rs` is new. It holds three things:

| Piece | Purpose |
|---|---|
| `ApprovalAnswer` | `Approved { reason }` / `Denied { reason }` — the operator's yes or no |
| `ApprovalRegistry` | The in-memory map of pending decisions, keyed by `(SessionId, TaskId)` |
| `ApprovalGate` | The cheap clonable handle carried in `RuntimeExecutionContext` |

Four properties were designed in rather than left to convention:

1. **Registration precedes announcement.** `ApprovalGate::request` registers the
   decision, and only then calls `sink.policy_decision_required(...)`. A client
   that answers the instant it sees the prompt therefore never finds nothing
   pending.
2. **The wait is cancellable.** The await races the caller's cancellation
   token, so a cancelled task drops its pending decision.
3. **The entry cannot outlive its task.** A `RegistrationGuard` removes the
   registration when the awaiting future is dropped — including the abort path.
   The guard carries the registration's identifier, so a late drop cannot
   remove a *later* decision registered for the same task.
4. **The runtime stays policy-neutral.** The registry stores a `PolicyPrompt` —
   a message, a confirmation-class **label**, and a flag — and never a
   `ConfirmationClass`. The gate returns an answer; it does not interpret what
   an answer permits. Guard 174 enforces this.

### 3.2 The engine's hold and bounded resume

`crates/eggsec/src/runtime_bridge/executor.rs` is where the hold happens:

- The first authorisation call still passes `None` for the manual override.
- A refusal that demands a confirmation is **held** — but only where the
  execution surface honours manual overrides. A strict surface's refusal is
  unchanged, and no hold is registered.
- The prompt names the operation and the decision's reasons, and carries the
  required classes' `as_str()` labels joined by `", "`.
- On approval, the executor builds `ManualOverride::permitting_classes(&required_classes)`
  with the operator's reason and **retries the same `approve_run_request_bundle`
  call**. It does not construct an `ApprovedOperation` itself, and it does not
  bypass the kernel. If the retry still fails, the task fails closed.

`crates/eggsec/src/runtime_bridge/surface.rs` gained
`RuntimeBridgeError::ConfirmationRequired { reason, required_classes }`, and
`crates/eggsec/src/runtime_bridge/manual.rs` maps the kernel's error into it.
**This was a necessary change**: the bridge previously flattened every
enforcement refusal into `EnforcementDenied { reason: String }`, which
destroyed the required classes — and without them the answer would have been
unbounded.

### 3.3 The daemon's answer path

`crates/eggsec-daemon/src/host.rs` — the `ApprovePolicy` arm now resolves the
runtime's pending decision, and audits the answer under the action
`approve-policy` with the answering client's identity, the session, the
session's **real** surface label, and the outcome (`allow`, `deny`, or
`rejected`). An answer for a task with nothing pending is refused with
`ErrorCode::InvalidState` — a truthful refusal that distinguishes "there is no
decision pending" from "this daemon cannot answer".

The daemon's existing RBAC is unchanged and still applies:
`CommandPermission::Approver`, with `Owner`, `Controller`, and `Approver`
permitted on a manual surface and `Owner` only on a strict surface.

`crates/eggsec-daemon/src/http.rs` — the REST approval route was **unreachable**:
it was declared `/sessions/{session_id}/policy/approve` (one path parameter)
while the handler extracted `Path((session_id, task_id))` (two). The route now
matches the sibling cancel route's shape:
`/sessions/{session_id}/tasks/{task_id}/policy/approve`.

### 3.4 The client contract

`crates/eggsec-frontend-client/src/lib.rs` — `RuntimeClient` gains
`approve_task(session_id, task_id, approved, reason)`. This is a
source-breaking change for external implementors of the trait; both in-tree
implementations (`DaemonRuntimeClient`, `EmbeddedRuntimeClient`) are updated in
the same change.

### 3.5 The GUI

- `crates/eggsec-gui/src/tasks.rs` — `TaskRow` gains
  `policy_prompt: Option<PolicyPrompt>`, so the webview sees the policy's own
  wording. (The GUI's `TaskState` already held the prompt; the row dropped it.)
- `crates/eggsec-gui/src/commands.rs` — the GUI's **thirteenth** IPC command,
  `approve_task`. It reaches the daemon through `RuntimeClient`, never through
  `ClientCommand` — the GUI's existing guard test bans that string in this
  file.
- `crates/eggsec-gui/frontend/app.js` — a task whose status is
  `awaiting_policy` shows the prompt's message, the confirmation class, and
  **Approve** / **Reject** buttons.

### 3.6 The one mapping table

`ManualOverride::permitting_classes` is the **single** place a confirmation
class maps onto the override field that permits it — the exact inverse of
`ManualOverride::permits`. The TUI's own `confirm_override` already had this
table; it now calls the shared helper, so the table exists once. Guard 175
pins it.

Two properties of the mapping are deliberate and recorded in the code:

- `assume_yes` is never set. An approval is not prompt suppression: it grants
  the classes that were demanded, not "yes" to everything.
- `OutOfScope` and `TargetExpansion` share `allow_out_of_scope`, so permitting
  either permits both. The two cannot be separated without changing the
  override's shape, which the CLI's flag set already fixes. This was the TUI's
  existing behaviour, preserved exactly.

### 3.7 Measured counts

| Count | Before | After |
|---|---|---|
| `ConfirmationClass` variants | 8 | 8 (unchanged) |
| `eggsec-runtime` source files | 13 | **14** (`approval.rs` added) |
| `RuntimeError` variants | 13 | **14** (`NoPendingApproval` added) |
| GUI IPC commands | 12 | **13** (`approve_task` added) |
| Architecture guards | 171 | **175** (guards 173–176 added; highest number 176) |
| `RuntimeClient` trait methods | 8 | **9** (`approve_task` added) |

## 4. Verification executed

All commands were run in the working tree of this milestone. Results are as
observed.

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | clean (after `cargo fmt --all` applied) |
| `cargo check --workspace --no-default-features` | clean, no errors |
| `cargo test -p eggsec-policy` | **93** passed (88 before, +5) |
| `cargo test -p eggsec-runtime` | **108** lib + **22** integration passed (102 + 22 before) |
| `cargo test -p eggsec-daemon-protocol` | **89** lib + **7** passed (unchanged) |
| `cargo test -p eggsec-frontend-client` | **15** passed (13 before, +2) |
| `cargo test -p eggsec-output` | **73** + **7** + **12** passed (unchanged) |
| `cargo test -p eggsec-daemon --features service` | **108** lib + **1** + **10** + **20** + **4** + **6** passed (the last is the new `policy_approval` suite) |
| `cargo test -p eggsec --features cli --lib` | **1405** passed (1401 before, +4) |
| `cargo test -p eggsec-gui --features shell --lib` | **84** passed (83 before, +1) |
| `cargo test -p eggsec-gui --no-default-features --lib` | **80** passed (79 before, +1) |
| `cargo test -p eggsec-cli --features daemon-client` | **31** passed (unchanged) |
| `cargo test -p eggsec-python` | **237** passed (unchanged) |
| `cargo clippy --lib -p eggsec -- -D warnings` | clean |
| `cargo clippy --lib -p eggsec --features cli -- -D warnings` | clean |
| `cargo clippy -p eggsec-core -p eggsec-tool-core -p eggsec-report-model -p eggsec-output -p eggsec-runtime -p eggsec-ui-model -p eggsec-agent -p eggsec-transport -p eggsec-transport-eggfetch -p eggsec-policy -- -D warnings` | clean |
| `cargo clippy -p eggsec-cli --features daemon-client -- -D warnings` | clean |
| `cargo clippy -p eggsec-runtime --features schema -- -D warnings` | clean |
| `cargo clippy -p eggsec-runtime --all-targets -- -D warnings` | clean |
| `cargo clippy -p eggsec-gui --all-targets -- -D warnings` | clean |
| `cargo clippy -p eggsec-gui --no-default-features --all-targets -- -D warnings` | clean |
| `cargo clippy -p eggsec-daemon-protocol --all-targets -- -D warnings` | clean |
| `cargo clippy -p eggsec-frontend-client --all-targets -- -D warnings` | clean |
| `bash scripts/check-architecture-guards.sh` | **ALL PASSED: No architecture drift detected.** 175 distinct checks, highest number 176 |
| `python3 scripts/check_doc_references.py` | **Total missing: 29** — unchanged baseline |
| `make check-msrv` | exit 0 |

### 4.1 Guard negative tests

Each case injected **one** violation, asserted the injection landed, asserted
the guard failed with its expected message, and then asserted the guard passed
again after the file was restored from a `cp` backup. The restore deliberately
does **not** use `git checkout --`: that would revert every uncommitted change
in the file, not just the injected one.

| Guard | Injected violation | Expected message | Result |
|---|---|---|---|
| 173 | the daemon's refusal message restored to "not wired yet" | "the daemon still refuses policy approval as unwired" | **PASS** |
| 174 | `eggsec-policy` added to the runtime's `Cargo.toml` | "the runtime depends on the policy crate" | **PASS** |
| 175 | `mo.allow_high_risk = true;` re-added to the TUI's `enforcement_facade.rs` | "maps a confirmation class onto an override field itself" | **PASS** |
| 176 | the audit action renamed away from `approve-policy` | "does not record the approval under its own action" | **PASS** |

**4 passed, 0 failed.** The working tree was verified afterwards to contain no
leftover injection (the four injected markers are absent).

### 4.2 The end-to-end proof

`crates/eggsec-daemon/tests/policy_approval.rs` proves the whole chain twice:

1. **At the daemon's command surface.** A manual client declares itself, opens
   a `DesktopGuiManual` session, submits a task, receives the
   `PolicyDecisionRequired` prompt, answers it, and the held operation
   completes with its outcome.
2. **Over the real Unix socket, through the frontend-neutral contract.** The
   same sequence runs through `DaemonRuntimeClient` — connect, create session,
   subscribe, submit, observe the prompt, `approve_task`, observe the held
   operation run. This is the proof the milestone's acceptance criterion #2
   asks for: *the client answers through the `RuntimeClient` contract, and the
   held operation then runs*.

The executor in that suite is a deliberate stand-in that holds at the
confirmation point through the runtime's approval gate — exactly as the
engine's executor does — so the daemon-side proof needs neither the engine nor
the network. The engine's own hold and its bounded resume are proved
separately, in `crates/eggsec/src/runtime_bridge/executor.rs`.

## 5. Invariant review

| Invariant | How it held |
|---|---|
| I1 — `EnforcementContext::evaluate()` remains the mandatory pre-dispatch gate | The approval path **re-runs** the existing authorisation call. The executor never constructs an `ApprovedOperation` or `ApprovedExecution`; it passes an override to `approve_run_request_bundle` and lets the kernel decide. |
| I2 — `OperationMetadata` remains the single source of operation policy | No new policy check was introduced. The required classes come from the kernel's own decision. |
| I3 — strict surfaces fail closed | A strict surface never registers a hold, so there is never an approval for a strict client to give. Proved by `a_strict_surface_never_holds_the_operation` and by the daemon's `InvalidState` refusal. |
| I4 — scope enforcement unchanged | The retry re-runs the whole authorisation path, scope included. An approval does not relax the scope check. |
| I5 — the engine's runtime bridge imports no daemon, CLI, TUI, or daemon-protocol types | The bridge uses only `eggsec_runtime` and `eggsec`'s own config and policy re-exports. The hold rides in `RuntimeExecutionContext`, which is the runtime's own type. |
| I6 — the GUI links no engine, daemon, CLI, or TUI crate | Unchanged; the GUI reaches the daemon through `eggsec-frontend-client` and names no `ClientCommand` in `commands.rs`. |
| I7 — the runtime stays policy-neutral | The registry stores a prompt, not a class. Guard 174 enforces it. |
| I8 — no parallel lookup tables | `ManualOverride::permitting_classes` is the only class→override table; the TUI's duplicate is deleted. Guard 175 enforces it. |
| I9 — no `let _ =` or `filter_map(|e| e.ok())` | No such construct was added. Every fallible step in the new code is handled or logged through `tracing`. |
| I10 — spawned Tokio tasks keep a 30–300 s timeout wrapper | No new task was spawned. The hold is awaited inside the executor's existing task, which the runtime already bounds with the task timeout. |
| I11 — ring-only TLS; Tokio baseline unchanged | No dependency or feature changed. |
| I12 — the published and admitted capability sets stay the same object | Unchanged. This milestone touches neither set. |

## 6. Failure and recovery review

| Situation | Behaviour | Evidence |
|---|---|---|
| The operator approves | The held operation resumes, re-runs authorisation with the bounded override, and dispatches | `a_held_operation_resumes_when_the_client_answers` |
| The operator rejects | The task fails with the operator's reason; the pending entry is removed | `a_refused_operation_stays_refused` |
| The operator never answers | The task's 300 s timeout fires; the pending entry is removed with the task | The runtime's timeout wraps the executor; the entry cannot outlive its task |
| The task is cancelled while held | The cancel token wins the race; the pending entry is removed | `a_cancelled_waiter_drops_its_pending_decision` |
| The daemon restarts while a task is held | The registry is lost with the process and the held task dies with it | Recorded, not hidden — §10 F3 |
| Two clients answer the same prompt | The first wins; the second finds nothing pending and is refused as invalid state | `a_second_answer_for_the_same_task_is_refused` |
| A strict client answers | Nothing is pending, because a strict surface never holds; the answer is refused as invalid state | `a_strict_surface_never_holds_the_operation` |
| A client without the approver role answers | The RBAC refuses it before the handler runs, and the decision stays pending | `a_client_without_the_approver_role_cannot_answer` |

## 7. Migration and compatibility review

- **Wire protocol**: unchanged. `ClientCommand::ApprovePolicy` already carried
  `session_id`, `task_id`, `approved`, and `reason`; `ErrorCode::InvalidState`
  already existed with the doc "no pending policy decision"; and
  `RuntimeEvent::PolicyDecisionRequired` and `PolicyPrompt` already existed
  with no producer. **No message shape changed.**
- **Protocol version**: unchanged at 2. The daemon's behaviour in response to
  a command that previously always failed changed; the command's shape did not.
- **`RuntimeClient` trait**: gains one method — source-breaking for external
  implementors. Both in-tree implementations are updated in the same change.
- **`RuntimeExecutionContext`**: gains one field — source-breaking for
  external constructors; the only in-tree construction site is updated in the
  same change.
- **`RuntimeError`**: gains one variant — a non-breaking addition for a
  non-`exhaustive` enum.
- **Storage**: no migration. The approval registry is in-memory and
  per-process.
- **Behavioural compatibility**: a client that asks to approve a task with
  nothing pending previously received `ErrorCode::Unsupported` ("not wired
  yet") and now receives `ErrorCode::InvalidState` ("no pending policy
  decision for task …"). The new refusal is narrower and more truthful.

## 8. Security review

- **Authorisation is bounded by the kernel, not by the client.** The answer
  permits exactly the confirmation classes the kernel demanded, computed by
  the kernel. The client names *yes* or *no*.
- **The mapping cannot widen.** `permitting_classes` never sets `assume_yes`
  (prompt suppression) and never sets `allow_db_pentest` (an alternative
  grant for the same high-risk class, not a wider one). The one widening that
  remains — `OutOfScope` and `TargetExpansion` sharing a flag — is inherent
  to the override's shape and is the TUI's pre-existing behaviour.
- **Scope is not relaxed.** The retry re-runs the whole authorisation path.
- **A strict surface cannot acquire an operator.** It never holds.
- **The answer is audited.** Under the action `approve-policy`, with the
  answering client's identity, the session, the session's real surface, and
  the outcome.
- **The operator's reason is text.** It is stored as text by the audit trail
  and is never evaluated, interpolated into a command, or echoed into a shell.
- **Replay is bounded.** An answer resolves one pending entry, which is
  removed on use; a second answer finds nothing pending.
- **The pending decision cannot outlive its task.** A dropped waiter removes
  its registration, and a stale drop cannot remove a later decision for the
  same task.

## 9. Documentation and operations

Updated in this milestone:

- `docs/DAEMON.md` — a new "Answering a policy decision" section.
- `docs/ENFORCEMENT_MODES.md` — the paragraph that recorded the approval path
  as unwired.
- `docs/GUI_DISTRIBUTION_HANDOFF.md` — known gap 4.
- `docs/CI_ARCHITECTURE_GUARDS.md` — the four new guards.
- `architecture/daemon.md` — the approval path, invariant 4, the error-code
  table, and the corrected HTTP route.
- `architecture/desktop_gui.md` — the approval path, the known gap, invariant
  13, and the IPC count.
- `architecture/runtime.md` — the pending-approval registry, the module table,
  the error count, and the execution-context fields. **This document also
  carried a stale statement from milestone 010** — the `supported_task_kinds()`
  default was still described as `RuntimeCapabilities::daemon_conservative()`,
  which M010 had already replaced with the canonical list. Corrected here.
- `architecture/overview.md` — the IPC command count.
- `.opencode/skills/eggsec-daemon/SKILL.md` — the approval path and the audit
  record.
- `.opencode/skills/eggsec-tui/SKILL.md` — the approval path and the shared
  mapping.
- `.opencode/skills/eggsec-gui/SKILL.md` — the approval affordance and the IPC
  count.
- `.opencode/skills/eggsec-frontend-client/SKILL.md` — the trait's method set
  and `approve_task`.

## 10. Unresolved findings

**F1 — `ADR-0012` is `proposed`, not `accepted`.** The milestone implemented
the ADR as written. Acceptance is the maintainer's decision, and the ADR is
left `proposed` rather than self-accepted. The implementation plan's §14
stop-condition 1 required stopping if the ADR had been *amended* before
acceptance; it was not amended.

**F2 — the audit record does not carry the task id or the operator's reason.
This is a deviation from the plan's §6 wording, recorded here rather than
silently accepted.** The plan asked the daemon to record "the answering
client's identity, the session, the task, the outcome, and the reason". The
daemon's audit schema (`audit_events`) has columns for the action, the
surface, the outcome, the client, the session, and the timestamp — and no
others. Adding columns would need a real migration: the store's DDL uses
`CREATE TABLE IF NOT EXISTS`, so a new column is silently absent from an
existing database and the insert would fail. The milestone therefore records
the answer with the fields the schema already has. This is adequate because a
session holds **at most one active task** (`max_active_tasks_per_session: 1`),
so the session identifies the decision; and the operator's free-text reason is
**not persisted**. The reason is carried to the executor, which logs the
approval, but it does not survive in the audit trail. **A richer audit record
is a separate, explicit request** — one that should decide whether the
migration is worth it.

**F3 — pending approvals are not persisted.** A daemon restart drops them, and
the held task dies with the process. Recorded here rather than hidden.

**F4 — the approval is bounded to the confirmation classes, not to a
re-decision.** The operator cannot widen the grant at answer time. Widening is
the CLI's `--allow-*` flags, decided before the run. This is deliberate
(ADR-0012's Decision §3) but it means the answer-time decision is narrower
than the CLI's pre-run decision.

**F5 — `make check` cannot complete.** It stops at the inherited `check-deps`
advisory backlog (23 entries across 20 advisory IDs, unchanged). Owned by
milestone 015, not by this milestone.

**F6 — `make check-python` was not run.** It requires `.venv-ci/`, which is
not present. This milestone changed no Python binding, stub, or script — the
Python crate's 237 tests pass unchanged — so the gate was not required by the
plan's §11.

**F7 — `cargo clippy -p eggsec-policy --all-targets` and
`cargo clippy -p eggsec-python --all-targets` are not clean.** 8 and 3
pre-existing findings respectively, in lines this milestone did not add. The
`make clippy` contract lints the policy crate with `--lib`, which is clean.

**F8 — the `service` profile is not clippy-clean at `--all-targets`.** 80
pre-existing engine findings in the files it newly compiles. Pre-existing and
out of this milestone's scope.

**F9 — the TUI's local approval flow is unchanged.** It keeps its own prompt
and its own local flow; only its class→override *mapping* was refactored onto
the shared helper. The TUI does not use the runtime's gate.

**F10 — the CLI's `packet capture` still resolves against the `packet`
operation.** Unchanged by this milestone, and deliberately so: milestone 010's
acceptance criterion requires `packet` to stay `ExplicitScopeRequired`.

## 11. Roadmap disposition

The roadmap's **gap 4** ("interactive approval unwired") is **closed** by this
milestone. The roadmap's milestone 11 entry is marked **implemented
2026-10-24** with its closure record named, and the roadmap's status line now
reads "M10 and M11 implemented 2026-10-24; M12–M15 planned 2026-10-24".

Milestone 11's exit conditions are all met:

| Exit condition | Status |
|---|---|
| a manual client submits an operation that requires confirmation, observes the decision, answers it, and the operation runs | met — §4.2's two proofs |
| a strict client of the same daemon is still refused | met — §6 |
| a denied or unanswered decision leaves the operation refused rather than pending forever | met — §6 |

**Milestone 011's operator-visible outcome was, in the end, mostly outside the
GUI crate.** The hold lives in the runtime, the answer path in the daemon and
the client contract, and the GUI's share is the prompt's wording and two
buttons. That is recorded here because milestone 010's operator-visible
outcome depends on it: without the approval path, a manual daemon client can
start a confirmation-required operation but cannot finish it.

## 12. Registry updates

`plans/registry.md`:

- The subsystem-standing row for `desktop-gui-frontend` records that M10 and
  M11 are implemented.
- The subsystem section's status line reads "M10–M11 implemented 2026-10-24;
  M12–M15 planned 2026-10-24".
- `ADR-0012` is listed with its `proposed` status and its subject.
- The plan list gains the milestone 011 entry with its closure record and the
  four new guards.
- The "five milestones remain planned" sentence is now "four", and the
  approval path is recorded as implemented rather than as a prerequisite.
