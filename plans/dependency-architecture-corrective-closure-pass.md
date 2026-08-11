# Corrective Closure Plan: Dependency, Architecture, and Verification Simplification

## Status

Ready for implementation.

This is a narrow corrective pass after the A–J dependency, architecture, and
verification simplification roadmap. It exists because the implementation
substantially achieved the roadmap goals, but the final closure record currently
contains several contradictions and accepts active dependency-security work as
closed.

The pass must close those remaining items without reopening broad architecture
work, adding new capabilities, or rebuilding the verification apparatus that the
roadmap intentionally simplified.

## Baseline

Plan against `main` after the Phase J documentation cleanup, beginning from:

```text
e29cdfacb2920c55a139c7b956c2a7491a3f9fc5
```

If implementation starts from a later commit, record the actual starting SHA in
the implementation summary and verify that the conditions described here still
exist before changing them.

Current confirmed residuals:

1. the closure report declares the roadmap complete while PyO3 and quick-xml
   remain on lines with active RustSec advisories;
2. the closure report says those dependencies should be reopened on a security
   advisory even though the listed advisories already satisfy that trigger;
3. routine `ci.yml` still runs stable Rust, exact MSRV, macOS, Windows, and
   Python jobs on pull requests, while the closure report describes portability
   as optional;
4. headless and daemon-client artifact sizes are recorded as estimates despite
   successful release builds and an artifact measurement helper existing;
5. Phase F's remaining CLI parsing location is functionally acceptable but is
   described inconsistently as both partially executed and closed;
6. active architecture documentation still lists obsolete scope helpers and a
   legacy command-registry fallback as remaining work without a final remove-or-
   retain disposition.

## Objective

Produce one honest, reproducible closure state in which:

- active dependency advisories are either removed by supported upgrades or
  retained only under explicit, time-bounded exceptions with a concrete blocker;
- CI reflects the repository's intended lightweight iteration contract;
- supported artifact profiles have measured, not estimated, sizes;
- Phase F and other accepted residual boundaries are described accurately;
- obsolete compatibility helpers are removed when trivial and safe, otherwise
  explicitly retained with rationale;
- the closure report and active architecture documentation describe the code and
  workflows that actually exist;
- package publication remains entirely manual.

## Non-goals

This corrective pass does **not** authorize:

- feature expansion;
- removal of user-visible capability to obtain a smaller binary;
- a broad crate decomposition or domain-extraction project;
- rewriting the command dispatch architecture solely to eliminate a legacy
  label;
- replacing SQLite because it is implemented in C;
- replacing libssh2/OpenSSL/native platform APIs where protocol compatibility or
  platform functionality would regress;
- introducing Dependabot, Renovate, a new release bot, or automated publication;
- adding a new CI matrix framework, custom change-classification framework, or
  generated verification evidence;
- imposing hard binary-size regression gates;
- making every optional feature compile on every pull request;
- changing security policy semantics from Phases A–D except where required to
  preserve compatibility during a dependency upgrade.

## Ordering

Execute the work in this order:

```text
A. dependency-security disposition
B. CI contract correction
C. artifact measurement correction
D. residual architecture cleanup/disposition
E. closure/documentation reconciliation
F. final focused validation
```

Do not mark the corrective pass complete before Workstream A is resolved. The
remaining documentation and measurement work must not be used to hide or defer a
security dependency decision.

---

## Workstream A — Resolve active dependency-security exceptions

### A1. PyO3 0.22 migration

The existing closure record lists both:

```text
RUSTSEC-2025-0020
RUSTSEC-2026-0177
```

against the PyO3 line used by `eggsec-python`. The existing reopen trigger says a
security advisory should reopen the migration; that trigger is already true.

Treat the PyO3 migration as corrective security work, not optional modernization.

Primary files:

```text
crates/eggsec-python/Cargo.toml
crates/eggsec-python/src/**/*.rs
Cargo.lock
deny.toml
docs/DEPENDENCY_EXCEPTIONS.md
docs/BUILD.md
docs/VERIFICATION.md
docs/python/**
```

Implementation sequence:

1. inventory all direct PyO3 APIs currently used by `eggsec-python`;
2. select the newest supported PyO3 line that removes both listed advisories and
   remains compatible with the repository's chosen MSRV after A2;
3. update the binding crate in one migration rather than maintaining parallel
   old/new compatibility layers;
4. make mechanical API changes required by the newer PyO3 ownership, bound,
   conversion, module, exception, and GIL APIs;
5. preserve the existing public Python API names, request/result schemas,
   exceptions, sync/async semantics, and operation registry behavior;
