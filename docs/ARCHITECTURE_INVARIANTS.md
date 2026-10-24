# Architecture Invariants

Normative rules that all code in the eggsec workspace must preserve. Violations are bugs regardless of whether tests currently catch them.

## Enforcement Invariants

1. **Centralized authorization**: All side-effecting operations must have an `OperationDescriptor` evaluated by `EnforcementContext::evaluate()` before execution. No handler may execute a tool without passing through enforcement.

2. **No automated overrides**: Automated surfaces (`McpStrict`, `AgentStrict`, `CiStrict`) must never honor `ManualOverride`. Only `CliManual`, `TuiManual`, and `DesktopGuiManual` surfaces honor overrides.

3. **Fail-closed strict**: Strict surfaces (`McpStrict`, `AgentStrict`, `CiStrict`) must fail closed on `Warn`, `RequireConfirmation`, or `Deny`. Only `Allow` permits dispatch.

4. **Type-level dispatch**: Strict programmatic surfaces (REST, MCP, gRPC, Agent) must dispatch only via `EnforcedDispatcher::dispatch_execution()` with an `ApprovedExecution` bundle (token + scope snapshot from the same context, via `approve_execution()`/`approve_manual_execution()`). Raw `dispatch_checked()` with `ApprovedOperation` alone remains only for scope-insensitive tools. Internal helpers (e.g., the pipeline orchestrator) may use raw dispatch only after caller-level enforcement has been satisfied.

5. **Token uniqueness**: `ApprovedOperation` tokens must not be reusable for a different tool or target. `dispatch_checked()` verifies both tool name (alias-aware) and target match.

6. **Scope provenance**: Automated networked operations requiring explicit scope must check `LoadedScope::is_explicit_manifest()`. `DefaultEmpty` scope must not satisfy this requirement.

7. **Domain crates don't authorize**: Domain crates (`eggsec-db-lab`, `eggsec-web-proxy`, `eggsec-mobile-lab`) must not decide authorization. They declare capabilities and execute; enforcement is the caller's responsibility. (`eggsec-nse` is an external published dependency, not a workspace crate, and is held to the same boundary.)

8. **Feature gates ≠ authorization**: Feature gates are compile-time guards, not authorization. Runtime policy evaluation via `EnforcementContext::evaluate()` must still apply even when features are enabled.

9. **Registry is passive**: `ToolRegistry` is a pure lookup container with no enforcement awareness. Enforcement is the dispatcher's responsibility.

## Execution Invariants

10. **Dry-run purity**: Operations in dry-run mode must be side-effect free. No network connections, file writes, or system modifications.

11. **Preflight is advisory**: Preflight evaluation (`preflight_operation()`) must produce advisory results only. It must not execute tools or produce side effects.

12. **Audit event emission**: Every enforcement decision (`Allow`, `Warn`, `RequireConfirmation`, `Deny`) must emit an `EnforcementAuditEvent` via `emit_audit_event()`.

13. **No silent error suppression**: Enforcement errors must be logged or propagated. `let _ =` on enforcement results is a bug.

14. **Operation metadata consistency**: All operation IDs, risk levels, capabilities, and surface exposure flags must be defined in the static `ALL_OPERATION_METADATA` registry. `DomainDescriptor` operation IDs must resolve to matching `OperationMetadata` entries. `docs/CAPABILITY_MATRIX.md` must stay consistent with metadata (validated by `crates/eggsec/tests/metadata_consistency.rs`). Individual surfaces must not hardcode divergent metadata.

## Frontend Invariants

15. **CLI scope loading**: CLI must load scope via `load_scope_with_source()` returning a `LoadedScope` with provenance. Default scope must be `LoadedScope::default_empty()`, not an implicit allow-all.

16. **TUI enforcement toggle**: `TuiEnforcementState::toggle_posture()` must update the enforcement profile, clear cached preflight, and re-evaluate on next dispatch.

17. **REST strict by default**: REST API uses `McpStrict` profile. Only `EnforcementOutcome::Allow` permits dispatch. `rest_exposable` metadata must be checked before policy evaluation.

18. **Agent profile validation**: `Agent::new()` must reject non-`AgentStrict` enforcement contexts. If `enforced_dispatcher` is present but `ApprovedOperation` is missing at dispatch time, agent must return a hard invariant error.

