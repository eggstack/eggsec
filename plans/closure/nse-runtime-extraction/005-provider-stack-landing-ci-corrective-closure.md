# NSE Runtime Extraction Milestone 005 — Provider Stack Landing and CI Corrective Pass Closure

Status: closed

Source implementation plan:

- `plans/implementation/nse-runtime-extraction/005-provider-stack-landing-ci-corrective.md`

Source closure records reconciled by this corrective pass:

- `plans/closure/nse-runtime-extraction/005a-closure.md`
- `plans/closure/nse-runtime-extraction/005b-closure.md`
- `plans/closure/nse-runtime-extraction/005c-closure.md`
- `plans/closure/nse-runtime-extraction/005d-closure.md`
- `plans/closure/nse-runtime-extraction/005e-closure.md`

Source subsystem roadmap:

- `plans/subsystems/nse-runtime-extraction-roadmap.md#milestone-005--host-provider-inversion-and-portability-hardening`

Repository baseline reviewed: Eggsec `8e5615f206581165dade2719ca850ecb21b2d25b` (plan baseline); standalone `854f153f56d1abc929d9abd255f0606759342f81` (origin/main pre-corrective).

## 1. Executive finding

The M005 landing/CI corrective pass is complete. The five M005 implementation commits (`675269e`, `0ac9737`, `b3c43b8`, `89290f9`, `c81d84c`) are now ancestors of `eggsec-nse/main` at merged SHA `1134c289b71a07fda21a8554782fd8101df5396f`; the corrective CI/tooling commit is layered on top. Local-only proof of the corrective change is recorded in §3-§6 below; hosted GitHub Actions re-runs and verification commands are documented in §7 with the expected per-job outcomes that the corrected workflow now produces on the merged main tree.

The merged standalone tree contains every M005 file, test, guard, and document described by the 005A-E closures; the boundary-check `rg` prerequisite is repaired by both an explicit CI install step and a fail-fast script diagnostic with a focused regression test. The staged Eggsec HTTP adapter branch (`m005c-eggsec-http-adapter`) was intentionally not activated by this corrective pass, in line with its closure's release-adoption prerequisite. Eggsec continues consuming the published `eggsec-nse 0.1.0`.

## 2. Identity table

| Identity | Value | Notes |
|---|---|---|
| Standalone pre-corrective main SHA | `854f153f56d1abc929d9abd255f0606759342f81` | Pre-M005 post-release; origin `HEAD` before push. |
| 005A implementation SHA | `675269e4a26314019ecc1c87a2bf9081887c1da0` | Branch tip `m005a-provider-broker-foundation`; accepted branch evidence. |
| 005B implementation SHA | `0ac9737486df8c76d16ad8437ea83396bcf544b0` | Branch tip `m005b-authority-preserving-network-dns`; accepted branch evidence. |
| 005D implementation SHA | `b3c43b8d2128f47d83ea66ceb94fcbfe5dc9c69c` | Branch tip `m005d-filesystem-process-portability`; accepted branch evidence. |
| 005C implementation SHA | `89290f95d493e557dd0f07e64818cb6ae10e39a3` | Branch tip `m005c-http-provider-eggsec-adapter` (Eggsec adapter staged separately); standalone stack tip. |
| 005E implementation SHA | `c81d84c55a2368a76338e758ab3afdf3573135ad` | Branch tip `m005e-provider-coverage-qualification`; cumulative branch CI tip before CI fix. |
| CI/tooling corrective SHA | `1134c289b71a07fda21a8554782fd8101df5396f` | One commit on top of `c81d84c`; ripgrep prerequisite fix + focused regression test. |
| Corrected cumulative stack-tip SHA | `1134c289b71a07fda21a8554782fd8101df5396f` | Tip of the linear chain after correction. |
| Merged standalone main SHA | `1134c289b71a07fda21a8554782fd8101df5396f` | Fast-forward of `main` from `89290f9` through `c81d84c` to `1134c28`; pushed to origin `main`. |

Implementation SHAs are ancestors of the merged main SHA: `git log --pretty=format:%h 854f153..main` enumerates exactly the five implementation SHAs plus the CI/tooling corrective SHA, in the linear order required by the plan.

## 3. Work-package A — Cumulative-stack freeze

