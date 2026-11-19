# Desktop GUI Frontend Milestone 010 — The Daemon's Full Manual Capability Policy

Status: ready for handoff

Repository baseline: branch `feat/auto-20261022-76692679` at commit `6de8294f`
("docs(plans): record the closure of the desktop GUI milestone 006"), with
milestones 001–005 and 007–009 closed or conditionally closed, and milestone
006 implemented.

Source roadmap:

- `plans/subsystems/desktop-gui-frontend-roadmap.md#milestone-10--the-daemons-full-manual-capability-policy`

Authoritative decision:

- `plans/adrs/ADR-0011-daemon-manual-capability-policy.md` (status: proposed)

Related long-term requirements:

- `plans/000-long-term-specification.md#4-enforcement-invariants`
- `plans/000-long-term-specification.md#6-dispatch-runtime-and-wire-identity`
- `plans/000-long-term-specification.md#10-frontend-rules`

Primary class: capability

## 0. Baseline and supersession record

This is a **new** plan, not an amendment: the 16 numbered sections below are
the whole of it, and nothing here is executed. This block records what this
plan **supersedes** in the measured evidence of the plans it follows, because
two of those measurements are the reason this milestone exists.

| # | Earlier plan said | Now true | Superseded by |
|---|---|---|---|
| S1 | "the daemon's conservative policy … 20 kinds" and "the runtime-enforced set is `full_lab()` (29 kinds)" are two hand-written tables that nest (`009`, §3.3) | Both lists are **retired**. One derived list covers every canonical capability name, and the daemon's published set and its admitted set are that same list intersected with the executor's declaration | 010 |
| S2 | "`resume` is in the canonical 30 but in neither hand-written list … M009 **does not** change it" (`009`, §3.3) | `resume` is in the derived list and is advertised. The omission was the symptom of transcription, and transcription is what this milestone removes | 010 |
| S3 | "the published list is the declared support ∩ the daemon's conservative policy" (`006`, §0 A4; `docs/DAEMON.md`, "Capabilities") | The published list is the declared support ∩ the **derived** canonical list. The conservative policy no longer exists as a filter | 010 |
| S4 | "A published kind whose engine feature is off is denied at dispatch with `FeatureMissing`, not `UnsupportedTaskKind`" (`006`, §3.4; `architecture/daemon.md`, invariant 10) | Unchanged. The feature gap stays a separate fact from the capability set | 010 |

The **§0 amendment record of plan 006** is *not* rewritten by this plan. Its
A1/A2 rows remain the record of what milestone 008 changed; this plan's §3
replaces the *measurements* 006 relied on, and the closure record for 010 is
where the new numbers are recorded. Plan 006's "Amendment record" SHOULD be
extended, by whoever owns the plan set, with a pointer to ADR-0011 — this
milestone does not edit it.

**The one dependency this milestone waits on is ADR-0011, and ADR-0011 is
`Status: proposed`.** The roadmap states the dependency as hard: "`ADR-0011`
(**hard** — the milestone waits on its acceptance)". This plan implements
ADR-0011's Decision section verbatim, and **must not be executed until the ADR
is accepted**. If the ADR is amended before acceptance, §14 requires the
implementer to stop and report the divergence rather than follow this plan's
reading of it.

## 1. Objective

Make the daemon **publish and admit every canonical capability name the engine
can execute**, from **one derived list**, so that no capability is blocked for
manual execution at the runtime layer.

Four concrete outcomes in one milestone:

1. **One derived capability list.** The workspace holds exactly one list of
   capability names, derived from the canonical `TaskKind` set and covering
   every canonical name — `resume` included. The two hand-written lists are
   retired and the duplicated private name table is deleted.
2. **Publication and admission agree by construction.** The daemon's published
   set and its admitted set are the same object. The daemon never advertises a
   capability it will refuse, and never refuses one it advertised. A rejecting
   daemon still publishes and admits nothing.
3. **The four interface-bound kinds get a coherent target story**, so that
   `packet-capture`, `wireless`, `wireless-active` and `resume` are either
   genuinely runnable or reported with an honest reason — never offered as a
   button that fails.
4. **The `wireless-active` operation identity is corrected** before
   `wireless-active` is published, so a deauth is never authorised under the
   passive operation's risk level.

**What this milestone explicitly does not deliver.** The hazardous families'
*authorisation* — `stress-test`, `wireless-active`, the packet family,
`db-pentest`, `intercept` and `c2` are refused by the operator's default
`[execution_policy]` risk flags, and a manual daemon client still cannot
answer a confirmation. That is **milestone 011**
(`plans/subsystems/desktop-gui-frontend-roadmap.md#milestone-11--daemon-mediated-policy-approval`),
whose work this milestone **MUST NOT** absorb. This milestone makes those
families *offered*; it does not make them *permitted*.

## 2. Why this milestone is ready

Closed hard dependencies:

- Milestone 009 (closed 2026-10-23) — the published list is already **derived**
  from the executor's declaration (`crates/eggsec-daemon/src/host.rs:104-110`,
  guard 165 at `scripts/check-architecture-guards.sh:5268`). This milestone
  replaces the *filter* inside that derivation; it does not build the
  derivation.
- Milestone 006 (implemented 2026-10-24) — the GUI's coverage model is already
  a **derived** join over the daemon's published task kinds, the operation
  catalog, the advertised capabilities and the published enabled-feature set
  (`crates/eggsec-gui/src/coverage.rs:85-99`, `:106-116`). The GUI has no
  per-operation code to rewrite, so a widened published set reaches it
  automatically.
- Milestone 008 (closed 2026-10-23) — the `service` profile enables the engine
  features the widened set needs: `DAEMON_SERVICE_PROFILE`
  (`crates/eggsec/src/config/daemon_profile.rs:71-92`) already contains
  `packet-inspection`, `stress-testing`, `db-pentest`, `web-proxy`, `c2`,
  `wireless` and `wireless-advanced`.

