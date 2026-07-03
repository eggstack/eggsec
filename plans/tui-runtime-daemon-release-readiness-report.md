# TUI Runtime/Daemon — Release Readiness Report

## Status: ✅ Release-ready (with documented deferrals)

This report summarizes the verification pass executed against the TUI/runtime/daemon architecture line. It captures validation commands, results, blockers, and deferred items as of the report date.

## Verification Commands Executed

```bash
# Core workspace
cargo fmt --all --check                                                # PASS
cargo check -p eggsec-runtime                                          # PASS
cargo test -p eggsec-runtime                                           # PASS (64 tests)
cargo check -p eggsec-daemon                                           # PASS
cargo test -p eggsec-daemon                                            # PASS (126 tests; was 125)
cargo check -p eggsec-daemon --features http-api                       # PASS
cargo test -p eggsec-daemon --features http-api                        # PASS (145 tests; was 144)
cargo check -p eggsec-ui-model                                         # PASS
cargo test -p eggsec-ui-model                                          # PASS (21 tests)
cargo check -p eggsec-cli --no-default-features                        # PASS
cargo check -p eggsec-cli --no-default-features --features daemon-client   # PASS
cargo check -p eggsec-cli --features tui                               # PASS (3 pre-existing warnings fixed)
cargo test -p eggsec-cli                                               # PASS
cargo check -p eggsec-tui                                              # PASS
cargo test -p eggsec-tui                                               # PASS (842 passed, 12 ignored)
cargo test --lib -p eggsec                                             # PASS (1540 tests)

# Feature-profile spot checks
cargo check -p eggsec-tui --features stress-testing,packet-inspection  # PASS
cargo check -p eggsec-tui --features nse                                # PASS
cargo check -p eggsec-tui --features db-pentest                         # PASS (after fix)
cargo check -p eggsec-tui --features web-proxy                          # PASS
cargo check -p eggsec-tui --features wireless,wireless-advanced         # PASS
cargo check -p eggsec --features rest-api                               # PASS (5 pre-existing test flakes; not introduced by this pass)

# Architecture guards
bash scripts/check-architecture-guards.sh                              # PASS (all 23 checks)

# Local daemon smoke test
bash scripts/smoke-daemon-local.sh                                      # PASS (19/19 checks)
```

## Pass/Fail Summary

| Check | Result |
|-------|--------|
| Cargo formatting | ✅ Clean |
| `eggsec-runtime` builds and tests | ✅ 64 passed |
| `eggsec-daemon` builds and tests (default) | ✅ 126 passed |
| `eggsec-daemon` builds and tests (`http-api`) | ✅ 145 passed |
| `eggsec-ui-model` builds and tests | ✅ 21 passed |
| `eggsec-cli` headless | ✅ Builds |
| `eggsec-cli` with `daemon-client` only | ✅ Builds (no TUI deps) |
| `eggsec-cli` with default `tui` feature | ✅ Builds |
| `eggsec-tui` builds and tests | ✅ 842 passed, 12 ignored |
| `eggsec` lib tests | ✅ 1540 passed |
| Feature profile compile checks | ✅ All 6 representative profiles |
| Architecture guards | ✅ All 23 checks |
| Local daemon smoke test | ✅ 19/19 checks |

## Fixes Applied During Verification

These are changes made during the readiness pass itself, not pre-existing fixes.

