# Phase J Plan: Measurement, Documentation Reconciliation, and Closure

## Status

Ready for implementation after Phases A–I.

## Objective

Close the dependency, architecture, and verification simplification line with
measured artifact/dependency results, current architecture documentation, and a
small direct validation record.

This is a closure and documentation phase. It must not become another feature,
refactor, evidence-bundle, or verification-framework phase.

## Preconditions

All prior phases must either be complete or explicitly recorded as partially
deferred with a concrete technical blocker. Phase J must not label an unexecuted
or blocked item as complete.

## Scope

Primary files and outputs:

```text
plans/dependency-architecture-simplification-roadmap.md
plans/dependency-architecture-phase-*.md
architecture/overview.md
architecture/runtime_bridge.md
architecture/python_api.md
docs/BUILD.md
docs/VERIFICATION.md
docs/RELEASING.md
docs/DAEMON.md
docs/FEATURE_MATRIX.md
docs/METADATA_OWNERSHIP.md
docs/COMMAND_REGISTRY.md
docs/TOOL_REGISTRATION.md
docs/CI_ARCHITECTURE_GUARDS.md
docs/EXTENSIBILITY.md
docs/extending/
AGENTS.md
README.md
Cargo.toml
Cargo.lock
.github/workflows/
```

A single closure report may be added under `plans/` if useful. Do not create
multiple evidence artifacts.

## Non-goals

This phase does not:

- implement missing earlier-phase work;
- add new capabilities;
- perform another broad dependency upgrade;
- tune code based only on one size number;
- impose exact binary-size CI gates;
- publish packages;
- run registry preflight unless maintainers separately choose to do so;
- create generated evidence bundles, JSON schemas, or artifact archives;
- rewrite historical executed plans.

## Workstream 1 — Confirm phase status honestly

Review every phase plan and classify:

```text
Executed
Partially executed with accepted deferment
Blocked
Superseded
Not started
```

For executed phases, add a short status/outcome section to the phase plan or link
to the implementation commit/PR if that is the repository convention.

For deferred items, record:

- exact remaining item;
- why it is not required for roadmap closure;
- owner/dependency;
- what would trigger reopening.

Security correctness items from Phases A–C cannot be silently deferred. If they
are incomplete, the roadmap remains open.

## Workstream 2 — Capture final artifact profiles

Build and record the final supported profiles using the same toolchain and host
where possible:

```bash
cargo build -p eggsec-cli --release
cargo build -p eggsec-cli --release --no-default-features
cargo build -p eggsec-cli --release --no-default-features --features daemon-client
cargo build -p eggsec-daemon --release
cargo build -p eggsec-python --release
```

For the Python extension, use the actual maturin release artifact when the direct
Cargo output is not representative:

```bash
cd crates/eggsec-python
maturin build --release
```

Record for each artifact:

```text
profile/features
artifact name/path
host/architecture
rustc/cargo version
file size
whether stripped
notable native libraries or external runtime requirements
```

Do not compare artifacts built with different stripping/profile settings without
labeling the difference.

## Workstream 3 — Capture final dependency topology

Run and retain concise summaries from:

```bash
cargo tree -p eggsec -e features
cargo tree -p eggsec-cli -e features
cargo tree -p eggsec-cli --no-default-features -e features
cargo tree -p eggsec-daemon -e features
cargo tree -p eggsec-python -e features
cargo tree -d
```

The closure record should state:

- whether CLI-only dependencies are absent from engine/Python graphs;
- whether server SQLite is absent from client/TUI graphs;
- selected Rustls provider per artifact;
- remaining duplicate major generations and their owners;
- remaining native/system dependencies and owning features;
- optional domains that remain intentionally heavy.

Do not paste full multi-thousand-line trees into documentation. Summaries plus
commands are sufficient.

## Workstream 4 — Compare against baselines

Use the baselines recorded in Phases F and G. Report deltas for:

```text
standard interactive CLI/TUI size
headless CLI size
Python extension/wheel size
daemon server size
transitive crate counts
duplicate major generations
native/build dependency reachability
mandatory CI jobs/invocations
architecture guard count
```

Explain regressions or unchanged metrics. The roadmap may still succeed when a
particular artifact does not shrink if the dependency boundary is corrected and
capability is preserved.

Avoid unsupported percentage precision. Use direct byte/megabyte and count
deltas from reproducible commands.

## Workstream 5 — Validate authorization and metadata end state

Confirm directly that:

- target-required operations cannot be approved without a target;
- approval tokens bind canonical operation and target;
- address-set scope behavior is deterministic;
- unknown feature names fail closed;
- one canonical operation catalog owns risk/mode/target/features/exposure;
- command/domain/tool/runtime/Python views derive from that catalog;
- hazardous operations are not accidentally default-visible to strict automated
  profiles.

Use existing tests from earlier phases. Do not add a new closure-only test suite
that duplicates them.

## Workstream 6 — Validate dependency security and MSRV end state

