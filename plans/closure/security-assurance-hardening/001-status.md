# Security Assurance Hardening Milestone 001 — Closure Status

Status: closed

Source implementation plan:

- `plans/implementation/security-assurance-hardening/001-enforcement-gate-integrity.md`

Source subsystem roadmap:

- `plans/subsystems/security-assurance-hardening-roadmap.md` §7 Milestone 1

Repository baseline reviewed: `77188755a41bee15aee9507055ac6b2761ff9271` (implementation remains uncommitted in the worktree)

Implementation commits or pull requests:

- None. No commit or PR was created, per instruction; evidence below describes the current working tree.

## 1. Executive finding

The manual TUI submission boundary now consumes a mandatory `ApprovedOperation` and validates its operation/target binding against the `RunRequest` before runtime submission. The unchecked dispatch entry points are narrowed, while the TUI retains its guarded canonical path. The milestone changes assurance structure, not policy semantics.

## 2. Requirement-to-evidence matrix

| Requirement | Evidence | Result | Notes |
|---|---|---|---|
| TUI submission cannot omit approval | `crates/eggsec-tui/src/app/task_runtime.rs`; `spawn_task_rejects_mismatched_approval_before_runtime_submit` and matching-approval test | pass | Approval is no longer optional. |
| Approval binds operation and effective target | `crates/eggsec/src/dispatch/binding.rs`; engine binding tests; `cargo test -q -p eggsec --features rest-api,cli --test canonical_dispatch_ownership` | pass | Resume uses the selected session target, not a request-name special case. |
| Unchecked entry points are not general API | `crates/eggsec/src/dispatch/mod.rs`, `canonical_execution.rs`; architecture guard 151 | pass | Guard 151 ran in `make check`; a temporary optional/discarded-approval mutation made it fail, then the original source was restored. |
| Existing strict authorization semantics remain | `EnforcementContext`, canonical dispatcher, static guards and feature-enabled dispatch tests | pass | No policy or `OperationMetadata` semantics changed. |

## 3. Production implementation evidence

`spawn_task` accepts `ApprovedOperation` directly, validates the request binding before changing task/runtime state, and reports a typed error on mismatch. `dispatch_inner` is crate-visible only. `execute_canonical` remains doc-hidden/public only for its compatibility test and the TUI canonical route. No adapter-side scope check or alternate policy table was introduced.

## 4. Verification executed

### Commands run

```bash
cargo test -q -p eggsec --features rest-api,cli --test canonical_dispatch_ownership
cargo test -q -p eggsec --features rest-api,cli
cargo test -q -p eggsec-tui --features db-pentest,web-proxy,c2 --lib --no-fail-fast
make check
make check-full
make test-tui-pty
```

### Results

- Canonical dispatch integration target: 26 passed.
- Engine test suite with REST/CLI features: exit 0. An initial run exposed an obsolete doctest field name; the example was corrected to `request.id`, then the full command passed.
- Representative TUI profile: 1,077 passed, 0 failed. The test scope now treats Intercept as a non-text-input control and separately checks its running-state navigation guards; a same-second timestamp assertion was also made deterministic.
- `make check`, `make check-full`, and PTY smoke all passed. `make check-full` includes the mandatory check, domain lint, representative feature profiles, and PTY smoke.

## 5. Invariant review

- `EnforcementContext::evaluate()` remains the mandatory pre-dispatch gate.
- Strict surfaces retain `LoadedScope` and scope-carrying `ApprovedExecution` dispatch through `EnforcedDispatcher::dispatch_execution()`.
- `OperationMetadata` remains the single policy source; no inline authorization logic was added.
- The manual path's approval is bound to the request before runtime submission; no unchecked engine entry point was added to adapters.

## 6. Failure and recovery review

A binding mismatch returns before task submission, channels, or runtime state are created. Cancellation continues through the shared `race_with_cancel` path. This milestone adds no persistence, retry, restart, or concurrency behavior.

## 7. Migration and compatibility review

Internal Rust API only; no on-disk or wire-schema migration. The TUI remains on the same manual authorization and confirmation model.

## 8. Security review

The TUI approval is load-bearing. Target validation is binding validation against the approved descriptor; strict-surface authority and scope provenance are unchanged. No new secret or target logging was introduced.

## 9. Documentation and operations

- TUI ownership and dispatch guidance: `architecture/tui.md`.
- Regression guard: `scripts/check-architecture-guards.sh` (Check 151), exercised by `make check`.
- Related operator and development guidance remains in `AGENTS.md`.

## 10. Unresolved findings

| Severity | Finding | Impact | Required action |
|---|---|---|---|
| — | No unresolved milestone finding identified by executed checks. | — | — |

## 11. Roadmap disposition

Milestone closed; Milestone 004's soft ordering after 001 is satisfied. No next dependency is blocked by this milestone.

## 12. Registry updates

- Record this milestone as closed with this closure reference in the security-assurance-hardening roadmap and `plans/registry.md`.