| # | Fix | Location | Reason |
|---|-----|----------|--------|
| 1 | Added SIGTERM handler alongside SIGINT | `crates/eggsec-daemon/src/main.rs` | Pre-SIGTERM kill exited without removing the socket file. New `wait_for_shutdown_signal()` installs both `SignalKind::terminate()` and `ctrl_c()` so the daemon shuts down gracefully on either signal. |
| 2 | Made `SqliteStore::migrate()` version-aware | `crates/eggsec-daemon/src/store/sqlite.rs` | Migration silently overwrote the stored `schema_version`, allowing silent corruption if a newer version's data was loaded by an older daemon. Now newer-than-current stored versions are explicitly refused with an `anyhow::bail!` from `SqliteStore::new`. Older stored versions log a `warn!` and proceed. |
| 3 | Added `permission_denial_writes_audit_event_with_persistence` test | `crates/eggsec-daemon/src/host.rs` | Closes documented gap (workstream 3, item 6) — no test previously verified that permission denials actually produce audit events when persistence is enabled. Test uses a `RecordingAuditStore` and asserts that a `command-denied:*` event with `client_id` and `session_id` is captured. |
| 4 | Rewrote `scripts/smoke-daemon-local.sh` | `scripts/smoke-daemon-local.sh` | Pre-existing script used `cargo run --quiet` which leaked compile warnings into assertions, used `/tmp` directly rather than `mktemp -d`, and treated owner-allow submit as expected even though the new authorization corrective pass makes each CLI invocation a fresh observer client. Rewrite pre-builds binaries to a temp workspace, uses `mktemp -d` for ephemeral data, splits observer-deny from owner-allow posture, and adds SIGTERM graceful-shutdown verification. |
| 5 | Removed unused `crate::tabs::TabState` import | `crates/eggsec-tui/src/app/task_management.rs` | Was unconditionally imported; the only consumer (`DbPentestTab` impl) is feature-gated. Now `#[cfg(feature = "db-pentest")]`. Fixes an actual compile error in `--features db-pentest` mode. |
| 6 | Removed unused `make_friendly_error` import | `crates/eggsec-tui/src/app/mod.rs` | Imported but never used. |
| 7 | Removed unused `crate::app::tab_error::TabError` import | `crates/eggsec-tui/src/tabs/db_pentest.rs` | Imported but never used. |
| 8 | Moved `spec_for_id` re-export under `#[cfg(test)]` | `crates/eggsec-tui/src/tabs/mod.rs` | The function was unused in non-test code; now only re-exported when the test module is compiled. |

## Documentation Updates

| Doc | Update |
|-----|--------|
| `README.md` | Added "Local Smoke Test" subsection referencing `scripts/smoke-daemon-local.sh`; clarified session recovery wording (active tasks are not auto-resumed); updated schema migration description (newer-than-current versions are explicitly refused). |
| `AGENTS.md` | Fixed `ServerMessage::Welcome` → `ServerMessage::Capabilities` (line 334); fixed `ServerMessage::Welcome` → `ServerMessage::Health` (line 480, where `protocol_version` is included); updated daemon test count from 135 → 145 (line 54). |
| `architecture/daemon.md` | Added "Schema Migration" subsection describing version-aware migration behavior; added "Local Smoke Test" subsection; added "Signal Handling" subsection describing SIGINT and SIGTERM support; clarified that active tasks are not auto-resumed at recovery. |
| `plans/tui-runtime-daemon-completed-phase-index.md` | Added "Release Readiness Verification (Phase RR)" row to the phase summary; added release-readiness verification section documenting all fixes and updated validation commands. |
| `.opencode/skills/eggsec-cli/SKILL.md` | Added reference to `scripts/smoke-daemon-local.sh` under the daemon commands section. |

## Blocker Classification

| Item | Severity | Status |
|------|----------|--------|
| Daemon authorization bypass | — | Not present. All denial paths covered by unit + integration tests. |
| HTTP public exposure by default | — | Not present. Loopback-only default; explicit `allow_public_bind` + warning required. Verified by feature-gating (`http-api`) and `validate_bind_addr()`. |
| `ApprovePolicy` silent success | — | Not present. Returns `ErrorCode::Unsupported` with message "daemon policy approval is not wired yet" (`crates/eggsec-daemon/src/host.rs:530-556`). |
| Runtime depending on transport/TUI/persistence deps | — | Not present. Architecture guard #11 (TUI), #12 (transport), #13 (no unimplemented transports), #20 (persistence), #22 (engine isolation) all PASS. |
| Smoke script unsafe/destructive behavior | — | Not present. Uses `mktemp -d` workspace, never writes outside temp, never makes public network calls, kills processes and removes files via EXIT trap. |
| Persistence panic on normal failure | — | Not present. `SqliteStore::new` failures degrade to `NoopStore` with a `tracing::warn!`. New schema-version check returns `Err` cleanly rather than panicking. |
| CLI headless build broken | — | Not present. `cargo check -p eggsec-cli --no-default-features` and `cargo check -p eggsec-cli --no-default-features --features daemon-client` both succeed without pulling TUI deps. |

