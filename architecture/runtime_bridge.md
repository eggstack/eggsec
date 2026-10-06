# Runtime Bridge

> **Corrections (2026-10-06)** — verified against source: module is **1,900**
> lines (was ~1,851); `descriptor.rs` 444→488 and `executor.rs` 295→300.
> `TaskKind` is **30** variants, not 29 — added the missing `Resume` row
> (operation `pipeline`, target `None`). The bundle dispatch path is stale in
> the doc: it calls `into_execution_parts()` + `dispatch::execute_approved_execution`,
> not `into_parts()` + `execute_approved`. Many `file:line` citations were
> re-anchored (surface `:44`→`:52`, bundle `:34`→`:41`, executor `:283`→`:129`).
> Dropped a "verifies unsupported kinds error" test claim that no longer matches
> source, and flagged a `Resume` test-coverage gap.

**Module:** `crates/eggsec/src/runtime_bridge/` (6 files, 1,900 lines)

Bridges frontend-neutral `eggsec-runtime` DTOs to the engine's enforcement model. This is the security boundary between the daemon/runtime layer and the engine's policy system. The dependency direction is one-way: `eggsec` depends on `eggsec-runtime`, never vice versa.

Parent overview: [overview.md](overview.md). Related: [dispatch.md](dispatch.md), [runtime.md](runtime.md), [daemon.md](daemon.md).

## Purpose

The daemon and runtime crates operate with protocol-neutral types (`RuntimeSurface`, `RunRequest`, `TaskKind`). The engine enforces policy via `ExecutionSurface`, `OperationDescriptor`, and `EnforcementContext`. The runtime bridge converts between these type systems while preserving security invariants.

The bridge is the **only** place where runtime DTOs are converted to enforcement types. It never hardcodes `CliManual` or `default_empty` scope — the `EggsecRuntimeExecutor` uses the actual session surface and scope from the runtime context.

## Module Files

| File | Lines | Purpose |
|------|-------|---------|
| `mod.rs` | 38 | Module root; re-exports all public types; documents invariants |
| `surface.rs` | 221 | Bidirectional surface conversion (`RuntimeSurface` ↔ `ExecutionSurface`, exhaustive; `Unknown` rejected wire → engine); `RuntimeBridgeError` enum (7 variants) |
| `descriptor.rs` | 488 | `TaskKind` → `OperationDescriptor` via `OperationMetadata` lookup; `resolve_operation_and_target()` delegates to the single wire-side match |
| `manual.rs` | 479 | `preflight_run_request()`, `approve_run_request()`, and `approve_run_request_execution()` entry points |
| `bundle.rs` | 374 | `ApprovedRunRequest` bundle type; `approve_run_request_bundle()`; `dispatch_approved_runtime_request()` with anti-tamper validation → canonical `execute_approved_execution` |
| `executor.rs` | 300 | `EggsecRuntimeExecutor` implementing `RuntimeTaskExecutor` trait; envelope via engine-owned `dispatch::task_result_envelope`; cancellation via shared `race_with_cancel` |

## Key Types

### `RuntimeBridgeError`

Error enum with **7 variants** covering all bridge failure modes (`surface.rs:6–37`):

| Variant | Cause | Line |
|---------|-------|------|
| `UnknownSurface` | `RuntimeSurface::Unknown` cannot map to `ExecutionSurface` | `:9` |
| `UnsupportedTaskKind` | Task kind with no canonical mapping (none currently; all 30 `TaskKind` variants map) | `:12` |
| `MissingTarget` | Task kind requires a target but none was provided | `:16` |
| `UnknownOperationId` | No registered metadata for the operation ID | `:20` |
| `InvalidTarget` | Target failed validation for the given operation | `:24` |
| `ManualOverrideRejected` | Manual override attempted on a strict surface | `:31` |
| `EnforcementDenied` | Enforcement layer denied the operation | `:35` |

### `ApprovedRunRequest`

Couples an `ApprovedExecution` bundle (token + scope snapshot from the same enforcement context) with the original `RunRequest` (`bundle.rs:41–74`). Private fields prevent construction outside the bridge. This prevents approve-one-dispatch-another attacks where the request might be mutated between approval and dispatch. Per-hop transport authorization uses the approval scope carried in the bundle, never a reloaded config or wildcard.

Methods:
- `approved()` → `&ApprovedOperation` (via the inner execution bundle)
- `execution()` → `&ApprovedExecution`
- `request()` → `&RunRequest`
- `into_parts()` → `(ApprovedOperation, RunRequest)`
- `into_execution_parts()` → `(ApprovedExecution, RunRequest)`

