# Dual-Mode Enforcement Corrective Pass

## Goal

Close the remaining gaps after the first dual-mode enforcement implementation. Phases 1-5 substantially improved the repo: `ExecutionSurface` exists, CLI/MCP/agent profile selection is explicit, the security agent now rebuilds `AgentStrict`, manual discretion has regression tests, and the TUI has a first-class enforcement posture model.

The remaining work is a targeted corrective pass, not a redesign. The priority is to make sure the new architecture is applied consistently across dispatch surfaces and that the manual/agent split remains exact:

- REST must not remain a weaker parallel dispatch path.
- TUI direct-launch actions must be gated before side effects, not retroactively.
- TUI confirmation must actually satisfy the confirmation classes it claims to satisfy.
- TUI preflight display must use the active execution policy, not a default policy.
- Agent production construction should not allow missing enforcement through the public constructor.

## Current state summary

What appears good:

- `ExecutionSurface` is now the caller-origin source of truth and maps surfaces to profiles.
- CLI with `rest-api` enabled maps agent, MCP, serve, CI, strict-scope, and default manual cases explicitly.
- `CommandContext` carries both `execution_surface` and `execution_profile`, with `with_execution_surface()` rebuilding enforcement.
- Agent CLI handler rejects missing explicit scope and rebuilds `EnforcementContext::agent_strict(...)` before `AgentConfig`.
- Agent runtime validates supplied enforcement profile and re-evaluates per-scan before dispatch.
- TUI has `TuiEnforcementState`, posture toggle, preflight storage, policy confirmation overlay, and manual/guarded status.
- Manual override behavior remains narrow and tested.

Remaining gaps:

1. REST still stores raw `Option<Scope>` and dispatches directly after optional scope check.
2. TUI direct-launch tabs perform a post-dispatch/retroactive policy gate.
3. TUI `confirm_policy_action()` does not set `allow_out_of_scope` or `assume_yes` for `OutOfScope` / `TargetExpansion`, so pressing Enter can fail despite the UI treating confirmation as operator discretion.
4. TUI preflight computes confirmation classes with `Default::default()` policy in `TuiPreflightResult::from_outcome()`, which can make displayed flags/classes drift from the active policy.
5. `Agent::new(config)` permits `config.enforcement == None` as “test-only construction,” but the constructor is public and not type-restricted.

## Workstream 1: Retrofit REST with shared enforcement

### Objective

Make REST use the same `LoadedScope` + `EnforcementContext` path as MCP/agent before dispatch. REST should be strict by default for the existing programmatic API surface.

### Files likely to change

- `crates/eggsec/src/commands/handlers/serve.rs`
- `crates/eggsec/src/tool/protocol/rest.rs`
- `crates/eggsec/src/tool/mod.rs` or tool metadata helpers if needed
- `crates/eggsec/src/tool/protocol/mcp/handlers/server.rs` only if descriptor helper reuse is practical
- Tests in `rest.rs` or new integration tests under `crates/eggsec/tests/`

### Implementation steps

#### Step 1.1: Change REST state to carry enforcement context

Replace raw `scope: Option<Scope>` in `RestState` with:

```rust
pub enforcement: EnforcementContext,
```

Optionally keep raw scope only as a derived/debug field if existing code needs it, but dispatch decisions must use `enforcement.evaluate(...)`.

Update constructor:

```rust
pub fn new(
    registry: ToolRegistry,
    api_key: Option<String>,
    enforcement: EnforcementContext,
    tls_config: Option<TlsConfig>,
) -> Self
```

Update `create_router(...)` signature similarly.

#### Step 1.2: Build REST enforcement from the command context

In `handle_serve(ctx, args)`, stop loading `Scope::from_file(...)` directly into REST state.

Preferred behavior:

- If `args.scope_file` is present, load it with `load_scope_with_source(Some(scope_file))` or an equivalent helper that produces `LoadedScope` with explicit provenance.
- Else reuse `ctx.enforcement.loaded_scope.clone()`.
- Build enforcement with `ExecutionSurface::RestApi`:

```rust
let enforcement = EnforcementContext::for_surface(
    ExecutionSurface::RestApi,
    ctx.config.execution_policy.clone(),
    loaded_scope,
);
```

If `ServeArgs` has a separate `scope_file`, it should override top-level `--scope` for REST only. Document this behavior in code comments.

#### Step 1.3: Add REST operation descriptor construction

Before dispatch in `execute_tool()`, construct an `OperationDescriptor` for the REST tool call.

