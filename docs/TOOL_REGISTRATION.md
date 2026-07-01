# Tool Registration Inventory

Work Item 1 — Phase 7: Tool Registration Audit

## 1. Overview

Tools in Eggsec are registered and filtered through multiple independent sources. Each protocol surface (MCP, REST, gRPC, Agent) has its own listing and filtering behavior, and the exposure checks are enforced at different points in the request lifecycle. This document inventories every place tools are registered or filtered.

## 2. Registration Sources

| Source | Location | Type | Purpose |
|--------|----------|------|---------|
| `ToolRegistry` | `tool/registry.rs:54` | `FxHashMap<String, Arc<dyn SecurityTool>>` | Runtime tool storage |
| `create_default_registry()` | `tool/mod.rs:88` | Imperative builder | Populates `ToolRegistry` with 11 base + 3 feature-gated tools |
| `ALL_OPERATION_METADATA` | `config/policy.rs:1034` | Static slice (29 entries + 32 aliases) | Risk, capabilities, exposure flags per operation |
| `metadata_for_tool_id()` | `config/policy.rs:1626` | Lookup function | Resolves tool ID → `OperationMetadata` (alias-aware) |
| `all_domain_descriptors()` | `domain/mod.rs:274` | Static slice (3 domains) | Domain-level tool integration metadata |
| `ToolMetadataRegistry` | `tool/metadata.rs:82` | Per-tool risk/policy metadata | Supplementary risk metadata (separate from `OperationMetadata`) |
| `McpProfilePolicy` | `tool/protocol/mcp/policy.rs:64` | Per-profile filtering | MCP tool visibility by profile |

## 3. Strict Dispatch Path

All protocol surfaces share this enforcement chain:

1. **Parse request** — extract tool ID and parameters
2. **Resolve metadata** — `metadata_for_tool_id(tool_id)` → `OperationMetadata` (`config/policy.rs:1626`)
3. **Build descriptor** — `metadata.descriptor_for_target(target)` → `OperationDescriptor`
4. **Evaluate policy** — `EnforcementContext::evaluate(descriptor)` → outcome
5. **Require token** — `approve()` produces `ApprovedOperation` (private fields)
6. **Dispatch** — `EnforcedDispatcher::dispatch_checked(request, approved)` (`tool/dispatcher.rs:114`)

Raw `ToolDispatcher::dispatch()` (`tool/dispatcher.rs:36`) is `pub(crate)` and `#[doc(hidden)]`. Strict surfaces must never use it.

## 4. Protocol Listing Behavior

| Protocol | Listing Source | Filtering at Listing | Exposure Check at Execute |
|----------|---------------|----------------------|---------------------------|
| MCP (OpsAgent) | `registry.list()` | `McpProfilePolicy.filter_tools()` (`policy.rs:159`) → `mcp_tool_registrations("ops-agent")` (filter on `mcp_metadata_exposable`) | `EnforcementContext::evaluate()` + `ApprovedOperation` token |
| MCP (CodingAgent) | `registry.list()` | `McpProfilePolicy.filter_tools()` (`policy.rs:159`) → `mcp_tool_registrations("coding-agent")` (hardcoded narrow allowlist) | Same as OpsAgent |
| MCP (conservative default) | `mcp_tool_registrations_default_visible()` | Filter on `mcp_default_visible` (passive/safe-active, no feature gate) | Same as OpsAgent |
| REST | `registry.list()` | None at listing (full registration available; routing checks `rest_exposable`) | `rest_exposable` checked at execute via `EnforcementContext::evaluate()` |
| gRPC | `registry.list()` | None at listing (full registration available; routing checks `grpc_exposable`) | `grpc_exposable` checked at execute via `EnforcementContext::evaluate()` |
| Agent | `create_default_registry()` → `agent_tool_registrations()` | None at listing | `agent_exposable` checked at dispatch via `EnforcedDispatcher::dispatch_checked()` |

MCP listing **does** apply `ToolRegistration` filtering on top of `McpProfilePolicy`:
- OpsAgent uses `mcp_tool_registrations("ops-agent")` which returns all
  tools with `mcp_metadata_exposable = true`. This is the
  **profile-expanded** listing, not the conservative default.
- CodingAgent uses a hardcoded narrow allowlist (scan-ports, fingerprint,
  scan-endpoints, endpoints, waf-detect, search).
- REST and gRPC expose `rest_exposable` / `grpc_exposable` checks at
  execute time (via `EnforcementContext::evaluate()`), not at listing.

