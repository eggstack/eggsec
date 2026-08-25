# Corrective Phase G Plan: Crates.io Publishability and Release-Check Closure

## Status


Status: Executed.
Completed. All acceptance criteria met. Linux designated as the supported release host.

## Objective

Make the manual Rust release procedure internally valid and prove it through an end-to-end, non-publishing local release check. Every crate intentionally published to crates.io must have registry-resolvable internal dependencies, a validated package set, and a topologically correct publication order. The release-check command must complete with useful diagnostics on the supported maintainer platform rather than timing out or reporting partial execution as success.

This phase must not publish any package.

## Confirmed defects to correct

### Path-only internal dependencies

Publishable workspace crates currently declare internal normal dependencies using only local paths, for example:

```toml
[dependencies]
eggsec-core = { path = "../eggsec-core" }
```

A package uploaded to crates.io cannot resolve a repository-relative path. For an internal dependency that is also published, the manifest must include both the local path and a compatible registry version:

```toml
[dependencies]
eggsec-core = { path = "../eggsec-core", version = "0.1.0" }
```

The path remains active for workspace development; the version makes the packaged manifest registry-resolvable.

### Invalid publication order

`docs/RELEASING.md` currently lists a hand-written order that places the primary `eggsec` crate before several internal crates on which it depends. The order must be derived from the actual Cargo package graph, not manually guessed.

### Incomplete release-check execution

`scripts/release-check.sh` currently:

- discovers publishable crates by filesystem sort order;
- runs both `cargo package` and `cargo publish --dry-run` for every crate;
- truncates Cargo output with `tail -1`;
- treats a broad, potentially network-sensitive dry-run loop as one mandatory command;
- uses Linux-oriented hashing assumptions in a release flow expected to work from macOS;
- has not completed end-to-end in recorded validation.

The script must be simplified and made diagnostic.

## Scope

Primary files:

```text
Cargo.toml
Cargo.lock
crates/*/Cargo.toml
scripts/release-check.sh
docs/RELEASING.md
docs/VERIFICATION.md
AGENTS.md
Makefile
plans/ci-verification-release-simplification-closure-report.md
```

Recommended new helper:

```text
scripts/release-package-graph.py
```

The helper name may differ, but ownership must be explicit and implementation must remain dependency-light.

## Workstream 1 — Establish the authoritative package set

Use Cargo metadata rather than documentation as the source of truth.

Run:

```bash
cargo metadata --format-version 1 > /tmp/eggsec-metadata.json
```

For every workspace package, record:

- Cargo package name;
- manifest path;
- inherited or literal version;
- `publish` value;
- normal internal dependencies;
- build internal dependencies;
- optional internal dependencies;
- dev-only internal dependencies;
- whether the package is part of the Python wheel but intentionally not published to crates.io.

Classify each crate as exactly one of:

- `publish-crates-io`;
- `publish-python-only`;
- `private-workspace`.

Do not rely on Cargo's implicit default `publish = true` for private crates. Every package not intended for crates.io must explicitly declare:

```toml
publish = false
```

Expected review set includes all 15 workspace crates. The current documentation suggests the following intent, which must be verified against architecture and actual package ownership rather than copied blindly:

```text
Potential crates.io packages:
  eggsec-core
  eggsec-tool-core
  eggsec-runtime
  eggsec-output
  eggsec-agent
  eggsec-db-lab
  eggsec-web-proxy
  eggsec-mobile-lab
  eggsec-ui-model
  eggsec-daemon
  eggsec-nse
  eggsec

Expected private/non-crates packages:
  eggsec-cli
  eggsec-tui
  eggsec-python
```

If maintainers do not intend to publish the full library graph at version `0.1.0`, reduce the package set explicitly by applying `publish = false` and update release documentation. Do not leave ambiguous implicit publication.

## Workstream 2 — Make internal dependencies publishable

For each `publish-crates-io` package, inspect all internal workspace dependencies.

### Normal and build dependencies

Every internal normal or build dependency must satisfy one of:

1. dependency is also published to crates.io and the manifest contains both `path` and `version`;
2. dependency is not included in the published feature/package and Cargo package validation confirms it is absent;
3. dependent package is reclassified as `publish = false`.

