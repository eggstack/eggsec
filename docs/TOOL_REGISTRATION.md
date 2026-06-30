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
| MCP | `registry.list()` | `McpProfilePolicy.filter_tools()` (`policy.rs:159`) | No `mcp_exposable` check at listing |
| REST | `registry.list()` | None (all listed) | `rest_exposable` checked at execute time only |
| gRPC | `registry.list()` | None (all listed) | `grpc_exposable` checked at execute time only |
| Agent | `create_default_registry()` | None at listing | `agent_exposable` checked at dispatch |

Key gap: MCP listing does not check `mcp_exposable` from `OperationMetadata`. REST and gRPC listing do not filter by their respective exposure flags.

## 5. Feature-Gated Tools

| Feature Gate | Tool ID | Required MCP Feature | Domain? |
|-------------|---------|---------------------|---------|
| `web-proxy-mcp` | `proxy` | `Some("web-proxy-mcp")` | No |
| `db-pentest-mcp` | `db-pentest` | `Some("db-pentest-mcp")` | Yes (`db-pentest`) |
| `c2-mcp` | `c2` | `Some("c2-mcp")` | No |

Feature-gated tools are only registered in `create_default_registry()` when their feature is enabled at compile time.

## 6. Default MCP Exposure vs Opt-in

| Profile / Domain | Visibility | Mechanism |
|-----------------|------------|-----------|
| OpsAgent | All registered tools | `ToolSelector::All` (`policy.rs:100`) |
| CodingAgent | 6 hardcoded tools | `ToolSelector::Exact(vec![...])` (`policy.rs:124`) |
| `db-pentest` | Opt-in | `mcp_exposed_by_default: false` (`domain/mod.rs:461`), requires `db-pentest-mcp` feature |
| `mobile-static` | Opt-in | `mcp_exposed_by_default: false` (`domain/mod.rs:525`), not in default registry |
| `mobile-dynamic` | Opt-in | `mcp_exposed_by_default: false` (`domain/mod.rs:587`), not in default registry |

The OpsAgent profile shows all registered tools regardless of `mcp_exposable` in `OperationMetadata`. There is no intersection between the profile selector and the metadata exposure flag.

## 7. Raw Dispatch Exceptions

- `ToolDispatcher::dispatch()` is `pub(crate)` with `#[doc(hidden)]` (`tool/dispatcher.rs:34-36`)
- Agent test-only path (`new_for_test()`) sets `enforced_dispatcher = None` and uses raw dispatch exclusively
- `enforced_dispatch_regression.rs` tests scan for raw dispatch calls in strict surfaces
- If `enforced_dispatcher` is `Some` but `ApprovedOperation` is `None` at dispatch time, the agent returns a hard invariant error — no raw dispatch fallback

## 8. Known Legacy/Manual Registrations

- `create_default_registry()` (`tool/mod.rs:88`) is purely imperative — no declarative metadata ties tool registration to `OperationMetadata`
- `ToolMetadataRegistry` (`tool/metadata.rs:82`) is a separate per-tool risk/policy registry, supplementary to `ALL_OPERATION_METADATA`
- REST listing (`registry.list()`) does not filter by `rest_exposable` at listing time
- gRPC listing (`registry.list()`) does not filter by `grpc_exposable` at listing time
- MCP listing filters by profile policy but not by `mcp_exposable` from `OperationMetadata`

## 9. Safety Invariants

1. **No bypass**: Strict surfaces (REST, MCP, Agent, gRPC) must obtain `ApprovedOperation` before dispatch
2. **No raw dispatch**: `ToolDispatcher::dispatch()` is `pub(crate)` + `#[doc(hidden)]`; regression tests enforce this
3. **Metadata coverage**: Every registered tool must have a matching `OperationMetadata` entry (validated in `config/policy.rs` tests at line ~1841)
4. **Domain descriptors always present**: Domain descriptors exist regardless of feature state; check `required_feature` before use
5. **Alias resolution**: `metadata_for_tool_id()` resolves aliases before falling back to exact match (`config/policy.rs:1630`)

## 10. Phase 7 Changes

**Implemented:**

- `ToolRegistration` type and builder functions added in `tool::registration` (`all_tool_registrations()`, `mcp_tool_registrations()`, `rest_tool_registrations()`, `grpc_tool_registrations()`, `agent_tool_registrations()`)
- MCP, REST, gRPC, and Agent listing now filter through registration metadata instead of raw `registry.list()` calls
- 10 new tool registration consistency tests (`tests/tool_registration.rs`) validate registration coverage, exposure flag alignment, source correctness, and protocol filtering
- Enforcement paths verified unchanged — `EnforcementContext::evaluate()` remains the sole authorization gate
- Registration-to-execution bridge demonstrated for the `search` tool: registration metadata resolves to `OperationDescriptor` via `metadata_for_tool_id()` → `descriptor_for_target()` → `EnforcementContext::approve()` → `EnforcedDispatcher::dispatch_checked()`

**Remaining (deferred):**

- [ ] Audit `ToolMetadataRegistry` vs `ALL_OPERATION_METADATA` overlap
- [ ] Consider declarative registration (tool declares its metadata at registration time)

### Work Item 6 — Comment Cleanup

Shortened durable-fact comments in protocol surfaces that duplicated metadata now available through `ToolRegistration` and `OperationMetadata`:

- **`tool/mod.rs`**: Added doc comment on `create_default_registry()` pointing to this file as the canonical source for registration model and protocol exposure rules.
- **`tool/protocol/mcp/handlers/server.rs`**: Replaced verbose "Registration-based guard" comments in `handle_tools_list` and `handle_tools_list_by_category` with one-line references to `ToolRegistration mcp_exposed_by_default` and this document.
- **`tool/protocol/rest.rs`**: Replaced "Registration-based guard" comment in `list_tools` with one-line reference to `ToolRegistration rest_exposable`.
- **`tool/protocol/grpc.rs`**: Replaced "Registration-based guard" comment in `list_tools` with one-line reference to `ToolRegistration grpc_exposable`.

**Not removed** (safety-critical or non-duplicative):
- Enforcement approval comments (shared enforcement, strict profiles, audit events)
- Feature-gate `#[cfg]` attributes on `create_default_registry()` registrations
- MCP profile filtering logic comments
