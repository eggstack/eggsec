# NSE Milestone 3 Phase 03: Filesystem and Process Capability Wrappers

> Status: Executed. Retained for handoff and audit continuity. See `plans/nse-milestone-3-corrective-pass.md` for follow-up corrective work (profile propagation fix, AgentSafe filesystem-read semantics, and additional guards).

## Purpose

Migrate filesystem and process-related NSE helper operations behind the Milestone 3 capability context.

Filesystem and process helpers are the highest-risk helper classes because they can read local data, mutate the host, or execute external commands while Lua execution hooks cannot interrupt native Rust work.

## Background

Milestone 1 secured script/module loading through `ScriptResolver`, but some library helpers still perform filesystem/process work directly. Milestone 3 Phase 02 introduces central capability decisions. This phase applies that context to filesystem and process operations.

## Non-Goals

Do not remove legitimate manual CLI/TUI functionality.

Do not enforce agent-grade restrictions on manual-permissive mode.

Do not migrate network/DNS helpers in this phase.

Do not redesign `ScriptResolver`.

## Target State

By the end of this phase:

- Filesystem reads/writes/deletes/metadata calls used by NSE helpers go through wrappers or are explicitly documented as safe/pure exceptions.
- Process execution helpers go through wrappers and are denied for agent/CI profiles by default.
- Resource counters account for filesystem operations and bytes.
- Capability denials appear in structured reports.
- Architecture guards catch new direct filesystem/process helper calls.

## Workstream 1: Filesystem Wrapper API

### Proposed API

Add helpers such as:

```rust
pub fn nse_fs_read_to_string(ctx: &NseCapabilityContext, path: &Path) -> LuaResult<String>;
pub fn nse_fs_read(ctx: &NseCapabilityContext, path: &Path) -> LuaResult<Vec<u8>>;
pub fn nse_fs_write(ctx: &NseCapabilityContext, path: &Path, bytes: &[u8]) -> LuaResult<()>;
pub fn nse_fs_metadata(ctx: &NseCapabilityContext, path: &Path) -> LuaResult<MetadataSummary>;
pub fn nse_fs_exists(ctx: &NseCapabilityContext, path: &Path) -> LuaResult<bool>;
```

Keep wrappers synchronous unless the surrounding code is async.

### Required Behavior

- Check cancellation before and after operation.
- Enforce profile policy.
- Enforce sandbox roots where configured.
- Increment filesystem operation counters.
- Track bytes read/written where available.
- Record capability events/warnings.
- Return stable Lua errors on denial.

### Acceptance Criteria

- Wrappers compile and are unit-tested.
- Denied reads/writes return clear errors.
- Manual-permissive path remains usable.

## Workstream 2: Process Wrapper API

### Proposed API

```rust
pub fn nse_process_exec(
    ctx: &NseCapabilityContext,
    command: &str,
    args: &[String],
    timeout: Option<Duration>,
) -> LuaResult<ProcessOutputSummary>;
```

### Required Behavior

- AgentSafe and CiSafe deny by default.
- ManualPermissive may allow with event/reporting.
- ManualStrict should allow only explicit configured commands if that policy exists; otherwise deny or warn.
- Apply timeout when possible.
- Check cancellation before spawn and after wait.
- Record process event.
- Avoid shell string execution where possible. Prefer direct command + args.

### Acceptance Criteria

- Process execution cannot bypass capability policy.
- Tests cover manual allowed/agent denied/CI denied paths.

## Workstream 3: Migrate Filesystem Libraries

### Likely Files

Inspect and migrate direct filesystem operations in:

- `crates/eggsec-nse/src/libraries/io.rs`
- `crates/eggsec-nse/src/libraries/lfs.rs`
- `crates/eggsec-nse/src/libraries/os.rs`
- any `datafiles`, `target`, or helper modules that read local files.

### Steps

1. Replace direct `std::fs` calls with wrapper calls.
2. Preserve current manual behavior where possible.
3. Add profile-aware denial for automated surfaces.
4. Ensure file paths in errors/reports are redacted or summarized for automated reports if needed.
5. Update tests for each migrated helper.

### Acceptance Criteria

- High-risk direct filesystem calls are removed from migrated helper modules.
- Automated profiles deny unscoped filesystem operations.
- Manual behavior remains compatible.

## Workstream 4: Migrate Process Libraries

### Likely Files

Inspect:

- `os` library helpers;
- `io.popen` equivalents;
- any module using `std::process::Command`.

### Steps

1. Route process execution through `nse_process_exec()`.
2. Deny by default for AgentSafe and CiSafe.
3. Preserve manual path with explicit event logging.
4. Ensure command allowlists, if present, are enforced by the wrapper.

### Acceptance Criteria

- No direct process execution remains in NSE helpers outside wrapper implementation and tests.
- Denials are visible in structured reports.

## Workstream 5: Tests

Required tests:

- manual filesystem read allowed within expected policy;
- agent filesystem read denied when unscoped;
- CI filesystem mutation denied;
- manual process execution either allowed or explicitly reported according to policy;
- agent process execution denied;
- cancellation before filesystem/process call denies;
- filesystem bytes counters update on read/write;
- capability events appear in reports.

## Workstream 6: Guards and Docs

### Guards

Add or tighten architecture guards for:

- `std::fs::read_to_string`
- `std::fs::read`
- `std::fs::write`
- `std::fs::remove_file`
- `std::fs::rename`
- `std::process::Command`

Allow only wrapper modules, tests, and explicitly documented safe code paths.

### Docs

Update `architecture/nse_integration.md` and `architecture/nse_capability_inventory.md` with migrated helper status.

## Verification

Run:

```bash
cargo check -p eggsec-nse --features nse
cargo test -p eggsec-nse --features nse filesystem
cargo test -p eggsec-nse --features nse process
cargo test -p eggsec-nse --features nse,sandbox
bash scripts/check-architecture-guards.sh
```

## Final Acceptance Criteria

Phase 03 is complete when:

- Filesystem/process helper wrappers exist.
- High-risk filesystem/process helper calls are migrated or explicitly documented as deferred.
- Automated profiles deny dangerous unscoped operations.
- Manual behavior remains usable.
- Resource counters and reports reflect helper-side filesystem/process activity.
- Guards prevent new direct bypasses.
