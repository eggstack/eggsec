# Dual-Mode Enforcement Contract

Eggsec intentionally supports two usage families with distinct enforcement postures:

- **Manual operator posture** (CLI/TUI): Human-directed security assessment. Operators may proceed through warnings and explicit confirmations where appropriate. This mode is designed to remain productive and should not inherit agent-grade strictness by default.
- **Automated agent posture** (MCP, security agent, CI, REST, gRPC): Programmatic noninteractive execution. Strict, explicitly scoped, non-overridable. Manual overrides are never honored.

The contract below is the source of truth for how enforcement behaves per execution surface. All later implementation phases must follow this contract to prevent drift in either direction: over-hardening manual use, or under-hardening agent use.

## Terminology

| Term | Definition |
|------|------------|
| **Execution surface** | Where the request originates: CLI, TUI, MCP server, security agent, CI pipeline, REST API. |
| **Execution profile** | Enforcement behavior, represented by `ExecutionProfile` (`ManualPermissive`, `ManualGuarded`, `CiStrict`, `McpStrict`, `AgentStrict`). |
| **Manual permissive** | Human-directed default mode (`ManualPermissive`). Warnings and confirmation prompts are available; operator may override low-risk classes. |
| **Manual guarded** | Strict human mode (`ManualGuarded`). Equivalent to CLI `--strict-scope` and future TUI guarded toggle. No discretion path. |
| **Agent strict** | Noninteractive/model-controlled strict posture (`AgentStrict`). Cannot self-approve scope expansion or override any enforcement. Handler defensively rebuilds `AgentStrict`; runtime validates profile at construction. |
| **Scope provenance** | Whether scope came from an explicit manifest (`ConfigFile`, `CliScopeFile`, `GeneratedPreset`) versus `DefaultEmpty` (no manifest provided). |
| **Manual override** | Explicit operator acceptance of specific confirmation classes. Only valid in `ManualPermissive` contexts. Honored and audited only there. |
| **Confirmation class** | Machine-readable class (`ConfirmationClass`) requiring explicit operator action before dispatch. |

## Surface Behavior Matrix

| Execution Surface | Intended Posture | `ExecutionProfile` | Explicit Scope Manifest Required | `Warn` May Dispatch | `RequireConfirmation` May Dispatch After Override | Manual Override Flags Honored | Policy Re-evaluated Before Dispatch |
|---|---|---|---|---|---|---|---|
| CLI default | Manual permissive | `ManualPermissive` | No (warnings for safe scope misses) | Yes | Yes (with matching `ManualOverride`) | Yes | No (single evaluation) |
| CLI `--strict-scope` | Manual guarded | `ManualGuarded` | Yes (networked operations) | No | No (treated as deny) | No | No (single evaluation) |
| TUI default | Manual permissive | `ManualPermissive` | No (warnings for safe scope misses) | Yes | Yes (with matching `ManualOverride`) | Yes | No (single evaluation) |
| TUI guarded | Manual guarded | `ManualGuarded` | Yes (networked operations) | No | No (treated as deny) | No | No (single evaluation) |
| MCP server | Agent strict | `McpStrict` | Yes (networked operations) | No (treated as deny) | No (treated as deny) | No | Yes |
| Security agent | Agent strict | `AgentStrict` | Yes (networked operations) | No (treated as deny) | No (treated as deny) | No | Yes |
| CI | Agent strict | `CiStrict` | Yes (target/networked operations) | No (treated as deny) | No (treated as deny) | No | No (single evaluation) |
| REST API | Agent strict | `McpStrict` or `CiStrict` | Yes (networked operations) | No (treated as deny) | No (treated as deny) | No | Yes |
| gRPC API | Agent strict | `McpStrict` | Yes (networked operations) | No (treated as deny) | No (treated as deny) | No | Yes |
| Daemon HTTP | Agent strict | `McpStrict` | Yes (networked operations) | No (treated as deny) | No (treated as deny) | No | Yes |

**Key invariant**: `ManualPermissive` behavior must not bleed into MCP, security agent, CI, REST, or gRPC. Agent strict behavior must not become the default for normal CLI/TUI manual use.

