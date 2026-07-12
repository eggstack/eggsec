# Eggsec Python API Corrective and Integration Pass

## Purpose

This plan defines the next corrective pass for `crates/eggsec-python` after implementation of the Python API roadmap milestones A through G.

The repository now has a broad and well-organized public surface, but several core abstractions are only partially integrated. The highest-risk issue is that the new `Engine` executes real Rust operations while discarding domain results and returning zero-filled generic `OperationResult` values. In parallel, operation dispatch, policy, events, cancellation, artifacts, feature introspection, and secret handling are not yet consistently unified across all domains.

This pass must prioritize semantic correctness and end-to-end integration over new feature breadth. No new domain families should be added until the central execution model preserves real data, enforces policy uniformly, emits real lifecycle events, and passes behavior-oriented integration tests.

## Primary outcomes

At completion:

1. `Engine` and `AsyncEngine` preserve complete domain results, findings, artifacts, statistics, timing, and typed failures.
2. Every supported public operation resolves through one authoritative operation registry and one execution path.
3. `ExecutionContext`, policy, scope, audit, events, cancellation, and artifact storage are mandatory execution concerns rather than parallel optional APIs.
4. Sync, async, local, pipeline, and daemon execution use compatible request and result schemas.
5. Feature introspection is internally consistent across `features()`, `has_feature()`, `feature_matrix()`, exports, stubs, and wheel profiles.
6. All secret-bearing fields use redacted secret types and are excluded from unsafe serialization.
7. Tests validate real operation behavior and data preservation, not merely symbol availability or DTO construction.
8. Stability labels reflect actual maturity; incomplete domains remain provisional or experimental.

---

# Workstream 1 — Redesign and repair `OperationResult`

## Problem

Current engine methods convert successful Rust results to Python DTOs and then discard them. The engine returns completion status, empty statistics, and basic metadata rather than the operation output.

This invalidates the central unified-engine abstraction and makes pipelines, repositories, events, reporting, and daemon parity impossible to validate meaningfully.

## Required design

Introduce a versioned, typed result envelope with these conceptual fields:

```text
OperationResult
- schema_version
- operation_id
- execution_id
- status
- started_at
- finished_at
- duration
- stats
- findings
- artifacts
- payload
- metadata
- error
- partial
- cancelled
```

`payload` must retain the domain-specific result. Implement one of the following approaches:

### Preferred approach: tagged payload enum in Rust

Create an internal Rust enum such as:

```rust
pub enum OperationPayload {
    PortScan(PortScanResult),
    EndpointScan(EndpointScanResult),
    Fingerprint(FingerprintScanResult),
    DnsRecon(DnsRecordSet),
    TlsInspection(TlsInspectionResult),
    TechnologyDetection(TechDetectionResult),
    WafDetection(WafDetectionResultPy),
    WafValidation(WafScanResultPy),
    HttpFuzz(FuzzSessionPy),
    LoadTest(LoadTestResultPy),
    // feature-gated variants for remaining domains
}
```

Expose a Python-safe accessor returning the concrete registered Python object.

### Acceptable alternative: `Py<PyAny>` payload

Use a Python object payload only if the ownership, GIL, serialization, pickling, and cross-thread behavior are tested rigorously. Avoid untyped dictionary payloads as the primary representation.

## Required corrections

- Remove all `_py_result` temporary variables whose values are discarded.
- Populate real `ExecutionStats` from domain results.
- Derive findings and artifacts where domain adapters already exist.
- Preserve target, normalized target, operation-specific configuration summary, and feature information.
- Preserve partial results after cancellation or timeout where the Rust engine supports them.
- Add `payload_type`, `payload`, and typed convenience accessors.
- Implement `raise_for_status()` using typed error metadata.
- Ensure `to_dict()` and `to_json()` preserve the payload without losing domain detail.
- Define round-trip behavior for persistence and daemon transport.

## Typed error model

Replace string-only errors with a structured error DTO:

```text
OperationError
- kind
- message
- code
- denial_class
- retryable
- source_operation
- details
- cause_chain
```

Map Rust errors to stable Python error kinds without exposing unstable internal implementation details.

## Acceptance criteria

- A real port scan through `Engine` returns actual open ports in `result.payload`.
- Endpoint, fingerprint, DNS, TLS, technology, WAF, fuzz, validation, and load-test outputs survive the engine boundary.
- No successful engine operation reports zero statistics unless the real operation produced zero values.
- Serialized results round-trip without loss of payload type or data.
- `raise_for_status()` recreates the correct Python exception class.
- Existing convenience functions and engine methods produce equivalent domain data.

