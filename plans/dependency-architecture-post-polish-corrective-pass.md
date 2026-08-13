# Post-Polish Corrective Plan: Headless Pipeline Parity and Final Closure

## Status

Ready for implementation.

This is a narrow follow-up to the dependency/architecture final polish pass.
The prior pass correctly removed the Python binding's direct dependency on the
engine `cli` feature, removed stale scope helpers, and repaired several closure
records. Final review of the resulting head found that the dependency cleanup
also gated real engine behavior behind `cli`, and that the repository was marked
closed before the final implementation head was validated and before all active
CI documentation was reconciled.

This plan exists to correct those remaining defects only. It must not reopen the
completed dependency-security, authorization, scope/DNS, metadata, daemon,
release, or broad CI work.

## Baseline

Plan against current `main` beginning from:

```text
b28f6711e0a1d9d60f54c9087e17f2af3d9995c1
```

Relevant implementation history immediately before this baseline:

```text
440da70c3ee3998665b756b719bcaa254ee65705
  initial final-polish implementation

b28f6711e0a1d9d60f54c9087e17f2af3d9995c1
  removed eggsec-python -> eggsec/cli and added broad cli gating
```

If implementation begins from a later `main`, record the actual starting SHA and
reconfirm the residuals below before changing code.

## Confirmed residuals

The remaining issues are bounded and concrete:

1. `eggsec-python` now correctly uses `eggsec` with `default-features = false`,
   but the follow-up implementation achieved that partly by feature-gating real
   engine behavior behind `cli`;
2. non-CLI `Pipeline` execution currently returns an explicit configuration error
   for `Stage::Fuzz`, `Stage::LoadTest`, `Stage::Waf`, and `Stage::Recon`, even
   though `Pipeline::new()`, `Pipeline::add_stage()`, and those stage variants are
   public engine/library API;
3. the same follow-up commit changed several `#[cfg(feature = "tool-api")]`
   scanner/recon callback paths to `#[cfg(all(feature = "tool-api", feature =
   "cli"))]`, creating a risk that headless tool/API consumers lost capability
   solely because their parameter type was CLI-owned;
4. other newly added `cli` gates (`runtime_bridge`, distributed worker, load-test
   constructors, recon helpers, scanner helpers) were not individually classified
   as frontend-only versus engine/runtime behavior before being hidden;
5. `CONTRIBUTING.md` still describes exact MSRV and macOS/Windows portability as
   per-PR jobs and still describes an exhaustive CI feature matrix that no longer
   exists;
6. the closure report still records validation against the old `7b878d79` state
   and still contains the obsolete `4442/4443` Python PASS claim;
7. the final-polish plan is marked `Executed` while its completion record still
   contains `<pending final commit>` and does not contain validation evidence for
   the current implementation head;
8. `plans/README.md` currently says the whole line is closed even though the
   issues above remain;
9. hosted CI status for the current head has not been independently verified and
   must not be inferred from local validation.

## Objective

Produce one final state where:

- Python remains free of the engine `cli` feature and Clap dependencies;
- removing CLI parser dependencies does not remove engine/library functionality;
- Fuzz, LoadTest, WAF, and Recon pipeline stages remain executable through plain
  engine configuration in non-CLI builds where they were previously part of the
  engine contract;
- tool/API callback surfaces are gated by actual capability requirements rather
  than by CLI parser availability;
- every `cli` gate introduced by the final dependency-separation commit has an
  explicit and correct ownership disposition;
- active CI documentation matches the actual lightweight workflow contract;
- closure evidence is recorded against the exact final implementation commit;
- the planning index is not marked closed until those checks are complete;
- release cadence and publication remain manual maintainer actions.

## Non-goals

This corrective pass does **not** authorize:

- adding new scanners, stages, transports, tools, or user-visible capabilities;
- changing authorization, target binding, scope policy, DNS rebinding behavior,
  enforcement profiles, or automated-surface restrictions;
- restoring `eggsec-python -> eggsec/cli` as a shortcut;
- moving all command implementation out of `eggsec` merely for source-layout
  purity;