19. **CI has no dispatch path**: The CI handler is a passive quality gate. It must not import or use `ToolDispatcher`, `EnforcedDispatcher`, or any tool execution API.

## Structural Invariants

20. **Single canonical types**: `Severity` has a single canonical definition in `eggsec-core::types`. Other crates re-export, not redefine.

21. **Policy types centralized**: All enforcement types (`ExecutionSurface`, `ExecutionProfile`, `OperationDescriptor`, `EnforcementContext`, `EnforcementOutcome`, `ApprovedOperation`) are defined in `eggsec::config` and re-exported. No competing definitions.

22. **Regression test guard**: The enforced dispatch regression test (`crates/eggsec/tests/enforced_dispatch_regression.rs`) must remain green. New raw dispatch sites must be justified and added to the narrow allowlist.

23. **Dependency direction**: Leaf crates (`eggsec-core`, `eggsec-output`, `eggsec-agent`) must not depend on the main `eggsec` crate. The dependency graph must remain acyclic.

24. **No circular workspace deps**: Workspace crates must not create circular dependency chains.

## Scope Invariants

25. **Private IP blocking**: Private IPs are blocked when no scope rules exist and `require_explicit_scope` is false. Scope rules like `allow 10.0.0.0/8` correctly match private IPs before the fallback block.

26. **Exclusion precedence**: Exclusion rules are checked before allowed rules. A target matching both is denied.

27. **Scope file provenance**: `ScopeSource::DefaultEmpty` is not an explicit manifest. `ConfigFile`, `CliScopeFile`, and `GeneratedPreset` are explicit manifests.

## Internal Dispatch Rules

28. **Raw dispatch**: `ToolDispatcher::dispatch()` is `pub(crate)` and `#[doc(hidden)]`. Only `EnforcedDispatcher` internals and the pipeline orchestrator may use it. All other callers must use `EnforcedDispatcher::dispatch_checked()`. The orchestrator is an internal implementation detail; callers must satisfy enforcement before constructing it.

29. **Runtime bridge direction**: `eggsec` depends on `eggsec-runtime` (not reverse). The `runtime_bridge` module converts `eggsec-runtime` DTOs (`RuntimeSurface`, `RunRequest`, `TaskKind`) to engine types (`ExecutionSurface`, `OperationDescriptor`, `EnforcementContext`). `Unknown` surface always errors. Strict surfaces never honor manual overrides through the bridge.

30. **Daemon engine dependency**: `eggsec-daemon` depends on `eggsec` (engine) only behind the `service` profile (deprecated alias: `full-executor`). Without it, `NoopExecutorStub` rejects all tasks and the daemon publishes an **empty** task-kind list, because the published list is derived from the executor's declared support. The `EggsecRuntimeExecutor` must not hold `Arc<Runtime>` to avoid circular ownership.

## Runtime Bridge Invariants

31. **Session-derived surface**: The real daemon executor (`EggsecRuntimeExecutor`) must derive its `ExecutionSurface` from the `RuntimeExecutionContext` provided by the runtime, not from hardcoded defaults. The runtime populates context from the owning `RuntimeSession`, not from client-submitted request fields.

32. **Session-derived scope**: The real daemon executor must resolve scope from `RuntimeExecutionContext.scope`. For strict surfaces, the executor must fail closed if no explicit scope is available. For permissive manual surfaces, `LoadedScope::default_empty()` is permitted.

33. **ApprovedRunRequest bundle**: Dispatch through the runtime bridge must use an `ApprovedRunRequest` bundle that couples the `ApprovedOperation` token with the specific `RunRequest`. The bundle must validate that the approved operation descriptor matches the request's resolved descriptor before dispatching.

34. **Unknown surface never executes**: `RuntimeSurface::Unknown` must not reach execution. It may be resolved to a configured concrete default at session creation, but the executor must reject it.

35. **RuntimeExecutionContext origin**: `RuntimeExecutionContext` must be populated by the runtime from `RuntimeSession` state (surface, scope). Client-submitted `RunRequest.surface` is informational only and must not influence enforcement.

