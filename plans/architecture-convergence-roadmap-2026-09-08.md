# Architecture Convergence and Capability Maturity Roadmap

## Status

Status: Ready for implementation.

## Baseline

Plan against `main` beginning from:

```text
ae7c7d441ad906f4b444dfdf8d6f532279bf8424
```

If implementation begins from a later head, record the actual starting SHA and reconfirm each residual before changing code.

## Purpose

The previous dependency/architecture simplification roadmap is complete and must remain preserved as historical engineering record. This roadmap is a corrective convergence pass over residual architecture and maturity gaps visible in the current repository.

The repository now has strong crate boundaries, centralized authorization, extensive verification, and substantial domain coverage. The remaining problem is not lack of capability. It is that several migrations stopped at compatibility layers: new canonical abstractions coexist with older dispatch/request/scope representations, while a few user-facing APIs expose mature type surfaces without equivalent execution parity.

The objective of this roadmap is to reduce the number of independently maintained representations of authorization scope, operations, feature/build profiles, runtime requests, protocol adapters, and programmable session behavior. It must improve maintainability and completeness without weakening policy, removing capability, or initiating another broad rewrite.

## Confirmed residual problem classes

### 1. Dual scope semantics

`eggsec-tool-core::request::Scope` is a protocol-neutral DTO with glob-style `is_allowed()` semantics and a permissive default. `eggsec::config::Scope` is the authoritative security boundary and implements explicit scope provenance, target rules, CIDR and port policy, DNS/address-set evaluation, non-public address handling, exclusions, and rate limits.

Two public types named `Scope` that answer an authorization-looking question differently are unacceptable long-term in a scope-enforced security engine. Tool DTOs may describe requested scope, but only the policy engine may decide authorization.

### 2. Feature/build contract drift

The feature registry is intended to be authoritative, but Cargo manifests, documentation, aggregate features, and representative verification profiles are not fully aligned. Current examples include the documented default feature set differing from the actual main crate default and the `full` aggregate not enabling every declared feature.

The verification contract intentionally uses representative feature profiles rather than combinatorial all-feature coverage. That remains correct, but each declared feature should compile in at least one direct profile and aggregate feature semantics must be explicit rather than implied.

### 3. Incomplete operation/dispatch migration

`OperationMetadata` is canonical for policy metadata, but the command registry still distinguishes a four-command `RegistryBacked` pilot from a much larger `LegacyWrapped` set. Runtime request DTOs separately mirror operation kinds and parameters. Protocol/tool registration and Python schemas add additional adapter representations.

The repository therefore still pays synchronization cost when operation parameters, aliases, visibility, or dispatch behavior change.

### 4. Protocol and autonomous-agent extraction remains partial

Protocol adapters remain in the main `eggsec` composition crate. Existing architecture notes identify gRPC as the cleanest extraction candidate, REST/OpenAI/OpenResponses as moderate candidates, and MCP as deeply coupled to registry/session/AI/policy state. Tool-agent coordination has been extracted to `eggsec-agent`, while the autonomous security agent remains engine-coupled.

This boundary should be completed through dependency injection and adapter traits, not through another compatibility mirror.

### 5. Security-critical maintenance hotspots

Several policy/runtime/daemon files have grown large enough that reviewability is itself a maintenance risk. Refactoring should split responsibilities behind stable public APIs while preserving behavior and direct semantic tests.

### 6. Programmability parity gaps

Python browser-session APIs expose extensive lifecycle/event/storage/screenshot types, but the managed browser session is not backed by a real engine. Daemon programmability remains provisional until request/result/event/cancellation/reconnect/artifact parity is closed. Proxy bindings similarly lag the Rust proxy domain in exchange/result semantics.

These areas have already paid most of the schema/API cost; execution parity is higher-value than adding new domains.

### 7. Platform integration maturity gaps

Mobile dynamic, packet inspection, wireless, and related platform-sensitive domains are constrained more by reproducible integration environments and lifecycle coverage than by missing types. The correct next investment is deterministic fixtures/runners, not broader attack primitives.

## Non-negotiable invariants

This roadmap must preserve all of the following:

- `EnforcementContext` remains the sole authorization decision owner for engine execution;
- strict automated surfaces remain fail-closed and cannot use manual overrides;
- no protocol-neutral DTO may independently authorize network execution;
- `ApprovedOperation` or its direct successor remains bound to the exact approved operation/target contract;
- CLI, TUI, daemon, REST, MCP, gRPC, agent, CI, and Python surfaces retain existing safety posture;
- domain crates declare requirements and execute capability but do not decide authorization;
- current user-visible commands and stable Python operations remain available unless a separately documented compatibility decision is made;
- package publication remains manual;
- routine CI remains proportionate and Linux-first; this roadmap must not recreate an exhaustive combinatorial CI matrix;
- no new hazardous capability is added merely to justify platform test infrastructure;
- historical plans remain untouched except for status/index links where required.

## Phase sequence

### Phase A — Scope contract unification

Plan: [`architecture-convergence-phase-a-scope-contract-unification.md`](architecture-convergence-phase-a-scope-contract-unification.md)

Remove authorization-like semantics from the protocol-neutral tool scope representation. Establish a single declarative scope contract and an explicit conversion boundary into the authoritative engine policy model. Rename types where necessary so callers cannot confuse a request filter/specification with authorization state.

Exit condition: there is exactly one implementation that answers whether a target is authorized for execution.

### Phase B — Feature/build/verification reconciliation

Plan: [`architecture-convergence-phase-b-feature-build-verification-reconciliation.md`](architecture-convergence-phase-b-feature-build-verification-reconciliation.md)

Make Cargo feature declarations, the runtime feature registry, aggregate profile semantics, documentation, and direct compilation checks agree. Add a low-frequency per-feature compile sweep without turning routine CI into a combinatorial matrix.

Exit condition: every declared feature is known to the runtime registry and directly compiled in at least one maintained verification profile; documentation is generated or mechanically validated from the same declarations.

### Phase C — Operation, dispatch, and runtime request convergence

Plan: [`architecture-convergence-phase-c-operation-dispatch-runtime-convergence.md`](architecture-convergence-phase-c-operation-dispatch-runtime-convergence.md)

Finish the command-registry migration, retire `LegacyWrapped` as a permanent architecture state, and introduce canonical operation request types/adapters so runtime, CLI, TUI, protocol, and Python surfaces do not hand-maintain equivalent parameter schemas.

Exit condition: operation identity, policy metadata, request normalization, and dispatch routing have one canonical ownership path; frontend adapters only translate into it.

### Phase D — Protocol/agent boundary completion and hotspot decomposition

Plan: [`architecture-convergence-phase-d-protocol-agent-boundaries-hotspot-decomposition.md`](architecture-convergence-phase-d-protocol-agent-boundaries-hotspot-decomposition.md)

Extract protocol transport/adapters behind engine service traits in increasing coupling order, complete autonomous-agent dependency injection, and split the largest policy/runtime/daemon implementation files by responsibility without changing public behavior.

Exit condition: protocol servers and the autonomous agent depend on explicit engine interfaces rather than constructing engine internals directly, and security-critical files are reviewable modules with narrow responsibilities.

### Phase E — Programmability parity: browser, daemon, and proxy

Plan: [`architecture-convergence-phase-e-programmability-parity-browser-daemon-proxy.md`](architecture-convergence-phase-e-programmability-parity-browser-daemon-proxy.md)

Wire the Python browser session to a real browser backend, close daemon local/remote behavior parity, and make proxy binding exchange/results/events reflect the actual Rust domain implementation.

Exit condition: provisional status is retained only for clearly documented platform/hazard constraints, not because major API methods are placeholders.

### Phase F — Platform integration maturity

Plan: [`architecture-convergence-phase-f-platform-integration-maturity.md`](architecture-convergence-phase-f-platform-integration-maturity.md)

Build reproducible Android/browser/privileged-network integration fixtures and lifecycle tests for platform-sensitive domains. Keep them scheduled/manual or environment-gated as appropriate.

Exit condition: mobile-dynamic, packet-inspection, and selected wireless behaviors have deterministic integration evidence and bounded skip budgets without increasing hazardous capability.

