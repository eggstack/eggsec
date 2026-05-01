# Slapper Agent Harness Improvement Plan

**Date**: 2026-04-30
**Status**: Ready for implementation
**Priority**: High

This file is the active handoff plan for hardening the autonomous agent harness. Completed TUI phase history was intentionally pruned to save context.

The goal is not to add new scan capabilities. The goal is to make the existing harness enforce policy, schedule work exactly once, persist state correctly, honor cancellation, reload configuration, and expose tests that cover the real agent entry points.

---

## Current Verified Problems

Verify current code before editing. These observations came from `crates/slapper/src/agent/*` on 2026-04-30.

| Area | Problem | Primary Files |
|------|---------|---------------|
| Feature build | `rest-api` fails to compile at `agent/mod.rs:470` because `trigger_event` captures `&mut self` in an async closure. | `crates/slapper/src/agent/mod.rs` |
| Constraints | `ConstraintChecker` exists but is not called by `Agent::execute_scan`, `trigger_scan`, or scheduled scans. | `agent/mod.rs`, `agent/constraints/checker.rs` |
| Scheduling | `process_scheduled_scans` checks only whether cron matches now. It updates `last_scan` but never uses it, so short poll intervals can run the same schedule repeatedly. | `agent/mod.rs`, `agent/portfolio.rs` |
| Cancellation | `execute_scan_with_depth` accepts a Tokio `CancellationToken` but creates an unrelated tool cancellation token. | `agent/mod.rs`, `tool/request.rs` |
| Hot reload | `ConfigWatcher` logs changes, but `SlapperConfigReloader::reload` does not mutate live agent state. Watcher setup errors are swallowed with `.ok()`. | `agent/config_watcher.rs`, `agent/mod.rs` |
| Target typing/scope | Agent requests always use `TargetType::Url` and `scope: None`, ignoring `TargetConfig.target_type` and configured scope. | `agent/mod.rs`, `tool/request.rs`, `config/*` |
| Alert routing | `TargetConfig.alert_channels` exists, but the agent creates an empty `AlertRouter` and does not route per target. | `agent/mod.rs`, `agent/portfolio.rs`, `agent/alerts/*` |
| Alert contents | Critical alerts include all finding IDs instead of only critical finding IDs. | `agent/mod.rs` |
| Failed scan bookkeeping | Scheduled scans update `last_scan` even when dispatch fails. | `agent/mod.rs` |
| Portfolio persistence | Loading a missing portfolio path returns `TargetPortfolio::new()` with `file_path: None`, so later `save()` is a no-op. `get_mut_target` returns a clone. | `agent/portfolio.rs` |
| Memory consistency | Memory and alert dedup files use read-modify-write without locking or atomic replacement. Concurrent calls can lose updates. | `agent/memory.rs` |

---

## Non-Goals

- Do not rework scanner, fuzzer, recon, or tool internals except where the agent request contract requires it.
- Do not add new payload types or security tools.
- Do not migrate alert providers to a new framework.
- Do not refactor the whole agent into actors unless smaller changes cannot solve the listed problems.

---

## Phase 1: Restore Feature Compilation (COMPLETED 2026-04-30)

Objective: make agent code compile under documented feature combinations before deeper changes.

Steps:

1. Fix `Agent::trigger_event` in `crates/slapper/src/agent/mod.rs`.
2. Remove the async closure that captures `&mut self`.
3. Preserve handler restoration:
   - Temporarily take `self.event_handlers`.
   - Iterate handlers.
   - Restore handlers before returning success or error.
4. Add a test that registers two handlers where the first returns an error, calls `trigger_event`, and verifies handlers are still registered afterward.
5. Run:

```bash
cargo check --lib -p slapper
cargo check --lib -p slapper --features rest-api,ai-integration
cargo test --lib -p slapper agent:: --features rest-api,ai-integration
```

Acceptance criteria:

- `rest-api,ai-integration` no longer fails at `agent/mod.rs:470`.
- Handler restoration is tested on the error path.

---

## Phase 2: Add Testable Agent Seams (COMPLETED 2026-04-30)

Objective: make agent behavior testable without real network dispatch or real alert delivery.

Current `Agent` constructs `ToolDispatcher` and `AlertRouter` directly. That makes scheduled scan behavior difficult to test deterministically.

Steps:

1. Add narrow internal seams for tests:
   - A scan dispatch abstraction, or an internal constructor that accepts a dispatcher-like test double.
   - An alert sink abstraction, or an internal constructor that accepts a test alert collector.
2. Keep the public API stable unless a public change is clearly needed.
3. Avoid broad trait hierarchies. The seam only needs what `Agent` uses:
   - `dispatch(ToolRequest) -> Result<ToolResponse>`
   - `send(Alert) -> Result<()>`
