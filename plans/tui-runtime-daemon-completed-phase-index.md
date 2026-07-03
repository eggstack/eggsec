# TUI Runtime/Daemon Migration — Completed Phase Index

## Purpose

Audit trail for the TUI→Runtime→Daemon migration. Each phase lists status, key deliverables, plan file path, and follow-up items. This is the handoff map for future work.

## Phase Summary

| Phase | Name | Status | Plan File |
|-------|------|--------|-----------|
| 0 | Architecture Inventory | ✅ Complete | (in `tui-runtime-daemon-roadmap.md` § Phase 0) |
| 1 | Runtime DTO and Protocol Skeleton | ✅ Complete | (in `tui-runtime-daemon-roadmap.md` § Phase 1) |
| 2 | Task Lifecycle Extraction | ✅ Complete | (in `tui-runtime-daemon-roadmap.md` § Phase 2) |
| 3 | Worker Dispatch Migration | ✅ Complete | `plans/tui-runtime-daemon-phase-03-worker-dispatch-migration.md` |
| 4 | Runtime Event Reducer and TUI Adapter | ✅ Complete | `plans/tui-runtime-daemon-phase-04-runtime-event-reducer-tui-adapter.md` |
| 5 | Session/View State Split | ✅ Complete | `plans/tui-runtime-daemon-phase-05-session-view-state-split.md` |
| 6 | Embedded Runtime Compatibility Closure + Pre-Daemon Readiness | ✅ Complete | `plans/tui-runtime-daemon-phase-06-pre-daemon-readiness.md` |
| 7 | Local Daemon MVP | ✅ Complete | `plans/tui-runtime-daemon-phase-07-local-daemon-mvp.md` |
| 8 | TUI Remote Attach Mode | ✅ Complete | `plans/tui-runtime-daemon-phase-08-tui-remote-attach.md` |
| 9 | CLI Headless and Daemon Cleanup | ✅ Complete | (absorbed into Phase 10 scope) |
| 10 | Multi-Session and Multi-Frontend Semantics | ✅ Complete | `plans/tui-runtime-daemon-phase-10-multi-session-multi-frontend.md` |
| 11 | Persistence, Artifacts, and Resumability | ✅ Complete | `plans/tui-runtime-daemon-phase-11-persistence-artifacts-resumability.md` |
| 12 | Transport APIs Beyond Local Socket | ✅ Complete | `plans/tui-runtime-daemon-phase-12-transport-apis.md` |
| 13 | Frontend Plugin and Component Model | ✅ Complete | `plans/tui-runtime-daemon-phase-13-frontend-plugin-component-model.md` |
| 14 | Final Cleanup, Dependency Hardening, and Release Readiness | ✅ Complete | `plans/tui-runtime-daemon-phase-14-final-cleanup-hardening.md` |
| CP | Security Corrective Pass | ✅ Complete | `plans/tui-runtime-daemon-security-corrective-pass.md` |
| RR | Release Readiness Verification | ✅ Complete | `plans/tui-runtime-daemon-release-readiness-verification.md` (+ `plans/tui-runtime-daemon-release-readiness-report.md`) |

## Phase Details

### Phase 0: Architecture Inventory

Documented current TUI coupling. Classified `App` state into frontend view state, frontend input state, runtime/session state, and engine execution. Produced `architecture/tui.md` as the boundary reference.

### Phase 1: Runtime DTO and Protocol Skeleton

Introduced `eggsec-runtime` as a workspace crate. Defined serializable DTOs: `SessionId`, `TaskId`, `RunRequest`, `TaskKind`, `TaskStatus`, `TaskProgress`, `TaskOutcome`, `RuntimeEvent`, `SessionSnapshot`, `RuntimeCapabilities`, `TaskResultEnvelope`, `ArtifactRef`. Crate builds without `eggsec-tui`. JSON round-trip tests exist.

**Key files:** `eggsec-runtime/src/event.rs`, `request.rs`, `session.rs`, `capabilities.rs`

