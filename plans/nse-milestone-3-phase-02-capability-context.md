# NSE Milestone 3 Phase 02: Capability Context and Wrapper API

## Purpose

Introduce a central capability context and wrapper API for NSE helper-side enforcement. Later phases will migrate filesystem, process, network, DNS, time, randomness, crypto, and compression helpers onto this API.

The goal is to avoid ad hoc policy checks scattered across every library file.

## Background

Lua execution limits do not automatically enforce profile policy inside Rust helper calls. A Lua script can enter a Rust helper that performs blocking filesystem, network, DNS, process, crypto, compression, time, or randomness work. That helper must check policy, limits, cancellation, and accounting directly.

Milestone 1 closed script and module loading policy through `ScriptResolver`. Milestone 2 made registry/report truthfulness explicit. This phase creates the shared enforcement surface that Milestone 3 migrations will use.

## Non-Goals

Do not migrate every library helper in this phase.

Do not change manual/agent profile semantics.

Do not replace `ScriptResolver`.

Do not add heavy async architecture unless necessary.

Do not claim full Nmap parity.

## Target State

By the end of this phase:

- A central `NseCapabilityContext` or equivalent exists.
- Helpers can ask whether a capability operation is allowed.
- Capability checks update counters, diagnostics, and report state.
- Cancellation checks are available before and after helper operations.
- Wrappers return consistent errors that Lua helpers can surface cleanly.
- At least one low-risk pilot helper uses the capability context.

## Proposed Module Layout

Add:

```text
crates/eggsec-nse/src/capabilities.rs
```

or a submodule tree if the initial file would be too large:

```text
crates/eggsec-nse/src/capabilities/mod.rs
crates/eggsec-nse/src/capabilities/filesystem.rs
crates/eggsec-nse/src/capabilities/network.rs
crates/eggsec-nse/src/capabilities/process.rs
crates/eggsec-nse/src/capabilities/time.rs
```

Start with one module if simpler. Split only when migration pressure justifies it.

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

Adjust names to match existing crate style. Keep the first version deliberately small and easy to serialize.

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
   - network policy;
   - script/module policy for filesystem/module-related helper decisions.
5. Keep the context cloneable if library closures need to capture it.
6. Export the context types under the `nse` feature only.

### Acceptance Criteria

- `ExecutorCore` can expose a capability context or wrapper handle.
- Context construction does not change current behavior yet.
- The context can be captured by Lua library closures without lifetime friction.

## Workstream 2: Decision Engine

### Steps

1. Implement a conservative `check_capability(request)` method.
2. Map profile kinds to default decisions:
   - `ManualPermissive`: allow, record event.
   - `ManualStrict`: enforce roots/scopes where available; otherwise warn or deny based on capability class.
   - `AgentSafe`: deny dangerous/default-unscoped operations; allow only explicitly scoped network/local deterministic operations.
   - `CiSafe`: deny external network/process and allow only deterministic local fixture operations.
   - `CompatibilityLab`: allow controlled local compatibility operations, record warnings.
3. Return structured denial reasons.
4. Do not panic on unknown capability classes. Deny for automated profiles, warn/allow for manual profiles if appropriate.
5. Keep denial text stable enough for tests and report assertions.

### Acceptance Criteria

- Unit tests cover decisions for each profile kind and high-level capability class.
- Denial reasons are deterministic.
- ManualPermissive is not silently constrained to automated defaults.

## Workstream 3: Cancellation and Limit Helpers

### Steps

1. Add `check_cancelled(operation)` helper.
2. Add `before_blocking_operation(request)` and `after_blocking_operation(request, result_bytes)` helpers.
3. Update relevant counters:
   - network operations/bytes;
   - filesystem operations/bytes;
   - process operation count if added;
   - crypto/compression counters if added later.
4. Ensure checks can be called from synchronous Rust helper functions.
5. Where timeout wrappers are not yet implemented, at least check cancellation before and after the call.
6. Ensure limit violations map to existing `NseLimitViolation` where possible.

### Acceptance Criteria

- Helpers can perform consistent pre/post checks.
- Tests prove cancellation before helper execution denies cleanly.
- Counter updates are centralized rather than ad hoc.

## Workstream 4: Report Integration

### Steps

1. Add capability events to `NseRunReport` or to an existing diagnostics/warnings field.
2. If adding a full field is too broad, include capability denials/warnings in `warnings` and `compatibility.unsupported_features` initially.
3. Provide conversion from `NseCapabilityEvent` to report summaries.
4. Ensure denied helper operations affect compatibility status.
5. Ensure report output distinguishes:
   - policy denial;
   - limit exceeded;
   - cancellation;
   - helper runtime error.

### Acceptance Criteria

- A denied helper operation appears in structured reports.
- Report serialization covers capability events or stable warning summaries.
- Reports do not conflate helper denial with script/module loader denial.

## Workstream 5: Minimal Pilot Wrapper

### Steps

1. Pick one low-risk helper class for a pilot, preferably a time/randomness helper or simple filesystem metadata read.
2. Route it through the capability context.
3. Add tests for:
   - manual allow;
   - agent/CI deny or warning according to policy;
   - cancellation before call;
   - report event/warning emission.
4. Use the pilot to validate wrapper ergonomics before broad migration.

### Acceptance Criteria

- At least one helper path uses the capability context.
- The pilot demonstrates policy decision, accounting, cancellation check, and reporting.
- The implementation pattern is clear enough for Phases 03 through 05.

## Workstream 6: Architecture Guards

Add initial guards that detect new direct high-risk operations outside wrapper modules. Start with warnings if the repo still has many legacy hits, then later phases can tighten migrated classes to failures.

Candidate patterns:

- `std::process::Command`
- `std::fs::read_to_string`
- `std::fs::write`
- `std::fs::remove_file`
- `std::fs::rename`
- `TcpStream::connect`
- `UdpSocket::bind`
- direct DNS resolver calls
- direct environment reads outside explicit manual paths

### Acceptance Criteria

- Guard output points contributors to capability wrappers.
- Existing intentional direct calls are either allowlisted or documented as deferred.

## Workstream 7: Documentation

Update `architecture/nse_integration.md` or `architecture/nse_capability_inventory.md` with:

- capability context purpose;
- capability classes;
- profile decision defaults;
- wrapper migration rules;
- report event semantics;
- deferred helper classes.

Update `.opencode/skills/eggsec-nse/SKILL.md` with a rule: new side-effecting helpers must use the capability context.

## Tests

Required tests:

- context construction from manual profile;
- context construction from agent-safe profile;
- manual allow decision for low-risk capability;
- agent deny decision for unscoped filesystem/process/network;
- CI deny decision for external network/process;
- cancellation before operation;
- capability event recorded;
- report includes capability warning/event if integrated in this phase.

## Verification

Run:

```bash
cargo check -p eggsec-nse --features nse
cargo test -p eggsec-nse --features nse capability
cargo test -p eggsec-nse --features nse
cargo test -p eggsec-nse --features nse,sandbox
bash scripts/check-architecture-guards.sh
```

## Final Acceptance Criteria

Phase 02 is complete when:

- Capability context and decision types exist.
- Executor/library code can access the context.
- Cancellation and accounting helpers exist.
- At least one pilot helper uses the wrapper path.
- Capability denials/warnings can appear in reports or stable warnings.
- Initial architecture guards are in place.
- Phases 03 through 05 can migrate helpers without inventing new local policy systems.