**REST enforcement specifics**: REST API now constructs `EnforcementContext::for_surface(ExecutionSurface::RestApi, ...)` and dispatches every tool call through `enforcement.evaluate()` before execution. Only `EnforcementOutcome::Allow` permits dispatch. `Warn`, `RequireConfirmation`, and `Deny` all result in HTTP 403 Forbidden with a structured `RestPolicyErrorResponse` (code: `POLICY_DENIED`, includes serialized `PolicyDecision`). REST is noninteractive and programmatic — warning-class ambiguity must not dispatch. Metadata `rest_exposable` flags are enforced before policy evaluation; non-exposed tools fail closed. `RestState` carries `EnforcementContext` instead of `Option<Scope>`.

**Daemon HTTP enforcement specifics**: Daemon HTTP transport (feature-gated `http-api`) uses `EnforcementContext` with `McpStrict` profile by default. HTTP routes map 1:1 to `ClientCommand` variants and go through `DaemonHost::handle_command()` with `DaemonRequestContext`. Since the HTTP surface is noninteractive and programmatic, it follows the same enforcement contract as REST: only `Allow` permits dispatch; `Warn`/`RequireConfirmation`/`Deny` result in error responses. Loopback-only bind is enforced by default; public bind requires explicit configuration and emits a warning.

**gRPC enforcement specifics**: gRPC API now constructs `GrpcService` with `EnforcementContext::for_surface(ExecutionSurface::GrpcApi, ...)` and dispatches every tool call through `EnforcementContext::approve()` → `EnforcedDispatcher::dispatch_checked()`. Only `EnforcementOutcome::Allow` produces an `ApprovedOperation` token; `Warn`, `RequireConfirmation`, and `Deny` all fail with `EnforcementError` and return gRPC `Status::permission_denied`. Metadata `grpc_exposable` flags are enforced before policy evaluation; non-exposed tools fail closed. Audit events emitted for all enforcement outcomes including denials.

## Outcome Semantics

`EnforcementOutcome` wraps a `PolicyDecision` with profile-aware dispatch semantics:

| Outcome | Manual Permissive | Manual Guarded | Automated (CI/MCP/Agent) |
|---------|-------------------|----------------|--------------------------|
| `Allow` | Dispatch permitted | Dispatch permitted | Dispatch permitted |
| `Warn` | Dispatch permitted; warnings must be visible and audited | Treated as deny | Treated as deny |
| `RequireConfirmation` | Dispatch permitted **only** after matching `ManualOverride` classes are present | Treated as deny | Treated as deny |
| `Deny` | Dispatch never permitted | Dispatch never permitted | Dispatch never permitted |

**Invariant**: Automated surfaces must treat `Warn` conservatively (as denial) and must treat `RequireConfirmation` as denial. Only `ManualPermissive` may dispatch on `Warn` or `RequireConfirmation` (with matching override). REST specifically: only `Allow` permits dispatch; `Warn`, `RequireConfirmation`, and `Deny` all return HTTP 403 with structured policy error.

## Manual Discretion Classes

`ConfirmationClass` variants represent categories of conditions that trigger `RequireConfirmation` under `ManualPermissive`:

| Class | `as_str()` | Override Mechanism | Notes |
|-------|-----------|-------------------|-------|
| `OutOfScope` | `out-of-scope` | `--allow-out-of-scope` or `--yes` | Low-risk scope confirmation |
| `TargetExpansion` | `target-expansion` | `--allow-out-of-scope` or `--yes` | Low-risk scope confirmation |
| `HighRisk` | `high-risk` | `--allow-high-risk` or `--allow-db-pentest` | Requires dedicated flag and reason |
| `NonBaselineCapability` | `nonbaseline-capability` | `--allow-nonbaseline-capability` | Requires dedicated flag |
| `PrivateResolution` | `private-resolution` | `--allow-private-resolution` | Requires dedicated flag |
| `CrossHostRedirect` | `cross-host-redirect` | `--allow-cross-host-redirect` | Requires dedicated flag |
| `TrafficInterception` | `traffic-interception` | `--allow-web-proxy` | Requires dedicated web-proxy flag |
| `ExplicitExclusion` | `explicit-exclusion` | `--allow-explicit-exclusion` | Requires dedicated flag and audit reason |

### `--yes` Scope

