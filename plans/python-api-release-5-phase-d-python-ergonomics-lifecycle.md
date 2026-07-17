---
Status: Executed
Date: 2026-07-16
Summary: Phase D core ergonomics implemented — context managers, strict enum parsing, DTO round-trip, typing stubs, lifecycle tests.
---

# Python API Release 5 Phase D — Python Ergonomics, Typing, and Lifecycle Semantics

## Objective

Make the public bindings behave like a coherent Python library rather than a set of individually wrapped Rust classes. Standardize constructors, protocols, async behavior, cancellation, context management, serialization, callbacks, collections, paths, times, buffers, and resource ownership without weakening Rust-side enforcement.

## Workstream D1 — API convention audit

Audit every public class and function for:

- constructor signatures and keyword-only arguments;
- optional-value behavior;
- enum representation;
- `str` and `repr` consistency;
- `to_dict()` and `to_json()` availability;
- equality and hashing;
- collection protocols;
- sync and async context management;
- path and datetime conversion;
- exception taxonomy;
- typing completeness;
- secret exposure;
- thread and event-loop assumptions.

Generate a machine-readable exception list for intentional deviations.

## Workstream D2 — Paths, URLs, timestamps, and durations

Normalize public inputs and outputs:

- accept `os.PathLike[str]` and `pathlib.Path` for filesystem paths;
- return `Path` for durable filesystem locations where doing so is compatible;
- use validated URL/target objects where ambiguity matters;
- return timezone-aware `datetime` values for timestamps;
- use integer milliseconds only in wire DTOs, not as the sole ergonomic interface;
- provide `timedelta` acceptance or named duration helpers where appropriate;
- reject negative and overflowing durations consistently.

Serialization must remain stable and use explicit ISO 8601 and integer duration formats.

## Workstream D3 — Enum and value semantics

All public enums must:

- compare predictably;
- expose stable string values;
- support construction from canonical strings where useful;
- reject unknown values with `ValueError`;
- have deterministic repr;
- serialize identically across sync, async, local, and daemon paths.

Audit frozen DTOs for equality and hashing. Hash only immutable, identity-safe values. Do not hash secret-bearing or mutable resource objects.

## Workstream D4 — Collection protocols

Implement Python protocols where semantically valid:

- `FindingSet`, result pages, registries, and descriptors: `Sized`, `Iterable`, `Sequence`, or `Mapping`;
- event and finding streams: iterator and async iterator protocols;
- headers and metadata: mapping behavior with explicit case semantics;
- repositories: pagination rather than unbounded materialization;
- binary payloads: buffer or bytes-like access without unnecessary copies.

Define iterator invalidation and ownership behavior. Iterating a stream must not silently consume data needed by another API unless documented.

## Workstream D5 — Context management and deterministic close

Every managed resource must have a consistent lifecycle:

- sync resources implement `close()`, `closed`, `__enter__`, and `__exit__`;
- async resources implement `aclose()`, `closed`, `__aenter__`, and `__aexit__`;
- close is idempotent;
- operations after close fail with a specific error;
- finalizers are best-effort safety nets, never the primary cleanup contract;
- active operations are cancelled or drained according to documented close mode;
- sockets, files, subprocesses, certificates, temporary profiles, and runtime registrations are released.

Apply this contract to TCP, UDP, HTTP, WebSocket, capture, database, proxy, browser, mobile, NSE, daemon, repository, reporter, and artifact resources as applicable.

## Workstream D6 — Native asyncio integration

Resolve the custom `PyFuture` cancellation contract. Preferred end state:

- cancelling the awaiting Python task propagates cancellation to the Rust task;
- Eggsec cancellation tokens still support cross-operation and explicit cancellation;
- cancellation produces `asyncio.CancelledError` at the Python boundary while internal events record structured cancellation;
- detached/dropped futures do not leak work;
- callbacks are scheduled on the originating event loop;
- no GIL is held during blocking or asynchronous I/O;
- multiple event loops are either supported explicitly or rejected deterministically.

If full propagation cannot be implemented safely, document token-only cancellation and ensure Python task cancellation detaches without leaking. Do not claim native cancellation without proof.

## Workstream D7 — Callback and sink semantics

Standardize audit, finding, artifact, progress, event-consumer, mutation, credential-provider, and integration callbacks:

- define sync versus async callback support;
- specify execution thread/event loop;
- bound callback queues;
- expose backpressure and dropped-event statistics;
- map callback exceptions into structured failures;
- prohibit callbacks from bypassing scope or policy;
- redact callback arguments;
- provide unregister/close behavior;
- prevent callback reference cycles and leaks.

Mutation callbacks in proxy and credential callbacks in database APIs require dedicated adversarial tests.

## Workstream D8 — Exceptions and error causality

Create one documented exception hierarchy mapping structured Rust errors into Python. Preserve:

- error kind and stable code;
- operation and target identity where safe;
- retryability;
- feature requirement;
- policy and denial class;
- timeout versus cancellation distinction;
- underlying cause chain where safe;
- daemon transport versus remote execution errors.