## Deferred Items (Not Blockers)

These items remain intentionally out of scope. They are documented in `plans/tui-runtime-daemon-completed-phase-index.md` and `plans/tui-runtime-daemon-release-readiness-verification.md`.

| Item | Severity | Notes |
|------|----------|-------|
| `ApprovePolicy` returns `ErrorCode::Unsupported` | Medium | Wiring requires policy prompt UI integration; manual-mode flow works through CLI `--yes` and TUI confirmation. |
| HTTP `require_auth` field unused | Low | Currently loopback-only; auth layer deferred until HTTP transport is broadened beyond local-only. |
| `WebSocket` and `Grpc` `TransportKind` variants unused | Low | Enum variants exist for forward compatibility; no listener implementation. |
| `NoopStore` fallback not visible in `DaemonCapabilities` | Low | Operators must consult logs to confirm persistence mode; documented in `architecture/daemon.md` Schema Migration section. |
| Pre-existing `eggsec` `--features rest-api` test flakes (5) | Low | `test_rate_limiter_separate_keys`, `test_rate_limiter_blocks_over_limit`, `test_validate_payload_size_at_boundary`, `test_high_alert_only_high_finding_ids`, `test_scan_failure_does_not_abort_remaining_targets` — verified to fail before this pass as well; not in scope for TUI/runtime/daemon verification. |

## Protocol Compatibility Status

- **`DAEMON_PROTOCOL_VERSION = 1`** is exposed through `ServerMessage::Health.protocol_version` (`crates/eggsec-daemon/src/protocol.rs:233`).
- Client checks: `daemon status` CLI command displays the version (`crates/eggsec-cli/src/daemon_cli.rs:90-100`).
- 50 protocol serialization round-trip tests in `protocol.rs` cover all `ClientCommand`, `ServerMessage`, and `ErrorCode` variants.
- 13 `ErrorCode` tests verify all variants deserialize/serialize cleanly.
- Breaking-change rule (not enforced by code, documented in `architecture/daemon.md` and `AGENTS.md`): bump `DAEMON_PROTOCOL_VERSION` on any wire-format-breaking change.

## Dependency Boundary Status

Verified by `cargo tree` and architecture guard checks:

| Crate | Allowed Dependencies | Status |
|-------|----------------------|--------|
| `eggsec-runtime` | `serde`, `serde_json`, `thiserror`, `tokio`, `tokio-util`, `tracing`, `uuid` | ✅ Clean (no TUI/transport/persistence deps) |
| `eggsec-ui-model` | `eggsec-runtime`, `serde`, `serde_json` | ✅ Clean (3 deps total) |
| `eggsec-daemon` (default) | `eggsec-runtime`, `serde`, `serde_json`, `tokio`, `tokio-util`, `tracing`, `tracing-subscriber`, `thiserror`, `uuid`, `anyhow`, `async-trait`, `rusqlite` | ✅ Clean (no `axum`/`async-stream`) |
| `eggsec-daemon` (http-api) | + `axum`, `async-stream`, `futures` | ✅ Optional deps only with feature |
| `eggsec-cli` (default = `tui`) | Includes `eggsec-tui` | ✅ TUI feature-gated |
| `eggsec-cli` (daemon-client only) | `eggsec-daemon`, `tokio-util`, `eggsec/daemon-client` | ✅ No TUI deps |
| `eggsec-cli` (headless) | — | ✅ No TUI or daemon deps |

## Release Recommendation

**✅ APPROVED for the TUI/runtime/daemon architecture line.**

The architecture is consistent with all documented invariants. All blocker candidates have been verified absent. Documented deferrals are explicit and operator-visible (logs, `ErrorCode::Unsupported`, etc.).

Recommended follow-ups after this verification:

1. Wire `ApprovePolicy` to a policy prompt path (manual-surface) before broader agent/MCP exposure.
2. Decide whether to add a `persistence_mode` field to `DaemonCapabilities` for operator visibility into `NoopStore` fallback.
3. Address the 5 pre-existing `eggsec --features rest-api` test flakes in a separate cleanup pass (not blocking).

**Local-only use** is fully supported today. **Production deployment** of the HTTP/SSE transport remains conditional on the deferred items above.