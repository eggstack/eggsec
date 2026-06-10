# Strict Agent/MCP Enforcement Correction Plan

Date: 2026-06-10
Repository: eggstack/eggsec
Purpose: make the newly added execution-profile, enforcement-outcome, and capability-policy abstractions load-bearing in the actual MCP and autonomous-agent execution paths.

## Current state summary

The previous implementation pass added the right foundational types:

- `ExecutionProfile::{ManualPermissive, ManualGuarded, CiStrict, McpStrict, AgentStrict}` in `crates/eggsec/src/config/policy.rs`.
- `Capability` and `OperationDescriptor::required_capabilities`.
- `ExecutionPolicy::{allowed_capabilities, denied_capabilities}`.
- `EnforcementOutcome::{Allow, Warn, Deny}` and `evaluate_enforcement()` in `crates/eggsec/src/config/policy_decision.rs`.
- `CommandContext::execution_profile` and `CommandContext::evaluate_and_enforce_operation()` wiring.
- CLI `--strict-scope` and CI profile selection in `crates/eggsec-cli/src/main.rs`.
- MCP now receives `Scope` in `handle_mcp_serve()` and stores scope inside `McpServer`.

However, the strict programmatic boundary is not complete. MCP and autonomous-agent paths still do not consistently use `ExecutionProfile::McpStrict` / `ExecutionProfile::AgentStrict`, and the repo still does not distinguish an explicit user-supplied scope manifest from an empty default scope.

This plan is a corrective pass. Avoid adding another parallel policy system. The goal is to make the existing new abstractions unavoidable at execution choke points.

## Main problems to correct

1. `handle_mcp_serve()` passes `Scope` into MCP, but does not pass the configured `ExecutionPolicy`, does not force `ExecutionProfile::McpStrict`, and does not construct a shared enforcement context.

2. `McpServer::with_scope_and_profile()` initializes `execution_policy` to `ExecutionPolicy::default()`. There is a `with_execution_policy()` method, but the production MCP startup path does not call it.

3. `policy_decision_for_mcp_call()` calls `evaluate_operation_policy()`, not `evaluate_enforcement(..., ExecutionProfile::McpStrict)`, so MCP decisions bypass the new profile-aware semantics.

4. `McpProfilePolicy::ops_agent()` is still permissive: `require_explicit_scope: false`, `allow_external_network: true`, `allow_stress_testing: true`, and `allow_packet_features: true`. Ops-agent may have broader capabilities than coding-agent, but it is still an MCP/model-facing surface and must require an explicit scope manifest for networked operations.

5. `McpProfilePolicy::coding_agent()` has `require_explicit_scope: false`. That should not be true for any networked MCP tool.

6. `CommandContext` stores a non-optional `Scope`. Since `load_scope(None)` produces a default scope, strict paths cannot reliably tell the difference between “no scope manifest supplied” and “explicit empty scope.”

7. `evaluate_enforcement()` currently calls `evaluate_operation_policy()` and immediately returns `Deny` if the base decision is denied. This prevents `ManualPermissive` from downgrading intended low-risk scope misses into warnings. Manual permissive mode is therefore less distinct from strict mode than intended.

8. Autonomous agent handling ignores `CommandContext`. `handle_agent(_ctx, ...)` does not force `AgentStrict`, does not require a scope manifest, and does not pass scope/policy into `AgentConfig`.

9. Capability enforcement is not yet load-bearing. `policy_decision_for_mcp_call()` constructs descriptors with `required_capabilities: Vec::new()`, so `Capability` mostly exists as schema rather than as an enforcement input.

## Desired end state

Manual CLI/TUI:

- Defaults to familiar, low-friction behavior for ordinary safe operations.
- `--strict-scope` activates guarded behavior.
- Explicit exclusions remain strong denials.
- Hazardous operations remain gated by policy and feature flags.

CI:

- Uses `CiStrict`.
- Requires explicit scope for networked assessment.
- Fails deterministically on denials.

MCP:

- Always uses `McpStrict` internally.
- Requires explicit scope manifest for any network-capable tool call.
- Uses the configured `ExecutionPolicy`, not `ExecutionPolicy::default()`.
- Calls the shared enforcement evaluator before dispatch.
- Cannot be downgraded by model/tool-call arguments.

Autonomous agent:

- Always uses `AgentStrict` internally.
- Requires explicit scope manifest before running scheduled/autonomous networked assessments.
- Uses the configured `ExecutionPolicy` and explicit scope manifest.
- Cannot approve its own discovered-target expansions.

