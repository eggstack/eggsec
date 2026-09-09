# Phase D Plan: Protocol/Agent Boundary Completion and Hotspot Decomposition

## Status

Status: Executed (2026-09-09). All workstreams implemented; see Completion record below.

## Objective

Complete the partial extraction of protocol adapters and autonomous-agent execution behind explicit engine service interfaces, then split the largest security-critical implementation files into cohesive modules without changing observable behavior.

This phase addresses two related maintenance problems:

1. protocol servers and the autonomous agent still reach deeply into the `eggsec` composition root and construct or own engine internals directly;
2. several policy/runtime/daemon files are sufficiently large that security review, ownership, and regression analysis are unnecessarily difficult.

The solution is dependency inversion and module decomposition, not another mirror layer.

## Preconditions

Phase C should first provide stable canonical request/execution service boundaries. Do not extract adapters against transitional request APIs only to rewrite them immediately afterward.

## Primary areas

```text
crates/eggsec/src/tool/protocol/grpc.rs
crates/eggsec/src/tool/protocol/rest.rs
crates/eggsec/src/tool/protocol/openai/
crates/eggsec/src/tool/protocol/openresponses/
crates/eggsec/src/tool/protocol/mcp/
crates/eggsec/src/agent/
crates/eggsec-agent/
crates/eggsec/src/config/policy.rs
crates/eggsec/src/config/policy_decision.rs
crates/eggsec/src/config/scope.rs
crates/eggsec-runtime/src/runtime.rs
crates/eggsec-daemon/src/host.rs
crates/eggsec-daemon/src/http.rs
crates/eggsec-daemon/src/protocol.rs
crates/eggsec-daemon/src/server.rs
docs/architecture/api_extraction_boundary.md
docs/ARCHITECTURE.md
docs/AGENT.md
docs/DAEMON.md
```

## Non-goals

This phase does not redesign MCP, change protocol wire formats, change authorization policy, replace Axum/tonic, rewrite daemon persistence, or merge crates solely to reduce crate count. It does not split files according to arbitrary line-count thresholds without cohesive responsibility boundaries.

## Workstream 1 — Define minimal engine service interfaces

Introduce narrow interfaces consumed by adapters. The exact traits should be based on current call graphs, but likely concepts include:

```text
OperationCatalog / OperationIntrospection
OperationExecutor
PreflightService
SessionService
Artifact/Result lookup where required
AiAnalysisService (optional)
```

Rules:

- authorization remains inside the engine service path;
- adapters cannot receive a raw dispatcher that permits unchecked execution;
- traits expose canonical request/result DTOs from Phase C;
- protocol exposure metadata remains separate from authorization;
- trait objects or generics are acceptable; prefer the simplest API that preserves testability;
- do not create one enormous `EngineService` trait if smaller capability interfaces avoid unnecessary coupling.

## Workstream 2 — Extract gRPC first

The existing architecture note identifies gRPC as the cleanest adapter boundary. Use it as the pilot.

Target outcome:

- move gRPC transport/service implementation and proto build ownership into a dedicated protocol/API crate if the dependency graph remains clean;
- inject operation catalog/execution/preflight interfaces;
- keep generated protobuf types within the adapter crate;
- preserve exact wire behavior and reflection behavior;
- maintain strict `GrpcApi` enforcement semantics in engine service calls;
- add contract tests against the old/new path before deleting the old adapter.

If a new `eggsec-api` crate is created, document its dependency rule: it may depend on protocol-neutral DTO crates and engine service traits, but must not become a second composition root containing domain implementations.

## Workstream 3 — Extract REST, OpenAI, and OpenResponses adapters

These HTTP adapters share Axum and catalog/execution translation patterns.

Extract transport-specific concerns:

- routing;
- wire request/response types;
- authentication middleware that is transport-level;
- SSE/WebSocket transport plumbing;
- OpenAI/OpenResponses schema adaptation.

Keep engine-owned concerns in engine services:

- operation authorization;
- target/scope evaluation;
- feature/capability decision;
- canonical request validation;
- dispatch;
- canonical result generation.

Move TLS configuration to an appropriate shared process-host type or pass fully constructed listener/server configuration from the host. Do not move distributed-engine internals into the API crate merely to satisfy a type import.

## Workstream 4 — MCP decoupling and extraction

MCP is the hardest boundary and should be last.

First split `McpServer` responsibility internally:

- JSON-RPC/wire handling;
- MCP profile/tool visibility policy;
- session/resource bookkeeping;
- engine operation invocation;
- optional AI/prompts integration;
- streaming/event adaptation.

Inject operation execution/catalog and optional AI/session services rather than holding concrete `ToolRegistry`, `ToolDispatcher`, and `AiClient` where avoidable.

