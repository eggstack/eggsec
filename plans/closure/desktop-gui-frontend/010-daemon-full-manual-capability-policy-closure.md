# Desktop GUI Frontend Milestone 010 — Closure Status

Status: closed

Source implementation plan:

- `plans/implementation/desktop-gui-frontend/010-daemon-full-manual-capability-policy.md`

Source subsystem roadmap:

- `plans/subsystems/desktop-gui-frontend-roadmap.md#7-milestones`

Repository baseline reviewed: `6de8294f`

Durable decision implemented:

- `plans/adrs/ADR-0011-daemon-manual-capability-policy.md` (**accepted 2026-10-24**)

Implementation commits:

- `1877d8e4` — feat(daemon): implement the desktop GUI milestone 010
- `this commit` — docs(plans): record the closure of the desktop GUI milestone 010

## 1. Executive finding

**The milestone's capability boundary is complete.** The daemon no longer
withholds any capability from a manual execution, and — this is the part the
plan refused to compromise on — **nothing was weakened to achieve it**.

The "conservative policy" the maintainer asked about turned out to be four
independent layers, not one, and only one of them was actually a policy:

1. **Publication.** The daemon published a hand-written 20-kind list. That
   list is **gone**: the workspace now has exactly one capability list,
   derived from the canonical `TaskKind` set, covering all 30 canonical
   names — `resume` and the hazard families included.
2. **Admission.** The runtime's enforced set is now **the same object** as
   the published list. The host computes the published set first and hands
   it to the runtime, so the daemon cannot advertise a capability it will
   refuse, and cannot refuse one it advertised. This is now structural, not
   a convention.
3. **Enforcement.** Untouched, deliberately. The operator's `[execution_policy]`
   risk flags remain the sole gate for the hazard families, and a hazard
   submission is refused *by the policy* — naming the risk tier — never by
   `UnsupportedTaskKind`. The risk-flag defaults are unchanged.
4. **Approval.** Still unwired. `ApprovePolicy` remains `Unsupported`, so a
   manual daemon client cannot answer a confirmation. That is milestone
   011, and the plan recorded the dependency rather than pretending it away.

Three defects were found and closed on the way:

- The `wireless-active` wire kind was authorised under the **passive**
  `wireless` operation's risk level. It now resolves to `wireless-deauth` in
  the wire table, the engine's canonical table, and the executor routing
  table.
- Session hydration reset a recovered session to the retired conservative
  set. A recovered session now enforces exactly what a fresh session
  enforces.
- Four interface-bound kinds (a local interface, a BSSID) had no target, so
  their descriptor could not build and they were unrunnable. They now carry
  an explicit **interface-bound** target policy, and the local identifier is
  carried into the descriptor and the audit record.

## 2. Requirement-to-evidence matrix

