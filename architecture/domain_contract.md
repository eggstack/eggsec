# Domain Module Contract

## Role & Responsibilities

The domain module defines a **static metadata interface** (`DomainDescriptor`) that declares what a capability domain can do, how it integrates with CLI/TUI/MCP/tool/report surfaces, and what feature gates control its availability. It is the single registration point for adding new domains to the system.

**Core design principle** ([overview.md](overview.md) Enforcement Model):

> A domain may declare what it can do. A domain may execute already-approved work. A domain must not decide whether work is authorized.

**Non-responsibilities:**
- **Authorization** — Policy evaluation stays in `config/policy_decision.rs` ([config.md](config.md)). Descriptors declare capability without authorizing it.
- **Execution** — Domain logic stays in domain crates (`eggsec-db-lab`, `eggsec-mobile-lab`, `eggsec-web-proxy`) or feature-gated modules ([database_pentest.md](database_pentest.md), [mobile.md](mobile.md)).
- **CLI parsing** — Clap definitions stay in `cli/`.
- **TUI rendering** — Tab definitions stay in `eggsec-tui`.
- **Tool registration** — MCP/REST/gRPC registration stays in `tool/protocol/`, though `tool/registration.rs` reads domain descriptors to derive MCP exposure settings.
- **Report generation** — Output formatting stays in `eggsec-output` or `output/`.
- **Runtime enforcement** — `EnforcementContext::evaluate()` is the mandatory pre-dispatch gate; descriptors do not participate in enforcement.

## Location & Feature Gating

| Item | Path | Feature Gate |
|------|------|:------------:|
| Domain module | `crates/eggsec/src/domain/mod.rs` | None (always compiled) |
| Feature registry | `crates/eggsec/src/config/feature_registry.rs` | None |
| Tool registration (consumer) | `crates/eggsec/src/tool/registration.rs` | None |
| Preflight handler (consumer) | `crates/eggsec/src/commands/handlers/preflight.rs` | None |
| Metadata consistency tests | `crates/eggsec/tests/metadata_consistency.rs` | Various |
| Capability matrix output | `docs/CAPABILITY_MATRIX.md` | N/A |

## Architecture

### Core Types (file:line)

| Type | File:Line | Fields | Purpose |
|------|-----------|--------|---------|
| `DomainDescriptor` | `domain/mod.rs:233` | 15 | Central static metadata struct for a capability domain |
| `DomainCategory` | `domain/mod.rs:27` | 5 variants | Classifies domains by risk/operating mode |
| `OperationIntegration` | `domain/mod.rs:67` | 9 | Maps a domain operation to `OperationMetadata` |
| `CliIntegration` | `domain/mod.rs:90` | 3 | Maps an operation to a CLI command |
| `TuiIntegration` | `domain/mod.rs:101` | 3 | Maps an operation to a TUI tab |
| `ToolIntegration` | `domain/mod.rs:112` | 4 | Maps an operation to MCP/REST/gRPC tool exposure |
| `ReportIntegration` | `domain/mod.rs:125` | 4 | Maps an operation to report output |
| `DryRunSupport` | `domain/mod.rs:138` | 3 variants | Dry-run availability |
| `EvidenceSupport` | `domain/mod.rs:149` | 3 variants | Evidence bundle availability |
| `BaselineSupport` | `domain/mod.rs:160` | 3 variants | Baseline/regression availability |
| `CapabilityMatrixRow` | `domain/mod.rs:171` | 25 | Generated row for capability matrix |

### DomainCategory (5 variants, `domain/mod.rs:27`)

| Variant | Label (`:42`) | Display (`:53`) | Description |
|---------|---------------|-----------------|-------------|
| `StandardAssessment` | "standard assessment" | "standard-assessment" | Scoped recon, scanning, fuzzing, API testing |
| `DefenseLab` | "defense lab" | "defense-lab" | Local/private defense validation and regression |
| `HazardousLab` | "hazardous lab" | "hazardous-lab" | High-risk operations requiring explicit auth |
| `FrontendAdapter` | "frontend adapter" | "frontend-adapter" | Protocol bridges (REST, MCP, gRPC) |
| `OutputAdapter` | "output adapter" | "output-adapter" | Report format adapters |

### Support Enums (3 variants each)

Each of `DryRunSupport`, `EvidenceSupport`, `BaselineSupport` has the same shape:
- `AlwaysAvailable` — unconditional support
- `FeatureGated(&'static str)` — support gated behind a named Cargo feature
- `NotSupported` — capability not implemented

### Registry Functions (`domain/mod.rs`)

| Function | Line | Signature | Purpose |
|----------|------|-----------|---------|
| `all_domain_descriptors()` | `:276` | `-> &'static [DomainDescriptor]` | All known domains regardless of feature state |
| `domain_descriptor_by_id()` | `:318` | `(id: &str) -> Option<&'static DomainDescriptor>` | Lookup by ID |
| `available_domain_descriptors()` | `:325` | `-> Vec<&'static DomainDescriptor>` | Only domains with compiled features |
| `feature_missing_hint()` | `:349` | `(feature: &str) -> Option<&'static str>` | Diagnostic hint for missing features |
| `generate_capability_matrix()` | `:358` | `-> Vec<CapabilityMatrixRow>` | One row per operation across all domains |

