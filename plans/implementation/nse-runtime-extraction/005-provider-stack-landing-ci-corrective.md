# NSE Runtime Extraction Milestone 005 — Provider Stack Landing and CI Corrective Pass

Status: ready for handoff

Eggsec planning baseline: `8e5615f206581165dade2719ca850ecb21b2d25b`

Standalone main baseline: `eggstack/eggsec-nse@854f153f56d1abc929d9abd255f0606759342f81`

Standalone M005 stack tip: `eggstack/eggsec-nse@m005e-provider-coverage-qualification@c81d84c55a2368a76338e758ab3afdf3573135ad`

Source roadmap:

- `plans/subsystems/nse-runtime-extraction-roadmap.md#milestone-005--host-provider-inversion-and-portability-hardening`

Original implementation plans and closure records:

- `plans/implementation/nse-runtime-extraction/005-provider-broker-foundation.md` / `plans/closure/nse-runtime-extraction/005a-closure.md`
- `plans/implementation/nse-runtime-extraction/005-authority-preserving-network-dns.md` / `plans/closure/nse-runtime-extraction/005b-closure.md`
- `plans/implementation/nse-runtime-extraction/005-filesystem-process-portability.md` / `plans/closure/nse-runtime-extraction/005d-closure.md`
- `plans/implementation/nse-runtime-extraction/005-http-provider-eggsec-adapter.md` / `plans/closure/nse-runtime-extraction/005c-closure.md`
- `plans/implementation/nse-runtime-extraction/005-provider-coverage-qualification.md` / `plans/closure/nse-runtime-extraction/005e-closure.md`

Applicable ADRs:

- `plans/adrs/ADR-0003-nse-host-provider-boundary.md`
- `plans/adrs/ADR-0001-scoped-transport-eggfetch-backend.md`

Primary class: infrastructure

Affected repositories:

- landing/CI correction: `eggstack/eggsec-nse`;
- planning/closure reconciliation: `eggstack/eggsec`.

## 1. Objective

Correct the operational closure defect in Milestone 005 without reopening its provider implementation scope.

The M005 implementation is present as one linear standalone branch stack, but the stack has never landed on `eggsec-nse/main` and every GitHub Actions CI run for the implementation branches is red on Linux/macOS because `scripts/check-boundaries.sh` requires `rg` while the workflow does not install ripgrep.

This corrective pass must:

1. make the boundary-check tooling dependency explicit and reliable in CI;
2. qualify the full cumulative M005 stack at its tip with green GitHub Actions;
3. land that exact qualified stack on standalone `main` with traceable commit identity;
4. obtain green post-merge `main` CI;
5. reconcile the 005A-E closure records, roadmap, and registry to the actual merged standalone SHA;
6. leave the staged Eggsec HTTP adapter and the planned `0.2.0` release/adoption work correctly sequenced after this corrective closure.

No provider-domain feature work belongs in this pass.

## 2. Why this corrective pass is required

The original closures contain substantial local/focused verification, but closure accepted branch-local implementation as if it were repository-landed implementation and did not require a successful GitHub Actions conclusion or a merged-main identity.

Current repository evidence contradicts the closed state:

- `eggsec-nse/main` is still `854f153f56d1abc929d9abd255f0606759342f81`, the pre-M005 post-release main.
- `src/providers.rs` is absent from standalone `main`.
- No M005 implementation PR exists for the standalone repository.
- The five implementation commits form a clean linear chain:
  - 005A `675269e4a26314019ecc1c87a2bf9081887c1da0`
  - 005B `0ac9737486df8c76d16ad8437ea83396bcf544b0`
  - 005D `b3c43b8d2128f47d83ea66ceb94fcbfe5dc9c69c`
  - 005C `89290f95d493e557dd0f07e64818cb6ae10e39a3`
  - 005E `c81d84c55a2368a76338e758ab3afdf3573135ad`
- Every standalone branch CI run is red:
  - 005A run `36337158009`
  - 005B run `36455834771`
  - 005D run `36461074205`
  - 005C runs `36466449173` / `36466452158`
  - 005E run `36476950685`
- On the latest cumulative 005E run, `msrv`, `ssh-runtime`, and Windows passed; Linux/macOS failed at the boundary-check step because `rg` was not installed.
- The same missing-tool failure is present on the earlier M005 branch runs, so this is a CI-contract defect rather than evidence of a provider-code regression.

The original verification missed this because local environments had ripgrep available and closure relied on local command outcomes without checking the repository-hosted CI conclusion and merged-main state.

## 3. Current branch-stack evidence

The standalone stack is linear and has not diverged from main:

