# Tool Exposure Guide

This guide explains how tools are exposed through MCP, REST, gRPC, and agent
surfaces in Eggsec. It covers the registration model, listing filters, and the
mandatory enforcement chain that must precede any tool execution.

**Required invariant**: Listing is not authorization. Every tool listed in any
surface still requires `EnforcementContext::evaluate()` and an
`ApprovedOperation` token before dispatch via `EnforcedDispatcher::dispatch_checked()`.

## 1. What ToolRegistration Owns

`ToolRegistration` (defined in `crates/eggsec/src/tool/registration.rs`) is the
canonical metadata record for a tool's cross-surface exposure. It is the single
source of truth for whether a tool appears in MCP, REST, gRPC, or agent listings.

Fields:

| Field | Type | Meaning |
|-------|------|---------|
| `tool_id` | `&'static str` | Canonical tool identifier used in protocol requests |
| `operation_id` | `&'static str` | Corresponding `OperationMetadata` ID |
| `display_name` | `&'static str` | Human-readable name |
| `source` | `ToolRegistrationSource` | Origin: `Base`, `FeatureGated(&str)`, or `Domain(&str)` |
| `feature` | `Option<&'static str>` | Cargo feature gate required to compile this tool |
| `required_mcp_feature` | `Option<&'static str>` | Additional feature gate for MCP exposure specifically |
| `mcp_metadata_exposable` | `bool` | Operation metadata declares this tool as MCP-exposable |
| `mcp_default_visible` | `bool` | Appears in conservative default MCP listing |
| `rest_exposable` | `bool` | Visible via REST API |
| `grpc_exposable` | `bool` | Visible via gRPC API |
| `agent_exposable` | `bool` | Visible to the security agent |
| `category` | `ToolCategory` | Classification for protocol categorization |

`ToolRegistration` does not contain authorization logic, enforcement context,
or execution paths. It is pure metadata.

## 2. Runtime Tool Registry vs. Canonical Metadata Registration

Eggsec has two distinct registries with different purposes:

**Runtime Tool Registry** (`tool/registry.rs`):
- Type: `FxHashMap<String, Arc<dyn SecurityTool>>`
- Populated by `create_default_registry()` in `tool/mod.rs`
- Holds executable tool implementations
- 11 base tools + feature-gated tools (web-proxy-mcp, db-pentest-mcp, c2-mcp)
- Used at execution time: `ToolDispatcher` looks up and invokes tools here
- Not the source of truth for listing decisions

**Canonical Metadata Registration** (`tool/registration.rs`):
- Derived from `ALL_OPERATION_METADATA` (32 entries + 33 aliases in `config/policy.rs`) and `all_domain_descriptors()`
- `all_tool_registrations()` builds the canonical list
- Source of truth for MCP, REST, gRPC, and agent listing filters
- Cross-references `OperationMetadata` exposure flags and `DomainDescriptor` tool integrations
- Does not hold executable implementations

The bridge between them: `metadata_for_tool_id()` resolves a tool ID to its
`OperationMetadata`, which carries the exposure flags and risk profile that
`ToolRegistration` reflects. At execution time, `EnforcementContext::evaluate()`
uses the same metadata to build an `OperationDescriptor` for authorization.

**Never** conflate "tool is in the runtime registry" with "tool is listed for a
protocol surface." A tool can be registered but not exposed on a given surface.
Exposure decisions are metadata-driven, not registry-driven.

## 3. Exposure Flag Semantics

### mcp_metadata_exposable

Derived from `OperationMetadata.mcp_exposable`. This is the broad MCP permission
gate. When `true`, the tool *may* be registered on MCP when the required feature
is compiled, scoped, and policy-authorized. This flag is necessary but not
sufficient for MCP listing -- the profile-specific filter and feature gate also
apply.

Domain tools override this: the `ToolIntegration.mcp_exposed_by_default` field
in a `DomainDescriptor` determines the `mcp_default_visible` value for
domain-sourced registrations.

### mcp_default_visible

The conservative default listing. A tool has `mcp_default_visible = true` when:

1. `mcp_metadata_exposable == true`
2. Risk is `Passive` or `SafeActive` (not high-risk)
3. No feature gate requirement (unconditional registration)

