# Milestone C: Runtime/Enforcement Bridge Plan

## Objective

Build the security-critical bridge between `eggsec-runtime`'s frontend-neutral DTOs and `eggsec`'s canonical enforcement model. This milestone should make it possible for daemon-backed execution to evaluate runtime requests under the correct manual or strict profile before real dispatch is wired in Milestone D.

The central rule: the daemon/runtime layer identifies the caller surface and request, but the `eggsec` enforcement layer decides whether the operation may execute.

## Product Constraint

Human-facing CLI/TUI must remain first-class and practical. A daemon-backed CLI/TUI session is still manual if its bound surface is `CliManual` or `TuiManual`. It should use manual permissive semantics by default, with warnings and targeted confirmations/overrides where appropriate.

MCP, agent, REST, gRPC, and other automated/programmatic surfaces must remain strict. They must fail closed and never honor manual override fields.

## Current Findings

`eggsec-runtime::RuntimeSurface` intentionally mirrors `eggsec::config::ExecutionSurface`, but conversion is currently noted as future work. This is a trust-boundary gap. The conversion must be explicit and exhaustively tested.

`eggsec-runtime::RunRequest` and `TaskKind` contain frontend-neutral task payloads. They now need conversion into the main engine's operation descriptors and eventually tool requests.

The main `eggsec` crate already has `EnforcementContext`, `OperationDescriptor`, `ManualOverride`, `ApprovedOperation`, and `EnforcedDispatcher`. This milestone should use those primitives rather than creating new policy logic in runtime or daemon crates.

## Dependency Direction

Do not make `eggsec-runtime` depend on `eggsec`. The runtime crate must stay dependency-light and frontend-neutral.

Place bridge code in the main `eggsec` crate or a new adapter module/crate that is allowed to depend on both:

- allowed: `eggsec` depends on `eggsec-runtime` and converts runtime DTOs into enforcement/domain types;
- not allowed: `eggsec-runtime` imports `eggsec::config` or `eggsec::tool`.

Recommended initial location:

```text
crates/eggsec/src/runtime_bridge.rs
```

or a module tree:

```text
crates/eggsec/src/runtime_bridge/mod.rs
crates/eggsec/src/runtime_bridge/surface.rs
crates/eggsec/src/runtime_bridge/descriptor.rs
crates/eggsec/src/runtime_bridge/manual.rs
```

Export only stable adapter functions needed by the daemon executor later.

## Work Item C1: Implement Exhaustive `RuntimeSurface -> ExecutionSurface` Conversion

### Desired behavior

Every runtime surface maps explicitly to the corresponding enforcement surface.

Expected mapping:

```text
RuntimeSurface::CliManual       -> ExecutionSurface::CliManual
RuntimeSurface::CliManualStrict -> ExecutionSurface::CliManualStrict
RuntimeSurface::TuiManual       -> ExecutionSurface::TuiManual
RuntimeSurface::TuiManualStrict -> ExecutionSurface::TuiManualStrict
RuntimeSurface::Ci              -> ExecutionSurface::Ci
RuntimeSurface::McpServer       -> ExecutionSurface::McpServer
RuntimeSurface::RestApi         -> ExecutionSurface::RestApi
RuntimeSurface::GrpcApi         -> ExecutionSurface::GrpcApi
RuntimeSurface::SecurityAgent   -> ExecutionSurface::SecurityAgent
RuntimeSurface::Unknown         -> error, not permissive default
```

`Unknown` must not silently map to a manual permissive profile. It should be rejected before execution unless the daemon/session creation layer explicitly resolved it to a configured concrete surface.

### Implementation guidance

Implement `TryFrom<eggsec_runtime::RuntimeSurface> for ExecutionSurface` if orphan rules allow it because `ExecutionSurface` is local to `eggsec`. Otherwise implement a named function:

```rust
pub fn runtime_surface_to_execution_surface(surface: RuntimeSurface) -> Result<ExecutionSurface, RuntimeBridgeError>
```

Add a dedicated error type such as `RuntimeBridgeError::UnknownSurface`.

### Tests

Add exhaustive tests asserting:

- every non-unknown runtime surface maps correctly;
- `Unknown` errors;
- strict runtime surfaces map to enforcement profiles where `honors_manual_override() == false`;
- manual runtime surfaces map to enforcement surfaces where manual override is honored only for `CliManual` and `TuiManual`.

## Work Item C2: Convert `RunRequest -> OperationDescriptor`

### Desired behavior

A frontend-neutral `RunRequest` should be convertible into the same kind of `OperationDescriptor` used by CLI/TUI/MCP/agent paths.

The conversion must not decide authorization. It only extracts operation ID, target, risk/mode/capability requirements from canonical metadata.

### Initial supported task set

Keep the first pass focused. Suggested task kinds:

- `PortScan`
- `EndpointScan`
- `Fingerprint`
- `Waf`
- `WafStress`
- `Pipeline`
- `Recon`
- `LoadTest`
- `Fuzz`

If any of these have unclear operation IDs or metadata gaps, start with the subset that cleanly maps to `ALL_OPERATION_METADATA` and explicitly return `UnsupportedTaskKind` for the rest.

### Implementation guidance

Add a function such as:

```rust
pub fn descriptor_for_run_request(request: &RunRequest) -> Result<OperationDescriptor, RuntimeBridgeError>
```

For each supported `TaskKind`:

1. Determine canonical operation ID.
2. Extract target if the operation has a target.
3. Resolve operation metadata using the existing registry/metadata functions.
4. Build the descriptor via canonical metadata helpers rather than duplicating risk/capability literals.