6. build the actual maturin wheel and run the canonical Python verification;
7. remove PyO3 advisory ignores/exceptions once the old vulnerable version is no
   longer reachable from supported artifacts.

Do not introduce a local PyO3 abstraction framework merely to conceal upstream
API differences.

Required checks:

```bash
cargo tree -p eggsec-python -i pyo3
cargo check -p eggsec-python
make check-python
cd crates/eggsec-python && maturin build --release
cargo deny check advisories
```

The resulting supported Python artifact must not depend on PyO3 0.22.

### A2. quick-xml migration

The existing closure record lists:

```text
RUSTSEC-2026-0194
RUSTSEC-2026-0195
```

against quick-xml 0.31.

Primary files include all manifests and call sites that directly select or use
quick-xml, especially:

```text
crates/eggsec/Cargo.toml
crates/eggsec-output/Cargo.toml
crates/eggsec-mobile-lab/Cargo.toml
crates/**/src/**/*.rs
Cargo.lock
deny.toml
docs/DEPENDENCY_EXCEPTIONS.md
```

Implementation sequence:

1. use `cargo tree -i quick-xml@0.31.0` or the actual locked version to enumerate
   every path keeping the advisory-bearing generation reachable;
2. determine the minimum maintained quick-xml release that fixes both listed
   advisories;
3. update direct consumers together where practical so the old generation is
   actually removed rather than duplicated;
4. adapt reader/writer/deserialization APIs with focused behavior tests for the
   XML inputs Eggsec actually processes;
5. confirm namespace handling, bounded parsing, mobile metadata parsing, report
   serialization/deserialization, and malformed-input behavior remain correct;
6. remove the advisory exceptions after the vulnerable generation disappears.

### A3. MSRV decision

Do not preserve Rust 1.85 merely to avoid a one-minor MSRV increase while keeping
a dependency with active advisories.

After selecting the PyO3 and quick-xml target versions:

1. determine their actual minimum supported Rust versions from Cargo resolution
   and direct compilation;
2. set workspace `rust-version` to the lowest version that supports the chosen
   secure dependency set and all current workspace crates;
3. update `rust-toolchain.toml`, CI/deep-check documentation, and contributor
   documentation consistently;
4. run the exact declared MSRV once in the final validation/deep-check boundary.

A small MSRV increase is acceptable. A large increase must be justified by a
specific dependency requirement in the implementation summary.

### A4. Other advisory exceptions

Review the remaining exceptions in `docs/DEPENDENCY_EXCEPTIONS.md` during the
same pass, but do not turn this into a broad dependency-upgrade sweep.

For each retained exception require:

```text
advisory ID
dependency path
reachable feature/artifact
whether affected API is used
exploitability/relevance assessment
owner
review-by date
upgrade/unblock condition
```

Remove stale exceptions immediately when the dependency is no longer present or
the advisory no longer applies.

### A5. Rusqlite remains separately justified

Do not force a rusqlite/SQLx migration in this corrective pass unless the
existing blocker has disappeared by implementation time.

Re-check the current resolver conflict. If it still exists, retain the daemon-
only dependency with a concise blocker note. Do not raise the entire workspace
MSRV to a substantially newer compiler solely for this optional backend unless a
security issue requires it.

---

## Workstream B — Make routine CI match the lightweight contract

Current `.github/workflows/ci.yml` runs four routine job classes:

```text
Rust/Linux
exact MSRV/Linux
portability/macOS+Windows
Python/Linux
```

The roadmap goal was a small merge-time contract with broad compatibility checks
outside the critical iteration loop.

### B1. Routine PR/push contract

The preferred end state is:

```text
ci.yml
  rust / ubuntu-latest / make check
  python / ubuntu-latest / make check-python
```

Keep Python in routine CI unless implementation can skip it using a simple native
GitHub path filter without adding a third-party change-detection action or custom
script. Python is a first-class supported API, so avoiding a small amount of CI
is not worth adding workflow machinery.

The Rust job remains the mandatory general merge-time signal.

### B2. Move exact MSRV out of the routine loop

Move exact-MSRV validation to the existing `deep-checks.yml` weekly/manual
workflow, or another already-existing non-merge-critical compatibility boundary.
Do not create a dedicated MSRV workflow solely for this purpose.

The deep workflow should run the declared MSRV command explicitly, for example:

```bash
cargo +<declared-msrv> check --workspace --no-default-features
cargo +<declared-msrv> check -p eggsec-cli --no-default-features
```

The stable Rust routine job already protects normal development compilation.

### B3. Move macOS/Windows portability out of the routine loop

