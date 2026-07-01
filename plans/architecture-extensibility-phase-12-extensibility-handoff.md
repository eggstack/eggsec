# Architecture Extensibility Phase 12: Extensibility Handoff and Contributor Model

## Objective

Create the final extensibility handoff layer for Eggsec so maintainers and contributors can add new operations, domains, commands, tool integrations, TUI actions, report/evidence outputs, feature gates, and documentation without violating the architecture invariants now guarded by Phase 11 CI.

This phase is primarily documentation, templates, examples, and consistency checks. It should turn the architecture work from an internal refactor into a durable contributor model.

## Current context

The architecture/extensibility roadmap has established the following project model:

- `OperationMetadata` is the canonical operation policy metadata layer.
- `DomainDescriptor` is the canonical domain/integration grouping layer.
- `ToolRegistration` is the canonical protocol/tool listing bridge.
- MCP OpsAgent uses Model A: profile-expanded `mcp_metadata_exposable` listing, not conservative default visibility.
- `mcp_tool_registrations_default_visible()` is the conservative default subset.
- `CommandRegistration` separates CLI visibility, TUI visibility, programmatic visibility, CLI-helper-only behavior, registry-backed dispatch, and dispatch mode.
- TUI action specs should point back to canonical operation metadata and not invent policy/risk semantics.
- Strict surfaces must use `EnforcementContext::evaluate()` and `ApprovedOperation` before dispatch.
- Manual CLI/TUI surfaces may preserve operator discretion, but should still use shared preflight/metadata where possible.
- Feature metadata is validated against `crates/eggsec/Cargo.toml`.
- Report/evidence output has a normalized envelope in `eggsec-output`.
- Phase 11 CI guards now enforce the critical invariants.

Phase 12 should document this as a repeatable extension process.

## Non-goals

- Do not add a new operation/domain solely as part of this phase unless it is a minimal documentation example and not compiled behavior.
- Do not broaden protocol exposure.
- Do not weaken manual/strict enforcement separation.
- Do not change MCP Model A semantics.
- Do not disable or loosen Phase 11 guards.
- Do not move domain authorization into domain crates.
- Do not convert remaining legacy commands to registry-backed dispatch as part of this phase; document the migration path instead.

## Design target

Add a contributor-facing extensibility guide set that answers these questions:

1. How do I add a new operation?
2. How do I add a new domain crate or domain descriptor?
3. How do I add a CLI command?
4. How do I make a command registry-backed instead of legacy-wrapped?
5. How do I expose a tool through MCP, REST, gRPC, or agent surfaces?
6. How do I add a TUI tab/action without duplicating policy semantics?
7. How do I add report/evidence output using `eggsec-output`?
8. How do I add a Cargo feature and keep the feature matrix valid?
9. What tests must I add?
10. What local/CI checks should I run before handoff?

The output should be practical and mechanical: tables, checklists, code skeletons, and file ownership maps.

## Work item 1: Add an extensibility index document

Create:

- `docs/EXTENSIBILITY.md`

Purpose:

- Serve as the top-level contributor entry point for extending Eggsec.
- Link to specific guides for operations, domains, commands, tool exposure, TUI, reports/evidence, features, and tests.
- Summarize the architectural invariants in one concise section.

Required sections:

1. Overview
2. Core invariants
3. Extension decision tree
4. File ownership map
5. Required local checks
6. Links to detailed guides

Suggested decision tree:

```text
Do you need a new security action?
  -> Add OperationMetadata first.
  -> Decide whether it belongs to an existing domain or a new DomainDescriptor.
  -> Add command/tool/TUI/report integration only after metadata exists.

Do you need a new protocol-exposed tool?
  -> Add/verify OperationMetadata.
  -> Add ToolRegistration or domain ToolIntegration.
  -> Ensure strict dispatch uses ApprovedOperation.

Do you need a new manual CLI command?
  -> Add CommandRegistration.
  -> Choose RegistryBacked, LegacyWrapped, HelperOnly, ServerLifecycle, or CatalogOnly.
  -> Wire preflight/enforcement appropriately.
```

