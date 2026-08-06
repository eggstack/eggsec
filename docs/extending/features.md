# Adding Cargo Features and Keeping the Feature Matrix Valid

This guide explains how to add a new Cargo feature to the eggsec workspace and
keep the feature matrix, CI checks, and metadata cross-references consistent.

See also:

- [FEATURE_MATRIX.md](../FEATURE_MATRIX.md) for the canonical feature inventory
  and classification table.
- [METADATA_OWNERSHIP.md](../METADATA_OWNERSHIP.md) for the ownership model
  and validation pipeline.
- [domains.md](domains.md) for adding domain crates and DomainDescriptor entries.
- [operations.md](operations.md) for adding OperationMetadata entries.

## Core Invariant

**Feature-gated metadata still exists; availability and execution are separate
concerns.** Enabling a feature makes code compile and metadata visible, but
runtime policy (`EnforcementContext`) is always required before dispatch. A
feature flag never grants authorization.

## 1. Declare the Feature in Cargo.toml

All main-crate features live in `crates/eggsec/Cargo.toml` under `[features]`.

```toml
[features]
default = []

# My new feature
my-new-feature = ["dep:some-crate", "other-feature"]
```

Rules:

- Feature names must be kebab-case (lowercase letters, digits, hyphens only).
- Must not start or end with a hyphen.
- Must not contain consecutive hyphens.
- If the feature pulls in optional dependencies, list them as `dep:name` or
  reference them by name if they appear in `[dependencies]` with `optional = true`.
- If the feature depends on other eggsec features, list them by name (e.g.,
  `"tool-api"`, `"nse"`).

Domain crate features are forwarded via the main crate feature. For example,
`db-pentest` enables `eggsec-db-lab/db-drivers`:

```toml
db-pentest = ["sqlx", "dep:eggsec-db-lab", "eggsec-db-lab/db-drivers"]
```

## 2. Add to the Authoritative Feature Registry

The **single source of truth** for all feature metadata is the `feature_registry!`
macro in `crates/eggsec/src/config/feature_registry.rs`. Every feature declared
in `Cargo.toml [features]` (except `default`) must appear here exactly once.

Add a new entry to the `feature_registry!` invocation:

```rust
feature_registry! {
    // ... existing entries ...
    "my-new-feature" => {
        enabled: cfg!(feature = "my-new-feature"),
        category: DomainCapability,
        hint: "enable feature 'my-new-feature' in Cargo.toml: cargo build --features my-new-feature"
    },
}
```

### Classification Categories

| Category | `FeatureCategory` variant | Meaning | Examples |
|----------|--------------------------|---------|----------|
| **Protocol adapter** | `ProtocolAdapter` | Exposes a serving surface (REST, gRPC, WebSocket) | `tool-api`, `rest-api`, `grpc-api`, `ws-api`, `websocket` |
| **Domain capability** | `DomainCapability` | Enables a domain's core functionality | `db-pentest`, `mobile`, `wireless`, `web-proxy`, `evasion`, `postex`, `c2`, `nse` |
| **Protocol exposure marker** | `ProtocolExposure` | Opt-in MCP/agent exposure for a domain | `db-pentest-mcp`, `web-proxy-mcp`, `c2-mcp` |
| **Marker-only** | `MarkerOnly` | No dependencies; compile-time gate only | `advanced-hunting`, `compliance`, `cloud`, `git-secrets`, `api-schema` |
| **Backend driver** | `BackendDriver` | Pulls in a specific driver crate for a domain | `db-pentest-mssql-tiberius`, `db-pentest-mongodb`, `db-pentest-redis` |
| **Platform-sensitive** | `PlatformSensitive` | Requires root, CAP_NET_ADMIN, libpcap, or system libs | `stress-testing`, `packet-inspection`, `nse-ssh2`, `nse-sandbox`, `headless-browser` |
| **Storage/integration** | `StorageIntegration` | Adds persistence or external service integration | `database`, `sbom`, `container`, `pdf` |
| **Aggregate** | `Aggregate` | Meta-feature enabling many sub-features | `full` |
| **Security risk** | `SecurityRisk` | Introduces security vulnerabilities; lab-only | `insecure-tls` |
| **AI integration** | `AiIntegration` | AI/LLM analysis and payload generation | `ai-integration` |
| **Advanced extension** | `AdvancedExtension` | Extends a base domain with advanced capabilities | `mobile-dynamic`, `wireless-advanced`, `transparent-proxy`, `dynamic-plugins` |

### Decision Guide

Ask these questions in order:

1. **Does it expose a network serving surface (REST, gRPC, WebSocket)?**
   -> `ProtocolAdapter`

2. **Does it enable a security assessment domain's core capability?**
   -> `DomainCapability`

