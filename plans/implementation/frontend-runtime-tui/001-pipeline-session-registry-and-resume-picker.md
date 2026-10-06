# Pipeline Session Registry and TUI Resume Picker — Milestone 001

Status: active

Source subsystem roadmap: `plans/frontend-runtime-convergence-roadmap-2026-09-10.md` (subsystem `frontend-runtime-tui`, previously closed; this milestone reopens a bounded slice of it)

Long-term references:

- `plans/000-long-term-specification.md#2-primary-product-goals`
- `plans/002-long-term-roadmap.md`

Related ADRs: none required. This milestone adds no new transport, enforcement, or runtime-ownership decision.

## 1. Objective

Make `Tab::Resume` a real, dispatchable TUI capability backed by a discoverable
store of scan checkpoints, replacing the current honest-but-inert refusal.

## 2. Non-goals

- No new operation id and no `OperationMetadata` entry (see §5).
- No change to the CLI `resume` command's user-facing contract.
- No daemon/agent/MCP exposure of resume. Resume stays manual-surface only.
- No session retention/pruning policy, no multi-session resume.
- No migration of checkpoints written by older versions.

## 3. Current implementation evidence

Three unrelated "session" concepts exist. Confusing them is the root cause of
the dead tab:

| Concept | Type | Location | Role |
|---|---|---|---|
| Runtime session | `eggsec_runtime::session::SessionSnapshot` | `crates/eggsec-runtime/src/session.rs:350` | in-memory runtime/daemon state; not a file |
| TUI UI state | `eggsec_tui::session::SessionState` | `crates/eggsec-tui/src/session.rs` | bookmarks/theme/last-tab; listed by the History tab |
| Scan checkpoint | `eggsec::pipeline::session::PipelineSession` | `crates/eggsec/src/pipeline/session.rs:10` | the only thing `eggsec resume` consumes |

The blocker is checkpoint *location*, not schema. `PipelineSession` is written
by `Pipeline::run` only when `self.session_path` is `Some`, and that path is
derived from the scan's `--output` when it ends in `.session.json` or `.session`
(`crates/eggsec/src/pipeline/executor.rs:178-181`). There is no session
directory, no index, and no manifest: checkpoints land wherever the operator
pointed their report output. Nothing in the TUI ever sets a session path, so a
TUI-initiated scan writes no resumable checkpoint at all.

## 4. Invariants that cannot regress

- `EnforcementContext::evaluate()` remains the mandatory pre-dispatch gate;
  resume must not bypass it.
- Strict surfaces dispatch only via `EnforcedDispatcher::dispatch_execution()`.
  Resume adds no automated-surface exposure.
- Production `eggsec-tui` writes no terminal bytes (guard Check 138). A
  non-printing pipeline resume path is therefore required; `resume_cli` today
  does `println!` at `crates/eggsec/src/pipeline/mod.rs:311`.
- `OperationMetadata` stays the single source of truth for operation policy.
- No `let _ =` / `filter_map(|e| e.ok())`; unreadable checkpoints are traced.
- The TUI session directory holds TUI UI state and MUST NOT be reused for
  scan checkpoints — the schemas are unrelated and a checkpoint would fail to
  deserialize as `SessionState`.

## 5. Key design decision: no new operation id

`resume` is already classified as a command whose canonical operation family is
`["pipeline"]` (`crates/eggsec/src/commands/route.rs:293`), and
`handle_resume` already resolves enforcement through that registry
(`crates/eggsec/src/commands/handlers/scan.rs:182-185`). So `Tab::Resume`
declares `operation: Some("pipeline")` — canonical, already covered by a TUI tab
(Scan), and satisfying `parity.rs` without a new catalog entry. Two tabs sharing
one operation is allowed: `tabs/surface.rs` asserts uniqueness only for aliases
and discoverable palette commands, not for operations.

The enforcement target is the target stored in the session, which requires
loading the session before dispatch builds its descriptor.

## 6. Expected production-code changes

### 6.1 Engine — session store (`crates/eggsec/src/pipeline/session.rs`)

- `default_session_dir()` — `EGGSEC_SESSION_DIR` override, else the platform
  data dir + `scan-sessions`. Deliberately a different directory from the TUI's
  `sessions/`.
