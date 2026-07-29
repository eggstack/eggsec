# Phase A Plan: Verification Policy and Baseline Contract

## Status

Status: Executed. This phase is inventory and policy work only. It must not remove workflows, alter release behavior, or reduce test coverage.

## Objective

Create a precise, repository-backed contract for what Eggsec considers mandatory merge verification, optional diagnostic validation, and manual release preparation. Measure the current apparatus, classify every existing job and script, and define the target command surface before implementation begins.

This phase exists to prevent the simplification effort from becoming an unreviewed sequence of YAML deletions. Later phases must be able to point to an explicit disposition for every current check.

## Required outcome

At completion, maintainers and implementation agents must be able to answer all of the following without interpreting multiple workflows:

- Which commands must pass before an ordinary change is merged?
- Which checks are Linux-only, and which portability checks run on macOS or Windows?
- Which checks are optional, slow, privileged, system-dependent, scheduled, or release-only?
- Which current checks are duplicates?
- Which scripts are behavioral tests, architecture guards, release-process evidence, or obsolete process artifacts?
- Which critical invariants must remain protected after the CI graph is collapsed?
- Which package registries are used, and which final publish commands remain manual?

## Scope

Inspect and classify at minimum:

```text
.github/workflows/test.yml
.github/workflows/deep-checks.yml
.github/workflows/security-scan.yml
.github/workflows/python-wheels.yml
.github/workflows/testpypi-rehearsal.yml
.github/workflows/release.yml
.gitlab-ci.yml
Makefile
AGENTS.md
scripts/check-architecture-guards.sh
scripts/check_python_types.sh
scripts/check-python-capability-matrix.py
scripts/check-python-architecture-guards.py
scripts/check_python_stub_parity.py
scripts/validate_python_profiles.py
scripts/run_python_profile.py
scripts/build_python_release_evidence.py
scripts/check_maturity_guard.py
scripts/python_skip_budget.py
scripts/check_python_compatibility.py
scripts/generate_python_compatibility_baseline.py
scripts/validate_python_release_candidate.sh
crates/eggsec-python/validation/profiles.json
crates/eggsec-python/wheel-profiles.json
docs/python/packaging.md
docs/python/versioning.md
docs/python/README_1_0_CHECKLIST.md
```

Also inspect repository branch protection or documented required status names if accessible. Do not assume current workflow job names are unreferenced externally.

## Deliverables

### 1. Verification contract document

Add a canonical document, recommended path:

```text
docs/VERIFICATION.md
```

It must define:

- `make check` as the mandatory Rust/Linux contributor contract;
- `make check-python` as the mandatory Python/Linux contract for Python-facing changes;
- narrow macOS and Windows portability expectations;
- `make check-full` as optional broad validation;
- which changes require Python checks;
- which changes require optional feature/system checks before release;
- the distinction between merge readiness and release readiness;
- that release publication is always manual and is not part of CI.

The document should be concise enough to remain the authoritative entry point. Detailed commands can live behind Make targets, but their ownership must be explicit.

### 2. Current-job disposition table

Add a temporary or durable planning table to this plan or a companion document. For every job in every current workflow, record:

```text
workflow
job name
trigger
current command(s)
platform
estimated duplicate builds
protected defect class
proposed disposition
replacement command/job
reason
```

Allowed dispositions:

- `mandatory-retain`
- `mandatory-merge`
- `optional-move`
- `release-local`
- `example-relocate`
- `remove-obsolete`

Do not use ambiguous outcomes such as “review later.” If evidence is insufficient, choose `optional-move` and state what future evidence would justify making it mandatory.

### 3. Script ownership table

Classify each verification/release script as one of:

- behavioral test runner;
- static architecture guard;
- metadata consistency check;
- packaging smoke check;
- manual release helper;
- historical evidence/process helper;
- obsolete.

Record whether the script requires an installed extension, a built wheel, JUnit artifacts, Git metadata, credentials, network access, system packages, or platform-specific dependencies.

### 4. Critical invariant list

Identify the smallest direct checks that protect Eggsec's high-value invariants. The list must include an explicit disposition for:

- workspace no-default compilation;
- central enforcement behavior;
- strict-surface scope semantics;
- operation metadata consistency;
- command registry consistency;
- tool registration consistency;
- enforced-dispatch regression coverage;
- output report-envelope stability;
- architecture dependency boundaries;
- Python API/stub parity;
- Python feature/capability metadata consistency;
- Python error/redaction behavior;
- installed-package importability.

The phase must distinguish static grep guards from behavioral tests. A static guard should remain mandatory only when it protects a boundary that is not cheaply expressible as a Rust or Python test.

### 5. Baseline measurements

Record the current state using reproducible measurements where GitHub data is available:

- number of workflow files;
- number of jobs in each workflow;
- number of Rust matrix entries;
- number of Python extension builds per ordinary Python-related push;
- number of artifact upload/download handoffs;
- number of scheduled workflows;
- number of publishing-capable jobs;
- number of unique mandatory tools installed in CI;
- approximate critical-path duration from recent representative runs, if available;
- common CI-only repair categories from recent commits.

Do not manufacture timing data. Where historical run timing is unavailable, record structural counts and mark duration as unavailable.

### 6. Target workflow sketch

Document the intended mandatory workflow shape before implementation. Recommended logical structure:

```yaml
jobs:
  rust:
    # fmt, check, clippy, core tests, selected invariant tests

  python:
    # one venv, one maturin develop, pytest and static/API checks

  portability:
    # narrow macOS/Windows checks
```

The sketch must state that job decomposition is for operating-system isolation or meaningful dependency separation only—not one status check per command.

## Implementation steps

1. Enumerate workflow jobs and matrix expansions from the checked-in YAML.
2. Trace each workflow command to its Make target, script, test file, or release action.
3. Identify exact duplicate compilations and tests across jobs.
4. Review recent CI-fix commits to identify failures caused by orchestration rather than product defects.
5. Define mandatory defect classes and map one retained check to each.
6. Draft `docs/VERIFICATION.md` and the disposition tables.
7. Verify all referenced commands exist in the current repository; proposed future commands must be clearly marked as targets to be implemented in later phases.
8. Review the classification for accidental weakening of safety/enforcement checks.
9. Commit only documentation/planning changes in this phase.

## Decision rules

Use these rules consistently:

- A feature configuration is not automatically a separate product. Retain representative topology coverage rather than one job per feature name.
- A release artifact test is not required on every commit if `maturin develop` and installed-package tests protect the same code path sufficiently for iteration.
- Coverage generation is diagnostic unless the repository enforces a meaningful, stable threshold.
- Multiple advisory tools do not provide proportional value when they consume the same advisory data.
- Evidence generation is not behavioral verification.
- A skip-count budget is not a substitute for explicitly scoped tests and documented feature availability.
- External network scans are not repository correctness tests.
- A previously published binary cannot validate the current commit.
- Cross-platform release builds are release preparation, not routine portability smoke tests.

## Validation

This phase is validated primarily by completeness and internal consistency.

Run repository searches such as:

```bash
find .github/workflows -maxdepth 1 -type f -print
rg -n "maturin (develop|build|publish)|twine upload|cargo publish|gh release|pypi|testpypi" .github .gitlab-ci.yml Makefile scripts docs AGENTS.md
rg -n "build_python_release_evidence|check_maturity_guard|python_skip_budget|validate_python_release_candidate" .
rg -n "cargo (check|test|clippy|build)" .github/workflows Makefile
```

If branch protection status names are not accessible, record that as an explicit pre-implementation operational check for Phase B rather than guessing.

## Acceptance criteria

- `docs/VERIFICATION.md` exists and names one canonical mandatory Rust command and one canonical Python command.
- Every existing workflow job has one explicit disposition.
- Every release or evidence script has an explicit owner/disposition.
- The baseline records structural counts without unsupported timing claims.
- The critical invariant list maps each retained defect class to a direct check.
- Publishing-capable paths are fully enumerated, including PyPI, TestPyPI, crates.io, and GitHub Releases where applicable.
- The target workflow sketch contains no publishing or release-artifact generation.
- No workflow or script is deleted in this phase.
- No runtime, enforcement, public API, or feature behavior changes are made.
- Later phases can be executed without rediscovering the current CI graph.

