# NSE Runtime Extraction Milestone 001 — Canonical Execution and Report Convergence

Status: ready for handoff

Repository baseline: `2ef67febf17a2ae42e2b8863b140f6c47cb7f387`

Source roadmap:

- `plans/subsystems/nse-runtime-extraction-roadmap.md#milestone-001--canonical-execution-and-report-convergence`

Long-term requirements:

- `plans/000-long-term-specification.md#2-primary-product-goals`
- `plans/000-long-term-specification.md#4-enforcement-invariants`
- `plans/000-long-term-specification.md#5-crate-ownership`
- `plans/001-terminology-and-domain-model.md#3-execution-terms`
- `plans/001-terminology-and-domain-model.md#5-data-and-reporting-terms`
- `plans/002-long-term-roadmap.md#phase-7--standing-maintenance-and-future-capability-open`

Applicable ADRs:

- none; ADR-0001/0002 transport decisions remain unchanged and must not be bypassed by this milestone.

Primary class: infrastructure

## 1. Objective

Create one authoritative `eggsec-nse` execution pipeline that owns script resolution, executor setup, rule/action execution, runtime evidence collection, compatibility computation, and complete `NseRunReport` assembly, then migrate all current NSE entry paths to that pipeline without changing Eggsec authorization semantics, profile defaults, feature names, or supported NSE behavior.

The milestone is complete when callers select request/profile/context and render/convert the returned report; they no longer reproduce runtime orchestration.

## 2. Why this milestone is ready

All hard dependencies are already closed:

- Eggsec authorization and canonical dispatch ownership are closed and documented.
- `eggsec-nse` already owns the required resolver, profile, limits, executor, capability, library registry, compatibility, and report primitives.
- Existing manual and Python surfaces already expose explicit profile choices.
- No new transport contract or persistent schema is required.
- The work can be completed entirely in-tree before any repository extraction.

The milestone deliberately precedes dependency decoupling so that runtime behavior is canonical before files or ownership move.

## 3. Current implementation evidence

At the baseline:

- `crates/eggsec-nse/src/lib.rs::run_cli_with_profile` resolves profile/script state, runs the executor, gathers rule/library/capability information, computes compatibility, and extracts evidence.
- `crates/eggsec/src/dispatch/api.rs::run_nse` independently constructs an `NseExecutor`, sets target/args, executes rules, and calls report assembly. Custom script files are read directly through `std::fs::read_to_string`, bypassing the runtime `ScriptResolver` path used by the runtime CLI helper.
- The Eggsec dispatch path does not assemble the same complete report data as the runtime helper; in particular resolver diagnostics and rule/evidence population can diverge.
- `crates/eggsec-python/src/nse.rs` has a third orchestration path using `AgentSafe`, `ScriptResolver`, executor/rule execution, library reporting, capability events, and report assembly. It duplicates static `require` fallback logic and does not use exactly the same execution-stats/evidence sequence as the runtime helper.
- `NseRunReport` already has the structured fields required by the TUI/report surfaces: profile/source/resolver data, rules, libraries, capability events, execution stats/limits, compatibility/fidelity, output, and evidence.
- `crates/eggsec-nse/tests/fixtures/nse_corpus` provides a clean-room compatibility corpus suitable for parity tests.

This duplication is the primary extraction blocker because moving the crate now would preserve divergent behavior across consumers.

## 4. Invariants that must not regress

- Eggsec authorization remains outside `eggsec-nse`; this plan does not move or weaken `EnforcementContext`.
- Strict Eggsec surfaces still dispatch only after canonical Eggsec approval.
- Runtime profile defaults remain unchanged: manual helpers may default to `ManualPermissive`; automated bindings must pass explicit safe/strict profiles.
- File/script source policy is enforced by `ScriptResolver`; no new ad-hoc filesystem read path is introduced.
- Resolver path-containment, symlink-escape rejection, module-name validation, extension/size checks, and profile restrictions remain fail-closed.
- Existing `NseCancellationToken`, execution-limit, sandbox, and capability-event behavior is preserved.
- `NseRunReport` serialized fields are not silently removed or renamed.
- `nse`, `nse-ssh2`, `nse-sandbox`, and stress-testing feature behavior remains compatible.
- No upstream Nmap script corpus is copied into fixtures or package assets.
- Ring-only TLS policy remains intact.

## 5. Scope

### In scope