---

# Workstream 2 — Create one authoritative operation registry and dispatcher

## Problem

The public API exposes many domains, while `Engine.dispatch()` recognizes only a small core subset. This leaves two architectures: a central engine for selected operations and standalone domain modules for the rest.

## Required architecture

Use one registry as the source of truth for:

- stable operation ID;
- request type;
- result payload type;
- domain;
- required feature flags;
- risk level;
- intended use;
- privilege requirements;
- sync support;
- async support;
- local execution support;
- daemon execution support;
- planning/preflight implementation;
- execution adapter.

The registry should be generated or declared once in Rust and consumed by:

- `Engine.run()`;
- `AsyncEngine.run()`;
- `OperationRegistry`;
- `DomainRegistry`;
- `api_surface()`;
- `feature_matrix()`;
- daemon task conversion;
- documentation/export validation tests.

Avoid hand-maintained parallel match statements where possible.

## Implementation tasks

- Define stable operation ID constants.
- Normalize underscore and hyphen aliases at the boundary only.
- Add typed request conversion for every currently public operation.
- Add operation executors for all default-wheel operations first.
- Add feature-gated executors in domain groups.
- Reject unavailable operations with `FeatureUnavailableError`, not generic unknown-operation failures.
- Reject truly unknown IDs with suggestions derived from the registry.
- Ensure operation descriptors expose the actual request and result schema versions.
- Add registry invariants: unique IDs, unique aliases, valid feature names, registered payload type, and matching sync/async support.

## Migration strategy

Do not remove existing module-level functions. Convert them into compatibility façades that:

1. construct the canonical request object;
2. call the canonical execution adapter;
3. unwrap or return the expected domain result;
4. preserve existing exception behavior where documented.

## Acceptance criteria

- Every public executable operation appears in the registry.
- Every registry operation is dispatchable through the supported engine surface.
- Every exported convenience function maps to exactly one registry operation.
- Unknown feature-gated operations return a feature-unavailable result with required feature metadata.
- Registry, stubs, docs, and runtime exports pass automated parity checks.

---

# Workstream 3 — Make execution context and policy mandatory

## Problem

The repository now contains policy, authorization, scope, preflight, and audit types, but `Engine` primarily stores `Scope`, mode, concurrency, and timeout. Direct scope checks remain the main execution gate.

## Required engine state

Refactor `Engine` and `AsyncEngine` to own a shared immutable execution configuration:

```text
EngineState
- EggsecConfig
- ExecutionContext
- Scope
- AuthorizationPolicy / ExecutionPolicy
- OperationRegistry
- AuditSink
- EventSink / EventLog
- ArtifactStore
- runtime resources
- cancellation registry
- backend selection
```

Provide a compatibility constructor for the current `scope`, `mode`, `concurrency`, and `timeout_ms` signature, but internally normalize it into the full state model.

## Mandatory execution flow

Every operation must pass through these ordered stages:

1. request validation;
2. target normalization and resolution policy;
3. operation metadata lookup;
4. scope evaluation;
5. capability and feature evaluation;
6. privilege evaluation;
7. authorization policy evaluation;
8. preflight result creation;
9. audit emission;
10. execution;
11. finding/artifact normalization;
12. completion/failure/cancellation audit;
13. final result assembly.

No domain executor may call the underlying Rust operation before the common pre-dispatch gate succeeds.

## Policy equivalence

Build fixtures that compare the same request across:

- Python local engine;
- Python async engine;
- daemon submission;
- Rust preflight APIs;
- strict CLI/CI policy path where practical.

Assert equivalent decisions for:

- allowed in-scope operation;
- denied out-of-scope target;
- explicitly excluded target;
- private/loopback resolution;
- cross-host redirect;
- high-risk operation;
- database pentest;
- interception proxy;
- raw packet/stress operation;
- remote execution;
- credential-bearing operation.

## Acceptance criteria

- Engine constructors accept full `EggsecConfig` and `ExecutionContext`.
- Every operation emits a preflight decision before execution.
- Policy denials carry structured denial class and required grant information.
- Audit events exist for allow, deny, override, start, cancellation, failure, and completion.
- No public execution path bypasses the common gate.

---

# Workstream 4 — Wire real events, progress, cancellation, and handles

## Problem

Event, callback, cancellation, backpressure, and handle types exist, but they must be proven to reflect real operation lifecycle rather than only being constructible DTOs.

## Event lifecycle

Emit versioned events from actual execution:

- planning started/completed;
- preflight completed;
- execution started;
- target normalized/resolved;
- stage started/completed;
- progress update;
- finding emitted;
- artifact created;
- cancellation requested/acknowledged;
- failure;
- completion.