Acceptance criteria:

- `docs/EXTENSIBILITY.md` exists and links all new detailed guides.
- It explicitly states metadata-first extension.
- It explicitly states domains do not authorize work.

## Work item 2: Add operation metadata guide

Create:

- `docs/extending/operations.md`

Required content:

- What `OperationMetadata` owns.
- How operation IDs and aliases work.
- How to select `OperationRisk`.
- How to select required `Capability` values.
- How to set exposure flags:
  - `manual_exposable`
  - `tui_exposable`
  - `mcp_exposable`
  - `rest_exposable`
  - `grpc_exposable`
  - `agent_exposable`
- How to set `required_features`.
- How metadata interacts with `OperationDescriptor`.
- Required tests.

Include a skeleton example, using clearly fake placeholder names so it cannot be mistaken for a real capability:

```rust
OperationMetadata {
    id: "example-check",
    display_name: "Example Check",
    aliases: &["example"],
    mode: OperationMode::Active,
    risk: OperationRisk::SafeActive,
    required_capabilities: &[Capability::ActiveProbe],
    required_features: &[],
    manual_exposable: true,
    tui_exposable: true,
    mcp_exposable: true,
    rest_exposable: true,
    grpc_exposable: false,
    agent_exposable: true,
}
```

Required warnings:

- Do not mark a tool programmatic-exposable just because it is manual-exposable.
- Do not use `mcp_exposable` to mean default-visible.
- High-risk operations require explicit capabilities and strict runtime approval.

Acceptance criteria:

- Guide gives enough information to add metadata without reading historical plans.
- Guide names the tests that must be updated or expected to fail.

## Work item 3: Add domain extension guide

Create:

- `docs/extending/domains.md`

Required content:

- What `DomainDescriptor` owns.
- When to add a new domain versus extend an existing one.
- How `required_feature` works.
- How domain operations map to global `OperationMetadata`.
- How domain `ToolIntegration` affects MCP default visibility and opt-in features.
- How `all_domain_descriptors()` and `available_domain_descriptors()` differ.
- Required tests.

Required invariant:

Domains may declare and group operations, but they must not decide authorization. Authorization remains in `EnforcementContext` and dispatch approval tokens.

Include checklist:

- Add or update domain descriptor.
- Add operation metadata for every domain operation.
- Add feature gate if optional.
- Add tool integration only when protocol listing is needed.
- Add docs and tests.
- Run metadata/feature/tool registration tests.

Acceptance criteria:

- Guide distinguishes compile-time feature availability from metadata declaration.
- Guide explicitly describes MCP default visibility versus opt-in domain exposure.

## Work item 4: Add command registry guide

Create:

- `docs/extending/commands.md`

Required content:

- What `CommandRegistration` owns.
- Meaning of:
  - `cli_visible`
  - `tui_visible`
  - `programmatic_visible`
  - `cli_interactive_only`
  - `registry_backed`
  - `dispatch_mode`
- Meaning and selection criteria for `CommandDispatchMode` variants:
  - `RegistryBacked`
  - `LegacyWrapped`
  - `CatalogOnly`
  - `ServerLifecycle`
  - `HelperOnly`
- How to add a new CLI command.
- How to migrate a command from `LegacyWrapped` to `RegistryBacked`.
- How descriptors are built and used for preflight/enforcement.
- Required tests.

Required warnings:

- `cli_interactive_only` does not mean all human-interactive surfaces; TUI visibility is separate.
- Registry metadata is not authorization.
- Side-effecting registry-backed commands must build descriptors.

Acceptance criteria:

- A contributor can choose a dispatch mode without guessing.
- Guide explains manual CLI/TUI discretion versus programmatic strictness.

## Work item 5: Add protocol/tool exposure guide

Create:

- `docs/extending/tool-exposure.md`

Required content:

- What `ToolRegistration` owns.
- Difference between runtime tool registry and canonical metadata registration.
- Difference between:
  - `mcp_metadata_exposable`
  - `mcp_default_visible`
  - `required_mcp_feature`
  - `rest_exposable`
  - `grpc_exposable`
  - `agent_exposable`