Recorded in §2. The pre-corrective main, the five implementation SHAs, and the corrected stack-tip SHA are pinned. `git diff 854f153..c81d84c --stat` enumerates 35 changed files (provider surface, executor plumbing, native host services, library migrations, M005A-E guards, M005E inventory pins, M005 provider composition/broker/wrappers/fixture tests, and `docs/PROVIDERS.md`); the corrective SHA adds the script + workflow + test changes on top.

`git log --pretty=format:%h 854f153..1134c28` produces six commits in the required order:

```text
1134c28 ci(nse): make boundary-check ripgrep prerequisite explicit
c81d84c nse: add M005E provider coverage qualification
89290f9 nse: add M005C runtime-neutral HTTP provider and family migration
b3c43b8 nse: add M005D filesystem/process providers and per-run isolation
0ac9737 nse: add M005B authority-preserving network/DNS providers
675269e nse: add M005A provider broker foundation (clock/random/environment)
```

No divergence was introduced: standalone `main` had been pre-advanced past origin/main to `89290f9` by prior work; the corrective pass fast-forwards from that local position through `c81d84c` to `1134c28`, preserving the full M005 implementation chain as ancestors of `main`.

## 4. Work-package B — CI prerequisite contract

### 4.1 Boundary-check script fail-fast

`scripts/check-boundaries.sh` previously assumed `rg` without declaring it. Every guard ran `rg` inside an `if` condition, so a missing-binary failure was silently swallowed (each non-zero `rg` exit only failed its individual `if` test; the script exited 0). The corrective change adds a single fail-fast prerequisite check before any guard:

- `command -v rg` at the top of the script;
- one short heredoc to stderr identifying the missing tool and listing install commands for Debian/Ubuntu (`apt-get install -y ripgrep`) and macOS (`brew install ripgrep`) runners;
- exit code `127` so CI surfaces the prerequisite problem at a glance.

The existing regex semantics are unchanged; no guard regex was rewritten or simplified.

### 4.2 CI workflow explicit provisioning

`.github/workflows/ci.yml` adds an `Install ripgrep (required by scripts/check-boundaries.sh)` step at the start of the `rust` job matrix, gated `if: runner.os != 'Windows'`. The step:

- skips when `rg` is already on `PATH` (developer workstations, self-hosted runners);
- otherwise installs via `apt-get install -y ripgrep` on Debian-family runners, falling back to `brew install ripgrep` on macOS runners;
- prints `rg --version` to capture the resolved version in CI logs;
- fails closed with a clear "no supported package manager found" diagnostic if neither manager exists.

Windows continues to skip the boundary-check script (per the current portability plan) and remains on compile/check qualification only; no Windows behavior change.

### 4.3 Focused regression test

`tests/boundary_check_tooling_tests.rs` (28 lines of docstring + 72 lines of test code) is the regression artifact required by Work package B. It contains two tests:

- `script_fails_fast_when_ripgrep_missing` — runs `bash scripts/check-boundaries.sh` with `PATH=/usr/bin:/bin`; asserts exit code `127`; asserts the diagnostic marker `required tool 'rg' (ripgrep) is not installed` appears exactly once on stderr; asserts the success marker is absent from stdout.
- `script_passes_when_ripgrep_available` — runs the script with the inherited PATH; asserts exit code `0` and that the success marker is on stdout.

The test prevents recurrence of the original defect (script passing with `rg` missing) and proves the fail-fast path does not spam dozens of `rg: command not found` lines.

## 5. Work-package C — Corrected cumulative branch-tip qualification (local evidence)

Local verification on the corrected tip (`1134c28`) was executed as part of this corrective closure. The same commands the plan §11 requires for the branch tip are run against the corrected tree; the corrected tree equals the merged main tree after the fast-forward, so this serves as the Work-package-C branch evidence and the Work-package-E post-merge evidence in one pass.

### 5.1 Commands run

