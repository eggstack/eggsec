# NSE Runtime Extraction Milestone 001 — Closure Status

Status: closed

Source implementation plan:

- `plans/implementation/nse-runtime-extraction/001-canonical-execution-report-convergence.md`

Source subsystem roadmap:

- `plans/subsystems/nse-runtime-extraction-roadmap.md#milestone-001--canonical-execution-and-report-convergence`

Repository baseline reviewed: `0509ac66` (plan registration) through implementation commit below.

Implementation commits or pull requests:

- `8ff2237` — feat(nse): converge on canonical runtime execution/report pipeline (run.rs, three caller migrations, contract/parity/dispatch tests, guards 25/30/35 + new 143, docs/skill updates, pre-existing `_reg` typo fix)

## 1. Executive finding

The milestone's infrastructure boundary is complete. One runtime-owned pipeline — `NseRunRequest` + `execute_nse_run()` in `crates/eggsec-nse/src/run.rs` — performs script resolution through complete `NseRunReport` production, and all three production surfaces (runtime CLI helper, Eggsec dispatch/TUI, Python binding) are thin adapters over it. No production surface reproduces runtime orchestration. No Eggsec authorization semantics, profile defaults, feature names, or supported NSE behavior changed. Milestone 002 is unblocked.

## 2. Requirement-to-evidence matrix