- redesigning the pipeline architecture;
- replacing the command registry or `LegacyWrapped` adapter;
- extracting additional crates;
- broad dependency modernization;
- changing PyO3 0.29, quick-xml 0.41, or MSRV 1.88 absent a concrete build defect;
- removing `indicatif` merely to make the graph smaller; its existing engine use
  is an accepted residual unless this pass proves otherwise;
- re-expanding routine CI with MSRV or macOS/Windows jobs;
- introducing a new test harness, retry framework, feature-matrix framework, or
  generated verification-evidence system;
- automating crates.io, PyPI, TestPyPI, or GitHub Releases;
- adding binary-size regression gates;
- rewriting historical plans to pretend prior intermediate states never existed.

## Required ordering

Execute in this order:

```text
A. classify the b28f `cli` gates
B. restore headless pipeline/tool-api parity with plain engine types
C. verify Python remains parser-independent
D. reconcile active CI and closure documentation
E. validate the exact final implementation SHA
F. record closure without another code change
```

Do not mark the line closed before Workstreams B and E pass.

---

# Workstream A — Classify every new `cli` gate from the dependency-separation commit

## A1. Audit the exact diff rather than searching the whole repository blindly

Start with the changes introduced by:

```text
b28f6711e0a1d9d60f54c9087e17f2af3d9995c1
```

Review each new or broadened `#[cfg(feature = "cli")]` / `cfg(all(..., cli))`
condition added by that commit.

At minimum inspect:

```text
crates/eggsec/src/lib.rs
crates/eggsec/src/types.rs
crates/eggsec/src/fuzzer/config.rs
crates/eggsec/src/distributed/mod.rs
crates/eggsec/src/distributed/worker.rs
crates/eggsec/src/runtime_bridge.rs
crates/eggsec/src/loadtest/mod.rs
crates/eggsec/src/loadtest/runner.rs
crates/eggsec/src/pipeline/executor.rs
crates/eggsec/src/recon/mod.rs
crates/eggsec/src/recon/runner.rs
crates/eggsec/src/scanner/ports/**
crates/eggsec/src/scanner/endpoints.rs
crates/eggsec/src/scanner/fingerprint.rs
```

For each gate, classify the guarded symbol as exactly one of:

```text
1. parser/presentation adapter
2. conversion from CLI type into engine type
3. process-host-only behavior
4. engine/library operation
5. tool/API callback or reusable runtime behavior
```

Only categories 1-3 may remain unconditionally `cli`-gated.

Categories 4-5 must remain available without `cli` unless an independent feature
or platform requirement already justifies their gating.

## A2. Treat type ownership as the source of the coupling

When a reusable function is currently gated only because it accepts a
`crate::cli::*Args` type, do **not** preserve the gate as the solution.

Instead:

1. identify the minimal parser-independent values the function actually needs;
2. define or reuse a plain engine config/request type;
3. convert CLI args into that type in the CLI adapter;
4. have Python/tool/pipeline callers construct the plain type directly;
5. gate only the `From<crate::cli::...>` conversion or parser wrapper.

The existing `FuzzConfig`, `WafConfig`, `WafStressConfig`, `CommonHttpArgs`, and
plain `ScanProfile` pattern should be reused where it fits.

Do not introduce a generic configuration abstraction layer.

## A3. Explicitly audit public API visibility

For every symbol newly hidden by `cli`, determine whether it was previously:

- public and documented;
- re-exported;
- used by another workspace crate;
- available under `tool-api` or another non-CLI feature;
- reachable from Python or daemon/library consumers;
- relied on by tests or examples.

A public symbol must not disappear from non-CLI builds merely because its
historical argument struct carried Clap derives.

If a symbol is intentionally process-host-only, document that ownership in the
implementation summary and leave it gated.

---

# Workstream B — Restore headless pipeline and tool/API capability

## B1. Remove the non-CLI pipeline rejection branch

Current `Pipeline::execute_stage()` has a non-CLI branch that rejects:

```text
Stage::Fuzz
Stage::LoadTest
Stage::Waf
Stage::Recon
```

with a `requires the cli feature` configuration error.

That branch is not an acceptable final state for an engine/library pipeline that
publicly exposes those stage variants.

Target end state:

- `Pipeline::new()` + `add_stage()` may execute those stages in a no-default /
  non-CLI engine build;
- stage execution uses plain engine types;
- CLI parsing remains optional;
- no stage implementation imports Clap types solely to execute.

Delete the synthetic `requires the cli feature` fallback after the underlying
stage implementations compile without CLI types.

## B2. Fuzz and WAF stages — use the plain config types already introduced

The previous polish pass created:

```text
crate::fuzzer::config::FuzzConfig
crate::fuzzer::config::WafConfig
crate::fuzzer::config::WafStressConfig
```

Use these as the engine-facing contract.

Required direction:

```text
CLI FuzzArgs/WafArgs/WafStressArgs
    -> conversion adapter (cli feature)
    -> plain config
    -> fuzzer/WAF engine

Python / Pipeline / tool-api
    -> plain config directly
    -> fuzzer/WAF engine
```

Do not construct CLI argument structs inside pipeline execution.

If pipeline-specific defaults are needed, build the plain config from
`Pipeline` fields and `PipelineContext` in a small helper local to the pipeline
or the owning engine module.

Preserve existing runtime semantics for timeout, concurrency, profile/risk,
common HTTP settings, output suppression, and target selection.

## B3. LoadTest stage — introduce or reuse one parser-independent run config

Audit `LoadTestRunner` and the existing `from_args_with_tui_mode` /
`from_args_with_config` methods.

If an engine config already exists, make it the canonical constructor input.
Otherwise introduce the smallest plain type needed to represent the current
load-test run configuration, for example conceptually:

```text
LoadTestConfig / LoadTestRunConfig
```

It should contain only engine/runtime values, not Clap metadata.

Required direction:

```text
CLI LoadArgs
    -> conversion adapter
    -> plain load-test config
    -> LoadTestRunner

Pipeline/library
    -> plain load-test config
    -> LoadTestRunner
```

Keep `from_args_*` convenience methods behind `cli` if useful, but they may not
be the only path to construct a fully functional runner.

Do not change load-test feature scope or behavior.

## B4. Recon stage — separate execution config from `ReconArgs`

Audit `run_full_recon`, `run_cli`, and `run_cli_with_callback`.

The reusable reconnaissance engine must not require a Clap-derived `ReconArgs`
solely to run.

Preferred shape:

- retain CLI-specific printing/output wrappers behind `cli`;
- expose/reuse a plain recon request/config for engine execution;
- keep result production available to non-CLI consumers;
- make pipeline recon use the plain engine path;
- preserve callback/result paths needed by tool/API consumers without `cli`.

If `run_full_recon` already accepts sufficiently plain values internally, expose
or wrap that existing engine path rather than introducing duplicate recon logic.

## B5. Scanner callback paths — restore `tool-api` ownership

The follow-up commit changed several scanner callbacks from:

```rust
#[cfg(feature = "tool-api")]
```

to:

```rust
#[cfg(all(feature = "tool-api", feature = "cli"))]
```

for ports, endpoints, fingerprinting, and recon paths.

Audit whether `cli` is semantically required.

If the only dependency is a CLI argument struct:

- replace the callback's input with an existing/plain scanner config or request;
- keep a CLI wrapper that converts `*Args` into that config;
- restore the reusable callback to `tool-api` without `cli`.

Do not make headless tools compile in Clap merely to preserve callback behavior.

## B6. Other b28f gates

### `runtime_bridge`

Determine whether this is truly a process-host bridge or reusable engine/runtime
behavior.

- If process-host-only by design: retain `cli` gate and document why.
- If used by daemon/tool/library callers independently of CLI parsing: move the
  parser dependency out and restore the module under its real feature boundary.

### distributed worker

Determine whether `distributed::worker` represents engine runtime behavior or
only CLI orchestration.