### Phase 2: Task Lifecycle Extraction

Moved task spawning, handles, cancellation, timeout policy, progress/result channels, and active-task bookkeeping out of `App` into the runtime layer. Runtime owns `create_session`, `submit`, `cancel`, `snapshot`, `subscribe`.

**Key files:** `eggsec-runtime/src/runtime.rs` (Runtime, RuntimeConfig, SessionOptions)

### Phase 3: Worker Dispatch Migration

Moved execution dispatch from `eggsec-tui/src/workers` into `eggsec::dispatch`. Runtime translates neutral `TaskKind` into engine calls. TUI workers deleted. Feature-gated dispatch verified across 6 representative profiles.

**Key files:** `eggsec/src/dispatch/mod.rs`, `eggsec-tui/src/app/task_dispatcher.rs`

### Phase 4: Runtime Event Reducer and TUI Adapter

Replaced direct `App::handle_result` mutation with `TuiRuntimeAdapter` event reducer. Results associated with `TaskId` and session. Runtime events map to tab view updates through explicit adapter boundary.

**Key files:** `eggsec-tui/src/app/runtime_adapter/mod.rs`

### Phase 5: Session/View State Split

Split canonical runtime session state from local TUI view state. Runtime owns config snapshot, loaded scope, execution surface, task registry, audit stream. TUI owns tab, input mode, overlays, focus, theme, scroll offsets. Sessions constructible without Ratatui.

**Key files:** `eggsec-runtime/src/session.rs`, `eggsec-tui/src/app/state_update.rs`

### Phase 6: Embedded Runtime Compatibility Closure + Pre-Daemon Readiness

Two sub-phases:

- **6a (Closure):** Verified TUI behaves identically pre/post refactor. Added typed result bridge (`TaskOutcome::Result(TaskResultEnvelope)`), runtime capability truthfulness, 5 architecture guards, 9 adapter edge-case tests.
- **6b (Pre-Daemon Readiness):** Added 22 in-process client contract tests proving runtime works independently of TUI. Verified all 25 `TaskResult` variants have envelope mappings. Total runtime tests: 22 in-process + 64 across 3 suites. TUI adapter: 28 tests. Task dispatcher: 21 tests.

**Key files:** `crates/eggsec-runtime/tests/in_process_client.rs`, `crates/eggsec-tui/src/app/task_dispatcher.rs`, `crates/eggsec-tui/src/app/runtime_adapter/mod.rs`

### Phase 7: Local Daemon MVP

Added `eggsec-daemon` crate with local-only Unix socket transport. Daemon hosts `eggsec-runtime`. Supports session creation, attach/list, task submission, cancellation, event subscription, snapshot retrieval, capabilities, and health. Protocol version constant added.

**Key files:** `crates/eggsec-daemon/src/` (host.rs, protocol.rs, server.rs, client.rs)

### Phase 8: TUI Remote Attach Mode

TUI can connect to either embedded in-process runtime or local daemon. Runtime client abstraction with `embedded.rs` and `daemon.rs` backends. Embedded mode remains default. Same protocol commands/events for both modes.

**Key files:** `crates/eggsec-tui/src/runtime_client/` (embedded.rs, daemon.rs)

### Phase 9: CLI Headless and Daemon Cleanup

Refactored CLI packaging so headless builds (`--no-default-features`) don't need terminal dependencies. Added daemon-client CLI operations for listing sessions, attaching, submitting tasks, streaming events, cancelling. Feature split: `tui`, `daemon-client`, `headless` markers.

**Key files:** `crates/eggsec-cli/src/` (headless dispatch, daemon client commands)

### Phase 10: Multi-Session and Multi-Frontend Semantics

Defined client/observer/controller/owner/approver roles. `CommandPermission` enum for per-command RBAC. Session-scoped commands require declared client. Strict sessions restrict policy approval to Owner. Multiple clients can subscribe to one session. Stale clients don't block completion.

