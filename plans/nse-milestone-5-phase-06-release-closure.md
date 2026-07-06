# NSE Milestone 5 Phase 06: Release Closure

## Purpose

Close Milestone 5 with durable verification, documentation, guard alignment, and a clear boundary for Milestone 6.

Milestone 5 should resolve the Milestone 4 caveats: high-parallelism runtime flake, lenient observed-field assertions, and local protocol/deferred-library coverage gaps. This phase ensures those outcomes are recorded and release-ready.

## Non-Goals

Do not add new features during closure unless needed to fix verification blockers.

Do not claim full Nmap NSE parity.

Do not hide known flakes or known unsupported protocol libraries.

Do not delete plan files after execution.

## Workstream 1: Final Verification Matrix

Record in `architecture/nse_integration.md`:

```markdown
## Milestone 5 Final Verification

| Command | Status | Tests | Notes |
|---------|--------|-------|-------|
```

Required commands:

```bash
cargo check -p eggsec-nse --features nse
cargo test -p eggsec-nse --features nse --test runtime_corpus_tests -- --test-threads=1
cargo test -p eggsec-nse --features nse --test runtime_corpus_tests -- --test-threads=4
cargo test -p eggsec-nse --features nse --test runtime_corpus_tests
cargo test -p eggsec-nse --features nse --test runtime_smoke_tests
cargo test -p eggsec-nse --features nse --test compatibility_corpus_tests
cargo test -p eggsec-nse --features nse --test evidence_tests
cargo test -p eggsec-nse --features nse --test context_fidelity_tests
cargo test -p eggsec-nse --features nse
cargo test -p eggsec-nse --features nse,sandbox
cargo check -p eggsec --features nse
cargo test -p eggsec --features nse --test nse_tests
bash scripts/check-architecture-guards.sh
cargo fmt --all --check
cargo clippy --lib -p eggsec-nse --features nse
cargo clippy --lib -p eggsec --features nse
```

### Acceptance Criteria

- Verification state is visible in repo docs.
- Any remaining failures are categorized as blocker, known pre-existing, or deferred.

## Workstream 2: Close Runtime Flake Status

Document one of these:

- default parallel runtime corpus is stable;
- runtime corpus is intentionally serialized with rationale;
- known flake remains and blocks release closure.

If serialized, add an explicit command alias or docs guidance.

### Acceptance Criteria

- Future agents do not need to rediscover the thread-count behavior.

## Workstream 3: Compatibility Matrix Finalization

Update `docs/NSE_COMPATIBILITY.md` with:

- runtime verification mode per fixture/category;
- strict versus optional expectations;
- local protocol fixture coverage;
- deferred library status;
- Milestone 5 changes;
- Milestone 6 candidates.

### Acceptance Criteria

- Full/Complete labels only appear where runtime-strict tests back them.
- Deferred/partial entries remain explicit.

## Workstream 4: Guard Alignment

Ensure guards reflect the final state:

- runtime corpus file exists;
- runtime corpus uses `NseExecutor::with_profile()`;
- static corpus remains resolver-only;
- public-network fixtures are forbidden;
- self-referential runtime report construction is forbidden;
- local protocol fixtures use loopback only;
- evidence construction remains centralized;
- docs do not overclaim full parity.

### Acceptance Criteria

- `bash scripts/check-architecture-guards.sh` passes.
- Guard messages are actionable.

## Workstream 5: Documentation and Agent Guidance

Update:

- `architecture/nse_integration.md`;
- `docs/NSE_COMPATIBILITY.md`;
- `.opencode/skills/eggsec-nse/SKILL.md`;
- `AGENTS.md`;
- `crates/eggsec-nse/AGENTS.override.md`.

Required guidance:

- all new NSE compatibility claims must be backed by runtime tests;
- new side-effecting helpers must use capability wrappers;
- runtime corpus tests must remain local-only;
- evidence items are observations, not automatically vulnerabilities;
- plan files must be retained or archived.

## Workstream 6: Milestone 6 Boundary

Possible Milestone 6 directions:

- deeper upstream subset coverage;
- service-probe integration with Eggsec scan results;
- richer TUI workflow for compatibility debugging;
- incremental migration of remaining database/protocol libraries;
- performance/caching for large corpus runs;
- release packaging and user-facing examples.

Milestone 6 should not reopen:

- loader/profile enforcement;
- report truthfulness;
- core capability-context design;
- evidence semantics;
- runtime harness split.

## Final Acceptance Criteria

Phase 06 is complete when:

- Verification matrix is recorded.
- Runtime flake status is resolved or explicitly blocks closure.
- Compatibility matrix matches runtime verification mode.
- Architecture guards pass.
- Docs and agent guidance are aligned.
- Milestone 6 boundary is documented.

## Handoff Notes

Closure is a verification pass, not an expansion pass. Do not add more compatibility claims while closing Milestone 5 unless those claims are backed by strict runtime tests and local-only fixtures.