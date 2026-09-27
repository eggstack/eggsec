# NSE Runtime Extraction Milestone 005E — Provider Coverage Qualification and Parent-Milestone Closure

Status: blocked

Eggsec planning baseline: `b5d27348a829a4c1c2a49fbc267349c52e86a1f6`

Standalone runtime baseline: `eggstack/eggsec-nse@854f153f56d1abc929d9abd255f0606759342f81`

Source roadmap:

- `plans/subsystems/nse-runtime-extraction-roadmap.md#milestone-005--host-provider-inversion-and-portability-hardening`

Hard dependencies:

- accepted closure of `plans/implementation/nse-runtime-extraction/005-provider-broker-foundation.md`;
- accepted closure of `plans/implementation/nse-runtime-extraction/005-authority-preserving-network-dns.md`;
- accepted closure of `plans/implementation/nse-runtime-extraction/005-http-provider-eggsec-adapter.md`;
- accepted closure of `plans/implementation/nse-runtime-extraction/005-filesystem-process-portability.md`.

Long-term requirements:

- `plans/000-long-term-specification.md#2-primary-product-goals`
- `plans/000-long-term-specification.md#5-crate-ownership`
- `plans/001-terminology-and-domain-model.md#3-execution-terms`
- `plans/002-long-term-roadmap.md#phase-7--standing-maintenance-and-future-capability-open`

Applicable ADRs:

- `plans/adrs/ADR-0003-nse-host-provider-boundary.md`
- `plans/adrs/ADR-0001-scoped-transport-eggfetch-backend.md`

Primary class: infrastructure

Affected repositories:

- `eggstack/eggsec-nse`;
- `eggstack/eggsec`.

## 1. Objective

Qualify M005 as one coherent provider-inversion/portability milestone, reconcile provider coverage claims with actual source, close any integration-only defects that do not require redesign, and produce the evidence needed to decide whether the new runtime provider surface is ready for a versioned standalone release.

This plan is intentionally a qualification/integration pass, not another broad migration phase.

## 2. Why this milestone is blocked

Parent-milestone closure cannot be evaluated until 005A-D have accepted closures.

The earlier slices are deliberately separable:

- 005A establishes provider injection/broker semantics;
- 005B covers authority-preserving DNS/network;
- 005C covers HTTP and the concrete Eggsec scoped-transport consumer;
- 005D covers filesystem/process/platform portability.

005E verifies that those pieces compose under one run and that no documentation or static guard overstates coverage.

## 3. Current implementation evidence

At planning time M005 has not been implemented. Existing evidence motivating final qualification includes:

- runtime capability checks and direct host operations are currently split in multiple libraries;
- integration-era capability inventory claims can lag the standalone source;
- public `public_api` remains a second native I/O surface;
- protocol-specific libraries may intentionally retain specialized direct host calls;
- Eggsec must remain the sole product authorization authority even when runtime providers are injected.

Therefore closure requires an explicit source-based provider coverage inventory rather than a blanket "all side effects providerized" statement.

## 4. Invariants that must not regress

- No monolithic host trait.
- Native-default existing caller behavior remains available.
- Eggsec authorization remains outside runtime providers.
- Standalone runtime remains free of Eggsec dependencies.
- Provider-backed operations pass through the capability broker.
- Provider public contracts do not expose implementation-specific host types.
- Network authority preservation remains concrete-endpoint based.
- Eggsec HTTP adapter uses existing approved authority.
- Per-run provider/CWD state remains isolated.
- Windows qualification added by 005D stays green.
- `NseRunReport`, profile semantics, feature names, and `eggsec::nse` facade remain compatible.
- Residual direct host operations are explicitly classified, not hidden.

## 5. Scope

### In scope

- Build an authoritative source inventory of direct host side effects after 005A-D.
- Classify each site as:
  - provider-backed;
  - native-provider implementation;
  - compatibility shim;
  - intentionally specialized/deferred;
  - defect requiring correction before M005 closure.
- Add/adjust static guards so provider-coverage claims are mechanically enforced where feasible.
- Run deterministic injected-provider tests spanning multiple provider domains in one execution.
- Run authority/cancellation/accounting tests spanning DNS -> network -> HTTP where applicable.
- Run concurrent per-run isolation tests.
- Run Windows/Linux/macOS qualification.
- Run Eggsec scoped-provider integration tests and full relevant consumer checks.
- Reconcile standalone/Eggsec docs, compatibility tables, and old capability inventories.
- Decide release/versioning follow-up for the standalone provider public API.
- Create the M005 closure record if all acceptance criteria pass.