## Pass 1: represent loaded scope provenance explicitly

Problem: `Scope` alone cannot tell whether the user supplied a real manifest.

Add a scope provenance wrapper, likely in `crates/eggsec/src/config/scope.rs` or a new `config/loaded_scope.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ScopeSource {
    DefaultEmpty,
    ConfigFile,
    CliScopeFile,
    GeneratedPreset,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadedScope {
    pub scope: Scope,
    pub source: ScopeSource,
    pub path: Option<String>,
}

impl LoadedScope {
    pub fn is_explicit_manifest(&self) -> bool {
        matches!(
            self.source,
            ScopeSource::ConfigFile | ScopeSource::CliScopeFile | ScopeSource::GeneratedPreset
        )
    }
}
```

Update `load_scope()` or add `load_scope_with_source()` so callers can distinguish:

- No `--scope` and no config-provided scope: `DefaultEmpty`.
- `--scope path`: `CliScopeFile`.
- Config path or profile-generated scope: appropriate explicit source.

Prefer preserving the existing `load_scope()` API for compatibility, but add a new API for strict enforcement paths:

```rust
pub fn load_scope_with_source(path: Option<&str>) -> Result<LoadedScope, ScopeError>
```

Update `CommandContext`:

```rust
pub loaded_scope: LoadedScope,
```

Either replace `pub scope: Scope` or keep `scope` temporarily for compatibility and add `loaded_scope`. If both exist, ensure they cannot diverge.

Acceptance criteria:

- Strict paths can ask `ctx.loaded_scope.is_explicit_manifest()`.
- Tests cover no-scope vs explicit-scope-file behavior.

## Pass 2: make `evaluate_enforcement()` profile-aware before final denial

Problem: `evaluate_enforcement()` currently cannot downgrade selected manual-permissive scope misses because the base evaluator has already set `allowed = false`.

Refactor carefully. Do not make hazardous operations permissive.

Recommended approach:

- Keep `evaluate_operation_policy()` as the strict base evaluator for legacy/strict paths.
- Add a richer internal evaluation representation, or teach `evaluate_enforcement()` to distinguish denial classes.
- At minimum, classify denial reasons before returning.

Suggested helper:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DenialClass {
    ScopeMissing,
    TargetOutOfScope,
    ExplicitExclusion,
    FeatureMissing,
    RiskPolicyDenied,
    CapabilityDenied,
    InvalidTarget,
    Unknown,
}
```

ManualPermissive should only downgrade low-risk `TargetOutOfScope` or `ScopeMissing` cases when all of the following are true:

- Operation risk is `Passive` or `SafeActive`.
- Operation is not `DefenseLab` or `HazardousLab`.
- No explicit exclusion matched.
- No missing feature gate.
- No denied capability.
- No credential, raw-packet, stress, exploit-adjacent, or remote-execution risk.

Strict profiles deny all denial classes.

Acceptance criteria:

- Manual permissive safe scan with no explicit scope can warn.
- Manual guarded denies the same case.
- MCP strict denies the same case.
- Explicit exclusion denies in all profiles.
- Feature/risk/capability denial stays denial in all profiles.

## Pass 3: create a real `EnforcementContext`

Add a reusable context object in `config/policy_decision.rs` or a new `config/enforcement.rs`:

```rust
#[derive(Debug, Clone)]
pub struct EnforcementContext {
    pub execution_profile: ExecutionProfile,
    pub execution_policy: ExecutionPolicy,
    pub loaded_scope: LoadedScope,
}

impl EnforcementContext {
    pub fn manual_permissive(policy: ExecutionPolicy, loaded_scope: LoadedScope) -> Self;
    pub fn manual_guarded(policy: ExecutionPolicy, loaded_scope: LoadedScope) -> Self;
    pub fn ci_strict(policy: ExecutionPolicy, loaded_scope: LoadedScope) -> Self;
    pub fn mcp_strict(policy: ExecutionPolicy, loaded_scope: LoadedScope) -> Self;
    pub fn agent_strict(policy: ExecutionPolicy, loaded_scope: LoadedScope) -> Self;