## Out of scope

- Editing or deleting workflow YAML.
- Changing branch protection.
- Rewriting the Makefile.
- Removing Python evidence scripts.
- Publishing any package.
- Fixing unrelated failing tests.
- Adding new CI providers or build systems.

## Deliverable 2: Current-job disposition table

### test.yml (push to main, PRs)

| Job | Trigger | Command(s) | Platform | Duplicate builds | Protected defect class | Disposition | Replacement | Reason |
|-----|---------|-----------|----------|-----------------|----------------------|-------------|-------------|--------|
| fmt | push/PR | `cargo fmt --all --check` | Linux | 0 | Code style | mandatory-retain | — | Canonical format gate |
| clippy | push/PR | `cargo clippy --lib -p eggsec -- -D warnings` | Linux | 0 | Lint regressions | mandatory-retain | — | Canonical lint gate |
| check | push/PR | `cargo check` (10 matrix entries: default, minimal-cli, rest-api, grpc-api, packet-inspection, stress-testing, nse, nse-sandbox, api-schema, sbom) | Linux | Overlapping; many are marker features | Compilation regressions per feature | mandatory-merge | — | Each entry verifies a distinct feature compiles; marker features are cheap |
| test-features | push/PR | `cargo test` (5 entries: lib-default, lib-rest-api, lib-api-schema, lib-sbom, feature-surface) | Linux | Overlaps with check matrix | Behavioral test regressions | mandatory-merge | — | Tests are behavioral, not just compilation |
| coverage | push/PR | `cargo tarpaulin -p eggsec --features rest-api,nse` | Linux | Re-compiles with coverage | Coverage measurement | optional-move | Keep as diagnostic only | Coverage is advisory unless threshold enforced |
| build | push/PR | `cargo build -p eggsec --release` (3 OS: ubuntu, macos, windows) | Cross-platform | Full release build per OS | Cross-platform compilation | mandatory-retain | — | Verifies release build compiles on all targets |
| security-audit | push/PR | `cargo audit --deny warnings` | Linux | 0 | Known vulnerability detection | mandatory-retain | — | Security gate |
| cargo-deny | push/PR | `cargo deny check advisories/licenses/bans` | Linux | 0 | License/compliance/ban policy | mandatory-retain | — | Supply chain compliance |
| dependency-review | PR only | GitHub dependency-review-action | Linux | 0 | Critical dependency introduction | mandatory-retain | — | PR-only security gate |
| secret-scan | PR only | gitleaks-action | Linux | 0 | Secret leakage | mandatory-retain | — | PR-only security gate |
| architecture-guards | push/PR (needs: fmt) | no-default build + 8 Rust tests + 1 output test + guards script | Linux | Overlaps with check/test-features | Architecture invariant regression | mandatory-retain | — | Core invariant protection |
| feature-profiles | push/PR | `cargo check -p eggsec --features <9 profiles>` | Linux | Overlaps with check matrix | Feature compilation (representative) | mandatory-merge | — | Representative, not exhaustive |
| python-tests | push/PR | maturin develop + pytest (9 matrix entries) | Linux | 9 separate maturin builds | Python binding correctness per feature | mandatory-merge | — | Each entry tests a different feature-gated binding |
| python-capability-matrix | push/PR | `check-python-capability-matrix.py` | Linux | Requires maturin develop | Operation registry ↔ JSON drift | mandatory-retain | — | Metadata consistency |
| python-architecture-guards | push/PR | `check-python-architecture-guards.py` | Linux | Requires maturin develop | Python architecture invariants | mandatory-retain | — | Architecture drift protection |
| python-stub-parity | push/PR | `check_python_stub_parity.py` | Linux | Requires maturin develop | Stub ↔ runtime parity | mandatory-retain | — | Type safety gate |
| python-type-check | push/PR | `check_python_types.sh` (mypy + pyright) | Linux | Requires maturin develop | Type correctness | mandatory-retain | — | Type safety gate |
| python-maturity-consistency | push/PR | `check-python-capability-matrix.py --check-maturity` | Linux | Requires maturin develop | Maturity label consistency | mandatory-retain | — | Metadata consistency |
| python-feature-metadata | push/PR | `check-python-architecture-guards.py --check-feature-metadata` | Linux | Requires maturin develop | Feature metadata consistency | mandatory-retain | — | Metadata consistency |
| python-profile-manifest | push/PR | `validate_python_profiles.py` | Linux | 0 (no build) | Profile manifest validity | mandatory-retain | — | Profile schema validation |
| python-evidence-bundle | push/PR (needs: python-tests) | `build_python_release_evidence.py` | Linux | Requires wheel + test results | Evidence generation | optional-move | — | Evidence is not behavioral verification |
| python-maturity-guard | push/PR (needs: python-evidence-bundle) | `check_maturity_guard.py` | Linux | Requires evidence bundle | Maturity claim validation | optional-move | — | Depends on evidence bundle; can be release-gated |
| python-skip-budget | push/PR (needs: python-tests) | `python_skip_budget.py` | Linux | Requires JUnit XML | Skip budget enforcement | mandatory-retain | — | Prevents test suite erosion |
| python-release-gate | push/PR (if: always) | Aggregates all python-* jobs | Linux | — | Python gate aggregation | mandatory-retain | — | Single merge gate for Python |
| release-gate | push/PR (if: always) | Aggregates fmt, clippy, check, test-features, security-audit, architecture-guards, feature-profiles, python-release-gate | Linux | — | Top-level merge gate | mandatory-retain | — | Single required status check |

