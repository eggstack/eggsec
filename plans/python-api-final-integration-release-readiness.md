# Eggsec Python API Final Integration and Release-Readiness Plan

## Purpose

This plan defines the remaining work required to move `eggsec-python` from strong pre-release quality for the stable core to a defensible release candidate with coherent execution semantics, reliable cross-surface behavior, and a clear boundary between stable and experimental domains.

The corrective integration pass resolved the largest correctness issue: `OperationResult` now preserves real domain payloads, shared engine state exists, sync and async execution are more closely aligned, cancellation and checkpoints are more substantial, and feature introspection parity has been corrected.

The remaining work is narrower but more consequential. It centers on completing the mandatory execution path, eliminating registry and policy drift, making failures and events structurally reliable, hardening deterministic integration testing, and defining release boundaries that accurately reflect implementation maturity.

This plan intentionally avoids adding new tool families. No further feature expansion should occur until the gates in this document are complete.

## Implementation status — 2026-07-12

The integration pass implemented the stable-core registry/dispatch boundary,
structured operation errors, mandatory policy/audit recording, reliable event
delivery accounting, domain-maturity introspection, and the corresponding
Python exports, stubs, tests, and documentation. The default Python test
suite is green.

This is a scoped pre-1.0 release-candidate checkpoint, not a 1.0 completion
claim. Deterministic daemon/pipeline/secret fixtures, publication validation,
and the repository-wide architecture guard debt remain release gates. The
stable-core boundary is intentionally limited to the ten operations listed
below; broader domains remain provisional or experimental.

---

## Current state

The stable engine path currently covers ten core operations:

- `scan_ports`
- `scan_endpoints`
- `fingerprint_services`
- `recon_dns`
- `inspect_tls`
- `detect_technology`
- `detect_waf`
- `validate_waf`
- `fuzz_http`
- `load_test`

These operations now return typed payloads through `OperationResult` and share common engine state.

The broader Python package also exposes additional domain APIs, including GraphQL, OAuth/OIDC, authentication assessment, browser testing, hunting, database testing, NSE, packet inspection, proxying, mobile analysis, compliance, integrations, wireless, evasion, post-exploitation simulation, C2 simulation, distributed execution, notifications, and AI post-processing.

These broader domains should remain provisional or experimental until they use the same execution, policy, event, result, and validation contracts as the stable core.

---

# Workstream 1 — Compiler-enforced operation registry and dispatch

## Objective

Replace the current metadata registry plus separate string-based dispatcher with one compiler-enforced source of truth.

## Problem

The registry currently stores operation metadata, while execution still delegates to a separate dispatch match. This allows drift between:

- registered operation IDs;
- operation metadata;
- sync dispatch;
- async dispatch;
- feature requirements;
- request types;
- result payload variants;
- documentation and API introspection.

## Required implementation

Introduce a canonical operation identifier enum or equivalent sealed representation:

```rust
pub enum StableOperation {
    ScanPorts,
    ScanEndpoints,
    FingerprintServices,
    ReconDns,
    InspectTls,
    DetectTechnology,
    DetectWaf,
    ValidateWaf,
    FuzzHttp,
    LoadTest,
}
```

Each operation definition must carry:

- stable string ID;
- human-readable name;
- domain;
- stability classification;
- required Cargo feature;
- risk level;
- intended-use classification;
- required capability set;
- accepted request type;
- result payload type;
- sync executor;
- async executor or async adapter;
- planning adapter;
- preflight adapter;
- daemon task kind, where supported.

Use one macro, static descriptor table, trait implementation set, or generated registry to derive:

- `Engine.run()` dispatch;
- `AsyncEngine.run()` dispatch;
- operation introspection;
- feature checking;
- API surface reporting;
- docs tables;
- daemon mapping tests;
- stable operation ID constants.

Unknown operation errors should retain suggestion support.

## Tests

Add compile-time or exhaustive tests proving:

- every stable operation has metadata;
- every stable operation has sync and async dispatch;
- every stable operation has a request parser;
- every stable operation has a payload mapping;
- no registry ID lacks an executor;
- no executor lacks a registry entry;
- daemon mappings are complete where declared;
- operation IDs are unique and stable.

## Acceptance criteria

- There is one authoritative operation definition source.
- Adding a new stable operation requires editing one canonical declaration and implementing required traits.
- Registry, dispatch, introspection, and docs cannot silently diverge.
- Existing public operation IDs remain backward compatible.

---

# Workstream 2 — Mandatory execution context, policy, preflight, and audit gate

