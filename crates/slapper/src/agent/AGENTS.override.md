# Agent Module Override

Specialized guidance for the security agent module.

## Key Types

- `Agent` - Main agent struct (`agent/mod.rs`)
- `AgentConfig` - Configuration including `operational_constraints: Option<OperationalConstraints>`
- `ConstraintChecker` - Enforces operational constraints (`agent/constraints/checker.rs`)
- `OperationalConstraints` - Constraint config (forbidden actions/targets, rate limits)
- `ScanDispatcherTrait` - Testable seam for scan dispatch (crate-private)
- `AlertSenderTrait` - Testable seam for alert sending (crate-private)
- `TaskStatus` - Task lifecycle state (`tool/agents/scheduler.rs`): `Pending`, `Leased`, `Completed`, `Failed`, `Cancelled`
- `TaskScheduler` - Manages task queue with priority and scheduled_for ordering

## Test Seams

Prefer small test seams over making private fields public:
- `ScanDispatcherTrait` - dispatch scans to tools
- `AlertSenderTrait` - send alerts via router
- `Agent::new_for_test(...)` - create agent with custom dispatch/alert

## Observability

`agent/logging.rs` provides `AgentLogger`:
- Stored as `Option<AgentLogger>` field on `Agent` struct (`agent/mod.rs:140`)
- Initialized lazily in `Agent::run()` at line 296
- Logs to `log_dir/agent.log` with daily rotation
- JSON format with thread IDs, file/line info

## Config Hot-Reloading

`agent/config_watcher.rs` provides `ConfigWatcher`:
- Stored as `Option<ConfigWatcher>` field (`agent/mod.rs:139`)
- Wired in `Agent::new()` at line 207
- Use `SlapperConfigReloader` for portfolio + main config paths
- Gracefully handles missing files via `.ok()`

## Alert Fatigue Prevention

**Baseline-Aware Alerting:**
- `Agent::process_scheduled_scans` uses `LongitudinalMemory::compare_with_baseline`
- Only NEW findings (not in baseline) trigger alerts

**Cross-Scan Deduplication:**
- `LongitudinalMemory::deduplicate_findings` prevents repeat alerts
- Alerted finding IDs stored in `alerted_findings.json`

## Handler Registry Safety

`Agent::trigger_event` uses panic-safe restoration:
- Uses `AssertUnwindSafe` + `catch_unwind()` to catch handler panics
- Handlers are restored after success, error, or panic
- Panics are converted to `Err("handler panicked")`

## Scheduled Scan Persistence

`process_scheduled_scans` persists portfolio state after successful dispatch:
- Calls `self.portfolio.save()` after updating `last_scan`
- Failure to persist causes the scan result to be treated as error
- Ensures `last_scan` survives agent restart

## CLI Portfolio Handling

All target commands in `commands/handlers/agent.rs` use consistent portfolio loading:
- `resolve_portfolio_path()` - expands `~` and resolves default path
- `load_portfolio_for_cli()` - loads portfolio from resolved path
- `TargetPortfolio::new()` is NOT used in target commands (would discard state)

## Test Best Practices

- Use `tempfile::TempDir` for isolated tests
- Avoid `AgentConfig::default()` for tests that write to memory/portfolio files
- Verify call sites with `rg` before removing/renaming APIs
- Use `TaskStatus` enum for task state transitions in tests

## Known Issues

- **Panic in alerts/routing.rs:79**: `.expect("Failed to create fallback HTTP client")` will panic if client creation fails. Convert to graceful error handling with `?` or match. This is the only remaining `expect()` in the agent module that can cause runtime crashes.