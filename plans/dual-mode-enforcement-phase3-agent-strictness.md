> **Status: COMPLETED**

# Phase 3 Handoff Plan: Security Agent Strictness Correction

## Goal

Ensure the security agent can never inherit manual-permissive enforcement. Agent-facing execution must always use `ExecutionProfile::AgentStrict`, require explicit scope provenance for networked operations, ignore manual override flags, and re-evaluate enforcement immediately before dispatch.

This phase is the first high-priority correctness fix in the dual-mode enforcement roadmap.

## Current concern

The CLI entrypoint currently selects execution profile based on CI and `--strict-scope`, otherwise defaulting to manual-permissive. If `eggsec agent run --scope scope.toml` enters through this path and the agent handler simply clones `ctx.enforcement`, the agent can inherit a manual operator posture.

The agent handler already requires an explicit scope manifest before running, which is good, but that is not sufficient. The full enforcement context must be `AgentStrict`, not just manually checked for explicit scope.

## Desired behavior

Security-agent execution must satisfy all of these:

- The top-level CLI maps `Commands::Agent(_)` to the security-agent surface.
- The resulting enforcement profile is `ExecutionProfile::AgentStrict`.
- The agent handler defensively rebuilds an `AgentStrict` context from the current policy and loaded scope before constructing `AgentConfig`.
- The agent runtime rejects, normalizes, or fails closed when given a non-agent-strict enforcement context.
- Manual override flags are ignored and cannot produce an accepted decision.
- `RequireConfirmation` is treated as hard denial.
- Explicit scope manifest provenance comes from `LoadedScope`, not raw `Scope`.

Manual CLI/TUI behavior must not change in this phase.

## Files likely to change

Primary:

- `crates/eggsec-cli/src/main.rs`
- `crates/eggsec/src/commands/handlers/agent.rs`
- `crates/eggsec/src/agent/...` or relevant agent runtime module under `crates/eggsec/src/agent` when `rest-api` is enabled
- `crates/eggsec/src/config/policy.rs`
- `crates/eggsec/src/config/policy_decision.rs`

Tests:

- `crates/eggsec/src/commands/handlers/mod.rs` tests, if convenient.
- Dedicated `crates/eggsec/tests/agent_enforcement.rs`, if integration-style tests are clearer.
- Existing agent tests under `crates/eggsec/src/agent` or `crates/eggsec-agent` if applicable.

## Step 1: Ensure top-level surface/profile selection maps Agent to AgentStrict

If Phase 2 has already introduced `ExecutionSurface`, use:

```rust
Some(eggsec::cli::Commands::Agent(_)) => eggsec::config::ExecutionSurface::SecurityAgent,
```

and derive `ExecutionProfile::AgentStrict` from the surface.

If Phase 2 has not yet landed, use the minimal direct fix:

```rust
let execution_profile = if matches!(cli.command.as_ref(), Some(eggsec::cli::Commands::Ci(_))) {
    eggsec::config::ExecutionProfile::CiStrict
} else if matches!(cli.command.as_ref(), Some(eggsec::cli::Commands::Agent(_))) {
    eggsec::config::ExecutionProfile::AgentStrict
} else if cli.strict_scope {
    eggsec::config::ExecutionProfile::ManualGuarded
} else {
    eggsec::config::ExecutionProfile::ManualPermissive
};
```

Feature-gate the `Commands::Agent(_)` match consistently with the command definition if needed.

## Step 2: Defensively rebuild AgentStrict in `handle_agent`

In `crates/eggsec/src/commands/handlers/agent.rs`, do not pass `ctx.enforcement.clone()` directly into `AgentConfig`.

Instead, after checking scope provenance, rebuild:

```rust
let agent_enforcement = crate::config::EnforcementContext::agent_strict(
    ctx.config.execution_policy.clone(),
    ctx.enforcement.loaded_scope.clone(),
);
```

Pass `agent_enforcement` into `handle_agent_run_impl` and `AgentConfig`.

Keep the explicit manifest check:

```rust
if !ctx.enforcement.loaded_scope.is_explicit_manifest() { ... }
```

Consider changing that check to use `agent_enforcement.loaded_scope` after construction for clarity.

## Step 3: Add runtime validation in the agent constructor or run path

Find the agent runtime construction path, likely `Agent::new(config).await?` and `AgentConfig { enforcement: Some(enforcement), ... }`.

Add a validation step:

- If `config.enforcement` is `None`, fail closed for real agent execution unless the runtime has a documented test-only fallback.
- If `config.enforcement.execution_profile != ExecutionProfile::AgentStrict`, either:
  - return an error, or
  - rebuild/normalize to `AgentStrict` using the same policy and loaded scope.

