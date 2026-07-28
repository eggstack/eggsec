# Runtime Bridge

**Module:** `crates/eggsec/src/runtime_bridge/`

Bridges frontend-neutral `eggsec-runtime` DTOs to the engine's enforcement model. This is the security boundary between the daemon/runtime layer and the engine's policy system. The dependency direction is one-way: `eggsec` depends on `eggsec-runtime`, never vice versa.

## Purpose

The daemon and runtime crates operate with protocol-neutral types (`RuntimeSurface`, `RunRequest`, `TaskKind`). The engine enforces policy via `ExecutionSurface`, `OperationDescriptor`, and `EnforcementContext`. The runtime bridge converts between these type systems while preserving security invariants.

## Module Files

| File | Lines | Purpose |
|------|-------|---------|
| `surface.rs` | 142 | `RuntimeSurface` → `ExecutionSurface` conversion; `RuntimeBridgeError` enum |
| `descriptor.rs` | 424 | `TaskKind` → `OperationDescriptor` via `OperationMetadata` lookup |
| `manual.rs` | 438 | `preflight_run_request()` and `approve_run_request()` entry points |
| `bundle.rs` | 313 | `ApprovedRunRequest` bundle type and validated dispatch |
| `executor.rs` | 435 | `EggsecRuntimeExecutor` implementing `RuntimeTaskExecutor` trait |

## Key Types

### `RuntimeBridgeError`

Error enum with 6 variants covering all bridge failure modes:

| Variant | Cause |
|---------|-------|
| `UnknownSurface` | `RuntimeSurface::Unknown` cannot map to `ExecutionSurface` |
| `UnsupportedTaskKind` | Task kind not yet bridged (e.g., `PacketTraceroute`, `PacketSend`) |
| `MissingTarget` | Task kind requires a target but none was provided |
| `UnknownOperationId` | No registered metadata for the operation ID |
| `ManualOverrideRejected` | Manual override attempted on a strict surface |
| `EnforcementDenied` | Enforcement layer denied the operation |

### `ApprovedRunRequest`

Couples an `ApprovedOperation` token with the original `RunRequest`. Private fields prevent construction outside the bridge. This prevents approve-one-dispatch-another attacks where the request might be mutated between approval and dispatch.

## Flow

```
RuntimeSurface + RunRequest
        │
        ▼
  runtime_surface_to_execution_surface()    [surface.rs]
        │
        ▼
  descriptor_for_run_request()              [descriptor.rs]
  (TaskKind → operation_id → metadata → OperationDescriptor)
        │
        ▼
  EnforcementContext::evaluate() / approve() [crate::config]
        │
        ▼
  ApprovedRunRequest                        [bundle.rs]
  (approved token + request coupled)
        │
        ▼
  dispatch_approved_runtime_request()       [bundle.rs]
  (validates op + target match → dispatch_inner())
        │
        ▼
  TaskResult → task_result_to_outcome()     [executor.rs]
  → TaskOutcome
```

### Surface Conversion (`surface.rs`)

Maps each `RuntimeSurface` variant to its `ExecutionSurface` counterpart:

| RuntimeSurface | ExecutionSurface |
|----------------|-----------------|
| `CliManual` | `CliManual` |
| `CliManualStrict` | `CliManualStrict` |
| `TuiManual` | `TuiManual` |
| `TuiManualStrict` | `TuiManualStrict` |
| `Ci` | `Ci` |
| `McpServer` | `McpServer` |
| `RestApi` | `RestApi` |
| `GrpcApi` | `GrpcApi` |
| `SecurityAgent` | `SecurityAgent` |
| `Unknown` | **Error** |

### TaskKind Resolution (`descriptor.rs`)

`resolve_operation_and_target()` maps ~27 `TaskKind` variants to canonical operation IDs:

| TaskKind | Operation ID | Target |
|----------|-------------|--------|
| `PortScan` | `scan-ports` | Required |
| `EndpointScan` | `scan-endpoints` | Required |
| `Fingerprint` | `fingerprint` | Required |
| `Waf` | `waf-detect` | Required |
| `WafStress` | `waf-stress` | Required |
| `Pipeline` | `pipeline` | Required |
| `Recon` | `recon` | Required |
| `LoadTest` | `load-test` | Required |
| `Fuzz` | `fuzz` | Required |
| `StressTest` | `stress-test` | Required |
| `GraphQl` | `graphql` | Required |
| `OAuth` | `oauth` | Required |
| `AuthTest` | `auth-test` | Required |
| `Nse` | `nse` | Required |
| `Hunt` | `hunt` | Required |
| `Browser` | `browser` | Required |
| `Compliance` | `compliance` | Required |
| `Vuln` | `vuln` | Required |
| `DbPentest` | `db-pentest` | Required |
| `PacketCapture` | `packet` | None |
| `Storage` | `storage` | None |
| `Integrations` | `integrations` | None |
| `Workflow` | `workflow` | None |
| `Wireless` | `wireless` | None |
| `WirelessActive` | `wireless` | None |
| `Intercept` | `proxy-intercept` | None |
| `C2` | `c2` | None |
| `PacketTraceroute` | *Unsupported* | — |
| `PacketSend` | *Unsupported* | — |