The internal `feature_enabled()` helper (`:338`) delegates to `crate::config::is_feature_enabled_registry()`, which calls into `config::feature_registry::is_feature_enabled()` — the authoritative compile-time feature check ([config.md](config.md)).

### Current Domains (3 total)

All three descriptors are `DefenseLab` category. The registry is ordered by category (comments at `:278` show StandardAssessment first, then DefenseLab, then HazardousLab, then adapters).

| # | ID | Display Name | Category | Required Feature | Operations | Risk | Capabilities | Dry Run | Evidence | Baseline | Strict Surface | MCP Exposed | MCP Feature |
|---|-----|-------------|----------|-----------------|------------|------|--------------|---------|----------|----------|---------------|:-----------:|-------------|
| 1 | `db-pentest` | Database Pentesting | DefenseLab | `db-pentest` | 1 | `DbPentest` | `DatabaseAssessment` | Always | Always | Always | Yes | No | `db-pentest-mcp` |
| 2 | `mobile-static` | Mobile Static Analysis | DefenseLab | `mobile` | 1 | `SafeActive` | *(empty)* | Always | NotSupported | NotSupported | Yes | No | None |
| 3 | `mobile-dynamic` | Mobile Dynamic Analysis | DefenseLab | `mobile-dynamic` | 1 | `Intrusive` | `MobileDynamicAnalysis` | Always | Always | Always | No | No | None |

**db-pentest** (`domain/mod.rs:505`): operation `db-pentest` (`:457`), requires explicit scope, CLI/TUI/tool/report each 1 entry.

**mobile-static** (`domain/mod.rs:568`): operation `mobile-static` (`:525`), `OperationMode::StandardAssessment` (not DefenseLab despite domain category), no explicit scope, CLI command ID `"mobile"`, no MCP feature gate.

**mobile-dynamic** (`domain/mod.rs:631`): operation `mobile-dynamic` (`:588`), `OperationMode::DefenseLab`, `OperationRisk::Intrusive`, `strict_surface_support: false`, normalized report not supported.

## Behavior / Flow

### Feature Availability Check

```
DomainDescriptor::is_available()         domain/mod.rs:297
  → self.required_feature
  → feature_enabled(f)                   domain/mod.rs:338
  → crate::config::is_feature_enabled_registry(f)  config/mod.rs:50
  → config::feature_registry::is_feature_enabled(f)  feature_registry.rs:141
  → cfg!(feature = ...) match
```

If `required_feature` is `None`, the domain is always available. Otherwise availability depends on compile-time `cfg!()` state.

### Capability Matrix Generation

`generate_capability_matrix()` (`:358`) iterates all domain descriptors and their operations, producing one `CapabilityMatrixRow` per operation. For each row it:
1. Derives `dry_run`/`evidence_report`/`baseline` as string descriptions from the support enums.
2. Computes `scope_requirement` from `requires_explicit_scope` / `requires_private_or_local_target`.
3. Cross-references CLI/TUI/Tool integrations by matching `operation_id`.
4. Looks up REST/agent exposure from `OperationMetadata` via `metadata_for_tool_id()`.
5. Derives `target_policy` from scope requirements.

### Tool Registration Integration

`tool/registration.rs:68` — `all_tool_registrations()` reads `all_domain_descriptors()` and for each `OperationMetadata` entry, checks if any domain has a matching `ToolIntegration`. If so, the domain's `tool_id`, `mcp_exposed_by_default`, and `required_mcp_feature` override the defaults. Domain tools get `ToolRegistrationSource::Domain(domain_id)`.

### Preflight Integration

`commands/handlers/preflight.rs:53,70` — The preflight CLI handler calls `domain_descriptor_by_id()` to enrich both JSON and human-readable output with domain metadata (ID, display name, description, category).

## Integration Points

| Consumer | File:Line | How It Uses Domains |
|----------|-----------|---------------------|
| Tool registration | `tool/registration.rs:69` | Reads `all_domain_descriptors()` for MCP exposure settings |
| Preflight handler | `commands/handlers/preflight.rs:53,70` | Enriches output via `domain_descriptor_by_id()` |
| Capability matrix | `docs/CAPABILITY_MATRIX.md` | Generated from `generate_capability_matrix()` |
| Metadata consistency tests | `crates/eggsec/tests/metadata_consistency.rs` | Validates domain ↔ `OperationMetadata` alignment |
| (Future) CLI/TUI | CLI/TUI code | Domain-integrated commands and tabs |

## Testing

All tests are in `domain/mod.rs:651-1056`. Test count: 19 tests total.

