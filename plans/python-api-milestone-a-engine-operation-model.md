# Python API Milestone A — Unified Engine and Operation Model

## Goal

Create the common execution architecture that turns the existing Python bindings into a coherent Eggsec library. All current convenience functions and future domain bindings should converge on this model.

## Dependencies

- Existing `eggsec-python` scope, client, async bridge, DTO, findings, and feature-introspection modules.
- Stable Rust runtime, dispatch, pipeline, and tool request/result abstractions.
- No dependency on Milestone B for initial scaffolding, but enforcement hooks must be designed for Milestone B integration.

## Workstream A1 — Public engine objects

Add `Engine` and `AsyncEngine` with context-manager support. The engine owns shared runtime state, network clients, resolver state, concurrency controls, execution context, audit/event plumbing, artifact policy, and graceful shutdown.

Keep `Client` and `AsyncClient` as compatibility façades. Route their methods through the engine rather than maintaining parallel execution paths. Document a deprecation strategy only after parity is proven.

## Workstream A2 — Typed operation requests

Define a stable `OperationRequest` base protocol and typed request DTOs for every currently bound operation: port scan, endpoint scan, fingerprinting, DNS/TLS/technology recon, WAF detection/validation, HTTP fuzzing, load testing, WebSocket, Git secrets, SBOM, database probes, proxy pool, mobile static, container, packet inspection, stress, NSE, and daemon operations.

Common fields should include operation ID, targets, timeout, concurrency, tags, caller metadata, artifact policy, progress granularity, and optional per-request context overrides where policy permits.

Requests must support validation, `to_dict()`, `to_json()`, schema-version metadata, and daemon-safe serialization.

## Workstream A3 — Common result protocol

Introduce `ExecutionStatus`, `ExecutionStats`, `Artifact`, and `OperationResult`. Every domain result must expose operation ID, execution ID, status, findings, statistics, artifacts, start/end times, partial/cancelled flags, and structured errors.

Standard methods:

- `to_dict()`
- `to_json()`
- `to_rows()` where meaningful
- `summary()`
- `raise_for_status()`
- `write_artifacts()`

Preserve domain-specific fields through composition or subclassing without duplicating lifecycle metadata.

## Workstream A4 — Execution handles and events

Add `ExecutionHandle` and `AsyncExecutionHandle` returned by `submit()`. Handles expose status, progress snapshot, cancellation, partial result access, final result access, and event streaming.

Define initial event types: execution started, planning completed, target resolved, stage started, progress updated, finding emitted, artifact created, stage completed, cancelled, failed, and completed.

Use bounded queues and explicit backpressure/drop accounting. Avoid invoking Python callbacks in request-rate or packet-rate hot loops.

## Workstream A5 — Cancellation and timeout

Add a shared cancellation token bridged into Rust tasks. Python task cancellation must propagate into Rust execution. Distinguish cancellation, timeout, policy rejection, feature unavailability, and execution failure.

Audit cleanup for sockets, files, child processes, captures, proxy listeners, browser processes, daemon streams, and runtime threads. Add leak-focused tests.

## Workstream A6 — Pipelines and assessments

Expose `Assessment`, `AssessmentBuilder`, `Pipeline`, `PipelineStage`, `AssessmentProfile`, and `AssessmentResult`.

Support ordered and conditional stages, dependencies, parallel-safe stages, shared target inventories, finding/artifact propagation, stage failure policy, retries, partial completion, and checkpoints. Mirror stable Rust profiles rather than defining Python-only semantics.

## Workstream A7 — Planning

Add `engine.plan(request_or_assessment) -> ExecutionPlan`. Plans must not send assessment traffic. Include normalized targets, stages, required features/privileges, risk metadata placeholders, target expansion, resource estimates, expected artifacts, backend compatibility, and reasons execution cannot proceed.

Milestone B will enrich plans with full policy and authorization details.

## Workstream A8 — Checkpoints and resume

Add versioned `Checkpoint` and `ResumeToken` types. Validate Eggsec version, operation schema, target set, feature set, pipeline definition, and policy context before resume. Ensure checkpoints never embed raw credentials.

## Workstream A9 — Compatibility migration

Refactor existing top-level functions and `Client` methods to construct typed requests and delegate through the engine. Preserve current signatures during the transition. Add parity tests proving identical results and enforcement behavior.

## Testing

- Unit tests for request validation and serialization.
- Sync/async parity tests for all default-feature operations.
- Cancellation tests for port scan, endpoint scan, fuzzing, load testing, and one feature-gated operation.
- Event-order and bounded-queue tests.
- Pipeline planning/execution/cancellation/resume tests.
- Resource cleanup tests under success, exception, timeout, and cancellation.
- Compatibility tests for existing public functions.

## Acceptance criteria

- All current default-feature Python operations execute through `Engine` and `AsyncEngine`.
- Existing convenience functions delegate to the same path.
- Sync and async result shapes are equivalent.
- Multi-stage assessments support planning, execution, cancellation, checkpoints, and resume.
- No operation bypasses the central engine execution path.
- Result lifecycle metadata is standardized.
- Cancellation does not leak Rust tasks or resources.

## Risks

- Overexposing unstable Rust internals: mitigate with binding-owned DTOs.
- Event overhead: use coarse-grained bounded events and sampling.
- Async bridge complexity: specify loop affinity and shutdown behavior explicitly.
- Compatibility breakage: retain existing functions and add contract tests before deprecation.

## Handoff notes

Implement in small commits: shared DTOs/status, engine skeleton, request conversion for one operation, result protocol, execution handles/events, cancellation, pipeline/checkpoint support, then migrate remaining operations. Keep stubs and runtime exports synchronized in every commit.