Only after this split should transport/wire modules move out of the main engine. It is acceptable for a small MCP-engine bridge to remain in `eggsec` if moving it would invert dependencies incorrectly.

Do not duplicate `EnforcementContext` logic in MCP.

## Workstream 5 — Autonomous agent dependency injection

`eggsec-agent` already owns coordination primitives, while the autonomous security agent remains in the engine and historically constructs default registry/dispatcher state.

Refactor autonomous agent construction to receive:

- operation catalog/introspection;
- approved execution/preflight service;
- scheduler/coordination primitives;
- optional AI service;
- persistence/alert channels as explicit adapters.

Target outcome:

- agent tests can use fake execution services without building the entire tool registry;
- the agent cannot bypass strict `SecurityAgent` enforcement because the injected executor exposes only checked execution;
- coordination primitives remain engine-independent;
- consider moving autonomous-agent orchestration into `eggsec-agent` only after engine dependencies are represented as traits. Do not force the move if it would create circular dependencies.

## Workstream 6 — Policy hotspot decomposition

Split `config/policy.rs` and `config/policy_decision.rs` along semantic boundaries while preserving public exports.

Suggested ownership:

```text
policy/types.rs              ExecutionSurface/Profile/Risk/Mode/Capability
policy/catalog.rs            OperationMetadata and alias lookup
policy/config.rs             deserialized ExecutionPolicy
policy/descriptor.rs         OperationDescriptor construction/validation
policy/decision.rs           evaluation outcomes/reasons
policy/approval.rs           ApprovedOperation issuance and binding checks
policy/manual_override.rs    manual override semantics
policy/tests/...             semantic tests grouped by invariant
```

Exact filenames may differ. The requirement is that approval issuance and evaluation logic become independently reviewable without changing semantics.

## Workstream 7 — Scope/runtime/daemon hotspot decomposition

Apply the same principle to:

- `config/scope.rs`: address classification/resolution, rule parsing/matching, loaded-scope provenance, evaluation;
- `eggsec-runtime/runtime.rs`: submission/lifecycle, task registry/state, events/backpressure, cancellation/cleanup;
- `eggsec-daemon/host.rs`: client authorization/RBAC, session lifecycle, persistence bridging, request handling, recovery.

Keep public facade modules and re-exports stable where feasible.

Avoid splitting purely by file size; each extracted module should have a clear invariant and minimal dependency surface.

## Workstream 8 — Tests and dependency guards

Add/retain direct contract tests proving:

- REST/MCP/gRPC cannot invoke unchecked dispatch;
- identical canonical requests through extracted adapters produce identical engine approval/dispatch behavior;
- agent strict mode cannot be downgraded through injected services;
- daemon RBAC and session ownership behavior is unchanged;
- policy approval token construction remains private/controlled;
- crate dependency direction prevents protocol crates from depending on concrete domain implementations.

Use Cargo dependency tests/architecture guards only for durable crate-direction rules.

## Acceptance criteria

- gRPC and simpler HTTP adapters are outside the engine composition root or have an equivalent clean injected boundary;
- MCP no longer directly owns more engine internals than required for a narrow bridge;
- autonomous agent execution is dependency-injected and strict by construction;
- no extracted adapter duplicates scope/policy evaluation;
- large policy/runtime/daemon files are split into cohesive modules with stable facades;
- crate dependency direction is acyclic and documented;
- protocol wire compatibility tests pass;
- daemon authorization/RBAC regression tests pass;
- `make check`, protocol feature profiles, daemon tests, and applicable Python checks pass.

## Completion record

Executed 2026-09-09.

- Baseline SHA: `e47aaaa3` (Phase C follow-up head).
- Final SHA: `8dd20331` (protocol/agent boundary + hotspot implementation).
- No new crate created (deliberate): a separate `eggsec-api` crate would
  become a second composition root (Axum/tonic closures, generated protobuf,
  `TlsConfig`) or invert dependencies (bridge needs engine policy types).
  The injected `EngineServices` boundary is the equivalent clean boundary
  per the acceptance criteria; rationale recorded in
  `architecture/api_extraction_boundary.md`.
- New engine service interfaces (WS1):
  - `crates/eggsec/src/tool/service.rs`: `OperationCatalog`
    (`StaticOperationCatalog`), `CheckedExecutor` (checked-only; blanket
    impl for `EnforcedDispatcher`), `PreflightService` (blanket impl for
    `EnforcementContext`), `EngineServices` bundle (`new` for composition
    roots, `with_executor`/`with_parts` for injection).
  - `crates/eggsec/src/tool/protocol/mcp/bridge.rs`: `McpEngineBridge`
    narrow bridge (evaluate/decision/dispatch delegate to services; no
    `EnforcementContext` duplication).
  - `crates/eggsec/src/agent/services.rs`: `AgentExecutionService`
    (checked-only, `AgentStrict` by construction; blanket impl for
    `EngineServices`; `FakeAgentExecutor` for tests).