Preferred form at the current unified workspace version:

```toml
eggsec-core = { path = "../eggsec-core", version = "0.1.0" }
```

For optional dependencies:

```toml
eggsec-nse = { path = "../eggsec-nse", version = "0.1.0", optional = true }
```

Preserve existing feature flags and `default-features` settings exactly.

### Dev dependencies

Audit internal dev dependencies separately. Cargo may omit path-only dev dependencies from the published manifest, but the implementation must not assume this silently. Either:

- add matching versions for consistency; or
- document and test the intentional path-only dev-dependency behavior with `cargo package --list` / extracted manifest inspection.

Do not force test-only crates onto crates.io solely for a dev dependency.

### Version policy

Because workspace crates currently inherit one workspace version, use that exact version for internal registry dependencies unless a deliberate semver range policy is documented.

For initial `0.1.0` publication, exact compatible versions generated as `0.1.0` are acceptable. Do not use wildcard dependencies. Do not use `*` or broad `>=` constraints.

### Lockfile

Regenerate `Cargo.lock` only as required by manifest edits. Review the diff for unrelated dependency updates. This phase must not become a dependency-upgrade pass.

## Workstream 3 — Add a deterministic package-graph helper

Add a small standard-library Python helper, recommended interface:

```bash
python scripts/release-package-graph.py list
python scripts/release-package-graph.py validate
python scripts/release-package-graph.py order
```

It should invoke or parse:

```bash
cargo metadata --format-version 1
```

Required behavior:

### `list`

Print a stable table or JSON containing:

```text
package
version
classification
manifest path
internal dependencies
```

### `validate`

Fail when:

- a private package lacks explicit `publish = false`;
- a publishable package has an internal normal/build dependency without a registry version;
- a publishable package depends on an internal private crate in a way that will remain in the package;
- an internal dependency version does not match the intended workspace release version or documented semver policy;
- a dependency cycle exists among publishable packages;
- documentation names a package that is not in the validated set, if documentation checking is included here.

Diagnostics must identify the package, dependency, manifest path, dependency kind, and corrective form.

### `order`

Topologically sort only the crates.io-publishable package graph. Dependencies must appear before dependents. Stable tie-breaking should use package name.

Example output shape:

```text
eggsec-core
eggsec-output
eggsec-tool-core
...
eggsec
```

Do not hard-code the final order in the helper. Tests may use a small synthetic graph plus the real workspace metadata.

### Tests

Add focused tests for:

- path-only normal dependency rejected;
- path+version dependency accepted;
- private internal dependency rejected for a publishable package;
- optional publishable dependency included in ordering;
- dev-only dependency policy handled as intended;
- cycle detection;
- stable topological ordering;
- explicit `publish = false` classification.

Tests may use Python `unittest` and fixture dictionaries to avoid new dependencies.

## Workstream 4 — Redesign `release-check.sh`

The release check must be predictable and separable into local deterministic checks versus optional registry-sensitive checks.

### Command interface

Recommended interface:

```bash
scripts/release-check.sh [expected-version]
```

Optional environment controls:

```text
EGGSEC_RELEASE_SKIP_PYTHON=1       # only when intentionally validating Rust-only release
EGGSEC_RELEASE_REGISTRY_PREFLIGHT=1
EGGSEC_RELEASE_KEEP_ARTIFACTS=1
```

Do not add many switches. Defaults must perform the full local release validation without publication.

### Required default stages

1. clean working tree and explicit current commit;
2. version alignment using TOML-aware parsing;
3. package-set and dependency-graph validation;
4. `make check`;
5. `make check-python`, unless explicitly and visibly skipped for a Rust-only release;
6. `cargo package` for each publishable crate in topological order;
7. Python wheel and sdist build;
8. `twine check`;
9. fresh-environment wheel installation and smoke tests;
10. portable artifact inventory and SHA-256 output;
11. final explicit `No artifacts were published` message.

### Avoid duplicated Rust validation

Use `cargo package` as the mandatory local package-content and manifest validation. Do not automatically run both `cargo package` and `cargo publish --dry-run` for every package in the default path.

