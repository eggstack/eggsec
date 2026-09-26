# NSE Runtime Extraction Milestone 004 — Versioned Release and Eggsec Registry Adoption

Status: ready for handoff

Eggsec repository baseline: `86dd20c4df4b4d9e6ece1282e158bf096816190a`

Standalone runtime baseline: `eggstack/eggsec-nse@3f57e6c33c8fb17f39ddfcd3bde80a2bde6769a2`

Source roadmap:

- `plans/subsystems/nse-runtime-extraction-roadmap.md#milestone-004--versioned-release-and-eggsec-adoption`

Hard-dependency closure:

- `plans/closure/nse-runtime-extraction/003-closure.md` — closed; standalone extraction and exact-revision cross-repository qualification complete.

Long-term requirements:

- `plans/000-long-term-specification.md#2-primary-product-goals`
- `plans/000-long-term-specification.md#5-crate-ownership`
- `plans/001-terminology-and-domain-model.md#3-execution-terms`
- `plans/001-terminology-and-domain-model.md#5-data-and-reporting-terms`
- `plans/002-long-term-roadmap.md#phase-7--standing-maintenance-and-future-capability-open`

Applicable ADRs:

- `plans/adrs/ADR-0001-scoped-transport-eggfetch-backend.md` remains controlling for the Eggsec-owned scoped HTTP adapter; publishing the runtime must not move transport authority into it.
- `plans/adrs/ADR-0002-eggress-selective-reuse-boundary.md` is unaffected.
- No new ADR is required if this milestone only establishes the first standalone package release and changes Eggsec's dependency source from the qualified Git revision to crates.io. Stop if implementation requires a public-contract, authorization, or transport redesign.

Primary class: capability

Affected repositories:

- release producer: `eggstack/eggsec-nse`;
- release consumer and planning authority: `eggstack/eggsec`.

## 1. Objective

Publish the first versioned `eggsec-nse` release to crates.io from an independently qualified standalone release commit, establish durable release metadata and post-bootstrap publishing controls, then replace Eggsec's temporary exact Git-revision dependency with the published registry version and requalify Eggsec against the registry artifact.

The milestone is complete only when:

1. the registry artifact is publicly resolvable and independently verifiable;
2. the source tag/GitHub release identifies the exact source commit used to publish it;
3. Eggsec no longer permits or consumes the temporary Git source;
4. Eggsec's lockfile resolves the released crates.io package;
5. standalone and Eggsec verification remain green.

The intended initial public version is `0.1.0`, which already matches the standalone package metadata. If `eggsec-nse 0.1.0` has been published or the crate name has become unavailable before implementation begins, stop and record the registry state rather than silently renaming the package or choosing a different version.

## 2. Why this milestone is ready

Milestone 003 closed the hard dependency:

- `eggstack/eggsec-nse` exists and is independently buildable;
- standalone Linux/macOS CI, Rust 1.89 qualification, package verification, clean-room corpus, and real loopback SSH runtime qualification passed at `3f57e6c33c8fb17f39ddfcd3bde80a2bde6769a2`;
- Eggsec consumes that exact commit and passes its NSE/TUI/Python/feature checks;
- the local runtime source has been removed;
- only Eggsec directly consumes the runtime;
- runtime/Eggsec ownership is guarded.

Eggsec's current main push CI for the Milestone 003 closure also passed its Rust, Python, and dependency-policy jobs.

Operational release prerequisites are intentionally checked inside this milestone before any irreversible action:

- confirm `eggsec-nse` is still available/owned as expected on crates.io;
- confirm an authorized crates.io account/API token is available for the first publish;
- confirm the release commit has clean CI and reproducible package evidence.

Crates.io package names are first-come-first-served, and the first release of a new crate must be published with ordinary authenticated Cargo publication before Trusted Publishing can be configured. After the initial release exists, Trusted Publishing may be configured for later versions. These are operational gates, not reasons to keep the implementation plan blocked.

## 3. Current implementation evidence

### Standalone runtime

