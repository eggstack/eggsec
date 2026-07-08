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
| Capability matrix | `docs/CAPABILITY_MATRIX.md` | Humans (manually maintained from OperationMetadata + DomainDescriptor; metadata consistency tests validate underlying structures) |
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

## Programmatic Exposure Semantics

The `mcp_metadata_exposable`, `rest_exposable`, `agent_exposable`, and `grpc_exposable` flags on `OperationMetadata` indicate whether an operation **may** be registered on the relevant programmatic surface when the required feature is compiled and the runtime policy approves.

These flags do **not** mean:
- The operation executes safely by default
- The operation is baseline-agent-safe
- No scope manifest is required
- `EnforcementContext::evaluate()` will permit dispatch

Strict programmatic surfaces (MCP, REST, agent, gRPC) always require:
1. Explicit scope manifest (`LoadedScope`)
2. `EnforcementContext::evaluate()` approval
3. For strict profiles: `ApprovedOperation` token before dispatch

High-risk operations (risk > `SafeActive`) with programmatic exposure flags still require non-baseline capabilities and strict policy gates. The `docs/CAPABILITY_MATRIX.md` Standalone Operations table documents these flags as metadata-level exposure permissions, not default runtime behavior.

## MCP Profile Visibility Model A

The MCP surface distinguishes between two layers of visibility:

- **`mcp_metadata_exposable`** — broad metadata-level permission gate set from
  `OperationMetadata.mcp_exposable`. OpsAgent uses this for its expanded
  profile listing.
- **`mcp_default_visible`** — conservative subset visible to
  `mcp_tool_registrations_default_visible()`. Restricts to passive/safe-active
  operations with no feature gate and `mcp_metadata_exposable = true`.

OpsAgent is **profile-expanded**, not conservative default. It lists every
`mcp_metadata_exposable` tool, including high-risk operations. Strict runtime
policy (`EnforcementContext::evaluate()` + `ApprovedOperation`) is still
required before any listed tool executes. See `docs/TOOL_REGISTRATION.md` for
the full protocol-by-protocol listing behavior.

## Tool Registration (Phase 7)

`ToolRegistration` (`tool::registration`) is a **derived metadata source** that bridges `OperationMetadata` and `DomainDescriptor` `ToolIntegration` to produce per-protocol tool listings. It is not a third static registry — it is computed from the two existing ones.

Each `ToolRegistration` carries:
- Tool ID, display name, operation ID
- Protocol exposure flags (`rest_exposable`, `mcp_metadata_exposable`, `mcp_default_visible`, `grpc_exposable`, `agent_exposable`)
- Source: `Base`, `FeatureGated(&str)`, or `Domain(&str)`
- Required MCP feature (if any)

Builder functions filter registrations by protocol:
- `all_tool_registrations()` — full inventory
- `mcp_tool_registrations("ops-agent")` — Model A profile-expanded listing: every `mcp_metadata_exposable` tool
- `mcp_tool_registrations("coding-agent")` — hardcoded narrow allowlist
- `mcp_tool_registrations_default_visible()` — conservative default subset (passive/safe-active, no feature gate)
- `rest_tool_registrations()` — tools with `rest_exposable = true`
- `grpc_tool_registrations()` — tools with `grpc_exposable = true`
- `agent_tool_registrations()` — tools with `agent_exposable = true`

These functions derive from `OperationMetadata` (base + feature-gated tools) and `DomainDescriptor::ToolIntegration` (domain tools). Protocol listing functions (MCP, REST, gRPC, Agent) now filter through registration metadata, replacing the previous direct `registry.list()` approach.

Registration does **not** grant authorization. The `EnforcementContext::evaluate()` gate remains the sole authorization path.

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
| `all_capability_variants_appear_in_metadata` | All 19 `Capability` variants are used |
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
| `mobile_dynamic_strict_surface_support_is_false` | Safety: mobile-dynamic strict surface support is false |
| `mobile_dynamic_requires_explicit_scope_is_false` | Safety: mobile-dynamic does not require explicit scope |
| `mobile_dynamic_declares_dynamic_analysis_capability` | mobile-dynamic declares MobileDynamicAnalysis capability |
| `mobile_static_requires_explicit_scope_is_false` | Safety: mobile-static does not require explicit scope |
| `high_risk_agent_exposable_ops_are_not_baseline_safe` | Safety: high-risk agent ops are not baseline-safe |
| `domain_docs_urls_reference_existing_files` | Local docs URLs reference existing files |
| `domains_with_normalized_report_declare_report_integration` | Every domain with `normalized_report_supported: true` has report integration |

## Safety Invariants

1. **Metadata must not grant authorization.** Descriptors are data, not policy.
2. **Feature presence must not imply runtime authorization.** Both compile-time `cfg` and runtime policy must be checked.
3. **Hazardous domains must not be MCP-exposed by default.** MCP exposure requires opt-in.
4. **Generated docs must not overstate capabilities.** Feature-gated capabilities must be clearly marked.
5. **Manual CLI/TUI semantics must remain distinct from automated strict semantics.** Metadata surfaces are separate.

## Phase History

- **Phase 3**: Introduced `DomainDescriptor` and `OperationIntegration` for the `db-pentest` pilot domain.
- **Phase 4**: Added `CapabilityMatrixRow`, `generate_capability_matrix()`, `BaselineSupport`, `docs_url`, `strict_surface_support`, metadata consistency tests, and `docs/CAPABILITY_MATRIX.md`.
- **Phase 7**: Added `ToolRegistration` as derived metadata source bridging `OperationMetadata` and `DomainDescriptor::ToolIntegration`.
- **Phase 9**: Added `normalized_report_supported` to `ReportIntegration` in `DomainDescriptor`. Introduced normalized report/evidence envelope model (`ReportEnvelope`, `FindingRecord`, `EvidenceItem`, `EvidenceManifest`, `BaselineSummary`) in `eggsec-output::envelope`. Added `RedactionPolicy` to `EvidenceManifest` for manifest-level redaction semantics. Pilot domain bridges (db-pentest, mobile-static) emit normalized envelopes. `docs/CAPABILITY_MATRIX.md` tracks normalized report support per domain.

## Exposure Model Decision

The project uses **Model A** (broad programmatic exposure flags with explicit semantics):
- High-risk operations may have `mcp_metadata_exposable`/`rest_exposable`/`agent_exposable` set to `true`
- This means metadata permits registration when compiled, registered, scoped, and policy-authorized
- It does **not** mean default safe execution or baseline agent safety
- Strict surfaces require `EnforcementContext::evaluate()` and `ApprovedOperation` tokens
- OpsAgent is a **profile-expanded** listing under Model A; the conservative default is `mcp_tool_registrations_default_visible()`