Open dependency (blocking, by the roadmap's own rule):

- **ADR-0011 is `Status: proposed`.** The roadmap makes its acceptance a hard
  dependency of this milestone. The implementation agent MUST confirm the ADR
  is `Status: accepted` before touching any file.

Stable interface dependencies:

- `TaskKind` — 30 adjacently tagged variants
  (`crates/eggsec-runtime/src/request.rs:72-103`), with the single
  `capability_name()` table (`:562-598`), the `operation_id()` table (`:620-658`)
  and `canonical_target()` (`:666-707`).
- `RuntimeTaskExecutor::supported_task_kinds()`
  (`crates/eggsec-runtime/src/runtime.rs:68-70`) — the declaration seam added
  by milestone 009.
- `OperationMetadata::target_policy` and `TargetPolicyKind`
  (`crates/eggsec-policy/src/target.rs:173-179`) — the target lattice this
  milestone extends.
- `OperationMetadata::try_descriptor_for_target()`
  (`crates/eggsec-policy/src/catalog.rs:106-210`) — the single point where a
  target is validated against a policy.
- The published `OperationDescription.target_policy`
  (`crates/eggsec-daemon-protocol/src/operations.rs:48-49`) — the GUI can
  already read an operation's target policy over the wire, so a new lattice
  value needs no protocol change to be visible to a client.

The milestone is ready **because the derivation seam already exists**. 009
made the published set a function of the executor's declaration; 010 makes
both that set and the admission set a function of one derived list.

## 3. Current implementation evidence

### 3.1 The four layers, and which one actually gates

| # | Layer | Decides | Where | Now |
|---|---|---|---|---|
| 1 | Publication | what the daemon advertises in `Capabilities` | `crates/eggsec-daemon/src/host.rs:104-110` | `daemon_conservative() ∩ executor declaration` |
| 2 | Executor declaration | what the engine's executor says it supports | `crates/eggsec-runtime/src/runtime.rs:68-70` (the trait default) | `daemon_conservative()` |
| 3 | Runtime admission | what `Runtime::submit` accepts | `crates/eggsec-runtime/src/runtime_config.rs:32` | `full_lab()` |
| 4 | Engine enforcement | whether the operation may run at all | `crates/eggsec-policy/src/policy.rs:106-127`, `crates/eggsec-policy/src/decision.rs:1003-1013` | the operator's `[execution_policy]` risk flags; default **deny** for every hazardous family |

**The daemon already admits what it does not advertise.** Layer 3 is
`full_lab()`, not the conservative set: `RuntimeConfig::default()` sets
`capabilities: RuntimeCapabilities::full_lab()`
(`crates/eggsec-runtime/src/runtime_config.rs:32`) and `DaemonHost::new`
builds its runtime with that default
(`crates/eggsec-daemon/src/host.rs:119`). A daemon client can therefore
already submit `stress-test`, `wireless-active`, `db-pentest`, `intercept`
and `c2`; they are not rejected as `RuntimeError::UnsupportedTaskKind`
(`crates/eggsec-runtime/src/runtime.rs:321-333`).

**Widening publication does not unblock the hazard families.** Layer 4 is the
real gate and its denial is a **hard deny**:
`crates/eggsec-policy/src/decision.rs:1361-1372` explicitly excludes the
risk-policy denial from the confirmation path, so no `--yes` and no
`--allow-*` flag can downgrade it. The thirteen `allow_*` flags that default
to `false` are at `crates/eggsec-policy/src/policy.rs:110-122`, and
`max_risk_without_confirm` defaults to `OperationRisk::SafeActive` (`:123`).

### 3.2 The canonical sets, measured from source

| Set | Value | Owner |
|---|---|---|
| `TaskKind` variants | 30 | `crates/eggsec-runtime/src/request.rs:72-103` |
| distinct capability names | 30 | `TaskKind::capability_name()`, `:562-598` |
| distinct operation ids | 26 | `TaskKind::operation_id()`, `:620-658` |
| catalog operations | 34 | `ALL_OPERATION_METADATA`, `crates/eggsec-policy/src/catalog.rs:291` |
| `daemon_conservative()` | 20 | `crates/eggsec-runtime/src/capabilities.rs:45-74` |
| `full_lab()` | 29 | `crates/eggsec-runtime/src/capabilities.rs:77-115` |

The **ten withheld kinds** — the 30 canonical names minus the 20 conservative
names — are `stress-test`, `resume`, `packet-capture`, `traceroute`,
`packet-send`, `wireless`, `wireless-active`, `db-pentest`, `intercept` and
`c2`.

**`resume` is in neither hand-written list.** `full_lab()` has 29 entries and
`"resume"` is not one of them. `TaskKind::Resume` maps to capability name
`"resume"` (`crates/eggsec-runtime/src/request.rs:576`) and operation id
`"pipeline"` (`:636`). The engine fully implements it:
`crates/eggsec/src/dispatch/canonical_execution.rs:1451-1452` dispatches
`CanonicalOperationRequest::Resume` to `super::recon::run_pipeline_resume`.

**Two parallel name tables exist, and the repository forbids parallel tables**
(`AGENTS.md`, "Dispatch / scope / runtime ownership"): the public
`TaskKind::capability_name()` (`:562-598`) and the private
`fn task_kind_name()` (`crates/eggsec-runtime/src/capabilities.rs:142-175`).
The **private one is the one the admission check actually calls**
(`crates/eggsec-runtime/src/capabilities.rs:135-138`, inside
`supports_task_kind`).

### 3.3 Five defects this milestone must close

**D1 — `resume` is unadvertised**, because the lists are hand-written rather
than derived. There is no policy reason for its absence; the comment at
`crates/eggsec-runtime/src/request.rs:573-575` says the conservative set
"deliberately omits it", and that comment describes the set, not the intent.

**D2 — the executor declaration defaults to the conservative set.**
`EggsecRuntimeExecutor` (`crates/eggsec/src/runtime_bridge/executor.rs:141`)
overrides `enabled_features()` (`:147`) and `report_for()` (`:159`) but **not**
`supported_task_kinds()`, so it inherits the trait default
(`crates/eggsec-runtime/src/runtime.rs:68-70`). **Changing only the daemon's
policy would be inert**: `published_task_kinds()` intersects the policy with
the declaration, so the intersection would stay at the conservative set.

**D3 — hydration resets to the conservative default.**
`RuntimeSession::hydrate_from_snapshot` sets
`capabilities: RuntimeCapabilities::default()`
(`crates/eggsec-runtime/src/session.rs:324`), and
`RuntimeCapabilities::default()` **is** `daemon_conservative()`
(`crates/eggsec-runtime/src/capabilities.rs:33-37`). It ignores both the
runtime's configured capabilities and the snapshot's own persisted field. The
runtime's hydration site is `crates/eggsec-runtime/src/runtime.rs:759`. After
a daemon restart, recovered sessions enforce the conservative set while fresh
sessions enforce `full_lab()`.

**D4 — four of the ten withheld kinds cannot be submitted at all**, whatever
the capability set says.

| Kind | Params | Carries a target? | Operation | Target policy |
|---|---|---|---|---|
| `packet-capture` | `interface`, `filter`, `duration_secs`, `max_packets`, `promiscuous` (`crates/eggsec-runtime/src/request.rs:276-282`) | no | `packet` | `ExplicitScopeRequired` |
| `wireless` | `interface`, `duration_secs` (`:452-455`) | no | `wireless` | `ExplicitScopeRequired` |
| `wireless-active` | `interface`, `target_bssid`, `attack_type`, `client`, `frame_count`, `rate_limit` (`:486-501`) | no | `wireless` | `ExplicitScopeRequired` |
| `resume` | `session_path` (`:261-263`) | no | `pipeline` | `ExplicitScopeRequired` |

`TaskKind::canonical_target()` (`crates/eggsec-runtime/src/request.rs:666-704`)
returns `None` for exactly these four among the ten. Their operations are all
`TargetPolicyKind::ExplicitScopeRequired`
(`crates/eggsec-policy/src/catalog.rs:291` onward), and
`try_descriptor_for_target` rejects a `None` target for that policy with
`DescriptorError::MissingTarget`
(`crates/eggsec-policy/src/catalog.rs:170-178`,
`crates/eggsec-policy/src/target.rs:207-214`). The engine's bridge therefore
fails to build a descriptor
(`crates/eggsec/src/runtime_bridge/descriptor.rs:15-31`,
`crates/eggsec/src/runtime_bridge/descriptor.rs:40-47`). The other six carry a
target: `stress-test` (`target: String`), `traceroute` (`target: String`),
`packet-send` (`target: String`), `db-pentest` (`target: String`),
`intercept` (`target: Option<String>`), `c2` (`target: Option<String>`).

> **A precision the ten-kind table above does not carry.** `intercept` and
> `c2` carry an *optional* target, and their operations
> (`proxy-intercept`, `c2`) are `ExplicitScopeRequired`. A client that omits
> the target therefore fails to build a descriptor **as well**. The plan
> treats "the client supplied a target" as part of those kinds' contract, not
> as something the daemon can supply.

`TargetPolicyKind` (`crates/eggsec-policy/src/target.rs:173-179`) has exactly
five values — `NoTarget`, `OptionalTarget`, `TargetRequired`,
`ExplicitScopeRequired`, `PrivateOrLocalRequired` — and **no** interface-bound
value, although `canonical_target()`'s own doc comment already uses the
phrase ("`None` for `NoTarget` or **interface-bound** operations",
`crates/eggsec-runtime/src/request.rs:660-662`).

`storage`, `integrations` and `workflow` are already published, are
`OptionalTarget`, and are the working precedent for a target-less operation
(`TARGET_LESS_OPERATIONS`, `crates/eggsec-policy/src/catalog.rs:1418`).

**D5 — `TaskKind::WirelessActive` maps to the PASSIVE operation.**
`crates/eggsec-runtime/src/request.rs:653` maps it to operation id
`"wireless"`. The catalog's `wireless` operation
(`crates/eggsec-policy/src/catalog.rs:603-617`) is risk `SafeActive`,
capability `PassiveFingerprint`, feature `wireless`. The catalog's
`wireless-deauth` operation (`crates/eggsec-policy/src/catalog.rs:620-635`) is
risk `Intrusive`, capability `RawPacketProbe`, feature `wireless-advanced`.
The CLI resolves deauth against `wireless-deauth`
(`crates/eggsec/src/commands/registry.rs:406-407`,
`crates/eggsec/src/commands/handlers/wireless.rs:68-70`), and the CLI's
command route multiplexes `["wireless", "wireless-deauth"]`
(`crates/eggsec/src/commands/route.rs:169-175`). **No runtime task kind maps
to `wireless-deauth`.** So a daemon-submitted `wireless-active` is authorised
under the passive policy while the CLI's deauth is authorised under the
intrusive one. The `Resume` mapping carries an explanatory comment
(`crates/eggsec-runtime/src/request.rs:630-635`); this one does not.

> **A second table the D5 fix must move with.** The engine has its own
> operation-identity table over its internal request type:
> `CanonicalOperationRequest::operation_id()`
> (`crates/eggsec/src/dispatch/canonical_execution.rs:288-323`), which maps
> `WirelessActive` to `"wireless"` at `:319`. Its companion routing table
> `executor_route_for()` (`:590-605`) has **no** `"wireless-deauth"` arm, so
> the corrected identity would route to `"unknown"` until that arm is added.
> Both tables move with the wire-side one, or the fix is half-applied.

### 3.4 What changes on the GUI's side

- `crates/eggsec-gui/src/coverage.rs` — `Reachability` has six verdicts
  (`Available`, `NotExposed`, `NotAdvertised`, `FeatureDisabled`, `Unverified`,
  `Blocked`; `:37-53`), and `coverage_for` (`:85-99`) /
  `coverage_for_with_ledger` (`:106-116`) compute one row per published task
  kind from the daemon's published task kinds, the operation catalog, the
  advertised capabilities and the published enabled-feature set. A kind whose
  capability is not advertised is `NotAdvertised` today (`:172-195`); after the
  widening it resolves to `Available` (`:219-224`) — or, for the four
  interface-bound kinds, must carry an honest reason.
- `crates/eggsec-gui/src/availability.rs` — `Denial` (`:23-34`),
  `denial_from_message` (`:68`) and `DenialLedger` (`:102-111`): a learned
  denial wins over every static verdict. The ledger currently recognises only
  `FeatureMissing` and `Other` (`:23-34`); a risk-policy denial arrives as
  `Other` and would be reported as an opaque failure.
- The GUI's IPC surface is **12** commands
  (`crates/eggsec-gui/src/shell.rs:33-46`); `declared_command_set_is_exact`
  (`crates/eggsec-gui/src/commands.rs:993`) and
  `every_command_declares_its_argument_naming` (`:1024`) pin the count. If
  this milestone adds a command, all three move together.
- `crates/eggsec-gui/src/connection.rs:180-188` — the GUI's "advertises no
  task kinds" check. A wider published set must not trip it.
- The GUI's forms are generated from the published request schema
  (`crates/eggsec-daemon-protocol/src/operations.rs:72-82`), so the four
  interface-bound kinds already have target-free forms. No per-operation
  frontend code is needed — guard 161 forbids it.

### 3.5 The tests that will fail when the set widens

| Test | Anchor | Why it fails |
|---|---|---|
| `a_declared_kind_outside_the_policy_is_not_published` | `crates/eggsec-daemon/src/host.rs:3936-3946` | asserts `"stress-test"` is withheld |
| `an_executor_that_declares_nothing_keeps_the_conservative_set` | `crates/eggsec-daemon/src/host.rs:3948-3956` | asserts the published length equals the conservative set's length |
| `the_start_path_never_publishes_a_capability_it_cannot_execute` | `crates/eggsec-daemon/src/host.rs:3976-3983` | asserts a count derived from `daemon_conservative()` |
| `daemon_conservative_includes_safe_kinds` / `daemon_conservative_excludes_hazardous_kinds` | `crates/eggsec-runtime/src/capabilities.rs:219-271` | the constructor they name is retired |
| `full_lab_has_all_task_kinds` | `crates/eggsec-runtime/src/capabilities.rs:274-309` | the constructor it names is retired, and its list omits `resume` |
| the in-process client's capability assertion | `crates/eggsec-runtime/tests/in_process_client.rs:731` | asserts `full.task_kinds.len() == 29` |
| `conservative_capabilities_reject_hazardous_task_kinds` | `crates/eggsec-runtime/src/runtime.rs:1804-1864` | constructs `RuntimeConfig { capabilities: daemon_conservative(), .. }` explicitly, so it survives a `RuntimeConfig::default()` change — **see §7, work package A, for whether it should** |
| a doc comment reading "20 advertised, 0 executable" | `crates/eggsec-daemon/src/host.rs:3910` | the number is stale |

### 3.6 The guards

| Guard | Anchor | Scope |
|---|---|---|
| 159 | `scripts/check-architecture-guards.sh:4768` | the GUI names no task-kind variant |
| 161 | `:4905` | the GUI's coverage model is derived |
| 162 | `:4974` | the GUI's report display carries no remote origin |
| 163 | `:5114` | the `service` profile matches the engine's contract |
| 165 | `:5268` | the daemon host derives its published capabilities — scoped to the `Capabilities` **arm body** only |

Guard 165 forbids the `Capabilities` arm from naming `RuntimeCapabilities::default()`,
`daemon_conservative()` or `full_lab()`
(`scripts/check-architecture-guards.sh:5300-5310`). **The replacement policy
constant MUST therefore live in the `published_task_kinds()` helper, outside
the arm.** The guard also does not need to change: it forbids a *constant* in
the arm and requires a *derived* list, which is what the arm already
publishes (`crates/eggsec-daemon/src/host.rs:411-425`).

The guard script's highest numbered check today is **168**
(`scripts/check-architecture-guards.sh:5450`). Any new guard this milestone
adds MUST be numbered **169 or above** and MUST be described as proposed until
it is merged.

### 3.7 The documents and skills that state the current counts

- `docs/DAEMON.md:22-35` (the "Capabilities" section) and `:91-100` ("8 of the
  20 advertised capabilities are feature-gated off" — true of the
  **CLI-hosted** daemon, which links the engine with the `cli` feature only).
- `architecture/daemon.md:459` (the capability-honesty test list) and `:476`
  (invariant 10, "the published capability list is derived, not declared").
- `architecture/runtime.md:213-217` (the `RuntimeCapabilities` table).
- `architecture/overview.md` — it owns the headline counts; point at it rather
  than restating them.
- `.opencode/skills/eggsec-daemon/SKILL.md:61-69` ("Capability honesty").
- `.opencode/skills/eggsec-gui/SKILL.md:22` (the guard range 155–162) and
  `:24-25` (the pointer to `architecture/overview.md`).
- `plans/implementation/desktop-gui-frontend/006-full-operation-coverage-and-report-export.md`
  §3.4 and §3.7 — superseded by this plan's §3 (see §0).
- `plans/implementation/desktop-gui-frontend/009-daemon-obtainability-and-start-honesty.md`
  §3.2 and §3.3 — superseded by this plan's §3 (see §0).

## 4. Invariants that must not regress

1. **A rejecting executor publishes and admits nothing.**
   `RuntimeCapabilities::noop()` (`crates/eggsec-runtime/src/capabilities.rs:124-132`)
   and every rejecting executor stub — `NoopExecutorStub`
   (`crates/eggsec-daemon/src/host.rs:4019-4027`), the frontend client's
   `NoopExecutor` (`crates/eggsec-frontend-client/src/lib.rs:141-156`) — keep
   publishing an empty list.
2. **`RuntimeTaskExecutor::supported_task_kinds()` keeps its semantics for a
   rejecting executor**: a rejecting executor MUST declare the empty set
   explicitly. The trait default changes (§6), the *rule* does not.
3. **`RuntimeTaskExecutor::enabled_features()` and `report_for()` are
   unchanged.** They were added by milestone 006 as defaulted, additive
   methods; this milestone touches neither.
4. **`TaskResultEnvelope.payload` (`json!({})`) and `.artifacts` (`vec![]`)
   are unchanged.**
5. **`DAEMON_PROTOCOL_VERSION` stays 2**
   (`crates/eggsec-daemon-protocol/src/protocol.rs:15`). The change adds list
   entries and one catalog entry; it alters no message shape.
6. **The daemon's `Capabilities` match arm keeps publishing the host's
   *derived* list** (guard 165). The policy constant lives in the
   `published_task_kinds()` helper, outside the arm.
7. **The strict surfaces — REST, MCP, gRPC, CI, the agent — are untouched.**
   They fail closed on *policy*, not on the capability set. A strict client of
   the same daemon still cannot authorise a hazardous operation.
8. **The operator's `[execution_policy]` risk flags remain the sole gate** for
   the hazardous families, and the confirmation classes remain the sole path
   past them. The risk-policy denial stays a hard deny
   (`crates/eggsec-policy/src/decision.rs:1361-1372`).
9. **The published capability list stays surface-agnostic.** There is no
   "manual capability list"; a manual surface and an automated surface of the
   same daemon see the same published list.
10. **The feature gap stays a separate fact.** A published kind whose engine
    feature is compiled out is denied at dispatch with `FeatureMissing`, not
    with `UnsupportedTaskKind`.
11. **Enforcement is untouched.** The admission check reads capability *names*;
    the enforcement gate reads the descriptor's risk, required features,
    required capabilities, the execution policy, the loaded scope and the
    enabled-feature set, and it runs inside the executor, behind the
    approved-execution bundle's exact descriptor re-validation.
12. **No new `TaskOutcome` variant, no wire-shape change, no protocol bump.**
13. **The GUI adds no per-operation code** and names no task-kind variant
    (guards 159 and 161).

## 5. Scope

### In scope

- **One derived capability list** in `eggsec-runtime`, covering every
  canonical capability name, `resume` included; the retirement of
  `daemon_conservative()` and `full_lab()`; and the deletion of the private
  duplicate name table in favour of `TaskKind::capability_name()`.
- **The engine executor's declaration** — `EggsecRuntimeExecutor` declares the
  derived list.
- **Publication and admission made equal by construction**, in the daemon host
  and in the runtime's session hydration.
- **The four interface-bound kinds' target story**: one new
  `TargetPolicyKind` value, the catalog split it requires, and the scope
  semantics that value carries.
- **The `wireless-active` operation identity fix**, in the wire-side table,
  the engine's internal table, and the engine's executor-routing table.
- **The GUI's honest reporting** of a published-but-policy-denied kind, and
  of the interface-bound kinds.
- **New guards (169+)** and the documentation updates in §12.

### Explicitly out of scope

- **`ApprovePolicy` wiring, and the confirmation path.** That is milestone
  011. This milestone MUST NOT implement `ApprovePolicy`, MUST NOT raise
  `RuntimeEvent::PolicyDecisionRequired`, and MUST NOT add an approval
  affordance to the GUI. `ClientCommand::ApprovePolicy` keeps answering
  `ErrorCode::Unsupported` (`crates/eggsec-daemon/src/host.rs:928-932`).
- **The hazard families' authorisation.** This milestone makes them *offered*;
  it does not make them *permitted*.
- **The socket path work and the bind contract** (roadmap milestones 012 and
  gap 5).
- **The report bridge's coverage** (roadmap milestone 013, gap 11).
- **The platform qualification evidence** (gap 9) and the dependency advisory
  backlog (roadmap milestone 015).
- **A first-class "local host" subject in the scope model.** §6 (Security and
  authorization) states what the new lattice value does *instead*, and names
  the residual gap.
- **Any change to the CLI's or the TUI's user-visible behaviour**, other than
  the recorded consequences of the target-policy change stated in §6 and §9.
- **A protocol version bump, a storage migration, or a new `TaskOutcome`
  variant.**

## 6. Required production changes

### Core/domain

**C1 — one derived capability list.** `crates/eggsec-runtime/src/capabilities.rs`
gains a single derived constructor (the implementer may choose the name; the
plan uses `canonical()`) whose `task_kinds` are derived from the canonical
`TaskKind` set:

- `TaskKind::capability_name()` (`crates/eggsec-runtime/src/request.rs:562-598`)
  remains the **only** name table.
- The private `fn task_kind_name()` (`crates/eggsec-runtime/src/capabilities.rs:142-175`)
  is **deleted**, and `supports_task_kind` (`:135-138`) delegates to
  `TaskKind::capability_name()`.
- `daemon_conservative()` (`:45-74`) and `full_lab()` (`:77-115`) are
  **retired**. `RuntimeCapabilities::default()` (`:34-38`) stops being
  `daemon_conservative()` and becomes the derived list.
- **The derivation must be exhaustive by construction**: adding a `TaskKind`
  variant MUST be a compile error until the derived list covers it. The
  implementer achieves this by adding an exhaustive enumeration of the
  variants to `request.rs` (the plan calls it `TaskKind::all_variants()`)
  whose construction of each variant's payload is exhaustive over the same
  enum. Twelve of the thirty params structs do not currently derive `Default`
  (`PipelineParams`, `ResumeParams`, `ReconParams`, `PacketTracerouteParams`,
  `NseParams`, `HuntParams`, `BrowserParams`, `ComplianceParams`,
  `IntegrationsParams`, `WorkflowParams`, `VulnParams`, `WirelessParams`); the
  implementer adds it to those, which is additive and does not change any
  serde shape.
- **`TaskCapability.description`** is *data*, not identity. The derived list
  MUST carry a description for every name. The implementer keeps a curated
  description map **keyed by capability name** (not by variant), and the new
  guard (§7, work package G) requires that map to cover exactly the derived
  name set — no more, no fewer. This keeps the wire's description strings
  unchanged while the *names* stay derived. The alternative — deriving the
  description from the name by a formatting rule — is acceptable, but it
  changes the wire's description strings and MUST be recorded as a deviation
  in the closure record.

**C2 — the trait default.** `RuntimeTaskExecutor::supported_task_kinds()`
(`crates/eggsec-runtime/src/runtime.rs:68-70`) defaults to the **derived**
list, because ADR-0011 retires the hand-written lists and there is then only
one. The doc comment's rule is kept verbatim in substance: *an executor that
rejects every task MUST declare the empty set explicitly*. Every rejecting
executor in the workspace therefore MUST gain an explicit empty
`supported_task_kinds()` override — `NoopExecutorStub`
(`crates/eggsec-daemon/src/host.rs:4019-4027`, which already declares empty)
and the frontend client's `NoopExecutor`
(`crates/eggsec-frontend-client/src/lib.rs:141-156`, which does not yet).
The new guard in §7 work package G pins this.

**C3 — the engine's executor declares the derived list.**
`EggsecRuntimeExecutor` (`crates/eggsec/src/runtime_bridge/executor.rs:141`)
gains an explicit `supported_task_kinds()` override returning the derived
list. Without this, work package C is inert (defect D2).

### Storage and migrations

- **None.** The session snapshot's `capabilities` field
  (`crates/eggsec-runtime/src/session.rs:362`) stays persisted and stays
  **non-authoritative**: hydration ignores it today, and the corrected
  hydration makes that explicit. No persisted representation changes shape.

### Protocol and DTOs

- **No wire-shape change and no protocol bump.** `DAEMON_PROTOCOL_VERSION`
  stays 2 (`crates/eggsec-daemon-protocol/src/protocol.rs:15`).
- The published `Capabilities.runtime.task_kinds` list **widens**. The field
  is already additive and already tolerant on the consumer side, so an old
  client reading a new daemon receives a longer list rather than a parse
  failure.
- The published `OperationDescription.target_policy` string
  (`crates/eggsec-daemon-protocol/src/operations.rs:48-49`) **gains a new
  value** for the interface-bound lattice entry. It is already a free-form
  label read from the catalog, so this is a new label, not a new field — but
  a client that switches on the label must handle it, and §7 work package G
  requires the GUI to.
- The published `DescribeOperationsResponse.operations` list **grows** by the
  catalog split (see C5). The GUI's join is by operation id
  (`crates/eggsec-gui/src/coverage.rs:106-116`), so a new operation needs no
  per-operation frontend code.

### Runtime and concurrency

**C4 — publication and admission agree by construction.**

- `crates/eggsec-daemon/src/host.rs:104-110` — `published_task_kinds()`
  intersects the **derived** list with the executor's declaration. The policy
  constant lives **here**, outside the `Capabilities` arm, so guard 165 stays
  satisfied.
- `crates/eggsec-daemon/src/host.rs:113-131` — `DaemonHost::new` computes the
  published set **first**, then builds the runtime with
  `RuntimeConfig { capabilities: <the published set>, .. }`, so the runtime
  enforces exactly what the daemon publishes. This is the "same object"
  property the roadmap's exit conditions demand.
- `crates/eggsec-daemon/src/host.rs:134-161` — `DaemonHost::new_noop` is
  unchanged: `RuntimeCapabilities::noop()` and an empty published list.
- `crates/eggsec-daemon/src/host.rs:164-180` — `DaemonHost::with_runtime`
  takes a pre-built runtime whose enforced set is fixed at construction. It
  MUST publish the intersection of the derived list with the runtime's
  **declaration**, and it MUST refuse to construct when the runtime's enforced
  set is not that same intersection — otherwise the host would advertise a
  capability the runtime would refuse. The implementer states the chosen
  failure mode (a typed construction error is preferred over a silent
  narrowing). Measured at the baseline, `with_runtime` has **no production
  caller** — only its own definition — so the change is cheap, but the
  implementer MUST re-measure before assuming that.
- `crates/eggsec-runtime/src/runtime.rs:321-333` — the admission check is
  unchanged in shape: it reads the session's capability set by name. It now
  reads the derived set.
- `crates/eggsec-runtime/src/runtime.rs:145-163` and `:183-189` — `Runtime`
  gains an **additive** accessor for its *enforced* set, so that
  "publication == admission" is assertable by a test rather than assumed. The
  accessor is lock-taking, because the enforced set lives in the runtime's
  `RuntimeState` (`crates/eggsec-runtime/src/runtime.rs:153-158`).
- `crates/eggsec-runtime/src/runtime_config.rs:32` — `RuntimeConfig::default()`
  carries the derived list.
- `crates/eggsec-runtime/src/session.rs:311-327` — `hydrate_from_snapshot`
  stops substituting the conservative default. The recovered session MUST
  enforce **the runtime's configured set**. The implementer chooses the
  mechanism (a new parameter, or the runtime applying
  `RuntimeSession::with_capabilities` — `:162-165` — after hydrating at
  `crates/eggsec-runtime/src/runtime.rs:759`); the property is what matters:
  a recovered session admits exactly what a fresh session admits.
- `crates/eggsec-runtime/src/runtime.rs:1804-1864` —
  `conservative_capabilities_reject_hazardous_task_kinds` constructs
  `RuntimeConfig { capabilities: daemon_conservative(), .. }` explicitly. With
  `daemon_conservative()` retired, the test must be **rewritten, not deleted**:
  its intent — *a runtime configured with a narrow set rejects the kinds
  outside it* — is still a real property, and it is the direct regression test
  for the admission path this milestone touches. The implementer rewrites it
  against the derived list and the explicit narrow-set construction, and
  records the rewrite in the closure.

### Frontend or operator surface

**C6 — the GUI reports the widened set honestly.**

- `crates/eggsec-gui/src/coverage.rs:150-224` — the `resolve` function
  gains the **interface-bound** fact. The plan keeps the six existing
  verdicts and carries the fact in the existing reason string
  (`TaskCoverage.reason`): a kind whose operation's `target_policy` is the
  new interface-bound label resolves to `Available` with a reason that says
  the operation binds to a **local interface**, not a network target. The
  verdict enum is a counted set owned by `architecture/overview.md`; adding a
  seventh verdict is a deviation the implementer MAY take, but MUST then
  update that count and the GUI skill's claims alongside it, and MUST record
  the choice.
- `crates/eggsec-gui/src/availability.rs:23-34` — the `Denial` enum gains a
  **policy-denial** variant, and `denial_from_message` (`:68`) recognises the
  risk-policy denial text (`operation risk '<X>' is not allowed by current
  execution policy`,
  `crates/eggsec-policy/src/decision.rs:1005-1011`). ADR-0011 requires that a
  published-but-policy-denied kind be reported **as such, never as
  available**. The `DenialLedger` (`:102-111`) already lets a learned denial
  win over every static verdict, so the mechanism exists; only the
  recognition is missing.
- `crates/eggsec-gui/src/commands.rs:514-567` — `submit_task` keeps
  checking exposure and capability before submitting. No change to the
  submission path is required: the four interface-bound kinds' forms are
  target-free already, because the form is generated from the published
  request schema.
- `crates/eggsec-gui/src/connection.rs:180-188` — the empty-list check is
  unchanged. A wider published set does not trip it; a rejecting daemon
  still does.

### Security and authorization

**C5 — the interface-bound target policy.** This is the largest change in the
milestone and the one ADR-0011's Option C requires.

- `crates/eggsec-policy/src/target.rs:173-179` — `TargetPolicyKind` gains one
  value meaning *"this operation binds to a local interface or local
  identifier, not to a network target"*. The implementer chooses the variant
  name (the plan uses `InterfaceBound`) and the `Display` label
  (`crates/eggsec-policy/src/target.rs:181-191`).
- The new value's contract, which the implementer MUST satisfy and MUST
  state in the closure:
  1. It accepts a `None` target. The operation takes no network target.
  2. It accepts an optional **local identifier** — an interface name, a BSSID,
     a checkpoint path — as the descriptor's target, and that identifier is
     **carried into the descriptor and the audit record** so it is never
     silently dropped. The CLI already passes the interface name as the
     target for the passive wireless scan
     (`crates/eggsec/src/commands/handlers/wireless.rs:30-31`) and the BSSID
     for the deauth (`:68-70`), so this is the shape the existing code
     already uses.
  3. `requires_explicit_scope` **stays true**, exactly as for
     `ExplicitScopeRequired` and `PrivateOrLocalRequired`
     (`crates/eggsec-policy/src/catalog.rs:162-166`, `:201-205`). The
     operator's explicit-scope requirement is **not** relaxed.
  4. `requires_private_or_local_target` stays **false**: the local binding is
     not a network target to be matched against a private-range rule.
  5. The local identifier is **not matched against the scope's target rules**,
     because it is not a remote target. This is the ADR's intent ("the scope
     check applies to the local host rather than to a remote target").
- **The residual gap this milestone does not close, stated plainly.** The
  scope model has **no** first-class local-host subject: `Scope`
  (`crates/eggsec-policy/src/scope.rs:88-109`) carries
  `allowed_targets`/`excluded_targets` rules, ports, a rate limit and
  `require_explicit_scope`, and nothing that names "the machine this runs
  on". So the new value's scope semantics are: *the operator's explicit scope
  is still required, and the local binding is not a remote target and is not
  matched against the remote target rules.* Introducing a local-host subject
  into the scope model would touch every surface's target validation and is
  **out of scope** for this milestone. The implementer MUST record this as a
  named residual gap in the closure record, and MUST NOT describe the change
  as "the scope check now applies to the local host" — it does not, because
  the model has no way to say that.
- **The catalog split this requires.** The catalog is **operation**-granular
  while the wire kinds are finer, so an interface-bound wire kind cannot own
  a shared operation without relaxing it (ADR-0011 Option B's failure mode:
  `packet` would become runnable with no target, and `packet-send` — a
  `RawPacket` operation — would write raw frames to an unspecified
  destination). The implementer therefore splits the catalog so each
  interface-bound wire kind owns its operation:

  | Wire kind | Operation today | After the split |
  |---|---|---|
  | `packet-capture` | `packet` (shared with `traceroute` and `packet-send`) | a new `packet-capture` operation, interface-bound; `packet` keeps `traceroute` and `packet-send` and stays `ExplicitScopeRequired` |
  | `wireless` | `wireless` | `wireless` becomes interface-bound (its only wire kind is already target-less) |
  | `wireless-active` | `wireless` (see D5) | `wireless-deauth` (the catalog operation already at `crates/eggsec-policy/src/catalog.rs:620-635`), interface-bound |
  | `resume` | `pipeline` (shared with `pipeline`) | **no split.** `resume`'s target lives in the checkpoint, not in the params (`crates/eggsec-runtime/src/request.rs:679-683`), so the runtime bridge MUST resolve the checkpoint's target and build the descriptor against `pipeline` with that target. `pipeline` stays `ExplicitScopeRequired` and the resumed target is scope-checked like any other |

  The `resume` resolution is the strongest available story: it keeps the
  existing target policy and the existing scope check, and it refuses a
  resume whose checkpoint carries no readable target. If the checkpoint's
  target **cannot** be resolved safely, the implementer MUST NOT fall back to
  a target-less `resume`; §14 requires them to stop and report.
- The split changes the counted sets: the distinct operation ids go from 26
  to **28** (`wireless-active` → `wireless-deauth`, which is a new distinct id
  while `wireless` stays with the passive scan; and `packet-capture` → a new
  `packet-capture`, while `packet` keeps `traceroute` and `packet-send`;
  `resume` keeps `pipeline`, so it adds none). The catalog goes from 34
  operations to **35** (one new operation, `packet-capture`; `wireless-deauth`
  already exists; `pipeline` is reused). The implementer MUST re-derive
  these numbers from source and record them with the command that produced
  them; the numbers above are the plan's expectation and are **not** a
  substitute for the measurement.
- `crates/eggsec-policy/src/catalog.rs:1418` — `TARGET_LESS_OPERATIONS` gains
  the interface-bound operations, so the catalog's own consistency checks
  (`:1038-1046`, `:1400-1411`) stay true.
- `crates/eggsec/src/runtime_bridge/descriptor.rs:15-31` — the bridge keeps
  resolving the operation id and target from the canonical tables
  (`:40-47`) and keeps failing explicitly on a target-policy violation. The
  implementer adds the `resume` checkpoint resolution here or in the
  canonical-execution path, and MUST NOT add a second operation-identity
  table.

**C7 — the `wireless-active` operation identity.**

- `crates/eggsec-runtime/src/request.rs:653` — `TaskKind::WirelessActive`
  maps to operation id `"wireless-deauth"`, with the explanatory comment the
  `Resume` mapping already carries (`:630-635`).
- `crates/eggsec/src/dispatch/canonical_execution.rs:319` —
  `CanonicalOperationRequest::WirelessActive` maps to `"wireless-deauth"`, and
  `Self::Wireless(_)` keeps `"wireless"`.
- `crates/eggsec/src/dispatch/canonical_execution.rs:590-605` —
  `executor_route_for` gains a `"wireless-deauth" => "security"` arm. Without
  it the corrected identity routes to `"unknown"`.
- The engine's canonical dispatch is by **variant**
  (`crates/eggsec/src/dispatch/canonical_execution.rs:1801-1830`), so the
  dispatch arms do not move; what moves is the **identity the enforcement
  layer reads**. The implementer MUST prove, with a test, that a
  daemon-submitted `wireless-active` is now evaluated against the
  `wireless-deauth` metadata — risk `Intrusive`, capability
  `RawPacketProbe`, feature `wireless-advanced` — and not against the passive
  `wireless` metadata.
- `wireless-advanced` is in the `service` profile
  (`crates/eggsec/src/config/daemon_profile.rs:91`), so a `service`-profile
  daemon has the feature; a daemon without it denies the kind with
  `FeatureMissing`, which is the honest outcome under invariant 10.

### Documentation and static guards

**C8 — four new guards, numbered 169 and above.** The numbers are **proposed**
until merged; the highest existing check is 168
(`scripts/check-architecture-guards.sh:5450`).

| Proposed | What it pins |
|---|---|
| 169 | The capability list is **derived**. `daemon_conservative()` and `full_lab()` MUST NOT exist; the private `task_kind_name()` MUST NOT exist; `TaskKind::capability_name()` MUST be the only name table; the derived list MUST cover every canonical capability name, `resume` included. A new `TaskKind` variant MUST make the check fail. |
| 170 | Publication and admission are the **same object**. The daemon's published task-kind names MUST equal the runtime's enforced task-kind names, for every host construction path; and a rejecting executor MUST declare the empty set explicitly. |
| 171 | The `wireless-active` wire kind resolves to the `wireless-deauth` operation, and `wireless-deauth` is reachable from the runtime's canonical dispatch and from `executor_route_for`. |
| 172 | An interface-bound operation carries the interface-bound target policy, and **no** operation's target policy was relaxed to `OptionalTarget` to accommodate a target-less wire kind. The check MUST name the specific shared operations whose policy MUST NOT change (`packet`, `pipeline`). |

Each guard MUST be negative-tested: the closure record shows it failing under
a deliberate violation and passing after restore.

## 7. Ordered work packages

The order below is chosen so that **each package leaves the tree green** and
so that **nothing is published before it is coherent**: the published set
widens only in work package F, after the wireless identity fix (D5) and the
interface-bound target story (D4) have landed.

### Work package A — The derived capability list

Intent:

Give the workspace exactly one capability list, derived from the canonical
`TaskKind` set, covering every canonical name including `resume`.

Required changes:

1. `crates/eggsec-runtime/src/request.rs` — add the exhaustive variant
   enumeration (§6, C1) and `Default` to the twelve params structs that lack
   it. The enumeration's construction is exhaustive over `TaskKind`, so adding
   a variant is a compile error until it is covered.
2. `crates/eggsec-runtime/src/capabilities.rs` — add the derived constructor;
   delete `task_kind_name()` (`:142-174`) and delegate `supports_task_kind`
   (`:135-138`) to `TaskKind::capability_name()`; retire `daemon_conservative()`
   (`:45-74`) and `full_lab()` (`:77-115`); repoint `RuntimeCapabilities::default()`
   (`:34-38`) at the derived list.
3. `crates/eggsec-runtime/src/runtime_config.rs:32` — `RuntimeConfig::default()`
   carries the derived list.
4. Update every call site of the two retired constructors. Measured at the
   baseline, the call sites are in `crates/eggsec-runtime/src/capabilities.rs`
   (the tests at `:219-309`), `crates/eggsec-runtime/src/runtime.rs` (the trait
   default at `:68-70` and the test at `:1804-1864`),
   `crates/eggsec-runtime/tests/in_process_client.rs:731`,
   `crates/eggsec-daemon/src/host.rs:105` (the publication helper) and
   `crates/eggsec-daemon/src/host.rs:3951,3978` (the tests). The implementer
   re-measures before editing.
5. Rewrite — **not delete** — `conservative_capabilities_reject_hazardous_task_kinds`
   (`crates/eggsec-runtime/src/runtime.rs:1804-1864`) against the derived list
   and an explicitly narrow set (§6, C4).

Acceptance evidence:

- A test asserting the derived list contains **every** canonical capability
  name, `resume` included, and that its length equals the number of `TaskKind`
  variants.
- A test asserting that a `TaskKind` variant added without a derived entry
  fails to compile (the property is the compile error; the guard in work
  package G is the machine-checked backstop).
- The rewritten narrow-set test proving that a runtime configured with a
  narrower set still rejects the kinds outside it.

Invariant protected: §4.1, §4.2, and the repository's single-table rule.

### Work package B — The executor declaration

Intent:

Make the engine's executor declare the derived list, so that widening the
publication is not inert (defect D2).

Required changes:

1. `crates/eggsec/src/runtime_bridge/executor.rs:141` — add an explicit
   `supported_task_kinds()` override returning the derived list, next to the
   existing `enabled_features()` (`:147`) and `report_for()` (`:159`)
   overrides.
2. `crates/eggsec-runtime/src/runtime.rs:68-70` — the trait default becomes
   the derived list, and its doc comment keeps the rule that a rejecting
   executor MUST declare the empty set explicitly.
3. Give every rejecting executor an explicit empty declaration:
   `NoopExecutorStub` (`crates/eggsec-daemon/src/host.rs:4019-4027`, already
   empty) and the frontend client's `NoopExecutor`
   (`crates/eggsec-frontend-client/src/lib.rs:141-156`, **not** empty today).

Acceptance evidence:

- A test asserting the engine's executor declares every canonical capability
  name.
- A test asserting a rejecting executor publishes and admits nothing.

Invariant protected: §4.1, §4.2, and guard 165's property.

### Work package C — The runtime's admission set and session hydration

Intent:

Make a recovered session enforce exactly what a fresh session enforces
(defect D3), and give the runtime an accessor for its enforced set so the
"publication == admission" property is assertable rather than assumed.

Required changes:

1. `crates/eggsec-runtime/src/session.rs:311-327` —
   `hydrate_from_snapshot` stops substituting `RuntimeCapabilities::default()`.
2. `crates/eggsec-runtime/src/runtime.rs:759` — the hydration site passes the
   runtime's configured capabilities.
3. `crates/eggsec-runtime/src/runtime.rs:145-189` — add the additive
   accessor for the runtime's **enforced** set.

Acceptance evidence:

- A test proving a recovered session admits exactly what a fresh session
  admits, for the same configured set.
- A test proving the snapshot's own persisted `capabilities` field is not
  authoritative: a snapshot carrying a stale set does not narrow the
  recovered session.

Invariant protected: §4.1, §4.2.

### Work package D — The `wireless-active` operation identity

Intent:

Stop the daemon from authorising a deauth under the passive operation's risk
level (defect D5), **before** `wireless-active` is published.

Required changes:

1. `crates/eggsec-runtime/src/request.rs:653` — `TaskKind::WirelessActive` →
   `"wireless-deauth"`, with an explanatory comment in the style of `:630-635`.
2. `crates/eggsec/src/dispatch/canonical_execution.rs:319` — the engine's
   internal table moves with it.
3. `crates/eggsec/src/dispatch/canonical_execution.rs:590-605` —
   `executor_route_for` gains the `"wireless-deauth"` arm.
4. The catalog's `wireless-deauth` operation
   (`crates/eggsec-policy/src/catalog.rs:620-635`) is the operation the kind
   now names. Its `target_policy` is addressed by work package E.

Acceptance evidence:

- A test proving the `wireless-active` wire kind resolves to the
  `wireless-deauth` operation, in **both** the wire-side table and the
  engine's internal table.
- A test proving the `wireless-deauth` catalog operation is reachable from
  the runtime, and that `executor_route_for("wireless-deauth")` is not
  `"unknown"`.
- A test proving the two paths authorise the same deauth work at the same
  risk level: the CLI's deauth and a daemon-submitted `wireless-active`
  resolve to the same metadata.

Invariant protected: §4.8, §4.10, §4.11.

### Work package E — The interface-bound target story

Intent:

Give the four interface-bound kinds a coherent target policy, without
relaxing any shared operation (defect D4, ADR-0011 Option C).

Required changes:

1. `crates/eggsec-policy/src/target.rs:173-179` — the new lattice value, its
   `Display` label (`:181-191`), and its behaviour in
   `try_descriptor_for_target` (`crates/eggsec-policy/src/catalog.rs:106-210`)
   per §6, C5's five contract points.
2. The catalog split: a new `packet-capture` operation (interface-bound),
   `wireless` and `wireless-deauth` become interface-bound, and `resume`
   keeps `pipeline` with the checkpoint's target resolved by the runtime
   bridge (§6, C5's table).
3. `crates/eggsec-policy/src/catalog.rs:1418` — `TARGET_LESS_OPERATIONS`
   updated.
4. `crates/eggsec/src/runtime_bridge/descriptor.rs:15-47` — the `resume`
   checkpoint-target resolution, with an explicit failure when the
   checkpoint's target cannot be read.
5. Every other surface's target validation that consumes
   `TargetPolicyKind` is updated: `crates/eggsec/src/domain/mod.rs:402-406`
   and `crates/eggsec/src/runtime_bridge/descriptor.rs:442,460,483`.

Acceptance evidence:

- A test proving each of the four interface-bound kinds builds a descriptor
  successfully and carries its local identifier.
- A test proving the shared operations' target policies are **unchanged**:
  `packet` and `pipeline` remain `ExplicitScopeRequired`, and `packet-send`
  still refuses to build a descriptor with no target.
- A test proving an interface-bound operation still requires an explicit
  scope (`requires_explicit_scope == true`), so the operator's consent gate
  is not weakened.
- A test proving a `resume` whose checkpoint carries no readable target fails
  **explicitly**, not with a policy denial and not with a silent target-less
  descriptor.

Invariant protected: §4.8, §4.11, and ADR-0011 Option B's failure mode
(a `RawPacket` operation with no destination).

### Work package F — Publication and admission agree by construction

Intent:

Make the daemon's published set and its admitted set the same object, and
widen both to the derived list.

Required changes:

1. `crates/eggsec-daemon/src/host.rs:104-110` — `published_task_kinds()`
   intersects the **derived** list with the executor's declaration. The
   policy constant lives here, **outside** the `Capabilities` arm.
2. `crates/eggsec-daemon/src/host.rs:113-131` — `DaemonHost::new` builds the
   runtime with the published set as the runtime's enforced set.
3. `crates/eggsec-daemon/src/host.rs:134-161` — `new_noop` unchanged.
4. `crates/eggsec-daemon/src/host.rs:164-180` — `with_runtime` refuses to
   construct when the runtime's enforced set is not the published set.
5. The daemon's own tests (`crates/eggsec-daemon/src/host.rs:3936-3983`) are
   rewritten against the new property: the published set is the derived list
   intersected with the declaration; a rejecting executor publishes nothing;
   a declared kind outside the derived list is not published.

Acceptance evidence:

- A test proving the daemon's published set and its admitted set are the same
  object, for every host construction path.
- A test proving a no-op daemon publishes and admits nothing.
- A test proving a kind that was previously unadvertised — `resume` — is now
  published, admitted, and reaches the executor.

Invariant protected: §4.1, §4.6, and guard 165.

### Work package G — The GUI's honest reporting, the guards, and the docs

Intent:

Make the GUI report the widened set truthfully, pin the new properties with
guards, and correct every document and skill that states the old counts.

Required changes:

1. `crates/eggsec-gui/src/coverage.rs:150-224` — the interface-bound fact in
   the reason string (or, as a recorded deviation, a seventh verdict).
2. `crates/eggsec-gui/src/availability.rs:23-34,68` — the policy-denial
   recognition.
3. Guards **169**, **170**, **171** and **172** (§6, C8), added to
   `scripts/check-architecture-guards.sh` with the next free numbers, and
   `docs/CI_ARCHITECTURE_GUARDS.md` updated with their section.
4. The documentation in §12.
5. The closure-evidence pack in §15.

Acceptance evidence:

- A test proving a published-but-policy-denied kind is reported as denied,
  never as available.
- A test proving an interface-bound kind is reported with a reason naming the
  local binding.
- Each new guard demonstrated to fail under a deliberate violation and to
  pass after restore.

Invariant protected: §4.13, and the "a published capability the client cannot
run is not published as available" property.

## 8. Failure, cancellation, restart, and contention semantics

- **Partial failure.** A daemon that fails to build its published set MUST
  fail closed: it publishes nothing rather than publishing a partial list. A
  `with_runtime` host whose runtime enforces a different set MUST refuse to
  construct (§6, C4) rather than publish a list it will not admit.
- **Duplicate delivery.** A repeated `Capabilities` request MUST return the
  same derived list. The list is computed once at construction and stored
  (`crates/eggsec-daemon/src/host.rs:118-130`), so a repeat is a read.
- **Process restart.** A restarted daemon recomputes the derived list from the
  same canonical source, so the published set is stable across restarts. A
  **recovered** session now enforces the runtime's configured set, not the
  conservative default (work package C). The recovered session's *scope* and
  *generation* handling is unchanged.
- **Stale generations.** A recovered session carries its snapshot's
  generation (`crates/eggsec-runtime/src/session.rs:318`). This milestone
  changes the recovered session's **capabilities**, not its generation, and
  MUST NOT weaken the generation check.
- **Cancellation races.** Unchanged. This milestone adds no task, no
  background work and no awaited future on the submission path. No new
  spawned tokio task is introduced, so no new timeout wrapper is required;
  if the implementer introduces one, it MUST be bounded to 30–300 s per the
  workspace rule.
- **Concurrent callers.** The published list is immutable after construction
  and read under the existing locks. The runtime's new enforced-set accessor
  takes the runtime's state mutex; the implementer MUST NOT hold it across an
  await, and MUST NOT introduce a lock-order inversion against the executor
  handle the runtime keeps outside the mutex
  (`crates/eggsec-runtime/src/runtime.rs:137-142`).
- **Admission versus publication.** A submission that arrives while the
  daemon is starting MUST be refused with `UnsupportedTaskKind` rather than
  admitted against a half-built list. The list is built before the runtime is
  constructed, so there is no window.

## 9. Compatibility and migration

- **Wire.** The `Capabilities` handshake's `runtime.task_kinds` list widens.
  The field is additive and already tolerant on the consumer side, so an old
  client reading a new daemon receives a longer list rather than a parse
  failure. A new client reading an old daemon receives the old narrow list and
  MUST continue to degrade honestly — the GUI's coverage model already does
  this, reporting the difference rather than hiding it.
- **The published operation catalog grows** by one operation (`packet-capture`)
  and one target-policy label (the interface-bound label). Both are additive:
  a client that switches on `target_policy` must handle the new label, which
  is why the GUI change is in work package G.
- **No storage migration.** No persisted state changes shape. The session
  snapshot's `capabilities` field is not authoritative — hydration already
  ignores it — and the corrected hydration makes that explicit.
- **No protocol version bump.** The change adds list entries; it does not
  alter a message shape. The desktop GUI hard-refuses to connect on a protocol
  mismatch, so the version stays at `DAEMON_PROTOCOL_VERSION = 2`.
- **Operator configuration.** Nothing is required of an operator to keep
  today's behaviour: the risk-policy defaults are unchanged, so the hazard
  families remain refused unless an operator opts in. **What changes for an
  operator** is that those families are now *visible* to a manual client, and
  that a client can attempt them and receive a policy denial rather than
  `UnsupportedTaskKind`.
- **The CLI and the TUI.** Behaviourally unchanged except for the recorded
  consequence of the target-policy change: the CLI's passive wireless scan and
  deauth pass a **local identifier** (an interface name, a BSSID) as the
  target, and that identifier is no longer matched against the scope's remote
  target rules (§6, C5, contract point 5). The explicit-scope requirement is
  unchanged. This is a **recorded behavioural change** and MUST appear in the
  closure record, because it is the one place where this milestone relaxes a
  scope check.
- **Removal criteria.** `daemon_conservative()` and `full_lab()` are removed
  in this milestone, not deprecated. `RuntimeCapabilities::full()`
  (`crates/eggsec-runtime/src/capabilities.rs:118-121`) is already
  `#[deprecated]` and points at `full_lab()`; it MUST be removed with it.

## 10. Required tests

### Focused unit tests

- The derived list contains every canonical capability name, `resume`
  included, and its length equals the variant count.
- The description map covers exactly the derived name set.
- `supports_task_kind` agrees with `TaskKind::capability_name()` for every
  variant.
- The trait default is the derived list; a rejecting executor's explicit empty
  declaration wins.
- `RuntimeCapabilities::noop()` is empty and the no-op host publishes nothing.
- `TargetPolicyKind`'s new value: accepts `None`; accepts a local identifier;
  keeps `requires_explicit_scope`; keeps `requires_private_or_local_target`
  false; is not matched against the scope's target rules.
- The `resume` checkpoint resolution: a readable checkpoint yields the
  checkpoint's target; an unreadable or target-less checkpoint fails
  explicitly.

### Integration tests

- The daemon's published set equals the runtime's enforced set, for
  `DaemonHost::new`, `DaemonHost::new_noop` and `DaemonHost::with_runtime`.
- A daemon submitting `resume`, `stress-test`, `wireless-active`,
  `db-pentest`, `intercept` and `c2` is **admitted** (no
  `UnsupportedTaskKind`) and then **refused by the policy** with the
  risk-policy denial — not with `UnsupportedTaskKind`. This is ADR-0011's
  verification requirement, and it is the test that distinguishes this
  milestone from a capability widening that would weaken enforcement.
- The four interface-bound kinds reach the executor.
- A `wireless-active` submission is evaluated against the `wireless-deauth`
  metadata.
- The GUI's coverage table against a widened daemon: the previously
  `NotAdvertised` kinds resolve to `Available` or to an honest reason; the
  policy-denied kinds resolve to `Blocked` with the policy reason; the
  interface-bound kinds carry the local-binding reason.

### Restart and recovery tests

- A daemon that restarts and recovers a session enforces the same set as a
  fresh session.
- A snapshot carrying a stale `capabilities` field does not narrow the
  recovered session.

### Contention and cancellation tests

- Concurrent `Capabilities` requests return the same list.
- The runtime's new enforced-set accessor does not deadlock against the
  executor handle, under concurrent submission.

### Security and negative tests

- A no-op daemon publishes and admits nothing.
- A strict surface (REST/MCP/gRPC/agent) of the same daemon is still refused
  a hazardous operation.
- The operator's default policy still refuses every hazardous family, with
  the risk-policy denial.
- The shared operations' target policies are unchanged: `packet-send` with no
  target still fails to build a descriptor; `packet` and `pipeline` are still
  `ExplicitScopeRequired`.
- An interface-bound operation with no explicit scope is still refused.
- Each new guard fails under a deliberate violation.

### Migration and compatibility tests

- An old client decoding a new daemon's `Capabilities` succeeds and sees a
  longer list.
- A new client decoding an old daemon's `Capabilities` succeeds and degrades
  to the honest "not advertised" verdict.
- A client that sees the new `target_policy` label degrades to a stated
  reason rather than to "unknown".

## 11. Required verification commands

```bash
make check                  # mandatory Rust contract
make check-features-individual  # deep check only: the feature surface changes
```

The implementer MUST also run, because this milestone touches the runtime,
the daemon host, the policy crate and the GUI crate:

```bash
cargo test -p eggsec-runtime
cargo test -p eggsec-daemon
cargo test -p eggsec-policy
cargo test -p eggsec-gui
cargo test -p eggsec --features cli --lib runtime_bridge
bash scripts/check-architecture-guards.sh
python3 scripts/check_doc_references.py
```

`make check-python` is **not** required unless a Python binding, stub or doc
changes; this milestone changes none, and the Python bindings' surface is
unaffected.

The guard script requires `ripgrep` (`rg`). The implementer MUST NOT claim a
command that was not run.

## 12. Documentation updates

- `docs/DAEMON.md:22-35` — the "Capabilities" section: the published set is
  now the derived canonical list intersected with the executor's declaration;
  the "conservative policy" wording goes.
- `docs/DAEMON.md:91-100` — the honest-limitation section: the "8 of the 20"
  sentence is restated against the CLI-hosted daemon's build, and the widened
  published set is distinguished from what that build can execute.
- `architecture/daemon.md:459` — the capability-honesty test list.
- `architecture/daemon.md:476` — invariant 10, "the published capability list
  is derived, not declared".
- `architecture/runtime.md:213-217` — the `RuntimeCapabilities` table: the
  conservative and full-lab rows are replaced by the derived list, the no-op
  row stays.
- `architecture/overview.md` — it owns the headline counts; the operation-id
  multiplicity and the catalog size are updated here, with the new guard
  references. **Do not restate its counts elsewhere.**
- `.opencode/skills/eggsec-daemon/SKILL.md:61-69` — "Capability honesty": the
  conservative-policy wording becomes the derived-list wording.
- `.opencode/skills/eggsec-gui/SKILL.md:22` — the guard range becomes
  155–172 (or whatever the implementer's numbers are), and the six-verdict
  claim is updated if the verdict count changes.
- `docs/CI_ARCHITECTURE_GUARDS.md` — a new section for guards 169–172.
- `plans/implementation/desktop-gui-frontend/006-full-operation-coverage-and-report-export.md`
  — **not** by this milestone's implementer: the "Amendment record" SHOULD be
  extended with a pointer to ADR-0011 by whoever owns the plan set. This
  milestone records the supersession in its own §0.
- `plans/implementation/desktop-gui-frontend/009-daemon-obtainability-and-start-honesty.md`
  — same: not edited by this milestone's implementer; the supersession is
  recorded in this plan's §0.
- `plans/registry.md` — **not** by this milestone's implementer either. The
  registry entry for this plan is the maintainer's step (see §16).

## 13. Acceptance criteria

Externally observable or contract-level statements. **Delivered by this
milestone:**

1. The daemon publishes and admits **every canonical capability name the
   engine can execute**, and the two sets are the same object: a client can
   read the published list, submit each name, and observe either execution or
   a *policy* denial — never `UnsupportedTaskKind` for a name the daemon
   published.
2. `resume` is published, admitted, and reaches the engine's resume path.
3. There is exactly **one** capability-name list in the workspace, derived from
   the canonical `TaskKind` set; adding a `TaskKind` variant without adding
   it to the derived list is a compile error, and guard 169 fails if a
   hand-written list reappears.
4. A rejecting daemon still publishes and admits nothing.
5. A recovered session enforces exactly what a fresh session enforces.
6. The four interface-bound kinds are runnable through the daemon — their
   descriptor builds, their local identifier is carried into the descriptor
   and the audit record — **or** they are reported with an honest reason. They
   are never reported as available and then refused for want of a target.
7. The shared operations' target policies are unchanged: `packet` and
   `pipeline` are still `ExplicitScopeRequired`, and `packet-send` still
   refuses to build a descriptor with no target.
8. A `wireless-active` submission is evaluated against the `wireless-deauth`
   metadata — risk `Intrusive`, capability `RawPacketProbe`, feature
   `wireless-advanced` — and the CLI's deauth and the daemon's
   `wireless-active` resolve to the same operation identity.
9. The operator's default `[execution_policy]` still refuses every hazardous
   family, with the risk-policy denial, and the refusal is **not** a
   confirmation-class denial that a flag could downgrade.
10. A strict surface of the same daemon is still refused a hazardous
    operation.
11. The GUI reports a published-but-policy-denied kind **as denied**, never as
    available, and reports an interface-bound kind with a reason naming the
    local binding.
12. The GUI adds no per-operation code, names no task-kind variant, and adds
    no IPC command.
13. `DAEMON_PROTOCOL_VERSION` is 2; no message shape changed; no persisted
    representation changed; no `TaskOutcome` variant was added; the pinned
    empty-payload envelope contract is green and unmodified.
14. Guards 169–172 exist, pass, and each was demonstrated to fail under a
    deliberate violation.
15. Every document and skill in §12 is updated, and
    `python3 scripts/check_doc_references.py` reports no failure on any line
    this milestone changed.

**Explicitly NOT delivered by this milestone — milestone 011's:**

16. A manual daemon client **cannot** answer a confirmation:
    `ClientCommand::ApprovePolicy` still answers `ErrorCode::Unsupported`
    (`crates/eggsec-daemon/src/host.rs:928-932`), and
    `RuntimeEvent::PolicyDecisionRequired` is still raised by nothing. The
    hazard families remain **offered and refused**, not **permitted**.
17. The socket path and bind contract, the report bridge's coverage, the
    platform qualification evidence, and the dependency advisory backlog.

## 14. Stop conditions

Stop and escalate rather than improvise if any of these arise:

- **ADR-0011 is not accepted, or is amended before acceptance.** The roadmap
  makes its acceptance a hard dependency of this milestone. Escalate; do not
  implement a reading of the ADR that the maintainers have not ratified.
- **The four interface-bound kinds cannot be given a coherent target story
  without weakening a shared operation's target policy.** This is ADR-0011
  Option C's failure mode, and it is the case the governance requires an
  agent to stop for. Specifically: if splitting the catalog so that
  `packet-capture` owns its own operation would require relaxing `packet` to
  `OptionalTarget` — making `packet-send` runnable with no destination — STOP
  and report. Do not relax the shared operation.
- **`resume`'s checkpoint target cannot be resolved safely.** If the runtime
  bridge cannot read the checkpoint's target and bind it to the resumed
  operation, STOP. Do not fall back to a target-less `resume`, and do not
  relax `pipeline`'s target policy to accommodate it.
- **The new lattice value cannot be given a scope story that keeps
  `requires_explicit_scope` true.** If honouring the local binding requires
  dropping the explicit-scope requirement, STOP. That is the operator's
  consent gate, and ADR-0011's Decision section leaves enforcement untouched.
- **A first-class local-host subject in the scope model turns out to be
  required** to make the new lattice value coherent
  (`crates/eggsec-policy/src/scope.rs:88-109` has no such subject today).
  That is a scope-model change touching every surface's target validation.
  Escalate; do not widen the milestone into the scope model.
- **The derived list cannot be made exhaustive by construction.** If the
  implementer finds themselves maintaining a name list by hand to satisfy the
  derivation, STOP. A hand-written list is the defect this milestone exists
  to remove, and re-introducing one would make the milestone worse than doing
  nothing.
- **`DAEMON_PROTOCOL_VERSION` would have to change.** A message shape must
  not change. Escalate.
- **A `TaskOutcome` variant would have to be added.** An externally tagged
  serde enum is not wire-additive. Escalate.
- **The published capability list would have to differ between a manual
  surface and an automated surface of the same daemon.** The set is
  surface-agnostic (ADR-0011, Decision 9). Escalate.
- **Widening the set appears to require changing an `ExecutionPolicy`
  default.** The risk flags are the operator's consent gate, not the
  capability set's business. Escalate.
- **The strict surfaces appear to need a change.** They fail closed on
  policy, not on the capability set. Escalate.
- **The dependency advisory backlog or the platform qualification evidence
  blocks the closure.** Both are separate milestones. Escalate; do not widen
  this milestone to fix them, and do not add a `deny.toml` exception on the
  implementer's own authority.

## 15. Closure evidence required

The closure record
(`plans/closure/desktop-gui-frontend/010-daemon-full-manual-capability-policy-closure.md`)
MUST contain:

1. The **measured** canonical sets after the change: the number of `TaskKind`
   variants, the number of distinct capability names, the number of distinct
   operation ids, the catalog's operation count, and the number of distinct
   target-policy labels — with the command that produced each. The plan's
   expectations (28 operation ids, 35 catalog operations) are recorded as
   *expectations*, and any divergence is recorded with its reason.
2. The **published-versus-admitted evidence**: for every host construction
   path, the published list and the enforced list, shown to be the same
   object, and the no-op host shown to publish and admit nothing.
3. The **derivation evidence**: the proof that the list is derived — the
   compile error on an unlisted variant, and guard 169's negative test.
4. The **interface-bound evidence**: the four kinds' descriptors, the local
   identifier each carries, the explicit-scope requirement still enforced,
   the shared operations' policies proven unchanged, and the `resume`
   checkpoint resolution — with the failure mode for an unreadable checkpoint.
5. **The residual gap, stated in full**: the scope model has no local-host
   subject, so the new lattice value's scope semantics are "explicit scope
   required; the local identifier is not matched against the scope's remote
   target rules", and the CLI's wireless paths no longer scope-check the
   interface name or the BSSID as a host. This is the milestone's only
   recorded scope relaxation, and the closure record MUST say so plainly.
6. **The wireless identity evidence**: the wire-side and engine-side tables,
   `executor_route_for`, and the proof that the CLI's deauth and the daemon's
   `wireless-active` authorise the same work at the same risk level.
7. **The enforcement-unchanged evidence**: a `stress-test` submitted to a
   daemon with the default execution policy is still refused, with the
   risk-policy denial and **not** with `UnsupportedTaskKind`; a strict surface
   of the same daemon is still refused; and the operator's risk-flag defaults
   are unchanged.
8. **The guard evidence**: guards 169–172, each with a demonstration of
   failing under a deliberate violation and passing after restore.
9. **The GUI evidence**: the coverage verdicts against a widened daemon, the
   policy-denial recognition, the interface-bound reason, and the statement
   that the GUI's IPC command count is unchanged.
10. **The verification commands actually run**, with their outcomes.
    Commands that were not run MUST NOT be claimed.
11. **Every deviation** from this plan's wording, with its reason — including
    any change to the operation-id count, the catalog size, the target-policy
    label set, the derived description source, and the GUI's verdict count.

## 16. Handoff notes

- **Do not start until ADR-0011 is `Status: accepted`.** The roadmap makes it
  a hard dependency, and §14 makes it a stop condition. This plan implements
  ADR-0011's Decision section verbatim; it does not interpret or extend it.
- **Register the plan in `plans/registry.md`** — the README's handoff rules
  require it, and this milestone's implementer does **not** own the registry.
  The same applies to the ADR's status line and the roadmap's milestone status:
  the implementer does not own either.
- **Known hazards.**
  - The trait default change (work package B) makes "says nothing" mean
    "declares everything". Every rejecting executor must gain an explicit
    empty declaration, and guard 169/170 pins it. The frontend client's test
    `NoopExecutor` is the one that does not declare empty today — it is
    easy to miss.
  - The catalog split (work package E) changes the operation-id multiplicity
    and the catalog size. `architecture/overview.md` owns those counts; the
    GUI skill and the daemon skill must be updated alongside.
  - The interface-bound lattice value is the **only** scope relaxation in this
    milestone, and it is invisible in the diff of a policy-crate enum. Read
    §6, C5, contract point 5 and §9 again before changing it.
  - Work packages D and E must land **before** F. Publishing `wireless-active`
    before the identity fix ships a live authorisation divergence; publishing
    an interface-bound kind before its target story ships a button that
    fails.
- **Resource constraints.** No new dependency is added, so no `deny.toml` or
  `docs/DEPENDENCY_EXCEPTIONS.md` change is expected. If one becomes
  necessary, that is a maintainer decision, not the implementer's.
- **Environment requirements.** `make check` needs the workspace toolchain
  (MSRV 1.89) and the guard script needs `ripgrep`. `make check` must not be
  run as root.
- **Preserved user changes.** The implementer MUST NOT revert unrelated
  working-tree changes, and MUST NOT touch any file outside the crates and
  documents this milestone names.
- **The implementer may adjust file-level mechanics** — names, module layout,
  test placement — as long as the properties in §13 hold and the deviations
  are recorded in the closure record.
