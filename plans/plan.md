# Slapper Agent Harness Improvement Plan

## Status

Iteration 2 handoff plan, updated after a partial implementation attempt.

Do not restore the old completed CookieStore/Regex/AgentLogger plan items. They are not part of this work. This file should contain only the remaining agent-harness work.

## Current Verification Snapshot

Last checked on 2026-05-01:

```bash
cargo check --lib -p slapper --features rest-api
```

Result: passes with warnings.

```bash
cargo check --lib -p slapper --features rest-api,ai-integration
```

Result: fails.

Current compile failure:

- `crates/slapper/src/commands/handlers/agent.rs:83`
- The `ai-integration` branch calls `agent.with_ai_client(ai_config.unwrap()).await`, but `ai_config` is not in scope.
- Fix this before doing any deeper work so both required feature combinations compile.

## Relevant Files

- `crates/slapper/src/agent/mod.rs`
- `crates/slapper/src/agent/portfolio.rs`
- `crates/slapper/src/agent/AGENTS.override.md`
- `crates/slapper/src/tool/agents/scheduler.rs`
- `crates/slapper/src/tool/agents/lifecycle.rs`
- `crates/slapper/src/tool/agents/registry.rs`
- `crates/slapper/src/tool/agents/communication.rs`
- `crates/slapper/src/tool/protocol/agent_routes.rs`
- `crates/slapper/src/commands/handlers/agent.rs`
- `crates/slapper/src/cli/agent.rs`

## Implementation Rules For The Next Agent

- Read `AGENTS.md` and `crates/slapper/src/agent/AGENTS.override.md` before editing.
- Keep changes scoped to this plan.
- Use existing test seams in `agent/mod.rs`:
  - `ScanDispatcherTrait`
  - `AlertSenderTrait`
  - `Agent::new_for_test(...)`
- Do not make private fields public just to test them.
- Prefer focused regression tests over broad refactors.
- Use `tempfile::TempDir` for portfolio/memory tests.
- After each workstream, run the smallest relevant check before moving on.

## What Was Partially Implemented

These items exist in the current code, but some are incomplete or incorrect:

- Callback URL validation was added in `tool/protocol/agent_routes.rs`.
- `TaskStatus`, `assigned_agent_id`, and `leased_until` were added to `ScheduledTask`.
- Task list now returns actual queued task summaries.
- Lease and result endpoints were added:
  - `POST /api/v1/tasks/{id}/lease`
  - `POST /api/v1/tasks/{id}/result`
- `Agent::run_once()` was added.
- CLI target commands now mostly load a portfolio instead of always using `TargetPortfolio::new()`.
- `trigger_event(...)` now catches handler panics and restores handlers on the explicit error paths.
- Scheduled scan path now attempts `portfolio.save()` after updating `last_scan`.

Do not assume these are complete. The sections below list the remaining defects.

## Workstream 0: Restore Build Health

### Problem

`rest-api,ai-integration` does not compile.

Current failure:

- `crates/slapper/src/commands/handlers/agent.rs:83`
- `ai_config` is referenced but not defined.

### Required Fix

- Preserve the loaded AI config before moving `config` into `Agent::new(config).await?`.
- Avoid `unwrap()` unless the branch invariant is very obvious and tested.
- Keep behavior:
  - If `--with-ai` and valid AI config are provided, call `with_ai_client(...)`.
  - If `--with-ai` is requested but AI config cannot be loaded, behavior should be explicit. Prefer a warning or error over silently running without AI.

### Acceptance Criteria

```bash
cargo check --lib -p slapper --features rest-api
cargo check --lib -p slapper --features rest-api,ai-integration
```

Both pass.

## Workstream 1: Correct Callback URL SSRF Validation

### Status: COMPLETE

### Current Code

- Validator lives in `crates/slapper/src/tool/protocol/agent_routes.rs`.
- It uses `url::Url`, `ToSocketAddrs`, and helper functions:
  - `validate_callback_url`
  - `is_forbidden_ip`
  - `is_private_ip`
  - `is_link_local_ip`
  - `is_documentation_ip`

### Remaining Problems

1. DNS validation checks only the first resolved IP:
   - Current code collects `addrs` and checks `addrs.first()`.
   - If a hostname resolves to both public and private IPs, private IPs after the first can pass.
