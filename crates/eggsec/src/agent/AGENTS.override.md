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

## Enforcement Helpers (`enforcement.rs`)

Factored helper functions for per-scan enforcement, called immediately before dispatch in `execute_scan_with_depth`:

- `risk_for_agent_scan_depth(depth, scan_type)` - Maps `ScanDepth` + scan type string to `OperationRisk`. Pattern-matches scan type for stress/load/packet/credential/remote keywords; falls back to depth-based mapping.
- `capabilities_for_agent_scan(scan_type, depth)` - Builds the `Vec<Capability>` for the scan. Shallow gets `ActiveProbe`+`Crawl`; deep gets `HttpFuzzLowImpact`. Additional capabilities appended based on scan type keywords.
- `operation_descriptor_for_agent_scan(...)` - Constructs the `OperationDescriptor` with mode, risk, and capabilities for `EnforcementContext::evaluate()`.

These replace inline risk/capability mapping that was previously duplicated in `execute_scan_with_depth`. The enforcement invariant holds: `EnforcementContext::evaluate()` is the mandatory pre-dispatch gate; scope provenance must come from `LoadedScope`.

### Audit Integration (Phase 10)

Agent enforcement decisions emit normalized `EnforcementAuditEvent` records via `crate::audit`. Audit events are emitted for:
- Preflight evaluations (advisory, using `audit_event_from_preflight()`)
- Hard enforcement gate decisions (Allow, Warn/deny-as-deny, RequireConfirmation, Deny)

Agent uses `ExecutionSurface::SecurityAgent` for audit records. Agent never accepts manual overrides.

## Test Seams

Prefer small test seams over making private fields public:
- `ScanDispatcherTrait` - dispatch scans to tools
- `AlertSenderTrait` - send alerts via router
- `Agent::new_for_test(...)` - create agent with custom dispatch/alert (no enforcement required)
- `new_agent_for_test_default()` - convenience helper in `mod tests` for tests that need a default agent

## Observability

Logging is configured centrally in `logging/init.rs` via `init_logging()`:
- When the `agent` subcommand runs, a rolling JSON file layer (`agent.log`, daily rotation) is composed alongside the console layer
- The `WorkerGuard` returned by `init_logging()` is held in `main.rs` for process lifetime
- Agent-specific log directory is derived from the agent's `memory_dir`

## Config Hot-Reloading

`agent/config_watcher.rs` provides `ConfigWatcher`:
- Stored as `Option<ConfigWatcher>` field (`agent/mod.rs:185`)
- Wired in `Agent::new()` at config watcher creation
- Use `EggsecConfigReloader` for portfolio + main config paths
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
- Use `new_agent_for_test_default()` for tests that need a basic agent (no enforcement, mock dispatch)
- Avoid `AgentConfig::default()` with `Agent::new()` — it requires enforcement; use `new_for_test()` instead
- Verify call sites with `rg` before removing/renaming APIs
- Use `TaskStatus` enum for task state transitions in tests

## Known Issues

- **Panic in alerts/routing.rs:251**: `.expect("HMAC can take key of any size")` - safe per HMAC documentation (accepts any key size), but inconsistent with the "no expect() in agent" pattern. Consider converting to `?` for consistency.