Prefer returning an error in production paths because silent normalization can hide caller bugs. If many tests currently omit enforcement, add `AgentConfig::new_for_testing()` or a test helper rather than weakening production behavior.

Potential error text:

```text
security agent requires AgentStrict enforcement context; manual or guarded profiles are not accepted
```

## Step 4: Ensure immediate pre-dispatch re-evaluation

Search the agent runtime for every place it launches scans, tools, or command handlers. Before any networked or target-bearing operation, construct or obtain the relevant `OperationDescriptor` and call:

```rust
let outcome = enforcement.evaluate(&descriptor);
```

For agent execution:

- `Allow` may proceed.
- `Warn` should be treated as deny or fail closed unless there is a documented strict-safe warning class that does not indicate ambiguity. Prefer deny for now.
- `RequireConfirmation` must deny.
- `Deny` must deny.

If the current agent runtime delegates back into command handlers that already call `evaluate_and_enforce_operation`, ensure those command handlers receive an `AgentStrict` context and cannot see manual overrides.

## Step 5: Ignore manual overrides by construction

Manual overrides should never be part of agent config, portfolio, MCP request, AI output, or tool request params.

Add or verify tests that even if the top-level CLI includes flags such as:

- `--yes`
- `--allow-out-of-scope`
- `--allow-high-risk`
- `--allow-private-resolution`
- `--allow-cross-host-redirect`
- `--allow-nonbaseline-capability`

agent enforcement still denies anything that would require confirmation.

Do not merely ignore these flags silently in code comments. The behavior should be asserted.

## Step 6: Add tests

### Unit tests for profile selection

If `ExecutionSurface` exists:

- `ExecutionSurface::SecurityAgent.profile() == ExecutionProfile::AgentStrict`.
- `ExecutionSurface::SecurityAgent.honors_manual_override() == false`.
- `ExecutionSurface::SecurityAgent.requires_explicit_manifest_for_networked() == true`.

### Command-context tests

Create a context with a valid explicit scope and verify that selecting agent surface/profile results in `AgentStrict`.

### Agent handler tests

Test that `handle_agent` or the extracted helper rejects missing explicit manifest.

Test that it rebuilds/passes `AgentStrict` even when the incoming `CommandContext` is manual permissive. If this is hard to test through the async handler, extract a small helper:

```rust
fn build_agent_enforcement(ctx: &CommandContext) -> Result<EnforcementContext>
```

Then test that helper directly.

### Runtime validation tests

Test `Agent::new(config)` or relevant constructor:

- Accepts `AgentStrict` enforcement.
- Rejects `ManualPermissive` enforcement.
- Rejects missing enforcement for production config, unless an explicit test config is used.

### Override leakage tests

Construct an agent-strict context and a descriptor that would be `RequireConfirmation` in manual mode. Verify it denies in agent mode, even with manual override flags present elsewhere.

## Acceptance criteria

- `eggsec agent ...` maps to `AgentStrict` at the top-level entrypoint.
- `handle_agent()` rebuilds or enforces `AgentStrict` before constructing `AgentConfig`.
- Agent runtime fails closed if provided manual-permissive or manual-guarded enforcement.
- Agent runtime requires explicit scope manifest for networked operations.
- Manual override flags do not affect agent decisions.
- Tests cover the above behavior.
- Manual CLI/TUI default behavior is unchanged.

## Suggested validation

Run:

```bash
cargo fmt --all
cargo test -p eggsec --features rest-api agent
cargo test -p eggsec --features rest-api --lib
cargo check -p eggsec-cli --features rest-api
```

If `agent` is not a feature name, use the actual feature set that exposes `Commands::Agent` in this repo. At minimum, run with `rest-api` because the agent command appears behind that feature in the command handler table.

## Non-goals

- Do not change manual CLI/TUI confirmation behavior.
- Do not redesign `ManualOverride`.
- Do not complete REST enforcement in this phase.
- Do not implement full TUI posture UI in this phase.
- Do not add metadata-derived descriptors yet.

## Common pitfalls

- Checking only `LoadedScope::is_explicit_manifest()` is not enough; the profile must be `AgentStrict`.
- Passing `ctx.enforcement.clone()` to the agent is fragile unless the context has already been forced to `AgentStrict`.
- Silent acceptance of manual overrides in agent mode is a safety bug even if the current descriptor happens to deny.
- Do not make the global default strict as a shortcut. That would break the manual operator model.
