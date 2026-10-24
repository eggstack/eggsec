# ADR-0011: The daemon's manual capability policy

Status: proposed

Date: 2026-10-24

Decision owners: project maintainers

Related specification sections:

- `plans/000-long-term-specification.md#4-enforcement-invariants`
- `plans/000-long-term-specification.md#6-dispatch-runtime-and-wire-identity`
- `plans/000-long-term-specification.md#10-frontend-rules`
- `plans/001-terminology-and-domain-model.md`

Affected subsystem roadmaps:

- `plans/subsystems/desktop-gui-frontend-roadmap.md`

## Context

The maintainer's directive, on 2026-10-24: *"we do not want to block any
capability for manual execution."*

That directive is unambiguous as an intent and, as written, it does not yet
name a single mechanism — because the repository has **four** independent
mechanisms that can each block a capability, and they currently disagree
with each other. Before any of them can be changed, the layering has to be
stated precisely, because the naive reading ("the daemon's runtime policy is
conservative, widen it") points at the wrong code.

### The four layers, measured

| # | Layer | What it decides | Where | Current state |
|---|---|---|---|---|
| 1 | **Publication** | What the daemon *advertises* in its `Capabilities` handshake | `crates/eggsec-daemon/src/host.rs:104-110` | `daemon_conservative() ∩ executor declaration` → **20** of 30 |
| 2 | **Executor declaration** | What the engine's executor says it supports | `crates/eggsec-runtime/src/runtime.rs:68-70` (trait default) | `daemon_conservative()` → **20** of 30 |
| 3 | **Runtime admission** | What `Runtime::submit` will accept | `crates/eggsec-runtime/src/runtime_config.rs:32` | `full_lab()` → **29** of 30 |
| 4 | **Engine enforcement** | Whether the operation may run *at all* | `crates/eggsec-policy/src/policy.rs:106-128`, `crates/eggsec-policy/src/decision.rs:1003-1013` | the operator's `[execution_policy]` risk flags; default **deny** for every hazardous family |

Two consequences follow immediately, and both were verified against the source
rather than assumed.

**First, the daemon already admits what it does not advertise.** Layer 3 is
`full_lab()`, not the conservative set: `RuntimeConfig::default()` sets
`capabilities: RuntimeCapabilities::full_lab()`
(`crates/eggsec-runtime/src/runtime_config.rs:32`) and `DaemonHost::new`
builds its runtime with that default (`crates/eggsec-daemon/src/host.rs:119`).
A daemon client can therefore already *submit* `stress-test`,
`wireless-active`, `db-pentest`, `intercept` and `c2`; they are not
rejected as `RuntimeError::UnsupportedTaskKind`. The conservative set governs
only what the daemon *says*. The daemon has never actually refused hazardous
work at the runtime layer.

**Second, widening publication does not unblock the hazardous families.**
Layer 4 is the real gate, and its denial is a **hard deny**: the
risk-policy denial is explicitly excluded from the confirmation path
(`crates/eggsec-policy/src/decision.rs:1361-1372`), so no `--yes` or
`--allow-*` flag can downgrade it. With a default `ExecutionPolicy`,
`stress-test`, `wireless-active`, the packet family, `db-pentest`,
`intercept` and `c2` are all refused outright, whatever the daemon
advertises.

### The remaining gaps in the layering itself

- **`resume` is in neither hand-written list.** The canonical set has 30
  capability names; `daemon_conservative()` has 20 and `full_lab()` has 29.
  The missing name is `resume` — a distinct wire kind that shares the
  `pipeline` operation identity
  (`crates/eggsec-runtime/src/request.rs:630-636`) and is fully implemented
  in the engine (`crates/eggsec/src/dispatch/canonical_execution.rs:1451`
  → `run_pipeline_resume`). It is advertised nowhere because the lists are
  hand-written, not derived.
- **The lists are hand-written, and one of them is duplicated.** The
  capability names live as string literals in `daemon_conservative()`,
  `full_lab()`, and a private `task_kind_name()`
  (`crates/eggsec-runtime/src/capabilities.rs:142-175`) that duplicates
  `TaskKind::capability_name()`
  (`crates/eggsec-runtime/src/request.rs:562-598`) — two parallel tables
  that the repository's own rule ("engine helpers delegate to it, no
  parallel tables") forbids.
- **A recovered session enforces a different set than a fresh one.**
  `RuntimeSession::hydrate_from_snapshot` sets
  `capabilities: RuntimeCapabilities::default()` — the conservative set —
  ignoring both the runtime's configured capabilities and the snapshot's own
  persisted field. After a daemon restart, recovered sessions enforce 20
  while fresh sessions enforce 29.
- **Four of the ten withheld kinds cannot be submitted at all**, whatever the
  capability set says. `packet-capture`, `wireless`, `wireless-active` and
  `resume` carry no target in their params, and their operations
  (`packet`, `wireless`, `wireless-deauth`, `pipeline`) are all
  `TargetPolicyKind::ExplicitScopeRequired`. The descriptor therefore fails
  to build. Publishing them as "available" would put a live button in the
  GUI that cannot work — the exact failure milestone 006 was built to avoid.
- **A manual daemon client cannot answer a confirmation.**
  `ClientCommand::ApprovePolicy` returns `ErrorCode::Unsupported`
  (`crates/eggsec-daemon/src/host.rs:928-932`) and the engine's executor
  passes `manual_override: None`
  (`crates/eggsec/src/runtime_bridge/executor.rs:198`). So even with the policy
  flag set, the operation moves from "hard deny" to "requires confirmation"
  and then stops there. `RuntimeEvent::PolicyDecisionRequired` exists in the
  runtime but **nothing raises it**.
- **The `wireless-active` wire kind is authorised under the passive
  operation's policy.** `TaskKind::WirelessActive` maps to operation id
  `wireless` (`crates/eggsec-runtime/src/request.rs:653`) — risk `SafeActive`,
  capability `PassiveFingerprint`, feature `wireless`. The catalog's
  `wireless-deauth` operation (`crates/eggsec-policy/src/catalog.rs`,
  risk `Intrusive`, capability `RawPacketProbe`, feature `wireless-advanced`)
  is the one the CLI's multiplexer resolves against
  (`crates/eggsec/src/commands/registry.rs:406-407`,
  `crates/eggsec/src/commands/handlers/wireless.rs:69-70`), and **no runtime
  task kind maps to it**. The `Resume` mapping carries an explanatory
  comment; this one does not, and the two paths authorise the same
  deauth work under different risk levels.

## Decision drivers

- **The directive.** No capability may be blocked for manual execution.
- **The roadmap's own completion bar.** `desktop-gui-frontend-roadmap.md`
  §11 closes only when "a desktop GUI exists that reaches the engine's full
  operation surface through the daemon protocol alone" and "every operation
  is reachable from the GUI without per-operation frontend code". That is
  unmet today, and this ADR is the decision that unblocks it.
- **Enforcement must not be weakened.** The capability set is an
  *advertisement and admission* concern. The operator's risk-policy flags
  are the *authorization* concern. They are separate mechanisms in separate
  crates, and widening one provably does not touch the other.
- **The operator's consent gate must survive.** "Nothing is blocked" must
  not mean "nothing needs the operator to say yes". The existing
  `[execution_policy]` flags and the confirmation classes are that gate, and
  they are already operator-configurable.
- **The GUI must not offer a button that cannot work.** A published
  capability the client cannot actually run is the failure mode milestone
  006's coverage model was built to prevent.
- **The published capability list is a durable wire contract.** It is
  consumed by the frontend client and by the GUI, and widening it is a
  compatibility event, not a local refactor.
- **One derived list, not four hand-written ones.** Every drift found above
  — `resume` missing, the duplicated name table, the hydration inconsistency
  — has the same root cause: the lists are transcribed instead of derived.

## Considered options

### Option A — Widen publication to the 26 target-bearing kinds

Publish every capability name whose operation can carry a target
(`stress-test`, `traceroute`, `packet-send`, `db-pentest`, `intercept`,
`c2`), and leave the four interface-bound kinds unpublished.

- **Benefits.** Smallest possible change to the published contract. No
  policy-crate change. The GUI gains six hazard families it can genuinely
  run.
- **Costs.** Does not satisfy the directive: four capabilities stay blocked.
  It also institutionalises the hand-written list — the four exclusions
  would be another hand-maintained carve-out, and the next kind would be
  forgotten the same way `resume` was.
- **Failure modes.** The GUI reports four kinds as unavailable with no
  principled reason a reader could check.

### Option B — Widen publication to all 30 and relax the shared operations' target policy

Publish all 30 canonical capability names, and change `packet`, `wireless`
and `pipeline` from `ExplicitScopeRequired` to `OptionalTarget` so the
four interface-bound kinds can build a descriptor.

- **Benefits.** Reaches all 30 with no new policy concept.
- **Costs.** The catalog is **operation**-granular while the wire kinds are
  finer: `packet` covers `packet-capture` (no target), `traceroute` and
  `packet-send` (targets); `wireless` covers `wireless` and
  `wireless-active` (neither with a target); `pipeline` covers `pipeline`
  (target) and `resume` (no target). Relaxing the shared operation to
  `OptionalTarget` would also make **`packet-send` runnable with no target**
  — a `RawPacket` operation that writes raw frames to an unspecified
  destination. That is a security regression, not a trade.
- **Failure modes.** A raw-packet operation with no destination.

### Option C — Widen publication to all 30, and give interface-bound operations their own target policy

Publish all 30, and resolve the four interface-bound kinds with a distinct
target-policy value meaning "this operation binds to a local interface, not
to a network target", rather than by relaxing the operations they share.

- **Benefits.** The concept already exists in prose — `canonical_target()`
  documents `None` as being for "`NoTarget` or **interface-bound**
  operations" (`crates/eggsec-runtime/src/request.rs:660-663`) — but has no
  representation in `TargetPolicyKind`
  (`crates/eggsec-policy/src/target.rs:173-179`). Making it explicit lets
  the scope check apply to the local host rather than to a remote target,
  which is what an interface-bound operation actually means.
- **Costs.** A change to the policy crate's target lattice, which every
  surface's target validation depends on. It is the largest change of the
  three and needs its own guard.
- **Failure modes.** A new lattice value that some validation path forgets,
  silently admitting an operation with no target where one was required.

### Option D — Change the `ExecutionPolicy` defaults so nothing is blocked

Leave the capability sets alone and set every `allow_*` risk flag to `true`
by default.

- **Benefits.** Literally satisfies "nothing is blocked" in one edit.
- **Costs.** It removes the operator's explicit consent gate for hazardous
  work by default, for **every** surface, including the strict ones. The
  strict surfaces fail closed on *policy*, not on the capability set; a
  default-on risk policy would hand an autonomous agent or a CI run
  unattended `RawPacket` and `DbPentest` capability. The capability set is
  not where this belongs.
- **Failure modes.** Hazardous operations become the default everywhere.

## Decision

**The capability set becomes one derived list, used for both publication and
admission; the enforcement layer is left exactly as it is; and daemon-mediated
approval becomes a prerequisite rather than a follow-up.**

Specifically:

1. **One derived capability list.** The workspace holds exactly one list of
   capability names, derived from the canonical `TaskKind` set
   (`TaskKind::capability_name()` over every variant) and covering all 30
   names, `resume` included. The hand-written `daemon_conservative()` and
   `full_lab()` lists are retired, and the private duplicate
   `task_kind_name()` table is deleted in favour of delegating to
   `TaskKind::capability_name()`. A guard MUST pin that the list is derived,
   so that adding a task kind cannot again leave it unadvertised.

2. **Publication and admission agree by construction.** The daemon's
   published set is the derived list intersected with the executor's
   declaration, and the daemon's **admitted** set is that same
   intersection. The daemon therefore never advertises a capability it will
   refuse, and never refuses one it advertised. The no-op daemon still
   publishes nothing and admits nothing.

3. **The engine's executor declares the derived list**, so the daemon's
   publication reaches every canonical name the engine can execute.

4. **Recovered sessions enforce the runtime's set.** Session hydration stops
   substituting the conservative default, so a restarted daemon's recovered
   sessions admit exactly what its fresh sessions admit.

5. **Enforcement is untouched.** The operator's `[execution_policy]` risk
   flags remain the sole gate for the hazardous families, and the
   confirmation classes remain the sole path past them. This ADR records the
   proof that widening the capability set does not reach them: the admission
   check reads a list of capability *names* from the session, while the
   enforcement gate reads the operation descriptor's risk, required
   features, required capabilities, the execution policy, the loaded scope
   and the enabled-feature set; the enforcement gate runs **inside** the
   executor, which is only reached after admission passes; and the
   approved-execution bundle re-validates exact descriptor binding before
   dispatch. Widening names cannot weaken any of that.

6. **"Nothing is blocked for manual execution" includes the confirmation
   step.** Because a manual daemon client currently cannot answer a
   confirmation, the directive is unsatisfiable without daemon-mediated
   policy approval. That work is therefore a **prerequisite** of the
   capability work, not a later improvement. The daemon must be able to
   hold an operation at its confirmation point, expose that decision to the
   client, accept the client's answer, and dispatch the held operation.

7. **A capability the client cannot run is not published as available.**
   The four interface-bound kinds (`packet-capture`, `wireless`,
   `wireless-active`, `resume`) are resolved by giving interface-bound
   operations an explicit target-policy value (Option C's concept), never by
   relaxing the operations they share. Until a kind has a coherent target
   story, it is reported with an honest reason rather than offered as a
   button that fails.

8. **The `wireless-active` operation identity is corrected before
   `wireless-active` is published.** The runtime's mapping to the passive
   `wireless` operation — and the resulting divergence from the CLI's
   `wireless-deauth` resolution — is a defect independent of this ADR, and
   it MUST be closed before the daemon advertises the kind. A deauth task
   must not be authorised under the passive operation's risk level.

9. **The capability set stays surface-agnostic.** There is no "manual
   capability list". A manual surface and an automated surface of the same
   daemon see the same published list; what differs is the *authorisation*
   applied afterwards, which is the enforcement layer's business. A
   profile-dependent capability set would duplicate a policy that already
   exists, in a place where it would be enforced less well.

10. **The feature gap stays a separate fact.** A published kind whose engine
    feature is compiled out is denied at dispatch with `FeatureMissing`, not
    with `UnsupportedTaskKind`. The GUI already distinguishes these, and
    the distinction is preserved: "published" means the daemon can execute
    it, not that the binary has the feature.

## Consequences

### Positive

- The daemon advertises every canonical capability the engine can execute,
  so the GUI's coverage table stops reporting a twenty-kind ceiling.
- Publication and admission agree, so the "advertises twenty, executes
  twenty-nine" inconsistency disappears.
- `resume` becomes reachable, and no future task kind can be forgotten in
  the same way, because the list is derived.
- The duplicated capability-name table is deleted, restoring the
  single-table rule.
- Recovered sessions stop enforcing a different set than fresh ones.
- The operator's consent gate for hazardous work is preserved and made
  usable from a remote client.

### Negative

- The published capability list is a **breaking change for any consumer that
  assumes a fixed set**, and the GUI's coverage model changes shape: kinds
  that were `not_advertised` become `available` or become available with a
  different reason.
- The hazard families become *offered* to a manual client. They remain
  refused by default, but a client now sees them and can attempt them,
  which is a visible change in the product's posture even though the
  authorisation outcome is unchanged.
- The policy crate gains a target-policy value, which touches every
  surface's target validation.
- The daemon gains a suspend-and-resume path for held operations, which is
  new runtime machinery.

### Neutral or deferred

- The strict surfaces are unaffected: they fail closed on policy, and the
  capability set is not their gate. A strict client of the same daemon
  still cannot authorise a hazardous operation.
- The CLI and the TUI are unaffected in behaviour. They already run these
  operations directly against the engine, and the TUI already has its own
  local interactive approval flow.
- The Python bindings' surface is unaffected.
- The TUI's hand-built forms are not migrated by this ADR. That remains the
  ADR-gated decision the subsystem roadmap already records.

## Compatibility and migration

- **Wire.** The `Capabilities` handshake's `runtime.task_kinds` list widens.
  The field is already additive and already has a `#[serde(default)]`-shaped
  tolerance on the consumer side, so an old client reading a new daemon
  receives a longer list rather than a parse failure. A new client reading
  an old daemon receives the old twenty and MUST continue to degrade
  honestly — the GUI's coverage model already does this, reporting the
  difference rather than hiding it.
- **No storage migration.** No persisted state changes shape. The session
  snapshot's capability field is not authoritative — hydration already
  ignores it — and the corrected hydration makes that explicit.
- **No protocol version bump.** The change adds list entries; it does not
  alter a message shape. The desktop GUI hard-refuses to connect on a
  protocol mismatch, so the version stays at `DAEMON_PROTOCOL_VERSION = 2`.
- **Operator configuration.** Nothing is required of an operator to keep
  today's behaviour: the risk-policy defaults are unchanged, so the hazard
  families remain refused unless an operator opts in.

## Security and reliability implications

- **Authorisation is unchanged.** The capability set is not an
  authorisation mechanism. The proof is recorded in the decision: the
  admission check reads names, the enforcement gate reads risk, features,
  capabilities, policy, scope and enabled features, and it runs inside the
  executor behind the approved-execution bundle's exact descriptor
  re-validation.
- **The operator's consent gate is strengthened, not weakened.** Today a
  manual daemon client cannot satisfy a confirmation at all; after this
  decision it can, but only by an explicit, deliberate act.
- **The published set no longer implies "safe".** A consumer MUST NOT read
  "published" as "permitted". The GUI's coverage model already reports a
  distinct reason per verdict; this ADR adds the requirement that a
  published-but-policy-denied kind be reported as such, never as available.
- **The `wireless-active` identity defect is a live authorisation
  divergence** and is closed as part of this work rather than after it. Until
  it is closed, `wireless-active` MUST NOT be published.
- **Four interface-bound kinds carry no network target.** A new target-policy
  value must make the scope check apply to the local host, so that an
  interface-bound operation is authorised against the machine it runs on and
  not against an absent target.
- **Denial of service.** The admitted set widens, so a manual client may now
  submit operations that consume raw sockets, saturate a link, or drive a
  wireless interface. That is the directive's intent, and the operator's
  policy flags remain the gate.

## Verification

An implementation of this decision MUST produce:

- a guard proving the capability list is **derived** from the task-kind
  enum, so that no hand-written list can reappear and so that a new task
  kind is advertised automatically;
- a test proving the daemon's published set and its admitted set are the
  same object, and that a no-op daemon publishes and admits nothing;
- a test proving the engine's executor declares every canonical capability
  name;
- a test proving a recovered session admits exactly what a fresh session
  admits;
- a test proving the `wireless-active` wire kind resolves to the
  `wireless-deauth` operation, and that the `wireless-deauth` catalog
  operation is reachable from the runtime;
- a test proving the four interface-bound kinds are either runnable or
  reported with an honest reason — never reported as available and then
  refused;
- a test proving that widening the capability set does not admit a
  policy-denied operation: a `stress-test` submitted to a daemon with the
  default execution policy MUST still be refused, with the risk-policy
  denial, and not with `UnsupportedTaskKind`;
- an end-to-end proof that a manual daemon client can answer a confirmation
  and that the held operation then runs.

## Supersession

None.
