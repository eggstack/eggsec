# Tool Module Override

Specialized guidance for the tool abstraction layer.

## SecurityTool Trait

`tool/traits.rs:117` has `SecurityTool` trait for tool abstraction.

## ToolRegistry

`tool/registry.rs:9` has `ToolRegistry` for managing tool instances.

Feature-gated behind `tool-api` (enabled by `rest-api`, `grpc-api`, `nse`).

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