Sequence numbers must be monotonic per execution. Timestamps must be generated at emission time and use one documented timezone/format.

## Progress integration

Where the Rust engine already supports progress channels, bridge those channels into the Python event stream instead of synthesizing only start/end events.

For operations without progress support, define a minimum lifecycle contract without fabricating percentage completion.

## Cancellation

- Connect `CancellationToken` to actual Rust tasks.
- Propagate Python `asyncio.CancelledError` into the Rust cancellation path.
- Distinguish requested, acknowledged, and completed cancellation states.
- Preserve partial findings/results where supported.
- Ensure sockets, packet captures, browser processes, proxy listeners, mobile sessions, daemon subscriptions, and child processes are closed.
- Add cancellation deadlines and forced cleanup fallback where cooperative cancellation stalls.

## Execution handles

`ExecutionHandle` should expose live state backed by the running task, not a detached snapshot:

- `status()`;
- `progress()`;
- `events()`;
- `cancel()`;
- `result()` / awaitable result;
- `partial_result()`;
- artifact retrieval;
- deterministic close.

## Backpressure

- Use bounded queues for events and findings.
- Document overflow behavior.
- Never block packet/request hot loops on Python callbacks.
- Track dropped/coalesced event counts in execution statistics.

## Acceptance criteria

- Integration tests observe real progress events from at least port scan, endpoint scan, fuzz, and load testing.
- Cancelling an async operation stops the underlying Rust work.
- Cancellation does not leak tasks, threads, subprocesses, sockets, or channels.
- Event streams terminate exactly once with completion, failure, or cancellation.
- Slow callback consumers do not cause unbounded memory growth.

---

# Workstream 5 — Integrate pipelines, planning, checkpoints, and resume

## Planning

Replace target-only planning with request- and assessment-aware planning:

```python
engine.plan(request)
engine.plan(assessment)
```

The plan must include:

- normalized operation IDs;
- stages and dependencies;
- feature requirements;
- privilege requirements;
- policy and scope decisions;
- target expansion;
- expected artifacts;
- supported backend;
- validation errors;
- reasons execution cannot proceed.

Planning must not send assessment traffic.

## Pipelines

- Execute pipeline stages through the same canonical operation dispatcher.
- Preserve each stage's typed `OperationResult`.
- Aggregate findings, artifacts, timing, errors, and partial status.
- Support stop-on-failure, continue-on-failure, and conditional-stage semantics.
- Emit stage lifecycle events.
- Propagate cancellation through all active stages.

## Checkpoints and resume

- Store versioned request, plan, completed stage results, pending stage IDs, feature set, scope/policy fingerprint, and schema versions.
- Reject incompatible checkpoints with actionable diagnostics.
- Avoid serializing raw secrets; store references only.
- Resume through the canonical execution path and re-run preflight where required.

## Acceptance criteria

- A multi-stage pipeline preserves actual domain payloads for each stage.
- Planning and execution use the same registry and operation IDs.
- Checkpoints round-trip and resume without duplicate completed work.
- Policy changes invalidate or require reauthorization of checkpoints as documented.
- Pipeline cancellation stops active child operations.

---

# Workstream 6 — Local/async/daemon schema parity

## Objective

Ensure callers can switch among local sync, local async, and daemon-backed execution without rewriting request/result handling.

## Tasks

- Use one request schema for local and daemon operations.
- Use one result envelope and payload tagging scheme.
- Use the same event schema and version constants.
- Add backend metadata to results without changing domain payloads.
- Map daemon task status to canonical execution status.
- Preserve typed errors across transport.
- Add protocol/schema compatibility negotiation.
- Make unsupported daemon capabilities discoverable during planning/preflight.
- Test reconnect, cancellation, partial result, and artifact retrieval.

## Acceptance criteria

- Equivalent local and daemon requests deserialize into the same request type.
- Equivalent successful operations return structurally equivalent results.
- Daemon transport does not reduce typed errors to strings.
- Local and daemon event streams use the same event kinds and schema version.

---

# Workstream 7 — Repair feature and export introspection

## Problem

`feature_matrix()` currently reports more features than `features()` and `has_feature()` recognize. This makes capability discovery unreliable.

## Tasks

