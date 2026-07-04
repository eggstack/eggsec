# NSE Milestone 2 Phase 04: Compatibility Corpus and Fixtures

## Purpose

Create a representative NSE compatibility corpus that verifies supported, partial, approximate, unsupported, denied, and errored behavior. The corpus should make compatibility claims testable and prevent future work from overclaiming Nmap parity.

This phase uses the Phase 01 registry, Phase 02 rule reports, and Phase 03 structured reports.

## Background

A production-grade NSE compatibility layer needs a repeatable corpus. The corpus should not run risky external actions by default. It should use safe fixtures, local-only scripts, synthetic host/port context, and deterministic expectations.

The goal is not to import a huge upstream Nmap script suite blindly. The goal is a curated set that proves Eggsec's stated compatibility contract.

## Non-Goals

Do not vendor the full upstream Nmap NSE script tree.

Do not run scripts against public internet targets.

Do not add attack payloads or unsafe stress behavior to the default test corpus.

Do not require network access for normal CI corpus tests.

Do not hide unsupported behavior by excluding it from the corpus.

## Target State

By the end of this phase:

- There is a `crates/eggsec-nse/tests/fixtures/` or equivalent corpus directory.
- Corpus manifests describe each script fixture's expected compatibility status.
- Tests execute representative fixtures through the same resolver/profile/report path as real runs.
- Corpus covers success, partial compatibility, approximate rule handling, unsupported features, policy denial, and error cases.
- Corpus results validate structured report fields, not only raw execution success.

## Proposed Layout

Suggested structure:

```text
crates/eggsec-nse/tests/fixtures/nse_corpus/
  manifest.toml
  scripts/
    simple-portrule.nse
    simple-hostrule.nse
    stdnse-output.nse
    http-require-partial.nse
    invalid-module-name.nse
    unsupported-process.nse
    agent-denied-file.nse
  modules/
    custom_ok.lua
    partial_dep.lua
  expected/
    simple-portrule.json
    ...
```

If TOML parsing introduces unwanted dependencies, use static Rust fixture declarations instead.

## Manifest Model

Suggested manifest fields:

```toml
[[case]]
name = "simple-portrule"
script = "scripts/simple-portrule.nse"
profile = "manual-strict"
expected_status = "compatible-with-warnings"
expected_rule_status = "present-evaluated"
expected_rule_fidelity = "synthetic-input"
expected_libraries = ["stdnse"]
expected_warnings = ["synthetic"]
network = "none"
```

For the first implementation, a Rust static array may be simpler:

```rust
struct NseCorpusCase {
    name: &'static str,
    script_path: &'static str,
    profile_kind: NseExecutionProfileKind,
    expected_status: NseRunCompatibilityStatus,
    expected_rule_status: Option<NseRuleStatus>,
    expected_rule_fidelity: Option<NseRuleFidelity>,
    expected_libraries: &'static [&'static str],
    expected_warnings_contains: &'static [&'static str],
}
```

## Workstream 1: Corpus Harness

### Steps

1. Add a new integration test file, for example:

```text
crates/eggsec-nse/tests/compatibility_corpus_tests.rs
```

2. Add fixture directory and minimal scripts.
3. Add a helper that resolves a script fixture through `ScriptResolver`.
4. Execute using the same profile/report path used by CLI where possible.
5. Assert structured report fields.
6. Ensure all corpus tests are local-only and deterministic.

### Acceptance Criteria

- Corpus tests run under `cargo test -p eggsec-nse --features nse`.
- Tests do not require external network.
- Tests use real files and resolver path, not only inline strings.

## Workstream 2: Supported Behavior Cases

### Required Cases

- simple script with no side effects;
- `stdnse` output helper usage;
- `shortport`/portrule practical case;
- `hostrule` practical case;
- builtin module require case.

### Assertions

- Report status is compatible or compatible-with-warnings.
- Library metadata appears for used libraries.
- Rule metadata appears where applicable.
- No unexpected unsupported features.

### Acceptance Criteria

- Corpus proves the happy path.
- Reports reflect compatibility status, not only raw output.

## Workstream 3: Partial and Approximate Cases

### Required Cases

- rule evaluated with synthetic host/port context;
- library marked `Partial` or `Approximate` in registry;
- script that depends on simplified service metadata;
- script with category fallback behavior.

### Assertions

- Report marks approximation explicitly.
- Compatibility summary downgrades from exact/compatible when appropriate.
- Warnings are stable and actionable.

### Acceptance Criteria

- Partial behavior is represented, not hidden.
- Approximation affects report summary.

## Workstream 4: Unsupported and Denied Cases

### Required Cases

- invalid module name;
- filesystem module missing root;
- agent-safe arbitrary script-file denial;
- unsupported process execution path if safe to represent;
- missing required library metadata case if still possible.

### Assertions

- Denial happens before unsafe filesystem/network/process action.
- Report identifies `BlockedByPolicy`, `Unsupported`, or `Failed` accurately.
- Automated profiles never execute arbitrary fixture files except through explicitly allowed test harness paths.

### Acceptance Criteria

- Corpus proves safety denial paths.
- Unsupported behavior has explicit report status.

## Workstream 5: Snapshot or Stable Assertions

### Guidance

Prefer explicit field assertions over broad JSON snapshots unless the repo already uses snapshot testing. Full JSON snapshots become noisy. Stable assertions should cover:

- profile kind;
- script source summary;
- compatibility status;
- rule status/fidelity;
- library descriptors;
- warning substrings;
- resolver diagnostics kinds;
- success/failure status.

### Acceptance Criteria

- Tests are robust to harmless formatting changes.
- Tests fail on semantic regressions.

## Workstream 6: Corpus Documentation

### Steps

1. Add a corpus section to `architecture/nse_integration.md`.
2. Document how to add a fixture:
   - choose safe local-only script;
   - declare expected compatibility status;
   - assert report fields;
   - avoid external targets.
3. Add `.opencode/skills/eggsec-nse/SKILL.md` note directing agents to the corpus for compatibility claims.

### Acceptance Criteria

- Future contributors can add fixtures without guessing conventions.
- Docs state the corpus is representative, not exhaustive upstream Nmap parity.

## Verification

Run:

```bash
cargo test -p eggsec-nse --features nse compatibility_corpus
cargo test -p eggsec-nse --features nse
cargo test -p eggsec-nse --features nse,sandbox
cargo check -p eggsec --features nse
bash scripts/check-architecture-guards.sh
```

## Final Acceptance Criteria

Phase 04 is complete when:

- A local-only compatibility corpus exists.
- Corpus cases cover supported, partial, approximate, unsupported, denied, and errored behavior.
- Tests assert structured report fields.
- Corpus docs explain how to add cases safely.
- Compatibility claims in docs can point to corpus coverage.