4. Prefer crate-private traits or constructors.
5. Update tests that write state to use `tempfile::TempDir` instead of `AgentConfig::default()`.

Acceptance criteria:

- Unit tests can simulate successful scan, failed scan, findings returned, and alert sends without outbound network.
- Production behavior remains unchanged except for testability.

---

## Phase 3: Enforce Operational Constraints

Objective: make `ConstraintChecker` part of the actual scan path.

Steps:

1. Determine where `OperationalConstraints` should come from:
   - Existing config if already present.
   - `AgentConfig` extension if not wired yet.
2. Add a `ConstraintChecker` field or equivalent policy object to `Agent`.
3. Before manual and scheduled scans:
   - Evaluate target.
   - Evaluate action type.
   - Evaluate approval requirement.
   - Evaluate scan depth and downgrade only if the constraints model explicitly intends fallback.
   - Evaluate rate limit.
4. Scheduled scans should skip disallowed work and log the target plus reason.
5. Manual scans should return an error for hard violations.
6. Do not silently scan forbidden targets.
7. Add tests for:
   - Manual scan blocked by forbidden target.
   - Scheduled scan skips forbidden target and does not update `last_scan`.
   - Deep scan is downgraded or rejected according to `ConstraintChecker::evaluate_scan_depth`.
   - Rate budget blocks repeated scans.
   - Approval-required action blocks execution.

Acceptance criteria:

- The only way to dispatch an agent scan is through constraint evaluation.
- Constraint tests verify dispatch was not called when blocked.

---

## Phase 4: Make Scheduling Idempotent

Objective: prevent duplicate scheduled scans within the same matching cron window.

Steps:

1. Use `TargetConfig.last_scan` to determine whether the current cron match was already handled.
2. Add a helper with explicit semantics, for example `should_run_scheduled_target(config, schedule, now) -> bool`.
3. The helper should return false if `last_scan` already falls in the same matching minute/window.
4. Only call `update_last_scan` after successful dispatch and enough memory handling to count as a run.
5. Add a scan history record for successful scheduled scans if that is the intended portfolio contract.
6. Add tests:
   - Poll twice in same matching minute dispatches once.
   - Poll in next matching minute dispatches again.
   - Failed scan does not update `last_scan`.
   - Disabled target is ignored.
   - Off-peak skip does not update `last_scan`.

Acceptance criteria:

- A 5s or 10s poll interval cannot trigger repeated scans for a once-per-minute cron expression.
- Tests pass explicit `DateTime<Utc>` values and do not depend on wall-clock time.

---

## Phase 5: Honor Cancellation End-to-End

Objective: a caller-provided cancellation token must cancel the dispatched tool request.

Steps:

1. Inspect `crate::tool::request::CancellationToken` and how tools observe it.
2. Bridge Tokio `CancellationToken` to the tool cancellation type instead of creating an unrelated token.
3. If the tool token cannot be externally cancelled, change the API shape so the agent can pass a real linked token.
4. Add tests with a fake dispatcher that records whether the request carried a cancellation token.
5. If practical, add a test where a long-running fake dispatch exits when the caller cancels.

Acceptance criteria:

- Passing `Some(token)` to `execute_scan_with_depth` gives the dispatched request a token that changes state when the caller cancels.
- `None` still means uncancellable request.

---

## Phase 6: Implement Real Config and Portfolio Reload

Objective: file watcher changes must update live agent state or fail visibly.

Steps:

1. Stop swallowing watcher initialization errors with `.ok()` unless config explicitly allows no watcher.
2. Redesign `SlapperConfigReloader` so reload has safe access to live state:
   - Reload portfolio file into `TargetPortfolio`.
   - Reload main config fields that the agent actually uses.
3. Handle files that do not exist at startup:
   - Watch parent directory if needed.
   - Start watching the file after it is created.
4. Invalid updated config should not corrupt existing live state.
5. Add tests:
   - Portfolio file change adds/removes target in live agent.
   - Invalid portfolio JSON leaves previous live portfolio intact.
   - Missing file at startup can be created later and loaded.

Acceptance criteria:

- Reload tests prove live `Agent` state changes, not just log messages.
- Watcher setup failure is not silently ignored in normal construction.

---

## Phase 7: Respect Target Type and Scope

Objective: tool requests must reflect target configuration and configured scope.

Steps:

1. Map `TargetConfig.target_type` to `crate::tool::TargetType`.
2. Reject unknown target types with clear errors.
3. Attach configured scope to the `ToolRequest` target where supported.
4. Ensure scope validation happens before dispatch for manual and scheduled scans.
5. Add tests:
   - URL target produces `TargetType::Url`.
   - Host/IP target produces the correct target type.
   - Unknown target type errors before dispatch.
   - Out-of-scope target is rejected before dispatch.