### Explicitly out of scope

- New provider domains not required by 005A-D.
- Rewriting every specialized protocol library merely to eliminate an inventory entry.
- Breaking `public_api` cleanup.
- New transport/proxy architecture.
- Release publication itself.
- New user-facing NSE features unrelated to providerization.
- Hiding unresolved high-severity bypasses behind documentation.

## 6. Required production changes

### Core/domain

Only integration/corrective changes required to make already-approved 005A-D contracts compose.

Do not redesign provider traits in this pass unless a closure blocker proves the accepted ADR/contracts are internally inconsistent. Such a finding requires a corrective plan or superseding ADR.

### Storage and migrations

None.

### Protocol and DTOs

No new DTO surface beyond prior slices.

### Runtime and concurrency

Verify one run can use a coherent custom service bundle across several domains and another run can simultaneously use different provider state.

Verify no provider slice fell back to process-global state that defeats per-run isolation.

### Frontend or operator surface

No intended behavior change.

### Security and authorization

Qualify end-to-end:

```text
Eggsec authorization
-> approved execution / authority
-> NseRunRequest with selected services
-> runtime capability broker
-> provider
-> concrete host operation
-> counters/events/report
```

The test matrix must include denials at both the Eggsec authority boundary and runtime capability boundary where applicable.

### Documentation and static guards

Replace stale capability/provider claims with source-derived current state. Historical documents may remain as evidence but must be labeled historical if no longer authoritative.

## 7. Ordered work packages

### Work package A — Source-based provider coverage audit

Intent:

Establish truth before closure.

Required changes:

- enumerate direct filesystem/network/DNS/HTTP/process/time/random/environment host calls;
- classify every site;
- compare with documentation/guards.

Acceptance evidence:

- machine-readable or reproducible inventory;
- no unclassified direct side-effect sites in the selected source scope.

### Work package B — Cross-domain provider composition tests

Intent:

Prove the provider bundle is coherent rather than several unrelated injection mechanisms.

Required changes:

- deterministic multi-provider run;
- concurrent different-bundle runs;
- broker event/counter assertions.

Acceptance evidence:

- no global provider-state leakage;
- expected providers receive exactly the intended operations.

### Work package C — Authority/accounting/cancellation integration qualification

Intent:

Prove the security/reliability objective of M005.

Required changes:

- hostname resolution -> concrete endpoint -> connect tests;
- network/HTTP authority tests in Eggsec;
- cancellation around provider-backed blocking operations;
- resource counter/byte accounting around actual provider calls.

Acceptance evidence:

- denied operations do not reach host providers;
- approved operations retain endpoint/authority identity;
- counters reflect actual provider execution.

### Work package D — Portability matrix

Intent:

Prove native providers isolate platform mechanics.

Required changes:

- Linux/macOS/Windows compile/check;
- platform-safe provider tests;
- Unix-specific tests where required;
- no unguarded Unix imports in portable core/library modules.

Acceptance evidence:

- CI matrix green or unsupported behavior explicitly fails at feature/runtime boundary rather than compile time.

### Work package E — Eggsec consumer and guard qualification

Intent:

Ensure the principal consumer uses the intended provider seam without ownership regression.

Required changes:

- scoped HTTP/provider integration tests;
- TUI/Python NSE smoke;
- dependency and architecture guards;
- `make check`.

Acceptance evidence:

- standalone remains Eggsec-independent;
- Eggsec remains the sole direct consumer;
- adapters stay engine-owned.

### Work package F — Documentation reconciliation and release disposition

Intent:

Make provider coverage claims accurate and decide the next operational step.

Required changes:

- update standalone provider/compatibility docs;
- update Eggsec NSE architecture/capability inventory;
- identify residual specialized direct host paths;
- recommend whether provider API warrants `eggsec-nse 0.2.0`, a compatible 0.1.x release, or further corrective work.

Acceptance evidence:

- docs match source inventory;
- release recommendation is explicit and evidence-based.

## 8. Failure, cancellation, restart, and contention semantics

No new runtime semantics should be introduced.

Any failure indicating provider state is process-global, authority is lost, cancellation is bypassed, or counters do not reflect actual host operations is a closure blocker.

No durable restart behavior.

## 9. Compatibility and migration

