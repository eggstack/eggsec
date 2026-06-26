# Final Enforcement Cleanup Handoff Plan

Date: 2026-06-10
Repository: eggstack/eggsec
Purpose: clean up stale APIs and duplicate enforcement construction after the enforcement series. The architecture is now functionally close; this pass should reduce drift, clarify invariants, and add regression tests around the final choke points.

## Current state

The enforcement architecture is now largely in place:

- `LoadedScope` / `ScopeSource` preserve explicit-manifest provenance.
- `EnforcementContext::evaluate()` centrally denies automated target-bearing operations that require explicit scope when the scope source is `DefaultEmpty`.
- `DenialClass` is now load-bearing for manual-permissive downgrade logic.
- Strict profiles require explicit allow for non-baseline capabilities.
- MCP has `operation_descriptor_for_mcp_call()` and `policy_decision_for_mcp_call_with_enforcement()`.
- MCP production startup uses `McpServer::with_enforcement(...)` instead of build-then-patch.
- Agent scans evaluate `EnforcementContext` immediately before `ToolDispatcher::dispatch(...)`.

Remaining cleanup items:

1. MCP `handle_tools_call()` still hand-builds an `OperationDescriptor` instead of reusing `operation_descriptor_for_mcp_call()`.
2. MCP router/stdio functions still accept a stale `scope: Option<Scope>` argument even though production `McpServer::with_enforcement()` sets `scope: None` and the authoritative scope is now inside `EnforcementContext.loaded_scope`.
3. `McpServer` still stores both `execution_policy` and `enforcement`; `execution_policy` is probably redundant now except for compatibility/testing.
4. Legacy constructors remain available. They are documented as test/support-only, but tests should prove production paths do not rely on them.
5. Deprecated MCP helpers still exist and may emit inconsistent decisions if reused by future code.
6. Agent per-scan capability/risk mapping is inline and string-based; it should be factored into helpers to avoid drift and make tests easier.
7. Documentation should explicitly name the final invariant: every MCP/agent network-capable operation must pass `EnforcementContext::evaluate()` immediately before dispatch.

## Goals

- Make MCP descriptor construction single-source-of-truth.
- Remove stale MCP scope arguments from production router/stdio APIs.
- Reduce redundant policy fields in `McpServer`, or clearly quarantine them for compatibility.
- Factor agent scan risk/capability mapping into testable helpers.
- Add regression tests proving MCP/agent denial prevents dispatch.
- Update docs to reflect the final architecture after cleanup.

## Non-goals

- Do not redesign `Scope`, `ExecutionPolicy`, `EnforcementContext`, or MCP profiles.
- Do not change default safety policy unless a test exposes a bug.
- Do not remove manual permissive behavior.
- Do not rewrite tool internals.
- Do not remove deprecated public APIs if that would cause broad churn; prefer deprecation and internal replacement.

## Pass 1: deduplicate MCP descriptor construction

Current issue: `crates/eggsec/src/tool/protocol/mcp/handlers/server.rs::handle_tools_call()` hand-builds an `OperationDescriptor` even though `operation_descriptor_for_mcp_call()` exists.

Change the shared enforcement block from manual construction to:

```rust
let descriptor = crate::tool::protocol::mcp::policy::operation_descriptor_for_mcp_call(
    &self.policy,
    &tool_id,
    capability.as_deref(),
    &arguments,
);

let outcome = self.enforcement.evaluate(&descriptor);
```

Remove now-unused imports in that block.

Acceptance criteria:

- `operation_descriptor_for_mcp_call()` is the only production path for MCP tool-call descriptors.
- Capability mapping remains unchanged.
- Existing MCP tests still pass.

## Pass 2: remove stale `scope` argument from MCP production APIs

Current production functions still accept `scope: Option<Scope>`:

- `create_mcp_router(registry, api_key, profile, scope, enforcement)`
- `run_stdio(registry, api_key, profile, scope, enforcement)`