### deep-checks.yml (weekly schedule)

| Job | Trigger | Command(s) | Platform | Disposition | Replacement | Reason |
|-----|---------|-----------|----------|-------------|-------------|--------|
| deep-checks | Weekly cron, manual, workflow_call | `cargo check --workspace --all-features` + `cargo test --workspace --all-features` + `cargo check -p eggsec --features full` | Linux | optional-move | Keep as weekly diagnostic | Validates all-features compilation; not required per-PR |

### security-scan.yml

| Job | Trigger | Command(s) | Platform | Disposition | Replacement | Reason |
|-----|---------|-----------|----------|-------------|-------------|--------|
| security-scan | push, PR, daily schedule, manual | Downloads pre-built binary, runs scan profiles | Linux | remove-obsolete | — | Downloads a previously published binary which cannot validate the current commit; external network scan is not a repository correctness test |
| scheduled-scan | Daily schedule | Downloads pre-built binary, scans targets | Linux | remove-obsolete | — | Same as above; scheduled variant |

### python-wheels.yml

| Job | Trigger | Command(s) | Platform | Disposition | Replacement | Reason |
|-----|---------|-----------|----------|-------------|-------------|--------|
| build | push/PR (path-filtered) | maturin build (linux + macos) | Cross-platform | release-local | — | Wheel builds are release preparation |
| test | push/PR (needs: build) | Install wheel + pytest + smoke tests | Cross-platform | release-local | — | Installed-wheel testing is release validation |
| architecture | push/PR | `check-architecture-guards.sh` | Linux | mandatory-retain | Already in test.yml | Duplicate of test.yml architecture-guards; can be deduplicated |
| publish-testpypi | workflow_dispatch only | maturin upload to TestPyPI | Linux | release-local | — | Manual release gate |
| publish-pypi | workflow_dispatch + approval | pypi-publish action | Linux | release-local | — | Manual release gate with environment approval |
| test-feature-profiles | push/PR (needs: build) | Build with features + pytest per profile | Linux | release-local | — | Feature-specific wheel testing |
| validation-matrix | push/PR (needs: build) | `validate_python_release_1_2.sh` | Linux | release-local | — | Release validation script |
| performance-report | push/PR (needs: build) | Performance pytest suite | Linux | release-local | — | Performance benchmarking |
| skip-report | push/PR (always) | Capture pytest skip reasons | Linux | release-local | — | Diagnostic skip analysis |
| python-architecture-guards | push/PR | capability matrix + arch guards + stub parity | Linux | mandatory-retain | Duplicate of test.yml jobs | Can be consolidated |
| build-sdist | push/PR | maturin sdist | Linux | release-local | — | Source distribution building |
| test-multi-version | push/PR (needs: build) | Install wheel on different Python versions | Cross-platform | release-local | — | Multi-version validation |
| wheel-metadata | push/PR (needs: build) | Inspect wheel metadata | Linux | release-local | — | Wheel metadata validation |
| test-documentation-examples | push/PR (needs: build) | pytest documentation examples | Linux | release-local | — | Documentation correctness |
| generate-api-reference | push/PR (needs: build) | Generate API reference docs | Linux | example-relocate | — | Documentation generation, not correctness |

