# Daemon/Frontend Cleanup and Polish Pass

## Objective

Close the remaining daemon/frontend separation issues after the trust-boundary corrective pass. The repo now has the core structure in place: runtime execution context, session-bound surface/scope propagation, strict-surface fail-closed behavior, approved request bundles, terminal event ordering, progress forwarding, and adapter-level cancellation. This pass should focus on cleanup, polish, and final correctness rather than expanding the architecture.

The goal is to make daemon-backed execution credible for future CLI/TUI dual-backend work while preserving the product split:

- Manual CLI/TUI remain first-class human operator workflows.
- Daemon-backed manual CLI/TUI remains manual when bound to manual surfaces.
- MCP, agent, REST, gRPC, and CI remain strict, fail-closed, and non-overridable.
- The daemon is a runtime host, not an enforcement profile.

## Current State Summary

The prior corrective pass materially improved the repo:

- `RuntimeTaskExecutor::execute()` now receives `RuntimeExecutionContext`.
- Runtime context is populated from session state rather than executor defaults.
- `EggsecRuntimeExecutor` uses context surface and session scope metadata.
- Strict surfaces fail closed if explicit scope cannot be resolved.
- `RuntimeSurface::Unknown` is rejected before real execution.
- `ApprovedRunRequest` couples approval with the request and validates operation/target before dispatch.
- Terminal runtime state is written before terminal events are emitted.
- Progress forwarding and adapter-level cancellation were added.
- Owner client ID is persisted into session snapshots for access filtering.

Remaining work is concentrated in four areas:

1. close-session persistence still deletes useful session history;
2. full-executor capabilities are still too broad for a safe default daemon surface;
3. `RunRequest.surface` can still be stale/spoofed in stored task records and events;
4. strict-surface scope handling is path-based and should be documented/validated as a limitation or tightened.

## Non-goals

Do not add new transports in this pass. Do not implement desktop/mobile/web frontend SDKs. Do not expand task families. Do not make the daemon mandatory for normal CLI/TUI operation. Do not loosen strict automated enforcement to improve convenience.

## Work Item 1: Preserve Closed-session History

### Problem

`Runtime::close_session()` now cancels active tasks, marks the session closed, and emits events after state mutation. That is correct. The daemon host still deletes the persisted session snapshot when handling `CloseSession`, which removes useful audit/history state.

### Desired behavior

Closing a session should preserve a final closed snapshot. Deleting a session should be a separate explicit destructive action, not the meaning of close.

### Implementation guidance

In `crates/eggsec-daemon/src/host.rs`, change `ClientCommand::CloseSession` handling:

1. call `runtime.close_session(session_id)`;
2. immediately fetch `runtime.snapshot(session_id)`;
3. persist that final closed snapshot with `store.save_session_snapshot(&snapshot)`;
4. record the close-session audit event;
5. do not call `store.delete_session(session_id)`.

If close should remove the session from live `ListSessions`, keep that behavior through `Runtime::list_sessions()` filtering closed sessions. Do not delete persisted state.

### Tests

Add tests proving:

- close persists a snapshot marked `closed = true`;
- persisted snapshot remains retrievable after close;
- active tasks are cancelled in the final closed snapshot;
- `ListSessions` omits closed sessions if that remains desired;
- `GetPersistedSnapshot` can retrieve the closed session for authorized clients.

### Acceptance criteria

- No `delete_session()` call remains in close-session handling.
- Closing a session produces durable final history.
- Tests cover close with and without active task.

## Work Item 2: Normalize or Reject Request-surface Mismatches

### Problem

The executor now uses session surface from `RuntimeExecutionContext`, which fixes the major bypass. However, the submitted `RunRequest` still carries its own `surface` field. If a client submits a request with `request.surface = CliManual` into an `McpServer` session, enforcement uses the session context, but task records and runtime events may still show the spoofed request surface.

This is confusing and can mislead audit, debugging, UI rendering, and future code that accidentally reads `request.surface`.

### Desired behavior

The session surface should be authoritative. Stored and emitted requests should either be normalized to session surface or mismatches should be rejected at submit time.

Preferred first pass: normalize.

### Implementation guidance

In `Runtime::submit()`:

1. load the session surface while holding the state lock;
2. clone the incoming request into `normalized_request`;
3. if `normalized_request.surface == RuntimeSurface::Unknown`, set it to the session surface;
4. if `normalized_request.surface != session_surface`, either:
   - normalize it to session surface and emit a debug/warn log; or
   - return a new `RuntimeError::SurfaceMismatch`.

Given existing clients may not set request surface perfectly, normalizing is lower-friction and better for manual CLI/TUI migration. Strict enforcement remains safe because context surface is authoritative.