When `EGGSEC_RELEASE_REGISTRY_PREFLIGHT=1` is set, run a separate registry-sensitive preflight in topological order. It may use:

```bash
cargo publish -p <crate> --dry-run
```

Requirements:

- clearly label the stage as registry/network-sensitive;
- preserve full output in a log file;
- print the failing crate and log path;
- do not convert a timeout into success;
- do not require registry preflight for ordinary local command completion unless maintainers explicitly choose that policy.

### Diagnostics

Replace constructs such as:

```bash
cargo package ... 2>&1 | tail -1
```

with full or tee'd output:

```bash
cargo package -p "$crate" 2>&1 | tee "$LOG_DIR/package-$crate.log"
```

On failure, print:

```text
Rust package validation failed: <crate>
Log: <path>
```

Strict shell error propagation must remain active. Be careful with pipelines under `set -o pipefail`.

### Temporary directories and cleanup

Use a single release-check temporary root with an EXIT trap. Avoid overwriting an existing trap when creating the wheel smoke environment. Quote all paths.

Recommended pattern:

```bash
TMP_ROOT=$(mktemp -d)
cleanup() { rm -rf "$TMP_ROOT"; }
trap cleanup EXIT
```

Do not build the trap from unquoted interpolated shell fragments.

### macOS/Linux portability

Support both GNU and BSD userlands.

Required portable handling:

- SHA-256: detect `sha256sum`; otherwise use `shasum -a 256`;
- file size: use a small Python helper or platform detection rather than relying on one `stat` syntax;
- virtual environment activation: POSIX path is acceptable for macOS/Linux;
- temporary directories: use `mktemp -d` compatibly;
- path checks: compare resolved paths rather than hard-coded `projects/eggsec` substrings;
- `python3`/active Python usage: document the minimum version required for `tomllib`.

The script need not support Windows shell execution unless the repository explicitly designates Windows as a release host.

### Artifact isolation

Build artifacts into a temporary or clean version-specific directory. Do not validate stale wheels from a previous run.

The script must identify the wheel matching the current Python interpreter/platform. If multiple wheels exist, fail or select deterministically with an explicit rule.

## Workstream 5 — Validate package contents and order

For each publishable crate in topological order:

```bash
cargo package -p <crate> --allow-dirty=false
```

Use Cargo's actual supported syntax; do not invent flags. A clean working tree is already enforced, so plain `cargo package -p <crate>` is generally sufficient.

Inspect at least representative package archives to ensure:

- internal dependencies contain registry versions in packaged `Cargo.toml`;
- excluded tests/assets are intentional;
- README/license files are present;
- no large build artifacts or secrets are included;
- packages do not depend on private workspace paths.

Prefer an automated extracted-manifest assertion for every package rather than manual inspection only.

The helper may inspect the generated `.crate` archive using Python `tarfile` and parse the normalized packaged `Cargo.toml`.

## Workstream 6 — Correct release documentation

Update `docs/RELEASING.md` from validated metadata.

Required changes:

- exact crates.io package set;
- exact topological publication order;
- explicit private package set;
- default release-check versus optional registry preflight;
- supported release host platforms and prerequisites;
- dependency availability wait requirements;
- Python artifact procedure;
- clear statement that no plan execution publishes anything.

Do not hand-maintain a second order if the helper can print it. Recommended documentation:

```bash
python scripts/release-package-graph.py order
```

Then show a validated example order with a note that the command is authoritative.

Update `docs/VERIFICATION.md` so release readiness does not require commands the repository does not execute or cannot currently pass. All-feature validation must either become a real supported command or be removed from mandatory release criteria with the known unsupported profiles documented separately.

## Workstream 7 — Correct prior closure status

At the start of implementation, amend the retained closure report status to indicate that post-closure review found release-path blockers. Preserve the original evidence; do not erase history.

Recommended status language:

```text
Reopened for corrective closure. CI simplification is complete; manual release validation remains incomplete pending Corrective Phase G and H.
```

After all Phase G acceptance criteria pass, record exact outcomes without marking the overall line closed until Phase H also passes.

## Implementation steps

