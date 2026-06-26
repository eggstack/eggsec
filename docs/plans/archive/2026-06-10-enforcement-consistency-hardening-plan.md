# Enforcement Consistency Hardening Handoff Plan

Date: 2026-06-10
Repository: eggstack/eggsec
Purpose: finish the remaining consistency gaps after the strict agent/MCP enforcement correction pass. This is a narrow hardening pass, not a redesign.

## Current state

The repo is now much closer to the intended architecture:

- `ScopeSource` and `LoadedScope` exist and distinguish `DefaultEmpty`, `ConfigFile`, `CliScopeFile`, and `GeneratedPreset`.
- `load_scope_with_source()` exists and is used by `eggsec-cli`.
- `EnforcementContext` exists and is stored in `CommandContext`.
- `CommandContext::evaluate_and_enforce_operation()` routes through `self.enforcement.evaluate()`.
- MCP startup now constructs `EnforcementContext::mcp_strict(...)` and passes it into MCP router/stdio startup.
- MCP `tools/call` builds an `OperationDescriptor` and evaluates `self.enforcement` before dispatch.
- MCP profiles now set `require_explicit_scope: true`.
- MCP capability mapping exists via `required_capabilities_for_tool_call()`.
- `handle_agent()` now uses `CommandContext`, refuses to proceed without an explicit scope manifest, and passes enforcement into `AgentConfig`.

Remaining gaps are now specific:

1. `EnforcementContext::evaluate()` passes only `Some(&self.loaded_scope.scope)` into `evaluate_enforcement()`. The evaluator does not receive or enforce scope provenance centrally.
2. `evaluate_enforcement()` still immediately returns `Deny` if `evaluate_operation_policy()` denies, so `ManualPermissive` cannot intentionally downgrade low-risk scope denials into warnings.
3. `DenialClass` exists but is not load-bearing in visible evaluator logic.
4. MCP still has legacy error-reporting helpers that call `evaluate_operation_policy()` instead of `evaluate_enforcement()` and do not populate required capabilities.
5. Agent startup is guarded, but `Agent::execute_scan_with_depth()` still dispatches after operational-constraint checks without a per-scan `EnforcementContext` evaluation.
6. `McpServer::with_scope_and_profile()` still initializes default policy/default enforcement, then production startup patches in enforcement afterward. This is workable, but easy to misuse.

## Goals

- Make explicit-manifest enforcement central inside `EnforcementContext` rather than scattered at command edges.
- Make `DenialClass` actually drive manual-permissive downgrade behavior.
- Ensure MCP policy-denial responses use the same strict evaluator and capability mapping as the pre-dispatch path.
- Ensure autonomous agent scans evaluate policy/scope immediately before dispatch, not just at startup.
- Reduce footguns from constructors that silently create default MCP enforcement contexts.
- Preserve low-friction manual CLI/TUI behavior for ordinary safe operations.

## Non-goals

- Do not rewrite scanner, fuzzer, load-test, or WAF internals.
- Do not create another parallel policy engine.
- Do not make manual CLI/TUI strict by default.
- Do not remove existing public APIs unless necessary; prefer deprecation or test-only constructors if compatibility matters.
- Do not rely on documentation or prompt policy as enforcement.

## Pass 1: move explicit-manifest checks into `EnforcementContext::evaluate()`

Current issue: `EnforcementContext` stores `LoadedScope`, but `evaluate()` strips it down to `Scope` before calling `evaluate_enforcement()`. That means strict profile logic cannot centrally reason about `DefaultEmpty` versus explicit manifest.

Update `crates/eggsec/src/config/policy_decision.rs`.

Recommended implementation:

```rust
impl EnforcementContext {
    pub fn evaluate(&self, descriptor: &OperationDescriptor) -> EnforcementOutcome {
        let mut outcome = evaluate_enforcement(
            descriptor,
            &self.execution_policy,
            Some(&self.loaded_scope.scope),
            self.execution_profile,
        );

        if self.requires_explicit_manifest_for(descriptor)
            && !self.loaded_scope.is_explicit_manifest()
        {
            let decision = outcome.decision().clone()
                .with_denied_reason("explicit scope manifest required for automated networked operation");
            let mut decision = decision;
            decision.allowed = false;
            return EnforcementOutcome::Deny(decision);
        }

        outcome
    }

    pub fn requires_explicit_manifest_for(&self, descriptor: &OperationDescriptor) -> bool {
        self.execution_profile.is_automated()
            && descriptor.target.is_some()
            && descriptor.requires_explicit_scope
    }
}
```

Use clearer code than this sketch if `PolicyDecision` ownership makes this awkward.

Rules:

- `CiStrict`, `McpStrict`, and `AgentStrict` require explicit manifest for target-bearing operations where `requires_explicit_scope == true`.
- `ManualGuarded` may require explicit manifest if `descriptor.requires_explicit_scope == true`, but do not accidentally break safe manual commands without targets.
- `ManualPermissive` should not require explicit manifest unless the operation itself is hazardous or explicitly requires it.

Acceptance criteria:

- A strict profile with `LoadedScope::default_empty()` denies even if the inner `Scope` default would otherwise allow/ambiguous-pass.
- The denial reason explicitly mentions missing explicit scope manifest.
- Tests call `EnforcementContext::evaluate()`, not only `evaluate_enforcement()`.

## Pass 2: make `DenialClass` load-bearing

Current issue: `DenialClass` exists but the visible evaluator still returns immediately on base denial.

Add classification helpers near `evaluate_enforcement()`:

```rust
pub fn classify_denial_reasons(decision: &PolicyDecision) -> Vec<DenialClass> { ... }

pub fn may_downgrade_to_warning(
    descriptor: &OperationDescriptor,
    classes: &[DenialClass],
    profile: ExecutionProfile,
) -> bool { ... }
```

Suggested mapping:

- contains `"target not in scope"` => `TargetOutOfScope`
- contains `"scope file required"` => `ScopeMissing`
- contains `"excluded"` or matched exclusion rules present => `ExplicitExclusion`
- missing features not empty or contains `"required feature"` => `FeatureMissing`
- contains `"operation risk"` / `"not allowed by current execution policy"` => `RiskPolicyDenied`
- contains `"capability"` => `CapabilityDenied`
- scope parse/check errors or invalid target wording => `InvalidTarget`
- fallback => `Unknown`

Then update `evaluate_enforcement()`:

- If base decision denies and profile is `ManualPermissive`, call the classifier.
- Downgrade only if all denial classes are `ScopeMissing` or `TargetOutOfScope`, operation risk is `Passive` or `SafeActive`, mode is `StandardAssessment`, and no explicit exclusion/missing feature/risk/capability denial exists.
- Downgrade by setting `decision.allowed = true`, moving denial reasons into warnings, and returning `EnforcementOutcome::Warn(decision)`.
- For `ManualGuarded`, `CiStrict`, `McpStrict`, and `AgentStrict`, keep denials as denials.

Acceptance criteria:

- Manual permissive safe scan with no explicit manifest can warn.
- Manual permissive safe out-of-scope target can warn only when no explicit exclusion exists.
- Manual permissive does not downgrade `Intrusive`, `LoadTest`, `StressTest`, `RawPacket`, `CredentialTesting`, `ExploitAdjacent`, `RemoteExecution`, or `AgentAutonomous`.
- Manual permissive does not downgrade feature/capability/risk denials.
- Strict profiles never downgrade.

## Pass 3: enforce capabilities positively in strict profiles

Current issue: `evaluate_enforcement()` checks `denied_capabilities`, but strict automated profiles should also require explicit allow for non-baseline capabilities.

Define a small safe baseline if desired:

```rust
pub fn baseline_allowed_capability(cap: Capability) -> bool {
    matches!(cap, Capability::PassiveFingerprint | Capability::ActiveProbe | Capability::Crawl | Capability::WafDetect)
}
```

Recommended policy:

- Denied capability always denies.
- In `McpStrict` and `AgentStrict`, required capabilities outside the baseline must appear in `ExecutionPolicy.allowed_capabilities`.
- In `CiStrict`, either follow the same rule or allow safe baseline plus configured capabilities.
- In manual profiles, missing explicit capability may warn for safe baseline only, but should deny high-risk capabilities unless the corresponding risk policy flag is enabled.

High-risk capabilities that should require explicit allow in automated profiles:

- `HttpFuzzLowImpact` if it mutates or sends payloads beyond passive checks.
- `IntrusiveFuzz`
- `WafBypassSimulation`
- `WafStressTest`
- `LoadTest`
- `RawPacketProbe`
- `CredentialTesting`
- `RemoteExecution`
- `NseIntrusive`

Acceptance criteria:

- MCP stress tool denies unless `WafStressTest` is explicitly allowed and stress policy flags/features permit it.
- MCP raw-packet tool denies unless `RawPacketProbe` is explicitly allowed and feature/policy flags permit it.
- Coding-agent remains narrow by profile plus shared enforcement.
- Denial reasons identify the missing capability allow.

## Pass 4: modernize MCP policy-decision helpers

Current issue: `policy_decision_for_mcp_call()` still builds descriptors with `required_capabilities: Vec::new()` and calls `evaluate_operation_policy()` rather than shared enforcement.

Update `crates/eggsec/src/tool/protocol/mcp/policy.rs`.

Preferred design:

```rust
pub fn operation_descriptor_for_mcp_call(
    profile_policy: &McpProfilePolicy,
    tool_id: &str,
    capability: Option<&str>,
    arguments: &serde_json::Value,
) -> OperationDescriptor
```

This helper should be reused by both:

- MCP pre-dispatch enforcement in `handlers/server.rs`
- Error-reporting helpers such as `policy_decision_for_mcp_call()` / `denial_from_violation()`

Then change `policy_decision_for_mcp_call()` to accept an `&EnforcementContext` instead of separate `ExecutionPolicy` and `Scope`, or add a new helper and deprecate the old one:

```rust
pub fn policy_decision_for_mcp_call_with_enforcement(
    profile_policy: &McpProfilePolicy,
    tool_id: &str,
    capability: Option<&str>,
    arguments: &serde_json::Value,
    enforcement: &EnforcementContext,
) -> PolicyDecision
```

It should call `enforcement.evaluate(&descriptor)` and return `outcome.decision().clone()`, then append MCP-profile-specific violation reasons if needed.

Acceptance criteria:

- No MCP denial path emits a `PolicyDecision` that omits required capabilities.
- No MCP denial path uses `evaluate_operation_policy()` directly unless explicitly justified in a test-only helper.
- MCP errors are consistent with pre-dispatch enforcement decisions.

## Pass 5: add per-scan agent enforcement before dispatch

Current issue: `handle_agent()` gates startup, but `Agent::execute_scan_with_depth()` still dispatches after operational constraints without evaluating the shared enforcement context.

Update `crates/eggsec/src/agent/mod.rs`.

Inside `execute_scan_with_depth()`, before constructing or dispatching `ToolRequest`, evaluate the configured enforcement if present:

```rust
if let Some(enforcement) = &self.config.enforcement {
    let descriptor = OperationDescriptor {
        operation: scan_type.to_string(),
        mode: OperationMode::StandardAssessment,
        risk: risk_for_agent_scan_depth(depth),
        intended_uses: vec![IntendedUse::WebAssessment],
        target: Some(target.to_string()),
        required_features: Vec::new(),
        required_policy_flags: Vec::new(),
        requires_private_or_local_target: false,
        requires_explicit_scope: true,
        required_capabilities: capabilities_for_agent_scan(scan_type, depth),
    };

    match enforcement.evaluate(&descriptor) {
        EnforcementOutcome::Allow(_) => {}
        EnforcementOutcome::Warn(decision) => {
            if enforcement.execution_profile.is_automated() {
                bail!("agent strict enforcement warning treated as denial: ...");
            }
            tracing::warn!(...);
        }
        EnforcementOutcome::Deny(decision) => bail!(...),
    }
}
```