## Objective

Make the full authorization model the mandatory gateway for every engine operation.

## Problem

The current pre-dispatch path performs scope enforcement, feature checks, and tracing, but does not visibly execute the richer policy model exposed to Python.

## Required implementation

Extend shared engine state to own:

- `ExecutionContext`;
- `AuthorizationPolicy` or equivalent Rust policy object;
- actor/session/correlation metadata;
- operation mode;
- intended use;
- environment classification;
- audit sink collection;
- optional manual override grant;
- capability grants;
- redirect and target-resolution policy;
- secret-handling policy;
- artifact retention policy.

Replace `pre_dispatch_validate()` with a structured gate that returns a typed decision:

```rust
pub struct DispatchDecision {
    pub outcome: Allow | Deny | ConfirmationRequired,
    pub operation: OperationDescriptor,
    pub scope: ScopeDecision,
    pub policy: AuthorizationDecision,
    pub capabilities: CapabilityDecision,
    pub audit_event: EnforcementAuditEvent,
}
```

Every operation path must:

1. normalize the request;
2. resolve and canonicalize targets;
3. evaluate scope;
4. evaluate risk and intended use;
5. verify required capabilities and privileges;
6. evaluate feature availability;
7. emit a structured audit event;
8. reject or continue;
9. attach the decision summary to the result metadata.

The gate must be identical for:

- sync local execution;
- async local execution;
- pipeline stages;
- resumed operations;
- daemon submission;
- convenience functions that delegate through the engine.

Manual overrides must remain impossible on strict automated surfaces unless explicitly allowed by Rust policy.

## Tests

Add a shared policy matrix covering:

- in-scope allow;
- out-of-scope deny;
- excluded-target deny;
- private/loopback resolution;
- redirect boundary crossing;
- high-risk operation without grant;
- high-risk operation with valid grant;
- expired or mismatched override;
- missing feature;
- missing privilege;
- daemon/local equivalence;
- sync/async equivalence;
- audit event content and redaction.

## Acceptance criteria

- No engine execution reaches a domain executor without a structured authorization decision.
- Tracing is supplemental; it is not the audit mechanism.
- Every allow and deny path emits a structured audit event.
- Equivalent requests produce equivalent policy outcomes across supported surfaces.

---

# Workstream 3 — Structured failure model and exception reconstruction

## Objective

Replace string-only failures with a versioned, typed error payload.

## Required implementation

Add a public structured error DTO:

```python
OperationError(
    kind,
    code,
    message,
    operation_id,
    retryable,
    denial_class,
    source,
    details,
    causes,
)
```

Required error kinds should include at least:

- validation;
- configuration;
- scope denial;
- policy denial;
- capability unavailable;
- feature unavailable;
- privilege missing;
- network;
- timeout;
- cancellation;
- parsing;
- serialization;
- daemon transport;
- internal.

`OperationResult` should carry `error: OperationError | None` rather than only a string. Preserve a compatibility `error_message` getter if needed.

`raise_for_status()` must reconstruct the documented Eggsec exception hierarchy:

- `ConfigError`
- `ScopeError`
- `EnforcementError`
- `FeatureUnavailableError`
- `NetworkError`
- `ScanError`
- `TimeoutError`
- `SerializationError`
- `InternalError`
- a cancellation-specific exception if one is public

Ensure sync and async behavior is identical.

## Tests

- round-trip each error kind through JSON;
- verify exception reconstruction;
- verify `retryable` semantics;
- verify denial classes survive daemon transport;
- verify no secret-bearing details leak;
- preserve compatibility for callers reading `result.error` where feasible.

## Acceptance criteria

- No stable engine failure is represented only by an unclassified string.
- `raise_for_status()` raises specific Eggsec exceptions.
- Error serialization is versioned and cross-surface compatible.

---

# Workstream 4 — Event delivery guarantees and backpressure accounting

## Objective

Define which events may be dropped, which must be delivered, and how loss is reported.

## Required implementation

Classify events into delivery tiers:

### Best effort

- granular progress updates;
- repetitive diagnostics;
- sampled packet/request events.

### Reliable within process

- planning completed;
- preflight completed;
- stage started/completed;
- finding emitted;
- artifact created;
- cancellation acknowledged;
- execution failed;
- execution completed.

Implement either:

- separate bounded channels by priority;
- reserved channel capacity for terminal events;
- a small reliable terminal-event queue;
- or a documented equivalent design.

Add event statistics:

- emitted count;
- delivered count;
- dropped count by event kind;
- maximum queue depth;
- consumer lag;
- terminal-event delivery failures.

Expose these through execution stats or an event-stream diagnostics object.

Ensure event sequence numbers remain monotonic and terminal events are emitted exactly once.

## Tests

- saturate progress channel and prove terminal events still arrive;
- verify dropped progress accounting;
- verify finding events according to declared guarantee;
- verify cancellation produces one terminal event;
- verify daemon reconnect does not duplicate terminal events;
- verify event ordering for sync, async, pipeline, and daemon paths.

## Acceptance criteria

- Silent unaccounted event loss is eliminated.
- Delivery guarantees are documented by event type.
- Terminal events are reliable and exactly-once within the supported process/session model.

---

# Workstream 5 — Deterministic integration fixtures and non-skippable core tests

## Objective

Replace permissive localhost tests with deterministic fixtures that prove real behavior.

## Required fixtures

Create controlled test services for:

- TCP open/closed port validation;
- HTTP endpoint discovery;
- service fingerprinting banners;
- DNS responses where practical;
- local TLS with known certificate properties;
- technology-detection headers and bodies;
- WAF detection and validation responses;
- HTTP fuzzing targets;
- load-test endpoint;
- WebSocket endpoint if included in a stable profile;
- daemon local transport.

Prefer in-process Rust or Python fixtures bound to ephemeral localhost ports.

Tests must assert actual values:

- expected open port is present;
- expected closed port is absent or classified correctly;
- endpoint result contains known route;
- fingerprint result contains expected service/banner;
- TLS result contains expected SAN/issuer properties;
- WAF result matches fixture behavior;
- fuzzing returns expected findings;
- load testing reports correct request totals;
- payload serialization preserves all relevant fields;
- cancellation stops a deliberately slow fixture;
- timeout is deterministic.

Tests should not skip merely because an operation fails. A fixture setup failure should fail the test with diagnostics.

## Acceptance criteria

- Stable-core behavior tests are deterministic and non-skippable on supported CI platforms.
- Result preservation is proven with exact payload assertions.
- Cancellation and timeout tests exercise real running Rust work.

---

# Workstream 6 — Stable-core expansion criteria and provisional-domain boundary

## Objective

Define how additional domains graduate into the unified engine without forcing premature parity.

## Required implementation

Create a machine-readable domain maturity table with states:

- `stable-core`
- `provisional`
- `experimental`
- `internal`

A domain may enter `stable-core` only when it has:

- canonical operation IDs;
- request DTOs;
- typed `OperationPayload` variants;
- sync and async dispatch;
- mandatory policy/preflight integration;
- structured errors;
- events and cancellation;
- deterministic integration fixtures;
- serialization tests;
- daemon mapping where declared;
- documentation and stubs;
- supported wheel profile coverage.

Recommended graduation order:

1. consolidated recon;
2. GraphQL;
3. OAuth/OIDC;
4. authentication assessment;
5. daemon task operations;
6. NSE execution;
7. database assessment;
8. browser assessment;
9. proxy/interception;
10. packet inspection and mobile dynamic analysis.

Wireless, evasion, postex, C2, distributed, and AI-related domains should remain experimental until the stable core release is complete.

## Acceptance criteria

- The top-level API clearly communicates domain maturity.
- Experimental modules are not reported as stable by `api_surface()`.
- Graduation is checklist-driven rather than commit-message-driven.

---

# Workstream 7 — Secret-type completion and redaction audit

## Objective

Ensure every credential-bearing value uses one redacted secret abstraction.

## Required implementation

Audit all Python-bound configuration and request types for:

- proxy authentication;
- HTTP authorization headers;
- OAuth client secrets;
- database credentials;
- API tokens;
- integration tokens;
- daemon authentication;
- remote execution credentials;
- AI provider keys;
- mobile and instrumentation secrets;
- certificate private-key material.

Replace raw strings with `SensitiveString`, `SecretReference`, or a purpose-specific redacted wrapper.

Guarantee redaction in:

- `repr`;
- `str`;
- exceptions;
- audit events;
- event payloads;
- result metadata;
- JSON serialization;
- pickle support;
- debug logging;
- daemon messages where values are not explicitly required.

Do not permit accidental `to_dict()` exposure. Secret access should require an explicit method such as `expose_secret()`.

## Tests

Use sentinel secret values and scan:

- all repr/str outputs;
- serialized objects;
- captured logs;
- audit/event streams;
- raised exceptions;
- daemon request/result snapshots;
- report output.

