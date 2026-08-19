# Phase D Plan: Optional, Security, and Consumer Workflow Cleanup

## Status

Status: Executed. Completed CI simplification Phase D. All acceptance criteria met.
Security tool ownership documented in `docs/VERIFICATION.md`. Deep checks workflow verified manually via `workflow_dispatch` (run 30472088644, all steps passed).

## Objective

Remove workflow surfaces that do not validate the current commit, consolidate slow and security-oriented diagnostics into at most one optional non-publishing workflow, and relocate consumer examples out of repository CI configuration.

This phase addresses workflow sprawl outside the core build/test path. It must distinguish project security from using Eggsec as a scanner against arbitrary targets.

## Scope

Primary files:

```text
.github/workflows/deep-checks.yml
.github/workflows/security-scan.yml
.gitlab-ci.yml
.github/workflows/ci.yml
Makefile
docs/VERIFICATION.md
AGENTS.md
README.md
```

Also inspect dependency policy files and any external references to these workflows.

## Required policy

At completion:

- project CI validates source and package behavior from the current commit;
- project CI does not download a previously published Eggsec binary to test itself;
- project CI does not scan `example.com`, arbitrary workflow inputs, target lists, or externally supplied URLs;
- scheduled workflows do not publish artifacts or mutate releases;
- advisory/license/secret checks are retained according to distinct value, not maximum tool count;
- consumer integration examples live in documentation or example templates, not as active repository CI.

## Workstream 1 — Remove `security-scan.yml`

Delete:

```text
.github/workflows/security-scan.yml
```

Rationale to preserve in the commit or documentation:

- it downloads a released binary rather than building the current commit;
- its `latest` release URL and artifact naming are external mutable state;
- default scans against `example.com` are not repository correctness tests;
- arbitrary workflow-dispatch targets create authorization and hosted-runner concerns;
- scheduled target-file scanning is an operator deployment pattern, not project CI;
- silent skip behavior creates a weak success signal.

Before deletion, identify any useful consumer instructions and move them to an explicitly non-executing example document, such as:

```text
the historical GitHub Actions scanning integration document (removed)
```

or an examples directory. The example must make authorization, target ownership, version pinning, and scope configuration explicit. It must not be wired to Eggsec's own repository triggers.

Do not replace the workflow with an equivalent self-scan script.

## Workstream 2 — Rationalize `deep-checks.yml`

Retain at most one optional diagnostic workflow. Recommended path:

```text
.github/workflows/deep-checks.yml
```

Allowed triggers:

```yaml
on:
  workflow_dispatch:
  schedule:
    - cron: "0 0 * * 0"
```

The schedule is optional. If scheduled runs generate noise or consume material resources without maintainers reviewing them, make the workflow manual-only.

The workflow must be clearly non-required and non-publishing. It may run:

- representative feature-profile compilation;
- selected slow/ignored tests;
- all-feature checks only where the feature set is coherent;
- dependency policy/advisory checks;
- coverage generation without making ordinary merges depend on it;
- long Python resource or compatibility checks;
- packaging smoke construction that stops before upload.

Do not run `cargo test --workspace --all-features` blindly if features require unavailable system libraries, privileges, devices, network services, or mutually incompatible configurations. Replace it with named representative profiles from Phase A.

Recommended job shape:

```yaml
jobs:
  deep:
    runs-on: ubuntu-latest
    steps:
      # setup once
      # make check-full
```

Use multiple jobs only where operating-system or system-dependency isolation is genuinely required.

## Workstream 3 — Consolidate dependency and security tooling

Review these current categories:

- `cargo audit`;
- `cargo deny check advisories`;
- `cargo deny check licenses`;
- `cargo deny check bans`;
- GitHub Dependency Review;
- Gitleaks;
- GitHub-native secret scanning settings where available.

Recommended disposition:

### Secret detection

Keep one PR-oriented secret detection mechanism. If GitHub-native secret scanning is enabled and adequate, a custom Gitleaks action may be optional. If Gitleaks is retained, keep it as one lightweight job or step and document why it is needed.

Do not require full-history checkout for every push if the retained scanner can inspect the pull-request diff or current changes safely.

### Dependency advisories

Use one primary advisory policy. Prefer `cargo deny check advisories` when `cargo-deny` is already used for licenses/bans, or `cargo audit` if advisory-only simplicity is preferred. Do not install and run both on every ordinary commit.

Run advisory checks on dependency-changing pull requests if path filtering is reliable, or in the optional scheduled workflow. A mutable advisory database should not make an unrelated documentation commit fail unpredictably unless maintainers deliberately accept that policy.

### Licenses and bans

Retain `cargo deny check licenses bans` in the optional diagnostic workflow or on dependency changes. Keep it mandatory only if the repository has an active, reviewed deny policy and failures are actionable.