Use `normalized_request` everywhere after that point:

- task record storage;
- `TaskQueued` event;
- executor call;
- approved bundle request;
- snapshots.

### Tests

Add runtime tests proving:

- request with `Unknown` surface is stored/emitted with session surface;
- request with spoofed `CliManual` in `McpServer` session is normalized or rejected according to chosen behavior;
- executor receives session surface and normalized request surface consistently;
- snapshots do not show spoofed surface.

### Acceptance criteria

- No task record stores a request surface that disagrees with the session surface.
- `TaskQueued` events cannot misrepresent surface.
- Tests cover Unknown and mismatched request surfaces.

## Work Item 3: Split Full-executor Capabilities from Conservative Daemon Capabilities

### Problem

`RuntimeCapabilities::full()` still advertises all task kinds, including lab/hazardous families such as `packet-send`, `wireless-active`, `db-pentest`, `intercept`, and `c2`. The noop executor now reports no task kinds, which is correct, but the real daemon executor should not automatically advertise every feature family as a safe default.

### Desired behavior

Capabilities should be honest and conservative by default. The daemon full executor should advertise a safe initial subset unless an explicit lab/full-lab mode is configured.

Suggested capability profiles:

- `RuntimeCapabilities::noop()` — no executable task kinds.
- `RuntimeCapabilities::daemon_conservative()` — safe daemon-backed default task kinds.
- `RuntimeCapabilities::full_lab()` or `RuntimeCapabilities::all_known()` — all known task kinds for explicit lab builds/profiles.

Recommended daemon conservative subset:

- `port-scan`
- `endpoint-scan`
- `fingerprint`
- `waf`
- `pipeline`
- `recon`
- optionally `load-test` and `fuzz` if rate/target controls are clear

Do not include by default:

- `stress-test`
- `packet-send`
- `wireless-active`
- `db-pentest`
- `intercept`
- `c2`
- dynamic mobile/runtime instrumentation
- other lab/hazardous feature families

### Implementation guidance

Update `crates/eggsec-runtime/src/capabilities.rs`:

- keep `Default` conservative if appropriate, or document exactly what `Default` means;
- add `daemon_conservative()`;
- rename or clarify `full()` as `full_lab()` if that better matches semantics;
- update tests to assert the daemon conservative list excludes hazardous task kinds.

Update `crates/eggsec-daemon/src/host.rs`:

- `new_noop()` should continue using `RuntimeCapabilities::noop()`;
- real daemon host should use `RuntimeCapabilities::daemon_conservative()` by default;
- add future-oriented configuration hook for lab/full capabilities, but do not enable it implicitly.

If the daemon does not yet have executor-mode config beyond `--full-executor`, defer `--lab-capabilities` to a later pass unless needed for tests.

### Enforcement vs capability

Capabilities are not enforcement. Unsupported-by-capability task kinds should also be rejected before dispatch when the daemon is in conservative mode. Add a runtime/daemon pre-submit check or executor-level check so a client cannot submit `c2` simply because it knows the serialized `TaskKind` variant.

### Tests

Add tests proving:

- noop capabilities are empty;
- daemon conservative capabilities include expected safe subset;
- daemon conservative capabilities exclude hazardous families;
- unsupported-by-capability submissions fail before dispatch;
- full/lab capability profile, if retained, is explicitly named and tested.

### Acceptance criteria

- Real daemon default does not advertise broad hazardous capability set.
- Capability list and actual accepted task kinds are consistent.
- Tests protect against accidental re-expansion.

## Work Item 4: Add Executor-supported Task Gate

### Problem

Descriptor mapping and dispatch may support more `TaskKind` variants than the daemon should accept in its default real-executor profile. Capability honesty alone is not enough; clients can still submit serialized task kinds directly.

### Desired behavior

The daemon/runtime should reject task kinds that are not supported by the current executor capability profile before they reach approval or dispatch.

### Implementation options

Option A: runtime-level gate.

- Add `RuntimeCapabilities::supports_task_kind(&TaskKind) -> bool`.
- In `Runtime::submit()`, reject unsupported task kinds using the session/runtime capabilities.
- Return `RuntimeError::UnsupportedTaskKind`.

Option B: executor-level gate.

- `EggsecRuntimeExecutor` checks the task kind against a configured supported set before approval.
- This keeps the runtime generic but makes per-executor behavior explicit.

Preferred: combine a generic helper with executor-level configuration. The runtime should avoid policy semantics, but it can enforce declared executor capability constraints if capabilities are part of `RuntimeConfig`.

### Tests

