# Phase 4 Handoff Plan: Preserve Manual CLI/TUI Discretion

## Goal

Add regression coverage and small ergonomics improvements that preserve Eggsec's manual operator model. Manual CLI/TUI usage should remain practical, comparable to other legitimate security tools, and distinct from agent-strict execution.

This phase protects against the opposite failure mode from Phase 3. Phase 3 prevents agents from inheriting manual permissiveness. Phase 4 prevents manual users from inheriting agent-grade strictness by accident.

## Desired behavior

Default manual CLI/TUI operation should support:

- Safe active or passive operations with missing/ambiguous scope producing warnings when appropriate.
- Explicit operator confirmation for discretion cases.
- Narrow `--yes` behavior for low-risk prompt suppression only.
- Dedicated `--allow-*` flags for higher-risk or sensitive confirmation classes.
- Audit records when a manual override is accepted.
- Clear error text explaining which flag/class is needed.

Default manual CLI/TUI operation should not require agent-grade explicit manifests for every useful operation.

Manual strict/guarded mode remains available for operators who want hard enforcement.

## Files likely to change

Primary:

- `crates/eggsec/src/config/policy_decision.rs`
- `crates/eggsec/src/commands/handlers/mod.rs`
- `crates/eggsec-cli/src/main.rs`
- `crates/eggsec-tui/src/app/...` only for small status text if needed
- `docs/ENFORCEMENT_MODES.md` if Phase 1 landed and minor clarifications are needed

Tests:

- Existing tests in `policy_decision.rs`.
- Existing command-context tests in `commands/handlers/mod.rs`.
- New `crates/eggsec/tests/manual_discretion.rs` if integration-style tests are clearer.

## Manual-mode behavior matrix to protect

### Safe ambiguity

Scenario: manual default, safe active/passive operation, target-bearing, no explicit positive scope rules.

Expected: `Warn` or allowed with warning, not hard denial.

Examples:

- Passive recon against a user-supplied domain.
- Safe active scan with no explicit scope file where current policy allows safe active.

### Positive allowlist miss

Scenario: manual default, explicit positive scope rules exist, target misses allowlist.

Expected: `RequireConfirmation`, not silent warn and not immediate hard denial.

Reason: the operator supplied positive scope, so a miss is meaningful. But in manual mode the operator can explicitly accept responsibility.

Required override: `--allow-out-of-scope` or equivalent TUI confirmation.

### Explicit exclusion

Scenario: manual default, target matches explicit exclusion.

Expected: follow the Phase 1 contract. Prefer one of these two policies and make it consistent:

1. Conservative: hard deny explicit exclusions in all modes.
2. Operator-discretion: `RequireConfirmation` only in manual default, requiring `--allow-excluded-target` plus audit reason.

Do not allow explicit exclusion via `--yes`.

### High-risk operation

Scenario: manual default, policy permits the risk class, operation is high-risk such as load/stress/raw packet/db pentest/web proxy.

Expected: `RequireConfirmation` with a dedicated high-risk or capability-specific flag.

Required flags should stay specific where possible:

- `--allow-high-risk` for generic high-risk.
- `--allow-db-pentest` for db-pentest where applicable.
- `--allow-web-proxy` for traffic interception.

### Nonbaseline capability

Scenario: manual default, operation requires nonbaseline capability.

Expected: `RequireConfirmation`, requiring `--allow-nonbaseline-capability` or a more specific flag if the class has one.

### Private resolution

Scenario: public-looking target resolves to private/loopback/misdirected address.

Expected: `RequireConfirmation`, requiring `--allow-private-resolution`.

Explicitly targeting a private IP should be informational or scope-dependent, not automatically treated as DNS-rebinding-style private resolution.

### Cross-host redirect

Scenario: operation follows or discovers redirect/canonical host outside original target.

Expected: `RequireConfirmation`, requiring `--allow-cross-host-redirect`.

### Target expansion

Scenario: crawler/recon/discovery expands beyond original target.

Expected: manual default may require confirmation and `--allow-out-of-scope`/target expansion approval depending on scope state.

### `--yes`

Expected:

- May cover low-risk `OutOfScope` and `TargetExpansion` prompts where current policy allows.
- Must not cover `HighRisk`, `PrivateResolution`, `CrossHostRedirect`, `NonBaselineCapability`, `TrafficInterception`, or `ExplicitExclusion`.

## Implementation steps

### Step 1: Review current confirmation-class logic

Inspect:

- `confirmation_classes_for(...)`
- `ManualOverride::permits(...)`
- `may_downgrade_to_warning(...)`
- `evaluate_enforcement(...)`
- `CommandContext::evaluate_and_enforce_operation(...)`

