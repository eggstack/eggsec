# TUI Runtime/Daemon Security Corrective Pass Plan

## Purpose

This plan is a focused corrective/security pass after the Phase 6-10 daemon implementation. The repo now has the right broad structure: `eggsec-runtime` owns lifecycle, `eggsec-daemon` owns local transport/client registry/session access, `eggsec-tui` can attach through embedded or daemon runtime clients, and `eggsec-cli` can act as a daemon client.

The remaining risk is semantic, not structural. Before adding broader transports or richer frontend features, tighten daemon authorization, policy approval semantics, session-surface truth, client attribution, and auditability.

## Current State Summary

Recent commits appear to have added:

- `crates/eggsec-daemon` with protocol, client, server, host, config, error, and client registry modules.
- `ClientKind`, `ClientRole`, `ClientInfo`, `ClientRegistry`, and `SessionAccess`.
- `DeclareClient`, `CloseSession`, permission error codes, and role-based authorization.
- `TuiRuntimeClient` embedded/daemon backends and TUI daemon attach.
- CLI daemon subcommands and daemon-client wiring.
- Runtime snapshot generation and completed task outcome retention.
- A corrective commit that moved permission checks before command destructuring inside daemon `handle_command` and expanded daemon tests.

The next pass should assume the broad direction is correct, but should audit every command path for authorization, surface correctness, policy handling, and test coverage.

## Primary Risks

### 1. Policy approval placeholder behavior

If `ApprovePolicy` currently returns `Ok` without being connected to real pending policy state, it must be fixed. A no-op success is unsafe because clients can believe an approval happened when it did not, and future code may accidentally treat this as valid authorization.

### 2. Session surface derivation inside permission checks

Permission checks must use the actual runtime session surface, not a proxy such as `default_controller_allowed`. Strict/agent/MCP/CI sessions must never be treated as TUI/manual sessions because of access defaults.

### 3. Client declaration and command attribution

Every mutating command should be attributable to a declared client. Undeclared clients should be allowed only for explicitly safe commands such as health/capabilities, and maybe list-sessions if local policy allows it.

### 4. Observer/controller/approver semantics need hard tests

The permission model should have denial tests, not only success tests. Especially test observer denial, unrelated TUI denial, strict-surface approval denial, undeclared-client denial, and ghost-session behavior.

### 5. Completed plan file retention

The Phase 9 plan file appears to have been removed after implementation. If plan files are intentionally archived into an index, document that convention. Otherwise restore completed plan files to preserve handoff/audit history.

## Non-Goals

Do not add WebSocket, REST, SSE, gRPC, or remote network exposure.

Do not add authentication/user accounts.

Do not implement durable persistence.

Do not redesign the daemon protocol broadly unless needed to correct authorization semantics.

Do not loosen manual-mode discretion for CLI/TUI. The distinction remains: manual CLI/TUI is user-discretion-oriented; MCP/agent/CI/programmatic surfaces are strict.

## Workstream 1: Centralize Command Authorization

### Goal

Make every daemon command pass through a single authorization decision point before it mutates runtime or session state.

### Current concern

A corrective commit moved permission checks before command destructuring. Preserve that direction and make it impossible to add future mutating command arms without explicit permission mapping.

### Implementation steps

1. Review `crates/eggsec-daemon/src/host.rs`.
2. Ensure `handle_command` calls a central authorization function before all session-scoped command execution.
3. Replace stringly permission names if practical with an enum:

```rust
pub enum CommandPermission {
    Health,
    Capabilities,
    DeclareClient,
    ListSessions,
    CreateSession,
    GetSnapshot,
    SubmitTask,
    CancelTask,
    CancelActive,
    Subscribe,
    CloseSession,
    ApprovePolicy,
}
```

4. Add `impl From<&ClientCommand> for CommandPermission` or a total mapping method.
5. Add a compile/test guard that every `ClientCommand` variant maps to a permission.
6. Ensure commands are classified as:
   - public local-safe: health, capabilities;
   - declared-client-safe: list/create depending on local policy;
   - observer: get snapshot, subscribe;
   - controller: submit/cancel;
   - owner/controller: close session;
   - approver: approve policy.
7. Remove duplicate per-arm authorization if it risks drift, but keep defense-in-depth assertions where useful.

### Acceptance criteria

- Every `ClientCommand` maps to a permission enum.
- Every session-scoped command is authorized before execution.
- Adding a new command without authorization mapping fails tests or compile checks.
- Mutating commands from undeclared clients are denied.

## Workstream 2: Use Actual Runtime Session Surface for Authorization

### Goal

Permission checks must evaluate the real session surface bound in runtime/session state.

### Current concern

If `check_command_permission` infers surface from access defaults, that is incorrect. Surface must come from the actual `RuntimeSession`/`SessionSnapshot`/session metadata created at `CreateSession`.

### Implementation steps

1. Add a daemon-host helper:

```rust
fn session_surface(&self, session_id: &SessionId) -> Option<RuntimeSurface>
```

