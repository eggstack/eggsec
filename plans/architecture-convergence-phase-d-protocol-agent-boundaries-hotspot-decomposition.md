# Phase D Plan: Protocol/Agent Boundary Completion and Hotspot Decomposition

## Status

Status: Ready for implementation.

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

Record baseline/final SHA, modules/crates moved, trait/service interfaces introduced, largest-file before/after sizes, compatibility shims retained, and verification results.