# Eggsec Canonical Terminology and Domain Model

Status: normative companion to `plans/000-long-term-specification.md`

This document defines the language Eggsec implementation plans, protocol types, architecture documents, tests, UI labels, and operator documentation MUST use. When current code uses a term differently, the compatibility mapping in this document describes the migration target.

## 1. Naming rules

1. A scope that authorizes execution MUST be a `LoadedScope`, never a raw `Scope`, on strict surfaces.
2. A `ScopeSpec` is a transport DTO and MUST NOT be treated as an authorization decision.
3. An operation, a tool, a pipeline stage, and a runtime task are distinct objects.
4. A report data contract and a report rendering are distinct owners (`eggsec-report-model` vs `eggsec-output`).
5. Policy semantics and policy bridging are distinct owners (`eggsec-policy` vs engine `policy_bridge/`).
6. Compatibility fields MAY remain during migration but MUST be labeled as compatibility projections.
7. Terms MUST NOT be used as interchangeable shorthand when they cross enforcement, transport, or node boundaries.

## 2. Enforcement terms

### EnforcementContext

The mandatory pre-dispatch authorization gate for all surfaces. Evaluates an operation against the loaded scope, operation metadata, and surface profile. Manual (CLI/TUI) contexts use the permissive profile with operator overrides allowed; REST/MCP/agent/CI contexts use strict profiles with no overrides, failing closed.

### OperationMetadata

The single source of truth for operation policy. All authorization decisions derive from it; inline policy checks are forbidden.

### ApprovedExecution

The token + scope-snapshot bundle authorizing one strict-surface execution. Obtained only via `EnforcementContext::approve_execution()` / `approve_manual_execution()` from the same context that evaluated the request. Never constructed directly.

### ApprovedOperation

Authorization for scope-insensitive tools only. Dispatched via raw `dispatch_checked()`. MUST NOT be used as a substitute for `ApprovedExecution` on scope-sensitive paths.

### Scope / LoadedScope / ScopeSpec

- `Scope`: loaded configuration shape; never authorizes strict surfaces directly.
- `LoadedScope`: the effective engine scope participating in authorization.
- `ScopeSpec` (`eggsec-tool-core`): transport DTO with no auth methods. Converted via `eggsec::config::scope_from_spec` (fail-closed). Effective auth is engine-scope ∩ converted-spec.

### EnforcedDispatcher

The strict-surface dispatch entry point. Dispatches only `ApprovedExecution` bundles via `dispatch_execution()`.

## 3. Execution terms

### Operation

A typed engine capability (scan, fuzz, probe, …) with `OperationMetadata`, default parameters, and validation. Canonical typed requests live in the engine operation model.

### Tool / SecurityTool

The tool-abstraction integration surface (`SecurityTool` trait, `ToolRegistry`, MCP/REST/gRPC protocol adapters). Tools execute only through enforced dispatch; adapters use narrow service traits and never call `tool.execute()` directly.

### Dispatch

Canonical execution owns running approved work: `dispatch::canonical_execution::execute_approved_execution` (scope-sensitive) and `execute_approved` (scope-insensitive). CLI routes once via `commands::route::route_for_commands`.

### TaskKind

The single wire-side operation identity: `operation_id()` / `canonical_target()`. Engine helpers delegate to it; parallel operation-name tables are forbidden.

### RuntimeSurface

A wire DTO describing where a task runs. `runtime_bridge` owns both conversion directions; `Unknown` is rejected.

### Pipeline / ScanProfile / Stage

`ScanProfile` is the single source of truth for stage selection, risk budget, and profile-specific validation (`Pipeline::from_profile()` is the canonical parser-independent constructor). `Stage` enumerates assessment phases; `PipelineContext` carries session-persisted execution state.

## 4. Policy and transport terms

### Policy (eggsec-policy)

Deterministic authorization semantics only: descriptors, catalog, scope data, pure matching, decisions, approval tokens. Evaluated over explicit `EnabledFeatures` + `TargetScope` facts. No I/O, DNS, `cfg!`, transport, or frontend dependencies.

### Policy bridge (engine policy_bridge/)

Adapts engine reality (features, DNS via `HostResolver`, `NetworkAuthority` checkpoints) into policy facts. The resolver reports facts; policy decides.

### Transport (eggsec-transport)

The scope-aware outbound HTTP contract: neutral DTOs, mandatory `NetworkAuthority` checkpoints (including proxy-peer `authorize_proxy_resolved` / `authorize_proxy_socket`), TOCTOU-closed resolver binding, recording fake for tests.

### Backend (eggsec-transport-eggfetch)

The pinned production `HttpTransport` over published `eggfetch-core`: logical-URL + singular resolved-address direct routing, manual authorized redirects, H1/H2 route reuse via ALPN, pinned proxy peers/targets where enforceable, total deadline through body EOF.

## 5. Data and reporting terms

### Finding

The canonical vulnerability record (see findings workflow): scored, triaged, assigned, and SLA-tracked through the finding lifecycle.

### Report model (eggsec-report-model)

Stable report/evidence data contracts: `ScanReportData`, `ReportEnvelope`, evidence and summary DTOs. Data only.

### Output (eggsec-output)

Rendering and analysis over the report model: JSON/CSV/HTML/SARIF/JUnit/Markdown, dedup, trends, diff. Never owns data contracts.

## 6. Frontend and session terms

### Manual surface

CLI and TUI. Permissive enforcement profile; operator overrides allowed. CLI handlers live in-engine (`commands/handlers/`); the `eggsec-cli` crate is a thin shell.

### Strict surface

REST, MCP, gRPC, agent, CI. No overrides; fail closed; only `Allow` dispatches.

### Daemon

The persistent session host (`eggsec-daemon`): Unix-socket server, session lifecycle, SQLite storage. IPC types live in `eggsec-daemon-protocol` (`ClientCommand`, `ServerMessage`, `ErrorCode`, RBAC registry).

### View model (eggsec-ui-model)

Frontend-neutral view DTOs plus renderer registry consumed by the TUI. Depends only on `eggsec-runtime`.

## 7. Compatibility mapping

| Legacy / loose usage | Normative term |
|---|---|
| “scope allows target” inside an adapter | `EnforcementContext::evaluate()` decision |
| raw `Scope` passed to strict dispatch | `LoadedScope` snapshot in `ApprovedExecution` |
| operation name string match for auth | `matches_descriptor()` + scope/policy/surface checks |
| report DTO in `eggsec-output` | contract in `eggsec-report-model`, renderer in `eggsec-output` |
| policy code reading config/DNS directly | engine `policy_bridge/` fact adapter + pure `eggsec-policy` evaluation |
| second HTTP client abstraction | single `eggsec-transport` contract + `eggsec-transport-eggfetch` backend |
| ad-hoc feature check via `cfg!` in policy | explicit `EnabledFeatures` fact |
