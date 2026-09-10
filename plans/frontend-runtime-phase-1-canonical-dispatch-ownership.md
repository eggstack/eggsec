# Phase 1 Plan: Canonical Dispatch Ownership

## Status

Status: Ready for handoff.

Depends on: Phase 0 parity/binding guards green.

## Objective

Remove the remaining dual ownership between command metadata and actual execution routing. Operation-backed CLI, embedded TUI, daemon/runtime, and programmatic adapters should converge on one engine-owned execution seam after they have produced the existing canonical typed operation request and `ApprovedOperation`.

This phase must **reuse**, not replace, the canonical request work already completed in `eggsec-tool-core::operation_request` and `eggsec::operation_request`.

## Starting topology

At the audited baseline the relevant flow is approximately:

```text
CLI
  Cli/Commands
      -> command registry lookup/validation
      -> handle_command exhaustive Commands match
      -> handle_* function
      -> per-handler descriptor/enforcement/execution/output

TUI embedded/runtime
  key/palette/tab state
      -> TUI command/action logic
      -> runtime TaskKind / frontend TaskDispatcher
      -> eggsec::dispatch::CommandDispatcher methods
      -> engine executor

Daemon
  TaskRequest/TaskKind
      -> runtime lifecycle
      -> daemon/frontend dispatcher adapter
      -> engine execution
```

Canonical request normalization exists inside this topology, but the path that chooses the executor remains split.

## Design principle

There should be exactly one production owner for the mapping:

```text
(canonical operation ID + canonical typed request + ApprovedOperation)
                              ->
                       engine executor
```

Other layers may own conversion into that tuple and conversion of results out of it. They should not own a second semantic mapping from operation name to implementation.

## Workstream 1.1 — Define the engine execution boundary

Introduce or formalize a narrow engine application service. Naming is implementation-dependent; examples are `OperationExecutor`, `ExecutionService`, or a refined `CommandDispatcher`.

The boundary must accept enough information to prove authorization and preserve type safety. A suitable conceptual API is:

```rust
execute(approved: ApprovedOperation, request: CanonicalOperationRequest, sink: ExecutionSink)
    -> Result<OperationOutcome, ExecutionError>
```

The concrete request representation does **not** need to be a single enormous enum if that harms type locality. Acceptable implementations include:

- a typed enum whose variants wrap the existing canonical request structs;
- a registry of statically typed executor adapters selected by canonical operation ID;
- family-specific typed executor methods behind one application facade.

Requirements:

- canonical operation ID is not inferred independently from user-facing aliases at execution time;
- request/approval binding is checked immediately before executor entry;
- feature availability is checked in one predictable layer;
- execution does not depend on Clap, Ratatui, terminal types, daemon protocol DTOs, or Python objects;
- executor output is domain/event data, not preformatted terminal strings.

Avoid a dynamic `HashMap<String, Box<dyn Any>>` or JSON dispatch map. The repo already invested in typed canonical contracts; retain compile-time exhaustiveness.

## Workstream 1.2 — Convert the CLI registry from prelude to routing contract

`commands/registry.rs` currently establishes command metadata and dispatch class while `handle_command` separately chooses the handler through an exhaustive `Commands` match.

Refactor so the registry/classification and CLI conversion path have one coherent owner.

A practical migration:

1. Parse Clap as today.
2. Classify the `Commands` variant once as operation-backed, operation multiplexer, helper, or lifecycle.
3. For operation-backed variants, convert CLI args to the existing canonical request adapter.
4. Resolve canonical `OperationMetadata`, construct the descriptor from canonical request target/metadata, and obtain `ApprovedOperation`.
5. Invoke the engine execution boundary from Workstream 1.1.
6. Render/serialize the returned outcome through the CLI/output adapter.
7. Keep helper/lifecycle commands on explicit non-operation paths.

The final CLI path may still contain an exhaustive `Commands` conversion match because Rust enum conversion is useful and compile-time safe. The problem to remove is **two separate exhaustive mappings that both claim semantic routing ownership**.

If the command registry cannot itself contain typed conversion functions cleanly, keep the static metadata registry and add one adjacent `Commands -> CommandRoute` conversion whose exhaustiveness is compiler-enforced. Do not force function pointers with heterogeneous argument types into metadata merely to eliminate a match statement.

## Workstream 1.3 — Separate command routing from CLI rendering

Many existing `handle_*` functions likely combine some subset of conversion, policy checks, execution, progress output, and final rendering. Split only where needed to establish the application seam.

Preferred responsibilities:

```text
CLI adapter:
- parse Clap DTO
- shallow validation helpful to CLI UX
- canonical request conversion
- invoke application service
- choose output format / exit semantics

Application/engine:
- canonical request validation/normalization
- canonical target/descriptor construction
- approval binding verification
- executor selection
- cancellation/progress lifecycle
- typed outcome/error
```

Do not mechanically rewrite every small helper command. Scope this work to operation-backed execution and operation multiplexers.