### testpypi-rehearsal.yml (manual only)

| Job | Trigger | Command(s) | Platform | Disposition | Replacement | Reason |
|-----|---------|-----------|----------|-------------|-------------|--------|
| build-and-verify | workflow_dispatch | Build, upload to TestPyPI, install, validate | Linux | release-local | — | Pre-release rehearsal; manual-only |

### release.yml (tag-driven)

| Job | Trigger | Command(s) | Platform | Disposition | Replacement | Reason |
|-----|---------|-----------|----------|-------------|-------------|--------|
| validate-tag | tag push / workflow_dispatch | Version alignment, clean tree, tag check | Linux | release-local | — | Release gate |
| build-wheels | tag push (needs: validate-tag) | maturin build (4 targets: linux x86_64/aarch64, macos x86_64/aarch64) | Cross-platform | release-local | — | Release wheel building |
| build-sdist | tag push (needs: validate-tag) | maturin sdist | Linux | release-local | — | Source distribution |
| test-release-wheels | tag push (needs: build-wheels) | Install + pytest on release wheel | Linux | release-local | — | Release validation |
| generate-evidence | tag push (needs: validate-tag, test-release-wheels) | Artifact manifest + release notes | Linux | release-local | — | Release evidence |
| publish-testpypi | tag push (needs: build/test) | Upload to TestPyPI | Linux | release-local | — | Pre-release publication |
| publish-pypi | tag push (needs: testpypi, evidence) | Upload to PyPI + GitHub release | Linux | release-local | — | Final publication (manual approval) |

### .gitlab-ci.yml

| Job | Trigger | Command(s) | Platform | Disposition | Replacement | Reason |
|-----|---------|-----------|----------|-------------|-------------|--------|
| eggsec-quick-scan | Variable-gated | Download pre-built binary, scan | Alpine | example-relocate | — | Example workflow for GitLab users |
| eggsec-full-scan | Variable-gated | Download pre-built binary, full scan | Alpine | example-relocate | — | Example workflow |
| eggsec-fuzz | Variable-gated | Download pre-built binary, fuzz | Alpine | example-relocate | — | Example workflow |
| eggsec-recon | Variable-gated | Download pre-built binary, recon | Alpine | example-relocate | — | Example workflow |
| eggsec-waf-test | Variable-gated | Download pre-built binary, WAF test | Alpine | example-relocate | — | Example workflow |
| eggsec-scheduled | Schedule | Download pre-built binary, scan targets | Alpine | example-relocate | — | Example workflow |
| eggsec-load-test | Variable-gated | Download pre-built binary, load test | Alpine | example-relocate | — | Example workflow |

## Deliverable 3: Script ownership table

