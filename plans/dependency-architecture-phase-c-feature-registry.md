# Phase C Plan: Exhaustive Compile-Time Feature Registry

## Status

Executed. Changes landed in main.

Phase C is fully implemented. The `feature_registry!` macro in
`config/feature_registry.rs` generates `ALL_FEATURES`, `feature_state()`,
`is_feature_enabled()`, `is_known_feature()`, `feature_missing_hint()`, and
`classify_feature()`. Unknown features return `false`/`FeatureState::Unknown`
(fail-closed). A bidirectional test validates every Cargo.toml feature is in the
registry and vice versa. Domain availability and policy checks delegate to the
unified registry.

## Objective

Replace fail-open and manually duplicated compile-time feature detection with one
exhaustive definition used by production policy checks, domain availability,
diagnostics, metadata validation, and tests.

The implementation must make an unknown or misspelled feature unavailable by
default and make it difficult to add a Cargo feature without updating the exact
runtime availability path used by authorization.

## Current problem

The repository currently represents feature knowledge in several places:

```text
crates/eggsec/Cargo.toml [features]
config::policy_decision::is_feature_enabled()
config::policy_decision::is_known_feature()
domain::feature_enabled()
domain::feature_missing_hint()
crates/eggsec/tests/feature_matrix.rs static snapshots
command registration feature strings
operation metadata required_features
domain descriptors and tool exposure markers
frontend feature-forwarding manifests
Python feature metadata
```

The production matcher defaults unknown names to available. Tests validate a
separate static list rather than the production matcher itself. This permits a
metadata typo or omitted match arm to pass preflight/policy checks.

## Scope

Primary files:

```text
crates/eggsec/Cargo.toml
crates/eggsec/src/config/policy_decision.rs
crates/eggsec/src/config/policy.rs
crates/eggsec/src/domain/mod.rs
crates/eggsec/src/commands/registry.rs
crates/eggsec/src/tool/registration.rs
crates/eggsec/tests/feature_matrix.rs
crates/eggsec-cli/Cargo.toml
crates/eggsec-tui/Cargo.toml
crates/eggsec-python/Cargo.toml
crates/eggsec-python/src/
docs/FEATURE_MATRIX.md
docs/extending/features.md
```

The phase should cover the `eggsec` feature namespace first. Other crate-local
features should be represented separately rather than incorrectly pretending all
workspace features belong to one crate.

## Non-goals

This phase does not:

- change which capabilities each feature enables except to correct proven wiring
  defects;
- remove useful optional features;
- introduce a build-time code generator that requires nonstandard tooling;
- parse Cargo metadata at runtime;
- make the `full` aggregate conservative or default;
- redesign operation/domain metadata beyond the feature fields needed for Phase
  D;
- add exhaustive all-feature CI matrices.

## Preferred implementation model

Use one in-source declarative definition, implemented through a macro or a
single static table plus generated match logic. Example shape:

```rust
feature_registry! {
    ToolApi => {
        cargo: "tool-api",
        enabled: cfg!(feature = "tool-api"),
        category: ProtocolAdapter,
        hint: "enable feature 'tool-api'",
    },
    DbPentest => {
        cargo: "db-pentest",
        enabled: cfg!(feature = "db-pentest"),
        category: DomainCapability,
        hint: "enable feature 'db-pentest'",
    },
}
```

The exact macro syntax is not prescribed. The definition must support:

- stable Cargo feature string;
- compile-time enabled state;
- feature category where still useful;
- user-facing enablement hint;
- dependency/parent relationships where those are needed by diagnostics;
- whether it is an aggregate, exposure marker, backend driver, platform-sensitive
  feature, or security-risk feature.

Do not duplicate Cargo's complete dependency solver in Rust. Feature dependency
relationships should be retained only where they support useful diagnostics or
validation that Cargo cannot express.

## Workstream 1 — Build an authoritative feature inventory

Extract the current `[features]` table and classify each entry. Confirm every
feature referenced by:

```bash
rg -n 'required_features|feature: Some|cfg!\(feature|cfg\(feature' crates/eggsec
```

Inventory at least:

- canonical feature name;
- Cargo definition;
- direct optional dependencies activated;
- parent/base feature dependencies;
- whether it is referenced by operation metadata;
- whether it is referenced by domain/tool/command metadata;
- which frontend manifests forward it;
- whether it is marker-only or behavior-bearing.

Resolve known discrepancies before writing the registry. Do not copy the current
static test snapshot without verification.

## Workstream 2 — Implement fail-closed lookup

Replace `is_feature_enabled(&str) -> bool` behavior so:

```text
known enabled feature -> true
known disabled feature -> false
unknown feature -> false or typed UnknownFeature error
```

Authorization/preflight code should distinguish:

- known but disabled feature;
- unknown metadata feature, which indicates a repository defect.

Strict execution should deny both. Diagnostics should clearly identify unknown
metadata rather than presenting it as a normal user-disabled feature.

