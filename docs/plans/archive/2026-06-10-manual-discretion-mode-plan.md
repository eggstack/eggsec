# Manual CLI/TUI Discretion Mode Handoff Plan

Date: 2026-06-10
Repository: eggstack/eggsec
Purpose: adjust manual CLI/TUI behavior so it matches mature operator-directed security tools: scope guidance, warnings, confirmation, and audit in manual mode; strict immutable enforcement remains mandatory for CI, MCP, and autonomous agent modes.

## Context

The recent enforcement work correctly tightened MCP and autonomous-agent paths:

- MCP and agent execution use `EnforcementContext` as the sole policy/scope authority.
- Scope provenance flows through `LoadedScope`.
- Automated target-bearing operations require explicit scope manifests.
- MCP/agent tool calls are evaluated before dispatch.
- Non-baseline capabilities are gated by `ExecutionPolicy.allowed_capabilities` in strict profiles.

That is the right boundary for agents and MCP. However, manual CLI/TUI operation should not feel more restrictive than comparable legitimate tools. For direct human operation, Eggsec should provide strong scope assistance and explicit risk acknowledgement, not agent-style hard stops for every out-of-scope or high-risk condition.

The desired posture:

- Manual CLI/TUI default: operator-directed, warn/confirm/audit.
- Manual strict (`--strict-scope`): hard enforcement.
- CI/MCP/Agent: hard enforcement, unchanged.

## Goals

- Shift manual CLI/TUI default from hard policy boundaries toward user discretion where appropriate.
- Preserve hard denials for correctness/runtime impossibilities and automated execution.
- Add a `RequireConfirmation` outcome for manual operations that should not silently proceed but should be operator-overridable.
- Add explicit manual override flags for CLI.
- Prepare TUI to show confirmation prompts instead of failing immediately.
- Keep MCP/agent unable to synthesize or pass manual overrides.
- Preserve auditability by recording what was overridden.

## Non-goals

- Do not weaken MCP, CI, or autonomous-agent enforcement.
- Do not remove `EnforcementContext`, `LoadedScope`, `ExecutionProfile`, or capability mapping.
- Do not make dangerous operations silent in manual mode.
- Do not bypass compile-time feature gates or unsupported operation checks.
- Do not add agent-visible override fields.

## Desired behavior matrix

### ManualPermissive, default CLI/TUI

Default manual mode should behave like mature operator-driven tooling:

- Missing scope: warning, proceed for safe operations.
- Ambiguous target: warning, proceed for safe operations.
- Target not in configured allowlist: warning or confirmation depending whether the allowlist was explicit.
- Explicit exclusion: confirmation required, not silent proceed.
- High-risk operation: confirmation required.
- Non-baseline capability: confirmation required.
- Cross-host redirect / private DNS resolution / target expansion: confirmation required.
- Missing compile-time feature / unsupported operation / malformed target: hard deny.

### ManualGuarded, `--strict-scope`

Strict manual mode remains hard-enforcing:

- Missing explicit scope: deny.
- Out-of-scope target: deny.
- Explicit exclusion: deny.
- High-risk capability not allowed by policy: deny.
- Scope ambiguity: deny.

### CI/MCP/Agent

Unchanged:

- Treat warning/confirmation states as denial.
- Explicit scope manifest required for target-bearing operations.
- No manual override path.
- No agent- or MCP-visible confirmation fields.

## Pass 1: add `RequireConfirmation` to `EnforcementOutcome`

Target: `crates/eggsec/src/config/policy_decision.rs`.

Change:

```rust
pub enum EnforcementOutcome {
    Allow(PolicyDecision),
    Warn(PolicyDecision),
    Deny(PolicyDecision),
}
```

to:

```rust
pub enum EnforcementOutcome {
    Allow(PolicyDecision),
    Warn(PolicyDecision),
    RequireConfirmation(PolicyDecision),
    Deny(PolicyDecision),
}
```

Update helper methods:

- `decision()` includes `RequireConfirmation`.
- `is_allowed()` should remain true only for `Allow` and `Warn`.
- Add `requires_confirmation()`.
- `is_denied()` remains true only for `Deny`.

Rationale: `RequireConfirmation` is not allowed by itself. It is a manual-only intermediate state. CLI/TUI may convert it to proceed if an explicit manual override is present. CI/MCP/Agent must treat it as denial.

Acceptance criteria:

- Exhaustive matches compile.
- Automated profiles never proceed on `RequireConfirmation`.

## Pass 2: define manual override model

Add a manual override struct in a policy/config-adjacent module, preferably `policy_decision.rs` or a small `manual_override.rs` under `config`.

Suggested type:

```rust
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ManualOverride {
    pub assume_yes: bool,
    pub allow_out_of_scope: bool,
    pub allow_explicit_exclusion: bool,
    pub allow_high_risk: bool,
    pub allow_nonbaseline_capability: bool,
    pub allow_private_resolution: bool,
    pub allow_cross_host_redirect: bool,
    pub reason: Option<String>,
}
```

Add a helper:

```rust
impl ManualOverride {
    pub fn permits(&self, class: ConfirmationClass) -> bool { ... }
}
```

Suggested `ConfirmationClass` enum:

```rust
pub enum ConfirmationClass {
    OutOfScope,
    ExplicitExclusion,
    HighRisk,
    NonBaselineCapability,
    PrivateResolution,
    CrossHostRedirect,
    TargetExpansion,
}
```

Keep this struct available only to manual CLI/TUI command handling. Do not add it to MCP request types, agent config, or tool request serialization.

Acceptance criteria:

- Manual override model is not part of MCP protocol schema.
- Manual override model is not part of autonomous-agent config.

## Pass 3: classify confirmation-worthy conditions

Extend the current denial/capability logic so manual permissive can return `RequireConfirmation` instead of `Deny` for operator-discretion cases.

Recommended helper:

```rust
pub fn confirmation_classes_for(
    descriptor: &OperationDescriptor,
    decision: &PolicyDecision,
    policy: &ExecutionPolicy,
) -> Vec<ConfirmationClass>
```

Conditions:

- `TargetOutOfScope` with explicit positive scope rules => `OutOfScope`.
- `ExplicitExclusion` => `ExplicitExclusion`.
- `OperationRisk::{Intrusive, LoadTest, StressTest, RawPacket, CredentialTesting, ExploitAdjacent, RemoteExecution}` => `HighRisk`.
- Required capability not baseline and not explicitly allowed => `NonBaselineCapability`.
- Scope warning or resolver signal that public input resolves to private/loopback => `PrivateResolution`.
- Redirect/canonicalization signal to different host/scheme/security boundary => `CrossHostRedirect`.
- Discovered target expansion outside original input => `TargetExpansion`.

Some of these signals may not exist yet. Implement the cases that are currently represented by `PolicyDecision`, `Scope`, or `OperationDescriptor`, and leave TODOs/tests for resolver/redirect-specific signals if not yet wired.

Acceptance criteria:

- ManualPermissive high-risk operation returns `RequireConfirmation`, not immediate hard `Deny`, when runtime support exists.
- ManualGuarded and automated profiles still return `Deny`.

## Pass 4: revise `evaluate_enforcement()` manual semantics

Current manual behavior already downgrades safe scope misses to `Warn`. Keep that.

Update `evaluate_enforcement()` logic:

- Missing compile-time feature: always `Deny`.
- Invalid target/malformed config/impossible runtime: always `Deny`.
- `ManualPermissive`:
  - safe missing scope / ambiguous scope => `Warn`.
  - explicit allowlist miss with positive scope rules => `RequireConfirmation(OutOfScope)`.
  - explicit exclusion => `RequireConfirmation(ExplicitExclusion)`.
  - non-baseline capability => `RequireConfirmation(NonBaselineCapability)`.
  - high-risk operation if feature/runtime exists => `RequireConfirmation(HighRisk)`.
- `ManualGuarded`, `CiStrict`, `McpStrict`, `AgentStrict`:
  - keep hard `Deny` behavior.

Important: preserve hard denials for operations not compiled in. User discretion cannot make unavailable code exist.

Acceptance criteria:

- Existing strict tests continue passing.
- New manual tests assert `RequireConfirmation` where prior behavior was too hard.

## Pass 5: CLI flags for manual overrides

Target: `crates/eggsec/src/cli/mod.rs` and command context setup.

Add global manual-only flags:

```rust
#[arg(long, global = true)]
pub yes: bool,

#[arg(long, global = true)]
pub allow_out_of_scope: bool,

#[arg(long, global = true)]
pub allow_excluded_target: bool,

#[arg(long, global = true)]
pub allow_high_risk: bool,

#[arg(long, global = true)]
pub allow_nonbaseline_capability: bool,

#[arg(long, global = true)]
pub manual_override_reason: Option<String>,
```

Naming can be adjusted for consistency with existing CLI style. Prefer specific flags over a single broad bypass.

Rules:

- These flags are honored only for `ExecutionProfile::ManualPermissive`.
- `--strict-scope`, CI, MCP, and agent ignore or reject them.
- `--yes` can satisfy confirmation prompts only for classes already permitted by specific flags, or can be documented as broad manual confirmation if that fits project style. Prefer requiring specific flags for high-risk/excluded target.

Acceptance criteria:

- CLI help makes clear these are manual-only.
- MCP/agent commands cannot use these flags to weaken enforcement.

## Pass 6: wire manual override into `CommandContext`

Target: `crates/eggsec/src/commands/handlers/mod.rs` and `eggsec-cli/src/main.rs`.

Add to `CommandContext`:

```rust
pub manual_override: ManualOverride,
```

Add constructor method:

```rust
pub fn with_manual_override(mut self, manual_override: ManualOverride) -> Self
```

Update `evaluate_and_enforce_operation()`:

```rust
match outcome {
    Allow(decision) => Ok(decision),
    Warn(decision) => { log warnings; Ok(decision) }
    RequireConfirmation(decision) => {
        if self.execution_profile != ExecutionProfile::ManualPermissive {
            bail deny
        }
        if self.manual_override.permits_all_required(&decision) {
            audit/log override;
            Ok(decision.with_manual_override_record(...))
        } else {
            bail with message explaining required flags
        }
    }
    Deny(decision) => bail
}
```

