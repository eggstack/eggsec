# slapper-agent Skill

Slapper autonomous agent development — operational constraints, testable seams, event handling, scheduled scans. Triggers: agent, constraint, scan, schedule, trigger_event, dispatch, alert, portfolio, memory.

## Key Files
- `crates/slapper/src/agent/mod.rs` — Main Agent struct, ScanDispatcherTrait, AlertSenderTrait
- `crates/slapper/src/agent/constraints/` — ConstraintChecker, OperationalConstraints
- `crates/slapper/src/agent/portfolio.rs` — TargetPortfolio, TargetConfig
- `crates/slapper/src/agent/memory.rs` — LongitudinalMemory, dedup

## Common Tasks
- Add operational constraints: Update `OperationalConstraints` in `agent/constraints.rs`, use `ConstraintChecker` in scan paths.
- Add test doubles: Use `ScanDispatcherTrait`/`AlertSenderTrait` with `Agent::new_for_test()`.
- Fix event handling: Ensure `trigger_event` restores handlers on success/error.
- Scheduled scans: Use `should_run_scheduled_target` helper, update `last_scan` only on success.

## Testable Seams
- `ScanDispatcherTrait` — mock dispatch: `Box::new(MockDispatcher { response: Arc::new(Mutex::new(...)) })`
- `AlertSenderTrait` — mock alert: `Box::new(MockAlertSender { sent_alerts: Arc::new(Mutex::new(vec![])) })`
- Use `tempfile::TempDir` for tests writing state.

## Verification
```bash
cargo check --lib -p slapper
cargo test --lib -p slapper agent:: --features rest-api,ai-integration
```