At `eggstack/eggsec-nse@3f57e6c3`:

- package name/version are `eggsec-nse 0.1.0`;
- edition is 2021 and MSRV is 1.89;
- license is MIT;
- repository/homepage/documentation/readme metadata are explicit and standalone;
- dependencies have no Eggsec workspace edges;
- README and provenance docs describe clean-room corpus and embedding authorization boundaries;
- `cargo package` passed;
- GitHub Actions has Linux/macOS, MSRV, package, and loopback SSH runtime qualification;
- there is no published GitHub Release in the standalone repository at plan authoring time;
- no release/publish workflow is currently part of the repository.

Public web search at plan authoring time returned no result for an existing `eggsec-nse` crate, but this is not authoritative registry reservation evidence. The implementation must query crates.io/Cargo immediately before first publish.

### Eggsec

At `eggstack/eggsec@86dd20c4`:

```toml
eggsec-nse = {
    git = "https://github.com/eggstack/eggsec-nse",
    rev = "3f57e6c33c8fb17f39ddfcd3bde80a2bde6769a2",
    version = "0.1.0",
    optional = true
}
```

The lockfile records the same Git revision.

`deny.toml` currently permits exactly the standalone Git URL and requires a `rev`.

Architecture Check 144 verifies:

- no local runtime source;
- canonical Git repository;
- a full exact SHA;
- lockfile equality with the manifest revision.

Checks 145-146 verify single direct consumer, engine facade/adapter ownership, and canonical execution integration.

Milestone 004 must remove the temporary Git-specific policy rather than leave dormant exceptions after registry adoption.

## 4. Invariants that must not regress

- Eggsec remains the product authorization and scope authority; the released runtime is not an Eggsec authorization layer.
- `eggsec-nse` remains scanner-independent and has zero `eggsec-*` dependencies.
- Package name remains `eggsec-nse`.
- Runtime features remain `nse`, `nse-ssh2`, `sandbox`, and `stress-testing`.
- Eggsec features remain `nse`, `nse-ssh2`, and `nse-sandbox`.
- `eggsec::nse` remains the Eggsec-facing facade and TUI/Python remain indirect consumers.
- `nse_bridge` and `nse_http_capability` remain Eggsec-owned.
- `NseRunRequest -> execute_nse_run -> NseRunReport` remains the canonical runtime pipeline.
- `NseRunReport` serialization and execution-profile semantics remain compatible.
- Resolver containment, capability policy, limits, cancellation, sandbox behavior, and clean-room corpus provenance remain intact.
- The published archive must correspond to the qualified source commit; do not publish from a dirty tree.
- The first published version is immutable. Never use `--allow-dirty` or `--no-verify` to force publication.
- No crates.io token, OIDC token, release credential, test password, or other secret may be committed, printed into closure evidence, or persisted in repository files.
- Registry adoption must remove the Git source exception and exact-SHA guard rather than leaving two accepted runtime sources.
- Publication does not authorize upstream Nmap script/nselib redistribution.
- Milestone 004 does not introduce provider inversion or HTTP backend changes.

## 5. Scope

### In scope

Standalone repository:

- verify crate-name/version availability and owner/auth readiness;
- add or reconcile release documentation/changelog needed for a first public release;
- define the exact release candidate commit;
- rerun full standalone qualification on that commit;
- run `cargo publish --dry-run` / package inspection from a clean checkout;
- manually/authenticated-publish `eggsec-nse 0.1.0` to crates.io;
- wait for and verify registry/index availability;
- verify the published package source/metadata matches the release candidate;
- create an annotated/lightweight `v0.1.0` tag according to repository convention pointing at the published source commit;
- create a GitHub Release for `v0.1.0` with concise release notes and verification identity;
- after first publish, configure/document Trusted Publishing for future releases if repository/account permissions permit;
- add a future-release workflow only if it is safe, credential-minimal, and cannot republish `0.1.0` accidentally.

Eggsec repository:

- replace the Git dependency with the crates.io version;
- update `Cargo.lock`;
- remove the `allow-git` exception for `eggsec-nse` and retain fail-closed unknown-git policy;
- retarget Check 144 from exact Git-revision enforcement to released-registry-source enforcement;
- preserve Checks 145-146 ownership/integration behavior;
- update architecture/release/NSE documentation and the NSE skill;
- run full cross-repository/adoption qualification;
- close Milestone 004 only after the crates.io artifact itself is the dependency under test.

Planning reconciliation:

- correct the stale Milestone 003 roadmap/control-point status before registering Milestone 004 as the active handoff.

### Explicitly out of scope

- publishing an Eggsec release;
- publishing `eggsec-nse` under a different package name;
- publishing a second runtime version merely to exercise automation;
- changing public NSE behavior to justify a version bump;
- provider inversion;
- switching Lua HTTP libraries to `nse_http_capability`;
- API pruning/package decomposition;
- bundling upstream Nmap scripts/nselib;
- changing Eggsec authorization/scope semantics;
- adding TUI/Python direct runtime dependencies;
- Milestone 005 implementation.

## 6. Required production changes

### Core/domain

The runtime code should not require behavior changes for this milestone.

If release qualification exposes a packaging-only defect (missing included file, bad README link, invalid metadata, docs.rs configuration, etc.), fix the smallest release defect and create a new release candidate commit. Any source/API change requires full standalone qualification again and must be documented.

Eggsec should change only dependency source/policy and any required documentation/guards.

### Storage and migrations

No persistent application data migration.

Cargo source migration:

Before:

```text
eggsec -> git+https://github.com/eggstack/eggsec-nse?rev=<qualified-sha>
```

After:

```text
eggsec -> registry+https://github.com/rust-lang/crates.io-index : eggsec-nse 0.1.0
```

The lockfile must show the registry package rather than a Git source.

### Protocol and DTOs

No protocol, report schema, Python DTO, TUI DTO, or CLI contract change is intended.

The published `0.1.0` package must expose the same runtime API qualified in Milestone 003.

### Runtime and concurrency

No lifecycle change.

Standalone release-candidate CI must continue to exercise:

- normal NSE execution;
- cancellation and limits coverage;
- compatibility corpus;
- sandbox feature;
- SSH2 runtime path against disposable loopback OpenSSH;
- MSRV 1.89.

### Frontend or operator surface

Eggsec users should observe no NSE behavior change.

Dependency acquisition changes from Git to crates.io, which improves standard Cargo consumption but must not alter feature behavior.

### Security and authorization

First publication is an irreversible supply-chain event. Treat release authentication as a security boundary.

Required controls:

- publish from a clean, reviewed release commit only;
- use ordinary authenticated `cargo publish` for the first release;
- do not pass tokens on a command line that will be captured in logs;
- do not store credentials in repository files;
- after the initial crate exists, prefer crates.io Trusted Publishing for future releases if account/repository administration permits;
- if Trusted Publishing is configured, use an explicit GitHub environment and minimum `id-token: write` / `contents: read` permissions;
- never use `pull_request_target` or `workflow_run` as a publish trigger;
- do not auto-publish from ordinary pushes to `main`;
- retain human/version/tag checks before any future publish job.

Eggsec's Cargo policy must return to denying unknown/unnecessary Git sources after registry adoption.

### Documentation and static guards

Standalone documentation should include:

- `CHANGELOG.md` or an equivalent curated `0.1.0` release record;
- `docs/RELEASING.md` describing first-release bootstrap and subsequent releases;
- release verification commands;
- tag/version/source invariants;
- rollback/yank guidance that recognizes published versions are immutable;
- future Trusted Publishing setup if adopted.

Eggsec documentation/guards should state:

- `eggsec-nse` is now an external crates.io dependency;
- runtime upgrades require explicit version/lockfile change plus standalone + Eggsec qualification;
- Check 144 rejects a Git/path runtime source;
- only Eggsec may directly consume the runtime;
- engine-owned NSE adapters remain local.

## 7. Ordered work packages

