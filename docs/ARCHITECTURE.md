# Eggsec Architecture

Policy-mediated security assessment engine with multiple frontends, centralized enforcement, and domain execution crates.

## 1. System Overview

Eggsec is a Rust-native, scope-enforced security assessment and defense-validation engine. It supports:

- **Manual operator workflows** (CLI, TUI) where humans decide which tests to run
- **Automated workflows** (REST, MCP, gRPC, Agent, CI) where policy must be enforced without operator discretion

The architecture enforces a critical invariant: **authorization is centralized; domain crates declare and execute but must not authorize**. Strict programmatic surfaces (REST, MCP, gRPC, Agent) always use `EnforcedDispatcher::dispatch_checked()` with an `ApprovedOperation` token. The orchestrator currently retains raw `ToolDispatcher::dispatch()` as a transitional exception; it must only be invoked after caller-level enforcement.

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
         │   → ApprovedOperation token   │
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

## 2. Workspace Crate Ownership

| Crate | Role | Policy Decisions | Execution | Frontend | Dependency-Light | Notes |
|-------|------|:---:|:---:|:---:|:---:|-------|
| `eggsec-core` | Shared primitives | No | No | No | Yes | `Severity`, `SensitiveString`, constants. Zero internal deps. |
| `eggsec-tool-core` | Protocol-neutral DTOs | No | No | No | Yes | `ToolRequest`, `ToolResponse`, `ToolError`, history types. |
| `eggsec-output` | Report formatting | No | Output adapters | No | Yes | JSON/CSV/HTML/SARIF/JUnit/Markdown. Portable adapters. |
| `eggsec-agent` | Agent coordination | No | Coordination only | No | Yes | Registry, scheduler, lifecycle. Depends only on `eggsec-core`. |
| `eggsec` | Composition root | **Yes** | All domains | No | No | Central policy, orchestration, all security modules. |
| `eggsec-cli` | Binary entrypoint | No | No | **Yes** | Yes | Thin wrapper: depends on `eggsec` + `eggsec-tui`. |
| `eggsec-tui` | TUI frontend | No | No | **Yes** | No | 33 tabs, enforcement toggle, packaged themes. |
| `eggsec-nse` | NSE compatibility | No | Domain execution | No | Yes | Lua VM, 166 NSE libraries. Optional. |
| `eggsec-db-lab` | DB pentest domain | No | Domain execution | No | Yes | Postgres/MySQL/MSSQL/MongoDB/Redis checks. |
| `eggsec-web-proxy` | Web proxy domain | No | Domain execution | No | Yes | MITM intercept, TLS, protocol handlers. |
| `eggsec-mobile-lab` | Mobile analysis domain | No | Domain execution | No | Yes | APK/IPA static analysis + Android dynamic runtime testing (ADB, Frida, behavioral correlation). |

**Dependency direction**: Leaf crates (`eggsec-core`, `eggsec-output`, `eggsec-agent`) have no internal workspace dependencies. The main `eggsec` crate is the composition root. `eggsec-cli` and `eggsec-tui` are the only frontends.

## 3. Enforcement Model

### 3.1 Core Types

| Type | Location | Purpose |
|------|----------|---------|
| `ExecutionSurface` | `config/policy.rs` | Caller origin identity. 9 variants: `CliManual`, `TuiManual`, `CliManualStrict`, `TuiManualStrict`, `McpServer`, `SecurityAgent`, `Ci`, `RestApi`, `GrpcApi`. |
| `ExecutionProfile` | `config/policy.rs` | Trust boundary. 5 variants: `ManualPermissive`, `ManualGuarded`, `CiStrict`, `McpStrict`, `AgentStrict`. |
| `OperationRisk` | `config/policy.rs` | Risk tier ordering. 15 variants from `Passive` to `AgentAutonomous`. |
| `OperationMode` | `config/policy.rs` | Semantic mode: `StandardAssessment`, `DefenseLab`, `HazardousLab`. |
| `Capability` | `config/policy.rs` | Fine-grained capability declarations. 18 variants. |
| `OperationDescriptor` | `config/policy.rs` | The unit of policy evaluation. Bundles operation name, mode, risk, target, required features, capabilities, and scope requirements. |
| `OperationMetadata` | `config/policy.rs` | Static registry entry. 29 operations + 32 aliases. Single source of truth for all surfaces. |
| `ExecutionPolicy` | `config/policy.rs` | TOML-deserialized config controlling which risk tiers and capabilities are allowed. |
| `LoadedScope` | `config/scope.rs` | Scope with provenance. `is_explicit_manifest()` distinguishes "no scope" from "explicitly empty scope". |
| `EnforcementContext` | `config/policy_decision.rs` | Bundles `ExecutionProfile` + `ExecutionPolicy` + `LoadedScope`. Created once per execution path. |
| `EnforcementOutcome` | `config/policy_decision.rs` | Profile-aware result: `Allow`, `Warn`, `RequireConfirmation`, `Deny`. |
| `ManualOverride` | `config/policy_decision.rs` | CLI/TUI override flags. `--yes` is narrow (only `OutOfScope`/`TargetExpansion`). |
| `ApprovedOperation` | `config/policy_decision.rs` | Proof-of-enforcement token. Private fields. Created exclusively by `EnforcementContext::approve()` or `approve_manual()`. |
| `EnforcedDispatcher` | `tool/dispatcher.rs` | Wraps `ToolDispatcher` requiring `ApprovedOperation` before dispatch. Type-level enforcement gate. |

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

