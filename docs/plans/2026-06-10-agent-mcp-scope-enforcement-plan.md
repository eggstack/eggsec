# Agent/MCP Scope Enforcement Handoff Plan

Date: 2026-06-10
Repository: eggstack/eggsec
Primary goal: split manual CLI/TUI scope ergonomics from strict MCP/autonomous-agent enforcement, using the current `Scope`, `ExecutionPolicy`, `OperationDescriptor`, command context, and tool dispatcher architecture.

## Context

Eggsec already has the right foundational pieces for scope-aware execution:

- Workspace split includes `eggsec`, `eggsec-cli`, `eggsec-tui`, `eggsec-tool-core`, `eggsec-agent`, and optional integration crates.
- `crates/eggsec/src/config/scope.rs` defines `Scope`, `ScopeRule`, target allow/exclude matching, port allow/exclude checks, URL validation, private IP classification, and scope-file parsing.
- `crates/eggsec/src/config/policy.rs` defines `ExecutionPolicy`, `OperationRisk`, `OperationMode`, `IntendedUse`, and `OperationDescriptor`.
- `crates/eggsec/src/config/policy_decision.rs` defines `PolicyDecision` and `evaluate_operation_policy()`.
- `crates/eggsec/src/commands/handlers/mod.rs` defines `CommandContext` and `evaluate_and_enforce_operation()`.
- `crates/eggsec/src/tool/mod.rs` defines the programmatic tool registry used by MCP/REST/API adapters.
- `crates/eggsec/src/commands/handlers/serve.rs` starts REST/MCP surfaces.

The current gap is that policy evaluation is binary and does not distinguish caller trust context. Manual CLI/TUI usage should mostly mirror mature pentesting tools: scope assistance, warnings, dry-run planning, exclusions, and opt-in strict mode. MCP/autonomous/agent execution should be strict, deny-by-default, and impossible for the model/tool caller to bypass.

This plan should be implemented as a focused hardening/refactor pass, not as a broad rewrite.

## Desired behavior

Manual CLI/TUI default behavior:

- Familiar, low-friction operation.
- Scope files are supported and visible, but safe/manual operations can warn instead of hard-denying unless `--strict-scope` is set.
- Explicit exclusions should still be treated as strong denials unless the user gives a deliberately named human-only override.
- Hazardous operations remain gated by feature flags and explicit execution policy.
- Dry-run/plan/policy-explain/scope-explain remain first-class.

Manual strict behavior:

- `--strict-scope` or equivalent TUI setting makes manual behavior closer to CI/MCP enforcement.
- Missing scope for target-networked operations denies.
- Out-of-scope targets deny.
- External redirects deny.
- Scope ambiguity denies unless explicitly overridden by a human CLI/TUI flag.

CI behavior:

- Noninteractive, deterministic, strict.
- Scope warnings should be configurable as failure.
- Missing scope for networked assessment should deny.

MCP behavior:

- Always strict.
- Scope manifest required for any networked tool.
- No model/tool-call-provided flag can downgrade enforcement to warn-only.
- All tool execution must pass through the same enforcement context before scanner/fuzzer/loadtest/WAF/recon code runs.

Autonomous agent behavior:

- Always strict.
- Scope manifest required.
- Discovery can produce pending candidates, but cannot silently expand allowed scope.
- The agent cannot approve its own exceptions.
- Policy changes/exceptions must come from a human-controlled path.

## Implementation pass 1: introduce execution profile and enforcement outcome

Add a new module or extend `crates/eggsec/src/config/policy.rs` / `policy_decision.rs` with caller-trust semantics.

Recommended new enum:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExecutionProfile {
    ManualPermissive,
    ManualGuarded,
    CiStrict,
    McpStrict,
    AgentStrict,
}
```

Recommended new outcome type:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnforcementOutcome {
    Allow(PolicyDecision),
    Warn(PolicyDecision),
    Deny(PolicyDecision),
}
```

Keep `OperationMode` as the semantic category of the operation: `StandardAssessment`, `DefenseLab`, `HazardousLab`.

Use `ExecutionProfile` for the caller trust boundary: manual human, guarded manual, CI, MCP, or autonomous agent.

Do not overload `OperationMode` to mean both operation semantics and caller type.

Add a new evaluator wrapper:

```rust
pub fn evaluate_enforcement(
    descriptor: &OperationDescriptor,
    policy: &ExecutionPolicy,
    scope: Option<&Scope>,
    profile: ExecutionProfile,
) -> EnforcementOutcome
```

This function can call the existing `evaluate_operation_policy()` internally at first, then transform the resulting `PolicyDecision` into `Allow`, `Warn`, or `Deny` according to `ExecutionProfile`.

Initial mapping:

- `ManualPermissive`: allow safe operations; convert nonfatal scope ambiguity into warnings; deny hazardous policy failures, missing feature gates, and explicit exclusions.
- `ManualGuarded`: deny missing required scope, out-of-scope targets, explicit exclusions, and risky operations without policy approval.
- `CiStrict`: deny missing scope, out-of-scope targets, risky operations without approval, and optionally warnings depending on config.
- `McpStrict`: deny missing scope, out-of-scope targets, missing capability declarations, policy failures, feature-gate failures, redirect violations, and ambiguous target resolution.
- `AgentStrict`: same as MCP, plus disallow scope expansion through discoveries unless promoted by a human/manifest path.

