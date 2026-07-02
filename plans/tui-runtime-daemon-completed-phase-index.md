# TUI Runtime/Daemon Migration — Completed Phase Index

## Purpose

Audit trail for the TUI→Runtime migration. Each phase lists status, key deliverables, and follow-up items.

## Phase Status

| Phase | Name | Status | Key Files |
|-------|------|--------|-----------|
| 0 | Architecture Inventory | ✅ Complete | `architecture/tui.md` |
| 1 | Runtime DTO and Protocol Skeleton | ✅ Complete | `eggsec-runtime/src/event.rs`, `request.rs`, `session.rs`, `capabilities.rs` |
| 2 | Task Lifecycle Extraction | ✅ Complete | `eggsec-runtime/src/runtime.rs` (Runtime, RuntimeConfig, SessionOptions) |
| 3 | Worker Dispatch Migration | ✅ Complete | `eggsec/src/dispatch/mod.rs`, `eggsec-tui/src/app/task_dispatcher.rs` |
| 4 | Runtime Event Reducer and TUI Adapter | ✅ Complete | `eggsec-tui/src/app/runtime_adapter/mod.rs` |
| 5 | Session/View State Split | ✅ Complete | `eggsec-runtime/src/session.rs`, `eggsec-tui/src/app/state_update.rs` |
| 6 | Embedded Runtime Compatibility Closure | ✅ Complete | `eggsec-runtime/src/capabilities.rs`, architecture guards |
| 7–14 | Daemon, Remote Attach, Transport, Plugin | ⏳ Not started | See `tui-runtime-daemon-roadmap.md` |

## Closure Pass (Phase 6) Deliverables

The tightening closure plan (`tui-runtime-daemon-tightening-closure-plan.md`) completed these workstreams:

1. **Typed Result Bridge** — `TaskOutcome::Result(TaskResultEnvelope)` added to `eggsec-runtime`. `TuiTaskDispatcher` returns envelope with kind+summary. Typed `TaskResult` still flows through `result_rx` compatibility channel for TUI rendering.
2. **Runtime Capability Truthfulness** — Fixed `supports_multiple_active_tasks: false`, `transports: ["in-process"]`, added missing task kinds.
3. **Engine/Runtime Dependency Boundary** — Decision: keep `eggsec → eggsec-runtime` dependency intentionally. Guardrails documented.
4. **Feature-Gated Dispatch Verification** — 6 representative feature profiles checked. No regressions.
5. **TUI Runtime Adapter Edge Case Tests** — 9 tests added covering unknown tasks, duplicate events, zero/nil progress, auto-registration, tab routing.
6. **Plan Audit Trail** — This file.
7. **Architecture Guards** — 5 guards added to `scripts/check-architecture-guards.sh` (workers absent, no TUI deps in runtime, no transport deps, no unimplemented transports, no canonical TaskConfig/TaskResult in TUI).

## Key Types (Post-Phase 6)

- `eggsec-runtime`: `Runtime`, `RuntimeConfig`, `SessionOptions`, `RunRequest`, `TaskKind`, `TaskOutcome`, `RuntimeEvent`, `RuntimeCapabilities`, `TaskResultEnvelope`, `ArtifactRef`, `PolicyPrompt`
- `eggsec-tui`: `TuiTaskDispatcher` (implements `TaskDispatcher`), `TuiExecutor` (implements `RuntimeTaskExecutor`), `TuiRuntimeAdapter` (event reducer)
- `eggsec::dispatch`: `dispatch_inner()`, `TaskResult` (~30 variants, some feature-gated)

## Follow-Up Items

- `task_result_to_envelope()` is wired into `TuiTaskDispatcher::dispatch()` — ready for REST/MCP surfaces to consume `TaskResultEnvelope`
- Pre-existing feature-gate compile errors in `stress-testing` fixed (missing `ProxyEntry::new`/`to_log_key`/`ProxyManager::get_all_healthy_proxies` stubs); `db-pentest`, `web-proxy`, `wireless` compile cleanly
- Phase 7+ (daemon transport) blocked on this closure pass completion

## References

- Roadmap: `plans/tui-runtime-daemon-roadmap.md`
- Closure plan: `plans/tui-runtime-daemon-tightening-closure-plan.md`
- Architecture: `architecture/tui.md`
- Guards: `scripts/check-architecture-guards.sh`