- Generate `features()`, `has_feature()`, and `feature_matrix()` from one internal feature descriptor table.
- Include all supported feature names and aliases.
- Record compiled, available, runtime-ready, and system-dependency status separately.
- Distinguish compile-time presence from runtime usability, especially for browser, packet, wireless, mobile, NSE, and database backends.
- Add operation-level availability derived from feature and platform state.
- Remove internal `Py` suffix leakage from `api_surface()`.
- Validate top-level imports, `_core` registration, `__all__`, `.pyi` exports, and feature descriptors.
- Validate default, `full-no-system`, and selected system-dependent build profiles.

## Acceptance criteria

- Every feature in `feature_matrix()` is accepted by `has_feature()`.
- `features()` and `feature_matrix()` agree on compiled status.
- Export and stub parity tests pass for each wheel profile.
- No internal class names appear in the public API inventory.
- Runtime dependency failures are distinguishable from compile-time feature absence.

---

# Workstream 8 — Complete secret redaction and configuration safety

## Tasks

Audit all public configuration and request fields for secrets, including:

- proxy authentication;
- HTTP authorization headers;
- database credentials;
- OAuth client secrets;
- session tokens and cookies;
- webhook secrets;
- GitHub/GitLab/Jira credentials;
- remote execution credentials;
- AI provider keys;
- mobile instrumentation secrets;
- certificate private keys.

Convert secret-bearing strings to `SensitiveString`, `SecretReference`, or credential-provider interfaces.

Ensure:

- `repr` and `str` redact;
- JSON serialization omits values or emits references;
- audit events never include raw values;
- exceptions never interpolate secrets;
- pickling is disabled for live secret values;
- checkpoints store references only;
- debug logging is safe;
- equality and hashing do not expose values indirectly.

Add property-based or table-driven redaction tests across all secret-bearing DTOs.

## Acceptance criteria

- No known secret field remains an ordinary public `String` without documented justification.
- Repository-wide tests search serialized output, exceptions, reprs, logs, and audit events for sentinel secrets and find none.

---

# Workstream 9 — Reclassify API stability honestly

## Tasks

Define maturity requirements:

### Stable

- real execution path exists;
- result data preserved;
- policy and cancellation integrated;
- serialization versioned;
- runtime/stub/docs parity;
- behavior tests pass;
- compatibility commitment documented.

### Provisional

- public API shape accepted;
- implementation works but lacks full backend/platform validation or schema freeze.

### Experimental

- platform-sensitive, hazardous, incomplete, or subject to substantial change.

### Internal

- no compatibility guarantee; not top-level exported.

Reclassify:

- core scanning/recon/WAF only after engine result repair;
- Milestone C–E domains as provisional until end-to-end tests pass;
- wireless, evasion, postex, C2, remote/distributed, dynamic mobile, live packet, interception proxy, and AI integration as experimental unless they satisfy the full stable gate.

Update:

- `api_surface()`;
- stability documentation;
- README status language;
- type-stub annotations/comments where used;
- 1.0 readiness checklist.

Do not present 1.0 readiness as complete while blocking semantic issues remain.

---

# Workstream 10 — Behavior-oriented validation suite

## Local fixtures

Create deterministic local fixtures for:

- TCP open/closed ports;
- HTTP endpoints with known status/body/header behavior;
- TLS certificate fixture;
- technology-detection fixture;
- WAF-like filtering fixture;
- fuzz target with controlled reflections/errors;
- load-test endpoint;
- GraphQL fixture;
- OAuth/OIDC metadata fixture;
- authentication/session fixture;
- WebSocket fixture;
- daemon local transport.

Use containers only where necessary; prefer in-process local fixtures for core CI speed and reliability.

## Required tests

### Result preservation

- real open ports appear in engine payload;
- endpoint findings preserved;
- fingerprints preserved;
- DNS/TLS/technology results preserved;
- real load-test counters preserved;
- findings/artifacts included where expected.

### Cross-surface parity

- convenience function vs engine;
- sync engine vs async engine;
- local vs daemon;
- direct operation vs pipeline stage.

### Policy

- allow/deny equivalence;
- manual override restrictions;
- high-risk grant requirements;
- private resolution and redirect boundaries.

### Events/cancellation

- real event sequence;
- monotonic sequence IDs;
- progress presence where supported;
- cancellation propagation;
- partial result semantics;
- no resource leaks.

### Serialization

- request/result/checkpoint/event round-trip;
- schema-version mismatch behavior;
- secret redaction;
- payload type preservation.

### Features/builds

- default profile;
- full-no-system profile;
- representative system-dependent profiles where locally available;
- missing-runtime-dependency diagnostics.

## Test quality requirements

- Avoid tests that only instantiate DTOs unless testing DTO behavior directly.
- Avoid asserting only `status == completed`; assert meaningful domain output.
- Use sentinel values that prove data came from the fixture.
- Ensure skips include explicit feature/platform reason.
- Track integration test counts separately from unit/API-shape counts.

