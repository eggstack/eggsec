# Adding New OperationMetadata

This guide explains how to add a new `OperationMetadata` entry to the static
registry in `crates/eggsec/src/config/policy.rs`. Every tool that performs a
side-effecting or analytical operation must have a corresponding metadata entry.
The metadata drives policy enforcement, preflight evaluation, surface exposure,
and descriptor generation across CLI, TUI, REST, MCP, gRPC, and agent surfaces.

## What OperationMetadata Owns

Each entry in `ALL_OPERATION_METADATA` is a `const` struct with these fields:

| Field | Type | Purpose |
|-------|------|---------|
| `id` | `&'static str` | Canonical operation ID (kebab-case) |
| `display_name` | `&'static str` | Human-readable label |
| `mode` | `OperationMode` | Execution environment classification |
| `risk` | `OperationRisk` | Risk tier for policy evaluation |
| `intended_uses` | `&'static [IntendedUse]` | Intended assessment context |
| `required_features` | `&'static [&'static str]` | Cargo feature strings that must be enabled |
| `required_policy_flags` | `&'static [&'static str]` | Policy flag strings required at runtime |
| `required_capabilities` | `&'static [Capability]` | Capabilities the operation requires |
| `target_policy` | `TargetPolicyKind` | Target requirement classification |
| `manual_exposable` | `bool` | CLI/TUI surfaces may register this operation |
| `tui_exposable` | `bool` | TUI tab may render this operation |
| `mcp_exposable` | `bool` | MCP surface may register this operation |
| `rest_exposable` | `bool` | REST API may expose this operation |
| `agent_exposable` | `bool` | Agent surface may register this operation |
| `grpc_exposable` | `bool` | gRPC surface may expose this operation |

The metadata is a static `const` array. It never performs I/O, never checks
authorization, and never depends on runtime state. It is a data-only contract.

## How Operation IDs and Aliases Work

**Operation IDs** are kebab-case strings that uniquely identify an operation in
the registry. They are the canonical name used by `operation_metadata(id)` for
direct lookup.

Examples: `"scan-ports"`, `"fuzz"`, `"db-pentest"`, `"mobile-static"`.

**Aliases** are alternate names that resolve to a canonical operation ID. They
live in `ALL_OPERATION_METADATA_ALIASES` as `(&str, &str)` tuples of
`(alias_id, canonical_id)`. The `metadata_for_tool_id(tool_id)` function first
tries a direct canonical lookup, then falls through to the alias table.

Examples:

| Alias | Canonical |
|-------|-----------|
| `"scan"` | `"scan-ports"` |
| `"waf"` | `"waf-detect"` |
| `"fuzzer"` | `"fuzz"` |
| `"mobile"` | `"mobile-static"` |

Rules:

- IDs must be unique across the registry (enforced by
  `every_metadata_id_is_unique` test).
- Aliases must not map to themselves.
- Every alias must resolve to a known canonical ID (enforced by
  `all_aliases_resolve_to_known_metadata` test).
- Every registered tool in `create_default_registry()` must have a metadata
  entry or alias (enforced by `every_registered_tool_has_operation_metadata`
  test).

## How to Select OperationRisk

`OperationRisk` is a `PartialOrd` enum with ordering derived from declaration.
Lower variants are safer; higher variants require stricter policy approval.

Ordered from safest to most dangerous:

1. `Passive` -- read-only, no network interaction
2. `SafeActive` -- active probes, standard scanning
3. `Intrusive` -- fuzzing, bypass simulation, exploitation-adjacent
4. `LoadTest` -- controlled load generation
5. `StressTest` -- flood-style stress testing
6. `RawPacket` -- raw packet crafting and injection
7. `CredentialTesting` -- brute force, credential stuffing
8. `DbPentest` -- database penetration testing
9. `TrafficInterception` -- MITM proxy operations
10. `EvasionTesting` -- defense evasion simulation
11. `PostExploitation` -- post-exploitation simulation
12. `ExploitAdjacent` -- exploit-adjacent operations
13. `C2Operation` -- command-and-control simulation
14. `RemoteExecution` -- remote code execution
15. `AgentAutonomous` -- autonomous agent operation

Selection criteria:

- **`Passive`**: Only for operations that read data without sending anything to
  the target (e.g., cached lookups, local file parsing).
- **`SafeActive`**: Standard active operations that send probes to targets but
  do not attempt exploitation. This is the default safe tier.
- **`Intrusive`**: Operations that attempt exploitation, bypass controls, or
  fuzz inputs. Requires explicit policy approval.
- **Higher tiers**: Reserved for specialized operations that perform
  destructive, resource-intensive, or adversarial actions.

The `OperationMode` constrains the maximum risk allowed by default:

| Mode | Default Max Risk |
|------|-----------------|
| `StandardAssessment` | `SafeActive` |
| `DefenseLab` | `Intrusive` |
| `HazardousLab` | `AgentAutonomous` |

If your operation requires `risk > SafeActive`, set `mode` to `DefenseLab` or
`HazardousLab` as appropriate.

## How to Select Required Capability Values

`Capability` values declare what the operation needs to function. They are used
by policy enforcement to gate operations on capability permissions.

Available capabilities:

| Capability | Typical Use |
|-----------|-------------|
| `PassiveFingerprint` | Recon, technology detection |
| `ActiveProbe` | Port scanning, active probing |
| `Crawl` | Endpoint discovery, content crawling |
| `HttpFuzzLowImpact` | HTTP fuzzing (non-intrusive payloads) |
| `IntrusiveFuzz` | Intrusive fuzzing, boundary testing |
| `WafDetect` | WAF identification |
| `WafBypassSimulation` | WAF bypass technique testing |
| `WafStressTest` | WAF stress and load testing |
| `LoadTest` | HTTP load testing |
| `RawPacketProbe` | Raw packet operations |
| `CredentialTesting` | Brute force, credential stuffing |
| `RemoteExecution` | Remote command execution |
| `NseSafe` | Safe NSE script execution |
| `NseIntrusive` | Intrusive NSE script execution |
| `TrafficInterception` | MITM proxy interception |
| `EvasionTesting` | Defense evasion simulation |
| `DatabaseAssessment` | Database security testing |
| `C2Simulation` | C2 framework simulation |
| `MobileDynamicAnalysis` | Mobile dynamic analysis |

Rules:

- High-risk operations (`risk > SafeActive`) **must** declare at least one
  non-baseline capability. Baseline capabilities are `PassiveFingerprint`,
  `ActiveProbe`, `Crawl`, and `WafDetect`. This is enforced by the
  `high_risk_ops_declare_nonbaseline_capability` test.
- An empty `required_capabilities` array is allowed only for low-risk,
  capability-independent operations (e.g., `search`, `mobile-static`).
- If the operation requires a new capability that does not exist in the
  `Capability` enum, you must add it first and update all downstream
  consumers.

## How to Set Exposure Flags

The six boolean flags control which surfaces may register the operation:

| Flag | Surface |
|------|---------|
| `manual_exposable` | CLI and TUI |
| `tui_exposable` | TUI tab rendering |
| `mcp_exposable` | MCP tool registration |
| `rest_exposable` | REST API endpoint |
| `agent_exposable` | Agent tool registration |
| `grpc_exposable` | gRPC service registration |

These flags mean the operation **may** be registered on the surface when the
required feature is compiled and runtime policy approves. They do **not** mean:

- The operation is safe by default.
- The operation is baseline-agent-safe.
- `EnforcementContext::evaluate()` will permit dispatch.
- The operation appears in default/conservative listings.

Additional constraints:

- Agent-exposable and MCP-exposable operations that require a target **must**
  set `target_policy` to `ExplicitScopeRequired` or `PrivateOrLocalRequired`.
  This is enforced by `agent_exposable_ops_require_explicit_scope`.
- Feature-gated operations should only set `mcp_exposable`/`rest_exposable`/
  `agent_exposable` to `true` if they are safe for the corresponding strict
  profiles.
- `mobile-static` and `mobile-dynamic` set all programmatic flags to `false`
  because they operate on local files, not network targets.

Common patterns:

- Base operations available everywhere: set all flags to `true`.
- TUI-only or CLI-only operations: set `manual_exposable: true`,
  `tui_exposable: true`, all programmatic flags to `false`.
- Feature-gated operations that are not yet ready for MCP: set
  `mcp_exposable: false` and `agent_exposable: false`.

## How to Set required_features

`required_features` is a list of Cargo feature strings that must be enabled at
compile time for the operation to be available. The strings must match the
feature names in `Cargo.toml` exactly.

Examples:

| Operation | required_features |
|-----------|-------------------|
| `nse` | `["nse"]` |
| `db-pentest` | `["db-pentest"]` |
| `proxy-intercept` | `["web-proxy"]` |
| `mobile-static` | `["mobile"]` |
| `mobile-dynamic` | `["mobile-dynamic"]` |
| `wireless` | `["wireless"]` |
| `c2` | `["c2"]` |

Rules:

- An empty `&[]` means the operation is always available (no feature gate).
- Feature strings must not be empty (enforced by
  `feature_gated_ops_declare_feature_name` test).
- The feature must exist in the workspace `Cargo.toml`. The
  `tests/feature_matrix.rs` test validates that feature strings in metadata
  match actual Cargo features.
- If you add a new feature-gated operation, update `KNOWN_EGGSEC_FEATURES` in
  the feature matrix test.

## How Metadata Interacts with OperationDescriptor

