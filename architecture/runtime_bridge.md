# Runtime Bridge

**Module:** `crates/eggsec/src/runtime_bridge/` (6 files, ~1,833 lines)

Bridges frontend-neutral `eggsec-runtime` DTOs to the engine's enforcement model. This is the security boundary between the daemon/runtime layer and the engine's policy system. The dependency direction is one-way: `eggsec` depends on `eggsec-runtime`, never vice versa.

Parent overview: [overview.md](overview.md). Related: [dispatch.md](dispatch.md), [runtime.md](runtime.md), [daemon.md](daemon.md).

## Purpose

The daemon and runtime crates operate with protocol-neutral types (`RuntimeSurface`, `RunRequest`, `TaskKind`). The engine enforces policy via `ExecutionSurface`, `OperationDescriptor`, and `EnforcementContext`. The runtime bridge converts between these type systems while preserving security invariants.

The bridge is the **only** place where runtime DTOs are converted to enforcement types. It never hardcodes `CliManual` or `default_empty` scope — the `EggsecRuntimeExecutor` uses the actual session surface and scope from the runtime context.

## Module Files

| File | Lines | Purpose |
|------|-------|---------|
| `mod.rs` | 35 | Module root; re-exports all public types; documents invariants |
| `surface.rs` | ~210 | Bidirectional surface conversion (`RuntimeSurface` ↔ `ExecutionSurface`, exhaustive; `Unknown` rejected wire → engine); `RuntimeBridgeError` enum (7 variants) |
| `descriptor.rs` | 459 | `TaskKind` → `OperationDescriptor` via `OperationMetadata` lookup; `resolve_operation_and_target()` delegates to the single wire-side match |
| `manual.rs` | 438 | `preflight_run_request()` and `approve_run_request()` entry points |
| `bundle.rs` | ~340 | `ApprovedRunRequest` bundle type; `approve_run_request_bundle()`; `dispatch_approved_runtime_request()` with anti-tamper validation → canonical `execute_approved` |
| `executor.rs` | ~300 | `EggsecRuntimeExecutor` implementing `RuntimeTaskExecutor` trait; envelope via engine-owned `dispatch::task_result_envelope`; cancellation via shared `race_with_cancel` |

## Key Types

### `RuntimeBridgeError`

Error enum with **7 variants** covering all bridge failure modes (`surface.rs:6–37`):

| Variant | Cause | Line |
|---------|-------|------|
| `UnknownSurface` | `RuntimeSurface::Unknown` cannot map to `ExecutionSurface` | `:8` |
| `UnsupportedTaskKind` | Task kind with no canonical mapping (none currently; all 29 `TaskKind` variants map) | `:12` |
| `MissingTarget` | Task kind requires a target but none was provided | `:16` |
| `UnknownOperationId` | No registered metadata for the operation ID | `:20` |
| `InvalidTarget` | Target failed validation for the given operation | `:24` |
| `ManualOverrideRejected` | Manual override attempted on a strict surface | `:31` |
| `EnforcementDenied` | Enforcement layer denied the operation | `:35` |

### `ApprovedRunRequest`

Couples an `ApprovedOperation` token with the original `RunRequest` (`bundle.rs:34–57`). Private fields prevent construction outside the bridge. This prevents approve-one-dispatch-another attacks where the request might be mutated between approval and dispatch.

Methods:
- `approved()` → `&ApprovedOperation`
- `request()` → `&RunRequest`
- `into_parts()` → `(ApprovedOperation, RunRequest)`

## Flow

```
RuntimeSurface + RunRequest
        │
        ▼
  runtime_surface_to_execution_surface()    [surface.rs:44]
  (10 variants → ExecutionSurface; Unknown → Err)
        │
        ▼
  descriptor_for_run_request()              [descriptor.rs:15]
  (TaskKind → operation_id → metadata → OperationDescriptor; all 29 kinds map
  to an operation, target-less/interface families fail explicitly downstream
  as InvalidTarget; packet traceroute/send share the `packet` family)
        │
        ▼
  EnforcementContext::evaluate() / approve() [crate::config]
  (Manual: approve_manual; Strict: approve + no override)
        │
        ▼
  ApprovedRunRequest                        [bundle.rs:34]
  (approved token + request coupled at single point in time)
        │
        ▼
  dispatch_approved_runtime_request()       [bundle.rs]
  (re-resolve descriptor → exact match → canonical execute_approved())
        │
        ▼
  TaskResult → dispatch::task_result_envelope()  [canonical_execution.rs]
  → TaskOutcome::Result(envelope) (stable wire kind + summary; single owner —
  TUI and daemon adapters consume it, never a parallel TaskResult match)
```

