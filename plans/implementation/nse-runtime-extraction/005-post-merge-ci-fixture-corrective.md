# NSE Runtime Extraction Milestone 005 — Post-Merge CI Fixture Corrective Pass

Status: ready for handoff

Eggsec planning baseline: `222bd171b480130f5184af40f390fbca0b8203ed`

Standalone main baseline: `eggstack/eggsec-nse@1134c289b71a07fda21a8554782fd8101df5396f`

Failed hosted post-merge CI:

- GitHub Actions run `36486527159` on `eggsec-nse/main@1134c289b71a07fda21a8554782fd8101df5396f`
- Ubuntu Rust job failed; macOS, Windows, MSRV, and SSH runtime jobs passed.
- Ubuntu failure is isolated to `tests/boundary_check_tooling_tests.rs::script_fails_fast_when_ripgrep_missing`.

Controlling prior corrective:

- `plans/implementation/nse-runtime-extraction/005-provider-stack-landing-ci-corrective.md`
- `plans/closure/nse-runtime-extraction/005-provider-stack-landing-ci-corrective-closure.md`

Source roadmap:

- `plans/subsystems/nse-runtime-extraction-roadmap.md#milestone-005--host-provider-inversion-and-portability-hardening`

Applicable ADRs:

- `plans/adrs/ADR-0003-nse-host-provider-boundary.md`
- `plans/adrs/ADR-0001-scoped-transport-eggfetch-backend.md`

Primary class: infrastructure

Affected repositories:

- test/CI correction: `eggstack/eggsec-nse`;
- planning/closure reconciliation: `eggstack/eggsec`.

## 1. Objective

Correct the final hosted-CI defect in the M005 landing evidence without reopening provider implementation, changing provider semantics, or altering the already-landed M005 commit chain.

The previous corrective successfully:

- landed the full M005A-E stack on standalone `main`;
- made ripgrep an explicit CI prerequisite;
- made `scripts/check-boundaries.sh` fail fast when `rg` is unavailable;
- preserved the original M005 implementation SHAs as ancestors of main.

However, its negative regression test is environment-dependent. The test uses:

```rust
const MINIMAL_PATH: &str = "/usr/bin:/bin";
```

to simulate a missing `rg`. The corrected Ubuntu workflow installs ripgrep through apt, placing `rg` in `/usr/bin`. The supposed no-ripgrep PATH therefore contains ripgrep, the script correctly runs to completion, and the test incorrectly expects exit 127.

This pass must make the negative test deterministic, obtain a fully green hosted GitHub Actions run on the resulting standalone `main` SHA, and reconcile the prior corrective closure so its claims are based on observed hosted evidence rather than expected outcomes.

## 2. Why this corrective pass is required

Actual hosted evidence on merged standalone main contradicts the prior corrective closure:

- run `36486527159` conclusion: **failure**;
- `rust (macos-latest)`: success;
- `rust (windows-latest)`: success;
- `msrv`: success;
- `ssh-runtime`: success;
- `rust (ubuntu-latest)`: failure.

The Ubuntu log proves:

```text
test script_fails_fast_when_ripgrep_missing ... FAILED

assertion left == right failed:
expected exit 127 when rg is missing; got Some(0)

standalone boundary and provenance checks passed
```

This is a test-fixture defect, not a provider/runtime defect and not a failure of the actual ripgrep prerequisite installation.

The previous closure currently states `Status: closed` and marks hosted branch/post-merge CI acceptance criteria as passed. Those claims must be treated as conditional until a real all-green hosted run exists.

## 3. Current implementation evidence that remains valid

The following corrective results remain valid and are not reopened:

- M005A-E commits are all ancestors of standalone main.
- Standalone main contains the full provider stack.
- `1134c28` installs/provisions ripgrep in Linux/macOS CI before running boundary guards.
- `scripts/check-boundaries.sh` has an explicit `command -v rg` prerequisite and exit 127.
- The real boundary checker itself passed in the Ubuntu hosted run after ripgrep installation.
- The hosted macOS, Windows, MSRV, and SSH jobs passed.
- M005 provider behavior, accounting correction, Windows portability, and pinned residual inventories are not implicated by the failing test.