But `McpServer::with_enforcement(...)` ignores this and sets `scope: None`; real scope lives in `enforcement.loaded_scope`.

Update signatures to:

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

Update all call sites, especially `crates/eggsec/src/commands/handlers/serve.rs`.

Remove temporary `let scope = Some(ctx.scope.clone());` in `handle_mcp_serve()`.

Acceptance criteria:

- No production MCP startup function accepts a separate `Scope`.
- Scope provenance is carried only by `EnforcementContext.loaded_scope`.
- Any remaining `scope: Option<Scope>` in `McpServer` is only for legacy/test constructors or removed in Pass 3.

## Pass 3: reduce redundant policy/scope fields in `McpServer`

Current `McpServer` fields include:

- `scope: Option<Scope>` behind `rest-api`
- `execution_policy: ExecutionPolicy`
- `enforcement: EnforcementContext`

The authoritative policy and scope are now inside `enforcement`.

Preferred cleanup:

- Remove `execution_policy` from `McpServer` if no production code uses it.
- Remove `scope` from `McpServer` if no legacy path still needs it.
- If removal causes broad compatibility churn, keep fields but mark comments clearly:
  - `scope`: legacy/test-only compatibility; production uses `enforcement.loaded_scope`.
  - `execution_policy`: legacy/test-only compatibility mirror; production uses `enforcement.execution_policy`.

Search for references before editing:

```bash
rg "\.execution_policy|self\.scope|scope:" crates/eggsec/src/tool/protocol/mcp crates/eggsec/src/commands/handlers/serve.rs
```

Acceptance criteria:

- Production MCP handler paths use `self.enforcement`, not `self.scope` or `self.execution_policy`.
- If fields remain, comments make their status explicit and tests guard against production use.

## Pass 4: remove legacy scope check from MCP `handle_tools_call()` if redundant

There is still a conditional block:

```rust
#[cfg(feature = "rest-api")]
if let Some(ref scope) = self.scope {
    match scope.is_target_allowed(target_value) { ... }
}
```

With production `with_enforcement()`, `self.scope` is `None`, so this block is stale in production. The authoritative check is now `self.enforcement.evaluate(&descriptor)`.

Options:

A. Remove the block entirely.
B. Keep it only for legacy constructors, but add a comment explaining that it is a compatibility belt-and-suspenders path and not production authority.

Preferred: remove if tests pass. If legacy tests require it, migrate tests to `with_enforcement()`.

Acceptance criteria:

- MCP dispatch denial relies on `EnforcementContext`, not the legacy `self.scope` block.
- No reduction in test coverage for out-of-scope denial.

## Pass 5: factor agent scan enforcement mapping into helpers

Current agent per-scan enforcement mapping is inline inside `execute_scan_with_depth()`.

Add helper functions in `crates/eggsec/src/agent/mod.rs` or a small submodule:

```rust
fn risk_for_agent_scan_depth(depth: ScanDepth, scan_type: &str) -> OperationRisk

fn capabilities_for_agent_scan(scan_type: &str, depth: ScanDepth) -> Vec<Capability>

fn operation_descriptor_for_agent_scan(
    target: &str,
    scan_type: &str,
    depth: ScanDepth,
) -> OperationDescriptor
```

Suggested behavior:

- Shallow default pipeline: `SafeActive`, `ActiveProbe`, `Crawl`.
- Deep default pipeline: `Intrusive`, `HttpFuzzLowImpact` or `IntrusiveFuzz` depending mutation/payload behavior.
- Scan type containing `stress`, `syn`, `udp`, `icmp`: include `WafStressTest`; risk `StressTest`.
- Scan type containing `load` or `bench`: include `LoadTest`; risk `LoadTest`.
- Scan type containing `packet` or `raw`: include `RawPacketProbe`; risk `RawPacket`.
- Scan type containing `credential`, `brute`, `auth`: include `CredentialTesting`; risk `CredentialTesting`.
- Scan type containing `remote`, `exec`, `ssh`: include `RemoteExecution`; risk `RemoteExecution`.