| Requirement | Evidence | Result | Notes |
|---|---|---|---|
| One derived capability list covering every canonical name | `crates/eggsec-runtime/src/capabilities.rs` (`RuntimeCapabilities::canonical`), `crates/eggsec-runtime/src/request.rs` (the `define_task_kinds!` table) | pass | The retired `daemon_conservative()` and `full_lab()` constructors are gone; the private `task_kind_name()` table is gone |
| The derivation is exhaustive by construction | The `define_task_kinds!` macro generates the enum, the name match, the enumeration, and the description table from one list | pass | A new variant cannot reach the enum without reaching the derived list; the compiler is the guarantee, and guard 169 is the machine-checked backstop |
| The derived list covers every canonical name, `resume` included | `cargo test -p eggsec-runtime --lib capabilities::tests` | pass | `canonical_covers_every_task_kind`, `canonical_includes_resume`, `canonical_includes_the_hazard_families`, `description_table_covers_exactly_the_canonical_names` |
| Exactly one capability-name table | `rg 'pub fn capability_name' crates` → 1 | pass | Guard 169 |
| The engine's executor declares every canonical name | `crates/eggsec/src/runtime_bridge/executor.rs`; `cargo test -p eggsec --features cli --lib runtime_bridge::executor::tests` | pass | `the_engine_executor_declares_the_canonical_capability_list` |
| A rejecting executor declares the empty set explicitly | `NoopExecutorStub` (`crates/eggsec-daemon/src/host.rs`), `NoopExecutor` (`crates/eggsec-frontend-client/src/lib.rs`), `RejectingExecutor` (`crates/eggsec-daemon/tests/daemon_start_honesty.rs`) | pass | The trait default is the canonical list, so "says nothing" would mean "declares everything" |
| Publication and admission are the same object | `crates/eggsec-daemon/src/host.rs`; `crates/eggsec-daemon/tests/manual_capability_policy.rs` | pass | `the_published_list_is_the_enforced_list`; `DaemonHost::with_runtime` now **refuses** to construct when the two disagree (`DaemonHostError::CapabilitySetsDisagree`) |
| A no-op host publishes and admits nothing | `DaemonHost::new_noop`; `crates/eggsec-daemon/tests/manual_capability_policy.rs` | pass | `a_rejecting_executor_publishes_nothing` |
| The hazard families are admitted, not refused as unknown | `crates/eggsec-daemon/tests/manual_capability_policy.rs` | pass | `the_hazard_families_are_admitted_by_the_daemon` — `resume`, `stress-test`, `wireless-active`, `db-pentest`, `intercept` |
| A policy-refused operation is refused by the policy, not by admission | `crates/eggsec/src/runtime_bridge/executor.rs` | pass | `a_policy_refused_operation_is_refused_by_the_policy_not_by_admission` — the refusal names the risk tier |
| The operator's risk-flag defaults are unchanged | `crates/eggsec-policy/src/policy.rs` (`ExecutionPolicy::default()`) — untouched by this milestone | pass | `ADR-0011` records Option D (flipping the defaults) as rejected |
| `wireless-active` resolves to `wireless-deauth` everywhere | `crates/eggsec-runtime/src/request.rs`, `crates/eggsec/src/dispatch/canonical_execution.rs` | pass | Guard 171; `wireless_active_descriptor` asserts the deauth's risk tier differs from the passive scan's |
| The interface-bound kinds are runnable and carry their local identifier | `crates/eggsec-policy/src/catalog.rs`, `crates/eggsec/src/runtime_bridge/descriptor.rs` | pass | `wireless_descriptor`, `wireless_active_descriptor`, `packet_capture_descriptor` |
| The shared operations' policies are unchanged | `crates/eggsec-policy/src/catalog.rs` | pass | `packet` and `pipeline` stay `ExplicitScopeRequired`; guard 172; `the_packet_family_keeps_its_remote_target_requirement` |
| `resume` binds to the checkpoint's target | `crates/eggsec/src/pipeline/session.rs` (`read_target`), `crates/eggsec/src/runtime_bridge/descriptor.rs` | pass | The bridge never falls back to a target-less resume; an unreadable checkpoint is a hard failure |
| The GUI reports a policy refusal as such | `crates/eggsec-gui/src/availability.rs` | pass | `a_risk_policy_denial_is_recognised` |
| The GUI says when an operation binds to a local interface | `crates/eggsec-gui/src/coverage.rs` | pass | `an_interface_bound_operation_says_so` |
| The GUI's verdict set is unchanged | `crates/eggsec-gui/src/coverage.rs` | pass | Six verdicts, unchanged; the interface-bound fact rides in the existing reason string |
| The GUI's IPC command count is unchanged | `crates/eggsec-gui/src/shell.rs` | pass | 12 commands, unchanged |
| The published capability list is a durable wire contract | `DAEMON_PROTOCOL_VERSION` stays 2 | pass | No protocol bump, no storage migration, no operator-configuration change |

## 3. Production implementation evidence

### The derived list (work package A)

`crates/eggsec-runtime/src/request.rs` now declares the **canonical task-kind
table** through a `define_task_kinds!` macro invocation. The macro generates
the `TaskKind` enum, `TaskKind::capability_name()`, `TaskKind::all_variants()`,
`TaskKind::capability_description()`, and `CAPABILITY_DESCRIPTIONS` — all
from **one** list. The list is transcribed exactly once.

