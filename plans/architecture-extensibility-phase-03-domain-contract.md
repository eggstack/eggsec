# Phase 3 Handoff Plan: Domain Module Contract Design

## Objective

Define and pilot a static domain integration contract for Eggsec. The contract should make new capability domains easier to add without scattered edits across policy metadata, CLI, TUI, MCP/tool registration, report conversion, docs, and feature gates.

This phase should not attempt dynamic plugin loading. The target is a static Rust-side contract used by in-tree workspace crates and feature-gated modules.

## Context

Eggsec already has several architectural ingredients for extensibility:

- centralized policy/enforcement;
- operation descriptors and operation metadata;
- tool-core DTO extraction;
- output crate separation;
- TUI crate separation;
- domain extraction precedent in `eggsec-db-lab`;
- partial domain extraction in `eggsec-web-proxy`.

The remaining problem is integration sprawl. Adding a new domain can require hand edits to many places:

- Cargo features;
- CLI command enum and args;
- command dispatcher;
- policy metadata;
- operation descriptor builder;
- TUI tab enum and tab store;
- TUI preflight and task launch;
- MCP/tool registration;
- report bridge;
- docs/README capability table;
- tests and examples.

The domain contract should not solve every integration in one pass. It should create the stable shape that later phases can consume.

## Design principle

A domain may declare what it can do. A domain may execute already-approved work. A domain must not decide whether work is authorized.

Central enforcement remains in the main `eggsec` crate or a future dedicated policy crate.

## Deliverables

1. Add a `DomainDescriptor` or `EggsecDomain` abstraction in an appropriate crate/module.

2. Define supporting metadata structs for operations, feature gates, CLI exposure, TUI exposure, MCP/tool exposure, report adapters, dry-run support, and evidence support.

3. Implement the contract for one pilot domain. Prefer `db-pentest` or `web-proxy` because both already have extracted-domain characteristics and high architectural value.

4. Add a registry function that returns the static set of known domain descriptors for the current feature set.

5. Add tests proving the pilot descriptor exposes expected metadata and does not perform authorization.

6. Add documentation explaining how the contract should be used and what it intentionally does not own.

## Placement options

Evaluate these placement options before implementing:

### Option A: `crates/eggsec/src/domain/`

Pros: quickest, can depend on existing config/policy types. Good for first pass.

Cons: keeps contract in the main crate and may make extraction harder later.

### Option B: new `eggsec-domain-core` crate

Pros: cleaner long-term seam. Domain crates could depend on the contract without depending on the full main engine.

Cons: requires workspace changes and careful dependency design. The contract must avoid depending on heavy main-crate types unless those are moved or re-exported from a policy/core crate.

### Recommended phase choice

Use Option A unless the implementation is already straightforward with a new lightweight crate. The priority is to stabilize the shape, not perfect final placement. If placed inside the main crate, document that it may later move to `eggsec-domain-core` or `eggsec-policy-core`.

## Proposed contract shape

The exact names can change, but the model should cover the following concepts.

```rust
pub struct DomainDescriptor {
    pub id: &'static str,
    pub display_name: &'static str,
    pub category: DomainCategory,
    pub required_feature: Option<&'static str>,
    pub operations: &'static [OperationIntegration],
    pub cli: &'static [CliIntegration],
    pub tui: &'static [TuiIntegration],
    pub tools: &'static [ToolIntegration],
    pub reports: &'static [ReportIntegration],
    pub dry_run: DryRunSupport,
    pub evidence: EvidenceSupport,
}
```

Suggested supporting enums/structs:

```rust
pub enum DomainCategory {
    StandardAssessment,
    DefenseLab,
    HazardousLab,
    FrontendAdapter,
    OutputAdapter,
}

pub struct OperationIntegration {
    pub operation_id: &'static str,
    pub display_name: &'static str,
    pub mode: OperationMode,
    pub risk: OperationRisk,
    pub capabilities: &'static [Capability],
    pub intended_uses: &'static [IntendedUse],
    pub required_features: &'static [&'static str],
    pub requires_explicit_scope: bool,
    pub requires_private_or_local_target: bool,
}

pub struct CliIntegration {
    pub command_id: &'static str,
    pub operation_id: &'static str,
    pub feature: Option<&'static str>,
}

pub struct TuiIntegration {
    pub tab_id: &'static str,
    pub operation_id: &'static str,
    pub feature: Option<&'static str>,
}

pub struct ToolIntegration {
    pub tool_id: &'static str,
    pub operation_id: &'static str,
    pub mcp_exposed_by_default: bool,
    pub required_mcp_feature: Option<&'static str>,
}

pub struct ReportIntegration {
    pub report_kind: &'static str,
    pub operation_id: &'static str,
    pub evidence_bundle_supported: bool,
}
```