- `pub struct SessionEntry { path, target, completed_stages, remaining_stages, modified }`
- `pub async fn list_sessions(dir) -> Vec<SessionEntry>` — reads `*.json`,
  deserializes `PipelineSession`, skips unreadable/unparseable entries with
  `tracing::warn!`, sorts newest first.
- `save` / `load` keep their existing schema, so checkpoints already on disk
  remain resumable.

### 6.2 Engine — non-printing resume (`crates/eggsec/src/pipeline/mod.rs`)

- Split into `pub async fn resume(path, config) -> Result<PipelineReport>` plus a
  thin `resume_cli` that renders the report. No behavior change for the CLI.

### 6.3 Runtime DTO (`crates/eggsec-runtime/src/request.rs`)

- `pub struct ResumeParams { pub session_path: String }`
- `TaskKind::Resume(ResumeParams)`
- `PipelineParams.session_path: Option<String>` with `#[serde(default)]`, so a
  TUI-initiated pipeline run can request a checkpoint at an engine-chosen path.
  The CLI's `--output *.session.json` derivation is unchanged.

### 6.4 Engine — dispatch (`crates/eggsec/src/dispatch/canonical_execution.rs`)

- `CanonicalOperationRequest::Resume(ResumeParams)`
- Arm calls `pipeline::resume`, returns `TaskResult::Pipeline(report)` so the
  existing `"pipeline"` renderer in `eggsec-ui-model` applies unchanged; no new
  result renderer.

### 6.5 TUI (`crates/eggsec-tui`)

- `TabSpec` for `Tab::Resume`: `operation: Some("pipeline")`.
- `ResumeTab`: a selector listing `SessionEntry` rows (target, completed /
  remaining stage counts, age) plus a path field as a manual escape hatch for
  checkpoints outside the store directory.
- `TaskBuilder` producing `TaskKind::Resume`.
- `Enter` on a row resumes; the tab reports load failures per row rather than
  aborting the whole listing.

## 7. Storage, protocol, migration, compatibility

- **Storage:** a new `scan-sessions/` directory. Checkpoints are opt-in — no
  scan writes one unless it was asked to — so there is no default disk growth.
- **Protocol:** `TaskKind` gains a variant under `#[serde(tag = "kind", content =
  "params")]`. Strict/automated surfaces reject it because `pipeline` is not
  automated-exposed for this path and Resume declares no new exposure.
- **Migration:** none. The checkpoint schema is unchanged, so `.session.json`
  files written today still load.

## 8. Work packages

1. Session store: dir convention, `SessionEntry`, `list_sessions`, tests.
2. Split `resume_cli` into library + CLI wrapper; add unit tests for the library
   function's error surface.
3. Runtime DTO additions; update the six `TaskKind` construction fixtures.
4. `CanonicalOperationRequest::Resume` arm returning `TaskResult::Pipeline`.
5. TUI picker, builder, and `TabSpec` change; update the Resume tab tests that
   currently assert the refusal.
6. Docs: `architecture/tui.md` tab inventory and operation count; the counted
   sets the skills pin.

## 9. Verification

Focused:

```bash
cargo test --lib -p eggsec-tui resume
cargo test -p eggsec --features rest-api,cli --lib pipeline
cargo test -p eggsec --test runtime_contract_closure
```

Broad (minimum before closure):

```bash
make check
```

Additionally: confirm the two guard assertions that Resume's route change touches
(`parity.rs` operation coverage, `surface.rs` route invariants) pass, and that
`cargo check -p eggsec-tui --features nse,wireless,web-proxy,db-pentest,c2` is
clean.

## 10. Acceptance and stop conditions

Accepted when: a checkpoint written through the store can be selected in the TUI
picker and resumed end to end, producing a `TaskResult::Pipeline` report;
`make check` passes; guards 138/139 still pass; no new clippy warnings in
touched files.

Stop and re-plan if: resuming a checkpoint requires weakening the enforcement
gate, or if the picker cannot be built without changing the checkpoint schema.

## 11. Closure evidence required

Implementation commits; requirement-to-evidence mapping against §4; `make check`
output; confirmation that Resume adds no automated-surface exposure; and a
note on the known limitation that CLI checkpoints written outside the store
directory are reachable only through the manual path field.
