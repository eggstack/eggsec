# Agent Module Override

Specialized guidance for the autonomous agent module.

## Key Types

- `Agent` - Main agent struct (`agent/mod.rs`)
- `AgentConfig` - Configuration including `operational_constraints: Option<OperationalConstraints>`
- `ConstraintChecker` - Enforces operational constraints (`agent/constraints/checker.rs`)
- `OperationalConstraints` - Constraint config (forbidden actions/targets, rate limits)
- `ScanDispatcherTrait` - Testable seam for scan dispatch (crate-private)
- `AlertSenderTrait` - Testable seam for alert sending (crate-private)

## Test Seams

Prefer small test seams over making private fields public:
- `ScanDispatcherTrait` - dispatch scans to tools
- `AlertSenderTrait` - send alerts via router

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

**Handler Registry Safety:**
- `Agent::trigger_event` uses deferred restoration pattern
- Handlers are taken, processed, then ALWAYS restored regardless of panic/error

## Test Best Practices

- Use `tempfile::TempDir` for isolated tests
- Avoid `AgentConfig::default()` for tests that write to memory/portfolio files
- Verify call sites with `rg` before removing/renaming APIs