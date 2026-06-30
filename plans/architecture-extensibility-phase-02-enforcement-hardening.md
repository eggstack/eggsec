# Phase 2 Handoff Plan: Enforcement Invariant Hardening

## Objective

Make Eggsec's central enforcement model mechanically harder to bypass or regress. This phase should convert the Phase 1 invariant map into tests, API restrictions, and targeted call-site cleanups. The focus is strict-surface safety, manual override confinement, scope provenance, and approval-token discipline.

## Context

Eggsec now has the right enforcement primitives: `ExecutionSurface`, `ExecutionProfile`, `OperationDescriptor`, `EnforcementContext`, `EnforcementOutcome`, `ManualOverride`, and `ApprovedOperation`. The architectural concern is ensuring every side-effecting path consistently uses them, especially strict programmatic surfaces such as MCP, agent, CI, REST, and gRPC.

Manual CLI/TUI behavior is intentionally more permissive than automated behavior. Do not erase that distinction. The target is not universal hard blocking. The target is a crisp split:

- manual permissive CLI/TUI may warn or require explicit operator confirmation/override;
- manual guarded CLI/TUI denies discretionary cases;
- MCP, agent, CI, REST, and gRPC fail closed and never honor manual overrides.

## Deliverables

1. Add enforcement matrix tests covering surfaces, profiles, risks, capabilities, scope provenance, and manual override handling.

2. Audit and restrict raw dispatch paths so strict surfaces use `ApprovedOperation` or equivalent enforced dispatch.

3. Deprecate, remove, or quarantine transitional helpers that allow command code to substitute local scope checks for central descriptor enforcement.

4. Remove or quarantine direct profile-setting APIs where surface-derived profile selection is possible.

5. Add regression tests proving automated surfaces ignore manual override flags.

6. Add regression tests proving automated target-bearing operations that require explicit scope fail without explicit manifest provenance.

7. Add regression tests proving approval tokens cannot be reused for a different tool or target.

8. Document any intentionally retained exceptions in `docs/ARCHITECTURE.md` or a dedicated enforcement notes section.

## Primary files and modules

Inspect and modify as needed:

- `crates/eggsec/src/config/policy.rs`
- `crates/eggsec/src/config/policy_decision.rs`
- `crates/eggsec/src/config/scope.rs`
- `crates/eggsec/src/commands/handlers/mod.rs`
- `crates/eggsec/src/tool/dispatcher.rs`
- `crates/eggsec/src/tool/registry.rs`
- `crates/eggsec/src/tool/implementations/**`
- `crates/eggsec/src/commands/handlers/*`
- `crates/eggsec-cli/src/main.rs`
- `crates/eggsec-tui/src/app/enforcement.rs`
- `crates/eggsec-tui/src/app/mod.rs`
- MCP/agent/API modules under feature gates

## Enforcement behavior matrix

Add tests for the following cases. Prefer unit tests close to policy code for pure enforcement behavior and integration-style tests near command/tool dispatch for call-path behavior.

### Manual permissive CLI/TUI

- Passive and safe-active standard assessment operations allow or warn as currently intended.
- Missing/ambiguous scope for safe low-risk manual operations can downgrade to warning only when allowed by the current rules.
- Explicit allowlist miss with positive scope rules requires confirmation, not silent warn.
- Explicit exclusions are not downgraded to warning.
- `--yes` or `assume_yes` only permits low-risk classes that are intentionally supported.
- Dedicated override flags are required for private resolution, cross-host redirects, high risk, non-baseline capability, traffic interception, and explicit exclusions.
- Accepted manual overrides are audited and visible in `PolicyDecision`/audit records.

### Manual guarded CLI/TUI

- Manual guarded surfaces do not honor manual overrides.
- `RequireConfirmation` outcomes fail closed.
- Scope ambiguity and out-of-scope cases deny where guarded semantics require denial.

### MCP / agent / CI / REST / gRPC

- Manual override flags do not affect approval.
- `Warn` does not approve strict dispatch.
- `RequireConfirmation` does not approve strict dispatch.
- `Deny` remains deny.
- Networked operations with `requires_explicit_scope = true` deny when `LoadedScope` is default/implicit.
- Explicit manifest provenance allows evaluation to proceed to the normal policy/capability checks.
- Non-baseline capabilities require explicit `allowed_capabilities` in strict profiles.
- `denied_capabilities` wins over allowed capabilities.

### Approval token tests

