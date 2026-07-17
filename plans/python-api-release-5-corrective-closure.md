# Eggsec Python API Release 5 Corrective Closure Plan

## Handoff objective

Release 5 is feature-complete in broad shape. This pass is intentionally narrow: it must not add new Python domains, operations, session types, tool abstractions, wheel profiles, or protocol capabilities.

The purpose of this pass is to convert the current implementation-complete state into an evidence-complete, internally consistent, publication-ready Release 5 candidate. The work is limited to five known closure areas:

1. make all release evidence and skip-budget checks fail closed;
2. resolve or explicitly disposition every current redaction xfail;
3. remove documentation and command-name drift introduced during Phase F;
4. produce authoritative exact-commit CI evidence for the final release candidate;
5. complete a clean TestPyPI upload/install/import/example rehearsal.

Do not use this pass to refactor unrelated code, expand the stable API, promote provisional domains, change operation semantics, or introduce new compatibility promises.

## Required end state

At completion:

- missing test artifacts, JUnit files, evidence files, checksums, or workflow outputs cause hard failure;
- the redaction suite has no unexplained xfails and no stable or persisted boundary has a known secret-exposure gap;
- all documentation, Make targets, CI jobs, and handoff instructions reference the actual canonical scripts and test files;
- one final commit has a complete, visible, successful release-gate run and a retained evidence bundle bound to that exact SHA;
- TestPyPI publication and clean-environment installation succeed using the artifacts intended for release;
- the release checklist contains only claims supported by exact-commit evidence;
- Release 5 can be declared closed without relying on commit-message assertions or local-only test reports.

## Scope boundaries

Allowed changes:

- CI workflow corrections;
- evidence generation and validation fixes;
- skip/xfail policy enforcement;
- redaction fixes and narrowly related serialization changes;
- documentation and command-name corrections;
- release scripts and TestPyPI rehearsal automation;
- tests needed to prove the above.

Disallowed changes:

- new operations or tools;
- new public Python classes unrelated to closure;
- new feature flags or wheel profiles;
- new subsystem implementations;
- broad namespace restructuring;
- maturity promotion except where already planned and fully evidenced;
- unrelated cleanup or dependency upgrades.

## Workstream 1 — Make release evidence fail closed

### Problem

The current skip-budget workflow can succeed when the expected JUnit file is absent. Similar fail-open behavior must be ruled out across the complete evidence pipeline.

### Required work

Audit:

- `.github/workflows/test.yml`;
- `.github/workflows/release.yml`;
- any reusable Python validation workflows;
- `scripts/python_skip_budget.py`;
- `scripts/build_python_release_evidence.py`;
- `scripts/run_python_profile.py`;
- `scripts/validate_python_profiles.py`.

Replace every pattern equivalent to:

```sh
if artifact_exists; then
    validate
else
    echo "skipping"
fi
```

with a hard failure for required profiles and required release artifacts.

Required behavior:

- missing JUnit XML fails the skip-budget job;
- missing downloaded artifacts fail the consuming job;
- an empty test result file fails validation;
- zero executed tests fail any blocking profile;
- malformed XML or JSON fails validation;
- missing evidence bundle members fail aggregation;
- checksum mismatches fail aggregation;
- commit-SHA mismatch fails aggregation;
- skipped, cancelled, or neutral required jobs fail the final release gate;
- scheduled or external profiles remain explicitly non-blocking and cannot be reported as passed.

Add negative tests for each failure mode. Where practical, run the evidence validator against intentionally incomplete temporary bundles.

### Explicit acceptance criteria

- Deleting or renaming the expected default-wheel JUnit file causes `python-skip-budget` to fail.
- Supplying an empty JUnit file causes failure.
- Supplying a JUnit report with zero executed tests causes failure.
- Removing any required evidence file causes evidence aggregation to fail.
- Modifying one evidence file after checksum generation causes failure.
- Supplying evidence generated for a different commit SHA causes failure.
- The aggregate release gate fails when any required dependency is `failure`, `cancelled`, `skipped`, or absent.
- No required CI validation path contains `|| true`, `continue-on-error: true`, or a missing-artifact success branch.

## Workstream 2 — Redaction xfail triage and closure

### Problem

The current comprehensive redaction suite reports 23 expected failures. These document useful gaps, but they do not support the claim that redaction passes across all boundaries.

### Required work

Create a machine-readable redaction disposition manifest, for example:

`crates/eggsec-python/tests/redaction_dispositions.json`

Each current xfail must record:

- test node ID;
- affected type and field;
- boundary category: repr, str, dict, JSON, event, checkpoint, artifact, report, callback, daemon, schema, or tool transport;
- maturity level of the affected API;
- whether the type is expected to carry secrets;
- disposition: `fix`, `intentional_raw_evidence`, or `deferred_provisional`;
- rationale;
- issue or plan reference for any deferral;
- expiry release or review date for deferrals.

Rules:

- stable APIs may not retain `deferred_provisional` redaction gaps;
- persisted or transported authentication material may not be intentionally exposed;
- `__repr__` and `__str__` must not leak credentials, tokens, secrets, private keys, passwords, authorization headers, cookies, or connection strings;
- JSON and dictionary serialization must redact secret-bearing wrapper types unless a clearly named privileged/raw export method is used;
- checkpoints, events, logs, reports, artifacts, callbacks, and daemon envelopes must be tested independently;
- raw evidence fields may remain raw only when that is their explicit purpose and documentation warns that callers must treat them as sensitive.

Prefer fixing the underlying serialization or representation behavior. Use xfail only for provisional APIs with explicit tracked deferral and no stable-path exposure.

### Explicit acceptance criteria

- Every original redaction xfail has a recorded disposition.
- The blocking redaction suite has zero unexplained xfails.
- Stable APIs have zero redaction xfails.
- Authentication data is absent from `repr`, `str`, logs, event envelopes, checkpoints, reports, and daemon messages.
- Secret-bearing values round-trip only through explicitly privileged/raw APIs, not default serializers.
- A canary suite injects unique secret markers and asserts they do not appear in captured output across every boundary.
- Any retained provisional xfail names an issue, rationale, expiry point, and affected feature profile.
- The release checklist does not mark redaction closure complete unless the generated disposition report satisfies these rules.

## Workstream 3 — Documentation and command consistency

### Problem

Phase F documentation contains stale or inconsistent filenames and commands, including references to scripts and tests that do not match the repository’s canonical files.

### Required work

Audit all Release 5 documentation and operational instructions for references to:

- compatibility baseline generation;
- compatibility checking;
- redaction tests;
- resource-budget tests;
- evidence generation;
- profile validation;
- release commands;
- TestPyPI publication.

Canonicalize names around the files that actually exist, including:

- `scripts/generate_python_compatibility_baseline.py`;
- `scripts/check_python_compatibility.py`;
- `crates/eggsec-python/tests/test_redaction_comprehensive.py`;
- `crates/eggsec-python/tests/test_resource_budgets.py`;
- `scripts/build_python_release_evidence.py`.

Add a documentation-link and command-reference checker that scans Markdown, Makefiles, AGENTS files, and skill files for repository-local paths and verifies that referenced files exist. Allow explicit placeholders only when marked as such.

Review operation identifier examples and standardize the canonical operation naming convention. Aliases may be documented, but every example must distinguish canonical IDs from compatibility aliases.

### Explicit acceptance criteria

- No documentation references nonexistent Release 5 scripts or tests.
- All documented Make targets exist and invoke the documented canonical files.
- All documented repository-local paths pass an automated existence check.
- Compatibility, evidence, redaction, and publication instructions are identical across README, AGENTS, skill, architecture, and Python docs where duplicated.
- Operation ID examples consistently identify the canonical form.
- Running each documented Release 5 command from a clean checkout succeeds or fails only for an explicitly documented external prerequisite.

## Workstream 4 — Exact-commit CI and evidence closure

### Problem

Implementation commits report passing local tests, but Release 5 closure requires authoritative CI evidence tied to the exact final commit.

### Required work

Define one release-candidate commit as the evidence target. The full release gate must run on that exact SHA after all corrective changes land.

The required CI set must include at minimum:

- Rust workspace checks relevant to `eggsec-python`;
- Python unit and golden-contract tests;
- tool-core and schema tests;
- namespace governance checks;
- stub parity and type checks;
- compatibility baseline and semantic checker;
- resource-budget tests;
- redaction suite and disposition validator;
- profile manifest validation;
- skip-budget enforcement;
- default-wheel build and installed-wheel tests;
- representative feature-profile builds;
- documentation example tests;
- evidence bundle generation and verification.

The evidence bundle must include:

- exact commit SHA;
- dirty-tree status;
- workflow run identifier;
- toolchain and platform metadata;
- wheel and sdist filenames and SHA-256 values;
- test totals, skips, xfails, and reasons;
- compatibility diff;
- resource-budget results;
- redaction disposition report;
- namespace and maturity report;
- profile results;
- documentation-example results;
- final aggregate status.

Do not manually check off release gates before the corresponding exact-commit artifact exists.

### Explicit acceptance criteria

- The final release-candidate SHA has a visible successful CI run.
- Every required release-gate dependency reports `success`.
- The retained evidence bundle names the same SHA as the workflow checkout.
- The evidence validator passes when run after downloading the artifact into a clean checkout.
- Test counts in the bundle match the uploaded JUnit reports.
- Skip and xfail totals match their machine-readable reports.
- No evidence file is generated from an older commit or copied from a prior run.
- The release checklist links or records the exact workflow run and artifact identity.

## Workstream 5 — TestPyPI publication rehearsal

### Problem

The final publication gate remains open. Local wheel builds do not prove index upload, metadata correctness, clean installation, or real consumer import behavior.

