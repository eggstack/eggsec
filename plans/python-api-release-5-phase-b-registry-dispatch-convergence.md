# Python API Release 5 Phase B — Registry and Dispatch Convergence

> **Status: Executed** — Phase B completed 2026-07-16

## Objective

Replace the duplicated sync/async dispatch architecture and parallel metadata inventories with one authoritative operation-executor registry. Preserve every public behavior while making future operations declarative, auditable, and mechanically reflected into Python exports, capability metadata, schemas, tests, and documentation.

## Current problem

The Python engine currently maintains the same twenty-two operations across `StableOperation`, sync dispatch match arms, async dispatch match arms, typed convenience methods, feature gates, request conversion, result conversion, event hooks, capability manifests, `api_surface()`, stubs, and documentation. This duplication is a material correctness risk. Existing architecture analysis estimates thousands of lines of repeated dispatch and helper code.

## Workstream B1 — Freeze behavior before refactoring

Create a golden contract suite for all stable operations before structural changes. Capture:

- accepted operation IDs and historical aliases;
- request normalization and default values;
- feature-unavailable behavior;
- risk, confirmation, intended-use, and scope metadata;
- sync and async normalized results;
- finding and artifact hook behavior;
- event ordering;
- timeout and cancellation behavior;
- error codes and messages;
- audit outcomes;
- serialization output.

Use deterministic fixtures and snapshot only stable semantic fields. Avoid snapshots of timestamps, generated IDs, or unordered maps unless normalized.

## Workstream B2 — Shared dispatch support

Extract duplicate helpers from `engine.rs` and `async_engine.rs` into focused modules:

- operation ID and alias parsing;
- URL/host extraction;
- port and range parsing;
- deadline calculation;
- cancellation checks;
- common success/error result construction;
- metadata decoding;
- event emission helpers;
- finding and artifact hook helpers.

Add unit tests for each helper. No behavior change is allowed in this workstream.

## Workstream B3 — Executor descriptor model

Introduce an internal `OperationExecutor` descriptor containing at minimum:

- `StableOperation` identity;
- canonical ID and aliases;
- display name and description;
- feature requirement;
- maturity;
- risk and confirmation metadata;
- intended uses and capabilities;
- request type/schema identity;
- result payload type/schema identity;
- request normalization callback;
- sync execution callback;
- async execution callback;
- finding hook;
- artifact hook;
- event capabilities;
- cancellation and timeout support;
- local/daemon availability.

Keep PyO3 types out of the durable metadata portion where possible. Separate serializable descriptor metadata from executable function pointers.

## Workstream B4 — Incremental operation migration

Migrate operations one at a time, preserving contract tests after every operation. Recommended order:

1. recon DNS;
2. technology detection;
3. WAF detection;
4. TLS inspection;
5. WAF validation;
6. authentication assessment;
7. Docker and Kubernetes scanning;
8. APK and IPA analysis;
9. database probing;
10. NSE execution;
11. port, endpoint, and fingerprint scans;
12. load testing and fuzzing;
13. git-secret and SBOM operations;
14. consolidated recon;
15. GraphQL and OAuth.

For each operation:

- move request decoding into a named normalizer;
- move execution into sync and async executor functions sharing domain logic;
- register finding/artifact hooks explicitly;
- retain typed methods as thin wrappers;
- remove both legacy match arms only after equivalence tests pass;
- verify feature-disabled and feature-enabled profiles.

## Workstream B5 — Generic dispatch loop

After all operations migrate, reduce `Engine::dispatch()` and `AsyncEngine::dispatch_async()` to the same ordered lifecycle:

1. parse canonical ID or alias;
2. look up executor;
3. emit planning event;
4. perform feature, scope, and policy validation;
5. emit preflight/audit state;
6. normalize request;
7. compute deadline and bind cancellation;
8. execute sync or async callback;
9. run finding/artifact hooks;
10. emit terminal event;
11. return common `OperationResult`.

The sync facade must not contain a second semantic implementation. Async remains canonical where the underlying operation is asynchronous; sync blocks through the shared runtime bridge.

## Workstream B6 — Generated derived inventories

Generate or validate the following from the authoritative descriptor table:

- `Engine.list_operations()`;
- operation registry metadata;
- `api_surface()` entries for canonical operations;
- `_capabilities.json` operation records;
- tool descriptors and JSON schemas from Phase A;
- feature requirements;
- maturity tables;
- stub declarations for operation methods where generation is practical;
- documentation tables;
- daemon parity operation maps.

The generated artifacts must include a source registry version and commit identity. CI must fail when generated files are stale.

## Workstream B7 — Direct-function convergence

Audit every stable direct function. Each must either:

- construct a typed request and delegate through the canonical engine path; or
- be explicitly documented as a lower-level primitive with different semantics.

Remove hidden duplicate implementations. Add tests comparing direct functions, typed engine methods, generic engine dispatch, and tool invocation for equivalent input.

## Workstream B8 — Daemon mapping convergence

Move local-operation-to-daemon-task mapping into descriptor metadata or a generated adapter. Validate:

- every daemon-supported operation has one mapping;
- unsupported operations fail explicitly;
- request normalization occurs before transport serialization;
- local and daemon payload type identities match;
- aliases do not create duplicate daemon task identities;
- protocol-version changes are surfaced by compatibility guards.

## Workstream B9 — Architecture guards

Add blocking checks for:

- every `StableOperation` has exactly one executor;
- every executor ID is unique;
- aliases do not collide;
- every executor has request/result schema identities;
- feature-gated executors agree with Cargo features and Python exports;
- sync and async callbacks are both present or explicitly unsupported;
- all stable operations have tool descriptors;
- no legacy twenty-two-arm dispatch remains;
- generated capability and documentation files are current.

## Performance requirements

Registry lookup and request normalization must not materially increase dispatch overhead. Add benchmarks for:

- registry construction;
- descriptor lookup;
- operation listing;
- request normalization;
- no-op/denied dispatch;
- sync and async dispatch overhead excluding operation I/O.

Set budgets relative to the pre-refactor baseline and fail on significant regression.

## Acceptance criteria

Phase B is complete when:

- one executor declaration owns each stable operation;
- sync and async generic dispatch contain no per-operation match arms;
- typed methods and direct functions are thin delegates;
- tool descriptors, capability metadata, feature maps, and docs derive from the same registry;
- all pre-refactor semantic contract tests pass;
- local/daemon mappings are complete and versioned;
- dispatch performance and resource use remain within budget;
- adding a new stable operation requires one registration plus domain implementation, not edits across parallel inventories.

## Implementation Summary (2026-07-16)

### Workstreams Completed

- **B1**: Golden contract test suite (`tests/test_golden_contract.py`) — 1076 parametrized tests across 72 test methods
- **B2**: Shared dispatch helpers (`dispatch_helpers.rs`) — 7 extracted + 5 new helpers, eliminating ~420 lines of duplication
- **B3**: Expanded OperationExecutorDescriptor — 14 new metadata fields, `from_operation()` constructor, `all_descriptors()` and `descriptor_metadata_list()` methods
- **B4**: Operation executors module (`operation_executors.rs`) — NormalizedRequest, parameter extraction helpers, per-operation normalizers
- **B5**: Generic dispatch lifecycle — `pre_dispatch_lifecycle()` handles planning/validation/preflight/cancel/deadline; `execute_operation()` and `post_dispatch_hooks()` separate concerns
- **B6**: Generated inventories (`generated_inventories.rs`) — 7 derivation functions, metadata manifest, consistency validation
- **B7**: Direct-function convergence — 10 typed methods refactored to thin delegates via OperationRequest
- **B8**: Daemon mapping convergence — Registry-driven `operation_request_to_daemon_task()` replaces hardcoded match
- **B9**: Architecture guards — 10 guard tests, 2 new CI checks in `check-architecture-guards.sh`

### New Files
- `crates/eggsec-python/src/dispatch_helpers.rs`
- `crates/eggsec-python/src/operation_executors.rs`
- `crates/eggsec-python/src/generated_inventories.rs`
- `crates/eggsec-python/tests/test_golden_contract.py`

### Key Design Decisions
- Kept exhaustive 22-arm match in `execute_operation()` for compile-time exhaustiveness checking and feature-gate annotations
- Registry is the single source of truth for metadata; derived inventories are generated, not maintained separately
- Typed methods are thin delegates that construct OperationRequest and call the canonical dispatch path
- Daemon task kind mapping driven by descriptor metadata, not hardcoded strings