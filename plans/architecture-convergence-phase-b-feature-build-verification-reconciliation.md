# Phase B Plan: Feature, Build, and Verification Reconciliation

## Status

Status: Ready for implementation.

## Objective

Make the declared Cargo feature graph, runtime feature registry, aggregate build profiles, documentation, and verification commands describe the same system.

The current repository already has a strong fail-closed runtime feature registry, but there are still observable mismatches between source and documentation and between the conceptual meaning of aggregate features and their actual Cargo membership. This phase corrects those mismatches without introducing an exhaustive combinatorial CI matrix.

## Baseline files

```text
Cargo.toml
crates/eggsec/Cargo.toml
crates/*/Cargo.toml
crates/eggsec/src/config/feature_registry.rs
crates/eggsec/tests/feature_matrix.rs
crates/eggsec/tests/metadata_consistency.rs
Makefile
.github/workflows/ci.yml
.github/workflows/deep-checks.yml
docs/FEATURE_MATRIX.md
docs/BUILD.md
docs/VERIFICATION.md
docs/CAPABILITY_MATRIX.md
README.md
scripts/check-architecture-guards.sh
```

## Confirmed residuals to reconfirm on implementation head

- `docs/FEATURE_MATRIX.md` describes an empty main-crate default feature set while `crates/eggsec/Cargo.toml` currently declares `default = ["cli"]`.
- the `full` aggregate is described as a broad/all-capabilities build but does not contain every declared main-crate feature;
- representative profile compilation covers many important domains but not every declared feature individually;
- routine clippy is concentrated on the main `eggsec` library even though extracted crates now own substantial implementation code.

## Non-goals

This phase does not require every arbitrary feature combination to compile. It does not make `full` the default. It does not move weekly/platform checks into every PR. It does not remove a feature because it is expensive to build. It does not add a new external build system.

## Workstream 1 — Define exact feature classes and aggregate semantics

For every main-crate feature classify it as one of:

- capability/domain;
- protocol adapter;
- protocol exposure marker;
- backend/driver;
- process-host adapter;
- test/internal marker;
- platform-sensitive extension;
- aggregate/meta feature.

Then define what `full` means. Choose one of two explicit contracts:

1. `full` literally enables every supported non-test main-crate feature that can coexist on the primary Linux build environment; or
2. rename/document it as a curated `full-lab`/`standard-full` aggregate and maintain a separate mechanically checked exhaustive list for individual feature compilation.

Do not leave “full” semantically ambiguous.

If features are intentionally incompatible, capture those relationships in a small typed/tested table with a reason. Do not rely on tribal knowledge.

## Workstream 2 — Make registry/Cargo coverage mechanical

Retain `feature_registry.rs` as the runtime source for fail-closed availability if it continues to fit the architecture, but strengthen bidirectional validation:

- every Cargo feature except `default` must appear exactly once in the registry or in an explicit exempt set (`full`, test-only markers if handled separately);
- every registry name must exist in Cargo;
- every feature referenced by `OperationMetadata`, domain descriptors, command metadata, runtime capability declarations, or protocol registrations must be known;
- aggregate feature membership should be checked against its declared contract;
- feature aliases should not exist unless Cargo itself supports/declares the relationship.

Prefer parsing Cargo metadata/TOML in a test or small script over duplicating the list yet again in Rust test constants.

## Workstream 3 — Add direct per-feature compilation sweep

Create a maintained deep-check command that compiles each declared feature individually or in the minimum dependency set needed to activate it.

Conceptual command surface:

```text
make check-features-individual
```

The implementation may use a script that reads Cargo metadata and a small exceptions map for features that require companion markers/system tools.

Requirements:

- run weekly/manual in `deep-checks.yml`, not necessarily on every PR;
- fail if a newly declared feature is not classified into a direct compile profile;
- record prerequisite failures separately from Rust compile failures where practical;
- include domain crate feature sweeps for `eggsec-nse`, `eggsec-db-lab`, `eggsec-web-proxy`, `eggsec-mobile-lab`, daemon/CLI feature sets, and Python Cargo feature profiles where applicable;
- keep the existing fast representative profiles for pre-release unless measurements show the individual sweep is cheap enough to include there.

Do not use `cargo check --all-features` as the sole oracle.

## Workstream 4 — Workspace lint ownership

Expand lint coverage so extracted implementation crates do not depend on ad hoc audits for warnings.

Recommended split:

- routine `make check`: retain main engine `-D warnings` plus cheap leaf crates (`eggsec-core`, `eggsec-tool-core`, `eggsec-output`, `eggsec-runtime`, `eggsec-ui-model`, `eggsec-agent`) if runtime is acceptable;
- deep checks: clippy domain/platform crates with their relevant features;
- Python: retain its dedicated checks.

Use `cargo clippy --workspace --no-default-features` only if it is demonstrably clean/fast and does not produce feature-induced false positives. Otherwise maintain ownership-oriented groups.

## Workstream 5 — Documentation generation/validation

Stop hand-maintaining source facts that Cargo can provide.

At minimum mechanically validate or generate:

- default features;
- all declared main-crate features;
- category and aggregate membership;
- direct build/check example for each maintained feature class;
- domain crate feature inventory.

Human documentation may add explanations, risk notes, prerequisites, and surface exposure, but source-level membership must come from machine-readable declarations.

If generation is introduced, keep generated output deterministic and easy to update with one command. Do not add a service or complex code-generation pipeline.

## Workstream 6 — Reconcile `full` and release checks

After defining `full`, update:

- Cargo aggregate membership;
- `docs/FEATURE_MATRIX.md`;
- `docs/BUILD.md`;
- `docs/VERIFICATION.md`;
- README examples;
- release-check scripts if they use `full` as an implicit completeness signal.

Pre-release verification should continue to use curated representative profiles plus the direct individual feature sweep where appropriate. Explicitly state that unsupported incompatible combinations are not release blockers.

## Workstream 7 — Regression tests

Add tests for:

- Cargo feature <-> feature registry bidirectional equality;
- all metadata feature references are known;
- aggregate membership contract;
- docs/source default-feature consistency if docs remain hand-authored;
- the individual feature profile map has no orphan feature;
- platform/backend feature prerequisites are classified rather than silently skipped.

## Acceptance criteria

- no feature/default claim in active documentation contradicts Cargo;
- `full` has a precise, mechanically tested definition;
- every main-crate feature is represented in the runtime registry or explicitly exempted for a documented reason;
- every feature is directly compiled in a maintained profile;
- extracted implementation crates receive lint coverage at an appropriate cadence;
- routine CI does not become a combinatorial feature matrix;
- `make check`, `make check-full`, and the new individual-feature sweep pass on the supported environment or report only explicitly documented system prerequisites;
- manual release/publication behavior remains unchanged.

## Completion record

Append baseline/final SHA, feature-count totals, aggregate semantics chosen, direct sweep command, runtime measurements, and verification results.