Acceptance criteria:

- Agent-created `ToolRequest` no longer hardcodes all targets as URL.
- Scope bypass through the agent path is covered by tests.

---

## Phase 8: Wire Alert Routing Correctly

Objective: findings should alert through configured channels with correct contents and dedup semantics.

Steps:

1. Decide how `TargetConfig.alert_channels` resolves to actual `AlertChannel` definitions.
2. Ensure agent startup loads channel definitions from config or a registry.
3. During `handle_findings`, route alerts only to channels selected for that target unless global defaults are configured.
4. Fix critical alert `finding_ids` to use only critical findings.
5. Review all severity buckets for consistent filtering.
6. Decide whether Email/PagerDuty failures should continue to log-and-suppress or return errors like webhook failures.
7. Add tests:
   - Critical alert contains only critical finding IDs.
   - High/medium/low alerts contain only their severity IDs.
   - Target with no alert channels follows documented behavior.
   - Target with selected channel sends to that channel only.
   - Duplicate findings are suppressed across scans via memory dedup.

Acceptance criteria:

- Alert delivery path is testable without network.
- Finding IDs in alerts match alert severity.

---

## Phase 9: Fix Portfolio Persistence Semantics

Objective: portfolio mutations should persist predictably when a path is configured.

Steps:

1. In `TargetPortfolio::load_from_file`, if the path does not exist, return an empty portfolio that still retains `file_path: Some(path.clone())`.
2. Decide what to do with `get_mut_target`:
   - Remove it if unused.
   - Rename it if it intentionally returns a clone.
   - Or replace it with an update method that mutates under the lock.
3. Add save tests using `tempfile`:
   - Missing file load, add target, save creates file.
   - Existing file load, mutate, save preserves path.
   - Invalid path validation still rejects paths outside the allowed base.
4. Consider writing portfolio files atomically via temp file plus rename.

Acceptance criteria:

- Configured portfolio path is never lost because the file was initially absent.
- Misleading mutation API is removed or corrected.

---

## Phase 10: Harden Memory Storage

Objective: avoid lost updates and partial writes in memory and dedup files.

Steps:

1. Identify all read-modify-write paths in `LongitudinalMemory`:
   - `deduplicate_findings`
   - `store_scan_results`
   - `set_baseline`
   - snapshot writes
2. Add per-file or per-target async locking inside `LongitudinalMemory`.
3. Write JSON through temp files followed by atomic rename where practical.
4. Preserve existing max-scans trimming behavior.
5. Add tests:
   - Concurrent `store_scan_results` calls for the same target preserve all scans.
   - Concurrent `deduplicate_findings` calls do not lose alerted IDs.
   - Corrupt target memory file returns a useful error and does not overwrite data unexpectedly.

Acceptance criteria:

- Concurrent memory tests pass reliably under normal parallel test execution.
- Alert dedup cannot lose IDs through two simultaneous writes in one process.

---

## Phase 11: Integration Tests for the Real Harness

Objective: cover the complete agent control path, not just isolated modules.

Add tests that exercise:

1. Manual scan success:
   - Constraints pass.
   - Dispatch called once.
   - Memory stored.
   - Findings handled.
2. Manual scan blocked:
   - Constraints fail.
   - Dispatch not called.
   - Memory not written.
3. Scheduled scan success:
   - Cron matches.
   - Off-peak allows.
   - Dispatch called once.
   - `last_scan` updated after success.
4. Scheduled scan failure:
   - Dispatch returns error.
   - `last_scan` not updated.
   - Next poll can retry.
5. Findings and baseline:
   - Baseline findings do not alert.
   - New findings alert once.
   - Already-alerted findings do not alert again.
6. Config reload:
   - Changed portfolio affects next scheduling pass.

Acceptance criteria:

- Tests use fake dispatcher and alert sink.
- Tests do not make network calls.
- Tests use temp dirs for memory and portfolio files.

---

## Verification Commands

Run after each phase if touched code compiles quickly:

```bash
cargo check --lib -p slapper
cargo test --lib -p slapper agent::
```

Run before handing back the branch:

```bash
cargo test --lib -p slapper
cargo test --test agent_tests -p slapper --features rest-api
cargo check --lib -p slapper --features rest-api,ai-integration
cargo check --lib -p slapper --features python-plugins,ruby-plugins
```

If a command fails for a pre-existing unrelated reason, record the exact command and the first relevant compiler/test error in the handoff notes.

---

## Handoff Notes

- Start with Phase 1. Do not skip the feature compile failure.
- Prefer small test seams over making private fields public.
- Keep all agent tests isolated with `tempfile::TempDir`; avoid `AgentConfig::default()` for tests that write memory.
- Use `rg` to verify current call sites before removing or renaming any API.
- Verify each line reference and behavior against the current code before editing.