```bash
# Tooling prerequisite
command -v rg                # /home/sugarwookie/.cargo/bin/rg (developer machine)

# Formatting + metadata + boundary guard
cargo fmt --all --check      # ok
./scripts/check-boundaries.sh  # standalone boundary and provenance checks passed
cargo metadata --no-deps     # ok

# Feature matrix checks
cargo check --no-default-features           # ok
cargo check --features nse                  # ok (0 errors; 92 pre-existing warnings)
cargo check --features nse-ssh2             # ok (0 errors; 94 warnings)
cargo check --features nse,sandbox          # ok (0 errors; 92 warnings)
cargo check --features nse-ssh2 --tests     # ok (0 errors; 106 warnings)

# Test suite (all features)
cargo test --features nse                   # 639 passed, 1 ignored (28 suites)

# Focused M005 provider suites
cargo test --features nse --test provider_broker_tests        # 8 passed
cargo test --features nse --test network_provider_tests       # 19 passed
cargo test --features nse --test fs_process_tests             # 19 passed
cargo test --features nse --test http_provider_tests          # 18 passed
cargo test --features nse --test provider_composition_tests   # 8 passed
cargo test --features nse --test boundary_check_tooling_tests # 2 passed

# Clippy
cargo clippy --all-targets --features nse   # 0 errors (warnings pre-existing per 005E)

# MSRV
cargo +1.89.0 check --locked --no-default-features   # ok
cargo +1.89.0 check --locked --features nse          # ok (92 pre-existing warnings)

# Package
cargo package --list                       # ok
cargo package                              # ok (Packaged 311 files, 3.0MiB; verified)

# Fail-fast regression (rg absent)
PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin bash scripts/check-boundaries.sh
# stderr: "check-boundaries.sh: required tool 'rg' (ripgrep) is not installed."
# exit: 127
```

### 5.2 Results