2. IPv6 private detection is wrong:
   - Current expression compares the first segment to `0xfc00 >> 8` and `0xfd00 >> 8`.
   - Unique-local IPv6 is `fc00::/7`; check `(segments[0] & 0xfe00) == 0xfc00`.
3. IPv6 link-local detection is wrong:
   - Link-local is `fe80::/10`; check `(segments[0] & 0xffc0) == 0xfe80`.
4. Documentation IPv4 detection checks only exact `.0` addresses:
   - Must reject all of:
     - `192.0.2.0/24`
     - `198.51.100.0/24`
     - `203.0.113.0/24`
5. Benchmark IPv4 detection only checks `198.18.x.x`, but benchmark range is `198.18.0.0/15`, including `198.19.x.x`.
6. Hostname resolution during validation may perform live DNS in unit tests.
   - Avoid tests that depend on external DNS such as `example.com`, or inject a resolver.
7. `localhost` is only rejected because it resolves locally. Make this explicit before DNS resolution.
8. The route returns `&'static str` errors, so all invalid callback cases become a generic 500-ish style handler error depending on Axum conversion. Prefer explicit `400 Bad Request`.

### Required Behavior

- Reject unsafe callback URLs at registration time.
- Only allow `http` and `https`.
- Reject embedded credentials.
- Reject missing host.
- Reject explicit forbidden IP literals.
- Reject hostnames that resolve to any forbidden IP.
- Reject `localhost` and localhost-like case-insensitive spelling directly.
- For testability, use an injectable or helper-based resolver. The registration route can use the system resolver, but unit tests should not need network.
- Return a client error for invalid callback URLs.

### Suggested Implementation Shape

- Move callback validation helpers into a small internal struct or module inside `agent_routes.rs`, unless reuse is needed elsewhere.
- Add:
  - `fn is_forbidden_ip(ip: IpAddr) -> bool`
  - `fn validate_callback_url_with_resolver<F>(url: &str, resolver: F) -> Result<(), CallbackUrlValidationError>`
  - where `F: Fn(&str, u16) -> Result<Vec<IpAddr>, CallbackUrlValidationError>`
- Use the actual URL port or scheme default, not `(host, 0)`.
- Validate every resolved IP with `any(is_forbidden_ip)`.
- Keep `validate_callback_url(url)` as the production wrapper.

### Tests To Add Or Fix

- Direct IP rejection:
  - `127.0.0.1`
  - `127.255.255.255`
  - `10.0.0.1`
  - `172.16.0.1`
  - `172.31.255.255`
  - `192.168.1.1`
  - `169.254.169.254`
  - `0.0.0.0`
  - `224.0.0.1`
  - `192.0.2.55`
  - `198.51.100.55`
  - `203.0.113.55`
  - `198.18.0.1`
  - `198.19.255.255`
  - `::1`
  - `fc00::1`
  - `fd00::1`
  - `fe80::1`
- Hostname validation:
  - `localhost` rejected without DNS.
  - fake resolver returning `[8.8.8.8, 10.0.0.1]` is rejected.
  - fake resolver returning `[8.8.8.8]` is accepted.
- Route-level invalid callback returns `400 Bad Request`.

## Workstream 2: Fix Scheduler Semantics

### Status: COMPLETE

### Current Code

- `TaskStatus` was added.
- `ScheduledTask` has:
  - `status`
  - `assigned_agent_id`
  - `leased_until`
- `next_task()` skips future scheduled tasks and leased tasks.
- `lease_task()` marks a queued task as leased.
- `submit_result()` marks leased tasks completed or failed.

### Remaining Problems

1. `requeue()` still writes to `retry_queue`, but `next_task()` never drains `retry_queue`.
2. `retry_count()` reports stranded tasks in `retry_queue`.
3. `cancel()` only cancels pending tasks in the main queue. It cannot cancel leased, delayed, failed, or retry-queue tasks.
4. `submit_result()` discards the result and error arguments.
5. Failed tasks are not automatically retried according to policy.
6. Lease expiration is not enforced. `leased_until` is set but never used to make a task available again.
7. `create_task` route accepts `agent_id` but sets `assigned_agent_id: None`.
8. `next_task()` removes the task from the queue entirely, while `lease_task()` expects tasks to remain in the queue. These are competing models.