### Work package A — Reconcile Milestone 003 planning state and freeze Milestone 004 release intent

Intent:

Ensure planning control surfaces describe reality before the irreversible release begins.

Required changes:

- mark Milestone 003 closed in the subsystem roadmap and link its closure;
- advance the registry control point to Milestone 004;
- keep 005 not started/evidence-gated;
- record the intended first version as `0.1.0`.

Acceptance evidence:

- roadmap, registry, 003 plan, and 003 closure all agree;
- only Milestone 004 is the active handoff.

### Work package B — Registry/name/authentication preflight

Intent:

Prove publication is possible before modifying the release candidate.

Required changes:

- query crates.io for exact package name `eggsec-nse`;
- verify whether `0.1.0` already exists;
- verify the publishing user/account is authenticated and permitted;
- confirm required package metadata is accepted by current Cargo/crates.io;
- record current crates.io release mechanism constraints.

Decision rules:

- if the name is unclaimed and version absent, continue;
- if the crate already exists and is owned by the expected maintainer, inspect state and stop for reconciliation rather than duplicate-publishing;
- if the name is owned by another party, stop; do not silently rename;
- if release credentials/permissions are unavailable, complete only reversible prep and report the operational block.

Acceptance evidence:

- closure records the registry state without exposing credentials;
- irreversible publishing does not begin on assumptions.

### Work package C — Prepare a standalone 0.1.0 release candidate

Intent:

Create one immutable source commit suitable for publication.

Required changes:

- add/reconcile release notes/changelog;
- add `docs/RELEASING.md`;
- ensure README examples work against the package form;
- inspect package file list and provenance;
- ensure package metadata/version remain `0.1.0`;
- ensure no accidental local/path/Git-only dependency is present;
- ensure package contents exclude secrets/build artifacts/unlicensed corpus material.

Acceptance evidence:

- one release-candidate commit SHA is recorded;
- working tree is clean;
- package content is reviewable and deterministic enough for source/archive comparison.

### Work package D — Fully qualify the release candidate

Intent:

Do not publish a commit that has only packaging checks.

Required changes:

Run/re-run standalone CI and focused local verification against the exact candidate:

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
cargo clippy --all-targets --features nse -- -D warnings
cargo +1.89.0 check --locked --no-default-features
cargo +1.89.0 check --locked --features nse
cargo publish --dry-run
cargo package --list
cargo package
```

CI must again exercise the real loopback SSH2 runtime test.

Acceptance evidence:

- all required jobs are green at the release-candidate SHA;
- package dry-run succeeds without bypass flags;
- archive/file-list evidence is retained.

### Work package E — Publish and verify `eggsec-nse 0.1.0`

Intent:

Perform the one irreversible operation only after the candidate is qualified.

Required changes:

- publish from the exact clean candidate using authenticated `cargo publish`;
- wait for registry/index availability;
- verify `cargo search`/registry metadata resolves `eggsec-nse 0.1.0`;
- in a clean scratch crate, resolve/build the published package by version with required feature combinations;
- compare the published crate source/metadata against the release candidate/package archive where practical;
- verify docs.rs ingestion when available, but do not fail the entire release solely on asynchronous documentation build latency unless the build itself reports a real package defect.

Acceptance evidence:

- public registry version is resolvable;
- a clean Cargo consumer builds the registry artifact;
- published metadata points to the standalone repository/license/readme;
- no Git checkout is required.

### Work package F — Tag and create the GitHub release

Intent:

Tie source-control release identity to the immutable registry artifact.

Required changes:

- create `v0.1.0` pointing exactly to the published candidate SHA;
- create a GitHub Release for that tag;
- record the crates.io package/version and principal qualification evidence;
- do not move/recreate the tag after publication.

Acceptance evidence:

- tag commit equals the release-candidate/published source commit;
- release notes identify compatibility scope and do not overclaim full Nmap parity.

### Work package G — Establish future publishing controls

Intent:

Avoid retaining long-lived crates.io API tokens as the normal release mechanism.

Required changes:

After `0.1.0` exists:

- configure crates.io Trusted Publishing for `eggstack/eggsec-nse` if account/repository permissions permit;
- optionally enable Trusted-Publishing-only mode once a tested workflow exists and recovery ownership is understood;
- add a minimal future-release GitHub Actions workflow using the official crates.io authentication action or current supported mechanism;
- require explicit version/tag consistency;
- use minimum permissions and an explicit release environment;
- ensure ordinary pushes/PRs cannot publish;
- document manual recovery/owner procedure.

Do not publish another version merely to prove the workflow.

If Trusted Publishing cannot be configured during the milestone, record that as a low/operational residual and retain a documented manual-token release path; this does not invalidate the already-published 0.1.0 artifact.

Acceptance evidence:

- future release procedure is documented;
- no repository secret is required when Trusted Publishing is successfully configured;
- workflow static/security review passes.

### Work package H — Switch Eggsec to the crates.io release

Intent:

Make the public registry artifact the production dependency.

Hard gate:

Work packages B-F must be complete and the registry artifact must be independently resolvable.

Required changes:

- replace the Git/rev dependency with the released version;
- use the repository's normal version-spec convention unless a stricter exact requirement is intentionally adopted;
- update `Cargo.lock`;
- remove the `eggsec-nse` Git allow-list entry from `deny.toml`;
- retain `unknown-git = "deny"`;
- retarget Check 144 to require a crates.io/registry dependency and reject `path`, `git`, `branch`, `tag`, or `rev` source declarations for `eggsec-nse`;
- verify the lockfile source is registry-backed and version `0.1.0`;
- preserve Checks 145-146.

Acceptance evidence:

- `cargo tree -p eggsec --features nse,cli -i eggsec-nse` resolves `eggsec-nse v0.1.0` from crates.io;
- no runtime Git source exception remains;
- no local/sibling standalone checkout is required.

### Work package I — Cross-repository registry-artifact qualification

Intent:

Prove Eggsec behavior against the package users will actually receive.

Required changes:

Run at minimum:

```bash
cargo metadata --no-deps
cargo tree -p eggsec --features nse,cli -i eggsec-nse
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

Acceptance evidence:

- all relevant Eggsec verification is green with the registry artifact;
- lockfile and dependency tree contain no Git runtime source;
- public feature behavior and reports remain compatible.

### Work package J — Documentation and closure

Intent:

Make the release source-of-truth obvious to maintainers and future agents.

Required changes:

- update Eggsec architecture/NSE/release docs and `.opencode/skills/eggsec-nse/SKILL.md`;
- remove stale descriptions of the Git revision as the active runtime source from current docs;
- preserve Milestone 003/historical evidence unchanged where it intentionally records the prior Git phase;
- update roadmap and registry only after closure evidence exists.

Acceptance evidence:

- current docs say crates.io release is authoritative;
- historical extraction evidence remains traceable;
- closure can recommend whether Milestone 005 should proceed from concrete portability/provider evidence or remain deferred.

## 8. Failure, cancellation, restart, and contention semantics

Runtime execution semantics are unchanged.

Release failure semantics are important:

- all work before `cargo publish` is reversible;
- once `0.1.0` is successfully published, do not attempt to overwrite it;
- if a post-publish source/tag problem is discovered, do not move the release tag to a different source commit while claiming it is the published source;
- if the artifact is critically defective, evaluate crates.io yank semantics and publish a corrected version under a new semver version through a separate corrective plan;
- do not switch Eggsec to the registry artifact until package verification is complete;
- if Eggsec qualification fails against the published artifact, leave Eggsec on the previously qualified Git revision while diagnosing and create a corrective release/adoption plan rather than forcing the dependency switch.

No background daemon/restart behavior is introduced.

## 9. Compatibility and migration

Expected user-visible behavior is unchanged.

Source migration:

```text
Milestone 003:
eggsec -> eggsec-nse 0.1.0 from exact Git rev

Milestone 004:
eggsec -> eggsec-nse 0.1.0 from crates.io
```

The registry artifact becomes the canonical consumable package.

