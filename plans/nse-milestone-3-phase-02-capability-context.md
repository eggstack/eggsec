# NSE Milestone 3 Phase 02: Capability Context and Wrapper API

## Purpose

Introduce a central capability context and wrapper API for NSE helper-side enforcement. Later phases will migrate filesystem, process, network, DNS, time, randomness, crypto, and compression helpers onto this API.

The goal is to avoid ad hoc policy checks scattered across every library file.

## Background

Lua execution limits do not automatically enforce profile policy inside Rust helper calls. A Lua script can enter a Rust helper that performs blocking filesystem/network/process work. That helper must check policy, limits, cancellation, and accounting directly.

## Non-Goals

Do not migrate every library helper in this phase.

Do not change manual/agent profile semantics.

Do not replace `ScriptResolver`.

Do not add heavy async architecture unless necessary.

## Target State

By the end of this phase:

- A central `NseCapabilityContext` or equivalent exists.
- Helpers can ask whether a capability operation is allowed.
- Capability checks update counters, diagnostics, and report state.
- Cancellation checks are available before and after helper operations.
- Wrappers return consistent errors that Lua helpers can surface cleanly.

## Proposed Module Layout

Add:

```text
crates/eggsec-nse/src/capabilities.rs
```

or a submodule tree:

```text
crates/eggsec-nse/src/capabilities/mod.rs
crates/eggsec-nse/src/capabilities/filesystem.rs
crates/eggsec-nse/src/capabilities/network.rs
...
```

Start with one module if simpler.

## Proposed Types

```rust
pub struct NseCapabilityContext {
    pub profile_kind: NseExecutionProfileKind,
    pub network_policy: NseNetworkPolicy,
    pub script_policy: NseScriptPolicy,
    pub module_policy: NseModulePolicy,
    pub sandbox: SandboxConfig,
    pub limits: NseExecutionLimits,
    pub cancellation: NseCancellationToken,
    pub counters: Arc<NseResourceCounters>,
    pub events: Arc<Mutex<Vec<NseCapabilityEvent>>>,
}

pub enum NseCapabilityKind {
    FilesystemRead,
    FilesystemWrite,
    ProcessExec,
    NetworkTcp,
    NetworkUdp,
    DnsResolution,
    TimeClock,
    Randomness,
    Crypto,
    Compression,
    Environment,
}

pub struct NseCapabilityRequest {
    pub kind: NseCapabilityKind,
    pub target: Option<String>,
    pub bytes_hint: Option<u64>,
    pub operation: &'static str,
}

pub enum NseCapabilityDecision {
    Allow,
    Deny { reason: String },
    AllowWithWarning { warning: String },
}

pub struct NseCapabilityEvent {
    pub kind: NseCapabilityKind,
    pub operation: String,
    pub target: Option<String>,
    pub allowed: bool,
    pub reason: Option<String>,
    pub bytes: Option<u64>,
}
```

Adjust names to match existing style.

## Workstream 1: Context Construction

### Steps

1. Add capability context types.
2. Add constructors from `ResolvedNseExecutionProfile` and from existing executor core fields.
3. Store context inside `ExecutorCore` or make it cheaply derivable from existing fields.
4. Ensure it has access to:
   - profile kind;
   - sandbox config;
   - limits;
   - cancellation token;
   - resource counters;
   - network policy.
5. Keep the context cloneable if library closures need to capture it.

### Acceptance Criteria

- `ExecutorCore` can expose a capability context or wrapper handle.
- Context construction does not change current behavior yet.

## Workstream 2: Decision Engine

### Steps

1. Implement a conservative `check_capability(request)` method.
2. Map profile kinds to default decisions:
   - `ManualPermissive`: allow, record event.
   - `ManualStrict`: enforce roots/scopes where available, otherwise warn/deny based on capability class.
   - `AgentSafe`: deny dangerous/default-unscoped operations; allow only explicitly scoped network/local deterministic operations.
   - `CiSafe`: deny external network/process; allow deterministic local fixture operations.
   - `CompatibilityLab`: allow controlled local compatibility operations, record warnings.
3. Return structured denial reasons.
4. Do not panic on unknown capability classes. Deny for automated profiles, warn/allow for manual if appropriate.

### Acceptance Criteria

- Unit tests cover decisions for each profile kind and capability class.
- Denial reasons are stable enough for tests and reports.

## Workstream 3: Cancellation and Limit Helpers

### Steps

1. Add `check_cancelled(operation)` helper.
2. Add `before_blocking_operation(request)` and `after_blocking_operation(request, result_bytes)` helpers.
3. Update relevant counters:
   - network operations/bytes;
   - filesystem operations/bytes;
   - future process/crypto/compression counters if added.
4. Ensure checks can be called from synchronous Rust helper functions.
5. Where timeout wrappers are not yet implemented, at least check cancellation before and after the call.

### Acceptance Criteria

- Helpers can perform consistent pre/post checks.
- Tests prove cancellation before helper execution denies cleanly.

## Workstream 4: Report Integration

### Steps

1. Add capability events to `NseRunReport` or to an existing diagnostics/warnings field.
2. If adding a full field is too broad, include capability denials/warnings in `warnings` and `compatibility.unsupported_features` initially.
3. Provide conversion from `NseCapabilityEvent` to report summaries.
4. Ensure denied helper operations affect compatibility status.

### Acceptance Criteria

- A denied helper operation appears in structured reports.
- Report serialization covers capability events or warnings.

## Workstream 5: Minimal Pilot Wrapper

### Steps

1. Pick one low-risk helper class for a pilot, preferably time/randomness or a simple filesystem metadata read.
2. Route it through the capability context.
3. Add tests for manual allow and agent/CI denial or warning as appropriate.
4. Use the pilot to validate wrapper API ergonomics before broad migration.

### Acceptance Criteria

- At least one helper path uses the capability context.
- The pilot demonstrates policy decision, accounting, cancellation check, and reporting.

## Architecture Guards

Add guards that detect new direct high-risk operations outside wrapper modules:

- `std::process::Command`
- `std::fs::read_to_string`, `std::fs::write`, `remove_file`, `rename`
- `TcpStream`, `UdpSocket`
- direct DNS resolver calls

Start as warnings if the repo still has many existing hits, then tighten to failures as phases migrate.

## Verification

Run:

```bash
cargo check -p eggsec-nse --features nse
cargo test -p eggsec-nse --features nse capability
cargo test -p eggsec-nse --features nse
bash scripts/check-architecture-guards.sh
```

## Final Acceptance Criteria

Phase 02 is complete when:

- Capability context and decision types exist.
- Executor/library code can access the context.
- Cancellation and accounting helpers exist.
- At least one pilot helper uses the wrapper path.
- Capability denials/warnings can appear in reports.
- Initial architecture guards are in place.
