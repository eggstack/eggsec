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
`tool/orchestrator/mod.rs` handles parallel and sequential tool execution with `FxHashMap` for stage results.

### FxHashMap Usage (Performance)

For performance in hot paths, these modules use `rustc_hash::FxHashMap`/`FxHashSet`:

| Module | Location | Purpose |
|--------|----------|---------|
| `orchestrator/mod.rs` | Lines 21, 50, 84, 89, 302 | Stage results, enabled stages |
| `tool/session.rs` | Lines 288, 316, 461, 465, 1076 | Session cookies, variables |
| `tool/state.rs` | Lines 124, 136 | Severity summary, sessions |
| `recon/mod.rs` | Lines 221, 253 | Technology metadata, takeover metadata |

## Recent Fixes (2026-06-01)

### Silent Error Suppression Fixed

| File | Line | Issue |
|------|------|-------|
| `lifecycle.rs` | 337 | Silent `update_status` |
| `lifecycle.rs` | 381,416,429,434,447 | Silent `event_tx.send()` |
| `mcp/routes.rs` | 216-252 | Silent write/flush errors |

### FxHashMap Replaced

- `orchestrator/mod.rs`: HashMap/HashSet → FxHashMap/FxHashSet
- `tool/session.rs`: HashMap → FxHashMap
- `tool/state.rs`: HashMap → FxHashMap
- `recon/mod.rs`: std HashMap → FxHashMap

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