This is the subset returned by `mcp_tool_registrations_default_visible()`. It
excludes feature-gated tools, high-risk operations, and tools that opt out via
`DomainDescriptor` tool integration (`mcp_exposed_by_default: false`).

### required_mcp_feature

An additional feature gate for MCP exposure specifically. When present, the tool
requires this Cargo feature to appear in MCP listings even if the base tool is
compiled. Example: `db-pentest` requires `db-pentest-mcp` for MCP exposure.

Tools with `required_mcp_feature` always have `mcp_default_visible = false`.

### rest_exposable

Derived from `OperationMetadata.rest_exposable`. When `true`, the tool is
visible in REST API listings. REST does not filter at listing time -- the flag
is enforced at execute time via `EnforcementContext::evaluate()`.

### grpc_exposable

Derived from `OperationMetadata.grpc_exposable`. Same semantics as
`rest_exposable` but for the gRPC surface.

### agent_exposable

Derived from `OperationMetadata.agent_exposable`. When `true`, the tool is
visible in agent tool listings via `agent_tool_registrations()`.

## 4. MCP Model A: Profile-Expanded Visibility

MCP tool visibility uses **Model A** (profile-expanded metadata-exposable
listing). This is implemented in `mcp_tool_registrations(profile)`:

| Profile | Behavior |
|---------|----------|
| `"ops-agent"` | Returns every tool with `mcp_metadata_exposable == true` (profile-expanded) |
| `"coding-agent"` | Returns a hardcoded narrow allowlist (scan-ports, fingerprint, scan-endpoints, endpoints, waf-detect, search) |
| Any other | Returns empty list |

**Critical distinction**: OpsAgent is **not** the conservative default listing.
It is an expanded operator profile that includes high-risk operations (e.g.,
`IntrusiveFuzz` risk tier tools) that are `mcp_metadata_exposable` but not
`mcp_default_visible`. The conservative default is
`mcp_tool_registrations_default_visible()`.

The test `ops_agent_is_expanded_metadata_exposable_not_conservative_default`
validates this invariant: the OpsAgent listing is strictly larger than the
conservative default listing (`ops_ids.len() > default_ids.len()`), and the
conservative default is a proper subset.

**Warning**: Do not conflate `mcp_metadata_exposable` (broad permission gate)
with `mcp_default_visible` (conservative listing). The former is a per-operation
declaration; the latter is a curated safe subset. OpsAgent uses the former.

## 5. CodingAgent Allowlist Behavior

The coding-agent MCP profile uses a hardcoded allowlist rather than a
metadata-driven filter:

```rust
let coding_agent_ids = [
    "scan", "scan-ports", "fingerprint", "scan-endpoints",
    "endpoints", "waf-detect", "search",
];
```

This is intentionally narrow: coding agents receive only passive and safe-active
scanning tools. High-risk, feature-gated, and domain-specific tools are excluded
regardless of their metadata flags.

The coding-agent allowlist is a subset of the ops-agent listing (validated by
`coding_agent_registrations_are_subset_of_mcp`).

## 6. REST and gRPC: Listing vs. Execution Checks

REST and gRPC do **not** filter tool listings through `ToolRegistration` at
listing time. Instead, the exposure flags are enforced at execute time:

**REST execution path**:
1. `handle_serve()` constructs `EnforcementContext::for_surface(ExecutionSurface::RestApi, ...)`
2. Metadata `rest_exposable` is checked before policy evaluation
3. `enforcement.evaluate()` evaluates the operation descriptor
4. Only `EnforcementOutcome::Allow` permits dispatch
5. `Warn`, `RequireConfirmation`, and `Deny` all return HTTP 403 with `POLICY_DENIED`
6. Dispatch goes through `EnforcedDispatcher::dispatch_checked()`

**gRPC execution path**:
1. `GrpcService` carries `EnforcementContext::for_surface(ExecutionSurface::GrpcApi, ...)`
2. Metadata `grpc_exposable` is checked before policy evaluation
3. `enforcement.approve()` produces `ApprovedOperation`
4. Only `EnforcementOutcome::Allow` produces a token
5. `Warn`, `RequireConfirmation`, and `Deny` all return `Status::permission_denied`
6. Dispatch goes through `EnforcedDispatcher::dispatch_checked()`