```text
854f153  standalone main / pre-M005
   |
675269e  005A provider broker foundation
   |
0ac9737  005B network/DNS providers
   |
b3c43b8  005D filesystem/process portability
   |
89290f9  005C HTTP provider family migration
   |
c81d84c  005E coverage qualification/accounting correction
```

This permits one cumulative qualification and one history-preserving landing instead of independently merging stale intermediate branches.

The Eggsec-specific `m005c-eggsec-http-adapter` branch is intentionally **not** part of this landing corrective. Its accepted closure explicitly stages it until a new standalone runtime release is adopted. Do not merge or activate that adapter here.

## 4. Invariants that must not regress

- Do not change ADR-0003 provider semantics merely to make CI green.
- Do not weaken/remove boundary guards to avoid installing their declared tooling.
- The standalone runtime must remain free of `eggsec-*` dependencies.
- The cumulative stack must preserve the accepted 005A-E behavior and tests.
- Windows qualification introduced by 005D remains enabled.
- MSRV 1.89 and SSH runtime qualification remain green.
- The 72-file specialized ungated residual identified by 005E remains pinned/documented; this corrective pass does not absorb the protocol-gating follow-up.
- The M005E send-accounting correction remains intact.
- No crates.io release is published from this corrective pass.
- Eggsec continues consuming `eggsec-nse 0.1.0` until the separate 0.2.0 release/adoption milestone.
- The staged Eggsec HTTP adapter remains off main until its declared release/adoption prerequisite is satisfied.
- Existing closure evidence must remain traceable; reconciliation adds corrective evidence rather than pretending the branch-only phase never happened.

## 5. Scope

### In scope

- Repair the standalone CI boundary-check prerequisite contract.
- Prefer an explicit, fail-fast ripgrep dependency:
  - `check-boundaries.sh` should detect missing `rg` with one clear diagnostic before running checks;
  - every GitHub Actions job/OS that invokes the script must install/provide ripgrep explicitly.
- An equivalent self-contained rewrite of the guard script is acceptable only if it preserves all current regex semantics and guard coverage; do not silently downgrade checks to simpler matching.
- Commit the CI/tooling correction on the cumulative M005 stack tip.
- Run/obtain full green branch CI at the corrected cumulative tip.
- Review `main...corrected-tip` to prove it consists only of the accepted M005 stack plus the CI/tooling corrective change.
- Land the corrected cumulative stack to standalone `main`.
- Prefer history-preserving integration so the five closure-record commit SHAs remain ancestors of main. If repository policy forces squash, record a complete old-SHA -> merged-SHA/file-diff mapping in corrective closure.
- Obtain green post-merge CI on the resulting standalone main SHA.
- Add corrective addenda/status reconciliation to 005A-E closure records.
- Reconcile parent M005 status and registry only after the merged-main evidence exists.
- Record the exact merged standalone SHA as the source candidate for the later 0.2.0 release plan.

### Explicitly out of scope

- New provider domains.
- Reworking the 005A-E provider APIs.
- Protocol-library capability gating for the 72 ungated residual files.
- Publishing `eggsec-nse 0.2.0`.
- Bumping Eggsec from 0.1.0.
- Merging/activating `m005c-eggsec-http-adapter`.
- Adding NSE operation enforcement metadata/scope threading in Eggsec.
- Fixing unrelated pre-existing clippy warnings.
- Rewriting closure test results that were actually run.

## 6. Required production and CI changes

### Standalone CI/tooling contract

The boundary checker currently assumes `rg` without declaring it.

Corrective implementation must make this dependency explicit. Preferred shape:

1. `scripts/check-boundaries.sh` performs an upfront `command -v rg` check and exits with a concise actionable error if absent.
2. Linux/macOS GitHub Actions jobs install/provide ripgrep before invoking the script.
3. Windows may continue skipping the shell boundary script if the current portability plan intentionally does so; Windows compile/check qualification remains mandatory.
4. Installation must not require secrets or mutable repository credentials.
5. CI must not mask boundary-check failures with `|| true`.

If implementation chooses to eliminate the `rg` dependency instead, closure must show guard-equivalence tests/source review proving none of the existing M005A-E checks were weakened.

### Standalone landing

Do not cherry-pick isolated provider commits onto main in a different order. Preserve the accepted stack order unless a merge conflict makes that impossible.

The landing candidate must be one exact commit reachable from:

```text
854f153 -> 675269e -> 0ac9737 -> b3c43b8 -> 89290f9 -> c81d84c -> <CI-fix>
```

or a traceably equivalent history-preserving merge.

### Eggsec production code

No Eggsec production-code change is required.

Planning/closure documentation is updated to reflect operational reality.

## 7. Ordered work packages

### Work package A — Freeze and verify the cumulative stack

Intent:

Prevent later branch movement from invalidating closure evidence.

Required changes/evidence:

- record all five existing branch SHAs and the current standalone main SHA;
- confirm 005E is descendant of 005C -> 005D -> 005B -> 005A -> main;
- confirm the stack is not behind/diverged from standalone main before correction;
- capture `main...m005e-provider-coverage-qualification` changed-file inventory.

Acceptance:

- one exact cumulative source identity is established before modification.

### Work package B — Repair the CI prerequisite contract

Intent:

Make boundary enforcement runnable in repository-hosted CI rather than relying on developer workstation state.

Required changes:

- explicit `rg` prerequisite check or equivalent self-contained guard implementation;
- CI provisioning on every non-Windows job that runs `check-boundaries.sh`;
- concise documentation/comment identifying the dependency.

Regression requirement:

- add a focused script/tooling test or CI preflight that fails clearly when the required tool is unavailable; the failure must occur before dozens of repeated `rg: command not found` lines.

Acceptance:

- boundary checker passes in a clean runner after declared setup;
- missing-tool behavior is deterministic and actionable.

### Work package C — Qualify the corrected cumulative branch tip

Run the real GitHub Actions workflow on the corrected tip.

Required jobs:

- Rust / Ubuntu: success;
- Rust / macOS: success;
- Rust / Windows: success;
- MSRV: success;
- SSH runtime: success.

Also run/record focused local commands as needed to confirm the CI-only change did not alter implementation behavior.

Acceptance:

- one GitHub Actions run on the exact corrected stack tip is fully green.

### Work package D — Land the stack to standalone main

Intent:

Make the code described by the 005A-E closures part of the canonical standalone repository.

Required changes:

- open/review a PR or use the repository's normal protected-main integration path;
- ensure the head SHA matches the green corrected tip;
- preserve branch history where repository policy allows;
- merge only after branch CI is green.

Acceptance:

- `eggsec-nse/main` contains `src/providers.rs`, all M005 tests/guards/docs, and the CI correction;
- the five implementation SHAs are ancestors of main or have an explicit squash mapping.

### Work package E — Post-merge main qualification

Intent:

Do not treat branch CI as sufficient landing evidence.

Required evidence:

- GitHub Actions run on the merged standalone main SHA;
- all required jobs green;
- clean package build;
- boundary guards green;
- current provider composition suite green.

If main CI differs from branch CI because of merge-generated content, investigate and correct before closure.

### Work package F — Reconcile M005 closure evidence

For each `005a-closure.md` through `005e-closure.md`:

- preserve the original implementation/test evidence;
- append a corrective addendum explaining that the original implementation existed on a branch and local verification passed, but hosted CI/merge evidence was missing;
- record the original implementation SHA;
- record the corrected stack-tip SHA;
- record the merged standalone main SHA;
- link the green branch and post-merge CI runs;
- return the closure status to closed only after those conditions are satisfied.

The corrective closure record should summarize the chain once rather than duplicating full test matrices five times.

### Work package G — Restore parent-roadmap and release sequencing

After Work Package F:

- mark parent M005 closed again;
- point the roadmap/registry at the corrective closure;
- unblock the standalone `0.2.0` release/adoption planning path;
- keep protocol-library capability gating as the separate medium-severity follow-up identified by 005E;
- keep the Eggsec HTTP adapter staged until 0.2.0 adoption and its enforcement prerequisite are satisfied.

## 8. Failure, cancellation, restart, and contention semantics

No runtime behavior is redesigned.

CI failure handling:

- a red required job keeps the corrective milestone open;
- do not merge with required Linux/macOS jobs red merely because MSRV/Windows/SSH pass;
- do not bypass boundary checks;
- if the corrected workflow exposes a real provider test/build failure after `rg` is fixed, classify it as implementation correctness debt and write/execute the smallest corrective change before landing.

Merge failure handling:

- if main advances independently before integration, rebase/merge and rerun the full branch CI against the new base;
- never claim the old green SHA qualifies a different merged tree.

## 9. Compatibility and migration

No user-facing runtime migration occurs until the later 0.2.0 release.

This corrective pass changes repository canonical state only:

Before:

```text
eggsec-nse/main -> pre-M005 0.1.0-era source
feature branches -> M005 implementation
Eggsec planning -> says M005 closed
```

After:

```text
eggsec-nse/main -> qualified M005 implementation + CI fix
Eggsec planning -> M005 closed against merged-main evidence
Eggsec dependency -> still crates.io 0.1.0
```

The later release/adoption milestone remains responsible for publishing and consumer migration.

## 10. Required tests

### CI prerequisite regression

- missing-ripgrep path produces one deterministic prerequisite error;
- provisioned runner executes all boundary checks.

### Standalone branch qualification

- full GitHub Actions matrix at corrected cumulative tip.

