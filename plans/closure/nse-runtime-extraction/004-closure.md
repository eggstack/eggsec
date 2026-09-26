# NSE Runtime Extraction Milestone 004 — Closure Status

Status: closed

Source implementation plan:

- `plans/implementation/nse-runtime-extraction/004-versioned-release-and-eggsec-adoption.md`

Source subsystem roadmap:

- `plans/subsystems/nse-runtime-extraction-roadmap.md#milestone-004--versioned-release-and-eggsec-adoption`

Repository baseline reviewed: `4ae6ac11` (Eggsec main at implementation start;
plan authored against `86dd20c4`; intervening commits are planning-only)

Implementation commits or pull requests:

- Standalone `9982c7fc060bb8d9cfe7b5549ba3f8a83336c00c` — 0.1.0 release
  candidate (`CHANGELOG.md`, `docs/RELEASING.md`); exact published source.
- Standalone tag `v0.1.0` (annotated `ff617caccf3e8ef6ba29e8f75d3e9ad089274628`)
  pointing at `9982c7f`; GitHub Release `v0.1.0` published.
- Standalone `854f153f56d1abc929d9abd255f0606759342f81` — post-release
  main-line updates only (README crates.io install form, tag-gated release
  workflow, Trusted Publishing status note). Not part of the published source.