## Acceptance criteria

- No raw credential field remains in stable public DTOs.
- Secret sentinel tests cover all supported serialization and logging paths.

---

# Workstream 8 — Daemon/local schema and lifecycle parity

## Objective

Complete the local-versus-daemon abstraction so callers can switch backends without rewriting request/result handling.

## Required implementation

Align daemon schemas with the canonical operation model:

- same operation IDs;
- same request schema versions;
- same result payload types;
- same structured errors;
- same status states;
- same event envelope;
- same cancellation semantics;
- same checkpoint identifiers;
- same policy decision summaries;
- same artifact references.

Implement explicit capability negotiation:

- daemon protocol version;
- supported operation IDs;
- feature flags;
- executor profile;
- platform capabilities;
- schema versions;
- maximum request/result sizes;
- event replay support.

Clarify lifecycle behavior for:

- submit;
- attach;
- detach;
- reconnect;
- cancel;
- retrieve result;
- retrieve artifacts;
- resume event stream;
- daemon restart.

## Tests

Run the same contract test suite against:

- local sync engine;
- local async engine;
- daemon client using local transport.

Normalize expected transport-only differences.

## Acceptance criteria

- Core requests and results are interchangeable across local and daemon execution.
- Capability mismatch failures are structured and actionable.
- Reconnection behavior is deterministic and documented.

---

# Workstream 9 — Pipeline, checkpoint, and resume hardening

## Objective

Ensure composed assessments preserve data and lifecycle semantics across failure, cancellation, and resume.

## Required implementation

Pipeline stage results must retain:

- typed domain payload;
- findings;
- artifacts;
- structured errors;
- policy decision;
- execution statistics;
- event sequence range;
- checkpoint metadata.

Define failure policies:

- fail fast;
- continue independent stages;
- continue with partial inputs;
- retry;
- skip dependent stages;
- rollback/cleanup where supported.

Checkpoint schemas must include:

- schema version;
- Eggsec version;
- operation registry version;
- pipeline definition hash;
- normalized target inventory;
- scope/policy fingerprint;
- completed stage results or references;
- pending stages;
- artifact references;
- event sequence state.

Resume must reject incompatible checkpoints with structured diagnostics.

## Tests

- successful multi-stage pipeline;
- failure with fail-fast;
- failure with continue policy;
- cancellation mid-stage;
- checkpoint after partial completion;
- resume and compare with uninterrupted result;
- policy change invalidates checkpoint;
- feature change invalidates unsupported resume;
- payload/artifact preservation across checkpoint serialization.

## Acceptance criteria

- Pipeline composition does not discard domain data.
- Resumed results are semantically equivalent to uninterrupted execution where applicable.
- Invalid resume conditions fail before network activity.

---

# Workstream 10 — API stability reclassification and documentation correction

## Objective

Align documentation and machine-readable stability labels with actual maturity.

## Required implementation

Reclassify public APIs using strict definitions:

### Stable

- used end-to-end by supported execution paths;
- deterministic behavior tests exist;
- serialization contract is versioned;
- sync/async parity is verified;
- policy and cancellation semantics are integrated;
- supported in declared wheel profiles.

### Provisional

- API shape is intended for public use;
- implementation works but contract may still evolve;
- incomplete common-engine integration or limited platform validation.

### Experimental

- hazardous, platform-specific, or incomplete domain;
- no compatibility guarantee;
- may require source builds or system dependencies.

Update:

- `api_surface()`;
- stability classifications document;
- namespace documentation;
- package README;
- API reference;
- feature matrix;
- migration guidance;
- 1.0 readiness checklist.

The package should not claim broad 1.0 readiness until all release gates below are satisfied.

## Acceptance criteria

- Stable labels are limited to fully validated APIs.
- Provisional and experimental status is visible through introspection and docs.
- No documentation claims parity that the common engine does not yet provide.

---

# Workstream 11 — CI and release evidence

## Objective

Make validation visible, reproducible, and enforceable on the default branch.

## Required CI matrix

At minimum:

### Platforms

- Linux x86_64;
- macOS arm64;
- Windows x86_64 as experimental or required, depending on release scope.

### Python versions

- minimum supported Python;
- primary supported Python;
- latest supported Python.

### Build profiles

- default/core;
- full-no-system;
- selected feature-gated builds;
- daemon-client profile;
- source distribution build;
- wheel install smoke test.

### Checks

