# eggsec-agent Skill

Eggsec autonomous agent development — operational constraints, testable seams, event handling, scheduled scans, task scheduling, lifecycle management. Triggers: agent, constraint, scan, schedule, trigger_event, dispatch, alert, portfolio, memory, task, lease, lifecycle.

## Key Files
- `crates/eggsec/src/agent/mod.rs` — Main Agent struct, ScanDispatcherTrait, AlertSenderTrait
- `crates/eggsec/src/agent/constraints/` — ConstraintChecker, OperationalConstraints
- `crates/eggsec/src/agent/portfolio.rs` — TargetPortfolio, TargetConfig
- `crates/eggsec/src/agent/memory.rs` — LongitudinalMemory, dedup
- `crates/eggsec-agent/src/scheduler.rs` — TaskScheduler with lease-based model
- `crates/eggsec-agent/src/lifecycle.rs` — LifecycleManager with health monitoring
- `crates/eggsec/src/tool/protocol/agent_routes.rs` — REST API for agents and tasks

## Common Tasks
- Add operational constraints: Update `OperationalConstraints` in `agent/constraints.rs`, use `ConstraintChecker` in scan paths.
- Add test doubles: Use `ScanDispatcherTrait`/`AlertSenderTrait` with `Agent::new_for_test()`.
- Fix event handling: Ensure `trigger_event` restores handlers on success/error.
- Scheduled scans: Use `should_run_scheduled_target` helper, update `last_scan` only on success.

## Scheduler Model (Lease-Based)

Tasks stay in scheduler until completed/cancelled. Agents lease tasks:
- `lease_next_task(agent_id, lease_duration_ms)` — claim next available task
- `lease_task(task_id, agent_id, lease_duration_ms)` — claim specific task
- `submit_result(task_id, success, result, error)` — complete with retry logic
- Failed tasks auto-retry if `retry_count < max_retries`
- Lease expiration tracked via `leased_until` field

Key fields on `ScheduledTask`:
- `result: Option<serde_json::Value>` — retained after completion
- `error: Option<String>` — retained after failure
- `completed_at`, `updated_at` — timestamps

## Lifecycle Monitor

`LifecycleManager` monitors agent health:
- Split-phase health checks (no lock held during I/O)
- Stale agents set to `AgentStatus::Offline` (not `Idle`)
- `start_health_monitor()` returns `JoinHandle` for cancellation
- Recovery emits `AgentRecovered` once when state changes from unhealthy to healthy

## Agent Runtime Shutdown

`Agent` uses `tokio::sync::Notify` for immediate shutdown:
- `stop()` calls `shutdown_notify.notify_one()` to wake run loop
- `run()` uses `tokio::select!` on shutdown, ctrl-c, and poll interval
- `run_once()` properly resets `running = false` on all exit paths
- No detached ctrl-c task accumulation

## Callback URL Validation

`validate_callback_url()` in `agent_routes.rs`:
- Rejects localhost, loopback, private, link-local, documentation, benchmark IPs
- Validates ALL resolved IPs (not just first) to prevent DNS rebinding bypass
- Use `validate_callback_url_with_resolver(url, resolver)` for testable fake DNS

## Performance Optimizations

All agent module collections use `rustc_hash::FxHashMap` and `FxHashSet` for performance:
- `AlertRouter.recent_alerts` and `ChannelRegistry.channels`
- `LongitudinalMemory.target_locks`
- `ScanRecord.severity_counts`
- `AggregatedAlert.severity_counts`
- `PortfolioSnapshot.findings_by_severity`

## Testable Seams
- `ScanDispatcherTrait` — mock dispatch: `Box::new(MockDispatcher { response: Arc::new(Mutex::new(...)) })`
- `AlertSenderTrait` — mock alert: `Box::new(MockAlertSender { sent_alerts: Arc::new(Mutex::new(vec![])) })`
- `Agent::new_for_test(...)` — create agent with custom dispatch/alert
- Use `tempfile::TempDir` for tests writing state.

## Verification
```bash
cargo check --lib -p eggsec --features rest-api
cargo check --lib -p eggsec --features rest-api,ai-integration
cargo test --lib -p eggsec --features rest-api
cargo test --test agent_tests -p eggsec --features rest-api
```