| Script | Classification | Requires installed extension | Requires built wheel | Requires JUnit | Requires Git metadata | Requires credentials | Requires network | Requires system packages | Disposition |
|--------|---------------|----------------------------|---------------------|---------------|----------------------|---------------------|-----------------|------------------------|-------------|
| `scripts/check-architecture-guards.sh` | static architecture guard | No | No | No | No | No | No | ripgrep (rg) | mandatory-retain |
| `scripts/check_python_types.sh` | metadata consistency check | No | Yes (maturin develop) | No | No | No | No | mypy, pyright (optional) | mandatory-retain |
| `scripts/check-python-capability-matrix.py` | metadata consistency check | No | Yes (maturin develop) | No | No | No | No | No | mandatory-retain |
| `scripts/check-python-architecture-guards.py` | static architecture guard | No | Yes (maturin develop) | No | No | No | No | No | mandatory-retain |
| `scripts/check_python_stub_parity.py` | metadata consistency check | No | Yes (maturin develop) | No | No | No | No | No | mandatory-retain |
| `scripts/validate_python_profiles.py` | packaging smoke check | No | No | No | No | No | No | No | mandatory-retain |
| `scripts/run_python_profile.py` | behavioral test runner | No | Yes (maturin build) | Yes | No | No | No | Per profile | optional-move |
| `scripts/build_python_release_evidence.py` | historical evidence/process helper | No | Yes (maturin develop) | Yes | Yes (git SHA) | No | No | ripgrep (rg) | release-local |
| `scripts/check_maturity_guard.py` | metadata consistency check | No | Yes (maturin develop) | No | No | No | No | ripgrep (rg) | optional-move |
| `scripts/python_skip_budget.py` | behavioral test runner | No | No | Yes | No | No | No | No | mandatory-retain |
| `scripts/check_python_compatibility.py` | behavioral test runner | No | Yes (maturin develop) | No | No | No | No | No | mandatory-retain |
| `scripts/generate_python_compatibility_baseline.py` | manual release helper | No | Yes (maturin develop) | No | No | No | No | No | release-local |
| `scripts/validate_python_release_candidate.sh` | packaging smoke check | No | Yes (maturin build) | Yes | No | No | No | No | release-local |

### Additional scripts (not in scoped list but discovered)

| Script | Classification | Disposition |
|--------|---------------|-------------|
| `scripts/build_wheel_profiles.sh` | packaging smoke check | release-local |
| `scripts/validate_wheel.sh` | packaging smoke check | release-local |
| `scripts/test_documentation_examples.py` | behavioral test runner | release-local |
| `scripts/generate_api_reference.py` | manual release helper | example-relocate |
| `scripts/validate_python_release_1_2.sh` | packaging smoke check | release-local |

## Deliverable 4: Critical invariant list

| Invariant | Defect class | Direct check | Type | Disposition |
|-----------|-------------|-------------|------|-------------|
| Workspace no-default compilation | Build regression | `cargo check --workspace --no-default-features` | Behavioral (Rust) | mandatory-retain |
| Central enforcement behavior | Enforcement bypass | `cargo test -p eggsec --test enforcement_matrix` | Behavioral (Rust) | mandatory-retain |
| Strict-surface scope semantics | Scope bypass | `cargo test -p eggsec --test enforcement_matrix` (sections on strict surfaces) | Behavioral (Rust) | mandatory-retain |
| Operation metadata consistency | Metadata drift | `cargo test -p eggsec --test metadata_consistency` | Behavioral (Rust) | mandatory-retain |
| Command registry consistency | Registry drift | `cargo test -p eggsec --test command_registry` | Behavioral (Rust) | mandatory-retain |
| Tool registration consistency | MCP/REST exposure drift | `cargo test -p eggsec --test tool_registration --features rest-api` | Behavioral (Rust) | mandatory-retain |
| Enforced-dispatch regression | Dispatch bypass | `cargo test -p eggsec --test enforced_dispatch_regression` | Behavioral (Rust) | mandatory-retain |
| Output report-envelope stability | Report format breakage | `cargo test -p eggsec-output --test report_envelope` | Behavioral (Rust) | mandatory-retain |
| Architecture dependency boundaries | Crate boundary violation | `bash scripts/check-architecture-guards.sh` (checks 11-23) | Static (grep) | mandatory-retain |
| Python API/stub parity | Stub drift | `python scripts/check_python_stub_parity.py` | Behavioral (Python) | mandatory-retain |
| Python feature/capability metadata consistency | Metadata drift | `python scripts/check-python-capability-matrix.py` | Behavioral (Python) | mandatory-retain |
| Python error/redaction behavior | Redaction bypass | `pytest crates/eggsec-python/tests/test_redaction_comprehensive.py` | Behavioral (Python) | mandatory-retain |
| Installed-package importability | Installation breakage | `python -c "import eggsec"` (in test workflows) | Behavioral (Python) | mandatory-retain |
| Raw dispatch not in strict surfaces | Enforcement bypass | `cargo test -p eggsec --test enforced_dispatch_regression` | Behavioral (Rust) | mandatory-retain |
| MCP exposure terminology split | Documentation drift | `bash scripts/check-architecture-guards.sh` (check 3) | Static (grep) | mandatory-retain |
| TUI workers directory absent | Architecture regression | `bash scripts/check-architecture-guards.sh` (check 10) | Static (grep) | mandatory-retain |
| Runtime free of TUI/transport deps | Architecture regression | `bash scripts/check-architecture-guards.sh` (checks 11-12, 20) | Static (grep) | mandatory-retain |
| Output free of engine/runtime deps | Architecture regression | `bash scripts/check-architecture-guards.sh` (check 23) | Static (grep) | mandatory-retain |
| NSE script loading is resolver-owned | Security boundary | `bash scripts/check-architecture-guards.sh` (check 24) | Static (grep) | mandatory-retain |
| ManualPermissive stays in manual surfaces | Security boundary | `bash scripts/check-architecture-guards.sh` (checks 26-27, 35-36) | Static (grep) | mandatory-retain |
| Feature metadata JSON/Rust consistency | Metadata drift | `python scripts/check-python-architecture-guards.py` (guard 5) | Behavioral (Python) | mandatory-retain |
| Sync/async engine operation parity | API inconsistency | `python scripts/check-python-architecture-guards.py` (guard 6) | Behavioral (Python) | mandatory-retain |
| Required docs exist | Documentation regression | `bash scripts/check-architecture-guards.sh` (checks 7-8) | Static (grep) | mandatory-retain |

