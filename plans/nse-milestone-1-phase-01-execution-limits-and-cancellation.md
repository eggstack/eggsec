# NSE Milestone 1 Phase 01: Execution Limits and Cancellation

## Purpose

Tighten NSE execution semantics so scripts cannot run indefinitely, consume unbounded resources, or continue executing after the caller receives a timeout.

This is the first Milestone 1 workstream because all later sandbox and policy work depends on having a trustworthy execution boundary.

## Current Problem

The existing timeout path uses a spawned thread and waits on a channel with `recv_timeout`. If the timeout expires, the caller receives an error, but the spawned script thread is not forcibly stopped. This means a timed-out script can continue executing and can potentially continue producing side effects.

That behavior is not production-grade. It is also dangerous for automated surfaces because higher-level orchestration may assume the operation stopped when it did not.

## Target State

The executor should support a single execution limit model that applies consistently across synchronous, asynchronous, CLI, TUI, daemon, and agent/MCP use.

The target semantics are:

- Wall-clock timeout bounds total script execution.
- Lua instruction budget or equivalent cooperative hook bounds CPU-bound Lua loops.
- Output size limit bounds memory growth and reporting payloads.
- Optional network operation count and byte limits bound side-effect volume.
- Optional filesystem operation count and byte limits bound local impact.
- Cancellation is visible to Lua libraries and Rust-side helper functions.
- Timeout/cancellation result includes enough detail to distinguish timeout, instruction budget, output cap, network limit, filesystem limit, and explicit user cancellation.

## Proposed Types

Introduce a central limit type in `eggsec-nse`, naming adjusted to fit existing code style:

```rust
pub struct NseExecutionLimits {
    pub wall_clock_timeout: Option<Duration>,
    pub lua_instruction_budget: Option<u64>,
    pub max_output_bytes: Option<usize>,
    pub max_script_bytes: Option<usize>,
    pub max_required_module_bytes: Option<usize>,
    pub max_network_operations: Option<u64>,
    pub max_network_bytes_read: Option<u64>,
    pub max_network_bytes_written: Option<u64>,
    pub max_filesystem_operations: Option<u64>,
    pub max_filesystem_bytes_read: Option<u64>,
    pub max_lua_memory_bytes: Option<usize>,
}
```

Add explicit result/error variants:

```rust
pub enum NseLimitViolation {
    WallClockTimeout,
    LuaInstructionBudgetExceeded,
    OutputLimitExceeded,
    ScriptSizeLimitExceeded,
    ModuleSizeLimitExceeded,
    NetworkOperationLimitExceeded,
    NetworkByteLimitExceeded,
    FilesystemOperationLimitExceeded,
    FilesystemByteLimitExceeded,
    ExplicitCancellation,
}
```

Add a runtime cancellation handle:

```rust
pub struct NseCancellationToken { /* internal atomic state */ }
```

The exact implementation can use an internal `Arc<AtomicBool>` or an existing cancellation primitive if already available in the workspace.

## Implementation Steps

### Step 1: Add Limit and Cancellation Types

Create a small module such as `limits.rs` or `execution_limits.rs` inside `crates/eggsec-nse/src/`.

Add:

- `NseExecutionLimits`.
- `NseLimitViolation`.
- `NseCancellationToken`.
- `NseExecutionStats` for counters collected during a run.

Initial `NseExecutionStats` should include:

- elapsed time.
- output bytes produced.
- Lua instruction count if available.
- network operations.
- network bytes read/written.
- filesystem operations.
- filesystem bytes read.
- cancellation/limit state.

### Step 2: Add Limits to Executor Construction

Extend `ExecutorCore`, `NseExecutor`, and `AsyncNseExecutor` constructors with limit-aware variants.

Suggested API shape:

```rust
impl NseExecutor {
    pub fn with_policy(
        sandbox: SandboxConfig,
        limits: NseExecutionLimits,
        cancellation: NseCancellationToken,
    ) -> Result<Self>;
}
```

Keep compatibility constructors, but make them call the policy-aware constructor with explicit manual defaults.

Avoid hidden defaults for agent/MCP/daemon call paths.

### Step 3: Replace Misleading Timeout Thread Behavior

Replace or deprecate `run_script_with_timeout`.

Preferred target:

```rust
pub fn run_script_with_limits(&self, script: &str) -> Result<NseRunOutcome>;
```

If preserving `run_script_with_timeout` for API compatibility, change it to delegate to the new limit model and actually cancel execution.

Do not keep an API where timeout returns while a worker thread continues. If a temporary compatibility shim is needed, name it honestly, for example `run_script_wait_timeout_non_canceling`, mark it deprecated, and keep it out of agent/MCP surfaces.

### Step 4: Add Lua Interruption

Use mlua debug hooks or the closest supported mechanism to periodically check:

- cancellation token.
- wall-clock deadline.
- instruction budget.
- memory/output pressure if available.

If the selected Lua backend cannot enforce a hard memory cap, document that limitation and enforce all available output and instruction limits.

The hook should return a structured runtime error that maps to `NseLimitViolation`.

### Step 5: Bound Output Collection

Update `_SCRIPT_OUTPUT` collection and any stdnse output helpers to count serialized bytes before storing or returning values.

The output cap should apply to:

- string output.
- table output rendered into text.
- JSON output if already supported.
- accumulated compatibility diagnostics.

On limit breach, return a partial-output result with a clear `OutputLimitExceeded` warning if preserving partial data is useful; otherwise return a hard error. For agent/MCP surfaces, prefer a hard error unless the profile explicitly allows partial reports.

### Step 6: Make Rust-Side Operations Cancellation-Aware

Any Rust-side NSE helper that performs network or filesystem work should check the cancellation token before starting and after blocking calls return.

For this phase, do not attempt to refactor every protocol helper. At minimum, identify direct side-effecting call sites and create TODO-backed wrappers or adapter points that Phase 2/3 can enforce.

Callers should not start new side effects once cancellation has been requested.

### Step 7: Add Tests

Add focused tests under `crates/eggsec-nse`.

Required tests:

- Infinite Lua loop is interrupted.
- Script that appends output forever is capped.
- Very large script is rejected before execution when max script bytes is set.
- Timeout returns a cancellation/timeout violation.
- After timeout, a side-effecting test hook cannot continue appending to shared state.
- Compatibility constructor still works for manual/default use.
- Agent-safe limits are stricter than manual defaults once Phase 2 types land.

For post-timeout side-effect testing, prefer a deterministic test helper registered only under `#[cfg(test)]`, not real network or filesystem effects.

## API Compatibility Guidance

Avoid breaking the public API more than necessary, but prioritize truthfulness over compatibility.

Acceptable compatibility path:

- Keep old constructors.
- Route old constructors to explicit manual/default limits.
- Keep `run_script` behavior for manual use.
- Replace timeout internals so timeout is real.

Unacceptable compatibility path:

- Keeping non-canceling timeout behavior under the same name.
- Allowing automated surfaces to call compatibility constructors without selecting a profile.

## Documentation Updates

Update crate docs and any CLI help that references timeout behavior.

Document:

- What timeout covers.
- Whether timeout interrupts Lua CPU loops.
- Whether timeout interrupts blocking network calls immediately or after their own I/O timeout.
- Which limits are enforced in each profile.
- Which limits are best-effort because of Lua/runtime constraints.

## Verification Commands

Run at least:

```bash
cargo check -p eggsec-nse --features nse
cargo test -p eggsec-nse --features nse
cargo test -p eggsec-nse --features nse,sandbox
cargo check -p eggsec --features nse
```

If `make test-nse` exists in the checked-out environment, run it before handoff.

## Acceptance Criteria

This phase is complete when:

- Timeout no longer leaves a script running invisibly.
- Infinite-loop scripts are interrupted under configured limits.
- Output growth is bounded.
- Script-size limits are enforced before Lua evaluation.
- Limit violations are represented as structured errors or structured run outcomes.
- Existing manual/default use remains possible.
- Tests cover timeout, cancellation, output caps, and large script rejection.
- Follow-up work can consume the same `NseExecutionLimits` model for sandbox profiles.

## Reviewer Checklist

- Search for old thread/channel timeout behavior and verify it is removed or no longer misleading.
- Verify limit defaults are not accidentally strict for manual compatibility constructors.
- Verify automated callers do not use permissive compatibility constructors after Phase 2 lands.
- Verify tests fail before the implementation and pass after.
- Verify no new raw side-effecting helper bypasses the cancellation token.