| Profile | Scope Missing | Scope Ambiguous | RequireConfirmation | Warn |
|---------|:---:|:---:|:---:|:---:|
| `ManualPermissive` | Downgrade to Warn (if safe) | Warn | Operator override | Proceed |
| `ManualGuarded` | Deny | Allow | Deny | Allow |
| `McpStrict` | Deny | Deny | Deny | Deny |
| `AgentStrict` | Deny | Deny | Deny | Deny |
| `CiStrict` | Deny | Deny | Deny | Deny |

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
  → handle_command() → handler builds OperationDescriptor
  → ctx.evaluate_and_enforce_operation(descriptor)
  → on approval: execute tool
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
  → cache ApprovedOperation in pending_approved
  → evaluate_policy_and_dispatch() → spawn_task()
```

**Surface**: `TuiManual` (default), toggle to `TuiManualStrict` via Ctrl+G.
**Profile**: `ManualPermissive` → `ManualGuarded` on toggle.
**Overrides**: Supported via confirmation overlay.

### 4.3 REST API

```
HTTP POST /api/v1/tools/{tool_id}/execute
  → handle_serve() constructs EnforcementContext(RestApi)
  → validate target/payload → build OperationDescriptor
  → check rest_exposable → enforcement.approve(RestApi, descriptor)
  → only Allow proceeds → dispatcher.dispatch_checked(approved, request)