- Engine/runtime worker behavior must not be hidden behind `cli`.
- CLI-only launch/parsing wrappers may remain gated.

Do not broaden this into a distributed-system redesign.

## B7. Preserve behavior rather than merely compiling

Do not satisfy this workstream by replacing the current explicit `requires cli`
error with silent no-op behavior.

For each restored stage, preserve the same underlying operation that the CLI
path invokes.

No capability removal is acceptable as a dependency-slimming technique.

---

# Workstream C — Keep the Python boundary clean after restoring engine behavior

## C1. Preserve the current Cargo dependency declaration

`crates/eggsec-python/Cargo.toml` must remain equivalent to:

```toml
eggsec = { path = "../eggsec", default-features = false }
```

plus explicit optional engine features forwarded by Python feature flags.

Do not re-add:

```text
features = ["cli"]
```

## C2. Keep parser crates out of the Python graph

Required final checks:

```bash
cargo tree -p eggsec-python -i clap
cargo tree -p eggsec-python -i clap_complete
```

Both must be not reachable from the supported default Python artifact.

If another transitive path unexpectedly introduces one, fix the ownership path;
do not weaken the acceptance criterion.

## C3. Preserve Python API behavior

Run the existing canonical Python verification after the engine changes:

```bash
make check-python
```

It must exit 0 with no failing tests.

Existing capability, parity, stub, and type checks remain the compatibility
contract. Do not add a second Python verification system.

## C4. Do not require pipeline API expansion in Python

This pass is about preserving existing engine/headless capability and keeping
Python parser-independent. It does not require exposing new pipeline methods or
new Python bindings.

---

# Workstream D — Reconcile active documentation and closure records

## D1. Fix `CONTRIBUTING.md`

Replace the stale CI description with the actual contract:

```text
Routine push/PR (`ci.yml`)
- Rust / ubuntu-latest / make check
- Python / ubuntu-latest / make check-python

Scheduled/manual (`deep-checks.yml`)
- make check-full
- exact MSRV 1.88
- macOS/Windows portability compile checks
```

Remove claims that:

- MSRV runs on every PR;
- macOS/Windows run on every PR;
- all feature combinations are tested through a matrix strategy.

If representative feature profiles are described, make clear they live in the
broad/deep validation boundary rather than the routine merge loop.

## D2. Update the existing closure report, not a new one

`plans/dependency-architecture-simplification-closure-report.md` must be updated
after code validation.

Required corrections:

- add the final-polish and post-polish corrective pass to the status/history;
- remove the obsolete `7b878d79` final-validation identity;
- remove the obsolete `4442/4443 PASS` Python statement;
- record the exact final validated implementation SHA;
- state the final Python/CLI dependency result;
- state the final headless pipeline result;
- preserve historical artifact measurements with their original measurement SHA
  unless this pass materially rebuilds/re-measures them;
- do not pretend historical measurements were taken at the new final SHA;
- keep hosted CI `NOT VERIFIED` unless an actual run was inspected.

## D3. Correct the final-polish completion record

`plans/dependency-architecture-final-polish-pass.md` is historical after this
follow-up.

Its completion record should no longer contain placeholder values such as:

```text
implementation SHA: <pending final commit>
```

Record the actual commits that implemented that pass:

```text
440da70c...
b28f6711...
```

and accurately note that final review found the headless capability regression
that required this follow-up plan.

Do not rewrite its original objectives or erase the fact that an additional
corrective pass was needed.

## D4. Reopen the planning index during implementation

Until this plan is executed, `plans/README.md` must not say:

```text
This line is now closed.
```

Register this document as the active corrective handoff and describe the prior
A-J/corrective/final-polish work as substantially complete but awaiting this
specific closure correction.

After successful implementation and exact-SHA validation, update the same index
to close the line with no active handoff.

## D5. Reconcile pipeline/library docs with final capability

Inspect active pipeline documentation and examples.

If Fuzz, LoadTest, WAF, and Recon remain listed as pipeline stages, they must work
in the supported engine/headless configuration represented by that documentation.