### Surface Conversion (`surface.rs:44–59`)

Maps each `RuntimeSurface` variant to its `ExecutionSurface` counterpart. This is the security boundary — `Unknown` surfaces are rejected rather than silently mapped to a permissive profile.

| RuntimeSurface | ExecutionSurface | `honors_manual_override()` |
|----------------|-----------------|---------------------------|
| `CliManual` | `CliManual` | **true** |
| `CliManualStrict` | `CliManualStrict` | false |
| `TuiManual` | `TuiManual` | **true** |
| `TuiManualStrict` | `TuiManualStrict` | false |
| `Ci` | `Ci` | false |
| `McpServer` | `McpServer` | false |
| `RestApi` | `RestApi` | false |
| `GrpcApi` | `GrpcApi` | false |
| `SecurityAgent` | `SecurityAgent` | false |
| `Unknown` | **Error** (`UnknownSurface`) | — |

Only `CliManual` and `TuiManual` honor manual overrides (`config/policy.rs:412–413`).

### TaskKind Resolution (`descriptor.rs`)

`resolve_operation_and_target()` maps all **29** `TaskKind` variants to canonical operation IDs (Phase 0: packet traceroute/send explicitly share the `packet` family; no unsupported wire kinds remain):

| TaskKind | Operation ID | Target Required | Line |
|----------|-------------|----------------|------|
| `PortScan` | `scan-ports` | Yes | `:41` |
| `EndpointScan` | `scan-endpoints` | Yes | `:42` |
| `Fingerprint` | `fingerprint` | Yes | `:43` |
| `Waf` | `waf-detect` | Yes | `:44` |
| `WafStress` | `waf-stress` | Yes | `:45` |
| `Pipeline` | `pipeline` | Yes | `:46` |
| `Recon` | `recon` | Yes | `:47` |
| `LoadTest` | `load-test` | Yes | `:48` |
| `Fuzz` | `fuzz` | Yes | `:49` |
| `StressTest` | `stress-test` | Yes | `:50` |
| `PacketCapture` | `packet` | None | `:51` |
| `PacketTraceroute` | `packet` | Yes | `:52` |
| `PacketSend` | `packet` | Yes | `:53` |
| `GraphQl` | `graphql` | Yes | `:54` |
| `OAuth` | `oauth` | Yes | `:55` |
| `AuthTest` | `auth-test` | Yes | `:56` |
| `Nse` | `nse` | Yes | `:57` |
| `Hunt` | `hunt` | Yes | `:58` |
| `Browser` | `browser` | Yes | `:59` |
| `Compliance` | `compliance` | Yes | `:60` |
| `Storage` | `storage` | None | `:61` |
| `Integrations` | `integrations` | None | `:62` |
| `Workflow` | `workflow` | None | `:63` |
| `Vuln` | `vuln` | Yes | `:64` |
| `Wireless` | `wireless` | None | `:65` |
| `WirelessActive` | `wireless` | None | `:66` |
| `DbPentest` | `db-pentest` | Yes | `:67` |
| `Intercept` | `proxy-intercept` | Optional | `:68` |
| `C2` | `c2` | Optional | `:69` |

The resolved operation ID is looked up in `ALL_OPERATION_METADATA` to produce the full `OperationDescriptor` (risk tier, mode, capabilities, scope requirements, feature gates). `descriptor_for_run_request()` uses `metadata.try_descriptor_for_target()` for validated construction (`descriptor.rs:25–30`).

### Runtime task tuning

Task payloads expose the execution tunables that dispatch applies, including
load-test request count and concurrency, scan concurrency/ports/timeouts,
fuzz mode/mutation/HTTP and GraphQL/OAuth options, WAF techniques, packet
capture/send limits, GraphQL/OAuth/auth settings, database budgets, proxy
listen/dry-run settings, and C2 dry-run mode. These fields are optional so
older serialized requests retain their existing defaults. For compatibility,
`LoadTestParams.connections` supplies both the request count and concurrency
when the newer `requests` field is absent.

### Preflight & Approval (`manual.rs:17–77`)

Two entry points for callers (daemon, MCP server, etc.):