| Test | Line | What It Verifies |
|------|------|------------------|
| `domain_category_label_is_stable` | `:655` | All 5 variant labels match expected strings |
| `domain_category_display_is_kebab_case` | `:667` | All 5 Display impls produce kebab-case |
| `dry_run_support_equality` | `:682` | PartialEq/Eq for DryRunSupport |
| `evidence_support_equality` | `:694` | PartialEq/Eq for EvidenceSupport |
| `db_pentest_descriptor_exists` | `:715` | (cfg db-pentest) ID and display name |
| `db_pentest_category_is_defense_lab` | `:722` | Category is DefenseLab |
| `db_pentest_requires_db_pentest_feature` | `:727` | Required feature matches |
| `db_pentest_has_one_operation` | `:732` | Exactly 1 operation |
| `db_pentest_operation_risk_is_db_pentest` | `:741` | Risk tier |
| `db_pentest_operation_mode_is_defense_lab` | `:748` | Operation mode |
| `db_pentest_requires_database_assessment_capability` | `:757` | Capability check |
| `db_pentest_requires_explicit_scope` | `:764` | Scope requirement |
| `db_pentest_mcp_not_exposed_by_default` | `:769` | MCP exposure flag |
| `db_pentest_mcp_requires_feature` | `:774` | MCP feature gate |
| `db_pentest_dry_run_always_available` | `:782` | Dry-run support |
| `db_pentest_evidence_always_available` | `:790` | Evidence support |
| `db_pentest_has_cli_integration` | `:798` | CLI entry |
| `db_pentest_has_tui_integration` | `:804` | TUI entry |
| `db_pentest_has_report_integration` | `:810` | Report entry |
| `registry_includes_db_pentest` | `:816` | Registry presence |
| `lookup_by_id_works` | `:822` | `domain_descriptor_by_id` |
| `lookup_missing_id_returns_none` | `:829` | Negative lookup |
| `descriptor_is_const_constructible` | `:834` | Compile-time construction |
| `descriptor_does_not_authorize` | `:840` | Safety invariant |
| `all_domain_operation_ids_have_metadata` | `:853` | Every op has `OperationMetadata` |
| `domain_operation_matches_metadata` | `:871` | Mode/risk/caps/features match metadata |
| `domain_ids_are_unique` | `:911` | No duplicate domain IDs |
| `domain_operation_ids_within_domain_are_unique` | `:919` | No duplicate op IDs per domain |
| `feature_missing_hint_returns_something_for_known_features` | `:934` | Hints exist for 8 known features |
| `feature_missing_hint_returns_none_for_unknown` | `:963` | Unknown → None |
| `domain_is_available_matches_feature_state` | `:968` | is_available consistency |
| `domain_availability_hint_consistent_with_is_available` | `:984` | Hint ↔ availability consistency |
| `capability_matrix_generation_works` | `:1013` | Matrix rows have non-empty fields |
| `capability_matrix_pilot_domain_row_present` | `:1032` | (cfg db-pentest) db-pentest row present |
| `mobile_dynamic_not_baseline_safe` | `:1051` | (cfg mobile-dynamic) Intrusive risk, no strict surface |

## Invariants & Gotchas

### Safety Invariants

1. **No authorization in descriptors** — Descriptors are metadata only. They contain no policy evaluation or scope checking logic. (`:228-230`)
2. **No network I/O** — Descriptor construction is purely compile-time/const.
3. **No approval tokens** — Descriptors are not `ApprovedOperation` tokens.
4. **Hazardous domains hidden from MCP** — Hazardous domains must not be exposed via MCP by default.
5. **Feature + policy gating** — Descriptor presence does not imply feature availability. Both compile-time `cfg` and runtime policy must be checked.
6. **No dynamic plugins** — Phase 3 is static-only. Dynamic plugin loading is a future phase.

### Gotchas

- **Domain vs. Operation mode mismatch**: `mobile-static` has `category: DefenseLab` but `OperationMode::StandardAssessment`. The domain category is a classification label; the operation mode is the enforcement-relevant field.
- **`strict_surface_support: false` for mobile-dynamic**: This domain is excluded from strict surfaces (MCP/Agent/REST/gRPC). The field is purely declarative — enforcement is in `EnforcementContext::evaluate()`.
- **`available_domain_descriptors()` returns `Vec`**: This allocates on every call (not `&'static`). Consumers in hot paths should cache or use `all_domain_descriptors()` with manual filtering.
- **Feature hints are static strings**: `feature_missing_hint()` returns `&'static str` from the feature registry, not dynamically generated.
- **`CapabilityMatrixRow` target_policy derivation**: Target policy is derived from scope requirements, not from `OperationMetadata` directly. This is a heuristic for documentation purposes.

## Phase Handoff

This contract was defined in Phase 3. Phase 4 completed metadata unification:
- **Phase 4 (complete)**: Added `description` field, `CapabilityMatrixRow` type, `generate_capability_matrix()`, `docs/CAPABILITY_MATRIX.md`, metadata consistency tests, preflight domain metadata integration.
- **Phase 5**: Migrate additional domains to the contract.
- **Future**: Consider `eggsec-domain-core` crate extraction.

---

*Last verified against source: 2026-08-25*