Confirm that current logic matches the Phase 1 contract. Do not loosen hard-deny classes such as missing features, invalid target, denied capability, or risk-policy-denied.

### Step 2: Add focused unit tests for policy outcomes

Add tests directly around `evaluate_enforcement(...)` because they are cheap and deterministic.

Recommended test helpers:

- `scope_empty()`
- `scope_allow_localhost()`
- `scope_allow_example_exclude_admin()`
- `descriptor_safe(target)`
- `descriptor_high_risk(target)`
- `descriptor_nonbaseline(target, cap)`
- `policy_default()`
- `policy_allow_high_risk()`

Required tests:

1. `manual_safe_missing_scope_warns_not_denies`.
2. `manual_positive_scope_miss_requires_confirmation`.
3. `guarded_positive_scope_miss_denies`.
4. `mcp_positive_scope_miss_denies`.
5. `agent_positive_scope_miss_denies`.
6. `manual_yes_permits_only_out_of_scope_or_target_expansion`.
7. `manual_yes_does_not_permit_high_risk`.
8. `manual_yes_does_not_permit_private_resolution`.
9. `manual_specific_private_resolution_flag_permits_private_resolution_confirmation`.
10. `manual_specific_cross_host_redirect_flag_permits_redirect_confirmation`.
11. `manual_nonbaseline_requires_specific_override`.
12. `manual_missing_feature_hard_denies`.
13. `manual_denied_capability_hard_denies`.
14. `manual_risk_policy_denied_hard_denies`.

### Step 3: Add CommandContext override tests

Policy-level tests prove outcome classification. `CommandContext` tests prove CLI manual override wiring.

Required tests:

- Manual permissive + positive scope miss + no override -> error mentioning needed flag.
- Manual permissive + positive scope miss + `allow_out_of_scope` -> ok and `manual_override_used == true`.
- Manual permissive + high risk + `assume_yes == true` only -> error explaining `--yes` is insufficient.
- Manual permissive + high risk + `allow_high_risk` -> ok and audit classes include high-risk.
- Manual guarded + same override flags -> still denial.
- AgentStrict + same override flags -> still denial.

### Step 4: Improve error text if needed

Manual errors should explain the class and the exact flag(s) needed. Existing logic already produces flag suggestions; preserve or sharpen it.

Expected qualities:

- Says `manual confirmation required for: high-risk`.
- Names needed flags.
- Explains that `--yes` alone does not permit non-low-risk classes.
- Does not imply the action is impossible when a manual override is intentionally supported.
- Does not suggest manual override flags in MCP/agent/CI/strict contexts.

### Step 5: TUI status text only if cheap

If the TUI already displays enforcement status/preflight information, make sure manual-default text says something like:

- `Manual mode: warnings and explicit confirmations are available.`
- `Guarded/agent mode: confirmations are denials.`

Do not implement the full TUI posture model in this phase. That belongs to Phase 5.

## Acceptance criteria

- Manual CLI default still permits safe ambiguous operations with warnings where intended.
- Manual CLI default maps positive allowlist misses to confirmation rather than silent warn or hard deny.
- Manual override flags work only for matching confirmation classes.
- `--yes` remains narrow.
- Strict profiles ignore manual overrides.
- MCP/agent/CI behavior remains strict.
- Tests lock all of the above behavior.
- No change makes default TUI/CLI require agent-grade explicit manifests for every normal manual workflow.

## Suggested validation

Run:

```bash
cargo fmt --all
cargo test -p eggsec --lib config::policy_decision
cargo test -p eggsec --lib commands::handlers
cargo test -p eggsec-cli
cargo check -p eggsec-tui
```

If test names or module paths differ, run the equivalent targeted policy/command tests plus a broader `cargo test -p eggsec --lib`.

## Non-goals

- Do not weaken MCP or security-agent enforcement.
- Do not make explicit scope optional for agent-controlled execution.
- Do not implement REST strictness here.
- Do not implement metadata-derived descriptors here.
- Do not add the full enforcement matrix yet; this phase is focused on manual-mode regression tests.

## Common pitfalls

- Do not convert risk-policy denials into manual confirmations. If policy says the risk class is not allowed, manual override should not bypass that unless the policy and CLI flag model intentionally supports it.
- Do not let `--yes` become a blanket override.
- Do not make missing feature or invalid target errors confirmable.
- Do not make manual mode silently proceed through explicit positive-scope misses without confirmation.
- Do not use agent-strict logic as the default TUI/CLI path.