Maintain:

- `eggsec::nse` facade;
- feature names;
- runtime public API;
- report serialization;
- Python/TUI behavior.

Do not use this milestone to make a semver-significant API cleanup. If a release-blocking API defect requires such a change, stop and re-scope/version intentionally.

## 10. Required tests

### Focused unit tests

Standalone:

- all existing runtime unit tests;
- release/version metadata tests/guards where added.

Eggsec:

- existing engine adapter/facade tests.

### Integration tests

Standalone:

- full clean-room compatibility/runtime corpus;
- local protocol fixtures;
- SSH2 loopback runtime qualification;
- clean scratch consumer against the published crates.io artifact.

Eggsec:

- NSE engine/bridge/real-script integration;
- TUI;
- Python.

### Restart and recovery tests

Not applicable to runtime behavior.

Release recovery is procedural: verify tag/source/version identity and document yank/new-version handling.

### Contention and cancellation tests

Re-run standalone canonical cancellation/concurrent isolation coverage as part of normal runtime tests.

### Security and negative tests

- standalone boundary guards;
- no Eggsec dependencies;
- no unlicensed upstream corpus;
- no secrets in package/repository/workflow;
- publish workflow cannot run from PRs or ordinary main pushes;
- Eggsec guard rejects Git/path runtime dependency after adoption;
- Eggsec remains sole direct consumer;
- engine adapters stay local;
- Cargo Deny returns to rejecting unexpected Git sources.

### Migration and compatibility tests

- release candidate versus published artifact metadata/source comparison;
- scratch-crate compile from crates.io;
- report serialization round-trip;
- `nse`, `nse-ssh2`, `sandbox` feature combinations;
- Eggsec full feature sweep;
- clean Eggsec checkout builds without sibling `eggsec-nse` repo.

## 11. Required verification commands

Standalone pre-publish:

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
cargo clippy --all-targets --features nse -- -D warnings
cargo +1.89.0 check --locked --no-default-features
cargo +1.89.0 check --locked --features nse
cargo publish --dry-run
cargo package --list
cargo package
```

First publication, only after all gates:

```bash
cargo publish
```

Do not include credentials in the command transcript.

Post-publish, use a clean temporary Cargo consumer to resolve `eggsec-nse = "0.1.0"` from crates.io and exercise the needed features.

Eggsec post-adoption:

```bash
cargo metadata --no-deps
cargo tree -p eggsec --features nse,cli -i eggsec-nse
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

Do not claim any command in closure evidence unless it actually ran.

## 12. Documentation updates

Standalone:

- `CHANGELOG.md` or equivalent release-history file;
- `docs/RELEASING.md`;
- README installation example changed from Git/rev to crates.io after publication;
- CONTRIBUTING release-verification guidance if appropriate;
- future Trusted Publishing workflow/docs if configured.

Eggsec:

- `architecture/overview.md`;
- `architecture/nse_integration.md`;
- `docs/NSE_COMPATIBILITY.md`;
- `docs/CI_ARCHITECTURE_GUARDS.md`;
- release/dependency policy docs affected by removal of the Git exception;
- `.opencode/skills/eggsec-nse/SKILL.md`;
- current README/workspace/dependency references where the Git source is described as current;
- subsystem roadmap/registry after closure.

Historical Milestone 003 evidence that accurately describes the former exact-Git phase must remain unchanged.

## 13. Acceptance criteria

