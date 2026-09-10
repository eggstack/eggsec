# Phase 3 Plan: Runtime Contract, Maintenance Hotspots, and Closure

## Status

Status: Ready for handoff.

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
