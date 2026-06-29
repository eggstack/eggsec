# Phase 12 Handoff Plan: Type-Level Enforced Dispatch

## Goal

Move enforcement from convention to type-level structure for automated and programmatic dispatch. MCP, REST, security-agent, CI, and eventually high-risk TUI direct-launch paths should not be able to dispatch a target-bearing operation unless they possess an `ApprovedOperation` produced by the shared enforcement evaluator.

This phase hardens the architecture so future contributors cannot accidentally bypass policy by calling a raw dispatcher.

## Current context

The corrective pass closed the obvious dispatch bypasses:

- REST now evaluates enforcement before tool dispatch.
- Agent evaluates before scan dispatch.
- TUI direct-launch tabs preflight before side effects.
- MCP already evaluates before dispatch.

However, these are still largely conventions: call sites are expected to remember to evaluate first. Phase 12 introduces a type that represents an approved enforcement decision and changes dispatch APIs so strict surfaces require that token.

## Core design

Add an approval token:

```rust
#[derive(Debug, Clone)]
pub struct ApprovedOperation {
    descriptor: OperationDescriptor,
    decision: PolicyDecision,
    surface: ExecutionSurface,
    profile: ExecutionProfile,
    audit_event_id: Option<String>,
}
```

Fields should be private. Provide read-only accessors:

```rust
impl ApprovedOperation {
    pub fn descriptor(&self) -> &OperationDescriptor;
    pub fn decision(&self) -> &PolicyDecision;
    pub fn surface(&self) -> ExecutionSurface;
    pub fn profile(&self) -> ExecutionProfile;
}
```

Only enforcement code should be able to construct it.

## Approval API

Add approval methods:

```rust
impl EnforcementContext {
    pub fn approve(
        &self,
        surface: ExecutionSurface,
        descriptor: OperationDescriptor,
    ) -> Result<ApprovedOperation, EnforcementError>;

    pub fn approve_manual(
        &self,
        surface: ExecutionSurface,
        descriptor: OperationDescriptor,
        manual_override: Option<&ManualOverride>,
    ) -> Result<ApprovedOperation, EnforcementError>;
}
```

Suggested semantics:

- `approve()` is strict: only `Allow` succeeds.
- `approve_manual()` supports manual permissive confirmation logic when `surface.honors_manual_override()`.
- Automated surfaces never accept manual override.
- `Warn` behavior should be explicit. For strict automated surfaces, `Warn` should fail. For manual surfaces, either `approve_manual()` can return approved with warning or a separate `ApprovedOperation` field can record warning decision.

Do not hide warnings. If an operation proceeds with warning in manual mode, the `ApprovedOperation` should carry that decision.

## Error type

Add a structured error:

```rust
#[derive(Debug, thiserror::Error)]
pub enum EnforcementError {
    #[error("operation denied by policy")]
    Denied { decision: PolicyDecision },
    #[error("manual confirmation required")]
    ConfirmationRequired {
        decision: PolicyDecision,
        required_classes: Vec<ConfirmationClass>,
    },
    #[error("manual override unavailable for surface {surface}")]
    ManualOverrideUnavailable { surface: ExecutionSurface, decision: PolicyDecision },
}
```

Map this into CLI/TUI/REST/MCP errors at the edges.

## Dispatcher API

Introduce checked dispatch wrappers rather than immediately deleting raw dispatch.

Example:

```rust
pub struct EnforcedDispatcher {
    inner: ToolDispatcher,
}

impl EnforcedDispatcher {
    pub async fn dispatch_checked(
        &self,
        approved: ApprovedOperation,
        request: ToolRequest,
    ) -> Result<ToolResponse, EggsecError> {
        verify_request_matches_descriptor(&approved, &request)?;
        self.inner.dispatch(request).await
    }
}
```

Verification should check at minimum:

- `approved.descriptor.operation == request.tool` or metadata alias matches.
- Target in descriptor matches request target value.
- The approved operation was produced for the expected surface/profile where practical.

Do not overfit exact target normalization in the first pass. Add conservative checks and fail closed on mismatch.

## Step 1: Add `ApprovedOperation` and approval helpers

Implement the type and helper methods in config/enforcement modules.

Keep existing `evaluate()` for preflight and diagnostics. `approve()` is for dispatch authorization.

## Step 2: Convert REST dispatch

REST is the best first strict surface because its execute path is compact.

Flow should become:

1. Build descriptor from metadata.
2. `let approved = state.enforcement.approve(ExecutionSurface::RestApi, descriptor)?;`
3. Build `ToolRequest`.
4. `state.dispatcher.dispatch_checked(approved, request).await`.

REST should no longer pattern-match raw `EnforcementOutcome` in its execute path except for converting `EnforcementError` into response.

## Step 3: Convert MCP dispatch

MCP tool execution should require `ApprovedOperation` before dispatch.

Flow:

1. Validate MCP profile/tool availability.
2. Build descriptor from metadata.
3. `approve(McpServer, descriptor)`.
4. Dispatch checked.

Manual overrides must not be present in this path.

## Step 4: Convert security-agent dispatch

Agent scan execution should require approval token immediately before tool request dispatch.

Flow:

1. Build descriptor.
2. `approve(SecurityAgent, descriptor)`.
3. Build request.
4. Dispatch checked.

If request building depends on the descriptor target/operation, consider building descriptor and request from a shared local struct to avoid mismatch.

## Step 5: Convert CI dispatch

Any CI command path that dispatches tools should use `approve(Ci, descriptor)`.

## Step 6: Convert TUI direct-launch and task dispatch carefully

TUI manual dispatch is more nuanced because manual permissive may allow warnings and confirmations.

Use:

```rust
approve_manual(ExecutionSurface::TuiManual, descriptor, Some(&manual_override))
```

For TUI guarded:

```rust
approve(ExecutionSurface::TuiManualStrict, descriptor)
```

Do not block the phase on converting every TUI tab at once. Prioritize direct-launch/high-risk tabs first:

- Packet.
- Stress.
- Load.
- Web proxy/intercept.
- DB pentest.
- Wireless active.
- C2.

## Step 7: Restrict raw dispatcher access

After strict surfaces are converted:

- Make raw `ToolDispatcher::dispatch()` crate-private if possible.
- Or add `#[doc(hidden)]` / comments and prefer `EnforcedDispatcher` in public protocol code.
- Add grep/static tests to detect direct calls from protocol/agent modules.

Potential test:

```rust
#[test]
fn strict_surfaces_do_not_call_raw_dispatcher() { ... }
```

This can be a simple source scan test if the repo already tolerates such tests.

## Step 8: Tests

Required tests:

- `approve()` returns `ApprovedOperation` on `Allow`.
- `approve()` rejects `Warn`, `RequireConfirmation`, and `Deny` for automated strict surfaces.
- `approve_manual()` accepts manual warning/confirmation only when matching override is present.
- `approve_manual()` rejects override on strict/automated surfaces.
- `dispatch_checked()` rejects request/tool mismatch.
- `dispatch_checked()` rejects target mismatch.
- REST execute path uses approval token.
- MCP execute path uses approval token.
- Agent scan path uses approval token.
- Raw dispatcher is not called directly from strict protocol modules.

## Step 9: Audit integration

If Phase 10 audit exists, attach audit event ID to `ApprovedOperation`:

- Approval creates audit event or receives one from caller.
- Dispatch logs approved event ID.
- Denial paths produce audit events without approval token.

If Phase 10 is not implemented, keep `audit_event_id: Option<String>` for forward compatibility.

## Acceptance criteria

- `ApprovedOperation` exists with private fields.
- Only enforcement approval helpers construct it.
- REST uses approval token before dispatch.
- MCP uses approval token before dispatch.
- Agent uses approval token before dispatch.
- At least high-risk TUI direct-launch paths use approval token or are explicitly queued for follow-up.
- Raw dispatch access is restricted or guarded by tests.
- Tests cover approval success/failure and request/descriptor mismatch.

## Validation commands

Run:

```bash
cargo fmt --all
cargo test -p eggsec --features rest-api --lib
cargo test -p eggsec --features rest-api --test enforcement_matrix
cargo test -p eggsec-tui
cargo check -p eggsec-cli --features rest-api
```

## Non-goals

- Do not remove `evaluate()`; it is still needed for preflight/explanations.
- Do not force all manual CLI handlers into tokenized dispatch in one pass if it becomes too large.
- Do not change policy semantics.
- Do not add a manual REST mode.

## Expected final state

After this phase, Eggsec's strict programmatic surfaces should be structurally unable to dispatch without an approval token. The enforcement system will no longer rely only on developer discipline at call sites; the type system will enforce the intended control flow for MCP, REST, agent, and CI execution.
