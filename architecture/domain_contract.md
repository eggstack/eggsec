# Domain Module Contract

## Purpose

The domain module contract defines a static metadata interface for capability domains in Eggsec. It makes new domains easier to add by centralizing the integration points (operations, CLI, TUI, MCP/tool, reports, feature gates) into a single descriptor.

## Design Principle

> A domain may declare what it can do. A domain may execute already-approved work. A domain must not decide whether work is authorized.

Central enforcement remains in the main `eggsec` crate or a future dedicated policy crate.

## Core Types

### `DomainDescriptor`

The central type. A static, `const`-constructible metadata struct declaring everything a domain needs to integrate with the system.

```rust
pub struct DomainDescriptor {
    pub id: &'static str,
    pub display_name: &'static str,
    pub description: &'static str,
    pub category: DomainCategory,
    pub required_feature: Option<&'static str>,
    pub operations: &'static [OperationIntegration],
    pub cli: &'static [CliIntegration],
    pub tui: &'static [TuiIntegration],
    pub tools: &'static [ToolIntegration],
    pub reports: &'static [ReportIntegration],
    pub dry_run: DryRunSupport,
    pub evidence: EvidenceSupport,
    pub baseline: BaselineSupport,
    pub strict_surface_support: bool,
    pub docs_url: Option<&'static str>,
}
```

### `DomainCategory`

Classifies domains by their risk and operating mode:

| Variant | Description |
|---------|-------------|
| `StandardAssessment` | Scoped recon, scanning, fuzzing, API testing |
| `DefenseLab` | Local/private defense validation and regression |
| `HazardousLab` | High-risk operations requiring explicit auth |
| `FrontendAdapter` | Protocol bridges (REST, MCP, gRPC) |
| `OutputAdapter` | Report format adapters |

### Supporting Types

- **`OperationIntegration`** — Maps a domain operation to `OperationMetadata` (operation ID, mode, risk, capabilities, features, scope requirements).
- **`CliIntegration`** — Maps an operation to a CLI command.
- **`TuiIntegration`** — Maps an operation to a TUI tab.
- **`ToolIntegration`** — Maps an operation to MCP/REST/gRPC tool exposure.
- **`ReportIntegration`** — Maps an operation to report output.
- **`DryRunSupport`** — `AlwaysAvailable`, `FeatureGated(&str)`, or `NotSupported`.
- **`EvidenceSupport`** — `AlwaysAvailable`, `FeatureGated(&str)`, or `NotSupported`.
- **`BaselineSupport`** — `AlwaysAvailable`, `FeatureGated(&str)`, or `NotSupported` (whether baseline capture and regression comparison is supported).

## Registry

```rust
pub fn all_domain_descriptors() -> &'static [DomainDescriptor];
pub fn domain_descriptor_by_id(id: &str) -> Option<&'static DomainDescriptor>;
pub fn available_domain_descriptors() -> Vec<&'static DomainDescriptor>;
pub fn feature_missing_hint(feature: &str) -> Option<&'static str>;
pub fn generate_capability_matrix() -> Vec<CapabilityMatrixRow>;
```

The registry returns all known domains. Domains behind disabled features are included (their `required_feature` field indicates gating). Consumers should check feature availability before use.

`available_domain_descriptors()` returns only domains whose required feature is compiled (convenience wrapper over `all_domain_descriptors()`).

`feature_missing_hint()` returns a diagnostic hint string naming the exact Cargo feature flag needed to enable a domain (e.g. `"enable the 'db-pentest' feature in Cargo.toml: cargo build --features db-pentest"`). Returns `None` for unrecognized features.

`generate_capability_matrix()` produces `CapabilityMatrixRow` entries from all registered domain descriptors, suitable for documentation generation and validation. `docs/CAPABILITY_MATRIX.md` is the canonical human-readable output.

## Pilot Domain: db-pentest

The `db-pentest` domain is the first pilot implementation:

| Field | Value |
|-------|-------|
| `id` | `"db-pentest"` |
| `category` | `DefenseLab` |
| `required_feature` | `"db-pentest"` |
| `operation_id` | `"db-pentest"` |
| `risk` | `DbPentest` |
| `capabilities` | `[DatabaseAssessment]` |
| `mcp_exposed_by_default` | `false` |
| `required_mcp_feature` | `"db-pentest-mcp"` |
| `dry_run` | `AlwaysAvailable` |
| `evidence` | `AlwaysAvailable` |
| `baseline` | `AlwaysAvailable` |
| `strict_surface_support` | `true` |
| `docs_url` | `Some("docs/DATABASE_PENTEST.md")` |