These are **listing-level** filters. Runtime authorization remains
`EnforcementContext::evaluate()` → `ApprovedOperation` → `EnforcedDispatcher::dispatch_checked()`.

## 5. Feature-Gated Tools

| Feature Gate | Tool ID | Required MCP Feature | Domain? |
|-------------|---------|---------------------|---------|
| `web-proxy-mcp` | `proxy` | `Some("web-proxy-mcp")` | No |
| `db-pentest-mcp` | `db-pentest` | `Some("db-pentest-mcp")` | Yes (`db-pentest`) |
| `c2-mcp` | `c2` | `Some("c2-mcp")` | No |

Feature-gated tools are only registered in `create_default_registry()` when their feature is enabled at compile time.

## 6. MCP Visibility Model A: Profile-Expanded vs Default-Conservative

MCP tool visibility is governed by **two distinct fields** on `ToolRegistration`:

- **`mcp_metadata_exposable`** (`OperationMetadata`-level): Whether the operation metadata declares the tool as MCP-exposable. This is the broad permission gate — the tool *may* be registered on MCP when the required feature is compiled, registered, scoped, and policy-authorized.
- **`mcp_default_visible`** (conservative default listing): Whether the tool appears in the default MCP tool listing without profile-specific expansion. This is a conservative subset: passive/safe-active operations with `mcp_metadata_exposable = true` and no feature gate.

The project uses **Model A** (profile-expanded metadata-exposable listing):

| Listing | Source | Visibility | Mechanism |
|---------|--------|------------|-----------|
| OpsAgent MCP listing | `mcp_tool_registrations("ops-agent")` | **Profile-expanded** — every `mcp_metadata_exposable` tool | `ToolSelector::All` (`policy.rs:100`) + `mcp_tool_registrations("ops-agent")` (filter on `mcp_metadata_exposable`) |
| CodingAgent MCP listing | `mcp_tool_registrations("coding-agent")` | Hardcoded narrow allowlist | `ToolSelector::Exact(vec![...])` (`policy.rs:124`) + hardcoded allowlist filter |
| Conservative default | `mcp_tool_registrations_default_visible()` | Conservative subset (passive/safe-active, no feature gate) | Filter on `mcp_default_visible` |
| `db-pentest` domain | Domain registration | Opt-in | `mcp_exposed_by_default: false` (`domain/mod.rs:504`), requires `db-pentest-mcp` feature |
| `mobile-static` domain | Domain registration | Opt-in | `mcp_exposed_by_default: false` (`domain/mod.rs:569`), not in default registry |
| `mobile-dynamic` domain | Domain registration | Opt-in | `mcp_exposed_by_default: false` (`domain/mod.rs:632`), not in default registry |

**Important**: OpsAgent is **not** the conservative default listing. It is an
expanded operator profile that lists every `mcp_metadata_exposable` tool,
including high-risk operations that are not `mcp_default_visible`. Strict
runtime policy (`EnforcementContext::evaluate()` + `ApprovedOperation`) is
still required to dispatch any listed tool.

The `mcp_tool_registrations_default_visible()` function returns the
conservative subset for callers that want only default-visible tools.

## 7. Raw Dispatch Exceptions

- `ToolDispatcher::dispatch()` is `pub(crate)` with `#[doc(hidden)]` (`tool/dispatcher.rs:34-36`)
- Agent test-only path (`new_for_test()`) sets `enforced_dispatcher = None` and uses raw dispatch exclusively
- `enforced_dispatch_regression.rs` tests scan for raw dispatch calls in strict surfaces
- If `enforced_dispatcher` is `Some` but `ApprovedOperation` is `None` at dispatch time, the agent returns a hard invariant error — no raw dispatch fallback

## 8. Tool Listing Audit Summary

| Surface | Filters through `ToolRegistration` | Filter mechanism |
|---------|-----------------------------------|------------------|
| MCP (OpsAgent) | Yes | `mcp_tool_registrations("ops-agent")` → `mcp_metadata_exposable` |
| MCP (CodingAgent) | Yes | `mcp_tool_registrations("coding-agent")` → hardcoded allowlist |
| MCP (default-visible) | Yes | `mcp_tool_registrations_default_visible()` → `mcp_default_visible` |
| REST | Indirect | `rest_exposable` checked at execute time only |
| gRPC | Indirect | `grpc_exposable` checked at execute time only |
| Agent | Yes | `agent_tool_registrations()` → `agent_exposable` |

**All protocol listing now filters through `ToolRegistration`** for MCP and
Agent surfaces. REST and gRPC do not filter at listing time, but their
respective exposure flags are enforced at execute time via
`EnforcementContext::evaluate()` (which queries the same metadata).