1. Generate a full workspace package/dependency inventory from Cargo metadata.
2. Decide and document the exact crates.io package set; add explicit `publish = false` everywhere else.
3. Add path+version metadata to every publishable internal normal/build/optional dependency.
4. Add the package-graph helper and tests.
5. Generate and review the topological publication order.
6. Run `cargo metadata` and normal workspace checks to catch manifest regressions.
7. Redesign `release-check.sh` around one deterministic `cargo package` pass.
8. Add optional registry preflight without making it the default completion path.
9. Make hashing, size reporting, temporary paths, and workspace-import detection portable across Linux/macOS.
10. Update release and verification documentation from validated behavior.
11. Run the complete validation sequence on Linux.
12. Run the complete validation sequence on macOS, preferably the maintainer's Apple Silicon environment.
13. Record results in the closure report as Phase G evidence.

## Validation commands

### Manifest and graph

```bash
python scripts/release-package-graph.py list
python scripts/release-package-graph.py validate
python scripts/release-package-graph.py order
cargo metadata --format-version 1 --locked >/tmp/eggsec-metadata.json
```

### Standard verification

```bash
make check
make check-python
```

### Package validation

```bash
make release-check
```

Run with an explicit expected version:

```bash
scripts/release-check.sh "$(python3 - <<'PY'
import tomllib
with open('Cargo.toml', 'rb') as f:
    print(tomllib.load(f)['workspace']['package']['version'])
PY
)"
```

### Optional registry preflight

Only when network access and crates.io state allow:

```bash
EGGSEC_RELEASE_REGISTRY_PREFLIGHT=1 make release-check
```

This still must not publish.

### Static searches

```bash
rg -n 'eggsec-[a-z0-9-]+\s*=\s*\{[^}]*path\s*=' crates/*/Cargo.toml
rg -n 'publish\s*=\s*false' crates/*/Cargo.toml
rg -n 'cargo publish|maturin publish|twine upload' .github/workflows .gitlab-ci.yml 2>/dev/null || true
```

Review every path dependency result against the helper's validation; raw grep alone is not sufficient.

## Acceptance criteria

- Every workspace crate has an explicit publication classification.
- Every private crate declares `publish = false`.
- Every publishable crate's internal normal/build dependency is registry-resolvable with an appropriate version.
- Optional internal dependencies in published manifests are version-qualified.
- The package-graph helper validates the real workspace and has focused tests.
- The generated publication order is acyclic and places every internal dependency before its dependent.
- `docs/RELEASING.md` matches the generated package set and order.
- `cargo package` succeeds for every intended crates.io package in topological order.
- Packaged manifests contain no unresolved local-only normal/build dependencies.
- `make check` passes after manifest changes.
- `make check-python` passes after manifest changes.
- `make release-check` completes end-to-end on Linux without publishing.
- `make release-check` completes end-to-end on macOS without publishing, or one release host is explicitly designated and justified.
- The default release check does not duplicate `cargo package` and `cargo publish --dry-run` for every crate.
- Optional registry preflight failures or timeouts are reported as failures, not passes.
- Full diagnostics are retained for a failing package.
- Artifact hashing and size reporting work on Linux and macOS.
- Fresh-environment Python imports resolve to the built wheel, not the workspace source.
- The closure report no longer claims this phase passed before it actually did.
- No package, tag, or GitHub Release is created while executing the plan.

## Explicit non-goals

- Publishing the initial `0.1.0` release.
- Changing crate APIs or dependency architecture.
- Upgrading third-party dependencies.
- Splitting or merging crates.
- Adding automated crates.io publication.
- Adding release provenance or signing systems.
- Making Windows a supported release host unless maintainers explicitly choose it.
- Fixing unrelated compiler warnings.

## Rollback strategy

If publishing all library crates proves undesirable, explicitly reduce the crates.io package set with `publish = false`, then recalculate the graph and documentation. Do not retain manifests that appear publishable but cannot package. If registry dry-run remains unreliable, keep it optional and preserve deterministic `cargo package` validation as the local release-check contract.

## Handoff notes

A smaller implementation model should work crate-by-crate and run the graph validator after every manifest group. Do not manually edit the publication order before the dependency graph is valid. The final report must include the exact package order and a complete, successful `make release-check` outcome.