- `cargo fmt --all --check`;
- targeted Clippy for Python and modified crates;
- Rust unit/integration tests;
- Python test suite;
- deterministic fixture tests;
- stub/runtime export parity;
- feature introspection parity;
- serialization compatibility;
- policy-equivalence matrix;
- daemon/local contract suite;
- wheel build/install/import;
- documentation links/build;
- API snapshot review;
- secret redaction tests;
- performance budget tests.

Publish CI status on commits and protect the release branch/tag from bypassing required checks.

## Acceptance criteria

- Default branch tip has visible passing status checks.
- Release artifacts are produced only from a validated commit.
- Test counts in commit messages are supplemental, not the sole evidence.

---

# Workstream 12 — Packaging and release-candidate preparation

## Objective

Prepare a scoped pre-1.0 or 1.0 release candidate only after semantic gates close.

## Required implementation

Finalize:

- package name and ownership;
- version policy;
- Python classifiers;
- supported Python/platform matrix;
- wheel profiles;
- source distribution behavior;
- feature-unavailable errors;
- bundled type stubs and `py.typed`;
- license and metadata;
- provenance/signing where supported;
- changelog;
- migration notes;
- security policy;
- vulnerability reporting route;
- release checklist;
- rollback/yank procedure.

Run installation validation in clean environments:

- fresh virtual environment;
- no Rust toolchain for wheel installs;
- source build with Rust toolchain;
- minimal import;
- stable-core operation fixture;
- type checker smoke test.

## Release naming recommendation

Until broad domain integration is complete, prefer one of:

- `0.x` stable-core release;
- `1.0.0rc1` explicitly scoped to the stable core;
- or a documented `1.0` where experimental modules are clearly excluded from compatibility guarantees.

Do not imply that all Eggsec domains are stable merely because they are importable.

## Acceptance criteria

- Wheel installation works on all declared primary platforms.
- Stable-core examples execute successfully from installed artifacts.
- Release documentation accurately states included and experimental domains.

---

# Implementation sequence

## Phase 1 — Semantic closure

1. Compiler-enforced operation registry.
2. Mandatory execution context and authorization gate.
3. Structured failure model.
4. Event delivery guarantees.

These changes may require schema adjustments and should land before further compatibility commitments.

## Phase 2 — Verification closure

1. Deterministic integration fixtures.
2. Non-skippable stable-core execution tests.
3. Pipeline/checkpoint/resume hardening.
4. Secret redaction completion.
5. Daemon/local contract parity.

## Phase 3 — Stability and release closure

1. Domain maturity classification.
2. Documentation correction.
3. CI matrix and branch status.
4. Packaging validation.
5. Release candidate decision.

---

# Required test gates

The following must pass before a release candidate:

- all stable-core operations return real typed payloads;
- all stable-core operations have nonzero meaningful statistics where applicable;
- all stable-core operations pass deterministic fixture tests;
- sync and async result schemas are equivalent;
- local and daemon result schemas are equivalent where supported;
- policy outcomes are equivalent across surfaces;
- cancellation stops real work and releases resources;
- terminal events are delivered exactly once;
- event drops are accounted for;
- pipeline/checkpoint/resume preserves payloads and artifacts;
- feature APIs agree exactly;
- no secret sentinel appears in logs, events, errors, reports, or serialization;
- runtime exports match stubs;
- supported wheels install and import cleanly;
- documentation and API snapshots are current;
- required CI checks are visible and passing.

---

# Explicit non-goals

This pass should not:

- add new assessment domains;
- expand hazardous capabilities;
- redesign the entire Rust engine;
- force every experimental domain into the stable core;
- add Python callbacks to performance-sensitive hot loops;
- preserve inaccurate stability labels for compatibility optics;
- publish to PyPI before release gates are satisfied.

---

# Completion definition

This plan is complete when:

- the stable operation registry is the sole source of dispatch truth;
- every stable-core operation passes through structured policy, preflight, audit, event, cancellation, and result handling;
- failures are typed and reconstruct the documented Python exception hierarchy;
- deterministic tests prove real payload preservation and lifecycle behavior;
- local, async, pipeline, and daemon surfaces share compatible contracts;
- event loss is governed and observable;
- secret handling is complete;
- stability classifications match actual maturity;
- CI results are visible on the default branch;
- supported wheels can be installed and exercised in clean environments;
- a release candidate can be cut without implying stability for provisional or experimental domains.

The intended outcome is not maximum apparent feature parity. It is a credible, maintainable stable core with explicit growth paths for the remaining Eggsec domains.