## Workstream 1.4 — Normalize execution events/results

`eggsec::dispatch::CommandDispatcher` currently exposes progress-channel APIs shaped partly around TUI needs. Define a frontend-neutral event vocabulary for long-running operations.

At minimum distinguish:

- accepted/started;
- structured progress (completed/total or phase where known);
- finding/result item where streaming is meaningful;
- warning/diagnostic;
- completed outcome;
- cancelled;
- execution failure.

Use a bounded or backpressure-aware channel where high-frequency producers can otherwise outrun the UI. If some operations cannot block safely on progress delivery, define explicit loss/coalescing semantics for noncritical progress events while never dropping final outcomes/findings silently.

The event model should live in a dependency location usable by the runtime and TUI without pulling terminal code into the engine. Existing `eggsec-ui-model` or a dependency-light execution-contract module is preferable to creating another frontend-specific event type.

## Workstream 1.5 — Move embedded TUI execution onto the same engine boundary

`eggsec-runtime::TaskDispatcher` may remain the lifecycle abstraction used by the TUI, but its embedded implementation should become a shallow adapter:

```text
TaskRequest/TaskKind
  -> canonical request conversion
  -> descriptor/enforcement
  -> canonical engine execution boundary
  -> runtime progress/outcome conversion
```

The frontend crate must no longer own an independent mapping from operation identity to domain executor beyond this explicit conversion.

Where `TaskKind` variants multiplex an operation family (for example packet variants), conversion must select a canonical operation/request before approval and execution. Unsupported runtime tasks must fail explicitly.

## Workstream 1.6 — Put daemon execution behind the same boundary

The daemon is allowed to own session transport, task persistence, cancellation commands, wire versioning, and remote progress delivery. It should not own a second implementation dispatch table.

Add equivalence tests demonstrating that a canonical request executed through embedded and daemon-backed runtime paths reaches the same engine executor and produces semantically equivalent outcome/error classification. Exact progress timing/order need not be byte-for-byte identical, but terminal state and final result semantics must agree.

## Workstream 1.7 — Preserve programmatic surfaces

REST/MCP/gRPC/Python/tool API paths that already use `ApprovedOperation` and canonical request adapters must either invoke the new execution service directly or remain behind an existing facade that delegates to it.

Do not create a CLI-first application service that programmatic callers cannot use. The architecture should converge *below* surface ergonomics.

Run `make check-python` whenever request/result contracts used by Python are touched.

## Workstream 1.8 — Remove transitional routing concepts after migration

After all operation-backed routes use the canonical execution boundary:

- remove registry bridge code whose only purpose was to validate metadata before a separate legacy handler match;
- remove duplicated operation-to-executor mappings proven redundant by Phase 0 tests;
- update comments that describe engine paths as TUI-only when the executor is actually surface-neutral;
- retain explicit helper/lifecycle dispatch categories where they remain semantically useful;
- retain compatibility aliases only at input/wire boundaries.

Do not remove `TaskKind` or CLI `Commands`; both are valid boundary representations. Remove only duplicate **semantic ownership**.

## Acceptance criteria

- operation-backed CLI commands convert once into canonical requests and reach one engine execution service;
- command registry/classification cannot say one route while a separate legacy match silently executes another operation;
- embedded TUI runtime and daemon runtime delegate to the same engine executor selection;
- operation aliases are resolved before the execution boundary;
- executor code does not import Clap/Ratatui/daemon/Python presentation types;
- exact request/approval binding is validated at executor entry;
- progress/outcome contracts are frontend-neutral and preserve cancellation/final-result semantics;
- helper and lifecycle commands remain explicit non-operation routes;
- Phase 0 support/parity tests remain green with fewer independent mapping owners.

## Required tests

Add end-to-end application-boundary tests for representative families:

```text
scan-ports
recon
fuzz
waf-detect
load-test
pipeline
packet family
one auth family (auth/graphql/oauth)
one feature-gated domain operation
one no-target/local-management operation where runtime-supported
```

For each supported surface pair, assert the same canonical request normalization, operation identity, target binding, executor route, and terminal outcome class.

Add cancellation tests proving no detached task survives cancellation for both embedded and daemon-backed adapters.

## Verification

At minimum:

```text
make fmt
make test-feature-matrix
make test-architecture-guards
make check
make check-feature-profiles
make check-python   # when shared programmatic contracts are touched
```

Use `make check-full` for closure if system prerequisites are available.

## Handoff constraints

Keep commits bisectable. A recommended order is: introduce neutral execution/event contracts; adapt one representative operation end-to-end; add equivalence tests; migrate remaining operation families; then remove redundant routing code. Avoid a single rewrite that changes CLI routing, runtime wire format, TUI state, and output formatting simultaneously.

Append a completion record listing every removed/retained dispatch mapping and why any retained mapping is a legitimate boundary conversion rather than duplicate execution ownership.