- MCP Model A explanation.
- CodingAgent allowlist behavior.
- REST/gRPC listing versus execution checks.
- Agent dispatch checks.
- How to add a tool integration through a domain descriptor.
- How to expose a base tool.
- Required tests.

Required invariant:

Listing is not authorization. Execution still requires shared enforcement and approved dispatch.

Acceptance criteria:

- Guide prevents future conflation of MCP metadata exposure and default visibility.
- Guide states OpsAgent is expanded metadata-exposable, not conservative default.

## Work item 6: Add TUI extension guide

Create:

- `docs/extending/tui-actions.md`

Required content:

- How TUI tabs/actions are declared.
- How `TuiActionSpec` should reference canonical operation metadata.
- How TUI posture maps manual and strict modes.
- How shared policy preflight is used.
- How direct-launch gates should obtain approval before dispatch.
- How TUI theme/startup behavior should remain non-blocking.
- Required tests.

Required warnings:

- Do not duplicate risk/capability/scope semantics in TUI.
- Do not block visible TUI startup on optional files/themes/resources.
- Do not make TUI stricter than agent mode by accident, but preserve explicit strict posture mode.

Acceptance criteria:

- Guide aligns TUI extension with existing action specs and enforcement facade.
- Guide preserves manual operator discretion while keeping shared metadata/preflight.

## Work item 7: Add report/evidence extension guide

Create:

- `docs/extending/report-evidence.md`

Required content:

- What belongs in `eggsec-output`.
- How to use the report envelope.
- How to define evidence items, evidence source, redaction state, and finding records.
- How command/tool/domain outputs should map into normalized evidence.
- How to avoid domain-specific report schemas that bypass the envelope.
- Required tests.

Acceptance criteria:

- New output-producing features have a clear path into the normalized report/evidence model.
- Guide references `docs/REPORT_EVIDENCE_MODEL.md` and `architecture/report_envelope.md` if present.

## Work item 8: Add feature gate extension guide

Create:

- `docs/extending/features.md`

Required content:

- How to add a Cargo feature in `crates/eggsec/Cargo.toml`.
- How to classify it in `tests/feature_matrix.rs`.
- How to add dependency edges.
- How to decide whether it is protocol, domain, storage, backend, aggregate, security-risk, or platform-sensitive.
- How to decide whether it belongs in required PR feature-profile checks or deep checks.
- How to document platform-sensitive dependencies.
- Required tests.

Required warnings:

- Adding a feature without updating the snapshot should fail CI.
- `full` is an aggregate/deep profile, not a conservative/default profile.
- Feature-gated metadata still exists; availability and execution are separate concerns.

Acceptance criteria:

- Contributors know how to satisfy `feature_matrix` and CI feature-profile guards.

## Work item 9: Add test matrix and pre-handoff checklist

Create:

- `docs/extending/testing.md`

Required content:

- Which tests correspond to which extension type.
- Required local commands.
- `make check-architecture-ci` as the final pre-handoff target.
- Feature-profile checks.
- Platform-sensitive/deep checks.

Suggested table:

| Extension type | Required tests |
|----------------|----------------|
| Operation metadata | `metadata_consistency`, `feature_matrix` |
| Domain descriptor | `metadata_consistency`, `tool_registration`, `feature_matrix` |
| Command | `command_registry`, `enforcement_matrix` when side-effecting |
| Tool exposure | `tool_registration`, `enforced_dispatch_regression` |
| TUI action | `eggsec-tui --lib`, TUI action spec tests |
| Report output | `eggsec-output --test report_envelope` |
| Feature | `feature_matrix`, representative `cargo check --features ...` |

Acceptance criteria:

- There is one checklist contributors can follow before handoff.
- It matches Phase 11 CI docs and Makefile targets.

## Work item 10: Add lightweight templates/checklists

Create:

- `docs/extending/templates.md`

Include short templates for:

1. New operation metadata.
2. New domain descriptor.
3. New command registration.
4. New tool exposure decision.
5. New TUI action.
6. New report/evidence output.
7. New Cargo feature.
8. PR checklist.

Keep templates compact and explicit. Avoid large boilerplate that will go stale.

Acceptance criteria:

- Contributors can copy checklist structure but still must fill in real risk/capability/scope reasoning.

## Work item 11: Update top-level docs to point at extensibility guides

Update:

- `README.md`
- `docs/ARCHITECTURE.md`
- `docs/CI_ARCHITECTURE_GUARDS.md`
- `AGENTS.md`
- `CONTRIBUTING.md`

Required additions:

- Link to `docs/EXTENSIBILITY.md`.
- State that new extension work should start with the extensibility guide.
- State that `make check-architecture-ci` should pass before handoff.

Acceptance criteria:

- Extensibility docs are discoverable from top-level docs.
- Agent/contributor instructions are consistent with Phase 11 checks.

## Work item 12: Add documentation existence guard for extensibility docs

Extend `scripts/check-architecture-guards.sh` to require the new docs:

- `docs/EXTENSIBILITY.md`
- `docs/extending/operations.md`
- `docs/extending/domains.md`
- `docs/extending/commands.md`
- `docs/extending/tool-exposure.md`
- `docs/extending/tui-actions.md`
- `docs/extending/report-evidence.md`
- `docs/extending/features.md`
- `docs/extending/testing.md`
- `docs/extending/templates.md`

Keep this as existence-only unless richer semantic checks are clearly useful.

Acceptance criteria:

- CI fails if a core extensibility handoff guide is removed.
- The guard does not parse historical `plans/` content.

## Work item 13: Validation commands

Run:

```bash
cargo fmt --all --check
cargo check --workspace --no-default-features
cargo test -p eggsec --lib
cargo test -p eggsec --test metadata_consistency
cargo test -p eggsec --test command_registry
cargo test -p eggsec --test tool_registration --features rest-api
cargo test -p eggsec --test feature_matrix
cargo test -p eggsec --test enforcement_matrix
cargo test -p eggsec --test enforced_dispatch_regression
cargo test -p eggsec-output --test report_envelope
bash scripts/check-architecture-guards.sh
make check-feature-profiles
```

If TUI docs or action spec references are changed in code, also run:

```bash
cargo test -p eggsec-tui --lib
```

Final local reproduction:

```bash
make check-architecture-ci
```

## Files likely to change

New docs:

- `docs/EXTENSIBILITY.md`
- `docs/extending/operations.md`
- `docs/extending/domains.md`
- `docs/extending/commands.md`
- `docs/extending/tool-exposure.md`
- `docs/extending/tui-actions.md`
- `docs/extending/report-evidence.md`
- `docs/extending/features.md`
- `docs/extending/testing.md`
- `docs/extending/templates.md`

Updated docs/scripts:

- `README.md`
- `docs/ARCHITECTURE.md`
- `docs/CI_ARCHITECTURE_GUARDS.md`
- `AGENTS.md`
- `CONTRIBUTING.md`
- `scripts/check-architecture-guards.sh`

Code changes should be minimal or nonexistent. If code changes become necessary, keep them limited to doc guard lists or small doc-link constants.

## Completion criteria

Phase 12 is complete when:

- A top-level extensibility guide exists and is discoverable.
- Detailed guides exist for operations, domains, commands, tool exposure, TUI actions, report/evidence, features, testing, and templates.
- Each guide states the relevant architectural invariants and required tests.
- Top-level docs and contributor docs link to the extensibility guide.
- Phase 11 static guard script requires the extensibility docs to exist.
- Required validation commands pass.
- Contributors can add a new extension by following docs without reading the historical roadmap/plans.

## Handoff note

This is the final phase of the architecture/extensibility roadmap. After it lands, future work should shift from roadmap implementation to normal maintenance: using the extensibility guides, keeping CI guards green, and adding targeted plans only for new domains or major architectural changes.
