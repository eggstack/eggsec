# MCP Legacy Field Removal Handoff Plan

Date: 2026-06-10
Repository: eggstack/eggsec
Purpose: remove the remaining MCP legacy/test compatibility fields and deprecated helper paths now that production MCP execution uses `EnforcementContext` as the sole policy/scope authority.

## Current state

The final enforcement cleanup pass left the architecture in a good state:

- Production MCP startup uses `McpServer::with_enforcement(...)`.
- `create_mcp_router(...)` and `run_stdio(...)` no longer accept a separate `scope: Option<Scope>`.
- `handle_tools_call()` uses `operation_descriptor_for_mcp_call(...)` and `self.enforcement.evaluate(...)` before dispatch.
- Agent scan enforcement is factored and pre-dispatch.
- `McpServer` still carries two compatibility fields:
  - `scope: Option<Scope>` behind `rest-api`
  - `execution_policy: ExecutionPolicy`
- Deprecated helpers still exist:
  - `policy_decision_for_mcp_call(...)`
  - `denial_from_violation(...)`
- Legacy constructors still accept raw `Scope`:
  - `McpServer::with_scope(...)`
  - `McpServer::with_scope_and_profile(...)`

These are now mostly compatibility/test artifacts. This pass should remove them or quarantine them behind test-only code if full removal causes unacceptable churn.

## Goals

- Make `McpServer` production state unambiguous: `enforcement: EnforcementContext` is the only source of scope/policy authority.
- Remove stale `scope` and `execution_policy` fields from `McpServer` if possible.
- Remove or test-gate constructors that accept raw `Scope`.
- Remove deprecated MCP decision helpers that bypass `EnforcementContext`.
- Keep production behavior unchanged.
- Keep tests readable with explicit test constructors that build `EnforcementContext`.

## Non-goals

- Do not alter enforcement semantics.
- Do not change `ExecutionPolicy`, `LoadedScope`, or `EnforcementContext` behavior.
- Do not change MCP profile defaults.
- Do not change tool registry behavior.
- Do not do unrelated formatting churn outside touched files.

## Pass 1: inventory remaining legacy references

Run searches before editing:

```bash
rg "scope: Option<Scope>|self\.scope|execution_policy: crate::config::ExecutionPolicy|self\.execution_policy|with_scope\(|with_scope_and_profile\(|policy_decision_for_mcp_call\(|denial_from_violation\(" crates/eggsec/src/tool/protocol/mcp crates/eggsec/src/commands/handlers/serve.rs crates/eggsec/tests
```

Classify every hit as one of:

- production path
- test path
- compatibility-only path
- dead code

Acceptance criteria:

- No production path should require `scope: Option<Scope>` or `execution_policy` outside `EnforcementContext`.
- If a production path still does, fix that before removing fields.

## Pass 2: remove `scope` from `McpServer`

Target: `crates/eggsec/src/tool/protocol/mcp/handlers/server.rs`.

Remove the field:

```rust
#[cfg(feature = "rest-api")]
scope: Option<Scope>,
```

Then update constructors:

- `with_enforcement(...)` should no longer initialize `scope: None`.
- Legacy constructors should either be removed or rewritten to construct an `EnforcementContext` from their raw scope argument.

Preferred approach:

- Delete `with_scope(...)` and `with_scope_and_profile(...)` if only tests use them.
- Replace tests with a helper like:

```rust
fn test_mcp_server(
    registry: ToolRegistry,
    profile: McpProfile,
    scope: Scope,
    source: ScopeSource,
) -> McpServer {
    let loaded_scope = LoadedScope::explicit(scope, source, None);
    let enforcement = EnforcementContext::mcp_strict(ExecutionPolicy::default(), loaded_scope);
    McpServer::with_enforcement(registry, None, profile, enforcement)
}
```

If broad removal is too noisy, keep a single `#[cfg(test)]` constructor:

```rust
#[cfg(test)]
pub(crate) fn new_for_test(
    registry: ToolRegistry,
    api_key: Option<String>,
    profile: McpProfile,
    enforcement: EnforcementContext,
) -> Self {
    Self::with_enforcement(registry, api_key, profile, enforcement)
}
```

Acceptance criteria:

- `McpServer` no longer stores `scope`.
- `handle_tools_call()` has no `self.scope` references.
- Production code cannot pass raw `Scope` into `McpServer`.

## Pass 3: remove `execution_policy` from `McpServer`

Remove field:

```rust
pub(crate) execution_policy: crate::config::ExecutionPolicy,
```

Remove constructor assignments:

```rust
execution_policy: enforcement.execution_policy.clone(),
```

