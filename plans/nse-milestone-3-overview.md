# NSE Milestone 3 Overview: Capability Wrappers and Helper-Side Enforcement

## Purpose

Milestone 3 hardens NSE library execution by moving side-effecting Rust helper operations behind profile-aware capability wrappers with consistent accounting, cancellation checks, diagnostics, and report integration.

Milestone 1 closed script/module loading policy. Milestone 2 made compatibility/report truthfulness explicit. Milestone 3 should now address the known deferred gap: Lua instruction hooks cannot reliably interrupt Rust-side blocking helper work once control has entered a native helper. Side-effecting helpers must therefore check capability policy themselves.

## Boundary From Prior Milestones

Do not reopen these closed areas:

- `ScriptResolver` owns user script/module file loading.
- Manual versus automated profile semantics remain unchanged.
- `NseRunReport.libraries` means per-run observed/attempted `require()` activity.
- Registry APIs describe capability metadata, not per-run usage.
- Rule reports distinguish match, no-match, unsupported, and error states.

Milestone 3 builds on these contracts.

## Target End State

At the end of Milestone 3:

1. Side-effecting NSE Rust helpers route through a central capability context.
2. Network, filesystem, process, time, randomness, DNS, crypto, and compression helper classes have explicit policy/accounting hooks.
3. Blocking helper calls check cancellation before execution and after returning, and use bounded timeouts where feasible.
4. Automated profiles deny or bound operations that manual profiles may allow.
5. Helper-side operations update resource counters and structured reports.
6. Tests prove agent-safe/CI-safe denial, manual allowance, cancellation, limits, and reporting.
7. Docs clearly state what helper-side enforcement covers and what remains approximate.

## Phase Files

Milestone 3 is split into six detailed phase plans:

1. `plans/nse-milestone-3-phase-01-capability-inventory.md`
2. `plans/nse-milestone-3-phase-02-capability-context.md`
3. `plans/nse-milestone-3-phase-03-filesystem-process-wrappers.md`
4. `plans/nse-milestone-3-phase-04-network-dns-wrappers.md`
5. `plans/nse-milestone-3-phase-05-time-random-crypto-accounting.md`
6. `plans/nse-milestone-3-phase-06-reporting-docs-verification.md`

## Recommended Sequence

Implement in order.

Phase 01 inventories helper-side effects and assigns risk classes. Phase 02 introduces the shared capability context and wrapper API. Phases 03 through 05 migrate helper classes incrementally. Phase 06 closes reports, docs, tests, and verification.

## Non-Goals

Do not attempt full Nmap parity.

Do not rewrite all library behavior in one broad pass.

Do not remove manual CLI/TUI discretion.

Do not use capability wrappers to block legitimate manual workflows by default.

Do not bypass Milestone 1 `ScriptResolver` or Milestone 2 report semantics.

## Global Acceptance Criteria

Milestone 3 is complete when:

- Side-effecting helper classes have central wrappers or explicit documented exclusions.
- Agent-safe and CI-safe profiles cannot accidentally run unbounded filesystem/process/network helper operations.
- Resource counters reflect helper-side network/filesystem/process operations.
- Cancellation checks exist before and after potentially blocking helper calls.
- Structured reports expose helper denials, limit violations, and side-effect summaries.
- Architecture guards catch new direct side-effect helpers that bypass wrappers.
- Verification commands and closure notes are recorded in repo docs.

## Verification Gate

At each phase, run:

```bash
cargo check -p eggsec-nse --features nse
cargo test -p eggsec-nse --features nse
cargo test -p eggsec-nse --features nse,sandbox
bash scripts/check-architecture-guards.sh
```

At milestone completion, also run:

```bash
cargo check -p eggsec --features nse
cargo test -p eggsec --features nse --test nse_tests
cargo fmt --all --check
cargo clippy --lib -p eggsec-nse --features nse
cargo clippy --lib -p eggsec --features nse
make test-nse
```

If `make test-nse` is unavailable, record the closest equivalent.
