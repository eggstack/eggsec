# NSE Runtime Extraction Milestone 003 — Standalone Repository Extraction and Cross-Repository Qualification

Status: ready for handoff

Repository baseline: `d412e204e1866a8ea09d1d9d6e8be66f4a9097e2`

Source roadmap:

- `plans/subsystems/nse-runtime-extraction-roadmap.md#milestone-003--standalone-repository-extraction-and-cross-repository-qualification`

Hard-dependency closure:

- `plans/closure/nse-runtime-extraction/001-closure.md` — closed.
- `plans/closure/nse-runtime-extraction/002-closure.md` — closed; explicitly recommends GO for Milestone 003 planning.

Long-term requirements:

- `plans/000-long-term-specification.md#2-primary-product-goals`
- `plans/000-long-term-specification.md#5-crate-ownership`
- `plans/001-terminology-and-domain-model.md#3-execution-terms`
- `plans/001-terminology-and-domain-model.md#5-data-and-reporting-terms`
- `plans/002-long-term-roadmap.md#phase-7--standing-maintenance-and-future-capability-open`

Applicable ADRs:

- `plans/adrs/ADR-0001-scoped-transport-eggfetch-backend.md` remains controlling for the Eggsec-owned scoped HTTP adapter. This milestone must not turn the standalone runtime into an Eggsec transport owner.
- `plans/adrs/ADR-0002-eggress-selective-reuse-boundary.md` is unaffected and must remain so.
- No new ADR is required if this milestone performs only the repository split and exact-revision qualification described here. Stop if implementation requires a durable authorization, transport, or public-contract redesign.

Primary class: infrastructure

## 1. Objective

Physically extract the already-independent `eggsec-nse` runtime from the Eggsec workspace into a new standalone repository at `eggstack/eggsec-nse`, give that repository explicit package metadata, documentation, provenance assets, tests, and CI, then make Eggsec consume one exact standalone Git revision and qualify both repositories against that same revision.

This milestone is complete only when the standalone repository passes its own qualification and Eggsec passes its NSE and broad repository verification while consuming the exact external revision.

This milestone does **not** publish a crates.io release. Versioned publication and replacement of the temporary Git revision are Milestone 004.

## 2. Why this milestone is ready

The hard dependency is satisfied.

Milestone 001 established one canonical resolver/execution/report pipeline and closed with parity evidence.

Milestone 002 established the physical-extraction boundary:

- `eggsec-nse` has zero `eggsec-*` dependencies in its manifest, source, and tests;
- Eggsec-specific report and transport adapters are engine-owned;
- only `eggsec` directly consumes `eggsec-nse`;
- TUI and Python consume through `eggsec::nse`;
- guards 144/145/146 prevent inward-dependency and duplicate-consumer regression;
- the clean-room runtime corpus passes independently;
- `make check` and the feature matrix passed;
- the push CI for baseline `d412e204` completed successfully.

The target repository `eggstack/eggsec-nse` does not exist at plan authoring time, so repository bootstrap is an explicit first-class work package rather than an external hidden prerequisite.

Milestone 002 carried two qualifications into this plan:

1. exercise the `nse-ssh2` runtime path on an SSH-capable environment before making standalone SSH parity claims;
2. make the extraction disposition of the dormant engine seams explicit.

Their disposition for this milestone is:

- `eggsec::nse_bridge` remains **Eggsec-owned** and is not copied into the standalone repository. It remains a tested compatibility/composition adapter during Milestone 003; adopting it into a production report path or removing its public module is not required to prove the runtime extraction and would unnecessarily mix report-plumbing/API changes into the split.
- `eggsec::nse_http_capability` remains **Eggsec-owned** and is not copied into the standalone repository. The deferred Phase D HTTP-family backend cutover remains deferred; Milestone 003 must not rewrite Lua HTTP libraries or claim that this adapter is their active backend.

These decisions resolve repository ownership while preserving current behavior. A later plan may adopt or retire either engine seam based on direct product evidence.

## 3. Current implementation evidence

At the repository baseline:

- runtime source is rooted at `crates/eggsec-nse/`;
- the package is named `eggsec-nse`, version `0.1.0`, edition 2021, MIT, MSRV 1.89 through workspace inheritance;
- `crates/eggsec-nse/Cargo.toml` still inherits package metadata and many dependency versions from the Eggsec root workspace;
- its `readme` points to Eggsec's workspace-level `../../README.md`, which is not suitable after extraction;
- runtime tests and clean-room fixtures live with `crates/eggsec-nse/tests/`;
- runtime architecture/compatibility guidance is partly embedded in Eggsec-level docs and the Eggsec OpenCode skill;
- `crates/eggsec/src/lib.rs` exposes the supported Eggsec-facing facade with `pub use eggsec_nse as nse`;
- `crates/eggsec/src/nse_bridge.rs` and `crates/eggsec/src/nse_http_capability.rs` are already engine-owned and must remain in Eggsec;
- TUI and Python forward their `nse` feature through `eggsec/nse` and have no direct runtime dependency;
- guards 144/145/146 currently assume the runtime is present in-tree and therefore need to be replaced/retargeted when the local crate is removed;
- no `eggstack/eggsec-nse` GitHub repository exists yet.

The intended temporary post-milestone dependency is:

```text
eggstack/eggsec-nse @ exact commit SHA
            ^
            |
         eggsec
        /  |  \
      CLI TUI Python
```

The exact Git SHA is the qualification identity. No branch-only dependency is acceptable for the Milestone 003 Eggsec integration.

## 4. Invariants that must not regress

- Eggsec authorization remains authoritative and external to `eggsec-nse`; the standalone runtime does not authorize Eggsec operations.
- Strict Eggsec surfaces continue through the canonical Eggsec enforcement/dispatch path before NSE execution.
- The package name remains `eggsec-nse`.
- Existing runtime feature names and semantics remain available: `nse`, `nse-ssh2`, `sandbox`, and `stress-testing`.
- Existing Eggsec feature names remain available: `nse`, `nse-ssh2`, and `nse-sandbox`.
- `NseRunRequest`, `execute_nse_run`, execution-profile behavior, resolver policy, cancellation/limits semantics, compatibility/fidelity reporting, and `NseRunReport` serialization are not intentionally changed.
- Resolver containment, symlink-escape rejection, extension/size policy, and source/profile restrictions remain fail-closed.
- The clean-room corpus remains provenance-tracked. Do not copy or vendor the upstream Nmap script/nselib corpus.
- MIT licensing applies only to material Eggsec can lawfully distribute under that license; fixture provenance notes move with the runtime.
- Only Eggsec directly consumes the external runtime in the Eggsec workspace. TUI/Python continue through `eggsec::nse`.
- Cross-repository qualification always names the exact external commit. A moving branch must never be treated as qualification evidence.
- Ring-only rustls behavior and existing reqwest/rustls feature policy remain intact.
- Physical extraction must not be combined with provider inversion, HTTP backend cutover, package decomposition, or broad API pruning.

## 5. Scope

### In scope

- Create the canonical standalone repository `eggstack/eggsec-nse`.
- Transfer the runtime source, runtime-owned tests, clean-room fixtures, and runtime-owned documentation/provenance assets.
- Prefer a root-level single-crate layout in the standalone repository: `Cargo.toml`, `src/`, `tests/`, `README.md`, `LICENSE`, and supporting docs/scripts.
- Convert workspace-inherited package metadata and dependencies into standalone explicit metadata/version requirements.
- Set standalone metadata to the current contract unless an incompatibility is discovered: package `eggsec-nse`, version `0.1.0`, edition 2021, MIT, MSRV 1.89, repository/homepage/documentation pointing to `eggstack/eggsec-nse`.
- Add standalone README, license, provenance documentation, compatibility documentation, and contributor guidance sufficient to use and qualify the runtime without Eggsec.
- Add independent CI and static checks.
- Qualify `nse-ssh2` on an SSH-capable environment using a local/disposable test target; no public Internet target is required.
- Preserve source-history traceability. A history-preserving filtered/subtree extraction is preferred; if a clean snapshot bootstrap is materially safer, the initial standalone commit and provenance document must record the exact Eggsec source SHA and transfer inventory.
- After standalone CI is green, change Eggsec from its local workspace member/path dependency to an exact Git `rev` dependency on the qualified standalone commit.
- Remove the local `crates/eggsec-nse` source only in the same migration that establishes the exact external dependency.
- Update Eggsec guards, lockfile, docs, skills, workspace membership, and feature qualification for the external ownership model.
- Run standalone and Eggsec cross-repository qualification against the exact same runtime commit.
- Produce closure evidence with source/external commit mapping.