`--yes` (`assume_yes`) is intentionally narrow. It suppresses low-risk manual prompts for:
- `OutOfScope`
- `TargetExpansion`

`--yes` does **not** authorize:
- `HighRisk`
- `NonBaselineCapability`
- `PrivateResolution`
- `CrossHostRedirect`
- `TrafficInterception`
- `ExplicitExclusion`

Those classes require their dedicated `--allow-*` flags. This prevents accidental authorization of high-risk or sensitive operations through prompt suppression.

## Hard-Deny Classes

The following conditions produce hard denial and must **never** be converted to manual confirmation:

| Condition | Rationale |
|-----------|-----------|
| Missing compile-time feature | Build configuration error; cannot proceed |
| Invalid target | Unresolvable or malformed target |
| Scope parse/check error | Scope configuration is broken |
| Capability explicitly denied by policy | Policy explicitly blocks this capability |
| Risk not allowed by execution policy | Operation exceeds policy risk limits |
| Missing explicit scope manifest in automated mode | Strict profiles require explicit scope for networked operations |
| Agent/model-supplied override attempt | Automated surfaces cannot self-approve scope expansion |

## Policy Invariants

These invariants hold across all execution paths:

1. **Manual permissive isolation**: Manual permissive behavior must not bleed into MCP, security agent, CI, REST, or gRPC.
2. **Agent strict isolation**: Agent strict behavior must not become the default for normal CLI/TUI manual use.
3. **Override scope**: Manual override flags are only honored in `ManualPermissive` contexts.
4. **Scope provenance**: Scope provenance for automated networked execution must come from `LoadedScope`, not raw `Scope`.
5. **Shared evaluation**: Every dispatch path must eventually flow through `EnforcementContext::evaluate()`.
6. **Re-evaluation**: Agent/MCP dispatch must re-evaluate enforcement immediately before dispatch.
7. **Constructor intent**: Programmatic constructors for agent-facing servers should require explicit enforcement context or be clearly test-only.
8. **Type-level dispatch**: Strict programmatic surfaces (REST, MCP, Agent, gRPC) require an `ApprovedOperation` token before dispatch via `EnforcedDispatcher::dispatch_checked()`. Raw `ToolDispatcher::dispatch()` is not reachable from these surfaces.

## Operation Metadata Integration (Phase 6)

All protocol surfaces now derive `OperationDescriptor` from the canonical `OperationMetadata` registry:

- **REST**: Uses `metadata_for_tool_id(tool_id)` with fallback for unknown tools. Always sets `requires_explicit_scope = true`.
- **MCP**: Uses `metadata_for_tool_id(tool_id)` with profile-specific `intended_uses` and `requires_explicit_scope` from `McpProfilePolicy`.
- **TUI**: Uses `operation_metadata(op_id)` from tab spec. Tab-specific overrides for wireless-advanced (DefenseLab mode) and db-pentest (DefenseLab mode).
- **Agent**: Uses `metadata_for_tool_id(scan_type)` for known scan types. Falls back to keyword-based classification for unknown scan types.

Missing metadata for an externally executable tool triggers a runtime warning (REST/MCP/agent) or uses a conservative fallback (agent only).

## Examples

### CLI manual scan with missing scope

**Scenario**: Operator runs `eggsec scan example.com` without a scope file.

**Expected**: `EnforcementOutcome::Warn` (safe scope-selection miss for passive/safe-active StandardAssessment). Warning is visible. Scan proceeds. No hard denial.

### CLI manual positive allowlist miss

**Scenario**: Scope has `[[allowed_targets]] pattern = "*.lab.internal"` but operator scans `example.com`.

**Expected**: `EnforcementOutcome::RequireConfirmation` with class `out-of-scope`. Operator must pass `--allow-out-of-scope` to proceed. `--yes` also suppresses this prompt.

### CLI strict positive allowlist miss

**Scenario**: Same as above but with `--strict-scope` (`ManualGuarded`).

**Expected**: `EnforcementOutcome::Deny`. No discretion path. Hard denial.

### MCP missing explicit manifest

**Scenario**: MCP server receives a networked tool call with `DefaultEmpty` scope.

**Expected**: `EnforcementOutcome::Deny`. `LoadedScope::is_explicit_manifest()` returns false for `DefaultEmpty`. Strict profiles require explicit manifest for networked operations.

