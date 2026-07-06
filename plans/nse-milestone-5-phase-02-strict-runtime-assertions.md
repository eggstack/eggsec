# NSE Milestone 5 Phase 02: Strict Runtime Assertions

## Purpose

Tighten runtime corpus assertions so required manifest expectations become hard checks against observed runtime behavior.

Milestone 4 made the corpus execute real fixtures. Some assertions remain intentionally lenient while runtime behavior stabilizes: missing libraries may only be logged, empty rule reports may be accepted for broad cases, and resolver errors can satisfy expected capability-denial cases. Phase 02 converts those lenient areas into precise required/optional semantics.

## Non-Goals

Do not tighten assertions before Phase 01 stabilizes runtime flakiness.

Do not fabricate runtime fields from manifest expectations.

Do not require public internet.

Do not downgrade security semantics to make tests pass.

## Workstream 1: Extend Manifest Expectation Semantics

### Problem

The manifest currently has broad fields like `expected_libraries`, `expected_rules`, and `expected_capability_events`. The runtime harness sometimes treats these as advisory because some fixtures are approximate or blocked before execution.

### Required Outcome

The manifest should distinguish required and optional expectations explicitly.

### Suggested Fields

```toml
expected_libraries = ["stdnse"]
optional_libraries = ["http"]
expected_rules = ["portrule"]
optional_rules = []

[[expected_capability_events]]
kind = "process_exec"
allowed = false
required = true
satisfiable_by_resolver_block = false

[harness]
execute = true
allow_static_require_fallback = false
allow_missing_runtime_libraries = false
allow_missing_runtime_rules = false
```

### Acceptance Criteria

- Every lenient expectation is explicitly encoded as optional or allowed by harness metadata.
- Required fields are hard assertions.

## Workstream 2: Enforce Required Library Observations

### Steps

1. For non-blocked fixtures, assert every `expected_libraries` entry appears in `report.libraries` unless `allow_static_require_fallback` or `allow_missing_runtime_libraries` is explicitly true.
2. Assert expected loaded libraries have `loaded = true` unless manifest marks them static-only or attempted-failed.
3. Assert no all-registry-loaded behavior by checking the observed library count is bounded by fixture expectations plus documented optional runtime dependencies.
4. Keep no-require fixtures strict: `libraries` must be empty unless runtime genuinely requires internal support modules and the manifest declares them.

### Acceptance Criteria

- A missing `stdnse` require observation fails for fixtures that require `stdnse`.
- A report containing all registry entries fails unless the fixture explicitly expects that impossible behavior, which it should not.

## Workstream 3: Enforce Required Rule Observations

### Steps

1. For non-blocked fixtures, assert every `expected_rules` entry appears in `report.rules` unless `allow_missing_runtime_rules` is true.
2. Add manifest fields for expected matched/evaluated/error state where useful:

```toml
[[expected_rule_report]]
kind = "portrule"
evaluated = true
matched = true
exactness = "approximate"
```

3. For false/error/unsupported fixtures, assert exact observed state instead of allowing empty rules broadly.
4. Assert context metadata for context fixtures: `host_context_source`, `port_context_source`, and `service_context_available`.

### Acceptance Criteria

- Rule expectations fail when the runtime does not evaluate the expected rule.
- Unsupported/error fixtures prove the expected error/unsupported path.

## Workstream 4: Enforce Required Capability Events

### Steps

1. For each expected capability event with `required = true`, assert an observed event with matching kind and allowed/denied state.
2. Only allow resolver block substitution if `satisfiable_by_resolver_block = true`.
3. For runtime-denial fixtures such as process/file/network denial, do not allow resolver block substitution.
4. Assert `CapabilityDenial` evidence exists for observed denied events.

### Acceptance Criteria

- `process-denied` fails if `process_exec` denial is not observed.
- `fs-read-denied` fails if it is meant to test runtime wrapper denial and only resolver-blocks.
- Resolver-block fixtures remain valid if explicitly marked.

## Workstream 5: Strict Evidence Assertions

### Steps

Add manifest fields:

```toml
expected_evidence_kinds = ["capability-denial", "script-output"]
optional_evidence_kinds = []
```

Assert observed `report.evidence` contains required kinds and does not overclaim vulnerability/finding semantics.

### Acceptance Criteria

- Runtime capability-denial fixtures produce capability-denial evidence.
- Output-only fixtures produce script-output evidence if output is expected.

## Workstream 6: Guard Against Self-Referential Runtime Tests

Strengthen architecture guards:

- runtime corpus tests must not construct `NseLibraryUseReport` from manifest expected libraries;
- runtime corpus tests must not construct `NseRuleEvaluationReport` from manifest expected rules except for explicitly marked execution-error fallback paths;
- runtime corpus tests must not construct `NseCapabilityEvent` from expected events;
- metadata/static corpus tests may construct reports, but file/module names should make that role clear.

### Acceptance Criteria

- A regression to manifest-synthesized runtime reports fails guards.

## Workstream 7: Update Compatibility Matrix

Add verification mode and strictness columns:

- `runtime-strict`
- `runtime-observed-optional`
- `resolver-only`
- `metadata-only`

Only `runtime-strict` entries should be described as fully runtime verified.

## Verification

Run:

```bash
cargo test -p eggsec-nse --features nse --test runtime_corpus_tests -- --test-threads=1
cargo test -p eggsec-nse --features nse --test runtime_corpus_tests -- --test-threads=4
cargo test -p eggsec-nse --features nse --test runtime_smoke_tests
cargo test -p eggsec-nse --features nse --test compatibility_corpus_tests
bash scripts/check-architecture-guards.sh
```

## Final Acceptance Criteria

Phase 02 is complete when:

- Required libraries/rules/capability events/evidence are hard assertions.
- Optional expectations are explicit in manifest metadata.
- Runtime tests no longer silently accept missing required fields.
- The compatibility matrix distinguishes strict runtime verification from weaker modes.
- Guards prevent self-referential runtime test construction.