Portability remains supported, but compile checks do not need to block every
ordinary PR.

Preferred implementation:

- add macOS and Windows compile checks to a manual/scheduled compatibility path;
- reuse `deep-checks.yml` if doing so stays readable;
- do not add release publication, artifact upload, packaging, or cross-platform
  test matrices;
- use `cargo check` for representative CLI/engine targets only.

If the existing deep-check workflow cannot express cross-platform jobs cleanly,
a single small `compatibility.yml` workflow is acceptable, but it must contain
only scheduled/manual MSRV and portability checks. Do not fragment verification
into multiple feature-specific workflows.

### B4. Preserve manual releases

Verify all workflow files remain free of:

```text
cargo publish
maturin publish
twine upload
gh release
id-token: write
tag-triggered publishing
```

Do not add registry credentials or release permissions.

### B5. Simplify documentation with the workflow

Update `docs/VERIFICATION.md`, `CONTRIBUTING.md`, `AGENTS.md`, and any active CI
documentation so the merge-time and scheduled/manual contracts are described in
one place and linked elsewhere.

Acceptance is based on actual workflow definitions, not labels such as
"optional" in Markdown.

---

## Workstream C — Replace estimated artifact data with reproducible measurements

The closure report currently records estimated sizes for:

```text
headless CLI
headless + daemon-client CLI
```

Both profiles were reportedly built successfully, so estimates are unnecessary.

Primary files:

```text
scripts/artifact-sizes.sh
plans/dependency-architecture-simplification-closure-report.md
docs/BUILD.md
```

### C1. Define exact profiles

Measure these primary profiles at minimum:

```bash
cargo build -p eggsec-cli --release
cargo build -p eggsec-cli --release --no-default-features
cargo build -p eggsec-cli --release --no-default-features --features daemon-client
cargo build -p eggsec-daemon --release
cd crates/eggsec-python && maturin build --release
```

Record exactly which binary/wheel each command produces. If multiple commands
write the same executable path, measure immediately after each build or copy the
artifact into a temporary ignored measurement directory before the next build.

### C2. Make the helper useful but small

Update `scripts/artifact-sizes.sh` only if needed to make the measurement
repeatable. It may:

- build named supported profiles;
- print byte and MiB sizes;
- record the command/features used;
- optionally print `cargo tree` summary counts.

It must not:

- create committed generated evidence;
- add binary-size pass/fail thresholds;
- upload artifacts;
- depend on nonstandard analysis tooling for the basic result.

### C3. Record measured values

Update the closure report so every primary artifact size is an observed value
from one named host/toolchain/commit.

Remove `~`, `estimated`, or inferred crate-count wording from values presented as
final closure evidence.

If a profile cannot be built in the implementation environment, record
`BLOCKED`/`NOT MEASURED`; do not invent a number.

---

## Workstream D — Resolve small residual architecture items without scope creep

### D1. Phase F CLI boundary

The engine still contains CLI modules behind the optional `cli` feature. This is
acceptable when the dependency boundary is effective and moving all command
implementation would cause churn without measurable benefit.

Choose and document one of these dispositions:

```text
A. remove the remaining CLI parsing from eggsec because the move is now small and
   mechanical;
B. retain the feature-gated module as an intentional compatibility boundary.
```

Default to B unless dependency inspection shows Clap/application dependencies are
still leaking into headless/Python consumers or the remaining move is genuinely
small.

For disposition B, active docs must say:

- CLI parsing code physically remains in `eggsec`;
- it is excluded from headless/library consumers by the `cli` feature;
- process-host concerns already moved to `eggsec-cli` where valuable;
- physical source location is accepted because the dependency graph, not crate
  aesthetics, is the optimization target.

Do not leave the phase labelled ambiguously as both partial and complete.

### D2. Legacy scope helpers

Audit:

```text
utils::check_scope()
utils::check_scope_from_url()
```

If there are no production callers and removal is mechanical:

- delete the helpers and tests that exist solely for them;
- update docs to point only to `EnforcementContext` scope evaluation.

If a supported external/public API still uses them:

- retain them with deprecation/documentation;
- record the exact consumer preventing removal.

Do not maintain dead helpers merely because historical documentation mentions
them.

### D3. Command-registry legacy fallback

Audit the current `LegacyWrapped`/legacy command registration path and determine
whether it is still carrying real commands.

If only a few trivial registrations remain and migration uses the already-
existing `OperationMetadata` model, migrate them.

Otherwise retain the fallback and document it as an intentional dispatch adapter,
not an incomplete security boundary. The corrective pass does not justify a
wholesale command-handler rewrite.