---

# Workstream 11 — Documentation and migration corrections

## Tasks

- Update engine documentation to show real payload access.
- Document error-result versus raised-exception behavior precisely.
- Document canonical operation IDs.
- Document execution context and mandatory preflight path.
- Document event guarantees and unsupported progress cases.
- Document cancellation and partial-result semantics.
- Document local/daemon parity and compatibility negotiation.
- Document feature compiled/runtime-ready distinction.
- Update stability classifications.
- Add migration examples from convenience functions to engine requests.
- Add a corrective-pass completion report with validated operation matrix.

The validated operation matrix should list for every operation:

- request type;
- result type;
- sync support;
- async support;
- engine dispatch;
- pipeline support;
- daemon support;
- policy integration;
- events;
- cancellation;
- persistence;
- feature/profile;
- maturity classification.

---

# Recommended implementation sequence

## Phase 1 — Semantic core repair

1. Redesign `OperationResult` and structured errors.
2. Preserve real payloads and statistics for the existing core dispatcher.
3. Add real result-preservation integration tests.
4. Fix sync/async parity for those core operations.

Do not proceed until core result data survives execution and serialization.

## Phase 2 — Mandatory execution pipeline

1. Refactor engine state around configuration and execution context.
2. Integrate preflight, policy, scope, and audit.
3. Wire actual events and cancellation.
4. Upgrade execution handles.

## Phase 3 — Registry convergence

1. Make the operation registry authoritative.
2. Route all default-wheel operations through it.
3. Migrate feature-gated domain executors in coherent groups.
4. Convert convenience APIs to façades.

## Phase 4 — Pipeline and daemon convergence

1. Route pipeline stages through canonical dispatch.
2. Preserve typed stage results.
3. Align daemon request/result/event schemas.
4. Validate checkpoints and resume.

## Phase 5 — Introspection, secrets, and stability

1. Repair feature API parity.
2. Complete secret-type migration.
3. Reclassify stability.
4. Update docs and readiness checklist.

## Phase 6 — Full validation and closure

1. Run default and feature-rich local validation.
2. Execute behavior-oriented integration suite.
3. Run leak/cancellation tests repeatedly.
4. Produce a completion matrix and list remaining experimental domains.
5. Only then reconsider PyPI publication or 1.0 language.

---

# Explicit non-goals

This pass must not:

- add new security domains;
- expand hazardous capability breadth;
- introduce additional public aliases without need;
- declare 1.0 readiness based solely on symbol/test counts;
- replace typed domain payloads with generic dictionaries;
- fork policy semantics for Python;
- bypass the Rust engine in order to make tests pass;
- weaken scope or authorization requirements;
- require GitHub CI as the sole validation mechanism.

Local reproducible validation remains required because CI availability cannot be assumed.

---

# Completion gates

The corrective pass is complete only when all of the following are true:

1. No core engine method discards a successful domain result.
2. Core operation statistics are derived from actual results.
3. Every stable executable operation is represented in and dispatched through the authoritative registry.
4. Every execution passes through common validation, preflight, scope, authorization, and audit logic.
5. Events and cancellation are backed by real running operations.
6. Pipelines preserve typed stage results and aggregate real findings/artifacts.
7. Local and daemon schemas are compatible and versioned.
8. `features()`, `has_feature()`, and `feature_matrix()` are consistent.
9. Secret sentinel tests show no leakage.
10. Stability classifications match demonstrated maturity.
11. Default-profile behavior-oriented integration tests pass.
12. `full-no-system` builds, imports, and passes applicable integration tests.
13. Representative system-dependent feature builds are validated locally where dependencies are available, with explicit unresolved-platform notes otherwise.
14. Documentation and stubs match runtime behavior.
15. A final corrective-pass report records exact commands, test totals, skipped tests with reasons, feature profiles, and remaining limitations.

## Handoff note

The implementation agent should begin by inspecting `src/status.rs`, `src/engine.rs`, `src/async_engine.rs`, `src/requests.rs`, `src/operation_metadata.rs`, `src/event_protocol.rs`, `src/event_stream.rs`, `src/authorization.rs`, `src/audit.rs`, `src/pipeline.rs`, and `src/daemon.rs` together. Avoid isolated patches that preserve the current split architecture. The first deliverable should be the corrected result envelope and one fully integrated vertical slice—port scanning through request validation, preflight, policy, execution, real payload preservation, events, cancellation, serialization, and sync/async tests. Use that vertical slice as the reference implementation for all remaining operations.