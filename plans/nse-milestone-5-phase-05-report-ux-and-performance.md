# NSE Milestone 5 Phase 05: Report UX and Runtime Performance

## Purpose

Polish user-facing report output and keep the runtime corpus usable as compatibility coverage grows.

Milestone 4 introduced `NseRunReport`, `ReportEnvelope` bridging, evidence extraction, and human-readable CLI report output. Phase 05 improves the display contract and ensures runtime corpus execution remains reasonably fast and diagnosable.

## Non-Goals

Do not redesign `NseRunReport` fields unless a bug requires it.

Do not add a new TUI architecture.

Do not parse arbitrary script prose into high-confidence findings.

Do not optimize by weakening safety checks.

## Workstream 1: CLI Report Formatting

### Goals

Improve the existing human report output so manual users can quickly see:

- compatibility status and fidelity;
- profile and trust boundary;
- rule summary;
- libraries used/attempted;
- capability denials/warnings;
- evidence items;
- raw output;
- verification caveats.

### Steps

1. Move report formatting into a testable formatter if not already separated from CLI printing.
2. Add snapshot-lite tests that assert headings and key lines without brittle full-output snapshots.
3. Ensure partial/approximate/denied states are visually distinct.
4. Ensure raw output remains available and not conflated with evidence.

### Acceptance Criteria

- Human report output is stable and tested.
- JSON output remains full fidelity.

## Workstream 2: TUI/Frontend Data Contract

### Goal

Define the display model for TUI or later frontends without forcing immediate implementation.

Required sections:

- Summary: status, fidelity, profile, target, script.
- Rule panel: rule kind, matched, exactness, context source.
- Libraries panel: loaded/attempted/failed, side effects.
- Capability panel: allowed/denied, reason, target.
- Evidence panel: kind, confidence, summary.
- Raw output panel.
- Diagnostics panel: resolver, errors, warnings.

### Acceptance Criteria

- Data contract references structured fields, not raw prose parsing.
- TUI implementation can consume `NseRunReport` or `ReportEnvelope` directly.

## Workstream 3: Runtime Corpus Performance Baseline

### Steps

1. Measure runtime corpus duration at:
   - `--test-threads=1`
   - `--test-threads=4`
   - default parallelism
2. Record fixture count, runtime duration, and known flakes.
3. Identify top slow fixtures.
4. Add lightweight timing logs or `--nocapture` diagnostics if useful.

### Acceptance Criteria

- A baseline exists before adding more fixtures.
- Slow fixtures are identifiable.

## Workstream 4: Performance Improvements

Possible targets:

- avoid repeated manifest parse by using a test-local lazy loader;
- reduce repeated fixture copying where safe;
- reuse local service setup only if it does not reintroduce shared-state flakes;
- avoid excessive JSON serialization in tight loops;
- keep per-fixture executor isolation.

### Acceptance Criteria

- Performance improvements do not weaken isolation.
- Runtime corpus remains deterministic.

## Workstream 5: ReportEnvelope Bridge Hardening

### Steps

1. Ensure `bridge.rs` maps report metadata and evidence consistently.
2. Add tests for:
   - compatible run envelope;
   - partial run envelope;
   - capability-denial evidence;
   - rule-error evidence;
   - raw-output evidence.
3. Ensure weak evidence does not become high-severity findings by default.

### Acceptance Criteria

- Envelope findings are conservative.
- Capability denials are execution limitations, not target vulnerabilities.

## Workstream 6: Docs

Update:

- `docs/NSE_COMPATIBILITY.md` report UX section;
- `architecture/nse_integration.md` report/UX/perf notes;
- `.opencode/skills/eggsec-nse/SKILL.md` guidance for future report changes.

## Verification

Run:

```bash
cargo test -p eggsec-nse --features nse --test runtime_smoke_tests
cargo test -p eggsec-nse --features nse --test runtime_corpus_tests -- --test-threads=1
cargo test -p eggsec-nse --features nse --test evidence_tests
cargo test -p eggsec-nse --features nse --test profile_report_tests
cargo test -p eggsec-nse --features nse
bash scripts/check-architecture-guards.sh
```

## Final Acceptance Criteria

Phase 05 is complete when:

- CLI report formatting is tested and readable.
- TUI/frontend data contract is documented.
- Runtime corpus performance baseline is recorded.
- Performance improvements preserve isolation.
- ReportEnvelope bridge tests cover compatibility, evidence, and denial cases.