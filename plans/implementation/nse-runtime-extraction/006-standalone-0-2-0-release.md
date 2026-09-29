# NSE Runtime Extraction Milestone 006A — Standalone 0.2.0 Release Preparation and Publication

Status: ready for handoff

Eggsec planning baseline: `51f9747deef7bb5c2488e7dd392b0f2c96667653`

Standalone runtime baseline: `eggstack/eggsec-nse@9fe149fbb22480a63e254e91d60083b7a29a8ff4`

Source roadmap:

- `plans/subsystems/nse-runtime-extraction-roadmap.md#milestone-006--020-release-and-safe-eggsec-adoption`

Hard dependency closure:

- `plans/closure/nse-runtime-extraction/005-post-merge-ci-fixture-corrective-closure.md` — closed; M005 is landed on standalone main with hosted run `36490773625` fully green.

Applicable ADRs:

- `plans/adrs/ADR-0003-nse-host-provider-boundary.md`

Primary class: capability

Affected repository:

- release producer: `eggstack/eggsec-nse`.

## 1. Objective

Prepare, qualify, and publish `eggsec-nse 0.2.0` from the current M005-qualified standalone runtime, with truthful release documentation and a release workflow that actually satisfies the repository's boundary-check prerequisites.

The 0.2.0 release is required because M005 introduced public provider APIs and a breaking public signature change, and corrected network-byte accounting semantics. This plan owns the irreversible crates.io publication boundary only; Eggsec dependency adoption is Milestone 006B.

## 2. Why this milestone is ready

M005 closure establishes:

- provider broker foundation, DNS/TCP/UDP, filesystem/process, HTTP, and deterministic provider surfaces are merged on standalone main;
- `register_vulns_library(lua)` changed to `register_vulns_library(lua, capability_ctx)`, requiring a 0.x minor release rather than a patch;
- send accounting is direction-correct and write limits are now live;
- hosted Linux/macOS/Windows/MSRV/SSH CI is green on `9fe149f`;
- no high-severity M005 defect remains.

Current release state:

- `Cargo.toml` is still `version = "0.1.0"`;
- `CHANGELOG.md` contains only 0.1.0;
- the only GitHub Release is `v0.1.0` from source `9982c7f`;
- README installation still targets `version = "0.1"`;
- `.github/workflows/release.yml` is tag-only and Trusted-Publishing-ready, but its boundary-check step does not currently provision ripgrep even though `check-boundaries.sh` requires it;
- `docs/RELEASING.md` records crates.io-side Trusted Publisher configuration as pending and documents manual-token publication as the recovery path.

## 3. Release content that must be documented

0.2.0 release notes/changelog must explicitly include:

### Added

- `NseHostServices` per-run provider bundle;
- clock/random/environment providers and deterministic test doubles;
- DNS/TCP/UDP provider contracts with exact-endpoint selection;
- filesystem/process providers and per-run virtual CWD;
- HTTP provider contract and native backend;
- public broker/provider DTO types required by embedders;
- Windows compile/check qualification.

### Changed

- `register_vulns_library` now requires `&NseCapabilityContext`;
- network send bytes now count as written rather than read;
- HTTP accounting splits request bytes written from response bytes read;
- provider-backed libraries now execute through capability-aware brokers.

### Security/compatibility notes

- runtime provider mechanics are not embedding-application authorization;
- the known 72-file specialized direct-I/O residual remains outside provider/capability coverage and is pinned against expansion;
- automated embedding applications must not treat 0.2.0 as complete protocol-wide scope enforcement;
- no upstream Nmap script/nselib corpus is bundled.

## 4. Invariants

- Package name remains `eggsec-nse`.
- Version becomes exactly `0.2.0`.
- Edition remains 2021, license MIT, MSRV 1.89.
- No `eggsec-*` dependency/import enters standalone.
- Existing feature names remain unchanged.
- `NseRunReport` serialization remains compatible unless release notes explicitly identify a measured intentional change.
- M005 provider/accounting behavior is not rewritten during release preparation.
- The 72-file ungated residual stays documented and guard-pinned.
- No release tag is created until the exact candidate commit is fully qualified.
- Published versions are immutable; do not use `--allow-dirty` or `--no-verify`.
- No crates.io credential is committed or printed.

## 5. Scope

### In scope

- bump `Cargo.toml` to 0.2.0;
- add a complete 0.2.0 `CHANGELOG.md` entry;
- update README dependency example to `version = "0.2"`;
- reconcile provider/release docs with 0.2.0;
- fix the release workflow's ripgrep prerequisite so the tag workflow cannot fail at `check-boundaries.sh`;
- reconcile `docs/RELEASING.md` with the actual lint gate: do not claim `cargo clippy ... -- -D warnings` passed if the repository still carries accepted warning debt;
- run full candidate qualification and `cargo publish --dry-run`;
- publish 0.2.0 through Trusted Publishing if crates.io-side configuration is present, otherwise use the documented manual-token recovery path;
- verify crates.io/index resolution from a clean scratch consumer;
- verify docs.rs ingestion/build;
- create immutable `v0.2.0` tag and GitHub Release pointing at the published source commit;
- record release identity/checksum/source evidence.