- Add a canonical request/result orchestration API in `eggsec-nse`.
- Route built-in, trusted-registry, file, and inline/manual script sources through one resolution/execution path.
- Centralize rule evaluation and action execution.
- Centralize library-use detection including any static `require` fallback needed for compatibility reporting.
- Centralize collection of resolver diagnostics, rule reports, execution stats, library reports, capability events, output, compatibility/fidelity, and evidence.
- Migrate `run_cli_with_profile` to the canonical API.
- Migrate Eggsec NSE dispatch/TUI execution to the canonical API.
- Migrate Python NSE execution to the canonical API.
- Add parity tests proving report completeness and equivalent request behavior across entry surfaces.
- Update NSE architecture/compatibility documentation to identify the canonical pipeline.

### Explicitly out of scope

- Removing `eggsec-core`, `eggsec-report-model`, or `eggsec-transport` dependencies.
- Moving `bridge.rs`.
- Replacing the existing HTTP/comm/brute/vulns/upnp native implementations.
- Creating a standalone repository.
- Publishing a new crate/version specifically for extraction.
- Renaming `eggsec-nse`.
- Redesigning the entire public convenience API.
- Expanding NSE compatibility claims.

## 6. Required production changes

### Core/domain

Introduce a runtime-owned request type and one top-level execution function or service. Exact names are flexible; semantics are not. The API must accept:

- target;
- script source identity;
- optional script arguments;
- resolved execution profile;
- optional host/port/service context where currently supported;
- cancellation/limit overrides only through an explicit runtime-owned mechanism.

Prefer a request value over positional argument proliferation. Keep low-level `NseExecutor` APIs available where needed for compatibility/tests, but production surfaces should use the canonical orchestration API.

The canonical pipeline must:

1. resolve the supplied `NseScriptSource` using `ScriptResolver` and profile policy;
2. initialize the executor with the resolved profile;
3. set target/script args/context;
4. execute NSE rule evaluation and action semantics;
5. gather final execution stats;
6. gather dynamic library-use data plus one runtime-owned static fallback if required;
7. gather capability events;
8. preserve resolver diagnostics;
9. build the report;
10. compute compatibility/fidelity once;
11. extract evidence once;
12. return the complete report or a typed execution failure carrying enough context for a failure report where current behavior requires one.

Do not leave helper-specific copies of steps 5-11.

### Storage and migrations

None expected. Treat existing `NseRunReport` JSON output as a compatibility fixture. If a schema change proves unavoidable, stop and record it as an intentional compatibility change rather than folding it into refactoring.

### Protocol and DTOs

No daemon/wire DTO change is required.

If a new `NseRunRequest` is public, it should remain runtime-specific and should not import Clap, Ratatui, PyO3, daemon protocol, Eggsec policy, or Eggsec report-model types.

### Runtime and concurrency

Preserve current sync/async boundaries. It is acceptable for the high-level API to expose async orchestration if that best matches current runtime behavior, but do not create nested unbounded runtimes or duplicate Lua VMs solely for report assembly.

Cancellation must continue through `NseCancellationToken` and existing limit checks. Any newly spawned Tokio task requires repository-standard timeout bounds.

### Frontend or operator surface

`run_cli_with_profile` becomes a thin adapter:

- choose/default profile;
- construct the runtime request;
- invoke the canonical execution pipeline;
- render JSON/text and current warnings/errors.

Eggsec dispatch/TUI becomes a thin adapter:

- preserve Eggsec authorization upstream;
- choose `ManualPermissive` for the current manual TUI path;
- map UI/script selection to runtime source;
- invoke the canonical runtime API;
- return/render the resulting report.

Python becomes a thin adapter:

- preserve `AgentSafe` default and supported limit overrides;
- construct request;
- invoke runtime API;
- convert the returned report to PyO3 DTOs.

Remove duplicated static `require` parsing from bindings after runtime ownership exists.

### Security and authorization

This milestone must not confuse NSE runtime capability policy with Eggsec operation authorization.

The runtime may reject script/module/network/process/filesystem activity under its resolved NSE profile. Eggsec still decides whether the NSE operation may execute at all.

The Eggsec custom-file path must stop using direct `std::fs::read_to_string` and instead construct `NseScriptSource::File` for runtime resolution.

Automated surfaces must not call manual-only `NseExecutor` constructors as part of convergence.

### Documentation and static guards

Update NSE architecture documentation to state the canonical orchestration owner and allowed callers.

Add a static guard or focused test that production Eggsec/Python NSE paths use the high-level runtime API rather than constructing their own report pipeline. The guard may be dependency/import based if exact function-name matching would be brittle.

Document that report completeness is a contract across supported surfaces.

## 7. Ordered work packages

### Work package A — Freeze report and behavior baselines

Intent:

Create regression evidence before consolidating orchestration.

Required changes:

- add representative serialized `NseRunReport` fixtures or field-level contract tests for built-in and file/fixture execution;
- cover at least one matched rule, one unmatched rule, one compatibility downgrade/diagnostic case, and one capability denial/limit case where deterministic;
- capture current profile defaults for runtime CLI/manual Eggsec/Python;
- confirm clean-room corpus provenance remains unchanged.

Acceptance evidence:

- tests fail if a report section is silently dropped;
- tests identify profile/source/resolver/rule/library/capability/stats/compatibility/evidence fields.

### Work package B — Introduce canonical runtime request/execution API

Intent:

Make `eggsec-nse` the single owner of execution orchestration.

Required changes:

- add request/error types as needed;
- implement resolver-first execution;
- centralize rule/action execution and report assembly;
- centralize static/dynamic library-use reconciliation;
- ensure final stats and evidence are populated;
- preserve failure-report behavior expected by current callers.

Acceptance evidence:

- focused runtime tests execute built-in and fixture file sources solely through the new API;
- reports contain all required sections;
- no caller-specific state is required to finish report assembly.

### Work package C — Migrate runtime CLI helper

Intent:

Prove the existing runtime-facing adapter can become thin without behavior loss.

Required changes:

- rewrite `run_cli_with_profile` around the canonical API;
- preserve current output mode, warnings, errors, and manual default;
- remove now-dead orchestration helpers only after tests cover replacement behavior.

Acceptance evidence:

- existing CLI/runtime tests pass;
- no duplicate report assembly remains in `lib.rs`.

### Work package D — Migrate Eggsec dispatch and TUI path

Intent:

Eliminate the resolver/report drift in the manual Eggsec surface.

Required changes:

- replace direct custom-file read with `NseScriptSource::File`;
- construct the canonical request from current UI/dispatch inputs;
- preserve manual profile selection and current success/error projection;
- pass the complete report to TUI/report views;
- do not move Eggsec authorization into the runtime.

Acceptance evidence:

- direct custom-file filesystem bypass is gone;
- resolver diagnostics appear in resulting reports;
- rule reports and evidence are present when produced;
- existing TUI controls and expected outputs remain functional.

### Work package E — Migrate Python NSE execution

Intent:

Remove the third orchestration implementation.

Required changes:

- build the canonical request using `AgentSafe`;
- preserve supported execution-limit overrides;
- consume returned report;
- remove duplicated static `require` parsing/report assembly;
- preserve Python DTO and exception compatibility where possible.

Acceptance evidence:

- Python NSE tests pass;
- equivalent fixture requests report the same runtime fields as direct runtime execution apart from caller-specific rendering/projection.

### Work package F — Add parity/ownership guards and reconcile docs

Intent:

Prevent orchestration drift from returning.

Required changes:

- add surface-parity tests using deterministic local fixtures;
- add static guard(s) against independent production report assembly;
- update NSE architecture/compatibility docs and contributor guidance;
- record any intentional report/schema deltas.

Acceptance evidence:

- a future direct custom-file read or caller-owned report assembly is caught by test/guard where practical;
- docs identify one canonical pipeline.

## 8. Failure, cancellation, restart, and contention semantics

NSE execution remains request-scoped and non-durable. Process restart may terminate active NSE work; this milestone introduces no restart recovery.

Cancellation must be observable through the existing runtime token/limit machinery regardless of caller. The canonical API must not swallow cancellation or convert it to an ordinary successful empty report.

Partial script/module resolution failure must preserve structured diagnostics and fail according to existing profile policy.

If rule evaluation succeeds but action execution fails, the report/error behavior must remain explicit rather than silently dropping the failure. If current surfaces differ, converge on the richest existing runtime semantics and encode that choice in tests.

Concurrent executions must not share mutable per-run report state. Existing global/process-level TLS-provider initialization may remain one-time and idempotent.

## 9. Compatibility and migration

This is an internal ownership migration, not a feature migration.

Compatibility expectations:

- keep existing public executor/report/profile/resolver types unless removal is clearly safe and separately justified;
- keep `run_cli` and `run_cli_with_profile` public behavior;
- keep `eggsec::nse::*` re-export behavior;
- keep Python class/function names and fields;
- keep feature names and default-off behavior;
- keep built-in script identifiers;
- preserve `NseRunReport` serialization.

New code should prefer the canonical request API. Low-level executor APIs may remain for advanced consumers/tests; production Eggsec adapters should not rebuild orchestration from them.

## 10. Required tests

### Focused unit tests

- request construction/default-independent semantics;
- resolver-first built-in/file/inline policy;
- rule report inclusion;
- stats inclusion;
- library-use reconciliation;
- capability-event inclusion;
- compatibility/fidelity computation;
- evidence extraction;
- cancellation/limit propagation;
- failure-report context.