CLI-specific examples may remain CLI-specific, but documentation must distinguish
constructor/parser convenience from engine capability.

Do not document a capability as generic engine behavior while the implementation
rejects it without `cli`.

---

# Workstream E — Final validation against the exact implementation head

## E1. Validate after all executable changes are committed

Use this sequence:

1. implement Workstreams A-D;
2. run focused checks during development;
3. commit all executable code, manifests, scripts, and active documentation needed
   to describe the implementation;
4. record the implementation SHA;
5. confirm a clean tree;
6. run all required final validation against that exact SHA;
7. if any code/config/script/workflow change is required, create a new commit and
   restart final validation against the new SHA;
8. only after validation passes, make at most one closure-record-only commit.

A closure-record-only commit may update plan status/result tables and the validated
SHA. It must not modify Rust/Python code, manifests, scripts, Makefile, or
workflows.

## E2. Required core validation

Run on the exact final implementation SHA:

```bash
git rev-parse HEAD
git status --porcelain
cargo fmt --all -- --check
make check
make check-python
cargo deny check advisories
cargo +1.88 check --workspace --no-default-features
cargo check -p eggsec --no-default-features
cargo check -p eggsec --no-default-features --features tool-api
cargo check -p eggsec --no-default-features --features tool-api,rest-api
cargo check -p eggsec-python
```

`make check-python` must exit 0. A partial test count is not PASS.

## E3. Required dependency-boundary validation

Run:

```bash
cargo tree -p eggsec-python -i clap
cargo tree -p eggsec-python -i clap_complete
cargo tree -p eggsec-python -e features
```

Record Clap/Clap-complete as `not reachable` when Cargo reports no inverse path.
Do not treat an expected nonzero `cargo tree -i` result as a failed boundary.

`indicatif` may remain reachable because its engine ownership was already
accepted; do not turn that accepted residual into a new blocker.

## E4. Required headless pipeline validation

At minimum prove all four affected stages are no longer compile/runtime-gated by
`cli` solely for parser reasons:

```text
Fuzz
LoadTest
Waf
Recon
```

Preferred validation, using existing test infrastructure:

- add focused no-default/unit tests around plain config construction and pipeline
  stage dispatch where deterministic execution is already possible;
- use existing mocks/fixtures rather than live Internet access;
- confirm there is no `requires the cli feature` fallback for those stages;
- confirm each stage resolves to its real engine execution path in a non-CLI
  build.

Do not create a new pipeline test harness solely for this pass. If a stage cannot
be safely executed in tests without network or privileged side effects, use the
smallest compile/dispatch/config test that proves the engine path exists and rely
on the existing component tests for operation semantics.

Required direct search:

```bash
rg -n 'requires the `cli` feature|requires the cli feature' crates/eggsec/src
```

No result may represent Fuzz/LoadTest/WAF/Recon engine capability being disabled
solely because CLI parsing is absent.

## E5. Tool/API boundary validation

Compile the non-CLI tool/API profiles above and run existing focused tests for
scanner/recon callbacks if present.

Acceptance requires that a callback/API previously available under `tool-api`
was not silently changed to require `cli` unless it is genuinely a CLI-only
wrapper.

If a public callback is replaced by a plain-config equivalent, update internal
callers/tests and document the compatibility rationale.

Do not add Clap back into the `tool-api` graph merely to preserve the old
signature.

## E6. Workflow/release inspection

Verify the workflow contract remains unchanged by this pass:

```bash
rg -n 'cargo publish|maturin publish|twine upload|gh release|id-token: write|tags:' \
  .github/workflows
```

Inspect `.github/workflows/ci.yml` and `deep-checks.yml` directly.

Required final state:

```text
routine CI: Rust + Python on Linux
scheduled/manual: check-full + exact MSRV + macOS/Windows portability
publication: manual only
```

Do not add release permissions or registry credentials.

## E7. Hosted CI evidence

If an actual hosted Actions run for the final SHA is available and inspected,
record its result.

