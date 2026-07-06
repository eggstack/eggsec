# NSE Milestone 4 Closure Plan

## Purpose

Close the remaining Milestone 4 gaps after the broad compatibility, evidence, context, and UX implementation pass.

The implementation landed the right architecture pieces: expanded corpus fixtures, a manifest, upstream-style local fixtures, host/port/service context types, structured evidence, human-readable report output, a compatibility matrix, and new architecture guards. The remaining issue is validation quality: the manifest-backed corpus harness currently verifies metadata shape and report construction, but it does not yet execute each fixture through the real NSE runtime and observe actual libraries, rules, capability events, evidence, and compatibility state.

This pass must convert Milestone 4 from “implemented and documented” to “runtime-verified.”

## Current State Summary

Confirmed progress:

- `NseRunReport` now includes `evidence` and `capability_events`.
- `NseEvidenceItem` and `NseEvidenceKind` exist and serialize.
- `extract_evidence()` conservatively derives evidence from capability denials, compatibility warnings, rule errors, and raw script output.
- `run_cli_with_profile()` builds reports, computes compatibility, extracts evidence, and prints human-readable report summaries for non-JSON output.
- `NseHostContext`, `NsePortContext`, `NseServiceContext`, and `NseContextSource` exist.
- `evaluate_rule_with_context()` annotates rule reports with context source and marks synthetic matched contexts approximate.
- The corpus fixture tree is expanded with discovery, version, default, auth, protocol, partial, unsupported, regression, and upstream-style fixtures.
- `manifest.toml` contains expected status/fidelity/libraries/rules/capability events, provenance, upstream references, local-only flags, and gap classifications.
- `docs/NSE_COMPATIBILITY.md` exists and avoids full Nmap parity claims.
- Architecture guards check local-only fixtures, compatibility matrix presence, bridge module presence, and prior profile-propagation constraints.

Remaining gaps:

1. The manifest-backed corpus harness creates expected rule reports, expected library reports, and expected capability events from manifest data instead of observing them from runtime execution.
2. The harness therefore proves manifest/schema consistency but not script compatibility.
3. Evidence tests validate extraction helpers, but fixture-level evidence is not yet verified through actual execution.
4. The compatibility matrix may overstate closure if entries are not backed by observed runtime fixture results.
5. Some guard checks remain informational where closure claims should eventually become enforceable.
6. Final Milestone 4 verification needs to be recorded with command results and known caveats.

## Non-Goals

Do not reopen Milestone 1 loader/profile semantics.

Do not reopen Milestone 2 library-report truthfulness.

Do not redesign Milestone 3 capability context semantics.

Do not expand the fixture corpus further unless a missing fixture is needed to validate closure behavior.

Do not require public internet or arbitrary upstream script downloads.

Do not claim full Nmap NSE parity.

## Workstream 1: Convert Manifest Harness to Runtime Execution

### Problem

The current manifest harness reads fixture content and resolves scripts, but then constructs rules, libraries, and capability events from manifest expectations. This is self-referential and cannot catch runtime regressions.

### Required Outcome

Each manifest fixture should execute through `NseExecutor::with_profile()` and `run_script_with_rules()` or a shared report-building helper that uses the same path as CLI report generation.

### Implementation Steps

1. Add a helper in tests, or a crate-private helper if useful:

```rust
fn run_fixture_runtime(entry: &FixtureEntry) -> NseRunReport
```

2. For each fixture:
   - copy script and fixture modules into a temp corpus root;
   - construct the profile from manifest metadata;
   - resolve the script with `ScriptResolver`;
   - construct `NseExecutor::with_profile(&profile)`;
   - set target and script args;
   - configure ports/service context where manifest metadata requires it;
   - run `executor.run_script_with_rules(&resolved.content)`;
   - collect runtime `executor.library_reports()`;
   - collect runtime `executor.capability_events()`;
   - build `NseRunReport` using the same builder sequence as CLI;
   - compute compatibility;
   - extract evidence with `extract_evidence()`.
3. Use static `require()` fallback only when runtime execution fails before required modules are observed and the report clearly marks the entries as statically detected.
4. Preserve a separate metadata/schema test for manifest completeness, but do not confuse it with compatibility execution.

