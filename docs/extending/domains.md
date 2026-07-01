# Adding New Domain Crates and Domain Descriptors

This guide explains how to add a new capability domain to Eggsec. A domain
groups related operations under a shared metadata umbrella, providing a
single place to declare CLI, TUI, MCP, tool, and report integrations for
that capability area.

See also:

- [Adding New OperationMetadata](operations.md) for the per-operation metadata
  registry that domain operations reference.
- [METADATA_OWNERSHIP.md](../METADATA_OWNERSHIP.md) for the ownership model
  and validation pipeline.
- [CAPABILITY_MATRIX.md](../CAPABILITY_MATRIX.md) for the human-readable
  capability matrix derived from domain and operation metadata.

## Core Invariant

**Domains may declare and group operations, but they must not decide
authorization.** Authorization remains in `EnforcementContext` and dispatch
approval tokens. A `DomainDescriptor` is pure metadata: it never evaluates
scope, never checks policy, and never performs network I/O.

## 1. What DomainDescriptor Owns

`DomainDescriptor` is a `const`-constructible struct defined in
`crates/eggsec/src/domain/mod.rs`. Each field is static data:

| Field | Type | Purpose |
|-------|------|---------|
| `id` | `&'static str` | Unique domain identifier (kebab-case, e.g. `"db-pentest"`) |
| `display_name` | `&'static str` | Human-readable label |
| `description` | `&'static str` | Brief purpose statement |
| `category` | `DomainCategory` | Classification (see below) |
| `required_feature` | `Option<&'static str>` | Cargo feature flag to compile this domain |
| `operations` | `&'static [OperationIntegration]` | Operations this domain groups |
| `cli` | `&'static [CliIntegration]` | CLI command integrations |
| `tui` | `&'static [TuiIntegration]` | TUI tab integrations |
| `tools` | `&'static [ToolIntegration]` | MCP/REST/gRPC tool integrations |
| `reports` | `&'static [ReportIntegration]` | Report output integrations |
| `dry_run` | `DryRunSupport` | Dry-run support level |
| `evidence` | `EvidenceSupport` | Evidence bundle support level |
| `baseline` | `BaselineSupport` | Baseline/regression support level |
| `strict_surface_support` | `bool` | Whether MCP/Agent/REST/gRPC surfaces support this domain |
| `docs_url` | `Option<&'static str>` | Documentation URL or path |

### DomainCategory

| Variant | Meaning |
|---------|---------|
| `StandardAssessment` | Standard scoped assessment (recon, scanning, fuzzing) |
| `DefenseLab` | Local/private defense validation and regression testing |
| `HazardousLab` | High-risk operations requiring explicit authorization |
| `FrontendAdapter` | Protocol bridge (REST, MCP, gRPC) |
| `OutputAdapter` | Output format adapter (reports, exports) |

### OperationIntegration

Each entry in the `operations` array describes one operation the domain
groups:

```rust
OperationIntegration {
    operation_id: "db-pentest",         // Must match an OperationMetadata.id
    display_name: "Database Pentesting",
    mode: OperationMode::DefenseLab,
    risk: OperationRisk::DbPentest,
    capabilities: &[Capability::DatabaseAssessment],
    intended_uses: &[IntendedUse::WebAssessment],
    required_features: &["db-pentest"],
    requires_explicit_scope: true,
    requires_private_or_local_target: false,
}
```

### ToolIntegration

Each entry in the `tools` array describes how an operation surfaces through
MCP/REST/gRPC:

```rust
ToolIntegration {
    tool_id: "db-pentest",
    operation_id: "db-pentest",
    mcp_exposed_by_default: false,
    required_mcp_feature: Some("db-pentest-mcp"),
}
```

### ReportIntegration

Each entry in the `reports` array describes how an operation produces
report output:

```rust
ReportIntegration {
    report_kind: "db-pentest",
    operation_id: "db-pentest",
    evidence_bundle_supported: true,
    normalized_report_supported: true,
}
```

## 2. When to Add a New Domain vs Extend an Existing One

**Add a new domain** when:

- You have a coherent group of operations that share a purpose (e.g.,
  "database security", "mobile analysis", "wireless assessment").
- The operations need distinct CLI commands, TUI tabs, or report types
  from existing domains.
- The operations require a separate feature gate.
- The domain has a distinct risk profile or category.

**Extend an existing domain** when:

- You are adding an operation that logically belongs to an existing
  capability area (e.g., adding a new database type check to `db-pentest`).
- The new operation shares the same CLI command, TUI tab, and feature gate
  as the existing domain.
- The operation's report output fits the existing report kind.

## 3. How required_feature Works

`required_feature` on a `DomainDescriptor` is a Cargo feature string that
controls compile-time availability:

- **`None`**: The domain is always compiled. Use this only for
  always-available domains with no system dependencies.
- **`Some("feature-name")`**: The domain is only compiled when the
  specified feature is enabled.

### Compile-time vs metadata declaration

A domain descriptor with `required_feature: Some("db-pentest")` is always
**declared** in `all_domain_descriptors()` -- it is always present in the
static registry regardless of feature state. However, its availability
depends on whether the feature is compiled:

```rust
// Always returns all declared domains, including those behind disabled features:
let all = all_domain_descriptors();  // includes db-pentest even without the feature

// Returns only domains whose features are compiled:
let available = available_domain_descriptors();  // excludes db-pentest if feature disabled
```

The `is_available()` method on `DomainDescriptor` checks this at runtime:

```rust
let descriptor = domain_descriptor_by_id("db-pentest").unwrap();
if descriptor.is_available() {
    // Feature is compiled, domain is usable
} else {
    // Feature is not compiled; availability_hint() returns a helpful message
    let hint = descriptor.availability_hint();
    // e.g., "enable the 'db-pentest' feature in Cargo.toml: cargo build --features db-pentest"
}
```

The `feature_enabled()` function maps feature strings to `cfg!()` checks:

```rust
fn feature_enabled(feature: &str) -> bool {
    match feature {
        "db-pentest" => cfg!(feature = "db-pentest"),
        "mobile" => cfg!(feature = "mobile"),
        "mobile-dynamic" => cfg!(feature = "mobile-dynamic"),
        "web-proxy" => cfg!(feature = "web-proxy"),
        // ...
        _ => false,
    }
}
```

When adding a new feature-gated domain, you must:

1. Add the feature string to `feature_enabled()` in `domain/mod.rs`.
2. Add a matching hint to `feature_missing_hint()` in `domain/mod.rs`.
3. Add the feature string to `KNOWN_EGGSEC_FEATURES` in
   `tests/feature_matrix.rs`.

### Operation-level features vs domain-level features

The `required_feature` on `DomainDescriptor` gates the entire domain. The
`required_features` on `OperationIntegration` and `OperationMetadata`
gate individual operations. A domain's feature string should also appear in
the `required_features` of each operation it groups. Operation-level
features take precedence for per-operation checks.

## 4. How Domain Operations Map to Global OperationMetadata

Every `operation_id` in a `DomainDescriptor`'s `operations` array must
have a corresponding entry in `ALL_OPERATION_METADATA` (defined in
`config/policy.rs`). This is enforced by the
`all_domain_operation_ids_have_metadata` unit test in `domain/mod.rs`.

The mapping works as follows:

1. `DomainDescriptor.operations` declares what operations the domain
   groups, with integration-specific metadata (mode, risk, capabilities).
2. `ALL_OPERATION_METADATA` declares the canonical per-operation metadata
   (risk, capabilities, feature gates, exposure flags).
3. `metadata_for_tool_id(operation_id)` resolves an operation ID to its
   canonical `OperationMetadata` (alias-aware).
4. `generate_capability_matrix()` cross-references both to produce the
   capability matrix.

**The domain and metadata must agree.** The metadata consistency tests
validate:

- Risk tiers match between domain `OperationIntegration` and
  `OperationMetadata`.
- Capabilities match between domain and metadata.
- Features declared on the domain are a subset of features in metadata.

When adding a new domain operation:

1. Add the `OperationMetadata` entry to `ALL_OPERATION_METADATA` first
   (see [operations.md](operations.md)).
2. Add the `OperationIntegration` to the domain's `operations` array.
3. Ensure `operation_id`, `risk`, `capabilities`, and `required_features`
   are consistent across both.

## 5. How Domain ToolIntegration Affects MCP Default Visibility

`ToolIntegration` on a domain has two fields that control MCP visibility:

| Field | Purpose |
|-------|---------|
| `mcp_exposed_by_default` | Whether the tool appears in the **conservative default** MCP listing |
| `required_mcp_feature` | An additional feature required for MCP exposure (beyond the domain's own feature) |

### MCP default visibility vs opt-in domain exposure

There are two distinct layers of MCP visibility:

- **`mcp_exposed_by_default: true`** on `ToolIntegration`: The tool appears
  in `mcp_tool_registrations_default_visible()`. This is a strict
  conservative subset restricted to passive/safe-active operations with no
  feature gate. Hazardous domains must **never** set this to `true`.

- **`mcp_metadata_exposable`** on `OperationMetadata`: The tool appears in
  `mcp_tool_registrations("ops-agent")`. This is the profile-expanded
  listing that includes high-risk operations. Runtime policy still gates
  execution.

The relationship between domain `ToolIntegration` and `OperationMetadata`:

| Domain ToolIntegration | OperationMetadata | MCP behavior |
|------------------------|-------------------|--------------|
| `mcp_exposed_by_default: true` | `mcp_exposable: true` | Appears in both conservative default and OpsAgent listings |
| `mcp_exposed_by_default: false` | `mcp_exposable: true` | Appears in OpsAgent listing only (opt-in, not conservative default) |
| `mcp_exposed_by_default: false` | `mcp_exposable: false` | Not exposed via MCP |

The `required_mcp_feature` field adds an additional compile-time gate
beyond the domain's own feature. For example, `db-pentest` requires the
`db-pentest` feature for the domain and `db-pentest-mcp` for MCP exposure.
This allows decoupling domain availability from MCP exposure.

### Safety rule

No hazardous domain may set `mcp_exposed_by_default: true`. This is
enforced by the `hazardous_domains_not_mcp_exposed_by_default` metadata
consistency test.

## 6. How all_domain_descriptors() and available_domain_descriptors() Differ

### all_domain_descriptors()

Returns a static slice of **all known** domain descriptors, regardless of
compile-time feature state. Domains behind disabled features are still
included. Their `required_feature` field indicates gating.

Use this when:

- Building the capability matrix (includes all known domains).
- Displaying the full domain registry.
- Checking whether a domain ID is valid.

### available_domain_descriptors()

Returns a `Vec` of only domains whose `required_feature` is `None` or
whose feature is currently compiled. This filters `all_domain_descriptors()`
to actionable domains.

Use this when:

- Listing domains for user-facing selection (TUI, CLI help).
- Building tool registrations for protocol surfaces.
- Checking which domain operations are actually runnable.

### domain_descriptor_by_id()

Looks up a single domain by ID from `all_domain_descriptors()`. Returns
`None` for unknown IDs. The returned descriptor may or may not be
available -- check `is_available()` before using domain-specific
functionality.

```rust
use crate::domain::{all_domain_descriptors, available_domain_descriptors, domain_descriptor_by_id};

// Full registry (always returns all 3+ domains)
let all = all_domain_descriptors();

// Only actionable domains
let available = available_domain_descriptors();

// Specific lookup
if let Some(d) = domain_descriptor_by_id("db-pentest") {
    if d.is_available() {
        // use domain
    } else {
        eprintln!("{}", d.availability_hint().unwrap_or("unknown feature"));
    }
}
```

## 7. Required Tests

### Unit tests in domain/mod.rs

Add unit tests inside `#[cfg(test)] mod tests` in `domain/mod.rs`:

```rust
#[cfg(feature = "your-feature")]
mod your_domain_tests {
    use super::*;

    #[test]
    fn your_domain_descriptor_exists() {
        let d = YOUR_DOMAIN_DESCRIPTOR;
        assert_eq!(d.id, "your-domain");
    }

    #[test]
    fn your_domain_category_is_correct() {
        assert_eq!(YOUR_DOMAIN_DESCRIPTOR.category, DomainCategory::DefenseLab);
    }

    #[test]
    fn your_domain_requires_feature() {
        assert_eq!(YOUR_DOMAIN_DESCRIPTOR.required_feature, Some("your-feature"));
    }

    #[test]
    fn your_domain_operation_has_metadata() {
        assert!(metadata_for_tool_id("your-operation").is_some());
    }

    #[test]
    fn your_domain_is_const_constructible() {
        const _: DomainDescriptor = YOUR_DOMAIN_DESCRIPTOR;
    }
}
```

### Cross-cutting tests (always run)

These tests in `domain/mod.rs` validate all domains automatically:

| Test | What it checks |
|------|----------------|
| `all_domain_operation_ids_have_metadata` | Every domain operation has an `OperationMetadata` entry |
| `domain_ids_are_unique` | No duplicate domain IDs |
| `domain_operation_ids_within_domain_are_unique` | No duplicate operation IDs within a domain |
| `feature_missing_hint_returns_something_for_known_features` | Every known feature has a diagnostic hint |
| `domain_is_available_matches_feature_state` | `is_available()` matches `cfg!()` |
| `capability_matrix_generation_works` | Matrix rows have non-empty fields |

### Metadata consistency tests

```bash
cargo test -p eggsec --test metadata_consistency
```

Validates:

- Domain operation IDs resolve to `OperationMetadata`.
- Domain and metadata risk tiers agree.
- Domain and metadata capabilities agree.
- Domain features are a subset of metadata features.
- No hazardous domains are MCP-exposed by default.
- MCP exposure in domain matches metadata flag.
- Domains with `normalized_report_supported: true` have report integration.
- Domain docs URLs reference existing files.

### Feature matrix tests

```bash
cargo test -p eggsec --test feature_matrix
```

Validates:

- Feature strings in metadata match actual Cargo features.
- `KNOWN_EGGSEC_FEATURES` is in sync.

### Tool registration tests

```bash
cargo test -p eggsec --test tool_registration
```

Validates:

- Tool registration metadata is consistent across MCP, REST, gRPC, and
  agent surfaces.
- Domain `ToolIntegration` entries align with `OperationMetadata` exposure
  flags.

### Full validation

```bash
cargo test --lib -p eggsec
cargo test -p eggsec --test metadata_consistency
cargo test -p eggsec --test feature_matrix
cargo test -p eggsec --test tool_registration
cargo clippy --lib -p eggsec
```

## Checklist

When adding or modifying a domain:

- [ ] Add or update the `DomainDescriptor` const in `domain/mod.rs`
- [ ] Add `OperationMetadata` entries for every domain operation in
      `config/policy.rs` (see [operations.md](operations.md))
- [ ] Add feature gate to `feature_enabled()` and
      `feature_missing_hint()` in `domain/mod.rs` if optional
- [ ] Add feature string to `KNOWN_EGGSEC_FEATURES` in
      `tests/feature_matrix.rs`
- [ ] Add `ToolIntegration` only when protocol listing is needed (MCP,
      REST, gRPC, or agent exposure)
- [ ] Set `mcp_exposed_by_default: false` for hazardous or high-risk
      domains
- [ ] Add `ReportIntegration` if the domain produces report output
- [ ] Add unit tests in `domain/mod.rs` for the new descriptor
- [ ] Add integration tests in `tests/metadata_consistency.rs` if
      adding new safety invariants
- [ ] Update `docs/CAPABILITY_MATRIX.md` with new rows
- [ ] Run metadata/feature/tool registration tests:

```bash
cargo test --lib -p eggsec
cargo test -p eggsec --test metadata_consistency
cargo test -p eggsec --test feature_matrix
cargo test -p eggsec --test tool_registration
cargo clippy --lib -p eggsec
```

## Skeleton Example

```rust
// ─── Domain: my-domain ──────────────────────────────────────────────────────

#[allow(dead_code)]
const MY_DOMAIN_OPERATION: OperationIntegration = OperationIntegration {
    operation_id: "my-domain-check",
    display_name: "My Domain Check",
    mode: OperationMode::DefenseLab,
    risk: OperationRisk::SafeActive,
    capabilities: &[Capability::ActiveProbe],
    intended_uses: &[IntendedUse::WebAssessment],
    required_features: &["my-domain"],
    requires_explicit_scope: true,
    requires_private_or_local_target: false,
};

#[allow(dead_code)]
const MY_DOMAIN_CLI: CliIntegration = CliIntegration {
    command_id: "my-domain",
    operation_id: "my-domain-check",
    feature: Some("my-domain"),
};

#[allow(dead_code)]
const MY_DOMAIN_TUI: TuiIntegration = TuiIntegration {
    tab_id: "my-domain",
    operation_id: "my-domain-check",
    feature: Some("my-domain"),
};

#[allow(dead_code)]
const MY_DOMAIN_TOOL: ToolIntegration = ToolIntegration {
    tool_id: "my-domain-check",
    operation_id: "my-domain-check",
    mcp_exposed_by_default: false,
    required_mcp_feature: None,
};

#[allow(dead_code)]
const MY_DOMAIN_REPORT: ReportIntegration = ReportIntegration {
    report_kind: "my-domain",
    operation_id: "my-domain-check",
    evidence_bundle_supported: true,
    normalized_report_supported: false,
};

#[allow(dead_code)]
const MY_DOMAIN_DESCRIPTOR: DomainDescriptor = DomainDescriptor {
    id: "my-domain",
    display_name: "My Domain",
    description: "Description of what this domain does",
    category: DomainCategory::DefenseLab,
    required_feature: Some("my-domain"),
    operations: &[MY_DOMAIN_OPERATION],
    cli: &[MY_DOMAIN_CLI],
    tui: &[MY_DOMAIN_TUI],
    tools: &[MY_DOMAIN_TOOL],
    reports: &[MY_DOMAIN_REPORT],
    dry_run: DryRunSupport::AlwaysAvailable,
    evidence: EvidenceSupport::NotSupported,
    baseline: BaselineSupport::NotSupported,
    strict_surface_support: true,
    docs_url: Some("docs/MY_DOMAIN.md"),
};
```

Then register in `all_domain_descriptors()`:

```rust
pub fn all_domain_descriptors() -> &'static [DomainDescriptor] {
    &[
        // ── Standard Assessment ──
        // ── Defense Lab ──
        DB_PENTEST_DESCRIPTOR,
        MOBILE_STATIC_DESCRIPTOR,
        MOBILE_DYNAMIC_DESCRIPTOR,
        MY_DOMAIN_DESCRIPTOR,  // <-- add here
        // ── Hazardous Lab ──
        // ── Adapters ──
    ]
}
```

## Warnings

- **Do not conflate compile-time availability with runtime authorization.**
  A domain being compiled (feature enabled) does not mean its operations
  are authorized. `EnforcementContext::evaluate()` and `ApprovedOperation`
  tokens are always required before dispatch on strict surfaces.

- **Do not set `mcp_exposed_by_default: true` for high-risk or hazardous
  domains.** This field controls visibility in the conservative default MCP
  listing, which assumes low-risk operations. High-risk operations should
  use `mcp_exposed_by_default: false` with `mcp_exposable: true` on
  `OperationMetadata` for opt-in OpsAgent visibility.

- **Do not skip OperationMetadata.** A domain operation without a matching
  `OperationMetadata` entry will fail the
  `all_domain_operation_ids_have_metadata` test. The metadata is the
  canonical source for risk, capabilities, and exposure flags.

- **Domain metadata is data, not policy.** Never add authorization logic,
  scope checking, or policy evaluation to `DomainDescriptor` or its
  associated types. The `#[allow(dead_code)]` on domain constants is
  intentional -- they are referenced by the registry and tests but not by
  runtime dispatch logic.