Otherwise record:

```text
hosted CI: NOT VERIFIED
```

Do not infer PASS from local checks or commit metadata.

---

# Workstream F — Close the planning record once, without another implementation pass

After Workstream E passes:

1. mark this plan `Executed`;
2. fill its completion record with the exact validated implementation SHA;
3. if a documentation-only closure commit follows, record that SHA separately;
4. update `plans/README.md` to say the dependency/architecture simplification
   line is closed and has no active corrective handoff;
5. update the existing closure report with the final validation result;
6. leave A-J, the first corrective pass, and the final-polish pass in place as
   historical engineering records;
7. do not create another closure report or another plan unless final validation
   exposes a genuinely new defect.

---

# Explicit acceptance criteria

This plan is complete only when all applicable criteria below are satisfied.

## Headless engine/pipeline parity

1. `Pipeline::new()` remains available without the `cli` feature.
2. `Pipeline::add_stage()` remains available without the `cli` feature.
3. `Stage::Fuzz` does not fail solely because `cli` is disabled.
4. `Stage::LoadTest` does not fail solely because `cli` is disabled.
5. `Stage::Waf` does not fail solely because `cli` is disabled.
6. `Stage::Recon` does not fail solely because `cli` is disabled.
7. the synthetic non-CLI `requires the cli feature` fallback for those stages is
   removed.
8. Fuzz pipeline execution uses plain `FuzzConfig` or an equivalent parser-free
   engine type.
9. WAF pipeline execution uses plain `WafConfig`/engine types rather than
   `crate::cli::WafArgs`.
10. LoadTest has a parser-independent construction/execution path.
11. Recon has a parser-independent construction/execution path.
12. no stage is converted into a no-op to satisfy dependency separation.
13. existing risk, timeout, concurrency, target, and common HTTP semantics are
    preserved.

## `cli` gate ownership

14. every `cli` gate introduced or broadened by `b28f6711` is classified.
15. parser derives and CLI conversion adapters may remain `cli`-gated.
16. process-host-only behavior may remain `cli`-gated with explicit rationale.
17. engine/library operations are not hidden behind `cli` merely because their
    former arguments were Clap-derived.
18. `tool-api` callback capability is not made CLI-dependent solely for argument
    type convenience.
19. `runtime_bridge` has an explicit retain-or-restore disposition based on actual
    ownership.
20. distributed worker behavior has an explicit retain-or-restore disposition
    based on actual ownership.

## Python boundary

21. `eggsec-python` does not enable `eggsec/cli`.
22. `clap` is not reachable from the default Python graph.
23. `clap_complete` is not reachable from the default Python graph.
24. Python uses plain engine types for reusable Fuzz/WAF configuration.
25. existing Python public API names/signatures remain unchanged unless an
    existing test proves a correction is required.
26. `make check-python` exits 0 on the final implementation SHA.
27. all canonical Python checks reach their normal success terminus with no
    failing pytest tests.

## Build and verification

28. `cargo fmt --all -- --check` passes.
29. `make check` passes on the final implementation SHA.
30. `cargo deny check advisories` passes with only documented retained exceptions.
31. Rust 1.88 no-default workspace validation passes.
32. `cargo check -p eggsec --no-default-features` passes.
33. `cargo check -p eggsec --no-default-features --features tool-api` passes.
34. `cargo check -p eggsec --no-default-features --features tool-api,rest-api`
    passes.
35. `cargo check -p eggsec-python` passes.
36. focused existing tests or minimal new tests prove the affected pipeline paths
    are present without `cli`.
37. no retry framework or broad new matrix is introduced.

## Documentation and planning record

38. `CONTRIBUTING.md` no longer says MSRV runs on every PR.
39. `CONTRIBUTING.md` no longer says macOS/Windows portability runs on every PR.
40. active docs no longer claim exhaustive CI feature-matrix testing if the
    workflow does not do that.
41. pipeline docs distinguish CLI parser convenience from engine capability.
42. the closure report no longer records `7b878d79` as the final validated head.
43. the closure report no longer records `4442/4443` as a successful final Python
    result.