Required property regardless of disposition:

- policy/risk/mode/target/features/exposure remain owned by canonical operation
  metadata;
- legacy command routing must not reintroduce independent authorization metadata.

---

## Workstream E — Reconcile the closure record and active documentation

The closure report must not claim a condition that is contradicted by the same
file or by current workflows.

Primary files:

```text
plans/dependency-architecture-simplification-roadmap.md
plans/dependency-architecture-simplification-closure-report.md
plans/dependency-architecture-phase-e-advisory-dependency-remediation.md
plans/dependency-architecture-phase-f-engine-application-boundary.md
plans/dependency-architecture-phase-h-upstream-msrv-native-deps.md
plans/dependency-architecture-phase-i-ci-verification-simplification.md
plans/dependency-architecture-phase-j-measurement-and-closure.md
plans/README.md
docs/ARCHITECTURE.md
docs/ARCHITECTURE_INVARIANTS.md
docs/BUILD.md
docs/VERIFICATION.md
docs/DEPENDENCY_EXCEPTIONS.md
AGENTS.md
CONTRIBUTING.md
```

### E1. While this plan is open

Do not rewrite historical implementation outcomes as failures. Instead mark the
roadmap/closure state along these lines:

```text
A–J implementation substantially complete; corrective closure pass open.
```

The original phase history remains useful.

### E2. Correct advisory language

The closure report must not state that a dependency will be reopened on a
security advisory when an active advisory is already listed against it.

After Workstream A:

- removed advisories disappear from the live exception table;
- retained advisories have explicit blocker and review date;
- the closure status reflects actual residual security debt.

### E3. Correct CI language

After Workstream B, document exact workflow behavior:

```text
merge-time jobs
scheduled/manual compatibility jobs
scheduled/manual deep security/feature checks
manual release boundary
```

Do not call a PR-blocking job optional.

### E4. Correct artifact language

Replace estimated artifact values with observed measurements from Workstream C.
Include the exact commit and host/toolchain for the measurement set.

### E5. Correct Phase F language

Use one stable phrase for the accepted source-layout residual. Recommended:

```text
Executed with accepted residual: CLI parsing remains physically in the engine
crate behind the optional cli feature; headless and Python dependency graphs do
not include CLI-only dependencies.
```

### E6. Do not fabricate hosted CI evidence

Local command results may be recorded as local results. Hosted GitHub Actions
results may only be recorded when an actual run/check is inspected.

Absence of a hosted result is `NOT VERIFIED`, not `PASS`.

---

## Workstream F — Final focused validation

Run final validation against one exact clean commit after all corrective changes
are staged/committed locally or in the implementation environment.

### Required core validation

```bash
git rev-parse HEAD
git status --porcelain
cargo fmt --all -- --check
make check
make check-python
cargo deny check advisories
cargo +<declared-msrv> check --workspace --no-default-features
```

Run `make check-full` once before final closure if its required tools are
available. It remains a broad diagnostic, not a merge-time requirement.

### Required dependency validation

```bash
cargo tree -p eggsec-python -i pyo3
cargo tree -d
cargo tree -i quick-xml@<old-version>
```

The old advisory-bearing PyO3 and quick-xml generations must not be reachable
from supported artifacts unless a retained exception explicitly documents a real
blocker.

### Required profile builds

```bash
cargo build -p eggsec-cli --release
cargo build -p eggsec-cli --release --no-default-features
cargo build -p eggsec-cli --release --no-default-features --features daemon-client
cargo build -p eggsec-daemon --release
cd crates/eggsec-python && maturin build --release
```

### Required workflow inspection

```bash
rg -n 'cargo publish|maturin publish|twine upload|gh release|id-token: write|tags:' .github/workflows
```

Inspect the workflow definitions directly and confirm routine PR jobs match the
final documented contract.

### Result vocabulary

Use only:

```text
PASS
FAIL
BLOCKED
NOT RUN
NOT VERIFIED
```

Do not convert warnings, skipped commands, estimates, absent hosted runs, or
failed optional checks into PASS.

---

## Rollback and failure handling

### Dependency migration rollback

If a PyO3 or quick-xml upgrade causes an API/behavior regression that cannot be
resolved within a focused migration:

1. do not silently restore the old version and close the roadmap;
2. capture the minimal concrete blocker;
3. retain the advisory exception with a near-term review date;
4. keep this corrective plan open or mark the specific item BLOCKED;
5. avoid introducing a large compatibility abstraction merely to declare
   success.

### CI rollback