## Mobile Domain Descriptors

### mobile-static

| Field | Value |
|-------|-------|
| `id` | `"mobile-static"` |
| `category` | `DefenseLab` |
| `required_feature` | `"mobile"` |
| `operation_id` | `"mobile-static"` |
| `risk` | `SafeActive` |
| `mcp_exposed_by_default` | `false` |
| `dry_run` | `AlwaysAvailable` |
| `evidence` | `NotSupported` |
| `baseline` | `NotSupported` |
| `strict_surface_support` | `true` |
| `docs_url` | `Some("docs/MOBILE.md")` |

### mobile-dynamic

| Field | Value |
|-------|-------|
| `id` | `"mobile-dynamic"` |
| `category` | `DefenseLab` |
| `required_feature` | `"mobile-dynamic"` |
| `operation_id` | `"mobile-dynamic"` |
| `risk` | `Intrusive` |
| `capabilities` | `[MobileDynamicAnalysis]` |
| `mcp_exposed_by_default` | `false` |
| `dry_run` | `AlwaysAvailable` |
| `evidence` | `AlwaysAvailable` |
| `baseline` | `AlwaysAvailable` |
| `strict_surface_support` | `false` |
| `docs_url` | `Some("docs/MOBILE.md")` |

## DomainDescriptor Methods

- `is_available()` - Returns `true` if the domain's required feature (if any) is currently compiled
- `availability_hint()` - Returns a diagnostic hint if the domain is unavailable, or `None` if available

## Safety Invariants

1. **No authorization in descriptors** — Descriptors are metadata only. They contain no policy evaluation or scope checking logic.
2. **No network I/O** — Descriptor construction is purely compile-time/const.
3. **No approval tokens** — Descriptors are not `ApprovedOperation` tokens.
4. **Hazardous domains hidden from MCP** — Hazardous domains must not be exposed via MCP by default.
5. **Feature + policy gating** — Descriptor presence does not imply feature availability. Both compile-time `cfg` and runtime policy must be checked.
6. **No dynamic plugins** — Phase 3 is static-only. Dynamic plugin loading is a future phase.

## How to Add a New Domain

1. Define a `const DomainDescriptor` in `crates/eggsec/src/domain/mod.rs`.
2. Add supporting `const` integration structs (operations, CLI, TUI, tools, reports).
3. Add the descriptor to `all_domain_descriptors()`.
4. Add tests verifying metadata stability and safety invariants.
5. If the domain has a feature gate, use `#[cfg(feature = "...")]` on the descriptor and its entry in the registry.

## What This Contract Does NOT Own

- **Authorization** — Policy evaluation stays in `config/policy_decision.rs`.
- **Execution** — Domain logic stays in domain crates or modules.
- **CLI parsing** — Clap definitions stay in `cli/`.
- **TUI rendering** — Tab definitions stay in `eggsec-tui`.
- **Tool registration** — MCP/REST/gRPC registration stays in `tool/protocol/`.
- **Report generation** — Output formatting stays in `eggsec-output` or `output/`.

## Relationship to `OperationMetadata`

`OperationMetadata` (in `config/policy.rs`) defines the canonical metadata for individual operations. `DomainDescriptor` groups related operations under a domain umbrella and adds domain-level metadata (category, feature gates, integration points). The `OperationIntegration` struct within a domain descriptor references `OperationMetadata` by operation ID.

## Phase Handoff

This contract was defined in Phase 3 of the architecture extensibility plan. Phase 4 completed metadata unification:

- **Phase 4 (complete)**: Added `description` field, `CapabilityMatrixRow` type, `generate_capability_matrix()`, `docs/CAPABILITY_MATRIX.md`, metadata consistency tests (`tests/metadata_consistency.rs`), preflight domain metadata integration, and README simplification.
- **Phase 5**: Migrate additional domains to the contract.
- **Future**: Consider `eggsec-domain-core` crate extraction if the contract outgrows the main crate.

## Location

`crates/eggsec/src/domain/mod.rs`

May later move to `eggsec-domain-core` or `eggsec-policy-core` crate.