**Key files:** `crates/eggsec-daemon/src/client_registry.rs`

### Phase 11: Persistence, Artifacts, and Resumability

SQLite-backed session snapshots stored at lifecycle points. `DaemonStore` trait with `SqliteStore` implementation. Recovery on daemon startup via `recover_persisted_state()`. Artifact references fetchable through daemon APIs. All persistence operations fire-and-forget (best-effort).

**Key files:** `crates/eggsec-daemon/src/store.rs`, `crates/eggsec-daemon/src/http.rs`

### Phase 12: Transport APIs Beyond Local Socket

Added HTTP transport (feature-gated `http-api`). `axum`-based loopback server. `DaemonCapabilities` declares available transports. `DaemonRequestContext` carries `TransportKind` on every inbound command. Default: loopback `127.0.0.1:0`. Network-facing transport requires explicit opt-in.

**Key files:** `crates/eggsec-daemon/src/http.rs`

### Phase 13: Frontend Plugin and Component Model

Defined frontend-neutral view DTOs in `eggsec-ui-model`: `SessionSummaryView`, `SessionView`, `TaskView`, `TaskProgressView`, `ResultEnvelopeView`, `OutcomeView`, `ArtifactView`, `EventView`, `DashboardSummaryView`, `ClientRoleView`, `PolicyPromptView`. `ResultRendererDescriptor` registry for kind→renderer mapping.

**Key files:** `crates/eggsec-ui-model/src/`

### Phase 14: Final Cleanup, Dependency Hardening, and Release Readiness

Final audit across 8 workstreams: crate boundary enforcement, temporary bridge audit, feature/build matrix verification, daemon security review, documentation truth pass, API stability review, manual smoke test script, and completed plan index. Architecture guards verified. All docs updated to reflect implementation.

**Key files:** `scripts/check-architecture-guards.sh`, `architecture/tui.md`, `AGENTS.md`

### Security Corrective Pass (CP)

Addressed semantic security risks in daemon authorization:

1. Centralized `CommandPermission` enum replacing stringly-typed names
2. Actual `RuntimeSurface` used for authorization (not derived defaults)
3. `ApprovePolicy` returns `ErrorCode::Unsupported` (no silent no-op)
4. `CreateSession` classified as `DeclaredClient`
5. Strict surface policy approval restricted to session Owner only
6. New error codes: `ClientNotDeclared`, `Unsupported`, `InvalidState`
7. Comprehensive denial tests for observer/approver/strict-surface scenarios

**Key files:** `crates/eggsec-daemon/src/client_registry.rs`, `host.rs`, `protocol.rs`

## Known Deferred Items (Phase 14 Audit)

These items are intentionally deferred — not bugs, but recognized open edges for future work:

| Item | Location | Status |
|------|----------|--------|
| `ApprovePolicy` unwired (returns `Unsupported`) | `eggsec-daemon/src/client_registry.rs` | Explicit placeholder; wiring requires policy prompt UI |
| Packet sending stub | `eggsec/src/dispatch/network.rs:356` | Stub; real packet sending requires raw socket + scope |
| AI routes placeholder data | `eggsec/src/ai/` (~20 placeholders) | Placeholder data for AI analysis routes; functional but not production-hardened |
| `TODO(reframe-pass3)` defense-lab config profiles | `eggsec/src/config/` | Deferred config profile work for defense lab mode |
| Phase 5 async bridge fields in TUI | `eggsec-tui/src/app/` | Legacy async bridge fields retained for compatibility; not yet cleaned up |
| `result_rx`/`progress_rx` dual-channel architecture | `eggsec-tui/src/app/task_dispatcher.rs` | Typed `mpsc` channels for TUI rendering coexist with `TaskOutcome` envelope path; planned consolidation |
| `InterceptRule` legacy types in web-proxy | `eggsec-web-proxy/src/` | Legacy intercept rule types; functional but not yet unified with newer model |
| `frida_script` single-field compat in mobile-dynamic | `eggsec-mobile-lab/src/` | Single-field compatibility struct for frida script config |
| NSE library stubs | `eggsec-nse/src/` | Lua library stubs; functional but not all NSE libraries fully implemented |

