# Dispatch Profile Parity Corrective Pass

## Status

Ready for implementation.

This is a final narrow follow-up to the dependency/architecture simplification
line. The post-polish corrective pass successfully restored parser-independent
headless execution for Fuzz, LoadTest, WAF, Recon, scanner callbacks, and
related worker paths. Review of the resulting implementation found one remaining
behavioral defect: dispatch and tool-API pipeline construction no longer preserve
the requested `ScanProfile` as canonical pipeline state.

This pass must correct that defect without reopening the completed dependency,
security, authorization, CI, daemon, or release work.

## Baseline

Plan against current `main`:

```text
a106d920a117395e9af48e0e26de12b05a4f3694
```

The implementation that introduced the remaining behavior is:

```text
2966ebc5878215de3a5ca78c1d2339af69d057b7
  Post-polish corrective pass: restore headless pipeline/tool-API parity
```

If implementation begins from a later `main`, record the actual starting SHA
and reconfirm every residual below before changing code.

## Confirmed residuals

### 1. Dispatch pipeline profile state is not preserved

`crates/eggsec/src/dispatch/recon.rs::run_pipeline()` currently starts with:

```rust
let pipeline = Pipeline::new(&target).with_concurrency(10);
```

`Pipeline::new()` initializes:

```text
profile = ScanProfile::Quick
risk_budget = Quick.max_risk_budget()
stages = []
```

The dispatch function then manually appends stages for non-Quick profiles. It
never updates the pipeline's internal `profile` or `risk_budget`.

Consequences:

- Quick dispatch runs an empty pipeline instead of the canonical Quick stages;
- non-Quick dispatch keeps Quick's `SafeActive` risk budget;
- intrusive/stress stages may be skipped even when the requested profile allows
  them;
- profile-specific scope and compile-time feature validation can observe Quick
  rather than the requested profile;
- behavior can diverge from CLI/library construction even though the same
  `ScanProfile` was requested.

### 2. Dispatch duplicates the canonical profile-to-stage mapping

`dispatch/recon.rs` contains its own hand-written `ScanProfile -> Vec<Stage>`
match even though `Stage::from_profile()` is already the canonical mapping.

The two mappings already differ materially. Examples at this baseline include:

```text
Quick
  canonical: PortScan, Fingerprint
  dispatch:  no stages

Full
  canonical: PortScan, Fingerprint, EndpointScan, Fuzz, LoadTest
  dispatch:  PortScan, Fingerprint, EndpointScan, Fuzz, LoadTest, Waf, Recon, Vuln

Recon
  canonical: PortScan, Fingerprint, EndpointScan, Recon, Fuzz
  dispatch:  Recon, PortScan

DefenseLab
  canonical: PortScan, Fingerprint, EndpointScan, Waf, Fuzz
  dispatch:  PortScan
```

This duplicate mapping must be removed rather than synchronized manually.

### 3. Tool-API pipeline profile is parsed but ignored

`crates/eggsec/src/tool/implementations/pipeline.rs` parses the requested profile
into `profile_enum` and uses it to estimate stage count, but execution calls:

```rust
crate::pipeline::run_with_callback(target, &config, callback)
```

The helper currently constructs:

```rust
let pipeline = Pipeline::new(target);
```

which contains no stages. Therefore the tool-API `scan` path can report success
without executing the requested profile pipeline.

### 4. Dispatch output parameters are silently discarded

The post-polish implementation currently contains:

```rust
let _ = output_file;
let _ = output_format;
```

The earlier path fed those values through `ScanArgs`. `output_file` at minimum
participated in session/checkpoint-path behavior; `output_format` may have been a
presentation-only/dead parameter in this internal dispatch path.

This pass must establish the actual contract and either preserve meaningful
behavior or remove truly dead parameters coherently. Silently accepting and
ignoring them is not an acceptable final state.

### 5. Closure records are ahead of behavioral validation

The planning index and closure report currently describe the entire line as
closed and validated at `2966ebc...`, but the validation matrix did not include
profile-state/stage-selection behavior and therefore did not catch the defects
above.

The post-polish plan also still contains a literal:

```text
closure-record-only SHA: <this commit>
```

instead of the actual documentation-only SHA.

## Objective

Reach one final state where:

- `ScanProfile` is the single source of truth for pipeline profile state, stage
  selection, risk budget, and profile-specific validation;
- dispatch and tool-API callers use a parser-independent canonical constructor;
- Quick, Full, Recon, defense-lab, and all other profiles use
  `Stage::from_profile()` rather than a second mapping;
- no caller can request one profile while the `Pipeline` internally remains
  Quick;
- tool-API `scan` executes the requested profile rather than an empty pipeline;
- meaningful dispatch output/session semantics are preserved, while genuinely
  dead parameters are removed instead of ignored;
- Python remains independent of `cli`/Clap;
- existing headless Fuzz/LoadTest/WAF/Recon parity remains intact;
- final evidence includes behavioral tests, not compile checks alone;
- the closure record names the exact validated implementation SHA and any
  documentation-only closure SHA separately.

## Non-goals

Do **not** use this pass to:

- redesign the pipeline execution architecture;
- add new scan profiles or change the intended stage membership of existing
  profiles;
- change `Stage::from_profile()` semantics unless a pre-existing canonical bug is
  proven by current docs/tests;
- change authorization, scope/DNS policy, target binding, enforcement profiles,
  or automated-surface restrictions;
- restore Python's dependency on `eggsec/cli`;
- move more code between crates merely for layout purity;
- alter PyO3, quick-xml, MSRV, TLS, SQLite, or other dependency families;
- redesign dispatch/task protocol types;
- expand routine CI or add a feature matrix;
- add retry infrastructure or generated verification evidence;
- automate crates.io, PyPI, TestPyPI, or GitHub Releases;
- remeasure all release artifacts unless this narrow change materially alters
  their dependency graph or build profile;
- create another roadmap.

## Required ordering

Execute in this order:

```text
A. establish one parser-independent profile constructor
B. migrate dispatch pipeline construction
C. migrate tool-API profile execution
D. resolve output/session parameter semantics
E. add focused behavioral regression tests
F. reconcile closure records
G. validate the exact implementation SHA
```

Do not mark the line closed before Workstreams E and G pass.

---

# Workstream A — Establish canonical parser-independent profile construction

## A1. Add one non-CLI constructor for a complete profile pipeline

Add the smallest parser-independent construction path needed to create a
`Pipeline` from a `ScanProfile`.

Preferred shape:

```rust
impl Pipeline {
    pub fn from_profile(target: &str, profile: ScanProfile) -> Self {
        // initialize canonical state
    }
}
```

An equivalent name is acceptable if it matches established API naming, but do
not introduce a generalized builder framework.

The constructor must initialize, from the same `profile` value:

```text
target
profile
risk_budget = profile.max_risk_budget()
stages = Stage::from_profile(profile)
default concurrency/common/spoof/context/session/config state
```

It must be available without the `cli` feature.

## A2. Keep `Pipeline::new()` semantics explicit

`Pipeline::new(target)` is currently useful as a manual empty-pipeline builder
for callers that intend to call `add_stage()`.

Do not silently change it to mean "Quick profile" unless repository-wide call
site review proves that behavior is safe and preferable.

Preferred outcome:

```text
Pipeline::new(target)
  = manual/empty pipeline construction

Pipeline::from_profile(target, profile)
  = canonical predefined profile construction
```

Document that distinction briefly in rustdoc.

## A3. Use one initialization helper only if it reduces duplication

It is acceptable to implement a small private initializer used by both
`new()` and `from_profile()`. Do not create a hierarchy of config/build traits.

## A4. Preserve CLI behavior through the same canonical state

Audit `Pipeline::from_args*()`.

Where CLI args select a predefined profile and do not explicitly override the
stage list, initialize profile/stages/risk through the same canonical
`ScanProfile` path.

If explicit `--stages` overrides are supported, preserve those overrides while
keeping `self.profile` and `risk_budget` derived from the selected profile.

The CLI wrapper may remain `cli`-gated. The profile constructor must not be.

---