The resolved operation ID is looked up in `ALL_OPERATION_METADATA` to produce the full `OperationDescriptor` (risk tier, mode, capabilities, scope requirements, feature gates).

### Preflight & Approval (`manual.rs`)

Two entry points for callers (daemon, MCP server, etc.):

```rust
pub fn preflight_run_request(
    surface: RuntimeSurface,
    policy: ExecutionPolicy,
    loaded_scope: LoadedScope,
    request: &RunRequest,
    manual_override: Option<&ManualOverride>,
) -> Result<PreflightResult, RuntimeBridgeError>

pub fn approve_run_request(
    surface: RuntimeSurface,
    policy: ExecutionPolicy,
    loaded_scope: LoadedScope,
    request: &RunRequest,
    manual_override: Option<&ManualOverride>,
) -> Result<ApprovedOperation, RuntimeBridgeError>
```

Both functions:
1. Convert `RuntimeSurface` → `ExecutionSurface` (rejects `Unknown`)
2. Convert `RunRequest` → `OperationDescriptor` (rejects unsupported task kinds)
3. Create `EnforcementContext::for_surface(exec_surface, policy, loaded_scope)`
4. Branch on surface type:
   - **Manual surfaces** (`honors_manual_override() == true`): Use `approve_manual()` which supports operator overrides
   - **Strict/automated surfaces**: Reject any manual override with `ManualOverrideRejected`; use `approve()` which only allows `Allow` outcomes

### Bundle & Dispatch (`bundle.rs`)

`approve_run_request_bundle()` creates the coupled `ApprovedRunRequest`:

```rust
pub fn approve_run_request_bundle(
    surface: RuntimeSurface,
    policy: ExecutionPolicy,
    loaded_scope: LoadedScope,
    request: RunRequest,
    manual_override: Option<&ManualOverride>,
) -> Result<ApprovedRunRequest, RuntimeBridgeError>
```

`dispatch_approved_runtime_request()` validates before dispatch:

1. Re-resolves the `OperationDescriptor` from the request
2. Validates `approved.descriptor().operation == current_descriptor.operation`
3. Validates `approved.descriptor().target == current_descriptor.target`
4. Only then delegates to `dispatch_inner()`

These anti-tamper checks prevent approve-one-dispatch-another attacks.

### Executor (`executor.rs`)

`EggsecRuntimeExecutor` implements `eggsec_runtime::RuntimeTaskExecutor`, the trait the daemon runtime calls to execute tasks:

```rust
pub struct EggsecRuntimeExecutor {
    policy: ExecutionPolicy,
}
```

**Execution flow:**
1. Check cancellation (`cancel.is_cancelled()`)
2. Reject `RuntimeSurface::Unknown`
3. Resolve scope: strict surfaces require explicit `LoadedScope` from disk; permissive manual surfaces use `default_empty()`
4. Call `approve_run_request_bundle()` for full enforcement
5. Call `dispatch_approved_runtime_request()` racing against cancellation via `tokio::select!`
6. Forward progress from `mpsc` channel to `RuntimeEventSink`
7. Convert `TaskResult` → `TaskOutcome` via `task_result_to_outcome()`

## Trust Model

1. **Approval** produces an `ApprovedRunRequest` capturing both the token and the request at a single point in time.
2. **Dispatch** re-resolves the descriptor and validates operation ID + target match — preventing approve-one-dispatch-another attacks.
3. **Surface/profile consistency** is enforced at the enforcement layer during approval; the bundle preserves the approved surface for audit.
4. **Scope** for strict surfaces must come from `LoadedScope` (not raw `Scope`); manual surfaces use `default_empty` fallback.

## Invariants

1. `RuntimeSurface::Unknown` is never executable — always errors.
2. Manual surfaces retain operator-directed semantics (even daemon-backed).
3. Automated surfaces never honor manual overrides.
4. Any new `RuntimeSurface` variant must update conversion tests.

## Architecture Guards

- No TUI dependencies in this module.
- No transport dependencies (axum, tonic, etc.).
- Dependency direction: `eggsec` → `eggsec-runtime` (not reverse).
- All daemon-dispatched operations must pass through `approve_run_request_bundle()` before execution.

## See Also

- [runtime.md](runtime.md) — `eggsec-runtime` crate (task lifecycle, `Runtime`, `RuntimeTaskExecutor` trait)
- [daemon.md](daemon.md) — Daemon host that uses the bridge
- [config.md](config.md) — `EnforcementContext`, `ExecutionPolicy`, `OperationMetadata`
- [../docs/ARCHITECTURE.md](../docs/ARCHITECTURE.md) — Section 4.8 (Daemon / Runtime execution flow)