### Post-merge qualification

- full GitHub Actions matrix on main.

### Provider regression smoke

At minimum:

```bash
./scripts/check-boundaries.sh
cargo test --features nse --test provider_broker_tests
cargo test --features nse --test network_provider_tests
cargo test --features nse --test fs_process_tests
cargo test --features nse --test http_provider_tests
cargo test --features nse --test provider_composition_tests
```

### Security and negative tests

No guard may be disabled or weakened to obtain green CI.

Confirm the M005E pinned residual inventories remain present and unchanged except for intentional corrective metadata.

## 11. Required verification commands

Standalone corrected tip and merged main:

```bash
command -v rg
cargo fmt --all --check
./scripts/check-boundaries.sh
cargo metadata --no-deps
cargo check --no-default-features
cargo check --features nse
cargo test --features nse
cargo check --features nse-ssh2
cargo check --features nse,sandbox
cargo clippy --all-targets --features nse
cargo +1.89.0 check --locked --no-default-features
cargo +1.89.0 check --locked --features nse
cargo package --list
cargo package
```

And all focused M005 provider suites listed in §10.

Eggsec:

No dependency change is expected. Run planning/static consistency checks required by the repository and at least:

```bash
make test-architecture-guards
make check
```

Do not use a temporary path patch as proof of final landing; the standalone main SHA and hosted CI are the corrective evidence.

## 12. Documentation updates

Standalone:

- CI/workflow comment or contributor note documenting boundary-check prerequisite;
- no provider architecture rewrite unless correction changes tooling instructions.

Eggsec:

- append corrective references to 005A-E closure records;
- subsystem roadmap;
- planning registry;
- optionally a dedicated corrective closure:
  `plans/closure/nse-runtime-extraction/005-provider-stack-landing-ci-corrective-closure.md`.

Historical local test evidence remains intact.

## 13. Acceptance criteria

1. The cumulative M005 stack remains traceable to the five existing implementation commits.
2. The boundary-check tooling dependency is explicit and reliably provisioned or equivalently removed without weakening guards.
3. A GitHub Actions run on the corrected cumulative stack tip is fully green on Ubuntu, macOS, Windows, MSRV, and SSH jobs.
4. The corrected stack is merged to `eggsec-nse/main`.
5. The merged main tree contains all M005A-E provider implementation/tests/docs/guards.
6. A GitHub Actions run on the merged standalone main SHA is fully green.
7. Provider-focused regression suites pass on the landed tree.
8. M005E residual capability-gating pins remain enforced.
9. No new Eggsec dependency enters standalone.
10. The staged Eggsec HTTP adapter is not prematurely merged/activated.
11. Eggsec remains on crates.io `eggsec-nse 0.1.0` until the separate release/adoption milestone.
12. 005A-E closure records contain corrective addenda tying branch-local evidence to the merged main SHA and green CI.
13. Parent M005 is only restored to closed after items 1-12 pass.
14. The 0.2.0 release/adoption follow-up becomes dependency-ready only after corrective closure.

## 14. Stop conditions

Stop and report rather than forcing closure if:

- fixing the CI prerequisite exposes a real provider-code failure;
- the stack has diverged from standalone main and cannot be integrated without semantic conflict;
- merge policy would lose commit traceability and no reliable source mapping can be produced;
- required hosted CI remains red;
- landing requires weakening a boundary/security guard;
- protocol-gating or release/publication work begins to enter this corrective pass;
- the Eggsec HTTP adapter would need to be activated before 0.2.0 adoption.

## 15. Closure evidence required

The corrective closure must record:

- standalone pre-corrective main SHA;
- all five original M005 implementation SHAs;
- CI-fix commit SHA;
- corrected cumulative stack-tip SHA;
- branch CI run URL/ID and per-job results;
- integration method (PR/merge/rebase/fast-forward) and PR number if applicable;
- merged standalone main SHA;
- proof original implementation SHAs are ancestors of main, or explicit squash mapping;
- post-merge main CI run URL/ID and per-job results;
- focused provider-suite results;
- boundary-guard result;
- package/MSRV/SSH/Windows results;
- 005A-E closure addendum references;
- Eggsec planning-registry reconciliation;
- confirmation Eggsec still consumes 0.1.0;
- confirmation staged HTTP adapter remains staged;
- GO/NO-GO for the 0.2.0 release/adoption follow-up.

## 16. Handoff notes

This is a landing/evidence corrective pass, not a sixth provider implementation milestone.

The current stack is unusually favorable for correction because it is linear and standalone main has not advanced underneath it. Preserve that property if possible.

Do not mark this corrective pass complete from local tests alone. The defect being corrected is specifically the absence of repository-hosted green CI and merged-main evidence, so both are mandatory closure artifacts.