Remove or rewrite:

```rust
pub fn with_execution_policy(mut self, policy: ExecutionPolicy) -> Self
```

Preferred replacement:

- Remove `with_execution_policy(...)` entirely if no production path uses it.
- For tests, update the `EnforcementContext` before constructing `McpServer`.

Acceptance criteria:

- `McpServer` no longer stores an execution-policy mirror.
- All policy access goes through `self.enforcement.execution_policy` or `self.enforcement.evaluate(...)`.
- No production or test code calls `with_execution_policy(...)`.

## Pass 4: remove deprecated MCP decision helpers

Target: `crates/eggsec/src/tool/protocol/mcp/policy.rs`.

Remove deprecated helpers if no longer used:

- `policy_decision_for_mcp_call(...)`
- `denial_from_violation(...)`

Keep:

- `operation_descriptor_for_mcp_call(...)`
- `policy_decision_for_mcp_call_with_enforcement(...)`
- `McpPolicyDenial` only if still used by tests or response formatting.

If a test still needs a simple decision helper, update it to use:

```rust
let descriptor = operation_descriptor_for_mcp_call(...);
let decision = enforcement.evaluate(&descriptor).decision().clone();
```

Acceptance criteria:

- No MCP helper calls `evaluate_operation_policy(...)` directly for a tool call.
- All MCP tool-call decisions use `EnforcementContext::evaluate(...)`.
- No `#[allow(deprecated)]` remains for these helpers.

## Pass 5: tighten constructors and naming

After field removal, constructors should be explicit:

Production:

```rust
pub fn with_enforcement(
    registry: ToolRegistry,
    api_key: Option<String>,
    profile: McpProfile,
    enforcement: EnforcementContext,
) -> Self
```

Optional tests only:

```rust
#[cfg(test)]
pub(crate) fn new_for_test(...)
```

Avoid ambiguous names like `new(...)` if they create a default strict context with empty scope. If `new(...)` must remain for API compatibility, document it as non-networked/test-only and have it call `with_enforcement(...)` with an explicitly named `test_default_enforcement_context()` helper.

Acceptance criteria:

- Production construction cannot accidentally omit `EnforcementContext`.
- Test construction is concise but explicit about enforcement context.

## Pass 6: update route/server tests

Update tests that currently call removed constructors.

Add or preserve tests proving:

1. `create_mcp_router(...)` requires `EnforcementContext`.
2. `run_stdio(...)` requires `EnforcementContext`.
3. `McpServer::with_enforcement(...)` stores `McpStrict` when passed an MCP strict context.
4. Missing explicit manifest denies a networked target-bearing tool.
5. Explicit in-scope manifest allows a baseline tool.
6. Non-baseline capability denies unless explicitly allowed.
7. Denial payloads from profile validation and shared enforcement both use `policy_decision_for_mcp_call_with_enforcement(...)`.

Acceptance criteria:

- Tests fail if a raw `Scope` path is reintroduced into production MCP startup.
- Tests fail if an MCP tool-call decision bypasses `EnforcementContext`.

## Pass 7: update docs/comments

Update only narrow references:

- MCP constructor comments in `server.rs`.
- MCP docs if they mention raw scope fields or legacy constructors.
- `docs/SAFETY.md` / README only if they reference legacy MCP startup internals.

State the invariant:

> MCP server state stores one enforcement authority: `EnforcementContext`. Scope provenance and execution policy are accessed through that context. Raw `Scope` is not accepted by production MCP constructors.

Acceptance criteria:

- No docs describe `scope: Option<Scope>` as part of production MCP state.
- No docs recommend `with_scope(...)` or `with_scope_and_profile(...)`.

## Pass 8: validation

Run targeted checks first:

```bash
cargo fmt --all
cargo test -p eggsec --lib mcp
cargo test -p eggsec --lib enforcement
cargo test -p eggsec --lib scope
```

Then run the repo’s normal checks. If feasible:

```bash
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

If all-features is too expensive or blocked by known feature-combination issues, run the documented AGENTS quick-ref checks and list exact commands in the commit message.

## Final acceptance criteria

This pass is complete when:

- `McpServer` no longer stores raw `Scope`.
- `McpServer` no longer stores a separate `ExecutionPolicy` mirror.
- Production MCP constructors require `EnforcementContext`.
- Deprecated MCP decision helpers that bypass `EnforcementContext` are removed.
- Tests construct MCP servers through explicit enforcement contexts.
- MCP tool-call denial and allow paths remain unchanged behaviorally.

After this pass, the MCP enforcement cleanup should be considered complete unless future feature work introduces new MCP execution surfaces.