```

**Surface**: `RestApi` → `McpStrict`. Always strict. No overrides.
**Scope**: `--scope-file` or inherited. Always sets `requires_explicit_scope = true`.
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

## 5. Side-Effecting Execution Path Inventory

### 5.1 CLI Command Handlers

| Operation Family | Handler File | Operation ID | Risk | Feature Gate | Descriptor | Enforcement | Extra Runtime Gate |
|-----------------|-------------|-------------|------|-------------|:---:|:---:|-------------------|
| Port scan | `scan.rs` | `scan-ports` | SafeActive | — | ✓ | ✓ | — |
| Endpoint scan | `scan.rs` | `scan-endpoints` | SafeActive | — | ✓ | ✓ | — |
| Fingerprint | `scan.rs` | `fingerprint` | SafeActive | — | ✓ | ✓ | — |
| NSE script | `scan.rs` | `nse` | Intrusive | `nse` | ✓ | ✓ | — |
| Pipeline scan | `scan.rs` | `scan` | SafeActive | — | ✓ | ✓ | — |
| Resume scan | `scan.rs` | `scan-resume` | SafeActive | — | ✓ | ✓ | — |
| Recon | `recon.rs` | `recon` | SafeActive | — | ✓ | ✓ | — |
| Fuzz | `fuzz.rs` | `fuzz` | Intrusive | — | ✓ | ✓ | — |
| WAF detect | `fuzz.rs` | `waf-detect` | Intrusive | — | ✓ | ✓ | — |
| WAF stress | `fuzz.rs` | `waf-stress` | Intrusive | — | ✓ | ✓ | — |
| GraphQL fuzz | `fuzz.rs` | `graphql` | Intrusive | — | ✓ | ✓ | — |
| OAuth fuzz | `fuzz.rs` | `oauth` | Intrusive | — | ✓ | ✓ | — |
| Load test | `load.rs` | `load` | LoadTest | — | ✓ | ✓ | — |
| Auth test | `auth_test.rs` | `auth-test` | CredentialTesting | — | ✓ | ✓ | — |
| Stress test | `stress.rs` | `stress` | StressTest | `stress-testing` | ✓ | ✓ | — |
| Proxy add | `stress.rs` | `proxy-add` | ExploitAdjacent | `stress-testing` | ✓ | ✓ | — |
| Proxy test | `stress.rs` | `proxy-test` | ExploitAdjacent | `stress-testing` | ✓ | ✓ | — |
| Packet send | `network.rs` | `packet-send` | RawPacket | `packet-inspection` | ✓ | ✓ | — |
| Packet traceroute | `network.rs` | `packet-traceroute` | RawPacket | `packet-inspection` | ✓ | ✓ | — |
| ICMP | `network.rs` | `icmp` | SafeActive | `stress-testing` | ✓ | ✓ | — |
| Traceroute | `network.rs` | `traceroute` | RawPacket | `stress-testing` | ✓ | ✓ | — |
| DB pentest | `db_pentest.rs` | `db-pentest` | DbPentest | `db-pentest` | ✓ | ✓ | `--allow-db-pentest` |
| Web proxy | `web_proxy.rs` | `proxy-intercept` | TrafficInterception | `web-proxy` | ✓ | ✓ | `--allow-web-proxy` |
| Wireless scan | `wireless.rs` | `wireless` | SafeActive | `wireless` | ✓ | ✓ | — |
| Wireless deauth | `wireless.rs` | `wireless-deauth` | Intrusive | `wireless-advanced` | ✓ | ✓ | `--allow-active-wireless` |
| Mobile static | `mobile.rs` | `mobile-static` | SafeActive | `mobile` | ✓ | ✓ | — |
| Mobile dynamic | `mobile.rs` | `mobile-dynamic` | SafeActive/Intrusive | `mobile-dynamic` | ✓ | ✓ | `--allow-dynamic-mobile` |
| Evasion | `evasion.rs` | `evasion` | EvasionTesting | `evasion` | ✓ | ✓ | Always dry-run |
| Postex | `postex.rs` | `postex` | SafeActive/ExploitAdjacent | `postex` | ✓ | ✓ | Always dry-run |
| C2 | `c2.rs` | `c2` | SafeActive/C2Operation | `c2` | ✓ | ✓ | `--allow-c2` |
| Browser | `browser.rs` | `browser` | SafeActive | `headless-browser` | ✓ | ✓ | — |
| Hunt | `hunt.rs` | `hunt` | Intrusive | `advanced-hunting` | ✓ | ✓ | — |

### 5.2 Programmatic Surfaces

| Surface | Entry Point | Dispatch Method | Profile | Overrides |
|---------|-----------|----------------|---------|-----------|
| REST | `rest.rs::handle_tool_call()` | `EnforcedDispatcher::dispatch_checked()` | McpStrict | No |
| gRPC | `grpc.rs::execute_tool()` | `EnforcedDispatcher::dispatch_checked()` | McpStrict | No |
| MCP | `mcp/handlers/server.rs::handle_tools_call()` | `EnforcedDispatcher::dispatch_checked()` | McpStrict | No |
| Agent | `agent/mod.rs::execute_scan()` | `EnforcedDispatcher::dispatch_checked()` | AgentStrict | No |
| TUI | `app/mod.rs::evaluate_policy_and_dispatch()` | `EnforcedDispatcher::dispatch_checked()` | ManualPermissive/Guarded | Yes |
| Orchestrator | `tool/orchestrator/mod.rs::execute_stage()` | `ToolDispatcher::dispatch()` (raw) | Caller must enforce | N/A |

### 5.3 Passive/Analytical Commands (No Dispatch)

| Command | Handler | Notes |
|---------|---------|-------|
| CI | `ci.rs` | Reads findings from stdin. No tool dispatch. |
| Vuln management | `vuln.rs` | CVSS scoring, triage, remediation. Pure computation. |
| Proxy list/health | `stress.rs` | Read-only queries. |

## 6. Transitional APIs and Risk Register

| Item | Location | Status | Recommended Disposition |
|------|----------|--------|------------------------|
| `CommandContext::ensure_scope()` / `ensure_scope_url()` | `commands/handlers/mod.rs:223-228` | **Deprecated (Phase 2)**. No callers. | **Deprecate**. Scope checks are centralized in `EnforcementContext::evaluate()`. |
| `CommandContext::with_execution_profile()` | `commands/handlers/mod.rs:161` | **Deprecated (Phase 2)**. Test-only. | **Deprecate**. Replace with `with_execution_surface()` or direct `EnforcementContext` construction. |
| `ToolDispatcher::dispatch()` (raw) | `tool/dispatcher.rs:36` | `pub(crate)`, `#[doc(hidden)]`. Used by Orchestrator. | **Restrict visibility**. Keep for Orchestrator with regression test guard. |
| Orchestrator raw dispatch | `tool/orchestrator/mod.rs:194,210` | Raw dispatch without enforcement. Regression test allows it. | **Keep with invariant**. Callers must enforce before constructing Orchestrator. |
| `utils::check_scope()` / `check_scope_from_url()` | `utils/scope.rs` | Legacy standalone helpers. No handler callers. | **Deprecate**. Superseded by `EnforcementContext` scope evaluation. |
| Feature metadata duplication | Cargo.toml, README, policy metadata, tool docs | Feature descriptions exist in multiple places. | **Migrate**. Consolidate to `OperationMetadata` as single source of truth. |
| Central command match growth | `commands/handlers/mod.rs` | Growing match arms in `handle_command()`. | **Keep for now**. Monitor; refactor in Phase 2 if needed. |
| Domain logic in main crate | Various modules in `eggsec/src/` | Some domain logic still embedded (e.g., scanner, fuzzer internals). | **Keep for now**. Domain extraction is a Phase 2+ concern. |
| CI handler dispatch invariant | `commands/handlers/ci.rs:5` | **Tested (Phase 2)**. Regression test `ci_handler_has_no_dispatch_path`. | **Test**. Add regression test verifying no `ToolDispatcher` import in CI handler. |