Keep the first implementation simple. Avoid overfitting. The minimum useful contract is domain ID, operations, feature gates, tool exposure, and report support.

## Pilot domain selection

### Preferred pilot: `db-pentest`

Reasons:

- Already extracted into `eggsec-db-lab`.
- Has rich domain semantics: dry-run, real mode, baselines, compliance, correlation, evidence bundles, optional MCP exposure.
- Clearly defense-lab and high-risk enough to test safety metadata.

Expected metadata:

- domain ID: `db-pentest` or `database-lab`;
- category: `DefenseLab`;
- feature: `db-pentest`;
- optional MCP feature: `db-pentest-mcp`;
- operation risk: `DbPentest`;
- capability: `DatabaseAssessment`;
- explicit scope/manifest expectations;
- dry-run supported;
- evidence bundle supported.

### Alternate pilot: `web-proxy`

Reasons:

- Already extracted into `eggsec-web-proxy`.
- Traffic interception has a clear capability and risk class.
- Optional MCP exposure and transparent/dynamic plugin feature markers are useful metadata stressors.

Use this if db-pentest proves too broad for the first pass.

## Implementation steps

1. Review Phase 1 and Phase 2 outputs.

2. Inspect existing operation metadata facilities: `OperationMetadata`, `ALL_OPERATION_METADATA`, `metadata_for_tool_id`, `operation_matches_tool_id`, and related helpers.

3. Decide placement for the domain contract.

4. Add domain metadata types with serde support only if needed. Avoid unnecessary dependency growth.

5. Add a static domain registry function, for example `all_domain_descriptors()` or `registered_domains()`.

6. Implement the pilot domain descriptor.

7. Connect the pilot descriptor to one low-risk consumer. Suggested consumers in order of preference:

   - a new `eggsec domain-list` internal/debug command if easy;
   - tests only;
   - policy explain display;
   - generated docs in Phase 4, not this phase.

8. Add tests:

   - descriptor exists when feature is available;
   - operation IDs are stable;
   - required capabilities match expected risk;
   - MCP exposure remains false unless the explicit MCP feature is enabled;
   - descriptor construction performs no network or domain execution;
   - descriptor does not authorize anything.

9. Document the contract in `docs/DOMAIN_CONTRACT.md` or a section of `docs/ARCHITECTURE.md`.

10. Run validation.

## Safety requirements

- Do not move authorization into domain crates.
- Do not let descriptors become approval tokens.
- Do not expose hazardous domains to MCP by default.
- Do not infer feature availability from descriptor presence alone; runtime policy and compile-time cfg must both be respected.
- Do not perform network I/O while building descriptors.
- Do not add dynamic plugin loading in this phase.

## Validation commands

Run at minimum:

```bash
cargo fmt --all --check
cargo check --workspace --no-default-features
cargo test -p eggsec --lib
```

Run pilot-domain feature checks:

```bash
cargo check -p eggsec --features db-pentest
cargo test -p eggsec --features db-pentest --lib
```

If using web-proxy as pilot:

```bash
cargo check -p eggsec --features web-proxy
cargo test -p eggsec --features web-proxy --lib
```

If MCP exposure metadata is touched:

```bash
cargo check -p eggsec --features rest-api,tool-api
cargo check -p eggsec --features db-pentest-mcp,rest-api,tool-api
```

## Non-goals

Do not refactor the full command dispatcher.

Do not generate docs yet unless it is trivial.

Do not extract more domains yet.

Do not change user-facing CLI behavior unless needed for a small debug/introspection command.

Do not convert the TUI tab registry yet.

Do not introduce runtime dynamic plugins.

## Acceptance criteria

- A domain descriptor contract exists.
- One real domain implements the contract.
- The descriptor includes operation/risk/capability/feature/exposure metadata.
- Tests prove descriptor metadata is stable and authorization-neutral.
- Documentation explains how domain descriptors should be used.
- Existing behavior remains compatible.

## Handoff notes for Phase 4

Phase 4 should consume the domain descriptor and operation metadata to generate or validate docs, policy explain output, tool metadata, and capability matrices. Keep any TODOs explicit where the Phase 3 pilot exposes metadata that is not yet consumed.