## 9. Safety Invariants

1. **No bypass**: Strict surfaces (REST, MCP, Agent, gRPC) must obtain `ApprovedOperation` before dispatch
2. **No raw dispatch**: `ToolDispatcher::dispatch()` is `pub(crate)` + `#[doc(hidden)]`; regression tests enforce this
3. **Metadata coverage**: Every registered tool must have a matching `OperationMetadata` entry (validated in `config/policy.rs` tests at line ~1841)
4. **Domain descriptors always present**: Domain descriptors exist regardless of feature state; check `required_feature` before use
5. **Alias resolution**: `metadata_for_tool_id()` resolves aliases before falling back to exact match (`config/policy.rs:1630`)

## 10. Phase 7 Changes

**Implemented:**

- `ToolRegistration` type and builder functions added in `tool::registration` (`all_tool_registrations()`, `mcp_tool_registrations()`, `mcp_tool_registrations_default_visible()`, `rest_tool_registrations()`, `grpc_tool_registrations()`, `agent_tool_registrations()`)
- MCP, REST, gRPC, and Agent listing now filter through registration metadata instead of raw `registry.list()` calls
- 10 new tool registration consistency tests (`tests/tool_registration.rs`) validate registration coverage, exposure flag alignment, source correctness, and protocol filtering
- Enforcement paths verified unchanged — `EnforcementContext::evaluate()` remains the sole authorization gate
- Registration-to-execution bridge demonstrated for the `search` tool: registration metadata resolves to `OperationDescriptor` via `metadata_for_tool_id()` → `descriptor_for_target()` → `EnforcementContext::approve()` → `EnforcedDispatcher::dispatch_checked()`

**Phase 6-10 polish pass (Model A made explicit):**

- Renamed `interactive_only` on `CommandRegistration` to `cli_interactive_only` to remove ambiguity with TUI manual actions.
- Tightened `side_effecting_entries_have_descriptor_builder()` into three dispatch-mode-scoped tests:
  - `registry_backed_side_effecting_commands_build_descriptors` (registry-backed only)
  - `legacy_wrapped_operation_metadata_is_optional_but_valid_when_present` (legacy metadata is documentation, not dispatch proof)
  - `helper_and_server_commands_do_not_require_descriptors` (helper/server have no descriptor requirement)
- Added `ops_agent_is_expanded_metadata_exposable_not_conservative_default` to encode Model A explicitly.

**Remaining (deferred):**

- [ ] Audit `ToolMetadataRegistry` vs `ALL_OPERATION_METADATA` overlap
- [ ] Consider declarative registration (tool declares its metadata at registration time)

### Work Item 6 — Comment Cleanup

Shortened durable-fact comments in protocol surfaces that duplicated metadata now available through `ToolRegistration` and `OperationMetadata`:

- **`tool/mod.rs`**: Added doc comment on `create_default_registry()` pointing to this file as the canonical source for registration model and protocol exposure rules.
- **`tool/protocol/mcp/handlers/server.rs`**: Replaced verbose "Registration-based guard" comments in `handle_tools_list` and `handle_tools_list_by_category` with one-line references describing Model A profile-expanded visibility and pointing to `ToolRegistration mcp_metadata_exposable` / `mcp_default_visible` and this document.
- **`tool/protocol/rest.rs`**: Replaced "Registration-based guard" comment in `list_tools` with one-line reference to `ToolRegistration rest_exposable`.
- **`tool/protocol/grpc.rs`**: Replaced "Registration-based guard" comment in `list_tools` with one-line reference to `ToolRegistration grpc_exposable`.

**Not removed** (safety-critical or non-duplicative):
- Enforcement approval comments (shared enforcement, strict profiles, audit events)
- Feature-gate `#[cfg]` attributes on `create_default_registry()` registrations
- MCP profile filtering logic comments

### Glossary

- **metadata-exposable** — `mcp_metadata_exposable: true` flag set from `OperationMetadata.mcp_exposable`. Means the operation *may* be MCP-registered when the required feature is compiled, scoped, and policy-authorized.
- **default-visible** — `mcp_default_visible: true` flag. Conservative subset visible to `mcp_tool_registrations_default_visible()`.
- **profile-expanded** — Profile-specific listing (`ops-agent`, `coding-agent`) that goes beyond the conservative default. OpsAgent is profile-expanded.
- **runtime-approved** — Has passed `EnforcementContext::evaluate()` and produced an `ApprovedOperation` token. Only at this point can `EnforcedDispatcher::dispatch_checked()` be invoked.