    pub fn require_explicit_scope_for_networked(&self) -> bool;
    pub fn evaluate(&self, descriptor: &OperationDescriptor) -> EnforcementOutcome;
}
```

Use this context anywhere a tool-call surface can execute networked work.

Acceptance criteria:

- Command handlers can use `ctx.enforcement.evaluate(...)`.
- MCP can receive `EnforcementContext` directly.
- Agent can receive `EnforcementContext` directly.
- No strict path relies on `ExecutionPolicy::default()` unless tests explicitly construct it.

## Pass 4: wire `CommandContext` around `EnforcementContext`

Update `CommandContext` to contain one enforcement object instead of separate partial fields where possible:

```rust
pub enforcement: EnforcementContext,
```

`CommandContext::evaluate_and_enforce_operation()` should call:

```rust
let outcome = self.enforcement.evaluate(&descriptor);
```

Update `crates/eggsec-cli/src/main.rs`:

- Use `load_scope_with_source(cli.scope.as_deref())`.
- Select profile:
  - `Ci` command => `CiStrict`.
  - `--strict-scope` => `ManualGuarded`.
  - otherwise => `ManualPermissive`.
- Build `EnforcementContext` from actual config policy and loaded scope.

Acceptance criteria:

- Manual and CI paths use the same enforcement object that MCP/agent will use.
- Existing tests for `CommandContext` are updated.

## Pass 5: force MCP through `McpStrict`

Update `crates/eggsec/src/commands/handlers/serve.rs`:

- `handle_mcp_serve(ctx, args)` must build or clone `ctx.enforcement` but force profile to `McpStrict`.
- It must reject startup or reject all networked tool calls if `loaded_scope.is_explicit_manifest() == false`.
- It must pass configured `ExecutionPolicy`, loaded scope, and profile into MCP.

Recommended production shape:

```rust
let enforcement = EnforcementContext::mcp_strict(
    ctx.config.execution_policy.clone(),
    ctx.loaded_scope.clone(),
);

let server = McpServer::with_enforcement(registry, args.api_key.clone(), profile, enforcement);
```

Update MCP route creation and stdio APIs:

```rust
pub async fn create_mcp_router(
    registry: ToolRegistry,
    api_key: Option<String>,
    profile: McpProfile,
    enforcement: EnforcementContext,
) -> Router

