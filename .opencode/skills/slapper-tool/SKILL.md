# Slapper Tool Skill

Tool abstraction layer workflows and patterns for security tool integration.

## Key Types and Patterns

### SecurityTool Trait
`tool/traits.rs:117` has `SecurityTool` trait for tool abstraction.

### ToolRegistry
`tool/registry.rs:9` has `ToolRegistry` for managing tool instances. Feature-gated behind `tool-api` (enabled by `rest-api`, `grpc-api`, `nse`).

### Protocol Implementations
`tool/protocol/`:
- `mcp/` - MCP server (`handlers/server.rs`, `handlers/helpers.rs`)
- `openai/` - OpenAI-compatible chat completions
- `rest.rs` - REST API (scope validation implemented)
- `grpc.rs` - gRPC service

### Agent Routes (REST API)
`tool/protocol/agent_routes.rs` - Agent and task management:
- `validate_callback_url()` - SSRF protection for agent callback URLs
  - Rejects non-http/https schemes
  - Rejects URLs with embedded credentials
  - Rejects loopback, private, link-local, multicast, unspecified IPs
  - Validates via DNS resolution for hostnames

### Tool Agents (Scheduler)
`tool/agents/scheduler.rs` - Task scheduling:
- `TaskStatus` enum: `Pending`, `Leased`, `Completed`, `Failed`, `Cancelled`
- `next_task()` returns only `Pending` tasks where `scheduled_for <= now`
- `lease_task()` - marks task as `Leased` with agent and timeout
- `submit_result()` - transitions `Leased` task to `Completed` or `Failed`

`tool/agents/lifecycle.rs` - Agent lifecycle management:
- `HealthIssue::CallbackUnhealthy(String)` - tracks callback health separately
- Uses `saturating_sub` for clock skew safety
- Health checks probe callback URLs before acquiring lock

### Tool Implementations
`tool/implementations/` - Recon, scanner, fuzzer, waf, search, etc.

### Pipeline Orchestrator
`tool/orchestrator/mod.rs` handles parallel and sequential tool execution:

**Parallel Execution** (correct pattern):
```rust
use futures::future::join_all;

let handles: Vec<_> = stage.tools.iter().map(|tool| {
    async move { /* dispatch tool */ }
}).collect();

let results = join_all(handles).await;
```

**Sequential Execution**:
```rust
for tool in &stage.tools {
    let request = Self::build_request(tool, target);
    let result = self.dispatcher.dispatch(request).await;
    // process result
}
```

**Result Passing**: Previous stage results are passed via `request.params["results"] = previous_output`.

## Testing

### Running Tool Tests
```bash
cargo test --lib -p slapper tool::
```

### Writing Tests
Follow existing test patterns in `tool/` modules, testing trait implementations, registry, and protocol handlers.

## Common Tasks

### Adding a New Security Tool
1. Implement `SecurityTool` trait in `tool/implementations/`
2. Register tool in `ToolRegistry`
3. Add tests for new tool implementation

### Adding a New Protocol Handler
1. Create module in `tool/protocol/`
2. Implement protocol logic
3. Add SSRF protection for external URLs
4. Add tests for new handler

## Resources
- `crates/slapper/src/tool/AGENTS.override.md` - Detailed tool patterns
- `AGENTS.md` - General project guidelines
- `ARCHITECTURE.md` - Overall design