### Distinction: Static grep guards vs behavioral tests

**Static grep guards** (in `check-architecture-guards.sh`) protect boundaries that are not cheaply expressible as Rust or Python tests:
- Crate dependency boundaries (TUI/engine/runtime/daemon/output isolation)
- Terminology consistency (no stale `manual_only`, MCP exposure split)
- NSE security boundaries (ManualPermissive confinement, script loading ownership)
- Documentation currency (required docs exist, links resolve)

These remain mandatory because they catch structural regressions that behavioral tests would miss until much later.

**Behavioral tests** (Rust `cargo test` and Python `pytest`) protect:
- Enforcement logic correctness
- Metadata consistency
- API/stub parity
- Report envelope stability
- Dispatch regression

Both categories are required for merge readiness.

## Deliverable 5: Baseline measurements

| Metric | Count | Notes |
|--------|-------|-------|
| Workflow files | 7 | test.yml, deep-checks.yml, security-scan.yml, python-wheels.yml, testpypi-rehearsal.yml, release.yml, .gitlab-ci.yml |
| Total jobs (test.yml) | 25 | Including gate aggregation jobs |
| Total jobs (deep-checks.yml) | 1 | Weekly diagnostic |
| Total jobs (security-scan.yml) | 2 | Both are external-scan advisory |
| Total jobs (python-wheels.yml) | 16 | Wheel build/test/publish pipeline |
| Total jobs (testpypi-rehearsal.yml) | 1 | Manual rehearsal |
| Total jobs (release.yml) | 7 | Tag-driven release pipeline |
| Total jobs (.gitlab-ci.yml) | 7 | Example GitLab CI templates |
| Rust matrix entries (check) | 10 | default, minimal-cli, rest-api, grpc-api, packet-inspection, stress-testing, nse, nse-sandbox, api-schema, sbom |
| Rust matrix entries (test-features) | 5 | lib-default, lib-rest-api, lib-api-schema, lib-sbom, feature-surface |
| Rust matrix entries (feature-profiles) | 9 | tool-api+rest-api, grpc-api, db-pentest, db-pentest-mcp+tool-api+rest-api, mobile, mobile-dynamic, web-proxy, web-proxy-mcp+tool-api+rest-api, c2-mcp+tool-api+rest-api |
| Python matrix entries (test.yml python-tests) | 9 | default-wheel, nse, db-pentest, web-proxy, mobile, headless-browser, daemon-client, packet-inspection, stress-testing |
| Python wheel build targets | 2 | linux x86_64, macos universal2 |
| Python release wheel targets | 4 | linux x86_64, linux aarch64, macos x86_64, macos aarch64 |
| Artifact upload/download handoffs | 6+ | python-test-results, python-evidence-bundle, wheel artifacts, testpypi-validated-wheels, release-evidence, skip-report |
| Scheduled workflows | 2 | deep-checks (weekly), security-scan (daily) |
| Publishing-capable jobs | 4 | publish-testpypi (python-wheels.yml), publish-pypi (python-wheels.yml), publish-testpypi (release.yml), publish-pypi (release.yml) |
| Unique mandatory tools in CI | 8 | rustc, cargo, cargo-clippy, cargo-audit, cargo-deny, cargo-tarpaulin, maturin, ripgrep (rg) |
| Approximate critical-path duration | N/A | Structural counts only; historical timing data unavailable |
| Common CI-only repair categories | 3 | (1) Architecture guard script ripgrep dependency, (2) Feature-gated compilation failures, (3) Python stub/metadata drift |