### Acceptance Criteria

- `corpus_harness_all_fixtures_execute()` actually executes every manifest script unless a fixture is explicitly metadata-only.
- Runtime libraries are observed from `require()` activity, not synthesized from `expected_libraries`.
- Runtime rules are observed from actual `hostrule`/`portrule`/`prerule`/`postrule` execution.
- Runtime capability events are observed from wrapper/capability context behavior.
- Runtime evidence is extracted from the resulting report.

## Workstream 2: Add Manifest Controls for Runtime Context

### Problem

Some fixtures need host/port/service context, script args, module roots, local mock data, or expected denial behavior. The harness needs metadata for this without hardcoding each fixture.

### Required Manifest Fields

Add optional fields as needed:

```toml
[target]
host = "127.0.0.1"
hostname = "localhost"

[[port]]
number = 80
protocol = "tcp"
state = "open"
service = "http"
version = "mock/1.0"
source = "fixture"

[script_args]
key = "value"

[harness]
execute = true
expect_runtime_error = false
allow_static_require_fallback = false
```

Keep this minimal. Only add fields needed by fixtures.

### Acceptance Criteria

- Context-dependent fixtures get host/port/service data from manifest metadata.
- Fixtures do not rely on implicit defaults that hide context gaps.
- Synthetic context is marked as synthetic in rule reports.

## Workstream 3: Runtime Assertions Against Observed Fields

### Required Assertions

For each executed fixture, assert against observed report fields:

1. **Compatibility**
   - `report.compatibility.status == expected_status`
   - `report.compatibility.fidelity == expected_fidelity`
2. **Libraries**
   - observed library names include all `expected_libraries`;
   - unexpected all-registry-loaded behavior fails;
   - no-require fixtures keep `libraries` empty unless runtime actually required a library.
3. **Rules**
   - observed rule kinds include all `expected_rules`;
   - error/unsupported rule fixtures assert the correct error/unsupported field.
4. **Capability events**
   - observed events include expected kinds and allowed/denied state;
   - denied fixture categories must produce a real denial event.
5. **Evidence**
   - expected evidence kinds appear for capability denials, rule errors, approximations, and output fixtures;
   - capability-denial evidence is not mistaken for a target vulnerability.
6. **Resolver diagnostics**
   - expected resolved/blocked/rejected state is observed from `ScriptResolver`.

### Acceptance Criteria

- Tests fail if report fields are only manifest-synthesized.
- Tests fail if runtime does not produce expected library/rule/capability/evidence behavior.

## Workstream 4: Preserve Metadata Tests Separately

### Problem

The manifest itself is useful, but metadata tests should be clearly separate from runtime compatibility tests.

### Required Test Split

Split tests into two groups or naming conventions:

- `corpus_manifest_*`: manifest loads, file exists, provenance, local-only, gap classification validity.
- `corpus_runtime_*`: actual script execution and observed report assertions.

### Acceptance Criteria

- Test names make it clear what is metadata-only and what is runtime validation.
- Docs and compatibility matrix refer to runtime tests for compatibility claims.

## Workstream 5: Compatibility Matrix Truthfulness

### Problem

`docs/NSE_COMPATIBILITY.md` should not imply runtime-verified compatibility unless entries are backed by actual execution assertions.

### Required Updates

1. Add a column or note indicating verification mode:
   - `runtime-verified`
   - `metadata-only`
   - `planned`
   - `manual-reviewed`
2. Ensure every “Full” or “Complete” entry is runtime-verified.
3. Downgrade entries that are currently only metadata-backed.
4. Link matrix categories to manifest fixture IDs and runtime test names where feasible.

### Acceptance Criteria

- The matrix does not overclaim coverage.
- “Full”/“Complete” status requires runtime execution tests.

## Workstream 6: Tighten Architecture Guards

### Required Guard Improvements