# Workstream B — Fix dispatch pipeline construction

## B1. Delete the duplicate profile-to-stage match

Remove the hand-written `ScanProfile -> Vec<Stage>` mapping from
`dispatch/recon.rs`.

Dispatch must obtain predefined stages through the canonical path, preferably:

```rust
let pipeline = Pipeline::from_profile(&target, profile);
```

No second profile mapping may remain in dispatch.

## B2. Preserve requested profile state

For every dispatch profile:

```text
pipeline.profile == requested profile
pipeline.risk_budget == requested_profile.max_risk_budget()
pipeline.stages == Stage::from_profile(requested_profile)
```

This must hold before execution.

## B3. Preserve dispatch-specific runtime settings

The historical dispatch path used CLI construction with TUI/progress suppression
semantics and a fixed/default concurrency path.

Audit and preserve only the settings that were semantically relevant:

- concurrency;
- TUI/progress mode;
- engine configuration if supplied/required;
- common HTTP defaults;
- checkpoint/session path behavior where applicable.

If a tiny parser-independent setter is needed (for example `with_tui_mode`), add
only that setter rather than reintroducing `ScanArgs`.

## B4. Preserve risk enforcement rather than bypassing it

Do not solve profile parity by disabling `validate_stage_risk()`.

The requested profile's normal `max_risk_budget()` must continue to decide
whether a stage can execute.

Examples that must remain true:

```text
Quick -> SafeActive budget
Web/Waf/Recon/etc. -> Intrusive where defined
Full/Api/Deep -> Stress where defined
Stealth -> Passive
```

## B5. Preserve profile-specific scope/feature validation

The dispatch-created pipeline must expose the real requested `self.profile` to:

```text
validate_defense_lab_scope()
validate_feature_gates()
```

Defense-lab/private-scope profiles must not masquerade as Quick.
`ProtocolEdge`/`NseSafe` feature checks must likewise observe their real profile.

---

# Workstream C — Fix tool-API profile execution

## C1. Add a profile-aware callback entry point

The tool API needs a parser-independent helper that accepts `ScanProfile`.

Preferred shape:

```rust
#[cfg(feature = "tool-api")]
pub async fn run_with_callback_for_profile<F>(
    target: &str,
    profile: ScanProfile,
    config: &EggsecConfig,
    callback: F,
) -> Result<()>
```

An equivalent API is acceptable if simpler.

It must use `Pipeline::from_profile()` (or the equivalent canonical constructor).

## C2. Preserve the existing callback API if it is public

If `run_with_callback(target, config, callback)` is an existing supported public
API, retain it as a compatibility wrapper with explicit Quick semantics:

```text
run_with_callback(...)
  -> run_with_callback_for_profile(..., ScanProfile::Quick, ...)
```

Do not silently change its signature unless it is proven private/internal.

## C3. Pass `PipelineTool`'s parsed profile into execution

`PipelineTool::execute()` already parses `profile_enum`.

Use that exact value for both:

- stage-count/timeout estimation;
- actual pipeline construction/execution.

The requested profile must not be parsed for metadata and then discarded.

## C4. Ensure Quick actually executes Quick

The tool-API default profile is Quick. It must execute canonical Quick stages:

```text
PortScan
Fingerprint
```

It must not return success from an empty stage list.

## C5. Preserve callback/report behavior

Keep existing finding callbacks, timeout handling, response metadata, and error
mapping. This is a construction correction, not a tool-response redesign.

---

# Workstream D — Resolve dispatch output/session parameter semantics

## D1. Determine the real pre-regression behavior

Compare `dispatch/recon.rs::run_pipeline()` before and after `2966ebc...`.

Establish exactly what these parameters previously affected:

```text
output_file
output_format
```

Do not assume they wrote a final report if the old path only used them for
session/checkpoint configuration.

## D2. Preserve meaningful behavior

If `output_file` previously affected session/checkpoint persistence, preserve
that behavior through parser-independent pipeline configuration.

If final report output was part of the dispatch contract, route it through the
existing pipeline output writer rather than duplicating serializers.

## D3. Remove truly dead parameters instead of ignoring them