Record:

```bash
cargo deny check advisories
cargo +<declared-msrv> check --workspace --no-default-features
cargo +stable check --workspace --no-default-features
```

Summarize retained advisory exceptions with review dates. Confirm active docs and
manifests agree on MSRV and release-tool requirements.

If an optional backend remains on an old line, record the exact blocker and why
it does not invalidate core closure.

## Workstream 7 — Validate final CI/release boundaries

Inspect workflows and run final local commands:

```bash
make check
make check-python
make check-full
rg -n 'cargo publish|maturin publish|twine upload|gh release|id-token: write|tags:' .github/workflows
```

`make release-check` may be run when manifest/package topology changed and the
maintainer release environment is available. It must not publish.

Record:

- mandatory workflow/job shape;
- optional workflow shape;
- Python build count;
- portability policy;
- release publication remains manual;
- registry preflight/publication status as `NOT RUN` unless actually performed.

## Workstream 8 — Reconcile architecture documentation

Update active documents to reflect the final design:

### Architecture overview

- final workspace crate count and roles;
- engine/application separation;
- operation catalog ownership;
- approval/target/address binding;
- runtime/daemon client/server split;
- Python adapter boundary.

### Build documentation

- standard/headless/daemon/Python/full profiles;
- optional feature/system dependency ownership;
- Rustls provider policy;
- MSRV.

### Metadata/extensibility documentation

- one operation-addition workflow;
- one feature-addition workflow;
- derived command/domain/tool views;
- no transitional pilot terminology.

### Verification documentation

- compact mandatory command;
- optional diagnostics;
- retained architecture guards and defect ownership;
- manual release boundary.

Use one canonical document per topic and links elsewhere. Remove repeated stale
lists.

## Workstream 9 — Clean obsolete transition artifacts

Search for:

```text
Phase/pilot terminology presented as current architecture
registry_backed or LegacyWrapped references
old independent feature snapshots
old daemon-client/server dependency claims
old CLI-in-engine documentation
stale MSRV and Cargo requirement claims
deleted workflow/test/guard names
```

Delete obsolete scripts/tests/docs only when their purpose is fully superseded.
Retain historical plans with executed/superseded status.

Do not delete useful handoff history from `plans/`.

## Workstream 10 — Produce one concise closure report

Add, if needed:

```text
plans/dependency-architecture-simplification-closure-report.md
```

Recommended sections:

1. scope and final status;
2. implementation commits/PRs by phase;
3. confirmed correctness outcomes;
4. artifact/dependency before/after table;
5. retained native and duplicate dependencies with reasons;
6. advisory/MSRV status;
7. CI/release status;
8. deferred non-blocking items;
9. exact validation commands and outcomes;
10. publication status.

Do not include raw logs, generated archives, screenshots, or fabricated hosted
run conclusions.

## Required final validation

Run against one clean exact commit:

```bash
git rev-parse HEAD
git status --porcelain
rustc --version --verbose
cargo --version --verbose
python3 --version
make check
make check-python
make check-full
cargo deny check advisories
cargo +<declared-msrv> check --workspace --no-default-features
```

Add focused artifact builds and `cargo tree` commands from Workstreams 2 and 3.

Classification rules:

```text
successful completed command -> PASS
command failed -> FAIL
command timed out -> TIMEOUT
required tool/environment unavailable -> BLOCKED or NOT RUN
not requested/manual publication operation -> NOT RUN
```

Never convert failure, timeout, skip, or absence into PASS.

## Rollback considerations

Phase J should contain documentation, plan status, and narrowly scoped cleanup.
If a cleanup deletion breaks validation, restore that file and mark it retained
with an owner. Do not reopen architectural implementation merely to improve the
closure narrative.

## Acceptance criteria

1. Every phase has an honest final status.
2. Security correctness phases A–C are executed and passing.
3. Metadata ownership is single-source and current docs match it.
4. Standard, headless, daemon, and Python artifacts are built and measured.
5. Before/after size and dependency deltas are recorded consistently.
6. CLI-only dependencies are absent from engine/Python graphs by default.
7. daemon server persistence is absent from client/TUI graphs by default.
8. Rustls provider selection is documented per primary artifact.
9. Remaining duplicate major generations have explicit owners/reasons.
10. Remaining native/external dependencies have feature/artifact ownership and
    justification.
11. advisory exceptions are current and time-bounded.
12. the declared MSRV passes a direct workspace check.
13. `make check`, `make check-python`, and `make check-full` outcomes are recorded
    against one exact clean commit.
14. mandatory CI and optional deep checks match current verification docs.
15. no hosted workflow contains package publication or tag-triggered release
    mutation.
16. release cadence and publication remain manual.
17. active docs contain no stale transition/pilot/registry claims.
18. historical plan files are retained and status-marked.
19. at most one concise closure report is added.
20. registry preflight and publication are labeled `NOT RUN` unless actually
    performed.
21. no package or release is published during closure.
