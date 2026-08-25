# Phase I Plan: CI and Verification Simplification

## Status

Status: Executed.

Executed. Changes landed in main.

Phase I is fully implemented. `make check` uses package-level Cargo commands
(not hand-curated test lists). Mandatory CI is Linux-first (Rust, MSRV, Python).
Portability (macOS/Windows) is a separate optional job. `make check-full` is the
optional broad validation workflow (weekly schedule or manual trigger). No
publication or tag-triggered release in any CI workflow.

## Objective

Reduce mandatory verification to a compact, direct, reproducible contract that
protects Eggsec's actual behavioral and architectural invariants without
recreating the previously removed CI bureaucracy.

This phase must simplify around the architecture produced by earlier phases. It
must not preserve obsolete registry snapshots or grep guards simply because they
were once mandatory.

## Current baseline

The repository currently has:

- one mandatory workflow with Rust, Python, and macOS/Windows portability jobs;
- one scheduled/manual deep-check workflow;
- `make check` with a hand-curated sequence of individual tests;
- `make check-python` with one extension build and retained checks;
- `make check-full` with dependency policy and representative feature profiles;
- architecture shell guards requiring ripgrep;
- manual, non-CI release publication.

This baseline is substantially better than the historical workflow graph and is
not to be replaced with another complex orchestration layer.

## Preconditions

- semantic authorization tests from Phases A and B exist;
- feature and operation metadata are single-source or derived from Phases C and
  D;
- advisory exception hygiene from Phase E is established;
- final artifact/crate boundaries from Phases F–H are stable;
- the real MSRV is known.

## Scope

Primary files:

```text
.github/workflows/ci.yml
.github/workflows/deep-checks.yml
Makefile
scripts/check-architecture-guards.sh
scripts/check-python.sh
docs/VERIFICATION.md
docs/CI_ARCHITECTURE_GUARDS.md
AGENTS.md
CONTRIBUTING.md
crates/eggsec/tests/
crates/eggsec-python/tests/
```

The phase may delete obsolete test files or scripts only when their defect class
is directly covered by retained tests/types or their underlying representation
was removed.

## Non-goals

This phase does not:

- reduce behavioral coverage solely to make CI faster;
- restore release workflows, artifact evidence DAGs, skip budgets, maturity
  gates, or synthetic aggregate statuses;
- add a new CI provider;
- add self-hosted runners;
- add privileged/network tests to hosted CI;
- make `full` builds mandatory;
- require cargo-nextest;
- publish packages or releases;
- optimize exact job count at the expense of clarity.

## Verification policy

A check belongs in mandatory CI only when it:

1. catches a distinct merge-time defect class;
2. is deterministic on hosted runners;
3. does not require credentials, privileged hardware, external mutable services,
   or public targets;
4. is not already covered by another retained command;
5. produces a direct actionable failure;
6. has cost proportional to the frequency and severity of the defect.

Checks that do not meet this threshold belong in:

- change-aware optional jobs;
- scheduled/manual deep checks;
- maintainer release validation;
- local specialist commands;
- removal, when they validate deleted representations or process artifacts.

## Workstream 1 — Replace the hand-curated Rust test list

The current `make check` names individual integration tests. This creates two
problems:

- new tests are not automatically included;
- deleted/renamed tests require manual Makefile maintenance.

Replace the list with the smallest package-level commands that include all
mandatory tests. Candidate contract:

```makefile
check:
	cargo fmt --all -- --check
	cargo check --workspace --no-default-features
	cargo clippy -p eggsec --all-targets --features rest-api -- -D warnings
	cargo test -p eggsec --features rest-api --tests --no-fail-fast
	cargo test -p eggsec-output --tests
```

This is illustrative, not mandatory syntax. Final selection must account for:

- feature-gated tests that should be mandatory;
- CLI/runtime/daemon boundary tests introduced in earlier phases;
- duplicate execution of library tests;
- platform-specific exclusions;
- compile cost.

Prefer a small number of package/feature commands over many named targets.

## Workstream 2 — Define the mandatory Rust profile

Mandatory Linux Rust CI should cover:

- format;
- no-default workspace compile;
- Clippy for the primary engine/adapter profile;
- all tests in the primary `eggsec` integration profile;
- direct tests for leaf crates whose behavior is not exercised through `eggsec`;
- exact MSRV compile check, either in the same job or a small separate job.

Do not require all optional backends. Select one coherent primary feature profile
that includes protocol/enforcement paths without requiring system services.

Document why each package outside `eggsec` has a mandatory test command. If no
distinct defect class exists, keep it in optional diagnostics.

## Workstream 3 — Reduce architecture grep guards

Audit every check in `scripts/check-architecture-guards.sh` and classify it:

```text
replace with type/module visibility
replace with Cargo manifest dependency boundary
replace with direct semantic test
retain as narrow static guard
remove because representation is gone
remove because it validates documentation wording only
```

High-value guards may remain for facts not easily expressed otherwise, such as:

- prohibited dependency directions;
- required current architecture documents;
- absence of known generated/orphan directories;
- release workflow publication commands.

Remove guards that police:

- transitional field names;
- duplicate registries removed in Phase D;
- raw dispatch through broad substring searches when private visibility and
  semantic tests now enforce it;
- stale historical wording that has no product impact.

If the remaining checks no longer need ripgrep, remove the mandatory ripgrep
prerequisite. If a few `rg` checks remain, keep the dependency explicit and
small.

## Workstream 4 — Make Python CI change-aware where reliable

Keep `make check-python` as the canonical complete Python verification command.
In GitHub Actions, evaluate path filtering or job conditions so the Python build
runs when changes affect:

```text
crates/eggsec-python/**
Python package/stubs/tests
engine public APIs used by bindings
core DTO/operation metadata
Cargo manifests/lockfile
shared release/build scripts
workflow itself
```

Do not skip Python on broad engine changes merely because the path list is too
narrow. A conservative path filter is acceptable; a brittle generated dependency
map is not.

If change-aware filtering becomes complex or unreliable, retain the one Python
job on every PR rather than adding orchestration.

## Workstream 5 — Move portability checks to optional/change-aware execution

The current macOS and Windows compile matrix runs on every push/PR. Reclassify it
based on observed defect frequency and project scope.

Preferred policy:

- scheduled/manual portability checks for the standard CLI/TUI and headless
  profiles;
- run on PRs that modify platform-specific code, TUI/clipboard, sockets, path
  handling, manifests, or workflows;
- Linux remains the comprehensive mandatory environment;
- portability failures remain important but need not block unrelated docs or
  pure Linux/domain changes.

If branch protection requires static status names and cannot support optional
conditions cleanly, retain one compact portability job rather than introducing a
synthetic gate.

## Workstream 6 — Keep dependency policy and broad profiles optional

`cargo deny`, duplicate dependency review, optional backend compile checks,
coverage, benchmarks, slow tests, and broad feature profiles belong in
`deep-checks.yml` or local commands.

The deep workflow should remain:

- manual and/or scheduled;
- non-publishing;
- one logical workflow;
- small enough to understand;
- explicit about optional failures versus release blockers.

It may run:

```text
cargo deny check
representative feature profiles
exact MSRV and stable checks
portability profiles if not elsewhere
slow/ignored tests
artifact/dependency size report
```

Do not turn it into an exhaustive matrix.

## Workstream 7 — Simplify Make targets

Retain a clear command surface:

```text
make test          fast/default developer loop
make check         mandatory Rust/Linux merge contract
make check-python  complete Python binding contract
make check-full    optional broad diagnostics
make release-check manual local release validation; never publishes
```

Remove or alias duplicate targets such as identical `test-ci` and
`test-integration` definitions. Keep specialist targets only when they are used
and documented.

`make help` should distinguish primary commands from specialist diagnostics.
Avoid wrappers that merely rename one Cargo command without adding stable value.

## Workstream 8 — Preserve manual release boundaries

Verify no workflow contains:

```text
cargo publish
maturin publish
twine upload
gh release
package-index credentials
id-token write for publication
tag-triggered release mutation
```

`make release-check` remains local and stops before publication. This phase must
not revalidate or redesign the detailed release-package graph unless an earlier
phase changed manifests in a way that requires documentation alignment.

## Workstream 9 — Update verification ownership documentation

Rewrite `docs/VERIFICATION.md` around defect classes rather than historical
phase names. For each mandatory command, state:

- defect class;
- owning test/type/manifest boundary;
- why it is merge-time;
- optional deeper check, if any.

Update `docs/CI_ARCHITECTURE_GUARDS.md` to list only retained guards. Remove stale
counts and deleted invariant descriptions.

`AGENTS.md` should show the minimal commands first and link to the canonical
document rather than restating matrices.

## Workstream 10 — Measure workflow cost and signal

Before and after changes, record:

```text
number of mandatory jobs
number of maturin builds per PR
number of Rust compile/test invocations
portability frequency
median/representative runtime where available
number of retained shell architecture checks
number of optional deep-check commands
```

A concise Markdown table in the implementation PR or Phase J closure is enough.
Do not add telemetry or a persistent CI analytics service.

## Validation commands

Before deleting old paths, ensure the replacement command includes their tests.
Final local validation should normally be:

```bash
make test
make check
make check-python
make check-full
```

Also inspect workflows statically:

```bash
rg -n 'cargo publish|maturin publish|twine upload|gh release|id-token: write|tags:' .github/workflows
```

Do not run publication or registry preflight.

## Migration sequence

1. inventory current mandatory commands and distinct defect classes;
2. define package-level `make check` replacement;
3. prove old named tests execute under the replacement;
4. reduce architecture guards after typed/test replacements exist;
5. simplify duplicate Make targets;
6. make Python/portability execution change-aware or optional where simple;
7. simplify deep checks;
8. update docs and branch-protection guidance;
9. record workflow/runtime deltas.

Never delete the old mandatory path before the replacement is present in the
same branch.

## Rollback considerations

If path filtering skips necessary Python or portability checks, remove the
filter and run the compact job universally. Do not build a complex dependency
impact analyzer.

If a package-level Cargo test command is materially slower due to irrelevant
features, split it into at most a few coherent profiles rather than returning to
individual test enumeration.

## Acceptance criteria

1. `make check` uses package/feature-level commands, not a long named-test list.
2. Newly added primary-profile integration tests run automatically.
3. Mandatory Linux CI protects direct authorization, scope, metadata, and crate
   boundary behavior.
4. The exact declared MSRV is compiled in CI or an equivalent required check.
5. Python is built once per required CI profile.
6. Python change filtering, if used, is conservative and understandable.
7. Portability is optional/change-aware unless a documented branch-protection
   constraint requires a compact universal job.
8. `deep-checks.yml` remains the single optional broad workflow.
9. `cargo deny`, broad feature profiles, slow tests, and size reports are not
   fragmented into mandatory jobs.
10. Obsolete registry/documentation grep guards are removed.
11. Retained architecture guards each protect a distinct invariant not already
    enforced by types/tests/Cargo.
12. duplicate Make targets are removed or true aliases.
13. no release publication or tag-triggered mutation exists in CI.
14. `make release-check` remains manual and non-publishing.
15. verification documentation describes current defect ownership without phase
    archaeology.
16. before/after CI invocation and guard counts are recorded.
17. no package or release is published.
