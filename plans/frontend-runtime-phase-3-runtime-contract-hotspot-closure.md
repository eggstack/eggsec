# Phase 3 Plan: Runtime Contract, Maintenance Hotspots, and Closure

## Status

Status: Executed 2026-09-11 (closure record below; roadmap status updated).

Depends on: Phases 0–2 complete and green.

## Objective

Finish the convergence by making `eggsec-runtime` an explicit transport/lifecycle boundary rather than a second owner of operation semantics, prove embedded/daemon behavioral parity, remove stale transitional comments/mirrors where practical, and decompose remaining maintenance hotspots along the stable boundaries established in Phases 1–2.

This is also the closure/measurement phase for the roadmap.

## Starting state

`eggsec-runtime` is correctly dependency-light and serializable, but currently carries several semantic mirrors:

- `RuntimeSurface` mirrors `eggsec::config::ExecutionSurface` and documents future conversion/convergence work.
- `TaskKind` owns a large wire enum plus parameter structs.
- operation identity and canonical target are expressed on runtime tasks and again through engine adapter mappings.
- `TaskDispatcher` deliberately delegates concrete execution translation to frontend crates.

These are not all defects. Wire DTOs and lifecycle abstractions are legitimate. The defect boundary is crossed when wire types independently define canonical defaults, target semantics, policy identity, or executor selection.

## Design principle

After this phase, runtime types should answer transport/session questions; engine contracts should answer operation questions.

```text
Runtime/daemon owns:
- request IDs / task IDs
- serialization and protocol compatibility
- session attachment
- task lifecycle state
- cancellation transport
- progress/outcome transport
- wire-version compatibility

Engine owns:
- canonical operation identity
- request normalization/defaults
- target semantics
- policy descriptor/enforcement
- executor selection
- domain result/error semantics
```

## Workstream 3.1 — Resolve `ExecutionSurface` / `RuntimeSurface` ownership

Do not keep two semantically independent enums indefinitely.

Choose the least-coupled option after checking crate dependency direction:

### Preferred if dependency graph permits

Move the neutral execution-surface enum to a dependency-light shared crate already consumed by both engine and runtime (for example `eggsec-core` or another established contract crate), then re-export it from current public facades for compatibility.

### Acceptable if wire compatibility requires distinct types

Keep `RuntimeSurface` as a wire DTO but make conversion to/from canonical `ExecutionSurface` exhaustive, centrally implemented, and round-trip tested. The runtime type must be documented as serialization compatibility state, not a domain-semantic owner.

Requirements:

- no stringly conversion;
- no wildcard fallback;
- every new surface breaks compilation/tests until mapped;
- session-bound surface identity remains server/runtime-controlled where required by the enforcement model;
- public compatibility re-exports/deprecations are documented.

Remove the current “later phases can implement conversion” transitional comment once the ownership decision is complete.

## Workstream 3.2 — Make `TaskKind` a wire adapter, not a semantic registry

Retain `TaskKind` where it provides useful stable daemon/runtime serialization, but eliminate redundant semantic methods/mappings after Phase 1 provides one engine execution service.

A target shape is:

```text
TaskKind + params
    -> one exhaustive runtime-to-engine conversion
    -> canonical typed request + canonical operation ID/route
```

There should not be separate independently maintained implementations of:

- `TaskKind::operation_id()`;
- engine `operation_id_for_task_kind()`;
- target extraction in multiple modules;
- runtime defaults that duplicate canonical request defaults;
- frontend executor selection by `TaskKind` variant.

If a runtime-side operation ID is useful for telemetry before engine conversion, derive it from the same exhaustive conversion table or generated typed mapping. Do not retain parallel matches simply for convenience.

Unsupported wire kinds must return a structured unsupported-capability error. Never fall through an alias string.

## Workstream 3.3 — Version and test runtime conversions

For every runtime-supported task family, add serialization and conversion tests covering:

- wire JSON/serde stability where compatibility is promised;
- canonical operation route;
- canonical target extraction;
- canonical request defaults after normalization;
- feature-disabled behavior;
- malformed/unknown enum behavior according to protocol policy;
- no-target/interface/file target families;
- multiplexer variants such as packet/wireless where applicable.

If restructuring `TaskKind` would break documented daemon compatibility, introduce an explicit protocol version or compatibility decoder rather than silently changing the wire representation.

## Workstream 3.4 — Embedded/daemon lifecycle equivalence

Run the same representative canonical operations through both runtime modes and assert equivalent lifecycle semantics:

```text
Accepted -> Running -> Completed
Accepted -> Running -> Cancelled
Accepted -> Running -> Failed
Rejected before execution (policy/feature/request)
```

Prove:

- cancellation reaches the same engine cancellation primitive;
- no child task remains detached after cancellation/session teardown;
- final success/failure/cancel state is delivered exactly once;
- late progress cannot resurrect a terminal task state;
- runtime reconnect/attach does not re-execute a completed request;
- daemon transport errors are distinguishable from engine operation failures;
- embedded mode does not receive capabilities or policy semantics unavailable to daemon mode merely because it is in-process.

Exact timing and progress granularity may differ between transports; terminal semantics and domain outcome classification must not.

## Workstream 3.5 — Neutralize progress/event transport

With the Phase 1 execution event contract in place, make TUI and daemon progress adapters consumers of that contract.

Remove engine API names/comments that describe an executor as TUI-only when the behavior is actually domain/application logic. NSE and other feature-gated domains should follow the same rule: frontend-specific availability may differ, but executor ownership should not.

Where existing unbounded progress channels can accumulate arbitrarily, either:

- use bounded channels with backpressure; or
- coalesce/drop only explicitly noncritical progress updates under pressure while guaranteeing final outcomes and findings.

Document the chosen event delivery guarantees and test slow-consumer behavior for at least one high-frequency operation.

## Workstream 3.6 — Close operation-surface maturity gaps revealed by the matrix

Use the Phase 0 support matrix as the backlog oracle. For every canonical operation/runtime task/TUI tab combination still classified as ambiguous, choose a final status:

- supported and wired through the canonical executor;
- intentionally CLI-only;
- intentionally TUI-only navigation/management view;
- intentionally programmatic-only;
- feature-disabled/unavailable shell;
- deprecated compatibility alias;
- unsupported in runtime/daemon for a documented reason.

Do not add new offensive/security functionality solely to make the matrix rectangular. This work is about wiring existing capabilities and making omissions explicit.

Update user-facing help/docs where a feature is intentionally unavailable on a surface.

## Workstream 3.7 — Decompose engine/frontend maintenance hotspots

Only after routing/metadata boundaries are stable, split files that still combine unrelated responsibilities.

### TUI candidates

```text
crates/eggsec-tui/src/tabs/core.rs
crates/eggsec-tui/src/app/command.rs
crates/eggsec-tui/src/app/help_config.rs
crates/eggsec-tui/src/app/apply.rs
crates/eggsec-tui/src/app/export.rs
crates/eggsec-tui/src/app/enforcement.rs
```

### Engine candidates

```text
crates/eggsec/src/commands/handlers/mod.rs
crates/eggsec/src/dispatch/api.rs
crates/eggsec/src/dispatch/security.rs
large policy modules where residual mixed ownership remains
```

Use module cohesion and dependency direction as the criterion. A good split should make forbidden dependencies easier to express and unit tests more local. Avoid fragmentation into dozens of files that each contain only forwarding boilerplate.

Potential stable module boundaries include:

```text
surface catalog / aliases
action parsing
state reducer
execution effects
runtime adapter
help/export presentation
enforcement UI
operation route conversion
engine execution service
event/result conversion
```

## Workstream 3.8 — Strengthen architecture guards without overfitting

After duplicate owners are removed, add low-cost guards that prevent their return.

Examples:

- TUI modules may not import concrete engine executor internals when an application facade exists;
- engine dispatch modules may not import Ratatui/Clap types;
- runtime crate may not depend on `eggsec` if that would create an architectural cycle; use shared contracts instead;
- deprecated pilot registries/migration enums cannot be reintroduced;
- exact approval binding remains required at executor entry.

Keep semantic assertions in Rust tests and dependency/import assertions in static guard scripts.

## Workstream 3.9 — Documentation reconciliation

Update at minimum:

- command/operation registry documentation;
- TUI help/architecture documentation if present;
- runtime/daemon architecture documentation;
- `plans/README.md` completion status;
- any comments that still call completed migrations “pilot”, “legacy”, or “later phase”.

Document the final distinction between canonical operation metadata, frontend presentation metadata, and wire DTO metadata. This distinction is central to preventing future re-convergence work.

## Workstream 3.10 — Measure before/after maintenance surface

Record quantitative closure evidence. At minimum count:

- canonical operations;
- registry command entries by route class;
- TUI tabs/actions by class;
- runtime `TaskKind` variants;
- independently maintained operation-ID maps before/after;
- independently maintained target-extraction maps before/after;
- manual command/tab alias matches before/after;
- files over an agreed hotspot threshold (for example 20 KiB or 500 LOC), with rationale for any retained hotspot;
- parity tests and feature profiles exercised.

The goal is not zero large files or zero conversion matches. The goal is that retained duplication is either compiler-enforced boundary conversion or clearly documented presentation/wire metadata, not competing semantic ownership.

## Acceptance criteria

- execution-surface identity has one canonical semantic owner or an explicit exhaustive wire conversion;
- `TaskKind` no longer independently owns operation semantics duplicated by engine adapters;
- embedded and daemon modes reach the same canonical engine execution seam;
- cancellation/final-state/error semantics are tested across both runtime modes;
- progress/event transport is frontend-neutral at the engine boundary with defined backpressure behavior;
- every ambiguous support-matrix item has an explicit final classification;
- stale migration/pilot comments are removed or converted into current documentation;
- major TUI/engine hotspots are decomposed where the new boundaries make a cohesive split possible;
- architecture guards protect the new dependency direction;
- mandatory verification and representative feature/runtime profiles pass.

## Verification

Mandatory:

```text
make fmt
make test-feature-matrix
make test-architecture-guards
make check
make check-python
make check-feature-profiles
```

Closure/deep validation where prerequisites permit:

```text
make check-full
make check-features-individual
cargo test -p eggsec-runtime
cargo test -p eggsec-tui
cargo test -p eggsec-daemon
cargo check -p eggsec-cli --no-default-features
```

If daemon protocol compatibility is changed, add explicit compatibility fixtures/tests before merging that change.

## Final closure record

When this phase is complete, append a closure section to this file or create a dedicated dated closure document containing:

1. baseline SHA and final SHA;
2. list of plans/phases executed;
3. support-matrix summary and intentional asymmetries;
4. removed duplicate semantic mappings and retained boundary mappings;
5. approval-binding regression evidence;
6. embedded/daemon equivalence evidence;
7. hotspot before/after measurements;
8. exact verification commands and results;
9. known residual debt with owner/removal criterion.

Do not mark the roadmap executed solely because code compiles. Closure requires the parity/mapping tests, runtime lifecycle evidence, and documentation reconciliation described above.

## Completion record (Phase 3 executed 2026-09-11)

Baseline: `585b4fec` (Phases 0–2 executed); final: commit containing this record.

### Workstream disposition