```rust
pub fn preflight_run_request(
    surface: RuntimeSurface,
    policy: ExecutionPolicy,
    loaded_scope: LoadedScope,
    request: &RunRequest,
    manual_override: Option<&ManualOverride>,
) -> Result<PreflightResult, RuntimeBridgeError>   // manual.rs:17

pub fn approve_run_request(
    surface: RuntimeSurface,
    policy: ExecutionPolicy,
    loaded_scope: LoadedScope,
    request: &RunRequest,
    manual_override: Option<&ManualOverride>,
) -> Result<ApprovedOperation, RuntimeBridgeError>   // manual.rs:47
```

Both functions:
1. Convert `RuntimeSurface` → `ExecutionSurface` (rejects `Unknown`)
2. Convert `RunRequest` → `OperationDescriptor` (rejects unsupported task kinds)
3. Create `EnforcementContext::for_surface(exec_surface, policy, loaded_scope)`
4. Branch on surface type:
   - **Manual surfaces** (`honors_manual_override() == true`): Use `approve_manual()` which supports operator overrides (`manual.rs:60–64`)
   - **Strict/automated surfaces**: Reject any manual override with `ManualOverrideRejected` (`manual.rs:66–69`); use `approve()` which only allows `Allow` outcomes (`manual.rs:71–76`)

The `approve_manual` vs `approve` split is the core security distinction: permissive manual surfaces can escalate through overrides, strict surfaces cannot.

### Bundle & Dispatch (`bundle.rs:64–116`)

`approve_run_request_bundle()` creates the coupled `ApprovedRunRequest` (`bundle.rs:64`):

```rust
pub fn approve_run_request_bundle(
    surface: RuntimeSurface,
    policy: ExecutionPolicy,
    loaded_scope: LoadedScope,
    request: RunRequest,
    manual_override: Option<&ManualOverride>,
) -> Result<ApprovedRunRequest, RuntimeBridgeError>
```

`dispatch_approved_runtime_request()` validates before dispatch (Phase 1: canonical boundary):

1. Calls `bundle.into_parts()` to get `(approved, request)`
2. Re-resolves the `OperationDescriptor` from the current request
3. Requires exact binding via `approved.matches_descriptor(&current)` (derived `PartialEq`; all policy-relevant fields participate automatically)
4. Preserves granular `operation` / `normalized_target` diagnostics for approve-one-dispatch-another mutations
5. Converts `TaskKind → CanonicalOperationRequest::from_task_kind()` (exhaustive) and invokes `dispatch::execute_approved()` — binding re-checked at executor entry, single executor owner shared with embedded TUI execution

These anti-tamper checks prevent approve-one-dispatch-another attacks. The checks are **fail-closed** — any mismatch returns an error before any engine code executes.

### Executor (`executor.rs:33–373`)

`EggsecRuntimeExecutor` implements `eggsec_runtime::RuntimeTaskExecutor`, the trait the daemon runtime calls to execute tasks (`executor.rs:283`):

```rust
pub struct EggsecRuntimeExecutor {
    policy: ExecutionPolicy,
}
```

**Execution flow** (`executor.rs:284–372`):

1. Check cancellation (`cancel.is_cancelled()`) — pre-cancelled tasks never start detached work
2. Reject `RuntimeSurface::Unknown`
3. Resolve scope via `resolve_loaded_scope()`
   - Strict surfaces: require explicit `LoadedScope` from disk; fail closed if unavailable
   - Permissive manual surfaces (`CliManual`/`TuiManual`): use `default_empty()`
4. Call `approve_run_request_bundle()` for full enforcement
5. Log approved operation for audit
6. Spawn progress forwarder task
7. Call `dispatch_approved_runtime_request()` through the shared
   `eggsec-runtime::race_with_cancel` primitive (same primitive as the embedded
   TUI adapter — terminal/cancel semantics match by construction; the progress
   forwarder drains on success and is aborted when cancellation wins)
8. Forward progress from `mpsc` channel to `RuntimeEventSink`
9. Convert `TaskResult` → `TaskOutcome::Result(envelope)` via the single
   engine-owned `dispatch::task_result_envelope()` mapping (no parallel
   `TaskResult` match in the executor)

The `resolve_loaded_scope()` method (`executor.rs:62–113`) determines scope resolution strategy:
- If session has explicit scope with a path → loads from disk via `crate::config::load_scope()`
- If session has explicit scope but no path → returns `None` (caller fails closed)
- If permissive surface (`CliManual`/`TuiManual`) → `LoadedScope::default_empty()`
- If strict surface without explicit scope → `None` (fail closed)

## Trust Model