This pass must preserve those facts and change only what is necessary to make the missing-tool regression test independent of the host package layout.

## 4. Invariants that must not regress

- Do not change any M005 provider trait, DTO, broker, native provider, library migration, accounting behavior, or profile semantics.
- Do not weaken or skip `scripts/check-boundaries.sh`.
- Do not remove the explicit ripgrep installation from hosted CI merely to make the negative test pass.
- Do not rely on a host-specific path such as `/usr/bin:/bin` to mean "tool absent."
- The negative test must prove `rg` is actually unavailable in the child environment before asserting the script's failure behavior.
- The positive tooling test must continue exercising the real boundary script with a normal PATH containing `rg`.
- Windows compile/check, MSRV 1.89, and SSH runtime jobs remain required.
- No crates.io publication occurs in this pass.
- Eggsec remains on `eggsec-nse 0.1.0`.
- The staged Eggsec HTTP adapter remains staged.
- The protocol-library capability-gating follow-up remains separate.
- M005/0.2.0 release readiness may only be restored after an observed fully green hosted run on the exact new standalone main SHA.

## 5. Scope

### In scope

Standalone:

- make `script_fails_fast_when_ripgrep_missing` deterministic;
- adjust the boundary script's fail-fast placement only if needed to permit a hermetic no-PATH/no-`rg` test;
- keep the ripgrep prerequisite diagnostic concise and exit 127;
- run the focused tooling test locally;
- push the minimal fix to standalone `main`;
- obtain a hosted GitHub Actions run on that exact main SHA;
- require all Rust matrix, MSRV, and SSH jobs to succeed.

Eggsec planning:

- mark the prior landing/CI corrective closure conditional while this plan is open;
- mark parent M005 operational closure corrective-required;
- block the 0.2.0 release/adoption follow-up;
- after a real green hosted run, append a factual closure addendum with the successful run ID/job outcomes and restore closed/dependency-ready state.

### Preferred fixture design

The implementation should make the negative environment hermetic rather than guessing where `rg` is installed.

Preferred approach:

1. Move the `command -v rg` prerequisite check to the beginning of `check-boundaries.sh`, before any external utility such as `dirname` is needed.
2. Emit the missing-tool diagnostic using Bash builtins (`printf`/equivalent) so the fail-fast path itself does not require external binaries.
3. In the negative Rust test:
   - invoke Bash by an absolute path or otherwise independently resolved executable;
   - set the child `PATH` to an empty temporary directory or another controlled directory known to contain no `rg`;
   - assert the controlled PATH contains no `rg` before invoking the script;
   - assert exit 127, one diagnostic, and no success marker.
4. Keep the positive test on the inherited/provisioned PATH.

An alternative design is acceptable if it proves tool absence hermetically and does not depend on distro/macOS installation paths.

### Explicitly out of scope

- Provider feature work.
- Protocol-gating work.
- `eggsec-nse 0.2.0` publication.
- Eggsec dependency adoption.
- Eggsec HTTP adapter merge/activation.
- Reworking the entire CI workflow.
- Removing ripgrep as a boundary-check dependency unless a separate equivalence proof shows every guard remains semantically identical.
- Fixing unrelated warning debt.

## 6. Required production/test changes

No runtime production-code change is expected.

Expected standalone changes should be limited to some subset of:

- `tests/boundary_check_tooling_tests.rs`;
- `scripts/check-boundaries.sh` if fail-fast placement/builtin output needs adjustment;
- comments/docs directly describing the tooling contract.

The GitHub Actions workflow should remain functionally unchanged unless a minimal correction is necessary.

### Test contract

The negative test must establish a true invariant:

```text
child environment has no resolvable rg
-> run check-boundaries.sh
-> prerequisite check executes before normal guards
-> exit 127
-> one actionable diagnostic
-> no "checks passed" marker
```

The positive test must establish:

```text
rg available
-> normal guard suite executes
-> exit 0
-> success marker present
```

The test must not encode assumptions about where apt, Homebrew, or developer installations place `rg`.

## 7. Ordered work packages

### Work package A — Reproduce and lock the fixture defect

Required evidence:

- run the current negative test on an environment where `rg` is in `/usr/bin`;
- show that `MINIMAL_PATH=/usr/bin:/bin` resolves `rg`;
- confirm the observed exit 0 is the expected behavior of the corrected script, not a hidden boundary-check bypass.

Acceptance:

- root cause is documented as fixture contamination by the installed tool.

### Work package B — Make missing-tool simulation hermetic

Implement a controlled no-`rg` environment.

Required properties:

- child shell is executable without depending on the child PATH;
- child PATH contains no `rg`;
- prerequisite check runs before external commands that would themselves fail due the intentionally minimal PATH;
- no fake/placeholder `rg` binary is present.

Acceptance:

- the test itself can prove `rg` absence;
- the script exits 127 for the intended reason.

### Work package C — Run focused local regression

Run at minimum:

```bash
cargo fmt --all --check
cargo test --features nse --test boundary_check_tooling_tests -- --nocapture
./scripts/check-boundaries.sh
```

Also run a direct negative invocation equivalent to the test's hermetic child environment.

Acceptance:

- both tooling tests pass;
- real boundary guards pass with normal PATH;
- negative path emits one clear diagnostic.

### Work package D — Run standalone regression smoke

Because the change touches only CI/test tooling, a bounded regression is sufficient before push:

```bash
cargo check --features nse
cargo test --features nse --test provider_composition_tests
cargo +1.89.0 check --locked --features nse
```

Do not reinterpret this bounded local smoke as a substitute for hosted CI.

### Work package E — Push and obtain real hosted main CI

Push the minimal corrective commit to `eggsec-nse/main`.

Required hosted jobs on the exact new main SHA:

- `rust (ubuntu-latest)`: success;
- `rust (macos-latest)`: success;
- `rust (windows-latest)`: success;
- `msrv`: success;
- `ssh-runtime`: success.

Acceptance:

- workflow conclusion is `success`;
- no required job is skipped unexpectedly;
- Ubuntu tooling tests pass in the environment where apt-installed ripgrep is actually present.

### Work package F — Reconcile closure evidence from observed results

Only after Work package E succeeds:

- append a post-merge CI fixture corrective addendum to the prior corrective closure;
- record the new standalone main SHA;
- record the successful hosted run ID/URL;
- record each required job conclusion;
- explicitly supersede the earlier "expected outcomes" statements with observed outcomes;
- restore the prior corrective closure to closed;
- restore parent M005 closed;
- restore 0.2.0 release/adoption to dependency-ready.

Do not claim a future run will pass; the run must already have completed successfully.

## 8. Failure, cancellation, restart, and contention semantics

No runtime lifecycle behavior changes.

CI/test failure handling:

- any red required hosted job keeps this corrective pass open;
- if fixing the hermetic fixture reveals a real boundary-script failure, correct the real guard defect rather than weakening the test;
- if a provider/runtime test fails after this tooling-only change, stop and classify it separately before release planning.

Repository state:

- if standalone main advances before the corrective push, rebase/merge as normal and qualify the exact resulting SHA;
- the successful hosted run must reference the same SHA recorded in closure.

## 9. Compatibility and migration

No runtime or consumer migration.

Before:

```text
eggsec-nse/main = M005 landed at 1134c28
hosted CI = red due environment-dependent negative test
Eggsec planning = incorrectly says M005 fully closed / 0.2.0 ready
```

After:

```text
eggsec-nse/main = M005 + deterministic tooling-test fix
hosted CI = fully green on exact main SHA
Eggsec planning = M005 closed from observed evidence
0.2.0 release/adoption = dependency-ready
```

## 10. Required tests

### Tooling fixture tests