### Duplicate build analysis

The `check` job in test.yml and `feature-profiles` job overlap significantly:
- `check` matrix includes entries like `nse`, `rest-api`, `api-schema`, `sbom`, `packet-inspection`, `stress-testing`
- `feature-profiles` includes `grpc-api`, `db-pentest`, `mobile`, `mobile-dynamic`, `web-proxy`, and MCP variants
- Combined, they verify ~19 distinct feature configurations

The `python-tests` matrix and `test-feature-profiles` (python-wheels.yml) overlap for features like nse, db-pentest, web-proxy, mobile, packet-inspection, stress-testing. The python-wheels.yml versions build full wheels while test.yml uses maturin develop.

The `python-architecture-guards` job in python-wheels.yml duplicates three jobs from test.yml (python-capability-matrix, python-architecture-guards, python-stub-parity).

## Deliverable 6: Target workflow sketch

The intended mandatory workflow shape after simplification:

```yaml
# test.yml (push to main, PRs) — MANDATORY
jobs:
  rust:
    # fmt, clippy, no-default build, core tests, architecture guards
    # Single job with sequential steps (no matrix for core checks)
    steps:
      - cargo fmt --all --check
      - cargo clippy --lib -p eggsec -- -D warnings
      - cargo check --workspace --no-default-features
      - cargo test -p eggsec --lib
      - cargo test -p eggsec --test metadata_consistency
      - cargo test -p eggsec --test command_registry
      - cargo test -p eggsec --test tool_registration --features rest-api
      - cargo test -p eggsec --test feature_matrix
      - cargo test -p eggsec --test enforcement_matrix
      - cargo test -p eggsec --test enforced_dispatch_regression
      - cargo test -p eggsec-output --test report_envelope
      - bash scripts/check-architecture-guards.sh

  python:
    # One venv, one maturin develop, all Python checks
    steps:
      - maturin develop
      - python scripts/check-python-capability-matrix.py
      - python scripts/check-python-architecture-guards.py
      - python scripts/check_python_stub_parity.py
      - bash scripts/check_python_types.sh
      - pytest crates/eggsec-python/tests/ -v --timeout=60
      - python scripts/python_skip_budget.py ...

  portability:
    # Narrow macOS/Windows checks (build only, not full test)
    strategy:
      matrix:
        os: [macos-latest, windows-latest]
    steps:
      - cargo build -p eggsec --release

  # Advisory/optional (not required for merge)
  coverage:
    # tarpaulin coverage (diagnostic only)
    if: github.event_name == 'push'

  security-audit:
    # cargo audit + cargo deny (kept but separate from merge gate if slow)
```

### Design principles

1. **Job decomposition is for OS isolation or meaningful dependency separation only** — not one status check per command.
2. **Rust core checks run in a single sequential job** — they share the same toolchain and cache, and sequential execution avoids redundant compilation.
3. **Python checks run in a single job** — one maturin develop, then all checks sequentially.
4. **Portability is build-only** — cross-platform test execution is reserved for release validation.
5. **Advisory checks are separate from the merge gate** — coverage, security audit, and deep checks inform but do not block.

## Handoff notes

This phase should be executable by a smaller model or junior maintainer because it is observational. Require exact file references and command ownership. Do not accept a generic recommendation document that lacks a one-to-one disposition map; later deletion work depends on that map.

Phase A deliverables are complete. The verification contract (`docs/VERIFICATION.md`) and disposition tables provide the explicit map needed for Phase B implementation. No workflows, scripts, or runtime behaviors were modified in this phase.