# NSE Runtime Extraction Milestone 007C — Standalone Security Patch Release

Status: ready for handoff

Unblocked: `plans/closure/nse-runtime-extraction/007b-closure.md` is accepted and hosted standalone CI is fully green on the closure SHA (`d4a22f1dbe56f4ccfb17b2a8135aae8395f44f19`, run `36640412317`, 5/5 jobs). Precondition from the original handoff satisfied.

Planning baseline: `386fe63a522aac66340386ac83e9f5ed54500d8b`

Standalone baseline for the compatibility gate: `eggstack/eggsec-nse@d4a22f1dbe56f4ccfb17b2a8135aae8395f44f19` (`0.2.0` published; corrected tree not yet released)

**Carry-forward from M007B closure.** The compatibility gate must be run against the *corrected* classification, not the inherited one. The public surface is larger than the red CI at `699d374` suggested: 65 registered libraries move from manual-only to `ProviderBacked` (automated-visible), plus `target` from `Pure`, and three new additive `*_with_services` registration entry points exist (`register_target_library_with_services`, `register_radius_library_with_services`, `register_dnsbl_library_with_services`). The pre-existing `register_*_library(lua)` signatures are retained, so this is expected to remain a `0.2.1` patch release; the gate, not this note, decides. Release notes must disclose the automated gating change and the corrected residual honestly (see `007b-closure.md` §5: 97 -> 22 residual, manifest manual-only 106 -> 41).

One medium finding is explicitly **not** a M007C matter: `broker_dns_lookup` does not evaluate per-target membership for the resolved name (closure §12). That is M007D scope.

Hard dependencies:

- accepted closure of `plans/implementation/nse-runtime-extraction/007-automated-library-effect-gate.md`;
- accepted closure of `plans/implementation/nse-runtime-extraction/007-broker-compatible-protocol-migration.md`.

Source roadmap:

- `plans/subsystems/nse-runtime-extraction-roadmap.md#milestone-007--protocol-library-gating-and-controlled-automated-activation`

Applicable ADR:

- `plans/adrs/ADR-0004-nse-automated-activation-boundary.md`

Primary class: capability + security release

Release repository:

- `eggstack/eggsec-nse`

## 1. Objective

Publish the qualified M007 standalone changes as the next compatible `0.2.x` security release so Eggsec can consume the real registry artifact before scoped automated activation work begins.

Expected version: **0.2.1**.

The expected patch release is valid only if M007A/B remain additive/source-compatible for current 0.2.0 consumers. If implementation requires a breaking public API, stop and replan the release as 0.3.0 rather than silently publishing an incompatible patch.

## 2. Release contents

Expected 0.2.1 notes must include:

- complete automated library effect/eligibility manifest;
- AgentSafe/CiSafe registration + require gate;
- authority-bound HTTP provider assurance;
- native/default HTTP denied for automated authority claims;
- broker-compatible protocol migrations;
- residual before/after counts and remaining manual-only effect classes;
- no claim that the remaining long-tail direct-I/O protocols are provider-backed.

## 3. Semver gate

Before version bump, perform a public API compatibility review against 0.2.0.

Patch release permitted when:

- existing constructors remain source-compatible;
- existing provider traits are not changed with required methods;
- existing public structs do not gain required fields;
- existing enum exhaustive-match surface is not changed;
- new APIs/builders/enums are additive;
- behavior tightening is confined to automated safety/security posture documented by existing profile contracts.

If any criterion fails:

- stop;
- mark this plan blocked;
- create a 0.3.0 release plan with explicit migration notes.

## 4. Scope

### In scope

- API compatibility audit;
- version/changelog/README updates;
- release workflow verification;
- exact candidate qualification;
- crates.io publication;
- tag/GitHub Release;
- docs.rs + clean scratch consumer verification;
- source/archive identity evidence.

### Out of scope

- Eggsec dependency adoption;
- Eggsec scope threading;
- adapter activation;
- automated surface metadata changes;
- additional residual migration after candidate freeze.

## 5. Required qualification

Exact candidate SHA:

```bash
cargo fmt --all --check
./scripts/check-boundaries.sh
cargo metadata --no-deps
cargo tree --workspace
cargo check --no-default-features
cargo check --features nse
cargo test --features nse
cargo check --features nse-ssh2
cargo check --features nse,sandbox
cargo clippy --all-targets --features nse
cargo +1.89.0 check --locked --no-default-features
cargo +1.89.0 check --locked --features nse
cargo publish --dry-run
cargo package --list
cargo package
```

Hosted Linux/macOS/Windows/MSRV/SSH CI must be green on the same candidate.

## 6. Publication rules

- Verify 0.2.1 is absent before publication.
- Prefer Trusted Publishing if the crates.io-side configuration has been completed since 0.2.0.
- Otherwise use the documented manual-token recovery path without exposing credentials.
- Never use `--allow-dirty` or `--no-verify`.
- Tag only after qualification; tag must resolve to published candidate source.
- Published artifact VCS identity must match candidate/tag.

## 7. Post-publication verification

Create a clean scratch consumer resolving exactly:

```toml
eggsec-nse = { version = "=0.2.1", features = ["nse"] }
```

Also qualify combinations used by Eggsec:

- `nse`;
- `nse-ssh2`;
- `nse,sandbox`.

Confirm registry source, docs.rs, and no Git/path fallback.

## 8. Acceptance criteria

1. API audit confirms patch compatibility or the plan stops for 0.3.0 replanning.
2. Release notes disclose automated gating and remaining residual honestly.
3. Exact candidate CI is fully green.
4. Package/dry-run passes.
5. 0.2.1 publishes from the exact candidate.
6. Tag and GitHub Release resolve to the candidate.
7. Published archive VCS identity matches.
8. Scratch consumer resolves registry 0.2.1.
9. docs.rs state is known.
10. No Eggsec change occurs here.
11. Closure unblocks M007D.

## 9. Stop conditions

Stop if:

- any breaking API change is found;
- candidate CI is red;
- residual documentation overclaims provider coverage;
- source/tag/archive identity diverges;
- publication authentication cannot be performed safely;
- Eggsec integration work leaks into the release slice.

## 10. Closure evidence

Record:

- M007A/B closures;
- API compatibility result;
- candidate SHA;
- hosted run;
- package/dry-run;
- publication method;
- registry artifact/source identity;
- tag/release/docs.rs;
- scratch consumer;
- exact version selected;
- GO/NO-GO for M007D.