## 7. Architecture Invariants

See [ARCHITECTURE_INVARIANTS.md](ARCHITECTURE_INVARIANTS.md) for the complete normative list. Key invariants:

1. **Centralized authorization**: All side-effecting operations must have an `OperationDescriptor` evaluated by `EnforcementContext::evaluate()` before execution.
2. **No automated overrides**: Automated surfaces must never honor `ManualOverride`.
3. **Fail-closed strict**: Strict surfaces must fail closed on `Warn`, `RequireConfirmation`, or `Deny`.
4. **Scope provenance**: Explicit manifest provenance must be checked for automated networked operations.
5. **Domain crates don't authorize**: Domain crates must not decide authorization.
6. **Feature gates ≠ authorization**: Feature gates are not sufficient authorization; runtime policy must still apply.
7. **Dry-run purity**: Dry-run must be side-effect free.
8. **Token uniqueness**: Approval tokens must not be reusable for a different tool or target.
9. **Type-level dispatch**: Strict surfaces must use `EnforcedDispatcher::dispatch_checked()` with `ApprovedOperation`.
10. **Regression test guard**: The enforced dispatch regression test must remain green.

## Appendix: Key File Locations

| Concept | File |
|---------|------|
| `ExecutionSurface`, `ExecutionProfile` | `crates/eggsec/src/config/policy.rs` |
| `OperationDescriptor`, `OperationMetadata` | `crates/eggsec/src/config/policy.rs` |
| `EnforcementContext`, `ApprovedOperation` | `crates/eggsec/src/config/policy_decision.rs` |
| `LoadedScope`, `Scope` | `crates/eggsec/src/config/scope.rs` |
| `EnforcedDispatcher` | `crates/eggsec/src/tool/dispatcher.rs` |
| `TuiEnforcementState` | `crates/eggsec-tui/src/app/enforcement.rs` |
| CLI surface resolution | `crates/eggsec-cli/src/main.rs` |
| REST enforcement | `crates/eggsec/src/tool/protocol/rest.rs` |
| MCP enforcement | `crates/eggsec/src/tool/protocol/mcp/handlers/server.rs` |
| gRPC enforcement | `crates/eggsec/src/tool/protocol/grpc.rs` |
| Agent enforcement | `crates/eggsec/src/agent/mod.rs` |
| Enforced dispatch regression test | `crates/eggsec/tests/enforced_dispatch_regression.rs` |