- Eggsec adoption commit (this closure's companion commit) — registry
  dependency, `deny.toml`, Check 144 retarget, docs/skill updates.

## 1. Executive finding

Milestone 004 is complete. `eggsec-nse 0.1.0` is published on crates.io from
the qualified standalone release candidate, bound to its source tag and
GitHub Release, and Eggsec consumes the published registry artifact instead
of the temporary exact Git revision. The `allow-git` exception is removed,
Check 144 enforces registry-source consumption, and full Eggsec verification
is green against the registry artifact with byte-identical dependency
checksums to an independent scratch consumer.

Standalone release CI run
[36259799068](https://github.com/eggstack/eggsec-nse/actions/runs/36259799068)
passed on the release-candidate SHA.

## 2. Requirement-to-evidence matrix

| Requirement | Evidence | Result | Notes |
|---|---|---|---|
| 003 planning state reconciled before handoff | Roadmap §12 table (003 closed), registry (001–003 closed), 003 plan `Status: closed`, 003 closure record | pass | 004 advanced ready → active at implementation start. |
| Registry preflight without assumptions | crates.io API 404 + sparse-index `NoSuchKey` for `eggsec-nse`; `gh repo view` (public, no tags/releases, main @ `3f57e6c3`) | pass | Name unclaimed, `0.1.0` absent → continue per plan decision rules. |
| Publish credential readiness, no secret exposure | `~/.cargo/credentials.toml` holds a registry token (presence only); validity proven by `cargo publish` itself | pass | No token, password, or credential printed or committed anywhere. |
| Clean release-candidate commit | Standalone `9982c7f`; `git status` clean; diff vs qualified base `3f57e6c3` is docs-only (`CHANGELOG.md`, `docs/RELEASING.md`) | pass | No runtime source/API change in the candidate. |
| Full RC qualification (CI + MSRV + corpus + SSH + package) | Standalone CI run 36259799068 success; local `cargo test --features nse` 558 passed/1 ignored; loopback SSH test 1 passed; MSRV 1.89 checks pass | pass | Includes `cargo publish --dry-run` without bypass flags; 299 package files; secret scan clean. |
| Public registry artifact resolvable/verifiable | crates.io API: `max_version 0.1.0`, MIT, not yanked, size 449385; repo/homepage/docs metadata point at the standalone repository | pass | — |
| Scratch consumer builds published artifact | Clean `nse-consumer` crate: lockfile `registry+.../crates.io-index`, checksum `fc77859f…0dc982`; `execute_nse_run` executes, report `Compatible` | pass | No Git/path dependency required. |
| Published archive matches RC | `cargo package --list` vs `.crate` contents identical (299 files); `.cargo_vcs_info.json` pins `9982c7f` | pass | — |
| Tag + GitHub Release identity | `v0.1.0^{commit} = 9982c7f`; release notes describe curated compatibility, no full-Nmap-parity claim | pass | Tag never moved; post-release docs live on main (`854f153`), not the tag. |
| Future release controls documented | `.github/workflows/release.yml` (tag-only, `release` env, `id-token: write`, tag==version gate); `docs/RELEASING.md` Trusted Publishing status | pass | Registry-side Trusted Publisher config pending as low/operational residual (§10). |
| Eggsec consumes crates.io release | `crates/eggsec/Cargo.toml`: `eggsec-nse = { version = "0.1.0", optional = true }`; `cargo tree -i eggsec-nse` → `eggsec-nse v0.1.0` under `eggsec` only | pass | — |
| Lockfile is registry-backed | `Cargo.lock`: `source = "registry+https://github.com/rust-lang/crates.io-index"`, checksum `fc77859f…0dc982` — identical to scratch consumer | pass | No Git runtime source remains. |
| Git exception removed, deny stays fail-closed | `deny.toml`: `allow-git` entry removed; `unknown-git = "deny"`, `unknown-registry = "deny"` retained; `make check-deps` sources ok | pass | `required-git-spec = "rev"` retained as fail-closed default. |
| Check 144 retargeted; 145–146 green | `make test-architecture-guards` exit 0; Check 144 rejects git/path/branch/tag/rev and verifies registry version/source | pass | Negative logic reviewed against the former manifest line (would fail). |
| Single consumer, facade, adapters intact | Check 145 pass; `eggsec::nse` facade, TUI/Python `nse = ["eggsec/nse"]` forwarding, engine `nse_bridge`/`nse_http_capability` unchanged | pass | — |
| Eggsec NSE/TUI/Python/feature verification | Engine lib 1,710 passed; 4 NSE integration suites 237 passed; TUI 903 passed/12 ignored; Python 231 passed; `make check-features-individual` exit 0; `make check` exit 0; `make check-python` exit 0 | pass | Counts identical to the Milestone 003 baseline. |
| No auth/report/resolver/sandbox regression | `EnforcementContext`/dispatch untouched by adoption diff; adapter tests pass; report serialization unchanged | pass | Dependency source is the only production change. |
| Current docs name crates.io authoritative | Skill, README, `docs/ARCHITECTURE.md`, `architecture/{overview,nse_integration,feature_matrix}.md`, `docs/CI_ARCHITECTURE_GUARDS.md`, `docs/DEPENDENCY_EXCEPTIONS.md`, `docs/RELEASING.md` updated | pass | 003-era history files intentionally unchanged. |

## 3. Production implementation evidence

Standalone repository (`https://github.com/eggstack/eggsec-nse`):

- `9982c7f` adds only `CHANGELOG.md` (curated `0.1.0` record: pipeline,
  features, profiles, corpus provenance, MSRV/edition/license, no full-parity
  claim) and `docs/RELEASING.md` (bootstrap + steady-state procedure,
  tag/version/source invariants, verification commands, yank guidance).
- `cargo publish` ran from the exact clean candidate with the maintainer
  credential store; no `--allow-dirty`/`--no-verify`; no credentials in
  transcripts.
- `v0.1.0` + GitHub Release identify the published source commit and the
  crates.io artifact.
- `854f153` (main only): README install example now uses
  `eggsec-nse = { version = "0.1", features = ["nse"] }`; new
  `.github/workflows/release.yml` cannot publish from pushes or PRs; Trusted
  Publishing registry-side step documented as pending.

Eggsec repository:

- Dependency source migration exactly as planned:
  `eggsec -> git+https://github.com/eggstack/eggsec-nse?rev=<sha>` became
  `eggsec -> registry+https://github.com/rust-lang/crates.io-index :
  eggsec-nse 0.1.0`.
- No protocol, report schema, Python DTO, TUI DTO, CLI contract, feature-name,
  authorization, profile, resolver, cancellation, limit, sandbox, or
  corpus-provenance change. `nse`/`nse-ssh2`/`nse-sandbox` features and the
  `NseRunRequest -> execute_nse_run -> NseRunReport` pipeline are untouched.

## 4. Verification executed

### Commands run

```bash
# Standalone release candidate 9982c7f (clean checkout)
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
cargo clippy --all-targets --features nse -- -D warnings   # see residual R1
cargo +1.89.0 check --locked --no-default-features
cargo +1.89.0 check --locked --features nse
cargo publish --dry-run
cargo package --list
cargo package
# loopback SSH2 runtime (disposable local sshd + throwaway account, cleaned up)
NSE_SSH_TEST_PORT=22222 cargo test --locked --features nse-ssh2 --test ssh_runtime_tests -- --nocapture

# Irreversible step, only after all gates above
cargo publish

# Post-publish artifact verification
# crates.io API: crate metadata + 0.1.0 version record
# clean scratch crate: eggsec-nse = { version = "0.1.0", features = ["nse", "sandbox"] }
cargo test   # scratch consumer: resolve + build + execute against registry artifact
# .crate download: file-list diff vs cargo package --list; .cargo_vcs_info.json SHA check

# Eggsec adoption (registry artifact under test)
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

### Results

- Standalone: fmt, boundaries, metadata, tree, all feature checks pass;
  `cargo test --features nse` — 558 passed, 1 ignored; repo lint gate
  (`clippy` without `-D warnings`) exit 0; MSRV 1.89 checks pass;
  `publish --dry-run` succeeds; `cargo package` succeeds (299 files, no
  secrets/build artifacts/unlicensed corpus material).
- Standalone CI run 36259799068 (Linux, macOS, MSRV, package, loopback SSH2)
  — success at `9982c7f`.
- Local loopback SSH2 runtime test — 1 passed (disposable account/credentials
  removed afterwards).
- `cargo publish` — `Published eggsec-nse v0.1.0 at registry crates-io`.
- Registry: `eggsec-nse` / `max_version 0.1.0` / MIT / not yanked / size
  449385; homepage/repository/documentation point at the standalone repo;
  features `default, nse, nse-ssh2, sandbox, stress-testing`.
- Scratch consumer — lockfile registry source + checksum
  `fc77859fcbba77de2179aa2c55d9411c8767581655a22f81a4a8a804da0dc982`;
  execution returns a canonical `Compatible` report (1 passed).
- Published archive file list identical to RC package list; VCS SHA
  `9982c7f`.
- Eggsec: combined `nse-ssh2,nse-sandbox,cli` check — 0 errors (4 pre-existing
  warnings, unchanged); lib 1,710 passed; integration 237 passed; TUI 903
  passed/12 ignored; Python 231 passed; `check-deps` (advisories/licenses/
  bans/sources) ok; architecture guards all pass incl. retargeted 144–146;
  `check-features-individual` exit 0; `make check` exit 0; `make check-python`
  exit 0.
- docs.rs ingestion was not used as a gate (asynchronous build latency is
  explicitly non-blocking per the plan); package verification above is the
  controlling evidence.

## 5. Invariant review

- Eggsec remains the product authorization and scope authority. The adoption
  diff touches no `EnforcementContext`, `OperationMetadata`,
  `EnforcedDispatcher`, dispatch, or scope code; strict surfaces still
  dispatch via `EnforcedDispatcher::dispatch_execution()` with
  `ApprovedExecution` bundles. The released runtime is not an authorization
  layer (its README/`docs/RELEASING.md` state this explicitly).
- `eggsec-nse` has zero `eggsec-*` dependencies (standalone boundary script
  passes; published manifest confirms).
- Package name `eggsec-nse`, version `0.1.0`, features
  `nse`/`nse-ssh2`/`sandbox`/`stress-testing`; Eggsec features
  `nse`/`nse-ssh2`/`nse-sandbox` unchanged.
- `eggsec::nse` remains the facade; TUI/Python remain indirect consumers
  (Check 145).
- `nse_bridge`/`nse_http_capability` remain engine-owned (Check 145).
- `NseRunRequest -> execute_nse_run -> NseRunReport` pipeline, report
  serialization, execution-profile semantics, resolver containment, capability
  policy, limits, cancellation, sandbox behavior, and clean-room provenance
  are unchanged (identical test counts to the 003 baseline; corpus runs in
  the standalone repo).
- No crates.io token, credential, test password, or secret was committed,
  printed, or persisted in any repository file (verified by scan + review).
- No second runtime source remains: Git exception removed, Check 144 rejects
  git/path/branch/tag/rev, lockfile has no Git runtime source.
- No upstream Nmap script/nselib redistribution (boundary script + package
  review).
- No provider inversion or HTTP backend change (ADR-0001/ADR-0002 unaffected;
  no new ADR required).

## 6. Failure and recovery review

- All work before `cargo publish` was reversible (docs-only RC on a branch
  tip, pushed to `main` only to obtain CI).
- Published `0.1.0` was not overwritten; the tag was created once and never
  moved; post-publish fixes went to main (`854f153`), not the tag.
- Rollback for Eggsec remains the pre-adoption lockfile (Git rev
  `3f57e6c3`); no yank was needed and none is proposed.
- No daemon/restart/persistence behavior introduced; runs remain
  request-scoped with existing cancellation/limit semantics.
- SSH qualification used disposable loopback credentials with cleanup; no
  public targets.

## 7. Migration and compatibility review

- Source migration: `eggsec -> eggsec-nse 0.1.0` from exact Git rev became
  `eggsec -> eggsec-nse 0.1.0` from crates.io. Lockfile checksums match an
  independent consumer, proving the registry artifact — not a local build —
  is under test.
- No persistent-data migration, serialized DTO change, feature rename, or
  user-visible NSE behavior change. Test counts are identical to baseline.
- Future runtime upgrades require an explicit version/lockfile change plus
  standalone + Eggsec qualification (documented in the engine manifest
  comment and Eggsec release docs).

## 8. Security review

- First publication treated as a supply-chain boundary: clean reviewed
  candidate, ordinary authenticated publish, no `--allow-dirty`/`--no-verify`,
  no credentials in logs/files/transcripts.
- Future workflow (`release.yml`) requires version tags, an explicit
  `release` environment, `id-token: write`/`contents: read` minimums, and
  tag==version + clean-tree + full-qualification gates; pushes/PRs cannot
  trigger it. Registry-side Trusted Publisher configuration remains pending
  (residual R2); until then the workflow fails closed at authentication.
- Eggsec Cargo policy denies unknown registries and all Git sources
  (`check-deps` sources ok).
- `SensitiveString`/authn paths untouched; no new privilege boundary.

## 9. Documentation and operations

- Standalone: `CHANGELOG.md`, `docs/RELEASING.md` (incl. Trusted Publishing
  status + yank guidance), README crates.io install form, release workflow.
- Eggsec: skill (`eggsec-nse`), README, `docs/ARCHITECTURE.md`,
  `architecture/overview.md`, `architecture/nse_integration.md`,
  `architecture/feature_matrix.md`, `docs/CI_ARCHITECTURE_GUARDS.md`,
  `docs/DEPENDENCY_EXCEPTIONS.md` (Git exception retired, history retained),
  `docs/RELEASING.md` (0.1.0 registry note), Check 144 retarget.
- Milestone 003 history (plan, closure, roadmap rows) intentionally
  unchanged where it records the former Git phase.

## 10. Unresolved findings

| Severity | Finding | Impact | Required action |
|---|---|---|---|
| low | `cargo clippy --all-targets --features nse -- -D warnings` fails on pre-existing toolchain-drift lints (deprecated `openssl as_utf8`, unreachable pattern, never-read fields). | None on behavior/packaging; repo lint gate (CI form, no `-D`) passes. | Do not fix in this release milestone (would be a semantic change). Track as standalone maintenance; a future corrective pass may address it. |
| low | crates.io-side Trusted Publisher entry for `eggstack/eggsec-nse` is not configured (website-only step, no API path available). | Future releases use the documented manual-token path; the new workflow fails closed until configured. | A crate owner completes the website-side step; no code change needed. |

No correctness or security findings remain for Milestone 004.

## 11. Roadmap disposition

Milestone closed; the next dependency may proceed. Milestone 005
(provider inversion / portability hardening) may proceed to **planning**
— its soft dependency on a standalone release is now satisfied by the
published `0.1.0` artifact — while provider-interface **implementation**
remains evidence-gated per the roadmap (measured behavior or concrete
consumers required). No corrective implementation plan is required for
this milestone.

## 12. Registry updates

- Mark Milestone 004 closed and link this record from the subsystem roadmap.
- Mark the 004 implementation plan closed and record the published
  version/tag/adoption evidence.
- Update the registry to record 001–004 closed; permit Milestone 005
  planning while preserving its implementation evidence gate.
- Keep the NSE subsystem active for future milestones; no implementation
  work is presently blocked.