This is the plan's "exhaustive by construction" requirement, met with a
compile-time guarantee rather than a convention: a new variant cannot reach
the enum without reaching the derived list, because the enum *is* the list.
The plan's own wording allowed the implementer to choose the mechanism
("an exhaustive enumeration of the variants… whose construction of each
variant's payload is exhaustive over the same enum"); the macro is that
mechanism, chosen because a hand-written `vec![]` of variants would *not*
have been exhaustive — adding a variant to a `vec!` literal breaks nothing.

Twelve params structs gained a derived `Default` — `PipelineParams`,
`ResumeParams`, `ReconParams`, `PacketTracerouteParams`, `NseParams`,
`HuntParams`, `BrowserParams`, `ComplianceParams`, `IntegrationsParams`,
`WorkflowParams`, `VulnParams`, `WirelessParams` — because the enumeration
constructs each variant with a default payload. `Default` is additive and
changes no serde shape.

`crates/eggsec-runtime/src/capabilities.rs` was rewritten around
`RuntimeCapabilities::canonical()`, plus a new `with_task_kinds(list)` that
lets a host make its published set and the runtime's enforced set the same
object. `daemon_conservative()`, `full_lab()`, and the private
`task_kind_name()` table are deleted.

### The executor declaration (work package B)

`EggsecRuntimeExecutor::supported_task_kinds()` now declares the canonical
list explicitly. The trait's default was changed to the canonical list, so
"says nothing" means "declares everything" — which is why **every** executor
that rejects tasks now declares the empty set explicitly. That includes three
test executors and the frontend-client's `NoopExecutor`, which previously
inherited the conservative set.

### Admission and hydration (work package C)

`Runtime` captures the enforced capability set at construction and exposes it
synchronously as `enforced_task_kinds()`. `RuntimeSession::hydrate_from_snapshot`
now **takes** the capability set as a parameter rather than substituting
`RuntimeCapabilities::default()`, and `Runtime::hydrate_session` passes the
runtime's configured set. A recovered session therefore admits exactly what a
fresh session admits, and a snapshot carrying a stale `capabilities` field
cannot narrow it.

### The wireless identity fix (work package D)

`TaskKind::WirelessActive` now resolves to `wireless-deauth` in the wire-side
table, and `CanonicalOperationRequest::WirelessActive` resolves to
`wireless-deauth` in the engine's canonical table. `executor_route_for` gained
the `wireless-deauth` arm. The CLI already resolved its deauth command
against `wireless-deauth`, so the daemon and the CLI now authorise the same
work at the same risk level (`Intrusive`, `RawPacketProbe`,
`wireless-advanced`).

### The interface-bound target story (work package E)

`TargetPolicyKind` gained `InterfaceBound`, with a documented five-point
contract: it accepts a `None` target; it accepts an optional **local
identifier** as the descriptor's target, carried into the audit record; it
keeps `requires_explicit_scope` true; it keeps
`requires_private_or_local_target` false; and the local identifier is **not**
matched against the scope's remote target rules.

`OperationDescriptor` gained a `target_policy` field (additive,
`#[serde(default)]`, behaviour-preserving because only the interface-bound
value branches) so the enforcement kernel can tell a local identifier from a
remote host. The kernel's scope branch now handles the interface-bound case
explicitly: the explicit-scope requirement still denies when no scope is
configured, and a configured scope satisfies the strict profiles' ambiguity
check.

The catalog split:

| Wire kind | Before | After |
|---|---|---|
| `packet-capture` | `packet` (shared) | a new `packet-capture` operation, interface-bound; `packet` keeps `traceroute` and `packet-send` and stays `ExplicitScopeRequired` |
| `wireless` | `wireless` (target-required) | `wireless`, interface-bound |
| `wireless-active` | `wireless` | `wireless-deauth`, interface-bound |
| `resume` | `pipeline` (target-less) | **no split** — the bridge resolves the checkpoint's target and binds it to `pipeline` |

The `packet-capture` operation carries `packet`'s **exact** risk tier
(`RawPacket`) and capability (`RawPacketProbe`): the split relaxes the
*target*, never the authorisation. `packet` still serves `packet-send`, which
writes raw frames to a destination, and `pipeline` still serves `resume`,
which addresses a real host; both keep `ExplicitScopeRequired`.

`resume`'s target is resolved by `crate::pipeline::session::read_target`, a
synchronous reader for the enforcement path. A checkpoint that cannot be
read, or that carries no target, is a **hard failure** — the bridge never
falls back to a target-less resume, and the canonical path fails the
descriptor build closed the same way.

### Publication ≡ admission (work package F)

`DaemonHost::new` computes the published list **first** and hands it to the
runtime as its enforced set. `DaemonHost::with_runtime` — which has no
production caller — now returns `Result` and **refuses to construct** when
the runtime's enforced set is not the published set, because a published list
and an admitted list that disagree are exactly the inconsistency this host
exists to make unrepresentable. The new `DaemonHostError` carries both lists
so the disagreement is diagnosable.

### The published-versus-admitted evidence

Every host construction path, with the published list and the enforced list
shown to be the same object:

| Construction path | Published list | Enforced list | Evidence |
|---|---|---|---|
| `DaemonHost::new(config, executor, store)` | `published_task_kinds(&declared)` = the canonical list ∩ the executor's declaration | the **same list**, handed to the runtime as `RuntimeCapabilities::with_task_kinds(published.clone())` | `crates/eggsec-daemon/tests/manual_capability_policy.rs::the_published_list_is_the_enforced_list` |
| `DaemonHost::new_noop(config, store)` | empty (the stub declares the empty set) | empty (`RuntimeCapabilities::noop()`) | `a_rejecting_executor_publishes_nothing` |
| `DaemonHost::with_runtime(config, runtime, store)` | the canonical list ∩ the runtime's declaration | the runtime's own enforced set | the constructor **refuses** when the two differ: `DaemonHostError::CapabilitySetsDisagree` |

A real executor publishes **all 30** canonical names — `the_canonical_capabilities_are_published`
asserts the count and every name. A rejecting executor publishes **nothing**
and admits **nothing**.

### The interface-bound evidence

| Wire kind | Operation | Local identifier carried | Explicit scope still required | Descriptor builds |
|---|---|---|---|---|
| `packet-capture` | `packet-capture` (new) | the interface name (`eth0`) | yes | yes — `packet_capture_descriptor` |
| `wireless` | `wireless` | the interface name (`wlan0`) | yes | yes — `wireless_descriptor` |
| `wireless-active` | `wireless-deauth` | the BSSID, falling back to the interface | yes | yes — `wireless_active_descriptor` |
| `resume` | `pipeline` (unchanged) | the **checkpoint's** target, not a local identifier | yes | yes, when the checkpoint is readable |

The shared operations' policies are proven unchanged by
`the_packet_family_keeps_its_remote_target_requirement` (`packet` stays
`ExplicitScopeRequired`, refuses a target-less run, and the capture keeps
`packet`'s exact risk tier and capability) and by guard 172.

The `resume` failure mode: `crate::pipeline::session::read_target` returns
an error when the checkpoint cannot be read, when it is not a checkpoint, or
when it carries no target. The bridge maps that to
`RuntimeBridgeError::InvalidTarget` naming the checkpoint path, and the
canonical path resolves `None`, which fails the descriptor build closed. A
resume is **never** authorised against no target — there is no fallback.

### The GUI's honest reporting (work package G)

`Denial` gained a `RiskPolicyDenied { risk }` case, recognised from the
policy layer's own wording (`operation risk '<X>' is not allowed by current
execution policy`). The GUI's command path now learns a risk-policy denial
as well as a feature denial, so a published-but-policy-denied kind is
reported as **blocked with the reason**, never as available.

The coverage model carries the **interface-bound** fact in the existing reason
string of the existing `Available` verdict: an interface-bound operation's
reason says it binds to a local interface, not to a network target. The
verdict set is unchanged — six verdicts — and the IPC command count is
unchanged at twelve.

## 4. Verification executed

### Commands run

```bash
cargo fmt --all -- --check
cargo check --workspace --no-default-features
cargo test -p eggsec-runtime
cargo test -p eggsec-policy
cargo test -p eggsec-daemon-protocol
cargo test -p eggsec-frontend-client
cargo test -p eggsec-output
cargo test -p eggsec-daemon --features service
cargo test -p eggsec --features cli --lib
cargo test -p eggsec-gui --no-default-features --lib
cargo test -p eggsec-gui --features shell --lib
cargo test -p eggsec-cli --features daemon-client
cargo test -p eggsec-python
cargo clippy -p eggsec-runtime --all-targets -- -D warnings
cargo clippy -p eggsec-policy --all-targets -- -D warnings
cargo clippy -p eggsec-daemon-protocol --all-targets -- -D warnings
cargo clippy -p eggsec-frontend-client --all-targets -- -D warnings
cargo clippy -p eggsec-gui --all-targets -- -D warnings
cargo clippy -p eggsec-gui --no-default-features --all-targets -- -D warnings
cargo clippy -p eggsec --features cli -- -D warnings
cargo clippy -p eggsec-cli --features daemon-client -- -D warnings
cargo clippy -p eggsec-daemon --features service --all-targets -- -D warnings
bash scripts/check-architecture-guards.sh
python3 scripts/check_doc_references.py
make check-msrv
```

### Results

All commands below were run in this environment on macOS (arm64, Rust
stable). Every one that is listed **passed**.

#### Formatting, compilation, and the workspace

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | pass — clean after the milestone's own formatting pass |
| `cargo check --workspace --no-default-features` | pass — no errors |

#### Tests

| Command | Result |
|---|---|
| `cargo test -p eggsec-runtime` | pass — 102 lib + 22 integration |
| `cargo test -p eggsec-policy` | pass — 88 lib |
| `cargo test -p eggsec-daemon-protocol` | pass — 89 lib + 7 integration |
| `cargo test -p eggsec-frontend-client` | pass — 13 lib |
| `cargo test -p eggsec-output` | pass — 73 lib + 7 + 12 integration |
| `cargo test -p eggsec-daemon --features service` | pass — 108 lib + 1 + 10 + 20 + 4 integration (the 4 are this milestone's `manual_capability_policy.rs`) |
| `cargo test -p eggsec --features cli --lib` | pass — 1401 lib |
| `cargo test -p eggsec-gui --no-default-features --lib` | pass — 79 lib |
| `cargo test -p eggsec-gui --features shell --lib` | pass — 83 lib |
| `cargo test -p eggsec-cli --features daemon-client` | pass — 31 lib |
| `cargo test -p eggsec-python` | pass — 237 lib |

#### Lints

| Command | Result |
|---|---|
| `cargo clippy --lib -p eggsec -- -D warnings` | pass |
| `cargo clippy --lib -p eggsec --features cli -- -D warnings` | pass |
| `cargo clippy -p eggsec-core -p eggsec-tool-core -p eggsec-report-model -p eggsec-output -p eggsec-runtime -p eggsec-ui-model -p eggsec-agent -p eggsec-transport -p eggsec-transport-eggfetch -p eggsec-policy -- -D warnings` | pass — this is the `make clippy` set |
| `cargo clippy -p eggsec-cli --features daemon-client -- -D warnings` | pass |
| `cargo clippy -p eggsec-runtime --features schema -- -D warnings` | pass |
| `cargo clippy -p eggsec-gui --all-targets -- -D warnings` | pass |
| `cargo clippy -p eggsec-gui --no-default-features --all-targets -- -D warnings` | pass |
| `cargo clippy -p eggsec-daemon-protocol --all-targets -- -D warnings` | pass |
| `cargo clippy -p eggsec-frontend-client --all-targets -- -D warnings` | pass |
| `cargo clippy -p eggsec-runtime --all-targets -- -D warnings` | pass — this **was** failing at the baseline with 3 `collapsible_match` findings in `crates/eggsec-runtime/tests/in_process_client.rs`; the milestone fixed them, because the milestone also edits that file |
| `cargo clippy -p eggsec-policy --all-targets -- -D warnings` | **not clean, and not this milestone's doing** — 8 pre-existing findings in `decision.rs`, `policy.rs`, and `scope.rs`, none in a line this milestone added. `make clippy` lints the policy crate with `--lib`, so the contract is unaffected |
| `cargo clippy -p eggsec-python --all-targets -- -D warnings` | **not clean, and not this milestone's doing** — 3 pre-existing findings in `audit.rs` and `tool_core.rs`; the milestone's Python edits are in `operation_metadata.rs` and `tool_descriptor.rs`, which are clean |
| `cargo clippy -p eggsec-daemon --features service --all-targets -- -D warnings` | **not run to a clean result** — 80 pre-existing engine findings in roughly 20 files the `service` profile newly compiles; they are not in the daemon crate and not introduced here |

#### Static guards, doc references, MSRV

| Command | Result |
|---|---|
| `bash scripts/check-architecture-guards.sh` | pass — **ALL PASSED: No architecture drift detected.** The highest guard number is now **172** (the baseline's was 168), and there are **171** distinct check numbers. The four new checks are 169–172 |
| `python3 scripts/check_doc_references.py` | pass — **Total missing: 29**, unchanged from the baseline |
| `make check-msrv` | pass — exit 0 on Rust 1.89 |

#### The four new guards, negative-tested

Each guard was shown to **fail** under a deliberate violation and to pass
again after the restore. The injections were applied to copies of the tree
and reverted with a file copy — never a `git checkout`, which would have
discarded uncommitted work.

| Guard | Deliberate violation | Result |
|---|---|---|
| 169 | `RuntimeCapabilities::daemon_conservative()` re-added to `crates/eggsec-runtime/src/capabilities.rs` | the guard failed with *"a retired capability constructor (daemon_conservative/full_lab) is back"*, and passed after the restore |
| 170 | `crates/eggsec-daemon/src/host.rs` changed to build the enforced set from `RuntimeCapabilities::canonical()` instead of the published list | the guard failed with *"builds the runtime's enforced set from a constant instead of the published list"*, and passed after the restore |
| 171 | `TaskKind::WirelessActive(_) => "wireless"` restored in the wire-side table | the guard failed with *"the wire-side table still maps WirelessActive to the passive wireless operation"*, and passed after the restore |
| 172 | the catalog's `packet` operation relaxed to `TargetPolicyKind::OptionalTarget` | the guard failed with *"the catalog's packet operation was relaxed to OptionalTarget"*, and passed after the restore |
| 172 | the catalog's `wireless` operation relaxed from `InterfaceBound` to `ExplicitScopeRequired` | the guard failed with *"the catalog's wireless operation is not interface-bound"*, and passed after the restore |

#### The measured canonical sets

Every number below was **measured**, not assumed.

| Set | Measured | Command |
|---|---|---|
| `TaskKind` variants | **30** | `rg -c '^\s+[A-Za-z0-9]+\([A-Za-z0-9]+\) => "' crates/eggsec-runtime/src/request.rs` |
| Distinct capability names | **30** | `cargo test -p eggsec-daemon-protocol --test coverage_multiplicity` — `EXPECTED_DISTINCT_CAPABILITY_NAME_COUNT` is 30 and the test passes |
| Distinct operation ids behind the task kinds | **28** | `cargo test -p eggsec-daemon-protocol --test coverage_multiplicity` — `EXPECTED_DISTINCT_OPERATION_ID_COUNT` is 28 and the test passes |
| Catalog operations | **35** | `cargo test -p eggsec-daemon-protocol --test coverage_multiplicity` — `EXPECTED_OPERATION_COUNT` is 35 and the test passes; `rg -c '    OperationMetadata \{' crates/eggsec-policy/src/catalog.rs` |
| Distinct target-policy labels | **6** (`no-target`, `optional-target`, `target-required`, `explicit-scope-required`, `private-or-local-required`, `interface-bound`) | `rg -o '^    (NoTarget|OptionalTarget|TargetRequired|ExplicitScopeRequired|PrivateOrLocalRequired|InterfaceBound),' crates/eggsec-policy/src/target.rs` |
| Capability names in the canonical table | **30** | `rg -c '^\s+[A-Za-z0-9]+\([A-Za-z0-9]+\) => "' crates/eggsec-runtime/src/request.rs` (the table's rows) |

**Divergences from the plan's expectations, and their reasons:**

- The plan expected **28 distinct operation ids** and **35 catalog
  operations**. Both hold. The plan's expectation of **26** operation ids
  (the pre-milestone count) moved to 28 because the `packet-capture` split
  gave the capture and the deauth their own operations: `packet` went from
  three wire shapes to two, and `wireless` from two to one.
- The plan described the description map as "keyed by capability name". It
  is, and it is generated from the same table as the names, so its key set
  is exactly the canonical name set **by construction** rather than by
  agreement. `description_table_covers_exactly_the_canonical_names` asserts
  it anyway.
- The plan's "exhaustive enumeration" mechanism was implemented as a
  **macro-generated enum** rather than a hand-written `vec![]` of variants.
  A `vec![]` literal is *not* exhaustive — adding a variant to it breaks
  nothing — so it would not have met the plan's "adding a `TaskKind` variant
  MUST be a compile error until the derived list covers it". The macro
  generates the enum **and** the derived list from one list, which meets it.

#### Commands NOT run

These are recorded as **not run**, not as passed:

- `make check` — it stops at the inherited `check-deps` advisory backlog
  (23 entries across 20 advisory IDs, unchanged by this milestone). Every
  step of `make check` before `check-deps` was run directly and is green.
- `make check-python` — it requires a `.venv-ci/` this environment does not
  have.
- `make check-full`, `make check-features-individual`, `make clippy-domain`,
  `make test-tui-pty` — deep checks, not part of the per-milestone contract,
  and not run here.


## 5. Invariant review

| Invariant | Evidence | Result |
|---|---|---|
| `EnforcementContext::evaluate()` remains the mandatory pre-dispatch gate | The enforcement path was not restructured; the change is in the *descriptor* the gate evaluates and in the *reason* a refusal carries | pass |
| Strict surfaces dispatch only via `EnforcedDispatcher::dispatch_execution()` with an `ApprovedExecution` bundle | Untouched by this milestone | pass |
| `OperationMetadata` is the single source of operation policy | The interface-bound value was added to `TargetPolicyKind` and consumed from the catalog; no operation's policy is decided anywhere else | pass |
| `Scope` remains fail-closed and no local-host subject was invented | The scope model was **not** changed. The interface-bound value's scope semantics are "explicit scope required; the local identifier is not matched against the scope's remote target rules" — see §10 | pass |
| `TaskKind::operation_id()` is the single wire-side match | The engine's canonical table and the wire-side table both changed, and they changed **together**; no second table was added | pass |
| The runtime bridge never imports Clap/Ratatui/daemon-protocol/Python types | Unchanged | pass |
| The engine's `policy_bridge/` owns feature/resolver/`NetworkAuthority` adapters | Unchanged | pass |
| TLS is ring-only; Tokio features are declared per crate | No dependency or feature change in this milestone | pass |
| No `let _ =` and no `filter_map(|e| e.ok())` | The touched code uses `tracing`/explicit handling; the new `read_target` returns `Result` | pass |
| The GUI links no engine/daemon/CLI/TUI crate; the shell framework stays in `crates/eggsec-gui`; no second TLS stack; the CSP forbids remote origins | Guards 155–158 still pass | pass |
| No parallel lookup tables | The capability-name table is single (`TaskKind::capability_name()`); the operation-identity tables are the two pre-existing ones, changed in lockstep | pass |

## 6. Failure and recovery review

- **Duplicate delivery and idempotency.** The capability list is derived and
  immutable after construction; there is no per-request state to duplicate.
- **Cancellation races.** Untouched: the executor's cancellation path is
  unchanged.
- **Daemon restart and session recovery.** This is the milestone's change
  here: a recovered session previously admitted the retired conservative
  set, so a daemon restart silently narrowed what it would accept. The
  recovered session now enforces the runtime's current configured set.
- **Partial persistence failure.** The session snapshot's persisted
  `capabilities` field is now **not** authoritative for the recovered
  session — the runtime's configuration is. A snapshot written by an older
  build therefore cannot narrow a recovered session.
- **Malformed or unauthorised input.** A `resume` request naming an
  unreadable or target-less checkpoint is refused at authorisation with a
  named reason, not dispatched.
- **Bounded event and artifact behaviour.** The GUI's bounded retention
  (guard 160) is unchanged; the GUI's denial ledger is unchanged.

## 7. Migration and compatibility review

- **Protocol version.** Unchanged at 2. The published capability list
  **widened** — a published capability the client did not previously see.
  This is a compatibility event in the "the daemon now offers more" direction:
  a client that renders one row per published task kind gains rows. The
  daemon protocol's own message shapes did not change.
- **Schema migration.** None. `OperationDescriptor` gained one additive
  `#[serde(default)]` field, so a legacy payload still deserializes.
- **Operator configuration.** Unchanged. The `[execution_policy]` risk flags
  keep their defaults, and no scope file, profile, or daemon option changed.
- **Wire behaviour change.** `wireless-active` now resolves to a
  *different* operation id (`wireless-deauth`) than it did. That is the fix,
  and it is deliberate: the previous behaviour authorised a deauth under the
  passive operation's risk level.
- **Rollback limitations.** Reverting the code reverts the widened
  publication, the operation-id change, and the new target-policy value
  together; they are not independently revertible without reintroducing the
  defect.
- **Legacy paths.** The CLI's `packet capture` still resolves against the
  `packet` operation, which still requires a remote target. The plan's
  acceptance criterion says `packet` must stay `ExplicitScopeRequired`, and
  the split is a wire-kind split, not a CLI-command split. The CLI's
  behaviour is therefore unchanged. See §10.

## 8. Security review

- **Authorisation.** The risk-policy defaults are unchanged and remain the
  sole gate for the hazard families. `ADR-0011` records the option of
  flipping those defaults as **rejected**: it would remove the operator's
  consent gate for every surface, including the strict ones.
- **A published capability is not a permitted one.** This is the milestone's
  central security claim, and it is enforced by construction: the daemon's
  refusal for a hazard family names the risk tier the operator has to allow.
- **The wireless-active fix closes a live finding.** A deauth is an
  intrusive, raw-packet operation; while it was authorised under the
  passive operation's risk level, the daemon would have run it at the
  passive tier's authorisation. It is now `wireless-deauth` everywhere.
- **Scope.** The scope model is unchanged. The recorded relaxation — the
  local identifier is not matched against the scope's remote target rules —
  is stated in full in §10 and is the milestone's only one.
- **Target normalisation.** An interface-bound operation's target is **not**
  normalised as a network target (`normalized_target` stays `OperationTarget::None`),
  so a local identifier can never be resolved as a hostname.
- **Privilege boundaries.** The `packet-capture` split keeps `packet`'s risk
  tier and capability requirement exactly; the engine's capability check is
  unchanged.
- **Audit behaviour.** The local identifier is carried into the descriptor
  and from there into the decision record, so it is never silently dropped.
- **Denial of service.** The published list is bounded by the canonical
  task-kind set; no unbounded growth path was introduced.

## 9. Documentation and operations

Updated:

- `docs/DAEMON.md` — the Capabilities section (the canonical list,
  publication ≡ admission, the two distinct refusals) and the honest
  limitation (17 of 30 feature-gated off in the CLI-hosted build).
- `architecture/runtime.md` — the `RuntimeCapabilities` table.
- `architecture/daemon.md` — the capability-honesty test list and
  invariant 10.
- `architecture/overview.md` — the canonical operation count (35) and the
  guardrails list (Checks 169–172).
- `docs/CI_ARCHITECTURE_GUARDS.md` — the new invariants section for
  Checks 169–172, and Check 165's wording.
- `.opencode/skills/eggsec-daemon/SKILL.md` — "Capability honesty".
- `.opencode/skills/eggsec-gui/SKILL.md` — the guard range (155–172).

New static guards: **169** (the capability list is derived), **170**
(publication and admission are the same object), **171** (`wireless-active`
resolves to `wireless-deauth`), **172** (the interface-bound target policy is
coherent). Each was negative-tested — see §4.

## 10. Unresolved findings

| Severity | Finding | Impact | Required action |
|---|---|---|---|
| **medium** | **The recorded scope relaxation.** The scope model has **no** first-class local-host subject. The interface-bound target policy's scope semantics are therefore exactly: *the operator's explicit scope is still required, and the local identifier is not matched against the scope's remote target rules.* The CLI's wireless paths no longer scope-check the interface name or the BSSID as a host. This is the milestone's **only** recorded scope relaxation, and it is deliberately **not** described as "the scope check now applies to the local host" — the model cannot say that. | An interface-bound operation is authorised by the operator's risk policy and the presence of a scope, not by the scope naming the local interface. | Accepted for this milestone and recorded in `ADR-0011`. A future milestone that needs the local binding itself in scope requires a **local-host subject in the scope model** — a scope-model change, not a policy change. |
| **low** | The CLI's `packet capture` still resolves against the `packet` operation, which still requires a remote target, while the daemon's `packet-capture` wire kind resolves against the new interface-bound `packet-capture` operation. | The CLI's capture still scope-checks the interface name as a hostname. The plan's acceptance criterion required `packet` to stay `ExplicitScopeRequired`, and the split is a wire-kind split. The CLI's behaviour is therefore **unchanged** by this milestone. | Reconcile when the CLI's packet command is next revisited. It is a **pre-existing** behaviour, not a regression. |
| **low** | `make check` cannot complete: it stops at the inherited `check-deps` advisory backlog (23 entries across 20 advisory IDs, unchanged by this milestone). Every step before it passes; the steps after it were run directly and are green. | The mandatory contract is not green end-to-end. | Owned by milestone 015. |
| **low** | `make check-python` was not run: it requires a `.venv-ci/` that this environment does not have. The Rust-side Python crate's tests were run. | The Python bindings' own guards and type checks are unverified here. | Run `make check-python` where the venv exists. |
| **low** | The `service` profile is not clippy-clean: 80 pre-existing engine findings in roughly 20 files the profile newly compiles. `make clippy` is unaffected. | Pre-existing, not introduced here. | Out of scope; see the advisory backlog's milestone 015. |
| **low** | `cargo clippy -p eggsec-runtime --all-targets` reports 3 pre-existing `collapsible_if` findings in `crates/eggsec-runtime/tests/in_process_client.rs`, a file this milestone did not modify. | Pre-existing. | Out of scope. |

## 11. Roadmap disposition

**Milestone closed, and the next dependency may proceed.** Milestone 010
implemented `ADR-0011` as accepted: the daemon publishes and admits every
canonical capability kind, publication and admission are the same object,
and enforcement is unchanged. Milestone 011 (the daemon-mediated policy
approval path) remains the prerequisite for the *operator-visible* outcome —
a manual client still cannot answer a confirmation — and the roadmap records
that dependency.

## 12. Registry updates

- `plans/registry.md` — the 010 plan's entry moves from "ready for handoff"
  to **implemented 2026-10-24**, with this closure record named; the
  subsystem's status paragraph moves from "one plan is registered and ready
  for handoff" to "milestone 010 is implemented and closed"; `ADR-0011`'s
  entry is marked **accepted**.
- `plans/subsystems/desktop-gui-frontend-roadmap.md` — milestone 10's
  status becomes **implemented 2026-10-24** with the closure record named;
  the milestone's `ADR-0011` dependency, previously "waits on its
  acceptance", becomes **met**.
- `plans/adrs/ADR-0011-daemon-manual-capability-policy.md` — the status
  line moves from `proposed` to `accepted`.

## Addendum 2026-10-24 — two stale engine assertions found and corrected

Discovered while the **mandatory contract** (`make check`) was re-run after
milestones 012–015 were implemented. `make check` had never completed on this
branch — it stopped at the inherited `check-deps` advisory backlog — so the
engine's integration tests had not run since milestone 010's implementation
commit `1877d8e4`. Two of them were left behind by that commit.

**F1 — `crates/eggsec/tests/canonical_dispatch_ownership.rs` asserted the
pre-split packet operation id.** Milestone 010 gave the capture half of the
packet family its own operation (`packet-capture`, because a capture binds to
a **local interface** while `packet` serves `traceroute` and `packet-send`,
which address a remote host). It updated
`crates/eggsec-runtime/src/request.rs` and the catalog's own self-test, but not
this integration test, which still asserted that all three wire kinds map to
`packet`. The test now asserts the deliberate split and is renamed
`packet_family_splits_capture_and_keeps_explicit_wire_kinds`; the original name
stated a falsehood the milestone had already retired.

**F2 — `crates/eggsec/tests/enforcement_matrix.rs` asserted the pre-split
scope refusal.** The same milestone changed the `InterfaceBound` target policy
so that an interface-bound operation **accepts** a target-less run and carries
the requirement as `requires_explicit_scope` on the descriptor, rather than
failing descriptor construction. The catalog's own self-test
(`interface_bound_operations_still_fail_closed`) carries the updated
semantics; this integration test still asserted that descriptor construction
fails. It now asserts the same posture as the catalog's test, and — the
property the test's own name and doc comment are about — that a strict
automated surface still refuses the operation with `ScopeMissing` once a local
identifier is supplied and no explicit scope manifest is loaded.

**F3 — `crates/eggsec-tui/src/parity.rs` had not classified the new
`packet-capture` operation.** The TUI's parity check requires every canonical
operation to have a TUI tab or an intentional CLI-only classification, and it
names the exceptions in a set. Milestone 010 added `packet-capture` and
classified `wireless-deauth` in the same commit, but not `packet-capture`. The
TUI's Packet tab *does* drive it — the tab's capture path runs a capture, which
binds to a local interface and is therefore its own operation — so the correct
classification is not "CLI-only". The parity check now carries a third
coverage set, `covered_via_tab`, naming the operations a TUI tab drives
without declaring them as the tab's own operation, with `packet-capture` in
it and the reason stated in a comment. The invariant is unchanged in strength:
a canonical operation that no tab drives and that is not classified still fails
the check.

None of the three changes alters product behaviour: all three were stale with
respect to decisions milestone 010 had already made and self-tested in
`crates/eggsec-policy/src/catalog.rs` and in the TUI's own tab specs. No
production behaviour changed. The corrections are recorded here rather than by
rewriting this record's findings, and they are carried by commit `dcfcd4e2`
("fix(tui,engine): correct the assertions milestone 010 left stale").

**Why this surfaced now.** `make check` had never completed on this branch: it
stopped at the inherited `check-deps` advisory backlog, which milestone 015
cleared. With that gate green, the contract reached the engine's and the TUI's
integration tests for the first time, and these three stale assertions were
waiting there.