## Flow

```
RuntimeSurface + RunRequest
        │
        ▼
  runtime_surface_to_execution_surface()    [surface.rs:52]
  (10 variants → ExecutionSurface; Unknown → Err)
        │
        ▼
  descriptor_for_run_request()              [descriptor.rs:15]
  (TaskKind → operation_id → metadata → OperationDescriptor; all 30 kinds map
  to an operation, target-less/interface families fail explicitly downstream
  as InvalidTarget; packet traceroute/send share the `packet` family and
  Resume shares the `pipeline` family)
        │
        ▼
  EnforcementContext::evaluate() / approve() [crate::config]
  (Manual: approve_manual; Strict: approve + no override)
        │
        ▼
  ApprovedRunRequest                        [bundle.rs:41]
  (approved token + request coupled at single point in time)
        │
        ▼
  dispatch_approved_runtime_request()       [bundle.rs]
  (re-resolve descriptor → exact match → canonical execute_approved_execution())
        │
        ▼
  TaskResult → dispatch::task_result_envelope()  [canonical_execution.rs]
  → TaskOutcome::Result(envelope) (stable wire kind + summary; single owner —
  TUI and daemon adapters consume it, never a parallel TaskResult match)
```

### Surface Conversion (`surface.rs:52–67`)

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

Only `CliManual` and `TuiManual` honor manual overrides (`eggsec-policy/src/policy.rs:417`).

### TaskKind Resolution (`descriptor.rs`)

`resolve_operation_and_target()` maps all **30** `TaskKind` variants to canonical operation IDs (Phase 3: no per-kind match remains in the bridge — it delegates to the single wire-side match `TaskKind::{operation_id, canonical_target}` at `descriptor.rs:40-47`; packet traceroute/send share the `packet` family and `Resume` shares the `pipeline` family; no unsupported wire kinds remain). Effective mapping (derived, not per-kind arms):

| TaskKind | Operation ID | Target Required | Source |
|----------|-------------|----------------|------|
| `PortScan` | `scan-ports` | Yes | derived |
| `EndpointScan` | `scan-endpoints` | Yes | derived |
| `Fingerprint` | `fingerprint` | Yes | derived |
| `Waf` | `waf-detect` | Yes | derived |
| `WafStress` | `waf-stress` | Yes | derived |
| `Pipeline` | `pipeline` | Yes | derived |
| `Resume` | `pipeline` | None (target lives in the saved checkpoint) | derived |
| `Recon` | `recon` | Yes | derived |
| `LoadTest` | `load-test` | Yes | derived |
| `Fuzz` | `fuzz` | Yes | derived |
| `StressTest` | `stress-test` | Yes | derived |
| `PacketCapture` | `packet` | None | derived |
| `PacketTraceroute` | `packet` | Yes | derived |
| `PacketSend` | `packet` | Yes | derived |
| `GraphQl` | `graphql` | Yes | derived |
| `OAuth` | `oauth` | Yes | derived |
| `AuthTest` | `auth-test` | Yes | derived |
| `Nse` | `nse` | Yes | derived |
| `Hunt` | `hunt` | Yes | derived |
| `Browser` | `browser` | Yes | derived |
| `Compliance` | `compliance` | Yes | derived |
| `Storage` | `storage` | None | derived |
| `Integrations` | `integrations` | None | derived |
| `Workflow` | `workflow` | None | derived |
| `Vuln` | `vuln` | Yes | derived |
| `Wireless` | `wireless` | None | derived |
| `WirelessActive` | `wireless` | None | derived |
| `DbPentest` | `db-pentest` | Yes | derived |
| `Intercept` | `proxy-intercept` | Optional | derived |
| `C2` | `c2` | Optional | derived |

The resolved operation ID is looked up in `ALL_OPERATION_METADATA` to produce the full `OperationDescriptor` (risk tier, mode, capabilities, scope requirements, feature gates). `descriptor_for_run_request()` uses `metadata.try_descriptor_for_target()` for validated construction (`descriptor.rs:26–31`).

### Runtime task tuning

Task payloads expose the execution tunables that dispatch applies, including
load-test request count and concurrency, scan concurrency/ports/timeouts,
fuzz mode/mutation/HTTP and GraphQL/OAuth options, WAF techniques, packet
capture/send limits, GraphQL/OAuth/auth settings, database budgets, proxy
listen/dry-run settings, and C2 dry-run mode. These fields are optional so
older serialized requests retain their existing defaults. For compatibility,
`LoadTestParams.connections` supplies both the request count and concurrency
when the newer `requests` field is absent.

