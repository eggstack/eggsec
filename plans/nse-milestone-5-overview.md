# NSE Milestone 5 Overview: Runtime Stability, Strictness, and Protocol Fidelity

## Purpose

Milestone 5 builds on the now-closed Milestone 4 runtime corpus by tightening validation strictness, eliminating high-parallelism flakes, and moving from representative compatibility to stronger protocol/runtime fidelity.

Milestone 4 established the split between static corpus metadata tests and runtime execution tests. It also documented two important caveats:

1. `runtime_corpus_tests.rs` can be flaky at high default test parallelism.
2. Some runtime assertions are intentionally lenient around observed libraries, rules, and capability events.

Milestone 5 should close those caveats before expanding compatibility claims further.

## Target End State

At the end of Milestone 5:

1. Runtime corpus tests are stable under default test parallelism or explicitly serialized with a documented reason.
2. Observed runtime fields are strict enough to catch missing libraries, missing rules, and missing capability events for fixtures that declare them as required.
3. The corpus manifest can distinguish required versus optional expectations.
4. Local protocol fixtures exercise real local HTTP, TCP, UDP, DNS-like, TLS/certificate, and filesystem/process denial paths where practical.
5. Deferred protocol-library migration has a prioritized implementation track.
6. CLI/TUI/report UX has a clearer bridge from `NseRunReport` / `ReportEnvelope` to user-facing summaries.
7. Milestone 5 verification is recorded with known gaps and next boundary.

## Phase Files

Milestone 5 is split into six phase plans:

1. `plans/nse-milestone-5-phase-01-runtime-flake-isolation.md`
2. `plans/nse-milestone-5-phase-02-strict-runtime-assertions.md`
3. `plans/nse-milestone-5-phase-03-local-protocol-fixtures.md`
4. `plans/nse-milestone-5-phase-04-deferred-library-migration.md`
5. `plans/nse-milestone-5-phase-05-report-ux-and-performance.md`
6. `plans/nse-milestone-5-phase-06-release-closure.md`

## Recommended Sequence

Implement phases in order. First stabilize runtime tests, then tighten assertions. Only after the harness is reliable should the corpus expand deeper protocol fixtures or migrate deferred libraries. Finish with report UX, performance, docs, guards, and release closure.

## Non-Goals

Do not reopen Milestone 1 loader/profile enforcement.

Do not reopen Milestone 2 library-report truthfulness semantics.

Do not reopen Milestone 3 capability context construction.

Do not reopen Milestone 4 corpus split or evidence semantics except to tighten validation.

Do not add public-internet-dependent tests.

Do not claim full Nmap NSE parity.

## Global Acceptance Criteria

Milestone 5 is complete when:

- Runtime corpus behavior is deterministic under the chosen test execution strategy.
- Required manifest expectations are enforced as hard assertions.
- Optional/approximate expectations are explicitly marked and reported.
- Local protocol fixtures cover common HTTP/TCP/UDP/DNS/TLS-style paths without public internet.
- At least one deferred or partially wrapped library class is migrated or explicitly reclassified with rationale.
- Report/UX output remains driven by `NseRunReport`/`ReportEnvelope`, not raw prose parsing.
- Guards protect runtime harness strictness and local-only protocol fixture behavior.
- Verification is recorded in architecture docs.

## Verification Gate

At each phase:

```bash
cargo check -p eggsec-nse --features nse
cargo test -p eggsec-nse --features nse --test runtime_corpus_tests
cargo test -p eggsec-nse --features nse --test runtime_smoke_tests
bash scripts/check-architecture-guards.sh
```

At Milestone 5 closure:

```bash
cargo check -p eggsec-nse --features nse
cargo test -p eggsec-nse --features nse --test runtime_corpus_tests -- --test-threads=1
cargo test -p eggsec-nse --features nse --test runtime_corpus_tests -- --test-threads=4
cargo test -p eggsec-nse --features nse --test runtime_corpus_tests
cargo test -p eggsec-nse --features nse --test runtime_smoke_tests
cargo test -p eggsec-nse --features nse
cargo test -p eggsec-nse --features nse,sandbox
cargo check -p eggsec --features nse
cargo test -p eggsec --features nse --test nse_tests
bash scripts/check-architecture-guards.sh
cargo fmt --all --check
cargo clippy --lib -p eggsec-nse --features nse
cargo clippy --lib -p eggsec --features nse
```

Record exact status and known flakes. If default parallelism remains intentionally unsupported, document and enforce serialized runtime corpus execution.