- **3.1 Surface ownership** — Acceptable option taken (wire DTO + exhaustive bridge), because guard 22 forbids `eggsec-runtime` depending on engine/domain crates (a shared enum in `eggsec-core` would violate runtime isolation). `RuntimeSurface` documented as serialization-compat state, not a semantic owner; removed the "later phases can implement conversion" comment; added infallible `execution_surface_to_runtime_surface` + round-trip tests (unit + `runtime_contract_closure` integration). Guard 96 pins bidirectionality.
- **3.2 TaskKind as wire adapter** — Engine `operation_id_for_task_kind`/`target_for_task_kind` (29-arm parallel tables) replaced with thin delegates to the single wire-side `TaskKind::operation_id()`/`canonical_target()` (runtime cannot depend on the engine, so the wire match is the base and the engine wrappers agree by construction). The engine-side typed conversion `CanonicalOperationRequest::from_task_kind` is unchanged and remains the canonical request owner. Removed the duplicate `TaskResult`→envelope matches in the daemon executor and the TUI dispatcher; both consume the new single engine-owned `dispatch::task_result_envelope` (stable wire kinds preserved: `waf`, `traceroute`, etc.). Unsupported wire kinds still fail explicitly (`InvalidTarget`/`FeatureUnavailable`); unknown serde tags are rejected. Guard 94/95 pin delegation and single ownership.
- **3.3 Conversion tests** — New `crates/eggsec/tests/runtime_contract_closure.rs` (16 tests): surface round-trips, 29-kind identity/target/route agreement, wire JSON stability + unknown-tag rejection, no-target/interface family `InvalidTarget` failures, multiplexer family sharing, stable envelope kinds, embedded/daemon seam equivalence, approval-binding regression. No wire representation changed, so no protocol-version bump was needed.
- **3.4 Lifecycle equivalence** — Both adapters now route through the shared `eggsec-runtime::race_with_cancel` primitive (no ad-hoc `cancel.cancelled()` select in adapter code; guard 86 enforces). Pre-cancelled tasks never start detached work; mid-execution cancellation drops the dispatch future and releases senders; the daemon progress forwarder drains on success and aborts when cancellation wins. Terminal semantics (exactly-once delivery, stale-completion guard, reconnect-does-not-re-execute via session snapshots, transport-vs-engine error distinction) are covered by the pre-existing `eggsec-runtime` lifecycle suite (105 tests) plus the new seam-equivalence test. Timing/progress granularity may differ; terminal semantics and outcome classification do not.
- **3.5 Progress/event transport** — `ExecutionSink` (bounded 100, coalescing with explicit drop counter; findings/terminal always deliver) and `RuntimeEventSink`/broadcast (lag recoverable, critical terminal events warn on drop) were already frontend-neutral and remain the boundary. Documented the backpressure contract in `architecture/runtime.md`. Removed engine comments calling the TUI executor path TUI-only where the behavior is shared (`bundle.rs`/`executor.rs` dispatch-path docs corrected to `execute_approved`).
- **3.6 Matrix gaps** — No ambiguous items: `frontend_surface_matrix.rs` and `parity.rs` contain zero TODO/FIXME/ambiguous markers; every entry is classified (operation-backed/multiplexer/helper/lifecycle + explicit exception lists). No new offensive functionality added.
- **3.7 Hotspots** — Targeted decomposition (duplicate matches deleted, see measurements) instead of fragmenting large cohesive files. Retained hotspots recorded with rationale below.
- **3.8 Guards** — New checks 94 (adapter delegation), 95 (single envelope owner), 96 (bidirectional surface conversion), 97 (closure tests wired); check 86 strengthened (adapters must use `race_with_cancel`, ad-hoc select fails).
- **3.9 Docs** — `AGENTS.md` (invariant 10 + runtime-mapping bullet + dispatch-flow line), `architecture/runtime_bridge.md`, `architecture/dispatch.md` (Phase 3 closure record), `architecture/runtime.md`, `docs/ARCHITECTURE.md` (§4.8 flow), skills `eggsec-tui`/`eggsec-daemon`, stale transitional comments converted (`request.rs`, `runtime.rs`, `bundle.rs`, `executor.rs`, `dispatch/mod.rs`, `canonical_execution.rs` non-goals, TUI `state.rs`/`task_runtime.rs`). README has no runtime-internals content (verified — nothing to prune).
- **3.10 Measurements** — Below.

### Before/after: independently maintained mappings