It should read from runtime session metadata or a daemon-side mirror populated directly from `CreateSession` surface.

2. Avoid deriving surface from `SessionAccess.default_controller_allowed` or similar access policy fields.
3. Store immutable session metadata in `SessionAccess` if runtime does not expose a cheap surface query:

```rust
pub struct SessionAccess {
    pub owner_client_id: Option<ClientId>,
    pub surface: RuntimeSurface,
    pub scope: Option<SessionScope>,
    ...
}
```

4. Add tests for every important surface:
   - `TuiManual`
   - `CliManual`
   - `McpServer`
   - `Agent`
   - `Ci`
   - strict/manual variants that exist in the codebase
5. Ensure policy/manual affordances are allowed only for manual surfaces.

### Acceptance criteria

- Authorization checks use real session surface.
- Strict/programmatic surfaces cannot be accidentally treated as manual.
- Tests prove `McpServer`/agent/CI sessions reject manual approval paths.

## Workstream 3: Fix Policy Approval Semantics

### Goal

`ApprovePolicy` must either perform a real pending-policy decision or fail explicitly with a structured unsupported/invalid-state error. It must not return success as a placeholder.

### Required behavior

Define the daemon policy approval lifecycle:

1. A task or command reaches a policy decision point.
2. Runtime/daemon emits `PolicyDecisionRequired` with:
   - session ID;
   - task ID or operation ID;
   - operation metadata;
   - required approval level;
   - expiration/deadline if applicable;
   - allowed approver role/surface.
3. An authorized approver sends `ApprovePolicy` or denial.
4. Runtime records the decision and resumes/fails the pending operation.
5. Audit record is emitted or stored.

If this is too large for the corrective pass, use explicit rejection:

```text
ErrorCode::Unsupported
message: "daemon policy approval is not wired yet"
```

or:

```text
ErrorCode::InvalidState
message: "no pending policy decision for task"
```

### Implementation steps

1. Inspect all `PolicyDecisionRequired`, enforcement approval, manual confirmation, and TUI preflight code paths.
2. Determine whether daemon approval can be wired cleanly now.
3. If not, change `ApprovePolicy` to return `InvalidState` or `Unsupported` unless a real pending decision exists.
4. Add `PendingPolicyDecision` DTO if implementing actual approval:

```rust
pub struct PendingPolicyDecision {
    pub decision_id: String,
    pub session_id: SessionId,
    pub task_id: Option<TaskId>,
    pub operation: String,
    pub required_surface: RuntimeSurface,
    pub created_at_secs: u64,
    pub expires_at_secs: Option<u64>,
}
```

5. Add daemon host storage for pending decisions only if needed.
6. Ensure approval requires `ClientRole::Approver` or `Owner` on manual sessions.
7. Ensure strict/programmatic sessions reject manual approval from TUI/CLI clients.
8. Add audit output for approvals and denials.

### Tests

- `approve_policy_without_pending_decision_returns_invalid_state`.
- `observer_cannot_approve_policy`.
- `unrelated_tui_cannot_approve_strict_session_policy`.
- `owner_can_approve_tui_manual_pending_decision` if real approval is implemented.
- `approval_records_client_id_surface_task_id_reason` if audit storage is implemented.

### Acceptance criteria

- `ApprovePolicy` never returns success as a placeholder.
- Unsupported/pending-missing states return explicit errors.
- Strict surfaces cannot receive manual approvals from daemon clients.
- Policy decisions are auditable when implemented.

## Workstream 4: Client Declaration and Attribution Hardening

### Goal

Mutating daemon commands should be attributable to a declared client ID, client kind, and session role.

### Implementation steps

1. Define which commands are allowed before `DeclareClient`.
2. Recommended allowed-before-declare commands:
   - `Health`
   - `Capabilities`
   - `DeclareClient`
3. Decide whether `ListSessions` is allowed before declaration. For stricter semantics, require declaration.
4. Deny session mutation without client ID:
   - `CreateSession` may either require declared client or create anonymous owner only if explicitly documented. Prefer requiring declaration.
   - `SubmitTask`, `CancelTask`, `CancelActive`, `CloseSession`, `ApprovePolicy` must require declared client.
5. Ensure server connection state stores the declared client ID and passes it into `handle_command` for every command.
6. Ensure CLI/TUI daemon clients fail loudly if declaration fails instead of ignoring the declaration result.
7. Add tests for undeclared clients.

### Acceptance criteria

- Mutating commands require declared client identity.
- CLI/TUI do not ignore failed `DeclareClient` responses.
- Audit/log fields include client ID and kind for mutations.

## Workstream 5: Session Access Semantics Review

### Goal

Make observer/controller/owner/approver behavior explicit and test-protected.

### Decisions to make

1. Should default observers be allowed for all local sessions?
2. Should default controllers be allowed for all local sessions?
3. Should session creator always be owner?
4. Can a second TUI attach as controller by default, or observer only?
5. Can CLI attach as controller by default?
6. Are strict sessions visible in `ListSessions`, and if so, what is redacted?

