# Eggsec Tool Skill

Tool abstraction layer workflows and patterns for security tool integration.

## Key Types and Patterns

### SecurityTool Trait
`tool/traits.rs:159` has `SecurityTool` trait for tool abstraction.

### ToolRegistry
`tool/registry.rs:23` has `ToolRegistry` for managing tool instances. Feature-gated behind `tool-api` (enabled by `rest-api`, `grpc-api`, `nse`).

### Protocol Implementations
`tool/protocol/`:
- `mcp/` - MCP server (`handlers/server.rs`, `handlers/helpers.rs`)
- `mcp/policy.rs` - MCP profile policy enforcement, `extract_hostname()` IPv6-aware parsing, `classify_tool_risk()`, `policy_decision_for_mcp_call()`
- `mcp/coding_agent_output.rs` - Typed `CodingAgentFindingReport` struct for coding-agent output
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
| `state.rs` | 216 | Silent `fs::remove_file()` - now uses tracing::debug! |
| `mcp/handlers/server.rs` | 758 | Silent `manager.update_session()` - now uses tracing::debug! |

### FxHashMap Replaced

- `orchestrator/mod.rs`: HashMap/HashSet → FxHashMap/FxHashSet
- `tool/session.rs`: HashMap → FxHashMap (line 519 - response headers)
- `tool/state.rs`: HashMap → FxHashMap
- `recon/mod.rs`: std HashMap → FxHashMap
- `tool/agents/lifecycle.rs`: HashMap → FxHashMap (health_status)
- `tool/agents/communication.rs`: HashMap → FxHashMap (subscriptions)

### SystemTime unw() Panic Prevention (2026-06-01)

Locations in `tool/agents/lifecycle.rs` replaced `.unwrap()` with `.unwrap_or_else(|_| Duration::from_secs(0))` to prevent panic if system clock goes backwards:

- Line 179: `check_stale_agents()` now calculation
- Line 357: `update_health()` last_health_check
- Line 390: `record_task_failure()` HealthCheckFailed event
- Line 428: `initiate_graceful_shutdown()` event
- Line 451: `force_shutdown()` event
- (also at lines 497, 510, 555, 597, 620, 644, 698, 822 - same pattern)

### Spoofed Packet Send Error Handling (2026-06-01)

Added proper error handling for spoofed packet sends in `scanner/ports/spoofed.rs`:
- Line 281: Fragmented UDP packets send - only increment counter on success
- Line 348: Decoy TCP packets send - now logs warning on failure
- Line 384: Staggered decoy packets send - now logs warning on failure
- Line 454-458: Changed `Mutex<u64>` to `AtomicU64` for scanned_count

### Timeout Wrappers Added (2026-06-05)

Added timeout wrappers to prevent indefinite hangs:

| File | Line | Operation | Timeout |
|------|------|-----------|---------|
| `session.rs` | 511 | HTTP request | 30s |
| `scanner.rs` | 136, 157, 186 | `run_cli_with_callback` (PortScan, Fingerprint, Endpoints) | 60s |
| `fuzzer.rs` | 170 | `run_cli_with_callback` | 60s |
| `recon.rs` | 141 | `run_cli_with_callback` | 60s |
| `pipeline.rs` | 98 | `run_cli_with_callback` | 60s |
| `loadtest.rs` | 78 | `run_cli` | 60s |
| `routes.rs` | 182, 241 | `handle_request` (batch, stdio) | 30s |

### IPv6 Hostname Parsing Fix (2026-06-10)

`extract_hostname()` in `mcp/policy.rs` now correctly handles bare IPv6 addresses. The fix counts colons: >=2 colons means bare IPv6 (returned as-is), 1 colon means host:port (port stripped if valid u16). Previously, `::1` and `2001:db8::1` were incorrectly truncated.

### FxHashMap Replacements (2026-06-05)

Replaced `HashMap` with `FxHashMap` in performance-critical paths:

| File | Lines | Fields |
|------|-------|--------|
| `finding.rs` | 2, 19, 39, 227, 284, 340, 400, 468, 524 | metadata, local vars |
| `aggregator.rs` | 2, 52, 53, 61, 62, 91, 92, 103, 104 | results, in_progress, stage_results, tool_results |

## Testing

### Running Tool Tests
```bash
cargo test --lib -p eggsec tool::
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
- `crates/eggsec/src/tool/AGENTS.override.md` - Detailed tool patterns
- `AGENTS.md` - General project guidelines
- `ARCHITECTURE.md` - Overall design