### Required work

Add a manually triggered or protected release-rehearsal workflow that:

1. verifies tag, Cargo version, Python package version, and compatibility baseline version alignment;
2. builds the same wheel and sdist artifacts intended for release;
3. verifies artifact hashes and metadata;
4. uploads to TestPyPI using trusted publishing or the repository’s approved secret mechanism;
5. creates a fresh isolated environment with no workspace source path on `PYTHONPATH`;
6. installs the package from TestPyPI only;
7. verifies the installed distribution location is outside the repository;
8. imports `eggsec` and all stable public submodules;
9. runs capability discovery and `build_info()`;
10. runs deterministic stable-core smoke tests and executable examples;
11. verifies type stubs are present in the installed wheel;
12. verifies failure behavior for unavailable optional features;
13. records the TestPyPI project/version URL and installation logs in the evidence bundle.

Use a unique pre-release version or disposable rehearsal version so repeated rehearsals do not collide. Do not publish to production PyPI during this pass.

### Explicit acceptance criteria

- Wheel and sdist upload to TestPyPI succeeds.
- A clean environment installs from TestPyPI without using local paths or editable installs.
- `import eggsec` and every stable namespace import succeed.
- `build_info()` reports the expected version, wheel profile, ABI, schema, and compiled features.
- Stable deterministic examples pass against the installed distribution.
- Optional unavailable features raise the documented `FeatureUnavailableError` or equivalent, not `ImportError` caused by packaging omissions.
- Installed `.pyi` files and `py.typed` metadata are present and usable by at least one type checker smoke test.
- The rehearsal logs and artifact identities are included in the exact-commit evidence bundle.
- The TestPyPI release-checklist item is checked only after this workflow succeeds.

## Workstream 6 — Release claim reconciliation

### Problem

Some current checklist and documentation claims are stronger than the available evidence, particularly around redaction and final release readiness.

### Required work

Review and reconcile:

- `crates/eggsec-python/RELEASE_CHECKLIST.md`;
- `crates/eggsec-python/README.md`;
- `docs/python/PHASE_F.md`;
- `docs/python/GRADUATION_REVIEW.md`;
- `docs/python/domain-maturity.md`;
- package classifiers and status text;
- generated compatibility and maturity reports.

Rules:

- implementation-complete is not equivalent to release-closed;
- xfailed tests cannot be described as passing without qualification;
- scheduled or external profiles cannot be described as blocking CI proof;
- provisional domains remain provisional unless all promotion evidence is present;
- Beta status must be explicitly scoped to the stable-core API if broader domains remain provisional or experimental;
- publication readiness must remain false until TestPyPI rehearsal and exact-commit evidence pass.

### Explicit acceptance criteria

- Every checked release gate has a corresponding artifact or CI result.
- No document states that all redaction boundaries pass while unresolved blocking xfails exist.
- No provisional or experimental subsystem is described as stable.
- Package status language clearly distinguishes stable-core guarantees from provisional and experimental surfaces.
- The final closure report lists any intentionally deferred non-blocking items with rationale and future trigger conditions.

## Required validation commands

The implementation agent must provide final output for at least:

```sh
cargo test -p eggsec-python
python scripts/check-python-architecture-guards.py
python scripts/check_phase_c_governance.py
python scripts/validate_python_profiles.py
python scripts/check_python_compatibility.py
python -m pytest crates/eggsec-python/tests/test_resource_budgets.py -v
python -m pytest crates/eggsec-python/tests/test_redaction_comprehensive.py -v
python -m pytest crates/eggsec-python/tests/test_golden_contract.py -v
python scripts/build_python_release_evidence.py --commit "$(git rev-parse HEAD)"
```

Also run any new negative evidence tests, documentation-reference checker, disposition validator, installed-wheel tests, and TestPyPI rehearsal workflow introduced by this pass.

## Final handoff deliverables

The implementing agent must leave:

1. corrected fail-closed CI and evidence scripts;
2. a redaction disposition manifest and validator;
3. fixes or explicit allowed deferrals for every current redaction xfail;
4. an automated documentation/path consistency checker;
5. corrected Release 5 documentation and commands;
6. a successful exact-commit release-gate run;
7. a downloadable verified evidence bundle;
8. a successful TestPyPI rehearsal record;
9. an updated release checklist containing only evidenced claims;
10. a concise closure report stating whether Release 5 is fully closed.

## Final release decision

Release 5 may be marked complete only when all of the following are true:

- all required CI jobs pass on the final SHA;
- evidence validation is fail-closed and passes;
- no stable or persisted redaction gap remains;
- all retained xfails are explicitly non-blocking, provisional, tracked, and time-bounded;
- documentation references are internally consistent;
- TestPyPI upload and clean installation succeed;
- the final evidence bundle is retained and independently reproducible.

If any condition remains unmet, the closure report must state `Release 5 implementation complete; release closure pending` and identify the exact blocking gate.