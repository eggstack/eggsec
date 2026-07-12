# Python API Milestone G — Extensibility and API Stabilization

Status: Executed

## Goal

Finalize the long-term Python extension model, native ergonomics, compatibility policy, packaging matrix, documentation, and release gates required for a stable public API.

## Dependencies

- Milestones A through F substantially complete.
- Stable operation, event, finding, artifact, policy, and daemon schemas.
- Existing type-stub/export CI guard and wheel workflow.

## Workstream G1 — Operation registry

Expose a read-only operation registry with stable descriptors, request/result schema references, risk/capability metadata, feature availability, and supported backends.

Optionally support controlled Python-defined post-processors, finding enrichers, result adapters, and orchestration providers. Do not move packet-rate or request-rate execution into Python.

## Workstream G2 — Event protocol stabilization

Version and finalize event types for planning, preflight, resolution, stage lifecycle, progress, findings, artifacts, cancellation, failure, and completion.

Local and daemon execution must share the same schema. Define ordering, replay, dropped-event accounting, serialization, and compatibility guarantees.

## Workstream G3 — Callback and sink contracts

Finalize interfaces for audit sinks, finding sinks, artifact stores, credential providers, event consumers, progress consumers, and logging adapters.

Document threading, GIL behavior, reentrancy, backpressure, exception handling, timeout behavior, shutdown, and whether callbacks are synchronous or queued. Add guarded adapters that prevent user callback failure from destabilizing Rust execution.

## Workstream G4 — Python-native ergonomics

Ensure consistent context-manager support, `pathlib.Path` acceptance, Python `datetime` values, stable enums, collection protocols, async iterators, useful redacted `repr`, equality/hash semantics for immutable DTOs, and typed overloads.

Support pickling only for explicitly versioned, secret-free DTOs. Avoid pickling live engine/session/credential objects.

## Workstream G5 — Data and buffer efficiency

Use iterators instead of materialized lists for large results, lazy artifact loading, batch conversion helpers, `memoryview`/buffer protocol for packet and binary artifact data, and optional Arrow-compatible row export where justified.

Benchmark before introducing zero-copy complexity. Small control-plane DTOs should prioritize clarity and stability.

## Workstream G6 — Namespace and import stability

Finalize package namespaces, top-level re-exports, feature-gated import behavior, experimental namespace, and deprecation policy.

Optional features should be discoverable without fragile `AttributeError` probing. Prefer explicit capability descriptors and documented optional imports while preserving backwards compatibility.

## Workstream G7 — Versioning and governance

Document semantic-versioning policy, schema versioning, experimental/stable classifications, deprecation windows, supported Python versions, supported OS/architectures, wheel feature profiles, and Rust/Python ABI expectations.

Add machine-readable version and schema metadata to requests, results, checkpoints, events, findings, and daemon messages.

## Workstream G8 — Documentation

Produce architecture, installation profile, sync quickstart, async quickstart, engine/assessment, scope/authorization, daemon, findings/reporting, domain, migration, notebook, CI, and typed-application guides.

Generate or validate API reference from runtime exports and stubs. Every stable operation should have minimal, async, preflight, and compositional examples where applicable.

## Workstream G9 — Packaging and wheel profiles

Define supported wheel profiles, likely including a portable core profile and explicitly named feature-rich profiles where distribution constraints permit. Document system dependencies and unavailable platform features.

Validate wheels across supported Python versions and Linux/macOS/Windows architectures. Ensure minimal wheels import cleanly and report unavailable capabilities accurately.

## Workstream G10 — Release hardening

Extend CI with:

- Runtime/stub export parity.
- API-surface snapshots.
- Minimal and feature-rich import tests.
- Sync/async contract parity.
- Cancellation/leak/shutdown tests.
- Policy-equivalence tests.
- Serialization compatibility fixtures.
- Documentation build and link checks.
- Wheel smoke tests.
- Deprecation warning tests.
- PyPI provenance/signing checks where supported.

## Workstream G11 — Performance gates

Track engine startup, repeated-call overhead, Python/Rust transition cost, event delivery, large-result serialization, packet-stream backpressure, daemon overhead, callback overhead, and async concurrency scaling.

Set regression budgets and require benchmark review for changes that materially affect hot paths.

## Workstream G12 — 1.0 readiness review

Run a final public API audit for naming, exception hierarchy, type consistency, feature behavior, docs, migration path, security semantics, and packaging. Resolve experimental APIs or move them under an explicit experimental namespace.

## Testing

- API snapshot and semantic compatibility tests.
- Callback stress/reentrancy/failure tests.
- Buffer lifetime and ownership tests.
- Namespace/import tests under every wheel profile.
- Cross-version serialization fixtures.
- Documentation example execution.
- Platform wheel smoke tests.
- Performance benchmark comparison against defined budgets.

## Acceptance criteria

- Public APIs have documented stability classifications.
- Runtime exports and stubs are automatically verified.
- Local and daemon events share a versioned schema.
- Callback contracts are documented and stress-tested.
- Supported wheels install and import without Rust knowledge.
- Deprecated APIs emit actionable warnings with replacements.
- Performance regressions are measured and gated.
- A release candidate satisfies the documented 1.0 checklist.

## Risks

- Stabilizing too early: retain an explicit experimental namespace.
- Optional-feature import confusion: centralize runtime capability discovery.
- Callback complexity: offer safe built-ins before custom hooks.
- Wheel explosion: define a limited, documented distribution matrix.
- Documentation drift: generate/check references against runtime exports and stubs.

## Handoff notes

Begin governance and namespace decisions before final domain work freezes the surface. Land event/callback contracts next, then ergonomics and data efficiency, followed by docs/packaging and the final release-hardening pass. Treat API snapshot changes as reviewed compatibility events.