- Adapter decoupling (WS2-4):
  - `GrpcService::with_services`, `RestState::with_services`,
    `McpServer::with_services`, `openai::router_with_services`,
    `openresponses::router_with_services`; legacy `new`/`router` remain as
    composition-root/compat shims building inert fail-closed `EngineServices`.
  - REST/gRPC/MCP approve/dispatch migrated to `services` (no behavior
    change; wire formats preserved).
  - OpenAI/OpenResponses fixed: removed `Scope::is_target_allowed` DTO check
    and direct `tool.execute` bypass; per-tool `try_descriptor_for_target` +
    `services.approve(RestApi)` + `services.dispatch_checked`, denials fail
    closed per tool with wire-compatible rendering.
  - TLS config stays as process-host input (not moved).
- Agent DI (WS5):
  - `Agent::with_engine_services(config, services, alert_router)` (no default
    registry construction; validates `AgentStrict`); `Agent::new` remains as
    composition-root shim; execution prefers `execution_services`, then
    `enforced_dispatcher`, then test-only raw dispatcher.
- Hotspot decomposition (WS6-7, stable facades via re-exports):
  - `config/policy.rs` 2453 → 1009; new `policy_target.rs` (220:
    `TargetHint`/`OperationTarget`/`normalize_*`/`TargetPolicyKind`/`DescriptorError`),
    `policy_catalog.rs` (1270: `OperationMetadata`/statics/lookups + catalog
    tests), `policy_approval.rs` (149: `ApprovedOperation` + token tests).
  - `config/policy_decision.rs` 3768 → 3696 (approval moved out).
  - `config/scope.rs` 1634 → 1430; new `scope_address.rs` (174) +
    `scope_resolver.rs` (132) with facts tests.
  - `eggsec-runtime/src/runtime.rs` 1851 → 1668; new `runtime_config.rs`
    (64) + `runtime_sink.rs` (207) with backpressure tests.
  - `eggsec-daemon/src/host.rs` 3149 → 3124; new `host_auth.rs` (140: RBAC
    role/observe helpers + tests) + `host_persistence.rs` (67: fan-out
    timeout/audit helpers + tests). Ownership behavior unchanged.
- Compatibility shims retained (with Phase G removal criterion):
  - `RestState::new`, `GrpcService::new`, `McpServer::with_enforcement`,
    `openai::router`, `openresponses::router`, `Agent::new` (all delegate to
    injected constructors); concrete `dispatcher`/`registry` fields kept
    where external field access or history bridging requires them;
    `OpenAiState.scope` retained as ignored deprecated field.
- Tests/guards (WS8):
  - New `crates/eggsec/tests/phase_d_protocol_agent.rs` (8 tests):
    REST/MCP/gRPC identical approval, empty-scope deny, binding mismatches
    fail closed, agent downgrade rejected/accept, facade equivalence, token
    binding.
  - Unit tests in `service.rs`, `bridge.rs`, `agent/services.rs`,
    `policy_approval.rs`, `scope_address.rs`, `scope_resolver.rs`,
    `runtime_config.rs`, `runtime_sink.rs`, `host_auth.rs`,
    `host_persistence.rs`.
  - New arch guards 73-78 (service traits exist; checked dispatch only; no
    DTO auth in adapters; approval construction controlled; protocol free of
    concrete impls; hotspot facades exist). Guards comment-aware (exclude
    `//` prose and `constraints.rs` profile checks).
- Docs pruning:
  - `AGENTS.md` (service/hotspot patterns + guards 73-78), `README.md`
    (service boundary note), `architecture/api_extraction_boundary.md`
    (injected boundary, resolved blockers, Phase 3-6 DONE), `runtime.md`,
    `daemon.md`, `config.md`, `dispatch.md`, `overview.md` (new modules,
    pruned stale counts), skills `eggsec-tool`/`agent`/`daemon`/`config`
    (Phase D sections), `docs/ARCHITECTURE.md`/`AGENT.md`/`DAEMON.md`.
- Verification: `make check` PASS, `make check-python` PASS (no Python
  changes; bindings untouched), `cargo check` protocol profiles
  (`rest-api`, `grpc-api`, `tool-api`) PASS, `cargo test -p eggsec-daemon`
  PASS, `cargo test -p eggsec-runtime` PASS, arch guards ALL PASSED.
  New tests: `phase_d_protocol_agent` (8 passed), plus unit tests listed
  above.