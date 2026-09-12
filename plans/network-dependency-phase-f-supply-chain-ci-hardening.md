# Phase F — Dependency policy and CI supply-chain hardening

Status: Ready for handoff

Date: 2026-09-11

Depends on: Phase A baseline; may execute alongside Phases B-E with measurement coordination

## Purpose

Make dependency and workflow supply-chain policy enforceable on pull requests, reconcile the divergent RustSec exception mechanisms, and reduce mutable CI inputs. This phase changes governance and verification, not EggSec runtime capability.

## Workstream 1 — Make Cargo Deny a normal merge gate

Move `cargo deny check` from scheduled/manual-only validation into the required pull-request contract.

Preferred implementation:

- install `cargo-deny` in the Rust CI job using an immutable-SHA-pinned action or a version-pinned install mechanism;
- run `cargo deny check` on pull requests and pushes affecting Rust dependency/security configuration;
- keep Deep Checks as the broad feature/native-dependency oracle, not the first place advisories/source violations are discovered;
- retain local `make check-full`, but add a dedicated deterministic target such as `make check-deps` if that keeps normal developer usage clear.

Do not make the security gate depend on network-facing integration tests.

## Workstream 2 — Reconcile Cargo Deny and Cargo Audit policy

Current state has two incompatible exception surfaces: `deny.toml` documents only the known current exceptions while `.cargo/audit.toml` suppresses a much larger historical advisory set.

Choose Cargo Deny as the canonical repository dependency-policy source unless implementation discovers a missing requirement it cannot express.

Then do one of:

1. remove `.cargo/audit.toml` and document that `cargo audit` is diagnostic/non-canonical; or
2. reduce `.cargo/audit.toml` to the exact currently approved advisory IDs and add a synchronization guard/test.

Do not retain historical ignores “just in case.” Each accepted advisory exception must have:

- advisory ID;
- dependency path;
- affected artifact/feature;
- exploitability/API-use assessment;
- compensating control;
- owner;
- created date;
- review-by/removal date;
- upstream upgrade blocker/removal criterion.

Reuse `docs/DEPENDENCY_EXCEPTIONS.md` as the narrative owner if it remains canonical.

## Workstream 3 — Harden Cargo source policy

Extend `[sources]` in `deny.toml` to fail closed for unexpected sources:

```toml
[sources]
unknown-registry = "deny"
unknown-git = "deny"
required-git-spec = "rev"
```

Allow crates.io explicitly only if needed for clarity; cargo-deny already treats crates.io as the default allowed registry when no custom list is supplied. Add explicit `allow-git` entries only when an actual approved git dependency exists.

Before enabling the gate:

- inspect `Cargo.lock` and all manifests for git/alternate-registry dependencies;
- convert any floating branch/tag dependency to a crates.io release or full commit `rev` where possible;
- document unavoidable git sources with owner/removal criteria.

Set wildcard dependency policy to `deny` after confirming no intentional wildcard declarations exist. Keep `multiple-versions = "warn"` unless the Phase A graph proves a small actionable set suitable for denial; duplicate-version noise should not obscure real security failures.

## Workstream 4 — Pin GitHub Actions immutably

Replace mutable action refs such as:

```text
actions/checkout@v4
dtolnay/rust-toolchain@stable / @master
Swatinem/rust-cache@v2
actions/setup-python@v5
taiki-e/install-action@cargo-deny
```

with verified full-length commit SHAs. Add a comment beside each SHA containing the human-readable upstream release/tag to preserve maintainability.

Apply this to every workflow, not only `ci.yml` and `deep-checks.yml`.

GitHub's secure-use guidance treats a full-length commit SHA as the immutable action pin. Do not pin to mutable major tags merely because the action is first-party.

## Workstream 5 — Automate pin/dependency maintenance

No `.github/dependabot.yml` is present at the baseline. Add Dependabot (or the repository's chosen equivalent) for at least:

- `cargo` dependencies;
- `github-actions` action pins.

Choose a low-noise cadence consistent with the repository's maintenance model (weekly is reasonable). Group compatible patch/minor updates where that reduces PR churn, but keep security updates visible.

Do not configure automatic merge in this plan. Updates must still pass the full CI/security contract.

## Workstream 6 — Dependency Review where available

Add GitHub's Dependency Review Action for pull requests if supported for this public repository/account context.

Requirements:

- `permissions: contents: read` at workflow/job scope;
- immutable full-SHA pin;
- vulnerability check enabled;
- choose and document severity threshold; default recommendation for a security project is to fail on at least `moderate`, while Cargo Deny/RustSec remains authoritative for Rust advisories;
- align license policy with `deny.toml` rather than maintaining contradictory allowlists;
- do not grant `pull-requests: write` merely to post comments; log/check output is sufficient unless maintainers explicitly want comments.

If Dependency Review cannot run due repository/platform entitlement, record that as a hosted-platform limitation; do not weaken Cargo Deny as compensation.

## Workstream 7 — Workflow least privilege and deterministic toolchain

Audit all workflows for token permissions and add explicit least-privilege permissions. Normal build/test/dependency-review jobs should generally require only:

```yaml
permissions:
  contents: read
```

Grant write permissions only to release/publication jobs that demonstrably need them.

Review toolchain selection:

- normal CI may track the project's chosen stable policy, but document it;
- MSRV remains explicit 1.88;
- tools installed during CI should be version-pinned or action-SHA-pinned rather than implicitly latest when feasible.

Do not conflate Rust compiler update policy with GitHub Action immutability.

## Workstream 8 — Security check ergonomics

Expose clear local commands, for example:

```text
make check-deps      # cargo deny/source/license/advisory checks
make check           # mandatory source/test contract + check-deps, if runtime acceptable
make check-full      # broad feature/native profile validation
```

If installing cargo-deny on every developer machine would make `make check` unexpectedly fail due a missing tool, either provide a deterministic bootstrap/documented prerequisite or keep a separate required CI job while making the local target explicit. Do not silently skip dependency policy when the tool is absent.

## Required verification

```text
cargo deny check
cargo deny check advisories
cargo deny check bans
cargo deny check licenses
cargo deny check sources
cargo audit   # only if retained/documented
make check
make check-full
```

Validate all workflow YAML and verify the hosted pull-request CI includes the new dependency gate. Trigger/observe Deep Checks after pinning actions.

## Acceptance criteria

1. Dependency advisory/license/source policy runs on pull requests, not only weekly/manual checks.
2. Cargo Deny is documented as canonical and Cargo Audit cannot silently suppress a larger advisory set.
3. Unknown registries and git sources fail closed; git dependencies require immutable revs if any remain.
4. Wildcard dependencies are denied unless a narrowly documented exception is technically required.
5. All GitHub Actions are pinned to verified full commit SHAs with readable version comments.
6. Cargo and GitHub Actions dependency updates are automated through Dependabot/equivalent without auto-merge.
7. Workflow token permissions are explicit and least-privilege.
8. Dependency Review runs with read-only permissions where platform support permits it, or the limitation is documented.
9. Existing advisory exceptions remain documented with owners and review/removal criteria.
10. Hosted normal CI and Deep Checks are green after the hardening changes.

## Expected files touched

- `deny.toml`;
- `.cargo/audit.toml` (remove or reconcile);
- `docs/DEPENDENCY_EXCEPTIONS.md`;
- `Makefile`;
- `.github/workflows/*.yml`;
- new `.github/dependabot.yml` or equivalent config;
- optional dependency-review workflow/config;
- maintainer/security documentation;
- this plan completion record.