Do not hardcode risk tiers in the bridge unless there is no existing metadata helper. If hardcoding is unavoidable, add a test that checks consistency with operation metadata.

### Unsupported tasks

For task kinds not yet supported by the bridge, return a typed error:

```text
RuntimeBridgeError::UnsupportedTaskKind { kind: ... }
```

Do not silently downgrade to a generic operation.

### Tests

Add descriptor tests for every initially supported task kind:

- descriptor operation ID matches metadata;
- target is extracted correctly;
- descriptor requires explicit scope where metadata says it should;
- capability/risk/mode are consistent with canonical metadata.

Add tests for unsupported task kinds returning typed errors.

## Work Item C3: Add Runtime Request Preflight Helper

### Desired behavior

The daemon and future daemon-backed frontends should be able to preflight a runtime request before dispatch.

Suggested function:

```rust
pub fn preflight_run_request(
    surface: RuntimeSurface,
    policy: ExecutionPolicy,
    loaded_scope: LoadedScope,
    request: &RunRequest,
    manual_override: Option<&ManualOverride>,
) -> Result<PreflightResult, RuntimeBridgeError>
```

This should:

1. convert runtime surface to execution surface;
2. build `EnforcementContext::for_surface(...)`;
3. convert request to descriptor;
4. call existing `preflight_operation(...)`;
5. return the result.

### Manual vs automated behavior

Manual override may be passed only for manual surfaces. If an override is supplied for strict/automated surfaces, the helper should either ignore it with an explicit flag in the result or reject it. Prefer rejection in this bridge layer for clarity unless existing preflight behavior expects ignored overrides.

### Tests

- Manual permissive surface can produce `Warn` or `RequireConfirmation` with suggested flags.
- Manual strict surface does not honor override.
- MCP/agent/REST/gRPC surfaces do not honor override.
- Missing explicit scope for automated networked operation produces denial when descriptor requires explicit scope.

## Work Item C4: Add Runtime Request Approval Helper

### Desired behavior

The daemon executor in Milestone D should be able to ask the bridge for an `ApprovedOperation` before dispatch.

Suggested function:

```rust
pub fn approve_run_request(
    surface: RuntimeSurface,
    policy: ExecutionPolicy,
    loaded_scope: LoadedScope,
    request: &RunRequest,
    manual_override: Option<&ManualOverride>,
) -> Result<ApprovedOperation, RuntimeBridgeError>
```

Behavior:

- manual surfaces use `EnforcementContext::approve_manual(...)`;
- strict/automated surfaces use `EnforcementContext::approve(...)`;
- manual overrides are rejected or ignored for strict surfaces, but never honored;
- `Warn` can proceed only for manual permissive surfaces as currently defined by enforcement behavior;
- `RequireConfirmation` becomes an error unless manual override/interactive approval permits it on a manual surface.

### Tests

- `CliManual` can approve warnings.
- `TuiManual` can approve required confirmation only with matching manual override.
- `CliManualStrict` rejects confirmation even with override.
- `McpServer`, `RestApi`, `GrpcApi`, and `SecurityAgent` reject warning/confirmation/denial outcomes.
- `Unknown` surface errors before policy evaluation.

## Work Item C5: Preserve Manual CLI/TUI Ergonomics

### Desired behavior

The bridge must not accidentally convert daemon-backed manual use into agent-like strictness.

Add tests that represent daemon-backed manual CLI/TUI sessions:

- create or simulate a `RunRequest` with surface `CliManual`;
- use a default or manual-permissive policy/scope combination that would be a warning rather than a hard denial;
- assert it remains a manual warning/confirmation path, not an automated denial.

This is the key product guarantee: daemon-backed does not mean automated.

## Work Item C6: Add Documentation and Invariant Notes

Update architecture docs or add a short section to the daemon/frontend roadmap docs explaining:

- `RuntimeSurface` is a DTO mirror, not policy by itself;
- conversion to `ExecutionSurface` is the security boundary;
- `Unknown` is not executable;
- manual surfaces retain operator semantics whether embedded or daemon-backed;
- automated surfaces remain strict whether embedded or daemon-backed.

If the repository has architecture invariant tests, add a note that any new `RuntimeSurface` variant must update the conversion tests.

## Validation Commands

Run at minimum:

```bash
cargo test -p eggsec-runtime
cargo test -p eggsec --lib runtime_bridge
cargo test -p eggsec --tests metadata_consistency
cargo check -p eggsec --all-targets
```

If targeted test filters differ, run the full package tests:

```bash
cargo test -p eggsec
cargo check --workspace --all-targets
```

## Acceptance Checklist

- [ ] `RuntimeSurface -> ExecutionSurface` conversion exists and is exhaustive.
- [ ] `RuntimeSurface::Unknown` cannot execute.
- [ ] Initial supported `RunRequest` task kinds convert to canonical `OperationDescriptor`s.
- [ ] Unsupported task kinds return typed errors.
- [ ] Runtime preflight helper uses existing enforcement logic.
- [ ] Runtime approval helper returns `ApprovedOperation` only through existing enforcement APIs.
- [ ] Manual surfaces preserve operator-directed semantics.
- [ ] Automated surfaces never honor manual overrides.
- [ ] Tests cover all surface mappings and representative manual/strict outcomes.

## Handoff Notes

This milestone should not wire the daemon to real execution yet. It should produce the adapter layer that Milestone D can call from a real executor.
