# Eggsec Architecture

Policy-mediated security assessment engine with multiple frontends, centralized enforcement, and domain execution crates.

## 1. System Overview

Eggsec is a Rust-native, scope-enforced security assessment and defense-validation engine. It supports:

- **Manual operator workflows** (CLI, TUI) where humans decide which tests to run
- **Automated workflows** (REST, MCP, gRPC, Agent, CI) where policy must be enforced without operator discretion

The architecture enforces a critical invariant: **authorization is centralized; domain crates declare and execute but must not authorize**. `EnforcementContext::evaluate()` is the mandatory pre-dispatch gate for **all** surfaces. Strict programmatic surfaces (REST, MCP, gRPC, Agent, CI) honor no overrides and fail closed — they dispatch only through `EnforcedDispatcher::dispatch_execution()` with an `ApprovedExecution` bundle (token plus the scope snapshot from the same context). Manual surfaces (CLI, TUI) use the permissive profile with operator overrides allowed. `dispatch_checked()` with `ApprovedOperation` alone remains for scope-insensitive tools. Adapters go through narrow service traits (`tool::service::EngineServices`, `agent::services::AgentExecutionService`, `mcp::bridge::McpEngineBridge`) and never call `Scope::is_target_allowed` or `tool.execute()` directly.

```
┌──────────────────────────────────────────────────────┐
│                   User Interfaces                     │
│   CLI    TUI    REST API    MCP    gRPC    Agent      │
└────────────────────────┬─────────────────────────────┘
                         │
              ┌──────────▼──────────┐
              │  Command Dispatch    │
              │  (handlers/)         │
              └──────────┬──────────┘
                         │
         ┌───────────────▼───────────────┐
         │   EnforcementContext::evaluate │
         │   → ApprovedExecution bundle   │
         └───────────────┬───────────────┘
                         │
    ┌────────────────────▼────────────────────┐
    │         Security Modules                 │
    │  Scanner  Fuzzer  WAF  Recon  Loadtest  │
    │  Auth  Stress  Packet  Pipeline  Proxy  │
    │  Evasion  Postex  C2  Browser  Mobile   │
    └────────────────────┬────────────────────┘
                         │
    ┌────────────────────▼────────────────────┐
    │         Infrastructure Layer             │
    │  Config  Output  Distributed  Storage   │
    └──────────────────────────────────────────┘
```

The **command registry** (`commands/registry.rs`) provides static, inspectable metadata for CLI/TUI dispatch: 49 entries. `commands/route.rs` is the single routing owner that classifies every Clap variant once. All operation-backed commands use `RegistryBacked` dispatch (`OperationMetadata` → descriptor → `EnforcementContext` → canonical dispatcher); there is no permanent `LegacyWrapped` mode. The registry is metadata and routing, not authorization — `EnforcementContext::evaluate()` remains the mandatory pre-dispatch gate. See [COMMAND_REGISTRY.md](COMMAND_REGISTRY.md) for the full inventory and [architecture/cli_commands.md](../architecture/cli_commands.md) for the routing contract.

## 2. Workspace Crate Ownership

23 workspace crates (`eggsec-nse` is external and not a member).