Both surfaces are strict and noninteractive. Warning-class ambiguity must not
dispatch. Manual overrides are never honored.

## 7. Agent Dispatch Checks

Agent execution uses `AgentStrict` profile:

1. `Agent::new()` validates the enforcement profile is `AgentStrict`
2. Tool listing is filtered through `agent_tool_registrations()` (checks
   `agent_exposable`)
3. Execution: handler rebuilds `AgentStrict` defensively at runtime
4. `EnforcementContext::evaluate()` is called before dispatch
5. `EnforcedDispatcher::dispatch_checked()` requires `ApprovedOperation`
6. If `enforced_dispatcher` is `Some` but `ApprovedOperation` is `None` at
   dispatch time, agent returns a hard invariant error -- no raw dispatch
   fallback

The agent test-only path (`new_for_test()`) sets `enforced_dispatcher = None`
and uses raw dispatch exclusively. This path is guarded by the
`enforced_dispatch_regression` tests.

## 8. How to Add a Tool Integration Through a Domain Descriptor

To expose a new tool through a domain's MCP/REST/gRPC integration:

**Step 1**: Define `ToolIntegration` in the domain module:

```rust
const MY_TOOL: ToolIntegration = ToolIntegration {
    tool_id: "my-tool",
    operation_id: "my-operation",
    mcp_exposed_by_default: false,  // opt-in for MCP
    required_mcp_feature: Some("my-domain-mcp"),  // feature gate for MCP
};
```

**Step 2**: Add the tool to the domain descriptor's `tools` slice:

```rust
const MY_DOMAIN: DomainDescriptor = DomainDescriptor {
    id: "my-domain",
    // ...
    tools: &[MY_TOOL],
    // ...
};
```

**Step 3**: Ensure the operation has `OperationMetadata` in
`ALL_OPERATION_METADATA` with appropriate exposure flags (`rest_exposable`,
`agent_exposable`, `grpc_exposable`).

**Step 4**: Feature-gate the domain descriptor registration in
`all_domain_descriptors()` and `create_default_registry()`.

**Step 5**: Register the runtime tool implementation behind the feature gate in
`create_default_registry()`.

**Step 6**: Add tests (see section 10).

The `ToolRegistration` builder in `all_tool_registrations()` automatically picks
up domain tool integrations and sets `source = ToolRegistrationSource::Domain(domain_id)`.

## 9. How to Expose a Base Tool

Base tools (recon, scan-ports, fuzz, etc.) are tools not owned by a domain. To
expose a new base tool:

**Step 1**: Add `OperationMetadata` entry in `ALL_OPERATION_METADATA` in
`config/policy.rs`:

```rust
OperationMetadata {
    id: "my-tool",
    display_name: "My Tool",
    mode: OperationMode::StandardAssessment,
    risk: OperationRisk::SafeActive,
    intended_uses: &[IntendedUse::WebAssessment],
    required_features: &[],  // or &["my-feature"] for feature-gated
    required_policy_flags: &[],
    required_capabilities: &[],
    target_policy: TargetPolicyKind::ExplicitScopeRequired,
    manual_exposable: true,
    tui_exposable: true,
    mcp_exposable: true,   // MCP-exposable
    rest_exposable: true,  // REST-exposable
    agent_exposable: true, // agent-exposable
    grpc_exposable: true,  // gRPC-exposable
},
```

**Step 2**: Add a `ToolRegistration` in `all_tool_registrations()` if the tool
is not automatically picked up (it will be if the `OperationMetadata` exists).
If the tool is feature-gated, the builder assigns `source =
ToolRegistrationSource::FeatureGated(feature)`.

**Step 3**: Register the runtime tool in `create_default_registry()`, feature-gated
with `#[cfg(feature = "my-feature")]` if applicable.

**Step 4**: Add alias mapping in `metadata_for_tool_id()` if the tool needs to
be reachable via alternate names.

**Step 5**: Add tests (see section 10).

## 10. Required Tests

When adding or modifying tool exposure, the following test suites must pass:

### tool_registration tests

Location: `crates/eggsec/tests/tool_registration.rs`

Run:
```bash
cargo test --test tool_registration -p eggsec
```

Key invariants validated:

| Test | Invariant |
|------|-----------|
| `every_tool_registration_resolves_to_operation_metadata` | Every registration has a matching `OperationMetadata` |
| `default_mcp_exposed_tools_have_metadata_flag` | `mcp_metadata_exposable` matches `OperationMetadata.mcp_exposable` |
| `opt_in_mcp_tools_not_default_exposed` | Tools with `required_mcp_feature` are not `mcp_default_visible` |
| `hazardous_domains_never_default_mcp_exposed` | HazardousLab domain tools are not default-visible |
| `high_risk_agent_exposable_ops_declare_capabilities` | High-risk agent-exposable ops declare required capabilities |
| `feature_gated_registrations_declare_nonempty_features` | `FeatureGated` source matches `feature` field |
| `all_protocol_registrations_are_subsets_of_all` | REST/gRPC/Agent registrations are subsets of `all_tool_registrations()` |
| `coding_agent_registrations_are_subset_of_mcp` | CodingAgent subset of OpsAgent |
| `no_duplicate_tool_ids_in_registrations` | Unique `tool_id` across all registrations |
| `mcp_default_visible_implies_metadata_exposable` | `mcp_default_visible` implies `mcp_metadata_exposable` |
| `high_risk_operations_not_default_mcp_visible` | High-risk ops are not `mcp_default_visible` |
| `ops_agent_is_expanded_metadata_exposable_not_conservative_default` | OpsAgent is strictly broader than conservative default |

### enforced_dispatch_regression tests

Location: `crates/eggsec/tests/enforced_dispatch_regression.rs`

Run:
```bash
cargo test --test enforced_dispatch_regression -p eggsec
```

Key invariants validated:

| Test | Invariant |
|------|-----------|
| `strict_surfaces_do_not_call_raw_dispatch_directly` | REST, gRPC, MCP, agent source files cannot call raw `.dispatch()` -- must use `dispatch_checked()` |
| `ci_handler_has_no_dispatch_path` | CI handler contains no `ToolDispatcher`, `EnforcedDispatcher`, or dispatch APIs (Architecture Invariant #19) |

### Unit tests in registration.rs

Located in `tool/registration.rs` module tests:

| Test | Invariant |
|------|-----------|
| `all_registrations_have_non_empty_ids` | No empty tool_id, operation_id, or display_name |
| `base_tools_are_always_present` | 11 base tools always in registrations |
| `base_tools_have_no_feature_gate` | Base tools have `ToolRegistrationSource::Base` |
| `mcp_ops_agent_returns_all_metadata_exposable` | OpsAgent returns only `mcp_metadata_exposable` tools |
| `mcp_coding_agent_returns_coding_tools` | CodingAgent includes expected tool IDs |
| `mcp_unknown_profile_returns_empty` | Unknown profiles return empty list |
| `rest_registrations_are_all_rest_exposable` | REST listing only returns `rest_exposable` tools |
| `grpc_registrations_are_all_grpc_exposable` | gRPC listing only returns `grpc_exposable` tools |
| `agent_registrations_are_all_agent_exposable` | Agent listing only returns `agent_exposable` tools |
| `every_registration_has_operation_metadata` | Cross-reference with `metadata_for_tool_id()` |
| `registration_source_matches_feature_state` | Source variant matches feature field |

### Enforcement matrix tests

Run:
```bash
cargo test --test enforcement_matrix -p eggsec
cargo test -p eggsec --features rest-api --test enforcement_matrix
```

Covers 134+ tests validating enforcement outcomes across all surfaces, risk
tiers, capabilities, and override handling.

### Additional validation

```bash
cargo clippy --lib -p eggsec
cargo test -p eggsec --test feature_matrix
bash scripts/check-architecture-guards.sh
```

## Glossary

- **metadata-exposable**: `mcp_metadata_exposable: true`. The operation may be
  MCP-registered when the required feature is compiled, scoped, and
  policy-authorized.
- **default-visible**: `mcp_default_visible: true`. Conservative subset visible
  to `mcp_tool_registrations_default_visible()`.
- **profile-expanded**: Profile-specific listing (ops-agent, coding-agent) that
  goes beyond the conservative default. OpsAgent is profile-expanded.
- **runtime-approved**: Has passed `EnforcementContext::evaluate()` and produced
  an `ApprovedOperation` token. Only at this point can
  `EnforcedDispatcher::dispatch_checked()` be invoked.