3. **Does it opt a domain into MCP/agent exposure?**
   -> `ProtocolExposure`

4. **Does it pull in a specific database or service driver?**
   -> `BackendDriver`

5. **Does it require root, CAP_NET_ADMIN, libpcap, or other system deps?**
   -> `PlatformSensitive`

6. **Does it add persistence or report output formats?**
   -> `StorageIntegration`

7. **Does it extend a base domain with advanced/lab-only capabilities?**
   -> `AdvancedExtension`

8. **Is it a meta-feature enabling many others?**
   -> `Aggregate`

9. **Does it introduce security vulnerabilities?**
   -> `SecurityRisk`

10. **Does it integrate AI/LLM analysis?**
    -> `AiIntegration`

11. **Is it a compile-time gate with no dependencies?**
    -> `MarkerOnly`

The `hint` field should be a user-facing message naming the exact Cargo feature
flag needed, e.g. `"enable feature 'my-new-feature' in Cargo.toml: cargo build --features my-new-feature"`.

**This is required.** The `all_cargo_features_are_in_registry` test in
`tests/feature_matrix.rs` validates that every feature in `Cargo.toml [features]`
appears in the registry. If you add a feature to `Cargo.toml` without updating
the registry, CI will fail.

## 3. Add Dependency Edges

If your feature depends on other features (implied-by relationships), add
entries to `FEATURE_DEPENDENCIES` in `crates/eggsec/tests/feature_matrix.rs`:

```rust
static FEATURE_DEPENDENCIES: &[(&str, &str)] = &[
    // ... existing edges ...
    // my-new-feature depends on tool-api
    ("my-new-feature", "tool-api"),
];
```

Each entry is `(feature, depends_on)`. The `no_circular_feature_dependencies`
test runs a DFS over this graph to detect cycles.

Rules:

- Every `dep:` optional dependency that is feature-activated does not need an
  edge. Edges are only for feature-to-feature dependencies.
- Protocol exposure markers (e.g., `db-pentest-mcp`) must have edges to their
  base domain feature (e.g., `db-pentest`). This is enforced by
  `protocol_exposure_markers_require_base_domain`.
- If the feature belongs in the `full` aggregate, add a `("full", "my-new-feature")`
  edge.

## 4. Decide: Required PR Feature-Profile Check vs. Deep Check

CI runs two tiers of feature checks:

### Representative feature checks

These run in the optional weekly/manual `deep-checks.yml` workflow and locally
through `make check-full`. They are representative compile profiles, not an
exhaustive all-feature matrix or a per-PR required check.

**Include your feature in the PR matrix if:**

- It is a domain capability that users will build with independently.
- It is a protocol adapter with significant new dependencies.
- It is a backend driver that changes the compilation graph.
- It is an advanced extension of an existing domain.

**How to add:** Add a `cargo check` entry to the `check-feature-profiles` target
in the `Makefile`.

```makefile
# Makefile
check-feature-profiles:
	# ... existing checks ...
	cargo check -p eggsec --features my-new-feature
```

### Deep checks (weekly/manual)

These run weekly via `.github/workflows/deep-checks.yml` or locally via
`make check-full`. They run `cargo deny check` (advisories, licenses, bans) and
representative feature profiles.

**Features that belong only in deep checks:**

- Marker-only features with no dependencies (`cloud`, `git-secrets`).
- Features that require system deps not available in CI (e.g., `mobile-dynamic`
  needs ADB + Android device).
- Features that are subsets of `full` and do not introduce independent build
  paths.

Platform-sensitive profiles like `mobile-dynamic` may fail in CI due to missing
system dependencies. These are tested in deep checks with `continue-on-error: true`.

## 5. Document Platform-Sensitive Dependencies

If your feature requires system-level dependencies (libraries, root, capabilities),
document them in `docs/FEATURE_MATRIX.md` under section 3.2 "System Dependency
Requirements":

```markdown
| Profile | Required System Dep | Install (Debian/Ubuntu) |
|---------|-------------------|------------------------|
| `my-new-feature` | libfoo-dev | `apt install libfoo-dev` |
```

Also add a comment in `Cargo.toml` near the feature declaration:

```toml
# My new feature (requires libfoo-dev: apt install libfoo-dev)
my-new-feature = ["dep:foo"]
```

If the feature requires root or special capabilities, note it in the feature
comment and in `FEATURE_MATRIX.md`.

## 6. Update Metadata (If Applicable)

Features that enable operations or domain integrations have additional
metadata to update:

### OperationMetadata

If the feature enables a new operation, add an entry to `ALL_OPERATION_METADATA`
in `crates/eggsec/src/config/policy.rs`. Set `required_features` to include
your feature string. See [operations.md](operations.md) for the full process.