## Implementation pass 2: wire `ExecutionProfile` into `CommandContext`

Modify `crates/eggsec/src/commands/handlers/mod.rs`.

Add to `CommandContext`:

```rust
pub execution_profile: ExecutionProfile,
```

Update constructor behavior:

- `CommandContext::new(...)` should default to `ExecutionProfile::ManualPermissive`.
- Add `with_execution_profile(profile: ExecutionProfile) -> Self`.
- Preserve existing APIs where possible to avoid a large call-site churn.

Update `evaluate_and_enforce_operation()` so it calls `evaluate_enforcement()` instead of directly using `evaluate_operation_policy()`.

Expected behavior:

- `Allow(decision)` returns `Ok(decision)`.
- `Warn(decision)` logs warnings and returns `Ok(decision)` in manual permissive mode.
- `Deny(decision)` preserves current JSON/human-readable error behavior.

In JSON mode, warning outcomes should still be serializable and visible. Avoid making warnings invisible in machine-readable output.

## Implementation pass 3: CLI/TUI controls for manual ergonomics

Add manual-only controls:

- `--strict-scope` as a global CLI flag or applicable command flag.
- Optionally `--scope-warn-only` if useful, but do not allow this for CI/MCP/agent paths.

Preferred default:

- Normal manual CLI/TUI: `ManualPermissive`.
- Manual CLI/TUI with `--strict-scope`: `ManualGuarded`.
- CI command: `CiStrict`.
- MCP command: forced `McpStrict`.
- Agent command: forced `AgentStrict`.

Important: MCP/agent code must not respect any user/model-supplied downgrade flag such as warn-only. If such a flag is present in a shared args struct, explicitly ignore or reject it in MCP/agent startup.

Update the TUI scope display if practical:

- Current scope mode.
- Scope file path, if loaded.
- Warning count / denial count.
- Pending discovered targets.

This TUI work is secondary to the MCP enforcement path.

## Implementation pass 4: fix MCP enforcement boundary

Current target file: `crates/eggsec/src/commands/handlers/serve.rs`.

Current issue:

- `handle_mcp_serve()` creates a default registry and launches MCP stdio/router using only registry, API key, and MCP profile.
- It does not pass the current `CommandContext`, `Scope`, or `ExecutionPolicy` into the MCP execution path.

Required change:

- Construct an enforcement context from `ctx.config.execution_policy`, `ctx.scope`, and `ExecutionProfile::McpStrict`.
- Pass that enforcement context into the MCP execution path.
- Prefer dispatcher-level enforcement over registry-level enforcement.

Recommended shape:

```rust
let enforcement = EnforcementContext::new(
    ctx.config.execution_policy.clone(),
    ctx.scope.clone(),
    ExecutionProfile::McpStrict,
);

let dispatcher = ToolDispatcher::new(registry).with_enforcement(enforcement);
```

Then MCP stdio/router should receive the dispatcher, not a raw unguarded registry.

If changing MCP adapters to accept `ToolDispatcher` is too large for one pass, add a wrapper around `ToolRegistry` execution that enforces before dispatch. The invariant is more important than the exact type shape:

Every MCP-exposed tool execution must pass through policy/scope enforcement before any network-capable module runs.

## Implementation pass 5: add capability-aware operation descriptors

The current `OperationRisk` enum is useful but too coarse for agent/MCP policy. Add explicit operation capabilities.

Recommended enum, likely in `config/policy.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Capability {
    PassiveFingerprint,
    ActiveProbe,
    Crawl,
    HttpFuzzLowImpact,
    IntrusiveFuzz,
    WafDetect,
    WafBypassSimulation,
    WafStressTest,
    LoadTest,
    RawPacketProbe,
    CredentialTesting,
    RemoteExecution,
    NseSafe,
    NseIntrusive,
}
```

Extend `OperationDescriptor`:

```rust
#[serde(default)]
pub required_capabilities: Vec<Capability>,
```

Extend `ExecutionPolicy` or create a dedicated capability section:

```rust
#[serde(default)]
pub allowed_capabilities: Vec<Capability>,

#[serde(default)]
pub denied_capabilities: Vec<Capability>,
```

Suggested semantics:

- Explicit deny wins over allow.
- For `ManualPermissive`, missing capability can warn for safe/manual classes but deny hazardous classes.
- For `McpStrict` and `AgentStrict`, missing explicit capability denies.
- Unknown capability denies in MCP/agent modes.

Update tool metadata to expose capabilities. Start with the registered default tools in `crates/eggsec/src/tool/mod.rs`:

- recon: `PassiveFingerprint` / possibly `ActiveProbe` depending on subcommand.
- scanner ports: `ActiveProbe`.
- scanner fingerprint: `PassiveFingerprint` or `ActiveProbe` depending behavior.
- scanner endpoints: `Crawl` / `ActiveProbe`.
- fuzzer: `HttpFuzzLowImpact` or `IntrusiveFuzz` depending args.
- loadtest: `LoadTest`.
- waf detect: `WafDetect`.
- waf bypass: `WafBypassSimulation`.
- waf stress: `WafStressTest`.
- pipeline: aggregate from selected profile.
- search: should normally be non-network or external-query scoped separately; classify carefully.

Do not attempt perfect granularity in the first pass. The important part is to build the descriptor pathway and set conservative defaults for MCP/agent.

## Implementation pass 6: strict discovery promotion model

For agent/MCP mode, discovered targets must not silently become authorized targets.

Add a `DiscoveredTargetStatus` model either in the tool/session/state layer or config/policy layer:

```rust
pub enum DiscoveredTargetStatus {
    Candidate,
    PendingApproval,
    ApprovedInScope,
    RejectedOutOfScope,
}
```

Initial behavior:

- Manual permissive mode may warn and optionally allow continuing.
- MCP/agent modes record discovered targets as pending candidates.
- Only explicitly configured wildcard/CIDR scope or a human approval path may promote candidate targets to approved scope.

This does not need a full UI in the first pass. A structured output field and audit record is sufficient.

## Implementation pass 7: tests

Add focused tests before broad refactors.

Suggested unit tests:

1. Same in-scope target under all profiles returns allow.
2. Same out-of-scope target:
   - `ManualPermissive` returns warn for safe operation.
   - `ManualGuarded` returns deny.
   - `CiStrict` returns deny.
   - `McpStrict` returns deny.
   - `AgentStrict` returns deny.
3. Explicitly excluded target denies under all profiles unless a human-only override is explicitly implemented.
4. Missing scope file:
   - manual safe operation warns.
   - MCP/agent networked operation denies.
5. Load/stress/raw-packet operation denies unless the relevant policy flag and feature gate are present.
6. Missing capability denies in MCP/agent mode.
7. Unknown capability denies in MCP/agent mode.
8. JSON denial output still includes `allowed:false`, `decision_id`, `operation_risk`, and `denied_reasons`.
9. Warning output is visible in JSON mode.
10. MCP dispatcher cannot execute a networked tool without an `EnforcementContext`.

Suggested integration-ish tests:

- `policy-explain` continues to work.
- `scope-explain` continues to work.
- Manual `scan` without `--strict-scope` does not regress into surprising hard denials for ordinary safe usage.
- `codegg-mcp` / MCP profile startup fails early or refuses network tools if no scope is loaded.

## Documentation updates

Update `docs/SAFETY.md` and README safety sections after implementation.

Document the split clearly:

- Manual CLI/TUI: scope-assisted by default; strict mode available.
- CI: strict by default.
- MCP: strict by default and scope manifest required.
- Autonomous agent: strict by default and scope manifest required.

Document examples:

```bash
eggsec scan example.com --profile quick
```

Manual permissive, familiar behavior.

```bash
eggsec scan example.com --profile quick --scope scope.toml --strict-scope
```

Manual strict behavior.

```bash
eggsec codegg-mcp --scope scope.toml --stdio
```

Strict MCP behavior.

```bash
eggsec agent run --portfolio portfolio.json --scope scope.toml
```

Strict autonomous behavior.

Also document that agent/MCP callers cannot use warn-only or downgrade flags.

## Non-goals for this pass

Do not rewrite all scanner/fuzzer/loadtest internals.

Do not attempt a full sandbox implementation.

Do not remove existing `Scope`, `ExecutionPolicy`, or `PolicyDecision` types.

Do not make manual CLI behavior more restrictive than mature comparable tools unless `--strict-scope` or a hazardous profile is selected.

Do not rely on prompt-level instructions for MCP/agent safety. Enforcement must be in Rust code paths.

## Acceptance criteria

The pass is complete when:

- `ExecutionProfile` exists and is used by command/MCP/agent execution paths.
- Manual CLI defaults to permissive/warning-oriented scope behavior for ordinary safe operations.
- `--strict-scope` or equivalent makes manual CLI strict.
- CI is strict.
- MCP is strict and cannot run networked tools without scope enforcement.
- Autonomous agent is strict and cannot run networked tools without scope enforcement.
- `handle_mcp_serve()` no longer launches a raw unguarded registry path for tool execution.
- At least the high-value tests above exist and pass.
- README/docs explain the distinction between manual scope assistance and agent/MCP scope enforcement.

## Suggested implementation order

1. Add `ExecutionProfile` and `EnforcementOutcome`.
2. Add `evaluate_enforcement()` as a wrapper around existing `evaluate_operation_policy()`.
3. Add `execution_profile` to `CommandContext`.
4. Wire manual/CI/MCP/agent profile selection.
5. Fix MCP dispatcher/registry enforcement path.
6. Add tests for profile-specific outcomes.
7. Add capability declarations to descriptors and high-risk tools.
8. Add docs updates.

Prioritize MCP/agent enforcement over CLI polish. The user-facing CLI/TUI ergonomics matter, but the immediate risk is any programmatic tool surface that can bypass the existing command context enforcement.