### Explicitly out of scope

- changing Eggsec's dependency;
- merging/activating the Eggsec HTTP adapter;
- production NSE scope threading;
- protocol-library capability gating;
- clearing unrelated warning debt merely to make a historical `-D warnings` documentation claim true;
- publishing any version other than 0.2.0.

## 6. Required release-workflow corrections

Before tagging:

1. Provision ripgrep in `.github/workflows/release.yml` before `./scripts/check-boundaries.sh`, using the same explicit/fail-closed posture as normal CI.
2. Keep tag-only trigger semantics.
3. Keep minimum permissions: `contents: read`, job-level `id-token: write` only for Trusted Publishing.
4. Preserve the tag==manifest-version and clean-tree checks.
5. Do not use `pull_request_target`, `workflow_run`, ordinary push-to-main publication, or `continue-on-error`.
6. If Trusted Publishing remains unconfigured on crates.io, the workflow may remain fail-closed; use the documented manual-token recovery path rather than weakening authentication.

Release-critical third-party action policy should follow repository convention. Do not introduce a one-off pinning policy here unless the repository-wide policy requires it.

## 7. Ordered work packages

### A — Freeze 0.2.0 release scope

Record the M005 closure/release recommendation, current main SHA, public API break, accounting behavior change, residual-risk disclosure, and intended version.

### B — Prepare the release candidate

Update version/changelog/README/releasing docs and release workflow. No provider feature changes.

### C — Candidate qualification

Require hosted CI success on the exact candidate SHA plus local/package checks:

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

The hosted candidate run must also include Windows and loopback SSH qualification.

### D — Publication

Verify 0.2.0 is absent, publish from the exact clean candidate, and record the registry response without exposing credentials.

### E — Registry artifact verification

Use a clean scratch crate resolving `eggsec-nse = "0.2.0"` from crates.io, with required feature combinations. Verify no Git/path source.

Compare the published archive's VCS/source identity with the candidate where practical.

### F — Tag and GitHub Release

Create `v0.2.0` pointing exactly to the published source commit. Release notes must disclose the breaking signature, provider surface, accounting correction, and ungated residual.

### G — Post-publication verification

Confirm crates.io and docs.rs resolve 0.2.0. Do not change Eggsec from this plan.

## 8. Failure and recovery

- If 0.2.0 already exists unexpectedly, stop and reconcile; never overwrite.
- If candidate CI is red, do not tag/publish.
- If release workflow authentication is unavailable, use the documented manual-token recovery path only after all candidate gates pass.
- If publication succeeds but the artifact is defective, do not move the tag; evaluate yank + a separately planned corrective version.
- If docs.rs is delayed, distinguish asynchronous delay from an actual build failure.
- If a release-prep fix changes provider/runtime semantics, re-run the complete M005-relevant qualification and record the delta.

## 9. Required tests

- complete standalone suite;
- provider broker/network/fs/process/http/composition tests;
- boundary/tooling tests;
- compatibility corpus/local protocol fixtures;
- MSRV 1.89;
- Windows compile/check;
- loopback SSH runtime;
- package/dry-run;
- scratch consumer from crates.io after publication.

## 10. Acceptance criteria

1. Cargo version is exactly 0.2.0.
2. Changelog/release notes truthfully describe the public break and behavior changes.
3. README installation targets 0.2.
4. Release workflow provisions ripgrep before the boundary script.
5. Release docs no longer overclaim a warnings-as-errors gate that was not actually passed.
6. Exact release-candidate SHA has fully green hosted CI.
7. `cargo publish --dry-run` and package verification pass.
8. `eggsec-nse 0.2.0` is published and publicly resolvable.
9. Clean scratch consumer builds the registry artifact.
10. Published source identity matches the qualified candidate.
11. `v0.2.0` and GitHub Release point to that same source commit.
12. docs.rs builds or any delay/failure is explicitly classified.
13. No Eggsec consumer change occurs in this plan.
14. Closure unblocks Milestone 006B.

## 11. Stop conditions

Stop if:

- release requires changing M005 provider semantics;
- package contents contain unreviewed/unlicensed corpus material;
- candidate CI is not fully green;
- tag/version/source identity diverges;
- publishing credentials/Trusted Publisher state cannot be safely satisfied;
- work expands into Eggsec adoption or protocol gating.

## 12. Closure evidence

Record:

- pre-release main SHA;
- release-candidate SHA;
- version/changelog diff;
- candidate CI run/job outcomes;
- package/dry-run results;
- publication method (Trusted Publishing or manual recovery, without secrets);
- crates.io version evidence;
- published archive/source identity;
- `v0.2.0` tag + GitHub Release identity;
- docs.rs outcome;
- scratch-consumer results;
- explicit GO/NO-GO for 006B.

## 13. Handoff notes

Treat `cargo publish` as irreversible.

The release workflow currently has a known release-only tooling gap: it calls the ripgrep-dependent boundary script without provisioning ripgrep. Fix that before creating the tag.

Do not activate the staged Eggsec HTTP adapter from this plan.