### Recommended default

- Session creator: owner + controller + approver only for manual surfaces.
- Additional TUI attach: observer by default unless explicit `--control` or local config grants controller.
- CLI attach: observer by default unless command explicitly creates/owns the session or uses `--control`.
- MCP/Agent/CI sessions: strict; no unrelated manual approver.

### Implementation steps

1. Add explicit attach mode to protocol if missing:

```rust
AttachMode::Observer
AttachMode::Controller
```

2. Make `Subscribe` observer-safe.
3. Make `SubmitTask`/`Cancel*` controller-only.
4. Make `ApprovePolicy` approver-only and manual-surface-limited.
5. Add `SessionAccess` unit tests for role resolution.
6. Add daemon host tests for default attach roles.

### Acceptance criteria

- Observer cannot mutate.
- Controller can submit/cancel but may not approve strict/manual policy unless also approver.
- Owner has expected rights.
- Approver rights are limited by surface semantics.

## Workstream 6: Error Code and Protocol Consistency

### Goal

Make client-visible failures deterministic and machine-readable.

### Required error codes

Ensure protocol has clear codes for:

- `PermissionDenied`
- `ClientNotDeclared`
- `SessionNotFound`
- `TaskNotFound`
- `InvalidSurface`
- `InvalidState`
- `Unsupported`
- `BadRequest`
- `Internal`

### Implementation steps

1. Review `crates/eggsec-daemon/src/protocol.rs` and `error.rs`.
2. Replace string-matching permission errors with structured error variants.
3. Ensure missing session returns `SessionNotFound`, not `PermissionDenied`, when appropriate.
4. Ensure unauthorized existing session returns `PermissionDenied`.
5. Ensure unsupported policy approval returns `Unsupported` or `InvalidState`.
6. Update `DaemonClient` to expose structured errors where possible.
7. Update CLI JSON output to include exact error code.

### Acceptance criteria

- Tests assert exact error codes, not just failure.
- CLI `--json` exposes error code and message.
- TUI can distinguish permission denial from disconnect/internal failure.

## Workstream 7: Local Socket Security Baseline

### Goal

The daemon remains local-only and does not accidentally expose unsafe surfaces.

### Implementation steps

1. Verify Unix socket binding behavior and default path.
2. Ensure stale socket cleanup is safe and does not unlink arbitrary paths.
3. Set restrictive permissions for socket path where supported.
4. If loopback TCP fallback exists, require explicit opt-in and warn clearly.
5. Add tests for invalid socket path handling if practical.
6. Document local-only support and non-goal of public remote exposure.

### Acceptance criteria

- Default daemon transport is local-only.
- Socket cleanup is path-safe.
- Runtime crate remains transport-free.
- Public network exposure is not added in this pass.

## Workstream 8: Plan/Audit Trail Cleanup

### Goal

Preserve handoff history and make completed phase status clear.

### Implementation steps

1. Inspect `plans/` for removed phase files, especially Phase 9.
2. Either restore removed plan files or document a clear convention in `plans/tui-runtime-daemon-completed-phase-index.md`.
3. Add the corrective pass plan entry to the completed phase index once implemented.
4. Include commit SHAs for phases 6-10 and corrective pass when known.

### Acceptance criteria

- Future reviewers can trace phase plan -> implementation commit -> corrective commit.
- No completed handoff plan disappears without index documentation.

## Suggested Validation Commands

Run:

```bash
cargo fmt --all --check
cargo check -p eggsec-runtime
cargo test -p eggsec-runtime
cargo check -p eggsec-daemon
cargo test -p eggsec-daemon
cargo check -p eggsec-tui
cargo test -p eggsec-tui
cargo check -p eggsec-cli
cargo test -p eggsec-cli
./scripts/check-architecture-guards.sh
```

Add focused daemon tests if they do not already exist:

```bash
cargo test -p eggsec-daemon permission
cargo test -p eggsec-daemon approve_policy
cargo test -p eggsec-daemon client_not_declared
cargo test -p eggsec-daemon strict_surface
```

Feature/build checks:

```bash
cargo check -p eggsec-cli --no-default-features
cargo check -p eggsec-cli --no-default-features --features daemon-client
cargo check -p eggsec-cli --features tui
```

## Final Acceptance Criteria

This corrective pass is complete when:

- Every daemon command has a central permission mapping.
- Every session-scoped command is authorized before execution.
- Authorization uses actual session surface, not inferred access defaults.
- Mutating commands require a declared client.
- CLI/TUI clients fail clearly if declaration fails.
- `ApprovePolicy` is either fully wired to pending policy state or returns explicit unsupported/invalid-state errors.
- Strict/MCP/agent/CI sessions cannot receive unrelated manual approvals.
- Observer/controller/owner/approver semantics are tested with denial cases.
- Error codes are structured and deterministic.
- Local socket security baseline is documented and tested where practical.
- The plan/audit trail remains intact.

Only after this pass should the repo proceed to broader transport work or richer remote frontend behavior.