### Dependency Review

Retain only if supported by repository settings and it adds pull-request diff analysis not supplied by the Cargo tools. Do not preserve it solely because the action exists.

The final plan implementation must state one owner for each retained defect class: secret introduction, known advisory, disallowed license, and banned/duplicate dependency.

## Workstream 4 — Relocate `.gitlab-ci.yml`

The current `.gitlab-ci.yml` is a set of consumer examples that downloads the latest GitHub release and runs Eggsec against `$TARGET`. It is not a GitLab pipeline that builds or tests this repository.

Preferred action:

1. move its useful patterns into documentation or example templates, for example:

```text
examples/ci/gitlab/eggsec-scan.yml
```

2. delete root `.gitlab-ci.yml` so GitLab mirrors do not interpret consumer scanning as project CI;
3. document that consumers must pin an explicit Eggsec version, configure authorization/scope, and choose targets themselves.

If the root file must remain for a known mirror, rewrite it to validate the current source commit using the same compact contract. Do not retain download-and-scan behavior at the root.

## Workstream 5 — Optional full-check command

Implement or finalize:

```bash
make check-full
```

It should aggregate the optional diagnostics selected in Phase A. It may include:

```bash
make check
make check-python
make check-feature-profiles
cargo deny check
# selected slow/resource/compatibility tests
```

Requirements:

- no publication;
- no credentials;
- no arbitrary external targets;
- no unbounded stress or load tests;
- no automatic generation of release evidence bundles;
- clear prerequisites for system-dependent profiles;
- failures identify the failing optional category.

Do not make `check-full` the new de facto mandatory command in `AGENTS.md`.

## Implementation steps

1. Confirm Phases B and C replacement jobs are passing.
2. Delete `security-scan.yml` and relocate any useful consumer example.
3. Classify every command in `deep-checks.yml`; retain only coherent optional diagnostics.
4. Consolidate advisory/license/secret tooling according to Phase A ownership.
5. Move or rewrite `.gitlab-ci.yml` as a consumer example.
6. Implement `make check-full` without release/evidence side effects.
7. Remove stale badges, documentation links, and required-status references.
8. Verify no scheduled workflow scans external targets.
9. Run the optional workflow manually once, if permissions allow, and record the result.
10. Ensure the optional workflow is not a required branch-protection status.

## Validation commands

Repository searches:

```bash
rg -n "example\.com|github\.event\.inputs\.target|TARGETS_FILE|RUN_LOAD_TEST|SCAN_TYPE" .github/workflows .gitlab-ci.yml
rg -n "releases/(latest|download)|eggsec-linux-amd64" .github/workflows .gitlab-ci.yml
rg -n "cargo audit|cargo deny|gitleaks|dependency-review" .github/workflows
rg -n "schedule:" .github/workflows
```

Run optional local validation:

```bash
make check-full
```

Inspect workflow inventory:

```bash
find .github/workflows -maxdepth 1 -type f -print | sort
```

Expected after Phase E eventually completes: one mandatory workflow and at most one optional workflow. During Phase D, release workflows may still exist pending Phase E, but no self-scan workflow should remain.

## Acceptance criteria

- `.github/workflows/security-scan.yml` is deleted.
- No project workflow scans `example.com` or accepts an arbitrary scan target.
- No project workflow downloads a previously released Eggsec binary as validation of the current commit.
- `.gitlab-ci.yml` no longer acts as consumer scan automation at repository root.
- At most one optional diagnostic workflow remains outside release workflows pending Phase E.
- The optional workflow is manual or scheduled, non-required, and non-publishing.
- `make check-full` is documented as optional.
- Advisory checking has one primary tool/owner.
- Secret detection has one primary mechanism/owner.
- License and ban checks run at a proportionate cadence.
- Optional all-feature validation uses coherent representative profiles rather than an invalid Cartesian assumption.
- No external network, privilege, stress, or load operation runs automatically from project CI.
- Removed workflow names have no stale badges or required-status references.

## Explicit non-goals

- Removing security tests from Eggsec itself.
- Preventing consumers from integrating Eggsec into their own CI.
- Creating a hosted scanning service.
- Adding CodeQL or another broad security platform as compensation for deletion.
- Making optional diagnostics mandatory through documentation language.
- Fixing every optional feature in this phase.

## Rollback strategy

If maintainers need a consumer scan example, restore it under `examples/` or documentation with explicit version pinning and authorization guidance; do not restore it as the repository's own triggered workflow. If one dependency tool proves insufficient, add a second tool only with a documented distinct defect class and cadence.

## Handoff notes

The implementation report must list the final workflow inventory, scheduled triggers, and security-tool ownership. It must explicitly confirm that no automated job invokes Eggsec against a non-loopback target.