### Phase G — Closure, measurement, and documentation reconciliation

Plan: [`architecture-convergence-phase-g-closure-measurement-documentation.md`](architecture-convergence-phase-g-closure-measurement-documentation.md)

Reconcile architecture, feature, command, Python maturity, and verification documentation; record dependency/test/complexity deltas; remove temporary migration shims; and close this roadmap only after the final implementation head is validated.

Exit condition: no active compatibility migration introduced by this roadmap remains undocumented or open-ended.

## Ordering rules

Phases A and B are foundational and may proceed in parallel only where they do not touch the same shared contract files. Phase C depends on both because operation request types need unambiguous scope and feature semantics. Phase D should follow the canonical operation/service interfaces from Phase C. Phase E depends on those interfaces for daemon/proxy/browser programmability parity. Phase F may research environments in parallel but should land after core request/event contracts stabilize. Phase G is strictly last.

Each phase should land as reviewable commits. Compatibility shims introduced in one phase must include a planned removal point no later than the next phase unless they are intentionally retained as public semver facades.

## Deliberate exclusions

This roadmap does not authorize:

- adding additional C2, post-exploitation, evasion, stress, or wireless attack primitives;
- changing scope policy to make adapters easier to implement;
- replacing Tokio, Axum, tonic, reqwest, PyO3, SQLx, Ratatui, Rustls, or other mature dependencies without a concrete defect;
- creating a procedural-macro framework solely for code generation if ordinary Rust types/macros/build scripts are sufficient;
- making `full` the default build;
- requiring `--all-features` to succeed if explicitly incompatible platform/backend combinations exist;
- adding permanent synchronization tests between duplicate representations when one representation can be deleted;
- broad public API churn solely for source-layout aesthetics;
- hosted publication or release automation.

## Roadmap acceptance criteria

The roadmap is complete only when all of the following are true:

1. protocol/tool scope DTOs cannot independently authorize execution;
2. one engine policy implementation determines target authorization;
3. all public scope conversions are explicit and tested for conservative behavior;
4. Cargo feature defaults and aggregate semantics are documented from current manifests;
5. every declared feature appears in the runtime feature registry;
6. every declared feature compiles in at least one maintained direct profile, subject only to documented platform/toolchain prerequisites;
7. `full` has an explicitly defined meaning that is mechanically checked;
8. operation-backed CLI commands no longer depend on a permanent `LegacyWrapped` dispatch mode;
9. operation parameter normalization is shared across runtime/programmatic/manual surfaces rather than independently reconstructed;
10. runtime `TaskKind` mapping is derived from or validated directly against canonical operation request contracts;
11. protocol adapters use injected engine service interfaces;
12. the autonomous agent does not construct a default registry/dispatcher internally when an injected service is available;
13. major policy/runtime/daemon hotspots are split into cohesive modules without weakening semantic tests;
14. Python managed browser sessions perform real backend operations for all advertised supported capabilities;
15. daemon result retrieval, event ordering/replay, cancellation, timeout, reconnect, and artifact behavior have explicit parity tests;
16. proxy Python bindings return real exchange/result data for supported workflows;
17. platform-sensitive integration suites run in reproducible documented environments with bounded skips;
18. workspace linting covers extracted implementation crates at an appropriate cadence;
19. documentation no longer claims feature/default/command/maturity states contradicted by source;
20. all temporary migration shims introduced by this roadmap are removed or explicitly retained as stable compatibility facades;
21. `make check` and applicable Python checks pass on the final implementation head;
22. optional deep feature/platform checks pass or have a concrete documented blocker;
23. manual publication/release policy is unchanged.

## Handoff guidance

Prefer deleting duplicate policy/request representations over creating more comparison tests. Use conversion adapters only at true architectural boundaries. Security-sensitive behavior should be tested semantically, not through source-text assertions alone.

When a domain is provisional because the execution backend is absent, implement the backend before expanding the public type surface. When a domain is provisional because it is platform-sensitive, invest in a reproducible test fixture before adding features.

Every phase implementation should record the actual baseline SHA, final SHA, commands run, skipped/blocked checks, and any intentionally retained compatibility layer in its completion section.