| Mapping | Before | After |
|---|---|---|
| Operation-ID tables | `TaskKind::operation_id()` (runtime) + `operation_id_for_task_kind()` 29-arm match (engine) + `CanonicalOperationRequest::operation_id()` (engine typed) | Wire-side single match; engine adapter delegates (`Some(kind.operation_id())`); typed conversion unchanged (different type, legitimate owner) |
| Target tables | `TaskKind::canonical_target()` + `target_for_task_kind()` 29-arm match | Delegation (`kind.canonical_target()`) |
| Result→envelope | `task_result_to_outcome()` private match (executor, ~150 lines) + `task_result_to_envelope()` private match (TUI, ~150 lines) | `dispatch::task_result_envelope()` single owner; both adapters consume it |
| Surface conversion | Wire→engine only (`runtime_surface_to_execution_surface`), "later phases" comment | Bidirectional + round-trip tests; wire-DTO documentation |
| Cancellation | Shared primitive in daemon path; TUI pre-check + ad-hoc select; daemon select | Both adapters route through `race_with_cancel`; ad-hoc select fails guard 86 |
| Manual alias matches | 0 (Phase 2) | 0 (unchanged) |

### Line counts (hotspot files, before → after)

| File | Before | After | Note |
|---|---|---|---|
| `eggsec-tui/.../task_dispatcher.rs` | 559 | 399 | Duplicate envelope match deleted; consumes engine helper |
| `eggsec/.../runtime_bridge/executor.rs` | 441 | 294 | Duplicate outcome match deleted; `race_with_cancel` |
| `eggsec/.../operation_request.rs` | 646 | 589 | Parallel tables → 2 delegating fns |
| `eggsec/.../dispatch/canonical_execution.rs` | 1835 | ~2000 | + single-owned envelope mapping + docs |
| `eggsec/.../runtime_bridge/surface.rs` | 149 | ~210 | + reverse conversion + round-trip tests |
| `eggsec-runtime/.../request.rs` | 523 | 544 | + wire-DTO docs |
| `eggsec-tui/.../task_runtime.rs` | 453 | 445 | Ad-hoc select → shared primitive |
| `eggsec-tui/.../tabs/core.rs` | 2886 | 2886 | Retained: per-tab render/input impls; splitting would fragment tab cohesion |
| `eggsec/.../commands/handlers/mod.rs` | 1634 | 1634 | Retained: single-owned CLI route table (guard 84); scattering would weaken ownership |
| `eggsec-tui/.../tabs/spec.rs` | 1387 | 1387 | Retained: single surface owner (guard 81); already decomposed via `surface.rs`/`palette.rs` (Phase 2) |
| `eggsec-tui/.../app/help_config.rs` | 1078 | 1078 | Retained: help prose keyed by `Tab`, pinned 1:1 by test |
| `eggsec-tui/.../app/command.rs` | 961 | 961 | Retained: delegates alias lookup to single owner (guard 89) |
| `eggsec/.../dispatch/security.rs` | 731 | 731 | Retained: feature-gated security family workers |
| `eggsec-tui/.../app/apply.rs` `export.rs` `enforcement.rs` | 676/589/672 | unchanged | Retained: stable Phase 2 boundaries (reducer/effects/presentation) |

Net: ~360 lines of parallel semantic matches removed; ~170 lines added as the single engine-owned mapping plus ~60 lines of bridge conversion/tests. New: `runtime_contract_closure.rs` (16 tests), guards 94–97.

### Support-matrix summary and intentional asymmetries

- Canonical operations: 34 (`ALL_OPERATION_METADATA`); compat aliases: 42 (input/wire boundary only, resolved before approval/execution).
- `TaskKind`: 29 variants, all mapped; `RuntimeSurface`: 9 known + `Unknown` (rejected).
- TUI: 33 tabs (25 operation-backed canonical, 1 multiplexer, 3 helper, 1 lifecycle, 3 UI-only) — unchanged from Phase 2.
- Intentional asymmetries (unchanged): Stress/Packet tabs are always-visible availability shells; `reload-scope` is restart-required info only; fuzz uses `--http-session`; storage/integrations/workflow/wireless/interface families carry no canonical target and fail explicitly in daemon paths.