| Crate | Role | Policy Decisions | Execution | Frontend | Dependency-Light | Notes |
|-------|------|:---:|:---:|:---:|:---:|-------|
| `eggsec-core` | Shared primitives | No | No | No | Yes | `Severity`, `SensitiveString`, constants. Zero internal deps. |
| `eggsec-tool-core` | Protocol-neutral DTOs | No | No | No | Yes | `ToolRequest`, `ToolResponse`, `ToolError`, history types. |
| `eggsec-output` | Report rendering | No | Output adapters | No | Yes | JSON/CSV/HTML/SARIF/JUnit/Markdown over `eggsec-report-model`. Portable adapters. |
| `eggsec-report-model` | Report data contracts | No | No | No | Yes | `ScanReportData`, `ReportEnvelope`, evidence/summary DTOs. Data only; zero workspace deps except `eggsec-core`. |
| `eggsec-agent` | Agent coordination | No | Coordination only | No | Yes | Registry, scheduler, lifecycle, cron. Depends only on `eggsec-core`. |
| `eggsec` | Composition root | **Yes** | All domains | No | No | Central policy, orchestration, all security modules. |
| `eggsec-cli` | Binary entrypoint | No | No | **Yes** | Yes | Thin wrapper: depends on `eggsec` + `eggsec-tui`. |
| `eggsec-tui` | TUI frontend | No | No | **Yes** | No | 33 tabs, enforcement toggle, packaged themes. |
| `eggsec-nse` (external) | NSE compatibility runtime | No | Domain execution | No | Yes | [Standalone repository](https://github.com/eggstack/eggsec-nse), consumed by Eggsec as the versioned `eggsec-nse 0.2.0` crates.io release behind the optional engine `nse` feature. |
| `eggsec-db-lab` | DB pentest domain | No | Domain execution | No | Yes | Postgres/MySQL/MSSQL/MongoDB/Redis checks. |
| `eggsec-web-proxy` | Web proxy domain | No | Domain execution | No | Yes | MITM intercept, TLS, protocol handlers. |
| `eggsec-mobile-lab` | Mobile analysis domain | No | Domain execution | No | Yes | APK/IPA static analysis + Android dynamic runtime testing (ADB, Frida, behavioral correlation). |
| `eggsec-service-db` | Knowledge corpus leaf | No | No | No | Yes | Port-to-service tables + banner heuristics. Data only. |
| `eggsec-secrets` | Knowledge corpus leaf | No | No | No | Yes | Credential patterns + the entropy gate. Data only. |
| `eggsec-payloads` | Knowledge corpus leaf | No | No | No | Yes | Attack payload corpora. Data only; the 6 live probers stay engine-side. |
| `eggsec-udp-scan` | Knowledge corpus leaf | No | No | No | Yes | UDP scanning + ICMP error correlation, behind the `udp-scan` feature. |
| `eggsec-runtime` | Frontend-neutral runtime | No | No | No | Yes | Task lifecycle management (`Runtime`, `RuntimeConfig`, `RuntimeTaskExecutor`) for daemon architecture. |
| `eggsec-daemon` | Persistent daemon host | No | No | **Yes** | Yes | Unix socket server, session lifecycle, RBAC, persistence. Default deps are `eggsec-runtime` + `eggsec-daemon-protocol` only; engine behind `full-executor`, transport behind `http-api`, no TUI deps. |
| `eggsec-daemon-protocol` | Daemon IPC types | No | No | No | Yes | Wire DTOs for the daemon protocol. |
| `eggsec-policy` | Deterministic authorization semantics | **Yes** | No | No | Yes | Data + pure algorithms only: no Tokio, no network, no filesystem. Engine bridges DNS, features, and transport. |
| `eggsec-ui-model` | Frontend view DTOs | No | No | No | Yes | View model types for TUI/daemon event rendering. |
| `eggsec-transport` | Scope-aware outbound HTTP contract | No | No | No | Yes | Neutral DTOs, mandatory `NetworkAuthority`, recording fake. No concrete clients. |
| `eggsec-transport-eggfetch` | `HttpTransport` over `eggfetch-core` | No | No | No | Yes | Approved-IP pinning, manual authorized redirects. No production consumers yet. |
| `eggsec-python` | Python bindings | No | No | No | Yes | PyO3/maturin. Depends on `eggsec` + `eggsec-core`. Engine/AsyncEngine entry points, scope enforcement, OperationRegistry, EnforcementContext, event protocol, callbacks/sinks, domain registry, 1.0 readiness. |

**Dependency direction**: Leaf crates (`eggsec-core`, `eggsec-report-model`, `eggsec-output`, `eggsec-agent`, `eggsec-policy`) have no engine/runtime dependencies (`eggsec-report-model` depends only on `eggsec-core`; `eggsec-output` renders over the model, never the reverse). The main `eggsec` crate is the composition root. `eggsec-cli` and `eggsec-tui` are the only frontends. The workspace root is a virtual manifest — install with `cargo install --path crates/eggsec-cli`.

The knowledge-corpus leaves (`eggsec-service-db`, `eggsec-secrets`, `eggsec-payloads`, `eggsec-udp-scan`) are near-empty-dependency data crates reached only through engine `pub use` facades. The seam is one-directional: **the corpus owns data, the engine owns I/O** — the live probers stay engine-side even though their `get_payloads()` are pure data. See `architecture/knowledge_corpus.md`.

## 3. Enforcement Model

### 3.1 Core Types

| Type | Canonical Location | Purpose |
|------|--------------------|---------|
| `ExecutionSurface` | `eggsec-policy/src/policy.rs` | Caller origin identity. 9 variants: `CliManual`, `TuiManual`, `CliManualStrict`, `TuiManualStrict`, `McpServer`, `SecurityAgent`, `Ci`, `RestApi`, `GrpcApi`. |
| `ExecutionProfile` | `eggsec-policy/src/policy.rs` | Trust boundary. 5 variants: `ManualPermissive`, `ManualGuarded`, `CiStrict`, `McpStrict`, `AgentStrict`. |
| `OperationRisk` | `eggsec-policy/src/policy.rs` | Risk tier ordering. 15 variants from `Passive` to `AgentAutonomous`. |
| `OperationMode` | `eggsec-policy/src/policy.rs` | Semantic mode: `StandardAssessment`, `DefenseLab`, `HazardousLab`. |
| `Capability` | `eggsec-policy/src/policy.rs` | Fine-grained capability declarations. 19 variants. |
| `IntendedUse` | `eggsec-policy/src/policy.rs` | Declared purpose. 8 variants. |
| `OperationDescriptor` | `eggsec-policy/src/policy.rs` | The unit of policy evaluation. Bundles operation name, mode, risk, target, required features, capabilities, and scope requirements. |
| `OperationMetadata` | `eggsec-policy/src/catalog.rs` | Static registry entry. 34 operations + 42 aliases. Single source of truth for all surfaces. |
| `ExecutionPolicy` | `eggsec-policy/src/policy.rs` | TOML-deserialized config controlling which risk tiers and capabilities are allowed. |
| `LoadedScope` | `eggsec-policy/src/scope.rs` | Scope with provenance. `is_explicit_manifest()` distinguishes "no scope" from "explicitly empty scope". |
| `DenialClass` / `ConfirmationClass` | `eggsec-policy/src/decision.rs` | Denial/confirmation taxonomy. 8 variants each. |
| `EnforcementContext` | `eggsec-policy/src/decision.rs` | Bundles `ExecutionProfile` + `ExecutionPolicy` + `LoadedScope` + explicit `EnabledFeatures`. Created once per execution path. `evaluate()` is the mandatory pre-dispatch gate for all surfaces. |
| `EnforcementOutcome` | `eggsec-policy/src/decision.rs` | Profile-aware result: `Allow`, `Warn`, `RequireConfirmation`, `Deny`. |
| `ManualOverride` | `eggsec-policy/src/decision.rs` | CLI/TUI override flags. `--yes` is narrow (only `OutOfScope`/`TargetExpansion`). |
| `ApprovedOperation` | `eggsec-policy/src/approval.rs` | Proof-of-enforcement token. Private fields. Created via `EnforcementContext::approve()` / `approve_manual()`. |
| `ApprovedExecution` | `eggsec-policy/src/approval.rs` | Token **plus** the scope snapshot from the same enforcement context. Created via `approve_execution()` / `approve_manual_execution()`. Required for strict-surface dispatch. |
| `EnforcedDispatcher` | `tool/dispatcher.rs` | Wraps `ToolDispatcher`. `dispatch_execution()` requires an `ApprovedExecution`; `dispatch_checked()` requires an `ApprovedOperation`. Type-level enforcement gate. |
| `EngineServices` | `tool/service.rs` | Injected adapter boundary (`OperationCatalog`, `CheckedExecutor` = checked-only dispatch, `PreflightService`); composition roots build via `new`, adapters via `with_services` |
| `McpEngineBridge` | `tool/protocol/mcp/bridge.rs` | Narrow MCP bridge (wire/profile/session stay adapter-owned) |
| `AgentExecutionService` | `agent/services.rs` | Checked-only agent execution (`AgentStrict` by construction); `Agent::with_engine_services` injects |
| `RuntimeBridgeError` | `runtime_bridge/surface.rs` | Bridge error type: `UnknownSurface`, `UnsupportedTaskKind`, `MissingTarget`, `UnknownOperationId`, `InvalidTarget`, `ManualOverrideRejected`, `EnforcementDenied`. |
| `EggsecRuntimeExecutor` | `runtime_bridge/executor.rs` | Real daemon executor behind the `full-executor` feature. Receives `RuntimeExecutionContext`, resolves scope, obtains an `ApprovedRunRequest`, dispatches. |
| `runtime_surface_to_execution_surface()` | `runtime_bridge/surface.rs` | Converts `RuntimeSurface` (daemon DTO) → `ExecutionSurface` (engine type). |
| `descriptor_for_run_request()` | `runtime_bridge/descriptor.rs` | Converts `RunRequest` + `TaskKind` → `OperationDescriptor` via `operation_metadata()`. |
| `preflight_run_request()` | `runtime_bridge/manual.rs` | Pre-dispatch policy preview for daemon operations. Returns `PreflightResult`. |
| `approve_run_request()` | `runtime_bridge/manual.rs` | Pre-dispatch authorization for daemon operations. Returns `ApprovedOperation` or error. |
| `approve_run_request_execution()` | `runtime_bridge/manual.rs` | Same, but returns `ApprovedExecution` (token + scope snapshot). Used by the bundle path. |
| `approve_run_request_bundle()` | `runtime_bridge/bundle.rs` | Wraps `approve_run_request_execution()` into an `ApprovedRunRequest` bundle coupling token + request. |

### 3.2 Surface-to-Profile Mapping

| ExecutionSurface | ExecutionProfile | honors_manual_override | is_automated |
|-----------------|-----------------|:---:|:---:|
| `CliManual` | `ManualPermissive` | Yes | No |
| `TuiManual` | `ManualPermissive` | Yes | No |
| `CliManualStrict` | `ManualGuarded` | No | No |
| `TuiManualStrict` | `ManualGuarded` | No | No |
| `McpServer` | `McpStrict` | No | Yes |
| `SecurityAgent` | `AgentStrict` | No | Yes |
| `Ci` | `CiStrict` | No | Yes |
| `RestApi` | `McpStrict` | No | Yes |
| `GrpcApi` | `McpStrict` | No | Yes |

### 3.3 Authorization Flow

```
1. CALLER identifies execution surface

2. CONTEXT CREATION
   EnforcementContext::for_surface(surface, policy, loaded_scope)

3. OPERATION METADATA LOOKUP
   metadata_for_tool_id(tool_id) -> OperationMetadata
   metadata.descriptor_for_target(target) -> OperationDescriptor

4. ENFORCEMENT EVALUATION
   enforcement.evaluate(&descriptor) -> EnforcementOutcome

5. APPROVAL (type-level gate)
   Strict:  enforcement.approve(surface, descriptor) -> ApprovedOperation
   Manual:  enforcement.approve_manual(surface, descriptor, override) -> ApprovedOperation

6. DISPATCH (type-level enforcement)
   EnforcedDispatcher::dispatch_checked(approved, request)
   -> Verifies tool name match (alias-aware)
   -> Verifies target match
   -> Delegates to ToolDispatcher::dispatch()
```

### 3.4 Profile Behavior

`ManualPermissive` downgrades only *safe* scope-selection denials to `Warn`;
explicit allowlist misses, exclusions, high-risk operations, and non-baseline
capabilities become `RequireConfirmation`. `ManualGuarded` and every strict
profile treat `RequireConfirmation` as a hard `Deny` (no override path).
Missing features, invalid targets, denied capabilities, and compile-time
unavailability are hard `Deny` at every profile.

| Profile | Safe Scope Miss | Explicit Allowlist Miss | RequireConfirmation | Warn |
|---------|-----------------|-------------------------|---------------------|------|
| `ManualPermissive` | Downgrade to Warn | RequireConfirmation | Operator override | Proceed |
| `ManualGuarded` | Deny | Deny | Deny | Allow |
| `McpStrict` | Deny | Deny | Deny | Deny |
| `AgentStrict` | Deny | Deny | Deny | Deny |
| `CiStrict` | Deny | Deny | Deny | Deny |

Strict surfaces dispatch only on `Allow`; `Warn` and `RequireConfirmation` are
denied.

### 3.5 ManualOverride Semantics

| Flag | Permits |
|------|---------|
| `--yes` (`assume_yes`) | `OutOfScope`, `TargetExpansion` ONLY |
| `--allow-out-of-scope` | `OutOfScope`, `TargetExpansion` |
| `--allow-explicit-exclusion` | `ExplicitExclusion` |
| `--allow-high-risk` | `HighRisk` |
| `--allow-db-pentest` | `HighRisk` (alias) |
| `--allow-web-proxy` | `TrafficInterception` |
| `--allow-nonbaseline-capability` | `NonBaselineCapability` |
| `--allow-private-resolution` | `PrivateResolution` |
| `--allow-cross-host-redirect` | `CrossHostRedirect` |

Strict profiles (MCP, Agent, CI, REST, gRPC) never honor manual overrides.

## 4. Frontend Execution Flows

### 4.1 CLI (Manual)

```
Cli::parse() → resolve_execution_surface() → CommandContext::new()
  → load config/scope → build EnforcementContext
  → attach ManualOverride from CLI flags
  → handle_command() → route_for_commands() classify once (canonical IDs)
  → handler builds OperationDescriptor → ctx.evaluate_and_enforce_operation(descriptor)
  → on approval: canonical request → execute_approved() → outcome → CLI rendering
```

**Surface**: `CliManual` (default) or `CliManualStrict` (with `--strict-scope`).
**Profile**: `ManualPermissive` or `ManualGuarded`.
**Overrides**: Supported via narrow CLI flags.

### 4.2 TUI

```
App::run() → TuiEnforcementState::new(TuiManual, scope, enforcement)
  → user presses Enter on tab
  → handle_enter() → build_current_operation_descriptor()
  → try_approve(desc) → enforcement.evaluate() → approve_manual()
  → cache ApprovedOperation via set_cached_approval (exact descriptor +
     scope/policy/surface/override generation; see matches_descriptor)
  → evaluate_policy_and_dispatch() → spawn_task()
```

**Surface**: `TuiManual` (default), toggle to `TuiManualStrict` via Ctrl+G.
**Profile**: `ManualPermissive` → `ManualGuarded` on toggle.
**Overrides**: Supported via confirmation overlay.
**Cache binding (Phase 0.1)**: reuse requires exact `matches_descriptor()` plus unchanged generation; stale tokens are discarded for fresh evaluation; `validate_request_binding` remains the final gate.

### 4.3 REST API

```
HTTP POST /api/v1/tools/{tool_id}/execute
  → handle_serve() constructs EnforcementContext(RestApi)
  → validate target/payload → build OperationDescriptor
  → check rest_exposable → enforcement.approve(RestApi, descriptor)
  → only Allow proceeds → dispatcher.dispatch_checked(approved, request)
```

**Surface**: `RestApi` → `McpStrict`. Always strict. No overrides.
**Scope**: `serve --scope-file` or the global `--scope`. Always sets `requires_explicit_scope = true`. Note `mcp-serve` has **no** `--scope-file`; use the global `--scope`.
**Preflight**: `POST /api/v1/tools/{tool_id}/preflight` endpoint.

### 4.4 MCP Server

```
JSON-RPC tools/call → handle_tools_call()
  → rate limit → resolve tool → profile validation
  → build OperationDescriptor → enforcement.approve(McpServer, descriptor)
  → only Allow proceeds → dispatcher.dispatch_checked(approved, request)
```

**Surface**: `McpServer` → `McpStrict`. Always strict. No overrides.
**Profile filtering**: `McpProfilePolicy` controls tool visibility per profile (OpsAgent vs CodingAgent).
**Preflight**: `eggsec_preflight` MCP tool.

### 4.5 gRPC API

```
gRPC ExecuteToolRequest → execute_tool()
  → build OperationDescriptor → check grpc_exposable
  → enforcement.approve(GrpcApi, descriptor)
  → only Allow proceeds → dispatcher.dispatch_checked(approved, request)
```

**Surface**: `GrpcApi` → `McpStrict`. Always strict. No overrides.

### 4.6 Agent

```
eggsec agent run --scope scope.toml
  → validate explicit scope manifest
  → EnforcementContext::agent_strict(policy, loaded_scope)
  → Agent::new(config) validates AgentStrict profile
  → agent loop → execute_scan()
  → enforcement.approve(SecurityAgent, descriptor)
  → enforced_dispatcher.dispatch_checked(approved, request)
```

**Surface**: `SecurityAgent` → `AgentStrict`. Always strict. No overrides.
**Invariant**: Agent rejects non-`AgentStrict` profiles. If `enforced_dispatcher` is present but `ApprovedOperation` missing at dispatch, returns hard error.

### 4.7 CI

```
cat findings.json | eggsec ci --baseline baseline.json
  → handle_ci() reads from stdin
  → compares against baselines
  → outputs diff report
```

**No dispatch path**. CI is a passive quality gate that processes pre-existing findings. No enforcement, no tool execution.

### 4.8 Daemon / Runtime (runtime_bridge)

```
DaemonClient → Runtime::submit(RunRequest)
  → Runtime::submit() builds RuntimeExecutionContext from RuntimeSession state
  → RuntimeTaskExecutor::execute(context, request, sink, cancel)
  → EggsecRuntimeExecutor:
      → runtime_surface_to_execution_surface(context.surface) → ExecutionSurface
      → resolve_loaded_scope(context.scope) → LoadedScope (or fail closed)
      → approve_run_request_bundle(surface, policy, loaded_scope, request, override)
        → same surface/descriptor conversion
        → manual surfaces → enforcement.approve_manual(surface, descriptor, override)
        → strict surfaces → enforcement.approve(surface, descriptor)
        → ApprovedRunRequest bundle (couples token + request)
      → dispatch_approved_runtime_request(bundle, progress_tx)
        → validates descriptor match (exact matches_descriptor)
        → CanonicalOperationRequest::from_task_kind() → execute_approved()
        → (same single executor owner as embedded TUI) → TaskResult
      → dispatch::task_result_envelope() → TaskOutcome::Result(envelope)
      (single engine-owned mapping; cancellation via shared race_with_cancel)
```

**Surface**: Derived from `RuntimeExecutionContext.surface` (session-bound) → `ExecutionSurface` (engine type). Request surface is normalized to session surface before storage/emission — `RunRequest.surface` is informational only and must not influence enforcement. `Unknown` maps to error.
**Scope**: Resolved from `RuntimeExecutionContext.scope`. Strict surfaces fail closed without explicit scope. Permissive manual surfaces allow `LoadedScope::default_empty()`.
**Bridge**: `runtime_bridge/` module converts `eggsec-runtime` DTOs to engine enforcement types. Dependency direction: `eggsec` → `eggsec-runtime` (not reverse).
**Invariant**: All daemon-dispatched operations must pass through `approve_run_request_bundle()` before execution. The `ApprovedRunRequest` bundle couples the approval token with the specific request, preventing token reuse.
**Close-session behavior**: Closing a session persists a final snapshot (with `closed=true` and cancelled tasks) but does **not** delete the session. History is preserved and accessible via `daemon history` / `daemon show`.

**Real executor**: `EggsecRuntimeExecutor` (feature-gated behind `full-executor`) implements `RuntimeTaskExecutor` by receiving `RuntimeExecutionContext` from the runtime, resolving scope, obtaining an `ApprovedRunRequest` bundle, then dispatching. The daemon depends on `eggsec` only when this feature is enabled. Without it, `NoopExecutorStub` rejects all tasks.

## 5. Side-Effecting Execution Path Inventory

### 5.1 CLI Command Handlers

Operation IDs are the canonical `ALL_OPERATION_METADATA` IDs — not CLI
subcommand names. Where a subcommand is an alias or a multiplexer branch, both
spellings are shown. Risk comes from `OperationRisk`; the feature gate from
`required_features`. See [COMMAND_REGISTRY.md](COMMAND_REGISTRY.md) for the
`command_id` ↔ `operation_id` mapping and `architecture/knowledge_corpus.md`
for the corpus/engine split.

| CLI Subcommand | Handler File | Operation ID | Risk | Feature Gate | Extra Runtime Gate |
|----------------|-------------|-------------|------|-------------|-------------------|
| `scan-ports` | `scan.rs` | `scan-ports` | SafeActive | — | — |
| `scan-endpoints` | `scan.rs` | `scan-endpoints` | SafeActive | — | — |
| `fingerprint` | `scan.rs` | `fingerprint` | SafeActive | — | — |
| `scan` / `resume` | `scan.rs` | `pipeline` | SafeActive | — | — |
| `recon` | `recon.rs` | `recon` | SafeActive | — | — |
| `fuzz` | `fuzz.rs` | `fuzz` | Intrusive | — | — |
| `waf` | `fuzz.rs` | `waf-detect` | SafeActive | — | — |
| `waf-stress` | `fuzz.rs` | `waf-stress` | StressTest | — | — |
| `graphql` | `fuzz.rs` | `graphql` | Intrusive | — | — |
| `oauth` | `fuzz.rs` | `oauth` | CredentialTesting | — | — |
| `auth-test` | `auth_test.rs` | `auth-test` | CredentialTesting | — | — |
| `load` | `load.rs` | `load-test` | LoadTest | — | — |
| `stress` | `stress.rs` | `stress-test` | StressTest | `stress-testing` | — |
| `packet` / `icmp` / `traceroute` | `network.rs` | `packet` | RawPacket | `packet-inspection` (CLI gates `icmp`/`traceroute` on `stress-testing`) | — |
| `nse` | `scan.rs` | `nse` | SafeActive | `nse` | — |
| `hunt` | `hunt.rs` | `hunt` | SafeActive | `advanced-hunting` | — |
| `browser` | `browser.rs` | `browser` | SafeActive | `headless-browser` | — |
| `db` | `db_pentest.rs` | `db-pentest` | DbPentest | `db-pentest` | `--allow-db-pentest` |
| `proxy-intercept` | `web_proxy.rs` | `proxy-intercept` | TrafficInterception | `web-proxy` | `--allow-web-proxy` |
| `wireless` | `wireless.rs` | `wireless` | SafeActive | `wireless` | — |
| `wireless` (active branch) | `wireless.rs` | `wireless-deauth` | Intrusive | `wireless-advanced` | `--allow-active-wireless` |
| `mobile` (static branch) | `mobile.rs` | `mobile-static` | SafeActive | `mobile` | — |
| `mobile-dynamic` | `mobile.rs` | `mobile-dynamic` | Intrusive | `mobile-dynamic` | `--allow-dynamic-mobile` |
| `evasion` | `evasion.rs` | `evasion` | EvasionTesting | `evasion` | — |
| `postex` | `postex.rs` | `postex` | ExploitAdjacent | `postex` | — |
| `c2` | `c2.rs` | `c2` | C2Operation | `c2` | `--allow-c2` |

`packet`, `icmp`, `traceroute`, `wireless`, and `mobile` are **multiplexers**:
`commands/route.rs` resolves the concrete operation per execution branch before
approval. `proxy` (stress-testing proxy pool), `cluster`, `remote`,
`notify`, and the server/daemon verbs are non-operation routes and dispatch no
operation.

### 5.2 Programmatic Surfaces

| Surface | Entry Point | Profile | Overrides |
|---------|-----------|---------|-----------|
| REST | `tool/protocol/rest.rs` | McpStrict | No |
| gRPC | `tool/protocol/grpc.rs` | McpStrict | No |
| MCP | `tool/protocol/mcp/handlers/server.rs` | McpStrict | No |
| Agent | `agent/mod.rs` (via `agent::services::AgentExecutionService`) | AgentStrict | No |
| TUI | `eggsec-tui/src/app/` | ManualPermissive/Guarded | Yes |
| CI | `commands/handlers/ci.rs` | CiStrict | No — no dispatch path at all |

Strict surfaces reach `ToolDispatcher` only through `EnforcedDispatcher`.
Scope-sensitive dispatch (load testing, transport per-hop checks) uses
`dispatch_execution()` with an `ApprovedExecution` bundle; scope-insensitive
tools use `dispatch_checked()` with `ApprovedOperation`. Adapters never call
`tool.execute()` or `Scope::is_target_allowed` directly — they use the narrow
service traits.

### 5.3 Passive/Analytical Commands (No Dispatch)

| Command | Handler | Notes |
|---------|---------|-------|
| CI | `ci.rs` | Reads findings from stdin. No tool dispatch. |
| Vuln management | `vuln.rs` | CVSS scoring, triage, remediation. Pure computation. |
| Proxy list/health | `stress.rs` | Read-only queries. |

## 6. Internal Dispatch Boundary

| Item | Location | Status | Notes |
|------|----------|--------|-------|
| `ToolDispatcher::dispatch()` (raw) | `tool/dispatcher.rs` | Internal implementation detail. | Used only beneath `EnforcedDispatcher`. The enforced-dispatch regression test guards against use in strict surfaces. |
| `dispatch_mode` = `LegacyWrapped` | — | **Removed.** | All operation-backed commands use `RegistryBacked`. |
| Command routing | `commands/route.rs` | Single owner. | `route_for_commands()` is exhaustive over `Commands`; adding a variant without updating it is a compile error. |
| Corpus/engine seam | `architecture/knowledge_corpus.md` | One-directional. | Corpus owns data; the engine owns I/O. Guarded by architecture checks 114/147/148/149/150. |

## 7. Architecture Invariants

See [ARCHITECTURE_INVARIANTS.md](ARCHITECTURE_INVARIANTS.md) for the complete normative list, and
[architecture/overview.md](../architecture/overview.md) for the birds-eye view and the deep-dive
index (every component links to its own `architecture/<topic>.md`). Key invariants:

1. **Centralized authorization**: All side-effecting operations must have an `OperationDescriptor` evaluated by `EnforcementContext::evaluate()` before execution.
2. **No automated overrides**: Automated surfaces must never honor `ManualOverride`.
3. **Fail-closed strict**: Strict surfaces must fail closed on `Warn`, `RequireConfirmation`, or `Deny`.
4. **Scope provenance**: Explicit manifest provenance must be checked for automated networked operations.
5. **Domain crates don't authorize**: Domain crates must not decide authorization.
6. **Feature gates ≠ authorization**: Feature gates are not sufficient authorization; runtime policy must still apply.
7. **Dry-run purity**: Dry-run must be side-effect free.
8. **Token uniqueness**: Approval tokens must not be reusable for a different tool or target.
9. **Type-level dispatch**: Strict surfaces must dispatch through `EnforcedDispatcher` — `dispatch_execution()` with `ApprovedExecution` for scope-sensitive operations, `dispatch_checked()` with `ApprovedOperation` for scope-insensitive tools.
10. **Regression test guard**: The enforced dispatch regression test must remain green.
11. **Session-derived surface**: Daemon executor derives surface from `RuntimeSession`, not hardcoded defaults.
12. **ApprovedRunRequest bundle**: Dispatch through runtime bridge uses coupled approval+request bundle.
13. **Daemon capabilities reflect mode**: Capabilities advertise only what the executor can actually perform.

## Appendix: Key File Locations

| Concept | File |
|---------|------|
| `ExecutionSurface`, `ExecutionProfile` | `crates/eggsec-policy/src/policy.rs` (facade: `crates/eggsec/src/config/policy.rs`) |
| `OperationDescriptor`, `OperationMetadata` | `crates/eggsec-policy/src/policy.rs` + `catalog.rs` (facade: `crates/eggsec/src/config/policy.rs`) |
| `EnforcementContext`, `ApprovedOperation` | `crates/eggsec-policy/src/decision.rs` + `approval.rs` (facade: `crates/eggsec/src/config/policy_decision.rs`; engine DNS/feature adapters in `crates/eggsec/src/policy_bridge/`) |
| `LoadedScope`, `Scope` | `crates/eggsec-policy/src/scope.rs` (facade: `crates/eggsec/src/config/scope.rs`) |
| `ScopeSpec` (declarative), `scope_from_spec`, intersection | `crates/eggsec-tool-core/src/request.rs`, `crates/eggsec/src/config/scope_spec.rs` |
| `EnforcedDispatcher` | `crates/eggsec/src/tool/dispatcher.rs` |
| `EngineServices` | `crates/eggsec/src/tool/service.rs` |
| `McpEngineBridge` | `crates/eggsec/src/tool/protocol/mcp/bridge.rs` |
| `AgentExecutionService` | `crates/eggsec/src/agent/services.rs` |
| `runtime_bridge` (Runtime→Engine bridge) | `crates/eggsec/src/runtime_bridge/` |
| `TuiEnforcementState` | `crates/eggsec-tui/src/app/enforcement.rs` |
| CLI surface resolution | `crates/eggsec-cli/src/main.rs` |
| REST enforcement | `crates/eggsec/src/tool/protocol/rest.rs` |
| MCP enforcement | `crates/eggsec/src/tool/protocol/mcp/handlers/server.rs` |
| gRPC enforcement | `crates/eggsec/src/tool/protocol/grpc.rs` |
| Agent enforcement | `crates/eggsec/src/agent/mod.rs` |
| Enforced dispatch regression test | `crates/eggsec/tests/enforced_dispatch_regression.rs` |
| Command registry | `crates/eggsec/src/commands/registry.rs` |

## Extending Eggsec

For contributor-facing guidance on adding operations, domains, commands, tool integrations, TUI actions, report/evidence outputs, and features, see [EXTENSIBILITY.md](EXTENSIBILITY.md) and the detailed guides in [extending/](extending/).