## Key Types (Post-Phase 14 + CP)

- **`eggsec-runtime`**: `Runtime`, `RuntimeConfig`, `SessionOptions`, `RunRequest`, `TaskKind`, `TaskOutcome`, `RuntimeEvent`, `RuntimeCapabilities`, `TaskResultEnvelope`, `ArtifactRef`, `PolicyPrompt`, `RuntimeTaskExecutor`, `TaskDispatcher`
- **`eggsec-tui`**: `TuiTaskDispatcher`, `TuiExecutor`, `TuiRuntimeAdapter`, `EnforcementFacade`, `TuiEnforcementState`
- **`eggsec::dispatch`**: `dispatch_inner()`, `TaskResult` (~25 variants, some feature-gated)
- **`eggsec-daemon`**: `DaemonStore`, `SqliteStore`, `CommandPermission`, `DaemonRequestContext`, `TransportKind`, `DaemonCapabilities`, `HttpConfig`
- **`eggsec-ui-model`**: `SessionView`, `TaskView`, `ResultEnvelopeView`, `EventView`, `DashboardSummaryView`, `ResultRendererDescriptor`

## Validation Commands

```bash
cargo fmt --all --check
cargo clippy --lib -p eggsec
cargo test --lib -p eggsec
cargo test -p eggsec-daemon
cargo test -p eggsec-daemon --features http-api
cargo test -p eggsec-runtime
cargo test -p eggsec-tui
cargo test -p eggsec-ui-model
cargo check -p eggsec-cli --no-default-features --features daemon-client
bash scripts/check-architecture-guards.sh
bash scripts/smoke-daemon-local.sh
```

## Release Readiness Verification (Phase RR)

The release-readiness pass added:

- **`scripts/smoke-daemon-local.sh` rewrite** — uses `mktemp -d` for an ephemeral workspace, pre-builds binaries to avoid `cargo run` recompile warnings leaking into assertions, validates SIGTERM graceful shutdown, and exercises observer-deny + owner-allow posture in addition to the standard lifecycle.
- **`SIGTERM` handling in daemon main** — previously only `SIGINT` was handled; now `tokio::signal::unix::signal(SignalKind::terminate())` is installed alongside `ctrl_c()` so the daemon exits cleanly when sent SIGTERM. The server loop removes the socket file before returning.
- **`SqliteStore::migrate()` version-awareness** — newer-than-current schema versions are explicitly refused (returns `Err` from `SqliteStore::new`), preventing silent data corruption on downgrade. Older stored versions log a `warn!` and proceed.
- **`permission_denial_writes_audit_event_with_persistence` test** — closes the documented authorization-test gap (workstream 3 / item 6) by exercising a `RecordingAuditStore` and asserting that a denial produces a `command-denied:*` audit event with `client_id` and `session_id`.
- **Documentation updates** — README, AGENTS.md, architecture/daemon.md corrected for: `ServerMessage::Welcome` → `ServerMessage::Capabilities` (and `Health` carries `protocol_version`), schema migration is version-aware, session recovery clarifies that active tasks are not auto-resumed, smoke script reference added, daemon test count updated from 135 to 145, signal handling section added.

See `plans/tui-runtime-daemon-release-readiness-report.md` for blocker classification and final release recommendation.

## References

- Roadmap: `plans/tui-runtime-daemon-roadmap.md`
- Closure plan: `plans/tui-runtime-daemon-tightening-closure-plan.md`
- Security pass: `plans/tui-runtime-daemon-security-corrective-pass.md`
- Architecture: `architecture/tui.md`
- Guards: `scripts/check-architecture-guards.sh`
- In-process client tests: `crates/eggsec-runtime/tests/in_process_client.rs`
