# Dual-Mode Enforcement Final Closure Correction Plan

## Goal

Close the final issues found after the closure verification pass. The dual-mode enforcement architecture is largely complete, but one remaining agent dispatch branch still falls back to raw `ToolDispatcher::dispatch()` in production code, and the raw-dispatch regression test currently allowlists the entire `src/agent` tree, which can mask that class of defect.

This pass should make the security-agent dispatch path fail closed and make the regression test precise enough to catch future strict-surface bypasses.

## Current state

The latest closure remediation commit added useful hardening:

- `operation_matches_tool_id()` for metadata alias-aware dispatch checks.
- `EnforcedDispatcher::dispatch_checked()` now uses alias-aware matching.
- A raw-dispatch source-scan test exists.
- Preflight/evaluate parity tests were added.
- Agent audit event tests were added.
- Documentation and validation notes were updated.

Remaining defects:

1. `Agent::execute_scan_with_depth()` still has an `UNREACHABLE in production` branch that raw-dispatches when `enforced_dispatcher` is present but `approved_token` is missing.
2. `crates/eggsec/tests/enforced_dispatch_regression.rs` allowlists the full `src/agent` prefix, so it does not catch the production fallback.
3. The test should distinguish test-only dispatch adapters from production agent execution paths.

## Non-negotiable invariant

For `ExecutionSurface::SecurityAgent`, target-bearing scan dispatch must never call raw `ToolDispatcher::dispatch()` in production. If an approval token is missing, that is an internal invariant violation and must be a hard error, not a fallback.

## Workstream 1: Remove production raw-dispatch fallback from agent execution

### File

- `crates/eggsec/src/agent/mod.rs`

### Current problematic shape

The agent dispatch tail currently has this structure conceptually:

```rust
if let Some(ref enforced) = self.enforced_dispatcher {
    if let Some(ref approved) = approved_token {
        enforced.dispatch_checked(approved, request).await
    } else {
        // UNREACHABLE in production ...
        self.dispatcher.dispatch(request).await
    }
} else {
    // Test-only path
    self.dispatcher.dispatch(request).await
}
```

The production fallback must be removed.

### Required change

Change the missing-token branch to a hard error:

```rust
if let Some(ref enforced) = self.enforced_dispatcher {
    let approved = approved_token.as_ref().ok_or_else(|| {
        anyhow::anyhow!(
            "internal enforcement invariant violation: security-agent dispatch reached without ApprovedOperation"
        )
    })?;

    enforced
        .dispatch_checked(approved, request)
        .await
        .map_err(|e| anyhow::anyhow!("{:?}", e))
} else {
    // Test-only path: new_for_test() sets enforced_dispatcher to None.
    self.dispatcher
        .dispatch(request)
        .await
        .map_err(|e| anyhow::anyhow!("{:?}", e))
}
```

Alternative equivalent structures are acceptable, but the rule is strict: if `enforced_dispatcher.is_some()` and `approved_token.is_none()`, return an error and do not dispatch.

### Additional recommendation

Make the invariant message specific enough to grep in tests:

```text
security-agent dispatch reached without ApprovedOperation
```

### Acceptance criteria

- There is no raw `self.dispatcher.dispatch(request)` fallback inside the production `enforced_dispatcher.is_some()` branch.
- Missing `ApprovedOperation` under production agent dispatch returns an error.
- Test-only raw dispatch remains available only when `enforced_dispatcher` is `None`.

## Workstream 2: Narrow the raw-dispatch regression test allowlist

### File

- `crates/eggsec/tests/enforced_dispatch_regression.rs`

### Current issue

The test scans `src/agent`, but the allowlist includes `("src/agent", "Test-only new_for_test() fallback")`, which effectively suppresses all raw dispatch violations inside production agent code.

### Required change

Remove the broad `src/agent` allowlist entry.

Replace it with narrow handling for known test-only locations. Options:

1. Allow only explicit line/function patterns, such as `new_for_test`, `MockDispatcher`, or `#[cfg(test)]` blocks.
2. Allow raw dispatch inside specific test modules or test-only files only.
3. Keep no agent allowlist and refactor test-only dispatch behind more precise markers.

Preferred approach:

- Remove `("src/agent", ...)` from `allowlist`.
- Update `check_file()` to track whether a line is inside a `#[cfg(test)] mod tests` block only if needed.
- Or simpler: do not allow any `.dispatch(` in `src/agent/mod.rs` except comments and `dispatch_checked`.

If the trait implementation for `ScanDispatcherTrait for ToolDispatcher` at the top of `src/agent/mod.rs` remains necessary, handle it with a precise allow entry rather than the full file. Example:

```rust
let allow_exact = [
    ("src/agent/mod.rs", "Box::pin(self.dispatch(request))", "ScanDispatcherTrait test adapter")
];
```

Then check both path and line substring.

### Suggested revised helper

Use a structured allowlist:

```rust
struct RawDispatchAllow {
    path_suffix: &'static str,
    line_contains: &'static str,
    reason: &'static str,
}
```

Allowed entries should be narrow, for example:

```rust
RawDispatchAllow {
    path_suffix: "src/tool/dispatcher.rs",
    line_contains: "self.inner.dispatch(request).await",
    reason: "EnforcedDispatcher internal terminal call",
}
RawDispatchAllow {
    path_suffix: "src/agent/mod.rs",
    line_contains: "Box::pin(self.dispatch(request))",
    reason: "ScanDispatcherTrait adapter; production execution path must still use EnforcedDispatcher",
}
RawDispatchAllow {
    path_suffix: "src/agent/mod.rs",
    line_contains: "// Test-only path",
    reason: "comment marker only; not a dispatch line",
}
```

Do not allow production agent fallback dispatch lines.

### Acceptance criteria

- The source-scan test fails if a production `.dispatch(request)` appears in `src/agent/mod.rs` outside a precise test/adapter allowlist.
- The test still allows `EnforcedDispatcher::dispatch_checked()` internals.
- The test does not block unrelated non-tool notification dispatch.
- The test remains simple and maintainable.

## Workstream 3: Add targeted agent invariant test

### Goal

Add a focused regression test proving the agent cannot dispatch when an enforced dispatcher exists but no approval token exists.

This can be implemented directly or indirectly depending on current visibility.

### Options

#### Option A: Unit-test a helper

Extract the final dispatch selection into a small helper:

```rust
async fn dispatch_agent_request(
    &self,
    approved_token: Option<&ApprovedOperation>,
    request: ToolRequest,
) -> Result<ToolResponse>
```

Then test:

- `enforced_dispatcher = Some`, `approved_token = None` returns invariant error.
- `enforced_dispatcher = Some`, `approved_token = Some` uses checked dispatch.
- `enforced_dispatcher = None`, test-only path uses raw dispatch.

#### Option B: Source-scan-only coverage

If extracting the helper is too invasive, rely on the narrowed source-scan regression plus existing agent approval tests. This is acceptable for a small closure patch, but Option A is stronger.

### Acceptance criteria

At minimum:

- The raw-dispatch source-scan test catches the removed fallback if reintroduced.

Preferred:

- A direct unit test confirms missing token under enforced dispatcher returns the invariant error.

## Workstream 4: Re-check MCP/REST strict dispatch coverage

### Goal

Confirm the agent fix did not regress the already-correct strict surfaces.

### Steps

Run searches:

```bash
rg "\.dispatch\(" crates/eggsec/src/tool/protocol/rest.rs crates/eggsec/src/tool/protocol/mcp crates/eggsec/src/agent crates/eggsec/src/commands/handlers/ci.rs
rg "dispatch_checked" crates/eggsec/src/tool/protocol/rest.rs crates/eggsec/src/tool/protocol/mcp crates/eggsec/src/agent
rg "approve\(" crates/eggsec/src/tool/protocol/rest.rs crates/eggsec/src/tool/protocol/mcp crates/eggsec/src/agent
```

Expected:

- REST uses `approve(ExecutionSurface::RestApi, ...)` and `dispatch_checked()`.
- MCP uses `approve(ExecutionSurface::McpServer, ...)` and `dispatch_checked()`.
- Agent uses `approve(ExecutionSurface::SecurityAgent, ...)` and `dispatch_checked()`.
- Raw `.dispatch(` appears only in `EnforcedDispatcher` internals, test-only code, or explicitly documented non-strict helper paths.

## Workstream 5: Validation commands

Run focused validation first:

```bash
cargo fmt --all
cargo test -p eggsec --test enforced_dispatch_regression
cargo test -p eggsec --lib agent::tests
cargo test -p eggsec --lib config::policy
cargo test -p eggsec --features rest-api --test enforcement_matrix
```

Then run broader relevant checks:

```bash
cargo test -p eggsec --lib
cargo test -p eggsec --features rest-api --lib
cargo check -p eggsec-cli --features rest-api
```

If pre-existing failures remain, document them precisely in the commit message. Do not let this pass introduce new failures.

## Documentation updates

Update only if code changes require it:

- `plans/dual-mode-enforcement-closure-verification.md`: add a completion note if the repo convention is to annotate completed plans.
- `docs/ENFORCEMENT_MODES.md`: only update if raw dispatch restrictions are not already described.
- `AGENTS.md`: update validation counts only if the repo keeps exact counts current.

Avoid broad documentation churn.

## Expected final commit message contents

The implementation commit should include:

```text
fix(agent): remove raw dispatch fallback from strict agent execution

- Replace production missing-ApprovedOperation branch with hard invariant error
- Narrow enforced_dispatch_regression allowlist so src/agent production raw dispatch is not masked
- Add/adjust tests for alias-aware dispatch and strict-surface raw-dispatch scanning
- Validation: <commands and results>
- Known pre-existing failures: <only if still present>
```

## Final acceptance criteria

This pass is complete when:

- `Agent::execute_scan_with_depth()` cannot raw-dispatch in production if an `ApprovedOperation` is missing.
- `enforced_dispatch_regression` would fail if that fallback is reintroduced.
- REST/MCP/agent strict dispatch paths still use `approve()` + `dispatch_checked()`.
- Focused tests pass.
- Any remaining failures are documented as pre-existing and unrelated.

After this pass, the dual-mode enforcement roadmap can be considered closed from an architecture standpoint.
