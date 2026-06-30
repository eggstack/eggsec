# Metadata Ownership Model

## Overview

Eggsec uses two complementary static metadata registries that together form the canonical source of truth for all operations, tools, and domains:

1. **`OperationMetadata`** (`config/policy.rs`) — Per-operation metadata: risk, capabilities, feature gates, surface exposure flags.
2. **`DomainDescriptor`** (`domain/mod.rs`) — Per-domain metadata: category, integration points (CLI/TUI/tool/report), dry-run/evidence/baseline support.

These registries are **static**, **const-constructible**, and **authorization-neutral**. They describe what operations exist and how they integrate with the system — they never decide whether an operation is authorized.

## Where Metadata Lives

| Metadata | Location | Consumed By |
|----------|----------|-------------|
| Operation ID, risk, capabilities, feature gates | `ALL_OPERATION_METADATA` in `config/policy.rs` | Policy enforcement, preflight, REST/MCP/gRPC registration, TUI tabs |
| Tool aliases | `ALL_OPERATION_METADATA_ALIASES` in `config/policy.rs` | `metadata_for_tool_id()` resolver |
| Domain category, integrations, dry-run/evidence/baseline | `DomainDescriptor` in `domain/mod.rs` | Capability matrix generation, preflight domain info, documentation |
| Capability matrix | `docs/CAPABILITY_MATRIX.md` | Humans, CI validation |
| Consistency tests | `tests/metadata_consistency.rs` | CI, development validation |

## Ownership Rules

### Adding a New Operation

1. Add an `OperationMetadata` entry to `ALL_OPERATION_METADATA` in `crates/eggsec/src/config/policy.rs`.
2. If the operation is domain-scoped, add an `OperationIntegration` to the relevant `DomainDescriptor`.
3. Add any needed aliases to `ALL_OPERATION_METADATA_ALIASES`.
4. Update `docs/CAPABILITY_MATRIX.md` with the new row.
5. Run `cargo test -p eggsec --test metadata_consistency` to verify consistency.

### Adding a New Domain

1. Define `const` integration structs in `crates/eggsec/src/domain/mod.rs` (operations, CLI, TUI, tools, reports).
2. Define a `const DomainDescriptor` with all required fields.
3. Add the descriptor to `all_domain_descriptors()`.
4. Add unit tests in `domain/mod.rs` and integration tests in `tests/metadata_consistency.rs`.
5. Update `docs/CAPABILITY_MATRIX.md`.
6. Run `cargo test -p eggsec --lib -- domain` and `cargo test -p eggsec --test metadata_consistency`.

### Modifying Existing Metadata

1. Edit the relevant registry (`ALL_OPERATION_METADATA` or `DomainDescriptor`).
2. Update `docs/CAPABILITY_MATRIX.md` to match.
3. Run the full metadata consistency test suite.
4. If removing or renaming an operation, update all aliases and integration points.

## Validation Pipeline

The following tests validate metadata consistency:

| Test | What It Checks |
|------|---------------|
| `all_domain_operations_have_matching_metadata` | Every domain operation ID resolves to an `OperationMetadata` entry |
| `domain_risk_matches_operation_metadata_risk` | Domain and metadata risk tiers agree |
| `domain_capabilities_match_metadata` | Domain and metadata capabilities agree |
| `domain_features_subset_of_metadata` | Domain features are a subset of metadata features |
| `operation_metadata_ids_are_unique` | No duplicate operation IDs |
| `all_aliases_resolve_to_known_metadata` | All aliases point to valid canonical IDs |
| `no_alias_maps_to_self` | No redundant self-mapping aliases |
| `all_capability_variants_appear_in_metadata` | All 18 `Capability` variants are used |
| `all_operation_risk_variants_appear_in_metadata` | All 15 `OperationRisk` variants are used |
| `all_registered_base_tools_have_operation_metadata` | Every default tool has metadata |
| `operation_id_lookup_is_stable` | Self-lookup returns the same entry |
| `alias_risk_matches_canonical` | Alias and canonical entries agree on risk |
| `hazardous_domains_not_mcp_exposed_by_default` | Safety: no hazardous MCP default exposure |
| `high_risk_agent_exposable_ops_declare_capability` | Safety: high-risk agent ops have capabilities |
| `mcp_exposed_ops_have_metadata_flag` | MCP exposure in domain matches metadata flag |
| `domain_docs_urls_are_nonempty_when_present` | Docs URLs are non-empty strings |
| `all_domains_declare_dry_run_support` | Every domain declares dry-run support |
| `all_domains_declare_baseline_support` | Every domain declares baseline support |

## Safety Invariants

1. **Metadata must not grant authorization.** Descriptors are data, not policy.
2. **Feature presence must not imply runtime authorization.** Both compile-time `cfg` and runtime policy must be checked.
3. **Hazardous domains must not be MCP-exposed by default.** MCP exposure requires opt-in.
4. **Generated docs must not overstate capabilities.** Feature-gated capabilities must be clearly marked.
5. **Manual CLI/TUI semantics must remain distinct from automated strict semantics.** Metadata surfaces are separate.

## Phase History

- **Phase 3**: Introduced `DomainDescriptor` and `OperationIntegration` for the `db-pentest` pilot domain.
- **Phase 4**: Added `CapabilityMatrixRow`, `generate_capability_matrix()`, `BaselineSupport`, `docs_url`, `strict_surface_support`, metadata consistency tests, and `docs/CAPABILITY_MATRIX.md`.
- **Phase 5** (future): Extract additional domains to the contract, slim the main crate.