Suggested mappings:

- Shallow pipeline scan: `OperationRisk::SafeActive`; capabilities `ActiveProbe`, `Crawl`, maybe `WafDetect` if applicable.
- Deep pipeline scan: `OperationRisk::Intrusive`; capabilities `HttpFuzzLowImpact` or `IntrusiveFuzz` depending payload mutation behavior.
- Explicit load/stress/raw scan types should map to matching risks/capabilities.

Acceptance criteria:

- Agent scheduled scan cannot reach dispatcher when target is out of scope.
- Agent scheduled scan cannot reach dispatcher without explicit manifest in `AgentStrict`.
- Agent deep scan respects intrusive-fuzzing policy/capability gates.
- Test uses a fake dispatcher and proves dispatch is not called on enforcement denial.

## Pass 6: reduce MCP constructor footguns

Current issue: `McpServer::with_scope_and_profile()` still creates default execution policy/default MCP strict enforcement, and production patches in a real context afterward.

Options:

A. Add a production constructor and make it the only production route:

```rust
pub fn with_enforcement(
    registry: ToolRegistry,
    api_key: Option<String>,
    profile: McpProfile,
    enforcement: EnforcementContext,
) -> Self
```

B. Mark `with_scope_and_profile()` as test/support-only in docs and make `create_mcp_router()` / `run_stdio()` use the new constructor.

Preferred: option A.

Acceptance criteria:

- Production MCP startup does not build then patch enforcement.
- Tests can still use a simple constructor, but it should be visibly test/default-oriented.
- `McpServer::new()` should either use an inert non-network test profile or be documented as test/basic initialization only.

## Pass 7: tests

Add focused tests. Do not rely only on total test count.

Required tests:

1. `EnforcementContext::evaluate()` denies strict profile with `LoadedScope::default_empty()` for target-bearing `requires_explicit_scope` operation.
2. Same operation with explicit loaded scope and matching target allows.
3. Manual permissive downgrades safe `TargetOutOfScope`/`ScopeMissing` to warning where allowed.
4. Manual permissive does not downgrade explicit exclusion.
5. Manual permissive does not downgrade risk-policy denial.
6. Manual permissive does not downgrade feature-missing denial.
7. Manual permissive does not downgrade capability denial.
8. `DenialClass` classifier maps current denial strings correctly.
9. MCP descriptor helper includes required capabilities for stress/load/raw/fuzz/WAF tools.
10. MCP denial helper returns a `PolicyDecision` with required capability denial details.
11. MCP production constructor stores `McpStrict` enforcement profile.
12. MCP production constructor stores the configured `ExecutionPolicy`, not default policy.
13. MCP `tools/call` does not dispatch on missing explicit manifest.
14. Agent `execute_scan_with_depth()` does not dispatch when enforcement denies.
15. Agent deep scan is denied unless intrusive capability/policy allows it.
16. Agent shallow in-scope scan reaches fake dispatcher under explicit manifest.

## Pass 8: docs and comments

Update only the docs affected by this hardening:

- README safety/enforcement section.
- `docs/SAFETY.md`.
- MCP/codegg integration docs if present.
- Agent docs if present.

Clarify:

- `LoadedScope` provenance is the source of truth for strict automated manifest checks.
- `EnforcementContext` is the mandatory boundary for CLI, CI, MCP, and agent execution.
- MCP and agent strict profiles treat warnings as denials.
- Manual permissive mode only downgrades safe scope-selection misses, never feature/risk/capability/hazard denials.

## Final acceptance criteria

This pass is complete when:

- Explicit manifest requirement is enforced centrally by `EnforcementContext::evaluate()`.
- `DenialClass` is used by actual downgrade logic.
- Manual permissive behavior differs intentionally from strict mode without weakening hazardous operation gates.
- MCP denial reporting uses the same descriptor/capability/enforcement path as pre-dispatch checks.
- Agent scans evaluate shared enforcement immediately before dispatch.
- Production MCP constructors require or directly accept `EnforcementContext`.
- Tests prove denials happen before dispatch for MCP and agent.