- daemon conservative executor rejects `C2`, `PacketSend`, `WirelessActive`, `DbPentest`, and `Intercept` task kinds;
- accepted task kinds still reach approval;
- noop executor reports unsupported clearly;
- manual CLI/TUI embedded paths are not unintentionally constrained by daemon conservative capability if they do not use that runtime config.

### Acceptance criteria

- Capabilities cannot lie by omission while dispatch still accepts hidden task kinds.
- Unsupported task rejection is deterministic and typed.

## Work Item 5: Tighten Strict-surface Scope Semantics

### Problem

The executor currently reconstructs `LoadedScope` from `SessionScope` by loading from `SessionScope.path` when `is_explicit = true`. If an explicit scope has no path, strict surfaces fail closed. That is safe, but it means inline/preset/in-memory explicit scopes may not be usable for daemon strict execution.

### Desired behavior

Strict daemon sessions should either carry enough scope information to reconstruct the actual scope rules or fail closed with a clear error.

### Implementation guidance

For this polish pass, choose one of:

1. Document path-backed explicit scope as the only supported strict daemon scope source for now.
2. Extend `SessionScope` to carry serialized scope rules for inline/preset scope sources.
3. Add a scope registry/store in daemon host and pass a scope handle through runtime context.

Preferred for cleanup: document and validate path-backed scope as the only supported strict daemon scope source. Add tests to lock in fail-closed behavior for missing/unresolvable path.

If serialized scope rules are already easy to add without pulling `eggsec` into `eggsec-runtime`, consider adding a protocol-neutral allow/exclude representation to `eggsec-runtime`. Otherwise defer this as a future phase.

### Tests

- strict session with no scope fails closed;
- strict session with `is_explicit = true` but no path fails closed;
- strict session with nonexistent path fails closed;
- manual session with no scope remains allowed according to manual profile;
- docs describe the limitation.

### Acceptance criteria

- Strict scope behavior is explicit, tested, and documented.
- No strict path falls back to default-empty scope.

## Work Item 6: Refine Persisted-session Access Policy

### Problem

Owner filtering was added, but CLI/TUI/daemon-internal clients are currently treated as elevated for persisted-session listing. That may be acceptable for local single-user mode, but it should be deliberate, documented, and configurable before future multi-client frontends depend on it.

### Desired behavior

Persisted-session listing and snapshot reads should have a clear access model:

- owner can read/list own persisted sessions;
- explicitly allowed clients can read where sharing exists;
- daemon-internal/admin can list all;
- ordinary unrelated clients cannot list/read another owner's sessions;
- legacy sessions with no owner should have a documented temporary behavior.

### Implementation guidance

For this pass:

- rename the broad elevated check to something explicit such as `is_local_trusted_client_kind` if keeping it;
- add comments documenting that CLI/TUI elevation assumes local single-user daemon mode;
- consider removing CLI/TUI from elevated global listing and reserving all-session listing to `DaemonInternal` only;
- if keeping CLI/TUI elevation, add a config flag in `DaemonConfig`, e.g. `allow_local_clients_global_persisted_listing`, defaulting to current local behavior.

Preferred security posture: only owner and `DaemonInternal` get global persisted listing; CLI/TUI see own sessions by default. This does not harm normal single-user use if owner metadata works.

### Tests

- owner lists own persisted sessions;
- unrelated client does not see another owner's sessions;
- daemon-internal can list all;
- legacy/no-owner behavior is tested and documented;
- snapshot read follows the same policy.

### Acceptance criteria

- Access behavior is intentional, tested, and documented.
- No broad client kind elevation exists without a clear config/documentation rationale.

## Work Item 7: Align Runtime Error Semantics

### Problem

Closed sessions and strict enforcement failures may currently collapse into generic `SessionNotFound` or `DispatchFailed` errors. That is workable but less precise for frontends and tests.

### Desired behavior

Runtime and daemon errors should distinguish:

- session not found;
- session closed;
- unsupported task kind;
- surface mismatch or normalized mismatch warning;
- enforcement denied;
- strict scope unavailable;
- cancellation.

### Implementation guidance

Add or refine `RuntimeError` variants if the enum already supports extension:

- `SessionClosed(String)`
- `SurfaceMismatch { session: RuntimeSurface, request: RuntimeSurface }`
- `EnforcementDenied(String)`
- `ScopeUnavailable(String)`
- `Cancelled`

Then map them to daemon `ErrorCode`s as specifically as possible. If protocol error codes are too narrow, add new codes or use existing `InvalidRequest`/`PermissionDenied` consistently.

Do not churn public protocol more than needed; precision is useful, but compatibility matters.