1. **Approval** produces an `ApprovedRunRequest` capturing both the token and the request at a single point in time (`bundle.rs:11–12`).
2. **Dispatch** re-resolves the descriptor and requires exact binding (`matches_descriptor`) — preventing approve-one-dispatch-another attacks (`bundle.rs:77–83`).
3. **Surface/profile consistency** is enforced at the enforcement layer during approval; the bundle preserves the approved surface for audit.
4. **Scope** for strict surfaces must come from `LoadedScope` (not raw `Scope`); manual surfaces use `default_empty` fallback (`executor.rs:99–112`).
5. **No hardcoded permissive defaults** — the executor uses the actual session surface and scope from `RuntimeExecutionContext`.

## Invariants

1. **`RuntimeSurface::Unknown` is never executable** — always errors (`surface.rs:57`).
2. **Manual surfaces retain operator-directed semantics** (even daemon-backed) — `CliManual` and `TuiManual` use `approve_manual()` (`manual.rs:59–64`).
3. **Automated surfaces never honor manual overrides** — rejected with `ManualOverrideRejected` before enforcement evaluation (`manual.rs:66–69`).
4. **Any new `RuntimeSurface` variant must update conversion tests** — `surface.rs` unit tests plus the `runtime_contract_closure` integration test (both fail until mapped).
5. **`dispatch_approved_runtime_request` validates both operation ID and target** — mismatches are rejected with explicit errors (`bundle.rs:95–110`).
6. **`EggsecRuntimeExecutor` must not hardcode `CliManual` or `default_empty` scope** — architecture guards enforce actual session context usage.
7. **Phase 3 closure (2026-09-11)**: `RuntimeSurface` is a wire DTO with an exhaustive bidirectional bridge (`Unknown` rejected wire → engine); engine operation-ID/target adapters delegate to the single wire-side `TaskKind` match (guard 94); envelope conversion is single-owned (`dispatch::task_result_envelope`, guard 95); both adapters route through `race_with_cancel` with no ad-hoc select (guard 86). Closure tests: `crates/eggsec/tests/runtime_contract_closure.rs` (guard 97).

## Architecture Guards

- No TUI dependencies in this module.
- No transport dependencies (axum, tonic, etc.).
- Dependency direction: `eggsec` → `eggsec-runtime` (not reverse).
- All daemon-dispatched operations must pass through `approve_run_request_bundle()` before execution.
- `EggsecRuntimeExecutor` uses session-provided surface and scope, not hardcoded values.

## Testing

The bridge module has extensive test coverage across all files:

- **Surface conversion** (`surface.rs:62–148`): Tests all 9 known mappings, rejects `Unknown`, verifies `honors_manual_override()` for permissive vs strict surfaces.
- **Descriptor resolution** (`descriptor.rs:77–459`): Tests every `TaskKind` variant individually, verifies unsupported kinds error, checks `requires_explicit_scope` for agent-exposable ops.
- **Preflight & approval** (`manual.rs:88–438`): Tests preflight/approve paths for CLI/TUI manual, strict, MCP, REST, gRPC, CI, SecurityAgent surfaces; override rejection; daemon-backed manual surfaces remain manual.
- **Bundle & dispatch** (`bundle.rs`): Tests bundle capture, strict surface rejection, operation mismatch detection, target mismatch detection, surface preservation.
- **Executor** (`executor.rs`): Tests envelope conversion for port scan, error, and load test variants (via the single engine-owned mapping).
- **Application boundary** (`crates/eggsec/tests/canonical_dispatch_ownership.rs`, mandatory path): per-family normalization/identity/binding/route equivalence plus daemon-bundle cancel race.
- **Runtime-contract closure** (`crates/eggsec/tests/runtime_contract_closure.rs`, guard 97): surface round-trips, wire-identity agreement across all 29 `TaskKind` variants, wire JSON stability, no-target family failures, stable envelope kinds, embedded/daemon seam equivalence, approval-binding regression.

## See Also

- [runtime.md](runtime.md) — `eggsec-runtime` crate (task lifecycle, `Runtime`, `RuntimeTaskExecutor` trait)
- [daemon.md](daemon.md) — Daemon host that uses the bridge
- [dispatch.md](dispatch.md) — The dispatch layer this bridge feeds into
- [config.md](config.md) — `EnforcementContext`, `ExecutionPolicy`, `OperationMetadata`
- [overview.md](overview.md) — System-wide architecture, enforcement model
- [../docs/ARCHITECTURE.md](../docs/ARCHITECTURE.md) — Section 4.8 (Daemon / Runtime execution flow)

*Last verified against source: 2026-09-11 (Phase 3 closure)*
