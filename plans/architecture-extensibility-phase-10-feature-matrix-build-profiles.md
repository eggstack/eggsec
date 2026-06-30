# Architecture Extensibility Phase 10: Feature Matrix and Build Profile Cleanup

## Objective

Clean up Cargo feature documentation, build-profile expectations, and metadata validation so optional Eggsec capabilities remain understandable, testable, and safe as the workspace continues to split domains into crates and use metadata-driven registration.

This phase is build hygiene. It should reduce stale comments, accidental feature coupling, unclear docs, and metadata feature-name drift without changing runtime authorization behavior.

## Current context

Eggsec has many optional feature groups:

- protocol/front-end adapters such as tool API, REST, WebSocket, and gRPC;
- compatibility layers such as NSE support;
- assessment domains such as database lab, web proxy, mobile lab, wireless lab, and other defense-lab modules;
- optional protocol exposure markers such as domain-specific MCP features;
- report/output features;
- storage, integration, and driver backends;
- platform-sensitive features that may require OS libraries or specific tools.

Feature gates are compile-time availability controls. They are not authorization. Runtime safety remains enforced through `ExecutionPolicy`, `OperationMetadata`, `EnforcementContext`, `ApprovedOperation`, and execution-surface strictness.

## Non-goals

- Do not remove capabilities merely because they are optional.
- Do not make optional feature presence count as runtime authorization.
- Do not add optional heavy dependencies to default builds.
- Do not rewrite all Cargo files unless necessary.
- Do not change user-facing command semantics except for clearer feature-missing diagnostics.
- Do not broaden default feature sets.

## Design target

Establish a feature ownership model:

- root workspace features stay minimal;
- main `eggsec` feature names are documented and stable;
- domain crates own internal dependency-driver features where possible;
- programmatic exposure markers are documented separately from base domain features;
- advanced/lab-only features are never enabled by broad/default profiles accidentally;
- tests cover representative feature combinations.

Feature metadata should be mechanically checkable where practical.

## Work item 1: Feature inventory and classification

Create a complete feature inventory.

Deliverable:

- Add `docs/FEATURE_MATRIX.md` or update an equivalent existing feature document.

For each feature, document:

- feature name;
- declaring crate;
- category:
  - protocol/front-end adapter;
  - domain capability;
  - domain protocol exposure marker;
  - report/output;
  - storage/integration;
  - backend/driver dependency;
  - platform-sensitive or lab-only capability;
- implied features/dependencies;
- whether it belongs in defaults;
- whether it affects programmatic exposure;
- related `OperationMetadata` IDs;
- related `DomainDescriptor` IDs.

Acceptance criteria:

- Maintainers can determine what enabling a feature does without reading every Cargo file.
- Domain base features are clearly distinguished from protocol exposure features.

## Work item 2: Remove stale phase-history comments from Cargo feature blocks

Cargo feature comments should explain stable build semantics, not preserve implementation history.

Required changes:

- Replace long phase-history comments with concise descriptions.
- Move detailed history to plan files or architecture docs when still useful.
- Keep durable safety-relevant comments:
  - feature availability is not authorization;
  - protocol exposure markers are opt-in where applicable;
  - runtime policy is still required.

Acceptance criteria:

- `Cargo.toml` feature comments are concise and current.
- Phase-history prose is not embedded in feature definitions.

## Work item 3: Normalize feature naming conventions

Review feature names for consistency.

Suggested conventions:

- base domain feature: `<domain>` or established existing names;
- protocol exposure feature: `<domain>-mcp` or similar established convention;
- backend driver feature: `<domain>-<backend>` or `<domain>-driver-<backend>`;
- advanced extension: `<domain>-advanced` where already established.

Do not rename features lightly. If a rename is necessary, keep a backward-compatible alias and document deprecation.

Acceptance criteria:

- Any inconsistent names are documented.
- No breaking feature rename is made without an alias.
- Naming convention is recorded in `docs/FEATURE_MATRIX.md`.

## Work item 4: Feature metadata validation tests

Add tests that validate features referenced by metadata are known.

Current metadata stores feature strings in `OperationMetadata`, `DomainDescriptor`, and integrations. Add a known-feature list or parse Cargo metadata if practical.

Preferred approach for this phase:

- Add a static `KNOWN_EGGSEC_FEATURES: &[&str]` in a test module.
- Validate every metadata feature string appears in that list.
- Validate every domain `required_feature` appears in that list.
- Validate every protocol-exposure feature string appears in that list.

More advanced approach:

- Parse `crates/eggsec/Cargo.toml` in tests using an existing or lightweight dev-dependency.
- Compare metadata feature strings directly against Cargo feature keys.

Acceptance criteria:

- Metadata cannot reference a misspelled feature silently.
- Optional protocol exposure feature names are validated.

## Work item 5: Build profile and test matrix documentation

Define the supported build/test profiles.

Recommended profiles:

- `minimal`: no default features;
- `manual-standard`: CLI/TUI plus standard manual workflows;
- `protocol`: tool API plus REST/gRPC/WebSocket as supported;
- `database-lab`: database domain and selected driver features;
- `mobile-lab`: mobile static and optional dynamic runtime workflows;
- `proxy-lab`: web proxy domain and optional protocol exposure marker;
- `advanced-lab`: explicitly opt-in advanced/lab-only features;
- `docs-metadata`: broad metadata/doc validation without heavy optional dependencies.

Document exact commands. Do not claim CI covers combinations it does not cover.

Acceptance criteria:

- A reader knows which feature combinations are intended to be supported.
- Heavy or platform-sensitive combinations are identified.

## Work item 6: Prepare CI guard commands

This phase should prepare for Phase 11 CI guards by documenting or adding reusable commands.

Possible additions:

- `scripts/check-features.sh` if the repo already uses scripts;
- Makefile targets if Makefiles already exist;
- documented `cargo check` command groups only, if adding scripts would be premature.

Do not add a new build system solely for this phase.

Acceptance criteria:

- Phase 11 can wire important no-default, metadata, and feature-profile checks into CI without re-inventing the matrix.

## Work item 7: Feature-to-metadata consistency in docs

Update docs so the following agree:

- `docs/CAPABILITY_MATRIX.md` feature column;
- `docs/FEATURE_MATRIX.md` feature list;
- `OperationMetadata.required_features`;
- `DomainDescriptor.required_feature`;
- Cargo feature definitions.

Acceptance criteria:

- A reader can trace a domain operation from docs to metadata to Cargo feature.
- Programmatic exposure markers are not confused with runtime authorization.

## Work item 8: Optional feature-missing diagnostics

Improve feature-disabled diagnostics where straightforward.

Required behavior:

- Feature-missing errors name the Cargo feature.
- Errors distinguish "not compiled" from "compiled but denied by policy".
- Programmatic surfaces return structured feature-missing errors where possible.

Acceptance criteria:

- At least one feature-disabled path has clearer diagnostics.
- No policy-denial behavior changes.

## Safety requirements

- Advanced/lab-only features remain opt-in.
- Programmatic exposure markers remain opt-in where already designed.
- Feature presence never bypasses policy enforcement.
- Metadata validation must not require optional heavy dependencies to compile.
- Docs must distinguish build availability from authorization.

## Files likely to change

- `crates/eggsec/Cargo.toml`
- workspace `Cargo.toml` if workspace-level comments need updates
- domain crate Cargo files if feature comments are inconsistent
- `crates/eggsec/src/config/policy.rs`
- `crates/eggsec/src/domain/mod.rs`
- `crates/eggsec/tests/metadata_consistency.rs`
- optionally `crates/eggsec/tests/feature_matrix.rs`
- `docs/FEATURE_MATRIX.md`
- `docs/CAPABILITY_MATRIX.md`
- `docs/METADATA_OWNERSHIP.md`
- optional scripts or CI docs

## Validation commands

Run:

```bash
cargo fmt --all --check
cargo check --workspace --no-default-features
cargo test -p eggsec --test metadata_consistency
cargo test -p eggsec --lib
```

Representative feature checks should cover:

- protocol/front-end adapters;
- database domain;
- mobile static and dynamic domain features;
- web proxy domain;
- domain-specific protocol exposure markers;
- selected advanced/lab-only feature combinations where platform dependencies are available.

Record exact commands in `docs/FEATURE_MATRIX.md`. If platform-sensitive checks fail due to missing system dependencies, document the expected environment rather than hiding the issue.

## Completion criteria

Phase 10 is complete when:

- A feature matrix doc exists and is current.
- Cargo feature comments are concise and stable.
- Metadata feature strings are validated against known features or parsed Cargo feature keys.
- Programmatic exposure markers are documented separately from base domain features.
- Supported build/test profiles are documented.
- Feature-disabled diagnostics are clearer where touched.
- No optional advanced feature becomes default or implicitly authorized.

## Handoff note

This phase should leave the project ready for CI architecture guards. The next phase should wire the most important no-default, metadata, enforcement, and feature-profile checks into CI so future changes cannot regress silently.
