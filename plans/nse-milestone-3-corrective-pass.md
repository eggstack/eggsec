# NSE Milestone 3 Corrective Pass

## Purpose

This corrective pass closes the first major correctness gaps after the initial Milestone 3 implementation.

The implementation landed a real capability context, broad helper wrappers, report integration, docs, tests, and initial guards. However, one critical entry-point bug remains: the main `run_cli_with_profile()` execution path constructs `NseExecutor::with_policy(...)`, while `ExecutorCore::with_policy(...)` currently hardcodes `ManualPermissive` and `AllowAllManual` into the capability context. That means profile-aware helper enforcement can silently run with manual-permissive capability decisions even when the resolved profile is `AgentSafe` or `CiSafe`.

This pass must make profile propagation correct end-to-end before continuing further Milestone 3 migration work.

## Current State Summary

Confirmed progress:

- `crates/eggsec-nse/src/capabilities.rs` defines `NseCapabilityContext`, capability kinds, requests, decisions, events, cancellation checks, pre/post blocking hooks, resource accounting, and profile-specific decisions.
- `crates/eggsec-nse/src/wrappers.rs` defines check-only and executing wrappers for filesystem, process, network, DNS, time, randomness, environment, compression, and decompression operations.
- `ExecutorCore` stores `profile_kind`, `network_policy`, and `capability_context`.
- `NseRunReport` includes `capability_events`.
- CLI JSON collects capability events through `executor.capability_events()` and serializes them through `with_capability_events(...)`.
- Several libraries now use capability checks/wrappers.
- Architecture guards now fail direct process execution outside wrappers and show filesystem/network bypasses as informational.

Remaining issues:

1. `run_cli_with_profile()` constructs `NseExecutor::with_policy(...)` instead of `NseExecutor::with_profile(&resolved_profile)`, so the capability context may be initialized as `ManualPermissive` even for automated profiles.
2. `ExecutorCore::with_policy(...)` hardcodes `profile_kind = ManualPermissive` and `network_policy = AllowAllManual`, so any caller using it cannot express automated capability policy.
3. The public `NseExecutor::with_policy(...)` API accepts sandbox, limits, cancellation, script policy, and module policy, but not profile kind or network policy. That makes it easy for automated surfaces to accidentally get manual capability semantics.
4. AgentSafe filesystem-read behavior is unclear: tests currently allow arbitrary filesystem reads in AgentSafe. This may be acceptable only if scoped/root-limited, but it is not clearly enforced or documented.
5. `plans/nse-milestone-3-phase-03-filesystem-process-wrappers.md` was deleted and should be restored or archived.
6. Architecture guards do not yet detect profile-loss entry points, such as `run_cli_with_profile()` using `with_policy(...)` instead of `with_profile(...)`.
7. Real entry-point integration tests are needed to prove AgentSafe/CiSafe denial through the actual CLI/profile path, not only wrapper unit tests.

## Non-Goals

Do not redesign Milestone 1 loader/profile policy.

Do not redesign Milestone 2 report/library truthfulness semantics.

Do not migrate additional protocol libraries in this pass unless needed to test profile propagation.

Do not attempt full Nmap parity.

Do not remove manual CLI/TUI discretion.

Do not tighten all filesystem/network guard checks to failure in this pass unless a migrated class is clearly complete.

## Workstream 1: Restore Phase 03 Plan History

### Problem

`plans/nse-milestone-3-phase-03-filesystem-process-wrappers.md` was deleted after implementation. This repeats the plan-retention issue seen in earlier milestones.

### Required Outcome

The phase 03 plan must be restored or moved to an explicit completed-plan archive.

### Steps

1. Restore `plans/nse-milestone-3-phase-03-filesystem-process-wrappers.md` from commit history.
2. Add a status header:

```markdown
> Status: Executed / partially executed. Retained for handoff and audit continuity. See `plans/nse-milestone-3-corrective-pass.md` for follow-up corrective work.
```

3. If an archive convention exists, move it there instead of deleting it.
4. Add or update contributor/agent guidance stating that executed plan files should be retained or archived, not removed.

### Acceptance Criteria

- The phase 03 plan is visible in the repo.
- Future reviewers can trace overview → phase plans → corrective pass.

## Workstream 2: Fix Profile Propagation in CLI Execution

### Problem

`run_cli_with_profile()` resolves a profile, but creates the executor with `NseExecutor::with_policy(...)`. That path loses profile kind and network policy because `with_policy(...)` does not accept those fields.

### Required Outcome