44. the closure report records the exact final implementation SHA.
45. the closure report records headless pipeline parity outcome.
46. the final-polish plan contains its real implementation SHAs rather than
    `<pending>` placeholders.
47. the final-polish history accurately notes that this follow-up was required.
48. `plans/README.md` identifies this plan as active until implementation passes.
49. after implementation, `plans/README.md` records no active corrective handoff.
50. hosted CI is `NOT VERIFIED` unless an actual run for the final SHA is
    inspected.

## CI/release scope

51. routine CI remains Linux Rust `make check` plus Python `make check-python`.
52. exact MSRV remains scheduled/manual.
53. macOS/Windows portability remains scheduled/manual.
54. `make check-full` remains broad weekly/manual validation rather than a routine
    merge requirement.
55. no workflow publishes Rust or Python packages.
56. no tag-triggered release automation is added.
57. no release-oriented OIDC permission is added.
58. crates.io/PyPI/GitHub release cadence remains manual.

## Scope control

59. no authorization/enforcement semantic change is introduced.
60. no scope/DNS policy change is introduced.
61. no new user-facing feature is introduced.
62. no broad pipeline rewrite is introduced.
63. no new crate extraction is introduced.
64. no unrelated dependency modernization is introduced.
65. `indicatif` is not treated as a blocker solely because it remains in the
    engine graph for legitimate engine progress reporting.
66. the pass ends after one validated closure state rather than creating another
    planning layer for already-resolved work.

---

# Handoff completion record

```text
final status: Executed
starting SHA: b28f6711e0a1d9d60f54c9087e17f2af3d9995c1
validated implementation SHA: 2966ebc5878215de3a5ca78c1d2339af69d057b7
closure-record-only SHA: 11db32b53b8f335c2d3f5a3e666441b779fda952
Fuzz non-cli pipeline: PASS — Pipeline::run_fuzz constructs FuzzConfig directly; FuzzConfig gained Default so dispatch::fuzzer::run_fuzz no longer needs cli types
LoadTest non-cli pipeline: PASS — Pipeline::run_load_test uses LoadTestRunner::from_config_with_engine with LoadTestRunConfig
WAF non-cli pipeline: PASS — Pipeline::run_waf constructs WafConfig and drives waf::WafEngine::new(WafConfig).run()
Recon non-cli pipeline: PASS — Pipeline::run_recon uses recon::runner::run_full_recon_from_request(&ReconRequest, ...)
tool-api callback disposition: RESTORED — PortScanRequest/EndpointScanRequest/FingerprintRequest introduced; run_with_callback takes plain requests; run_cli_with_callback remains as CLI wrapper gated by both tool-api and cli
runtime_bridge disposition: RETAIN cli gate — runtime_bridge depends on dispatch which uses cli-gated worker types in dispatch::fuzzer/dispatch::recon; the bridge itself is process-host and not parser-dependent, but its caller (dispatch) requires cli to compile
distributed worker disposition: RESTORED — distributed::worker no longer gated by cli; worker modules use plain FuzzConfig/WafConfig/LoadTestRunConfig/ReconRequest directly
eggsec-python cli feature: absent
clap reachable from Python: not reachable (cargo tree -i returns no inverse path)
clap_complete reachable from Python: not reachable (cargo tree -i returns no inverse path)
make check: PASS
make check-python: PASS (all canonical Python checks reach their normal success terminus)
cargo deny advisories: PASS (3 retained documented exceptions)
MSRV 1.88: PASS (cargo +1.88 check --workspace --no-default-features)
no-default engine: PASS (cargo check -p eggsec --no-default-features)
no-default tool-api: PASS (cargo check -p eggsec --no-default-features --features tool-api)
tool-api+rest-api: PASS (cargo check -p eggsec --no-default-features --features tool-api,rest-api)
eggsec-python: PASS (cargo check -p eggsec-python)
hosted CI: NOT VERIFIED — only local validation was performed
publication status: NOT RUN
```