M005 should be additive for current runtime consumers.

If public provider types are added, closure must identify their semver implications. Do not publish from this plan; hand release work to a separate release/adoption plan after qualification.

Residual native compatibility shims must be documented with removal criteria.

## 10. Required tests

### Focused unit tests

Only missing regression tests discovered during the coverage audit.

### Integration tests

- multi-provider deterministic run;
- Eggsec authority-backed HTTP/provider run;
- local DNS/TCP/UDP/HTTP/filesystem/process representative flows.

### Restart and recovery tests

Not applicable.

### Contention and cancellation tests

- concurrent distinct service bundles;
- virtual CWD isolation;
- cancellation on network/filesystem/process/provider operations.

### Security and negative tests

- runtime denial provider-not-called;
- Eggsec out-of-scope provider-not-called;
- no hostname re-resolution after approval;
- no direct bypass in guarded migrated modules.

### Migration and compatibility tests

- corpus/local protocol suites;
- TUI/Python consumer smoke;
- report serialization/API compatibility;
- MSRV/package;
- Windows compile/check.

## 11. Required verification commands

Standalone:

```bash
cargo fmt --all --check
./scripts/check-boundaries.sh
cargo check --no-default-features
cargo check --features nse
cargo test --features nse
cargo check --features nse-ssh2
cargo check --features nse,sandbox
cargo clippy --all-targets --features nse
cargo +1.89.0 check --locked --no-default-features
cargo +1.89.0 check --locked --features nse
cargo package
```

Run all provider-specific integration suites and the source-coverage guard/inventory command.

Eggsec:

```bash
cargo check -p eggsec --features nse,cli
cargo check -p eggsec --features nse-ssh2,nse-sandbox,cli
cargo test -p eggsec --features nse,cli --lib
cargo test -p eggsec --features nse,cli --test nse_bridge_tests --test nse_integration_tests --test nse_real_scripts --test nse_tests
cargo test -p eggsec-tui --features nse
cargo test -p eggsec-python --features nse
make check-deps
make test-architecture-guards
make check-features-individual
make check
make check-python
```

## 12. Documentation updates

Standalone:

- provider architecture;
- compatibility/provider coverage;
- platform support;
- contributor verification.

Eggsec:

- `architecture/nse_capability_inventory.md`;
- `architecture/nse_integration.md`;
- `docs/NSE_COMPATIBILITY.md`;
- architecture guard docs;
- subsystem roadmap and registry at closure.

## 13. Acceptance criteria

1. Every direct host side-effect site in the audited source scope is classified.
2. Provider-backed domains use one per-run service bundle and broker sequence.
3. No monolithic host trait exists.
4. Denied runtime operations do not call providers.
5. Network resolution/authorization/connection preserves concrete endpoint identity.
6. Eggsec scoped HTTP/provider execution carries existing approved authority with no native fallback.
7. Actual provider operations drive resource accounting/capability events.
8. Concurrent runs isolate provider state and CWD.
9. Linux/macOS/Windows qualification is green for the declared support matrix.
10. Standalone remains free of Eggsec dependencies.
11. Eggsec consumer/facade/adapter ownership remains correct.
12. Existing report/profile/feature behavior remains compatible.
13. Residual specialized direct host paths are explicitly documented and guarded against accidental expansion.
14. Documentation claims match the source inventory.
15. Closure makes an explicit release/versioning recommendation.

## 14. Stop conditions

Stop and require corrective planning if:

- any high-severity unclassified/bypass side effect remains;
- provider contracts require redesign rather than small integration fixes;
- Eggsec authority is bypassed or reconstructed from runtime target data;
- Windows support requires weakening security invariants;
- report/public behavior changes materially;
- a broad new migration domain is discovered.

## 15. Closure evidence required

- 005A-D closure references;
- source-side-effect inventory and classifications;
- provider API inventory;
- deterministic/concurrent composition results;
- authority-preservation tests;
- cancellation/accounting tests;
- platform CI matrix;
- Eggsec consumer/guard verification;
- corpus/report compatibility results;
- residual risk list by severity;
- release/versioning recommendation;
- parent M005 closed/conditionally-closed/corrective recommendation.

## 16. Handoff notes

Do not turn this into a fifth implementation rewrite. The purpose is to prove the architectural outcome of M005 and make remaining exceptions explicit.

If the audit discovers a new major provider domain, write a corrective/new milestone plan rather than silently absorbing it here.
