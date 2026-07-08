# Architecture Invariants

Normative rules that all code in the eggsec workspace must preserve. Violations are bugs regardless of whether tests currently catch them.

## Enforcement Invariants

1. **Centralized authorization**: All side-effecting operations must have an `OperationDescriptor` evaluated by `EnforcementContext::evaluate()` before execution. No handler may execute a tool without passing through enforcement.

2. **No automated overrides**: Automated surfaces (`McpStrict`, `AgentStrict`, `CiStrict`) must never honor `ManualOverride`. Only `CliManual` and `TuiManual` surfaces honor overrides.

3. **Fail-closed strict**: Strict surfaces (`McpStrict`, `AgentStrict`, `CiStrict`) must fail closed on `Warn`, `RequireConfirmation`, or `Deny`. Only `Allow` permits dispatch.

4. **Type-level dispatch**: Strict programmatic surfaces (REST, MCP, gRPC, Agent) must use `EnforcedDispatcher::dispatch_checked()` with an `ApprovedOperation` token. The orchestrator is a tracked transitional exception that may only be reached after caller-level enforcement.

5. **Token uniqueness**: `ApprovedOperation` tokens must not be reusable for a different tool or target. `dispatch_checked()` verifies both tool name (alias-aware) and target match.

6. **Scope provenance**: Automated networked operations requiring explicit scope must check `LoadedScope::is_explicit_manifest()`. `DefaultEmpty` scope must not satisfy this requirement.

7. **Domain crates don't authorize**: Domain crates (`eggsec-db-lab`, `eggsec-web-proxy`, `eggsec-mobile-lab`, `eggsec-nse`) must not decide authorization. They declare capabilities and execute; enforcement is the caller's responsibility.

8. **Feature gates ≠ authorization**: Feature gates are compile-time guards, not authorization. Runtime policy evaluation via `EnforcementContext::evaluate()` must still apply even when features are enabled.

9. **Registry is passive**: `ToolRegistry` is a pure lookup container with no enforcement awareness. Enforcement is the dispatcher's responsibility.

## Execution Invariants

10. **Dry-run purity**: Operations in dry-run mode must be side-effect free. No network connections, file writes, or system modifications.

11. **Preflight is advisory**: Preflight evaluation (`preflight_operation()`) must produce advisory results only. It must not execute tools or produce side effects.

12. **Audit event emission**: Every enforcement decision (`Allow`, `Warn`, `RequireConfirmation`, `Deny`) must emit an `EnforcementAuditEvent` via `emit_audit_event()`.

13. **No silent error suppression**: Enforcement errors must be logged or propagated. `let _ =` on enforcement results is a bug.

14. **Operation metadata consistency**: All operation IDs, risk levels, capabilities, and surface exposure flags must be defined in the static `ALL_OPERATION_METADATA` registry. `DomainDescriptor` operation IDs must resolve to matching `OperationMetadata` entries. `docs/CAPABILITY_MATRIX.md` must stay consistent with metadata (validated by `tests/metadata_consistency.rs`). Individual surfaces must not hardcode divergent metadata.

## Frontend Invariants

15. **CLI scope loading**: CLI must load scope via `load_scope_with_source()` returning a `LoadedScope` with provenance. Default scope must be `LoadedScope::default_empty()`, not an implicit allow-all.

16. **TUI enforcement toggle**: `TuiEnforcementState::toggle_posture()` must update the enforcement profile, clear cached preflight, and re-evaluate on next dispatch.

17. **REST strict by default**: REST API uses `McpStrict` profile. Only `EnforcementOutcome::Allow` permits dispatch. `rest_exposable` metadata must be checked before policy evaluation.

18. **Agent profile validation**: `Agent::new()` must reject non-`AgentStrict` enforcement contexts. If `enforced_dispatcher` is present but `ApprovedOperation` is missing at dispatch time, agent must return a hard invariant error.

19. **CI has no dispatch path**: The CI handler is a passive quality gate. It must not import or use `ToolDispatcher`, `EnforcedDispatcher`, or any tool execution API.

## Structural Invariants