You may need to add confirmation metadata to `PolicyDecision` to avoid parsing strings. If too large, use denial/warning strings in the first pass, then refactor.

Acceptance criteria:

- Manual CLI can proceed with explicit override flags.
- Without override flags, CLI explains exactly which flag is required.
- Strict/CI/MCP/agent cannot proceed on `RequireConfirmation`.

## Pass 7: audit/manual override record

Extend `PolicyDecision` or add a companion audit event to record manual overrides.

Suggested fields in `PolicyDecision`:

```rust
pub manual_override_used: bool,
pub manual_override_reason: Option<String>,
pub manual_override_classes: Vec<String>,
```

If expanding `PolicyDecision` is undesirable, emit a structured tracing event:

```rust
tracing::warn!(
    operation = %decision.operation,
    target = ?decision.target_original,
    classes = ?classes,
    reason = ?manual_override.reason,
    "manual enforcement override accepted"
);
```

Acceptance criteria:

- JSON output includes enough information to tell when an override occurred, or logs do.
- Override reason is preserved if provided.

## Pass 8: TUI confirmation design hook

If the TUI currently routes through `CommandContext`, prepare it to treat `RequireConfirmation` differently.

Minimum implementation:

- Add a typed return/error path that distinguishes hard denial from confirmation required.
- TUI can show a modal with:
  - operation
  - target
  - confirmation classes
  - warnings/denied reasons
  - buttons: Proceed once, Cancel, Switch to strict mode

Do not implement a full UI if that is too large. Add enough type plumbing so future TUI work does not parse string errors.

Acceptance criteria:

- TUI/manual code can distinguish `RequireConfirmation` from `Deny`.
- Agent/MCP code never receives this as an actionable prompt.

## Pass 9: tests

Add focused tests.

Policy/enforcement tests:

1. ManualPermissive safe missing scope returns `Warn`.
2. ManualPermissive out-of-scope with explicit positive scope returns `RequireConfirmation`.
3. ManualPermissive explicit exclusion returns `RequireConfirmation`.
4. ManualPermissive high-risk operation returns `RequireConfirmation` if feature/runtime available.
5. ManualPermissive missing feature returns `Deny`.
6. ManualGuarded out-of-scope returns `Deny`.
7. McpStrict/AgentStrict high-risk without policy returns `Deny`.
8. Non-baseline capability in ManualPermissive returns `RequireConfirmation`.
9. Non-baseline capability in McpStrict returns `Deny` unless explicitly allowed.

Command context tests:

10. `RequireConfirmation` without manual override fails with required flag message.
11. `RequireConfirmation` with matching manual override succeeds.
12. Override logs or decision records manual override class/reason.
13. Manual override flags are ignored or rejected for CI/MCP/agent profiles.

CLI tests, if existing harness supports them:

14. CLI help includes manual-only override flags.
15. Safe manual scan without scope warns but proceeds.
16. Manual high-risk command requires override flag.
17. Strict mode denies even with manual override flags.

## Pass 10: docs

Update:

- README safety/enforcement section.
- `docs/SAFETY.md`.
- CLI docs/help examples.
- TUI docs if present.

Document the mode split explicitly:

```text
Manual CLI/TUI default is operator-directed: Eggsec warns, prompts, and audits.
Manual strict / CI / MCP / Agent are enforcement-directed: Eggsec denies unsafe or out-of-scope operations.
```

Example commands:

```bash
eggsec scan example.com
# warns if no scope, proceeds for safe active scan

eggsec scan example.com --strict-scope --scope scope.toml
# hard-enforces scope

eggsec waf-stress https://lab.example --allow-high-risk --manual-override-reason "authorized Synvoid WAF regression"
# manual-only explicit override; audited
```

Clearly state that manual override flags are not available to MCP/autonomous agent execution.

## Pass 11: validation

Run targeted checks:

```bash
cargo fmt --all
cargo test -p eggsec --lib enforcement
cargo test -p eggsec --lib scope
cargo test -p eggsec --lib commands
cargo test -p eggsec --lib mcp
cargo test -p eggsec --lib agent
```

Then run the repo’s normal validation. If feasible:

```bash
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

If full workspace all-features is too expensive or blocked by known feature combinations, list exact AGENTS quick-ref checks in the commit message.

## Final acceptance criteria

This pass is complete when:

- ManualPermissive can return `RequireConfirmation` for operator-discretion cases.
- CLI has explicit manual-only override flags.
- `CommandContext` can convert `RequireConfirmation` to proceed only for manual profiles with matching override.
- Manual strict, CI, MCP, and agent remain hard-deny paths.
- Missing feature/runtime impossibilities remain hard denials.
- Overrides are auditable.
- Docs clearly describe manual discretion versus automated enforcement.

After this pass, Eggsec should align better with legitimate mature tools: low-friction for direct human operators, strict and fail-closed for automated agents.