### Required Decision

Choose one scheduler model and make it consistent:

#### Preferred Model: Queue With Leasing

- Tasks stay in the scheduler until completed/cancelled/expired retention.
- Agents lease tasks instead of callers removing them with `next_task()`.
- `next_task()` should either:
  - be removed/replaced by `lease_next_task(agent_id, capabilities, lease_duration_ms)`, or
  - only be used by tests/internal code and should mark the task leased instead of removing it.

### Required Behavior

- One authoritative storage path for all tasks. Avoid a separate undrained `retry_queue`.
- Pending due tasks can be leased.
- Future scheduled tasks cannot be leased before `scheduled_for`.
- Leased tasks cannot be leased by another agent before `leased_until`.
- Expired leases become pending again.
- Failed tasks retry if `retry_count < max_retries`.
- Cancel works for pending, delayed, leased, and retryable failed tasks.
- Submitted result/error is retained somewhere queryable.

### Suggested Implementation Shape

- Replace `retry_queue` with one `VecDeque<ScheduledTask>` or `Vec<ScheduledTask>`.
- Extend `ScheduledTask`:
  - `result: Option<serde_json::Value>`
  - `error: Option<String>`
  - `completed_at: Option<u64>`
  - `updated_at: Option<u64>`
- Add helper:
  - `fn now_ms() -> u64`
  - `fn is_due(task, now_ms) -> bool`
  - `fn lease_expired(task, now_ms) -> bool`
- Add scheduler methods:
  - `lease_next_task(agent_id, lease_duration_ms) -> Option<ScheduledTask>`
  - `lease_task(task_id, agent_id, lease_duration_ms) -> bool`
  - `submit_result(task_id, success, result, error) -> bool`
  - `cancel(task_id) -> bool`
  - `get_task(task_id) -> Option<ScheduledTask>`
  - `list_all_tasks() -> Vec<ScheduledTask>`
- When submitting failure:
  - If retries remain, increment `retry_count`, set `status = Pending`, set `scheduled_for` to a retry delay if policy exists, and preserve last error.
  - If retries exhausted, set `status = Failed`.

### Tests

- `requeue()` no longer strands a task.
- Delayed task is not leaseable before due time.
- Delayed task is leaseable after due time.
- Leased task is not returned to another agent before lease expiry.
- Expired leased task becomes leaseable.
- Failed task with retries returns to pending and increments `retry_count`.
- Failed task without retries remains failed.
- Cancel works on pending and leased tasks.
- `result` and `error` are retained after completion/failure.

## Workstream 3: Fix Task Routes

### Current Code

- `GET /api/v1/tasks` now returns actual summaries.
- `POST /api/v1/tasks/{id}/lease` exists.
- `POST /api/v1/tasks/{id}/result` exists.

### Remaining Problems

1. `GET /api/v1/tasks/{id}` still returns hardcoded `not_found`.
2. `CreateTaskRequest.agent_id` is ignored.
3. Lease endpoint does not verify that the agent exists or is active/idle.
4. Lease endpoint requires a task ID; there is no route for “give this agent the next eligible task”.
5. Result submission does not verify that the submitting agent owns the lease.
6. Route errors are still mostly `&'static str`, which makes proper status codes hard.

### Required Behavior

- `GET /api/v1/tasks/{id}` returns real task details or `404`.
- Task creation either preserves `agent_id` or rejects it as unsupported.
- Lease operation verifies agent exists and is not offline.
- Add a next-task lease route if real external agents are intended:
  - `POST /api/v1/agents/{id}/tasks/lease`
  - or `POST /api/v1/tasks/lease-next`
- Result submission should require `agent_id` and verify it matches `assigned_agent_id`.
- Invalid request data should return `400`, missing task/agent should return `404`, auth failure should return `401`.

### Suggested Implementation Shape

- Introduce small response DTO:
  - `TaskDetail`
  - Include id, task_type, payload, priority, status, retry_count, timestamps, assigned_agent_id, result, error.
- Change handlers that need status codes to return `Result<impl IntoResponse, (StatusCode, String)>`.
- Do not expose internal structs directly unless they already derive the right serialization traits.

### Tests