### Explicitly out of scope

- crates.io publication or a semver release/tag intended as the long-term dependency source.
- Renaming `eggsec-nse`.
- Moving Eggsec authorization, report-envelope DTOs, scoped transport authority, TUI/Python DTOs, or Eggsec policy into the standalone repository.
- Copying `nse_bridge.rs` or `nse_http_capability.rs` into the standalone repository.
- Activating `nse_http_capability` as the Lua HTTP-family backend.
- Replacing reqwest/native socket paths with provider traits.
- Removing or redesigning the broad `public_api`, CVE helpers, library compatibility surface, or built-in script APIs.
- Claiming full Nmap NSE compatibility.
- Bundling upstream Nmap scripts/nselib.
- Milestone 004 publication work.
- Unrelated dependency upgrades except where an explicit standalone version is required to reproduce the already-resolved workspace dependency.

## 6. Required production changes

### Core/domain

Standalone repository:

- move the contents of `crates/eggsec-nse/src/` to standalone `src/`;
- move runtime-owned integration tests and their clean-room fixtures to standalone `tests/`;
- preserve module/API names and feature gates;
- do not introduce Eggsec-specific types merely to compensate for the repository split;
- preserve the canonical `run.rs` pipeline as the only high-level execution owner.

Eggsec repository after external qualification:

- remove `crates/eggsec-nse` from workspace members;
- remove the local crate directory after the external exact revision is proven;
- change the engine's optional `eggsec-nse` dependency to the standalone Git source with `rev = "<qualified SHA>"`;
- preserve `pub use eggsec_nse as nse`;
- keep TUI/Python feature forwarding unchanged at the public surface.

Do not use a branch-only Git dependency. During Milestone 003, a tag without an exact `rev` is also insufficient as the qualification identity.

### Storage and migrations

No persistent data migration is expected.

The Cargo source graph and lockfile do change. The Eggsec `Cargo.lock` must resolve `eggsec-nse` to the exact Git source/commit used for qualification. Closure must record that source line/commit.

### Protocol and DTOs

No protocol or serialized DTO change is intended.

`NseRunReport` JSON contracts and Eggsec-facing report/view DTO behavior must remain compatible.

The runtime standalone repository must not acquire `eggsec-report-model` or any other Eggsec DTO dependency.

### Runtime and concurrency

No execution lifecycle redesign is intended.

Moving repository ownership must preserve:

- per-run executor isolation;
- cancellation token behavior;
- limits and sandbox behavior;
- resolver diagnostics;
- capability-event recording;
- static/dynamic require reporting;
- process-level TLS provider behavior.

Standalone CI must exercise the same representative runtime/corpus behavior as the in-tree baseline.

### Frontend or operator surface

Eggsec CLI/TUI/Python behavior should be unchanged. Existing users should not need to change feature names, NSE script names, report consumption, or Python APIs.

No new user-facing standalone CLI is required merely because the library moved. Existing runtime helper APIs remain available.

### Security and authorization

The repository split must not weaken Eggsec's outer authorization boundary.

The standalone README must state clearly that runtime profiles/capability checks are NSE execution controls and do not substitute for an embedding application's authorization/scope policy.

The standalone repository must retain negative tests for source policy, resolver containment, capability denial, limits, sandbox behavior, and cancellation.

For SSH qualification, use a loopback/local disposable SSH endpoint with deterministic credentials generated for the test environment. Do not rely on public scanning targets or persistent credentials.

### Documentation and static guards

Standalone repository must contain, at minimum:

- `README.md` describing scope, supported compatibility model, features, examples, safety/embedding model, and non-claim of full NSE parity;
- `LICENSE`;
- a provenance document identifying the Eggsec source commit and clean-room fixture policy;
- compatibility guidance derived from/migrated out of Eggsec documentation where it is runtime-owned;
- contributor/verification guidance;
- CI workflows or equivalent checks.

Eggsec must update:

- workspace layout/architecture documentation;
- NSE integration documentation to point to the external repository;
- `.opencode/skills/eggsec-nse/SKILL.md` to distinguish runtime-repo changes from Eggsec adapter changes;
- architecture guards 144/145/146 or their successors.

Post-extraction guards must assert at least:

1. no local `crates/eggsec-nse` workspace member/source tree has silently reappeared;
2. only `crates/eggsec/Cargo.toml` directly declares `eggsec-nse`;
3. the Eggsec dependency uses the canonical Git repository and an exact `rev`, not only a branch;
4. TUI/Python continue to consume NSE through `eggsec::nse`;
5. Eggsec-owned `nse_bridge` and `nse_http_capability` remain outside the standalone runtime.

## 7. Ordered work packages

### Work package A — Freeze extraction identity and bootstrap the standalone repository

Intent:

Create a traceable external ownership boundary before modifying Eggsec consumption.

Required changes:

- record the exact Eggsec source commit used for extraction;
- create `eggstack/eggsec-nse`;
- transfer runtime-owned files into a root-level crate layout;
- preserve history when practical, otherwise create an explicit source-to-extraction provenance record;
- copy the MIT license and only runtime-owned/lawfully redistributable assets;
- verify no Eggsec engine adapter or upstream Nmap corpus is copied.

Acceptance evidence:

- canonical repository exists;
- initial standalone commit records its Eggsec source SHA;
- source/test inventory matches the runtime-owned boundary;
- no `eggsec_*` dependency/import is introduced.

### Work package B — Make package metadata and dependencies standalone

Intent:

Remove all hidden reliance on the Eggsec workspace while preserving the package contract.

Required changes:

- replace `version.workspace`, `edition.workspace`, `license.workspace`, `repository.workspace`, `rust-version.workspace`, and dependency `.workspace` entries with explicit standalone values;
- replace the workspace README path with a runtime-specific README;
- preserve the current effective dependency features/default-feature choices rather than opportunistically upgrading;
- preserve `rust-version = "1.89"`;
- generate/update the standalone lockfile for CI as appropriate;
- confirm `cargo metadata` and package file inclusion contain no path outside the standalone repository.

Acceptance evidence:

- `cargo metadata --no-deps` succeeds in a clean standalone checkout;
- `cargo tree` contains no Eggsec workspace crate;
- no path dependency points back to Eggsec;
- package metadata points to `eggstack/eggsec-nse`.

### Work package C — Transfer runtime documentation, provenance, tests, and guards

Intent:

Make the standalone repository independently understandable and auditable.

Required changes:

- migrate/adapt runtime-owned compatibility documentation;
- document clean-room corpus provenance and the prohibition on silently importing upstream Nmap corpus material;
- migrate runtime tests/fixtures;
- add repository-local checks for forbidden `eggsec_*` dependencies/imports;
- add documentation for the canonical request/execution/report pipeline and execution profiles;
- add verification instructions that do not depend on Eggsec's Makefile.

Acceptance evidence:

- a new contributor can build/test the runtime from the standalone repository alone;
- fixture provenance is present and machine/human reviewable;
- no documentation points to removed Eggsec-local runtime paths as authoritative.

### Work package D — Establish independent CI and SSH runtime qualification

Intent:

Prove the external repository is independently buildable and that the carried SSH qualification is resolved.

Required changes:

- add formatting, compile, clippy, test, feature, and provenance/architecture checks;
- test the default/no-feature crate and the `nse` runtime path;
- compile/test `sandbox` and `nse-ssh2` feature combinations;
- include an MSRV 1.89 build/check gate;
- add at least compile qualification on supported non-Linux hosts where existing runtime behavior claims portability;
- on an SSH-capable Linux runner, exercise the SSH-backed runtime path against a local/disposable SSH service or fixture;
- bound any spawned service/test with repository-standard timeouts and guaranteed cleanup.

Acceptance evidence:

- standalone CI is green at one exact commit;
- closure records the exact commit and CI URL/run;
- SSH evidence proves the backed path executed, not merely compiled;
- if the environment cannot execute SSH qualification, Milestone 003 must not make an SSH-parity claim and must remain open/conditionally closed rather than silently converting the requirement to compile-only.

### Work package E — Qualify packageability without publishing

Intent:

Catch repository-boundary packaging defects before Eggsec switches over.

Required changes:

- inspect `cargo package --list`;
- run a non-publishing package build/verification appropriate for a library crate;
- ensure README/license/provenance-required source files are present;
- confirm no Eggsec-relative files are required by build/tests/package metadata.

Acceptance evidence:

- package preflight succeeds without accessing the Eggsec checkout;
- no publication is performed;
- package contents contain no unexpected upstream Nmap corpus.

### Work package F — Switch Eggsec to the exact external revision

Intent:

Make the external repository the real runtime source while preserving Eggsec behavior.

Hard gate:

Work packages A-E must be green first.

Required changes:

- update Eggsec root workspace membership to remove `crates/eggsec-nse`;
- replace the engine's path dependency with `git = "https://github.com/eggstack/eggsec-nse"` plus exact `rev = "<standalone commit>"`;
- update `Cargo.lock`;
- remove the local runtime source tree;
- retain the engine facade and both Eggsec-owned seams;
- audit all repository references to `crates/eggsec-nse` and classify them as historical planning evidence, documentation that must change, or guards/tests that must be retargeted;
- do not rewrite grandfathered historical plan records.

Acceptance evidence:

- `cargo metadata` and `cargo tree` show the external exact-revision runtime;
- no production workspace crate except `eggsec` declares `eggsec-nse`;
- no current build/test/docs path requires the deleted local crate.

### Work package G — Retarget Eggsec architecture guards and integration evidence

Intent:

Make external ownership durable and catch accidental re-vendoring or unpinned consumption.

Required changes:

- replace/retarget guards 144/145/146 to the post-extraction model;
- add an exact-revision guard;
- retain direct-consumer guard;
- retain/import boundary tests around `eggsec::nse`;
- update Eggsec architecture docs, NSE docs, skill instructions, and workspace layout;
- ensure Eggsec integration tests use exported runtime APIs/fixtures rather than relative paths into the deleted repository tree.

Acceptance evidence:

- intentionally changing the dependency to a branch-only Git source fails a static guard;
- adding a TUI/Python direct dependency fails a static guard;
- restoring a local runtime workspace member fails a static guard;
- Eggsec integration tests remain green.

### Work package H — Cross-repository qualification and extraction closure preparation

Intent:

Prove both repositories agree on one runtime identity.

Required changes:

Standalone at exact revision:

- rerun its full CI/verification set;
- record commit, feature matrix, corpus counts, SSH runtime evidence, package preflight, and provenance checks.

Eggsec consuming that exact revision:

- run focused NSE engine tests;
- run TUI/Python NSE checks;
- run architecture guards;
- run `make check`;
- run `make check-features-individual` because dependency/feature wiring changed;
- run Python verification where required by current repository policy;
- confirm `Cargo.lock` resolves the same exact revision.

Acceptance evidence:

- both sides are green against one immutable runtime commit;
- report/API behavior matches the Milestone 002 baseline except for repository/source metadata;
- closure has enough evidence to recommend whether Milestone 004 publication can be planned.

## 8. Failure, cancellation, restart, and contention semantics

The runtime remains request-scoped and non-durable. No restart protocol is introduced.

Repository migration failure must be fail-safe:

- do not remove the in-tree runtime before the external commit is independently green;
- if standalone qualification fails, fix/qualify the standalone repository first and do not switch Eggsec;
- if Eggsec integration fails against an otherwise-green standalone commit, keep the external commit immutable, fix the appropriate side, and pin a new exact runtime commit only after rerunning standalone verification;
- never "fix" a failure by switching the dependency to a moving branch.

Runtime cancellation, limits, sandbox, resolver behavior, and concurrent-run isolation remain Milestone 002 semantics and must be regression-tested rather than redesigned.

CI services used for SSH qualification must be locally scoped, timeout-bounded, deterministic, and cleaned up even on failure.

## 9. Compatibility and migration

This is a source-ownership migration, not a user-facing NSE behavior migration.

Compatibility requirements:

- package name remains `eggsec-nse`;
- Eggsec-facing namespace remains `eggsec::nse`;
- Eggsec feature names remain unchanged;
- TUI/Python public behavior remains unchanged;
- runtime API/report serialization remains compatible;
- built-in identifiers and clean-room corpus expectations remain intact.

Cargo-source migration:

Before:
```text
eggsec -> path ../eggsec-nse
```

After Milestone 003:
```text
eggsec -> git https://github.com/eggstack/eggsec-nse @ exact rev
```

Milestone 004 later replaces the temporary Git source with the versioned released dependency after publication qualification.

Historical plan/closure documents that mention `crates/eggsec-nse` are evidence and must not be rewritten. Current architecture/docs/skills must describe the new ownership.

## 10. Required tests

### Focused unit tests

Standalone:

- all runtime unit tests under default/no-feature conditions where applicable;
- profile/resolver/limits/capability/report/executor/library tests with `nse`;
- metadata/provenance/static-boundary checks.

