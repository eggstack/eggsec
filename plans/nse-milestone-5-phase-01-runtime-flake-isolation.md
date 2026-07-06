# NSE Milestone 5 Phase 01: Runtime Corpus Flake Isolation

## Purpose

Stabilize `runtime_corpus_tests.rs` under the chosen test execution strategy before tightening assertions or adding deeper protocol fixtures.

Milestone 4 documented an intermittent high-parallelism flake where the `process-denied` fixture occasionally reports no `process_exec` capability event. The suspected cause is cross-test interaction with runtime-global Lua/library state such as `nmap._ports`, library-level static state, or shared temp directories.

## Non-Goals

Do not expand corpus coverage in this phase.

Do not relax AgentSafe/CiSafe policy.

Do not hide flakes by weakening expectations.

Do not require public internet.

## Workstream 1: Reproduce and Characterize the Flake

### Steps

1. Run the runtime corpus repeatedly at different parallelism levels:

```bash
cargo test -p eggsec-nse --features nse --test runtime_corpus_tests -- --test-threads=1
cargo test -p eggsec-nse --features nse --test runtime_corpus_tests -- --test-threads=4
cargo test -p eggsec-nse --features nse --test runtime_corpus_tests
```

2. Repeat each at least 10 times if feasible.
3. Record which fixture fails, expected event, observed events, resolver diagnostics, rules, errors, and output.
4. Add targeted tracing/log output behind `RUST_LOG` or test-only debug helpers; do not print noisy logs on success.

### Acceptance Criteria

- The flake has a written reproduction matrix.
- The repo docs state whether the issue reproduces only at default parallelism or also under `--test-threads=4`.

## Workstream 2: Isolate Shared State

### Investigation Targets

Inspect and isolate:

- Lua global `nmap._ports` initialization and mutation;
- per-executor library registration state;
- `http.HTTP_CLIENT` or other library-level statics;
- temp directory naming and cleanup;
- shared module roots;
- cached required modules;
- global runtime state inside libraries such as `io`, `os`, `nmap`, `socket`, `http`, `comm`.

### Required Fixes

Prefer deterministic isolation over test serialization:

- unique temp directory per fixture and per test process/thread;
- fresh `NseExecutor::with_profile()` for every fixture;
- no reuse of Lua global state across fixtures;
- clear `required_modules`, capability events, counters, and `nmap._ports` before each run;
- isolate or reset any library-level static caches that can affect tests.

### Acceptance Criteria

- The known `process-denied` flake no longer reproduces under default parallelism, or the remaining reason is documented and technically justified.

## Workstream 3: Add Fixture-Level Diagnostics

### Required Diagnostics

When a runtime fixture fails, the failure message should include:

- fixture id;
- profile kind;
- expected status/fidelity;
- observed status/fidelity;
- expected libraries/rules/capability events;
- observed libraries/rules/capability events;
- resolver diagnostics;
- report errors;
- output excerpt.

### Acceptance Criteria

- Flake triage does not require rerunning with ad hoc debug code.

## Workstream 4: Choose Execution Strategy

### Preferred Outcome

Default parallel `cargo test` passes reliably.

### Fallback Outcome

If runtime corpus must be serialized because upstream Lua/global-state behavior cannot be isolated without major redesign:

1. Document the reason in `architecture/nse_integration.md`.
2. Add a wrapper script or make target:

```bash
cargo test -p eggsec-nse --features nse --test runtime_corpus_tests -- --test-threads=1
```

3. Update docs and CI guidance to run runtime corpus serialized.
4. Keep unit tests parallel where safe.

### Acceptance Criteria

- The chosen strategy is explicit and enforced by docs or scripts.
- The repo does not claim default-parallel stability if it is not true.

## Workstream 5: Guards and Docs

Update guards if needed:

- check for runtime corpus test binary existence;
- check for `NseExecutor::with_profile()` usage;
- optionally check for unique temp-dir pattern;
- optionally check that runtime corpus command appears in verification docs with the correct thread strategy.

Update docs:

- `architecture/nse_integration.md` Milestone 5 Phase 01 note;
- `.opencode/skills/eggsec-nse/SKILL.md` runtime corpus guidance;
- `AGENTS.md` if needed.

## Verification

Run:

```bash
cargo test -p eggsec-nse --features nse --test runtime_corpus_tests -- --test-threads=1
cargo test -p eggsec-nse --features nse --test runtime_corpus_tests -- --test-threads=4
cargo test -p eggsec-nse --features nse --test runtime_corpus_tests
cargo test -p eggsec-nse --features nse --test runtime_smoke_tests
bash scripts/check-architecture-guards.sh
```

## Final Acceptance Criteria

Phase 01 is complete when:

- The runtime corpus flake is fixed or explicitly serialized with rationale.
- Failure diagnostics are detailed enough for fixture triage.
- Verification records include the chosen thread strategy.
- Later Milestone 5 phases can tighten assertions without fighting nondeterministic failures.