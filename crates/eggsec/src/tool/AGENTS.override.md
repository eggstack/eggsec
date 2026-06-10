# Tool Module Override

 Specialized guidance for the tool abstraction layer.

## SecurityTool Trait

 `tool/traits.rs:117` has `SecurityTool` trait for tool abstraction.

## ToolRegistry

 `tool/registry.rs:9` has `ToolRegistry` for managing tool instances.

 Feature-gated behind `tool-api` (enabled by `rest-api`, `grpc-api`, `nse`).

## Known Issues (2026-05-28)

### HashSet Performance Issue
**File**: `planner.rs:4`

Uses `std::collections::HashSet` instead of `FxHashSet`. This is in the hot path for execution planning.

**Fix**: Change to `rustc_hash::FxHashSet`:
```rust
use rustc_hash::FxHashSet;
// Line 80, 204, 247, 309, 351, 386, 429, 492: Change HashSet → FxHashSet
```

## Protocol Implementations

`tool/protocol/`:
- `mcp/` - MCP server (`handlers/server.rs`, `handlers/helpers.rs`)
- `openai/` - OpenAI-compatible chat completions
- `rest.rs` - REST API (scope validation implemented)
- `grpc.rs` - gRPC service

## Agent Routes (REST API)

`tool/protocol/agent_routes.rs` - Agent and task management:
- `validate_callback_url()` - SSRF protection for agent callback URLs
  - Rejects non-http/https schemes
  - Rejects URLs with embedded credentials
  - Rejects loopback, private, link-local, multicast, unspecified IPs
  - Validates via DNS resolution for hostnames

## Tool Agents (Scheduler)

`tool/agents/scheduler.rs` - Task scheduling:
- `TaskStatus` enum: `Pending`, `Leased`, `Completed`, `Failed`, `Cancelled`
- `next_task()` returns only `Pending` tasks where `scheduled_for <= now`
- `lease_task()` - marks task as `Leased` with agent and timeout
- `submit_result()` - transitions `Leased` task to `Completed` or `Failed`

`tool/agents/lifecycle.rs` - Agent lifecycle management:
- `HealthIssue::CallbackUnhealthy(String)` - tracks callback health separately
- Uses `saturating_sub` for clock skew safety
- Health checks probe callback URLs before acquiring lock

## Tool Implementations

`tool/implementations/` - Recon, scanner, fuzzer, waf, search, etc.

## Pipeline Orchestrator

`tool/orchestrator/mod.rs` handles parallel and sequential tool execution:
- **Parallel execution**: Uses `futures::future::join_all()` for concurrent tool execution
- **Result passing**: Previous stage results are passed via `request.params["results"]`
- **Error handling**: Returns `StageToolResult` with success/failure per tool

```rust
// Parallel execution pattern (correct)
let handles: Vec<_> = stage.tools.iter().map(|tool| { ... }).collect();
let results = join_all(handles).await;

// Sequential execution pattern
for tool in &stage.tools {
    let result = self.dispatcher.dispatch(request).await;
    tool_results.push(self.process_tool_result(tool, result, duration));
}
```

## MCP Profile System

- `McpProfile` enum (`OpsAgent`, `CodingAgent`) in `tool/protocol/mcp/profile.rs`
- `McpProfilePolicy` struct in `tool/protocol/mcp/policy.rs` — 18 fields controlling tool visibility and call restrictions
- Policy enforcement must happen at **both** discovery and call time — discovery filtering alone is insufficient
- `TargetPolicy` enum controls which targets each profile can access
- `extract_hostname()` counts colons to distinguish bare IPv6 (>=2, returned as-is) from host:port (1 colon, port stripped if valid u16)
- `classify_tool_risk()` maps tool IDs to `OperationRisk` for MCP policy decisions
- `policy_decision_for_mcp_call_with_enforcement` (via `EnforcementContext::evaluate`) builds a full `PolicyDecision` (capabilities populated, provenance enforced); legacy `policy_decision_for_mcp_call` deprecated for denial paths (2026-06-10). MCP server preferred constructor: `McpServer::with_enforcement`.