A fallible API is preferred for metadata validation:

```rust
fn feature_state(name: &str) -> Result<FeatureState, UnknownFeature>
```

A convenience boolean may remain for UI availability if it fails closed.

## Workstream 3 — Unify domain and policy availability

Remove the independent domain `feature_enabled()` and
`feature_missing_hint()` match statements. Domain descriptors, operation
metadata, and tool registration must consume the same registry.

Ensure features previously omitted from production policy matching are covered,
including at minimum the current domain and advanced markers such as:

```text
db-pentest
web-proxy
mobile-dynamic
evasion
postex
c2
wireless-advanced
transparent-proxy
dynamic-plugins
daemon-client
```

The final list must come from the actual manifest, not this plan.

## Workstream 4 — Replace test snapshots with production-backed validation

Refactor `feature_matrix.rs` so it validates the authoritative registry rather
than maintaining another complete feature list.

Required tests:

1. every Cargo feature except `default` is represented in the registry;
2. every registry feature exists in Cargo.toml;
3. every operation `required_features` entry resolves through production
   lookup;
4. every domain feature and MCP exposure feature resolves;
5. every command feature resolves;
6. unknown feature lookup fails closed;
7. known disabled feature reports disabled;
8. known enabled feature reports enabled under representative feature builds;
9. aggregate features contain required domain features where that remains a
   documented contract;
10. frontend-forwarded feature names reference real engine features.

Parsing the crate's own Cargo.toml in a test is acceptable. Avoid manually
mirroring the full feature list or dependency graph.

## Workstream 5 — Validate feature forwarding

Check `eggsec-cli`, `eggsec-tui`, and `eggsec-python` feature forwarding for:

- missing engine forwards;
- forwarded names that no longer exist;
- features that accidentally activate heavy optional dependencies locally and
  through the engine twice;
- aggregate features that omit a documented capability;
- marker features documented as dependency-free but actually activating large
  stacks.

Correct wiring defects narrowly. Do not expand the default feature set.

## Workstream 6 — Expose feature state to Python from the same source

Python `_feature_guard` and capability introspection should derive from engine
feature state where possible. If the extension has wrapper-local features,
combine two explicit registries:

```text
engine feature state
binding-local feature state
```

Do not maintain a third hand-written list of engine feature availability in
Python code or test fixtures.

Preserve stable Python names and exception/result behavior unless a current name
is demonstrably wrong.

## Workstream 7 — Documentation and contributor guidance

Update `docs/FEATURE_MATRIX.md` and `docs/extending/features.md` to describe:

- the authoritative registry location;
- how to add an engine feature;
- how frontend forwarding is validated;
- how unknown feature metadata fails;
- the difference between a domain capability, protocol exposure marker, backend
  driver, platform-sensitive feature, and aggregate feature;
- that adding a feature does not require editing multiple static snapshots.

Do not duplicate the full generated matrix in multiple documents.

## Validation commands

```bash
cargo fmt --all -- --check
cargo test -p eggsec --test feature_matrix
cargo test -p eggsec --test metadata_consistency
cargo test -p eggsec --test command_registry
cargo test -p eggsec --test tool_registration --features rest-api
cargo check --workspace --no-default-features
cargo check -p eggsec --features db-pentest
cargo check -p eggsec --features web-proxy
cargo check -p eggsec --features mobile-dynamic
cargo check -p eggsec --features c2
make check-python   # when Python feature introspection changes
```

Use a small representative set of enabled-feature checks. Do not introduce an
exhaustive Cartesian product.

## Migration strategy

1. Add the canonical registry and tests while old helpers still exist.
2. Route policy lookup through the registry and change unknown behavior to fail
   closed.
3. Route domain diagnostics through the registry.
4. Route command/tool validation through the registry.
5. Route Python introspection where applicable.
6. Delete old snapshots and independent match statements.
7. Update documentation.

Do not leave compatibility fallbacks that return `true` for unknown names.

## Rollback considerations

If the macro implementation proves difficult to maintain, use a single static
registry plus one explicit exhaustive `match`. The rollback may change the code
shape but must retain one authoritative list and fail-closed unknown behavior.

## Acceptance criteria

1. Unknown feature names are unavailable in production policy checks.
2. Unknown metadata produces a distinct diagnostic or test failure.
3. Every Cargo feature is represented once in the authoritative registry.
4. Every operation feature reference uses production lookup.
5. Every domain/command/tool feature reference uses the same lookup.
6. Domain availability hints are generated from the same definition.
7. The old independent `is_known_feature`/domain matcher snapshots are removed
   or reduced to wrappers around the registry.
8. `feature_matrix.rs` no longer manually mirrors the complete feature list.
9. Frontend feature forwarding is validated against actual engine features.
10. Python engine-feature introspection does not maintain a divergent list.
11. Representative no-default and enabled-feature builds pass.
12. No default feature is added and no capability is removed.
13. No package or release is published.