If `output_format` (or either parameter) is proven to have had no observable
behavior in this dispatch function and no caller relies on it:

- remove it from the internal function signature;
- update every caller in the same pass;
- update tests/docs as needed.

Do not leave:

```rust
let _ = output_file;
let _ = output_format;
```

as the final disposition.

## D4. Do not expand output architecture

Do not introduce a new report abstraction, output service, or persistence crate.
Reuse the current `pipeline::write_output`/report/session mechanisms where
appropriate.

---

# Workstream E — Add focused behavioral regression tests

Compile-only validation was insufficient for this defect. Add small deterministic
tests around construction and routing.

Prefer unit tests inside the owning modules so private state can be asserted
without exposing new public getters solely for tests.

## E1. Canonical profile constructor tests

At minimum test:

```text
Quick
Full
Recon
DefenseLab
```

For each, assert:

```text
stored profile == requested profile
stored risk budget == profile.max_risk_budget()
stages == Stage::from_profile(profile)
```

Add `ProtocolEdge` or `NseSafe` if needed to prove compile-time feature validation
observes the requested profile.

## E2. Quick regression test

Add an explicit regression assertion that canonical Quick construction contains:

```text
[PortScan, Fingerprint]
```

and is not empty.

## E3. Dispatch construction test

Factor only enough pure construction logic from `dispatch::recon::run_pipeline()`
to test that dispatch uses the canonical profile state without performing real
network scans.

Do not build a mock pipeline framework.

At minimum prove:

```text
dispatch Quick -> canonical Quick stages
dispatch Full -> canonical Full stages
dispatch Recon -> canonical Recon stages
```

## E4. Risk-budget regression test

Prove that a dispatch/profile-built Full pipeline has Stress budget and a Quick
pipeline has SafeActive budget.

This is the direct regression guard for the bug where all dispatch profiles
remained internally Quick.

## E5. Defense-lab/profile validation regression

Using deterministic/private helper tests, prove a defense-lab profile still
engages its private-scope validation when built through the parser-independent
profile constructor.

Do not perform external network access.

## E6. Tool-API profile forwarding test

Add a focused test around the pure construction/helper boundary showing that the
profile parsed by `PipelineTool` reaches profile-aware pipeline construction.

If testing full `SecurityTool::execute()` would require network activity, test the
profile-aware helper or extracted constructor instead.

## E7. Output/session behavior test

If Workstream D retains output/session semantics, add one deterministic test for
the preserved behavior. If a dead parameter is removed instead, compile/call-site
tests are sufficient.

---

# Workstream F — Reconcile documentation and closure records

## F1. Reopen the planning index during implementation

Update `plans/README.md` to state that the dependency/architecture line has one
active narrow dispatch-profile parity handoff:

```text
dependency-architecture-dispatch-profile-parity-corrective-pass.md
```

Do not describe the broader A-J/security/CI work as reopened.

## F2. Correct the post-polish plan's closure SHA

Replace:

```text
closure-record-only SHA: <this commit>
```

with the actual documentation-only closure SHA already established in history
(`11db32b53b8f335c2d3f5a3e666441b779fda952`, or the exact intended SHA after
verifying the record).

Preserve the rest of that plan as historical evidence.

## F3. Update the existing closure report only after behavioral validation

Do not create another closure report.

After Workstream G passes, update
`plans/dependency-architecture-simplification-closure-report.md` to:

- add this dispatch-profile parity correction to the corrective history;
- record the new exact validated implementation SHA;
- state that dispatch/tool-API profile state and stage selection were behaviorally
  validated;
- preserve old artifact-size measurements under their original SHA;
- preserve previous security/dependency evidence rather than rerunning unrelated
  narrative work;
- keep hosted CI as `NOT VERIFIED` unless an actual run is inspected.

## F4. Close the planning index only after validation

After the implementation SHA passes all required checks, update
`plans/README.md` once more to say there is no active corrective handoff.

Do not mark it closed in the implementation commit before validation evidence is
known.

---

# Workstream G — Validate the exact final implementation SHA

## G1. Validate behavior and build state before closure docs