Preferred implementation: add a helper in REST module first, then migrate to metadata-derived descriptors later.

```rust
fn operation_descriptor_for_rest_tool(
    tool_id: &str,
    target: &str,
    payload: &ExecuteRequest,
    registry: &ToolRegistry,
) -> OperationDescriptor
```

Use the best available information:

- `operation`: `tool_id.to_string()`
- `target`: `Some(payload.target.clone())`
- `risk`: classify from existing helper if available, or mirror MCP classification logic
- `mode`: `StandardAssessment` by default; `DefenseLab` for db-pentest, traffic interception, load/stress, etc.
- `required_features`: from registry/tool metadata if available; otherwise conservative manual mapping for known feature-gated tool IDs
- `required_capabilities`: from registry tool capabilities if available; otherwise mirror MCP/classification helpers
- `requires_explicit_scope`: `true` for target-bearing REST execution

Do not block this corrective pass on the larger Phase 6 metadata refactor. A local REST helper is acceptable if it reuses existing MCP/tool classification wherever possible.

#### Step 1.4: Enforce before dispatch

In `execute_tool()`:

1. Auth.
2. Validate target and payload size.
3. Rate limit.
4. Build descriptor.
5. Evaluate `state.enforcement.evaluate(&descriptor)`.
6. Only dispatch on `Allow`.
7. Treat `Warn`, `RequireConfirmation`, and `Deny` as errors for REST.

REST is a programmatic surface, so warnings should fail closed unless an explicit future local/manual API mode is added.

Suggested mapping:

- `Allow`: proceed.
- `Warn`: `403 Forbidden` with policy warning text.
- `RequireConfirmation`: `403 Forbidden`; manual confirmation unavailable on REST strict.
- `Deny`: `403 Forbidden`.

Return a JSON error including the serialized policy decision where possible:

```json
{
  "error": "REST strict enforcement denied tool execution",
  "code": "POLICY_DENIED",
  "decision": { ... }
}
```

If `EggsecError` cannot currently carry a structured body, add a small REST-local response path rather than widening the global error type prematurely.

#### Step 1.5: Remove or demote raw scope check

Once `EnforcementContext` is authoritative, remove the old:

```rust
if let Some(ref scope) = state.scope { scope.is_target_allowed(...) }
```

or convert it into a debug assertion/helper inside descriptor evaluation. Avoid double policy with divergent behavior.

### REST tests

Add tests for:

- `RestState::new` stores `McpStrict`/strict profile when built from `ExecutionSurface::RestApi`.
- REST execution without explicit manifest denies target-bearing operations when descriptor requires explicit scope.
- REST execution with explicit scope but out-of-scope target denies.
- REST execution with positive scope match allows evaluation to reach dispatcher path.
- REST treats `RequireConfirmation` as deny.
- REST ignores manual override semantics completely.
- REST still validates auth, target format, payload size, and rate limits before dispatch.

If full axum integration tests are expensive, unit-test `operation_descriptor_for_rest_tool()` and a small extracted `evaluate_rest_policy(...)` helper.

### Acceptance criteria

- `RestState` carries `EnforcementContext` or an equivalent strict enforcement object.
- REST dispatch cannot happen without `state.enforcement.evaluate(...)`.
- REST uses `LoadedScope` provenance, not raw `Scope` alone.
- Missing explicit manifest denies target-bearing REST execution.
- `Warn` and `RequireConfirmation` do not dispatch over REST.
- Tests cover REST strict behavior.

## Workstream 2: Move TUI direct-launch policy gate before side effects

### Objective

Eliminate the current retroactive/post-dispatch gate for direct-launch tabs. Policy evaluation must happen before a tab transitions into running state, sends packets, starts a proxy, launches stress/load behavior, or performs any other side effect.

### Files likely to change

- `crates/eggsec-tui/src/app/mod.rs`
- `crates/eggsec-tui/src/app/operation.rs`
- `crates/eggsec-tui/src/app/dispatch.rs`
- `crates/eggsec-tui/src/tabs/*` for direct-launch tabs
- `crates/eggsec-tui/src/tabs/spec.rs`

### Problem to fix

`handle_enter()` currently calls the current tab dispatcher first, then checks `is_running`, builds task config/descriptor, and for direct-launch tabs runs a “post-dispatch retroactive policy gate.” That pattern is unsafe for direct-launch tabs because the tab may already have performed side effects by the time enforcement runs.

### Implementation options

#### Preferred: preflight before `dispatcher.handle_enter()` for executable tabs

In `handle_enter()`:

1. Handle overlay/dashboard/settings special cases.
2. If the current tab is executable and Enter would launch an action, build descriptor first.
3. Evaluate policy first.
4. If deny or confirmation required, block before calling `dispatcher.handle_enter()`.
5. If allow/warn, call dispatcher or spawn task.

This requires a reliable way to know whether Enter will execute versus just focus/input. If such a method does not exist, add one:

```rust
trait TabInput {
    fn enter_intent(&self) -> EnterIntent { EnterIntent::EditOrNavigate }
}

enum EnterIntent {
    EditOrNavigate,
    LaunchTask,
    DirectLaunch,
}
```

Use conservative defaults: if unknown, do not pre-gate, but direct-launch tabs must override.

#### Lower-risk alternative: split direct-launch into prepare + commit

For direct-launch tabs, change `handle_enter()` implementations so they do not perform side effects directly. Instead they return or store a `PendingDirectLaunch` / task config that `App` evaluates before calling a commit method.

This is cleaner but may touch more tabs.

### Required behavior

- No direct-launch side effect starts until enforcement returns `Allow` or `Warn`.
- `RequireConfirmation` opens the policy confirmation overlay before any side effect.
- `Deny` shows the policy error and leaves the tab not running.
- Confirmed manual override replays the action only after confirmation.

### Tests

Add tests around representative direct-launch tabs:

- Direct-launch tab with guarded/out-of-scope descriptor does not enter running state.
- Direct-launch tab with manual positive scope miss opens policy confirmation before running.
- Direct-launch tab with allow proceeds.
- Canceling policy confirmation leaves tab stopped.
- Confirming policy confirmation starts only after confirmation.

If direct-launch tab integration tests are hard, add a fake test tab implementing direct-launch semantics and route it through the same `App` handler path.

### Acceptance criteria

- The retroactive direct-launch policy gate is removed or no longer reachable for side-effecting launches.
- Direct-launch enforcement happens before `running` state and before side effects.
- Tests prove denial/confirmation do not start direct-launch actions.

## Workstream 3: Fix TUI confirmation semantics for OutOfScope / TargetExpansion

### Objective

Make TUI policy confirmation actually satisfy the same manual override classes the CLI would satisfy.

### Files likely to change

- `crates/eggsec-tui/src/app/mod.rs`
- `crates/eggsec-tui/src/app/enforcement.rs`
- Tests in TUI app/enforcement modules

### Problem

`confirm_policy_action()` treats the popup confirmation as equivalent to narrow `--yes` for `OutOfScope` and `TargetExpansion`, but it does not set `mo.assume_yes` or `mo.allow_out_of_scope`. Later it checks `mo.permits(...)`, and `ManualOverride::permits(OutOfScope)` / `TargetExpansion` requires `allow_out_of_scope || assume_yes`.

### Fix

In `confirm_policy_action()`, set:

```rust
ConfirmationClass::OutOfScope | ConfirmationClass::TargetExpansion => {
    mo.allow_out_of_scope = true;
}
```

Keep:

```rust
mo.assume_yes = false;
```

This preserves the narrow model: the TUI confirmation is explicit and class-specific, not a broad `--yes`.

### Tests

Add tests for:

- TUI pending policy confirmation for only `OutOfScope` succeeds after Enter.
- TUI pending policy confirmation for only `TargetExpansion` succeeds after Enter.
- TUI pending policy confirmation for `OutOfScope + HighRisk` sets both `allow_out_of_scope` and `allow_high_risk`.
- TUI confirmation still does not set `assume_yes`.
- TUI confirmation for unrelated classes does not accidentally permit private resolution/cross-host/nonbaseline unless those classes are required.

### Acceptance criteria

- TUI confirmation no longer fails for pure out-of-scope/target-expansion confirmation.
- TUI still does not use broad `assume_yes`.
- Tests cover the exact regression.

## Workstream 4: Fix TUI preflight policy drift

### Objective

Ensure displayed preflight classes and suggested CLI flags are derived from the active execution policy, not `ExecutionPolicy::default()`.

### Files likely to change

- `crates/eggsec-tui/src/app/enforcement.rs`
- `crates/eggsec-tui/src/app/mod.rs`

### Problem

`TuiPreflightResult::from_outcome()` calls:

```rust
confirmation_classes_for(descriptor, &decision, &Default::default())
```

That can make UI display drift from actual enforcement if the active policy has different risk/capability settings.

### Fix

Change the constructor to accept policy:

```rust
pub fn from_outcome(
    descriptor: &OperationDescriptor,
    outcome: &EnforcementOutcome,
    policy: &ExecutionPolicy,
) -> Self
```

Then call:

```rust
confirmation_classes_for(descriptor, &decision, policy)
```

Update all call sites:

- `TuiEnforcementState::preflight()` passes `&self.enforcement.execution_policy`.
- `App::evaluate_policy_and_dispatch()` passes `&self.enforcement_state.enforcement.execution_policy`.

### Additional flag fix

`ExplicitExclusion` currently maps to `--allow-out-of-scope` in `cli_flags_for_classes()`. The CLI uses `--allow-excluded-target`. Fix this mapping:

```rust
ConfirmationClass::ExplicitExclusion => "--allow-excluded-target"
```

### Tests

Add tests for:

- `from_outcome()` uses active policy to compute high-risk class behavior.
- `ExplicitExclusion` suggests `--allow-excluded-target`.
- Displayed class list matches `request_policy_confirmation()` recomputation for the same descriptor/policy.

### Acceptance criteria

- No TUI preflight code uses `Default::default()` policy for confirmation class calculation.
- Suggested CLI flags match CLI behavior.
- Tests cover policy-sensitive class calculation.

## Workstream 5: Harden agent construction API

### Objective

Prevent production code from constructing a public `Agent` without enforcement.

### Files likely to change

- `crates/eggsec/src/agent/mod.rs`
- `crates/eggsec/src/commands/handlers/agent.rs`
- Agent tests

### Problem

`Agent::new(config)` allows `config.enforcement == None` as “test-only construction,” but the constructor is public. That makes the safety invariant conventional rather than enforced by API shape.

### Preferred fix

Make `Agent::new(config)` require enforcement:

```rust
pub async fn new(config: AgentConfig) -> Result<Self> {
    let enforcement = config.enforcement.as_ref().ok_or_else(|| anyhow!(...))?;
    if enforcement.execution_profile != ExecutionProfile::AgentStrict { bail!(...) }
    ...
}
```

Move test-only permissive construction to an explicit cfg-test helper:

```rust
#[cfg(test)]
pub(crate) async fn new_for_test_without_enforcement(...)
```

or keep the existing `new_for_test(...)` as the only path that permits missing enforcement.

### Lower-impact alternative

If many tests call `Agent::new(AgentConfig::default())`, add:

```rust
#[cfg(test)]
pub async fn new_for_testing(config: AgentConfig) -> Result<Self>
```

and update tests. Production `Agent::new()` should fail closed on missing enforcement.

### Tests

Add tests for:

- `Agent::new()` rejects `None` enforcement.
- `Agent::new()` rejects `ManualPermissive` enforcement.
- `Agent::new()` accepts `AgentStrict` enforcement with explicit loaded scope.
- Test helper still allows isolated unit tests without full enforcement when needed.

### Acceptance criteria

- Public production constructor cannot create an agent without `AgentStrict` enforcement.
- Tests use explicit test helpers for no-enforcement cases.
- Existing CLI agent path still works.

## Validation checklist

Run as much of this as the feature matrix permits:

```bash
cargo fmt --all
cargo test -p eggsec --lib config::policy_decision
cargo test -p eggsec --features rest-api --lib
cargo test -p eggsec --features rest-api agent
cargo test -p eggsec-tui
cargo check -p eggsec-cli --features rest-api
cargo check -p eggsec-tui
```

If feature names differ, use the actual feature set needed for REST, MCP, agent, and TUI tabs.

Also run targeted tests for:

- REST strict denial without explicit manifest.
- REST out-of-scope denial.
- TUI direct-launch pre-dispatch denial.
- TUI out-of-scope confirmation success.
- TUI preflight policy/flag correctness.
- Agent `None` enforcement rejection.

## Non-goals

- Do not implement the full Phase 6 metadata-derived descriptor refactor.
- Do not extract domain crates.
- Do not change manual CLI default posture.
- Do not weaken MCP or agent strictness.
- Do not add a local/manual REST API mode in this pass unless strictly necessary. Existing REST should become strict by default.

## Expected final state

After this corrective pass:

- REST is no longer a policy bypass surface.
- TUI direct-launch actions are preflighted before side effects.
- TUI confirmation reliably mirrors CLI manual override semantics.
- TUI preflight display matches the active policy.
- Agent construction is fail-closed for production callers.

At that point, the repo should be ready to continue with the larger roadmap: metadata-derived operation descriptors, full enforcement matrix tests, preflight everywhere, normalized audit events, and eventually type-level enforced dispatch.
