# NSE Expansion Phase 00: Corrective Documentation and Verification Closeout

> **Status: Executed** — Completed 2026-07-07. All 4 workstreams done, 511 tests pass, 52 architecture guards pass, committed and pushed.

## Purpose

Clean up the remaining documentation drift after the HTTP method-enforcement hardening work and establish a clean baseline before new expansion.

The implementation now supports HTTP sync method enforcement with zero-hit denial tests, stricter runtime corpus expectations, and stronger guards. Some architecture notes still contain older wording that says the reqwest capability bypass remains unresolved. This phase reconciles those notes with the current implementation.

## Non-Goals

Do not add new NSE runtime behavior.

Do not change profile or capability semantics.

Do not add new fixtures except if needed to stabilize verification wording.

Do not broaden the roadmap while closeout docs are stale.

## Workstream 1: Remove Stale HTTP Bypass Language

### Files

- `architecture/nse_integration.md`
- `docs/NSE_COMPATIBILITY.md`
- `.opencode/skills/eggsec-nse/SKILL.md`
- `AGENTS.md`
- `crates/eggsec-nse/AGENTS.override.md`

### Required Updates

1. Remove or revise statements that say the HTTP reqwest capability bypass is still unresolved for covered sync methods.
2. Replace stale wording with precise current-state language:
   - sync HTTP network methods are preflight-gated by `NseCapabilityContext`;
   - denied automated-profile HTTP calls have zero-hit local fixture tests;
   - denied calls return a Lua denial response and do not reach reqwest;
   - protocol fidelity remains partial.
3. Keep any actual remaining gaps:
   - full Nmap HTTP parity is not claimed;
   - HTTP/2, advanced redirect/cookie behavior, and TLS edge behavior may remain partial;
   - async helper status should be described exactly.

### Acceptance Criteria

- No doc says covered sync HTTP methods still bypass capability checks.
- Docs still clearly avoid full Nmap NSE parity claims.

## Workstream 2: Reconcile Milestone Numbering

### Problem

Some files describe the state as Milestone 5 while compatibility docs now refer to Milestone 6. Mixed milestone references make handoff ambiguous.

### Steps

1. Decide final label for the current closed state: either “Milestone 6” or “NSE expansion baseline.”
2. Update headings and narrative sections consistently.
3. Keep historical references where they are clearly historical, but avoid contradictory future-work language.
4. Add a short note explaining that Milestone 5 closed runtime corpus/report hardening and Milestone 6 begins expansion.

### Acceptance Criteria

- Milestone references are internally consistent.
- Future agents can determine the current phase without reading commit history.

## Workstream 3: Record Final Verification

### Required Commands

Record results in `architecture/nse_integration.md`:

```bash
cargo check -p eggsec-nse --features nse
cargo test -p eggsec-nse --features nse --test local_protocol_tests
cargo test -p eggsec-nse --features nse --test runtime_corpus_tests -- --test-threads=1
cargo test -p eggsec-nse --features nse --test runtime_corpus_tests -- --test-threads=4
cargo test -p eggsec-nse --features nse --test runtime_smoke_tests
cargo test -p eggsec-nse --features nse --test format_tests
cargo test -p eggsec-nse --features nse
bash scripts/check-architecture-guards.sh
cargo fmt --all --check
cargo clippy --lib -p eggsec-nse --features nse
```

### Acceptance Criteria

- Final verification results are recorded.
- Any unavailable commands or pre-existing failures are stated explicitly.
- Architecture guard count and status are included.

## Workstream 4: Freeze Corrective Baseline

Add a short “baseline frozen” note:

- profile enforcement is not to be reopened without a regression;
- runtime library reporting is observed, not registry-synthesized;
- local protocol fixtures remain local-only;
- HTTP sync method enforcement is considered closed for covered methods;
- expansion work must preserve manual-vs-automated profile boundaries.

## Final Acceptance Criteria

Phase 00 is complete when:

- stale HTTP bypass references are corrected;
- milestone wording is consistent;
- verification is recorded;
- baseline invariants are documented;
- no code behavior changed except incidental doc/guard cleanup if required.