`OperationMetadata` is the static data source. `OperationDescriptor` is the
runtime value that gets passed to `EnforcementContext::evaluate()` for policy
decisions.

The `descriptor_for_target(target)` method generates an `OperationDescriptor`
from metadata:

```rust
let metadata = metadata_for_tool_id("scan-ports").unwrap();
let descriptor = metadata.descriptor_for_target(Some("192.168.1.1".to_string()));
```

The descriptor copies:

- `operation` from `id`
- `mode` from `mode`
- `risk` from `risk`
- `intended_uses` from `intended_uses`
- `target` from the argument
- `required_features` from `required_features`
- `required_policy_flags` from `required_policy_flags`
- `required_capabilities` from `required_capabilities`
- `requires_explicit_scope` derived from `target_policy`
- `requires_private_or_local_target` derived from `target_policy`

There is also `descriptor_for_target_with_risk(target, risk)` which allows
overriding the risk tier for dry-run or tab-specific adjustments.

All surfaces (CLI, TUI, REST, MCP, agent, gRPC) use
`metadata_for_tool_id()` to look up canonical metadata and
`descriptor_for_target()` to build descriptors. No surface constructs
descriptors inline -- the metadata is the single source of truth.

## Required Tests

After adding a new `OperationMetadata` entry, verify it passes all existing
tests:

### Unit tests in `config/policy.rs`

```bash
cargo test --lib -p eggsec -- config::policy::operation_metadata_tests
```

These validate:

- Non-empty `id` and `display_name` on every entry.
- Unique IDs across the registry.
- Agent/MCP-exposable ops require explicit scope.
- Feature-gated ops declare non-empty feature strings.
- Descriptor generation matches metadata fields.
- High-risk operations declare non-baseline capabilities.
- Every registered tool has metadata.
- Alias lookups resolve correctly.

### Metadata consistency tests

```bash
cargo test -p eggsec --test metadata_consistency
```

These validate cross-references between `OperationMetadata` and
`DomainDescriptor`:

- Every domain operation ID resolves to an `OperationMetadata` entry.
- Domain and metadata risk tiers agree.
- Domain and metadata capabilities agree.
- All aliases resolve to known metadata.
- All `Capability` and `OperationRisk` variants appear in metadata.
- No hazardous domains are MCP-exposed by default.

### Feature matrix tests

```bash
cargo test -p eggsec --test feature_matrix
```

These validate:

- Feature strings in `required_features` match actual Cargo features.
- `KNOWN_EGGSEC_FEATURES` is kept in sync with new features.

### Full validation

```bash
cargo test --lib -p eggsec
cargo test -p eggsec --test metadata_consistency
cargo test -p eggsec --test feature_matrix
cargo clippy --lib -p eggsec
```

## Skeleton Example

```rust
OperationMetadata {
    id: "example-check",
    display_name: "Example Check",
    mode: OperationMode::StandardAssessment,
    risk: OperationRisk::SafeActive,
    intended_uses: &[IntendedUse::WebAssessment],
    required_features: &[],
    required_policy_flags: &[],
    required_capabilities: &[Capability::ActiveProbe],
    target_policy: TargetPolicyKind::ExplicitScopeRequired,
    manual_exposable: true,
    tui_exposable: true,
    mcp_exposable: true,
    rest_exposable: true,
    agent_exposable: true,
    grpc_exposable: false,
}
```

Aliases are registered separately in `ALL_OPERATION_METADATA_ALIASES`:

```rust
pub static ALL_OPERATION_METADATA_ALIASES: &[(&str, &str)] = &[
    // ... existing aliases ...
    ("example", "example-check"),
];
```

## Warnings

- **Do not mark a tool programmatic-exposable just because it is
  manual-exposable.** `manual_exposable` controls CLI/TUI. `rest_exposable`,
  `mcp_exposable`, `agent_exposable`, and `grpc_exposable` are independent
  gates. Setting them all to `true` just because `manual_exposable` is `true`
  exposes the tool on strict automated surfaces where manual overrides are not
  available.

- **Do not use `mcp_exposable` to mean default-visible.** `mcp_exposable`
  means the MCP surface may register the operation. The conservative default
  listing (`mcp_tool_registrations_default_visible()`) is a strict subset that
  excludes high-risk and feature-gated operations. `mcp_exposable = true` does
  not imply the tool appears in conservative MCP listings.

- **High-risk operations require explicit capabilities and strict runtime
  approval.** Setting `risk > SafeActive` without declaring at least one
  non-baseline capability will fail the `high_risk_ops_declare_nonbaseline_capability`
  test. Even with the correct metadata, high-risk operations still require
  `EnforcementContext::evaluate()` approval and `ApprovedOperation` tokens on
  strict surfaces (MCP, REST, agent, gRPC). Metadata alone does not grant
  authorization.