### Approval-binding regression evidence

- `runtime_contract_closure::bundle_binding_predicate_rejects_approve_one_dispatch_another` (integration, public boundary): cross-operation and cross-target descriptors fail `matches_descriptor`.
- `bundle.rs` unit tests: dispatch rejects operation/target mismatch with granular diagnostics.
- `canonical_dispatch_ownership.rs` (41 tests incl. matrix): binding rejections green.
- Full engine lib suite: 1750 passed.

### Embedded/daemon equivalence evidence

- Both adapters call `race_with_cancel` (guard 86 + `task_runtime.rs` cancel-contract tests + `cancel.rs` primitive tests: pre-cancelled never starts work, cancel releases senders, pass-through).
- `embedded_and_daemon_paths_resolve_the_same_canonical_operation`: all 29 kinds agree on operation/target across `from_task_kind` (embedded) and `descriptor_for_run_request` (daemon).
- Runtime lifecycle suite (105 tests): stale-completion guard, timeout, cancel, close-session semantics green.

### Verification commands and results (local, before push)

- `cargo fmt --all`: pass.
- `scripts/check-architecture-guards.sh`: ALL PASSED (incl. new 94–97, strengthened 86).
- `cargo test -p eggsec-runtime`: 105 passed.
- `cargo test -p eggsec --lib`: 1750 passed.
- `cargo test -p eggsec-tui`: 873 passed, 12 ignored.
- `cargo test -p eggsec --features rest-api --test runtime_contract_closure --test frontend_surface_matrix --test canonical_dispatch_ownership`: 57 passed (16 new + 41 existing).
- `make check`: pass (EXIT=0; fmt, no-default workspace check, clippy `-D warnings`, doc tests, no-default package tests, full `--features rest-api` package suite, output tests, TUI lib tests, guards).
- `make check-python`: pass (EXIT=0; no Python-facing contracts changed — engine/runtime/TUI-only phase).
- `make check-feature-profiles`: pass (EXIT=0; incl. `compliance`/`finding-workflow`/`vuln-management` marker profiles that pin `TaskResult` match-arm coverage).
- `make test-feature-matrix`: pass (EXIT=0; 26 passed).
- `cargo test -p eggsec-daemon`: 74 passed. `cargo check -p eggsec-cli --no-default-features`: pass.
- Targeted gated profiles (envelope arms): `stress-testing,packet-inspection`, `db-pentest,web-proxy,wireless`, `c2,advanced-hunting,headless-browser`, `database,external-integrations` — all `cargo check` clean.
- `cargo check -p eggsec-tui --features full`: fails with E0063 in `task_management.rs` (missing `InterceptParams`/`C2Params`/`DbPentestParams` fields) — identical failure documented on clean base HEAD in the Phase 2 record; file untouched by this phase; pre-existing, out of scope. The exhaustive per-feature sweep (`make check-features-individual`) remains the remote deep-checks oracle.

### Known residual debt (owner / removal criterion)

1. `ExecutionSink.legacy_tx` (`u64,u64`) bridge for domain workers — engine dispatch owner; remove when a worker changes its progress contract for an unrelated reason.
2. `dispatch_inner`/`dispatch_task` legacy manual shims — engine dispatch owner; remove when no manual-surface caller remains (currently `dispatch_task` is the manual-compat entry).
3. `ExecutorRegistry` tool-path adapter — tool owner; unify onto `execute_canonical` when the tool dispatch path is next touched.
4. Large hotspot files listed above — respective area owners; split only when a cohesive boundary (not forwarding boilerplate) emerges.
5. `RuntimeSurface` vs `ExecutionSurface` two-type shape — bridge owner; reunify only if guard 22 (runtime isolation) is ever lifted, which is not planned.
