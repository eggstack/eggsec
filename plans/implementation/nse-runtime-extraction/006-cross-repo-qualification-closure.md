# NSE Runtime Extraction Milestone 006C — 0.2.0 Cross-Repository Qualification and Closure

Status: ready for handoff

Eggsec planning baseline: `51f9747deef7bb5c2488e7dd392b0f2c96667653`

Standalone runtime baseline: `eggstack/eggsec-nse@9fe149fbb22480a63e254e91d60083b7a29a8ff4`

Source roadmap:

- `plans/subsystems/nse-runtime-extraction-roadmap.md#milestone-006--020-release-and-safe-eggsec-adoption`

Hard dependencies:

- accepted closure of `plans/implementation/nse-runtime-extraction/006-standalone-0-2-0-release.md`;
- accepted closure of `plans/implementation/nse-runtime-extraction/006-eggsec-0-2-0-adoption-safe-staging.md`.

Applicable ADRs:

- `plans/adrs/ADR-0003-nse-host-provider-boundary.md`
- `plans/adrs/ADR-0001-scoped-transport-eggfetch-backend.md`

Primary class: infrastructure

Affected repositories:

- `eggstack/eggsec-nse`;
- `eggstack/eggsec`.

## 1. Objective

Qualify the published 0.2.0 artifact and Eggsec adoption as one release boundary, verify that the dormant adapter and automated-exposure quarantine match source reality, and close M006 only if the system is ready to move to the separate protocol-library capability-gating/scope-threading milestone.

This is a qualification/closure pass, not another feature tranche.

## 2. Why this slice exists

M006 contains one irreversible producer step and one consumer integration step. A separate closure pass is required because:

- registry publication must be verified against the actual artifact users receive;
- Eggsec must be tested against crates.io 0.2.0, not a local/path surrogate;
- the replayed adapter must remain dormant;
- automated NSE must remain fail-closed/manual-only while the M005E ungated residual exists;
- documentation and architecture guards must not overclaim provider-wide scope enforcement.

## 3. Invariants

- standalone 0.2.0 tag/source/archive identity is immutable and traceable;
- Eggsec resolves 0.2.0 from crates.io only;
- only Eggsec directly consumes the runtime;
- TUI/Python remain facade consumers;
- adapter remains engine-owned;
- adapter has no production dispatch caller;
- automated NSE exposure remains disabled/fail-closed;
- manual CLI/TUI NSE still works;
- no source path, Git override, or temporary patch remains;
- M005E residual inventories remain documented and unchanged unless separately corrected;
- no claim is made that protocol-wide NSE scope enforcement is complete.

## 4. Scope

### In scope

- verify crates.io 0.2.0 artifact identity and docs.rs state;
- verify Eggsec dependency tree/lockfile source;
- run standalone release smoke against registry artifact where useful;
- run full relevant Eggsec NSE/TUI/Python/feature/dependency/architecture checks;
- source-audit `nse_http_provider.rs` for production call sites;
- source-audit operation metadata/enumeration so automated NSE remains unavailable;
- verify strict dispatch cannot execute NSE through a hidden route;
- verify manual dispatch remains native/default-provider behavior;
- reconcile README/architecture/skill/release docs across both repos;
- close M006 and sequence M007.

### Explicitly out of scope

- protocol-library capability gating;
- production `NetworkAuthority` threading into NSE;
- re-enabling MCP/REST/agent/gRPC NSE;
- adapter activation;
- publishing a patch release;
- provider API redesign.

## 5. Ordered work packages

### A — Registry artifact identity

Confirm:

- crates.io version 0.2.0 resolves;
- `v0.2.0` points to the candidate source;
- published archive VCS identity matches;
- docs.rs result is known;
- clean scratch consumers build expected features.

### B — Eggsec registry-source qualification

Confirm:

```bash
cargo tree -p eggsec --features nse,cli -i eggsec-nse
cargo metadata --no-deps
make check-deps
```

No Git/path/patch source is acceptable.

### C — Consumer behavior qualification

Run:

```bash
cargo check -p eggsec --features nse-ssh2,nse-sandbox,cli
cargo test -p eggsec --features nse,cli --lib
cargo test -p eggsec --features nse,cli --test nse_bridge_tests --test nse_integration_tests --test nse_real_scripts --test nse_tests
cargo test -p eggsec-tui --features nse
cargo test -p eggsec-python --features nse
make check-features-individual
make check
make check-python
```

### D — Dormant-adapter proof

Source and test evidence must show:

- `NseHttpTransportProvider` exists and its tests pass;
- no production module constructs it;
- no production NSE request injects it through `NseHostServices`;
- manual NSE continues to use native default services;
- no native fallback claim is made for an automated path because automated NSE remains disabled.

### E — Automated-exposure quarantine proof

Verify:

- operation metadata/registry surfaces do not advertise NSE to MCP/REST/agent/gRPC;
- strict canonical/approved dispatch cannot run NSE before M007;
- manual/TUI operation remains exposed;
- documentation matches these facts.

### F — Documentation/guard reconciliation

Update current docs and architecture guards if any stale statement says:
- Eggsec has activated the scoped adapter;
- automated NSE is supported on 0.2.0;
- all protocol libraries are provider/capability-backed.

Historical M005 closure evidence remains historical.

## 6. Required tests

### Standalone

- clean scratch registry consumer;
- expected features compile;
- package source identity verification.

### Eggsec

- NSE engine integration;
- TUI;
- Python;
- adapter unit tests;
- dependency policy;
- architecture guards;
- feature sweep;
- full `make check`.

### Negative/security

- automated operation enumeration excludes NSE;
- strict approved NSE attempt fails closed;
- no adapter construction in production;
- no runtime source override;
- no authority fabricated from URL/target;
- residual inventory guard remains green.

## 7. Acceptance criteria

1. Published 0.2.0 artifact/source/tag identity is verified.
2. docs.rs state is recorded.
3. Eggsec manifest and lockfile resolve crates.io 0.2.0.
4. No Git/path/patch source exists.
5. Full relevant Eggsec verification passes.
6. Adapter code is present, compiled, and tested.
7. Adapter has no production caller.
8. Automated NSE exposure is disabled and strict dispatch fails closed.
9. Manual/TUI NSE remains functional.
10. TUI/Python remain indirect consumers.
11. Architecture/dependency guards reflect 0.2.0 and dormant-adapter state.
12. No provider-wide enforcement overclaim remains.
13. Closure explicitly sequences M007 protocol-library capability gating + scope-aware adapter activation.

## 8. Stop conditions

Stop and require corrective planning if:

- registry artifact identity differs from the release candidate;
- Eggsec needs a local/path/Git override;
- automated NSE remains exposed;
- a production caller activates the adapter before protocol gating;
- manual behavior regresses materially;
- source/docs claim complete NSE scope enforcement despite the pinned residual.

## 9. Closure evidence

Record:

- 006A closure;
- 006B closure;
- release/tag/source identity;
- crates.io/docs.rs evidence;
- Eggsec manifest/lockfile/cargo-tree evidence;
- adapter source/caller audit;
- automated-exposure metadata and strict-failure tests;
- manual/TUI/Python results;
- dependency/architecture/full-check results;
- unresolved risk list;
- GO/NO-GO for M007.

## 10. Handoff notes

The correct M006 outcome is deliberately conservative: Eggsec consumes the new runtime and carries the tested scoped adapter code, but automated NSE remains disabled.

M007 should own the security-sensitive work needed before activation: protocol-library capability gating, approved-scope/profile threading into NSE execution, scoped provider injection, and controlled re-enablement of automated surfaces.