1. Add a guard that fails if the manifest harness constructs production-looking `NseLibraryUseReport`, `NseRuleEvaluationReport`, or `NseCapabilityEvent` directly from expected manifest fields inside runtime tests.
2. Allow direct construction only in metadata/schema tests and unit tests.
3. Add a guard or test that runtime corpus tests call `NseExecutor::with_profile()`.
4. Keep evidence construction centralized through `extract_evidence()`; if direct construction outside tests appears, fail or warn depending on scope.
5. Keep local-only fixture guard as failure.

### Acceptance Criteria

- A self-referential manifest runtime harness would fail.
- Metadata-only tests remain allowed.
- Guard output explains how to migrate to observed runtime reports.

## Workstream 7: End-to-End CLI/Report Smoke Tests

### Required Tests

Add at least two end-to-end or near-end-to-end tests using the CLI report path or a shared internal report helper:

1. **AgentSafe denial JSON report**
   - run a fixture that triggers a filesystem/process denial;
   - assert JSON/report includes a capability denial event and evidence item.
2. **Context fidelity report**
   - run a fixture with service context;
   - assert rule report includes host/port context source and service availability.
3. **Human report smoke**
   - optional: format a report through `print_human_report` or equivalent formatter and assert key headings appear.

If direct stdout capture is brittle, factor report construction into a testable helper and test the helper.

### Acceptance Criteria

- At least one test covers the same report-building path used by `run_cli_with_profile()`.
- JSON/report output contains observed capability/evidence fields.

## Workstream 8: Final Verification Record

### Required Commands

Run and record in `architecture/nse_integration.md` or a dedicated closure note:

```bash
cargo check -p eggsec-nse --features nse
cargo test -p eggsec-nse --features nse compatibility_corpus
cargo test -p eggsec-nse --features nse context_fidelity
cargo test -p eggsec-nse --features nse evidence
cargo test -p eggsec-nse --features nse profile_report
cargo test -p eggsec-nse --features nse report
cargo test -p eggsec-nse --features nse
cargo test -p eggsec-nse --features nse,sandbox
cargo check -p eggsec --features nse
cargo test -p eggsec --features nse --test nse_tests
bash scripts/check-architecture-guards.sh
cargo fmt --all --check
cargo clippy --lib -p eggsec-nse --features nse
cargo clippy --lib -p eggsec --features nse
```

If any command is unavailable or has known pre-existing failures, record exact file/line and status.

### Acceptance Criteria

- Milestone 4 final verification is visible in repo docs.
- Verification distinguishes metadata tests from runtime fixture execution.

## Workstream 9: Closure Note and Milestone 5 Boundary

### Required Closure Note

Add a Milestone 4 closure note stating:

- how many fixtures are metadata-validated;
- how many fixtures are runtime-verified;
- which categories are runtime-verified;
- which categories remain metadata-only or partial;
- what compatibility matrix entries are backed by runtime tests;
- known gaps and deferred libraries;
- Milestone 5 recommended scope.

### Milestone 5 Candidate Scope

Milestone 5 should focus on:

- deeper protocol-library migration;
- real local HTTP/TLS/DNS service fixtures where practical;
- direct runtime validation for deferred protocol libraries;
- broader upstream-style subset coverage;
- TUI-first compatibility debugging workflow;
- performance/caching for large corpus runs.

Milestone 5 should not reopen:

- loader/profile enforcement;
- library-report truthfulness;
- capability-context construction;
- evidence semantics.

## Final Acceptance Criteria

This closure pass is complete when:

- Manifest corpus runtime tests execute scripts through `NseExecutor::with_profile()`.
- Runtime tests assert observed libraries, rules, capability events, evidence, resolver diagnostics, and compatibility.
- Metadata-only checks are clearly separated from runtime compatibility checks.
- Compatibility matrix claims are tied to runtime-verified fixture IDs or downgraded.
- Guards prevent self-referential manifest report construction in runtime tests.
- End-to-end report smoke tests exist for denial/evidence and context fidelity.
- Final verification is recorded.

## Handoff Notes

Keep this pass focused. The architecture pieces are present. The issue is evidence quality: compatibility claims must be backed by observed runtime behavior, not reports assembled from expected manifest fields. Do not add more fixtures until the current fixture set is genuinely executed and asserted.