20. **Single canonical types**: `Severity` has a single canonical definition in `eggsec-core::types`. Other crates re-export, not redefine.

21. **Policy types centralized**: All enforcement types (`ExecutionSurface`, `ExecutionProfile`, `OperationDescriptor`, `EnforcementContext`, `EnforcementOutcome`, `ApprovedOperation`) are defined in `eggsec::config` and re-exported. No competing definitions.

22. **Regression test guard**: The enforced dispatch regression test (`tests/enforced_dispatch_regression.rs`) must remain green. New raw dispatch sites must be justified and added to the narrow allowlist.

23. **Dependency direction**: Leaf crates (`eggsec-core`, `eggsec-output`, `eggsec-agent`) must not depend on the main `eggsec` crate. The dependency graph must remain acyclic.

24. **No circular workspace deps**: Workspace crates must not create circular dependency chains.

## Scope Invariants

25. **Private IP blocking**: Private IPs are blocked when no scope rules exist and `require_explicit_scope` is false. Scope rules like `allow 10.0.0.0/8` correctly match private IPs before the fallback block.

26. **Exclusion precedence**: Exclusion rules are checked before allowed rules. A target matching both is denied.

27. **Scope file provenance**: `ScopeSource::DefaultEmpty` is not an explicit manifest. `ConfigFile`, `CliScopeFile`, and `GeneratedPreset` are explicit manifests.

## Transitional API Rules

28. **`with_execution_profile`**: **Removed** (Phase 2). Replaced by `with_execution_surface()` and direct `EnforcementContext` construction.

29. **`ensure_scope` / `ensure_scope_url`**: **Removed** (Phase 2). Scope checks are centralized in `EnforcementContext::evaluate()`.

30. **Raw dispatch**: `ToolDispatcher::dispatch()` is `pub(crate)` and `#[doc(hidden)]`. Only the Orchestrator and `EnforcedDispatcher` internals may use it. All other callers must use `EnforcedDispatcher::dispatch_checked()`.

31. **Runtime bridge direction**: `eggsec` depends on `eggsec-runtime` (not reverse). The `runtime_bridge` module converts `eggsec-runtime` DTOs (`RuntimeSurface`, `RunRequest`, `TaskKind`) to engine types (`ExecutionSurface`, `OperationDescriptor`, `EnforcementContext`). `Unknown` surface always errors. Strict surfaces never honor manual overrides through the bridge.

32. **Daemon engine dependency**: `eggsec-daemon` depends on `eggsec` (engine) only behind the `full-executor` feature flag. Without it, `NoopExecutorStub` rejects all tasks. The `EggsecRuntimeExecutor` must not hold `Arc<Runtime>` to avoid circular ownership.

## Runtime Bridge Invariants

33. **Session-derived surface**: The real daemon executor (`EggsecRuntimeExecutor`) must derive its `ExecutionSurface` from the `RuntimeExecutionContext` provided by the runtime, not from hardcoded defaults. The runtime populates context from the owning `RuntimeSession`, not from client-submitted request fields.

34. **Session-derived scope**: The real daemon executor must resolve scope from `RuntimeExecutionContext.scope`. For strict surfaces, the executor must fail closed if no explicit scope is available. For permissive manual surfaces, `LoadedScope::default_empty()` is permitted.

35. **ApprovedRunRequest bundle**: Dispatch through the runtime bridge must use an `ApprovedRunRequest` bundle that couples the `ApprovedOperation` token with the specific `RunRequest`. The bundle must validate that the approved operation descriptor matches the request's resolved descriptor before dispatching.

36. **Unknown surface never executes**: `RuntimeSurface::Unknown` must not reach execution. It may be resolved to a configured concrete default at session creation, but the executor must reject it.

37. **RuntimeExecutionContext origin**: `RuntimeExecutionContext` must be populated by the runtime from `RuntimeSession` state (surface, scope). Client-submitted `RunRequest.surface` is informational only and must not influence enforcement.

38. **Daemon capabilities reflect executor mode**: `RuntimeCapabilities` must reflect whether the daemon has a real executor (`full()`) or a no-op stub (`noop()`). No-op executors must not advertise task kinds they cannot execute.
