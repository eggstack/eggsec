# TUI Runtime/Daemon Release-Readiness Verification Plan

## Purpose

This plan verifies the completed TUI/runtime/daemon architecture line before treating it as release-ready. The intent is not to add new architecture. The intent is to prove that the daemon stack, TUI attach mode, CLI daemon client, persistence layer, HTTP feature gate, UI model, documentation, and architecture guards are consistent and safe enough for handoff.

This should be run as a focused verification pass after phases 0-14 and the daemon authorization corrective pass.

## Current Baseline

The repo now appears to include:

- `eggsec-runtime` as the frontend-neutral lifecycle/session/event layer.
- `eggsec-daemon` as the daemon host, Unix socket server, optional HTTP/SSE transport, client registry, authorization layer, persistence store, and client library.
- `eggsec-cli` daemon commands for lifecycle, sessions, tasks, history, and persisted snapshots.
- `eggsec-tui` embedded and daemon runtime-client modes.
- `eggsec-ui-model` for frontend-neutral view DTOs and result rendering descriptors.
- SQLite-backed session snapshot persistence and audit-event storage.
- `scripts/smoke-daemon-local.sh` for manual daemon lifecycle validation.
- `DAEMON_PROTOCOL_VERSION = 1` exposed in daemon health/welcome paths.
- Architecture guards covering runtime/daemon/TUI/dependency boundaries.

## Verification Principles

1. Validate behavior through actual commands, not only unit tests.
2. Treat daemon authorization and local-only transport posture as release blockers.
3. Treat persistence failures as important operational behavior, not a secondary concern.
4. Verify feature-gated behavior explicitly.
5. Ensure docs match the implementation exactly.
6. Do not broaden transport exposure during this pass.

## Workstream 1: Full Build and Test Matrix

### Goal

Confirm every relevant crate and feature profile builds after the daemon, persistence, HTTP, and UI-model additions.

### Required checks

Run the project-standard matrix first:

```bash
cargo fmt --all --check
cargo check -p eggsec-runtime
cargo test -p eggsec-runtime
cargo check -p eggsec-daemon
cargo test -p eggsec-daemon
cargo check -p eggsec-daemon --features http-api
cargo test -p eggsec-daemon --features http-api
cargo check -p eggsec-ui-model
cargo test -p eggsec-ui-model
cargo check -p eggsec-cli --no-default-features
cargo check -p eggsec-cli --no-default-features --features daemon-client
cargo check -p eggsec-cli --features tui
cargo test -p eggsec-cli
cargo check -p eggsec-tui
cargo test -p eggsec-tui
cargo test --lib -p eggsec
./scripts/check-architecture-guards.sh
```

Run optional feature profiles where practical:

```bash
cargo check -p eggsec-tui --features stress-testing,packet-inspection
cargo check -p eggsec-tui --features nse
cargo check -p eggsec-tui --features db-pentest
cargo check -p eggsec-tui --features web-proxy
cargo check -p eggsec-tui --features wireless,wireless-advanced
cargo check -p eggsec-cli --features rest-api
```

If all-features is expected to work on the target platform, run:

```bash
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

If all-features is not expected to work because of system dependencies, document exact exclusions and required system packages.

### Acceptance criteria

- Default workspace validation passes.
- Daemon with and without `http-api` passes.
- CLI headless/daemon-client/TUI feature profiles pass.
- TUI feature profiles pass or have documented platform constraints.
- Architecture guards pass.

## Workstream 2: Local Daemon Smoke Script Verification

### Goal

Run and verify `scripts/smoke-daemon-local.sh` as the canonical manual lifecycle test.

### Steps

1. Inspect the script for destructive commands or non-local network assumptions.
2. Ensure it uses temporary directories and socket paths where possible.
3. Run the smoke script on a clean checkout.
4. Confirm it covers:
   - daemon start;
   - health/status;
   - client declaration;
   - session creation;
   - session listing;
   - task submission with safe/local task fixture;
   - event watch;
   - cancellation;
   - persisted history/show commands if persistence is enabled;
   - daemon shutdown;
   - cleanup.
5. Add missing steps if any critical lifecycle path is not covered.
6. Make the script idempotent: repeated runs should not fail because of stale sockets or previous data.

### Acceptance criteria

- Smoke script passes on a clean machine with documented prerequisites.
- Smoke script does not require public network exposure.
- Smoke script is safe to run locally.
- Smoke script produces actionable failure output.
- README or daemon docs reference the script.

## Workstream 3: Daemon Authorization Regression Review

### Goal

Reconfirm the daemon security corrective pass holds after phases 11-14.

### Checks

Verify through tests and, where possible, command/client calls:

- every `ClientCommand` maps to `CommandPermission`;
- every session-scoped command passes through central authorization;
- mutating commands require declared client identity;
- `ListSessions` and `CreateSession` behavior before declaration matches documented policy;
- observer cannot submit/cancel/close;
- controller cannot approve strict-surface policy unless allowed by owner-only rule;
- strict surfaces use actual stored/runtime surface;
- `ApprovePolicy` returns `Unsupported` or real pending-decision behavior, never placeholder success;
- permission denial writes audit event if persistence is enabled;
- `ClientNotDeclared`, `PermissionDenied`, `Unsupported`, and `InvalidState` are distinguishable in CLI JSON.

### Suggested tests to add if missing

```bash
cargo test -p eggsec-daemon command_permission
cargo test -p eggsec-daemon client_not_declared
cargo test -p eggsec-daemon observer_cannot
cargo test -p eggsec-daemon approve_policy
cargo test -p eggsec-daemon strict_surface
cargo test -p eggsec-daemon audit
```

### Acceptance criteria

- Denial cases are covered, not only success cases.
- `ApprovePolicy` cannot silently succeed.
- Strict surfaces cannot be downgraded by attach/default role behavior.
- Audit event behavior is verified for denials and mutations.

## Workstream 4: Persistence Failure and Recovery Behavior

### Goal

Verify persistence is operationally safe under success and failure modes.

### Success-path checks

1. Start daemon with persistence enabled and a temporary data directory.
2. Create a session.
3. Submit a safe task.
4. Verify persisted session appears in `daemon history`.
5. Verify `daemon show <session-id>` includes snapshot, surface, generation, task status, and outcome summary.
6. Restart daemon with same data dir.
7. Verify session is recovered.
8. Verify completed task outcome is retained.

### Active-task recovery checks

1. Start a long-running safe task.
2. Stop daemon before completion.
3. Restart daemon.
4. Verify previously active task is marked cancelled/interrupted/abandoned and is not automatically resumed.

### Failure-mode checks

Test behavior when:

- data directory is unwritable;
- SQLite file is locked or invalid;
- schema version is unknown/newer;
- persistence is disabled;
- artifact path is missing.

Expected behavior:

- daemon should degrade to `NoopStore` only if explicitly documented;
- degraded mode should be visible in logs/health/capabilities;
- security audit persistence failures should not be silent if audit is expected to be durable;
- daemon should not panic on corrupted persistence state.

### Acceptance criteria

- Persistence success path works across daemon restart.
- Active tasks are not auto-resumed.
- Persistence failure behavior is documented and observable.
- No panic on invalid/corrupt store.
- Health/capabilities expose persistence mode if practical.

## Workstream 5: HTTP/SSE Feature-Gate and Local-Only Verification

### Goal

Ensure HTTP/SSE transport is safe, feature-gated, and not accidentally enabled or public.

### Checks

Build without HTTP:

```bash
cargo check -p eggsec-daemon
```

Confirm HTTP dependencies are not required without `http-api`.

Build with HTTP:

```bash
cargo check -p eggsec-daemon --features http-api
cargo test -p eggsec-daemon --features http-api
```

Behavior checks:

- default bind is loopback only;
- public bind requires explicit config;
- public bind emits a clear warning;
- HTTP request handlers call the same `DaemonHost::handle_command` path;
- HTTP requests carry `TransportKind::LoopbackHttp` in `DaemonRequestContext`;
- client declaration is required before mutating routes;
- HTTP uses strict/noninteractive default posture;
- SSE event stream does not bypass session authorization.

### Acceptance criteria

- HTTP/SSE is off by default.
- HTTP deps are feature-gated.
- Public exposure cannot happen accidentally.
- HTTP authorization is identical to Unix socket authorization.
- SSE stream is session-filtered and permission-checked.

## Workstream 6: Dependency and Lockfile Audit

### Goal

Review dependency changes caused by SQLite, HTTP, and UI model additions.

### Checks

Run:

```bash
cargo tree -p eggsec-runtime
cargo tree -p eggsec-daemon
cargo tree -p eggsec-daemon --features http-api
cargo tree -p eggsec-ui-model
cargo tree -p eggsec-cli --no-default-features --features daemon-client
```

Verify:

- `eggsec-runtime` has no TUI, daemon transport, SQLite, HTTP, or UI-model dependencies.
- `eggsec-ui-model` depends only on `eggsec-runtime`, `serde`, and minimal serialization dependencies.
- `eggsec-daemon` owns SQLite and HTTP feature-gated dependencies.
- `eggsec-cli --no-default-features --features daemon-client` does not pull TUI deps.
- `eggsec-tui` may depend on UI model and daemon client intentionally.

Review lockfile churn:

- SQL-related version changes;
- duplicate dependency versions;
- unexpected downgrades;
- new transitive dependencies from `axum`/HTTP;
- `rusqlite`/`libsqlite3-sys` portability implications.

Optional if installed:

```bash
cargo deny check
cargo machete
cargo udeps
```

### Acceptance criteria

- Dependency boundaries match architecture.
- No unexpected heavy deps enter runtime/UI-model/headless CLI.
- Lockfile changes are understood.
- Any duplicate or downgraded dependencies are documented or fixed.

## Workstream 7: UI Model and Rendering Contract Verification

### Goal

Verify `eggsec-ui-model` is stable enough for TUI/CLI/web-facing view usage.

### Checks

- DTO serialization round-trips pass.
- Every known `TaskResultEnvelope.kind` has a `ResultRendererDescriptor`, or unknown handling is tested.
- CLI daemon human-readable output uses `SessionSummaryView`, `SessionView`, and `EventView` consistently.
- TUI daemon attach uses `OutcomeView`/renderer registry for envelope rendering.
- Unknown result kind renders safely with fallback label/summary.
- UI model does not expose local artifact paths as public remote URLs.

### Acceptance criteria

- View DTO tests pass.
- Renderer registry is complete enough for current result kinds.
- CLI and TUI output do not duplicate incompatible formatting logic for daemon state.
- Unknown/partial envelopes are safe and readable.

## Workstream 8: Protocol Compatibility Verification

### Goal

Confirm daemon protocol versioning and message compatibility behavior are explicit.

### Checks

- `DAEMON_PROTOCOL_VERSION` is exposed through health/welcome/capabilities as documented.
- Client checks protocol version before issuing commands, or docs explicitly state compatibility expectations.
- Every protocol message has serde round-trip tests.
- Error codes are stable and documented.
- Breaking-change rules are documented.

### Acceptance criteria

- Protocol version is visible to clients.
- Protocol DTO tests cover all variants.
- Docs explain when to bump the protocol version.

## Workstream 9: Documentation Truth and Release Notes

### Goal

Ensure docs describe implemented behavior, not roadmap intent.

### Required docs to check

- `README.md`
- `architecture/overview.md`
- `architecture/daemon.md`
- `architecture/tui.md`
- `architecture/cli_commands.md`
- `docs/CI_ARCHITECTURE_GUARDS.md`
- `docs/FEATURE_MATRIX.md`
- `docs/ENFORCEMENT_MODES.md`
- `AGENTS.md`
- `.opencode/skills/eggsec-cli/SKILL.md`
- `.opencode/skills/eggsec-tui/SKILL.md`
- `plans/tui-runtime-daemon-completed-phase-index.md`

Docs must state clearly:

- default daemon socket path;
- persistence data directory;
- HTTP/SSE feature flag and loopback-only default;
- `ApprovePolicy` unsupported status, if still true;
- CLI/TUI manual vs MCP/agent/CI strict behavior;
- daemon protocol version;
- validation commands;
- smoke script usage;
- known deferred items.

### Acceptance criteria

- Docs align with code.
- No stale command counts, path defaults, or feature names.
- Completed phase index includes phases 0-14 plus corrective/security/release-readiness status.

## Workstream 10: Release Blocker Classification

### Goal

End the verification pass with a concrete release decision.

Classify findings as:

- **Blocker**: release should not proceed.
- **High**: must be fixed before public announcement or broader usage.
- **Medium**: fix soon, but not a release blocker for local/manual use.
- **Low**: docs/polish.
- **Deferred**: consciously out of scope.

Likely blockers:

- daemon authorization bypass;
- HTTP public exposure by default;
- `ApprovePolicy` silent success;
- runtime depending on transport/TUI/persistence deps;
- smoke script unsafe/destructive behavior;
- persistence panic on normal failure;
- CLI headless build broken.

### Deliverable

Update or create a release readiness report:

```text
plans/tui-runtime-daemon-release-readiness-report.md
```

Include:

- validation commands run;
- pass/fail results;
- known failures;
- blocker table;
- deferred items;
- release recommendation.

## Final Acceptance Criteria

The repo is release-ready for the TUI/runtime/daemon architecture line when:

- build/test/feature matrix passes or documented exclusions are justified;
- smoke daemon script passes;
- daemon authorization denial cases pass;
- persistence recovery and failure modes are verified;
- HTTP/SSE remains feature-gated and local-only by default;
- dependency boundaries are clean;
- UI model rendering contract is tested;
- protocol versioning is visible and documented;
- docs match implementation;
- release readiness report has no blockers.

Only after this verification should future work branch into remote authentication, full policy approval lifecycle, richer web/desktop frontends, or additional transports.