- A token approved for one operation cannot dispatch a different tool ID.
- A token approved for one target cannot dispatch a request for another target.
- Operation aliases are resolved only through the canonical operation metadata matcher.
- Raw dispatch remains inaccessible from strict-surface code or is confined to a private/internal path with explicit comments.

## Raw dispatch audit

Search for direct calls to raw dispatch or internal tool execution. Identify all call sites that do any of the following:

- call `ToolDispatcher::dispatch` directly;
- execute tool implementations without preflight;
- perform network side effects inside a command handler before descriptor evaluation;
- call domain execution functions before policy approval;
- use direct scope helpers as the only guard.

For each call site, classify it:

- safe pure/local operation;
- already approved before call;
- should be converted to `EnforcedDispatcher`;
- should build an `OperationDescriptor` and call `CommandContext::evaluate_and_enforce_operation`;
- should be documented exception.

Prefer small, surgical fixes. Do not restructure the entire tool layer in this phase.

## Transitional API cleanup

### `CommandContext::with_execution_profile`

If possible, replace internal test and production uses with `with_execution_surface`. If immediate removal is too invasive, mark it clearly as test-only or deprecated with comments and add a TODO referencing this plan.

The desired invariant is: entrypoints choose `ExecutionSurface`; `ExecutionSurface` derives `ExecutionProfile`.

### `CommandContext::ensure_scope` and `ensure_scope_url`

Audit use sites. These helpers can remain for pure validation/display, but they must not be the sole authorization gate for side-effecting operations. Where they guard execution, replace or supplement them with descriptor-based enforcement.

### manual override propagation

Ensure manual override state is built only from CLI/TUI manual controls and does not appear in MCP/agent request types. If any programmatic request schema exposes manual override fields, remove or ignore them with tests.

## Implementation steps

1. Read Phase 1 architecture inventory and use its execution path table as the worklist.

2. Add or extend policy unit tests for `ExecutionSurface`, `ExecutionProfile`, `ManualOverride`, `may_downgrade_to_warning`, `evaluate_enforcement`, and explicit manifest checks.

3. Add dispatcher tests for `EnforcedDispatcher::dispatch_checked` mismatch behavior.

4. Search for raw dispatch and classify all call sites.

5. Convert strict-surface raw dispatch to enforced dispatch where feasible.

6. Audit command handlers for side effects before enforcement. Fix the highest-risk cases first: stress, packet, proxy intercept, db, mobile dynamic, wireless advanced, evasion, postex, C2, MCP/agent.

7. Audit direct scope helper uses and convert execution authorization to descriptors where needed.

8. Replace production use of `with_execution_profile` with `with_execution_surface` where feasible.

9. Update architecture docs with any retained exceptions.

10. Run validation commands and record failures honestly.

## Validation commands

Run at minimum:

```bash
cargo fmt --all --check
cargo check --workspace --no-default-features
cargo test -p eggsec --lib
cargo test -p eggsec-tool-core --lib
cargo test -p eggsec-tui --lib
```

Also run relevant feature-gated checks where available:

```bash
cargo check -p eggsec --features rest-api,tool-api
cargo check -p eggsec --features db-pentest
cargo check -p eggsec --features web-proxy
cargo check -p eggsec --features stress-testing,packet-inspection
```

If a feature combination is platform-sensitive, document the reason and run the closest safe subset.

## Non-goals

Do not redesign operation metadata yet. That belongs to Phase 3 and Phase 4.

Do not broadly extract domains yet. That belongs to Phase 5.

Do not change the intended manual CLI/TUI permissive model.

Do not expose new MCP tools.

Do not make feature gates the only safety mechanism; runtime policy must remain central.

## Acceptance criteria

- Enforcement matrix tests cover manual permissive, manual guarded, and strict automated surfaces.
- Strict programmatic surfaces cannot use manual override flags to proceed.
- Automated operations requiring explicit scope fail without explicit manifest provenance.
- Approval-token mismatch tests cover operation/tool mismatch and target mismatch.
- Raw dispatch use is either eliminated from strict paths or explicitly documented as safe/internal.
- Direct scope helpers are no longer the only authorization gate for side-effecting operations.
- The codebase still supports the manual CLI/TUI discretion model.

## Handoff notes for Phase 3

Phase 3 should build on the hardened invariants by defining a domain module contract. The contract should assume that domains declare metadata and execute work, while central enforcement approves operations before execution. Any call-site patterns discovered during this phase should inform the domain contract design.