- Create task then get by ID returns task details.
- Get unknown task returns `404`.
- Create with `agent_id` either stores assignment or returns `400`.
- Lease unknown task returns `404` or `leased: false` with a clear status; prefer `404`.
- Lease by unknown/offline agent fails.
- Submit result by non-owner fails.
- Submit result by owner succeeds and result is visible in `GET`.

## Workstream 4: Fix Lifecycle Monitor

### Status: COMPLETE

### Current Code

- `HealthIssue::CallbackUnhealthy(String)` was added.
- `saturating_sub` is used for heartbeat age.
- Callback issue dedupe was attempted.

### Fixed Problems

1. `perform_health_check()` now splits work into phases:
   - Phase 1: Read agents and compute `is_stale` with read lock
   - Phase 2: Await callback probes outside any lock
   - Phase 3: Acquire write lock and update `AgentHealth`
2. `start_health_monitor()` now returns `JoinHandle<()>` and uses `tokio::select!`.
3. Added `start_health_monitor_with_token(token)` for testability.
4. Recovery condition fixed: now checks `!was_healthy && !is_stale && !callback_unhealthy` with proper state change detection.
5. Stale/unhealthy agents now set to `AgentStatus::Offline` instead of `Idle`.
6. `LifecycleEventType` derives `PartialEq` for test comparisons.

### Required Behavior

- Do not hold the health write lock while awaiting HTTP. ✅
- Monitor can be stopped. ✅
- Recovery clears issues and emits `AgentRecovered` once when state changes from unhealthy to healthy. ✅
- Stale/unhealthy status should not be `Idle`. ✅
- Callback failure event should not spam each interval. ✅

### Tests Added

- Slow callback does not block `record_task_start`/`record_task_success`. ✅
- Callback failure emits one stale event while issue remains. ✅
- Healthy callback after failure emits one recovery event and clears issues. ✅
- Future heartbeat timestamp does not panic. ✅
- Health monitor can be stopped. ✅
- Stale agent status is not `Idle`. ✅

## Workstream 5: Fix Agent Runtime Shutdown And Run State

### Current Code

- `Agent::run()` still creates a local `CancellationToken`.
- `Agent::run()` still spawns a detached ctrl-c waiter.
- `Agent::stop()` only sets `running = false`.
- `Agent::run_once()` was added but sets `running = true` and never clears it.

### Remaining Problems

1. `stop()` does not wake `run()` promptly. The loop can wait until the next interval tick.
2. `running` is not reset when `run()` exits via ctrl-c token.
3. `run_once()` leaves the agent permanently marked running.
4. A second `run()` or `run_once()` can return early because stale `running` remains true.
5. Detached ctrl-c task is never joined or cancelled.

### Required Behavior

- `stop()` wakes the loop immediately.
- `running` is false after any `run()` or `run_once()` exit path.
- Repeated `run_once()` works.
- Long-lived background tasks are cancellable or do not accumulate per run.

### Suggested Implementation Shape

- Add a shutdown token field to `Agent`, or use a `tokio::sync::Notify`.
- `run()` should select on:
  - shutdown token/notify
  - ctrl-c
  - interval tick
- Avoid spawning a ctrl-c task every run if direct `tokio::select!` can wait on `tokio::signal::ctrl_c()`.
- Use a small guard or explicit cleanup to reset `running = false`.
- `run_once()` should set running only if needed, and must reset it before return, including error paths.

### Tests

- `run_once()` can be called twice on the same agent.
- `run_once()` resets running after a successful pass.
- `run_once()` resets running after `process_scheduled_scans()` returns an error.
- `stop()` wakes `run()` in much less than `poll_interval_secs`.
- `run()` does not leave stale running state after cancellation.

## Workstream 6: Fix Scheduled Scan Persistence

### Current Code

- `process_scheduled_scans()` now calls `self.portfolio.save()` after `update_last_scan`.

### Remaining Problems

1. Confirm that `TargetPortfolio::save()` writes to the same path that was loaded from `AgentConfig.portfolio_path`.
2. If `TargetPortfolio::new()` has no path, saving may write to an unexpected default path or fail.
3. Saving happens before `store_scan_results()`. Decide whether that ordering is correct.
4. No scan history record appears to be added in the scheduled scan path.

### Required Behavior

- Successful scheduled dispatch persists `last_scan` to the intended portfolio path.
- Failed dispatch does not update/persist `last_scan`.
- If no portfolio path exists, behavior should be explicit.
- If scan history is part of the portfolio contract, add a `ScanRecord` and persist it.