### Preflight & Approval (`manual.rs:17–86`)

Three entry points for callers (daemon, MCP server, etc.):

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

pub fn approve_run_request_execution(
    surface: RuntimeSurface,
    policy: ExecutionPolicy,
    loaded_scope: LoadedScope,
    request: &RunRequest,
    manual_override: Option<&ManualOverride>,
) -> Result<ApprovedExecution, RuntimeBridgeError>   // manual.rs:86
```

Both functions (all three entry points):
1. Convert `RuntimeSurface` → `ExecutionSurface` (rejects `Unknown`)
2. Convert `RunRequest` → `OperationDescriptor` (rejects unsupported task kinds)
3. Create `EnforcementContext::for_surface(exec_surface, policy, loaded_scope)`
4. Branch on surface type:
   - **Manual surfaces** (`honors_manual_override() == true`): Use `approve_manual()` which supports operator overrides (`manual.rs:60–65`)
   - **Strict/automated surfaces**: Reject any manual override with `ManualOverrideRejected` (`manual.rs:66–71`); use `approve()` which only allows `Allow` outcomes (`manual.rs:72–77`)

The `approve_manual` vs `approve` split is the core security distinction: permissive manual surfaces can escalate through overrides, strict surfaces cannot.

### Bundle & Dispatch (`bundle.rs:81–116`)

`approve_run_request_bundle()` creates the coupled `ApprovedRunRequest` (`bundle.rs:81`):

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

1. Calls `bundle.into_execution_parts()` to get `(execution, request)`, then `execution.approved()` (`bundle.rs:115–116`)
2. Re-resolves the `OperationDescriptor` from the current request (`bundle.rs:119–120`)
3. Requires exact binding via `approved.matches_descriptor(&current)` (derived `PartialEq`; all policy-relevant fields participate automatically) (`bundle.rs:127`)
4. Preserves granular `operation` / `normalized_target` diagnostics for approve-one-dispatch-another mutations (`bundle.rs:128–148`)
5. Converts `TaskKind → CanonicalOperationRequest::from_task_kind()` (exhaustive) and invokes `dispatch::execute_approved_execution()` (`bundle.rs:113`, `:153`) — the scope-sensitive canonical executor, binding re-checked at executor entry, single executor owner shared with embedded TUI execution

These anti-tamper checks prevent approve-one-dispatch-another attacks. The checks are **fail-closed** — any mismatch returns an error before any engine code executes.

### Executor (`executor.rs:36–300`)

`EggsecRuntimeExecutor` implements `eggsec_runtime::RuntimeTaskExecutor`, the trait the daemon runtime calls to execute tasks (`executor.rs:128–129`):

```rust
pub struct EggsecRuntimeExecutor {
    policy: ExecutionPolicy,
}
```

**Execution flow** (`executor.rs:129–226`):

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

The `resolve_loaded_scope()` method (`executor.rs:65–118`) determines scope resolution strategy:
- If session has explicit scope with a path → loads from disk via `crate::config::load_scope()`
- If session has explicit scope but no path → returns `None` (caller fails closed)
- If permissive surface (`CliManual`/`TuiManual`) → `LoadedScope::default_empty()`
- If strict surface without explicit scope → `None` (fail closed)

## Trust Model

1. **Approval** produces an `ApprovedRunRequest` capturing both the token and the request at a single point in time (`bundle.rs:41–46`; module contract at `bundle.rs:12–13`).
2. **Dispatch** re-resolves the descriptor and requires exact binding (`matches_descriptor`) — preventing approve-one-dispatch-another attacks (`bundle.rs:119–148`; module contract at `bundle.rs:14–16`).
3. **Surface/profile consistency** is enforced at the enforcement layer during approval; the bundle preserves the approved surface for audit.
4. **Scope** for strict surfaces must come from `LoadedScope` (not raw `Scope`); manual surfaces use `default_empty` fallback (`executor.rs:97–105`).
5. **No hardcoded permissive defaults** — the executor uses the actual session surface and scope from `RuntimeExecutionContext`.

## Invariants

1. **`RuntimeSurface::Unknown` is never executable** — always errors (`surface.rs:65`).
2. **Manual surfaces retain operator-directed semantics** (even daemon-backed) — `CliManual` and `TuiManual` use `approve_manual()` (`manual.rs:60–65`).
3. **Automated surfaces never honor manual overrides** — rejected with `ManualOverrideRejected` before enforcement evaluation (`manual.rs:66–71`).
4. **Any new `RuntimeSurface` variant must update conversion tests** — `surface.rs` unit tests plus the `runtime_contract_closure` integration test (both fail until mapped).
5. **`dispatch_approved_runtime_request` validates both operation ID and target** — mismatches are rejected with explicit errors (`bundle.rs:127–148`).
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

- **Surface conversion** (`surface.rs:89–221`): Tests all 9 known mappings, rejects `Unknown`, verifies `honors_manual_override()` for permissive vs strict surfaces.
- **Descriptor resolution** (`descriptor.rs:49–488`): 34 tests covering the per-kind descriptors, packet traceroute/send sharing the `packet` family, `requires_explicit_scope` for agent-exposable ops, and optional-target scope checking. All 30 kinds map exhaustively, so there is no longer an "unsupported kind errors" case — `RuntimeBridgeError::UnsupportedTaskKind` is now unreachable from `resolve_operation_and_target()`.
- **Preflight & approval** (`manual.rs:127–479`): Tests preflight/approve paths for CLI/TUI manual, strict, MCP, REST, gRPC, CI, SecurityAgent surfaces; override rejection; daemon-backed manual surfaces remain manual.
- **Bundle & dispatch** (`bundle.rs:158–374`): Tests bundle capture, strict surface rejection, operation mismatch detection, target mismatch detection, surface preservation.
- **Executor** (`executor.rs:228–300`): Tests envelope conversion for port scan, error, and load test variants (via the single engine-owned mapping).
- **Application boundary** (`crates/eggsec/tests/canonical_dispatch_ownership.rs`, mandatory path): per-family normalization/identity/binding/route equivalence plus daemon-bundle cancel race.
- **Runtime-contract closure** (`crates/eggsec/tests/runtime_contract_closure.rs`, guard 97): surface round-trips, wire-identity agreement across `TaskKind` variants, wire JSON stability, no-target family failures, stable envelope kinds, embedded/daemon seam equivalence, approval-binding regression.

> **Coverage gap closed 2026-10-06 (was flagged the same day as a finding).** `Resume`
> (`TaskKind::Resume`) is handled on the conversion path
> (`operation_request.rs:295` `resume_from_runtime`) but had **no** test
> coverage in `runtime_bridge`, and the closure test's
> `task_kind_variant_count_is_pinned` pinned **29** against a helper that
> omitted `Resume` (`crates/eggsec/tests/runtime_contract_closure.rs:238–244`),
> so the claimed "all 29 `TaskKind` variants" wire-identity coverage was really
> **29 of 30** while the assertion still passed.
>
> Fixed in source: `TaskKind::Resume` is now in `all_task_kinds()`, the pin is
> **30**, and two new assertions were added — a `TASK_KIND_VARIANT_NAMES` list
> cross-checked against the same pin, and `enumerated_task_kind_wire_tags_match_the_enum`,
> which zips that list against the helper's serde tags so the name list cannot
> drift. `Resume` now flows through all five helper-iterating tests (adapters,
> canonical conversion, metadata resolution, wire JSON, embedded/daemon seam).
> Verified: `runtime_contract_closure` 18 passed / 0 failed.
>
> Note the real tripwire for a *new* variant is the compiler, not the count:
> `capability_name`, `operation_id`, `canonical_target` and `route_for_command_id`
> in `eggsec-runtime/src/request.rs`, plus the capability, session, TUI and
> ui-model matches, are all exhaustive with no wildcard arm. The counts catch
> accidental removals and a helper that stops covering the enum.

## See Also

- [runtime.md](runtime.md) — `eggsec-runtime` crate (task lifecycle, `Runtime`, `RuntimeTaskExecutor` trait)
- [daemon.md](daemon.md) — Daemon host that uses the bridge
- [dispatch.md](dispatch.md) — The dispatch layer this bridge feeds into
- [config.md](config.md) — `EnforcementContext`, `ExecutionPolicy`, `OperationMetadata`
- [overview.md](overview.md) — System-wide architecture, enforcement model
- [../docs/ARCHITECTURE.md](../docs/ARCHITECTURE.md) — Section 4.8 (Daemon / Runtime (`runtime_bridge`))

*Last verified against source: 2026-09-11 (Phase 3 closure); cites re-verified 2026-09-22 (systematic review); file sizes and `ApprovedExecution` bundle shape re-verified 2026-09-25; full re-verification 2026-10-06 (1,900 lines; `TaskKind` 30 variants incl. `Resume`; dispatch path corrected to `execute_approved_execution`)*
