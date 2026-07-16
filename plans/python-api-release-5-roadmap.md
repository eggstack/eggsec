# Eggsec Python API Release 5 Roadmap

## Purpose

Release 5 turns the broad Releases 1–4 binding surface into a governed public Python platform. The repository already exposes the main assessment engine, twenty-two canonical operations, reusable network primitives, subsystem-specific NSE/proxy/database APIs, managed mobile and browser contracts, daemon parity types, repositories, artifacts, and streaming reporters. The remaining problem is coherence: tool schemas are not yet first-class Python objects, operation dispatch is duplicated, experimental capabilities share the top-level namespace, typing and Python protocols remain uneven, wheel profiles are not yet a durable compatibility promise, and release evidence is still being tightened.

Release 5 is therefore an integration and stabilization release. It must not become another broad capability-expansion tranche. New security behavior is in scope only where required to complete an already exposed contract, make a subsystem genuinely usable, or provide deterministic validation of an existing API.

## Required end state

At Release 5 completion:

1. `eggsec-tool-core` request, response, error, stream, rate-limit, and history types have deliberate Python equivalents;
2. stable Eggsec operations can be converted into framework-neutral tool descriptors and JSON schemas;
3. one authoritative operation registration model drives sync dispatch, async dispatch, metadata, capability manifests, type stubs, and documentation;
4. stable, provisional, and experimental APIs are separated into intentional namespaces with compatibility aliases and migration warnings where necessary;
5. public classes implement consistent Python protocols for paths, dates, iteration, context management, serialization, equality, hashing, buffers, and async cancellation;
6. package metadata, repository URLs, feature profiles, supported Python versions, and wheel matrices are correct and tested from built artifacts;
7. executable examples cover the stable operation API, low-level network primitives, events, cancellation, pipelines, persistence, daemon use, and tool-schema integration;
8. semantic API compatibility is checked against a committed baseline;
9. performance, memory, file-descriptor, thread, import-time, and wheel-size budgets are blocking release gates;
10. an exact-commit release evidence bundle proves all required profiles and records the maturity of every public domain.

## Release structure

Release 5 is divided into six detailed implementation plans:

- **Phase A — Tool-core and schema integration**: expose reusable tool request/response primitives, descriptors, schemas, validation, and operation-to-tool conversion.
- **Phase B — Registry and dispatch convergence**: replace duplicated sync/async operation dispatch with one authoritative executor registry and generated metadata.
- **Phase C — Namespace and maturity governance**: establish stable subpackages, isolate experimental domains, preserve compatibility aliases, and make maturity machine-enforced.
- **Phase D — Python ergonomics, typing, and lifecycle semantics**: normalize Python protocols, async behavior, cancellation, serialization, callbacks, and resource ownership.
- **Phase E — Packaging, documentation, and examples**: define wheel profiles, repair metadata, build executable documentation, and publish feature-aware API references.
- **Phase F — Compatibility, performance, and release closure**: add semantic API baselines, blocking budgets, multi-profile validation, evidence bundles, and final graduation review.

## Architectural invariants

### One execution model

`Engine` and `AsyncEngine` remain the canonical path for stable assessment operations. Direct convenience functions must delegate through the same request normalization, policy, scope, audit, event, timeout, cancellation, and result conversion path. A feature-gated operation may be absent from a wheel, but its descriptor and unavailability behavior must remain discoverable and deterministic.

### Stateful resources remain stateful

Not every capability belongs in `Engine.run()`. Database connections, interception proxies, packet captures, browser sessions, mobile sessions, NSE runtimes, and daemon administration should remain managed resource APIs where their lifecycle is the primary abstraction. Release 5 must classify each exposed domain as:

- canonical engine operation;
- managed subsystem API;
- utility/data API;
- experimental lab API;
- internal and intentionally unbound.

### Rust owns security-sensitive execution

Rust remains authoritative for networking, parsing, scope enforcement, authorization, rate limits, concurrency, resource cleanup, and protocol semantics. Python owns composition and integration. Framework adapters must consume Eggsec tool descriptors rather than reimplement policy or validation.

### Generated surfaces, not parallel inventories

The operation registry, capability manifest, `api_surface()`, `_capabilities.json`, `.pyi` exports, feature matrix, and generated documentation must not be maintained as independent hand-written lists. Release 5 should establish one declaration model and generate or strictly validate all derived artifacts.

### Explicit maturity

Importability does not imply stability. Every public symbol and domain must be `stable`, `provisional`, `experimental`, `deprecated`, or `internal`. Maturity changes require corresponding validation evidence and compatibility review.

## Dependency order

Phase A and Phase B may begin in parallel after a short shared inventory pass, but their merge order must be controlled:

1. establish canonical tool and operation schemas;
2. converge operation registration and dispatch;
3. generate capability and export metadata from the converged registry;
4. reorganize namespaces with compatibility aliases;
5. normalize Python protocols and typing over the final namespace shape;
6. finalize packaging, docs, examples, compatibility baselines, and release evidence.

Do not perform broad namespace moves before export generation and compatibility tests exist. Do not remove existing aliases until at least one deprecation cycle is defined.

## Cross-cutting acceptance requirements

Every stable operation must retain tests for request validation, feature discovery, scope denial, policy denial/confirmation, audit emission, sync execution, async execution, timeout, cancellation, event ordering, serialization, redaction, cleanup, direct/engine equivalence, and installed-wheel execution.

Every managed session must additionally prove:

- deterministic open/close state transitions;
- sync and async context management where both forms exist;
- idempotent close;
- cancellation during active I/O;
- bounded queues and backpressure;
- child-process, socket, thread, and file-descriptor cleanup;
- behavior after disconnect, crash, or partial failure;
- secret-safe repr, events, checkpoints, and artifacts.

Every generated schema must be deterministic, versioned, round-trippable, and guarded against accidental exposure of internal fields or secret-bearing values.

## Explicit non-goals

Release 5 does not require:

- binding TUI widgets or UI-model internals;
- exposing Tokio runtimes, task handles, channels, or daemon server internals;
- adding new offensive techniques merely to increase Python surface area;
- making experimental wireless, evasion, post-exploitation, C2, raw packet injection, or provider-dependent AI behavior stable;
- supporting arbitrary Python plugins inside Eggsec;
- framework-specific agent orchestration in the stable core;
- promising every Cargo feature in every binary wheel.

## Release completion definition

Release 5 is complete when Python is a first-class Eggsec host API rather than a flat PyO3 export layer. A Python application must be able to discover capabilities, inspect schemas, validate and invoke tools, run stable operations locally or through supported remote execution, compose workflows, receive typed events, cancel work, manage resources, persist artifacts and findings, and rely on documented compatibility and maturity guarantees.

The release must finish with a generated API inventory showing no unexplained drift among Rust registrations, Python exports, stubs, capability metadata, documentation, and built wheels.