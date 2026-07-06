# NSE Milestone 3 Closure and Verification Pass

## Purpose

Close Milestone 3 after the profile-propagation corrective pass by adding end-to-end verification, recording the final gate, and tightening the boundary for Milestone 4.

The corrective pass fixed the critical issue where `run_cli_with_profile()` could construct executors through `with_policy(...)`, causing `AgentSafe`/`CiSafe` profile decisions to silently degrade to `ManualPermissive` capability semantics. This closure pass should verify that fix through real execution/report paths, not just constructor-level tests.

## Current State

The repo is close to Milestone 3 closure:

- `run_cli_with_profile()` now constructs the executor with `NseExecutor::with_profile(&execution_profile)`.
- `ExecutorCore::with_full_policy(...)` exists and accepts `profile_kind` plus `network_policy`.
- `with_policy(...)` is documented as manual-only capability semantics.
- `AgentSafe` filesystem reads are scoped: unscoped reads are denied unless the path is under sandbox `allowed_dir` or an explicit allowed root.
- `NseRunReport` includes `capability_events`.
- Architecture guards reject `run_cli_with_profile()` using `with_policy(...)` and scan automated surfaces for manual-only constructors.
- `plans/nse-milestone-3-phase-03-filesystem-process-wrappers.md` has been restored.

Remaining closure gaps:

1. Tests currently verify constructor/profile propagation, but not a full `run_cli_with_profile()` execution producing JSON/report capability events.
2. The Milestone 3 corrective verification gate is not recorded as a durable table in architecture docs.
3. Some direct filesystem/network helper bypass checks remain informational rather than failures because protocol/data helper migration is incomplete.
4. The Milestone 3 closure note should clearly distinguish fully migrated, partially migrated, and deferred helper classes.
5. Milestone 4 should start from a clean boundary and focus on corpus/fidelity/report UX, not reopening capability context semantics.

## Non-Goals

Do not redesign Milestone 1 loader/profile semantics.

Do not redesign Milestone 2 report/library truthfulness.

Do not migrate new protocol libraries unless required for verification.

Do not tighten all filesystem/network guards to failure if the implementation still documents deferred protocol/data helpers.

Do not claim full Nmap parity.

## Workstream 1: End-to-End Profile/Report Tests

### Goal

Add tests that exercise the real execution/report path, or the closest internal helper if direct stdout capture is too brittle.

### Preferred Tests

1. **AgentSafe process denial report**
   - Run a small script or helper that attempts a process-exec path under `AgentSafe`.
   - Assert the operation is denied through `NseCapabilityContext`.
   - Assert `NseRunReport.capability_events` includes `process_exec` with `allowed = false`.

2. **AgentSafe unscoped filesystem-read denial report**
   - Script/helper attempts an unscoped read.
   - Assert capability denial appears in report events.
   - Assert compatibility status degrades to `Partial` or equivalent.

3. **AgentSafe scoped filesystem-read allow report**
   - Configure sandbox `allowed_dir` to a temp fixture directory.
   - Assert read succeeds and event is allowed.

4. **CiSafe network/DNS denial report**
   - Attempt TCP/DNS helper under `CiSafe` without public internet dependency.
   - Assert denial before external network operation.

5. **ManualPermissive process warning report**
   - Use a non-executing or check-only path if command execution should not occur in CI.
   - Assert manual profile records a warning/allow decision without downgrading automated profiles.

### Implementation Notes

If stdout capture makes direct `run_cli_with_profile()` testing awkward, factor the blocking execution/report construction into an internal testable helper such as:

```rust
#[cfg(feature = "nse")]
fn run_cli_profile_inner_for_report(
    config: &NseConfig,
    profile: ResolvedNseExecutionProfile,
) -> anyhow::Result<NseRunReport>
```

This helper should be private or `pub(crate)` and used by `run_cli_with_profile()` so tests cover the same path.

### Acceptance Criteria

