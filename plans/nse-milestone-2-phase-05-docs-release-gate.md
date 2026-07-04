# NSE Milestone 2 Phase 05: Docs, Release Gate, and Compatibility Truthfulness

## Purpose

Close Milestone 2 by aligning documentation, architecture guards, verification commands, and release criteria around the new registry, rule-semantics, structured-report, and compatibility-corpus work.

This phase turns implementation into a durable production-readiness contract. It should prevent future contributors and agents from overstating compatibility, bypassing registry/report metadata, or confusing selective compatibility with full Nmap parity.

## Background

Milestone 1 closed loader/profile policy. Milestone 2 adds truthfulness around what is supported, partial, approximate, unsupported, or denied. The final phase ensures the repo communicates that truth consistently and enforces it through docs and checks.

## Non-Goals

Do not add new library behavior in this phase.

Do not redesign the registry or report data model unless a blocker is found.

Do not attempt full upstream Nmap parity.

Do not remove partial compatibility claims if they are backed by registry/report/corpus evidence.

## Target State

By the end of this phase:

- Docs identify the library registry as the source of truth for library compatibility.
- Docs identify rule reports as the source of truth for rule fidelity.
- JSON/report examples show compatibility status and warnings.
- Architecture guards catch missing registry entries and obvious report bypasses.
- CI/release gate commands are documented.
- Milestone 2 closure note exists with next-work boundary.

## Workstream 1: Documentation Consistency Pass

### Files to Inspect

- `architecture/nse_integration.md`
- `architecture/overview.md`
- `.opencode/skills/eggsec-nse/SKILL.md`
- `crates/eggsec-nse/AGENTS.override.md`
- root `AGENTS.md`
- root `README.md`
- any NSE-specific docs under `docs/`

### Required Wording

Use consistent claims:

- Eggsec has selective practical NSE compatibility, not full Nmap parity.
- Library compatibility is defined by `NseLibraryRegistry` metadata.
- Rule behavior is defined by `NseRuleEvaluationReport` / rule semantics metadata.
- Run output truthfulness is defined by `NseRunReport`.
- The compatibility corpus is representative and local-only by default.
- Manual/automated loader semantics remain closed from Milestone 1.
- Rust-side blocking helper cancellation remains Milestone 3 work unless wrappers have been implemented later.

### Acceptance Criteria

- No docs claim full Nmap compatibility.
- No docs claim exact rule semantics unless report fidelity can emit `Exact` for that case.
- No docs describe library counts/status in a way that contradicts the registry.

## Workstream 2: Compatibility Matrix

### Steps

1. Add a generated or manually-maintained compatibility matrix section, preferably in `architecture/nse_integration.md` or a dedicated doc such as:

```text
docs/NSE_COMPATIBILITY.md
```

2. Include at minimum:
   - library name;
   - status;
   - compatibility level;
   - side effects;
   - sandbox posture;
   - known gaps;
   - corpus coverage indicator.
3. If generating the matrix is too much for this phase, add a script/task as follow-up and include a small representative table.
4. Link the matrix from README/skill docs.

### Acceptance Criteria

- Users can see what is implemented, partial, stubbed, unknown, or unsupported.
- Matrix wording matches registry metadata.
- Unknowns are explicit.

## Workstream 3: JSON/Report Examples

### Steps

1. Add one or two compact JSON examples showing:
   - compatible run with warnings;
   - denied agent-safe arbitrary script file;
   - partial/approximate rule case.
2. Examples should include:
   - profile;
   - compatibility summary;
   - resolver diagnostics;
   - library metadata;
   - rule metadata;
   - warnings.
3. Keep examples short enough for docs readability.
4. Add comments nearby that field names are illustrative if the schema is not yet fully stable.

### Acceptance Criteria

- Users and agents know how to interpret structured reports.
- Examples do not overclaim exactness.

## Workstream 4: Architecture Guards

### Required Guards

Add/update `scripts/check-architecture-guards.sh` checks for:

1. library files without registry descriptors;
2. registry descriptors for removed/missing libraries;
3. direct library registration without registry metadata;
4. `run_cli_with_profile()` JSON path bypassing `NseRunReport` once reports exist;
5. new rule-evaluation convenience APIs that do not produce/report `NseRuleEvaluationReport` metadata;
6. docs claiming full Nmap parity.

### Guidance

Keep guards simple and maintainable. Static grep checks are acceptable if documented. Avoid overly brittle line-number allowlists unless necessary.

### Acceptance Criteria

- Guards fail with actionable messages.
- Existing Milestone 1 guards still pass.
- New guards protect Milestone 2 source-of-truth boundaries.

## Workstream 5: Verification Gate

### Required Gate

Record and run:

```bash
cargo check -p eggsec-nse --features nse
cargo test -p eggsec-nse --features nse
cargo test -p eggsec-nse --features nse,sandbox
cargo check -p eggsec --features nse
cargo test -p eggsec --features nse --test nse_tests
bash scripts/check-architecture-guards.sh
cargo fmt --all --check
cargo clippy --lib -p eggsec-nse --features nse
cargo clippy --lib -p eggsec --features nse
```

If `nextest` or project make targets are installed, also run:

```bash
make test-nse
```

### Documentation

Add a `Milestone 2 Verification Record` section to `architecture/nse_integration.md` with:

- commands run;
- pass/fail status;
- warning counts if useful;
- unavailable commands and closest equivalent;
- date/commit if known.

### Acceptance Criteria

- Release gate is visible outside commit messages.
- Failures are not hidden.

## Workstream 6: Milestone 2 Closure Note

### Steps

1. Add a `Milestone 2 Closure Note` section near the Milestone 1 closure note.
2. State what is closed:
   - library registry source of truth;
   - rule semantics report path;
   - structured reports;
   - compatibility corpus;
   - docs/release gate.
3. State what remains deferred:
   - Milestone 3 capability wrappers and Rust-side blocking cancellation;
   - any full Nmap parity gaps;
   - expanding corpus breadth;
   - additional library behavior upgrades.
4. State the boundary for future work.

### Acceptance Criteria

- Future contributors can tell what Milestone 2 completed.
- Milestone 3 starts cleanly at capability wrappers/hardening rather than redoing Milestone 2 truthfulness work.

## Final Acceptance Criteria

Phase 05 is complete when:

- Docs consistently state selective compatibility.
- Compatibility matrix exists or a clear first version exists.
- JSON/report examples show status, warnings, resolver diagnostics, library metadata, and rule metadata.
- Architecture guards protect registry/report source-of-truth boundaries.
- Verification record is documented.
- Milestone 2 closure note exists.

## Handoff Notes

Keep this phase evidence-oriented. The point is to make compatibility claims auditable. Avoid polishing prose in ways that soften or obscure partial/unsupported status. Unknowns and gaps are acceptable if they are visible and testable.