### Tests

- Temp portfolio loaded through `AgentConfig.portfolio_path` updates `last_scan` on disk after successful scheduled scan.
- Reloading the portfolio sees the updated `last_scan`.
- Failed dispatch leaves `last_scan` unchanged.
- No portfolio path case is tested or explicitly documented.

## Workstream 7: Finish CLI Portfolio Fixes

### Current Code

- Helper functions were added:
  - `resolve_portfolio_path`
  - `load_portfolio_for_cli`
- Most target subcommands use them.
- Skill default paths now use `expand_path`.

### Remaining Problems

1. `handle_status_impl()` still does not use `resolve_portfolio_path`/`load_portfolio_for_cli`.
   - With no `--portfolio`, it still shows an empty `TargetPortfolio::new()`.
   - With `~/...`, it does not expand the path.
2. Target subcommands call `portfolio.save()` but need confirmation that save uses the same path resolved by CLI.
3. No command-handler tests were observed for add/list/enable/disable/remove persistence.

### Required Behavior

- Status, list, add, update, remove, enable, disable all operate on the same resolved path.
- `~` is expanded consistently.
- Existing targets are preserved when adding a target.

### Tests

- Add then status/list against temp portfolio shows the target.
- Add second target preserves first.
- Enable/disable persist to disk.
- Remove persists to disk.
- `~/...` paths are expanded in status and targets commands.

## Workstream 8: Event Handler Panic Safety

### Current Code

- `trigger_event(...)` catches unwind with `FutureExt::catch_unwind()`.
- It restores handlers on the explicit `Ok(Err(...))` and panic branches.

### Remaining Concerns

- There is no visible panic regression test yet.
- The import `std::panic::AssertUnwindSafe` is unused because the code calls `std::panic::AssertUnwindSafe(...)` fully qualified.

### Required Behavior

- Handler list is restored after success, normal error, and panic.
- Panic behavior is documented by tests: either converted to `Err` or resumed after restoration. Current code converts to `Err`.

### Tests

- Add a panicking handler test.
- Assert `trigger_event(...)` returns an error containing "panicked" or the chosen message.
- Assert subsequent event trigger still sees the registered handlers.

## Workstream 9: Verification And Cleanup

### Required Commands

Run at minimum:

```bash
cargo check --lib -p slapper --features rest-api
cargo check --lib -p slapper --features rest-api,ai-integration
cargo test --lib -p slapper --features rest-api
cargo test --test agent_tests -p slapper --features rest-api
```

If AI behavior is changed beyond the compile fix:

```bash
cargo test --lib -p slapper --features rest-api,ai-integration
```

### Warnings To Clean If Touching The Area

- `crates/slapper/src/tool/agents/scheduler.rs`
  - `submit_result` currently does not use `result` or `error`.
- `crates/slapper/src/tool/protocol/agent_routes.rs`
  - callback validation error variable is unused.
- `crates/slapper/src/agent/mod.rs`
  - unused import of `std::panic::AssertUnwindSafe`.

Warnings outside this workstream can be left alone unless the touched code makes them worse.

## Suggested Execution Order

1. Workstream 0: restore `rest-api,ai-integration` compile.
2. Workstream 1: fix callback URL validation correctness.
3. Workstream 2: make scheduler state model coherent.
4. Workstream 3: fix task routes on top of the scheduler model.
5. Workstream 4: fix lifecycle lock, recovery, and monitor cancellation.
6. Workstream 5: fix `Agent::run`, `run_once`, and `stop` state.
7. Workstream 7: finish CLI status/path handling.
8. Workstream 6: verify scheduled scan persistence.
9. Workstream 8: add panic-safety regression test.
10. Workstream 9: run full verification.

## Review Notes From Current Attempt

- The current attempt is progress but is partial. Do not assume a workstream is done just because a type or endpoint exists.
- The largest correctness issue is the scheduler having two conflicting models: removing tasks via `next_task()` and retaining tasks for lease/result APIs.
- The largest security issue is callback validation checking only the first resolved address and having incorrect IPv6/range checks.
- The largest reliability issue is lifecycle health checks still awaiting network I/O under a write lock, plus runtime state never being reset.