Eggsec:

- engine facade/re-export tests;
- `nse_bridge` adapter tests;
- `nse_http_capability` existing tests.

### Integration tests

Standalone:

- clean-room compatibility corpus;
- runtime corpus/smoke/local-protocol suites;
- canonical run/report-contract tests;
- feature-specific SSH runtime integration on a local endpoint.

Eggsec:

- NSE dispatch/integration/real-script suites;
- TUI NSE tests;
- Python NSE tests;
- report-envelope bridge integration.

### Restart and recovery tests

No durable restart behavior applies.

For CI fixture services, test/ensure cleanup and timeout behavior so failed jobs do not leak background processes.

### Contention and cancellation tests

Re-run the canonical concurrent-run isolation and cancellation/limits coverage in the standalone repository.

### Security and negative tests

- resolver path/symlink/source-policy denials;
- capability/profile denials;
- sandbox/limits negatives;
- no `eggsec_*` imports/dependencies in standalone;
- no untracked upstream Nmap corpus;
- Eggsec external dependency must be exact-revision pinned;
- only Eggsec directly consumes the runtime;
- Eggsec authorization remains upstream of execution.

### Migration and compatibility tests

- `NseRunReport` serialization/round-trip contract;
- current runtime public API build examples;
- Eggsec `nse`, `nse-ssh2`, `nse-sandbox` feature combinations;
- TUI/Python engine-facade consumption;
- standalone package preflight from a clean checkout;
- Eggsec build from a clean checkout with no sibling runtime repository present.

## 11. Required verification commands

The implementation agent must adapt exact commands to the final standalone Makefile/scripts if introduced, but closure must record the commands actually run.

Standalone repository minimum:

```bash
cargo fmt --all --check
cargo metadata --no-deps
cargo tree
cargo check --no-default-features
cargo check --features nse
cargo test --features nse
cargo check --features nse-ssh2
cargo check --features nse,sandbox
cargo clippy --all-targets --features nse -- -D warnings
cargo package --list
cargo package
```

Also run the repository-local architecture/provenance guard suite and the SSH runtime qualification command/test added in Work Package D. Run an MSRV 1.89 check in CI.

Eggsec after the external switch:

```bash
cargo metadata --no-deps
cargo tree -p eggsec --features nse,cli -i eggsec-nse
cargo test -p eggsec --features nse,cli --lib
cargo test -p eggsec --features nse,cli --test nse_bridge_tests --test nse_tests --test nse_integration_tests --test nse_real_scripts
cargo test -p eggsec-tui --features nse
cargo test -p eggsec-python --features nse
cargo check -p eggsec --features nse-ssh2,nse-sandbox,cli
make test-architecture-guards
make check
make check-features-individual
make check-python
```

If a monolithic Eggsec test command remains too large for the available execution window, the closure may use the repository's CI plus the same focused partitioning accepted in Milestones 001-002, but it must record the partition and results rather than claiming an unrun command.

## 12. Documentation updates

Standalone:

- `README.md`;
- `LICENSE`;
- provenance/license/corpus documentation;
- compatibility/runtime architecture documentation;
- contributor verification instructions;
- CI status and feature documentation.

Eggsec:

- `architecture/overview.md`;
- `architecture/nse_integration.md`;
- `docs/NSE_COMPATIBILITY.md` where repository ownership/path guidance changes;
- `docs/python/NSE_RUNTIME_ARCHITECTURE.md` where source/repository setup is described;
- `.opencode/skills/eggsec-nse/SKILL.md`;
- workspace layout references in current docs/README;
- architecture-guard documentation if checks 144-146 are renamed/redefined;
- subsystem roadmap/registry after closure.

Do not edit canonical long-term docs unless the extraction reveals an actual contradiction, and do not rewrite grandfathered historical plans.

## 13. Acceptance criteria