Commit the code/test implementation first, then validate that exact SHA with a
clean tree.

Required commands:

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

Also run the focused profile/dispatch/tool-API regression tests added by this
pass directly and record them by test target/name.

## G2. Reconfirm Python remains parser-independent

Run:

```bash
cargo tree -p eggsec-python -i clap
cargo tree -p eggsec-python -i clap_complete
```

Both must remain not reachable.

Do not re-add `eggsec/cli` to Python as part of the fix.

## G3. Reconfirm the synthetic headless regression remains gone

Search for the previously introduced fallback:

```bash
rg -n 'requires the .cli. feature|requires the cli feature' crates/eggsec/src
```

No Fuzz/LoadTest/WAF/Recon engine-stage rejection should reappear.

## G4. Reconfirm no duplicate profile map remains in dispatch

Use focused review/search to verify that dispatch does not carry a second
`ScanProfile -> Vec<Stage>` match.

`Stage::from_profile()` must remain the canonical predefined-profile mapping.

## G5. Reconfirm lightweight CI/manual release policy

Inspect `.github/workflows/ci.yml` and `deep-checks.yml` and confirm:

```text
routine: Rust make check + Python make check-python
scheduled/manual: check-full + MSRV + macOS/Windows
publication: none
```

Run:

```bash
rg -n 'cargo publish|maturin publish|twine upload|gh release|id-token: write|tags:' \
  .github/workflows
```

Do not modify workflows unless final validation finds a direct break caused by
this pass.

## G6. Hosted CI evidence

If an actual hosted run for the exact implementation SHA is available and
inspected, record it.

Otherwise:

```text
hosted CI: NOT VERIFIED
```

Local PASS does not imply hosted PASS.

## G7. Closure-record-only commit

After validation, a documentation-only closure commit may update the plan,
planning index, and closure report.

Record separately:

```text
validated implementation SHA
closure-record-only SHA
```

Verify that the closure-only commit changes no Rust/Python source, manifests,
scripts, Makefile, or workflows.

---

# Explicit acceptance criteria

This pass is complete only when all applicable criteria below are true.

## Canonical profile construction

1. A parser-independent predefined-profile constructor exists for `Pipeline`.
2. It is available without the `cli` feature.
3. It stores the requested `ScanProfile` unchanged.
4. It derives `risk_budget` from that same profile.
5. It derives stages exclusively through `Stage::from_profile(profile)`.
6. `Pipeline::new()` manual/empty semantics are either preserved or any change is
   proven safe across all callers.
7. CLI profile construction does not maintain a second predefined-profile stage
   mapping.
8. Explicit CLI stage overrides, if supported, continue to work.

## Dispatch parity

9. `dispatch::recon::run_pipeline()` no longer hand-maps profiles to stages.
10. Dispatch Quick uses canonical Quick stages.
11. Dispatch Full uses canonical Full stages.
12. Dispatch Recon uses canonical Recon stages.
13. Dispatch DefenseLab uses canonical DefenseLab stages.
14. Requested profile and stored pipeline profile are identical.
15. Requested profile and pipeline risk budget are consistent.
16. Quick no longer produces an empty dispatch pipeline.
17. Full/Api/Deep are not accidentally constrained by Quick's SafeActive budget.
18. Defense-lab/private-scope validation observes the requested profile.
19. ProtocolEdge/NseSafe feature validation observes the requested profile.
20. The post-polish headless Fuzz/LoadTest/WAF/Recon execution paths remain
    parser-independent.

## Tool-API parity

21. `PipelineTool` passes its parsed profile into actual execution.
22. Tool-API Quick executes canonical Quick stages rather than an empty pipeline.
23. A non-Quick tool profile uses `Stage::from_profile()` through the canonical
    constructor.
24. Existing callback finding behavior is preserved.
25. Existing timeout/error response behavior is preserved.
26. Existing public `run_with_callback` compatibility is preserved if it is a
    supported public API.
27. No tool-API path requires Clap merely to select a profile.

## Output/session semantics

28. `output_file` is not silently ignored.
29. Any historical checkpoint/session behavior attached to `output_file` is
    preserved or deliberately removed with call-site/test evidence.