### Security agent with high-risk nonbaseline capability not allowlisted

**Scenario**: Agent requests an `IntrusiveFuzz` capability but policy has not added it to `allowed_capabilities`.

**Expected**: `EnforcementOutcome::Deny`. Non-baseline capabilities (`IntrusiveFuzz`, `LoadTest`, etc.) require explicit listing in `allowed_capabilities` for strict profiles. `PassiveFingerprint`, `ActiveProbe`, `Crawl`, `WafDetect` are baseline and allowed by default.

### TUI manual high-risk action

**Scenario**: Operator triggers a high-risk action in TUI (e.g., WAF stress test).

**Expected**: TUI preflight shows `RequireConfirmation` with class `high-risk`. Operator must pass `--allow-high-risk` flag with a reason. `--yes` does not suppress this prompt.

## Phase 4 Regression Coverage

Phase 4 added regression tests to protect manual CLI/TUI discretion from agent-grade strictness leaking into default operation. Tests cover:

- **Policy-level outcomes** (`config::policy_decision::tests`): 48 tests verifying `evaluate_enforcement` produces correct outcomes (Allow/Warn/RequireConfirmation/Deny) for each profile, risk level, and scope configuration.
- **CommandContext override wiring** (`commands::handlers::tests`): 48 tests verifying CLI flags map correctly to `ManualOverride`, error messages list exact flags needed, strict profiles ignore overrides, and audit fields are recorded.

Key invariants locked by tests:

| Invariant | Test Coverage |
|-----------|--------------|
| `--yes` only covers OutOfScope/TargetExpansion | `manual_override_permits_narrow_yes_for_outofscope_targetexpansion_only`, `yes_alone_does_not_permit_high_risk`, `yes_alone_does_not_permit_explicit_exclusion` |
| High-risk requires dedicated flag | `allow_high_risk_permits_high_risk_without_explicit_exclusion`, `manual_permissive_high_risk_no_override_error_explains_yes_insufficient` |
| Private/cross-host require dedicated flags | `allow_private_resolution_permits_private_resolution_class`, `allow_cross_host_redirect_permits_cross_host_class`, `allow_out_of_scope_does_not_permit_private_or_cross_host` |
| Strict profiles ignore overrides | `manual_guarded_with_all_overrides_still_denies_require_confirmation`, `ci_strict_with_all_overrides_still_denies_require_confirmation`, `mcp_strict_via_command_context_ignores_overrides`, `agent_strict_via_command_context_ignores_overrides` |
| TrafficInterception requires web-proxy flag | `manual_override_traffic_interception_permits_only_web_proxy` |
| Error messages name exact flags | `command_context_error_messages_list_exact_dedicated_flags`, `manual_permissive_out_of_scope_no_override_error_suggests_allow_flag` |
| Audit records use stable kebab strings | `successful_override_records_stable_kebab_case_classes_on_decision_no_debug_no_dups`, `successful_out_of_scope_override_records_audit_fields` |
| Explicit exclusions hard-deny in non-permissive | `explicit_exclusion_denies_in_all_profiles`, `manual_permissive_does_not_downgrade_explicit_exclusion` |
| Risk-policy/feature/capability denials stay hard-deny | `manual_permissive_does_not_downgrade_risk_policy_denial`, `manual_permissive_does_not_downgrade_feature_missing_denial`, `manual_permissive_does_not_downgrade_capability_denial` |

## Phase 8 Enforcement Matrix Coverage

Phase 8 added a comprehensive enforcement matrix test suite (`crates/eggsec/tests/enforcement_matrix.rs`) that systematically tests all execution surfaces against scope states, risk tiers, capabilities, and override handling. The matrix protects the dual-mode contract by catching:

1. Manual CLI/TUI becoming too strict to be useful.
2. Agent/MCP/REST/CI becoming too permissive or honoring manual discretion.

**169 tests** covering:

- **Surface mapping invariants**: All 8 `ExecutionSurface` variants map to correct `ExecutionProfile`.
- **Manual permissive invariants**: Safe ops allow, scope misses require confirmation, `assume_yes` is narrow, denied capabilities hard-deny, missing features hard-deny.
- **Manual guarded invariants**: Scope misses deny, overrides ignored, high-risk with policy allows.
- **MCP invariants**: Missing scope denies, no confirmation path, baseline capabilities allowed, non-baseline requires explicit allow.
- **Security agent invariants**: Same as MCP, plus `AgentStrict` profile, warnings treated as denial.
- **REST invariants**: Explicit manifest required, only `Allow` dispatches, no confirmation/warn path, overrides ignored.
- **CI invariants**: Matches automated strict behavior, no override honoring.
- **Risk tier matrix**: All risk tiers (Passive through C2Operation, including DbPentest and TrafficInterception) tested across all surfaces with and without policy flags.
- **Capability matrix**: Baseline vs non-baseline capabilities across all surfaces, denied capability hard-deny, all 6 nonbaseline capability variants (RawPacketProbe, CredentialTesting, DatabaseAssessment, TrafficInterception, RemoteExecution, C2Simulation).
- **Override isolation**: `ManualOverride::permits()` tested for each `ConfirmationClass`, override flags don't leak across surfaces.
- **Scope state matrix**: DefaultEmpty, explicit allow, allow miss, exclusion - tested across permissive and strict surfaces.
- **Dual-mode contract**: Permissive never hard-deny safe in-scope, strict never produce Warn/RequireConfirmation.
- **Private/local target scope**: `requires_private_or_local_target` denies when no scope provided (all profiles), allows with explicit scope, scope miss behavior under permissive vs strict.
- **Metadata integration**: Descriptors generated from `OperationMetadata` for recon, fuzz, db-pentest, proxy-intercept, and packet operations, verifying risk + capability + feature requirements.

Run:
```bash
cargo test --test enforcement_matrix -p eggsec
cargo test -p eggsec --features rest-api --test enforcement_matrix
```

## Phase 2: Enforcement Invariant Hardening

Phase 2 verified and hardened the enforcement invariants established in earlier phases:

### Transitional API Cleanup

| API | Status | Disposition |
|-----|--------|-------------|
| `CommandContext::with_execution_profile()` | **Removed** | Replaced by `with_execution_surface()` and direct `EnforcementContext` construction. |
| `CommandContext::ensure_scope()` / `ensure_scope_url()` | **Removed** | Scope checks are centralized in `EnforcementContext::evaluate()`. |
| `ToolDispatcher::dispatch()` (raw) | `#[doc(hidden)]`, `pub(crate)` | Regression test guard (`enforced_dispatch_regression.rs`) enforces no raw dispatch in strict surfaces. |

### Regression Tests Added

- **CI handler dispatch invariant** (`ci_handler_has_no_dispatch_path`): Verifies CI handler contains no `ToolDispatcher`, `EnforcedDispatcher`, `SecurityTool`, `ToolRegistry`, or `dispatch_checked` imports. Architecture Invariant #19.

### Verified Coverage

Phase 2 confirmed that all acceptance criteria from the Phase 2 plan are satisfied by existing test infrastructure:

- **Enforcement matrix**: 169 tests in `enforcement_matrix.rs` covering manual permissive, manual guarded, and strict automated surfaces.
- **Override isolation**: Automated surfaces cannot use manual override flags to proceed.
- **Scope provenance**: Automated operations requiring explicit scope fail without explicit manifest provenance.
- **Approval token mismatch**: `dispatch_checked` rejects tool mismatch, target mismatch, and allows alias match.
- **Raw dispatch guard**: Strict surfaces cannot reach raw `ToolDispatcher::dispatch()`.
- **CI handler isolation**: CI handler has no dispatch path (Architecture Invariant #19).

Run:
```bash
cargo test --test enforcement_matrix -p eggsec
cargo test --test enforced_dispatch_regression -p eggsec
```

## Phase 12: Type-Level Enforcement Dispatch

Phase 12 hardened enforcement from convention (call sites expected to evaluate first) to type-level structure. Strict programmatic surfaces cannot dispatch a tool without an `ApprovedOperation` token, enforced structurally rather than by discipline at call sites.

### Core Types

**`ApprovedOperation`** (`config/policy_decision.rs`): Proof-of-enforcement token with private fields. Produced exclusively by `EnforcementContext::approve()` or `approve_manual()`. Read-only accessors: `descriptor()`, `decision()`, `surface()`, `profile()`, `audit_event_id()`. Cannot be constructed outside enforcement code.

**`EnforcementError`** (`config/policy_decision.rs`): Structured error from `approve()`/`approve_manual()`:
- `Denied { decision }` - Policy denied the operation (`Deny` and `Warn` on strict surfaces).
- `ConfirmationRequired { decision, required_classes }` - Manual confirmation needed.
- `ManualOverrideUnavailable { surface, decision }` - Override not supported on this surface.

**`EnforcedDispatcher`** (`tool/dispatcher.rs`): Wrapper around `ToolDispatcher` requiring `ApprovedOperation` before dispatch via `dispatch_checked()`. Verifies request tool name and target match the approved descriptor; fails closed on mismatch.

### Approval Methods

- `EnforcementContext::approve(surface, descriptor)` - Strict: only `Allow` produces a token. `Warn`, `RequireConfirmation`, and `Deny` all fail with `EnforcementError`. Used by REST, MCP, Agent, CI.
- `EnforcementContext::approve_manual(surface, descriptor, manual_override)` - Manual permissive: supports `Warn` (approved with warning) and `RequireConfirmation` when matching override flags are present. Strict/automated surfaces reject overrides. Used by CLI/TUI.

### Dispatch Flow

```
1. Build OperationDescriptor from OperationMetadata
2. let approved = enforcement.approve(surface, descriptor)?;
3. Build ToolRequest
4. dispatcher.dispatch_checked(&approved, request).await
```

### Adoption Status

| Surface | Status | Entry Point |
|---------|--------|-------------|
| REST | Complete | `tool/protocol/rest.rs` |
| MCP | Complete | `tool/protocol/mcp/handlers/server.rs` |
| Agent | Complete | `agent/mod.rs` |
| gRPC | Complete | `tool/protocol/grpc.rs` |
| TUI | Complete | `eggsec-tui/src/app/mod.rs` (direct-launch/high-risk tabs) |
| CLI | Via `approve_manual()` | Command handlers via `evaluate_and_enforce_operation()` |

### Non-Goals

- `evaluate()` is retained for preflight, diagnostics, and advisory checks.
- Policy semantics are unchanged.
- Raw `ToolDispatcher::dispatch()` is not removed but `EnforcedDispatcher` is required for all protocol/agent production code. Agent dispatch returns a hard invariant error if `enforced_dispatcher` is present but `ApprovedOperation` is missing.

## Audit Trail

Every meaningful enforcement decision produces a normalized `EnforcementAuditEvent` via `audit.rs`.

### Event Model

- `event_id`: UUID v4 per decision
- `timestamp`: UTC timestamp
- `surface`: `ExecutionSurface` (CliManual, TuiManual, McpServer, RestApi, SecurityAgent, Ci)
- `profile`: `ExecutionProfile` (ManualPermissive, ManualGuarded, McpStrict, AgentStrict, CiStrict)
- `operation_id`: canonical operation name
- `target`: optional target string
- `outcome`: `AuditOutcome` (Allow, Warn, Confirmed, Deny, ConfirmationRequired)
- `decision`: full `PolicyDecision` with decision_id
- `confirmation_classes`: classes required for this decision
- `manual_override`: override details (only when confirmed)
- `manual_override_ignored`: true if override flags were present but surface does not honor them
- `scope`: `ScopeAudit` with source, path, allow/exclusion counts, explicit_manifest flag
- `correlation_id`: optional request/correlation ID for REST/MCP

### Per-Surface Behavior

| Surface | Audit Emitted | Manual Override Record | Correlation ID |
|---------|--------------|----------------------|----------------|
| CLI | Yes | Accepted overrides include class+reason | None |
| TUI | Yes | Accepted overrides include class+reason | None |
| REST | Yes | Never (REST never confirms) | `generate_correlation_id()` |
| gRPC | Yes | Never (gRPC never confirms) | `uuid::Uuid::new_v4()` |
| MCP | Yes | Never (MCP never confirms) | JSON-RPC request id |
| Agent | Yes | Never (Agent never confirms) | None |
| CI | Via CLI handler | Never (CI never confirms) | None |

### Tracing Levels

- `Allow`, `Warn`, `Confirmed`: `tracing::info!`
- `Deny`, `ConfirmationRequired`: `tracing::warn!`