- missing-ripgrep environment genuinely contains no `rg`;
- fail-fast exits 127;
- diagnostic appears exactly once;
- success marker absent;
- normal PATH executes all guards and returns success.

### Guard regression

- existing M005A-E boundary checks remain active;
- M005E pinned residual inventories remain enforced.

### Hosted matrix

- Ubuntu;
- macOS;
- Windows;
- MSRV 1.89;
- SSH runtime.

### Security/negative tests

- do not convert the missing-tool case to success;
- do not skip boundary guards in Linux/macOS;
- do not set `continue-on-error`;
- do not mask test failure with shell fallbacks.

## 11. Required verification commands

Standalone local:

```bash
cargo fmt --all --check
./scripts/check-boundaries.sh
cargo test --features nse --test boundary_check_tooling_tests -- --nocapture
cargo check --features nse
cargo test --features nse --test provider_composition_tests
cargo +1.89.0 check --locked --features nse
```

After push, inspect the hosted run rather than relying on local inference.

Eggsec planning repository after hosted success:

```bash
make test-architecture-guards
make check
```

if those commands are applicable to the documentation-only reconciliation commit.

## 12. Documentation updates

Standalone:

- comments in the tooling regression test should describe hermetic PATH isolation rather than fixed system directories;
- boundary script comments should accurately describe fail-fast ordering.

Eggsec:

- prior corrective closure receives an addendum with observed hosted evidence;
- roadmap and registry reflect this second corrective while open and restore closure only after success.

No provider architecture documentation changes are expected.

## 13. Acceptance criteria

1. The negative tooling test no longer assumes `/usr/bin:/bin` excludes ripgrep.
2. The child test environment proves `rg` is unavailable.
3. `check-boundaries.sh` fails with exit 127 before normal guards in the hermetic no-`rg` environment.
4. The missing-tool diagnostic appears once and no success marker is printed.
5. The positive tooling test passes with normal/provisioned PATH.
6. Existing boundary guards remain unchanged in semantics and pass normally.
7. The corrective change is pushed to `eggsec-nse/main`.
8. A hosted GitHub Actions run on the exact new main SHA concludes success.
9. Ubuntu, macOS, Windows, MSRV, and SSH runtime jobs all conclude success.
10. No provider/runtime production behavior changes.
11. The prior landing/CI closure is amended with the actual successful run rather than predicted outcomes.
12. Parent M005 is restored to closed only after hosted success.
13. The 0.2.0 release/adoption follow-up is restored to dependency-ready only after hosted success.
14. Staged Eggsec HTTP adapter and protocol-gating follow-up sequencing remain unchanged.

## 14. Stop conditions

Stop and report if:

- the tooling test cannot simulate missing `rg` without relying on host-specific paths;
- moving the prerequisite check changes normal boundary-guard semantics;
- any required hosted job remains red;
- a real provider/runtime regression appears;
- fixing CI would require disabling/weakening guards;
- release/adoption or protocol-gating work begins entering this pass.

## 15. Closure evidence required

The closure/addendum must record:

- pre-corrective standalone main SHA `1134c289b71a07fda21a8554782fd8101df5396f`;
- failed run `36486527159` and Ubuntu failure root cause;
- corrective commit SHA;
- final standalone main SHA;
- exact negative-fixture design;
- focused tooling-test results;
- boundary-guard result;
- provider-composition/MSRV smoke results;
- successful hosted GitHub Actions run ID/URL;
- per-job outcomes for Ubuntu/macOS/Windows/MSRV/SSH;
- proof the successful run SHA equals final standalone main;
- Eggsec roadmap/registry reconciliation commit;
- explicit confirmation 0.2.0 publication did not occur;
- GO/NO-GO for the 0.2.0 release/adoption milestone.

## 16. Handoff notes

The prior corrective solved the real CI prerequisite defect and landed the M005 stack. Do not undo that work.

The only current correctness issue is that the negative regression test's "missing tool" environment is not actually guaranteed to be missing the tool. Make that environment hermetic, prove it in hosted CI, and base closure on the observed successful run.

Do not close this pass from local evidence alone.