### DomainDescriptor

If the feature enables a new domain, add a `DomainDescriptor` entry in
`crates/eggsec/src/domain/mod.rs`. Set `required_feature` to your feature
string. See [domains.md](domains.md) for the full process.

### Feature-to-Metadata Cross-Reference

Update the "Feature-to-Metadata Cross-Reference" table in
`docs/FEATURE_MATRIX.md` with the new feature's operation and domain IDs.

### Documentation

Update `docs/FEATURE_MATRIX.md` section 1.1 to include
the new feature in the main crate feature table with its category, implied
features, and metadata IDs.

## 7. Required Tests

After adding your feature, run these tests to verify consistency:

```bash
# Feature matrix validation (registry, classification, dependencies, naming)
cargo test -p eggsec --test feature_matrix

# Metadata cross-reference validation
cargo test -p eggsec --test metadata_consistency

# No-default-features build still works
cargo check --workspace --no-default-features

# Your feature compiles
cargo check -p eggsec --features my-new-feature

# Unit tests pass with your feature enabled
cargo test --lib -p eggsec --features my-new-feature
```

The full architecture guard CI reproduction can be run locally:

```bash
make check
```

### What the Tests Validate

| Test | Validates |
|------|-----------|
| `all_cargo_features_are_in_registry` | Every `Cargo.toml` feature (except `default`) is in the authoritative registry |
| `registry_features_exist_in_cargo_toml` | Every registry feature exists in `Cargo.toml [features]` |
| `operation_metadata_required_features_resolve` | OperationMetadata `required_features` resolve through production lookup |
| `domain_descriptor_required_features_resolve` | DomainDescriptor `required_feature` resolves through production lookup |
| `domain_mcp_features_resolve` | DomainDescriptor `required_mcp_feature` resolves through production lookup |
| `command_registry_features_resolve` | Command registry feature gates resolve through production lookup |
| `feature_names_follow_naming_conventions` | All names are kebab-case; MCP markers have valid base features |
| `no_circular_feature_dependencies` | `FEATURE_DEPENDENCIES` graph has no cycles |
| `aggregate_feature_includes_domain_features` | `full` includes all domain capabilities |
| `protocol_exposure_markers_require_base_domain` | MCP markers depend on their base domain feature |
| `unknown_feature_fails_closed` | Unknown feature names return `FeatureState::Unknown` and `false` for `is_feature_enabled` |

## Checklist

When adding a new feature, verify each item:

- [ ] Feature declared in `crates/eggsec/Cargo.toml` `[features]`
- [ ] Feature added to `feature_registry!` in `crates/eggsec/src/config/feature_registry.rs`
- [ ] Dependency edges added to `FEATURE_DEPENDENCIES` in `tests/feature_matrix.rs`
- [ ] If in `full` aggregate: `("full", "my-new-feature")` edge added
- [ ] `cargo test -p eggsec --test feature_matrix` passes
- [ ] `cargo test -p eggsec --test metadata_consistency` passes
- [ ] `cargo check -p eggsec --features my-new-feature` compiles
- [ ] Platform-sensitive deps documented in `FEATURE_MATRIX.md` section 3.2
- [ ] Feature table updated in `FEATURE_MATRIX.md` section 1.1
- [ ] If required for PR checks: added to `Makefile` `check-feature-profiles` target

## Warnings

**Adding a feature without updating the registry should fail CI.** The
`all_cargo_features_are_in_registry` test enforces bidirectional consistency
between the feature registry and `Cargo.toml [features]`. If you add a
feature to `Cargo.toml` and forget the registry, CI fails. If you add to
the registry and forget `Cargo.toml`, CI fails.

**Unknown features fail closed.** The `feature_state()` function returns
`FeatureState::Unknown` for unrecognized names, and `is_feature_enabled()`
returns `false`. This means typos in feature names will deny operations
rather than silently allowing them. The `unknown_feature_fails_closed` test
verifies this behavior.

**`full` is an aggregate/deep profile, not a conservative/default profile.** The
`full` meta-feature enables all non-default features including advanced/lab-only
capabilities (`wireless-advanced`, `evasion`, `postex`, `c2`, `mobile-dynamic`).
It is intended for development, integration testing, and explicit lab builds.
Never recommend `full` as a default or production build profile. The default
feature set is empty (`default = []`).

**Feature-gated metadata still exists; availability and execution are separate
concerns.** Enabling a feature makes code compile and metadata visible to
protocol surfaces, but runtime policy (`EnforcementContext`) is always required
before dispatch. `--allow-*` flags, scope rules, and confirmation prompts are
mandatory for side-effecting operations. Feature presence is not authorization.
