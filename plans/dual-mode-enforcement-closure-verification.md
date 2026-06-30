# Dual-Mode Enforcement Closure Verification Plan

## Goal

Close out the dual-mode enforcement roadmap with a targeted verification and cleanup pass. The major phases are now implemented: operation metadata, strict REST posture, enforcement matrix tests, shared preflight, normalized audit events, initial domain crate extraction, and type-level approved dispatch. This plan is intentionally not another feature phase. It is a final hardening pass to verify that the architecture is consistent, testable, documented, and difficult to regress.

The desired final state is simple:

- Manual CLI/TUI remains useful and operator-directed.
- MCP, REST, security-agent, and CI remain strict and noninteractive.
- Every externally invokable operation has canonical metadata.
- Every strict programmatic dispatch path requires an `ApprovedOperation` token.
- Raw dispatch cannot be accidentally used by strict surfaces.
- Preflight, audit, and execution agree on descriptors and policy decisions.
- Domain crate extraction did not move enforcement out of the central control plane.

## Current state to verify

From the recent implementation history, the repo appears to have:

- `OperationMetadata` and metadata-derived descriptors.
- REST strict allow-only posture and structured policy errors.
- Enforcement matrix coverage expanded to 134 tests.
- Shared `PreflightResult` and preflight routes/tools/CLI/TUI integration.
- Normalized audit events across major surfaces.
- Initial domain crates: `eggsec-db-lab` and `eggsec-web-proxy`.
- `ApprovedOperation`, `EnforcementError`, and `EnforcedDispatcher` for type-level dispatch.

This closure pass should confirm these claims by inspecting code and tests directly, then patching remaining gaps.

## Workstream 1: Raw dispatch call-site audit

### Objective

Ensure strict programmatic surfaces cannot bypass enforcement by directly calling raw `ToolDispatcher::dispatch()`.

### Files and modules to inspect

- `crates/eggsec/src/tool/dispatcher.rs`
- `crates/eggsec/src/tool/protocol/rest.rs`
- MCP protocol handlers under `crates/eggsec/src/tool/protocol/mcp/`
- Agent execution paths under `crates/eggsec/src/agent/`
- CI handlers under `crates/eggsec/src/commands/handlers/`
- TUI direct-launch and worker paths under `crates/eggsec-tui/src/`
- Tool orchestrator/planner paths if they can execute tools programmatically

### Steps

1. Search the repository for raw dispatch calls:

   ```bash
   rg "\.dispatch\(" crates/eggsec crates/eggsec-tui crates/eggsec-cli crates/eggsec-agent
   rg "ToolDispatcher::" crates/eggsec crates/eggsec-agent
   rg "EnforcedDispatcher" crates/eggsec crates/eggsec-agent
   ```

2. Classify each raw dispatch call:

   - Allowed: inside `EnforcedDispatcher::dispatch_checked()`.
   - Allowed: test-only helpers or isolated unit tests.
   - Allowed with comment: internal non-networked helper paths that cannot execute a target-bearing operation.
   - Needs patch: REST/MCP/agent/CI strict surfaces using raw dispatch.
   - Needs review: TUI direct-launch/high-risk paths using raw dispatch without explicit preflight/approval.

3. Convert any strict-surface raw dispatch to `approve()` + `dispatch_checked()`.

4. For any remaining raw dispatch in production code, add a short comment explaining why it is not a strict external execution path.

5. Add a source-scan regression test if practical.

Suggested test shape:

```rust
#[test]
fn strict_surfaces_do_not_call_raw_dispatch_directly() {
    let roots = [
        "src/tool/protocol/rest.rs",
        "src/tool/protocol/mcp",
        "src/agent",
        "src/commands/handlers/ci.rs",
    ];
    // scan source text for `.dispatch(` and allow only known false positives / dispatch_checked
}
```

Keep the allowlist narrow and documented.

### Acceptance criteria

- No REST/MCP/security-agent/CI production path calls raw `ToolDispatcher::dispatch()` directly.
- Any remaining raw dispatch call has a clear reason and cannot bypass strict external enforcement.
- A source-scan or equivalent regression test exists, or the rationale for not adding one is documented.

## Workstream 2: ApprovedOperation coverage verification

### Objective

Confirm that type-level approved dispatch is actually active in the high-value strict surfaces, not merely defined.

### Steps

1. Verify REST:

   - REST execute builds descriptor from metadata.
   - REST checks `rest_exposable`.
   - REST calls `EnforcementContext::approve(ExecutionSurface::RestApi, descriptor)`.
   - REST dispatches through `EnforcedDispatcher::dispatch_checked()`.
   - REST never accepts manual overrides.

2. Verify MCP:

   - MCP tool execution builds descriptor from metadata.
   - MCP checks `mcp_exposable` or profile availability.
   - MCP calls `approve(ExecutionSurface::McpServer, descriptor)`.
   - MCP dispatches through `dispatch_checked()`.
   - MCP fails closed on missing metadata.

3. Verify security agent:

   - Agent scan execution builds descriptor from metadata or documented conservative fallback.
   - Agent uses `ExecutionSurface::SecurityAgent` and `AgentStrict` enforcement.
   - Agent calls `approve(...)` immediately before dispatch.
   - Agent dispatches through `dispatch_checked()`.
   - Agent records/audits denial without dispatch.

4. Verify CI:

   - CI execution paths use strict profile.
   - If CI dispatches tools, it uses `approve(ExecutionSurface::Ci, ...)` or an equivalent strict token path.
   - If CI does not dispatch tools directly, document that fact in a comment/test.

5. Verify TUI high-risk/direct-launch paths:

   - Direct-launch tabs preflight before side effects.
   - For high-risk direct-launch tabs, consider moving from preflight-only to `approve_manual()` if not already done.
   - At minimum, confirm no side-effecting direct-launch tab can start after `Deny` or unresolved `RequireConfirmation`.

### Acceptance criteria

- REST, MCP, and agent all require `ApprovedOperation` before dispatch.
- CI either requires approval or is documented as non-dispatching.
- TUI direct-launch side effects are blocked before denial/confirmation.
- Tests cover at least REST and one agent/MCP strict dispatch approval path.

## Workstream 3: Metadata coverage and drift check

### Objective

Ensure operation metadata is complete enough to remain the source of truth.

### Steps

1. Compare all registered tool IDs from `create_default_registry()` against `metadata_for_tool_id()`.

2. Compare all TUI executable tab `operation_id`s against `operation_metadata()`.

3. Compare all MCP-exposed tool IDs against metadata and `mcp_exposable`/profile rules.

4. Compare REST tool IDs against metadata and `rest_exposable`.

5. Compare agent known scan types against metadata and ensure unknown fallback is conservative:

   - target-bearing fallback requires explicit scope.
   - fallback is not lower risk than the scan type implies.
   - fallback never grants nonbaseline capability silently.

6. Ensure high-risk metadata has expected capabilities:

   - `DbPentest` -> `DatabaseAssessment`.
   - `TrafficInterception` -> `TrafficInterception`.
   - `RawPacket` -> `RawPacketProbe`.
   - `CredentialTesting` -> `CredentialTesting`.
   - `C2Operation` -> `C2Simulation`.
   - `RemoteExecution` -> `RemoteExecution`.

7. Ensure feature-gated metadata declares required features where appropriate:

   - `db-pentest`.
   - `web-proxy` / `web-proxy-mcp`.
   - `wireless-advanced`.
   - `nse`.
   - `c2`.

### Tests to add or tighten

- Every registered tool has metadata.
- Every executable TUI operation has metadata.
- Every MCP-exposed tool has metadata and `mcp_exposable`.
- Every REST-exposed tool has metadata and `rest_exposable`.
- Every high-risk operation declares at least one nonbaseline capability unless explicitly documented as a management/read-only operation.
- Metadata alias IDs resolve to the canonical operation expected by dispatcher checks.

### Acceptance criteria

- No externally invokable tool lacks metadata.
- Metadata coverage tests fail closed on future additions.
- Known aliases do not break `dispatch_checked()` operation matching.

## Workstream 4: Descriptor/dispatch alias consistency

### Objective

Prevent `ApprovedOperation` descriptor IDs from mismatching concrete tool IDs because of metadata aliases.

### Problem to check

`dispatch_checked()` compares `request.tool` to `approved.descriptor().operation`. If metadata aliases map tool IDs to canonical IDs, strict dispatch can fail unless either:

- descriptor operation uses the concrete dispatch tool ID, or
- `dispatch_checked()` understands canonical aliases.

### Steps

1. Identify every metadata alias.

2. For REST and MCP calls, verify the approved descriptor operation equals the eventual `ToolRequest.tool`, or add an alias-aware matcher.

3. Add a helper near metadata:

```rust
pub fn operation_matches_tool_id(operation_id: &str, tool_id: &str) -> bool
```

This should return true when:

- exact match, or
- `tool_id` aliases to `operation_id`, or
- both resolve to the same canonical metadata entry.

4. Use this helper in `EnforcedDispatcher::dispatch_checked()` instead of raw string equality.

5. Add tests for representative aliases:

- `scan` vs `scan-ports` if applicable.
- `ports` vs `scan-ports` if applicable.
- `loadtest` vs `load` if applicable.
- `proxy-intercept` vs `proxy-start`/proxy tool ID if applicable.
- `waf_detect`/`waf-detect` variants if present.

### Acceptance criteria

- Alias-based descriptor generation does not break strict dispatch.
- `dispatch_checked()` remains fail-closed for unrelated tool IDs.
- Tests cover exact match, alias match, and mismatch.

## Workstream 5: Preflight/execution parity

### Objective

Ensure preflight answers match execution decisions for the same surface, descriptor, and scope.

### Steps

1. Add helper tests that build a descriptor once, then compare:

   - `preflight_operation(...)` result outcome.
   - `enforcement.evaluate(...)` outcome.
   - `approve(...)` or `approve_manual(...)` result/error.

2. Cover:

   - CLI manual safe op.
   - CLI manual positive scope miss requiring confirmation.
   - TUI guarded out-of-scope denial.
   - REST strict allow.
   - REST strict warn/confirmation/deny.
   - MCP strict missing metadata or out-of-scope denial.
   - Agent strict explicit-scope allow and miss.

3. Ensure REST/MCP preflight endpoints/tools never dispatch.

4. Ensure TUI preflight display uses the same `PreflightResult` or a lossless wrapper.

### Acceptance criteria

- Preflight and execution use the same descriptor/evaluator path.
- Route/tool-level preflight tests prove no dispatch occurs.
- Manual suggested flags appear only for manual surfaces.
- Automated preflight does not imply user-overridable approval.

## Workstream 6: Audit event parity and noise control

### Objective

Verify normalized audit is useful and not noisy or misleading.

### Steps

1. Verify audit events include:

   - event ID.
   - timestamp.
   - surface/profile.
   - operation/target.
   - outcome.
   - policy decision.
   - scope provenance.
   - manual override accepted/ignored fields.
   - correlation/request ID where applicable.

2. Confirm manual override audit is only accepted for manual permissive surfaces.

3. Confirm strict surfaces never emit “confirmed” manual override events.

4. Confirm REST/MCP/agent denials emit audit before returning error.

5. Confirm allow events are emitted once per operation, not multiple times for the same dispatch path.

6. Bound agent retained policy denials if it stores them.

7. Consider adding audit-level filtering if logs become too chatty.

### Tests

- Manual CLI/TUI confirmed override audit event contains classes/reason.
- REST denial audit event includes correlation ID.
- MCP denial audit event includes request ID.
- Agent denial audit is stored/emitted and bounded.
- Preflight audit does not look like execution approval unless explicitly marked as preflight.

### Acceptance criteria

- Audit events are consistent across surfaces.
- Manual override audit cannot be confused with automated approval.
- No excessive duplicate events in normal dispatch paths.

## Workstream 7: Domain crate boundary verification

### Objective

Ensure `eggsec-db-lab` and `eggsec-web-proxy` extraction did not move enforcement decisions into domain crates or create dependency cycles.

### Steps

1. Verify workspace membership:

   - `crates/eggsec-db-lab`.
   - `crates/eggsec-web-proxy`.

2. Verify dependency direction:

   - `eggsec` may depend on domain crates.
   - domain crates must not depend on `eggsec`.
   - domain crates may depend on `eggsec-core`, `eggsec-output`, and `eggsec-tool-core` if needed.

3. Search domain crates for central enforcement types:

   ```bash
   rg "ExecutionSurface|ExecutionProfile|EnforcementContext|ApprovedOperation|ManualOverride" crates/eggsec-db-lab crates/eggsec-web-proxy
   ```

4. Any domain crate use of enforcement types should be scrutinized. Prefer domain crates to receive already-sanitized execution config or domain inputs.

5. Verify main-crate adapters perform enforcement before calling domain crates.

6. Verify feature flags remain optional and do not pull heavy dependencies into default builds.

### Acceptance criteria

- No domain crate depends on main `eggsec` crate.
- Domain crates do not make allow/deny decisions based on execution surface.
- Main crate adapter remains the enforcement boundary.
- Domain crate tests and adapter smoke tests pass.

## Workstream 8: Documentation and plan status cleanup

### Objective

Make documentation match the new architecture and mark the roadmap as implemented/closed.

### Docs to inspect/update

- `plans/dual-mode-enforcement-roadmap.md`
- Phase plan files 1–12.
- `plans/dual-mode-enforcement-closure-verification.md` after implementation.
- `docs/ENFORCEMENT_MODES.md`
- `docs/SAFETY.md`
- `docs/CAPABILITIES.md`
- `architecture/overview.md`
- `architecture/config.md`
- `architecture/tui.md`
- `architecture/database_pentest.md`
- `architecture/proxy.md` / `architecture/web_proxy.md`
- `AGENTS.md`
- crate-local `AGENTS.override.md` files if present.

### Required documentation updates

- State that operation metadata is now source of truth.
- State that REST is strict allow-only and noninteractive.
- State that MCP/REST/agent strict dispatch uses `ApprovedOperation`.
- State that `eggsec-db-lab` and `eggsec-web-proxy` are extracted domain crates.
- Document raw dispatch restrictions.
- Update test counts only if the project convention requires exact counts.
- Mark phase plans complete or superseded where appropriate.

### Acceptance criteria

- No docs describe REST as raw scope-only or warning-allowing.
- No docs describe descriptor string maps as current source of truth.
- No docs say DB pentest/web proxy still live entirely inside main crate.
- Roadmap status accurately reflects completed phases and remaining future extraction candidates.

## Workstream 9: Final validation matrix

Run a final focused validation matrix. Adjust features as needed for actual repo names.

```bash
cargo fmt --all --check
cargo check --workspace --all-features
cargo test -p eggsec --lib
cargo test -p eggsec --features rest-api --lib
cargo test -p eggsec --features rest-api --test enforcement_matrix
cargo test -p eggsec-db-lab
cargo test -p eggsec-web-proxy
cargo test -p eggsec --features db-pentest --test db_pentest_adapter
cargo test -p eggsec --features web-proxy --test proxy_adapter_smoke
cargo test -p eggsec-tui
cargo check -p eggsec-cli --features rest-api
cargo check -p eggsec-tui --features db-pentest,web-proxy
```

If some feature combinations are known to have pre-existing failures, document them clearly in the closure commit message and in the plan completion notes.

## Expected deliverables

The closure implementation should produce:

1. Any small source fixes found by the audits.
2. Additional regression tests for raw dispatch, alias matching, preflight parity, and metadata coverage.
3. Documentation updates marking the dual-mode enforcement roadmap as closed.
4. A final validation note in the commit message listing commands run and any known pre-existing failures.

## Non-goals

- Do not extract every remaining domain crate in this closure pass.
- Do not change the manual CLI/TUI default posture.
- Do not add a manual REST mode.
- Do not redesign policy semantics unless a test exposes a contradiction with the documented contract.
- Do not remove preflight/evaluate APIs; they remain necessary for explainability and UI.

## Final acceptance criteria

The closure pass is complete when:

- Strict surfaces cannot bypass enforcement through raw dispatch.
- `ApprovedOperation` is required for REST/MCP/agent strict dispatch.
- Metadata covers all external operations and aliases correctly.
- Preflight and execution decisions match.
- Audit events are consistent and non-misleading.
- Domain crates remain enforcement-agnostic.
- Documentation accurately reflects the implemented architecture.
- Validation commands pass or pre-existing failures are documented with specificity.
