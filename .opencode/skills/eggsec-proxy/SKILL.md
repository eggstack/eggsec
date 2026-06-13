# Eggsec Proxy Skill

Intercepting proxy module workflows and patterns for traffic inspection.

## Key Types and Patterns

### Intercepting Proxy
`proxy/intercept/` - Intercepting proxy with dynamic SSL certificates.

### Phase 2: Interactive TUI
- `Tab::Intercept` — TUI tab with live flow inspection, header/body detail panes
- `ManipulationRecord` — immutable audit trail of request/response edits
- `InterceptSession` — saveable session with flows, manipulations, and flow actions
- `FlowAction` — per-flow actions (Forward/Drop/Replay/Paused)
- Session save/load (JSON), HAR export, intercept rules display

### Phase 3: Advanced Protocols
- `EnhancedRule` / `EnhancedRuleSet` — Enhanced rule engine with complex conditions
- `RuleCondition` — AND/OR/NOT condition combinators
- `RuleContext` — Context for rule evaluation
- `WebSocketSession` / `WebSocketMessage` — WebSocket interception types
- `Http2Session` / `Http2Stream` — HTTP/2 stream tracking
- `GrpcSession` / `GrpcCall` — gRPC call interception
- `CorrelationContext` / `CorrelationReference` — Cross-loadout correlation
- `ProxyProtocol` — Protocol detection enum

### Phase 4: Pipeline, MCP, Evidence, Performance
- `ScanProfile::WebProxy` / `Stage::WebProxy` — Pipeline profile integration
- `EvidenceBundle` / `BundleManifest` — Evidence bundle export/import for multi-loadout correlation (`proxy/intercept/bundle.rs`)
- `FlowBuffer` — Capacity-capped flow buffer (`proxy/intercept/types.rs`)
- `ProxyMetrics` — Runtime performance telemetry snapshot (`proxy/intercept/types.rs`)
- `WebProxyToolSchema` / `WebProxyToolCall` — MCP proxy tool types (`proxy/mcp.rs`)
- `ProxyTool` — MCP tool handler implementation (`tool/implementations/proxy.rs`)
- Real WebSocket (`tokio-tungstenite`) and HTTP/2 (`h2`) protocol backends

### MCP Proxy Tools (12 tools via `web-proxy-mcp` feature)

The following 12 tools are available when the `web-proxy-mcp` feature is enabled:

| Tool ID | Action | Description |
|---------|--------|-------------|
| `proxy-start` | Start proxy | Start the intercepting proxy on a listen address |
| `proxy-stop` | Stop proxy | Stop the running proxy and clear session |
| `proxy-status` | Status | Get session status, flow count, and budget usage |
| `proxy-list-flows` | List flows | List intercepted flows with pagination |
| `proxy-inspect-flow` | Inspect flow | Get full detail of a specific flow by index |
| `proxy-forward-flow` | Forward | Forward a paused flow to upstream |
| `proxy-drop-flow` | Drop | Drop a paused flow without forwarding |
| `proxy-replay-flow` | Replay | Replay a flow |
| `proxy-add-rule` | Add rule | Add an intercept rule with pattern and action |
| `proxy-list-rules` | List rules | List all configured intercept rules |
| `proxy-remove-rule` | Remove rule | Remove a rule by ID |
| `proxy-export-session` | Export | Export session data as JSON or HAR |

**Policy enforcement:** All proxy tools require `EnforcementContext::evaluate()` before dispatch. Real runs need `--allow-web-proxy` + policy confirmation. Dry-run is always safe.

**Tool implementation:** `tool/implementations/proxy.rs` implements the `SecurityTool` trait with all 12 actions. Tools use a shared `PROXY_SESSION` static for session state.

### Safe Logging
`proxy` module uses `to_log_key()` for safe logging of sensitive data.

## Testing

### Running Proxy Tests
```bash
cargo test --lib -p eggsec proxy::
```

### Running with Features
```bash
# Dry-run (no hardware required)
cargo test --lib -p eggsec --features web-proxy

# MCP proxy surface
cargo test --lib -p eggsec --features web-proxy-mcp
```

### Writing Tests
Follow existing test patterns in `proxy/` modules, testing interception and safe logging.

## Common Tasks

### Adding a New Proxy Feature
1. Implement logic in `proxy/` modules
2. Use `to_log_key()` for logging sensitive data
3. Add tests for new proxy feature

### Adding Dynamic SSL Certificate Support
1. Update `proxy/intercept/` with certificate generation logic
2. Test certificate handling

### Working with Phase 4
1. Pipeline profile: `ScanProfile::WebProxy` in pipeline module
2. MCP tools: `tool/implementations/proxy.rs` (requires `web-proxy-mcp` feature)
3. Evidence bundles: `proxy/intercept/bundle.rs` (`EvidenceBundle`/`BundleManifest`)
4. Performance: `proxy/intercept/types.rs` (`FlowBuffer`, `ProxyMetrics`)
5. gRPC protobuf: `proxy/intercept/protocols.rs` (prost-based encoding/decoding)
6. Async rules: `EnhancedRuleSet::evaluate_async()` and `evaluate_indexed_async()`
7. Session resume: `WebProxySessionReport::save_to_file()` / `load_from_file()`

### Adding a New MCP Proxy Tool
1. Add the tool action to `ProxyAction` enum in `proxy/mcp.rs`
2. Add the handler in `tool/implementations/proxy.rs` `execute()` method
3. Add tool ID to `classify_tool_risk()` and `infer_tool_category()` in `tool/protocol/mcp/policy.rs`
4. Add policy entries in `McpProfilePolicy` for tool visibility per profile
5. Add capability definition in `SecurityTool::capabilities()`
6. Add tests in both `proxy/mcp.rs` and `tool/implementations/proxy.rs`

## Bug Fixes (2026-05-30)

- **health.rs:158-170**: Changed `filter_map(|r| r.ok())` to explicit `match` with `is_panic()` detection and `tracing::warn!` for JoinErrors. Previously, panics in health check tasks were silently dropped.

## Resources
- `crates/eggsec/src/proxy/AGENTS.override.md` - Detailed proxy patterns
- `architecture/web_proxy.md` - Full web proxy architecture
- `AGENTS.md` - General project guidelines
- `architecture/overview.md` - Overall design