1. `eggstack/eggsec-nse` exists and contains the runtime as an independently buildable root crate.
2. The standalone source is traceable to an exact Eggsec extraction commit.
3. Standalone metadata is explicit: package name, version, edition, MIT license, MSRV 1.89, repository/homepage/documentation, and dependency versions/features no longer rely on the Eggsec workspace.
4. The standalone repository contains only runtime-owned/lawfully distributable source, tests, clean-room fixtures, docs, and provenance assets; no upstream Nmap corpus is silently bundled.
5. The standalone crate has zero `eggsec-*` dependencies/imports and guards prevent regression.
6. The canonical execution/report API and runtime behavior remain compatible with the Milestone 002 baseline.
7. Independent standalone CI passes at one exact commit, including MSRV and relevant feature checks.
8. `nse-ssh2` has actual runtime qualification on an SSH-capable local/disposable test environment before closure claims SSH parity.
9. Non-publishing package preflight succeeds from the standalone repository.
10. `nse_bridge` and `nse_http_capability` remain explicitly Eggsec-owned and are absent from the standalone repository; HTTP backend cutover remains deferred.
11. Eggsec removes the local runtime workspace member/source and consumes `https://github.com/eggstack/eggsec-nse` by exact Git `rev`.
12. Eggsec's lockfile resolves the exact standalone commit used for qualification.
13. Only `eggsec` directly declares the runtime dependency; TUI/Python continue through `eggsec::nse`.
14. Post-extraction architecture guards reject local re-vendoring, duplicate direct consumers, and branch-only/unpinned runtime dependencies.
15. Eggsec NSE, TUI, Python, feature-matrix, architecture-guard, and `make check` verification pass while no sibling/local runtime checkout is required.
16. Report/profile/resolver/cancellation/limits/sandbox behavior and serialized `NseRunReport` compatibility are preserved or any intentional difference is explicitly documented.
17. Closure records both repository commit identities and recommends GO/NO-GO for Milestone 004 versioned release/adoption.

## 14. Stop conditions

The agent must stop and report rather than improvise when:

- `eggstack/eggsec-nse` cannot be created or the canonical repository name is unavailable; do not silently substitute another owner/name;
- the extraction requires copying code/assets with unclear or incompatible licensing/provenance;
- standalone compilation reveals a hidden dependency on Eggsec semantics rather than workspace metadata;
- an Eggsec adapter must move into the standalone runtime to make the split work;
- changing authorization/scope semantics appears necessary;
- the external runtime requires a public API/report schema break not already approved;
- SSH runtime qualification cannot be made deterministic without introducing unrelated production behavior;
- making Eggsec consume the external runtime requires TUI/Python direct dependencies;
- provider inversion or the HTTP backend cutover becomes necessary;
- package publication is required to proceed;
- standalone and Eggsec qualification cannot be tied to one exact external commit.

## 15. Closure evidence required

The closure record must contain:

- Milestones 001/002 closure references and readiness finding;
- Eggsec extraction source commit SHA;
- standalone repository URL and initial/qualified commit SHA(s);
- source/history-transfer method and source-to-external commit mapping;
- standalone file/asset transfer inventory;
- explicit package metadata/dependency before/after evidence;
- proof of zero `eggsec-*` standalone dependencies/imports;
- clean-room fixture/provenance evidence;
- standalone CI run(s), MSRV result, feature matrix, corpus counts, clippy/fmt results, and package preflight;
- concrete `nse-ssh2` runtime-execution evidence and environment description;
- explicit confirmation that `nse_bridge` and `nse_http_capability` stayed Eggsec-owned;
- Eggsec workspace/dependency/lockfile diff showing exact Git revision consumption;
- updated guard evidence for exact pinning/single consumer/no local re-vendoring;
- Eggsec NSE/TUI/Python/feature verification results;
- `make check`, architecture guards, and Python verification results;
- compatibility/security invariant review;
- unresolved findings classified by severity;
- recommendation for Milestone 004.

## 16. Handoff notes

This milestone changes repository ownership, not NSE semantics.

Create and qualify the external repository **before** deleting the in-tree source. The external commit is the atomic unit of qualification; every fix after qualification begins creates a new commit that must be requalified and repinned.

Use `eggstack/eggsec-nse` as the canonical target. Do not publish to crates.io in this milestone.

Keep the standalone repository focused on reusable NSE runtime semantics. Eggsec-specific scope enforcement, report-envelope conversion, scoped transport authority, and frontend adapters stay in Eggsec.

Preserve the current broad runtime API during extraction. API pruning, provider inversion, HTTP transport redesign, and release-version decisions are later work.

The prior local verification encountered very large Eggsec `target/` growth. Standalone qualification should use clean build directories/CI caches conservatively, and the Eggsec cross-repo pass should avoid allowing stale build artifacts to masquerade as proof that the external source is self-contained.