1. Milestone 003 planning status is reconciled as closed before 004 handoff is considered active.
2. Registry preflight confirms the canonical package name/version can be published by the authorized maintainer; no silent rename/version substitution occurs.
3. A clean standalone release-candidate commit is fully qualified by current CI, MSRV 1.89, corpus, feature, package, and SSH runtime checks.
4. `cargo publish --dry-run` and package inspection pass without `--allow-dirty` or `--no-verify`.
5. `eggsec-nse 0.1.0` is successfully published and publicly resolvable from crates.io.
6. A clean scratch Cargo project can resolve/build the published artifact without any Git/path dependency.
7. Published metadata/source/provenance correspond to the qualified standalone release candidate.
8. `v0.1.0` and the GitHub Release point to the exact published source commit.
9. Release notes accurately describe practical/curated NSE compatibility and do not claim full Nmap parity.
10. Future release procedure is documented; Trusted Publishing is configured where operationally possible after first publish, with safe trigger/permission boundaries.
11. Eggsec consumes the released crates.io version rather than Git/path source.
12. Eggsec `Cargo.lock` resolves `eggsec-nse 0.1.0` from the registry.
13. The prior `allow-git` exception for `eggsec-nse` is removed while `unknown-git = "deny"` remains.
14. Check 144 rejects Git/path runtime consumption and verifies the registry version/source; Checks 145-146 remain green.
15. Only Eggsec directly consumes `eggsec-nse`; TUI/Python remain behind `eggsec::nse`.
16. Eggsec NSE/TUI/Python tests, dependency policy, architecture guards, feature sweep, `make check`, and Python verification pass against the registry artifact.
17. No authorization, profile, report, resolver, cancellation, limit, sandbox, or corpus-provenance regression is introduced.
18. Closure identifies both the standalone release commit/tag/version and the Eggsec adoption commit and recommends Milestone 005 disposition.

## 14. Stop conditions

The agent must stop and report rather than improvise when:

- `eggsec-nse` is already owned on crates.io by an unexpected party;
- `0.1.0` already exists but does not correspond to the intended release candidate;
- publishing credentials/permissions are unavailable;
- package verification requires `--allow-dirty` or `--no-verify`;
- publication would include unclear/unlicensed corpus material;
- a public API/report compatibility break is required;
- runtime authorization/scope responsibility would have to change;
- Eggsec cannot consume the crates.io artifact without adding a direct frontend dependency or bypass;
- release qualification fails and the proposed fix changes runtime semantics materially;
- implementation expands into provider inversion, HTTP backend cutover, or Milestone 005;
- a failed/defective published release would require a yank/new patch version: create a corrective release plan rather than silently extending this plan.

## 15. Closure evidence required

The closure record must contain:

- Milestone 003 closure reference;
- pre-publish crates.io name/version/ownership result;
- release-auth readiness statement with no credential material;
- standalone release-candidate SHA;
- standalone CI run(s) and MSRV/SSH/package/corpus results;
- `cargo publish --dry-run` result;
- package file count/list review and provenance result;
- published crate name/version and public registry evidence;
- clean scratch-consumer result against crates.io;
- published archive/source identity comparison;
- tag and GitHub Release identity;
- Trusted Publishing/future release-control disposition;
- Eggsec manifest before/after dependency source;
- Eggsec lockfile registry-source evidence;
- Cargo Deny removal of the temporary Git exception;
- Check 144-146 results after retargeting;
- Eggsec NSE/TUI/Python/feature/`make check`/`make check-python` results;
- security/authorization invariant review;
- unresolved findings classified by severity;
- explicit GO/NO-GO disposition for Milestone 005.

## 16. Handoff notes

Treat `cargo publish` as the irreversible boundary.

Do all metadata, docs, packaging, provenance, CI, MSRV, corpus, and SSH qualification before that command. Once publication succeeds, preserve the source commit and release tag identity.

The first release must use ordinary authenticated Cargo publication because crates.io Trusted Publishing cannot bootstrap a crate that does not yet exist. Configure Trusted Publishing only after the initial crate exists and only if the repository/account permissions make it safe to do so.

Do not place the crates.io token in repository secrets unless the chosen first-release procedure specifically requires it and the user has approved that mechanism; a local authenticated first publish is acceptable. Never echo the token.

After publishing, qualify the actual registry artifact before changing Eggsec. The Git-pinned Milestone 003 dependency is the rollback point until registry-artifact qualification succeeds.

The release/adoption boundary is intentionally one milestone because the public artifact is not complete from Eggsec's perspective until the principal consumer has successfully moved from the temporary Git source to that exact released version.