pub async fn run_stdio(
    registry: ToolRegistry,
    api_key: Option<String>,
    profile: McpProfile,
    enforcement: EnforcementContext,
)
```

Update `McpServer`:

- Replace `scope: Option<Scope>` and `execution_policy: ExecutionPolicy` with a single `enforcement: EnforcementContext` where possible.
- If keeping old fields temporarily, ensure `with_scope_and_profile()` is test-only or deprecated.
- Do not default production MCP to `ExecutionPolicy::default()`.

Acceptance criteria:

- MCP startup cannot silently use default policy when CLI config supplied a different policy.
- MCP has an unavoidable `ExecutionProfile::McpStrict` internally.
- MCP strict enforcement can see whether scope came from an explicit manifest.

## Pass 6: use shared enforcement inside MCP `tools/call`

Update `crates/eggsec/src/tool/protocol/mcp/handlers/server.rs`.

Before `self.dispatcher.dispatch(request).await`, build an `OperationDescriptor` from:

- Resolved tool ID.
- Target argument.
- Classified operation risk.
- Intended use based on MCP profile.
- Required features if known.
- Required capabilities inferred from tool metadata/capability name/tool ID.
- `requires_explicit_scope = true` for network-capable tools.

Then call:

```rust
let outcome = self.enforcement.evaluate(&descriptor);
if let EnforcementOutcome::Deny(decision) = outcome { ... }
```

For MCP, `Warn` should not proceed unless explicitly decided otherwise. Simpler and safer: in MCP strict mode, `Warn` should be converted to denial. The evaluator should generally not emit `Warn` for `McpStrict`.

Keep `McpProfilePolicy::validate_tool_call()` as an additional MCP-profile filter, but do not let it replace shared enforcement. The order should be:

1. Resolve tool ID and metadata.
2. Validate MCP profile-specific visibility and argument caps.
3. Build `OperationDescriptor`.
4. Evaluate shared enforcement with `McpStrict`.
5. Run target scope check/canonicalization.
6. Dispatch.

Acceptance criteria:

- `policy_decision_for_mcp_call()` uses `evaluate_enforcement(..., McpStrict)` or is replaced by the shared enforcement context.
- Denial responses include structured `PolicyDecision` data.
- No MCP tool call can reach `dispatcher.dispatch()` without shared enforcement.

## Pass 7: tighten MCP profile defaults

Update `crates/eggsec/src/tool/protocol/mcp/policy.rs`.

Recommended defaults:

For both `OpsAgent` and `CodingAgent`:

```rust
require_explicit_scope: true
```

For `OpsAgent`:

- It may retain broader tool visibility than coding-agent.
- It may allow stress/packet features only if the runtime `ExecutionPolicy` and explicit scope manifest allow them.
- Do not let `allow_external_network: true` bypass the scope manifest requirement.
- Consider `default_target_policy: ExplicitScopeOnly` or `AnyWithScopeEngine` only if shared enforcement guarantees explicit manifest + target match.

For `CodingAgent`:

- Keep narrow tools.
- Keep `allow_external_network: false` unless the explicit scope manifest authorizes the target.
- Keep stress/load/raw/packet denied unless explicitly needed for a coding-agent validation workflow.

Acceptance criteria:

- MCP profile metadata reports explicit scope required.
- Ops-agent cannot call public/external targets without explicit scope match.
- Coding-agent cannot call public/external targets without explicit scope match.

## Pass 8: make capabilities load-bearing

Add a helper, likely in MCP policy or tool metadata conversion:

```rust
pub fn required_capabilities_for_tool_call(
    tool_id: &str,
    capability_name: Option<&str>,
    arguments: &serde_json::Value,
) -> Vec<Capability>
```

Initial mapping:

- `recon` => `PassiveFingerprint` or `ActiveProbe` depending args.
- `scan`, `scan-ports`, `fingerprint` => `ActiveProbe` / `PassiveFingerprint` as appropriate.
- `endpoints`, `scan-endpoints` => `Crawl`.
- `fuzz`, `api-fuzz` => `HttpFuzzLowImpact` or `IntrusiveFuzz` depending mode/profile.
- `load`, `loadtest`, `http-bench` => `LoadTest`.
- `waf-detect` => `WafDetect`.
- `waf-bypass` => `WafBypassSimulation`.
- `waf-stress`, `stress` => `WafStressTest` / `LoadTest` / `RawPacketProbe` depending tool.
- `packet`, `raw-packet` => `RawPacketProbe`.
- `auth-test`, `credential` => `CredentialTesting`.
- `exec`, `remote`, `ssh` => `RemoteExecution`.
- `nse-safe` => `NseSafe`.
- intrusive NSE categories => `NseIntrusive`.

Update `policy_decision_for_mcp_call()` / descriptor builder to populate `required_capabilities`.

Update `evaluate_enforcement()`:

- Explicit denied capability always denies.
- In `McpStrict` / `AgentStrict`, any required capability not explicitly allowed by policy should deny, except for a small built-in safe baseline if intentionally chosen.
- In `ManualPermissive`, missing capability may warn only for passive/safe-active operations.

Be careful: if you make every safe capability require explicit allow, existing manual usage may become noisy. Keep the strict requirement limited to automated profiles.

Acceptance criteria:

- MCP call to stress tool is denied unless `WafStressTest` or relevant capability is allowed and scope is explicit.
- MCP call to raw-packet tool is denied unless `RawPacketProbe` is allowed and feature/policy gates pass.
- Capability denials appear in `PolicyDecision.denied_reasons`.

## Pass 9: wire autonomous agent through `AgentStrict`

Update `crates/eggsec/src/commands/handlers/agent.rs`.

Current issue: `handle_agent(_ctx, ...)` ignores context.

Required changes:

- Rename `_ctx` to `ctx` and use it.
- Build `EnforcementContext::agent_strict(ctx.config.execution_policy.clone(), ctx.loaded_scope.clone())`.
- Refuse `agent run` if no explicit scope manifest is loaded.
- Pass enforcement into `AgentConfig` and then into `Agent`.

Example additions:

```rust
pub struct AgentConfig {
    ...
    pub enforcement: Option<EnforcementContext>,
}
```

Then inside the agent runtime, before any scheduled assessment/tool execution, build an `OperationDescriptor` and call `enforcement.evaluate(...)`.

If full runtime enforcement is too large for this pass, implement an immediate conservative gate:

- Agent run refuses to start without explicit scope manifest.
- Agent target portfolio entries are validated against scope at load/start time.
- Any out-of-scope target disables that target and logs a structured denial.

Acceptance criteria:

- `eggsec agent run` without explicit scope fails before running scans.
- `eggsec agent run --scope scope.toml` validates portfolio targets against scope.
- Out-of-scope portfolio target does not run.
- Agent code no longer ignores `CommandContext`.

## Pass 10: discovery promotion remains pending-only for automated profiles

If `DiscoveredTargetStatus` already exists, verify it is used. If it does not exist in production paths, add minimal use.

Automated profiles:

- Discovery may produce candidates.
- Candidates are not automatically inserted into allowed scope.
- Candidates must be `PendingApproval` unless they already match explicit scope.
- Only human/config/manifest path can promote to `ApprovedInScope`.

Acceptance criteria:

- Tests prove discovered external target is not auto-approved in MCP/agent mode.
- Structured output or logs identify pending discoveries.

## Pass 11: tests

Add/repair focused tests. Prefer small unit tests around enforcement plus a few MCP handler tests.

Required tests:

1. `load_scope_with_source(None)` returns `DefaultEmpty` and `is_explicit_manifest() == false`.
2. `load_scope_with_source(Some(path))` returns `CliScopeFile` and `is_explicit_manifest() == true`.
3. `ManualPermissive` can warn for safe low-risk missing-scope case.
4. `ManualGuarded` denies the same case.
5. `CiStrict` denies the same case.
6. `McpStrict` denies the same case.
7. `AgentStrict` denies the same case.
8. Explicit exclusion denies in all profiles.
9. Missing feature gate denies in all profiles.
10. Risk policy denial denies in all profiles.
11. Denied capability denies in all profiles.
12. MCP server constructed from production route has `ExecutionProfile::McpStrict`.
13. MCP route/stdio uses configured execution policy, not default policy.
14. MCP `tools/call` without explicit scope manifest denies networked tools.
15. MCP `tools/call` with explicit scope but out-of-scope target denies.
16. MCP `tools/call` with explicit in-scope target can proceed for safe tools.
17. MCP stress/raw/packet tools deny unless policy/capability allows them.
18. Agent run without explicit scope manifest fails early.
19. Agent run validates portfolio targets against scope.
20. JSON denial includes `decision_id`, `allowed:false`, `operation_risk`, and `denied_reasons`.

Do not rely only on commit messages claiming tests pass. The tests should exercise production constructors/routes where practical.

## Pass 12: documentation cleanup

Update:

- `README.md`
- `docs/SAFETY.md`
- `docs/AGENT.md`
- MCP/codegg integration docs if present
- Any architecture doc that currently implies MCP is already strictly enforced

Document the corrected model:

- Manual CLI/TUI: scope-assisted by default; `--strict-scope` available.
- CI: strict.
- MCP: strict, explicit scope manifest required for networked tools.
- Agent: strict, explicit scope manifest required.
- Ops-agent may be broader than coding-agent in capability set, but not weaker in scope enforcement.

Include examples:

```bash
eggsec scan example.com --profile quick
```

Manual permissive.

```bash
eggsec scan example.com --profile quick --scope scope.toml --strict-scope
```

Manual guarded.

```bash
eggsec codegg-mcp --stdio --scope scope.toml
```

MCP strict.

```bash
eggsec agent run --scope scope.toml --portfolio portfolio.json
```

Agent strict.

## Non-goals

Do not rewrite the scanner/fuzzer/loadtest internals.

Do not create a second parallel policy system.

Do not remove `Scope`, `ExecutionPolicy`, `OperationDescriptor`, `PolicyDecision`, `ExecutionProfile`, or `EnforcementOutcome`.

Do not make manual CLI/TUI universally strict by default.

Do not rely on prompt instructions, README claims, or MCP profile descriptions as enforcement.

Do not treat passing scope alone as sufficient. Strict automated paths require explicit scope provenance, configured execution policy, execution profile, and shared enforcement before dispatch.

## Final acceptance criteria

This corrective pass is complete when:

- Scope provenance exists and strict paths can distinguish explicit manifest from default empty scope.
- `EnforcementContext` or equivalent exists and is used by CLI, CI, MCP, and agent paths.
- MCP production startup forces `McpStrict` and uses configured `ExecutionPolicy`.
- MCP tool calls cannot reach dispatcher without shared enforcement.
- Both MCP profiles require explicit scope for networked tools.
- Agent run forces `AgentStrict` and refuses to run networked assessments without explicit scope.
- Capability declarations are populated for MCP calls and affect enforcement.
- Manual CLI still supports low-friction permissive behavior for ordinary safe operations.
- Tests cover the strict/programmatic boundary, not only type serialization.
- Docs accurately describe the distinction between manual scope assistance and automated strict enforcement.