If moving compatibility/MSRV checks prevents scheduled/manual coverage, restore
the previous jobs temporarily and fix the non-blocking workflow before closure.
Do not solve the problem by making every job merge-critical again.

### Artifact measurement rollback

Measurement scripts are diagnostic only. If the helper becomes complicated,
remove the automation and record direct `stat`/file-size results from the explicit
build commands instead.

### Architecture cleanup rollback

If removing a legacy helper/fallback breaks a supported API, restore it and mark
the residual intentional. No broad refactor is required for this corrective
pass.

---

## Explicit acceptance criteria

The corrective pass is complete only when all applicable criteria below are
satisfied.

### Dependency security

1. PyO3 0.22 is no longer reachable from the supported `eggsec-python` artifact,
   **or** the migration has a concrete technical blocker documented with a
   time-bounded exception and the corrective pass is not described as fully
   security-closed.
2. `RUSTSEC-2025-0020` and `RUSTSEC-2026-0177` are removed from live exceptions
   when the vulnerable PyO3 line disappears.
3. quick-xml 0.31/advisory-bearing generation is no longer reachable from the
   supported artifacts that previously selected it, **or** a concrete technical
   blocker is documented with a time-bounded exception.
4. `RUSTSEC-2026-0194` and `RUSTSEC-2026-0195` are removed from live exceptions
   when the vulnerable quick-xml generation disappears.
5. every retained advisory exception has path, feature/artifact, affected API
   assessment, owner, review-by date, and unblock condition.
6. `cargo deny check advisories` passes without stale or unexplained ignores.
7. workspace MSRV reflects the secure dependency set rather than pinning a
   vulnerable dependency solely to preserve Rust 1.85.
8. the exact declared MSRV successfully compiles the documented baseline.

### CI and release

9. routine PR CI no longer runs macOS and Windows portability checks.
10. exact-MSRV verification is scheduled/manual rather than merge-critical.
11. routine CI retains one canonical Linux Rust check using `make check`.
12. Python verification remains present without adding a custom path-detection
    framework; any skipping mechanism is native and simpler than running the job.
13. broad feature/security checks remain weekly/manual through the existing deep
    validation boundary.
14. active documentation exactly matches the workflow definitions.
15. no workflow publishes crates, Python packages, GitHub Releases, or uses
    release-oriented OIDC permissions.
16. release cadence and publication remain manual maintainer actions.

### Artifact/dependency topology

17. default CLI/TUI, headless CLI, daemon-client CLI, daemon server, and Python
    wheel have actual measured release sizes or are explicitly marked
    `BLOCKED`/`NOT MEASURED`.
18. final closure tables contain no estimated values presented as measurements.
19. measurements identify commit, host/architecture, toolchain, profile/features,
    artifact, and size.
20. headless/Python consumers remain free of CLI-only dependencies by default.
21. TUI/client consumers remain free of daemon SQLite persistence dependencies by
    default.
22. no new native dependency is introduced without feature/artifact ownership and
    justification.

### Architecture residuals

23. Phase F has one unambiguous final disposition.
24. CLI parsing either moves cleanly to the frontend crate or remains explicitly
    accepted behind the `cli` feature; no cosmetic crate move is required solely
    to satisfy the plan.
25. legacy scope helpers are removed when unused, otherwise their supported
    consumer and deprecation/retention rationale are documented.
26. the command-registry fallback is either trivially migrated or explicitly
    retained without duplicating policy metadata.
27. canonical operation metadata continues to own risk, mode, target policy,
    required features/capabilities, aliases, and automated-surface exposure.
28. Phases A–D security/enforcement behavior does not regress during dependency or
    cleanup work.

### Closure record

29. the roadmap/closure report does not say "all work complete" while an active
    corrective security item is simultaneously listed as deferred without a
    blocker.
30. advisory reopen triggers do not contradict current advisory state.
31. CI optional/mandatory descriptions match actual workflow triggers/jobs.
32. hosted CI results are only claimed when directly verified; otherwise they are
    labelled `NOT VERIFIED`.
33. final validation commands and results are recorded against one exact clean
    commit.
34. historical A–J plan files remain retained as engineering history.
35. this corrective plan receives an execution summary and final status when the
    work is actually complete.
36. no package or release is published as part of this corrective pass.

## Handoff completion record

When implementation is complete, append a concise section here containing:

```text
final status
implementation commit(s)
PyO3 disposition
quick-xml disposition
final MSRV
routine CI jobs
scheduled/manual jobs
measured artifact table reference
retained advisory exceptions
retained architecture residuals
validation command outcomes
publication status
```

Do not create another closure framework or multiple evidence files. Update the
existing closure report and this plan in place.
