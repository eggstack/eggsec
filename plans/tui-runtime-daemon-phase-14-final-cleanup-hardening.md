# Phase 14 Plan: Final Cleanup, Dependency Hardening, and Release Readiness

## Goal

Complete the TUI/runtime/daemon architecture line of work with a final cleanup and hardening pass. This phase should leave the repo with clear crate boundaries, truthful docs, stable validation commands, no temporary daemon/TUI bridge leaks, and a release-ready local daemon story.

This is a closure phase. Do not add broad new features unless they fix correctness, safety, or maintainability issues found during the audit.

## Current Baseline

By this point the repo should have:

- runtime/task/session lifecycle extracted from TUI;
- TUI embedded and daemon runtime clients;
- local daemon host and client library;
- CLI daemon client commands;
- multi-session/multi-frontend permission semantics;
- daemon authorization hardening;
- persistence/artifact/resumability if Phase 11 landed;
- transport APIs if Phase 12 landed;
- frontend view/component model if Phase 13 landed.

Phase 14 verifies the whole stack and removes ambiguity.

## Workstream 1: Crate Boundary Audit

Audit workspace dependencies and enforce intended direction.

Expected boundaries:

- `eggsec-runtime`: lifecycle/protocol DTOs only; no TUI, daemon transport, engine-heavy, or persistence dependencies unless intentionally accepted.
- `eggsec-daemon`: transport, protocol host, client registry, authorization, persistence/store; no TUI dependency.
- `eggsec-tui`: frontend rendering/input; no canonical task execution dispatch; no daemon authorization logic beyond client role display.
- `eggsec-cli`: direct CLI and daemon client; TUI dependency feature-gated.
- `eggsec`: engine and dispatch; owns task execution, not UI lifecycle.

Add or update architecture guards for every invariant.

## Workstream 2: Temporary Bridge Audit

Search for temporary compatibility names and decide whether each is still valid.

Search terms:

```text
compat
bridge
TODO
FIXME
stub
placeholder
tui-compat
Unsupported
ApprovePolicy
result_rx
progress_rx
TaskOutcome::Empty
```

For each finding:

- remove if obsolete;
- document if intentional;
- add issue/plan entry if deferred;
- add guard if it must not spread.

Special focus:

- typed result channel path vs result envelope path;
- `ApprovePolicy` unsupported behavior;
- daemon local-only transport warnings;
- session attach role defaults;
- plan file deletion/archive convention.

## Workstream 3: Feature Matrix and Build Matrix

Verify feature combinations after daemon/client/TUI split.

Required checks:

```bash
cargo check -p eggsec-cli --no-default-features
cargo check -p eggsec-cli --no-default-features --features daemon-client
cargo check -p eggsec-cli --features tui
cargo check -p eggsec-daemon
cargo test -p eggsec-daemon
cargo check -p eggsec-tui
cargo test -p eggsec-tui
cargo check -p eggsec-runtime
cargo test -p eggsec-runtime
```

Representative optional feature checks:

```bash
cargo check -p eggsec-tui --features stress-testing,packet-inspection
cargo check -p eggsec-tui --features nse
cargo check -p eggsec-tui --features db-pentest
cargo check -p eggsec-tui --features web-proxy
cargo check -p eggsec-tui --features wireless,wireless-advanced
cargo check -p eggsec-cli --features rest-api
```

Update CI and docs so the expected matrix is reproducible.

## Workstream 4: Daemon Security Review

Perform a final focused review of daemon safety.

Checklist:

- All mutating commands require declared client.
- All session-scoped commands pass through `CommandPermission`.
- `ApprovePolicy` cannot silently succeed.
- Strict surfaces do not accept unrelated manual approvals.
- Actual session surface is used for authorization.
- Observer/controller/owner/approver semantics are tested.
- Local socket default is safe.
- Public bind requires explicit opt-in if transport APIs exist.
- Audit events exist for permission denials and mutations if persistence landed.
- Error codes are structured and documented.

## Workstream 5: Documentation Truth Pass

Update docs to reflect actual implementation, not roadmap intent.

Required docs:

- `README.md`
- `architecture/overview.md`
- `architecture/tui.md`
- `architecture/cli_commands.md`
- `docs/CI_ARCHITECTURE_GUARDS.md`
- `docs/FEATURE_MATRIX.md`
- `AGENTS.md`
- `.opencode/skills/eggsec-cli/SKILL.md`
- `.opencode/skills/eggsec-tui/SKILL.md`
- `plans/tui-runtime-daemon-completed-phase-index.md`

Docs must clearly state:

- what daemon transports exist;
- default local-only behavior;
- CLI/TUI manual vs agent/MCP/CI strict semantics;
- which result rendering paths are complete;
- which policy approval behavior is supported/unsupported;
- how to run validation.

## Workstream 6: API Stability Review

Review public/protocol DTOs for naming, versioning, and forward compatibility.

Targets:

- `ClientCommand`
- `ServerMessage`
- `ErrorCode`
- `RuntimeEvent`
- `SessionSnapshot`
- `TaskResultEnvelope`
- `ArtifactRef`
- `ClientKind`
- `ClientRole`
- `CommandPermission`

Add protocol version metadata if missing.

Suggested:

```rust
pub const DAEMON_PROTOCOL_VERSION: u32 = 1;
```

Expose in health/capabilities.

## Workstream 7: Manual Smoke Test Script

Add a script or documented smoke flow for local daemon mode.

Suggested smoke:

```text
1. Start daemon.
2. Check health.
3. Declare CLI client.
4. Create session.
5. List sessions.
6. Submit safe task.
7. Watch events.
8. Cancel running task.
9. Attach TUI to daemon session.
10. Restart daemon if persistence exists and verify recovered session.
```

If possible, add `scripts/smoke-daemon-local.sh` with safe no-network or localhost-only task fixtures.

## Workstream 8: Completed Plan Index

Finalize `plans/tui-runtime-daemon-completed-phase-index.md`.

Include:

- Phase 0-14 summary;
- plan file path;
- implementation commit(s);
- corrective commit(s);
- validation status;
- known deferred items.

This index should be the handoff map for future work.

## Non-Goals

Do not add major new transports.

Do not add dynamic plugins.

Do not change enforcement philosophy.

Do not remove manual CLI/TUI discretion.

Do not promote daemon remote/public use unless the security model is complete.

## Validation

Run full local validation:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
./scripts/check-architecture-guards.sh
```

If all-features is not practical because of platform deps, document exclusions and run the project-standard matrix.

## Acceptance Criteria

- Crate boundaries match documented architecture.
- No obsolete temporary bridges remain undocumented.
- Feature/build matrix is reproducible.
- Daemon security invariants are tested and documented.
- Public/protocol DTOs have stable names and versioning plan.
- Docs match implementation.
- Completed phase index is current.
- Local daemon smoke flow exists.
- Repo is ready for future feature work outside this roadmap.