36. **Daemon capabilities reflect executor mode**: `RuntimeCapabilities` must reflect whether the daemon has a real executor (`full()`) or a no-op stub (`noop()`). No-op executors must not advertise task kinds they cannot execute.

37. **Close-session must not delete persisted session state**: `CloseSession` must persist a final snapshot (with `closed=true` and cancelled tasks) but must not call `delete_session()`. Session history is preserved for later inspection via `daemon history` / `daemon show`.

38. **Request surface is normalized to session surface**: The executor must derive its `ExecutionSurface` from `RuntimeExecutionContext` (session-bound), not from `RunRequest.surface`. Client-submitted surface is informational only and must not influence enforcement decisions or persisted state.

39. **Unsupported task kinds are rejected before dispatch**: If the daemon's runtime capabilities do not include a submitted `TaskKind`, the task must be rejected during submission (before dispatch) rather than silently dropped or returned as an unexpected runtime error.

## Desktop GUI Invariants

40. **The desktop GUI is daemon-only**: `eggsec-gui` must not depend on `eggsec` (the engine), `eggsec-daemon`, `eggsec-cli` or `eggsec-tui`, and must not import them from any source file. Its only workspace dependencies are `eggsec-frontend-client`, `eggsec-runtime`, `eggsec-daemon-protocol`, `eggsec-ui-model` and `eggsec-output`. All engine traffic travels over the daemon's Unix socket; the GUI performs no outbound HTTP and declares no TLS stack of its own. Enforced by architecture guards 155–157.

41. **The published schema is descriptive, never authoritative**: The operation/request schema the daemon publishes (`DescribeOperations`) is a *description* of what a request looks like, not a source of authorization or of the truth about whether a task may run. The GUI may use it to generate a form and to give advisory type/bounds feedback, and it may deserialize the assembled request into the same `RunRequest` the engine will deserialize — but the engine's own validation remains the authority that decides whether a task runs. A published schema must never encode a per-operation policy rule, and no frontend may treat "the schema allows it" as "it is allowed".

    The same rule extends to a published operation's **`required_features`** list. A requirement list is a statement about *what an operation needs*, not a report of *whether the connected daemon has it*. `required_features: ["nse"]` says the operation is gated on the `nse` feature; it does not say the daemon in front of you was built with `nse`. The fact about the daemon is the **published enabled-feature set** the daemon advertises, and a requirement list must never be read as one. A requirement list also does not say whether the operation is *exposed* on the current surface, whether the daemon *advertises* the capability, or whether the engine would *authorize* it — those are separate facts with separate owners. A client that treats a requirement list as a state report will report a feature-gated operation as unavailable on a daemon that has the feature, or as available on one that does not. When the enabled-feature set is **unknown** — an absent field, which is not the same as an empty set — the honest verdict is that availability cannot be verified; it is never "the feature is missing".

42. **The GUI's form is generated, not written**: The GUI must contain no per-operation code — no `match` or `switch` over the published task-kind variant names, and no hand-written table of operation names or fields. A task kind the engine has not published yet must get a form with no GUI change. Enforced by architecture guard 159.

43. **The GUI's retention is bounded**: The GUI must cap the log lines it retains per task and the log lines it forwards per flush, and must report what it drops rather than discarding silently. Unbounded retention is a disqualifying defect, not a tuning problem. Enforced by architecture guard 160.

44. **The GUI's coverage model is derived, not transcribed**: The GUI must not name an operation id or a capability name in its production sources or in its frontend. Its coverage table must be computed from what the daemon publishes — the task kinds, the operations, their required features, the advertised capabilities, and the published enabled-feature set. A learned denial from a live engine refusal wins over every static verdict, because the engine's own word about the running daemon is the last word. Enforced by architecture guard 161.

45. **The GUI's report display carries no remote origin and no executable content**: The GUI must not inject an engine-produced HTML document into its webview, and must not relax its Content Security Policy to make room for one. The report's content is attacker-influenced — banners, service names and finding text come from the target — so the CSP is the control that stops it from doing anything. The frontend must render report content as text, with no markup-injection sink, and the HTML report format stays export-only: the exported file is the operator's artifact, opened in the operator's browser. Enforced by architecture guard 162.
