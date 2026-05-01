# Slapper Agent Harness Improvement Plan

**Date**: 2026-05-01
**Status**: ✅ ALL PHASES COMPLETED
**Priority**: Complete

## Summary

All phases from the original plan have been implemented and verified:

| Phase | Description | Status |
|-------|-------------|--------|
| Phase 1 | Restore Feature Compilation | ✅ COMPLETED 2026-04-30 |
| Phase 2 | Add Testable Agent Seams | ✅ COMPLETED 2026-04-30 |
| Phase 3 | Enforce Operational Constraints | ✅ COMPLETED 2026-04-30 |
| Phase 4 | Make Scheduling Idempotent | ✅ COMPLETED 2026-05-01 |
| Phase 5 | Honor Cancellation End-to-End | ✅ COMPLETED 2026-05-01 |
| Phase 6 | Implement Real Config and Portfolio Reload | ✅ COMPLETED 2026-05-01 |
| Phase 7 | Respect Target Type and Scope | ✅ COMPLETED 2026-05-01 |
| Phase 8 | Wire Alert Routing Correctly | ✅ COMPLETED 2026-05-01 |
| Phase 9 | Fix Portfolio Persistence Semantics | ✅ COMPLETED 2026-05-01 |
| Phase 10 | Harden Memory Storage | ✅ COMPLETED 2026-05-01 |
| Phase 11 | Integration Tests for the Real Harness | ✅ COMPLETED 2026-05-01 |

## Current Verified Problems - ALL FIXED

All problems identified on 2026-04-30 have been resolved:

| Area | Problem | Status |
|------|---------|--------|
| Feature build | `rest-api` fails to compile at `agent/mod.rs:470` | ✅ Fixed: removed duplicate `constraints` module |
| Constraints | `ConstraintChecker` not called by scans | ✅ Fixed: integrated into scan path |
| Scheduling | `process_scheduled_scans` can run repeatedly | ✅ Fixed: added idempotent check with `should_run_target()` |
| Cancellation | Creates unrelated cancellation token | ✅ Fixed: bridged Tokio token to tool token |
| Hot reload | Watcher errors swallowed, no state reload | ✅ Fixed: errors propagated, state reloaded |
| Target typing/scope | Always uses `TargetType::Url` | ✅ Fixed: maps `target_type` and attaches scope |
| Alert routing | Empty `AlertRouter`, incorrect IDs | ✅ Fixed: wired channels, fixed critical alert IDs |
| Alert contents | Critical alerts include all finding IDs | ✅ Fixed: only critical IDs used |
| Failed scan bookkeeping | Updates `last_scan` on failure | ✅ Fixed: only update on success |
| Portfolio persistence | `file_path: None` after load | ✅ Fixed: retain path, atomic writes |
| Memory consistency | No locking, partial writes | ✅ Fixed: per-target locks, atomic writes |

## Non-Goals

- Do not rework scanner, fuzzer, recon, or tool internals except where the agent request contract requires it.
- Do not add new payload types or security tools.
- Do not migrate alert providers to a new framework.
- Do not refactor the whole agent into actors unless smaller changes cannot solve the listed problems.

## Verification Commands

Run before handing back the branch:

```bash
cargo test --lib -p slapper
cargo test --lib -p slapper agent:: --features rest-api,ai-integration
cargo check --lib -p slapper --features rest-api,ai-integration
cargo check --lib -p slapper --features python-plugins,ruby-plugins
```

## Handoff Notes

- ✅ ALL PHASES COMPLETED
- Preferences small test seams over making private fields public.
- Keep all agent tests isolated with `tempfile::TempDir`; avoid `AgentConfig::default()` for tests that write memory.
- Use `rg` to verify current call sites before removing or renaming any API.
- Verify each line reference and behavior against the current code before editing.
- The skills directory has been reorganized to `.opencode/skills/slapper-agent/`.