`run_cli_with_profile()` must construct executors through a path that preserves the full `ResolvedNseExecutionProfile`, including profile kind and network policy, so capability decisions match the resolved profile.

### Preferred Fix

Replace the current executor construction in `run_cli_with_profile()` with:

```rust
let mut executor = NseExecutor::with_profile(&resolved_profile)
    .map_err(|e| anyhow::anyhow!("Failed to create NSE executor: {}", e))?;
```

Then remove redundant local extraction of sandbox/limits/cancellation if no longer needed.

### Important Detail

The current implementation moves `resolved_profile` into `spawn_blocking`. If switching to `with_profile`, clone a profile specifically for execution:

```rust
let execution_profile = resolved_profile.clone();
let report_profile = resolved_profile.clone();
```

Inside the blocking task, call `NseExecutor::with_profile(&execution_profile)`.

### Acceptance Criteria

- AgentSafe execution through `run_cli_with_profile()` results in AgentSafe capability decisions.
- CiSafe execution through `run_cli_with_profile()` results in CiSafe capability decisions.
- ManualPermissive execution remains manual-permissive.
- No surface accidentally downgrades automated profiles to manual capability behavior.

## Workstream 3: Repair or Deprecate `with_policy(...)` Semantics

### Problem

`NseExecutor::with_policy(...)` and `ExecutorCore::with_policy(...)` are easy to misuse because their names imply explicit policy, but they silently use manual profile kind and allow-all network policy for capability decisions.

### Required Outcome

The API should either accept full capability profile metadata or be clearly marked manual-only / compatibility-only.

### Option A: Add Full Profile Inputs

Add a new constructor with complete capability semantics:

```rust
pub fn with_full_policy(
    sandbox: SandboxConfig,
    limits: NseExecutionLimits,
    cancellation: NseCancellationToken,
    script_policy: NseScriptPolicy,
    module_policy: NseModulePolicy,
    profile_kind: NseExecutionProfileKind,
    network_policy: NseNetworkPolicy,
) -> LuaResult<Self>
```

Then:

- Have `with_profile(...)` call `with_full_policy(...)`.
- Keep `with_policy(...)` as manual-only compatibility wrapper and document it accordingly.
- Update automated surfaces to use `with_profile(...)` or `with_full_policy(...)`.

### Option B: Extend Existing `with_policy(...)`

Change the existing signature to include profile kind and network policy. This is cleaner semantically but may require broader call-site updates.

### Recommended Choice

Use Option A for lower churn and safer compatibility. Rename later if desired.

### Acceptance Criteria

- There is a constructor path that accepts full capability policy.
- `with_profile(...)` uses the full constructor and does not first create a manual capability context and then overwrite it unless that overwrite is provably safe.
- `with_policy(...)` docs explicitly state manual-permissive capability context unless extended.

## Workstream 4: Add Profile-Propagation Integration Tests

### Problem

Wrapper tests prove isolated decisions, but they do not prove real entry points preserve profile semantics.

### Required Tests

Add tests that exercise real executor/profile construction paths:

1. **with_profile AgentSafe process deny**
   - Construct `ResolvedNseExecutionProfile::agent_safe(...)`.
   - Create executor with `NseExecutor::with_profile(...)`.
   - Invoke a helper path or wrapper through executor context that requests `ProcessExec`.
   - Assert denial and capability event profile semantics.

2. **with_profile CiSafe network deny**
   - Construct CiSafe profile.
   - Assert TCP/DNS/network capability decisions deny.

3. **run_cli_with_profile path preserves AgentSafe**
   - Use a test helper if direct stdout capture is difficult.
   - At minimum, factor executor creation in `run_cli_with_profile()` into a small internal helper and test that helper.
   - Assert `executor.capability_context().profile_kind == AgentSafe` or equivalent.

4. **ManualPermissive remains permissive**
   - Construct manual profile.
   - Assert process exec decision is allowed with warning, matching manual behavior.

5. **Regression test for with_policy manual default**
   - If `with_policy(...)` remains manual-only, add a test and doc that it returns manual capability semantics.

### Acceptance Criteria

- Tests fail on the current bug where CLI/profile execution loses `AgentSafe`/`CiSafe` capability semantics.
- Tests pass after profile propagation is fixed.

## Workstream 5: Decide AgentSafe Filesystem Read Semantics

### Problem

Current tests allow `nse_fs_read_to_string(...)` under AgentSafe for arbitrary temp paths. This may be intentional, but the intended policy has been “automated surfaces are scoped and bounded.” Unscoped read access is a sensitive capability.

### Required Decision

Choose one of these models and encode it in docs/tests:

### Option A: AgentSafe denies unscoped filesystem reads by default

- AgentSafe permits filesystem reads only under explicit allowed roots, fixtures, or resolver-approved module/script roots.
- This is stricter and safer for agent/MCP/autonomous use.

### Option B: AgentSafe allows read-only filesystem access but reports it

- AgentSafe permits reads but denies writes/process/network unless scoped.
- This is more compatible but weaker for automated surfaces.
- Must clearly document that AgentSafe is not read-confined unless sandbox roots are configured.

### Recommended Choice

Use Option A unless there is a strong compatibility reason not to. Manual CLI/TUI already covers discretionary local reads.

### Implementation Steps for Option A

1. Update `check_agent_safe(...)` for `FilesystemRead`.
2. If `sandbox.enabled` and `allowed_dir` exists, allow reads only under allowed dir.
3. If no allowed root is configured, deny or require explicit fixture mode.
4. Update tests currently expecting AgentSafe filesystem read allow.
5. Add tests for allowed rooted read and denied unscoped read.

### Acceptance Criteria

- AgentSafe filesystem-read behavior is explicit and tested.
- Docs match behavior.
- ManualPermissive remains unchanged.

## Workstream 6: Strengthen Architecture Guards

### Required Guards

Add or update guards in `scripts/check-architecture-guards.sh` for:

1. `run_cli_with_profile()` must use `NseExecutor::with_profile(...)` or a helper that accepts `ResolvedNseExecutionProfile`.
2. Automated surfaces must not call manual-only `with_policy(...)` unless they pass full profile metadata through a new constructor.
3. `ExecutorCore::with_policy(...)` hardcoded `ManualPermissive` must be clearly allowlisted as manual-only, or removed.
4. Direct all-profile capability context construction with `ManualPermissive` outside manual constructors should fail.
5. Deleted plan files should not be silently removed if a simple plan-retention guard exists. If this is too much for shell guards, document it instead.

### Acceptance Criteria

- The current `run_cli_with_profile()` bug would fail a guard.
- Manual-only constructors remain allowed for manual CLI/TUI surfaces.
- Guard messages identify the correct constructor to use.

## Workstream 7: Report and Docs Corrections

### Required Updates

Update:

- `architecture/nse_integration.md`
- `architecture/nse_capability_inventory.md`
- `.opencode/skills/eggsec-nse/SKILL.md`
- `crates/eggsec-nse/AGENTS.override.md`
- root `AGENTS.md` if needed

### Required Wording

- Capability decisions must use the resolved execution profile.
- `run_cli_with_profile()` is the canonical profile-aware CLI path.
- `with_policy(...)` is manual-only unless extended to include profile kind and network policy.
- AgentSafe filesystem read behavior must be stated precisely.
- Capability events in reports reflect helper-side policy decisions and must not be interpreted as loader-policy decisions.

### Acceptance Criteria

- Docs do not imply automated profiles are enforced unless the execution path preserves profile semantics.
- Docs match actual AgentSafe filesystem read policy.

## Workstream 8: Verification Record

### Required Commands

Run and record:

```bash
cargo check -p eggsec-nse --features nse
cargo test -p eggsec-nse --features nse capability
cargo test -p eggsec-nse --features nse
cargo test -p eggsec-nse --features nse,sandbox
cargo check -p eggsec --features nse
cargo test -p eggsec --features nse --test nse_tests
bash scripts/check-architecture-guards.sh
cargo fmt --all --check
cargo clippy --lib -p eggsec-nse --features nse
cargo clippy --lib -p eggsec --features nse
```

If any command is unavailable, record the reason and closest equivalent.

### Acceptance Criteria

- Verification is recorded outside commit messages.
- Failures are not hidden.

## Final Acceptance Criteria

This corrective pass is complete when:

- `run_cli_with_profile()` preserves full resolved profile semantics in capability decisions.
- Automated profiles no longer degrade to manual-permissive capability behavior.
- Constructor APIs make manual-only versus full-profile semantics explicit.
- AgentSafe filesystem-read policy is decided, documented, and tested.
- The deleted phase 03 plan is restored or archived.
- Guards catch profile propagation regressions.
- Integration tests prove AgentSafe/CiSafe enforcement through real entry-point construction.
- Verification results are recorded.

## Handoff Notes

Keep this pass narrow. The capability wrapper architecture is already present. The urgent issue is not adding more wrappers; it is making sure existing wrappers receive the correct profile context from every entry point. Do not continue migrating deeper protocol libraries until this profile propagation bug is closed.
