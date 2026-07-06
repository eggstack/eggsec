# NSE Milestone 4 Overview: Compatibility Fidelity, Evidence, and UX

## Purpose

Milestone 4 improves practical NSE compatibility confidence and user-facing output after Milestones 1 through 3 established the core safety and truthfulness model.

Milestone 1 closed loader/profile enforcement. Milestone 2 closed registry/report truthfulness. Milestone 3 introduced capability wrappers for helper-side enforcement. Milestone 4 should now improve fidelity: broader local corpus coverage, representative upstream NSE subset validation, host/port/service context accuracy, structured evidence reporting, and CLI/TUI report usability.

## Target End State

At the end of Milestone 4:

1. Eggsec has a broader local-only NSE compatibility corpus with tiered fixtures.
2. A curated upstream NSE subset can be tested deterministically without public internet.
3. Rule execution uses richer host/port/service context where available.
4. Reports include structured evidence suitable for findings, not just raw script output.
5. CLI/TUI surfaces expose compatibility status, warnings, capability events, and evidence clearly.
6. Docs publish a truthful compatibility matrix without claiming full Nmap parity.
7. Deferred protocol/helper gaps are visible and actionable.

## Phase Files

Milestone 4 is split into five detailed phase plans:

1. `plans/nse-milestone-4-phase-01-corpus-expansion.md`
2. `plans/nse-milestone-4-phase-02-upstream-subset-validation.md`
3. `plans/nse-milestone-4-phase-03-host-port-service-context.md`
4. `plans/nse-milestone-4-phase-04-structured-evidence-reports.md`
5. `plans/nse-milestone-4-phase-05-ux-docs-release-closure.md`

## Recommended Sequence

Implement in order. Corpus expansion should come first so later fidelity/report work has representative fixtures. Upstream subset validation should remain curated and local-only. Context fidelity then improves rule behavior and corpus results. Evidence reports and UX should build on stable report fields. The final phase closes docs, matrix, verification, and release notes.

## Non-Goals

Do not claim full Nmap parity.

Do not download or execute arbitrary upstream scripts during normal tests.

Do not reopen loader/profile semantics from Milestone 1.

Do not reopen library-report truthfulness from Milestone 2.

Do not redesign capability context semantics from Milestone 3.

Do not make tests require public internet.

## Global Acceptance Criteria

Milestone 4 is complete when:

- Local compatibility corpus coverage is broad enough for common discovery/version/default scripts.
- Curated upstream subset validation is deterministic and auditable.
- Rule reports include more truthful host/port/service context summaries.
- Structured reports can produce evidence objects for findings.
- CLI/TUI output presents compatibility status, warnings, denials, and evidence clearly.
- Compatibility docs state supported, partial, approximate, unsupported, and deferred categories precisely.
- Verification commands and known gaps are recorded.

## Verification Gate

At each phase:

```bash
cargo check -p eggsec-nse --features nse
cargo test -p eggsec-nse --features nse
cargo test -p eggsec-nse --features nse,sandbox
bash scripts/check-architecture-guards.sh
```

At Milestone 4 closure:

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
make test-nse
```

Record unavailable commands or known pre-existing failures precisely.
