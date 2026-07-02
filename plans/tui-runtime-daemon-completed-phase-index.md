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
| 6a | Embedded Runtime Compatibility Closure | ✅ Complete | `eggsec-runtime/src/capabilities.rs`, architecture guards |
| 6b | Pre-Daemon Runtime Readiness | ✅ Complete | `crates/eggsec-runtime/tests/in_process_client.rs`, `crates/eggsec-tui/src/app/task_dispatcher.rs`, `crates/eggsec-tui/src/app/runtime_adapter/mod.rs` |
| 7–14 | Daemon, Remote Attach, Transport, Plugin | ⏳ Not started | See `tui-runtime-daemon-roadmap.md` |
| CP | Security Corrective Pass | ✅ Complete | `crates/eggsec-daemon/src/client_registry.rs`, `host.rs`, `protocol.rs` |

## Closure Pass (Phase 6a) Deliverables

The tightening closure plan (`tui-runtime-daemon-tightening-closure-plan.md`) completed these workstreams:

1. **Typed Result Bridge** — `TaskOutcome::Result(TaskResultEnvelope)` added to `eggsec-runtime`. `TuiTaskDispatcher` returns envelope with kind+summary. Typed `TaskResult` still flows through `result_rx` compatibility channel for TUI rendering.
2. **Runtime Capability Truthfulness** — Fixed `supports_multiple_active_tasks: false`, `transports: ["in-process"]`, added missing task kinds.
3. **Engine/Runtime Dependency Boundary** — Decision: keep `eggsec → eggsec-runtime` dependency intentionally. Guardrails documented.
4. **Feature-Gated Dispatch Verification** — 6 representative feature profiles checked. No regressions.
5. **TUI Runtime Adapter Edge Case Tests** — 9 tests added covering unknown tasks, duplicate events, zero/nil progress, auto-registration, tab routing.
6. **Plan Audit Trail** — This file.
7. **Architecture Guards** — 5 guards added to `scripts/check-architecture-guards.sh` (workers absent, no TUI deps in runtime, no transport deps, no unimplemented transports, no canonical TaskConfig/TaskResult in TUI).

## Pre-Daemon Readiness (Phase 6b) Deliverables

The pre-daemon readiness plan (`tui-runtime-daemon-phase-06-pre-daemon-readiness.md`) completed these workstreams:

1. **In-Process Client Contract Tests** — 22 integration tests in `crates/eggsec-runtime/tests/in_process_client.rs` proving runtime works independently of TUI: session creation with 4 surfaces (CliManual, McpServer, RestApi, SecurityAgent), scope binding, task lifecycle events, cancellation, timeout, failure reporting, multi-session independence, single-active-task policy, result envelopes (Result/Text/JSON), snapshot serialization, capabilities verification.
2. **Result Envelope Completeness Audit** — Verified all 25 `TaskResult` variants have envelope mappings. Added 3 new tests for OAuth, WafBypass, and Pipeline summary formatting. Total envelope tests: 20.
3. **Capability Reporting Audit** — Verified `RuntimeCapabilities::default()` truthfully reflects implemented behavior. No gaps found.
4. **Embedded TUI Runtime Adapter Regression Tests** — 7 new tests added: envelope outcome delivery, cancel-then-clean-state, multiple independent cancellations, idempotent duplicate completions, auto-register-then-cancel, envelope result variant propagation. Total runtime adapter tests: 28.
5. **Documentation** — Phase index updated.

## Key Types (Post-Phase 6b)

- `eggsec-runtime`: `Runtime`, `RuntimeConfig`, `SessionOptions`, `RunRequest`, `TaskKind`, `TaskOutcome`, `RuntimeEvent`, `RuntimeCapabilities`, `TaskResultEnvelope`, `ArtifactRef`, `PolicyPrompt`, `RuntimeTaskExecutor`, `TaskDispatcher`
- `eggsec-tui`: `TuiTaskDispatcher` (implements `TaskDispatcher`), `TuiExecutor` (implements `RuntimeTaskExecutor`), `TuiRuntimeAdapter` (event reducer)
- `eggsec::dispatch`: `dispatch_inner()`, `TaskResult` (~25 variants, some feature-gated)

## Follow-Up Items

- `task_result_to_envelope()` is wired into `TuiTaskDispatcher::dispatch()` — ready for REST/MCP surfaces to consume `TaskResultEnvelope`
- Pre-existing feature-gate compile errors in `stress-testing` fixed (missing `ProxyEntry::new`/`to_log_key`/`ProxyManager::get_all_healthy_proxies` stubs); `db-pentest`, `web-proxy`, `wireless` compile cleanly
- Phase 7+ (daemon transport) blocked on this readiness gate
- Runtime has 22 in-process client tests + 64 total tests across 3 test suites
- TUI runtime adapter has 28 tests, task dispatcher has 21 tests

## Security Corrective Pass (CP) Deliverables

The security corrective pass (`tui-runtime-daemon-security-corrective-pass.md`) addressed semantic security risks in the daemon authorization model:

1. **Centralized Command Authorization** — Replaced stringly-typed permission names with `CommandPermission` enum in `client_registry.rs`. Every `ClientCommand` variant now maps to a permission level via `command_permission()`. Adding a new command without updating the mapping causes a compile error.
2. **Actual Runtime Surface for Authorization** — Fixed `check_command_permission()` in `host.rs` to query `Runtime::session_surface()` for the actual session surface instead of deriving it from `SessionAccess.default_controller_allowed`. `SessionAccess` now stores `surface: RuntimeSurface` and `owner_client_kind: ClientKind` directly.
3. **Policy Approval Semantics** — `ApprovePolicy` now returns `ErrorCode::Unsupported` with explicit message instead of silently succeeding as a no-op placeholder. No client can believe an approval happened when it did not.
4. **Client Declaration Hardening** — `CreateSession` is classified as `CommandPermission::DeclaredClient` (allowed before declaration but without owner attribution). Session-scoped commands (`GetSnapshot`, `SubmitTask`, etc.) require declared client and return `ErrorCode::ClientNotDeclared` if missing.
5. **Strict Surface Approval Restriction** — On strict sessions (McpServer, RestApi, etc.), only the session Owner can approve policies. Controllers, Approvers, and Observers from unrelated clients are denied.
6. **New Error Codes** — Added `ClientNotDeclared`, `Unsupported`, and `InvalidState` to `ErrorCode` enum with roundtrip tests.
7. **Comprehensive Denial Tests** — Added tests for observer denial, approver denial on strict surfaces, unrelated TUI denial on strict sessions, and undeclared client denial.
8. **Local Socket Security** — Verified: Unix socket only (no TCP fallback), runtime crate has no transport dependencies, socket cleanup is path-safe. Documented as non-goal: public network exposure.

## References

- Roadmap: `plans/tui-runtime-daemon-roadmap.md`
- Closure plan: `plans/tui-runtime-daemon-tightening-closure-plan.md`
- Pre-daemon readiness plan: `plans/tui-runtime-daemon-phase-06-pre-daemon-readiness.md`
- Architecture: `architecture/tui.md`
- Guards: `scripts/check-architecture-guards.sh`
- In-process client tests: `crates/eggsec-runtime/tests/in_process_client.rs`