### Tests

- submit to closed session returns a closed-session error;
- strict scope failure maps to permission/invalid-scope style error, not generic internal;
- unsupported-by-capability maps to invalid request;
- cancellation does not appear as internal failure.

### Acceptance criteria

- Frontends can distinguish common lifecycle/security failures.
- Error mappings are tested.

## Work Item 8: Expand Regression Tests Around the Fixed Trust Boundary

### Desired behavior

The current static architecture guard now documents/guards against hardcoded permissive executor defaults. This should be complemented with runtime tests that fail if the executor/session context behavior regresses.

### Tests to add or verify

Runtime tests:

- executor receives session surface from `RuntimeExecutionContext`;
- request surface is normalized or rejected;
- closed session rejects submit;
- terminal event snapshot consistency;
- close persists cancelled active task in completed history.

Bridge/executor tests:

- strict `McpServer` without scope fails closed;
- strict `SecurityAgent` without scope fails closed;
- strict surface with explicit but missing scope path fails closed;
- manual `CliManual` without scope remains allowed under manual profile;
- approved bundle rejects operation mismatch;
- approved bundle rejects target mismatch.

Daemon host tests:

- close preserves final persisted snapshot;
- persisted listing is owner-filtered;
- capabilities match executor mode;
- unsupported task kind is rejected in conservative daemon mode.

Architecture guard:

- keep grep guard for hardcoded `RuntimeSurface::CliManual` in `EggsecRuntimeExecutor`, but allow tests and comments only if the pattern cannot avoid them;
- add guard ensuring close-session path does not call `delete_session()`.

### Acceptance criteria

- New tests exercise behavior, not only static patterns.
- Architecture guards cover the two most likely regressions: hardcoded manual executor surface and deleting closed-session history.

## Work Item 9: Documentation Polish

Update docs to match the final behavior. Avoid aspirational wording that implies transports or frontend SDKs are already implemented if they are not.

Docs to review:

- `architecture/daemon.md`
- `docs/ARCHITECTURE.md`
- `docs/ARCHITECTURE_INVARIANTS.md`
- `docs/CI_ARCHITECTURE_GUARDS.md`
- `docs/FEATURE_MATRIX.md`
- README daemon/runtime sections if present

Required documentation points:

- daemon is optional for CLI/TUI;
- embedded CLI/TUI manual mode remains first class;
- daemon-backed CLI/TUI manual mode remains manual by session surface;
- automated surfaces remain strict by session surface;
- strict daemon execution currently requires resolvable explicit scope metadata;
- noop daemon mode is protocol/session only;
- conservative daemon real-executor mode exposes only a safe supported subset unless lab mode is configured;
- close preserves history and does not delete session state.

### Acceptance criteria

- Docs match implemented behavior.
- Docs do not claim WebSocket/gRPC/frontend SDK readiness unless implemented.
- Manual vs automated surface semantics are stated consistently.

## Validation Commands

Run targeted checks first:

```bash
cargo test -p eggsec-runtime
cargo test -p eggsec-daemon
cargo test -p eggsec --lib runtime_bridge
cargo check -p eggsec-daemon --all-targets
cargo check -p eggsec-daemon --features full-executor --all-targets
```

Then run broader checks if local resources allow:

```bash
cargo test -p eggsec --lib
cargo check --workspace --all-targets
./scripts/check-architecture-guards.sh
```

If full workspace validation is too heavy, record the exact targeted commands that passed and any unrelated failures.

## Acceptance Checklist

- [ ] Close-session no longer deletes persisted history.
- [ ] Closed sessions persist final closed snapshots.
- [ ] Request surface is normalized to session surface or mismatches are rejected.
- [ ] Task records/events/snapshots do not show spoofed request surfaces.
- [ ] Daemon full-executor default capabilities are conservative.
- [ ] Hazardous/lab task kinds are not advertised by default.
- [ ] Unsupported-by-capability task submissions are rejected before dispatch.
- [ ] Strict daemon scope behavior is documented and tested.
- [ ] Persisted-session access policy is explicit and tested.
- [ ] Error semantics are precise enough for frontends.
- [ ] Regression tests cover context surface, scope fail-closed, approved bundle validation, close history, and capabilities.
- [ ] Architecture guards protect against hardcoded manual daemon execution and close-session deletion.
- [ ] Documentation reflects implemented behavior, not future aspirations.

## Handoff Notes

Keep this pass narrow. The daemon/frontend architecture is close enough that broad refactoring would add risk. Focus on making current semantics exact, tested, and documented so the next phase can safely add CLI/TUI backend selection and frontend attach/reconnect workflows.