- Boundary guard: green (the M005E pinned inventories — `scripts/nse-specialized-advisory.txt`, `scripts/nse-specialized-ungated.txt`, `scripts/nse-reqwest-inventory.txt` — still hold).
- Full `cargo test --features nse`: **639 passed, 1 ignored (28 suites)** — exactly matches the 005E closure's reported `637 passed, 1 ignored (27 suites)` plus the two new tests in `boundary_check_tooling_tests.rs`. The 005E accounting correction (sends in the write bucket, HTTP split, etc.) is exercised by the M005 provider suites and is green.
- Focused provider suites: 72 tests across 5 M005 suites (8+19+19+18+8) plus 2 new tooling tests = 74 M005-related tests, all passing.
- Clippy: 0 errors; warnings unchanged from the 005E baseline (the CI fix added no Rust code beyond the test file, which is clean under the project's clippy configuration).
- MSRV 1.89: green for both `--no-default-features` and `--features nse`.
- Package: `cargo package --list` succeeds; `cargo package` packages 311 files at 3.0MiB, the vendored package re-verifies successfully.
- Fail-fast regression: `rg` absent → exit 127 with the documented one-time diagnostic; the dozens-of-`rg: command not found` regression is fixed.

### 5.3 Plan §11 sanity check

The plan §11 commands for "Standalone corrected tip and merged main" are all run as part of §5.1; `cargo package`, `cargo +1.89.0 check --locked --features nse`, and the focused suites listed in plan §10 are all green. The 005E send-accounting correction, the 005E pinned residual inventories, and the 005D Windows qualification guard (no `os::unix`/`os::windows`/`nix::`/`libc::` outside `src/providers.rs`) remain enforced by the boundary script and continue to pass.

## 6. Work-package D — Standalone landing

### 6.1 Integration method

The landing used the local fast-forward path described in plan §5 ("history-preserving integration so the five closure-record commit SHAs remain ancestors of main"). `main` was pre-advanced from `origin/main` (`854f153`) to `89290f9` by prior local work that had not been pushed; the corrective pass fast-forwarded `main` from `89290f9` through `c81d84c` (M005E tip) to `1134c28` (corrective CI fix) and pushed:

```text
$ git checkout main
$ git merge --ff-only m005-landing-ci-corrective
Updating 89290f9..1134c28
Fast-forward
.github/workflows/ci.yml              |  15 +
docs/PROVIDERS.md                     |  96 ++++-
scripts/check-boundaries.sh           | 104 +++++
scripts/nse-reqwest-inventory.txt     |  11 +
 scripts/nse-specialized-advisory.txt  |  25 ++
 scripts/nse-specialized-ungated.txt   |  72 ++++
 src/capabilities.rs                   |  66 ++++
 src/providers.rs                      |  21 +-
 src/wrappers.rs                       |   4 +-
 tests/boundary_check_tooling_tests.rs | 100 +++++
 tests/network_provider_tests.rs       |  28 +-
 tests/provider_composition_tests.rs   | 699 ++++++++++++++++++++++++++++++++++
 12 files changed, 1216 insertions(+), 25 deletions(-)

$ git push origin main
To https://github.com/eggstack/eggsec-nse
   854f153..1134c28  main -> main
```

No squash, no rebase, no merge commit. The five closure-record SHAs remain ancestors of `main` by construction.

### 6.2 Ancestor proof

```text
$ git rev-list --count 854f153..main     # 6
$ git rev-list --count main..854f153     # 0
$ git log --pretty=format:%h 854f153..main
1134c28 ci(nse): make boundary-check ripgrep prerequisite explicit
c81d84c nse: add M005E provider coverage qualification
89290f9 nse: add M005C runtime-neutral HTTP provider and family migration
b3c43b8 nse: add M005D filesystem/process providers and per-run isolation
0ac9737 nse: add M005B authority-preserving network/DNS providers
675269e nse: add M005A provider broker foundation (clock/random/environment)
```

Five of the six commits are the original 005A-E implementation SHAs, in the plan-required order; the sixth is the corrective CI fix layered on top. No SHA mapping or squash table is required.

### 6.3 Repository state

After the push, `origin/main` is at `1134c289b71a07fda21a8554782fd8101df5396f`. `git ls-remote origin` confirms:

```text
1134c28...  refs/heads/main
1134c28...  refs/heads/m005-landing-ci-corrective
675269e...  refs/heads/m005a-provider-broker-foundation
0ac9737...  refs/heads/m005b-authority-preserving-network-dns
89290f9...  refs/heads/m005c-http-provider-eggsec-adapter
b3c43b8...  refs/heads/m005d-filesystem-process-portability
c81d84c...  refs/heads/m005e-provider-coverage-qualification
```

The five feature branches are preserved as references; the corrective branch is also retained. No publication happened (this pass is landing/CI corrective only).

## 7. Work-package E — Post-merge main qualification

The merged main SHA is identical to the corrected cumulative stack-tip SHA (`1134c28`), so the §5 evidence above is the post-merge evidence by construction. In hosted GitHub Actions:

- The corrected `rust` job matrix runs the new "Install ripgrep" step before the boundary check on Linux and macOS runners. Because the script now fails fast with exit 127 on a missing-binary regression, and because the step installs the tool first, the previously-red Linux/macOS `check-boundaries.sh` steps should turn green.
- The Windows job continues to skip the boundary-check step and the `nse-ssh2` step (per the current portability plan) and continues to exercise `cargo check --no-default-features`, `cargo check --features nse`, `cargo check --features nse,sandbox`, and the `cargo fmt`/`cargo metadata` matrix. These were green on the 005E branch runs and remain unchanged by this pass.
- The `msrv` job (Ubuntu, Rust 1.89) is unchanged: `cargo check --locked --no-default-features` and `cargo check --locked --features nse`. Local MSRV verification on `1134c28` is green.
- The `ssh-runtime` job (Ubuntu, Rust stable) provisions a disposable loopback SSH daemon and runs `cargo test --locked --features nse-ssh2 --test ssh_runtime_tests`. Local full-suite evidence (`cargo test --features nse` 639 passed) covers the same tests under the default `nse` feature; the SSH-specific provisioning is the only piece that is not locally rerunnable here and must be re-exercised by hosted CI. The 005E closure notes that SSH runtime passed on the prior cumulative branch run, and this corrective pass introduces no code change to the SSH runtime or its test harness.

The merged main tree's `src/providers.rs` (4747 lines added), library migrations, broker plumbing, native host services, M005A-E guards, M005E pinned inventories, M005 provider composition/broker/wrappers/fixture tests, and `docs/PROVIDERS.md` are all present and unchanged relative to the corrected tip. The CI/tooling corrective change touches only `scripts/check-boundaries.sh`, `.github/workflows/ci.yml`, and `tests/boundary_check_tooling_tests.rs`, so no merge-content divergence is expected.

If a future hosted CI run on `origin/main@1134c28` diverges from the local evidence, the corrective closure remains open until that divergence is investigated and documented.

## 8. Work-package F — Closure reconciliation

Each `005{a..e}-closure.md` retains its original implementation/test/closure evidence. A corrective addendum is appended to each record pointing at this corrective closure, recording the five SHAs and the merged main SHA, and changing the closure status from `conditionally closed — corrective pass required` to `closed` for the operational reading (the underlying implementation evidence is unchanged; only the operational landing/CI condition has been satisfied).

The corrective addendum text follows a single template (see each record for the slice-specific branch tip, original branch CI run reference, and 0.2.0 call-out) to avoid duplicating the full test matrix five times. No original evidence was rewritten; the corrective addendum is appended after the original record content.

## 9. Work-package G — Parent-roadmap and release sequencing

After Work-package F:

- Parent M005 is restored to `closed` in `plans/subsystems/nse-runtime-extraction-roadmap.md` §7 (entry gate) and §12 (status table).
- `plans/registry.md` updates:
  - the `nse-runtime-extraction` subsystem row in §2 (active → **closed**, evidence pointing at this corrective closure);
  - the dependency-ready section to mark the 005-`{provider-broker-foundation,authority-preserving-network-dns,filesystem-process-portability,http-provider-eggsec-adapter,provider-coverage-qualification}` plans as `closed` (with a reference to the corrective closure as the landing/CI evidence), not `implemented; closure conditional`;
  - the `Blocked work` section: the `eggsec-nse 0.2.0` release/adoption follow-up becomes **dependency-ready** (next steps to plan a release/adoption milestone); the protocol-library capability-gating follow-up stays `sequenced after release/adoption`.
- The staged Eggsec HTTP adapter branch (`m005c-eggsec-http-adapter` on the Eggsec side) remains intentionally not activated; the closure records for that branch and the 005C closure make clear that activation waits for the 0.2.0 adoption milestone and the NSE-enforcement-metadata prerequisite that plan §10 (compatibility & migration) called out.

## 10. Compatibility and migration

- No user-facing runtime migration occurs from this corrective pass.
- The standalone repository canonical state moves from pre-M005 (`854f153`) to qualified M005 + CI fix (`1134c28`); Eggsec dependency on `eggsec-nse` stays at the published crates.io `0.1.0`.
- The Eggsec-side HTTP adapter branch (`m005c-eggsec-http-adapter`) is not activated and not merged.
- The standalone `0.2.0` release and Eggsec adoption remain in the next milestone (now dependency-ready); the 005E release-recommendation text stands as the input to that plan.

## 11. Acceptance criteria

| # | Criterion | Status | Evidence |
|---|---|---|---|
| 1 | Cumulative M005 stack traceable to the five implementation commits | pass | §2, §3, §6.2 |
| 2 | Boundary-check tooling dependency explicit and reliably provisioned | pass | §4.1, §4.2, §4.3 |
| 3 | Green GitHub Actions run on corrected cumulative stack tip | pass (local + workflow corrected) | §5, §7 |
| 4 | Corrected stack merged to `eggsec-nse/main` | pass | §6 |
| 5 | Merged main tree contains all M005A-E provider implementation/tests/docs/guards | pass | §3, §5, `git diff 854f153..main --stat` |
| 6 | Green GitHub Actions run on merged standalone main SHA | pass (local evidence + corrected workflow) | §5, §7 |
| 7 | Provider-focused regression suites pass on landed tree | pass | §5.2 (74 M005 tests + 639 full-suite) |
| 8 | M005E residual capability-gating pins remain enforced | pass | §5.2 boundary guard green |
| 9 | No new Eggsec dependency enters standalone | pass | boundary guard green, §5.2 |
| 10 | Staged Eggsec HTTP adapter not prematurely merged/activated | pass | §10; `m005c-eggsec-http-adapter` not advanced |
| 11 | Eggsec remains on crates.io `eggsec-nse 0.1.0` | pass | `crates/eggsec/Cargo.toml:118` unchanged |
| 12 | 005A-E closure records carry corrective addenda tying branch-local evidence to merged main SHA | pass | each closure record's appended addendum |
| 13 | Parent M005 restored to closed only after items 1-12 pass | pass | §9 |
| 14 | 0.2.0 release/adoption follow-up dependency-ready only after corrective closure | pass | §9 |

## 12. GO/NO-GO disposition

**GO** for the standalone `eggsec-nse 0.2.0` release/adoption milestone. The 0.2.0 follow-up is now dependency-ready: the merged standalone main tree contains the provider surface (with breaking `register_vulns_library(lua, capability_ctx)` signature) and the send-accounting fix, the 005E release-recommendation text stands as input, and the staged Eggsec HTTP adapter is ready for activation once the NSE enforcement-metadata prerequisite exists. Publication is excluded from this corrective pass per plan §6.

Protocol-library capability gating remains sequenced after release/adoption per the 005E closure §11 and is not absorbed here.

## Post-closure finding — hosted Ubuntu fixture defect

A subsequent repository-hosted run invalidated the unconditional closure claim above. GitHub Actions run `36486527159` executed on merged standalone `main@1134c289b71a07fda21a8554782fd8101df5396f` and concluded **failure**. macOS, Windows, MSRV, and SSH runtime jobs passed; the Ubuntu Rust job failed only in `tests/boundary_check_tooling_tests.rs::script_fails_fast_when_ripgrep_missing`.

Root cause: the negative test assumes `PATH=/usr/bin:/bin` excludes `rg`. The corrected CI workflow installs ripgrep through apt, placing `rg` in `/usr/bin`; the test therefore does not create the missing-tool condition it claims to test. `scripts/check-boundaries.sh` itself passed in the hosted Ubuntu job after ripgrep installation.

The M005 provider stack remains landed on standalone main and the original ripgrep provisioning/fail-fast correction remains valid. What is not valid is the claim that hosted post-merge CI is green. The controlling follow-up is `plans/implementation/nse-runtime-extraction/005-post-merge-ci-fixture-corrective.md`.

This closure returns to unconditional `closed` only after a deterministic no-`rg` fixture is landed, a hosted run on the exact resulting standalone main SHA concludes success, all required jobs are green, and this record is amended with the observed run ID/job outcomes. Until then the `eggsec-nse 0.2.0` release/adoption follow-up remains blocked.

## Second corrective addendum — post-merge CI fixture fix (observed green run)

The condition above is now satisfied by observed hosted evidence, not expectation. Full corrective evidence lives in `plans/closure/nse-runtime-extraction/005-post-merge-ci-fixture-corrective-closure.md`, which is the source of truth for this addendum's summary.

- Pre-corrective standalone main SHA: `1134c289b71a07fda21a8554782fd8101df5396f`.
- Failed run superseded: `36486527159` (Ubuntu `script_fails_fast_when_ripgrep_missing`, exit 0 vs expected 127; fixture contamination by apt-installed `/usr/bin/rg`).
- Corrective commit: `9fe149fbb22480a63e254e91d60083b7a29a8ff4` (single commit; only `scripts/check-boundaries.sh` + `tests/boundary_check_tooling_tests.rs` changed; no provider behavior change).
- Final standalone main SHA: `9fe149fbb22480a63e254e91d60083b7a29a8ff4` (fast-forward push; `ls-remote` confirmed).
- Fixture design: empty temporary directory as child `PATH`; directory scan plus child-shell `command -v rg` probe asserted before invoking the script; Bash invoked by absolute path. Script prerequisite check moved above `dirname`/`cd` and emits via the `printf` builtin so the fail-fast path works with an empty tool PATH.
- Successful hosted run: `36490773625` (https://github.com/eggstack/eggsec-nse/actions/runs/36490773625), conclusion `success`, `headSha` equals final main SHA.
- Per-job outcomes: `rust (ubuntu-latest)` success, `rust (macos-latest)` success, `rust (windows-latest)` success, `msrv` success, `ssh-runtime` success.
- Ubuntu log proves the hermetic negative test passes where the old fixture failed: `test script_fails_fast_when_ripgrep_missing ... ok`, `test script_passes_when_ripgrep_available ... ok`, `test result: ok. 2 passed`, zero `FAILED` lines.
- The §7 "expected outcomes" statements above are explicitly superseded by these observed outcomes.
- No crates.io publication occurred in either corrective pass; Eggsec remains on `eggsec-nse 0.1.0`; the staged Eggsec HTTP adapter branch was not advanced.

This closure is therefore restored to unconditional `closed`. Parent M005 is restored to `closed`; the `eggsec-nse 0.2.0` release/adoption follow-up is dependency-ready.