Acceptance criteria:

- `execute_scan_with_depth()` becomes mostly orchestration: constraints, descriptor helper, enforcement, request build, dispatch.
- Helper tests cover shallow/deep/high-risk string mappings.

## Pass 6: add dispatch-prevention regression tests

Add tests that prove enforcement denial prevents execution, not just returns a denial object.

MCP tests:

- Construct an MCP server through the production `with_enforcement()` route with `McpStrict` + `LoadedScope::default_empty()`.
- Call a networked tool with a target.
- Assert response error exists and dispatch is not reached. If direct dispatch mocking is hard, use a dummy tool/registry that would return a sentinel success if reached, and assert sentinel is absent.

Agent tests:

- Use `Agent::new_for_test()` with a fake dispatcher that records call count.
- Configure `AgentConfig.enforcement = Some(EnforcementContext::agent_strict(... DefaultEmpty ...))`.
- Call `execute_scan_with_depth()` with a target.
- Assert error and fake dispatcher call count is zero.
- Repeat with explicit in-scope manifest and shallow scan; assert dispatcher is called once.

Policy tests:

- Assert `operation_descriptor_for_mcp_call()` and MCP pre-dispatch path produce equivalent descriptors/capabilities for representative tools.
- Assert deprecated helper is not used by production handler code if feasible with a small grep-style test or by searching references in code review.

Acceptance criteria:

- Tests prove MCP and agent enforcement is pre-dispatch.
- Tests fail if someone bypasses `EnforcementContext::evaluate()` in production paths.

## Pass 7: clean up docs to match final architecture

Update only relevant docs:

- README safety/enforcement section.
- `docs/SAFETY.md`.
- MCP/codegg integration docs if present.
- Agent/autonomous docs if present.

Add a concise invariant statement:

> For MCP and autonomous-agent execution, `EnforcementContext::evaluate()` is the mandatory pre-dispatch gate. Scope provenance must come from `LoadedScope`; raw `Scope` is not sufficient for automated execution.

Clarify baseline capabilities:

- Baseline strict automated capabilities: `PassiveFingerprint`, `ActiveProbe`, `Crawl`, `WafDetect`.
- Non-baseline capabilities require explicit `allowed_capabilities` in `ExecutionPolicy` and matching risk/feature gates.

Clarify manual behavior:

- Manual permissive can downgrade only safe scope-selection misses.
- Explicit exclusions, feature gates, risk gates, and capability denials remain hard denials.

Acceptance criteria:

- Docs no longer describe stale `scope` arguments in MCP startup internals.
- Docs name `EnforcementContext` and `LoadedScope` as the final source of truth.

## Pass 8: final code hygiene

Run normal repo checks. Suggested minimum:

```bash
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

If all-features is too expensive or feature-combination-sensitive, run the repo’s documented AGENTS quick-ref checks and record which commands passed in the commit message.

Also run targeted tests for:

- enforcement/provenance
- MCP handlers/routes
- agent dispatch/enforcement
- scope handling

Acceptance criteria:

- No new warnings from stale imports after descriptor deduplication.
- New tests cover MCP/agent pre-dispatch enforcement.
- Commit message lists exact commands run.

## Final acceptance criteria

This cleanup pass is complete when:

- MCP production code has one descriptor construction path: `operation_descriptor_for_mcp_call()`.
- MCP production startup no longer accepts or passes a stale `scope: Option<Scope>` argument.
- MCP production execution relies on `EnforcementContext`, not legacy `self.scope` checks.
- Agent scan enforcement mapping is factored into helper functions and tested.
- MCP and agent tests prove dispatch is not reached on enforcement denial.
- Docs accurately describe the final enforcement invariant.

At that point, treat the scope/enforcement architecture as settled unless future feature work reveals a concrete bypass or ergonomics issue.