30. `output_format` is not silently ignored; it is either meaningfully consumed
    or removed coherently from the internal API.
31. No duplicate output serialization subsystem is introduced.

## Behavioral regression coverage

32. Quick constructor/state is covered by a deterministic test.
33. Full constructor/state is covered by a deterministic test.
34. Recon constructor/state is covered by a deterministic test.
35. DefenseLab constructor/state is covered by a deterministic test.
36. Tests assert stage lists through canonical `Stage::from_profile()` semantics.
37. Tests assert risk-budget/profile consistency.
38. A focused dispatch regression test proves the duplicate/manual mapping bug is
    gone.
39. A focused tool-API regression test proves requested profile forwarding.
40. Tests do not require public internet access.
41. No broad mock/test framework is introduced.

## Existing boundary preservation

42. `eggsec-python` still uses `eggsec` with `default-features = false`.
43. `eggsec-python` does not enable `cli`.
44. `clap` is not reachable from the default Python graph.
45. `clap_complete` is not reachable from the default Python graph.
46. distributed worker parser-independent behavior remains restored.
47. scanner/tool-API parser-independent callback paths remain restored.
48. `runtime_bridge` disposition is not reopened unless this change directly
    proves the existing classification incorrect.

## Verification

49. `cargo fmt --all -- --check` passes on the validated implementation SHA.
50. `make check` passes on that SHA.
51. `make check-python` passes on that SHA.
52. `cargo deny check advisories` passes with only documented retained exceptions.
53. Rust 1.88 no-default workspace check passes.
54. no-default `eggsec` check passes.
55. no-default `tool-api` check passes.
56. no-default `tool-api,rest-api` check passes.
57. `eggsec-python` check passes.
58. focused behavioral regression tests pass directly.
59. final validation records a clean working tree.
60. hosted CI is not called PASS unless directly inspected.

## Documentation/closure

61. `plans/README.md` marks this handoff active during implementation.
62. the post-polish plan contains its real closure-record SHA rather than
    `<this commit>`.
63. the closure report records the new exact validated implementation SHA.
64. the closure report explicitly records dispatch/tool-API profile parity.
65. old artifact measurements remain associated with their original measurement
    SHA unless actually remeasured.
66. after validation, `plans/README.md` records no active corrective handoff.
67. any closure-record-only SHA is recorded separately from the validated code
    SHA.

## CI/release/scope control

68. routine CI remains Linux Rust + Python only.
69. MSRV/macOS/Windows remain scheduled/manual.
70. no publication workflow is added.
71. release cadence remains manual.
72. no authorization/scope/DNS semantic change is introduced.
73. no new feature/profile is introduced.
74. no broad pipeline architecture rewrite is introduced.
75. no unrelated dependency modernization is introduced.
76. the pass ends after this behavioral closure rather than creating another
    planning layer for already-resolved work.

---

# Handoff completion record

Fill only after implementation and exact-SHA validation:

```text
final status: Pending
starting SHA: a106d920a117395e9af48e0e26de12b05a4f3694
validated implementation SHA: <pending>
closure-record-only SHA: <pending or none>
canonical profile constructor: <pending>
dispatch duplicate stage map: <pending>
Quick dispatch stages: <pending>
Full dispatch stages: <pending>
Recon dispatch stages: <pending>
DefenseLab dispatch stages: <pending>
dispatch risk-budget parity: <pending>
tool-api profile forwarding: <pending>
tool-api Quick non-empty: <pending>
output_file disposition: <pending>
output_format disposition: <pending>
eggsec-python cli feature: absent
clap reachable from Python: <pending final tree check>
clap_complete reachable from Python: <pending final tree check>
make check: <pending>
make check-python: <pending>
cargo deny advisories: <pending>
MSRV 1.88: <pending>
no-default engine: <pending>
no-default tool-api: <pending>
tool-api+rest-api: <pending>
eggsec-python: <pending>
focused behavioral tests: <pending>
hosted CI: NOT VERIFIED unless inspected
publication status: NOT RUN
```