| Requirement (plan acceptance #) | Evidence | Result | Notes |
|---|---|---|---|
| 1. Single runtime-owned API: resolution → report | `run.rs::execute_nse_run` implements all 12 pipeline steps; guard 143 | pass | `NseExecutor::build_report` retained for low-level/tests only |
| 2. CLI, dispatch/TUI, Python use that API | `lib.rs::run_cli_with_profile`, `dispatch/api.rs::run_nse`, `nse.rs::run_nse_inner` all construct `NseRunRequest` + call `execute_nse_run`; guard 143 | pass | — |
| 3. No `ScriptResolver` bypass for custom files | `std::fs::read_to_string` removed from `dispatch/api.rs`; guard 143 fails on reintroduction | pass | File sources resolve via `NseScriptSource::File` everywhere |
| 4. Complete reports on all surfaces | `canonical_run_tests` (16), dispatch tests (3), manual-vs-automated parity test | pass | stats/evidence now present on CLI/Python/dispatch |
| 5. One static-`require` owner | `run::extract_static_requires`; Python duplicate deleted; guard 143 | pass | — |
| 6. Profile defaults unchanged | `report_contract_tests` profile-default tests; manual-permissive default in CLI, AgentSafe in Python/dispatch-manual preserved | pass | check 26 still green |
| 7. Serialization preserved | contract tests + roundtrip test; only `script_source.kind` value unified (builtin), no field added/removed | pass | intentional delta recorded (§7) |
| 8. Cancellation/limits/sandbox/negatives enforced | cancellation + limits-override + denial tests; full pre-existing suites green | pass | — |
| 9. Clean-room corpus passes canonically | `compatibility_corpus_tests` (43) + `runtime_corpus_tests` (16) green | pass | provenance untouched |
| 10. `nse`/`nse-ssh2`/`nse-sandbox` builds valid | `cargo check` for each; `check-features-individual` 89/89 | pass | `nse-ssh2` compile-verified (no native SSH runtime in env) |
| 11. `make check` passes | exit 0 (fmt, no-default, deny, clippy, tests, guards) | pass | — |
| 12. No inward Eggsec dependency added | `eggsec-nse` manifest unchanged (still `eggsec-core`/`eggsec-report-model`/`eggsec-transport`, the known 002 scope) | pass | no new edge |

## 3. Production implementation evidence

Final canonical API: `NseRunRequest { target, script: NseScriptSource, script_args, profile, host_context?, port_context?, limits_override?, cancellation? }`, `NseRunError { kind, message, target, script_name, script_source, profile, diagnostics }` with `failure_report()`, `execute_nse_run(request) -> Result<NseRunReport, NseRunError>` (sync; callers wrap in `spawn_blocking`).

Before/after caller inventory:

| Caller | Before | After |
|---|---|---|
| `run_cli_with_profile` (`eggsec-nse/src/lib.rs`) | own resolver/executor/report assembly (~90 lines), `InlineManual` builtins, no stats | thin adapter; `Builtin` source identity; JSON contract preserved (failure JSON on resolution errors) |
| `dispatch/api.rs::run_nse` | direct `std::fs::read_to_string`, no target set, `build_report` with empty diagnostics, rules/evidence dropped | `NseRunRequest` (`File`/`Builtin`), full report; target now set (orchestration bug fixed) |
| `run_nse_inner` (`eggsec-python/src/nse.rs`) | own orchestration + duplicated static-require parsing, no stats/evidence | thin adapter; `Builtin` identity; stats + evidence now populated |
| `nse_tool.rs` (unregistered `SecurityTool`) | unchanged | out of scope: no report pipeline, still unwired (pre-existing) |

Direct custom-file read path removed (evidence: `rg std::fs::read_to_string crates/eggsec/src/dispatch/api.rs` empty; guard 143).

Report field parity matrix (runtime CLI adapter / Eggsec manual dispatch / Python): resolver diagnostics, rule evaluations, execution stats, library-use data, capability events, compatibility/fidelity, output, evidence — all populated by the single pipeline; manual-vs-automated parity test asserts equal rules/output/libraries/compatibility/evidence for the same fixture apart from profile/limits sections.

## 4. Verification executed

### Commands run

```bash
cargo test -p eggsec-nse --features nse
cargo test -p eggsec --features nse,cli --lib
cargo test -p eggsec --features nse,cli --test nse_tests --test nse_integration_tests --test nse_real_scripts
cargo test -p eggsec-tui --features nse
cargo test -p eggsec-python --features nse
cargo check -p eggsec-nse --features nse
cargo check -p eggsec --features nse,cli
cargo check -p eggsec-tui --features nse
cargo check -p eggsec-python --features nse
cargo check -p eggsec-nse --features nse-ssh2
cargo check -p eggsec-nse --features nse,sandbox
cargo check -p eggsec --features nse-ssh2,nse-sandbox,cli
make check
make check-features-individual
make check-python
bash scripts/check-architecture-guards.sh
```

### Results

- `eggsec-nse` (nse): 577 passed, 0 failed, 1 ignored (23 suites; incl. new `report_contract_tests` 7, `canonical_run_tests` 16).
- `eggsec` lib (nse,cli): 1704 passed, 0 failed (incl. new `dispatch::api::nse_canonical_dispatch_tests` 3).
- `eggsec` nse binaries: `nse_tests` 174, `nse_integration_tests` 21, `nse_real_scripts` 28 — all pass.
- `eggsec-tui` (nse): 903 passed, 0 failed.
- `eggsec-python` (nse): 231 Rust tests pass; `make check-python` exit 0 (pytest 4454 passed, 1706 skipped, 23 deselected, 17 xfailed against rebuilt bindings).
- `make check`: exit 0. `make check-features-individual`: 89 passed, 0 failed. Guards: all pass incl. new check 143 and updated 25/30/35.
- `nse-ssh2` paths compile-verified only (no native SSH runtime in this environment); recorded as environmental, not omitted.
- Full `cargo test -p eggsec --features nse,cli` (all integration binaries) was not run as one shot (suite exceeds command timeout); coverage obtained via lib + targeted NSE binaries above. No failures anywhere.

## 5. Invariant review

- Eggsec authorization outside `eggsec-nse`: unchanged; `EnforcementContext` untouched; dispatch still gated upstream (`handle_nse` enforces before execution). Evidence: no auth code moved; `nse_tool.rs` still unwired.
- Strict surfaces dispatch after canonical Eggsec approval: unchanged paths.
- Profile defaults: manual helpers default `ManualPermissive`; Python/dispatch-manual explicit as before (contract tests pin this).
- `ScriptResolver` file policy enforced everywhere; resolver containment/symlink/name/extension/size checks untouched and fail-closed (existing resolver + policy suites green).
- Cancellation token, limits, sandbox, capability events preserved (tests added).
- `NseRunReport` fields intact (contract tests).
- Feature behavior (`nse`, `nse-ssh2`, `nse-sandbox`, stress-testing) compatible (feature sweep green).
- Clean-room corpus provenance unchanged (guard 38/54 green).
- Ring-only TLS intact (guards green).

## 6. Failure and recovery review

- NSE execution is request-scoped, non-durable: unchanged; restart recovery N/A (no durable state introduced).
- Cancellation: pre-execution and mid-execution cancellation return typed `NseRunErrorKind::Cancelled`, never a successful empty report (tests). Executor hook/limit machinery untouched.
- Partial resolution failure preserves diagnostics in `NseRunError` and `failure_report()` (`Failed` compatibility).
- Rule-success/action-failure stays explicit (executor semantics unchanged; converged on CLI's full assembly including evidence).
- Concurrent runs share no mutable per-run state (4-thread isolation test; executors are per-run).

## 7. Migration and compatibility review

Internal ownership migration; no storage/protocol migration. `NseRunReport` JSON shape preserved (contract + roundtrip tests). Intentional, documented deltas (`architecture/nse_integration.md#canonical-executionreport-convergence-extraction-milestone-001`):

1. Named scripts report `script_source.kind="builtin"` on all surfaces (CLI/Python were `"inline"`).
2. CLI/Python reports gained stats/evidence sections.
3. Dispatch gained target, diagnostics, rules, evidence (fixed unset-target/empty-diagnostics drift).
4. Custom-file dispatch failures are errors via resolver instead of direct reads.
5. `nse-ssh2` runtime execution not exercised here (compile-only; environmental).

Public helpers (`run_cli`, `run_cli_with_profile`), `eggsec::nse::*` re-exports, Python class/function names and fields, feature names/defaults, built-in identifiers: all preserved (`make check-python` + stub checks green).

## 8. Security review

- No authorization moved into the runtime; automated surfaces never touch manual-only constructors (checks 26/27/36 green).
- Script-file policy now enforced on the dispatch path that previously bypassed it (positive security fix; negative tests pin denial + diagnostics).
- `NseRunError` messages for file denials preserve the historical CLI wording (no information-structure change).
- Pre-existing drive-by fix (same commit, recorded here): `dispatch/mod.rs` test `executor_registry_feature_gated_operations` referenced `reg` after binding `_reg`, breaking lib-test compilation under `nse`/`db-pentest` features; renamed uses to `_reg`. Production code untouched.

## 9. Documentation and operations

- `architecture/nse_integration.md`: canonical pipeline owner, allowed callers, intentional deltas.
- `.opencode/skills/eggsec-nse/SKILL.md`: canonical API guidance; stale direct-construction claims corrected.
- `docs/NSE_COMPATIBILITY.md`: no change required (no ownership/path claims to reconcile; compatibility counts untouched).
- Guards: check 143 added (canonical ownership); checks 25/30/35 updated to the new ownership (run.rs as report owner, lib.rs as adapter).
- No operator action required; no new flags, profiles, or defaults.

## 10. Unresolved findings

| Severity | Finding | Impact | Required action |
|---|---|---|---|
| low | `nse-ssh2` execution not exercised in this environment | native SSH runtime unavailable; compile-only evidence | qualify on SSH-capable runner before standalone extraction claims SSH parity (carry to 002 closure) |
| low | Full engine integration sweep not run as a single shot (command timeout) | coverage via lib + targeted binaries, all green | none; CI runs the full matrix |

No critical/high/medium findings.

## 11. Roadmap disposition

Milestone 001 closed; Milestone 002 dependency may proceed. The hard gate for 002 (accepted 001 closure with one canonical pipeline + parity evidence) is satisfied. No corrective pass required.

## 12. Registry updates

- `plans/registry.md`: 001 → closed (link this record); 002 → ready.
- `plans/subsystems/nse-runtime-extraction-roadmap.md`: milestone table 001 closed / 002 ready.
- `plans/implementation/nse-runtime-extraction/001-*.md`: Status → implemented.