### Integration tests

- clean-room runtime corpus through canonical API;
- local protocol fixtures;
- Eggsec dispatch/TUI representative built-in execution;
- Eggsec custom script-file path through resolver;
- Python representative built-in and fixture execution.

### Restart and recovery tests

No durable restart behavior is added. Record not applicable in closure evidence.

### Contention and cancellation tests

- concurrent independent runtime requests do not cross-contaminate report state;
- cancellation produces the expected typed failure/report state;
- execution limits remain enforced after orchestration consolidation.

### Security and negative tests

- disallowed script file under safe profile remains denied;
- symlink/path escape remains denied;
- invalid module names remain denied;
- capability denials are retained in report data;
- automated profile cannot silently fall back to manual-permissive execution.

### Migration and compatibility tests

- serialized report contract fixture(s);
- existing public helper compilation;
- TUI/Python surface field parity;
- feature compilation for `nse`, `nse-ssh2`, and `nse-sandbox`.

## 11. Required verification commands

At minimum:

```bash
cargo test -p eggsec-nse --features nse
cargo test -p eggsec --features nse,cli
cargo test -p eggsec-tui --features nse
cargo test -p eggsec-python --features nse
cargo check -p eggsec-nse --features nse
cargo check -p eggsec --features nse,cli
cargo check -p eggsec-tui --features nse
cargo check -p eggsec-python --features nse
make check
```

Because feature wiring and optional native dependencies are involved, also run the repository's feature-matrix/deep check that covers these flags if it is supported in the implementation environment:

```bash
make check-features-individual
```

If `nse-ssh2` cannot run in the environment, compilation still must be verified and the closure record must identify the environmental limitation rather than omitting it.

## 12. Documentation updates

- NSE architecture/integration documentation: identify the canonical request/execution/report pipeline.
- `docs/NSE_COMPATIBILITY.md`: reconcile claims that depend on the unified report path; do not inflate compatibility claims.
- Contributor/skill documentation that currently teaches direct executor/report construction.
- Any API docs for `run_cli_with_profile` and the new high-level request API.

Do not rewrite historical flat-era plans.

## 13. Acceptance criteria

1. A single runtime-owned high-level API performs script resolution through complete `NseRunReport` production.
2. Runtime CLI, Eggsec dispatch/TUI, and Python production paths use that API.
3. Eggsec custom script files no longer bypass `ScriptResolver`.
4. Successful reports consistently contain applicable resolver diagnostics, rule evaluations, execution stats, library-use data, capability events, compatibility/fidelity, output, and evidence.
5. Static `require` fallback/report logic has one owner.
6. Manual and automated profile defaults remain unchanged.
7. Existing report serialization is preserved or any intentional delta is explicitly versioned and documented.
8. Cancellation, limits, sandbox, and negative resolver policy remain enforced.
9. Clean-room corpus tests pass through the canonical path.
10. Feature builds for `nse`, `nse-ssh2`, and `nse-sandbox` remain valid.
11. `make check` passes.
12. No inward Eggsec dependency is added as part of convergence.

## 14. Stop conditions

The agent must stop and report rather than improvise when:

- convergence appears to require moving Eggsec authorization into `eggsec-nse`;
- a required profile default is ambiguous or contradictory across documented supported surfaces;
- preserving `NseRunReport` would require a material schema break;
- the proposed runtime API would import frontend-specific types;
- resolving an execution gap requires redesigning the Eggsec transport contract;
- upstream Nmap source would need to be copied into the test corpus;
- work expands into physical repository extraction.

## 15. Closure evidence required

The closure record must contain:

- implementation commit(s);
- final canonical API/type names;
- before/after production caller inventory;
- evidence that the direct custom-file read path was removed;
- report field parity matrix across runtime CLI adapter, Eggsec manual path, and Python;
- representative serialized report compatibility evidence;
- clean-room corpus result summary;
- cancellation/limit/security-negative test results;
- `nse`/`nse-ssh2`/`nse-sandbox` feature verification;
- `make check` result;
- residual findings classified by severity;
- recommendation for Milestone 002 readiness.

## 16. Handoff notes

Preserve unrelated user changes.

Do not begin by moving files to another repository. The purpose of this pass is to make the current in-tree runtime semantically self-contained before dependency ownership changes.

The current `http_capability.rs` adapter is not evidence that all Lua HTTP-family libraries already use `eggsec-transport`; avoid coupling this milestone to a transport cutover.

Where existing callers expose slightly different errors, prefer a typed runtime error plus caller-specific rendering rather than embedding UI/Python concerns in `eggsec-nse`.