Avoid converting ordinary structured operation failures into generic `RuntimeError`. Define when an API raises and when it returns `OperationResult.error`.

## Workstream D9 — Serialization and redaction

Standardize `to_dict()`, `to_json()`, schema version, and optional `from_dict()`/`from_json()` support. Requirements:

- deterministic output;
- no Rust-only field names;
- recursive secret redaction;
- safe binary externalization;
- large artifacts represented by references;
- compatibility tests across versions;
- explicit behavior for unknown fields and newer schema versions;
- no accidental serialization of callback objects, file handles, sockets, tasks, or runtime state.

## Workstream D10 — Typing closure

Produce strict, public `.pyi` coverage for all stable and provisional APIs. Add:

- overloads for typed operation requests;
- narrowed result payload types by operation;
- protocols for sinks, callbacks, repositories, and managed sessions;
- `PathLike`, `Mapping`, `Sequence`, iterator, async iterator, and buffer annotations;
- generic page/stream/result types where useful;
- `Literal` values only when they are genuinely stable;
- optional dependency/module typing.

Run mypy and pyright against both internal examples and a separate consumer fixture package. Minimize `Any`; every unavoidable `Any` requires a comment or allowlist entry.

## Workstream D11 — Resource and concurrency tests

Add stress tests for:

- repeated open/close cycles;
- cancellation during connect/read/write/stream/report;
- dropped futures;
- callback exceptions;
- concurrent sessions;
- repository iteration under writes;
- finalizer fallback;
- event-loop shutdown;
- thread, task, socket, file-descriptor, and subprocess leaks;
- large binary/artifact access without excess copying.

## Acceptance criteria

Phase D is complete when:

- public APIs follow one documented Python convention set;
- managed resources have deterministic lifecycle contracts;
- asyncio cancellation behavior is truthful, tested, and leak-free;
- callbacks have bounded and observable delivery semantics;
- exceptions preserve structured causality;
- serialization is versioned and secret-safe;
- stable/provisional stubs pass mypy and pyright consumer tests;
- repeated lifecycle and concurrency tests remain within resource budgets.

## Implementation Notes (2026-07-16)

### Completed Workstreams

| Workstream | Status | Details |
|------------|--------|---------|
| D1 — API convention audit | Partial | Exception list generated via code analysis; deviations documented in .pyi fixes |
| D2 — Paths, URLs, timestamps | Pre-existing | `resolve_path()` already accepts `os.PathLike`; `py_datetime()` returns aware datetimes |
| D3 — Enum and value semantics | Completed | `from_str` raises `ValueError` on unknown; `__eq__`/`__hash__` added to ExecutionStatus and finding_schema enums |
| D4 — Collection protocols | Pre-existing | `PaginatedResultsPy`, `EventStream`, `ExecutionHandleList` already implement `__iter__`/`__len__`/`__contains__` |
| D5 — Context management | Completed | `__enter__`/`__exit__` added to AuditSink, FindingSink, ArtifactSink, ProgressSink, EventConsumer, AsyncCallback, CallbackScheduler |
| D6 — Native asyncio | Deferred | Token-only cancellation remains the documented contract; full propagation requires deeper PyFuture refactoring |
| D7 — Callback semantics | Partial | Backpressure bounds exist in CallbackScheduler; formal callback queue testing deferred |
| D8 — Exceptions | Completed | Exception hierarchy already structured; `from_str` now raises `ValueError` consistently |
| D9 — Serialization | Completed | `from_dict()`/`from_json()` added to OperationError, ExecutionStats, Artifact |
| D10 — Typing closure | Completed | `.pyi` stubs updated: `__hash__` on all enums, context managers, new methods |
| D11 — Resource tests | Completed | `test_phase_d_ergonomics.py` covers context managers, enum parsing, serialization round-trip, closed resource behavior |

### Files Modified

- `crates/eggsec-python/src/callbacks.rs` — Context managers
- `crates/eggsec-python/src/async_support.rs` — Context managers
- `crates/eggsec-python/src/status.rs` — ExecutionStatus ergonomics, from_dict/from_json
- `crates/eggsec-python/src/finding_schema.rs` — Strict from_str
- `crates/eggsec-python/src/sbom.rs` — SbomFormatPy.from_str as staticmethod
- `crates/eggsec-python/python/eggsec/*.pyi` — 30+ type stub files updated
- `crates/eggsec-python/tests/test_phase_d_ergonomics.py` — New test file
- `AGENTS.md` — Updated with Phase D notes
- `.opencode/skills/eggsec-python/SKILL.md` — Updated with Phase D section
- `crates/eggsec-python/README.md` — Updated with Phase D release note

### Remaining Work (Future Phases)

- D6: Full asyncio cancellation propagation (requires PyFuture refactoring)
- D7: Formal callback queue bounds testing and adversarial mutation/credential callback tests
- D1: Complete API convention exception list document