- At least one test fails if `run_cli_with_profile()` reverts to `with_policy(...)`.
- At least one test verifies `capability_events` in a real `NseRunReport`.
- Tests require no public internet and no real external command execution unless explicitly manual-only and skipped in CI.

## Workstream 2: Verification Record

### Goal

Record the final Milestone 3 closure gate in repo docs.

### Commands

Run and record:

```bash
cargo check -p eggsec-nse --features nse
cargo test -p eggsec-nse --features nse capability
cargo test -p eggsec-nse --features nse profile_propagation
cargo test -p eggsec-nse --features nse report
cargo test -p eggsec-nse --features nse
cargo test -p eggsec-nse --features nse,sandbox
cargo check -p eggsec --features nse
cargo test -p eggsec --features nse --test nse_tests
bash scripts/check-architecture-guards.sh
cargo fmt --all --check
cargo clippy --lib -p eggsec-nse --features nse
cargo clippy --lib -p eggsec --features nse
```

If any command is unavailable or has a known pre-existing failure, document it precisely.

### Acceptance Criteria

- `architecture/nse_integration.md` has a `Milestone 3 Final Verification` table.
- Verification distinguishes pass, known pre-existing failure, and newly introduced failure.
- The closure note does not rely on GitHub status checks, which may be unavailable through connectors.

## Workstream 3: Guard Closure

### Goal

Make the guard state explicit and avoid overclaiming wrapper migration.

### Steps

1. Confirm Check 35 fails if `run_cli_with_profile()` uses `NseExecutor::with_policy(...)`.
2. Confirm Check 36 scans automated surfaces for manual-only constructors.
3. Keep direct filesystem/network checks informational if deferred helper classes still exist.
4. Add comments listing when those informational checks can become failures.
5. Add a guard or doc note that plan files should be retained/archived, not deleted, if feasible.

### Acceptance Criteria

- Guard output clearly separates closed/failing checks from informational migration checks.
- Deferred filesystem/network bypasses are named in docs.
- Future agents know not to treat info-only guards as closure proof.

## Workstream 4: Documentation Reconciliation

### Goal

Make docs internally consistent after the corrective pass.

### Files

- `architecture/nse_integration.md`
- `architecture/nse_capability_inventory.md`
- `.opencode/skills/eggsec-nse/SKILL.md`
- `crates/eggsec-nse/AGENTS.override.md`
- root `AGENTS.md`

### Required Wording

- Milestone 3 capability context is closed for the core wrapper architecture and migrated high-value helpers.
- Protocol-specific internal helper migration is partial and deferred.
- `AgentSafe` filesystem reads are scoped-only.
- `with_policy(...)` is manual-only; automated surfaces use `with_profile(...)` or `with_full_policy(...)`.
- `capability_events` are helper-side enforcement events, not loader-policy diagnostics.
- Milestone 4 begins with compatibility/fidelity/report UX, not capability-context redesign.

### Acceptance Criteria

- Docs do not claim complete helper migration where guards remain informational.
- The Milestone 3 closure note states both coverage and deferred gaps.

## Workstream 5: Milestone 4 Boundary

### Goal

Prepare the handoff boundary for Milestone 4.

### Milestone 4 Should Focus On

- broader local compatibility corpus;
- representative upstream NSE subset fixtures;
- service/port/host context fidelity;
- structured evidence and finding reports;
- CLI/TUI report UX;
- compatibility matrix publishing;
- deferred protocol-specific helper tracking.

### Milestone 4 Should Not Reopen

- loader/profile resolver semantics;
- library-report truthfulness;
- core capability-context construction;
- manual vs automated trust-boundary model.

## Final Acceptance Criteria

This closure pass is complete when:

- Real execution/report tests cover capability events under at least one automated profile.
- The Milestone 3 final verification table is recorded.
- Guards are aligned with the actual migration state.
- Docs clearly distinguish complete, partial, and deferred Milestone 3 work.
- Milestone 4 has a clean documented starting boundary.
